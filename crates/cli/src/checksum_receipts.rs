//! UNVERIFIED performance: no named cost test or measured latency bound is established here.
//! Opt-in historical checksum receipts, separate from every legacy run identity.
//! Cold admission scans the complete bounded source. Warm reads check one format
//! block and one fixed receipt; neither wall-clock latency nor a full audit is O(1).
use std::fs::{self, File, OpenOptions};
use std::io::{Read as _, Seek as _, SeekFrom, Write as _};
use std::os::unix::fs::{FileExt as _, MetadataExt as _};
use std::path::{Path, PathBuf};

use brutex_core::blake3::Hasher;
use brutex_core::instrument::InstrumentKey;
use brutex_core::vendor::Vendor;
use store::checksum_audit::{AuditedBarFile, Evidence};
use store::file::BarFile;
use store::format::Bar;
use store::path::{FileKind, StorePath, Timeframe, YearMonth};

const BYTES: usize = 512;
const SEAL_START: usize = BYTES - 32;
const SOURCE_DOMAIN: &[u8] = b"brutex-historical-checksum-source-v1\0";
const ID_DOMAIN: &[u8] = b"brutex-historical-checksum-receipt-v1\0";
const SEAL_DOMAIN: &[u8] = b"brutex-historical-checksum-completion-v1\0";

/// Explicit source and physical audit ceiling. No policy threshold is inferred.
#[derive(Clone, Copy)]
pub struct MonthRequest<'a> {
    /// Existing store root; never used to fetch vendor data.
    pub store_root: &'a Path,
    /// Existing directory in which append-only receipts and attempts are kept.
    pub receipt_root: &'a Path,
    /// Exact feed owning this month.
    pub vendor: Vendor,
    /// Exact canonical stored instrument.
    pub key: &'a InstrumentKey,
    /// Exact stored bar width.
    pub timeframe: Timeframe,
    /// Exact civil month.
    pub month: YearMonth,
    /// Maximum combined raw data-file and checksum-sidecar bytes for this audit.
    pub max_bytes: u64,
}

/// Exact source reader plus the immutable receipt that acknowledges its audit.
/// Detached receipt bytes cannot construct this type or replace its source.
pub struct AdmittedMonth {
    source: AuditedBarFile,
    receipt: Receipt,
    source_identity: [u8; 32],
    receipt_identity: [u8; 32],
}
impl AdmittedMonth {
    /// Content identity binding canonical source path and exact audited bytes.
    #[must_use]
    pub const fn receipt_identity(&self) -> [u8; 32] {
        self.receipt_identity
    }
    /// Vendor/instrument/rung/month identity, independent of the host root path.
    #[must_use]
    pub const fn source_identity(&self) -> [u8; 32] {
        self.source_identity
    }
    /// Checksum facts only, never calendar or institutional-policy approval.
    #[must_use]
    pub const fn evidence(&self) -> Evidence {
        self.source.evidence()
    }
    /// Durable receipt's exact path.
    #[must_use]
    pub fn receipt_path(&self) -> &Path {
        &self.receipt.path
    }
    /// Confirm all held source and receipt generations remain current.
    ///
    /// # Errors
    /// Refuses missing, changed, aliased or unreadable data, sidecar, lock or receipt.
    pub fn require_current(&self) -> Result<(), String> {
        self.receipt.require_current()?;
        self.source.require_current()?;
        self.receipt.require_current()
    }
    /// Read the exact row from its freshly verified fixed-size source block.
    ///
    /// # Errors
    /// Refuses any generation/receipt/CRC failure and uncommitted row indices.
    pub fn read_record(&self, index: u64) -> Result<Bar, String> {
        self.receipt.require_current()?;
        let row = self.source.read_record(index)?;
        self.receipt.require_current()?;
        self.source.require_current()?;
        Ok(row)
    }
}

/// Fully audit and acknowledge this exact source, retaining its original handles.
/// An exact existing receipt is reused; a torn exact prefix is completed by append.
///
/// # Errors
/// Refuses unsupported/unsealed/missing/corrupt/changed/over-limit input or receipts,
/// and any failed start, publication or completion durability barrier.
pub fn audit_month(request: MonthRequest<'_>) -> Result<AdmittedMonth, String> {
    perform(request, None)
}

/// Admit only the explicitly requested prior receipt and freshly audited source.
/// This cold boundary never trusts metadata as a substitute for a full byte audit.
///
/// # Errors
/// Also refuses a missing receipt or a full receipt identity different from expected.
/// It never creates or repairs a receipt in this strict-read operation.
pub fn admit_month(request: MonthRequest<'_>, expected: [u8; 32]) -> Result<AdmittedMonth, String> {
    perform(request, Some(expected))
}

fn perform(request: MonthRequest<'_>, expected: Option<[u8; 32]>) -> Result<AdmittedMonth, String> {
    let root = receipt_directory(request.receipt_root, expected.is_none())?;
    let path = StorePath::for_key(
        request.vendor,
        request.key,
        request.timeframe,
        request.month,
        FileKind::Bars,
    )
    .map_err(error)?;
    let mut hash = Hasher::new();
    hash.update(SOURCE_DOMAIN);
    hash.update(path.to_string().as_bytes());
    let source_identity = hash.finalize();
    let mut plan = Hasher::new();
    plan.update(b"brutex-historical-checksum-audit-plan-v1\0");
    plan.update(&source_identity);
    plan.update(&request.max_bytes.to_le_bytes());
    plan.update(&[u8::from(expected.is_some())]);
    if let Some(id) = expected {
        plan.update(&id);
    }
    let attempt = crate::sweep_evidence::begin(
        request.receipt_root,
        plan.finalize(),
        crate::sweep_evidence::Operation::ChecksumAudit,
    )?;
    let result = (|| {
        // Same low-32-bit identifier as the canonical store writer.
        let hash = brutex_core::universe::fnv1a(request.key.underlying.as_str()).to_le_bytes();
        let symbol_id = u32::from_le_bytes(
            hash.get(..4)
                .ok_or("symbol id range")?
                .try_into()
                .map_err(error)?,
        );
        let source =
            BarFile::open_existing_audited(request.store_root, path, symbol_id, request.max_bytes)?;
        let (bytes, receipt_identity) = encode(source_identity, source.evidence())?;
        if expected.is_some_and(|id| id != receipt_identity) {
            return Err(
                "strict checksum admission received a different source/receipt identity".to_owned(),
            );
        }
        let receipt_path = root.join(format!("{}.bin", crate::identity_hex(&receipt_identity)));
        source.require_current()?;
        if expected.is_none() {
            publish(&receipt_path, &bytes)?;
        }
        let receipt = Receipt::open(&receipt_path, &bytes)?;
        let admitted = AdmittedMonth {
            source,
            receipt,
            source_identity,
            receipt_identity,
        };
        admitted.require_current()?;
        crate::note(
            &telemetry::Event::info("cli.checksum", "historical checksum receipt admitted")
                .with(
                    "source_identity",
                    crate::identity_hex(&source_identity).as_str(),
                )
                .with(
                    "receipt_identity",
                    crate::identity_hex(&receipt_identity).as_str(),
                )
                .with("records", admitted.evidence().header().n_valid)
                .with("blocks", admitted.evidence().blocks()),
        );
        Ok(admitted)
    })();
    match result {
        Ok(admitted) => {
            admitted.require_current()?;
            attempt.finish(crate::sweep_evidence::Completion::Completed)?;
            Ok(admitted)
        }
        Err(why) => {
            note_refusal(&attempt, &source_identity, &why);
            match attempt.finish(crate::sweep_evidence::Completion::Refused) {
                Ok(()) => Err(why),
                Err(terminal) => Err(format!(
                    "{why}; checksum audit refusal could not be durably completed: {terminal}"
                )),
            }
        }
    }
}

fn note_refusal(attempt: &crate::sweep_evidence::Attempt, source: &[u8; 32], reason: &str) {
    crate::note(
        &telemetry::Event::new(
            telemetry::Level::Error,
            "cli.checksum",
            "historical checksum audit refused",
        )
        .with("source_identity", crate::identity_hex(source).as_str())
        .with(
            "identity",
            crate::identity_hex(&attempt.identity()).as_str(),
        )
        .with("attempt", attempt.token())
        .with("reason", reason),
    );
}

fn encode(source: [u8; 32], evidence: Evidence) -> Result<([u8; BYTES], [u8; 32]), String> {
    let mut bytes = [0; BYTES];
    put(&mut bytes, 0, b"BRHCRC01")?;
    put(&mut bytes, 8, &1_u64.to_le_bytes())?;
    put(&mut bytes, 16, &source)?;
    put(&mut bytes, 80, &evidence.canonical_bytes())?;
    let mut hash = Hasher::new();
    hash.update(ID_DOMAIN);
    hash.update(bytes.get(..48).ok_or("receipt identity prefix")?);
    hash.update(bytes.get(80..SEAL_START).ok_or("receipt identity body")?);
    let identity = hash.finalize();
    put(&mut bytes, 48, &identity)?;
    let mut seal = Hasher::new();
    seal.update(SEAL_DOMAIN);
    seal.update(bytes.get(..SEAL_START).ok_or("receipt seal body")?);
    put(&mut bytes, SEAL_START, &seal.finalize())?;
    Ok((bytes, identity))
}

fn put(bytes: &mut [u8; BYTES], offset: usize, value: &[u8]) -> Result<(), String> {
    bytes
        .get_mut(
            offset
                ..offset
                    .checked_add(value.len())
                    .ok_or("receipt offset overflow")?,
        )
        .ok_or("receipt field outside fixed record")?
        .copy_from_slice(value);
    Ok(())
}

pub(crate) struct Receipt {
    file: File,
    path: PathBuf,
    generation: crate::result_set::FileGeneration,
    expected: [u8; BYTES],
}
impl Receipt {
    fn open(path: &Path, expected: &[u8; BYTES]) -> Result<Self, String> {
        let file = open(path, false)?;
        // Retained for this authority's entire lifetime. Completed receipts are
        // reused through this same read door, so identical audits need no writer.
        file.try_lock_shared().map_err(error)?;
        let receipt = Self {
            generation: regular_generation(&file, path)?,
            file,
            path: path.to_path_buf(),
            expected: *expected,
        };
        receipt.require_current()?;
        Ok(receipt)
    }
    pub(crate) fn require_current(&self) -> Result<(), String> {
        crate::result_set::require_generation_unchanged(
            self.generation,
            regular_generation(&self.file, &self.path)?,
            &self.path,
        )?;
        if self.generation.len != BYTES as u64 {
            return Err("checksum receipt is missing, torn or extended".to_owned());
        }
        let mut bytes = [0; BYTES];
        self.file.read_exact_at(&mut bytes, 0).map_err(error)?;
        if bytes != self.expected {
            return Err("checksum receipt differs from the exact audited source".to_owned());
        }
        crate::result_set::require_generation_unchanged(
            self.generation,
            regular_generation(&self.file, &self.path)?,
            &self.path,
        )
    }
}

fn publish(path: &Path, expected: &[u8; BYTES]) -> Result<(), String> {
    if fs::symlink_metadata(path)
        .is_ok_and(|metadata| metadata.is_file() && metadata.len() == BYTES as u64)
    {
        Receipt::open(path, expected)?;
        return Ok(());
    }
    let mut file = open(path, true)?;
    file.try_lock().map_err(error)?;
    let result = (|| {
        let before = regular_generation(&file, path)?;
        let length = usize::try_from(before.len).map_err(error)?;
        if length > BYTES {
            return Err(
                "checksum receipt exceeds its fixed stride; no overwrite or truncation".to_owned(),
            );
        }
        let mut actual = [0; BYTES];
        let prefix = actual.get_mut(..length).ok_or("receipt prefix length")?;
        file.read_exact(prefix).map_err(error)?;
        if Some(&*prefix) != expected.get(..length) {
            return Err(
                "checksum receipt is not an exact prefix; no overwrite or truncation".to_owned(),
            );
        }
        crate::result_set::require_generation_unchanged(
            before,
            regular_generation(&file, path)?,
            path,
        )?;
        file.seek(SeekFrom::Start(before.len)).map_err(error)?;
        if length < SEAL_START {
            file.write_all(
                expected
                    .get(length..SEAL_START)
                    .ok_or("receipt payload suffix")?,
            )
            .map_err(error)?;
        }
        file.sync_all().map_err(error)?;
        if length < BYTES {
            file.write_all(
                expected
                    .get(length.max(SEAL_START)..)
                    .ok_or("receipt seal suffix")?,
            )
            .map_err(error)?;
        }
        file.sync_all().map_err(error)?;
        File::open(path.parent().ok_or("receipt parent absent")?)
            .and_then(|dir| dir.sync_all())
            .map_err(error)?;
        regular_generation(&file, path)?;
        Ok(())
    })();
    let unlock = file.unlock().map_err(error);
    result.and(unlock)
}

fn receipt_directory(root: &Path, create: bool) -> Result<PathBuf, String> {
    namespace_directory(root, "checksum-receipts-v1", create)
}

fn namespace_directory(root: &Path, namespace: &str, create: bool) -> Result<PathBuf, String> {
    if !fs::symlink_metadata(root).map_err(error)?.is_dir() {
        return Err("checksum receipts require an existing nonsymlink directory".to_owned());
    }
    let root = fs::canonicalize(root).map_err(error)?;
    let base = root.join(namespace);
    if create {
        match fs::create_dir(&base) {
            Ok(()) => File::open(&root)
                .and_then(|dir| dir.sync_all())
                .map_err(error)?,
            Err(why) if why.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(why) => return Err(error(why)),
        }
    }
    if !fs::symlink_metadata(&base).map_err(error)?.is_dir() {
        return Err("checksum receipt namespace must be a nonsymlink directory".to_owned());
    }
    Ok(base)
}

/// Durable fixed six-role manifest, reusing the locked append/read protocol.
pub(crate) fn publish_binding(
    root: &Path,
    roles: &[[u8; 32]; 6],
) -> Result<(Receipt, [u8; 32]), String> {
    let base = namespace_directory(root, "audited-inputs-v1", true)?;
    let mut bytes = [0; BYTES];
    put(&mut bytes, 0, b"BRHIN001")?;
    put(&mut bytes, 8, &1_u64.to_le_bytes())?;
    for (index, id) in roles.iter().enumerate() {
        let offset = 16 + index * 40;
        put(
            &mut bytes,
            offset,
            &u64::try_from(index).map_err(error)?.to_le_bytes(),
        )?;
        put(&mut bytes, offset + 8, id)?;
    }
    let mut hash = Hasher::new();
    hash.update(b"brutex-historical-six-role-input-binding-v1\0");
    hash.update(
        bytes
            .get(..SEAL_START)
            .ok_or("input binding fixed payload")?,
    );
    let identity = hash.finalize();
    let mut seal = Hasher::new();
    seal.update(b"brutex-historical-six-role-input-completion-v1\0");
    seal.update(bytes.get(..SEAL_START).ok_or("input binding fixed seal")?);
    put(&mut bytes, SEAL_START, &seal.finalize())?;
    let path = base.join(format!("{}.bin", crate::identity_hex(&identity)));
    publish(&path, &bytes)?;
    Ok((Receipt::open(&path, &bytes)?, identity))
}

/// One immutable fixed-stride node in a chronological strict range binding.
/// The zero-based ordinal, civil month and six exact source roles are linked
/// to the preceding node; a final node therefore binds the complete prefix.
pub(crate) fn publish_span_binding(
    root: &Path,
    previous: [u8; 32],
    ordinal: u64,
    when: (u16, u8),
    roles: &[[u8; 32]; 6],
) -> Result<(Receipt, [u8; 32]), String> {
    YearMonth::new(when.0, when.1).map_err(error)?;
    if (ordinal == 0) != (previous == [0; 32]) {
        return Err(
            "audited span binding requires an exact predecessor after its first month".to_owned(),
        );
    }
    let base = namespace_directory(root, "audited-spans-v1", true)?;
    let mut bytes = [0; BYTES];
    put(&mut bytes, 0, b"BRHSP001")?;
    put(&mut bytes, 8, &1_u64.to_le_bytes())?;
    put(&mut bytes, 16, &previous)?;
    put(&mut bytes, 48, &ordinal.to_le_bytes())?;
    put(&mut bytes, 56, &u64::from(when.0).to_le_bytes())?;
    put(&mut bytes, 64, &u64::from(when.1).to_le_bytes())?;
    for (index, id) in roles.iter().enumerate() {
        let offset = 72 + index * 40;
        put(
            &mut bytes,
            offset,
            &u64::try_from(index).map_err(error)?.to_le_bytes(),
        )?;
        put(&mut bytes, offset + 8, id)?;
    }
    let payload = bytes
        .get(..SEAL_START)
        .ok_or("span binding fixed payload")?;
    let mut hash = Hasher::new();
    hash.update(b"brutex-historical-span-input-binding-v1\0");
    hash.update(payload);
    let identity = hash.finalize();
    let mut seal = Hasher::new();
    seal.update(b"brutex-historical-span-input-completion-v1\0");
    seal.update(payload);
    put(&mut bytes, SEAL_START, &seal.finalize())?;
    let path = base.join(format!("{}.bin", crate::identity_hex(&identity)));
    publish(&path, &bytes)?;
    Ok((Receipt::open(&path, &bytes)?, identity))
}

fn regular_generation(
    file: &File,
    path: &Path,
) -> Result<crate::result_set::FileGeneration, String> {
    let held = file.metadata().map_err(error)?;
    let named = fs::symlink_metadata(path).map_err(error)?;
    if !held.is_file() || !named.is_file() || held.nlink() != 1 {
        return Err("checksum receipt refuses a non-regular file or alias".to_owned());
    }
    crate::result_set::file_generation(file, path)
}

fn open(path: &Path, writable: bool) -> Result<File, String> {
    if !writable {
        let file = crate::readonly_file::open(path).map_err(error)?;
        regular_generation(&file, path)?;
        return Ok(file);
    }
    let file = open_writable(path)?;
    regular_generation(&file, path)?;
    Ok(file)
}

#[cfg(any(
    target_os = "macos",
    all(
        target_os = "linux",
        any(target_arch = "x86_64", target_arch = "aarch64")
    )
))]
fn open_writable(path: &Path) -> Result<File, String> {
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        #[cfg(target_os = "macos")]
        options.custom_flags(0x100 | 0x4);
        #[cfg(target_os = "linux")]
        options.custom_flags(0x20_000 | 0x800);
    }
    let file = options.open(path).map_err(error)?;
    regular_generation(&file, path)?;
    Ok(file)
}

#[cfg(not(any(
    target_os = "macos",
    all(
        target_os = "linux",
        any(target_arch = "x86_64", target_arch = "aarch64")
    )
)))]
fn open_writable(_path: &Path) -> Result<File, String> {
    Err(
        "strict receipt writes require verified macOS or Linux x86_64/aarch64 open flags"
            .to_owned(),
    )
}

/// Operator command: vendor, underlying, rung, year, month, receipt root, byte cap.
/// Source root comes from the existing configured stored-reader boundary.
///
/// # Errors
/// Refuses malformed arguments and every strict audit/durability failure.
pub fn command(args: &[&str]) -> Result<String, String> {
    let [
        vendor,
        underlying,
        rung,
        year,
        month,
        receipt_root,
        max_bytes,
    ] = args
    else {
        return Err("checksum-audit-stored requires VENDOR UNDERLYING RUNG YEAR MONTH RECEIPT_ROOT MAX_BYTES".to_owned());
    };
    let vendor = crate::parse_vendor(vendor)?;
    let key = crate::stored::swept_index(underlying)?;
    let timeframe = crate::stored::rung(rung)?;
    let month = YearMonth::new(
        year.parse::<u16>().map_err(error)?,
        month.parse::<u8>().map_err(error)?,
    )
    .map_err(error)?;
    let store_root = crate::store_root()?;
    let admitted = audit_month(MonthRequest {
        store_root: &store_root,
        receipt_root: Path::new(receipt_root),
        vendor,
        key: &key,
        timeframe,
        month,
        max_bytes: max_bytes.parse::<u64>().map_err(error)?,
    })?;
    let evidence = admitted.evidence();
    Ok(format!(
        "HISTORICAL CHECKSUM AUDIT V1 · exact source bytes verified\nreceipt {}\nsource {}\nrecords {} · blocks {}\nheader {}\ndata {}\nsidecar {}\nfull-file {}\nreceipt-file {}\nChecksum integrity only; calendar completeness, vendor correctness and institutional policy admission are separate. Legacy stored run identities and ReferenceIntegrity flags remain unchanged.\n",
        crate::identity_hex(&admitted.receipt_identity()),
        crate::identity_hex(&admitted.source_identity()),
        evidence.header().n_valid,
        evidence.blocks(),
        crate::identity_hex(&evidence.header_digest()),
        crate::identity_hex(&evidence.data_digest()),
        crate::identity_hex(&evidence.sidecar_digest()),
        crate::identity_hex(&evidence.file_digest()),
        admitted.receipt_path().display(),
    ))
}

fn error(why: impl std::fmt::Display) -> String {
    why.to_string()
}

#[cfg(test)]
#[path = "checksum_receipts_tests.rs"]
mod tests;

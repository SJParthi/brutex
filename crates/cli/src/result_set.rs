//! The fixed-stride receipt that binds one public ledger row to both detail files.
//!
//! `runs.bin` is the result-set commit marker, but the ledger's aggregate
//! `combinations` and chosen-grid `trades` are not the row counts of
//! `frontier.bin` and `chosen-trades.bin`. In particular, either detail can be empty
//! while its similarly named aggregate is non-zero. Inferring completeness
//! from those aggregates therefore confuses a legitimate empty result with a
//! crash.
//!
//! This sidecar records the two exact detail counts, including zero. It is
//! appended and synced after both detail preparations and before `runs.bin`.
//! A public reader requires all three facts with the same identity: ledger
//! parent, receipt, and the receipt's exact block count. An interrupted receipt
//! has no ledger parent and remains private; a receipt-less legacy ledger row is
//! refused as unverifiable rather than guessed complete.
//!
//! The file is append-only, fixed-stride, and has no dynamic schema. Opening is
//! O(receipts) to build the identity index; an in-process identity lookup is one
//! hash probe. Neither bound is presented as measured.

use std::fs::{File, OpenOptions};
use std::io::{Read as _, Seek as _, SeekFrom, Write as _};
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::MetadataExt as _;
#[cfg(windows)]
use std::os::windows::fs::MetadataExt as _;

use crate::results::Refusal;

/// `BRUTEXRC`: the result-detail receipt, distinct from every detail file.
const MAGIC: [u8; 8] = *b"BRUTEXRC";
const VERSION: u32 = 2;
const HEADER: u64 = 16;
const HEADER_BYTES: usize = 16;
const STRIDE: u64 = 64;
const STRIDE_BYTES: usize = 64;
const SEAL_BYTES: usize = 8;
const PAYLOAD_BYTES: usize = STRIDE_BYTES - SEAL_BYTES;

const _: () = assert!(HEADER_BYTES as u64 == HEADER);
const _: () = assert!(STRIDE_BYTES as u64 == STRIDE);
const _: () = assert!(PAYLOAD_BYTES == 32 + 8 + 8 + 1 + 1 + 6);

/// The durable interpretation of the chosen-trade child.
///
/// A new policy is a new receipt/file version; an unknown value is refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TradePolicy {
    /// Rows re-walk the exact selected exit-grid cell and reconcile to it.
    ChosenGridV1,
}

impl TradePolicy {
    /// Stable wire/UI label.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ChosenGridV1 => "chosen-grid-v1",
        }
    }
}

/// One result set's exact child cardinalities.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Receipt {
    /// The same nine-term run identity carried by every parent and child row.
    pub identity: [u8; 32],
    /// Exact number of rows this run owns in `frontier.bin`, including zero.
    pub frontier_rows: u64,
    /// Exact number of rows this run owns in `chosen-trades.bin`, including zero.
    pub trade_rows: u64,
    /// Direction of the finally admitted selected candidate.
    ///
    /// This is result metadata, not inferred from the undirected frequency-run
    /// identity, and remains available when `trade_rows == 0`.
    pub direction: costs::fill::Direction,
    /// Exact semantic policy of the trade child.
    pub trade_policy: TradePolicy,
}

impl Receipt {
    fn to_bytes(self) -> [u8; STRIDE_BYTES] {
        let mut out = [0_u8; STRIDE_BYTES];
        out.get_mut(..32)
            .unwrap_or(&mut [])
            .copy_from_slice(&self.identity);
        out.get_mut(32..40)
            .unwrap_or(&mut [])
            .copy_from_slice(&self.frontier_rows.to_le_bytes());
        out.get_mut(40..48)
            .unwrap_or(&mut [])
            .copy_from_slice(&self.trade_rows.to_le_bytes());
        if let Some(byte) = out.get_mut(48) {
            *byte = direction_byte(self.direction);
        }
        if let Some(byte) = out.get_mut(49) {
            *byte = policy_byte(self.trade_policy);
        }
        let seal = seal_of(&out);
        out.get_mut(PAYLOAD_BYTES..)
            .unwrap_or(&mut [])
            .copy_from_slice(&seal);
        out
    }

    fn from_bytes(raw: &[u8; STRIDE_BYTES]) -> Result<Self, Refusal> {
        let mut identity = [0_u8; 32];
        identity.copy_from_slice(raw.get(..32).unwrap_or(&[]));
        if raw.get(50..56) != Some(&[0_u8; 6]) {
            return Err(
                "receipt has non-zero reserved bytes; this build will not invent their meaning"
                    .to_owned(),
            );
        }
        Ok(Self {
            identity,
            frontier_rows: u64::from_le_bytes(
                raw.get(32..40)
                    .and_then(|bytes| bytes.try_into().ok())
                    .unwrap_or([0; 8]),
            ),
            trade_rows: u64::from_le_bytes(
                raw.get(40..48)
                    .and_then(|bytes| bytes.try_into().ok())
                    .unwrap_or([0; 8]),
            ),
            direction: direction_of(*raw.get(48).unwrap_or(&0))?,
            trade_policy: policy_of(*raw.get(49).unwrap_or(&0))?,
        })
    }
}

const fn direction_byte(direction: costs::fill::Direction) -> u8 {
    match direction {
        costs::fill::Direction::Long => 1,
        costs::fill::Direction::Short => 2,
    }
}

fn direction_of(byte: u8) -> Result<costs::fill::Direction, Refusal> {
    match byte {
        1 => Ok(costs::fill::Direction::Long),
        2 => Ok(costs::fill::Direction::Short),
        _ => Err(format!(
            "receipt direction byte {byte} is unknown; only 1=long and 2=short are defined"
        )),
    }
}

const fn policy_byte(policy: TradePolicy) -> u8 {
    match policy {
        TradePolicy::ChosenGridV1 => 1,
    }
}

fn policy_of(byte: u8) -> Result<TradePolicy, Refusal> {
    match byte {
        1 => Ok(TradePolicy::ChosenGridV1),
        _ => Err(format!(
            "receipt trade-policy byte {byte} is unknown; only 1=chosen-grid-v1 is defined"
        )),
    }
}

fn seal_of(raw: &[u8; STRIDE_BYTES]) -> [u8; SEAL_BYTES] {
    let mut hasher = brutex_core::blake3::Hasher::new();
    hasher.update(raw.get(..PAYLOAD_BYTES).unwrap_or(&[]));
    let digest = hasher.finalize();
    let mut out = [0_u8; SEAL_BYTES];
    out.copy_from_slice(digest.get(..SEAL_BYTES).unwrap_or(&[]));
    out
}

fn seal_matches(raw: &[u8; STRIDE_BYTES]) -> bool {
    raw.get(PAYLOAD_BYTES..) == Some(&seal_of(raw)[..])
}

/// Whether this call wrote a receipt or verified one left by an interrupted run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Prepared {
    /// The receipt was appended and durably synced by this call.
    Written,
    /// The exact receipt already existed and its durability was reconfirmed.
    Reused,
}

/// Constant-size filesystem evidence for the exact file generation already
/// scanned into memory.
///
/// Unix and Windows expose a stable file identity plus a change clock. Other
/// targets may still read and validate a manifest, but append fails closed:
/// portable `std::fs::Metadata` has no stable file identity with which to
/// distinguish a same-length replacement. These fields detect ordinary
/// filesystem mutation; they are not authentication against an actor able to
/// forge filesystem metadata.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FileGeneration {
    pub(crate) len: u64,
    platform: PlatformGeneration,
}

#[cfg(unix)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PlatformGeneration {
    device: u64,
    inode: u64,
    modified_seconds: i64,
    modified_nanoseconds: i64,
    changed_seconds: i64,
    changed_nanoseconds: i64,
}

#[cfg(windows)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PlatformGeneration {
    volume_serial: u32,
    file_index: u64,
    creation_time: u64,
    last_write_time: u64,
}

#[cfg(not(any(unix, windows)))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PlatformGeneration;

/// The append-only receipt sidecar, indexed by run identity when opened.
#[derive(Debug)]
pub struct Receipts {
    file: File,
    path: PathBuf,
    seen: std::collections::HashMap<[u8; 32], Receipt>,
    scanned: u64,
    generation: FileGeneration,
    max_bytes: Option<u64>,
}

impl Receipts {
    /// The receipt path beside the ledger and both detail files.
    #[must_use]
    pub fn path(root: &Path) -> PathBuf {
        root.join("results").join("detail-sets.bin")
    }

    /// Opens an existing receipt file without creating it or its directory.
    ///
    /// # Errors
    ///
    /// Refuses an absent file, wrong magic/version, ragged length, failed seal,
    /// duplicate identity, or I/O failure. Every refusal names the path/cause.
    pub fn open_read(root: &Path) -> Result<Self, Refusal> {
        let path = Self::path(root);
        let file = File::open(&path)
            .map_err(|why| format!("{} could not be opened: {why}", path.display()))?;
        file.lock_shared().map_err(|why| {
            format!(
                "{} could not be locked for validation: {why}",
                path.display()
            )
        })?;
        Self::from_file(file, path, None)
    }

    /// Opens for reading only when the receipt manifest fits `max_bytes`.
    ///
    /// The opened handle is measured before its header or any receipt is read,
    /// so this is the hard gate used by bounded HTTP detail requests rather
    /// than a path-level metadata hint that can go stale before open.
    ///
    /// # Errors
    ///
    /// Every refusal from [`Self::open_read`], plus an over-limit manifest. An
    /// over-limit file is not partially indexed.
    pub fn open_read_bounded(root: &Path, max_bytes: u64) -> Result<Self, Refusal> {
        let path = Self::path(root);
        let file = File::open(&path)
            .map_err(|why| format!("{} could not be opened: {why}", path.display()))?;
        file.lock_shared().map_err(|why| {
            format!(
                "{} could not be locked for validation: {why}",
                path.display()
            )
        })?;
        Self::from_file(file, path, Some(max_bytes))
    }

    /// Opens for append, creating a fresh header when the file does not exist.
    ///
    /// # Errors
    ///
    /// Refuses when the directory/file cannot be created or when an existing
    /// file fails any validation performed by [`Self::open_read`].
    pub fn open(root: &Path) -> Result<Self, Refusal> {
        let dir = root.join("results");
        std::fs::create_dir_all(&dir)
            .map_err(|why| format!("the results directory could not be made: {why}"))?;
        let path = Self::path(root);
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)
            .map_err(|why| format!("{} could not be opened: {why}", path.display()))?;
        file.lock().map_err(|why| {
            format!(
                "{} could not be locked for validation: {why}",
                path.display()
            )
        })?;
        if file
            .metadata()
            .map_err(|why| format!("{} could not be measured: {why}", path.display()))?
            .len()
            == 0
        {
            write_header(&mut file, &path)?;
        }
        Self::from_file(file, path, None)
    }

    fn from_file(mut file: File, path: PathBuf, max_bytes: Option<u64>) -> Result<Self, Refusal> {
        let len = file
            .metadata()
            .map_err(|why| format!("{} could not be measured: {why}", path.display()))?
            .len();
        if let Some(max_bytes) = max_bytes
            && len > max_bytes
        {
            return Err(format!(
                "{} is {len} bytes; this bounded receipt reader accepts at most {max_bytes}. No header or receipt was read and no partial identity index was built",
                path.display()
            ));
        }
        check_shape(&mut file, &path, len)?;
        let mut seen = std::collections::HashMap::with_capacity(
            usize::try_from(len.saturating_sub(HEADER) / STRIDE).unwrap_or(0),
        );
        let mut at = HEADER;
        while at.saturating_add(STRIDE) <= len {
            let record = read_at(&mut file, &path, at)?;
            if seen.insert(record.identity, record).is_some() {
                return Err(format!(
                    "{} contains duplicate receipt identity {}. The manifest is ambiguous and no result set is exposed.",
                    path.display(),
                    identity_hex(&record.identity)
                ));
            }
            at = at.saturating_add(STRIDE);
        }
        let generation = file_generation(&file, &path)?;
        if generation.len != at {
            return Err(format!(
                "{} changed length from the validated byte {at} to {} while it was being opened. No partial identity index is exposed",
                path.display(),
                generation.len
            ));
        }
        file.unlock().map_err(|why| {
            format!(
                "{} could not release its validation lock: {why}",
                path.display()
            )
        })?;
        Ok(Self {
            file,
            path,
            seen,
            scanned: at,
            generation,
            max_bytes,
        })
    }

    /// Finds one receipt with one in-memory hash probe after the open-time scan.
    #[must_use]
    pub fn of_identity(&self, identity: &[u8; 32]) -> Option<Receipt> {
        self.seen.get(identity).copied()
    }

    /// Refresh only newly appended receipts under a shared lock.
    ///
    /// # Errors
    /// Refuses a changed generation, malformed receipt, exceeded read bound or
    /// I/O error. The initial read limit remains active for the handle's life.
    pub fn refresh(&mut self) -> Result<(), Refusal> {
        self.file
            .lock_shared()
            .map_err(|why| format!("the receipt file could not be locked: {why}"))?;
        let refreshed = self.absorb_new();
        let released = self
            .file
            .unlock()
            .map_err(|why| format!("the receipt file could not be unlocked: {why}"));
        refreshed.and(released)
    }

    /// Appends and syncs one receipt, or verifies the exact existing receipt.
    ///
    /// A different count for the same identity is nondeterminism and is refused;
    /// no byte is replaced. The per-file lock is in addition to the whole-set
    /// writer lock held by `record_all`, so direct callers cannot interleave two
    /// receipt rows either.
    ///
    /// # Errors
    ///
    /// Refuses lock, validation, duplicate mismatch, write, sync, or unlock
    /// failures. A same-length external mutation or replacement of the already
    /// validated prefix is refused from constant-size filesystem-generation
    /// evidence; no O(history) prefix rescan is hidden here. On a target without
    /// a stable file identity, append fails closed. A partial write is rolled
    /// back only to this call's known start.
    pub fn append_exact(&mut self, receipt: Receipt) -> Result<Prepared, Refusal> {
        self.file
            .lock()
            .map_err(|why| format!("the receipt file could not be locked: {why}"))?;
        let result = self.append_locked(receipt);
        let released = self
            .file
            .unlock()
            .map_err(|why| format!("the receipt file could not be unlocked: {why}"));
        match (result, released) {
            (Ok(state), Ok(())) => Ok(state),
            (Err(why), _) | (Ok(_), Err(why)) => Err(why),
        }
    }

    fn append_locked(&mut self, receipt: Receipt) -> Result<Prepared, Refusal> {
        self.absorb_new()?;
        if let Some(existing) = self.of_identity(&receipt.identity) {
            if existing != receipt {
                return Err(format!(
                    "run {} already has receipt frontier={} trades={} direction={} policy={}, but this rerun produced frontier={} trades={} direction={} policy={}. The ledger was NOT appended and no receipt byte was replaced",
                    identity_hex(&receipt.identity),
                    existing.frontier_rows,
                    existing.trade_rows,
                    existing.direction,
                    existing.trade_policy.as_str(),
                    receipt.frontier_rows,
                    receipt.trade_rows,
                    receipt.direction,
                    receipt.trade_policy.as_str()
                ));
            }
            self.file
                .sync_all()
                .map_err(|why| format!("the existing detail receipt could not be synced: {why}"))?;
            let scanned = self.scanned;
            self.refresh_generation(scanned)?;
            return Ok(Prepared::Reused);
        }
        let at = self
            .file
            .seek(SeekFrom::End(0))
            .map_err(|why| format!("the receipt file could not be extended: {why}"))?;
        self.file
            .write_all(&receipt.to_bytes())
            .map_err(|why| match self.file.set_len(at) {
                Ok(()) => format!(
                    "the detail receipt could not be written: {why}. Its partial bytes were rolled back"
                ),
                Err(and) => format!(
                    "the detail receipt could not be written: {why}. Rolling its partial bytes back also failed: {and}; the file may now end mid-record"
                ),
            })?;
        self.file
            .sync_all()
            .map_err(|why| format!("the new detail receipt could not be synced: {why}"))?;
        let scanned = at.saturating_add(STRIDE);
        let generation = self.validated_generation(scanned)?;
        self.seen.insert(receipt.identity, receipt);
        self.scanned = scanned;
        self.generation = generation;
        Ok(Prepared::Written)
    }

    fn absorb_new(&mut self) -> Result<(), Refusal> {
        let observed = file_generation(&self.file, &self.path)?;
        let len = observed.len;
        if self.max_bytes.is_some_and(|bound| len > bound) {
            return Err(format!(
                "{} grew to {len} bytes beyond this receipt reader's {:?}-byte bound; no new receipt was indexed",
                self.path.display(),
                self.max_bytes
            ));
        }
        if len < self.scanned {
            return Err(format!(
                "{} shrank from the already validated byte {} to {len}. An append-only receipt file may never lose bytes, so this stale handle will not reuse cached identities or append",
                self.path.display(),
                self.scanned
            ));
        }
        if len < HEADER || !len.saturating_sub(HEADER).is_multiple_of(STRIDE) {
            return Err(format!(
                "{} has length {len}, which is not a header plus whole {STRIDE}-byte receipts. A torn tail is never ignored or padded",
                self.path.display()
            ));
        }
        if len == self.scanned {
            require_generation_unchanged(self.generation, observed, &self.path)?;
            self.generation = observed;
            return Ok(());
        }
        while self.scanned.saturating_add(STRIDE) <= len {
            let receipt = read_at(&mut self.file, &self.path, self.scanned)?;
            if let Some(existing) = self.seen.insert(receipt.identity, receipt) {
                return Err(format!(
                    "{} gained duplicate receipt identity {} (frontier={} trades={} direction={} policy={} then frontier={} trades={} direction={} policy={}). The manifest is ambiguous",
                    self.path.display(),
                    identity_hex(&receipt.identity),
                    existing.frontier_rows,
                    existing.trade_rows,
                    existing.direction,
                    existing.trade_policy.as_str(),
                    receipt.frontier_rows,
                    receipt.trade_rows,
                    receipt.direction,
                    receipt.trade_policy.as_str()
                ));
            }
            self.scanned = self.scanned.saturating_add(STRIDE);
        }
        self.refresh_generation(len)?;
        Ok(())
    }

    fn refresh_generation(&mut self, expected_len: u64) -> Result<(), Refusal> {
        self.generation = self.validated_generation(expected_len)?;
        Ok(())
    }

    fn validated_generation(&self, expected_len: u64) -> Result<FileGeneration, Refusal> {
        let generation = file_generation(&self.file, &self.path)?;
        if generation.len != expected_len {
            return Err(format!(
                "{} changed length from the just-validated byte {expected_len} to {} before its filesystem generation could be retained. Nothing further was appended",
                self.path.display(),
                generation.len
            ));
        }
        Ok(generation)
    }
}

pub(crate) fn file_generation(file: &File, path: &Path) -> Result<FileGeneration, Refusal> {
    let held = file
        .metadata()
        .map_err(|why| format!("{} open file could not be measured: {why}", path.display()))?;
    let named = std::fs::metadata(path)
        .map_err(|why| format!("{} path could not be measured: {why}", path.display()))?;
    platform_generation(&held, &named, path)
}

#[cfg(unix)]
fn platform_generation(
    held: &std::fs::Metadata,
    named: &std::fs::Metadata,
    path: &Path,
) -> Result<FileGeneration, Refusal> {
    let held = FileGeneration {
        len: held.len(),
        platform: PlatformGeneration {
            device: held.dev(),
            inode: held.ino(),
            modified_seconds: held.mtime(),
            modified_nanoseconds: held.mtime_nsec(),
            changed_seconds: held.ctime(),
            changed_nanoseconds: held.ctime_nsec(),
        },
    };
    let named = FileGeneration {
        len: named.len(),
        platform: PlatformGeneration {
            device: named.dev(),
            inode: named.ino(),
            modified_seconds: named.mtime(),
            modified_nanoseconds: named.mtime_nsec(),
            changed_seconds: named.ctime(),
            changed_nanoseconds: named.ctime_nsec(),
        },
    };
    if (held.platform.device, held.platform.inode) != (named.platform.device, named.platform.inode)
    {
        return Err(format!(
            "{} no longer names the opened receipt file; a replacement or path swap was refused",
            path.display()
        ));
    }
    if held != named {
        return Err(format!(
            "{} changed while its filesystem generation was being measured; cached receipts were not reused",
            path.display()
        ));
    }
    Ok(held)
}

#[cfg(windows)]
fn platform_generation(
    held: &std::fs::Metadata,
    named: &std::fs::Metadata,
    path: &Path,
) -> Result<FileGeneration, Refusal> {
    fn of(metadata: &std::fs::Metadata, path: &Path) -> Result<FileGeneration, Refusal> {
        let volume_serial = metadata.volume_serial_number().ok_or_else(|| {
            format!(
                "{} has no Windows volume serial; same-length stale detection fails closed",
                path.display()
            )
        })?;
        let file_index = metadata.file_index().ok_or_else(|| {
            format!(
                "{} has no Windows file index; same-length stale detection fails closed",
                path.display()
            )
        })?;
        Ok(FileGeneration {
            len: metadata.len(),
            platform: PlatformGeneration {
                volume_serial,
                file_index,
                creation_time: metadata.creation_time(),
                last_write_time: metadata.last_write_time(),
            },
        })
    }

    let held = of(held, path)?;
    let named = of(named, path)?;
    if (held.platform.volume_serial, held.platform.file_index)
        != (named.platform.volume_serial, named.platform.file_index)
    {
        return Err(format!(
            "{} no longer names the opened receipt file; a replacement or path swap was refused",
            path.display()
        ));
    }
    if held != named {
        return Err(format!(
            "{} changed while its filesystem generation was being measured; cached receipts were not reused",
            path.display()
        ));
    }
    Ok(held)
}

#[cfg(not(any(unix, windows)))]
fn platform_generation(
    held: &std::fs::Metadata,
    named: &std::fs::Metadata,
    path: &Path,
) -> Result<FileGeneration, Refusal> {
    if held.len() != named.len() {
        return Err(format!(
            "{} changed length while it was being measured; cached receipts were not reused",
            path.display()
        ));
    }
    Ok(FileGeneration {
        len: held.len(),
        platform: PlatformGeneration,
    })
}

#[cfg(any(unix, windows))]
pub(crate) fn require_generation_unchanged(
    expected: FileGeneration,
    observed: FileGeneration,
    path: &Path,
) -> Result<(), Refusal> {
    if expected == observed {
        return Ok(());
    }
    Err(format!(
        "{} kept length {} but its validated filesystem generation changed; cached receipts were not reused and nothing was appended. Reopen to validate the complete manifest",
        path.display(),
        observed.len
    ))
}

#[cfg(not(any(unix, windows)))]
pub(crate) fn require_generation_unchanged(
    _expected: FileGeneration,
    observed: FileGeneration,
    path: &Path,
) -> Result<(), Refusal> {
    Err(format!(
        "{} is unchanged at {} bytes, but this target exposes no stable file identity; append fails closed rather than trusting a possibly replaced same-length manifest",
        path.display(),
        observed.len
    ))
}

/// Returns the sealed detail receipt only when the ledger has committed this
/// identity. This is the public-read gate used even when a child file itself is
/// missing, because an absent file cannot construct a `Frontier` or `Trades`
/// handle on which to perform the ordinary check.
///
/// `Ok(None)` means no ledger parent exists. A parent with no readable matching
/// receipt is a named unverifiable refusal, never an inferred legacy success.
///
/// # Errors
///
/// Refuses a damaged/unreadable ledger or receipt file and a committed identity
/// whose receipt is absent.
pub fn committed_receipt(root: &Path, identity: &[u8; 32]) -> Result<Option<Receipt>, Refusal> {
    committed_receipt_with_limit(root, identity, None)
}

/// Refreshable, bounded parent-ledger and receipt snapshot for detail readers.
///
/// Initial admission reads the existing history once. Subsequent refreshes
/// validate file generations and index appended rows only. A receipt is never
/// returned without a currently readable, seal-verified ledger parent.
#[derive(Debug)]
pub struct CommittedParents {
    root: PathBuf,
    max_bytes: u64,
    ledger: Option<crate::results::Results>,
    receipts: Option<Receipts>,
    valid: bool,
}

impl CommittedParents {
    /// Admit both existing parent files under the same per-file byte ceiling.
    /// Missing files stay absent; this reader creates no path.
    ///
    /// # Errors
    /// Refuses malformed, changed, inaccessible or over-limit parent evidence.
    pub fn open_read_bounded(root: &Path, max_bytes: u64) -> Result<Self, Refusal> {
        let mut reader = Self {
            root: root.to_path_buf(),
            max_bytes,
            ledger: None,
            receipts: None,
            valid: false,
        };
        reader.refresh()?;
        Ok(reader)
    }

    /// Refresh the ledger first, then its receipt sidecar, before child reads.
    ///
    /// # Errors
    /// Refuses generation changes, corruption, exceeded limits and I/O errors.
    pub fn refresh(&mut self) -> Result<(), Refusal> {
        self.valid = false;
        if let Some(ledger) = &mut self.ledger {
            ledger.refresh()?;
        } else if crate::results::Results::path(&self.root)
            .try_exists()
            .map_err(|why| format!("ledger path cannot be inspected: {why}"))?
        {
            self.ledger = Some(crate::results::Results::open_read_bounded(
                &self.root,
                self.max_bytes,
            )?);
        }
        if let Some(receipts) = &mut self.receipts {
            receipts.refresh()?;
        } else if Receipts::path(&self.root)
            .try_exists()
            .map_err(|why| format!("receipt path cannot be inspected: {why}"))?
        {
            self.receipts = Some(Receipts::open_read_bounded(&self.root, self.max_bytes)?);
        }
        self.valid = true;
        Ok(())
    }

    /// Return owned receipt evidence from the last successful parent refresh.
    ///
    /// # Errors
    /// Refuses a corrupt parent or committed row lacking a sealed receipt.
    pub fn receipt(&mut self, identity: &[u8; 32]) -> Result<Option<Receipt>, Refusal> {
        if !self.valid {
            return Err("parent evidence has not passed its latest refresh; cached receipts are unavailable".to_owned());
        }
        let Some(ledger) = &mut self.ledger else {
            return Ok(None);
        };
        if ledger.of_identity(identity)?.is_none() {
            return Ok(None);
        }
        self.receipts.as_ref().and_then(|receipts| receipts.of_identity(identity)).map(Some).ok_or_else(|| format!(
            "run {} has a results-ledger parent but no validated detail receipt; its children are not exposed", identity_hex(identity)
        ))
    }
}

/// The committed-receipt read gate with a hard ceiling on each parent file.
///
/// This is intentionally a separate entry point: CLI commands retain their
/// historical unrestricted reader, while an HTTP request must not turn a
/// growing ledger or receipt manifest into unbounded blocking work.
///
/// # Errors
///
/// Every refusal from [`committed_receipt`], plus either parent file exceeding
/// `max_bytes`. No prefix index is used to answer an over-limit request.
pub fn committed_receipt_bounded(
    root: &Path,
    identity: &[u8; 32],
    max_bytes: u64,
) -> Result<Option<Receipt>, Refusal> {
    committed_receipt_with_limit(root, identity, Some(max_bytes))
}

fn committed_receipt_with_limit(
    root: &Path,
    identity: &[u8; 32],
    max_bytes: Option<u64>,
) -> Result<Option<Receipt>, Refusal> {
    if !crate::results::Results::path(root).exists() {
        return Ok(None);
    }
    let mut ledger = match max_bytes {
        Some(max_bytes) => crate::results::Results::open_read_bounded(root, max_bytes)?,
        None => crate::results::Results::open_read(root)?,
    };
    if ledger.of_identity(identity)?.is_none() {
        return Ok(None);
    }
    let receipts = match max_bytes {
        Some(max_bytes) => Receipts::open_read_bounded(root, max_bytes),
        None => Receipts::open_read(root),
    }
    .map_err(|why| {
        format!(
            "run {} has a results-ledger parent, but its detail receipt could not be read: {why}. This receipt-less or damaged result set is unverifiable and is NOT exposed",
            identity_hex(identity)
        )
    })?;
    receipts.of_identity(identity).map(Some).ok_or_else(|| {
        format!(
            "run {} has a results-ledger parent but no detail receipt. It predates the receipt protocol or is a legacy ledger-first partial commit; completeness is unverifiable and the run is NOT exposed",
            identity_hex(identity)
        )
    })
}

fn write_header(file: &mut File, path: &Path) -> Result<(), Refusal> {
    let mut header = [0_u8; HEADER_BYTES];
    header
        .get_mut(..8)
        .unwrap_or(&mut [])
        .copy_from_slice(&MAGIC);
    header
        .get_mut(8..12)
        .unwrap_or(&mut [])
        .copy_from_slice(&VERSION.to_le_bytes());
    file.write_all(&header)
        .and_then(|()| file.sync_all())
        .map_err(|why| {
            format!(
                "{} could not receive a durable header: {why}",
                path.display()
            )
        })
}

fn check_shape(file: &mut File, path: &Path, len: u64) -> Result<(), Refusal> {
    // READ THE FORMAT IDENTITY BEFORE APPLYING THIS VERSION'S GEOMETRY.
    // Version 1 had a 56-byte receipt. Checking the version-2 64-byte stride
    // first turned an intact legacy file into a generic "bad length" report,
    // hiding the only safe remedy: rerun to write chosen-grid metadata.
    if len < HEADER {
        return Err(format!(
            "{} has length {len}, shorter than its {HEADER}-byte receipt header",
            path.display()
        ));
    }
    let mut header = [0_u8; HEADER_BYTES];
    file.seek(SeekFrom::Start(0))
        .and_then(|_| file.read_exact(&mut header))
        .map_err(|why| format!("{} header could not be read: {why}", path.display()))?;
    if header.get(..8) != Some(&MAGIC) {
        return Err(format!(
            "{} does not begin with `BRUTEXRC`. No receipt was trusted",
            path.display()
        ));
    }
    let version = u32::from_le_bytes(
        header
            .get(8..12)
            .and_then(|bytes| bytes.try_into().ok())
            .unwrap_or([0; 4]),
    );
    if version != VERSION {
        return Err(format!(
            "{} is receipt version {version}; this build reads version {VERSION} and will not guess a stride",
            path.display()
        ));
    }
    if header.get(12..16) != Some(&[0_u8; 4]) {
        return Err(format!(
            "{} has non-zero reserved header bytes. This build assigns them no meaning and refuses rather than guessing",
            path.display()
        ));
    }
    if !len.saturating_sub(HEADER).is_multiple_of(STRIDE) {
        return Err(format!(
            "{} has length {len}, which is not a {HEADER}-byte header plus whole {STRIDE}-byte version-{VERSION} receipts",
            path.display()
        ));
    }
    Ok(())
}

fn read_at(file: &mut File, path: &Path, at: u64) -> Result<Receipt, Refusal> {
    let mut raw = [0_u8; STRIDE_BYTES];
    file.seek(SeekFrom::Start(at))
        .and_then(|_| file.read_exact(&mut raw))
        .map_err(|why| {
            format!(
                "{} receipt at byte {at} could not be read: {why}",
                path.display()
            )
        })?;
    if !seal_matches(&raw) {
        return Err(format!(
            "{} receipt at byte {at} failed its seal. No parent using this manifest is exposed",
            path.display()
        ));
    }
    Receipt::from_bytes(&raw).map_err(|why| {
        format!(
            "{} receipt at byte {at} is sealed but invalid: {why}",
            path.display()
        )
    })
}

fn identity_hex(identity: &[u8; 32]) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(64);
    for byte in identity {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "tests must fail loudly when their fixture cannot be built"
)]
mod tests {
    use super::{Prepared, Receipt, Receipts, TradePolicy};
    use std::fs::OpenOptions;
    use std::io::Write as _;

    fn root(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "brutex-detail-receipt-{name}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ))
    }

    #[test]
    fn zero_counts_are_first_class_and_exact_reuse_adds_no_byte() {
        let root = root("zero");
        let _ = std::fs::remove_dir_all(&root);
        let receipt = Receipt {
            identity: [1; 32],
            frontier_rows: 0,
            trade_rows: 0,
            direction: costs::fill::Direction::Long,
            trade_policy: TradePolicy::ChosenGridV1,
        };
        let mut store = Receipts::open(&root).expect("receipt store");
        assert_eq!(
            store.append_exact(receipt).expect("first"),
            Prepared::Written
        );
        let before = std::fs::metadata(Receipts::path(&root))
            .expect("metadata")
            .len();
        assert_eq!(
            store.append_exact(receipt).expect("reuse"),
            Prepared::Reused
        );
        assert_eq!(
            std::fs::metadata(Receipts::path(&root))
                .expect("metadata")
                .len(),
            before
        );
        assert_eq!(
            Receipts::open_read(&root)
                .expect("reader")
                .of_identity(&receipt.identity),
            Some(receipt)
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn bounded_read_refuses_before_indexing_an_over_limit_manifest() {
        let root = root("bounded-read");
        let _ = std::fs::remove_dir_all(&root);
        drop(Receipts::open(&root).expect("valid receipt header"));
        let measured = std::fs::metadata(Receipts::path(&root))
            .expect("receipt metadata")
            .len();
        let why = Receipts::open_read_bounded(&root, measured.saturating_sub(1))
            .expect_err("a valid but over-limit manifest refuses");
        assert!(why.contains(&format!("is {measured} bytes")), "{why}");
        assert!(why.contains("No header or receipt was read"), "{why}");
        assert!(why.contains("no partial identity index"), "{why}");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_count_mismatch_is_never_rewritten() {
        let root = root("mismatch");
        let _ = std::fs::remove_dir_all(&root);
        let mut store = Receipts::open(&root).expect("receipt store");
        let first = Receipt {
            identity: [2; 32],
            frontier_rows: 3,
            trade_rows: 5,
            direction: costs::fill::Direction::Short,
            trade_policy: TradePolicy::ChosenGridV1,
        };
        store.append_exact(first).expect("first");
        let before = std::fs::read(Receipts::path(&root)).expect("bytes");
        let why = store
            .append_exact(Receipt {
                trade_rows: 4,
                ..first
            })
            .expect_err("different count");
        assert!(why.contains("NOT appended"), "{why}");
        assert!(
            why.contains(&"02".repeat(32)),
            "the exact conflicting identity is named: {why}"
        );
        assert_eq!(std::fs::read(Receipts::path(&root)).expect("bytes"), before);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_torn_tail_and_a_corrupt_seal_are_both_refused() {
        let torn = root("torn");
        let _ = std::fs::remove_dir_all(&torn);
        Receipts::open(&torn).expect("header");
        let mut file = OpenOptions::new()
            .append(true)
            .open(Receipts::path(&torn))
            .expect("tail");
        file.write_all(&[1]).expect("one byte");
        assert!(
            Receipts::open_read(&torn)
                .expect_err("ragged")
                .contains("whole")
        );

        let corrupt = root("corrupt");
        let _ = std::fs::remove_dir_all(&corrupt);
        let mut store = Receipts::open(&corrupt).expect("store");
        store
            .append_exact(Receipt {
                identity: [3; 32],
                frontier_rows: 1,
                trade_rows: 2,
                direction: costs::fill::Direction::Long,
                trade_policy: TradePolicy::ChosenGridV1,
            })
            .expect("receipt");
        drop(store);
        let mut bytes = std::fs::read(Receipts::path(&corrupt)).expect("bytes");
        let cell = bytes.get_mut(16).expect("first payload byte");
        *cell ^= 1;
        std::fs::write(Receipts::path(&corrupt), bytes).expect("corrupts fixture");
        assert!(
            Receipts::open_read(&corrupt)
                .expect_err("seal")
                .contains("seal")
        );
        let _ = std::fs::remove_dir_all(&torn);
        let _ = std::fs::remove_dir_all(&corrupt);
    }

    #[test]
    fn two_stale_handles_converge_on_one_exact_receipt() {
        let root = root("concurrent");
        let _ = std::fs::remove_dir_all(&root);
        let mut one = Receipts::open(&root).expect("first handle");
        let mut two = Receipts::open(&root).expect("second stale handle");
        let receipt = Receipt {
            identity: [4; 32],
            frontier_rows: 7,
            trade_rows: 9,
            direction: costs::fill::Direction::Short,
            trade_policy: TradePolicy::ChosenGridV1,
        };
        let first = std::thread::spawn(move || one.append_exact(receipt).expect("first append"));
        let second = std::thread::spawn(move || two.append_exact(receipt).expect("second append"));
        let states = [
            first.join().expect("first thread"),
            second.join().expect("second thread"),
        ];
        assert!(states.contains(&Prepared::Written));
        assert!(states.contains(&Prepared::Reused));
        assert_eq!(
            std::fs::metadata(Receipts::path(&root))
                .expect("receipt metadata")
                .len(),
            super::HEADER + super::STRIDE,
            "two writers leave one fixed-stride receipt"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_stale_handle_refuses_a_new_ragged_tail_before_appending() {
        let root = root("stale-ragged");
        let _ = std::fs::remove_dir_all(&root);
        let mut stale = Receipts::open(&root).expect("stale handle");
        let mut file = OpenOptions::new()
            .append(true)
            .open(Receipts::path(&root))
            .expect("receipt bytes");
        file.write_all(&[0xff]).expect("ragged tail");
        file.sync_all().expect("durable fixture");
        let before = std::fs::metadata(Receipts::path(&root))
            .expect("metadata")
            .len();
        let why = stale
            .append_exact(Receipt {
                identity: [6; 32],
                frontier_rows: 1,
                trade_rows: 1,
                direction: costs::fill::Direction::Long,
                trade_policy: TradePolicy::ChosenGridV1,
            })
            .expect_err("stale handle must remeasure");
        assert!(why.contains("torn tail"), "{why}");
        assert_eq!(
            std::fs::metadata(Receipts::path(&root))
                .expect("metadata")
                .len(),
            before,
            "no row follows the tear"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_stale_handle_refuses_to_reuse_a_receipt_after_the_file_shrinks() {
        let root = root("stale-shrink");
        let _ = std::fs::remove_dir_all(&root);
        let receipt = Receipt {
            identity: [7; 32],
            frontier_rows: 2,
            trade_rows: 3,
            direction: costs::fill::Direction::Short,
            trade_policy: TradePolicy::ChosenGridV1,
        };
        let mut stale = Receipts::open(&root).expect("stale handle");
        assert_eq!(
            stale.append_exact(receipt).expect("first receipt"),
            Prepared::Written
        );
        OpenOptions::new()
            .write(true)
            .open(Receipts::path(&root))
            .expect("receipt bytes")
            .set_len(super::HEADER)
            .expect("truncate fixture");

        let why = stale
            .append_exact(receipt)
            .expect_err("cached receipt must not survive a shrink");
        assert!(why.contains("shrank"), "{why}");
        assert_eq!(
            std::fs::metadata(Receipts::path(&root))
                .expect("metadata")
                .len(),
            super::HEADER,
            "a stale cached identity is neither reused nor rewritten"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[cfg(any(unix, windows))]
    #[test]
    fn a_same_length_mutation_invalidates_the_stale_generation_before_append() {
        let root = root("stale-same-length-mutation");
        let _ = std::fs::remove_dir_all(&root);
        let first = Receipt {
            identity: [8; 32],
            frontier_rows: 2,
            trade_rows: 3,
            direction: costs::fill::Direction::Long,
            trade_policy: TradePolicy::ChosenGridV1,
        };
        let mut stale = Receipts::open(&root).expect("stale handle");
        stale.append_exact(first).expect("first receipt");
        let path = Receipts::path(&root);
        let before = std::fs::metadata(&path).expect("metadata").len();
        let mut bytes = std::fs::read(&path).expect("receipt bytes");
        *bytes.get_mut(super::HEADER_BYTES).expect("payload byte") ^= 1;
        std::fs::write(&path, bytes).expect("same-length mutation");
        assert_eq!(
            std::fs::metadata(&path).expect("mutated metadata").len(),
            before,
            "the adversarial write changes no length"
        );

        let why = stale
            .append_exact(Receipt {
                identity: [9; 32],
                frontier_rows: 5,
                trade_rows: 8,
                direction: costs::fill::Direction::Short,
                trade_policy: TradePolicy::ChosenGridV1,
            })
            .expect_err("same-length mutation must invalidate cached receipts");
        assert!(why.contains("generation changed"), "{why}");
        assert!(why.contains("nothing was appended"), "{why}");
        assert_eq!(
            std::fs::metadata(&path).expect("refused metadata").len(),
            before,
            "no receipt follows the changed generation"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[cfg(unix)]
    #[test]
    fn a_same_length_replacement_cannot_redirect_an_open_append_handle() {
        let root = root("stale-same-length-replacement");
        let _ = std::fs::remove_dir_all(&root);
        let first = Receipt {
            identity: [10; 32],
            frontier_rows: 13,
            trade_rows: 21,
            direction: costs::fill::Direction::Short,
            trade_policy: TradePolicy::ChosenGridV1,
        };
        let mut stale = Receipts::open(&root).expect("stale handle");
        stale.append_exact(first).expect("first receipt");
        let path = Receipts::path(&root);
        let replacement = root.join("results").join("replacement.bin");
        let bytes = std::fs::read(&path).expect("receipt bytes");
        let before = u64::try_from(bytes.len()).expect("fixture length fits u64");
        std::fs::write(&replacement, bytes).expect("same-length replacement bytes");
        std::fs::rename(&replacement, &path).expect("replace named manifest");
        assert_eq!(
            std::fs::metadata(&path)
                .expect("replacement metadata")
                .len(),
            before,
            "the replacement preserves manifest length"
        );

        let why = stale
            .append_exact(Receipt {
                identity: [11; 32],
                frontier_rows: 34,
                trade_rows: 55,
                direction: costs::fill::Direction::Long,
                trade_policy: TradePolicy::ChosenGridV1,
            })
            .expect_err("the stale descriptor must not append after path replacement");
        assert!(
            why.contains("no longer names the opened receipt file"),
            "{why}"
        );
        assert!(why.contains("replacement or path swap"), "{why}");
        assert_eq!(
            std::fs::metadata(&path).expect("refused metadata").len(),
            before,
            "the replacement path receives no stale append"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_header_stride_version_and_reserve_are_exact() {
        let root = root("shape");
        let _ = std::fs::remove_dir_all(&root);
        let mut store = Receipts::open(&root).expect("store");
        store
            .append_exact(Receipt {
                identity: [5; 32],
                frontier_rows: 8,
                trade_rows: 13,
                direction: costs::fill::Direction::Short,
                trade_policy: TradePolicy::ChosenGridV1,
            })
            .expect("receipt");
        drop(store);
        let bytes = std::fs::read(Receipts::path(&root)).expect("bytes");
        assert_eq!(bytes.len(), 16 + 64);
        assert_eq!(bytes.get(..8), Some(&b"BRUTEXRC"[..]));
        assert_eq!(bytes.get(8..12), Some(&2_u32.to_le_bytes()[..]));
        assert_eq!(bytes.get(12..16), Some(&[0_u8; 4][..]));

        let mut bad_version = bytes.clone();
        bad_version
            .get_mut(8..12)
            .expect("version slot")
            .copy_from_slice(&3_u32.to_le_bytes());
        std::fs::write(Receipts::path(&root), &bad_version).expect("version fixture");
        assert!(
            Receipts::open_read(&root)
                .expect_err("unknown version")
                .contains("version 3")
        );

        let mut bad_reserve = bytes;
        *bad_reserve.get_mut(12).expect("reserve byte") = 1;
        std::fs::write(Receipts::path(&root), bad_reserve).expect("reserve fixture");
        assert!(
            Receipts::open_read(&root)
                .expect_err("assigned reserve")
                .contains("reserved")
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn an_intact_legacy_receipt_names_its_version_before_this_versions_stride() {
        let root = root("legacy-v1");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("results")).expect("results directory");
        let mut bytes = vec![0_u8; 16 + 56];
        bytes
            .get_mut(..8)
            .expect("magic slot")
            .copy_from_slice(b"BRUTEXRC");
        bytes
            .get_mut(8..12)
            .expect("version slot")
            .copy_from_slice(&1_u32.to_le_bytes());
        std::fs::write(Receipts::path(&root), bytes).expect("legacy fixture");

        let why = Receipts::open_read(&root).expect_err("legacy receipt must not be widened");
        assert!(why.contains("receipt version 1"), "{why}");
        assert!(why.contains("reads version 2"), "{why}");
        assert!(!why.contains("not a 16-byte header"), "{why}");
        let _ = std::fs::remove_dir_all(&root);
    }
}

//! Dated NSE cash-session eligibility, independent of candle acquisition.
//!
//! Each immutable `NSE_CM_security_ddmmyyyy.csv.gz` has a `.receipt` sibling
//! binding its date, official source URL, compressed length and SHA-256. A
//! receipt is the commit marker: incomplete or unrecognised entries refuse.
//! The receipt records acquisition provenance, not an exchange signature or
//! proof that an auction ran. NSE/CMTR/73845 Annexure A defines the identifier;
//! NSE/CMTR/74466 defines its effective date and exceptional-session exclusions.
//!
//! Sources: <https://nsearchives.nseindia.com/content/circulars/CMTR73845.zip>
//! and <https://nsearchives.nseindia.com/content/circulars/CMTR74466.zip>.

use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::future::Future;
use std::hash::BuildHasher;
use std::io::{ErrorKind, Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use flate2::bufread::GzDecoder;
use sha2::{Digest, Sha256};

use crate::calendar::{self, DayKind, Session};
use crate::cash_auction::{DailyEligibility, LifecycleAssessment, LifecycleEvidence};
use crate::session::{Day, Window};

const MAX_COMPRESSED: usize = 4 * 1024 * 1024;
const MAX_EXPANDED: usize = 32 * 1024 * 1024;
const MAX_RECEIPT: usize = 1024;
const SOURCE_ROOT: &str = "https://nsearchives.nseindia.com//content/cm/";

/// Provenance checked against the cache receipt, not an exchange signature.
/// The date is the acquisition assertion, NOT a date inferred from CSV cells.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LifecycleProvenance {
    /// Exact civil date of the requested master and its receipt.
    pub master_day: Day,
    /// Official URL bound into the receipt.
    pub source_url: String,
    /// Length of the exact compressed payload whose receipt was checked.
    pub compressed_bytes: usize,
    /// SHA-256 of those compressed bytes (not the decompressed CSV).
    pub sha256: [u8; 32],
}

/// One immutable receipt-checked master, parsed/indexed once for many lookups.
/// It cannot be constructed from unbound CSV via this public API.
#[derive(Debug, Clone)]
pub struct VerifiedLifecycleMaster {
    provenance: LifecycleProvenance,
    daily: DailyEligibility,
}

/// Exact-identity snapshot evidence with a source/date/content binding.
/// Even `SnapshotOnly` is NOT point-in-time membership or a gap exemption.
#[derive(Debug, Clone)]
pub struct BoundLifecycleEvidence<'a> {
    /// Verified local receipt details, shared with this loaded master.
    pub provenance: &'a LifecycleProvenance,
    /// Exact symbol, ISIN, native ID and the three source assertions.
    pub identity: LifecycleEvidence<'a>,
    /// Conservative assessment using this master date, never today's date.
    pub assessment: LifecycleAssessment,
}

impl VerifiedLifecycleMaster {
    /// Receipt binding for every lookup made against this loaded snapshot.
    #[must_use]
    pub const fn provenance(&self) -> &LifecycleProvenance {
        &self.provenance
    }

    /// Inspect one exact EQ identity using the already parsed index.
    /// Expected O(1) plus bounded key bytes/three-field assessment. This does
    /// not scan the CSV, read disk or fetch metadata again for each symbol.
    ///
    /// # Errors
    /// Returns `UNVERIFIED` for an absent symbol, ISIN mismatch or invalid key.
    pub fn inspect(&self, symbol: &str, isin: &str) -> Result<BoundLifecycleEvidence<'_>, String> {
        let identity = self.daily.lifecycle(symbol, isin)?;
        Ok(BoundLifecycleEvidence {
            provenance: &self.provenance,
            assessment: identity.metadata.assess(self.provenance.master_day),
            identity,
        })
    }
}

/// Read an exact local master through its existing lock, receipt and gzip/CSV
/// checks. `root` is the session-master directory, not the overall store root.
/// No files are created, installed, repaired or downloaded by this function.
/// Missing/incomplete evidence is a refusal, never an empty successful master.
/// The immutable payload is read and parsed once; reuse the returned index.
///
/// The receipt binds bytes to the acquisition's asserted date/source. It cannot
/// prove the CSV was not mislabelled by its original acquirer or authenticate
/// a historical event from one snapshot. Lifecycle dates are source assertions
/// only: no pre-listing day, interruption or unexplained gap is exempted here.
///
/// # Errors
/// Returns `UNVERIFIED` for absent/busy/nonregular locks, missing or corrupt
/// cache evidence, unsupported receipts, bad gzip/CSV and filesystem failures.
pub fn read_local_lifecycle(
    root: &Path,
    master_day: Day,
) -> Result<VerifiedLifecycleMaster, String> {
    let path = lock_path(root, master_day);
    if !regular_file(&path)? {
        return Err(format!(
            "UNVERIFIED local lifecycle master for {master_day}: existing cache lock is absent; no files created or download attempted"
        ));
    }
    let lock = File::open(&path).map_err(|why| {
        format!(
            "UNVERIFIED cannot open local lifecycle lock {}: {why}",
            path.display()
        )
    })?;
    lock.try_lock_shared().map_err(|why| {
        format!(
            "UNVERIFIED local lifecycle lock {} unavailable: {why}",
            path.display()
        )
    })?;
    let bytes = read_entry(root, master_day)?.ok_or_else(|| {
        format!(
            "UNVERIFIED local lifecycle master for {master_day} is absent; no download attempted"
        )
    })?;
    let daily = decode(&bytes)?;
    Ok(VerifiedLifecycleMaster {
        provenance: LifecycleProvenance {
            master_day,
            source_url: source_url(master_day),
            compressed_bytes: bytes.len(),
            sha256: Sha256::digest(&bytes).into(),
        },
        daily,
    })
}

/// Loads every required dated master, fetching missing entries from NSE.
///
/// Keys are IST civil days as `Day::days_from_epoch()`, not `yyyymmdd`.
/// Days before 2026-08-03 and closed days are omitted. Every other day must be
/// a measured, full calendar session. Calendar and existing-cache validation
/// finish before any fetch. A later fetch failure may leave earlier validated
/// days cached, but never returns an incomplete map. No broker or credential
/// API is involved; downloads are sequential and each has a 30-second timeout.
///
/// # Errors
/// Refuses exceptional/unmeasured dates, invalid or incomplete cache entries,
/// conflicts, transport failures, oversized bodies and malformed gzip/CSV.
pub async fn prepare(
    root: &Path,
    window: Window,
) -> Result<HashMap<u32, DailyEligibility>, String> {
    prepare_with(root, window, download).await
}

/// Acquires eligibility flags for IST dates actually observed in returned rows.
///
/// This DOES NOT attest that a date was open, a full session occurred, or the
/// returned rows are complete. Unknown and exceptional dates are permitted for
/// acquiring this evidence only; calendar audits must remain `UNVERIFIED` and
/// derived output must remain withheld wherever session evidence is missing.
/// No date is inferred from a requested window or a weekday rule. Dates before
/// 2026-08-03 are skipped; remaining dates are sorted and deduplicated.
///
/// `cache` uses `Day::days_from_epoch()` keys and must belong to this `root`.
/// Requested disk entries are receipt-checked and parsed even if already in
/// memory; a memory entry without its disk evidence refuses. All existing
/// entries are validated before any missing master is fetched. Valid entries
/// are reused without downloading or overwriting their files. The map changes
/// only on complete success, although earlier validated downloads may remain
/// on disk after a later failure. No broker or credential API is called here.
///
/// # Errors
/// Refuses invalid/incomplete cache evidence, conflicts, transport failures,
/// oversized bodies and malformed gzip/CSV, using the same bounds as `prepare`.
pub async fn prepare_observed<S: BuildHasher>(
    root: &Path,
    days: &[Day],
    cache: &mut HashMap<u32, DailyEligibility, S>,
) -> Result<(), String> {
    prepare_observed_with(root, days, cache, download).await
}

/// Receipt-check existing evidence for committed source dates without fetching.
/// Missing or corrupt historical metadata refuses rather than replacing it or
/// trusting an in-memory entry. The cache changes only on complete success.
///
/// # Errors
/// The same local validation refusals as `prepare_observed`, plus missing files.
pub async fn prepare_local_observed<S: BuildHasher>(
    root: &Path,
    days: &[Day],
    cache: &mut HashMap<u32, DailyEligibility, S>,
) -> Result<(), String> {
    prepare_observed_with(root, days, cache, |_| async {
        Err(
            "required committed-source eligibility is missing locally; no download attempted"
                .to_owned(),
        )
    })
    .await
}

/// Validates and immutably installs an already acquired, dated NSE gzip file.
///
/// The caller must supply bytes acquired for `day` from that day's official
/// URL. The CSV has no trade-date column: this function cannot discover a
/// mislabelled acquisition by inspecting its rows. The receipt records the
/// caller's date assertion and binds it to the bytes; `UpdDt` is not used as
/// a substitute trade date. Existing byte-identical, valid entries are safe
/// to reinstall. Unknown metadata and conflicts are never repaired in place.
/// Installation retains source evidence even when the calendar is unmeasured;
/// only `prepare` decides whether that date may enter a requested schedule.
///
/// # Errors
/// Refuses pre-CAS dates, invalid gzip/CSV, either size limit,
/// malformed receipts, conflicting entries and filesystem failures.
pub fn install(root: &Path, day: Day, compressed: &[u8]) -> Result<(), String> {
    install_and_read(root, day, compressed).map(|_| ())
}

fn is_required(day: Day, kind: DayKind) -> Result<bool, String> {
    if !crate::vendor::cash_auction_eligibility_required(day) {
        return Ok(false);
    }
    match kind {
        DayKind::Closed => Ok(false),
        DayKind::Open(session) if session == Session::full() => Ok(true),
        _ => Err(format!(
            "UNVERIFIED NSE cash session on {day}: exceptional or unmeasured calendar day ({kind:?})"
        )),
    }
}

fn required_days(window: Window) -> Result<Vec<Day>, String> {
    let mut days = Vec::new();
    for epoch in window.from().days_from_epoch()..=window.to().days_from_epoch() {
        let day = Day::from_days(epoch).map_err(|why| why.to_string())?;
        if is_required(day, calendar::kind_of(i64::from(epoch)))? {
            days.push(day);
        }
    }
    Ok(days)
}

async fn prepare_with<F, Fut>(
    root: &Path,
    window: Window,
    mut fetch: F,
) -> Result<HashMap<u32, DailyEligibility>, String>
where
    F: FnMut(String) -> Fut,
    Fut: Future<Output = Result<Vec<u8>, String>>,
{
    let days = required_days(window)?;
    let mut result = HashMap::new();
    let mut missing = Vec::new();
    for day in days {
        let _lock = lock_day(root, day)?;
        match read_entry(root, day)? {
            Some(bytes) => {
                result.insert(day.days_from_epoch(), decode(&bytes)?);
            }
            None => missing.push(day),
        }
    }
    for day in missing {
        let url = source_url(day);
        let bytes = fetch(url.clone())
            .await
            .map_err(|why| format!("missing NSE cash eligibility for {day} ({url}): {why}"))?;
        let eligibility = install_and_read(root, day, &bytes)?;
        result.insert(day.days_from_epoch(), eligibility);
    }
    Ok(result)
}

async fn prepare_observed_with<S, F, Fut>(
    root: &Path,
    days: &[Day],
    cache: &mut HashMap<u32, DailyEligibility, S>,
    mut fetch: F,
) -> Result<(), String>
where
    S: BuildHasher,
    F: FnMut(String) -> Fut,
    Fut: Future<Output = Result<Vec<u8>, String>>,
{
    let mut days: Vec<_> = days
        .iter()
        .copied()
        .filter(|day| crate::vendor::cash_auction_eligibility_required(*day))
        .collect();
    days.sort_unstable();
    days.dedup();
    let mut pending = HashMap::new();
    let mut missing = Vec::new();
    for day in days {
        let _lock = lock_day(root, day)?;
        match read_entry(root, day)? {
            Some(bytes) => {
                pending.insert(day.days_from_epoch(), decode(&bytes)?);
            }
            None if cache.contains_key(&day.days_from_epoch()) => {
                return Err(format!(
                    "UNVERIFIED in-memory cash eligibility for {day} has no receipted evidence in {}",
                    root.display()
                ));
            }
            None => missing.push(day),
        }
    }
    for day in missing {
        let url = source_url(day);
        let bytes = fetch(url.clone())
            .await
            .map_err(|why| format!("missing NSE cash eligibility for {day} ({url}): {why}"))?;
        pending.insert(day.days_from_epoch(), install_and_read(root, day, &bytes)?);
    }
    cache.extend(pending);
    Ok(())
}

fn filename(day: Day) -> String {
    format!(
        "NSE_CM_security_{:02}{:02}{:04}.csv.gz",
        day.day(),
        day.month(),
        day.year()
    )
}

fn source_url(day: Day) -> String {
    format!("{SOURCE_ROOT}{}", filename(day))
}

fn paths(root: &Path, day: Day) -> (PathBuf, PathBuf) {
    let name = filename(day);
    (root.join(&name), root.join(format!("{name}.receipt")))
}

fn receipt(day: Day, bytes: &[u8]) -> String {
    format!(
        "brutex-nse-cash-session-v1\ndate={day}\nsource={}\ncompressed_bytes={}\nsha256={:x}\n",
        source_url(day),
        bytes.len(),
        Sha256::digest(bytes)
    )
}

// Persistent advisory locks serialize cooperating readers and installers.
// A busy entry refuses promptly; no async task waits on a filesystem lock.
fn lock_path(root: &Path, day: Day) -> PathBuf {
    root.join(format!(".{}.lock", filename(day)))
}

fn lock_day(root: &Path, day: Day) -> Result<File, String> {
    fs::create_dir_all(root)
        .map_err(|why| format!("cannot create cash-session cache {}: {why}", root.display()))?;
    let path = lock_path(root, day);
    let _ = regular_file(&path)?;
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&path)
        .map_err(|why| format!("cannot open cache lock {}: {why}", path.display()))?;
    file.try_lock().map_err(|why| {
        format!(
            "cash-session cache lock {} unavailable: {why}",
            path.display()
        )
    })?;
    Ok(file)
}

fn regular_file(path: &Path) -> Result<bool, String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_file() => Ok(true),
        Ok(_) => Err(format!(
            "UNVERIFIED cache path {} is not a regular file",
            path.display()
        )),
        Err(why) if why.kind() == ErrorKind::NotFound => Ok(false),
        Err(why) => Err(format!("cannot inspect {}: {why}", path.display())),
    }
}

fn read_limited(path: &Path, limit: usize) -> Result<Vec<u8>, String> {
    let file = File::open(path).map_err(|why| format!("cannot open {}: {why}", path.display()))?;
    let metadata = file
        .metadata()
        .map_err(|why| format!("cannot stat {}: {why}", path.display()))?;
    if !metadata.is_file() || metadata.len() > limit as u64 {
        return Err(format!(
            "{} is not a regular file within {limit} bytes",
            path.display()
        ));
    }
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|why| format!("cannot read {}: {why}", path.display()))?;
    if bytes.len() > limit {
        return Err(format!("{} exceeds {limit} bytes", path.display()));
    }
    Ok(bytes)
}

// Called only while holding this day's advisory lock. Neither an unreceipted
// payload nor an orphan receipt is interpreted as a cache miss.
fn read_entry(root: &Path, day: Day) -> Result<Option<Vec<u8>>, String> {
    let (payload, metadata) = paths(root, day);
    match (regular_file(&payload)?, regular_file(&metadata)?) {
        (false, false) => return Ok(None),
        (true, true) => {}
        _ => {
            return Err(format!(
                "UNVERIFIED incomplete cash-session cache for {day}: payload and receipt must both exist"
            ));
        }
    }
    let bytes = read_limited(&payload, MAX_COMPRESSED)?;
    let recorded = read_limited(&metadata, MAX_RECEIPT)?;
    if recorded != receipt(day, &bytes).as_bytes() {
        return Err(format!(
            "UNVERIFIED cash-session receipt for {day}: unknown format or date/source/length/SHA-256 mismatch; {} retained",
            metadata.display()
        ));
    }
    Ok(Some(bytes))
}

fn decode(compressed: &[u8]) -> Result<DailyEligibility, String> {
    if compressed.len() > MAX_COMPRESSED {
        return Err(format!(
            "NSE cash master exceeds {MAX_COMPRESSED} compressed bytes"
        ));
    }
    let mut decoder = GzDecoder::new(compressed);
    let mut expanded = Vec::new();
    decoder
        .by_ref()
        .take(MAX_EXPANDED as u64 + 1)
        .read_to_end(&mut expanded)
        .map_err(|why| format!("invalid NSE cash master gzip: {why}"))?;
    if expanded.len() > MAX_EXPANDED {
        return Err(format!(
            "NSE cash master exceeds {MAX_EXPANDED} expanded bytes"
        ));
    }
    if !decoder.into_inner().is_empty() {
        return Err(
            "invalid NSE cash master gzip: trailing bytes or additional gzip members".to_owned(),
        );
    }
    let csv = std::str::from_utf8(&expanded)
        .map_err(|why| format!("NSE cash master is not UTF-8: {why}"))?;
    DailyEligibility::parse(csv).map_err(|why| format!("invalid NSE cash eligibility CSV: {why}"))
}

fn install_and_read(root: &Path, day: Day, bytes: &[u8]) -> Result<DailyEligibility, String> {
    if !crate::vendor::cash_auction_eligibility_required(day) {
        return Err(format!(
            "UNVERIFIED {day} precedes the 2026-08-03 cash-session identifier"
        ));
    }
    // Validation precedes even creating the cache directory.
    let eligibility = decode(bytes)?;
    let _lock = lock_day(root, day)?;
    if let Some(existing) = read_entry(root, day)? {
        if existing != bytes {
            return Err(format!(
                "conflicting NSE cash master for {day}; existing cache retained"
            ));
        }
        return Ok(eligibility);
    }
    let (payload, metadata) = paths(root, day);
    write_new(&payload, bytes)?;
    // The receipt is last. Failed or interrupted publication leaves evidence
    // in place and cannot become an implicit refetch/overwrite on the next run.
    write_new(&metadata, receipt(day, bytes).as_bytes())?;
    File::open(root).and_then(|directory| directory.sync_all())
        .map_err(|why| format!("cache for {day} is visible but directory sync failed: {why}; crash durability UNVERIFIED"))?;
    Ok(eligibility)
}

fn write_new(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|why| {
            format!(
                "cannot create {}; no existing file was overwritten: {why}",
                path.display()
            )
        })?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|why| {
            format!(
                "incomplete publication at {} retained: {why}",
                path.display()
            )
        })
}

async fn download(url: String) -> Result<Vec<u8>, String> {
    crate::ensure_tls_provider();
    let client = reqwest::Client::builder()
        .http1_only()
        .no_gzip()
        .no_brotli()
        .no_deflate()
        .no_zstd()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30))
        .user_agent("Mozilla/5.0 (compatible; brutex/1.0; public-metadata)")
        .build()
        .map_err(|why| format!("cannot build NSE public metadata client: {why}"))?;
    let mut response = client
        .get(&url)
        .header(reqwest::header::ACCEPT_ENCODING, "identity")
        .header(
            reqwest::header::REFERER,
            "https://www.nseindia.com/all-reports",
        )
        .send()
        .await
        .map_err(|why| format!("{url}: {why:#}"))?;
    if response.status() != reqwest::StatusCode::OK {
        return Err(format!(
            "{url} answered HTTP {}; redirects and partial responses are refused",
            response.status()
        ));
    }
    if response
        .content_length()
        .is_some_and(|length| length > MAX_COMPRESSED as u64)
    {
        return Err(format!(
            "{url} declares more than {MAX_COMPRESSED} compressed bytes"
        ));
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|why| format!("{url} body: {why:#}"))?
    {
        if bytes.len().saturating_add(chunk.len()) > MAX_COMPRESSED {
            return Err(format!("{url} exceeds {MAX_COMPRESSED} compressed bytes"));
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "tests fail immediately when fixture setup fails"
)]
mod tests {
    use super::*;
    use flate2::{Compression, write::GzEncoder};
    use std::sync::atomic::{AtomicU64, Ordering};

    const CSV: &str = "FinInstrmId,TckrSymb,SctySrs,ISIN,ElgbltyClsgAuctnSsn\n1363,HINDALCO,EQ,INE038A01020,1\n16921,20MICRONS,EQ,INE144J01027,0\n";

    struct Temp(PathBuf);
    impl Temp {
        fn new() -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "brutex-cash-cache-test-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).expect("unique test directory");
            Self(path)
        }
    }
    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn day(date: u8) -> Day {
        Day::new(2026, 8, date).expect("test date")
    }
    fn window(from: u8, to: u8) -> Window {
        Window::new(day(from), day(to)).expect("test window")
    }
    fn gzip(text: &[u8]) -> Vec<u8> {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::fast());
        encoder.write_all(text).expect("compress fixture");
        encoder.finish().expect("finish gzip")
    }
    fn seed(temp: &Temp, date: u8) -> Vec<u8> {
        let bytes = gzip(CSV.as_bytes());
        install(&temp.0, day(date), &bytes).expect("valid fixture installs");
        bytes
    }

    const LIFECYCLE_CSV: &str = "FinInstrmId,TckrSymb,SctySrs,ISIN,ElgbltyClsgAuctnSsn,ListgDt,RmvlDt,RadmssnDt\n2029,IRFC,EQ,INE053F01010,1,1296345600,0,0\n";

    #[test]
    fn local_lifecycle_binds_exact_identity_date_source_length_and_digest() {
        use crate::cash_auction::{LifecycleDate, LifecycleQuality};

        let temp = Temp::new();
        let bytes = gzip(LIFECYCLE_CSV.as_bytes());
        install(&temp.0, day(3), &bytes).expect("fixture receipt installs");
        let master = read_local_lifecycle(&temp.0, day(3)).expect("local verified master");
        let expected_digest: [u8; 32] = Sha256::digest(&bytes).into();
        assert_eq!(
            master.provenance(),
            &LifecycleProvenance {
                master_day: day(3),
                source_url:
                    "https://nsearchives.nseindia.com//content/cm/NSE_CM_security_03082026.csv.gz"
                        .to_owned(),
                compressed_bytes: bytes.len(),
                sha256: expected_digest,
            }
        );
        let report = master
            .inspect("IRFC", "INE053F01010")
            .expect("exact EQ pair");
        assert_eq!(report.provenance, master.provenance());
        assert_eq!(report.identity.symbol, "IRFC");
        assert_eq!(report.identity.isin, "INE053F01010");
        assert_eq!(report.identity.native_id, "2029");
        assert_eq!(
            report.identity.metadata.listing.date(),
            Some(Day::new(2021, 1, 29).expect("decoded assertion"))
        );
        assert_eq!(report.identity.metadata.removal, LifecycleDate::Zero);
        assert_eq!(report.identity.metadata.readmission, LifecycleDate::Zero);
        assert_eq!(report.assessment.quality, LifecycleQuality::Partial);
        assert!(
            master
                .inspect("IRFC", "WRONG")
                .expect_err("no ISIN borrowing")
                .contains("ISIN mismatch")
        );
        assert!(master.inspect("ALIAS", "INE053F01010").is_err());
        assert!(master.inspect("irfc", "INE053F01010").is_err());
        assert!(master.inspect("IRFC", "INE053F01010 ").is_err());
        assert_eq!(
            master
                .clone()
                .inspect("IRFC", "INE053F01010")
                .expect("same immutable snapshot")
                .assessment,
            report.assessment
        );

        // Reusing the already bound snapshot performs no hidden per-symbol IO.
        let (payload, metadata) = paths(&temp.0, day(3));
        fs::remove_file(payload).expect("remove owned test payload");
        fs::remove_file(metadata).expect("remove owned test receipt");
        assert_eq!(
            master
                .inspect("IRFC", "INE053F01010")
                .expect("in-memory bound snapshot")
                .provenance
                .sha256,
            expected_digest
        );
        assert!(read_local_lifecycle(&temp.0, day(3)).is_err());
    }

    #[test]
    fn local_lifecycle_old_cas_schema_stays_explicitly_unavailable() {
        use crate::cash_auction::{LifecycleDate, LifecycleQuality};

        let temp = Temp::new();
        seed(&temp, 3);
        let master = read_local_lifecycle(&temp.0, day(3)).expect("old CAS schema remains valid");
        let report = master
            .inspect("HINDALCO", "INE038A01020")
            .expect("exact fixture pair");
        assert_eq!(report.assessment.quality, LifecycleQuality::Unavailable);
        assert_eq!(
            report.identity.metadata.listing,
            LifecycleDate::HeaderAbsent
        );
        assert_eq!(
            report.identity.metadata.removal,
            LifecycleDate::HeaderAbsent
        );
        assert_eq!(
            report.identity.metadata.readmission,
            LifecycleDate::HeaderAbsent
        );
        assert_eq!(report.assessment.issues.len(), 3);
        assert_eq!(
            master.daily.eligibility("HINDALCO", "INE038A01020"),
            Ok(true)
        );
    }

    #[test]
    fn local_lifecycle_is_read_only_when_root_or_evidence_is_missing() {
        let temp = Temp::new();
        let missing = temp.0.join("must-not-be-created");
        let why = read_local_lifecycle(&missing, day(3)).expect_err("no cache creation");
        assert!(why.contains("UNVERIFIED") && why.contains("no files created"));
        assert!(!missing.exists());
        let lock = lock_day(&temp.0, day(3)).expect("test lock only");
        drop(lock);
        let why = read_local_lifecycle(&temp.0, day(3)).expect_err("no payload or receipt");
        assert!(why.contains("absent") && why.contains("no download attempted"));
        let (payload, metadata) = paths(&temp.0, day(3));
        assert!(!payload.exists() && !metadata.exists());
        let bytes = gzip(LIFECYCLE_CSV.as_bytes());
        fs::write(&payload, &bytes).expect("owned orphan fixture");
        assert!(
            read_local_lifecycle(&temp.0, day(3))
                .expect_err("orphan payload")
                .contains("incomplete")
        );
        assert_eq!(fs::read(&payload).expect("orphan untouched"), bytes);
        assert!(!metadata.exists());
        fs::remove_file(&payload).expect("remove owned fixture");
        fs::write(&metadata, receipt(day(3), &bytes)).expect("owned orphan receipt");
        assert!(
            read_local_lifecycle(&temp.0, day(3))
                .expect_err("orphan receipt")
                .contains("incomplete")
        );
        assert!(!payload.exists());
    }

    #[test]
    fn local_lifecycle_wrong_date_url_length_digest_and_version_all_refuse() {
        let temp = Temp::new();
        let bytes = gzip(LIFECYCLE_CSV.as_bytes());
        install(&temp.0, day(3), &bytes).expect("valid fixture");
        let (payload, metadata) = paths(&temp.0, day(3));
        let valid = receipt(day(3), &bytes);
        for invalid in [
            receipt(day(4), &bytes),
            valid.replace(
                "https://nsearchives.nseindia.com//",
                "https://untrusted.invalid/",
            ),
            valid.replace(
                &format!("compressed_bytes={}", bytes.len()),
                "compressed_bytes=1",
            ),
            receipt(day(3), b"different source bytes"),
            valid.replace("brutex-nse-cash-session-v1", "brutex-nse-cash-session-v2"),
        ] {
            fs::write(&metadata, &invalid).expect("damage owned receipt");
            let why = read_local_lifecycle(&temp.0, day(3)).expect_err("binding mismatch");
            assert!(
                why.contains("UNVERIFIED") && why.contains("receipt"),
                "{why}"
            );
            assert_eq!(fs::read(&payload).expect("payload preserved"), bytes);
            assert_eq!(
                fs::read_to_string(&metadata).expect("receipt not repaired"),
                invalid
            );
        }
        fs::write(metadata, valid).expect("restore owned fixture");
        assert!(read_local_lifecycle(&temp.0, day(3)).is_ok());
        assert!(read_local_lifecycle(&temp.0, day(4)).is_err());
    }

    #[test]
    fn local_lifecycle_busy_or_nonregular_lock_refuses_without_replacing_evidence() {
        let temp = Temp::new();
        let bytes = seed(&temp, 3);
        let held = lock_day(&temp.0, day(3)).expect("exclusive publisher lock");
        assert!(
            read_local_lifecycle(&temp.0, day(3))
                .expect_err("busy lock")
                .contains("unavailable")
        );
        drop(held);
        assert!(read_local_lifecycle(&temp.0, day(3)).is_ok());
        let lock = lock_path(&temp.0, day(3));
        fs::remove_file(&lock).expect("owned test lock");
        fs::create_dir(&lock).expect("nonregular lock fixture");
        assert!(
            read_local_lifecycle(&temp.0, day(3))
                .expect_err("nonregular lock")
                .contains("not a regular file")
        );
        assert_eq!(
            fs::read(paths(&temp.0, day(3)).0).expect("payload unchanged"),
            bytes
        );
    }

    #[cfg(unix)]
    #[test]
    fn local_lifecycle_refuses_symlinked_lock_payload_and_receipt() {
        for component in 0..3 {
            let temp = Temp::new();
            seed(&temp, 3);
            let (payload, metadata) = paths(&temp.0, day(3));
            let selected = match component {
                0 => lock_path(&temp.0, day(3)),
                1 => payload,
                _ => metadata,
            };
            let original = fs::read(&selected).expect("owned fixture bytes");
            let elsewhere = temp.0.join("elsewhere");
            fs::rename(&selected, &elsewhere).expect("relocate owned fixture");
            std::os::unix::fs::symlink(&elsewhere, &selected).expect("symlink fixture");
            let why = read_local_lifecycle(&temp.0, day(3)).expect_err("no symbolic substitution");
            assert!(why.contains("not a regular file"), "{why}");
            assert_eq!(fs::read(elsewhere).expect("original retained"), original);
        }
    }

    #[test]
    fn local_lifecycle_matching_receipt_cannot_hide_bad_gzip_csv_or_identity_conflicts() {
        let temp = Temp::new();
        let held = lock_day(&temp.0, day(3)).expect("fixture lock");
        drop(held);
        let (payload, metadata) = paths(&temp.0, day(3));
        let conflict = format!("{LIFECYCLE_CSV}2030,IRFC,EQ,INE053F01010,1,1296345600,0,0\n");
        for bytes in [
            b"invalid gzip".to_vec(),
            gzip(b"wrong CSV\n"),
            gzip(conflict.as_bytes()),
        ] {
            fs::write(&payload, &bytes).expect("owned invalid fixture");
            fs::write(&metadata, receipt(day(3), &bytes)).expect("matching acquisition receipt");
            assert!(read_local_lifecycle(&temp.0, day(3)).is_err());
            assert_eq!(
                fs::read(&payload).expect("invalid evidence retained"),
                bytes
            );
        }
    }

    #[test]
    fn local_lifecycle_qualifies_invalid_conflicting_and_future_metadata_without_hiding_it() {
        use crate::cash_auction::{LifecycleField, LifecycleIssue, LifecycleQuality};

        for (csv, quality, issue) in [
            (
                LIFECYCLE_CSV.replace("1296345600", "not-a-date"),
                LifecycleQuality::Unverified,
                LifecycleIssue::Invalid(LifecycleField::Listing),
            ),
            (
                format!("{LIFECYCLE_CSV}2029,IRFC,EQ,INE053F01010,1,1250640000,0,0\n"),
                LifecycleQuality::Contradictory,
                LifecycleIssue::Conflicting(LifecycleField::Listing),
            ),
            (
                LIFECYCLE_CSV.replace("1296345600", "1577923200"),
                LifecycleQuality::Unverified,
                LifecycleIssue::AfterMaster(LifecycleField::Listing),
            ),
        ] {
            let temp = Temp::new();
            install(&temp.0, day(3), &gzip(csv.as_bytes()))
                .expect("CAS data still structurally valid");
            let master =
                read_local_lifecycle(&temp.0, day(3)).expect("inspect unverified metadata");
            let report = master
                .inspect("IRFC", "INE053F01010")
                .expect("exact identity");
            assert_eq!(report.assessment.quality, quality);
            assert!(report.assessment.issues.contains(&issue));
            assert_eq!(master.daily.eligibility("IRFC", "INE053F01010"), Ok(true));
        }
    }

    #[test]
    #[ignore = "requires BRUTEX_NSE_CASH_SAMPLE_DIR with the receipted 2026-09-04 master; read-only, no network"]
    fn actual_receipted_lifecycle_snapshot_never_claims_complete_history() -> Result<(), String> {
        use crate::cash_auction::{LifecycleDate, LifecycleQuality};
        use brutex_core::universe::{FNO_UNDERLYINGS, NTM_INDEX, nse_isin};

        let root = std::env::var_os("BRUTEX_NSE_CASH_SAMPLE_DIR")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .ok_or("BRUTEX_NSE_CASH_SAMPLE_DIR must name the receipted master directory")?;
        let date = Day::new(2026, 9, 4).map_err(|why| why.to_string())?;
        let master = read_local_lifecycle(&root, date)?;
        assert_eq!(master.provenance().master_day, date);
        assert_eq!(
            master.provenance().source_url,
            "https://nsearchives.nseindia.com//content/cm/NSE_CM_security_04092026.csv.gz"
        );
        let mut count = 0;
        let mut unavailable = 0;
        let mut partial = 0;
        for symbol in FNO_UNDERLYINGS
            .into_iter()
            .filter(|symbol| NTM_INDEX.contains(symbol))
        {
            let isin = nse_isin(symbol).expect("current cash table identity");
            let report = master.inspect(symbol, isin.as_str())?;
            assert_eq!(
                report.identity.metadata.removal,
                LifecycleDate::Zero,
                "{symbol}"
            );
            assert_eq!(
                report.identity.metadata.readmission,
                LifecycleDate::Zero,
                "{symbol}"
            );
            match report.identity.metadata.listing {
                LifecycleDate::Zero => {
                    assert_eq!(
                        report.assessment.quality,
                        LifecycleQuality::Unavailable,
                        "{symbol}"
                    );
                    unavailable += 1;
                }
                LifecycleDate::Date { .. } => {
                    assert_eq!(
                        report.assessment.quality,
                        LifecycleQuality::Partial,
                        "{symbol}"
                    );
                    partial += 1;
                }
                ref other => {
                    return Err(format!(
                        "unexpected actual listing evidence for {symbol}: {other:?}"
                    ));
                }
            }
            count += 1;
        }
        assert_eq!(count, 208);
        assert_eq!(partial + unavailable, count);
        assert!(partial > 0 && unavailable > 0);
        assert_eq!(
            master
                .inspect("ADANIENT", "INE423A01024")?
                .identity
                .metadata
                .listing,
            LifecycleDate::Zero
        );
        for (symbol, isin, expected) in [
            ("IRFC", "INE053F01010", Day::new(2021, 1, 29)),
            ("FORCEMOT", "INE451A01017", Day::new(2019, 8, 19)),
            ("RELIANCE", "INE002A01018", Day::new(1995, 11, 29)),
        ] {
            let report = master.inspect(symbol, isin)?;
            assert_eq!(
                report.identity.metadata.listing.date(),
                Some(expected.expect("fixture date"))
            );
        }
        eprintln!(
            "2026-09-04: {count} exact cash identities, {partial} Partial and {unavailable} Unavailable lifecycle snapshots; no historical exemption inferred"
        );
        Ok(())
    }

    #[tokio::test]
    async fn cached_window_never_fetches_and_uses_epoch_keys() {
        let temp = Temp::new();
        seed(&temp, 3);
        seed(&temp, 4);
        let result = prepare_with(&temp.0, window(1, 4), |_| async {
            Err("network must not be used".to_owned())
        })
        .await
        .expect("fully cached window");
        assert_eq!(result.len(), 2);
        assert!(result.contains_key(&day(3).days_from_epoch()));
        assert!(result.contains_key(&day(4).days_from_epoch()));
        let first = result.get(&day(3).days_from_epoch()).expect("first date");
        assert!(
            first
                .eligibility("HINDALCO", "INE038A01020")
                .expect("explicit one")
        );
        assert!(
            !first
                .eligibility("20MICRONS", "INE144J01027")
                .expect("explicit zero")
        );
        assert!(
            required_days(window(8, 9))
                .expect("closed weekend")
                .is_empty()
        );
    }

    #[tokio::test]
    async fn local_observed_requires_receipts_and_never_acquires_missing_history() {
        let temp = Temp::new();
        seed(&temp, 3);
        let mut cache = HashMap::new();
        prepare_local_observed(&temp.0, &[day(3)], &mut cache)
            .await
            .expect("receipted history");
        assert!(
            cache
                .get(&day(3).days_from_epoch())
                .expect("dated master")
                .eligibility("HINDALCO", "INE038A01020")
                .expect("exact identity")
        );
        assert!(
            cache
                .get(&day(3).days_from_epoch())
                .expect("dated master")
                .eligibility("HINDALCO", "INE144J01027")
                .is_err()
        );
        let why = prepare_local_observed(&temp.0, &[day(3), day(4)], &mut cache)
            .await
            .expect_err("historical evidence cannot be downloaded implicitly");
        assert!(why.contains("no download attempted"), "{why}");
        assert_eq!(cache.len(), 1);
        let (payload, receipt) = paths(&temp.0, day(4));
        assert!(!payload.exists());
        assert!(!receipt.exists());
    }

    #[tokio::test]
    async fn missing_day_fetches_only_its_official_url() {
        let temp = Temp::new();
        seed(&temp, 3);
        let mut requested = Vec::new();
        let result = prepare_with(&temp.0, window(3, 4), |url| {
            requested.push(url);
            std::future::ready(Ok(gzip(CSV.as_bytes())))
        })
        .await
        .expect("missing day installs");
        assert_eq!(
            requested,
            vec!["https://nsearchives.nseindia.com//content/cm/NSE_CM_security_04082026.csv.gz"]
        );
        assert_eq!(result.len(), 2);
        assert!(
            read_entry(&temp.0, day(4))
                .expect("receipt validates")
                .is_some()
        );
    }

    #[tokio::test]
    async fn observed_unknown_sep11_loads_flag_without_attesting_a_session() {
        let temp = Temp::new();
        let date = Day::new(2026, 9, 11).expect("unattested fixture date");
        let epoch = date.days_from_epoch();
        assert_eq!(calendar::kind_of(i64::from(epoch)), DayKind::Unmeasured);
        let mut cache = HashMap::new();
        let mut requested = Vec::new();
        prepare_observed_with(&temp.0, &[date], &mut cache, |url| {
            requested.push(url);
            std::future::ready(Ok(gzip(CSV.as_bytes())))
        })
        .await
        .expect("an observed date may acquire eligibility evidence");
        assert_eq!(
            requested,
            ["https://nsearchives.nseindia.com//content/cm/NSE_CM_security_11092026.csv.gz"]
        );
        assert_eq!(cache.len(), 1);
        assert!(
            cache
                .get(&epoch)
                .expect("observed date retained")
                .eligibility("HINDALCO", "INE038A01020")
                .expect("fixture's explicit one")
        );
        assert_eq!(calendar::kind_of(i64::from(epoch)), DayKind::Unmeasured);
        assert!(required_days(Window::new(date, date).expect("fixture window")).is_err());
    }

    #[tokio::test]
    async fn observed_dates_sort_dedup_and_reuse_without_a_weekend_filter() {
        let temp = Temp::new();
        let mut cache = HashMap::new();
        let dates = [day(23), day(8), day(22), day(2), day(23), day(8)];
        assert_eq!(
            calendar::kind_of(i64::from(day(8).days_from_epoch())),
            DayKind::Closed
        );
        let mut requested = Vec::new();
        prepare_observed_with(&temp.0, &dates, &mut cache, |url| {
            requested.push(url);
            std::future::ready(Ok(gzip(CSV.as_bytes())))
        })
        .await
        .expect("only observed post-CAS dates are acquired");
        assert_eq!(
            requested,
            [source_url(day(8)), source_url(day(22)), source_url(day(23))]
        );
        assert_eq!(cache.len(), 3);
        assert!(!cache.contains_key(&day(2).days_from_epoch()));
        assert!(!paths(&temp.0, day(2)).0.exists());
        prepare_observed_with(&temp.0, &dates, &mut cache, |_| async {
            Err("cached dates must never fetch again".to_owned())
        })
        .await
        .expect("duplicate dates and calls reuse receipted files");
        assert_eq!(cache.len(), 3);
        assert_eq!(
            calendar::kind_of(i64::from(day(8).days_from_epoch())),
            DayKind::Closed
        );
    }

    #[tokio::test]
    async fn observed_empty_and_pre_cas_dates_have_no_io() {
        let temp = Temp::new();
        let root = temp.0.join("unused");
        let mut cache = HashMap::new();
        for dates in [Vec::new(), vec![day(1), day(2), day(2)]] {
            prepare_observed_with(&root, &dates, &mut cache, |_| async {
                Err("empty input must never fetch".to_owned())
            })
            .await
            .expect("no evidence needed");
            assert!(cache.is_empty());
            assert!(!root.exists());
        }
    }

    #[tokio::test]
    async fn observed_warm_cache_still_checks_receipts_and_csv_before_fetching() {
        let temp = Temp::new();
        let bytes = seed(&temp, 23);
        let mut cache = HashMap::from([(
            day(23).days_from_epoch(),
            decode(&bytes).expect("initial validated cache"),
        )]);
        let (payload, metadata) = paths(&temp.0, day(23));
        let invalid_csv = gzip(b"SYMBOL,FINAL VOLUME\nHINDALCO,42\n");
        for (compressed, recorded, reason) in [
            (bytes.clone(), receipt(day(22), &bytes), "receipt"),
            (bytes.clone(), "unknown-format\n".to_owned(), "receipt"),
            (invalid_csv.clone(), receipt(day(23), &invalid_csv), "CSV"),
        ] {
            fs::write(&payload, compressed).expect("damage owned payload");
            fs::write(&metadata, recorded).expect("damage owned receipt");
            let mut fetched = false;
            let error = prepare_observed_with(&temp.0, &[day(22), day(23)], &mut cache, |_| {
                fetched = true;
                std::future::ready(Err("unexpected network".to_owned()))
            })
            .await
            .expect_err("memory must not mask invalid disk evidence");
            assert!(error.contains(reason), "{error}");
            assert!(!fetched);
            assert_eq!(cache.len(), 1);
            assert!(
                cache
                    .get(&day(23).days_from_epoch())
                    .expect("previous date retained")
                    .eligibility("HINDALCO", "INE038A01020")
                    .expect("unchanged memory entry")
            );
        }
    }

    #[tokio::test]
    async fn observed_unreceipted_memory_refuses_and_late_failure_is_atomic() {
        let temp = Temp::new();
        let bytes = seed(&temp, 3);
        let mut cache = HashMap::from([(
            day(3).days_from_epoch(),
            decode(&bytes).expect("initial validated cache"),
        )]);
        let different_root = temp.0.join("other");
        let mut fetched = false;
        let error = prepare_observed_with(&different_root, &[day(3)], &mut cache, |_| {
            fetched = true;
            std::future::ready(Err("unexpected network".to_owned()))
        })
        .await
        .expect_err("wrong cache root refuses");
        assert!(error.contains("no receipted evidence"));
        assert!(!fetched);
        let mut requested = Vec::new();
        let error = prepare_observed_with(&temp.0, &[day(23), day(22)], &mut cache, |url| {
            let result = if url == source_url(day(22)) {
                Ok(bytes.clone())
            } else {
                Err("HTTP 404".to_owned())
            };
            requested.push(url);
            std::future::ready(result)
        })
        .await
        .expect_err("missing later date refuses the whole map update");
        assert!(error.contains("2026-08-23") && error.contains("HTTP 404"));
        assert_eq!(requested, [source_url(day(22)), source_url(day(23))]);
        assert_eq!(cache.len(), 1);
        assert!(cache.contains_key(&day(3).days_from_epoch()));
        assert_eq!(
            read_entry(&temp.0, day(22)).expect("retained evidence"),
            Some(bytes)
        );
        assert!(
            read_entry(&temp.0, day(23))
                .expect("no partial entry")
                .is_none()
        );
    }

    #[tokio::test]
    async fn unavailable_day_never_returns_a_partial_map() {
        let temp = Temp::new();
        let original = seed(&temp, 3);
        let error = prepare_with(&temp.0, window(3, 4), |_| async {
            Err("HTTP 404".to_owned())
        })
        .await
        .expect_err("missing day refuses");
        assert!(error.contains("2026-08-04") && error.contains("HTTP 404"));
        assert_eq!(
            read_entry(&temp.0, day(3)).expect("previous cache survives"),
            Some(original)
        );
        assert!(
            read_entry(&temp.0, day(4))
                .expect("no partial entry")
                .is_none()
        );
    }

    #[tokio::test]
    async fn bad_cached_metadata_refuses_before_any_fetch() {
        let temp = Temp::new();
        let bytes = seed(&temp, 4);
        let (_, metadata) = paths(&temp.0, day(4));
        for recorded in [
            receipt(day(3), &bytes),
            "unknown-version\n".to_owned(),
            receipt(day(4), b"wrong digest"),
        ] {
            fs::write(&metadata, recorded).expect("damage test receipt");
            let mut fetched = false;
            let result = prepare_with(&temp.0, window(3, 4), |_| {
                fetched = true;
                std::future::ready(Err("unexpected network".to_owned()))
            })
            .await;
            assert!(
                result
                    .expect_err("bad metadata refuses")
                    .contains("receipt")
            );
            assert!(!fetched);
        }
    }

    #[test]
    fn reinstall_is_idempotent_but_conflicts_are_immutable() {
        let temp = Temp::new();
        let bytes = seed(&temp, 3);
        install(&temp.0, day(3), &bytes).expect("same bytes are safe");
        let changed = gzip(CSV.replace("INE038A01020,1", "INE038A01020,0").as_bytes());
        assert!(
            install(&temp.0, day(3), &changed)
                .expect_err("different valid bytes conflict")
                .contains("conflicting")
        );
        assert_eq!(
            read_entry(&temp.0, day(3)).expect("cache unchanged"),
            Some(bytes)
        );
    }

    #[test]
    fn wrong_header_bad_utf8_and_trailing_gzip_are_rejected() {
        let temp = Temp::new();
        let invalid = gzip(
            CSV.replace("ElgbltyClsgAuctnSsn", "ElgbltyRETDBTMkt")
                .as_bytes(),
        );
        assert!(
            install(&temp.0, day(3), &invalid)
                .expect_err("old header refuses")
                .contains("CSV")
        );
        assert!(
            read_entry(&temp.0, day(3))
                .expect("nothing published")
                .is_none()
        );
        assert!(
            decode(&gzip(&[0xff]))
                .expect_err("invalid UTF-8")
                .contains("UTF-8")
        );
        let mut trailing = gzip(CSV.as_bytes());
        trailing.push(0);
        assert!(
            decode(&trailing)
                .expect_err("trailing bytes")
                .contains("trailing")
        );
        assert!(decode(b"not gzip").is_err());
    }

    #[test]
    fn correctly_receipted_bad_csv_still_refuses() {
        let temp = Temp::new();
        let bytes = gzip(b"SYMBOL,FINAL VOLUME\nHINDALCO,42\n");
        let (payload, metadata) = paths(&temp.0, day(3));
        fs::write(payload, &bytes).expect("write malformed fixture");
        fs::write(metadata, receipt(day(3), &bytes)).expect("matching receipt");
        let raw = read_entry(&temp.0, day(3))
            .expect("digest valid")
            .expect("entry present");
        assert!(decode(&raw).is_err());
        assert!(install(&temp.0, day(3), &gzip(CSV.as_bytes())).is_err());
    }

    #[test]
    fn incomplete_entries_and_unknown_dates_fail_closed() {
        let temp = Temp::new();
        let bytes = gzip(CSV.as_bytes());
        let (payload, metadata) = paths(&temp.0, day(3));
        fs::write(&payload, &bytes).expect("orphan gzip");
        assert!(
            install(&temp.0, day(3), &bytes)
                .expect_err("no implicit receipt upgrade")
                .contains("incomplete")
        );
        fs::remove_file(&payload).expect("remove owned test fixture");
        fs::write(metadata, receipt(day(3), &bytes)).expect("orphan receipt");
        assert!(
            read_entry(&temp.0, day(3))
                .expect_err("orphan receipt refuses")
                .contains("incomplete")
        );
        assert!(install(&temp.0, day(2), &bytes).is_err());
        let future = Day::from_days(u32::try_from(calendar::LAST_DAY + 1).expect("positive day"))
            .expect("future date");
        assert!(
            required_days(Window::new(future, future).expect("future window"))
                .expect_err("unmeasured date")
                .contains("UNVERIFIED")
        );
        assert!(is_required(day(3), DayKind::OpenLengthUnmeasured).is_err());
        assert!(
            is_required(
                day(3),
                DayKind::Open(Session {
                    windows: [
                        calendar::Window { from: 555, to: 599 },
                        calendar::Window { from: 0, to: 0 }
                    ],
                    count: 1
                })
            )
            .is_err()
        );
    }

    #[test]
    fn both_size_limits_and_corrupt_gzip_are_enforced() {
        assert!(
            decode(&vec![0; MAX_COMPRESSED + 1])
                .expect_err("compressed cap")
                .contains("compressed bytes")
        );
        let oversized = gzip(&vec![b'a'; MAX_EXPANDED + 1]);
        assert!(
            decode(&oversized)
                .expect_err("expanded cap")
                .contains("expanded bytes")
        );
        let mut corrupt = gzip(CSV.as_bytes());
        corrupt.pop();
        assert!(decode(&corrupt).is_err());
    }

    fn sample_day(name: &str) -> Result<Day, String> {
        let stamp = name
            .strip_prefix("NSE_CM_security_")
            .and_then(|name| name.strip_suffix(".csv.gz"))
            .filter(|stamp| stamp.len() == 8 && stamp.bytes().all(|byte| byte.is_ascii_digit()))
            .ok_or_else(|| format!("invalid dated NSE master filename: {name}"))?;
        let (day, remainder) = stamp.split_at_checked(2).ok_or("missing day")?;
        let (month, year) = remainder.split_at_checked(2).ok_or("missing month")?;
        Day::new(
            year.parse().map_err(|why| format!("invalid year: {why}"))?,
            month
                .parse()
                .map_err(|why| format!("invalid month: {why}"))?,
            day.parse().map_err(|why| format!("invalid day: {why}"))?,
        )
        .map_err(|why| why.to_string())
    }

    // Explicitly invoked only with the directory of public NSE downloads.
    // Absence of the environment variable is an error, never an empty pass.
    #[tokio::test]
    #[ignore = "requires BRUTEX_NSE_CASH_SAMPLE_DIR containing the 25 official dated gzip samples"]
    async fn official_25_dated_masters_install_and_validate_without_network() -> Result<(), String>
    {
        let source = std::env::var_os("BRUTEX_NSE_CASH_SAMPLE_DIR")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .ok_or("BRUTEX_NSE_CASH_SAMPLE_DIR must name the public NSE sample directory")?;
        let end = Day::new(2026, 9, 4).map_err(|why| why.to_string())?;
        let span = Window::new(day(3), end).map_err(|why| why.to_string())?;
        // These are the dated source samples actually acquired, not a second
        // production trading calendar or a weekday fallback for unknown days.
        let mut expected: Vec<_> = [
            3, 4, 5, 6, 7, 10, 11, 12, 13, 14, 17, 18, 19, 20, 21, 24, 25, 26, 27, 28, 31,
        ]
        .into_iter()
        .map(day)
        .collect();
        for date in 1..=4 {
            expected.push(Day::new(2026, 9, date).map_err(|why| why.to_string())?);
        }
        assert_eq!(expected.len(), 25);
        let mut missing: std::collections::BTreeSet<_> = expected.iter().copied().collect();
        let temp = Temp::new();
        let entries = fs::read_dir(&source)
            .map_err(|why| format!("cannot read sample directory {}: {why}", source.display()))?;
        for entry in entries {
            let entry = entry.map_err(|why| why.to_string())?;
            let name = entry.file_name();
            let Some(name) = name.to_str() else { continue };
            if !name.starts_with("NSE_CM_security_") || !name.ends_with(".csv.gz") {
                continue;
            }
            let date = sample_day(name)?;
            if !missing.remove(&date) {
                return Err(format!("unexpected or duplicate dated master: {name}"));
            }
            let bytes = read_limited(&entry.path(), MAX_COMPRESSED)?;
            install(&temp.0, date, &bytes).map_err(|why| format!("{name}: {why}"))?;
        }
        if !missing.is_empty() {
            return Err(format!(
                "missing official dated master samples: {missing:?}"
            ));
        }
        let mut observed = HashMap::new();
        prepare_observed_with(&temp.0, &expected, &mut observed, |url| async move {
            Err(format!("sample audit must not fetch: {url}"))
        })
        .await?;
        assert_eq!(observed.len(), 25);
        for date in expected {
            let bytes =
                read_entry(&temp.0, date)?.ok_or_else(|| format!("cache omitted {date}"))?;
            let eligibility = decode(&bytes)?;
            assert!(
                eligibility.eligibility("HINDALCO", "INE038A01020")?,
                "HINDALCO CAS flag on {date}"
            );
            assert!(
                observed
                    .get(&date.days_from_epoch())
                    .expect("observed master retained")
                    .eligibility("HINDALCO", "INE038A01020")?
            );
            assert!(
                !eligibility.eligibility("20MICRONS", "INE144J01027")?,
                "20MICRONS CAS flag on {date}"
            );
        }
        let result = prepare_with(&temp.0, span, |url| async move {
            Err(format!("sample audit must not fetch: {url}"))
        })
        .await;
        match required_days(span) {
            Ok(days) => assert_eq!(result?.len(), days.len()),
            Err(calendar_refusal) => {
                assert_eq!(
                    result.expect_err("unmeasured calendar must refuse"),
                    calendar_refusal
                );
                eprintln!(
                    "All 25 official masters validated and reloaded; full-window prepare refused: {calendar_refusal}"
                );
            }
        }
        Ok(())
    }
}

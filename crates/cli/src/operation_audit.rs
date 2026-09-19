//! UNVERIFIED performance: no named cost test or measured latency bound is established here.
//! Durable invocation history, separate from computation identity and admission.
//!
//! `audit/invocations-v1/index.bin` reserves monotonically increasing IDs under
//! an advisory file lock. Each ID has its own append-only `.bin` journal. A
//! fixed 256-byte CRC32C record is synced before acknowledgement. Exact lookup
//! reads the indexed start and the first/last invocation records, never history.
//! A start without a terminal is **unconfirmed**, not proof of a live worker.
//! No record contains command arguments, request queries, headers or bodies.
//!
//! Records: magic 0..8; ID 8..16; milliseconds 16..24; elapsed microseconds
//! 24..32; completed boundaries 32..40; phase/origin 40/41; response status
//! 42..44; label length 44..46; reserved zero 46..48; ASCII label 48..144;
//! reserved zero 144..252; CRC32C 252..256. Version one never changes in place.
//! Appends and exact reads have fixed work, but filesystem latency is not O(1).
//! Disk use grows with invocations and recorded structural boundaries.

use std::cell::RefCell;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read as _, Seek as _, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;

const BYTES: usize = 256;
const STRIDE: u64 = 256;
const MAGIC: &[u8; 8] = b"BXOPAU01";
const LABEL_START: usize = 48;
const LABEL_END: usize = 144;
const CRC: usize = 252;
/// Largest retained invocation page, independently of total history.
pub const MAX_PAGE: usize = 32;
/// Durable IDs have a separate namespace from legacy rotating telemetry IDs.
pub const ID_BASE: u64 = 1 << 63;

/// Who owns this invocation; a browser task is separate from its HTTP request.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Origin {
    /// A terminal command.
    Cli,
    /// A detached browser-started engine task.
    Browser,
    /// An HTTP handler, not the work it may have launched.
    Http,
}

impl Origin {
    /// Stable serialized presentation label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Cli => "cli",
            Self::Browser => "browser",
            Self::Http => "http",
        }
    }
    const fn code(self) -> u8 {
        match self {
            Self::Cli => 1,
            Self::Browser => 2,
            Self::Http => 3,
        }
    }
    fn decode(code: u8) -> Result<Self, String> {
        match code {
            1 => Ok(Self::Cli),
            2 => Ok(Self::Browser),
            3 => Ok(Self::Http),
            _ => Err("unknown invocation origin".to_owned()),
        }
    }
}

/// Explicit boundary state; absence of a terminal is never inferred success.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Phase {
    /// Durable admission intent, not proof of a currently running process.
    Started,
    /// A completed structural boundary, never a fabricated percentage.
    Progress,
    /// The caller finished its requested operation, not strategy admission.
    Completed,
    /// An explicit input or evidence refusal.
    Refused,
    /// A panic or execution/storage failure.
    Failed,
    /// The owning future/task was dropped before normal completion.
    Cancelled,
}

impl Phase {
    /// Stable state for UI and automated readers.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Started | Self::Progress => "unconfirmed",
            Self::Completed => "completed",
            Self::Refused => "refused",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
    /// Whether the caller explicitly ended this invocation.
    #[must_use]
    pub const fn terminal(self) -> bool {
        !matches!(self, Self::Started | Self::Progress)
    }
    const fn code(self) -> u8 {
        match self {
            Self::Started => 0,
            Self::Progress => 1,
            Self::Completed => 2,
            Self::Refused => 3,
            Self::Failed => 4,
            Self::Cancelled => 5,
        }
    }
    fn decode(code: u8) -> Result<Self, String> {
        match code {
            0 => Ok(Self::Started),
            1 => Ok(Self::Progress),
            2 => Ok(Self::Completed),
            3 => Ok(Self::Refused),
            4 => Ok(Self::Failed),
            5 => Ok(Self::Cancelled),
            _ => Err("unknown invocation phase".to_owned()),
        }
    }
}

/// Facts stored in one immutable fixed-stride boundary record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Record {
    /// Exact persistent invocation ID, not a strategy or data identity.
    pub id: u64,
    /// Wall clock, milliseconds since Unix epoch; zero means unstamped.
    pub at_millis: u64,
    /// Monotonic elapsed duration in this owning process.
    pub elapsed_micros: u64,
    /// Count of explicitly finished coarse work units; no total is invented.
    pub completed_boundaries: u64,
    /// Explicit lifecycle state.
    pub phase: Phase,
    /// Invocation owner.
    pub origin: Origin,
    /// HTTP status, or zero for an operation with no HTTP response.
    pub response_status: u16,
    /// A bounded public operation/route name, never arguments or a query.
    pub label: String,
}

fn put<const N: usize>(image: &mut [u8; BYTES], offset: usize, bytes: &[u8; N]) {
    if let Some(target) = image.get_mut(offset..offset + N) {
        target.copy_from_slice(bytes);
    }
}

fn word<const N: usize>(image: &[u8; BYTES], offset: usize) -> Result<[u8; N], String> {
    image
        .get(offset..offset + N)
        .and_then(|bytes| bytes.try_into().ok())
        .ok_or_else(|| "invocation record field is outside its fixed layout".to_owned())
}

impl Record {
    fn encode(&self) -> Result<[u8; BYTES], String> {
        if self.id <= ID_BASE
            || self.label.is_empty()
            || self.label.len() > LABEL_END - LABEL_START
            || !self
                .label
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b" /._-".contains(&b))
            || (self.response_status != 0 && !(100..=599).contains(&self.response_status))
        {
            return Err(
                "invalid invocation ID, public label or HTTP status; no values were truncated"
                    .to_owned(),
            );
        }
        let mut image = [0; BYTES];
        put(&mut image, 0, MAGIC);
        for (offset, value) in [
            (8, self.id),
            (16, self.at_millis),
            (24, self.elapsed_micros),
            (32, self.completed_boundaries),
        ] {
            put(&mut image, offset, &value.to_le_bytes());
        }
        put(&mut image, 40, &[self.phase.code(), self.origin.code()]);
        put(&mut image, 42, &self.response_status.to_le_bytes());
        let length = u16::try_from(self.label.len()).map_err(|why| why.to_string())?;
        put(&mut image, 44, &length.to_le_bytes());
        image
            .get_mut(LABEL_START..LABEL_START + self.label.len())
            .ok_or("label layout overflow")?
            .copy_from_slice(self.label.as_bytes());
        let crc = store::crc::crc32c(image.get(..CRC).ok_or("CRC layout overflow")?);
        put(&mut image, CRC, &crc.to_le_bytes());
        Ok(image)
    }

    fn decode(image: &[u8; BYTES]) -> Result<Self, String> {
        if word::<8>(image, 0)? != *MAGIC
            || store::crc::crc32c(image.get(..CRC).ok_or("CRC layout overflow")?)
                != u32::from_le_bytes(word(image, CRC)?)
        {
            return Err("invocation record magic/version or CRC is invalid".to_owned());
        }
        let length = usize::from(u16::from_le_bytes(word(image, 44)?));
        let label = image
            .get(LABEL_START..LABEL_START.saturating_add(length))
            .filter(|_| length <= LABEL_END - LABEL_START)
            .ok_or("invalid invocation label length")?;
        let record = Self {
            id: u64::from_le_bytes(word(image, 8)?),
            at_millis: u64::from_le_bytes(word(image, 16)?),
            elapsed_micros: u64::from_le_bytes(word(image, 24)?),
            completed_boundaries: u64::from_le_bytes(word(image, 32)?),
            phase: Phase::decode(
                word::<1>(image, 40)?
                    .into_iter()
                    .next()
                    .ok_or("missing phase")?,
            )?,
            origin: Origin::decode(
                word::<1>(image, 41)?
                    .into_iter()
                    .next()
                    .ok_or("missing origin")?,
            )?,
            response_status: u16::from_le_bytes(word(image, 42)?),
            label: std::str::from_utf8(label)
                .map_err(|why| why.to_string())?
                .to_owned(),
        };
        if record.encode()? != *image {
            return Err("invocation record contains noncanonical padding or fields".to_owned());
        }
        Ok(record)
    }
}

fn base(root: &Path) -> PathBuf {
    root.join("audit/invocations-v1")
}
fn own(base: &Path, id: u64) -> PathBuf {
    base.join(format!("{id:020}.bin"))
}
fn error(why: impl std::fmt::Display) -> String {
    format!("invocation audit: {why}")
}

const BUSY: &str = "invocation audit busy: ";

/// Whether a refusal denotes bounded lock/snapshot contention rather than
/// corrupt data or a storage failure. HTTP may retry only before dispatch.
#[must_use]
pub fn is_busy(why: &str) -> bool {
    why.starts_with(BUSY)
}

fn lock_error(why: std::fs::TryLockError) -> String {
    match why {
        std::fs::TryLockError::WouldBlock => {
            format!("{BUSY}another operation holds the journal lock; no work was queued")
        }
        std::fs::TryLockError::Error(why) => error(why),
    }
}

fn directory(path: &Path) -> Result<(), String> {
    fs::create_dir_all(path).map_err(error)?;
    if fs::symlink_metadata(path)
        .map_err(error)?
        .file_type()
        .is_symlink()
    {
        return Err(error("directory symlinks are refused"));
    }
    File::open(path)
        .and_then(|file| file.sync_all())
        .map_err(error)?;
    if let Some(parent) = path.parent() {
        File::open(parent)
            .and_then(|file| file.sync_all())
            .map_err(error)?;
    }
    Ok(())
}

fn options() -> OpenOptions {
    use std::os::unix::fs::OpenOptionsExt as _;
    let mut options = OpenOptions::new();
    // Same supported native platforms as the immutable store readers. Refuse
    // final-component symlinks and avoid blocking on an unexpected FIFO.
    #[cfg(target_os = "macos")]
    options.custom_flags(0x100 | 0x4);
    #[cfg(target_os = "linux")]
    options.custom_flags(0x20000 | 0x800);
    options
}

fn length(file: &File) -> Result<u64, String> {
    let metadata = file.metadata().map_err(error)?;
    if !metadata.is_file() || metadata.len() % STRIDE != 0 {
        return Err(error(
            "nonregular file or torn fixed-stride record; nothing was repaired",
        ));
    }
    Ok(metadata.len())
}

fn at(file: &mut File, ordinal: u64) -> Result<Record, String> {
    let offset = ordinal
        .checked_mul(STRIDE)
        .ok_or("invocation offset overflow")?;
    file.seek(SeekFrom::Start(offset)).map_err(error)?;
    let mut image = [0; BYTES];
    file.read_exact(&mut image).map_err(error)?;
    Record::decode(&image)
}

fn append(file: &mut File, expected: u64, record: &Record) -> Result<(), String> {
    let image = record.encode()?;
    file.try_lock().map_err(lock_error)?;
    let result = (|| {
        if length(file)? != expected {
            return Err(error("journal changed outside its owning invocation"));
        }
        write_synced(file, &image)
    })();
    let unlocked = file.unlock().map_err(error);
    result.and(unlocked)
}

trait DurableWrite: io::Write {
    fn sync(&mut self) -> io::Result<()>;
}
impl DurableWrite for File {
    fn sync(&mut self) -> io::Result<()> {
        self.sync_all()
    }
}
fn write_synced(file: &mut impl DurableWrite, image: &[u8; BYTES]) -> Result<(), String> {
    file.write_all(image)
        .and_then(|()| file.sync())
        .map_err(error)
}

struct State {
    file: File,
    record: Record,
    bytes: u64,
    started: Instant,
    failure: Option<String>,
}

impl State {
    fn update(&mut self, phase: Phase, status: u16, advance: bool) -> Result<(), String> {
        if let Some(why) = &self.failure {
            return Err(why.clone());
        }
        if self.record.phase.terminal() {
            return Err(error(
                "duplicate terminal or progress after completion refused",
            ));
        }
        let mut next = self.record.clone();
        next.phase = phase;
        next.response_status = status;
        next.at_millis = u64::try_from(telemetry::now_millis()).unwrap_or(0);
        next.elapsed_micros = u64::try_from(self.started.elapsed().as_micros()).unwrap_or(u64::MAX);
        if advance {
            next.completed_boundaries = next
                .completed_boundaries
                .checked_add(1)
                .ok_or("boundary counter exhausted")?;
        }
        if let Err(why) = append(&mut self.file, self.bytes, &next) {
            self.failure = Some(why.clone());
            return Err(why);
        }
        self.bytes = self
            .bytes
            .checked_add(STRIDE)
            .ok_or("invocation byte count exhausted")?;
        self.record = next;
        Ok(())
    }
}

/// One admitted operation; ownership covers a queued closure as well as execution.
pub struct Attempt {
    state: Arc<Mutex<State>>,
    armed: bool,
}

impl Attempt {
    /// Exact persistent invocation token.
    #[must_use]
    pub fn id(&self) -> u64 {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .record
            .id
    }

    /// Record a completed coarse unit, never called inside candle/candidate loops.
    /// # Errors
    /// Refuses corruption, busy writers, I/O failure and a previous terminal.
    pub fn progress(&self) -> Result<(), String> {
        self.state
            .lock()
            .map_err(error)?
            .update(Phase::Progress, 0, true)
    }

    /// Explicitly end the operation after its result publication has settled.
    /// # Errors
    /// A missing durability barrier, prior audit failure or duplicate terminal.
    pub fn finish(&mut self, phase: Phase, status: u16) -> Result<(), String> {
        if !phase.terminal() {
            return Err(error("finish requires an explicit terminal phase"));
        }
        let result = self
            .state
            .lock()
            .map_err(error)?
            .update(phase, status, false);
        self.armed = false;
        result
    }

    /// Attach this invocation only to synchronous structural work on this thread.
    /// The prior owner is restored even if the closure panics.
    pub fn enter<T>(&self, work: impl FnOnce() -> T) -> T {
        let prior = CURRENT.with(|slot| slot.replace(Some(Arc::clone(&self.state))));
        let _restore = Restore(prior);
        work()
    }
}

impl Drop for Attempt {
    fn drop(&mut self) {
        if self.armed {
            let phase = if std::thread::panicking() {
                Phase::Failed
            } else {
                Phase::Cancelled
            };
            if let Err(why) = self.finish(phase, 0) {
                let _noted = telemetry::emit(
                    &telemetry::Event::error("cli.audit", "terminal audit unconfirmed")
                        .with("why", telemetry::Value::Str(&why)),
                );
                eprintln!("{why}; terminal audit is unconfirmed");
            }
        }
    }
}

thread_local! { static CURRENT: RefCell<Option<Arc<Mutex<State>>>> = const { RefCell::new(None) }; }
struct Restore(Option<Arc<Mutex<State>>>);
impl Drop for Restore {
    fn drop(&mut self) {
        CURRENT.with(|slot| {
            slot.replace(self.0.take());
        });
    }
}

/// Current thread's invocation, never an unrelated ambient telemetry run.
#[must_use]
pub fn current_id() -> Option<u64> {
    CURRENT.with(|slot| {
        slot.borrow().as_ref().map(|state| {
            state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .record
                .id
        })
    })
}

/// Persist one finished rung/command boundary. A failure poisons final admission.
pub fn completed_boundary() {
    CURRENT.with(|slot| {
        if let Some(state) = slot.borrow().as_ref() {
            let result = state
                .lock()
                .map_err(error)
                .and_then(|mut state| state.update(Phase::Progress, 0, true));
            if let Err(why) = result {
                let _noted = telemetry::emit(
                    &telemetry::Event::error("cli.audit", "boundary audit unconfirmed")
                        .with("why", telemetry::Value::Str(&why)),
                );
                eprintln!("{why}; this invocation cannot acknowledge a successful terminal audit");
            }
        }
    });
}

/// Reserve and sync an exact ID and start before dispatching any work.
/// # Errors
/// Refuses busy writers, missing roots, torn/corrupt files, symlinks and I/O faults.
pub fn begin(root: &Path, origin: Origin, label: &str) -> Result<Attempt, String> {
    if !root.is_dir() {
        return Err(error("the configured store root is absent"));
    }
    let base = base(root);
    directory(&root.join("audit"))?;
    directory(&base)?;
    let mut index = options()
        .read(true)
        .append(true)
        .create(true)
        .open(base.join("index.bin"))
        .map_err(error)?;
    index.try_lock().map_err(lock_error)?;
    let bytes = length(&index)?;
    let ordinal = (bytes / STRIDE)
        .checked_add(1)
        .ok_or("invocation IDs exhausted")?;
    let id = ID_BASE
        .checked_add(ordinal)
        .ok_or("invocation IDs exhausted")?;
    if ordinal > 1 {
        let previous = at(&mut index, ordinal - 2)?;
        if previous.id != id - 1 || previous.phase != Phase::Started {
            return Err(error("invocation index tail does not match its address"));
        }
    }
    let record = Record {
        id,
        at_millis: u64::try_from(telemetry::now_millis()).unwrap_or(0),
        elapsed_micros: 0,
        completed_boundaries: 0,
        phase: Phase::Started,
        origin,
        response_status: 0,
        label: label.to_owned(),
    };
    let image = record.encode()?;
    write_synced(&mut index, &image)?;
    index.unlock().map_err(error)?;
    let mut file = options()
        .read(true)
        .append(true)
        .create_new(true)
        .open(own(&base, id))
        .map_err(error)?;
    write_synced(&mut file, &image)?;
    File::open(&base)
        .and_then(|file| file.sync_all())
        .map_err(error)?;
    Ok(Attempt {
        state: Arc::new(Mutex::new(State {
            file,
            record,
            bytes: STRIDE,
            started: Instant::now(),
            failure: None,
        })),
        armed: true,
    })
}

/// Look up exactly one invocation across process restarts, with bounded reads.
/// Missing per-invocation creation returns its unconfirmed indexed start.
/// # Errors
/// Refuses busy writers, torn/corrupt records, wrong ancestry and invalid IDs.
pub fn read(root: &Path, id: u64) -> Result<Option<Record>, String> {
    if !root.is_dir() {
        return Err(error("the configured store root is absent"));
    }
    let ordinal = id
        .checked_sub(ID_BASE)
        .filter(|value| *value > 0)
        .ok_or_else(|| error("ID is outside the durable invocation namespace"))?;
    let base = base(root);
    let mut index = match options().read(true).open(base.join("index.bin")) {
        Ok(file) => file,
        Err(why) if why.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(why) => return Err(error(why)),
    };
    index.try_lock_shared().map_err(lock_error)?;
    if ordinal > length(&index)? / STRIDE {
        return Ok(None);
    }
    let started = at(&mut index, ordinal - 1)?;
    if started.id != id || started.phase != Phase::Started {
        return Err(error("invocation start does not match its exact address"));
    }
    index.sync_all().map_err(error)?;
    index.unlock().map_err(error)?;
    let mut file = match options().read(true).open(own(&base, id)) {
        Ok(file) => file,
        Err(why) if why.kind() == io::ErrorKind::NotFound => return Ok(Some(started)),
        Err(why) => return Err(error(why)),
    };
    // A writer only appends. Taking its lock to poll progress could fail the
    // computation. Verify this bounded snapshot again after the sync instead.
    let bytes = length(&file)?;
    if bytes == 0 || at(&mut file, 0)? != started {
        return Err(error("invocation journal lost its indexed start"));
    }
    let last = at(&mut file, bytes / STRIDE - 1)?;
    let expected_boundaries =
        (bytes / STRIDE).saturating_sub(if last.phase.terminal() { 2 } else { 1 });
    if last.id != id
        || last.origin != started.origin
        || last.label != started.label
        || (bytes > STRIDE && last.phase == Phase::Started)
        || last.completed_boundaries != expected_boundaries
    {
        return Err(error("invocation terminal belongs to a different start"));
    }
    file.sync_all().map_err(error)?;
    if length(&file)? != bytes {
        return Err(format!(
            "{BUSY}invocation changed during the bounded snapshot; retry the read"
        ));
    }
    Ok(Some(last))
}

/// Read a newest-first page of invocation IDs without scanning directories.
/// `before` is exclusive; callers continue with the last returned ID.
/// # Errors
/// Refuses noncanonical bounds, unavailable/corrupt records and busy writers.
pub fn page(root: &Path, before: Option<u64>, limit: usize) -> Result<Vec<Record>, String> {
    if !root.is_dir() {
        return Err(error("the configured store root is absent"));
    }
    if limit == 0 || limit > MAX_PAGE || before.is_some_and(|before| before <= ID_BASE) {
        return Err(error("invalid bounded invocation page"));
    }
    let index = match options().read(true).open(base(root).join("index.bin")) {
        Ok(file) => file,
        Err(why) if why.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(why) => return Err(error(why)),
    };
    let total = length(&index)? / STRIDE;
    let last = before.map_or(total, |before| total.min(before - ID_BASE - 1));
    let count = last.min(u64::try_from(limit).map_err(error)?);
    (0..count)
        .map(|offset| {
            read(root, ID_BASE + last - offset)?
                .ok_or_else(|| error("indexed invocation disappeared"))
        })
        .collect()
}

#[cfg(test)]
#[path = "operation_audit_tests.rs"]
mod tests;

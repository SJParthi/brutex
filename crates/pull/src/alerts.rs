//! The alert queue: `<store>/audit/alerts`, format v1 (D-4883, D-4893, D-4895).
//!
//! An audited event reaches someone only if they read the log. A condition the
//! operator must act on is also put, once, into this one durable queue, which
//! every page and `cli alerts` read. The format is `docs/02-store-format.md`
//! section 38: a 64-byte header and 128-byte records, little-endian, each with
//! its own CRC-32C, appended and never edited.
//!
//! # Cost
//!
//! * [`put`] and [`ack`] inside the window: one positional read of the newest
//!   [`WINDOW`] records (at most 128 KiB), one 128-byte write, one `sync_data`,
//!   whatever the file's length. Bench C-AL-01.
//! * [`window`] and [`page`]: one read of at most [`WINDOW`] records under the
//!   lock held shared. Bench C-AL-02.
//! * [`older`], [`page_settled`], [`of_file`] and an [`ack`] with
//!   [`Scope::Whole`] of an alert older than the window read the records after
//!   it: O(records), on the operator's command only, never on a route
//!   (`docs/06-limits.md` L-IA-34). The two walks read through one open file,
//!   the lock held shared for one read of at most [`WINDOW`] records at a
//!   time: one view of one queue, and a put never waits for a walk.
//!
//! # What it never does
//!
//! It never writes over a record whose CRC passes, never truncates, never
//! repairs: a queue whose tail holds two bad records is refused by name and
//! replaced by the operator ([`replace_damaged`]), the damaged file kept. The
//! replacement is all or nothing: at every moment, and after a kill at any
//! step, `audit/alerts` is the damaged queue or the new one already holding
//! its `queue_replaced` record.

use std::borrow::Cow;
use std::collections::BTreeSet;
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write as _};
use std::os::unix::fs::FileExt as _;
use std::os::unix::fs::MetadataExt as _;
use std::path::{Path, PathBuf};

use store::crc::crc32c;
use store::flock::Flock;

/// The eight bytes every queue starts with.
pub const MAGIC: [u8; 8] = *b"BRUTEXAL";
/// The only format version this build writes or reads.
pub const VERSION: u16 = 1;
/// The header's length in bytes.
pub const HEADER_LEN: u64 = 64;
/// One record's length in bytes.
pub const RECORD_LEN: u64 = 128;
/// How many of the newest records one put, one acknowledgement and one reader
/// take: the de-duplication window and the listing window are the same one.
pub const WINDOW: u64 = 1_024;
/// The longest text a record holds, in bytes.
pub const TEXT_MAX: usize = 75;
/// How many records' worth of space the reserve file holds for a full disk.
pub const RESERVE_RECORDS: u64 = 512;
/// The target of every event this module emits. Its last segment is
/// `lifecycle`, so each of them at `info` or above is audited (D-4893).
pub const TARGET: &str = "pull.alerts.lifecycle";

/// Record type 1: an alert.
pub const TYPE_ALERT: u8 = 1;
/// Record type 2: an acknowledgement of an earlier alert.
pub const TYPE_ACK: u8 = 2;
/// Flag bit 0: the text was longer than [`TEXT_MAX`] and was cut.
pub const FLAG_TEXT_CUT: u8 = 1;

/// The kind table of format v1: append-only, never renamed or reused.
///
/// 1 to 12 are the design's; 13 to 20 are the kinds the rules audit of
/// 11 Oct 2026 asked for before the format was pinned (D-4883).
pub const KINDS: [(u8, &str); 20] = [
    (1, "interrupted_twice"),
    (2, "store_lost"),
    (3, "build_skew"),
    (4, "calendar_horizon"),
    (5, "disk_full"),
    (6, "hole"),
    (7, "store_damaged"),
    (8, "runs_ledger_damaged"),
    (9, "mismatch"),
    (10, "spool_damaged"),
    (11, "verdict_refused"),
    (12, "queue_replaced"),
    (13, "run_ended_early"),
    (14, "interrupted"),
    (15, "stalled"),
    (16, "autopilot_stalled"),
    (17, "disk_low"),
    (18, "file_shrank"),
    (19, "month_damaged"),
    (20, "index_refused"),
];

/// Kind 12, the first record of a queue that replaced a damaged one.
pub const KIND_QUEUE_REPLACED: u8 = 12;

/// The word of a kind this build can name.
#[must_use]
pub fn kind_word(kind: u8) -> Option<&'static str> {
    let at = usize::from(kind).checked_sub(1)?;
    KINDS.get(at).filter(|row| row.0 == kind).map(|row| row.1)
}

/// The kind number of a word, for a caller that holds the word.
#[must_use]
pub fn kind_of(word: &str) -> Option<u8> {
    for (kind, name) in KINDS {
        if name == word {
            return Some(kind);
        }
    }
    None
}

/// The word a reader shows: the table's, or `kind_<n>` for a number a later
/// build wrote. A record is never dropped because this build cannot name it.
#[must_use]
pub fn kind_label(kind: u8) -> Cow<'static, str> {
    kind_word(kind).map_or_else(|| Cow::Owned(format!("kind_{kind}")), Cow::Borrowed)
}

/// Why the queue refused, by name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AlertError {
    /// `alerts_unavailable`: the store root is not a directory. Never created.
    Unavailable {
        /// The root that was asked for.
        root: PathBuf,
    },
    /// `alert_kind_unknown`: the writer names what it writes.
    KindUnknown {
        /// The kind that is not in [`KINDS`].
        kind: u8,
    },
    /// `alert_key_empty`: an alert with no de-duplication key.
    KeyEmpty,
    /// `alerts_damaged`: the header, or two records at the tail, fail. Every
    /// put refuses such a queue; the operator replaces it.
    Damaged {
        /// The byte offset of the first thing that failed.
        offset: u64,
    },
    /// `alert_record_damaged`: the record an acknowledgement names fails its
    /// CRC. The queue itself still takes puts, so there is nothing to
    /// replace; the record stays listed as damaged.
    RecordDamaged {
        /// The ordinal asked for.
        ordinal: u32,
        /// Its byte offset.
        offset: u64,
    },
    /// `alerts_version_unknown`: a later build's queue.
    VersionUnknown {
        /// The version the header carries.
        found: u16,
    },
    /// `alerts_not_a_file`: a link, a pipe or a directory sits at the name.
    NotAFile {
        /// The name that is not a regular file.
        path: PathBuf,
    },
    /// `no_such_alert`: ordinal 0, one past the end, or not an alert.
    NoSuchAlert {
        /// The ordinal asked for.
        ordinal: u32,
    },
    /// `already_acknowledged`.
    AlreadyAcknowledged {
        /// The ordinal asked for.
        ordinal: u32,
    },
    /// `ack_outside_window`: older than the newest [`WINDOW`] records; the
    /// cli's `--ack` does it.
    AckOutsideWindow {
        /// The ordinal asked for.
        ordinal: u32,
    },
    /// `alerts_not_damaged`: a replacement asked of a queue the put accepts.
    NotDamaged,
    /// `alerts_full`: the file holds as many records as an ordinal can name.
    Full,
    /// `alerts_io`: the host refused.
    Io {
        /// What was being done.
        op: &'static str,
        /// The host's error kind.
        kind: io::ErrorKind,
        /// The host's own words.
        text: String,
    },
}

impl AlertError {
    /// The stable code of this refusal.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Unavailable { .. } => "alerts_unavailable",
            Self::KindUnknown { .. } => "alert_kind_unknown",
            Self::KeyEmpty => "alert_key_empty",
            Self::Damaged { .. } => "alerts_damaged",
            Self::RecordDamaged { .. } => "alert_record_damaged",
            Self::VersionUnknown { .. } => "alerts_version_unknown",
            Self::NotAFile { .. } => "alerts_not_a_file",
            Self::NoSuchAlert { .. } => "no_such_alert",
            Self::AlreadyAcknowledged { .. } => "already_acknowledged",
            Self::AckOutsideWindow { .. } => "ack_outside_window",
            Self::NotDamaged => "alerts_not_damaged",
            Self::Full => "alerts_full",
            Self::Io { .. } => "alerts_io",
        }
    }

    fn io(op: &'static str, why: &io::Error) -> Self {
        Self::Io {
            op,
            kind: why.kind(),
            text: why.to_string(),
        }
    }
}

impl fmt::Display for AlertError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let code = self.code();
        match self {
            Self::Unavailable { root } => {
                write!(f, "{code}: {} is not a directory", root.display())
            }
            Self::KindUnknown { kind } => write!(f, "{code}: kind {kind} is not in the table"),
            Self::KeyEmpty => write!(f, "{code}: an alert needs a key"),
            Self::Damaged { offset } => write!(
                f,
                "{code}{{offset: {offset}}}: run `cli alerts --replace-damaged`"
            ),
            Self::RecordDamaged { ordinal, offset } => write!(
                f,
                "{code}{{ordinal: {ordinal}, offset: {offset}}}: the record fails its CRC and \
                 cannot be acknowledged; it stays listed as damaged"
            ),
            Self::VersionUnknown { found } => {
                write!(f, "{code}: version {found}, this build reads {VERSION}")
            }
            Self::NotAFile { path } => {
                write!(f, "{code}: {} is not a regular file", path.display())
            }
            Self::NoSuchAlert { ordinal } | Self::AlreadyAcknowledged { ordinal } => {
                write!(f, "{code}: {ordinal}")
            }
            Self::AckOutsideWindow { ordinal } => {
                write!(f, "{code}: run `cli alerts --ack {ordinal}`")
            }
            Self::NotDamaged | Self::Full => f.write_str(code),
            Self::Io { op, kind, text } => write!(f, "{code}: {op}: {kind:?}: {text}"),
        }
    }
}

impl std::error::Error for AlertError {}

/// The queue's four names under one store root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Names {
    /// The store root.
    pub root: PathBuf,
    /// `<root>/audit`.
    pub dir: PathBuf,
    /// `<root>/audit/alerts`.
    pub queue: PathBuf,
    /// `<root>/audit/alerts.lock`.
    pub lock: PathBuf,
    /// `<root>/audit/alerts.reserve`.
    pub reserve: PathBuf,
}

impl Names {
    /// The names under `root`. Touches nothing.
    #[must_use]
    pub fn under(root: &Path) -> Self {
        let dir = root.join("audit");
        Self {
            root: root.to_path_buf(),
            queue: dir.join("alerts"),
            lock: dir.join("alerts.lock"),
            reserve: dir.join("alerts.reserve"),
            dir,
        }
    }
}

/// One alert to put.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NewAlert<'a> {
    /// A kind of [`KINDS`].
    pub kind: u8,
    /// The de-duplication key, per run or per store-wide fact.
    pub key: &'a str,
    /// Days since 1970-01-01; 0 for none.
    pub day: u32,
    /// The run id; 0 for none.
    pub run: u64,
    /// What happened, in at most [`TEXT_MAX`] bytes; a longer text is cut.
    pub text: &'a str,
}

/// One record, decoded. Every field of the 128 bytes but the CRC.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    /// 1 an alert, 2 an acknowledgement; any other number is a later build's.
    pub kind_of_record: u8,
    /// The alert's kind; 0 for an acknowledgement.
    pub kind: u8,
    /// The flag byte.
    pub flags: u8,
    /// Days since 1970-01-01; 0 for none.
    pub day: u32,
    /// The run id; 0 for none.
    pub run: u64,
    /// When it was put, unix seconds.
    pub put_at: u64,
    /// The first 16 bytes of the BLAKE3 of the key; zero for an acknowledgement.
    pub key: [u8; 16],
    /// The ordinal an acknowledgement names; 0 for an alert.
    pub acked: u32,
    /// The count of unacknowledged alerts once this record is counted.
    pub open_after: u32,
    /// The text, whole characters only.
    pub text: String,
}

impl Record {
    /// Whether the text was cut to fit.
    #[must_use]
    pub const fn text_cut(&self) -> bool {
        self.flags & FLAG_TEXT_CUT != 0
    }

    /// The 128 bytes of this record, CRC included.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(128);
        out.extend_from_slice(&[self.kind_of_record, self.kind, self.flags, 0]);
        out.extend_from_slice(&self.day.to_le_bytes());
        out.extend_from_slice(&self.run.to_le_bytes());
        out.extend_from_slice(&self.put_at.to_le_bytes());
        out.extend_from_slice(&self.key);
        out.extend_from_slice(&self.acked.to_le_bytes());
        out.extend_from_slice(&self.open_after.to_le_bytes());
        let text = self.text.as_bytes();
        let text = text.get(..TEXT_MAX).unwrap_or(text);
        out.push(u8::try_from(text.len()).unwrap_or(0));
        out.extend_from_slice(text);
        out.resize(124, 0);
        let crc = crc32c(&out);
        out.extend_from_slice(&crc.to_le_bytes());
        out
    }

    /// The record in 128 bytes, or `None` when they fail their CRC.
    #[must_use]
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        let body = bytes.get(..124)?;
        if bytes.len() != 128 || crc32c(body) != le_u32(bytes, 124) {
            return None;
        }
        let held = usize::from(byte(bytes, 48)).min(TEXT_MAX);
        let text = bytes.get(49..49 + held).unwrap_or(&[]);
        Some(Self {
            kind_of_record: byte(bytes, 0),
            kind: byte(bytes, 1),
            flags: byte(bytes, 2),
            day: le_u32(bytes, 4),
            run: le_u64(bytes, 8),
            put_at: le_u64(bytes, 16),
            key: bytes
                .get(24..40)
                .and_then(|s| <[u8; 16]>::try_from(s).ok())
                .unwrap_or([0; 16]),
            acked: le_u32(bytes, 40),
            open_after: le_u32(bytes, 44),
            text: String::from_utf8_lossy(text).into_owned(),
        })
    }
}

fn byte(bytes: &[u8], at: usize) -> u8 {
    bytes.get(at).copied().unwrap_or(0)
}

fn le_u16(bytes: &[u8], at: usize) -> u16 {
    bytes
        .get(at..at + 2)
        .and_then(|s| <[u8; 2]>::try_from(s).ok())
        .map_or(0, u16::from_le_bytes)
}

fn le_u32(bytes: &[u8], at: usize) -> u32 {
    bytes
        .get(at..at + 4)
        .and_then(|s| <[u8; 4]>::try_from(s).ok())
        .map_or(0, u32::from_le_bytes)
}

fn le_u64(bytes: &[u8], at: usize) -> u64 {
    bytes
        .get(at..at + 8)
        .and_then(|s| <[u8; 8]>::try_from(s).ok())
        .map_or(0, u64::from_le_bytes)
}

/// The 64 header bytes of a queue created at `created` (unix seconds).
#[must_use]
pub fn header(created: u64) -> Vec<u8> {
    let mut out = Vec::with_capacity(64);
    out.extend_from_slice(&MAGIC);
    out.extend_from_slice(&VERSION.to_le_bytes());
    out.extend_from_slice(&128_u16.to_le_bytes());
    out.extend_from_slice(&64_u16.to_le_bytes());
    out.extend_from_slice(&0_u16.to_le_bytes());
    out.extend_from_slice(&created.to_le_bytes());
    out.resize(60, 0);
    let crc = crc32c(&out);
    out.extend_from_slice(&crc.to_le_bytes());
    out
}

/// The first 16 bytes of the BLAKE3 of a de-duplication key.
#[must_use]
pub fn key_hash(key: &str) -> [u8; 16] {
    let full = brutex_core::blake3::hash(key.as_bytes());
    full.get(..16)
        .and_then(|s| <[u8; 16]>::try_from(s).ok())
        .unwrap_or([0; 16])
}

/// Whether a character would break or reorder the one line a page gives an
/// alert: a control character, the Unicode line and paragraph separators, the
/// bidirectional marks, embeddings, overrides and isolates, and the
/// zero-width and other invisible format characters.
fn unprintable(c: char) -> bool {
    c.is_control()
        || matches!(
            c,
            '\u{00AD}'
                | '\u{061C}'
                | '\u{180E}'
                | '\u{200B}'..='\u{200F}'
                | '\u{2028}'..='\u{202E}'
                | '\u{2060}'..='\u{206F}'
                | '\u{FEFF}'
                | '\u{FFF9}'..='\u{FFFB}'
        )
}

/// A text as a record holds it: every [`unprintable`] character becomes a
/// space, then it is cut at the largest character boundary at or below
/// [`TEXT_MAX`] bytes. Returns the text and whether it was cut.
#[must_use]
pub fn fit_text(text: &str) -> (String, bool) {
    let clean: String = text
        .chars()
        .map(|c| if unprintable(c) { ' ' } else { c })
        .collect();
    if clean.len() <= TEXT_MAX {
        return (clean, false);
    }
    let mut end = TEXT_MAX;
    while end > 0 && !clean.is_char_boundary(end) {
        end -= 1;
    }
    (clean.get(..end).unwrap_or("").to_owned(), true)
}

/// The byte offset of ordinal `k` (1-based).
#[must_use]
pub const fn offset_of(ordinal: u64) -> u64 {
    HEADER_LEN + RECORD_LEN * ordinal.saturating_sub(1)
}

/// Unix seconds now; 0 when the clock reads before 1970.
#[must_use]
pub fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

// ── Reading the file ───────────────────────────────────────────────────────

/// One slot of a span: a record that passed its CRC, or one that did not.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Slot {
    Good(Record),
    Bad,
}

/// A run of whole records read in one positional read.
#[derive(Debug)]
struct Span {
    /// The ordinal of `slots[0]`; 1 when the span is empty.
    first: u64,
    /// Oldest first.
    slots: Vec<Slot>,
}

impl Span {
    fn ordinal(&self, at: usize) -> u64 {
        self.first + u64::try_from(at).unwrap_or(0)
    }

    /// The slot index of an ordinal inside this span.
    fn index(&self, ordinal: u64) -> Option<usize> {
        let at = usize::try_from(ordinal.checked_sub(self.first)?).ok()?;
        (at < self.slots.len()).then_some(at)
    }

    /// The newest good record among the first `live` slots.
    fn newest_good(&self, live: usize) -> Option<&Record> {
        for slot in self.slots.iter().take(live).rev() {
            if let Slot::Good(record) = slot {
                return Some(record);
            }
        }
        None
    }
}

/// What the file's length says: whole records and the bytes past them.
#[derive(Debug, Clone, Copy)]
struct Shape {
    records: u64,
    torn: u64,
}

fn regular(path: &Path) -> Result<bool, AlertError> {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_file() => Ok(true),
        Ok(_) => Err(AlertError::NotAFile {
            path: path.to_path_buf(),
        }),
        Err(why) if why.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(why) => Err(AlertError::io("stat", &why)),
    }
}

/// Checks the header and returns the file's shape.
fn shape(file: &File) -> Result<Shape, AlertError> {
    let len = file
        .metadata()
        .map_err(|why| AlertError::io("stat the queue", &why))?
        .len();
    if len < HEADER_LEN {
        return Err(AlertError::Damaged { offset: 0 });
    }
    let mut head = [0_u8; 64];
    file.read_exact_at(&mut head, 0)
        .map_err(|why| AlertError::io("read the header", &why))?;
    let crc_ok = head
        .get(..60)
        .is_some_and(|b| crc32c(b) == le_u32(&head, 60));
    if !crc_ok || head.get(..8) != Some(MAGIC.as_slice()) {
        return Err(AlertError::Damaged { offset: 0 });
    }
    let version = le_u16(&head, 8);
    if version != VERSION {
        return Err(AlertError::VersionUnknown { found: version });
    }
    if le_u16(&head, 10) != 128 || le_u16(&head, 12) != 64 {
        return Err(AlertError::Damaged { offset: 0 });
    }
    let body = len - HEADER_LEN;
    Ok(Shape {
        records: body / RECORD_LEN,
        torn: body % RECORD_LEN,
    })
}

/// One positional read of at most [`WINDOW`] whole records ending at `end`.
fn span(file: &File, end: u64) -> Result<Span, AlertError> {
    let count = end.min(WINDOW);
    span_from(file, end - count + 1, count)
}

/// One positional read of `count` whole records from ordinal `first`.
fn span_from(file: &File, first: u64, count: u64) -> Result<Span, AlertError> {
    let bytes = usize::try_from(count * RECORD_LEN).unwrap_or(0);
    let mut buffer = vec![0_u8; bytes];
    file.read_exact_at(&mut buffer, offset_of(first))
        .map_err(|why| AlertError::io("read the window", &why))?;
    let slots = buffer
        .chunks_exact(128)
        .map(|chunk| Record::decode(chunk).map_or(Slot::Bad, Slot::Good))
        .collect();
    Ok(Span { first, slots })
}

/// Opens `audit/alerts.lock`. A reader and an acknowledgement (`create` false)
/// open an existing lock read-only, which is all `flock` needs, so a store
/// that cannot be written can still be read; with no lock and no queue there
/// is nothing to lock and nothing is created.
fn open_lock(names: &Names, create: bool) -> Result<File, AlertError> {
    let present = regular(&names.lock)?;
    if present && !create {
        return File::open(&names.lock).map_err(|why| AlertError::io("open the lock", &why));
    }
    if !present && !create && !regular(&names.queue)? {
        return Err(AlertError::Io {
            op: "open the lock",
            kind: io::ErrorKind::NotFound,
            text: "no queue".to_owned(),
        });
    }
    OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&names.lock)
        .map_err(|why| AlertError::io("open the lock", &why))
}

fn root_is_dir(names: &Names) -> Result<(), AlertError> {
    if names.root.is_dir() {
        Ok(())
    } else {
        Err(AlertError::Unavailable {
            root: names.root.clone(),
        })
    }
}

fn sync_dir(dir: &Path) -> Result<(), AlertError> {
    File::open(dir)
        .and_then(|d| d.sync_all())
        .map_err(|why| AlertError::io("sync the directory", &why))
}

/// Creates a file that must not exist, removing one stale or planted name
/// first. `create_new` never follows a link, so nothing behind one is written.
fn create_fresh(path: &Path) -> io::Result<File> {
    let fresh = || OpenOptions::new().write(true).create_new(true).open(path);
    match fresh() {
        Err(why) if why.kind() == io::ErrorKind::AlreadyExists => {
            fs::remove_file(path)?;
            fresh()
        }
        other => other,
    }
}

/// A whole new queue under the temporary name `audit/alerts.<pid>.tmp`: the
/// header and, for a replacement, its first record, in one write, synced.
/// Nothing a reader opens is touched. Called under the exclusive lock.
fn write_new(names: &Names, now: u64, first: Option<&Record>) -> Result<PathBuf, AlertError> {
    let tmp = names.dir.join(format!("alerts.{}.tmp", std::process::id()));
    let mut bytes = header(now);
    if let Some(record) = first {
        bytes.extend_from_slice(&record.encode());
    }
    let written = create_fresh(&tmp).and_then(|mut file| {
        file.write_all(&bytes)?;
        file.sync_all()
    });
    if let Err(why) = written {
        let _already_gone = fs::remove_file(&tmp);
        return Err(AlertError::io("write the new queue", &why));
    }
    Ok(tmp)
}

/// Genesis: the header is written to a temporary name, synced and linked into
/// place; the link fails if a queue exists, so a queue is born whole or not at
/// all. Called under the exclusive lock.
fn genesis(names: &Names, now: u64) -> Result<(), AlertError> {
    let tmp = write_new(names, now, None)?;
    let linked = fs::hard_link(&tmp, &names.queue);
    let _tmp_removed = fs::remove_file(&tmp);
    linked.map_err(|why| AlertError::io("link the new queue", &why))?;
    sync_dir(&names.dir)?;
    note_created(&names.queue);
    Ok(())
}

/// What the reserve file did for one put.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reserve {
    /// It is in place: [`RESERVE_RECORDS`] records of space held back.
    Held,
    /// The disk was full; it was given up so this alert could be written.
    Spent,
    /// It is absent and could not be made.
    Absent(io::ErrorKind),
}

/// Keeps `audit/alerts.reserve` in place: 64 KiB of written zeros that a put
/// meeting a full disk deletes, so the `disk_full` alert itself has room.
fn hold_reserve(names: &Names) -> Reserve {
    match fs::symlink_metadata(&names.reserve) {
        Ok(meta) if meta.file_type().is_file() && meta.len() >= RESERVE_RECORDS * RECORD_LEN => {
            return Reserve::Held;
        }
        _ => {}
    }
    let bytes = usize::try_from(RESERVE_RECORDS * RECORD_LEN).unwrap_or(0);
    let made = create_fresh(&names.reserve).and_then(|mut file| {
        file.write_all(&vec![0_u8; bytes])?;
        file.sync_all()
    });
    match made {
        Ok(()) => Reserve::Held,
        Err(why) => {
            let _partial = fs::remove_file(&names.reserve);
            Reserve::Absent(why.kind())
        }
    }
}

#[cfg(test)]
thread_local! {
    /// Test seam: how many of this thread's next record writes meet a full disk.
    pub(crate) static FULL_WRITES: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

/// Whether the test seam says this write meets a full disk.
#[cfg(test)]
fn seam_says_full() -> bool {
    FULL_WRITES.with(|left| {
        let now = left.get();
        left.set(now.saturating_sub(1));
        now > 0
    })
}

/// Outside tests there is no seam: the host alone says when a disk is full.
#[cfg(not(test))]
const fn seam_says_full() -> bool {
    false
}

fn write_once(file: &File, bytes: &[u8], at: u64) -> io::Result<()> {
    if seam_says_full() {
        return Err(io::Error::from(io::ErrorKind::StorageFull));
    }
    file.write_all_at(bytes, at)?;
    file.sync_data()
}

/// One 128-byte write and one `sync_data`. A full disk spends the reserve and
/// tries once more; any other refusal, or a second one, is returned.
fn write_record(names: &Names, file: &File, bytes: &[u8], at: u64) -> Result<Reserve, AlertError> {
    match write_once(file, bytes, at) {
        Ok(()) => Ok(hold_reserve(names)),
        Err(why) if why.kind() == io::ErrorKind::StorageFull => {
            let freed = fs::remove_file(&names.reserve);
            let _synced = sync_dir(&names.dir);
            note_reserve_spent(freed.is_ok());
            write_once(file, bytes, at)
                .map(|()| Reserve::Spent)
                .map_err(|again| AlertError::io("append after spending the reserve", &again))
        }
        Err(why) => Err(AlertError::io("append", &why)),
    }
}

/// The queue opened for a writer, under the exclusive lock, with its tail
/// rule applied.
struct Writer {
    names: Names,
    file: File,
    _lock: Flock<File>,
    span: Span,
    /// Slots of `span` that stand: all of them, or all but a bad last one.
    live: usize,
    /// The ordinal the next record takes.
    next: u64,
    /// The offset of a bad last record the next write goes over.
    over: Option<u64>,
    /// Bytes past the last whole record.
    torn: u64,
    created: bool,
}

impl Writer {
    fn open(root: &Path, now: u64, create: bool) -> Result<Self, AlertError> {
        let names = Names::under(root);
        root_is_dir(&names)?;
        if create {
            fs::create_dir_all(&names.dir).map_err(|why| AlertError::io("create audit/", &why))?;
        }
        let lock_file = open_lock(&names, create)?;
        let lock = Flock::lock(lock_file, names.lock.clone())
            .map_err(|why| AlertError::io("lock", &why))?;
        Self::under(names, lock, now, create)
    }

    /// The writer over a lock held exclusive: genesis when there is no queue
    /// and one may be made, then the window and the tail rule.
    fn under(names: Names, lock: Flock<File>, now: u64, create: bool) -> Result<Self, AlertError> {
        let mut created = false;
        if !regular(&names.queue)? {
            if !create {
                return Err(AlertError::NoSuchAlert { ordinal: 0 });
            }
            genesis(&names, now)?;
            let _first = hold_reserve(&names);
            created = true;
        }
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&names.queue)
            .map_err(|why| AlertError::io("open the queue", &why))?;
        let shape = shape(&file)?;
        let span = span(&file, shape.records)?;
        let mut live = span.slots.len();
        let mut next = shape.records + 1;
        let mut over = None;
        if matches!(span.slots.last(), Some(Slot::Bad)) {
            let before = live.checked_sub(2).and_then(|at| span.slots.get(at));
            if matches!(before, Some(Slot::Bad)) {
                return Err(AlertError::Damaged {
                    offset: offset_of(shape.records - 1),
                });
            }
            live -= 1;
            next = shape.records;
            over = Some(offset_of(shape.records));
        }
        Ok(Self {
            names,
            file,
            _lock: lock,
            span,
            live,
            next,
            over,
            torn: shape.torn,
            created,
        })
    }

    fn open_now(&self) -> u32 {
        self.span.newest_good(self.live).map_or(0, |r| r.open_after)
    }

    /// Appends `record` at the next ordinal.
    fn append(&self, record: &Record) -> Result<(u32, Reserve), AlertError> {
        let ordinal = u32::try_from(self.next).map_err(|_| AlertError::Full)?;
        let reserve = write_record(
            &self.names,
            &self.file,
            &record.encode(),
            offset_of(self.next),
        )?;
        if let Some(offset) = self.over {
            note_written_over(offset, ordinal);
        }
        Ok((ordinal, reserve))
    }

    /// Whether an acknowledgement among the live slots after `from` names `k`.
    fn acked_after(&self, from: usize, k: u32) -> bool {
        for slot in self.span.slots.iter().take(self.live).skip(from) {
            if let Slot::Good(r) = slot
                && r.kind_of_record == TYPE_ACK
                && r.acked == k
            {
                return true;
            }
        }
        false
    }
}

// ── The put ────────────────────────────────────────────────────────────────

/// What one put did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Put {
    /// One record was appended.
    Written {
        /// The new alert's ordinal.
        ordinal: u32,
        /// The text was cut to fit.
        text_cut: bool,
        /// The offset of an unsynced last record this put wrote over.
        wrote_over: Option<u64>,
        /// Bytes of a torn append this put wrote over.
        torn: u64,
        /// What the reserve did.
        reserve: Reserve,
        /// This put made the queue.
        created: bool,
    },
    /// An unacknowledged alert in the window carries the same key: nothing
    /// was written.
    Duplicate {
        /// That alert's ordinal.
        ordinal: u32,
    },
}

/// Puts one alert, stamped now. See [`put_at`].
///
/// # Errors
///
/// As [`put_at`].
pub fn put(root: &Path, alert: &NewAlert<'_>) -> Result<Put, AlertError> {
    put_at(root, alert, unix_now())
}

/// Puts one alert under `audit/alerts.lock` held exclusive: one read of the
/// newest [`WINDOW`] records, then nothing (a duplicate) or one append.
///
/// # Errors
///
/// [`AlertError`] by name; nothing is written then, and an error event says
/// which alert was not saved and why, so the failure is never silent.
pub fn put_at(root: &Path, alert: &NewAlert<'_>, now: u64) -> Result<Put, AlertError> {
    let done = put_inner(root, alert, now);
    match &done {
        Ok(Put::Written { ordinal, .. }) => note_put(*ordinal, alert),
        Ok(Put::Duplicate { .. }) => {}
        Err(why) => note_not_saved(alert, why),
    }
    done
}

fn put_inner(root: &Path, alert: &NewAlert<'_>, now: u64) -> Result<Put, AlertError> {
    if kind_word(alert.kind).is_none() {
        return Err(AlertError::KindUnknown { kind: alert.kind });
    }
    if alert.key.is_empty() {
        return Err(AlertError::KeyEmpty);
    }
    // Hashed before the lock is taken: the one step whose cost grows with the
    // caller's key is not paid while every other writer and reader waits.
    let key = key_hash(alert.key);
    let writer = Writer::open(root, now, true)?;
    put_with(&writer, alert, key, now)
}

/// The put proper, over a writer that holds the lock.
fn put_with(
    writer: &Writer,
    alert: &NewAlert<'_>,
    key: [u8; 16],
    now: u64,
) -> Result<Put, AlertError> {
    // Newest first: an acknowledgement is always newer than its alert, so it
    // is met before the alert it names (D-4895: an acknowledged key is put
    // again; only an open alert makes a duplicate).
    let mut acked = vec![false; writer.live];
    for (at, slot) in writer.span.slots.iter().enumerate().take(writer.live).rev() {
        let Slot::Good(record) = slot else { continue };
        if record.kind_of_record == TYPE_ACK {
            if let Some(mark) = writer
                .span
                .index(u64::from(record.acked))
                .and_then(|i| acked.get_mut(i))
            {
                *mark = true;
            }
        } else if record.kind_of_record == TYPE_ALERT
            && record.key == key
            && acked.get(at) == Some(&false)
        {
            let ordinal = u32::try_from(writer.span.ordinal(at)).map_err(|_| AlertError::Full)?;
            return Ok(Put::Duplicate { ordinal });
        }
    }
    let (text, text_cut) = fit_text(alert.text);
    let record = Record {
        kind_of_record: TYPE_ALERT,
        kind: alert.kind,
        flags: if text_cut { FLAG_TEXT_CUT } else { 0 },
        day: alert.day,
        run: alert.run,
        put_at: now,
        key,
        acked: 0,
        open_after: writer.open_now().saturating_add(1),
        text,
    };
    let (ordinal, reserve) = writer.append(&record)?;
    Ok(Put::Written {
        ordinal,
        text_cut,
        wrote_over: writer.over,
        torn: writer.torn,
        reserve,
        created: writer.created,
    })
}

// ── The acknowledgement ────────────────────────────────────────────────────

/// How far an acknowledgement may look.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// The newest [`WINDOW`] records only: O(1). A route's.
    ///
    /// UNVERIFIED: this cost names no test or bench that proves it (docs/06-limits.md,
    /// "Cost claims of the /db fix round that name no proof").
    Window,
    /// The whole file when the alert is older: O(records after it). The cli's.
    Whole,
}

/// What one acknowledgement did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Acked {
    /// The alert that was acknowledged.
    pub ordinal: u32,
    /// The acknowledgement's own ordinal.
    pub record: u32,
    /// The alert's kind.
    pub kind: u8,
    /// The alert's run.
    pub run: u64,
    /// Unacknowledged alerts left in the file.
    pub open_after: u32,
}

/// Acknowledges alert `ordinal`, stamped now. See [`ack_at`].
///
/// # Errors
///
/// As [`ack_at`].
pub fn ack(root: &Path, ordinal: u32, by: &str, scope: Scope) -> Result<Acked, AlertError> {
    ack_at(root, ordinal, by, scope, unix_now())
}

/// Appends an acknowledgement of alert `ordinal` under the exclusive lock.
/// `by` says who (`cli`, `api`); it is kept in the record's text and in the
/// audited "alert acknowledged" event.
///
/// # Errors
///
/// `no_such_alert` for 0, an ordinal past the end, or a record that is not an
/// alert; `already_acknowledged`; `ack_outside_window` under [`Scope::Window`]
/// for an alert older than the window; `alert_record_damaged` when the record
/// named fails its CRC; `alerts_damaged` for a queue the put refuses too; the
/// host's refusal.
pub fn ack_at(
    root: &Path,
    ordinal: u32,
    by: &str,
    scope: Scope,
    now: u64,
) -> Result<Acked, AlertError> {
    let missing = AlertError::NoSuchAlert { ordinal };
    let writer = match Writer::open(root, now, false) {
        Err(
            AlertError::NoSuchAlert { .. }
            | AlertError::Io {
                kind: io::ErrorKind::NotFound,
                ..
            },
        ) => return Err(missing),
        other => other?,
    };
    let k = u64::from(ordinal);
    if k == 0 || k >= writer.next {
        return Err(missing);
    }
    let alert = if let Some(at) = writer.span.index(k) {
        let Some(Slot::Good(record)) = writer.span.slots.get(at) else {
            return Err(AlertError::RecordDamaged {
                ordinal,
                offset: offset_of(k),
            });
        };
        if writer.acked_after(at, ordinal) {
            return Err(AlertError::AlreadyAcknowledged { ordinal });
        }
        record.clone()
    } else {
        if scope == Scope::Window {
            return Err(AlertError::AckOutsideWindow { ordinal });
        }
        older_alert(&writer, ordinal)?
    };
    if alert.kind_of_record != TYPE_ALERT {
        return Err(missing);
    }
    let (text, text_cut) = fit_text(by);
    let open_after = writer.open_now().saturating_sub(1);
    let record = Record {
        kind_of_record: TYPE_ACK,
        kind: 0,
        flags: if text_cut { FLAG_TEXT_CUT } else { 0 },
        day: alert.day,
        run: alert.run,
        put_at: now,
        key: [0; 16],
        acked: ordinal,
        open_after,
        text,
    };
    let (at, _reserve) = writer.append(&record)?;
    note_acked(ordinal, alert.kind, alert.run, by);
    Ok(Acked {
        ordinal,
        record: at,
        kind: alert.kind,
        run: alert.run,
        open_after,
    })
}

/// The cli's half: alert `ordinal` is older than the window, so its own
/// record is one read at a known offset and "not acknowledged yet" is the
/// records after it, a window at a time (L-IA-34).
fn older_alert(writer: &Writer, ordinal: u32) -> Result<Record, AlertError> {
    let k = u64::from(ordinal);
    if writer.acked_after(0, ordinal) {
        return Err(AlertError::AlreadyAcknowledged { ordinal });
    }
    let mut end = writer.span.first.saturating_sub(1);
    loop {
        let part = span(&writer.file, end)?;
        for slot in &part.slots {
            if let Slot::Good(r) = slot
                && r.kind_of_record == TYPE_ACK
                && r.acked == ordinal
            {
                return Err(AlertError::AlreadyAcknowledged { ordinal });
            }
        }
        if let Some(at) = part.index(k) {
            return match part.slots.get(at) {
                Some(Slot::Good(record)) => Ok(record.clone()),
                _ => Err(AlertError::RecordDamaged {
                    ordinal,
                    offset: offset_of(k),
                }),
            };
        }
        end = part.first.saturating_sub(1);
        if end == 0 {
            return Err(AlertError::NoSuchAlert { ordinal });
        }
    }
}

// ── The readers ────────────────────────────────────────────────────────────

/// One alert as a reader lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Alert {
    /// Its ordinal, which an acknowledgement names.
    pub ordinal: u32,
    /// Its record.
    pub record: Record,
}

impl Alert {
    /// The kind's word, or `kind_<n>` for one a later build wrote.
    #[must_use]
    pub fn word(&self) -> Cow<'static, str> {
        kind_label(self.record.kind)
    }
}

/// One page of the queue: at most [`WINDOW`] records, read once.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Page {
    /// Whether a queue exists. No queue is no alert, not a failure.
    pub exists: bool,
    /// The alerts of this page that no acknowledgement in it names, newest
    /// first. For the newest page that is every acknowledgement there is; for
    /// an older page [`page`] has not read the records after it, and
    /// [`page_settled`] has.
    pub alerts: Vec<Alert>,
    /// Ordinals of records in this page that fail their CRC: named, never
    /// skipped.
    pub damaged: Vec<u32>,
    /// Good records of a type this build cannot name (ordinal, type): a later
    /// build's, shown, never dropped.
    pub unknown: Vec<(u32, u8)>,
    /// Ordinals older than this page that an acknowledgement in it names; a
    /// caller paging backwards carries them to the older pages.
    pub acked_older: Vec<u32>,
    /// Unacknowledged alerts older than this page, as of its newest good
    /// record: that record's "open after" less the alerts listed. Exact for
    /// the newest page and for a page from [`page_settled`], which takes off
    /// the older alerts a later record acknowledges; with a damaged record in
    /// the page it includes it.
    pub older_unacknowledged: u32,
    /// Records AFTER this page that fail their CRC, counted by
    /// [`page_settled`]: an acknowledgement among them could not be applied.
    /// Always 0 from [`page`], which reads no later record.
    pub later_damaged: u32,
    /// The first ordinal of this page; pass it as `before` for the page
    /// before. 0 when the page is empty.
    pub first: u32,
    /// Whole records in the file.
    pub records: u64,
    /// Bytes past the last whole record (a torn append).
    pub torn: u64,
}

/// The newest page: what `/alerts.json` and `cli alerts` show.
///
/// # Errors
///
/// As [`page`].
pub fn window(root: &Path) -> Result<Page, AlertError> {
    page(root, None)
}

/// What the walk seam runs: one step, given the store root.
#[cfg(test)]
pub(crate) type BetweenReads = Box<dyn FnOnce(&Path)>;

#[cfg(test)]
thread_local! {
    /// Test seam: run once, with the store root, before the next later read of
    /// a walk on this thread (no lock is held then), as another process's put
    /// or replacement would land between two of a walk's reads.
    pub(crate) static BETWEEN_READS: std::cell::RefCell<Option<BetweenReads>> =
        const { std::cell::RefCell::new(None) };
}

/// Runs the test seam's step, once.
#[cfg(test)]
fn seam_between_reads(root: &Path) {
    let step = BETWEEN_READS.with(|held| held.borrow_mut().take());
    if let Some(step) = step {
        step(root);
    }
}

/// Outside tests there is no seam: only another process lands between reads.
#[cfg(not(test))]
const fn seam_between_reads(_root: &Path) {}

/// One queue held open for every read of one answer.
///
/// The open file is the answer's identity. A replacement puts a NEW file on
/// the name `audit/alerts` and keeps the damaged one under its kept name, so
/// every read through this one descriptor is of the queue the first read
/// opened, whatever the name holds by then: a walk of many reads is one view
/// of one queue, never part of one and part of another. Records are only
/// appended, so the `records` the first read counted are the same bytes at
/// every later read; the one exception is a bad LAST record, which the next
/// put writes over, and a later read takes the lock shared, so it meets the
/// bad record or the whole new one, never a half.
struct View {
    names: Names,
    lock: File,
    file: File,
    /// Whole records and torn bytes as the first read saw them.
    shape: Shape,
}

impl View {
    /// The first read, under `audit/alerts.lock` held shared, so a reader
    /// never meets a put's half-written record: the whole records `pick`
    /// chooses (first ordinal, count) from the number in the file. `None`
    /// when there is no queue.
    fn open(
        root: &Path,
        pick: impl FnOnce(u64) -> (u64, u64),
    ) -> Result<Option<(Self, Span)>, AlertError> {
        let names = Names::under(root);
        root_is_dir(&names)?;
        // The lock first, the queue second: a replacement holds the lock while
        // it moves the new queue onto the name, so a reader opens the damaged
        // queue or the new one whole, never a name in between.
        let lock = match open_lock(&names, false) {
            Ok(file) => file,
            Err(AlertError::Io {
                kind: io::ErrorKind::NotFound,
                ..
            }) => return Ok(None),
            Err(other) => return Err(other),
        };
        let held = Flock::lock_shared(&lock, names.lock.as_path())
            .map_err(|why| AlertError::io("lock shared", &why))?;
        if !regular(&names.queue)? {
            return Ok(None);
        }
        let file =
            File::open(&names.queue).map_err(|why| AlertError::io("open the queue", &why))?;
        let shape = shape(&file)?;
        let (first, count) = pick(shape.records);
        let span = span_from(&file, first, count)?;
        drop(held);
        Ok(Some((
            Self {
                names,
                lock,
                file,
                shape,
            },
            span,
        )))
    }

    /// A later read of the same open file: `count` whole records from ordinal
    /// `first`, all within the records the first read counted. The lock is
    /// held shared for this one read only, so a put never waits for a walk.
    fn read(&self, first: u64, count: u64) -> Result<Span, AlertError> {
        seam_between_reads(&self.names.root);
        let held = Flock::lock_shared(&self.lock, self.names.lock.as_path())
            .map_err(|why| AlertError::io("lock shared", &why))?;
        let span = span_from(&self.file, first, count)?;
        drop(held);
        Ok(span)
    }

    /// A span of this queue as a page.
    fn page(&self, span: &Span) -> Page {
        let mut page = listed(span);
        page.exists = true;
        page.records = self.shape.records;
        page.torn = self.shape.torn;
        page
    }
}

/// The at most [`WINDOW`] records before ordinal `before` (the newest when
/// `None`) of a queue of `records`: first ordinal and count.
fn span_before(records: u64, before: Option<u32>) -> (u64, u64) {
    let end = before.map_or(records, |b| u64::from(b).saturating_sub(1).min(records));
    let count = end.min(WINDOW);
    (end - count + 1, count)
}

/// One read of the at most [`WINDOW`] records before ordinal `before` (the
/// newest when `None`), under `audit/alerts.lock` held shared, so a reader
/// never meets a put's half-written record. O(1) at any `before`.
///
/// A page before the newest lists its alerts as of its own last record: an
/// acknowledgement in a later record is not applied. A caller paging
/// backwards carries [`Page::acked_older`] from the newer pages;
/// [`page_settled`] is the reader that does it for one page.
///
/// # Errors
///
/// `alerts_unavailable` for a root that is no directory; `alerts_damaged`
/// for a short or failed header; `alerts_version_unknown`; the host's refusal.
///
/// UNVERIFIED as a bound: `the_operators_older_steps_timed` times this at 102,400
/// records and asserts no time; it is ignored and run by hand.
pub fn page(root: &Path, before: Option<u32>) -> Result<Page, AlertError> {
    let read = View::open(root, |records| span_before(records, before))?;
    Ok(read.map_or_else(Page::default, |(view, span)| view.page(&span)))
}

/// Applies the acknowledgements of newer records to a page read by [`page`]:
/// an alert one of them names leaves the list, and each one that names an
/// alert older than the page lowers the page's older count by one (the
/// writer acknowledges an alert at most once). What is left in `carried` are
/// the ordinals below the page, for the page before it.
fn settle(page: &mut Page, carried: &mut BTreeSet<u32>) {
    page.alerts
        .retain(|alert| !carried.contains(&alert.ordinal));
    let _named_this_page_or_newer = carried.split_off(&page.first);
    let older = u32::try_from(carried.len()).unwrap_or(u32::MAX);
    page.older_unacknowledged = page.older_unacknowledged.saturating_sub(older);
}

/// The page before ordinal `before` as it stood at this call's first read:
/// [`page`]'s one read, then every record after that page up to the count
/// that read saw, a window at a time, for the acknowledgements that name its
/// alerts or older ones. Its list and its older count therefore agree with
/// [`window`] and [`older`]: no alert a record acknowledges is shown as
/// unacknowledged. O(records after the page); the cli's
/// (`docs/06-limits.md` L-IA-34), never a route's.
///
/// Every read is of the one file the first read opened, so a replacement
/// that lands between two of them changes nothing in the answer: it is the
/// page of the queue that was replaced, whole.
///
/// # Errors
///
/// As [`page`].
pub fn page_settled(root: &Path, before: u32) -> Result<Page, AlertError> {
    let Some((view, span)) = View::open(root, |records| span_before(records, Some(before)))? else {
        return Ok(Page::default());
    };
    let mut page = view.page(&span);
    if page.first == 0 {
        return Ok(page);
    }
    let top = view.shape.records;
    let last = u64::from(before).saturating_sub(1).min(top);
    let mut carried = BTreeSet::new();
    let mut from = last + 1;
    while from <= top {
        let count = (top - from + 1).min(WINDOW);
        // Every record of this span is newer than the page, so an
        // acknowledgement of anything at or below `last` is "older" to it.
        let part = listed(&view.read(from, count)?);
        carried.extend(
            part.acked_older
                .iter()
                .copied()
                .filter(|k| u64::from(*k) <= last),
        );
        let bad = u32::try_from(part.damaged.len()).unwrap_or(u32::MAX);
        page.later_damaged = page.later_damaged.saturating_add(bad);
        from += count;
    }
    settle(&mut page, &mut carried);
    page.acked_older.extend(carried);
    Ok(page)
}

/// A span as a page: newest first, acknowledgements met before their alerts.
fn listed(span: &Span) -> Page {
    let mut page = Page {
        first: if span.slots.is_empty() {
            0
        } else {
            u32::try_from(span.first).unwrap_or(u32::MAX)
        },
        ..Page::default()
    };
    let mut acked = vec![false; span.slots.len()];
    let mut newest_open = None;
    for (at, slot) in span.slots.iter().enumerate().rev() {
        let ordinal = u32::try_from(span.ordinal(at)).unwrap_or(u32::MAX);
        let Slot::Good(record) = slot else {
            page.damaged.push(ordinal);
            continue;
        };
        if newest_open.is_none() {
            newest_open = Some(record.open_after);
        }
        match record.kind_of_record {
            TYPE_ACK => match span.index(u64::from(record.acked)) {
                Some(i) => {
                    if let Some(mark) = acked.get_mut(i) {
                        *mark = true;
                    }
                }
                // Only a record older than the span is handed on: the writer
                // never acknowledges ordinal 0 or a record that does not
                // exist yet, and an acknowledgement that names one (a
                // crafted record) takes nothing off any older count.
                None => {
                    if record.acked >= 1 && u64::from(record.acked) < span.first {
                        page.acked_older.push(record.acked);
                    }
                }
            },
            TYPE_ALERT => {
                if acked.get(at) == Some(&false) {
                    page.alerts.push(Alert {
                        ordinal,
                        record: record.clone(),
                    });
                }
            }
            other => page.unknown.push((ordinal, other)),
        }
    }
    let shown = u32::try_from(page.alerts.len()).unwrap_or(u32::MAX);
    page.older_unacknowledged = newest_open.unwrap_or(0).saturating_sub(shown);
    page
}

/// What [`older`] found: the alerts older than the newest window and every
/// record there it could not read or name.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Older {
    /// Whether a queue exists. No queue is no alert, not a failure.
    pub exists: bool,
    /// The unacknowledged alerts older than the newest window, newest first.
    pub alerts: Vec<Alert>,
    /// Ordinals older than the newest window whose record fails its CRC,
    /// newest first: named, never skipped. One that was an open alert is
    /// counted by [`Page::older_unacknowledged`] and cannot be listed; one
    /// that was an acknowledgement is not applied, so its alert is in
    /// `alerts` again.
    pub damaged: Vec<u32>,
    /// Good records older than the newest window of a type this build cannot
    /// name (ordinal, type), newest first: a later build's, shown, never
    /// dropped.
    pub unknown: Vec<(u32, u8)>,
    /// Records of the newest window that fail their CRC: an acknowledgement
    /// among them could not be applied to an older alert. [`window`] names
    /// them.
    pub newer_damaged: u32,
    /// The first ordinal of the newest window (every record below it was
    /// walked); 0 when the queue holds no record.
    pub first: u32,
    /// Whole records in the file at this call's first read.
    pub records: u64,
}

/// The unacknowledged alerts older than the newest window, newest first, as
/// the queue stood at this call's first read: every page before the window,
/// read backwards to the header, carrying the acknowledged ordinals. A
/// record that fails its CRC or is of a later build's type is returned by
/// its ordinal beside the alerts, as [`window`] and [`page_settled`] return
/// theirs. O(records in the file); the cli's (L-IA-34).
///
/// Every read is of the one file the first read opened, so the listing is of
/// one queue even when a replacement lands in the middle of the walk.
///
/// # Errors
///
/// As [`page`].
pub fn older(root: &Path) -> Result<Older, AlertError> {
    let Some((view, span)) = View::open(root, |records| span_before(records, None))? else {
        return Ok(Older::default());
    };
    let newest = view.page(&span);
    let mut carried: BTreeSet<u32> = newest.acked_older.iter().copied().collect();
    let mut found = Older {
        exists: true,
        newer_damaged: u32::try_from(newest.damaged.len()).unwrap_or(u32::MAX),
        first: newest.first,
        records: newest.records,
        ..Older::default()
    };
    let mut end = span.first.saturating_sub(1);
    while end > 0 {
        let count = end.min(WINDOW);
        let mut part = listed(&view.read(end - count + 1, count)?);
        settle(&mut part, &mut carried);
        found.alerts.append(&mut part.alerts);
        found.damaged.append(&mut part.damaged);
        found.unknown.append(&mut part.unknown);
        carried.extend(part.acked_older.iter().copied());
        end -= count;
    }
    Ok(found)
}

/// One record of a kept file, as [`of_file`] hands it over.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kept {
    /// A record that passes its CRC, with its ordinal.
    Good(u32, Record),
    /// A record that fails its CRC, with its ordinal.
    Damaged(u32),
}

/// What [`of_file`] found around the records.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeptShape {
    /// Whether the header passed its checks.
    pub header_ok: bool,
    /// Whole records by the file's length.
    pub records: u64,
    /// Bytes past the last whole record.
    pub torn: u64,
}

/// Lists every record of a kept queue file (`audit/alerts.damaged-<unix_s>`),
/// oldest first, a window at a time: good ones whole, damaged ones by
/// ordinal. A failed header is said and the records are still listed.
/// O(records in that file); the cli's (L-IA-34). Takes no lock: a kept file
/// is never written again.
///
/// # Errors
///
/// `alerts_not_a_file`, or the host's refusal.
pub fn of_file(path: &Path, each: &mut dyn FnMut(Kept)) -> Result<KeptShape, AlertError> {
    if !regular(path)? {
        return Err(AlertError::Io {
            op: "open the kept file",
            kind: io::ErrorKind::NotFound,
            text: path.display().to_string(),
        });
    }
    let file = File::open(path).map_err(|why| AlertError::io("open the kept file", &why))?;
    let header_ok = shape(&file).is_ok();
    let len = file
        .metadata()
        .map_err(|why| AlertError::io("stat the kept file", &why))?
        .len();
    let body = len.saturating_sub(HEADER_LEN);
    let records = body / RECORD_LEN;
    let mut done = 0;
    while done < records {
        let count = (records - done).min(WINDOW);
        let part = span_from(&file, done + 1, count)?;
        for (at, slot) in part.slots.iter().enumerate() {
            let ordinal = u32::try_from(part.ordinal(at)).unwrap_or(u32::MAX);
            each(match slot {
                Slot::Good(record) => Kept::Good(ordinal, record.clone()),
                Slot::Bad => Kept::Damaged(ordinal),
            });
        }
        done += count;
    }
    Ok(KeptShape {
        header_ok,
        records,
        torn: body % RECORD_LEN,
    })
}

// ── The replacement ────────────────────────────────────────────────────────

/// What a replacement did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Replaced {
    /// The damaged file, kept for ever under this name.
    pub kept: PathBuf,
    /// The unacknowledged alerts it held by its newest good record, or `None`
    /// when no record of its window passes its CRC (a failed header alone
    /// does not make it unknown).
    pub open: Option<u32>,
    /// The ordinal of the `queue_replaced` alert in the new queue.
    pub ordinal: u32,
}

/// Replaces a damaged queue, stamped now. See [`replace_damaged_at`].
///
/// # Errors
///
/// As [`replace_damaged_at`].
pub fn replace_damaged(root: &Path) -> Result<Replaced, AlertError> {
    replace_damaged_at(root, unix_now())
}

#[cfg(test)]
thread_local! {
    /// Test seam: the step of a replacement after which this thread's next
    /// one stops, as a kill there would (0: never). The steps are those
    /// [`replace_damaged_at`] numbers.
    pub(crate) static REPLACEMENT_STOPS_AFTER: std::cell::Cell<u8> = const { std::cell::Cell::new(0) };
}

/// Whether the test seam stops the replacement after `step`.
#[cfg(test)]
fn seam_stops_after(step: u8) -> bool {
    REPLACEMENT_STOPS_AFTER.with(std::cell::Cell::get) == step
}

/// Outside tests there is no seam: only a kill stops a replacement.
#[cfg(not(test))]
const fn seam_stops_after(_step: u8) -> bool {
    false
}

/// The name a damaged queue is kept under: a hard link, which never writes
/// over an earlier kept file (a second replacement in the same second takes
/// `.2`, `.3`, ...). A replacement that was stopped after this step left the
/// queue with a second name; the next one finds it by device and inode and
/// keeps that name, so one damaged file is kept once. The look is taken only
/// when the queue has a second link, and reads `audit/`'s names once.
fn keep(names: &Names, now: u64) -> Result<PathBuf, AlertError> {
    let queue = fs::symlink_metadata(&names.queue)
        .map_err(|why| AlertError::io("stat the damaged queue", &why))?;
    if queue.nlink() > 1 {
        let listing =
            fs::read_dir(&names.dir).map_err(|why| AlertError::io("list audit/", &why))?;
        let mut already: Option<PathBuf> = None;
        for entry in listing {
            let entry = entry.map_err(|why| AlertError::io("list audit/", &why))?;
            if !entry
                .file_name()
                .to_string_lossy()
                .starts_with("alerts.damaged-")
            {
                continue;
            }
            let path = entry.path();
            let same = fs::symlink_metadata(&path)
                .is_ok_and(|meta| meta.dev() == queue.dev() && meta.ino() == queue.ino());
            if same && already.as_ref().is_none_or(|kept| path < *kept) {
                already = Some(path);
            }
        }
        if let Some(kept) = already {
            return Ok(kept);
        }
    }
    let mut kept = names.dir.join(format!("alerts.damaged-{now}"));
    let mut attempt = 1_u32;
    loop {
        match fs::hard_link(&names.queue, &kept) {
            Ok(()) => return Ok(kept),
            Err(why) if why.kind() == io::ErrorKind::AlreadyExists && attempt < 64 => {
                attempt += 1;
                kept = names.dir.join(format!("alerts.damaged-{now}.{attempt}"));
            }
            Err(why) => return Err(AlertError::io("keep the damaged queue", &why)),
        }
    }
}

/// Under the exclusive lock: keeps a queue the put refuses as
/// `audit/alerts.damaged-<unix_s>` and puts a new queue in its place whose
/// first record is one `queue_replaced` alert naming the kept file and its
/// open count.
///
/// All or nothing, for a reader and across a kill. The steps, each durable
/// before the next:
///
/// 1. the damaged queue gets its kept name (a hard link; the directory is
///    synced), so it can never be lost;
/// 2. the new queue, header AND first record, is written whole under the
///    temporary name and synced;
/// 3. the temporary name is renamed over `audit/alerts`, which the host does
///    atomically, and the directory is synced;
/// 4. the reserve is made and the two events are emitted.
///
/// Stopped before step 3, `audit/alerts` is still the damaged queue and the
/// next replacement finishes the work under the same kept name; stopped
/// after it, the new queue already says what it replaced. No moment has no
/// queue, or a queue without that record.
///
/// # Errors
///
/// `alerts_not_damaged` when the put accepts the queue (a healthy queue is
/// never replaced) or none exists; the host's refusal.
pub fn replace_damaged_at(root: &Path, now: u64) -> Result<Replaced, AlertError> {
    let names = Names::under(root);
    root_is_dir(&names)?;
    if !regular(&names.queue)? {
        return Err(AlertError::NotDamaged);
    }
    let lock_file = open_lock(&names, true)?;
    let lock =
        Flock::lock(lock_file, names.lock.clone()).map_err(|why| AlertError::io("lock", &why))?;
    let stop = |step: u8| {
        if seam_stops_after(step) {
            Err(AlertError::Io {
                op: "replace the queue",
                kind: io::ErrorKind::Interrupted,
                text: format!("stopped after step {step} by the test seam"),
            })
        } else {
            Ok(())
        }
    };
    let open = damaged_open_count(&names)?;

    // Step 1: the kept name, durable before anything else changes.
    let kept = keep(&names, now)?;
    stop(1)?;
    sync_dir(&names.dir)?;
    stop(2)?;

    // Step 2: the new queue, whole, under the temporary name.
    let name = kept
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let held = open.map_or_else(|| "unknown".to_owned(), |n| n.to_string());
    let said = format!("kept {name}; open alerts it held: {held}");
    let key = format!("queue_replaced:{name}");
    let alert = NewAlert {
        kind: KIND_QUEUE_REPLACED,
        key: &key,
        day: 0,
        run: 0,
        text: &said,
    };
    let (text, text_cut) = fit_text(alert.text);
    let first = Record {
        kind_of_record: TYPE_ALERT,
        kind: alert.kind,
        flags: if text_cut { FLAG_TEXT_CUT } else { 0 },
        day: alert.day,
        run: alert.run,
        put_at: now,
        key: key_hash(alert.key),
        acked: 0,
        open_after: 1,
        text,
    };
    let tmp = match write_new(&names, now, Some(&first)) {
        Ok(tmp) => tmp,
        Err(why) => {
            note_not_saved(&alert, &why);
            return Err(why);
        }
    };
    stop(3)?;

    // Step 3: one atomic rename. Before it the name is the damaged queue,
    // after it the new one with its first record; never anything else.
    if let Err(why) = fs::rename(&tmp, &names.queue) {
        let why = AlertError::io("put the new queue in place", &why);
        note_not_saved(&alert, &why);
        return Err(why);
    }
    stop(4)?;
    sync_dir(&names.dir)?;
    stop(5)?;

    // Step 4: the reserve and the events, with the lock still held.
    let _reserve = hold_reserve(&names);
    note_created(&names.queue);
    note_replaced(&name, &held);
    note_put(1, &alert);
    drop(lock);
    Ok(Replaced {
        kept,
        open,
        ordinal: 1,
    })
}

/// The open count of a queue that must be damaged: `Err(NotDamaged)` when the
/// put would accept it. `None` only when no record of its window passes its
/// CRC: a failed header does not hide the records behind it, which are where
/// the file's length puts them.
fn damaged_open_count(names: &Names) -> Result<Option<u32>, AlertError> {
    let file = File::open(&names.queue).map_err(|why| AlertError::io("open the queue", &why))?;
    let (header_ok, records) = match shape(&file) {
        Ok(shape) => (true, shape.records),
        Err(AlertError::Damaged { .. }) => {
            let len = file
                .metadata()
                .map_err(|why| AlertError::io("stat the queue", &why))?
                .len();
            (false, len.saturating_sub(HEADER_LEN) / RECORD_LEN)
        }
        Err(other) => return Err(other),
    };
    let span = span(&file, records)?;
    let live = span.slots.len();
    let last_two_bad = matches!(span.slots.last(), Some(Slot::Bad))
        && matches!(
            live.checked_sub(2).and_then(|at| span.slots.get(at)),
            Some(Slot::Bad)
        );
    if header_ok && !last_two_bad {
        return Err(AlertError::NotDamaged);
    }
    Ok(span.newest_good(live).map(|r| r.open_after))
}

// ── Events (D-4893): every one is audited; `EVENTS` is the table a test checks.

/// Every event this module emits: level, target, message.
pub const EVENTS: [(telemetry::Level, &str, &str); 7] = [
    (telemetry::Level::Info, TARGET, "alert put"),
    (telemetry::Level::Info, TARGET, "alert acknowledged"),
    (telemetry::Level::Warn, TARGET, "alert queue replaced"),
    (telemetry::Level::Info, TARGET, "alert queue created"),
    (telemetry::Level::Error, TARGET, "alert record written over"),
    (telemetry::Level::Error, TARGET, "alert reserve spent"),
    (telemetry::Level::Error, TARGET, "alert not saved"),
];

/// The most of a key an event copies. A key is its caller's and has no length
/// bound (the record holds its hash); an audited line is bounded, so it
/// carries this much of the key and the key's whole length.
pub const EVENT_KEY_MAX: usize = 160;

/// A key as an event carries it: cut at the largest character boundary at or
/// below [`EVENT_KEY_MAX`] bytes.
fn event_key(key: &str) -> &str {
    let mut end = key.len().min(EVENT_KEY_MAX);
    while end > 0 && !key.is_char_boundary(end) {
        end -= 1;
    }
    key.get(..end).unwrap_or("")
}

fn key_bytes(key: &str) -> telemetry::Value<'static> {
    telemetry::Value::Uint(u64::try_from(key.len()).unwrap_or(u64::MAX))
}

fn note_put(ordinal: u32, alert: &NewAlert<'_>) {
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::info(TARGET, "alert put")
            .with("ordinal", telemetry::Value::Uint(u64::from(ordinal)))
            .with("kind", telemetry::Value::Str(&kind_label(alert.kind)))
            .with("run", telemetry::Value::Uint(alert.run))
            .with("day", telemetry::Value::Uint(u64::from(alert.day)))
            .with("key", telemetry::Value::Str(event_key(alert.key)))
            .with("key_bytes", key_bytes(alert.key)),
    );
}

fn note_not_saved(alert: &NewAlert<'_>, why: &AlertError) {
    let said = why.to_string();
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::error(TARGET, "alert not saved")
            .with("kind", telemetry::Value::Str(&kind_label(alert.kind)))
            .with("run", telemetry::Value::Uint(alert.run))
            .with("key", telemetry::Value::Str(event_key(alert.key)))
            .with("key_bytes", key_bytes(alert.key))
            .with("code", telemetry::Value::Str(why.code()))
            .with("why", telemetry::Value::Str(&said)),
    );
}

fn note_acked(ordinal: u32, kind: u8, run: u64, by: &str) {
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::info(TARGET, "alert acknowledged")
            .with("ordinal", telemetry::Value::Uint(u64::from(ordinal)))
            .with("kind", telemetry::Value::Str(&kind_label(kind)))
            .with("run", telemetry::Value::Uint(run))
            .with("by", telemetry::Value::Str(by)),
    );
}

fn note_replaced(kept: &str, open: &str) {
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::warn(TARGET, "alert queue replaced")
            .with("kept", telemetry::Value::Str(kept))
            .with("open", telemetry::Value::Str(open)),
    );
}

fn note_created(queue: &Path) {
    let path = queue.display().to_string();
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::info(TARGET, "alert queue created")
            .with("path", telemetry::Value::Str(&path)),
    );
}

fn note_written_over(offset: u64, ordinal: u32) {
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::error(TARGET, "alert record written over")
            .with("offset", telemetry::Value::Uint(offset))
            .with("ordinal", telemetry::Value::Uint(u64::from(ordinal))),
    );
}

fn note_reserve_spent(freed: bool) {
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::error(TARGET, "alert reserve spent")
            .with("freed", telemetry::Value::Bool(freed))
            .with(
                "bytes",
                telemetry::Value::Uint(RESERVE_RECORDS * RECORD_LEN),
            ),
    );
}

#[cfg(test)]
#[path = "alerts_tests.rs"]
mod tests;

//! `pull::gdfl_run::locks`: the lock files of a GDFL run, the claim written
//! into them, the request sidecar beside them, and the probe that reads them
//! (IA-2; D-4803, D-4804, D-4849, D-4905, D-4906, D-4907; design §5.3, §5.6).
//!
//! # The layout decides the locks (D-4803)
//!
//! | census | store lock `imports/gdfl.lock` | kind lock `imports/gdfl.<kind>.lock` | the claim is in |
//! |---|---|---|---|
//! | v3 (`Single`) | exclusive, for every run and verb | none | the store lock |
//! | v4 (`Partitioned`) | shared for a run, exclusive for a census verb | exclusive | the kind lock |
//!
//! A run takes the store lock, then its own kind lock, never another kind's,
//! so no two runs wait on each other; then it locates the census again and
//! refuses `census_layout_changed` when a migration finished in between. A
//! busy lock is tried [`PROBE_RETRY`] times [`PROBE_STEP`] apart before the
//! taker answers `run_busy` naming the holder's claim, because a probe
//! ([`lock_state`]) holds a lock only for one read of at most
//! [`PROBE_READ`] bytes (D-4849, `docs/06-limits.md` L-IA-25).
//!
//! # A lock file is the one its name names (RA-285)
//!
//! A lock file is opened `O_NOFOLLOW | O_NONBLOCK` (a link at the name is
//! refused, a pipe cannot hold the open), must be a regular file with exactly
//! one name (a second name would let the claim's truncate cut another file),
//! and after its flock is taken its device and inode are compared with what
//! the name holds NOW: a lock file removed or replaced under a live run
//! (a tidy-up, a restore, Finder) lets the next taker create a fresh file and
//! take a second lock of the same name, so the comparison refuses
//! `lock_replaced` instead, and a running run asks [`RunLocks::still_held`]
//! at every day's end.
//!
//! # No lock of this design waits for ever (D-4906)
//!
//! The journal append lock and the runs ledger lock are taken through
//! [`take_within`]: a `try_lock` every [`LOCK_STEP`], for at most
//! [`LOCK_WAIT`], and past it `lock_stalled{lock}` by name with an error
//! event. A holder stuck on a stalling drive (WD_BLACK on 29 Sep) no longer
//! hangs every run of every kind with no word.
//!
//! # The claim (D-4804) and the request sidecar (D-4907)
//!
//! The claim is one line of at most [`CLAIM_MAX`] bytes, every field bounded
//! by construction ([`Claim::line`]); it is written into the claim-holding
//! lock file (truncate, write, `sync_all`) under the runs ledger lock, right
//! after the run's id is allocated. The full filter and a repair reason do not
//! fit a bounded line: they go into the request sidecar
//! `imports/gdfl[.<kind>].request`, one line ending ` crc=<8 hex>`, written to
//! a staging name, synced and renamed (never torn), and the claim names the
//! sidecar's BLAKE3 in `request=` (or `request=-` when there is none, and then
//! any sidecar left by a dead run is removed). A reader that finds no sidecar,
//! a bad CRC or another digest says [`SidecarState::Stale`], and a claim that
//! does not parse is [`ClaimState::Unreadable`]: never auto-resumed.

use std::fmt::Write as _;
use std::fs::{self, File, OpenOptions, TryLockError};
use std::io::{self, ErrorKind};
use std::os::unix::fs::{FileExt as _, MetadataExt as _, OpenOptionsExt as _};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use brutex_core::vendor::Vendor;
use store::flock::Flock;
use store::open_flags::O_NOFOLLOW_NONBLOCK;
use telemetry::run_dirs::GdflLayout;

use crate::ingest::census_store::{Location, locate};

/// The events of the locks, the claim and the stop.
pub const TARGET: &str = "gdfl.import.lifecycle";

/// The GDFL store lock, beneath the store root (D-4803).
pub const STORE_LOCK: &str = "imports/gdfl.lock";
/// The journal append lock (D-4803, D-4905): every append to the journal,
/// the day ledger, the lineage and lots ledgers and the scope ledger.
pub const JOURNAL_LOCK: &str = "imports/gdfl.journal.lock";
/// How many times a busy run lock is tried before `run_busy` (D-4849).
pub const PROBE_RETRY: u32 = 10;
/// How far apart those tries are.
pub const PROBE_STEP: Duration = Duration::from_millis(10);
/// The longest a run waits for a lock a writer holds for one append or one
/// allocation (D-4906): past it, `lock_stalled`.
pub const LOCK_WAIT: Duration = Duration::from_secs(30);
/// How far apart the tries of [`take_within`] are.
pub const LOCK_STEP: Duration = Duration::from_millis(10);
/// The most bytes a probe reads of a lock file, and the most a claim holds.
pub const PROBE_READ: usize = 1_024;
/// The most bytes of a claim line, its newline included.
pub const CLAIM_MAX: usize = PROBE_READ;
/// The most bytes of a request sidecar (the request line's own bound).
pub const SIDECAR_MAX: usize = 8 * 1_024 + 32;
/// The most bytes of the encoded host name in a claim (RA-307).
pub const HOST_MAX: usize = 64;

/// How a lock is held.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hold {
    /// Shared: probes, and v4 runs on the store lock.
    Shared,
    /// Exclusive: runs on their claim lock, census verbs on the store lock.
    Exclusive,
}

/// Why a lock could not be taken, by name (design §5.8). Codes are never
/// renamed or reused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LockFault {
    /// Another process holds it; `holder` is its claim as read (at most
    /// [`PROBE_READ`] bytes, made printable), empty while it is being written.
    Busy {
        /// The lock's name beneath the store.
        lock: String,
        /// The holder's claim, as read.
        holder: String,
    },
    /// It could not be opened or taken: a link, a directory, a special file
    /// or a second name at its name, or the host's refusal.
    Refused {
        /// The lock's name beneath the store.
        lock: String,
        /// Why.
        why: String,
    },
    /// The file locked is no longer the one the name holds (RA-285).
    Replaced {
        /// The lock's name beneath the store.
        lock: String,
    },
    /// It was held past [`LOCK_WAIT`] (D-4906).
    Stalled {
        /// The lock's name beneath the store.
        lock: String,
        /// How long this taker waited, in milliseconds.
        waited_ms: u64,
    },
    /// The census layout changed between the first look and the locks: a
    /// migration finished in between (design §5.3).
    LayoutChanged {
        /// The layout the locks were taken for.
        locked: GdflLayout,
        /// The layout found after.
        found: GdflLayout,
    },
    /// The census layout could not be read.
    Layout {
        /// The census's words.
        why: String,
    },
}

impl LockFault {
    /// The code a refusal names it by.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Busy { .. } => "run_busy",
            Self::Refused { .. } => "run_lock_refused",
            Self::Replaced { .. } => "lock_replaced",
            Self::Stalled { .. } => "lock_stalled",
            Self::LayoutChanged { .. } => "census_layout_changed",
            Self::Layout { .. } => "census_unreadable",
        }
    }
}

impl core::fmt::Display for LockFault {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}: ", self.code())?;
        match self {
            Self::Busy { lock, holder } if holder.is_empty() => write!(
                f,
                "another process holds {lock}; its claim is being written"
            ),
            Self::Busy { lock, holder } => write!(f, "{lock} is held: {holder}"),
            Self::Refused { lock, why } => write!(f, "{lock}: {why}"),
            Self::Replaced { lock } => write!(
                f,
                "{lock} was removed or replaced after it was opened, so a lock on it excludes \
                 no one; nothing was written (no command deletes a lock file)"
            ),
            Self::Stalled { lock, waited_ms } => write!(
                f,
                "{lock} was held for {waited_ms} ms, past the {} s bound (D-4906); its holder \
                 is stuck or the drive stalls",
                LOCK_WAIT.as_secs()
            ),
            Self::LayoutChanged { locked, found } => write!(
                f,
                "the census was {} when the locks were taken and is {} now: a migration \
                 finished in between; every lock was released",
                layout_word(*locked),
                layout_word(*found)
            ),
            Self::Layout { why } => write!(f, "the census layout cannot be read: {why}"),
        }
    }
}

/// The layout's word.
#[must_use]
pub const fn layout_word(layout: GdflLayout) -> &'static str {
    match layout {
        GdflLayout::V3 => "v3",
        GdflLayout::V4 => "v4",
    }
}

/// The census layout of `store`, as the locks are chosen by it.
///
/// # Errors
///
/// [`LockFault::Layout`] with the census's words.
pub fn layout_at(store: &Path) -> Result<GdflLayout, LockFault> {
    match locate(store, Vendor::Gdfl) {
        Ok(Location::Single(_)) => Ok(GdflLayout::V3),
        Ok(Location::Partitioned(_)) => Ok(GdflLayout::V4),
        Err(why) => Err(LockFault::Layout { why }),
    }
}

/// The lock file of a kind on a v4 store, beneath the store root. `census`
/// has none: a census verb takes the store lock alone.
#[must_use]
pub fn kind_lock(kind: &str) -> String {
    format!("imports/gdfl.{kind}.lock")
}

/// Where the claim of a run of `kind` on `layout` is written: the kind lock
/// on v4, the store lock on v3 and for a census verb.
#[must_use]
pub fn claim_lock(kind: &str, layout: GdflLayout) -> String {
    match layout {
        GdflLayout::V4 if kind != "census" => kind_lock(kind),
        _ => STORE_LOCK.to_owned(),
    }
}

/// The stop file of the run that holds `claim_lock` (design §5.5):
/// `imports/gdfl.<kind>.stop` beside `imports/gdfl.<kind>.lock`.
#[must_use]
pub fn stop_name(claim_lock: &str) -> String {
    claim_lock.strip_suffix(".lock").map_or_else(
        || format!("{claim_lock}.stop"),
        |stem| format!("{stem}.stop"),
    )
}

/// The request sidecar of the run that holds `claim_lock` (D-4907).
#[must_use]
pub fn sidecar_name(claim_lock: &str) -> String {
    claim_lock.strip_suffix(".lock").map_or_else(
        || format!("{claim_lock}.request"),
        |stem| format!("{stem}.request"),
    )
}

/// What is at `path`, in words, never followed.
fn what_is_at(path: &Path) -> String {
    match fs::symlink_metadata(path) {
        Ok(m) if m.file_type().is_symlink() => "a symbolic link (never followed)".to_owned(),
        Ok(m) if m.is_dir() => "a directory".to_owned(),
        Ok(m) if m.is_file() => "a regular file".to_owned(),
        Ok(_) => "a special file (a pipe, a socket or a device)".to_owned(),
        Err(why) => format!("unmeasurable ({why})"),
    }
}

/// Opens (creating) the lock file `name` beneath `store`: `imports/` (or
/// `audit/`) alone is made, never the store root above it, so a root whose
/// volume went is refused, never born again empty on the mount point.
///
/// # Errors
///
/// [`LockFault::Refused`] for a link, a directory, a special file or a second
/// name at the name, a store root that is gone, or the host's refusal.
pub fn open_lock_file(store: &Path, name: &str) -> Result<File, LockFault> {
    let path = store.join(name);
    let refused = |why: String| LockFault::Refused {
        lock: name.to_owned(),
        why,
    };
    if !store.is_dir() {
        return Err(refused(format!(
            "the store root {} is not a directory (gone, or never made)",
            store.display()
        )));
    }
    if let Some(parent) = path.parent() {
        match fs::create_dir(parent) {
            Ok(()) => {}
            Err(why) if why.kind() == ErrorKind::AlreadyExists => {}
            Err(why) => return Err(refused(format!("its folder: {why}"))),
        }
    }
    open_lock_path(&path, name)
}

/// Opens (creating) the lock file at `path`, named `name` in a refusal, by
/// [`open_lock_file`]'s rules, in a folder that exists.
///
/// # Errors
///
/// As [`open_lock_file`].
pub fn open_lock_path(path: &Path, name: &str) -> Result<File, LockFault> {
    let refused = |why: String| LockFault::Refused {
        lock: name.to_owned(),
        why,
    };
    let path = path.to_path_buf();
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o644)
        .custom_flags(O_NOFOLLOW_NONBLOCK)
        .open(&path)
        .map_err(|why| {
            refused(if fs::symlink_metadata(&path).is_err() {
                format!("cannot be created ({why})")
            } else {
                format!("is {} and cannot be opened ({why})", what_is_at(&path))
            })
        })?;
    let found = file.metadata().map_err(|why| refused(why.to_string()))?;
    if !found.is_file() {
        return Err(refused(format!(
            "is {}, not a regular file",
            what_is_at(&path)
        )));
    }
    if found.nlink() != 1 {
        return Err(refused(format!(
            "has {} names (a hard link to another file); never truncated or written through",
            found.nlink()
        )));
    }
    Ok(file)
}

/// Whether the open `file` is still the file `path` names (RA-285): one
/// `fstat` and one `lstat`, compared by device and inode.
#[must_use]
pub fn same_file(file: &File, path: &Path) -> bool {
    match (file.metadata(), fs::symlink_metadata(path)) {
        (Ok(open), Ok(named)) => open.dev() == named.dev() && open.ino() == named.ino(),
        _ => false,
    }
}

/// At most [`PROBE_READ`] bytes from the start of `file`, made printable:
/// control bytes and non-ASCII bytes become `\xNN`, so a claim of another
/// format or random bytes is shown and can forge no line.
#[must_use]
pub fn head_of(file: &File) -> String {
    let mut buf = vec![0_u8; PROBE_READ];
    let mut filled = 0;
    while let Some(rest) = buf.get_mut(filled..).filter(|rest| !rest.is_empty()) {
        match file.read_at(rest, filled as u64) {
            Ok(0) | Err(_) => break,
            Ok(n) => filled += n,
        }
    }
    buf.truncate(filled);
    printable(buf.trim_ascii_end())
}

/// `bytes` as one printable line.
#[must_use]
pub fn printable(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len());
    for &b in bytes {
        match b {
            b'\\' => out.push_str("\\\\"),
            b'\t' => out.push_str("\\t"),
            b'\n' => out.push_str("\\n"),
            b'\r' => out.push_str("\\r"),
            0x20..=0x7e => out.push(char::from(b)),
            _ => {
                let _ = write!(out, "\\x{b:02x}");
            }
        }
    }
    out
}

/// Takes `file`'s lock as `hold`, trying it `1 + retry` times [`PROBE_STEP`]
/// apart, then checks it is still the file `name` holds.
///
/// # Errors
///
/// [`LockFault::Busy`] with the holder's claim, [`LockFault::Replaced`], and
/// [`LockFault::Refused`] for the host's refusal.
pub fn take(
    file: File,
    store: &Path,
    name: &str,
    hold: Hold,
    retry: u32,
) -> Result<Flock<File>, LockFault> {
    let path = store.join(name);
    let reader = file.try_clone().map_err(|why| LockFault::Refused {
        lock: name.to_owned(),
        why: why.to_string(),
    })?;
    let mut file = Some(file);
    let mut attempt = 0_u32;
    loop {
        let Some(one) = file.take() else {
            return Err(LockFault::Refused {
                lock: name.to_owned(),
                why: "the lock file was lost between two tries".to_owned(),
            });
        };
        let tried = match hold {
            Hold::Shared => Flock::try_lock_shared(one, path.clone()),
            Hold::Exclusive => Flock::try_lock(one, path.clone()),
        };
        match tried {
            Ok(flock) => {
                if !same_file(&flock, &path) {
                    let _released = flock.release();
                    return Err(LockFault::Replaced {
                        lock: name.to_owned(),
                    });
                }
                return Ok(flock);
            }
            Err(TryLockError::WouldBlock) if attempt < retry => {
                attempt += 1;
                std::thread::sleep(PROBE_STEP);
                file = Some(reader.try_clone().map_err(|why| LockFault::Refused {
                    lock: name.to_owned(),
                    why: why.to_string(),
                })?);
            }
            Err(TryLockError::WouldBlock) => {
                return Err(LockFault::Busy {
                    lock: name.to_owned(),
                    holder: head_of(&reader),
                });
            }
            Err(TryLockError::Error(why)) => {
                return Err(LockFault::Refused {
                    lock: name.to_owned(),
                    why: format!("flock: {why}"),
                });
            }
        }
    }
}

/// Takes the lock file `name` beneath `store` exclusively, waiting at most
/// `wait` in [`LOCK_STEP`] tries (D-4906), and emits an error event when it
/// gives up. For a lock held for one append or one allocation, never for a
/// run.
///
/// # Errors
///
/// [`LockFault::Stalled`] past `wait`, and what [`open_lock_file`] and the
/// identity check refuse.
pub fn take_within(store: &Path, name: &str, wait: Duration) -> Result<Flock<File>, LockFault> {
    let file = open_lock_file(store, name)?;
    hold_within(file, &store.join(name), name, wait)
}

/// [`take_within`] for the lock file at `path` in a folder that exists (the
/// journal's lock beside the journal, D-4905).
///
/// # Errors
///
/// As [`take_within`].
pub fn take_path_within(path: &Path, wait: Duration) -> Result<Flock<File>, LockFault> {
    let name = path.file_name().map_or_else(
        || path.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    );
    let file = open_lock_path(path, &name)?;
    hold_within(file, path, &name, wait)
}

/// The counted `try_lock` loop of [`take_within`] (D-4906).
fn hold_within(
    file: File,
    path: &Path,
    name: &str,
    wait: Duration,
) -> Result<Flock<File>, LockFault> {
    let path = path.to_path_buf();
    let began = Instant::now();
    let mut file = Some(file);
    loop {
        let Some(one) = file.take() else {
            return Err(LockFault::Refused {
                lock: name.to_owned(),
                why: "the lock file was lost between two tries".to_owned(),
            });
        };
        let kept = one.try_clone().map_err(|why| LockFault::Refused {
            lock: name.to_owned(),
            why: why.to_string(),
        })?;
        match Flock::try_lock(one, path.clone()) {
            Ok(flock) => {
                if !same_file(&flock, &path) {
                    let _released = flock.release();
                    return Err(LockFault::Replaced {
                        lock: name.to_owned(),
                    });
                }
                return Ok(flock);
            }
            Err(TryLockError::WouldBlock) => {
                let waited = began.elapsed();
                if waited >= wait {
                    let waited_ms = u64::try_from(waited.as_millis()).unwrap_or(u64::MAX);
                    let _dropped_when_filtered = telemetry::emit(
                        &telemetry::Event::error(TARGET, "lock stalled")
                            .with("lock", name)
                            .with("waited_ms", waited_ms),
                    );
                    return Err(LockFault::Stalled {
                        lock: name.to_owned(),
                        waited_ms,
                    });
                }
                std::thread::sleep(LOCK_STEP);
                file = Some(kept);
            }
            Err(TryLockError::Error(why)) => {
                return Err(LockFault::Refused {
                    lock: name.to_owned(),
                    why: format!("flock: {why}"),
                });
            }
        }
    }
}

// ═══ The run's locks ════════════════════════════════════════════════════════

/// The locks a run or a census verb holds, and what it found in them.
#[derive(Debug)]
pub struct RunLocks {
    store_root: PathBuf,
    layout: GdflLayout,
    /// The store lock, shared (v4 run) or exclusive.
    store: Option<Flock<File>>,
    /// The kind lock (v4 run only).
    kind: Option<Flock<File>>,
    /// The name of the lock that holds the claim.
    claim_name: String,
    /// The claim a holder left without releasing: it died (the kernel freed
    /// its lock, the claim stayed).
    dead: Option<String>,
}

impl RunLocks {
    /// Takes the locks of a run of `kind` (`indices`, `stocks`, `options`)
    /// on `store`'s layout, each tried `1 + retry` times (D-4849): v3 the
    /// store lock exclusive; v4 the store lock shared, then the kind lock
    /// exclusive. Then the census is located again.
    ///
    /// # Errors
    ///
    /// Every [`LockFault`]; nothing is held on any of them.
    pub fn take(store: &Path, kind: &str, retry: u32) -> Result<Self, LockFault> {
        let layout = layout_at(store)?;
        let held = match layout {
            GdflLayout::V3 => {
                let file = open_lock_file(store, STORE_LOCK)?;
                let flock = take(file, store, STORE_LOCK, Hold::Exclusive, retry)?;
                (Some(flock), None)
            }
            GdflLayout::V4 => {
                let file = open_lock_file(store, STORE_LOCK)?;
                let shared = take(file, store, STORE_LOCK, Hold::Shared, retry)?;
                let name = kind_lock(kind);
                let file = open_lock_file(store, &name)?;
                let own = take(file, store, &name, Hold::Exclusive, retry)?;
                (Some(shared), Some(own))
            }
        };
        Self::held(store, (layout, true), held, claim_lock(kind, layout))
    }

    /// Takes the store lock exclusive for a census verb (`census-migrate`,
    /// `census-recount`, and `--seal`), tried `1 + retry` times: a run holds
    /// it shared (v4) or exclusive (v3), so the verb is refused `run_busy`
    /// while any GDFL run is live, and a probe's shared hold never makes it
    /// refuse (round 4, N-4).
    ///
    /// # Errors
    ///
    /// Every [`LockFault`].
    pub fn take_store(store: &Path, retry: u32) -> Result<Self, LockFault> {
        // A census verb takes the store lock whatever the census's state: it
        // may be the very verb that repairs it (a seal a crash left half done),
        // so the layout is recorded when it can be read and never checked.
        let layout = layout_at(store).unwrap_or(GdflLayout::V3);
        let file = open_lock_file(store, STORE_LOCK)?;
        let flock = take(file, store, STORE_LOCK, Hold::Exclusive, retry)?;
        Self::held(
            store,
            (layout, false),
            (Some(flock), None),
            STORE_LOCK.to_owned(),
        )
    }

    fn held(
        store: &Path,
        (layout, checked): (GdflLayout, bool),
        (store_lock, kind_lock): (Option<Flock<File>>, Option<Flock<File>>),
        claim_name: String,
    ) -> Result<Self, LockFault> {
        let mut locks = Self {
            store_root: store.to_path_buf(),
            layout,
            store: store_lock,
            kind: kind_lock,
            claim_name,
            dead: None,
        };
        match if checked {
            layout_at(store)
        } else {
            Ok(layout)
        } {
            Ok(found) if found == layout => {}
            Ok(found) => {
                locks.unlock();
                return Err(LockFault::LayoutChanged {
                    locked: layout,
                    found,
                });
            }
            Err(why) => {
                locks.unlock();
                return Err(why);
            }
        }
        let dead = locks.claim_file().map(head_of).unwrap_or_default();
        if !dead.is_empty() {
            let _dropped_when_filtered = telemetry::emit(
                &telemetry::Event::warn(TARGET, "run superseded")
                    .with("lock", locks.claim_name.as_str())
                    .with("old_claim", dead.as_str()),
            );
            locks.dead = Some(dead);
        }
        // A stop file left beside a lock whose holder released or died names
        // no live run: it is removed by the run that takes the lock (§5.5).
        let stop = store.join(stop_name(&locks.claim_name));
        if fs::symlink_metadata(&stop).is_ok_and(|m| m.is_file()) {
            let _removed = fs::remove_file(&stop);
        }
        Ok(locks)
    }

    fn claim_file(&self) -> Option<&File> {
        self.kind
            .as_deref()
            .or(self.store.as_deref())
            .filter(|_| self.store.is_some())
    }

    /// The census layout the locks were taken for.
    #[must_use]
    pub const fn layout(&self) -> GdflLayout {
        self.layout
    }

    /// The name of the lock that holds the claim.
    #[must_use]
    pub fn claim_name(&self) -> &str {
        &self.claim_name
    }

    /// The stop file of this run, beneath the store.
    #[must_use]
    pub fn stop_path(&self) -> PathBuf {
        self.store_root.join(stop_name(&self.claim_name))
    }

    /// The request sidecar of this run, beneath the store.
    #[must_use]
    pub fn sidecar_path(&self) -> PathBuf {
        self.store_root.join(sidecar_name(&self.claim_name))
    }

    /// The claim a dead holder left, if any.
    #[must_use]
    pub fn dead(&self) -> Option<&str> {
        self.dead.as_deref()
    }

    /// Writes `claim` into the claim-holding lock file: truncate, write,
    /// `sync_all` (D-4804). Called under the runs ledger lock, right after
    /// the run's id is allocated.
    ///
    /// # Errors
    ///
    /// The host's refusal, or a claim past [`CLAIM_MAX`].
    pub fn write_claim(&self, claim: &Claim) -> io::Result<()> {
        let line = claim.line();
        if line.len() > CLAIM_MAX {
            return Err(io::Error::other(format!(
                "a claim of {} bytes is past the {CLAIM_MAX}-byte bound",
                line.len()
            )));
        }
        let file = self
            .claim_file()
            .ok_or_else(|| io::Error::other("the claim lock is not held"))?;
        file.set_len(0)?;
        file.write_all_at(line.as_bytes(), 0)?;
        file.sync_all()
    }

    /// Whether every lock is still the file its name holds (RA-285): asked
    /// at every day's end, two `lstat`s and two `fstat`s.
    #[must_use]
    pub fn still_held(&self) -> bool {
        let store_ok = self
            .store
            .as_deref()
            .is_some_and(|f| same_file(f, &self.store_root.join(STORE_LOCK)));
        let kind_ok = self
            .kind
            .as_deref()
            .is_none_or(|f| same_file(f, &self.store_root.join(&self.claim_name)));
        store_ok && kind_ok
    }

    /// Releases: truncates the claim to 0 bytes and syncs it, removes the
    /// stop file and the request sidecar, and releases the flocks, kind lock
    /// first (D-4804). Every step is tried; the first refusal is answered.
    ///
    /// # Errors
    ///
    /// Each step that failed, by name; the flocks are released whatever.
    pub fn release(mut self) -> Result<(), String> {
        let mut faults = Vec::new();
        if let Some(file) = self.claim_file()
            && let Err(why) = file.set_len(0).and_then(|()| file.sync_all())
        {
            faults.push(format!(
                "{}: the claim was not cleared ({why})",
                self.claim_name
            ));
        }
        for path in [self.stop_path(), self.sidecar_path()] {
            match fs::remove_file(&path) {
                Ok(()) => {}
                Err(why) if why.kind() == ErrorKind::NotFound => {}
                Err(why) => faults.push(format!("{}: {why}", path.display())),
            }
        }
        self.unlock_into(&mut faults);
        if faults.is_empty() {
            Ok(())
        } else {
            Err(faults.join("; "))
        }
    }

    /// Releases the flocks, leaving the claim as it is (a run released
    /// before its claim was written, or a layout that changed).
    fn unlock(&mut self) {
        let mut ignored = Vec::new();
        self.unlock_into(&mut ignored);
    }

    fn unlock_into(&mut self, faults: &mut Vec<String>) {
        for flock in [self.kind.take(), self.store.take()].into_iter().flatten() {
            if let Err(why) = flock.release() {
                faults.push(why.to_string());
            }
        }
    }
}

impl Drop for RunLocks {
    /// A run that returns without [`RunLocks::release`] after its claim was
    /// written (an early refusal) still clears its claim: only a holder that
    /// died leaves one.
    fn drop(&mut self) {
        if let Some(file) = self.claim_file() {
            let _cleared = file.set_len(0);
        }
    }
}

// ═══ The claim ══════════════════════════════════════════════════════════════

/// A run's claim (D-4804, design §5.3): one line, at most [`CLAIM_MAX`]
/// bytes, every value a number, a day, a word of a fixed list, a digest, or
/// the percent-encoded host.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "the claim's four flags are four fields of its one-line format (§37.2)"
)]
pub struct Claim {
    /// The run's id from the runs ledger.
    pub run: u64,
    /// `import`, `verify`, `deferred`, `completeness`, `census-migrate`,
    /// `census-recount` or `dayflags`.
    pub mode: String,
    /// `indices`, `stocks`, `options` or `census`.
    pub kind: String,
    /// The press's kinds and ids, `k:id,...`; on v4 only its own.
    pub batch: String,
    /// `cm_zip`, `nfo_zips`, `tickstore`, `dayzips` or `-`.
    pub source: String,
    /// The first day, `yyyy-mm-dd`, or `-`.
    pub from: String,
    /// The last day, or `-`.
    pub to: String,
    /// `*`, or the filter's count and the first 16 hex digits of the BLAKE3
    /// of its sorted, comma-joined names: `<n>:<16 hex>`.
    pub filter: String,
    /// `inline` or `off`.
    pub verify: String,
    /// Whether restated days are read again.
    pub restate: bool,
    /// `0`, or the first 16 hex digits of the repair reason's BLAKE3.
    pub repair: String,
    /// The run's workers.
    pub workers: u32,
    /// Whether the api may resume it (always 0 for `by=cli`, D-4804).
    pub auto_resume: bool,
    /// The process.
    pub pid: u32,
    /// The host, percent-encoded and cut to [`HOST_MAX`] bytes, or `-`.
    pub host: String,
    /// The executable's identity `<dev>:<ino>:<mtime>:<len>`, or `-`.
    pub exe: String,
    /// `cli` or `api`.
    pub by: String,
    /// When the run started, UNIX seconds.
    pub started: u64,
    /// The run it resumes, or 0.
    pub resumes: u64,
    /// The day it resumes on, or `-`.
    pub resume_day: String,
    /// Whether the operator acknowledged a legacy open day.
    pub legacy_ack: bool,
    /// Whether an unstamped build was allowed to write (D-4894).
    pub unstamped: bool,
    /// The request sidecar's 16-hex BLAKE3, or `-` for none (D-4907).
    pub request: String,
}

/// The claim's field names, in their one order.
pub const CLAIM_FIELDS: [&str; 23] = [
    "run",
    "mode",
    "kind",
    "batch",
    "source",
    "from",
    "to",
    "filter",
    "verify",
    "restate",
    "repair",
    "workers",
    "auto_resume",
    "pid",
    "host",
    "exe",
    "by",
    "started",
    "resumes",
    "resume_day",
    "legacy_ack",
    "unstamped",
    "request",
];

/// The modes a claim may name.
pub const MODES: [&str; 7] = [
    "import",
    "verify",
    "deferred",
    "completeness",
    "census-migrate",
    "census-recount",
    "dayflags",
];

/// The kinds a claim may name.
pub const KINDS: [&str; 4] = ["indices", "stocks", "options", "census"];

/// The first 16 hex digits of the BLAKE3 of `bytes`.
#[must_use]
pub fn digest16(bytes: &[u8]) -> String {
    let hash = brutex_core::blake3::hash(bytes);
    let mut out = String::with_capacity(16);
    for b in hash.iter().take(8) {
        let _ = write!(out, "{b:02x}");
    }
    out
}

/// A filter as a claim names it: `*`, or `<n>:<16 hex>` of the sorted,
/// comma-joined names (bounded whatever the filter).
#[must_use]
pub fn filter_field(only: &[String]) -> String {
    if only.is_empty() {
        return "*".to_owned();
    }
    let mut names: Vec<&str> = only.iter().map(String::as_str).collect();
    names.sort_unstable();
    format!("{}:{}", names.len(), digest16(names.join(",").as_bytes()))
}

/// The host as a claim holds it (RA-307): every byte outside ASCII letters,
/// digits, `.` and `-` percent-encoded, cut on a whole escape at
/// [`HOST_MAX`] bytes; `-` for an empty name.
#[must_use]
pub fn host_field(host: &str) -> String {
    let mut out = String::new();
    for &b in host.as_bytes() {
        let piece = if b.is_ascii_alphanumeric() || b == b'.' || b == b'-' {
            char::from(b).to_string()
        } else {
            format!("%{b:02X}")
        };
        if out.len() + piece.len() > HOST_MAX {
            break;
        }
        out.push_str(&piece);
    }
    if out.is_empty() || out == "-" {
        "-".to_owned()
    } else {
        out
    }
}

/// The executable's identity `<dev>:<ino>:<mtime>:<len>` (design §5.6: the child compares it with the api's, IA-10a), or `-`.
#[must_use]
pub fn exe_field() -> String {
    std::env::current_exe().and_then(fs::metadata).map_or_else(
        |_| "-".to_owned(),
        |m| format!("{}:{}:{}:{}", m.dev(), m.ino(), m.mtime(), m.len()),
    )
}

/// Whether `value` is a claim value of field `name`.
fn value_ok(name: &str, value: &str) -> bool {
    let digits = |v: &str| !v.is_empty() && v.len() <= 20 && v.bytes().all(|b| b.is_ascii_digit());
    let flag = |v: &str| v == "0" || v == "1";
    let day = |v: &str| v == "-" || (v.len() == 10 && super::day_of(v).is_ok());
    let hex16 =
        |v: &str| v.len() == 16 && v.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'));
    match name {
        "run" | "workers" | "pid" | "started" | "resumes" => digits(value),
        "mode" => MODES.contains(&value),
        "kind" => KINDS.contains(&value),
        "batch" => {
            value.len() <= 120
                && value.split(',').all(|one| {
                    one.split_once(':')
                        .is_some_and(|(k, id)| KINDS.contains(&k) && digits(id))
                })
        }
        "source" => ["cm_zip", "nfo_zips", "tickstore", "dayzips", "-"].contains(&value),
        "from" | "to" | "resume_day" => day(value),
        "filter" => {
            value == "*"
                || value
                    .split_once(':')
                    .is_some_and(|(n, d)| digits(n) && hex16(d))
        }
        "verify" => value == "inline" || value == "off",
        "restate" | "auto_resume" | "legacy_ack" | "unstamped" => flag(value),
        "repair" => value == "0" || hex16(value),
        "host" => {
            value.len() <= HOST_MAX
                && !value.is_empty()
                && value
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'%'))
        }
        "exe" => {
            value == "-"
                || (value.len() <= 84
                    && value.split(':').count() == 4
                    && value.split(':').all(|p| {
                        let p = p.strip_prefix('-').unwrap_or(p);
                        digits(p)
                    }))
        }
        "by" => value == "cli" || value == "api",
        "request" => value == "-" || hex16(value),
        _ => false,
    }
}

impl Claim {
    /// The claim line, with its newline.
    #[must_use]
    pub fn line(&self) -> String {
        let flag = |b: bool| if b { "1" } else { "0" };
        format!(
            "claim run={} mode={} kind={} batch={} source={} from={} to={} filter={} verify={} \
             restate={} repair={} workers={} auto_resume={} pid={} host={} exe={} by={} \
             started={} resumes={} resume_day={} legacy_ack={} unstamped={} request={}\n",
            self.run,
            self.mode,
            self.kind,
            self.batch,
            self.source,
            self.from,
            self.to,
            self.filter,
            self.verify,
            flag(self.restate),
            self.repair,
            self.workers,
            flag(self.auto_resume),
            self.pid,
            self.host,
            self.exe,
            self.by,
            self.started,
            self.resumes,
            self.resume_day,
            flag(self.legacy_ack),
            flag(self.unstamped),
            self.request,
        )
    }

    /// Reads a claim line (its trailing newline optional).
    ///
    /// # Errors
    ///
    /// Why it is not one: a field out of order, missing, repeated, unknown,
    /// or a value outside its class, or a line past [`CLAIM_MAX`].
    pub fn parse(text: &str) -> Result<Self, String> {
        if text.len() > CLAIM_MAX {
            return Err(format!("{} bytes, past {CLAIM_MAX}", text.len()));
        }
        let text = text.strip_suffix('\n').unwrap_or(text);
        let mut words = text.split(' ');
        if words.next() != Some("claim") {
            return Err("does not begin with `claim`".to_owned());
        }
        let mut values: Vec<&str> = Vec::with_capacity(CLAIM_FIELDS.len());
        for name in CLAIM_FIELDS {
            let word = words
                .next()
                .ok_or_else(|| format!("field {name} is missing"))?;
            let value = word
                .strip_prefix(name)
                .and_then(|rest| rest.strip_prefix('='))
                .ok_or_else(|| format!("expected {name}=, found {word:?}"))?;
            if !value_ok(name, value) {
                return Err(format!("{name}={value:?} is not a value of {name}"));
            }
            values.push(value);
        }
        if let Some(extra) = words.next() {
            return Err(format!("an unknown field after request=: {extra:?}"));
        }
        let get = |at: usize| values.get(at).copied().unwrap_or_default();
        let number = |at: usize| get(at).parse::<u64>().map_err(|why| why.to_string());
        let small = |at: usize| get(at).parse::<u32>().map_err(|why| why.to_string());
        Ok(Self {
            run: number(0)?,
            mode: get(1).to_owned(),
            kind: get(2).to_owned(),
            batch: get(3).to_owned(),
            source: get(4).to_owned(),
            from: get(5).to_owned(),
            to: get(6).to_owned(),
            filter: get(7).to_owned(),
            verify: get(8).to_owned(),
            restate: get(9) == "1",
            repair: get(10).to_owned(),
            workers: small(11)?,
            auto_resume: get(12) == "1",
            pid: small(13)?,
            host: get(14).to_owned(),
            exe: get(15).to_owned(),
            by: get(16).to_owned(),
            started: number(17)?,
            resumes: number(18)?,
            resume_day: get(19).to_owned(),
            legacy_ack: get(20) == "1",
            unstamped: get(21) == "1",
            request: get(22).to_owned(),
        })
    }
}

// ═══ The request sidecar (D-4907) ═══════════════════════════════════════════

/// Writes `request` (the request line) as the sidecar at `path`: one line,
/// ` crc=<8 hex>` after it, to a staging name, synced, renamed, and the
/// directory synced; answers the digest the claim names in `request=`.
///
/// # Errors
///
/// The staging helper's refusal or the host's, by name.
pub fn write_sidecar(path: &Path, request: &str) -> Result<String, String> {
    let line = sidecar_line(request);
    if line.len() > SIDECAR_MAX {
        return Err(format!(
            "a request sidecar of {} bytes is past {SIDECAR_MAX}",
            line.len()
        ));
    }
    let staging = path.with_extension("request.tmp");
    let staged = store::staging::create(&staging).map_err(|why| why.to_string())?;
    let file = &staged.file;
    file.write_all_at(line.as_bytes(), 0)
        .and_then(|()| file.sync_all())
        .map_err(|why| format!("{}: {why}", staging.display()))?;
    fs::rename(&staging, path).map_err(|why| format!("{}: {why}", path.display()))?;
    if let Some(dir) = path.parent() {
        File::open(dir)
            .and_then(|d| d.sync_all())
            .map_err(|why| format!("{}: {why}", dir.display()))?;
    }
    Ok(digest16(line.as_bytes()))
}

/// The sidecar's one line for `request`.
#[must_use]
pub fn sidecar_line(request: &str) -> String {
    let body = request.trim_end_matches('\n');
    format!("{body} crc={:08x}\n", store::crc::crc32c(body.as_bytes()))
}

/// What a reader finds of a run's request sidecar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SidecarState {
    /// The claim names none (`request=-`).
    None,
    /// It verifies: its CRC holds and its digest is the claim's. The request
    /// line.
    Verified(String),
    /// Missing, torn, its CRC fails, or another digest: never the basis of an
    /// automatic resume (`request_sidecar_stale`).
    Stale(String),
}

/// Reads the sidecar at `path` against the claim's `request=` digest.
#[must_use]
pub fn read_sidecar(path: &Path, digest: &str) -> SidecarState {
    if digest == "-" {
        return SidecarState::None;
    }
    let stale = |why: String| SidecarState::Stale(why);
    let bytes = match store::safe_open::open_read_len(path) {
        Ok((file, len)) if usize::try_from(len).is_ok_and(|len| len <= SIDECAR_MAX) => {
            let mut buf = vec![0_u8; usize::try_from(len).unwrap_or(0)];
            if file.read_exact_at(&mut buf, 0).is_err() {
                return stale("unreadable".to_owned());
            }
            buf
        }
        Ok(_) => return stale(format!("past {SIDECAR_MAX} bytes")),
        Err(why) => return stale(format!("absent or refused: {why}")),
    };
    let Ok(text) = std::str::from_utf8(&bytes) else {
        return stale("not text".to_owned());
    };
    let Some(line) = text.strip_suffix('\n') else {
        return stale("torn: no newline".to_owned());
    };
    let Some((body, crc)) = line.rsplit_once(" crc=") else {
        return stale("no crc".to_owned());
    };
    if crc != format!("{:08x}", store::crc::crc32c(body.as_bytes())) {
        return stale("its crc fails".to_owned());
    }
    if digest16(text.as_bytes()) != digest {
        return stale("another request's sidecar (its digest is not the claim's)".to_owned());
    }
    SidecarState::Verified(body.to_owned())
}

// ═══ The probe (design §5.6) ════════════════════════════════════════════════

/// What a probe reads of a claim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClaimState {
    /// The file is empty.
    Empty,
    /// A claim.
    Claim(Box<Claim>),
    /// Bytes that are not a claim: never auto-resumed (D-4907).
    Unreadable(String),
}

impl ClaimState {
    fn of(text: &str) -> Self {
        if text.is_empty() {
            Self::Empty
        } else {
            Claim::parse(text).map_or_else(
                |why| Self::Unreadable(format!("{why}: {}", printable(text.as_bytes()))),
                |claim| Self::Claim(Box::new(claim)),
            )
        }
    }
}

/// One lock file as a probe found it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LockState {
    /// The lock was taken and the file is empty.
    Free {
        /// The lock's name.
        lock: String,
    },
    /// The lock is held by a live process; an empty claim is "held (claim
    /// being written)".
    Held {
        /// The lock's name.
        lock: String,
        /// Its claim.
        claim: ClaimState,
    },
    /// The lock was free and the file holds a claim: its holder died.
    Interrupted {
        /// The lock's name.
        lock: String,
        /// Its claim.
        claim: ClaimState,
        /// Whether a stop file names its run.
        stop_requested: bool,
    },
    /// The lock file could not be probed.
    Unprobed {
        /// The lock's name.
        lock: String,
        /// Why.
        why: String,
    },
}

impl LockState {
    /// The lock's name.
    #[must_use]
    pub fn lock(&self) -> &str {
        match self {
            Self::Free { lock }
            | Self::Held { lock, .. }
            | Self::Interrupted { lock, .. }
            | Self::Unprobed { lock, .. } => lock,
        }
    }

    /// The claim, when one was read.
    #[must_use]
    pub fn claim(&self) -> Option<&Claim> {
        match self {
            Self::Held {
                claim: ClaimState::Claim(claim),
                ..
            }
            | Self::Interrupted {
                claim: ClaimState::Claim(claim),
                ..
            } => Some(claim),
            _ => None,
        }
    }
}

/// The GDFL lock files a probe looks at: the store lock and every kind lock
/// whose file exists. At most four.
#[must_use]
pub fn lock_names(store: &Path) -> Vec<String> {
    let mut names = vec![STORE_LOCK.to_owned()];
    for kind in ["indices", "stocks", "options"] {
        let name = kind_lock(kind);
        if store.join(&name).exists() {
            names.push(name);
        }
    }
    names
}

/// Probes the GDFL lock files of `store` (design §5.6): each tried SHARED
/// and non-blocking `1 + retry` times, at most [`PROBE_READ`] bytes read,
/// released at once. A probe never creates a lock file and never makes a
/// press busy (D-4849). O(1) per file, at most four files; measured by
/// `pull::gdfl_run::ia2_tests::ia2_timings`.
#[must_use]
pub fn lock_state(store: &Path, retry: u32) -> Vec<LockState> {
    lock_names(store)
        .into_iter()
        .filter(|name| store.join(name).exists())
        .map(|name| probe_one(store, &name, retry))
        .collect()
}

/// One lock file, probed.
#[must_use]
pub fn probe_one(store: &Path, name: &str, retry: u32) -> LockState {
    let path = store.join(name);
    let file = match OpenOptions::new()
        .read(true)
        .custom_flags(O_NOFOLLOW_NONBLOCK)
        .open(&path)
    {
        Ok(file) if file.metadata().is_ok_and(|m| m.is_file()) => file,
        Ok(_) => {
            return LockState::Unprobed {
                lock: name.to_owned(),
                why: format!("is {}", what_is_at(&path)),
            };
        }
        Err(why) => {
            return LockState::Unprobed {
                lock: name.to_owned(),
                why: why.to_string(),
            };
        }
    };
    let text = read_head(&file);
    match take(file, store, name, Hold::Shared, retry) {
        Ok(flock) => {
            // Read again under the hold: the claim cannot change while a
            // shared hold is in place.
            let text = read_head(&flock);
            let _released = flock.release();
            if text.is_empty() {
                LockState::Free {
                    lock: name.to_owned(),
                }
            } else {
                let claim = ClaimState::of(&text);
                let stop_requested = match &claim {
                    ClaimState::Claim(claim) => {
                        stop_names_run(&store.join(stop_name(name)), claim.run)
                    }
                    _ => false,
                };
                LockState::Interrupted {
                    lock: name.to_owned(),
                    claim,
                    stop_requested,
                }
            }
        }
        Err(LockFault::Busy { .. }) => LockState::Held {
            lock: name.to_owned(),
            claim: ClaimState::of(&text),
        },
        Err(why) => LockState::Unprobed {
            lock: name.to_owned(),
            why: why.to_string(),
        },
    }
}

/// At most [`PROBE_READ`] bytes of `file` as text (lossy), trailing
/// whitespace dropped.
fn read_head(file: &File) -> String {
    let mut buf = vec![0_u8; PROBE_READ];
    let n = file.read_at(&mut buf, 0).unwrap_or(0);
    buf.truncate(n);
    String::from_utf8_lossy(buf.trim_ascii_end()).into_owned()
}

/// Whether the stop file at `path` names run `run`.
#[must_use]
pub fn stop_names_run(path: &Path, run: u64) -> bool {
    super::stop::read_stop(path).is_some_and(|stop| stop.run == run)
}

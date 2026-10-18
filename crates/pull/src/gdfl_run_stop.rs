//! `pull::gdfl_run::stop`: the whole stop of a GDFL run (IA-2; round 4,
//! SF-11; D-4827, D-4853, D-4893; design §5.5).
//!
//! A stop request is a **stop file**, one per run process, beside the lock
//! that holds the run's claim: `imports/gdfl.<kind>.stop` on a v4 store,
//! `imports/gdfl.stop` on v3. It is one line,
//! `stop run=<id> requested=<unix_s> by=<api|cli|operator|shutdown>`, written
//! to a staging name, synced, renamed and its directory synced, BEFORE the
//! asker is answered ([`request_stop`]), so a stop that was acknowledged
//! survives a crash of either side.
//!
//! The run asks [`StopControl::requested`] before it takes each file, never
//! inside one: one atomic load, and while no stop is seen one `metadata` of
//! the stop file (`docs/06-limits.md` L-IA-16). A stop file that names
//! another run is not this run's stop (it is a dead run's, and the next taker
//! of the lock removes it, `locks::RunLocks::take`). Workers finish the file
//! in hand and take no new one; the day closes by D-4853's rules
//! (`incomplete stage=run reason=stopped` when its `begin` was written, no
//! line when not, `done` when the stop came after its last file), and the
//! run releases.
//!
//! Who asked and when is kept: the stop file's `by=` and `requested=` are
//! read before the file is removed at release, and the run's "run stopped"
//! event carries them as `stop_by` and `requested` (D-4893). The asker emits
//! "stop requested" itself, so the cli's stop, the api's route and the
//! shutdown each leave an audited line.

use std::fs::{self, File};
use std::os::unix::fs::FileExt as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, PoisonError};

use super::locks::{self, ClaimState, LockState, PROBE_READ, TARGET};

/// The words `by=` may hold.
pub const BY: [&str; 4] = ["api", "cli", "operator", "shutdown"];

/// One stop request, as its file holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StopRequest {
    /// The run it stops.
    pub run: u64,
    /// When it was asked, UNIX seconds.
    pub requested: u64,
    /// Who asked: a word of [`BY`].
    pub by: String,
}

impl StopRequest {
    /// The stop file's one line, with its newline.
    #[must_use]
    pub fn line(&self) -> String {
        format!(
            "stop run={} requested={} by={}\n",
            self.run, self.requested, self.by
        )
    }

    /// Reads a stop line.
    ///
    /// # Errors
    ///
    /// Why it is not one.
    pub fn parse(text: &str) -> Result<Self, String> {
        let text = text.strip_suffix('\n').unwrap_or(text);
        let words: Vec<&str> = text.split(' ').collect();
        let ["stop", run, requested, by] = words.as_slice() else {
            return Err(format!(
                "not a stop line: {}",
                locks::printable(text.as_bytes())
            ));
        };
        let number = |word: &str, name: &str| {
            word.strip_prefix(name)
                .and_then(|rest| rest.strip_prefix('='))
                .filter(|v| !v.is_empty() && v.len() <= 20 && v.bytes().all(|b| b.is_ascii_digit()))
                .and_then(|v| v.parse::<u64>().ok())
                .ok_or_else(|| format!("{name}= is not a number in {word:?}"))
        };
        let by = by
            .strip_prefix("by=")
            .filter(|v| BY.contains(v))
            .ok_or_else(|| format!("by= is not one of {BY:?} in {by:?}"))?;
        Ok(Self {
            run: number(run, "run")?,
            requested: number(requested, "requested")?,
            by: by.to_owned(),
        })
    }
}

/// The stop file at `path`, when it is a regular file holding a stop line;
/// at most [`PROBE_READ`] bytes read, never following a link.
#[must_use]
pub fn read_stop(path: &Path) -> Option<StopRequest> {
    let (file, len) = store::safe_open::open_read_len(path).ok()?;
    if len > PROBE_READ as u64 {
        return None;
    }
    let mut buf = vec![0_u8; usize::try_from(len).ok()?];
    file.read_exact_at(&mut buf, 0).ok()?;
    StopRequest::parse(std::str::from_utf8(&buf).ok()?).ok()
}

/// Writes `request` as the stop file at `path`: a staging name, synced,
/// renamed, the directory synced. A second stop of the same run keeps the
/// first asker (RA-336): the file already naming this run is left as it is.
///
/// # Errors
///
/// The staging helper's refusal or the host's, by name.
pub fn write_stop(path: &Path, request: &StopRequest) -> Result<StopRequest, String> {
    if let Some(first) = read_stop(path).filter(|first| first.run == request.run) {
        return Ok(first);
    }
    let staging = path.with_extension("stop.tmp");
    let staged = store::staging::create(&staging).map_err(|why| why.to_string())?;
    staged
        .file
        .write_all_at(request.line().as_bytes(), 0)
        .and_then(|()| staged.file.sync_all())
        .map_err(|why| format!("{}: {why}", staging.display()))?;
    fs::rename(&staging, path).map_err(|why| format!("{}: {why}", path.display()))?;
    if let Some(dir) = path.parent() {
        File::open(dir)
            .and_then(|d| d.sync_all())
            .map_err(|why| format!("{}: {why}", dir.display()))?;
    }
    Ok(request.clone())
}

/// A run's stop, as the run asks it (design §5.5): the flag is set only by
/// the run's own process, from its stop file or [`StopControl::ask`].
#[derive(Debug)]
pub struct StopControl {
    path: PathBuf,
    run: u64,
    flag: AtomicBool,
    checks: AtomicU64,
    /// Test seam: the check number at which a stop is asked (0: never).
    #[cfg(test)]
    trip_at: u64,
    request: Mutex<Option<StopRequest>>,
}

impl StopControl {
    /// The stop of run `run`, whose stop file is `path`.
    #[must_use]
    pub const fn new(path: PathBuf, run: u64) -> Self {
        Self {
            path,
            run,
            flag: AtomicBool::new(false),
            checks: AtomicU64::new(0),
            #[cfg(test)]
            trip_at: 0,
            request: Mutex::new(None),
        }
    }

    /// A stop that asks itself at its `trip_at`-th check: a test's way to
    /// stop a run between two given files, where no other thread can be timed.
    #[cfg(test)]
    pub(crate) fn tripping_at(path: PathBuf, run: u64, trip_at: u64) -> Self {
        Self {
            trip_at,
            ..Self::new(path, run)
        }
    }

    #[cfg(test)]
    fn tripped(&self, checked: u64) -> bool {
        if self.trip_at != 0 && checked >= self.trip_at {
            self.ask("operator", 0);
            return true;
        }
        false
    }

    #[cfg(not(test))]
    #[allow(clippy::unused_self, reason = "the test build's seam has a body")]
    const fn tripped(&self, _checked: u64) -> bool {
        false
    }

    /// The run this stop belongs to.
    #[must_use]
    pub const fn run(&self) -> u64 {
        self.run
    }

    /// The stop file.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Whether a stop of this run was asked: one atomic load, and while none
    /// was seen one `metadata` of the stop file (L-IA-16). Called before each
    /// file is taken and before each day, never inside a file.
    #[must_use]
    pub fn requested(&self) -> bool {
        if self.flag.load(Ordering::Acquire) {
            return true;
        }
        let checked = self
            .checks
            .fetch_add(1, Ordering::Relaxed)
            .saturating_add(1);
        if self.tripped(checked) {
            return true;
        }
        if fs::symlink_metadata(&self.path).is_err() {
            return false;
        }
        match read_stop(&self.path) {
            Some(request) if request.run == self.run => {
                *self.request.lock().unwrap_or_else(PoisonError::into_inner) = Some(request);
                self.flag.store(true, Ordering::Release);
                true
            }
            _ => false,
        }
    }

    /// Asks this run to stop from inside its own process (a signal handler,
    /// or a test), as `by`.
    pub fn ask(&self, by: &str, requested: u64) {
        let mut kept = self.request.lock().unwrap_or_else(PoisonError::into_inner);
        if kept.is_none() {
            *kept = Some(StopRequest {
                run: self.run,
                requested,
                by: by.to_owned(),
            });
        }
        self.flag.store(true, Ordering::Release);
    }

    /// Who asked and when, once a stop was seen.
    #[must_use]
    pub fn request(&self) -> Option<StopRequest> {
        self.request
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// How many times the stop file was looked at.
    #[must_use]
    pub fn checks(&self) -> u64 {
        self.checks.load(Ordering::Relaxed)
    }

    /// Emits the run's "run stopped" event (D-4893): the run, the kind, who
    /// asked and when, read from the stop file before release removes it.
    pub fn note_stopped(&self, kind: &str, day: Option<&str>) {
        let request = self.request();
        let by = request.as_ref().map_or("-", |r| r.by.as_str());
        let requested = request.as_ref().map_or(0, |r| r.requested);
        let event = telemetry::Event::warn(TARGET, "run stopped")
            .with("run", self.run)
            .with("kind", kind)
            .with("stop_by", by)
            .with("requested", requested)
            .with("day", day.unwrap_or("-"));
        let _dropped_when_filtered = telemetry::emit(&event);
    }
}

/// Which run a stop names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopTarget<'a> {
    /// The live run of a kind.
    Kind(&'a str),
    /// The live run with this id.
    Run(u64),
}

/// What [`request_stop`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StopAnswer {
    /// The lock whose run was asked to stop.
    pub lock: String,
    /// That run.
    pub run: u64,
    /// Its kind.
    pub kind: String,
    /// Who holds it: its claim's `by=` (`cli` or `api`).
    pub held_by: String,
    /// The stop file as written (or as a first asker left it).
    pub request: StopRequest,
}

/// Why a stop was not written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StopRefusal {
    /// No live run matches (the route's 404).
    NotLive {
        /// What was asked for, in words.
        asked: String,
    },
    /// The matching lock is held and its claim is not written yet: ask again.
    ClaimBeingWritten {
        /// The lock.
        lock: String,
    },
    /// `by` is not a word of [`BY`].
    ByUnknown {
        /// The word.
        by: String,
    },
    /// The stop file could not be written.
    Unwritten {
        /// Why.
        why: String,
    },
}

impl StopRefusal {
    /// The code.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::NotLive { .. } => "run_not_live",
            Self::ClaimBeingWritten { .. } => "claim_being_written",
            Self::ByUnknown { .. } => "stop_by_unknown",
            Self::Unwritten { .. } => "stop_unwritten",
        }
    }
}

impl core::fmt::Display for StopRefusal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}: ", self.code())?;
        match self {
            Self::NotLive { asked } => write!(f, "no live GDFL run is {asked}"),
            Self::ClaimBeingWritten { lock } => write!(
                f,
                "{lock} is held and its claim is being written; ask again in a moment"
            ),
            Self::ByUnknown { by } => write!(f, "by={by} is not one of {BY:?}"),
            Self::Unwritten { why } => f.write_str(why),
        }
    }
}

/// Asks the live run `target` of `store` to stop, as `by`, at `now`: probes
/// the lock files ([`locks::lock_state`], with the probe retry), writes the
/// stop file of the matching held lock before answering, and emits the
/// audited "stop requested" event. A run held by another process (a cli run
/// asked from the api, or the reverse) is stopped the same way.
///
/// # Errors
///
/// [`StopRefusal`] by name; nothing is written on any of them.
pub fn request_stop(
    store: &Path,
    target: StopTarget<'_>,
    by: &str,
    now: u64,
) -> Result<StopAnswer, StopRefusal> {
    if !BY.contains(&by) {
        return Err(StopRefusal::ByUnknown { by: by.to_owned() });
    }
    let asked = match target {
        StopTarget::Kind(kind) => format!("of kind {kind}"),
        StopTarget::Run(run) => format!("run {run}"),
    };
    for state in locks::lock_state(store, locks::PROBE_RETRY) {
        let LockState::Held { lock, claim } = state else {
            continue;
        };
        let claim = match claim {
            ClaimState::Claim(claim) => claim,
            ClaimState::Empty => {
                if matches!(target, StopTarget::Kind(kind) if lock == locks::kind_lock(kind)) {
                    return Err(StopRefusal::ClaimBeingWritten { lock });
                }
                continue;
            }
            ClaimState::Unreadable(_) => continue,
        };
        let matches = match target {
            StopTarget::Kind(kind) => claim.kind == kind,
            StopTarget::Run(run) => claim.run == run,
        };
        if !matches {
            continue;
        }
        let request = StopRequest {
            run: claim.run,
            requested: now,
            by: by.to_owned(),
        };
        let path = store.join(locks::stop_name(&lock));
        let written = write_stop(&path, &request).map_err(|why| StopRefusal::Unwritten { why })?;
        let event = telemetry::Event::info(TARGET, "stop requested")
            .with("run", written.run)
            .with("kind", claim.kind.as_str())
            .with("by", written.by.as_str())
            .with("requested", written.requested)
            .with("lock", lock.as_str());
        // Under the stopped run's key, so `/logs.json?run=<id>` finds who
        // asked beside the run's own lines; a key this process holds for
        // another run is left alone and the field still names the run.
        let keyed = telemetry::global().filter(|sink| sink.claim_run(written.run));
        let _dropped_when_filtered = telemetry::emit(&event);
        if let Some(sink) = keyed {
            sink.release_run(written.run);
        }
        return Ok(StopAnswer {
            lock,
            run: claim.run,
            kind: claim.kind.clone(),
            held_by: claim.by.clone(),
            request: written,
        });
    }
    Err(StopRefusal::NotLive { asked })
}

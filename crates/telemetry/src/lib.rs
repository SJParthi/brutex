//! The structured event stream: one line per thing that happened, on disk,
//! bounded, and readable from the end.
//!
//! # What this is for
//!
//! Twenty-three `println!` and `eprintln!` calls across this workspace write
//! to a terminal nobody is watching. When a twelve-hour backfill goes wrong at
//! hour eleven, every one of them has already scrolled away or was never
//! attached to a terminal at all. This crate is where those events go instead:
//! a file that survives the process, carries typed fields a consumer can
//! filter on without parsing prose, and can be read from the end without being
//! read at all.
//!
//! # How it relates to the run journal, which already exists
//!
//! `crates/api/src/audit.rs` writes `~/.brutex/store/audit/pull.journal`:
//! fixed 256-byte records, CRC-32C per record, `fsync` before the answer page
//! renders, never rotated. **This crate does not replace it and must not.**
//! They answer different questions and are built to different rules:
//!
//! | | the journal | this |
//! |---|---|---|
//! | grain | one record per *run* | one line per *event* |
//! | volume | ~a few hundred, ever | tens of thousands per backfill |
//! | shape | fixed 256-byte stride | variable-width NDJSON |
//! | integrity | CRC-32C per record | none; a damaged line is counted and skipped |
//! | durability | `fsync` per record | page cache; [`Sink::sync`] on request |
//! | lifetime | append-only, never rotated | bounded at 64 MiB and rolled |
//! | the question it answers | *what did that run do* | *what was happening at 14:03* |
//!
//! They compose. The journal is the record an operator acts on and is the
//! thing `CLAUDE.md` §3 rule 8 protects; this is the fine-grained stream
//! underneath it, which has to be bounded precisely *because* it is
//! fine-grained. A run that ends `FAILED` in the journal is the pointer; the
//! events at that millisecond in this file are the explanation. Nothing here
//! duplicates a journal field, and nothing here is `fsync`-ed per event,
//! because doing either would make this a slower second journal rather than a
//! stream.
//!
//! # The format
//!
//! Newline-delimited JSON. One event is one line, and the line is valid JSON
//! on its own, so `grep` works, `tail -f` works, and a machine can parse a
//! line without seeing the file. The key order is fixed — `seq`, `ts`, `ms`,
//! `level`, `target`, `msg`, `fields` — so the columns line up for a person;
//! see `encode` for why, and for why the JSON is hand-written rather than
//! taken from `serde_json`.
//!
//! ```text
//! {"seq":41,"ts":"2026-08-08T14:03:11.427Z","ms":1786197791427,"level":"info",
//!  "target":"pull.http","msg":"vendor responded","fields":{"status":200,"bytes":81922}}
//! ```
//!
//! # The bounds, in numbers
//!
//! * A file rolls at [`DEFAULT_MAX_FILE_BYTES`] — 8 MiB, roughly 33,000 events.
//! * [`DEFAULT_KEEP_FILES`] files are kept — 8, including the one being
//!   written. **The whole crate therefore occupies at most 64 MiB, forever.**
//! * One event carries at most [`MAX_FIELDS`] fields; text past its ceiling is
//!   cut on a character boundary and the line says `"cut":true`.
//! * A tail query returns at most [`MAX_LIMIT`] events and reads at most
//!   [`Query::max_scan_bytes`], and says when it stopped early.
//!
//! # What one event costs, and what a hard kill loses
//!
//! One atomic load, one clock read, one mutex, one render into a reused buffer
//! (**zero allocations in steady state**), one integer comparison and one
//! `write` syscall. Nothing scales with how many events came before or how
//! large the file is. Once per 8 MiB there is a rotation inside the same lock.
//! `sink` states all of it, including what is not free.
//!
//! Nothing is buffered in user space, so a `SIGKILL` loses nothing that
//! [`Sink::emit`] said it had written. A power cut loses whatever the kernel
//! had not flushed, because this does not `fsync` per event — deliberately,
//! and stated in `sink`.
//!
//! # When it cannot write
//!
//! It never takes the caller down, and it never hides the failure: `emit`
//! returns [`Emitted::Dropped`], [`Sink::health`] carries a running count and
//! the reason in its own words, and the *first* failure — only the first —
//! goes to `stderr`. The trade-off is stated in `sink`: those events are
//! genuinely lost and are not queued, because an unbounded in-memory backlog
//! of a log that cannot be written is how a logger kills its host.
//!
//! # Using it
//!
//! ```
//! use telemetry::{Config, Event, Level, Query, Sink};
//!
//! let dir = std::env::temp_dir().join(format!("telemetry-doc-{}", std::process::id()));
//! let sink = Sink::open(&Config::new(&dir).with_min_level(Level::Debug))?;
//!
//! sink.emit(
//!     &Event::info("pull.http", "vendor responded")
//!         .with("status", 200u32)
//!         .with("instrument", "NSE-NIFTY")
//!         .with("bytes", 81_922usize),
//! );
//!
//! let found = telemetry::tail(&dir, sink.keep_files(), &Query::last(20).at_least(Level::Info));
//! assert_eq!(found.records.len(), 1);
//! assert_eq!(found.records[0].target, "pull.http");
//! assert_eq!(
//!     found.records[0].field("status").and_then(|v| v.as_u64()),
//!     Some(200)
//! );
//! std::fs::remove_dir_all(&dir).ok();
//! # Ok::<(), String>(())
//! ```

#![forbid(unsafe_code)]

mod clock;
mod encode;
mod event;
mod json;
mod level;
mod record;
mod sink;
mod tail;
mod value;

pub use crate::clock::{civil_from_days, now_millis};
pub use crate::event::{
    Event, MAX_FIELDS, MAX_KEY_BYTES, MAX_MESSAGE_BYTES, MAX_STR_VALUE_BYTES, MAX_TARGET_BYTES,
};
pub use crate::json::LineFault;
pub use crate::level::{LEVELS, Level};
pub use crate::record::Record;
pub use crate::sink::{
    BASENAME, Config, DEFAULT_KEEP_FILES, DEFAULT_MAX_FILE_BYTES, EXTENSION, Emitted, FileTarget,
    Health, MAX_TARGET_LEVELS, MIN_FILE_BYTES, Sink, Target, current_path, dir_beneath_store,
    paths_newest_first, rotated_path,
};
pub use crate::tail::{
    DEFAULT_MAX_SCAN_BYTES, MAX_LIMIT, MAX_LINE_BYTES, Query, READ_BLOCK, Tail, tail,
};
pub use crate::value::{OwnedValue, Value};

use std::sync::OnceLock;

/// The process-wide sink, once something installs one.
static GLOBAL: OnceLock<Sink> = OnceLock::new();

/// Installs the process-wide sink.
///
/// Optional. A caller that would rather hold its own [`Sink`] — a test, a
/// library, anything that wants two of them — never touches this. It exists so
/// that code deep inside a request handler can emit an event without every
/// function between it and `main` growing a parameter.
///
/// # Errors
///
/// The open failure in its own words, or a refusal naming the already-installed
/// path. **A second install is refused rather than ignored**: two sinks on one
/// path would each keep their own byte count, so each would roll the other's
/// file out from under it and neither count would be right.
pub fn install(config: &Config) -> Result<&'static Sink, String> {
    // The configuration is judged FIRST, before the slot is even looked at, so
    // that a configuration this crate would never accept is refused for that
    // reason whether or not something is already installed. Judging the slot
    // first would make the answer depend on what else the process had done.
    if let Some(why) = config.refusal() {
        return Err(why);
    }
    if let Some(existing) = GLOBAL.get() {
        return Err(format!(
            "a telemetry sink is already installed, writing {}",
            existing.path().display()
        ));
    }
    let sink = Sink::open(config)?;
    let wanted = sink.path();
    let installed = GLOBAL.get_or_init(|| sink);
    if installed.path() == wanted {
        return Ok(installed);
    }
    // ANOTHER THREAD WON THE RACE between the `get` above and this
    // `get_or_init`, with a different path. The sink this call built has been
    // dropped and the caller is told which one is live rather than handed one
    // that is not writing where it asked for. NO TEST DRIVES THIS ARM: it
    // needs two `install` calls to interleave inside a window of a few
    // instructions, and `OnceLock` is per process so a test could not retry.
    // Named here rather than left for a coverage report to find, per
    // `CLAUDE.md` §3 rule 6.
    Err(format!(
        "a telemetry sink was installed concurrently, writing {}; {} is not it",
        installed.path().display(),
        wanted.display()
    ))
}

/// The process-wide sink, if one is installed.
#[must_use]
pub fn global() -> Option<&'static Sink> {
    GLOBAL.get()
}

/// Whether the process-wide sink would write an event at `level` from `target`.
///
/// [`false`] when nothing is installed, because an event with nowhere to go is
/// not written. See [`Sink::admits`] for why this exists separately from
/// [`emit`].
#[must_use]
pub fn admits(level: Level, target: &str) -> bool {
    global().is_some_and(|sink| sink.admits(level, target))
}

/// Emits an event, evaluating its arguments **only if it would be written**.
///
/// # Why a macro when [`emit`] is a function
///
/// Because that is the whole difference. `emit` is a function call, so Rust
/// evaluates every argument and builds the entire `Event` before the level can
/// be looked at. Measured against `tracing`'s macro in one binary under one
/// harness:
///
/// | filtered event | this crate, via `emit` | `tracing` macro |
/// |---|---:|---:|
/// | no fields | 6,750 ps | 270 ps |
/// | three fields | 14,146 ps | 250 ps |
///
/// The bespoke cost **doubles** with the field count and `tracing`'s does not
/// move. That is O(call-site fields) against `CLAUDE.md` §3 rule 4's O(1), and
/// it is the only measured O(1) violation on the write path.
///
/// This closes it with no new dependency: [`admits`] runs the same two checks
/// `emit` runs, and the `Event` is only constructed inside the `if`.
///
/// # Cost
///
/// A filtered event costs one relaxed atomic load, one comparison, and — only
/// when per-target overrides exist — at most [`MAX_TARGET_LEVELS`] bounded
/// prefix comparisons. **Nothing else is evaluated.** A written event costs
/// exactly what [`emit`] costs, because it is [`emit`].
///
/// Proved by `telemetry::sink::a_filtered_event_never_evaluates_its_arguments`,
/// which counts side effects rather than timing them, and measured flat by
/// `telemetry::bench::a_filtered_event_touches_nothing_and_stays_flat` (C-T-02).
///
/// # Examples
///
/// ```
/// use telemetry::{Level, Value, emit_if};
/// // The `expensive()` call does not happen unless the event is written.
/// fn expensive() -> u64 { 42 }
/// let _outcome = emit_if!(Level::Debug, "pull.member", "landed",
///     "bars" => Value::Uint(expensive()));
/// ```
#[macro_export]
macro_rules! emit_if {
    ($level:expr, $target:expr, $message:expr $(, $key:expr => $value:expr)* $(,)?) => {{
        let level = $level;
        let target = $target;
        if $crate::admits(level, target) {
            $crate::emit(
                &$crate::Event::new(level, target, $message)
                    $(.with($key, $value))*
            )
        } else {
            $crate::Emitted::Filtered
        }
    }};
}

/// Writes one event to the process-wide sink.
///
/// [`Emitted::NotInstalled`] when there is none. **Not a panic and not a
/// silent success**: a caller that wants to know whether its events are being
/// recorded can see that they are not, and a caller that does not care is not
/// killed for it.
#[must_use]
pub fn emit(event: &Event<'_>) -> Emitted {
    global().map_or(Emitted::NotInstalled, |sink| sink.emit(event))
}

/// Writes one event to the process-wide sink under an explicit run id.
///
/// This is the concurrent-safe counterpart to [`Sink::set_run`]: it stamps
/// this event only and leaves the sink's ambient run untouched.
#[must_use]
pub fn emit_for_run(run: u64, event: &Event<'_>) -> Emitted {
    global().map_or(Emitted::NotInstalled, |sink| sink.emit_for_run(run, event))
}

/// Reserves a non-zero correlation id from the process-wide log sequence.
///
/// [`None`] means either that logging is not installed or that the id space is
/// exhausted. A caller that needs an auditable exact attempt boundary must
/// refuse in either case rather than fall back to a timestamp guess.
#[must_use]
pub fn reserve_run_id() -> Option<u64> {
    global().and_then(Sink::reserve_run_id)
}

#[cfg(test)]
#[allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]
mod tests {
    use super::{Config, Emitted, Event, Level, Query, Sink, install, tail};

    /// Removes scratch directories left by test runs that have already finished.
    ///
    /// **Why this exists.** Every `scratch(..)` call names its directory after the
    /// running process id and empties it *before* use. That makes concurrent test
    /// binaries safe, and it made the suite's own doc comment — "this suite deletes
    /// what it creates" — false: nothing deleted anything at exit, and since each
    /// run has a fresh pid, nothing ever collided either. Measured on one machine
    /// after a day of work: **9,958 directories, 2.5 GB**.
    ///
    /// Swept by AGE rather than by asking whether a pid is still alive, because that
    /// question needs `libc` and every crate root here is `#![forbid(unsafe_code)]`.
    /// An hour is far longer than any run of this suite and far shorter than the gap
    /// between sessions, so a directory older than that belongs to a process that is
    /// gone. A binary running concurrently keeps writing its own directories, so
    /// their mtimes stay recent and it is never swept out from under itself.
    ///
    /// Runs once per process, before the first scratch directory is handed out.
    pub(crate) fn sweep_stale_scratch() {
        use std::sync::Once;
        static ONCE: Once = Once::new();
        ONCE.call_once(|| {
            sweep_stale_scratch_in(&std::env::temp_dir(), std::time::SystemTime::now());
        });
    }

    /// Sweeps test scratch with explicit directory and clock inputs, so fresh
    /// CI workers can prove the cleanup without inheriting old local files.
    fn sweep_stale_scratch_in(root: &std::path::Path, now: std::time::SystemTime) {
        let hour = std::time::Duration::from_hours(1);
        let Ok(entries) = std::fs::read_dir(root) else {
            return;
        };
        for entry in entries.flatten() {
            let name = entry.file_name();
            if !is_scratch_name(&name) {
                continue;
            }
            let stale = entry
                .metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| now.duration_since(t).ok())
                .is_some_and(|age| age > hour);
            if stale {
                let _ignored = std::fs::remove_dir_all(entry.path());
            }
        }
    }

    fn is_scratch_name(name: &std::ffi::OsStr) -> bool {
        name.to_str()
            .is_some_and(|name| name.starts_with("brutex-telemetry-"))
    }

    #[test]
    fn scratch_cleanup_uses_explicit_age_and_preserves_foreign_or_recent_entries() {
        use std::time::{Duration, SystemTime};

        let root = std::env::temp_dir().join(format!(
            "brutex-telemetry-cleanup-proof-{}",
            std::process::id()
        ));
        let _ignored = std::fs::remove_dir_all(&root);
        std::fs::create_dir(&root).expect("create private cleanup fixture");
        let now = SystemTime::UNIX_EPOCH + Duration::from_hours(2);
        let entries = [
            ("brutex-telemetry-old", 0, false),
            ("brutex-telemetry-boundary", 1, true),
            ("brutex-telemetry-fresh", 2, true),
            ("brutex-telemetry-future", 3, true),
            ("foreign-old", 0, true),
        ];
        for (name, hours, _) in entries {
            let path = root.join(name);
            std::fs::create_dir(&path).expect("create classified scratch entry");
            std::fs::write(path.join("sentinel"), b"private fixture")
                .expect("write cleanup sentinel");
            let modified = SystemTime::UNIX_EPOCH + Duration::from_hours(hours);
            std::fs::File::open(&path)
                .expect("open scratch directory")
                .set_times(std::fs::FileTimes::new().set_modified(modified))
                .expect("set deterministic scratch age");
        }
        sweep_stale_scratch_in(&root, now);
        for (name, _, retained) in entries {
            assert_eq!(root.join(name).exists(), retained);
            assert_eq!(root.join(name).join("sentinel").exists(), retained);
        }
        let absent = root.join("missing-root");
        sweep_stale_scratch_in(&absent, now);
        assert!(!absent.exists());
        std::fs::remove_dir_all(root).expect("remove private cleanup fixture");
    }

    #[cfg(unix)]
    #[test]
    fn scratch_cleanup_does_not_interpret_non_utf8_names_as_owned_entries() {
        use std::os::unix::ffi::OsStringExt;

        let mut name = b"brutex-telemetry-".to_vec();
        name.push(0xff);
        // APFS cannot create this name; classify the OS string directly so the
        // ownership rule is still proved on filesystems that refuse the bytes.
        assert!(!is_scratch_name(&std::ffi::OsString::from_vec(name)));
    }

    /// The whole surface, exercised the way a caller uses it, end to end.
    #[test]
    fn an_event_written_through_the_public_surface_reads_back_through_it() {
        let dir = std::env::temp_dir().join(format!("brutex-telemetry-lib-{}", std::process::id()));
        let _ignored = std::fs::remove_dir_all(&dir);
        let sink = Sink::open(&Config::new(&dir).with_min_level(Level::Trace)).expect("opens");

        let event = Event::warn("pull.ssm", "token re-read returned the same dead value")
            .with("attempt", 2u32)
            .with("halted", true);
        assert_eq!(sink.emit(&event), Emitted::Written);

        let found = tail(&dir, sink.keep_files(), &Query::last(10));
        assert_eq!(found.records.len(), 1);
        assert!(found.records[0].matches(&event));
        assert_eq!(found.records[0].level, Level::Warn);
        assert!(found.records[0].at_utc.ends_with('Z'));
        assert!(found.records[0].at_unix_millis > 1_767_225_600_000);
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    /// A SECOND INSTALL IS REFUSED, NOT IGNORED.
    ///
    /// This test also installs the global for this binary, which is why it is
    /// the only one here that touches it: `OnceLock` is per process, and the
    /// `NotInstalled` arm therefore has to be proved by a test binary that
    /// never installs — `tests/global_absent.rs`.
    #[test]
    fn installing_twice_is_refused_by_name_and_the_first_one_keeps_working() {
        let dir =
            std::env::temp_dir().join(format!("brutex-telemetry-global-{}", std::process::id()));
        let _ignored = std::fs::remove_dir_all(&dir);
        let sink = install(&Config::new(&dir)).expect("installs");
        assert_eq!(sink.path(), super::current_path(&dir));
        assert!(super::global().is_some());

        let elsewhere = dir.join("second");
        let said = install(&Config::new(&elsewhere)).expect_err("a second install is refused");
        assert!(said.contains("already installed"), "{said}");
        assert!(
            said.contains(&dir.display().to_string()),
            "and names the one that won: {said}"
        );
        assert!(!elsewhere.exists(), "the refused install created nothing");

        // A refused install cannot have changed which sink `emit` reaches.
        assert_eq!(
            super::emit(&Event::error("t", "through the global")),
            Emitted::Written
        );
        let found = tail(&dir, sink.keep_files(), &Query::last(10));
        assert_eq!(found.records.len(), 1);
        assert_eq!(found.records[0].message, "through the global");
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    /// An install that cannot open refuses and leaves the global unset, so a
    /// later one can succeed. Ordering-independent: it uses a path that can
    /// never open, so it never installs anything.
    #[test]
    fn an_install_that_cannot_open_is_refused_and_installs_nothing() {
        let said = install(&Config::new("/nowhere").with_max_file_bytes(1))
            .expect_err("a bound below the floor is refused before any I/O");
        assert!(said.contains("below the"), "{said}");
    }
}

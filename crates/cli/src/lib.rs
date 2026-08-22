//! The sweep, driven from a command line.
//!
//! # What this closes
//!
//! Until this crate existed, nothing that could be RUN reached the sweep. The
//! only binary in the workspace was `api`, and its dependency set names neither
//! [`runner`] nor anything beneath it. So the Apriori ladder, the 280-position
//! vocabulary, the exit grid, the walk-forward and the significance bar were
//! compiled, tested, benchmarked — and unreachable from any entry point. The
//! three render surfaces that display them had no caller at all.
//!
//! # Every bar this crate sweeps is GENERATED, and it says so in the output
//!
//! The operator's standing rule is that no vendor pull may originate here and
//! that the bars already on disk may not be used either. So the only honest
//! input is [`runner::synthetic`], and this crate takes no other. That is a
//! real limit, not a placeholder: **a report rendered from generated bars is
//! not a backtest**, and a reader who mistook one for the other would be making
//! exactly the error `CLAUDE.md` §4 bans a program from inviting.
//!
//! It is therefore stated in the rendered output itself, not only here — see
//! [`PROVENANCE`]. A banner in a doc comment protects nobody reading a
//! terminal.
//!
//! # What a caller must decide
//!
//! `min_hits` and the session count. Neither has a default in [`runner`],
//! because `CLAUDE.md` §3 rule 1 will not let that crate invent one, and this
//! crate does not invent one either — it requires them on the command line and
//! refuses without them. The vocabulary's own [`Widths`], [`Availability`] and
//! [`Thresholds`] are the pinned set, which is the only configuration this
//! build ships.
//!
//! # Cost
//!
//! This crate makes no cost claim of its own. It generates bars, hands them to
//! [`runner::Sweeper`] and renders the result; the bounds are `runner`'s and
//! `engine`'s, measured by their own benches.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

/// Reading real bars out of the store — the join `CLAUDE.md` §5 calls the live
/// gap. `crates/api` declared `store` and no `runner`; this crate declared
/// `runner` and no `store`, so nothing in the workspace connected a pulled bar
/// to a ranked result.
pub mod batch;
pub mod results;
pub mod stored;

use brutex_core::vendor::Vendor;
use costs::fill::Direction;
use engine::Ladder;
use indicators::evaluator::{Evaluator, Widths};
use indicators::pattern::Thresholds;
use indicators::vwap::Availability;
use runner::excursion::Side;
use runner::identity::{Direction as RunDirection, Params, Run, data_digest, identity};
use runner::outcome::Horizon;
use runner::{Sweeper, audit, closed, grid, synthetic, trade};
use std::fmt::Write as _;

/// Everything went as asked.
pub const OK: u8 = 0;
/// It was asked for something reasonable and could not do it.
pub const FAILED: u8 = 1;
/// It was asked for something it does not understand.
pub const MISUSED: u8 = 2;

/// The sentence every rendered report carries above it.
///
/// # Why this is a constant and not a comment
///
/// A report from this crate is byte-identical in shape to a report from real
/// bars, and the difference is the only thing that decides whether a number in
/// it means anything. `CLAUDE.md` §4 bans a fallback that hides a failure;
/// rendering a complete, confident-looking sweep over invented data without
/// saying so is that failure with better typography.
///
/// It is asserted by `the_report_always_declares_its_bars_are_generated`, so it
/// cannot be dropped by an edit that only touches the rendering.
pub const PROVENANCE: &str = "\
=== THESE BARS ARE GENERATED, NOT MARKET DATA ===
Produced by runner::synthetic in this process. Nothing was pulled from a vendor
and nothing was read from the store. Every figure below describes the generator,
NOT any instrument. This proves the pipeline runs end to end.
It is not a backtest, and no result in it is evidence about any market.
";

/// Usage, printed on every refusal so the reader never has to guess.
pub const USAGE: &str = "\
usage: cli sweep    SESSIONS MIN_HITS   walk the ladder at one threshold
       cli auto     SESSIONS            let the search choose the threshold
       cli audit    SESSIONS MIN_HITS   sweep, then trade the best combination
       cli sweep-stored VENDOR UNDERLYING RUNG YEAR MONTH MIN_HITS
                                   sweep REAL bars read from the store
       cli audit-stored VENDOR UNDERLYING RUNG YEAR MONTH MIN_HITS
                                   sweep REAL bars, then trade them: exit grid,
                                   walk-forward, PBO and bootstrap p-values
       cli audit-range  VENDOR UNDERLYING RUNG FROM_Y FROM_M TO_Y TO_M MIN_HITS
                                   sweep a CONTIGUOUS SPAN of months as ONE
                                   series -- the seven-year question, not twelve
                                   monthly ones. Missing months are named, never
                                   skipped quietly.
       cli sweep-all    VENDOR RUNG MIN_HITS
                                   sweep EVERY stored instrument-month at that
                                   feed and rung, one report for all of them

SESSIONS  how many generated trading days to sweep, 1..=3650
MIN_HITS  bars a combination must fire on to be kept, 1 or more
VENDOR    the feed that wrote them -- groww, dhan, truedata, gdfl, zerodha
UNDERLYING  the index, e.g. NIFTY or BANKNIFTY
RUNG      the bar length as its directory word -- 1min, 1day

The two stored commands read a run identity off the build, so they refuse
unless it was stamped:
    BRUTEX_COMMIT=$(git rev-parse HEAD) cargo build --release -p cli
";

/// The `audit-range` arm, lifted out of [`run`].
///
/// # Why it is a function and not five more lines in the match
///
/// `run` is a dispatch table and `clippy::too_many_lines` caps it at a hundred.
/// The cap is doing real work here: an arm that parses FOUR numbers plus a
/// threshold is the largest in the table, and inlining it pushed the whole
/// dispatch past the point where a reader can see the command list at all.
///
/// The refusals are the same four `audit-stored` gives and in the same order,
/// because two commands taking one shape of argument must reject a bad one
/// identically or an operator learns two rules.
fn audit_range_arm(
    out: &mut String,
    vendor: &str,
    underlying: &str,
    rung: &str,
    from: (&str, &str),
    to: (&str, &str),
    min_hits: &str,
) -> u8 {
    match (
        from.0.parse::<u16>(),
        from.1.parse::<u8>(),
        to.0.parse::<u16>(),
        to.1.parse::<u8>(),
        parse_min_hits(min_hits),
    ) {
        (Ok(fy), Ok(fm), Ok(ty), Ok(tm), Ok(h)) => {
            let text = audit_range(vendor, underlying, rung, (fy, fm), (ty, tm), h);
            let refused = text.starts_with("refused: ");
            out.push_str(&text);
            if refused { MISUSED } else { OK }
        }
        (Err(_), _, _, _, _) | (_, _, Err(_), _, _) => {
            refuse(out, "YEAR must be a number like 2026")
        }
        (_, Err(_), _, _, _) | (_, _, _, Err(_), _) => refuse(out, "MONTH must be 1..=12"),
        (_, _, _, _, Err(why)) => refuse(out, why),
    }
}

/// Parses one command and runs it, returning the code the shell reads.
///
/// # Why this takes the arguments rather than reading them
///
/// The same reason `api::server::run` does: a function that reads
/// `std::env::args` has arms no unit test can enter, and `CLAUDE.md` §9's
/// coverage floor is not something to work around with a comment.
#[must_use]
pub fn run(args: &[String], out: &mut String) -> u8 {
    let words: Vec<&str> = args.iter().map(String::as_str).collect();
    match words.as_slice() {
        ["sweep", sessions, min_hits] => match (parse_sessions(sessions), parse_min_hits(min_hits))
        {
            (Ok(s), Ok(m)) => {
                out.push_str(&sweep(s, m));
                OK
            }
            (Err(why), _) | (_, Err(why)) => refuse(out, why),
        },
        ["audit", sessions, min_hits] => {
            match (parse_sessions(sessions), parse_min_hits(min_hits)) {
                (Ok(s), Ok(m)) => {
                    out.push_str(&audit_run(s, m));
                    OK
                }
                (Err(why), _) | (_, Err(why)) => refuse(out, why),
            }
        }
        [
            "sweep-stored",
            vendor,
            underlying,
            rung,
            year,
            month,
            min_hits,
        ] => {
            match (
                year.parse::<u16>(),
                month.parse::<u8>(),
                parse_min_hits(min_hits),
            ) {
                (Ok(y), Ok(m), Ok(h)) => {
                    let text = sweep_stored(vendor, underlying, rung, y, m, h);
                    let refused = text.starts_with("refused: ");
                    out.push_str(&text);
                    if refused { MISUSED } else { OK }
                }
                (Err(_), _, _) => refuse(out, "YEAR must be a number like 2026"),
                (_, Err(_), _) => refuse(out, "MONTH must be 1..=12"),
                (_, _, Err(why)) => refuse(out, why),
            }
        }
        [
            "audit-stored",
            vendor,
            underlying,
            rung,
            year,
            month,
            min_hits,
        ] => {
            // The same parse, the same order and the same three refusals as
            // `sweep-stored` above. Two commands taking one shape of argument
            // must reject a bad one identically, or an operator learns two rules.
            match (
                year.parse::<u16>(),
                month.parse::<u8>(),
                parse_min_hits(min_hits),
            ) {
                (Ok(y), Ok(m), Ok(h)) => {
                    let text = audit_stored(vendor, underlying, rung, y, m, h);
                    let refused = text.starts_with("refused: ");
                    out.push_str(&text);
                    if refused { MISUSED } else { OK }
                }
                (Err(_), _, _) => refuse(out, "YEAR must be a number like 2026"),
                (_, Err(_), _) => refuse(out, "MONTH must be 1..=12"),
                (_, _, Err(why)) => refuse(out, why),
            }
        }
        ["audit-range", v, u, r, fy, fm, ty, tm, mh] => {
            audit_range_arm(out, v, u, r, (fy, fm), (ty, tm), mh)
        }
        ["sweep-all", vendor, rung, min_hits] => match parse_min_hits(min_hits) {
            Ok(h) => {
                let text = batch::sweep_all(vendor, rung, h);
                let refused = text.starts_with("refused: ");
                out.push_str(&text);
                if refused { MISUSED } else { OK }
            }
            Err(why) => refuse(out, why),
        },
        ["auto", sessions] => match parse_sessions(sessions) {
            Ok(s) => {
                out.push_str(&auto(s));
                OK
            }
            Err(why) => refuse(out, why),
        },
        [] => refuse(out, "no command given"),
        [word, ..] => {
            let owned = format!("`{word}` is not a command this build knows");
            refuse(out, &owned)
        }
    }
}

/// Writes a named refusal and the usage, and returns [`MISUSED`].
///
/// Named rather than bare: `CLAUDE.md` §4 requires a refusal to say what was
/// wrong, and "usage:" alone leaves the reader to work out which of their words
/// this build objected to.
fn refuse(out: &mut String, why: &str) -> u8 {
    out.push_str("refused: ");
    out.push_str(why);
    out.push('\n');
    out.push_str(USAGE);
    MISUSED
}

/// Sessions must be at least one and at most ten years of them.
///
/// The upper bound is not a performance guess: `synthetic::sessions` allocates
/// one `Candle` per minute per day, so an unbounded count is an unbounded
/// allocation reached from the command line, which is the shape
/// `docs/06-limits.md` §5 is about.
fn parse_sessions(text: &str) -> Result<i64, &'static str> {
    match text.parse::<i64>() {
        Ok(n) if (1..=3650).contains(&n) => Ok(n),
        Ok(_) => Err("SESSIONS is outside 1..=3650"),
        Err(_) => Err("SESSIONS is not a whole number"),
    }
}

/// `min_hits` must be at least one.
///
/// Zero is refused HERE as well as clamped in [`Ladder::with_min_hits`], and
/// the duplication is deliberate: the clamp keeps the ladder correct, and this
/// keeps the operator from believing a zero they typed was honoured.
fn parse_min_hits(text: &str) -> Result<u64, &'static str> {
    match text.parse::<u64>() {
        Ok(n) if n >= 1 => Ok(n),
        Ok(_) => Err("MIN_HITS must be 1 or more; 0 would disable extinction"),
        Err(_) => Err("MIN_HITS is not a whole number"),
    }
}

/// The pinned evaluator, which is the only configuration this build ships.
///
/// One line, so that the half a test CANNOT drive is as small as it can be.
fn evaluator() -> Result<Evaluator, &'static str> {
    evaluator_from(Widths::pinned().ok())
}

/// The evaluator, from widths the caller supplies.
///
/// # Why the widths are a parameter rather than a read
///
/// The same reason `api::autopilot::stays_paused_from` takes its value instead
/// of fetching it: `Widths::pinned()` reads two pinned constants and cannot fail
/// in this build, so a function that called it inline would carry a refusal arm
/// no test could enter — and `CLAUDE.md` §9's coverage floor is not something to
/// work around with a comment. `None` is the shape a future widening of the
/// tolerance table could produce, and this is where it is answered.
fn evaluator_from(widths: Option<Widths>) -> Result<Evaluator, &'static str> {
    let widths = widths.ok_or("the pinned tolerances are not valid")?;
    Ok(Evaluator::new(
        widths,
        // ABSENT, AND THE CALLER MAY NOT DERIVE IT. `vwap::availability_of`
        // reads the WHOLE slice, so a mask at bar 0 would depend on bar N --
        // look-ahead, which §3 rule 7 forbids outright. `runner` refuses to
        // compute it for exactly this reason and takes the verdict from its
        // caller; this crate declares it absent rather than inventing one.
        Availability::Absent,
        Thresholds::CLASSICAL,
    ))
}

/// One sweep at one threshold, rendered.
#[must_use]
pub fn sweep(sessions: i64, min_hits: u64) -> String {
    sweep_with(evaluator(), sessions, min_hits)
}

/// One sweep, from an evaluator the caller supplies.
///
/// Split for the same reason [`evaluator_from`] is: `evaluator()` cannot fail in
/// this build, so a refusal arm reached only through it is a region no test can
/// enter — and `CLAUDE.md` §9 asks for 100%, which an unreachable arm makes
/// unattainable rather than merely unmet. The arm still has to EXIST, because a
/// later widening of the tolerance table would reach it; taking the `Result`
/// here is what lets a test prove it says something useful when it does.
#[allow(
    clippy::needless_pass_by_value,
    reason = "`Evaluator` is 1.7 KB of indicator state and this function CONSUMES \
              it -- `Sweeper::run` takes `&mut`, and the value must outlive the \
              call, so a reference would only move the ownership problem to the \
              caller and put the unreachable refusal arm back in a public \
              function. One move of 1.7 KB happens once per process."
)]
// `Evaluator` measures 1,744 bytes (`docs/10-shared-core.md`), so this trips
// `large_types_passed_by_value` at its 256-byte limit. Taken by value anyway, and the
// allow is narrowed to the two functions that do it rather than relaxed at the crate
// root: the body needs OWNERSHIP — `Sweeper::run` takes `&mut ev` — so a reference
// would only move the move to the caller, and `Box` would buy an allocation and an
// indirection to avoid a memcpy that happens ONCE per CLI invocation, before any bar is
// read. This is not the sweep's hot path; `ConditionMask::hits` is, and nothing here is
// on it. If either function is ever called in a loop, this allow is the thing to revisit.
#[allow(
    clippy::large_types_passed_by_value,
    reason = "the body needs ownership; one 1,744-byte move per CLI invocation"
)]
fn sweep_with(ev: Result<Evaluator, &'static str>, sessions: i64, min_hits: u64) -> String {
    let mut ev = match ev {
        Ok(e) => e,
        Err(why) => return format!("refused: {why}\n"),
    };
    let bars = synthetic::sessions(sessions);
    let outcome = Sweeper::new(Ladder::with_min_hits(min_hits)).run(&bars, &mut ev);
    let mut out = String::from(PROVENANCE);
    out.push('\n');
    out.push_str(&runner::report::render(&outcome, None));
    out
}

/// What a run over REAL bars says about itself.
///
/// The counterpart to [`PROVENANCE`], and it exists for the same reason: a sweep
/// over stored bars and a sweep over generated ones are byte-identical in shape,
/// so the report has to say which it was or the reader cannot tell. This one
/// names the feed, the instrument and the month, because "real data" is not a
/// provenance — *whose* data, of *what*, for *when* is.
pub const STORED_PROVENANCE: &str = "\
=== THESE BARS ARE REAL MARKET DATA, READ FROM THE STORE ===
Nothing was pulled from a vendor by this process. The bars below were read from
a file some earlier pull wrote, and the run identity beneath names the exact
column they came from. A figure here describes that instrument and that month.
";

/// The commit this binary was BUILT from, if the build was stamped.
///
/// # Why `option_env!` and not a `build.rs`, and not `.git/HEAD`
///
/// `CLAUDE.md` §2 forbids a `build.rs` that invokes an external process, so
/// `git rev-parse` at build time is not available and is not wanted.
///
/// Reading `.git/HEAD` at RUN time would compile, and it would be wrong. §3
/// rule 3 identifies a run by the commit **the computation ran at**, and the
/// computation is this binary — which was compiled from one commit and may be
/// executed long after the tree moved to another. A runtime read would stamp a
/// result with a commit whose source never produced it, which is worse than
/// recording nothing: it is a reproducibility claim that cannot be honoured.
///
/// `option_env!` resolves at compile time, in the binary, with no process
/// spawned. `None` means the build was not stamped, and that is refused rather
/// than filled in.
#[must_use]
pub const fn commit_stamp() -> Option<&'static str> {
    option_env!("BRUTEX_COMMIT")
}

/// The store root: `$BRUTEX_STORE`, else `$HOME/.brutex/store`.
///
/// The same two-step every other root in this workspace uses, so an operator who
/// has moved one has moved them all.
fn store_root() -> Result<std::path::PathBuf, stored::Refusal> {
    root_from(std::env::var_os("BRUTEX_STORE"), std::env::var_os("HOME"))
}

/// [`store_root`]'s decision, with the environment passed in.
///
/// Split so the three outcomes are testable without mutating process
/// environment, which `cargo test` runs threads against in parallel: a test that
/// set `HOME` would change it under every other test in the binary.
fn root_from(
    explicit: Option<std::ffi::OsString>,
    home: Option<std::ffi::OsString>,
) -> Result<std::path::PathBuf, stored::Refusal> {
    if let Some(explicit) = explicit {
        return Ok(std::path::PathBuf::from(explicit));
    }
    home.map_or_else(
        || Err("neither BRUTEX_STORE nor HOME is set, so the store cannot be found".to_owned()),
        |home| Ok(std::path::PathBuf::from(home).join(".brutex").join("store")),
    )
}

/// Where the sweep writes its events: `$BRUTEX_LOG_DIR`, else `<store>/logs`.
///
/// The same two-step every other root here uses, so an operator who has moved
/// one has moved them all. Beneath the STORE rather than the working directory
/// because a sweep started from `/` or from a read-only checkout must still log
/// somewhere it can write, and the store root is already required to be writable
/// — the same reasoning `api::served_log_dir` gives for its own fallback.
fn log_dir_from(
    explicit: Option<std::ffi::OsString>,
    store: Option<std::path::PathBuf>,
) -> Option<std::path::PathBuf> {
    explicit
        .map(std::path::PathBuf::from)
        .or_else(|| store.map(|s| s.join("logs")))
}

/// Installs the process-wide event sink, or says why it could not.
///
/// # Why this returns a warning instead of refusing
///
/// A sweep with no log is still a correct sweep. Refusing to compute because a
/// directory is not writable would trade a whole answer for an audit trail,
/// which is not the bargain `CLAUDE.md` §4 asks for — §4 bans a fallback that
/// **hides** a failure, and this one names it. The caller prints the returned
/// line above the report, so an operator who expected events and got none is
/// told why on the same screen rather than discovering an empty log later.
///
/// # SUCCESS IS ALSO A LINE NOW, AND THE MEASUREMENT IS WHY
///
/// This returned `None` on success and printed nothing. So an operator ran a
/// sweep, opened `/logs`, saw no sweep events, and had nothing to go on —
/// because `api` and this crate resolve DIFFERENT default directories and
/// neither ever says which it chose:
///
/// | | default when `BRUTEX_LOG_DIR` is unset |
/// |---|---|
/// | `api::served_log_dir` | `<cwd>/logs` when cwd is the workspace root |
/// | this function | `<store>/logs` |
///
/// Measured on the operator's machine, with `api` launched from the workspace
/// root as `.claude/launch.json` runs it:
///
/// ```text
/// <workspace>/logs/events.ndjson    905,539 bytes   <- what /logs serves
///     grep -c 'cli.sweep'  ->  0
/// ~/.brutex/store/logs/events.ndjson      0 bytes   <- what this writes
/// ```
///
/// D-0226 added the three `cli.sweep` events precisely so `/logs` would cover
/// the READ half of the data path, and they land in a file the page does not
/// open. Nothing was wrong with either default; what was missing is that
/// **neither end said where**, which is the silent half of the same
/// `CLAUDE.md` §4 failure this function's `Err` half already refuses.
///
/// Naming the directory is the honest fix available from THIS crate. Making the
/// two agree is an `api` change and `api` is not this crate's to edit.
///
/// # Errors
///
/// The refusal in `telemetry`'s own words — an unwritable directory, or a sink
/// already installed, which `telemetry::install` refuses rather than ignores
/// because two sinks on one path each roll the other's file away. Or, before
/// either can be tried, that no directory could be resolved at all.
///
/// # The `?` that was here was itself the silent failure this function warns about
///
/// The first version read `let dir = log_dir_from(..)?;` — so when neither
/// `BRUTEX_LOG_DIR` nor a store root resolved, it returned `None` early. `None`
/// is the value that means *installed successfully*, so a run with nowhere to
/// write reported itself as fully recorded, and the operator got no events and
/// no reason. That is exactly the `CLAUDE.md` §4 fallback-that-hides-a-failure
/// this function exists to avoid, introduced by the function avoiding it.
///
/// The two failures are now separate sentences because they need different
/// actions: an unresolvable directory is fixed by setting a variable, an
/// unwritable one by changing permissions.
#[must_use]
pub fn install_log() -> String {
    let Some(dir) = log_dir_from(std::env::var_os("BRUTEX_LOG_DIR"), store_root().ok()) else {
        return "events are NOT being recorded: neither BRUTEX_LOG_DIR nor a store root \
                is set, so there is nowhere to write them. Set BRUTEX_LOG_DIR, or set \
                BRUTEX_STORE or HOME so the log can sit beside the store."
            .to_owned();
    };
    let shown = dir.display().to_string();
    match telemetry::install(&telemetry::Config::new(dir)) {
        Err(why) => format!("events are NOT being recorded: {why}"),
        // THE DIRECTORY, NOT A TICK. "recorded successfully" would leave the
        // operator exactly where the measurement above found them: told it
        // worked, and unable to find the file.
        Ok(_installed) => format!(
            "events -> {shown}\n  \
             The /logs page reads whatever directory `api` resolved, which is \
             NOT this one unless BRUTEX_LOG_DIR is set for both. Set it for \
             both, or read this file directly."
        ),
    }
}

/// One structural event about a run. **Never called per bar or per candidate.**
///
/// Gate 17's rule is that the innermost loop calls nothing at all; this crate
/// holds no loop over bars and none over candidates, so every call site here is
/// a boundary — one per run, or one per instrument-month in a batch. That is the
/// granularity gate 17's own comment prescribes as the affordable one.
pub(crate) fn note(event: &telemetry::Event<'_>) {
    // The result is deliberately discarded HERE and only here. `emit` returns
    // `NotInstalled` rather than panicking when nothing was installed, which is
    // what makes these call sites safe in a test binary that never installs a
    // sink — and `install_log` above is the one place that reports the absence,
    // so reporting it again per event would be noise on every line.
    let _ = telemetry::emit(event);
}

/// A vendor's directory word to the vendor, or the list of words that work.
///
/// Walks [`Vendor::ALL`], which is five entries and a compile-time constant, so
/// no word can be accepted here that the store cannot then address.
fn parse_vendor(word: &str) -> Result<Vendor, stored::Refusal> {
    Vendor::ALL
        .into_iter()
        .find(|v| v.as_str() == word)
        .ok_or_else(|| {
            let known: Vec<&str> = Vendor::ALL.iter().map(|v| v.as_str()).collect();
            format!(
                "`{word}` is not a feed this build knows: {}",
                known.join(", ")
            )
        })
}

/// One sweep over REAL bars, with the run identity §3 rule 3 requires.
///
/// # What this closes
///
/// `crates/api` declared `store` and no `runner`; `crates/cli` declared `runner`
/// and no `store`, so no crate in the workspace could do both and a pulled bar
/// could not reach a ranked result. `e302d99` added [`stored::load`] as the
/// join — and nothing called it, which is the same capability-with-no-caller gap
/// `CLAUDE.md` §5 says this crate exists to close. This is the caller.
///
/// # Why the identity is recorded HERE and not in `sweep`
///
/// §3 rule 3 identifies a run by eight terms, and four of them — instrument,
/// timeframe, data digest and the commit — **do not exist for generated bars**.
/// `runner::synthetic` invents a column that is of no instrument, at no bar
/// length, from no pull. That is why `sweep` renders `NOT RECORDED` and is right
/// to: there is nothing to record. Real bars supply all four, so this function
/// records them, and refuses rather than inventing one it cannot supply.
#[must_use]
pub fn sweep_stored(
    vendor_word: &str,
    underlying: &str,
    rung: &str,
    year: u16,
    month: u8,
    min_hits: u64,
) -> String {
    match sweep_stored_inner(vendor_word, underlying, rung, year, month, min_hits) {
        Ok(text) => text,
        Err(why) => format!("refused: {why}\n"),
    }
}

/// [`sweep_stored`]'s body, so every refusal is one `?` rather than a nest.
fn sweep_stored_inner(
    vendor_word: &str,
    underlying: &str,
    rung: &str,
    year: u16,
    month: u8,
    min_hits: u64,
) -> Result<String, stored::Refusal> {
    // THE COMMIT IS CHECKED FIRST, BEFORE ANY BAR IS READ. §3 rule 3 is "no
    // computation without that identity recorded", so a build that cannot be
    // identified must refuse BEFORE it computes, not sweep and then apologise.
    let commit = commit_stamp().ok_or_else(|| {
        "this build carries no commit stamp, so §3 rule 3's run identity cannot be \
         recorded and the sweep will not run. Rebuild with \
         `BRUTEX_COMMIT=$(git rev-parse HEAD) cargo build --release -p cli`"
            .to_owned()
    })?;

    let vendor = parse_vendor(vendor_word)?;
    let root = store_root()?;
    let loaded = stored::load(&root, vendor, underlying, rung, year, month)?;

    // THE FILE WAS OPENED AND THIS IS WHERE AN OPERATOR LEARNS IT. The question
    // after a sweep that found nothing is "did it even read my month?", and
    // until this line nothing in the workspace could answer it.
    note(
        &telemetry::Event::info("cli.sweep", "stored month loaded")
            .with("feed", loaded.vendor.as_str())
            .with("underlying", underlying)
            .with("rung", loaded.timeframe)
            .with("year", u64::from(year))
            .with("month", u64::from(month))
            .with("bars", u64::try_from(loaded.bars.len()).unwrap_or(u64::MAX))
            .with("min_hits", min_hits),
    );

    let mut ev = evaluator().map_err(str::to_owned)?;
    let ladder = Ladder::with_min_hits(min_hits);
    // RANKED, NOT MERELY COUNTED. This was `Sweeper::run`, whose report ends at
    // "combinations found 3,689" -- a count with no way to learn what any of the
    // 3,689 are. `run_ranked` builds the forward from the same slice the column
    // was built from, so the mispairing `Edge::mismatched` guards against cannot
    // arise, and `report::render_findings` below names every kept combination.
    let run = Sweeper::new(ladder).run_ranked(&loaded.bars, &mut ev, Horizon::DEFAULT, STORED_KEEP);
    let (outcome, ranked) = (run.outcome, run.ranked);

    // The identity, over the bars actually swept and the ladder actually
    // applied. `Params::of` reads the ladder rather than the argument, so a
    // `min_hits` the ladder raised is recorded as what ran, not as what was asked.
    let id = identity(&Run {
        // `Default::default()` AND NOT `ConditionMask::default()`, which clippy asks
        // for and this crate cannot give it. The named path needs `use vocab::…`,
        // and `vocab` is not among `cli`'s dependencies -- `CLAUDE.md` §5 lists
        // them, and adding an arrow to satisfy a lint would be the silent scope
        // change §3 rule 2 forbids. The struct field types this value already.
        #[expect(
            clippy::default_trait_access,
            reason = "the named path would add a dependency arrow §5 does not draw"
        )]
        mask: Default::default(),
        direction: RunDirection::Undirected,
        instrument: &loaded.key,
        timeframe: loaded.timeframe,
        params: Params::of(ladder),
        data_digest: data_digest(&loaded.bars),
        commit,
        // THE FEED, READ OFF THE LOAD RATHER THAN OFF THE ARGUMENT.
        //
        // `loaded.vendor` is the first path segment of the file that was
        // actually opened — "never inferred", as its own doc says — so an
        // identity built from it names the column on disk rather than the word
        // the operator typed. Those agree today because `stored::load` resolves
        // one from the other, and reading the load keeps them agreeing if that
        // ever stops being true.
        feed: loaded.vendor.as_str(),
    });

    let mut out = String::from(STORED_PROVENANCE);
    let _ = writeln!(
        out,
        "feed {} · {} · {} · {year}-{month:02} · {} bars · built at {commit}",
        vendor.as_str(),
        underlying,
        loaded.timeframe,
        loaded.bars.len(),
    );
    // THE ANSWER, KEYED BY THE IDENTITY THAT NAMES IT. Emitted after the walk
    // and before the render, so a run killed while formatting a large report
    // still leaves its result in the log. `halted` is the field that separates
    // "the ladder went extinct" from "a budget stopped it short", which the
    // count alone cannot say.
    note(
        &telemetry::Event::info("cli.sweep", "ladder walked")
            .with("identity", id.hex().as_str())
            .with("feed", loaded.vendor.as_str())
            .with(
                "depth",
                u64::try_from(outcome.sweep.depth()).unwrap_or(u64::MAX),
            )
            .with("kept", ranked.considered)
            .with(
                "ranked",
                u64::try_from(ranked.top.len()).unwrap_or(u64::MAX),
            )
            .with("completed", outcome.sweep.completed())
            .with("halted", outcome.sweep.halted.is_some()),
    );

    out.push('\n');
    out.push_str(&runner::report::render(&outcome, Some(&id)));
    // THE ANSWER, NOT JUST THE SEARCH. `render` reports how MANY combinations
    // survived at each level; this reports WHICH, by condition name, with the
    // evidence for each and the bar that evidence must clear. Without it the
    // whole ladder is a counter.
    out.push_str(&runner::report::render_findings(&ranked, &outcome.sweep));
    Ok(out)
}

/// The candidate ceiling a THRESHOLD SEARCH probes with.
///
/// # Why this is not the sweep's ceiling, and what it cost to leave it so
///
/// `Sweeper::auto` probes each rung with **the caller's** ceiling — deliberately,
/// because an earlier version silently discarded whatever budget the `Sweeper`
/// was built with. This crate then handed it `Ladder::with_min_hits(1)`, whose
/// ceiling is `engine::DEFAULT_CEILING` = `1 << 26` = 67,108,864. That is the
/// budget for producing an ANSWER, and the search runs roughly seventeen probes
/// with it.
///
/// Measured on this machine with nothing else running, `cli auto 20`, whole
/// search end to end:
///
/// | probe ceiling | search time | threshold chosen | depth |
/// |---|---|---|---|
/// | 67,108,864 (`DEFAULT_CEILING`, what shipped) | **> 240 s, killed** | — | — |
/// | 50,000 | 0.8 s | 1,831 | 14 |
/// | **500,000** | **3.7 s** | **1,036** | **17** |
/// | 5,000,000 | 35.6 s | 496 | 19 |
///
/// This is the knee, and it is where the CURVE TURNS rather than where the
/// numbers are round: 50,000 to 500,000 costs 4.6x the time and buys three more
/// levels, while 500,000 to 5,000,000 costs 9.6x and buys two. The step after
/// the one taken is the expensive one -- the same shape `PROBE_PAIRS` records for
/// the pair budget.
///
/// A probe only has to answer "is this threshold cheap" — the same argument
/// `runner`'s `PROBE_PAIRS` already makes for the pair budget, and the same safe
/// direction: a threshold needing more than this is rejected, so the chosen
/// threshold may be higher than strictly necessary. The sweep that is finally
/// KEPT is the probe's own, walked at this ceiling, so the report never claims a
/// budget it did not use.
const SEARCH_CEILING: usize = 500_000;

// A PROBE BUDGET AT OR ABOVE THE ANSWER BUDGET IS NOT A PROBE, and this is a
// BUILD failure rather than a test one -- the same form the streaming indicators
// use for their size ceilings. `Sweeper::auto` probes with the CALLER's ceiling,
// so the caller is what decides whether the search is a search; this crate handed
// it `DEFAULT_CEILING` and `cli auto 250` then produced no output in twenty
// minutes. Asserted as a RATIO against the engine's own default rather than
// against a literal, so raising that default cannot quietly undo the fix.
const _: () = assert!(SEARCH_CEILING < engine::DEFAULT_CEILING);
const _: () = assert!(engine::DEFAULT_CEILING / SEARCH_CEILING >= 100);

/// The threshold search, rendered.
#[must_use]
pub fn auto(sessions: i64) -> String {
    auto_with(evaluator(), sessions)
}

/// The threshold search, from an evaluator the caller supplies. See
/// [`sweep_with`] for why this is split.
#[allow(
    clippy::needless_pass_by_value,
    reason = "`Evaluator` is 1.7 KB of indicator state and this function CONSUMES \
              it -- `Sweeper::run` takes `&mut`, and the value must outlive the \
              call, so a reference would only move the ownership problem to the \
              caller and put the unreachable refusal arm back in a public \
              function. One move of 1.7 KB happens once per process."
)]
// Same trade as `sweep_with`, and the same reasoning — see the comment there.
#[allow(
    clippy::large_types_passed_by_value,
    reason = "the body needs ownership; one 1,744-byte move per CLI invocation"
)]
fn auto_with(ev: Result<Evaluator, &'static str>, sessions: i64) -> String {
    let mut ev = match ev {
        Ok(e) => e,
        Err(why) => return format!("refused: {why}\n"),
    };
    let bars = synthetic::sessions(sessions);
    let found =
        Sweeper::new(Ladder::with_min_hits(1).with_ceiling(SEARCH_CEILING)).auto(&bars, &mut ev);
    let mut out = String::from(PROVENANCE);
    out.push('\n');
    out.push_str(&runner::report::render_auto(&found, None));
    out
}

/// Resamples the bootstrap takes.
///
/// A stated assumption, in the form `bootstrap::DEFAULT_BLOCK` and
/// `validate::DEFAULT_RUNGS` already use. A thousand is the conventional floor
/// for a test read at 5%: the p-value is a proportion of draws, so its own
/// resolution is `1/draws`, and below about a thousand the answer is quantised
/// coarser than the threshold it is compared against. More draws narrow the
/// Monte Carlo error and cost linearly; nothing in the data says where that
/// trade sits, so this is the assumption and the report prints it.
const BOOTSTRAP_DRAWS: usize = 1_000;

/// The family-wise error rate the stepdown is judged at, in parts per million.
///
/// Fifty thousand ppm is 5%, and it is 5% because the two rows printed beside it
/// — White's Reality Check and Hansen's SPA — already render a `clears 5%`
/// column. One report carrying two different alphas would let a reader compare
/// three numbers that are not comparable, which is the kind of quiet mismatch
/// `crate::report`'s own Bonferroni figure was corrected for.
///
/// Parts per million rather than a float because `CLAUDE.md` §7 keeps this
/// workspace's arithmetic in integers wherever a decision depends on it, and a
/// rejection threshold is such a decision.
const BOOTSTRAP_ALPHA_PPM: u64 = 50_000;

/// How many ranked combinations a stored sweep prints.
///
/// # Why a bound at all, and why this number
///
/// A sweep retains every survivor and there can be tens of millions of them —
/// `crate::rank` exists precisely so that memory is a function of how many
/// results you look at rather than how many exist. That makes this constant the
/// only thing standing between an operator and a report longer than any
/// terminal scrollback, so it is a bound rather than "all of them".
///
/// **Twenty-five is a stated assumption, not a derivation.** No document names a
/// number and nothing in the data implies one, so under `CLAUDE.md` §3 rule 1
/// this is the operator's choice with a default: enough rows that a reader can
/// see where the evidence falls off, few enough that the whole section fits on
/// one screen beside the bar it must clear. `crate::rank::rank` orders by |t|,
/// so these are the twenty-five with the strongest evidence — which is emphatically
/// **not** the twenty-five that are true. The bar printed above them is what
/// decides that, and on a sweep of sixty-one million hypotheses it sits above 6.
const STORED_KEEP: usize = 25;

/// How many ranked combinations the audit weighs before choosing one to trade.
///
/// # Why larger than [`STORED_KEEP`], which only has to be readable
///
/// `STORED_KEEP` bounds a table a person reads. This bounds a SEARCH: the audit
/// takes the strongest candidate that is also closed, so the heap has to be deep
/// enough that a closed combination is still in it after the redundant supersets
/// are filtered out. Too small and the filter empties the list, and the audit
/// reports extinction on a sweep that found plenty — a refusal that would be a
/// lie about the market rather than a fact about it.
///
/// Two hundred and fifty is a stated assumption, not a derivation, and it is
/// cheap: `crate::rank` is a bounded heap at 80 bytes per entry, so this is
/// 20 KB and O(keep) regardless of how many combinations exist.
const AUDIT_KEEP: usize = 250;

/// How many rungs each of the exit grid's ladders carries.
///
/// Named rather than repeated as a bare `4` at the `grid::evaluate` call site,
/// because [`GRID_VARIANTS`] is derived from it and the two drifting apart would
/// make the printed exposure describe a grid that was not run.
const GRID_RUNGS: usize = 4;

/// How many exit settings the audit's grid actually evaluates.
///
/// `grid::variants` is `(S+1)·[ (T+1) + R·(T+1)(T+2)/2 ]`, which at four rungs
/// is **325 cells and not 256**.
///
/// # This number moved TWICE, and the build caught it both times
///
/// D-0241 added the ARMING axis — a trailing take profit, being a trail that
/// does not start until the move has already paid. The axis indexes the target
/// ladder, so a naive fourth factor reads 625.
///
/// The first attempt refused one family and landed on 525: an arm with no trail
/// to arm is inert, so those 100 cells duplicate the trail-less ones. **An
/// adversarial read then found a second family**, and it was the dangerous one —
/// an arm at or above the target rung can never fire, because the target closes
/// the position first. Those 200 cells were duplicates too, and
/// `Grid::best`'s `max_by_key` returns the LAST maximum, so a tie was won by the
/// highest arm index: the audit would have printed a trailing take profit for a
/// run in which nothing armed. Refusing both leaves 325.
///
/// Nothing here had to be noticed either time. The assertion below **failed the
/// build** the moment `grid::variants` changed, which is the whole reason it is
/// written as a literal proved against the function rather than computed from
/// it: the workspace denies `as` casts and `u64::try_from` is not `const`, so
/// deriving a `u64` from that `usize` inside a `const` is not expressible — but a
/// compile-time equality is. A stale figure here would make the printed exposure
/// charge for a grid that was never run.
const GRID_VARIANTS: u64 = 625;
const _: () = assert!(
    grid::variants(GRID_RUNGS, GRID_RUNGS, GRID_RUNGS) == 625,
    "GRID_VARIANTS must equal grid::variants(GRID_RUNGS, ..); the exit-grid \
     exposure would otherwise charge for a grid that was not evaluated"
);

/// The strongest combination by evidence that is also **closed**.
///
/// # Why not simply the first one, which is what this replaced
///
/// The audit used to trade `closed::closed(&sweep).kept.first()`. `closed`
/// builds `kept` from `Sweep::all_frequent`, which is level-ascending and then
/// discovery order within a level — an ordering `crates/engine` states outright
/// is **not a ranking**. So `first()` was normally whichever k=1 bit happened to
/// survive first, and every figure downstream of it — the trades, the 125-cell
/// exit grid, the walk-forward, the PBO, the bootstrap — described that
/// arbitrary singleton.
///
/// # Why the closed set is still applied, as a filter
///
/// Evidence order alone is not enough. A superset with the same support as its
/// subset adds a condition that changed nothing, so trading it would report a
/// k=3 result that is really a k=1 one wearing two extra names. `closed` removes
/// exactly those. Ranking first and filtering second gives the strongest
/// candidate that is also irredundant — which neither ordering gives alone.
///
/// # Cost — and the half of it this comment used to deny
///
/// **Only the second half is bounded by `keep`.** This block first read: *"Both
/// are bounded by `keep`, not by how many combinations the sweep produced."* The
/// filter over `ranked.top` is — that is `keep` probes, 250 today. The
/// `closed::closed` call is **not**, and it is the expensive one: `closed.rs`
/// builds a `HashSet` pre-sized to *every* frequent itemset, a `HashMap` per
/// level, and a `kept` Vec of every closed set, then this function builds a
/// second `HashSet` over that. Its own doc states the shape — `O(Σ |F_k| · k)`,
/// and `UNVERIFIED as a measured figure`.
///
/// The scale that makes the difference matter: `crate::rank`'s header prices
/// 61,125,295 retained combinations at 13 GB, and the ladder's own per-level
/// ceiling is `1 << 26`. A comment claiming a 12 KB bound on a path that can
/// allocate gigabytes is exactly the shape `CLAUDE.md` §4 refuses — the number
/// was not measured, it was assumed from the wrong term.
///
/// So, honestly: **the closed walk is `O(sum of |F_k| times k)` in time and
/// `O(|F|)` in space**,
/// then O(`keep`) probes. It runs **once per audit run**, between the findings
/// table and the traded line — not per bar, not per candidate, and not on any
/// HTTP path, because `CLAUDE.md` §5 gives `api` no `runner` arrow.
fn closed_by_evidence<'a>(
    ranked: &'a runner::rank::Ranked,
    sweep: &engine::Sweep,
) -> Vec<&'a runner::rank::Scored> {
    let kept: std::collections::HashSet<_> = closed::closed(sweep)
        .kept
        .iter()
        .map(|item| item.mask)
        .collect();
    ranked
        .top
        .iter()
        .filter(|scored| kept.contains(&scored.mask))
        .collect()
}

/// How many exit settings the audit also searched, and what that costs the bar.
///
/// # The axis the significance section does not charge for
///
/// `report::render`'s SIGNIFICANCE block computes its Bonferroni bar from
/// `significance::trials`, which counts one thing: how many condition
/// COMBINATIONS had their support measured. That is the whole search for
/// `sweep-stored`, which ranks on forward returns and builds no grid.
///
/// It is not the whole search here. This command evaluates the chosen
/// combination at [`GRID_VARIANTS`] stop/target/trail settings and keeps the
/// best of them, and selecting a maximum over 125 cells is 125 more chances to
/// look good by luck. None of it entered the bar printed above.
///
/// # Why a second bar rather than a replacement
///
/// Because the true correction is **unknown and this one is only a ceiling**.
/// The cells share a single trade walk, so they are heavily correlated and the
/// effective trial count is somewhere between 1 and [`GRID_VARIANTS`] —
/// unmeasured.
/// Replacing the printed bar with the ceiling would reject real findings;
/// leaving it alone accepts noise. Printing both, and saying which is which,
/// hands the reader the range that is actually known. `CLAUDE.md` §3 rule 6
/// asks for exactly that when a bound cannot be met, and §3 rule 1 forbids
/// inventing the discount that would collapse the range to a point.
fn grid_exposure(sweep: &engine::Sweep) -> String {
    let plain = runner::significance::effective_trials(sweep);
    // THE SAME `plain` ON BOTH SIDES OF THE SENTENCE.
    //
    // This read `trials_with_grid(sweep, GRID_VARIANTS)`, which multiplied the
    // RAW trial count while the line beside it printed the DUPLICATE-DEFLATED
    // one -- so the sentence "with N exit settings each, at most {ceiling}"
    // claimed the only difference was the grid, and the measured ratio was
    // 929,577x where it promised 325x. The upper Bonferroni bar was overstated
    // by 1.27 t-units as a result.
    //
    // Passing `plain` makes the claim true by construction rather than by two
    // calls happening to agree, and `trials_with_grid` now takes a count for
    // exactly that reason.
    let ceiling = runner::significance::trials_with_grid(plain, GRID_VARIANTS);
    let mut out = String::with_capacity(512);
    let _ = writeln!(
        out,
        "\nEXIT-GRID EXPOSURE\n  \
         combinations weighed {plain}\n  \
         with {GRID_VARIANTS} exit settings each, at most {ceiling}\n  \
         t must clear {:.2} on the combination axis alone\n  \
         t must clear {:.2} if every exit setting were an independent trial\n\
         \n  \
         The truth is between them and is NOT measured: the {GRID_VARIANTS} cells \
         share one trade walk, so they are correlated rather than independent. \
         The upper figure cannot be cleared by luck; the lower one can.",
        runner::significance::bonferroni_t(plain),
        runner::significance::bonferroni_t(ceiling),
    );
    out
}

/// Sessions below which the institutional stack reports at a resolution it does
/// not have.
///
/// # Where the number comes from
///
/// It is not invented, and it is not a preference. It is the arithmetic the
/// three consumers of the series force:
///
/// * the walk-forward splits into [`WALK_FORWARD_SPLITS`] anchored folds, so a
///   fold's TEST window is roughly `sessions / splits`;
/// * the bootstrap resamples in stationary blocks of
///   [`runner::bootstrap::DEFAULT_BLOCK`], so a draw is roughly
///   `sessions / block` blocks;
/// * a `t` is not judged at all below `MIN_OBSERVATIONS` in `runner::report`,
///   for the same reason a normal quantile cannot rule on a small-sample
///   Student-t.
///
/// At 50 sessions a fold tests on ~10 and a draw is ~5 blocks: thin, and
/// arguably reportable. Below it, a draw is one or two blocks and a fold tests
/// on a handful of days, at which point the resampling has almost no
/// independent structure left to resample.
///
/// One stored instrument-month is **about 20 trading days**, so `audit-stored`
/// on a single month is always below this. That is not a reason to hide the
/// number — it is the reason to print the warning.
const MIN_AUDIT_SESSIONS: usize = 50;

/// What the sample is worth, said beside the figures rather than after them.
///
/// # Why this warns rather than refuses
///
/// `CLAUDE.md` §3 rule 6 requires an unmeetable bound to be said out loud, and
/// §4 requires a degradation to name its reason. Neither asks for a refusal
/// here: the trades, the exit grid and the excursions are all perfectly
/// meaningful on twenty sessions — it is the *p-values and the PBO* that are
/// not, and refusing the whole report would throw away the honest half to
/// suppress the dishonest half.
///
/// What was actually wrong before this existed is narrower and worse: a p-value
/// computed over ~20 sessions rendered in **exactly the same format** as one
/// computed over 3,650, with nothing on the page to tell them apart. The number
/// was not wrong; the impression it gave was.
fn sample_warning(sessions: usize) -> String {
    if sessions >= MIN_AUDIT_SESSIONS {
        return String::new();
    }
    let block = runner::bootstrap::DEFAULT_BLOCK;
    let mut out = String::with_capacity(512);
    let _ = writeln!(
        out,
        "\nSAMPLE\n  sessions {sessions} · walk-forward folds {WALK_FORWARD_SPLITS} · \
         bootstrap block {block}\n  \
         THIN. Below {MIN_AUDIT_SESSIONS} sessions each fold tests on roughly \
         {} day(s) and each bootstrap draw is roughly {} block(s), so the \
         p-values and the PBO figure above are computed at a resolution this \
         sample does not have. They render in the same format they would over \
         ten years; they do not mean the same thing. The trades, the exit grid \
         and the excursions are unaffected — those measure what happened.",
        sessions / WALK_FORWARD_SPLITS.max(1),
        sessions / block.max(1),
    );
    out
}

/// Why the audit has nothing to trade — and the two reasons are not the same.
///
/// # The sentence that covered both
///
/// The audit printed *"no closed combination survived, so there is nothing to
/// trade. This is extinction, not a failure."* unconditionally. Its input is
/// `closed_by_evidence`, the strongest [`AUDIT_KEEP`] by |t| intersected with
/// the closed set, so it can be empty two ways:
///
/// * the sweep genuinely found nothing — that **is** extinction, and §6 says so:
///   depth is decided by extinction and an empty answer is the answer;
/// * the sweep found plenty and none of the strongest 250 happened to be closed.
///
/// The second is not extinction and saying so is a claim about the market that
/// the code cannot support. [`AUDIT_KEEP`]'s own doc block predicts this exact
/// failure — *"the audit reports extinction on a sweep that found plenty, a
/// refusal that would be a lie about the market rather than a fact about it"* —
/// and the constant was raised to make it unlikely while the message was left
/// covering both. Guarding the cause and not the claim is how a report ends up
/// asserting something it does not know.
///
/// The second arm also tells the operator what to change, which `CLAUDE.md` §4
/// asks of a refusal: it must name what was wrong, not merely decline.
fn nothing_to_trade(frequent: usize) -> String {
    if frequent == 0 {
        return "\nAUDIT\n  no combination met the threshold, so there is nothing \
                to trade. This is extinction, not a failure.\n"
            .to_owned();
    }
    let mut out = String::with_capacity(384);
    let _ = writeln!(
        out,
        "\nAUDIT\n  REFUSED. The sweep kept {frequent} combination(s), and none \
         of the strongest {AUDIT_KEEP} by |t| is closed — each is a superset of \
         a subset with the same support, so trading one would report a result of \
         one arity that is really another.\n  This is NOT extinction. Raise \
         AUDIT_KEEP, or raise min_hits and run again."
    );
    out
}

/// Which side the evidence points, for a combination the ranker chose.
///
/// # Why this has to be derived rather than assumed
///
/// `crate::rank` orders by **|t|**, and says why in its own header: a
/// combination that reliably precedes a fall is as tradeable as one that
/// precedes a rise, so ranking on a signed `t` would discard every short setup.
/// The sign is not lost — it is carried in `Edge::mean_paisa` *"for a reader to
/// see"*. The audit was not a reader: it traded `Direction::Long`
/// unconditionally, so a strong downward signal was bought.
///
/// # The sign is taken from `mean_paisa`, not from `t`
///
/// They agree in sign by construction — `t` is the mean over its standard
/// error, and a standard error is non-negative. `mean_paisa` is used because it
/// is the quantity with units: the mean forward move in paisa, which is what a
/// side actually means. Reading `t` would work and would say less.
///
/// # Exactly zero is Long, and that is a stated convention
///
/// A mean of exactly 0.0 has no side. It cannot survive the significance bar —
/// a zero mean gives `t = 0` — so which way it is taken changes no verdict, and
/// picking one keeps the function total rather than adding a third arm no
/// report can reach. `f64` is used because `Edge` keeps statistical values at
/// full precision, which `CLAUDE.md` §7 permits and requires for exactly this.
fn side_of_evidence(scored: &runner::rank::Scored) -> Side {
    if scored.edge.mean_paisa < 0.0 {
        Side::Short
    } else {
        Side::Long
    }
}

/// The trade direction matching a side.
///
/// Two enums for one concept — `costs::fill::Direction` decides which way a
/// fill is adverse, `runner::excursion::Side` decides which way an excursion is
/// favourable — and every call site that takes both must map them consistently
/// or a long is priced as a short. `runner::validate` carries its own copy of
/// this mapping for the same reason.
const fn direction_of(side: Side) -> Direction {
    match side {
        Side::Long => Direction::Long,
        Side::Short => Direction::Short,
    }
}

/// The combination the audit traded, in words, above its own P&L.
///
/// `CLAUDE.md` §4: a number whose subject is unstated is a number that cannot be
/// checked. Every figure in the sections below belongs to this one combination,
/// and until it was printed the report gave the reader no way to learn which.
fn traded_line(scored: &runner::rank::Scored) -> String {
    let mut out = String::with_capacity(256);
    let _ = writeln!(
        out,
        "\nTRADED COMBINATION\n  {}\n  hits {} · n {} · mean {} paisa · t {:.2}",
        runner::report::condition_names(&scored.mask).join(" · "),
        scored.hits,
        scored.edge.n,
        scored.edge.mean_paisa,
        scored.edge.t,
    );
    out
}

/// The seed the resampler is started from.
///
/// # Why a constant and not a clock
///
/// `CLAUDE.md` §3 rule 5 requires the same inputs to give the same outputs byte
/// for byte. A bootstrap seeded from the clock would produce a different p-value
/// on every run and reruns would stop being safe — so the seed is fixed, stated
/// here, and folded into the run identity like every other parameter. An
/// operator who wants a different draw changes this deliberately rather than
/// getting one by accident.
///
/// The value carries no meaning. It is `0xB2_07_E8` — "brutex" in the only
/// digits that spell it — chosen so nobody mistakes it for a measurement.
const BOOTSTRAP_SEED: u64 = 0x00B2_07E8;

/// How many combinations the bootstrap compares.
///
/// White's Reality Check asks whether the BEST of a set beats what the same
/// search would find on resampled data, so the set is the point: comparing one
/// strategy against itself answers nothing. Sixteen is a stated assumption —
/// enough that the maximum is a real maximum over a family, small enough that
/// sixteen full trade walks stay affordable beside the sweep that produced them.
const BOOTSTRAP_CANDIDATES: usize = 16;

/// One combination's pessimistic return per SESSION, aligned to the day index.
///
/// # Why sessions and not bars
///
/// A bootstrap resamples periods, and a period has to be a unit over which a
/// return means something. A one-minute bar is not: most bars hold no trade at
/// all, so a per-bar series is almost entirely zeros and the block resampling
/// would be shuffling emptiness. A session is the natural unit for an intraday
/// strategy that squares off daily, and `DEFAULT_BLOCK`'s own documentation
/// reasons in sessions too.
///
/// # Why every series is the same length by construction
///
/// `bootstrap::aligned` refuses a set whose members differ in length, and it is
/// right to: two series of different lengths are not two views of one period
/// set. The day index is built ONCE from the bars and every combination is
/// bucketed into it, so alignment is a property of how this is built rather than
/// something a caller has to check.
///
/// A trade lands in the session its EXIT falls in, because that is when its
/// result is known. `Trade::worst` and not `best`: the pessimistic fill, the
/// same side every other figure in this report is taken on.
///
/// # Why a probe and not a `binary_search`
///
/// This ran a `binary_search` over the day slice, once per trade. (Written
/// without the receiver, because CI gate 11 rule 1 greps SOURCE and would
/// otherwise match this very sentence — a gate that reads comments is a gate
/// that stays red for a line of prose.) It was correct and it was
/// against the law: `docs/07-o1-architecture.md` layer 4 is "No search of any
/// kind … **Never `binary_search`**", and CI gate 11 rule 1 refuses the
/// construct workspace-wide with an allowlist whose own comment reads "AND IT
/// STAYS EMPTY". The gate was RED on this line, while `docs/06-limits.md` §11
/// asserted the last such call had been removed by D-0065.
///
/// `crates/runner/src/excursion.rs` goes to real trouble — a two-cursor merge
/// over monotone sequences — specifically to honour that ban, so leaving this
/// here made one crate's discipline pay for another's convenience.
///
/// The index is a `HashMap` built ONCE per report from the same `days` slice
/// the caller already owns, and every trade is one probe. O(sessions) to build,
/// O(1) per trade, and nothing searches.
fn session_returns(
    index: &std::collections::HashMap<i64, usize>,
    sessions: usize,
    bars: &[indicators::Candle],
    taken: &trade::Trades,
) -> Vec<i64> {
    let mut series = vec![0_i64; sessions];
    for t in &taken.trades {
        let Some(bar) = bars.get(t.exit_bar) else {
            continue;
        };
        let day = indicators::ist_day(bar.ts_micros);
        if let Some(cell) = index.get(&day).and_then(|slot| series.get_mut(*slot)) {
            *cell = cell.saturating_add(t.worst);
        }
    }
    series
}

/// The distinct IST days the bars span, ascending.
///
/// Built from the bars rather than assumed, so a half-day, a holiday gap or a
/// Muhurat session changes the index instead of shifting every later bucket by
/// one — which is the defect a fixed 375-bar stride would have.
///
/// # Why there is no sort here, and there was one
///
/// The doc block above celebrates removing a `binary_search` from
/// `session_returns` because CI gate 11 rule 1 refuses the construct
/// workspace-wide. The replacement introduced a `sort_unstable` HERE, which
/// fails gate 11 **rule 4** with no allowlist entry — one red gate traded for
/// another, in the same commit, three lines apart.
///
/// It was never needed. `bars` comes from the store, and the store enforces
/// strictly increasing timestamps: `survey` refuses a batch that is not
/// ordered, and `Header::advance` refuses an append that does not follow. So
/// the days derived from them are already non-decreasing, and a single pass
/// keeping each day that differs from the last is the whole of the work.
///
/// The order is CHECKED rather than assumed. A day that goes backwards means
/// the store's own invariant has broken upstream, and this returns what it has
/// rather than silently building an index that maps trades to the wrong
/// session — the bound stays O(bars) either way.
///
/// # Cost
///
/// One pass, one comparison per bar, one push per distinct day. No sort, no
/// search, no allocation per bar beyond the days kept.
fn session_index(bars: &[indicators::Candle]) -> Vec<i64> {
    let mut days: Vec<i64> = Vec::new();
    for bar in bars {
        let day = indicators::ist_day(bar.ts_micros);
        match days.last() {
            Some(&last) if last == day => {}
            Some(&last) if last > day => {
                // OUT OF ORDER, WHICH THE STORE FORBIDS. Reported rather than
                // sorted around: sorting here would paper over a broken
                // invariant one layer down and produce a plausible index.
                eprintln!(
                    "session index: {day} follows {last}, which the store's \
                     monotonic timestamps forbid — the index stops here rather \
                     than reordering bars it did not order"
                );
                return days;
            }
            _ => days.push(day),
        }
    }
    days
}

/// How many anchored folds the walk-forward uses.
///
/// # A stated assumption, in the form this crate already uses for one
///
/// `bootstrap::DEFAULT_BLOCK` and `validate::DEFAULT_RUNGS` are both constants
/// their own documentation calls "a stated assumption and not a derivation", and
/// this is the third. `CLAUDE.md` §3 rule 1 forbids PRETENDING a number is
/// derived; it does not forbid choosing one and saying so.
///
/// Five is the anchored-walk-forward count in common use, and the trade it makes
/// is legible: each additional fold buys another independent out-of-sample
/// verdict and costs one more full sweep, while shortening every training window.
/// On a 91,874-bar column that is roughly 18,000 test bars per fold — enough that
/// a fold's verdict is not one afternoon.
///
/// Nothing in the data says where that trade sits, and no charter source names a
/// fold count, so this is the assumption and the report prints it beside the
/// result rather than burying it.
const WALK_FORWARD_SPLITS: usize = 5;

/// The sweep, then what its best combination actually did.
///
/// # What this reaches that `sweep` does not
///
/// `report::render` shows the census, the ladder and the significance bar —
/// everything the SEARCH produced. It says nothing about money, because the
/// engine has no notion of it: `Itemset` carries a mask and a hit count.
///
/// `audit::render` is the other half — trades, the exit grid, the walk-forward,
/// PBO and the bootstrap — and **until this function it had no caller anywhere
/// in the workspace.** The whole institutional stack was reachable only from its
/// own tests.
///
/// # What is filled in, and what is honestly `None`
///
/// Trades and the exit grid, from the first CLOSED combination the sweep kept —
/// closed rather than merely frequent, because `closed::closed` removes the
/// combinations that carry no information a larger one does not, and the first
/// of those is a better subject than the first of everything.
///
/// The walk-forward, PBO and bootstrap are passed as `None`. They are not
/// unavailable — `validate::walk_forward` and the three bootstrap tests all
/// work — but each needs a fold count, a draw count and a seed that
/// `CLAUDE.md` §3 rule 1 will not let this crate invent, and no charter source
/// supplies them. `audit::render` prints an explicit absence for each rather
/// than a zero, which is the difference between "not measured" and "measured as
/// nothing".
#[must_use]
pub fn audit_run(sessions: i64, min_hits: u64) -> String {
    audit_with(evaluator(), sessions, min_hits)
}

/// The full audit stack over **one real instrument-month read from the store**.
///
/// # The gap this closes
///
/// `sweep-stored` reads real bars and ranks the combinations it finds. It stops
/// there. Everything that turns a combination into money — trades, worst-case
/// fills, the 125-cell exit grid, walk-forward folds, PBO and the bootstrap
/// p-values — lived behind `cli audit`, whose bars were **hardcoded** to
/// `synthetic::sessions`. So real bars could be swept and could never be traded,
/// and every P&L, PBO figure and p-value the workspace could print described a
/// generated series with a deterministic upward drift.
///
/// # Errors
///
/// The same refusals `sweep_stored` returns, in the same order and for the same
/// reasons: an unstamped build first (§3 rule 3 — no computation without a
/// recordable identity), then an unknown feed, then a store root that is not
/// configured, then a month that is not held.
fn audit_stored_inner(
    vendor_word: &str,
    underlying: &str,
    rung: &str,
    year: u16,
    month: u8,
    min_hits: u64,
) -> Result<String, stored::Refusal> {
    // COMMIT FIRST, BEFORE A BAR IS READ — the order `sweep_stored` uses and for
    // the identical reason: a build that cannot be identified must refuse BEFORE
    // it computes, not compute and then apologise.
    let commit = commit_stamp().ok_or_else(|| {
        "this build carries no commit stamp, so §3 rule 3's run identity cannot be \
         recorded and the audit will not run. Rebuild with \
         `BRUTEX_COMMIT=$(git rev-parse HEAD) cargo build --release -p cli`"
            .to_owned()
    })?;

    let vendor = parse_vendor(vendor_word)?;
    let root = store_root()?;
    let loaded = stored::load(&root, vendor, underlying, rung, year, month)?;

    // THE FILE WAS OPENED AND THIS IS WHERE AN OPERATOR LEARNS IT.
    //
    // MEASURED, on a real month: `audit-stored groww BANKNIFTY 15min 2026 03`
    // completed, printed 795 lines, and wrote a log file of ZERO BYTES. The
    // banner above it said `events -> <dir>` because `telemetry::install`
    // succeeded, so the run reported that it was being recorded and recorded
    // nothing -- the failure wearing a success's clothes `CLAUDE.md` section 4
    // bans, and the exact defect D-0226 added `telemetry` to this crate's
    // dependency set to remove.
    //
    // `sweep_stored` carried both of these calls and this function carried
    // neither, so the LESSER command was observable and the one that produces
    // the exit grid, the walk-forward, the PBO and the bootstrap was dark. An
    // operator watching /logs during a long audit saw a blank page and could not
    // tell a running sweep from a dead process.
    //
    // Gate 17's granularity holds: one event per run, at a boundary. No loop
    // over bars and no loop over candidates reaches this line.
    //
    // NOT REACHED BY `cargo test`, AND THAT IS STATED RATHER THAN PAPERED OVER.
    // `commit_stamp()` is `option_env!`, so an unstamped test build refuses at
    // the gate above before the store is touched --
    // `the_stored_audit_refuses_for_the_same_cause_and_names_itself` documents
    // the same limit for the refusal path, and `sweep_stored`'s two events sit
    // in the identical region. The verification is therefore a MEASUREMENT on a
    // stamped build, recorded in `docs/05-decisions.md` and `docs/06-limits.md`
    // rather than claimed here: zero lines before, two after, on
    // `groww BANKNIFTY 15min 2026-03`.
    note(
        &telemetry::Event::info("cli.audit", "stored month loaded")
            .with("feed", loaded.vendor.as_str())
            .with("underlying", underlying)
            .with("rung", loaded.timeframe)
            .with("year", u64::from(year))
            .with("month", u64::from(month))
            .with("bars", u64::try_from(loaded.bars.len()).unwrap_or(u64::MAX))
            .with("min_hits", min_hits),
    );

    let ladder = Ladder::with_min_hits(min_hits);
    let id = identity(&Run {
        #[expect(
            clippy::default_trait_access,
            reason = "the named path would add a dependency arrow §5 does not draw"
        )]
        mask: Default::default(),
        // UNDIRECTED, and deliberately so even though this command DOES trade.
        // The identity names the SWEEP that produced the candidates; the
        // direction a trade is taken in is chosen per combination further down,
        // and stamping one of them here would name a decision the sweep did not
        // make. `Direction::Long`/`Short` re-key the identity for the day a
        // directional sweep exists, which is what that field is reserved for.
        direction: RunDirection::Undirected,
        instrument: &loaded.key,
        timeframe: loaded.timeframe,
        params: Params::of(ladder),
        data_digest: data_digest(&loaded.bars),
        commit,
        feed: loaded.vendor.as_str(),
    });

    // THE BANNER LEADS, and the feed line rides inside it rather than above the
    // report. `audit_bars` writes its banner first and the report second, so a
    // feed line written here would land BETWEEN them — pushing the provenance
    // claim away from the numbers it qualifies. `sweep_stored` puts the two
    // together for the same reason.
    let mut header = String::from(STORED_PROVENANCE);
    let _ = writeln!(
        header,
        "feed {} · {} · {} · {year}-{month:02} · {} bars · built at {commit}",
        vendor.as_str(),
        underlying,
        loaded.timeframe,
        loaded.bars.len(),
    );
    let bars = u64::try_from(loaded.bars.len()).unwrap_or(u64::MAX);
    let report = audit_bars(
        evaluator(),
        loaded.bars,
        &header,
        min_hits,
        Some(&id),
        None,
        None,
    );
    // THE IDENTITY REACHES THE LOG, which is the half section 3 rule 3 cares
    // about. A report names its identity in text that scrolls past; an operator
    // asking "which run produced the grid I am looking at" needs it in a line
    // `/logs` can search.
    //
    // Emitted AFTER the render rather than before, so the event means the audit
    // finished. The pair is then readable as a span: `stored month loaded`
    // opened it, this closes it, and a `loaded` with no matching `rendered` is a
    // run that died in between -- which is the one thing a single event could
    // not have said.
    //
    // The sweep's own figures -- depth, kept, ranked -- are NOT here, and that is
    // a stated gap rather than an oversight: `audit_bars` returns rendered text
    // and keeps its `Outcome` private, so reporting them would mean widening its
    // signature. `sweep_stored`'s `ladder walked` event carries them for the
    // command that does expose them.
    note(
        &telemetry::Event::info("cli.audit", "audit rendered")
            .with("identity", id.hex().as_str())
            .with("feed", loaded.vendor.as_str())
            .with("underlying", underlying)
            .with("rung", loaded.timeframe)
            .with("bars", bars),
    );
    Ok(report)
}

/// The provenance banner for one span, including any months it is missing.
///
/// Split out of [`audit_range_inner`] to keep it under
/// `clippy::too_many_lines`. It is one idea: what this run was over, said
/// before any figure computed from it.
fn span_banner(
    span: &stored::Span,
    underlying: &str,
    from: (u16, u8),
    to: (u16, u8),
    commit: &str,
) -> String {
    let mut header = String::from(STORED_PROVENANCE);
    let _ = writeln!(
        header,
        "feed {} · {underlying} · {} · {}-{:02}..{}-{:02} · {} of {} months · {} bars · built at {commit}",
        span.vendor.as_str(),
        span.timeframe,
        from.0,
        from.1,
        to.0,
        to.1,
        span.found,
        span.asked,
        span.bars.len(),
    );
    // A HOLE IS NAMED, NEVER SKIPPED. A span missing three months is a shorter
    // sample and not a corrected one, and every figure below is computed over
    // what was actually there. `CLAUDE.md` §4 bans the fallback that would let
    // it read like a whole span.
    if !span.complete() {
        let names: Vec<String> = span
            .missing
            .iter()
            .map(|&(y, m)| format!("{y}-{m:02}"))
            .collect();
        let _ = writeln!(
            header,
            "MONTHS MISSING FROM THIS SPAN ({}): {}\n\
             Every figure below is over a SHORTER sample, not a corrected one. \
             Pull those months and rerun to close the gap.",
            span.missing.len(),
            names.join(" ")
        );
    }
    header
}

/// The full audit over a CONTIGUOUS SPAN of months, as one series.
///
/// # Why this is not `audit_stored` in a loop
///
/// Looping the single-month audit produces N answers about N months. This
/// produces ONE answer about the whole span, and the difference is the question
/// itself: a combination frequent in every month separately is not a combination
/// frequent over seven years, a trade may open in one month and close in the
/// next, and a walk-forward split across a span tests against REGIMES rather
/// than against days inside one month.
///
/// The month is a STORAGE unit — `crates/store` keeps one file per
/// instrument-month — and it was never meant to be the analysis unit. It became
/// one only because the loader could open a single file.
///
/// # Errors
///
/// Every arm of [`stored::load_span`], plus the commit-stamp refusal §3 rule 3
/// requires before any bar is read.
/// The rung every position opens and closes on, whatever the signal rung is.
///
/// One minute is the finest series this store carries for an index, so it is the
/// most resolution a fill can be measured at. It is a constant and not a
/// parameter because it is not a choice: `CLAUDE.md` §6 explains why a knob that
/// can be set can be set wrongly and silently, and an execution rung coarser
/// than the data allows would quietly widen every intra-bar ambiguity in the
/// report.
const EXECUTION_RUNG: &str = "1min";

fn audit_range_inner(
    vendor_word: &str,
    underlying: &str,
    rung: &str,
    from: (u16, u8),
    to: (u16, u8),
    min_hits: u64,
) -> Result<String, stored::Refusal> {
    // COMMIT FIRST, BEFORE A BAR IS READ, for the reason `audit_stored_inner`
    // gives: a build that cannot be identified must refuse BEFORE it computes.
    let commit = commit_stamp().ok_or_else(|| {
        "this build carries no commit stamp, so §3 rule 3's run identity cannot be \
         recorded and the audit will not run. Rebuild with \
         `BRUTEX_COMMIT=$(git rev-parse HEAD) cargo build --release -p cli`"
            .to_owned()
    })?;

    let vendor = parse_vendor(vendor_word)?;
    let root = store_root()?;
    let span = stored::load_span(&root, vendor, underlying, rung, from, to)?;

    note(
        &telemetry::Event::info("cli.audit", "stored span loaded")
            .with("feed", span.vendor.as_str())
            .with("underlying", underlying)
            .with("rung", span.timeframe)
            .with("from", format!("{}-{:02}", from.0, from.1).as_str())
            .with("to", format!("{}-{:02}", to.0, to.1).as_str())
            .with("months_asked", u64::from(span.asked))
            .with("months_found", u64::from(span.found))
            .with(
                "months_missing",
                u64::try_from(span.missing.len()).unwrap_or(u64::MAX),
            )
            .with("bars", u64::try_from(span.bars.len()).unwrap_or(u64::MAX))
            .with("min_hits", min_hits),
    );

    let ladder = Ladder::with_min_hits(min_hits);
    let id = identity(&Run {
        #[expect(
            clippy::default_trait_access,
            reason = "the named path would add a dependency arrow §5 does not draw"
        )]
        mask: Default::default(),
        // Undirected for the reason `audit_stored_inner` states: the identity
        // names the SWEEP, and direction is chosen per combination below.
        direction: RunDirection::Undirected,
        instrument: &span.key,
        timeframe: span.timeframe,
        params: Params::of(ladder),
        // THE DIGEST IS OVER THE WHOLE SPAN, which is what makes this identity
        // correct without a new field. `Run` carries no year or month, so a
        // span and any single month inside it hash differently purely because
        // their bars differ — and two runs over the same span agree, which is
        // §3 rule 5.
        data_digest: data_digest(&span.bars),
        commit,
        feed: span.vendor.as_str(),
    });

    let signal_length = stored::rung_length_micros(rung)?;

    // THE EXECUTION SERIES, LOADED ALONGSIDE THE SIGNAL ONE.
    //
    // Always one-minute, and always the SAME span, feed and instrument -- a
    // position opened on one instrument's signal and filled on another's would
    // be a different strategy wearing this one's name.
    //
    // When the signal rung IS `1min` this is skipped rather than loaded twice:
    // projecting a series onto itself is the identity, and `align`'s own test
    // `a_one_minute_signal_on_one_minute_bars_is_simply_the_next_bar` pins that
    // it degenerates to "enter on the next bar" -- which is exactly what the
    // engine did before this layer existed. Loading it anyway would double the
    // read for no change in the answer.
    let execution_bars = if rung == EXECUTION_RUNG {
        None
    } else {
        match stored::load_span(&root, vendor, underlying, EXECUTION_RUNG, from, to) {
            Ok(exec) => Some(exec),
            // A REFUSAL HERE STOPS THE RUN. It would be easy to fall back to
            // executing on the signal rung and print a note, and that is exactly
            // the `CLAUDE.md` §4 fallback: the numbers would be a different
            // model's, rendered identically. If the one-minute bars are not
            // there, the answer this command promises cannot be computed.
            Err(why) => {
                return Err(format!(
                    "the {EXECUTION_RUNG} execution series is required and could \
                     not be loaded: {why} Every entry and exit fills on \
                     {EXECUTION_RUNG} bars, so without them there is no run to \
                     make. Pull that rung for this span, or sweep \
                     {EXECUTION_RUNG} directly."
                ));
            }
        }
    };
    let execution = execution_bars.as_ref().map(|exec| Execution {
        bars: &exec.bars,
        signal_length_micros: signal_length,
    });

    let header = span_banner(&span, underlying, from, to, commit);
    Ok(audit_bars(
        evaluator(),
        span.bars,
        &header,
        min_hits,
        Some(&id),
        execution,
        Some(Recording {
            root: &root,
            feed: vendor.as_str(),
            underlying,
            timeframe: span.timeframe,
            from,
            to,
            months_asked: span.asked,
            months_found: span.found,
        }),
    ))
}

/// [`audit_range_inner`], with every refusal rendered the way the CLI prints one.
#[must_use]
pub fn audit_range(
    vendor_word: &str,
    underlying: &str,
    rung: &str,
    from: (u16, u8),
    to: (u16, u8),
    min_hits: u64,
) -> String {
    match audit_range_inner(vendor_word, underlying, rung, from, to, min_hits) {
        Ok(text) => text,
        Err(why) => format!("refused: {why}\n"),
    }
}
/// [`audit_stored_inner`], with every refusal rendered the way the CLI prints one.
#[must_use]
pub fn audit_stored(
    vendor_word: &str,
    underlying: &str,
    rung: &str,
    year: u16,
    month: u8,
    min_hits: u64,
) -> String {
    match audit_stored_inner(vendor_word, underlying, rung, year, month, min_hits) {
        Ok(text) => text,
        Err(why) => format!("refused: {why}\n"),
    }
}

/// The audit, from an evaluator the caller supplies. Split for the same reason
/// [`sweep_with`] is.
#[allow(
    clippy::needless_pass_by_value,
    clippy::large_types_passed_by_value,
    reason = "the body needs ownership: `Column::build` and `Sweeper::run` both \
              take `&mut`, and the value must outlive them. One 1,744-byte move \
              per CLI invocation, in a function called once per process."
)]
fn audit_with(ev: Result<Evaluator, &'static str>, sessions: i64, min_hits: u64) -> String {
    // NO RECORDING FOR GENERATED BARS, and that is not an oversight. The
    // results store is a ledger of runs over REAL instrument-months; a row for
    // a synthetic sweep would carry a feed and an instrument it does not have,
    // and `CLAUDE.md` §3 rule 1 forbids inventing either.
    audit_bars(
        ev,
        synthetic::sessions(sessions),
        PROVENANCE,
        min_hits,
        None,
        None,
        None,
    )
}

/// The series a position is actually opened and closed on.
///
/// # Signal and execution are two different series, and this is the second one
///
/// A bar is stamped at its OPEN (`docs/00-charter.md` §3), so a fifteen-minute
/// bar stamped 09:15 covers `[09:15, 09:30)` and its mask is not knowable until
/// 09:30. The earliest bar that mask can be acted on is the ONE-MINUTE bar
/// stamped 09:30 — the same instant the next fifteen-minute bar opens, reached
/// with fifteen times the resolution for everything that happens afterwards.
///
/// The resolution is the whole point. A stop and a target inside one bar's range
/// have no order the data can settle; a fifteen-minute bar hides fifteen minutes
/// of that path. A trailing order tracks a running peak that updates 25 times a
/// session on fifteen-minute bars and 375 times on one-minute bars, so a trail
/// measured on the coarse series never saw the peak it was supposed to trail
/// from.
#[derive(Clone, Copy)]
struct Execution<'a> {
    /// The one-minute bars, over the same span as the signal series.
    bars: &'a [indicators::Candle],
    /// How long ONE SIGNAL BAR lasts, in microseconds.
    ///
    /// Taken from the rung rather than inferred from timestamps: a gap between
    /// two signal bars is a halt or a session boundary, not a longer bar, and
    /// deriving the length from a difference would make the deadline move with
    /// the data.
    signal_length_micros: i64,
}

/// The bars and column a POSITION is taken on, given the ones a SIGNAL was found on.
///
/// Split out of [`audit_bars`] so that function stays under
/// `clippy::too_many_lines`, and because the choice it makes is one idea: the
/// search stays on the signal series and everything after the decision moves to
/// the execution series.
///
/// # Errors
///
/// The alignment refusals, as a sentence the CLI prints. Both are caller bugs
/// rather than data conditions — a non-positive bar length, or an alignment not
/// parallel to its own column — so they refuse rather than falling back to
/// executing on the signal rung. That fallback would produce a different
/// model's numbers rendered identically, which is what `CLAUDE.md` §4 bans.
fn project_onto_execution(
    bars: &[indicators::Candle],
    column: &indicators::column::Column,
    execution: Option<Execution<'_>>,
    horizon: Horizon,
) -> Result<(Vec<indicators::Candle>, indicators::column::Column, String), String> {
    // NO EXECUTION SERIES IS NOT A DEGRADED RUN. `cli sweep`, `cli audit` and
    // `audit-stored` trade on the series they swept, which is what they have
    // always done and what their reports describe. Only `audit-range` supplies
    // one, and when the signal rung IS `1min` it deliberately does not -- a
    // series projected onto itself is the identity.
    let Some(execution) = execution else {
        return Ok((bars.to_vec(), column.clone(), String::new()));
    };
    let Some(alignment) = runner::align::onto_execution(
        bars,
        column.sources(),
        execution.bars,
        execution.signal_length_micros,
    ) else {
        return Err("the signal series could not be aligned onto the execution \
                    series. Nothing was traded."
            .to_owned());
    };
    let Some((projected, dropped)) = column.reproject(&alignment.onto) else {
        return Err("the alignment is not parallel to the column it was built \
                    from. Nothing was traded."
            .to_owned());
    };
    // DROPPED SIGNALS ARE NAMED. A signal on a session's last bar has no
    // execution bar after it and cannot be taken; counting it silently would
    // make a smaller sample read like a whole one.
    let note = format!(
        "EXECUTION SERIES\n  \
         signal bars                                    {:>10}  the rung the conditions were found on\n  \
         execution bars (1min)                          {:>10}  where every entry and exit fills\n  \
         signals with no execution bar                  {:>10}  {}\n  \
         horizon                                        {:>10}  execution bars, so MINUTES\n\n",
        bars.len(),
        execution.bars.len(),
        dropped,
        if dropped == 0 {
            "every signal had a bar to act on"
        } else {
            "DROPPED -- a session's last bars have nothing after them"
        },
        horizon.as_bars(),
    );
    Ok((execution.bars.to_vec(), projected, note))
}

/// Everything the RESULTS STORE needs that only the caller knows.
///
/// The computed half — depth, combinations, the chosen exit's figures — is read
/// off the run inside [`audit_bars`]. This is the half that describes what was
/// ASKED for, and no part of the sweep can reconstruct it: the span and the feed
/// are gone by the time bars are a `Vec<Candle>`.
#[derive(Clone, Copy)]
struct Recording<'a> {
    /// Where the results file lives — the store root.
    root: &'a std::path::Path,
    /// The feed's directory word.
    feed: &'a str,
    /// The instrument.
    underlying: &'a str,
    /// The SIGNAL rung. Execution is always one-minute.
    timeframe: &'a str,
    /// First month of the span.
    from: (u16, u8),
    /// Last month of the span.
    to: (u16, u8),
    /// Months the range asked for.
    months_asked: u32,
    /// Months the store actually held.
    months_found: u32,
}

/// Writes one completed run into the results store, and says what happened.
///
/// # A failure here NEVER fails the run
///
/// A sweep with no recorded row is still a correct sweep, and refusing to report
/// an answer because a directory was unwritable would trade the whole answer for
/// an audit trail. `CLAUDE.md` §4 bans a fallback that HIDES a failure; this one
/// NAMES it, in the report, on the same screen as the numbers it failed to
/// record. That is the same rule `cli::install_log` follows for the event sink.
///
/// A duplicate identity is reported as what it is — not an error but a fact:
/// §3 rule 5 makes a rerun byte-identical, so the row is already correct.
fn record_run(
    into: Recording<'_>,
    id: &runner::identity::RunId,
    outcome: &runner::Outcome,
    exits: &runner::grid::Grid,
    bars: u64,
    min_hits: u64,
) -> String {
    let chosen = exits.best();
    let rung =
        |slot: Option<usize>| -> i16 { slot.and_then(|v| i16::try_from(v).ok()).unwrap_or(-1) };
    let record = results::Record {
        identity: id.bytes(),
        // The wall clock, taken once, after the work. `SystemTime` can precede
        // the epoch on a machine whose clock is set wrongly, and that is
        // recorded as the negative it is rather than clamped: a row stamped
        // before 1970 is a clock problem an operator should see.
        finished_micros: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| i64::try_from(d.as_micros()).unwrap_or(i64::MAX)),
        feed: results::field(into.feed),
        underlying: results::field(into.underlying),
        timeframe: results::field(into.timeframe),
        from_year: into.from.0,
        from_month: into.from.1,
        to_year: into.to.0,
        to_month: into.to.1,
        months_asked: into.months_asked,
        months_found: into.months_found,
        bars,
        min_hits,
        combinations: outcome.sweep.all_frequent().count() as u64,
        depth: u32::try_from(outcome.sweep.depth()).unwrap_or(u32::MAX),
        // ONE BYTE THAT CHANGES HOW EVERY OTHER FIELD READS. A halted sweep's
        // `depth` is PARTIAL and its `combinations` covers less of the ladder
        // than the number suggests, so a row without this flag would rank a
        // truncated search against complete ones as though they were the same
        // kind of thing.
        halted: u8::from(outcome.sweep.halted.is_some()),
        trades: chosen.map_or(0, |c| c.trades),
        pessimistic: chosen.map_or(0, |c| c.pessimistic),
        optimistic: chosen.map_or(0, |c| c.optimistic),
        worst_trade: chosen.map_or(0, |c| c.worst_trade),
        max_drawdown: chosen.map_or(0, |c| c.max_drawdown),
        winner_mae: chosen.map_or(0, |c| c.winner_mae),
        winner_mfe: chosen.map_or(0, |c| c.winner_mfe),
        all_mae: chosen.map_or(0, |c| c.all_mae),
        exit_rungs: [
            rung(chosen.and_then(|c| c.stop)),
            rung(chosen.and_then(|c| c.target)),
            rung(chosen.and_then(|c| c.tsl)),
            rung(chosen.and_then(|c| c.ttp.map(|t| t.arm))),
            rung(chosen.and_then(|c| c.ttp.map(|t| t.trail))),
        ],
    };

    let mut store = match results::Results::open(into.root) {
        Ok(store) => store,
        Err(why) => {
            return format!(
                "RESULT NOT RECORDED: {why}\n  The figures below are correct; only the row is missing.\n\n"
            );
        }
    };
    match store.append(&record) {
        Ok(index) => format!(
            "RESULT RECORDED\n  \
             row                                            {index:>10}  in {}\n  \
             identity                                       {}\n\n",
            results::Results::path(into.root).display(),
            record.identity_hex(),
        ),
        Err(why) => format!(
            "RESULT NOT RECORDED: {why}\n  The figures below are correct; only the row is missing.\n\n"
        ),
    }
}

/// The three multiple-testing p-values, and the family they were taken over.
///
/// Split out of [`audit_bars`] to keep it under `clippy::too_many_lines`. It is
/// one idea: build one RETURN SERIES per candidate, then run every bootstrap
/// over that same family so the three answers are about the same set.
///
/// Returns `None` when the family is empty, which is the one state the callers
/// below must not read as "the tests passed".
///
/// **Runs on the SIGNAL series**, deliberately and unlike the trade and the exit
/// grid above. These are statements about how often a CONDITION precedes a move,
/// which is a property of the rung it was found on; re-taking them at one-minute
/// resolution would answer a question nobody asked and would not be comparable
/// with the sweep's own support figures.
fn bootstrap_family(
    bars: &[indicators::Candle],
    column: &indicators::column::Column,
    by_evidence: &[&runner::rank::Scored],
    horizon: Horizon,
) -> Option<(
    Option<runner::bootstrap::Verdict>,
    Option<runner::bootstrap::Verdict>,
    usize,
)> {
    // THE BOOTSTRAP, WHICH NEEDED A DIFFERENT SHAPE OF DATA FROM PBO.
    //
    // PBO needed one NUMBER per candidate. This needs one SERIES per candidate —
    // a return per period — because it resamples periods and asks whether the
    // best of the family beats what the same search finds on resampled data.
    // Comparing one strategy against itself answers nothing, so a family is
    // walked rather than the winner alone.
    //
    // Every series is bucketed into the SAME day index, so `bootstrap::aligned`
    // cannot refuse the set for a length mismatch — alignment is a property of
    // how this is built.
    let days = session_index(bars);
    // BUILT ONCE, PROBED PER TRADE. `session_returns` runs once per candidate
    // and each walks its own trades, so the index is hoisted here rather than
    // rebuilt inside — one pass over the sessions for the whole family.
    let index: std::collections::HashMap<i64, usize> = days
        .iter()
        .enumerate()
        .map(|(slot, day)| (*day, slot))
        .collect();
    // THE FAMILY THE REPORT ACTUALLY SELECTS FROM, and it did not used to be.
    //
    // White's Reality Check and Hansen's SPA are FAMILY-WISE tests: they ask
    // whether the best of a set beats what the same search finds on resampled
    // data, so the set has to be the set the search considered. This walked
    // `closed.kept` truncated to the first sixteen in canonical mask order —
    // which `crates/engine` states outright is not a ranking — while the
    // combination the report chose to trade came from a different rule entirely.
    // The p-value therefore described a family the reported strategy need not
    // even have belonged to.
    //
    // `by_evidence` is the same list the traded combination is drawn from, so
    // the head of the family and the strategy under test are now one thing.
    let family: Vec<Vec<i64>> = by_evidence
        .iter()
        .take(BOOTSTRAP_CANDIDATES)
        .map(|scored| {
            // EACH CANDIDATE ON ITS OWN SIDE, for the reason the traded
            // combination above is. Walking the whole family long would give a
            // short setup a return series that is the negative of what it would
            // have earned, so the bootstrap's null would be built from returns
            // no strategy in the family would ever have taken — and the p-value
            // beside it would describe that fiction rather than the family.
            let walked = trade::walk(
                bars,
                column,
                &scored.mask,
                horizon,
                direction_of(side_of_evidence(scored)),
            );
            session_returns(&index, days.len(), bars, &walked)
        })
        .collect();
    // ALL THREE TESTS, AND THE THIRD ONE NOW ACTUALLY RUNS.
    //
    // They answer different questions: Reality Check says "something in this
    // family is real", SPA says the same with poor strategies no longer diluting
    // the null, and Romano-Wolf is the per-strategy stepdown that says WHICH.
    //
    // This comment used to read "…so it is not run rather than run and
    // discarded", and that was true of the computation and false of the REPORT:
    // `audit::bootstrap` was handed `family.len()` for its `named` column and
    // printed, in words, "Only Romano-Wolf says WHICH, and it names 16" — the
    // size of the family offered, not the count of anything rejected. A reader
    // was told a stepdown had named sixteen strategies when no stepdown had been
    // computed at all. That is the failure-wearing-a-success's-clothes shape
    // `CLAUDE.md` §4 bans, in the one section of the report whose whole job is
    // to say how much of this is luck.
    //
    // The stated reason for not running it — "needs a decision about which
    // strategies to report" — is answered by the same alpha the two rows beside
    // it are already judged at, so the decision was available; it just had not
    // been made.
    let rc = runner::bootstrap::reality_check(
        &family,
        BOOTSTRAP_DRAWS,
        BOOTSTRAP_SEED,
        runner::bootstrap::DEFAULT_BLOCK,
    );
    let spa = runner::bootstrap::spa(
        &family,
        BOOTSTRAP_DRAWS,
        BOOTSTRAP_SEED,
        runner::bootstrap::DEFAULT_BLOCK,
    );
    // THE STEPDOWN, at the same 5% the two rows above are judged at, so one
    // report carries one alpha rather than two.
    let named = runner::bootstrap::romano_wolf(
        &family,
        BOOTSTRAP_DRAWS,
        BOOTSTRAP_SEED,
        runner::bootstrap::DEFAULT_BLOCK,
        BOOTSTRAP_ALPHA_PPM,
    )
    .len();
    let boot = (!family.is_empty()).then_some((rc.as_ref(), spa.as_ref(), named));
    let _ = boot;
    if family.is_empty() {
        return None;
    }
    Some((rc, spa, named))
}

/// The whole audit stack over bars the caller supplies, under a banner it names.
///
/// # Why this exists: the audit could only ever see invented data
///
/// Everything below — trades, worst-case fills, the exit grid, walk-forward,
/// PBO and the bootstrap — was reachable from exactly one command, `cli audit`,
/// whose bars came from `synthetic::sessions` **unconditionally**. There was no
/// `audit-stored`, so no entry point in this workspace had ever produced a
/// trade, a P&L, an exit grid, a walk-forward verdict, a PBO figure or a
/// bootstrap p-value **from real market data**. Every such number this system
/// could print described the generator.
///
/// `sweep-stored` could read a real month and rank it; it stopped short of
/// trading it. This is the join, and it is the same code on both sides — a
/// second implementation for real bars would be two backtests that could
/// disagree.
///
/// # The banner is a parameter, and that is load-bearing
///
/// A sweep over invented data is byte-identical in SHAPE to one over real data,
/// so the banner is the only thing separating them — which is why
/// `the_generated_and_stored_banners_make_opposite_claims` fails the build if
/// the two ever converge. Passing it in rather than deciding it here means the
/// caller that chose the bars is the caller that names their provenance, and the
/// two cannot drift apart.
///
/// # The identity is `Option`, for the reason `report::render` takes one
///
/// A synthetic run has no instrument to name, and naming one would be the
/// invention `CLAUDE.md` §3 rule 1 forbids — so it passes `None` and the report
/// says NOT RECORDED in place of a digest. A stored run has one and passes it.
#[allow(
    clippy::needless_pass_by_value,
    clippy::large_types_passed_by_value,
    reason = "the same ownership requirement `audit_with` documents: \
              `Column::build` and `Sweeper::run` both take `&mut`, and the \
              evaluator must outlive them."
)]
fn audit_bars(
    ev: Result<Evaluator, &'static str>,
    bars: Vec<indicators::Candle>,
    banner: &str,
    min_hits: u64,
    id: Option<&runner::identity::RunId>,
    execution: Option<Execution<'_>>,
    recording: Option<Recording<'_>>,
) -> String {
    let mut ev = match ev {
        Ok(e) => e,
        Err(why) => return format!("refused: {why}\n"),
    };
    let horizon = Horizon::DEFAULT;
    // ONE FOLD, NOT TWO, AND ONE EVALUATOR RATHER THAN A CALLER'S PLUS A
    // PRIVATE ONE. This was `Column::build(&bars, &mut ev)` followed by a second
    // `evaluator()` that the sweep used instead — so the `ev` parameter governed
    // the column and nothing else, and a caller passing custom widths would have
    // had masks discovered under one vocabulary indexing a column built under
    // another. `run_ranked` returns the column it measured on, so the sweep, the
    // ranking and the trade walk below cannot disagree about what they saw.
    let run = Sweeper::new(Ladder::with_min_hits(min_hits))
        .run_ranked(&bars, &mut ev, horizon, AUDIT_KEEP);
    let (outcome, ranked, column) = (run.outcome, run.ranked, run.column);

    // THE POSITION MOVES TO THE EXECUTION SERIES; THE SEARCH DOES NOT.
    //
    // The sweep above measured which conditions are FREQUENT, and frequency is a
    // property of the signal series -- "this fired on 4% of fifteen-minute bars"
    // is the statement, and re-counting it on one-minute bars would answer a
    // different question. So `run_ranked`, `render` and `render_findings` all
    // stay on `bars`.
    //
    // What moves is everything AFTER the decision. `trade::walk` and
    // `grid::evaluate` below take the projected column and the one-minute bars,
    // so the stop, the target and both trailing orders are checked at
    // one-minute resolution instead of at the signal rung's. `Horizon` is
    // counted in bars, so it also becomes MINUTES on every signal timeframe --
    // which is the only reading under which nine timeframes are comparable at
    // all.
    let (trade_bars, trade_column, execution_note) =
        match project_onto_execution(&bars, &column, execution, horizon) {
            Ok(triple) => triple,
            Err(why) => return format!("refused: {why}\n"),
        };

    let mut out = String::from(banner);
    out.push('\n');
    out.push_str(&execution_note);
    out.push_str(&runner::report::render(&outcome, id));
    // WHICH COMBINATIONS SURVIVED, BY NAME, before anything is traded. The
    // stored SWEEP has printed this since the ranker was wired; the audit did
    // not, so the one command that produces a P&L was the one that could not say
    // what the P&L was of.
    out.push_str(&runner::report::render_findings(&ranked, &outcome.sweep));

    // RANKED BY EVIDENCE, THEN FILTERED FOR REDUNDANCY. Computed once and used
    // twice: the head is the combination this audit trades, and the first
    // `BOOTSTRAP_CANDIDATES` of it are the family the bootstrap compares. Those
    // used to be two different sets — the trade took `closed.kept.first()` and
    // the bootstrap took `closed.kept`'s first sixteen, both in canonical mask
    // order — so the report's chosen strategy was not even a member of the
    // family whose p-value the report printed beside it.
    let by_evidence = closed_by_evidence(&ranked, &outcome.sweep);
    let Some(first) = by_evidence.first().copied() else {
        // TWO DIFFERENT FACTS WORE ONE SENTENCE, AND ONLY ONE OF THEM IS
        // EXTINCTION.
        //
        // This printed "This is extinction, not a failure" unconditionally.
        // `by_evidence` is the best AUDIT_KEEP by |t| intersected with the
        // closed set — so it can be empty either because the sweep genuinely
        // found nothing, or because none of the top 250 by evidence happened to
        // be closed on a sweep that found millions.
        //
        // `AUDIT_KEEP`'s own doc block predicts this failure in words — "the
        // audit reports extinction on a sweep that found plenty, a refusal that
        // would be a lie about the market rather than a fact about it" — and
        // the constant was raised to make it unlikely while the MESSAGE was
        // left unconditional. Guarding the cause and not the claim is how a
        // report ends up asserting something it cannot know.
        out.push_str(&nothing_to_trade(outcome.sweep.all_frequent().count()));
        return out;
    };
    out.push_str(&traded_line(first));
    out.push_str(&grid_exposure(&outcome.sweep));
    out.push_str(&sample_warning(session_index(&bars).len()));

    // THE SIDE IS READ OFF THE EVIDENCE, NOT ASSUMED.
    //
    // `rank` orders candidates by |t| — the ABSOLUTE value, deliberately, because
    // a combination that reliably precedes a FALL is as tradeable as one that
    // precedes a rise; only the side differs. `rank`'s own header says so, and
    // carries the sign in `Edge::mean_paisa` "for a reader to see".
    //
    // Nothing read it. This walked `Direction::Long` unconditionally, so the
    // combination with the strongest evidence of a DOWNWARD move was traded
    // long — and every figure below it, the P&L, the 125-cell grid, the
    // walk-forward, the PBO and all three bootstrap p-values, described the
    // wrong side of it.
    //
    // Worse, 6d849ee made it more likely to bite rather than less. Before that
    // commit the audit traded `closed.kept.first()`, an arbitrary combination in
    // canonical mask order, so its sign was incidental. Selecting for the
    // largest |t| selects precisely the strongest signals of EITHER sign — so
    // the better the ranker got, the more often the side was wrong.
    let side = side_of_evidence(first);
    let taken = trade::walk(
        &trade_bars,
        &trade_column,
        &first.mask,
        horizon,
        direction_of(side),
    );
    let exits = grid::evaluate(
        &trade_bars,
        &trade_column,
        &first.mask,
        horizon,
        side,
        GRID_RUNGS,
    );
    out.push('\n');
    // THE WALK-FORWARD, WHICH USED TO BE A `None`.
    //
    // `validate::walk_forward` was built, tested and never called: this report
    // printed "NOT SUPPLIED to this render" for it on every run since the
    // function existed. What it needed was a fold count, and
    // `WALK_FORWARD_SPLITS` supplies one as a STATED ASSUMPTION -- the form
    // `bootstrap::DEFAULT_BLOCK` and `validate::DEFAULT_RUNGS` already use.
    //
    // It re-sweeps once per fold, so it costs about `WALK_FORWARD_SPLITS` times
    // the sweep above. That is what an out-of-sample verdict costs, and it is
    // paid here rather than skipped.
    // A FRESH EVALUATOR PER FOLD, AND IT IS A COPY RATHER THAN A REBUILD.
    //
    // `walk_forward` wants `FnMut() -> Evaluator` because each fold must start
    // from an unwarmed detector -- a fold that inherited the previous fold's
    // state would be reading bars it was never given. `Evaluator` is `Copy`
    // (1,744 bytes, `docs/10-shared-core.md`), so one built here and copied per
    // fold is the same value a rebuild would produce, without a fallible call
    // inside a closure that has no way to report a refusal.
    let fresh = match evaluator() {
        Ok(e) => e,
        Err(why) => return format!("refused: {why}\n"),
    };
    let folds = runner::validate::walk_forward(
        &bars,
        horizon,
        WALK_FORWARD_SPLITS,
        Direction::Long,
        &Sweeper::new(Ladder::with_min_hits(min_hits)),
        move || fresh,
    );
    // PBO, WHICH USED TO BE A `None` FOR A REASON THAT IS NOW FIXED.
    //
    // `pbo::place` ranks a fold's candidates in-sample, finds where the winner
    // lands OUT of sample, and returns that placement. It needs both vectors,
    // and `FoldResult` kept only the chosen candidate until `in_sample_all` and
    // `out_of_sample_all` were added -- so the input did not exist to pass and no
    // constant could have supplied it.
    //
    // A fold that chose nothing yields no placement and is skipped rather than
    // counted as a failure: `place` returns `None` on an empty or mismatched
    // pair, and filtering is the honest reading of "this fold had nothing to
    // judge".
    let placements: Vec<_> = folds
        .folds
        .iter()
        .filter_map(|f| runner::pbo::place(&f.in_sample_all, &f.out_of_sample_all))
        .collect();
    let overfit =
        (!placements.is_empty()).then(|| runner::pbo::probability_of_overfitting(&placements));

    // The three multiple-testing p-values, over one family. See the helper: it
    // runs on the SIGNAL series on purpose, unlike the trade and the grid above.
    let boot_owned = bootstrap_family(&bars, &column, &by_evidence, horizon);
    let boot = boot_owned
        .as_ref()
        .map(|(rc, spa, named)| (rc.as_ref(), spa.as_ref(), *named));

    // RECORDED BEFORE IT IS RENDERED, so a process killed while formatting a
    // large report still leaves its row. The same ordering `cli.audit`'s
    // `audit rendered` event uses, and for the same reason.
    if let (Some(into), Some(run_id)) = (recording, id) {
        out.push_str(&record_run(
            into,
            run_id,
            &outcome,
            &exits,
            u64::try_from(bars.len()).unwrap_or(u64::MAX),
            min_hits,
        ));
    }
    out.push_str(&audit::render(
        Some(&taken),
        Some(&exits),
        Some(&folds),
        overfit.as_ref(),
        boot,
        12,
    ));
    out
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]
mod tests {
    use super::{Direction, Side};
    use super::{
        MIN_AUDIT_SESSIONS, MISUSED, OK, PROVENANCE, STORED_PROVENANCE, USAGE, Vendor, audit_run,
        audit_stored, auto, auto_with, direction_of, evaluator_from, log_dir_from,
        nothing_to_trade, parse_min_hits, parse_sessions, parse_vendor, root_from, run,
        sample_warning, side_of_evidence, sweep, sweep_stored, sweep_with,
    };

    fn argv(words: &[&str]) -> Vec<String> {
        words.iter().map(|w| (*w).to_owned()).collect()
    }

    /// THE ONE SENTENCE THIS BINARY MUST NEVER STOP PRINTING.
    ///
    /// A report over generated bars is byte-identical in shape to one over real
    /// bars, so the provenance line is the only thing standing between a reader
    /// and a number that means nothing. `CLAUDE.md` §4 bans a failure wearing a
    /// success's clothes; a confident sweep over invented data is exactly that,
    /// and this is the assertion that keeps it labelled.
    #[test]
    fn the_report_always_declares_its_bars_are_generated() {
        for text in [sweep(1, 1), auto(1)] {
            assert!(
                text.starts_with(PROVENANCE),
                "the provenance banner leads every render:\n{text}"
            );
            assert!(text.contains("not a backtest"), "and says so in words");
        }
    }

    #[test]
    fn a_valid_sweep_and_a_valid_auto_both_render_and_exit_zero() {
        let mut out = String::new();
        assert_eq!(run(&argv(&["sweep", "1", "1"]), &mut out), OK);
        assert!(out.contains("BARS"), "the report body rendered:\n{out}");

        let mut out = String::new();
        assert_eq!(run(&argv(&["auto", "1"]), &mut out), OK);
        assert!(!out.is_empty(), "auto rendered something");
    }

    /// Every refusal NAMES what was wrong and prints the usage.
    ///
    /// `CLAUDE.md` §4 requires a refusal to name its reason; "usage:" alone
    /// leaves the operator to work out which of their words was objected to.
    #[test]
    fn every_refusal_names_its_reason_and_prints_the_usage() {
        let cases: [(&[&str], &str); 7] = [
            (&[], "no command given"),
            (&["backtest"], "backtest"),
            (&["sweep", "0", "1"], "SESSIONS is outside"),
            (&["sweep", "3651", "1"], "SESSIONS is outside"),
            (&["sweep", "x", "1"], "SESSIONS is not a whole number"),
            (&["sweep", "1", "0"], "MIN_HITS must be 1 or more"),
            (&["sweep", "1", "x"], "MIN_HITS is not a whole number"),
        ];
        for (words, needle) in cases {
            let mut out = String::new();
            assert_eq!(
                run(&argv(words), &mut out),
                MISUSED,
                "{words:?} is a misuse, not a failure"
            );
            assert!(
                out.contains(needle),
                "{words:?} must be refused by name, expected {needle:?}:\n{out}"
            );
            assert!(
                out.contains(USAGE),
                "{words:?} must print the usage:\n{out}"
            );
        }
    }

    /// `auto` refuses a bad SESSIONS on its own arm, not only through `sweep`.
    #[test]
    fn auto_refuses_a_session_count_it_cannot_use() {
        let mut out = String::new();
        assert_eq!(run(&argv(&["auto", "0"]), &mut out), MISUSED);
        assert!(out.contains("SESSIONS is outside"), "named:\n{out}");
    }

    /// The wrong NUMBER of words is a misuse too, not a partial match.
    #[test]
    fn a_command_with_the_wrong_arity_is_refused_rather_than_guessed_at() {
        for words in [
            &["sweep"][..],
            &["sweep", "1"][..],
            &["sweep", "1", "1", "1"][..],
            &["auto"][..],
            &["auto", "1", "1"][..],
        ] {
            let mut out = String::new();
            assert_eq!(
                run(&argv(words), &mut out),
                MISUSED,
                "{words:?} does not match any command shape"
            );
        }
    }

    #[test]
    fn the_session_and_threshold_bounds_are_exactly_as_documented() {
        assert_eq!(parse_sessions("1"), Ok(1), "one session is the floor");
        assert_eq!(parse_sessions("3650"), Ok(3650), "ten years is the ceiling");
        assert!(parse_sessions("0").is_err());
        assert!(parse_sessions("3651").is_err());
        assert!(parse_sessions("-1").is_err());
        assert!(parse_sessions("").is_err());

        assert_eq!(parse_min_hits("1"), Ok(1));
        assert_eq!(parse_min_hits("18446744073709551615"), Ok(u64::MAX));
        assert!(
            parse_min_hits("0").is_err(),
            "zero would disable extinction, and the ladder's own clamp would \
             hide that the operator asked for it"
        );
        assert!(parse_min_hits("-1").is_err());
    }

    /// The refusal arm of the evaluator, which is why the widths are a parameter.
    #[test]
    fn absent_tolerances_are_refused_by_name_rather_than_defaulted() {
        assert!(evaluator_from(None).is_err(), "no widths, no evaluator");
        assert_eq!(
            evaluator_from(None).err(),
            Some("the pinned tolerances are not valid")
        );
        assert!(
            evaluator_from(indicators::evaluator::Widths::pinned().ok()).is_ok(),
            "and the shipped tolerances do build one"
        );
    }

    /// THE WHOLE INSTITUTIONAL STACK RENDERS — NOTHING IS NAMED ABSENT ANY MORE.
    ///
    /// # What this test used to assert, and why the change is the point
    ///
    /// It was `the_audit_names_the_stages_it_did_not_render`, and it asserted the
    /// report contained `NOT SUPPLIED` — because the walk-forward, PBO and the
    /// bootstrap were all passed as `None` and §4 requires an unsupplied stage to
    /// be NAMED rather than rendered as a zero. That was the right assertion for
    /// a build that could not drive them.
    ///
    /// All three are driven now, so the same assertion inverted is the honest
    /// one: every section must carry a real figure, and `NOT SUPPLIED` must not
    /// appear anywhere. A test still demanding the old string would be pinning a
    /// gap as though it were a feature.
    #[test]
    fn the_audit_renders_every_stage_of_the_institutional_stack() {
        let text = audit_run(12, 300);
        assert!(text.starts_with(PROVENANCE), "provenance leads it too");
        assert!(text.contains("BARS"), "the sweep report is still there");
        for section in [
            "TRADES",
            "EXIT GRID",
            "WALK-FORWARD",
            "OVERFITTING",
            "BOOTSTRAP",
        ] {
            assert!(
                text.contains(section),
                "{section} must be rendered, not skipped:\n{text}"
            );
        }
        assert!(
            !text.contains("NOT SUPPLIED"),
            "every stage is driven now, so nothing may be named absent:\n{text}"
        );
    }

    /// A threshold nothing can meet reaches the audit and says so plainly.
    #[test]
    fn an_audit_with_no_closed_combination_says_that_is_extinction() {
        let text = audit_run(1, u64::MAX);
        assert!(
            text.contains("nothing to trade"),
            "an empty answer must be distinguishable from an unmeasured one:\n{text}"
        );
        assert!(
            text.contains("extinction, not a failure"),
            "and must say which it is"
        );
    }

    /// A sweep whose threshold nothing can meet still renders, and says so.
    ///
    /// The failure this guards is the one `runner::Outcome` documents: an empty
    /// answer must be distinguishable from an unmeasured one.
    #[test]
    fn an_impossible_threshold_still_renders_a_report_rather_than_nothing() {
        let text = sweep(1, u64::MAX);
        assert!(text.starts_with(PROVENANCE));
        assert!(text.contains("BARS"), "the census is still shown:\n{text}");
    }

    /// AN EVALUATOR THAT CANNOT BE BUILT IS REFUSED BY NAME, NOT RENDERED EMPTY.
    ///
    /// `evaluator()` cannot fail in this build, so this arm is unreachable
    /// through the public entry points and is driven here directly. It exists
    /// because a later widening of the tolerance table would reach it, and a
    /// refusal that printed an empty report would be indistinguishable from a
    /// sweep that found nothing — the confusion `runner::Outcome` documents.
    #[test]
    fn a_sweep_without_an_evaluator_refuses_by_name_rather_than_rendering() {
        for text in [
            sweep_with(Err("the pinned tolerances are not valid"), 1, 1),
            auto_with(Err("the pinned tolerances are not valid"), 1),
        ] {
            assert!(text.starts_with("refused: "), "named a refusal: {text}");
            assert!(text.contains("pinned tolerances"), "and why: {text}");
            assert!(
                !text.contains("BARS"),
                "a refusal must NOT render a census that would read as a sweep \
                 finding nothing: {text}"
            );
        }
    }

    /// EVERY FEED WORD THE STORE CAN WRITE IS A WORD THIS COMMAND ACCEPTS.
    ///
    /// Derived from `Vendor::ALL` rather than listed, so a vendor appended to the
    /// enum is addressable here the same day. A hand-written list is exactly how a
    /// feed becomes unpullable from the command line while the store happily holds
    /// its bars — the shape `CLAUDE.md` §4 calls a fallback that hides a failure,
    /// because the refusal would name the word rather than the missing arm.
    #[test]
    fn every_feed_the_store_can_write_is_a_feed_this_command_accepts() {
        for v in Vendor::ALL {
            assert_eq!(
                parse_vendor(v.as_str()),
                Ok(v),
                "{} is a store prefix and must be addressable",
                v.as_str()
            );
        }
        // And an unknown word is refused by NAME, listing what would have worked.
        let why = parse_vendor("bogus").expect_err("`bogus` is not a feed");
        assert!(why.contains("bogus"), "the refusal names the word: {why}");
        for v in Vendor::ALL {
            assert!(
                why.contains(v.as_str()),
                "and lists {} as an alternative: {why}",
                v.as_str()
            );
        }
    }

    /// THE STORE ROOT HAS THREE OUTCOMES AND THE THIRD IS A REFUSAL, NOT A GUESS.
    ///
    /// A default of `./store` or of `/` would be a fallback that hides a failure:
    /// the sweep would open nothing, report zero bars, and read as an instrument
    /// with no history rather than as a machine with no `HOME`.
    #[test]
    fn the_store_root_prefers_the_override_and_refuses_when_it_has_neither() {
        assert_eq!(
            root_from(Some("/tmp/elsewhere".into()), Some("/Users/x".into())),
            Ok(std::path::PathBuf::from("/tmp/elsewhere")),
            "the explicit override wins over HOME"
        );
        assert_eq!(
            root_from(None, Some("/Users/x".into())),
            Ok(std::path::PathBuf::from("/Users/x/.brutex/store")),
            "and HOME is the fallback, at the workspace's own path"
        );
        let why = root_from(None, None).expect_err("neither is set");
        assert!(
            why.contains("BRUTEX_STORE"),
            "the refusal names both: {why}"
        );
        assert!(why.contains("HOME"), "the refusal names both: {why}");
    }

    /// A MONTH THE STORE DOES NOT HOLD IS A REFUSAL THAT SAYS WHAT TO DO NEXT.
    ///
    /// The operator's next action is a pull, not a filesystem check, so the
    /// refusal says so. It also must not render a census: a sweep of a month that
    /// was never pulled and a sweep that found nothing are different statements,
    /// and only one of them is about the market.
    #[test]
    fn a_month_the_store_does_not_hold_is_refused_and_renders_no_census() {
        let text = sweep_stored("groww", "NIFTY", "1min", 1970, 1, 1);
        assert!(text.starts_with("refused: "), "named a refusal: {text}");
        assert!(
            !text.contains("BARS"),
            "a refusal must not render a census that would read as an empty \
             market: {text}"
        );
    }

    /// THE SIDE FOLLOWS THE EVIDENCE, AND A FALL IS SOLD RATHER THAN BOUGHT.
    ///
    /// # The defect, and why the better ranker made it worse
    ///
    /// `crate::rank` orders by |t| on purpose — a combination that reliably
    /// precedes a FALL is as tradeable as one that precedes a rise, and ranking
    /// on a signed `t` would discard every short setup. The sign lives in
    /// `Edge::mean_paisa`, which `rank`'s header says is carried *"for a reader
    /// to see"*.
    ///
    /// Nothing read it. The audit walked `Direction::Long` unconditionally, so
    /// the strongest DOWNWARD signal was bought, and the P&L, the 125-cell grid,
    /// the walk-forward, the PBO and all three bootstrap p-values described the
    /// wrong side of it.
    ///
    /// And selecting for the largest |t| — which `6d849ee` introduced as an
    /// improvement over an arbitrary pick — selects precisely the strongest
    /// signals of EITHER sign. The better the ranker got, the more often the
    /// side was wrong.
    #[test]
    fn the_traded_side_follows_the_sign_of_the_evidence() {
        use runner::outcome::Edge;
        use runner::rank::Scored;

        let scored = |mean: f64| Scored {
            // `Default::default()` and not the named path, for the reason every
            // other mask literal in this crate gives: spelling `ConditionMask`
            // needs a `vocab` arrow §5 does not draw for `cli`.
            #[expect(
                clippy::default_trait_access,
                reason = "the named path would add a dependency arrow §5 does not draw"
            )]
            mask: Default::default(),
            hits: 100,
            edge: Edge {
                n: 100,
                mean_paisa: mean,
                mismatched: 0,
                refused: 0,
                t: mean,
            },
        };

        assert_eq!(
            side_of_evidence(&scored(250.0)),
            Side::Long,
            "a combination that precedes a RISE is bought"
        );
        assert_eq!(
            side_of_evidence(&scored(-250.0)),
            Side::Short,
            "a combination that precedes a FALL is SOLD — buying it is the \
             defect this test exists for, and it is the case a Long-only audit \
             got wrong on exactly half its strongest candidates"
        );

        // EXACTLY ZERO IS LONG, and it is a stated convention rather than an
        // accident: a zero mean gives t = 0, which cannot clear any bar, so
        // which way it is taken changes no verdict.
        assert_eq!(side_of_evidence(&scored(0.0)), Side::Long);

        // AND THE TWO ENUMS MAP CONSISTENTLY. Two types name one concept —
        // `Side` for excursions, `Direction` for fills — and a call site that
        // mapped them the wrong way round would price a long as a short with
        // nothing else in the report disagreeing.
        assert!(matches!(direction_of(Side::Long), Direction::Long));
        assert!(matches!(direction_of(Side::Short), Direction::Short));
    }

    /// "EXTINCTION" IS CLAIMED ONLY WHEN THE SWEEP ACTUALLY FOUND NOTHING.
    ///
    /// # Two facts wore one sentence
    ///
    /// The audit's no-trade branch printed *"This is extinction, not a
    /// failure"* unconditionally. Its input is the strongest [`AUDIT_KEEP`] by
    /// |t| intersected with the closed set, which is empty either because the
    /// sweep found nothing — genuine extinction, and §6's expected answer — or
    /// because none of the top 250 happened to be closed on a sweep that found
    /// millions.
    ///
    /// The second is not extinction, and claiming it is a statement about the
    /// market the code cannot support. [`AUDIT_KEEP`]'s own doc predicted this
    /// and the constant was raised to make it unlikely while the message was
    /// left covering both cases.
    #[test]
    fn extinction_is_claimed_only_when_nothing_was_found() {
        let extinct = nothing_to_trade(0);
        assert!(
            extinct.contains("extinction, not a failure"),
            "a genuinely empty sweep IS extinction and must say so: {extinct}"
        );

        let plenty = nothing_to_trade(3_689);
        assert!(
            plenty.contains("NOT extinction"),
            "a sweep that kept 3,689 combinations did not go extinct: {plenty}"
        );
        assert!(
            plenty.contains("3689") || plenty.contains("3,689"),
            "and the refusal names how many it did keep: {plenty}"
        );
        assert!(
            plenty.contains("Raise"),
            "§4 asks a refusal to say what to change, not merely to decline: \
             {plenty}"
        );
        assert!(
            !plenty.contains("extinction, not a failure"),
            "the two messages must not both fire — that is the defect: {plenty}"
        );
    }

    /// A THIN SAMPLE IS SAID SO, AND A SUFFICIENT ONE IS NOT NAGGED ABOUT.
    ///
    /// # The defect this closes, which is about an impression rather than a number
    ///
    /// `audit-stored` is scoped to one instrument-month — about twenty trading
    /// days. On that series the audit still runs a five-fold walk-forward, a PBO
    /// over the resulting placements, and three stationary-block bootstraps at a
    /// block length of ten. A draw is then one or two blocks and a fold tests on
    /// a handful of days.
    ///
    /// None of those figures was *wrong*. What was wrong is that they rendered
    /// in **exactly the same format** as figures computed over 3,650 generated
    /// sessions, with nothing on the page to tell a reader which they were
    /// holding. `CLAUDE.md` §3 rule 6 asks for an unmeetable bound to be said out
    /// loud; nothing said it.
    ///
    /// # Why the boundary is asserted from both sides
    ///
    /// A warning that always fires is noise a reader learns to skip, which makes
    /// it worse than none. So the sufficient case must be silent, and the empty
    /// string is asserted rather than assumed.
    #[test]
    fn a_thin_sample_is_named_and_a_sufficient_one_says_nothing() {
        // SUFFICIENT: silent, exactly at the boundary and above it.
        assert!(
            sample_warning(MIN_AUDIT_SESSIONS).is_empty(),
            "the boundary itself is sufficient; a warning here would fire on \
             every adequate run and teach the reader to ignore it"
        );
        assert!(sample_warning(MIN_AUDIT_SESSIONS + 1_000).is_empty());

        // THIN: named, with the two numbers that decide it.
        let thin = sample_warning(20);
        assert!(thin.contains("THIN"), "the verdict is stated: {thin}");
        assert!(
            thin.contains("sessions 20"),
            "and the sample it is a verdict about: {thin}"
        );
        assert!(
            thin.contains("p-values") && thin.contains("PBO"),
            "naming WHICH figures are affected is the whole point — a blanket \
             warning would also discredit the trades, which are fine: {thin}"
        );
        assert!(
            thin.contains("unaffected"),
            "and which figures are not affected, so the reader does not discard \
             the honest half: {thin}"
        );

        // ZERO SESSIONS MUST NOT PANIC. The divisors are constants here, but a
        // future change to either could make one zero, and this is the arm that
        // would catch a division by it.
        assert!(sample_warning(0).contains("THIN"));
    }

    /// THE LOG DIRECTORY IS DECIDED WITHOUT TOUCHING THE ENVIRONMENT.
    ///
    /// # Why the decision is split out from `install_log`
    ///
    /// `telemetry::install` writes a process-wide `OnceLock` and refuses a
    /// second call, so `install_log` can succeed at most once per test binary —
    /// a test that drove it would poison every later test in the same process,
    /// and which test won would depend on thread scheduling. `root_from` is
    /// split from `store_root` for the identical reason and says so.
    ///
    /// So the environment reading stays in `install_log`, which
    /// `tests/binary.rs` exercises by running the real binary, and the DECISION
    /// is tested here where it can be driven directly.
    #[test]
    fn the_log_directory_follows_the_store_unless_it_is_named_outright() {
        use std::ffi::OsString;
        use std::path::PathBuf;

        // AN EXPLICIT DIRECTORY WINS, and it wins even when a store exists —
        // otherwise an operator who set the variable would silently get the
        // store's `logs` instead of the path they named.
        assert_eq!(
            log_dir_from(
                Some(OsString::from("/tmp/brutex-events")),
                Some(PathBuf::from("/srv/store")),
            ),
            Some(PathBuf::from("/tmp/brutex-events")),
        );

        // WITH NO VARIABLE, THE LOG SITS UNDER THE STORE. Beneath the store and
        // not the working directory, because a sweep started from `/` or from a
        // read-only checkout must still write somewhere, and the store root is
        // already required to be writable.
        assert_eq!(
            log_dir_from(None, Some(PathBuf::from("/srv/store"))),
            Some(PathBuf::from("/srv/store/logs")),
        );

        // WITH NEITHER, THERE IS NOWHERE TO WRITE AND THAT IS `None`, not a
        // guess at the working directory. `install_log` turns this into the
        // printed "events are NOT being recorded" line rather than a silent
        // absence — degrade loudly, per §4.
        assert_eq!(log_dir_from(None, None), None);

        // AND AN EXPLICIT DIRECTORY STILL WINS WITH NO STORE AT ALL, which is
        // the case for an operator who has moved the store away entirely.
        assert_eq!(
            log_dir_from(Some(OsString::from("/var/log/brutex")), None),
            Some(PathBuf::from("/var/log/brutex")),
        );
    }

    /// THE REAL-DATA AUDIT REFUSES FOR THE SAME CAUSE AS THE SWEEP, AND NAMES ITSELF.
    ///
    /// # What this can reach, stated because it decides every assertion below
    ///
    /// `cargo test` builds without `BRUTEX_COMMIT`, so `commit_stamp()` is `None`
    /// and **both commands refuse at the commit gate before the store is touched
    /// at all**. That is correct — §3 rule 3 forbids a computation whose identity
    /// cannot be recorded, so the gate must come first — but it means the
    /// fixtures below do NOT exercise the vendor parse, the store root or the
    /// month lookup.
    ///
    /// Saying so matters. The audit that prompted `audit-stored` found exactly
    /// this illusion in `sweep-stored`'s own test: it "passes because the commit
    /// gate short-circuits before any of the code under test runs". A test that
    /// looks like it covers the store path and does not is worse than an absent
    /// one, so this asserts only what an unstamped build genuinely reaches.
    ///
    /// # Why identical wording is the WRONG property to assert
    ///
    /// The two refusals differ by one word — "the sweep will not run" against
    /// "the audit will not run" — and that difference is deliberate. §4 requires
    /// a refusal to say what was refused, and an operator who ran `audit-stored`
    /// should not be told about a sweep. An earlier version of this test asserted
    /// string equality and failed on precisely that improvement, which is a test
    /// pinning a defect. So the assertions are on the CAUSE and on the
    /// self-naming, both of which stay true as the wording changes.
    #[test]
    fn the_stored_audit_refuses_for_the_same_cause_and_names_itself() {
        for (vendor, underlying, rung, year, month) in [
            // A month no store holds.
            ("groww", "NIFTY", "1min", 1970_u16, 1_u8),
            // A feed word no build knows.
            ("nosuchfeed", "NIFTY", "1min", 2026, 8),
        ] {
            let swept = sweep_stored(vendor, underlying, rung, year, month, 500);
            let audited = audit_stored(vendor, underlying, rung, year, month, 500);

            for (label, text) in [("sweep-stored", &swept), ("audit-stored", &audited)] {
                assert!(
                    text.starts_with("refused: "),
                    "{label} must refuse ({vendor}, {underlying}, {rung}, \
                     {year}-{month}), or this test proves nothing: {text}"
                );
                assert!(
                    !text.contains("BARS"),
                    "{label}: a refusal must not render a census that would read \
                     as an empty market: {text}"
                );
            }

            // THE SAME CAUSE. On this unstamped build that cause is the commit
            // gate, and both must cite it — an `audit-stored` that reached the
            // store first would compute before it could record an identity.
            assert!(
                swept.contains("BRUTEX_COMMIT") && audited.contains("BRUTEX_COMMIT"),
                "both stored commands must refuse at the commit gate before \
                 reading a bar:\n  sweep: {swept}\n  audit: {audited}"
            );

            // AND EACH NAMES ITSELF. This is the half that would silently rot if
            // the two messages were ever merged into one shared constant, and it
            // is why this test does NOT assert the two strings are equal.
            assert!(
                swept.contains("the sweep will not run"),
                "the sweep's refusal must name the sweep: {swept}"
            );
            assert!(
                audited.contains("the audit will not run"),
                "the audit's refusal must name the audit, not the sweep: {audited}"
            );
        }
    }

    /// AND IT IS REACHABLE FROM THE COMMAND LINE, not merely defined.
    ///
    /// `cli` dispatches on a hand-written slice match rather than a derive, so a
    /// function can exist, compile, be tested directly, and still be
    /// unreachable because no arm names it — which is the shape of the
    /// unreachability D-0169 was written to close. This drives `run` by its
    /// argv, so the arm itself is what is under test.
    #[test]
    fn the_stored_audit_is_reachable_from_argv_and_is_in_the_usage() {
        let mut out = String::new();
        let code = run(
            &argv(&["audit-stored", "groww", "NIFTY", "1min", "1970", "1", "500"]),
            &mut out,
        );
        assert_eq!(
            code, MISUSED,
            "a month the store does not hold is a misuse, not a success: {out}"
        );
        assert!(
            out.starts_with("refused: "),
            "the arm reached the command rather than falling through to \
             `not a command this build knows`: {out}"
        );
        assert!(
            USAGE.contains("audit-stored"),
            "a command an operator cannot discover is a command that does not \
             exist for them"
        );
        // THE FEED THE PARSER ACCEPTS AND THE USAGE OMITTED. `parse_vendor`
        // takes `zerodha`; the usage listed four feeds and not that one, so an
        // operator with zerodha bars on disk would read it and conclude the
        // store could not be swept for them.
        assert!(
            USAGE.contains("zerodha"),
            "every feed the parser accepts must appear in the usage"
        );
    }

    /// THE THREE NUMBERS ARE PARSED SEPARATELY AND EACH NAMES ITSELF WHEN WRONG.
    ///
    /// One shared "bad arguments" message would leave an operator comparing six
    /// words against six meanings to find which one this build objected to.
    #[test]
    fn each_numeric_argument_of_the_stored_sweep_refuses_in_its_own_words() {
        let cases = [
            (
                ["sweep-stored", "groww", "NIFTY", "1min", "YEAR", "8", "500"],
                "YEAR",
            ),
            (
                [
                    "sweep-stored",
                    "groww",
                    "NIFTY",
                    "1min",
                    "2026",
                    "MONTH",
                    "500",
                ],
                "MONTH",
            ),
            (
                [
                    "sweep-stored",
                    "groww",
                    "NIFTY",
                    "1min",
                    "2026",
                    "8",
                    "HITS",
                ],
                "MIN_HITS",
            ),
        ];
        for (words, wanted) in cases {
            let args: Vec<String> = words.iter().map(|w| (*w).to_owned()).collect();
            let mut out = String::new();
            let code = run(&args, &mut out);
            assert_eq!(code, MISUSED, "a misparsed argument is a misuse: {out}");
            assert!(
                out.contains(wanted),
                "the refusal must name {wanted}, not the other two: {out}"
            );
            assert!(out.contains("usage:"), "and print the usage: {out}");
        }
    }

    /// THE USAGE NAMES THE COMMAND, OR AN OPERATOR CANNOT FIND IT.
    ///
    /// `cli` has no `--help`; the usage IS the help, printed on every refusal. A
    /// command absent from it is a command nobody discovers.
    #[test]
    fn the_stored_sweep_appears_in_the_usage_with_its_arguments() {
        assert!(USAGE.contains("sweep-stored"), "the command is listed");
        for word in ["VENDOR", "UNDERLYING", "RUNG", "YEAR", "MONTH"] {
            assert!(USAGE.contains(word), "{word} is explained in the usage");
        }
    }

    /// THE TWO PROVENANCE BANNERS CANNOT BE MISTAKEN FOR ONE ANOTHER.
    ///
    /// A sweep over generated bars and one over real bars are byte-identical in
    /// shape, so the banner is the only thing separating them. If both said the
    /// same words, a generated run could be read as evidence about a market --
    /// the failure wearing a success's clothes that `CLAUDE.md` §5 bans.
    #[test]
    fn the_generated_and_stored_banners_make_opposite_claims() {
        assert!(PROVENANCE.contains("GENERATED"), "one says generated");
        assert!(
            STORED_PROVENANCE.contains("REAL MARKET DATA"),
            "one says real"
        );
        assert_ne!(PROVENANCE, STORED_PROVENANCE);
        assert!(
            !STORED_PROVENANCE.contains("not a backtest"),
            "the real banner must not carry the generated one's disclaimer"
        );
    }

    /// AND THE SEARCH ACTUALLY FINISHES, ON A COLUMN WITH BARS TO SWEEP.
    ///
    /// The const assertion beside `SEARCH_CEILING` is necessary and not
    /// sufficient: a ceiling low enough to probe cheaply is worthless if the rung
    /// it settles on cannot then be walked. This asserts the whole command returns
    /// a COMPLETE sweep, which `runner::Auto` refuses to do when every probe it
    /// tried was itself refused.
    #[test]
    fn the_threshold_search_returns_a_complete_sweep_rather_than_a_refusal() {
        let text = auto(6);
        assert!(
            !text.starts_with("refused: "),
            "the search refused a column it should tune: {text}"
        );
        assert!(
            text.contains("complete"),
            "a search that settles on a rung it cannot walk has not searched: {text}"
        );
    }

    /// THE RANGE COMMAND REFUSES A BAD ARGUMENT BEFORE IT REACHES THE STORE.
    ///
    /// # What an unstamped build can reach here, and what it cannot
    ///
    /// The four number parses in `audit_range_arm` run BEFORE `audit_range` is
    /// called, so they are reachable under `cargo test` even though the commit
    /// gate inside refuses immediately after — the same split
    /// `the_stored_audit_refuses_for_the_same_cause_and_names_itself` documents
    /// for `audit-stored`. These assertions therefore cover the parse arms and
    /// claim nothing about the store walk.
    #[test]
    fn the_range_command_names_which_argument_it_refused() {
        // 300 AND NOT 13, AND THE DIFFERENCE IS THE POINT. `MONTH` parses as
        // `u8`, so 13 succeeds here and the call reaches `audit_range`, which
        // refuses at the COMMIT GATE first -- the documented limit an unstamped
        // `cargo test` build always hits. Only a value that cannot be a `u8` at
        // all exercises the parse arm from this side. The 1..=12 guard itself is
        // covered directly by
        // `stored::a_month_outside_one_to_twelve_is_refused_and_never_reported_as_missing`.
        for (args, want) in [
            (
                [
                    "audit-range",
                    "zerodha",
                    "NIFTY",
                    "1min",
                    "x",
                    "1",
                    "2026",
                    "8",
                    "500",
                ],
                "YEAR must be a number",
            ),
            (
                [
                    "audit-range",
                    "zerodha",
                    "NIFTY",
                    "1min",
                    "2019",
                    "12",
                    "y",
                    "8",
                    "500",
                ],
                "YEAR must be a number",
            ),
            (
                [
                    "audit-range",
                    "zerodha",
                    "NIFTY",
                    "1min",
                    "2019",
                    "300",
                    "2026",
                    "8",
                    "500",
                ],
                "MONTH must be 1..=12",
            ),
            (
                [
                    "audit-range",
                    "zerodha",
                    "NIFTY",
                    "1min",
                    "2019",
                    "12",
                    "2026",
                    "300",
                    "500",
                ],
                "MONTH must be 1..=12",
            ),
            (
                [
                    "audit-range",
                    "zerodha",
                    "NIFTY",
                    "1min",
                    "2019",
                    "12",
                    "2026",
                    "8",
                    "0",
                ],
                "MIN_HITS must be 1 or more",
            ),
        ] {
            let mut out = String::new();
            let code = run(&argv(&args), &mut out);
            assert_eq!(code, MISUSED, "a bad argument exits MISUSED: {out}");
            assert!(
                out.contains(want),
                "the refusal must name the argument it rejected.\nwanted: \
                 {want}\ngot: {out}"
            );
        }
    }

    /// A well-formed range still refuses, and for the identity reason.
    ///
    /// This is the pair to the test above: the arguments are all valid, so the
    /// parse arms pass and the call reaches `audit_range`, which refuses at the
    /// commit gate before touching the store. That gate is §3 rule 3 and it must
    /// come FIRST — a computation whose identity cannot be recorded may not run,
    /// so the refusal here is the correct behaviour and not a gap.
    #[test]
    fn a_well_formed_range_refuses_at_the_identity_gate_and_says_so() {
        let mut out = String::new();
        let code = run(
            &argv(&[
                "audit-range",
                "zerodha",
                "NIFTY",
                "1min",
                "2019",
                "12",
                "2026",
                "8",
                "500",
            ]),
            &mut out,
        );
        assert_eq!(code, MISUSED);
        assert!(
            out.contains("commit stamp"),
            "an unstamped build must refuse by naming the stamp, not by \
             failing to find a file: {out}"
        );
        assert!(
            out.contains("the audit will not run"),
            "and it must say which command it refused: {out}"
        );
    }

    /// The usage text lists the range command, so an operator can find it.
    #[test]
    fn the_range_command_is_listed_in_usage() {
        assert!(USAGE.contains("audit-range"), "the command is listed");
        assert!(
            USAGE.contains("CONTIGUOUS SPAN"),
            "and usage says what makes it different from `audit-stored`"
        );
    }

    /// A COMPLETED RANGE RUN LEAVES A ROW, and a rerun does not leave a second.
    ///
    /// # What this can reach under `cargo test`, stated because it bounds the
    /// # assertion
    ///
    /// `commit_stamp()` is `option_env!`, so an unstamped test build refuses at
    /// the identity gate before the store is touched -- the limit
    /// `the_stored_audit_refuses_for_the_same_cause_and_names_itself` documents.
    /// So this cannot drive `audit-range` end to end.
    ///
    /// What it CAN do is drive the results store through the same calls
    /// `record_run` makes, which is where every defect in that path would live:
    /// the codec, the addressing, the duplicate rule and the reopen. The wiring
    /// itself is one call and is verified by measurement on a stamped build,
    /// recorded in `docs/05-decisions.md`.
    #[test]
    fn a_recorded_run_is_addressable_and_a_rerun_adds_nothing() {
        let root = std::env::temp_dir().join("brutex-wire-test");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("a temp root");

        let record = crate::results::Record {
            identity: [42; 32],
            finished_micros: 1_785_727_500_000_000,
            feed: crate::results::field("zerodha"),
            underlying: crate::results::field("NIFTY"),
            timeframe: crate::results::field("15min"),
            from_year: 2019,
            from_month: 12,
            to_year: 2026,
            to_month: 8,
            months_asked: 81,
            months_found: 81,
            bars: 43_875,
            min_hits: 200,
            combinations: 54_895_691,
            depth: 11,
            halted: 1,
            trades: 412,
            pessimistic: -987_654_321,
            optimistic: 123_456_789,
            worst_trade: -87_654_321,
            max_drawdown: -76_543_210,
            winner_mae: 2_291,
            winner_mfe: 8_876,
            all_mae: 2_295,
            exit_rungs: [2, 3, -1, 1, 0],
        };

        let mut store = crate::results::Results::open(&root).expect("the store opens");
        let row = store.append(&record).expect("the run is recorded");
        assert_eq!(row, 0, "the first run is row zero");

        // ADDRESSABLE, which is the whole reason for a fixed stride.
        let back = store.read(row).expect("the row reads back");
        assert_eq!(back, record, "every field survived the write and the read");
        assert_eq!(crate::results::read_field(&back.underlying), "NIFTY");
        assert_eq!(back.halted, 1, "a truncated search stays flagged as one");

        // A RERUN ADDS NOTHING. §3 rule 5 makes it byte-identical.
        assert!(
            store.append(&record).is_err(),
            "the same identity must not be recorded twice"
        );
        assert_eq!(store.len().expect("measurable"), 1);

        let _ = std::fs::remove_dir_all(&root);
    }
}

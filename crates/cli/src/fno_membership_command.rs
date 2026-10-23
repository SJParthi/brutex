//! `cli fno-membership build|show|bench`: the point-in-time F&O membership of
//! `pull::fno_membership` (D-5160 to D-5168, PIT-1).
//!
//! # What `build` reads and writes
//!
//! It reads the GDFL tick store's options day files READ-ONLY, from
//! `BRUTEX_GDFL_TICKSTORE` (the same variable `gdfl-import tickstore` reads,
//! with no default): one day file's index per day and no tick row. It writes
//! month files into `--out` and nothing else, and only months that gained a
//! record: a month whose current generation already holds the same bytes is
//! not written again.
//!
//! `build --out DIR` RESUMES: after the last sealed record in `DIR` (or from
//! the IST calendar's first day when `DIR` holds none) to the calendar's last
//! day, sealing records up to `known_to`, the last day holding a stored
//! stock-derivative record. Nothing about the window is typed (operator, 16
//! Oct 17:15 IST, "NOTHING TYPED"). `build FROM TO --out DIR` is a typed
//! window for synthetic stores and scratch copies ONLY: it is refused when
//! `DIR` lies inside the operator's store (`$BRUTEX_STORE`, else
//! `$HOME/.brutex/store`) or inside the `--store ROOT` on the same line, when
//! `DIR` is laid out as a store's own (`reference/fno-membership-v1`) wherever
//! it is (D-5169), and, everywhere, when it would overlap or leave a gap after
//! a sealed record. `FROM` may be `resume`. A day the source refuses stops the
//! build after everything before it is installed, exit non-zero, and the next
//! build resumes at it (D-5168).
//!
//! # What `show` and `bench` print
//!
//! `show DAY` prints the day's status; for a known day its pool size and
//! members in series order (series id, the spelling filed that day, `*` for a
//! share of today's 2026-08-01 snapshot), the members that listed no file, and
//! the names that differ from `brutex_core::universe::FNO_UNDERLYINGS` (which
//! nothing here edits). A day without an answer prints "membership unknown"
//! with its reason and exits non-zero; a directory that is not there is refused
//! by name. `bench` times `members`, `pool_size` and `is_member` once per known
//! day and `members` once per refused day (inside the span, and the 400 days
//! either side of it), sequentially, and prints n, p50, p99 and max of each
//! (`CLAUDE.md` §3 rule 6: a measurement, not an argument).
//!
//! # `--store ROOT`: the census as a dated cross-check
//!
//! With `--store ROOT`, `build` also lists the store's GDFL NSE option-underlying
//! and share-series folders (`ROOT/bars/gdfl/NSE/FNO`, `.../CASH`), READ ONLY,
//! and prints their counts, the membership series whose newest spelling has no
//! share folder, and the option folders no membership spelling reaches; `show`
//! names the day's members with no share folder under their spelling that day.
//! The 13 Oct census figures are never constants or assertions here (operator,
//! 16 Oct 17:15 IST, "NOTHING TYPED" item 3; D-5162 item 5): they are read
//! from the store each time. A `ROOT` without that tree is refused before
//! anything is built or read.

use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use brutex_core::instrument::{Exchange, Segment};
use brutex_core::universe::{FNO_INDEX_UNDERLYINGS, FNO_UNDERLYINGS};
use brutex_core::vendor::Vendor;
use pull::fno_membership::{
    DIR_NAME, DayStatus, Installed, Membership, MonthBuilt, SOURCE_LABEL, Window,
    build as build_dir,
};
use pull::gdfl_nfo::NfoTickStore;
use pull::session::Day;

use crate::gdfl_import_command::TICKSTORE_VAR;
use crate::store_write::{ALLOW_UNSTAMPED_VAR, unstamped_write_refusal};
use crate::{FAILED, MISUSED, OK};

/// The telemetry target of every event here.
const TARGET: &str = "cli.fno_membership";
/// Readers of the tick store at once unless `--workers` says otherwise: the
/// operator's ceiling for a read-only run over the real stores.
const DEFAULT_WORKERS: usize = 4;
/// The most `--workers` may ask: the machine's cores.
const MAX_WORKERS: usize = 14;
/// Days `bench` times either side of the span, so the refusals outside it are
/// measured as well as the answers inside it (n >= 200 each side).
const BENCH_MARGIN: u32 = 400;

/// The verb, reading the process environment.
pub(crate) fn command(args: &[&str], out: &mut String) -> u8 {
    command_with(
        args,
        &|var| std::env::var_os(var),
        crate::commit_stamp(),
        out,
    )
}

/// The verb with its environment and commit stamp passed in, so every arm is
/// driven by a test.
pub(crate) fn command_with(
    args: &[&str],
    env: &dyn Fn(&str) -> Option<OsString>,
    commit: Option<&str>,
    out: &mut String,
) -> u8 {
    let parsed = match args {
        ["build", rest @ ..] if rest.first().is_none_or(|word| word.starts_with("--")) => {
            flags(rest).map(|flags| Verb::Build(None, flags))
        }
        ["build", from, to, rest @ ..] => {
            flags(rest).map(|flags| Verb::Build(Some((from, to)), flags))
        }
        ["show", day, rest @ ..] => flags(rest).map(|flags| Verb::Show(day, flags)),
        ["bench", rest @ ..] => flags(rest).map(Verb::Bench),
        _ => Err(
            "fno-membership takes `build --out DIR [--workers N] [--store ROOT]`, `build FROM \
             TO --out DIR [--workers N] [--store ROOT]` (scratch only), `show DAY --out DIR \
             [--store ROOT]` or `bench --out DIR`"
                .to_owned(),
        ),
    };
    let verb = match parsed.and_then(Verb::checked) {
        Ok(verb) => verb,
        Err(why) => return refuse(out, &why),
    };
    match verb {
        Verb::Build(typed, flags) => build(typed, &flags, (env, commit), out),
        Verb::Show(day, flags) => show(day, &flags, out),
        Verb::Bench(flags) => bench(&flags.dir, out),
    }
}

/// One parsed invocation.
enum Verb<'a> {
    /// `build [FROM TO]`.
    Build(Option<(&'a str, &'a str)>, Flags),
    /// `show DAY`.
    Show(&'a str, Flags),
    /// `bench`.
    Bench(Flags),
}

impl Verb<'_> {
    /// The invocation, or the refusal of a `--store` that is not a store's
    /// GDFL NSE tree (refused before anything is built or read).
    fn checked(self) -> Result<Self, String> {
        let store = match &self {
            Self::Build(_, flags) | Self::Show(_, flags) => flags.store.as_deref(),
            Self::Bench(flags) => {
                return match &flags.store {
                    Some(_) => Err("bench takes no --store".to_owned()),
                    None => Ok(self),
                };
            }
        };
        match store.map(gdfl_nse) {
            Some(tree) if !tree.is_dir() => Err(format!(
                "--store {}: {} is not a directory; the cross-check reads a store's GDFL NSE \
                 folders",
                store.map_or_else(String::new, |root| root.display().to_string()),
                tree.display()
            )),
            _ => Ok(self),
        }
    }
}

/// Writes `refused: why` and returns [`MISUSED`].
fn refuse(out: &mut String, why: &str) -> u8 {
    let _ = writeln!(out, "refused: {why}");
    MISUSED
}

/// The flags of every form.
struct Flags {
    /// `--out DIR` (required): where the month files are.
    dir: PathBuf,
    /// `--workers N` (optional).
    workers: usize,
    /// `--store ROOT` (optional): a store whose GDFL folders are listed as a
    /// dated cross-check, read only.
    store: Option<PathBuf>,
}

/// `--out DIR` (required), `--workers N` and `--store ROOT` (optional).
fn flags(rest: &[&str]) -> Result<Flags, String> {
    let mut dir = None;
    let mut store = None;
    let mut workers = DEFAULT_WORKERS;
    let mut words = rest.iter();
    while let Some(word) = words.next() {
        match *word {
            "--out" => dir = words.next().map(PathBuf::from),
            "--store" => {
                store = Some(
                    words
                        .next()
                        .map(PathBuf::from)
                        .ok_or("--store takes a store root")?,
                );
            }
            "--workers" => {
                workers = words
                    .next()
                    .and_then(|text| text.parse().ok())
                    .filter(|n| (1..=MAX_WORKERS).contains(n))
                    .ok_or(format!("--workers takes 1..={MAX_WORKERS}"))?;
            }
            other => return Err(format!("{other:?} is not a flag of fno-membership")),
        }
    }
    let dir = dir.ok_or("--out DIR is required: it names where the month files are")?;
    Ok(Flags {
        dir,
        workers,
        store,
    })
}

/// `<store>/bars/gdfl/NSE`: the GDFL tree whose folders the cross-check lists.
fn gdfl_nse(store: &Path) -> PathBuf {
    store
        .join("bars")
        .join(Vendor::Gdfl.as_str())
        .join(Exchange::Nse.as_str())
}

/// The names of the folders of `store`'s GDFL NSE `segment`, read once.
fn folders(store: &Path, segment: Segment) -> Result<BTreeSet<String>, String> {
    let dir = gdfl_nse(store).join(segment.as_str());
    let named = |why: std::io::Error| format!("{}: {why}", dir.display());
    let mut found = BTreeSet::new();
    for entry in std::fs::read_dir(&dir).map_err(named)? {
        let entry = entry.map_err(named)?;
        if entry.file_type().map_err(named)?.is_dir() {
            found.insert(entry.file_name().to_string_lossy().into_owned());
        }
    }
    Ok(found)
}

/// The census counts as a DATED CROSS-CHECK, read from `store` now and never
/// used as a constant or an assertion (operator, 16 Oct 17:15 IST, "NOTHING
/// TYPED" item 3; D-5162 item 5): the GDFL option-underlying and share-series
/// folders, the membership's series whose newest spelling has no share
/// folder, and the option folders no membership spelling reaches.
fn cross_check(out: &mut String, held: &Membership, store: &Path) -> Result<(), String> {
    let (options, cash) = (
        folders(store, Segment::Fno)?,
        folders(store, Segment::Cash)?,
    );
    let newest: Vec<String> = (0..held.series_count())
        .filter_map(|id| u32::try_from(id).ok())
        .filter_map(|id| held.series(id).map(|spelling| spelling.name.to_string()))
        .collect();
    let no_cash: Vec<&str> = newest
        .iter()
        .filter(|name| !cash.contains(name.as_str()))
        .map(String::as_str)
        .collect();
    let unreached: Vec<&str> = options
        .iter()
        .filter(|name| matches!(held.series_of(name), Ok(None)))
        .map(String::as_str)
        .collect();
    let _ = writeln!(
        out,
        "cross-check of {} (read now; a dated cross-check, never a constant or an assertion, \
         D-5162 item 5): {} GDFL option-underlying folders, {} GDFL share-series folders; {} \
         membership series",
        store.display(),
        options.len(),
        cash.len(),
        newest.len()
    );
    let _ = writeln!(
        out,
        "  membership series whose newest spelling has no GDFL share folder ({}): {}",
        no_cash.len(),
        no_cash.join(" ")
    );
    let _ = writeln!(
        out,
        "  share option folders no membership spelling reaches ({}): {}",
        unreached.len(),
        unreached.join(" ")
    );
    Ok(())
}

/// `YYYY-MM-DD`.
fn day_of(text: &str) -> Result<Day, String> {
    let bad = || format!("{text:?} is not a day (YYYY-MM-DD)");
    let mut parts = text.splitn(3, '-');
    let mut next = || parts.next().ok_or_else(bad);
    let (year, month, day) = (next()?, next()?, next()?);
    let day = Day::new(
        year.parse().map_err(|_| bad())?,
        month.parse().map_err(|_| bad())?,
        day.parse().map_err(|_| bad())?,
    )
    .map_err(|_| bad())?;
    (day.to_string() == text).then_some(day).ok_or_else(bad)
}

/// `path` with its longest existing ancestor resolved, so a directory not yet
/// made compares like the one it will be.
fn resolved(path: &Path) -> PathBuf {
    let mut tail = Vec::new();
    let mut at = path;
    loop {
        if let Ok(real) = at.canonicalize() {
            return tail.iter().rev().fold(real, |acc, part| acc.join(part));
        }
        match (at.parent(), at.file_name()) {
            (Some(parent), Some(name)) => {
                tail.push(name.to_owned());
                at = parent;
            }
            _ => return path.to_path_buf(),
        }
    }
}

/// The operator's store root as the cli resolves it: `$BRUTEX_STORE`, else
/// `$HOME/.brutex/store`.
fn operator_store(env: &dyn Fn(&str) -> Option<OsString>) -> Option<PathBuf> {
    match env("BRUTEX_STORE") {
        Some(root) if !root.is_empty() => Some(PathBuf::from(root)),
        _ => env("HOME")
            .filter(|home| !home.is_empty())
            .map(|home| PathBuf::from(home).join(".brutex").join("store")),
    }
}

/// The typed window, or the refusal of one aimed at a store: inside the
/// operator's store (`$BRUTEX_STORE`, else `$HOME/.brutex/store`), inside the
/// `--store ROOT` named on the same line, or at a directory laid out as a
/// store's own membership (`reference/fno-membership-v1`) wherever it is.
fn typed_window(
    (from, to): (&str, &str),
    dir: &Path,
    named: Option<&Path>,
    env: &dyn Fn(&str) -> Option<OsString>,
) -> Result<Window, String> {
    let from = if from == "resume" {
        None
    } else {
        Some(day_of(from)?)
    };
    let to = day_of(to)?;
    let real = resolved(dir);
    let laid_out = real.file_name().is_some_and(|name| name == DIR_NAME)
        && real
            .parent()
            .and_then(Path::file_name)
            .is_some_and(|name| name == "reference");
    if laid_out {
        return Err(format!(
            "{} is laid out as a store's membership (reference/{DIR_NAME}): a typed window is \
             for synthetic stores and scratch copies only; build it with `build --out DIR`, \
             which resumes after the last sealed record and runs to known_to",
            dir.display()
        ));
    }
    if let Some(store) = operator_store(env)
        .into_iter()
        .chain(named.map(Path::to_path_buf))
        .find(|store| real.starts_with(resolved(store)))
    {
        return Err(format!(
            "{} is inside the store {}: a typed window is for synthetic stores and scratch \
             copies only; the store's membership is built by `build --out DIR`, which resumes \
             after the last sealed record and runs to known_to",
            dir.display(),
            store.display()
        ));
    }
    Ok(Window::Typed { from, to })
}

/// How a window reads in the report.
fn window_word(window: Window) -> String {
    match window {
        Window::Resume => "resume to known_to".to_owned(),
        Window::Typed { from, to } => format!(
            "typed window {}..={to}, scratch only",
            from.map_or_else(|| "resume".to_owned(), |day| day.to_string())
        ),
    }
}

/// How the verb reads its environment: a test hands it a table.
type Lookup<'a> = &'a dyn Fn(&str) -> Option<OsString>;

/// `build [FROM TO] --out DIR [--workers N] [--store ROOT]`.
fn build(
    typed: Option<(&str, &str)>,
    flags: &Flags,
    (env, commit): (Lookup<'_>, Option<&str>),
    out: &mut String,
) -> u8 {
    let (dir, workers) = (flags.dir.as_path(), flags.workers);
    let window = match typed.map(|typed| typed_window(typed, dir, flags.store.as_deref(), env)) {
        None => Window::Resume,
        Some(Ok(window)) => window,
        Some(Err(why)) => return refuse(out, &why),
    };
    if let Some(why) = unstamped_write_refusal(commit, env(ALLOW_UNSTAMPED_VAR).as_deref()) {
        return refuse(out, &why);
    }
    let ticks = match tick_store(env) {
        Ok(ticks) => ticks,
        Err(why) => return refuse(out, &why),
    };
    let source = NfoTickStore::new(&ticks);
    let _ = writeln!(
        out,
        "fno-membership build ({}) from {} into {}, {workers} readers, commit {}; source {}, \
         D-5160",
        window_word(window),
        ticks.display(),
        dir.display(),
        commit.unwrap_or("unstamped"),
        SOURCE_LABEL,
    );
    let run = Instant::now();
    let mut month_millis: Vec<u128> = Vec::new();
    let built = build_dir(&source, dir, window, workers, &mut |month| {
        month_millis.push(month.millis);
        report_month(out, month);
    });
    let built = match built {
        Ok(built) => built,
        Err(why) => {
            let _ = writeln!(out, "refused: {why}");
            let _noted = telemetry::emit(
                &telemetry::Event::error(TARGET, "membership build refused")
                    .with("dir", dir.display().to_string().as_str())
                    .with("why", why.to_string().as_str()),
            );
            return FAILED;
        }
    };
    report_edges(out, &built);
    // The directory as a reader sees it: a build that leaves it unservable
    // fails here, naming why, never quietly.
    let held = match Membership::open(dir) {
        Ok(held) => held,
        Err(why) => {
            let _ = writeln!(out, "refused: {} does not serve: {why}", dir.display());
            let _noted = telemetry::emit(
                &telemetry::Event::error(TARGET, "membership built and does not serve")
                    .with("dir", dir.display().to_string().as_str())
                    .with("why", why.to_string().as_str()),
            );
            return FAILED;
        }
    };
    summary(out, &held);
    // A writer killed between its staging create and its unlink leaves a
    // hidden temporary under its own process id, which no later install
    // reuses: it is never read and never removed, and every build names it
    // (PIT-33), so the daily run is not silent about it.
    if held.temporaries() > 0 {
        let _ = writeln!(
            out,
            "note: {} hidden file(s) in {} were not read: a killed writer's leftover temporary \
             is never served and is left in place (PIT-33)",
            held.temporaries(),
            dir.display()
        );
    }
    month_millis.sort_unstable();
    let _ = writeln!(
        out,
        "folded {} days in {} ms; per month written: n {}, p50 {} ms, p99 {} ms, max {} ms",
        built.folded,
        run.elapsed().as_millis(),
        month_millis.len(),
        percentile(&month_millis, 50),
        percentile(&month_millis, 99),
        month_millis.last().copied().unwrap_or(0)
    );
    let _noted = telemetry::emit(
        &telemetry::Event::info(TARGET, "membership built")
            .with("dir", dir.display().to_string().as_str())
            .with("folded", built.folded)
            .with("months", month_millis.len())
            .with("digest", held.file_digest_hex().as_str())
            .with("hidden", held.temporaries())
            .with(
                "ms",
                u64::try_from(run.elapsed().as_millis()).unwrap_or(u64::MAX),
            ),
    );
    if let Some(store) = &flags.store
        && let Err(why) = cross_check(out, &held, store)
    {
        let _ = writeln!(
            out,
            "refused: the cross-check of {}: {why}",
            store.display()
        );
        return FAILED;
    }
    OK
}

/// The GDFL tick store `build` reads: `$BRUTEX_GDFL_TICKSTORE`, which has no
/// default and must hold an `options` folder, or the refusal naming why.
fn tick_store(env: Lookup<'_>) -> Result<PathBuf, String> {
    let ticks = match env(TICKSTORE_VAR) {
        Some(value) if !value.is_empty() => PathBuf::from(value),
        _ => {
            return Err(format!(
                "{TICKSTORE_VAR} is not set; it names the GDFL tick store and has no default"
            ));
        }
    };
    if !ticks.join("options").is_dir() {
        return Err(format!(
            "{} holds no options folder: not a GDFL tick store",
            ticks.display()
        ));
    }
    Ok(ticks)
}

/// The days probed before the span and after `known_to`: never written.
fn report_edges(out: &mut String, built: &pull::fno_membership::Built) {
    for (word, days) in [
        ("before the span", &built.leading),
        ("after known_to", &built.trailing),
    ] {
        let sessions = days
            .iter()
            .filter(|(_, status, _)| *status != DayStatus::NotSession)
            .count();
        if let (Some(first), Some(last)) = (days.first(), days.last()) {
            let _ = writeln!(
                out,
                "{word}: {} days probed ({}..={}), {sessions} of them sessions without a \
                 share-option record: not written, membership unknown",
                days.len(),
                first.0,
                last.0
            );
        }
    }
}

/// The `p`th percentile (nearest rank) of sorted `values`.
fn percentile<T: Copy + Default>(values: &[T], p: usize) -> T {
    if values.is_empty() {
        return T::default();
    }
    let rank = (values.len() * p).div_ceil(100).max(1);
    values
        .get(rank.min(values.len()) - 1)
        .copied()
        .unwrap_or_default()
}

/// The listing lag of every series first stored after the span's first day
/// (`docs/06-limits.md` L-PIT-3): the days between the first day any share's
/// contract of its earliest first-day expiry was stored and its own first
/// stored record, an UPPER BOUND on how late its membership may start. Every
/// series with a lag is named.
///
/// Only the spelling that OPENS a series is counted. A spelling joined to an
/// earlier series by a measured handover (`BSOFT`, `TATACONSUM` on the real
/// tick store) continues a series that was a member under its earlier
/// spelling, so its "lag" is no lateness of membership at all; it is named
/// apart, never counted.
fn listing_lag(out: &mut String, held: &Membership, known_from: Day) {
    let mut opened = std::collections::HashSet::with_capacity(held.series_count());
    let mut joined: Vec<String> = Vec::new();
    let mut later = Vec::new();
    for spelling in held.registry() {
        if !opened.insert(spelling.series_id) {
            joined.push(spelling.name.to_string());
        } else if spelling.first_day > known_from {
            later.push(spelling);
        }
    }
    let mut lagged: Vec<(u32, String)> = later
        .iter()
        .filter(|spelling| spelling.listing_lag_days() > 0)
        .map(|spelling| (spelling.listing_lag_days(), spelling.name.to_string()))
        .collect();
    lagged.sort_unstable_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    let mut days: Vec<u32> = lagged.iter().map(|(lag, _)| *lag).collect();
    days.sort_unstable();
    let _ = writeln!(
        out,
        "listing lag (L-PIT-3, an upper bound): {} series first stored after {known_from}, {} \
         of them later than their first expiry's series start: p50 {} days, p99 {} days, max {} \
         days",
        later.len(),
        lagged.len(),
        percentile(&days, 50),
        percentile(&days, 99),
        days.last().copied().unwrap_or(0)
    );
    let named: Vec<String> = lagged
        .iter()
        .map(|(lag, name)| format!("{name}:{lag}"))
        .collect();
    let _ = writeln!(out, "  lagged series (name:days): {}", named.join(" "));
    let _ = writeln!(
        out,
        "  spellings that joined an earlier series by a measured handover, not counted ({}): {}",
        joined.len(),
        joined.join(" ")
    );
}

/// The span, the largest pool, the series and the per-year counts.
fn summary(out: &mut String, held: &Membership) {
    let Some((from, to)) = held.known_span() else {
        let _ = writeln!(out, "no record is held: every day is membership unknown");
        return;
    };
    let _ = writeln!(
        out,
        "span {from}..={to} (known_from..=known_to, measured); max_pool {} (a rank network \
         sizes itself to the next power of two at or above it); {} series, {} spellings, {} \
         measured handovers; digest {}",
        held.max_pool(),
        held.series_count(),
        held.registry().len(),
        held.handovers().len(),
        held.file_digest_hex()
    );
    if let Some(sealed) = held.sealed_through().filter(|sealed| *sealed != to) {
        let _ = writeln!(
            out,
            "  sealed through {sealed}: the closed days after known_to that close its month are \
             held for the next build and answered as after the span (D-5167)"
        );
    }
    for one in held.handovers() {
        let name = |id| {
            held.spelling(id)
                .map_or_else(String::new, |s| s.name.to_string())
        };
        let _ = writeln!(
            out,
            "  handover ({}) {}: {} -> {} ({} of {} keys)",
            pull::fno_membership::HANDOVER_LABEL,
            one.day,
            name(one.old),
            name(one.new),
            one.shared,
            one.old_keys
        );
    }
    listing_lag(out, held, from);
    for count in held.year_counts() {
        let _ = writeln!(
            out,
            "year {}: {} known sessions, {} refused sessions inside the span, {} sessions \
             outside it (membership unknown)",
            count.year, count.known, count.refused_inside, count.outside
        );
    }
}

/// One month's line, its unknown days and its refusals, and its event.
fn report_month(out: &mut String, month: &MonthBuilt) {
    let (year, number) = month.table.month();
    let pools: Vec<u16> = month
        .table
        .rows()
        .iter()
        .filter(|row| row.status == DayStatus::Known)
        .map(|row| row.pool)
        .collect();
    let (word, path, generation, cleared) = match &month.installed {
        Installed::Unchanged { path, generation } => ("unchanged", path, generation, None),
        Installed::Written {
            path,
            generation,
            cleared,
        } => ("written", path, generation, *cleared),
    };
    let _ = writeln!(
        out,
        "{year:04}-{number:02} {word} g{generation:06} {}: {} known sessions, pool {}..={}, {} \
         unknown sessions, {} handovers, {} not joined, {} stray files, {} ms",
        path.display(),
        pools.len(),
        pools.iter().min().copied().unwrap_or(0),
        pools.iter().max().copied().unwrap_or(0),
        month.unknown.len(),
        month.handovers.len(),
        month.ambiguous.len(),
        month.strays.len(),
        month.millis,
    );
    for (day, status, why) in &month.unknown {
        let _ = writeln!(
            out,
            "  {day} membership unknown: {}: {}{}",
            status.word(),
            status.reason(),
            why.as_ref()
                .map(|why| format!(" ({why})"))
                .unwrap_or_default()
        );
    }
    for (one, old, new) in &month.handovers {
        let _ = writeln!(
            out,
            "  {} handover ({}): {old} -> {new} ({} of {} keys)",
            one.day,
            pull::fno_membership::HANDOVER_LABEL,
            one.shared,
            one.old_keys
        );
    }
    for (day, why) in &month.ambiguous {
        let _ = writeln!(out, "  {day} handover candidate {why}");
    }
    for (day, why) in &month.strays {
        let _ = writeln!(out, "  {day} stray: {why}");
    }
    for (day, refused, sample) in &month.refusals {
        let _ = writeln!(
            out,
            "  {day} {refused} option names refused, first: {}",
            sample.join("; ")
        );
    }
    if let Some(bytes) = cleared {
        let _ = writeln!(
            out,
            "  a dead writer's leftover temporary of {bytes} bytes under this write's own name \
             was cleared first (store::staging::create)"
        );
    }
    let _noted = telemetry::emit(
        &telemetry::Event::info(TARGET, "membership month")
            .with("month", format!("{year:04}-{number:02}").as_str())
            .with("outcome", word)
            .with("generation", *generation)
            .with("known", pools.len())
            .with("unknown", month.unknown.len())
            .with("not_joined", month.ambiguous.len())
            .with("strays", month.strays.len())
            .with("cleared_bytes", cleared.unwrap_or(0))
            .with("ms", u64::try_from(month.millis).unwrap_or(u64::MAX)),
    );
}

/// The shares of today's snapshot: `FNO_UNDERLYINGS` less its indices.
fn snapshot_shares() -> impl Iterator<Item = &'static str> {
    FNO_UNDERLYINGS
        .iter()
        .copied()
        .filter(|name| !FNO_INDEX_UNDERLYINGS.contains(name))
}

/// The names that differ from today's snapshot on `day`.
fn snapshot_diff(
    out: &mut String,
    held: &Membership,
    members: &[pull::fno_membership::Member],
    day: Day,
) {
    let not_members: Vec<&str> = snapshot_shares()
        .filter(|name| held.is_member(name, day) == Ok(false))
        .collect();
    let _ = writeln!(
        out,
        "shares of the 2026-08-01 snapshot FNO_UNDERLYINGS not a member that day ({}): {}",
        not_members.len(),
        not_members.join(" ")
    );
    let outside: Vec<&str> = members
        .iter()
        .filter(|member| !member.in_current_list)
        .map(|member| member.symbol_on_day.as_str())
        .collect();
    let _ = writeln!(
        out,
        "members whose series is not in that snapshot ({}): {}",
        outside.len(),
        outside.join(" ")
    );
}

/// A day's members in series order, and those that listed no file.
fn member_lines(out: &mut String, members: &[pull::fno_membership::Member]) {
    let listed: Vec<String> = members
        .iter()
        .map(|member| {
            format!(
                "{}:{}{}",
                member.series_id,
                member.symbol_on_day,
                if member.in_current_list { "*" } else { "" }
            )
        })
        .collect();
    let _ = writeln!(
        out,
        "members (series:spelling, * = a share of the 2026-08-01 snapshot): {}",
        listed.join(" ")
    );
    let unfiled: Vec<&str> = members
        .iter()
        .filter(|member| !member.filed)
        .map(|member| member.symbol_on_day.as_str())
        .collect();
    let _ = writeln!(
        out,
        "members with no file that day ({}): {}",
        unfiled.len(),
        unfiled.join(" ")
    );
}

/// `show DAY`.
fn show(day: &str, flags: &Flags, out: &mut String) -> u8 {
    let dir = flags.dir.as_path();
    let day = match day_of(day) {
        Ok(day) => day,
        Err(why) => return refuse(out, &why),
    };
    let held = match Membership::open(dir) {
        Ok(held) => held,
        Err(why) => {
            let _ = writeln!(out, "refused: {why}");
            return FAILED;
        }
    };
    let members = match held.members(day) {
        Ok(members) => members,
        Err(why) => {
            let _ = writeln!(out, "{why}");
            if let Ok(refused) = held.refused_through(day) {
                let _ = writeln!(
                    out,
                    "refused sessions of {} inside the span through {day}: {refused}",
                    day.year()
                );
            }
            return FAILED;
        }
    };
    let row = held.row(day).ok();
    let _ = writeln!(
        out,
        "{day}: pool {} shares with a live stock-option contract ({SOURCE_LABEL}, D-5160); {} \
         listed a file; {} option names, {} refused, {} of index underlyings",
        members.len(),
        row.map_or(0, |row| row.filed),
        row.map_or(0, |row| row.names),
        row.map_or(0, |row| row.refused),
        row.map_or(0, |row| row.index_names),
    );
    member_lines(out, members);
    snapshot_diff(out, &held, members, day);
    if let Some(store) = &flags.store {
        match folders(store, Segment::Cash) {
            Ok(cash) => {
                let missing: Vec<&str> = members
                    .iter()
                    .map(|member| member.symbol_on_day.as_str())
                    .filter(|name| !cash.contains(*name))
                    .collect();
                let _ = writeln!(
                    out,
                    "members with no GDFL share folder in {} under their spelling that day ({}): {}",
                    store.display(),
                    missing.len(),
                    missing.join(" ")
                );
            }
            Err(why) => {
                let _ = writeln!(
                    out,
                    "refused: the cross-check of {}: {why}",
                    store.display()
                );
                return FAILED;
            }
        }
    }
    let _ = writeln!(
        out,
        "leverage outside F&O: none (D-5160); figure: {}",
        pull::fno_membership::LEVERAGE
    );
    if held.temporaries() > 0 {
        let _ = writeln!(
            out,
            "note: {} hidden file(s) in {} were not read",
            held.temporaries(),
            dir.display()
        );
    }
    OK
}

/// Nanoseconds of `took`.
fn nanos(took: Duration) -> u64 {
    u64::try_from(took.as_nanos()).unwrap_or(u64::MAX)
}

/// `bench`: every day of the span and the [`BENCH_MARGIN`] days either side
/// of it, each lookup once, sequentially: on a known day `members`,
/// `pool_size` and `is_member`; on every other day `members`, which refuses
/// (a refused session or closed day inside the span, or a day outside it).
fn bench(dir: &Path, out: &mut String) -> u8 {
    let clock = Instant::now();
    let held = match Membership::open(dir) {
        Ok(held) => held,
        Err(why) => {
            let _ = writeln!(out, "refused: {why}");
            return FAILED;
        }
    };
    let opened = clock.elapsed();
    let Some((from, to)) = held.known_span() else {
        let _ = writeln!(out, "refused: {} holds no record to time", dir.display());
        return FAILED;
    };
    let mut times: [Vec<u64>; 4] = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
    let epoch = |day: Day, epoch: Option<u32>| {
        epoch
            .and_then(|epoch| Day::from_days(epoch).ok())
            .unwrap_or(day)
    };
    let (first_day, last_day) = (
        epoch(from, from.days_from_epoch().checked_sub(BENCH_MARGIN)),
        epoch(to, to.days_from_epoch().checked_add(BENCH_MARGIN)),
    );
    let mut day = first_day;
    let mut checksum = 0_usize;
    while day <= last_day {
        if held.members(day).is_err() {
            let clock = Instant::now();
            let refused = held.members(day).is_err();
            times[3].push(nanos(clock.elapsed()));
            checksum += usize::from(refused);
        } else if let Ok(first) = held.members(day).map(|members| members.first().copied()) {
            let clock = Instant::now();
            let members = held.members(day).map_or(0, <[_]>::len);
            times[0].push(nanos(clock.elapsed()));
            let clock = Instant::now();
            let pool = held.pool_size(day).map_or(0, usize::from);
            times[1].push(nanos(clock.elapsed()));
            if let Some(first) = first {
                let clock = Instant::now();
                let member = held.is_member(first.symbol_on_day.as_str(), day) == Ok(true);
                times[2].push(nanos(clock.elapsed()));
                checksum += usize::from(member);
            }
            checksum += members + pool;
        }
        let Ok(next) = day.succ() else { break };
        day = next;
    }
    let _ = writeln!(
        out,
        "fno-membership bench {} (span {from}..={to}, timed {first_day}..={last_day}): open {} \
         ms; every day once, sequential, Instant per call (checksum {checksum})",
        dir.display(),
        opened.as_millis()
    );
    for (name, values) in [
        "members(day)",
        "pool_size(day)",
        "is_member(name, day)",
        "members(day) refused (membership unknown)",
    ]
    .iter()
    .zip(&mut times)
    {
        values.sort_unstable();
        let _ = writeln!(
            out,
            "{name}: n {}, p50 {} ns, p99 {} ns, max {} ns",
            values.len(),
            percentile(values, 50),
            percentile(values, 99),
            values.last().copied().unwrap_or(0)
        );
    }
    OK
}

#[cfg(test)]
#[path = "fno_membership_command_tests.rs"]
mod tests;

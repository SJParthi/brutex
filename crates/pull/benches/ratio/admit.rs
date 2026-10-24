//! C-CHK-10 (CC-20, FM-615, D-6801, D-6802): the import's admit decision, `admit` in
//! `crates/pull/src/gdfl_import_day.rs`, which is private (that file is owned
//! by R4-09 and R3-26, neither merged on 24 Oct 2026), so it is timed
//! through the public run API, `pull::gdfl_import::run_cm_from`, on an
//! invented source. No seam was added to that file.
//!
//! What is timed, exactly: the run tells its observer `day_begun` and
//! `day_passed` around each calendar day, and the time between the two is
//! the day loop's step that holds `admit` — the day's journal key formatted,
//! `widen`'s probe for a wider scope, then `admit` itself (`KeyState::of`'s
//! hash probes and `admit_rule`'s table, then the calendar gate). The stop
//! probe and the store root's `metadata` call come BEFORE `day_begun`, and
//! the journal's load, O(lines), happens once before the first day: none of
//! them is in the figure. So the figure is an UPPER BOUND on one `admit`
//! decision, never `admit` alone, and the row says so.
//!
//! The journal is written through the real path: a first run over 25, 250
//! and 2,500 calendar days from 2018-12-17 closes every trading day `done`
//! (the source lists each day with no file in it). The timed run then asks
//! the same first 25 days again at each size, so every size times the same
//! days: each trading day is `admit`'s skip of a day already done, each
//! weekend or holiday its calendar gate's `closed`. The window crosses the
//! year 2018 into 2019. The decision must cost the same per day at 1x, 10x and
//! 100x the journal's lines. The journal's line counts are printed. The store
//! roots are left for clean-up (R4-08's no-delete rule) and printed. The
//! harness, `crates/pull/benches/ratio.rs`, fails on a ratio above its ceiling
//! and prints p50, p99 and max of the single steps beside it.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};
use std::time::Instant;

use pull::gdfl_cm::{CmKind, CmRefusal, CmSource, DayListing, ListedFile};
use pull::gdfl_import::{ImportKind, ImportRefusal, Observer, Report, Run, RunOptions, run_cm_from};
use pull::session::Day;

use super::{ratio, refuse, spread_of};

/// Calendar days the first run closes at 1x; 10x and 100x close ten and a
/// hundred times as many. The timed window is these first days.
const WINDOW: u32 = 25;
/// Timed runs per size; the minimum of their mean step is kept.
const TRIALS: u32 = 40;

/// A source holding every day of the indices tree, each with no file: what a
/// run closes `done` with nothing to file. It is never asked for a file.
struct EveryDayEmpty;

impl CmSource for EveryDayEmpty {
    type Locator = ();

    fn day(&self, kind: CmKind, day: Day) -> Result<Option<DayListing<()>>, CmRefusal> {
        Ok(Some(DayListing::new(kind, day)))
    }

    fn fetch(
        &self,
        _listing: &DayListing<()>,
        _file: &ListedFile<()>,
    ) -> Result<Vec<u8>, CmRefusal> {
        Err(CmRefusal::ArchiveUnavailable {
            kind: std::io::ErrorKind::NotFound,
        })
    }
}

/// The time between each `day_begun` and its `day_passed`, in nanoseconds.
#[derive(Debug, Default)]
struct Steps {
    began: Mutex<Option<Instant>>,
    ns: Mutex<Vec<u128>>,
}

impl Steps {
    fn take(&self) -> Vec<u128> {
        std::mem::take(&mut *self.ns.lock().unwrap_or_else(PoisonError::into_inner))
    }
}

impl Observer for Steps {
    fn begun(&self, _kind: ImportKind, _run: u64) {}

    fn day_begun(&self, _kind: ImportKind, _day: Day, _at: (usize, usize)) {
        let mut began = self.began.lock().unwrap_or_else(PoisonError::into_inner);
        *began = Some(Instant::now());
    }

    fn day_passed(&self, _kind: ImportKind, _day: Day, _report: &Report) {
        let now = Instant::now();
        let began = self
            .began
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take();
        if let Some(began) = began {
            self.ns
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(now.duration_since(began).as_nanos());
        }
    }

    fn finished(&self, _kind: ImportKind, _end: Result<&Report, &ImportRefusal>) {}
}

/// The first day of every run here: a Monday, so the window holds both
/// answers and crosses the year.
fn first() -> Day {
    Day::new(2018, 12, 17).unwrap_or_else(|_| refuse("a real day"))
}

/// The `n`th calendar day from [`first`].
fn nth(n: u32) -> Day {
    Day::from_days(first().days_from_epoch() + n).unwrap_or_else(|_| refuse("a day in range"))
}

/// A new store root under the system temp directory, not yet made (the run
/// makes a fresh store's root itself), printed.
fn root(days: u32) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "brutex-chk-admit-{days}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos())
    ));
    println!("  C-CHK-10 scratch (left for clean-up): {}", dir.display());
    dir
}

/// One run over `days` calendar days from [`first`] into `store`.
fn run(store: &Path, days: u32, observer: Option<&dyn Observer>) -> Report {
    let run = Run {
        kind: ImportKind::Indices,
        from: first(),
        to: nth(days - 1),
        only: &[],
        store_root: store,
    };
    let options = RunOptions {
        observer,
        ..RunOptions::default()
    };
    run_cm_from(&EveryDayEmpty, &run, options)
        .unwrap_or_else(|why| refuse(&format!("an invented run was refused: {why}")))
}

/// The journal's lines.
fn journal_lines(store: &Path) -> usize {
    std::fs::read_to_string(store.join("imports/gdfl.journal"))
        .map_or(0, |text| text.lines().count())
}

/// A store whose journal closed `days` calendar days, and its trading days.
fn closed(days: u32) -> (PathBuf, usize) {
    let store = root(days);
    let first_run = run(&store, days, None);
    let total = usize::try_from(days).unwrap_or(usize::MAX);
    if first_run.days_imported == 0
        || first_run.days_imported + first_run.days_closed != total
        || !first_run.failures.is_empty()
    {
        refuse(&format!(
            "the first run closed every day: imported {}, closed {}, refused {}, \
             failures {}",
            first_run.days_imported,
            first_run.days_closed,
            first_run.days_refused,
            first_run.failures.len()
        ));
    }
    (store, first_run.days_imported)
}

/// The minimum mean step over [`TRIALS`] runs of the window over `store`,
/// in picoseconds, and every single step's nanoseconds.
fn window_steps(store: &Path) -> (u128, Vec<u128>) {
    let steps = Steps::default();
    let mut best = u128::MAX;
    let mut all = Vec::new();
    for _ in 0..TRIALS {
        let report = run(store, WINDOW, Some(&steps));
        let total = usize::try_from(WINDOW).unwrap_or(usize::MAX);
        if report.days_skipped + report.days_closed != total || report.days_imported != 0 {
            refuse("the timed window is every day skipped as done or closed");
        }
        let ns = steps.take();
        if ns.len() != total {
            refuse("one step timed per calendar day");
        }
        let mean_ps = ns.iter().sum::<u128>() * 1_000 / u128::from(WINDOW);
        best = best.min(mean_ps);
        all.extend(ns);
    }
    (best, all)
}

/// C-CHK-10: the day step that holds `admit`, flat at 1x, 10x and 100x the
/// journal's lines.
pub(crate) fn admit_step_is_flat() -> bool {
    let mut costs = Vec::with_capacity(3);
    let mut largest = Vec::new();
    for days in [WINDOW, WINDOW * 10, WINDOW * 100] {
        let (store, trading) = closed(days);
        let before = journal_lines(&store);
        let (best, all) = window_steps(&store);
        println!(
            "  C-CHK-10 journal of {days:>5} days ({trading} done): {before} lines, \
             {} after the timed runs",
            journal_lines(&store)
        );
        costs.push(best);
        largest = all;
    }
    let [one, ten, hundred] = costs.as_slice() else {
        refuse("three sizes measured")
    };
    let ok = ratio("C-CHK-10 admit day step, 10x journal", *one, *ten)
        & ratio("C-CHK-10 admit day step, 100x journal", *one, *hundred);
    spread_of("C-CHK-10 admit day step, 100x journal", largest);
    ok
}

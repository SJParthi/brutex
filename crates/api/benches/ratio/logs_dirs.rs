//! Gate 8 for the readers IA-11r widened: a keyed `/logs.json` request and
//! the importer-evidence read over the closed set of run directories
//! (C-LG-01), and `/imports.json`'s runs page (C-LG-02). D-4862, D-4813.
//!
//! # What is timed, and why it is flat
//!
//! **C-LG-01.** A keyed `/logs.json` request (`run=<id>&durable=1`, 20 rows)
//! over the server's directory and the seven run directories, each holding a
//! real audit stream written through a real sink. The run's posting in every
//! directory holds 1x and then 10x as many pointers; the request still reads
//! one page of them, so its cost may not track the posting
//! (`docs/06-limits.md` L-IA-13). Beside it, the importer-evidence read behind
//! `/imports.json`, `/dayflags.json` and `/store`: the ring tails of the five
//! importer directories with nothing new to read, and the keyed probe of one
//! day's 16 refusals in each of the three directories of its kind (48 lines
//! merged), while the streams around that day grow 10x (L-IA-37).
//!
//! **C-LG-02.** `/imports.json?feed=gdfl` over journals of 1,000, 10,000 and
//! 100,000 `run` lines: the answer decodes the 20 lines of its page and
//! `run=<id>` the id's own, so neither may cost more on the longer journal.
//!
//! **Printed, not asserted:** each figure's p50, p99 and max over single
//! calls, and the first read of each store (the ring read once, the journal
//! parsed once), which is bounded by the files and paid once per process.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::time::Instant;

use super::{RATIO_CEILING_PERMILLE, cost_ps, ratio, refuse};

/// The run every directory's posting is keyed by.
const RUN: u64 = 1_791_700_000;

/// The day whose refusals the importer-evidence probe reads, and how many it
/// holds per directory: the cap a run keeps per (run, day, stage, code).
const DAY: &str = "2022-03-07";
/// See [`DAY`].
const DAY_REFUSALS: usize = 16;

/// Audited lines per directory at 1x and 10x.
const POSTING_SIZES: [usize; 2] = [200, 2_000];

/// `run` lines per journal.
const JOURNAL_SIZES: [usize; 3] = [1_000, 10_000, 100_000];

/// The directories a run process logs to, under `logs/`: the closed set of
/// D-4862, spelt here as the store spells it.
const RUN_DIRS: [&str; 7] = [
    "cli",
    "gdfl/indices",
    "gdfl/stocks",
    "gdfl/options",
    "gdfl/store",
    "verify/zerodha",
    "verify/merge",
];

/// A scratch store of this process's own, emptied first.
fn root_of(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "brutex-api-bench-logs-{name}-{}",
        std::process::id()
    ));
    let _ignored = std::fs::remove_dir_all(&root);
    if let Err(why) = std::fs::create_dir_all(&root) {
        refuse(&format!("the logs bench root would not create: {why}"));
    }
    root
}

/// Writes `lines` audited lines of [`RUN`] to `dir` through a real sink:
/// [`DAY_REFUSALS`] importer refusals of `kind` on [`DAY`], the rest on other
/// days, so the day's posting is the same size at every `lines`.
fn fill(dir: &Path, kind: &str, lines: usize) {
    if let Err(why) = std::fs::create_dir_all(dir) {
        refuse(&format!("{} would not create: {why}", dir.display()));
    }
    let Ok(sink) = telemetry::Sink::open(&telemetry::Config::new(dir)) else {
        refuse("a bench sink would not open");
    };
    for n in 0..lines {
        let day = if n < DAY_REFUSALS {
            DAY.to_owned()
        } else {
            format!("2021-{:02}-{:02}", 1 + (n / 28) % 12, 1 + n % 28)
        };
        let event = telemetry::Event::error("gdfl.import", "refused")
            .with("run", RUN)
            .with("kind", kind)
            .with("day", day.as_str())
            .with("about", "ZZ BENCH FIXTURE.csv")
            .with("code", "malformed_row");
        if !sink.emit(&event).is_written() {
            refuse("a bench line was not written");
        }
    }
}

/// A store whose every log directory holds `lines` audited lines of [`RUN`].
fn logs_store(lines: usize) -> PathBuf {
    let root = root_of(&format!("dirs-{lines}"));
    let logs = root.join("logs");
    fill(&logs, "served", lines);
    for dir in RUN_DIRS {
        // `logs/cli`, `logs/gdfl/indices` and `logs/gdfl/store` all hold
        // indices refusals, so the indices probe merges three directories.
        let kind = match dir {
            "gdfl/stocks" => "stocks",
            "gdfl/options" => "options",
            _ => "indices",
        };
        fill(&logs.join(dir), kind, lines);
    }
    root
}

/// A store whose journal holds `runs` `run` lines and nothing else.
fn journal_store(runs: usize) -> PathBuf {
    let root = root_of(&format!("runs-{runs}"));
    let path = pull::gdfl_import::journal_path(&root);
    let mut text = String::with_capacity(runs.saturating_mul(48));
    for id in 1..=runs {
        let _ = writeln!(text, "run {id} source=zip started={id}");
    }
    let made = path
        .parent()
        .map_or(Ok(()), std::fs::create_dir_all)
        .and_then(|()| std::fs::write(&path, text));
    if let Err(why) = made {
        refuse(&format!("the bench journal would not write: {why}"));
    }
    root
}

/// Single calls of `op`, sorted: p50, p99 and max in nanoseconds, printed.
fn spread<T>(label: &str, reps: usize, mut op: impl FnMut() -> T) {
    let mut ns: Vec<u128> = (0..reps)
        .map(|_| {
            let start = Instant::now();
            std::hint::black_box(op());
            start.elapsed().as_nanos()
        })
        .collect();
    ns.sort_unstable();
    let at = |permille: usize| {
        ns.get((ns.len() * permille / 1_000).min(ns.len().saturating_sub(1)))
            .copied()
            .unwrap_or(0)
    };
    println!(
        "  {label:<46} p50 {:>9} ns  p99 {:>9} ns  max {:>9} ns  n={reps}",
        at(500),
        at(990),
        ns.last().copied().unwrap_or(0),
    );
}

/// C-LG-01: the keyed request and the importer-evidence read, at 1x and 10x.
fn keyed_reads_are_flat(id: &str) -> bool {
    let stores: Vec<(usize, PathBuf)> = POSTING_SIZES
        .iter()
        .map(|&lines| (lines, logs_store(lines)))
        .collect();
    let query = format!("run={RUN}&durable=1&limit=20");
    let mut keyed = Vec::with_capacity(stores.len());
    let mut tails = Vec::with_capacity(stores.len());
    let mut probes = Vec::with_capacity(stores.len());
    for (lines, root) in &stores {
        let served = root.join("logs");
        let body = api::logs::logs_json_over(Some(&served), root, &query);
        // THE FIXTURE IS WHAT IT CLAIMS: 20 rows, from the index, and every
        // one of the 8 directories named as read.
        let rows = body.matches("\"source\":\"audit\"").count();
        let read = body.matches("\"state\":\"read\"").count();
        if rows != 20 || read != 8 {
            refuse(&format!(
                "the keyed request over {lines} lines a directory answered {rows} rows from \
                 {read} directories, not 20 from 8"
            ));
        }
        // The first evidence read parses each ring once; printed, then warm.
        let first = Instant::now();
        let found = api::logs::import_evidence_over(root, "indices", DAY);
        println!(
            "  {:<46} {:>9} µs  (5 rings read once, {lines} lines each)",
            format!("{id} first evidence read"),
            first.elapsed().as_micros()
        );
        if found.0 != 3 * DAY_REFUSALS || found.1 != 3 * DAY_REFUSALS {
            refuse(&format!(
                "the evidence read found {found:?}, not {} in the rings and in the streams of \
                 logs/cli, logs/gdfl/indices and logs/gdfl/store",
                3 * DAY_REFUSALS
            ));
        }
        keyed.push(cost_ps(40, || {
            api::logs::logs_json_over(Some(&served), root, &query).len()
        }));
        tails.push(cost_ps(200, || {
            api::logs::import_evidence_over(root, "stocks", "1999-01-01").0
        }));
        probes.push(cost_ps(40, || {
            api::logs::import_evidence_over(root, "indices", DAY).1
        }));
        spread(&format!("{id} keyed /logs.json, {lines}/dir"), 200, || {
            api::logs::logs_json_over(Some(&served), root, &query).len()
        });
        spread(&format!("{id} evidence probe, {lines}/dir"), 200, || {
            api::logs::import_evidence_over(root, "indices", DAY).1
        });
    }
    let flat = |label: &str, costs: &[u128]| -> bool {
        let (Some(&base), Some(&big)) = (costs.first(), costs.get(1)) else {
            refuse("two sizes measured");
        };
        ratio(
            &format!("{id} {label} {}->{}", POSTING_SIZES[0], POSTING_SIZES[1]),
            base,
            big,
            RATIO_CEILING_PERMILLE,
        )
    };
    let ok = flat("keyed /logs.json, 8 dirs", &keyed)
        & flat("evidence tails, nothing new", &tails)
        & flat("evidence probe of one day", &probes);
    for (_, root) in stores {
        let _ignored = std::fs::remove_dir_all(root);
    }
    ok
}

/// C-LG-02: the runs page and `run=` at every journal size.
fn the_runs_page_is_flat(id: &str) -> bool {
    let mut pages = Vec::with_capacity(JOURNAL_SIZES.len());
    let mut by_run = Vec::with_capacity(JOURNAL_SIZES.len());
    let mut roots = Vec::with_capacity(JOURNAL_SIZES.len());
    for runs in JOURNAL_SIZES {
        let root = journal_store(runs);
        let first = Instant::now();
        let Ok(body) = api::logs::imports_json_over(&root, "feed=gdfl") else {
            refuse("the runs page was refused");
        };
        println!(
            "  {:<46} {:>9} µs  (the journal parsed once)",
            format!("{id} first read, {runs} runs"),
            first.elapsed().as_micros()
        );
        if !body.contains(&format!("\"runs_total\":{runs}")) || !body.contains("\"runs_next\":\"") {
            refuse(&format!(
                "the runs page over {runs} runs is not a page of them"
            ));
        }
        let one = format!("feed=gdfl&run={}", runs / 2);
        pages.push(cost_ps(200, || {
            api::logs::imports_json_over(&root, "feed=gdfl").map_or(0, |body| body.len())
        }));
        by_run.push(cost_ps(200, || {
            api::logs::imports_json_over(&root, &one).map_or(0, |body| body.len())
        }));
        spread(&format!("{id} runs page, {runs} runs"), 300, || {
            api::logs::imports_json_over(&root, "feed=gdfl").map_or(0, |body| body.len())
        });
        spread(&format!("{id} run=<id>, {runs} runs"), 300, || {
            api::logs::imports_json_over(&root, &one).map_or(0, |body| body.len())
        });
        roots.push(root);
    }
    let flat = |label: &str, costs: &[u128]| -> bool {
        let (Some(&base), Some(&mid), Some(&big)) = (costs.first(), costs.get(1), costs.get(2))
        else {
            refuse("three sizes measured");
        };
        ratio(
            &format!("{id} {label} {}->{}", JOURNAL_SIZES[0], JOURNAL_SIZES[1]),
            base,
            mid,
            RATIO_CEILING_PERMILLE,
        ) & ratio(
            &format!("{id} {label} {}->{}", JOURNAL_SIZES[0], JOURNAL_SIZES[2]),
            base,
            big,
            RATIO_CEILING_PERMILLE,
        )
    };
    let ok = flat("runs page", &pages) & flat("run=<id>", &by_run);
    for root in roots {
        let _ignored = std::fs::remove_dir_all(root);
    }
    ok
}

/// Both rows. `keyed` and `runs` are their ids, named by
/// `crates/api/benches/ratio.rs`'s `main`.
pub(crate) fn reads_are_flat(keyed: &str, runs: &str) -> bool {
    keyed_reads_are_flat(keyed) & the_runs_page_is_flat(runs)
}

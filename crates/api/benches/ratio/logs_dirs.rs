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
//! **C-LG-03 (R3-24).** Every keyed search `/logs.json` answers since R3-24
//! (day, month, reason, import kind, code, run, a target alone, a time span
//! alone and a span beside a key) over the same stores at 1x and 10x, gated
//! on the ratio as C-LG-01 is; each one's first call ("cold") and p50, p99
//! and max over n=200 single calls ("warm"), then at 10x with 60 clients at
//! once and while an import writes into `logs/cli`; and the live tail from
//! an installed sink's copy in memory beside the same tail read from files.
//! Each row prints whether its p99 met R3-24's 5 ms target on THIS machine;
//! the operator's figures are the Mac's (the R3-24 report).
//!
//! **Printed, not asserted:** each figure's p50, p99 and max over single
//! calls, and the first read of each store (the ring read once, the journal
//! parsed once), which is bounded by the files and paid once per process.
//! And, beside C-LG-02, `/dayflags.json`'s day band over the same three
//! journals: its read of the journal keeps every `run` line, so it tracks
//! the journal and is `docs/06-limits.md` L-IA-39, measured here at 1,000,
//! 10,000 and 100,000 `run` lines and gated by nothing.

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

/// The directories a run process logs to, under `logs/`: `cli` and every row
/// of the one table the writers and the readers share
/// (`telemetry::run_dirs`, D-4897). Never spelt here.
fn run_dirs() -> impl Iterator<Item = &'static str> {
    std::iter::once(telemetry::run_dirs::CLI)
        .chain(telemetry::run_dirs::ALL.iter().map(|dir| dir.sub))
}

/// The `/dayflags.json` request whose day band is timed beside the runs
/// page: one month of one instrument of a kind, so the answer reads the
/// import journal through the crate-internal caller that keeps every run.
const DAY_BAND: &str = "feed=gdfl&instrument=NSE-INDEX-NIFTY&from=2019-10&to=2019-10";

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
            .with("code", "malformed_row")
            .with("reason", "row_refused");
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
    let mut filled = 1usize;
    for dir in run_dirs() {
        // A KIND'S OWN DIRECTORY holds that kind's refusals; every other one
        // (`logs/cli`, `logs/gdfl/store`, the verify pair) holds indices
        // refusals, so the indices probe merges three directories.
        let kind = telemetry::run_dirs::GDFL_KINDS
            .iter()
            .find(|own| own.sub == dir)
            .and_then(|own| own.gdfl_kind)
            .unwrap_or("indices");
        fill(&logs.join(dir), kind, lines);
        filled += 1;
    }
    if filled != 8 {
        refuse(&format!(
            "the table names {filled} log directories with the server's own, and \
             docs/06-limits.md L-IA-13 is computed for 8: recompute it with the table"
        ));
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
        let rows = body.matches("\"source\":\"audit\",\"scope\"").count();
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
        // THE DAY BAND'S READ OF THE SAME JOURNAL (L-IA-39): it resolves a
        // refusal's run by time, so it keeps every `run` line and its cost
        // tracks the journal. Printed at every size and NEVER gated: it is a
        // named limit until the run comes from the event's own field
        // (D-4896), and a ratio gate here would only say what the row says.
        let Ok(band) = api::logs::dayflags_json_over(&root, DAY_BAND) else {
            refuse("the day band's request was refused");
        };
        if !band.contains("\"instrument\":\"NSE-INDEX-NIFTY\"") || !band.contains("\"days\":[") {
            refuse(&format!(
                "the day band over {runs} runs is not a /dayflags.json answer"
            ));
        }
        spread(
            &format!("{id} day band (L-IA-39), {runs} runs"),
            100,
            || api::logs::dayflags_json_over(&root, DAY_BAND).map_or(0, |body| body.len()),
        );
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

/// Every row. `keyed` and `runs` are the first two's ids, named by
/// `crates/api/benches/ratio.rs`'s `main`; R3-24's row is C-LG-03, named
/// here because `ratio.rs` is not R3-24's file.
pub(crate) fn reads_are_flat(keyed: &str, runs: &str) -> bool {
    keyed_reads_are_flat(keyed) & the_runs_page_is_flat(runs) & r3_24_searches_are_flat("C-LG-03")
}

// ---------------------------------------------------------------------------
// C-LG-03 (R3-24): EVERY KEYED SEARCH, COLD AND WARM, 1 AND 60 CLIENTS, AND
// WITH AN IMPORT WRITING; THE LIVE TAIL FROM MEMORY
// ---------------------------------------------------------------------------

/// The searches R3-24 times, by name, over a store of [`logs_store`]: each a
/// key `/logs.json` reads through the index (or, for a span alone, through
/// the audit files its time meets). `{ts}` is replaced by the newest line's
/// stamp, so the span is one a real line falls in.
const SEARCHES: [(&str, &str); 9] = [
    ("day", "day=2022-03-07&limit=20"),
    ("month", "month=2021-03&limit=20"),
    ("reason", "reason=row_refused&limit=20"),
    ("kind", "kind=stocks&limit=20"),
    ("code", "code=malformed_row&limit=20"),
    ("run", "run=1791700000&durable=1&limit=20"),
    ("target alone", "target=gdfl.import&limit=20"),
    ("span alone", "from={ts}&to={ts}&limit=20"),
    (
        "span beside a code",
        "code=malformed_row&from=0&to={ts}&limit=20",
    ),
];

/// R3-24's target for every keyed search: p99 under 5 ms (the operator's
/// store measured 11 to 15 ms for the slowest keys before it). Printed per
/// row, never asserted here: this machine's p99 is not the operator's, and
/// `crates/api/benches/ratio.rs` gates shapes, not seconds.
const TARGET_P99_NS: u128 = 5_000_000;

/// p50, p99 and max of sorted nanoseconds.
fn quantiles(ns: &mut [u128]) -> (u128, u128, u128) {
    ns.sort_unstable();
    let at = |permille: usize| {
        ns.get((ns.len() * permille / 1_000).min(ns.len().saturating_sub(1)))
            .copied()
            .unwrap_or(0)
    };
    (at(500), at(990), ns.last().copied().unwrap_or(0))
}

/// Prints one row of [`quantiles`] with whether its p99 meets
/// [`TARGET_P99_NS`].
fn print_row(label: &str, ns: &mut [u128]) {
    let n = ns.len();
    let (p50, p99, max) = quantiles(ns);
    println!(
        "  {label:<46} p50 {p50:>9} ns  p99 {p99:>9} ns  max {max:>9} ns  n={n}  {}",
        if p99 < TARGET_P99_NS {
            "p99 < 5 ms"
        } else {
            "p99 OVER 5 ms"
        }
    );
}

/// `reps` single calls of `op` from each of `clients` threads at once.
fn clients_at_once(clients: usize, reps: usize, op: impl Fn() -> usize + Sync) -> Vec<u128> {
    std::thread::scope(|scope| {
        let handles: Vec<_> = (0..clients)
            .map(|_| {
                scope.spawn(|| {
                    (0..reps)
                        .map(|_| {
                            let start = Instant::now();
                            std::hint::black_box(op());
                            start.elapsed().as_nanos()
                        })
                        .collect::<Vec<u128>>()
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|handle| handle.join().unwrap_or_default())
            .collect()
    })
}

/// The newest line's stamp in `root`'s served directory, read through the
/// route itself.
fn newest_stamp(served: &Path, root: &Path) -> String {
    let body = api::logs::logs_json_over(Some(served), root, "code=malformed_row&limit=1");
    let Some(at) = body.split("\"ts\":").nth(1) else {
        refuse("the newest line's stamp");
    };
    at.chars().take_while(char::is_ascii_digit).collect()
}

/// C-LG-03: each search at 1x and 10x (gated on the ratio, as C-LG-01),
/// cold (the first call on its store) and warm (n=200), then at 10x with 60
/// clients at once and with an import writing; then the live tail from the
/// installed sink's copy in memory beside the same tail read from files.
fn r3_24_searches_are_flat(id: &str) -> bool {
    let stores: Vec<(usize, PathBuf)> = POSTING_SIZES
        .iter()
        .map(|&lines| (lines, logs_store(lines)))
        .collect();
    let mut ok = true;
    let mut costs: Vec<Vec<u128>> = vec![Vec::new(); SEARCHES.len()];
    for (lines, root) in &stores {
        let served = root.join("logs");
        let ts = newest_stamp(&served, root);
        for (n, (name, query)) in SEARCHES.iter().enumerate() {
            let query = query.replace("{ts}", &ts);
            let cold = Instant::now();
            let body = api::logs::logs_json_over(Some(&served), root, &query);
            let cold = cold.elapsed().as_nanos();
            if !body.contains("\"keyed\":{") || body.contains("\"ignored\":[{") {
                refuse(&format!(
                    "the {name} search ({query}) is not a keyed answer"
                ));
            }
            if !body.contains("\"records\":[{") {
                refuse(&format!("the {name} search ({query}) found nothing"));
            }
            println!(
                "  {:<46} cold {cold:>9} ns",
                format!("{id} {name}, {lines}/dir")
            );
            let mut warm: Vec<u128> = (0..200)
                .map(|_| {
                    let start = Instant::now();
                    std::hint::black_box(api::logs::logs_json_over(Some(&served), root, &query));
                    start.elapsed().as_nanos()
                })
                .collect();
            print_row(&format!("{id} {name}, {lines}/dir, warm"), &mut warm);
            if let Some(row) = costs.get_mut(n) {
                row.push(cost_ps(20, || {
                    api::logs::logs_json_over(Some(&served), root, &query).len()
                }));
            }
        }
    }
    for ((name, _), row) in SEARCHES.iter().zip(&costs) {
        let (Some(&base), Some(&big)) = (row.first(), row.get(1)) else {
            refuse("two sizes measured");
        };
        ok &= ratio(
            &format!("{id} {name} {}->{}", POSTING_SIZES[0], POSTING_SIZES[1]),
            base,
            big,
            RATIO_CEILING_PERMILLE,
        );
    }
    // AT 10x: 60 CLIENTS AT ONCE, then ONE CLIENT WHILE AN IMPORT WRITES.
    if let Some((lines, root)) = stores.last() {
        let served = root.join("logs");
        let ts = newest_stamp(&served, root);
        for (name, query) in SEARCHES {
            let query = query.replace("{ts}", &ts);
            let mut many = clients_at_once(60, 4, || {
                api::logs::logs_json_over(Some(&served), root, &query).len()
            });
            print_row(&format!("{id} {name}, {lines}/dir, 60 clients"), &mut many);
        }
        let writing = std::sync::atomic::AtomicBool::new(true);
        let cli = served.join(telemetry::run_dirs::CLI);
        std::thread::scope(|scope| {
            let writer = scope.spawn(|| {
                let Ok(sink) = telemetry::Sink::open(&telemetry::Config::new(&cli)) else {
                    refuse("the import's sink would not open");
                };
                let mut written = 0u64;
                while writing.load(std::sync::atomic::Ordering::Relaxed) {
                    let _ = sink.emit(
                        &telemetry::Event::error("gdfl.import", "refused")
                            .with("kind", "options")
                            .with("day", "2021-06-01")
                            .with("code", "malformed_row")
                            .with("reason", "row_refused"),
                    );
                    written += 1;
                }
                written
            });
            for (name, query) in SEARCHES {
                let query = query.replace("{ts}", &ts);
                let mut during: Vec<u128> = (0..200)
                    .map(|_| {
                        let start = Instant::now();
                        std::hint::black_box(api::logs::logs_json_over(
                            Some(&served),
                            root,
                            &query,
                        ));
                        start.elapsed().as_nanos()
                    })
                    .collect();
                print_row(
                    &format!("{id} {name}, {lines}/dir, import writing"),
                    &mut during,
                );
            }
            writing.store(false, std::sync::atomic::Ordering::Relaxed);
            let written = writer.join().unwrap_or(0);
            println!("  {id} the import wrote {written} refusal(s) while those rows ran");
        });
    }
    // THE LIVE TAIL: the installed sink's copy in memory, beside the same
    // request over a directory no installed sink writes (its files).
    let live = root_of("live");
    let Ok(sink) = telemetry::install(&telemetry::Config::new(&live)) else {
        refuse("the bench's sink would not install");
    };
    for n in 0..POSTING_SIZES[1] {
        let _ = sink.emit(&telemetry::Event::info("api.bench", "a line").with("n", n));
    }
    let files = root_of("live-files");
    fill(&files.join("logs"), "served", POSTING_SIZES[1]);
    for (label, dir) in [
        ("live tail from memory", &live),
        ("tail from files", &files.join("logs")),
    ] {
        let mut ns: Vec<u128> = (0..200)
            .map(|_| {
                let start = Instant::now();
                std::hint::black_box(api::logs::logs_json_over(Some(dir), &files, "limit=50"));
                start.elapsed().as_nanos()
            })
            .collect();
        print_row(
            &format!("{id} {label}, {} lines", POSTING_SIZES[1]),
            &mut ns,
        );
    }
    for (_, root) in stores {
        let _ignored = std::fs::remove_dir_all(root);
    }
    let _ignored = std::fs::remove_dir_all(files);
    ok
}

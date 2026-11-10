//! The o1p99 attack on `crates/api` (cloud lane `cloud/o1p99-attack-20261110`).
//!
//! `benches/ratio/logs_dirs.rs` times `/logs.json` KEYED (C-LG-01, the
//! durable index) at 200 and 2,000 lines a log directory and gates the
//! median. `crates/api/src/logs.rs` says of the rest: "A request cannot make
//! the server read the disk without a bound" and "only the fallback reads
//! the files". This file times both kinds per call through the public
//! `api::logs::logs_json_over` (what the route runs) at 200, 2,000, 20,000
//! and 100,000 lines in each of the eight log directories, and prints p50
//! and p99. See `o1p99/harness.rs` and `O1-ATTACK-REPORT.md`.
//!
//! * `O1P99-API-01` keyed, `run=<id>&durable=1&limit=20`: GATED, flat in
//!   the lines each directory holds.
//! * `O1P99-API-02` a plain filter that matches no line, `run=1&limit=20`
//!   (no `durable=1`, so it stays the ring's filter and falls back to reading
//!   the files). PRINTED, NOT GATED: it grows with the log bytes until each
//!   directory's `SCAN_BYTES` (4 MiB) cap, eight directories a request,
//!   which is a bound and not a flat cost.
//!
//! The routes this file times are owned by `benches/ratio.rs`'s `ROUTES`
//! (`/logs.json`); it declares none of its own.

use std::hint::black_box;
use std::path::{Path, PathBuf};

#[allow(
    dead_code,
    reason = "the harness is copied into every crate's o1p99 bench and not every bench calls every helper"
)]
#[path = "o1p99/harness.rs"]
mod harness;

use harness::{FACTOR_PERMILLE, measure, verdict};

/// The run every fixture line belongs to.
const RUN: u64 = 1_791_700_000;

/// Lines in each of the eight log directories.
const SIZES: [u64; 4] = [200, 2_000, 20_000, 100_000];

/// A setup failure this bench cannot measure past.
fn refuse(why: &str) -> ! {
    println!("BENCH SETUP FAILED — {why}");
    std::process::exit(1)
}

/// `lines` audited importer refusals of [`RUN`] written to `dir` through a
/// real sink, as `benches/ratio/logs_dirs.rs` writes them.
fn fill(dir: &Path, kind: &str, lines: u64) {
    if std::fs::create_dir_all(dir).is_err() {
        refuse("a log directory would not create")
    }
    let Ok(sink) = telemetry::Sink::open(&telemetry::Config::new(dir)) else {
        refuse("a bench sink would not open")
    };
    for n in 0..lines {
        let day = format!("2021-{:02}-{:02}", 1 + (n / 28) % 12, 1 + n % 28);
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

/// A store whose eight log directories each hold `lines` lines.
fn logs_store(lines: u64) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "brutex-o1p99-api-logs-{lines}-{}",
        std::process::id()
    ));
    let _ignored = std::fs::remove_dir_all(&root);
    let logs = root.join("logs");
    fill(&logs, "served", lines);
    for dir in std::iter::once(telemetry::run_dirs::CLI)
        .chain(telemetry::run_dirs::ALL.iter().map(|d| d.sub))
    {
        let kind = telemetry::run_dirs::GDFL_KINDS
            .iter()
            .find(|own| own.sub == dir)
            .and_then(|own| own.gdfl_kind)
            .unwrap_or("indices");
        fill(&logs.join(dir), kind, lines);
    }
    root
}

/// The answer's length, so the call cannot be optimised away.
fn ask(root: &Path, query: &str) -> u64 {
    let served = root.join("logs");
    u64::try_from(api::logs::logs_json_over(Some(&served), root, black_box(query)).len())
        .unwrap_or(0)
}

fn main() {
    let clock = harness::clock_ps();
    println!(
        "o1p99 attack — crates/api, clock {} ns a pair",
        harness::ns(clock)
    );
    let mut roots: Vec<PathBuf> = SIZES
        .iter()
        .map(|&n| {
            let started = std::time::Instant::now();
            let root = logs_store(n);
            println!(
                "O1P99|O1P99-API|fixture|n={n}|eight directories written in {} ms",
                started.elapsed().as_millis()
            );
            root
        })
        .collect();
    let keyed = format!("run={RUN}&durable=1&limit=20");
    let tails = measure(
        &mut roots,
        5,
        200,
        1,
        clock,
        |_| {},
        |root, _| ask(root, &keyed),
    );
    let ok = verdict(
        "O1P99-API-01",
        "/logs.json keyed (durable index) by lines a directory",
        &SIZES,
        &tails,
        FACTOR_PERMILLE,
    );
    let tails = measure(
        &mut roots,
        4,
        25,
        1,
        clock,
        |_| {},
        |root, _| ask(root, "run=1&limit=20"),
    );
    let _printed = verdict(
        "O1P99-API-02",
        "/logs.json plain filter matching nothing, by lines a directory (printed)",
        &SIZES,
        &tails,
        u128::MAX,
    );
    for root in roots {
        let _ignored = std::fs::remove_dir_all(root);
    }
    if ok {
        println!("every gated o1p99 row held its ceiling");
    } else {
        println!("AN o1p99 ROW GREW PAST ITS CEILING — see the VERDICT lines marked GROWS.");
        std::process::exit(1);
    }
}

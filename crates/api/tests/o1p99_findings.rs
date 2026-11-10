//! Findings of the o1p99 attack on `crates/api` (cloud lane
//! `cloud/o1p99-attack-20261110`, 10 Nov 2026). `O1-ATTACK-REPORT.md` at the
//! repository root has the measurements, the likely cause and a proposed fix.
//!
//! EVERY TEST HERE IS `#[ignore]`d BECAUSE IT FAILS on the code it was
//! written against. Run by hand, in the optimised test profile:
//!
//! ```text
//! cargo test --locked -p api --test o1p99_findings -- --ignored --nocapture
//! ```
//!
//! Un-ignore a test when its fix lands; it is then the regression guard.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::path::{Path, PathBuf};

#[allow(
    dead_code,
    reason = "the bench harness is shared with the findings and not every helper is called here"
)]
#[path = "../benches/o1p99/harness.rs"]
mod harness;

use harness::{FACTOR_PERMILLE, measure, permille, verdict};

/// The run every fixture line belongs to.
const RUN: u64 = 1_791_700_000;

/// `lines` audited importer refusals of [`RUN`] written to `dir` through a
/// real sink, as `benches/ratio/logs_dirs.rs` writes them.
fn fill(dir: &Path, kind: &str, lines: u64) {
    std::fs::create_dir_all(dir).unwrap();
    let sink = telemetry::Sink::open(&telemetry::Config::new(dir)).unwrap();
    for n in 0..lines {
        let day = format!("2021-{:02}-{:02}", 1 + (n / 28) % 12, 1 + n % 28);
        let event = telemetry::Event::error("gdfl.import", "refused")
            .with("run", RUN)
            .with("kind", kind)
            .with("day", day.as_str())
            .with("about", "ZZ BENCH FIXTURE.csv")
            .with("code", "malformed_row")
            .with("reason", "row_refused");
        assert!(sink.emit(&event).is_written());
    }
}

/// A store whose eight log directories each hold `lines` lines.
fn logs_store(lines: u64) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "brutex-o1p99-finding-logs-{lines}-{}",
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

/// FINDING O1P99-F-API-01 — an unkeyed `/logs.json` filter costs the log's
/// size, up to eight 4 MiB scans a request.
///
/// Claim: `crates/api/src/logs.rs`, "A request cannot make the server read
/// the disk without a bound", and the ring answer "constant ... only the
/// fallback reads the files". True as a bound: `run=<id>` without
/// `durable=1` stays the ring's filter, and a filter that matches too few
/// lines falls back to `telemetry::tail` over each of the eight directories,
/// up to `logs::SCAN_BYTES` (4 MiB) each. So its p99 grows with the log until
/// that cap. Measured on the attack host, `run=1&limit=20` (no line has run
/// 1): quietest-round p99 2.6 ms at 200 lines a directory, 26.8 ms at 2,000,
/// 209 ms at 20,000 and 207 ms at 100,000 (the cap). The keyed form
/// (`durable=1`) holds within 2.0x over the same sizes.
#[test]
#[ignore = "o1p99 finding O1P99-F-API-01: fails today, an unkeyed /logs.json filter reads up to 8 x 4 MiB"]
fn finding_api_unkeyed_logs_filter_costs_the_log_size_up_to_the_scan_cap() {
    let sizes: [u64; 2] = [200, 20_000];
    let clock = harness::clock_ps();
    let mut roots: Vec<PathBuf> = sizes.iter().map(|&n| logs_store(n)).collect();
    let tails = measure(
        &mut roots,
        4,
        10,
        1,
        clock,
        |_| {},
        |root, _| {
            let served = root.join("logs");
            u64::try_from(api::logs::logs_json_over(Some(&served), root, "run=1&limit=20").len())
                .unwrap()
        },
    );
    let flat = verdict(
        "O1P99-F-API-01",
        "/logs.json run=1 (matches nothing): 200 -> 20,000 lines a directory",
        &sizes,
        &tails,
        FACTOR_PERMILLE,
    );
    for root in roots {
        let _ignored = std::fs::remove_dir_all(root);
    }
    assert!(
        flat,
        "the unkeyed filter costs {} at 20,000 lines a directory what it costs at 200",
        harness::times(permille(tails[0].quiet_p99_ps, tails[1].quiet_p99_ps))
    );
}

//! A cli invocation's ledger lines name its run (AG-02, finding stocks-7):
//! [`super::RunClaim`] holds the invocation's attempt id as the telemetry
//! sink's run, so the `run=` every pull ledger line stamps from that sink is
//! the attempt id and not the constant `0` 34,014 `gdfl.repairs`, 47,140
//! `zerodha.repairs` and 76,476 `zerodha.gaps` lines carried. The lines are
//! written by the real `cli zerodha-repair` path on invented stores; every
//! price is made up.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "a test that cannot panic cannot fail"
)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use pull::fetch::{RawRow, RawWindow};
use pull::session::{Day, IST_OFFSET_SECS, Window};

use super::{Origin, RunClaim, begin, claim_invocation_run, current_id};
use crate::OK;

/// The sink's run key is process-wide and these are the only tests in this
/// binary that claim it on the installed sink, so they take turns: a test
/// that reads `run=0` must not overlap one that holds a run.
static SERIAL: Mutex<()> = Mutex::new(());

static NEXT: AtomicU64 = AtomicU64::new(0);

/// A store root of this test's own, removed when it drops.
struct Root(PathBuf);

impl Root {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "brutex-cli-runclaim-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        Self(root)
    }
}

impl Drop for Root {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn day(y: u16, m: u8, d: u8) -> Day {
    Day::new(y, m, d).unwrap()
}

/// Every minute the calendar of record owes on `from..=to`, invented.
fn answer(from: Day, to: Day) -> Vec<RawRow> {
    let mut rows = Vec::new();
    for number in from.days_from_epoch()..=to.days_from_epoch() {
        let kind = pull::session::with_recorded_window(
            i64::from(number),
            pull::calendar::sourced_kind_of(i64::from(number)),
        );
        let pull::calendar::DayKind::Open(session) = kind else {
            continue;
        };
        for minute in (0..1_440_u16).filter(|m| session.expects(*m)) {
            let base = 3_100_000 + i64::from(minute % 83) * 11;
            rows.push(RawRow {
                timestamp: i64::from(number) * 86_400 - IST_OFFSET_SECS + i64::from(minute) * 60,
                open: base,
                high: base + 20,
                low: base - 20,
                close: base + 1,
                volume: 0,
                open_interest: None,
            });
        }
    }
    rows
}

/// Lands an ordinary minute answer for `symbol` under `root`.
fn land(root: &Path, symbol: &str, from: Day, to: Day) {
    let request = pull::fetch::BarRequest {
        instrument_id: String::new(),
        listing: pull::vendor::Listing::Index,
        window: Window::new(from, to).unwrap(),
        granularity: pull::vendor::Granularity::Minute1,
    };
    let plan = pull::ingest::Plan {
        calendar: pull::calendar::Runtime::default(),
        cash_schedule: None,
        columns: pull::csv::Columns::Gdfl,
        request: &request,
        encoding: pull::vendor::TimestampEncoding::EpochSecondsUtc,
        scale: pull::vendor::PriceScale::Paisa,
        vendor: brutex_core::vendor::Vendor::Zerodha,
        exchange: "NSE",
        segment: "INDEX",
        contract: None,
    };
    let done = pull::ingest::from_window(
        &RawWindow {
            rows: answer(from, to),
        },
        symbol,
        "fixture",
        root,
        plan,
    );
    assert!(done.failures.is_empty(), "{:#?}", done.failures);
}

/// One `cli zerodha-repair` back-fill of December 2020's first half from a
/// second store, and the `run=` of every line it appended to the store's
/// `audit/zerodha.repairs`.
fn repair_runs(symbol: &str) -> Vec<String> {
    let store = Root::new();
    let source = Root::new();
    land(&store.0, symbol, day(2020, 12, 16), day(2020, 12, 31));
    land(&source.0, symbol, day(2020, 12, 1), day(2020, 12, 31));
    let source_root = source.0.display().to_string();
    let mut out = String::new();
    let root = store.0.clone();
    let code = crate::zerodha_repair::command_with(
        &[
            symbol,
            "2020-12-01",
            "2020-12-31",
            "--from-store",
            source_root.as_str(),
            "--reason",
            "fixture run claim",
        ],
        &move || Ok(root.clone()),
        &mut out,
    );
    assert_eq!(code, OK, "{out}");
    let journal = fs::read_to_string(store.0.join("audit").join("zerodha.repairs")).unwrap();
    let runs: Vec<String> = journal
        .lines()
        .map(|line| {
            line.rsplit(' ')
                .next()
                .filter(|last| last.starts_with("run="))
                .unwrap_or_else(|| panic!("a line with no run= last: {line:?}"))
                .to_owned()
        })
        .collect();
    assert!(runs.len() >= 2, "a repairing and a repair line: {journal}");
    runs
}

/// THE FINDING'S LINE, NAMING ITS RUN: the attempt id the cli computes for a
/// command (`run_with_sink`: the durable invocation's id, else one reserved
/// from the sink), claimed for the command, is the `run=` of every repair
/// line the command writes, and the sink's run is zero again after it.
#[test]
fn a_cli_written_repair_line_carries_the_invocations_run() {
    let _turn = SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let sink = crate::ledger_all::tests::sink();
    assert_eq!(sink.run(), 0, "no run is held before the invocation");

    // A NON-SWEEP COMMAND: no durable invocation, so the attempt is one
    // reserved from the sink, exactly as `run_with_sink` reserves it.
    let attempt = current_id().or_else(|| sink.reserve_run_id());
    let claim = claim_invocation_run(attempt);
    assert!(claim.held());
    let id = attempt.unwrap();
    assert_ne!(id, 0);
    for run in repair_runs("RUNIDXA") {
        assert_eq!(run, format!("run={id}"));
    }
    drop(claim);
    assert_eq!(sink.run(), 0, "released with the invocation");

    // A SWEEP COMMAND'S DURABLE INVOCATION: its id is the attempt.
    let audit_root = Root::new();
    let mut audit = begin(&audit_root.0, Origin::Cli, "zerodha-repair").unwrap();
    let durable = audit.id();
    audit.enter(|| {
        let attempt = current_id().or_else(|| sink.reserve_run_id());
        assert_eq!(attempt, Some(durable));
        let _claim = claim_invocation_run(attempt);
        for run in repair_runs("RUNIDXB") {
            assert_eq!(run, format!("run={durable}"));
        }
    });
    audit
        .finish(super::Phase::Completed, 0)
        .expect("terminal audit");
    assert_eq!(sink.run(), 0);
}

/// D-0088: zero is the absence of a run. A process with no attempt claims
/// nothing and its lines say `run=0`, as they always did.
#[test]
fn a_process_with_no_attempt_writes_run_0() {
    let _turn = SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let sink = crate::ledger_all::tests::sink();
    let claim = claim_invocation_run(None);
    assert!(!claim.held());
    assert_eq!(claim.run(), 0);
    for run in repair_runs("RUNIDXC") {
        assert_eq!(run, "run=0");
    }
    drop(claim);
    assert_eq!(sink.run(), 0);
}

/// TWO INVOCATIONS IN ONE PROCESS: the second claims its own id once the
/// first released, and a claim made while another run is held takes
/// nothing and releases nothing.
#[test]
fn two_invocations_in_one_process_each_name_their_own_run() {
    let _turn = SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let sink = crate::ledger_all::tests::sink();
    let first = sink.reserve_run_id();
    let second = sink.reserve_run_id();
    assert_ne!(first, second);
    {
        let _claim = claim_invocation_run(first);
        for run in repair_runs("RUNIDXD") {
            assert_eq!(run, format!("run={}", first.unwrap()));
        }
        // NESTED: a claim of another id while the first is held is a no-op,
        // and dropping it leaves the first holding the key.
        let inner = claim_invocation_run(second);
        assert!(!inner.held());
        drop(inner);
        assert_eq!(Some(sink.run()), first);
    }
    assert_eq!(sink.run(), 0);
    {
        let _claim = claim_invocation_run(second);
        for run in repair_runs("RUNIDXE") {
            assert_eq!(run, format!("run={}", second.unwrap()));
        }
    }
    assert_eq!(sink.run(), 0);
}

/// The guard's own edges on a private sink: no sink, no id and a zero id
/// claim nothing; a held run is released on drop; an ambient run another
/// caller holds is never replaced and never released.
#[test]
fn a_run_claim_takes_only_a_free_key_and_releases_only_its_own() {
    let dir = std::env::temp_dir().join(format!(
        "brutex-cli-runclaim-sink-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&dir);
    let sink = telemetry::Sink::open(&telemetry::Config::new(&dir)).expect("private sink");
    assert!(!RunClaim::take(None, Some(5)).held());
    assert!(!RunClaim::take(Some(&sink), None).held());
    assert!(!RunClaim::take(Some(&sink), Some(0)).held());
    {
        let claim = RunClaim::take(Some(&sink), Some(5));
        assert!(claim.held());
        assert_eq!((claim.run(), sink.run()), (5, 5));
    }
    assert_eq!(sink.run(), 0, "released on drop");
    assert!(sink.claim_run(777));
    {
        let claim = RunClaim::take(Some(&sink), Some(5));
        assert!(!claim.held());
    }
    assert_eq!(
        sink.run(),
        777,
        "the ambient run is neither replaced nor released"
    );
    sink.release_run(777);
    let _ = fs::remove_dir_all(&dir);
}

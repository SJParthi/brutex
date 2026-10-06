#![cfg(test)]
//! Generated finite stores exercise batch publication and its refusal ledger.
#![allow(clippy::expect_used)]

use super::*;
use crate::results::Results;
use crate::sweep_evidence::{self, Completion};
use std::fs;
use std::path::Path;

const COMMIT: &str = "generated-batch-publication-fixture";

fn held(root: &Path, rung: &str) -> Held {
    catalog::walk(root)
        .expect("fixture census")
        .held
        .into_iter()
        .find(|held| {
            held.vendor == brutex_core::vendor::Vendor::Zerodha
                && held.symbol == "NIFTY"
                && held.timeframe.as_str() == rung
                && held.month.year() == 2025
                && held.month.month() == 5
        })
        .expect("requested generated month")
}

fn require_completed(row: &Row) {
    assert!(row.refused.is_none(), "{:?}", row.refused);
    assert!(row.completed);
    assert!(row.bars > 0);
    assert_eq!(row.kept, 0);
    assert_eq!(row.identity.as_ref().expect("run identity").len(), 64);
}

#[test]
fn stored_batch_publication_reconciles_exact_retries_and_keeps_other_feeds_out() {
    let _knobs = crate::knobs::serially();
    crate::knobs::clear_all();
    for rung in ["1min", "5min"] {
        crate::audited_stored::with_warmed_store(|root| {
            let held = held(root, rung);
            let first = one(root, &held, u64::MAX, COMMIT);
            require_completed(&first);
            let mut ledger = Results::open_read(root).expect("recorded ledger");
            assert_eq!(ledger.len().expect("parent count"), 1);
            let record = ledger.read(0).expect("acknowledged parent");
            assert_eq!(Some(crate::identity_hex(&record.identity)), first.identity);
            assert_eq!(record.bars, first.bars);
            assert_eq!(record.min_hits, u64::MAX);
            assert_eq!(
                (record.trades, record.combinations, record.halted),
                (0, 0, 0)
            );
            assert_eq!(crate::results::read_field(&record.timeframe), rung);
            drop(ledger);
            let before = fs::read(Results::path(root)).expect("parent bytes");
            let first_evidence = sweep_evidence::latest(root, 1_048_576)
                .expect("authenticated evidence")
                .expect("attempt recorded");
            assert_eq!(first_evidence.identity, record.identity);
            assert_eq!(first_evidence.completion, Completion::Completed);

            let retry = one(root, &held, u64::MAX, COMMIT);
            require_completed(&retry);
            assert_eq!(retry.identity, first.identity);
            assert_eq!(retry.bars, first.bars);
            let retry_evidence = sweep_evidence::latest(root, 1_048_576)
                .expect("retry evidence")
                .expect("retry attempt");
            assert!(retry_evidence.attempt > first_evidence.attempt);
            assert_eq!(retry_evidence.identity, first_evidence.identity);
            assert_eq!(retry_evidence.completion, Completion::Completed);
            assert_eq!(
                fs::read(Results::path(root)).expect("retry parents"),
                before
            );

            let other = root.join(format!("bars/dhan/NSE/INDEX/NIFTY/{rung}"));
            fs::create_dir_all(&other).expect("owned other-feed directory");
            fs::write(other.join("2025-05.bin"), b"not a bar file")
                .expect("owned other-feed malformed file");
            let report = sweep_under(root, "zerodha", rung, u64::MAX, COMMIT)
                .expect("whole-catalog execution");
            assert!(report.contains("1 swept"), "{report}");
            assert!(report.contains(first.identity.as_ref().expect("identity")));
            assert!(!report.contains("REFUSED  dhan"));
            assert!(!report.contains("DOES NOT RECONCILE"));
            assert_eq!(
                sweep_under(root, "zerodha", rung, u64::MAX, COMMIT).expect("whole-catalog retry"),
                report
            );
            assert_eq!(
                fs::read(Results::path(root)).expect("catalog parents"),
                before
            );
        });
    }
    crate::knobs::clear_all();
}

#[test]
fn batch_missing_required_context_refuses_before_creating_a_parent_or_attempt() {
    for relative in [
        "bars/zerodha/NSE/INDEX/NIFTY/1day/2025-04.bin",
        "bars/zerodha/NSE/INDEX/NIFTY/1min/2025-05.bin",
    ] {
        crate::audited_stored::with_warmed_store(|root| {
            let held = held(root, "5min");
            fs::remove_file(root.join(relative)).expect("remove only owned required context");
            let row = one(root, &held, u64::MAX, COMMIT);
            assert!(row.refused.is_some());
            assert!(row.identity.is_none());
            assert!(!row.completed);
            assert_eq!((row.bars, row.depth, row.kept), (0, 0, 0));
            assert!(!Results::path(root).exists());
            assert_eq!(
                sweep_evidence::latest(root, 1_048_576).expect("no evidence"),
                None
            );
        });
    }
}

#[test]
fn batch_publication_failure_is_durable_refusal_and_a_repaired_retry_can_complete() {
    let _knobs = crate::knobs::serially();
    crate::knobs::clear_all();
    crate::audited_stored::with_warmed_store(|root| {
        let held = held(root, "1min");
        let parent = Results::path(root);
        fs::create_dir_all(&parent).expect("owned publication obstruction");
        let refused = one(root, &held, u64::MAX, COMMIT);
        assert!(
            refused
                .refused
                .as_ref()
                .expect("publication refusal")
                .contains("not recorded")
        );
        assert!(refused.bars > 0);
        assert!(refused.completed);
        let evidence = sweep_evidence::latest(root, 1_048_576)
            .expect("refusal evidence")
            .expect("refused attempt");
        assert_eq!(evidence.completion, Completion::Refused);
        assert_eq!(
            Some(crate::identity_hex(&evidence.identity)),
            refused.identity
        );
        assert!(parent.is_dir());
        fs::remove_dir(&parent).expect("remove only owned empty obstruction");
        let recovered = one(root, &held, u64::MAX, COMMIT);
        require_completed(&recovered);
        assert_eq!(recovered.identity, refused.identity);
        let completed = sweep_evidence::latest(root, 1_048_576)
            .expect("restored evidence")
            .expect("completed attempt");
        assert_eq!(completed.completion, Completion::Completed);
        assert!(completed.attempt > evidence.attempt);
        assert_eq!(
            Results::open_read(root)
                .expect("restored parent")
                .len()
                .expect("count"),
            1
        );
    });
    crate::knobs::clear_all();
}

/// **A chunk files its months in INPUT order, whatever order they finish
/// in.** GAP13-13, D-1701.
///
/// Three instrument-months sweep in one chunk while the FIRST is held back,
/// so it finishes last. Its ledger row is still row 0, the attempt tokens rise
/// in input order, and the journal's last terminal is the last month's.
/// Before D-1701 each worker appended its own row and began its own attempt,
/// so the held-back month was filed last.
#[test]
fn a_chunk_files_its_months_in_input_order_whatever_order_they_finish() {
    let _knobs = crate::knobs::serially();
    crate::knobs::clear_all();
    crate::audited_stored::with_warmed_store_of(&["NIFTY", "BANKNIFTY", "RELIANCE"], |root| {
        let wanted: Vec<Held> = catalog::walk(root)
            .expect("fixture census")
            .held
            .into_iter()
            .filter(|held| {
                held.timeframe.as_str() == "1min"
                    && held.month.year() == 2025
                    && held.month.month() == 5
            })
            .collect();
        assert_eq!(wanted.len(), 3, "premise: three May months");
        let chunk: Vec<&Held> = wanted.iter().collect();
        slow_symbol(Some(chunk.first().expect("a first month").symbol.as_str()));
        let rows = sweep_chunk(root, &chunk, u64::MAX, COMMIT);
        slow_symbol(None);
        assert_eq!(rows.len(), 3);
        for row in &rows {
            require_completed(row);
        }
        let mut ledger = Results::open_read(root).expect("ledger");
        assert_eq!(ledger.len().expect("rows"), 3);
        let mut previous = 0;
        for (index, row) in (0_u64..).zip(&rows) {
            let identity = row.identity.as_ref().expect("identity");
            assert_eq!(
                &ledger.read(index).expect("row").identity_hex(),
                identity,
                "ledger row {index} is input month {index}"
            );
            let bytes: Vec<u8> = (0..32)
                .map(|i| u8::from_str_radix(&identity[i * 2..i * 2 + 2], 16).expect("hex"))
                .collect();
            let evidence =
                sweep_evidence::read(root, bytes.try_into().expect("32 bytes"), 1_048_576)
                    .expect("evidence")
                    .expect("its attempt");
            assert_eq!(evidence.completion, Completion::Completed);
            assert!(
                evidence.attempt > previous,
                "attempt tokens rise in input order"
            );
            previous = evidence.attempt;
        }
        let last = sweep_evidence::latest(root, 1_048_576)
            .expect("journal")
            .expect("a terminal");
        assert_eq!(
            Some(crate::identity_hex(&last.identity)),
            rows.last().and_then(|row| row.identity.clone()),
            "the last terminal journaled is the last input month's"
        );
    });
    crate::knobs::clear_all();
}

/// A chunk whose every month refuses before identification begins nothing and
/// files nothing; the refusals keep input order. D-1701.
#[test]
fn a_chunk_of_unidentified_months_begins_no_attempt() {
    crate::audited_stored::with_warmed_store(|root| {
        let april: Vec<Held> = catalog::walk(root)
            .expect("fixture census")
            .held
            .into_iter()
            .filter(|held| held.timeframe.as_str() == "1min" && held.month.month() == 4)
            .collect();
        assert!(!april.is_empty(), "premise: an April month");
        let chunk: Vec<&Held> = april.iter().collect();
        let rows = sweep_chunk(root, &chunk, u64::MAX, COMMIT);
        assert_eq!(rows.len(), chunk.len());
        for row in &rows {
            assert!(row.refused.is_some() && row.identity.is_none() && !row.ran);
        }
        assert!(!Results::path(root).exists());
        assert_eq!(
            sweep_evidence::latest(root, 1_048_576).expect("no evidence"),
            None
        );
    });
}

/// AC-whp-law-0 and AC-whp-law-2, D-1661: one stored month swept through
/// `sweep-all`'s `one` and through `sweep-stored` — typed in lower case —
/// files two ledger rows that agree on the two fields readers compare across
/// doors. `bars` is the column's swept count, warm-up excluded, on both: it was
/// `loaded.bars.len()` on `sweep-stored`, which counts warming bars the sweep
/// never folded. `underlying` is the canonical key on both: it was the typed
/// word on `sweep-stored`, while the run identity already used the key.
#[test]
fn sweep_all_and_sweep_stored_record_the_same_swept_bars_and_canonical_name() {
    let _knobs = crate::knobs::serially();
    crate::knobs::clear_all();
    crate::audited_stored::with_warmed_store(|root| {
        let row = one(root, &held(root, "5min"), u64::MAX, COMMIT);
        require_completed(&row);
        let report = crate::sweep_stored_kernel(crate::StoredSweepRequest {
            root: root.to_path_buf(),
            vendor: brutex_core::vendor::Vendor::Zerodha,
            underlying: "nifty",
            rung: "5min",
            year: 2025,
            month: 5,
            min_hits: u64::MAX,
            commit: COMMIT,
        })
        .expect("the lower-case word names the same stored month");
        assert!(report.contains("RESULT RECORDED"), "{report}");
        let mut ledger = Results::open_read(root).expect("two parents");
        assert_eq!(ledger.len().expect("parent count"), 2);
        let all = ledger.read(0).expect("sweep-all parent");
        let stored = ledger.read(1).expect("sweep-stored parent");
        assert_eq!(stored.bars, all.bars, "one month, one swept count");
        assert_eq!(all.bars, row.bars);
        assert_eq!(crate::results::read_field(&stored.underlying), "NIFTY");
        assert_eq!(crate::results::read_field(&all.underlying), "NIFTY");
        // Strictly fewer than the month's bars: warm-up is not swept.
        let loaded = crate::stored::load(
            root,
            brutex_core::vendor::Vendor::Zerodha,
            "NIFTY",
            "5min",
            2025,
            5,
        )
        .expect("the month");
        assert!(stored.bars < u64::try_from(loaded.bars.len()).expect("fits"));
    });
}
/// The identities a report prints, in the order it prints them.
fn report_identities(report: &str) -> Vec<String> {
    report
        .lines()
        .filter_map(|line| line.trim().strip_prefix("identity "))
        .map(str::to_owned)
        .collect()
}

/// audit-20261003 hunt-conc-1 (GAP13-13): a whole-store sweep writes its
/// ledger rows, and allocates its evidence attempts, in the walk's own order
/// rather than in the order worker threads happen to finish. Eight months
/// sweep in parallel; the ledger must list them exactly as the report does,
/// and the newest evidence attempt must belong to the last month offered.
#[test]
fn a_whole_store_sweep_files_its_rows_in_walk_order_not_thread_order() {
    const SYMBOLS: [&str; 8] = [
        "NIFTY",
        "BANKNIFTY",
        "RELIANCE",
        "TCS",
        "INFY",
        "SBIN",
        "HDFCBANK",
        "ITC",
    ];
    let _knobs = crate::knobs::serially();
    crate::knobs::clear_all();
    for round in 0..3 {
        crate::audited_stored::with_warmed_store_of(&SYMBOLS, |root| {
            let report = rayon::ThreadPoolBuilder::new()
                .num_threads(8)
                .build()
                .expect("an eight-worker pool")
                .install(|| sweep_under(root, "zerodha", "5min", u64::MAX, COMMIT))
                .expect("the walk completes");
            let printed = report_identities(&report);
            assert_eq!(printed.len(), SYMBOLS.len(), "round {round}: {report}");
            let mut ledger = Results::open_read(root).expect("recorded ledger");
            let filed: Vec<String> = (0..ledger.len().expect("rows"))
                .map(|index| crate::identity_hex(&ledger.read(index).expect("row").identity))
                .collect();
            assert_eq!(
                filed, printed,
                "round {round}: ledger order follows threads"
            );
            let newest = sweep_evidence::latest(root, 1_048_576)
                .expect("evidence")
                .expect("an attempt");
            assert_eq!(
                Some(&crate::identity_hex(&newest.identity)),
                printed.last(),
                "round {round}: the last attempt token went to another month"
            );
        });
    }
    crate::knobs::clear_all();
}

/// BA-05 (P12-06, D-1795): `cli` emits progress per INSTRUMENT-MONTH, never
/// per bar. Three generated months sweep; exactly one `stored month swept`
/// event lands for each identity the report prints, each event's own `bars`
/// field says it stood for a whole month of bars, and no `cli.sweep` record
/// of any message carrying those identities comes anywhere near one per bar.
///
/// Read back through the shared sink and `telemetry::tail`, the shipped
/// reader, and filtered on the printed identities: the fixture commit is
/// this test's own, so a concurrent sweep of the same months elsewhere in
/// this binary cannot share an identity with it.
#[test]
fn a_whole_store_sweep_emits_one_progress_event_per_instrument_month_not_per_bar() {
    const SYMBOLS: [&str; 3] = ["NIFTY", "BANKNIFTY", "RELIANCE"];
    let _knobs = crate::knobs::serially();
    crate::knobs::clear_all();
    let from = crate::ledger_all::tests::mark();
    let report = crate::audited_stored::with_warmed_store_of(&SYMBOLS, |root| {
        sweep_under(root, "zerodha", "5min", u64::MAX, "ba05-per-month-events")
            .expect("the walk completes")
    });
    crate::knobs::clear_all();
    let printed = report_identities(&report);
    assert_eq!(printed.len(), SYMBOLS.len(), "{report}");

    let sink = crate::ledger_all::tests::sink();
    let dir = sink
        .path()
        .parent()
        .expect("the sink writes its file inside a directory")
        .to_path_buf();
    let query = telemetry::Query::last(telemetry::MAX_LIMIT).from_target("cli.sweep");
    let mine: Vec<telemetry::Record> = telemetry::tail(&dir, sink.keep_files(), &query)
        .records
        .into_iter()
        .filter(|record| {
            record.seq >= from
                && record
                    .field("identity")
                    .and_then(telemetry::OwnedValue::as_str)
                    .is_some_and(|id| printed.iter().any(|hex| hex == id))
        })
        .collect();
    let swept: Vec<&telemetry::Record> = mine
        .iter()
        .filter(|record| record.message == "stored month swept")
        .collect();
    assert_eq!(swept.len(), SYMBOLS.len(), "one per month: {mine:?}");
    for hex in &printed {
        assert!(
            swept
                .iter()
                .any(|record| crate::ledger_all::tests::says(record, "identity", hex)),
            "every printed identity has its event: {mine:?}"
        );
    }
    let fewest_bars = swept
        .iter()
        .filter_map(|record| record.field("bars").and_then(telemetry::OwnedValue::as_u64))
        .min()
        .expect("every event carries its bar count");
    assert!(fewest_bars > 1, "an event stood for many bars: {mine:?}");
    assert!(
        u64::try_from(mine.len()).expect("fits") < fewest_bars,
        "events are per month, so far fewer than bars: {mine:?}"
    );
}

/// G18-cli-a-02, D-2003: the slow seam holds back exactly the named symbol.
/// `a_chunk_files_its_months_in_input_order_whatever_order_they_finish` passes
/// whichever month is slow, so the seam's own choice is pinned here.
#[test]
fn the_slow_seam_holds_back_exactly_the_named_symbol() {
    assert!(held_back(Some("NIFTY"), "NIFTY"));
    assert!(!held_back(Some("NIFTY"), "BANKNIFTY"));
    assert!(!held_back(None, "NIFTY"));
}

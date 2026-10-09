#![cfg(test)]
//! Generated finite stores exercise batch publication and its refusal ledger.
#![allow(clippy::expect_used, clippy::indexing_slicing)]

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

/// `run` on a rayon pool of exactly `threads`, so a verdict about order never
/// depends on the width of the machine running the test. The slow seam is a
/// thread-local read by `sweep_chunk`'s caller, so a test sets it inside `run`.
fn on_pool<R: Send>(threads: usize, run: impl FnOnce() -> R + Send) -> R {
    rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .expect("a pool")
        .install(run)
}

/// The three May months of `rung` in a store [`with_warmed_store_of`] or
/// [`with_unsourceable_close_of`] wrote, in walk order.
///
/// [`with_warmed_store_of`]: crate::audited_stored::with_warmed_store_of
/// [`with_unsourceable_close_of`]: crate::audited_stored::with_unsourceable_close_of
fn may_months(root: &Path, rung: &str) -> Vec<Held> {
    let wanted: Vec<Held> = catalog::walk(root)
        .expect("fixture census")
        .held
        .into_iter()
        .filter(|held| {
            held.timeframe.as_str() == rung && held.month.year() == 2025 && held.month.month() == 5
        })
        .collect();
    assert_eq!(wanted.len(), 3, "premise: three May months at {rung}");
    wanted
}

/// The identities of the shared journal's TERMINAL rows, in journal order:
/// each attempt's second row, its first being the start `begin` allocated.
fn journaled_terminals(root: &Path) -> Vec<[u8; 32]> {
    let journal = fs::read(
        root.join("results")
            .join("sweep-evidence-v1")
            .join("attempts.bin"),
    )
    .expect("the shared journal");
    let mut seen = std::collections::HashMap::new();
    let mut terminals = Vec::new();
    for row in journal.get(16..).expect("a header").chunks(96) {
        let identity: [u8; 32] = row[8..40].try_into().expect("32 bytes");
        let token: [u8; 8] = row[..8].try_into().expect("8 bytes");
        let rows = seen.entry(token).or_insert(0_u8);
        *rows += 1;
        if *rows == 2 {
            terminals.push(identity);
        }
    }
    terminals
}

/// A 64-character identity as its bytes.
fn identity_bytes(hex: &str) -> [u8; 32] {
    let bytes: Vec<u8> = (0..32)
        .map(|i| u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).expect("hex"))
        .collect();
    bytes.try_into().expect("32 bytes")
}

/// **A chunk files its months in INPUT order, whatever order they finish
/// in, on a three-thread pool and on a one-thread pool.** GAP13-13, D-1701,
/// D-4702.
///
/// Three instrument-months sweep in one chunk while the FIRST is held back,
/// so it finishes last. Its ledger row is still row 0, the attempt tokens rise
/// in input order, and the journal's last terminal is the last month's.
/// Before D-1701 each worker appended its own row and began its own attempt,
/// so the held-back month was filed last. The pool is built here: on the
/// global pool a one-thread runner finished the months in input order anyway
/// and could not see a revert (GAP13-13 test gap, D-4702).
#[test]
fn a_chunk_files_its_months_in_input_order_whatever_order_they_finish() {
    let _knobs = crate::knobs::serially();
    crate::knobs::clear_all();
    for threads in [3, 1] {
        crate::audited_stored::with_warmed_store_of(&["NIFTY", "BANKNIFTY", "RELIANCE"], |root| {
            let wanted = may_months(root, "1min");
            let chunk: Vec<&Held> = wanted.iter().collect();
            let first = chunk.first().expect("a first month").symbol.clone();
            let rows = on_pool(threads, || {
                slow_symbol(Some(first.as_str()));
                let rows = sweep_chunk(root, &chunk, u64::MAX, COMMIT);
                slow_symbol(None);
                rows
            });
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
                    "{threads} thread(s): ledger row {index} is input month {index}"
                );
                let evidence = sweep_evidence::read(root, identity_bytes(identity), 1_048_576)
                    .expect("evidence")
                    .expect("its attempt");
                assert_eq!(evidence.completion, Completion::Completed);
                assert!(
                    evidence.attempt > previous,
                    "{threads} thread(s): attempt tokens rise in input order"
                );
                previous = evidence.attempt;
            }
            let last = sweep_evidence::latest(root, 1_048_576)
                .expect("journal")
                .expect("a terminal");
            assert_eq!(
                Some(crate::identity_hex(&last.identity)),
                rows.last().and_then(|row| row.identity.clone()),
                "{threads} thread(s): the last terminal journaled is the last input month's"
            );
        });
    }
    crate::knobs::clear_all();
}

/// **A begin that refuses partway through a chunk sweeps exactly the months it
/// began and refuses the rest by name.** GAP13-13 test gap, D-1701, D-4702.
///
/// The second month's `starts.bin` is a directory, so `begin_many` makes the
/// first month's start durable and then refuses at the second. The first
/// month still sweeps and is filed; the second and third are refused with the
/// begin's reason and their identities, and file nothing.
#[test]
fn a_begin_refused_partway_sweeps_the_months_it_began_and_names_the_rest() {
    let _knobs = crate::knobs::serially();
    crate::knobs::clear_all();
    crate::audited_stored::with_warmed_store_of(&["NIFTY", "BANKNIFTY", "RELIANCE"], |root| {
        let wanted = may_months(root, "1min");
        let chunk: Vec<&Held> = wanted.iter().collect();
        let second = prepare(root, chunk[1], u64::MAX, COMMIT)
            .map_err(|row| row.refused)
            .expect("premise: the second month is identified")
            .id
            .hex();
        fs::create_dir_all(
            root.join("results")
                .join("sweep-evidence-v1")
                .join(&second)
                .join("starts.bin"),
        )
        .expect("an obstructed start index");
        let rows = sweep_chunk(root, &chunk, u64::MAX, COMMIT);
        assert_eq!(rows.len(), 3);
        require_completed(&rows[0]);
        let why = rows[1]
            .refused
            .clone()
            .expect("the second month is refused");
        assert!(!why.is_empty());
        assert_eq!(rows[1].identity.as_deref(), Some(second.as_str()));
        assert_eq!(
            rows[2].refused.as_deref(),
            Some(why.as_str()),
            "the same begin"
        );
        assert!(rows[2].identity.is_some());
        for row in &rows[1..] {
            assert!(
                !row.ran && !row.completed && row.bars == 0,
                "{:?}",
                row.refused
            );
        }
        let mut ledger = Results::open_read(root).expect("ledger");
        assert_eq!(
            ledger.len().expect("rows"),
            1,
            "only the begun month is filed"
        );
        assert_eq!(
            Some(ledger.read(0).expect("row").identity_hex()),
            rows[0].identity.clone()
        );
    });
    crate::knobs::clear_all();
}

/// **A month whose column build refuses files its Refused terminal in INPUT
/// order, as every other month files.** G1-2, D-4701.
///
/// Three 5min months each miss the closing minutes of one session, so each is
/// loaded, identified and begun, and its column build then refuses. The first
/// is held back. Before D-4701 each refused attempt was dropped inside its
/// rayon worker, whose `Drop` journaled the Refused terminal there, in thread
/// completion order: the held-back month's terminal landed last.
#[test]
fn a_chunk_files_its_column_refusals_in_input_order() {
    let _knobs = crate::knobs::serially();
    crate::knobs::clear_all();
    crate::audited_stored::with_unsourceable_close_of(
        &["NIFTY", "BANKNIFTY", "RELIANCE"],
        5,
        |root| {
            let wanted = may_months(root, "5min");
            let chunk: Vec<&Held> = wanted.iter().collect();
            let first = chunk.first().expect("a first month").symbol.clone();
            let rows = on_pool(3, || {
                slow_symbol(Some(first.as_str()));
                let rows = sweep_chunk(root, &chunk, u64::MAX, COMMIT);
                slow_symbol(None);
                rows
            });
            assert_eq!(rows.len(), 3);
            let mut begun = Vec::new();
            for row in &rows {
                assert!(
                    !row.ran && row.refused.as_ref().is_some_and(|why| !why.is_empty()),
                    "premise: the column build refused: {:?}",
                    row.refused
                );
                begun.push(identity_bytes(row.identity.as_ref().expect("identified")));
            }
            assert!(
                !Results::path(root).exists(),
                "a refused month files no row"
            );
            assert_eq!(
                journaled_terminals(root),
                begun,
                "each begun month's Refused terminal, in input order"
            );
            for identity in begun {
                let evidence = sweep_evidence::read(root, identity, 1_048_576)
                    .expect("evidence")
                    .expect("its attempt");
                assert_eq!(evidence.completion, Completion::Refused);
            }
        },
    );
    crate::knobs::clear_all();
}

/// **A begun month whose column refused is sealed Refused by phase 4, and a
/// seal that fails is named on its row, not swallowed.** G1-2, D-4701.
///
/// `refuse_begun` is driven alone, twice, on one scratch evidence root. A
/// seal that succeeds leaves the row's reason exactly as the column gave it
/// and journals the identity's Refused terminal. With the shared journal
/// replaced by a directory, the seal fails, and the row's reason is the
/// column's followed by the seal's own failure. Leaving the attempt to its
/// `Drop` instead -- what phase 3 did inside the worker until D-4701 --
/// writes the same terminal when it can, so only the failing seal tells the
/// two apart.
#[test]
fn a_refused_months_terminal_is_sealed_and_a_failed_seal_is_named() {
    let root =
        std::env::temp_dir().join(format!("brutex-batch-refuse-begun-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("scratch");
    let refused = |byte: u8| {
        Row::refused_before(
            "zerodha NIFTY 5min 2025-05".to_owned(),
            Some(format!("{byte:02x}").repeat(32)),
            "the column refused".to_owned(),
        )
    };
    let sealed = [7_u8; 32];
    let attempt = sweep_evidence::begin(&root, sealed, sweep_evidence::Operation::Sweep)
        .expect("a begun attempt");
    let row = refuse_begun(refused(7), attempt);
    assert_eq!(row.refused.as_deref(), Some("the column refused"));
    assert!(!row.ran && !row.completed);
    assert_eq!(journaled_terminals(&root), vec![sealed]);
    assert_eq!(
        sweep_evidence::read(&root, sealed, 1_048_576)
            .expect("evidence")
            .expect("its attempt")
            .completion,
        Completion::Refused
    );

    let unsealed = [8_u8; 32];
    let attempt = sweep_evidence::begin(&root, unsealed, sweep_evidence::Operation::Sweep)
        .expect("a second begun attempt");
    let journal = root
        .join("results")
        .join("sweep-evidence-v1")
        .join("attempts.bin");
    let aside = journal.with_extension("aside");
    fs::rename(&journal, &aside).expect("move the journal aside");
    fs::create_dir(&journal).expect("obstruct the journal");
    let row = refuse_begun(refused(8), attempt);
    fs::remove_dir(&journal).expect("clear the obstruction");
    fs::rename(&aside, &journal).expect("restore the journal");
    let why = row.refused.expect("still refused");
    assert!(
        why.starts_with("the column refused; sealing its refused terminal failed: ")
            && why.len() > "the column refused; sealing its refused terminal failed: ".len(),
        "{why}"
    );
    assert_eq!(
        journaled_terminals(&root),
        vec![sealed],
        "no terminal reached the obstructed journal"
    );
    fs::remove_dir_all(&root).expect("scratch removed");
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

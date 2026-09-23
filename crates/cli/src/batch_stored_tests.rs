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

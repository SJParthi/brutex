//! The day module's own arms, at the unit (IA-17b): a gathered day with no
//! close waiting, which through the import's front door only a source that
//! says it holds no file of a day, yet hands one, reaches
//! (`gdfl_gather_tests.rs` drives that one; this reads the arm alone).

#![expect(clippy::expect_used, reason = "a test that cannot panic cannot fail")]

use super::*;
use crate::gdfl_fixtures::scratch;

/// **A GATHERED DAY WHOSE PASS LEFT NO CLOSE STAYS BEGUN, ITS FAILURES NAMED**
/// (IA-RP-33, D-4899 (4)). Should a day reach its months' repair with no close
/// waiting (its pass returned before it, and the run went on), its close is
/// never invented: no line is written, no day counted closed, and the
/// failures its months' repairs gave its files are each named in the report,
/// so a day its pass left begun is read again and nothing it met is silent.
#[test]
fn a_gathered_day_whose_pass_left_no_close_stays_begun_and_its_failures_are_named() {
    let store = scratch("ia17b-unclosed");
    let journal = Journal::load(journal_path(&store)).expect("an empty journal");
    let day = Day::new(2026, 9, 22).expect("a legal day");
    let run = Run {
        kind: ImportKind::Stocks,
        from: day,
        to: day,
        only: &[],
        store_root: &store,
    };
    let ctx = Ctx {
        run: &run,
        journal: &journal,
        options: RunOptions::default(),
        id: 2,
        started: 0,
        announced: std::cell::Cell::new(false),
        scoped: None,
        held_back: std::cell::RefCell::new(HashMap::new()),
        store_id: std::cell::Cell::new(None),
        gather: std::cell::RefCell::new(super::super::filing::Gather::default()),
    };
    let refusal = ImportRefusal::Filing {
        why: "INVENTED.NSE.csv: a test's repair refusal".to_owned(),
    };
    let staged = note_staged(&run, day, "INVENTED.NSE.csv", &refusal);
    let mut report = Report::default();

    close_deferred(
        &ctx,
        super::super::filing::Deferred {
            key: format!("stocks {day} *"),
            day,
            close: None,
            failures: vec![staged],
            committed: 0,
        },
        &mut report,
    );

    assert_eq!(report.failures_total, 1, "the repair's failure is named");
    assert!(
        report
            .recent
            .iter()
            .any(|(_, failure)| failure.why.contains("a test's repair refusal")),
        "{report:?}"
    );
    assert_eq!(report.days_closed, 0, "no close is invented");
    assert_eq!(report.days_imported, 0, "no day is counted imported");
    assert!(
        !journal_path(&store).exists(),
        "no line is written: the day stays begun"
    );
    let _ = std::fs::remove_dir_all(&store);
}

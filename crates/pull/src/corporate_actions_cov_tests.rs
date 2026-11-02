//! BASE-COV-PULL: the refusals, words and folds of `corporate_actions` no test
//! had reached (Route B, recipe v4): field counts, tags, caps, faults of a
//! fold, the wobble window's expiry, and every word a page reads. Every symbol
//! and ratio is invented; nothing here is vendor data.

#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail"
)]

use super::tests::{actions_draft, around, day, load_actions, nse, r, run, runs_draft};
use super::*;

use crate::gdfl_fixtures::scratch;
use crate::reference_data::{Parsed, Receipt, load_one, write};

fn receipt(table: Table, standing: Standing, rows: usize) -> Receipt {
    Receipt {
        table,
        generation: 1,
        source: "invented".to_owned(),
        source_sha256: None,
        fetched: day(2026, 10, 22),
        standing,
        decision: DECISION.to_owned(),
        rows,
        sha256: "a".repeat(64),
    }
}

fn refused(draft: &Draft) -> String {
    write(&scratch("cor-cov-bad"), draft)
        .unwrap_err()
        .to_string()
}

/// `n` one-day runs of one share, one a day from 2000-01-01.
fn dailies(symbol: &str, n: u32) -> Vec<Run> {
    let base = day(2000, 1, 1).days_from_epoch();
    (0..n)
        .map(|i| {
            let on = Day::from_days(base + i).unwrap();
            run(symbol, on, on, "1")
        })
        .collect()
}

#[test]
fn a_ratio_past_the_bound_is_no_ratio_whichever_term_passes() {
    assert_eq!(Ratio::new(MAX_TERM + 1, 1), None);
    assert_eq!(Ratio::new(1, MAX_TERM + 1), None);
    assert!(Ratio::new(MAX_TERM, 1).is_some(), "the bound itself fits");
    assert!(
        Ratio::new(MAX_TERM + 1, MAX_TERM + 1).is_some(),
        "the terms are judged reduced"
    );
    assert_eq!(Ratio::new(u128::MAX, 0), None);
}

#[test]
fn a_signed_fraction_is_a_ratio_only_when_positive_and_it_fits() {
    assert_eq!(Ratio::of_signed(4, 2), Some(r("2")));
    assert_eq!(Ratio::of_signed(-4, -2), Some(r("2")));
    assert_eq!(Ratio::of_signed(-4, 2), None);
    assert_eq!(Ratio::of_signed(4, -2), None);
    assert_eq!(Ratio::of_signed(i128::MIN, 1), None, "no negation of MIN");
    assert_eq!(Ratio::of_signed(1, i128::MIN), None);
    assert_eq!(Ratio::of_signed(0, 0), None);
}

#[test]
fn an_empty_shelf_indexes_nothing_and_spends_no_budget() {
    let mut shelf: Shelf<Run> = Shelf::default();
    let mut budget = 7;
    assert_eq!(
        shelf.index(|row| (row.first, row.last), &mut budget),
        Ok(())
    );
    assert_eq!(budget, 7);
}

/// A row of the wrong width never reaches a field: `render` always writes the
/// table's width, so `build` is called with the parse a damaged reader hands.
#[test]
fn a_row_of_the_wrong_width_is_refused_by_both_tables() {
    let path = Path::new("/invented/table");
    let parsed = |table, standing| Parsed {
        receipt: receipt(table, standing, 1),
        rows: vec![(7, vec!["ACO".to_owned(), "-".to_owned()])],
    };
    let actions =
        CorporateActions::build(&parsed(Table::CorporateActions, Standing::Sourced), path)
            .unwrap_err();
    assert!(
        actions.to_string().contains("wrong field count"),
        "{actions}"
    );
    let runs = BasisRuns::build(&parsed(Table::BasisRuns, Standing::Measured), path).unwrap_err();
    assert!(runs.to_string().contains("wrong field count"), "{runs}");
}

#[test]
fn a_tag_that_is_neither_word_is_refused_and_the_tables_standing_reads_back() {
    let on = day(2023, 7, 3);
    let mut bad = actions_draft(Standing::Sourced, &[nse("ACO", on, Kind::Split, "2")]);
    bad.rows[0][6] = "rumour".to_owned();
    assert!(refused(&bad).contains("\"rumour\" is neither nse nor measured"));
    let store = scratch("cor-cov-standing");
    let good = actions_draft(Standing::Sourced, &[nse("ACO", on, Kind::Split, "2")]);
    write(&store, &good).unwrap();
    assert_eq!(
        load_actions(&store).table().unwrap().standing(),
        Standing::Sourced
    );
}

/// Two shares whose rows span the Day type pass the share-days bound: the
/// actions table refuses by name, as the runs table does, and names the share
/// the budget ran out on in file order. A combination of two extremes: the
/// widest span of days and the fewest rows that make it.
#[test]
fn actions_whose_day_tables_would_pass_the_bound_are_refused_by_name() {
    let (first, last) = (day(1970, 1, 1), day(9999, 12, 31));
    let rows = [
        nse("ACO", first, Kind::Dividend, "1"),
        nse("ACO", last, Kind::Dividend, "1"),
        nse("BCO", first, Kind::Dividend, "1"),
        nse("BCO", last, Kind::Dividend, "1"),
    ];
    let said = refused(&actions_draft(Standing::Sourced, &rows));
    assert!(
        said.contains("\"BCO\"") && said.contains("share-days"),
        "{said}"
    );
}

#[test]
fn a_run_holds_both_of_its_ends_and_nothing_outside() {
    let held = run("ACO", day(2024, 3, 4), day(2024, 3, 8), "1");
    assert!(held.holds(day(2024, 3, 4)));
    assert!(held.holds(day(2024, 3, 8)));
    assert!(!held.holds(day(2024, 3, 3)));
    assert!(!held.holds(day(2024, 3, 9)));
}

#[test]
fn outside_says_each_neighbour_or_a_dash() {
    let on = day(2024, 3, 4);
    let both = Outside::Between {
        before: Some(on),
        after: Some(day(2024, 3, 8)),
    };
    assert_eq!(
        both.to_string(),
        "outside_every_run: the run before ends 2024-03-04, the run after starts 2024-03-08"
    );
    let neither = Outside::Between {
        before: None,
        after: None,
    };
    assert_eq!(
        neither.to_string(),
        "outside_every_run: the run before ends -, the run after starts -"
    );
}

#[test]
fn a_backwards_run_and_a_ratio_above_its_interval_are_refused() {
    let held = [run("ACO", day(2024, 3, 4), day(2024, 3, 8), "1")];
    let mut backwards = runs_draft(&held);
    backwards.rows[0][1] = "2024-03-09".to_owned();
    assert!(
        refused(&backwards).contains("run 2024-03-09..2024-03-08 runs backwards"),
        "{}",
        refused(&backwards)
    );
    let mut above = runs_draft(&held);
    above.rows[0][3] = "5/1".to_owned();
    assert!(refused(&above).contains("outside its own interval"));
}

/// The cap of runs for one share: 4,096 load, one more is refused by name.
#[test]
fn more_runs_than_a_share_may_hold_are_refused_by_name_and_the_cap_itself_loads() {
    let cap = u32::try_from(MAX_RUNS_PER_SYMBOL).unwrap();
    let store = scratch("cor-cov-runs-cap");
    write(&store, &runs_draft(&dailies("MANYCO", cap))).unwrap();
    let over = refused(&runs_draft(&dailies("MANYCO", cap + 1)));
    assert!(
        over.contains(&format!("\"MANYCO\" has more than {cap} runs")),
        "{over}"
    );
}

/// A ratio that doubles and halves every day closes a run a day: the cap of
/// runs is reached by a fold too, at exactly the 4,097th day, and the fold
/// says so rather than dropping a run.
#[test]
fn a_fold_past_the_cap_of_runs_refuses_at_exactly_the_day_past_it() {
    let base = day(2000, 1, 1).days_from_epoch();
    let cap = u32::try_from(MAX_RUNS_PER_SYMBOL).unwrap();
    let days = |n: u32| {
        (0..n).map(move |i| {
            let on = Day::from_days(base + i).unwrap();
            around(on, if i % 2 == 0 { "1" } else { "2" })
        })
    };
    assert_eq!(
        fold_runs("ACO", days(cap)).unwrap().len(),
        MAX_RUNS_PER_SYMBOL
    );
    assert_eq!(fold_runs("ACO", days(cap + 1)), Err(FoldError::TooMany));
}

#[test]
fn a_fold_error_says_what_stopped_it() {
    let on = day(2024, 3, 4);
    assert_eq!(
        FoldError::Backwards(on).to_string(),
        "day 2024-03-04: its interval runs backwards"
    );
    assert_eq!(FoldError::TooMany.to_string(), "more than 4096 runs");
    assert_eq!(
        FoldError::TooLarge(on).to_string(),
        "day 2024-03-04: its interval's terms are too large to widen exactly"
    );
}

#[test]
fn every_anomaly_has_its_word_and_the_wobble_says_its_reach() {
    let wobble = Anomaly::WithinWobble { wobble: r("3/2") };
    assert_eq!(wobble.word(), "basis_within_wobble");
    assert_eq!(
        wobble.to_string(),
        "basis_within_wobble: the share's own reversed steps within 365 days reach a factor of 3/2"
    );
    assert_eq!(
        Anomaly::Reversed {
            by: day(2024, 3, 4)
        }
        .word(),
        "basis_reversed"
    );
    assert_eq!(
        Anomaly::Short {
            old_days: 1,
            new_days: 2
        }
        .word(),
        "basis_short"
    );
}

fn step(symbol: &str, after: Day, ex_date: Day, low: &str, high: &str) -> Step {
    Step {
        symbol: symbol.to_owned(),
        after,
        ex_date,
        from: r("1"),
        to: r("2"),
        low: r(low),
        high: r(high),
        factor: r("2"),
        old_days: 9,
        new_days: 9,
    }
}

#[test]
fn every_finding_of_the_check_has_its_word_and_its_line() {
    let (after, on) = (day(2023, 6, 30), day(2023, 7, 3));
    let differs = Finding::FactorDiffers {
        step: step("ACO", after, on, "1999/1000", "2001/1000"),
        product: r("3"),
    };
    assert_eq!(differs.word(), "factor_differs");
    assert_eq!(
        differs.to_string(),
        "factor_differs ACO (2023-06-30, 2023-07-03]: the rows multiply to 3/1, the step's factor \
         lies in [1999/1000, 2001/1000]"
    );
    let stray = Finding::RowWithoutStep(nse("ACO", on, Kind::Split, "2"));
    assert_eq!(stray.word(), "row_without_step");
    assert_eq!(
        stray.to_string(),
        "row_without_step ACO 2023-07-03 split 2/1 (nse): the basis did not move across it"
    );
}

/// A step's old run out of range has no sides to count, whichever end of the
/// index is asked; the last run has no new run after it.
#[test]
fn the_days_held_of_a_step_past_the_runs_are_none() {
    let runs = [run("ACO", day(2024, 3, 4), day(2024, 3, 8), "1")];
    assert_eq!(held(&runs, 0, true, Ratio::ONE), (0, 0));
    assert_eq!(held(&runs, 5, true, Ratio::ONE), (0, 0));
    assert_eq!(held(&runs, usize::MAX, false, Ratio::ONE), (0, 0));
    assert_eq!(held(&[], 0, true, Ratio::ONE), (0, 0));
}

/// A run that does not end in a step asks for no step, so it is never counted
/// as skipped; only the pair past the bound is.
#[test]
fn a_run_that_drifted_away_is_not_a_skipped_step() {
    let drifted = [
        run("ACO", day(2024, 3, 4), day(2024, 3, 8), "1"),
        run("ACO", day(2024, 3, 11), day(2024, 3, 15), "2"),
    ];
    assert_eq!(steps_skipped(&drifted), 0);
    assert_eq!(steps_of(&drifted), []);
    assert_eq!(steps_skipped(&[]), 0);
}

/// A reversed step's far end raises the wobble of the steps within 365 days
/// of it and of none past them: the window forgets, on the day past, exactly.
#[test]
fn a_reversed_steps_wobble_expires_exactly_past_its_window() {
    let first = day(2020, 1, 1);
    let at = |days: u32| Day::from_days(first.days_from_epoch() + days).unwrap();
    let steps = [
        step("ACO", first, first, "3", "4"),
        step("ACO", first, at(WOBBLE_DAYS), "1", "2"),
        step("ACO", first, at(WOBBLE_DAYS + 1), "1", "2"),
        step("ACO", first, at(WOBBLE_DAYS + 2), "1", "2"),
    ];
    let mut wobbles = [Ratio::ONE; 4];
    wobble_pass(
        steps
            .iter()
            .zip(wobbles.iter_mut())
            .enumerate()
            .map(|(at, (step, wobble))| (step, at == 0, wobble)),
    );
    assert_eq!(wobbles[0], Ratio::ONE, "no reversed step before the first");
    assert_eq!(wobbles[1], steps[0].far(), "the last day of the window");
    assert_eq!(wobbles[2], Ratio::ONE, "one day past it");
    assert_eq!(wobbles[3], Ratio::ONE);
}

/// A short OLD side is as much a short side as a short new one.
#[test]
fn a_step_whose_old_side_is_short_is_named_short() {
    let mut days = weekdays(day(2022, 3, 1), day(2022, 3, 3), "1/2");
    days.extend(weekdays(day(2022, 3, 4), day(2022, 4, 29), "1"));
    let store = scratch("cor-cov-old-short");
    let runs = fold_runs("SHORTCO", days).unwrap();
    write(&store, &runs_draft(&runs)).unwrap();
    let loaded = load_one(&store, Table::BasisRuns, BasisRuns::build);
    let verdicts: Vec<Verdict> = judge(loaded.table().unwrap().of("SHORTCO"))
        .iter()
        .map(|j| j.verdict)
        .collect();
    assert_eq!(
        verdicts,
        [Verdict::Anomaly(Anomaly::Short {
            old_days: 3,
            new_days: 41
        })]
    );
}

/// The check over a share with a row of the wrong factor and a row that moves
/// no price: the first is named `factor_differs` for a product ABOVE the
/// step's interval, the second (a ratio of one) is no finding however it sits
/// inside a run.
#[test]
fn a_product_above_the_step_differs_and_a_row_of_ratio_one_is_never_a_finding() {
    let store = scratch("cor-cov-check");
    let mut days = weekdays(day(2023, 6, 1), day(2023, 6, 30), "1/2");
    days.extend(weekdays(day(2023, 7, 3), day(2023, 7, 31), "1"));
    let runs = fold_runs("SPLITCO", days).unwrap();
    write(&store, &runs_draft(&runs)).unwrap();
    let basis = load_one(&store, Table::BasisRuns, BasisRuns::build);
    let rows = [
        nse("SPLITCO", day(2023, 6, 15), Kind::Dividend, "1"),
        nse("SPLITCO", day(2023, 7, 3), Kind::Split, "8"),
    ];
    write(&store, &actions_draft(Standing::Sourced, &rows)).unwrap();
    let found = check(
        basis.table().unwrap(),
        load_actions(&store).table().unwrap(),
    );
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(found[0].word(), "factor_differs");
}

/// Every weekday of `from..=to` at `ratio`.
fn weekdays(from: Day, to: Day, ratio: &str) -> Vec<BasisDay> {
    let mut days = Vec::new();
    let mut at = from;
    while at <= to {
        if !matches!(at.days_from_epoch() % 7, 2 | 3) {
            days.push(around(at, ratio));
        }
        at = at.succ().unwrap();
    }
    days
}

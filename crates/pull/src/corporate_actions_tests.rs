//! Tests of the corporate-action table and the basis runs (R4-26: COR-06,
//! Z8, FM-260): both through R3-30's one loader, refused by name when their
//! bytes are not the bytes written, and a measured row that labels and
//! never excuses.
//!
//! EVERY SYMBOL AND RATIO BELOW IS INVENTED (`SPLITCO`, `BONUSCO`,
//! `DEMERGECO`, ...). No vendor row is copied; the intervals are written as
//! fractions around invented ratios, not read from any store.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail"
)]

use super::*;

use crate::gdfl_fixtures::scratch;
use crate::reference_data::{Slot, load_one, write};

fn day(y: u16, m: u8, d: u8) -> Day {
    Day::new(y, m, d).expect("a real day")
}

fn r(text: &str) -> Ratio {
    Ratio::parse(text).expect("a ratio")
}

/// A basis day whose interval is `ratio` give or take one part in two
/// thousand: the width DATA-1's allowance gives a minute of a share near
/// Rs 1,000 on a 5-paisa grid.
fn around(on: Day, ratio: &str) -> BasisDay {
    let ratio = r(ratio);
    BasisDay {
        day: on,
        low: ratio.times(r("1999/2000")).unwrap(),
        high: ratio.times(r("2001/2000")).unwrap(),
    }
}

/// Every weekday of `from..=to` at `ratio`.
fn weekdays(from: Day, to: Day, ratio: &str) -> Vec<BasisDay> {
    let mut days = Vec::new();
    let mut at = from;
    while at <= to {
        // 1970-01-01 was a Thursday: days % 7 of 2 and 3 are Saturday and Sunday.
        if !matches!(at.days_from_epoch() % 7, 2 | 3) {
            days.push(around(at, ratio));
        }
        at = at.succ().unwrap();
    }
    days
}

fn runs_draft(runs: &[Run]) -> Draft {
    Draft {
        table: Table::BasisRuns,
        source: "invented basis runs for a test".to_owned(),
        source_sha256: None,
        fetched: day(2026, 10, 22),
        standing: Standing::Measured,
        decision: DECISION.to_owned(),
        rows: runs.iter().map(Run::fields).collect(),
    }
}

fn actions_draft(standing: Standing, rows: &[Action]) -> Draft {
    Draft {
        table: Table::CorporateActions,
        source: "https://invented.example/corporate-actions.csv".to_owned(),
        source_sha256: Some("a".repeat(64)),
        fetched: day(2026, 10, 22),
        standing,
        decision: DECISION.to_owned(),
        rows: rows.iter().map(Action::fields).collect(),
    }
}

fn nse(symbol: &str, ex_date: Day, kind: Kind, ratio: &str) -> Action {
    Action {
        symbol: symbol.to_owned(),
        isin: Some("INE000INVENT".to_owned()),
        ex_date,
        kind,
        ratio: r(ratio),
        tag: Tag::Nse,
        after: None,
        evidence: "invented circular NSE/CML/00000".to_owned(),
    }
}

fn load_runs(store: &Path) -> Slot<BasisRuns> {
    load_one(store, Table::BasisRuns, BasisRuns::build)
}

fn load_actions(store: &Path) -> Slot<CorporateActions> {
    load_one(store, Table::CorporateActions, CorporateActions::build)
}

/// A 2:1 split on 2023-07-03: Zerodha back-adjusts the old prices to half,
/// so the ratio Zerodha / GDFL reads 1/2 before and 1/1 from the ex-date.
fn split_days() -> Vec<BasisDay> {
    let mut days = weekdays(day(2023, 6, 1), day(2023, 6, 30), "1/2");
    days.extend(weekdays(day(2023, 7, 3), day(2023, 7, 31), "1"));
    days
}

// ─────────────────────────────── the fraction ───────────────────────────────

#[test]
fn a_ratio_is_exact_reduced_and_refuses_garbage() {
    assert_eq!(r("4/8"), r("1/2"));
    assert_eq!(r("3").to_string(), "3/1");
    for garbage in [
        "",
        "0",
        "1/0",
        "0/5",
        "-1/2",
        "1/-2",
        "1.5",
        "a/b",
        "1/2/3",
        " 1/2",
        "10000000000000000000/1",
    ] {
        assert_eq!(Ratio::parse(garbage), None, "{garbage:?}");
    }
    assert!(r("1/3") < r("1/2"));
    assert_eq!(r("2/3").times(r("3/4")), Some(r("1/2")));
    assert_eq!(r("1").over(r("1/2")), Some(r("2")));
    assert_eq!(Ratio::of_signed(-6, -4), Some(r("3/2")));
    assert_eq!(Ratio::of_signed(-6, 4), None);
    // The level converts exactly: 50,000 paisa on a 1/2 basis is 100,000 raw.
    assert_eq!(r("1/2").to_gdfl(50_000), Some(r("100000")));
    assert_eq!(r("1/2").to_gdfl(-1), None);
}

#[test]
fn the_simplest_fraction_is_the_one_inside_the_interval_with_the_smallest_terms() {
    let simplest = |low: &str, high: &str| Ratio::simplest_between(r(low), r(high));
    assert_eq!(simplest("1999/4000", "2001/4000"), Some(r("1/2")));
    assert_eq!(simplest("1999/2000", "2001/2000"), Some(r("1")));
    assert_eq!(simplest("3/10", "3/10"), Some(r("3/10")));
    // 1/3 lies just outside [0.31, 0.33]; 5/16 (0.3125) is the simplest inside,
    // and 1/3 is the simplest inside [0.32, 0.34].
    assert_eq!(simplest("31/100", "33/100"), Some(r("5/16")));
    assert_eq!(simplest("32/100", "34/100"), Some(r("1/3")));
    assert_eq!(simplest("2/1", "5/2"), Some(r("2")));
    assert_eq!(
        simplest("1/2", "1/3"),
        None,
        "an interval that runs backwards"
    );
    // A demerger's factor need not be an integer: 0.4766 give or take 1e-4.
    let factor = simplest("4765/10000", "4767/10000").unwrap();
    assert!(r("4765/10000") <= factor && factor <= r("4767/10000"));
    assert!(factor.den() < 10_000, "{factor}");
}

// ─────────────────────────────── the fold ───────────────────────────────

#[test]
fn a_measured_basis_run_ends_on_the_day_the_ratio_steps() {
    let runs = fold_runs("SPLITCO", split_days()).unwrap();
    assert_eq!(runs.len(), 2, "{runs:#?}");
    assert_eq!(
        (runs[0].first, runs[0].last, runs[0].ratio, runs[0].end),
        (day(2023, 6, 1), day(2023, 6, 30), r("1/2"), RunEnd::Step)
    );
    assert_eq!(
        (runs[1].first, runs[1].last, runs[1].ratio, runs[1].end),
        (day(2023, 7, 3), day(2023, 7, 31), r("1"), RunEnd::Open)
    );
    assert_eq!(runs[0].days, 22);
}

#[test]
fn a_ratio_that_walks_ends_its_run_as_drift_not_as_a_step() {
    // Each day's interval meets the day before's, and the run's intersection
    // still empties: the ratio walked. Not an action.
    // Eight days, each 0.1% above the last (under the 0.2% step), so no day
    // steps; the band (each day widened by 0.2%) empties on the seventh.
    let walk = [
        "1000/1000",
        "1001/1000",
        "1002/1000",
        "1003/1000",
        "1004/1000",
        "1005/1000",
        "1006/1000",
        "1007/1000",
    ];
    let days = walk
        .iter()
        .enumerate()
        .map(|(i, ratio)| BasisDay {
            day: day(2024, 2, 5 + u8::try_from(i).unwrap()),
            low: r(ratio).times(r("9993/10000")).unwrap(),
            high: r(ratio).times(r("10007/10000")).unwrap(),
        })
        .collect::<Vec<_>>();
    let runs = fold_runs("DRIFTCO", days).unwrap();
    assert_eq!(runs.len(), 2, "{runs:#?}");
    assert_eq!((runs[0].end, runs[0].days), (RunEnd::Drift, 6));
    assert_eq!(runs[1].first, day(2024, 2, 11));
    assert!(steps_of(&runs).is_empty(), "a drift is never a step");
}

#[test]
fn a_share_with_no_common_minutes_has_no_run_and_days_must_ascend() {
    assert_eq!(fold_runs("EMPTYCO", []).unwrap(), []);
    let one = fold_runs("ONECO", [around(day(2020, 1, 1), "1")]).unwrap();
    assert_eq!((one.len(), one[0].days, one[0].end), (1, 1, RunEnd::Open));
    let twice = [around(day(2020, 1, 2), "1"), around(day(2020, 1, 2), "1")];
    assert!(matches!(
        fold_runs("DUPCO", twice),
        Err(FoldError::OutOfOrder { .. })
    ));
    let late = [around(day(2020, 1, 3), "1"), around(day(2020, 1, 2), "1")];
    let refused = fold_runs("LATECO", late).unwrap_err();
    assert!(
        refused
            .to_string()
            .contains("2020-01-02 follows 2020-01-03"),
        "{refused}"
    );
    let backwards = BasisDay {
        day: day(2020, 1, 2),
        low: r("2"),
        high: r("1"),
    };
    assert_eq!(
        fold_runs("BACKCO", [backwards]),
        Err(FoldError::Backwards(day(2020, 1, 2)))
    );
}

#[test]
fn a_gap_in_the_middle_keeps_the_run_and_widens_the_step_window() {
    // No basis days for a fortnight (a missing month mid-range is the same):
    // the run is not broken by the absence, and a step across the gap names
    // the whole window it could lie in.
    let mut days = weekdays(day(2022, 3, 1), day(2022, 3, 10), "1/5");
    days.extend(weekdays(day(2022, 3, 28), day(2022, 4, 8), "1"));
    let runs = fold_runs("GAPCO", days).unwrap();
    let steps = steps_of(&runs);
    assert_eq!(steps.len(), 1);
    assert_eq!(
        (steps[0].after, steps[0].ex_date, steps[0].factor),
        (day(2022, 3, 10), day(2022, 3, 28), r("5"))
    );
}

// ─────────────────────────────── the ledger (FM-260) ───────────────────────────────

#[test]
fn a_two_for_one_split_is_ledgered_from_the_vendor_ratio_step() {
    let store = scratch("cor-fm260");
    let runs = fold_runs("SPLITCO", split_days()).unwrap();
    let landed = write(&store, &runs_draft(&runs)).unwrap();
    let Slot::Store { table: loaded, .. } = load_runs(&store) else {
        panic!("the runs load from the store");
    };
    let rows = measured_rows(&loaded, landed.generation);
    assert_eq!(rows.len(), 1, "ONE row: {rows:#?}");
    let row = &rows[0];
    assert_eq!(row.ratio, r("2"), "factor 2");
    assert_eq!(
        (row.ex_date, row.after),
        (day(2023, 7, 3), Some(day(2023, 6, 30)))
    );
    assert_eq!((row.tag, row.kind), (Tag::Measured, Kind::Unclassed));
    // TAGGED MEASURED, SO IT LABELS AND NEVER EXCUSES.
    assert!(!row.excuses());
    assert_eq!(row.class(), None);
    // The ledger lands as a generation through the same writer and loader.
    let draft = measured_draft(
        &loaded,
        landed.generation,
        &landed.sha256,
        day(2026, 10, 22),
    );
    write(&store, &draft).unwrap();
    let Slot::Store {
        table: ledger,
        receipt,
        ..
    } = load_actions(&store)
    else {
        panic!("the ledger loads from the store");
    };
    assert_eq!(receipt.standing, Standing::Measured);
    assert_eq!(
        receipt.source_sha256.as_deref(),
        Some(landed.sha256.as_str())
    );
    let labelled = ledger.labels("SPLITCO", Some(day(2023, 6, 30)), day(2023, 7, 3));
    assert_eq!(labelled.len(), 1);
    assert_eq!(
        ledger.excuse("SPLITCO", Some(day(2023, 6, 30)), day(2023, 7, 3)),
        None
    );
}

#[test]
fn every_measured_ratio_run_matches_a_table_factor_on_its_ex_date() {
    // Three shares: a 2:1 split, a 1:1 bonus then a 1:5 split (two steps), and
    // a rights-style factor that is no integer at all.
    // Written in the table's canonical order: shares ascending.
    let mut bonus = weekdays(day(2019, 1, 1), day(2019, 1, 31), "1/10");
    bonus.extend(weekdays(day(2019, 2, 1), day(2019, 9, 19), "1/5"));
    bonus.extend(weekdays(day(2019, 9, 20), day(2019, 12, 31), "1"));
    let mut runs = fold_runs("BONUSCO", bonus).unwrap();
    let mut rights = weekdays(day(2021, 5, 3), day(2021, 5, 28), "10000/10481");
    rights.extend(weekdays(day(2021, 5, 31), day(2021, 6, 30), "1"));
    runs.extend(fold_runs("RIGHTSCO", rights).unwrap());
    runs.extend(fold_runs("SPLITCO", split_days()).unwrap());
    let store = scratch("cor-every-run");
    let landed = write(&store, &runs_draft(&runs)).unwrap();
    let loaded = load_runs(&store).table().unwrap().clone();
    // The measured ledger matches every step it was drawn from.
    write(
        &store,
        &measured_draft(
            &loaded,
            landed.generation,
            &landed.sha256,
            day(2026, 10, 22),
        ),
    )
    .unwrap();
    let ledger = load_actions(&store).table().unwrap().clone();
    assert_eq!(ledger.rows(), 4);
    assert_eq!(check(&loaded, &ledger), [], "every step has its row");
    let factors: Vec<(String, Day, Ratio)> = ledger
        .symbols()
        .iter()
        .flat_map(|s| {
            ledger
                .of(s)
                .iter()
                .map(|a| (a.symbol.clone(), a.ex_date, a.ratio))
        })
        .collect();
    assert_eq!(
        factors
            .iter()
            .filter(|(symbol, ..)| symbol != "RIGHTSCO")
            .cloned()
            .collect::<Vec<_>>(),
        [
            ("BONUSCO".to_owned(), day(2019, 2, 1), r("2")),
            ("BONUSCO".to_owned(), day(2019, 9, 20), r("5")),
            ("SPLITCO".to_owned(), day(2023, 7, 3), r("2")),
        ]
    );
    // The rights factor is no integer: the measured row carries the
    // simplest fraction its interval holds (1.0481 give or take 0.5%), and
    // the exact NSE factor below lies in the same interval.
    let rights = ledger.of("RIGHTSCO");
    assert_eq!((rights.len(), rights[0].ex_date), (1, day(2021, 5, 31)));
    assert!(
        r("104/100") < rights[0].ratio && rights[0].ratio < r("106/100"),
        "{}",
        rights[0].ratio
    );
    assert!(rights[0].ratio.den() < 10_000, "{}", rights[0].ratio);
    // An NSE table with the same actions matches too: each factor lies in its
    // step's interval on its own ex-date.
    let sourced = actions_draft(
        Standing::Sourced,
        &[
            nse("BONUSCO", day(2019, 2, 1), Kind::Bonus, "2"),
            nse("BONUSCO", day(2019, 9, 20), Kind::Split, "5"),
            nse("RIGHTSCO", day(2021, 5, 31), Kind::Rights, "10481/10000"),
            nse("SPLITCO", day(2023, 7, 3), Kind::Split, "2"),
        ],
    );
    let other = scratch("cor-every-run-nse");
    write(&other, &sourced).unwrap();
    let table = load_actions(&other).table().unwrap().clone();
    assert_eq!(check(&loaded, &table), []);
}

#[test]
fn a_share_with_an_action_and_no_row_is_flagged() {
    let runs = fold_runs("SPLITCO", split_days()).unwrap();
    let store = scratch("cor-no-row");
    write(&store, &runs_draft(&runs)).unwrap();
    let loaded = load_runs(&store).table().unwrap().clone();
    // An NSE table that holds another share's action only.
    write(
        &store,
        &actions_draft(
            Standing::Sourced,
            &[nse("OTHERCO", day(2023, 7, 3), Kind::Split, "2")],
        ),
    )
    .unwrap();
    let table = load_actions(&store).table().unwrap().clone();
    let found = check(&loaded, &table);
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(found[0].word(), "no_row");
    let said = found[0].to_string();
    assert!(
        said.contains("SPLITCO")
            && said.contains("(2023-06-30, 2023-07-03]")
            && said.contains("factor 2/1"),
        "{said}"
    );
    // A row of the wrong factor in the window is flagged too, by its product.
    let wrong = scratch("cor-wrong-factor");
    write(
        &wrong,
        &actions_draft(
            Standing::Sourced,
            &[nse("SPLITCO", day(2023, 7, 3), Kind::Split, "5")],
        ),
    )
    .unwrap();
    let found = check(&loaded, load_actions(&wrong).table().unwrap());
    assert_eq!(found.len(), 1);
    assert!(matches!(&found[0], Finding::FactorDiffers { product, .. } if *product == r("5")));
    // A price-moving row inside one run (the basis never moved) is flagged.
    let inside = scratch("cor-inside");
    write(
        &inside,
        &actions_draft(
            Standing::Sourced,
            &[
                nse("SPLITCO", day(2023, 7, 3), Kind::Split, "2"),
                nse("SPLITCO", day(2023, 7, 17), Kind::Bonus, "2"),
            ],
        ),
    )
    .unwrap();
    let found = check(&loaded, load_actions(&inside).table().unwrap());
    assert_eq!(
        found.iter().map(Finding::word).collect::<Vec<_>>(),
        ["row_without_step"]
    );
}

#[test]
fn an_ex_date_on_a_holiday_applies_at_the_next_session_and_two_actions_on_one_day_multiply() {
    // The ratio steps from 1/6 to 1 across a Saturday ex-date: a split 1:2 and
    // a bonus 1:2 on one day, applying at Monday's session. (2:1 x 3:1 = 6.)
    let mut days = weekdays(day(2020, 11, 2), day(2020, 11, 13), "1/6");
    days.extend(weekdays(day(2020, 11, 16), day(2020, 11, 27), "1"));
    let runs = fold_runs("HOLIDAYCO", days).unwrap();
    let store = scratch("cor-holiday");
    write(&store, &runs_draft(&runs)).unwrap();
    let loaded = load_runs(&store).table().unwrap().clone();
    let saturday = day(2020, 11, 14);
    assert_eq!(saturday.days_from_epoch() % 7, 2, "a Saturday");
    write(
        &store,
        &actions_draft(
            Standing::Sourced,
            &[
                nse("HOLIDAYCO", saturday, Kind::Split, "2"),
                nse("HOLIDAYCO", saturday, Kind::Bonus, "3"),
            ],
        ),
    )
    .unwrap();
    let table = load_actions(&store).table().unwrap().clone();
    assert_eq!(
        check(&loaded, &table),
        [],
        "the two rows multiply to the step"
    );
    // Monday's session, asked as (Friday, Monday], carries both rows.
    let monday = day(2020, 11, 16);
    let friday = day(2020, 11, 13);
    assert_eq!(table.labels("HOLIDAYCO", Some(friday), monday).len(), 2);
    assert_eq!(
        table.labels("HOLIDAYCO", Some(day(2020, 11, 12)), friday),
        []
    );
    // Asked for the day alone, only the ex-date's own day holds them.
    assert_eq!(table.labels("HOLIDAYCO", None, saturday).len(), 2);
    assert_eq!(table.labels("HOLIDAYCO", None, monday), []);
}

// ─────────────────────────────── the law ───────────────────────────────

#[test]
fn a_measured_row_never_suppresses_a_refusal_or_decides_a_class() {
    let on = day(2023, 7, 3);
    let measured = Action {
        symbol: "SPLITCO".to_owned(),
        isin: None,
        ex_date: on,
        kind: Kind::Unclassed,
        ratio: r("2"),
        tag: Tag::Measured,
        after: Some(day(2023, 6, 30)),
        evidence: "basis_runs generation 1".to_owned(),
    };
    let store = scratch("cor-law");
    write(
        &store,
        &actions_draft(Standing::Measured, std::slice::from_ref(&measured)),
    )
    .unwrap();
    let table = load_actions(&store).table().unwrap().clone();
    // IT LABELS ...
    assert_eq!(
        table.labels("SPLITCO", Some(day(2023, 6, 30)), on),
        std::slice::from_ref(&measured)
    );
    // ... AND NEVER EXCUSES, NEVER CLASSES.
    assert_eq!(table.excuse("SPLITCO", Some(day(2023, 6, 30)), on), None);
    assert_eq!(table.class_of("SPLITCO", Some(day(2023, 6, 30)), on), None);
    assert!(!measured.excuses() && measured.class().is_none());
    // An NSE row in a sourced file does both.
    let sourced = scratch("cor-law-nse");
    let row = nse("SPLITCO", on, Kind::Split, "2");
    write(
        &sourced,
        &actions_draft(Standing::Sourced, &[row.clone(), measured.clone()]),
    )
    .unwrap();
    let table = load_actions(&sourced).table().unwrap().clone();
    assert_eq!(table.excuse("SPLITCO", None, on), Some(&row));
    assert_eq!(table.class_of("SPLITCO", None, on), Some(Kind::Split));
    // A measured file cannot carry an nse row: refused at the write, by name.
    let refused = write(
        &scratch("cor-law-refuse"),
        &actions_draft(Standing::Measured, &[row]),
    )
    .unwrap_err()
    .to_string();
    assert!(
        refused.contains("an nse row in a measured file"),
        "{refused}"
    );
    // A measured row cannot name a kind: it would decide a class.
    let mut classed = measured.clone();
    classed.kind = Kind::Split;
    let refused = write(
        &scratch("cor-law-class"),
        &actions_draft(Standing::Measured, &[classed]),
    )
    .unwrap_err()
    .to_string();
    assert!(refused.contains("never decides a class"), "{refused}");
    // An nse row must name its kind.
    let mut unclassed = nse("SPLITCO", on, Kind::Split, "2");
    unclassed.kind = Kind::Unclassed;
    assert!(
        write(
            &scratch("cor-law-unclassed"),
            &actions_draft(Standing::Sourced, &[unclassed])
        )
        .is_err()
    );
}

// ─────────────────────────────── refusals by name ───────────────────────────────

#[test]
fn a_corporate_action_table_with_a_wrong_checksum_is_refused() {
    let store = scratch("cor-checksum");
    let landed = write(
        &store,
        &actions_draft(
            Standing::Sourced,
            &[nse("SPLITCO", day(2023, 7, 3), Kind::Split, "2")],
        ),
    )
    .unwrap();
    // One byte of one row changed after the write: 2/1 becomes 3/1.
    let bytes = std::fs::read(&landed.path).unwrap();
    let text = String::from_utf8(bytes)
        .unwrap()
        .replace("|split|2|1|", "|split|3|1|");
    std::fs::write(&landed.path, text).unwrap();
    let Slot::Refused(why) = load_actions(&store) else {
        panic!("a swapped table is never served");
    };
    assert_eq!(
        (why.table, why.kind, why.generation),
        (Table::CorporateActions, RefusalKind::Checksum, Some(1))
    );
    let said = why.to_string();
    assert!(
        said.contains("corporate_actions") && said.contains("checksum"),
        "{said}"
    );
    // A generation of ANOTHER table filed under this table's name is refused
    // as the wrong table, not read as actions.
    let runs = fold_runs("SPLITCO", split_days()).unwrap();
    let other = scratch("cor-swapped");
    let written = write(&other, &runs_draft(&runs)).unwrap();
    std::fs::create_dir_all(Table::CorporateActions.dir(&other)).unwrap();
    std::fs::copy(&written.path, Table::CorporateActions.path(&other, 1)).unwrap();
    assert_eq!(
        load_actions(&other).table().unwrap_err().kind,
        RefusalKind::WrongTable
    );
    // A torn generation (cut mid-write) is refused, never read short.
    let torn = scratch("cor-torn");
    let written = write(&torn, &runs_draft(&runs)).unwrap();
    let whole = std::fs::read(&written.path).unwrap();
    std::fs::write(&written.path, &whole[..whole.len() / 2]).unwrap();
    assert_eq!(
        load_runs(&torn).table().unwrap_err().kind,
        RefusalKind::Checksum
    );
    // A zero-length generation likewise.
    std::fs::write(&written.path, b"").unwrap();
    assert_eq!(
        load_runs(&torn).table().unwrap_err().kind,
        RefusalKind::Checksum
    );
}

#[test]
fn an_absent_table_is_refused_absent_by_name_and_never_served_empty() {
    let store = scratch("cor-absent");
    for (slot, word) in [
        (
            load_actions(&store).table().map(drop).unwrap_err().clone(),
            "corporate-action",
        ),
        (
            load_runs(&store).table().map(drop).unwrap_err().clone(),
            "basis-run",
        ),
    ] {
        assert_eq!(slot.kind, RefusalKind::Absent);
        assert!(slot.detail.contains(word), "{}", slot.detail);
    }
    // seed() installs no generation of either: they are not compiled.
    let seeded = crate::reference_data::seed(&store).unwrap();
    assert!(
        seeded
            .iter()
            .all(|w| !matches!(w.table, Table::CorporateActions | Table::BasisRuns))
    );
}

#[test]
fn malformed_duplicate_overlapping_and_oversized_tables_are_refused_by_name() {
    let on = day(2023, 7, 3);
    let refused = |draft: &Draft| write(&scratch("cor-bad"), draft).unwrap_err().to_string();
    // Two rows of one (symbol, ex-date, kind).
    let twice = actions_draft(
        Standing::Sourced,
        &[
            nse("ACO", on, Kind::Split, "2"),
            nse("ACO", on, Kind::Split, "2"),
        ],
    );
    assert!(
        refused(&twice).contains("two split rows"),
        "{}",
        refused(&twice)
    );
    // A bad ISIN, a zero ratio, an `after` that is not before the ex-date.
    let mut bad = nse("ACO", on, Kind::Split, "2");
    bad.isin = Some("NOTANISIN".to_owned());
    assert!(refused(&actions_draft(Standing::Sourced, &[bad])).contains("ISIN"));
    let mut zero = actions_draft(Standing::Sourced, &[nse("ACO", on, Kind::Split, "2")]);
    zero.rows[0][4] = "0".to_owned();
    assert!(refused(&zero).contains("positive ratio"));
    let mut late = actions_draft(Standing::Sourced, &[nse("ACO", on, Kind::Split, "2")]);
    late.rows[0][7] = "2023-07-03".to_owned();
    assert!(refused(&late).contains("not before the ex-date"));
    let mut kind = actions_draft(Standing::Sourced, &[nse("ACO", on, Kind::Split, "2")]);
    kind.rows[0][3] = "spinoff".to_owned();
    assert!(refused(&kind).contains("not a kind"));
    // More than MAX_ACTIONS_PER_SYMBOL rows for one share.
    let many: Vec<Action> = (0..=MAX_ACTIONS_PER_SYMBOL)
        .map(|i| {
            let at = Day::from_days(day(2000, 1, 1).days_from_epoch() + u32::try_from(i).unwrap())
                .unwrap();
            nse("MANYCO", at, Kind::Dividend, "1")
        })
        .collect();
    assert!(
        refused(&actions_draft(Standing::Sourced, &many))
            .contains(&format!("more than {MAX_ACTIONS_PER_SYMBOL} rows"))
    );
    // Basis runs: overlapping, a ratio outside its interval, a sourced file.
    let runs = fold_runs("SPLITCO", split_days()).unwrap();
    let mut overlap = runs_draft(&runs);
    overlap.rows[1][1] = "2023-06-30".to_owned();
    assert!(
        refused(&overlap).contains("two runs that overlap"),
        "{}",
        refused(&overlap)
    );
    let mut outside = runs_draft(&runs);
    outside.rows[0][3] = "1/3".to_owned();
    assert!(refused(&outside).contains("outside its own interval"));
    let mut sourced = runs_draft(&runs);
    sourced.standing = Standing::Sourced;
    assert!(refused(&sourced).contains("measured, not sourced"));
    let mut zero_days = runs_draft(&runs);
    zero_days.rows[0][6] = "0".to_owned();
    assert!(refused(&zero_days).contains("day count"));
}

#[test]
fn an_empty_table_loads_and_answers_nothing_for_every_share() {
    let store = scratch("cor-empty");
    write(&store, &actions_draft(Standing::Measured, &[])).unwrap();
    write(&store, &runs_draft(&[])).unwrap();
    let actions = load_actions(&store).table().unwrap().clone();
    let runs = load_runs(&store).table().unwrap().clone();
    assert_eq!((actions.rows(), runs.rows()), (0, 0));
    assert_eq!(actions.labels("ANYCO", None, day(2020, 1, 1)), []);
    assert_eq!(runs.run_on("ANYCO", day(2020, 1, 1)), Err(Outside::NoRuns));
    assert_eq!(check(&runs, &actions), []);
}

// ─────────────────────────────── the compare ───────────────────────────────

#[test]
fn a_compare_inside_a_run_applies_its_factor() {
    let mut days = weekdays(day(2023, 6, 1), day(2023, 6, 30), "1/2");
    days.extend(weekdays(day(2023, 7, 10), day(2023, 7, 31), "1"));
    let runs = fold_runs("SPLITCO", days).unwrap();
    let store = scratch("cor-compare");
    write(&store, &runs_draft(&runs)).unwrap();
    let table = load_runs(&store).table().unwrap().clone();
    // Inside the first run: the factor applies, exactly.
    let factor = table.factor_on("SPLITCO", day(2023, 6, 15)).unwrap();
    assert_eq!(factor, r("1/2"));
    assert_eq!(factor.to_gdfl(50_000), Some(r("100000")));
    // Its edges are inside.
    assert_eq!(table.factor_on("SPLITCO", day(2023, 6, 1)), Ok(r("1/2")));
    assert_eq!(table.factor_on("SPLITCO", day(2023, 7, 31)), Ok(r("1")));
    // Between the runs (the step's window), before the first and after the
    // last: NO factor, never one borrowed from a neighbour.
    assert_eq!(
        table.factor_on("SPLITCO", day(2023, 7, 5)),
        Err(Outside::Between {
            before: Some(day(2023, 6, 30)),
            after: Some(day(2023, 7, 10))
        })
    );
    assert_eq!(
        table.factor_on("SPLITCO", day(2023, 5, 31)),
        Err(Outside::Between {
            before: None,
            after: Some(day(2023, 6, 1))
        })
    );
    assert_eq!(
        table.factor_on("SPLITCO", day(2023, 8, 1)),
        Err(Outside::Between {
            before: Some(day(2023, 7, 31)),
            after: None
        })
    );
    assert_eq!(
        table.factor_on("NOSUCHCO", day(2023, 6, 15)),
        Err(Outside::NoRuns)
    );
    assert!(Outside::NoRuns.to_string().starts_with("no_basis_run"));
}

// ─────────────────────────────── cost ───────────────────────────────

fn time(n: usize, mut call: impl FnMut()) -> (u128, u128, u128) {
    let mut took: Vec<u128> = (0..n)
        .map(|_| {
            let start = std::time::Instant::now();
            call();
            start.elapsed().as_nanos()
        })
        .collect();
    took.sort_unstable();
    (took[n / 2], took[(n * 99) / 100], took[n - 1])
}

#[test]
fn a_lookup_costs_the_same_at_1x_10x_and_100x_the_shares_measured() {
    // 1x, 10x and 100x shares, each with 16 runs and 8 actions: the lookup is
    // one hash and one bounded binary search, so its median holds flat.
    let mut medians = Vec::new();
    for shares in [10_usize, 100, 1_000] {
        let mut runs_rows = Vec::new();
        let mut action_rows = Vec::new();
        for s in 0..shares {
            let symbol = format!("SHARE{s:05}");
            let mut days = Vec::new();
            for k in 0..16_u32 {
                let ratio = format!("1/{}", 1 + (k % 2));
                let first = Day::from_days(day(2018, 1, 1).days_from_epoch() + k * 100).unwrap();
                let last = Day::from_days(first.days_from_epoch() + 60).unwrap();
                days.extend(weekdays(first, last, &ratio));
            }
            let runs = fold_runs(&symbol, days).unwrap();
            for step in steps_of(&runs).iter().take(8) {
                action_rows.push(Action {
                    symbol: symbol.clone(),
                    isin: None,
                    ex_date: step.ex_date,
                    kind: Kind::Unclassed,
                    ratio: step.factor,
                    tag: Tag::Measured,
                    after: Some(step.after),
                    evidence: "invented".to_owned(),
                });
            }
            runs_rows.extend(runs);
        }
        let store = scratch("cor-scale");
        write(&store, &runs_draft(&runs_rows)).unwrap();
        write(&store, &actions_draft(Standing::Measured, &action_rows)).unwrap();
        let runs = load_runs(&store).table().unwrap().clone();
        let actions = load_actions(&store).table().unwrap().clone();
        let probe = format!("SHARE{:05}", shares / 2);
        let on = day(2019, 6, 3);
        let (p50, p99, max) = time(2_000, || {
            std::hint::black_box(runs.run_on(std::hint::black_box(&probe), on).ok());
            std::hint::black_box(actions.labels(std::hint::black_box(&probe), None, on).len());
        });
        println!("lookup at {shares} shares: p50 {p50} ns, p99 {p99} ns, max {max} ns, n 2000");
        medians.push(p50);
    }
    assert!(
        medians[2] <= medians[0].max(200) * 4,
        "a lookup's median at 1x, 10x, 100x shares: {medians:?} ns"
    );
}

// ─────────────────────────────── canonical order and the day table ───────────────────────────────

fn run(symbol: &str, first: Day, last: Day, ratio: &str) -> Run {
    Run {
        symbol: symbol.to_owned(),
        first,
        last,
        ratio: r(ratio),
        low: r(ratio),
        high: r(ratio),
        days: 1,
        end: RunEnd::Drift,
    }
}

#[test]
fn a_table_out_of_canonical_order_is_refused_by_name_and_never_sorted() {
    let refused = |draft: &Draft| write(&scratch("cor-order"), draft).unwrap_err().to_string();
    let (a, b) = (day(2021, 3, 1), day(2022, 3, 1));
    // A share's actions descending.
    let backwards = actions_draft(
        Standing::Sourced,
        &[
            nse("ACO", b, Kind::Split, "2"),
            nse("ACO", a, Kind::Split, "2"),
        ],
    );
    assert!(
        refused(&backwards).contains("must ascend by ex-date"),
        "{}",
        refused(&backwards)
    );
    // One day, kinds out of order (bonus sorts after split).
    let kinds = actions_draft(
        Standing::Sourced,
        &[
            nse("ACO", a, Kind::Bonus, "2"),
            nse("ACO", a, Kind::Split, "2"),
        ],
    );
    assert!(
        refused(&kinds).contains("must ascend by ex-date, then kind"),
        "{}",
        refused(&kinds)
    );
    // Symbols descending, and a share split around another.
    let symbols = actions_draft(
        Standing::Sourced,
        &[
            nse("BCO", a, Kind::Split, "2"),
            nse("ACO", a, Kind::Split, "2"),
        ],
    );
    assert!(
        refused(&symbols).contains("rows must ascend by symbol"),
        "{}",
        refused(&symbols)
    );
    let split_up = actions_draft(
        Standing::Sourced,
        &[
            nse("ACO", a, Kind::Split, "2"),
            nse("BCO", a, Kind::Split, "2"),
            nse("ACO", b, Kind::Split, "2"),
        ],
    );
    assert!(
        refused(&split_up).contains("\"ACO\" after \"BCO\""),
        "{}",
        refused(&split_up)
    );
    // Runs: a run before the one above it, and symbols descending.
    let late = run("ACO", day(2021, 6, 1), day(2021, 6, 30), "1");
    let early = run("ACO", day(2021, 1, 4), day(2021, 1, 29), "1/2");
    let runs_back = runs_draft(&[late.clone(), early.clone()]);
    assert!(
        refused(&runs_back).contains("must ascend by first day"),
        "{}",
        refused(&runs_back)
    );
    let mut other = early;
    other.symbol = "BCO".to_owned();
    let runs_symbols = runs_draft(&[other, late]);
    assert!(refused(&runs_symbols).contains("rows must ascend by symbol"));
}

#[test]
fn a_run_lookup_answers_every_edge_of_the_day_table() {
    // Three runs with a gap, and a one-day run; every edge day asked.
    let runs = [
        run("EDGECO", day(2024, 1, 1), day(2024, 1, 31), "1/2"),
        run("EDGECO", day(2024, 2, 10), day(2024, 2, 10), "1"),
        run("EDGECO", day(2024, 2, 11), day(2024, 3, 31), "2"),
        run("ZEDCO", day(2019, 5, 6), day(2019, 5, 6), "3"),
    ];
    let store = scratch("cor-edges");
    write(&store, &runs_draft(&runs)).unwrap();
    let table = load_runs(&store).table().unwrap().clone();
    assert_eq!(table.symbols(), ["EDGECO".to_owned(), "ZEDCO".to_owned()]);
    let ratio = |on: Day| table.run_on("EDGECO", on).map(|run| run.ratio.to_string());
    // Inside: first day, last day, the one-day run, the run right after it.
    assert_eq!(ratio(day(2024, 1, 1)), Ok("1/2".to_owned()));
    assert_eq!(ratio(day(2024, 1, 31)), Ok("1/2".to_owned()));
    assert_eq!(ratio(day(2024, 2, 10)), Ok("1/1".to_owned()));
    assert_eq!(ratio(day(2024, 2, 11)), Ok("2/1".to_owned()));
    assert_eq!(ratio(day(2024, 3, 31)), Ok("2/1".to_owned()));
    // Before the first run, in the gap, after the last: named, never borrowed.
    assert_eq!(
        table.run_on("EDGECO", day(2023, 12, 31)),
        Err(Outside::Between {
            before: None,
            after: Some(day(2024, 1, 1))
        })
    );
    assert_eq!(
        table.run_on("EDGECO", day(2024, 2, 1)),
        Err(Outside::Between {
            before: Some(day(2024, 1, 31)),
            after: Some(day(2024, 2, 10))
        })
    );
    assert_eq!(
        table.run_on("EDGECO", day(2024, 4, 1)),
        Err(Outside::Between {
            before: Some(day(2024, 3, 31)),
            after: None
        })
    );
    // The far ends of the calendar the Day type holds.
    assert_eq!(
        table.run_on("EDGECO", day(1970, 1, 1)),
        Err(Outside::Between {
            before: None,
            after: Some(day(2024, 1, 1))
        })
    );
    assert_eq!(
        table.run_on("EDGECO", day(9999, 12, 31)),
        Err(Outside::Between {
            before: Some(day(2024, 3, 31)),
            after: None
        })
    );
    // A one-run share, and a share with none.
    assert_eq!(table.factor_on("ZEDCO", day(2019, 5, 6)), Ok(r("3")));
    assert_eq!(table.run_on("NOCO", day(2019, 5, 6)), Err(Outside::NoRuns));
    assert_eq!(table.factor_on("", day(2019, 5, 6)), Err(Outside::NoRuns));
}

#[test]
fn runs_whose_day_tables_would_pass_the_bound_are_refused_by_name() {
    // Two runs each spanning the whole calendar the Day type holds (1970 to
    // 9999, about 2.9 million days each) pass MAX_INDEXED_DAYS together: the
    // load refuses by name instead of allocating without bound.
    let whole = |symbol: &str| run(symbol, day(1970, 1, 1), day(9999, 12, 31), "1");
    let one = scratch("cor-span-one");
    write(&one, &runs_draft(&[whole("ACO")])).unwrap();
    assert_eq!(
        load_runs(&one)
            .table()
            .unwrap()
            .factor_on("ACO", day(5000, 6, 1)),
        Ok(r("1"))
    );
    let two = write(
        &scratch("cor-span-two"),
        &runs_draft(&[whole("ACO"), whole("BCO")]),
    )
    .unwrap_err()
    .to_string();
    assert!(two.contains("share-days"), "{two}");
    assert!(two.contains("\"BCO\""), "{two}");
}

#[test]
fn a_step_the_data_cannot_tell_from_none_makes_no_row_and_is_asked_none() {
    // Two runs whose bands do not separate: the step's factor interval holds
    // one. It ends a run and makes no ledger row; the checker asks none.
    let band = |symbol: &str, first: Day, last: Day, low: &str, high: &str, end| Run {
        symbol: symbol.to_owned(),
        first,
        last,
        ratio: Ratio::simplest_between(r(low), r(high)).unwrap(),
        low: r(low),
        high: r(high),
        days: 5,
        end,
    };
    let runs = [
        // NOISECO: [0.998, 1.002] then [0.999, 1.003]: factor in [0.997, 1.005].
        band(
            "NOISECO",
            day(2022, 1, 3),
            day(2022, 1, 7),
            "998/1000",
            "1002/1000",
            RunEnd::Step,
        ),
        band(
            "NOISECO",
            day(2022, 1, 10),
            day(2022, 1, 14),
            "999/1000",
            "1003/1000",
            RunEnd::Open,
        ),
        // REALCO: 1/2 then 1: factor in [1.99, 2.01], proven.
        band(
            "REALCO",
            day(2022, 1, 3),
            day(2022, 1, 7),
            "4995/10000",
            "5005/10000",
            RunEnd::Step,
        ),
        band(
            "REALCO",
            day(2022, 1, 10),
            day(2022, 1, 14),
            "9995/10000",
            "10005/10000",
            RunEnd::Open,
        ),
    ];
    let store = scratch("cor-noise");
    let landed = write(&store, &runs_draft(&runs)).unwrap();
    let table = load_runs(&store).table().unwrap().clone();
    let noise = steps_of(table.of("NOISECO"));
    assert_eq!(noise.len(), 1);
    assert!(!noise[0].proven(), "{:?}", noise[0]);
    let real = steps_of(table.of("REALCO"));
    assert!(real[0].proven(), "{:?}", real[0]);
    // The ledger holds REALCO's row alone.
    let rows = measured_rows(&table, landed.generation);
    assert_eq!(rows.len(), 1);
    assert_eq!((rows[0].symbol.as_str(), rows[0].ratio), ("REALCO", r("2")));
    // Against an empty table only the proven step is a finding.
    write(&store, &actions_draft(Standing::Measured, &[])).unwrap();
    let empty = load_actions(&store).table().unwrap().clone();
    let found = check(&table, &empty);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(matches!(&found[0], Finding::NoRow(step) if step.symbol == "REALCO"));
}

#[test]
fn an_action_lookup_is_one_index_at_1_64_and_4096_rows_measured() {
    // One share with 1, 64 and 4,096 rows (one a day from 2010-01-01; the
    // real measured ledger reached 121 for one share): every lookup answers
    // the right rows and its median holds flat.
    let mut medians = Vec::new();
    for count in [1_u32, 64, 4_096] {
        let base = day(2010, 1, 1).days_from_epoch();
        let rows: Vec<Action> = (0..count)
            .map(|i| {
                nse(
                    "DEEPCO",
                    Day::from_days(base + i).unwrap(),
                    Kind::Dividend,
                    "1",
                )
            })
            .collect();
        let store = scratch("cor-deep");
        write(&store, &actions_draft(Standing::Sourced, &rows)).unwrap();
        let table = load_actions(&store).table().unwrap().clone();
        let last = Day::from_days(base + count - 1).unwrap();
        // The first row, the last, the day after the last, the day before the first.
        assert_eq!(table.labels("DEEPCO", None, day(2010, 1, 1)).len(), 1);
        assert_eq!(table.labels("DEEPCO", None, last).len(), 1);
        assert_eq!(
            table.labels("DEEPCO", None, Day::from_days(base + count).unwrap()),
            []
        );
        assert_eq!(table.labels("DEEPCO", None, day(2009, 12, 31)), []);
        // A window over every row returns every row.
        assert_eq!(
            table.labels("DEEPCO", Some(day(2009, 12, 31)), last).len(),
            usize::try_from(count).unwrap()
        );
        assert_eq!(
            table.excuse("DEEPCO", None, last).map(|a| a.ex_date),
            Some(last)
        );
        let mid = Day::from_days(base + count / 2).unwrap();
        let (p50, p99, max) = time(2_000, || {
            std::hint::black_box(
                table
                    .labels(std::hint::black_box("DEEPCO"), None, mid)
                    .len(),
            );
        });
        println!("action lookup at {count} rows: p50 {p50} ns, p99 {p99} ns, max {max} ns, n 2000");
        medians.push(p50);
    }
    assert!(
        medians[2] <= medians[0].max(200) * 4,
        "an action lookup's median at 1, 64 and 4,096 rows: {medians:?} ns"
    );
}

// ─────────────────────────────── proven is not lasting (D-7195) ───────────────────────────────

/// The runs of `days` for `symbol`, loaded back through the store as the
/// ledger reads them.
fn loaded(tag: &str, shares: &[(&str, Vec<BasisDay>)]) -> (std::sync::Arc<BasisRuns>, u64) {
    let mut runs = Vec::new();
    for (symbol, days) in shares {
        runs.extend(fold_runs(symbol, days.clone()).unwrap());
    }
    let store = scratch(tag);
    let landed = write(&store, &runs_draft(&runs)).unwrap();
    (
        load_runs(&store).table().unwrap().clone(),
        landed.generation,
    )
}

/// An empty measured table, for the checker.
fn no_actions(tag: &str) -> std::sync::Arc<CorporateActions> {
    let store = scratch(tag);
    write(&store, &actions_draft(Standing::Measured, &[])).unwrap();
    load_actions(&store).table().unwrap().clone()
}

#[test]
fn a_ratio_that_alternates_around_one_over_sixty_days_ledgers_no_row() {
    // ALTCO: 0.80 / 0.81 / 0.80 a session at a time for sixty days; SEGCO:
    // the same three ratios twenty sessions each. Every change is a PROVEN
    // step (the bands separate by 1.25%), and none is a lasting change of
    // basis: the ratio goes back.
    let mut alt = Vec::new();
    let mut at = day(2022, 1, 3);
    while alt.len() < 60 {
        if !matches!(at.days_from_epoch() % 7, 2 | 3) {
            alt.push(around(
                at,
                if alt.len() % 2 == 0 { "4/5" } else { "81/100" },
            ));
        }
        at = at.succ().unwrap();
    }
    let mut seg = weekdays(day(2022, 1, 3), day(2022, 1, 28), "4/5");
    seg.extend(weekdays(day(2022, 1, 31), day(2022, 2, 25), "81/100"));
    seg.extend(weekdays(day(2022, 2, 28), day(2022, 3, 25), "4/5"));
    let (runs, generation) = loaded("cor-alternate", &[("ALTCO", alt), ("SEGCO", seg)]);
    let alt_steps = judge(runs.of("ALTCO"));
    assert_eq!(alt_steps.len(), 59, "a step a session");
    assert!(alt_steps.iter().all(|j| j.step.proven()));
    assert!(
        alt_steps
            .iter()
            .all(|j| matches!(j.verdict, Verdict::Anomaly(Anomaly::Reversed { .. }))),
        "{:#?}",
        alt_steps.iter().map(|j| j.verdict).collect::<Vec<_>>()
    );
    let seg_steps = judge(runs.of("SEGCO"));
    assert_eq!(seg_steps.len(), 2);
    assert_eq!(
        seg_steps.iter().map(|j| j.verdict).collect::<Vec<_>>(),
        [
            Verdict::Anomaly(Anomaly::Reversed {
                by: day(2022, 2, 28)
            }),
            Verdict::Anomaly(Anomaly::Reversed {
                by: day(2022, 1, 31)
            }),
        ]
    );
    // NO ROW: the ledger holds nothing of either share.
    assert_eq!(measured_rows(&runs, generation), []);
    // The checker names every one as a basis anomaly, never as a missing row.
    let found = check(&runs, &no_actions("cor-alternate-ca"));
    assert_eq!(found.len(), 61, "{found:#?}");
    assert!(found.iter().all(|f| f.word() == "basis_anomaly"));
    let said = found
        .iter()
        .find(|f| matches!(f, Finding::BasisAnomaly { step, .. } if step.symbol == "SEGCO"))
        .unwrap()
        .to_string();
    assert!(
        said.contains("basis_anomaly SEGCO (2022-01-28, 2022-01-31]")
            && said.contains("basis_reversed: a proven step the other way on 2022-02-28"),
        "{said}"
    );
}

#[test]
fn a_one_session_two_percent_excursion_ledgers_no_row() {
    // EXCO: 0.80 for a month, 0.816 (two percent up) for ONE session, then
    // 0.80 again for a month. Both steps are proven; neither lasts.
    let mut days = weekdays(day(2023, 5, 1), day(2023, 5, 31), "4/5");
    days.push(around(day(2023, 6, 1), "102/125"));
    days.extend(weekdays(day(2023, 6, 2), day(2023, 6, 30), "4/5"));
    let (runs, generation) = loaded("cor-excursion", &[("EXCO", days)]);
    let judged = judge(runs.of("EXCO"));
    assert_eq!(judged.len(), 2, "{judged:#?}");
    assert!(judged.iter().all(|j| j.step.proven()));
    assert!(
        judged
            .iter()
            .all(|j| matches!(j.verdict, Verdict::Anomaly(Anomaly::Reversed { .. }))),
        "{judged:#?}"
    );
    assert_eq!(measured_rows(&runs, generation), []);
    // The same session at the very END of the data, never reversed, is
    // still no row: one session is not a basis that holds.
    let mut days = weekdays(day(2023, 5, 1), day(2023, 5, 31), "4/5");
    days.push(around(day(2023, 6, 1), "102/125"));
    let (runs, generation) = loaded("cor-excursion-end", &[("EXCO", days)]);
    assert_eq!(
        judge(runs.of("EXCO"))
            .iter()
            .map(|j| j.verdict)
            .collect::<Vec<_>>(),
        [Verdict::Anomaly(Anomaly::Short {
            old_days: 23,
            new_days: 1
        })]
    );
    assert_eq!(measured_rows(&runs, generation), []);
    let found = check(&runs, &no_actions("cor-excursion-end-ca"));
    assert_eq!(found.len(), 1);
    assert!(
        found[0]
            .to_string()
            .contains("basis_short: the runs hold 23 and 1 days"),
        "{}",
        found[0]
    );
}

#[test]
fn a_step_inside_the_shares_own_wobble_is_named_and_one_beyond_it_is_ledgered() {
    // Both shares wobble 1.2% up and back in January (two reversed steps),
    // hold 1/2 to the end of May, then step on 2021-06-01 and hold for three
    // months. WOBCO's step is 0.8%, inside its own January wobble; LASTCO's
    // is 5%, beyond it. The step back lies 134 days before June: not within
    // REVERSAL_DAYS, so only the share's own wobble can name WOBCO's.
    let history = |june: &str| {
        let mut days = weekdays(day(2021, 1, 4), day(2021, 1, 8), "1/2");
        days.extend(weekdays(day(2021, 1, 11), day(2021, 1, 15), "253/500"));
        days.extend(weekdays(day(2021, 1, 18), day(2021, 5, 31), "1/2"));
        days.extend(weekdays(day(2021, 6, 1), day(2021, 8, 31), june));
        days
    };
    let (runs, generation) = loaded(
        "cor-wobble",
        &[("LASTCO", history("21/40")), ("WOBCO", history("63/125"))],
    );
    let wob = judge(runs.of("WOBCO"));
    assert_eq!(wob.len(), 3, "{wob:#?}");
    assert!(wob.iter().all(|j| j.step.proven()), "{wob:#?}");
    assert!(matches!(
        wob[2].verdict,
        Verdict::Anomaly(Anomaly::WithinWobble { wobble }) if wobble > r("1012/1000")
    ));
    let last = judge(runs.of("LASTCO"));
    assert_eq!(last[2].verdict, Verdict::Lasting, "{last:#?}");
    // ONE ROW: LASTCO's June step. January's wobble and WOBCO's step make none.
    let rows = measured_rows(&runs, generation);
    assert_eq!(
        rows.iter()
            .map(|a| (a.symbol.as_str(), a.ex_date))
            .collect::<Vec<_>>(),
        [("LASTCO", day(2021, 6, 1))]
    );
    assert!(
        rows[0].evidence.contains("lasting (") && rows[0].day_known(),
        "{}",
        rows[0].evidence
    );
    let found = check(&runs, &no_actions("cor-wobble-ca"));
    assert!(found.iter().any(|f| matches!(
        f,
        Finding::BasisAnomaly { step, why: Anomaly::WithinWobble { .. } } if step.symbol == "WOBCO"
    )));
    assert!(
        found
            .iter()
            .any(|f| matches!(f, Finding::NoRow(step) if step.symbol == "LASTCO"))
    );
}

#[test]
fn a_two_for_one_step_across_a_missing_month_does_not_know_its_day() {
    // GAPSPLIT: 1/4 through August 2019, September missing from the input,
    // 1/2 from October. The step is lasting and its row is drawn, but its
    // window (2019-08-30, 2019-10-01] holds sessions the data never saw: the
    // row does not know its day.
    let mut days = weekdays(day(2019, 8, 1), day(2019, 8, 30), "1/4");
    days.extend(weekdays(day(2019, 10, 1), day(2019, 10, 31), "1/2"));
    let (runs, generation) = loaded("cor-gap-split", &[("GAPSPLIT", days)]);
    let rows = measured_rows(&runs, generation);
    assert_eq!(rows.len(), 1, "{rows:#?}");
    let row = &rows[0];
    assert_eq!(
        (row.after, row.ex_date, row.ratio),
        (Some(day(2019, 8, 30)), day(2019, 10, 1), r("2"))
    );
    assert!(!row.day_known(), "a missing month is a window, not a day");
    assert!(
        row.evidence.contains("UNKNOWN inside the window"),
        "{}",
        row.evidence
    );
    // A split across a weekend knows its day.
    let (split, generation) = loaded("cor-weekend-split", &[("SPLITCO", split_days())]);
    let rows = measured_rows(&split, generation);
    assert_eq!(rows.len(), 1);
    assert!(rows[0].day_known(), "{rows:#?}");
    // An NSE row always knows its day.
    assert!(nse("SPLITCO", day(2023, 7, 3), Kind::Split, "2").day_known());
}

#[test]
fn a_session_between_two_days_is_found_within_a_bounded_walk() {
    // Friday to Monday: the weekend is closed.
    assert!(!session_between(day(2023, 6, 30), day(2023, 7, 3)));
    // Consecutive days, and one day with itself: nothing lies between.
    assert!(!session_between(day(2023, 7, 3), day(2023, 7, 4)));
    assert!(!session_between(day(2023, 7, 3), day(2023, 7, 3)));
    // A weekday between: a session may lie there.
    assert!(session_between(day(2023, 7, 3), day(2023, 7, 5)));
    // 2020-11-14 was a Saturday Muhurat session: the calendar does not call
    // it closed, so a Friday-to-Monday window around it is not one session.
    assert!(session_between(day(2020, 11, 13), day(2020, 11, 16)));
    // Longer than the walk: a session may lie there, and no day is walked.
    assert!(session_between(day(2023, 1, 1), day(2023, 12, 31)));
    let fifteen = Day::from_days(day(2023, 7, 3).days_from_epoch() + MAX_WINDOW_WALK + 1).unwrap();
    assert!(session_between(day(2023, 7, 3), fifteen));
}

#[test]
fn a_step_with_a_short_side_is_named_short_and_ledgers_no_row() {
    // SHORTCO: 1/2 through March 2022, then 1/1 for three sessions and the
    // data ends. Proven, never reversed, and not shown to hold.
    let mut days = weekdays(day(2022, 3, 1), day(2022, 3, 31), "1/2");
    days.extend(weekdays(day(2022, 4, 1), day(2022, 4, 5), "1"));
    let (runs, generation) = loaded("cor-short", &[("SHORTCO", days)]);
    assert_eq!(
        judge(runs.of("SHORTCO"))
            .iter()
            .map(|j| j.verdict)
            .collect::<Vec<_>>(),
        [Verdict::Anomaly(Anomaly::Short {
            old_days: 23,
            new_days: 3
        })]
    );
    assert_eq!(measured_rows(&runs, generation), []);
    // Five sessions hold: the same split is lasting.
    let mut days = weekdays(day(2022, 3, 1), day(2022, 3, 31), "1/2");
    days.extend(weekdays(day(2022, 4, 1), day(2022, 4, 7), "1"));
    let (runs, generation) = loaded("cor-short-held", &[("SHORTCO", days)]);
    assert_eq!(measured_rows(&runs, generation).len(), 1);
}

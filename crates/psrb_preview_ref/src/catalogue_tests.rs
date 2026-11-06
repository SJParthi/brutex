#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a test that cannot panic cannot fail."
)]

use super::*;
use crate::run::{Output, run};
use crate::trade::{ExitKind, Outcome};

fn by_name(name: &str) -> Scenario {
    hand().into_iter().find(|s| s.name == name).unwrap()
}

fn output(name: &str) -> Output {
    run(&by_name(name).market).unwrap()
}

fn kinds(out: &Output) -> Vec<ExitKind> {
    out.records.values().flatten().filter_map(|r| r.trade.map(|t| t.sim.kind)).collect()
}

#[test]
fn the_catalogue_has_two_hundred_uniquely_named_scenarios() {
    let all = all();
    assert_eq!(all.len(), 200);
    let names: std::collections::BTreeSet<&str> = all.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names.len(), 200);
    for (i, s) in all.iter().enumerate() {
        assert!(s.name.starts_with(&format!("s{:03}-", i + 1)), "{} at {i}", s.name);
    }
}

#[test]
fn the_pinned_exact_pairs_are_the_solver_s() {
    let d = crate::generator::day(2018, 9, 4);
    for (target, pair) in EXACT {
        assert_eq!(solve_exact(d, target), Some(pair));
    }
    assert_eq!(solve_exact(d, i128::MAX / 4), None);
}

#[test]
fn every_planted_exit_kind_occurs_where_its_scenario_says() {
    let cases = [
        ("s001-long-gap-drift-time-exit", ExitKind::Time),
        ("s004-long-gap-through-stop", ExitKind::Stop),
        ("s006-long-flip-close-below-band", ExitKind::Flip),
        ("s008-stop-and-flip-in-one-second", ExitKind::Flip),
        ("s009-stop-and-time-exit-in-one-second", ExitKind::Stop),
        ("s010-square-off-no-second-in-window", ExitKind::SquareOff),
        ("s011-flip-filled-at-square-off", ExitKind::SquareOff),
        ("s012-halted-afternoon-long-unclosable", ExitKind::Unclosable),
        ("s014-entry-is-the-last-second", ExitKind::Unclosable),
        ("s033-fallback-unclosable", ExitKind::Unclosable),
    ];
    for (name, kind) in cases {
        assert!(kinds(&output(name)).contains(&kind), "{name}: no {kind:?}");
    }
}

#[test]
fn every_planted_refusal_occurs_where_its_scenario_says() {
    let not = |name: &str, want: Outcome| output(name).records.values().flatten().any(|r| r.candidate.outcome == want);
    assert!(not("s016-entry-not-filled", Outcome::NotFilled));
    assert!(not("s017-no-ratio-minute", Outcome::NoRatio));
    assert!(not("s018-zero-quantity-above-5-lakh", Outcome::ZeroQuantity));
    for name in ["s022-missing-minute-taints", "s024-one-candle-day", "s037-outage-day-2021-02-24", "s039-membership-unknown-day", "s043-withheld-day", "s044-fold-error-month"] {
        assert!(output(name).records.is_empty(), "{name} traded");
    }
    let refused = output("s047-trading-start-month-absent");
    assert!(refused.stocks[0].refused.is_some());
    assert!(run(&by_name("s061-charge-refusal-outside-cost-window").market).is_err());
    assert!(run(&by_name("s062-two-stocks-one-spelling").market).is_err());
}

#[test]
fn fallback_scenarios_fill_on_minutes_and_late_fills_are_counted() {
    for name in ["s030-gdfl-month-missing-fallback", "s031-gdfl-month-refused-fallback", "s032-day-without-seconds-fallback", "s035-vendor-late-day-2019-05-30"] {
        let out = output(name);
        assert!(out.records.values().flatten().filter_map(|r| r.trade).all(|t| t.sim.fallback), "{name}");
        assert!(out.records.values().flatten().any(|r| r.trade.is_some()), "{name}");
    }
    let late = output("s015-late-fill-over-60-seconds");
    assert!(late.records.values().flatten().filter_map(|r| r.trade).any(|t| t.sim.late));
}

#[test]
fn the_slots_and_the_exact_months_hold() {
    let out = output("s051-twenty-slots-full-22-ties");
    let row = out.rows.iter().find(|r| r.key.set == 0 && r.key.k == 1).unwrap();
    let list = &out.records[&(0, 1)];
    let first_t = list[0].candidate.t;
    let first: Vec<&str> = row
        .admitted
        .iter()
        .map(|i| &list[*i].candidate)
        .filter(|c| c.t == first_t)
        .map(|c| c.symbol.as_str())
        .collect();
    let want: Vec<String> = (1..=20).map(|i| format!("S{i:02}")).collect();
    assert_eq!(first, want.iter().map(String::as_str).collect::<Vec<_>>());
    assert!(row.counts.slots_full >= 2);
    for (name, bp, pass) in [("s055-month-exactly-2-5000-percent-passes", 250, true), ("s056-month-2-4999-percent-fails", 249, false)] {
        let out = output(name);
        for row in out.rows.iter().filter(|r| r.key.view != crate::book::View::Short) {
            assert_eq!(row.months[0].bp, Some(bp), "{name} {:?}", row.key);
            assert_eq!(row.months[0].pass, pass);
        }
    }
    let zero = output("s059-account-at-or-below-zero");
    assert!(zero.rows.iter().any(|r| r.months.last().unwrap().bp.is_none()));
}

#[test]
fn random_stories_are_reproducible_and_varied() {
    assert_eq!(random(3, 99), random(3, 99));
    assert_ne!(random(3, 99).market, random(4, 99).market);
    let crowds = (0..115).filter(|i| random(*i, 1).market.stocks.len() == 22).count();
    assert!(crowds > 0);
}

#[test]
fn helpers_compute_their_instants() {
    assert_eq!(idx(9, 15), 0);
    assert_eq!(idx(15, 29), 374);
    assert_eq!(idx(9, 0), 0);
    assert_eq!(sec(15, 7, 59), crate::time::TIME_EXIT);
}

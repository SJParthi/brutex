#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a test that cannot panic cannot fail."
)]

use super::*;
use crate::generator::{DayPlan, StockPlan, calendar, day, market};
use crate::market::Window;

fn gap_market() -> Market {
    let cal = calendar(&[]);
    let warm = [Month::of(2018, 7), Month::of(2018, 8)];
    let quiet = DayPlan::flat(100_000, 375);
    let event = DayPlan::flat(100_000, 375).level(30, 375, 101_000);
    let stock = StockPlan::new("S01")
        .warmup(&cal, &warm, 100_000)
        .day(day(2018, 9, 3), &quiet, true)
        .day(day(2018, 9, 4), &event, true)
        .build();
    market(Window { first: Month::of(2018, 9), last: Month::of(2018, 9) }, cal, vec![stock])
}

#[test]
fn a_jump_after_a_quiet_spell_is_a_signal_and_a_trade() {
    let out = run(&gap_market()).unwrap();
    for ((set, k), list) in &out.records {
        for r in list {
            eprintln!("set {set} k {k} {:?} {} {:?}", r.candidate.day, r.candidate.t, r.candidate.outcome);
        }
    }
    let vo1 = out.records.get(&(0, 1)).unwrap();
    assert!(!vo1.is_empty());
}

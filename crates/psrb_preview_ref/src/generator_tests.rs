#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a test that cannot panic cannot fail."
)]

use super::*;
use crate::time::Month;

#[test]
fn a_day_plan_builds_minutes_and_one_second_per_traded_minute() {
    let plan = DayPlan::flat(100, 3).minute(1, Bar::flat(101, VOLUME)).second(OPEN + 30, Bar::flat(500, 2));
    let sd = plan.build();
    assert_eq!(sd.minutes.len(), 3);
    assert_eq!(sd.seconds.iter().map(|s| s.at).collect::<Vec<_>>(), [OPEN + 1, OPEN + 30, OPEN + 61, OPEN + 121]);
    assert_eq!(sd.seconds[2].bar.close, 101);
    let split = DayPlan::flat(100, 1).factor(5).build();
    assert_eq!(split.seconds[0].bar.close, 500);
    let none = DayPlan::flat(100, 2).seconds_rule(Seconds::None).build();
    assert!(none.seconds.is_empty());
    let dropped = DayPlan::flat(100, 3).drop_minute(1).drop_second(OPEN + 1).build();
    assert_eq!((dropped.minutes.len(), dropped.seconds.len()), (2, 1));
    let padded = DayPlan::flat(100, 3).pad(1, 3).build();
    assert!(padded.minutes[1].padded() && padded.seconds.len() == 1);
    let first_padded = DayPlan::default().pad(0, 1).build();
    assert_eq!(first_padded.minutes[0].bar.close, 100_000);
    let halted = DayPlan::flat(100, 3).halt(OPEN + 60).build();
    assert_eq!(halted.seconds.len(), 1);
    let cut = DayPlan::flat(100, 375).truncate(360).build();
    assert_eq!(cut.minutes.len(), 360);
    let level = DayPlan::flat(100, 4).level(2, 4, 105).build();
    assert_eq!(level.minutes[3].bar.close, 105);
}

#[test]
fn a_stock_plan_records_its_months_and_flags() {
    let cal = calendar(&[(day(2018, 7, 2), DayKind::Closed)]);
    let s = StockPlan::new("S")
        .warmup(&cal, &[Month::of(2018, 7)], 100)
        .day(day(2018, 9, 4), &DayPlan::flat(100, 2), false)
        .gdfl_month(Month::of(2018, 10), true)
        .zerodha_month(Month::of(2018, 11))
        .withhold(day(2018, 9, 4))
        .build();
    assert_eq!(s.name, "S");
    assert!(s.zerodha_months.contains(&Month::of(2018, 7)) && s.zerodha_months.contains(&Month::of(2018, 11)));
    assert!(!s.gdfl_months.contains(&Month::of(2018, 9)) && s.gdfl_refused.contains(&Month::of(2018, 10)));
    assert!(s.withheld.contains(&day(2018, 9, 4)));
    assert!(!s.days.contains_key(&day(2018, 7, 2)) && s.days.contains_key(&day(2018, 7, 3)));
    let gd = StockPlan::new("T").day(day(2018, 9, 4), &DayPlan::flat(100, 2), true).gdfl_month(Month::of(2018, 12), false).build();
    assert!(gd.gdfl_months.contains(&Month::of(2018, 9)) && gd.gdfl_refused.is_empty());
}

#[test]
fn markets_answers_probes_and_draws() {
    let w = Window { first: Month::of(2018, 9), last: Month::of(2018, 9) };
    let m = market(w, calendar(&[]), vec![StockPlan::new("A").build()]);
    assert!(m.membership.listed.contains("A"));
    let m = with_answer(m, "A", day(2018, 9, 4), Answer::No);
    assert_eq!(m.membership.answers.len(), 1);
    let p = Probe { opened: false, reliance_first: Answer::No, reliance_last: Answer::No, syrma_last: Answer::No };
    assert!(!with_probe(m, p).membership.probe.opened);
    assert_eq!(day(2018, 2, 30), Day::from_index(0));
    let mut d = Draw::new(SEEDS[0]);
    let mut e = Draw::new(SEEDS[0]);
    assert_eq!(d.int(1, 9), e.int(1, 9));
    assert!((3..=4).contains(&d.index(3, 4)));
    assert!(!d.chance(0, 1));
}

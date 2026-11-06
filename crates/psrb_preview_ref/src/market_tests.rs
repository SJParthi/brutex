#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a test that cannot panic cannot fail."
)]

use super::*;

#[test]
fn the_calendar_reads_weekdays_and_overrides() {
    let monday = Day::from_ymd(2018, 9, 3).unwrap();
    let saturday = Day::from_ymd(2018, 9, 8).unwrap();
    let mut cal = Calendar::default();
    assert_eq!(cal.kind_of(monday), DayKind::Full);
    assert_eq!(cal.kind_of(saturday), DayKind::Closed);
    cal.overrides.insert(monday, DayKind::Muhurat);
    cal.overrides.insert(saturday, DayKind::Full);
    assert_eq!(cal.kind_of(monday), DayKind::Muhurat);
    assert_eq!(cal.full_days(monday, Day::from_ymd(2018, 9, 9).unwrap()).len(), 5);
    assert!(cal.full_days(saturday, monday).is_empty());
}

#[test]
fn bars_minutes_and_windows() {
    assert_eq!(Bar::flat(5, 0), Bar::new(5, 5, 5, 5, 0));
    let m = Minute { start: 33_300, bar: Bar::flat(5, 0) };
    assert!(m.padded());
    assert!(!Minute { start: 33_300, bar: Bar::flat(5, 1) }.padded());
    let w = Window { first: Month::of(2018, 11), last: Month::of(2019, 2) };
    assert_eq!(w.months().len(), 4);
    assert!(Window { first: Month::of(2019, 2), last: Month::of(2018, 11) }.months().is_empty());
    let s = Stock::default();
    assert!(s.days.is_empty() && s.zerodha_months.is_empty());
    assert_eq!(StockDay::default(), StockDay { minutes: vec![], seconds: vec![] });
}

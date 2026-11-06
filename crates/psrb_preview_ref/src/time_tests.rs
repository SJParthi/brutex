#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a test that cannot panic cannot fail."
)]

use super::*;

#[test]
fn days_round_trip_through_their_civil_dates() {
    let d = Day::from_ymd(2018, 9, 3).unwrap();
    assert_eq!(d.ymd(), (2018, 9, 3));
    assert_eq!(d.label(), "2018-09-03");
    assert_eq!(d.weekday(), 0);
    assert_eq!(d.month(), Month::of(2018, 9));
    assert_eq!(d.next().ymd(), (2018, 9, 4));
    assert_eq!(Day::from_index(d.index()), d);
    assert_eq!(Day::from_ymd(1970, 1, 1).unwrap().index(), 0);
    assert_eq!(Day::from_ymd(1970, 1, 1).unwrap().weekday(), 3);
}

/// X: calendar edges (leap rules both ways, month ends, refused fields).
#[test]
fn x_calendar_edges() {
    let cases: [(i32, u32, u32, bool); 12] = [
        (2020, 2, 29, true),  // leap: divisible by 4, not by 100
        (2019, 2, 29, false), // not divisible by 4
        (2100, 2, 29, false), // divisible by 100, not by 400 (combo: both rules)
        (2000, 2, 29, true),  // divisible by 400 (combo)
        (2018, 4, 31, false),
        (2018, 12, 31, true),
        (2018, 13, 1, false),
        (2018, 0, 1, false),
        (2018, 1, 0, false),
        (1969, 12, 31, false), // oob: before the supported years
        (2200, 1, 1, false),   // oob: after
        (2199, 12, 31, true),
    ];
    for (y, m, d, ok) in cases {
        assert_eq!(Day::from_ymd(y, m, d).is_some(), ok, "{y}-{m}-{d}");
    }
    assert_eq!(Month::of(2000, 2).days().len(), 29);
    assert_eq!(Month::of(2100, 2).days().len(), 28);
    assert_eq!(Month::of(2018, 6).days().len(), 30);
    assert_eq!(Month::of(2300, 1).days().len(), 0);
}

#[test]
fn months_step_and_label() {
    let m = Month::of(2018, 12);
    assert_eq!(m.ym(), (2018, 12));
    assert_eq!(m.next().ym(), (2019, 1));
    assert_eq!(m.next().prev(), m);
    assert_eq!(m.plus(-12).ym(), (2017, 12));
    assert_eq!(m.label(), "2018-12");
    assert_eq!(m.ordinal(), 2018 * 12 + 11);
    assert_eq!(m.first_day().unwrap().label(), "2018-12-01");
    assert_eq!(m.start(), m.first_day().unwrap());
    assert_eq!(m.end().label(), "2018-12-31");
    assert_eq!(Month::of(2300, 1).first_day(), None);
    assert_eq!(
        Month::of(2300, 1).end().index() + 1,
        Month::of(2300, 2).start().index()
    );
}

#[test]
fn ist_seconds_and_utc_instants() {
    assert_eq!(ist(9, 15, 0), Some(OPEN));
    assert_eq!(ist(15, 7, 59), Some(TIME_EXIT));
    assert_eq!(ist(24, 0, 0), None);
    assert_eq!(ist(9, 60, 0), None);
    assert_eq!(ist(9, 15, 60), None);
    assert_eq!(hms(SQUARE_OFF), "15:10:00");
    assert_eq!(hm(CLOSE_AUCTION), "15:15");
    let d = Day::from_ymd(2018, 9, 3).unwrap();
    let u = utc_micros(d, OPEN);
    assert_eq!(from_utc_micros(u), Some((d, OPEN)));
    // 09:15 IST is 03:45 UTC; IST midnight is the previous UTC day's 18:30.
    assert_eq!(
        u.rem_euclid(86_400_000_000),
        (3 * 3_600 + 45 * 60) * 1_000_000
    );
    assert_eq!(from_utc_micros(utc_micros(d, 0)), Some((d, 0)));
    assert_eq!(from_utc_micros(u + 1), None);
    assert_eq!(from_utc_micros(i64::MIN / 2), None);
}

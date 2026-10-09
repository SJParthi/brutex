//! The five production copies of the civil-date conversion name the same day.
//! D-3521 (ONEAUTH-23).
//!
//! Howard Hinnant's algorithm is written in `telemetry::clock`, `costs::day`,
//! `store::path`, `pull::session` and `cli::stability`, because no one crate
//! can hold it for all five: `telemetry` depends on nothing (gate 21), and
//! `costs` and `store` name neither `telemetry` nor `pull` (`CLAUDE.md` §5).
//! Lens L4 recorded in `docs/06-limits.md` (round 2) that each copy is tested
//! where it lives and that no test compared them with one another, so a copy
//! edited wrongly in one crate would agree with its own tests and nobody else.
//! `cli` names all five crates; this test compares them on every day
//! `pull::session::Day` admits, 1970-01-01 to 9999-12-31.

#![allow(
    clippy::expect_used,
    reason = "the exception every test module in this workspace takes."
)]

use pull::session::{Day, IST_OFFSET_SECS, MAX_DAY_NUMBER};

#[test]
fn every_civil_date_copy_names_the_same_day() {
    let mut months = 0_u32;
    let mut trade_days = 0_u32;
    let mut previous_until = None;
    for number in 0..=MAX_DAY_NUMBER {
        let day = Day::from_days(number).expect("a day pull admits");
        assert_eq!(day.days_from_epoch(), number, "pull round trip at {number}");
        let (year, month, date) = (day.year(), day.month(), day.day());
        let days = i64::from(number);

        assert_eq!(
            telemetry::civil_from_days(days),
            (i64::from(year), i64::from(month), i64::from(date)),
            "telemetry::clock at day {number}"
        );

        // Noon IST, so the bucket's own IST shift cannot move the day.
        let noon = (days * 86_400 + 6 * 3_600 + 30 * 60) * 1_000_000;
        assert_eq!(
            cli::stability::Grain::Month.bucket(noon),
            i64::from(year) * 12 + i64::from(month) - 1,
            "cli::stability at day {number}"
        );

        if (costs::day::FIRST_YEAR..=costs::day::LAST_YEAR).contains(&year) {
            let ordinal = i32::try_from(number).expect("inside costs' window");
            let trade = costs::day::TradeDay::from_ordinal(ordinal).expect("inside the window");
            assert_eq!(
                (trade.year(), trade.month(), trade.day()),
                (year, month, date),
                "costs::day at day {number}"
            );
            let back = costs::day::TradeDay::new(year, month, date).expect("a costs date");
            assert_eq!(back.ordinal(), ordinal, "costs::day back at day {number}");
            trade_days += 1;
        }

        if date == 1 {
            let (from, until) = store::path::YearMonth::new(year, month)
                .expect("a month the store files")
                .ist_bounds_micros();
            assert_eq!(
                from,
                (days * 86_400 - IST_OFFSET_SECS) * 1_000_000,
                "store::path at {year}-{month:02}"
            );
            if let Some(previous) = previous_until {
                assert_eq!(
                    previous, from,
                    "store::path months tile at {year}-{month:02}"
                );
            }
            previous_until = Some(until);
            months += 1;
        }
    }
    assert_eq!(months, (9_999 - 1_970 + 1) * 12);
    // 1990-01-01 to 2100-12-31, the window `costs::day` admits.
    assert_eq!(trade_days, 40_542);
}

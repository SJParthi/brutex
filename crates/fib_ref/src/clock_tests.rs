use super::*;

#[test]
fn named_instants_are_the_kits() {
    assert_eq!(hms(OPEN), "09:15:00");
    assert_eq!(hms(CANDLE_END), "15:30:00");
    assert_eq!(hms(SESSION_END), "15:29:59");
    assert_eq!(hms(SESSION_END_AUCTION), "15:14:59");
    assert_eq!(hms(LAST_ENTRY), "15:07:59");
    assert_eq!(hms(CUT), "15:08:00");
    assert_eq!(hms(SQUARE_OFF), "15:10:00");
    assert_eq!(minute_start(SESSION_MINUTES), CANDLE_END);
    assert_eq!(LAMBDA, 14);
}

#[test]
fn minute_of_covers_the_edges() {
    assert_eq!(minute_of(OPEN - 1), None);
    assert_eq!(minute_of(OPEN), Some(0));
    assert_eq!(minute_of(OPEN + 59), Some(0));
    assert_eq!(minute_of(OPEN + 60), Some(1));
    assert_eq!(minute_of(SESSION_END), Some(374));
    assert_eq!(minute_of(CANDLE_END), None);
    assert_eq!(minute_of(0), None);
    assert_eq!(minute_of(u32::MAX), None);
    assert_eq!(minute_start(1), OPEN + 60);
}

#[test]
fn parse_hms_accepts_two_forms_and_refuses_the_rest() {
    assert_eq!(parse_hms("09:15:00"), Some(OPEN));
    assert_eq!(parse_hms("09:15"), Some(OPEN));
    assert_eq!(parse_hms("23:59:59"), Some(86_399));
    assert_eq!(parse_hms("00:00:00"), Some(0));
    assert_eq!(parse_hms("24:00:00"), None);
    assert_eq!(parse_hms("09:60:00"), None);
    assert_eq!(parse_hms("09:15:60"), None);
    assert_eq!(parse_hms("09:15:00:01"), None);
    assert_eq!(parse_hms("9"), None);
    assert_eq!(parse_hms("aa:bb"), None);
    assert_eq!(parse_hms("09:bb"), None);
    assert_eq!(parse_hms("09:15:xx"), None);
    assert_eq!(parse_hms(""), None);
}

#[test]
fn dates_round_trip_and_order() {
    for (y, m, d, days) in [
        (1970, 1, 1, 0),
        (1969, 12, 31, -1),
        (2000, 2, 29, 11_016),
        (2018, 9, 3, 17_777),
        (2026, 11, 7, 20_764),
        (2400, 12, 31, 157_419),
        (1600, 3, 1, -135_080),
        (2020, 7, 1, 18_444),
    ] {
        let date = Date::new(y, m, d).expect("a real day");
        assert_eq!(date.days(), days, "{y}-{m}-{d}");
        assert_eq!(Date::from_days(days), date);
    }
    assert!(Date::new(2026, 10, 30) < Date::new(2026, 11, 1));
    // every day of a leap year round-trips (a combination of both maps)
    let start = Date::new(2024, 1, 1).map_or(0, Date::days);
    for n in start..start + 366 {
        assert_eq!(Date::from_days(n).days(), n);
    }
}

#[test]
fn date_new_refuses_unreal_days() {
    assert_eq!(Date::new(2026, 2, 29), None);
    assert!(Date::new(2024, 2, 29).is_some());
    assert_eq!(Date::new(1900, 2, 29), None);
    assert!(Date::new(2000, 2, 29).is_some());
    assert_eq!(Date::new(2026, 13, 1), None);
    assert_eq!(Date::new(2026, 0, 1), None);
    assert_eq!(Date::new(2026, 4, 31), None);
    assert_eq!(Date::new(2026, 4, 0), None);
    assert_eq!(days_in_month(2026, 13), 0);
}

#[test]
fn month_index_weekday_iso_and_parse() {
    let d = Date::new(2026, 11, 7).unwrap_or(Date { y: 0, m: 1, d: 1 });
    assert_eq!(d.month_index(), 2026 * 12 + 10);
    assert_eq!(month_label(d.month_index()), "2026-11");
    assert_eq!(month_label(2018 * 12 + 8), "2018-09");
    assert_eq!(d.weekday(), 5); // a Saturday
    assert_eq!(Date::new(2026, 10, 5).map(Date::weekday), Some(0)); // Monday
    assert_eq!(d.iso(), "2026-11-07");
    assert_eq!(Date::parse("2026-11-07"), Some(d));
    assert_eq!(Date::parse("2026-11"), None);
    assert_eq!(Date::parse("2026-11-07-01"), None);
    assert_eq!(Date::parse("2026-02-30"), None);
    assert_eq!(Date::parse("x-1-1"), None);
    assert_eq!(Date::parse("2026-x-1"), None);
    assert_eq!(Date::parse("2026-1-x"), None);
}

//! The calendar of record as `/calendar.json` serves it (D-3336): day codes,
//! owed bars per rung, readings, and the answer built from them.
#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "finite generated fixtures and exact assertions"
)]

use super::*;
use pull::session::Day;

/// Days since the epoch of a civil date.
fn at(year: u16, month: u8, day: u8) -> i64 {
    i64::from(
        Day::new(year, month, day)
            .expect("a real day")
            .days_from_epoch(),
    )
}

/// A parsed answer.
fn parse(body: &str) -> serde_json::Value {
    serde_json::from_str(body).expect("an answer is JSON")
}

/// **EVERY CODE COMES FROM THE CALENDAR OF RECORD, AND OUTSIDE IT THERE IS
/// NONE.** D-3336 §3.
#[test]
fn each_day_takes_the_record_code_and_none_outside_the_record() {
    for (day, want) in [
        (at(2015, 1, 5), None),
        (at(2017, 12, 29), None),
        (at(2018, 1, 1), Some(REGULAR)),
        (at(2018, 1, 26), Some(CLOSED)),
        (at(2018, 1, 27), Some(CLOSED)),
        (at(2018, 11, 7), Some(SPECIAL)),
        (at(2019, 10, 27), Some(SPECIAL)),
        (at(2020, 2, 1), Some(SPECIAL)),
        (at(2020, 3, 13), Some(HALTED)),
        (at(2020, 3, 23), Some(HALTED)),
        (at(2021, 2, 24), Some(SPECIAL)),
        (at(2022, 3, 7), Some(REGULAR)),
        (at(2024, 1, 20), Some(SPECIAL)),
        (at(2024, 3, 2), Some(SPECIAL)),
        (at(2026, 7, 31), Some(REGULAR)),
        (at(2026, 8, 2), Some(CLOSED)),
        (at(2026, 8, 3), Some(AUCTION)),
        (at(2026, 9, 14), Some(CLOSED)),
        (at(2026, 9, 30), Some(AUCTION)),
        (pull::calendar::SOURCED_LAST_DAY + 1, None),
    ] {
        assert_eq!(record_code(day), want, "{}", iso(day));
    }
}

/// **NO WEEKEND IS A SESSION UNLESS THE RECORD NAMES ITS SESSION, AND THE
/// MEASURED WINDOW COUNTS EXACTLY WHAT `pull::calendar` COUNTS.** DB-015: a
/// G-sec index alone voted 132 weekend days of 2015 and 2016; the record has no
/// vote to cast.
#[test]
fn a_weekend_is_a_session_only_where_the_record_names_one() {
    let mut weekend_sessions = Vec::new();
    for day in pull::calendar::SOURCED_FIRST_DAY..=pull::calendar::SOURCED_LAST_DAY {
        let code = record_code(day).expect("inside the record");
        if is_session(code) && pull::calendar::weekday_of(day).is_weekend() {
            assert!(
                pull::calendar::special_session(day).is_some(),
                "{} is a weekend session with no recorded special session",
                iso(day)
            );
            assert_eq!(code, SPECIAL, "{}", iso(day));
            weekend_sessions.push(iso(day));
        }
    }
    assert_eq!(
        weekend_sessions,
        [
            "2019-10-27",
            "2020-02-01",
            "2020-11-14",
            "2023-11-12",
            "2024-01-20",
            "2024-03-02",
            "2024-05-18",
            "2025-02-01",
            "2026-02-01",
        ],
        "every weekend session of the record, and nothing else"
    );
    for year in 2020..=2025_u16 {
        let (first, last) = (at(year, 1, 1), at(year, 12, 31));
        let counted = (first..=last)
            .filter(|day| record_code(*day).is_some_and(is_session))
            .count();
        assert_eq!(
            Some(u32::try_from(counted).expect("a year of days")),
            pull::calendar::sessions_between(first, last),
            "{year}: the record's sessions are the measured window's"
        );
    }
}

/// **A REGULAR SESSION OWES 375 MINUTES, 22,500 SECONDS, ONE DAY, AND THE
/// BUCKETS `pull::fold` MAKES OF IT.** D-3336 §1.
#[test]
fn a_regular_session_owes_its_minutes_seconds_and_folded_buckets() {
    let day = at(2024, 1, 1);
    let minutes = regular_minutes();
    let owed = |rung| owed_at(day, &minutes, rung, pull::vendor::Venue::NseIndex);
    assert_eq!(owed(Timeframe::MINUTE_1), Some(375));
    assert_eq!(owed(Timeframe::SECOND_1), Some(22_500));
    assert_eq!(owed(Timeframe::DAY_1), Some(1));
    // THE FOLD'S GRID STARTS AT 09:15, so a rung whose length does not divide
    // 375 ends on one short bar (`pull::fold`, D-1447).
    for (rung, want) in [
        (Timeframe::MINUTE_2, 188),
        (Timeframe::MINUTE_3, 125),
        (Timeframe::MINUTE_5, 75),
        (Timeframe::MINUTE_10, 38),
        (Timeframe::MINUTE_15, 25),
        (Timeframe::MINUTE_30, 13),
        (Timeframe::MINUTE_60, 7),
    ] {
        assert_eq!(owed(rung), Some(want), "{}", rung.as_str());
    }
    // A CAS-ELIGIBLE SHARE'S CONTINUOUS SESSION ENDS AT 15:14 (D-3320).
    assert_eq!(
        owed_at(
            day,
            &cas_minutes(),
            Timeframe::MINUTE_1,
            pull::vendor::Venue::NseCash
        ),
        Some(360)
    );
    assert_eq!(
        owed_at(
            day,
            &cas_minutes(),
            Timeframe::MINUTE_15,
            pull::vendor::Venue::NseCash
        ),
        Some(24),
        "360 minutes divide by 15: no stub"
    );
    let table = owed_table(Timeframe::SECOND_1);
    assert_eq!(table["regular"], 22_500);
    assert!(
        table["casEligible"].is_null(),
        "a one-second share is trade-driven and owes no count"
    );
    assert_eq!(owed_table(Timeframe::MINUTE_1)["casEligible"], 360);
}

/// **A HALTED MINUTE IS NEITHER OWED NOR MISSING, AND A HALTED SECOND IS
/// COUNTED PER SEGMENT.** D-3320, D-3336 §1.
#[test]
fn a_halt_takes_its_minutes_and_its_segments_seconds_out_of_what_is_owed() {
    let day = at(2020, 3, 13);
    let minutes = owed_minutes(day).expect("a sized session");
    assert_eq!(minutes.len(), 375 - 59, "09:22-10:20 halted");
    assert!(!minutes.contains(&562) && !minutes.contains(&620));
    assert!(minutes.contains(&561) && minutes.contains(&621));
    // THE INDEX STOPS AT 09:21:10 AND RESUMES AT 10:21:01: 50 seconds of
    // 09:21 and one of 10:21 are halted too.
    assert_eq!(
        owed_at(
            day,
            &minutes,
            Timeframe::SECOND_1,
            pull::vendor::Venue::NseIndex
        ),
        Some(316 * 60 - 51)
    );
    // THE SHARES STOP AT 09:21:09 AND RESUME AT 10:21:00: 51 seconds of 09:21.
    assert_eq!(
        owed_at(
            day,
            &minutes,
            Timeframe::SECOND_1,
            pull::vendor::Venue::NseCash
        ),
        Some(316 * 60 - 51)
    );
    // THE OUTAGE DAY'S WINDOWS ALREADY EXCLUDE ITS SOURCED HALT.
    let outage = owed_minutes(pull::calendar::SYSTEMS_OUTAGE_DAY).expect("SEBI's windows");
    assert_eq!(outage.len(), 220);
    let muhurat = owed_minutes(at(2018, 11, 7)).expect("a recorded Muhurat window");
    assert_eq!(muhurat.len(), 60);
    assert_eq!(
        owed_at(
            at(2018, 11, 7),
            &muhurat,
            Timeframe::SECOND_1,
            pull::vendor::Venue::NseIndex
        ),
        Some(3_600)
    );
    assert_eq!(owed_minutes(at(2018, 1, 26)), Some(Vec::new()), "closed");
    assert_eq!(owed_minutes(at(2015, 1, 5)), None, "outside the record");
}

/// **THE SPECIAL LIST NAMES EACH SPECIAL OR HALT DAY WITH ITS WINDOWS, WHAT IT
/// OWES, AND THE OUTAGE DAY'S UNPROVED INDEX COUNT AS NULL.**
#[test]
fn the_special_list_sizes_each_recorded_day_and_refuses_the_outage_index_count() {
    let list = specials_json(at(2021, 2, 1), at(2021, 2, 28), Timeframe::MINUTE_1);
    assert_eq!(list.len(), 1, "{list:?}");
    let outage = &list[0];
    assert_eq!(outage["date"], "2021-02-24");
    assert_eq!(outage["code"], "P");
    assert_eq!(outage["owed"], 220);
    assert!(outage["indexOwed"].is_null(), "{outage}");
    assert_eq!(outage["windows"][0]["from"], "09:15");
    assert_eq!(outage["windows"][1]["to"], "16:59");
    let daily = specials_json(at(2021, 2, 1), at(2021, 2, 28), Timeframe::DAY_1);
    assert_eq!(daily[0]["indexOwed"], 1, "a daily bar is still one bar");
    let halts = specials_json(at(2020, 3, 1), at(2020, 3, 31), Timeframe::SECOND_1);
    assert_eq!(halts.len(), 2);
    assert_eq!(halts[0]["kind"], "suspected halt");
    assert_eq!(halts[0]["owed"], 316 * 60 - 51);
    assert_eq!(halts[0]["minuteOwed"], 316);
    let listed = halts_json(at(2020, 3, 1), at(2020, 3, 31));
    assert_eq!(listed.len(), 2);
    assert_eq!(listed[0]["suspected"], true);
    assert_eq!(listed[0]["halted"]["from"], "09:22");
    assert_eq!(listed[0]["seconds"]["index"]["from"], "09:21:10");
    assert_eq!(
        halts_json(at(2021, 2, 24), at(2021, 2, 24))[0]["suspected"],
        false,
        "SEBI's record is sourced"
    );
    assert_eq!(auction_start(), Some(at(2026, 8, 3)));
}

/// **A READING SAYS HELD, READ-AND-EMPTY, OR NOT READ, AND NOTHING ELSE.**
#[test]
fn a_reading_tells_held_from_read_and_empty_from_not_read() {
    let calendar = pull::calendar::Calendar::from_observed(&[
        pull::calendar::Observed::from_runs(at(2025, 5, 2), &[]),
        pull::calendar::Observed::from_runs(at(2025, 5, 6), &[(555, 929)]),
    ]);
    let daily = Reading::from_calendar("NIFTY", &calendar);
    assert_eq!(daily.at(at(2025, 5, 2)), Some(1));
    assert_eq!(
        daily.at(at(2025, 5, 5)),
        Some(0),
        "closed between two sessions"
    );
    assert_eq!(daily.at(at(2025, 5, 1)), None, "before the span");
    assert_eq!(daily.at(at(2025, 5, 7)), None, "after the span");
    assert_eq!(daily.held_span(), Some((at(2025, 5, 2), at(2025, 5, 6))));

    let month = |y, m| store::path::YearMonth::new(y, m).expect("a month");
    let mut counts = [0_u32; 31];
    counts[2] = 22_400;
    counts[9] = 7;
    let days = crate::calendar_of::tests::month_days_of(counts);
    let seconds = Reading::from_months(
        "NIFTY",
        &[(month(2022, 3), Some(days)), (month(2022, 5), None)],
    );
    assert_eq!(seconds.at(at(2022, 3, 3)), Some(22_400));
    assert_eq!(seconds.at(at(2022, 3, 7)), Some(0));
    assert_eq!(seconds.at(at(2022, 4, 4)), None, "a month not held");
    assert_eq!(
        seconds.at(at(2022, 5, 4)),
        None,
        "a month that would not read"
    );
    assert_eq!(seconds.held_span(), Some((at(2022, 3, 3), at(2022, 3, 10))));
    assert_eq!(Reading::from_months("NONE", &[]).held_span(), None);
}

/// **A SESSION THE FEED HOLDS NO BAR ON STAYS A SESSION, AND IS NAMED.** The
/// GDFL index files of 2022-03-07 were refused at import (D-3318): the day is
/// `R`, its held digit `0`, and it is listed in `sessionsNotHeld`. A session
/// one reference misses is listed as partly held.
#[test]
fn a_session_no_reference_holds_is_kept_and_named() {
    let month = store::path::YearMonth::new(2022, 3).expect("a month");
    let mut nifty = [0_u32; 31];
    let mut bank = [0_u32; 31];
    for (offset, count) in nifty.iter_mut().enumerate() {
        let day = at(2022, 3, 1) + i64::try_from(offset).expect("small");
        if record_code(day).is_some_and(is_session) && day != at(2022, 3, 7) {
            *count = 22_000;
        }
    }
    bank.copy_from_slice(&nifty);
    bank[9] = 0; // 2022-03-10: BANKNIFTY alone is missing.
    let read = |name, counts| {
        Reading::from_months(
            name,
            &[(
                month,
                Some(crate::calendar_of::tests::month_days_of(counts)),
            )],
        )
    };
    let references = [read("NIFTY", nifty), read("BANKNIFTY", bank)];
    let asked = Asked {
        feed: "gdfl",
        rung: Timeframe::SECOND_1,
        rung_asked: false,
        target: Target::Exchange,
        generation: Some(7),
    };
    let answer = parse(&answer(&asked, &references, &references, &[]));
    assert_eq!(answer["status"], "ok");
    assert_eq!(answer["fromDate"], "2022-03-02", "the first day held");
    assert_eq!(answer["toDate"], "2022-03-31");
    let days = answer["days"].as_str().expect("a status string");
    let held = answer["held"].as_str().expect("a held string");
    let index = usize::try_from(at(2022, 3, 7) - at(2022, 3, 2)).expect("inside");
    assert_eq!(&days[index..=index], "R");
    assert_eq!(&held[index..=index], "0");
    assert_eq!(answer["sessionsNotHeld"]["count"], 1);
    assert_eq!(answer["sessionsNotHeld"]["days"][0]["date"], "2022-03-07");
    assert_eq!(
        answer["sessionsPartlyHeld"]["days"][0]["date"],
        "2022-03-10"
    );
    assert_eq!(
        answer["calendarSessions"].as_u64(),
        Some(answer["sessions"].as_u64().expect("held sessions") + 1)
    );
    assert_eq!(answer["owed"]["regular"], 22_500);
    assert_eq!(
        answer["derivedFrom"],
        serde_json::json!(["NIFTY", "BANKNIFTY"])
    );
    assert_eq!(answer["withheld"], serde_json::json!([]));
    assert_eq!(answer["generation"], 7);
}

/// **OUTSIDE THE RECORD ONLY THE REFERENCES SPEAK.** A weekend bar is an
/// observed session of unrecorded length and is named; a day read and empty
/// is closed; a day no reference read is `-`, and withheld.
#[test]
fn outside_the_record_a_reference_bar_is_an_unsized_session_and_silence_is_closed() {
    let calendar = pull::calendar::Calendar::from_observed(&[
        pull::calendar::Observed::from_runs(at(2015, 2, 27), &[]),
        pull::calendar::Observed::from_runs(at(2015, 2, 28), &[]),
        pull::calendar::Observed::from_runs(at(2015, 3, 2), &[]),
    ]);
    let nifty = Reading::from_calendar("NIFTY", &calendar);
    let asked = Asked {
        feed: "zerodha",
        rung: Timeframe::DAY_1,
        rung_asked: true,
        target: Target::Exchange,
        generation: None,
    };
    let answer = parse(&answer(
        &asked,
        std::slice::from_ref(&nifty),
        std::slice::from_ref(&nifty),
        &[],
    ));
    assert_eq!(answer["days"], "UU.U", "Fri, Sat, Sun closed, Mon");
    assert_eq!(answer["held"], "1101");
    assert_eq!(answer["calendarSessions"], 3);
    assert_eq!(answer["observedWeekendSessions"]["count"], 1);
    assert_eq!(
        answer["observedWeekendSessions"]["days"][0]["date"],
        "2015-02-28"
    );
    assert_eq!(answer["basis"]["observed"]["days"], 3);
    // NO REFERENCE, NO OBSERVATION: the same days are outside, and withheld.
    let answer = parse(&answer_with_no_attesters(&asked, &nifty));
    assert_eq!(answer["days"], "----");
    assert_eq!(
        answer["withheld"],
        serde_json::json!([{ "from": at(2015, 2, 27), "to": at(2015, 3, 2) }])
    );
    assert_eq!(answer["calendarSessions"], 0);
}

/// [`answer`] with `nifty` holding and nobody attesting.
fn answer_with_no_attesters(asked: &Asked<'_>, nifty: &Reading) -> String {
    answer(asked, std::slice::from_ref(nifty), &[], &[])
}

/// **A FEED THAT HOLDS NOTHING AT THE RUNG IS WITHHELD WITH ITS REASON, NEVER
/// AN EMPTY LIST.** DB-G85: GDFL answered `{"sessions":0,"days":[]}`.
#[test]
fn nothing_held_is_withheld_with_a_reason() {
    let asked = Asked {
        feed: "gdfl",
        rung: Timeframe::DAY_1,
        rung_asked: true,
        target: Target::Exchange,
        generation: Some(3),
    };
    let empty = Reading::from_months("NIFTY", &[]);
    let answer = parse(&answer(&asked, &[empty], &[], &[]));
    assert_eq!(answer["status"], "withheld");
    assert!(
        answer["reason"]
            .as_str()
            .is_some_and(|why| why.contains("1day") && why.contains("NIFTY")),
        "{answer}"
    );
    assert_eq!(answer["days"], "");
    assert_eq!(answer["sessions"], 0);
    let named = parse(&withheld(
        "zerodha",
        None,
        Target::Series {
            exchange: "NSE",
            segment: "CASH",
            symbol: "X",
        },
        None,
        "why",
    ));
    assert_eq!(named["target"]["symbol"], "X");
    assert!(named["rung"].is_null());
    assert_eq!(named["reason"], "why");
}

/// **THE ANSWER IS SMALL.** DB-G85: 6.93 MB on every hit. A whole Zerodha
/// span, 2015-01 to 2026-09 with two references, is a few tens of kilobytes.
#[test]
fn a_whole_zerodha_span_is_tens_of_kilobytes() {
    let observed: Vec<pull::calendar::Observed> = (at(2015, 1, 1)..=at(2026, 9, 30))
        .filter(|day| !pull::calendar::weekday_of(*day).is_weekend())
        .map(|day| pull::calendar::Observed::from_runs(day, &[]))
        .collect();
    let calendar = pull::calendar::Calendar::from_observed(&observed);
    let readings = [
        Reading::from_calendar("NIFTY", &calendar),
        Reading::from_calendar("BANKNIFTY", &calendar),
    ];
    let asked = Asked {
        feed: "zerodha",
        rung: Timeframe::DAY_1,
        rung_asked: false,
        target: Target::Exchange,
        generation: None,
    };
    let body = answer(&asked, &readings, &readings, &[]);
    assert!(body.len() < 64 * 1024, "{} bytes", body.len());
    let parsed = parse(&body);
    let days = parsed["days"].as_str().expect("days");
    assert_eq!(days.len(), parsed["held"].as_str().expect("held").len());
    assert_eq!(
        i64::try_from(days.len()).expect("small"),
        at(2026, 9, 30) - at(2015, 1, 1) + 1
    );
}

/// **DB-B42: WIDENED TO THE RECORD, A DAY PAST THE HELD SPAN IS THE RECORD'S,
/// NEVER A GUESSED WEEKDAY.** A GDFL-like answer held 2022-03-02..2022-03-31 is
/// widened to 2018-01-01..2026-10-04: 2026-10-02 (Gandhi Jayanti) reads
/// closed, 2026-09-30 an auction session, every added day is held `-`, the
/// held count and the held span are unchanged, and the added sessions are
/// counted and listed as unread, in day order.
#[test]
fn a_held_answer_widened_to_the_record_reads_the_records_codes_beyond_it() {
    let month = store::path::YearMonth::new(2022, 3).expect("a month");
    let mut counts = [0_u32; 31];
    for (offset, count) in counts.iter_mut().enumerate() {
        let day = at(2022, 3, 1) + i64::try_from(offset).expect("small");
        if record_code(day).is_some_and(is_session) && day != at(2022, 3, 1) {
            *count = 22_000;
        }
    }
    let read = Reading::from_months(
        "NIFTY",
        &[(
            month,
            Some(crate::calendar_of::tests::month_days_of(counts)),
        )],
    );
    let asked = Asked {
        feed: "gdfl",
        rung: Timeframe::SECOND_1,
        rung_asked: false,
        target: Target::Exchange,
        generation: Some(7),
    };
    let held_body = answer(
        &asked,
        std::slice::from_ref(&read),
        std::slice::from_ref(&read),
        &[],
    );
    let held = parse(&held_body);
    let widened = parse(&widen_to_record(&held_body, None).expect("an ok answer widens"));
    assert_eq!(widened["span"], "record");
    assert_eq!(widened["fromDate"], "2018-01-01");
    assert_eq!(widened["toDate"], "2026-10-04");
    assert_eq!(widened["heldSpan"]["from"], held["fromDate"]);
    assert_eq!(widened["heldSpan"]["to"], held["toDate"]);
    assert_eq!(
        widened["sessions"], held["sessions"],
        "what is held is unchanged"
    );
    let days = widened["days"].as_str().expect("days");
    let held_digits = widened["held"].as_str().expect("held");
    let from = widened["from"].as_i64().expect("from");
    assert_eq!(days.len(), held_digits.len());
    assert_eq!(
        i64::try_from(days.len()).expect("small"),
        pull::calendar::SOURCED_LAST_DAY - pull::calendar::SOURCED_FIRST_DAY + 1
    );
    let code_on = |day: i64| {
        let index = usize::try_from(day - from).expect("inside");
        (&days[index..=index], &held_digits[index..=index])
    };
    assert_eq!(
        code_on(at(2026, 10, 2)),
        (".", "-"),
        "Gandhi Jayanti is closed by the record"
    );
    assert_eq!(code_on(at(2026, 9, 30)), ("X", "-"));
    assert_eq!(code_on(at(2018, 1, 1)), ("R", "-"));
    assert_eq!(
        code_on(at(2022, 3, 7)).1,
        "1",
        "the held digits are the store's"
    );
    let added = widened["calendarSessions"].as_u64().expect("count")
        - held["calendarSessions"].as_u64().expect("count");
    let listed = widened["sessionsUnread"]["count"].as_u64().expect("count")
        - held["sessionsUnread"]["count"].as_u64().expect("count");
    assert_eq!(added, listed, "every added session is unread, never held");
    assert!(added > 2000, "eight years of sessions were added: {added}");
    assert_eq!(widened["sessionsUnread"]["days"][0]["date"], "2018-01-01");
    assert_eq!(widened["sessionsUnread"]["truncated"], true);
    // THE RECORD'S SPECIAL DAYS ACROSS THE WHOLE SPAN, not the held month's.
    assert!(
        widened["special"]
            .as_array()
            .expect("a list")
            .iter()
            .any(|day| day["date"] == "2018-11-07"),
        "{}",
        widened["special"]
    );
    // A WITHHELD ANSWER IS NOT WIDENED.
    let none = withheld("gdfl", None, Target::Exchange, None, "nothing held");
    assert_eq!(widen_to_record(&none, None), None);
}

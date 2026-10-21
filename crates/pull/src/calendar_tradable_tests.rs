//! The tradable-day rule (CAL-1, D-5150..D-5159): `tradable_day`, the
//! computed spans, the measured versions and their digests, and the running
//! sums. Every expected value below is a calendar fact read from a table's
//! own row or an NSE circular, never a value printed by the code under test;
//! every synthetic table is invented and says so.

#![cfg(test)]
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]

use super::*;
use std::fmt::Write as _;

fn at(y: u16, m: u8, d: u8) -> i64 {
    i64::from(
        crate::session::Day::new(y, m, d)
            .expect("a real day")
            .days_from_epoch(),
    )
}

/// **A BUDGET WEEKEND IS NOT TRADABLE, THOUGH `kind_of` CALLS IT A FULL
/// SESSION.** The attack's H5: a rule written on `kind_of` alone admits all
/// three. Before D-5150 nothing refused them.
#[test]
fn a_budget_weekend_is_not_tradable_though_kind_of_calls_it_full() {
    for day in [at(2020, 2, 1), at(2025, 2, 1), at(2026, 2, 1)] {
        assert_eq!(kind_of(day), DayKind::Open(Session::full()), "{day}");
        assert!(weekday_of(day).is_weekend(), "{day}");
        assert_eq!(tradable_day(day), Err(NotTradable::BudgetWeekend), "{day}");
        assert!(NotTradable::BudgetWeekend.is_session());
    }
}

/// **2024-01-20, A SATURDAY OF REGULAR HOURS, IS NOT TRADABLE** (D-5150: the
/// operator's "any other special session"), and its neighbours are.
#[test]
fn the_saturday_full_session_of_2024_01_20_is_not_tradable() {
    let day = at(2024, 1, 20);
    assert_eq!(kind_of(day), DayKind::Open(Session::full()));
    assert_eq!(tradable_day(day), Err(NotTradable::WeekendFullSession));
    assert_eq!(tradable_day(at(2024, 1, 19)), Ok(()));
    assert_eq!(tradable_day(at(2024, 1, 21)), Err(NotTradable::Weekend));
    // 2024-01-22, the Ram Mandir closure: a weekday the exchange shut.
    assert_eq!(tradable_day(at(2024, 1, 22)), Err(NotTradable::Holiday));
    assert_eq!(tradable_day(at(2024, 1, 23)), Ok(()));
}

/// **A HALT DAY IS NOT TRADABLE.** Both March 2020 circuit breakers are full
/// sessions on `kind_of`; the 2021-02-24 outage is counted as the outage it
/// is and is a halt as well. Every row of `HALTS` is refused.
#[test]
fn a_halt_day_is_not_tradable() {
    for day in [at(2020, 3, 13), at(2020, 3, 23)] {
        assert_eq!(kind_of(day), DayKind::Open(Session::full()), "{day}");
        assert!(halt_on(day).is_some_and(|h| h.evidence.is_suspected()));
        assert_eq!(tradable_day(day), Err(NotTradable::MarketHalt), "{day}");
    }
    let outage = at(2021, 2, 24);
    assert!(halt_on(outage).is_some());
    assert_eq!(tradable_day(outage), Err(NotTradable::SystemsOutage));
    for halt in HALTS {
        assert!(tradable_day(halt.day).is_err(), "{}", halt.day);
    }
    // The sessions either side of the first halt are ordinary.
    assert_eq!(tradable_day(at(2020, 3, 12)), Ok(()));
    assert_eq!(tradable_day(at(2020, 3, 16)), Ok(()));
}

/// **A MUHURAT OF UNMEASURED LENGTH IS NOT TRADABLE, AND IS NAMED A
/// MUHURAT**: the five `kind_of` leaves unsized, the two of the sourced span
/// `sourced_kind_of` leaves unsized, and the 2026 one no circular has timed.
#[test]
fn an_unmeasured_muhurat_is_not_tradable_and_is_named_a_muhurat() {
    for day in LENGTH_UNMEASURED {
        assert_eq!(kind_of(day), DayKind::OpenLengthUnmeasured, "{day}");
        assert_eq!(tradable_day(day), Err(NotTradable::Muhurat), "{day}");
    }
    for day in SOURCED_SPECIAL {
        assert_eq!(sourced_kind_of(day), DayKind::OpenLengthUnmeasured, "{day}");
        assert_eq!(tradable_day(day), Err(NotTradable::Muhurat), "{day}");
    }
    let next = at(2026, 11, 8);
    assert_eq!(special_session(next), None);
    assert_eq!(sourced_kind_of(next), DayKind::OpenLengthUnmeasured);
    assert_eq!(tradable_day(next), Err(NotTradable::Muhurat));
    // The 2025 Muhurat, a weekday afternoon, is one too.
    assert_eq!(tradable_day(at(2025, 10, 21)), Err(NotTradable::Muhurat));
}

/// **A NORMAL DAY STAYS TRADABLE**, in the measured window and on both sides
/// of it; a drill does not; **a measured short day does** (the judge's fix of
/// 16 Oct 2026, operator item 5): its missing minutes are the readers'
/// question, not the calendar's.
#[test]
fn a_normal_day_stays_tradable_drills_do_not_and_short_days_do() {
    for day in [
        at(2018, 1, 1),
        at(2018, 9, 3),
        at(2018, 10, 1),
        at(2019, 3, 1),
        at(2019, 12, 2),
        at(2024, 3, 1),
        at(2024, 3, 4),
        at(2026, 9, 4),
        at(2026, 9, 21),
        at(2026, 9, 30),
        at(2026, 11, 9),
        at(2026, 12, 31),
    ] {
        assert_eq!(tradable_day(day), Ok(()), "{day}");
    }
    for day in [at(2024, 3, 2), at(2024, 5, 18)] {
        assert_eq!(
            tradable_day(day),
            Err(NotTradable::DisasterRecoveryDrill),
            "{day}"
        );
    }
    for short in SHORT_DAYS {
        // 2019-08-02 lies in the sourced span, before `kind_of`'s window.
        assert_eq!(sourced_kind_of(short.day), DayKind::Open(Session::full()));
        assert_eq!(tradable_day(short.day), Ok(()), "{}", short.day);
    }
    assert_eq!(tradable_day(at(2018, 1, 26)), Err(NotTradable::Holiday));
    assert_eq!(tradable_day(at(2019, 12, 25)), Err(NotTradable::Holiday));
    assert_eq!(tradable_day(at(2026, 12, 25)), Err(NotTradable::Holiday));
}

/// **OUTSIDE THE RECORD IS `Unmeasured`, ITS OWN REASON, NEVER `Unknown`
/// AND NEVER GUESSED**, at both edges and at both ends of `i64`. It is a
/// known gap (a year whose circular is owed), not an unexplained day.
#[test]
fn outside_the_record_is_unmeasured_its_own_reason() {
    for day in [
        SOURCED_FIRST_DAY - 1,
        at(2017, 12, 29),
        SOURCED_LAST_DAY + 1,
        at(2027, 1, 4),
        i64::MIN,
        i64::MAX,
    ] {
        assert_eq!(tradable_day(day), Err(NotTradable::Unmeasured), "{day}");
    }
    assert!(!NotTradable::Unmeasured.is_unexplained());
    assert!(!NotTradable::Unmeasured.is_session());
    assert_ne!(NotTradable::Unmeasured, NotTradable::Unknown);
    // 2018-01-01 (Mon) and 2026-12-31 (Thu) are ordinary sessions: the
    // record answers its own edges.
    assert_eq!(SOURCED_FIRST_DAY, at(2018, 1, 1));
    assert_eq!(SOURCED_LAST_DAY, at(2026, 12, 31));
    assert_eq!(tradable_day(SOURCED_FIRST_DAY), Ok(()));
    assert_eq!(tradable_day(SOURCED_LAST_DAY), Ok(()));
}

/// **THE RULE IS THE SPECIFICATION'S CONDITIONS ON EVERY DAY OF THE
/// RECORD**: `sourced_kind_of == Open(full)` on a weekday and no special
/// session, notice or halt. Inside the measured window that is `kind_of`,
/// and 1,666 of its 1,681 sessions are tradable.
#[test]
fn the_rule_is_the_specs_conditions_on_every_day_of_the_record() {
    let mut admitted_in_window = 0_u32;
    for day in SOURCED_FIRST_DAY - 3..=SOURCED_LAST_DAY + 3 {
        let expected = sourced_kind_of(day) == DayKind::Open(Session::full())
            && !weekday_of(day).is_weekend()
            && special_session(day).is_none()
            && session_circular(day).is_none()
            && halt_on(day).is_none();
        assert_eq!(tradable_day(day).is_ok(), expected, "day {day}");
        if (FIRST_DAY..=LAST_DAY).contains(&day) {
            assert_eq!(sourced_kind_of(day), kind_of(day), "day {day}");
            admitted_in_window += u32::from(expected);
        }
    }
    // 1,681 sessions in the window (module header), less the four IRREGULAR,
    // the five unsized Muhurats, the three Budget weekends, 2024-01-20 and
    // the two suspected halts. The short day inside it (2022-03-07) stays.
    assert_eq!(sessions_between(FIRST_DAY, LAST_DAY), Some(1_681));
    assert_eq!(admitted_in_window, 1_681 - 4 - 5 - 3 - 1 - 2);
}

/// **EVERY SPECIAL DAY HAS ITS NOTICE, AND THE NOTICE'S WINDOW IS THE
/// MEASURED ONE** (D-5152): the circulars and the measurements are two
/// lanes, and a disagreement between them fails here.
#[test]
fn every_special_day_has_its_notice_and_the_notice_window_is_the_measured_one() {
    for special in SPECIAL_SESSIONS {
        let notice = session_circular(special.day).expect("a notice for every special day");
        assert_eq!(notice.kind, special.kind, "day {}", special.day);
        assert_eq!(
            notice.notified,
            Some(special.session),
            "day {}",
            special.day
        );
        assert_eq!(
            tradable_day(special.day),
            Err(NotTradable::of_special(special.kind)),
            "day {}",
            special.day
        );
    }
    let unrecorded = notice_days_not_measured(&SESSION_CIRCULARS);
    assert_eq!(unrecorded, [20_765], "only the untimed 2026 Muhurat");
    let days = SESSION_CIRCULARS
        .iter()
        .filter(|notice| notice_in(&SESSION_CIRCULARS, notice.day) == Some(**notice))
        .count();
    assert_eq!(days, SPECIAL_SESSIONS.len() + unrecorded.len());
    assert_eq!(session_circular(at(2024, 1, 19)), None);

    // A LATER NOTICE FOR A DAY IS THE TABLE'S OWN SHAPE (D-5159), NOT A
    // DEFECT. This test used to require one row a day, strictly ascending,
    // and every unmeasured day untimed, so appending NSE's timing circular for
    // the 2026-11-08 Muhurat (the invented `LATER_NOTICE` here) failed it
    // while D-5159 says that row is exactly how the table grows.
    assert_eq!(notice_days_not_measured(&EXTENDED_NOTICES), [20_765]);
}

/// The shape every notice table holds (D-5152, D-5159), checked on the table
/// of record and on it with a later notice appended: sorted by day, a repeated
/// day being a later notice that supersedes the earlier one without editing
/// it; every row cites its number, date and address; every noticed day is a
/// session; the newest notice of a measured special day states its measured
/// kind and window. Returns the days whose newest notice has no measured
/// session: each either untimed and saying UNVERIFIED, or timed by a later
/// notice the measurements cannot confirm until the session has happened.
fn notice_days_not_measured(rows: &[SessionCircular]) -> Vec<i64> {
    let mut previous = i64::MIN;
    let mut unmeasured = Vec::new();
    for notice in rows {
        assert!(
            notice.day >= previous,
            "sorted by day; a repeated day is a later notice (D-5159)"
        );
        previous = notice.day;
        assert!(
            notice.source.contains(" https://") && notice.source.contains(" of "),
            "{}",
            notice.source
        );
        assert!(
            matches!(
                exchange_kind_of(notice.day),
                DayKind::Open(_) | DayKind::OpenLengthUnmeasured
            ),
            "a noticed day is a session, day {}",
            notice.day
        );
        if notice_in(rows, notice.day) != Some(*notice) {
            continue;
        }
        match special_session(notice.day) {
            Some(special) => {
                assert_eq!(notice.kind, special.kind, "day {}", notice.day);
                assert_eq!(notice.notified, Some(special.session), "day {}", notice.day);
            }
            None => {
                if notice.notified.is_none() {
                    assert!(notice.source.contains("UNVERIFIED"), "{}", notice.source);
                }
                unmeasured.push(notice.day);
            }
        }
    }
    unmeasured
}

/// **EVERY SHAPE THE TABLES COULD ONE DAY HOLD HAS A NAMED ANSWER.** The
/// unknown cases cannot be reached through today's tables, so they are
/// driven through `classify` directly.
#[test]
fn classify_names_every_shape_and_refuses_the_unknown_ones() {
    let regular = DayFacts {
        kind: DayKind::Open(Session::full()),
        weekend: false,
        special: None,
        halt: false,
    };
    assert_eq!(classify(regular), Ok(()));
    let with = |change: fn(&mut DayFacts)| {
        let mut facts = regular;
        change(&mut facts);
        classify(facts)
    };
    assert_eq!(
        with(|f| f.kind = DayKind::Unmeasured),
        Err(NotTradable::Unmeasured)
    );
    assert_eq!(
        with(|f| f.kind = DayKind::Closed),
        Err(NotTradable::Holiday)
    );
    assert_eq!(
        with(|f| {
            f.kind = DayKind::Closed;
            f.weekend = true;
        }),
        Err(NotTradable::Weekend)
    );
    // A closed day is closed whatever a row claims about it.
    assert_eq!(
        with(|f| {
            f.kind = DayKind::Closed;
            f.special = Some(SpecialKind::Muhurat);
        }),
        Err(NotTradable::Holiday)
    );
    assert_eq!(
        with(|f| f.kind = DayKind::OpenLengthUnmeasured),
        Err(NotTradable::OpenLengthUnmeasured)
    );
    assert_eq!(
        with(|f| f.kind = DayKind::Open(MUHURAT_1345)),
        Err(NotTradable::Unknown)
    );
    assert_eq!(with(|f| f.weekend = true), Err(NotTradable::Unknown));
    assert_eq!(with(|f| f.halt = true), Err(NotTradable::MarketHalt));
    for kind in [
        SpecialKind::Muhurat,
        SpecialKind::DisasterRecoveryDrill,
        SpecialKind::SystemsOutage,
        SpecialKind::BudgetWeekend,
        SpecialKind::WeekendFullSession,
    ] {
        let mut facts = regular;
        facts.special = Some(kind);
        facts.halt = true;
        assert_eq!(classify(facts), Err(NotTradable::of_special(kind)));
        assert!(NotTradable::of_special(kind).is_session());
        assert!(!NotTradable::of_special(kind).is_unexplained());
    }
    assert!(NotTradable::Unknown.is_unexplained());
    assert!(NotTradable::OpenLengthUnmeasured.is_unexplained());
    assert!(!NotTradable::Holiday.is_session() && !NotTradable::Weekend.is_session());
}

/// **THE REASON CODES ARE APPEND-ONLY AND NEVER RENUMBERED.** Each code is
/// written into the calendar digest and into any count a caller keeps, so
/// the table below is the ledger: a new reason is a new code at the end.
#[test]
fn the_reason_codes_are_pinned_and_every_reason_has_its_own_name_and_label() {
    use NotTradable as N;
    let pinned: [(NotTradable, u8); NOT_TRADABLE_REASONS] = [
        (N::Unmeasured, 0),
        (N::Weekend, 1),
        (N::Holiday, 2),
        (N::Muhurat, 3),
        (N::DisasterRecoveryDrill, 4),
        (N::SystemsOutage, 5),
        (N::BudgetWeekend, 6),
        (N::WeekendFullSession, 7),
        (N::OpenLengthUnmeasured, 8),
        (N::MarketHalt, 9),
        (N::Unknown, 10),
    ];
    for (reason, code) in pinned {
        assert_eq!(reason.code(), code, "{reason:?}");
        assert_eq!(NotTradable::from_code(code), Some(reason));
        assert_eq!(reason.index(), usize::from(code));
    }
    assert_eq!(NotTradable::from_code(11), None);
    assert_eq!(NotTradable::from_code(TRADABLE_CODE), None);
    let mut names = std::collections::BTreeSet::new();
    let mut labels = std::collections::BTreeSet::new();
    for (slot, reason) in NotTradable::ALL.into_iter().enumerate() {
        assert_eq!(reason.index(), slot);
        assert_eq!(format!("{reason:?}"), reason.name());
        assert!(names.insert(reason.name()));
        assert!(labels.insert(reason.label()));
        assert!(reason.label().contains(' '), "{}", reason.label());
    }
    assert_eq!(names.len(), NOT_TRADABLE_REASONS);
}

/// The count each year 2018..=2026 must give, by reason: the sessions the
/// tables and the circulars name, nothing unexplained, and the closed days
/// adding up to the rest of the year.
#[test]
fn the_yearly_counts_name_every_excluded_session_and_add_up() {
    use NotTradable::{
        BudgetWeekend, DisasterRecoveryDrill, MarketHalt, Muhurat, SystemsOutage,
        WeekendFullSession,
    };
    let expected: [(u16, &[(NotTradable, u32)]); 9] = [
        (2018, &[(Muhurat, 1)]),
        (2019, &[(Muhurat, 1)]),
        (2020, &[(BudgetWeekend, 1), (Muhurat, 1), (MarketHalt, 2)]),
        (2021, &[(SystemsOutage, 1), (Muhurat, 1)]),
        (2022, &[(Muhurat, 1)]),
        (2023, &[(Muhurat, 1)]),
        (
            2024,
            &[
                (WeekendFullSession, 1),
                (DisasterRecoveryDrill, 2),
                (Muhurat, 1),
            ],
        ),
        (2025, &[(BudgetWeekend, 1), (Muhurat, 1)]),
        (2026, &[(BudgetWeekend, 1), (Muhurat, 1)]),
    ];
    let mut all_excluded = 0_u32;
    for (year, sessions) in expected {
        let counted = year_exclusions(year).expect("inside the record");
        assert_eq!(counted.year, year);
        let counts = counted.counts;
        let leap = year % 4 == 0;
        assert_eq!(counts.days(), if leap { 366 } else { 365 }, "{year}");
        let total: u32 = counts.by_reason.iter().sum::<u32>() + counts.tradable;
        assert_eq!(i64::from(total), counts.days(), "{year}");
        assert_eq!(counts.unexplained(), 0, "{year}");
        assert_eq!(counts.count(NotTradable::Unmeasured), 0, "{year}");
        for reason in NotTradable::ALL.into_iter().filter(|r| r.is_session()) {
            let want: u32 = sessions
                .iter()
                .filter(|(r, _)| *r == reason)
                .map(|(_, n)| *n)
                .sum();
            assert_eq!(counts.count(reason), want, "{year} {reason:?}");
        }
        let want_excluded: u32 = sessions.iter().map(|(_, n)| n).sum();
        assert_eq!(counts.excluded_sessions(), want_excluded, "{year}");
        all_excluded += want_excluded;
        // The weekend days of the year, counted by date arithmetic alone:
        // every one is a closed weekend or a weekend session.
        let weekends = (counts.first_day..=counts.last_day)
            .filter(|d| matches!((d + 3).rem_euclid(7), 5 | 6))
            .count();
        let weekend_sessions = (counts.first_day..=counts.last_day)
            .filter(|d| matches!((d + 3).rem_euclid(7), 5 | 6))
            .filter(|d| tradable_day(*d).is_err_and(NotTradable::is_session))
            .count();
        assert_eq!(
            usize::try_from(counts.count(NotTradable::Weekend)).expect("small") + weekend_sessions,
            weekends,
            "{year}"
        );
    }
    assert_eq!(all_excluded, 18, "eighteen excluded sessions 2018-2026");
    // 2018, the year the sourced holiday table covers whole: its rows of the
    // year, less the Muhurat it also lists, are the holidays.
    let counted = year_exclusions(2018).expect("2018").counts;
    let rows = SOURCED_HOLIDAYS
        .iter()
        .filter(|d| (counted.first_day..=counted.last_day).contains(d))
        .count();
    assert_eq!(
        rows, 16,
        "NSE/CMTR/36475 lists sixteen weekday holidays in 2018"
    );
    assert_eq!(
        usize::try_from(counted.count(NotTradable::Holiday)).expect("small"),
        rows - 1
    );
}

/// A year not wholly inside the record is refused by name, never counted
/// short, and the refusal says its circular is owed.
#[test]
fn a_year_outside_the_record_is_refused_by_name() {
    for year in [2017_u16, 2027, 2015, 1969] {
        let refusal = year_exclusions(year).expect_err("outside the record");
        let text = refusal.to_string();
        match refusal {
            YearRefusal::OutsideRecord { year: asked } => {
                assert_eq!(asked, year);
                assert!(text.contains("2018-01-01 to 2026-12-31"), "{text}");
                assert!(text.contains("no NSE holiday circular"), "{text}");
            }
            YearRefusal::NotACivilYear(asked) => {
                assert_eq!(asked, year);
                assert!(text.contains("not a year"), "{text}");
            }
        }
    }
    assert_eq!(
        year_exclusions(2017),
        Err(YearRefusal::OutsideRecord { year: 2017 })
    );
    assert!(matches!(
        year_exclusions(0),
        Err(YearRefusal::NotACivilYear(0))
    ));
}

// ── The sourced span, computed from the circulars (operator item 3) ──

/// An invented whole-year row for `year`.
const fn whole(year: u16) -> HolidayCircular {
    HolidayCircular {
        year,
        number: "TEST/INVENTED",
        issued: "1999-01-01",
        url: "https://example.invalid/invented",
        whole_year: true,
    }
}

/// **THE SOURCED SPAN IS COMPUTED, AND THE TABLE MOVES IT**: an appended
/// synthetic year moves the last day; a gap year stops the span; a partial
/// row does not make a year recorded; a year wholly inside the measured
/// window needs no circular; no whole-year row is no span at all.
#[test]
fn the_sourced_span_is_computed_from_the_circulars_and_a_gap_stops_it() {
    let (m_first, m_last) = (FIRST_DAY, LAST_DAY);
    // Today's table gives today's span.
    assert_eq!(
        sourced_span(&HOLIDAY_CIRCULARS, m_first, m_last),
        Some((at(2018, 1, 1), at(2026, 12, 31)))
    );
    let today = [whole(2018), whole(2019), whole(2026)];
    assert_eq!(
        sourced_span(&today, m_first, m_last),
        Some((at(2018, 1, 1), at(2026, 12, 31)))
    );
    // Appending a synthetic 2027 row moves the bound by itself.
    let appended = [whole(2018), whole(2019), whole(2026), whole(2027)];
    assert_eq!(
        sourced_span(&appended, m_first, m_last),
        Some((at(2018, 1, 1), at(2027, 12, 31)))
    );
    // A gap year stops the span: 2019 neither recorded nor wholly measured.
    let gap = [whole(2018), whole(2026)];
    assert_eq!(
        sourced_span(&gap, m_first, m_last),
        Some((at(2018, 1, 1), at(2018, 12, 31)))
    );
    // A partial row (one added day) does not make its year recorded.
    let mut partial = whole(2019);
    partial.whole_year = false;
    assert_eq!(
        sourced_span(&[whole(2018), partial, whole(2026)], m_first, m_last),
        Some((at(2018, 1, 1), at(2018, 12, 31)))
    );
    // A missing 2026 with a window that ends 2026-09-04 stops at 2025.
    assert_eq!(
        sourced_span(&[whole(2018), whole(2019)], m_first, m_last),
        Some((at(2018, 1, 1), at(2025, 12, 31)))
    );
    // ...and a window measured through 2026-12-31 needs no 2026 circular.
    assert_eq!(
        sourced_span(&[whole(2018), whole(2019)], m_first, at(2026, 12, 31)),
        Some((at(2018, 1, 1), at(2026, 12, 31)))
    );
    // Rows in any order; the earliest whole year starts the span.
    assert_eq!(
        sourced_span(&[whole(2026), whole(2019), whole(2018)], m_first, m_last),
        Some((at(2018, 1, 1), at(2026, 12, 31)))
    );
    assert_eq!(sourced_span(&[], m_first, m_last), None);
    assert_eq!(sourced_span(&[partial], m_first, m_last), None);
    // The day after the computed last day is Unmeasured.
    assert_eq!(sourced_kind_of(SOURCED_LAST_DAY + 1), DayKind::Unmeasured);
    assert_eq!(
        tradable_day(SOURCED_LAST_DAY + 1),
        Err(NotTradable::Unmeasured)
    );
    assert_eq!(SOURCED_LAST_YEAR, 2026);
    // Every row the table holds is a real circular: an NSE number, an issue
    // date before the year it lists, and an nsearchives address.
    for row in HOLIDAY_CIRCULARS {
        assert!(row.number.starts_with("NSE/"), "{row:?}");
        assert!(
            row.url
                .starts_with("https://nsearchives.nseindia.com/content/circulars/"),
            "{row:?}"
        );
        let issued_year: u16 = row
            .issued
            .get(..4)
            .and_then(|y| y.parse().ok())
            .expect("YYYY-MM-DD");
        assert!(issued_year <= row.year, "{row:?}");
    }
}

/// The civil arithmetic the computed spans stand on agrees with the session
/// calendar's own, on every day of a wide span.
#[test]
fn civil_days_and_civil_year_agree_with_the_session_calendar() {
    for days in 0_u32..=40_000 {
        let day = crate::session::Day::from_days(days).expect("a day");
        assert_eq!(
            civil_days(
                i64::from(day.year()),
                i64::from(day.month()),
                i64::from(day.day())
            ),
            i64::from(days)
        );
        assert_eq!(civil_year(i64::from(days)), i64::from(day.year()));
    }
}

// ── The measured versions and the calendar digest (operator item 4) ──

/// **THE MEASURED WINDOW IS THE NEWEST VERSION'S OWN EXTENT, AND AN OLDER
/// VERSION KEEPS ITS OWN.** Version 1 ended 2026-08-21; version 2 (D-3346)
/// ends 2026-09-04. A run recorded under version 1 re-verifies against
/// version 1's digest; the extension re-keyed nothing.
#[test]
fn the_measured_versions_keep_their_extents_and_their_digests() {
    assert!(versions_ascend(&MEASURED_VERSIONS));
    assert_eq!(NEWEST_MEASURED, newest_of(&MEASURED_VERSIONS));
    assert_eq!(NEWEST_MEASURED.version, 2);
    assert_eq!((FIRST_DAY, LAST_DAY), (at(2019, 12, 2), at(2026, 9, 4)));
    assert_eq!(
        i64::try_from(DAYS).expect("small"),
        LAST_DAY - FIRST_DAY + 1
    );
    assert!(padding_is_clear(&TRADED, DAYS));

    let one = CalendarVersion::of(1).expect("version 1");
    let two = CalendarVersion::of(2).expect("version 2");
    assert_eq!(two, CalendarVersion::newest());
    assert_eq!(CalendarVersion::of(0), None);
    assert_eq!(CalendarVersion::of(3), None);
    assert_eq!((one.number(), two.number()), (1, 2));
    assert_eq!(one.measured(), (FIRST_DAY, at(2026, 8, 21)));
    assert_eq!(two.measured(), (FIRST_DAY, LAST_DAY));
    assert_eq!(one.sourced(), (at(2018, 1, 1), at(2026, 12, 31)));
    assert_eq!(two.record(), (SOURCED_FIRST_DAY, SOURCED_LAST_DAY));

    // 2026-08-24 (Mon): measured by version 2 only, answered by both.
    let monday = at(2026, 8, 24);
    assert_eq!(one.kind_of(monday), DayKind::Unmeasured);
    assert_eq!(two.kind_of(monday), DayKind::Open(Session::full()));
    assert_eq!(one.sourced_kind_of(monday), DayKind::Open(Session::full()));
    assert_eq!(one.tradable_day(monday), Ok(()));
    assert_eq!(two.tradable_day(monday), Ok(()));

    // The newest version IS the free functions, on every day of the record
    // and past it.
    for day in SOURCED_FIRST_DAY - 5..=SOURCED_LAST_DAY + 5 {
        assert_eq!(two.kind_of(day), kind_of(day), "{day}");
        assert_eq!(two.sourced_kind_of(day), sourced_kind_of(day), "{day}");
        assert_eq!(two.tradable_day(day), tradable_day(day), "{day}");
    }

    // Two versions, two digests; each stable; the free digest is the newest.
    let (d1, d2) = (one.digest(), two.digest());
    assert_ne!(d1, d2);
    assert_eq!(d1, one.digest());
    assert_eq!(calendar_digest(), d2);
    assert_eq!(calendar_digest(), calendar_digest());
    let bytes = two.canonical_bytes();
    assert!(bytes.starts_with(CALENDAR_DIGEST_DOMAIN));
    let mut hasher = brutex_core::blake3::Hasher::new();
    hasher.update(&bytes);
    assert_eq!(hasher.finalize(), d2);
    // One fixed-length row per day of the record, at least: the bytes grow
    // with the record, and a day's verdict is in them.
    let days = usize::try_from(SOURCED_LAST_DAY - SOURCED_FIRST_DAY + 1).expect("small");
    assert!(bytes.len() > days * 10, "{}", bytes.len());
}

/// The version ledger's own checks refuse what a bad append would look like.
#[test]
fn a_bad_version_table_or_padding_is_refused() {
    let rows = |version, days, holiday_circulars, session_circulars| MeasuredVersion {
        version,
        days,
        holiday_circulars,
        session_circulars,
        source: "invented",
    };
    let row = |version, days| rows(version, days, 5, 16);
    assert!(versions_ascend(&[]));
    assert!(versions_ascend(&[row(1, 10), row(2, 11)]));
    assert!(!versions_ascend(&[row(2, 10)]), "must start at 1");
    assert!(!versions_ascend(&[row(1, 10), row(2, 10)]), "must grow");
    assert!(!versions_ascend(&[row(1, 10), row(3, 11)]), "no gap");
    // A version may add only a circular or only a notice (D-5159)...
    assert!(versions_ascend(&[rows(1, 10, 5, 16), rows(2, 10, 6, 16)]));
    assert!(versions_ascend(&[rows(1, 10, 5, 16), rows(2, 10, 5, 17)]));
    // ...but never read fewer rows or days than the version before it.
    assert!(!versions_ascend(&[rows(1, 10, 5, 16), rows(2, 11, 4, 16)]));
    assert!(!versions_ascend(&[rows(1, 10, 5, 16), rows(2, 11, 5, 15)]));
    assert!(!versions_ascend(&[rows(1, 10, 5, 16), rows(2, 9, 6, 17)]));
    assert_eq!(newest_of(&[]), NO_VERSION);
    assert_eq!(newest_of(&[row(1, 10), row(2, 11)]).days, 11);
    // Every row belongs to a version, and no version reads past a table.
    assert!(versions_read_the_tables(&MEASURED_VERSIONS, 5, 16));
    assert!(
        !versions_read_the_tables(&MEASURED_VERSIONS, 6, 16),
        "a circular appended without a version"
    );
    assert!(
        !versions_read_the_tables(&MEASURED_VERSIONS, 5, 17),
        "a notice appended without a version"
    );
    assert!(!versions_read_the_tables(&[rows(1, 10, 6, 16)], 5, 16));
    assert!(!versions_read_the_tables(&[rows(1, 10, 5, 17)], 5, 16));
    assert_eq!(prefix(&[1, 2, 3], 2), &[1, 2]);
    assert_eq!(prefix(&[1, 2, 3], 9), &[1, 2, 3]);
    assert!(padding_is_clear(&[], 0));
    assert!(!padding_is_clear(&[], 1));
    assert!(padding_is_clear(&[0x7F], 7));
    assert!(!padding_is_clear(&[0xFF], 7));
    assert!(padding_is_clear(&[0xFF], 8));
    assert!(padding_is_clear(&[0xFF, 0x01], 9));
    assert!(!padding_is_clear(&[0xFF, 0x03], 9));
}

// ── Appended rows are new versions (D-5159) ──

/// An invented later notice for 2026-11-08, the untimed Muhurat: the shape
/// NSE's timing circular will take, with a window and source that cannot be
/// mistaken for the real one.
const LATER_NOTICE: SessionCircular = SessionCircular {
    day: 20_765,
    kind: SpecialKind::Muhurat,
    notified: Some(MUHURAT_1815),
    source: "TEST/INVENTED of 1 Jan 1999, https://example.invalid/invented : an invented \
             later notice, never a real one",
};

/// The table of record with [`LATER_NOTICE`] appended.
const fn notices_with_a_later_one() -> [SessionCircular; 17] {
    let mut out = [LATER_NOTICE; 17];
    let mut i = 0;
    while i < SESSION_CIRCULARS.len() {
        out[i] = SESSION_CIRCULARS[i];
        i += 1;
    }
    out
}

/// The table of record with an invented 2027 whole-year circular appended.
const fn circulars_with_2027() -> [HolidayCircular; 6] {
    let mut out = [whole(2027); 6];
    let mut i = 0;
    while i < HOLIDAY_CIRCULARS.len() {
        out[i] = HOLIDAY_CIRCULARS[i];
        i += 1;
    }
    out
}

static EXTENDED_NOTICES: [SessionCircular; 17] = notices_with_a_later_one();
static EXTENDED_CIRCULARS: [HolidayCircular; 6] = circulars_with_2027();

/// **AN APPENDED ROW IS A NEW VERSION AND RE-KEYS NOTHING BEFORE IT**
/// (operator, 16 Oct 2026, item 4; D-5159). Before D-5159 every version read
/// the whole of both circular tables, so appending NSE's 2027 holiday
/// circular or the 2026 Muhurat's timing notice would have changed version
/// 2's span, its answers and its digest: every run recorded under it would
/// have failed re-verification. Each version now reads its own prefix.
#[test]
fn an_appended_row_is_a_new_version_and_re_keys_none_before_it() {
    let two = CalendarVersion::of(2).expect("version 2");
    let muhurat = at(2026, 11, 8);

    // Version 2 over the tables with both rows appended is version 2.
    let two_later =
        CalendarVersion::on_tables(MEASURED_VERSIONS[1], &EXTENDED_CIRCULARS, &EXTENDED_NOTICES);
    assert_eq!(two_later, two);
    assert_eq!(two_later.digest(), two.digest());
    assert_eq!(two_later.sourced(), (at(2018, 1, 1), at(2026, 12, 31)));
    assert_eq!(
        two_later.session_circular(muhurat).map(|n| n.notified),
        Some(None),
        "version 2 keeps the untimed notice"
    );
    for day in [muhurat, at(2027, 1, 4), at(2026, 12, 31), at(2024, 1, 20)] {
        assert_eq!(two_later.tradable_day(day), two.tradable_day(day), "{day}");
    }

    // A version 3 that reads them is a new calendar with its own digest.
    let three_row = MeasuredVersion {
        version: 3,
        days: MEASURED_VERSIONS[1].days,
        holiday_circulars: 6,
        session_circulars: 17,
        source: "invented",
    };
    assert!(versions_ascend(&[
        MEASURED_VERSIONS[0],
        MEASURED_VERSIONS[1],
        three_row
    ]));
    let three = CalendarVersion::on_tables(three_row, &EXTENDED_CIRCULARS, &EXTENDED_NOTICES);
    assert_eq!(three.sourced(), (at(2018, 1, 1), at(2027, 12, 31)));
    assert_eq!(
        three.session_circular(muhurat),
        Some(LATER_NOTICE),
        "the later notice for a day supersedes the earlier one"
    );
    assert_eq!(three.tradable_day(muhurat), Err(NotTradable::Muhurat));
    assert_ne!(three.digest(), two.digest());
    assert_eq!(
        two.tradable_day(at(2027, 1, 4)),
        Err(NotTradable::Unmeasured)
    );

    // The lookup itself: the last row naming a day wins; none is none.
    assert_eq!(notice_in(&EXTENDED_NOTICES, muhurat), Some(LATER_NOTICE));
    assert_eq!(
        notice_in(&SESSION_CIRCULARS, muhurat).map(|n| n.notified),
        Some(None)
    );
    assert_eq!(notice_in(&EXTENDED_NOTICES, at(2024, 1, 19)), None);
    assert_eq!(notice_in(&[], muhurat), None);
}

/// **VERSION 2'S DIGEST IS THE ONE RECORDED.** First printed by `cli
/// calendar-days check` over the shadow store on 2026-10-18, before D-5159
/// moved the circular tables into the version: the move changed no byte. A
/// failure here means version 2 now answers some day differently, which
/// re-keys every run recorded under it: that is a new version, not an edit.
#[test]
fn version_two_keeps_its_recorded_digest() {
    let hex = CalendarVersion::of(2)
        .expect("version 2")
        .digest()
        .iter()
        .fold(String::new(), |mut out, byte| {
            let _ = write!(out, "{byte:02x}");
            out
        });
    assert_eq!(
        hex,
        "de177094a06002658c4ce9d9f79e423c635f63d2762756bfdbd215f27b333bf0"
    );
}

/// **VERSION 1'S DIGEST IS PINNED TOO** (verify-CAL-1 round 1). Every version
/// reads `SPECIAL_SESSIONS`, `HALTS`, `SOURCED_HOLIDAYS`, `SOURCED_SPECIAL`,
/// `IRREGULAR` and `LENGTH_UNMEASURED` whole, not a prefix of each
/// (docs/06-limits.md L-CAL-11), so a row appended to one of them can change
/// what an OLDER version answers. Pinning only version 2 left version 1's
/// answers free to move unseen; with both pinned, such an append fails the
/// build for every version it touches. Recorded 2026-10-21 by CAL-1 repair
/// round 1 from this tree (first printed by this test, at commit 9b7050735);
/// no recorded run names a `CalendarVersion` yet (L-CAL-7), so pinning it
/// re-keys nothing. It differs from version 2's, as two extents must.
#[test]
fn version_one_keeps_its_recorded_digest() {
    let hex = CalendarVersion::of(1)
        .expect("version 1")
        .digest()
        .iter()
        .fold(String::new(), |mut out, byte| {
            let _ = write!(out, "{byte:02x}");
            out
        });
    assert_eq!(
        hex,
        "3b695fd9cd936f511f7c1ac78de26e7f7e401e75bcda628c06f588124e0d03a4"
    );
}

/// A whole-year table with a gap: 2019 neither recorded nor wholly measured.
static GAP_CIRCULARS: [HolidayCircular; 2] = [whole(2018), whole(2026)];

/// **NO VERSION'S RECORD HAS A HOLE** (D-5155; verify-CAL-1 round 1).
/// `record()` widens the sourced span by the measured window, so a span that
/// stops at a gap year a year before the window starts would make a record
/// holding days neither answers, and a bucket over them would count them as
/// `Unmeasured` inside the record instead of refusing. The build refuses such
/// a version (a const assertion over `MEASURED_VERSIONS`); this drives the
/// check on an invented table and on both edges of "touching".
#[test]
fn a_record_with_a_hole_between_its_spans_is_refused() {
    for row in MEASURED_VERSIONS {
        assert!(CalendarVersion::on(row).record_is_unbroken(), "{row:?}");
    }
    assert!(records_are_unbroken(&MEASURED_VERSIONS));

    let row = MeasuredVersion {
        holiday_circulars: 2,
        ..MEASURED_VERSIONS[1]
    };
    let holed = CalendarVersion::on_tables(row, &GAP_CIRCULARS, &SESSION_CIRCULARS);
    assert_eq!(holed.sourced(), (at(2018, 1, 1), at(2018, 12, 31)));
    assert_eq!(holed.record(), (at(2018, 1, 1), LAST_DAY));
    assert!(!holed.record_is_unbroken());
    // The day the assertion keeps out of every real record: inside
    // `record()`, answered by neither span.
    assert_eq!(holed.sourced_kind_of(at(2019, 6, 3)), DayKind::Unmeasured);
    assert_eq!(
        holed.tradable_day(at(2019, 6, 3)),
        Err(NotTradable::Unmeasured)
    );

    // Touching is unbroken; one day apart is a hole, on either side.
    let two = CalendarVersion::of(2).expect("version 2");
    let with = |sourced: (i64, i64)| CalendarVersion { sourced, ..two };
    assert!(with((at(2018, 1, 1), FIRST_DAY - 1)).record_is_unbroken());
    assert!(!with((at(2018, 1, 1), FIRST_DAY - 2)).record_is_unbroken());
    assert!(with((LAST_DAY + 1, at(2026, 12, 31))).record_is_unbroken());
    assert!(!with((LAST_DAY + 2, at(2026, 12, 31))).record_is_unbroken());
    assert!(with((FIRST_DAY, LAST_DAY)).record_is_unbroken());
}

/// **A RECORDED YEAR BRINGS ITS HOLIDAYS** (D-5159). `sourced_kind` calls
/// every weekday open unless `SOURCED_HOLIDAYS` lists it, so a whole-year
/// circular appended without its days would widen the span and call each of
/// that year's holidays a session. The build refuses that table.
#[test]
fn a_recorded_year_without_its_holidays_is_refused() {
    assert!(recorded_years_list_their_holidays(
        &HOLIDAY_CIRCULARS,
        &SOURCED_HOLIDAYS,
        FIRST_DAY,
        LAST_DAY
    ));
    // An invented 2027 circular with no 2027 day is refused...
    assert!(!recorded_years_list_their_holidays(
        &EXTENDED_CIRCULARS,
        &SOURCED_HOLIDAYS,
        FIRST_DAY,
        LAST_DAY
    ));
    // ...and passes once one 2027 holiday (invented: 2027-01-26) is listed.
    let mut with_2027 = SOURCED_HOLIDAYS.to_vec();
    with_2027.push(at(2027, 1, 26));
    assert!(recorded_years_list_their_holidays(
        &EXTENDED_CIRCULARS,
        &with_2027,
        FIRST_DAY,
        LAST_DAY
    ));
    // A year wholly inside the measured window needs no listed day; a
    // partial row never asks for one; a day INSIDE the window does not count
    // for a year that runs past it.
    assert!(recorded_years_list_their_holidays(
        &[whole(2022)],
        &[],
        FIRST_DAY,
        LAST_DAY
    ));
    let mut partial = whole(2027);
    partial.whole_year = false;
    assert!(recorded_years_list_their_holidays(
        &[partial],
        &[],
        FIRST_DAY,
        LAST_DAY
    ));
    assert!(!recorded_years_list_their_holidays(
        &[whole(2026)],
        &[at(2026, 1, 26)],
        FIRST_DAY,
        LAST_DAY
    ));
}

// ── The running sums (operator item 7) and the per-year record ──

/// **ANY BUCKET IS TWO LOOKUPS, AND EQUALS THE DAY-BY-DAY COUNT.** Every
/// month of the record and a spread of odd buckets are compared with a
/// count by `tradable_day` itself; inverted and out-of-record buckets are
/// refused by name.
#[test]
fn the_running_sums_answer_every_bucket_exactly() {
    let sums = RunningSums::newest();
    assert_eq!(sums.version(), CalendarVersion::newest());
    let brute = |first: i64, last: i64| {
        let mut by_reason = [0_u32; NOT_TRADABLE_REASONS];
        let mut tradable = 0_u32;
        for day in first..=last {
            match tradable_day(day) {
                Ok(()) => tradable += 1,
                Err(reason) => by_reason[reason.index()] += 1,
            }
        }
        (tradable, by_reason)
    };
    let mut buckets = Vec::new();
    let mut first = SOURCED_FIRST_DAY;
    while first <= SOURCED_LAST_DAY {
        buckets.push((first, (first + 30).min(SOURCED_LAST_DAY)));
        first += 31;
    }
    buckets.extend([
        (SOURCED_FIRST_DAY, SOURCED_FIRST_DAY),
        (SOURCED_LAST_DAY, SOURCED_LAST_DAY),
        (SOURCED_FIRST_DAY, SOURCED_LAST_DAY),
        (at(2020, 3, 1), at(2020, 3, 31)),
        (at(2024, 1, 15), at(2024, 5, 20)),
    ]);
    for (first, last) in buckets {
        let counts = sums.between(first, last).expect("inside");
        assert_eq!((counts.first_day, counts.last_day), (first, last));
        assert_eq!((counts.tradable, counts.by_reason), brute(first, last));
        assert_eq!(
            excluded_between(first, last).expect("inside"),
            counts,
            "the shared sums"
        );
    }
    let whole = excluded_between(SOURCED_FIRST_DAY, SOURCED_LAST_DAY).expect("whole");
    assert_eq!(whole.count(NotTradable::Muhurat), 9);
    assert_eq!(whole.excluded_sessions(), 18);
    assert_eq!(
        sums.between(at(2020, 1, 2), at(2020, 1, 1)),
        Err(SpanRefusal::Inverted {
            first: at(2020, 1, 2),
            last: at(2020, 1, 1)
        })
    );
    for (first, last) in [
        (SOURCED_FIRST_DAY - 1, SOURCED_FIRST_DAY),
        (SOURCED_LAST_DAY, SOURCED_LAST_DAY + 1),
        (i64::MIN, i64::MAX),
    ] {
        let refusal = sums.between(first, last).expect_err("outside");
        assert!(
            matches!(refusal, SpanRefusal::OutsideRecord { record_first, record_last, .. }
                if (record_first, record_last) == (SOURCED_FIRST_DAY, SOURCED_LAST_DAY)),
            "{refusal:?}"
        );
        assert!(
            refusal.to_string().contains("not wholly inside"),
            "{refusal}"
        );
    }
    assert!(
        SpanRefusal::Inverted { first: 2, last: 1 }
            .to_string()
            .contains("end before they start")
    );
    // A version's sums are its own: version 1's are built over its record.
    let one = RunningSums::build(CalendarVersion::of(1).expect("v1"));
    assert_eq!(
        one.between(at(2026, 8, 24), at(2026, 8, 28))
            .expect("inside")
            .tradable,
        5
    );
}

/// **EACH YEAR SAYS WHETHER ITS CIRCULARS ARE RECORDED** (operator item 1):
/// a whole-year holiday circular, a year wholly measured, or owed; and its
/// special-session notices, with the untimed ones counted.
#[test]
fn each_year_says_whether_its_circulars_are_recorded() {
    let record = year_record(2018);
    assert_eq!(record.holidays, YearHolidays::Recorded("NSE/CMTR/36475"));
    assert_eq!((record.notices, record.untimed_notices), (1, 0));
    assert!(record.answered());
    assert_eq!(
        year_record(2019).holidays,
        YearHolidays::Recorded("NSE/CMTR/39612")
    );
    for year in 2020..=2025 {
        assert_eq!(year_record(year).holidays, YearHolidays::Measured, "{year}");
        assert!(year_record(year).answered());
    }
    let record = year_record(2026);
    assert_eq!(record.holidays, YearHolidays::Recorded("NSE/CMTR/71775"));
    assert_eq!((record.notices, record.untimed_notices), (2, 1));
    let record = year_record(2024);
    assert_eq!((record.notices, record.untimed_notices), (4, 0));
    for year in [2015, 2016, 2017, 2027] {
        let record = year_record(year);
        assert_eq!(record.holidays, YearHolidays::Owed, "{year}");
        assert!(!record.answered());
        assert_eq!(record.notices, 0);
    }
}

/// **CONCURRENT READERS GET ONE ANSWER** (CAL-1 self-check, extreme case
/// "concurrent requests"). Eight threads race on the two once-per-process
/// builds (`RunningSums::newest`, `calendar_digest`) and read every day and
/// every year of the record; each must see the same shared sums (one build,
/// one address) and exactly what a private, unshared build computes on this
/// thread. A torn or doubled first use would show as a second address or a
/// different count.
#[test]
fn concurrent_readers_share_one_build_and_get_one_answer() {
    /// What one reader thread saw.
    struct Seen {
        sums: &'static RunningSums,
        digest: [u8; 32],
        answers: Vec<Result<(), NotTradable>>,
        counts: Vec<Option<DayCounts>>,
    }
    let version = CalendarVersion::newest();
    let reference_sums = RunningSums::build(version);
    let reference_digest = version.digest();
    let days: Vec<i64> = (SOURCED_FIRST_DAY - 1..=SOURCED_LAST_DAY + 1).collect();
    let reference_days: Vec<Result<(), NotTradable>> =
        days.iter().map(|day| version.tradable_day(*day)).collect();
    let years: Vec<u16> = (2017..=2027).collect();
    let reference_years: Vec<Option<DayCounts>> = years
        .iter()
        .map(|year| {
            let first = civil_days(i64::from(*year), 1, 1);
            let last = civil_days(i64::from(*year), 12, 31);
            reference_sums.between(first, last).ok()
        })
        .collect();
    assert_eq!(
        reference_years.iter().flatten().count(),
        9,
        "2018..=2026 are counted, 2017 and 2027 refused"
    );
    let seen: Vec<Seen> = std::thread::scope(|scope| {
        let workers: Vec<_> = (0..8)
            .map(|_| {
                scope.spawn(|| Seen {
                    sums: RunningSums::newest(),
                    digest: calendar_digest(),
                    answers: days.iter().map(|day| tradable_day(*day)).collect(),
                    counts: years
                        .iter()
                        .map(|year| year_exclusions(*year).ok().map(|counted| counted.counts))
                        .collect(),
                })
            })
            .collect();
        workers
            .into_iter()
            .map(|worker| worker.join().expect("a reader thread"))
            .collect()
    });
    assert_eq!(seen.len(), 8);
    let shared = RunningSums::newest();
    for one in &seen {
        assert!(
            std::ptr::eq(one.sums, shared),
            "one shared build, one address"
        );
        assert_eq!(one.digest, reference_digest);
        assert_eq!(one.answers, reference_days);
        assert_eq!(one.counts, reference_years);
    }
    assert_eq!(shared, &reference_sums);
}

// ── The cost, measured (operator rule 1) ──

/// **THE PER-DAY LOOKUP AND EVERY BUCKET ARE CONSTANT-COST, MEASURED.**
/// p50, p99 and max of `tradable_day` over every day of the record and one
/// past each edge (64 rounds, each sample the mean of 16 calls), of
/// `excluded_between` over buckets of every length (n = 3,287), of
/// `year_exclusions` (n = 270) and of `year_record` (n = 270), after the sums
/// are built once. The bounds
/// asserted are generous so a loaded host does not fail them; the figures
/// are printed for the ledger (`--nocapture`).
#[test]
fn tradable_day_and_every_bucket_are_bounded() {
    use std::hint::black_box;
    use std::time::Instant;
    let built = Instant::now();
    let sums = black_box(RunningSums::build(CalendarVersion::newest()));
    let build_ns = built.elapsed().as_nanos();
    let digested = Instant::now();
    let digest = black_box(CalendarVersion::newest().digest());
    let digest_ns = digested.elapsed().as_nanos();
    assert_eq!(digest, calendar_digest());
    eprintln!(
        "RunningSums::build (once per version): {build_ns} ns; digest (once): {digest_ns} ns"
    );

    let mut samples = Vec::new();
    for _ in 0..64 {
        for day in SOURCED_FIRST_DAY - 1..=SOURCED_LAST_DAY + 1 {
            let started = Instant::now();
            for _ in 0..16 {
                black_box(tradable_day(black_box(day))).ok();
            }
            samples.push(started.elapsed().as_nanos() / 16);
        }
    }
    let (p50, p99, max) = spread(&mut samples);
    eprintln!(
        "tradable_day: p50 {p50} ns, p99 {p99} ns, max {max} ns, n = {}",
        samples.len()
    );
    assert!(samples.len() >= 200 && p99 < 50_000, "p99 {p99} ns");

    let mut buckets = Vec::new();
    for length in 0..=SOURCED_LAST_DAY - SOURCED_FIRST_DAY {
        let first = SOURCED_FIRST_DAY;
        let started = Instant::now();
        black_box(sums.between(black_box(first), black_box(first + length))).ok();
        buckets.push(started.elapsed().as_nanos());
    }
    let (p50, p99, max) = spread(&mut buckets);
    eprintln!(
        "excluded_between (any length): p50 {p50} ns, p99 {p99} ns, max {max} ns, n = {}",
        buckets.len()
    );
    assert!(buckets.len() >= 200 && p99 < 50_000, "p99 {p99} ns");

    let mut years = Vec::new();
    for _ in 0..30 {
        for year in 2018..=2026 {
            let started = Instant::now();
            black_box(year_exclusions(black_box(year))).ok();
            years.push(started.elapsed().as_nanos());
        }
    }
    let (p50, p99, max) = spread(&mut years);
    eprintln!(
        "year_exclusions: p50 {p50} ns, p99 {p99} ns, max {max} ns, n = {}",
        years.len()
    );
    assert!(years.len() >= 200 && p99 < 100_000, "p99 {p99} ns");

    let mut records = Vec::new();
    for _ in 0..30 {
        for year in 2018..=2026 {
            let started = Instant::now();
            black_box(year_record(black_box(year)));
            records.push(started.elapsed().as_nanos());
        }
    }
    let (p50, p99, max) = spread(&mut records);
    eprintln!(
        "year_record: p50 {p50} ns, p99 {p99} ns, max {max} ns, n = {}",
        records.len()
    );
    assert!(records.len() >= 200 && p99 < 100_000, "p99 {p99} ns");
}

fn spread(samples: &mut [u128]) -> (u128, u128, u128) {
    samples.sort_unstable();
    let pick = |permille: usize| {
        samples
            .get(samples.len() * permille / 1_000)
            .copied()
            .unwrap_or(u128::MAX)
    };
    (
        pick(500),
        pick(990),
        samples.last().copied().unwrap_or(u128::MAX),
    )
}

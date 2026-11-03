//! BASE-COV-PULL: the const builders of the calendar's tables, which only the
//! compiler ever ran (Route B counts no compile-time run), and the facts a
//! day answers. Every table row they make here is invented.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "a test that cannot panic cannot fail"
)]

use super::*;

#[test]
fn the_table_builders_make_what_their_rows_say_at_run_time_too() {
    let one = one_window(555, 929);
    assert_eq!(one.count, 1);
    assert_eq!((one.windows[0].from, one.windows[0].to), (555, 929));
    assert_eq!((one.windows[1].from, one.windows[1].to), (0, 0));
    let first = Window { from: 1, to: 2 };
    let second = Window { from: 3, to: 4 };
    let two = two_windows(first, second);
    assert_eq!(two.count, 2);
    assert_eq!((two.windows[1].from, two.windows[1].to), (3, 4));
    let halt = halt_seconds((9, 15, 1), (10, 0, 2));
    assert_eq!((halt.from, halt.resume), (33_301, 36_002));
    let row = note(20_000, "an invented holiday", "an invented circular");
    assert_eq!((row.day, row.name), (20_000, "an invented holiday"));
    assert_eq!(row.circular, "an invented circular");
}

/// The newest version's facts are the facts `at` gives it.
#[test]
fn a_days_facts_at_the_newest_version_are_its_facts() {
    let day = FIRST_DAY + 10;
    assert_eq!(
        DayFacts::of(day),
        DayFacts::at(CalendarVersion::newest(), day)
    );
}

/// A span with an end outside the record has no sessions to count, and one
/// that runs backwards has none either.
#[test]
fn sessions_between_refuses_a_span_outside_its_record() {
    assert_eq!(sessions_between(FIRST_DAY - 1, FIRST_DAY), None);
    assert_eq!(sessions_between(LAST_DAY, LAST_DAY + 1), None);
    assert_eq!(sessions_between(FIRST_DAY + 1, FIRST_DAY), None);
}

// ---------------------------------------------------------------------------
// BASE-COV-PULL-PS2 (3 Nov 2026): the calendar's arms Route B found unreached
// on START (2e7a0b6d3b56), by direct call or construction, and three walks of
// the whole record that prove the arms no day of it reaches.
// ---------------------------------------------------------------------------

/// PS2-CA-01: a measured version whose record has a hole (its only holiday
/// circular is 2018's, its measured window starts 2019-12-02) fails the
/// unbroken-record rule; every shipped version holds it.
#[test]
fn a_version_whose_record_has_a_hole_is_refused_as_broken() {
    let broken = MeasuredVersion {
        version: 99,
        days: 30,
        holiday_circulars: 1,
        session_circulars: 0,
        source: "invented: one 2018 circular and a month of measured days",
    };
    assert!(!CalendarVersion::on(broken).record_is_unbroken());
    assert!(!records_are_unbroken(&[MEASURED_VERSIONS[0], broken]));
    assert!(records_are_unbroken(&MEASURED_VERSIONS));
    assert!(records_are_unbroken(&[]));
}

/// PS2-CA-02: a session claiming more windows than it can hold is read to
/// `MAX_WINDOWS` and no further: its bars are its held windows' bars, and a
/// minute outside them is not expected.
#[test]
fn a_session_past_its_window_room_is_read_to_its_room() {
    let mut session = two_windows(Window { from: 555, to: 600 }, Window { from: 700, to: 709 });
    session.count = 9;
    assert_eq!(session.bars(), 46 + 10);
    assert!(session.expects(705));
    assert!(!session.expects(650), "between the windows");
    assert!(!session.expects(1_000), "after every held window");
}

/// PS2-CA-03: the civil arithmetic holds before year 0 as after it: a date of
/// a negative year round-trips to its year, and day numbers far below the
/// epoch name their years.
#[test]
fn civil_days_and_years_hold_before_year_zero() {
    for (year, month, day) in [
        (-1_i64, 1, 1),
        (-1, 12, 31),
        (-400, 3, 1),
        (-401, 2, 28),
        (0, 1, 1),
    ] {
        let number = civil_days(year, month, day);
        assert_eq!(civil_year(number), year, "{year}-{month}-{day}");
    }
    assert_eq!(civil_days(1970, 1, 1), 0);
    assert_eq!(
        civil_year(civil_days(0, 1, 1) - 1),
        -1,
        "the eve of year 0 is in year -1"
    );
    assert!(civil_year(-800_000) < 0);
}

/// PS2-CA-04: a sourced span runs year by year while a year is recorded or
/// measured whole, and stops at the last year a `u16` names: a measured
/// window to 65535 gives a span to the end of 65534. No whole-year circular
/// gives no span.
#[test]
fn a_sourced_span_stops_at_the_last_year_it_can_name() {
    let span = sourced_span(
        &HOLIDAY_CIRCULARS[..1],
        civil_days(2019, 1, 1),
        civil_days(65_535, 12, 31),
    );
    assert_eq!(
        span,
        Some((civil_days(2018, 1, 1), civil_days(65_534, 12, 31)))
    );
    assert_eq!(sourced_span(&[], FIRST_DAY, LAST_DAY), None);
}

/// PS2-CA-05: a second past the last minute a `u16` counts has no phase of
/// record: `Unverified`, never a wrapped minute's phase.
#[test]
fn a_second_past_every_countable_minute_is_unverified() {
    let day = FIRST_DAY + 10;
    for venue in [
        crate::vendor::Venue::NseIndex,
        crate::vendor::Venue::NseCash,
    ] {
        assert_eq!(
            phase_at_second(
                venue,
                day,
                u32::from(u16::MAX) * 60 + 60,
                CasEligibility::Eligible
            ),
            Phase::Unverified
        );
        assert_eq!(
            phase_at_second(venue, day, u32::MAX, CasEligibility::NotEligible),
            Phase::Unverified
        );
    }
}

/// PS2-CA-06: only a session that opens at 09:15, on a venue with a pre-open,
/// has one: a session that opens at any other minute has none before it, and
/// the derivatives venue never has one.
#[test]
fn only_a_nine_fifteen_open_on_a_venue_with_one_pre_opens() {
    use crate::vendor::Venue;
    assert!(pre_opens(Venue::NseCash, OPEN_MINUTE, PRE_OPEN_MINUTE));
    assert!(pre_opens(Venue::NseIndex, OPEN_MINUTE, OPEN_MINUTE - 1));
    assert!(!pre_opens(Venue::NseCash, 1_095, 1_090), "an 18:15 open");
    assert!(!pre_opens(
        Venue::NseIndex,
        OPEN_MINUTE + 1,
        PRE_OPEN_MINUTE
    ));
    assert!(!pre_opens(
        Venue::NseDerivatives,
        OPEN_MINUTE,
        PRE_OPEN_MINUTE
    ));
    assert!(
        !pre_opens(Venue::NseCash, OPEN_MINUTE, OPEN_MINUTE),
        "the open itself"
    );
}

/// PS2-CA-07: a day's phase bands answer `Unverified` for a minute no band
/// holds (before the first); a band of the same phase extends the last; a
/// band past `MAX_BANDS` is not written.
#[test]
fn phase_bands_hold_only_what_they_were_given() {
    let mut phases = DayPhases {
        bands: [Band {
            phase: Phase::Unverified,
            from: 0,
            to: 0,
        }; MAX_BANDS],
        count: 0,
    };
    phases.push(Phase::Continuous, 100, 200);
    assert_eq!(
        phases.phase_at(50),
        Phase::Unverified,
        "before the first band"
    );
    assert_eq!(phases.phase_at(150), Phase::Continuous);
    phases.push(Phase::Continuous, 201, 300);
    assert_eq!(phases.bands().len(), 1, "the same phase extends its band");
    assert_eq!(phases.phase_at(250), Phase::Continuous);
    let mut minute = 301_u16;
    for at in 1..MAX_BANDS {
        let phase = if at % 2 == 0 {
            Phase::Continuous
        } else {
            Phase::Break
        };
        phases.push(phase, minute, minute + 9);
        minute += 10;
    }
    assert_eq!(phases.bands().len(), MAX_BANDS);
    let full = phases;
    let last = phases.bands().last().copied().unwrap();
    // The sixteenth band (at = 15, odd) is a break; a close follows it.
    assert_eq!(last.phase, Phase::Break);
    let other = Phase::PostClose;
    phases.push(other, minute, minute + 9);
    assert_eq!(phases, full, "a band past the room is not written");
    assert_eq!(phases.phase_at(minute + 5), Phase::Unverified);
}

/// PS2-CA-08: a version with no whole-year holiday circular is sourced over
/// its measured window alone; one whose sourced span starts after its
/// measured window's start takes its record's first day from the measured
/// window; and a version whose record holds a day neither span answers
/// writes that day as unmeasured (code 4) into its canonical bytes.
#[test]
fn a_versions_spans_record_and_bytes_follow_its_tables() {
    let measured = MeasuredVersion {
        version: 98,
        days: 40,
        holiday_circulars: 0,
        session_circulars: 0,
        source: "invented: no circular",
    };
    let bare = CalendarVersion::on(measured);
    assert_eq!(bare.sourced(), (FIRST_DAY, last_measured_day(40)));
    assert_eq!(bare.sourced(), bare.measured());
    let later = CalendarVersion {
        sourced: (FIRST_DAY + 10, last_measured_day(40) + 60),
        ..bare
    };
    assert_eq!(later.record(), (FIRST_DAY, last_measured_day(40) + 60));
    let hole_day = FIRST_DAY - 5;
    let holed = CalendarVersion {
        sourced: (FIRST_DAY - 30, FIRST_DAY - 10),
        ..bare
    };
    assert_eq!(holed.record(), (FIRST_DAY - 30, last_measured_day(40)));
    assert_eq!(holed.sourced_kind_of(hole_day), DayKind::Unmeasured);
    let bytes = holed.canonical_bytes();
    let stamp = hole_day.to_le_bytes();
    let at = bytes
        .windows(stamp.len())
        .position(|window| window == stamp)
        .unwrap();
    assert_eq!(
        bytes[at + stamp.len()],
        4,
        "the hole's day is written unmeasured"
    );
}

/// PS2-CA-09: observed days so far apart that their span does not fit hold
/// the first day alone: every other observed day is read past and answers
/// unmeasured, never closed.
#[test]
fn observed_days_too_far_apart_hold_the_first_day_alone() {
    let first = i64::MIN + 10;
    let calendar = Calendar::from_observed(&[
        Observed {
            day: first,
            session: None,
        },
        Observed {
            day: first + 5,
            session: None,
        },
        Observed {
            day: i64::MAX,
            session: None,
        },
    ]);
    assert_eq!(calendar.kind_of(first), DayKind::OpenLengthUnmeasured);
    assert_eq!(calendar.kind_of(first + 5), DayKind::Unmeasured);
    assert_eq!(calendar.kind_of(i64::MAX), DayKind::Unmeasured);
    assert_eq!(calendar.sessions(), 1);
}

/// PS2-CA-10: three walks of the newest version's whole record, the proofs of
/// the arms no day of it reaches (see the BASE-COV-PULL-PS2 (U) list): no
/// stated expiry walks back eight days without a session; every day the
/// calendar of record leaves unsized is the 2026 Muhurat its circular names;
/// and no session notice day is named twice, so every notice is its day's
/// newest.
#[test]
fn the_whole_record_never_reaches_its_walks_unproved_arms() {
    let (first, last) = CalendarVersion::newest().record();
    let mut unsized_days = Vec::new();
    for day in first..=last {
        assert!(
            !matches!(
                settlement_on_or_before(day),
                Err(SettlementRefusal::NoSessionWithin { .. })
            ),
            "day {day}"
        );
        if exchange_kind_of(day) == DayKind::OpenLengthUnmeasured {
            unsized_days.push(day);
        }
    }
    assert_eq!(unsized_days, [20_765], "2026-11-08 alone");
    assert!(
        unsized_days
            .iter()
            .all(|day| listed(&SOURCED_SPECIAL, *day))
    );
    let notices = CalendarVersion::newest().session_circulars();
    for (at, notice) in notices.iter().enumerate() {
        assert!(
            notices[at + 1..]
                .iter()
                .all(|later| later.day != notice.day),
            "day {} named twice",
            notice.day
        );
        assert_eq!(session_circular(notice.day), Some(*notice));
    }
}

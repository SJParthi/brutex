//! BASE-COV-PULL: the gap ledger's corners no test had reached (Route B,
//! recipe v4): every absent minute counted whatever its reason, a range that
//! ends before it begins, and a run left open on another day. Every stamp
//! here is invented; the calendar answers are the repository's own.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail"
)]

use super::*;

/// Micros since the epoch for a day and an IST minute-of-day.
fn at(day: i64, minute: u16) -> i64 {
    (day * 86_400 + i64::from(minute) * 60 - 19_800) * 1_000_000
}

fn gap(day: i64, from: u16, to: u16, reason: Reason) -> Gap {
    Gap {
        day,
        from,
        to,
        reason,
    }
}

/// **EVERY ABSENT MINUTE IS COUNTED, WHATEVER ITS REASON, AND THE SUM STOPS
/// AT THE CEILING RATHER THAN WRAPPING.** The loss is the one reason that is
/// a loss; the absence is all five. A run written backwards is one minute,
/// as [`Gap::minutes`] says, never a negative count.
#[test]
fn every_absent_minute_is_counted_whatever_its_reason_and_saturates() {
    assert_eq!(Ledger::default().absent_minutes(), 0, "nothing absent");

    let mixed = Ledger {
        gaps: vec![
            gap(1, 555, 559, Reason::VendorHole),
            gap(1, 0, 554, Reason::OutsideWindow),
            gap(2, 0, 1_439, Reason::Closed),
            gap(3, 0, 1_439, Reason::Unmeasured),
            gap(4, 600, 609, Reason::NotOwed(NotOwed::Unread)),
            // BACKWARDS: `to` before `from` is one minute, not a wrap.
            gap(5, 700, 690, Reason::VendorHole),
        ],
        ..Ledger::default()
    };
    assert_eq!(mixed.absent_minutes(), 5 + 555 + 1_440 + 1_440 + 10 + 1);
    assert_eq!(
        mixed.lost_minutes(),
        5 + 1,
        "only the vendor holes are lost"
    );
    assert_eq!(mixed.unmeasured_minutes(), 1_440);
    assert_eq!(mixed.not_owed_minutes(), 10);
    assert_eq!(
        mixed.absent_minutes(),
        mixed.lost_minutes() + 555 + 1_440 + mixed.unmeasured_minutes() + mixed.not_owed_minutes(),
        "the five reasons partition the absence"
    );

    // THE WIDEST RUN A GAP CAN HOLD, 65,536 minutes, 65,537 times: the true
    // sum is past u32 and the answer is the ceiling, not a small number.
    let widest = gap(0, 0, u16::MAX, Reason::OutsideWindow);
    assert_eq!(widest.minutes(), 65_536);
    let flood = Ledger {
        gaps: vec![widest; 65_537],
        ..Ledger::default()
    };
    assert!(
        u64::from(widest.minutes()) * 65_537 > u64::from(u32::MAX),
        "the fixture is past the ceiling"
    );
    assert_eq!(flood.absent_minutes(), u32::MAX);
    assert_eq!(flood.lost_minutes(), 0, "and none of it is a loss");
    // AND ONE RUN SHORT OF IT is still the exact sum.
    let exact = Ledger {
        gaps: vec![widest; 65_535],
        ..Ledger::default()
    };
    assert_eq!(exact.absent_minutes(), 65_536 * 65_535);
}

/// **A RANGE THAT ENDS BEFORE IT BEGINS OWES NOTHING**, and the ledger still
/// states the bars it was handed: no day is walked, no run is opened, no
/// minute is expected, and nothing is called truncated — on every door.
#[test]
fn a_range_that_ends_before_it_begins_owes_nothing_and_counts_what_it_holds() {
    let day = 20_668; // 2026-08-03, a full session
    let held = [at(day, 555), at(day, 556), at(day, 557)];
    for (first, last) in [(day + 1, day), (i64::MAX, i64::MIN), (1, 0)] {
        for ledger in [
            classify(&held, first, last),
            classify_spot_index_against(&held, first, last, None),
            classify_derivative_against(&held, first, last, None),
            classify_cash(&held, first, last, None, &Key::NONE),
        ] {
            assert_eq!(
                ledger,
                Ledger {
                    held: 3,
                    ..Ledger::default()
                },
                "{first}..={last}"
            );
        }
    }
    // The same bars over the day itself are the control: owed and found.
    let one = classify(&held, day, day);
    assert_eq!((one.held, one.expected), (3, 375));
    assert_eq!(one.lost_minutes(), 372);
}

/// **A RUN LEFT OPEN ON ANOTHER DAY IS CLOSED BEFORE THE NEXT ONE STARTS**,
/// asked of [`push`] directly: every walk here flushes at the end of a day,
/// so a caller that did not would still get two runs, never one spanning
/// midnight. A contiguous minute of the same reason on the same day extends
/// the run; another reason, or a minute that is not the next, starts one.
#[test]
fn a_run_open_on_another_day_is_closed_before_the_next_starts() {
    let mut ledger = Ledger::default();
    let mut run = Some(gap(1, 1_430, 1_439, Reason::VendorHole));
    // THE NEXT DAY'S FIRST MINUTE: contiguous across midnight, not a run.
    push(&mut ledger, &mut run, 2, 0, 0, Reason::VendorHole);
    assert_eq!(ledger.gaps, [gap(1, 1_430, 1_439, Reason::VendorHole)]);
    assert_eq!(run, Some(gap(2, 0, 0, Reason::VendorHole)));
    // The next minute of the same day and reason extends it.
    push(&mut ledger, &mut run, 2, 1, 1, Reason::VendorHole);
    assert_eq!(run, Some(gap(2, 0, 1, Reason::VendorHole)));
    // Another reason starts a run; so does a minute that is not the next.
    push(&mut ledger, &mut run, 2, 2, 2, Reason::OutsideWindow);
    push(&mut ledger, &mut run, 2, 4, 4, Reason::OutsideWindow);
    // A DAY EARLIER than the open run is no extension either.
    push(&mut ledger, &mut run, 1, 5, 5, Reason::OutsideWindow);
    flush(&mut ledger, &mut run);
    assert_eq!(
        ledger.gaps,
        [
            gap(1, 1_430, 1_439, Reason::VendorHole),
            gap(2, 0, 1, Reason::VendorHole),
            gap(2, 2, 2, Reason::OutsideWindow),
            gap(2, 4, 4, Reason::OutsideWindow),
            gap(1, 5, 5, Reason::OutsideWindow),
        ]
    );
    assert_eq!(run, None, "a flush leaves nothing open");
    // A flush of nothing open writes nothing.
    flush(&mut ledger, &mut run);
    assert_eq!(ledger.gaps.len(), 5);
    assert!(!ledger.truncated);
}

/// **EVERY NOT-OWED ANSWER STATES ITS OWN EVIDENCE**: the day the master
/// states, on either side of a share's history, or that no reading covers
/// the day — one wording for every consumer, each distinct.
#[test]
fn every_not_owed_answer_states_its_own_evidence() {
    let listed_on = Day::new(2021, 1, 29).expect("an invented listing day");
    let delisted_on = Day::new(2024, 2, 13).expect("an invented removal day");
    let before = NotOwed::BeforeListing { listed_on }.evidence();
    let after = NotOwed::AfterDelisting { delisted_on }.evidence();
    let unread = NotOwed::Unread.evidence();
    assert!(
        before.starts_with("before its listing on 2021-01-29, "),
        "{before}"
    );
    assert!(
        after.starts_with("after its removal on 2024-02-13, "),
        "{after}"
    );
    assert!(after.ends_with("with no readmission after it"), "{after}");
    assert_eq!(unread, "no stored reading covers the day");
    assert!(before != after && after != unread && before != unread);
}

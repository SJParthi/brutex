#![cfg(test)]
//! BASE-COV-PULL-PS2 (3 Nov 2026), the (U) audit: recipe v4 counts a direct
//! call as an input, so the tail check's arm no public fold reaches is driven
//! by calling it. Nothing here is a vendor's.

use super::*;

/// A TAIL OF A DAY THAT SCHEDULES NO MINUTE withholds nothing and says
/// nothing. `fold_month` builds a tail only for an open session, but the
/// check asks the session itself, so a closed day's tail is passed over.
#[test]
fn a_tail_of_a_day_that_schedules_no_minute_withholds_nothing() {
    let mut out = Folded {
        bars: Vec::new(),
        notes: Vec::new(),
        tally: Tally::default(),
    };
    let tail = Tail {
        day: 19_000,
        missing: 0,
        close: 60 * MINUTE_MICROS,
        session: crate::calendar::DayKind::Closed,
        midnight: 0,
    };
    flush_tail(tail, Bucket::MINUTE, &mut out);
    assert!(out.notes.is_empty(), "nothing is said of a closed day");
    assert_eq!(out.tally, Tally::default(), "nothing is withheld");
    assert!(out.bars.is_empty());
}

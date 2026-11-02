//! BASE-COV-PULL: the const builders of the calendar's tables, which only the
//! compiler ever ran (Route B counts no compile-time run), and the facts a
//! day answers. Every table row they make here is invented.

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

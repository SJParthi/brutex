#![cfg(test)]
//! BASE-COV-PULL-PS2 (fork B): the once-per-shape memory behind a lock that
//! a panic elsewhere poisoned. The line is said every time rather than lost
//! (never silent), the remembered shape included. Run in a child process of
//! its own (D-0995's runner), so the poison stays out of every other test of
//! this binary, which shares the one process-wide memory.

#![expect(
    clippy::unwrap_used,
    clippy::panic,
    reason = "a test that cannot panic cannot fail; the panic poisons the lock"
)]

use super::*;

#[test]
fn a_poisoned_memory_says_every_change_and_never_loses_one() {
    crate::support::where_permission_binds(
        "gdfl_format::ps2_tests::a_poisoned_memory_says_every_change_and_never_loses_one",
        poisoned_body,
    );
}

fn poisoned_body() {
    let day = Day::new(2024, 4, 2).unwrap();
    // Before the poison: said once, then remembered.
    let remembered = HeaderShape::of(b"ZqPs2Remembered,Second");
    assert!(note_format_changed("stocks", day, "A.NSE.csv", &remembered));
    assert!(!note_format_changed(
        "stocks",
        day,
        "B.NSE.csv",
        &remembered
    ));
    // A thread panics while it holds the memory: the lock is poisoned.
    let said = SHAPES_SAID.get_or_init(Default::default);
    let joined = std::thread::spawn(move || {
        let _held = said.lock().unwrap();
        panic!("an invented panic while the shape memory is held");
    })
    .join();
    assert!(joined.is_err());
    assert!(said.is_poisoned());
    // Every sight is said now: a new shape twice, the remembered one again,
    // and a column-set change through the same door.
    let fresh = HeaderShape::of(b"ZqPs2Fresh");
    assert!(note_format_changed("indices", day, "C.NSE_IDX.csv", &fresh));
    assert!(note_format_changed("indices", day, "D.NSE_IDX.csv", &fresh));
    assert!(note_format_changed("stocks", day, "E.NSE.csv", &remembered));
    assert!(note_columns_changed(
        "options",
        day,
        "F.NFO.csv",
        &remembered
    ));
}

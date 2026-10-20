//! The retry loop's withheld-day count (AH-01 repair round 1).
//!
//! `column_withholding_at_build` pushed a day into its `dropped` list every
//! time the retry tightened it, so a day cut twice was counted twice in the
//! `exact-minute days withheld` event and in the refusal that names how many
//! days were withheld.
#![allow(clippy::expect_used, clippy::indexing_slicing)]

/// A day the retry tightens twice is one withheld day.
#[test]
fn a_day_tightened_twice_is_counted_once() {
    let mut days: Vec<i64> = Vec::new();
    crate::record_day_once(&mut days, 19_800);
    crate::record_day_once(&mut days, 19_800);
    crate::record_day_once(&mut days, 19_801);
    assert_eq!(days, vec![19_800, 19_801]);
}

/// The loop counts through that helper and nowhere else.
#[test]
fn the_retry_loop_counts_days_through_the_helper() {
    let source = include_str!("lib.rs");
    let start = source
        .find("fn column_withholding_at_build(")
        .expect("the loop exists");
    let body = &source[start..];
    let end = body.find("\n}\n").expect("its end");
    let body = &body[..end];
    assert!(body.contains("record_day_once(&mut dropped, day);"));
    assert!(!body.contains("dropped.push("), "a raw push double-counts");
}

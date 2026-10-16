//! The exactness proof in `vocab::implication` points at real source.
//!
//! `ET-vocabulary-conditions-bits-1`: the proof cited `daily.rs:579` and
//! `daily.rs:604` as the sites of the shared band half and the band test. Both
//! line numbers had drifted onto unrelated code, and the quoted test
//! `close > level + half` no longer existed anywhere in `daily.rs` -- the code
//! reads `close > level.saturating_add(half)`. The same stale `daily.rs:579`
//! pointer sat in a runtime string in `engine`.
//!
//! A line number cannot be checked without a parser for the whole file, and
//! it drifts on every unrelated edit above it. The proof now names symbols and
//! quotes source text, and this test holds both halves: no `daily.rs:<line>`
//! pointer survives in either file, and every excerpt the proof quotes is found
//! verbatim in the file it is attributed to.

#![allow(
    clippy::expect_used,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]

const IMPLICATION: &str = include_str!("../src/implication.rs");
const DAILY: &str = include_str!("../../indicators/src/daily.rs");
const ORB: &str = include_str!("../../indicators/src/orb.rs");
const ENGINE: &str = include_str!("../../engine/src/lib.rs");

/// Whether `text` holds `<file>:<digit>`, the shape of a line-number pointer.
fn has_line_pointer(text: &str, file: &str) -> bool {
    text.match_indices(file).any(|(at, _)| {
        text.get(at + file.len()..).is_some_and(|rest| {
            rest.starts_with(':') && rest[1..].starts_with(|c: char| c.is_ascii_digit())
        })
    })
}

#[test]
fn the_pivot_proof_carries_no_line_number_pointer() {
    for (name, text) in [
        ("vocab/src/implication.rs", IMPLICATION),
        ("engine/src/lib.rs", ENGINE),
    ] {
        assert!(
            !has_line_pointer(text, "daily.rs"),
            "{name} cites `daily.rs:<line>`; name the symbol and quote the source instead"
        );
    }
    assert!(
        !has_line_pointer(IMPLICATION, "orb.rs"),
        "vocab/src/implication.rs cites `orb.rs:<line>`"
    );
}

#[test]
fn the_line_pointer_detector_sees_the_shape_it_refuses() {
    assert!(has_line_pointer("see `daily.rs:579` here", "daily.rs"));
    assert!(!has_line_pointer("see `daily.rs` here", "daily.rs"));
    assert!(!has_line_pointer("daily.rs:x", "daily.rs"));
    assert!(!has_line_pointer("daily.rs:", "daily.rs"));
}

#[test]
fn every_excerpt_the_pivot_proof_quotes_is_in_the_source_it_names() {
    let excerpts: [(&str, &str, &str); 9] = [
        ("daily.rs", DAILY, "pub fn from_previous_session("),
        ("daily.rs", DAILY, "if high < low"),
        ("daily.rs", DAILY, "if close < low || close > high"),
        ("daily.rs", DAILY, "let half = levels.band_half();"),
        ("daily.rs", DAILY, "close > level.saturating_add(half)"),
        ("daily.rs", DAILY, "pub const fn cpr_span("),
        ("daily.rs", DAILY, "2 * p - b"),
        ("daily.rs", DAILY, "(p - b).abs()"),
        ("orb.rs", ORB, "self.extremes(usize::from(offset)) else"),
    ];
    for (file, source, excerpt) in excerpts {
        assert!(
            source.contains(excerpt),
            "{file} no longer contains `{excerpt}`"
        );
    }
    for quoted in [
        "`if high < low`",
        "`if close < low || close > high`",
        "`let half = levels.band_half();`",
        "`close > level.saturating_add(half)`",
        "`2 * p - b`",
        "`(p - b).abs()`",
        "`DailyLevels::from_previous_session`",
        "`DailyLevels::cpr_span`",
        "`bits_with`",
    ] {
        assert!(
            IMPLICATION.contains(quoted),
            "the proof no longer quotes {quoted}; drop it from this list in the same change"
        );
    }
    // The shared band half is bound before the loop in the function the proof names.
    let bits_with = DAILY
        .split("pub fn bits_with(")
        .nth(1)
        .expect("daily.rs defines `bits_with`");
    let half = bits_with
        .find("let half = levels.band_half();")
        .expect("`bits_with` binds `half`");
    let plan = bits_with
        .find("for (level, rel) in plan(levels)")
        .expect("`bits_with` loops the plan");
    assert!(
        half < plan,
        "`half` must be bound once, before the loop over the plan"
    );
}

/// The proof's "non-decreasing, including at the top of the type" sentence.
#[test]
fn saturating_addition_keeps_the_rung_order_at_the_top_of_the_type() {
    let edges = [i64::MIN, -1, 0, 1, i64::MAX / 2, i64::MAX - 1, i64::MAX];
    for &lower in &edges {
        for &upper in edges.iter().filter(|&&u| u >= lower) {
            for &half in &[0, 1, i64::MAX / 6, i64::MAX] {
                assert!(
                    lower.saturating_add(half) <= upper.saturating_add(half),
                    "{lower} <= {upper} but +{half} reversed them"
                );
            }
        }
    }
}

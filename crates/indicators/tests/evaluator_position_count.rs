//! `Evaluator::positions`' doc states its own count, and the count is checked.
//!
//! `ET-vocabulary-conditions-bits-2`: the doc read "238 positions today, which is
//! every live bit in the table" while the evaluator returned more and the table's
//! live mask had grown with it. Nothing read the sentence. This reads it against
//! both halves of the claim: the length `positions()` returns, and the live mask.

#![allow(
    clippy::expect_used,
    reason = "the same exception every test module in this workspace takes: a test \
              that cannot panic cannot fail."
)]

use indicators::evaluator::Evaluator;

const EVALUATOR_SRC: &str = include_str!("../src/evaluator.rs");

#[test]
fn the_documented_evaluator_position_count_is_the_live_table() {
    let stated: usize = EVALUATOR_SRC
        .split(" positions today,\n    /// which is every live bit in the table.")
        .next()
        .and_then(|head| head.rsplit(' ').next())
        .and_then(|n| n.parse().ok())
        .expect("evaluator.rs states `N positions today, which is every live bit`");
    let positions = Evaluator::positions();
    assert_eq!(
        stated,
        positions.len(),
        "the doc's count is not what `positions()` returns"
    );
    let live = usize::try_from(vocab::table::LIVE.popcount()).expect("a popcount fits usize");
    assert_eq!(positions.len(), live, "`positions()` is not every live bit");
    let emitted = positions
        .iter()
        .fold(vocab::ConditionMask::ZERO, |m, &p| m.with_bit(u32::from(p)));
    assert_eq!(
        emitted,
        vocab::table::LIVE,
        "`positions()` is not the live mask"
    );
}

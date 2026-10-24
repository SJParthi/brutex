//! C-CHK-09 (CC-20, FM-615, D-6804): `pull::gdfl_nfo::strike_implausible`,
//! the one comparison behind every `StrikeImplausible` refusal at import and
//! at re-key, benched directly (it is a `pub const fn`, so no seam). One
//! saturating multiply and one compare: its cost per call must be the same
//! over a ladder of 1,000, 10,000 and 100,000 strikes, and the same whatever
//! the magnitude of the strike and the median. The harness,
//! `crates/pull/benches/ratio.rs`, fails on a ratio above its ceiling and
//! prints p50, p99 and max beside it.
//!
//! Before anything is timed the extremes are asked and a wrong answer stops
//! the harness: exactly `k` times the median is admitted and one paisa more is
//! refused, an `i64::MAX` median saturates and refuses nothing, a zero median
//! refuses every positive strike and admits zero, and a negative median
//! refuses every strike above its saturated product.
//!
//! Every strike is invented here, in paisa; none is a contract's.

use std::hint::black_box;

use pull::gdfl_nfo::{STRIKE_PLAUSIBLE_MULTIPLE, strike_implausible};

use super::{cost_ps, ratio, refuse, spread};

/// Strikes in the 1x ladder; 10x and 100x hold ten and a hundred times as many.
const BASE: i64 = 1_000;
/// Ladders walked per timed repetition at 1x; fewer at the larger ladders, so
/// each repetition asks the same number of strikes.
const PASSES: u32 = 100;

/// A ladder of `n` invented strikes around `median`, a third of them past the
/// plausible bound, so both answers are in every pass.
fn ladder(n: i64, median: i64) -> Vec<i64> {
    let step = median.saturating_mul(STRIKE_PLAUSIBLE_MULTIPLE + 5) / n.max(1);
    (0..n).map(|i| step.saturating_mul(i) + 1).collect()
}

/// The extremes, asked once: a wrong answer is a refusal, never a timing.
fn the_extremes_answer_as_d3331_says() {
    let k = STRIKE_PLAUSIBLE_MULTIPLE;
    let cases: [(&str, i64, i64, bool); 9] = [
        ("exactly k times the median", 12_345 * k, 12_345, false),
        ("one paisa past k times", 12_345 * k + 1, 12_345, true),
        ("i64::MAX median saturates", i64::MAX, i64::MAX, false),
        ("i64::MAX strike, i64::MAX median", i64::MAX, i64::MAX / k + 1, false),
        ("i64::MAX strike past a real median", i64::MAX, 12_345, true),
        ("zero median, positive strike", 1, 0, true),
        ("zero median, zero strike", 0, 0, false),
        ("negative median, zero strike", 0, -7, true),
        ("i64::MIN median saturates low", i64::MIN, i64::MIN, false),
    ];
    for (label, strike, median, refused) in cases {
        if strike_implausible(strike, median) != refused {
            refuse(&format!("strike_implausible: {label}"));
        }
    }
}

/// C-CHK-09: one call, per strike, at 1x, 10x and 100x the ladder, and at a
/// median a hundred thousand times larger.
pub(crate) fn strike_check_is_flat() -> bool {
    the_extremes_answer_as_d3331_says();
    let median = 1_234_500;
    let sizes = [BASE, BASE * 10, BASE * 100];
    let mut per_call = Vec::with_capacity(sizes.len());
    for (n, passes) in sizes.into_iter().zip([PASSES, PASSES / 10, PASSES / 100]) {
        let strikes = ladder(n, median);
        let refused = strikes
            .iter()
            .filter(|s| strike_implausible(**s, median))
            .count();
        if refused == 0 || refused == strikes.len() {
            refuse("a ladder holding both answers");
        }
        let whole = cost_ps(passes, || {
            strikes
                .iter()
                .filter(|s| strike_implausible(black_box(**s), black_box(median)))
                .count()
        });
        per_call.push(whole / u128::from(u32::try_from(n).unwrap_or(u32::MAX)));
    }
    let [one, ten, hundred] = per_call.as_slice() else {
        refuse("three sizes measured")
    };
    let mut ok = ratio("C-CHK-09 strike check per call, 10x ladder", *one, *ten);
    ok &= ratio(
        "C-CHK-09 strike check per call, 100x ladder",
        *one,
        *hundred,
    );
    // Magnitude: the same ladder's shape at a median 100,000 times larger.
    let large = median.saturating_mul(100_000);
    let big = ladder(BASE, large);
    let at_large = cost_ps(PASSES, || {
        big.iter()
            .filter(|s| strike_implausible(black_box(**s), black_box(large)))
            .count()
    }) / u128::from(u32::try_from(BASE).unwrap_or(u32::MAX));
    ok &= ratio(
        "C-CHK-09 strike check per call, 1e5x median",
        *one,
        at_large,
    );
    let hundred_x = ladder(BASE * 100, median);
    let mut at = 0usize;
    spread("C-CHK-09 strike check, one call", 20_000, || {
        let strike = hundred_x.get(at % hundred_x.len()).copied().unwrap_or(0);
        at += 1;
        black_box(strike_implausible(black_box(strike), black_box(median)));
    });
    ok
}

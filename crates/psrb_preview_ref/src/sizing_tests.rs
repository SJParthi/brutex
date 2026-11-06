#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a test that cannot panic cannot fail."
)]

use super::*;
use crate::rng::SplitMix64;

/// T-F40 and X-F40: the quantity at its edges.
#[test]
fn x_f40_qty_extremes() {
    let cases: [(i64, i64); 10] = [
        (125_000, 400),
        (123_456, 405),
        (50_000_001, 0),
        (50_000_000, 1),
        (1, 50_000_000),
        (0, 0),
        (-5, 0),
        (i64::MAX, 0),     // combo: no costs call, no slot held
        (25_000_000, 2),   // combo: an exact divisor at the top
        (16_666_667, 2),   // oob: 3 x price is 1 paisa past the exposure
    ];
    for (price, want) in cases {
        assert_eq!(qty(price), want, "{price}");
    }
    assert_eq!((ACCOUNT, SLOTS), (200_000_000, 20));
}

/// P-F40-1: the exposure bound on 10,000 prices.
#[test]
fn p_f40_qty_holds_the_exposure_bound() {
    for seed in crate::generator::SEEDS {
        let mut r = SplitMix64::new(seed);
        for _ in 0..2_500 {
            let price = i64::try_from(r.below(60_000_000)).unwrap() + 1;
            let q = i128::from(qty(price));
            assert!(q * i128::from(price) <= i128::from(EXPOSURE));
            assert!((q + 1) * i128::from(price) > i128::from(EXPOSURE));
        }
    }
}

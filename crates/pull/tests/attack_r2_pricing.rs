//! ATTACK ROUND 2, PRICING: the vendor-volatility path and the solved path
//! must refuse a premium at the same bits (D-3117), on random quotes, and a
//! vendor volatility no model accepts must never price.
//!
//! splitmix64, fixed seed, so reruns are byte-identical.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::float_arithmetic
)]

use brutex_core::instrument::{Expiry, OptionSide};
use greeks::error::GreeksError;
use pull::pricing::{PricingError, Quote, Rate, RateSource, VolSource};
use pull::session::Day;
use pull::tenor::{Tenor, YearBasis};

struct Mix(u64);
impl Mix {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

fn rate() -> Rate {
    Rate::measured(0.065, YearBasis::Calendar365, RateSource::Operator).expect("a rate")
}

/// The arbitrage-bound arm of a refusal, if it was one.
fn bound_arm(r: &Result<pull::pricing::Priced, PricingError>) -> Option<&'static str> {
    match r {
        Err(PricingError::Model(GreeksError::PriceBelowIntrinsic { .. })) => Some("below"),
        Err(PricingError::Model(GreeksError::PriceAboveMaximum { .. })) => Some("above"),
        _ => None,
    }
}

fn quote(mix: &mut Mix) -> Quote {
    let d = 1 + mix.below(30) as u8;
    let day = Day::new(2025, 7, d).expect("a July day");
    let minute = mix.below(375) as i64;
    let ts =
        (i64::from(day.days_from_epoch()) * 86_400 - 19_800 + 9 * 3_600 + 15 * 60 + minute * 60)
            * 1_000_000;
    let expiry = Expiry::new(2025, 7, 31).expect("the July expiry");
    let spot = 2_300_000 + mix.below(200_000) as i64;
    // on the 50-rupee ladder, within twenty rungs of the money
    let atm = (spot + 2_500) / 5_000 * 5_000;
    let strike = atm + (mix.below(41) as i64 - 20) * 5_000;
    let side = if mix.below(2) == 0 {
        OptionSide::Call
    } else {
        OptionSide::Put
    };
    let intrinsic = match side {
        OptionSide::Call => (spot - strike).max(0),
        OptionSide::Put => (strike - spot).max(0),
    };
    // premiums straddling both bounds: below, at and just above intrinsic,
    // ordinary, and at or above the spot (a call's maximum) or strike (a put's)
    let premium = match mix.below(6) {
        0 => intrinsic - mix.below(500) as i64,
        1 => intrinsic,
        2 => intrinsic + 1 + mix.below(5) as i64,
        3 => intrinsic + 1 + mix.below(80_000) as i64,
        4 => spot.max(strike) + mix.below(1_000) as i64 - 500,
        _ => 1 + mix.below(spot as u64) as i64,
    }
    .max(1);
    let symbol = brutex_core::symbol::Symbol::new("NIFTY").unwrap();
    Quote {
        ts_micros: ts,
        spot,
        strike,
        premium,
        tenor: Tenor::between(ts, expiry).expect("a live tenor"),
        side,
        slot: pull::pricing::slot_of(symbol).expect("NIFTY is swept"),
        on: pull::pricing::trade_day_of(day).expect("a trade day"),
        vendor: brutex_core::vendor::Vendor::Dhan,
    }
}

/// **ONE PREMIUM, ONE VERDICT, WHICHEVER PATH SUPPLIES THE VOLATILITY.** A
/// premium the solver refuses as below intrinsic or above the maximum is
/// refused by the vendor path under the same arm, for every vendor volatility
/// tried; a premium the solver accepts is accepted by the vendor path handed
/// that very volatility, with the same greeks to the bit.
#[test]
fn the_vendor_and_solved_paths_refuse_the_same_premiums_under_the_same_arm() {
    let mut mix = Mix(0x3180_0000_D317_0001);
    let mut solved_ok = 0usize;
    let mut bounded = 0usize;
    for case in 0..200_000 {
        let q = quote(&mut mix);
        let solved = pull::pricing::price(q, None, rate(), YearBasis::Calendar365);
        let arm = bound_arm(&solved);
        if arm.is_some() {
            bounded += 1;
        }
        for vol in [0.05, 0.2, 1.5] {
            let vendor = pull::pricing::price(q, Some(vol), rate(), YearBasis::Calendar365);
            if arm.is_some() {
                assert_eq!(
                    bound_arm(&vendor),
                    arm,
                    "case {case}: {q:?} at {vol}: solved {solved:?}, vendor {vendor:?}"
                );
            } else {
                assert!(
                    bound_arm(&vendor).is_none(),
                    "case {case}: the vendor path refuses a premium the screen passes: \
                     {q:?} at {vol}: solved {solved:?}, vendor {vendor:?}"
                );
            }
        }
        if let Ok(row) = solved {
            solved_ok += 1;
            assert!(matches!(row.vol_from, VolSource::Solved { .. }));
            let again =
                pull::pricing::price(q, Some(row.volatility), rate(), YearBasis::Calendar365)
                    .unwrap_or_else(|why| {
                        panic!("case {case}: {q:?}: the solved vol refused: {why}")
                    });
            assert_eq!(again.greeks.price.to_bits(), row.greeks.price.to_bits());
            assert_eq!(again.greeks.delta.to_bits(), row.greeks.delta.to_bits());
            assert_eq!(again.moneyness, row.moneyness);
        }
    }
    assert!(
        solved_ok > 10_000,
        "the generator reaches the solver: {solved_ok}"
    );
    assert!(bounded > 10_000, "and the bounds: {bounded}");
}

/// **A VOLATILITY NO MODEL ACCEPTS NEVER PRICES.** Hostile vendor numbers on
/// otherwise good quotes are refused, never answered.
#[test]
fn a_hostile_vendor_volatility_is_refused_never_priced() {
    let mut mix = Mix(0x3180_0000_D317_0002);
    let hostile = [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        0.0,
        -0.0,
        -0.2,
        10.5,
        f64::MAX,
        -f64::MAX,
    ];
    let mut tried = 0usize;
    for _ in 0..5_000 {
        let q = quote(&mut mix);
        for vol in hostile {
            tried += 1;
            let got = pull::pricing::price(q, Some(vol), rate(), YearBasis::Calendar365);
            assert!(got.is_err(), "{q:?} priced at volatility {vol}: {got:?}");
        }
    }
    assert_eq!(tried, 45_000);
}

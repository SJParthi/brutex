//! Adversarial round-1 attack on `greeks`: pricing bounds, greek signs, the
//! normal distribution, the implied-volatility solver and the moneyness
//! ladder. Every property is asserted over a fixed-seed splitmix64 stream or an
//! exhaustive grid of extreme values, so a rerun is byte-identical.
//!
//! The one defect found is D-3100 (an on-grid strike refused as off-grid when
//! the price level is large against the strike interval); its regression is
//! `an_on_grid_strike_is_never_refused_however_large_the_level`.

#![allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::float_arithmetic,
    clippy::float_cmp,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    missing_docs
)]

use greeks::moneyness::{MAX_LEVEL_TO_INTERVAL, MAX_STEPS, STEP_TOLERANCE};
use greeks::solver::{MAX_ITERATIONS, MAX_RELATIVE_UNCERTAINTY};
use greeks::{
    Contract, GreeksError, Method, Moneyness, OptionKind, Position, standard_normal_cdf,
    standard_normal_pdf,
};

const KINDS: [OptionKind; 2] = [OptionKind::Call, OptionKind::Put];

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / 9_007_199_254_740_992.0
    }
    fn log_uniform(&mut self, lo: f64, hi: f64) -> f64 {
        (lo.ln() + self.unit() * (hi.ln() - lo.ln())).exp()
    }
}

/// `(lower, upper)` no-arbitrage bounds and the carry discount, computed here
/// independently of the crate's own `Checked`.
fn bounds(c: &Contract, kind: OptionKind) -> (f64, f64, f64, f64) {
    let carry_discount = (-c.carry * c.years_to_expiry).exp();
    let forward = c.spot * carry_discount;
    let discounted_strike = c.strike * (-c.rate * c.years_to_expiry).exp();
    let (lo, hi) = match kind {
        OptionKind::Call => ((forward - discounted_strike).max(0.0), forward),
        OptionKind::Put => ((discounted_strike - forward).max(0.0), discounted_strike),
    };
    (lo, hi, carry_discount, forward + discounted_strike)
}

/// Every invariant an `Ok` greeks value must satisfy. Returns a reason.
fn violation(c: &Contract, kind: OptionKind, g: &greeks::Greeks) -> Option<&'static str> {
    let (lo, hi, carry_discount, scale) = bounds(c, kind);
    let tolerance = 1e-12 * scale;
    if !g.is_finite() {
        return Some("non-finite Ok");
    }
    if g.price < 0.0 {
        return Some("negative price");
    }
    if g.price < lo - tolerance || g.price > hi + tolerance {
        return Some("price outside no-arbitrage bounds");
    }
    let slack = carry_discount * (1.0 + 4.0 * f64::EPSILON);
    let delta_ok = match kind {
        OptionKind::Call => g.delta >= 0.0 && g.delta <= slack,
        OptionKind::Put => g.delta <= 0.0 && g.delta >= -slack,
    };
    if !delta_ok {
        return Some("delta outside [0, e^-qT] / [-e^-qT, 0]");
    }
    if g.gamma < 0.0 || g.vega < 0.0 {
        return Some("negative gamma or vega");
    }
    None
}

#[test]
fn price_and_greek_signs_hold_on_a_random_stream_of_contracts() {
    let mut rng = Rng(0x00C0_FFEE);
    let mut checked = 0_u32;
    let mut refused = 0_u32;
    for _ in 0..100_000 {
        let spot = rng.log_uniform(1.0, 1.0e6);
        let c = Contract {
            spot,
            strike: spot * rng.log_uniform(0.01, 100.0),
            years_to_expiry: rng.log_uniform(1.0 / 525_600.0, 2.0),
            rate: (rng.unit() - 0.3) * 0.2,
            carry: (rng.unit() - 0.3) * 0.1,
        };
        let vol = rng.log_uniform(0.001, 5.0);
        for kind in KINDS {
            match c.greeks(vol, kind) {
                Ok(g) => {
                    checked += 1;
                    assert_eq!(violation(&c, kind, &g), None, "{c:?} {vol} {kind:?} {g:?}");
                }
                Err(e) => {
                    refused += 1;
                    assert_eq!(e, GreeksError::NotRepresentable, "{c:?} {vol}");
                }
            }
        }
    }
    assert_eq!(
        checked, 200_000,
        "{refused} in-range contracts were refused"
    );
}

#[test]
fn extreme_inputs_never_produce_a_non_finite_or_out_of_bounds_ok() {
    let levels = [
        5e-324,
        f64::MIN_POSITIVE,
        1e-300,
        f64::EPSILON,
        1e-6,
        1.0,
        25_851.19,
        1e6,
        1e11,
        1e12,
        1e12 * (1.0 + f64::EPSILON),
        f64::MAX,
        f64::INFINITY,
        f64::NAN,
        -0.0,
        0.0,
        -1.0,
    ];
    let years = [
        5e-324,
        f64::MIN_POSITIVE,
        1e-15,
        1.0 / 525_600.0,
        1.0 / 365.0,
        2.0,
        100.0,
        100.0_f64.next_up(),
        0.0,
        -0.0,
        f64::NAN,
    ];
    let vols = [
        5e-324,
        1e-300,
        1e-12,
        1e-6,
        1e-3,
        0.2,
        1.0,
        5.0,
        10.0,
        10.000_001,
        0.0,
        -0.2,
        f64::NAN,
    ];
    let rates = [
        -10.0,
        -1.0,
        -0.0,
        0.0,
        0.0946,
        10.0,
        10.000_001,
        f64::INFINITY,
    ];
    let mut evaluated = 0_u64;
    let mut answered = 0_u64;
    let mut solved = 0_u64;
    for &spot in &levels {
        for &strike in &levels {
            for &t in &years {
                for &vol in &vols {
                    for &rate in &rates {
                        for carry in [0.0, 0.05, -1.0] {
                            let c = Contract {
                                spot,
                                strike,
                                years_to_expiry: t,
                                rate,
                                carry,
                            };
                            for kind in KINDS {
                                evaluated += 1;
                                let Ok(g) = c.greeks(vol, kind) else {
                                    continue;
                                };
                                answered += 1;
                                assert_eq!(violation(&c, kind, &g), None, "{c:?} {vol} {g:?}");
                                if let Ok(iv) = c.implied_volatility(g.price, kind) {
                                    solved += 1;
                                    assert!(iv.volatility.is_finite() && iv.vega > 0.0);
                                    assert!(iv.uncertainty <= MAX_RELATIVE_UNCERTAINTY);
                                    assert!(iv.iterations <= MAX_ITERATIONS, "{iv:?}");
                                    let rel = (iv.volatility - vol).abs() / vol;
                                    assert!(
                                        rel <= 1e-9 + 10.0 * iv.uncertainty,
                                        "{c:?} {vol} {kind:?} -> {iv:?} rel {rel}"
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    println!("extremes: {evaluated} evaluated, {answered} answered, {solved} solved");
    assert!(answered > 10_000 && solved > 1_000, "{answered} {solved}");
}

#[test]
fn implied_volatility_round_trips_within_its_stated_uncertainty() {
    let mut rng = Rng(0x0005_EED5);
    let mut solved = 0_u32;
    let mut newton = 0_u32;
    let mut worst = 0.0_f64;
    for _ in 0..60_000 {
        let spot = rng.log_uniform(10.0, 1.0e5);
        let c = Contract {
            spot,
            strike: spot * rng.log_uniform(0.01, 100.0),
            years_to_expiry: rng.log_uniform(1.0 / 525_600.0, 2.0),
            rate: 0.07,
            carry: 0.01,
        };
        let vol = rng.log_uniform(0.001, 5.0);
        for kind in KINDS {
            let price = c.price(vol, kind).expect("in range");
            match c.implied_volatility(price, kind) {
                Ok(iv) => {
                    solved += 1;
                    if iv.method == Method::Newton {
                        newton += 1;
                    }
                    assert!(iv.iterations <= MAX_ITERATIONS, "{iv:?}");
                    let rel = (iv.volatility - vol).abs() / vol;
                    worst = worst.max(rel);
                    assert!(
                        rel <= 1e-9 + 10.0 * iv.uncertainty,
                        "{c:?} {vol} {kind:?} -> {iv:?} rel {rel}"
                    );
                    // And the answer reproduces the quoted price.
                    let back = c.price(iv.volatility, kind).unwrap();
                    assert!(
                        (back - price).abs() <= 1e-9 * (price + c.spot),
                        "{back} vs {price}"
                    );
                }
                Err(
                    GreeksError::PriceBelowIntrinsic { .. }
                    | GreeksError::Indeterminate { .. }
                    | GreeksError::OutsideVolatilityRange { .. },
                ) => {}
                Err(other) => panic!("unexpected refusal {other:?} at {c:?} {vol} {kind:?}"),
            }
        }
    }
    println!("round trip: {solved} solved ({newton} by Newton), worst relative {worst:e}");
    assert!(solved >= 12_000 && newton > 0, "{solved} {newton}");
}

#[test]
fn prices_at_and_beyond_the_no_arbitrage_bounds_are_refused_by_name() {
    let c = Contract {
        spot: 25_851.19,
        strike: 25_000.0,
        years_to_expiry: 7.0 / 365.0,
        rate: 0.07,
        carry: 0.0,
    };
    for kind in KINDS {
        let (lo, hi, _, _) = bounds(&c, kind);
        for price in [lo, lo.next_down(), -0.0, 0.0, -1.0] {
            assert!(
                matches!(
                    c.implied_volatility(price, kind),
                    Err(GreeksError::PriceBelowIntrinsic { .. })
                ),
                "{kind:?} {price}"
            );
        }
        for price in [hi, hi.next_up(), 1e300] {
            assert!(
                matches!(
                    c.implied_volatility(price, kind),
                    Err(GreeksError::PriceAboveMaximum { .. })
                ),
                "{kind:?} {price}"
            );
        }
        for price in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(
                c.implied_volatility(price, kind),
                Err(GreeksError::NotFinite {
                    field: "market_price"
                })
            );
        }
        // One ulp inside each bound: never answered. Below the band's lowest
        // price or above its highest it is outside the band; where the floor
        // of the band rounds onto the intrinsic value itself, a quote one ulp
        // above it carries no volatility and must be Indeterminate.
        for price in [lo.next_up(), hi.next_down()] {
            let out = c.implied_volatility(price, kind);
            println!("{kind:?} {price}: {out:?}");
            assert!(
                matches!(
                    out,
                    Err(GreeksError::OutsideVolatilityRange { .. }
                        | GreeksError::Indeterminate { .. })
                ),
                "{kind:?} {price}: {out:?}"
            );
        }
        // The band is open at 500%: the price at exactly 5.0 is refused, and
        // one just inside it solves.
        let at_top = c.price(greeks::solver::MAX_VOLATILITY, kind).unwrap();
        assert!(matches!(
            c.implied_volatility(at_top, kind),
            Err(GreeksError::OutsideVolatilityRange { .. })
        ));
        let inside = c.price(4.99, kind).unwrap();
        let iv = c.implied_volatility(inside, kind).unwrap();
        assert!((iv.volatility / 4.99 - 1.0).abs() < 1e-9, "{iv:?}");
    }
}

#[test]
fn the_normal_distribution_is_monotone_symmetric_and_total() {
    let mut previous = 0.0_f64;
    let mut x = -40.0_f64;
    let mut points = 0_u32;
    while x <= 40.0 {
        let value = standard_normal_cdf(x);
        assert!((0.0..=1.0).contains(&value), "N({x}) = {value}");
        assert!(value >= previous, "N dropped at {x}: {previous} -> {value}");
        assert_eq!(
            standard_normal_pdf(x),
            standard_normal_pdf(-x),
            "pdf at {x}"
        );
        assert!(standard_normal_pdf(x) >= 0.0);
        if x >= 0.0 {
            assert_eq!(standard_normal_cdf(x) + standard_normal_cdf(-x), 1.0, "{x}");
        }
        previous = value;
        x += 1e-4;
        points += 1;
    }
    // Ulp-level monotonicity across the two branch cuts and the tail cut.
    for cut in [7.071_067_811_865_475_f64, 37.0] {
        for edge in [cut, -cut] {
            let mut y = edge;
            for _ in 0..1000 {
                y = y.next_down();
            }
            let mut last = standard_normal_cdf(y);
            for _ in 0..2000 {
                y = y.next_up();
                let now = standard_normal_cdf(y);
                assert!(now >= last, "N dropped at {y}");
                last = now;
            }
        }
    }
    assert_eq!(standard_normal_cdf(f64::INFINITY), 1.0);
    assert_eq!(standard_normal_cdf(f64::NEG_INFINITY), 0.0);
    assert_eq!(standard_normal_cdf(0.0), 0.5);
    assert_eq!(standard_normal_cdf(-0.0), 0.5);
    assert_eq!(standard_normal_pdf(f64::INFINITY), 0.0);
    assert_eq!(standard_normal_pdf(f64::NEG_INFINITY), 0.0);
    assert!(standard_normal_cdf(f64::NAN).is_nan());
    assert!(standard_normal_pdf(f64::NAN).is_nan());
    assert!(points >= 799_000, "{points}");
}

#[test]
fn an_on_grid_strike_is_never_refused_however_large_the_level() {
    // D-3100. Built exactly the way a caller builds a ladder. Before the fix
    // the (1e9, 0.03) row refused 19,200 of these 40,001 strikes as off-grid.
    let ladders = [
        (25_850.0_f64, 50.0_f64),
        (1_234.5, 2.5),
        (1.0e9, 0.03),
        (1.0e8, 0.007),
        (1.5e5, 0.000_01),
        (1.0e5, 0.000_003),
        (1.0e12 - 1.0e9, 5.0),
    ];
    let mut placed = 0_u32;
    for (atm, interval) in ladders {
        for k in -20_000_i32..=20_000 {
            let strike = atm + f64::from(k) * interval;
            if strike <= 0.0 {
                continue;
            }
            for kind in KINDS {
                let rung = Moneyness::from_ladder(strike, atm, interval, kind)
                    .unwrap_or_else(|e| panic!("atm {atm} interval {interval} k {k}: {e}"));
                assert_eq!(rung.steps, k);
                assert_eq!(rung.distance, k.unsigned_abs());
                let expected = match (k.signum(), kind) {
                    (0, _) => Position::AtTheMoney,
                    (1, OptionKind::Call) | (-1, OptionKind::Put) => Position::OutOfTheMoney,
                    _ => Position::InTheMoney,
                };
                assert_eq!(rung.position, expected);
                placed += 1;
            }
        }
    }
    assert_eq!(placed, 482_032, "every positive on-grid strike, both kinds");
}

#[test]
fn a_ladder_too_fine_to_resolve_is_refused_by_name_and_a_half_step_still_is() {
    // Past the bound a strike cannot be placed on one rung rather than its
    // neighbour, so the answer is a named refusal, not a guess.
    let atm = 1.0e12;
    let interval = 1.0e-3;
    assert!(atm / interval > MAX_LEVEL_TO_INTERVAL);
    let error = Moneyness::from_ladder(atm, atm, interval, OptionKind::Call).unwrap_err();
    assert!(
        matches!(
            error,
            GreeksError::OutOfRange {
                field: "level_to_interval",
                ..
            }
        ),
        "{error}"
    );
    // Widening the tolerance did not swallow a genuine half step anywhere the
    // ladder is resolvable, and at an ordinary level it is still the floor.
    for (atm, interval) in [(25_850.0_f64, 50.0_f64), (1.0e9, 0.03), (1.0e11, 0.01)] {
        let error = Moneyness::from_ladder(atm + 1.5 * interval, atm, interval, OptionKind::Put)
            .unwrap_err();
        assert!(matches!(error, GreeksError::OffGrid { .. }), "{error}");
    }
    assert_eq!(
        Moneyness::from_ladder(25_875.0, 25_850.0, 50.0, OptionKind::Call).unwrap_err(),
        GreeksError::OffGrid {
            steps: 0.5,
            tolerance: STEP_TOLERANCE
        }
    );
    // The step bound is still enforced on both sides.
    let far = 25_850.0 + f64::from(MAX_STEPS + 1) * 50.0;
    assert!(matches!(
        Moneyness::from_ladder(far, 25_850.0, 50.0, OptionKind::Call),
        Err(GreeksError::OutOfRange { field: "steps", .. })
    ));
}

#[test]
fn the_solver_cost_is_a_constant_independent_of_the_input() {
    // O(1) per solve is a bound on model evaluations, and that bound is a
    // constant of the crate, not of the input.
    assert_eq!(MAX_ITERATIONS, 86);
    let mut rng = Rng(0x0000_B0B0);
    let mut most = 0_u32;
    for _ in 0..20_000 {
        let spot = rng.log_uniform(10.0, 1.0e5);
        let c = Contract {
            spot,
            strike: spot * rng.log_uniform(0.5, 2.0),
            years_to_expiry: rng.log_uniform(1.0 / 525_600.0, 2.0),
            rate: 0.07,
            carry: 0.0,
        };
        let (lo, hi, _, _) = bounds(&c, OptionKind::Call);
        // An arbitrary price strictly inside the bounds, not one the model made.
        let price = lo + (hi - lo) * rng.unit();
        if let Ok(iv) = c.implied_volatility(price, OptionKind::Call) {
            most = most.max(iv.iterations);
        }
    }
    assert!(most <= MAX_ITERATIONS && most > 0, "{most}");
}

/// Per-call latency of `price`, `greeks` and `implied_volatility`. Timing is
/// machine-dependent, so this prints rather than gates; run with
/// `--release -- --ignored --nocapture`.
#[test]
#[ignore = "timing report; run explicitly in release"]
fn per_call_latency_report() {
    use std::hint::black_box;
    use std::time::Instant;
    fn summarise(name: &str, n: usize, samples: &mut [u128]) -> (u128, u128, u128) {
        samples.sort_unstable();
        let p50 = samples[n / 2];
        let p99 = samples[n * 99 / 100];
        let max = samples[n - 1];
        println!("{name:>8} n={n:>8}: p50 {p50} ns, p99 {p99} ns, max {max} ns");
        (p50, p99, max)
    }
    for n in [1_000_usize, 10_000, 100_000, 1_000_000] {
        let mut rng = Rng(n as u64);
        let mut inputs = Vec::with_capacity(n);
        for _ in 0..n {
            let spot = rng.log_uniform(10.0, 1.0e5);
            let c = Contract {
                spot,
                strike: spot * rng.log_uniform(0.7, 1.4),
                years_to_expiry: rng.log_uniform(1.0 / 525_600.0, 2.0),
                rate: 0.07,
                carry: 0.0,
            };
            let vol = rng.log_uniform(0.05, 2.0);
            let price = c.price(vol, OptionKind::Call).unwrap();
            inputs.push((c, vol, price));
        }
        let mut samples = vec![0_u128; n];
        for (i, (c, vol, _)) in inputs.iter().enumerate() {
            let start = Instant::now();
            black_box(c.price(black_box(*vol), OptionKind::Call)).ok();
            samples[i] = start.elapsed().as_nanos();
        }
        summarise("price", n, &mut samples);
        for (i, (c, vol, _)) in inputs.iter().enumerate() {
            let start = Instant::now();
            black_box(c.greeks(black_box(*vol), OptionKind::Put)).ok();
            samples[i] = start.elapsed().as_nanos();
        }
        summarise("greeks", n, &mut samples);
        let mut most = 0;
        for (i, (c, _, price)) in inputs.iter().enumerate() {
            let start = Instant::now();
            let out = black_box(c.implied_volatility(black_box(*price), OptionKind::Call));
            samples[i] = start.elapsed().as_nanos();
            if let Ok(iv) = out {
                most = most.max(iv.iterations);
            }
        }
        summarise("solve_iv", n, &mut samples);
        assert!(most <= MAX_ITERATIONS);
    }
}

#[test]
fn a_subnormal_scale_reports_the_last_place_its_quote_really_has() {
    // D-3101. Before the fix this was answered with uncertainty 1.55e-10 and
    // an error of 8.6e-9 of itself: the stated last place was 150x finer than
    // the spacing of any double at that magnitude.
    let c = Contract {
        spot: f64::MIN_POSITIVE,
        strike: f64::MIN_POSITIVE,
        years_to_expiry: 100.0,
        rate: 0.0946,
        carry: 0.05,
    };
    let price = c.price(1.0, OptionKind::Call).unwrap();
    let smallest = f64::from_bits(1);
    match c.implied_volatility(price, OptionKind::Call) {
        Ok(iv) => {
            let error = (iv.volatility - 1.0).abs();
            assert!(
                iv.uncertainty >= smallest / (iv.vega * iv.volatility),
                "{iv:?}"
            );
            assert!(error <= iv.uncertainty, "error {error:e} against {iv:?}");
        }
        Err(e) => assert!(matches!(e, GreeksError::Indeterminate { .. }), "{e}"),
    }
}

#[test]
fn a_ladder_exactly_at_the_resolution_bound_is_placed_and_one_ulp_past_it_is_not() {
    // THE BOUND IS INCLUSIVE, AND IT IS AN EXACT DOUBLE (D-3129). It is
    // `0.25 / (4 * EPSILON)`, which is 2^48, so a level of 2^39 (inside
    // `MAX_UNDERLYING`) over an interval of 2^-9 sits on it with no rounding. "Past it" is refused; at
    // it the slack is exactly a quarter step and the strike is placed.
    assert_eq!(MAX_LEVEL_TO_INTERVAL, 2.0_f64.powi(48));
    let atm = 2.0_f64.powi(39);
    let interval = 2.0_f64.powi(-9);
    assert_eq!(atm / interval, MAX_LEVEL_TO_INTERVAL);
    for kind in [OptionKind::Call, OptionKind::Put] {
        let at = Moneyness::from_ladder(atm, atm, interval, kind).expect("at the bound");
        assert_eq!(
            at,
            Moneyness::from_ladder(1.0, 1.0, 1.0, kind).expect("ordinary")
        );
        let finer = f64::from_bits(interval.to_bits() - 1);
        assert!(atm / finer > MAX_LEVEL_TO_INTERVAL);
        let error = Moneyness::from_ladder(atm, atm, finer, kind).unwrap_err();
        assert!(
            matches!(
                error,
                GreeksError::OutOfRange {
                    field: "level_to_interval",
                    ..
                }
            ),
            "{error}"
        );
    }
}

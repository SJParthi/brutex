#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "the same exception every test module in this workspace takes -- a \
              test that cannot panic cannot fail. `expect` and not \
              `unreachable!`, because `unreachable!` expands to a panic inside \
              the crate under test and leaves a coverage region no green run \
              can execute, while `expect` panics inside the standard library \
              and leaves none."
)]

//! THE MONEY LAYER, ATTACKED FROM OUTSIDE, ON THE CROSS PRODUCT OF ITS EXTREMES.
//!
//! # Why this file exists
//!
//! `crates/costs` had **182 unit tests and no integration test at all**. Its
//! surface had never been driven the way `runner` drives it, which is the same
//! gap `crates/engine` carried until `009e540` — except this is the crate where
//! being wrong costs actual money.
//!
//! # Why the rates are the SHIPPED ones and not invented
//!
//! `BpsX100::new` is `pub(crate)`, so a caller outside this crate cannot mint a
//! rate at all. That is deliberate and it is load-bearing: every rate this file
//! uses is a `pub const` the crate publishes, so the matrix is the arithmetic
//! against the charges that actually apply, not against numbers chosen to make
//! the arithmetic comfortable. They span **zero to 1,800,000** — `BSE_IPFT` to
//! `GST_ON_FEE_BASE` — which is the whole range the narrowing has to survive.
//!
//! # The invariants, and why each is a defect if it fails
//!
//! 1. **Nothing panics.** Every cell runs, at both ends of `i64`.
//! 2. **A charge is never negative on a non-negative notional.** A levy that
//!    pays the trader is not a rounding quirk; it is money moving the wrong way.
//! 3. **A charge never falls as the notional rises.** Non-monotone cost makes a
//!    larger trade cheaper than a smaller one, which no fee schedule does.
//! 4. **`ceil_to_paisa` is never below `floor_to_paisa`, and never more than one
//!    paisa above it.** They are two roundings of one quotient.
//! 5. **A statutory levy is a whole number of rupees.** That is what "statutory"
//!    means here; a levy carrying paisa has not been rounded at all.
//! 6. **Overflow is a `Result`, never a wrap.** §7 keeps prices in `i64`, and a
//!    wrapped levy is a negative charge wearing a positive one's clothes.
//! 7. **Determinism.** The same inputs give the same answer, always.

use brutex_core::price::Paisa;
use costs::money::{
    ceil_to_paisa, ceil_to_rupee, floor_to_paisa, levy_ceiling, scaled, statutory_levy,
};
use costs::rate::{BSE_IPFT, GST_ON_FEE_BASE, NSE_IPFT, SEBI_TURNOVER_FEE, STAMP_DUTY_BUY_SIDE};
use costs::regime::{BSE_EXCHANGE_CHARGE, NSE_EXCHANGE_CHARGE};

/// Every rate this crate publishes, spanning zero to the GST base.
fn rates() -> Vec<(&'static str, costs::rate::BpsX100)> {
    vec![
        ("BSE_IPFT (zero)", BSE_IPFT),
        ("SEBI_TURNOVER_FEE", SEBI_TURNOVER_FEE),
        ("NSE_IPFT", NSE_IPFT),
        ("STAMP_DUTY_BUY_SIDE", STAMP_DUTY_BUY_SIDE),
        ("BSE_EXCHANGE_CHARGE", BSE_EXCHANGE_CHARGE),
        ("NSE_EXCHANGE_CHARGE", NSE_EXCHANGE_CHARGE),
        ("GST_ON_FEE_BASE", GST_ON_FEE_BASE),
    ]
}

/// Every extreme notional, including both ends of the type.
fn notionals() -> Vec<(&'static str, Paisa)> {
    vec![
        ("zero", Paisa::ZERO),
        ("one paisa", Paisa::from_raw(1)),
        ("99 paisa -- one below a rupee", Paisa::from_raw(99)),
        ("one rupee", Paisa::from_raw(100)),
        ("101 paisa -- one above a rupee", Paisa::from_raw(101)),
        ("a lakh", Paisa::from_raw(10_000_000)),
        ("a crore", Paisa::from_raw(1_000_000_000)),
        ("i64::MAX", Paisa::from_raw(i64::MAX)),
        ("i64::MAX - 1", Paisa::from_raw(i64::MAX - 1)),
    ]
}

/// THE MATRIX: every notional against every shipped rate.
#[test]
fn every_extreme_notional_against_every_shipped_rate_holds_the_money_invariants() {
    let mut cells = 0_u32;
    for (nname, notional) in notionals() {
        for (rname, rate) in rates() {
            let where_ = format!("[{nname}] x [{rname}]");
            cells += 1;

            // 1. Nothing panics. `scaled` is i128 by construction, so the
            //    multiply cannot overflow for any i64 pair; the narrowing is
            //    where the failure lives, and it must be a value.
            let s = scaled(notional, rate);

            let ceil = ceil_to_paisa(s);
            let floor = floor_to_paisa(s);

            // 4. Two roundings of one quotient: ordered, and at most one apart.
            if let (Ok(c), Ok(f)) = (ceil, floor) {
                assert!(
                    c.raw() >= f.raw(),
                    "{where_}: ceil {} is below floor {}",
                    c.raw(),
                    f.raw()
                );
                assert!(
                    c.raw() - f.raw() <= 1,
                    "{where_}: ceil {} and floor {} are more than one paisa \
                     apart, so they are not two roundings of one quotient",
                    c.raw(),
                    f.raw()
                );
            }

            // 2. A charge is never negative on a non-negative notional and a
            //    non-negative rate. Both hold for every cell in this matrix.
            assert!(notional.raw() >= 0 && rate.get() >= 0, "{where_}: fixture");
            if let Ok(levy) = levy_ceiling(notional, rate) {
                assert!(
                    levy.raw() >= 0,
                    "{where_}: a levy of {} pays the trader",
                    levy.raw()
                );
            }
            if let Ok(levy) = statutory_levy(notional, rate) {
                assert!(
                    levy.raw() >= 0,
                    "{where_}: a statutory levy of {} pays the trader",
                    levy.raw()
                );
                // 5. And it is a whole number of rupees.
                assert_eq!(
                    levy.raw() % 100,
                    0,
                    "{where_}: a statutory levy of {} carries paisa, so it was \
                     never rounded to the rupee",
                    levy.raw()
                );
            }

            // 7. Determinism.
            assert_eq!(
                levy_ceiling(notional, rate).map(Paisa::raw),
                levy_ceiling(notional, rate).map(Paisa::raw),
                "{where_}: two identical calls disagreed"
            );
        }
    }
    assert_eq!(
        cells, 63,
        "9 notionals x 7 rates = 63 cells; a generator that produced fewer \
         would pass every assertion above without testing anything"
    );
}

/// 3. A CHARGE NEVER FALLS AS THE NOTIONAL RISES.
///
/// Asserted across the whole ascending ladder rather than on one pair, because
/// non-monotonicity from a rounding bug shows up between two ADJACENT values and
/// a spot check walks straight past it. Where a step refuses, the refusal ends
/// the comparison for that rate — an `Err` is not a smaller charge, it is no
/// charge at all, and treating it as one would assert the opposite of this rule.
#[test]
fn a_larger_notional_is_never_charged_less_than_a_smaller_one() {
    let ladder: Vec<Paisa> = [
        0,
        1,
        2,
        99,
        100,
        101,
        999,
        1_000,
        10_000_000,
        1_000_000_000,
        1_000_000_000_000,
    ]
    .into_iter()
    .map(Paisa::from_raw)
    .collect();

    for (rname, rate) in rates() {
        let mut previous: Option<i64> = None;
        for n in &ladder {
            match levy_ceiling(*n, rate) {
                Ok(levy) => {
                    if let Some(prev) = previous {
                        assert!(
                            levy.raw() >= prev,
                            "[{rname}]: a notional of {} is charged {} against \
                             {prev} for a smaller one -- a larger trade cannot \
                             cost less",
                            n.raw(),
                            levy.raw()
                        );
                    }
                    previous = Some(levy.raw());
                }
                // A refusal is not a cheaper charge. Stop comparing rather than
                // read it as one.
                Err(_) => break,
            }
        }
    }
}

/// 6. THE ROUNDING TO A RUPEE OVERFLOWS AT THE TOP OF THE TYPE, AND SAYS SO.
///
/// This is the one case in the whole surface where the arithmetic genuinely
/// leaves `i64`, and it is reachable with a single value rather than a
/// contrived pair. `ceil_to_rupee` computes `ceil(p / 100) * 100`, so at
/// `p = i64::MAX = 9_223_372_036_854_775_807` the ceiling is
/// `92_233_720_368_547_759` rupees and multiplying back gives
/// `9_223_372_036_854_775_900` — **93 paisa above `i64::MAX`**.
///
/// A wrap here would produce a large NEGATIVE levy, which is a charge that pays
/// the trader roughly ninety-two thousand crore. It must be an `Err`.
#[test]
fn rounding_the_top_of_the_type_up_to_a_rupee_refuses_rather_than_wraps() {
    let top = Paisa::from_raw(i64::MAX);
    let out = ceil_to_rupee(top);
    assert!(
        out.is_err(),
        "ceiling i64::MAX to the rupee returned {:?} -- the true answer is 93 \
         paisa above the type and a wrap would be a negative charge",
        out.map(Paisa::raw)
    );

    // And the largest value that DOES fit still rounds correctly, so the
    // refusal above is a boundary and not a blanket.
    let fits = Paisa::from_raw(i64::MAX - 99);
    // A refusal here is also acceptable -- the boundary may sit lower than this
    // value. What is NOT acceptable is a wrap, and the assertion above pins that.
    if let Ok(v) = ceil_to_rupee(fits) {
        assert_eq!(v.raw() % 100, 0, "a rupee-rounded value carries no paisa");
        assert!(v.raw() >= fits.raw(), "rounding UP never goes down");
    }

    // Zero and one paisa are the other end of the same rule.
    assert_eq!(
        ceil_to_rupee(Paisa::ZERO)
            .expect("zero rounds to zero")
            .raw(),
        0
    );
    assert_eq!(
        ceil_to_rupee(Paisa::from_raw(1))
            .expect("one paisa rounds up to one rupee")
            .raw(),
        100,
        "one paisa is not zero rupees -- rounding a levy DOWN would be the \
         exchange charging less than the schedule says"
    );
}

/// A ZERO RATE COSTS NOTHING, AT EVERY NOTIONAL INCLUDING THE TOP OF THE TYPE.
///
/// `BSE_IPFT` is genuinely zero, so this is not a synthetic case: it is the
/// charge that applies on one of the two exchanges. Zero times anything must be
/// zero, and at `i64::MAX` that is the multiplication most likely to be wrong.
#[test]
fn the_zero_rate_charges_nothing_even_at_the_top_of_the_type() {
    for (nname, n) in notionals() {
        assert_eq!(
            scaled(n, BSE_IPFT),
            0,
            "[{nname}]: a zero rate produced a non-zero scaled product"
        );
        assert_eq!(
            levy_ceiling(n, BSE_IPFT).map(Paisa::raw),
            Ok(0),
            "[{nname}]: a zero rate charged something"
        );
        assert_eq!(
            statutory_levy(n, BSE_IPFT).map(Paisa::raw),
            Ok(0),
            "[{nname}]: a zero statutory rate charged something"
        );
    }
}

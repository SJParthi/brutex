//! BASE-COV-PULL-PS1 (D-7335): a contract the dated strike ladder cannot
//! place, by day and by spot, and the bound on the reasons a batch keeps.
//! Every quote below is invented.

#![allow(
    clippy::expect_used,
    clippy::panic,
    reason = "a test that cannot panic cannot fail"
)]

use super::*;
use brutex_core::instrument::Expiry;

/// A rate for these units only (the module's own tests say why no constant of
/// this type exists).
fn rate() -> Rate {
    Rate::measured(0.065, YearBasis::Calendar365, RateSource::Operator)
        .expect("a plausible rate with a provenance")
}

/// 09:15 IST on 2026-08-20, as micros.
fn stamp() -> i64 {
    let d = crate::session::Day::new(2026, 8, 20).expect("a real day");
    (i64::from(d.days_from_epoch()) * 86_400 - 19_800 + 9 * 3_600 + 15 * 60) * 1_000_000
}

/// An invented at-the-money call on the swept index slot, seven days out.
fn call() -> Quote {
    let symbol = brutex_core::symbol::Symbol::new("NIFTY").expect("a swept underlying");
    Quote {
        ts_micros: stamp(),
        spot: 2_500_000,
        strike: 2_500_000,
        premium: 25_000,
        tenor: Tenor::between(stamp(), Expiry::new(2026, 8, 27).expect("a real expiry"))
            .expect("a live tenor"),
        side: OptionSide::Call,
        slot: costs::venue::swept_slot(symbol).expect("a recorded regime"),
        on: TradeDay::new(2026, 8, 20).expect("a real trade day"),
        vendor: brutex_core::vendor::Vendor::Groww,
    }
}

/// **A CONTRACT THE DATED LADDER CANNOT PLACE IS REFUSED IN THE LADDER'S OWN
/// WORDS, NEVER PRICED**: a trade day before the ladder's first verified row
/// (1990-01-02, the window's first year) is `LadderUnknown` from the step, and
/// a spot of one paisa, whose nearest rung is rung zero, is `LadderUnknown`
/// from the at-the-money strike. Extremes at once: the earliest day the window
/// admits, and the smallest positive spot beside a full-size strike.
#[test]
fn a_contract_the_ladder_cannot_place_is_refused_in_its_words() {
    let early = Quote {
        on: TradeDay::new(1990, 1, 2).expect("the window's first year"),
        ..call()
    };
    let PricingError::LadderUnknown { why } =
        price(early, Some(0.2), rate(), YearBasis::Calendar365).expect_err("no step then")
    else {
        panic!("a day the ladder does not cover is LadderUnknown");
    };
    assert!(!why.is_empty());
    let said = PricingError::LadderUnknown { why: why.clone() }.to_string();
    assert!(
        said.starts_with("the dated strike ladder could not place this contract"),
        "{said}"
    );
    assert!(said.ends_with(&why), "{said}");
    let tiny = Quote { spot: 1, ..call() };
    let PricingError::LadderUnknown { why: rung } =
        price(tiny, Some(0.2), rate(), YearBasis::Calendar365).expect_err("rung zero")
    else {
        panic!("a spot on rung zero is LadderUnknown");
    };
    assert!(!rung.is_empty());
    assert_ne!(
        rung, why,
        "the step and the strike refuse in different words"
    );
}

/// **A BATCH KEEPS AT MOST [`REASONS_KEPT`] REASONS, EACH ONCE, AND COUNTS
/// EVERY REFUSAL**: seven refused quotes carrying six distinct reasons (and
/// one repeated) keep the first five reasons, in order, and count seven; the
/// one good quote is priced. Extremes at once: the bound reached and passed,
/// and a repeat met after the bound.
#[test]
fn a_batch_keeps_its_first_reasons_once_and_counts_every_refusal() {
    let refused = |spot: i64| Quote { spot, ..call() };
    let quotes = [
        refused(-1),
        refused(-2),
        refused(-1),
        call(),
        refused(-3),
        refused(-4),
        refused(-5),
        refused(-6),
    ];
    let all = price_all(&quotes, |_| Some(0.2), rate(), YearBasis::Calendar365);
    assert_eq!(all.refused, 7);
    assert_eq!(all.rows.len(), 1);
    assert_eq!(all.why.len(), REASONS_KEPT);
    let firsts: Vec<String> = [-1, -2, -3, -4, -5]
        .into_iter()
        .map(|paisa| {
            PricingError::NotPositive {
                field: "spot",
                paisa,
            }
            .to_string()
        })
        .collect();
    assert_eq!(all.why, firsts);
}

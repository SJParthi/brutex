#![cfg(test)]
//! ATTACK ROUND 3, API: the interactions of the round-1 and round-2 fixes with
//! the receipts and the chain pricing that consume them.
#![allow(clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

use super::*;

/// 2025-07-01 10:00 IST, as UTC microseconds.
const STAMP: i64 = 1_751_344_200_000_000;

fn index_bar(ts_micros: i64, close: i64) -> store::format::Bar {
    store::format::Bar {
        ts_micros,
        open: close,
        high: close,
        low: close,
        close,
        volume: 0,
        open_interest: store::format::OI_NULL,
    }
}

fn contract() -> pull::fno::Found {
    let expiry = brutex_core::instrument::Expiry::new(2025, 7, 31).expect("a real expiry");
    pull::fno::read_contract("NSE-NIFTY-31Jul25-24000-CE", expiry).expect("a readable contract")
}

fn month(found: &pull::fno::Found) -> ChainMonth<'_> {
    let (strike, side) = found.option.expect("an option contract");
    let rate = pull::pricing::Rate::measured(
        0.065,
        pull::tenor::YearBasis::Calendar365,
        pull::pricing::RateSource::Operator,
    )
    .expect("a finite operator rate");
    let symbol = brutex_core::symbol::Symbol::new("NIFTY").expect("a symbol");
    ChainMonth {
        found,
        chunk: pull::session::Window::new(
            Day::new(2025, 7, 1).expect("day"),
            Day::new(2025, 7, 31).expect("day"),
        )
        .expect("a July window"),
        timeframe: store::path::Timeframe::MINUTE_1,
        inputs: PriceInputs {
            strike: strike.raw(),
            side,
            expiry: found.expiry,
            slot: pull::pricing::slot_of(symbol).expect("NIFTY has a ladder slot"),
            rate,
            vendor: Vendor::Groww,
        },
    }
}

/// ROUND 3, D-3123: D-3110 made a stamp where two index bars disagree answer
/// nothing and gave `SpotBook::lookup` a refusal that says so
/// (`SpotAmbiguous`). The only production consumer, `chain_quotes`, calls
/// `SpotBook::at` and reports every `None` as "no index bar is stored at this
/// option bar's stamp" — a false sentence when two are stored there, and the
/// receipt then tells the operator to go and fetch an index minute that is
/// already on disk. The ambiguous stamp must be named as a disagreement, and a
/// genuinely missing stamp must still be named as missing.
#[test]
fn a_chain_row_at_a_contradicted_index_stamp_is_refused_as_a_disagreement() {
    let found = contract();
    let month_of = month(&found);
    let book =
        pull::pricing::SpotBook::of(&[index_bar(STAMP, 2_500_000), index_bar(STAMP, 2_600_000)]);
    assert_eq!(book.ambiguous(), 1, "the fixture's stamp is contradicted");
    let option = [index_bar(STAMP, 15_000)];
    let mut out = pull::pricing::PricedAll::default();
    let quotes = chain_quotes(&option, month_of, &book, &mut out);
    assert!(
        quotes.is_empty(),
        "nothing is priced at a contradicted stamp"
    );
    assert_eq!(out.refused, 1);
    assert_eq!(out.why.len(), 1, "{:?}", out.why);
    assert!(
        out.why[0].contains("disagree"),
        "the refusal names the disagreement: {:?}",
        out.why
    );
    assert!(
        !out.why[0].contains("no index bar is stored"),
        "two bars ARE stored at this stamp: {:?}",
        out.why
    );

    // MANY CONTRADICTED MINUTES ARE ONE REASON, not one per stamp, or a month
    // of them would crowd every other refusal off the receipt (D-3111).
    let later = STAMP + 60_000_000;
    let book = pull::pricing::SpotBook::of(&[
        index_bar(STAMP, 2_500_000),
        index_bar(STAMP, 2_600_000),
        index_bar(later, 2_500_000),
        index_bar(later, 2_700_000),
    ]);
    let options = [index_bar(STAMP, 15_000), index_bar(later, 15_000)];
    let mut many = pull::pricing::PricedAll::default();
    assert!(chain_quotes(&options, month(&found), &book, &mut many).is_empty());
    assert_eq!(many.refused, 2);
    assert_eq!(many.why.len(), 1, "{:?}", many.why);

    // AND A STAMP WITH NO INDEX BAR AT ALL STILL SAYS SO.
    let empty = pull::pricing::SpotBook::of(&[]);
    let mut missing = pull::pricing::PricedAll::default();
    let none = chain_quotes(&option, month(&found), &empty, &mut missing);
    assert!(none.is_empty());
    assert_eq!(missing.refused, 1);
    assert!(
        missing.why[0].contains("no index bar is stored"),
        "{:?}",
        missing.why
    );
}

/// One contract-month priced at one stamp: index close `spot`, option close
/// `premium`, both in paisa.
fn priced_month(spot: i64, premium: i64) -> pull::pricing::PricedAll {
    let found = contract();
    let month_of = month(&found);
    let book = pull::pricing::SpotBook::of(&[index_bar(STAMP, spot)]);
    let mut out = pull::pricing::PricedAll::default();
    let quotes = chain_quotes(&[index_bar(STAMP, premium)], month_of, &book, &mut out);
    assert_eq!(quotes.len(), 1, "the quote is built: {:?}", out.why);
    let done = pull::pricing::price_all(
        &quotes,
        |_| None,
        month_of.inputs.rate,
        pull::tenor::YearBasis::Calendar365,
    );
    assert_eq!(done.refused, 1, "the fixture premium is refused: {done:?}");
    done
}

/// ROUND 3, D-3124: D-3111 made `price_all` keep one sentence per CLASS of
/// refusal, because a sentence carries the row's own numbers and five rows
/// below intrinsic at five premiums filled every slot. The receipt then folds
/// one `PricedAll` per contract-month into `PricedCount` deduplicating by the
/// SENTENCE again, so five contract-months each refused below intrinsic — five
/// sentences differing only in their numbers — fill every slot, and a sixth
/// month refused for a different reason is counted and never named.
#[test]
fn a_receipt_names_a_new_kind_of_refusal_after_many_months_of_one_kind() {
    let mut count = PricedCount::default();
    for step in 0..6 {
        // Deep in the money at ₹25,000 + ₹100 a month, premium ₹1: below the
        // discounted intrinsic value, at a different intrinsic every month.
        count.absorb(&priced_month(2_500_000 + step * 10_000, 100));
    }
    assert_eq!(count.refused, 6);
    assert_eq!(
        count.why.len(),
        1,
        "six months of one kind of refusal are one reason: {:?}",
        count.why
    );
    // A premium above the spot: no volatility reaches it.
    count.absorb(&priced_month(2_500_000, 2_600_000));
    assert_eq!(count.refused, 7);
    assert!(
        count.why.iter().any(|why| why.contains("supremum")),
        "the new kind of refusal is named: {:?}",
        count.why
    );

    // AND THE RUN-LEVEL FOLD (`absorb_count`) KEEPS THE SAME RULE.
    let mut run = PricedCount::default();
    for step in 0..6 {
        let mut one = PricedCount::default();
        one.absorb(&priced_month(2_500_000 + step * 10_000, 100));
        run.absorb_count(&one);
    }
    let mut last = PricedCount::default();
    last.absorb(&priced_month(2_500_000, 2_600_000));
    run.absorb_count(&last);
    assert_eq!(run.why.len(), 2, "{:?}", run.why);
}

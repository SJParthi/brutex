//! BASE-COV-LIBS-LS4: `settle_dhan`'s refusal of a kind the calculator page
//! does not price, reached by a direct call.
//!
//! The calculator day hands its kind straight to `dhan_calc::page_lines`,
//! whose page refuses an ETF and a commodity by its own words. A ledger never
//! gets that far with one (its first order is refused by the same page), so
//! only a direct call with a day that already holds an order reaches the
//! refusal; `Totals` is private to `stack`, so this child module builds it.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]

use super::{Totals, settle_dhan};
use crate::charge_error::ChargeError;
use crate::day::TradeDay;
use crate::error::CostError;
use crate::schedule::{Product, TradeKind};
use brutex_core::instrument::Exchange;
use brutex_core::price::Paisa;

const ETF: ChargeError = ChargeError::NotOnPage {
    what: "an ETF (no calculator reading exists)",
};
const COMMODITY: ChargeError = ChargeError::NotOnPage {
    what: "a commodity (MCX is outside the engine's scope)",
};

fn day() -> TradeDay {
    TradeDay::new(2026, 10, 15).expect("a real date")
}

/// One bought order of ₹100.
fn one_order() -> Totals {
    Totals {
        orders: 1,
        buy_quantity: 1,
        buy_value: Paisa::from_raw(10_000),
        ..Totals::EMPTY
    }
}

/// **A CALCULATOR DAY OF A KIND THE PAGE DOES NOT PRICE IS REFUSED IN THE
/// PAGE'S OWN WORDS**, on both exchanges, and never priced as if it were
/// another kind.
#[test]
fn a_calculator_day_the_page_cannot_price_is_refused_by_the_page() {
    // 1-4. Each refused kind on each exchange.
    for exchange in [Exchange::Nse, Exchange::Bse] {
        assert_eq!(
            settle_dhan(TradeKind::Etf, exchange, day(), &one_order()).err(),
            Some(ETF),
            "{exchange:?}"
        );
        assert_eq!(
            settle_dhan(TradeKind::Commodity, exchange, day(), &one_order()).err(),
            Some(COMMODITY),
            "{exchange:?}"
        );
    }
    // 5. CONTROL: the same day as an intraday equity day prices, so the
    //    refusal above is the kind's and not the day's.
    let priced = settle_dhan(
        TradeKind::Equity(Product::Intraday),
        Exchange::Nse,
        day(),
        &one_order(),
    )
    .expect("an intraday day prices on the page");
    assert_eq!(priced.orders(), 1);

    // 6. OUT OF THE BOX: a day whose one order is worth nothing at all. The
    //    page refuses by kind, never by value.
    let worthless = Totals {
        orders: 1,
        ..Totals::EMPTY
    };
    assert_eq!(
        settle_dhan(TradeKind::Etf, Exchange::Nse, day(), &worthless).err(),
        Some(ETF)
    );
}

/// **THE CALCULATOR DAY'S REFUSALS COME IN ONE ORDER**, whichever kind it is:
/// an exercise first, then an empty day, then a turnover past `i64`, and only
/// then the page. Each pair below is a combination of two extremes at once.
#[test]
fn a_day_refused_twice_over_names_the_earlier_refusal() {
    // 7. COMBINED: an ETF day that also holds an exercise names the exercise.
    let exercised = Totals {
        exercised: Paisa::from_raw(1),
        ..one_order()
    };
    assert_eq!(
        settle_dhan(TradeKind::Etf, Exchange::Nse, day(), &exercised).err(),
        Some(ChargeError::NotOnPage {
            what: "an option exercise (the calculator page has no exercise input)",
        })
    );
    // 8. COMBINED: an empty commodity day is an empty day.
    assert_eq!(
        settle_dhan(TradeKind::Commodity, Exchange::Bse, day(), &Totals::EMPTY).err(),
        Some(ChargeError::EmptyDay)
    );
    // 9. COMBINED: an ETF day whose two sides sum past `i64` names the sum.
    let past = Totals {
        orders: 2,
        buy_quantity: 1,
        buy_value: Paisa::from_raw(i64::MAX),
        sell_quantity: 1,
        sell_value: Paisa::from_raw(1),
        ..Totals::EMPTY
    };
    assert_eq!(
        settle_dhan(TradeKind::Etf, Exchange::Nse, day(), &past).err(),
        Some(ChargeError::Cost(CostError::Overflow {
            operation: "the day's turnover"
        }))
    );
}

//! BASE-COV-LIBS-LS4: the precise day's STT sum past `i64`, reached through a
//! card whose rates no regime ships.
//!
//! `stack::settle_precise` adds a day's trade STT to its exercise STT before
//! rounding both to the rupee, and refuses a sum past `i64` by name. Every
//! shipped options rate is a fraction of a percent, so no shipped card can
//! reach that sum: both taxes stay below 2^54 paisa whatever the day holds. A
//! card's fields are private to `schedule`, so this child module builds one
//! at 100% STT on both lines, the one way to show the refusal holds.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]

use super::{Cited, RateCard, RateSelector, Settlement, Taxes, TradeKind};
use crate::charge_error::ChargeError;
use crate::day::TradeDay;
use crate::error::CostError;
use crate::rate::OrderSide;
use crate::stack::{ClientSecurityDay, Order};
use brutex_core::instrument::Exchange;
use brutex_core::price::Paisa;

/// 2^62 paisa: two of these sum to one past `i64::MAX`.
const HALF: i64 = 1 << 62;

fn options_card() -> RateCard {
    RateCard::resolve(
        TradeKind::Options,
        Exchange::Nse,
        TradeDay::new(2026, 10, 15).expect("a real date"),
        RateSelector::CurrentOnEveryDate,
    )
    .expect("today's options card")
}

/// The shipped options card with every STT line at 100% and no stamp duty.
fn whole_stt_card() -> RateCard {
    RateCard {
        taxes: Taxes::Options {
            stt_sell: Cited::named(1, 1, "a test rate: 100% STT on the premium sold"),
            stt_exercise: Cited::named(1, 1, "a test rate: 100% STT on the exercise"),
            stamp_buy: Cited::named(0, 1, "a test rate: no stamp duty"),
        },
        ..options_card()
    }
}

/// A day on `card` that sold `sold` paisa of premium and exercised `exercised`.
fn day_of(card: &RateCard, sold: i64, exercised: i64) -> ClientSecurityDay {
    let mut day = ClientSecurityDay::with_card(*card);
    if sold > 0 {
        day.add(Order::new(OrderSide::Sell, 1, Paisa::from_raw(sold)).expect("an order"))
            .expect("a sell the day can hold");
    }
    if exercised > 0 {
        day.exercise(Paisa::from_raw(exercised), Settlement::Cash)
            .expect("an exercise the day can hold");
    }
    day
}

const fn overflow(operation: &'static str) -> ChargeError {
    ChargeError::Cost(CostError::Overflow { operation })
}

/// **THE DAY'S STT SUM ONE PAISA PAST `i64` IS REFUSED BY NAME**, never
/// wrapped and never rounded into range.
#[test]
fn a_days_stt_sum_past_i64_is_refused_by_name() {
    // 1. Each STT line alone fits; their sum is 2^63, one past the type.
    assert_eq!(
        day_of(&whole_stt_card(), HALF, HALF).settle().err(),
        Some(overflow("the day's STT before the rupee"))
    );
    // 2. COMBINED: both lines at the far end of the type, each one paisa
    //    short of `i64::MAX`.
    assert_eq!(
        day_of(&whole_stt_card(), i64::MAX - 1, i64::MAX - 1)
            .settle()
            .err(),
        Some(overflow("the day's STT before the rupee"))
    );
    // 3. CONTROL: the same day on the shipped card prices: its STT is a
    //    fraction of a percent of each line.
    let shipped = day_of(&options_card(), HALF, HALF)
        .settle()
        .expect("the shipped card prices the day");
    assert!(shipped.stt_paisa().raw() > 0);
    assert!(shipped.stt_exercise_paisa().raw() > 0);
}

/// Inside the sum: a sale alone and an exercise alone at 100% each price,
/// their STT line the whole amount; and a sum exactly at `i64::MAX` passes
/// the STT line and is refused one step later, by that step's name.
#[test]
fn a_whole_stt_day_inside_the_sum_prices_or_names_a_later_step() {
    // 4. A sale alone at 2^62: the STT line is the sale itself.
    let sold = day_of(&whole_stt_card(), HALF, 0)
        .settle()
        .expect("a sale alone stays inside every sum");
    assert_eq!(sold.stt_paisa().raw(), HALF);
    assert_eq!(sold.stt_exercise_paisa().raw(), 0);
    assert!(sold.total().raw() > HALF, "the fees come on top");
    // 5. An exercise alone at 2^62: the exercise STT is the exercise itself.
    let exercised = day_of(&whole_stt_card(), 0, HALF)
        .settle()
        .expect("an exercise alone stays inside every sum");
    assert_eq!(exercised.stt_exercise_paisa().raw(), HALF);
    assert_eq!(exercised.stt_paisa().raw(), 0);
    // 6. COMBINED: the sum exactly at `i64::MAX` fits the STT line, and the
    //    day's total, which adds the fees on top, is what does not fit.
    assert_eq!(
        day_of(&whole_stt_card(), HALF, HALF - 1).settle().err(),
        Some(overflow("the day's total"))
    );
    // 7. OUT OF THE BOX: an empty day on the crafted card is an empty day,
    //    whatever its rates.
    assert_eq!(
        ClientSecurityDay::with_card(whole_stt_card())
            .settle()
            .err(),
        Some(ChargeError::EmptyDay)
    );
}

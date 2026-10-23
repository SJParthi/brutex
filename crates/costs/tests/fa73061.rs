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

//! NSE CIRCULAR `FA73061`, DRIVEN FROM OUTSIDE THE CRATE (CHG-0, D-5100).
//!
//! `FA73061` (`NSE/FA/73061`, dated 2026-02-27, effective 2026-03-01) sets the
//! equity options transaction charge at ₹3,552.99 and the IPFT contribution at
//! ₹0.01 per crore of premium each side: ₹3,553 in all, the same total as the
//! `FA64232` row (₹3,503) plus its ₹50 IPFT. The crate's rates are one rupee
//! per crore, so the 2026-03-01 rows fold the two: the exchange-charge row is
//! ₹3,553 and the IPFT row zero.
//!
//! What this file proves, through the public API only:
//!
//! 1. A trade dated 2026-02-27 is priced at the old rows, one dated 2026-03-02
//!    at the new ones, and the boundary day 2026-03-01 at the new ones.
//! 2. A trip straddling the date prices each leg at its own day's row.
//! 3. The live Dhan calculator case of 15 Oct 2026 — NIFTY 19 OCT 25300 CALL,
//!    one lot of 65, bought at 100 and sold at 120, premium turnover ₹14,300 —
//!    shows `Exchange 5.08`, and the rate this crate holds for that day
//!    reproduces it, while the 2024-10-01 row would have read 5.01. The case is
//!    a public calculator's output for an invented trade, cited from
//!    `state/charges/dhan-golden-20261015.json` (captured 2026-10-15), not
//!    vendor market data.
//! 4. Why the round-trip calculator's own figure for that trade is 5.09, not
//!    5.08: it ceils each leg to the paisa, on purpose (`docs/06-limits.md`
//!    §27), where the calculator rounds the combined turnover once.
//! 5. Every option case captured from the same calculator on 15 and 16 Oct
//!    2026 (eleven, `dhan-golden-20261015.json` and `-20261016.json`) shows
//!    the exchange line the 2026-03-01 row gives, and the 2024-10-01 row
//!    misses eight of the eleven.
//! 6. The extremes: every month and year boundary around the date and the
//!    2026-04-01 STT change, dates that do not exist, the fold's one-paisa
//!    bound from one paisa to `i64::MAX` with its real under-run at exactly
//!    ₹50 lakh a leg, and fourteen threads asking at once (D-5101).
//!
//! Every price here is an invented test figure.

use std::hint::black_box;
use std::time::Instant;

use brutex_core::instrument::Exchange;
use brutex_core::price::Paisa;
use brutex_core::symbol::Symbol;
use costs::day::TradeDay;
use costs::fill::{Bar, Direction};
use costs::money::levy_ceiling;
use costs::rate::{BpsX100, Broker, NSE_IPFT, RATE_SCALE, SEBI_TURNOVER_FEE};
use costs::regime::{
    NSE_EXCHANGE_CHARGE, NSE_EXCHANGE_CHARGE_FA73061, exchange_charge_rate, ipft_rate,
};
use costs::schedule::{RateCard, RateSelector, TradeKind};
use costs::scope::Segment;
use costs::trip::{Charges, Contract, Leg, Outcome, Rates, RoundTrip, price};
use costs::venue::swept_slot;

fn day(year: u16, month: u8, d: u8) -> TradeDay {
    TradeDay::new(year, month, d).expect("a real date")
}

/// A long NIFTY index-option round trip of `units`, on flat bars, with each
/// leg on its own day.
fn nifty_trip(entry_day: TradeDay, exit_day: TradeDay, entry: i64, exit: i64) -> RoundTrip {
    let slot = swept_slot(Symbol::new("NIFTY").expect("a valid symbol")).expect("swept");
    RoundTrip::new(
        Contract::new(slot, Segment::IndexOption),
        Direction::Long,
        Broker::Groww,
        Outcome::NormalClose,
        Leg::new(
            entry_day,
            Bar::flat(Paisa::from_raw(entry)).expect("a legal bar"),
        ),
        Leg::new(
            exit_day,
            Bar::flat(Paisa::from_raw(exit)).expect("a legal bar"),
        ),
        65,
    )
    .expect("a well-formed round trip")
}

fn priced(entry_day: TradeDay, exit_day: TradeDay) -> Charges {
    // ₹100.00 -> ₹120.00; the worst-case fill law moves each a tick, so the
    // notionals are 650,325 and 779,675 paisa.
    price(&nifty_trip(entry_day, exit_day, 10_000, 12_000)).expect("a verified regime")
}

/// `amount × rate / RATE_SCALE`, rounded half-up to the paisa: the one
/// rounding a public calculator applies to a whole line. Test arithmetic
/// only; the crate itself ceils every leg (`docs/06-limits.md` §27).
fn half_up(amount: Paisa, rate_units: i64, scale: i64) -> i64 {
    let product = i128::from(amount.raw()) * i128::from(rate_units);
    let half = i128::from(scale) / 2;
    i64::try_from((product + half) / i128::from(scale)).expect("fits")
}

#[test]
fn a_trade_dated_2026_02_27_is_priced_at_the_fa64232_row_and_its_50_rupee_ipft() {
    let on = day(2026, 2, 27);
    let rates = Rates::resolve(Broker::Groww, Exchange::Nse, on).expect("verified");
    assert_eq!(rates.exchange(), NSE_EXCHANGE_CHARGE);
    assert_eq!(rates.exchange().get(), 3_503);
    assert_eq!(rates.ipft(), NSE_IPFT);
    assert_eq!(rates.ipft().get(), 50);

    let charges = priced(on, on);
    assert_eq!(charges.buy_notional().raw(), 650_325);
    assert_eq!(charges.sell_notional().raw(), 779_675);
    // 228 + 274 at ₹3,503 per crore, ceiled per leg; IPFT 4 + 4 at ₹50.
    assert_eq!(charges.exchange().raw(), 502);
    assert_eq!(charges.ipft().raw(), 8);
}

#[test]
fn a_trade_dated_2026_03_02_is_priced_at_the_fa73061_row() {
    let on = day(2026, 3, 2);
    let rates = Rates::resolve(Broker::Groww, Exchange::Nse, on).expect("verified");
    assert_eq!(rates.exchange(), NSE_EXCHANGE_CHARGE_FA73061);
    assert_eq!(rates.exchange().get(), 3_553);
    assert_eq!(rates.ipft().get(), 0, "folded into the exchange row");

    let charges = priced(on, on);
    // 232 + 278 at ₹3,553 per crore, ceiled per leg; the IPFT line is zero.
    assert_eq!(charges.exchange().raw(), 510);
    assert_eq!(charges.ipft().raw(), 0);
    // The same 510 paisa as the old split's 502 + 8 on these notionals.
    let before = priced(day(2026, 2, 27), day(2026, 2, 27));
    assert_eq!(
        charges.exchange().raw() + charges.ipft().raw(),
        before.exchange().raw() + before.ipft().raw()
    );
}

#[test]
fn the_boundary_day_2026_03_01_is_priced_at_the_fa73061_row() {
    // Inclusive, like every boundary in the crate: the circular's own date is
    // the first day of the new row, not the last day of the old one.
    let on = day(2026, 3, 1);
    let rates = Rates::resolve(Broker::Groww, Exchange::Nse, on).expect("verified");
    assert_eq!(rates.exchange().get(), 3_553);
    assert_eq!(rates.ipft().get(), 0);
    let charges = priced(on, on);
    assert_eq!(charges.exchange().raw(), 510);
    assert_eq!(charges.ipft().raw(), 0);
    // And the day before it is the old row.
    let eve = Rates::resolve(Broker::Groww, Exchange::Nse, day(2026, 2, 28)).expect("verified");
    assert_eq!((eve.exchange().get(), eve.ipft().get()), (3_503, 50));
}

#[test]
fn a_trip_straddling_2026_03_01_prices_each_leg_at_its_own_days_row() {
    // Bought on 2026-02-27 under FA64232, sold on 2026-03-02 under FA73061.
    let charges = priced(day(2026, 2, 27), day(2026, 3, 2));
    let old = NSE_EXCHANGE_CHARGE;
    let new = NSE_EXCHANGE_CHARGE_FA73061;
    let buy_leg = levy_ceiling(charges.buy_notional(), old).expect("in range");
    let sell_leg = levy_ceiling(charges.sell_notional(), new).expect("in range");
    assert_eq!((buy_leg.raw(), sell_leg.raw()), (228, 278));
    assert_eq!(charges.exchange().raw(), 506);
    // The buy leg's ₹50 IPFT, ceiled: 4 paisa. The sell leg's is folded.
    assert_eq!(
        levy_ceiling(charges.buy_notional(), NSE_IPFT)
            .expect("in range")
            .raw(),
        4
    );
    assert_eq!(charges.ipft().raw(), 4);
    // Neither leg was priced at the other leg's day: a whole-trip entry-day
    // rate would give 502 + 8 and an exit-day one 510 + 0.
    assert_ne!(charges.exchange().raw(), 502);
    assert_ne!(charges.exchange().raw(), 510);
}

#[test]
fn the_live_dhan_nifty_25300_call_reproduces_exchange_5_08() {
    // `state/charges/dhan-golden-20261015.json`, case "UI OPT NIFTY 19 OCT 25300
    // CALL 1 lot (65) 100->120 (live page screenshot)", captured 2026-10-15:
    // EXCHANGE_TURNOVER 14300, EXCHANGE_CHARGES_SHOWN 5.08, IPFT_SHOWN 0,
    // SEBI_SHOWN 0.01. The calculator uses the prices as typed (no tick of
    // slippage) and prices the trade at the rates in force that day.
    let on = day(2026, 10, 15);
    let exchange = exchange_charge_rate(Exchange::Nse, on).expect("verified");
    let ipft = ipft_rate(Exchange::Nse, on).expect("verified");
    let buy = Paisa::from_raw(65 * 10_000);
    let sell = Paisa::from_raw(65 * 12_000);
    let turnover = Paisa::from_raw(buy.raw() + sell.raw());
    assert_eq!(turnover.raw(), 1_430_000, "₹14,300.00");

    // The line the calculator shows: the rate on the combined turnover,
    // rounded once to the paisa.
    let exchange_line = half_up(turnover, exchange.get(), RATE_SCALE);
    let ipft_line = half_up(turnover, ipft.get(), RATE_SCALE);
    assert_eq!(exchange_line, 508, "Exchange 5.08");
    assert_eq!(ipft_line, 0, "IPFT 0.00");
    assert_eq!(
        half_up(turnover, SEBI_TURNOVER_FEE.get(), RATE_SCALE),
        1,
        "SEBI 0.01"
    );

    // The circular's own split, each line rounded on its own, at a scale fine
    // enough to hold ₹3,552.99 and ₹0.01 (hundredths of a rupee per crore):
    // 5.08 + 0.00. The folded row gives the same figure on this trade.
    let fine_scale = RATE_SCALE * 100;
    assert_eq!(half_up(turnover, 355_299, fine_scale), 508);
    assert_eq!(half_up(turnover, 1, fine_scale), 0);

    // Before D-5100 the crate held the 2024-10-01 row for this day, whose
    // exchange line reads 5.01 (and 0.07 of IPFT beside it). That is the
    // defect NEXT-PROMPT step 0b5 saw on the live page.
    assert_eq!(
        half_up(turnover, NSE_EXCHANGE_CHARGE.get(), RATE_SCALE),
        501
    );
    assert_eq!(half_up(turnover, NSE_IPFT.get(), RATE_SCALE), 7);

    // The round-trip calculator's own figure is 5.09, and the existing rounding
    // cannot give 5.08: it ceils EACH LEG to the paisa (adverse by design,
    // `docs/06-limits.md` §27): ₹6,500 × 0.03553% = 2.309 -> 2.31 and
    // ₹7,800 × 0.03553% = 2.771 -> 2.78, so 5.09, where one half-up rounding
    // of ₹14,300 × 0.03553% = 5.0808 gives 5.08.
    let per_leg = levy_ceiling(buy, exchange).expect("in range").raw()
        + levy_ceiling(sell, exchange).expect("in range").raw();
    assert_eq!(per_leg, 509);
    assert_eq!(levy_ceiling(buy, exchange).expect("in range").raw(), 231);
    assert_eq!(levy_ceiling(sell, exchange).expect("in range").raw(), 278);
    assert!(
        per_leg >= exchange_line,
        "the ceiling is never below the calculator"
    );
    assert!(
        per_leg - exchange_line <= 2,
        "and at most a paisa a leg above it"
    );
}

/// Every option case captured live from Dhan's public calculator, as
/// `(capture day, premium turnover in paisa, Exchange line in paisa)`. The
/// IPFT line read 0.00 on every one. Only the two figures the exchange line
/// turns on are copied, never the typed prices or the scrip:
///
/// * 2026-10-15: `state/charges/dhan-golden-20261015.json`, the one `OPT`
///   case (NIFTY 19 OCT 25300 CALL, 1 lot, 100 -> 120);
/// * 2026-10-16: `state/charges/dhan-golden-20261016.json`, the ten cases whose
///   `tab` is the options tab, in file order (NIFTY, BANKNIFTY and RELIANCE
///   calls and puts, premiums from ₹0.05 to ₹1,236.50).
///
/// Public calculator outputs for invented test trades, cited by file and
/// date; not vendor market data.
fn dhan_option_cases() -> [(TradeDay, i64, i64); 11] {
    let oct_15 = day(2026, 10, 15);
    let oct_16 = day(2026, 10, 16);
    [
        (oct_15, 1_430_000, 508),
        (oct_16, 1_270_750, 451),
        (oct_16, 15_869_750, 5_639),
        (oct_16, 33_800, 12),
        (oct_16, 3_391_350, 1_205),
        (oct_16, 8_011_350, 2_846),
        (oct_16, 2_785_000, 990),
        (oct_16, 1_095_000, 389),
        (oct_16, 82_500, 29),
        (oct_16, 13_000, 5),
        (oct_16, 2_220_000, 789),
    ]
}

#[test]
fn every_captured_dhan_option_case_shows_the_exchange_line_of_the_fa73061_row() {
    let mut at_old_row = 0u32;
    for (on, turnover, shown) in dhan_option_cases() {
        let turnover = Paisa::from_raw(turnover);
        let exchange = exchange_charge_rate(Exchange::Nse, on).expect("verified");
        let ipft = ipft_rate(Exchange::Nse, on).expect("verified");
        assert_eq!(
            exchange, NSE_EXCHANGE_CHARGE_FA73061,
            "the row in force on {on}"
        );
        assert_eq!(
            half_up(turnover, exchange.get(), RATE_SCALE),
            shown,
            "Exchange line on a {turnover:?} turnover, {on}"
        );
        assert_eq!(half_up(turnover, ipft.get(), RATE_SCALE), 0, "IPFT 0.00");
        // The circular's own split, each line rounded alone, at a scale that
        // holds ₹3,552.99 and ₹0.01: the same figures on every case.
        let fine_scale = RATE_SCALE * 100;
        assert_eq!(half_up(turnover, 355_299, fine_scale), shown);
        assert_eq!(half_up(turnover, 1, fine_scale), 0);
        if half_up(turnover, NSE_EXCHANGE_CHARGE.get(), RATE_SCALE) == shown {
            at_old_row += 1;
        }
    }
    // The 2024-10-01 row reproduces only the three cases whose turnover is
    // under ₹1,000 (₹130, ₹338, ₹825): the missing ₹50 per crore is under
    // half a paisa there and both rates round to the same paisa. On the other
    // eight it reads 1.3% to 1.4% low: the defect, on the calculator's own
    // figures.
    assert_eq!(at_old_row, 3);
}

#[test]
fn bse_is_not_moved_by_an_nse_circular() {
    // No BSE circular is recorded for 2026-03-01, so both BSE rows hold.
    for on in [day(2026, 2, 27), day(2026, 3, 1), day(2026, 3, 2)] {
        let rates = Rates::resolve(Broker::Groww, Exchange::Bse, on).expect("verified");
        assert_eq!(rates.exchange().get(), 3_250);
        assert_eq!(rates.ipft().get(), 0);
    }
}

#[test]
fn every_boundary_around_fa73061_prices_its_own_rows_and_nothing_else_moves() {
    // Month and year boundaries on both sides of the circular, the two
    // boundaries before it, the Finance Act 2026 STT change a month after it,
    // and the last representable day. `None` is a refusal.
    let rows: [(TradeDay, Option<i64>, Option<i64>); 13] = [
        (day(2023, 3, 31), None, None),
        (day(2023, 4, 1), None, Some(50)),
        (day(2024, 9, 30), None, Some(50)),
        (day(2024, 10, 1), Some(3_503), Some(50)),
        (day(2025, 12, 31), Some(3_503), Some(50)),
        (day(2026, 1, 1), Some(3_503), Some(50)),
        (day(2026, 1, 31), Some(3_503), Some(50)),
        (day(2026, 2, 1), Some(3_503), Some(50)),
        (day(2026, 2, 28), Some(3_503), Some(50)),
        (day(2026, 3, 1), Some(3_553), Some(0)),
        (day(2026, 3, 31), Some(3_553), Some(0)),
        (day(2026, 4, 1), Some(3_553), Some(0)),
        (TradeDay::MAX, Some(3_553), Some(0)),
    ];
    for (on, exchange, ipft) in rows {
        let got_exchange = exchange_charge_rate(Exchange::Nse, on)
            .ok()
            .map(BpsX100::get);
        let got_ipft = ipft_rate(Exchange::Nse, on).ok().map(BpsX100::get);
        assert_eq!((got_exchange, got_ipft), (exchange, ipft), "NSE on {on}");
        // A round trip on the day prices exactly when the exchange row does.
        assert_eq!(
            Rates::resolve(Broker::Groww, Exchange::Nse, on).is_ok(),
            exchange.is_some(),
            "a round trip on {on}"
        );
        // The dated charge card (CLOUD-CHG1's stack) prices on the same days
        // and carries the same outflow, read from the same two rows (D-5101):
        // the exchange row plus the dated IPFT, never the undated ₹50.
        let card = RateCard::resolve(TradeKind::Options, Exchange::Nse, on, RateSelector::Dated);
        assert_eq!(card.is_ok(), exchange.is_some(), "the dated card on {on}");
        if let (Ok(card), Some(exchange), Some(ipft)) = (card, exchange, ipft) {
            let rate = card.exchange_charge().rate();
            assert_eq!(
                i128::from(rate.numerator()) * i128::from(RATE_SCALE),
                i128::from(exchange + ipft) * i128::from(rate.denominator()),
                "the dated card on {on}"
            );
            assert_eq!(exchange + ipft, 3_553, "the NSE outflow on {on}");
        }
    }
    // The STT moves on 2026-04-01 and the exchange charge does not: the two
    // tables are dated independently, and FA73061 is not the STT's boundary.
    let march = Rates::resolve(Broker::Groww, Exchange::Nse, day(2026, 3, 31)).expect("verified");
    let april = Rates::resolve(Broker::Groww, Exchange::Nse, day(2026, 4, 1)).expect("verified");
    assert_eq!(march.exchange(), april.exchange());
    assert_eq!(march.ipft(), april.ipft());
    assert_ne!(march.stt(), april.stt());
}

#[test]
fn a_date_that_does_not_exist_cannot_reach_either_row() {
    // 2026 is not a leap year, so the day before the circular's date is
    // 2026-02-28 and "2026-02-29" is refused before any rate is looked up.
    // The same for the other garbage a caller might hand the constructor.
    for (year, month, d) in [
        (2026, 2, 29),
        (2026, 2, 30),
        (2026, 3, 0),
        (2026, 0, 1),
        (2026, 13, 1),
        (2026, 3, 32),
        (0, 3, 1),
        (u16::MAX, 3, 1),
    ] {
        assert!(
            TradeDay::new(year, month, d).is_err(),
            "{year}-{month}-{d} must be refused"
        );
    }
    // The real neighbours of the boundary are exactly one day apart.
    assert_eq!(day(2026, 3, 1).ordinal() - day(2026, 2, 28).ordinal(), 1);
    assert_eq!(
        TradeDay::from_ordinal(day(2026, 3, 1).ordinal() - 1),
        Ok(day(2026, 2, 28))
    );
}

/// `amount × units / scale`, rounded up to the paisa.
fn ceil_line(amount: i64, units: i64, scale: i64) -> i64 {
    let product = i128::from(amount) * i128::from(units);
    let scale = i128::from(scale);
    let quotient = product / scale;
    let up = i128::from(product % scale != 0);
    i64::try_from(quotient + up).expect("fits")
}

/// The fold's cost per leg, against the circular's two lines kept apart at a
/// scale fine enough to hold ₹3,552.99 and ₹0.01: `(folded ceiling, the two
/// lines each ceiled, the two lines each rounded half-up)`, in paisa.
fn fold_against_split(notional: i64) -> (i64, i64, i64) {
    let fine_scale = RATE_SCALE * 100;
    let folded = levy_ceiling(Paisa::from_raw(notional), NSE_EXCHANGE_CHARGE_FA73061)
        .expect("in range")
        .raw();
    let ceiled = ceil_line(notional, 355_299, fine_scale) + ceil_line(notional, 1, fine_scale);
    let rounded = half_up(Paisa::from_raw(notional), 355_299, fine_scale)
        + half_up(Paisa::from_raw(notional), 1, fine_scale);
    (folded, ceiled, rounded)
}

#[test]
fn the_fold_costs_at_most_one_paisa_a_leg_and_under_50_lakh_is_never_below_the_split() {
    // `docs/06-limits.md` §25 states the bound; this is it, on one paisa, on
    // every notional up to ₹20, on a thousand paisa either side of ₹50 lakh
    // (where the ₹0.01 per crore reaches half a paisa), on 200,000 notionals
    // spread over the whole range by a fixed generator, and on the largest
    // notional the levy accepts.
    const FIFTY_LAKH: i64 = 5_000_000 * 100;
    let mut notionals: Vec<i64> = (1..=2_000).collect();
    notionals.extend(FIFTY_LAKH - 1_000..=FIFTY_LAKH + 1_000);
    let mut state: u64 = 0x5DEE_CE66_D000_0001;
    for _ in 0..200_000 {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        // Up to 2^44 paisa, about ₹17,600 crore a leg: far past any premium.
        notionals.push(i64::try_from(state >> 20).expect("fits") + 1);
    }
    notionals.push(i64::MAX);

    let mut below_split = 0u32;
    let mut lowest_below = i64::MAX;
    for notional in notionals {
        let (folded, ceiled, rounded) = fold_against_split(notional);
        // Against the two lines each ceiled: never above, at most a paisa below.
        assert!(folded <= ceiled, "{notional}: {folded} > {ceiled}");
        assert!(ceiled - folded <= 1, "{notional}: {ceiled} - {folded}");
        // Against the two lines each rounded half-up: at most a paisa below,
        // and never below while the leg is under ₹50 lakh.
        assert!(rounded - folded <= 1, "{notional}: {rounded} - {folded}");
        if folded < rounded {
            assert!(notional >= FIFTY_LAKH, "{notional} is under ₹50 lakh");
            below_split += 1;
            lowest_below = lowest_below.min(notional);
        }
    }
    // The under-run is real, not a theoretical bound: exactly ₹50 lakh puts
    // both of the circular's lines on a half paisa (177,649.5 and 0.5), so they
    // round up to 177,651 where the folded line is the exact 177,650.
    assert_eq!(fold_against_split(FIFTY_LAKH), (177_650, 177_651, 177_651));
    assert_eq!(lowest_below, FIFTY_LAKH);
    assert!(below_split >= 1);
}

#[test]
fn the_dated_rows_answer_the_same_from_fourteen_threads_at_once() {
    // The lookups are pure and the tables are `const`, so concurrent callers
    // cannot disagree; checked, not assumed, across the whole boundary
    // window from 2024-09-25 to 2026-04-10 on every one of 14 threads.
    let first = day(2024, 9, 25).ordinal();
    let last = day(2026, 4, 10).ordinal();
    let answer = |ordinal: i32| {
        let on = TradeDay::from_ordinal(ordinal).expect("in the window");
        (
            exchange_charge_rate(Exchange::Nse, on).ok(),
            ipft_rate(Exchange::Nse, on).ok(),
            Rates::resolve(Broker::Groww, Exchange::Nse, on).ok(),
        )
    };
    let expected: Vec<_> = (first..=last).map(answer).collect();
    assert_eq!(expected.len(), 563);
    std::thread::scope(|scope| {
        let handles: Vec<_> = (0..14)
            .map(|_| scope.spawn(|| (first..=last).map(answer).collect::<Vec<_>>()))
            .collect();
        for handle in handles {
            assert_eq!(
                handle.join().expect("a thread that did not panic"),
                expected
            );
        }
    });
    // And the window holds all three regimes.
    let exchange_rates: std::collections::BTreeSet<_> = expected
        .iter()
        .map(|(exchange, _, _)| exchange.map(BpsX100::get))
        .collect();
    assert_eq!(
        exchange_rates,
        std::collections::BTreeSet::from([None, Some(3_503), Some(3_553)])
    );
}

/// p50, p99 and max of a per-call cost, from `batches` timed batches of
/// `per_batch` calls each. Instants on this machine tick in tens of
/// nanoseconds, and one lookup is a few, so single calls cannot be timed.
fn percentiles(mut per_call_ns: Vec<u128>) -> (u128, u128, u128) {
    per_call_ns.sort_unstable();
    let at = |q: usize| {
        let index = (per_call_ns.len() - 1) * q / 100;
        per_call_ns.get(index).copied().unwrap_or(u128::MAX)
    };
    (
        at(50),
        at(99),
        per_call_ns.last().copied().unwrap_or(u128::MAX),
    )
}

fn time_per_call<T>(
    batches: usize,
    per_batch: u32,
    mut op: impl FnMut() -> T,
) -> (u128, u128, u128) {
    let mut samples = Vec::with_capacity(batches);
    for _ in 0..batches {
        let start = Instant::now();
        for _ in 0..per_batch {
            black_box(op());
        }
        samples.push(start.elapsed().as_nanos() / u128::from(per_batch));
    }
    percentiles(samples)
}

/// MEASUREMENT, not a gate: `cargo test --release -p costs --test fa73061 --
/// --ignored --nocapture`. Prints p50, p99 and max per call for the dated
/// lookups this change touches, on each row of the NSE tables, and fails if
/// any answer is wrong or a p99 passes a deliberately loose 10 µs ceiling.
#[test]
#[ignore = "a timing measurement; run in release with --ignored --nocapture"]
fn the_dated_lookups_cost_the_same_on_every_row_measured() {
    const BATCHES: usize = 2_000;
    const PER_BATCH: u32 = 1_000;
    let days = [
        ("2020-06-01 (refusal)", day(2020, 6, 1)),
        ("2024-10-01 (FA64232)", day(2024, 10, 1)),
        ("2026-02-27 (FA64232)", day(2026, 2, 27)),
        ("2026-03-01 (FA73061)", day(2026, 3, 1)),
        ("2100-12-31 (FA73061)", TradeDay::MAX),
    ];
    for (label, on) in days {
        let exchange = exchange_charge_rate(Exchange::Nse, on);
        let ipft = ipft_rate(Exchange::Nse, on);
        let (p50, p99, max) = time_per_call(BATCHES, PER_BATCH, || {
            exchange_charge_rate(black_box(Exchange::Nse), black_box(on))
        });
        println!(
            "exchange_charge_rate {label}: p50 {p50} ns  p99 {p99} ns  max {max} ns  (n = {BATCHES} x {PER_BATCH})"
        );
        assert!(p99 < 10_000);
        let (p50, p99, max) = time_per_call(BATCHES, PER_BATCH, || {
            ipft_rate(black_box(Exchange::Nse), black_box(on))
        });
        println!(
            "ipft_rate            {label}: p50 {p50} ns  p99 {p99} ns  max {max} ns  (n = {BATCHES} x {PER_BATCH})"
        );
        assert!(p99 < 10_000);
        let (p50, p99, max) = time_per_call(BATCHES, PER_BATCH, || {
            Rates::resolve(
                black_box(Broker::Groww),
                black_box(Exchange::Nse),
                black_box(on),
            )
        });
        println!(
            "Rates::resolve       {label}: p50 {p50} ns  p99 {p99} ns  max {max} ns  (n = {BATCHES} x {PER_BATCH})"
        );
        assert!(p99 < 10_000);
        // The dated charge card, which reads the same two functions since
        // D-5101.
        let card = RateCard::resolve(TradeKind::Options, Exchange::Nse, on, RateSelector::Dated);
        let (p50, p99, max) = time_per_call(BATCHES, PER_BATCH, || {
            RateCard::resolve(
                black_box(TradeKind::Options),
                black_box(Exchange::Nse),
                black_box(on),
                black_box(RateSelector::Dated),
            )
        });
        println!(
            "RateCard::resolve    {label}: p50 {p50} ns  p99 {p99} ns  max {max} ns  (n = {BATCHES} x {PER_BATCH})"
        );
        assert!(p99 < 10_000);
        assert_eq!(
            RateCard::resolve(TradeKind::Options, Exchange::Nse, on, RateSelector::Dated),
            card
        );
        // The answers are the right ones while being timed.
        assert_eq!(exchange_charge_rate(Exchange::Nse, on), exchange);
        assert_eq!(ipft_rate(Exchange::Nse, on), ipft);
    }
    let on = day(2026, 3, 2);
    let trip = nifty_trip(on, on, 10_000, 12_000);
    let (p50, p99, max) = time_per_call(BATCHES, PER_BATCH, || price(black_box(&trip)));
    println!(
        "price (2026-03-02, FA73061): p50 {p50} ns  p99 {p99} ns  max {max} ns  (n = {BATCHES} x {PER_BATCH})"
    );
    assert!(p99 < 10_000);
    assert_eq!(price(&trip).expect("verified").exchange().raw(), 510);
}

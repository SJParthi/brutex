//! Adversarial attacks on `pull::pricing`, `pull::tenor` and the spot join.
//!
//! Every property loop here is driven by a fixed-seed splitmix64, so a rerun
//! offers byte-identical inputs (`CLAUDE.md` §3 rule 5). Every test asserts a
//! named outcome: a value, a refusal by name, or an invariant that held.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::float_arithmetic,
    clippy::float_cmp,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    missing_docs,
    reason = "a test that cannot panic cannot fail, and a test that measures \
              a float has to do float arithmetic to measure it"
)]

use std::time::Instant;

use brutex_core::instrument::{Expiry, OptionSide};
use brutex_core::vendor::Vendor;
use greeks::error::GreeksError;
use pull::pricing::{
    PricedAll, PricingError, Quote, REASONS_KEPT, Rate, RateSource, SpotBook, VolSource, price,
    price_all, solve_iv,
};
use pull::session::Day;
use pull::tenor::{Tenor, TenorError, YearBasis};

const MINUTE: i64 = 60 * 1_000_000;

/// splitmix64, fixed seed: the same stream on every run.
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

/// Microseconds at an IST wall-clock moment.
fn ist(year: u16, month: u8, day: u8, hour: i64, minute: i64) -> i64 {
    let d = Day::new(year, month, day).expect("a real day");
    (i64::from(d.days_from_epoch()) * 86_400 - 19_800 + hour * 3_600 + minute * 60) * 1_000_000
}

fn bar(ts_micros: i64, close: i64) -> store::format::Bar {
    store::format::Bar {
        ts_micros,
        open: close,
        high: close,
        low: close,
        close,
        volume: 0,
        open_interest: 0,
    }
}

fn rate() -> Rate {
    Rate::measured(0.065, YearBasis::Calendar365, RateSource::Operator).expect("plausible")
}

fn expiry() -> Expiry {
    Expiry::new(2026, 8, 27).expect("a real expiry")
}

fn quote_at(ts_micros: i64, spot: i64, strike: i64, premium: i64, side: OptionSide) -> Quote {
    let symbol = brutex_core::symbol::Symbol::new("NIFTY").expect("swept");
    Quote {
        ts_micros,
        spot,
        strike,
        premium,
        tenor: Tenor::between(ts_micros, expiry()).expect("a live tenor"),
        side,
        slot: pull::pricing::slot_of(symbol).expect("NIFTY has a regime"),
        on: costs::day::TradeDay::new(2026, 8, 20).expect("a trade day"),
        vendor: Vendor::Groww,
    }
}

fn atm_call() -> Quote {
    quote_at(
        ist(2026, 8, 20, 9, 15),
        2_500_000,
        2_500_000,
        25_000,
        OptionSide::Call,
    )
}

// ---------------------------------------------------------------------------
// SpotBook
// ---------------------------------------------------------------------------

/// Exact stamp or nothing, at every sub-minute offset that could be confused
/// with the minute: one microsecond, one millisecond, the same second at a
/// different microsecond, thirty seconds, one minute less one microsecond.
#[test]
fn dpp_spot_book_never_answers_a_sub_minute_neighbour() {
    let base = ist(2026, 8, 20, 9, 15);
    let bars: Vec<_> = (0..375)
        .map(|i| bar(base + i * MINUTE, 2_500_000 + i))
        .collect();
    let book = SpotBook::of(&bars);
    assert_eq!(book.len(), 375);
    let mut tried = 0_u32;
    for i in 0..375 {
        let at = base + i * MINUTE;
        assert_eq!(book.at(at), Some(2_500_000 + i), "the exact stamp answers");
        for off in [
            1,
            -1,
            999,
            1_000,
            999_999,
            1_000_000,
            -1_000_000,
            30_000_000,
            MINUTE - 1,
            -(MINUTE - 1),
        ] {
            tried += 1;
            assert_eq!(book.at(at + off), None, "{off} µs off minute {i} answered");
        }
    }
    assert_eq!(tried, 3_750);
    // The extremes of the key space are refused, not wrapped onto a stamp.
    for probe in [i64::MIN, i64::MAX, 0, -1, 1] {
        assert_eq!(book.at(probe), None, "{probe}");
    }
}

/// Order of arrival is irrelevant for distinct stamps: 1,000 fixed-seed
/// shuffles of one month answer identically to the sorted month.
#[test]
fn dpp_spot_book_is_order_independent_for_distinct_stamps() {
    let base = ist(2026, 8, 20, 9, 15);
    let sorted: Vec<_> = (0..375)
        .map(|i| bar(base + i * MINUTE, 2_400_000 + 7 * i))
        .collect();
    let reference = SpotBook::of(&sorted);
    let mut mix = Mix(0x5EED_0001);
    for _ in 0..1_000 {
        let mut shuffled = sorted.clone();
        for i in (1..shuffled.len()).rev() {
            let j = mix.below(i as u64 + 1) as usize;
            shuffled.swap(i, j);
        }
        let book = SpotBook::of(&shuffled);
        assert_eq!(book.len(), reference.len());
        for b in &sorted {
            assert_eq!(book.at(b.ts_micros), reference.at(b.ts_micros));
        }
    }
}

/// **TWO DIFFERENT SPOTS AT ONE STAMP IS NOT A SPOT.**
///
/// On the unmodified code the later bar silently replaced the earlier one, so
/// the book answered a definite level for a stamp whose level it could not
/// know — the second of two contradicting witnesses, chosen by arrival order.
/// The stamp must answer nothing, and the book must be able to say how many
/// stamps it declined for this reason.
#[test]
fn dpp_spot_book_refuses_a_stamp_with_two_disagreeing_closes() {
    let at = ist(2026, 8, 20, 9, 15);
    let bars = [
        bar(at, 2_500_000),
        bar(at + MINUTE, 2_501_000),
        bar(at, 2_600_000),
    ];
    let book = SpotBook::of(&bars);
    assert_eq!(book.at(at), None, "a contradicted stamp answered a level");
    assert_eq!(book.ambiguous(), 1, "the contradicted stamp is counted");
    assert_eq!(
        book.lookup(at),
        Err(PricingError::SpotAmbiguous { ts_micros: at })
    );
    assert!(
        book.lookup(at)
            .unwrap_err()
            .to_string()
            .contains("disagree"),
        "the refusal says why"
    );
    assert_eq!(
        book.at(at + MINUTE),
        Some(2_501_000),
        "its neighbour is fine"
    );
    assert_eq!(book.lookup(at + MINUTE), Ok(2_501_000));
    assert_eq!(
        book.lookup(at + 2 * MINUTE),
        Err(PricingError::NoSpotAtStamp {
            ts_micros: at + 2 * MINUTE
        })
    );

    // A third copy agreeing with neither, or with one, does not rehabilitate it.
    let three = [bar(at, 2_500_000), bar(at, 2_600_000), bar(at, 2_500_000)];
    assert_eq!(SpotBook::of(&three).at(at), None);
    assert_eq!(SpotBook::of(&three).ambiguous(), 1);

    // An EXACT repeat is one witness said twice, and it is not ambiguous.
    let same = [bar(at, 2_500_000), bar(at, 2_500_000)];
    assert_eq!(SpotBook::of(&same).at(at), Some(2_500_000));
    assert_eq!(SpotBook::of(&same).ambiguous(), 0);

    // Arrival order does not change the verdict.
    let reversed = [bar(at, 2_600_000), bar(at, 2_500_000)];
    assert_eq!(SpotBook::of(&reversed).at(at), None);
}

/// Duplicates by property: 20,000 fixed-seed months with random repeats. A
/// stamp answers iff every copy of it carried one close, and then it answers
/// that close; `len + ambiguous` is the number of distinct stamps.
#[test]
fn dpp_spot_book_duplicates_property() {
    let base = ist(2026, 8, 20, 9, 15);
    let mut mix = Mix(0x5EED_0002);
    for _ in 0..20_000 {
        let n = 1 + mix.below(12) as usize;
        let bars: Vec<_> = (0..n)
            .map(|_| bar(base + mix.below(4) as i64 * MINUTE, 1 + mix.below(3) as i64))
            .collect();
        let book = SpotBook::of(&bars);
        let mut distinct = 0;
        for k in 0..4 {
            let ts = base + k * MINUTE;
            let mut closes = Vec::new();
            for b in &bars {
                if b.ts_micros == ts && !closes.contains(&b.close) {
                    closes.push(b.close);
                }
            }
            if !closes.is_empty() {
                distinct += 1;
            }
            let expected = if closes.len() == 1 {
                Some(closes[0])
            } else {
                None
            };
            assert_eq!(book.at(ts), expected, "{bars:?}");
        }
        assert_eq!(book.len() + book.ambiguous(), distinct);
    }
}

/// A zero or negative close is answered as stored and refused by name one step
/// later, as the spot of the quote; `i64` extremes neither panic nor wrap.
#[test]
fn dpp_spot_book_extreme_closes_are_refused_downstream_by_name() {
    let at = ist(2026, 8, 20, 9, 15);
    for close in [0, -1, i64::MIN] {
        let book = SpotBook::of(&[bar(at, close)]);
        let spot = book.at(at).expect("stored as is");
        let q = Quote { spot, ..atm_call() };
        assert!(
            matches!(
                price(q, None, rate(), YearBasis::Calendar365),
                Err(PricingError::NotPositive { field: "spot", .. })
            ),
            "close {close}"
        );
    }
    let book = SpotBook::of(&[bar(i64::MIN, 1), bar(i64::MAX, 2)]);
    assert_eq!(book.at(i64::MIN), Some(1));
    assert_eq!(book.at(i64::MAX), Some(2));
    assert_eq!(book.at(0), None);
    let empty = SpotBook::of(&[]);
    assert!(empty.is_empty());
    assert_eq!(empty.ambiguous(), 0);
    assert_eq!(
        empty.lookup(at),
        Err(PricingError::NoSpotAtStamp { ts_micros: at })
    );
}

fn percentile(sorted: &[u128], p: usize) -> u128 {
    sorted[(sorted.len() - 1) * p / 100]
}

/// **MEASURED**, not argued: one `SpotBook::at` probe timed alone, 20,000
/// probes (half hits, half one-microsecond misses) at each book size. Asserts
/// every answer; prints p50/p99/max for the report.
#[test]
fn dpp_spot_book_probe_cost_is_measured_at_four_sizes() {
    let base = ist(2026, 8, 3, 9, 15);
    let mut mix = Mix(0x5EED_0003);
    for n in [1_000_i64, 10_000, 100_000, 1_000_000] {
        let bars: Vec<_> = (0..n).map(|i| bar(base + i * MINUTE, 1 + i)).collect();
        let book = SpotBook::of(&bars);
        assert_eq!(book.len(), n as usize);
        let mut samples = Vec::with_capacity(20_000);
        for k in 0..20_000 {
            let i = mix.below(n as u64) as i64;
            let probe = base + i * MINUTE + i64::from(k % 2 == 1);
            let t = Instant::now();
            let got = std::hint::black_box(book.at(std::hint::black_box(probe)));
            samples.push(t.elapsed().as_nanos());
            let want = if k % 2 == 1 { None } else { Some(1 + i) };
            assert_eq!(got, want);
        }
        samples.sort_unstable();
        println!(
            "SpotBook::at n={n}: p50={}ns p99={}ns max={}ns",
            percentile(&samples, 50),
            percentile(&samples, 99),
            samples[samples.len() - 1]
        );
    }
}

// ---------------------------------------------------------------------------
// Rate
// ---------------------------------------------------------------------------

#[test]
fn dpp_rate_screen_at_its_edges() {
    let b = YearBasis::Calendar365;
    let op = RateSource::Operator;
    for ok in [
        0.0,
        -0.0,
        1.0,
        -1.0,
        f64::MIN_POSITIVE,
        f64::from_bits(1),
        f64::EPSILON,
    ] {
        assert!(Rate::measured(ok, b, op).is_ok(), "{ok} refused");
    }
    for bad in [1.0 + f64::EPSILON, -1.000_001, 6.5, f64::MAX, -f64::MAX] {
        assert_eq!(
            Rate::measured(bad, b, op),
            Err(PricingError::RateImplausible { annual: bad })
        );
    }
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(matches!(
            Rate::measured(bad, b, op),
            Err(PricingError::RateNotFinite { .. })
        ));
    }
    assert_eq!(
        Rate::measured(0.065, b, RateSource::Charter("\t\n ")),
        Err(PricingError::RateUnsourced)
    );
    let cited = Rate::measured(0.065, b, RateSource::Charter("§x")).expect("cited");
    assert_eq!(cited.basis(), b);
    assert!(cited.source().to_string().contains("§x"));
}

// ---------------------------------------------------------------------------
// Tenor
// ---------------------------------------------------------------------------

#[test]
fn dpp_tenor_edges() {
    let e = expiry();
    // Same day, before the open: the whole session plus the gap is left.
    let pre = Tenor::between(ist(2026, 8, 27, 0, 0), e).expect("the morning of");
    assert_eq!(pre.seconds(), 15 * 3_600 + 40 * 60);
    assert_eq!(pre.calendar_days(), 0);
    // One microsecond before the close is alive, with zero whole seconds?
    // No: the stamp is floored to its second, so 15:39:59.999999 is 1 s.
    let last = Tenor::between(ist(2026, 8, 27, 15, 40) - 1, e).expect("1 s left");
    assert_eq!(last.seconds(), 1);
    // At the close, and one microsecond after it, are expired.
    assert_eq!(
        Tenor::between(ist(2026, 8, 27, 15, 40), e),
        Err(TenorError::AlreadyExpired { seconds_past: 0 })
    );
    assert_eq!(
        Tenor::between(ist(2026, 8, 27, 15, 40) + 1, e),
        Err(TenorError::AlreadyExpired { seconds_past: 0 })
    );
    // Long after.
    let late = Tenor::between(ist(2027, 8, 27, 15, 40), e).unwrap_err();
    assert_eq!(
        late,
        TenorError::AlreadyExpired {
            seconds_past: 365 * 86_400
        }
    );
    // The key space extremes are refused by name, never wrapped.
    for ts in [i64::MIN, i64::MAX, i64::MIN + 1, i64::MAX - 1] {
        assert_eq!(
            Tenor::between(ts, e),
            Err(TenorError::StampOffCalendar { ts_micros: ts })
        );
    }
    // A SATURDAY "EXPIRY" IS ACCEPTED, and that is recorded rather than fixed.
    // `Venue::hours_on` is a dated hours table, not a calendar: it answers the
    // 15:40 close for any day, so a misread contract name that lands on a
    // weekend is priced with two extra days of time value. Refusing it needs
    // a sourced rule that NSE derivatives never expire on a non-trading day,
    // which this file does not have (special Saturday sessions exist), so the
    // current behaviour is pinned and the gap is reported (proposal D-3113).
    let sat = Expiry::new(2026, 8, 29).expect("a calendar day");
    let thu = Tenor::between(ist(2026, 8, 20, 9, 15), expiry()).expect("Thursday");
    let on_sat = Tenor::between(ist(2026, 8, 20, 9, 15), sat).expect("pinned: accepted");
    assert_eq!(on_sat.seconds() - thu.seconds(), 2 * 86_400);
    // Leap day.
    let leap = Expiry::new(2024, 2, 29).expect("leap");
    let t = Tenor::between(ist(2024, 2, 28, 15, 30), leap).expect("a leap-day expiry");
    assert_eq!(t.seconds(), 86_400, "15:30 to the next day's 15:30 close");
    // Across a year end.
    let t = Tenor::between(
        ist(2025, 12, 31, 15, 30),
        Expiry::new(2026, 1, 1).expect("day"),
    )
    .expect("a new-year expiry");
    assert_eq!(t.seconds(), 86_400);
}

/// Monotone by property: an earlier stamp never has a shorter tenor. 200,000
/// fixed-seed pairs inside one contract's life.
#[test]
fn dpp_tenor_is_monotone_and_exactly_seconds() {
    let e = expiry();
    let from = ist(2026, 7, 1, 0, 0);
    let to = ist(2026, 8, 27, 15, 40);
    let span = (to - from) as u64;
    let mut mix = Mix(0x5EED_0004);
    for _ in 0..200_000 {
        let a = from + mix.below(span) as i64;
        let b = from + mix.below(span) as i64;
        let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
        let tl = Tenor::between(lo, e).expect("alive");
        let th = Tenor::between(hi, e).expect("alive");
        assert!(tl >= th);
        assert_eq!(
            tl.seconds() - th.seconds(),
            hi.div_euclid(1_000_000) - lo.div_euclid(1_000_000),
            "a tenor is exactly the second count to the close"
        );
        assert_eq!(
            tl.seconds(),
            to.div_euclid(1_000_000) - lo.div_euclid(1_000_000)
        );
    }
}

// ---------------------------------------------------------------------------
// price / solve_iv / price_all
// ---------------------------------------------------------------------------

#[test]
fn dpp_extreme_quotes_refuse_by_name_and_never_report_nan() {
    let base = atm_call();
    let cases: [(&str, Quote); 10] = [
        (
            "spot i64::MAX",
            Quote {
                spot: i64::MAX,
                ..base
            },
        ),
        (
            "strike i64::MAX",
            Quote {
                strike: i64::MAX,
                ..base
            },
        ),
        (
            "premium i64::MAX",
            Quote {
                premium: i64::MAX,
                ..base
            },
        ),
        ("strike 0", Quote { strike: 0, ..base }),
        (
            "spot i64::MIN",
            Quote {
                spot: i64::MIN,
                ..base
            },
        ),
        ("premium 1 paisa ATM", Quote { premium: 1, ..base }),
        (
            "deep ITM below intrinsic",
            Quote {
                strike: 1_000_000,
                premium: 1,
                ..base
            },
        ),
        (
            "deep OTM 1 paisa",
            Quote {
                strike: 5_000_000,
                premium: 1,
                ..base
            },
        ),
        (
            "spot 1 paisa",
            Quote {
                spot: 1,
                strike: 5_000,
                premium: 1,
                ..base
            },
        ),
        (
            "off-ladder strike",
            Quote {
                strike: 2_500_001,
                ..base
            },
        ),
    ];
    for (name, q) in cases {
        for side in [OptionSide::Call, OptionSide::Put] {
            let q = Quote { side, ..q };
            for vol in [None, Some(0.15)] {
                match price(q, vol, rate(), YearBasis::Calendar365) {
                    Ok(row) => {
                        assert!(row.volatility.is_finite(), "{name}: NaN vol reported Ok");
                        assert!(row.greeks.is_finite(), "{name}: non-finite greeks Ok");
                    }
                    Err(why) => assert!(!why.to_string().is_empty(), "{name}"),
                }
            }
        }
    }
}

#[test]
fn dpp_vendor_volatility_is_screened_by_the_model() {
    for bad in [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        0.0,
        -0.0,
        -0.15,
        10.000_001,
        f64::MAX,
    ] {
        let got = price(atm_call(), Some(bad), rate(), YearBasis::Calendar365);
        assert!(
            matches!(got, Err(PricingError::Model(_))),
            "vendor vol {bad} gave {got:?}"
        );
    }
    let ok = price(
        atm_call(),
        Some(f64::MIN_POSITIVE),
        rate(),
        YearBasis::Calendar365,
    );
    if let Ok(row) = ok {
        assert!(row.greeks.is_finite());
        assert_eq!(row.vol_from, VolSource::Vendor(Vendor::Groww));
    }
}

/// The side flips moneyness and nothing else about the ladder.
#[test]
fn dpp_side_flip_mirrors_moneyness() {
    for rung in -6_i64..=6 {
        let strike = 2_500_000 + rung * 5_000;
        let c = quote_at(
            atm_call().ts_micros,
            2_500_000,
            strike,
            30_000,
            OptionSide::Call,
        );
        let p = Quote {
            side: OptionSide::Put,
            ..c
        };
        let rc = price(c, Some(0.15), rate(), YearBasis::Calendar365).expect("call");
        let rp = price(p, Some(0.15), rate(), YearBasis::Calendar365).expect("put");
        assert_eq!(rc.moneyness.steps, rp.moneyness.steps);
        assert_eq!(rc.at_the_money, 2_500_000);
        let (cs, ps) = (rc.moneyness.to_string(), rp.moneyness.to_string());
        match rung.signum() {
            0 => assert!(cs == "ATM" && ps == "ATM"),
            1 => assert!(
                cs.starts_with("OTM+") && ps.starts_with("ITM-"),
                "{cs} {ps}"
            ),
            _ => assert!(
                cs.starts_with("ITM-") && ps.starts_with("OTM+"),
                "{cs} {ps}"
            ),
        }
        // Put-call parity under the vendor's one volatility: C - P = S - K e^{-rT}.
        let t = c.tenor.years(YearBasis::Calendar365);
        let parity = 2_500_000.0 - strike as f64 * (-0.065 * t).exp();
        assert!(
            ((rc.greeks.price - rp.greeks.price) - parity).abs() < 1e-3,
            "rung {rung}: parity off"
        );
    }
}

/// **A SECOND REASON MUST NOT BE HIDDEN BY THE FIRST ONE'S NUMBERS.**
///
/// The unmodified dedupe compared whole sentences, and a sentence carries the
/// row's own numbers. Five rows refused for one reason at five different
/// premiums were five "distinct reasons", filled every slot, and a later row
/// refused for a DIFFERENT reason was counted in `refused` and never named.
#[test]
fn dpp_price_all_keeps_one_sentence_per_reason_class() {
    let good = atm_call();
    let mut quotes = Vec::new();
    for premium in 1..=8 {
        // Deep in the money at almost nothing: below intrinsic, eight values.
        quotes.push(Quote {
            spot: 2_600_000,
            strike: 2_500_000,
            premium,
            ..good
        });
    }
    quotes.push(Quote { premium: 0, ..good });
    quotes.push(Quote {
        strike: 2_500_001,
        ..good
    });
    let done = price_all(&quotes, |_| None, rate(), YearBasis::Calendar365);
    assert_eq!(done.refused, 10);
    assert_eq!(done.offered(), quotes.len());
    assert!(done.why.len() <= REASONS_KEPT);
    assert!(
        done.why.iter().any(|w| w.contains("premium is 0 paisa")),
        "the zero-premium reason was hidden: {:#?}",
        done.why
    );
    assert!(
        done.why.iter().any(|w| w.contains("steps from the money")),
        "the off-ladder reason was hidden: {:#?}",
        done.why
    );
    let below = done.why.iter().filter(|w| w.contains("intrinsic")).count();
    assert_eq!(below, 1, "one reason, one sentence: {:#?}", done.why);
}

/// Counts balance under a fixed-seed mix of good, refused and vendor rows,
/// and a rerun is equal value for value.
#[test]
fn dpp_price_all_counts_balance_and_rerun_identically() {
    let good = atm_call();
    let mut mix = Mix(0x5EED_0005);
    let mut quotes = Vec::new();
    for i in 0..4_000_i64 {
        let ts = good.ts_micros + i * MINUTE;
        let mut q = quote_at(ts, 2_500_000, 2_500_000, 25_000, OptionSide::Call);
        match mix.below(7) {
            0 => q.premium = 0,
            1 => q.strike = 2_500_000 + 1 + mix.below(4_999) as i64,
            2 => q.premium = 1,
            3 => q.side = OptionSide::Put,
            4 => q.spot = -(mix.below(1_000) as i64),
            5 => q.strike = 2_500_000 + 5_000 * (mix.below(21) as i64 - 10),
            _ => {}
        }
        quotes.push(q);
    }
    let vendor = |ts: i64| {
        if ts.div_euclid(MINUTE) % 3 == 0 {
            Some(0.13)
        } else {
            None
        }
    };
    let a = price_all(&quotes, vendor, rate(), YearBasis::Calendar365);
    let b = price_all(&quotes, vendor, rate(), YearBasis::Calendar365);
    assert_eq!(a, b, "a rerun is not identical");
    assert_eq!(a.offered(), quotes.len());
    let vendored = a
        .rows
        .iter()
        .filter(|r| matches!(r.vol_from, VolSource::Vendor(_)))
        .count();
    assert_eq!(a.solved() + vendored, a.rows.len());
    assert!(a.solved() > 0 && vendored > 0 && a.refused > 0);
    assert_eq!(a.below_validated_band(), a.rows.len(), "all weekly rows");
    for row in &a.rows {
        assert!(row.volatility.is_finite() && row.greeks.is_finite());
        if let VolSource::Vendor(v) = row.vol_from {
            assert_eq!(v, Vendor::Groww);
            assert_eq!(row.volatility, 0.13, "the vendor's number passes untouched");
        }
    }
    let mut seen = Vec::new();
    for w in &a.why {
        assert!(!seen.contains(w), "a reason kept twice");
        seen.push(w.clone());
    }
}

/// **Divergence, recorded.** With a vendor volatility a premium under
/// intrinsic is priced; without one the same quote is refused. Pinned so a
/// change to either half is a decision, not drift.
#[test]
fn dpp_vendor_path_does_not_read_the_premium() {
    let q = Quote {
        spot: 2_600_000,
        strike: 2_500_000,
        premium: 1,
        ..atm_call()
    };
    assert!(matches!(
        solve_iv(q, rate(), YearBasis::Calendar365),
        Err(PricingError::Model(GreeksError::PriceBelowIntrinsic { .. }))
    ));
    let row = price(q, Some(0.15), rate(), YearBasis::Calendar365).expect("vendor path");
    assert_eq!(row.premium, 1);
    assert!(
        row.greeks.price > 99_000.0,
        "the model price ignores the premium"
    );
}

/// An empty run is an empty, balanced answer.
#[test]
fn dpp_price_all_on_nothing() {
    let done = price_all(&[], |_| None, rate(), YearBasis::Calendar365);
    assert_eq!(done, PricedAll::default());
    assert_eq!(done.offered(), 0);
}

/// **MEASURED**: one `price` call timed alone on each volatility path, and
/// `price_all` amortised per row at four run sizes. Asserts every row priced;
/// prints the numbers for the report.
#[test]
fn dpp_price_cost_is_measured() {
    let q = atm_call();
    for (label, vol) in [("solved", None), ("vendor", Some(0.15))] {
        let mut samples = Vec::with_capacity(20_000);
        for _ in 0..20_000 {
            let t = Instant::now();
            let got = std::hint::black_box(price(
                std::hint::black_box(q),
                vol,
                rate(),
                YearBasis::Calendar365,
            ));
            samples.push(t.elapsed().as_nanos());
            assert!(got.is_ok());
        }
        samples.sort_unstable();
        println!(
            "price ({label}): p50={}ns p99={}ns max={}ns",
            percentile(&samples, 50),
            percentile(&samples, 99),
            samples[samples.len() - 1]
        );
    }
    for n in [1_000_usize, 10_000, 100_000, 1_000_000] {
        let quotes = vec![q; n];
        let t = Instant::now();
        let done = price_all(&quotes, |_| Some(0.15), rate(), YearBasis::Calendar365);
        let per = t.elapsed().as_nanos() / n as u128;
        assert_eq!(done.rows.len(), n);
        println!("price_all (vendor) n={n}: {per}ns per row amortised");
    }
}

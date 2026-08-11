//! The permutation matrix: every pathological candle against every module.
//!
//! # What this is for
//!
//! Nine modules each refuse a corrupt record on their own terms, and the aggregate
//! refuses centrally. Nothing checked that those refusals **agree**, or that a module
//! given something no market can print returns a loud refusal rather than a panic or a
//! plausible wrong answer.
//!
//! The inputs below are not a wish list. Each one is either a state a real feed has
//! produced or a boundary of the type:
//!
//! | Input | Why it is here |
//! |---|---|
//! | `high < low` | corruption; a market can print a zero range, never a negative one |
//! | `high == low` | real — a limit-locked or untraded minute |
//! | open above high, close below low | real — a vendor mis-assembling OHLC from ticks |
//! | `hi = i64::MAX`, `lo = i64::MIN` | the range does not fit `i64` |
//! | `i64::MIN` anywhere | §7 reserves it for the open-interest null; `abs()` panics on it |
//! | `ts = -1`, `ts = i64::MIN` | pre-epoch; truncating division mis-dates it |
//! | duplicate and out-of-order timestamps | real — a replayed or reordered feed |
//! | minute 375, 376, 16:59 IST | `docs/06-limits.md` records ten sessions outside 09:15–15:29 |
//! | volume `< 0` | corruption; zero is a real zero |
//!
//! # The rule every case asserts
//!
//! **Loud refusal, or a correct answer. Never a panic, never a silent wrong answer.**
//!
//! These tests run under the `test` profile, where integer overflow **panics** rather
//! than wrapping. So a case that merely returns without asserting still proves
//! something: the arithmetic did not overflow on the way.

#![allow(
    clippy::expect_used,
    reason = "the same exception every test module in this workspace takes — see \
              tests/shared_core_doc.rs beside this file: a test that cannot panic \
              cannot fail. `expect` and not the `let ... else { unreachable!() }` this \
              file used to spell, because `unreachable!` expands to a panic inside the \
              crate under test and is therefore a coverage region no green run can ever \
              execute, while `expect` panics inside the standard library and leaves no \
              such region behind."
)]

use indicators::daily::DailyLevels;
use indicators::evaluator::{Evaluator, Widths};
use indicators::gap::GapFib;
use indicators::orb::Orb;
use indicators::pattern::{Patterns, Thresholds};
use indicators::session::SessionState;
use indicators::trend::{TrendState, TrendThresholds};
use indicators::vwap::{Availability, Vwap};
use indicators::{Candle, Corrupt, CurDayFib};
use vocab::{ConditionMask, Tolerance};

const IST_OPEN_UTC_MICROS: i64 = (555 - 330) * 60 * 1_000_000;
const MINUTE: i64 = 60 * 1_000_000;

fn tol() -> Tolerance {
    vocab::tolerance::pinned_fib().expect("the pinned fib width is valid")
}

fn widths() -> Widths {
    Widths::pinned().expect("both pinned widths are valid")
}

fn c(ts: i64, open: i64, high: i64, low: i64, close: i64, volume: i64, oi: i64) -> Candle {
    Candle {
        ts_micros: ts,
        open,
        high,
        low,
        close,
        volume,
        open_interest: oi,
    }
}

/// A sane candle, as the baseline every case is a deviation from.
fn sane(minute: i64) -> Candle {
    let p = 2_500_000 + minute * 10;
    c(
        IST_OPEN_UTC_MICROS + minute * MINUTE,
        p,
        p + 500,
        p - 500,
        p + 100,
        0,
        i64::MIN,
    )
}

/// Every case, named, with what the aggregate evaluator must do with it.
///
/// `Some(Corrupt::..)` means a refusal is required. `None` means the candle is legal
/// and must be accepted — a refusal there would silently drop real bars from a run.
#[expect(
    clippy::too_many_lines,
    reason = "this IS the data. Splitting the matrix into four helpers to satisfy a \
              line count would put the cases in four places, and the whole value of \
              the table is that every pathological input this crate has been asked \
              about is visible in one screenful beside the reason it is there."
)]
fn matrix() -> Vec<(&'static str, Candle, Option<Corrupt>)> {
    vec![
        // ── corruption that MUST be refused ────────────────────────────────────
        (
            "high below low",
            c(0, 100, 90, 110, 100, 0, i64::MIN),
            Some(Corrupt::HighBelowLow),
        ),
        (
            "high below low by one paisa",
            c(0, 100, 99, 100, 100, 0, i64::MIN),
            Some(Corrupt::HighBelowLow),
        ),
        (
            "range straddles the whole i64",
            c(0, 0, i64::MAX, i64::MIN, 0, 0, i64::MIN),
            Some(Corrupt::RangeOverflows),
        ),
        (
            "range overflows by one",
            c(0, 0, 0, i64::MIN, 0, 0, i64::MIN),
            Some(Corrupt::RangeOverflows),
        ),
        // ── legal, and MUST be accepted ────────────────────────────────────────
        (
            "zero range: a limit-locked minute",
            c(0, 2_500_000, 2_500_000, 2_500_000, 2_500_000, 0, i64::MIN),
            None,
        ),
        (
            "four-price doji at the top of i64",
            c(0, i64::MAX, i64::MAX, i64::MAX, i64::MAX, 0, i64::MIN),
            None,
        ),
        (
            "four-price doji at the bottom of i64",
            c(0, i64::MIN, i64::MIN, i64::MIN, i64::MIN, 0, i64::MIN),
            None,
        ),
        (
            // This row said `None` — accepted — and that was the defect. A design
            // review traced it: with open 2_600_000, high 2_500_000, low 2_400_000,
            // close 2_300_000, both wicks are MINUS 100,000, every `_at_most` predicate
            // is satisfied by a negative against a positive bound, and the bar was
            // labelled a long bearish marubozu — "no wicks" — because its wicks were
            // impossible. `store::format::Bar::ohlc_is_sane` refuses exactly this; the
            // evaluator did not, because a doc comment in this crate wrongly said that
            // function checked field order only.
            "open above high, close below low: a mis-assembled OHLC",
            c(0, 2_600_000, 2_500_000, 2_400_000, 2_300_000, 0, i64::MIN),
            Some(Corrupt::PriceOutsideRange),
        ),
        (
            "open one paisa above high",
            c(0, 2_500_001, 2_500_000, 2_400_000, 2_450_000, 0, i64::MIN),
            Some(Corrupt::PriceOutsideRange),
        ),
        (
            "close one paisa below low",
            c(0, 2_450_000, 2_500_000, 2_400_000, 2_399_999, 0, i64::MIN),
            Some(Corrupt::PriceOutsideRange),
        ),
        (
            "negative prices",
            c(
                0,
                -2_500_000,
                -2_400_000,
                -2_600_000,
                -2_500_000,
                0,
                i64::MIN,
            ),
            None,
        ),
        ("all prices zero", c(0, 0, 0, 0, 0, 0, i64::MIN), None),
        (
            // This row said `None` — accepted — and the propagation of VWAP's refusals
            // corrected it. A negative volume is corruption on any reading: §7 says zero
            // is a real zero, so negative is not "no trades", it is a broken record. Only
            // the VWAP family reads volume at all, and while its refusal was discarded by
            // `if let Ok(v) = ...` the bar came back as a SUCCESSFUL evaluation with
            // twenty positions silently absent.
            "negative volume",
            c(0, 2_500_000, 2_500_500, 2_499_500, 2_500_000, -1, i64::MIN),
            Some(Corrupt::NegativeVolume),
        ),
        (
            "volume at i64::MAX",
            c(
                0,
                2_500_000,
                2_500_500,
                2_499_500,
                2_500_000,
                i64::MAX,
                i64::MIN,
            ),
            None,
        ),
        (
            "open interest zero, which means zero and not absent",
            c(0, 2_500_000, 2_500_500, 2_499_500, 2_500_000, 0, 0),
            None,
        ),
        (
            "open interest at i64::MAX",
            c(0, 2_500_000, 2_500_500, 2_499_500, 2_500_000, 0, i64::MAX),
            None,
        ),
        // ── timestamps at and past every boundary ─────────────────────────────
        (
            "timestamp zero: the epoch",
            c(0, 2_500_000, 2_500_500, 2_499_500, 2_500_000, 0, i64::MIN),
            None,
        ),
        (
            "timestamp minus one: one microsecond pre-epoch",
            c(-1, 2_500_000, 2_500_500, 2_499_500, 2_500_000, 0, i64::MIN),
            None,
        ),
        (
            "timestamp exactly the IST offset, negated",
            c(
                -19_800_000_000,
                2_500_000,
                2_500_500,
                2_499_500,
                2_500_000,
                0,
                i64::MIN,
            ),
            None,
        ),
        (
            "timestamp at i64::MIN",
            c(
                i64::MIN,
                2_500_000,
                2_500_500,
                2_499_500,
                2_500_000,
                0,
                i64::MIN,
            ),
            None,
        ),
        (
            "timestamp at i64::MIN plus one",
            c(
                i64::MIN + 1,
                2_500_000,
                2_500_500,
                2_499_500,
                2_500_000,
                0,
                i64::MIN,
            ),
            None,
        ),
        (
            "timestamp at i64::MAX",
            c(
                i64::MAX,
                2_500_000,
                2_500_500,
                2_499_500,
                2_500_000,
                0,
                i64::MIN,
            ),
            None,
        ),
        (
            "minute 375: one past the nominal session close",
            c(
                IST_OPEN_UTC_MICROS + 375 * MINUTE,
                2_500_000,
                2_500_500,
                2_499_500,
                2_500_000,
                0,
                i64::MIN,
            ),
            None,
        ),
        (
            "minute 376",
            c(
                IST_OPEN_UTC_MICROS + 376 * MINUTE,
                2_500_000,
                2_500_500,
                2_499_500,
                2_500_000,
                0,
                i64::MIN,
            ),
            None,
        ),
        (
            "16:59 IST: the 2021-02-24 session really ran that late",
            c(
                IST_OPEN_UTC_MICROS + 464 * MINUTE,
                2_500_000,
                2_500_500,
                2_499_500,
                2_500_000,
                0,
                i64::MIN,
            ),
            None,
        ),
        (
            "IST midnight: minute zero of the day, long before the open",
            c(
                IST_OPEN_UTC_MICROS - 555 * MINUTE,
                2_500_000,
                2_500_500,
                2_499_500,
                2_500_000,
                0,
                i64::MIN,
            ),
            None,
        ),
    ]
}

/// The aggregate evaluator refuses exactly what it should and accepts exactly what it
/// should, across the whole matrix.
///
/// A refusal on a legal candle is as much a defect as a panic: it silently drops a real
/// bar from a run, and no output says so.
#[test]
fn the_evaluator_agrees_with_the_matrix() {
    for (name, candle, expected) in matrix() {
        let mut e = Evaluator::new(widths(), Availability::Absent, Thresholds::CLASSICAL);
        let got = e.step(&candle);
        match expected {
            Some(want) => assert_eq!(got, Err(want), "{name}: expected a refusal"),
            None => assert!(
                got.is_ok(),
                "{name}: a legal candle was refused with {got:?}"
            ),
        }
    }
}

/// Every module's refusal agrees with the aggregate's.
///
/// Nine modules each re-derive "is this a candle". Nine chances to disagree, and a
/// disagreement means a partially-evaluated bar: some families emitted, others refused,
/// and the mask is a mixture of two answers.
#[test]
fn every_module_refuses_exactly_what_the_evaluator_refuses() {
    for (name, candle, expected) in matrix() {
        let refused = expected.is_some();

        let mut fib = CurDayFib::new();
        assert_eq!(
            fib.step(&candle, tol()).is_err(),
            refused,
            "{name}: CurDayFib disagreed with the evaluator"
        );

        let mut pat = Patterns::new(Thresholds::CLASSICAL);
        assert_eq!(
            pat.step(&candle).is_err(),
            refused,
            "{name}: Patterns disagreed"
        );

        let mut orb = Orb::new();
        assert_eq!(
            orb.step(&candle, tol()).is_err(),
            refused,
            "{name}: Orb disagreed"
        );

        let mut sess = SessionState::default();
        assert_eq!(
            sess.step(&candle, None, tol()).is_err(),
            refused,
            "{name}: SessionState disagreed"
        );

        let mut gap = GapFib::new();
        assert_eq!(
            gap.step(&candle, tol()).is_err(),
            refused,
            "{name}: GapFib disagreed"
        );

        let mut trend = TrendState::new(TrendThresholds::CLASSICAL);
        assert_eq!(
            trend.step(&candle, tol()).is_err(),
            refused,
            "{name}: TrendState disagreed"
        );

        // Vwap has its own error type and one extra refusal — a negative volume — so it
        // is a superset rather than an equal. Asserting equality here would be wrong.
        let mut v = Vwap::for_slice(Availability::Present);
        if refused {
            assert!(
                v.step(&candle, tol()).is_err(),
                "{name}: Vwap accepted a corrupt record"
            );
        }
    }
}

/// A refused candle leaves no trace: the next legal candle behaves as if it were first.
///
/// Otherwise a corrupt record silently poisons the state that follows it, which is
/// worse than the refusal it looks like.
#[test]
fn a_refused_candle_changes_nothing() {
    for (name, bad, expected) in matrix() {
        if expected.is_none() {
            continue;
        }
        let mut poisoned = Evaluator::new(widths(), Availability::Absent, Thresholds::CLASSICAL);
        assert!(poisoned.step(&bad).is_err(), "{name}: expected a refusal");
        // The case name is built into a `String` here rather than interpolated on the
        // failure path: `expect` takes a `&str`, and a message assembled only when the
        // test fails is a region only a red run reaches.
        let label = format!("{name}: a sane candle after a refusal");
        let after = poisoned.step(&sane(0)).expect(&label);

        let mut clean = Evaluator::new(widths(), Availability::Absent, Thresholds::CLASSICAL);
        let fresh = clean
            .step(&sane(0))
            .expect("a sane candle on a fresh evaluator");
        assert_eq!(after, fresh, "{name}: the refused candle left state behind");
    }
}

/// A receding or repeated timestamp is REFUSED, and a forward gap is accepted.
///
/// This test previously asserted the opposite — that any of these was fine as long as
/// it did not panic — and its own docstring said "none of these is required to produce
/// a *meaningful* answer". That was the defect, not the standard: the session rollover
/// keys on the IST day CHANGING, so a receding timestamp closes the books on a later
/// session and installs it as yesterday. Every level the daily, previous-day and
/// five-session families emit then comes from a session after the bar being evaluated.
/// Survival is not the property that matters; refusal is.
#[test]
fn a_receding_timestamp_is_refused_and_a_forward_gap_is_not() {
    let cases: Vec<(&str, Vec<Candle>)> = vec![
        ("the same candle twice", vec![sane(0), sane(0)]),
        (
            "a duplicate timestamp with different prices",
            vec![sane(0), c(IST_OPEN_UTC_MICROS, 1, 2, 0, 1, 0, i64::MIN)],
        ),
        (
            "strictly backwards",
            vec![sane(5), sane(4), sane(3), sane(2)],
        ),
        (
            "one candle far in the past after one far in the future",
            vec![sane(0), c(i64::MAX / 2, 1, 2, 0, 1, 0, i64::MIN), sane(1)],
        ),
        (
            "a three-year gap",
            vec![
                sane(0),
                c(
                    IST_OPEN_UTC_MICROS + 3 * 365 * 24 * 60 * MINUTE,
                    2_500_000,
                    2_500_500,
                    2_499_500,
                    2_500_000,
                    0,
                    i64::MIN,
                ),
            ],
        ),
        (
            "alternating days, back and forth",
            vec![sane(0), sane(1440), sane(1), sane(1441), sane(2)],
        ),
    ];
    for (name, series) in cases {
        let mut e = Evaluator::new(widths(), Availability::Absent, Thresholds::CLASSICAL);
        let mut last: Option<i64> = None;
        for (i, candle) in series.iter().enumerate() {
            let forward = last.is_none_or(|l| candle.ts_micros > l);
            let got = e.step(candle);
            if forward {
                assert!(
                    got.is_ok(),
                    "{name}: forward candle {i} was refused with {got:?}"
                );
                last = Some(candle.ts_micros);
            } else {
                assert_eq!(
                    got,
                    Err(Corrupt::TimestampNotIncreasing),
                    "{name}: candle {i} recedes and must be refused, not survived"
                );
            }
        }
    }
}

/// A single candle, then a query. Nothing may claim a level it cannot have.
#[test]
fn one_candle_claims_nothing_it_cannot_know() {
    let mut e = Evaluator::new(widths(), Availability::Absent, Thresholds::CLASSICAL);
    let mask = e.step(&sane(0)).expect("a sane candle");
    assert!(
        !e.has_yesterday(),
        "one candle cannot have produced a completed session"
    );
    assert_eq!(e.sessions_completed(), 0);
    // Whatever it emitted, no position outside the live set may be set.
    assert_eq!(vocab::table::only_live(mask), mask);
}

/// Zero candles: every accessor answers without panicking.
#[test]
fn an_empty_run_answers_without_panicking() {
    let e = Evaluator::new(widths(), Availability::Absent, Thresholds::CLASSICAL);
    assert!(!e.has_yesterday());
    assert_eq!(e.sessions_completed(), 0);
    assert_eq!(GapFib::new().leg(), None);
    assert_eq!(Vwap::for_slice(Availability::Present).value(), None);
    assert_eq!(Vwap::for_slice(Availability::Present).sigma(), None);
    assert_eq!(Orb::new().extremes(0), None);
    // And an out-of-range window index is a `None`, not an index panic.
    assert_eq!(Orb::new().extremes(99), None);
    assert!(!Orb::new().window_closed(99));
}

/// The previous session's own extremes at the edges of `i64`.
///
/// `DailyLevels` computes a pivot from three prices and a ladder from that. At the
/// extremes the intermediate sums leave `i64`, which is why the arithmetic is `i128`.
#[test]
fn daily_levels_at_the_edges_of_the_type() {
    let cases: [(i64, i64, i64, bool); 7] = [
        (i64::MAX, i64::MIN, 0, false),
        (i64::MAX, 0, i64::MAX, true),
        // h=0 with l=i64::MIN: the range does not fit i64, so this must NOT build.
        // The row said `true` and that was my error, not the code's.
        (0, i64::MIN, i64::MIN, false),
        (i64::MAX, i64::MAX, i64::MAX, true),
        (i64::MIN, i64::MIN, i64::MIN, true),
        (1, 0, 0, true),
        (2_500_000, 2_400_000, 2_450_000, true),
    ];
    for (high, low, close, should_build) in cases {
        let built = DailyLevels::from_previous_session(high, low, close);
        assert_eq!(
            built.is_ok(),
            should_build,
            "({high}, {low}, {close}): expected build = {should_build}"
        );
        // Where it did build, every level must be finite and the CPR ordered.
        if let Ok(levels) = built {
            let (lo, hi) = levels.cpr_span();
            assert!(
                lo <= hi,
                "({high}, {low}, {close}): cpr_span came back unordered"
            );
            for n in 1..=5_usize {
                assert!(levels.resistance(n).is_some(), "R{n} missing");
                assert!(levels.support(n).is_some(), "S{n} missing");
            }
        }
    }
}

/// The whole matrix, fed as one series, still produces only live bits.
///
/// The individual cases are checked above; this checks they do not interact. Nine
/// modules with nine day-change clocks, driven by timestamps that jump across the
/// entire `i64` range in one run, is the state-machine stress case.
#[test]
fn the_whole_matrix_as_one_series_emits_only_live_bits() {
    let mut e = Evaluator::new(widths(), Availability::Absent, Thresholds::CLASSICAL);
    let mut accepted = 0_u32;
    // Restamped forward, because the evaluator now refuses a receding timestamp and the
    // matrix deliberately holds several of the same. The PRICES are what this test is
    // about; ordering has its own test above.
    // Counted in `i64` by the zip rather than converted from a `usize` index: the
    // `i64::try_from` this used to do cannot fail on a table of this size, and an arm no
    // input can take is a region no test can reach.
    for ((name, mut candle, _), step) in matrix().into_iter().zip(0_i64..) {
        candle.ts_micros = IST_OPEN_UTC_MICROS + step * MINUTE;
        if let Ok(mask) = e.step(&candle) {
            accepted = accepted.saturating_add(1);
            assert_eq!(
                vocab::table::only_live(mask),
                mask,
                "{name}: a non-live position escaped"
            );
            let claimed = Evaluator::positions();
            let mut bit = 0_u32;
            while bit < ConditionMask::BITS {
                if mask.get(bit) {
                    let index =
                        u16::try_from(bit).expect("bit < ConditionMask::BITS, and that is 384");
                    assert!(
                        claimed.contains(&index),
                        "{name}: bit {bit} is claimed by nobody"
                    );
                }
                bit = bit.saturating_add(1);
            }
        }
    }
    assert!(accepted > 15, "only {accepted} of the matrix was accepted");
}

/// Two runs over the matrix agree exactly — §3 rule 5, on the worst input available.
#[test]
fn two_runs_over_the_matrix_agree() {
    let series: Vec<Candle> = matrix()
        .into_iter()
        .enumerate()
        .map(|(i, (_, mut candle, _))| {
            let step = i64::try_from(i).unwrap_or(0);
            candle.ts_micros = IST_OPEN_UTC_MICROS + step * MINUTE;
            candle
        })
        .collect();
    let run = || {
        let mut e = Evaluator::new(widths(), Availability::Absent, Thresholds::CLASSICAL);
        series
            .iter()
            .map(|candle| e.step(candle).ok())
            .collect::<Vec<_>>()
    };
    let first = run();
    for _ in 0..4 {
        assert_eq!(run(), first, "a rerun over the matrix disagreed");
    }
}

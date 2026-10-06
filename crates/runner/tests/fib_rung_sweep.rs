//! The screened sweep finds every Fibonacci combination that fires, including
//! rungs that fire together after whole-paisa rounding (h-eng-2, D-1861,
//! AHB-02).
//!
//! `vocab::implication` screens only the pivot chain. Were a Fibonacci pair
//! ever screened as "mutually exclusive", a column whose rungs share a paisa
//! would lose real combinations silently. This builds columns from the
//! production ladders at the legs where rungs do share a paisa (1 and 5 paisa
//! gap legs, and the previous-day ladders at 9, 100 and 902 paisa) and one
//! where they do not (10 paisa), walks `engine::Ladder` over each, and checks
//! the result against a brute-force enumeration of every subset with no
//! screen at all.

#![allow(
    clippy::expect_used,
    reason = "a failing fixture must name its premise"
)]

use std::collections::BTreeMap;

use engine::Ladder;
use indicators::Candle;
use indicators::daily::DailyLevels;
use indicators::evaluator::Calendar;
use indicators::fib::prev_day_bits;
use indicators::gap::GapFib;
use vocab::{ConditionMask, Tolerance};

const DAY: i64 = 86_400_000_000;
const MINUTE: i64 = 60_000_000;
const OPEN: i64 = 225 * MINUTE;
const BASE: i64 = 2_450_000;
const GAP_HIGH: i64 = BASE + 1_000;

fn tol() -> Tolerance {
    vocab::tolerance::pinned_fib().expect("the pinned Fibonacci width")
}

fn at(day: i64, minute: i64, high: i64, low: i64, close: i64) -> Candle {
    Candle {
        ts_micros: day * DAY + OPEN + minute * MINUTE,
        open: close,
        high,
        low,
        close,
        volume: 0,
        open_interest: i64::MIN,
    }
}

fn gap_up(range: i64) -> GapFib {
    let mut g = GapFib::new();
    let calendar = Calendar::all_regular();
    for m in 0..6 {
        g.step(&at(30_000, m, GAP_HIGH, BASE, BASE), tol(), &calendar)
            .expect("a sane bar");
    }
    for m in 0..4 {
        g.step(
            &at(30_001, m, GAP_HIGH + range, BASE, BASE),
            tol(),
            &calendar,
        )
        .expect("a sane bar");
    }
    assert!(g.leg().is_some());
    g
}

/// Every close from `lo` to `hi`, each twice so a pair can reach two hits,
/// and a third copy of every other close so supports differ.
fn column(lo: i64, hi: i64, bits: impl Fn(i64) -> ConditionMask) -> Vec<ConditionMask> {
    let mut out = Vec::new();
    for close in lo..=hi {
        let copies = if close % 2 == 0 { 3 } else { 2 };
        for _ in 0..copies {
            out.push(bits(close));
        }
    }
    out
}

fn live() -> Vec<u32> {
    (0..ConditionMask::BITS)
        .filter(|&b| vocab::table::LIVE.get(b))
        .collect()
}

/// Every non-empty subset of the positions that vary on this column, with its
/// support, kept when the support reaches `min_hits`. No screen of any kind.
fn brute_force(rows: &[ConditionMask], min_hits: u64) -> BTreeMap<[u64; 6], u64> {
    let bars = rows.len() as u64;
    let varying: Vec<u32> = (0..ConditionMask::BITS)
        .filter(|&b| {
            let support = rows.iter().filter(|r| r.get(b)).count() as u64;
            vocab::table::LIVE.get(b) && support > 0 && support < bars
        })
        .collect();
    assert!(varying.len() <= 16, "a brute force this test can afford");
    let mut out = BTreeMap::new();
    for subset in 1_u32..(1 << varying.len()) {
        let mask = varying
            .iter()
            .enumerate()
            .filter(|(i, _)| subset & (1 << i) != 0)
            .fold(ConditionMask::ZERO, |m, (_, b)| m.with_bit(*b));
        let hits = rows.iter().filter(|r| r.hits(&mask)).count() as u64;
        if hits >= min_hits {
            out.insert(mask.words(), hits);
        }
    }
    out
}

fn sweep(rows: &[ConditionMask], min_hits: u64) -> BTreeMap<[u64; 6], u64> {
    let result = Ladder::with_min_hits(min_hits).walk(rows, &live());
    assert!(result.halted.is_none(), "the walk went extinct");
    result
        .all_frequent()
        .map(|item| (item.mask.words(), item.hits))
        .collect()
}

fn pair(a: u32, b: u32) -> [u64; 6] {
    ConditionMask::ZERO.with_bit(a).with_bit(b).words()
}

fn check(rows: &[ConditionMask], co_firing: &[(u32, u32)]) {
    for min_hits in [1, 2, 3, 5] {
        let found = sweep(rows, min_hits);
        let expected = brute_force(rows, min_hits);
        assert_eq!(found, expected, "the screened sweep lost or invented a set");
        if min_hits <= 2 {
            for (a, b) in co_firing {
                assert!(
                    found.contains_key(&pair(*a, *b)),
                    "co-firing rungs {a} and {b} must be a combination the sweep finds"
                );
            }
        }
    }
}

/// AHB-02. Gap legs of 1, 5 and 10 paisa through `GapFib::bits`.
#[test]
fn gap_legs_of_one_five_and_ten_paisa_sweep_exactly_as_brute_force() {
    let one = gap_up(1);
    let rows = column(GAP_HIGH - 3, GAP_HIGH + 3, |c| one.bits(c, tol()));
    check(&rows, &[(133, 134), (137, 138), (139, 140)]);

    let five = gap_up(5);
    let rows = column(GAP_HIGH - 6, GAP_HIGH + 8, |c| five.bits(c, tol()));
    check(&rows, &[(133, 134), (136, 137)]);

    let ten = gap_up(10);
    let rows = column(GAP_HIGH - 30, GAP_HIGH + 12, |c| ten.bits(c, tol()));
    assert!(
        rows.iter().all(|r| r.popcount() <= 1),
        "10 paisa: one rung at most"
    );
    check(&rows, &[]);
}

/// AHB-02. The two previous-day ladders, where 20 and 70 (and 24 and 69) fire
/// together at ranges up to 902 paisa.
#[test]
fn previous_day_ranges_where_two_ladders_share_a_close_sweep_exactly_as_brute_force() {
    for range in [9, 100, 902] {
        let levels =
            DailyLevels::from_previous_session(BASE + range, BASE, BASE).expect("a usable day");
        let bits = |c| prev_day_bits(&levels, c, tol());
        // The closes around up-78.6% and down-23.6%, and up-23.6% and down-78.6%.
        let band = range / 100 + 2;
        let mut rows = column(
            BASE + range * 764 / 1000 - band,
            BASE + range * 786 / 1000 + band,
            bits,
        );
        rows.extend(column(
            BASE + range * 214 / 1000 - band,
            BASE + range * 236 / 1000 + band,
            bits,
        ));
        check(&rows, &[(20, 70), (24, 69)]);
    }
}

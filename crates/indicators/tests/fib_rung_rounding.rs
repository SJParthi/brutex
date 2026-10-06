//! Which Fibonacci rungs fire together once levels are whole paisa (h-eng-2,
//! D-1861, AHB-02).
//!
//! `vocab::tolerance` proved at compile time that two rungs of one ladder
//! never fire on one bar, on EXACT levels. Every ladder here floors its level
//! to a paisa, and at a range of at most 10 paisa the band itself is zero
//! paisa, so two rungs flooring to one paisa fire together on a close at that
//! price. This file measures every range 1..=2,000 paisa, plus three ranges
//! near the top of the type, on all four ladder families through their
//! production doors, and pins exactly where co-firing happens.
//!
//! The close set is exhaustive, not sampled: a close fires a rung only within
//! `range / 100` paisa of its level, so every close that could fire anything
//! is within that distance of some rung, and those are the closes walked.

#![allow(
    clippy::expect_used,
    reason = "a failing fixture must name its premise"
)]

use std::collections::BTreeMap;

use indicators::Candle;
use indicators::CurDayFib;
use indicators::daily::DailyLevels;
use indicators::evaluator::Calendar;
use indicators::fib::{PREV_DAY_DOWN, PREV_DAY_UP, Prev5, prev_day_bits};
use indicators::gap::GapFib;
use vocab::tolerance::RUNG_EXCLUSIVE_MIN_RANGE;
use vocab::{ConditionMask, Tolerance};

const DAY: i64 = 86_400_000_000;
const MINUTE: i64 = 60_000_000;
const OPEN: i64 = 225 * MINUTE;
const BASE: i64 = 2_450_000;
/// Yesterday's high on the gap fixture: an up gap's `X1`.
const GAP_HIGH: i64 = BASE + 1_000;
/// Every range up to here is walked one paisa at a time.
const WALKED: i64 = 2_000;
/// Ranges near the top of the type, where every ladder level still fits `i64`.
const EXTREME: [i64; 3] = [1_000_000_007, 1_000_000_000_003, i64::MAX / 8];

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

fn fired(mask: ConditionMask) -> Vec<u32> {
    (0..ConditionMask::BITS).filter(|&b| mask.get(b)).collect()
}

/// A gap leg of `range` paisa: up from `X1 = GAP_HIGH`, or down from
/// `X1 = BASE + range` to `X2 = BASE`, so no price leaves the positive range.
fn gap(range: i64, up: bool) -> GapFib {
    let mut g = GapFib::new();
    let calendar = Calendar::all_regular();
    let (y_high, y_low) = if up {
        (GAP_HIGH, BASE)
    } else {
        (GAP_HIGH + range, BASE + range)
    };
    for m in 0..6 {
        g.step(&at(30_000, m, y_high, y_low, y_low), tol(), &calendar)
            .expect("a sane bar");
    }
    let (high, low) = if up {
        (GAP_HIGH + range, BASE)
    } else {
        (y_high, BASE)
    };
    // Three bars of the opening candle and one of the next span, which settles
    // the leg (D-1441).
    for m in 0..4 {
        g.step(&at(30_001, m, high, low, low), tol(), &calendar)
            .expect("a sane bar");
    }
    assert!(g.leg().is_some(), "a {range}-paisa gap establishes a leg");
    g
}

/// A current-day leg of `range` paisa from `BASE`, up or down.
fn curday(range: i64, up: bool) -> CurDayFib {
    let mut c = CurDayFib::new();
    let (high, low) = (BASE + range, BASE);
    if up {
        c.step(&at(30_000, 0, low, low, low), tol()).expect("sane");
        c.step(&at(30_000, 1, high, low, high), tol())
            .expect("sane");
    } else {
        c.step(&at(30_000, 0, high, high, high), tol())
            .expect("sane");
        c.step(&at(30_000, 1, high, low, low), tol()).expect("sane");
    }
    assert_eq!(c.range(), range);
    c
}

/// Every close within one band (plus a paisa) of any level any family places
/// at this range: a superset of every close that can fire a rung. Near the top
/// of the type only the band edges are walked.
fn closes(range: i64) -> Vec<i64> {
    let band = range / 100 + 1;
    let offsets: Vec<i64> = if range <= WALKED {
        (-band..=band).collect()
    } else {
        vec![-band, 1 - band, -1, 0, 1, band - 1, band]
    };
    let mut out = Vec::new();
    for anchor in [BASE, BASE + range, GAP_HIGH, GAP_HIGH + range, BASE - range] {
        for p in [
            0_i128, 236, 382, 500, 618, 786, 1000, 1272, 1618, 2000, 2618, 4236,
        ] {
            let r = i128::from(range);
            for step in [(p * r).div_euclid(1000), -(-p * r).div_euclid(1000)] {
                for level in [i128::from(anchor) + step, i128::from(anchor) - step] {
                    for d in &offsets {
                        if let Ok(close) = i64::try_from(level + i128::from(*d)) {
                            out.push(close);
                        }
                    }
                }
            }
        }
    }
    out.sort_unstable();
    out.dedup();
    out
}

fn ranges() -> impl Iterator<Item = i64> {
    (1..=WALKED).chain(EXTREME)
}

/// The previous-day mask split into its two ladders.
fn split(bits: &[u32]) -> (usize, usize) {
    let on = |ladder: &[(i32, u16)], b: u32| ladder.iter().any(|(_, p)| u32::from(*p) == b);
    let down = bits.iter().filter(|b| on(&PREV_DAY_DOWN, **b)).count();
    let up = bits.iter().filter(|b| on(&PREV_DAY_UP, **b)).count();
    (down, up)
}

/// Per family, the ranges at which two rungs of ONE ladder fired together,
/// and the previous-day cross-ladder pairs with the ranges they fired at.
type Measured = (
    BTreeMap<&'static str, Vec<i64>>,
    BTreeMap<Vec<u32>, Vec<i64>>,
);

fn measure() -> Measured {
    let mut within: BTreeMap<&'static str, Vec<i64>> = BTreeMap::new();
    let mut cross: BTreeMap<Vec<u32>, Vec<i64>> = BTreeMap::new();
    for range in ranges() {
        let levels =
            DailyLevels::from_previous_session(BASE + range, BASE, BASE).expect("a usable day");
        let mut five = Prev5::default();
        for _ in 0..5 {
            five.push_completed_session(BASE + range, BASE);
        }
        let (cur_up, cur_down) = (curday(range, true), curday(range, false));
        let (gap_up, gap_down) = (gap(range, true), gap(range, false));
        for close in closes(range) {
            let families = [
                ("prev5", five.bits(close, tol())),
                ("current_up", cur_up.bits(close, tol())),
                ("current_down", cur_down.bits(close, tol())),
                ("gap_up", gap_up.bits(close, tol())),
                ("gap_down", gap_down.bits(close, tol())),
            ];
            for (name, mask) in families {
                if mask.popcount() >= 2 {
                    within.entry(name).or_default().push(range);
                }
            }
            let bits = fired(prev_day_bits(&levels, close, tol()));
            let (down, up) = split(&bits);
            assert_eq!(
                down + up,
                bits.len(),
                "prev_day_bits emits only its ladders"
            );
            if down >= 2 || up >= 2 {
                within.entry("prev_day").or_default().push(range);
            } else if bits.len() >= 2 {
                cross.entry(bits).or_default().push(range);
            }
        }
    }
    for list in within.values_mut().chain(cross.values_mut()) {
        list.dedup();
    }
    (within, cross)
}

/// AHB-02. Two rungs of one ladder fire together only below
/// `RUNG_EXCLUSIVE_MIN_RANGE`, on every family, and exactly at 1-6 and 8 paisa.
#[test]
fn two_rungs_of_one_ladder_fire_together_only_below_the_range_floor() {
    let (within, _) = measure();
    let families = [
        "current_down",
        "current_up",
        "gap_down",
        "gap_up",
        "prev5",
        "prev_day",
    ];
    assert_eq!(
        within.keys().copied().collect::<Vec<_>>(),
        families,
        "every family co-fires below the floor, through its own production door"
    );
    for (family, list) in &within {
        assert_eq!(
            *list,
            vec![1, 2, 3, 4, 5, 6, 8],
            "{family}: the measured co-firing ranges moved"
        );
        assert!(list.iter().all(|r| *r < RUNG_EXCLUSIVE_MIN_RANGE));
    }
}

/// The gap legs h-eng-2 probed, at 1, 5 and 10 paisa, through `GapFib::bits`.
#[test]
fn a_one_paisa_gap_fires_six_rungs_a_five_paisa_gap_two_and_ten_none() {
    let rungs = |g: &GapFib, close: i64| fired(g.bits(close, tol()));
    // Up 1 paisa: X2 = GAP_HIGH + 1; rungs 0..=1000 floor onto X2 or X1.
    let one = gap(1, true);
    assert_eq!(rungs(&one, GAP_HIGH), vec![133, 134, 135, 136, 137, 138]);
    assert_eq!(rungs(&one, GAP_HIGH - 1), vec![139, 140, 141]);
    // Up 5 paisa: two pairs share a paisa.
    let five = gap(5, true);
    assert_eq!(rungs(&five, GAP_HIGH + 3), vec![133, 134]);
    assert_eq!(rungs(&five, GAP_HIGH + 1), vec![136, 137]);
    // 10 paisa, both directions: every close fires at most one rung.
    for up in [true, false] {
        let ten = gap(10, up);
        let firing: usize = closes(10)
            .into_iter()
            .map(|close| {
                let n = rungs(&ten, close).len();
                assert!(n <= 1, "10-paisa leg fired {n} rungs at {close}");
                n
            })
            .sum();
        assert_eq!(
            firing, 11,
            "each of the eleven rungs fires on its own paisa"
        );
    }
}

/// The two previous-day ladders are two ladders on one range, and the bound
/// never covered them: down 23.6% and up 78.6% (positions 20 and 70) sit 22
/// thousandths apart against bands summing to 20, so two one-paisa floors close
/// the gap up to 1,000 paisa. Measured exactly, and none past it.
#[test]
fn the_two_previous_day_ladders_fire_together_only_up_to_a_thousand_paisa() {
    let (_, cross) = measure();
    let spans: Vec<(Vec<u32>, i64, i64)> = cross
        .iter()
        .map(|(pair, list)| {
            let lo = list.iter().copied().min().expect("non-empty");
            let hi = list.iter().copied().max().expect("non-empty");
            (pair.clone(), lo, hi)
        })
        .collect();
    assert_eq!(
        spans,
        vec![
            (vec![20, 70], 9, 902),
            (vec![20, 106], 3, 3),
            (vec![21, 70], 4, 10),
            (vec![22, 70], 5, 5),
            (vec![24, 69], 9, 902),
            (vec![26, 69], 2, 3),
        ]
    );
    assert!(cross.values().flatten().all(|r| *r <= 1_000));
}

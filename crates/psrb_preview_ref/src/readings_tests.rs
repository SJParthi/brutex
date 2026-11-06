#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::many_single_char_names,
    reason = "the same exception every test module in this workspace takes: a test that cannot panic cannot fail."
)]

use super::*;
use crate::indicators::SCALE;
use crate::indicators::supertrend::SuperTrend;
use crate::rng::SplitMix64;

fn bar(c: i64) -> Bar {
    Bar::new(c, c + 2, c - 2, c, 10)
}

#[test]
fn t_f12_a_clean_candle_folds_into_all_seven_and_a_refused_one_into_none() {
    let mut bank = Bank::default();
    assert_eq!(bank.step(Some(&bar(1_000)), false), Step::Clean);
    assert_eq!(bank.folded, 1);
    let before = bank.clone();
    let bad = Bar::new(100, 90, 110, 100, 1);
    assert_eq!(
        bank.step(Some(&bad), false),
        Step::Hole(Some(Refusal::Corrupt(Corrupt::Inverted)))
    );
    assert_eq!((bank.st2.reading(), bank.folded), (before.st2.reading(), 1));
    assert_eq!(bank.step(None, true), Step::Hole(None));
    assert_eq!(bank.step(Some(&bar(1_001)), true), Step::FoldedHole);
    assert_eq!(bank.folded, 2);
}

/// X-F12: refusals and the atomic fold at their edges.
#[test]
fn x_f12_bank_extremes() {
    let ceiling = crate::indicators::bollinger::MAX_CLOSE;
    assert_eq!(check(&Bar::flat(ceiling, 1)), Ok(()));
    assert_eq!(
        check(&Bar::flat(ceiling + 1, 1)),
        Err(Refusal::CloseAboveCeiling)
    );
    assert_eq!(
        check(&Bar::flat(i64::MAX, 1)),
        Err(Refusal::CloseAboveCeiling)
    );
    assert_eq!(
        check(&Bar::flat(0, 1)),
        Err(Refusal::Corrupt(Corrupt::NonPositive))
    );
    assert_eq!(
        check(&Bar::flat(-5, 1)),
        Err(Refusal::Corrupt(Corrupt::NonPositive))
    );
    // combo: a refusal on the first candle leaves every reading cold
    let mut first = Bank::new();
    assert!(matches!(
        first.step(Some(&Bar::flat(ceiling + 1, 1)), false),
        Step::Hole(Some(_))
    ));
    for k in KINDS {
        assert!(!first.warm(k));
        assert!(!first.taint.untainted(k));
    }
    // oob: the 2^32 + 1 close would be accepted by every state but the window: the bank
    // pre-checks, so not even the EMA folds it
    let mut ema_only = Bank::new();
    ema_only.step(Some(&Bar::flat(ceiling + 1, 1)), false);
    assert!(!ema_only.ema.warm() && ema_only.ema.scaled() == 0);
    // combo: a refused candle that is also a hole counts as one hole
    let mut both = Bank::new();
    assert!(matches!(
        both.step(Some(&Bar::new(5, 4, 6, 5, 1)), true),
        Step::Hole(Some(_))
    ));
    assert_eq!(both.folded, 0);
}

/// X-F19: the taint counts at their edges.
#[test]
fn x_f19_taint_extremes() {
    let mut t = Taint::default();
    assert!(KINDS.iter().all(|k| t.untainted(*k)));
    t.hole();
    for _ in 0..9 {
        t.clean();
    }
    assert!(!t.untainted(Kind::Supertrend));
    t.clean();
    assert!(t.untainted(Kind::Supertrend) && !t.untainted(Kind::Rsi));
    for _ in 0..4 {
        t.clean();
    }
    assert!(
        t.untainted(Kind::Rsi) && t.untainted(Kind::WilliamsR) && !t.untainted(Kind::Bollinger)
    );
    // a second hole restarts every count
    t.hole();
    assert!(!t.untainted(Kind::Supertrend));
    let mut ema = Taint::default();
    ema.hole();
    for _ in 0..199 {
        ema.clean();
    }
    // combo: only the EMA still blocks after 199 clean candles
    assert!(!ema.untainted(Kind::Ema) && ema.untainted(Kind::Cci));
    ema.clean();
    assert!(ema.untainted(Kind::Ema));
    // oob: every other candle a hole never untaints
    let mut alt = Taint::default();
    for _ in 0..1_000 {
        alt.hole();
        alt.clean();
    }
    assert!(!alt.untainted(Kind::Supertrend));
    assert_eq!(KINDS.map(Kind::taint_period), [10, 20, 14, 14, 20, 200]);
    assert_eq!(KINDS.map(Kind::index), [0, 1, 2, 3, 4, 5]);
    assert_eq!(
        KINDS.map(Kind::label),
        ["ST", "BB", "RSI", "WR", "CCI", "EMA"]
    );
}

#[test]
fn t_f12_warmth_per_reading() {
    let mut bank = Bank::new();
    let mut warm_at = [0usize; 6];
    for i in 1..=201 {
        bank.step(Some(&bar(1_000 + i64::try_from(i % 7).unwrap())), false);
        for (k, w) in KINDS.iter().zip(warm_at.iter_mut()) {
            if *w == 0 && bank.warm(*k) {
                *w = i;
            }
        }
    }
    assert_eq!(warm_at, [10, 20, 15, 14, 20, 200]);
}

/// The naive streaming models of P-F12-1 and P-F13..P-F18-1: every state recomputed from
/// the whole accepted history at every candle, by the published definitions written
/// again in their simplest form.
mod naive {
    use super::*;

    pub(super) fn atr(history: &[Bar], period: usize) -> Option<i128> {
        if history.len() < period {
            return None;
        }
        let tr = |i: usize| -> i128 {
            let b = &history[i];
            let span = i128::from(b.high - b.low);
            if i == 0 {
                return span;
            }
            let pc = i128::from(history[i - 1].close);
            span.max((i128::from(b.high) - pc).abs())
                .max((i128::from(b.low) - pc).abs())
        };
        let n = i128::try_from(period).unwrap();
        let mut v: i128 = (0..period)
            .map(|i| tr(i) * SCALE)
            .sum::<i128>()
            .div_euclid(n);
        for i in period..history.len() {
            v += (tr(i) * SCALE - v).div_euclid(n);
        }
        Some(v.div_euclid(SCALE))
    }

    pub(super) fn bb_above(closes: &[i64], z: i128) -> Option<bool> {
        let w = closes.get(closes.len().checked_sub(20)?..)?;
        let n = 20i128;
        let s: i128 = w.iter().map(|c| i128::from(*c)).sum();
        let q: i128 = w.iter().map(|c| i128::from(*c) * i128::from(*c)).sum();
        let d = n * i128::from(*w.last()?) - s;
        Some(d > 0 && (1_000 * d) * (1_000 * d) > z * z * (n * q - s * s))
    }

    pub(super) fn willr(history: &[Bar]) -> Option<(i64, i64)> {
        let w = history.get(history.len().checked_sub(14)?..)?;
        Some((
            w.iter().map(|b| b.high).max()?,
            w.iter().map(|b| b.low).min()?,
        ))
    }

    pub(super) fn cci_above(history: &[Bar], milli: i128) -> Option<bool> {
        let w = history.get(history.len().checked_sub(20)?..)?;
        // Exact rationals in thirds: TP = T/3, SMA = S/(3n), MD = sum|TP - SMA| / n.
        // CCI > m/1000 iff 1000 (TP - SMA) > m x 0.015 x MD, scaled by 3n x 200 x n.
        let n = 20i128;
        let t: Vec<i128> = w
            .iter()
            .map(|b| i128::from(b.high) + i128::from(b.low) + i128::from(b.close))
            .collect();
        let s: i128 = t.iter().sum();
        let md_num: i128 = t.iter().map(|x| (n * x - s).abs()).sum();
        let dev = n * t.last()? - s;
        Some(md_num > 0 && 200_000 * n * dev > 3 * milli * md_num)
    }
}

/// P-F12-1: generated streams with planted refusals; every state equals its naive model.
#[test]
fn p_f12_bank_vs_naive_states() {
    for seed in crate::generator::SEEDS {
        let mut r = SplitMix64::new(seed);
        for _ in 0..25 {
            let mut bank = Bank::new();
            let mut st2 = SuperTrend::new(10, 2_000);
            let mut accepted: Vec<Bar> = Vec::new();
            let mut price = 100_000i64;
            for _ in 0..100 {
                price = (price + i64::try_from(r.below(401)).unwrap() - 200).max(1_000);
                let h = price + i64::try_from(r.below(50)).unwrap();
                let l = price - i64::try_from(r.below(50)).unwrap();
                let b = if r.chance(1, 25) {
                    Bar::new(price, l, h, price, 1)
                } else {
                    Bar::new(price, h, l, price, 10)
                };
                let step = bank.step(Some(&b), false);
                if step == Step::Clean {
                    accepted.push(b);
                    st2.fold(&b);
                }
                assert_eq!(
                    bank.st2.atr(),
                    naive::atr(&accepted, 10).map(|v| i64::try_from(v).unwrap())
                );
                assert_eq!(bank.st2.reading(), st2.reading());
                let closes: Vec<i64> = accepted.iter().map(|b| b.close).collect();
                assert_eq!(
                    bank.bb.window().map(|w| w.above_upper(2_000)),
                    naive::bb_above(&closes, 2_000)
                );
                assert_eq!(
                    bank.wr.reading().map(|w| (w.highest, w.lowest)),
                    naive::willr(&accepted)
                );
                assert_eq!(
                    bank.cci.reading().map(|c| c.above(200_000)),
                    naive::cci_above(&accepted, 200_000)
                );
                assert_eq!(
                    bank.cci.reading().map(|c| c.above(-100_000)),
                    naive::cci_above(&accepted, -100_000)
                );
            }
        }
    }
}

/// P-F19-1: taint against a look-back model (a reading is clean when its last `period`
/// events hold no hole, or no hole ever happened).
#[test]
fn p_f19_taint_vs_lookback_model() {
    for seed in crate::generator::SEEDS {
        let mut r = SplitMix64::new(seed);
        for _ in 0..25 {
            let mut t = Taint::default();
            let mut events: Vec<bool> = Vec::new();
            for _ in 0..400 {
                let hole = r.chance(1, 30);
                events.push(hole);
                if hole {
                    t.hole();
                } else {
                    t.clean();
                }
                for k in KINDS {
                    let p = usize::try_from(k.taint_period()).unwrap();
                    let last_hole = events.iter().rposition(|h| *h);
                    let want = last_hole.is_none_or(|i| events.len() - 1 - i >= p);
                    assert_eq!(t.untainted(k), want);
                }
            }
        }
    }
}

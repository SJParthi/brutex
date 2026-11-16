//! Candle anatomy and patterns (AN-COL, AN-BODY, AN-WAG / AN-WWI, VB-RNG, PT, AN-DOJI, VB-NR), RAW, same-session (K15).
//!
//! Patterns are a v1 reading of the sniper's named list (SNIPER-ADANIENT-PLAN C-PAT: marubozu, engulfing, hammer /
//! shooting star, inside, outside, doji, NR4, NR7) with every threshold a data row ([`PatCfg`]). They are NOT a lift of
//! `indicators::pattern`'s 62 positions: the equality with that crate is an open item of v1 (F-G2), named in LIFT.md.
//! On a session's first candles a pattern that needs earlier candles is cold (`pattern_cold_same_session`): `None`.

use std::collections::VecDeque;

use crate::fold::Candle;
use crate::num::Ratio;

/// The pattern thresholds (FEATURES rows; percent of the candle's range).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PatCfg {
    /// Marubozu: body >= this % of the range.
    pub maru_body_pct: i128,
    /// Doji: body <= this % of the range.
    pub doji_body_pct: i128,
    /// Hammer / shooting star: the long wick >= this many times the body (x100: 200 = 2x).
    pub hammer_wick_x100: i128,
    /// Hammer / shooting star: the short wick <= this % of the range.
    pub hammer_short_wick_pct: i128,
}

impl Default for PatCfg {
    fn default() -> Self {
        Self { maru_body_pct: 95, doji_body_pct: 10, hammer_wick_x100: 200, hammer_short_wick_pct: 10 }
    }
}

/// Pattern bits (one candle). Long-side names; each pattern's bit carries its own side in [`Anatomy::pat_side`].
pub mod pat {
    /// Marubozu (green or red: the side is the colour).
    pub const MARUBOZU: u16 = 1;
    /// Bullish engulfing (the short side: bearish engulfing, in `pat_short`).
    pub const ENGULFING: u16 = 1 << 1;
    /// Hammer (the short side: shooting star).
    pub const HAMMER: u16 = 1 << 2;
    /// Inside bar (side-free).
    pub const INSIDE: u16 = 1 << 3;
    /// Outside bar (side-free).
    pub const OUTSIDE: u16 = 1 << 4;
    /// Doji (side-free).
    pub const DOJI: u16 = 1 << 5;
    /// NR4: the strictly smallest range of the last 4 same-session candles.
    pub const NR4: u16 = 1 << 6;
    /// NR7.
    pub const NR7: u16 = 1 << 7;
}

/// The anatomy of one candle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Anatomy {
    /// +1 green (close > open), -1 red, 0 flat.
    pub colour: i8,
    /// High - low (paisa).
    pub range: i64,
    /// |close - open|.
    pub body: i64,
    /// high - max(open, close).
    pub upper_wick: i64,
    /// min(open, close) - low.
    pub lower_wick: i64,
    /// Body as a percent of the range: 100 body / range (`None` when the range is 0).
    pub body_pct: Option<Ratio>,
    /// Upper wick as a percent of the range.
    pub upper_pct: Option<Ratio>,
    /// Lower wick as a percent of the range.
    pub lower_pct: Option<Ratio>,
    /// Long-side pattern bits (with `pat_cold` for the ones that could not be read).
    pub pat_long: u16,
    /// Short-side pattern bits (marubozu red, bearish engulfing, shooting star, and the side-free ones).
    pub pat_short: u16,
    /// Patterns that are cold on this candle (they need earlier same-session candles).
    pub pat_cold: u16,
}

/// The same-session ring of the last candles (K15), and the anatomy fold.
#[derive(Clone, Debug)]
pub struct AnatomyState {
    cfg: PatCfg,
    ring: VecDeque<(i64, i64, i64, i64)>,
}

impl AnatomyState {
    /// A new state.
    #[must_use]
    pub fn new(cfg: PatCfg) -> Self {
        Self { cfg, ring: VecDeque::with_capacity(7) }
    }

    /// A new session starts: the ring is cleared (K15). Under the mutant `m_pattern_cross` it is carried.
    pub fn session(&mut self) {
        if !cfg!(feature = "m_pattern_cross") {
            self.ring.clear();
        }
    }

    /// Folds one candle. Returns its anatomy and the step count (<= 12).
    #[allow(clippy::many_single_char_names, reason = "o, h, l, x are the candle's open, high, low, close as in REQ72-DESIGN A.4")]
    pub fn fold(&mut self, c: &Candle) -> (Anatomy, u32) {
        let cfg = self.cfg;
        let (o, h, l, x) = (i128::from(c.open), i128::from(c.high), i128::from(c.low), i128::from(c.close));
        let range = h - l;
        let body = (x - o).abs();
        let uw = h - o.max(x);
        let lw = o.min(x) - l;
        let colour: i8 = match x.cmp(&o) {
            std::cmp::Ordering::Greater => 1,
            std::cmp::Ordering::Less => -1,
            std::cmp::Ordering::Equal => 0,
        };
        let pct = |v: i128| Ratio::new(100 * v, range);
        let mut long = 0u16;
        let mut short = 0u16;
        let mut cold = 0u16;
        if range > 0 {
            if 100 * body >= cfg.maru_body_pct * range {
                if colour > 0 {
                    long |= pat::MARUBOZU;
                } else if colour < 0 {
                    short |= pat::MARUBOZU;
                }
            }
            if 100 * body <= cfg.doji_body_pct * range {
                long |= pat::DOJI;
                short |= pat::DOJI;
            }
            // hammer: long lower wick, short upper wick; shooting star: the mirror.
            if 100 * lw >= cfg.hammer_wick_x100 * body && 100 * uw <= cfg.hammer_short_wick_pct * range && body > 0 {
                long |= pat::HAMMER;
            }
            if 100 * uw >= cfg.hammer_wick_x100 * body && 100 * lw <= cfg.hammer_short_wick_pct * range && body > 0 {
                short |= pat::HAMMER;
            }
        }
        let mut steps = 6u32;
        match self.ring.back() {
            Some(&(po, ph, pl, px)) => {
                let (po, ph, pl, px) = (i128::from(po), i128::from(ph), i128::from(pl), i128::from(px));
                if px < po && x > o && o <= px && x >= po && (o < px || x > po) {
                    long |= pat::ENGULFING;
                }
                if px > po && x < o && o >= px && x <= po && (o > px || x < po) {
                    short |= pat::ENGULFING;
                }
                if h <= ph && l >= pl {
                    long |= pat::INSIDE;
                    short |= pat::INSIDE;
                }
                if h > ph && l < pl {
                    long |= pat::OUTSIDE;
                    short |= pat::OUTSIDE;
                }
                steps += 2;
            }
            None => cold |= pat::ENGULFING | pat::INSIDE | pat::OUTSIDE,
        }
        // NR4 / NR7 over this candle and the 3 / 6 before it in the session.
        for (k, bit) in [(3usize, pat::NR4), (6usize, pat::NR7)] {
            if self.ring.len() < k {
                cold |= bit;
                continue;
            }
            let start = self.ring.len() - k;
            let smallest = self.ring.iter().skip(start).all(|&(_, ph, pl, _)| range < i128::from(ph) - i128::from(pl));
            steps += u32::try_from(k).unwrap_or(0);
            if smallest {
                long |= bit;
                short |= bit;
            }
        }
        if self.ring.len() == 7 {
            self.ring.pop_front();
        }
        self.ring.push_back((c.open, c.high, c.low, c.close));
        let a = Anatomy {
            colour,
            range: c.high - c.low,
            body: (c.close - c.open).abs(),
            upper_wick: i64::try_from(uw).unwrap_or(0),
            lower_wick: i64::try_from(lw).unwrap_or(0),
            body_pct: pct(body),
            upper_pct: pct(uw),
            lower_pct: pct(lw),
            pat_long: long,
            pat_short: short,
            pat_cold: cold,
        };
        (a, steps)
    }
}

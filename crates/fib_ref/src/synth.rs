//! The seeded synthetic generator: every minute and second of every golden
//! scenario comes from here, never from a vendor.
//!
//! `SplitMix64` (Steele, Lea and Flood's mixer, the same constants
//! everywhere) drives a tick-grid random walk for minutes and a path inside
//! each minute for its GDFL seconds: the first traded second opens at the
//! minute's open, one second prints the minute's high and one its low, the
//! last closes at its close. Patterns are laid on top by explicit minute
//! scripts; the generator fills everything around them.

use crate::bars::{Minute, Sec};
use crate::clock::minute_start;

/// The 64-bit `SplitMix` generator.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SplitMix64(u64);

impl SplitMix64 {
    /// A generator seeded with `seed`.
    pub fn new(seed: u64) -> SplitMix64 {
        SplitMix64(seed)
    }

    /// The next 64 bits.
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A value in `0..n` (`n > 0`; modulo bias accepted: determinism is the
    /// point, not uniformity).
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n.max(1)
    }

    /// A value in `lo..=hi`.
    pub fn range(&mut self, lo: i64, hi: i64) -> i64 {
        if hi <= lo {
            return lo;
        }
        let span = i128::from(hi) - i128::from(lo) + 1;
        let r = i128::from(self.next_u64()) % span;
        i64::try_from(i128::from(lo) + r).unwrap_or(lo)
    }

    /// `true` with probability `permille / 1000`.
    pub fn chance(&mut self, permille: u64) -> bool {
        self.below(1000) < permille
    }
}

/// One scripted minute in ticks relative to a base price: `(open, high,
/// low, close)` offsets.
pub type Script = (i64, i64, i64, i64);

/// Minutes from a script: each entry is `(o, h, l, c)` in ticks above
/// `base`, with the step in paise; volume and the GDFL close come from the
/// generator (G equals the close: the feeds agree).
pub fn scripted(
    rng: &mut SplitMix64,
    base: i64,
    step: i64,
    script: &[Script],
    first_minute: u32,
) -> Vec<Minute> {
    script
        .iter()
        .enumerate()
        .map(|(k, &(o, h, l, c))| {
            let m = first_minute + u32::try_from(k).unwrap_or(0);
            Minute {
                m,
                o: base + o * step,
                h: base + h * step,
                l: base + l * step,
                c: base + c * step,
                v: rng.range(500, 5_000),
                held: true,
                padded: false,
                g: Some(base + c * step),
            }
        })
        .collect()
}

/// A random walk of `n` minutes from `start` (paise) on the tick grid:
/// each close moves by `drift` ticks plus up to `vol` ticks either way, each
/// wick up to `wick` ticks.
#[allow(
    clippy::too_many_arguments,
    reason = "a walk's shape is its eight dials"
)]
pub fn random_minutes(
    rng: &mut SplitMix64,
    start: i64,
    step: i64,
    n: u32,
    first_minute: u32,
    drift: i64,
    vol: i64,
    wick: i64,
) -> Vec<Minute> {
    let mut out = Vec::with_capacity(n as usize);
    let mut px = start;
    for k in 0..n {
        let o = px;
        let c = (o + step * (drift + rng.range(-vol, vol))).max(step);
        let h = o.max(c) + step * rng.range(0, wick);
        let l = (o.min(c) - step * rng.range(0, wick)).max(step);
        out.push(Minute {
            m: first_minute + k,
            o,
            h,
            l,
            c,
            v: rng.range(500, 5_000),
            held: true,
            padded: false,
            g: Some(c),
        });
        px = c;
    }
    out
}

/// GDFL seconds inside one minute, consistent with it: traded seconds with
/// probability `permille`, the first opening at the minute's open, one at its
/// high, one at its low, the last closing at its close, volumes in
/// `1..=vmax`. A minute that Zerodha does not hold still gets seconds (GDFL
/// can print where Zerodha holds a hole).
pub fn seconds_for(
    rng: &mut SplitMix64,
    m: &Minute,
    step: i64,
    permille: u64,
    vmax: i64,
) -> Vec<Sec> {
    let t0 = minute_start(m.m);
    let mut offs: Vec<u32> = (0..60).filter(|_| rng.chance(permille)).collect();
    if offs.len() < 3 {
        offs = vec![0, 29, 59];
    }
    let n = offs.len();
    let i_hi = usize::try_from(rng.below(n as u64)).unwrap_or(0);
    let mut i_lo = usize::try_from(rng.below(n as u64)).unwrap_or(0);
    if i_lo == i_hi {
        i_lo = (i_hi + 1) % n;
    }
    let ticks = ((m.h - m.l) / step.max(1)).max(0);
    let mut prev = m.o;
    let mut out = Vec::with_capacity(n);
    for (k, &off) in offs.iter().enumerate() {
        let close = if k + 1 == n {
            m.c
        } else if k == i_hi {
            m.h
        } else if k == i_lo {
            m.l
        } else {
            m.l + step * rng.range(0, ticks)
        };
        let mut h = prev.max(close);
        let mut l = prev.min(close);
        if k == i_hi {
            h = m.h;
        }
        if k == i_lo {
            l = m.l;
        }
        out.push(Sec {
            t: t0 + off,
            o: prev,
            h,
            l,
            c: close,
            v: rng.range(1, vmax),
        });
        prev = close;
    }
    out
}

/// Seconds for a run of minutes.
pub fn seconds_for_all(
    rng: &mut SplitMix64,
    minutes: &[Minute],
    step: i64,
    permille: u64,
    vmax: i64,
) -> Vec<Sec> {
    minutes
        .iter()
        .flat_map(|m| seconds_for(rng, m, step, permille, vmax))
        .collect()
}

#[cfg(test)]
#[path = "synth_tests.rs"]
mod tests;

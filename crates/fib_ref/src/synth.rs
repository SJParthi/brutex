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
pub fn scripted(rng: &mut SplitMix64, base: i64, step: i64, script: &[Script], first_minute: u32) -> Vec<Minute> {
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
        out.push(Minute { m: first_minute + k, o, h, l, c, v: rng.range(500, 5_000), held: true, padded: false, g: Some(c) });
        px = c;
    }
    out
}

/// GDFL seconds inside one minute, consistent with it: traded seconds with
/// probability `permille`, the first opening at the minute's open, one at its
/// high, one at its low, the last closing at its close, volumes in
/// `1..=vmax`. A minute that Zerodha does not hold still gets seconds (GDFL
/// can print where Zerodha holds a hole).
pub fn seconds_for(rng: &mut SplitMix64, m: &Minute, step: i64, permille: u64, vmax: i64) -> Vec<Sec> {
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
        out.push(Sec { t: t0 + off, o: prev, h, l, c: close, v: rng.range(1, vmax) });
        prev = close;
    }
    out
}

/// Seconds for a run of minutes.
pub fn seconds_for_all(rng: &mut SplitMix64, minutes: &[Minute], step: i64, permille: u64, vmax: i64) -> Vec<Sec> {
    minutes.iter().flat_map(|m| seconds_for(rng, m, step, permille, vmax)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splitmix_is_the_reference_sequence() {
        // the first outputs of SplitMix64 seeded with 0 (widely published)
        let mut r = SplitMix64::new(0);
        assert_eq!(r.next_u64(), 0xE220_A839_7B1D_CDAF);
        assert_eq!(r.next_u64(), 0x6E78_9E6A_A1B9_65F4);
        assert_eq!(r.next_u64(), 0x06C4_5D18_8009_454F);
        let mut a = SplitMix64::new(42);
        let mut b = SplitMix64::new(42);
        for _ in 0..100 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn range_below_and_chance_stay_in_bounds() {
        let mut r = SplitMix64::new(7);
        for _ in 0..1000 {
            let x = r.range(-3, 5);
            assert!((-3..=5).contains(&x));
            assert!(r.below(10) < 10);
        }
        assert_eq!(r.range(5, 5), 5);
        assert_eq!(r.range(9, 2), 9);
        assert_eq!(r.below(0), 0);
        assert!(!r.chance(0));
        assert!(r.chance(1000));
        let full = r.range(i64::MIN, i64::MAX);
        let _ = full;
    }

    #[test]
    fn seconds_stay_inside_their_minute_and_hit_its_extremes() {
        let mut r = SplitMix64::new(11);
        let minutes = random_minutes(&mut r, 100_000, 5, 50, 0, 0, 4, 3);
        assert_eq!(minutes.len(), 50);
        for m in &minutes {
            assert!(m.l <= m.o.min(m.c) && m.h >= m.o.max(m.c));
            for permille in [0, 300, 1000] {
                let secs = seconds_for(&mut r, m, 5, permille, 100);
                assert!(secs.len() >= 3);
                assert_eq!(secs[0].o, m.o);
                assert_eq!(secs.last().map(|s| s.c), Some(m.c));
                assert_eq!(secs.iter().map(|s| s.h).max(), Some(m.h));
                assert_eq!(secs.iter().map(|s| s.l).min(), Some(m.l));
                assert!(secs.windows(2).all(|w| w[0].t < w[1].t));
                assert!(secs.iter().all(|s| s.l <= s.h && s.v >= 1));
            }
        }
        let all = seconds_for_all(&mut r, &minutes[..3], 5, 500, 10);
        assert!(all.len() >= 9);
    }

    #[test]
    fn scripts_place_prices_on_the_grid() {
        let mut r = SplitMix64::new(3);
        let m = scripted(&mut r, 10_000, 5, &[(0, 4, -2, 3), (3, 3, 3, 3)], 10);
        assert_eq!((m[0].m, m[0].o, m[0].h, m[0].l, m[0].c), (10, 10_000, 10_020, 9_990, 10_015));
        assert_eq!(m[0].g, Some(10_015));
        assert_eq!(m[1].m, 11);
        // a walk that would go below zero stays at one step
        let low = random_minutes(&mut r, 5, 5, 20, 0, -10, 1, 1);
        assert!(low.iter().all(|x| x.l >= 5 && x.c >= 5));
    }
}

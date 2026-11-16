//! The random-entry twin's draws (plan 9, F10; D-S23): for each world (draw) r and each calendar year with m entries of the pick,
//! the m sessions of that year with the SMALLEST keys `splitmix(seed, r, session)` are drawn, each at the candle
//! `1 + splitmix(key) mod (its decision candles)`. A session's key depends only on (seed, world, session), so appending a day never
//! moves an earlier draw, and the draw set is a pure function of the final data: a day-by-day append and one full run give the
//! same draws byte for byte (G-INC; this answers the plan's open S9 x F10 point for the year still open).

use std::collections::BTreeMap;

use sniper_kernel::scale::splitmix;

/// One session a twin may draw from: (session index, year, decision candles of the context that day).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Slot {
    /// The session index.
    pub sess: u32,
    /// Its calendar year.
    pub year: i32,
    /// Decision candles (>= 1).
    pub candles: u16,
}

/// The session key of (seed, world, session).
#[must_use]
pub fn key(seed: u64, world: u32, sess: u32) -> u64 {
    splitmix(splitmix(seed ^ (u64::from(world) << 32)) ^ u64::from(sess))
}

/// World `world`'s draws: per year the `per_year[year]` sessions with the smallest keys, each with its candle. Sorted by session.
#[must_use]
pub fn draws(seed: u64, world: u32, slots: &[Slot], per_year: &BTreeMap<i32, u32>) -> Vec<(u32, u16)> {
    let mut by_year: BTreeMap<i32, Vec<(u64, Slot)>> = BTreeMap::new();
    for s in slots {
        by_year.entry(s.year).or_default().push((key(seed, world, s.sess), *s));
    }
    let mut out = Vec::new();
    for (y, mut v) in by_year {
        let m = per_year.get(&y).copied().unwrap_or(0) as usize;
        v.sort_unstable_by_key(|x| (x.0, x.1.sess));
        for (k, s) in v.into_iter().take(m) {
            let c = 1 + u16::try_from(splitmix(k) % u64::from(s.candles.max(1))).unwrap_or(0);
            out.push((s.sess, c));
        }
    }
    out.sort_unstable();
    out
}

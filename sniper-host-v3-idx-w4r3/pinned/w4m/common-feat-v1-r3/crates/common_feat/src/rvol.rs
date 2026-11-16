//! Relative volume VO-RVOL (SNIPER-ADANIENT-PLAN V-RVOL, F1; K14): the candle's volume against the same candle's mean
//! over the last `m` regular full sessions PRESENT.
//!
//! - The decision session's own candle k is never in its mean (it is read first, then pushed).
//! - A special or short session is neither pushed nor read (the host says `regular_full = false`).
//! - A session where candle k is absent pushes nothing; the divisor is the count of sessions present.
//!
//! One ring per candle index k (fixed capacity m), so a read and a push are O(1).

use std::collections::VecDeque;

use crate::num::Ratio;

/// The per-candle-index rings.
#[derive(Clone, Debug)]
pub struct Rvol {
    m: usize,
    need: usize,
    rings: Vec<(VecDeque<u64>, u128)>,
}

impl Rvol {
    /// Rings for candle indices 1..=`kmax`, each of the last `m` sessions; ready when `need` sessions are present.
    #[must_use]
    pub fn new(m: usize, need: usize, kmax: usize) -> Self {
        let m = m.max(1);
        Self { m, need: need.clamp(1, m), rings: (0..kmax.max(1)).map(|_| (VecDeque::with_capacity(m), 0)).collect() }
    }

    /// Reads candle `k`'s relative volume `v / mean` as the exact ratio `v cnt / sum`, then (on a regular full
    /// session) pushes `v`. Returns the reading and the step count (<= 4).
    pub fn fold(&mut self, k: i64, v: u64, regular_full: bool) -> (Option<Ratio>, u32) {
        let Some(slot) = usize::try_from(k - 1).ok().and_then(|i| self.rings.get_mut(i)) else {
            return (None, 1);
        };
        let push = |slot: &mut (VecDeque<u64>, u128), m: usize| {
            if slot.0.len() == m
                && let Some(old) = slot.0.pop_front()
            {
                slot.1 -= u128::from(old);
            }
            slot.0.push_back(v);
            slot.1 += u128::from(v);
        };
        if cfg!(feature = "m_rvol_self") && regular_full {
            push(slot, self.m);
        }
        let reading = if regular_full && slot.0.len() >= self.need && slot.1 > 0 {
            let cnt = i128::try_from(slot.0.len()).unwrap_or(0);
            Ratio::new(i128::from(v) * cnt, i128::try_from(slot.1).unwrap_or(i128::MAX))
        } else {
            None
        };
        if !cfg!(feature = "m_rvol_self") && regular_full {
            push(slot, self.m);
        }
        (reading, 4)
    }
}

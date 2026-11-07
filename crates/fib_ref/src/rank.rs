//! The top-mover clause: the rank of a stock's move at a moment
//! (PREVIEW-SPEC 5.1, 5.3).
//!
//! `m(s, T) = (P - C) / C`, with `P` the close of the stock's latest 1-minute
//! candle closed by `T` and `C` the previous session's close (either basis of
//! S10; the caller passes the basis it reads). The pool is every stock with a
//! candle closed by `T` and a previous close. **Competition ranks**: equal
//! moves share a rank, and a tie at the edge of top N includes every tied
//! stock. A LONG reads gainer ranks, a SHORT loser ranks.
//!
//! Moves compare exactly by cross-multiplication in `i128`; no ratio is ever
//! rounded. Building a frame sorts the pool, `O(P log P)` for a pool of `P`
//! (at most 218): this step is **not O(1)** and is named so in the benches.
//! A lookup in a built frame is O(1).

use std::cmp::Ordering;

/// How a gainer or loser is read (ambiguity: "top 10 gainers").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoverSign {
    /// Rank only: when fewer than N stocks are up, a falling stock can be in
    /// the top N gainers (the competition-rank reading, headline).
    RankOnly,
    /// A gainer must have `m > 0` and a loser `m < 0`.
    SignedOnly,
}

/// One stock's inputs at the moment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mover {
    /// Symbol order key (S81 (d)); also the stock's identity.
    pub sym: u32,
    /// `P`, paise.
    pub p: i64,
    /// `C`, paise, at least 1.
    pub c: i64,
    /// Minutes since the stock's latest 1-minute candle closed (0 = fresh).
    pub stale_min: u32,
}

/// The ranks of one stock in a frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ranked {
    /// Symbol.
    pub sym: u32,
    /// Competition rank among gainers (1 = the largest move).
    pub gainer: u32,
    /// Competition rank among losers (1 = the most negative move).
    pub loser: u32,
    /// `sign(P - C)`: 1, 0 or -1.
    pub sign: i8,
    /// Carried stale price, flagged (S15's seed: carry forward, flagged).
    pub stale: bool,
}

impl Ranked {
    /// In the long pool's top `n` under a sign reading.
    pub fn in_top_long(&self, n: u32, sign: MoverSign) -> bool {
        self.gainer <= n && (sign == MoverSign::RankOnly || self.sign > 0)
    }

    /// In the short pool's top `n` under a sign reading.
    pub fn in_top_short(&self, n: u32, sign: MoverSign) -> bool {
        self.loser <= n && (sign == MoverSign::RankOnly || self.sign < 0)
    }
}

/// Exact comparison of two moves, `m(a)` against `m(b)`.
pub fn cmp_move(a: &Mover, b: &Mover) -> Ordering {
    let left = i128::from(a.p - a.c) * i128::from(b.c);
    let right = i128::from(b.p - b.c) * i128::from(a.c);
    left.cmp(&right)
}

/// Ranks a pool. A stock whose `stale_min` exceeds `stale_limit` (when one is
/// set; S15's generated limit) leaves the pool; a stock with `c <= 0` has no
/// valid previous close (M11) and leaves it too. The result is in the pool's
/// input order, without the stocks that left.
pub fn frame(pool: &[Mover], stale_limit: Option<u32>) -> Vec<Ranked> {
    let kept: Vec<Mover> = pool
        .iter()
        .copied()
        .filter(|s| s.c > 0 && stale_limit.is_none_or(|lim| s.stale_min <= lim))
        .collect();
    let mut order: Vec<usize> = (0..kept.len()).collect();
    // descending by move: the gainers' order. Equal moves share one rank,
    // so their order inside the sort is never seen.
    order.sort_by(|&a, &b| cmp_move(&kept[b], &kept[a]));
    let mut gainer = vec![0u32; kept.len()];
    let mut loser = vec![0u32; kept.len()];
    let n = kept.len();
    let mut start = 0;
    while start < n {
        let mut end = start + 1;
        while end < n && cmp_move(&kept[order[end]], &kept[order[start]]) == Ordering::Equal {
            end += 1;
        }
        // competition ranks: the group's gainer rank is its first position + 1;
        // its loser rank counts the strictly smaller moves after the group.
        let g = u32::try_from(start + 1).unwrap_or(u32::MAX);
        let l = u32::try_from(n - end + 1).unwrap_or(u32::MAX);
        for &ix in &order[start..end] {
            gainer[ix] = g;
            loser[ix] = l;
        }
        start = end;
    }
    kept.iter()
        .enumerate()
        .map(|(ix, s)| Ranked {
            sym: s.sym,
            gainer: gainer[ix],
            loser: loser[ix],
            sign: match s.p.cmp(&s.c) {
                Ordering::Greater => 1,
                Ordering::Equal => 0,
                Ordering::Less => -1,
            },
            stale: s.stale_min > 0,
        })
        .collect()
}

#[cfg(test)]
#[path = "rank_tests.rs"]
mod tests;

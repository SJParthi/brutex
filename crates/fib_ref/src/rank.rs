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
    // descending by move: the gainers' order
    order.sort_by(|&a, &b| cmp_move(&kept[b], &kept[a]).then(kept[a].sym.cmp(&kept[b].sym)));
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
mod tests {
    use super::*;

    fn mv(sym: u32, p: i64, c: i64) -> Mover {
        Mover {
            sym,
            p,
            c,
            stale_min: 0,
        }
    }

    fn find(f: &[Ranked], sym: u32) -> Option<Ranked> {
        f.iter().copied().find(|r| r.sym == sym)
    }

    #[test]
    fn competition_ranks_share_ties_and_skip() {
        // moves: +10%, +5%, +5%, 0, -2%, -2%, -9%
        let pool = [
            mv(1, 110, 100),
            mv(2, 105, 100),
            mv(3, 210, 200),
            mv(4, 50, 50),
            mv(5, 98, 100),
            mv(6, 49, 50),
            mv(7, 91, 100),
        ];
        let f = frame(&pool, None);
        let g: Vec<u32> = f.iter().map(|r| r.gainer).collect();
        let l: Vec<u32> = f.iter().map(|r| r.loser).collect();
        assert_eq!(g, [1, 2, 2, 4, 5, 5, 7]);
        assert_eq!(l, [7, 5, 5, 4, 2, 2, 1]);
        // the edge tie: top 2 holds three stocks
        let top2: Vec<u32> = f
            .iter()
            .filter(|r| r.in_top_long(2, MoverSign::RankOnly))
            .map(|r| r.sym)
            .collect();
        assert_eq!(top2, [1, 2, 3]);
        let bottom2: Vec<u32> = f
            .iter()
            .filter(|r| r.in_top_short(2, MoverSign::RankOnly))
            .map(|r| r.sym)
            .collect();
        assert_eq!(bottom2, [5, 6, 7]);
    }

    #[test]
    fn the_sign_reading_differs_only_on_a_falling_market() {
        let pool = [mv(1, 99, 100), mv(2, 98, 100), mv(3, 100, 100)];
        let f = frame(&pool, None);
        assert!(find(&f, 1).is_some_and(|r| r.in_top_long(10, MoverSign::RankOnly)));
        assert!(find(&f, 1).is_some_and(|r| !r.in_top_long(10, MoverSign::SignedOnly)));
        assert!(find(&f, 3).is_some_and(|r| !r.in_top_short(10, MoverSign::SignedOnly)));
        assert!(find(&f, 3).is_some_and(|r| r.in_top_short(10, MoverSign::RankOnly)));
        assert!(find(&f, 2).is_some_and(|r| r.in_top_short(1, MoverSign::SignedOnly)));
        assert_eq!(find(&f, 3).map(|r| r.sign), Some(0));
    }

    #[test]
    fn exact_cross_multiplication_separates_near_moves() {
        // 1/3 against 333/1000: equal as floats to 3 digits, not exactly
        let a = mv(1, 4, 3);
        let b = mv(2, 1333, 1000);
        assert_eq!(cmp_move(&a, &b), Ordering::Greater);
        let c = mv(3, 2_000_000, 1_500_000); // exactly 1/3
        assert_eq!(cmp_move(&a, &c), Ordering::Equal);
        // the largest paise the store holds do not overflow
        let big = mv(4, 2 * (i64::MAX / 4), i64::MAX / 4);
        let small = mv(5, 2, 1);
        assert_eq!(cmp_move(&big, &small), Ordering::Equal);
    }

    #[test]
    fn stale_and_invalid_stocks_leave_the_pool() {
        let pool = [
            Mover {
                stale_min: 30,
                ..mv(1, 120, 100)
            },
            mv(2, 105, 100),
            mv(3, 105, 0), // no previous close
        ];
        let carried = frame(&pool, None);
        assert_eq!(carried.len(), 2);
        assert!(find(&carried, 1).is_some_and(|r| r.stale && r.gainer == 1));
        let limited = frame(&pool, Some(15));
        assert_eq!(limited.len(), 1);
        assert_eq!(find(&limited, 2).map(|r| r.gainer), Some(1));
        assert!(frame(&[], None).is_empty());
        assert_eq!(frame(&pool, Some(30)).len(), 2);
    }

    #[test]
    fn a_single_stock_is_first_both_ways() {
        let f = frame(&[mv(9, 100, 100)], None);
        assert_eq!((f[0].gainer, f[0].loser), (1, 1));
        let all_equal = frame(&[mv(1, 1, 1), mv(2, 2, 2), mv(3, 3, 3)], None);
        assert!(all_equal.iter().all(|r| r.gainer == 1 && r.loser == 1));
    }
}

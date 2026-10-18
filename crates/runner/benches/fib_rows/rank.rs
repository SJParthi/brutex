//! The bench rows of the mover rank (FIB-2; D-6322, D-6323).
//!
//! One call folds one Zerodha minute over a pool. The minute is O(pool +
//! buckets) by construction — the same work at every minute of a day — so the
//! per-minute row is declared BOUNDED and the claim FM-F010 makes is PER
//! MEMBER: the per-minute times over the pool, at three pools spanning more
//! than 100x, p99 and max ratios within 1.25x. The specification's four pools
//! (16, 128, 256, 512) are printed per minute beside them. A day's rank state
//! is sized once at the run's largest pool — a few KiB at the production pool
//! of about 208 members, cache-resident in production too — so the axis is
//! FIXED-STATE: the state never grows with the data inside a run.

use std::fmt::Write as _;

use runner::fib::rank::MoverRank;

use crate::harness::{Axis, BATCH, Class, Host, Row, measure, run};

/// Synthetic minutes cycled through by the fold, per pool.
const MINUTES: usize = 64;

/// A fold sized at its pool with a day open, and the minutes it cycles.
pub(crate) struct Day {
    /// The fold.
    rank: MoverRank,
    /// `MINUTES` minutes of closes, one per member each.
    closes: Vec<Vec<Option<i64>>>,
}

/// A deterministic value.
fn mix(n: u64) -> u64 {
    let mut z = n.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z ^ (z >> 31)
}

/// A day over `pool` members whose minutes move each member by a distinct
/// amount (no exact tie), with one member in ten missing at random.
fn day(pool: u64) -> Day {
    let size = usize::try_from(pool).unwrap();
    let mut rank = MoverRank::new(size).unwrap();
    let previous: Vec<Option<i64>> = (0..pool)
        .map(|s| Some(100_000 + i64::try_from(mix(s) % 900_000).unwrap()))
        .collect();
    rank.begin_day(&previous).unwrap();
    let closes = (0..MINUTES as u64)
        .map(|m| {
            previous
                .iter()
                .enumerate()
                .map(|(s, p)| {
                    let n = mix(m * 1_000_003 + s as u64);
                    (!n.is_multiple_of(10))
                        .then(|| p.unwrap() + i64::try_from(n % 2_001).unwrap() - 1_000)
                })
                .collect()
        })
        .collect();
    Day { rank, closes }
}

/// A day whose every member moves by the same basis points and distinct exact
/// ratios: one tie run of the whole pool, every minute.
fn tied(pool: u64) -> Day {
    let size = usize::try_from(pool).unwrap();
    let mut rank = MoverRank::new(size).unwrap();
    let previous: Vec<Option<i64>> = (0..pool)
        .map(|s| Some(10_000_000 + i64::try_from(s).unwrap() * 13))
        .collect();
    rank.begin_day(&previous).unwrap();
    let closes = (0..i64::try_from(MINUTES).unwrap())
        .map(|m| {
            previous
                .iter()
                .map(|p| Some(p.unwrap() + 10_000 + (m % 7)))
                .collect()
        })
        .collect();
    Day { rank, closes }
}

/// Folds call `i`'s minute.
fn fold(day: &mut Day, i: u64) -> u64 {
    let closes = &day.closes[usize::try_from(i).unwrap() % MINUTES];
    let minute = 1_000_000 + i64::try_from(i).unwrap() * 60;
    u64::from(day.rank.minute(minute, closes).unwrap().ranked)
}

/// The rows, run and reported. False when any row failed.
pub(crate) fn report(host: &Host, out: &mut String) -> bool {
    let fixed = Some(Axis::FixedState {
        why: "one day's rank state, sized once at the run's largest pool",
    });
    let minute = Row {
        id: "R-F-01",
        name: "MoverRank::minute (one Zerodha minute over the pool)",
        class: Some(Class::Bounded {
            bound: "O(pool + buckets) per minute, the same at every minute of a day: O(1) per member",
        }),
        axis: fixed,
        scales: [5, 50, 512],
        setup: day,
        op: fold,
        work: |day| day.rank.work(),
        bytes: |day| (day.rank.max_pool() * 64) as u64,
    };
    let ties = Row {
        id: "R-F-02",
        name: "MoverRank::minute with one exact tie run of the whole pool",
        class: Some(Class::Bounded {
            bound: "O(r log r) per tie run of r members, on top of the minute",
        }),
        axis: fixed,
        scales: [5, 50, 512],
        setup: tied,
        op: fold,
        work: |day| day.rank.work(),
        bytes: |day| (day.rank.max_pool() * 64) as u64,
    };
    let mut ok = true;
    for verdict in [run(&minute, host, out), run(&ties, host, out)] {
        ok &= !verdict.fails();
    }
    // FM-F010's claim, per member: each scale's per-minute percentiles over
    // its pool, in picoseconds.
    let _ = writeln!(
        out,
        "R-F-01m MoverRank::minute PER MEMBER — WORST-CASE per member (FM-F010)"
    );
    let at = minute.scales.map(|pool| (pool, measure(&minute, pool)));
    let per = |ps: u128, pool: u64| ps / u128::from(pool);
    for (pool, m) in &at {
        let _ = writeln!(
            out,
            "  pool {pool:>4}: p50 {:>7} ps  p99 {:>7} ps  max {:>9} ps per member \
             (batch means at {BATCH}); work p50 {} max {} per minute",
            per(m.time.0, *pool),
            per(m.time.1, *pool),
            per(m.time.2, *pool),
            m.work.quantile(1, 2),
            m.work.max()
        );
    }
    let [(small_pool, small), _, (large_pool, large)] = &at;
    let ratio = |a: u128, b: u128| a * 1_000 / b.max(1);
    let p99 = ratio(
        per(large.time.1, *large_pool),
        per(small.time.1, *small_pool),
    );
    let max = ratio(
        per(large.time.2, *large_pool),
        per(small.time.2, *small_pool),
    );
    let shown = |r: u128| format!("{}.{:03}x", r / 1_000, r % 1_000);
    let _ = writeln!(
        out,
        "  RATIO R-F-01m pool {large_pool} / {small_pool} per member: p99 {} max {}",
        shown(p99),
        shown(max)
    );
    if p99 <= 1_250 && max <= 1_250 {
        let _ = writeln!(out, "  PASS — per-member p99 and max ratios <= 1.250x");
    } else if p99 <= 3_000 && max <= 3_000 {
        let _ = writeln!(
            out,
            "  FAIL (time, within the shared ceiling) — per-member ratio above 1.250x on this host"
        );
    } else {
        let _ = writeln!(
            out,
            "  FAIL — per-member ratio above the shared ceiling 3.000x"
        );
        ok = false;
    }
    let _ = writeln!(out, "R-F-01s the specification's pools, per minute:");
    for pool in [16, 128, 256, 512] {
        let m = measure(&minute, pool);
        let _ = writeln!(
            out,
            "  pool {pool:>4}: p50 {:>9} ps  p99 {:>9} ps  max {:>11} ps per minute; \
             work p50 {} p99 {} max {}",
            m.time.0,
            m.time.1,
            m.time.2,
            m.work.quantile(1, 2),
            m.work.quantile(99, 100),
            m.work.max()
        );
    }
    let _ = writeln!(out);
    ok
}

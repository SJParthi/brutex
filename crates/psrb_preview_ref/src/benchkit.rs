//! The ratio bench's measuring functions (TEST-PLAN section 3), in the library so tests
//! cover them with injected samples: every per-call sample kept and sorted, p50, p99 and
//! max by nearest rank, and the p99 ratio between sizes judged against 3.000x.

use std::time::Instant;

use crate::cond::Side;
use crate::fills::{Print, StopScan};
use crate::market::Bar;
use crate::readings::Bank;
use crate::ruleset::RULE_SETS;
use crate::signal::{Context, evaluate};

/// The shared ceiling: a 10x or 100x p99 at most 3.000 times the 1x p99.
pub const CEILING_PERMILLE: u128 = 3_000;

/// One size's samples, sorted, with its nearest-rank summary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Dist {
    sorted: Vec<u64>,
}

impl Dist {
    /// The distribution of `samples` (nanoseconds per call).
    #[must_use]
    pub fn of(mut samples: Vec<u64>) -> Self {
        samples.sort_unstable();
        Self { sorted: samples }
    }

    /// The value at `permille` by nearest rank; 0 for an empty sample.
    #[must_use]
    pub fn at(&self, permille: usize) -> u64 {
        let rank = (permille * self.sorted.len()).div_ceil(1_000).max(1);
        self.sorted.get(rank - 1).copied().unwrap_or(0)
    }

    /// The median.
    #[must_use]
    pub fn p50(&self) -> u64 {
        self.at(500)
    }

    /// The 99th percentile.
    #[must_use]
    pub fn p99(&self) -> u64 {
        self.at(990)
    }

    /// The slowest call.
    #[must_use]
    pub fn max(&self) -> u64 {
        self.at(1_000)
    }

    /// Calls timed.
    #[must_use]
    pub fn n(&self) -> usize {
        self.sorted.len()
    }
}

/// A ratio's verdict.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// Within the ceiling: the ratio in permille.
    Flat(u128),
    /// Past the ceiling: the ratio in permille.
    Breach(u128),
    /// A zero 1x baseline: work the optimiser deleted has not been shown constant.
    Unmeasurable,
}

/// `p99_at / p99_base` in permille against [`CEILING_PERMILLE`].
#[must_use]
pub fn ratio(base: u64, at: u64) -> Verdict {
    if base == 0 {
        return Verdict::Unmeasurable;
    }
    let permille = u128::from(at) * 1_000 / u128::from(base);
    if permille <= CEILING_PERMILLE {
        Verdict::Flat(permille)
    } else {
        Verdict::Breach(permille)
    }
}

/// Times `reps` calls of `step`, one clock pair around each.
pub fn time_calls(reps: usize, mut step: impl FnMut(usize)) -> Dist {
    let mut out = Vec::with_capacity(reps);
    for i in 0..reps {
        let t0 = Instant::now();
        step(i);
        out.push(u64::try_from(t0.elapsed().as_nanos()).unwrap_or(u64::MAX));
    }
    Dist::of(out)
}

/// A deterministic candle stream: a triangle drift with jumps, so every reading and
/// both trends are exercised.
#[must_use]
pub fn candle(i: usize) -> Bar {
    let phase = i64::try_from(i % 400).unwrap_or(0);
    let drift = if phase < 200 { phase } else { 400 - phase };
    let jump = if i.is_multiple_of(97) { 300 } else { 0 };
    let c = 100_000 + 5 * drift + jump;
    Bar::new(c, c + 7, c - 7, c, 10)
}

/// B-per-candle: a bank already `depth` candles deep, then one candle into all seven
/// states plus the gate and the conjunction of every rule set, timed `reps` times.
#[must_use]
pub fn per_candle(depth: usize, reps: usize) -> Dist {
    let mut bank = Bank::new();
    for i in 0..depth {
        bank.step(Some(&candle(i)), false);
    }
    let prev = crate::dayref::AcceptedBar::from_hlc(100_100, 99_900, 100_000);
    time_calls(reps, |i| {
        let bar = candle(depth + i);
        bank.step(Some(&bar), false);
        for set in &RULE_SETS {
            let ctx = Context {
                bank: &bank,
                close: bar.close,
                prev,
                in_span: true,
                hole: false,
            };
            std::hint::black_box(evaluate(set, &ctx));
        }
    })
}

/// B-per-second: a stop scan already `scanned` prints into a trade, then one print per
/// call, timed `reps` times (the stop never reached, so every call is a full step).
#[must_use]
pub fn per_second(scanned: usize, reps: usize) -> Dist {
    let prints: Vec<Print> = (0..scanned + reps)
        .map(|i| {
            let at = u32::try_from(i).unwrap_or(u32::MAX);
            Print {
                start: at,
                slot: at,
                high: 100_010,
                low: 99_990,
            }
        })
        .collect();
    let mut scan = StopScan {
        stop: 90_000,
        side: Side::Long,
        cursor: 0,
    };
    for p in prints.iter().take(scanned) {
        scan.step(p);
    }
    time_calls(reps, |i| {
        let p = prints.get(scanned + i).copied().unwrap_or(Print {
            start: 0,
            slot: 0,
            high: 0,
            low: 0,
        });
        std::hint::black_box(scan.step(&p));
    })
}

/// One printed row of the bench: the three sizes and the two ratios' verdicts.
#[must_use]
pub fn row(label: &str, dists: &[Dist; 3]) -> (String, bool) {
    let [a, b, c] = dists;
    let v10 = ratio(a.p99(), b.p99());
    let v100 = ratio(a.p99(), c.p99());
    let ok = matches!(v10, Verdict::Flat(_)) && matches!(v100, Verdict::Flat(_));
    let line = format!(
        "{label}: 1x p50 {} p99 {} max {} n {} | 10x p50 {} p99 {} max {} n {} | 100x p50 {} p99 {} max {} n {} | ratio 10x/1x {v10:?} 100x/1x {v100:?} ceiling {CEILING_PERMILLE}",
        a.p50(),
        a.p99(),
        a.max(),
        a.n(),
        b.p50(),
        b.p99(),
        b.max(),
        b.n(),
        c.p50(),
        c.p99(),
        c.max(),
        c.n(),
    );
    (line, ok)
}

#[cfg(test)]
#[path = "benchkit_tests.rs"]
mod tests;

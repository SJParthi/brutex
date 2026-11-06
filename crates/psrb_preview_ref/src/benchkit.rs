//! The O(1) ratio measurement: per-step latency at 1x, 10x and 100x, p99 flat.

use std::time::Instant;

/// One size's per-step latency, in nanoseconds, by nearest rank.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StepCost {
    /// Steps timed.
    pub n: usize,
    /// Median.
    pub p50: u64,
    /// 99th percentile.
    pub p99: u64,
    /// Slowest step.
    pub max: u64,
}

/// The value at `permille` of a sorted sample by nearest rank (`ceil(p * n)`).
fn at_rank(sorted: &[u64], permille: usize) -> u64 {
    let n = sorted.len();
    let rank = (permille * n).div_ceil(1000).max(1);
    sorted.get(rank - 1).copied().unwrap_or(0)
}

/// Sorts `samples` and reads p50, p99 and max; `None` for an empty sample.
pub fn summarize(samples: &mut [u64]) -> Option<StepCost> {
    if samples.is_empty() {
        return None;
    }
    samples.sort_unstable();
    Some(StepCost {
        n: samples.len(),
        p50: at_rank(samples, 500),
        p99: at_rank(samples, 990),
        max: at_rank(samples, 1000),
    })
}

/// `big.p99 <= base.p99 * ceiling / 1000`, with the base floored at `floor_ns`
/// so a timer-resolution base cannot make noise look like growth.
#[must_use]
pub fn flat_within(base: StepCost, big: StepCost, ceiling_permille: u64, floor_ns: u64) -> bool {
    let b = u128::from(base.p99.max(floor_ns));
    u128::from(big.p99) * 1000 <= b * u128::from(ceiling_permille)
}

/// Times `steps` calls of `step`, one clock read pair per call.
pub fn time_steps(steps: usize, mut step: impl FnMut(usize)) -> Vec<u64> {
    let mut out = Vec::with_capacity(steps);
    for i in 0..steps {
        let t0 = Instant::now();
        step(i);
        let ns = t0.elapsed().as_nanos();
        out.push(u64::try_from(ns).unwrap_or(u64::MAX));
    }
    out
}

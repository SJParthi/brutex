//! The p50 / p99 harness shared by every `o1p99` attack bench (cloud lane,
//! 10 Nov 2026). Copied into each crate's `benches/o1p99/`, never shared by
//! path: a bench target links only its own crate.
//!
//! # What one sample is
//!
//! `batch` calls between one pair of `Instant` reads, divided by `batch`,
//! after subtracting the clock's own cost (calibrated once by [`clock_ps`]:
//! 23 to 35 ns a pair on the attack host, depending on load). `batch` is 1
//! wherever a call costs well over the clock, so a single slow call is a
//! single slow sample; a call near or under the clock is batched, per size,
//! so the clock's jitter is divided by the batch rather than read as cost.
//!
//! # How p99 is read
//!
//! Every size is measured in the same round, one after another, and the round
//! is repeated (`rounds`), so drift and a noisy neighbour land on every size
//! alike. Printed per size: p50 and p99 pooled over every round, the max, and
//! the QUIETEST ROUND's p99, the smallest of the per-round p99s. A structural
//! tail (a probe chain, a rehash, a scan, a cold read) recurs in every round
//! and survives that minimum; a scheduler stall lands in one round and does
//! not. The p99 of n samples is the sample at rank `ceil(0.99 n)`.
//!
//! # The verdict
//!
//! FLAT when the quietest-round p99 at the largest size is at most the stated
//! factor times the quietest-round p99 at the smallest size. The default
//! factor is 3.0x, the shared-CI ceiling in `docs/04-invariants.md`. A base
//! under one nanosecond is lifted to one nanosecond before dividing, so a
//! call the clock cannot resolve cannot manufacture a ratio; a row whose
//! sample is under 50 ns at any size says UNDER-RESOLVED beside its verdict.
//!
//! # Physics is printed, not hidden
//!
//! A random access into 10^6 entries misses the cache and a scan of 48 MB
//! streams from memory; neither is the code growing. [`memory_floor`] and
//! [`against`] put a row beside the cost of touching the same bytes, so a
//! growth the floor shares is read as the machine and a growth it does not
//! share is read as the code.
//!
//! Integer arithmetic only: `clippy::float_arithmetic` is a workspace lint.

use std::hint::black_box;
use std::time::Instant;

/// The default ceiling on the p99 ratio, largest size over smallest, in
/// thousandths: 3.0x.
pub(crate) const FACTOR_PERMILLE: u128 = 3_000;

/// The smallest base a ratio divides by, picoseconds.
const BASE_FLOOR_PS: u128 = 1_000;

/// A sample shorter than this (picoseconds, the whole batch) is under the
/// clock's resolution and the row says so.
const RESOLVED_PS: u128 = 50_000;

/// A deterministic `splitmix64` stream: every run attacks the same keys.
pub(crate) struct Rng(u64);

impl Rng {
    /// A stream from `seed`.
    pub(crate) const fn new(seed: u64) -> Self {
        Self(seed)
    }

    /// The next 64 bits.
    pub(crate) fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    /// Uniform-ish in `0..bound`; `0` when `bound` is `0`.
    pub(crate) fn below(&mut self, bound: u64) -> u64 {
        if bound == 0 {
            0
        } else {
            self.next_u64() % bound
        }
    }
}

/// One size's latencies, picoseconds per call (or thousandths of a floor,
/// after [`against`]).
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Tail {
    /// Median, every round pooled.
    pub(crate) p50_ps: u128,
    /// p99, every round pooled.
    pub(crate) p99_ps: u128,
    /// The smallest per-round p99.
    pub(crate) quiet_p99_ps: u128,
    /// The slowest sample.
    pub(crate) max_ps: u128,
    /// Samples taken.
    pub(crate) n: usize,
    /// The shortest whole-batch sample's p50, picoseconds, for the
    /// resolution check.
    pub(crate) sample_p50_ps: u128,
}

/// The sample at rank `ceil(permille / 1000 * n)` of a sorted slice.
fn rank(sorted: &[u128], permille: usize) -> u128 {
    let at = sorted.len().saturating_mul(permille).div_ceil(1_000).max(1);
    sorted.get(at - 1).copied().unwrap_or(0)
}

/// The clock's own cost, picoseconds per `Instant` pair: the median of
/// 20,000 empty timings.
pub(crate) fn clock_ps() -> u128 {
    let mut d: Vec<u128> = (0..20_000)
        .map(|_| {
            let start = Instant::now();
            black_box(());
            start.elapsed().as_nanos().saturating_mul(1_000)
        })
        .collect();
    d.sort_unstable();
    rank(&d, 500)
}

/// [`measure_each`] with one batch for every state.
pub(crate) fn measure<S>(
    states: &mut [S],
    rounds: usize,
    per_round: usize,
    batch: u32,
    clock: u128,
    prepare: impl FnMut(&mut S),
    op: impl FnMut(&mut S, u64) -> u64,
) -> Vec<Tail> {
    let batches = vec![batch; states.len()];
    measure_each(states, rounds, per_round, &batches, clock, prepare, op)
}

/// Measures `op` on every state, interleaved, and returns one [`Tail`] per
/// state in order. `batches[i]` calls make one sample of state `i`.
/// `op(state, i)` is one call; `i` counts that state's calls from zero so the
/// caller can walk its keys. `prepare(state)` runs before each sample OUTSIDE
/// the timer (an append that must start from the same size resets here).
pub(crate) fn measure_each<S>(
    states: &mut [S],
    rounds: usize,
    per_round: usize,
    batches: &[u32],
    clock: u128,
    mut prepare: impl FnMut(&mut S),
    mut op: impl FnMut(&mut S, u64) -> u64,
) -> Vec<Tail> {
    let count = states.len();
    let mut pooled: Vec<Vec<u128>> = (0..count)
        .map(|_| Vec::with_capacity(rounds.saturating_mul(per_round)))
        .collect();
    let mut whole: Vec<Vec<u128>> = (0..count)
        .map(|_| Vec::with_capacity(rounds.saturating_mul(per_round)))
        .collect();
    let mut quiet: Vec<u128> = vec![u128::MAX; count];
    let mut calls: Vec<u64> = vec![0; count];
    let mut round: Vec<u128> = Vec::with_capacity(per_round);
    for _ in 0..rounds {
        for ((((state, all), raw), q), (call, batch)) in states
            .iter_mut()
            .zip(pooled.iter_mut())
            .zip(whole.iter_mut())
            .zip(quiet.iter_mut())
            .zip(
                calls
                    .iter_mut()
                    .zip(batches.iter().copied().chain(std::iter::repeat(1))),
            )
        {
            let batch = batch.max(1);
            round.clear();
            for _ in 0..per_round {
                prepare(state);
                let start = Instant::now();
                for _ in 0..batch {
                    black_box(op(state, *call));
                    *call = call.wrapping_add(1);
                }
                let sample = start.elapsed().as_nanos().saturating_mul(1_000);
                raw.push(sample);
                round.push(sample.saturating_sub(clock) / u128::from(batch));
            }
            round.sort_unstable();
            *q = (*q).min(rank(&round, 990));
            all.extend_from_slice(&round);
        }
    }
    pooled
        .into_iter()
        .zip(whole)
        .zip(quiet)
        .map(|((mut all, mut raw), quiet_p99_ps)| {
            all.sort_unstable();
            raw.sort_unstable();
            Tail {
                p50_ps: rank(&all, 500),
                p99_ps: rank(&all, 990),
                quiet_p99_ps,
                max_ps: all.last().copied().unwrap_or(0),
                n: all.len(),
                sample_p50_ps: rank(&raw, 500),
            }
        })
        .collect()
}

/// Every figure of each tail divided by its own divisor (a per-bar or
/// per-row cost from a per-call one).
pub(crate) fn per_unit(tails: &[Tail], divisors: &[u64]) -> Vec<Tail> {
    tails
        .iter()
        .zip(divisors)
        .map(|(t, &d)| {
            let d = u128::from(d.max(1));
            Tail {
                p50_ps: t.p50_ps / d,
                p99_ps: t.p99_ps / d,
                quiet_p99_ps: t.quiet_p99_ps / d,
                max_ps: t.max_ps / d,
                n: t.n,
                sample_p50_ps: t.sample_p50_ps,
            }
        })
        .collect()
}

/// Each tail as thousandths of the matching floor's same figure: the cost in
/// FLOORS, so a growth both share cancels and a growth only the code has
/// remains.
pub(crate) fn against(op: &[Tail], floor: &[Tail]) -> Vec<Tail> {
    let of = |a: u128, b: u128| a.saturating_mul(1_000) / b.max(1);
    op.iter()
        .zip(floor)
        .map(|(o, f)| Tail {
            p50_ps: of(o.p50_ps, f.p50_ps),
            p99_ps: of(o.p99_ps, f.p99_ps),
            quiet_p99_ps: of(o.quiet_p99_ps, f.quiet_p99_ps),
            max_ps: of(o.max_ps, f.max_ps),
            n: o.n,
            sample_p50_ps: o.sample_p50_ps.min(f.sample_p50_ps),
        })
        .collect()
}

/// The memory floor at each working-set size: one random 8-byte load from a
/// buffer of that many bytes. A random-key lookup into a structure of that
/// size cannot beat it, so a random-key row is read against this one.
pub(crate) fn memory_floor(
    bytes: &[u64],
    rounds: usize,
    per_round: usize,
    clock: u128,
) -> Vec<Tail> {
    let mut buffers: Vec<(Vec<u64>, Rng)> = bytes
        .iter()
        .map(|&b| {
            let words = usize::try_from((b / 8).max(1)).unwrap_or(usize::MAX);
            let mut r = Rng::new(0x5eed);
            ((0..words).map(|_| r.next_u64()).collect(), Rng::new(b))
        })
        .collect();
    measure(
        &mut buffers,
        rounds,
        per_round,
        1,
        clock,
        |_| {},
        |(buf, r), _| {
            let at =
                usize::try_from(r.below(u64::try_from(buf.len()).unwrap_or(u64::MAX))).unwrap_or(0);
            buf.get(at).copied().unwrap_or(0)
        },
    )
}

/// Picoseconds as `n.nnn` nanoseconds.
pub(crate) fn ns(ps: u128) -> String {
    format!("{}.{:03}", ps / 1_000, ps % 1_000)
}

/// A ratio in thousandths as `n.nnnx`.
pub(crate) fn times(permille: u128) -> String {
    format!("{}.{:03}x", permille / 1_000, permille % 1_000)
}

/// `at / base` in thousandths, with the base lifted to [`BASE_FLOOR_PS`].
pub(crate) fn permille(base: u128, at: u128) -> u128 {
    at.saturating_mul(1_000) / base.max(BASE_FLOOR_PS)
}

/// Prints one claim's per-size table (nanoseconds) and returns whether the
/// quietest-round p99 at the largest size stayed within `factor`
/// (thousandths) of the smallest's. Every line starts `O1P99|` so a run greps
/// into the report.
pub(crate) fn verdict(
    id: &str,
    scenario: &str,
    sizes: &[u64],
    tails: &[Tail],
    factor: u128,
) -> bool {
    judge(id, scenario, sizes, tails, factor, "ns")
}

/// [`verdict`] for tails made by [`against`]: the figures are floors.
pub(crate) fn verdict_floors(
    id: &str,
    scenario: &str,
    sizes: &[u64],
    tails: &[Tail],
    factor: u128,
) -> bool {
    judge(id, scenario, sizes, tails, factor, "floors")
}

/// The shared body of [`verdict`] and [`verdict_floors`].
///
/// The gated ratio is the PEAK: the largest quietest-round p99 at any size
/// over the smallest size's, so a cost that rises and falls back (a
/// bisection below a threshold, an index above it) is read at its top and
/// not only at the ends. The largest-over-smallest ratio is printed beside it.
fn judge(
    id: &str,
    scenario: &str,
    sizes: &[u64],
    tails: &[Tail],
    factor: u128,
    unit: &str,
) -> bool {
    let mut resolved = true;
    for (size, t) in sizes.iter().zip(tails) {
        resolved &= t.sample_p50_ps >= RESOLVED_PS;
        println!(
            "O1P99|{id}|{scenario}|n={size}|p50={} {unit}|p99={} {unit}|quiet-p99={} {unit}|max={} {unit}|samples={}",
            ns(t.p50_ps),
            ns(t.p99_ps),
            ns(t.quiet_p99_ps),
            ns(t.max_ps),
            t.n
        );
    }
    let (Some(first), Some(last)) = (tails.first(), tails.last()) else {
        println!("O1P99|{id}|{scenario}|VERDICT|UNMEASURED|no sizes");
        return false;
    };
    let (peak_size, peak) = sizes
        .iter()
        .zip(tails)
        .map(|(s, t)| (*s, t.quiet_p99_ps))
        .max_by_key(|&(_, p)| p)
        .unwrap_or((0, 0));
    let p99_peak = permille(first.quiet_p99_ps, peak);
    let p99_end = permille(first.quiet_p99_ps, last.quiet_p99_ps);
    let p50 = permille(first.p50_ps, last.p50_ps);
    let ok = p99_peak <= factor;
    // A printed-only row says what the default ceiling would have said.
    let label = match (factor == u128::MAX, p99_peak <= FACTOR_PERMILLE, ok) {
        (true, true, _) => "PRINTED-FLAT",
        (true, false, _) => "PRINTED-GROWS",
        (false, _, true) => "FLAT",
        (false, _, false) => "GROWS",
    };
    println!(
        "O1P99|{id}|{scenario}|VERDICT|{label}|p99 ratio {} at n={peak_size} (quiet peak over smallest); largest over smallest p99 {} p50 {}; ceiling {}{}",
        times(p99_peak),
        times(p99_end),
        times(p50),
        if factor == u128::MAX {
            "none (printed only)".to_owned()
        } else {
            times(factor)
        },
        if resolved {
            ""
        } else {
            " UNDER-RESOLVED (a sample under 50 ns)"
        }
    );
    ok
}

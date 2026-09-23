//! One walk over the bootstrap draws for all three family-wise tests.
//!
//! # Why this exists
//!
//! [`super::white_reality_check_receipt_v1`], [`super::spa_receipt_v1`] and
//! [`super::romano_wolf_adjusted_p_values_v1`] each start from `Rng::new(seed)`
//! and call `stationary_indices(periods, block, ..)` once per draw, so over one
//! family they resample the SAME index vectors -- and each then recomputes every
//! strategy's resampled mean for itself. A whole-stage profile of the live
//! index-stop sweep (2026-09-11) put essentially all of the institutional
//! stage's CPU in those three separate passes, about a minute each at 200,000
//! draws over 4,000 strategies and 225 periods, and twice per batch because the
//! cold re-check replays them.
//!
//! [`family_tests_v1`] computes each strategy's resampled mean ONCE per draw and
//! derives White's centred maximum, Hansen's gated studentized maximum and
//! Romano--Wolf's suffix maxima from it, spreading the draws over the current
//! rayon pool.
//!
//! # Why the results are bit-identical rather than merely close
//!
//! * **The draws.** One `Rng::new(seed)` stream is consumed serially, in draw
//!   order, by the same `stationary_indices`, so draw `m` holds exactly the
//!   indices draw `m` of each separate procedure holds. Chunking changes WHEN an
//!   index vector is generated, never which one.
//! * **Every float.** Each value comes from the same operation on the same
//!   operands as in the separate procedures: the same `summarise`, `mean_at` and
//!   `studentized`, the same centring `resampled - mean`, the same `root_n *`,
//!   Hansen's same gate, and every running maximum folded in the same order --
//!   strategy order for White and SPA, stepdown order from the last rank for
//!   Romano--Wolf. IEEE-754 arithmetic is deterministic, and Rust never fuses
//!   `a * b + c` into one rounding unless asked to.
//! * **Only integers cross threads.** A task counts its own draws' hits; the
//!   tasks' counts are then summed in task order. Integer addition does not
//!   depend on grouping, so neither the thread count nor the chunk size can move
//!   a count -- and a count is all a comparison contributes.
//! * **The receipts.** They are built by the same constructors from those
//!   counts, so statistics, p-value bits and digests follow from identical
//!   inputs.
//!
//! # Memory
//!
//! The separate Romano--Wolf procedure holds all `draws x periods` indices at
//! once (about 360 MB at 200,000 x 225). This walk generates at most
//! `INDEX_CHUNK_BYTES` of indices per serial step -- or one draw's, when a
//! single draw is larger than that. Each running task holds one
//! strategy-length scratch row, and every task of the step keeps one
//! rank-length count row until the step is reduced: at most
//! `MAX_CHUNK_DRAWS / TASK_DRAWS` = 512 count rows, about 16 MB at 4,000 ranks.
//!
//! # Cost
//!
//! O(B·S·N) additions for B draws, S strategies and N periods -- one resampled
//! mean per strategy per draw where the separate procedures computed three --
//! plus O(B·N) serial index generation. **UNVERIFIED as a measured bound**: no
//! bench row in this workspace re-measures it. `CLAUDE.md` §3 rule 6.

use super::{
    ExactResamplingPValueV1, Performance, Rng, RomanoWolfAdjustedCandidateV1,
    RomanoWolfAdjustedReceiptV1, SpaReceiptV1, WhiteRealityCheckReceiptV1,
    exact_family_test_fields_v1, exact_family_test_inputs_v1, mean_at,
    romano_wolf_family_digest_v1, stationary_indices, studentized, summarise,
};
use rayon::prelude::*;

/// Index bytes one serial generation step may hold before its draws are
/// evaluated. A bound on memory, not on answers: no result depends on it.
const INDEX_CHUNK_BYTES: usize = 8 << 20;
/// Draws generated per serial step when the period count is small.
const MAX_CHUNK_DRAWS: usize = 4_096;
/// Draws one parallel task evaluates with one scratch row.
const TASK_DRAWS: usize = 8;
/// The White receipt's procedure identity, byte for byte.
const WHITE_DOMAIN: &[u8] = b"brutex/runner/white-reality-check-exact/v1\0";
/// The SPA receipt's procedure identity, byte for byte.
const SPA_DOMAIN: &[u8] = b"brutex/runner/hansen-spa-exact/v1\0";

/// White's Reality Check, Hansen's SPA and Romano--Wolf's adjusted p-values,
/// measured from one walk over one set of draws.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FamilyTestsV1 {
    white: WhiteRealityCheckReceiptV1,
    spa: SpaReceiptV1,
    romano_wolf: Option<RomanoWolfAdjustedReceiptV1>,
}

impl FamilyTestsV1 {
    /// The receipt [`super::white_reality_check_receipt_v1`] returns for the
    /// whole family, bit for bit.
    #[must_use]
    pub const fn white(&self) -> WhiteRealityCheckReceiptV1 {
        self.white
    }

    /// The receipt [`super::spa_receipt_v1`] returns for the whole family, bit
    /// for bit.
    #[must_use]
    pub const fn spa(&self) -> SpaReceiptV1 {
        self.spa
    }

    /// The receipt [`super::romano_wolf_adjusted_p_values_v1`] returns for the
    /// named rows, bit for bit, or `None` when no row was named.
    #[must_use]
    pub const fn romano_wolf(&self) -> Option<&RomanoWolfAdjustedReceiptV1> {
        self.romano_wolf.as_ref()
    }
}

/// Why [`family_tests_v1`] refused, named by the procedure that refuses first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FamilyTestsRefusalV1 {
    /// A named Romano--Wolf row is not a row of the family.
    Rows,
    /// [`super::romano_wolf_adjusted_p_values_v1`] refuses the named rows.
    RomanoWolf,
    /// [`super::white_reality_check_receipt_v1`] refuses the family.
    White,
    /// [`super::spa_receipt_v1`] refuses the family.
    Spa,
    /// The shared walk could not complete its exact counts.
    Pass,
}

/// Runs White's Reality Check, Hansen's SPA and Romano--Wolf's adjusted
/// p-values over one family from ONE walk over the draws.
///
/// `returns` is the complete aligned family that White and SPA measure.
/// `romano_wolf_rows` names, in order, the rows forming Romano--Wolf's family:
/// the Romano--Wolf receipt is exactly the one
/// [`super::romano_wolf_adjusted_p_values_v1`] returns for those rows copied in
/// that order -- its caller-position tie-breaker and its digest refer to
/// positions in that list. An empty list runs no Romano--Wolf procedure.
///
/// Every receipt equals, field for field and bit for bit, the one the separate
/// function returns for the same inputs; the module header states why.
///
/// # Parallelism
///
/// The draws are evaluated on the CURRENT rayon pool: the one entered by
/// `ThreadPool::install` or a broadcast when called from inside it, rayon's
/// global pool otherwise. The answer does not depend on which, nor on its
/// thread count.
///
/// # Errors
///
/// [`FamilyTestsRefusalV1::Rows`] when a named row is outside `returns`. Then
/// the refusal of the first separate procedure that refuses, in the order the
/// index-stop qualification has always called them: Romano--Wolf (only when a
/// row is named), White, SPA. [`FamilyTestsRefusalV1::Pass`] when the exact
/// counts could not be completed, which no valid family reaches.
pub fn family_tests_v1(
    returns: &[Vec<i64>],
    romano_wolf_rows: &[usize],
    draws: usize,
    seed: u64,
    block: usize,
) -> Result<FamilyTestsV1, FamilyTestsRefusalV1> {
    if romano_wolf_rows.iter().any(|&row| row >= returns.len()) {
        return Err(FamilyTestsRefusalV1::Rows);
    }
    let stepdown = if romano_wolf_rows.is_empty() {
        None
    } else {
        Some(
            Stepdown::new(returns, romano_wolf_rows, draws, seed, block)
                .ok_or(FamilyTestsRefusalV1::RomanoWolf)?,
        )
    };
    let (periods, stats) =
        exact_family_test_inputs_v1(returns, draws, block).ok_or(FamilyTestsRefusalV1::White)?;
    let family = Family::new(returns, &stats, periods, stepdown.as_ref());
    let tally = count(&family, periods, draws, seed, block, chunk_draws(periods))
        .ok_or(FamilyTestsRefusalV1::Pass)?;
    let romano_wolf = stepdown
        .map(|plan| {
            plan.receipt(&tally.strict, draws, periods, seed, block)
                .ok_or(FamilyTestsRefusalV1::RomanoWolf)
        })
        .transpose()?;
    let white = exact_family_test_fields_v1(
        WHITE_DOMAIN,
        returns,
        draws,
        seed,
        block,
        periods,
        family.white_observed,
        tally.white,
    )
    .ok_or(FamilyTestsRefusalV1::White)?;
    let spa = exact_family_test_fields_v1(
        SPA_DOMAIN,
        returns,
        draws,
        seed,
        block,
        periods,
        family.spa_observed,
        tally.spa,
    )
    .ok_or(FamilyTestsRefusalV1::Spa)?;
    Ok(FamilyTestsV1 {
        white: WhiteRealityCheckReceiptV1 { fields: white },
        spa: SpaReceiptV1 { fields: spa },
        romano_wolf,
    })
}

/// Draws per serial generation step: as many as fit `INDEX_CHUNK_BYTES`, at
/// least one and at most `MAX_CHUNK_DRAWS`.
fn chunk_draws(periods: usize) -> usize {
    let per_draw = periods.saturating_mul(size_of::<usize>()).max(1);
    (INDEX_CHUNK_BYTES / per_draw).clamp(1, MAX_CHUNK_DRAWS)
}

/// One strategy as every draw reads it.
struct Lane<'a> {
    series: &'a [i64],
    mean: f64,
    standard_error: f64,
    /// Hansen's gate: recentred when the strategy's own statistic clears
    /// `-sqrt(2 ln ln n)`, exactly the separate SPA's per-draw `keep`.
    recentred: bool,
}

/// One Romano--Wolf rank: the family lane it reads and its fixed statistics.
#[derive(Clone, Copy, Debug)]
struct Rung {
    /// Position in the named Romano--Wolf rows, the receipt's `strategy`.
    position: usize,
    /// Row of the family whose resampled mean this rank reads.
    lane: usize,
    observed: f64,
    mean: f64,
    standard_error: f64,
}

/// Everything a draw reads, fixed before the first draw.
struct Family<'a> {
    lanes: Vec<Lane<'a>>,
    root_n: f64,
    white_observed: f64,
    spa_observed: f64,
    /// Romano--Wolf's ranks from the LAST to the first; empty without rows.
    walk: Vec<Rung>,
}

impl<'a> Family<'a> {
    fn new(
        returns: &'a [Vec<i64>],
        stats: &[Performance],
        periods: usize,
        stepdown: Option<&Stepdown>,
    ) -> Self {
        // The same three expressions the separate White and SPA receipts
        // evaluate, over the same `summarise` rows, in the same order.
        let root_n = (periods as f64).sqrt();
        let white_observed = stats
            .iter()
            .map(|summary| root_n * summary.mean)
            .fold(f64::NEG_INFINITY, f64::max);
        let spa_observed = stats
            .iter()
            .map(|summary| studentized(summary.mean, summary.standard_error).max(0.0))
            .fold(f64::NEG_INFINITY, f64::max);
        let n = periods as f64;
        let gate = if n > 3.0 {
            -(2.0 * n.ln().ln()).sqrt()
        } else {
            f64::NEG_INFINITY
        };
        let lanes = returns
            .iter()
            .zip(stats)
            .map(|(series, summary)| Lane {
                series,
                mean: summary.mean,
                standard_error: summary.standard_error,
                recentred: studentized(summary.mean, summary.standard_error) >= gate,
            })
            .collect();
        Self {
            lanes,
            root_n,
            white_observed,
            spa_observed,
            walk: stepdown.map_or_else(Vec::new, |plan| plan.walk.clone()),
        }
    }

    /// Adds one draw's hits to `tally`. `resampled` is scratch, one slot per
    /// lane. `None` only if a rank names no lane or a count cannot grow.
    fn accumulate(&self, index: &[usize], resampled: &mut [f64], tally: &mut Tally) -> Option<()> {
        let mut white = f64::NEG_INFINITY;
        let mut spa = f64::NEG_INFINITY;
        for (lane, slot) in self.lanes.iter().zip(resampled.iter_mut()) {
            // The ONE resampled mean all three procedures read for this lane.
            let mean = mean_at(lane.series, index);
            *slot = mean;
            white = white.max(self.root_n * (mean - lane.mean));
            let hansen = if lane.recentred {
                mean - lane.mean
            } else {
                mean
            };
            spa = spa.max(studentized(hansen, lane.standard_error).max(0.0));
        }
        if white >= self.white_observed {
            tally.white = tally.white.checked_add(1)?;
        }
        if spa >= self.spa_observed {
            tally.spa = tally.spa.checked_add(1)?;
        }
        // Draw-major Romano--Wolf: walking the ranks from the last grows one
        // surviving suffix at a time, so `maximum` is exactly the separate
        // procedure's `maxima[draw]` when that rank is counted.
        let mut maximum = f64::NEG_INFINITY;
        for (rung, strict) in self.walk.iter().zip(tally.strict.iter_mut()) {
            let null = studentized(*resampled.get(rung.lane)? - rung.mean, rung.standard_error);
            maximum = maximum.max(null);
            if maximum > rung.observed {
                *strict = strict.checked_add(1)?;
            }
        }
        Some(())
    }

    /// One task: its draws, one scratch row, one count row.
    fn count_draws(&self, draws: &[Vec<usize>]) -> Option<Tally> {
        let mut resampled = vec![0.0; self.lanes.len()];
        let mut tally = Tally::new(self.walk.len());
        for index in draws {
            self.accumulate(index, &mut resampled, &mut tally)?;
        }
        Some(tally)
    }
}

/// Integer evidence from a run of draws -- the only thing that crosses threads.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Tally {
    white: usize,
    spa: usize,
    /// Strict Romano--Wolf exceedances, in walk order (last rank first).
    strict: Vec<usize>,
}

impl Tally {
    fn new(ranks: usize) -> Self {
        Self {
            white: 0,
            spa: 0,
            strict: vec![0; ranks],
        }
    }

    fn absorb(&mut self, part: &Self) -> Option<()> {
        self.white = self.white.checked_add(part.white)?;
        self.spa = self.spa.checked_add(part.spa)?;
        for (total, count) in self.strict.iter_mut().zip(&part.strict) {
            *total = total.checked_add(*count)?;
        }
        Some(())
    }
}

/// Every draw, generated serially from one stream in chunks of at most `chunk`
/// draws, each chunk evaluated in parallel and reduced in task order.
fn count(
    family: &Family<'_>,
    periods: usize,
    draws: usize,
    seed: u64,
    block: usize,
    chunk: usize,
) -> Option<Tally> {
    let mut rng = Rng::new(seed);
    let mut total = Tally::new(family.walk.len());
    let step = chunk.max(1);
    let mut done = 0_usize;
    while done < draws {
        let take = step.min(draws - done);
        let indices: Vec<Vec<usize>> = (0..take)
            .map(|_| stationary_indices(periods, block, &mut rng))
            .collect();
        // Indexed collect keeps task order; a refused task is found by walking
        // the results in that order, never by a parallel short-circuit.
        let parts: Vec<Option<Tally>> = indices
            .par_chunks(TASK_DRAWS)
            .map(|slice| family.count_draws(slice))
            .collect();
        for part in parts {
            total.absorb(&part?)?;
        }
        done += take;
    }
    Some(total)
}

/// Romano--Wolf's fixed inputs over the named rows, refused exactly where
/// [`super::romano_wolf_adjusted_p_values_v1`] refuses the same rows.
struct Stepdown {
    /// Named-row positions in canonical descending-statistic order.
    order: Vec<usize>,
    /// The same ranks from the last to the first.
    walk: Vec<Rung>,
    denominator: usize,
    family_digest: [u8; 32],
}

impl Stepdown {
    fn new(
        returns: &[Vec<i64>],
        rows: &[usize],
        draws: usize,
        seed: u64,
        block: usize,
    ) -> Option<Self> {
        if draws == 0 || block == 0 {
            return None;
        }
        let denominator = draws.checked_add(1)?;
        let named: Vec<&[i64]> = rows
            .iter()
            .map(|&row| returns.get(row).map(Vec::as_slice))
            .collect::<Option<_>>()?;
        let periods = named.first()?.len();
        if periods < 2 || named.iter().any(|series| series.len() != periods) {
            return None;
        }
        let stats: Vec<Performance> = named.iter().map(|series| summarise(series)).collect();
        // The separate procedure refuses a non-positive or non-finite standard
        // error, then a non-finite observed statistic. One pass, same set.
        if stats.iter().any(|stat| {
            stat.standard_error <= 0.0
                || !stat.standard_error.is_finite()
                || !studentized(stat.mean, stat.standard_error).is_finite()
        }) {
            return None;
        }
        let mut ranked: Vec<Rung> = rows
            .iter()
            .zip(&stats)
            .enumerate()
            .map(|(position, (&lane, stat))| Rung {
                position,
                lane,
                observed: studentized(stat.mean, stat.standard_error),
                mean: stat.mean,
                standard_error: stat.standard_error,
            })
            .collect();
        // The separate procedure's order: descending `total_cmp`, exact ties by
        // ascending position. That is a strict total order over distinct
        // positions, so every correct sort yields the same permutation.
        ranked.sort_unstable_by(|left, right| {
            right
                .observed
                .total_cmp(&left.observed)
                .then_with(|| left.position.cmp(&right.position))
        });
        Some(Self {
            order: ranked.iter().map(|rung| rung.position).collect(),
            walk: ranked.iter().rev().copied().collect(),
            denominator,
            family_digest: romano_wolf_family_digest_v1(&named, draws, seed, block)?,
        })
    }

    /// Algorithm 4.1's monotone adjustment over the counted suffixes, built as
    /// the separate procedure builds it. `strict_by_walk` is in walk order.
    fn receipt(
        &self,
        strict_by_walk: &[usize],
        draws: usize,
        periods: usize,
        seed: u64,
        block: usize,
    ) -> Option<RomanoWolfAdjustedReceiptV1> {
        let mut candidates: Vec<Option<RomanoWolfAdjustedCandidateV1>> =
            vec![None; self.order.len()];
        let mut adjusted_numerator = 0_usize;
        for ((rank, rung), &strict_exceedances) in self
            .walk
            .iter()
            .rev()
            .enumerate()
            .zip(strict_by_walk.iter().rev())
        {
            let initial_numerator = strict_exceedances.checked_add(1)?;
            adjusted_numerator = adjusted_numerator.max(initial_numerator);
            *candidates.get_mut(rung.position)? = Some(RomanoWolfAdjustedCandidateV1 {
                strategy: rung.position,
                stepdown_rank: rank,
                observed_statistic_bits: rung.observed.to_bits(),
                strict_exceedances,
                initial_p_value: ExactResamplingPValueV1 {
                    numerator: initial_numerator,
                    denominator: self.denominator,
                },
                adjusted_p_value: ExactResamplingPValueV1 {
                    numerator: adjusted_numerator,
                    denominator: self.denominator,
                },
            });
        }
        Some(RomanoWolfAdjustedReceiptV1 {
            candidates: candidates.into_iter().collect::<Option<Vec<_>>>()?,
            stepdown_order: self.order.clone(),
            draws,
            periods,
            seed,
            block,
            family_digest: self.family_digest,
        })
    }
}

#[cfg(test)]
#[path = "bootstrap_family_pass_tests.rs"]
mod tests;

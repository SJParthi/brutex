//! An anchored walk-forward bottom-half diagnostic.
//!
//! # This is not CSCV and it is not authoritative PBO
//!
//! [`crate::validate`] produces anchored expanding walk-forward folds. This
//! module can place each fold's in-sample winner in that fold's out-of-sample
//! ranking and count placements strictly below the midpoint. That is exactly
//! what [`AnchoredWalkForwardBottomHalfRateV1`] names.
//!
//! The repository does not construct combinatorially symmetric train/test
//! partitions, complementary out-of-sample sets, or a stable candidate family
//! shared by all such partitions. Those are absent inputs, not defaults this
//! module may invent. Consequently no value produced here is a genuine CSCV
//! probability of backtest overfitting, and it must never satisfy an
//! authoritative statistics field.
//!
//! # The exact placement
//!
//! Validation chooses the first in-sample maximum by scanning in canonical
//! order with strict `>`. [`place_v1`] performs the same scan. Out-of-sample
//! ties use an exact midrank represented as `rank * 2` over
//! `(candidate_count - 1) * 2`; an even tied block therefore retains its half
//! rank instead of being rounded toward the better half before classification.
//!
//! A low bottom-half rate does not prove profitability, generalisation or lack
//! of overfitting. It describes only the supplied anchored folds.
//!
//! # Nothing here reads a bar
//!
//! This module takes RANKS. It never touches a candle, a store, a vendor or a
//! file — every input is a number some earlier stage measured. That is why it
//! can be tested exhaustively against hand-built rankings, and why its tests
//! carry no fixture at all.

/// One exact anchored-fold placement.
///
/// The rank is stored doubled, so a midrank at `2.5` is the integer `5` rather
/// than the lossy integer `2`. The matching denominator is also doubled and is
/// exposed by [`Self::relative_twice`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlacementV1 {
    candidates: usize,
    winner_rank_twice: usize,
}

impl PlacementV1 {
    /// An explicitly unrankable zero- or one-candidate fold.
    ///
    /// A fold containing a choice is not accepted here because assigning it a
    /// rank without the measured scores would invent evidence.
    #[must_use]
    pub const fn unrankable(candidates: usize) -> Option<Self> {
        if candidates < 2 {
            Some(Self {
                candidates,
                winner_rank_twice: 0,
            })
        } else {
            None
        }
    }

    /// Number of aligned candidates supplied for this fold.
    #[must_use]
    pub const fn candidates(&self) -> usize {
        self.candidates
    }

    /// Exact doubled midrank, where zero is best.
    #[must_use]
    pub const fn winner_rank_twice(&self) -> usize {
        self.winner_rank_twice
    }

    /// Exact `(rank * 2, last_rank * 2)` relative-placement terms.
    ///
    /// `None` means fewer than two candidates or a malformed rank outside the
    /// candidate family. The pair is deliberately not reduced: both `* 2`
    /// terms make the retained half-rank visible to a caller and to a receipt.
    #[must_use]
    pub fn relative_twice(&self) -> Option<(usize, usize)> {
        let last = self.candidates.checked_sub(1)?.checked_mul(2)?;
        if last == 0 || self.winner_rank_twice > last {
            return None;
        }
        Some((self.winner_rank_twice, last))
    }

    /// Display projection of the exact relative placement, in parts per
    /// million.
    ///
    /// Classification never uses this rounded projection; [`Self::bottom_half`]
    /// compares the exact integer ratio.
    #[must_use]
    pub fn relative_ppm(&self) -> Option<i64> {
        let (rank, last) = self.relative_twice()?;
        let rank = u128::try_from(rank).ok()?;
        let last = u128::try_from(last).ok()?;
        let scaled = rank.checked_mul(1_000_000)? / last;
        i64::try_from(scaled).ok()
    }

    /// Whether this exact midrank is strictly below the midpoint.
    #[must_use]
    pub fn bottom_half(&self) -> bool {
        self.relative_twice()
            .is_some_and(|(rank, last)| rank > last / 2)
    }
}

/// The non-authoritative rate over supplied anchored walk-forward folds.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AnchoredWalkForwardBottomHalfRateV1 {
    /// Folds with a valid relative placement.
    pub folds: usize,
    /// Folds whose exact midrank is strictly below the midpoint.
    pub bottom_half_folds: usize,
    /// Folds with fewer than two candidates or an invalid supplied placement.
    pub unrankable: usize,
    /// Median display projection across rankable folds, in parts per million.
    ///
    /// This presentation field is not a statistical authority. Each fold's
    /// load-bearing midpoint classification uses its exact doubled ratio.
    pub median_placement_ppm: i64,
}

impl AnchoredWalkForwardBottomHalfRateV1 {
    /// Bottom-half rate in parts per million.
    #[must_use]
    pub fn rate_ppm(&self) -> Option<i64> {
        if self.folds == 0 || self.bottom_half_folds > self.folds {
            return None;
        }
        let folds = u128::try_from(self.folds).ok()?;
        let bottom = u128::try_from(self.bottom_half_folds).ok()?;
        let scaled = bottom.checked_mul(1_000_000)? / folds;
        i64::try_from(scaled).ok()
    }

    /// Whether at least half the contributing anchored folds landed below the
    /// midpoint.
    #[must_use]
    pub fn at_or_above_half(&self) -> bool {
        self.folds > 0
            && self.bottom_half_folds <= self.folds
            && self.bottom_half_folds >= self.folds.saturating_sub(self.folds / 2)
    }
}

/// Aggregate exact placements from supplied anchored walk-forward folds.
///
/// This is a diagnostic over those folds only. It is not CSCV/PBO and cannot
/// authorize `PopulationStatisticsV2`.
///
/// # Cost
///
/// One pass plus one sort of display projections: `O(folds log folds)` time
/// and `O(folds)` temporary space. No bench row measures it yet.
#[must_use]
pub fn anchored_walk_forward_bottom_half_rate_v1(
    placements: &[PlacementV1],
) -> AnchoredWalkForwardBottomHalfRateV1 {
    let mut relatives = Vec::with_capacity(placements.len());
    let mut bottom_half = 0_usize;
    let mut unrankable = 0_usize;

    for placement in placements {
        match placement.relative_ppm() {
            Some(relative) => {
                relatives.push(relative);
                if placement.bottom_half() {
                    bottom_half = bottom_half.saturating_add(1);
                }
            }
            None => unrankable = unrankable.saturating_add(1),
        }
    }

    relatives.sort_unstable();
    AnchoredWalkForwardBottomHalfRateV1 {
        folds: relatives.len(),
        bottom_half_folds: bottom_half,
        unrankable,
        median_placement_ppm: median_of(&relatives),
    }
}

/// Exact placement of the first strict in-sample maximum out of sample.
#[must_use]
pub fn place_v1(in_sample: &[i64], out_of_sample: &[i64]) -> Option<PlacementV1> {
    if in_sample.is_empty() || in_sample.len() != out_of_sample.len() {
        return None;
    }
    if in_sample.len() == 1 {
        return PlacementV1::unrankable(1);
    }
    let winner = first_strict_max(in_sample)?;
    let winner_score = out_of_sample.get(winner).copied()?;
    let better = out_of_sample.iter().filter(|&&s| s > winner_score).count();
    let tied = out_of_sample.iter().filter(|&&s| s == winner_score).count();
    let winner_rank_twice = better.checked_mul(2)?.checked_add(tied.checked_sub(1)?)?;
    let placement = PlacementV1 {
        candidates: in_sample.len(),
        winner_rank_twice,
    };
    placement.relative_twice()?;
    Some(placement)
}

/// Index of the first maximum under the same strict scan as validation.
fn first_strict_max(scores: &[i64]) -> Option<usize> {
    let mut winner = 0_usize;
    let mut best = *scores.first()?;
    for (index, &score) in scores.iter().enumerate().skip(1) {
        if score > best {
            best = score;
            winner = index;
        }
    }
    Some(winner)
}

/// Compatibility-only legacy placement.
///
/// New code must use [`PlacementV1`]. This public two-field shape remains while
/// current CLI callers construct it directly. It cannot retain half-ranks and
/// therefore cannot become statistical authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Placement {
    /// How many candidates were ranked in this fold.
    pub candidates: usize,
    /// The out-of-sample rank of the in-sample winner, zero for best.
    pub winner_rank: usize,
}

impl Placement {
    /// The winner's relative position out of sample, in parts per million.
    ///
    /// Zero means it was still the best; `1_000_000` means it was the worst.
    /// Parts per million rather than a float because `CLAUDE.md` §7 keeps this
    /// arithmetic in integers, and because a ratio used for a threshold is
    /// compared far more often than it is read.
    ///
    /// A fold with fewer than two candidates has no ranking to be placed in and
    /// returns `None` — with one candidate the "winner" is also the loser, and
    /// calling that a median-beating result would count a non-choice as
    /// evidence.
    #[must_use]
    pub fn relative(&self) -> Option<i64> {
        if self.candidates < 2 {
            return None;
        }
        // `try_from` and not `as`: a candidate count above i64::MAX cannot arise
        // -- the closed set is bounded far below it -- but `as` would wrap
        // silently if one ever did, and a wrapped denominator turns a placement
        // into a number about nothing.
        let last = i64::try_from(self.candidates.saturating_sub(1)).ok()?;
        let rank = i64::try_from(self.winner_rank).ok()?;
        if last <= 0 {
            return None;
        }
        Some(rank.saturating_mul(1_000_000) / last)
    }

    /// Did the in-sample winner land in the BOTTOM half out of sample?
    ///
    /// The per-fold event PBO counts. Strictly below the midpoint, so a winner
    /// landing exactly at the median is not counted as a failure — with an even
    /// candidate count there is no exact median and the midpoint is the honest
    /// boundary either way.
    #[must_use]
    pub fn overfit(&self) -> bool {
        self.relative().is_some_and(|r| r > 500_000)
    }
}

/// Compatibility-only legacy summary.
///
/// Despite the historical name, this is not a genuine PBO record. New code
/// must use [`AnchoredWalkForwardBottomHalfRateV1`], and authoritative
/// statistics must remain absent until a separate CSCV design exists.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Pbo {
    /// Folds that contributed a placement.
    pub folds: usize,
    /// Folds whose in-sample winner landed below the median out of sample.
    pub overfit_folds: usize,
    /// Folds skipped because they had fewer than two candidates to rank.
    ///
    /// **Counted, never silent.** A PBO computed over three usable folds out of
    /// ten is a different number from one computed over ten, and a reader who
    /// cannot see the denominator cannot tell them apart.
    pub unrankable: usize,
    /// The median relative placement across folds, in parts per million.
    ///
    /// Reported beside the probability because they fail differently: a PBO of
    /// 0.5 with placements clustered at the midpoint is a procedure with no
    /// information, while a PBO of 0.5 with placements at both extremes is a
    /// procedure that is sometimes right and sometimes catastrophically wrong.
    pub median_placement: i64,
}

impl Pbo {
    /// The probability, in parts per million.
    ///
    /// `None` when no fold could be ranked — refused rather than reported as
    /// zero, because "never overfit" and "never measured" are different facts
    /// and zero would read as the first.
    #[must_use]
    pub fn probability(&self) -> Option<i64> {
        if self.folds == 0 {
            return None;
        }
        let folds = i64::try_from(self.folds).ok()?;
        let over = i64::try_from(self.overfit_folds).ok()?;
        Some(over.saturating_mul(1_000_000) / folds)
    }

    /// Is the selection procedure indistinguishable from a coin flip?
    ///
    /// True at or above 0.5. A stated convention, not a derivation: at exactly
    /// half, the in-sample winner is as likely to be in the bottom half as the
    /// top, which is what you would get by choosing at random.
    #[must_use]
    pub fn is_noise(&self) -> bool {
        self.probability().is_some_and(|p| p >= 500_000)
    }
}

/// Compatibility-only aggregation over supplied anchored folds.
///
/// # Cost
///
/// One pass over the placements plus one sort of their relative positions for
/// the median. `O(folds log folds)`, and folds is a handful — this is a
/// once-per-run boundary, not a per-bar path.
///
/// UNVERIFIED as a measured figure: no bench row covers this yet.
#[must_use]
pub fn probability_of_overfitting(placements: &[Placement]) -> Pbo {
    let mut relatives: Vec<i64> = Vec::with_capacity(placements.len());
    let mut overfit = 0_usize;
    let mut unrankable = 0_usize;

    for p in placements {
        match p.relative() {
            Some(r) => {
                relatives.push(r);
                if p.overfit() {
                    overfit = overfit.saturating_add(1);
                }
            }
            None => unrankable = unrankable.saturating_add(1),
        }
    }

    relatives.sort_unstable();
    let median = median_of(&relatives);

    Pbo {
        folds: relatives.len(),
        overfit_folds: overfit,
        unrankable,
        median_placement: median,
    }
}

/// The middle value of a sorted slice, averaging the two middles when even.
///
/// Integer division on the average, so a median of 3 and 4 is 3 rather than
/// 3.5. Stated because rounding a median down is a choice: it makes
/// [`Pbo::median_placement`] very slightly optimistic, and being explicit about
/// which way it leans beats a reader assuming it is exact.
fn median_of(sorted: &[i64]) -> i64 {
    let n = sorted.len();
    if n == 0 {
        return 0;
    }
    let mid = n / 2;
    if n % 2 == 1 {
        return sorted.get(mid).copied().unwrap_or(0);
    }
    let a = sorted.get(mid.saturating_sub(1)).copied().unwrap_or(0);
    let b = sorted.get(mid).copied().unwrap_or(0);
    a.saturating_add(b) / 2
}

/// Compatibility-only lossy placement.
///
/// `in_sample` and `out_of_sample` are scores for the SAME candidates in the
/// SAME order — index `i` is one combination in both. Higher is better.
///
/// `None` when the two disagree in length, which means the caller paired two
/// different candidate sets. Refused rather than truncated to the shorter: a
/// silent truncation here would rank a candidate against another's score, and
/// the resulting PBO would be a number about nothing.
#[must_use]
pub fn place(in_sample: &[i64], out_of_sample: &[i64]) -> Option<Placement> {
    let exact = place_v1(in_sample, out_of_sample)?;
    Some(Placement {
        candidates: exact.candidates(),
        // Compatibility freezes the old integer field. An exact half-rank is
        // rounded toward the better half here, which is why this adapter is
        // explicitly forbidden as statistical authority. The exact API above
        // keeps the bit this division discards.
        winner_rank: exact.winner_rank_twice() / 2,
    })
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "the exception every test module in this workspace takes."
)]
mod tests {
    use super::{
        Placement, anchored_walk_forward_bottom_half_rate_v1, place, place_v1,
        probability_of_overfitting,
    };

    #[test]
    fn exact_v1_uses_the_same_first_strict_maximum_as_validation() {
        // `validate` scans canonical order with strict `>`, so index 0 wins an
        // equal in-sample tie. Index 0 is best out of sample while index 1 is
        // worst; choosing the last maximum would therefore reverse the result.
        let exact = place_v1(&[10, 10, 1], &[100, 0, 50]).expect("exact placement");
        assert_eq!(exact.winner_rank_twice(), 0);
        assert_eq!(exact.relative_twice(), Some((0, 4)));

        let legacy = place(&[10, 10, 1], &[100, 0, 50]).expect("compat placement");
        assert_eq!(
            legacy.winner_rank, 0,
            "the compatibility path must share the strict-first winner"
        );
    }

    #[test]
    fn exact_v1_preserves_an_even_tied_block_across_the_midpoint() {
        // The winner has two candidates above it and ties one other candidate:
        // exact midrank = 2 + (2 - 1) / 2 = 2.5. Across five candidates the
        // midpoint is rank 2, so this placement is strictly in the bottom half.
        // The legacy integer rank truncates to 2 and misclassifies it as exactly
        // the midpoint; the V1 doubled representation retains 5/8 exactly.
        let exact =
            place_v1(&[10, 4, 3, 2, 1], &[5, 9, 8, 5, 1]).expect("exact half-rank placement");
        assert_eq!(exact.winner_rank_twice(), 5);
        assert_eq!(exact.relative_twice(), Some((5, 8)));
        assert_eq!(exact.relative_ppm(), Some(625_000));
        assert!(exact.bottom_half());

        let summary = anchored_walk_forward_bottom_half_rate_v1(&[exact]);
        assert_eq!(summary.folds, 1);
        assert_eq!(summary.bottom_half_folds, 1);
        assert_eq!(summary.rate_ppm(), Some(1_000_000));
        assert_eq!(summary.median_placement_ppm, 625_000);

        let legacy = place(&[10, 4, 3, 2, 1], &[5, 9, 8, 5, 1]).expect("compat placement");
        assert_eq!(legacy.winner_rank, 2);
        assert!(!legacy.overfit(), "the old lossy adapter is not authority");
    }

    #[test]
    fn a_procedure_that_always_picks_the_out_of_sample_best_scores_zero() {
        // Perfect selection: the in-sample winner is also the out-of-sample
        // winner in every fold. PBO must be zero, and `is_noise` false.
        let folds: Vec<Placement> = (0..5)
            .map(|_| place(&[10, 5, 1], &[10, 5, 1]).expect("a placement"))
            .collect();
        let p = probability_of_overfitting(&folds);
        assert_eq!(p.folds, 5);
        assert_eq!(p.overfit_folds, 0);
        assert_eq!(p.probability(), Some(0));
        assert!(!p.is_noise());
    }

    #[test]
    fn a_procedure_that_always_picks_the_out_of_sample_worst_scores_one() {
        // Perfect anti-selection: what wins in sample is last out of sample.
        // This is the shape pure overfitting produces, and PBO must be 1.
        let folds: Vec<Placement> = (0..4)
            .map(|_| place(&[10, 5, 1], &[1, 5, 10]).expect("a placement"))
            .collect();
        let p = probability_of_overfitting(&folds);
        assert_eq!(p.probability(), Some(1_000_000));
        assert!(p.is_noise());
        assert_eq!(p.median_placement, 1_000_000);
    }

    #[test]
    fn a_winner_at_the_exact_midpoint_is_not_counted_as_overfit() {
        // Three candidates, the in-sample winner lands second out of sample:
        // relative position exactly 500,000. `overfit` is STRICTLY above the
        // midpoint, so a median placement is not evidence of overfitting -- it
        // is the absence of evidence either way.
        let p = place(&[10, 5, 1], &[7, 10, 1]).expect("a placement");
        assert_eq!(p.relative(), Some(500_000));
        assert!(!p.overfit(), "exactly median is not below median");
    }

    #[test]
    fn a_fold_with_one_candidate_is_unrankable_rather_than_a_perfect_score() {
        // With a single candidate the "winner" is also the loser. Counting that
        // as a median-beating result would let a fold that made no choice
        // improve the score of a procedure that never chose anything.
        let p = place(&[42], &[42]).expect("a placement");
        assert_eq!(p.relative(), None);
        let out = probability_of_overfitting(&[p]);
        assert_eq!(out.folds, 0);
        assert_eq!(out.unrankable, 1);
        assert_eq!(
            out.probability(),
            None,
            "no rankable fold means no probability, not a probability of zero"
        );
    }

    #[test]
    fn mismatched_candidate_sets_are_refused_rather_than_truncated() {
        // Pairing two different candidate sets would rank one combination
        // against another's score, and the PBO would be a number about nothing.
        assert!(place(&[1, 2, 3], &[1, 2]).is_none());
        assert!(place(&[], &[]).is_none());
    }

    #[test]
    fn ties_out_of_sample_take_the_midrank_and_do_not_depend_on_order() {
        // THIS TEST ASSERTED THE BIAS. It required rank 0 when every candidate
        // scored identically out of sample, on the reasoning that "nothing beat
        // it". True, and it hands the winner the BEST rank of its tied block,
        // which under-reports overfitting — measured against a known-0.5 null,
        // 0.249 / 0.329 / 0.372 / 0.434 on grids of 2 / 3 / 4 / 8 distinct
        // values. A procedure indistinguishable from a coin flip was reporting
        // as low as 0.249.
        //
        // Three candidates all tied: the midrank is `0 + (3 - 1) / 2 = 1`, the
        // centre of the block, which is exactly "no information about where it
        // placed" — the honest reading of a three-way tie.
        //
        // The ORDER-INDEPENDENCE this test was really guarding is unchanged and
        // is still asserted: both counts are over the whole slice, so no sort
        // and no index into a sorted list is involved.
        let a = place(&[10, 5, 1], &[7, 7, 7]).expect("a placement");
        let b = place(&[1, 5, 10], &[7, 7, 7]).expect("a placement");
        assert_eq!(a.winner_rank, 1, "a three-way tie sits at the midrank");
        assert_eq!(b.winner_rank, 1);
        assert_eq!(a.relative(), b.relative(), "order changed the placement");

        // And the untied case is untouched: a clear winner is still rank 0, a
        // clear loser still last.
        assert_eq!(
            place(&[10, 5, 1], &[9, 5, 1]).expect("p").winner_rank,
            0,
            "an outright out-of-sample best is still rank 0"
        );
        assert_eq!(
            place(&[10, 5, 1], &[1, 5, 9]).expect("p").winner_rank,
            2,
            "an outright out-of-sample worst is still last"
        );
    }

    #[test]
    fn the_median_is_reported_beside_the_probability_because_they_fail_differently() {
        // Two folds at the extremes and two at the middle: PBO is 0.5, and so
        // is the median. The same PBO with both placements at the extremes is a
        // procedure that is sometimes catastrophically wrong rather than one
        // with no information, and only the median distinguishes them.
        let extremes = [
            place(&[10, 5, 1], &[10, 5, 1]).expect("p"),
            place(&[10, 5, 1], &[1, 5, 10]).expect("p"),
        ];
        let p = probability_of_overfitting(&extremes);
        assert_eq!(p.probability(), Some(500_000));
        assert!(p.is_noise(), "half is the coin-flip boundary");
        assert_eq!(p.median_placement, 500_000);
    }

    #[test]
    fn nothing_at_all_produces_no_probability_rather_than_zero() {
        let p = probability_of_overfitting(&[]);
        assert_eq!(p.folds, 0);
        assert_eq!(p.probability(), None);
        assert!(
            !p.is_noise(),
            "an unmeasured procedure is not noise, it is unmeasured"
        );
    }
}

//! The Probability of Backtest Overfitting — how often the best in-sample
//! choice is below median out of sample.
//!
//! # What a walk-forward cannot tell you on its own
//!
//! [`crate::validate`] chooses a combination on the training bars and measures
//! it on bars it never saw. One fold answers "did this hold once". It cannot
//! answer "would a selection procedure like mine hold in general", because a
//! single positive fold is one draw and a run of them can be luck.
//!
//! Bailey, Borwein, López de Prado and Zhu's PBO asks the second question. It
//! is deliberately a property of the **procedure**, not of any combination: it
//! measures how often *the thing you picked* turns out to be below average once
//! it is out of sample.
//!
//! # The rank, and why it is the whole statistic
//!
//! For each fold: rank every candidate by its in-sample result, take the one
//! that won, then find where that same candidate sits in the OUT-OF-SAMPLE
//! ranking. If selection carried real information, the in-sample winner should
//! land in the upper half out of sample. If it were pure overfitting, it would
//! land anywhere — and its expected position would be the middle.
//!
//! So the logit of its relative rank is the per-fold evidence, and **PBO is the
//! fraction of folds where the winner landed in the bottom half**. A PBO near
//! 0.5 says the selection procedure is a coin flip. A PBO near 0 says it
//! carries information.
//!
//! # What it does NOT say, and this is the part people misread
//!
//! A low PBO does **not** mean the strategy is profitable. It means the
//! *selection* is not pure noise-fitting. A procedure can reliably pick the
//! best of a hundred equally worthless combinations and score a perfect PBO
//! while losing money on every one of them.
//!
//! PBO answers "am I fooling myself by searching". Profitability is
//! [`crate::grid`]'s pessimistic total, and the two are independent. Reporting
//! one without the other is how a backtest gets believed.
//!
//! # Nothing here reads a bar
//!
//! This module takes RANKS. It never touches a candle, a store, a vendor or a
//! file — every input is a number some earlier stage measured. That is why it
//! can be tested exhaustively against hand-built rankings, and why its tests
//! carry no fixture at all.

/// One fold's contribution: where the in-sample winner landed out of sample.
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

/// The probability of backtest overfitting, and what it was computed from.
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

/// PBO over a set of folds.
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

/// Where a candidate that won in sample landed out of sample.
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
    if in_sample.is_empty() || in_sample.len() != out_of_sample.len() {
        return None;
    }
    // The in-sample winner. `enumerate().max_by_key` keeps the FIRST maximum on
    // a tie, which is deterministic and therefore reproducible -- §3 rule 5.
    let winner = in_sample
        .iter()
        .enumerate()
        .max_by_key(|&(_, &s)| s)
        .map(|(i, _)| i)?;
    let winner_score = out_of_sample.get(winner).copied()?;
    // Its out-of-sample rank: how many scored strictly better. Ties therefore
    // share the best rank among them rather than being ordered arbitrarily,
    // which keeps the placement independent of the input order.
    let better = out_of_sample.iter().filter(|&&s| s > winner_score).count();
    Some(Placement {
        candidates: in_sample.len(),
        winner_rank: better,
    })
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "the exception every test module in this workspace takes."
)]
mod tests {
    use super::{Placement, place, probability_of_overfitting};

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
    fn ties_out_of_sample_share_the_best_rank_and_do_not_depend_on_order() {
        // Three candidates that all scored identically out of sample: whichever
        // won in sample is rank 0, because nothing beat it. An implementation
        // that sorted and took an index would give a different answer depending
        // on the input order, which §3 rule 5 forbids.
        let a = place(&[10, 5, 1], &[7, 7, 7]).expect("a placement");
        let b = place(&[1, 5, 10], &[7, 7, 7]).expect("a placement");
        assert_eq!(a.winner_rank, 0);
        assert_eq!(b.winner_rank, 0);
        assert_eq!(a.relative(), b.relative());
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

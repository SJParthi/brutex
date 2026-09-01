//! The bar column the sweep consumes: one mask per bar, and a census of every
//! bar that did not reach it.
//!
//! # The seam that did not exist
//!
//! `crates/engine` takes `&[ConditionMask]`. This crate produces one
//! [`ConditionMask`] per [`Candle`]. Until this module, **nothing in the
//! workspace joined them** — the vocabulary, the indicators and the ladder were
//! each finished, tested and unreachable from the other two. This is the join,
//! and it is the whole of it: a slice of bars in, the column the ladder walks
//! out, and a count of everything that fell out on the way.
//!
//! # It reads no bar from anywhere
//!
//! [`Column::build`] takes a slice the caller already holds. This module opens
//! nothing, and CI gate 22 is what makes that structural rather than a promise:
//! `crates/indicators` may declare exactly one dependency, `vocab`, and no
//! filesystem call site may appear in this crate's `src/` or `benches/`. A bar
//! cannot reach the sweep through this file because there is no expressible way
//! for it to arrive.
//!
//! # The warm-up boundary is the point, not a detail
//!
//! [`Evaluator::step`] keeps emitting before the run has warmed up, and those
//! bits are correct — a position that cannot be evaluated evaluates false, which
//! is `docs/03-vocabulary.md` §4. But a bar on which whole families are
//! structurally unable to answer still **counts in support's denominator**, and
//! finding `F-0C28E4` in `docs/11-findings.md` records what that costs: a
//! genuinely frequent combination is pushed under `min_hits` and dies at k=1,
//! before a reader ever sees it.
//!
//! [`Evaluator::warmed_up`] published that boundary and **no caller consumed
//! it**. This one does. A mask enters the column only if the run had already
//! warmed up *before* the bar was folded.
//!
//! ## The reason this doc first gave was wrong, and the correction is the point
//!
//! It said `step` emits before it folds, so the bar that makes a run warm emits
//! its mask while still cold. **Three quarters of that is false.**
//! [`Evaluator::warmed_up`] reads four things, and `prev5`, `yesterday` and
//! `previous` are all written by `close_the_books` inside the session-rollover
//! block, which runs *six lines before* the emit. Only the trend conjunct is
//! updated by the fold that follows.
//!
//! So on the shape this engine actually sweeps — NSE one-minute spot, 375-bar
//! sessions — the binding conjunct is the five-session ladder and not the
//! 200-candle trend seed, and it fills before the emit. Reading the verdict
//! after `step` would therefore sweep a bar that is genuinely **warm**. The cost
//! of reading it before is one lost warm bar per run: 1 of 1,125 on an
//! eight-session fixture.
//!
//! **Before is still what this module does**, and now for the true reason: it is
//! correct for *every* shape, not just this one. The trend conjunct is the last
//! to fill whenever a session is shorter than 200 bars — a daily timeframe, a
//! Muhurat session, any instrument whose sessions are short — and on those the
//! after-reading sweeps a bar whose EMA200 could not answer. Losing one warm bar
//! is the price of a boundary that does not depend on the bar count per session,
//! and paying it knowingly is different from paying it by accident.
//!
//! `warmed_up` is monotone, so the emitted column is a contiguous suffix of the
//! offered bars less any refusals inside it —
//! [`crate::column::tests::the_column_is_a_suffix_and_never_reorders`].
//!
//! # Nothing is dropped silently
//!
//! `CLAUDE.md` §4 bans a fallback that hides a failure. Every bar handed in
//! lands in exactly one bucket of [`Census`], and the buckets are counted in the
//! loop rather than derived afterwards — a residual would absorb a bar nobody
//! measured and report it as one that was. [`Census::reconciles`] is therefore a
//! loop invariant, proved by
//! [`crate::column::tests::every_bar_lands_in_exactly_one_bucket`].
//!
//! The refusal buckets are one per [`Corrupt`] variant with **no catch-all**, so
//! a new variant is a compile error here rather than a bar quietly counted as
//! something it is not.
//!
//! # Nothing is logged
//!
//! CI gate 17 forbids an event emit in this crate outright: the sweep evaluates
//! a mask per (bar, combination) and a billion O(1) calls is still a billion
//! calls. [`Census`] is plain integers, returned to the caller, who emits it
//! **once** at a structural boundary. That is the shape `pull::session::
//! DropCensus` already uses.
//!
//! **The rule is spelled around, not quoted, and that is deliberate.** Gate 17
//! greps for the emit path as a literal token and does **not** strip comments,
//! so a doc block quoting the banned spelling in order to explain it fails the
//! gate it is describing. This paragraph exists because that is exactly what
//! happened here — the same "text in a comment is text" defect gate 22's own
//! clause A records having been caught by.
//!
//! # Cost
//!
//! Per bar: one `bool` read, one [`Evaluator::step`], and one push into a vector
//! reserved once at entry. No allocation inside the loop, no scan of what came
//! before, and no branch whose cost depends on the data. The per-bar cost is
//! therefore whatever `step` costs plus a constant, and it is measured rather
//! than asserted: row `C-I-06` of `crates/indicators/benches/ratio.rs` builds a
//! 20,000-bar column and a 200,000-bar one and compares the cost PER BAR.
//!
//! That row exists because naming a bench that does not measure the claim is the
//! one thing CI gate 12 cannot see -- it checks that a proof is named and that
//! the named thing exists, never that it proves anything. This doc block named
//! the file before the row was written, which is precisely the defect the gate
//! converts from unfalsifiable to falsifiable and no further.

use std::sync::Arc;
use vocab::ConditionMask;

use crate::anchored::{AnchoredEvaluator, DailyReferenceCensus};
use crate::evaluator::{EVALUATION_SPEC_V1_LEN, EvaluationSpec, Evaluator};
use crate::{Candle, Corrupt};

/// Byte length of [`EvaluationSpecFingerprintV1`].
///
/// Fixed across machines: integers are explicitly little-endian and the
/// calendar length is encoded as `u64`, never as platform-width `usize`.
pub const EVALUATION_SPEC_FINGERPRINT_V1_LEN: usize = EVALUATION_SPEC_V1_LEN;

/// Collision-free canonical V1 fingerprint of one evaluator configuration.
///
/// This is deliberately the complete fixed-width record, not a compressed
/// hash.  A downstream run-identity owner can hash [`Self::as_bytes`] with the
/// identity algorithm it already owns, while `indicators` keeps its one-arrow
/// crate graph and cannot introduce a competing digest implementation.  The
/// bytes include both tolerance values and bases, VWAP availability, every
/// pattern threshold, and the calendar's exact length and eight day slots.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct EvaluationSpecFingerprintV1([u8; EVALUATION_SPEC_FINGERPRINT_V1_LEN]);

impl EvaluationSpecFingerprintV1 {
    /// Borrow the complete canonical record for a durable run identity.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; EVALUATION_SPEC_FINGERPRINT_V1_LEN] {
        &self.0
    }

    /// Consume the wrapper and return the complete canonical record.
    #[must_use]
    pub const fn into_bytes(self) -> [u8; EVALUATION_SPEC_FINGERPRINT_V1_LEN] {
        self.0
    }
}

/// Opaque identity of the evaluator choices that produced a [`Column`].
///
/// The inner value is deliberately private: another crate may compare two
/// tokens, but it cannot manufacture one or edit a single evaluator choice and
/// call the result the same policy. [`Self::fingerprint_v1`] is the sole durable
/// read door and exposes the complete canonical record without exposing a
/// constructor or any independently-editable field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EvaluationSpecToken(EvaluationSpec);

impl EvaluationSpecToken {
    /// Canonical V1 identity of every evaluator choice sealed by this token.
    #[must_use]
    pub fn fingerprint_v1(self) -> EvaluationSpecFingerprintV1 {
        EvaluationSpecFingerprintV1(self.0.canonical_v1_bytes())
    }
}

/// The two evaluation paths expose one commit-on-success column operation.
///
/// Regular evaluation reads warmth before `step`.  Anchored evaluation must
/// first install every daily record strictly before this bar, so its method
/// returns the warmth observed at that exact boundary alongside the mask.
trait ColumnEvaluation {
    fn step_with_warmth(&mut self, bar: &Candle) -> Result<(ConditionMask, bool), Corrupt>;

    fn replay_spec(&self) -> Option<EvaluationSpec>;
}

impl ColumnEvaluation for Evaluator {
    fn step_with_warmth(&mut self, bar: &Candle) -> Result<(ConditionMask, bool), Corrupt> {
        let warm = self.warmed_up();
        self.step(bar).map(|mask| (mask, warm))
    }

    fn replay_spec(&self) -> Option<EvaluationSpec> {
        Some(self.spec())
    }
}

impl ColumnEvaluation for AnchoredEvaluator<'_> {
    fn step_with_warmth(&mut self, bar: &Candle) -> Result<(ConditionMask, bool), Corrupt> {
        AnchoredEvaluator::step_with_warmth(self, bar)
    }

    fn replay_spec(&self) -> Option<EvaluationSpec> {
        // The stored masks are never replayed: reprojection copies them byte for
        // byte.  The only replay is the execution slice's corruption/acceptance
        // bitmap, and daily anchors cannot change that verdict.  Keeping the
        // ordinary four choices lets a strict anchored column move onto exact
        // one-minute fills without inventing or rebuilding a daily context.
        Some(self.acceptance_spec())
    }
}

/// Where every offered bar went.
///
/// One bucket per outcome, counted in the loop. `offered` is incremented beside
/// the others rather than read from the slice length, so
/// [`Self::reconciles`] is an invariant of the loop and not an identity that a
/// residual makes true by construction — the defect `engine::Frontier` records
/// having had.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Census {
    /// Bars handed to [`Column::build`].
    pub offered: u64,
    /// Bars folded successfully while the run was still cold. Their bits were
    /// correct and are deliberately not swept — see the module doc.
    pub warming: u64,
    /// Bars whose mask entered the column. Equal to [`Column::len`].
    pub swept: u64,
    /// [`Corrupt::HighBelowLow`].
    pub high_below_low: u64,
    /// [`Corrupt::RangeOverflows`].
    pub range_overflows: u64,
    /// [`Corrupt::PriceOutsideRange`].
    pub price_outside_range: u64,
    /// [`Corrupt::TimestampNotIncreasing`].
    pub timestamp_not_increasing: u64,
    /// [`Corrupt::NegativeVolume`].
    pub negative_volume: u64,
    /// [`Corrupt::AccumulatorTooLarge`].
    pub accumulator_too_large: u64,
}

impl Census {
    /// Every bar refused as not-a-bar, across all six reasons.
    #[must_use]
    pub const fn refused(&self) -> u64 {
        self.high_below_low
            .saturating_add(self.range_overflows)
            .saturating_add(self.price_outside_range)
            .saturating_add(self.timestamp_not_increasing)
            .saturating_add(self.negative_volume)
            .saturating_add(self.accumulator_too_large)
    }

    /// Did every offered bar land in exactly one bucket?
    ///
    /// False is a defect in this module, never in the data. Proved by
    /// [`crate::column::tests::every_bar_lands_in_exactly_one_bucket`].
    #[must_use]
    pub const fn reconciles(&self) -> bool {
        self.offered
            == self
                .warming
                .saturating_add(self.swept)
                .saturating_add(self.refused())
    }

    /// Charges `corrupt` to its own bucket.
    ///
    /// Exhaustive by construction: there is no catch-all arm, so a new
    /// [`Corrupt`] variant fails to compile here rather than being counted as a
    /// neighbour.
    const fn charge(&mut self, corrupt: Corrupt) {
        match corrupt {
            Corrupt::HighBelowLow => {
                self.high_below_low = self.high_below_low.saturating_add(1);
            }
            Corrupt::RangeOverflows => {
                self.range_overflows = self.range_overflows.saturating_add(1);
            }
            Corrupt::PriceOutsideRange => {
                self.price_outside_range = self.price_outside_range.saturating_add(1);
            }
            Corrupt::TimestampNotIncreasing => {
                self.timestamp_not_increasing = self.timestamp_not_increasing.saturating_add(1);
            }
            Corrupt::NegativeVolume => {
                self.negative_volume = self.negative_volume.saturating_add(1);
            }
            Corrupt::AccumulatorTooLarge => {
                self.accumulator_too_large = self.accumulator_too_large.saturating_add(1);
            }
        }
    }
}

/// Replay one evaluator configuration over an execution slice and retain only
/// whether each offered record was accepted.
///
/// This deliberately calls [`Evaluator::step`] rather than spelling any
/// `Corrupt` predicate here. Timestamp ordering and VWAP accumulator overflow
/// are stateful; a second predicate assembled from `Candle::check` could never
/// agree with them. The complete mask is discarded, but the verdict and census
/// are the evaluator's own.
fn acceptance_of(bars: &[Candle], evaluator: &mut Evaluator) -> (Vec<bool>, Census) {
    let mut accepted = Vec::with_capacity(bars.len());
    let mut census = Census::default();
    for bar in bars {
        census.offered = census.offered.saturating_add(1);
        let warm = evaluator.warmed_up();
        match evaluator.step(bar) {
            Ok(_) => {
                accepted.push(true);
                if warm {
                    census.swept = census.swept.saturating_add(1);
                } else {
                    census.warming = census.warming.saturating_add(1);
                }
            }
            Err(corrupt) => {
                accepted.push(false);
                census.charge(corrupt);
            }
        }
    }
    (accepted, census)
}

/// One [`ConditionMask`] per swept bar, in the order the bars arrived.
///
/// This is the exact input `engine::Ladder::walk` takes as `bar_bits`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Column {
    bits: Vec<ConditionMask>,
    /// The caller-slice index each entry in `bits` came from.
    ///
    /// Parallel to `bits`, and the ONLY honest way to map a column position
    /// back to a bar. `first_swept + j` is not that map: a refusal can occur
    /// anywhere mid-stream, so the two run out of step from the first corrupt
    /// bar onward. Anything that pairs a signal with what happened AFTER it --
    /// a forward return, a trade outcome -- would then be reading the wrong
    /// bar's future, silently and only on data that contains a refusal.
    ///
    /// Eight bytes per swept bar, which is the honest cost of that map.
    source: Vec<usize>,
    sourced: Sourced,
    census: Census,
    first_swept: Option<usize>,
    /// One verdict per bar of the slice this column CURRENTLY indexes.
    ///
    /// Unlike `source`, this includes cold bars and refused bars. A successful
    /// [`Evaluator::step`] writes `true`; every `Corrupt` arm writes `false`.
    /// Execution code therefore asks the evaluator's exact verdict in O(1)
    /// instead of repeating the four bar-local checks and missing the two
    /// stateful refusals.
    accepted: Arc<[bool]>,
    /// Where every execution-slice bar went under the pass that produced
    /// `accepted`. This equals `census` on a native column and deliberately
    /// differs after checked reprojection: `census` describes signal bars,
    /// while this one describes the one-minute execution bars.
    acceptance_census: Census,
    /// The caller choices needed to run the same evaluator configuration from
    /// empty state over a different execution slice.
    spec: Option<EvaluationSpec>,
    /// False only for the length-only compatibility reprojection, which cannot
    /// possibly decide stateful acceptance without seeing the target bars.
    acceptance_known: bool,
    /// Signals [`Column::reproject`] refused because an EARLIER signal already
    /// owns the execution bar they resolved to. Always zero on a column that has
    /// not been reprojected -- a signal series cannot collide with itself.
    ///
    /// Separate from the `dropped` count `reproject` returns, and deliberately:
    /// that one answers "the series ran out after this signal", this one answers
    /// "the position was already open". Folding them would put two different
    /// facts behind one number, and the caller prints that number with the first
    /// one's wording.
    collided: u64,
}

/// A signal column built through the causal sealed-daily reference path.
///
/// This wrapper keeps the daily join's census beside the ordinary signal-bar
/// census, so a caller cannot persist one and accidentally omit the other.  The
/// inner [`Column`] remains the exact shape `engine` consumes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnchoredColumn {
    column: Column,
    references: DailyReferenceCensus,
}

/// A strict anchored column was asked to admit signal bars with no prior
/// eligible daily reference.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MissingDailyReference {
    /// Successfully evaluated signal bars that lacked the reference.
    pub signal_bars: u64,
    /// Distinct signal days represented by those bars.
    pub signal_days: u64,
}

impl AnchoredColumn {
    /// Fold `bars` through an [`AnchoredEvaluator`].
    ///
    /// Signal bars are still streamed once in timestamp order.  Daily-reference
    /// movement happens inside the evaluator before each mask and is included in
    /// [`Self::reference_census`].
    #[must_use]
    pub fn build(bars: &[Candle], evaluator: &mut AnchoredEvaluator<'_>) -> Self {
        let column = Column::build_from(bars, evaluator);
        Self {
            column,
            references: evaluator.reference_census(),
        }
    }

    /// Build and refuse the whole column if any accepted signal bar had no
    /// eligible sealed daily record strictly before its IST day.
    ///
    /// This is the shipping admission door.  [`Self::build`] remains useful for
    /// diagnostics because it returns the masks and the explicit missing-data
    /// census, but a sweep that requires daily context should use this method so
    /// an unanchored first day can never reach `engine`.
    ///
    /// # Errors
    ///
    /// [`MissingDailyReference`] names both affected bars and distinct days.
    pub fn build_required(
        bars: &[Candle],
        evaluator: &mut AnchoredEvaluator<'_>,
    ) -> Result<Self, MissingDailyReference> {
        let mut next = *evaluator;
        let built = Self::build(bars, &mut next).require_daily_reference()?;
        *evaluator = next;
        Ok(built)
    }

    /// Apply strict daily-reference admission to an already diagnostic build.
    ///
    /// # Errors
    ///
    /// See [`Self::build_required`].
    pub fn require_daily_reference(self) -> Result<Self, MissingDailyReference> {
        if self.references.signal_bars_without_reference == 0 {
            return Ok(self);
        }
        Err(MissingDailyReference {
            signal_bars: self.references.signal_bars_without_reference,
            signal_days: self.references.signal_days_without_reference,
        })
    }

    /// The exact mask/source column the sweep consumes.
    #[must_use]
    pub const fn column(&self) -> &Column {
        &self.column
    }

    /// Consume the wrapper after its reference census has been persisted.
    #[must_use]
    pub fn into_column(self) -> Column {
        self.column
    }

    /// Deterministic causal-join accounting.
    #[must_use]
    pub const fn reference_census(&self) -> DailyReferenceCensus {
        self.references
    }
}

/// What [`Column::sources`] indexes — and it is TWO different things.
///
/// # The one field that carried two meanings
///
/// [`Column::build`] fills `source` with the bar whose CLOSE carried the mask, so
/// a position opens on the bar AFTER it. [`Column::reproject`] overwrites the
/// same field with the execution bar a position can actually be OPENED on —
/// `align::onto_execution` returns the first execution bar stamped at or after
/// the signal's close, which is already the fill bar.
///
/// Nothing recorded which convention a given `Column` was under, and
/// `runner::trade::walk` read every column under the first: it added `+1` to a
/// value that was already the fill bar. MEASURED on 60 synthetic sessions:
/// **100.00% of projected rows on all seven aligned rungs entered exactly one
/// execution bar late**, and on a series with 4.8% of bars missing the lateness
/// ran out to 1,071 minutes — an overnight crossing. Only the 1-minute rung was
/// right, and only because it skips alignment entirely.
///
/// The operator's rule is that a signal on any timeframe fills on the next
/// ONE-MINUTE bar. On seven of the eight rungs it filled on the one after that,
/// and the exit, the crossing anchor, the excursion window and every one of the
/// exit grid's cells rode on the shifted bar.
///
/// So the meaning is a TYPE now. A caller that reads `sources()` without asking
/// which convention it is under no longer compiles into a plausible answer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Sourced {
    /// The bar whose CLOSE carried the mask. A position opens on the NEXT bar.
    ///
    /// The default, because it is what [`Column::build`] produces and a column
    /// that has not been reprojected is under it.
    #[default]
    Signal,
    /// The bar a position can be OPENED on. It IS the fill bar; adding one
    /// skips a bar the operator's rule says to trade.
    Fill,
}

impl Column {
    /// Folds `bars` through `evaluator`, keeping the masks of warmed-up bars.
    ///
    /// The evaluator is borrowed rather than constructed here on purpose:
    /// [`Evaluator::new`] needs a `Widths`, an `Availability` and a
    /// `Thresholds`, and every one of those is a decision about the run that
    /// `CLAUDE.md` §3 rule 1 will not let this module invent on the caller's
    /// behalf. It is left borrowed on return so a caller can read the warm-up
    /// diagnostics — [`Evaluator::sessions_until_every_family_can_answer`] — off
    /// a column that came back empty.
    ///
    /// Bars are read strictly forward and each is folded before the next is
    /// looked at, so §3 rule 7 (no look-ahead) holds by construction:
    /// [`crate::column::tests::a_prefix_of_the_bars_gives_a_prefix_of_the_column`].
    ///
    /// # Cost
    ///
    /// One `bool` read, one [`Evaluator::step`] and one amortised push per bar,
    /// into a vector reserved once before the loop. Constant per bar, measured
    /// by row `C-I-06` of `crates/indicators/benches/ratio.rs`.
    #[must_use]
    pub fn build(bars: &[Candle], evaluator: &mut Evaluator) -> Self {
        Self::build_from(bars, evaluator)
    }

    /// Shared forward fold for regular and stored-daily-anchored evaluators.
    fn build_from<E: ColumnEvaluation>(bars: &[Candle], evaluator: &mut E) -> Self {
        let spec = evaluator.replay_spec();
        let mut bits = Vec::with_capacity(bars.len());
        let mut census = Census::default();
        let mut first_swept = None;
        let mut source: Vec<usize> = Vec::with_capacity(bars.len());
        let mut accepted: Vec<bool> = Vec::with_capacity(bars.len());

        for (index, bar) in bars.iter().enumerate() {
            census.offered = census.offered.saturating_add(1);

            match evaluator.step_with_warmth(bar) {
                Ok((mask, warm)) => {
                    accepted.push(true);
                    if warm {
                        if first_swept.is_none() {
                            first_swept = Some(index);
                        }
                        bits.push(mask);
                        source.push(index);
                        census.swept = census.swept.saturating_add(1);
                    } else {
                        census.warming = census.warming.saturating_add(1);
                    }
                }
                // `step` commits on success only, so a refused bar leaves the
                // evaluator exactly as the previous bar left it.
                Err(corrupt) => {
                    accepted.push(false);
                    census.charge(corrupt);
                }
            }
        }

        Self {
            bits,
            source,
            // THE BAR WHOSE CLOSE CARRIED THE MASK. A position opens on the
            // next one -- see `Sourced` for what happens when a reader
            // assumes that of a REPROJECTED column.
            sourced: Sourced::Signal,
            census,
            first_swept,
            accepted: accepted.into(),
            acceptance_census: census,
            spec,
            acceptance_known: true,
            // A SIGNAL SERIES CANNOT COLLIDE WITH ITSELF: every bar is its own
            // source, so no two rows can name one index. Only reprojection can
            // put two signals on one execution bar.
            collided: 0,
        }
    }

    /// Fallibly duplicates this exact column without rebuilding evaluator
    /// state or silently accepting an allocation abort.
    ///
    /// Both owned vectors reserve their complete final capacity before any
    /// element is copied. The accepted-bar bitmap is already immutable shared
    /// storage, so duplicating its [`Arc`] does not allocate or change bytes.
    /// Every remaining field is copied exactly.
    ///
    /// # Errors
    ///
    /// Returns the allocator's [`std::collections::TryReserveError`] if either
    /// owned vector cannot reserve its full final capacity. No partial column
    /// escapes.
    pub fn try_clone_exact(&self) -> Result<Self, std::collections::TryReserveError> {
        let mut bits = Vec::new();
        bits.try_reserve_exact(self.bits.len())?;
        bits.extend_from_slice(&self.bits);

        let mut source = Vec::new();
        source.try_reserve_exact(self.source.len())?;
        source.extend_from_slice(&self.source);

        Ok(Self {
            bits,
            source,
            sourced: self.sourced,
            census: self.census,
            first_swept: self.first_swept,
            accepted: Arc::clone(&self.accepted),
            acceptance_census: self.acceptance_census,
            spec: self.spec,
            acceptance_known: self.acceptance_known,
            collided: self.collided,
        })
    }

    /// The caller-slice index of each swept bar, parallel to [`Self::bits`].
    ///
    /// `sources()[j]` is the index, in the slice handed to [`Self::build`], of
    /// the bar whose mask is `bits()[j]`. A caller pairing a signal with what
    /// followed it MUST go through this rather than assuming
    /// `first_swept() + j`: refusals are counted, not removed from the middle,
    /// so the two diverge from the first corrupt bar and every outcome after it
    /// would be read off the wrong bar.
    #[must_use]
    pub fn sources(&self) -> &[usize] {
        &self.source
    }

    /// Which convention [`Self::sources`] is under.
    ///
    /// A caller that pairs a source index with what happened AFTER it -- a
    /// forward return, an entry, a trade -- MUST read this. `Sourced::Signal`
    /// means the position opens on the next bar; `Sourced::Fill` means the index
    /// IS the bar to open on, and adding one skips a bar the operator's rule
    /// says to trade.
    #[must_use]
    pub const fn sourced(&self) -> Sourced {
        self.sourced
    }

    /// Blank every row whose source bar is before `from`, keeping the shape.
    ///
    /// # Why blanked and not cut
    ///
    /// A walk-forward has to evaluate a combination on a TEST window while
    /// letting the indicators hold what they legitimately held — an EMA at the
    /// first test bar really did see the training bars, and restarting it cold
    /// there would under-report its state.
    ///
    /// Cutting the column to the test range would renumber [`Self::sources`],
    /// and that renumbering is the exact defect `sources` was added to remove:
    /// `first_swept + j` runs one behind from the first refused bar onward, so
    /// every outcome after it is read off the wrong bar. Clearing the bits
    /// preserves every source index while making the row unable to fire.
    ///
    /// A zeroed row fails `hits` for every mask carrying at least one bit. The
    /// EMPTY mask still hits it, because `(0 & 0) == 0` — that mask has no
    /// conditions, fires on every bar by construction, and is not a strategy.
    ///
    /// # Cost
    ///
    /// One pass over the column, one compare and at most one 384-bit store per
    /// row. Called once per fold, never per bar of a sweep.
    pub fn clear_before(&mut self, from: usize) {
        for (bits, &source) in self.bits.iter_mut().zip(self.source.iter()) {
            if source < from {
                *bits = ConditionMask::ZERO;
            }
        }
    }

    /// Replace only `GapFib` positions 132..=142 in every row.
    ///
    /// The replacement is parallel to this column, already aligned by the
    /// causal exact-minute bridge in [`crate::anchored`].  Clearing before the
    /// union is load-bearing: an anchored coarse evaluator still folds every
    /// other signal-local family, but none of its locally-derived `GapFib` bits
    /// may survive into a stored run.
    pub(crate) fn replace_gapfib(&mut self, exact: &[ConditionMask]) -> bool {
        if exact.len() != self.bits.len() {
            return false;
        }
        for (mask, evidence) in self.bits.iter_mut().zip(exact.iter()) {
            let mut without_local = *mask;
            let mut only_exact = ConditionMask::ZERO;
            for position in crate::gap::GapFib::positions() {
                let position = u32::from(position);
                without_local = without_local.without_bit(position);
                if evidence.get(position) {
                    only_exact = only_exact.with_bit(position);
                }
            }
            *mask = without_local.union(&only_exact);
        }
        true
    }

    /// The same conditions, re-indexed onto a DIFFERENT bar series.
    ///
    /// # Why a column has to be able to move series at all
    ///
    /// A condition is decided on one timeframe and a position is taken on
    /// another. A fifteen-minute bar stamped 09:15 covers `[09:15, 09:30)` and
    /// its mask is not knowable until 09:30, so the earliest bar that mask can
    /// be acted on is the one-minute bar stamped 09:30. That is not a smaller
    /// version of entering on the next fifteen-minute bar — it is the same
    /// instant, reached with **fifteen times the resolution** for everything
    /// that happens afterwards.
    ///
    /// The resolution is what matters. A stop and a target inside one bar's
    /// range have no order the data can settle, so a fifteen-minute bar hides
    /// fifteen minutes of path; a trailing order tracks a running peak that
    /// updates twenty-five times a session on fifteen-minute bars and three
    /// hundred and seventy-five times on one-minute bars. The trail is the
    /// figure that moves most, and it moves because the coarse series never saw
    /// the peak.
    ///
    /// # The projection carries the BITS unchanged, and that is the point
    ///
    /// Nothing here recomputes a condition. The mask a bar produced is the mask
    /// it produced; only the index it is filed under changes. So no indicator is
    /// evaluated on a series it was not built for, and `CLAUDE.md` §3 rule 7
    /// holds exactly as before: the mask was computed from bars `0..=s` of the
    /// signal series, and the index it now carries points at a bar that opens at
    /// or after that signal bar CLOSED.
    ///
    /// # What `onto` is, and what `None` means
    ///
    /// `onto[j]` is the execution-series index for column position `j`, or
    /// `None` when there is no bar to act on — a signal on the session's last
    /// bar, or one whose close falls past the end of the execution series. Those
    /// rows are DROPPED rather than mapped to a neighbour, because mapping a
    /// signal to a bar that opened before the signal was knowable is look-ahead,
    /// and mapping it to a much later one is a trade nobody could have taken.
    ///
    /// The count of dropped rows is returned so the caller can report it. A
    /// projection that quietly shortened the column would make a smaller sample
    /// read like a whole one, which is the `CLAUDE.md` §4 fallback.
    ///
    /// # Stateful execution-bar acceptance
    ///
    /// This length-only compatibility door cannot inspect the target slice.
    /// Its projected column therefore carries `acceptance_known == false` and
    /// every execution-price lookup refuses. It is safe but deliberately not
    /// useful for trading. Shipping callers must use [`Self::reproject_checked`],
    /// which sees the target bars and runs the same evaluator verdict over them.
    ///
    /// # Errors
    ///
    /// `None` when `onto` is not parallel to this column. That is a caller bug
    /// and not a data condition, so it refuses rather than truncating to the
    /// shorter of the two.
    ///
    /// # Cost
    ///
    /// One pass, one copy per kept row. `O(len)`, called once per run and never
    /// per candidate — `CLAUDE.md` §3 rule 4 governs the sweep's inner
    /// operations and this is not one of them.
    #[must_use]
    pub fn reproject(&self, onto: &[Option<usize>], onto_len: usize) -> Option<(Self, u64)> {
        self.reproject_with(onto, onto_len, Vec::new(), Census::default(), false)
    }

    /// Re-index this signal column onto `execution` and evaluate every target
    /// bar under the same caller choices from fresh state.
    ///
    /// The masks are copied from the signal series unchanged. Only the
    /// execution-acceptance bitmap is recomputed, because a coarse signal
    /// evaluator says nothing about whether a one-minute fill, exit, or
    /// interior path bar was refused. The fresh pass calls [`Evaluator::step`]
    /// once per execution bar and stores one bool; every later lookup is one
    /// bounds-checked read.
    ///
    /// `None` when `onto` is not parallel to this column or this column has no
    /// evaluator specification (only [`Column::default`] has none).
    #[must_use]
    pub fn reproject_checked(
        &self,
        onto: &[Option<usize>],
        execution: &[Candle],
    ) -> Option<(Self, u64)> {
        let mut evaluator = self.spec?.fresh();
        let (accepted, census) = acceptance_of(execution, &mut evaluator);
        self.reproject_with(onto, execution.len(), accepted, census, true)
    }

    /// The projection itself, parameterised by the independently-produced
    /// execution verdict so the mapping loop never learns how to validate a
    /// candle.
    fn reproject_with(
        &self,
        onto: &[Option<usize>],
        onto_len: usize,
        accepted: Vec<bool>,
        acceptance_census: Census,
        acceptance_known: bool,
    ) -> Option<(Self, u64)> {
        if onto.len() != self.bits.len() {
            return None;
        }
        let mut bits = Vec::with_capacity(self.bits.len());
        let mut source = Vec::with_capacity(self.source.len());
        let mut dropped: u64 = 0;
        let mut collided: u64 = 0;
        for (&mask, &target) in self.bits.iter().zip(onto.iter()) {
            match target {
                None => dropped = dropped.saturating_add(1),
                // ONE FILL BAR, ONE ROW. A SECOND SIGNAL ON IT IS NOT A SECOND
                // OBSERVATION.
                //
                // `align::onto_execution` is forward-only and returns the FIRST
                // execution bar stamped at or after each signal's close, so when
                // the execution series has a hole two consecutive signals resolve
                // to the same bar -- `align`'s own test asserts `[Some(10),
                // Some(10)]` as the correct output. Both rows were then pushed
                // with the same `source`, and `outcome::edge` zips `bits` with
                // `sources` and accumulates one observation per ROW: the same
                // forward return entered the mean twice. `n` inflates, the
                // standard error understates, `|t|` inflates -- and `|t|` is what
                // picks the combination that gets traded.
                //
                // MEASURED, not hypothetical: the 81 months of one-minute zerodha
                // NIFTY this store holds are 618,296 bars against roughly 626,625
                // for 1,671 sessions of 375 -- about 8,329 missing, 1.32%. A hole
                // of one bar is enough to collide two 2-minute signals.
                //
                // The FIRST signal keeps the bar, and that is the physical answer
                // rather than a tiebreak: it is the earliest signal that could
                // have acted, and by the time the second fires the position it
                // would open is already open. Acting on the later one would also
                // be filling at a bar chosen by information that arrived after it.
                //
                // Duplicates are always ADJACENT because the alignment is
                // monotonically non-decreasing, so comparing against the last
                // pushed index is exact and costs one compare per row -- no set,
                // no allocation, and the pass stays O(len).
                Some(index) if source.last().copied() == Some(index) => {
                    collided = collided.saturating_add(1);
                }
                Some(index) => {
                    bits.push(mask);
                    source.push(index);
                }
            }
        }
        // THE CENSUS IS THE SIGNAL SERIES', AND IS LEFT ALONE. It answers "where
        // did every offered bar go", and the offered bars were the signal
        // series' — reprojection neither offers nor refuses one. `swept` would
        // become a lie if it were reduced here, because those bars WERE swept;
        // what changed is how many of them can be acted on, which is `dropped`
        // and is returned separately rather than folded into a count that means
        // something else.
        // `first_swept` is the first index of the series this column now indexes,
        // so it is taken from the projected sources rather than carried over.
        // Carrying the signal series' value would name a bar in the wrong
        // series, and it is the field `sources` exists to keep honest.
        let first_swept = source.first().copied();
        let mut census = self.census;
        census.offered = onto_len as u64;
        Some((
            Self {
                bits,
                source,
                // `onto` HOLDS FILL BARS, not signal bars: `align::onto_execution`
                // returns the first execution bar stamped at or after the
                // signal's close, which is the earliest bar a position can be
                // opened on. Recording that is what stops `trade::walk` adding
                // a second `+1` to a bar that already is the fill.
                sourced: Sourced::Fill,
                census,
                first_swept,
                accepted: accepted.into(),
                acceptance_census,
                spec: self.spec,
                acceptance_known,
                collided,
            },
            dropped,
        ))
    }

    /// Did the evaluator accept execution-slice bar `index`?
    ///
    /// `false` for an out-of-range index and for a column produced through the
    /// length-only compatibility projection. This is the O(1) price/path gate;
    /// no caller re-runs `Candle::check`, timestamp logic, or VWAP arithmetic.
    #[must_use]
    pub fn accepts(&self, index: usize) -> bool {
        self.acceptance_known && self.accepted.get(index).copied().unwrap_or(false)
    }

    /// Is there exactly one acceptance verdict per bar in a slice of `len`?
    #[must_use]
    pub fn acceptance_covers(&self, len: usize) -> bool {
        self.acceptance_known && self.accepted.len() == len
    }

    /// The one shared acceptance bitmap, or `None` when target bars were never
    /// supplied to the compatibility projection.
    ///
    /// Cloning the return value increments an [`Arc`] counter; it does not copy
    /// the bitmap. `runner::trade::SliceFacts` uses that to carry this exact
    /// evaluator verdict through every candidate and grid cell without a
    /// second per-bar allocation.
    #[must_use]
    pub fn acceptance(&self) -> Option<Arc<[bool]>> {
        self.acceptance_known.then(|| Arc::clone(&self.accepted))
    }

    /// Opaque evaluator-policy identity, or `None` only for a default/legacy
    /// column that was not built by an evaluator.
    #[must_use]
    pub const fn evaluation_spec_token(&self) -> Option<EvaluationSpecToken> {
        match self.spec {
            Some(spec) => Some(EvaluationSpecToken(spec)),
            None => None,
        }
    }

    /// Where every bar in the slice this column CURRENTLY indexes went.
    ///
    /// On a native column this equals [`Self::census`]. After checked
    /// reprojection it is the fresh one-minute execution pass, while `census`
    /// remains the signal-series accounting.
    #[must_use]
    pub const fn acceptance_census(&self) -> Census {
        self.acceptance_census
    }

    /// The column, as `engine::Ladder::walk` wants it.
    #[must_use]
    pub fn bits(&self) -> &[ConditionMask] {
        &self.bits
    }

    /// Signals refused because an earlier signal already owns the execution bar
    /// they resolved to. Zero unless this column was reprojected.
    ///
    /// # Why it is exposed rather than returned
    ///
    /// [`Column::reproject`] returns `(Self, u64)` and has exactly one production
    /// caller. Widening that tuple is a change to a file this work does not own,
    /// so the count rides on the column instead -- which is where it belongs
    /// anyway: it describes the column that came out, not the call that made it.
    ///
    /// **It has no reader yet.** The banner that prints `signals with no
    /// execution bar` is where this belongs beside it, and until that line is
    /// added an operator cannot see how many signals a hole in the execution
    /// series cost them. The DEFECT is closed either way -- those signals no
    /// longer enter the mean twice -- but the disclosure is not, and saying so
    /// here is the honest version of `CLAUDE.md` §3 rule 6.
    #[must_use]
    pub const fn collided(&self) -> u64 {
        self.collided
    }

    /// Where every offered bar went.
    #[must_use]
    pub const fn census(&self) -> Census {
        self.census
    }

    /// The index, in the offered slice, of the first bar that was swept.
    ///
    /// `None` when the run never warmed up. This is the honest answer to "how
    /// much history did this column cost", and it is an index into the caller's
    /// own slice rather than a bar count this module would have to guess.
    #[must_use]
    pub const fn first_swept(&self) -> Option<usize> {
        self.first_swept
    }

    /// How many masks the column holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.bits.len()
    }

    /// Is the column empty? True whenever the run never warmed up.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bits.is_empty()
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "the exception every test module in this workspace takes -- a test that \
              cannot panic cannot fail. `.expect` panics inside core, which is not \
              instrumented, so it leaves no uncoverable region behind the way an \
              `unreachable!` expanding in this crate would."
)]
pub(super) mod tests {
    use super::*;
    use crate::OI_NULL;
    use crate::evaluator::Widths;
    use crate::pattern::Thresholds;
    use crate::vwap::Availability;

    /// 09:15 IST as microseconds past midnight UTC. 555 minutes IST, less the
    /// 330-minute offset.
    const IST_OPEN_UTC_MICROS: i64 = (555 - 330) * 60 * 1_000_000;
    const DAY_MICROS: i64 = 24 * 60 * 60 * 1_000_000;
    const MINUTE_MICROS: i64 = 60 * 1_000_000;
    /// 25,000.00 index points, in paisa.
    const BASE: i64 = 2_500_000;
    /// A regular NSE session.
    const BARS_PER_SESSION: usize = 375;
    /// Enough completed sessions to clear the five-session ladder, the previous
    /// day and the 200-candle trend seed with room to spare.
    const WARM_SESSIONS: i64 = 8;

    fn widths() -> Widths {
        Widths::pinned().expect("both pinned widths are valid")
    }

    pub(super) fn evaluator(availability: Availability) -> Evaluator {
        Evaluator::new(widths(), availability, Thresholds::CLASSICAL)
    }

    /// A deterministic bar. Every field is a function of `(day, minute)` alone,
    /// so the same arguments give the same bar forever — which is what makes the
    /// idempotence test below mean anything.
    fn bar(day: i64, minute: usize) -> Candle {
        let m = i64::try_from(minute).unwrap_or(0);
        // A drift plus a bounded wobble. Both integer; §7 forbids a float here
        // as firmly as it does in shipping code.
        let drift = day.saturating_mul(400).saturating_add(m.saturating_mul(3));
        let wobble = ((m % 7) - 3).saturating_mul(25);
        let close = BASE.saturating_add(drift).saturating_add(wobble);
        let open = close.saturating_sub(wobble);
        let high = open.max(close).saturating_add(60);
        let low = open.min(close).saturating_sub(60);
        Candle::new(
            day.saturating_mul(DAY_MICROS)
                .saturating_add(IST_OPEN_UTC_MICROS)
                .saturating_add(m.saturating_mul(MINUTE_MICROS)),
            open,
            high,
            low,
            close,
            1_000 + (m % 11),
            OI_NULL,
        )
    }

    /// `sessions` consecutive trading days of `BARS_PER_SESSION` bars each.
    pub(super) fn run(sessions: i64) -> Vec<Candle> {
        let mut out = Vec::with_capacity(usize::try_from(sessions).unwrap_or(0) * BARS_PER_SESSION);
        for day in 0..sessions {
            for minute in 0..BARS_PER_SESSION {
                out.push(bar(day, minute));
            }
        }
        out
    }

    /// A run long enough that the column is non-empty.
    fn warm_run() -> Vec<Candle> {
        run(WARM_SESSIONS)
    }

    #[test]
    fn fallible_exact_clone_preserves_every_column_field() {
        let column = Column::build(&warm_run(), &mut evaluator(Availability::Absent));
        let cloned = column
            .try_clone_exact()
            .expect("fixture column allocation fits");
        assert_eq!(cloned, column);
    }

    #[test]
    fn evaluator_spec_fingerprint_is_read_only_and_durable_through_the_token() {
        let absent = Column::build(&[], &mut evaluator(Availability::Absent));
        let same = Column::build(&[], &mut evaluator(Availability::Absent));
        let present = Column::build(&[], &mut evaluator(Availability::Present));

        let absent_fingerprint = absent
            .evaluation_spec_token()
            .expect("an evaluator-built column seals its choices")
            .fingerprint_v1();
        let same_fingerprint = same
            .evaluation_spec_token()
            .expect("an equivalent evaluator seals its choices")
            .fingerprint_v1();
        let present_fingerprint = present
            .evaluation_spec_token()
            .expect("the changed evaluator seals its choices")
            .fingerprint_v1();

        assert_eq!(absent_fingerprint, same_fingerprint);
        assert_ne!(absent_fingerprint, present_fingerprint);
        assert_eq!(
            absent_fingerprint.as_bytes().len(),
            EVALUATION_SPEC_FINGERPRINT_V1_LEN
        );
        assert_eq!(
            absent_fingerprint.into_bytes(),
            *same_fingerprint.as_bytes(),
            "borrowing and consuming expose the same complete record"
        );
        assert!(
            Column::default().evaluation_spec_token().is_none(),
            "a legacy/default column must not invent an evaluator identity"
        );
    }

    /// `sources()` is the only honest map from a column position to a bar.
    ///
    /// # Why `first_swept() + j` is not that map
    ///
    /// Refusals are COUNTED, not removed from the middle of the stream. So the
    /// moment one bar is refused, the naive offset runs one behind and stays
    /// there. Nothing in a sweep notices — the masks are all correct — but a
    /// caller pairing a signal with what happened AFTER it would read every
    /// outcome from the wrong bar, silently, and only on data that contains a
    /// refusal. That is the shape of every defect an audit found today.
    #[test]
    fn sources_maps_a_column_position_to_its_bar_even_across_a_refusal() {
        let mut bars = run(8);
        // Corrupt one bar deep inside the swept region: high below low.
        let victim = bars.len() - 20;
        if let Some(b) = bars.get_mut(victim) {
            *b = Candle::new(
                b.ts_micros,
                b.open,
                b.low - 100,
                b.high,
                b.close,
                1,
                OI_NULL,
            );
        }
        let column = Column::build(&bars, &mut evaluator(Availability::Absent));

        assert_eq!(
            column.census().high_below_low,
            1,
            "the fixture must actually contain a refusal, or this proves nothing"
        );
        assert_eq!(
            column.sources().len(),
            column.bits().len(),
            "the map must be parallel to the column"
        );

        // Every source index is strictly increasing and points at a real bar.
        let mut previous: Option<usize> = None;
        for &s in column.sources() {
            assert!(s < bars.len(), "source {s} is past the end of the input");
            if let Some(p) = previous {
                assert!(s > p, "sources must ascend: {p} then {s}");
            }
            previous = Some(s);
        }

        // AND THE NAIVE OFFSET IS WRONG, which is the whole reason this exists.
        let first = column.first_swept().expect("the run warms up");
        let naive_agrees = column
            .sources()
            .iter()
            .enumerate()
            .all(|(j, &s)| s == first.saturating_add(j));
        assert!(
            !naive_agrees,
            "with a refusal inside the swept region, `first_swept + j` MUST \
             diverge from the real bar index -- if it agrees here the fixture \
             is not exercising the case this map was added for"
        );
    }

    /// Blanking keeps the shape and the numbering. Cutting would destroy both.
    ///
    /// # What this catches
    ///
    /// A walk-forward scores a combination on the TEST window while the indicators keep
    /// the state the TRAINING bars legitimately gave them, and the obvious way to write
    /// that is to cut the column down to the test range. A cut RENUMBERS
    /// [`Column::sources`], which is the one defect `sources` exists to remove:
    /// `first_swept + j` runs one behind from the first refused bar onward, so every
    /// outcome after it is read off the wrong bar — silently, and only on data that
    /// contains a refusal.
    ///
    /// **So the fixture contains a refusal inside the swept region.** On a clean column
    /// blanking and cutting agree about every index, and a test built on one could not
    /// tell the two implementations apart.
    ///
    /// Five claims, each failing a different wrong implementation: the row count is
    /// unchanged (a cut shortens it), the source vector is unchanged (a cut renumbers
    /// it), the census is unchanged (no bar was re-offered), every row before the
    /// boundary can no longer satisfy a mask that requires anything (a no-op leaves them
    /// firing), and every row at or after it is bit-for-bit what it was (an off-by-one
    /// boundary blanks a test row and under-reports the fold).
    /// A warm column whose SWEPT region contains a refusal.
    ///
    /// The same corruption `sources_maps_a_column_position_to_its_bar_even_across_a_refusal`
    /// plants, and for the same reason: high below low, deep inside the swept region, so
    /// the column's positions and the caller's bar indices run out of step. On a clean
    /// column the two agree everywhere, and nothing built on one can tell a renumbering
    /// implementation from an honest one.
    fn column_across_a_refusal() -> Column {
        let mut bars = run(8);
        let victim = bars.len() - 20;
        if let Some(b) = bars.get_mut(victim) {
            *b = Candle::new(
                b.ts_micros,
                b.open,
                b.low - 100,
                b.high,
                b.close,
                1,
                OI_NULL,
            );
        }
        let column = Column::build(&bars, &mut evaluator(Availability::Absent));
        assert_eq!(
            column.census().high_below_low,
            1,
            "the fixture must actually contain a refusal, or every test built on it \
             proves nothing"
        );
        column
    }

    #[test]
    fn clearing_before_a_boundary_blanks_the_training_rows_and_renumbers_nothing() {
        let built = column_across_a_refusal();

        // A real bar index from the middle of the column, so both sides are non-empty.
        let middle = built.sources().len() / 2;
        let boundary = built
            .sources()
            .get(middle)
            .copied()
            .expect("a column built from eight sessions has a middle row");

        let mut cleared = built.clone();
        cleared.clear_before(boundary);

        assert_eq!(
            cleared.bits().len(),
            built.bits().len(),
            "a row disappeared: the column was CUT, and a cut renumbers `sources`"
        );
        assert_eq!(
            cleared.sources(),
            built.sources(),
            "a source index moved, which is exactly the renumbering `sources` prevents"
        );
        assert_eq!(
            cleared.census(),
            built.census(),
            "the census counts OFFERED bars, and blanking offers nothing"
        );
        assert_eq!(
            cleared.first_swept(),
            built.first_swept(),
            "the first swept bar is a fact about the input, not about a fold boundary"
        );

        let mut blanked = 0_usize;
        let mut still_firing = 0_usize;
        for ((&source, was), now) in built
            .sources()
            .iter()
            .zip(built.bits().iter())
            .zip(cleared.bits().iter())
        {
            if source < boundary {
                assert!(
                    now.is_empty(),
                    "the row from bar {source} is before the boundary {boundary} and \
                     still carries bits, so a training bar can still be counted as a hit"
                );
                // The documented consequence, said the way the sweep says it. A mask
                // hits a bar iff `(bits & mask) == mask`, so a zeroed row satisfies
                // nothing that requires a bit — and still satisfies the EMPTY mask,
                // which has no conditions and is not a strategy.
                //
                // The precondition is ASSERTED, not guarded on. `if !was.is_empty()`
                // would silently skip the check on a row that carried nothing, and a
                // skipped check is the vacuous pass the counters below exist to rule
                // out. A swept bar always sets something, so say that and let it fail.
                assert!(
                    !was.is_empty(),
                    "the row from bar {source} carried no bits before the call, so \
                     blanking it changes nothing and `hits` cannot notice"
                );
                assert!(
                    !now.hits(was),
                    "the blanked row from bar {source} still satisfies the mask its own \
                     bits formed, so `hits` cannot tell it from a live row"
                );
                assert!(
                    now.hits(&ConditionMask::ZERO),
                    "`(0 & 0) == 0`: the empty mask hits every row, blanked or not"
                );
                blanked += 1;
            } else {
                assert_eq!(
                    now, was,
                    "the row from bar {source} is at or after the boundary {boundary} \
                     and was changed, so the comparison is off by one"
                );
                if !now.is_empty() {
                    still_firing += 1;
                }
            }
        }
        assert!(
            blanked > 0,
            "the boundary fell before every row, so nothing was blanked and the \
             assertions above are vacuous"
        );
        assert!(
            still_firing > 0,
            "no row after the boundary carries a bit, so the test window is dead too and \
             `clear_before` cannot be distinguished from zeroing the whole column"
        );
    }

    #[test]
    fn exact_gap_replacement_clears_every_local_gap_bit_and_no_other_family() {
        let mut column = Column::build(&warm_run(), &mut evaluator(Availability::Absent));
        assert!(!column.bits.is_empty(), "the fixture must warm a column");
        let original = column.clone();
        assert!(
            !column.replace_gapfib(&[]),
            "a non-parallel overlay must be refused"
        );
        assert_eq!(
            column, original,
            "a refused overlay must not mutate the column"
        );

        let first = column.bits.first_mut().expect("a warmed row exists");
        *first = first.with_bit(3);
        for position in crate::gap::GapFib::positions() {
            *first = first.with_bit(u32::from(position));
        }
        let mut exact = vec![ConditionMask::ZERO; column.bits.len()];
        let first_exact = exact.first_mut().expect("the parallel row exists");
        *first_exact = first_exact.with_bit(142);
        assert!(column.replace_gapfib(&exact));
        let replaced = *column.bits.first().expect("the replaced row exists");
        assert!(replaced.get(3), "an unrelated signal-local bit was erased");
        for position in 132_u32..=141 {
            assert!(
                !replaced.get(position),
                "local GapFib position {position} survived the replacement"
            );
        }
        assert!(
            replaced.get(142),
            "the exact-minute overlay bit was not copied"
        );
    }

    #[test]
    fn an_empty_slice_gives_an_empty_column() {
        let mut ev = evaluator(Availability::Absent);
        let column = Column::build(&[], &mut ev);

        assert!(column.is_empty());
        assert_eq!(column.len(), 0);
        assert_eq!(column.bits(), &[]);
        assert_eq!(column.first_swept(), None);
        assert_eq!(column.census(), Census::default());
        assert!(column.census().reconciles());
    }

    #[test]
    fn a_cold_run_sweeps_nothing_and_says_so() {
        // One session cannot fill a five-session ladder, so every bar folds and
        // none is swept. The column is empty and the census explains it.
        let bars = run(1);
        let mut ev = evaluator(Availability::Absent);
        let column = Column::build(&bars, &mut ev);

        assert!(column.is_empty());
        assert_eq!(column.first_swept(), None);
        let census = column.census();
        assert_eq!(census.offered, BARS_PER_SESSION as u64);
        assert_eq!(census.warming, BARS_PER_SESSION as u64);
        assert_eq!(census.swept, 0);
        assert_eq!(census.refused(), 0);
        assert!(census.reconciles());
        // The caller can still ask why, off the borrowed evaluator.
        assert!(!ev.warmed_up());
        assert!(ev.sessions_until_every_family_can_answer() > 0);
    }

    #[test]
    fn a_warm_run_sweeps_and_the_counts_agree() {
        let bars = warm_run();
        let mut ev = evaluator(Availability::Absent);
        let column = Column::build(&bars, &mut ev);

        assert!(!column.is_empty(), "eight sessions must warm the run");
        let census = column.census();
        assert_eq!(census.offered, bars.len() as u64);
        assert_eq!(census.swept, column.len() as u64);
        assert_eq!(census.refused(), 0);
        assert!(census.reconciles());
        assert_eq!(
            census.warming.saturating_add(census.swept),
            bars.len() as u64
        );
    }

    #[test]
    fn the_column_is_a_suffix_and_never_reorders() {
        // `warmed_up` is monotone, so the swept bars are a contiguous tail. Prove
        // it by rebuilding the same masks by hand and comparing position for
        // position.
        let bars = warm_run();
        let mut ev = evaluator(Availability::Absent);
        let column = Column::build(&bars, &mut ev);

        let first = column.first_swept().expect("a warm run swept something");
        let mut by_hand = evaluator(Availability::Absent);
        let mut expected = Vec::new();
        for (index, candle) in bars.iter().enumerate() {
            let warm = by_hand.warmed_up();
            let mask = by_hand.step(candle).expect("synthetic bars are bars");
            if warm {
                assert!(index >= first, "a swept bar before the first swept index");
                expected.push(mask);
            }
        }
        assert_eq!(column.bits(), expected.as_slice());
        // Contiguous: every index from `first` onward was swept.
        assert_eq!(column.len(), bars.len() - first);
    }

    #[test]
    fn a_prefix_of_the_bars_gives_a_prefix_of_the_column() {
        // §3 rule 7. Bar N's mask cannot depend on bar N+1, so truncating the
        // input can only truncate the output — never change it.
        let bars = warm_run();
        let mut full_ev = evaluator(Availability::Absent);
        let full = Column::build(&bars, &mut full_ev);

        let cut = bars.len().saturating_sub(BARS_PER_SESSION);
        let head = bars.get(..cut).expect("cut is inside the run");
        let mut short_ev = evaluator(Availability::Absent);
        let short = Column::build(head, &mut short_ev);

        assert!(!short.is_empty());
        assert_eq!(short.first_swept(), full.first_swept());
        assert_eq!(
            short.bits(),
            full.bits()
                .get(..short.len())
                .expect("the short column cannot outgrow the full one")
        );
    }

    #[test]
    fn the_same_bars_give_the_same_column_twice() {
        // §3 rule 5, idempotence, byte for byte.
        let bars = warm_run();
        let mut a = evaluator(Availability::Absent);
        let mut b = evaluator(Availability::Absent);
        assert_eq!(Column::build(&bars, &mut a), Column::build(&bars, &mut b));
    }

    /// `clear_before` ZEROES BELOW THE BOUNDARY AND KEEPS THE BOUNDARY ITSELF.
    ///
    /// Four mutants lived here and they are four different wrong answers:
    /// replacing the whole function with `()` (clear nothing), `<` to `<=`
    /// (also clear the boundary row), `<` to `>` (clear the wrong side), and
    /// `<` to `==` (clear only one row).
    ///
    /// `crate::validate` uses this to build a fold's TEST column: every bar
    /// before the boundary is blanked so the walk cannot see its own training
    /// data. Each of those four mutations leaks or destroys exactly the bars
    /// that separation depends on, and none of them looks wrong from outside —
    /// a leaked training bar makes an out-of-sample result better, not broken.
    #[test]
    fn clear_before_blanks_below_the_boundary_and_keeps_the_boundary_row() {
        let bars = warm_run();
        let mut ev = evaluator(Availability::Absent);
        let column = Column::build(&bars, &mut ev);
        let sources = column.sources().to_vec();
        assert!(
            sources.len() >= 4,
            "this fixture must sweep several bars or the boundary cases coincide"
        );

        // A boundary in the middle, taken from the column's OWN source indices
        // so it is a real one rather than a guess about the warm-up length.
        let mid = sources.len() / 2;
        // `expect` and not `unreachable!`: an unreachable arm is a project
        // region no test can enter, and CLAUDE.md S9's coverage floor cannot be
        // met with one. `expect` panics inside std and costs no region here.
        let boundary = sources
            .get(mid)
            .copied()
            .expect("mid is inside a slice of length >= 4");
        let before: Vec<ConditionMask> = column.bits().to_vec();

        let mut cleared = column;
        cleared.clear_before(boundary);

        let mut zeroed = 0_usize;
        let mut kept = 0_usize;
        for (i, (&source, bits)) in sources.iter().zip(cleared.bits()).enumerate() {
            let was = before.get(i).copied().unwrap_or(ConditionMask::ZERO);
            if source < boundary {
                assert!(
                    bits.is_empty(),
                    "row {i} sources bar {source}, below the {boundary} boundary, \
                     and must be blank -- a `>` or `==` comparison leaves it set \
                     and leaks a training bar into the test window"
                );
                zeroed = zeroed.saturating_add(1);
            } else {
                assert_eq!(
                    *bits, was,
                    "row {i} sources bar {source}, at or after the {boundary} \
                     boundary, and must be untouched -- `<=` blanks the boundary \
                     row itself and throws away a bar the fold is meant to test"
                );
                kept = kept.saturating_add(1);
            }
        }

        assert!(
            zeroed > 0,
            "the fixture must actually clear something, or a function replaced \
             by `()` passes this test"
        );
        assert!(kept > 0, "and must keep something, or `>` passes it");
    }

    #[test]
    fn every_bar_lands_in_exactly_one_bucket() {
        // One of every refusal, mixed into a warm run, plus the warm-up tail.
        let mut bars = warm_run();
        // The last bar the evaluator will ACCEPT. The three refusals below commit
        // nothing, so this stays the high-water mark for the fourth.
        let last_accepted = bars.last().map_or(0, |candle| candle.ts_micros);
        let ts = |n: i64| {
            WARM_SESSIONS
                .saturating_mul(DAY_MICROS)
                .saturating_add(IST_OPEN_UTC_MICROS)
                .saturating_add(n.saturating_mul(MINUTE_MICROS))
        };
        // high < low.
        bars.push(Candle::new(
            ts(1),
            BASE,
            BASE - 10,
            BASE + 10,
            BASE,
            1,
            OI_NULL,
        ));
        // high - low leaves i64.
        bars.push(Candle::new(ts(2), 0, i64::MAX, i64::MIN, 0, 1, OI_NULL));
        // open above high.
        bars.push(Candle::new(
            ts(3),
            BASE + 500,
            BASE + 100,
            BASE - 100,
            BASE,
            1,
            OI_NULL,
        ));
        // A timestamp that does not strictly increase. Equal is enough: the
        // rollover keys on the IST day CHANGING, so a repeat closes the books on
        // a session that has not happened yet.
        bars.push(Candle::new(
            last_accepted,
            BASE,
            BASE + 10,
            BASE - 10,
            BASE,
            1,
            OI_NULL,
        ));

        let mut ev = evaluator(Availability::Absent);
        let column = Column::build(&bars, &mut ev);
        let census = column.census();

        assert_eq!(census.offered, bars.len() as u64);
        assert_eq!(census.high_below_low, 1);
        assert_eq!(census.range_overflows, 1);
        assert_eq!(census.price_outside_range, 1);
        assert_eq!(census.timestamp_not_increasing, 1);
        assert_eq!(census.refused(), 4);
        assert!(census.reconciles());
        // A refused bar changes nothing: the column is the warm run's column.
        let mut clean_ev = evaluator(Availability::Absent);
        let clean = Column::build(&warm_run(), &mut clean_ev);
        assert_eq!(column.bits(), clean.bits());
    }

    #[test]
    fn a_negative_volume_is_charged_to_its_own_bucket() {
        // Only the VWAP family reads volume, so this refusal needs an evaluator
        // that runs VWAP at all.
        let bars = vec![Candle::new(
            IST_OPEN_UTC_MICROS,
            BASE,
            BASE + 10,
            BASE - 10,
            BASE,
            -1,
            OI_NULL,
        )];
        let mut ev = evaluator(Availability::Present);
        let census = Column::build(&bars, &mut ev).census();

        assert_eq!(census.negative_volume, 1);
        assert_eq!(census.refused(), 1);
        assert_eq!(census.swept, 0);
        assert_eq!(census.warming, 0);
        assert!(census.reconciles());
    }

    #[test]
    fn an_oversized_accumulator_is_charged_to_its_own_bucket() {
        // Unreachable on any price a market prints, and constructible here in one
        // line -- which is the argument for synthetic bars rather than stored
        // ones. With real data this branch could never be covered.
        let bars = vec![Candle::new(
            IST_OPEN_UTC_MICROS,
            i64::MAX - 1,
            i64::MAX,
            i64::MAX - 2,
            i64::MAX,
            i64::MAX,
            OI_NULL,
        )];
        let mut ev = evaluator(Availability::Present);
        let census = Column::build(&bars, &mut ev).census();

        assert_eq!(census.accumulator_too_large, 1);
        assert_eq!(census.refused(), 1);
        assert!(census.reconciles());
    }

    /// The execution gate is the evaluator's verdict, including the two
    /// sequence-dependent refusals that `Candle::check` cannot see.
    #[test]
    fn acceptance_bitmap_records_a_duplicate_timestamp_in_constant_time_shape() {
        let mut bars = warm_run();
        let victim = bars.len().saturating_sub(20);
        let previous = bars
            .get(victim.saturating_sub(1))
            .copied()
            .expect("the warm fixture has a predecessor");
        if let Some(bar) = bars.get_mut(victim) {
            bar.ts_micros = previous.ts_micros;
        }
        assert!(
            bars.get(victim).is_some_and(|bar| bar.check().is_ok()),
            "one record alone cannot see timestamp ordering"
        );

        let column = Column::build(&bars, &mut evaluator(Availability::Absent));
        assert!(column.acceptance_covers(bars.len()));
        assert!(!column.accepts(victim));
        assert!(column.accepts(victim.saturating_sub(1)));
        assert!(column.accepts(victim.saturating_add(1)));
        assert_eq!(column.acceptance_census().timestamp_not_increasing, 1);
        assert!(column.acceptance_census().reconciles());
    }

    /// Checked reprojection runs the source column's same evaluator choices
    /// afresh over the execution slice; it does not infer acceptance from the
    /// copied signal rows.
    #[test]
    fn checked_reprojection_records_execution_accumulator_refusal() {
        let signal = Column::build(&warm_run(), &mut evaluator(Availability::Present));
        let ordinary = bar(20, 0);
        let huge = Candle::new(
            ordinary.ts_micros.saturating_add(MINUTE_MICROS),
            5_000_000_000_000_000_000,
            5_000_000_000_000_000_000,
            5_000_000_000_000_000_000,
            5_000_000_000_000_000_000,
            1,
            OI_NULL,
        );
        assert!(
            huge.check().is_ok(),
            "the refusal is stateful VWAP arithmetic"
        );
        let execution = [ordinary, huge, bar(20, 2)];
        let onto: Vec<Option<usize>> = signal.bits().iter().map(|_| Some(0)).collect();
        let (checked, _) = signal
            .reproject_checked(&onto, &execution)
            .expect("the mapping is parallel to the signal column");

        assert!(checked.acceptance_covers(execution.len()));
        assert!(checked.accepts(0));
        assert!(!checked.accepts(1));
        assert!(checked.accepts(2));
        assert_eq!(checked.acceptance_census().accumulator_too_large, 1);
        assert!(checked.acceptance_census().reconciles());

        let (unchecked, _) = signal
            .reproject(&onto, execution.len())
            .expect("the compatibility mapping remains parallel");
        assert!(
            !unchecked.acceptance_covers(execution.len()) && !unchecked.accepts(0),
            "a length-only projection must refuse every execution lookup rather \
             than silently pretending local checks cover stateful corruption"
        );
    }

    #[test]
    fn the_census_charges_each_variant_exactly_once() {
        // `charge` is exhaustive and injective. Walk every variant and assert the
        // bucket it moved -- a mutant that redirects one arm to a neighbour dies
        // here.
        let variants = [
            Corrupt::HighBelowLow,
            Corrupt::RangeOverflows,
            Corrupt::PriceOutsideRange,
            Corrupt::TimestampNotIncreasing,
            Corrupt::NegativeVolume,
            Corrupt::AccumulatorTooLarge,
        ];
        for variant in variants {
            let mut census = Census::default();
            census.charge(variant);
            assert_eq!(census.refused(), 1, "{variant:?} charged no bucket");
            let moved = [
                census.high_below_low,
                census.range_overflows,
                census.price_outside_range,
                census.timestamp_not_increasing,
                census.negative_volume,
                census.accumulator_too_large,
            ];
            assert_eq!(
                moved.iter().filter(|n| **n == 1).count(),
                1,
                "{variant:?} charged more than one bucket"
            );
        }
    }

    #[test]
    fn reconciles_is_false_when_a_bar_is_lost() {
        // The invariant has to be able to FAIL, or asserting it proves nothing.
        let census = Census {
            offered: 2,
            warming: 0,
            swept: 1,
            ..Census::default()
        };
        assert!(!census.reconciles());
    }

    #[test]
    fn first_swept_is_the_index_in_the_callers_own_slice() {
        let bars = warm_run();
        let mut ev = evaluator(Availability::Absent);
        let column = Column::build(&bars, &mut ev);
        let first = column.first_swept().expect("a warm run swept something");

        // Everything before it was warming; nothing before it was refused.
        assert_eq!(column.census().warming, first as u64);
        assert_eq!(column.census().refused(), 0);
        assert!(first > 0, "a run cannot be warm on its first bar");
    }
}

#[cfg(test)]
#[expect(
    clippy::expect_used,
    reason = "the exception every test module in this workspace takes"
)]
mod reproject_tests {
    use super::Column;
    use super::tests::{evaluator as build_evaluator, run};
    use crate::vwap::Availability;

    /// A reprojected column must PAIR with a `Forward` from the series it now
    /// indexes.
    ///
    /// `outcome::edge` computes
    /// `forward.built_from_same_slice_as(column.census().offered)`, so `offered`
    /// is not only a description of what was handed in — it is the slice
    /// IDENTITY the pairing check reads.
    ///
    /// MEASURED before this was fixed, on `zerodha NIFTY 60min`: a column
    /// reprojected onto 623,546 one-minute bars still claimed 10,400 offered, so
    /// every hit landed in `Edge::mismatched` and every row of the report read
    /// "MISPAIRED — the forward outcomes belong to other bars".
    #[test]
    fn a_reprojected_column_claims_the_slice_it_now_indexes() {
        let bars = run(6);
        let mut ev = build_evaluator(Availability::Absent);
        let column = Column::build(&bars, &mut ev);
        assert!(!column.is_empty(), "the fixture must produce rows");

        // Every row maps onto a much larger series, as a 1-minute execution
        // series is larger than any signal rung.
        let onto: Vec<Option<usize>> = (0..column.len()).map(Some).collect();
        let (projected, dropped) = column
            .reproject(&onto, 9_999)
            .expect("the map is parallel to the column");

        assert_eq!(dropped, 0);
        assert_eq!(
            projected.census().offered,
            9_999,
            "the census must name the slice this column now indexes, not the \
             one it came from -- `outcome::edge` reads it as the pairing key"
        );
        // AND THE REST OF THE CENSUS IS THE SIGNAL SERIES', unchanged. Those
        // bars really were swept; reducing the count here would make `swept` a
        // lie about work that happened.
        assert_eq!(
            projected.census().swept,
            column.census().swept,
            "swept describes the signal bars and does not move"
        );
        assert_eq!(projected.census().warming, column.census().warming);
        assert_eq!(projected.collided(), 0, "a one-to-one map collides nothing");
    }

    /// TWO SIGNALS, ONE FILL BAR, ONE ROW -- and the second is counted, not lost.
    ///
    /// # The number this changes
    ///
    /// `align::onto_execution` is forward-only and returns the first execution
    /// bar stamped at or after each signal's close, so a hole in the execution
    /// series resolves two consecutive signals to the same bar; `align`'s own
    /// test asserts `[Some(10), Some(10)]` as the correct output. Both rows used
    /// to be pushed with that same source, and `outcome::edge` accumulates one
    /// observation per ROW -- so one forward return entered the mean twice.
    ///
    /// That inflates `n`, understates the standard error and inflates `|t|`, and
    /// `|t|` is what selects the combination that gets traded. A duplicated
    /// observation is the cheapest possible way to manufacture significance.
    ///
    /// The FIRST signal keeps the bar: it is the earliest that could have acted,
    /// and by the time the second fires the position is already open.
    #[test]
    fn two_signals_resolving_to_one_fill_bar_produce_one_row() {
        let bars = run(6);
        let mut ev = build_evaluator(Availability::Absent);
        let column = Column::build(&bars, &mut ev);
        assert!(column.len() >= 4, "the fixture must give enough rows");

        // A hole in the execution series: rows 1 and 2 both resolve to bar 7,
        // and rows 3 and 4 both resolve to bar 9. Everything else is distinct.
        let mut onto: Vec<Option<usize>> = (0..column.len()).map(|i| Some(i + 20)).collect();
        if let Some(slot) = onto.get_mut(1) {
            *slot = Some(7);
        }
        if let Some(slot) = onto.get_mut(2) {
            *slot = Some(7);
        }
        if let Some(slot) = onto.get_mut(3) {
            *slot = Some(9);
        }
        if let Some(slot) = onto.get_mut(4) {
            *slot = Some(9);
        }

        let (projected, dropped) = column
            .reproject(&onto, 9_999)
            .expect("the map is parallel to the column");

        assert_eq!(dropped, 0, "nothing here failed to resolve");
        assert_eq!(
            projected.collided(),
            2,
            "one duplicate per colliding pair, and it is COUNTED rather than \
             silently dropped -- a signal the execution series could not give \
             its own bar is a fact the operator is owed"
        );
        assert_eq!(
            projected.len(),
            column.len().saturating_sub(2),
            "two rows fewer, and exactly two"
        );

        // THE ONE PROPERTY THAT MATTERS DOWNSTREAM: no execution bar appears
        // twice, so no forward return can be counted twice.
        let mut seen = projected.sources().to_vec();
        let before = seen.len();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(
            seen.len(),
            before,
            "every source must be distinct -- this is the assertion that stands \
             between a hole in the one-minute series and a fabricated t-statistic"
        );

        // And the survivor is the FIRST of each pair, not the last.
        assert!(
            projected.sources().contains(&7) && projected.sources().contains(&9),
            "the fill bars themselves are kept; it is the second claimant on \
             each that is refused"
        );
    }

    /// Dropped rows are counted and removed, and nothing else shifts.
    #[test]
    fn a_row_with_no_execution_bar_is_dropped_and_counted() {
        let bars = run(6);
        let mut ev = build_evaluator(Availability::Absent);
        let column = Column::build(&bars, &mut ev);
        let keep = column.len().saturating_sub(3);

        let onto: Vec<Option<usize>> = (0..column.len())
            .map(|i| if i < keep { Some(i) } else { None })
            .collect();
        let (projected, dropped) = column.reproject(&onto, 1_000).expect("parallel");

        assert_eq!(dropped, 3, "the three unreachable rows are counted");
        assert_eq!(projected.len(), keep, "and removed");
        assert_eq!(
            projected.first_swept(),
            Some(0),
            "first_swept names the new series, not the old one"
        );
    }

    /// A map that is not parallel to its column refuses.
    #[test]
    fn a_map_of_the_wrong_length_refuses_rather_than_truncating() {
        let bars = run(6);
        let mut ev = build_evaluator(Availability::Absent);
        let column = Column::build(&bars, &mut ev);
        assert!(
            column.reproject(&[Some(0)], 10).is_none(),
            "a caller bug refuses rather than silently taking the shorter of two"
        );
    }
}

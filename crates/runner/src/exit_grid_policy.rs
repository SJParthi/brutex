//! Versioned, data-resolved exit-grid policy for the two swept spot indices.
//!
//! This module separates three facts that were previously easy to blur:
//!
//! 1. a policy states every runtime choice;
//! 2. TRAINING one-minute OHLCV resolves that policy to exact distance rungs;
//! 3. the resolved rungs are replayed unchanged on later bars.
//!
//! No equity, future, option, reference index, BSE index, or unknown index can
//! reach resolution. Those instruments may be stored, but `AGENTS.md` section
//! 1 permits the sweep only for `NSE-NIFTY` and `NSE-BANKNIFTY`.
//!
//! The resolver creates no tick and no execution price. Long stops sample
//! `open - low` while long targets sample `high - open`; short swaps those two
//! axes. Trailing distances sample the full checked `high - low` range. Every
//! span is represented as an integer ratio to that same candle's printed open,
//! and percentile rungs are exact members of the relevant observed ratio set.
//! Entry and exit prices remain the job of [`crate::grid`], which brackets
//! fills with one-minute OHLC values.

use brutex_core::blake3::{Hasher, hash};
use brutex_core::instrument::{Exchange, InstrumentKey, Kind, Segment};
use brutex_core::vendor::Vendor;
use costs::fill::Direction as CostDirection;
use indicators::Candle;
use indicators::column::EvaluationSpecToken;
use vocab::ConditionMask;

use crate::excursion::{Ladder, Ladders, Ppm};
use crate::grid::{Cell, Chosen, Grid};
use crate::identity::{Direction, Run, RunId};
use crate::outcome::{FORCED_EXIT_MINUTE, Horizon};

/// The canonical version encoded into every V1 policy and resolution digest.
pub const EXIT_GRID_POLICY_VERSION_V1: u16 = 1;

/// One regular NSE minute in microseconds.
const ONE_MINUTE_MICROS: i64 = 60_000_000;

/// One civil day in microseconds.
const DAY_MICROS: i64 = 24 * 60 * ONE_MINUTE_MICROS;

/// First regular-session one-minute stamp, 09:15 IST.
#[cfg(test)]
const REGULAR_OPEN_IST_MINUTE: i64 = 9 * 60 + 15;

/// Identity of the fill/cost behavior implemented by the current grid engine.
///
/// It means printed one-minute OHLC bounds, pessimistic adverse printed
/// extremes, optimistic printed opens, level fills capped by opening gaps, and
/// no invented outside-bar tick. V1 resolution refuses any other ID because
/// merely hashing an arbitrary label would not prove which arithmetic produced
/// the cell money.
#[must_use]
pub fn printed_ohlcv_cost_model_id_v1() -> [u8; 32] {
    hash(b"brutex.runner.grid.printed-ohlcv-fill.v1.open-gap-before-retrace")
}

/// Canonical identity of every field in one [`InstrumentKey`].
///
/// This deliberately reuses the same private structural encoding carried by
/// resolved exit-grid identities.  Persistence callers therefore cannot drift
/// to a symbol-only alias that forgets exchange, segment, kind, expiry, strike
/// or option side.
#[must_use]
pub fn instrument_digest_v1(instrument: &InstrumentKey) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(b"brutex.runner.canonical-instrument.v1\0");
    put_instrument(&mut hasher, instrument);
    hasher.finalize()
}

/// Parts-per-million denominator used to encode an observed bar range.
const PPM_ONE: i128 = 1_000_000;

/// A rational nearest-rank percentile, with no floating-point boundary.
///
/// `numerator / denominator` is in `(0, 1]`. Resolution uses
/// `ceil(sample_count * numerator / denominator) - 1`, so the selected rung is
/// an exact member of the sorted TRAINING sample rather than an interpolated
/// value the market never supplied.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RationalPercentileV1 {
    numerator: u32,
    denominator: u32,
}

impl RationalPercentileV1 {
    /// Builds a non-zero percentile no greater than one.
    ///
    /// # Errors
    ///
    /// [`ExitGridErrorV1::InvalidPercentile`] when the denominator or
    /// numerator is zero, or the numerator exceeds the denominator.
    pub const fn new(numerator: u32, denominator: u32) -> Result<Self, ExitGridErrorV1> {
        if numerator == 0 || denominator == 0 || numerator > denominator {
            return Err(ExitGridErrorV1::InvalidPercentile {
                numerator,
                denominator,
            });
        }
        let divisor = gcd_u32(numerator, denominator);
        Ok(Self {
            numerator: numerator / divisor,
            denominator: denominator / divisor,
        })
    }

    /// The percentile numerator.
    #[must_use]
    pub const fn numerator(self) -> u32 {
        self.numerator
    }

    /// The percentile denominator.
    #[must_use]
    pub const fn denominator(self) -> u32 {
        self.denominator
    }
}

/// The execution-series resolution the policy expects.
///
/// The unsupported variant exists so a persisted/newer request is refused by
/// name rather than decoded as V1 one-minute data. It never resolves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExecutionResolutionV1 {
    /// Timestamp-aligned one-minute OHLCV candles.
    OneMinuteOhlcv,
    /// Any other duration, which V1 cannot execute or validate.
    UnsupportedSeconds(u32),
}

/// Attested exact-one-minute execution input for one instrument and feed.
///
/// The calendar digest identifies the caller's accepted-session policy; it is
/// not inferred from the timestamps. V1 verifies exact minute alignment and
/// strict timestamp order. Missing whole minutes remain gaps: they are never
/// interpolated, and
/// the exact execution fold later drops only a signal/hold path that needs one.
/// Accepted-session membership is an upstream calendar fact represented by
/// `calendar_digest`; this type binds that fact but does not pretend to verify
/// an exchange calendar from timestamps alone.
#[derive(Clone, Copy, Debug)]
pub struct ExecutionSeriesV1<'a> {
    instrument: &'a InstrumentKey,
    feed: &'a str,
    commit: &'a str,
    calendar_digest: [u8; 32],
    bars: &'a [Candle],
}

impl<'a> ExecutionSeriesV1<'a> {
    /// Constructs a source-attested execution series without defaulting any
    /// identity term.
    ///
    /// # Errors
    ///
    /// Empty feed/commit values or an all-zero calendar identity refuse.
    pub fn new(
        instrument: &'a InstrumentKey,
        feed: &'a str,
        commit: &'a str,
        calendar_digest: [u8; 32],
        bars: &'a [Candle],
    ) -> Result<Self, ExitGridErrorV1> {
        if feed.is_empty() {
            return Err(ExitGridErrorV1::MissingSeriesIdentity("feed"));
        }
        if commit.is_empty() {
            return Err(ExitGridErrorV1::MissingSeriesIdentity("commit"));
        }
        if all_zero(&calendar_digest) {
            return Err(ExitGridErrorV1::MissingSeriesIdentity("calendar"));
        }
        Ok(Self {
            instrument,
            feed,
            commit,
            calendar_digest,
            bars,
        })
    }

    /// Exact instrument whose bytes these are.
    #[must_use]
    pub const fn instrument(self) -> &'a InstrumentKey {
        self.instrument
    }

    /// Exact vendor/feed identity supplied by the stored loader.
    #[must_use]
    pub const fn feed(self) -> &'a str {
        self.feed
    }

    /// Clean source commit whose arithmetic is being executed.
    #[must_use]
    pub const fn commit(self) -> &'a str {
        self.commit
    }

    /// Identity of the accepted-session/calendar contract.
    #[must_use]
    pub const fn calendar_digest(self) -> [u8; 32] {
        self.calendar_digest
    }

    /// Exact ordered one-minute OHLCV bytes.
    #[must_use]
    pub const fn bars(self) -> &'a [Candle] {
        self.bars
    }
}

/// Canonical run identity sealed beside the execution bytes it names.
///
/// A raw [`RunId`] is not sufficient authority: any 32 bytes have that type,
/// and the callee cannot recover the nine identity terms from a digest to
/// verify them. This capability can only be built from a complete [`Run`] and
/// the exact execution slice. Evaluation/replay then compare its copied terms
/// with the attested series before recording its canonical ID.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExecutionRunV1 {
    run_id: RunId,
    instrument: InstrumentKey,
    mask: ConditionMask,
    direction: Direction,
    feed_digest: [u8; 32],
    commit_digest: [u8; 32],
    execution_digest: [u8; 32],
}

impl ExecutionRunV1 {
    /// Digest of the exact ordered one-minute execution candles bound into
    /// this run authority.
    ///
    /// Durable pre-admission adapters use this O(1) accessor to reconcile the
    /// exact execution stream they persist; it does not expose or rehash the
    /// candles.
    ///
    /// **UNVERIFIED as a measured bound.** No bench in this workspace
    /// times this, so the shape above is read from the source rather
    /// than measured. `CLAUDE.md` §3 rule 6.
    #[must_use]
    pub const fn execution_digest(&self) -> [u8; 32] {
        self.execution_digest
    }

    /// Seals all canonical run terms after recomputing the data term from the
    /// exact signal bytes and optional separate execution bytes. `None` means
    /// the signal series is also the one-minute execution series; `Some`
    /// applies the same composite digest as [`crate::identity::Run`].
    ///
    /// # Errors
    ///
    /// Empty timeframe/feed/commit terms, undirected trade runs, and a supplied
    /// `Run::data_digest` that does not match the exact bytes refuse.
    pub fn new(
        run: &Run<'_>,
        signal_bars: &[Candle],
        execution_bars: Option<&[Candle]>,
    ) -> Result<Self, ExitGridErrorV1> {
        let expected_data_digest =
            crate::identity::data_digest_with_execution(signal_bars, execution_bars);
        let exact_execution = execution_bars.unwrap_or(signal_bars);
        Self::seal(run, exact_execution, expected_data_digest)
    }

    /// Seals a stored run whose result depends on signal, exact one-minute,
    /// and previous-day reference streams plus their complete binding policy.
    ///
    /// This is deliberately a separate constructor rather than a flag on
    /// [`Self::new`]. A caller cannot accidentally validate the same stored run
    /// against the weaker two-stream digest and thereby drop daily-reference,
    /// eligibility, calendar, overlay-policy, or integrity terms.
    ///
    /// `reference_minute_context` is the complete minute stream the causal
    /// daily/GapFib join actually read, including prior-session/month warm-up;
    /// it is therefore the stream bound into the canonical three-stream data
    /// digest. `evaluated_execution_1m` is the exact requested span priced by
    /// the grid and alone becomes this capability's execution digest. It must
    /// be an exact contiguous byte-for-byte subslice of the context, so the
    /// separation cannot authorize unrelated execution bytes.
    ///
    /// # Errors
    ///
    /// In addition to the ordinary run-identity refusals, malformed daily
    /// eligibility is surfaced as
    /// [`ExitGridErrorV1::DailyReferenceIdentityRefused`], and an evaluated
    /// slice absent from the bound context is
    /// [`ExitGridErrorV1::EvaluatedExecutionOutsideReferenceContext`].
    pub fn new_with_daily_reference(
        run: &Run<'_>,
        signal_bars: &[Candle],
        reference_minute_context: &[Candle],
        evaluated_execution_1m: &[Candle],
        reference: crate::identity::DailyReferenceBinding<'_>,
    ) -> Result<Self, ExitGridErrorV1> {
        let expected_data_digest = crate::identity::data_digest_with_daily_reference(
            signal_bars,
            reference_minute_context,
            reference,
        )
        .map_err(ExitGridErrorV1::DailyReferenceIdentityRefused)?;
        require_exact_execution_subslice(reference_minute_context, evaluated_execution_1m)?;
        Self::seal(run, evaluated_execution_1m, expected_data_digest)
    }

    fn seal(
        run: &Run<'_>,
        exact_execution: &[Candle],
        expected_data_digest: [u8; 32],
    ) -> Result<Self, ExitGridErrorV1> {
        if run.timeframe.is_empty() {
            return Err(ExitGridErrorV1::MissingRunIdentity("timeframe"));
        }
        if run.feed.is_empty() {
            return Err(ExitGridErrorV1::MissingRunIdentity("feed"));
        }
        if run.commit.is_empty() {
            return Err(ExitGridErrorV1::MissingRunIdentity("commit"));
        }
        if run.direction == Direction::Undirected {
            return Err(ExitGridErrorV1::UndirectedExecutionRun);
        }
        if run.data_digest != expected_data_digest {
            return Err(ExitGridErrorV1::RunDataDigestMismatch);
        }
        Ok(Self {
            run_id: crate::identity::identity(run),
            instrument: *run.instrument,
            mask: run.mask,
            direction: run.direction,
            feed_digest: hash(run.feed.as_bytes()),
            commit_digest: hash(run.commit.as_bytes()),
            execution_digest: crate::identity::data_digest(exact_execution),
        })
    }

    /// Canonical nine-term run identity.
    #[must_use]
    pub const fn run_id(self) -> RunId {
        self.run_id
    }

    /// Combination carried by the sealed run.
    #[must_use]
    pub const fn mask(self) -> ConditionMask {
        self.mask
    }

    fn require_matches(
        self,
        series: ExecutionSeriesV1<'_>,
        side: crate::excursion::Side,
    ) -> Result<(), ExitGridErrorV1> {
        if self.instrument != *series.instrument() {
            return Err(ExitGridErrorV1::RunIdentityMismatch("instrument"));
        }
        if self.feed_digest != hash(series.feed().as_bytes()) {
            return Err(ExitGridErrorV1::RunIdentityMismatch("feed"));
        }
        if self.commit_digest != hash(series.commit().as_bytes()) {
            return Err(ExitGridErrorV1::RunIdentityMismatch("commit"));
        }
        if self.execution_digest != crate::identity::data_digest(series.bars()) {
            return Err(ExitGridErrorV1::RunIdentityMismatch("execution data"));
        }
        let expected = match side {
            crate::excursion::Side::Long => Direction::Long,
            crate::excursion::Side::Short => Direction::Short,
        };
        if self.direction != expected {
            return Err(ExitGridErrorV1::RunIdentityMismatch("direction"));
        }
        Ok(())
    }
}

fn require_exact_execution_subslice(
    reference_minute_context: &[Candle],
    evaluated_execution_1m: &[Candle],
) -> Result<(), ExitGridErrorV1> {
    let Some(first) = evaluated_execution_1m.first() else {
        return Err(ExitGridErrorV1::EmptyExecutionSeries);
    };
    let Some(start) = reference_minute_context.iter().position(|bar| bar == first) else {
        return Err(ExitGridErrorV1::EvaluatedExecutionOutsideReferenceContext);
    };
    let Some(end) = start.checked_add(evaluated_execution_1m.len()) else {
        return Err(ExitGridErrorV1::EvaluatedExecutionOutsideReferenceContext);
    };
    if reference_minute_context.get(start..end) != Some(evaluated_execution_1m) {
        return Err(ExitGridErrorV1::EvaluatedExecutionOutsideReferenceContext);
    }
    Ok(())
}

/// OOS execution input with an explicit first test bar.
///
/// Bars before `first_oos` may exist only as indicator warm-up context.  Replay
/// additionally proves that every signal source is at or after this index, so
/// no warm-up/training row can open a test trade.
#[derive(Clone, Copy, Debug)]
pub struct OosExecutionSeriesV1<'a> {
    series: ExecutionSeriesV1<'a>,
    first_oos: usize,
}

impl<'a> OosExecutionSeriesV1<'a> {
    /// Builds an explicit causal split.
    ///
    /// # Errors
    ///
    /// The split must index an existing bar.
    pub fn new(series: ExecutionSeriesV1<'a>, first_oos: usize) -> Result<Self, ExitGridErrorV1> {
        if first_oos >= series.bars.len() {
            return Err(ExitGridErrorV1::InvalidOosBoundary {
                first_oos,
                bars: series.bars.len(),
            });
        }
        Ok(Self { series, first_oos })
    }

    /// Complete attested execution slice, including any warm-up prefix.
    #[must_use]
    pub const fn series(self) -> ExecutionSeriesV1<'a> {
        self.series
    }

    /// First bar on which an OOS signal may exist.
    #[must_use]
    pub const fn first_oos(self) -> usize {
        self.first_oos
    }
}

/// Integer encoding of one observed printed axis distance.
///
/// Both divide by that candle's printed open. The existing excursion engine
/// places a candidate's PPM ladder from its printed entry open too, so a V1
/// training rung and `grid`'s crossing arithmetic share one denominator
/// convention. The sample is broader (all validated TRAINING minute bars), but
/// the unit is not silently changed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RangeResolutionV1 {
    /// `floor(span * 1_000_000 / open)`.
    PpmFloor,
    /// `ceil(span * 1_000_000 / open)`.
    PpmCeiling,
}

/// How a resolved cell is selected after quality and ratio admission.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExitGridSelectorV1 {
    /// Largest pessimistic total, with the existing simplicity tie-break.
    PessimisticTotal,
    /// Largest excursion edge, then pessimistic total and simplicity.
    EdgeThenPessimistic,
    /// Largest worst-case guaranteed floor, then total and simplicity.
    GuaranteedFloor,
}

/// Whether an operator-specified stop is absent, offered, or mandatory.
///
/// Both exact-observed variants refuse unless `ppm` is present in the TRAINING
/// range distribution. This is deliberately stricter than snapping: a silent
/// nearest value would change the stated risk rule.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ForcedStopV1 {
    /// No operator stop is added.
    Disabled,
    /// Add this exact observed TRAINING distance as a stop candidate.
    IncludeExactObserved(Ppm),
    /// Add this exact observed TRAINING distance and admit only cells using it.
    RequireExactObserved(Ppm),
}

/// Rational rung locations for each independent exit axis.
///
/// Vectors make the shape runtime-configurable. `max_levels_per_axis` is an
/// explicit resource bound rather than a hidden static rung count.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RungPlanV1 {
    stop: Vec<RationalPercentileV1>,
    target: Vec<RationalPercentileV1>,
    trail: Vec<RationalPercentileV1>,
    max_levels_per_axis: usize,
}

impl RungPlanV1 {
    /// Builds three strictly increasing rational percentile schedules.
    ///
    /// # Errors
    ///
    /// Refuses an empty schedule, zero bound, an input longer than the bound,
    /// or percentiles that are not strictly increasing by rational value.
    pub fn new(
        stop: Vec<RationalPercentileV1>,
        target: Vec<RationalPercentileV1>,
        trail: Vec<RationalPercentileV1>,
        max_levels_per_axis: usize,
    ) -> Result<Self, ExitGridErrorV1> {
        if max_levels_per_axis == 0 {
            return Err(ExitGridErrorV1::ZeroLimit("max_levels_per_axis"));
        }
        validate_percentiles("stop", &stop, max_levels_per_axis)?;
        validate_percentiles("target", &target, max_levels_per_axis)?;
        validate_percentiles("trail", &trail, max_levels_per_axis)?;
        Ok(Self {
            stop,
            target,
            trail,
            max_levels_per_axis,
        })
    }

    /// Stop-rung percentile schedule.
    #[must_use]
    pub fn stop(&self) -> &[RationalPercentileV1] {
        &self.stop
    }

    /// Target-rung percentile schedule.
    #[must_use]
    pub fn target(&self) -> &[RationalPercentileV1] {
        &self.target
    }

    /// Trailing-rung percentile schedule.
    #[must_use]
    pub fn trail(&self) -> &[RationalPercentileV1] {
        &self.trail
    }

    /// Maximum resolved rungs on any axis.
    #[must_use]
    pub const fn max_levels_per_axis(&self) -> usize {
        self.max_levels_per_axis
    }
}

/// Inclusive reward-to-risk admission and its explicit pair-count ceiling.
///
/// Ratios are hundredths: `100` is 1.00 and `250` is 2.50. The field names do
/// not call them basis points because that would be off by a factor of one
/// hundred. V1 compares `target * 100` to `stop * bound` by checked cross
/// multiplication. It intentionally does not floor `target / stop` first and
/// does not require equality to the older derived-ratio product ladder; an
/// exact rational interval cannot admit a value just above its maximum because
/// integer division rounded it down.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RatioLimitsV1 {
    min_hundredths: i64,
    max_hundredths: i64,
    max_pairs: u64,
}

impl RatioLimitsV1 {
    /// Builds a positive inclusive interval and non-zero pair ceiling.
    ///
    /// # Errors
    ///
    /// Refuses non-positive or inverted ratios and a zero pair ceiling.
    pub fn new(
        min_hundredths: i64,
        max_hundredths: i64,
        max_pairs: u64,
    ) -> Result<Self, ExitGridErrorV1> {
        if min_hundredths <= 0 || max_hundredths < min_hundredths {
            return Err(ExitGridErrorV1::InvalidRatioLimits {
                min_hundredths,
                max_hundredths,
            });
        }
        if max_pairs == 0 {
            return Err(ExitGridErrorV1::ZeroLimit("max_ratio_pairs"));
        }
        Ok(Self {
            min_hundredths,
            max_hundredths,
            max_pairs,
        })
    }

    /// Inclusive minimum target-to-stop ratio in hundredths.
    #[must_use]
    pub const fn min_hundredths(self) -> i64 {
        self.min_hundredths
    }

    /// Inclusive maximum target-to-stop ratio in hundredths.
    #[must_use]
    pub const fn max_hundredths(self) -> i64 {
        self.max_hundredths
    }

    /// Maximum admitted exact stop-target index pairs.
    #[must_use]
    pub const fn max_pairs(self) -> u64 {
        self.max_pairs
    }
}

/// Every runtime choice needed to resolve and select a V1 exit grid.
///
/// There is deliberately no [`Default`] implementation. A caller must name the
/// resolution, rational rungs, ratios, cell budget, selector, cost-model
/// identity, long/short side, forced-stop semantics, and uncertainty ceilings.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExitGridPolicyV1 {
    execution_resolution: ExecutionResolutionV1,
    range_resolution: RangeResolutionV1,
    side: crate::excursion::Side,
    rungs: RungPlanV1,
    ratios: RatioLimitsV1,
    max_cells: u64,
    selector: ExitGridSelectorV1,
    cost_model_id: [u8; 32],
    forced_stop: ForcedStopV1,
    max_ambiguous_bars: u64,
    max_gap_fills: u64,
}

impl ExitGridPolicyV1 {
    /// Canonical persisted schema version for this policy type.
    #[must_use]
    pub const fn version(&self) -> u16 {
        EXIT_GRID_POLICY_VERSION_V1
    }

    /// Constructs a complete explicit policy.
    ///
    /// # Errors
    ///
    /// Refuses a zero cell budget, an all-zero cost-model identity, or a
    /// non-positive exact forced stop. Unsupported execution durations are
    /// retained and later refused by [`Self::resolve`], allowing persisted
    /// input to fail with its exact duration rather than at an opaque parser.
    #[expect(
        clippy::too_many_arguments,
        reason = "these eleven independent decisions are intentionally explicit; bundling them behind defaults would recreate the hidden policy this type removes"
    )]
    pub fn new(
        execution_resolution: ExecutionResolutionV1,
        range_resolution: RangeResolutionV1,
        side: crate::excursion::Side,
        rungs: RungPlanV1,
        ratios: RatioLimitsV1,
        max_cells: u64,
        selector: ExitGridSelectorV1,
        cost_model_id: [u8; 32],
        forced_stop: ForcedStopV1,
        max_ambiguous_bars: u64,
        max_gap_fills: u64,
    ) -> Result<Self, ExitGridErrorV1> {
        if max_cells == 0 {
            return Err(ExitGridErrorV1::ZeroLimit("max_cells"));
        }
        if all_zero(&cost_model_id) {
            return Err(ExitGridErrorV1::MissingCostModelId);
        }
        match forced_stop {
            ForcedStopV1::IncludeExactObserved(ppm) | ForcedStopV1::RequireExactObserved(ppm)
                if ppm <= 0 =>
            {
                return Err(ExitGridErrorV1::InvalidForcedStop(ppm));
            }
            ForcedStopV1::Disabled
            | ForcedStopV1::IncludeExactObserved(_)
            | ForcedStopV1::RequireExactObserved(_) => {}
        }
        Ok(Self {
            execution_resolution,
            range_resolution,
            side,
            rungs,
            ratios,
            max_cells,
            selector,
            cost_model_id,
            forced_stop,
            max_ambiguous_bars,
            max_gap_fills,
        })
    }

    /// Resolve exact rungs from validated TRAINING one-minute OHLCV.
    ///
    /// The input is read once to validate and derive adverse, favourable and
    /// full-range samples, then the three axes are sorted. Nothing from a
    /// test/OOS slice enters this call.
    ///
    /// # Errors
    ///
    /// Refuses unsupported instruments/resolutions, empty or malformed data,
    /// arithmetic overflow, missing observed forced stops, empty ratio
    /// admission, and any rung/pair/cell budget breach.
    pub fn resolve_attested(
        &self,
        series: ExecutionSeriesV1<'_>,
    ) -> Result<ResolvedExitGridV1, ExitGridErrorV1> {
        let instrument = *series.instrument();
        let training_execution_1m = series.bars();
        let family = InstrumentFamilyV1::of(&instrument)?;
        if self.cost_model_id != printed_ohlcv_cost_model_id_v1() {
            return Err(ExitGridErrorV1::UnsupportedCostModelId);
        }
        if let ExecutionResolutionV1::UnsupportedSeconds(seconds) = self.execution_resolution {
            return Err(ExitGridErrorV1::UnsupportedExecutionResolution(seconds));
        }
        let forced_stop_level = forced_stop_level(self.forced_stop);
        let mut observed = observed_ranges(
            training_execution_1m,
            self.range_resolution,
            self.side,
            forced_stop_level,
        )?;
        observed.stops.sort_unstable();
        observed.targets.sort_unstable();
        observed.trails.sort_unstable();
        validate_observed_ranges(&observed, forced_stop_level)?;
        let (stops, forced_stop_index) = resolve_axis_with_forced(
            "stop",
            &observed.stops,
            self.rungs.stop(),
            forced_stop_level,
        )?;
        let targets = resolve_axis("target", &observed.targets, self.rungs.target())?;
        let trails = resolve_axis("trail", &observed.trails, self.rungs.trail())?;
        if stops.len() > self.rungs.max_levels_per_axis()
            || targets.len() > self.rungs.max_levels_per_axis()
            || trails.len() > self.rungs.max_levels_per_axis()
        {
            return Err(ExitGridErrorV1::ResolvedRungLimitExceeded {
                stop: stops.len(),
                target: targets.len(),
                trail: trails.len(),
                max: self.rungs.max_levels_per_axis(),
            });
        }

        let ratio_pairs = admitted_ratio_pairs(&stops, &targets, self.ratios)?;
        if ratio_pairs.is_empty() {
            return Err(ExitGridErrorV1::NoAdmittedRatioPair);
        }
        let pair_count = u64::try_from(ratio_pairs.len())
            .map_err(|_| ExitGridErrorV1::ArithmeticOverflow("ratio pair count"))?;
        if pair_count > self.ratios.max_pairs() {
            return Err(ExitGridErrorV1::RatioPairLimitExceeded {
                needed: pair_count,
                max: self.ratios.max_pairs(),
            });
        }
        let cell_count =
            checked_policy_cell_count(stops.len(), targets.len(), trails.len(), &ratio_pairs)
                .ok_or(ExitGridErrorV1::ArithmeticOverflow("grid cell count"))?;
        if cell_count > self.max_cells {
            return Err(ExitGridErrorV1::CellLimitExceeded {
                needed: cell_count,
                max: self.max_cells,
            });
        }
        let ratio_bitmap = ratio_bitmap_of(stops.len(), targets.len(), &ratio_pairs)?;

        let policy_digest = self.digest();
        let training_digest = crate::identity::data_digest(training_execution_1m);
        let training_bars = u64::try_from(training_execution_1m.len())
            .map_err(|_| ExitGridErrorV1::ArithmeticOverflow("training bar count"))?;
        let training_first_ts_micros = training_execution_1m
            .first()
            .map(|bar| bar.ts_micros)
            .ok_or(ExitGridErrorV1::EmptyExecutionSeries)?;
        let training_last_ts_micros = training_execution_1m
            .last()
            .map(|bar| bar.ts_micros)
            .ok_or(ExitGridErrorV1::EmptyExecutionSeries)?;
        let mut resolved = ResolvedExitGridV1 {
            instrument,
            family,
            feed_digest: hash(series.feed().as_bytes()),
            commit_digest: hash(series.commit().as_bytes()),
            calendar_digest: series.calendar_digest(),
            policy: self.clone(),
            policy_digest,
            training_digest,
            training_bars,
            training_first_ts_micros,
            training_last_ts_micros,
            stop_levels_ppm: stops,
            target_levels_ppm: targets,
            trail_levels_ppm: trails,
            ratio_pairs,
            ratio_bitmap,
            forced_stop_index,
            cell_count,
            digest: [0; 32],
        };
        resolved.digest = digest_resolved(&resolved);
        Ok(resolved)
    }

    /// Test-only shorthand whose fixed identities are visibly synthetic.
    #[cfg(test)]
    fn resolve(
        &self,
        instrument: &InstrumentKey,
        training_execution_1m: &[Candle],
    ) -> Result<ResolvedExitGridV1, ExitGridErrorV1> {
        let series = ExecutionSeriesV1::new(
            instrument,
            "test-feed",
            "test-commit",
            [0xA5; 32],
            training_execution_1m,
        )?;
        self.resolve_attested(series)
    }

    /// Canonical BLAKE3 digest over every policy field.
    #[must_use]
    pub fn digest(&self) -> [u8; 32] {
        let mut h = Hasher::new();
        h.update(b"brutex.runner.exit-grid-policy.v1");
        put_u16(&mut h, EXIT_GRID_POLICY_VERSION_V1);
        put_execution_resolution(&mut h, self.execution_resolution);
        put_range_resolution(&mut h, self.range_resolution);
        put_side(&mut h, self.side);
        put_percentiles(&mut h, self.rungs.stop());
        put_percentiles(&mut h, self.rungs.target());
        put_percentiles(&mut h, self.rungs.trail());
        put_usize(&mut h, self.rungs.max_levels_per_axis());
        put_i64(&mut h, self.ratios.min_hundredths());
        put_i64(&mut h, self.ratios.max_hundredths());
        put_u64(&mut h, self.ratios.max_pairs());
        put_u64(&mut h, self.max_cells);
        h.update(&[selector_byte(self.selector)]);
        h.update(&self.cost_model_id);
        put_forced_stop(&mut h, self.forced_stop);
        put_u64(&mut h, self.max_ambiguous_bars);
        put_u64(&mut h, self.max_gap_fills);
        h.finalize()
    }

    /// Configured execution-series resolution.
    #[must_use]
    pub const fn execution_resolution(&self) -> ExecutionResolutionV1 {
        self.execution_resolution
    }

    /// Configured observed-range integer encoding.
    #[must_use]
    pub const fn range_resolution(&self) -> RangeResolutionV1 {
        self.range_resolution
    }

    /// Trade direction whose adverse/favourable TRAINING ranges resolve the axes.
    #[must_use]
    pub const fn side(&self) -> crate::excursion::Side {
        self.side
    }

    /// Runtime rung plan.
    #[must_use]
    pub const fn rungs(&self) -> &RungPlanV1 {
        &self.rungs
    }

    /// Reward-to-risk pair bounds.
    #[must_use]
    pub const fn ratios(&self) -> RatioLimitsV1 {
        self.ratios
    }

    /// Exact ratio-filtered policy cell ceiling.
    #[must_use]
    pub const fn max_cells(&self) -> u64 {
        self.max_cells
    }

    /// Cell-selection rule.
    #[must_use]
    pub const fn selector(&self) -> ExitGridSelectorV1 {
        self.selector
    }

    /// Opaque identity of the cost/fill model the caller applies.
    #[must_use]
    pub const fn cost_model_id(&self) -> [u8; 32] {
        self.cost_model_id
    }

    /// Forced-stop inclusion/requirement semantics.
    #[must_use]
    pub const fn forced_stop(&self) -> ForcedStopV1 {
        self.forced_stop
    }

    /// Maximum ambiguous one-minute bars admitted on a selected cell.
    #[must_use]
    pub const fn max_ambiguous_bars(&self) -> u64 {
        self.max_ambiguous_bars
    }

    /// Maximum level exits filled through an opening gap.
    #[must_use]
    pub const fn max_gap_fills(&self) -> u64 {
        self.max_gap_fills
    }
}

/// Which of the two legal swept spot indices a resolution belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstrumentFamilyV1 {
    /// `NSE-NIFTY` spot index.
    Nifty,
    /// `NSE-BANKNIFTY` spot index.
    BankNifty,
}

impl InstrumentFamilyV1 {
    fn of(instrument: &InstrumentKey) -> Result<Self, ExitGridErrorV1> {
        if instrument.exchange != Exchange::Nse
            || instrument.segment != Segment::Index
            || instrument.kind != Kind::Index
        {
            return Err(ExitGridErrorV1::UnsupportedInstrument);
        }
        match instrument.underlying.as_str() {
            "NIFTY" => Ok(Self::Nifty),
            "BANKNIFTY" => Ok(Self::BankNifty),
            _ => Err(ExitGridErrorV1::UnsupportedInstrument),
        }
    }

    const fn byte(self) -> u8 {
        match self {
            Self::Nifty => 1,
            Self::BankNifty => 2,
        }
    }
}

/// One exact admissible `(stop rung, target rung)` coordinate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RatioPairV1 {
    stop_index: usize,
    target_index: usize,
}

impl RatioPairV1 {
    /// Index into [`ResolvedExitGridV1::stop_levels_ppm`].
    #[must_use]
    pub const fn stop_index(self) -> usize {
        self.stop_index
    }

    /// Index into [`ResolvedExitGridV1::target_levels_ppm`].
    #[must_use]
    pub const fn target_index(self) -> usize {
        self.target_index
    }
}

/// Owned exact ladders ready to borrow into [`Ladders`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedLaddersV1 {
    /// Exact TRAINING-resolved stop ladder.
    pub stops: Ladder,
    /// Exact TRAINING-resolved target ladder.
    pub targets: Ladder,
    /// Exact TRAINING-resolved trailing ladder.
    pub trails: Ladder,
}

impl ResolvedLaddersV1 {
    /// Borrows the three owned ladders as the existing grid engine expects.
    #[must_use]
    pub const fn borrowed(&self) -> Ladders<'_> {
        Ladders {
            stops: &self.stops,
            targets: &self.targets,
            trails: &self.trails,
        }
    }
}

/// One complete grid produced by [`ResolvedExitGridV1::evaluate_training_grid`].
///
/// Fields are private so a caller cannot attach an arbitrary public [`Grid`] to
/// a resolution and pass it to selection. Persistence will require its own
/// canonical codec and seal; this in-memory capability only proves which
/// constructor produced the value in this process.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvaluatedExitGridV1 {
    resolution_digest: [u8; 32],
    run_id: RunId,
    mask: ConditionMask,
    horizon: Horizon,
    column_digest: [u8; 32],
    evaluation_spec: EvaluationSpecToken,
    side: crate::excursion::Side,
    grid: Grid,
}

impl EvaluatedExitGridV1 {
    /// Exact resolution identity that authorized this evaluation.
    #[must_use]
    pub const fn resolution_digest(&self) -> [u8; 32] {
        self.resolution_digest
    }

    /// Direction priced by the complete grid.
    #[must_use]
    pub const fn side(&self) -> crate::excursion::Side {
        self.side
    }

    /// Canonical run identity under which the complete grid was priced.
    #[must_use]
    pub const fn run_id(&self) -> RunId {
        self.run_id
    }

    /// Exact combination priced by the grid.
    #[must_use]
    pub const fn mask(&self) -> ConditionMask {
        self.mask
    }

    /// Exact execution horizon priced by the grid.
    #[must_use]
    pub const fn horizon(&self) -> Horizon {
        self.horizon
    }

    /// Digest of the exact signal-to-execution column used by this grid.
    #[must_use]
    pub const fn column_digest(&self) -> [u8; 32] {
        self.column_digest
    }

    /// Read-only access for reporting and persistence adapters.
    #[must_use]
    pub const fn grid(&self) -> &Grid {
        &self.grid
    }
}

/// One complete evaluation after its full coordinate sequence was verified.
///
/// Validation is deliberately separate from coordinate authorization. It is
/// the one O(G) integrity pass over an opaque evaluated grid; this capability
/// then carries canonical row ordinals so every later Top-N coordinate lookup
/// is one checked index rather than another population walk.
#[derive(Debug)]
pub struct ValidatedExitGridV1<'a> {
    resolution_digest: [u8; 32],
    evaluation_digest: [u8; 32],
    evaluated: &'a EvaluatedExitGridV1,
    coordinate_row_offsets: Vec<Option<usize>>,
}

impl ValidatedExitGridV1<'_> {
    /// Identity of the resolution whose complete integrity pass minted this
    /// capability.
    #[must_use]
    pub const fn resolution_digest(&self) -> [u8; 32] {
        self.resolution_digest
    }

    /// Identity of the exact complete evaluation whose coordinate sequence
    /// passed the one full integrity check.
    ///
    /// This is an O(1) copy of the digest minted during validation. Durable
    /// adapters use it to bind one candidate row to that already-validated
    /// evaluation without scanning the grid a second time.
    ///
    /// **UNVERIFIED as a measured bound.** No bench in this workspace
    /// times this, so the shape above is read from the source rather
    /// than measured. `CLAUDE.md` §3 rule 6.
    #[must_use]
    pub const fn evaluation_digest(&self) -> [u8; 32] {
        self.evaluation_digest
    }

    /// Whether this capability was minted for this exact in-memory evaluation.
    ///
    /// This is an O(1) identity check.  It lets a downstream adapter retain the
    /// single O(G) validation boundary instead of rebuilding it once per cell.
    ///
    /// **UNVERIFIED as a measured bound.** No bench in this workspace
    /// times this, so the shape above is read from the source rather
    /// than measured. `CLAUDE.md` §3 rule 6.
    #[must_use]
    pub fn is_evaluation(&self, evaluated: &EvaluatedExitGridV1) -> bool {
        core::ptr::eq(self.evaluated, evaluated)
    }

    /// Returns one already-validated canonical cell by ordinal.
    ///
    /// The complete coordinate sequence and cell count were checked when this
    /// capability was created, so this later access is one bounds-checked
    /// index and never another scan of the grid.
    #[must_use]
    pub fn cell(&self, ordinal: usize) -> Option<&Cell> {
        self.evaluated.grid.cells.get(ordinal)
    }
}

/// One strategy chosen from a complete training grid.
///
/// Private fields are the replay authority.  A caller cannot construct a
/// coordinate, swap its mask/horizon/side, or attach it to another resolution;
/// it can only pass this capability to [`ResolvedExitGridV1::replay_selected`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectedExitV1 {
    resolution_digest: [u8; 32],
    selection_digest: [u8; 32],
    run_id: RunId,
    mask: ConditionMask,
    horizon: Horizon,
    column_digest: [u8; 32],
    evaluation_spec: EvaluationSpecToken,
    side: crate::excursion::Side,
    coordinate: Chosen,
    training_cell: Cell,
}

impl SelectedExitV1 {
    /// Stable identity of the exact training choice and its complete context.
    #[must_use]
    pub const fn digest(&self) -> [u8; 32] {
        self.selection_digest
    }

    /// Exact selected coordinate, exposed read-only for durable codecs.
    #[must_use]
    pub const fn coordinate(&self) -> Chosen {
        self.coordinate
    }

    /// Complete measured training cell selected under the policy.
    #[must_use]
    pub const fn training_cell(&self) -> &Cell {
        &self.training_cell
    }

    /// Canonical training run identity.
    #[must_use]
    pub const fn run_id(&self) -> RunId {
        self.run_id
    }
}

/// Stable policy-refusal reasons for one canonical execution coordinate.
///
/// These bits describe measured or policy-defined reasons that a coordinate
/// cannot mint a [`SelectedExitV1`]. They never encode structural corruption:
/// a foreign validation capability, an out-of-range index, a disallowed ratio
/// pair, or a coordinate/population mismatch remains an [`ExitGridErrorV1`].
/// The integer values are part of the V1 boundary and may be persisted through
/// [`Self::bits`]; unknown bits are rejected by [`Self::from_bits`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExecutionRefusalBitsV1(u64);

impl ExecutionRefusalBitsV1 {
    /// No policy refusal was measured.
    pub const NONE: Self = Self(0);
    /// The canonical comparison row has no stop coordinate.
    pub const MISSING_STOP: Self = Self(1 << 0);
    /// The canonical comparison row has no target coordinate.
    pub const MISSING_TARGET: Self = Self(1 << 1);
    /// The evaluated training cell took no trades.
    pub const ZERO_TRADES: Self = Self(1 << 2);
    /// The evaluated training cell exceeded the ambiguity ceiling.
    pub const AMBIGUITY_LIMIT: Self = Self(1 << 3);
    /// The evaluated training cell exceeded the opening-gap ceiling.
    pub const GAP_LIMIT: Self = Self(1 << 4);
    /// The cell's stop differs from the policy's mandatory exact stop.
    pub const FORCED_STOP_MISMATCH: Self = Self(1 << 5);
    /// Every refusal bit defined by V1.
    pub const ALL: Self = Self(
        Self::MISSING_STOP.0
            | Self::MISSING_TARGET.0
            | Self::ZERO_TRADES.0
            | Self::AMBIGUITY_LIMIT.0
            | Self::GAP_LIMIT.0
            | Self::FORCED_STOP_MISMATCH.0,
    );

    /// Reconstructs a refusal bitmap only when every bit is known to V1.
    #[must_use]
    pub const fn from_bits(bits: u64) -> Option<Self> {
        if bits & !Self::ALL.0 == 0 {
            Some(Self(bits))
        } else {
            None
        }
    }

    /// Exact stable integer representation.
    #[must_use]
    pub const fn bits(self) -> u64 {
        self.0
    }

    /// Whether no policy refusal reason is present.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Whether every bit in `reason` is present.
    #[must_use]
    pub const fn contains(self, reason: Self) -> bool {
        self.0 & reason.0 == reason.0
    }

    /// Combines independently measured V1 refusal reasons.
    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

/// Opaque result of classifying one canonical execution coordinate.
///
/// Only [`ResolvedExitGridV1::classify_coordinate`] can construct this value.
/// An authorized result carries the exact opaque [`SelectedExitV1`]; a policy
/// refusal carries one or more stable [`ExecutionRefusalBitsV1`] reasons.
/// Structural errors are returned separately and can never be represented as
/// a successful disposition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionDispositionV1 {
    selected: Option<SelectedExitV1>,
    refusal_bits: ExecutionRefusalBitsV1,
    coordinate: Chosen,
    resolution_digest: [u8; 32],
    run_id: RunId,
    mask: ConditionMask,
    horizon: Horizon,
    column_digest: [u8; 32],
    evaluation_spec_fingerprint: [u8; indicators::column::EVALUATION_SPEC_FINGERPRINT_V1_LEN],
    side: crate::excursion::Side,
    context_digest: [u8; 32],
    disposition_digest: [u8; 32],
}

impl ExecutionDispositionV1 {
    /// Exact selected-exit capability when this coordinate was authorized.
    #[must_use]
    pub const fn selected(&self) -> Option<&SelectedExitV1> {
        self.selected.as_ref()
    }

    /// Stable refusal bitmap, or [`ExecutionRefusalBitsV1::NONE`] when
    /// authorized.
    #[must_use]
    pub const fn refusal_bits(&self) -> ExecutionRefusalBitsV1 {
        self.refusal_bits
    }

    /// Exact canonical coordinate whose terminal disposition was classified.
    #[must_use]
    pub const fn coordinate(&self) -> Chosen {
        self.coordinate
    }

    /// Exact resolved-grid identity that owns the coordinate.
    #[must_use]
    pub const fn resolution_digest(&self) -> [u8; 32] {
        self.resolution_digest
    }

    /// Canonical training-run identity that produced the evaluated cell.
    #[must_use]
    pub const fn run_id(&self) -> RunId {
        self.run_id
    }

    /// Exact combination evaluated for this coordinate.
    #[must_use]
    pub const fn mask(&self) -> ConditionMask {
        self.mask
    }

    /// Exact one-minute execution horizon used to measure the cell.
    #[must_use]
    pub const fn horizon(&self) -> Horizon {
        self.horizon
    }

    /// Identity of the exact condition column used by grid evaluation.
    #[must_use]
    pub const fn column_digest(&self) -> [u8; 32] {
        self.column_digest
    }

    /// Complete canonical evaluator-policy fingerprint for durable comparison.
    #[must_use]
    pub fn evaluation_spec_fingerprint(
        &self,
    ) -> [u8; indicators::column::EVALUATION_SPEC_FINGERPRINT_V1_LEN] {
        self.evaluation_spec_fingerprint
    }

    /// Long or short side whose exact grid produced this cell.
    #[must_use]
    pub const fn side(&self) -> crate::excursion::Side {
        self.side
    }

    /// Identity of the complete evaluated grid, training context, coordinate,
    /// and canonical measured cell before its terminal outcome is attached.
    #[must_use]
    pub const fn context_digest(&self) -> [u8; 32] {
        self.context_digest
    }

    /// Domain-separated identity of this exact authorized or policy-refused
    /// terminal outcome.
    #[must_use]
    pub const fn disposition_digest(&self) -> [u8; 32] {
        self.disposition_digest
    }

    /// Whether this disposition carries a selected-exit capability.
    #[must_use]
    pub const fn is_authorized(&self) -> bool {
        self.selected.is_some()
    }

    /// Whether this canonical coordinate was refused by measured policy.
    #[must_use]
    pub const fn is_policy_refused(&self) -> bool {
        self.selected.is_none()
    }

    fn authorized(binding: &ExecutionDispositionBindingV1, selected: SelectedExitV1) -> Self {
        Self {
            selected: Some(selected),
            refusal_bits: ExecutionRefusalBitsV1::NONE,
            coordinate: binding.coordinate,
            resolution_digest: binding.resolution_digest,
            run_id: binding.run_id,
            mask: binding.mask,
            horizon: binding.horizon,
            column_digest: binding.column_digest,
            evaluation_spec_fingerprint: binding.evaluation_spec_fingerprint,
            side: binding.side,
            context_digest: binding.context_digest,
            disposition_digest: binding.disposition_digest,
        }
    }

    fn policy_refused(
        binding: &ExecutionDispositionBindingV1,
        bits: ExecutionRefusalBitsV1,
    ) -> Self {
        debug_assert!(!bits.is_empty());
        Self {
            selected: None,
            refusal_bits: bits,
            coordinate: binding.coordinate,
            resolution_digest: binding.resolution_digest,
            run_id: binding.run_id,
            mask: binding.mask,
            horizon: binding.horizon,
            column_digest: binding.column_digest,
            evaluation_spec_fingerprint: binding.evaluation_spec_fingerprint,
            side: binding.side,
            context_digest: binding.context_digest,
            disposition_digest: binding.disposition_digest,
        }
    }

    fn into_selected(self) -> Option<SelectedExitV1> {
        self.selected
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ExecutionDispositionBindingV1 {
    coordinate: Chosen,
    resolution_digest: [u8; 32],
    run_id: RunId,
    mask: ConditionMask,
    horizon: Horizon,
    column_digest: [u8; 32],
    evaluation_spec_fingerprint: [u8; indicators::column::EVALUATION_SPEC_FINGERPRINT_V1_LEN],
    side: crate::excursion::Side,
    context_digest: [u8; 32],
    disposition_digest: [u8; 32],
}

/// One OOS replay bound to the frozen training selection and exact OOS bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplayedExitV1 {
    run_id: RunId,
    selected_digest: [u8; 32],
    oos_data_digest: [u8; 32],
    column_digest: [u8; 32],
    digest: [u8; 32],
    cell: Option<Cell>,
}

impl ReplayedExitV1 {
    /// Exact OOS result; `None` means the frozen strategy took no trade.
    #[must_use]
    pub const fn cell(&self) -> Option<&Cell> {
        self.cell.as_ref()
    }

    /// Identity of selection, OOS run, OOS bytes and OOS column.
    #[must_use]
    pub const fn digest(&self) -> [u8; 32] {
        self.digest
    }

    /// Canonical OOS run identity supplied by the caller.
    #[must_use]
    pub const fn run_id(&self) -> RunId {
        self.run_id
    }
}

/// Complete chosen-coordinate OOS candidate evidence before global
/// one-position arbitration.
///
/// Every member is a known, reachable entry. A member whose pricing was
/// refused still holds the global lock through its conservative inclusive
/// time exit; it merely carries no money row. Private fields and the digest
/// prevent a caller from splicing candidates from another run/selection into
/// this capability.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplayedCandidateUniverseV1 {
    run_id: RunId,
    selected_digest: [u8; 32],
    oos_data_digest: [u8; 32],
    column_digest: [u8; 32],
    digest: [u8; 32],
    local_cell: Option<Cell>,
    candidates: Vec<crate::grid::ReplayCandidateV1>,
    pricing_refused_paths: u64,
}

/// Opaque OOS replay authority for the durable global single-position join.
///
/// This type is minted only by [`ResolvedExitGridV1::replay_global_witness`]
/// after the ordinary selected-universe replay has authenticated the exact
/// instrument, feed, direction, run identity, OOS boundary, selected exit and
/// ordered candidate evidence.  Consequently a downstream persistence crate
/// never accepts those facts as loose caller-authored fields.
///
/// The inner replay universe stays private.  Read-only projections are enough
/// for a versioned global scheduler to bind its own records without copying a
/// legacy replay byte format.
pub struct GlobalReplayWitnessUniverseV1 {
    instrument: InstrumentKey,
    feed: Vendor,
    direction: CostDirection,
    first_oos: usize,
    digest: [u8; 32],
    universe: ReplayedCandidateUniverseV1,
}

impl GlobalReplayWitnessUniverseV1 {
    /// Exact swept spot index whose OOS bytes were replayed.
    #[must_use]
    pub const fn instrument(&self) -> InstrumentKey {
        self.instrument
    }

    /// Canonical stored vendor whose exact OOS bars were replayed.
    #[must_use]
    pub const fn feed(&self) -> Vendor {
        self.feed
    }

    /// Long or short direction already checked against the sealed run.
    #[must_use]
    pub const fn direction(&self) -> CostDirection {
        self.direction
    }

    /// First bar permitted to create an OOS signal.
    #[must_use]
    pub const fn first_oos(&self) -> usize {
        self.first_oos
    }

    /// Canonical nine-term OOS run identity.
    #[must_use]
    pub const fn run_id(&self) -> RunId {
        self.universe.run_id
    }

    /// Exact selected-exit identity replayed by the candidate universe.
    #[must_use]
    pub const fn selected_exit_digest(&self) -> [u8; 32] {
        self.universe.selected_digest
    }

    /// Identity of the complete ordered pre-exclusivity candidate universe.
    #[must_use]
    pub const fn universe_digest(&self) -> [u8; 32] {
        self.universe.digest
    }

    /// Complete ordered reachable entries, including pricing refusals.
    #[must_use]
    pub fn candidates(&self) -> &[crate::grid::ReplayCandidateV1] {
        self.universe.candidates()
    }

    /// Fail-closed integrity gate for the complete successor capability.
    ///
    /// # Errors
    ///
    /// [`ExitGridErrorV1::ReplayEvidenceDigestMismatch`] when the replay
    /// evidence or any successor identity projection no longer matches its
    /// seal.
    pub fn require_integrity(&self) -> Result<(), ExitGridErrorV1> {
        self.universe.require_integrity()?;
        let expected = digest_global_replay_witness(
            &self.instrument,
            self.feed,
            self.direction,
            self.first_oos,
            &self.universe,
        );
        if self.digest == expected {
            Ok(())
        } else {
            Err(ExitGridErrorV1::ReplayEvidenceDigestMismatch)
        }
    }
}

impl ReplayedCandidateUniverseV1 {
    /// Canonical OOS run identity supplied by the caller.
    #[must_use]
    pub const fn run_id(&self) -> RunId {
        self.run_id
    }

    /// Exact frozen training selection this universe replayed.
    #[must_use]
    pub const fn selected_digest(&self) -> [u8; 32] {
        self.selected_digest
    }

    /// Ordered reachable entries before any strategy-local/global occupancy
    /// decision.
    #[must_use]
    pub fn candidates(&self) -> &[crate::grid::ReplayCandidateV1] {
        &self.candidates
    }

    /// Paths whose entry is reachable but whose exit pricing was refused.
    ///
    /// These remain occupancy authority; this count must never be interpreted
    /// as permission to remove them from the global scheduler.
    #[must_use]
    pub const fn pricing_refused_paths(&self) -> u64 {
        self.pricing_refused_paths
    }

    /// Strategy-local one-position fold, retained only as a reconciliation
    /// fact. Global arbitration must consume [`Self::candidates`] instead.
    #[must_use]
    pub const fn local_cell(&self) -> Option<&Cell> {
        self.local_cell.as_ref()
    }

    /// Identity of selection, OOS bytes/column and every ordered candidate.
    #[must_use]
    pub const fn digest(&self) -> [u8; 32] {
        self.digest
    }

    /// Whether every carried field still matches the opaque replay digest.
    #[must_use]
    pub fn digest_is_valid(&self) -> bool {
        self.evidence_reconciles()
            && self.digest
                == digest_replay_universe(
                    self.run_id,
                    self.selected_digest,
                    self.oos_data_digest,
                    self.column_digest,
                    self.local_cell.as_ref(),
                    &self.candidates,
                    self.pricing_refused_paths,
                )
    }

    fn evidence_reconciles(&self) -> bool {
        let mut prior: Option<(usize, i64)> = None;
        let mut pricing_refused = 0_u64;
        for candidate in &self.candidates {
            if candidate.occupied_through_bar < candidate.entry_bar
                || candidate.occupied_through_micros < candidate.entry_micros
                || prior.is_some_and(|(bar, stamp)| {
                    candidate.entry_bar <= bar || candidate.entry_micros <= stamp
                })
            {
                return false;
            }
            prior = Some((candidate.entry_bar, candidate.entry_micros));
            match candidate.path {
                crate::grid::ReplayPathV1::Priceable(price) => {
                    let row = price.row;
                    if price.ambiguous_bars > 1
                        || price.gap_fills > 1
                        || row.signal_bar != candidate.signal_bar
                        || row.entry_bar != candidate.entry_bar
                        || row.exit_bar != candidate.occupied_through_bar
                        || row.entry_micros != candidate.entry_micros
                        || row.exit_micros != candidate.occupied_through_micros
                    {
                        return false;
                    }
                }
                crate::grid::ReplayPathV1::BlockOnly
                | crate::grid::ReplayPathV1::CrossingRefused
                | crate::grid::ReplayPathV1::BlockOnlyAndCrossingRefused => {
                    let Some(next) = pricing_refused.checked_add(1) else {
                        return false;
                    };
                    pricing_refused = next;
                }
            }
        }
        pricing_refused == self.pricing_refused_paths
    }

    /// Fail-closed integrity gate for a scheduler or durable decoder.
    ///
    /// A consumer must call this before applying global occupancy; silently
    /// accepting a torn ordered universe would let a removed/shortened path
    /// create room for a trade the evidence still blocked.
    ///
    /// # Errors
    ///
    /// [`ExitGridErrorV1::ReplayEvidenceDigestMismatch`] when any persisted
    /// count, row or ordered-evidence identity differs from the sealed digest.
    pub fn require_integrity(&self) -> Result<(), ExitGridErrorV1> {
        if self.digest_is_valid() {
            Ok(())
        } else {
            Err(ExitGridErrorV1::ReplayEvidenceDigestMismatch)
        }
    }
}

/// Exact V1 exit-grid resolution derived from one TRAINING execution series.
///
/// There is deliberately no [`Default`] implementation: empty levels, absent
/// policy identity, or an all-zero digest are not a usable resolution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedExitGridV1 {
    instrument: InstrumentKey,
    family: InstrumentFamilyV1,
    feed_digest: [u8; 32],
    commit_digest: [u8; 32],
    calendar_digest: [u8; 32],
    policy: ExitGridPolicyV1,
    policy_digest: [u8; 32],
    training_digest: [u8; 32],
    training_bars: u64,
    training_first_ts_micros: i64,
    training_last_ts_micros: i64,
    stop_levels_ppm: Vec<Ppm>,
    target_levels_ppm: Vec<Ppm>,
    trail_levels_ppm: Vec<Ppm>,
    ratio_pairs: Vec<RatioPairV1>,
    ratio_bitmap: Vec<bool>,
    forced_stop_index: Option<usize>,
    cell_count: u64,
    digest: [u8; 32],
}

impl ResolvedExitGridV1 {
    /// Canonical persisted schema version for this resolution type.
    #[must_use]
    pub const fn version(&self) -> u16 {
        EXIT_GRID_POLICY_VERSION_V1
    }

    /// The only legal swept instrument family this resolution names.
    #[must_use]
    pub const fn family(&self) -> InstrumentFamilyV1 {
        self.family
    }

    /// Exact canonical instrument key; family labels are never used as a
    /// substitute at replay.
    #[must_use]
    pub const fn instrument(&self) -> InstrumentKey {
        self.instrument
    }

    /// Digest of the exact stored vendor/feed identity.
    #[must_use]
    pub const fn feed_digest(&self) -> [u8; 32] {
        self.feed_digest
    }

    /// Digest of the clean source commit.
    #[must_use]
    pub const fn commit_digest(&self) -> [u8; 32] {
        self.commit_digest
    }

    /// Exact accepted-session/calendar policy identity.
    #[must_use]
    pub const fn calendar_digest(&self) -> [u8; 32] {
        self.calendar_digest
    }

    /// Long/short side whose adverse and favourable TRAINING ranges resolved it.
    #[must_use]
    pub const fn side(&self) -> crate::excursion::Side {
        self.policy.side()
    }

    /// Complete policy carried beside the exact output.
    #[must_use]
    pub const fn policy(&self) -> &ExitGridPolicyV1 {
        &self.policy
    }

    /// Digest of every policy field.
    #[must_use]
    pub const fn policy_digest(&self) -> [u8; 32] {
        self.policy_digest
    }

    /// Digest of all seven fields of every TRAINING execution candle.
    #[must_use]
    pub const fn training_digest(&self) -> [u8; 32] {
        self.training_digest
    }

    /// Number of TRAINING execution candles bound into the resolution.
    #[must_use]
    pub const fn training_bars(&self) -> u64 {
        self.training_bars
    }

    /// First TRAINING timestamp bound into the resolution.
    #[must_use]
    pub const fn training_first_ts_micros(&self) -> i64 {
        self.training_first_ts_micros
    }

    /// Last TRAINING timestamp; every OOS entry source must be later.
    #[must_use]
    pub const fn training_last_ts_micros(&self) -> i64 {
        self.training_last_ts_micros
    }

    /// Exact ascending TRAINING-resolved stop distances.
    #[must_use]
    pub fn stop_levels_ppm(&self) -> &[Ppm] {
        &self.stop_levels_ppm
    }

    /// Exact ascending TRAINING-resolved target distances.
    #[must_use]
    pub fn target_levels_ppm(&self) -> &[Ppm] {
        &self.target_levels_ppm
    }

    /// Exact ascending TRAINING-resolved trailing distances.
    #[must_use]
    pub fn trail_levels_ppm(&self) -> &[Ppm] {
        &self.trail_levels_ppm
    }

    /// Exact stop-target index pairs within the configured ratio interval.
    #[must_use]
    pub fn ratio_pairs(&self) -> &[RatioPairV1] {
        &self.ratio_pairs
    }

    /// Row-major O(1) admission bitmap for exact stop-target coordinates.
    ///
    /// **UNVERIFIED as a measured bound.** No bench in this workspace
    /// times this, so the shape above is read from the source rather
    /// than measured. `CLAUDE.md` §3 rule 6.
    #[must_use]
    pub(crate) fn ratio_bitmap(&self) -> &[bool] {
        &self.ratio_bitmap
    }

    /// Resolved index of an included/required exact observed stop.
    #[must_use]
    pub const fn forced_stop_index(&self) -> Option<usize> {
        self.forced_stop_index
    }

    /// Exact grid width after ratio-coordinate admission.
    #[must_use]
    pub const fn cell_count(&self) -> u64 {
        self.cell_count
    }

    /// BLAKE3 over the complete policy, training digest, instrument, exact
    /// levels, exact ratio coordinates, forced-stop coordinate and cell count.
    #[must_use]
    pub const fn digest(&self) -> [u8; 32] {
        self.digest
    }

    /// Whether the carried policy digest and complete resolution digest still
    /// match their fields.
    ///
    /// A persistence/API boundary can call this before trusting decoded bytes;
    /// it catches a torn row even when all individual fields remain parseable.
    #[must_use]
    pub fn digest_is_valid(&self) -> bool {
        self.policy_digest == self.policy.digest() && self.digest == digest_resolved(self)
    }

    /// Enumerates the complete TRAINING grid over these exact resolved rungs.
    ///
    /// This is the production constructor [`Self::select`] requires. It checks
    /// that `bars` are the exact TRAINING bytes bound by this resolution, then
    /// uses one cached crossing table per candidate and O(1) ratio admission in
    /// [`crate::grid`]. No candidate-local ladder is derived a second time.
    ///
    /// **UNVERIFIED as a measured bound.** No bench in this workspace
    /// times this, so the shape above is read from the source rather
    /// than measured. `CLAUDE.md` §3 rule 6.
    ///
    /// # Errors
    ///
    /// Refuses a torn resolution, unsupported cost model, non-one-minute or
    /// changed TRAINING series, invalid exact ladders, and any mismatch between
    /// the resolved cell count and the enumerated coordinate population.
    pub fn evaluate_training_grid_attested(
        &self,
        series: ExecutionSeriesV1<'_>,
        column: &indicators::column::Column,
        horizon: Horizon,
        run: ExecutionRunV1,
    ) -> Result<EvaluatedExitGridV1, ExitGridErrorV1> {
        self.require_runtime_integrity()?;
        self.require_matching_series(series)?;
        run.require_matches(series, self.side())?;
        let bars = series.bars();
        validate_execution_bars(bars)?;
        let count = u64::try_from(bars.len())
            .map_err(|_| ExitGridErrorV1::ArithmeticOverflow("training replay bar count"))?;
        if count != self.training_bars || crate::identity::data_digest(bars) != self.training_digest
        {
            return Err(ExitGridErrorV1::TrainingSeriesMismatch);
        }
        let evaluation_spec = column
            .evaluation_spec_token()
            .ok_or(ExitGridErrorV1::MissingEvaluationSpec)?;
        require_complete_acceptance(column, bars.len())?;
        validate_column_sources(column, bars.len(), 0)?;
        validate_arithmetic_envelope(bars, self)?;
        let column_digest = digest_column(column);
        let grid = crate::grid::evaluate_resolved_policy_v1(
            bars,
            column,
            &run.mask,
            horizon,
            self.side(),
            self,
        )?;
        Ok(EvaluatedExitGridV1 {
            resolution_digest: self.digest,
            run_id: run.run_id,
            mask: run.mask,
            horizon,
            column_digest,
            evaluation_spec,
            side: self.side(),
            grid,
        })
    }

    /// Unit-test shorthand over the visibly synthetic source identity.
    #[cfg(test)]
    fn evaluate_training_grid(
        &self,
        bars: &[Candle],
        column: &indicators::column::Column,
        mask: &ConditionMask,
        horizon: Horizon,
        side: crate::excursion::Side,
    ) -> Result<EvaluatedExitGridV1, ExitGridErrorV1> {
        let series = ExecutionSeriesV1::new(
            &self.instrument,
            "test-feed",
            "test-commit",
            [0xA5; 32],
            bars,
        )?;
        let run = test_execution_run(&self.instrument, mask, side, bars)?;
        self.evaluate_training_grid_attested(series, column, horizon, run)
    }

    /// Rebuilds the exact resolved ladders without reading any market data.
    ///
    /// # Errors
    ///
    /// Refuses if an in-memory/persisted resolution has invalid ladder bytes.
    pub fn ladders(&self) -> Result<ResolvedLaddersV1, ExitGridErrorV1> {
        let stops = Ladder::new(self.stop_levels_ppm.clone())
            .ok_or(ExitGridErrorV1::InvalidResolvedLadder("stop"))?;
        let targets = Ladder::new(self.target_levels_ppm.clone())
            .ok_or(ExitGridErrorV1::InvalidResolvedLadder("target"))?;
        let trails = Ladder::new(self.trail_levels_ppm.clone())
            .ok_or(ExitGridErrorV1::InvalidResolvedLadder("trail"))?;
        Ok(ResolvedLaddersV1 {
            stops,
            targets,
            trails,
        })
    }

    /// Whether one cell obeys uncertainty, gap, forced-stop, and ratio policy.
    #[must_use]
    pub fn admits(&self, cell: &Cell) -> bool {
        self.execution_refusal_bits(cell).is_empty()
            && self.chosen_is_in_bounds(Chosen::from_cell(cell))
    }

    /// Selects one policy-admitted cell from a grid carrying these exact rungs.
    ///
    /// # Errors
    ///
    /// Refuses a grid whose ladders do not equal this TRAINING resolution or
    /// whose stored cell count breaches the policy ceiling.
    pub fn select(
        &self,
        evaluated: &EvaluatedExitGridV1,
    ) -> Result<Option<SelectedExitV1>, ExitGridErrorV1> {
        let grid = self.validated_evaluation(evaluated)?;
        let admitted = grid.cells.iter().filter(|cell| self.admits(cell));
        let selected = match self.policy.selector() {
            ExitGridSelectorV1::PessimisticTotal => {
                admitted.max_by_key(|cell| (cell.pessimistic, crate::grid::merit(cell)))
            }
            ExitGridSelectorV1::EdgeThenPessimistic => admitted.max_by_key(|cell| {
                (
                    cell.edge_ratio(),
                    cell.pessimistic,
                    crate::grid::merit(cell),
                )
            }),
            ExitGridSelectorV1::GuaranteedFloor => admitted.max_by_key(|cell| {
                (
                    cell.guaranteed_floor(),
                    cell.pessimistic,
                    crate::grid::merit(cell),
                )
            }),
        };
        Ok(selected.map(|cell| self.seal_selection(evaluated, cell)))
    }

    /// Verifies one complete evaluated population before external ranking
    /// authorizes coordinates from it.
    ///
    /// This is the single O(G) boundary: it verifies exact ladders, cell count,
    /// canonical coordinate order and zero refused paths, then builds the much
    /// smaller row-offset table used by [`Self::authorize_coordinate`]. Keep
    /// the returned opaque capability and reuse it for every retained Top-N
    /// coordinate from this population.
    ///
    /// # Errors
    ///
    /// Refuses a torn resolution/evaluation, a changed ladder, an incomplete
    /// or reordered coordinate population, refused execution paths, arithmetic
    /// overflow, or an allocation refusal for the bounded row-offset table.
    pub fn validate_evaluation<'a>(
        &self,
        evaluated: &'a EvaluatedExitGridV1,
    ) -> Result<ValidatedExitGridV1<'a>, ExitGridErrorV1> {
        self.validated_evaluation(evaluated)?;
        Ok(ValidatedExitGridV1 {
            resolution_digest: self.digest,
            evaluation_digest: digest_evaluated_exit_grid(evaluated),
            evaluated,
            coordinate_row_offsets: self.coordinate_row_offsets()?,
        })
    }

    /// Authorizes one exact coordinate retained by an external complete-
    /// population ranking pass.
    ///
    /// Unlike [`Self::select`], this method does not apply the policy's local
    /// one-cell selector. It exists for a Top-N pipeline whose ranking receipt
    /// already names a particular coordinate. The authority still comes from
    /// a [`ValidatedExitGridV1`] minted by [`Self::validate_evaluation`]. That
    /// boundary already checked exact ladders, coordinate sequence, resolution
    /// identity and refusal count. This later admission is O(1): one canonical
    /// row-offset lookup, one checked cell index and one policy bitmap read.
    ///
    /// **UNVERIFIED as a measured bound.** No bench in this workspace
    /// times this, so the shape above is read from the source rather
    /// than measured. `CLAUDE.md` §3 rule 6.
    ///
    /// # Errors
    ///
    /// Refuses a torn/incomplete evaluation, a coordinate outside the resolved
    /// ratio-filtered population, or a measured cell that failed the policy's
    /// trade, ambiguity, opening-gap or forced-stop limits.
    pub fn authorize_coordinate(
        &self,
        validated: &ValidatedExitGridV1<'_>,
        coordinate: Chosen,
    ) -> Result<SelectedExitV1, ExitGridErrorV1> {
        let missing_ratio_axis = coordinate.stop.is_none() || coordinate.target.is_none();
        let disposition = self.classify_coordinate(validated, coordinate)?;
        if let Some(selected) = disposition.into_selected() {
            return Ok(selected);
        }
        // Preserve this compatibility door's historical distinction: missing
        // ratio axes were invalid coordinates, while an existing two-sided
        // cell that failed measured policy was not admitted.
        if missing_ratio_axis {
            Err(ExitGridErrorV1::InvalidChosenCoordinate)
        } else {
            Err(ExitGridErrorV1::ChosenCellNotAdmitted)
        }
    }

    /// Classifies one canonical coordinate as authorized or policy-refused.
    ///
    /// Canonical baseline and one-sided comparison rows are successful
    /// classifications carrying missing-axis bits. This is what lets a durable
    /// population record one terminal disposition per evaluated cell without
    /// inventing a selected-exit capability. A foreign validation capability,
    /// malformed/out-of-range coordinate, disallowed ratio pair, or changed
    /// population remains a hard error and never becomes `PolicyRefused`.
    ///
    /// # Cost
    ///
    /// O(1): one row-offset lookup, one checked cell lookup, and a fixed six-
    /// reason policy fold after the caller retained the O(G) validation
    /// capability.
    ///
    /// **UNVERIFIED as a measured bound.** No bench in this workspace
    /// times this, so the shape above is read from the source rather
    /// than measured. `CLAUDE.md` §3 rule 6.
    ///
    /// # Errors
    ///
    /// Returns the exact structural, identity, arithmetic or population error
    /// before any policy-refusal disposition is minted.
    pub fn classify_coordinate(
        &self,
        validated: &ValidatedExitGridV1<'_>,
        coordinate: Chosen,
    ) -> Result<ExecutionDispositionV1, ExitGridErrorV1> {
        if validated.resolution_digest != self.digest {
            return Err(ExitGridErrorV1::EvaluationResolutionMismatch);
        }
        if !self.chosen_axes_are_in_bounds(coordinate) {
            return Err(ExitGridErrorV1::InvalidChosenCoordinate);
        }
        let ordinal = self
            .canonical_coordinate_ordinal(&validated.coordinate_row_offsets, coordinate)?
            .ok_or(ExitGridErrorV1::InvalidChosenCoordinate)?;
        let cell = validated
            .evaluated
            .grid
            .cells
            .get(ordinal)
            .ok_or(ExitGridErrorV1::ReplayCoordinatePopulationMismatch)?;
        if Chosen::from_cell(cell) != coordinate {
            return Err(ExitGridErrorV1::ReplayCoordinatePopulationMismatch);
        }
        let evaluated = validated.evaluated;
        let context_digest = digest_execution_disposition_context(
            validated.evaluation_digest,
            evaluated,
            coordinate,
            cell,
        );
        let binding = |disposition_digest| ExecutionDispositionBindingV1 {
            coordinate,
            resolution_digest: self.digest,
            run_id: evaluated.run_id,
            mask: evaluated.mask,
            horizon: evaluated.horizon,
            column_digest: evaluated.column_digest,
            evaluation_spec_fingerprint: evaluated.evaluation_spec.fingerprint_v1().into_bytes(),
            side: evaluated.side,
            context_digest,
            disposition_digest,
        };
        let refusal_bits = self.execution_refusal_bits(cell);
        if !refusal_bits.is_empty() {
            let disposition_digest = digest_execution_disposition(
                context_digest,
                EXECUTION_DISPOSITION_POLICY_REFUSED,
                refusal_bits,
                None,
            );
            let terminal_binding = binding(disposition_digest);
            return Ok(ExecutionDispositionV1::policy_refused(
                &terminal_binding,
                refusal_bits,
            ));
        }
        if !self.chosen_is_in_bounds(coordinate) {
            return Err(ExitGridErrorV1::InvalidChosenCoordinate);
        }
        let selected = self.seal_selection(evaluated, cell);
        if selected.resolution_digest != self.digest
            || selected.run_id != evaluated.run_id
            || selected.mask != evaluated.mask
            || selected.horizon != evaluated.horizon
            || selected.column_digest != evaluated.column_digest
            || selected.evaluation_spec != evaluated.evaluation_spec
            || selected.side != evaluated.side
            || selected.coordinate != coordinate
            || selected.training_cell != *cell
            || selected.selection_digest
                != digest_selected(
                    self.digest,
                    evaluated.run_id,
                    evaluated.mask,
                    evaluated.horizon,
                    evaluated.column_digest,
                    evaluated.evaluation_spec,
                    evaluated.side,
                    coordinate,
                    cell,
                )
        {
            return Err(ExitGridErrorV1::SelectionDigestMismatch);
        }
        let disposition_digest = digest_execution_disposition(
            context_digest,
            EXECUTION_DISPOSITION_AUTHORIZED,
            ExecutionRefusalBitsV1::NONE,
            Some(selected.selection_digest),
        );
        let terminal_binding = binding(disposition_digest);
        Ok(ExecutionDispositionV1::authorized(
            &terminal_binding,
            selected,
        ))
    }

    /// Replays the opaque TRAINING selection on causally later OOS bars.
    ///
    /// No OOS price contributes to a rung. The method rebuilds the stored exact
    /// ladders and delegates pricing to the checked V1 replay door.  Instrument,
    /// feed, commit, calendar, evaluator policy, side, mask, horizon and chosen
    /// coordinate all come from the carried capabilities rather than new loose
    /// arguments.
    ///
    /// # Errors
    ///
    /// Refuses a forged/torn selection, mismatched series, non-causal split,
    /// warm-up signal leakage, refused path, arithmetic envelope breach, or an
    /// OOS cell outside the ambiguity/gap ceilings.
    pub fn replay_selected(
        &self,
        oos: OosExecutionSeriesV1<'_>,
        column: &indicators::column::Column,
        selected: &SelectedExitV1,
        oos_run: ExecutionRunV1,
    ) -> Result<ReplayedExitV1, ExitGridErrorV1> {
        let universe = self.replay_selected_universe(oos, column, selected, oos_run)?;
        universe.require_integrity()?;
        if universe.pricing_refused_paths != 0 {
            return Err(ExitGridErrorV1::RefusedExecutionPaths {
                paths: universe.pricing_refused_paths,
            });
        }
        if let Some(value) = &universe.local_cell
            && (value.ambiguous_bars > self.policy.max_ambiguous_bars()
                || value.gapped > self.policy.max_gap_fills())
        {
            return Err(ExitGridErrorV1::ReplayQualityLimitExceeded {
                ambiguous_bars: value.ambiguous_bars,
                max_ambiguous_bars: self.policy.max_ambiguous_bars(),
                gap_fills: value.gapped,
                max_gap_fills: self.policy.max_gap_fills(),
            });
        }
        let digest = digest_replay(
            universe.run_id,
            universe.selected_digest,
            universe.oos_data_digest,
            universe.column_digest,
            universe.local_cell.as_ref(),
        );
        Ok(ReplayedExitV1 {
            run_id: universe.run_id,
            selected_digest: universe.selected_digest,
            oos_data_digest: universe.oos_data_digest,
            column_digest: universe.column_digest,
            digest,
            cell: universe.local_cell,
        })
    }

    /// Replays the frozen coordinate into the complete pre-exclusivity OOS
    /// candidate universe.
    ///
    /// This is the capability a global long/short one-position scheduler must
    /// consume. Every candidate is retained in source order and priced without
    /// sharing strategy-local occupancy with its neighbours. A known entry with
    /// an unpriceable exit remains reachable occupancy through its conservative
    /// time exit and carries no money row.
    ///
    /// The historical [`Self::replay_selected`] delegates here, then preserves
    /// its stricter behavior by refusing any pricing-refused path.
    ///
    /// # Cost
    ///
    /// O(B + C·(H + L)) time and O(B + C·L + C) transient space for B execution
    /// bars, C pre-exclusivity candidates, maximum chosen hold H and complete
    /// resolved crossing-table width L (including armed target×trail slots).
    /// This is intentionally outside the all-coordinate sweep loop.
    ///
    /// # Errors
    ///
    /// Returns the exact policy, series, column, selection, run-identity,
    /// candidate, pricing or arithmetic refusal encountered while validating
    /// and replaying the frozen coordinate over the OOS execution series.
    pub fn replay_selected_universe(
        &self,
        oos: OosExecutionSeriesV1<'_>,
        column: &indicators::column::Column,
        selected: &SelectedExitV1,
        oos_run: ExecutionRunV1,
    ) -> Result<ReplayedCandidateUniverseV1, ExitGridErrorV1> {
        self.require_runtime_integrity()?;
        if selected.resolution_digest != self.digest
            || selected.side != self.side()
            || selected.selection_digest
                != digest_selected(
                    self.digest,
                    selected.run_id,
                    selected.mask,
                    selected.horizon,
                    selected.column_digest,
                    selected.evaluation_spec,
                    selected.side,
                    selected.coordinate,
                    &selected.training_cell,
                )
        {
            return Err(ExitGridErrorV1::SelectionDigestMismatch);
        }
        let series = oos.series();
        self.require_matching_series(series)?;
        oos_run.require_matches(series, selected.side)?;
        if oos_run.mask != selected.mask {
            return Err(ExitGridErrorV1::RunIdentityMismatch("mask"));
        }
        let bars = series.bars();
        validate_execution_bars(bars)?;
        let first_oos = oos.first_oos();
        let first_test_stamp = bars.get(first_oos).map(|bar| bar.ts_micros).ok_or(
            ExitGridErrorV1::InvalidOosBoundary {
                first_oos,
                bars: bars.len(),
            },
        )?;
        if first_test_stamp <= self.training_last_ts_micros {
            return Err(ExitGridErrorV1::OosOverlapsTraining {
                training_last: self.training_last_ts_micros,
                first_oos: first_test_stamp,
            });
        }
        if column.evaluation_spec_token() != Some(selected.evaluation_spec) {
            return Err(ExitGridErrorV1::EvaluationSpecMismatch);
        }
        require_complete_acceptance(column, bars.len())?;
        validate_column_sources(column, bars.len(), first_oos)?;
        validate_arithmetic_envelope(bars, self)?;
        if !self.chosen_is_in_bounds(selected.coordinate) {
            return Err(ExitGridErrorV1::InvalidChosenCoordinate);
        }
        if matches!(
            self.policy.forced_stop(),
            ForcedStopV1::RequireExactObserved(_)
        ) && selected.coordinate.stop != self.forced_stop_index
        {
            return Err(ExitGridErrorV1::InvalidChosenCoordinate);
        }
        let ladders = self.ladders()?;
        let replay = crate::grid::replay_universe_v1(
            bars,
            column,
            &selected.mask,
            selected.horizon,
            selected.side,
            ladders.borrowed(),
            selected.coordinate,
        )?;
        let cell = replay.cell;
        let oos_data_digest = crate::identity::data_digest(bars);
        let column_digest = digest_column(column);
        let digest = digest_replay_universe(
            oos_run.run_id,
            selected.selection_digest,
            oos_data_digest,
            column_digest,
            cell.as_ref(),
            &replay.candidates,
            replay.refused_paths,
        );
        Ok(ReplayedCandidateUniverseV1 {
            run_id: oos_run.run_id,
            selected_digest: selected.selection_digest,
            oos_data_digest,
            column_digest,
            digest,
            local_cell: cell,
            candidates: replay.candidates,
            pricing_refused_paths: replay.refused_paths,
        })
    }

    /// Mints the opaque OOS authority consumed by Global Replay successors.
    ///
    /// This is deliberately a sibling of [`Self::replay_selected_universe`],
    /// not an adapter over its public projections.  The exact instrument,
    /// canonical stored vendor, direction and OOS boundary are captured from
    /// the same capabilities while the replay validates them.  Unknown feed
    /// strings refuse instead of being relabelled as a vendor.
    ///
    /// # Cost
    ///
    /// The replay has the same documented cost as
    /// [`Self::replay_selected_universe`].  Minting adds one fixed five-vendor
    /// lookup and one fixed-size digest; it does not scan bars or candidates.
    ///
    /// # Errors
    ///
    /// Returns the underlying replay refusal, a non-canonical stored vendor,
    /// or a post-mint integrity mismatch.
    pub fn replay_global_witness(
        &self,
        oos: OosExecutionSeriesV1<'_>,
        column: &indicators::column::Column,
        selected: &SelectedExitV1,
        oos_run: ExecutionRunV1,
    ) -> Result<GlobalReplayWitnessUniverseV1, ExitGridErrorV1> {
        let series = oos.series();
        let feed = Vendor::ALL
            .into_iter()
            .find(|vendor| vendor.as_str() == series.feed())
            .ok_or(ExitGridErrorV1::RunIdentityMismatch(
                "feed is not a canonical stored vendor",
            ))?;
        let direction = match self.side() {
            crate::excursion::Side::Long => CostDirection::Long,
            crate::excursion::Side::Short => CostDirection::Short,
        };
        let first_oos = oos.first_oos();
        let universe = self.replay_selected_universe(oos, column, selected, oos_run)?;
        universe.require_integrity()?;
        let instrument = *series.instrument();
        let digest =
            digest_global_replay_witness(&instrument, feed, direction, first_oos, &universe);
        let witness = GlobalReplayWitnessUniverseV1 {
            instrument,
            feed,
            direction,
            first_oos,
            digest,
            universe,
        };
        witness.require_integrity()?;
        Ok(witness)
    }

    fn require_runtime_integrity(&self) -> Result<(), ExitGridErrorV1> {
        if !self.digest_is_valid() {
            return Err(ExitGridErrorV1::ResolutionDigestMismatch);
        }
        if self.policy.cost_model_id() != printed_ohlcv_cost_model_id_v1() {
            return Err(ExitGridErrorV1::UnsupportedCostModelId);
        }
        Ok(())
    }

    fn validated_evaluation<'a>(
        &self,
        evaluated: &'a EvaluatedExitGridV1,
    ) -> Result<&'a Grid, ExitGridErrorV1> {
        self.require_runtime_integrity()?;
        if evaluated.resolution_digest != self.digest || evaluated.side != self.side() {
            return Err(ExitGridErrorV1::EvaluationResolutionMismatch);
        }
        let grid = &evaluated.grid;
        if grid.stops.rungs() != self.stop_levels_ppm()
            || grid.targets.rungs() != self.target_levels_ppm()
            || grid.trails.rungs() != self.trail_levels_ppm()
        {
            return Err(ExitGridErrorV1::ReplayLadderMismatch);
        }
        let cells = u64::try_from(grid.cells.len())
            .map_err(|_| ExitGridErrorV1::ArithmeticOverflow("replay cell count"))?;
        if cells != self.cell_count {
            return Err(ExitGridErrorV1::ReplayCellCountMismatch {
                expected: self.cell_count,
                actual: cells,
            });
        }
        if !self.coordinate_sequence_is_complete(&grid.cells) {
            return Err(ExitGridErrorV1::ReplayCoordinatePopulationMismatch);
        }
        if grid.refused_paths != 0 {
            return Err(ExitGridErrorV1::RefusedExecutionPaths {
                paths: grid.refused_paths,
            });
        }
        Ok(grid)
    }

    fn seal_selection(
        &self,
        evaluated: &EvaluatedExitGridV1,
        training_cell: &Cell,
    ) -> SelectedExitV1 {
        let coordinate = Chosen::from_cell(training_cell);
        let selection_digest = digest_selected(
            self.digest,
            evaluated.run_id,
            evaluated.mask,
            evaluated.horizon,
            evaluated.column_digest,
            evaluated.evaluation_spec,
            evaluated.side,
            coordinate,
            training_cell,
        );
        SelectedExitV1 {
            resolution_digest: self.digest,
            selection_digest,
            run_id: evaluated.run_id,
            mask: evaluated.mask,
            horizon: evaluated.horizon,
            column_digest: evaluated.column_digest,
            evaluation_spec: evaluated.evaluation_spec,
            side: evaluated.side,
            coordinate,
            training_cell: *training_cell,
        }
    }

    fn require_matching_series(
        &self,
        series: ExecutionSeriesV1<'_>,
    ) -> Result<(), ExitGridErrorV1> {
        if *series.instrument() != self.instrument {
            return Err(ExitGridErrorV1::SeriesIdentityMismatch("instrument"));
        }
        if hash(series.feed().as_bytes()) != self.feed_digest {
            return Err(ExitGridErrorV1::SeriesIdentityMismatch("feed"));
        }
        if hash(series.commit().as_bytes()) != self.commit_digest {
            return Err(ExitGridErrorV1::SeriesIdentityMismatch("commit"));
        }
        if series.calendar_digest() != self.calendar_digest {
            return Err(ExitGridErrorV1::SeriesIdentityMismatch("calendar"));
        }
        Ok(())
    }

    fn coordinate_row_offsets(&self) -> Result<Vec<Option<usize>>, ExitGridErrorV1> {
        let stop_settings = self.stop_levels_ppm.len().checked_add(1).ok_or(
            ExitGridErrorV1::ArithmeticOverflow("coordinate stop settings"),
        )?;
        let target_settings = self.target_levels_ppm.len().checked_add(1).ok_or(
            ExitGridErrorV1::ArithmeticOverflow("coordinate target settings"),
        )?;
        let row_count = stop_settings.checked_mul(target_settings).ok_or(
            ExitGridErrorV1::ArithmeticOverflow("coordinate row-offset count"),
        )?;
        let mut offsets = Vec::new();
        offsets.try_reserve_exact(row_count).map_err(|_| {
            ExitGridErrorV1::BufferAllocationRefused {
                buffer: "coordinate row offsets",
                elements: row_count,
            }
        })?;
        let mut ordinal = 0_usize;
        for stop_setting in 0..stop_settings {
            for target_setting in 0..target_settings {
                let stop = (stop_setting < self.stop_levels_ppm.len()).then_some(stop_setting);
                let target =
                    (target_setting < self.target_levels_ppm.len()).then_some(target_setting);
                if let (Some(stop_index), Some(target_index)) = (stop, target)
                    && !self.ratio_pair_admitted(stop_index, target_index)
                {
                    offsets.push(None);
                    continue;
                }
                offsets.push(Some(ordinal));
                ordinal = ordinal
                    .checked_add(coordinate_row_width(
                        target_setting,
                        self.trail_levels_ppm.len(),
                    )?)
                    .ok_or(ExitGridErrorV1::ArithmeticOverflow("coordinate row offset"))?;
            }
        }
        let actual = u64::try_from(ordinal)
            .map_err(|_| ExitGridErrorV1::ArithmeticOverflow("coordinate population width"))?;
        if actual != self.cell_count {
            return Err(ExitGridErrorV1::ReplayCellCountMismatch {
                expected: self.cell_count,
                actual,
            });
        }
        Ok(offsets)
    }

    fn canonical_coordinate_ordinal(
        &self,
        row_offsets: &[Option<usize>],
        coordinate: Chosen,
    ) -> Result<Option<usize>, ExitGridErrorV1> {
        let stop_setting = coordinate.stop.unwrap_or(self.stop_levels_ppm.len());
        let target_setting = coordinate.target.unwrap_or(self.target_levels_ppm.len());
        let target_settings = self.target_levels_ppm.len().checked_add(1).ok_or(
            ExitGridErrorV1::ArithmeticOverflow("coordinate target settings"),
        )?;
        let row_slot = stop_setting
            .checked_mul(target_settings)
            .and_then(|row| row.checked_add(target_setting))
            .ok_or(ExitGridErrorV1::ArithmeticOverflow(
                "coordinate row-offset slot",
            ))?;
        let Some(row_start) = row_offsets.get(row_slot).copied().flatten() else {
            return Ok(None);
        };
        let trail_setting = coordinate.tsl.unwrap_or(self.trail_levels_ppm.len());
        let prior_trail_groups = trail_setting
            .checked_add(
                target_setting
                    .checked_mul(checked_sum_below(trail_setting)?)
                    .ok_or(ExitGridErrorV1::ArithmeticOverflow(
                        "coordinate prior armed trails",
                    ))?,
            )
            .ok_or(ExitGridErrorV1::ArithmeticOverflow(
                "coordinate trail-group offset",
            ))?;
        let within_group = match coordinate.ttp {
            None => 0,
            Some(ttp) => ttp
                .arm
                .checked_mul(trail_setting)
                .and_then(|arm| arm.checked_add(ttp.trail))
                .and_then(|offset| offset.checked_add(1))
                .ok_or(ExitGridErrorV1::ArithmeticOverflow(
                    "coordinate armed-trail offset",
                ))?,
        };
        row_start
            .checked_add(prior_trail_groups)
            .and_then(|offset| offset.checked_add(within_group))
            .map(Some)
            .ok_or(ExitGridErrorV1::ArithmeticOverflow(
                "coordinate canonical ordinal",
            ))
    }

    fn coordinate_sequence_is_complete(&self, cells: &[Cell]) -> bool {
        let mut actual = cells.iter();
        let visited = self.visit_coordinates(|expected| {
            actual
                .next()
                .is_some_and(|cell| Chosen::from_cell(cell) == expected)
        });
        visited && actual.next().is_none()
    }

    /// Visits the complete canonical ratio-filtered coordinate sequence.
    ///
    /// Returning `false` from `visit` stops immediately and propagates false.
    pub(crate) fn visit_coordinates(&self, mut visit: impl FnMut(Chosen) -> bool) -> bool {
        for stop_setting in 0..=self.stop_levels_ppm.len() {
            for target_setting in 0..=self.target_levels_ppm.len() {
                let stop = (stop_setting < self.stop_levels_ppm.len()).then_some(stop_setting);
                let target =
                    (target_setting < self.target_levels_ppm.len()).then_some(target_setting);
                if let (Some(stop_index), Some(target_index)) = (stop, target)
                    && !self.ratio_pair_admitted(stop_index, target_index)
                {
                    continue;
                }
                for trail_setting in 0..=self.trail_levels_ppm.len() {
                    let tsl =
                        (trail_setting < self.trail_levels_ppm.len()).then_some(trail_setting);
                    let cap = tsl.unwrap_or(self.trail_levels_ppm.len());
                    if !visit(Chosen {
                        stop,
                        target,
                        tsl,
                        ttp: None,
                    }) {
                        return false;
                    }
                    for arm in 0..target_setting {
                        for trail in 0..cap {
                            if !visit(Chosen {
                                stop,
                                target,
                                tsl,
                                ttp: Some(crate::grid::Ttp { arm, trail }),
                            }) {
                                return false;
                            }
                        }
                    }
                }
            }
        }
        true
    }

    fn ratio_pair_admitted(&self, stop_index: usize, target_index: usize) -> bool {
        stop_index
            .checked_mul(self.target_levels_ppm.len())
            .and_then(|row| row.checked_add(target_index))
            .and_then(|slot| self.ratio_bitmap.get(slot))
            .copied()
            .unwrap_or(false)
    }

    fn chosen_is_in_bounds(&self, chosen: Chosen) -> bool {
        if !self.chosen_axes_are_in_bounds(chosen) {
            return false;
        }
        let (Some(stop_index), Some(target_index)) = (chosen.stop, chosen.target) else {
            // Baseline/one-sided rows remain in the complete comparison grid,
            // but a policy that states reward-to-risk limits may not select or
            // replay a coordinate for which that ratio is undefined.
            return false;
        };
        self.ratio_pair_admitted(stop_index, target_index)
    }

    fn chosen_axes_are_in_bounds(&self, chosen: Chosen) -> bool {
        if chosen.stop.is_some_and(|i| i >= self.stop_levels_ppm.len())
            || chosen
                .target
                .is_some_and(|i| i >= self.target_levels_ppm.len())
            || chosen.tsl.is_some_and(|i| i >= self.trail_levels_ppm.len())
        {
            return false;
        }
        if let Some(ttp) = chosen.ttp
            && (ttp.arm >= self.target_levels_ppm.len()
                || ttp.trail >= self.trail_levels_ppm.len()
                || chosen.target.is_some_and(|target| ttp.arm >= target)
                || chosen.tsl.is_some_and(|tsl| ttp.trail >= tsl))
        {
            return false;
        }
        true
    }

    fn execution_refusal_bits(&self, cell: &Cell) -> ExecutionRefusalBitsV1 {
        let mut bits = ExecutionRefusalBitsV1::NONE;
        if cell.stop.is_none() {
            bits = bits.union(ExecutionRefusalBitsV1::MISSING_STOP);
        }
        if cell.target.is_none() {
            bits = bits.union(ExecutionRefusalBitsV1::MISSING_TARGET);
        }
        if cell.trades == 0 {
            bits = bits.union(ExecutionRefusalBitsV1::ZERO_TRADES);
        }
        if cell.ambiguous_bars > self.policy.max_ambiguous_bars() {
            bits = bits.union(ExecutionRefusalBitsV1::AMBIGUITY_LIMIT);
        }
        if cell.gapped > self.policy.max_gap_fills() {
            bits = bits.union(ExecutionRefusalBitsV1::GAP_LIMIT);
        }
        if matches!(
            self.policy.forced_stop(),
            ForcedStopV1::RequireExactObserved(_)
        ) && cell.stop != self.forced_stop_index
        {
            bits = bits.union(ExecutionRefusalBitsV1::FORCED_STOP_MISMATCH);
        }
        bits
    }
}

/// Every explicit refusal produced by V1 policy construction/resolution/replay.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExitGridErrorV1 {
    /// A rational percentile is zero, inverted, or above one.
    InvalidPercentile {
        /// Supplied numerator.
        numerator: u32,
        /// Supplied denominator.
        denominator: u32,
    },
    /// One named resource limit was zero.
    ZeroLimit(&'static str),
    /// One axis had no percentile schedule.
    EmptyPercentiles(&'static str),
    /// A percentile schedule exceeded its runtime-declared bound.
    PercentileLimitExceeded {
        /// Axis name.
        axis: &'static str,
        /// Supplied schedule length.
        supplied: usize,
        /// Policy ceiling.
        max: usize,
    },
    /// Rational percentiles were not strictly increasing.
    PercentilesNotIncreasing(&'static str),
    /// Reward-to-risk limits were non-positive or inverted.
    InvalidRatioLimits {
        /// Inclusive lower bound.
        min_hundredths: i64,
        /// Inclusive upper bound.
        max_hundredths: i64,
    },
    /// The caller supplied no real cost-model identity.
    MissingCostModelId,
    /// One required source/run identity term was empty or all zero.
    MissingSeriesIdentity(&'static str),
    /// One canonical run-identity term was absent.
    MissingRunIdentity(&'static str),
    /// A priced trade run cannot carry the frequency-only direction.
    UndirectedExecutionRun,
    /// The Run data term did not name the supplied signal/execution bytes.
    RunDataDigestMismatch,
    /// The three-stream daily-reference identity could not be formed.
    DailyReferenceIdentityRefused(crate::identity::DailyBindingRefusal),
    /// The requested execution span was not an exact contiguous byte slice of
    /// the minute context used by the daily-reference/GapFib computation.
    EvaluatedExecutionOutsideReferenceContext,
    /// A sealed run term did not match the execution series or selected side.
    RunIdentityMismatch(&'static str),
    /// V1 grid arithmetic implements a different cost/fill model ID.
    UnsupportedCostModelId,
    /// An exact forced stop was zero or negative.
    InvalidForcedStop(Ppm),
    /// The instrument is storable but outside the two-index sweep surface.
    UnsupportedInstrument,
    /// V1 supports only sixty-second execution bars.
    UnsupportedExecutionResolution(u32),
    /// An execution series offered for resolution or replay was empty.
    EmptyExecutionSeries,
    /// A timestamp was not on the exact minute grid.
    OffMinuteTimestamp {
        /// Index of the bad candle.
        index: usize,
        /// Supplied UTC microsecond timestamp.
        timestamp: i64,
    },
    /// Timestamps were duplicated or moved backward.
    NonIncreasingTimestamp {
        /// Index of the later bad candle.
        index: usize,
        /// Previous timestamp.
        previous: i64,
        /// Current timestamp.
        current: i64,
    },
    /// `Candle::check` refused one execution row.
    CorruptExecutionCandle {
        /// Index of the refused candle.
        index: usize,
    },
    /// PPM range encoding requires a strictly positive printed open.
    NonPositiveOpen {
        /// Index of the refused candle.
        index: usize,
        /// Supplied open in paisa.
        open: i64,
    },
    /// One printed OHLC field was zero or negative for a swept spot index.
    NonPositivePrice {
        /// Index of the refused candle.
        index: usize,
    },
    /// Open interest was neither the null sentinel nor non-negative.
    InvalidOpenInterest {
        /// Index of the refused candle.
        index: usize,
        /// Supplied value.
        value: i64,
    },
    /// Checked integer arithmetic could not represent a result.
    ArithmeticOverflow(&'static str),
    /// The exact-grid allocation could not be reserved without aborting.
    AllocationRefused {
        /// Requested exact cell capacity.
        requested_cells: u64,
    },
    /// A non-cell bounded buffer could not be reserved without aborting.
    BufferAllocationRefused {
        /// Named buffer.
        buffer: &'static str,
        /// Requested element count.
        elements: usize,
    },
    /// Valid arithmetic could still overflow a grid accumulator on this slice.
    ArithmeticEnvelopeExceeded(&'static str),
    /// Every checked TRAINING range for one named axis encoded to zero.
    NoPositiveObservedRange(&'static str),
    /// A rational axis unexpectedly resolved to no positive rung.
    EmptyResolvedAxis(&'static str),
    /// The declared percentile selected a zero-distance observation.
    ZeroResolvedRung(&'static str),
    /// Forced stop was not an exact member of the observed TRAINING set.
    ForcedStopNotObserved(Ppm),
    /// Resolved rung counts exceeded the caller's declared axis bound.
    ResolvedRungLimitExceeded {
        /// Stop rung count.
        stop: usize,
        /// Target rung count.
        target: usize,
        /// Trail rung count.
        trail: usize,
        /// Maximum permitted on each axis.
        max: usize,
    },
    /// No exact stop-target pair cleared the ratio interval.
    NoAdmittedRatioPair,
    /// Exact ratio-coordinate count breached its policy ceiling.
    RatioPairLimitExceeded {
        /// Needed exact pairs.
        needed: u64,
        /// Policy maximum.
        max: u64,
    },
    /// Exact ratio-filtered grid width breached its policy ceiling.
    CellLimitExceeded {
        /// Needed cells.
        needed: u64,
        /// Policy maximum.
        max: u64,
    },
    /// Persisted/in-memory exact ladder bytes were malformed.
    InvalidResolvedLadder(&'static str),
    /// Carried policy/resolution fields no longer match their digest.
    ResolutionDigestMismatch,
    /// Training-grid enumeration was offered different bytes than resolution.
    TrainingSeriesMismatch,
    /// Instrument/feed/commit/calendar attestation differs from resolution.
    SeriesIdentityMismatch(&'static str),
    /// The column carried no evaluator policy identity.
    MissingEvaluationSpec,
    /// One or more execution bars lacked a complete accepted verdict.
    IncompleteExecutionAcceptance {
        /// Bars expected.
        bars: usize,
        /// Refused or missing verdicts.
        refused_or_missing: u64,
    },
    /// A column's masks and source-coordinate vectors were not parallel.
    ColumnShapeMismatch {
        /// Number of mask rows.
        bits: usize,
        /// Number of source coordinates.
        sources: usize,
    },
    /// A column source index lies outside its execution slice.
    ColumnSourceOutOfRange {
        /// Bad source index.
        source: usize,
        /// Execution slice length.
        bars: usize,
    },
    /// A warm-up/training signal was still able to enter the OOS replay.
    ColumnSourceBeforeOos {
        /// Bad source index.
        source: usize,
        /// First permitted OOS source.
        first_oos: usize,
    },
    /// Source coordinates were duplicated or moved backward.
    ColumnSourcesNotIncreasing {
        /// Previous source coordinate.
        previous: usize,
        /// Current source coordinate.
        current: usize,
    },
    /// The explicit OOS split did not index a bar.
    InvalidOosBoundary {
        /// Supplied first OOS index.
        first_oos: usize,
        /// Slice length.
        bars: usize,
    },
    /// The first OOS bar was not strictly later than all training bars.
    OosOverlapsTraining {
        /// Last training timestamp.
        training_last: i64,
        /// First OOS timestamp.
        first_oos: i64,
    },
    /// OOS evaluator choices differ from the training column.
    EvaluationSpecMismatch,
    /// Grid evaluation/replay was offered the opposite trade direction.
    SideMismatch {
        /// Side whose TRAINING ranges resolved the grid.
        resolved: crate::excursion::Side,
        /// Side the caller asked to price.
        offered: crate::excursion::Side,
    },
    /// An evaluated-grid capability belongs to another resolution or side.
    EvaluationResolutionMismatch,
    /// The opaque selected capability was changed or belongs elsewhere.
    SelectionDigestMismatch,
    /// A grid offered for selection used different rung values.
    ReplayLadderMismatch,
    /// A grid omitted policy cells or carried cells outside the exact policy.
    ReplayCellCountMismatch {
        /// Exact policy-resolved cell population.
        expected: u64,
        /// Cells offered by the caller.
        actual: u64,
    },
    /// Cell count matched, but a coordinate was missing, duplicated or reordered.
    ReplayCoordinatePopulationMismatch,
    /// At least one candidate execution path was refused; no clean subset wins.
    RefusedExecutionPaths {
        /// Refused path count.
        paths: u64,
    },
    /// The complete replay-evidence population could not reserve its bounded
    /// outer candidate storage.
    ReplayEvidenceAllocationRefused {
        /// Number of ordered candidate paths requiring storage.
        requested_paths: u64,
    },
    /// Candidate timing, ordering, or chosen-row evidence failed an exact
    /// reconciliation check.
    ReplayEvidenceRefused(&'static str),
    /// Carried candidate-universe fields no longer match their opaque digest.
    ReplayEvidenceDigestMismatch,
    /// A replay coordinate was out of bounds or excluded by policy.
    InvalidChosenCoordinate,
    /// The requested exact coordinate existed, but its measured training cell
    /// failed the policy's trade, ambiguity, opening-gap or forced-stop gate.
    ChosenCellNotAdmitted,
    /// Replayed OOS cell exceeded ambiguity or opening-gap limits.
    ReplayQualityLimitExceeded {
        /// Measured ambiguous bars.
        ambiguous_bars: u64,
        /// Policy ceiling.
        max_ambiguous_bars: u64,
        /// Measured opening-gap fills.
        gap_fills: u64,
        /// Policy ceiling.
        max_gap_fills: u64,
    },
}

impl core::fmt::Display for ExitGridErrorV1 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "exit-grid V1 refusal: {self:?}")
    }
}

impl std::error::Error for ExitGridErrorV1 {}

fn validate_percentiles(
    axis: &'static str,
    values: &[RationalPercentileV1],
    max: usize,
) -> Result<(), ExitGridErrorV1> {
    if values.is_empty() {
        return Err(ExitGridErrorV1::EmptyPercentiles(axis));
    }
    if values.len() > max {
        return Err(ExitGridErrorV1::PercentileLimitExceeded {
            axis,
            supplied: values.len(),
            max,
        });
    }
    if values.windows(2).any(|pair| {
        let Some(left) = pair.first() else {
            return true;
        };
        let Some(right) = pair.last() else {
            return true;
        };
        u64::from(left.numerator) * u64::from(right.denominator)
            >= u64::from(right.numerator) * u64::from(left.denominator)
    }) {
        return Err(ExitGridErrorV1::PercentilesNotIncreasing(axis));
    }
    Ok(())
}

struct ObservedRanges {
    stops: Vec<Ppm>,
    targets: Vec<Ppm>,
    trails: Vec<Ppm>,
    forced_stop_observed: bool,
}

const fn forced_stop_level(forced_stop: ForcedStopV1) -> Option<Ppm> {
    match forced_stop {
        ForcedStopV1::Disabled => None,
        ForcedStopV1::IncludeExactObserved(ppm) | ForcedStopV1::RequireExactObserved(ppm) => {
            Some(ppm)
        }
    }
}

fn validate_observed_ranges(
    observed: &ObservedRanges,
    forced_stop: Option<Ppm>,
) -> Result<(), ExitGridErrorV1> {
    for (axis, values) in [
        ("stop", observed.stops.as_slice()),
        ("target", observed.targets.as_slice()),
        ("trail", observed.trails.as_slice()),
    ] {
        if values.iter().all(|value| *value == 0) {
            return Err(ExitGridErrorV1::NoPositiveObservedRange(axis));
        }
    }
    if let Some(level) = forced_stop
        && !observed.forced_stop_observed
    {
        return Err(ExitGridErrorV1::ForcedStopNotObserved(level));
    }
    Ok(())
}

fn observed_ranges(
    bars: &[Candle],
    resolution: RangeResolutionV1,
    side: crate::excursion::Side,
    forced_stop: Option<Ppm>,
) -> Result<ObservedRanges, ExitGridErrorV1> {
    validate_execution_bars(bars)?;
    let mut out = ObservedRanges {
        stops: Vec::new(),
        targets: Vec::new(),
        trails: Vec::new(),
        forced_stop_observed: false,
    };
    for (name, values) in [
        ("observed stops", &mut out.stops),
        ("observed targets", &mut out.targets),
        ("observed trails", &mut out.trails),
    ] {
        values.try_reserve_exact(bars.len()).map_err(|_| {
            ExitGridErrorV1::BufferAllocationRefused {
                buffer: name,
                elements: bars.len(),
            }
        })?;
    }
    for bar in bars {
        // The product owns no position at or after 15:10 IST. Those rows may
        // remain in the attested source bytes (and therefore in its digest),
        // but allowing their ranges to choose an exit rung would calibrate a
        // tradable policy from prices no strategy can ever reach.
        if ist_minute_of(bar.ts_micros)? >= FORCED_EXIT_MINUTE {
            continue;
        }
        let full_span =
            bar.high
                .checked_sub(bar.low)
                .ok_or(ExitGridErrorV1::ArithmeticOverflow(
                    "printed high-low range",
                ))?;
        let down = bar
            .open
            .checked_sub(bar.low)
            .ok_or(ExitGridErrorV1::ArithmeticOverflow(
                "printed open-low range",
            ))?;
        let up = bar
            .high
            .checked_sub(bar.open)
            .ok_or(ExitGridErrorV1::ArithmeticOverflow(
                "printed high-open range",
            ))?;
        let (stop_span, target_span) = match side {
            crate::excursion::Side::Long => (down, up),
            crate::excursion::Side::Short => (up, down),
        };
        let stop = encode_observed(stop_span, bar.open, resolution, "stop")?;
        out.forced_stop_observed |= forced_stop == Some(stop);
        out.stops.push(stop);
        out.targets.push(encode_observed(
            target_span,
            bar.open,
            resolution,
            "target",
        )?);
        out.trails
            .push(encode_observed(full_span, bar.open, resolution, "trail")?);
    }
    Ok(out)
}

fn encode_observed(
    span: i64,
    open: i64,
    resolution: RangeResolutionV1,
    axis: &'static str,
) -> Result<Ppm, ExitGridErrorV1> {
    let scaled = i128::from(span)
        .checked_mul(PPM_ONE)
        .ok_or(ExitGridErrorV1::ArithmeticOverflow("range PPM numerator"))?;
    let denominator = i128::from(open);
    let floor = scaled / denominator;
    let encoded =
        match resolution {
            RangeResolutionV1::PpmFloor => floor,
            RangeResolutionV1::PpmCeiling => floor
                .checked_add(i128::from(scaled % denominator != 0))
                .ok_or(ExitGridErrorV1::ArithmeticOverflow("ceiling range PPM"))?,
        };
    let ppm = i64::try_from(encoded).map_err(|_| ExitGridErrorV1::ArithmeticOverflow(axis))?;
    // Zero is a real observation. Removing it would condition the percentile
    // on movement and turn nine flat bars plus one move into a median equal to
    // that lone move. Resolution below refuses when a requested rung lands on
    // zero; it never silently promotes the next positive sample.
    Ok(ppm)
}

fn validate_execution_bars(bars: &[Candle]) -> Result<(), ExitGridErrorV1> {
    if bars.is_empty() {
        return Err(ExitGridErrorV1::EmptyExecutionSeries);
    }
    let mut previous: Option<i64> = None;
    for (index, bar) in bars.iter().enumerate() {
        if bar.ts_micros.rem_euclid(ONE_MINUTE_MICROS) != 0 {
            return Err(ExitGridErrorV1::OffMinuteTimestamp {
                index,
                timestamp: bar.ts_micros,
            });
        }
        if let Some(before) = previous
            && bar.ts_micros <= before
        {
            return Err(ExitGridErrorV1::NonIncreasingTimestamp {
                index,
                previous: before,
                current: bar.ts_micros,
            });
        }
        previous = Some(bar.ts_micros);
        if bar.check().is_err() {
            return Err(ExitGridErrorV1::CorruptExecutionCandle { index });
        }
        if bar.open <= 0 {
            return Err(ExitGridErrorV1::NonPositiveOpen {
                index,
                open: bar.open,
            });
        }
        if bar.high <= 0 || bar.low <= 0 || bar.close <= 0 {
            return Err(ExitGridErrorV1::NonPositivePrice { index });
        }
        if bar.open_interest != indicators::OI_NULL && bar.open_interest < 0 {
            return Err(ExitGridErrorV1::InvalidOpenInterest {
                index,
                value: bar.open_interest,
            });
        }
    }
    Ok(())
}

fn ist_minute_of(timestamp: i64) -> Result<i64, ExitGridErrorV1> {
    Ok(timestamp
        .checked_add(indicators::IST_OFFSET_MICROS)
        .ok_or(ExitGridErrorV1::ArithmeticOverflow("IST timestamp"))?
        .rem_euclid(DAY_MICROS)
        / ONE_MINUTE_MICROS)
}

fn resolve_axis(
    axis: &'static str,
    observed: &[Ppm],
    percentiles: &[RationalPercentileV1],
) -> Result<Vec<Ppm>, ExitGridErrorV1> {
    resolve_axis_with_forced(axis, observed, percentiles, None).map(|(levels, _)| levels)
}

/// Resolves one ordered axis while placing an already-observed forced stop in
/// the same pass. The forced level's insertion ordinal is accumulated while
/// the percentile members are emitted; resolution never performs a second
/// search or scan over either source or resolved levels.
fn resolve_axis_with_forced(
    axis: &'static str,
    observed: &[Ppm],
    percentiles: &[RationalPercentileV1],
    forced: Option<Ppm>,
) -> Result<(Vec<Ppm>, Option<usize>), ExitGridErrorV1> {
    let count = u128::try_from(observed.len())
        .map_err(|_| ExitGridErrorV1::ArithmeticOverflow("observed range count"))?;
    let mut out = Vec::with_capacity(percentiles.len());
    let mut forced_index = None;
    let mut forced_insertion = 0_usize;
    for percentile in percentiles {
        let numerator = count
            .checked_mul(u128::from(percentile.numerator()))
            .ok_or(ExitGridErrorV1::ArithmeticOverflow("percentile numerator"))?;
        let denominator = u128::from(percentile.denominator());
        let rank = numerator
            .checked_add(denominator.saturating_sub(1))
            .ok_or(ExitGridErrorV1::ArithmeticOverflow("percentile ceiling"))?
            / denominator;
        let zero_based = rank.saturating_sub(1);
        let index = usize::try_from(zero_based)
            .map_err(|_| ExitGridErrorV1::ArithmeticOverflow("percentile index"))?;
        let value = observed
            .get(index)
            .copied()
            .ok_or(ExitGridErrorV1::ArithmeticOverflow("percentile lookup"))?;
        if value <= 0 {
            return Err(ExitGridErrorV1::ZeroResolvedRung(axis));
        }
        if out.last() != Some(&value) {
            if forced.is_some_and(|level| value < level) {
                forced_insertion =
                    out.len()
                        .checked_add(1)
                        .ok_or(ExitGridErrorV1::ArithmeticOverflow(
                            "forced stop insertion index",
                        ))?;
            } else if forced == Some(value) {
                forced_index = Some(out.len());
            }
            out.push(value);
        }
    }
    if out.is_empty() {
        return Err(ExitGridErrorV1::EmptyResolvedAxis(axis));
    }
    if let Some(level) = forced
        && forced_index.is_none()
    {
        out.insert(forced_insertion, level);
        forced_index = Some(forced_insertion);
    }
    Ok((out, forced_index))
}

fn admitted_ratio_pairs(
    stops: &[Ppm],
    targets: &[Ppm],
    limits: RatioLimitsV1,
) -> Result<Vec<RatioPairV1>, ExitGridErrorV1> {
    let mut out = Vec::new();
    let possible =
        stops
            .len()
            .checked_mul(targets.len())
            .ok_or(ExitGridErrorV1::ArithmeticOverflow(
                "ratio pair coordinate product",
            ))?;
    let policy_cap = usize::try_from(limits.max_pairs()).unwrap_or(usize::MAX);
    let reserve = possible.min(policy_cap);
    out.try_reserve_exact(reserve)
        .map_err(|_| ExitGridErrorV1::BufferAllocationRefused {
            buffer: "admitted ratio pairs",
            elements: reserve,
        })?;
    for (stop_index, stop) in stops.iter().copied().enumerate() {
        for (target_index, target) in targets.iter().copied().enumerate() {
            let scaled =
                i128::from(target)
                    .checked_mul(100)
                    .ok_or(ExitGridErrorV1::ArithmeticOverflow(
                        "reward-to-risk numerator",
                    ))?;
            let minimum = i128::from(stop)
                .checked_mul(i128::from(limits.min_hundredths()))
                .ok_or(ExitGridErrorV1::ArithmeticOverflow(
                    "minimum reward-to-risk cross product",
                ))?;
            let maximum = i128::from(stop)
                .checked_mul(i128::from(limits.max_hundredths()))
                .ok_or(ExitGridErrorV1::ArithmeticOverflow(
                    "maximum reward-to-risk cross product",
                ))?;
            if scaled >= minimum && scaled <= maximum {
                if u64::try_from(out.len()).unwrap_or(u64::MAX) >= limits.max_pairs() {
                    return Err(ExitGridErrorV1::RatioPairLimitExceeded {
                        needed: limits.max_pairs().saturating_add(1),
                        max: limits.max_pairs(),
                    });
                }
                out.push(RatioPairV1 {
                    stop_index,
                    target_index,
                });
            }
        }
    }
    Ok(out)
}

fn ratio_bitmap_of(
    stop_count: usize,
    target_count: usize,
    pairs: &[RatioPairV1],
) -> Result<Vec<bool>, ExitGridErrorV1> {
    let length = stop_count
        .checked_mul(target_count)
        .ok_or(ExitGridErrorV1::ArithmeticOverflow("ratio bitmap length"))?;
    let mut bitmap = Vec::new();
    bitmap
        .try_reserve_exact(length)
        .map_err(|_| ExitGridErrorV1::BufferAllocationRefused {
            buffer: "ratio admission bitmap",
            elements: length,
        })?;
    bitmap.resize(length, false);
    for pair in pairs {
        let slot = pair
            .stop_index
            .checked_mul(target_count)
            .and_then(|row| row.checked_add(pair.target_index))
            .ok_or(ExitGridErrorV1::ArithmeticOverflow("ratio bitmap slot"))?;
        let value = bitmap
            .get_mut(slot)
            .ok_or(ExitGridErrorV1::ArithmeticOverflow(
                "ratio bitmap coordinate",
            ))?;
        *value = true;
    }
    Ok(bitmap)
}

fn checked_sum_below(limit: usize) -> Result<usize, ExitGridErrorV1> {
    let limit = u128::try_from(limit)
        .map_err(|_| ExitGridErrorV1::ArithmeticOverflow("coordinate triangular limit"))?;
    let sum = limit
        .checked_mul(limit.saturating_sub(1))
        .and_then(|product| product.checked_div(2))
        .ok_or(ExitGridErrorV1::ArithmeticOverflow(
            "coordinate triangular number",
        ))?;
    usize::try_from(sum)
        .map_err(|_| ExitGridErrorV1::ArithmeticOverflow("coordinate triangular number"))
}

fn coordinate_row_width(target_setting: usize, trails: usize) -> Result<usize, ExitGridErrorV1> {
    let trail_settings = trails
        .checked_add(1)
        .ok_or(ExitGridErrorV1::ArithmeticOverflow(
            "coordinate trail settings",
        ))?;
    let armed_trails = checked_sum_below(trail_settings)?;
    trail_settings
        .checked_add(target_setting.checked_mul(armed_trails).ok_or(
            ExitGridErrorV1::ArithmeticOverflow("coordinate row armed trails"),
        )?)
        .ok_or(ExitGridErrorV1::ArithmeticOverflow("coordinate row width"))
}

fn checked_policy_cell_count(
    stops: usize,
    targets: usize,
    trails: usize,
    ratio_pairs: &[RatioPairV1],
) -> Option<u64> {
    let s = u128::try_from(stops).ok()?;
    let t = u128::try_from(targets).ok()?;
    let r = u128::try_from(trails).ok()?;
    let trail_settings = r.checked_add(1)?;
    let armed_trails = r.checked_mul(trail_settings)?.checked_div(2)?;

    // Every stop setting, including no stop, has the no-target row. On that
    // row every target rung may arm a TTP.
    let no_target_row = trail_settings.checked_add(t.checked_mul(armed_trails)?)?;
    let no_target_total = s.checked_add(1)?.checked_mul(no_target_row)?;

    // With no stop every target row remains valid. A target row at index `t`
    // has `t` arming rungs because an arm at/above the target is unreachable.
    let target_indices = t.checked_mul(t.saturating_sub(1))?.checked_div(2)?;
    let target_rows_without_stop = t
        .checked_mul(trail_settings)?
        .checked_add(target_indices.checked_mul(armed_trails)?)?;

    // With a stop, only exact admitted stop-target coordinates survive.
    let mut admitted_rows = 0_u128;
    for pair in ratio_pairs {
        let arm_count = u128::try_from(pair.target_index).ok()?;
        admitted_rows = admitted_rows
            .checked_add(trail_settings)?
            .checked_add(arm_count.checked_mul(armed_trails)?)?;
    }
    let cells = no_target_total
        .checked_add(target_rows_without_stop)?
        .checked_add(admitted_rows)?;
    u64::try_from(cells).ok()
}

fn digest_resolved(value: &ResolvedExitGridV1) -> [u8; 32] {
    let mut h = Hasher::new();
    h.update(b"brutex.runner.resolved-exit-grid.v1");
    put_u16(&mut h, EXIT_GRID_POLICY_VERSION_V1);
    put_instrument(&mut h, &value.instrument);
    h.update(&[value.family.byte()]);
    h.update(&value.feed_digest);
    h.update(&value.commit_digest);
    h.update(&value.calendar_digest);
    // Both are load-bearing. `policy.digest()` binds the policy value itself;
    // `policy_digest` binds the separately persisted cross-check field. A torn
    // row in which only one changed must not verify.
    h.update(&value.policy.digest());
    h.update(&value.policy_digest);
    h.update(&value.training_digest);
    put_u64(&mut h, value.training_bars);
    put_i64(&mut h, value.training_first_ts_micros);
    put_i64(&mut h, value.training_last_ts_micros);
    put_levels(&mut h, &value.stop_levels_ppm);
    put_levels(&mut h, &value.target_levels_ppm);
    put_levels(&mut h, &value.trail_levels_ppm);
    put_usize(&mut h, value.ratio_pairs.len());
    for pair in &value.ratio_pairs {
        put_usize(&mut h, pair.stop_index);
        put_usize(&mut h, pair.target_index);
    }
    put_usize(&mut h, value.ratio_bitmap.len());
    for admitted in &value.ratio_bitmap {
        h.update(&[u8::from(*admitted)]);
    }
    match value.forced_stop_index {
        None => h.update(&[0]),
        Some(index) => {
            h.update(&[1]);
            put_usize(&mut h, index);
        }
    }
    put_u64(&mut h, value.cell_count);
    h.finalize()
}

const EXECUTION_DISPOSITION_AUTHORIZED: u8 = 1;
const EXECUTION_DISPOSITION_POLICY_REFUSED: u8 = 2;

fn digest_evaluated_exit_grid(evaluated: &EvaluatedExitGridV1) -> [u8; 32] {
    let mut h = Hasher::new();
    h.update(b"brutex.runner.evaluated-exit-grid.v1");
    h.update(&evaluated.resolution_digest);
    h.update(&evaluated.run_id.bytes());
    put_mask(&mut h, evaluated.mask);
    put_u32(&mut h, evaluated.horizon.as_bars());
    h.update(&evaluated.column_digest);
    h.update(evaluated.evaluation_spec.fingerprint_v1().as_bytes());
    put_side(&mut h, evaluated.side);
    put_u64(&mut h, evaluated.grid.signals);
    put_levels(&mut h, evaluated.grid.stops.rungs());
    put_levels(&mut h, evaluated.grid.targets.rungs());
    put_levels(&mut h, evaluated.grid.trails.rungs());
    put_u64(&mut h, evaluated.grid.refused_paths);
    put_usize(&mut h, evaluated.grid.cells.len());
    for cell in &evaluated.grid.cells {
        put_cell(&mut h, cell);
    }
    h.finalize()
}

fn digest_execution_disposition_context(
    evaluation_digest: [u8; 32],
    evaluated: &EvaluatedExitGridV1,
    coordinate: Chosen,
    cell: &Cell,
) -> [u8; 32] {
    let mut h = Hasher::new();
    h.update(b"brutex.runner.execution-disposition-context.v1");
    h.update(&evaluation_digest);
    h.update(&evaluated.resolution_digest);
    h.update(&evaluated.run_id.bytes());
    put_mask(&mut h, evaluated.mask);
    put_u32(&mut h, evaluated.horizon.as_bars());
    h.update(&evaluated.column_digest);
    h.update(evaluated.evaluation_spec.fingerprint_v1().as_bytes());
    put_side(&mut h, evaluated.side);
    put_chosen(&mut h, coordinate);
    put_cell(&mut h, cell);
    h.finalize()
}

fn digest_execution_disposition(
    context_digest: [u8; 32],
    terminal_tag: u8,
    refusal_bits: ExecutionRefusalBitsV1,
    selected_digest: Option<[u8; 32]>,
) -> [u8; 32] {
    debug_assert!(matches!(
        (terminal_tag, selected_digest, refusal_bits.is_empty()),
        (EXECUTION_DISPOSITION_AUTHORIZED, Some(_), true)
            | (EXECUTION_DISPOSITION_POLICY_REFUSED, None, false)
    ));
    let mut h = Hasher::new();
    h.update(b"brutex.runner.execution-disposition.v1");
    h.update(&context_digest);
    h.update(&[terminal_tag]);
    put_u64(&mut h, refusal_bits.bits());
    h.update(&selected_digest.unwrap_or([0; 32]));
    h.finalize()
}

#[expect(
    clippy::too_many_arguments,
    reason = "the selection seal deliberately binds nine independent identity and result fields without hiding one behind a default"
)]
fn digest_selected(
    resolution_digest: [u8; 32],
    run_id: RunId,
    mask: ConditionMask,
    horizon: Horizon,
    column_digest: [u8; 32],
    evaluation_spec: EvaluationSpecToken,
    side: crate::excursion::Side,
    coordinate: Chosen,
    training_cell: &Cell,
) -> [u8; 32] {
    let mut h = Hasher::new();
    h.update(b"brutex.runner.selected-exit.v1");
    h.update(&resolution_digest);
    h.update(&run_id.bytes());
    put_mask(&mut h, mask);
    put_u32(&mut h, horizon.as_bars());
    h.update(&column_digest);
    h.update(evaluation_spec.fingerprint_v1().as_bytes());
    put_side(&mut h, side);
    put_chosen(&mut h, coordinate);
    put_cell(&mut h, training_cell);
    h.finalize()
}

fn digest_replay(
    run_id: RunId,
    selected_digest: [u8; 32],
    oos_data_digest: [u8; 32],
    column_digest: [u8; 32],
    cell: Option<&Cell>,
) -> [u8; 32] {
    let mut h = Hasher::new();
    h.update(b"brutex.runner.replayed-exit.v1");
    h.update(&run_id.bytes());
    h.update(&selected_digest);
    h.update(&oos_data_digest);
    h.update(&column_digest);
    match cell {
        None => h.update(&[0]),
        Some(value) => {
            h.update(&[1]);
            put_cell(&mut h, value);
        }
    }
    h.finalize()
}

fn digest_replay_universe(
    run_id: RunId,
    selected_digest: [u8; 32],
    oos_data_digest: [u8; 32],
    column_digest: [u8; 32],
    local_cell: Option<&Cell>,
    candidates: &[crate::grid::ReplayCandidateV1],
    pricing_refused_paths: u64,
) -> [u8; 32] {
    let mut h = Hasher::new();
    h.update(b"brutex.runner.replayed-candidate-universe.v1");
    h.update(&run_id.bytes());
    h.update(&selected_digest);
    h.update(&oos_data_digest);
    h.update(&column_digest);
    match local_cell {
        None => h.update(&[0]),
        Some(value) => {
            h.update(&[1]);
            put_cell(&mut h, value);
        }
    }
    put_u64(&mut h, pricing_refused_paths);
    put_usize(&mut h, candidates.len());
    for candidate in candidates {
        put_usize(&mut h, candidate.signal_bar);
        put_usize(&mut h, candidate.entry_bar);
        put_usize(&mut h, candidate.occupied_through_bar);
        put_i64(&mut h, candidate.signal_micros);
        put_i64(&mut h, candidate.entry_micros);
        put_i64(&mut h, candidate.occupied_through_micros);
        match candidate.path {
            crate::grid::ReplayPathV1::Priceable(price) => {
                h.update(&[1]);
                put_trade_row(&mut h, price.row);
                put_u64(&mut h, price.ambiguous_bars);
                put_u64(&mut h, price.gap_fills);
            }
            crate::grid::ReplayPathV1::BlockOnly => h.update(&[2]),
            crate::grid::ReplayPathV1::CrossingRefused => h.update(&[3]),
            crate::grid::ReplayPathV1::BlockOnlyAndCrossingRefused => h.update(&[4]),
        }
    }
    h.finalize()
}

fn digest_global_replay_witness(
    instrument: &InstrumentKey,
    feed: Vendor,
    direction: CostDirection,
    first_oos: usize,
    universe: &ReplayedCandidateUniverseV1,
) -> [u8; 32] {
    let mut h = Hasher::new();
    h.update(b"brutex.runner.global-replay-witness-universe.v1\0");
    h.update(&instrument_digest_v1(instrument));
    put_bytes(&mut h, feed.as_str().as_bytes());
    h.update(&[match direction {
        CostDirection::Long => 1,
        CostDirection::Short => 2,
    }]);
    put_usize(&mut h, first_oos);
    h.update(&universe.run_id.bytes());
    h.update(&universe.selected_digest);
    h.update(&universe.digest);
    h.finalize()
}

/// Stable V1 digest of every durable field in one indicator/execution column.
///
/// This is the sole codec used by grid evaluation and pre-admission durable
/// adapters. It is O(column length), so callers compute it once at a structural
/// boundary and retain the resulting fixed-size identity.
#[must_use]
pub fn column_digest_v1(column: &indicators::column::Column) -> [u8; 32] {
    let mut h = Hasher::new();
    h.update(b"brutex.indicators.execution-column.v1");
    put_usize(&mut h, column.bits().len());
    for mask in column.bits() {
        put_mask(&mut h, *mask);
    }
    put_usize(&mut h, column.sources().len());
    for source in column.sources() {
        put_usize(&mut h, *source);
    }
    h.update(&[match column.sourced() {
        indicators::column::Sourced::Signal => 1,
        indicators::column::Sourced::Fill => 2,
    }]);
    put_census(&mut h, column.census());
    put_option_usize(&mut h, column.first_swept());
    put_u64(&mut h, column.collided());
    match column.evaluation_spec_token() {
        None => h.update(&[0]),
        Some(token) => {
            h.update(&[1]);
            h.update(token.fingerprint_v1().as_bytes());
        }
    }
    match column.acceptance() {
        None => h.update(&[0]),
        Some(accepted) => {
            h.update(&[1]);
            put_usize(&mut h, accepted.len());
            for verdict in accepted.iter() {
                h.update(&[u8::from(*verdict)]);
            }
        }
    }
    put_census(&mut h, column.acceptance_census());
    h.finalize()
}

fn digest_column(column: &indicators::column::Column) -> [u8; 32] {
    column_digest_v1(column)
}

fn require_complete_acceptance(
    column: &indicators::column::Column,
    bars: usize,
) -> Result<(), ExitGridErrorV1> {
    let Some(accepted) = column.acceptance() else {
        return Err(ExitGridErrorV1::IncompleteExecutionAcceptance {
            bars,
            refused_or_missing: u64::try_from(bars).unwrap_or(u64::MAX),
        });
    };
    let false_count = accepted.iter().filter(|verdict| !**verdict).count();
    let missing = bars.abs_diff(accepted.len());
    let refused_or_missing = u64::try_from(false_count.saturating_add(missing)).unwrap_or(u64::MAX);
    let census = column.acceptance_census();
    if !column.acceptance_covers(bars)
        || refused_or_missing != 0
        || !census.reconciles()
        || census.offered != u64::try_from(bars).unwrap_or(u64::MAX)
        || census.refused() != 0
    {
        return Err(ExitGridErrorV1::IncompleteExecutionAcceptance {
            bars,
            refused_or_missing: refused_or_missing.max(census.refused()),
        });
    }
    Ok(())
}

fn validate_column_sources(
    column: &indicators::column::Column,
    bars: usize,
    first_permitted: usize,
) -> Result<(), ExitGridErrorV1> {
    if column.bits().len() != column.sources().len() {
        return Err(ExitGridErrorV1::ColumnShapeMismatch {
            bits: column.bits().len(),
            sources: column.sources().len(),
        });
    }
    let mut previous = None;
    for source in column.sources().iter().copied() {
        if source >= bars {
            return Err(ExitGridErrorV1::ColumnSourceOutOfRange { source, bars });
        }
        if source < first_permitted {
            return Err(ExitGridErrorV1::ColumnSourceBeforeOos {
                source,
                first_oos: first_permitted,
            });
        }
        if previous.is_some_and(|before| source <= before) {
            return Err(ExitGridErrorV1::ColumnSourcesNotIncreasing {
                previous: previous.unwrap_or(source),
                current: source,
            });
        }
        previous = Some(source);
    }
    Ok(())
}

fn validate_arithmetic_envelope(
    bars: &[Candle],
    resolved: &ResolvedExitGridV1,
) -> Result<(), ExitGridErrorV1> {
    let count = i128::try_from(bars.len())
        .map_err(|_| ExitGridErrorV1::ArithmeticEnvelopeExceeded("bar count"))?;
    let max_price = bars
        .iter()
        .map(|bar| bar.high)
        .max()
        .ok_or(ExitGridErrorV1::EmptyExecutionSeries)?;
    let min_open = bars
        .iter()
        .map(|bar| bar.open)
        .min()
        .ok_or(ExitGridErrorV1::EmptyExecutionSeries)?;
    let money_bound = i128::from(max_price)
        .checked_mul(count)
        .and_then(|value| value.checked_mul(4))
        .ok_or(ExitGridErrorV1::ArithmeticEnvelopeExceeded(
            "aggregate paisa product",
        ))?;
    if money_bound > i128::from(i64::MAX) {
        return Err(ExitGridErrorV1::ArithmeticEnvelopeExceeded(
            "aggregate paisa accumulator",
        ));
    }

    let max_observed_ppm = i128::from(max_price).checked_mul(PPM_ONE).ok_or(
        ExitGridErrorV1::ArithmeticEnvelopeExceeded("excursion PPM numerator"),
    )? / i128::from(min_open);
    let excursion_bound =
        max_observed_ppm
            .checked_mul(count)
            .ok_or(ExitGridErrorV1::ArithmeticEnvelopeExceeded(
                "excursion accumulator product",
            ))?;
    if excursion_bound > i128::from(i64::MAX) {
        return Err(ExitGridErrorV1::ArithmeticEnvelopeExceeded(
            "excursion accumulator",
        ));
    }

    let max_rung = resolved
        .stop_levels_ppm
        .iter()
        .chain(resolved.target_levels_ppm.iter())
        .chain(resolved.trail_levels_ppm.iter())
        .copied()
        .max()
        .ok_or(ExitGridErrorV1::ArithmeticEnvelopeExceeded(
            "resolved rung set",
        ))?;
    let distance = i128::from(max_rung)
        .checked_mul(i128::from(max_price))
        .ok_or(ExitGridErrorV1::ArithmeticEnvelopeExceeded(
            "resolved level product",
        ))?
        / PPM_ONE;
    if distance > i128::from(i64::MAX)
        || i128::from(max_price).checked_add(distance) > Some(i128::from(i64::MAX))
    {
        return Err(ExitGridErrorV1::ArithmeticEnvelopeExceeded(
            "resolved level price",
        ));
    }
    Ok(())
}

#[cfg(test)]
fn test_execution_run(
    instrument: &InstrumentKey,
    mask: &ConditionMask,
    side: crate::excursion::Side,
    bars: &[Candle],
) -> Result<ExecutionRunV1, ExitGridErrorV1> {
    test_execution_run_for_feed(instrument, mask, side, bars, "test-feed")
}

#[cfg(test)]
fn test_execution_run_for_feed(
    instrument: &InstrumentKey,
    mask: &ConditionMask,
    side: crate::excursion::Side,
    bars: &[Candle],
    feed: &str,
) -> Result<ExecutionRunV1, ExitGridErrorV1> {
    let direction = match side {
        crate::excursion::Side::Long => crate::identity::Direction::Long,
        crate::excursion::Side::Short => crate::identity::Direction::Short,
    };
    ExecutionRunV1::new(
        &crate::identity::Run {
            mask: *mask,
            direction,
            instrument,
            timeframe: "1min",
            params: crate::identity::Params {
                min_hits: 1,
                ceiling: 1,
                pair_budget: 1,
                policy: 0,
            },
            data_digest: crate::identity::data_digest(bars),
            commit: "test-commit",
            feed,
        },
        bars,
        None,
    )
}

fn put_levels(h: &mut Hasher, levels: &[Ppm]) {
    put_usize(h, levels.len());
    for level in levels {
        put_i64(h, *level);
    }
}

const fn gcd_u32(mut left: u32, mut right: u32) -> u32 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left
}

fn put_instrument(h: &mut Hasher, instrument: &InstrumentKey) {
    put_bytes(h, instrument.exchange.as_str().as_bytes());
    put_bytes(h, instrument.segment.as_str().as_bytes());
    put_bytes(h, instrument.underlying.as_str().as_bytes());
    match instrument.kind {
        Kind::Index => h.update(&[1]),
        Kind::Equity => h.update(&[2]),
        Kind::Future { expiry } => {
            h.update(&[3]);
            put_u16(h, expiry.year());
            h.update(&[expiry.month(), expiry.day()]);
        }
        Kind::Option {
            expiry,
            strike,
            side,
        } => {
            h.update(&[4]);
            put_u16(h, expiry.year());
            h.update(&[expiry.month(), expiry.day()]);
            put_i64(h, strike.raw());
            h.update(&[match side {
                brutex_core::instrument::OptionSide::Call => 1,
                brutex_core::instrument::OptionSide::Put => 2,
            }]);
        }
    }
}

fn put_mask(h: &mut Hasher, mask: ConditionMask) {
    for word in mask.words() {
        put_u64(h, word);
    }
}

fn put_chosen(h: &mut Hasher, chosen: Chosen) {
    put_option_usize(h, chosen.stop);
    put_option_usize(h, chosen.target);
    put_option_usize(h, chosen.tsl);
    match chosen.ttp {
        None => h.update(&[0]),
        Some(ttp) => {
            h.update(&[1]);
            put_usize(h, ttp.arm);
            put_usize(h, ttp.trail);
        }
    }
}

fn put_cell(h: &mut Hasher, cell: &Cell) {
    put_chosen(h, Chosen::from_cell(cell));
    put_u64(h, cell.trades);
    put_u64(h, cell.wins);
    put_i64(h, cell.pessimistic);
    put_i64(h, cell.optimistic);
    put_i64(h, cell.fill_cost);
    put_u64(h, cell.stopped);
    put_u64(h, cell.trailed_stop);
    put_u64(h, cell.trailed_profit);
    put_u64(h, cell.targeted);
    put_u64(h, cell.timed_out);
    put_u64(h, cell.ambiguous_bars);
    put_u64(h, cell.gapped);
    put_i64(h, cell.winner_mae);
    put_i64(h, cell.winner_mfe);
    put_i64(h, cell.all_mae);
    put_i64(h, cell.worst_mae);
    put_i64(h, cell.gross_win);
    put_i64(h, cell.gross_loss);
    put_i64(h, cell.best_trade);
    put_i64(h, cell.min_win);
    put_u64(h, cell.bars_held);
    put_u32(h, cell.max_losing_streak);
    put_u32(h, cell.max_winning_streak);
    put_i64(h, cell.worst_trade);
    put_i64(h, cell.max_drawdown);
}

fn put_trade_row(h: &mut Hasher, row: crate::grid::TradeRow) {
    put_usize(h, row.signal_bar);
    put_usize(h, row.entry_bar);
    put_usize(h, row.exit_bar);
    put_i64(h, row.best);
    put_i64(h, row.worst);
    put_i64(h, row.entry_micros);
    put_i64(h, row.exit_micros);
    put_i64(h, row.adverse);
    put_i64(h, row.adverse_paisa);
    put_i64(h, row.favourable);
    put_i64(h, row.favourable_paisa);
}

fn put_census(h: &mut Hasher, census: indicators::column::Census) {
    put_u64(h, census.offered);
    put_u64(h, census.warming);
    put_u64(h, census.swept);
    put_u64(h, census.high_below_low);
    put_u64(h, census.range_overflows);
    put_u64(h, census.price_outside_range);
    put_u64(h, census.timestamp_not_increasing);
    put_u64(h, census.negative_volume);
    put_u64(h, census.accumulator_too_large);
    // APPENDED, never inserted. The census buckets are folded in declaration
    // order and a run identity that reordered them would collide two different
    // censuses onto one hash -- `CLAUDE.md` §3 rule 8's append-only rule applied
    // to the fold rather than to a bit table.
    put_u64(h, census.price_not_positive);
}

fn put_option_usize(h: &mut Hasher, value: Option<usize>) {
    match value {
        None => h.update(&[0]),
        Some(index) => {
            h.update(&[1]);
            put_usize(h, index);
        }
    }
}

fn put_bytes(h: &mut Hasher, value: &[u8]) {
    put_usize(h, value.len());
    h.update(value);
}

fn put_percentiles(h: &mut Hasher, values: &[RationalPercentileV1]) {
    put_usize(h, values.len());
    for value in values {
        h.update(&value.numerator().to_le_bytes());
        h.update(&value.denominator().to_le_bytes());
    }
}

fn put_execution_resolution(h: &mut Hasher, value: ExecutionResolutionV1) {
    match value {
        ExecutionResolutionV1::OneMinuteOhlcv => h.update(&[1]),
        ExecutionResolutionV1::UnsupportedSeconds(seconds) => {
            h.update(&[2]);
            h.update(&seconds.to_le_bytes());
        }
    }
}

fn put_range_resolution(h: &mut Hasher, value: RangeResolutionV1) {
    h.update(&[match value {
        RangeResolutionV1::PpmFloor => 1,
        RangeResolutionV1::PpmCeiling => 2,
    }]);
}

fn put_side(h: &mut Hasher, value: crate::excursion::Side) {
    h.update(&[match value {
        crate::excursion::Side::Long => 1,
        crate::excursion::Side::Short => 2,
    }]);
}

const fn selector_byte(value: ExitGridSelectorV1) -> u8 {
    match value {
        ExitGridSelectorV1::PessimisticTotal => 1,
        ExitGridSelectorV1::EdgeThenPessimistic => 2,
        ExitGridSelectorV1::GuaranteedFloor => 3,
    }
}

fn put_forced_stop(h: &mut Hasher, value: ForcedStopV1) {
    match value {
        ForcedStopV1::Disabled => h.update(&[0]),
        ForcedStopV1::IncludeExactObserved(ppm) => {
            h.update(&[1]);
            put_i64(h, ppm);
        }
        ForcedStopV1::RequireExactObserved(ppm) => {
            h.update(&[2]);
            put_i64(h, ppm);
        }
    }
}

fn put_u16(h: &mut Hasher, value: u16) {
    h.update(&value.to_le_bytes());
}

fn put_u32(h: &mut Hasher, value: u32) {
    h.update(&value.to_le_bytes());
}

fn put_u64(h: &mut Hasher, value: u64) {
    h.update(&value.to_le_bytes());
}

fn put_i64(h: &mut Hasher, value: i64) {
    h.update(&value.to_le_bytes());
}

fn put_usize(h: &mut Hasher, value: usize) {
    put_u64(h, u64::try_from(value).unwrap_or(u64::MAX));
}

fn all_zero(bytes: &[u8; 32]) -> bool {
    bytes.iter().all(|byte| *byte == 0)
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "adversarial fixed-record fixtures must fail loudly and mutate exact byte positions"
)]
mod tests {
    use super::*;
    use brutex_core::instrument::{Expiry, OptionSide};
    use brutex_core::price::Paisa;
    use brutex_core::symbol::Symbol;
    use indicators::OI_NULL;
    use indicators::column::Column;
    use indicators::evaluator::{Evaluator, Widths};
    use indicators::pattern::Thresholds;
    use indicators::vwap::Availability;

    fn p(numerator: u32, denominator: u32) -> RationalPercentileV1 {
        RationalPercentileV1::new(numerator, denominator).unwrap_or(RationalPercentileV1 {
            numerator: 1,
            denominator: 1,
        })
    }

    fn policy() -> ExitGridPolicyV1 {
        let rungs = RungPlanV1::new(
            vec![p(1, 4), p(1, 2)],
            vec![p(3, 4), p(1, 1)],
            vec![p(1, 4), p(1, 2)],
            4,
        )
        .unwrap_or_else(|_| unreachable_policy_rungs());
        let ratios = RatioLimitsV1::new(100, 1_000, 16).unwrap_or(RatioLimitsV1 {
            min_hundredths: 100,
            max_hundredths: 1_000,
            max_pairs: 16,
        });
        ExitGridPolicyV1::new(
            ExecutionResolutionV1::OneMinuteOhlcv,
            RangeResolutionV1::PpmCeiling,
            crate::excursion::Side::Long,
            rungs,
            ratios,
            10_000,
            ExitGridSelectorV1::GuaranteedFloor,
            printed_ohlcv_cost_model_id_v1(),
            ForcedStopV1::Disabled,
            3,
            2,
        )
        .unwrap_or_else(|_| unreachable_policy())
    }

    fn unreachable_policy_rungs() -> RungPlanV1 {
        RungPlanV1 {
            stop: vec![p(1, 1)],
            target: vec![p(1, 1)],
            trail: vec![p(1, 1)],
            max_levels_per_axis: 1,
        }
    }

    fn unreachable_policy() -> ExitGridPolicyV1 {
        ExitGridPolicyV1 {
            execution_resolution: ExecutionResolutionV1::OneMinuteOhlcv,
            range_resolution: RangeResolutionV1::PpmCeiling,
            side: crate::excursion::Side::Long,
            rungs: unreachable_policy_rungs(),
            ratios: RatioLimitsV1 {
                min_hundredths: 100,
                max_hundredths: 100,
                max_pairs: 1,
            },
            max_cells: 1,
            selector: ExitGridSelectorV1::PessimisticTotal,
            cost_model_id: printed_ohlcv_cost_model_id_v1(),
            forced_stop: ForcedStopV1::Disabled,
            max_ambiguous_bars: 0,
            max_gap_fills: 0,
        }
    }

    fn bars(count: usize) -> Vec<Candle> {
        let first = REGULAR_OPEN_IST_MINUTE
            .saturating_mul(ONE_MINUTE_MICROS)
            .saturating_sub(indicators::IST_OFFSET_MICROS);
        (0..count)
            .filter_map(|index| {
                let i = i64::try_from(index).ok()?;
                let span = i.checked_add(1)?;
                Some(Candle {
                    ts_micros: first.checked_add(i.checked_mul(ONE_MINUTE_MICROS)?)?,
                    open: 1_000_000,
                    high: 1_000_000_i64.checked_add(span)?,
                    low: 1_000_000_i64.checked_sub(span)?,
                    close: 1_000_000,
                    volume: 1,
                    open_interest: OI_NULL,
                })
            })
            .collect()
    }

    fn shifted_bars(count: usize, days: i64) -> Vec<Candle> {
        let offset = days.saturating_mul(DAY_MICROS);
        bars(count)
            .into_iter()
            .map(|mut bar| {
                bar.ts_micros = bar.ts_micros.saturating_add(offset);
                bar
            })
            .collect()
    }

    fn test_column(bars: &[Candle]) -> Column {
        test_column_with_thresholds(bars, Thresholds::CLASSICAL)
    }

    fn test_column_with_thresholds(bars: &[Candle], thresholds: Thresholds) -> Column {
        let Ok(widths) = Widths::pinned() else {
            return Column::default();
        };
        let mut evaluator = Evaluator::new(widths, Availability::Absent, thresholds);
        Column::build(bars, &mut evaluator)
    }

    fn selected_for(resolved: &ResolvedExitGridV1, training: &[Candle]) -> Option<SelectedExitV1> {
        selected_for_feed(resolved, training, "test-feed")
    }

    fn selected_for_feed(
        resolved: &ResolvedExitGridV1,
        training: &[Candle],
        feed: &str,
    ) -> Option<SelectedExitV1> {
        let column = test_column(training);
        let series = ExecutionSeriesV1::new(
            &resolved.instrument,
            feed,
            "test-commit",
            [0xA5; 32],
            training,
        )
        .ok()?;
        let run = test_execution_run_for_feed(
            &resolved.instrument,
            &ConditionMask::ZERO,
            resolved.side(),
            training,
            feed,
        )
        .ok()?;
        let mut evaluated = resolved
            .evaluate_training_grid_attested(series, &column, Horizon::DEFAULT, run)
            .ok()?;
        let first = evaluated.grid.cells.first_mut()?;
        first.trades = 10;
        first.wins = 6;
        first.pessimistic = 100;
        first.gross_win = 600;
        first.gross_loss = -500;
        first.best_trade = 100;
        first.min_win = 100;
        first.worst_trade = -100;
        resolved.select(&evaluated).ok().flatten()
    }

    fn nifty() -> InstrumentKey {
        InstrumentKey::index(Exchange::Nse, "NIFTY").unwrap_or_else(|_| unsupported_key())
    }

    fn bank_nifty() -> InstrumentKey {
        InstrumentKey::index(Exchange::Nse, "BANKNIFTY").unwrap_or_else(|_| unsupported_key())
    }

    fn unsupported_key() -> InstrumentKey {
        InstrumentKey {
            exchange: Exchange::Nse,
            segment: Segment::Cash,
            underlying: symbol("X"),
            kind: Kind::Equity,
        }
    }

    fn symbol(text: &str) -> Symbol {
        let Ok(value) = Symbol::new(text) else {
            return symbol("X");
        };
        value
    }

    #[test]
    fn canonical_instrument_digest_binds_every_structural_field() {
        let nifty = nifty();
        assert_eq!(instrument_digest_v1(&nifty), instrument_digest_v1(&nifty));
        assert_ne!(
            instrument_digest_v1(&nifty),
            instrument_digest_v1(&bank_nifty())
        );
        let same_symbol_equity = InstrumentKey {
            exchange: Exchange::Nse,
            segment: Segment::Cash,
            underlying: symbol("NIFTY"),
            kind: Kind::Equity,
        };
        assert_ne!(
            instrument_digest_v1(&nifty),
            instrument_digest_v1(&same_symbol_equity)
        );
        let foreign_exchange =
            InstrumentKey::index(Exchange::Bse, "NIFTY").unwrap_or_else(|_| unsupported_key());
        assert_ne!(
            instrument_digest_v1(&nifty),
            instrument_digest_v1(&foreign_exchange)
        );

        let foreign_segment = InstrumentKey {
            segment: Segment::Cash,
            ..nifty
        };
        assert_ne!(
            instrument_digest_v1(&nifty),
            instrument_digest_v1(&foreign_segment)
        );

        let front_expiry = Expiry::new(2026, 8, 27)
            .unwrap_or_else(|_| Expiry::new(2026, 8, 1).unwrap_or_else(|_| unreachable_expiry()));
        let next_expiry = Expiry::new(2026, 9, 24)
            .unwrap_or_else(|_| Expiry::new(2026, 9, 1).unwrap_or_else(|_| unreachable_expiry()));
        let future = |expiry| InstrumentKey {
            exchange: Exchange::Nse,
            segment: Segment::Fno,
            underlying: symbol("NIFTY"),
            kind: Kind::Future { expiry },
        };
        assert_ne!(
            instrument_digest_v1(&future(front_expiry)),
            instrument_digest_v1(&future(next_expiry))
        );

        let option = |expiry, strike, side| InstrumentKey {
            exchange: Exchange::Nse,
            segment: Segment::Fno,
            underlying: symbol("NIFTY"),
            kind: Kind::Option {
                expiry,
                strike: Paisa::from_raw(strike),
                side,
            },
        };
        let baseline_option = option(front_expiry, 2_280_000, OptionSide::Call);
        for changed_option in [
            option(next_expiry, 2_280_000, OptionSide::Call),
            option(front_expiry, 2_285_000, OptionSide::Call),
            option(front_expiry, 2_280_000, OptionSide::Put),
        ] {
            assert_ne!(
                instrument_digest_v1(&baseline_option),
                instrument_digest_v1(&changed_option)
            );
        }
    }

    #[test]
    fn only_the_two_nse_spot_indices_resolve() {
        let input = bars(100);
        assert_eq!(
            policy().resolve(&nifty(), &input).map(|r| r.family()),
            Ok(InstrumentFamilyV1::Nifty)
        );
        assert_eq!(
            policy().resolve(&bank_nifty(), &input).map(|r| r.family()),
            Ok(InstrumentFamilyV1::BankNifty)
        );

        let expiry = Expiry::new(2026, 8, 27)
            .unwrap_or_else(|_| Expiry::new(2026, 8, 1).unwrap_or_else(|_| unreachable_expiry()));
        let symbols = [
            InstrumentKey {
                exchange: Exchange::Nse,
                segment: Segment::Cash,
                underlying: symbol("RELIANCE"),
                kind: Kind::Equity,
            },
            InstrumentKey {
                exchange: Exchange::Nse,
                segment: Segment::Fno,
                underlying: symbol("NIFTY"),
                kind: Kind::Future { expiry },
            },
            InstrumentKey {
                exchange: Exchange::Nse,
                segment: Segment::Fno,
                underlying: symbol("NIFTY"),
                kind: Kind::Option {
                    expiry,
                    strike: Paisa::from_raw(2_500_000),
                    side: OptionSide::Call,
                },
            },
            InstrumentKey::index(Exchange::Nse, "SENSEX").unwrap_or_else(|_| unsupported_key()),
            InstrumentKey::index(Exchange::Bse, "NIFTY").unwrap_or_else(|_| unsupported_key()),
        ];
        for instrument in symbols {
            assert_eq!(
                policy().resolve(&instrument, &input),
                Err(ExitGridErrorV1::UnsupportedInstrument),
                "{instrument} escaped the two-index surface"
            );
        }
    }

    #[test]
    fn each_legal_index_resolves_from_its_own_training_distribution() {
        let nifty_bars = bars(100);
        let mut bank_bars = nifty_bars.clone();
        for (index, bar) in bank_bars.iter_mut().enumerate() {
            let span = i64::try_from(index).unwrap_or(0).saturating_add(1);
            bar.high = bar.open.saturating_add(span.saturating_mul(10));
            bar.low = bar.open.saturating_sub(span.saturating_mul(10));
        }
        let nifty_grid = policy()
            .resolve(&nifty(), &nifty_bars)
            .unwrap_or_else(|_| unreachable_resolved());
        let bank_grid = policy()
            .resolve(&bank_nifty(), &bank_bars)
            .unwrap_or_else(|_| unreachable_resolved());
        assert_eq!(nifty_grid.version(), EXIT_GRID_POLICY_VERSION_V1);
        assert_eq!(nifty_grid.policy().version(), EXIT_GRID_POLICY_VERSION_V1);
        assert_eq!(nifty_grid.stop_levels_ppm(), &[25, 50]);
        assert_eq!(bank_grid.stop_levels_ppm(), &[250, 500]);
        assert_ne!(nifty_grid.digest(), bank_grid.digest());
    }

    #[test]
    fn long_and_short_resolve_separate_adverse_and_favourable_axes() {
        let mut asymmetric = bars(100);
        for (index, bar) in asymmetric.iter_mut().enumerate() {
            let span = i64::try_from(index).unwrap_or(0).saturating_add(1);
            bar.high = bar.open.saturating_add(span.saturating_mul(4));
            bar.low = bar.open.saturating_sub(span);
        }

        let long = policy()
            .resolve(&nifty(), &asymmetric)
            .unwrap_or_else(|_| unreachable_resolved());
        let mut short_policy = policy();
        short_policy.side = crate::excursion::Side::Short;
        let short = short_policy
            .resolve(&nifty(), &asymmetric)
            .unwrap_or_else(|_| unreachable_resolved());

        assert_eq!(long.side(), crate::excursion::Side::Long);
        assert_eq!(short.side(), crate::excursion::Side::Short);
        assert_eq!(long.stop_levels_ppm(), &[25, 50]);
        assert_eq!(long.target_levels_ppm(), &[300, 400]);
        assert_eq!(short.stop_levels_ppm(), &[100, 200]);
        assert_eq!(short.target_levels_ppm(), &[75, 100]);
        assert_eq!(long.trail_levels_ppm(), short.trail_levels_ppm());
        assert_eq!(long.trail_levels_ppm(), &[125, 250]);
        assert_ne!(long.policy_digest(), short.policy_digest());
        assert_ne!(long.digest(), short.digest());
    }

    #[test]
    fn post_deadline_prices_rekey_the_source_but_cannot_choose_a_tradable_rung() {
        let mut cfg = policy();
        cfg.rungs = RungPlanV1::new(vec![p(1, 1)], vec![p(1, 1)], vec![p(1, 1)], 1)
            .unwrap_or_else(|_| unreachable_policy_rungs());
        let baseline_bars = bars(375);
        let baseline = cfg
            .resolve(&nifty(), &baseline_bars)
            .unwrap_or_else(|_| unreachable_resolved());

        let mut post_deadline = baseline_bars.clone();
        for bar in post_deadline.iter_mut().skip(355) {
            bar.high = bar.open.saturating_add(100_000);
            bar.low = bar.open.saturating_sub(100_000);
        }
        let post = cfg
            .resolve(&nifty(), &post_deadline)
            .unwrap_or_else(|_| unreachable_resolved());
        assert_eq!(post.stop_levels_ppm(), baseline.stop_levels_ppm());
        assert_eq!(post.target_levels_ppm(), baseline.target_levels_ppm());
        assert_eq!(post.trail_levels_ppm(), baseline.trail_levels_ppm());
        assert_ne!(
            post.training_digest(),
            baseline.training_digest(),
            "attestation still binds post-deadline source bytes"
        );
        assert_ne!(post.digest(), baseline.digest());

        let mut last_tradable = baseline_bars;
        let forced_fill = last_tradable
            .get_mut(354)
            .expect("a complete regular session contains the 15:09 row");
        forced_fill.high = forced_fill.open.saturating_add(200_000);
        forced_fill.low = forced_fill.open.saturating_sub(200_000);
        let changed = cfg
            .resolve(&nifty(), &last_tradable)
            .unwrap_or_else(|_| unreachable_resolved());
        assert_ne!(changed.stop_levels_ppm(), baseline.stop_levels_ppm());
        assert_ne!(changed.target_levels_ppm(), baseline.target_levels_ppm());
        assert_ne!(changed.trail_levels_ppm(), baseline.trail_levels_ppm());
    }

    fn unreachable_expiry() -> Expiry {
        match Expiry::new(2026, 1, 1) {
            Ok(value) => value,
            Err(_) => unreachable_expiry(),
        }
    }

    #[test]
    fn p99_is_an_exact_observed_nearest_rank_and_duplicates_collapse() {
        let plan = RungPlanV1::new(
            vec![p(1, 2), p(99, 100)],
            vec![p(99, 100), p(1, 1)],
            vec![p(1, 2), p(99, 100)],
            4,
        )
        .unwrap_or_else(|_| unreachable_policy_rungs());
        let mut cfg = policy();
        cfg.rungs = plan;
        let resolved = cfg
            .resolve(&nifty(), &bars(100))
            .unwrap_or_else(|_| unreachable_resolved());
        assert_eq!(resolved.stop_levels_ppm(), &[50, 99]);
        assert_eq!(resolved.target_levels_ppm(), &[99, 100]);
        assert_eq!(resolved.trail_levels_ppm(), &[100, 198]);

        let flat = vec![
            Candle {
                ts_micros: 0,
                open: 1_000_000,
                high: 1_000_010,
                low: 999_990,
                close: 1_000_000,
                volume: 1,
                open_interest: OI_NULL,
            };
            10
        ];
        let mut flat = flat;
        let first = REGULAR_OPEN_IST_MINUTE
            .saturating_mul(ONE_MINUTE_MICROS)
            .saturating_sub(indicators::IST_OFFSET_MICROS);
        for (index, bar) in flat.iter_mut().enumerate() {
            bar.ts_micros = first.saturating_add(
                i64::try_from(index)
                    .unwrap_or(0)
                    .saturating_mul(ONE_MINUTE_MICROS),
            );
        }
        let one = policy()
            .resolve(&nifty(), &flat)
            .unwrap_or_else(|_| unreachable_resolved());
        assert_eq!(one.stop_levels_ppm(), &[10]);
        assert_eq!(one.target_levels_ppm(), &[10]);
        assert_eq!(one.trail_levels_ppm(), &[20]);
    }

    #[test]
    fn equivalent_rational_percentiles_canonicalize_to_one_identity() {
        let half = RationalPercentileV1::new(1, 2);
        let two_fourths = RationalPercentileV1::new(2, 4);
        assert_eq!(half, two_fourths);
        assert_eq!(two_fourths.as_ref().map(|value| value.numerator()), Ok(1));
        assert_eq!(two_fourths.as_ref().map(|value| value.denominator()), Ok(2));
    }

    #[test]
    fn flat_observations_remain_in_the_distribution_and_can_refuse_a_rung() {
        let mut input = bars(10);
        for bar in input.iter_mut().take(9) {
            bar.high = bar.open;
            bar.low = bar.open;
            bar.close = bar.open;
        }
        assert_eq!(
            policy().resolve(&nifty(), &input),
            Err(ExitGridErrorV1::ZeroResolvedRung("stop"))
        );
    }

    #[test]
    fn evaluator_spec_bytes_and_column_identity_change_with_every_policy() {
        let input = bars(3);
        let ordinary = test_column(&input);
        let mut thresholds = Thresholds::CLASSICAL;
        thresholds.doji_body = thresholds.doji_body.saturating_add(1);
        let changed = test_column_with_thresholds(&input, thresholds);
        let ordinary_token = ordinary.evaluation_spec_token();
        let changed_token = changed.evaluation_spec_token();
        assert!(ordinary_token.is_some());
        assert!(changed_token.is_some());
        assert_ne!(ordinary_token, changed_token);
        assert_ne!(digest_column(&ordinary), digest_column(&changed));
    }

    fn unreachable_resolved() -> ResolvedExitGridV1 {
        let policy = unreachable_policy();
        let instrument = nifty();
        let first = REGULAR_OPEN_IST_MINUTE
            .saturating_mul(ONE_MINUTE_MICROS)
            .saturating_sub(indicators::IST_OFFSET_MICROS);
        ResolvedExitGridV1 {
            instrument,
            family: InstrumentFamilyV1::Nifty,
            feed_digest: hash(b"test-feed"),
            commit_digest: hash(b"test-commit"),
            calendar_digest: [0xA5; 32],
            policy_digest: policy.digest(),
            training_digest: [1; 32],
            training_bars: 1,
            training_first_ts_micros: first,
            training_last_ts_micros: first,
            stop_levels_ppm: vec![1],
            target_levels_ppm: vec![1],
            trail_levels_ppm: vec![1],
            ratio_pairs: vec![RatioPairV1 {
                stop_index: 0,
                target_index: 0,
            }],
            ratio_bitmap: vec![true],
            forced_stop_index: None,
            cell_count: 1,
            digest: [2; 32],
            policy,
        }
    }

    fn canonical_evaluation(
        resolved: &ResolvedExitGridV1,
        input: &[Candle],
    ) -> EvaluatedExitGridV1 {
        canonical_evaluation_for_mask(resolved, input, ConditionMask::ZERO)
    }

    fn canonical_evaluation_for_mask(
        resolved: &ResolvedExitGridV1,
        input: &[Candle],
        mask: ConditionMask,
    ) -> EvaluatedExitGridV1 {
        let ladders = resolved
            .ladders()
            .expect("the freshly resolved ladders are valid");
        let mut cells = Vec::new();
        assert!(resolved.visit_coordinates(|chosen| {
            cells.push(Cell {
                stop: chosen.stop,
                target: chosen.target,
                tsl: chosen.tsl,
                ttp: chosen.ttp,
                trades: 1,
                ..Cell::default()
            });
            true
        }));
        let mut evaluated = resolved
            .evaluate_training_grid(
                input,
                &test_column(input),
                &mask,
                Horizon::DEFAULT,
                resolved.side(),
            )
            .expect("the canonical test execution evaluates");
        evaluated.grid = Grid {
            cells,
            signals: 1,
            stops: ladders.stops,
            targets: ladders.targets,
            trails: ladders.trails,
            refused_paths: 0,
        };
        evaluated
    }

    #[test]
    fn identical_bytes_and_policy_are_byte_identical_and_any_bar_byte_rekeys() {
        let input = bars(100);
        let a = policy()
            .resolve(&nifty(), &input)
            .unwrap_or_else(|_| unreachable_resolved());
        let b = policy()
            .resolve(&nifty(), &input)
            .unwrap_or_else(|_| unreachable_resolved());
        assert_eq!(a, b);
        assert_eq!(a.digest(), b.digest());

        let mut changed = input.clone();
        if let Some(bar) = changed.get_mut(50) {
            bar.volume = bar.volume.saturating_add(1);
        }
        let c = policy()
            .resolve(&nifty(), &changed)
            .unwrap_or_else(|_| unreachable_resolved());
        assert_eq!(a.stop_levels_ppm(), c.stop_levels_ppm());
        assert_ne!(a.training_digest(), c.training_digest());
        assert_ne!(a.digest(), c.digest());
    }

    #[test]
    fn execution_run_recomputes_data_and_refuses_mismatched_identity_terms() {
        let input = bars(4);
        let instrument = nifty();
        let params = crate::identity::Params {
            min_hits: 1,
            ceiling: 10,
            pair_budget: 10,
            policy: 7,
        };
        let wrong_data_run = crate::identity::Run {
            mask: ConditionMask::ZERO,
            direction: Direction::Long,
            instrument: &instrument,
            timeframe: "1min",
            params,
            data_digest: [0; 32],
            commit: "test-commit",
            feed: "test-feed",
        };
        assert_eq!(
            ExecutionRunV1::new(&wrong_data_run, &input, None),
            Err(ExitGridErrorV1::RunDataDigestMismatch)
        );

        let other_feed_run = crate::identity::Run {
            data_digest: crate::identity::data_digest(&input),
            feed: "other-feed",
            ..wrong_data_run
        };
        let sealed = ExecutionRunV1::new(&other_feed_run, &input, None);
        assert!(sealed.is_ok());
        let Some(sealed) = sealed.ok() else {
            return;
        };
        let resolved = policy()
            .resolve(&instrument, &input)
            .unwrap_or_else(|_| unreachable_resolved());
        let series =
            ExecutionSeriesV1::new(&instrument, "test-feed", "test-commit", [0xA5; 32], &input);
        assert!(series.is_ok());
        let Some(series) = series.ok() else {
            return;
        };
        assert_eq!(
            resolved.evaluate_training_grid_attested(
                series,
                &test_column(&input),
                Horizon::DEFAULT,
                sealed,
            ),
            Err(ExitGridErrorV1::RunIdentityMismatch("feed"))
        );
    }

    #[test]
    fn stored_execution_run_preserves_the_complete_daily_reference_identity() {
        let signal = bars(4);
        let execution = shifted_bars(5, 1);
        let daily = shifted_bars(2, -1);
        let eligibility = [1_u8, 0];
        let excluded = [20_382_i64];
        let reference = crate::identity::DailyReferenceBinding {
            daily_bars: &daily,
            eligibility: &eligibility,
            schema: 1,
            eligibility_policy: 2,
            gap_overlay_policy: 3,
            excluded_ist_days: &excluded,
            daily_integrity: crate::identity::ReferenceIntegrity::UnverifiedNoReceipt,
            minute_integrity: crate::identity::ReferenceIntegrity::UnverifiedNoReceipt,
        };
        let digest =
            crate::identity::data_digest_with_daily_reference(&signal, &execution, reference)
                .unwrap_or([0; 32]);
        let instrument = nifty();
        let run = crate::identity::Run {
            mask: ConditionMask::ZERO,
            direction: Direction::Long,
            instrument: &instrument,
            timeframe: "5min",
            params: crate::identity::Params {
                min_hits: 1,
                ceiling: 10,
                pair_budget: 10,
                policy: 7,
            },
            data_digest: digest,
            commit: "test-commit",
            feed: "test-feed",
        };

        assert_eq!(
            ExecutionRunV1::new(&run, &signal, Some(&execution)),
            Err(ExitGridErrorV1::RunDataDigestMismatch),
            "a three-stream run must never be accepted through the weaker two-stream door"
        );
        let sealed = ExecutionRunV1::new_with_daily_reference(
            &run, &signal, &execution, &execution, reference,
        );
        assert!(sealed.is_ok());
        assert_eq!(
            sealed.map(ExecutionRunV1::run_id),
            Ok(crate::identity::identity(&run))
        );

        let malformed = crate::identity::DailyReferenceBinding {
            eligibility: &eligibility[..1],
            ..reference
        };
        assert_eq!(
            ExecutionRunV1::new_with_daily_reference(
                &run, &signal, &execution, &execution, malformed,
            ),
            Err(ExitGridErrorV1::DailyReferenceIdentityRefused(
                crate::identity::DailyBindingRefusal::EligibilityLengthMismatch {
                    daily: 2,
                    eligibility: 1,
                }
            ))
        );
    }

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "one identity test keeps warm-up-only, detached-slice and jointly re-keyed mutations beside the clean authorization"
    )]
    fn daily_reference_context_and_exact_evaluated_slice_are_bound_separately() {
        let signal = bars(20);
        let reference_context = shifted_bars(120, 1);
        let evaluated_execution = reference_context[10..100].to_vec();
        let daily = shifted_bars(2, -1);
        let eligibility = [1_u8, 1];
        let excluded = [20_382_i64];
        let reference = crate::identity::DailyReferenceBinding {
            daily_bars: &daily,
            eligibility: &eligibility,
            schema: 1,
            eligibility_policy: 2,
            gap_overlay_policy: 3,
            excluded_ist_days: &excluded,
            daily_integrity: crate::identity::ReferenceIntegrity::UnverifiedNoReceipt,
            minute_integrity: crate::identity::ReferenceIntegrity::UnverifiedNoReceipt,
        };
        let instrument = nifty();
        let run_with = |data_digest| crate::identity::Run {
            mask: ConditionMask::ZERO,
            direction: Direction::Long,
            instrument: &instrument,
            timeframe: "5min",
            params: crate::identity::Params {
                min_hits: 1,
                ceiling: 10,
                pair_budget: 10,
                policy: 7,
            },
            data_digest,
            commit: "test-commit",
            feed: "test-feed",
        };
        let digest = crate::identity::data_digest_with_daily_reference(
            &signal,
            &reference_context,
            reference,
        )
        .expect("the complete daily binding");
        let run = run_with(digest);
        let sealed = ExecutionRunV1::new_with_daily_reference(
            &run,
            &signal,
            &reference_context,
            &evaluated_execution,
            reference,
        )
        .expect("the requested span is an exact context subslice");
        let original_id = sealed.run_id();

        let resolved = policy()
            .resolve(&instrument, &evaluated_execution)
            .unwrap_or_else(|_| unreachable_resolved());
        let series = ExecutionSeriesV1::new(
            &instrument,
            "test-feed",
            "test-commit",
            [0xA5; 32],
            &evaluated_execution,
        )
        .expect("the exact evaluated series");
        assert!(
            resolved
                .evaluate_training_grid_attested(
                    series,
                    &test_column(&evaluated_execution),
                    Horizon::DEFAULT,
                    sealed,
                )
                .is_ok(),
            "warm-up context may be wider while only the requested execution slice is priced"
        );

        let mut changed_warm_up = reference_context.clone();
        changed_warm_up[0].volume = changed_warm_up[0].volume.saturating_add(1);
        let unchanged_run = run_with(digest);
        assert_eq!(
            ExecutionRunV1::new_with_daily_reference(
                &unchanged_run,
                &signal,
                &changed_warm_up,
                &evaluated_execution,
                reference,
            ),
            Err(ExitGridErrorV1::RunDataDigestMismatch),
            "changing only warm-up context cannot retain the old run identity"
        );
        let changed_warm_digest =
            crate::identity::data_digest_with_daily_reference(&signal, &changed_warm_up, reference)
                .expect("the changed warm-up still has a complete binding");
        let changed_warm_run = run_with(changed_warm_digest);
        let changed_warm_seal = ExecutionRunV1::new_with_daily_reference(
            &changed_warm_run,
            &signal,
            &changed_warm_up,
            &evaluated_execution,
            reference,
        )
        .expect("re-keyed warm-up context authorizes the unchanged requested span");
        assert_ne!(changed_warm_seal.run_id(), original_id);

        let mut detached_execution = evaluated_execution.clone();
        detached_execution[0].volume = detached_execution[0].volume.saturating_add(1);
        let original_run = run_with(digest);
        assert_eq!(
            ExecutionRunV1::new_with_daily_reference(
                &original_run,
                &signal,
                &reference_context,
                &detached_execution,
                reference,
            ),
            Err(ExitGridErrorV1::EvaluatedExecutionOutsideReferenceContext),
            "execution bytes not present in the digested context cannot be substituted"
        );

        let mut changed_context = reference_context.clone();
        changed_context[10].volume = changed_context[10].volume.saturating_add(1);
        let changed_execution = changed_context[10..100].to_vec();
        let changed_digest =
            crate::identity::data_digest_with_daily_reference(&signal, &changed_context, reference)
                .expect("the changed context remains well formed");
        let changed_run = run_with(changed_digest);
        let changed_seal = ExecutionRunV1::new_with_daily_reference(
            &changed_run,
            &signal,
            &changed_context,
            &changed_execution,
            reference,
        )
        .expect("the changed requested bytes are bound through their changed context");
        assert_ne!(changed_seal.run_id(), original_id);
        assert_eq!(
            changed_seal.require_matches(series, crate::excursion::Side::Long),
            Err(ExitGridErrorV1::RunIdentityMismatch("execution data")),
            "a re-keyed evaluated slice cannot authorize the old ExecutionSeries"
        );
        let changed_series = ExecutionSeriesV1::new(
            &instrument,
            "test-feed",
            "test-commit",
            [0xA5; 32],
            &changed_execution,
        )
        .expect("the changed exact execution series");
        assert_eq!(
            changed_seal.require_matches(changed_series, crate::excursion::Side::Long),
            Ok(())
        );
    }

    #[test]
    fn every_resolved_content_field_is_digest_load_bearing() {
        let base = policy()
            .resolve(&nifty(), &bars(100))
            .unwrap_or_else(|_| unreachable_resolved());
        assert!(base.digest_is_valid());
        let digest = digest_resolved(&base);
        let mut changes = Vec::new();

        let mut value = base.clone();
        value.instrument = bank_nifty();
        changes.push(digest_resolved(&value));
        let mut value = base.clone();
        value.family = InstrumentFamilyV1::BankNifty;
        changes.push(digest_resolved(&value));
        let mut value = base.clone();
        value.feed_digest = [6; 32];
        changes.push(digest_resolved(&value));
        let mut value = base.clone();
        value.commit_digest = [7; 32];
        changes.push(digest_resolved(&value));
        let mut value = base.clone();
        value.calendar_digest = [8; 32];
        changes.push(digest_resolved(&value));
        let mut value = base.clone();
        value.policy.selector = ExitGridSelectorV1::EdgeThenPessimistic;
        changes.push(digest_resolved(&value));
        let mut value = base.clone();
        value.policy_digest = [9; 32];
        changes.push(digest_resolved(&value));
        let mut value = base.clone();
        value.training_digest = [10; 32];
        changes.push(digest_resolved(&value));
        let mut value = base.clone();
        value.training_bars = value.training_bars.saturating_add(1);
        changes.push(digest_resolved(&value));
        let mut value = base.clone();
        value.training_first_ts_micros = value.training_first_ts_micros.saturating_add(1);
        changes.push(digest_resolved(&value));
        let mut value = base.clone();
        value.training_last_ts_micros = value.training_last_ts_micros.saturating_add(1);
        changes.push(digest_resolved(&value));
        let mut value = base.clone();
        if let Some(level) = value.stop_levels_ppm.first_mut() {
            *level = level.saturating_add(1);
        }
        changes.push(digest_resolved(&value));
        let mut value = base.clone();
        if let Some(level) = value.target_levels_ppm.first_mut() {
            *level = level.saturating_add(1);
        }
        changes.push(digest_resolved(&value));
        let mut value = base.clone();
        if let Some(level) = value.trail_levels_ppm.first_mut() {
            *level = level.saturating_add(1);
        }
        changes.push(digest_resolved(&value));
        let mut value = base.clone();
        if let Some(pair) = value.ratio_pairs.first_mut() {
            pair.target_index = pair.target_index.saturating_add(1);
        }
        changes.push(digest_resolved(&value));
        let mut value = base.clone();
        if let Some(admitted) = value.ratio_bitmap.first_mut() {
            *admitted = !*admitted;
        }
        changes.push(digest_resolved(&value));
        let mut value = base.clone();
        value.forced_stop_index = Some(0);
        changes.push(digest_resolved(&value));
        let mut value = base;
        value.cell_count = value.cell_count.saturating_add(1);
        changes.push(digest_resolved(&value));

        assert!(changes.iter().all(|candidate| *candidate != digest));
        changes.sort_unstable();
        changes.dedup();
        assert_eq!(changes.len(), 18, "two resolved field mutations collided");
    }

    #[test]
    fn every_policy_field_is_digest_load_bearing() {
        let base = policy();
        let digest = base.digest();
        let mut changes = Vec::new();

        let mut p = base.clone();
        p.execution_resolution = ExecutionResolutionV1::UnsupportedSeconds(30);
        changes.push(p.digest());
        let mut p = base.clone();
        p.range_resolution = RangeResolutionV1::PpmFloor;
        changes.push(p.digest());
        let mut p = base.clone();
        p.side = crate::excursion::Side::Short;
        changes.push(p.digest());
        let mut p = base.clone();
        p.rungs.stop = vec![pctl(1, 3), pctl(1, 2)];
        changes.push(p.digest());
        let mut p = base.clone();
        p.rungs.target = vec![pctl(2, 3), pctl(1, 1)];
        changes.push(p.digest());
        let mut p = base.clone();
        p.rungs.trail = vec![pctl(1, 5), pctl(1, 2)];
        changes.push(p.digest());
        let mut p = base.clone();
        p.rungs.max_levels_per_axis = 5;
        changes.push(p.digest());
        let mut p = base.clone();
        p.ratios.min_hundredths = 101;
        changes.push(p.digest());
        let mut p = base.clone();
        p.ratios.max_hundredths = 999;
        changes.push(p.digest());
        let mut p = base.clone();
        p.ratios.max_pairs = 15;
        changes.push(p.digest());
        let mut p = base.clone();
        p.max_cells = 9_999;
        changes.push(p.digest());
        let mut p = base.clone();
        p.selector = ExitGridSelectorV1::PessimisticTotal;
        changes.push(p.digest());
        let mut p = base.clone();
        p.cost_model_id = [8; 32];
        changes.push(p.digest());
        let mut p = base.clone();
        p.forced_stop = ForcedStopV1::IncludeExactObserved(25);
        changes.push(p.digest());
        let mut p = base.clone();
        p.max_ambiguous_bars = 4;
        changes.push(p.digest());
        let mut p = base;
        p.max_gap_fills = 3;
        changes.push(p.digest());

        assert!(changes.iter().all(|candidate| *candidate != digest));
        changes.sort_unstable();
        changes.dedup();
        assert_eq!(changes.len(), 16, "two field mutations collided");
    }

    fn pctl(numerator: u32, denominator: u32) -> RationalPercentileV1 {
        RationalPercentileV1 {
            numerator,
            denominator,
        }
    }

    #[test]
    fn empty_duplicate_backward_off_grid_and_subminute_inputs_refuse_loudly() {
        assert_eq!(
            policy().resolve(&nifty(), &[]),
            Err(ExitGridErrorV1::EmptyExecutionSeries)
        );
        let mut duplicate = bars(3);
        let first = duplicate.first().map_or(0, |bar| bar.ts_micros);
        if let Some(second) = duplicate.get_mut(1) {
            second.ts_micros = first;
        }
        assert!(matches!(
            policy().resolve(&nifty(), &duplicate),
            Err(ExitGridErrorV1::NonIncreasingTimestamp { .. })
        ));
        let mut backward = bars(3);
        if let Some(second) = backward.get_mut(1) {
            second.ts_micros = -ONE_MINUTE_MICROS;
        }
        assert!(matches!(
            policy().resolve(&nifty(), &backward),
            Err(ExitGridErrorV1::NonIncreasingTimestamp { .. })
        ));
        let mut off_grid = bars(3);
        if let Some(second) = off_grid.get_mut(1) {
            second.ts_micros = second.ts_micros.saturating_add(1);
        }
        assert!(matches!(
            policy().resolve(&nifty(), &off_grid),
            Err(ExitGridErrorV1::OffMinuteTimestamp { .. })
        ));
    }

    #[test]
    fn whole_minute_gaps_stay_gaps_without_changing_observed_ranges() {
        let compact = bars(4);
        let mut gapped = compact.clone();
        if let Some(bar) = gapped.get_mut(2) {
            bar.ts_micros = bar.ts_micros.saturating_add(ONE_MINUTE_MICROS);
        }
        if let Some(bar) = gapped.get_mut(3) {
            bar.ts_micros = bar.ts_micros.saturating_add(ONE_MINUTE_MICROS);
        }
        let compact_grid = policy()
            .resolve(&nifty(), &compact)
            .unwrap_or_else(|_| unreachable_resolved());
        let gapped_grid = policy()
            .resolve(&nifty(), &gapped)
            .unwrap_or_else(|_| unreachable_resolved());
        assert_eq!(
            compact_grid.stop_levels_ppm(),
            gapped_grid.stop_levels_ppm()
        );
        assert_eq!(
            compact_grid.target_levels_ppm(),
            gapped_grid.target_levels_ppm()
        );
        assert_eq!(
            compact_grid.trail_levels_ppm(),
            gapped_grid.trail_levels_ppm()
        );
        assert_ne!(
            compact_grid.training_digest(),
            gapped_grid.training_digest(),
            "the missing minute remains visible in exact source identity"
        );
    }

    #[test]
    fn calendar_attested_nonregular_minutes_are_not_rewritten_as_regular_session_data() {
        let mut early_session = bars(100);
        for bar in &mut early_session {
            bar.ts_micros = bar.ts_micros.saturating_sub(2 * 60 * ONE_MINUTE_MICROS);
        }
        assert!(policy().resolve(&nifty(), &early_session).is_ok());

        let mut post_deadline_session = bars(100);
        for bar in &mut post_deadline_session {
            bar.ts_micros = bar.ts_micros.saturating_add(9 * 60 * ONE_MINUTE_MICROS);
        }
        assert_eq!(
            policy().resolve(&nifty(), &post_deadline_session),
            Err(ExitGridErrorV1::NoPositiveObservedRange("stop")),
            "calendar-attested rows remain source bytes, but a post-15:10 session supplies no tradable exit rung"
        );
    }

    #[test]
    fn corrupt_nonpositive_flat_and_extreme_arithmetic_inputs_never_fabricate_a_level() {
        let mut corrupt = bars(4);
        if let Some(bar) = corrupt.get_mut(1) {
            bar.high = bar.low.saturating_sub(1);
        }
        assert!(matches!(
            policy().resolve(&nifty(), &corrupt),
            Err(ExitGridErrorV1::CorruptExecutionCandle { .. })
        ));

        let mut zero_open = bars(4);
        if let Some(bar) = zero_open.get_mut(1) {
            bar.open = 0;
            bar.low = 0;
            bar.close = 0;
        }
        assert!(matches!(
            policy().resolve(&nifty(), &zero_open),
            Err(ExitGridErrorV1::NonPositiveOpen { .. })
        ));

        let flat: Vec<Candle> = bars(4)
            .into_iter()
            .map(|mut bar| {
                bar.high = bar.open;
                bar.low = bar.open;
                bar
            })
            .collect();
        assert_eq!(
            policy().resolve(&nifty(), &flat),
            Err(ExitGridErrorV1::NoPositiveObservedRange("stop"))
        );

        let extreme = [Candle {
            ts_micros: REGULAR_OPEN_IST_MINUTE
                .saturating_mul(ONE_MINUTE_MICROS)
                .saturating_sub(indicators::IST_OFFSET_MICROS),
            open: 1,
            high: i64::MAX,
            low: 1,
            close: 1,
            volume: 1,
            open_interest: OI_NULL,
        }];
        assert_eq!(
            policy().resolve(&nifty(), &extreme),
            Err(ExitGridErrorV1::ArithmeticOverflow("target"))
        );
        assert_eq!(
            checked_policy_cell_count(usize::MAX, usize::MAX, usize::MAX, &[]),
            None
        );
    }

    #[test]
    fn forced_stop_ratio_pair_and_cell_limits_are_fail_closed() {
        let input = bars(100);
        let mut missing = policy();
        missing.forced_stop = ForcedStopV1::IncludeExactObserved(101);
        assert_eq!(
            missing.resolve(&nifty(), &input),
            Err(ExitGridErrorV1::ForcedStopNotObserved(101))
        );

        let mut forced = policy();
        forced.forced_stop = ForcedStopV1::RequireExactObserved(25);
        let resolved = forced
            .resolve(&nifty(), &input)
            .unwrap_or_else(|_| unreachable_resolved());
        assert_eq!(resolved.stop_levels_ppm(), &[25, 50]);
        assert_eq!(resolved.forced_stop_index(), Some(0));

        let mut already_selected = policy();
        already_selected.forced_stop = ForcedStopV1::RequireExactObserved(50);
        let resolved = already_selected
            .resolve(&nifty(), &input)
            .unwrap_or_else(|_| unreachable_resolved());
        assert_eq!(resolved.stop_levels_ppm(), &[25, 50]);
        assert_eq!(resolved.forced_stop_index(), Some(1));

        let mut after_percentiles = policy();
        after_percentiles.forced_stop = ForcedStopV1::IncludeExactObserved(75);
        let resolved = after_percentiles
            .resolve(&nifty(), &input)
            .unwrap_or_else(|_| unreachable_resolved());
        assert_eq!(resolved.stop_levels_ppm(), &[25, 50, 75]);
        assert_eq!(resolved.forced_stop_index(), Some(2));

        let mut pair_limited = policy();
        pair_limited.ratios.max_pairs = 1;
        assert!(matches!(
            pair_limited.resolve(&nifty(), &input),
            Err(ExitGridErrorV1::RatioPairLimitExceeded { .. })
        ));

        let mut cell_limited = policy();
        cell_limited.max_cells = 1;
        assert!(matches!(
            cell_limited.resolve(&nifty(), &input),
            Err(ExitGridErrorV1::CellLimitExceeded { .. })
        ));

        let mut impossible = policy();
        impossible.ratios.min_hundredths = 20_000;
        impossible.ratios.max_hundredths = 30_000;
        assert_eq!(
            impossible.resolve(&nifty(), &input),
            Err(ExitGridErrorV1::NoAdmittedRatioPair)
        );
    }

    #[test]
    fn checked_cell_count_matches_every_small_ratio_filtered_enumeration() {
        for stops in 0..=3 {
            for targets in 0..=3 {
                for trails in 0..=3 {
                    let slots = stops * targets;
                    let subsets = 1_usize.checked_shl(u32::try_from(slots).unwrap_or(0));
                    let Some(subsets) = subsets else {
                        continue;
                    };
                    for bits in 0..subsets {
                        let mut pairs = Vec::new();
                        for stop_index in 0..stops {
                            for target_index in 0..targets {
                                let slot = stop_index * targets + target_index;
                                if bits & (1_usize << slot) != 0 {
                                    pairs.push(RatioPairV1 {
                                        stop_index,
                                        target_index,
                                    });
                                }
                            }
                        }
                        let exact = checked_policy_cell_count(stops, targets, trails, &pairs);
                        assert_eq!(
                            exact,
                            Some(brute_cell_count(stops, targets, trails, &pairs))
                        );
                    }
                }
            }
        }

        let mut forced = policy();
        forced.forced_stop = ForcedStopV1::RequireExactObserved(25);
        let resolved = forced
            .resolve(&nifty(), &bars(100))
            .unwrap_or_else(|_| unreachable_resolved());
        let mut visited = 0_u64;
        assert!(resolved.visit_coordinates(|_| {
            visited = visited.saturating_add(1);
            true
        }));
        assert_eq!(visited, resolved.cell_count());
    }

    #[test]
    fn validated_canonical_ordinals_name_every_authorizable_cell_exactly_once() {
        let input = bars(100);
        let resolved = policy()
            .resolve(&nifty(), &input)
            .unwrap_or_else(|_| unreachable_resolved());
        let ladders = resolved.ladders().unwrap_or_else(|_| ResolvedLaddersV1 {
            stops: Ladder::new(vec![1]).unwrap_or_default(),
            targets: Ladder::new(vec![1]).unwrap_or_default(),
            trails: Ladder::new(vec![1]).unwrap_or_default(),
        });
        let mut cells = Vec::new();
        assert!(resolved.visit_coordinates(|chosen| {
            cells.push(Cell {
                stop: chosen.stop,
                target: chosen.target,
                tsl: chosen.tsl,
                ttp: chosen.ttp,
                trades: 1,
                ..Cell::default()
            });
            true
        }));
        let grid = Grid {
            cells,
            signals: 1,
            stops: ladders.stops,
            targets: ladders.targets,
            trails: ladders.trails,
            refused_paths: 0,
        };
        let evaluated = resolved.evaluate_training_grid(
            &input,
            &test_column(&input),
            &ConditionMask::ZERO,
            Horizon::DEFAULT,
            resolved.side(),
        );
        assert!(evaluated.is_ok());
        let Some(mut evaluated) = evaluated.ok() else {
            return;
        };
        evaluated.grid = grid;
        let validated = resolved.validate_evaluation(&evaluated);
        assert!(validated.is_ok());
        let Some(validated) = validated.ok() else {
            return;
        };
        for (ordinal, cell) in validated.evaluated.grid.cells.iter().enumerate() {
            let coordinate = Chosen::from_cell(cell);
            assert_eq!(
                resolved
                    .canonical_coordinate_ordinal(&validated.coordinate_row_offsets, coordinate,),
                Ok(Some(ordinal)),
                "every canonical comparison row, including missing-axis rows, has one O(1) ordinal"
            );
            if coordinate.stop.is_none() || coordinate.target.is_none() {
                assert!(!resolved.chosen_is_in_bounds(coordinate));
                let disposition = resolved
                    .classify_coordinate(&validated, coordinate)
                    .expect("every canonical missing-axis row is a policy disposition");
                assert!(disposition.is_policy_refused());
                assert_eq!(
                    disposition
                        .refusal_bits()
                        .contains(ExecutionRefusalBitsV1::MISSING_STOP),
                    coordinate.stop.is_none()
                );
                assert_eq!(
                    disposition
                        .refusal_bits()
                        .contains(ExecutionRefusalBitsV1::MISSING_TARGET),
                    coordinate.target.is_none()
                );
                assert_eq!(
                    resolved.authorize_coordinate(&validated, coordinate),
                    Err(ExitGridErrorV1::InvalidChosenCoordinate),
                    "the legacy authorizer keeps treating undefined ratios as invalid"
                );
                continue;
            }
            assert_eq!(
                resolved
                    .authorize_coordinate(&validated, coordinate)
                    .map(|selected| *selected.training_cell()),
                Ok(*cell)
            );
        }
    }

    #[test]
    fn execution_refusal_bit_wire_values_are_stable_and_unknown_bits_refuse() {
        assert_eq!(ExecutionRefusalBitsV1::NONE.bits(), 0);
        assert_eq!(ExecutionRefusalBitsV1::MISSING_STOP.bits(), 1);
        assert_eq!(ExecutionRefusalBitsV1::MISSING_TARGET.bits(), 2);
        assert_eq!(ExecutionRefusalBitsV1::ZERO_TRADES.bits(), 4);
        assert_eq!(ExecutionRefusalBitsV1::AMBIGUITY_LIMIT.bits(), 8);
        assert_eq!(ExecutionRefusalBitsV1::GAP_LIMIT.bits(), 16);
        assert_eq!(ExecutionRefusalBitsV1::FORCED_STOP_MISMATCH.bits(), 32);
        assert_eq!(ExecutionRefusalBitsV1::ALL.bits(), 63);
        for bits in 0..=ExecutionRefusalBitsV1::ALL.bits() {
            assert_eq!(
                ExecutionRefusalBitsV1::from_bits(bits).map(ExecutionRefusalBitsV1::bits),
                Some(bits),
                "every subset of the six contiguous V1 bits must round-trip"
            );
        }
        assert_eq!(ExecutionRefusalBitsV1::from_bits(64), None);
        assert_eq!(ExecutionRefusalBitsV1::from_bits(u64::MAX), None);
    }

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "one table-driven test pins all five ordinary reasons, their union, and the authorized opposite on one immutable validation capability"
    )]
    fn execution_disposition_reports_each_single_reason_and_combines_reasons() {
        let mut cfg = policy();
        cfg.max_ambiguous_bars = 0;
        cfg.max_gap_fills = 0;
        let input = bars(100);
        let resolved = cfg
            .resolve(&nifty(), &input)
            .expect("the policy resolves over the deterministic training bars");
        let mut evaluated = canonical_evaluation(&resolved, &input);

        let missing_stop = evaluated
            .grid
            .cells
            .iter()
            .find(|cell| cell.stop.is_none() && cell.target.is_some() && cell.ttp.is_none())
            .map(Chosen::from_cell)
            .expect("the complete grid contains a stop-less comparison row");
        let missing_target = evaluated
            .grid
            .cells
            .iter()
            .find(|cell| cell.stop.is_some() && cell.target.is_none() && cell.ttp.is_none())
            .map(Chosen::from_cell)
            .expect("the complete grid contains a target-less comparison row");
        let complete: Vec<Chosen> = evaluated
            .grid
            .cells
            .iter()
            .filter(|cell| cell.stop.is_some() && cell.target.is_some() && cell.ttp.is_none())
            .map(Chosen::from_cell)
            .take(4)
            .collect();
        assert_eq!(complete.len(), 4, "the fixture needs four two-sided rows");
        let [zero_trades, ambiguity, gap, authorized] =
            <[Chosen; 4]>::try_from(complete).expect("the fixture has four retained coordinates");
        evaluated
            .grid
            .cells
            .iter_mut()
            .find(|cell| Chosen::from_cell(cell) == zero_trades)
            .expect("zero-trade coordinate exists")
            .trades = 0;
        evaluated
            .grid
            .cells
            .iter_mut()
            .find(|cell| Chosen::from_cell(cell) == ambiguity)
            .expect("ambiguity coordinate exists")
            .ambiguous_bars = 1;
        evaluated
            .grid
            .cells
            .iter_mut()
            .find(|cell| Chosen::from_cell(cell) == gap)
            .expect("gap coordinate exists")
            .gapped = 1;
        let multi = evaluated
            .grid
            .cells
            .iter()
            .find(|cell| {
                cell.stop.is_none()
                    && cell.target.is_none()
                    && cell.tsl.is_none()
                    && cell.ttp.is_none()
            })
            .map(Chosen::from_cell)
            .expect("the complete grid contains the baseline row");
        let multi_cell = evaluated
            .grid
            .cells
            .iter_mut()
            .find(|cell| Chosen::from_cell(cell) == multi)
            .expect("baseline coordinate exists");
        multi_cell.trades = 0;
        multi_cell.ambiguous_bars = 1;
        multi_cell.gapped = 1;

        let validated = resolved
            .validate_evaluation(&evaluated)
            .expect("the complete canonical population validates once");
        for (coordinate, expected) in [
            (missing_stop, ExecutionRefusalBitsV1::MISSING_STOP),
            (missing_target, ExecutionRefusalBitsV1::MISSING_TARGET),
            (zero_trades, ExecutionRefusalBitsV1::ZERO_TRADES),
            (ambiguity, ExecutionRefusalBitsV1::AMBIGUITY_LIMIT),
            (gap, ExecutionRefusalBitsV1::GAP_LIMIT),
        ] {
            let disposition = resolved
                .classify_coordinate(&validated, coordinate)
                .expect("a canonical policy refusal is a terminal disposition");
            assert!(disposition.is_policy_refused());
            assert!(!disposition.is_authorized());
            assert_eq!(disposition.selected(), None);
            assert_eq!(disposition.refusal_bits(), expected);
            assert_eq!(disposition.coordinate(), coordinate);
            assert_eq!(disposition.resolution_digest(), resolved.digest());
            assert_eq!(disposition.run_id(), evaluated.run_id());
            assert_eq!(disposition.mask(), evaluated.mask());
            assert_eq!(disposition.horizon(), evaluated.horizon());
            assert_eq!(disposition.column_digest(), evaluated.column_digest);
            assert_eq!(
                disposition.evaluation_spec_fingerprint(),
                evaluated.evaluation_spec.fingerprint_v1().into_bytes()
            );
            assert_eq!(disposition.side(), evaluated.side());
            assert_ne!(disposition.context_digest(), [0; 32]);
            assert_ne!(disposition.disposition_digest(), [0; 32]);
        }

        let expected_multi = ExecutionRefusalBitsV1::MISSING_STOP
            .union(ExecutionRefusalBitsV1::MISSING_TARGET)
            .union(ExecutionRefusalBitsV1::ZERO_TRADES)
            .union(ExecutionRefusalBitsV1::AMBIGUITY_LIMIT)
            .union(ExecutionRefusalBitsV1::GAP_LIMIT);
        let multi_disposition = resolved
            .classify_coordinate(&validated, multi)
            .expect("the baseline's independent policy reasons are all retained");
        assert_eq!(multi_disposition.refusal_bits(), expected_multi);
        assert!(multi_disposition.refusal_bits().contains(
            ExecutionRefusalBitsV1::MISSING_STOP.union(ExecutionRefusalBitsV1::ZERO_TRADES)
        ));

        let authorized_disposition = resolved
            .classify_coordinate(&validated, authorized)
            .expect("the clean canonical coordinate authorizes");
        assert!(authorized_disposition.is_authorized());
        assert!(!authorized_disposition.is_policy_refused());
        assert!(authorized_disposition.refusal_bits().is_empty());
        let selected = authorized_disposition
            .selected()
            .expect("authorized disposition carries the opaque selected exit");
        assert_eq!(selected.coordinate(), authorized);
        assert_eq!(selected.run_id(), evaluated.run_id());
        assert_eq!(authorized_disposition.coordinate(), authorized);
        assert_eq!(
            authorized_disposition.resolution_digest(),
            resolved.digest()
        );
        assert_eq!(authorized_disposition.run_id(), selected.run_id());
        assert_eq!(authorized_disposition.mask(), evaluated.mask());
        assert_eq!(authorized_disposition.horizon(), evaluated.horizon());
        assert_eq!(
            authorized_disposition.column_digest(),
            evaluated.column_digest
        );
        assert_eq!(authorized_disposition.side(), evaluated.side());
        assert_ne!(authorized_disposition.context_digest(), [0; 32]);
        assert_ne!(authorized_disposition.disposition_digest(), [0; 32]);
    }

    #[test]
    fn refusal_identity_binds_coordinate_run_and_the_complete_evaluated_grid() {
        let input = bars(100);
        let resolved = policy()
            .resolve(&nifty(), &input)
            .expect("the deterministic training policy resolves");
        let evaluated = canonical_evaluation(&resolved, &input);
        let coordinates: Vec<Chosen> = evaluated
            .grid
            .cells
            .iter()
            .filter(|cell| cell.stop.is_none() && cell.target.is_some() && cell.ttp.is_none())
            .map(Chosen::from_cell)
            .take(2)
            .collect();
        let [first_coordinate, second_coordinate] = <[Chosen; 2]>::try_from(coordinates)
            .expect("the complete population has two stop-less rows");
        let validated = resolved
            .validate_evaluation(&evaluated)
            .expect("the canonical population validates");
        let first = resolved
            .classify_coordinate(&validated, first_coordinate)
            .expect("the first canonical refusal classifies");
        let second = resolved
            .classify_coordinate(&validated, second_coordinate)
            .expect("the second canonical refusal classifies");
        assert_eq!(first.refusal_bits(), ExecutionRefusalBitsV1::MISSING_STOP);
        assert_eq!(first.refusal_bits(), second.refusal_bits());
        assert_ne!(first.coordinate(), second.coordinate());
        assert_ne!(first.context_digest(), second.context_digest());
        assert_ne!(first.disposition_digest(), second.disposition_digest());

        let changed_mask = ConditionMask::ZERO.with_bit(0);
        let changed_run_evaluation = canonical_evaluation_for_mask(&resolved, &input, changed_mask);
        let changed_run_validation = resolved
            .validate_evaluation(&changed_run_evaluation)
            .expect("the changed-mask population validates under its own run");
        let changed_run = resolved
            .classify_coordinate(&changed_run_validation, first_coordinate)
            .expect("the same canonical refusal classifies under the changed run");
        assert_eq!(changed_run.refusal_bits(), first.refusal_bits());
        assert_eq!(changed_run.coordinate(), first.coordinate());
        assert_eq!(changed_run.resolution_digest(), first.resolution_digest());
        assert_ne!(changed_run.run_id(), first.run_id());
        assert_eq!(changed_run.mask(), changed_mask);
        assert_ne!(changed_run.context_digest(), first.context_digest());
        assert_ne!(changed_run.disposition_digest(), first.disposition_digest());

        let mut changed_grid_evaluation = evaluated.clone();
        let other_cell = changed_grid_evaluation
            .grid
            .cells
            .iter_mut()
            .find(|cell| Chosen::from_cell(cell) != first_coordinate)
            .expect("the complete population contains another row");
        other_cell.pessimistic = other_cell.pessimistic.saturating_add(1);
        let changed_grid_validation = resolved
            .validate_evaluation(&changed_grid_evaluation)
            .expect("the measurement-only grid change remains structurally valid");
        let changed_grid = resolved
            .classify_coordinate(&changed_grid_validation, first_coordinate)
            .expect("the unchanged canonical refusal classifies in the changed grid");
        assert_eq!(changed_grid.refusal_bits(), first.refusal_bits());
        assert_eq!(changed_grid.coordinate(), first.coordinate());
        assert_eq!(changed_grid.run_id(), first.run_id());
        assert_ne!(changed_grid.context_digest(), first.context_digest());
        assert_ne!(
            changed_grid.disposition_digest(),
            first.disposition_digest()
        );
    }

    #[test]
    fn forced_stop_reason_is_independent_and_legacy_authorization_is_unchanged() {
        let mut cfg = policy();
        cfg.forced_stop = ForcedStopV1::RequireExactObserved(25);
        cfg.max_ambiguous_bars = 0;
        cfg.max_gap_fills = 0;
        let input = bars(100);
        let resolved = cfg
            .resolve(&nifty(), &input)
            .expect("the exact observed stop resolves");
        let mut evaluated = canonical_evaluation(&resolved, &input);
        let forced_stop_index = resolved
            .forced_stop_index()
            .expect("the required stop has a resolved ordinal");
        let mismatch = evaluated
            .grid
            .cells
            .iter()
            .find(|cell| {
                cell.stop.is_some_and(|stop| stop != forced_stop_index)
                    && cell.target.is_some()
                    && cell.ttp.is_none()
            })
            .map(Chosen::from_cell)
            .expect("the grid retains a non-forced comparison coordinate");
        let baseline = evaluated
            .grid
            .cells
            .iter()
            .find(|cell| {
                cell.stop.is_none()
                    && cell.target.is_none()
                    && cell.tsl.is_none()
                    && cell.ttp.is_none()
            })
            .map(Chosen::from_cell)
            .expect("the complete grid retains its baseline");
        let baseline_cell = evaluated
            .grid
            .cells
            .iter_mut()
            .find(|cell| Chosen::from_cell(cell) == baseline)
            .expect("baseline coordinate exists");
        baseline_cell.trades = 0;
        baseline_cell.ambiguous_bars = 1;
        baseline_cell.gapped = 1;

        let validated = resolved
            .validate_evaluation(&evaluated)
            .expect("the canonical forced-stop population validates");
        let mismatch_disposition = resolved
            .classify_coordinate(&validated, mismatch)
            .expect("a canonical forced-stop mismatch is a policy disposition");
        assert_eq!(
            mismatch_disposition.refusal_bits(),
            ExecutionRefusalBitsV1::FORCED_STOP_MISMATCH
        );
        let all_reasons = resolved
            .classify_coordinate(&validated, baseline)
            .expect("all six policy reasons can coexist on one canonical row");
        assert_eq!(all_reasons.refusal_bits(), ExecutionRefusalBitsV1::ALL);

        assert_eq!(
            resolved.authorize_coordinate(&validated, mismatch),
            Err(ExitGridErrorV1::ChosenCellNotAdmitted),
            "the compatibility authorizer keeps its measured-policy error"
        );
        assert_eq!(
            resolved.authorize_coordinate(&validated, baseline),
            Err(ExitGridErrorV1::InvalidChosenCoordinate),
            "the compatibility authorizer keeps its missing-axis error"
        );
    }

    #[test]
    fn structural_errors_precede_and_never_become_policy_refusals() {
        let mut cfg = policy();
        cfg.ratios =
            RatioLimitsV1::new(200, 300, 16).expect("the restrictive ratio interval is valid");
        let input = bars(100);
        let resolved = cfg
            .resolve(&nifty(), &input)
            .expect("at least one exact ratio pair survives");
        let evaluated = canonical_evaluation(&resolved, &input);
        let malformed = Chosen {
            stop: None,
            target: None,
            tsl: None,
            ttp: Some(crate::grid::Ttp {
                arm: usize::MAX,
                trail: 0,
            }),
        };

        let mut foreign = resolved
            .validate_evaluation(&evaluated)
            .expect("the canonical evaluation validates");
        foreign.resolution_digest = [0; 32];
        assert_eq!(
            resolved.classify_coordinate(&foreign, malformed),
            Err(ExitGridErrorV1::EvaluationResolutionMismatch),
            "foreign authority fails before even a malformed coordinate"
        );

        let validated = resolved
            .validate_evaluation(&evaluated)
            .expect("the canonical evaluation validates again");
        assert_eq!(
            resolved.classify_coordinate(&validated, malformed),
            Err(ExitGridErrorV1::InvalidChosenCoordinate),
            "missing axes do not hide a structurally impossible armed trail"
        );
        assert_eq!(
            resolved.classify_coordinate(
                &validated,
                Chosen {
                    stop: Some(usize::MAX),
                    target: None,
                    tsl: None,
                    ttp: None,
                },
            ),
            Err(ExitGridErrorV1::InvalidChosenCoordinate),
            "an out-of-range Some index is corruption, not missing-stop policy"
        );

        let disallowed_pair = (0..resolved.stop_levels_ppm().len())
            .flat_map(|stop| {
                (0..resolved.target_levels_ppm().len()).map(move |target| (stop, target))
            })
            .find(|&(stop, target)| !resolved.ratio_pair_admitted(stop, target))
            .expect("the restrictive interval excludes at least one exact pair");
        assert_eq!(
            resolved.classify_coordinate(
                &validated,
                Chosen {
                    stop: Some(disallowed_pair.0),
                    target: Some(disallowed_pair.1),
                    tsl: None,
                    ttp: None,
                },
            ),
            Err(ExitGridErrorV1::InvalidChosenCoordinate),
            "a ratio-filtered coordinate is absent from the population"
        );

        let first_coordinate = evaluated
            .grid
            .cells
            .first()
            .map(Chosen::from_cell)
            .expect("the evaluated population is non-empty");
        let mut torn_offsets = resolved
            .validate_evaluation(&evaluated)
            .expect("the canonical evaluation validates before the test tear");
        let first_offset = torn_offsets
            .coordinate_row_offsets
            .iter_mut()
            .flatten()
            .next()
            .expect("the population has an admitted row offset");
        *first_offset = first_offset.saturating_add(1);
        assert_eq!(
            resolved.classify_coordinate(&torn_offsets, first_coordinate),
            Err(ExitGridErrorV1::ReplayCoordinatePopulationMismatch),
            "a changed O(1) index cannot be laundered into a policy refusal"
        );
    }

    fn brute_cell_count(stops: usize, targets: usize, trails: usize, pairs: &[RatioPairV1]) -> u64 {
        let mut count = 0_u64;
        for stop_setting in 0..=stops {
            for target_setting in 0..=targets {
                if stop_setting < stops
                    && target_setting < targets
                    && !pairs.contains(&RatioPairV1 {
                        stop_index: stop_setting,
                        target_index: target_setting,
                    })
                {
                    continue;
                }
                for trail_setting in 0..=trails {
                    count = count.saturating_add(1);
                    let cap = if trail_setting < trails {
                        trail_setting
                    } else {
                        trails
                    };
                    let armed = target_setting.saturating_mul(cap);
                    count = count.saturating_add(u64::try_from(armed).unwrap_or(u64::MAX));
                }
            }
        }
        count
    }

    #[test]
    fn the_complete_training_grid_has_one_reachable_exact_enumerator() {
        let input = bars(100);
        let resolved = policy()
            .resolve(&nifty(), &input)
            .unwrap_or_else(|_| unreachable_resolved());
        let column = test_column(&input);
        let evaluated_result = resolved.evaluate_training_grid(
            &input,
            &column,
            &vocab::ConditionMask::ZERO,
            Horizon::DEFAULT,
            crate::excursion::Side::Long,
        );
        assert!(evaluated_result.is_ok());
        let Some(evaluated) = evaluated_result.ok() else {
            return;
        };
        let grid = evaluated.grid();
        assert_eq!(
            u64::try_from(grid.cells.len()).unwrap_or(u64::MAX),
            resolved.cell_count()
        );
        assert_eq!(resolved.select(&evaluated), Ok(None));

        let mut changed = input.clone();
        if let Some(bar) = changed.first_mut() {
            bar.volume = bar.volume.saturating_add(1);
        }
        assert_eq!(
            resolved.evaluate_training_grid(
                &changed,
                &test_column(&changed),
                &vocab::ConditionMask::ZERO,
                Horizon::DEFAULT,
                crate::excursion::Side::Long,
            ),
            Err(ExitGridErrorV1::TrainingSeriesMismatch)
        );
        assert_eq!(
            resolved.evaluate_training_grid(
                &input,
                &column,
                &vocab::ConditionMask::ZERO,
                Horizon::DEFAULT,
                crate::excursion::Side::Short,
            ),
            Err(ExitGridErrorV1::RunIdentityMismatch("direction"))
        );
    }

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "one adversarial replay table keeps every malformed minute shape beside the clean control it must not affect"
    )]
    fn oos_replay_refuses_every_non_one_minute_or_corrupt_shape_before_pricing() {
        let training = bars(100);
        let instrument = nifty();
        let resolved = policy()
            .resolve(&instrument, &training)
            .unwrap_or_else(|_| unreachable_resolved());
        let selected = selected_for(&resolved, &training);
        assert!(selected.is_some());
        let Some(selected) = selected else {
            return;
        };
        let replay = |candidate: &[Candle]| -> Result<ReplayedExitV1, ExitGridErrorV1> {
            let series = ExecutionSeriesV1::new(
                &instrument,
                "test-feed",
                "test-commit",
                [0xA5; 32],
                candidate,
            )?;
            let oos = OosExecutionSeriesV1::new(series, 0)?;
            resolved.replay_selected(
                oos,
                &test_column(candidate),
                &selected,
                test_execution_run(
                    &instrument,
                    &ConditionMask::ZERO,
                    crate::excursion::Side::Long,
                    candidate,
                )?,
            )
        };
        assert_eq!(
            replay(&[]),
            Err(ExitGridErrorV1::InvalidOosBoundary {
                first_oos: 0,
                bars: 0,
            })
        );

        let mut off_minute = shifted_bars(3, 1);
        if let Some(bar) = off_minute.get_mut(1) {
            bar.ts_micros = bar.ts_micros.saturating_add(1);
        }
        assert!(matches!(
            replay(&off_minute),
            Err(ExitGridErrorV1::OffMinuteTimestamp { .. })
        ));

        let mut duplicate = shifted_bars(3, 1);
        let first_stamp = duplicate.first().map_or(0, |bar| bar.ts_micros);
        if let Some(bar) = duplicate.get_mut(1) {
            bar.ts_micros = first_stamp;
        }
        assert!(matches!(
            replay(&duplicate),
            Err(ExitGridErrorV1::NonIncreasingTimestamp { .. })
        ));

        let mut corrupt = shifted_bars(3, 1);
        if let Some(bar) = corrupt.get_mut(1) {
            bar.high = bar.low.saturating_sub(1);
        }
        assert!(matches!(
            replay(&corrupt),
            Err(ExitGridErrorV1::CorruptExecutionCandle { .. })
        ));

        let mut whole_minute_gap = shifted_bars(3, 1);
        if let Some(bar) = whole_minute_gap.get_mut(2) {
            bar.ts_micros = bar.ts_micros.saturating_add(ONE_MINUTE_MICROS);
        }
        assert_eq!(
            replay(&whole_minute_gap).map(|value| value.cell().copied()),
            Ok(None),
            "a missing whole minute remains absent; no neighbour or fabricated tick substitutes"
        );

        let clean = shifted_bars(3, 1);
        assert_eq!(replay(&clean).map(|value| value.cell().copied()), Ok(None));
        let mut wrong_side = selected.clone();
        wrong_side.side = crate::excursion::Side::Short;
        let series =
            ExecutionSeriesV1::new(&instrument, "test-feed", "test-commit", [0xA5; 32], &clean);
        assert!(series.is_ok());
        let Some(series) = series.ok() else {
            return;
        };
        let oos = OosExecutionSeriesV1::new(series, 0);
        assert!(oos.is_ok());
        let Some(oos) = oos.ok() else {
            return;
        };
        let run = test_execution_run(
            &instrument,
            &ConditionMask::ZERO,
            crate::excursion::Side::Long,
            &clean,
        );
        assert!(run.is_ok());
        let Some(run) = run.ok() else {
            return;
        };
        assert_eq!(
            resolved.replay_selected(oos, &test_column(&clean), &wrong_side, run,),
            Err(ExitGridErrorV1::SelectionDigestMismatch)
        );

        let mut thresholds = Thresholds::CLASSICAL;
        thresholds.long_body = thresholds.long_body.saturating_add(1);
        let changed_column = test_column_with_thresholds(&clean, thresholds);
        let changed_spec = changed_column.evaluation_spec_token();
        assert!(changed_spec.is_some());
        let Some(changed_spec) = changed_spec else {
            return;
        };
        let mut wrong_spec = selected;
        wrong_spec.evaluation_spec = changed_spec;
        assert_eq!(
            resolved.replay_selected(oos, &test_column(&clean), &wrong_spec, run),
            Err(ExitGridErrorV1::SelectionDigestMismatch)
        );
    }

    #[test]
    fn candidate_universe_digest_binds_every_path_and_legacy_replay_stays_strict() {
        let training = crate::synthetic::sessions(6);
        let instrument = nifty();
        let resolved = policy()
            .resolve(&instrument, &training)
            .unwrap_or_else(|_| unreachable_resolved());
        let selected = selected_for(&resolved, &training)
            .expect("the fixture seals one policy-admitted training coordinate");

        let mut execution = crate::synthetic::sessions(6);
        for bar in &mut execution {
            bar.ts_micros = bar
                .ts_micros
                .saturating_add(8_i64.saturating_mul(crate::synthetic::DAY_MICROS));
            bar.high = bar.open;
            bar.low = bar.open;
            bar.close = bar.open;
        }
        let execution_column = test_column(&execution);
        assert!(
            !execution_column.bits().is_empty(),
            "the replay fixture must cross the evaluator's real warm-up boundary"
        );
        let series = ExecutionSeriesV1::new(
            &instrument,
            "test-feed",
            "test-commit",
            [0xA5; 32],
            &execution,
        )
        .expect("complete source identity");
        let oos = OosExecutionSeriesV1::new(series, 0).expect("the first bar begins OOS");
        let run = test_execution_run(
            &instrument,
            &ConditionMask::ZERO,
            resolved.side(),
            &execution,
        )
        .expect("the canonical OOS run");
        let universe = resolved
            .replay_selected_universe(oos, &execution_column, &selected, run)
            .expect("pricing refusals remain conservative occupancy evidence");

        assert!(universe.digest_is_valid());
        assert_eq!(universe.require_integrity(), Ok(()));
        assert!(!universe.candidates().is_empty());
        assert_eq!(
            universe.pricing_refused_paths(),
            u64::try_from(
                universe
                    .candidates()
                    .iter()
                    .filter(|candidate| candidate.pricing_refused())
                    .count()
            )
            .unwrap_or(u64::MAX),
            "the sealed refusal count reconciles with the ordered evidence"
        );
        for candidate in universe.candidates() {
            assert_eq!(
                execution
                    .get(candidate.entry_bar())
                    .map(|bar| bar.ts_micros),
                Some(candidate.entry_micros()),
                "entry timestamps come from the exact execution bar"
            );
            assert_eq!(
                execution
                    .get(candidate.occupied_through_bar())
                    .map(|bar| bar.ts_micros),
                Some(candidate.occupied_through_micros()),
                "occupied-through timestamps come from the exact execution bar"
            );
        }

        if universe.pricing_refused_paths() > 0 {
            assert_eq!(
                resolved.replay_selected(oos, &execution_column, &selected, run),
                Err(ExitGridErrorV1::RefusedExecutionPaths {
                    paths: universe.pricing_refused_paths(),
                }),
                "the compatibility replay delegates to the universe and retains its fail-closed refusal"
            );
        }

        let mut torn = universe;
        let first = torn
            .candidates
            .first_mut()
            .expect("the fixture produced candidate evidence");
        first.occupied_through_micros = first.occupied_through_micros.saturating_add(1);
        assert!(!torn.digest_is_valid());
        assert_eq!(
            torn.require_integrity(),
            Err(ExitGridErrorV1::ReplayEvidenceDigestMismatch)
        );
    }

    #[test]
    fn global_replay_witness_mints_every_identity_at_the_authenticated_replay_door() {
        let training = crate::synthetic::sessions(6);
        let instrument = nifty();
        let training_series = ExecutionSeriesV1::new(
            &instrument,
            Vendor::Zerodha.as_str(),
            "test-commit",
            [0xA5; 32],
            &training,
        )
        .expect("canonical stored training source");
        let resolved = policy()
            .resolve_attested(training_series)
            .expect("canonical stored training resolution");
        let selected = selected_for_feed(&resolved, &training, Vendor::Zerodha.as_str())
            .expect("the fixture seals one policy-admitted training coordinate");

        let mut execution = crate::synthetic::sessions(6);
        for bar in &mut execution {
            bar.ts_micros = bar
                .ts_micros
                .saturating_add(8_i64.saturating_mul(crate::synthetic::DAY_MICROS));
            bar.high = bar.open;
            bar.low = bar.open;
            bar.close = bar.open;
        }
        let column = test_column(&execution);
        let stored_series = ExecutionSeriesV1::new(
            &instrument,
            Vendor::Zerodha.as_str(),
            "test-commit",
            [0xA5; 32],
            &execution,
        )
        .expect("canonical stored OOS source");
        let oos = OosExecutionSeriesV1::new(stored_series, 0).expect("OOS boundary");
        let run = test_execution_run_for_feed(
            &instrument,
            &ConditionMask::ZERO,
            resolved.side(),
            &execution,
            Vendor::Zerodha.as_str(),
        )
        .expect("canonical stored OOS run");
        let witness = resolved
            .replay_global_witness(oos, &column, &selected, run)
            .expect("authenticated global replay witness");

        assert_eq!(witness.require_integrity(), Ok(()));
        assert_eq!(witness.instrument(), instrument);
        assert_eq!(witness.feed(), Vendor::Zerodha);
        assert_eq!(witness.direction(), CostDirection::Long);
        assert_eq!(witness.first_oos(), 0);
        assert_eq!(witness.run_id(), run.run_id());
        assert_eq!(witness.selected_exit_digest(), selected.selection_digest);
        assert!(!witness.candidates().is_empty());
        assert_ne!(witness.universe_digest(), [0; 32]);

        let mut torn = witness;
        torn.first_oos = 1;
        assert_eq!(
            torn.require_integrity(),
            Err(ExitGridErrorV1::ReplayEvidenceDigestMismatch),
            "the OOS boundary is part of the opaque successor seal"
        );

        let unknown_series = ExecutionSeriesV1::new(
            &instrument,
            "not-a-stored-vendor",
            "test-commit",
            [0xA5; 32],
            &execution,
        )
        .expect("an attested but non-canonical feed can reach the refusal boundary");
        let unknown_oos =
            OosExecutionSeriesV1::new(unknown_series, 0).expect("unknown-feed OOS boundary");
        let unknown_run = test_execution_run_for_feed(
            &instrument,
            &ConditionMask::ZERO,
            resolved.side(),
            &execution,
            "not-a-stored-vendor",
        )
        .expect("unknown-feed run remains internally self-consistent");
        assert!(
            matches!(
                resolved.replay_global_witness(unknown_oos, &column, &selected, unknown_run),
                Err(ExitGridErrorV1::RunIdentityMismatch(
                    "feed is not a canonical stored vendor"
                ))
            ),
            "an arbitrary feed string is never relabelled as stored vendor authority"
        );
    }

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "one capability test mutates completeness, admission, ladder, coordinate and resolution authority around the same evaluated grid"
    )]
    fn exact_ladders_and_quality_limits_drive_selection_without_oos_derivation() {
        let mut cfg = policy();
        cfg.max_ambiguous_bars = 0;
        cfg.max_gap_fills = 0;
        cfg.selector = ExitGridSelectorV1::PessimisticTotal;
        let input = bars(100);
        let resolved = cfg
            .resolve(&nifty(), &input)
            .unwrap_or_else(|_| unreachable_resolved());
        let ladders = resolved.ladders().unwrap_or_else(|_| ResolvedLaddersV1 {
            stops: Ladder::new(vec![1]).unwrap_or_default(),
            targets: Ladder::new(vec![1]).unwrap_or_default(),
            trails: Ladder::new(vec![1]).unwrap_or_default(),
        });
        assert_eq!(ladders.stops.rungs(), resolved.stop_levels_ppm());
        assert_eq!(ladders.targets.rungs(), resolved.target_levels_ppm());
        assert_eq!(ladders.trails.rungs(), resolved.trail_levels_ppm());

        let mut cells = Vec::new();
        assert!(resolved.visit_coordinates(|chosen| {
            cells.push(Cell {
                stop: chosen.stop,
                target: chosen.target,
                tsl: chosen.tsl,
                ttp: chosen.ttp,
                trades: 10,
                ..Cell::default()
            });
            true
        }));
        if let Some(slot) = cells.first_mut() {
            slot.trades = 10;
            slot.wins = 6;
            slot.pessimistic = 100;
        }
        if let Some(slot) = cells.get_mut(1) {
            slot.trades = 10;
            slot.wins = 10;
            slot.pessimistic = 1_000;
            slot.ambiguous_bars = 1;
        }
        let clean = cells.first().copied().unwrap_or_default();
        let grid = Grid {
            cells,
            signals: 10,
            stops: ladders.stops,
            targets: ladders.targets,
            trails: ladders.trails,
            refused_paths: 0,
        };
        let evaluated = resolved.evaluate_training_grid(
            &input,
            &test_column(&input),
            &ConditionMask::ZERO,
            Horizon::DEFAULT,
            resolved.side(),
        );
        assert!(evaluated.is_ok());
        let Some(mut evaluated) = evaluated.ok() else {
            return;
        };
        evaluated.grid = grid;
        assert_eq!(
            resolved
                .select(&evaluated)
                .map(|selected| selected.map(|value| *value.training_cell())),
            Ok(Some(clean))
        );
        let clean_coordinate = Chosen::from_cell(&clean);
        let validated = resolved.validate_evaluation(&evaluated);
        assert!(validated.is_ok());
        let Some(validated) = validated.ok() else {
            return;
        };
        assert_eq!(
            resolved
                .authorize_coordinate(&validated, clean_coordinate)
                .map(|selected| *selected.training_cell()),
            Ok(clean)
        );
        let rejected_coordinate = evaluated.grid.cells.get(1).map(Chosen::from_cell);
        assert!(rejected_coordinate.is_some());
        if let Some(rejected_coordinate) = rejected_coordinate {
            assert_eq!(
                resolved.authorize_coordinate(&validated, rejected_coordinate),
                Err(ExitGridErrorV1::ChosenCellNotAdmitted)
            );
        }
        assert_eq!(
            resolved.authorize_coordinate(
                &validated,
                Chosen {
                    stop: Some(usize::MAX),
                    target: clean_coordinate.target,
                    tsl: clean_coordinate.tsl,
                    ttp: clean_coordinate.ttp,
                },
            ),
            Err(ExitGridErrorV1::InvalidChosenCoordinate)
        );

        let mut duplicated = evaluated.clone();
        let first_coordinate = duplicated.grid.cells.first().map(Chosen::from_cell);
        if let (Some(first), Some(second)) = (first_coordinate, duplicated.grid.cells.get_mut(1)) {
            second.stop = first.stop;
            second.target = first.target;
            second.tsl = first.tsl;
            second.ttp = first.ttp;
        }
        assert_eq!(
            resolved.select(&duplicated),
            Err(ExitGridErrorV1::ReplayCoordinatePopulationMismatch)
        );
        assert!(matches!(
            resolved.validate_evaluation(&duplicated),
            Err(ExitGridErrorV1::ReplayCoordinatePopulationMismatch)
        ));

        let wrong_offset = resolved.validate_evaluation(&evaluated);
        assert!(wrong_offset.is_ok());
        let Some(mut wrong_offset) = wrong_offset.ok() else {
            return;
        };
        if let Some(Some(offset)) = wrong_offset.coordinate_row_offsets.first_mut() {
            *offset = offset.saturating_add(1);
        }
        assert_eq!(
            resolved.authorize_coordinate(&wrong_offset, clean_coordinate),
            Err(ExitGridErrorV1::ReplayCoordinatePopulationMismatch),
            "a changed canonical ordinal must not authorize the adjacent cell"
        );

        let mut wrong = evaluated.clone();
        wrong.grid.stops = Ladder::new(vec![999]).unwrap_or_default();
        assert_eq!(
            resolved.select(&wrong),
            Err(ExitGridErrorV1::ReplayLadderMismatch)
        );

        let mut foreign = evaluated;
        foreign.resolution_digest = [0; 32];
        assert_eq!(
            resolved.select(&foreign),
            Err(ExitGridErrorV1::EvaluationResolutionMismatch)
        );
    }

    #[test]
    fn invalid_policy_shapes_and_unsupported_duration_are_named() {
        assert_eq!(
            RationalPercentileV1::new(0, 1),
            Err(ExitGridErrorV1::InvalidPercentile {
                numerator: 0,
                denominator: 1
            })
        );
        assert_eq!(
            RungPlanV1::new(vec![], vec![p(1, 1)], vec![p(1, 1)], 1),
            Err(ExitGridErrorV1::EmptyPercentiles("stop"))
        );
        assert_eq!(
            RungPlanV1::new(vec![p(1, 2), p(1, 3)], vec![p(1, 1)], vec![p(1, 1)], 2),
            Err(ExitGridErrorV1::PercentilesNotIncreasing("stop"))
        );
        assert!(matches!(
            ExitGridPolicyV1::new(
                ExecutionResolutionV1::OneMinuteOhlcv,
                RangeResolutionV1::PpmCeiling,
                crate::excursion::Side::Long,
                unreachable_policy_rungs(),
                RatioLimitsV1 {
                    min_hundredths: 100,
                    max_hundredths: 100,
                    max_pairs: 1,
                },
                1,
                ExitGridSelectorV1::PessimisticTotal,
                [0; 32],
                ForcedStopV1::Disabled,
                0,
                0
            ),
            Err(ExitGridErrorV1::MissingCostModelId)
        ));
        let mut unsupported = policy();
        unsupported.execution_resolution = ExecutionResolutionV1::UnsupportedSeconds(5);
        assert_eq!(
            unsupported.resolve(&nifty(), &bars(100)),
            Err(ExitGridErrorV1::UnsupportedExecutionResolution(5))
        );
    }
}

//! Receipt-last authority for reconstructing a selected exit without defaults.
//!
//! A population row records a mask, side and exit coordinate, but its historical
//! fixed layout does not carry the [`Horizon`] or the runtime policy which made
//! that coordinate meaningful.  A digest cannot be decoded back into either
//! value.  This module supplies a separate append-only capability rather than
//! changing or reinterpreting the population format.
//!
//! Policy scalar records and percentile atoms have separate fixed strides.  The
//! split is deliberate: [`RungPlanV1`] is runtime-sized, so embedding a guessed
//! maximum number of rungs in a record would silently narrow the dynamic policy.
//! One row capability then binds the exact population payload to the training
//! run and opaque [`SelectedExitV1`] identities.  A caller can reconstruct the
//! latter only by re-resolving the recorded policy against the exact training
//! one-minute OHLCV, rebuilding the exact evaluator column and run, validating
//! the complete grid, and authorizing the population row's coordinate.
//!
//! # Cost
//!
//! Decoding or looking up one already-indexed capability is average O(1).
//! Opening a durable ledger, hashing its blocks, rebuilding an exit grid and
//! evaluating its complete coordinate population are linear in their respective
//! stored records, training bars and grid cells.  No full replay or persistence
//! operation is claimed O(1).
//!
//! **UNVERIFIED as a measured bound.** No bench in this workspace
//! times this, so the shape above is read from the source rather
//! than measured. `CLAUDE.md` §3 rule 6.

use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::MetadataExt as _;
#[cfg(windows)]
use std::os::windows::fs::MetadataExt as _;

use brutex_core::blake3::Hasher;
use indicators::column::{Column, EVALUATION_SPEC_FINGERPRINT_V1_LEN, EvaluationSpecFingerprintV1};
use runner::excursion::Side;
use runner::exit_grid_policy::{
    ExecutionResolutionV1, ExecutionRunV1, ExecutionSeriesV1, ExitGridPolicyV1, ExitGridSelectorV1,
    ForcedStopV1, RangeResolutionV1, RatioLimitsV1, RationalPercentileV1, ResolvedExitGridV1,
    RungPlanV1, SelectedExitV1, printed_ohlcv_cost_model_id_v1,
};
use runner::grid::{Chosen, Ttp};
use runner::identity::Params;
use runner::outcome::{FORCED_EXIT_MINUTE, Horizon};

use crate::population::{
    CompletionReceiptV2, CompletionReceiptV4, InstrumentFamilyV1, PopulationLedger,
    PopulationRowV1, TradeDirectionV1,
};

/// Operator-facing refusal from this authority.
pub type ExecutionCapabilityRefusal = String;

const HEADER_BYTES: u64 = 24;
const HEADER_BYTES_USIZE: usize = 24;
const FORMAT_VERSION: u32 = 1;
const SEAL_BYTES: usize = 32;
const PARAMETER_MAGIC: [u8; 8] = *b"BRUTXEP1";
const PERCENTILE_MAGIC: [u8; 8] = *b"BRUTXEG1";
const CAPABILITY_MAGIC: [u8; 8] = *b"BRUTXEC1";
const COMPLETION_MAGIC: [u8; 8] = *b"BRUTXEF1";
// 616, NOT 608. The payload carries the evaluation-spec fingerprint, and that
// widened 155 -> 163 when the charter gained its ninth non-regular day
// (2021-02-24, the NSE outage). Left at 608 this is not a compile error -- the
// encoder writes its trailing reserve at offset 608 into a 608-byte buffer and
// returns "execution encoder reserve exceeded its record", refusing EVERY
// parameter write at runtime.
const PARAMETER_PAYLOAD_BYTES: usize = 616;
const PERCENTILE_PAYLOAD_BYTES: usize = 96;
const CAPABILITY_PAYLOAD_BYTES: usize = 288;
const COMPLETION_PAYLOAD_BYTES: usize = 384;

/// Fixed byte width of one scalar execution-parameter record, including seal.
pub const EXECUTION_PARAMETER_STRIDE: usize = PARAMETER_PAYLOAD_BYTES + SEAL_BYTES;
/// Fixed byte width of one ordered percentile atom, including seal.
pub const EXECUTION_PERCENTILE_STRIDE: usize = PERCENTILE_PAYLOAD_BYTES + SEAL_BYTES;
/// Fixed byte width of one selected-row capability, including seal.
pub const EXECUTION_CAPABILITY_STRIDE: usize = CAPABILITY_PAYLOAD_BYTES + SEAL_BYTES;
/// Fixed byte width of one receipt-last population completion, including seal.
pub const EXECUTION_COMPLETION_STRIDE: usize = COMPLETION_PAYLOAD_BYTES + SEAL_BYTES;

const PARAMETER_ID_DOMAIN: &[u8] = b"brutex.cli.execution-parameter-id.v1\0";
const CAPABILITY_ID_DOMAIN: &[u8] = b"brutex.cli.execution-row-capability-id.v1\0";
const COMPLETION_ID_DOMAIN: &[u8] = b"brutex.cli.execution-capability-completion-id.v1\0";
const EXECUTION_LAW_DOMAIN: &[u8] = b"brutex.cli.exact-one-minute-1510-law.v1\0";
const PARAMETER_ORDER_DOMAIN: &[u8] = b"brutex.cli.execution-parameter-order.v1\0";
const PERCENTILE_ORDER_DOMAIN: &[u8] = b"brutex.cli.execution-percentile-order.v1\0";
const CAPABILITY_ORDER_DOMAIN: &[u8] = b"brutex.cli.execution-capability-order.v1\0";

const ENTRY_POLICY_TAG: u8 = 1;
const FORCED_EXIT_POLICY_TAG: u8 = 1;
const ENTRY_DELAY_MINUTES: u16 = 1;
const FORCED_EXIT_IST_MINUTE: u16 = 15 * 60 + 10;

const _: () = assert!(EXECUTION_PARAMETER_STRIDE == 648);
const _: () = assert!(EXECUTION_PERCENTILE_STRIDE == 128);
const _: () = assert!(EXECUTION_CAPABILITY_STRIDE == 320);
const _: () = assert!(EXECUTION_COMPLETION_STRIDE == 416);
// 163, NOT 155. The fingerprint carries the calendar, and the charter gained a
// ninth non-regular day (2021-02-24, the NSE outage) -- one more `i64`, so eight
// more bytes. That the constant had to move is the point: a run swept under a
// different calendar is a different run, and section 3 rule 3 makes the calendar
// part of what names it.
const _: () = assert!(EVALUATION_SPEC_FINGERPRINT_V1_LEN == 163);
const _: () = assert!(FORCED_EXIT_MINUTE == 910);

/// Stable identity of the exact next-minute and fixed-close execution law.
///
/// The digest binds the one-minute delay, the 15:10 IST deadline and the
/// printed-OHLCV fill model.  It names repository arithmetic, not a vendor or
/// exchange promise.
#[must_use]
pub fn exact_execution_law_digest_v1() -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(EXECUTION_LAW_DOMAIN);
    hasher.update(&[ENTRY_POLICY_TAG]);
    hasher.update(&ENTRY_DELAY_MINUTES.to_le_bytes());
    hasher.update(&[FORCED_EXIT_POLICY_TAG]);
    hasher.update(&FORCED_EXIT_IST_MINUTE.to_le_bytes());
    hasher.update(&printed_ohlcv_cost_model_id_v1());
    hasher.finalize()
}

/// Complete dynamic parameter set needed to reproduce one side's training grid.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionParametersV1 {
    parameter_id: [u8; 32],
    population_id: [u8; 32],
    population_v4_digest: [u8; 32],
    direction: TradeDirectionV1,
    instrument_family: InstrumentFamilyV1,
    rung_seconds: u32,
    horizon: Horizon,
    run_params: Params,
    evaluation_fingerprint: [u8; EVALUATION_SPEC_FINGERPRINT_V1_LEN],
    policy: ExitGridPolicyV1,
    expected_resolved_digest: [u8; 32],
    training_digest: [u8; 32],
    training_bars: u64,
    training_first_ts_micros: i64,
    training_last_ts_micros: i64,
}

impl ExecutionParametersV1 {
    /// Captures exact typed policy and resolution facts from a V4 population.
    ///
    /// # Errors
    ///
    /// Refuses a population/family/side/rung mismatch, non-one-minute policy,
    /// torn resolution, unsupported fill model, or a resolution digest different
    /// from the side-specific identity already sealed by Population V4.
    pub fn from_resolved(
        population_v4: CompletionReceiptV4,
        direction: TradeDirectionV1,
        horizon: Horizon,
        run_params: Params,
        evaluation_fingerprint: EvaluationSpecFingerprintV1,
        resolved: &ResolvedExitGridV1,
    ) -> Result<Self, ExecutionCapabilityRefusal> {
        if !resolved.digest_is_valid() {
            return Err("resolved exit-grid capability failed its internal digest".to_owned());
        }
        let v2 = population_v4.v3().v2();
        let population_v4_digest = population_v4.content_digest()?;
        let expected_family = family_from_resolved(resolved);
        if expected_family != v2.instrument_family {
            return Err("resolved exit-grid family differs from Population V4".to_owned());
        }
        if side_of_direction(direction) != resolved.side() {
            return Err("resolved exit-grid side differs from the parameter direction".to_owned());
        }
        let expected_grid = match direction {
            TradeDirectionV1::Long => v2.identities.exit_grids.long,
            TradeDirectionV1::Short => v2.identities.exit_grids.short,
        };
        if resolved.policy_digest() != expected_grid.policy_digest
            || resolved.digest() != expected_grid.resolved_digest
        {
            return Err(
                "resolved exit-grid identities differ from Population V4 side authority".to_owned(),
            );
        }
        if resolved.policy().execution_resolution() != ExecutionResolutionV1::OneMinuteOhlcv {
            return Err("execution capability requires exact one-minute OHLCV policy".to_owned());
        }
        if resolved.policy().cost_model_id() != printed_ohlcv_cost_model_id_v1() {
            return Err(
                "execution capability requires the implemented printed-OHLCV model".to_owned(),
            );
        }
        let mut parameters = Self {
            parameter_id: [0; 32],
            population_id: population_v4.population_id(),
            population_v4_digest,
            direction,
            instrument_family: v2.instrument_family,
            rung_seconds: v2.rung_seconds,
            horizon,
            run_params,
            evaluation_fingerprint: evaluation_fingerprint.into_bytes(),
            policy: resolved.policy().clone(),
            expected_resolved_digest: resolved.digest(),
            training_digest: resolved.training_digest(),
            training_bars: resolved.training_bars(),
            training_first_ts_micros: resolved.training_first_ts_micros(),
            training_last_ts_micros: resolved.training_last_ts_micros(),
        };
        parameters.parameter_id = parameters.derived_id()?;
        parameters.validate()?;
        Ok(parameters)
    }

    /// Content-derived parameter identity.
    #[must_use]
    pub const fn parameter_id(&self) -> [u8; 32] {
        self.parameter_id
    }

    /// Population whose rows may reference this parameter set.
    #[must_use]
    pub const fn population_id(&self) -> [u8; 32] {
        self.population_id
    }

    /// Exact V4 completion digest this parameter set extends.
    #[must_use]
    pub const fn population_v4_digest(&self) -> [u8; 32] {
        self.population_v4_digest
    }

    /// Long or short side resolved by this parameter set.
    #[must_use]
    pub const fn direction(&self) -> TradeDirectionV1 {
        self.direction
    }

    /// NIFTY or BANKNIFTY family bound by the source population.
    #[must_use]
    pub const fn instrument_family(&self) -> InstrumentFamilyV1 {
        self.instrument_family
    }

    /// Exact signal rung in seconds.
    #[must_use]
    pub const fn rung_seconds(&self) -> u32 {
        self.rung_seconds
    }

    /// Exact one-minute execution hold horizon.
    #[must_use]
    pub const fn horizon(&self) -> Horizon {
        self.horizon
    }

    /// Exact four-field run parameter record; no field is defaulted on reopen.
    #[must_use]
    pub const fn run_params(&self) -> Params {
        self.run_params
    }

    /// Canonical evaluator bytes required of a rebuilt training column.
    #[must_use]
    pub const fn evaluation_fingerprint(&self) -> &[u8; EVALUATION_SPEC_FINGERPRINT_V1_LEN] {
        &self.evaluation_fingerprint
    }

    /// Complete reconstructed exit-grid policy.
    #[must_use]
    pub const fn policy(&self) -> &ExitGridPolicyV1 {
        &self.policy
    }

    /// Exact expected TRAINING resolution identity.
    #[must_use]
    pub const fn expected_resolved_digest(&self) -> [u8; 32] {
        self.expected_resolved_digest
    }

    /// Re-resolves this policy against exact attested training one-minute bytes.
    ///
    /// # Errors
    ///
    /// Refuses any malformed series or mismatch in policy, resolved identity,
    /// training bytes, count, endpoints, family or side.
    pub fn reconstruct_grid(
        &self,
        series: ExecutionSeriesV1<'_>,
    ) -> Result<ResolvedExitGridV1, ExecutionCapabilityRefusal> {
        self.validate()?;
        let resolved = self
            .policy
            .resolve_attested(series)
            .map_err(|why| format!("recorded exit-grid policy could not resolve: {why:?}"))?;
        if resolved.digest() != self.expected_resolved_digest
            || resolved.policy_digest() != self.policy.digest()
            || resolved.training_digest() != self.training_digest
            || resolved.training_bars() != self.training_bars
            || resolved.training_first_ts_micros() != self.training_first_ts_micros
            || resolved.training_last_ts_micros() != self.training_last_ts_micros
            || family_from_resolved(&resolved) != self.instrument_family
            || resolved.side() != side_of_direction(self.direction)
        {
            return Err(
                "reconstructed exit grid differs from the sealed parameter authority".to_owned(),
            );
        }
        Ok(resolved)
    }

    /// Requires a rebuilt column to carry the exact recorded evaluator policy.
    ///
    /// # Errors
    ///
    /// Refuses legacy columns without a policy token and every changed canonical
    /// evaluator byte.
    pub fn require_column(&self, column: &Column) -> Result<(), ExecutionCapabilityRefusal> {
        let observed = column
            .evaluation_spec_token()
            .ok_or_else(|| "training column has no evaluator-policy capability".to_owned())?
            .fingerprint_v1()
            .into_bytes();
        if observed != self.evaluation_fingerprint {
            return Err("training column evaluator policy differs from authority".to_owned());
        }
        Ok(())
    }

    fn validate(&self) -> Result<(), ExecutionCapabilityRefusal> {
        require_digest("parameter_id", &self.parameter_id)?;
        require_digest("population_id", &self.population_id)?;
        require_digest("population_v4_digest", &self.population_v4_digest)?;
        require_digest("expected_resolved_digest", &self.expected_resolved_digest)?;
        require_digest("training_digest", &self.training_digest)?;
        validate_rung_seconds(self.rung_seconds)?;
        if self.horizon.as_bars() == 0 {
            return Err("execution horizon cannot be zero".to_owned());
        }
        if self.policy.execution_resolution() != ExecutionResolutionV1::OneMinuteOhlcv {
            return Err("execution parameters are not exact one-minute OHLCV".to_owned());
        }
        if self.policy.side() != side_of_direction(self.direction) {
            return Err("execution parameter direction and policy side differ".to_owned());
        }
        if self.policy.cost_model_id() != printed_ohlcv_cost_model_id_v1() {
            return Err("execution parameter cost model is unsupported".to_owned());
        }
        if self.training_bars == 0 || self.training_first_ts_micros > self.training_last_ts_micros {
            return Err("execution parameter training geometry is invalid".to_owned());
        }
        if self.parameter_id != self.derived_id()? {
            return Err(
                "execution parameter identity differs from its canonical fields".to_owned(),
            );
        }
        Ok(())
    }

    fn derived_id(&self) -> Result<[u8; 32], ExecutionCapabilityRefusal> {
        let mut hasher = Hasher::new();
        hasher.update(PARAMETER_ID_DOMAIN);
        hasher.update(&self.population_id);
        hasher.update(&self.population_v4_digest);
        hasher.update(&[direction_byte(self.direction)]);
        hasher.update(&[family_byte(self.instrument_family)]);
        hasher.update(&self.rung_seconds.to_le_bytes());
        hasher.update(&self.horizon.as_bars().to_le_bytes());
        for value in [
            self.run_params.min_hits,
            self.run_params.ceiling,
            self.run_params.pair_budget,
            self.run_params.policy,
        ] {
            hasher.update(&value.to_le_bytes());
        }
        hasher.update(&self.evaluation_fingerprint);
        hasher.update(&self.policy.digest());
        put_percentiles(&mut hasher, self.policy.rungs().stop())?;
        put_percentiles(&mut hasher, self.policy.rungs().target())?;
        put_percentiles(&mut hasher, self.policy.rungs().trail())?;
        hasher.update(&self.expected_resolved_digest);
        hasher.update(&self.training_digest);
        hasher.update(&self.training_bars.to_le_bytes());
        hasher.update(&self.training_first_ts_micros.to_le_bytes());
        hasher.update(&self.training_last_ts_micros.to_le_bytes());
        hasher.update(&exact_execution_law_digest_v1());
        Ok(hasher.finalize())
    }
}

/// One population row bound to its exact training and selected-exit capabilities.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExecutionStrategyCapabilityV1 {
    capability_id: [u8; 32],
    parameter_id: [u8; 32],
    population_id: [u8; 32],
    population_v4_digest: [u8; 32],
    row_payload_digest: [u8; 32],
    strategy_digest: [u8; 32],
    training_run_id: [u8; 32],
    selected_exit_digest: [u8; 32],
    row_sequence: u64,
    rung_seconds: u32,
    direction: TradeDirectionV1,
    instrument_family: InstrumentFamilyV1,
}

impl ExecutionStrategyCapabilityV1 {
    /// Seals one selected training capability against one population row.
    ///
    /// # Errors
    ///
    /// Refuses every parameter/row identity mismatch, a coordinate different
    /// from the selected capability, or an invalid population payload.
    pub fn new(
        parameters: &ExecutionParametersV1,
        row: PopulationRowV1,
        selected: &SelectedExitV1,
    ) -> Result<Self, ExecutionCapabilityRefusal> {
        parameters.validate()?;
        require_row_matches(parameters, &row)?;
        let coordinate = chosen_from_population(&row)?;
        if selected.coordinate() != coordinate {
            return Err("selected exit coordinate differs from population row".to_owned());
        }
        let mut capability = Self {
            capability_id: [0; 32],
            parameter_id: parameters.parameter_id,
            population_id: row.population_id,
            population_v4_digest: parameters.population_v4_digest,
            row_payload_digest: row.payload_digest()?,
            strategy_digest: row.strategy_digest,
            training_run_id: selected.run_id().bytes(),
            selected_exit_digest: selected.digest(),
            row_sequence: row.sequence,
            rung_seconds: row.rung_seconds,
            direction: row.direction,
            instrument_family: row.instrument_family,
        };
        capability.capability_id = capability.derived_id();
        capability.validate()?;
        Ok(capability)
    }

    /// Content-derived row-capability identity.
    #[must_use]
    pub const fn capability_id(self) -> [u8; 32] {
        self.capability_id
    }

    /// Parameter set required to reconstruct this row.
    #[must_use]
    pub const fn parameter_id(self) -> [u8; 32] {
        self.parameter_id
    }

    /// Source population identity.
    #[must_use]
    pub const fn population_id(self) -> [u8; 32] {
        self.population_id
    }

    /// Canonical zero-based source row sequence.
    #[must_use]
    pub const fn row_sequence(self) -> u64 {
        self.row_sequence
    }

    /// Expected training run identity.
    #[must_use]
    pub const fn training_run_id(self) -> [u8; 32] {
        self.training_run_id
    }

    /// Expected opaque selected-exit identity.
    #[must_use]
    pub const fn selected_exit_digest(self) -> [u8; 32] {
        self.selected_exit_digest
    }

    /// Reconstructs and re-authorizes the exact selected exit.
    ///
    /// # Errors
    ///
    /// Refuses every changed parameter, row, column, run, training series,
    /// complete grid, coordinate or final opaque selection digest.
    pub fn reconstruct_selected(
        &self,
        parameters: &ExecutionParametersV1,
        row: PopulationRowV1,
        series: ExecutionSeriesV1<'_>,
        column: &Column,
        run: ExecutionRunV1,
    ) -> Result<(ResolvedExitGridV1, SelectedExitV1), ExecutionCapabilityRefusal> {
        self.validate()?;
        self.require_binding(parameters, &row)?;
        parameters.require_column(column)?;
        if run.run_id().bytes() != self.training_run_id {
            return Err("training execution run differs from row capability".to_owned());
        }
        let resolved = parameters.reconstruct_grid(series)?;
        let evaluated = resolved
            .evaluate_training_grid_attested(series, column, parameters.horizon, run)
            .map_err(|why| format!("complete training grid could not be evaluated: {why:?}"))?;
        let validated = resolved
            .validate_evaluation(&evaluated)
            .map_err(|why| format!("complete training grid did not validate: {why:?}"))?;
        let selected = resolved
            .authorize_coordinate(&validated, chosen_from_population(&row)?)
            .map_err(|why| format!("population exit coordinate was not authorized: {why:?}"))?;
        if selected.run_id().bytes() != self.training_run_id
            || selected.digest() != self.selected_exit_digest
        {
            return Err("reconstructed selected exit differs from sealed capability".to_owned());
        }
        Ok((resolved, selected))
    }

    fn require_binding(
        &self,
        parameters: &ExecutionParametersV1,
        row: &PopulationRowV1,
    ) -> Result<(), ExecutionCapabilityRefusal> {
        parameters.validate()?;
        require_row_matches(parameters, row)?;
        if self.parameter_id != parameters.parameter_id
            || self.population_id != row.population_id
            || self.population_v4_digest != parameters.population_v4_digest
            || self.row_sequence != row.sequence
            || self.row_payload_digest != row.payload_digest()?
            || self.strategy_digest != row.strategy_digest
            || self.rung_seconds != row.rung_seconds
            || self.direction != row.direction
            || self.instrument_family != row.instrument_family
        {
            return Err(
                "execution row capability differs from its parameter/population row".to_owned(),
            );
        }
        Ok(())
    }

    fn validate(&self) -> Result<(), ExecutionCapabilityRefusal> {
        for (name, digest) in [
            ("capability_id", self.capability_id),
            ("parameter_id", self.parameter_id),
            ("population_id", self.population_id),
            ("population_v4_digest", self.population_v4_digest),
            ("row_payload_digest", self.row_payload_digest),
            ("strategy_digest", self.strategy_digest),
            ("training_run_id", self.training_run_id),
            ("selected_exit_digest", self.selected_exit_digest),
        ] {
            require_digest(name, &digest)?;
        }
        validate_rung_seconds(self.rung_seconds)?;
        if self.capability_id != self.derived_id() {
            return Err("execution row capability identity differs from its fields".to_owned());
        }
        Ok(())
    }

    fn derived_id(&self) -> [u8; 32] {
        let mut hasher = Hasher::new();
        hasher.update(CAPABILITY_ID_DOMAIN);
        for digest in [
            self.parameter_id,
            self.population_id,
            self.population_v4_digest,
            self.row_payload_digest,
            self.strategy_digest,
            self.training_run_id,
            self.selected_exit_digest,
        ] {
            hasher.update(&digest);
        }
        hasher.update(&self.row_sequence.to_le_bytes());
        hasher.update(&self.rung_seconds.to_le_bytes());
        hasher.update(&[direction_byte(self.direction)]);
        hasher.update(&[family_byte(self.instrument_family)]);
        hasher.finalize()
    }

    fn payload(self) -> Result<[u8; CAPABILITY_PAYLOAD_BYTES], ExecutionCapabilityRefusal> {
        self.validate()?;
        let mut raw = [0; CAPABILITY_PAYLOAD_BYTES];
        let mut encoder = Encoder::new(&mut raw);
        for digest in [
            self.capability_id,
            self.parameter_id,
            self.population_id,
            self.population_v4_digest,
            self.row_payload_digest,
            self.strategy_digest,
            self.training_run_id,
            self.selected_exit_digest,
        ] {
            encoder.bytes(&digest)?;
        }
        encoder.u64(self.row_sequence)?;
        encoder.u32(self.rung_seconds)?;
        encoder.u8(direction_byte(self.direction))?;
        encoder.u8(family_byte(self.instrument_family))?;
        encoder.zeros(18)?;
        encoder.finish()?;
        Ok(raw)
    }

    fn to_bytes(self) -> Result<[u8; EXECUTION_CAPABILITY_STRIDE], ExecutionCapabilityRefusal> {
        with_seal(self.payload()?)
    }

    fn from_bytes(
        raw: &[u8; EXECUTION_CAPABILITY_STRIDE],
    ) -> Result<Self, ExecutionCapabilityRefusal> {
        let payload = checked_payload::<CAPABILITY_PAYLOAD_BYTES, EXECUTION_CAPABILITY_STRIDE>(
            raw,
            "execution row capability",
        )?;
        let mut decoder = Decoder::new(&payload);
        let capability = Self {
            capability_id: decoder.array_32()?,
            parameter_id: decoder.array_32()?,
            population_id: decoder.array_32()?,
            population_v4_digest: decoder.array_32()?,
            row_payload_digest: decoder.array_32()?,
            strategy_digest: decoder.array_32()?,
            training_run_id: decoder.array_32()?,
            selected_exit_digest: decoder.array_32()?,
            row_sequence: decoder.u64()?,
            rung_seconds: decoder.u32()?,
            direction: direction_from_byte(decoder.u8()?)?,
            instrument_family: family_from_byte(decoder.u8()?)?,
        };
        decoder.zeros(18, "execution row capability trailing reserve")?;
        decoder.finish()?;
        capability.validate()?;
        Ok(capability)
    }
}

/// Receipt-last proof that every row in one V4 population has an exact,
/// reconstructible execution capability.
///
/// Physical block offsets are retained for bounded reopening but deliberately
/// excluded from `authority_id`: a crash may leave valid orphan records before
/// a retry, and that retry must preserve semantic identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExecutionCapabilityCompletionV1 {
    authority_id: [u8; 32],
    population_id: [u8; 32],
    population_v4_digest: [u8; 32],
    long_parameter_id: [u8; 32],
    short_parameter_id: [u8; 32],
    parameter_first: u64,
    parameter_count: u64,
    percentile_first: u64,
    percentile_count: u64,
    capability_first: u64,
    capability_count: u64,
    row_count: u64,
    long_count: u64,
    short_count: u64,
    ordered_parameter_digest: [u8; 32],
    ordered_percentile_digest: [u8; 32],
    ordered_capability_digest: [u8; 32],
    execution_law_digest: [u8; 32],
}

impl ExecutionCapabilityCompletionV1 {
    /// Stable content authority, independent of physical orphan offsets.
    #[must_use]
    pub const fn authority_id(self) -> [u8; 32] {
        self.authority_id
    }

    /// V4 population covered row-for-row by this receipt.
    #[must_use]
    pub const fn population_id(self) -> [u8; 32] {
        self.population_id
    }

    /// Exact V4 population payload digest extended by this receipt.
    #[must_use]
    pub const fn population_v4_digest(self) -> [u8; 32] {
        self.population_v4_digest
    }

    /// Exact number of row capabilities committed by this receipt.
    #[must_use]
    pub const fn row_count(self) -> u64 {
        self.row_count
    }

    /// Exact long/short row counts, in that order.
    #[must_use]
    pub const fn side_counts(self) -> (u64, u64) {
        (self.long_count, self.short_count)
    }

    fn for_prepared(
        prepared: &PreparedExecutionCapabilitiesV1,
        parameter_first: u64,
        percentile_first: u64,
        capability_first: u64,
    ) -> Result<Self, ExecutionCapabilityRefusal> {
        let [long, short] = &prepared.parameters;
        let mut receipt = Self {
            authority_id: [0; 32],
            population_id: prepared.population_id,
            population_v4_digest: prepared.population_v4_digest,
            long_parameter_id: long.parameter_id,
            short_parameter_id: short.parameter_id,
            parameter_first,
            parameter_count: 2,
            percentile_first,
            percentile_count: usize_u64(prepared.percentiles.len(), "execution percentile block")?,
            capability_first,
            capability_count: usize_u64(prepared.capabilities.len(), "execution capability block")?,
            row_count: prepared.row_count,
            long_count: prepared.long_count,
            short_count: prepared.short_count,
            ordered_parameter_digest: prepared.ordered_parameter_digest,
            ordered_percentile_digest: prepared.ordered_percentile_digest,
            ordered_capability_digest: prepared.ordered_capability_digest,
            execution_law_digest: exact_execution_law_digest_v1(),
        };
        receipt.authority_id = receipt.derived_id();
        receipt.validate()?;
        Ok(receipt)
    }

    fn validate(self) -> Result<(), ExecutionCapabilityRefusal> {
        for (name, digest) in [
            ("execution authority_id", self.authority_id),
            ("execution population_id", self.population_id),
            ("execution population_v4_digest", self.population_v4_digest),
            ("execution long_parameter_id", self.long_parameter_id),
            ("execution short_parameter_id", self.short_parameter_id),
            (
                "execution ordered_parameter_digest",
                self.ordered_parameter_digest,
            ),
            (
                "execution ordered_percentile_digest",
                self.ordered_percentile_digest,
            ),
            (
                "execution ordered_capability_digest",
                self.ordered_capability_digest,
            ),
            ("execution law digest", self.execution_law_digest),
        ] {
            require_digest(name, &digest)?;
        }
        if self.parameter_count != 2 {
            return Err(
                "execution completion must name exactly long and short parameters".to_owned(),
            );
        }
        if self.long_parameter_id == self.short_parameter_id {
            return Err(
                "execution completion copied one parameter identity across both sides".to_owned(),
            );
        }
        if self.percentile_count < 6 {
            return Err(
                "execution completion has fewer than one stop/target/trail atom per side"
                    .to_owned(),
            );
        }
        if self.capability_count != self.row_count {
            return Err("execution capability count differs from population row_count".to_owned());
        }
        let classified = self
            .long_count
            .checked_add(self.short_count)
            .ok_or_else(|| "execution side counts overflow u64".to_owned())?;
        if classified != self.row_count {
            return Err("execution long plus short counts differ from row_count".to_owned());
        }
        for (first, count, name) in [
            (self.parameter_first, self.parameter_count, "parameter"),
            (self.percentile_first, self.percentile_count, "percentile"),
            (self.capability_first, self.capability_count, "capability"),
        ] {
            first
                .checked_add(count)
                .ok_or_else(|| format!("execution {name} block range overflows u64"))?;
        }
        if self.execution_law_digest != exact_execution_law_digest_v1() {
            return Err(
                "execution completion does not bind the exact 1-minute/15:10 law".to_owned(),
            );
        }
        if self.authority_id != self.derived_id() {
            return Err(
                "execution completion authority_id differs from its semantic fields".to_owned(),
            );
        }
        Ok(())
    }

    fn derived_id(self) -> [u8; 32] {
        let mut hasher = Hasher::new();
        hasher.update(COMPLETION_ID_DOMAIN);
        for digest in [
            self.population_id,
            self.population_v4_digest,
            self.long_parameter_id,
            self.short_parameter_id,
            self.ordered_parameter_digest,
            self.ordered_percentile_digest,
            self.ordered_capability_digest,
            self.execution_law_digest,
        ] {
            hasher.update(&digest);
        }
        for count in [
            self.parameter_count,
            self.percentile_count,
            self.capability_count,
            self.row_count,
            self.long_count,
            self.short_count,
        ] {
            hasher.update(&count.to_le_bytes());
        }
        hasher.finalize()
    }

    fn payload(self) -> Result<[u8; COMPLETION_PAYLOAD_BYTES], ExecutionCapabilityRefusal> {
        self.validate()?;
        let mut raw = [0; COMPLETION_PAYLOAD_BYTES];
        let mut encoder = Encoder::new(&mut raw);
        for digest in [
            self.authority_id,
            self.population_id,
            self.population_v4_digest,
            self.long_parameter_id,
            self.short_parameter_id,
        ] {
            encoder.bytes(&digest)?;
        }
        for value in [
            self.parameter_first,
            self.parameter_count,
            self.percentile_first,
            self.percentile_count,
            self.capability_first,
            self.capability_count,
            self.row_count,
            self.long_count,
            self.short_count,
        ] {
            encoder.u64(value)?;
        }
        for digest in [
            self.ordered_parameter_digest,
            self.ordered_percentile_digest,
            self.ordered_capability_digest,
            self.execution_law_digest,
        ] {
            encoder.bytes(&digest)?;
        }
        encoder.zeros(24)?;
        encoder.finish()?;
        Ok(raw)
    }

    fn to_bytes(self) -> Result<[u8; EXECUTION_COMPLETION_STRIDE], ExecutionCapabilityRefusal> {
        with_seal(self.payload()?)
    }

    fn from_bytes(
        raw: &[u8; EXECUTION_COMPLETION_STRIDE],
    ) -> Result<Self, ExecutionCapabilityRefusal> {
        let payload = checked_payload::<COMPLETION_PAYLOAD_BYTES, EXECUTION_COMPLETION_STRIDE>(
            raw,
            "execution completion",
        )?;
        let mut decoder = Decoder::new(&payload);
        let receipt = Self {
            authority_id: decoder.array_32()?,
            population_id: decoder.array_32()?,
            population_v4_digest: decoder.array_32()?,
            long_parameter_id: decoder.array_32()?,
            short_parameter_id: decoder.array_32()?,
            parameter_first: decoder.u64()?,
            parameter_count: decoder.u64()?,
            percentile_first: decoder.u64()?,
            percentile_count: decoder.u64()?,
            capability_first: decoder.u64()?,
            capability_count: decoder.u64()?,
            row_count: decoder.u64()?,
            long_count: decoder.u64()?,
            short_count: decoder.u64()?,
            ordered_parameter_digest: decoder.array_32()?,
            ordered_percentile_digest: decoder.array_32()?,
            ordered_capability_digest: decoder.array_32()?,
            execution_law_digest: decoder.array_32()?,
        };
        decoder.zeros(24, "execution completion trailing reserve")?;
        decoder.finish()?;
        receipt.validate()?;
        Ok(receipt)
    }

    fn same_semantics(self, prepared: &PreparedExecutionCapabilitiesV1) -> bool {
        let [long, short] = &prepared.parameters;
        self.population_id == prepared.population_id
            && self.population_v4_digest == prepared.population_v4_digest
            && self.long_parameter_id == long.parameter_id
            && self.short_parameter_id == short.parameter_id
            && self.row_count == prepared.row_count
            && self.long_count == prepared.long_count
            && self.short_count == prepared.short_count
            && self.ordered_parameter_digest == prepared.ordered_parameter_digest
            && self.ordered_percentile_digest == prepared.ordered_percentile_digest
            && self.ordered_capability_digest == prepared.ordered_capability_digest
            && self.execution_law_digest == exact_execution_law_digest_v1()
    }
}

/// Fully checked material prepared from a generation-validated V4 population.
///
/// Fields are private so a caller cannot bypass [`Self::from_population_v4`]
/// with caller-authored row bytes.
#[derive(Clone, Debug)]
pub struct PreparedExecutionCapabilitiesV1 {
    population_id: [u8; 32],
    population_v4_digest: [u8; 32],
    parameters: [ExecutionParametersV1; 2],
    scalars: [ParameterScalarV1; 2],
    percentiles: Vec<PercentileRecordV1>,
    capabilities: Vec<ExecutionStrategyCapabilityV1>,
    row_count: u64,
    long_count: u64,
    short_count: u64,
    ordered_parameter_digest: [u8; 32],
    ordered_percentile_digest: [u8; 32],
    ordered_capability_digest: [u8; 32],
}

impl PreparedExecutionCapabilitiesV1 {
    /// Verifies every capability against the authoritative V4 population block.
    ///
    /// The population is read in bounded pages. The operation is O(rows), not
    /// O(1); only a later indexed row probe is average O(1).
    ///
    /// **UNVERIFIED as a measured bound.** No bench in this workspace
    /// times this, so the shape above is read from the source rather
    /// than measured. `CLAUDE.md` §3 rule 6.
    ///
    /// # Errors
    ///
    /// Refuses absent/stale V4 authority, copied side parameters, any missing,
    /// reordered or foreign row capability, changed V4 digest, or side counts
    /// different from the sealed V2 facts embedded in V4.
    pub fn from_population_v4(
        population: &mut PopulationLedger,
        population_id: [u8; 32],
        long: ExecutionParametersV1,
        short: ExecutionParametersV1,
        capabilities: Vec<ExecutionStrategyCapabilityV1>,
    ) -> Result<Self, ExecutionCapabilityRefusal> {
        let receipt = population
            .receipt_v4(&population_id)
            .ok_or_else(|| "execution capability preparation requires Population V4".to_owned())?;
        let population_v4_digest = receipt.content_digest()?;
        let v2 = receipt.v3().v2();
        validate_population_parameter_pair(
            population_id,
            population_v4_digest,
            &v2,
            &long,
            &short,
        )?;
        let row_count = usize_u64(capabilities.len(), "execution capability block")?;
        if row_count != v2.row_count {
            return Err(format!(
                "execution capability count {row_count} differs from V4 row_count {}",
                v2.row_count
            ));
        }

        let parameters = [long, short];
        let [long_parameters, short_parameters] = &parameters;
        let (long_count, short_count) = validate_population_capabilities(
            population,
            population_id,
            &v2,
            &parameters,
            &capabilities,
        )?;

        let scalars = [
            ParameterScalarV1::from_parameters(long_parameters)?,
            ParameterScalarV1::from_parameters(short_parameters)?,
        ];
        let mut percentiles = PercentileRecordV1::from_parameters(long_parameters)?;
        percentiles.extend(PercentileRecordV1::from_parameters(short_parameters)?);
        let ordered_parameter_digest = parameter_digest_records(&scalars)?;
        let ordered_percentile_digest = percentile_digest_records(&percentiles)?;
        let ordered_capability_digest = capability_digest_records(&capabilities)?;
        Ok(Self {
            population_id,
            population_v4_digest,
            parameters,
            scalars,
            percentiles,
            capabilities,
            row_count,
            long_count,
            short_count,
            ordered_parameter_digest,
            ordered_percentile_digest,
            ordered_capability_digest,
        })
    }

    /// Stable semantic identity before physical append offsets are assigned.
    ///
    /// # Errors
    ///
    /// Refuses any internally inconsistent count, digest or execution-law
    /// field; construction normally made those states unreachable.
    pub fn authority_id(&self) -> Result<[u8; 32], ExecutionCapabilityRefusal> {
        Ok(ExecutionCapabilityCompletionV1::for_prepared(self, 0, 0, 0)?.authority_id)
    }
}

/// Builds exact resolved-grid parameters for a test authority without
/// manufacturing a Population V4 production receipt.
#[cfg(test)]
#[allow(
    clippy::too_many_arguments,
    reason = "the test seam keeps every independently validated authority term explicit"
)]
pub(crate) fn canonical_execution_parameters_fixture_v1(
    population_id: [u8; 32],
    population_v4_digest: [u8; 32],
    direction: TradeDirectionV1,
    instrument_family: InstrumentFamilyV1,
    rung_seconds: u32,
    horizon: Horizon,
    run_params: Params,
    evaluation_fingerprint: EvaluationSpecFingerprintV1,
    resolved: &ResolvedExitGridV1,
) -> Result<ExecutionParametersV1, ExecutionCapabilityRefusal> {
    if family_from_resolved(resolved) != instrument_family
        || resolved.side() != side_of_direction(direction)
        || !resolved.digest_is_valid()
    {
        return Err(
            "canonical execution fixture resolution differs from its family, side or digest"
                .to_owned(),
        );
    }
    let mut parameters = ExecutionParametersV1 {
        parameter_id: [0; 32],
        population_id,
        population_v4_digest,
        direction,
        instrument_family,
        rung_seconds,
        horizon,
        run_params,
        evaluation_fingerprint: evaluation_fingerprint.into_bytes(),
        policy: resolved.policy().clone(),
        expected_resolved_digest: resolved.digest(),
        training_digest: resolved.training_digest(),
        training_bars: resolved.training_bars(),
        training_first_ts_micros: resolved.training_first_ts_micros(),
        training_last_ts_micros: resolved.training_last_ts_micros(),
    };
    parameters.parameter_id = parameters.derived_id()?;
    parameters.validate()?;
    Ok(parameters)
}

/// Builds one complete test execution block through the same parameter,
/// capability and ordered-digest validators used by durable append.
#[cfg(test)]
pub(crate) fn canonical_execution_capabilities_fixture_v1(
    long: ExecutionParametersV1,
    short: ExecutionParametersV1,
    rows: &[PopulationRowV1],
    selected: &[SelectedExitV1],
) -> Result<PreparedExecutionCapabilitiesV1, ExecutionCapabilityRefusal> {
    if rows.len() != selected.len() {
        return Err(format!(
            "canonical execution fixture has {} rows but {} selected exits",
            rows.len(),
            selected.len()
        ));
    }
    for (parameters, direction) in [
        (&long, TradeDirectionV1::Long),
        (&short, TradeDirectionV1::Short),
    ] {
        parameters.validate()?;
        if parameters.direction != direction {
            return Err(
                "canonical execution fixture parameter order is not long then short".to_owned(),
            );
        }
    }
    if long.population_id != short.population_id
        || long.population_v4_digest != short.population_v4_digest
        || long.instrument_family != short.instrument_family
        || long.rung_seconds != short.rung_seconds
    {
        return Err("canonical execution fixture parameter authorities differ".to_owned());
    }

    let parameters = [long, short];
    let mut capabilities = Vec::new();
    capabilities.try_reserve_exact(rows.len()).map_err(|why| {
        format!("canonical execution fixture capability allocation refused: {why}")
    })?;
    let mut long_count = 0_u64;
    let mut short_count = 0_u64;
    for (index, (row, exit)) in rows.iter().zip(selected).enumerate() {
        if row.sequence
            != u64::try_from(index)
                .map_err(|_| "canonical execution fixture sequence does not fit u64".to_owned())?
        {
            return Err("canonical execution fixture rows are not in sequence order".to_owned());
        }
        let parameter = match row.direction {
            TradeDirectionV1::Long => {
                long_count = long_count
                    .checked_add(1)
                    .ok_or_else(|| "canonical execution fixture long count overflow".to_owned())?;
                &parameters[0]
            }
            TradeDirectionV1::Short => {
                short_count = short_count
                    .checked_add(1)
                    .ok_or_else(|| "canonical execution fixture short count overflow".to_owned())?;
                &parameters[1]
            }
        };
        capabilities.push(ExecutionStrategyCapabilityV1::new(parameter, *row, exit)?);
    }

    let scalars = [
        ParameterScalarV1::from_parameters(&parameters[0])?,
        ParameterScalarV1::from_parameters(&parameters[1])?,
    ];
    let mut percentiles = PercentileRecordV1::from_parameters(&parameters[0])?;
    percentiles.extend(PercentileRecordV1::from_parameters(&parameters[1])?);
    let row_count = usize_u64(rows.len(), "canonical execution fixture")?;
    Ok(PreparedExecutionCapabilitiesV1 {
        population_id: parameters[0].population_id,
        population_v4_digest: parameters[0].population_v4_digest,
        ordered_parameter_digest: parameter_digest_records(&scalars)?,
        ordered_percentile_digest: percentile_digest_records(&percentiles)?,
        ordered_capability_digest: capability_digest_records(&capabilities)?,
        parameters,
        scalars,
        percentiles,
        capabilities,
        row_count,
        long_count,
        short_count,
    })
}

fn validate_population_parameter_pair(
    population_id: [u8; 32],
    population_v4_digest: [u8; 32],
    v2: &CompletionReceiptV2,
    long: &ExecutionParametersV1,
    short: &ExecutionParametersV1,
) -> Result<(), ExecutionCapabilityRefusal> {
    for (parameters, direction) in [
        (long, TradeDirectionV1::Long),
        (short, TradeDirectionV1::Short),
    ] {
        parameters.validate()?;
        if parameters.population_id != population_id
            || parameters.population_v4_digest != population_v4_digest
            || parameters.direction != direction
            || parameters.instrument_family != v2.instrument_family
            || parameters.rung_seconds != v2.rung_seconds
        {
            return Err(
                "execution parameters differ from their V4 population/side authority".to_owned(),
            );
        }
    }
    if long.parameter_id == short.parameter_id {
        return Err("long and short execution parameters have one copied identity".to_owned());
    }
    Ok(())
}

fn validate_population_capabilities(
    population: &mut PopulationLedger,
    population_id: [u8; 32],
    v2: &CompletionReceiptV2,
    parameters: &[ExecutionParametersV1; 2],
    capabilities: &[ExecutionStrategyCapabilityV1],
) -> Result<(u64, u64), ExecutionCapabilityRefusal> {
    let row_count = usize_u64(capabilities.len(), "execution capability block")?;
    let [long_parameters, short_parameters] = parameters;
    let mut offset = 0_u64;
    let mut long_count = 0_u64;
    let mut short_count = 0_u64;
    while offset < row_count {
        let limit = 256_u64.min(row_count.saturating_sub(offset));
        let page = population
            .page_v4(&population_id, offset, limit)?
            .ok_or_else(|| {
                "V4 population disappeared while capabilities were checked".to_owned()
            })?;
        if page.total != row_count || page.offset != offset || page.rows.is_empty() {
            return Err(
                "V4 population page geometry changed during capability preparation".to_owned(),
            );
        }
        for row in page.rows {
            let sequence = usize::try_from(row.sequence)
                .map_err(|_| "population sequence does not fit usize".to_owned())?;
            let capability = capabilities.get(sequence).ok_or_else(|| {
                "execution capability sequence is missing from the offered block".to_owned()
            })?;
            let parameters_for_row = match row.direction {
                TradeDirectionV1::Long => {
                    long_count = long_count
                        .checked_add(1)
                        .ok_or_else(|| "long execution count overflow".to_owned())?;
                    long_parameters
                }
                TradeDirectionV1::Short => {
                    short_count = short_count
                        .checked_add(1)
                        .ok_or_else(|| "short execution count overflow".to_owned())?;
                    short_parameters
                }
            };
            capability.require_binding(parameters_for_row, &row)?;
            if capability.row_sequence != row.sequence {
                return Err("execution capability block is reordered".to_owned());
            }
        }
        offset = offset
            .checked_add(limit)
            .ok_or_else(|| "execution capability page offset overflow".to_owned())?;
    }
    if long_count != v2.long_exit_cells_evaluated || short_count != v2.short_exit_cells_evaluated {
        return Err("execution capability side counts differ from Population V4 facts".to_owned());
    }
    Ok((long_count, short_count))
}

/// Whether an exact capability authority was appended or byte-identically reused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExecutionCapabilityCommit {
    /// New parameter, percentile, row-capability and receipt bytes were synced.
    Written,
    /// The already committed semantic authority matched exactly.
    Reused,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FileGeneration {
    len: u64,
    platform: PlatformGeneration,
}

#[cfg(unix)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PlatformGeneration {
    device: u64,
    inode: u64,
    modified_seconds: i64,
    modified_nanoseconds: i64,
    changed_seconds: i64,
    changed_nanoseconds: i64,
}

#[cfg(windows)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PlatformGeneration {
    volume_serial: u32,
    file_index: u64,
    creation_time: u64,
    last_write_time: u64,
}

#[cfg(not(any(unix, windows)))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PlatformGeneration;

#[derive(Clone, Debug)]
struct CommittedExecutionAuthorityV1 {
    receipt: ExecutionCapabilityCompletionV1,
    parameters: [ExecutionParametersV1; 2],
}

/// Append-only four-file execution-capability ledger.
///
/// Scalar policy records, dynamic percentile atoms and per-row capabilities
/// are non-authoritative prepared bytes until the final completion receipt is
/// durably synced. Valid unreferenced records are retained as crash evidence
/// and skipped on reopen; a torn record or receipt is always refused.
#[derive(Debug)]
pub struct ExecutionCapabilityLedger {
    parameter_file: File,
    percentile_file: File,
    capability_file: File,
    completion_file: File,
    writer_lock: File,
    parameter_path: PathBuf,
    percentile_path: PathBuf,
    capability_path: PathBuf,
    completion_path: PathBuf,
    parameter_generation: FileGeneration,
    percentile_generation: FileGeneration,
    capability_generation: FileGeneration,
    completion_generation: FileGeneration,
    authorities: HashMap<[u8; 32], CommittedExecutionAuthorityV1>,
    capabilities: HashMap<([u8; 32], u64), ExecutionStrategyCapabilityV1>,
    writable: bool,
}

impl ExecutionCapabilityLedger {
    /// Scalar parameter records (640-byte stride).
    #[must_use]
    pub fn parameter_path(root: &Path) -> PathBuf {
        root.join("results").join("execution-parameters-v1.bin")
    }

    /// Dynamic percentile atoms (128-byte stride).
    #[must_use]
    pub fn percentile_path(root: &Path) -> PathBuf {
        root.join("results").join("execution-percentiles-v1.bin")
    }

    /// Per-population-row capabilities (320-byte stride).
    #[must_use]
    pub fn capability_path(root: &Path) -> PathBuf {
        root.join("results").join("execution-capabilities-v1.bin")
    }

    /// Receipt-last completion records (416-byte stride).
    #[must_use]
    pub fn completion_path(root: &Path) -> PathBuf {
        root.join("results")
            .join("execution-capability-completions-v1.bin")
    }

    fn lock_path(root: &Path) -> PathBuf {
        root.join("results").join("population-write.lock")
    }

    /// Creates missing fixed-layout files, then scans and indexes all committed
    /// authorities while holding the common population writer lock.
    ///
    /// # Errors
    ///
    /// Refuses bad paths, lock/I/O failures, any header/version/stride/reserve
    /// mismatch, ragged/torn record, invalid block reference, overlap,
    /// duplicate population or semantic mismatch.
    pub fn open(root: &Path) -> Result<Self, ExecutionCapabilityRefusal> {
        let results = root.join("results");
        fs::create_dir_all(&results)
            .map_err(|why| format!("{} could not be created: {why}", results.display()))?;
        let lock_path = Self::lock_path(root);
        let writer_lock = open_or_create(&lock_path)?;
        writer_lock.lock().map_err(|why| {
            format!(
                "{} could not be exclusively locked: {why}",
                lock_path.display()
            )
        })?;
        let opened = (|| {
            let paths = LedgerPaths::of(root);
            let mut files = LedgerFiles {
                parameter: open_or_create(&paths.parameter)?,
                percentile: open_or_create(&paths.percentile)?,
                capability: open_or_create(&paths.capability)?,
                completion: open_or_create(&paths.completion)?,
            };
            ensure_record_header(
                &mut files.parameter,
                &paths.parameter,
                PARAMETER_MAGIC,
                EXECUTION_PARAMETER_STRIDE,
            )?;
            ensure_record_header(
                &mut files.percentile,
                &paths.percentile,
                PERCENTILE_MAGIC,
                EXECUTION_PERCENTILE_STRIDE,
            )?;
            ensure_record_header(
                &mut files.capability,
                &paths.capability,
                CAPABILITY_MAGIC,
                EXECUTION_CAPABILITY_STRIDE,
            )?;
            ensure_record_header(
                &mut files.completion,
                &paths.completion,
                COMPLETION_MAGIC,
                EXECUTION_COMPLETION_STRIDE,
            )?;
            sync_directory(&results)?;
            Self::from_files(
                files,
                paths,
                writer_lock.try_clone().map_err(|why| {
                    format!(
                        "{} lock handle could not be cloned: {why}",
                        lock_path.display()
                    )
                })?,
                true,
            )
        })();
        let released = writer_lock
            .unlock()
            .map_err(|why| format!("{} could not be unlocked: {why}", lock_path.display()));
        match (opened, released) {
            (Ok(ledger), Ok(())) => Ok(ledger),
            (Err(why), _) | (Ok(_), Err(why)) => Err(why),
        }
    }

    /// Opens existing files read-only and rebuilds exact indexes.
    ///
    /// Opening is O(total stored records), never claimed O(1).
    ///
    /// **UNVERIFIED as a measured bound.** No bench in this workspace
    /// times this, so the shape above is read from the source rather
    /// than measured. `CLAUDE.md` §3 rule 6.
    ///
    /// # Errors
    ///
    /// Every structural, semantic, lock and I/O refusal documented by
    /// [`Self::open`], plus any missing path.
    pub fn open_read(root: &Path) -> Result<Self, ExecutionCapabilityRefusal> {
        let lock_path = Self::lock_path(root);
        let writer_lock = File::open(&lock_path)
            .map_err(|why| format!("{} could not be opened: {why}", lock_path.display()))?;
        writer_lock
            .lock_shared()
            .map_err(|why| format!("{} could not be shared-locked: {why}", lock_path.display()))?;
        let opened = (|| {
            let paths = LedgerPaths::of(root);
            let files = LedgerFiles {
                parameter: File::open(&paths.parameter).map_err(|why| {
                    format!("{} could not be opened: {why}", paths.parameter.display())
                })?,
                percentile: File::open(&paths.percentile).map_err(|why| {
                    format!("{} could not be opened: {why}", paths.percentile.display())
                })?,
                capability: File::open(&paths.capability).map_err(|why| {
                    format!("{} could not be opened: {why}", paths.capability.display())
                })?,
                completion: File::open(&paths.completion).map_err(|why| {
                    format!("{} could not be opened: {why}", paths.completion.display())
                })?,
            };
            Self::from_files(
                files,
                paths,
                writer_lock.try_clone().map_err(|why| {
                    format!(
                        "{} lock handle could not be cloned: {why}",
                        lock_path.display()
                    )
                })?,
                false,
            )
        })();
        let released = writer_lock
            .unlock()
            .map_err(|why| format!("{} could not be unlocked: {why}", lock_path.display()));
        match (opened, released) {
            (Ok(ledger), Ok(())) => Ok(ledger),
            (Err(why), _) | (Ok(_), Err(why)) => Err(why),
        }
    }

    fn from_files(
        mut files: LedgerFiles,
        paths: LedgerPaths,
        writer_lock: File,
        writable: bool,
    ) -> Result<Self, ExecutionCapabilityRefusal> {
        check_record_file(
            &mut files.parameter,
            &paths.parameter,
            PARAMETER_MAGIC,
            EXECUTION_PARAMETER_STRIDE,
        )?;
        check_record_file(
            &mut files.percentile,
            &paths.percentile,
            PERCENTILE_MAGIC,
            EXECUTION_PERCENTILE_STRIDE,
        )?;
        check_record_file(
            &mut files.capability,
            &paths.capability,
            CAPABILITY_MAGIC,
            EXECUTION_CAPABILITY_STRIDE,
        )?;
        check_record_file(
            &mut files.completion,
            &paths.completion,
            COMPLETION_MAGIC,
            EXECUTION_COMPLETION_STRIDE,
        )?;

        let records = scan_execution_records(&mut files, &paths)?;
        let indexes = build_execution_indexes(&records)?;

        let parameter_generation = file_generation(&files.parameter, &paths.parameter)?;
        let percentile_generation = file_generation(&files.percentile, &paths.percentile)?;
        let capability_generation = file_generation(&files.capability, &paths.capability)?;
        let completion_generation = file_generation(&files.completion, &paths.completion)?;
        Ok(Self {
            parameter_file: files.parameter,
            percentile_file: files.percentile,
            capability_file: files.capability,
            completion_file: files.completion,
            writer_lock,
            parameter_path: paths.parameter,
            percentile_path: paths.percentile,
            capability_path: paths.capability,
            completion_path: paths.completion,
            parameter_generation,
            percentile_generation,
            capability_generation,
            completion_generation,
            authorities: indexes.authorities,
            capabilities: indexes.capabilities,
            writable,
        })
    }

    /// One average-O(1) completion lookup after constant-size generation checks.
    ///
    /// **UNVERIFIED as a measured bound.** No bench in this workspace
    /// times this, so the shape above is read from the source rather
    /// than measured. `CLAUDE.md` §3 rule 6.
    ///
    /// # Errors
    ///
    /// Refuses a lock failure or any post-open append, replacement or
    /// same-length mutation until the ledger is fully reopened.
    pub fn completion(
        &mut self,
        population_id: &[u8; 32],
    ) -> Result<Option<ExecutionCapabilityCompletionV1>, ExecutionCapabilityRefusal> {
        self.with_shared_lock("an execution completion lookup", |ledger| {
            ledger.require_generations_unchanged()?;
            Ok(ledger
                .authorities
                .get(population_id)
                .map(|entry| entry.receipt))
        })
    }

    /// One average-O(1) exact side-parameter lookup.
    ///
    /// **UNVERIFIED as a measured bound.** No bench in this workspace
    /// times this, so the shape above is read from the source rather
    /// than measured. `CLAUDE.md` §3 rule 6.
    ///
    /// # Errors
    ///
    /// Refuses a lock failure or any post-open file-generation change.
    pub fn parameters(
        &mut self,
        population_id: &[u8; 32],
        direction: TradeDirectionV1,
    ) -> Result<Option<&ExecutionParametersV1>, ExecutionCapabilityRefusal> {
        self.writer_lock.lock_shared().map_err(|why| {
            format!("the population writer lock could not be shared-locked: {why}")
        })?;
        let checked = self.require_generations_unchanged();
        let result = checked.map(|()| {
            self.authorities.get(population_id).map(|authority| {
                let [long, short] = &authority.parameters;
                match direction {
                    TradeDirectionV1::Long => long,
                    TradeDirectionV1::Short => short,
                }
            })
        });
        let released = self
            .writer_lock
            .unlock()
            .map_err(|why| format!("the population writer lock could not be unlocked: {why}"));
        match (result, released) {
            (Ok(value), Ok(())) => Ok(value),
            (Err(why), _) | (Ok(_), Err(why)) => Err(why),
        }
    }

    /// One average-O(1) committed row-capability lookup.
    ///
    /// **UNVERIFIED as a measured bound.** No bench in this workspace
    /// times this, so the shape above is read from the source rather
    /// than measured. `CLAUDE.md` §3 rule 6.
    ///
    /// # Errors
    ///
    /// Refuses a lock failure or any post-open file-generation change.
    pub fn capability(
        &mut self,
        population_id: &[u8; 32],
        row_sequence: u64,
    ) -> Result<Option<ExecutionStrategyCapabilityV1>, ExecutionCapabilityRefusal> {
        self.with_shared_lock("an execution row-capability lookup", |ledger| {
            ledger.require_generations_unchanged()?;
            Ok(ledger
                .capabilities
                .get(&(*population_id, row_sequence))
                .copied())
        })
    }

    /// Appends all prepared records and syncs the completion receipt last.
    ///
    /// A same-population rerun reuses only the same semantic authority. Valid
    /// records left by a crash remain append-only orphans and are never treated
    /// as committed without a receipt.
    ///
    /// # Errors
    ///
    /// Refuses a read-only or stale handle, changed semantic rerun, lock/I/O or
    /// sync failure, encoding failure, invalid range, or index overflow.
    pub fn append_complete(
        &mut self,
        prepared: &PreparedExecutionCapabilitiesV1,
    ) -> Result<ExecutionCapabilityCommit, ExecutionCapabilityRefusal> {
        if !self.writable {
            return Err(
                "a read-only execution-capability ledger cannot append; reopen with open"
                    .to_owned(),
            );
        }
        self.writer_lock.lock().map_err(|why| {
            format!("the population writer lock could not be exclusively locked: {why}")
        })?;
        let attempted = self.append_complete_locked(prepared);
        let released = self
            .writer_lock
            .unlock()
            .map_err(|why| format!("the population writer lock could not be unlocked: {why}"));
        match (attempted, released) {
            (Ok(value), Ok(())) => Ok(value),
            (Err(why), _) | (Ok(_), Err(why)) => Err(why),
        }
    }

    fn append_complete_locked(
        &mut self,
        prepared: &PreparedExecutionCapabilitiesV1,
    ) -> Result<ExecutionCapabilityCommit, ExecutionCapabilityRefusal> {
        self.require_generations_unchanged()?;
        if let Some(existing) = self.authorities.get(&prepared.population_id) {
            if !existing.receipt.same_semantics(prepared) {
                return Err(
                    "population already has a different execution capability authority".to_owned(),
                );
            }
            self.sync_all()?;
            return Ok(ExecutionCapabilityCommit::Reused);
        }

        let receipt = ExecutionCapabilityCompletionV1::for_prepared(
            prepared,
            record_count(
                &self.parameter_file,
                &self.parameter_path,
                EXECUTION_PARAMETER_STRIDE,
            )?,
            record_count(
                &self.percentile_file,
                &self.percentile_path,
                EXECUTION_PERCENTILE_STRIDE,
            )?,
            record_count(
                &self.capability_file,
                &self.capability_path,
                EXECUTION_CAPABILITY_STRIDE,
            )?,
        )?;
        self.append_prepared_records(prepared, &receipt)?;
        self.refresh_generations()?;
        self.index_prepared_authority(prepared, &receipt);
        Ok(ExecutionCapabilityCommit::Written)
    }

    fn append_prepared_records(
        &mut self,
        prepared: &PreparedExecutionCapabilitiesV1,
        receipt: &ExecutionCapabilityCompletionV1,
    ) -> Result<(), ExecutionCapabilityRefusal> {
        append_encoded(
            &mut self.parameter_file,
            &self.parameter_path,
            "execution parameter records",
            prepared.scalars.iter().map(ParameterScalarV1::to_bytes),
        )?;
        sync_record_file(&self.parameter_file, &self.parameter_path)?;
        append_encoded(
            &mut self.percentile_file,
            &self.percentile_path,
            "execution percentile records",
            prepared
                .percentiles
                .iter()
                .copied()
                .map(PercentileRecordV1::to_bytes),
        )?;
        sync_record_file(&self.percentile_file, &self.percentile_path)?;
        append_encoded(
            &mut self.capability_file,
            &self.capability_path,
            "execution row capabilities",
            prepared
                .capabilities
                .iter()
                .copied()
                .map(ExecutionStrategyCapabilityV1::to_bytes),
        )?;
        sync_record_file(&self.capability_file, &self.capability_path)?;
        append_encoded(
            &mut self.completion_file,
            &self.completion_path,
            "execution completion receipt",
            [(*receipt).to_bytes()],
        )?;
        sync_record_file(&self.completion_file, &self.completion_path)
    }

    fn refresh_generations(&mut self) -> Result<(), ExecutionCapabilityRefusal> {
        self.parameter_generation = file_generation(&self.parameter_file, &self.parameter_path)?;
        self.percentile_generation = file_generation(&self.percentile_file, &self.percentile_path)?;
        self.capability_generation = file_generation(&self.capability_file, &self.capability_path)?;
        self.completion_generation = file_generation(&self.completion_file, &self.completion_path)?;
        Ok(())
    }

    fn index_prepared_authority(
        &mut self,
        prepared: &PreparedExecutionCapabilitiesV1,
        receipt: &ExecutionCapabilityCompletionV1,
    ) {
        for capability in prepared.capabilities.iter().copied() {
            self.capabilities.insert(
                (capability.population_id, capability.row_sequence),
                capability,
            );
        }
        self.authorities.insert(
            prepared.population_id,
            CommittedExecutionAuthorityV1 {
                receipt: *receipt,
                parameters: prepared.parameters.clone(),
            },
        );
    }

    fn with_shared_lock<T>(
        &mut self,
        purpose: &str,
        operation: impl FnOnce(&mut Self) -> Result<T, ExecutionCapabilityRefusal>,
    ) -> Result<T, ExecutionCapabilityRefusal> {
        self.writer_lock.lock_shared().map_err(|why| {
            format!("the population writer lock could not be shared-locked for {purpose}: {why}")
        })?;
        let attempted = operation(self);
        let released = self.writer_lock.unlock().map_err(|why| {
            format!("the population writer lock could not be unlocked after {purpose}: {why}")
        });
        match (attempted, released) {
            (Ok(value), Ok(())) => Ok(value),
            (Err(why), _) | (Ok(_), Err(why)) => Err(why),
        }
    }

    fn require_generations_unchanged(&self) -> Result<(), ExecutionCapabilityRefusal> {
        for (expected, file, path, name) in [
            (
                self.parameter_generation,
                &self.parameter_file,
                &self.parameter_path,
                "execution parameter file",
            ),
            (
                self.percentile_generation,
                &self.percentile_file,
                &self.percentile_path,
                "execution percentile file",
            ),
            (
                self.capability_generation,
                &self.capability_file,
                &self.capability_path,
                "execution capability file",
            ),
            (
                self.completion_generation,
                &self.completion_file,
                &self.completion_path,
                "execution completion file",
            ),
        ] {
            let observed = file_generation(file, path)?;
            if observed != expected {
                return Err(format!(
                    "{} {name} changed after open; reopen and fully revalidate before use",
                    path.display()
                ));
            }
        }
        Ok(())
    }

    fn sync_all(&self) -> Result<(), ExecutionCapabilityRefusal> {
        for (file, path) in [
            (&self.parameter_file, &self.parameter_path),
            (&self.percentile_file, &self.percentile_path),
            (&self.capability_file, &self.capability_path),
            (&self.completion_file, &self.completion_path),
        ] {
            file.sync_all()
                .map_err(|why| format!("{} could not be synced: {why}", path.display()))?;
        }
        Ok(())
    }
}

struct LedgerFiles {
    parameter: File,
    percentile: File,
    capability: File,
    completion: File,
}

struct LedgerPaths {
    parameter: PathBuf,
    percentile: PathBuf,
    capability: PathBuf,
    completion: PathBuf,
}

impl LedgerPaths {
    fn of(root: &Path) -> Self {
        Self {
            parameter: ExecutionCapabilityLedger::parameter_path(root),
            percentile: ExecutionCapabilityLedger::percentile_path(root),
            capability: ExecutionCapabilityLedger::capability_path(root),
            completion: ExecutionCapabilityLedger::completion_path(root),
        }
    }
}

fn sync_record_file(file: &File, path: &Path) -> Result<(), ExecutionCapabilityRefusal> {
    file.sync_all()
        .map_err(|why| format!("{} could not be synced: {why}", path.display()))
}

struct ScannedExecutionRecords {
    parameters: Vec<ParameterScalarV1>,
    percentiles: Vec<PercentileRecordV1>,
    capabilities: Vec<ExecutionStrategyCapabilityV1>,
    completions: Vec<ExecutionCapabilityCompletionV1>,
}

struct ReopenedExecutionIndexes {
    authorities: HashMap<[u8; 32], CommittedExecutionAuthorityV1>,
    capabilities: HashMap<([u8; 32], u64), ExecutionStrategyCapabilityV1>,
}

#[derive(Default)]
struct PriorBlockEnds {
    parameters: u64,
    percentiles: u64,
    capabilities: u64,
}

fn scan_execution_records(
    files: &mut LedgerFiles,
    paths: &LedgerPaths,
) -> Result<ScannedExecutionRecords, ExecutionCapabilityRefusal> {
    Ok(ScannedExecutionRecords {
        parameters: scan_records::<EXECUTION_PARAMETER_STRIDE, ParameterScalarV1>(
            &mut files.parameter,
            &paths.parameter,
            ParameterScalarV1::from_bytes,
        )?,
        percentiles: scan_records::<EXECUTION_PERCENTILE_STRIDE, PercentileRecordV1>(
            &mut files.percentile,
            &paths.percentile,
            PercentileRecordV1::from_bytes,
        )?,
        capabilities: scan_records::<EXECUTION_CAPABILITY_STRIDE, ExecutionStrategyCapabilityV1>(
            &mut files.capability,
            &paths.capability,
            ExecutionStrategyCapabilityV1::from_bytes,
        )?,
        completions: scan_records::<EXECUTION_COMPLETION_STRIDE, ExecutionCapabilityCompletionV1>(
            &mut files.completion,
            &paths.completion,
            ExecutionCapabilityCompletionV1::from_bytes,
        )?,
    })
}

fn build_execution_indexes(
    records: &ScannedExecutionRecords,
) -> Result<ReopenedExecutionIndexes, ExecutionCapabilityRefusal> {
    let mut indexes = ReopenedExecutionIndexes {
        authorities: HashMap::new(),
        capabilities: HashMap::new(),
    };
    indexes
        .authorities
        .try_reserve(records.completions.len())
        .map_err(|why| format!("execution authority index allocation refused: {why}"))?;
    let capability_count = records
        .completions
        .iter()
        .try_fold(0_usize, |sum, receipt| {
            let count = usize::try_from(receipt.capability_count)
                .map_err(|_| "execution capability count does not fit usize".to_owned())?;
            sum.checked_add(count)
                .ok_or_else(|| "execution committed capability count overflow".to_owned())
        })?;
    indexes
        .capabilities
        .try_reserve(capability_count)
        .map_err(|why| format!("execution capability index allocation refused: {why}"))?;

    let mut prior_ends = PriorBlockEnds::default();
    for receipt in &records.completions {
        reopen_execution_authority(receipt, records, &mut prior_ends, &mut indexes)?;
    }
    Ok(indexes)
}

fn reopen_execution_authority(
    receipt: &ExecutionCapabilityCompletionV1,
    records: &ScannedExecutionRecords,
    prior_ends: &mut PriorBlockEnds,
    indexes: &mut ReopenedExecutionIndexes,
) -> Result<(), ExecutionCapabilityRefusal> {
    (*receipt).validate()?;
    require_monotonic_block(
        receipt.parameter_first,
        receipt.parameter_count,
        &mut prior_ends.parameters,
        "execution parameter",
    )?;
    require_monotonic_block(
        receipt.percentile_first,
        receipt.percentile_count,
        &mut prior_ends.percentiles,
        "execution percentile",
    )?;
    require_monotonic_block(
        receipt.capability_first,
        receipt.capability_count,
        &mut prior_ends.capabilities,
        "execution capability",
    )?;
    let parameter_block = block_slice(
        &records.parameters,
        receipt.parameter_first,
        receipt.parameter_count,
        "execution parameter",
    )?;
    let percentile_block = block_slice(
        &records.percentiles,
        receipt.percentile_first,
        receipt.percentile_count,
        "execution percentile",
    )?;
    let capability_block = block_slice(
        &records.capabilities,
        receipt.capability_first,
        receipt.capability_count,
        "execution capability",
    )?;
    if parameter_digest_records(parameter_block)? != receipt.ordered_parameter_digest
        || percentile_digest_records(percentile_block)? != receipt.ordered_percentile_digest
        || capability_digest_records(capability_block)? != receipt.ordered_capability_digest
    {
        return Err(
            "execution completion block digest differs from its referenced records".to_owned(),
        );
    }
    let (long_percentiles, short_percentiles) =
        split_parameter_percentiles(parameter_block, percentile_block)?;
    let [long_scalar, short_scalar] = parameter_block else {
        return Err("execution parameter block is not exactly two records".to_owned());
    };
    let long = long_scalar.reconstruct(long_percentiles)?;
    let short = short_scalar.reconstruct(short_percentiles)?;
    validate_parameter_pair(receipt, &long, &short)?;
    validate_capability_block(receipt, capability_block, &long, &short)?;
    if indexes.authorities.contains_key(&receipt.population_id) {
        return Err("duplicate execution completion for one population".to_owned());
    }
    for capability in capability_block.iter().copied() {
        let key = (capability.population_id, capability.row_sequence);
        if indexes.capabilities.insert(key, capability).is_some() {
            return Err("duplicate committed execution row capability".to_owned());
        }
    }
    indexes.authorities.insert(
        receipt.population_id,
        CommittedExecutionAuthorityV1 {
            receipt: *receipt,
            parameters: [long, short],
        },
    );
    Ok(())
}

pub(crate) fn parameter_digest_records(
    records: &[ParameterScalarV1],
) -> Result<[u8; 32], ExecutionCapabilityRefusal> {
    let mut hasher = Hasher::new();
    hasher.update(PARAMETER_ORDER_DOMAIN);
    hasher.update(&usize_u64(records.len(), "execution parameter block")?.to_le_bytes());
    for record in records {
        hasher.update(&record.payload()?);
    }
    Ok(hasher.finalize())
}

fn capability_digest_records(
    records: &[ExecutionStrategyCapabilityV1],
) -> Result<[u8; 32], ExecutionCapabilityRefusal> {
    let mut hasher = Hasher::new();
    hasher.update(CAPABILITY_ORDER_DOMAIN);
    hasher.update(&usize_u64(records.len(), "execution capability block")?.to_le_bytes());
    for record in records.iter().copied() {
        hasher.update(&record.payload()?);
    }
    Ok(hasher.finalize())
}

fn validate_parameter_pair(
    receipt: &ExecutionCapabilityCompletionV1,
    long: &ExecutionParametersV1,
    short: &ExecutionParametersV1,
) -> Result<(), ExecutionCapabilityRefusal> {
    long.validate()?;
    short.validate()?;
    if long.direction != TradeDirectionV1::Long
        || short.direction != TradeDirectionV1::Short
        || long.parameter_id != receipt.long_parameter_id
        || short.parameter_id != receipt.short_parameter_id
        || long.population_id != receipt.population_id
        || short.population_id != receipt.population_id
        || long.population_v4_digest != receipt.population_v4_digest
        || short.population_v4_digest != receipt.population_v4_digest
        || long.instrument_family != short.instrument_family
        || long.rung_seconds != short.rung_seconds
    {
        return Err(
            "execution completion has a copied, swapped or foreign parameter pair".to_owned(),
        );
    }
    Ok(())
}

fn validate_capability_block(
    receipt: &ExecutionCapabilityCompletionV1,
    records: &[ExecutionStrategyCapabilityV1],
    long: &ExecutionParametersV1,
    short: &ExecutionParametersV1,
) -> Result<(), ExecutionCapabilityRefusal> {
    if usize_u64(records.len(), "execution capability block")? != receipt.row_count {
        return Err("execution completion row_count differs from capability block".to_owned());
    }
    let mut long_count = 0_u64;
    let mut short_count = 0_u64;
    for (nth, record) in records.iter().copied().enumerate() {
        record.validate()?;
        let expected = usize_u64(nth, "execution capability sequence")?;
        let parameters = match record.direction {
            TradeDirectionV1::Long => {
                long_count = long_count
                    .checked_add(1)
                    .ok_or_else(|| "long execution count overflow".to_owned())?;
                long
            }
            TradeDirectionV1::Short => {
                short_count = short_count
                    .checked_add(1)
                    .ok_or_else(|| "short execution count overflow".to_owned())?;
                short
            }
        };
        if record.row_sequence != expected
            || record.population_id != receipt.population_id
            || record.population_v4_digest != receipt.population_v4_digest
            || record.parameter_id != parameters.parameter_id
            || record.instrument_family != parameters.instrument_family
            || record.rung_seconds != parameters.rung_seconds
        {
            return Err(
                "execution capability block is missing, reordered, copied or foreign".to_owned(),
            );
        }
    }
    if (long_count, short_count) != (receipt.long_count, receipt.short_count) {
        return Err("execution capability side counts differ from completion".to_owned());
    }
    Ok(())
}

fn split_parameter_percentiles<'a>(
    parameters: &[ParameterScalarV1],
    percentiles: &'a [PercentileRecordV1],
) -> Result<(&'a [PercentileRecordV1], &'a [PercentileRecordV1]), ExecutionCapabilityRefusal> {
    let [long, short] = parameters else {
        return Err("execution parameter block is not canonical long then short".to_owned());
    };
    if long.direction != TradeDirectionV1::Long || short.direction != TradeDirectionV1::Short {
        return Err("execution parameter block is not canonical long then short".to_owned());
    }
    let long_count = scalar_percentile_count(long)?;
    let short_count = scalar_percentile_count(short)?;
    let total = long_count
        .checked_add(short_count)
        .ok_or_else(|| "execution percentile block count overflow".to_owned())?;
    if total != percentiles.len() {
        return Err("execution percentile block does not reconcile to both parameters".to_owned());
    }
    Ok(percentiles.split_at(long_count))
}

fn scalar_percentile_count(
    scalar: &ParameterScalarV1,
) -> Result<usize, ExecutionCapabilityRefusal> {
    let count = u64::from(scalar.stop_count)
        .checked_add(u64::from(scalar.target_count))
        .and_then(|value| value.checked_add(u64::from(scalar.trail_count)))
        .ok_or_else(|| "execution scalar percentile count overflow".to_owned())?;
    usize::try_from(count).map_err(|_| "execution percentile count does not fit usize".to_owned())
}

fn require_monotonic_block(
    first: u64,
    count: u64,
    prior_end: &mut u64,
    subject: &str,
) -> Result<(), ExecutionCapabilityRefusal> {
    if first < *prior_end {
        return Err(format!(
            "{subject} completion blocks overlap or run backwards"
        ));
    }
    *prior_end = first
        .checked_add(count)
        .ok_or_else(|| format!("{subject} completion range overflow"))?;
    Ok(())
}

fn block_slice<'a, T>(
    records: &'a [T],
    first: u64,
    count: u64,
    subject: &str,
) -> Result<&'a [T], ExecutionCapabilityRefusal> {
    let first =
        usize::try_from(first).map_err(|_| format!("{subject} first record does not fit usize"))?;
    let count =
        usize::try_from(count).map_err(|_| format!("{subject} count does not fit usize"))?;
    let end = first
        .checked_add(count)
        .ok_or_else(|| format!("{subject} block address overflow"))?;
    records
        .get(first..end)
        .ok_or_else(|| format!("{subject} completion points outside the stored record file"))
}

fn usize_u64(value: usize, subject: &str) -> Result<u64, ExecutionCapabilityRefusal> {
    u64::try_from(value).map_err(|_| format!("{subject} count does not fit u64"))
}

fn scan_records<const STRIDE: usize, T>(
    file: &mut File,
    path: &Path,
    decode: fn(&[u8; STRIDE]) -> Result<T, ExecutionCapabilityRefusal>,
) -> Result<Vec<T>, ExecutionCapabilityRefusal> {
    let count = record_count(file, path, STRIDE)?;
    let capacity = usize::try_from(count)
        .map_err(|_| format!("{} record count does not fit usize", path.display()))?;
    let mut records = Vec::new();
    records
        .try_reserve_exact(capacity)
        .map_err(|why| format!("{} index allocation refused: {why}", path.display()))?;
    file.seek(SeekFrom::Start(HEADER_BYTES))
        .map_err(|why| format!("{} could not be seeked: {why}", path.display()))?;
    for sequence in 0..count {
        let mut raw = [0; STRIDE];
        file.read_exact(&mut raw).map_err(|why| {
            format!(
                "{} record {sequence} could not be read completely: {why}",
                path.display()
            )
        })?;
        records
            .push(decode(&raw).map_err(|why| {
                format!("{} record {sequence} is invalid: {why}", path.display())
            })?);
    }
    Ok(records)
}

fn append_encoded<const STRIDE: usize>(
    file: &mut File,
    path: &Path,
    subject: &str,
    records: impl IntoIterator<Item = Result<[u8; STRIDE], ExecutionCapabilityRefusal>>,
) -> Result<(), ExecutionCapabilityRefusal> {
    let at = file
        .seek(SeekFrom::End(0))
        .map_err(|why| format!("{} could not be extended: {why}", path.display()))?;
    for record in records {
        let raw = match record {
            Ok(raw) => raw,
            Err(why) => return Err(rollback_text(file, at, subject, &why)),
        };
        if let Err(why) = file.write_all(&raw) {
            return Err(rollback_io(file, at, subject, &why));
        }
    }
    Ok(())
}

fn open_or_create(path: &Path) -> Result<File, ExecutionCapabilityRefusal> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .map_err(|why| format!("{} could not be opened: {why}", path.display()))
}

fn ensure_record_header(
    file: &mut File,
    path: &Path,
    magic: [u8; 8],
    stride: usize,
) -> Result<(), ExecutionCapabilityRefusal> {
    let len = measured_len(file, path)?;
    if len != 0 {
        return Ok(());
    }
    let mut raw = [0; HEADER_BYTES_USIZE];
    let mut encoder = Encoder::new(&mut raw);
    encoder.bytes(&magic)?;
    encoder.u32(FORMAT_VERSION)?;
    encoder
        .u32(u32::try_from(HEADER_BYTES_USIZE).map_err(|_| "header width overflow".to_owned())?)?;
    encoder.u32(u32::try_from(stride).map_err(|_| "record stride overflow".to_owned())?)?;
    encoder.zeros(4)?;
    encoder.finish()?;
    file.write_all(&raw)
        .and_then(|()| file.sync_all())
        .map_err(|why| {
            format!(
                "{} could not receive a durable header: {why}",
                path.display()
            )
        })
}

fn check_record_file(
    file: &mut File,
    path: &Path,
    magic: [u8; 8],
    stride: usize,
) -> Result<(), ExecutionCapabilityRefusal> {
    let len = measured_len(file, path)?;
    if len < HEADER_BYTES {
        return Err(format!(
            "{} is shorter than its {}-byte header",
            path.display(),
            HEADER_BYTES
        ));
    }
    let mut raw = [0; HEADER_BYTES_USIZE];
    file.seek(SeekFrom::Start(0))
        .and_then(|_| file.read_exact(&mut raw))
        .map_err(|why| format!("{} header could not be read: {why}", path.display()))?;
    let mut decoder = Decoder::new(&raw);
    if decoder.bytes::<8>()? != magic {
        return Err(format!(
            "{} has the wrong execution-capability magic",
            path.display()
        ));
    }
    if decoder.u32()? != FORMAT_VERSION {
        return Err(format!(
            "{} has an unsupported format version",
            path.display()
        ));
    }
    if decoder.u32()? != u32::try_from(HEADER_BYTES_USIZE).unwrap_or(u32::MAX) {
        return Err(format!("{} has a different header width", path.display()));
    }
    if decoder.u32()? != u32::try_from(stride).unwrap_or(u32::MAX) {
        return Err(format!(
            "{} has a different fixed record stride",
            path.display()
        ));
    }
    decoder.zeros(4, "execution-capability header reserve")?;
    decoder.finish()?;
    let stride = u64::try_from(stride).map_err(|_| "record stride does not fit u64".to_owned())?;
    if !len.saturating_sub(HEADER_BYTES).is_multiple_of(stride) {
        return Err(format!(
            "{} has a ragged/torn tail; no bytes were ignored or padded",
            path.display()
        ));
    }
    Ok(())
}

fn record_count(
    file: &File,
    path: &Path,
    stride: usize,
) -> Result<u64, ExecutionCapabilityRefusal> {
    let len = measured_len(file, path)?;
    let body = len
        .checked_sub(HEADER_BYTES)
        .ok_or_else(|| format!("{} ended before its header", path.display()))?;
    let stride = u64::try_from(stride).map_err(|_| "record stride does not fit u64".to_owned())?;
    if !body.is_multiple_of(stride) {
        return Err(format!("{} has a ragged fixed-stride tail", path.display()));
    }
    Ok(body / stride)
}

fn measured_len(file: &File, path: &Path) -> Result<u64, ExecutionCapabilityRefusal> {
    file.metadata()
        .map(|metadata| metadata.len())
        .map_err(|why| format!("{} could not be measured: {why}", path.display()))
}

fn file_generation(file: &File, path: &Path) -> Result<FileGeneration, ExecutionCapabilityRefusal> {
    let held = file
        .metadata()
        .map_err(|why| format!("{} open file could not be measured: {why}", path.display()))?;
    let named = fs::metadata(path)
        .map_err(|why| format!("{} path could not be measured: {why}", path.display()))?;
    platform_generation(&held, &named, path)
}

#[cfg(unix)]
fn platform_generation(
    held: &fs::Metadata,
    named: &fs::Metadata,
    path: &Path,
) -> Result<FileGeneration, ExecutionCapabilityRefusal> {
    let of = |metadata: &fs::Metadata| FileGeneration {
        len: metadata.len(),
        platform: PlatformGeneration {
            device: metadata.dev(),
            inode: metadata.ino(),
            modified_seconds: metadata.mtime(),
            modified_nanoseconds: metadata.mtime_nsec(),
            changed_seconds: metadata.ctime(),
            changed_nanoseconds: metadata.ctime_nsec(),
        },
    };
    let held = of(held);
    let named = of(named);
    if (held.platform.device, held.platform.inode) != (named.platform.device, named.platform.inode)
    {
        return Err(format!(
            "{} no longer names the opened file",
            path.display()
        ));
    }
    if held != named {
        return Err(format!(
            "{} changed while its generation was measured",
            path.display()
        ));
    }
    Ok(held)
}

#[cfg(windows)]
fn platform_generation(
    held: &fs::Metadata,
    named: &fs::Metadata,
    path: &Path,
) -> Result<FileGeneration, ExecutionCapabilityRefusal> {
    let of = |metadata: &fs::Metadata| -> Result<FileGeneration, ExecutionCapabilityRefusal> {
        Ok(FileGeneration {
            len: metadata.len(),
            platform: PlatformGeneration {
                volume_serial: metadata
                    .volume_serial_number()
                    .ok_or_else(|| format!("{} has no Windows volume serial", path.display()))?,
                file_index: metadata
                    .file_index()
                    .ok_or_else(|| format!("{} has no Windows file index", path.display()))?,
                creation_time: metadata.creation_time(),
                last_write_time: metadata.last_write_time(),
            },
        })
    };
    let held = of(held)?;
    let named = of(named)?;
    if (held.platform.volume_serial, held.platform.file_index)
        != (named.platform.volume_serial, named.platform.file_index)
    {
        return Err(format!(
            "{} no longer names the opened file",
            path.display()
        ));
    }
    if held != named {
        return Err(format!(
            "{} changed while its generation was measured",
            path.display()
        ));
    }
    Ok(held)
}

#[cfg(not(any(unix, windows)))]
fn platform_generation(
    held: &fs::Metadata,
    named: &fs::Metadata,
    path: &Path,
) -> Result<FileGeneration, ExecutionCapabilityRefusal> {
    if held.len() != named.len() {
        return Err(format!(
            "{} changed length while it was measured",
            path.display()
        ));
    }
    Ok(FileGeneration {
        len: held.len(),
        platform: PlatformGeneration,
    })
}

fn sync_directory(path: &Path) -> Result<(), ExecutionCapabilityRefusal> {
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|why| {
            format!(
                "{} could not durably confirm file names: {why}",
                path.display()
            )
        })
}

fn rollback_io(file: &File, at: u64, subject: &str, why: &std::io::Error) -> String {
    match file.set_len(at) {
        Ok(()) => format!(
            "the {subject} could not be written: {why}; partial bytes were rolled back to {at}"
        ),
        Err(and) => format!(
            "the {subject} could not be written: {why}; rollback to {at} also failed: {and}; reopen refuses the ragged tail"
        ),
    }
}

fn rollback_text(file: &File, at: u64, subject: &str, why: &str) -> String {
    match file.set_len(at) {
        Ok(()) => format!(
            "the {subject} could not be encoded: {why}; partial bytes were rolled back to {at}"
        ),
        Err(and) => format!(
            "the {subject} could not be encoded: {why}; rollback to {at} also failed: {and}; reopen refuses the ragged tail"
        ),
    }
}

fn with_seal<const PAYLOAD: usize, const STRIDE: usize>(
    payload: [u8; PAYLOAD],
) -> Result<[u8; STRIDE], ExecutionCapabilityRefusal> {
    if STRIDE != PAYLOAD.saturating_add(SEAL_BYTES) {
        return Err("execution record payload/stride constants disagree".to_owned());
    }
    let mut raw = [0; STRIDE];
    raw.get_mut(..PAYLOAD)
        .ok_or_else(|| "execution record payload slot is absent".to_owned())?
        .copy_from_slice(&payload);
    raw.get_mut(PAYLOAD..)
        .ok_or_else(|| "execution record seal slot is absent".to_owned())?
        .copy_from_slice(&brutex_core::blake3::hash(&payload));
    Ok(raw)
}

fn checked_payload<const PAYLOAD: usize, const STRIDE: usize>(
    raw: &[u8; STRIDE],
    subject: &str,
) -> Result<[u8; PAYLOAD], ExecutionCapabilityRefusal> {
    if STRIDE != PAYLOAD.saturating_add(SEAL_BYTES) {
        return Err(format!("{subject} payload/stride constants disagree"));
    }
    let payload: [u8; PAYLOAD] = raw
        .get(..PAYLOAD)
        .and_then(|bytes| bytes.try_into().ok())
        .ok_or_else(|| format!("{subject} payload is absent"))?;
    let stored = raw
        .get(PAYLOAD..)
        .ok_or_else(|| format!("{subject} seal is absent"))?;
    if stored != brutex_core::blake3::hash(&payload) {
        return Err(format!("{subject} failed its BLAKE3 seal"));
    }
    Ok(payload)
}

struct Encoder<'a> {
    bytes: &'a mut [u8],
    at: usize,
}

impl<'a> Encoder<'a> {
    const fn new(bytes: &'a mut [u8]) -> Self {
        Self { bytes, at: 0 }
    }

    fn bytes(&mut self, value: &[u8]) -> Result<(), ExecutionCapabilityRefusal> {
        let end = self
            .at
            .checked_add(value.len())
            .ok_or_else(|| "execution encoder offset overflow".to_owned())?;
        self.bytes
            .get_mut(self.at..end)
            .ok_or_else(|| "execution encoder exceeded its fixed record".to_owned())?
            .copy_from_slice(value);
        self.at = end;
        Ok(())
    }

    fn u8(&mut self, value: u8) -> Result<(), ExecutionCapabilityRefusal> {
        self.bytes(&[value])
    }

    fn u16(&mut self, value: u16) -> Result<(), ExecutionCapabilityRefusal> {
        self.bytes(&value.to_le_bytes())
    }

    fn u32(&mut self, value: u32) -> Result<(), ExecutionCapabilityRefusal> {
        self.bytes(&value.to_le_bytes())
    }

    fn u64(&mut self, value: u64) -> Result<(), ExecutionCapabilityRefusal> {
        self.bytes(&value.to_le_bytes())
    }

    fn i64(&mut self, value: i64) -> Result<(), ExecutionCapabilityRefusal> {
        self.bytes(&value.to_le_bytes())
    }

    fn zeros(&mut self, count: usize) -> Result<(), ExecutionCapabilityRefusal> {
        let end = self
            .at
            .checked_add(count)
            .ok_or_else(|| "execution encoder reserve overflow".to_owned())?;
        let reserve = self
            .bytes
            .get_mut(self.at..end)
            .ok_or_else(|| "execution encoder reserve exceeded its record".to_owned())?;
        reserve.fill(0);
        self.at = end;
        Ok(())
    }

    fn finish(self) -> Result<(), ExecutionCapabilityRefusal> {
        if self.at == self.bytes.len() {
            Ok(())
        } else {
            Err(format!(
                "execution encoder wrote {} of {} fixed bytes",
                self.at,
                self.bytes.len()
            ))
        }
    }
}

struct Decoder<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Decoder<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, at: 0 }
    }

    fn bytes<const N: usize>(&mut self) -> Result<[u8; N], ExecutionCapabilityRefusal> {
        let end = self
            .at
            .checked_add(N)
            .ok_or_else(|| "execution decoder offset overflow".to_owned())?;
        let value = self
            .bytes
            .get(self.at..end)
            .and_then(|bytes| bytes.try_into().ok())
            .ok_or_else(|| "execution decoder reached a short fixed record".to_owned())?;
        self.at = end;
        Ok(value)
    }

    fn array_32(&mut self) -> Result<[u8; 32], ExecutionCapabilityRefusal> {
        self.bytes()
    }

    fn u8(&mut self) -> Result<u8, ExecutionCapabilityRefusal> {
        Ok(self.bytes::<1>()?[0])
    }

    fn u16(&mut self) -> Result<u16, ExecutionCapabilityRefusal> {
        Ok(u16::from_le_bytes(self.bytes()?))
    }

    fn u32(&mut self) -> Result<u32, ExecutionCapabilityRefusal> {
        Ok(u32::from_le_bytes(self.bytes()?))
    }

    fn u64(&mut self) -> Result<u64, ExecutionCapabilityRefusal> {
        Ok(u64::from_le_bytes(self.bytes()?))
    }

    fn i64(&mut self) -> Result<i64, ExecutionCapabilityRefusal> {
        Ok(i64::from_le_bytes(self.bytes()?))
    }

    fn zeros(&mut self, count: usize, subject: &str) -> Result<(), ExecutionCapabilityRefusal> {
        let end = self
            .at
            .checked_add(count)
            .ok_or_else(|| format!("{subject} offset overflow"))?;
        let reserve = self
            .bytes
            .get(self.at..end)
            .ok_or_else(|| format!("{subject} is absent"))?;
        if reserve.iter().any(|byte| *byte != 0) {
            return Err(format!("{subject} contains unknown non-zero bytes"));
        }
        self.at = end;
        Ok(())
    }

    fn finish(self) -> Result<(), ExecutionCapabilityRefusal> {
        if self.at == self.bytes.len() {
            Ok(())
        } else {
            Err(format!(
                "execution decoder consumed {} of {} fixed bytes",
                self.at,
                self.bytes.len()
            ))
        }
    }
}

fn require_row_matches(
    parameters: &ExecutionParametersV1,
    row: &PopulationRowV1,
) -> Result<(), ExecutionCapabilityRefusal> {
    let _ = row.payload_digest()?;
    if row.population_id != parameters.population_id
        || row.direction != parameters.direction
        || row.instrument_family != parameters.instrument_family
        || row.rung_seconds != parameters.rung_seconds
    {
        return Err("population row does not belong to the execution parameter set".to_owned());
    }
    Ok(())
}

fn chosen_from_population(row: &PopulationRowV1) -> Result<Chosen, ExecutionCapabilityRefusal> {
    Ok(Chosen {
        stop: optional_index(row.exit.stop, "stop")?,
        target: optional_index(row.exit.target, "target")?,
        tsl: optional_index(row.exit.tsl, "tsl")?,
        ttp: row
            .exit
            .ttp
            .map(|(arm, trail)| {
                Ok::<Ttp, ExecutionCapabilityRefusal>(Ttp {
                    arm: index(arm, "TTP arm")?,
                    trail: index(trail, "TTP trail")?,
                })
            })
            .transpose()?,
    })
}

fn optional_index(
    value: Option<u32>,
    name: &str,
) -> Result<Option<usize>, ExecutionCapabilityRefusal> {
    value.map(|present| index(present, name)).transpose()
}

fn index(value: u32, name: &str) -> Result<usize, ExecutionCapabilityRefusal> {
    if value == u32::MAX {
        return Err(format!("{name} uses the reserved u32::MAX sentinel"));
    }
    usize::try_from(value).map_err(|_| format!("{name} does not fit this platform"))
}

fn family_from_resolved(resolved: &ResolvedExitGridV1) -> InstrumentFamilyV1 {
    match resolved.family() {
        runner::exit_grid_policy::InstrumentFamilyV1::Nifty => InstrumentFamilyV1::Nifty,
        runner::exit_grid_policy::InstrumentFamilyV1::BankNifty => InstrumentFamilyV1::BankNifty,
    }
}

const fn side_of_direction(direction: TradeDirectionV1) -> Side {
    match direction {
        TradeDirectionV1::Long => Side::Long,
        TradeDirectionV1::Short => Side::Short,
    }
}

const fn direction_byte(direction: TradeDirectionV1) -> u8 {
    match direction {
        TradeDirectionV1::Long => 1,
        TradeDirectionV1::Short => 2,
    }
}

fn direction_from_byte(byte: u8) -> Result<TradeDirectionV1, ExecutionCapabilityRefusal> {
    match byte {
        1 => Ok(TradeDirectionV1::Long),
        2 => Ok(TradeDirectionV1::Short),
        _ => Err(format!("execution direction tag {byte} is unknown")),
    }
}

const fn family_byte(family: InstrumentFamilyV1) -> u8 {
    match family {
        InstrumentFamilyV1::Nifty => 1,
        InstrumentFamilyV1::BankNifty => 2,
    }
}

fn family_from_byte(byte: u8) -> Result<InstrumentFamilyV1, ExecutionCapabilityRefusal> {
    match byte {
        1 => Ok(InstrumentFamilyV1::Nifty),
        2 => Ok(InstrumentFamilyV1::BankNifty),
        _ => Err(format!("execution instrument-family tag {byte} is unknown")),
    }
}

fn validate_rung_seconds(rung: u32) -> Result<(), ExecutionCapabilityRefusal> {
    if [60, 120, 180, 300, 600, 900, 1_800, 3_600].contains(&rung) {
        Ok(())
    } else {
        Err(format!(
            "execution signal rung {rung}s is not one of the eight canonical rungs"
        ))
    }
}

fn require_digest(name: &str, digest: &[u8; 32]) -> Result<(), ExecutionCapabilityRefusal> {
    if digest.iter().all(|byte| *byte == 0) {
        Err(format!("{name} is all zero"))
    } else {
        Ok(())
    }
}

fn put_percentiles(
    hasher: &mut Hasher,
    values: &[RationalPercentileV1],
) -> Result<(), ExecutionCapabilityRefusal> {
    let count =
        u64::try_from(values.len()).map_err(|_| "percentile count does not fit u64".to_owned())?;
    hasher.update(&count.to_le_bytes());
    for value in values {
        hasher.update(&value.numerator().to_le_bytes());
        hasher.update(&value.denominator().to_le_bytes());
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PercentileAxisV1 {
    Stop,
    Target,
    Trail,
}

impl PercentileAxisV1 {
    const fn byte(self) -> u8 {
        match self {
            Self::Stop => 1,
            Self::Target => 2,
            Self::Trail => 3,
        }
    }

    fn from_byte(byte: u8) -> Result<Self, ExecutionCapabilityRefusal> {
        match byte {
            1 => Ok(Self::Stop),
            2 => Ok(Self::Target),
            3 => Ok(Self::Trail),
            _ => Err(format!("execution percentile axis tag {byte} is unknown")),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PercentileRecordV1 {
    parameter_id: [u8; 32],
    population_id: [u8; 32],
    axis: PercentileAxisV1,
    ordinal: u32,
    numerator: u32,
    denominator: u32,
}

impl PercentileRecordV1 {
    pub(crate) fn from_parameters(
        parameters: &ExecutionParametersV1,
    ) -> Result<Vec<Self>, ExecutionCapabilityRefusal> {
        let count = parameters
            .policy
            .rungs()
            .stop()
            .len()
            .checked_add(parameters.policy.rungs().target().len())
            .and_then(|value| value.checked_add(parameters.policy.rungs().trail().len()))
            .ok_or_else(|| "execution percentile count overflow".to_owned())?;
        let mut records = Vec::new();
        records
            .try_reserve_exact(count)
            .map_err(|why| format!("execution percentile allocation refused: {why}"))?;
        for (axis, values) in [
            (PercentileAxisV1::Stop, parameters.policy.rungs().stop()),
            (PercentileAxisV1::Target, parameters.policy.rungs().target()),
            (PercentileAxisV1::Trail, parameters.policy.rungs().trail()),
        ] {
            for (ordinal, value) in values.iter().copied().enumerate() {
                records.push(Self {
                    parameter_id: parameters.parameter_id,
                    population_id: parameters.population_id,
                    axis,
                    ordinal: u32::try_from(ordinal)
                        .map_err(|_| "execution percentile ordinal does not fit u32".to_owned())?,
                    numerator: value.numerator(),
                    denominator: value.denominator(),
                });
            }
        }
        Ok(records)
    }

    fn validate(self) -> Result<(), ExecutionCapabilityRefusal> {
        require_digest("percentile parameter_id", &self.parameter_id)?;
        require_digest("percentile population_id", &self.population_id)?;
        let rebuilt = RationalPercentileV1::new(self.numerator, self.denominator)
            .map_err(|why| format!("execution percentile is invalid: {why:?}"))?;
        if rebuilt.numerator() != self.numerator || rebuilt.denominator() != self.denominator {
            return Err("execution percentile is not in reduced canonical form".to_owned());
        }
        Ok(())
    }

    fn payload(self) -> Result<[u8; PERCENTILE_PAYLOAD_BYTES], ExecutionCapabilityRefusal> {
        self.validate()?;
        let mut raw = [0; PERCENTILE_PAYLOAD_BYTES];
        let mut encoder = Encoder::new(&mut raw);
        encoder.bytes(&self.parameter_id)?;
        encoder.bytes(&self.population_id)?;
        encoder.u8(self.axis.byte())?;
        encoder.zeros(3)?;
        encoder.u32(self.ordinal)?;
        encoder.u32(self.numerator)?;
        encoder.u32(self.denominator)?;
        encoder.zeros(16)?;
        encoder.finish()?;
        Ok(raw)
    }

    pub(crate) fn to_bytes(
        self,
    ) -> Result<[u8; EXECUTION_PERCENTILE_STRIDE], ExecutionCapabilityRefusal> {
        let payload = self.payload()?;
        with_seal(payload)
    }

    pub(crate) fn from_bytes(
        raw: &[u8; EXECUTION_PERCENTILE_STRIDE],
    ) -> Result<Self, ExecutionCapabilityRefusal> {
        let payload = checked_payload::<PERCENTILE_PAYLOAD_BYTES, EXECUTION_PERCENTILE_STRIDE>(
            raw,
            "execution percentile",
        )?;
        let mut decoder = Decoder::new(&payload);
        let record = Self {
            parameter_id: decoder.array_32()?,
            population_id: decoder.array_32()?,
            axis: PercentileAxisV1::from_byte(decoder.u8()?)?,
            ordinal: {
                decoder.zeros(3, "execution percentile tag reserve")?;
                decoder.u32()?
            },
            numerator: decoder.u32()?,
            denominator: decoder.u32()?,
        };
        decoder.zeros(16, "execution percentile trailing reserve")?;
        decoder.finish()?;
        record.validate()?;
        Ok(record)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ParameterScalarV1 {
    parameter_id: [u8; 32],
    population_id: [u8; 32],
    population_v4_digest: [u8; 32],
    stored_policy_digest: [u8; 32],
    expected_resolved_digest: [u8; 32],
    training_digest: [u8; 32],
    evaluation_fingerprint: [u8; EVALUATION_SPEC_FINGERPRINT_V1_LEN],
    direction: TradeDirectionV1,
    instrument_family: InstrumentFamilyV1,
    execution_resolution: ExecutionResolutionV1,
    range_resolution: RangeResolutionV1,
    selector: ExitGridSelectorV1,
    forced_stop: ForcedStopV1,
    rung_seconds: u32,
    horizon_bars: u32,
    stop_count: u32,
    target_count: u32,
    trail_count: u32,
    run_params: Params,
    max_levels_per_axis: u64,
    ratio_min_hundredths: i64,
    ratio_max_hundredths: i64,
    max_ratio_pairs: u64,
    max_cells: u64,
    max_ambiguous_bars: u64,
    max_gap_fills: u64,
    training_bars: u64,
    training_first_ts_micros: i64,
    training_last_ts_micros: i64,
    ordered_percentile_digest: [u8; 32],
    cost_model_id: [u8; 32],
    execution_law_digest: [u8; 32],
}

impl ParameterScalarV1 {
    pub(crate) fn from_parameters(
        parameters: &ExecutionParametersV1,
    ) -> Result<Self, ExecutionCapabilityRefusal> {
        parameters.validate()?;
        let policy = &parameters.policy;
        Ok(Self {
            parameter_id: parameters.parameter_id,
            population_id: parameters.population_id,
            population_v4_digest: parameters.population_v4_digest,
            stored_policy_digest: policy.digest(),
            expected_resolved_digest: parameters.expected_resolved_digest,
            training_digest: parameters.training_digest,
            evaluation_fingerprint: parameters.evaluation_fingerprint,
            direction: parameters.direction,
            instrument_family: parameters.instrument_family,
            execution_resolution: policy.execution_resolution(),
            range_resolution: policy.range_resolution(),
            selector: policy.selector(),
            forced_stop: policy.forced_stop(),
            rung_seconds: parameters.rung_seconds,
            horizon_bars: parameters.horizon.as_bars(),
            stop_count: count_u32(policy.rungs().stop().len(), "stop percentile")?,
            target_count: count_u32(policy.rungs().target().len(), "target percentile")?,
            trail_count: count_u32(policy.rungs().trail().len(), "trail percentile")?,
            run_params: parameters.run_params,
            max_levels_per_axis: u64::try_from(policy.rungs().max_levels_per_axis())
                .map_err(|_| "max_levels_per_axis does not fit u64".to_owned())?,
            ratio_min_hundredths: policy.ratios().min_hundredths(),
            ratio_max_hundredths: policy.ratios().max_hundredths(),
            max_ratio_pairs: policy.ratios().max_pairs(),
            max_cells: policy.max_cells(),
            max_ambiguous_bars: policy.max_ambiguous_bars(),
            max_gap_fills: policy.max_gap_fills(),
            training_bars: parameters.training_bars,
            training_first_ts_micros: parameters.training_first_ts_micros,
            training_last_ts_micros: parameters.training_last_ts_micros,
            ordered_percentile_digest: percentile_digest(parameters)?,
            cost_model_id: policy.cost_model_id(),
            execution_law_digest: exact_execution_law_digest_v1(),
        })
    }

    fn payload(&self) -> Result<[u8; PARAMETER_PAYLOAD_BYTES], ExecutionCapabilityRefusal> {
        let mut raw = [0; PARAMETER_PAYLOAD_BYTES];
        let mut encoder = Encoder::new(&mut raw);
        for digest in [
            self.parameter_id,
            self.population_id,
            self.population_v4_digest,
            self.stored_policy_digest,
            self.expected_resolved_digest,
            self.training_digest,
        ] {
            encoder.bytes(&digest)?;
        }
        encoder.bytes(&self.evaluation_fingerprint)?;
        encoder.u8(direction_byte(self.direction))?;
        encoder.u8(family_byte(self.instrument_family))?;
        let (execution_tag, execution_seconds) = execution_parts(self.execution_resolution);
        encoder.u8(execution_tag)?;
        encoder.u8(range_resolution_byte(self.range_resolution))?;
        encoder.u8(selector_byte(self.selector))?;
        let (forced_stop_tag, forced_stop_ppm) = forced_stop_parts(self.forced_stop);
        encoder.u8(forced_stop_tag)?;
        encoder.u8(ENTRY_POLICY_TAG)?;
        encoder.u8(FORCED_EXIT_POLICY_TAG)?;
        encoder.zeros(1)?;
        encoder.u32(self.rung_seconds)?;
        encoder.u32(self.horizon_bars)?;
        encoder.u32(execution_seconds)?;
        encoder.u32(self.stop_count)?;
        encoder.u32(self.target_count)?;
        encoder.u32(self.trail_count)?;
        encoder.u16(ENTRY_DELAY_MINUTES)?;
        encoder.u16(FORCED_EXIT_IST_MINUTE)?;
        for value in [
            self.run_params.min_hits,
            self.run_params.ceiling,
            self.run_params.pair_budget,
            self.run_params.policy,
            self.max_levels_per_axis,
        ] {
            encoder.u64(value)?;
        }
        encoder.i64(self.ratio_min_hundredths)?;
        encoder.i64(self.ratio_max_hundredths)?;
        encoder.u64(self.max_ratio_pairs)?;
        encoder.u64(self.max_cells)?;
        encoder.i64(forced_stop_ppm)?;
        encoder.u64(self.max_ambiguous_bars)?;
        encoder.u64(self.max_gap_fills)?;
        encoder.u64(self.training_bars)?;
        encoder.i64(self.training_first_ts_micros)?;
        encoder.i64(self.training_last_ts_micros)?;
        encoder.bytes(&self.ordered_percentile_digest)?;
        encoder.bytes(&self.cost_model_id)?;
        encoder.bytes(&self.execution_law_digest)?;
        encoder.zeros(8)?;
        encoder.finish()?;
        Ok(raw)
    }

    pub(crate) fn to_bytes(
        &self,
    ) -> Result<[u8; EXECUTION_PARAMETER_STRIDE], ExecutionCapabilityRefusal> {
        with_seal(self.payload()?)
    }

    pub(crate) fn from_bytes(
        raw: &[u8; EXECUTION_PARAMETER_STRIDE],
    ) -> Result<Self, ExecutionCapabilityRefusal> {
        let payload = checked_payload::<PARAMETER_PAYLOAD_BYTES, EXECUTION_PARAMETER_STRIDE>(
            raw,
            "execution parameter",
        )?;
        let mut decoder = Decoder::new(&payload);
        let parameter_id = decoder.array_32()?;
        let population_id = decoder.array_32()?;
        let population_v4_digest = decoder.array_32()?;
        let stored_policy_digest = decoder.array_32()?;
        let expected_resolved_digest = decoder.array_32()?;
        let training_digest = decoder.array_32()?;
        let evaluation_fingerprint = decoder.bytes::<EVALUATION_SPEC_FINGERPRINT_V1_LEN>()?;
        let direction = direction_from_byte(decoder.u8()?)?;
        let instrument_family = family_from_byte(decoder.u8()?)?;
        let execution_tag = decoder.u8()?;
        let range_resolution = range_resolution_from_byte(decoder.u8()?)?;
        let selector = selector_from_byte(decoder.u8()?)?;
        let forced_stop_tag = decoder.u8()?;
        if decoder.u8()? != ENTRY_POLICY_TAG {
            return Err("execution parameter entry-policy tag is unknown".to_owned());
        }
        if decoder.u8()? != FORCED_EXIT_POLICY_TAG {
            return Err("execution parameter forced-exit-policy tag is unknown".to_owned());
        }
        decoder.zeros(1, "execution parameter tag reserve")?;
        let rung_seconds = decoder.u32()?;
        let horizon_bars = decoder.u32()?;
        let execution_seconds = decoder.u32()?;
        let stop_count = decoder.u32()?;
        let target_count = decoder.u32()?;
        let trail_count = decoder.u32()?;
        if decoder.u16()? != ENTRY_DELAY_MINUTES {
            return Err("execution parameter does not name next-minute entry".to_owned());
        }
        if decoder.u16()? != FORCED_EXIT_IST_MINUTE {
            return Err("execution parameter does not name fixed 15:10 IST close".to_owned());
        }
        let run_params = Params {
            min_hits: decoder.u64()?,
            ceiling: decoder.u64()?,
            pair_budget: decoder.u64()?,
            policy: decoder.u64()?,
        };
        let max_levels_per_axis = decoder.u64()?;
        let ratio_min_hundredths = decoder.i64()?;
        let ratio_max_hundredths = decoder.i64()?;
        let max_ratio_pairs = decoder.u64()?;
        let max_cells = decoder.u64()?;
        let forced_stop_ppm = decoder.i64()?;
        let max_ambiguous_bars = decoder.u64()?;
        let max_gap_fills = decoder.u64()?;
        let training_bars = decoder.u64()?;
        let training_first_ts_micros = decoder.i64()?;
        let training_last_ts_micros = decoder.i64()?;
        let ordered_percentile_digest = decoder.array_32()?;
        let cost_model_id = decoder.array_32()?;
        let execution_law_digest = decoder.array_32()?;
        decoder.zeros(8, "execution parameter trailing reserve")?;
        decoder.finish()?;
        let scalar = Self {
            parameter_id,
            population_id,
            population_v4_digest,
            stored_policy_digest,
            expected_resolved_digest,
            training_digest,
            evaluation_fingerprint,
            direction,
            instrument_family,
            execution_resolution: execution_from_parts(execution_tag, execution_seconds)?,
            range_resolution,
            selector,
            forced_stop: forced_stop_from_parts(forced_stop_tag, forced_stop_ppm)?,
            rung_seconds,
            horizon_bars,
            stop_count,
            target_count,
            trail_count,
            run_params,
            max_levels_per_axis,
            ratio_min_hundredths,
            ratio_max_hundredths,
            max_ratio_pairs,
            max_cells,
            max_ambiguous_bars,
            max_gap_fills,
            training_bars,
            training_first_ts_micros,
            training_last_ts_micros,
            ordered_percentile_digest,
            cost_model_id,
            execution_law_digest,
        };
        scalar.validate_shape()?;
        Ok(scalar)
    }

    fn validate_shape(&self) -> Result<(), ExecutionCapabilityRefusal> {
        for (name, digest) in [
            ("parameter_id", self.parameter_id),
            ("population_id", self.population_id),
            ("population_v4_digest", self.population_v4_digest),
            ("stored_policy_digest", self.stored_policy_digest),
            ("expected_resolved_digest", self.expected_resolved_digest),
            ("training_digest", self.training_digest),
            ("ordered_percentile_digest", self.ordered_percentile_digest),
            ("cost_model_id", self.cost_model_id),
            ("execution_law_digest", self.execution_law_digest),
        ] {
            require_digest(name, &digest)?;
        }
        validate_rung_seconds(self.rung_seconds)?;
        if self.horizon_bars == 0 {
            return Err("execution parameter horizon is zero".to_owned());
        }
        if self.execution_resolution != ExecutionResolutionV1::OneMinuteOhlcv {
            return Err("execution parameter resolution is not one-minute OHLCV".to_owned());
        }
        if self.cost_model_id != printed_ohlcv_cost_model_id_v1() {
            return Err("execution parameter fill/cost model is unsupported".to_owned());
        }
        if self.execution_law_digest != exact_execution_law_digest_v1() {
            return Err("execution parameter next-minute/15:10 law digest differs".to_owned());
        }
        if self.training_bars == 0 || self.training_first_ts_micros > self.training_last_ts_micros {
            return Err("execution parameter training geometry is invalid".to_owned());
        }
        Ok(())
    }

    pub(crate) fn reconstruct(
        &self,
        records: &[PercentileRecordV1],
    ) -> Result<ExecutionParametersV1, ExecutionCapabilityRefusal> {
        self.validate_shape()?;
        if percentile_digest_records(records)? != self.ordered_percentile_digest {
            return Err(
                "execution percentile block digest differs from parameter record".to_owned(),
            );
        }
        let (stop, target, trail) = decode_percentile_axes(self, records)?;
        let max_levels_per_axis = usize::try_from(self.max_levels_per_axis)
            .map_err(|_| "max_levels_per_axis does not fit usize".to_owned())?;
        let rungs = RungPlanV1::new(stop, target, trail, max_levels_per_axis)
            .map_err(|why| format!("execution rung plan refused: {why:?}"))?;
        let ratios = RatioLimitsV1::new(
            self.ratio_min_hundredths,
            self.ratio_max_hundredths,
            self.max_ratio_pairs,
        )
        .map_err(|why| format!("execution ratio limits refused: {why:?}"))?;
        let policy = ExitGridPolicyV1::new(
            self.execution_resolution,
            self.range_resolution,
            side_of_direction(self.direction),
            rungs,
            ratios,
            self.max_cells,
            self.selector,
            self.cost_model_id,
            self.forced_stop,
            self.max_ambiguous_bars,
            self.max_gap_fills,
        )
        .map_err(|why| format!("execution exit-grid policy refused: {why:?}"))?;
        if policy.digest() != self.stored_policy_digest {
            return Err("decoded execution policy differs from stored policy digest".to_owned());
        }
        let horizon = Horizon::bars(self.horizon_bars)
            .ok_or_else(|| "execution parameter horizon is zero".to_owned())?;
        let parameters = ExecutionParametersV1 {
            parameter_id: self.parameter_id,
            population_id: self.population_id,
            population_v4_digest: self.population_v4_digest,
            direction: self.direction,
            instrument_family: self.instrument_family,
            rung_seconds: self.rung_seconds,
            horizon,
            run_params: self.run_params,
            evaluation_fingerprint: self.evaluation_fingerprint,
            policy,
            expected_resolved_digest: self.expected_resolved_digest,
            training_digest: self.training_digest,
            training_bars: self.training_bars,
            training_first_ts_micros: self.training_first_ts_micros,
            training_last_ts_micros: self.training_last_ts_micros,
        };
        parameters.validate()?;
        Ok(parameters)
    }

    /// Exact content-derived parameter identity carried by this scalar record.
    #[must_use]
    pub(crate) const fn parameter_id(&self) -> [u8; 32] {
        self.parameter_id
    }

    /// Population whose execution policy this scalar record extends.
    #[must_use]
    pub(crate) const fn population_id(&self) -> [u8; 32] {
        self.population_id
    }

    /// Canonical long/short side of this scalar record.
    #[must_use]
    pub(crate) const fn direction(&self) -> TradeDirectionV1 {
        self.direction
    }

    /// Runtime-sized percentile atom count required to reconstruct this side.
    pub(crate) fn percentile_count(&self) -> Result<u64, ExecutionCapabilityRefusal> {
        u64::from(self.stop_count)
            .checked_add(u64::from(self.target_count))
            .and_then(|value| value.checked_add(u64::from(self.trail_count)))
            .ok_or_else(|| "execution percentile scalar count overflow".to_owned())
    }
}

fn count_u32(value: usize, name: &str) -> Result<u32, ExecutionCapabilityRefusal> {
    u32::try_from(value).map_err(|_| format!("{name} count does not fit u32"))
}

fn percentile_digest(
    parameters: &ExecutionParametersV1,
) -> Result<[u8; 32], ExecutionCapabilityRefusal> {
    percentile_digest_records(&PercentileRecordV1::from_parameters(parameters)?)
}

pub(crate) fn percentile_digest_records(
    records: &[PercentileRecordV1],
) -> Result<[u8; 32], ExecutionCapabilityRefusal> {
    let mut hasher = Hasher::new();
    hasher.update(PERCENTILE_ORDER_DOMAIN);
    hasher.update(
        &u64::try_from(records.len())
            .map_err(|_| "execution percentile record count does not fit u64".to_owned())?
            .to_le_bytes(),
    );
    for record in records {
        hasher.update(&record.payload()?);
    }
    Ok(hasher.finalize())
}

type PercentileAxes = (
    Vec<RationalPercentileV1>,
    Vec<RationalPercentileV1>,
    Vec<RationalPercentileV1>,
);

fn decode_percentile_axes(
    scalar: &ParameterScalarV1,
    records: &[PercentileRecordV1],
) -> Result<PercentileAxes, ExecutionCapabilityRefusal> {
    let expected = u64::from(scalar.stop_count)
        .checked_add(u64::from(scalar.target_count))
        .and_then(|value| value.checked_add(u64::from(scalar.trail_count)))
        .ok_or_else(|| "execution percentile expected count overflow".to_owned())?;
    if u64::try_from(records.len())
        .map_err(|_| "execution percentile count does not fit u64".to_owned())?
        != expected
    {
        return Err("execution percentile count differs from parameter record".to_owned());
    }
    let mut stop = Vec::new();
    let mut target = Vec::new();
    let mut trail = Vec::new();
    stop.try_reserve_exact(
        usize::try_from(scalar.stop_count)
            .map_err(|_| "stop percentile count does not fit usize".to_owned())?,
    )
    .map_err(|why| format!("stop percentile allocation refused: {why}"))?;
    target
        .try_reserve_exact(
            usize::try_from(scalar.target_count)
                .map_err(|_| "target percentile count does not fit usize".to_owned())?,
        )
        .map_err(|why| format!("target percentile allocation refused: {why}"))?;
    trail
        .try_reserve_exact(
            usize::try_from(scalar.trail_count)
                .map_err(|_| "trail percentile count does not fit usize".to_owned())?,
        )
        .map_err(|why| format!("trail percentile allocation refused: {why}"))?;
    let mut next = [0_u32; 3];
    let mut last_axis = 0_u8;
    for record in records {
        record.validate()?;
        if record.parameter_id != scalar.parameter_id
            || record.population_id != scalar.population_id
        {
            return Err("execution percentile record names a foreign parameter".to_owned());
        }
        let axis_byte = record.axis.byte();
        if axis_byte < last_axis {
            return Err("execution percentile axes are not in stop/target/trail order".to_owned());
        }
        last_axis = axis_byte;
        let slot = usize::from(axis_byte.saturating_sub(1));
        let expected_ordinal = next
            .get(slot)
            .copied()
            .ok_or_else(|| "execution percentile axis slot is invalid".to_owned())?;
        if record.ordinal != expected_ordinal {
            return Err(
                "execution percentile ordinal is missing, duplicated or reordered".to_owned(),
            );
        }
        if let Some(value) = next.get_mut(slot) {
            *value = value
                .checked_add(1)
                .ok_or_else(|| "execution percentile ordinal overflow".to_owned())?;
        }
        let percentile = RationalPercentileV1::new(record.numerator, record.denominator)
            .map_err(|why| format!("execution percentile refused: {why:?}"))?;
        match record.axis {
            PercentileAxisV1::Stop => stop.push(percentile),
            PercentileAxisV1::Target => target.push(percentile),
            PercentileAxisV1::Trail => trail.push(percentile),
        }
    }
    if next != [scalar.stop_count, scalar.target_count, scalar.trail_count] {
        return Err("execution percentile axis counts do not reconcile".to_owned());
    }
    Ok((stop, target, trail))
}

fn execution_parts(value: ExecutionResolutionV1) -> (u8, u32) {
    match value {
        ExecutionResolutionV1::OneMinuteOhlcv => (1, 60),
        ExecutionResolutionV1::UnsupportedSeconds(seconds) => (2, seconds),
    }
}

fn execution_from_parts(
    tag: u8,
    seconds: u32,
) -> Result<ExecutionResolutionV1, ExecutionCapabilityRefusal> {
    match (tag, seconds) {
        (1, 60) => Ok(ExecutionResolutionV1::OneMinuteOhlcv),
        (2, value) if value != 0 && value != 60 => {
            Ok(ExecutionResolutionV1::UnsupportedSeconds(value))
        }
        _ => Err(format!(
            "execution resolution tag/seconds {tag}/{seconds} is noncanonical"
        )),
    }
}

const fn range_resolution_byte(value: RangeResolutionV1) -> u8 {
    match value {
        RangeResolutionV1::PpmFloor => 1,
        RangeResolutionV1::PpmCeiling => 2,
    }
}

fn range_resolution_from_byte(byte: u8) -> Result<RangeResolutionV1, ExecutionCapabilityRefusal> {
    match byte {
        1 => Ok(RangeResolutionV1::PpmFloor),
        2 => Ok(RangeResolutionV1::PpmCeiling),
        _ => Err(format!("range-resolution tag {byte} is unknown")),
    }
}

const fn selector_byte(value: ExitGridSelectorV1) -> u8 {
    match value {
        ExitGridSelectorV1::PessimisticTotal => 1,
        ExitGridSelectorV1::EdgeThenPessimistic => 2,
        ExitGridSelectorV1::GuaranteedFloor => 3,
    }
}

fn selector_from_byte(byte: u8) -> Result<ExitGridSelectorV1, ExecutionCapabilityRefusal> {
    match byte {
        1 => Ok(ExitGridSelectorV1::PessimisticTotal),
        2 => Ok(ExitGridSelectorV1::EdgeThenPessimistic),
        3 => Ok(ExitGridSelectorV1::GuaranteedFloor),
        _ => Err(format!("exit-grid selector tag {byte} is unknown")),
    }
}

const fn forced_stop_parts(value: ForcedStopV1) -> (u8, i64) {
    match value {
        ForcedStopV1::Disabled => (0, 0),
        ForcedStopV1::IncludeExactObserved(ppm) => (1, ppm),
        ForcedStopV1::RequireExactObserved(ppm) => (2, ppm),
    }
}

fn forced_stop_from_parts(tag: u8, ppm: i64) -> Result<ForcedStopV1, ExecutionCapabilityRefusal> {
    match (tag, ppm) {
        (0, 0) => Ok(ForcedStopV1::Disabled),
        (1, value) if value > 0 => Ok(ForcedStopV1::IncludeExactObserved(value)),
        (2, value) if value > 0 => Ok(ForcedStopV1::RequireExactObserved(value)),
        _ => Err(format!("forced-stop tag/value {tag}/{ppm} is noncanonical")),
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a test that cannot panic cannot fail"
)]
mod tests {
    use super::*;
    use crate::population::{
        AdmissionStatusV1, AdmissionV1, ClosureV1, CompletionReconciliationV2, ExitCellsPerMaskV2,
        ExitCoordinateV1, LongShortExitGridIdentitiesV2, PopulationCommit, PopulationIdentitiesV2,
        RequestedSpanIdentityV1, SideExitGridIdentityV2, TopMetricsV1,
    };
    use crate::stored::{CompleteCalendarReceiptV2, calendar_receipt_v2};
    use pull::session::Day;

    fn digest(seed: u8) -> [u8; 32] {
        [seed; 32]
    }

    fn percentiles(count: u32) -> Vec<RationalPercentileV1> {
        (1..=count)
            .map(|numerator| {
                RationalPercentileV1::new(numerator, 100)
                    .expect("test percentiles are positive, reduced and below one")
            })
            .collect()
    }

    fn parameters(direction: TradeDirectionV1, seed: u8) -> ExecutionParametersV1 {
        let rungs = RungPlanV1::new(percentiles(31), percentiles(31), percentiles(31), 64)
            .expect("runtime-sized test rung plan");
        let policy = ExitGridPolicyV1::new(
            ExecutionResolutionV1::OneMinuteOhlcv,
            RangeResolutionV1::PpmFloor,
            side_of_direction(direction),
            rungs,
            RatioLimitsV1::new(100, 500, 10_000).expect("test ratio limits"),
            1_000_000,
            ExitGridSelectorV1::GuaranteedFloor,
            printed_ohlcv_cost_model_id_v1(),
            ForcedStopV1::Disabled,
            0,
            0,
        )
        .expect("test execution policy");
        let mut value = ExecutionParametersV1 {
            parameter_id: [0; 32],
            population_id: digest(1),
            population_v4_digest: digest(2),
            direction,
            instrument_family: InstrumentFamilyV1::Nifty,
            rung_seconds: 300,
            horizon: Horizon::bars(15).expect("non-zero horizon"),
            run_params: Params {
                min_hits: 20,
                ceiling: 1_000_000,
                pair_budget: 2_000_000,
                policy: 7,
            },
            evaluation_fingerprint: [seed; EVALUATION_SPEC_FINGERPRINT_V1_LEN],
            policy,
            expected_resolved_digest: digest(seed.wrapping_add(1)),
            training_digest: digest(seed.wrapping_add(2)),
            training_bars: 10_000,
            training_first_ts_micros: 1_000_000,
            training_last_ts_micros: 2_000_000,
        };
        value.parameter_id = value.derived_id().expect("test parameter identity");
        value.validate().expect("valid test parameters");
        value
    }

    fn root(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "brutex-execution-capability-{tag}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ))
    }

    fn population_identities(seed: u8) -> PopulationIdentitiesV2 {
        PopulationIdentitiesV2 {
            run_identity: digest(seed),
            data_digest: digest(seed.wrapping_add(1)),
            feed_digest: digest(seed.wrapping_add(2)),
            source_commit_digest: digest(seed.wrapping_add(3)),
            vocabulary_digest: digest(seed.wrapping_add(4)),
            evaluation_policy_digest: digest(seed.wrapping_add(5)),
            exit_grids: LongShortExitGridIdentitiesV2 {
                long: SideExitGridIdentityV2 {
                    policy_digest: digest(seed.wrapping_add(6)),
                    resolved_digest: digest(seed.wrapping_add(7)),
                },
                short: SideExitGridIdentityV2 {
                    policy_digest: digest(seed.wrapping_add(8)),
                    resolved_digest: digest(seed.wrapping_add(9)),
                },
            },
            admission_policy_digest: digest(seed.wrapping_add(10)),
            ranking_policy_digest: digest(seed.wrapping_add(11)),
            calendar_policy_digest: digest(seed.wrapping_add(12)),
            daily_reference_policy_digest: digest(seed.wrapping_add(13)),
        }
    }

    const fn admitted() -> AdmissionV1 {
        AdmissionV1 {
            status: AdmissionStatusV1::Admitted,
            reasons: 0,
            failed: 0,
            unmeasured: 0,
            refused: 0,
        }
    }

    fn population_row(
        population_id: [u8; 32],
        sequence: u64,
        direction: TradeDirectionV1,
    ) -> PopulationRowV1 {
        PopulationRowV1 {
            population_id,
            sequence,
            strategy_digest: digest(
                u8::try_from(sequence)
                    .unwrap_or(0)
                    .wrapping_add(80)
                    .wrapping_add(direction_byte(direction)),
            ),
            mask_words: [7, 0, 0, 0, 0, 0],
            direction,
            instrument_family: InstrumentFamilyV1::Nifty,
            closure: ClosureV1::Closed,
            rung_seconds: 300,
            support_hits: 41,
            exit: ExitCoordinateV1 {
                stop: Some(u32::try_from(sequence).unwrap_or(0)),
                target: Some(2),
                tsl: None,
                ttp: Some((1, 1)),
            },
            metrics: TopMetricsV1 {
                drawdown: 1_000 + sequence,
                worst_loss: 500,
                losing_rate_ppm: 250_000,
                losing_trades: 1,
                loss_ratio_ppm: Some(100_000),
                pessimistic_profit: 8_000,
                winning_trades: 3,
                win_rate_ppm: 750_000,
                reward_to_risk_ppm: Some(2_000_000),
                average_win: 3_000,
                average_loss: 1_000,
                assurance_ppm: 700_000,
            },
            admission: admitted(),
        }
    }

    fn population_rows(population_id: [u8; 32]) -> [PopulationRowV1; 2] {
        [
            population_row(population_id, 0, TradeDirectionV1::Long),
            population_row(population_id, 1, TradeDirectionV1::Short),
        ]
    }

    fn population_reconciliation() -> CompletionReconciliationV2 {
        CompletionReconciliationV2 {
            sweep_trials: 3,
            frequent_itemsets: 1,
            infrequent_itemsets: 2,
            closed_itemsets: 1,
            redundant_itemsets: 0,
            unknown_closure_itemsets: 0,
            exit_cells_per_mask: ExitCellsPerMaskV2::new(1, 1)
                .expect("one cell on each side is valid"),
            extinction_depth: 2,
            extinction_complete: true,
            closure_complete: true,
        }
    }

    fn requested_span() -> RequestedSpanIdentityV1 {
        RequestedSpanIdentityV1::new(2026, 7, 2026, 7).expect("measured full-month span")
    }

    fn complete_calendar(
        requested_span: RequestedSpanIdentityV1,
        rung_seconds: u32,
    ) -> CompleteCalendarReceiptV2 {
        const MICROS_PER_MINUTE: i64 = 60_000_000;
        const MICROS_PER_DAY: i64 = 86_400_000_000;
        let first_day = i64::from(
            Day::new(requested_span.from_year(), requested_span.from_month(), 1)
                .expect("valid first day")
                .days_from_epoch(),
        );
        let last_day = i64::from(
            Day::new(requested_span.to_year(), requested_span.to_month(), 1)
                .expect("valid final month")
                .end_of_month()
                .days_from_epoch(),
        );
        let width = i64::from(rung_seconds / 60);
        let last_bucket = (929_i64 - 555) / width;
        let mut timestamps = Vec::new();
        for day in first_day..=last_day {
            match pull::calendar::kind_of(day) {
                pull::calendar::DayKind::Open(session) => {
                    for bucket in 0..=last_bucket {
                        let start = 555_i64 + bucket * width;
                        let end = start + width - 1;
                        let intersects = session
                            .windows
                            .iter()
                            .take(usize::from(session.count))
                            .any(|window| {
                                start <= i64::from(window.to) && end >= i64::from(window.from)
                            });
                        if intersects {
                            timestamps.push(
                                day * MICROS_PER_DAY + start * MICROS_PER_MINUTE
                                    - indicators::IST_OFFSET_MICROS,
                            );
                        }
                    }
                }
                pull::calendar::DayKind::Closed => {}
                other => panic!("fixture month is not fully measured: {other:?}"),
            }
        }
        calendar_receipt_v2(&timestamps, rung_seconds, first_day, last_day)
            .expect("calendar receipt")
            .require_complete()
            .expect("complete measured month")
    }

    fn population_receipt(
        population_id: [u8; 32],
        rows: &[PopulationRowV1],
    ) -> CompletionReceiptV4 {
        let span = requested_span();
        CompletionReceiptV4::for_rows(
            population_id,
            InstrumentFamilyV1::Nifty,
            300,
            rows,
            span,
            population_reconciliation(),
            population_identities(101),
            complete_calendar(span, 300),
            complete_calendar(span, 60),
        )
        .expect("canonical Population V4 receipt")
    }

    fn parameters_for_population(
        receipt: &CompletionReceiptV4,
        direction: TradeDirectionV1,
        seed: u8,
    ) -> ExecutionParametersV1 {
        let mut value = parameters(direction, seed);
        value.population_id = receipt.population_id();
        value.population_v4_digest = receipt.content_digest().expect("V4 content digest");
        value.instrument_family = receipt.v3().v2().instrument_family;
        value.rung_seconds = receipt.v3().v2().rung_seconds;
        value.parameter_id = value.derived_id().expect("bound parameter identity");
        value.validate().expect("valid population-bound parameters");
        value
    }

    fn capability_for_row(
        parameters: &ExecutionParametersV1,
        row: &PopulationRowV1,
        seed: u8,
    ) -> ExecutionStrategyCapabilityV1 {
        let mut value = ExecutionStrategyCapabilityV1 {
            capability_id: [0; 32],
            parameter_id: parameters.parameter_id,
            population_id: row.population_id,
            population_v4_digest: parameters.population_v4_digest,
            row_payload_digest: row.payload_digest().expect("canonical population row"),
            strategy_digest: row.strategy_digest,
            training_run_id: digest(seed),
            selected_exit_digest: digest(seed.wrapping_add(1)),
            row_sequence: row.sequence,
            rung_seconds: row.rung_seconds,
            direction: row.direction,
            instrument_family: row.instrument_family,
        };
        value.capability_id = value.derived_id();
        value.validate().expect("valid row-bound capability");
        value
    }

    fn population_v4_fixture(
        tag: &str,
    ) -> (
        PathBuf,
        [PopulationRowV1; 2],
        CompletionReceiptV4,
        PreparedExecutionCapabilitiesV1,
    ) {
        let root = root(tag);
        let _ignored = fs::remove_dir_all(&root);
        let population_id = digest(99);
        let rows = population_rows(population_id);
        let receipt = population_receipt(population_id, &rows);
        let mut population = PopulationLedger::open(&root).expect("population ledger");
        assert_eq!(
            population
                .append_complete_v4(&rows, &receipt)
                .expect("Population V4 commit"),
            PopulationCommit::Written
        );
        let long = parameters_for_population(&receipt, TradeDirectionV1::Long, 11);
        let short = parameters_for_population(&receipt, TradeDirectionV1::Short, 21);
        let capabilities = vec![
            capability_for_row(&long, rows.first().expect("long population row"), 31),
            capability_for_row(&short, rows.get(1).expect("short population row"), 41),
        ];
        let prepared = PreparedExecutionCapabilitiesV1::from_population_v4(
            &mut population,
            population_id,
            long,
            short,
            capabilities,
        )
        .expect("real Population V4 prepares execution capabilities");
        (root, rows, receipt, prepared)
    }

    fn capability(
        parameters: &ExecutionParametersV1,
        sequence: u64,
        seed: u8,
    ) -> ExecutionStrategyCapabilityV1 {
        let mut value = ExecutionStrategyCapabilityV1 {
            capability_id: [0; 32],
            parameter_id: parameters.parameter_id,
            population_id: parameters.population_id,
            population_v4_digest: parameters.population_v4_digest,
            row_payload_digest: digest(seed),
            strategy_digest: digest(seed.wrapping_add(1)),
            training_run_id: digest(seed.wrapping_add(2)),
            selected_exit_digest: digest(seed.wrapping_add(3)),
            row_sequence: sequence,
            rung_seconds: parameters.rung_seconds,
            direction: parameters.direction,
            instrument_family: parameters.instrument_family,
        };
        value.capability_id = value.derived_id();
        value.validate().expect("valid test row capability");
        value
    }

    fn prepared() -> PreparedExecutionCapabilitiesV1 {
        let parameters = [
            parameters(TradeDirectionV1::Long, 11),
            parameters(TradeDirectionV1::Short, 21),
        ];
        let scalars = [
            ParameterScalarV1::from_parameters(&parameters[0]).expect("long scalar"),
            ParameterScalarV1::from_parameters(&parameters[1]).expect("short scalar"),
        ];
        let mut percentiles =
            PercentileRecordV1::from_parameters(&parameters[0]).expect("long atoms");
        percentiles
            .extend(PercentileRecordV1::from_parameters(&parameters[1]).expect("short atoms"));
        let capabilities = vec![
            capability(&parameters[0], 0, 31),
            capability(&parameters[1], 1, 41),
        ];
        PreparedExecutionCapabilitiesV1 {
            population_id: parameters[0].population_id,
            population_v4_digest: parameters[0].population_v4_digest,
            ordered_parameter_digest: parameter_digest_records(&scalars)
                .expect("parameter order digest"),
            ordered_percentile_digest: percentile_digest_records(&percentiles)
                .expect("percentile order digest"),
            ordered_capability_digest: capability_digest_records(&capabilities)
                .expect("capability order digest"),
            parameters,
            scalars,
            percentiles,
            capabilities,
            row_count: 2,
            long_count: 1,
            short_count: 1,
        }
    }

    fn reseal<const PAYLOAD: usize, const STRIDE: usize>(raw: &mut [u8; STRIDE]) {
        let payload: [u8; PAYLOAD] = raw[..PAYLOAD]
            .try_into()
            .expect("test payload has fixed width");
        raw[PAYLOAD..].copy_from_slice(&brutex_core::blake3::hash(&payload));
    }

    #[test]
    fn runtime_sized_parameter_codec_reconstructs_thirty_one_levels_per_axis() {
        let parameters = parameters(TradeDirectionV1::Long, 9);
        let scalar = ParameterScalarV1::from_parameters(&parameters).expect("encode scalar");
        assert_eq!(scalar.stop_count, 31);
        assert_eq!(scalar.target_count, 31);
        assert_eq!(scalar.trail_count, 31);
        let scalar = ParameterScalarV1::from_bytes(&scalar.to_bytes().expect("scalar bytes"))
            .expect("decode scalar");
        let records = PercentileRecordV1::from_parameters(&parameters)
            .expect("runtime percentile records")
            .into_iter()
            .map(|record| {
                PercentileRecordV1::from_bytes(&record.to_bytes().expect("atom bytes"))
                    .expect("decode atom")
            })
            .collect::<Vec<_>>();
        let rebuilt = scalar.reconstruct(&records).expect("reconstruct policy");
        assert_eq!(rebuilt, parameters);
        assert_eq!(rebuilt.policy().rungs().stop().len(), 31);
    }

    #[test]
    fn codecs_refuse_resealed_unknown_tags_and_nonzero_reserves() {
        let parameters = parameters(TradeDirectionV1::Long, 13);
        let scalar = ParameterScalarV1::from_parameters(&parameters).expect("scalar");
        let mut parameter_bytes = scalar.to_bytes().expect("parameter bytes");
        // 355, NOT 347. `direction` sits immediately after the evaluation-spec
        // fingerprint, and that widened 155 -> 163 when the charter gained its
        // ninth non-regular day -- so this byte and every field below it moved
        // by eight. Derived from the width rather than retyped, so the next
        // calendar entry moves it again without anyone noticing it should.
        let direction_at = 192 + EVALUATION_SPEC_FINGERPRINT_V1_LEN;
        parameter_bytes[direction_at] = 99;
        reseal::<PARAMETER_PAYLOAD_BYTES, EXECUTION_PARAMETER_STRIDE>(&mut parameter_bytes);
        assert!(
            ParameterScalarV1::from_bytes(&parameter_bytes)
                .expect_err("unknown direction must refuse")
                .contains("direction")
        );

        let record = PercentileRecordV1::from_parameters(&parameters)
            .expect("atoms")
            .remove(0);
        let mut percentile_bytes = record.to_bytes().expect("percentile bytes");
        percentile_bytes[64] = 99;
        reseal::<PERCENTILE_PAYLOAD_BYTES, EXECUTION_PERCENTILE_STRIDE>(&mut percentile_bytes);
        assert!(
            PercentileRecordV1::from_bytes(&percentile_bytes)
                .expect_err("unknown axis must refuse")
                .contains("axis")
        );

        let capability = capability(&parameters, 0, 51);
        let mut capability_bytes = capability.to_bytes().expect("capability bytes");
        capability_bytes[270] = 1;
        reseal::<CAPABILITY_PAYLOAD_BYTES, EXECUTION_CAPABILITY_STRIDE>(&mut capability_bytes);
        assert!(
            ExecutionStrategyCapabilityV1::from_bytes(&capability_bytes)
                .expect_err("non-zero reserve must refuse")
                .contains("reserve")
        );

        let receipt = ExecutionCapabilityCompletionV1::for_prepared(&prepared(), 3, 9, 17)
            .expect("completion");
        let mut receipt_bytes = receipt.to_bytes().expect("completion bytes");
        receipt_bytes[360] = 1;
        reseal::<COMPLETION_PAYLOAD_BYTES, EXECUTION_COMPLETION_STRIDE>(&mut receipt_bytes);
        assert!(
            ExecutionCapabilityCompletionV1::from_bytes(&receipt_bytes)
                .expect_err("non-zero completion reserve must refuse")
                .contains("reserve")
        );
    }

    #[test]
    fn recomputed_outer_seal_cannot_forge_selected_exit_identity() {
        let parameters = parameters(TradeDirectionV1::Short, 17);
        let capability = capability(&parameters, 0, 61);
        let mut raw = capability.to_bytes().expect("capability bytes");
        raw[224] ^= 1;
        reseal::<CAPABILITY_PAYLOAD_BYTES, EXECUTION_CAPABILITY_STRIDE>(&mut raw);
        assert!(
            ExecutionStrategyCapabilityV1::from_bytes(&raw)
                .expect_err("changed selected-exit digest must refuse")
                .contains("identity")
        );
    }

    #[test]
    fn torn_seal_and_noncanonical_percentile_ordinal_are_refused() {
        let parameters = parameters(TradeDirectionV1::Long, 19);
        let record = PercentileRecordV1::from_parameters(&parameters)
            .expect("atoms")
            .remove(0);
        let mut raw = record.to_bytes().expect("percentile bytes");
        raw[EXECUTION_PERCENTILE_STRIDE - 1] ^= 1;
        assert!(
            PercentileRecordV1::from_bytes(&raw)
                .expect_err("torn seal must refuse")
                .contains("seal")
        );
        let mut raw = record.to_bytes().expect("percentile bytes");
        raw[68..72].copy_from_slice(&3_u32.to_le_bytes());
        reseal::<PERCENTILE_PAYLOAD_BYTES, EXECUTION_PERCENTILE_STRIDE>(&mut raw);
        let decoded = PercentileRecordV1::from_bytes(&raw).expect("record codec permits ordinal");
        let scalar = ParameterScalarV1::from_parameters(&parameters).expect("scalar");
        assert!(
            scalar
                .reconstruct(&[decoded])
                .expect_err("missing/reordered atom block must refuse")
                .contains("digest")
        );
    }

    #[test]
    fn crash_orphan_offsets_do_not_rekey_semantic_completion() {
        let prepared = prepared();
        let first = ExecutionCapabilityCompletionV1::for_prepared(&prepared, 0, 0, 0)
            .expect("first completion");
        let after_orphans = ExecutionCapabilityCompletionV1::for_prepared(&prepared, 8, 900, 45)
            .expect("completion after orphan tails");
        assert_eq!(first.authority_id(), after_orphans.authority_id());
        assert_ne!(
            first.to_bytes().expect("first bytes"),
            after_orphans.to_bytes().expect("offset bytes")
        );
        assert_eq!(exact_execution_law_digest_v1(), first.execution_law_digest);
        assert_eq!(FORCED_EXIT_MINUTE, i64::from(FORCED_EXIT_IST_MINUTE));
    }

    #[test]
    fn real_population_v4_prepares_every_row_and_refuses_reordered_capabilities() {
        let (root, rows, receipt, prepared) = population_v4_fixture("real-v4-binding");
        assert_eq!(prepared.population_id, receipt.population_id());
        assert_eq!(
            prepared.population_v4_digest,
            receipt.content_digest().expect("V4 content digest")
        );
        assert_eq!(prepared.row_count, 2);
        assert_eq!((prepared.long_count, prepared.short_count), (1, 1));
        assert_ne!(
            prepared.authority_id().expect("semantic authority"),
            [0; 32]
        );
        assert_eq!(prepared.capabilities[0].row_sequence, rows[0].sequence);
        assert_eq!(prepared.capabilities[1].row_sequence, rows[1].sequence);

        let mut population = PopulationLedger::open_read(&root).expect("reopen Population V4");
        let [long, short] = prepared.parameters.clone();
        let mut reordered = prepared.capabilities.clone();
        reordered.swap(0, 1);
        let refusal = PreparedExecutionCapabilitiesV1::from_population_v4(
            &mut population,
            receipt.population_id(),
            long,
            short,
            reordered,
        )
        .expect_err("reordered capabilities cannot inherit Population V4 authority");
        assert!(
            refusal.contains("differs") || refusal.contains("reordered"),
            "unexpected refusal: {refusal}"
        );
        drop(population);
        fs::remove_dir_all(root).expect("remove fixture root");
    }

    #[test]
    fn execution_ledger_reopens_reuses_and_retains_valid_orphan_evidence() {
        let (root, rows, receipt, prepared) = population_v4_fixture("append-reopen-orphan");
        let authority_id = prepared.authority_id().expect("semantic authority");
        let mut ledger = ExecutionCapabilityLedger::open(&root).expect("execution ledger");
        assert_eq!(
            ledger
                .append_complete(&prepared)
                .expect("append complete execution authority"),
            ExecutionCapabilityCommit::Written
        );
        assert_eq!(
            ledger
                .append_complete(&prepared)
                .expect("reuse exact execution authority"),
            ExecutionCapabilityCommit::Reused
        );
        drop(ledger);

        let parameter_path = ExecutionCapabilityLedger::parameter_path(&root);
        let orphan = prepared.scalars[0].to_bytes().expect("valid orphan bytes");
        let mut parameter_file = OpenOptions::new()
            .append(true)
            .open(&parameter_path)
            .expect("open parameter tail");
        parameter_file
            .write_all(&orphan)
            .expect("append valid crash orphan");
        parameter_file.sync_all().expect("sync valid crash orphan");
        drop(parameter_file);

        let mut reopened =
            ExecutionCapabilityLedger::open_read(&root).expect("reopen past valid orphan");
        let completion = reopened
            .completion(&receipt.population_id())
            .expect("completion lookup")
            .expect("committed completion");
        assert_eq!(completion.authority_id(), authority_id);
        assert_eq!(completion.row_count, 2);
        assert_eq!((completion.long_count, completion.short_count), (1, 1));
        for (direction, expected) in [
            (TradeDirectionV1::Long, prepared.parameters[0].parameter_id),
            (TradeDirectionV1::Short, prepared.parameters[1].parameter_id),
        ] {
            assert_eq!(
                reopened
                    .parameters(&receipt.population_id(), direction)
                    .expect("parameter lookup")
                    .expect("committed side parameters")
                    .parameter_id,
                expected
            );
        }
        for row in rows {
            let capability = reopened
                .capability(&receipt.population_id(), row.sequence)
                .expect("capability lookup")
                .expect("committed row capability");
            assert_eq!(
                capability.row_payload_digest,
                row.payload_digest().expect("row digest")
            );
            assert_eq!(capability.strategy_digest, row.strategy_digest);
        }
        drop(reopened);
        fs::remove_dir_all(root).expect("remove fixture root");
    }

    #[test]
    fn execution_ledger_refuses_corruption_and_a_stale_same_length_handle() {
        let (corrupt_root, _rows, _receipt, prepared) =
            population_v4_fixture("completion-corruption");
        let mut ledger = ExecutionCapabilityLedger::open(&corrupt_root).expect("execution ledger");
        assert_eq!(
            ledger
                .append_complete(&prepared)
                .expect("append execution authority"),
            ExecutionCapabilityCommit::Written
        );
        drop(ledger);
        let completion_path = ExecutionCapabilityLedger::completion_path(&corrupt_root);
        let mut completion_bytes = fs::read(&completion_path).expect("read completion file");
        let last = completion_bytes
            .len()
            .checked_sub(1)
            .expect("header plus one completion");
        completion_bytes[last] ^= 1;
        let mut completion_file = OpenOptions::new()
            .write(true)
            .open(&completion_path)
            .expect("open completion for corruption");
        completion_file
            .write_all(&completion_bytes)
            .expect("write same-length corruption");
        completion_file
            .sync_all()
            .expect("sync same-length corruption");
        drop(completion_file);
        assert!(
            ExecutionCapabilityLedger::open_read(&corrupt_root)
                .expect_err("corrupt completion must fail closed")
                .contains("seal")
        );
        fs::remove_dir_all(corrupt_root).expect("remove corrupt fixture root");

        let (stale_root, _rows, receipt, prepared) = population_v4_fixture("stale-handle");
        let mut stale = ExecutionCapabilityLedger::open(&stale_root).expect("execution ledger");
        assert_eq!(
            stale
                .append_complete(&prepared)
                .expect("append execution authority"),
            ExecutionCapabilityCommit::Written
        );
        let parameter_path = ExecutionCapabilityLedger::parameter_path(&stale_root);
        let parameter_bytes = fs::read(&parameter_path).expect("read parameter file");
        let changed_at = HEADER_BYTES_USIZE + 19;
        let changed = parameter_bytes[changed_at] ^ 1;
        let mut external = OpenOptions::new()
            .write(true)
            .open(&parameter_path)
            .expect("open parameter file externally");
        external
            .seek(SeekFrom::Start(
                u64::try_from(changed_at).expect("test offset fits u64"),
            ))
            .expect("seek to same-length mutation");
        external
            .write_all(&[changed])
            .expect("write same-length mutation");
        external.sync_all().expect("sync same-length mutation");
        drop(external);
        let refusal = stale
            .completion(&receipt.population_id())
            .expect_err("stale handle must refuse lookup");
        assert!(
            refusal.contains("changed after open"),
            "unexpected refusal: {refusal}"
        );
        drop(stale);
        fs::remove_dir_all(stale_root).expect("remove stale fixture root");
    }
}

//! Strict stored admission retained by every institutional successor.
use std::sync::Arc;

use brutex_core::blake3::Hasher;
use runner::identity::{DailyReferenceBinding, ReferenceIntegrity};

use super::{AdmittedRootV1, BoundedStoredContextV1, StoredContextLoadSpecV1};
use crate::audited_range_command::StrictConfig;
use crate::audited_stored::{RangeGuard, RangeInputs, RangeRequest};
use crate::sweep_evidence::{Attempt, Completion, Operation};

/// Opaque source locks and complete physical-policy binding; no detached bytes
/// can construct this authority outside the shared admission transaction.
pub(crate) struct Inputs {
    guard: RangeGuard,
    config: StrictConfig,
    identity: [u8; 32],
}

/// Includes extinct family sources: absence of selected rows never drops the
/// input evidence whose computation established that absence.
#[derive(Clone, Default)]
pub(crate) struct Guards(Vec<Arc<Inputs>>);

impl Guards {
    pub(crate) fn from_sources(sources: impl IntoIterator<Item = Option<Arc<Inputs>>>) -> Self {
        Self(sources.into_iter().flatten().collect())
    }

    pub(crate) fn require_current(&self) -> Result<(), String> {
        for source in &self.0 {
            source.require_current()?;
        }
        Ok(())
    }
}

impl Inputs {
    pub(crate) fn require_current(&self) -> Result<(), String> {
        self.guard.require_current()
    }

    pub(crate) fn config(&self) -> &StrictConfig {
        &self.config
    }

    pub(crate) const fn integrity(&self) -> ReferenceIntegrity {
        ReferenceIntegrity::ChecksumReceiptV1(self.identity)
    }
}

/// The support census uses the same admitted input policy as the real kernel.
/// Its guards remain held by the rung caller until selection is published.
pub(crate) fn size_sweeper(
    root: &std::path::Path,
    vendor: brutex_core::vendor::Vendor,
    request: &crate::ledger_all::LedgerAllRequest<'_>,
    rung: &str,
    bounds: super::StoredCandidatePreAdmissionBoundsV1,
    config: &StrictConfig,
) -> Result<(runner::Sweeper, Arc<Inputs>), String> {
    let root = AdmittedRootV1::admit(root)?;
    let context = load(
        StoredContextLoadSpecV1 {
            vendor,
            underlying: "NIFTY",
            rung_name: rung,
            from: request.from,
            to: request.to,
            signal_bound: bounds.signal_records,
            minute_bound: bounds.minute_records,
            daily_bound: bounds.daily_records,
        },
        &root,
        config,
    )?;
    let count = context.signal.bars.len();
    let inputs = context
        .strict
        .ok_or("strict institutional sizing lost its input authority")?;
    inputs.require_current()?;
    let min_hits = crate::min_hits_for(count, request.support_ppm);
    let sweeper = runner::Sweeper::new(crate::ladder_for(min_hits)?);
    crate::note(&crate::ledger_all::rung_sized_event(
        "ledger-v6",
        rung,
        count,
        min_hits,
        request.support_ppm,
    ));
    Ok((sweeper, inputs))
}

pub(super) fn load(
    spec: StoredContextLoadSpecV1<'_>,
    root: &AdmittedRootV1,
    config: &StrictConfig,
) -> Result<BoundedStoredContextV1, String> {
    root.require_same("before strict institutional source admission")?;
    let limits = [
        spec.signal_bound.max_records(),
        spec.minute_bound.max_records(),
        spec.daily_bound.max_records(),
    ];
    let (data, guard) = RangeInputs::load_bounded(
        RangeRequest {
            store_root: root.path(),
            vendor: spec.vendor,
            underlying: spec.underlying,
            rung: spec.rung_name,
            from: spec.from,
            to: spec.to,
            receipt_root: config.receipt_root(),
            max_bytes: config.max_bytes(),
            max_records: config.max_records(),
        },
        limits,
    )?
    .into_parts();
    let mut policy = Hasher::new();
    policy.update(b"brutex-institutional-checksum-inputs-v1\0");
    for value in [config.max_bytes(), config.max_records()]
        .into_iter()
        .chain(limits)
    {
        policy.update(&value.to_le_bytes());
    }
    let identity = guard.bind_digest(policy.finalize());
    let requested_span =
        super::RequestedSpanIdentityV1::new(spec.from.0, spec.from.1, spec.to.0, spec.to.1)
            .map_err(|why| format!("Step 3 strict requested span refused: {why}"))?;
    let (first_day, last_day) = super::requested_span_days(requested_span)?;
    super::require_complete_signal_span(&data.signal)?;
    let rung_seconds =
        super::rung_seconds(crate::stored::rung_length_micros(data.signal.timeframe)?)?;
    let signal_calendar = crate::stored::calendar_receipt_v2_for_bars(
        &data.signal.bars,
        rung_seconds,
        first_day,
        last_day,
    )
    .and_then(super::CalendarReceiptV2::require_complete)
    .map_err(|why| format!("Step 3 strict signal calendar refused: {why}"))?;
    // Execution is selected from the same retained complete minute context by
    // the common institutional kernel; the redundant requested-only copy drops.
    let context = BoundedStoredContextV1 {
        requested_span,
        first_day,
        last_day,
        rung_seconds,
        signal_calendar,
        signal: data.signal,
        daily: data.daily,
        minute: data.exact_minute,
        strict: Some(Arc::new(Inputs {
            guard,
            config: config.clone(),
            identity,
        })),
    };
    context.require_current()?;
    root.require_same("after strict institutional source admission")?;
    Ok(context)
}

impl BoundedStoredContextV1 {
    pub(crate) fn require_current(&self) -> Result<(), String> {
        self.strict
            .as_ref()
            .map_or(Ok(()), |inputs| inputs.require_current())
    }

    pub(crate) fn integrity(&self) -> ReferenceIntegrity {
        self.strict
            .as_ref()
            .map_or(ReferenceIntegrity::UnverifiedNoReceipt, |inputs| {
                inputs.integrity()
            })
    }

    pub(crate) fn daily_reference(&self) -> DailyReferenceBinding<'_> {
        DailyReferenceBinding {
            daily_bars: &self.daily.bars,
            eligibility: &self.daily.eligibility,
            schema: crate::stored::DAILY_REFERENCE_SCHEMA,
            eligibility_policy: crate::stored::DAILY_ELIGIBILITY_POLICY,
            gap_overlay_policy: crate::stored::EXACT_MINUTE_GAP_POLICY,
            excluded_ist_days: &indicators::evaluator::CHARTER_NON_REGULAR_IST_DAYS,
            daily_integrity: self.integrity(),
            minute_integrity: self.integrity(),
            swept_series_calendar_policy: crate::stored::SWEPT_SERIES_CALENDAR_POLICY,
        }
    }
}

pub(super) fn begin(
    context: &BoundedStoredContextV1,
    request: &super::StoredCandidatePreAdmissionRequestV1<'_>,
    commit: &str,
) -> Result<Option<Attempt>, String> {
    if context.strict.is_none() {
        return Ok(None);
    }
    context.require_current()?;
    let mut evaluator = indicators::evaluator::Evaluator::new(
        request.widths,
        request.availability,
        request.thresholds,
    );
    let specification = indicators::column::Column::build(&[], &mut evaluator)
        .evaluation_spec_token()
        .ok_or("strict institutional evaluator specification absent before fold")?
        .fingerprint_v1();
    let mut binding = Hasher::new();
    binding.update(b"brutex-strict-institutional-attempt-v1\0");
    binding.update(
        &runner::identity::data_digest_with_daily_reference(
            &context.signal.bars,
            &context.minute.bars,
            context.daily_reference(),
        )
        .map_err(|why| format!("strict institutional data identity refused: {why:?}"))?,
    );
    binding.update(specification.as_bytes());
    binding.update(&request.long_exit_policy.digest());
    binding.update(&request.short_exit_policy.digest());
    binding.update(&request.horizon.as_bars().to_le_bytes());
    binding.update(&context.requested_span.canonical_bytes());
    let identity = runner::identity::identity(&runner::identity::Run {
        mask: vocab::ConditionMask::default(),
        direction: runner::identity::Direction::Undirected,
        instrument: &context.signal.key,
        timeframe: context.signal.timeframe,
        params: runner::identity::Params::of(request.sweeper.ladder()),
        data_digest: binding.finalize(),
        commit,
        feed: context.signal.vendor.as_str(),
    });
    crate::sweep_evidence::begin(request.root, identity.bytes(), Operation::Preparation).map(Some)
}

pub(super) fn finish(
    attempt: Option<Attempt>,
    result: Result<super::family_v6::StoredFamilyV6, String>,
) -> Result<super::family_v6::StoredFamilyV6, String> {
    let Some(attempt) = attempt else {
        return result;
    };
    let result = result.and_then(|committed| {
        committed.require_current()?;
        Ok(committed)
    });
    let completion = if result.is_ok() {
        Completion::Completed
    } else {
        Completion::Refused
    };
    if let Err(terminal) = attempt.finish(completion) {
        return Err(format!(
            "{}; strict institutional terminal persistence refused: {terminal}",
            result
                .err()
                .unwrap_or_else(|| "computation completed".to_owned())
        ));
    }
    result
}

//! Strict multi-month admission feeding the existing audit/pricing transaction.
use std::path::Path;

use crate::audited_stored::{RangeData, RangeGuard, RangeInputs, RangeRequest};
use runner::identity::{Direction, Params, Run, identity};

#[path = "strict_range_config.rs"]
mod config;
pub use config::{ConfigRefusal, StrictConfig};
#[path = "strict_range_knobs.rs"]
mod settings;
pub use settings::{KnobRefusal, request_value, validate_boolean_runtime, validate_runtime};

/// A stored range to execute using server-owned paths and explicit physical limits.
#[derive(Clone, Copy)]
pub struct Request<'a> {
    /// The existing market/result store; never read from an HTTP request.
    pub store_root: &'a Path,
    /// Canonical stored feed word.
    pub vendor: &'a str,
    /// Canonical eligible underlying.
    pub underlying: &'a str,
    /// One of the eight supported intraday signal rungs.
    pub rung: &'a str,
    /// Inclusive first civil month.
    pub from: (u16, u8),
    /// Inclusive last civil month.
    pub to: (u16, u8),
    /// Explicit positive support count, separate from institutional admission.
    pub min_hits: u64,
    /// Exact browser attempt, or absent for standalone execution.
    pub attempt: Option<u64>,
}

/// Execute the existing strict audit kernel under this binary's real build identity.
///
/// # Errors
/// Refuses an unstamped build, invalid input, missing/changed source authority,
/// resource halt or unsuccessful evidence/parent publication. A report containing
/// a late refusal is returned as an error rather than successful completion.
pub fn audit(request: Request<'_>, config: &StrictConfig) -> Result<String, String> {
    let commit = crate::commit_stamp()
        .ok_or("strict range audit requires a verified clean build identity")?;
    audit_identified(request, config, commit)
}

fn audit_identified(
    request: Request<'_>,
    config: &StrictConfig,
    commit: &str,
) -> Result<String, String> {
    if request.min_hits == 0 || request.attempt == Some(0) {
        return Err(
            "strict range audit requires positive support and a nonzero attempt when supplied"
                .to_owned(),
        );
    }
    let range = RangeRequest {
        store_root: request.store_root,
        vendor: crate::parse_vendor(request.vendor)?,
        underlying: request.underlying,
        rung: request.rung,
        from: request.from,
        to: request.to,
        receipt_root: config.receipt_root(),
        max_bytes: config.max_bytes(),
        max_records: config.max_records(),
    };
    checked_report(run(range, request.min_hits, commit, request.attempt)?)
}

fn checked_report(report: String) -> Result<String, String> {
    if crate::carries_refusal(&report) {
        Err(report)
    } else {
        Ok(report)
    }
}

#[cfg(test)]
pub(crate) fn api_for_test(
    request: RangeRequest<'_>,
    min_hits: u64,
    attempt: u64,
) -> Result<String, String> {
    audit_identified(
        Request {
            store_root: request.store_root,
            vendor: request.vendor.as_str(),
            underlying: request.underlying,
            rung: request.rung,
            from: request.from,
            to: request.to,
            min_hits,
            attempt: Some(attempt),
        },
        &StrictConfig::explicit(request.receipt_root, request.max_bytes, request.max_records),
        "generated-strict-range-test-fixture",
    )
}

pub(crate) fn command(arguments: &[&str]) -> Result<String, String> {
    let [
        vendor,
        underlying,
        rung,
        fy,
        fm,
        ty,
        tm,
        hits,
        receipts,
        bytes,
        records,
    ] = arguments
    else {
        return Err("strict range audit requires eleven arguments".to_owned());
    };
    let from = month(fy, fm)?;
    let to = month(ty, tm)?;
    let min_hits = crate::parse_min_hits(hits).map_err(str::to_owned)?;
    let max_bytes = bound(bytes, "MAX_BYTES")?;
    let max_records = bound(records, "MAX_RECORDS")?;
    let commit = crate::commit_stamp()
        .ok_or("strict range audit requires a verified clean build identity")?;
    let root = crate::store_root()?;
    audit_identified(
        Request {
            store_root: &root,
            vendor,
            underlying,
            rung,
            from,
            to,
            min_hits,
            attempt: None,
        },
        &StrictConfig::explicit(Path::new(receipts), max_bytes, max_records),
        commit,
    )
}

fn month(year: &str, month: &str) -> Result<(u16, u8), String> {
    let year = year.parse().map_err(|_| "YEAR must be a valid u16")?;
    let month = month.parse().map_err(|_| "MONTH must be 1..=12")?;
    store::path::YearMonth::new(year, month).map_err(|why| why.to_string())?;
    Ok((year, month))
}

fn bound(raw: &str, name: &str) -> Result<u64, String> {
    raw.parse::<u64>()
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| format!("{name} must be a positive u64"))
}

#[cfg(test)]
pub(crate) fn run_for_test(request: RangeRequest<'_>, min_hits: u64) -> Result<String, String> {
    run(
        request,
        min_hits,
        "generated-strict-range-test-fixture",
        None,
    )
}

/// A source identity and durable preparation precede the indicator fold. The
/// resulting column, rules and exact minute execution reach the shared kernel.
fn run(
    request: RangeRequest<'_>,
    min_hits: u64,
    commit: &str,
    attempt: Option<u64>,
) -> Result<String, String> {
    settings::current()?;
    let (data, guard): (RangeData, RangeGuard) = RangeInputs::load(request)?.into_parts();
    let RangeData {
        signal: span,
        execution: execution_bars,
        daily,
        exact_minute,
    } = data;
    guard.require_current()?;
    let signal_length = crate::stored::rung_length_micros(request.rung)?;
    let availability = crate::stored::vwap_availability(&span.key);
    let execution = execution_bars.as_ref().map(|span| crate::Execution {
        bars: &span.bars,
        signal_length_micros: signal_length,
    });
    let execution_slice = execution.map_or(span.bars.as_slice(), |source| source.bars);
    crate::validate_one_minute_execution(execution_slice)?;
    let executed =
        crate::stored_executed_digest(&span.bars, &exact_minute, &daily, execution_slice)?;
    let digest = guard.bind_digest(executed);
    let replay = crate::StoredReplay {
        daily: &daily,
        exact_minute: &exact_minute,
        signal_length_micros: signal_length,
        availability,
    };
    let column = prepare(request, &span.bars, replay, digest, commit, &guard)?;
    let horizon = crate::horizon_for(&span.bars, execution.is_some());
    let rungs = crate::grid_rungs(&span.bars);
    let rules = crate::Rules::derived(crate::floors_measured_on(&span.bars, execution), horizon);
    let lens = runner::rank::Lens::Payoff;
    let validate = crate::validate_from_env();
    let ladder = crate::ladder_for(min_hits)?;
    settings::resolved()?;
    let id = identity(&Run {
        mask: vocab::ConditionMask::default(),
        direction: Direction::Undirected,
        instrument: &span.key,
        timeframe: span.timeframe,
        params: Params::of(ladder).with_policy(&crate::policy_of(
            &span.bars, rules, lens, validate, horizon, rungs,
        )),
        data_digest: digest,
        commit,
        feed: span.vendor.as_str(),
    });
    let mut header =
        crate::span_banner(&span, request.underlying, request.from, request.to, commit);
    header.push_str(&guard.note());
    header.push_str(&crate::daily_reference_note(&daily, &exact_minute));
    let check = || {
        settings::resolved()?;
        guard.require_current()
    };
    Ok(crate::audit_bars_guarded(
        &crate::evaluator_stored(availability),
        span.bars,
        &header,
        min_hits,
        Some(&id),
        crate::AuditOptions {
            prepared_column: Some(column),
            replay: Some(replay),
            execution,
            native_minute_execution: span.timeframe == crate::EXECUTION_RUNG,
            recording: Some(crate::Recording {
                root: request.store_root,
                feed: span.vendor.as_str(),
                underlying: request.underlying,
                timeframe: span.timeframe,
                from: request.from,
                to: request.to,
                attempt,
                months_asked: span.asked,
                months_found: span.found,
            }),
            rules,
            lens,
            ceiling: None,
            validate,
        },
        Some(&check),
    ))
}

fn prepare(
    request: RangeRequest<'_>,
    bars: &[indicators::Candle],
    replay: crate::StoredReplay<'_>,
    digest: [u8; 32],
    commit: &str,
    guard: &RangeGuard,
) -> Result<indicators::column::Column, String> {
    let preparation = crate::preparation_attempt_with_commit(
        request.store_root,
        request.vendor,
        request.underlying,
        request.rung,
        digest,
        commit,
    )?;
    let prepared = crate::stored_anchored_column(
        bars,
        replay.daily,
        replay.exact_minute,
        replay.signal_length_micros,
        replay.availability,
    );
    #[cfg(test)]
    preparation_fault(&preparation);
    let source = guard.require_current();
    let completed = prepared.is_ok() && source.is_ok();
    let terminal = preparation.finish(if completed {
        crate::sweep_evidence::Completion::Completed
    } else {
        crate::sweep_evidence::Completion::Refused
    });
    if let Err(why) = source {
        return Err(match terminal {
            Ok(()) => format!("strict range preparation source refused: {why}"),
            Err(terminal) => format!(
                "strict range preparation source refused: {why}; preparation terminal also refused: {terminal}"
            ),
        });
    }
    terminal?;
    prepared
}

#[cfg(test)]
type PreparationHook = Box<dyn FnOnce(&crate::sweep_evidence::Attempt)>;

#[cfg(test)]
std::thread_local! {
    static PREPARATION_HOOK: std::cell::RefCell<Option<PreparationHook>> = const { std::cell::RefCell::new(None) };
}

#[cfg(test)]
pub(crate) struct PreparationFault;

#[cfg(test)]
impl PreparationFault {
    pub(crate) fn install(hook: impl FnOnce(&crate::sweep_evidence::Attempt) + 'static) -> Self {
        PREPARATION_HOOK.with(|held| {
            *held.borrow_mut() = Some(Box::new(hook));
        });
        Self
    }
}

#[cfg(test)]
impl Drop for PreparationFault {
    fn drop(&mut self) {
        PREPARATION_HOOK.with(|held| {
            held.borrow_mut().take();
        });
    }
}

#[cfg(test)]
fn preparation_fault(attempt: &crate::sweep_evidence::Attempt) {
    let hook = PREPARATION_HOOK.with(|held| held.borrow_mut().take());
    if let Some(hook) = hook {
        hook(attempt);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strict_range_arguments_refuse_before_source_loading() {
        for args in [&[][..], &["zerodha"][..]] {
            assert!(command(args).is_err());
        }
        for (year, value) in [("bad", "1"), ("65536", "1"), ("2025", "0"), ("2025", "13")] {
            assert!(month(year, value).is_err());
        }
        assert_eq!(month("2025", "12"), Ok((2025, 12)));
        for raw in ["", "0", "-1", "18446744073709551616", "2.5"] {
            assert!(bound(raw, "MAX_BYTES").is_err());
        }
        assert_eq!(bound("1", "MAX_RECORDS"), Ok(1));
        assert_eq!(bound("18446744073709551615", "MAX_BYTES"), Ok(u64::MAX));
    }

    #[test]
    fn strict_api_reports_never_hide_late_evidence_or_resource_refusals() {
        for refused in [
            "REAL MARKET DATA\nRESULT NOT RECORDED: final child seal changed\n",
            "REAL MARKET DATA\n  REFUSED -- memory ceiling\n",
            "REAL MARKET DATA\nrefused: parent write failed\n",
        ] {
            assert_eq!(checked_report(refused.to_owned()), Err(refused.to_owned()));
        }
        let complete = "REAL MARKET DATA\n  refused 0\nRESULT ALREADY RECORDED AND VERIFIED\n";
        assert_eq!(checked_report(complete.to_owned()), Ok(complete.to_owned()));
    }
}

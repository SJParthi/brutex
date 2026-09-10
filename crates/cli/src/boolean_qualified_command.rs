//! Predeclared finite-catalog, eight-timeframe, fixed-training qualification.
use crate::boolean_catalog_command::{
    catalog,
    prepared::{Input, Prepared},
};
use crate::boolean_qualification_plan::Plan;
use crate::boolean_qualified_journal::{Link, Slot, Writer};
use crate::candidate_universe::boolean_candidate_v1::{oos, statistics::admission::qualification};
use std::fmt::Write as _;
use std::path::Path;

pub(crate) fn command(args: &[&str], out: &mut String) -> u8 {
    match parse(args).and_then(|request| execute(request, out)) {
        Ok(()) => crate::OK,
        Err(why) => crate::refuse(out, &why),
    }
}
#[derive(Clone, Copy)]
struct Request<'a> {
    input: Input<'a>,
    catalog: &'a Path,
    later_from: (u16, u8),
    later_to: (u16, u8),
}
fn month(year: &str, month: &str) -> Result<(u16, u8), String> {
    let year = year.parse().map_err(|_| "qualification invalid year")?;
    let month = month.parse().map_err(|_| "qualification invalid month")?;
    pull::session::Day::new(year, month, 1).map_err(|why| why.to_string())?;
    Ok((year, month))
}
fn parse<'a>(args: &[&'a str]) -> Result<Request<'a>, String> {
    let [
        vendor,
        symbols,
        fy,
        fm,
        ty,
        tm,
        programs,
        horizon,
        points,
        output,
        lfy,
        lfm,
        lty,
        ltm,
    ] = args
    else {
        return Err("boolean-qualified-campaign-stored requires 14 explicit arguments: VENDOR SYMBOLS FY FM TY TM CATALOG HORIZON MAX_POINTS OUTPUT LATER_FY LATER_FM LATER_TY LATER_TM".into());
    };
    let from = month(fy, fm)?;
    let to = month(ty, tm)?;
    let later_from = month(lfy, lfm)?;
    let later_to = month(lty, ltm)?;
    if from > to || later_from > later_to || later_from <= to {
        return Err("qualification requires ordered and wholly later month spans".into());
    }
    let horizon =
        crate::knobs::horizon_count(horizon).ok_or("qualification HORIZON must be positive")?;
    let max_points = points
        .parse::<u64>()
        .ok()
        .filter(|n| *n > 0)
        .ok_or("qualification MAX_POINTS must be positive")?;
    Ok(Request {
        input: Input {
            vendor,
            symbols,
            from,
            to,
            horizon,
            max_points,
            output: Path::new(*output),
        },
        catalog: Path::new(*programs),
        later_from,
        later_to,
    })
}
fn execute(request: Request<'_>, out: &mut String) -> Result<(), String> {
    let input = Prepared::new(request.input, out)?;
    let programs = catalog(request.catalog, input.strict.max_bytes())?;
    let preflight = |from, to, out: &mut String| {
        crate::boolean_campaign::prepare(
            &crate::boolean_campaign::Request {
                rungs: crate::boolean_campaign::RungScope::ALL,
                vendor: request.input.vendor,
                symbols: request.input.symbols,
                from,
                to,
                programs: &programs,
                horizon: request.input.horizon,
                max_points: request.input.max_points,
                output: request.input.output,
                max_rung_jobs: 8,
            },
            out,
        )
    };
    let training = preflight(request.input.from, request.input.to, out)?;
    let later = preflight(request.later_from, request.later_to, out)?;
    if input.policy_digest() != training.preparation_policy_digest() {
        return Err("qualification policy changed during preparation".into());
    }
    run_prepared(&input, &training, &later, &programs, out).map(|_| ())
}

/// Shared complete qualification used by text catalogs and binary grammar batches.
/// Both preparations remain live through final child reconciliation.
#[expect(
    clippy::too_many_lines,
    reason = "one ordered source-bound qualification publication"
)]
pub(crate) fn run_prepared(
    input: &Prepared,
    training: &crate::boolean_campaign::Prepared,
    later: &crate::boolean_campaign::Prepared,
    programs: &[runner::expression::Expression],
    out: &mut String,
) -> Result<Link, String> {
    if input.policy_digest() != training.preparation_policy_digest()
        || training.program_digest() != crate::boolean_campaign::program_digest(programs)
    {
        return Err("qualification prepared catalog or policy differs".into());
    }
    let (later_from, later_to) = later.span();
    let plan = Plan::new(training, later)?;
    let numerical = plan.numerical_bounds()?;
    let [max_bytes, max_records] = plan.source_limits();
    let verify_bytes = max_bytes
        .checked_mul(4)
        .ok_or("qualification observation admission overflow")?;
    let mut journal = Writer::open_for_rungs(
        &input.output,
        plan.descriptor(),
        *plan.units(),
        plan.rungs(),
        max_bytes,
        max_records,
        |index, unit, link| verify_link(&input.output, &plan, index, unit, link, verify_bytes),
    )?;
    let _ = writeln!(
        out,
        "\nFIXED-TRAINING QUALIFICATION: complete declared catalog across selected intraday timeframes {:?}; one finite testing budget allocated before later pricing. Eight statistical units remain reserved; unselected units are unspent. Original training exits remain frozen. All zero/refused coordinates remain present. This is cost-excluded research, not Selection V6 or live-trading approval.\nCampaign: {}\nLater windows: {}; bootstrap draw resolution can reach allocated threshold: {}\n| Timeframe | State | Coordinates | Admitted | Rejected | Unmeasured | Refused |\n|---|---|---:|---:|---:|---:|---:|",
        plan.rungs().labels(),
        crate::identity_hex(&journal.identity()),
        plan.windows().len(),
        plan.resolution_reachable()?
    );
    let _ = writeln!(
        out,
        "Progress view: /backtest?boolean_qualified_campaign={}",
        crate::identity_hex(&journal.identity())
    );
    let _ = writeln!(
        out,
        "Dashboard BRUTEX_STORE: {}; command observation allowance: {verify_bytes} bytes (BRUTEX_BOOLEAN_OBSERVATION_BYTES). This is serialized evidence admission, not a RAM or latency guarantee.",
        input.output.display()
    );
    for (index, rung) in crate::ledger_all::LEDGER_RUNGS.iter().enumerate() {
        if !plan.rungs().contains(index) {
            continue;
        }
        training.require_current()?;
        later.require_current()?;
        if journal
            .slots()
            .get(index)
            .ok_or("qualification rung slot missing")?
            .complete
            .is_some()
        {
            let _ = writeln!(
                out,
                "| {rung} | Verified saved completion | — | — | — | — | — |"
            );
            continue;
        }
        journal.begin(index)?;
        let work = (|| {
            let original = input.run(rung, programs, out, |_| Ok(()))?;
            let mut observations = Vec::new();
            observations
                .try_reserve_exact(original.statistics().sources().len())
                .map_err(|why| why.to_string())?;
            // Original families already use bounded parallel workers. Later families
            // are retained serially here so proof mapping buffers are never multiplied.
            for (family_index, source) in original.statistics().sources().iter().enumerate() {
                training.require_catalog(index, family_index, source.identity())?;
                let family = source.family();
                let key = family.instrument();
                let candidate = input.request(rung, programs, key.underlying.as_str());
                let later_request = oos::LaterRequest {
                    output: &input.output,
                    inputs: candidate.inputs,
                    from: later_from,
                    to: later_to,
                    bounds: candidate.bounds,
                };
                let validation = oos::ValidationRequest {
                    requested: plan.requested()?,
                    windows: plan.windows(),
                    max_folds: plan.windows().len(),
                    mapping_bytes: candidate.bounds.bytes,
                    projection_bytes: candidate.bounds.bytes,
                };
                #[cfg(test)]
                let observed = if input.generated_fixture {
                    oos::produce_campaign_fixture(source, later_request, validation)?
                } else {
                    oos::produce_validated(source, later_request, validation)?
                };
                #[cfg(not(test))]
                let observed = oos::produce_validated(source, later_request, validation)?;
                observations.push(observed);
                let observed = observations
                    .last()
                    .ok_or("qualification later source missing after production")?;
                later.require_later_source(index, family_index, observed.source_identity())?;
            }
            let saved = qualification::produce(&qualification::Request {
                root: &input.output,
                training: &original,
                later: &observations,
                policy: input.policy(),
                procedure: plan.procedure(),
                plan: &plan,
                rung: index,
                bounds: qualification::Bounds {
                    candidates: max_records,
                    observations: max_records,
                    work: numerical.max_work,
                    memory: numerical.max_bytes,
                    bytes: max_bytes,
                },
            })?;
            saved.require_current()?;
            training.require_current()?;
            later.require_current()?;
            let link = Link {
                identity: saved.identity(),
                pin: saved.pin(),
            };
            let count = saved.count();
            let counts = saved.counts();
            finish_checked(index, &mut journal, link, |index, slot| {
                verify_link(
                    &input.output,
                    &plan,
                    index,
                    slot.unit,
                    slot.complete
                        .ok_or("qualification completion link absent")?,
                    verify_bytes,
                )
            })?;
            saved.require_current()?;
            training.require_current()?;
            later.require_current()?;
            let [admitted, rejected, unmeasured, refused] = counts;
            let _ = writeln!(
                out,
                "| {rung} | Saved and verified | {count} | {admitted} | {rejected} | {unmeasured} | {refused} |\nQualification {} completion {}",
                crate::identity_hex(&link.identity),
                crate::identity_hex(&link.pin)
            );
            Ok::<(), String>(())
        })();
        if let Err(why) = work {
            return Err(settle_refusal(index, rung, &mut journal, &why));
        }
    }
    training.require_current()?;
    later.require_current()?;
    if !journal.completed() {
        return Err("qualification campaign lacks a selected timeframe completion".into());
    }
    let _ = writeln!(
        out,
        "All {} selected qualification computations complete. Completion pin: {}. Eight statistical units remain reserved, unselected units unspent. Individual failed or unavailable policy checks remain visible.",
        plan.rungs().count(),
        crate::identity_hex(&journal.pin()?)
    );
    Ok(Link {
        identity: journal.identity(),
        pin: journal.pin()?,
    })
}

// A terminal write can fail for the same resource reason as the work. Keep the
// original cause first, and distinguish acknowledged refusal from uncertainty.
fn settle_refusal(index: usize, rung: &str, journal: &mut Writer, why: &str) -> String {
    let prefix = format!("{why}; qualification timeframe {rung} refused");
    let Some(slot) = journal.slots().get(index) else {
        return format!(
            "{prefix}; refusal publication unavailable: qualification rung slot missing; refusal persistence is unconfirmed"
        );
    };
    if slot.complete.is_some() {
        return format!(
            "{prefix}; completion remains recorded; subsequent verification failed; saved history was not rewritten"
        );
    }
    match journal.refuse(index, why) {
        Ok(()) => format!("{prefix}; refusal recorded; completed slots remain saved"),
        Err(audit) => format!(
            "{prefix}; refusal publication also failed: {audit}; refusal persistence is unconfirmed"
        ),
    }
}

fn verify_link(
    root: &Path,
    plan: &Plan,
    index: usize,
    unit: [u8; 32],
    link: Link,
    max_bytes: u64,
) -> Result<(), String> {
    let saved = qualification::reader::Reader::open(root, link.identity, max_bytes)?;
    if saved.completion_digest() != link.pin
        || saved.scope() != plan.descriptor()
        || saved.unit() != unit
        || saved.allocation() != plan.allocation(index)?
    {
        return Err("qualification completion references a different declared unit".into());
    }
    saved.require_current()
}

// Observe children sequentially so final reconciliation does not retain eight
// full decoded populations. An observation error never becomes a success banner.
fn finish_checked(
    index: usize,
    journal: &mut Writer,
    link: Link,
    mut check: impl FnMut(usize, &Slot) -> Result<(), String>,
) -> Result<(), String> {
    if Some(index) == journal.rungs().indices().last() {
        for (position, slot) in journal.slots().iter().enumerate() {
            if slot.complete.is_some() {
                check(position, slot)?;
            }
        }
    }
    journal.finish(index, link)?;
    if journal.completed() {
        for (position, slot) in journal.slots().iter().enumerate() {
            if journal.rungs().contains(position) {
                check(position, slot)?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "boolean_qualified_command_tests.rs"]
mod completion_tests;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn qualified_command_rejects_invalid_or_overlapping_scope_before_io() -> Result<(), String> {
        let valid = [
            "zerodha",
            "NIFTY,RELIANCE",
            "2025",
            "4",
            "2025",
            "5",
            "absent-catalog.md",
            "5",
            "50",
            "absent-output",
            "2025",
            "6",
            "2025",
            "6",
        ];
        let request = parse(&valid)?;
        assert_eq!(request.input.from, (2025, 4));
        assert_eq!(request.input.to, (2025, 5));
        assert_eq!(request.later_from, (2025, 6));
        assert_eq!(request.later_to, (2025, 6));
        assert!(parse(&[]).is_err());
        for (position, value) in [
            (3, "0"),
            (5, "13"),
            (7, "0"),
            (8, "0"),
            (11, "5"),
            (13, "5"),
            (2, "70000"),
            (10, "-1"),
        ] {
            let mut changed = valid;
            *changed.get_mut(position).ok_or("test field missing")? = value;
            assert!(parse(&changed).is_err(), "field {position} value {value}");
        }
        assert!(crate::is_sweep_command("boolean-qualified-campaign-stored"));
        Ok(())
    }
}

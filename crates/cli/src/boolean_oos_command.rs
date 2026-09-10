//! Executable later-period comparison, sharing the catalog's typed preparation.
use crate::boolean_catalog_command::{
    catalog,
    prepared::{Input, Prepared},
};
use crate::candidate_universe::boolean_candidate_v1::{self as candidate, oos};
use rayon::prelude::*;
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
    rung: &'a str,
    catalog: &'a Path,
    later_from: (u16, u8),
    later_to: (u16, u8),
}
fn month(year: &str, month: &str) -> Result<(u16, u8), String> {
    let year = year.parse::<u16>().map_err(|_| "invalid year")?;
    let month = month.parse::<u8>().map_err(|_| "invalid month")?;
    pull::session::Day::new(year, month, 1).map_err(|why| why.to_string())?;
    Ok((year, month))
}
fn parse<'a>(args: &[&'a str]) -> Result<Request<'a>, String> {
    let [
        vendor,
        symbols,
        rung,
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
        return Err("boolean-oos-stored requires 15 explicit arguments: VENDOR SYMBOLS RUNG FY FM TY TM CATALOG HORIZON MAX_POINTS OUTPUT LATER_FY LATER_FM LATER_TY LATER_TM".into());
    };
    let from = month(fy, fm)?;
    let to = month(ty, tm)?;
    let later_from = month(lfy, lfm)?;
    let later_to = month(lty, ltm)?;
    if from > to
        || later_from > later_to
        || later_from <= to
        || !crate::ledger_all::LEDGER_RUNGS.contains(rung)
    {
        return Err(
            "Boolean later comparison requires ordered, distinct later months and an intraday rung"
                .into(),
        );
    }
    let horizon = crate::knobs::horizon_count(horizon).ok_or("HORIZON must be positive")?;
    let max_points = points
        .parse::<u64>()
        .ok()
        .filter(|value| *value > 0)
        .ok_or("MAX_POINTS must be positive")?;
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
        rung,
        catalog: Path::new(*programs),
        later_from,
        later_to,
    })
}
fn execute(request: Request<'_>, out: &mut String) -> Result<(), String> {
    let prepared = Prepared::new(request.input, out)?;
    let programs = catalog(request.catalog, prepared.strict.max_bytes())?;
    out.push_str("\nLATER-PERIOD COMPARISON: every original program × both directions × every frozen TRAINING exit coordinate. Later prices never choose exits. Cost-excluded, unvalidated research; printed-fill amounts are not net trading profits. Intraday only; forced exit no later than15:10IST. This receipt is not admission or portfolio selection.\n");
    let lanes = std::thread::available_parallelism()
        .map_err(|why| why.to_string())?
        .get()
        .min(prepared.scope.families().len());
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(lanes)
        .build()
        .map_err(|why| why.to_string())?;
    let results:Vec<Result<String,String>> = pool.install(|| prepared.scope.families().par_iter().map(|family| {
        let key = family.instrument();
        let training_request = prepared.request(request.rung, &programs, key.underlying.as_str());
        let training = candidate::produce(training_request)?;
        let observed = oos::produce(
            &training,
            oos::LaterRequest {
                output: &prepared.output,
                inputs: training_request.inputs,
                from: request.later_from,
                to: request.later_to,
                bounds: training_request.bounds,
            },
        )?;
        observed.require_current()?;
        let trades = observed.rows().iter().try_fold(0_u64, |sum, row| {
            sum.checked_add(row.cell().trades)
                .ok_or("Boolean later trade total overflow")
        })?;
        Ok(format!(
            "{}: original catalog={} completion={}; later comparison={} completion={}; {} frozen coordinates, {} accepted later sessions, {} complete-coordinate trade observations. Later dates {:04}-{:02} through {:04}-{:02}.",
            family.instrument().underlying.as_str(),
            hex(training.identity()),
            hex(training.completion_digest()),
            hex(observed.identity()),
            hex(observed.completion_digest()),
            observed.rows().len(),
            observed.sessions().len(),
            trades,
            request.later_from.0,
            request.later_from.1,
            request.later_to.0,
            request.later_to.1
        ))
    }).collect());
    let mut refused = Vec::new();
    for (family, result) in prepared.scope.families().iter().zip(results) {
        match result {
            Ok(report) => {
                out.push_str(&report);
                out.push('\n');
            }
            Err(why) => {
                let reason = format!("{}: {why}", family.instrument().underlying.as_str());
                let _ = writeln!(out, "REFUSED {reason}");
                refused.push(reason);
            }
        }
    }
    if refused.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "Boolean later comparison incomplete; completed family receipts remain saved: {}",
            refused.join("; ")
        ))
    }
}
fn hex(value: [u8; 32]) -> String {
    value
        .iter()
        .fold(String::with_capacity(64), |mut out, byte| {
            let _ = write!(out, "{byte:02x}");
            out
        })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explicit_later_command_rejects_overlap_daily_zero_and_missing_arguments_before_io()
    -> Result<(), String> {
        let args = [
            "zerodha",
            "RELIANCE",
            "1min",
            "2025",
            "4",
            "2025",
            "5",
            "catalog.md",
            "5",
            "50",
            "output",
            "2025",
            "6",
            "2025",
            "7",
        ];
        let parsed = parse(&args)?;
        assert_eq!(parsed.later_from, (2025, 6));
        assert_eq!(parsed.input.to, (2025, 5));
        for (index, value) in [(2, "1day"), (8, "0"), (9, "0"), (12, "5"), (14, "0")] {
            let mut bad = args;
            *bad.get_mut(index).ok_or("test index")? = value;
            assert!(parse(&bad).is_err());
        }
        assert!(parse(args.get(..14).ok_or("test prefix")?).is_err());
        Ok(())
    }
}

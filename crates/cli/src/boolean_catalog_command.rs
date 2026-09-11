//! Explicit program-catalog research over retained real OHLCV source receipts.
//! A complete supplied catalog is never called exhaustive Boolean grammar search.

use std::fmt::Write as _;
use std::io::Read as _;
use std::path::Path;

use runner::expression::Expression;
use runner::research_family::{RESEARCH_FAMILY_CAPACITY_V1, ResearchScopeV1};

use crate::audited_range_command::StrictConfig;
use crate::candidate_universe::boolean_candidate_v1::{self as candidate, statistics};
use crate::ledger_all::{LedgerAllRequest, admission_policy, exit_policy, render_gate_census};

const VERB: &str = "boolean-catalog-stored";
// A parser format bound, not a depth or candidate truncation setting.
const CATALOG_BYTES: u64 = 65_536;

#[path = "boolean_catalog_prepared.rs"]
pub(crate) mod prepared;

#[derive(Clone, Copy)]
struct Request<'a> {
    vendor: &'a str,
    symbols: &'a str,
    rung: &'a str,
    from: (u16, u8),
    to: (u16, u8),
    catalog: &'a Path,
    horizon: runner::outcome::Horizon,
    max_points: u64,
    output: &'a Path,
}

pub(crate) fn command(args: &[&str], out: &mut String) -> u8 {
    let result = parse(args).and_then(|request| execute(request, out));
    match result {
        Ok(()) => crate::OK,
        Err(why) => crate::refuse(out, &why),
    }
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
        catalog,
        horizon,
        points,
        output,
    ] = args
    else {
        return Err("boolean-catalog-stored requires its 11 explicit arguments".into());
    };
    let month = |year: &str, month: &str| -> Result<(u16, u8), String> {
        let year = year.parse::<u16>().map_err(|_| "invalid year".to_owned())?;
        let month = month
            .parse::<u8>()
            .map_err(|_| "invalid month".to_owned())?;
        pull::session::Day::new(year, month, 1).map_err(|why| why.to_string())?;
        Ok((year, month))
    };
    let from = month(fy, fm)?;
    let to = month(ty, tm)?;
    if from > to || !crate::ledger_all::LEDGER_RUNGS.contains(rung) {
        return Err("ordered month span and an intraday rung are required".into());
    }
    let horizon = crate::knobs::horizon_count(horizon)
        .ok_or_else(|| "HORIZON must be a positive one-minute execution-bar count".to_owned())?;
    let max_points = points
        .parse::<u64>()
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| "MAX_POINTS must be a positive u64 integer".to_owned())?;
    Ok(Request {
        vendor,
        symbols,
        rung,
        from,
        to,
        catalog: Path::new(*catalog),
        horizon,
        max_points,
        output: Path::new(*output),
    })
}

fn scope(symbols: &str) -> Result<ResearchScopeV1, String> {
    if symbols.len() as u64 > CATALOG_BYTES {
        return Err("research symbol list exceeds its bounded input size".into());
    }
    let mut keys = Vec::new();
    keys.try_reserve_exact(RESEARCH_FAMILY_CAPACITY_V1)
        .map_err(|why| why.to_string())?;
    for symbol in symbols.split(',') {
        if keys.len() >= RESEARCH_FAMILY_CAPACITY_V1 {
            return Err("research symbol list exceeds the allowed scope capacity".into());
        }
        keys.push(crate::stored::swept_index(symbol.trim())?);
    }
    ResearchScopeV1::new(&keys, RESEARCH_FAMILY_CAPACITY_V1).map_err(|why| why.to_string())
}

pub(crate) fn catalog(path: &Path, max_bytes: u64) -> Result<Vec<Expression>, String> {
    let max_bytes = max_bytes.min(CATALOG_BYTES);
    let file = crate::readonly_file::open(path).map_err(|why| why.to_string())?;
    let before = crate::result_set::file_generation(&file, path)?;
    let mut bytes = Vec::new();
    (&file)
        .take(max_bytes + 1)
        .read_to_end(&mut bytes)
        .map_err(|why| why.to_string())?;
    if bytes.len() as u64 > max_bytes {
        return Err("explicit program catalog exceeds its byte bound; no prefix accepted".into());
    }
    let after = crate::result_set::file_generation(&file, path)?;
    if before != after {
        return Err("explicit program catalog changed during read".into());
    }
    parse_catalog(std::str::from_utf8(&bytes).map_err(|_| "program catalog is not UTF-8")?)
}

fn parse_catalog(source: &str) -> Result<Vec<Expression>, String> {
    let mut programs = Vec::new();
    let mut identities = std::collections::HashSet::new();
    for (index, line) in source.lines().enumerate() {
        let line = line.split('#').next().unwrap_or_default().trim();
        if line.is_empty() {
            continue;
        }
        let program = Expression::parse(line)
            .map_err(|why| format!("program catalog line {}: {why:?}", index + 1))?;
        identities.try_reserve(1).map_err(|why| why.to_string())?;
        if !identities.insert(brutex_core::blake3::hash(&program.encode())) {
            return Err(format!(
                "duplicate canonical program on catalog line {}",
                index + 1
            ));
        }
        programs.try_reserve(1).map_err(|why| why.to_string())?;
        programs.push(program);
    }
    if programs.is_empty() {
        return Err("explicit program catalog is empty".into());
    }
    Ok(programs)
}

fn execute(request: Request<'_>, out: &mut String) -> Result<(), String> {
    let prepared = prepared::Prepared::new(
        prepared::Input {
            vendor: request.vendor,
            symbols: request.symbols,
            from: request.from,
            to: request.to,
            horizon: request.horizon,
            max_points: request.max_points,
            output: request.output,
        },
        out,
    )?;
    let programs = catalog(request.catalog, prepared.strict.max_bytes())?;
    prepared
        .run(request.rung, &programs, out, |_| Ok(()))
        .map(|_| ())
}

fn render_family_receipts(
    results: &[Result<candidate::CommittedBooleanFamilyV1, String>],
    out: &mut String,
) {
    out.push_str("\nSaved family receipts returned by workers. These remain separate from whole-cohort completion and later source checks.\n\n| Instrument | Candidate catalog identity | Completion pin |\n|---|---|---|\n");
    for family in results.iter().filter_map(|result| result.as_ref().ok()) {
        let _ = writeln!(
            out,
            "| {} | {} | {} |",
            family.family().instrument(),
            crate::identity_hex(&family.identity()),
            crate::identity_hex(&family.completion_digest())
        );
    }
}

fn collect_families(
    scope: &ResearchScopeV1,
    results: Vec<Result<candidate::CommittedBooleanFamilyV1, String>>,
) -> Result<Vec<candidate::CommittedBooleanFamilyV1>, String> {
    if results.len() != scope.families().len() {
        return Err("catalog worker results do not cover the complete requested scope".into());
    }
    let mut committed = Vec::new();
    committed
        .try_reserve_exact(results.len())
        .map_err(|why| why.to_string())?;
    let mut refused = Vec::new();
    for (family, result) in scope.families().iter().zip(results) {
        match result {
            Ok(value) if value.family() == *family => committed.push(value),
            Ok(_) => refused.push(format!(
                "{}: worker returned a different family",
                family.instrument()
            )),
            Err(why) => refused.push(format!("{}: {why}", family.instrument())),
        }
    }
    if !refused.is_empty() {
        return Err(format!(
            "complete catalog cohort refused: {}",
            refused.join("; ")
        ));
    }
    Ok(committed)
}

struct StatisticsPlan {
    procedure: crate::population_statistics_v2::PopulationStatisticsProcedureV2,
    bounds: statistics::Bounds,
}

impl StatisticsPlan {
    fn new(families: usize, strict: &StrictConfig) -> Result<Self, String> {
        let draws = crate::ledger_all::BOOTSTRAP_DRAWS;
        let procedure = crate::population_statistics_v2::PopulationStatisticsProcedureV2::new(
            draws,
            crate::ledger_all::BOOTSTRAP_SEED,
            crate::ledger_all::BOOTSTRAP_BLOCK,
        )
        .map_err(|why| format!("statistical procedure: {why:?}"))?;
        let work = strict
            .max_records()
            .checked_mul(draws)
            .ok_or("statistics work budget overflow")?;
        let bounds = statistics::Bounds {
            families: families as u64,
            candidates: strict.max_records(),
            observations: strict.max_records(),
            bootstrap_work: work,
            split_work: work,
            memory_bytes: strict.max_bytes(),
            bytes: strict.max_bytes(),
        };
        Ok(Self { procedure, bounds })
    }
}

fn finish_statistics(
    root: &Path,
    families: Vec<candidate::CommittedBooleanFamilyV1>,
    policy: &runner::admission::AdmissionPolicyV1,
    plan: &StatisticsPlan,
    out: &mut String,
    record: &mut impl FnMut(prepared::Stage<'_>) -> Result<(), String>,
) -> Result<statistics::admission::CommittedBooleanAdmissionV1, String> {
    let statistics = statistics::produce(root, families, plan.procedure, plan.bounds)?;
    statistics.require_current()?;
    let measured = statistics.measurements();
    let layout = statistics.layout();
    let _ = writeln!(
        out,
        "\nSTATISTICS SAVED: {}\nCompletion: {}\nFor {} complete candidate coordinates across {} families and {} complete aligned sessions. Cross-validation: {} equal segments, {} complementary splits. Romano-Wolf: {}. Statistical storage does not itself assert a passing strategy.",
        crate::identity_hex(&statistics.identity()),
        crate::identity_hex(&statistics.completion_digest()),
        measured.candidates.len(),
        statistics.families().len(),
        layout.period_count(),
        layout.segment_count(),
        layout.split_count(),
        measured.romano_availability.label()
    );
    record(prepared::Stage::Statistics(&statistics))?;
    let admission = statistics::admission::produce(root, statistics, policy, plan.bounds.bytes)?;
    admission.require_current()?;
    render_admission(&admission, out);
    record(prepared::Stage::Admission(&admission))?;
    admission.require_current()?;
    Ok(admission)
}

fn render_admission(
    admission: &statistics::admission::CommittedBooleanAdmissionV1,
    out: &mut String,
) {
    use runner::admission::AdmissionReasonV1;
    let _ = writeln!(
        out,
        "\nRESEARCH ADMISSION SAVED: {}\nCompletion: {}\nStatistics: {}\nAll {} candidate decisions are retained. Training-only evidence does not establish later out-of-sample performance or Selection V6 authority.\n\n| Check | Failed | Unmeasured | Refused |\n|---|---:|---:|---:|",
        crate::identity_hex(&admission.identity()),
        crate::identity_hex(&admission.completion_digest()),
        crate::identity_hex(&admission.statistics().identity()),
        admission.rows().len(),
    );
    for reason in AdmissionReasonV1::ALL {
        let mut counts = [0_u64; 3];
        for row in admission.rows() {
            let verdict = row.verdict();
            for (count, bits) in
                counts
                    .iter_mut()
                    .zip([verdict.failed(), verdict.unmeasured(), verdict.refused()])
            {
                *count += u64::from(bits.contains(reason));
            }
        }
        let [failed, unmeasured, refused] = counts;
        let _ = writeln!(
            out,
            "| {} | {failed} | {unmeasured} | {refused} |",
            reason.name()
        );
    }
    out.push_str("\nFirst 256 decisions at most; the saved body retains every decision. Counts retain their measured/unmeasured/refused state. Reason masks are exact decimal integers.\n\n| Source coordinate | Candidate identity | Status | Support | Trades | Wins | Failed mask | Unmeasured mask | Refused mask |\n|---:|---|---|---|---|---|---:|---:|---:|\n");
    for row in admission.rows().iter().take(256) {
        let verdict = row.verdict();
        let values = row.values();
        let _ = writeln!(
            out,
            "| {} | {} | {:?} | {:?} | {:?} | {:?} | {} | {} | {} |",
            row.source_index(),
            crate::identity_hex(&row.identity()),
            verdict.status(),
            values.support_hits,
            values.trades,
            values.winning_trades,
            verdict.failed().bits(),
            verdict.unmeasured().bits(),
            verdict.refused().bits()
        );
    }
}

#[cfg(test)]
#[expect(clippy::unwrap_used, reason = "finite configuration assertions")]
mod tests {
    use super::*;

    #[test]
    fn impossible_statistics_budget_refuses_before_catalog_market_or_output_access() {
        const CHILD: &str = "BRUTEX_BOOLEAN_OVERFLOW_CHILD";
        if let Some(root) = std::env::var_os(CHILD) {
            let root = std::path::PathBuf::from(root);
            let output = root.join("must-not-be-created");
            let catalog = root.join("absent-catalog.md");
            let mut out = String::new();
            let status = command(
                &[
                    "zerodha",
                    "NIFTY",
                    "1min",
                    "2025",
                    "4",
                    "2025",
                    "5",
                    catalog.to_str().unwrap(),
                    "5",
                    "50",
                    output.to_str().unwrap(),
                ],
                &mut out,
            );
            assert_ne!(status, crate::OK);
            assert!(out.contains("statistics work budget overflow"), "{out}");
            assert!(!output.exists());
            assert!(!catalog.exists());
            return;
        }
        let root =
            std::env::temp_dir().join(format!("brutex-boolean-preflight-{}", std::process::id()));
        std::fs::create_dir(&root).unwrap();
        let mut child = std::process::Command::new(std::env::current_exe().unwrap());
        child.args(["--exact", "boolean_catalog_command::tests::impossible_statistics_budget_refuses_before_catalog_market_or_output_access", "--nocapture"]);
        for (name, _) in std::env::vars_os() {
            if name.to_string_lossy().starts_with("BRUTEX_") {
                child.env_remove(name);
            }
        }
        child
            .env(CHILD, &root)
            .env("BRUTEX_CHECKSUM_RECEIPTS", &root)
            .env("BRUTEX_CHECKSUM_MAX_BYTES", "4194304")
            .env("BRUTEX_CHECKSUM_MAX_RECORDS", u64::MAX.to_string())
            .env("BRUTEX_STORE", root.join("absent-market"))
            .env(
                "BRUTEX_ADMISSION_POLICY_FILE",
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../../config/intraday-research-v1.toml"),
            );
        let result = child.output().unwrap();
        std::fs::remove_dir(&root).unwrap();
        assert!(
            result.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(String::from_utf8_lossy(&result.stdout).contains("1 passed"));
    }

    #[test]
    fn incomplete_or_refused_worker_results_cannot_complete_a_cohort() {
        let scope = scope("NIFTY,BANKNIFTY").unwrap();
        assert!(collect_families(&scope, Vec::new()).is_err());
        let result = collect_families(
            &scope,
            vec![Err("source missing".into()), Err("budget refused".into())],
        );
        let why = result.err().unwrap();
        for detail in ["NIFTY", "BANKNIFTY", "source missing", "budget refused"] {
            assert!(why.contains(detail), "{why}");
        }
    }

    #[test]
    fn catalog_preserves_program_meaning_and_refuses_empty_duplicates_and_bad_syntax() {
        let programs = parse_catalog("# explicit research\n52 & 53\n52 | 53\n!52\n").unwrap();
        assert_eq!(programs.len(), 3);
        assert_ne!(
            programs.first().unwrap().encode(),
            programs.get(1).unwrap().encode()
        );
        for source in ["", "# only comments", "52\n52", "52 &", "999"] {
            assert!(parse_catalog(source).is_err(), "{source}");
        }
    }

    #[test]
    fn scope_is_canonical_and_never_widens_into_references_or_derivatives() {
        let first = scope("RELIANCE,NIFTY,BANKNIFTY").unwrap();
        assert_eq!(
            first.digest(),
            scope("BANKNIFTY,NIFTY,RELIANCE").unwrap().digest()
        );
        assert_eq!(first.families().len(), 3);
        for source in [
            "",
            "NIFTY,NIFTY",
            "INDIAVIX",
            "SENSEX",
            "NIFTY-FUT",
            "FINNIFTY",
        ] {
            assert!(scope(source).is_err(), "{source}");
        }
    }

    #[test]
    fn command_shape_requires_explicit_valid_intraday_span_and_units() {
        let args = [
            "dhan",
            "NIFTY",
            "1min",
            "2025",
            "4",
            "2025",
            "5",
            "catalog.md",
            "60",
            "50",
            "output",
        ];
        let request = parse(&args).unwrap();
        assert_eq!(request.horizon.as_bars(), 60);
        for (index, bad) in [(2, "1day"), (4, "13"), (3, "2026"), (8, "0"), (9, "-1")] {
            let mut invalid = args;
            *invalid.get_mut(index).unwrap() = bad;
            assert!(parse(&invalid).is_err(), "{bad}");
        }
        assert!(parse(&[]).is_err());
    }
}

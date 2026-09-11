//! Mandatory release evidence. This is an admission check, never an activator.
//! Scope and measurement provenance come from the reviewed release manifest;
//! raw reports are checked, rather than accepting rounded summary percentages.
use super::{PLAN_BYTES, directory, files, hex, positive};
use serde::Deserialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

const EVIDENCE_BYTES: u64 = 67_108_864;
const TOTAL_REPORT_BYTES: u64 = 268_435_456;
const ITEMS: usize = 4096;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Requirements {
    pub source_root: PathBuf,
    pub crates: Vec<String>,
    pub modules: Vec<String>,
    pub sources: Vec<Source>,
    pub evidence: Pin,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Source {
    pub path: String,
    pub bytes: String,
    pub blake3: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Pin {
    pub path: PathBuf,
    pub bytes: String,
    pub blake3: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Evidence {
    schema_version: u8,
    source_commit: String,
    scope_digest: String,
    complete: bool,
    coverage: Vec<Coverage>,
    mutations: Vec<Mutation>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Coverage {
    crate_name: String,
    full_crate: bool,
    branch_instrumented: bool,
    report: Pin,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Mutation {
    module: String,
    full_module: bool,
    census: Pin,
    outcomes: Pin,
    compiler_failures: Vec<CompilerFailure>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CompilerFailure {
    mutant_name: String,
    log: Pin,
}

fn relative(raw: &str) -> bool {
    !raw.is_empty()
        && raw.split('/').all(|part| {
            !part.is_empty()
                && part != "."
                && part != ".."
                && part
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"_.-".contains(&b))
        })
}

fn unique(values: &[String]) -> Result<BTreeSet<&str>, String> {
    if values.is_empty() || values.len() > ITEMS {
        return Err("mandatory gate scope is empty or exceeds its finite bound".into());
    }
    let out = values.iter().map(String::as_str).collect::<BTreeSet<_>>();
    if out.len() != values.len() || out.iter().any(|v| !relative(v)) {
        return Err("mandatory gate scope has duplicates or noncanonical paths".into());
    }
    Ok(out)
}

/// Length-framed scope binds exact source bytes, crates and mutation modules.
/// The trusted measurement coordinator records this before running its tools.
pub(super) fn scope_digest(commit: &str, required: &Requirements) -> String {
    let mut hash = brutex_core::blake3::Hasher::new();
    fn part(hash: &mut brutex_core::blake3::Hasher, value: &str) {
        hash.update(&(value.len() as u64).to_le_bytes());
        hash.update(value.as_bytes());
    }
    part(&mut hash, "brutex-deployment-gates-v1");
    part(&mut hash, commit);
    for scope in [&required.crates, &required.modules] {
        hash.update(&(scope.len() as u64).to_le_bytes());
        for item in scope {
            part(&mut hash, item);
        }
    }
    hash.update(&(required.sources.len() as u64).to_le_bytes());
    for source in &required.sources {
        for value in [&source.path, &source.bytes, &source.blake3] {
            part(&mut hash, value);
        }
    }
    hash.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

fn read(
    pin: &Pin,
    cap: u64,
    total: &mut u64,
    held: &mut Vec<files::Verified>,
) -> Result<Vec<u8>, String> {
    let bytes = positive(&pin.bytes)?;
    *total = total
        .checked_add(bytes)
        .ok_or("mandatory report byte overflow")?;
    if bytes > cap
        || *total > TOTAL_REPORT_BYTES
        || !pin.path.is_absolute()
        || !hex(&pin.blake3, 64)
    {
        return Err("mandatory report exceeds its physical limit or has an invalid pin".into());
    }
    held.push(files::verify(&pin.path, bytes, &pin.blake3)?);
    files::bytes(&pin.path, cap)
}

fn covered(summary: &Value, key: &str) -> Result<u64, String> {
    let count = summary
        .get(key)
        .and_then(|v| v.get("count"))
        .and_then(Value::as_u64);
    let done = summary
        .get(key)
        .and_then(|v| v.get("covered"))
        .and_then(Value::as_u64);
    match (count, done) {
        (Some(count), Some(done)) if count == done => Ok(count),
        _ => Err(format!(
            "mandatory {key} coverage is absent or below 100%; rounded percentages are ignored"
        )),
    }
}

fn coverage(raw: &[u8], expected: &BTreeSet<PathBuf>) -> Result<(), String> {
    let report = files::json(raw, EVIDENCE_BYTES)
        .map_err(|_| "LLVM coverage report is malformed or duplicated")?;
    if report.get("type").and_then(Value::as_str) != Some("llvm.coverage.json.export") {
        return Err("mandatory coverage must be an LLVM export".into());
    }
    let data = report
        .get("data")
        .and_then(Value::as_array)
        .ok_or("coverage data missing")?;
    let mut seen = BTreeSet::new();
    let mut lines = 0_u64;
    for unit in data {
        for file in unit
            .get("files")
            .and_then(Value::as_array)
            .ok_or("coverage files missing")?
        {
            let path = PathBuf::from(
                file.get("filename")
                    .and_then(Value::as_str)
                    .ok_or("coverage filename missing")?,
            );
            if !expected.contains(&path) {
                continue;
            }
            if !seen.insert(path) {
                return Err("duplicate mandatory coverage file".into());
            }
            let summary = file.get("summary").ok_or("coverage file summary missing")?;
            lines = lines
                .checked_add(covered(summary, "lines")?)
                .ok_or("coverage line overflow")?;
            covered(summary, "branches")?;
        }
    }
    if &seen != expected || lines == 0 {
        return Err("whole-crate coverage omits source files or has no measured lines".into());
    }
    Ok(())
}

fn mutant_key(value: &Value, module: &str) -> Result<String, String> {
    if value.get("file").and_then(Value::as_str) != Some(module) {
        return Err("mutation census contains a foreign module".into());
    }
    let mut identity = serde_json::Map::new();
    for key in [
        "file",
        "package",
        "function",
        "span",
        "replacement",
        "genre",
        "name",
    ] {
        identity.insert(
            key.into(),
            value
                .get(key)
                .ok_or("mutation identity field missing")?
                .clone(),
        );
    }
    Ok(Value::Object(identity).to_string())
}

fn compiler_invalid(row: &Value, mutant: &Value, raw: &[u8]) -> Result<(), String> {
    let phases = row
        .get("phase_results")
        .and_then(Value::as_array)
        .ok_or("compiler failure has no build phase")?;
    if phases.len() != 1
        || phases[0].get("phase").and_then(Value::as_str) != Some("Build")
        || phases[0].get("process_status") != Some(&serde_json::json!({"Failure":101}))
    {
        return Err("unviable case is not a completed compiler build failure".into());
    }
    let name = mutant
        .get("name")
        .and_then(Value::as_str)
        .ok_or("mutant name missing")?;
    let module = mutant
        .get("file")
        .and_then(Value::as_str)
        .ok_or("mutant file missing")?;
    let line = mutant
        .get("span")
        .and_then(|v| v.get("start"))
        .and_then(|v| v.get("line"))
        .and_then(Value::as_u64)
        .ok_or("mutant line missing")?;
    let log = std::str::from_utf8(raw).map_err(|_| "compiler failure log is not UTF-8")?;
    if !log.starts_with(&format!("\n*** {name}\n"))
        || log.trim_end().lines().last() != Some("*** result: Failure(101)")
        || [
            "internal compiler error",
            "panicked at",
            "No space left on device",
            "timed out",
            "signal:",
        ]
        .iter()
        .any(|v| log.contains(v))
    {
        return Err(
            "compiler failure log is foreign, incomplete or contains infrastructure failure".into(),
        );
    }
    let mut diagnostic = false;
    let mut proven = false;
    for text in log.lines() {
        if let Some(tail) = text.strip_prefix("error[E") {
            diagnostic = tail
                .get(..4)
                .is_some_and(|v| v.bytes().all(|b| b.is_ascii_digit()))
                && tail.get(4..6) == Some("]:");
        } else if diagnostic && text.trim_start().starts_with("-->") {
            proven |= text
                .trim_start()
                .starts_with(&format!("--> {module}:{line}:"));
            diagnostic = false;
        }
    }
    if !proven {
        return Err(
            "unviable case lacks a Rust error diagnostic at the exact mutation location".into(),
        );
    }
    Ok(())
}

fn caught(row: &Value) -> Result<(), String> {
    let phases = row
        .get("phase_results")
        .and_then(Value::as_array)
        .ok_or("caught mutation phase evidence is missing")?;
    // The retained cargo-mutants corpora use native cargo test: its completed
    // test-failure status is 101. Other runners/statuses remain unsupported,
    // rather than turning timeouts or infrastructure failures into catches.
    if phases.len() != 2
        || phases[0].get("phase").and_then(Value::as_str) != Some("Build")
        || phases[0].get("process_status").and_then(Value::as_str) != Some("Success")
        || phases[1].get("phase").and_then(Value::as_str) != Some("Test")
        || phases[1].get("process_status") != Some(&serde_json::json!({"Failure":101}))
    {
        return Err(
            "caught mutation lacks a completed successful Build and failing cargo Test (101)"
                .into(),
        );
    }
    Ok(())
}

fn mutations(
    census: &[u8],
    outcomes: &[u8],
    module: &str,
    failures: &BTreeMap<String, Vec<u8>>,
) -> Result<(), String> {
    let census = files::json(census, EVIDENCE_BYTES)
        .map_err(|_| "mutation census is malformed or duplicated")?;
    let list = census.as_array().ok_or("mutation census is not an array")?;
    if list.is_empty() {
        return Err("full-module mutation census is empty; completeness unproved".into());
    }
    let mut wanted = BTreeSet::new();
    for row in list {
        if !wanted.insert(mutant_key(row, module)?) {
            return Err("duplicate mutant in full-module census".into());
        }
    }
    let report = files::json(outcomes, EVIDENCE_BYTES)
        .map_err(|_| "mutation outcomes are malformed or duplicated")?;
    let rows = report
        .get("outcomes")
        .and_then(Value::as_array)
        .ok_or("mutation outcomes missing")?;
    let mut baseline = false;
    let mut seen = BTreeSet::new();
    let mut compiler_cases = BTreeSet::new();
    for row in rows {
        let scenario = row.get("scenario").ok_or("mutation scenario missing")?;
        if scenario == "Baseline" {
            if baseline || row.get("summary").and_then(Value::as_str) != Some("Success") {
                return Err("mutation baseline missing, duplicated or unsuccessful".into());
            }
            let phases = row
                .get("phase_results")
                .and_then(Value::as_array)
                .ok_or("mutation baseline phases missing")?;
            if phases.len() != 2
                || phases
                    .iter()
                    .zip(["Build", "Test"])
                    .any(|(phase, expected)| {
                        phase.get("phase").and_then(Value::as_str) != Some(expected)
                            || phase.get("process_status").and_then(Value::as_str)
                                != Some("Success")
                    })
            {
                return Err("mutation baseline did not complete clean build and tests".into());
            }
            baseline = true;
        } else {
            let mutant = scenario
                .get("Mutant")
                .ok_or("unsupported mutation scenario")?;
            match row.get("summary").and_then(Value::as_str) {
                Some("CaughtMutant") => caught(row)?,
                Some("Unviable") => {
                    let name = mutant
                        .get("name")
                        .and_then(Value::as_str)
                        .ok_or("mutant name missing")?;
                    compiler_invalid(
                        row,
                        mutant,
                        failures
                            .get(name)
                            .ok_or("unviable case has no pinned compiler failure evidence")?,
                    )?;
                    compiler_cases.insert(name.to_owned());
                }
                _ => {
                    return Err(
                        "mandatory mutation evidence has a survivor, timeout or unresolved case"
                            .into(),
                    );
                }
            }
            if !seen.insert(mutant_key(mutant, module)?) {
                return Err("duplicate mutation outcome".into());
            }
        }
    }
    if !baseline || seen != wanted {
        return Err("mutation outcomes do not complete the exact full-module census".into());
    }
    if compiler_cases != failures.keys().cloned().collect() {
        return Err("compiler failure evidence does not match exact unviable cases".into());
    }
    Ok(())
}

pub(super) fn verify(
    commit: &str,
    required: Option<&Requirements>,
) -> Result<files::Retained, String> {
    let required = required.ok_or(
        "mandatory exact-source whole-crate coverage and full-module mutation evidence is missing",
    )?;
    let root = directory(&required.source_root)?;
    let crates = unique(&required.crates)?;
    if crates.iter().any(|name| name.contains('/')) {
        return Err("crate scope must contain names".into());
    }
    let modules = unique(&required.modules)?;
    if required.sources.is_empty() || required.sources.len() > ITEMS {
        return Err("source census missing or oversized".into());
    }
    let mut expected = BTreeSet::from([PathBuf::from("Cargo.toml"), PathBuf::from("Cargo.lock")]);
    let mut per_crate = BTreeMap::new();
    let mut inventories = Vec::new();
    for name in &crates {
        let folder = root.join("crates").join(name);
        let inventory = files::Inventory::capture(&folder, ITEMS, true)?;
        let rust = inventory.entries().clone();
        if rust.is_empty() {
            return Err("required crate has no Rust source inventory".into());
        }
        for path in &rust {
            expected.insert(
                path.strip_prefix(&root)
                    .map_err(|_| "source escaped root")?
                    .to_path_buf(),
            );
        }
        expected.insert(PathBuf::from(format!("crates/{name}/Cargo.toml")));
        per_crate.insert(*name, rust);
        inventories.push(inventory);
    }
    let mut held = Vec::new();
    let mut declared = BTreeSet::new();
    let mut source_bytes = 0_u64;
    for source in &required.sources {
        if !relative(&source.path)
            || !hex(&source.blake3, 64)
            || !declared.insert(PathBuf::from(&source.path))
        {
            return Err("source census has invalid or duplicate entries".into());
        }
        source_bytes = source_bytes
            .checked_add(positive(&source.bytes)?)
            .ok_or("source byte overflow")?;
        if source_bytes > EVIDENCE_BYTES {
            return Err("source census exceeds its complete 64 MiB byte ceiling".into());
        }
        held.push(files::verify(
            &root.join(&source.path),
            positive(&source.bytes)?,
            &source.blake3,
        )?);
    }
    if declared != expected
        || modules
            .iter()
            .any(|m| !declared.contains(Path::new(m)) || !m.ends_with(".rs"))
    {
        return Err(
            "mandatory source census differs from complete required crates or mutation modules"
                .into(),
        );
    }
    let mut total = 0;
    let raw = read(&required.evidence, PLAN_BYTES, &mut total, &mut held)?;
    let evidence: Evidence = serde_json::from_slice(&raw)
        .map_err(|_| "mandatory evidence schema invalid, duplicated or unknown")?;
    if evidence.schema_version != 1
        || !evidence.complete
        || evidence.source_commit != commit
        || evidence.scope_digest != scope_digest(commit, required)
    {
        return Err(
            "mandatory evidence is incomplete, stale or bound to a different exact source/scope"
                .into(),
        );
    }
    let mut measured = BTreeSet::new();
    for item in evidence.coverage {
        if !item.full_crate
            || !item.branch_instrumented
            || !measured.insert(item.crate_name.clone())
        {
            return Err("coverage is partial, not branch-instrumented or duplicated".into());
        }
        let expected = per_crate
            .get(item.crate_name.as_str())
            .ok_or("foreign coverage crate")?;
        coverage(
            &read(&item.report, EVIDENCE_BYTES, &mut total, &mut held)?,
            expected,
        )?;
    }
    if measured.iter().map(String::as_str).collect::<BTreeSet<_>>() != crates {
        return Err("mandatory whole-crate coverage is missing".into());
    }
    let mut measured = BTreeSet::new();
    for item in evidence.mutations {
        if !item.full_module
            || !modules.contains(item.module.as_str())
            || !measured.insert(item.module.clone())
        {
            return Err("mutation scope is partial, foreign or duplicated".into());
        }
        let mut failures = BTreeMap::new();
        if item.compiler_failures.len() > ITEMS {
            return Err("compiler failure evidence count exceeds its bound".into());
        }
        for failure in item.compiler_failures {
            let raw = read(&failure.log, EVIDENCE_BYTES, &mut total, &mut held)?;
            if failures.insert(failure.mutant_name, raw).is_some() {
                return Err("duplicate compiler failure evidence".into());
            }
        }
        mutations(
            &read(&item.census, EVIDENCE_BYTES, &mut total, &mut held)?,
            &read(&item.outcomes, EVIDENCE_BYTES, &mut total, &mut held)?,
            &item.module,
            &failures,
        )?;
    }
    if measured.iter().map(String::as_str).collect::<BTreeSet<_>>() != modules {
        return Err("mandatory full-module mutation evidence is missing".into());
    }
    let retained = files::Retained {
        files: held,
        inventories,
    };
    retained.require_current()?;
    Ok(retained)
}

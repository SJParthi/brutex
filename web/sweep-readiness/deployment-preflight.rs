//! Read-only staged-release verification; never starts or replaces a service.
//! See deployment-preflight.md for the pinned manifest and native build command.
#![forbid(unsafe_code)]

use api::detail::{MAX_CONCURRENT, MAX_SCAN_BYTES};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::fs;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

// Use the actual server-owned parser, including its addressability ceiling.
// This standalone observer is not part of the backend build graph.
#[path = "../../crates/api/src/boolean_observation_budget.rs"]
mod boolean_observation_budget;
#[path = "../../crates/api/src/boolean_search_budget.rs"]
mod boolean_search_budget;
#[path = "deployment-files.rs"]
mod files;
#[path = "deployment-gates.rs"]
mod gates;

const PLAN_BYTES: u64 = 1_048_576;
const MAX_FILES: usize = 4096;
const STATUS_BYTES: usize = 1_048_576;
const REQUIRED_CHECKS: [&str; 7] = [
    "Comparison matches the reviewed report",
    "Workspace formatting",
    "Workspace integration",
    "Workspace lint checks",
    "Dependency policy",
    "Frontier browser regressions",
    "Browser type checks",
];

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Plan {
    schema_version: u8,
    source_commit: String,
    artifact_bytes: String,
    observation_bytes: String,
    replay_nodes: String,
    required_evidence_bytes: String,
    web_root: PathBuf,
    source_store: PathBuf,
    output_root: PathBuf,
    server_store: PathBuf,
    masters_root: PathBuf,
    logs_root: PathBuf,
    address: SocketAddr,
    artifacts: Vec<Artifact>,
    #[serde(default)]
    mandatory_gates: Option<gates::Requirements>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Artifact {
    role: Role,
    path: PathBuf,
    bytes: String,
    blake3: String,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Role {
    Api,
    Cli,
    Checks,
    Web,
}

fn positive(raw: &str) -> Result<u64, String> {
    if raw.is_empty() || raw.starts_with('0') || !raw.bytes().all(|b| b.is_ascii_digit()) {
        return Err("positive byte limits must be canonical decimal strings".into());
    }
    raw.parse().map_err(|_| "byte limit overflow".into())
}

fn hex(raw: &str, width: usize) -> bool {
    raw.len() == width
        && raw
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn directory(path: &Path) -> Result<PathBuf, String> {
    if !path.is_absolute() || !fs::metadata(path).is_ok_and(|m| m.is_dir()) {
        return Err("every configured root must be an existing absolute directory".into());
    }
    fs::canonicalize(path).map_err(|_| "configured root cannot be resolved".into())
}

fn validate_checks(raw: &[u8]) -> Result<(), String> {
    let raw = std::str::from_utf8(raw).map_err(|_| "verification receipt is not UTF-8")?;
    let body = raw
        .strip_prefix("window.BRUTEX_AUDIT_CHECKS = ")
        .and_then(|v| v.trim_end().strip_suffix(';'))
        .ok_or("not the verifier's acknowledged evidence projection")?;
    let body = files::json(body.as_bytes(), PLAN_BYTES)
        .map_err(|_| "invalid or duplicated verification evidence")?;
    if body.get("complete") != Some(&Value::Bool(true))
        || body.get("stable") != Some(&Value::Bool(true))
    {
        return Err("verification is incomplete or its source checkpoint changed".into());
    }
    let checks = body
        .get("checks")
        .and_then(Value::as_array)
        .ok_or("checks missing")?;
    if checks.len() != REQUIRED_CHECKS.len() {
        return Err("a focused subset is not a full seven-check verification".into());
    }
    for (check, expected) in checks.iter().zip(REQUIRED_CHECKS) {
        if check.get("name").and_then(Value::as_str) != Some(expected)
            || check.get("passed") != Some(&Value::Bool(true))
            || check.get("exit").and_then(Value::as_i64) != Some(0)
        {
            return Err("full verification order, result or exit differs".into());
        }
    }
    Ok(())
}

#[derive(Debug)]
struct VerifiedReport {
    body: Value,
    retained: files::Retained,
}

fn verify_stage(plan: &Plan) -> Result<VerifiedReport, String> {
    if !matches!(plan.schema_version, 1 | 2)
        || (plan.schema_version == 1 && plan.mandatory_gates.is_some())
        || !hex(&plan.source_commit, 40)
    {
        return Err("unsupported plan version or noncanonical source commit".into());
    }
    if !plan.address.ip().is_loopback() || plan.address.port() == 0 {
        return Err("status address must be an explicit nonzero loopback socket".into());
    }
    let budget = boolean_observation_budget::BooleanObservationBudget::from_value(Some(
        OsStr::new(&plan.observation_bytes),
    ))?;
    let required = positive(&plan.required_evidence_bytes)?;
    let replay = boolean_search_budget::ReplayBudget::parse(Some(OsStr::new(&plan.replay_nodes)))?;
    if required > budget.bytes() {
        return Err(budget.context("recorded complete evidence exceeds the proposed runtime limit"));
    }
    let web = directory(&plan.web_root)?.join("build");
    let web = directory(&web)?;
    let source = directory(&plan.source_store)?;
    let output = directory(&plan.output_root)?;
    let server = directory(&plan.server_store)?;
    if output != server {
        return Err("dashboard server store differs from exact command output root".into());
    }
    directory(&plan.masters_root)?;
    directory(&plan.logs_root)?;
    if plan.artifacts.len() > MAX_FILES || plan.artifacts.len() < 6 {
        return Err("artifact census exceeds its finite bound or is incomplete".into());
    }
    let ceiling = positive(&plan.artifact_bytes)?;
    let mut total = 0_u64;
    let mut all = BTreeSet::new();
    let mut expected_web = BTreeSet::new();
    let mut roles = [0_u64; 3];
    let mut held = Vec::new();
    held.try_reserve_exact(plan.artifacts.len())
        .map_err(|_| "artifact handle allocation refused")?;
    for artifact in &plan.artifacts {
        if !artifact.path.is_absolute() || !hex(&artifact.blake3, 64) {
            return Err("artifact paths and digests must be absolute/canonical".into());
        }
        let canonical = fs::canonicalize(&artifact.path).map_err(|_| "artifact is missing")?;
        if !all.insert(canonical.clone()) {
            return Err("duplicate artifact path or alias".into());
        }
        let bytes = positive(&artifact.bytes)?;
        total = total
            .checked_add(bytes)
            .ok_or("artifact byte count overflow")?;
        if total > ceiling {
            return Err("complete staged artifacts exceed the declared scan limit".into());
        }
        held.push(files::verify(&artifact.path, bytes, &artifact.blake3)?);
        match artifact.role {
            Role::Web => {
                if !canonical.starts_with(&web) {
                    return Err("web artifact escapes the exact build root".into());
                }
                expected_web.insert(canonical);
            }
            Role::Api => roles[0] += 1,
            Role::Cli => roles[1] += 1,
            Role::Checks => {
                roles[2] += 1;
                validate_checks(&files::bytes(&artifact.path, PLAN_BYTES)?)?;
            }
        }
    }
    if roles != [1, 1, 1] {
        return Err("exactly one API, CLI and full-check evidence artifact are required".into());
    }
    let inventory = files::Inventory::capture(&web, MAX_FILES, false)?;
    if &expected_web != inventory.entries() {
        return Err("build tree has an extra, missing or unpinned file".into());
    }
    for required in ["index.html", "backtest.html", "_app/version.json"] {
        if !expected_web.contains(&web.join(required)) {
            return Err("the staged dashboard is missing a required entrypoint".into());
        }
    }
    let retained = files::Retained {
        files: held,
        inventories: vec![inventory],
    };
    retained.require_current()?;
    let body = json!({
        "state":"pinned_artifacts_verified", "source_commit":plan.source_commit,
        "artifact_count":plan.artifacts.len().to_string(), "artifact_bytes":total.to_string(),
        "observation_byte_limit":budget.bytes().to_string(),
        "replay_node_limit":replay.nodes().to_string(),
        "required_serialized_evidence_bytes":required.to_string(),
        "input_and_output_roots_equal":source==output,
        "dashboard_output_root_matches":true,
        "limits":"pinned bytes and recorded checks; not executable introspection, raw-market re-audit, available-RAM measurement or full coverage/mutation clearance"
    });
    Ok(VerifiedReport { body, retained })
}

fn response_json(raw: &[u8]) -> Result<Value, String> {
    let split = raw
        .windows(4)
        .position(|v| v == b"\r\n\r\n")
        .ok_or("HTTP headers incomplete")?;
    let headers = std::str::from_utf8(&raw[..split]).map_err(|_| "HTTP headers invalid")?;
    let mut lines = headers.lines();
    if !matches!(lines.next(), Some("HTTP/1.1 200 OK" | "HTTP/1.0 200 OK")) {
        return Err("status route did not return HTTP 200".into());
    }
    let mut length = None;
    for line in lines {
        let (name, value) = line.split_once(':').ok_or("HTTP header malformed")?;
        if name.eq_ignore_ascii_case("transfer-encoding") {
            return Err("transfer-encoded status is unsupported; no idle inference".into());
        }
        if name.eq_ignore_ascii_case("content-length") {
            if length.is_some() {
                return Err("duplicate response length".into());
            }
            length = Some(
                value
                    .trim()
                    .parse::<usize>()
                    .map_err(|_| "invalid response length")?,
            );
        }
    }
    let body = &raw[split + 4..];
    if length != Some(body.len()) {
        return Err("status body length differs or is absent".into());
    }
    files::json(body, STATUS_BYTES as u64)
        .map_err(|_| "status body is not valid unique-key JSON".into())
}

fn get(address: SocketAddr, route: &str) -> Result<Value, String> {
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut stream = TcpStream::connect_timeout(&address, Duration::from_secs(2))
        .map_err(|_| "local status connection unavailable")?;
    stream
        .set_write_timeout(Some(Duration::from_secs(2)))
        .map_err(|_| "timeout configuration failed")?;
    write!(stream, "GET {route} HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\nAccept: application/json\r\n\r\n")
        .map_err(|_| "local status request failed")?;
    let mut raw = Vec::new();
    let mut block = [0; 8192];
    loop {
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .ok_or("local status deadline reached")?;
        stream
            .set_read_timeout(Some(remaining))
            .map_err(|_| "timeout configuration failed")?;
        let count = stream
            .read(&mut block)
            .map_err(|_| "local status read failed or timed out")?;
        if count == 0 {
            break;
        }
        if raw
            .len()
            .checked_add(count)
            .is_none_or(|n| n > STATUS_BYTES)
        {
            return Err("local status exceeds its complete-response limit".into());
        }
        raw.extend_from_slice(&block[..count]);
    }
    response_json(&raw)
}

fn observation(route: &str, body: Result<Value, String>) -> Value {
    let Ok(body) = body else {
        return json!({"route":route,"state":"unknown","reason":"unavailable, refused or malformed complete response"});
    };
    let (state, reason) = match route {
        "/pull/run.json" => match body.get("running").and_then(Value::as_bool) {
            Some(true) => ("active", "pull slot is running"),
            Some(false) => (
                "not_observed_active",
                "pull slot is idle in this snapshot only",
            ),
            None => ("unknown", "pull slot state missing"),
        },
        "/autopilot.json" => match (
            body.get("state").and_then(Value::as_str),
            body.get("pull_active").and_then(Value::as_bool),
        ) {
            (Some("paused"), Some(false)) => (
                "paused",
                "scheduler pause is preserved; not a complete writer census",
            ),
            (Some("running"), _) | (_, Some(true)) => ("active", "scheduler reports active work"),
            _ => ("unknown", "scheduler state cannot prove a safe handoff"),
        },
        "/backtest/run.json" => match body.get("running").filter(|v| v.is_object()) {
            Some(run) if run.get("in_flight") == Some(&Value::Bool(true)) => {
                ("active", "observed sweep is in flight")
            }
            Some(run) if run.get("status").and_then(Value::as_str) == Some("unknown") => {
                ("unknown", "external sweep observation is uncertain")
            }
            Some(run)
                if run.get("in_flight") == Some(&Value::Bool(false))
                    && matches!(
                        run.get("where").and_then(Value::as_str),
                        Some("browser" | "cli")
                    ) =>
            {
                (
                    "not_observed_active",
                    "observed run is terminal; not an external process census",
                )
            }
            _ => (
                "unknown",
                "absent or malformed sweep observation is not an external process census",
            ),
        },
        _ => (
            "unknown",
            "recovery route exposes recent events, not an authoritative active-writer census",
        ),
    };
    json!({"route":route,"state":state,"reason":reason})
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Mode {
    offline: bool,
    require_release_gates: bool,
}

fn arguments(args: impl Iterator<Item = std::ffi::OsString>) -> Result<(Mode, PathBuf), String> {
    let mut mode = Mode {
        offline: false,
        require_release_gates: false,
    };
    let mut path = None;
    for arg in args {
        if arg == "--offline" && !mode.offline && path.is_none() {
            mode.offline = true;
        } else if arg == "--require-release-gates" && !mode.require_release_gates && path.is_none()
        {
            mode.require_release_gates = true;
        } else if path.is_none() && !arg.to_string_lossy().starts_with('-') {
            path = Some(PathBuf::from(arg));
        } else {
            return Err(
                "usage: deployment-preflight [--offline] [--require-release-gates] PLAN.json"
                    .into(),
            );
        }
    }
    Ok((mode, path.ok_or("exactly one pinned plan is required")?))
}

fn preflight(
    plan: &Plan,
    mode: Mode,
    mut request: impl FnMut(SocketAddr, &str) -> Result<Value, String>,
) -> Result<VerifiedReport, String> {
    let mut stage = verify_stage(plan)?;
    let gate = gates::verify(&plan.source_commit, plan.mandatory_gates.as_ref());
    let passed = gate.is_ok();
    let mandatory = match gate {
        Ok(retained) => {
            stage.retained.extend(retained);
            json!({"state":"passed", "source_commit":plan.source_commit,
                "limits":"reviewed scope and pinned measurement provenance; not independent rerunning of measurement tools"})
        }
        Err(why) => json!({"state":"blocked", "reason":why}),
    };
    // A missing mandatory gate never reaches live observation in admission mode.
    let observed: Vec<_> = if mode.offline || (mode.require_release_gates && !passed) {
        Vec::new()
    } else {
        [
            "/pull/run.json",
            "/autopilot.json",
            "/backtest/run.json",
            "/pull/recovery.json",
        ]
        .into_iter()
        .map(|route| observation(route, request(plan.address, route)))
        .collect()
    };
    let body = json!({"schema_version":2,"stage":stage.body,"mandatory_gates":mandatory,
        "observations":observed,"live_status_checked":!mode.offline && (!mode.require_release_gates || passed),
        "activation_admission":if passed {"mandatory_gates_passed_handoff_still_required"} else {"blocked_by_mandatory_gates"},
        "activation":"not_authorized_by_this_probe", "handoff":"requires coordinated active-task ownership and a fresh final check",
        "verification_work":"bounded byte scans and complete tree rescans; not constant-time or an atomic writer lease",
        "mutations_performed":false,"full_sweep_started":false});
    stage.retained.require_current()?;
    Ok(VerifiedReport {
        body,
        retained: stage.retained,
    })
}

fn main() -> std::process::ExitCode {
    let result = (|| {
        let (mode, path) = arguments(std::env::args_os().skip(1))?;
        let raw = files::bytes(&path, PLAN_BYTES)?;
        let plan: Plan = serde_json::from_slice(&raw)
            .map_err(|_| "plan schema is invalid, duplicated or unknown")?;
        let runtime = boolean_observation_budget::BooleanObservationBudget::load()?;
        let replay = boolean_search_budget::ReplayBudget::load()?;
        if replay.nodes() != positive(&plan.replay_nodes)? {
            return Err("preflight replay-node limit differs from the pinned launch plan".into());
        }
        if runtime.bytes() != positive(&plan.observation_bytes)? {
            return Err(
                "preflight environment observation limit differs from the pinned launch plan"
                    .into(),
            );
        }
        let body = preflight(&plan, mode, get)?;
        let blocked =
            mode.require_release_gates && body.body["mandatory_gates"]["state"] != "passed";
        Ok::<_, String>((body, blocked))
    })();
    match result {
        // Even clean observed slots do not constitute an atomic writer lease.
        // A successful stage check never authorizes automatic service replacement.
        Ok((body, blocked)) => {
            println!("{}", body.body);
            std::process::ExitCode::from(if blocked { 3 } else { 2 })
        }
        Err(why) => {
            println!(
                "{}",
                json!({"schema_version":1,"stage":{"state":"refused","reason":why},"activation":"refused","mutations_performed":false})
            );
            std::process::ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
#[path = "deployment-preflight-tests.rs"]
mod tests;

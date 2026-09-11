//! Declared AND/OR/NOT research launch through the existing audited worker slot.
use axum::http::{StatusCode, Uri};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::Path;
#[path = "booleanlaunch_work.rs"]
mod work;
use work::work_model;

/// The one native command exposed by this adapter.
pub const COMMAND: &str = "boolean-qualified-search-stored";
/// Maximum complete UTF-8 launch request; prefixes are never accepted.
pub const REQUEST_BYTES: usize = 16_384;
const MAX_POINTS: u64 = u64::MAX / 100;

/// Explicit browser declaration. Paths and ordinary sweep knobs are forbidden.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Asked {
    command: String,
    pub(crate) feed: String,
    pub(crate) symbols: Vec<String>,
    /// Omission is the original all-eight request, never an empty selection.
    #[serde(default = "all_timeframes")]
    timeframes: Vec<String>,
    pub(crate) from_year: u16,
    pub(crate) from_month: u8,
    pub(crate) to_year: u16,
    pub(crate) to_month: u8,
    later_from_year: u16,
    later_from_month: u8,
    later_to_year: u16,
    later_to_month: u8,
    bits: String,
    horizon_bars: String,
    max_points: String,
    batch_programs: String,
    node_allowance: String,
    batch_allowance: String,
    expected_policy_digest: String,
}

/// Typed status retained for an exact invocation, with journal-only completion.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Status {
    request: Asked,
    search_identity: Option<String>,
    exhausted: Option<bool>,
    completed_batches: Option<u64>,
}
impl Status {
    pub(crate) fn new(request: &Asked) -> Self {
        Self {
            request: request.clone(),
            search_identity: None,
            exhausted: None,
            completed_batches: None,
        }
    }
    pub(crate) fn fields(&self) -> Value {
        json!({
            "command": COMMAND, "symbols": self.request.symbols,
            "timeframes": self.request.timeframes,
            "later_from_year": self.request.later_from_year, "later_from_month": self.request.later_from_month,
            "later_to_year": self.request.later_to_year, "later_to_month": self.request.later_to_month,
            "bits": self.request.bits, "horizon_bars": self.request.horizon_bars,
            "max_points": self.request.max_points, "batch_programs": self.request.batch_programs,
            "node_allowance": self.request.node_allowance, "batch_allowance": self.request.batch_allowance,
            "expected_policy_digest": self.request.expected_policy_digest,
            "search_identity": self.search_identity, "exhausted": self.exhausted,
            "completed_batches": self.completed_batches.map(|count| count.to_string()),
        })
    }
    fn observe(&mut self, next: cli::boolean_search_launch::Progress) -> Result<(), String> {
        let identity = hex(&next.identity);
        if next.exhausted.is_some() != next.completed_batches.is_some() {
            return Err(
                "search outcome and acknowledged batch count must be observed together".into(),
            );
        }
        if let Some(prior) = self.completed_batches
            && next.completed_batches.is_none_or(|count| count < prior)
        {
            return Err("acknowledged search batch count cannot regress or disappear".into());
        }
        if self
            .search_identity
            .as_ref()
            .is_some_and(|prior| *prior != identity)
            || self.exhausted == Some(true) && next.exhausted != Some(true)
        {
            return Err("declared search identity or terminal evidence changed".into());
        }
        self.search_identity = Some(identity);
        self.exhausted = next.exhausted;
        self.completed_batches = next.completed_batches;
        Ok(())
    }
}

pub(crate) fn parse(body: &str) -> Result<Asked, String> {
    if body.len() > REQUEST_BYTES {
        return Err(format!(
            "Boolean launch request exceeds {REQUEST_BYTES} bytes; no prefix accepted"
        ));
    }
    let asked: Asked = serde_json::from_str(body)
        .map_err(|why| format!("Boolean launch schema refused: {why}"))?;
    if asked.command != COMMAND
        || asked.symbols.is_empty()
        || asked.symbols.len() > cli::boolean_search_launch::max_symbols()
        || asked.symbols.iter().any(|symbol| {
            symbol.is_empty()
                || symbol.len() > 64
                || symbol.trim() != symbol
                || symbol.contains(',')
                || symbol.chars().any(char::is_control)
        })
    {
        return Err("Boolean launch requires a bounded exact research symbol list".into());
    }
    if asked.expected_policy_digest.len() != 64
        || !asked
            .expected_policy_digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        || asked
            .expected_policy_digest
            .bytes()
            .all(|byte| byte == b'0')
    {
        return Err(
            "expected_policy_digest requires the displayed nonzero lowercase policy fingerprint"
                .into(),
        );
    }
    for (field, raw, maximum) in [
        ("horizon_bars", &asked.horizon_bars, u64::from(u32::MAX)),
        ("max_points", &asked.max_points, MAX_POINTS),
        ("batch_programs", &asked.batch_programs, u64::MAX),
        ("node_allowance", &asked.node_allowance, u64::MAX),
        ("batch_allowance", &asked.batch_allowance, u64::MAX),
    ] {
        if positive(raw).is_none_or(|value| value > maximum) {
            return Err(format!(
                "{field} requires a canonical positive decimal string within its native limit"
            ));
        }
    }
    if asked.bits != "all" {
        let mut previous = None;
        if asked.bits.len() > 4096 || asked.bits.is_empty() {
            return Err("Boolean alphabet exceeds its bounded protocol size".into());
        }
        for bit in asked.bits.split(',') {
            let parsed: u32 = bit
                .parse()
                .map_err(|_| "Boolean alphabet requires exact live bit IDs")?;
            if parsed.to_string() != bit || previous.is_some_and(|prior| parsed <= prior) {
                return Err("Boolean alphabet requires canonical increasing live bit IDs".into());
            }
            previous = Some(parsed);
        }
    }
    let timeframes: Vec<_> = asked.timeframes.iter().map(String::as_str).collect();
    if cli::boolean_campaign::RungScope::new(&timeframes)?.labels() != timeframes {
        return Err("Boolean timeframes require canonical increasing intraday order".into());
    }
    cli::boolean_search_launch::validate_for_rungs(
        &asked.arguments(Path::new("/not-opened-boolean-launch"))?,
        &timeframes,
    )?;
    Ok(asked)
}

impl Asked {
    pub(crate) fn window(&self) -> ((u16, u8), (u16, u8)) {
        (
            (self.from_year, self.from_month),
            (self.to_year, self.to_month),
        )
    }
    fn arguments(&self, root: &Path) -> Result<Vec<String>, String> {
        Ok(vec![
            self.feed.clone(),
            self.symbols
                .iter()
                .map(|symbol| symbol.strip_prefix("NSE-").unwrap_or(symbol))
                .collect::<Vec<_>>()
                .join(","),
            self.from_year.to_string(),
            self.from_month.to_string(),
            self.to_year.to_string(),
            self.to_month.to_string(),
            self.bits.clone(),
            self.horizon_bars.clone(),
            self.max_points.clone(),
            self.batch_programs.clone(),
            self.node_allowance.clone(),
            self.batch_allowance.clone(),
            root.to_str()
                .ok_or("Boolean launch store path is not UTF-8")?
                .to_owned(),
            self.later_from_year.to_string(),
            self.later_from_month.to_string(),
            self.later_to_year.to_string(),
            self.later_to_month.to_string(),
        ])
    }
}

pub(crate) fn prepare(
    asked: &Asked,
    root: &Path,
) -> Result<cli::boolean_search_launch::Admission, String> {
    let admitted = cli::boolean_search_launch::prepare_for_rungs(
        asked.arguments(root)?,
        root,
        &asked
            .timeframes
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
    )?;
    if asked.expected_policy_digest != hex(&admitted.configuration.policy_digest) {
        return Err("The displayed research policy changed before launch; refresh its configuration. No worker was accepted".into());
    }
    observation_limits(&admitted.configuration.strict)?;
    Ok(admitted)
}

fn observation_limits(
    strict: &cli::audited_range_command::StrictConfig,
) -> Result<(u64, u64), String> {
    let observation = crate::detail::BooleanObservationBudget::load()?.bytes();
    let replay = required("BRUTEX_BOOLEAN_SEARCH_REPLAY_NODES")?;
    let needed = strict
        .max_bytes()
        .checked_mul(4)
        .ok_or("Boolean observation byte bound overflows")?;
    if observation < needed || replay < strict.max_records() {
        return Err("Boolean dashboard observation/replay limits cannot read the admitted search; configure BRUTEX_BOOLEAN_OBSERVATION_BYTES and BRUTEX_BOOLEAN_SEARCH_REPLAY_NODES explicitly".into());
    }
    Ok((observation, replay))
}

pub(crate) fn conduct(
    asked: &Asked,
    admission: cli::boolean_search_launch::Admission,
    started: i64,
    attempt: u64,
    site: &crate::server::Loaded,
) -> crate::sweeprun::Progress {
    let mut progress = crate::sweeprun::Progress::started(
        &asked.feed,
        "SELECTED",
        asked.window().0,
        asked.window().1,
        None,
        started,
        attempt,
    )
    .of_kind(crate::sweeprun::Kind::Command);
    let mut status = Status::new(asked);
    let result = if asked
        .timeframes
        .iter()
        .map(String::as_str)
        .ne(admission.rungs().iter().copied())
    {
        Err(
            "Boolean timeframe selection changed after admission; no search work was started"
                .into(),
        )
    } else {
        admission.execute(&mut |next| {
            status.observe(next)?;
            let mut held = site
                .sweep
                .lock()
                .map_err(|_| "Boolean status lock is poisoned")?;
            let target = held
                .as_mut()
                .filter(|run| run.attempt == attempt && run.in_flight())
                .ok_or(
                    "Boolean invocation lost its exact active status; no further batch starts",
                )?;
            target.boolean_search = Some(Box::new(status.clone()));
            Ok(())
        })
    };
    progress.boolean_search = Some(Box::new(status));
    progress.finished_micros = Some(started);
    match result {
        Ok(report) => progress.report = Some(report),
        Err(why) => progress.refusal = Some(why),
    }
    progress
}

type Response = (
    StatusCode,
    [(axum::http::HeaderName, &'static str); 1],
    String,
);
/// Read server configuration only; this neither admits source history nor starts work.
pub async fn metadata(
    axum::extract::State(site): axum::extract::State<crate::server::Loaded>,
    uri: Uri,
) -> Response {
    let query = uri.query().map(str::to_owned);
    match crate::detail::run(move || metadata_value(&site.store_root, query.as_deref())).await {
        Ok(value) => response(StatusCode::OK, &value),
        Err(why) => response(
            StatusCode::SERVICE_UNAVAILABLE,
            &json!({"schema_version":1,
            "model":"boolean-qualified-search-launch","ready":false,"refusal":format!("bounded configuration read unavailable: {why:?}")}),
        ),
    }
}

fn metadata_value(root: &Path, query: Option<&str>) -> Value {
    let mut failures = Vec::new();
    let mut configured = serde_json::Map::new();
    for (field, setting, maximum) in [
        (
            "horizon_bars",
            "BRUTEX_BOOLEAN_HORIZON_BARS",
            u64::from(u32::MAX),
        ),
        ("max_points", "BRUTEX_BOOLEAN_MAX_POINTS", MAX_POINTS),
        ("batch_programs", "BRUTEX_BOOLEAN_BATCH_PROGRAMS", u64::MAX),
        ("node_allowance", "BRUTEX_BOOLEAN_NODE_ALLOWANCE", u64::MAX),
        (
            "batch_allowance",
            "BRUTEX_BOOLEAN_BATCH_ALLOWANCE",
            u64::MAX,
        ),
    ] {
        let raw = if field == "max_points" && query.is_some_and(|query| !query.is_empty()) {
            query
                .and_then(|query| query.strip_prefix("max_points="))
                .map(str::to_owned)
                .ok_or_else(|| "metadata accepts only one exact max_points query".to_owned())
                .map(Some)
        } else {
            optional(setting)
        };
        let value = match raw {
            Ok(Some(raw)) if positive(&raw).is_some_and(|value| value <= maximum) => Some(raw),
            Ok(None) => None,
            Ok(Some(_)) => {
                failures.push(format!("{setting} or explicit {field} is invalid"));
                None
            }
            Err(why) => {
                failures.push(why);
                None
            }
        };
        configured.insert(field.into(), value.map_or(Value::Null, Value::String));
    }
    let points = configured
        .get("max_points")
        .and_then(Value::as_str)
        .and_then(positive);
    let configuration = points
        .ok_or_else(|| {
            "Choose the visible max_points before resolving its research policy".to_owned()
        })
        .and_then(cli::boolean_search_launch::configuration);
    let policy = match configuration.as_ref() {
        Ok(config) => resolved_policy(config),
        Err(why) => {
            failures.push(why.clone());
            json!({"ready":false,"digest":null,"values":[],"refusal":why})
        }
    };
    let strict = cli::audited_range_command::StrictConfig::from_env();
    let (bytes, records) = match strict.as_ref() {
        Ok(strict) => (
            Some(strict.max_bytes().to_string()),
            Some(strict.max_records().to_string()),
        ),
        Err(why) => {
            failures.push(why.to_string());
            (None, None)
        }
    };
    let observation = match crate::detail::BooleanObservationBudget::load() {
        Ok(value) => Some(value.bytes().to_string()),
        Err(why) => {
            failures.push(why);
            None
        }
    };
    let replay = match required("BRUTEX_BOOLEAN_SEARCH_REPLAY_NODES") {
        Ok(value) => Some(value.to_string()),
        Err(why) => {
            failures.push(why);
            None
        }
    };
    if let Ok(strict) = strict.as_ref()
        && let Err(why) = observation_limits(strict)
    {
        failures.push(why);
    }
    if !root.is_absolute() || !root.is_dir() {
        failures.push("The serving store is not an existing absolute directory".into());
    }
    let ready = failures.is_empty();
    json!({"schema_version":1,"model":"boolean-qualified-search-launch","command":COMMAND,
        "selected_timeframes_supported":true,
        "index_consistency_policy":crate::booleanevidencejson::index_consistency_policy(cli::index_consistency::Policy::V1),
        "work_model":work_model(),
        "timeframes":cli::EVERY_RUNG,"ready":ready,"refusal":if ready {None} else {Some(failures.join("; "))},
        "limits":{"max_symbols":cli::boolean_search_launch::max_symbols(),"request_bytes":REQUEST_BYTES,
            "horizon_bars_max":u32::MAX.to_string(),"max_points_max":MAX_POINTS.to_string(),
            "checksum_max_bytes":bytes,"checksum_max_records":records,"observation_bytes":observation,"replay_nodes":replay},
        "configured":configured,"policy":policy})
}
fn all_timeframes() -> Vec<String> {
    cli::EVERY_RUNG
        .iter()
        .map(|rung| (*rung).to_owned())
        .collect()
}
fn resolved_policy(config: &cli::boolean_search_launch::Configuration) -> Value {
    let mut policy = crate::booleanevidencejson::search_policy(&config.policy);
    if let Some(fields) = policy.as_object_mut() {
        fields.insert("ready".into(), Value::Bool(true));
        fields.insert("refusal".into(), Value::Null);
    }
    policy
}
fn optional(name: &str) -> Result<Option<String>, String> {
    std::env::var_os(name)
        .map(|raw| {
            raw.into_string()
                .map_err(|_| format!("{name} must be UTF-8"))
        })
        .transpose()
}
fn required(name: &str) -> Result<u64, String> {
    optional(name)?
        .as_deref()
        .and_then(positive)
        .ok_or_else(|| format!("{name} requires an explicit canonical positive u64 value"))
}
fn positive(raw: &str) -> Option<u64> {
    if raw.is_empty()
        || raw.len() > 20
        || raw.starts_with('0')
        || !raw.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    raw.parse().ok()
}
fn hex(digest: &[u8; 32]) -> String {
    use std::fmt::Write as _;
    digest
        .iter()
        .fold(String::with_capacity(64), |mut out, byte| {
            let _ = write!(out, "{byte:02x}");
            out
        })
}
fn response(status: StatusCode, value: &Value) -> Response {
    (
        status,
        [(
            axum::http::header::CONTENT_TYPE,
            "application/json; charset=utf-8",
        )],
        value.to_string(),
    )
}

#[cfg(test)]
#[path = "booleanlaunch_tests.rs"]
mod tests;

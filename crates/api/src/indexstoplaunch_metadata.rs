//! Configuration-only preview of the exact native single-stop launch policy.
use super::{MAX_LOSS_POINTS, REQUEST_BYTES, hex, positive};
use serde_json::{Value, json};
use std::path::Path;

const DEFAULTS: [(&str, &str); 4] = [
    ("max_loss_points", "BRUTEX_INDEX_STOP_MAX_LOSS_POINTS"),
    ("batch_programs", "BRUTEX_INDEX_STOP_BATCH_PROGRAMS"),
    ("node_allowance", "BRUTEX_INDEX_STOP_NODE_ALLOWANCE"),
    ("batch_allowance", "BRUTEX_INDEX_STOP_BATCH_ALLOWANCE"),
];

pub(super) fn value(root: &Path, query: Option<&str>) -> Value {
    let mut failures = Vec::new();
    let configured = match configured(query, optional) {
        Ok(configured) => configured,
        Err(why) => {
            crate::booleanlaunch::note_metadata_refusal(&mut failures, why);
            serde_json::Map::new()
        }
    };
    for (field, setting) in DEFAULTS {
        if configured.get(field).is_none_or(Value::is_null) {
            crate::booleanlaunch::note_metadata_refusal(
                &mut failures,
                format!("{field} is not configured; set {setting}"),
            );
        }
    }
    let number = |field| {
        configured
            .get(field)
            .and_then(Value::as_str)
            .and_then(positive)
    };
    let config = number("max_loss_points")
        .zip(number("batch_programs"))
        .ok_or("Choose a research loss limit and batch size before resolving the policy".to_owned())
        .and_then(|(points, programs)| cli::index_stop_search::configuration(points, programs));
    let (policy, limits, parallel, procedure) = match &config {
        Ok(config) => {
            if number("node_allowance").is_some_and(|nodes| nodes > config.capture.records) {
                crate::booleanlaunch::note_metadata_refusal(
                    &mut failures,
                    "node_allowance exceeds the admitted native record bound".into(),
                );
            }
            if let Err(why) = observation_limits(config) {
                crate::booleanlaunch::note_metadata_refusal(&mut failures, why);
            }
            let mut policy = crate::booleanevidencejson::search_policy(&config.policy);
            if let Some(fields) = policy.as_object_mut() {
                fields.insert("ready".into(), Value::Bool(true));
                fields.insert("refusal".into(), Value::Null);
            }
            (
                policy,
                json!({"checksum_bytes":config.strict.max_bytes().to_string(),
                "checksum_records":config.strict.max_records().to_string(),
                "capture_programs":config.capture.programs.to_string(),
                "capture_records":config.capture.records.to_string(),"capture_bytes":config.capture.bytes.to_string(),
                "qualification_bytes":config.qualification.bytes.to_string(),
                "qualification_memory_bytes":config.qualification.memory_bytes.to_string(),
                "qualification_replay_bootstrap_work":config.qualification.bootstrap_work.to_string(),
                "qualification_replay_split_work":config.qualification.split_work.to_string(),
                "qualification_replay_memory_bytes":config.qualification.memory_bytes.to_string(),
                "history_bytes":config.history_bytes.to_string(),"replay_nodes":config.replay_nodes.to_string()}),
                Some(config.workers),
                json!({"draws":config.procedure.draws().to_string(),
                    "seed":config.procedure.seed().to_string(),"block_length":config.procedure.block_length().to_string()}),
            )
        }
        Err(why) => {
            crate::booleanlaunch::note_metadata_refusal(&mut failures, why.clone());
            (
                json!({"ready":false,"digest":null,"values":[],"refusal":why}),
                Value::Null,
                None,
                Value::Null,
            )
        }
    };
    if !root.is_absolute() || !root.is_dir() {
        crate::booleanlaunch::note_metadata_refusal(
            &mut failures,
            "The serving store is not an existing absolute directory".into(),
        );
    }
    let ready = failures.is_empty();
    json!({"schema_version":1,"model":"index-stop-qualified-search-launch","command":super::COMMAND,
        "ready":ready,"readiness_scope":"configuration-only; source admission occurs in the audited worker",
        "refusal":if ready {None} else {Some(failures.join("; "))},
        "selected_timeframes_supported":true,"indices":["NSE-NIFTY","NSE-BANKNIFTY"],"timeframes":cli::EVERY_RUNG,
        "execution_policy":"signal_candle_stop_v1",
        "execution_policy_digest":hex(&cli::index_stop_store::Policy::V1.digest()),
        "execution_rules":{"signal":"Completed signal candle","long_stop":"Signal candle low",
            "short_stop":"Signal candle high","entry":"Immediately following printed one-minute open",
            "gap_invalid_entry":"Skip entry at or beyond the stop",
            "forced_exit":"15:10 IST using the unique accepted 15:09 close",
            "exits":["risk stop","15:10 IST"],"directions":["long","short"],
            "readings":["pessimistic","optimistic"],"costs_included":false,
            "maximum_open_positions_per_setting":1},
        "index_consistency_policy":crate::booleanevidencejson::index_consistency_policy(cli::index_consistency::INDEX_STOP),
        "configured":configured,"policy":policy,"procedure":procedure,
        "limits":{"request_bytes":REQUEST_BYTES,"max_loss_points_max":MAX_LOSS_POINTS.to_string(),"physical":limits},
        "field_help":{"max_loss_points":"Institutional research loss limit. It does not set the signal-candle stop or investment size.",
            "batch_allowance":"Work allowed in this invocation; reaching it pauses the exact search, without claiming all combinations are finished."},
        "work_model":{"parallel_timeframe_workers_max":parallel,
            "parallelism":"Independent selected timeframes, capped by configured workers and available CPU parallelism",
            "within_timeframe":"Each complete expression is evaluated long and short on original training and fixed later days",
            "total_runtime":"Depends on admitted history, expression enumeration, held minutes and statistical work",
            "estimated_seconds":null,"estimate_status":"unmeasured"}})
}

pub(super) fn observation_limits(
    config: &cli::index_stop_search::Configuration,
) -> Result<(), String> {
    let available = crate::detail::BooleanObservationBudget::load()?.bytes();
    let needed = config.capture.bytes.max(config.qualification.bytes);
    if available < needed {
        return Err(format!(
            "Saved single-stop comparison needs observation admission of at least {needed} bytes; BRUTEX_BOOLEAN_OBSERVATION_BYTES currently admits {available}"
        ));
    }
    let replay = crate::indexstopqualificationjson::replay_bounds(available)?;
    if config.qualification.bootstrap_work > replay.bootstrap_work
        || config.qualification.split_work > replay.split_work
        || config.qualification.memory_bytes > replay.memory_bytes
    {
        return Err(format!(
            "Saved single-stop comparison must fit independent cold numerical replay admission before launch: bootstrap work {}, CSCV work {}, estimated numeric-memory bytes {}; these are work/byte bounds, not latency or RSS guarantees",
            replay.bootstrap_work, replay.split_work, replay.memory_bytes
        ));
    }
    Ok(())
}

fn configured(
    query: Option<&str>,
    read: impl Fn(&str) -> Result<Option<String>, String>,
) -> Result<serde_json::Map<String, Value>, String> {
    let mut overrides = serde_json::Map::new();
    if let Some(query) = query.filter(|value| !value.is_empty()) {
        crate::detail::query_is_bounded(query)?;
        for pair in query.split('&') {
            let (key, raw) = pair
                .split_once('=')
                .ok_or("single-stop preview requires key=value")?;
            if !matches!(key, "max_loss_points" | "batch_programs")
                || positive(raw).is_none()
                || overrides
                    .insert(key.into(), Value::String(raw.into()))
                    .is_some()
            {
                return Err("single-stop preview accepts each of max_loss_points and batch_programs once as canonical positive decimals".into());
            }
        }
    }
    let mut configured = serde_json::Map::new();
    for (field, setting) in DEFAULTS {
        let value = if let Some(raw) = overrides.remove(field) {
            Some(raw)
        } else {
            read(setting)?.map(Value::String)
        };
        if let Some(raw) = &value {
            let number = raw
                .as_str()
                .and_then(positive)
                .ok_or_else(|| format!("{setting} must be a canonical positive decimal"))?;
            if field == "max_loss_points" && number > MAX_LOSS_POINTS {
                return Err("max_loss_points cannot be represented in integer paisa".into());
            }
        }
        configured.insert(field.into(), value.unwrap_or(Value::Null));
    }
    Ok(configured)
}
fn optional(name: &str) -> Result<Option<String>, String> {
    std::env::var_os(name)
        .map(|raw| {
            raw.into_string()
                .map_err(|_| format!("{name} must be UTF-8"))
        })
        .transpose()
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "private preview fixture asserts exact parsing"
)]
mod tests {
    use super::*;
    #[test]
    fn preview_changes_only_named_values_without_hidden_defaults_or_duplicate_overrides() {
        let values = configured(Some("batch_programs=4&max_loss_points=9"), |_| Ok(None))
            .expect("pure preview");
        assert_eq!(values.get("max_loss_points"), Some(&json!("9")));
        assert_eq!(values.get("batch_programs"), Some(&json!("4")));
        assert_eq!(values.get("node_allowance"), Some(&Value::Null));
        for invalid in [
            "max_points=5",
            "max_loss_points=0",
            "max_loss_points=01",
            "batch_programs=4&batch_programs=4",
            "node_allowance=5",
            "max_loss_points=5&",
            "=1",
            "max_loss_points=184467440737095517",
            "batch_programs=18446744073709551616",
        ] {
            assert!(
                configured(Some(invalid), |_| Ok(None)).is_err(),
                "{invalid}"
            );
        }
        assert!(configured(None, |_| Ok(Some("01".into()))).is_err());
        assert!(configured(None, |_| Err("non-UTF8 environment".into())).is_err());
    }
}

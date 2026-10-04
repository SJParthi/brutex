//! `/selection-v6.json`: the committed Selection V6 records `ledger-v6` wrote.
//! D-1578, audit-20261003 gaps-10.
//!
//! # What it serves
//!
//! Every intraday rung's `ROOT/selection/<rung>/global-selection-v6.bin`
//! under the dashboard's store root, decoded by
//! [`cli::read_stored_selection_v6`]: each sealed block's identity, its
//! retained source receipts, its family envelopes, its ranking counters and
//! its stored Top-25 (whose first ten are the Top-10). A rung with no file is
//! named absent with its path, and a rung whose file cannot be read whole is
//! named refused with the reason; neither is shown as an empty winner list.
//!
//! # What it is not
//!
//! It is a read of sealed stored records, not a fresh Selection V6
//! capability: the upstream Execution V4 / Population V6 chain is not
//! re-authenticated here, and the payload's `authority` and `scope` say so.
//! It starts nothing and writes nothing. The dashboard's `BRUTEX_STORE` must
//! be the `ROOT` `ledger-v6` was given; no other folder is searched.
//!
//! # Equities
//!
//! Selection V6 holds NIFTY and BANKNIFTY only, and `CLAUDE.md` §1 forbids
//! any equity result entering it. `family=RELIANCE` is refused with that
//! sentence (400), a stored block naming any other family code is refused by
//! the decoder, and every payload carries the sentence as `equities`.

use axum::http::{StatusCode, Uri};
use serde_json::{Value, json};
use std::path::Path;

#[cfg(test)]
#[path = "selectionv6json_tests.rs"]
mod tests;

type Response = (
    StatusCode,
    [(axum::http::HeaderName, &'static str); 1],
    String,
);

/// Blocks shown per rung. A file holding more is refused whole by the reader,
/// never cut to a prefix.
pub const MAX_RECORDS_PER_RUNG: u64 = 64;

const SCOPE: &str = "Sealed Selection V6 blocks as ledger-v6 committed them: version, \
     identity and completion seal verified on every read. The upstream Execution V4 and \
     Population V6 receipts are named, not re-authenticated, so this is a stored record \
     and not a fresh Selection V6 capability. Per-rung selection does not prove that \
     selected trades never overlap; that is Global Replay V4. Index families only.";

fn response(status: StatusCode, body: &Value) -> Response {
    (
        status,
        [(axum::http::header::CONTENT_TYPE, "application/json")],
        body.to_string(),
    )
}

fn refused(status: StatusCode, why: &str) -> Response {
    response(
        status,
        &json!({"schema_version":1,"status":"refused","refusal":why,"rows":[],"rungs":[],
            "equities":cli::SELECTION_V6_EQUITY_REFUSAL}),
    )
}

/// The one selector this route takes: an optional family.
///
/// # Errors
///
/// An unknown or repeated key, an empty value, or a family that is not one
/// of the two, with the equity sentence for an equity.
pub(crate) fn asked_family(query: &str) -> Result<Option<&'static str>, String> {
    crate::detail::query_is_bounded(query)?;
    if query.is_empty() {
        return Ok(None);
    }
    let mut family = None;
    for pair in query.split('&') {
        let (key, value) = pair
            .split_once('=')
            .ok_or("selection query requires key=value")?;
        if key != "family" || value.is_empty() || family.is_some() {
            return Err("the only selector is one non-empty family=NIFTY|BANKNIFTY".to_owned());
        }
        family = Some(cli::selection_v6_family(&crate::server::param(
            query, "family",
        ))?);
    }
    Ok(family)
}

/// Reads the committed Selection V6 records under the dashboard's store root.
pub async fn selection_v6_json(uri: Uri) -> Response {
    let family = match asked_family(uri.query().unwrap_or_default()) {
        Ok(family) => family,
        Err(why) => return refused(StatusCode::BAD_REQUEST, &why),
    };
    let root = crate::server::store_dir();
    match crate::detail::run(move || root.map(|root| render(&root, family))).await {
        Ok(Ok(body)) => {
            let reply = response(StatusCode::OK, &body);
            if reply.2.len() <= crate::detail::MAX_RESPONSE_BYTES {
                reply
            } else {
                refused(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "Selection V6 response exceeds its byte ceiling; no prefix returned",
                )
            }
        }
        Err(crate::detail::RunError::Saturated) => refused(
            StatusCode::TOO_MANY_REQUESTS,
            "Selection V6 read capacity full; nothing queued",
        ),
        Ok(Err(why)) | Err(crate::detail::RunError::Join(why)) => {
            refused(StatusCode::SERVICE_UNAVAILABLE, &why)
        }
    }
}

/// The whole payload for `root`, optionally narrowed to one family's winners.
pub(crate) fn render(root: &Path, family: Option<&'static str>) -> Value {
    let rungs: Vec<Value> = cli::read_stored_selection_v6(root, MAX_RECORDS_PER_RUNG)
        .into_iter()
        .map(|(rung, read)| match read {
            cli::StoredSelectionV6Rung::Absent(path) => {
                json!({"rung":rung,"status":"absent","path":path,"refusal":null,"records":[]})
            }
            cli::StoredSelectionV6Rung::Refused(why) => {
                json!({"rung":rung,"status":"refused","path":null,"refusal":why,"records":[]})
            }
            cli::StoredSelectionV6Rung::Records(records) => {
                json!({"rung":rung,"status":"saved",
                "path":null,"refusal":null,
                "records":records.iter().map(|record| project(record, family)).collect::<Vec<_>>()})
            }
        })
        .collect();
    let saved = rungs
        .iter()
        .any(|rung| rung.get("status") == Some(&json!("saved")));
    json!({"schema_version":1,
        "status": if saved { "saved" } else { "absent" },
        "authority":"sealed-stored-selection-v6-record",
        "root":root.display().to_string(),
        "family":family,
        "scope":SCOPE,
        "equities":cli::SELECTION_V6_EQUITY_REFUSAL,
        "rungs":rungs,
        "refusal":null})
}

fn ratio(value: Option<u64>) -> Value {
    value.map_or(Value::Null, |v| json!(v.to_string()))
}

/// One record, its winners narrowed to `family` when one was asked for.
pub(crate) fn project(record: &cli::StoredSelectionV6Record, family: Option<&str>) -> Value {
    let hex = crate::server::hex32;
    let winners: Vec<Value> = record
        .winners
        .iter()
        .filter(|winner| family.is_none_or(|family| winner.family == family))
        .map(|w| {
            json!({"rank":w.rank,"top_ten":w.rank < 10,"family":w.family,"direction":w.direction,
                "strategy":hex(w.strategy_digest),"disposition":hex(w.disposition_id),
                "selected_exit":hex(w.selected_exit_digest),
                "global_sequence":w.global_sequence.to_string(),
                "family_sequence":w.family_sequence.to_string(),"score":w.score.to_string(),
                "mask_words":w.mask_words.iter().map(u64::to_string).collect::<Vec<_>>(),
                "drawdown":w.drawdown.to_string(),"worst_loss":w.worst_loss.to_string(),
                "losing_rate_ppm":w.losing_rate_ppm.to_string(),
                "losing_trades":w.losing_trades.to_string(),
                "winning_trades":w.winning_trades.to_string(),
                "win_rate_ppm":w.win_rate_ppm.to_string(),"average_win":w.average_win.to_string(),
                "average_loss":w.average_loss.to_string(),"assurance_ppm":w.assurance_ppm.to_string(),
                "pessimistic_profit":w.pessimistic_profit.to_string(),
                "loss_ratio_ppm":ratio(w.loss_ratio_ppm),
                "reward_to_risk_ppm":ratio(w.reward_to_risk_ppm)})
        })
        .collect();
    json!({"identity":hex(record.identity),"rung_seconds":record.rung_seconds.to_string(),
        "horizon_bars":record.horizon_bars.to_string(),"population":hex(record.population_id),
        "execution_completion":hex(record.execution_completion_id),
        "policy":hex(record.policy_digest),"rows":record.row_count.to_string(),
        "authorized":record.authorized_count.to_string(),
        "policy_refused":record.policy_refused_count.to_string(),
        "families":record.families.iter().map(|f| json!({"family":f.family,"terminal":f.terminal,
            "candidates":f.candidate_count.to_string(),"evaluated":f.evaluated_count.to_string(),
            "decisions":f.decision_count.to_string()})).collect::<Vec<_>>(),
        "considered":record.considered.to_string(),"admitted":record.admitted.to_string(),
        "refused":record.refused.to_string(),"unmeasured":record.unmeasured.to_string(),
        "winner_count":record.winners.len(),"winners":winners})
}

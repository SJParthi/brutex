//! Durable sweep invocation and HTTP boundary audit, with bounded read exposure.
//! Query/body/header values never enter this journal. A handler completing is
//! not proof that the engine task it launched completed or passed admission.

use axum::http::StatusCode;
use axum::response::IntoResponse as _;
use cli::operation_audit::{self as journal, Origin, Phase, Record};

type Response = (
    StatusCode,
    [(axum::http::HeaderName, &'static str); 1],
    String,
);
fn headers() -> [(axum::http::HeaderName, &'static str); 1] {
    [(
        axum::http::header::CONTENT_TYPE,
        "application/json; charset=utf-8",
    )]
}

fn failure(why: &str, handler_completed: bool) -> Response {
    (StatusCode::SERVICE_UNAVAILABLE, headers(), serde_json::json!({
        "schema_version": 1, "refusal": why,
        "code": "invocation_audit_unavailable", "handler_completed": handler_completed,
        "why": if handler_completed { "The handler already ran. Its work may still be running or saved; inspect the exact invocation before retrying a write." } else { "The handler was not dispatched because its required audit start was unavailable." }
    }).to_string())
}

fn read_failure(why: &str, busy: bool) -> Response {
    (
        if busy {
            StatusCode::TOO_MANY_REQUESTS
        } else {
            StatusCode::SERVICE_UNAVAILABLE
        },
        headers(),
        serde_json::json!({
            "schema_version": 1, "refusal": why,
            "code": "invocation_audit_read_unavailable",
            "why": "No audit snapshot was published. Reading this endpoint does not start an engine task or append an audit record."
        })
        .to_string(),
    )
}

/// Every route the invocation journal records, and the only names that enter
/// its labels. Unknown paths, assets, queries, request bodies and headers are
/// never copied into records.
///
/// **One list, read by everything that must agree.** [`audited_route`] answers
/// from it, the server's admission layer refuses a cross-site read of any path
/// it holds before [`note_request`] can journal it (D-0687), and the tests read
/// it to prove each entry is registered, journaled and refused cross-site. It
/// used to be a `match` with 21 arms and a test that named 10 of them, so
/// deleting the `/frontier.json` arm dropped both its audit record and its
/// cross-site refusal and every `api` test still passed. The length is in the
/// type: removing an entry without changing the count does not compile.
/// D-1445.
pub(crate) const AUDITED: [&str; 22] = [
    "/backtest/run",
    "/backtest/descend",
    "/engine/command",
    "/engine/boolean-launch.json",
    "/backtest.json",
    "/backtest/run.json",
    "/trades.json",
    "/frontier.json",
    "/candidate-trades.json",
    "/sweep-evidence.json",
    "/boolean-candidates.json",
    "/boolean-statistics.json",
    "/boolean-admission.json",
    "/boolean-qualification.json",
    "/boolean-qualified-search.json",
    "/boolean-campaign.json",
    "/boolean-qualified-campaign.json",
    "/boolean-oos.json",
    "/selection-v6.json",
    "/expression-search.json",
    "/engine/top.json",
    "/live.json",
];

/// The [`AUDITED`] entry equal to `path`, or `None`.
///
/// At most 22 whole-string comparisons, a bound fixed by the type of
/// [`AUDITED`] and independent of the request; proven by
/// `crate::operation_audit::tests::every_registered_route_is_audited_or_exempt_by_name`.
pub(crate) fn audited_route(path: &str) -> Option<&'static str> {
    AUDITED.iter().copied().find(|route| *route == path)
}

fn public_method(method: &axum::http::Method) -> &'static str {
    match method.as_str() {
        "GET" => "GET",
        "POST" => "POST",
        "HEAD" => "HEAD",
        "PUT" => "PUT",
        "PATCH" => "PATCH",
        "DELETE" => "DELETE",
        "OPTIONS" => "OPTIONS",
        _ => "OTHER",
    }
}

/// Required start before dispatch and terminal after response construction.
/// Blocking reads/writes use the shared bounded worker admission. A failed
/// terminal cannot undo an already-dispatched handler and explicitly says so.
pub async fn note_request(
    axum::extract::State(site): axum::extract::State<crate::server::Loaded>,
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let Some(route) = audited_route(request.uri().path()) else {
        return next.run(request).await;
    };
    let label = format!("{} {route}", public_method(request.method()));
    let root = site.store_root.clone();
    request_audited(root, label, next.run(request)).await
}

/// [`note_request`]'s journal around one handler, with the route label already
/// chosen.
///
/// `pub(crate)` so a sweep route's own tests can drive their handler through
/// the journal the production router wraps it in, and assert what the journal
/// holds after a refusal: a refusal sentence that says what was written must
/// be checked against the store as the operator's server leaves it, not as a
/// bare handler call does. D-0695.
pub(crate) async fn request_audited(
    root: std::path::PathBuf,
    label: String,
    handler: impl std::future::Future<Output = axum::response::Response>,
) -> axum::response::Response {
    let attempt = match crate::detail::run(move || journal::begin(&root, Origin::Http, &label))
        .await
    {
        Ok(Ok(attempt)) => attempt,
        Ok(Err(why)) => {
            let mut refusal = failure(&why, false);
            if journal::is_busy(&why) {
                refusal.0 = StatusCode::TOO_MANY_REQUESTS;
            }
            return refusal.into_response();
        }
        Err(crate::detail::RunError::Saturated) => {
            let mut refusal = failure(
                "bounded request audit capacity is full; no handler was dispatched and no work was queued",
                false,
            );
            refusal.0 = StatusCode::TOO_MANY_REQUESTS;
            return refusal.into_response();
        }
        Err(why) => {
            return failure(
                &format!("bounded request audit could not start: {why:?}"),
                false,
            )
            .into_response();
        }
    };
    let id = attempt.id();
    let mut response = handler.await;
    let status = response.status();
    let phase = if status.is_server_error() {
        Phase::Failed
    } else if status.is_client_error() {
        Phase::Refused
    } else {
        Phase::Completed
    };
    // THE TERMINAL IS OWED, NOT ADMITTED. The handler has already run, so a
    // full detail pool must not refuse this write: refusing dropped the armed
    // attempt, whose `Drop` then wrote `Cancelled`/0 synchronously on this
    // Tokio worker and replaced the handler's real answer with a 503. The owed
    // slot still counts against new detail work. D-1445.
    match crate::detail::run_owed(move || {
        let mut attempt = attempt;
        attempt.finish(phase, status.as_u16())
    })
    .await
    {
        Ok(Ok(())) => {
            if let Ok(value) = axum::http::HeaderValue::from_str(&id.to_string()) {
                response
                    .headers_mut()
                    .insert("x-brutex-request-audit", value);
            }
            response
        }
        Ok(Err(why)) => failure(&why, true).into_response(),
        Err(why) => failure(
            &format!("bounded terminal audit could not settle: {why:?}"),
            true,
        )
        .into_response(),
    }
}

fn record_json(record: &Record) -> serde_json::Value {
    serde_json::json!({
        "invocation": record.id.to_string(), "origin": record.origin.label(),
        "operation": record.label, "status": record.phase.label(),
        "terminal": record.phase.terminal(), "at_millis": record.at_millis.to_string(),
        "elapsed_micros": record.elapsed_micros.to_string(),
        "completed_boundaries": record.completed_boundaries.to_string(),
        "total_boundaries": null, "response_status": if record.response_status == 0 { None } else { Some(record.response_status) },
        "meaning": "Invocation outcome only; not a strategy identity, full sweep completion or institutional admission.",
        "integrity_scope": "Indexed start and first/last invocation records only; saved middle progress records were not rescanned.",
    })
}

enum Asked {
    Exact(u64),
    Page { before: Option<u64>, limit: usize },
}
fn integer(raw: &str) -> Result<u64, String> {
    if raw.is_empty() || raw.starts_with('0') || !raw.bytes().all(|b| b.is_ascii_digit()) {
        return Err("audit IDs and limits require canonical positive decimal integers".to_owned());
    }
    raw.parse()
        .map_err(|_| "audit integer overflows u64".to_owned())
}
fn parse(query: &str) -> Result<Asked, String> {
    if query.len() > 96 {
        return Err("audit query exceeds 96 bytes".to_owned());
    }
    if !query.is_empty() && query.split('&').any(str::is_empty) {
        return Err("empty audit query fields are refused".to_owned());
    }
    let (mut id, mut before, mut limit) = (None, None, None);
    for pair in query.split('&').filter(|pair| !pair.is_empty()) {
        let (key, raw) = pair
            .split_once('=')
            .ok_or("audit query requires key=value pairs")?;
        let slot = match key {
            "invocation" => &mut id,
            "before" => &mut before,
            "limit" => &mut limit,
            _ => return Err("unknown audit query field".to_owned()),
        };
        if slot.is_some() {
            return Err("duplicate audit query field".to_owned());
        }
        *slot = Some(integer(raw)?);
    }
    if let Some(id) = id {
        if before.is_some() || limit.is_some() {
            return Err("an exact invocation cannot also request a page".to_owned());
        }
        return Ok(Asked::Exact(id));
    }
    let limit = limit
        .map_or(Ok(journal::MAX_PAGE), usize::try_from)
        .map_err(|why| why.to_string())?;
    if limit > journal::MAX_PAGE {
        return Err("audit page exceeds 32 rows".to_owned());
    }
    Ok(Asked::Page { before, limit })
}

/// Read persistent invocation history, including after a process restart.
/// The audit reader itself is excluded from request auditing, so a failed
/// write path cannot prevent inspection or create recursive audit traffic.
pub async fn audit_json(
    axum::extract::State(site): axum::extract::State<crate::server::Loaded>,
    uri: axum::http::Uri,
) -> Response {
    let asked = match parse(uri.query().unwrap_or_default()) {
        Ok(asked) => asked,
        Err(why) => {
            return (
                StatusCode::BAD_REQUEST,
                headers(),
                serde_json::json!({"refusal":why}).to_string(),
            );
        }
    };
    let root = site.store_root.clone();
    match crate::detail::run(move || render(&root, &asked)).await {
        Ok(Ok(body)) => (StatusCode::OK, headers(), body),
        Ok(Err(why)) => read_failure(&why, journal::is_busy(&why)),
        Err(why) => read_failure(
            &format!("audit read unavailable: {why:?}"),
            why == crate::detail::RunError::Saturated,
        ),
    }
}

fn render(root: &std::path::Path, asked: &Asked) -> Result<String, String> {
    let (rows, next) = match asked {
        Asked::Exact(id) => (
            journal::read(root, *id)?.into_iter().collect::<Vec<_>>(),
            None,
        ),
        Asked::Page { before, limit } => {
            let rows = journal::page(root, *before, *limit)?;
            let next = rows
                .last()
                .map(|row| row.id)
                .filter(|id| *id > journal::ID_BASE + 1);
            (rows, next)
        }
    };
    Ok(serde_json::json!({
        "schema_version": 1, "records": rows.iter().map(record_json).collect::<Vec<_>>(),
        "next_before": next.map(|id| id.to_string()), "refusal": null,
        "claim": "Fixed-stride durable invocation history. Unconfirmed is not proof that a process is running; completion is not strategy admission."
    }).to_string())
}

/// Fallback for a selected engine attempt no longer retained in this process.
/// Never substitutes an HTTP request or a different attempt with the same label.
pub(crate) fn persisted_status(root: &std::path::Path, id: u64) -> Result<Option<String>, String> {
    let Some(record) = journal::read(root, id)? else {
        return Ok(None);
    };
    if record.origin != Origin::Browser {
        return Err("requested attempt is not a browser engine invocation".to_owned());
    }
    let terminal = record.phase.terminal();
    let refusal = matches!(record.phase, Phase::Refused | Phase::Failed | Phase::Cancelled).then(|| format!("Saved invocation ended {}. Its computation evidence and detailed logs remain separate.", record.phase.label()));
    Ok(Some(serde_json::json!({"running": {
        "where": "browser", "status": record.phase.label(), "attempt": id,
        "attempt_key": id.to_string(),
        "in_flight": false, "terminal": terminal, "durable": true,
        "report": null, "refusal": refusal, "audit": record_json(&record),
        "why": "The in-process worker/report is unavailable. This is exact persisted invocation evidence; no current liveness or strategy admission is inferred."
    }}).to_string()))
}

#[cfg(test)]
#[path = "operation_audit_tests.rs"]
pub(crate) mod tests;

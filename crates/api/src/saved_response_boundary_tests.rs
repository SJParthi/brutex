//! Public read-boundary probes. Hand-written invalid queries must finish before
//! any background work can start, so these tests deliberately have no runtime.
//! Seventeen handlers are probed (D-0679, D-0689). Fifteen share the strict
//! saved-selector grammar and the schema-1 refusal envelope. `/trades.json` and
//! `/frontier.json` keep their own detail envelope and grammar, and since
//! D-0689 parse their selector before detail admission rather than inside it.
use axum::http::{HeaderName, StatusCode, Uri, header::CONTENT_TYPE};
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

type Reply = (StatusCode, [(HeaderName, &'static str); 1], String);
type Handler = fn(Uri) -> Pin<Box<dyn Future<Output = Reply>>>;

const ID: &str = "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";

/// Which refusal envelope and selector grammar a handler answers under.
#[derive(Clone, Copy)]
enum Contract {
    /// The strict saved-selector grammar; a schema-1 envelope whose `rows` is
    /// an empty array.
    Saved,
    /// `/trades.json` or `/frontier.json`: the detail envelope, whose row list
    /// under this key is `null` on a refusal rather than an empty list.
    Detail(&'static str),
}

fn handlers() -> [(&'static str, Contract, Handler); 17] {
    [
        ("campaign", Contract::Saved, |uri| {
            Box::pin(crate::booleancampaignjson::campaign_json(uri))
        }),
        ("qualified campaign", Contract::Saved, |uri| {
            Box::pin(crate::booleancampaignjson::qualified_campaign_json(uri))
        }),
        ("statistics", Contract::Saved, |uri| {
            Box::pin(crate::booleanevidencejson::statistics_json(uri))
        }),
        ("admission", Contract::Saved, |uri| {
            Box::pin(crate::booleanevidencejson::admission_json(uri))
        }),
        ("qualification", Contract::Saved, |uri| {
            Box::pin(crate::booleanevidencejson::qualification_json(uri))
        }),
        ("Boolean catalog", Contract::Saved, |uri| {
            Box::pin(crate::booleanjson::boolean_json(uri))
        }),
        ("later comparison", Contract::Saved, |uri| {
            Box::pin(crate::booleanoosjson::later_json(uri))
        }),
        ("qualified search", Contract::Saved, |uri| {
            Box::pin(crate::booleansearchjson::search_json(uri))
        }),
        ("expression search", Contract::Saved, |uri| {
            Box::pin(crate::expressionsearchjson::expression_search_json(uri))
        }),
        ("index stop", Contract::Saved, |uri| {
            Box::pin(crate::indexstopjson::index_stop_json(uri))
        }),
        ("index candles", Contract::Saved, |uri| {
            Box::pin(crate::indexstopcandlesjson::index_stop_candles_json(uri))
        }),
        ("index qualification", Contract::Saved, |uri| {
            Box::pin(crate::indexstopqualificationjson::index_stop_qualification_json(uri))
        }),
        ("index ranking", Contract::Saved, |uri| {
            Box::pin(crate::indexstoprankingjson::index_stop_ranking_json(uri))
        }),
        ("index VIX", Contract::Saved, |uri| {
            Box::pin(crate::indexstopvixjson::index_stop_vix_json(uri))
        }),
        ("candidate trades", Contract::Saved, |uri| {
            Box::pin(crate::candidatejson::candidate_json(uri))
        }),
        ("chosen trades", Contract::Detail("trades"), |uri| {
            Box::pin(crate::trades::trades_json(uri))
        }),
        ("frontier", Contract::Detail("rows"), |uri| {
            Box::pin(crate::frontierjson::frontier_json(uri))
        }),
    ]
}

/// The malformed selectors a handler's own grammar must refuse.
fn malformed(contract: Contract) -> Vec<String> {
    let mut queries = vec![
        String::new(),
        "identity".into(),
        "identity=".into(),
        "identity=not-a-digest".into(),
    ];
    match contract {
        Contract::Saved => queries.extend([
            format!("identity={}", ID.to_ascii_uppercase()),
            format!("identity={ID}&identity={ID}"),
            format!("identity={ID}&unexpected=value"),
            format!("identity={ID}&%69dentity={ID}"),
            format!("identity={ID}&"),
            format!("identity={ID}&unknown=%22quoted%22"),
        ]),
        // The detail grammar is a 64-hex identity and an optional bounded page.
        // It reads the six strict-grammar queries above as valid selectors, so
        // they are not malformed here (D-0689).
        Contract::Detail(_) => queries.extend([
            format!("identity={}", "a".repeat(63)),
            format!("identity={ID}&page=banana"),
            format!("identity={ID}&page={}", crate::detail::MAX_PAGE + 1),
            format!("identity={ID}&limit=0"),
            format!("identity={ID}&limit={}", crate::detail::MAX_PAGE_ROWS + 1),
        ]),
    }
    queries
}

fn poll_once(name: &str, handler: Handler, query: &str) -> Result<Reply, String> {
    let uri = format!("/saved?{query}")
        .parse::<Uri>()
        .map_err(|why| why.to_string())?;
    let mut pending = handler(uri);
    let reply = pending
        .as_mut()
        .poll(&mut Context::from_waker(Waker::noop()));
    let Poll::Ready(reply) = reply else {
        return Err(format!(
            "{name} queued an invalid query instead of refusing immediately"
        ));
    };
    Ok(reply)
}

fn immediate_refusal(
    name: &str,
    contract: Contract,
    handler: Handler,
    query: &str,
) -> Result<String, String> {
    let (status, headers, body) = poll_once(name, handler, query)?;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{name}: {query}");
    assert!(body.len() <= crate::detail::MAX_RESPONSE_BYTES, "{name}");
    let value = serde_json::from_str::<Value>(&body).map_err(|why| format!("{name}: {why}"))?;
    match contract {
        Contract::Saved => {
            assert_eq!(headers, [(CONTENT_TYPE, "application/json")], "{name}");
            assert_eq!(
                value.get("schema_version").and_then(Value::as_u64),
                Some(1),
                "{name}"
            );
            assert_eq!(
                value.get("status").and_then(Value::as_str),
                Some("refused"),
                "{name}"
            );
            assert_eq!(
                value.get("rows").and_then(Value::as_array).map(Vec::len),
                Some(0),
                "{name}"
            );
        }
        Contract::Detail(rows) => {
            assert_eq!(
                headers,
                [(CONTENT_TYPE, "application/json; charset=utf-8")],
                "{name}"
            );
            assert_eq!(
                value.get(rows),
                Some(&Value::Null),
                "{name}: a refused detail page lists no rows, not an empty list"
            );
            assert_eq!(
                value.get("count").and_then(Value::as_u64),
                Some(0),
                "{name}"
            );
            assert_eq!(
                value.get("complete").and_then(Value::as_bool),
                Some(false),
                "{name}"
            );
        }
    }
    assert!(
        value
            .get("refusal")
            .and_then(Value::as_str)
            .is_some_and(|reason| !reason.trim().is_empty()),
        "{name}"
    );
    assert!(
        value.get("authority").is_none(),
        "{name}: refusal grants no saved authority"
    );
    Ok(body)
}

#[test]
fn invalid_saved_queries_return_immediate_named_http_refusals_without_rows() -> Result<(), String> {
    for (name, contract, handler) in handlers() {
        for query in &malformed(contract) {
            let first = immediate_refusal(name, contract, handler, query)?;
            assert_eq!(
                immediate_refusal(name, contract, handler, query)?,
                first,
                "{name}: exact retry"
            );
        }
    }
    Ok(())
}

#[test]
fn oversized_saved_queries_report_both_byte_counts_before_parsing_or_queueing() -> Result<(), String>
{
    let query = "a".repeat(crate::detail::MAX_QUERY_BYTES + 1);
    let expected = format!(
        "detail query is {} bytes; this endpoint accepts at most {}",
        query.len(),
        crate::detail::MAX_QUERY_BYTES
    );
    for (name, contract, handler) in handlers() {
        let body = immediate_refusal(name, contract, handler, &query)?;
        let value = serde_json::from_str::<Value>(&body).map_err(|why| why.to_string())?;
        assert_eq!(
            value.get("refusal").and_then(Value::as_str),
            Some(expected.as_str()),
            "{name}"
        );
    }
    Ok(())
}

/// The refusal envelopes `/trades.json` and `/frontier.json` wrote from inside
/// the blocking task before D-0689, copied from that code's output.
const TRADES_REFUSED: &str = r#"{"policy":null,"direction":null,"trades":null,"periods":null,"count":0,"total_count":null,"page_complete":false,"complete":false,"refusal":"#;
const FRONTIER_REFUSED: &str = r#"{"rows":null,"count":0,"total_count":null,"admitted":0,"total_admitted":null,"page_complete":false,"complete":false,"refusal":"#;
const IDENTITY_REASON: &str = "`identity` must be the 64 hex characters `/backtest.json` prints on every row. This file holds many runs, so which one is not a detail it can infer.";

/// A MALFORMED DETAIL SELECTOR NEVER REACHES ADMISSION, AND ITS BYTES DID NOT MOVE.
///
/// Both routes parsed inside `detail::run`, so a selector that could only be
/// refused took a slot and, while every slot was held, was answered 429. Here
/// each is polled once with no runtime: reaching admission would either panic
/// in `spawn_blocking` or return 429, and both fail. The expected bodies are
/// the ones the blocking path wrote, byte for byte (D-0689).
#[test]
fn detail_selectors_are_refused_before_admission_with_the_blocking_paths_bytes()
-> Result<(), String> {
    // The identity is decoded after the store root, as it was inside the task,
    // so with no configured root the root's refusal is the one expected.
    let identity_reason =
        crate::server::store_dir().map_or_else(|why| why, |_| IDENTITY_REASON.to_owned());
    let cases = [
        (String::new(), identity_reason.clone()),
        ("identity=not-a-digest".to_owned(), identity_reason.clone()),
        (format!("identity={}", "a".repeat(63)), identity_reason),
        (
            format!("identity={ID}&page=banana"),
            "`page` must be an unsigned decimal integer".to_owned(),
        ),
        (
            format!("identity={ID}&page=4096"),
            "`page` is 4096; the hard maximum is 4095".to_owned(),
        ),
        (
            format!("identity={ID}&limit=257"),
            "`limit` must be from 1 through 256; received 257".to_owned(),
        ),
    ];
    let routes: [(&str, Handler, &str); 2] = [
        (
            "chosen trades",
            |uri| Box::pin(crate::trades::trades_json(uri)),
            TRADES_REFUSED,
        ),
        (
            "frontier",
            |uri| Box::pin(crate::frontierjson::frontier_json(uri)),
            FRONTIER_REFUSED,
        ),
    ];
    for (name, handler, envelope) in routes {
        for (query, reason) in &cases {
            let (status, headers, body) = poll_once(name, handler, query)?;
            assert_eq!(status, StatusCode::BAD_REQUEST, "{name}: {query}");
            assert_eq!(
                headers,
                [(CONTENT_TYPE, "application/json; charset=utf-8")],
                "{name}"
            );
            assert_eq!(
                body,
                format!("{envelope}{}}}", crate::render::json_string(reason)),
                "{name}: {query}"
            );
        }
    }
    Ok(())
}

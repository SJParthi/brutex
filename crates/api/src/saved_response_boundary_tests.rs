//! Public read-boundary probes. Hand-written invalid queries must finish before
//! any background work can start, so these tests deliberately have no runtime.
//! `/trades.json` and `/frontier.json` are not probed: they parse the selector
//! inside the bounded blocking task (D-0679).
use axum::http::{HeaderName, StatusCode, Uri, header::CONTENT_TYPE};
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

type Reply = (StatusCode, [(HeaderName, &'static str); 1], String);
type Handler = fn(Uri) -> Pin<Box<dyn Future<Output = Reply>>>;

const ID: &str = "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";

fn handlers() -> [(&'static str, Handler); 15] {
    [
        ("campaign", |uri| {
            Box::pin(crate::booleancampaignjson::campaign_json(uri))
        }),
        ("qualified campaign", |uri| {
            Box::pin(crate::booleancampaignjson::qualified_campaign_json(uri))
        }),
        ("statistics", |uri| {
            Box::pin(crate::booleanevidencejson::statistics_json(uri))
        }),
        ("admission", |uri| {
            Box::pin(crate::booleanevidencejson::admission_json(uri))
        }),
        ("qualification", |uri| {
            Box::pin(crate::booleanevidencejson::qualification_json(uri))
        }),
        ("Boolean catalog", |uri| {
            Box::pin(crate::booleanjson::boolean_json(uri))
        }),
        ("later comparison", |uri| {
            Box::pin(crate::booleanoosjson::later_json(uri))
        }),
        ("qualified search", |uri| {
            Box::pin(crate::booleansearchjson::search_json(uri))
        }),
        ("expression search", |uri| {
            Box::pin(crate::expressionsearchjson::expression_search_json(uri))
        }),
        ("index stop", |uri| {
            Box::pin(crate::indexstopjson::index_stop_json(uri))
        }),
        ("index candles", |uri| {
            Box::pin(crate::indexstopcandlesjson::index_stop_candles_json(uri))
        }),
        ("index qualification", |uri| {
            Box::pin(crate::indexstopqualificationjson::index_stop_qualification_json(uri))
        }),
        ("index ranking", |uri| {
            Box::pin(crate::indexstoprankingjson::index_stop_ranking_json(uri))
        }),
        ("index VIX", |uri| {
            Box::pin(crate::indexstopvixjson::index_stop_vix_json(uri))
        }),
        ("candidate trades", |uri| {
            Box::pin(crate::candidatejson::candidate_json(uri))
        }),
    ]
}

fn immediate_refusal(name: &str, handler: Handler, query: &str) -> Result<String, String> {
    let uri = format!("/saved?{query}")
        .parse::<Uri>()
        .map_err(|why| why.to_string())?;
    let mut pending = handler(uri);
    let reply = pending
        .as_mut()
        .poll(&mut Context::from_waker(Waker::noop()));
    let Poll::Ready((status, headers, body)) = reply else {
        return Err(format!(
            "{name} queued an invalid query instead of refusing immediately"
        ));
    };
    assert_eq!(status, StatusCode::BAD_REQUEST, "{name}: {query}");
    assert_eq!(headers, [(CONTENT_TYPE, "application/json")], "{name}");
    assert!(body.len() <= crate::detail::MAX_RESPONSE_BYTES, "{name}");
    let value = serde_json::from_str::<Value>(&body).map_err(|why| format!("{name}: {why}"))?;
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
    let queries = [
        String::new(),
        "identity".into(),
        "identity=".into(),
        "identity=not-a-digest".into(),
        format!("identity={}", ID.to_ascii_uppercase()),
        format!("identity={ID}&identity={ID}"),
        format!("identity={ID}&unexpected=value"),
        format!("identity={ID}&%69dentity={ID}"),
        format!("identity={ID}&"),
        format!("identity={ID}&unknown=%22quoted%22"),
    ];
    for (name, handler) in handlers() {
        for query in &queries {
            let first = immediate_refusal(name, handler, query)?;
            assert_eq!(
                immediate_refusal(name, handler, query)?,
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
    for (name, handler) in handlers() {
        let body = immediate_refusal(name, handler, &query)?;
        let value = serde_json::from_str::<Value>(&body).map_err(|why| why.to_string())?;
        assert_eq!(
            value.get("refusal").and_then(Value::as_str),
            Some(expected.as_str()),
            "{name}"
        );
    }
    Ok(())
}

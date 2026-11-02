//! Every saved-research door, through its own handler (BASE-COV-API, D-7344).
//!
//! Each door reads an exact, pinned receipt under the store root. A door asked
//! for a receipt nobody saved must say so by name on the refused shape, a body
//! with no row in it, and a query it cannot read must be refused before the
//! store is touched. Each door's own tests drive its reader; this file drives
//! the part they share and none of them reached: the handler around it.
//!
//! The identities are invented, all ones, so no store holds them: the root the
//! environment names is asked for a file that is not there, and nothing writes.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "a test that cannot panic cannot fail"
)]

use axum::http::{StatusCode, Uri};
use serde_json::Value;

/// An invented identity, canonical lowercase hex.
const ID: &str = "1111111111111111111111111111111111111111111111111111111111111111";

macro_rules! doors {
    ($($name:literal => $door:path),* $(,)?) => {
        /// `name`'s handler, asked `uri`: its status and its body.
        async fn ask(name: &str, uri: Uri) -> (StatusCode, String) {
            match name {
                $($name => {
                    let (status, _, body) = $door(uri).await;
                    (status, body)
                })*
                other => panic!("no door named {other}"),
            }
        }
    };
}

doors! {
    "index-stop" => crate::indexstopjson::index_stop_json,
    "index-stop-vix" => crate::indexstopvixjson::index_stop_vix_json,
    "index-stop-qualification" => crate::indexstopqualificationjson::index_stop_qualification_json,
    "index-stop-ranking" => crate::indexstoprankingjson::index_stop_ranking_json,
    "index-stop-candles" => crate::indexstopcandlesjson::index_stop_candles_json,
    "boolean" => crate::booleanjson::boolean_json,
    "boolean-statistics" => crate::booleanevidencejson::statistics_json,
    "boolean-admission" => crate::booleanevidencejson::admission_json,
    "boolean-qualification" => crate::booleanevidencejson::qualification_json,
    "boolean-search" => crate::booleansearchjson::search_json,
    "boolean-campaign" => crate::booleancampaignjson::campaign_json,
    "boolean-qualified-campaign" => crate::booleancampaignjson::qualified_campaign_json,
    "boolean-later" => crate::booleanoosjson::later_json,
    "expression-search" => crate::expressionsearchjson::expression_search_json,
    "sweep-evidence" => crate::sweepevidence::evidence_json,
    "candidate" => crate::candidatejson::candidate_json,
}

/// Each door and the smallest query it reads, past `identity`.
fn every_door() -> Vec<(&'static str, String)> {
    let rung = cli::EVERY_RUNG[0];
    let id = format!("identity={ID}");
    vec![
        ("index-stop", id.clone()),
        ("index-stop-vix", format!("{id}&pin={ID}&setting=0")),
        ("index-stop-qualification", id.clone()),
        ("index-stop-ranking", format!("{id}&timeframe={rung}")),
        (
            "index-stop-candles",
            format!("{id}&pin={ID}&setting=0&kind=source"),
        ),
        ("boolean", id.clone()),
        ("boolean-statistics", id.clone()),
        ("boolean-admission", id.clone()),
        ("boolean-qualification", id.clone()),
        ("boolean-search", id.clone()),
        ("boolean-campaign", id.clone()),
        ("boolean-qualified-campaign", id.clone()),
        ("boolean-later", id.clone()),
        ("expression-search", id.clone()),
        ("sweep-evidence", id.clone()),
        ("candidate", format!("{id}&attempt=1")),
    ]
}

fn uri(query: &str) -> Uri {
    format!("/door?{query}").parse().unwrap()
}

/// A refused body: valid JSON whose `refusal` says why in words and which
/// carries no row a reader could take for an answer.
fn assert_refused(door: &str, query: &str, body: &str) -> String {
    let parsed: Value = serde_json::from_str(body).unwrap_or_else(|why| {
        panic!("{door} {query}: a refused body is JSON ({why}): {body}");
    });
    assert_eq!(parsed["status"], "refused", "{door} {query}: {body}");
    let why = parsed["refusal"].as_str().unwrap_or_default();
    assert!(!why.is_empty(), "{door} {query}: {body}");
    assert!(
        parsed
            .get("rows")
            .and_then(Value::as_array)
            .is_none_or(Vec::is_empty),
        "{door} {query}: a refusal carries no row: {body}"
    );
    why.to_owned()
}

/// **A RECEIPT NOBODY SAVED IS NAMED AS MISSING ON EVERY DOOR.** The query is
/// good, so the handler goes to the store, and the store holds no such
/// receipt: a `503` on the refused shape, or, on a door whose question is
/// whether a search was ever recorded, a `200` that says `missing` and why.
/// Never a `200` that is only an empty page.
#[tokio::test]
async fn a_receipt_nobody_saved_is_refused_by_name_on_every_door() {
    for (door, query) in every_door() {
        let (status, body) = ask(door, uri(&query)).await;
        if status == StatusCode::OK {
            let parsed: Value = serde_json::from_str(&body).expect("a JSON answer");
            assert_eq!(parsed["status"], "missing", "{door}: {body}");
            assert!(
                parsed["why"].as_str().is_some_and(|why| !why.is_empty()),
                "{door}: {body}"
            );
            assert_eq!(parsed["rows"], serde_json::json!([]), "{door}: {body}");
        } else {
            assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{door}: {body}");
            assert_refused(door, &query, &body);
        }
    }
}

/// **A QUERY A DOOR CANNOT READ IS A `400`, BEFORE THE STORE.** Extremes, on
/// every door: no query at all, an identity of the wrong case, one hex digit
/// short, one digit over, and a bare key; a repeated field, an unknown field
/// beside a good one and a limit past every door's cap; the two together, a
/// good identity with both a repeated and an unknown field; and, out of the
/// box, a query that is one long run of `&` and a non-ASCII digit in the hex.
#[tokio::test]
async fn a_query_a_door_cannot_read_is_a_400_before_the_store() {
    let long = "&".repeat(600);
    for (door, query) in every_door() {
        let upper = query.replacen(ID, &"A".repeat(64), 1);
        let short = query.replacen(ID, &ID[1..], 1);
        let over = query.replacen(ID, &format!("{ID}1"), 1);
        let bad_digit = query.replacen(ID, &format!("%D9%A1{}", &ID[1..]), 1);
        for bad in [
            String::new(),
            upper,
            short,
            over,
            bad_digit,
            "identity".to_owned(),
            format!("{query}&identity={ID}"),
            format!("{query}&alien=1"),
            format!("{query}&limit=18446744073709551616"),
            format!("{query}&identity={ID}&alien=1"),
            long.clone(),
        ] {
            let (status, body) = ask(door, uri(&bad)).await;
            assert_eq!(status, StatusCode::BAD_REQUEST, "{door} {bad}: {body}");
            assert_refused(door, &bad, &body);
        }
    }
}

//! Bounded sweep attempt evidence. Pages bind identity and attempt; lifecycle
//! completion is distinct from institutional admission or arbitrary trade replay.

use cli::sweep_evidence::{self, Evidence};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

type Response = (
    axum::http::StatusCode,
    [(axum::http::header::HeaderName, &'static str); 1],
    String,
);

struct Asked {
    identity: [u8; 32],
    page: crate::detail::Page,
    ranked: bool,
    attempt: Option<u64>,
}

impl Asked {
    fn parse(query: &str) -> Result<Self, String> {
        crate::detail::query_is_bounded(query)?;
        let mut seen = std::collections::BTreeSet::new();
        for pair in query.split('&') {
            let (key, value) = pair
                .split_once('=')
                .ok_or("evidence query requires key=value fields")?;
            if value.is_empty()
                || !matches!(key, "identity" | "kind" | "page" | "limit" | "attempt")
                || !seen.insert(key)
            {
                return Err(format!(
                    "empty, unknown or repeated evidence query field {key:?}"
                ));
            }
        }
        let raw = crate::server::param(query, "identity");
        let identity = crate::trades::from_hex_public(&raw)
            .ok_or("identity must be 64 canonical lowercase hex characters")?;
        if raw != crate::server::hex32(identity) {
            return Err("identity must use canonical lowercase hex".to_owned());
        }
        let ranked = match crate::server::param(query, "kind").as_str() {
            "" | "depth" => false,
            "ranked" => true,
            _ => return Err("kind must be depth or ranked".to_owned()),
        };
        let page = crate::detail::Page::parse(query)?;
        let raw = crate::server::param(query, "attempt");
        let attempt = if raw.is_empty() {
            None
        } else {
            let value = raw
                .parse::<u64>()
                .map_err(|_| "attempt must be a positive canonical u64")?;
            if value == 0 || raw != value.to_string() {
                return Err("attempt must be a positive canonical u64".to_owned());
            }
            Some(value)
        };
        if page.number != 0 && attempt.is_none() {
            return Err("later pages require the attempt returned by page zero".to_owned());
        }
        Ok(Self {
            identity,
            page,
            ranked,
            attempt,
        })
    }
}

/// Read one bounded saved page without blocking an async worker.
pub async fn evidence_json(uri: axum::http::Uri) -> Response {
    let asked = match Asked::parse(uri.query().unwrap_or_default()) {
        Ok(asked) => asked,
        Err(why) => return refusal(axum::http::StatusCode::BAD_REQUEST, &why),
    };
    let root = crate::server::store_dir();
    match crate::detail::run(move || respond(root, &asked)).await {
        Ok(response) => response,
        Err(crate::detail::RunError::Saturated) => refusal(
            axum::http::StatusCode::TOO_MANY_REQUESTS,
            "sweep evidence capacity is full; no blocking task was queued",
        ),
        Err(crate::detail::RunError::Join(why)) => {
            refusal(axum::http::StatusCode::SERVICE_UNAVAILABLE, &why)
        }
    }
}

fn respond(root: Result<PathBuf, String>, asked: &Asked) -> Response {
    match root.and_then(|root| render(&root, asked)) {
        Ok(body) if body.len() <= crate::detail::MAX_RESPONSE_BYTES => {
            (axum::http::StatusCode::OK, headers(), body)
        }
        Ok(_) => refusal(
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            "saved evidence exceeds the JSON response bound; no prefix was exposed",
        ),
        Err(why) => refusal(axum::http::StatusCode::SERVICE_UNAVAILABLE, &why),
    }
}

fn render(root: &Path, asked: &Asked) -> Result<String, String> {
    let identity = crate::server::hex32(asked.identity);
    let Some(evidence) = sweep_evidence::read(root, asked.identity, crate::detail::MAX_SCAN_BYTES)?
    else {
        if asked.attempt.is_some() || asked.page.number != 0 {
            return Err("the requested saved attempt is no longer available; no replacement page was served".to_owned());
        }
        return Ok(format!(
            r#"{{"schema_version":1,"identity":"{identity}","status":"missing","evidence":null,"rows":[],"refusal":null,"why":"No versioned sweep attempt evidence is stored for this identity. Older result rows cannot reconstruct the missing levels."}}"#
        ));
    };
    if asked
        .attempt
        .is_some_and(|attempt| evidence.attempt != attempt)
    {
        return Err("the saved attempt changed between pages; restart at page zero".to_owned());
    }
    let total = if asked.ranked {
        evidence.ranked_rows
    } else {
        evidence.depth_rows
    };
    let window = crate::detail::window(
        usize::try_from(total).map_err(|why| why.to_string())?,
        asked.page,
    )?;
    let limit = window.end - window.start;
    let offset = asked.page.offset();
    let mut rows = String::new();
    if limit != 0 {
        if asked.ranked {
            for row in sweep_evidence::ranked_page(
                root,
                &evidence,
                offset,
                limit,
                crate::detail::MAX_SCAN_BYTES,
            )? {
                if !rows.is_empty() {
                    rows.push(',');
                }
                ranked_json(&mut rows, &row);
            }
        } else {
            for row in sweep_evidence::depth_page(
                root,
                &evidence,
                offset,
                limit,
                crate::detail::MAX_SCAN_BYTES,
            )? {
                if !rows.is_empty() {
                    rows.push(',');
                }
                depth_json(&mut rows, row);
            }
        }
    }
    if sweep_evidence::read(root, asked.identity, crate::detail::MAX_SCAN_BYTES)? != Some(evidence)
    {
        return Err(
            "saved evidence changed during the bounded read; no mixed snapshot was exposed"
                .to_owned(),
        );
    }
    let kind = if asked.ranked { "ranked" } else { "depth" };
    let next = window
        .next_page
        .map_or_else(|| "null".to_owned(), |next| next.to_string());
    Ok(format!(
        r#"{{"schema_version":1,"identity":"{identity}","status":"saved","evidence":{},"kind":"{kind}","page":{},"limit":{},"total_count":"{total}","next_page":{next},"page_complete":true,"rows":[{rows}],"refusal":null}}"#,
        metadata(evidence),
        asked.page.number,
        asked.page.limit
    ))
}

fn metadata(e: Evidence) -> String {
    let validation = e
        .validation_requested
        .map_or_else(|| "null".to_owned(), |on| on.to_string());
    format!(
        r#"{{"attempt":"{}","operation":"{}","completion":"{}","started_micros":"{}","updated_micros":"{}","depth_rows":"{}","ranked_rows":"{}","ranked_available":{},"validation_requested":{validation}}}"#,
        e.attempt,
        e.operation.as_str(),
        e.completion.as_str(),
        e.started_micros,
        e.updated_micros,
        e.depth_rows,
        e.ranked_rows,
        e.ranked_available
    )
}

fn depth_json(out: &mut String, row: sweep_evidence::DepthRow) {
    let _ = write!(
        out,
        r#"{{"k":"{}","generated":"{}","duplicates":"{}","excluded":"{}","pruned":"{}","infrequent":"{}","frequent":"{}","admitted":"{}","pairs":"{}","reconciles":{}}}"#,
        row.k,
        row.generated,
        row.duplicates,
        row.excluded,
        row.pruned,
        row.infrequent,
        row.frequent,
        row.admitted,
        row.pairs,
        row.reconciles
    );
}

fn ranked_json(out: &mut String, row: &sweep_evidence::RankedRow) {
    let _ = write!(out, r#"{{"rank":"{}","mask_words":["#, row.rank);
    for (at, word) in row.mask_words.iter().enumerate() {
        if at > 0 {
            out.push(',');
        }
        let _ = write!(out, "\"{word}\"");
    }
    let _ = write!(
        out,
        r#"],"hits":"{}","observations":"{}","mean_bits":"{}","t_bits":"{}","refused":"{}","mismatched":"{}","wins":"{}","losses":"{}","win_sum_bits":"{}","loss_sum_bits":"{}","adverse_sum_bits":"{}","favourable_sum_bits":"{}"}} "#,
        row.hits,
        row.observations,
        row.mean_bits,
        row.t_bits,
        row.refused,
        row.mismatched,
        row.wins,
        row.losses,
        row.win_sum_bits,
        row.loss_sum_bits,
        row.adverse_sum_bits,
        row.favourable_sum_bits
    );
}

fn headers() -> [(axum::http::header::HeaderName, &'static str); 1] {
    [(
        axum::http::header::CONTENT_TYPE,
        "application/json; charset=utf-8",
    )]
}

fn refusal(status: axum::http::StatusCode, why: &str) -> Response {
    (
        status,
        headers(),
        format!(
            r#"{{"schema_version":1,"status":"refused","evidence":null,"rows":[],"refusal":{}}}"#,
            crate::render::json_string(why)
        ),
    )
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "tests fail through assertions"
)]
mod tests {
    use super::{Asked, depth_json, ranked_json, respond};
    use cli::sweep_evidence;

    fn level(k: u64) -> sweep_evidence::DepthRow {
        sweep_evidence::DepthRow {
            k,
            generated: 1,
            duplicates: 0,
            excluded: 0,
            pruned: 0,
            infrequent: 0,
            frequent: 1,
            admitted: k,
            pairs: k,
            reconciles: true,
        }
    }

    #[test]
    fn durable_depths_are_paged_with_exact_attempt_and_validation_state() {
        let dir = crate::scratch::path("sweep-evidence-http-pages");
        let _ = std::fs::remove_dir_all(&dir);
        let id = [0xab; 32];
        let attempt = sweep_evidence::begin_with_validation(
            &dir,
            id,
            sweep_evidence::Operation::Audit,
            Some(false),
        )
        .expect("durable start");
        for k in 1..=257 {
            attempt.level(level(k)).expect("bound depth row");
        }
        attempt.ranked(&[]).expect("published empty ranking");
        attempt
            .finish(sweep_evidence::Completion::Completed)
            .expect("durable finish");
        let summary = sweep_evidence::read(&dir, id, crate::detail::MAX_SCAN_BYTES)
            .expect("read")
            .expect("present");
        let query = format!("identity={}&kind=depth", crate::server::hex32(id));
        let asked = Asked::parse(&query).expect("page zero");
        let (status, _, body) = respond(Ok(dir.clone()), &asked);
        assert_eq!(status, axum::http::StatusCode::OK);
        let parsed: serde_json::Value = serde_json::from_str(&body).expect("valid endpoint JSON");
        assert_eq!(
            parsed.get("identity").and_then(serde_json::Value::as_str),
            Some(crate::server::hex32(id).as_str())
        );
        assert_eq!(
            parsed
                .get("rows")
                .and_then(serde_json::Value::as_array)
                .map(Vec::len),
            Some(256)
        );
        assert!(body.contains(r#""next_page":1"#));
        assert_eq!(body.matches(r#""reconciles":true"#).count(), 256);
        assert!(body.contains(r#""validation_requested":false"#));
        assert!(body.contains(r#""ranked_available":true"#));
        assert!(body.contains(r#""total_count":"257""#));
        let next = Asked::parse(&format!("{query}&page=1&attempt={}", summary.attempt))
            .expect("pinned next");
        let (_, _, tail) = respond(Ok(dir.clone()), &next);
        let _: serde_json::Value = serde_json::from_str(&tail).expect("valid final page JSON");
        assert_eq!(tail.matches(r#""reconciles":true"#).count(), 1);
        assert!(tail.contains(r#""k":"257""#));
        assert!(tail.contains(r#""next_page":null"#));
        let newer =
            sweep_evidence::begin(&dir, id, sweep_evidence::Operation::Sweep).expect("new attempt");
        let (status, _, refused) = respond(Ok(dir.clone()), &next);
        assert_eq!(status, axum::http::StatusCode::SERVICE_UNAVAILABLE);
        assert!(refused.contains("attempt changed"));
        drop(newer);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn absence_and_invalid_queries_never_create_or_invent_evidence() {
        let dir = crate::scratch::path("sweep-evidence-http-absence");
        let _ = std::fs::remove_dir_all(&dir);
        let query = format!("identity={}", "1a".repeat(32));
        let asked = Asked::parse(&query).expect("canonical identity");
        let (status, _, body) = respond(Ok(dir.clone()), &asked);
        assert_eq!(status, axum::http::StatusCode::OK);
        assert!(body.contains(r#""status":"missing""#));
        assert!(!dir.exists());
        for suffix in [
            "&page=1",
            "&page=0&page=1",
            "&limit=257",
            "&kind=trades",
            "&attempt=0",
            "&attempt=01",
            "&extra=x",
            "&limit=",
        ] {
            assert!(
                Asked::parse(&format!("{query}{suffix}")).is_err(),
                "{suffix}"
            );
        }
        assert!(Asked::parse(&format!("identity={}", "AB".repeat(32))).is_err());
        assert!(Asked::parse(&"x".repeat(crate::detail::MAX_QUERY_BYTES + 1)).is_err());
        let (_, _, failure) = respond(Err("unreadable evidence root".to_owned()), &asked);
        assert!(failure.contains(r#""status":"refused""#));
        assert!(!failure.contains(r#""status":"missing""#));
    }

    #[test]
    fn evidence_serialization_preserves_u64_and_ieee_bits_as_strings() {
        let mut out = String::new();
        depth_json(
            &mut out,
            sweep_evidence::DepthRow {
                generated: u64::MAX,
                infrequent: u64::MAX,
                frequent: 0,
                ..level(1)
            },
        );
        assert!(out.contains(r#""generated":"18446744073709551615""#));
        let mut out = String::new();
        ranked_json(
            &mut out,
            &sweep_evidence::RankedRow {
                rank: 1,
                mask_words: [u64::MAX; 6],
                hits: u64::MAX,
                observations: 2,
                mean_bits: 1.234_567_890_123_456_f64.to_bits(),
                t_bits: f64::INFINITY.to_bits(),
                refused: 0,
                mismatched: 0,
                wins: 1,
                losses: 1,
                win_sum_bits: 1.0_f64.to_bits(),
                loss_sum_bits: (-1.0_f64).to_bits(),
                adverse_sum_bits: 0,
                favourable_sum_bits: 0,
            },
        );
        let parsed: serde_json::Value =
            serde_json::from_str(&out).expect("valid exact-ranked JSON");
        assert_eq!(
            parsed
                .get("mask_words")
                .and_then(serde_json::Value::as_array)
                .map(Vec::len),
            Some(6)
        );
        assert_eq!(out.matches("\"18446744073709551615\"").count(), 7);
        assert!(out.contains(&format!(
            r#""mean_bits":"{}""#,
            1.234_567_890_123_456_f64.to_bits()
        )));
        assert!(out.contains(&format!(r#""t_bits":"{}""#, f64::INFINITY.to_bits())));
        assert!(out.contains(r#""losses":"1""#));
    }
}

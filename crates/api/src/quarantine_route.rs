//! `GET /pull/quarantine.json` and `POST /pull/quarantine`: the Zerodha
//! quarantine's plan and its press. R4-18; findings ING-01, RT-DATA-02, Z5,
//! Z6.
//!
//! Both run the ONE verb, `cli zerodha-quarantine` (`cli::zerodha_quarantine_at`):
//! the GET with `--plan`, which writes nothing; the POST with `--apply`. No
//! second implementation of what is set aside, how, or in what words.
//!
//! # The question
//!
//! `symbol` (a share, or `ALL`), `from` and `to` (`YYYY-MM-DD`), `cause`
//! (`before_listing`, `foreign_level`, `zero_price`), `reason` (1 to 200
//! bytes, one line; the POST's is required, the GET's defaults to `plan`),
//! `segment` (`CASH` unless named). Anything else is refused by name with
//! 400 before the verb runs.
//!
//! # Bounded, and one writer
//!
//! The plan runs on the store-read pool (`detail::run_store_read`); the
//! press on the operations pool (`detail::run_ops_read`) and only ONE at a
//! time in this process: a second press while one runs is 409
//! `quarantine_running`, never queued behind it. A full pool is 429 with
//! `Retry-After`. An unstamped build never reaches the press (the server's
//! write gate answers every POST 403 first).
//!
//! # Cost
//!
//! The plan: one plan file read and, per planned month, the month's bars —
//! O(plan + months named), NOT O(1); bounded by the plan, which names only
//! the quarantined months of one cause. The press adds the door's O(month)
//! per month. Neither is on a read path a page polls.

use std::sync::atomic::{AtomicBool, Ordering};

use super::{Loaded, param};
use crate::render;

/// Whether a press is running in this process.
static PRESSING: AtomicBool = AtomicBool::new(false);

/// The press, held: dropped when the request ends, whether it answered, was
/// refused or its reader left, so a press is never held for ever.
struct Pressing;

impl Drop for Pressing {
    fn drop(&mut self) {
        PRESSING.store(false, Ordering::Release);
    }
}

type Answer = (
    axum::http::StatusCode,
    [(axum::http::HeaderName, &'static str); 1],
    String,
);

fn json(status: axum::http::StatusCode, body: String) -> Answer {
    (
        status,
        [(
            axum::http::header::CONTENT_TYPE,
            "application/json; charset=utf-8",
        )],
        body,
    )
}

fn refused(status: axum::http::StatusCode, code: &str, why: &str) -> Answer {
    json(
        status,
        format!(
            r#"{{"error":{},"code":{}}}"#,
            render::json_string(why),
            render::json_string(code)
        ),
    )
}

/// The verb's arguments from a query or a form, or the refusal.
pub(crate) fn arguments(raw: &str, apply: bool) -> Result<Vec<String>, String> {
    let known = ["symbol", "from", "to", "cause", "reason", "segment"];
    for pair in raw.split('&').filter(|pair| !pair.is_empty()) {
        let key = pair.split_once('=').map_or(pair, |(key, _)| key);
        if !known.contains(&key) {
            return Err(format!("`{key}` is not a parameter of the quarantine"));
        }
    }
    let symbol = param(raw, "symbol");
    let (from, to, cause) = (param(raw, "from"), param(raw, "to"), param(raw, "cause"));
    for (name, value) in [
        ("symbol", &symbol),
        ("from", &from),
        ("to", &to),
        ("cause", &cause),
    ] {
        if value.is_empty() {
            return Err(format!("name `{name}`"));
        }
    }
    let asked = param(raw, "reason");
    // A PLAN WRITES NOTHING, so it needs no reason of its own; a press does.
    let reason = match asked.trim() {
        "" if !apply => "plan".to_owned(),
        given => given.to_owned(),
    };
    store::quarantine::check_reason(&reason).map_err(|why| why.to_string())?;
    let mut out = vec![
        symbol,
        from,
        to,
        "--cause".to_owned(),
        cause,
        "--reason".to_owned(),
        reason,
    ];
    let segment = param(raw, "segment");
    if !segment.is_empty() {
        out.push("--segment".to_owned());
        out.push(segment);
    }
    out.push(if apply { "--apply" } else { "--plan" }.to_owned());
    Ok(out)
}

fn answer(code: u8, report: &str, applied: bool) -> Answer {
    let status = match code {
        cli::OK => axum::http::StatusCode::OK,
        cli::MISUSED => axum::http::StatusCode::BAD_REQUEST,
        _ => axum::http::StatusCode::UNPROCESSABLE_ENTITY,
    };
    json(
        status,
        format!(
            r#"{{"exit":{code},"applied":{applied},"report":{}}}"#,
            render::json_string(report)
        ),
    )
}

/// `GET /pull/quarantine.json`: the plan, nothing written.
pub(super) async fn quarantine_plan_json(
    axum::extract::State(site): axum::extract::State<Loaded>,
    uri: axum::http::Uri,
) -> Answer {
    let args = match arguments(uri.query().unwrap_or(""), false) {
        Ok(args) => args,
        Err(why) => {
            return refused(
                axum::http::StatusCode::BAD_REQUEST,
                "quarantine_question",
                &why,
            );
        }
    };
    let root = site.store_root.clone();
    let run = crate::detail::run_store_read(move || {
        let borrowed: Vec<&str> = args.iter().map(String::as_str).collect();
        let mut out = String::new();
        let code = cli::zerodha_quarantine_at(&borrowed, &root, &mut out);
        (code, out)
    })
    .await;
    match run {
        Ok((code, report)) => answer(code, &report, false),
        Err(why) => busy(&why),
    }
}

fn busy(why: &crate::detail::RunError) -> Answer {
    let (status, body) = crate::detail::admission_refused(
        "the quarantine",
        crate::detail::MAX_STORE_READ_CONCURRENT,
        why,
    );
    json(status, body)
}

/// `POST /pull/quarantine`: the press. One at a time in this process.
pub(super) async fn quarantine_apply(
    axum::extract::State(site): axum::extract::State<Loaded>,
    body: String,
) -> Answer {
    let args = match arguments(&body, true) {
        Ok(args) => args,
        Err(why) => {
            return refused(
                axum::http::StatusCode::BAD_REQUEST,
                "quarantine_question",
                &why,
            );
        }
    };
    if PRESSING
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return refused(
            axum::http::StatusCode::CONFLICT,
            "quarantine_running",
            "a quarantine press is running in this server; wait for it and press again",
        );
    }
    let _pressing = Pressing;
    let root = site.store_root.clone();
    let run = crate::detail::run_ops_read(move || {
        let borrowed: Vec<&str> = args.iter().map(String::as_str).collect();
        let mut out = String::new();
        let code = cli::zerodha_quarantine_at(&borrowed, &root, &mut out);
        (code, out)
    })
    .await;
    match run {
        Ok((code, report)) => answer(code, &report, true),
        Err(why) => busy(&why),
    }
}

#[cfg(test)]
#[path = "quarantine_route_tests.rs"]
mod tests;

//! `GET /ingest/refusals.json?feed=zerodha&symbols=NIFTY,BANKNIFTY` -- each
//! named instrument's pull refusals from the pull journal, folded by month, so
//! /ingest can say a month was REQUESTED AND REFUSED rather than "never
//! pulled" (WP-61, DB-G23).
//!
//! # Why this route exists
//!
//! /ingest judged a month by the census alone: no file was "never pulled",
//! with the tooltip "Nothing has asked the vendor for it yet — a pull IS
//! warranted". For Zerodha's 1min months 2020-11, 2021-11 and 2023-11 that was
//! false: run #34 asked for them and the journal holds each instrument's
//! refusal (an exceptional session the calendar could not size). The journal's
//! own index files each refusal under the symbol it pulled
//! (`pull.journal.idx/refusals/<symbol>.idx`), and `/dayflags.json` already
//! reads it per instrument; this route asks the same reader
//! ([`crate::server::dayflags::zerodha_refusals_of`]) for a page's rows at once.
//!
//! # One field, a list in it
//!
//! `symbols` is ONE query field holding a comma-separated list. This server
//! refuses a query that names a field twice (`request_bounds_refusal`,
//! D-1202), and a store symbol never holds a comma, so the list needs no
//! second spelling of that rule. `M&M` travels as `M%26M`.
//!
//! # What a refusal says, and what it does not
//!
//! A refusal names the day its note names (`names_day`) or a whole window
//! refused before append (`whole_window`), the run's window, when it was
//! written and the note as journalled (`cut` when the journal kept only its
//! start; `/audit.json` carries the whole note by `record`). It does NOT name
//! the rung: a pull journal record carries no timeframe, so a page states that
//! the journal names this instrument's refusal of that day and never that it
//! was the rung on screen.
//!
//! # The answer is folded by month, and bounded
//!
//! One symbol can hold thousands of refusals (every refused minute of a
//! special session is a record). A page draws a month cell, so each symbol's
//! refusals are folded here to one object per month it names: how many, the
//! first [`MAX_DAYS`] distinct days, and the NEWEST record's note with its
//! record number. Whole-window refusals are listed newest first up to
//! [`MAX_WHOLE`], with a count of the rest. So an answer is at most one object
//! per (symbol, month), whatever the journal holds: nothing is dropped
//! silently, every cut is a number beside what was kept.
//!
//! # Cost
//!
//! Per symbol, one read of its index file and one positional read per refusal
//! it names: O(that instrument's refusals), never the journal's 4.2 million
//! records. At most [`MAX_SYMBOLS`] symbols per request.

use crate::server::{Loaded, param};
use serde_json::{Value, json};
use std::collections::BTreeMap;

/// The most symbols one request may name: one /ingest page of rows is 25 to
/// 50, and a request for a whole universe is refused rather than answered
/// slowly. A page with more rows asks in several requests.
pub(crate) const MAX_SYMBOLS: usize = 120;

/// The most distinct days one month's object lists; the rest are counted in
/// `more_days`.
pub(crate) const MAX_DAYS: usize = 4;

/// The most whole-window refusals one symbol lists; the rest are counted in
/// `whole_more`.
pub(crate) const MAX_WHOLE: usize = 4;

/// The content type every answer here carries.
fn json_header() -> [(axum::http::HeaderName, &'static str); 1] {
    [(
        axum::http::header::CONTENT_TYPE,
        "application/json; charset=utf-8",
    )]
}

/// The symbols a request names, or why it cannot be answered.
///
/// # Errors
///
/// The sentence a 400 carries: no feed, a feed with no pull journal, no
/// symbol, too many, a repeat, or one that is not a store symbol.
pub(crate) fn asked(query: &str) -> Result<Vec<String>, String> {
    let feed = param(query, "feed");
    let zerodha = brutex_core::vendor::Vendor::Zerodha.as_str();
    if feed.is_empty() {
        return Err(format!(
            "/ingest/refusals.json needs ?feed={zerodha} and never guesses one"
        ));
    }
    if !feed.eq_ignore_ascii_case(zerodha) {
        return Err(format!(
            "feed {feed:?} has no pull refusals here: only {zerodha}'s pulls are journalled \
             per instrument. GDFL is imported, and its refused days are in \
             /imports.json?feed=gdfl"
        ));
    }
    let list = param(query, "symbols");
    let symbols: Vec<String> = list
        .split(',')
        .filter(|symbol| !symbol.is_empty())
        .map(str::to_owned)
        .collect();
    if symbols.is_empty() {
        return Err(
            "/ingest/refusals.json needs symbols=<store symbol>[,<store symbol>...]: one field, \
             a comma-separated list"
                .to_owned(),
        );
    }
    if symbols.len() > MAX_SYMBOLS {
        return Err(format!(
            "{} symbols named; one request reads at most {MAX_SYMBOLS}. Ask for the rows on \
             screen, not the universe",
            symbols.len()
        ));
    }
    let mut seen = std::collections::HashSet::with_capacity(symbols.len());
    for symbol in &symbols {
        if symbol.len() > 64
            || !symbol
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '&' | '-' | '_' | '.'))
        {
            return Err(format!(
                "symbols names {symbol:?}, which is not a store symbol"
            ));
        }
        if !seen.insert(symbol.as_str()) {
            return Err(format!(
                "symbols names {symbol:?} twice; each instrument is answered once"
            ));
        }
    }
    Ok(symbols)
}

/// One month's refusals of one symbol, as it is folded.
#[derive(Default)]
struct Month {
    count: u64,
    days: Vec<String>,
    more_days: u64,
    newest: Option<Value>,
}

/// One symbol's refusals, folded by the month each names. See the module
/// header for the bounds.
fn folded(symbol: &str, refusals: &[Value]) -> Value {
    let mut months: BTreeMap<String, Month> = BTreeMap::new();
    let mut whole: Vec<&Value> = Vec::new();
    for refusal in refusals {
        if refusal["whole_window"] == true {
            whole.push(refusal);
        }
        let Some(day) = refusal["names_day"].as_str() else {
            continue;
        };
        let Some(month) = day.get(..7) else {
            continue;
        };
        let entry = months.entry(month.to_owned()).or_default();
        entry.count = entry.count.saturating_add(1);
        if !entry.days.iter().any(|held| held == day) {
            if entry.days.len() < MAX_DAYS {
                entry.days.push(day.to_owned());
            } else {
                entry.more_days = entry.more_days.saturating_add(1);
            }
        }
        // OLDEST FIRST IS THE READER'S ORDER, so the last one seen is the
        // newest: the one a re-pull since would have replaced.
        entry.newest = Some(refusal.clone());
    }
    let months: serde_json::Map<String, Value> = months
        .into_iter()
        .map(|(month, entry)| {
            let newest = entry.newest.unwrap_or(Value::Null);
            (
                month,
                json!({
                    "count": entry.count,
                    "days": entry.days,
                    // DISTINCT DAYS PAST THE LIST ARE NOT COUNTED EXACTLY: a
                    // repeat of an unlisted day adds one. So this is "at
                    // least one more record of a day not listed", said as a
                    // count of records, never as a count of days.
                    "more_day_records": entry.more_days,
                    "note": newest.get("note"),
                    "cut": newest.get("cut"),
                    "record": newest.get("record"),
                    "at_unix_secs": newest.get("at_unix_secs"),
                    "window": newest.get("window"),
                }),
            )
        })
        .collect();
    let whole_total = whole.len();
    let whole_listed: Vec<Value> = whole
        .iter()
        .rev()
        .take(MAX_WHOLE)
        .map(|refusal| {
            json!({
                "window": refusal["window"],
                "note": refusal["note"],
                "cut": refusal["cut"],
                "record": refusal["record"],
                "at_unix_secs": refusal["at_unix_secs"],
            })
        })
        .collect();
    json!({
        "symbol": symbol,
        "count": refusals.len(),
        "months": months,
        "whole": whole_listed,
        "whole_count": whole_total,
        "whole_more": whole_total.saturating_sub(MAX_WHOLE),
    })
}

/// The whole answer over the store at `root`.
#[must_use]
pub(crate) fn answer(root: &std::path::Path, symbols: &[String]) -> Value {
    let mut rows = Vec::with_capacity(symbols.len());
    let mut source = Value::Null;
    let mut errors = Vec::new();
    for symbol in symbols {
        match crate::server::dayflags::zerodha_refusals_of(root, symbol) {
            Ok((refusals, read)) => {
                rows.push(folded(symbol, &refusals));
                source = read;
            }
            Err(why) => {
                errors.push(json!({ "symbol": symbol, "error": why }));
            }
        }
    }
    json!({
        "feed": brutex_core::vendor::Vendor::Zerodha.as_str(),
        "symbols": rows,
        "errors": errors,
        "source": source,
        "rung": null,
        "rung_note": "a pull journal record carries no timeframe: a refusal names an instrument \
                      and a day (or a whole window), never the rung it was asked at",
        "whole_note": "a note marked cut is the journal's first bytes; /audit.json?feed=zerodha \
                       carries the whole note by its record, and /dayflags.json?feed=zerodha \
                       lists every refusal of one instrument day by day",
        "bounds": { "symbols": MAX_SYMBOLS, "days_per_month": MAX_DAYS, "whole_listed": MAX_WHOLE },
    })
}

/// `GET /ingest/refusals.json`.
pub(super) async fn refusals_json(
    axum::extract::State(site): axum::extract::State<Loaded>,
    uri: axum::http::Uri,
) -> (
    axum::http::StatusCode,
    [(axum::http::HeaderName, &'static str); 1],
    String,
) {
    let query = uri.query().unwrap_or("").to_owned();
    let symbols = match asked(&query) {
        Ok(symbols) => symbols,
        Err(why) => {
            return (
                axum::http::StatusCode::BAD_REQUEST,
                json_header(),
                json!({ "error": why }).to_string(),
            );
        }
    };
    let root = site.store_root.clone();
    match crate::detail::run_store_read(move || answer(&root, &symbols)).await {
        Ok(body) => (axum::http::StatusCode::OK, json_header(), body.to_string()),
        Err(why) => {
            let (status, body) = crate::detail::admission_refused(
                "pull refusal read",
                crate::detail::MAX_STORE_READ_CONCURRENT,
                &why,
            );
            (status, json_header(), body)
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "a test that cannot panic cannot fail, and these lints exist to \
              keep panics out of the crate rather than out of its tests"
)]
mod tests {
    use super::*;

    #[test]
    fn the_question_is_read_or_refused_by_name() {
        assert_eq!(
            asked("feed=zerodha&symbols=NIFTY,BANKNIFTY"),
            Ok(vec!["NIFTY".to_owned(), "BANKNIFTY".to_owned()])
        );
        assert!(
            asked("symbols=NIFTY")
                .unwrap_err()
                .contains("never guesses")
        );
        assert!(
            asked("feed=gdfl&symbols=NIFTY")
                .unwrap_err()
                .contains("/imports.json")
        );
        assert!(
            asked("feed=zerodha")
                .unwrap_err()
                .contains("needs symbols=")
        );
        assert!(
            asked("feed=zerodha&symbols=")
                .unwrap_err()
                .contains("needs symbols=")
        );
        assert!(
            asked("feed=zerodha&symbols=,,")
                .unwrap_err()
                .contains("needs symbols=")
        );
        assert!(asked("feed=zerodha&symbols=..%2Fx").is_err());
        assert!(
            asked("feed=zerodha&symbols=NIFTY,NIFTY")
                .unwrap_err()
                .contains("twice")
        );
        let many = (0..=MAX_SYMBOLS)
            .map(|n| format!("S{n}"))
            .collect::<Vec<_>>()
            .join(",");
        assert!(
            asked(&format!("feed=zerodha&symbols={many}"))
                .unwrap_err()
                .contains("at most")
        );
        let most = (0..MAX_SYMBOLS)
            .map(|n| format!("S{n}"))
            .collect::<Vec<_>>()
            .join(",");
        assert_eq!(
            asked(&format!("feed=zerodha&symbols={most}")).map(|list| list.len()),
            Ok(MAX_SYMBOLS)
        );
        assert_eq!(
            asked("feed=zerodha&symbols=M%26M"),
            Ok(vec!["M&M".to_owned()])
        );
    }

    /// The list is ONE field, so the server's refusal of a repeated query
    /// field (D-1202) never meets it: the question a page sends for fifty
    /// rows passes the bound that refused `symbol=A&symbol=B`, which is how
    /// this route's first form failed on a real server.
    #[test]
    fn a_fifty_row_question_passes_the_servers_own_query_bounds() {
        let list = (0..50)
            .map(|n| format!("SYM{n}"))
            .collect::<Vec<_>>()
            .join(",");
        let uri: axum::http::Uri = format!("/ingest/refusals.json?feed=zerodha&symbols={list}")
            .parse()
            .expect("a uri");
        assert_eq!(
            crate::server::request_bounds_refusal(&uri, &axum::http::HeaderMap::new()),
            None
        );
        let repeated: axum::http::Uri = "/ingest/refusals.json?feed=zerodha&symbol=A&symbol=B"
            .parse()
            .expect("a uri");
        assert!(
            crate::server::request_bounds_refusal(&repeated, &axum::http::HeaderMap::new())
                .is_some(),
            "a repeated field is still refused: the list is one field by design"
        );
    }

    /// A store with no pull journal answers every symbol with no refusal and
    /// says the journal is absent; nothing is invented.
    #[test]
    fn no_journal_is_said_not_guessed() {
        let root = crate::scratch::path("ingest-refusals-none");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("mkdir");
        let body = answer(&root, &["NIFTY".to_owned()]);
        assert_eq!(body["symbols"][0]["symbol"], "NIFTY");
        assert_eq!(body["symbols"][0]["count"], 0);
        assert_eq!(body["symbols"][0]["months"], json!({}));
        assert_eq!(body["symbols"][0]["whole_count"], 0);
        assert_eq!(body["source"]["present"], false);
        assert_eq!(body["source"]["via"]["kind"], "absent");
        assert!(body["rung"].is_null());
        assert!(
            body["rung_note"]
                .as_str()
                .unwrap_or_default()
                .contains("no timeframe")
        );
    }

    fn refusal(record: u64, day: Option<&str>, whole: bool, note: &str) -> Value {
        json!({
            "record": record,
            "at_unix_secs": 1_791_000_000_u64 + record,
            "window": { "from": "2018-01-01", "to": "2026-09-30" },
            "note": note,
            "cut": note.len() >= 60,
            "names_day": day,
            "whole_window": whole,
        })
    }

    /// The fold: one object per month, the newest record's note, at most
    /// `MAX_DAYS` days with the rest counted, whole-window refusals newest
    /// first with the rest counted. Empty, single, maximum and past-maximum.
    #[test]
    fn refusals_fold_to_one_bounded_object_per_month() {
        assert_eq!(folded("X", &[])["months"], json!({}));
        let one = folded(
            "X",
            &[refusal(
                7,
                Some("2020-11-14"),
                false,
                "2020-11-14: refused a",
            )],
        );
        assert_eq!(one["count"], 1);
        assert_eq!(one["months"]["2020-11"]["count"], 1);
        assert_eq!(one["months"]["2020-11"]["days"], json!(["2020-11-14"]));
        assert_eq!(one["months"]["2020-11"]["record"], 7);
        assert_eq!(one["whole_count"], 0);

        let mut many = Vec::new();
        // Six distinct days in one month, one of them twice, oldest first.
        for (n, day) in ["01", "02", "03", "04", "05", "06", "01"]
            .iter()
            .enumerate()
        {
            let day = format!("2021-11-{day}");
            many.push(refusal(
                10 + u64::try_from(n).expect("small"),
                Some(&day),
                false,
                &format!("{day}: refused {n}"),
            ));
        }
        // Another month, and six whole-window refusals (one names a day too).
        many.push(refusal(
            30,
            Some("2023-11-12"),
            false,
            "2023-11-12: refused",
        ));
        for n in 0..6_u64 {
            many.push(refusal(
                40 + n,
                (n == 5).then_some("2024-11-01"),
                true,
                "the whole window was refused",
            ));
        }
        let out = folded("X", &many);
        assert_eq!(out["count"], 14);
        let nov21 = &out["months"]["2021-11"];
        assert_eq!(nov21["count"], 7);
        assert_eq!(
            nov21["days"],
            json!(["2021-11-01", "2021-11-02", "2021-11-03", "2021-11-04"])
        );
        assert_eq!(
            nov21["more_day_records"], 2,
            "the 5th and 6th; the repeat of the 1st is listed"
        );
        assert_eq!(nov21["record"], 16, "the newest record of the month");
        assert_eq!(nov21["note"], "2021-11-01: refused 6");
        assert_eq!(out["months"]["2023-11"]["count"], 1);
        assert_eq!(
            out["months"]["2024-11"]["count"], 1,
            "a whole-window refusal that names a day"
        );
        assert_eq!(out["whole_count"], 6);
        assert_eq!(out["whole_more"], 2);
        let listed = out["whole"].as_array().expect("a list");
        assert_eq!(listed.len(), MAX_WHOLE);
        assert_eq!(listed[0]["record"], 45, "newest first");
        assert_eq!(listed[3]["record"], 42);
    }

    /// A Zerodha run's refused day is read back under the symbol it pulled,
    /// with its month: the 2020-11-14 Muhurat refusal that made /ingest say
    /// "never pulled" for a month the run had asked for.
    #[test]
    fn a_journalled_refusal_is_named_with_its_month() {
        let root = crate::scratch::path("ingest-refusals-one");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("audit")).expect("mkdir");
        let journal = crate::audit::Journal::at(&root);
        let at = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_791_266_622);
        let day =
            |text: &str| crate::server::import_journal::parse_day(text).expect("a fixture day");
        journal
            .append(&crate::audit::Record::refused(
                crate::audit::Scope::Spot,
                crate::audit::Outcome::Failed,
                at,
                "zerodha_nse_checked fixture run",
                "fixture run",
            ))
            .expect("a run record");
        journal
            .append(&crate::audit::Record::member_failure(
                crate::audit::Scope::Spot,
                at,
                "NIFTY",
                pull::session::Window::new(day("2018-01-01"), day("2026-09-30")).expect("a window"),
                "2020-11-14: 60 minute row(s) refused: the exchange calendar",
            ))
            .expect("a member failure");
        let body = answer(&root, &["NIFTY".to_owned(), "BANKNIFTY".to_owned()]);
        assert_eq!(body["errors"], json!([]));
        assert_eq!(body["symbols"][0]["count"], 1, "{body}");
        assert_eq!(
            body["symbols"][0]["months"]["2020-11"]["count"], 1,
            "{body}"
        );
        assert_eq!(
            body["symbols"][0]["months"]["2020-11"]["days"],
            json!(["2020-11-14"]),
            "{body}"
        );
        assert_eq!(
            body["symbols"][0]["months"]["2020-11"]["window"]["from"], "2018-01-01",
            "{body}"
        );
        assert_eq!(
            body["symbols"][1]["count"], 0,
            "another symbol's refusal is not this one's"
        );
    }
}

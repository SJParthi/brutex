//! Serves one run's round trips to the console.
//!
//! # What this replaces, and why none of it survived
//!
//! This file held a 1,033-line per-trade STORE — magic, stride, `append`,
//! `read_block`, a block index, a JSON serialiser — with **zero production
//! callers on either side**. Nothing wrote it and nothing read it, while
//! `web/src/routes/backtest/+page.svelte` drew a per-trade table whose every
//! cell rendered a padlock: *"No trade number — no trade list is recorded."*
//!
//! It could not be repaired where it stood. Three reasons, and the first is
//! structural:
//!
//! * **The writer cannot live here.** The crate arrow runs `api -> cli`
//!   (`CLAUDE.md` §5), and the trades exist inside `cli::audit_bars`. A store in
//!   `api` is unreachable from the code that has the data.
//! * Its block index was built as `HashMap::new()` on *both* open paths, so
//!   after any restart every lookup answered "no such run" while the module's
//!   own header claimed an O(1) find.
//! * Its record carried a bare sequence number and no run identity, so the index
//!   was **unreconstructible** — nothing on disk said which run a row belonged
//!   to, and no amount of reading could rebuild what was never written.
//!
//! The store now lives in [`cli::trades`], modelled on `cli::frontier`, which is
//! the identity-keyed detail file that already worked end to end. This file is
//! what it always should have been: a **reader**, on the side of the arrow that
//! can serve HTTP.
//!
//! # Cost
//!
//! After its indexes exist, one hash probe finds the run's block and reading it
//! is linear in the selected rows, which are the answer itself. This HTTP path
//! reads/indexes the ledger and receipts once to obtain one canonical commit
//! snapshot, then opens afresh and indexes all trade rows before reconciling
//! that exact receipt's count and direction. Request setup is therefore
//! O(total trades + total runs + total receipts), not O(1); only each in-memory
//! identity probe is O(1). D-0404 and limits §98 name that bound rather than
//! turning local lookup shape into a latency claim. The HTTP boundary makes
//! those linear terms finite: each indexed file is at most 64 MiB, one verified
//! result at most 4,096 rows, one response page at most 256 rows and encoded
//! JSON at most 8 MiB. Four detail tasks across both routes may be queued or
//! running. All filesystem/index/encoding work runs through `spawn_blocking`;
//! saturation answers 429 before another blocking task is queued. D-0435 and
//! limits §125.

use std::path::PathBuf;

/// The headers every JSON route here answers with.
type JsonHeaders = [(axum::http::header::HeaderName, &'static str); 1];

/// One run's round trips, as JSON.
///
/// `identity` is required and is the 64-character hex the ledger already emits
/// on every row, so the page has the key in hand before it asks. Without it
/// there is no question to answer: this file holds many runs and "the trades"
/// is not a request.
pub async fn trades_json(uri: axum::http::Uri) -> (axum::http::StatusCode, JsonHeaders, String) {
    let query = uri.query().unwrap_or("");
    if let Err(why) = crate::detail::query_is_bounded(query) {
        return refuse(json_headers(), &why);
    }
    let query = query.to_owned();
    let root = crate::server::store_dir();
    match crate::detail::run(move || respond(root, &query)).await {
        Ok(response) => response,
        Err(crate::detail::RunError::Saturated) => unavailable(
            axum::http::StatusCode::TOO_MANY_REQUESTS,
            "detail read capacity is full; no blocking task was queued. Retry after another trade/frontier request finishes",
        ),
        Err(crate::detail::RunError::Join(why)) => unavailable(
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            &format!("the bounded trade-detail task could not be joined: {why}"),
        ),
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "one ordered receipt-child-validation-page transaction keeps every early refusal beside the operation it protects"
)]
fn respond(
    root: Result<PathBuf, String>,
    query: &str,
) -> (axum::http::StatusCode, JsonHeaders, String) {
    let json = json_headers();

    let page = match crate::detail::Page::parse(query) {
        Ok(page) => page,
        Err(why) => return refuse(json, &why),
    };

    let root = match root {
        Ok(root) => root,
        Err(why) => return refuse(json, &why),
    };

    let raw = crate::server::param(query, "identity");
    let Some(identity) = from_hex(&raw) else {
        return refuse(
            json,
            "`identity` must be the 64 hex characters `/backtest.json` prints on \
             every row. This file holds many runs, so which one is not a detail \
             it can infer.",
        );
    };

    let ledger_path = cli::results::Results::path(&root);
    let receipt_path = cli::result_set::Receipts::path(&root);
    let trades_path = cli::trades::Trades::path(&root);
    if let Err(why) = crate::detail::preflight([
        ("results ledger", ledger_path.as_path()),
        ("detail receipts", receipt_path.as_path()),
        ("chosen trades", trades_path.as_path()),
    ]) {
        return unavailable(axum::http::StatusCode::SERVICE_UNAVAILABLE, &why);
    }

    // ONE CANONICAL RECEIPT SNAPSHOT, BEFORE THE CHILD IS OPENED. Re-reading it
    // after `open_read` would let metadata from one instant label a block index
    // built at another; a commit that races this request is either wholly in
    // this snapshot or wholly left for the next request.
    let receipt = match cli::result_set::committed_receipt_bounded(
        &root,
        &identity,
        crate::detail::MAX_SCAN_BYTES,
    ) {
        Ok(receipt) => receipt,
        Err(why) => return refuse(json, &why),
    };
    if let Some(committed) = receipt
        && committed.trade_rows > crate::detail::MAX_RESULT_ROWS
    {
        return too_large(
            json,
            &identity,
            committed.trade_rows,
            &format!(
                "run {} commits {} chosen-trade rows; one request verifies at most {}. No prefix was read or exposed",
                crate::server::hex32(identity),
                committed.trade_rows,
                crate::detail::MAX_RESULT_ROWS
            ),
        );
    }

    // OPENED FRESH AND READ-ONLY AFTER THE RECEIPT, AND THAT ORDER IS THE POINT.
    // A GET that creates its own empty file answers "no trades" by MAKING that
    // true, which is the failure §4 bans. `of_run_against_receipt` reconciles
    // this newly indexed child against the one snapshot above and never opens
    // the receipt sidecar again.
    let mut file =
        match cli::trades::Trades::open_read_bounded(&root, crate::detail::MAX_SCAN_BYTES) {
            Ok(file) => file,
            Err(why) => {
                return missing_file_response(&root, &identity, receipt, page, json, &why);
            }
        };

    let rows = match file.of_run_against_receipt(&identity, receipt) {
        Ok(rows) => rows,
        Err(why) => return refuse(json, &why),
    };
    let Some(receipt) = receipt else {
        return absent_response(
            json,
            &identity,
            page,
            &format!(
                "run {} has no committed result-set receipt at this request's canonical read. No clean empty or orphan chosen-trade answer is exposed",
                crate::server::hex32(identity)
            ),
        );
    };

    let window = match crate::detail::window(rows.len(), page) {
        Ok(window) => window,
        Err(why) => return range_refusal(json, &identity, rows.len(), &why),
    };
    let Some(page_rows) = rows.get(window.start..window.end) else {
        return refuse(
            json,
            "the validated trade page window did not fit its verified row vector; no fallback page is exposed",
        );
    };

    let mut out = String::with_capacity(page_rows.len().saturating_mul(256).saturating_add(512));
    let _ = std::fmt::Write::write_fmt(
        &mut out,
        format_args!(
            r#"{{"identity":"{}","policy":"{}","direction":"{}","trades":["#,
            crate::server::hex32(identity),
            receipt.trade_policy.as_str(),
            receipt.direction
        ),
    );
    for (at, row) in page_rows.iter().enumerate() {
        if at > 0 {
            out.push(',');
        }
        // HELD IN THE UNITS THE STORE HOLDS THEM IN. `best` and `worst` are
        // paisa and stay paisa; the page divides. Converting here would put a
        // second opinion about the unit on the wire, which is the two-copies
        // shape §5 refuses -- and `CLAUDE.md` §7 keeps prices integral.
        let _ = std::fmt::Write::write_fmt(
            &mut out,
            format_args!(
                r#"{{"seq":{},"direction":"{}","signal_bar":{},"entry_bar":{},"exit_bar":{},"best":{},"worst":{},"bars_held":{},"entry_micros":{},"exit_micros":{},"adverse_ppm":{},"adverse_paisa":{},"favourable_ppm":{},"favourable_paisa":{}}}"#,
                row.seq,
                row.direction,
                row.signal_bar,
                row.entry_bar,
                row.exit_bar,
                row.best,
                row.worst,
                row.exit_bar.saturating_sub(row.entry_bar),
                row.entry_micros,
                row.exit_micros,
                row.adverse_ppm,
                row.adverse_paisa,
                row.favourable_ppm,
                row.favourable_paisa,
            ),
        );
    }
    out.push_str(r#"],"periods":"#);
    append_periods(&mut out, &rows);
    let partial = (!window.complete).then(|| {
        format!(
            "partial page only: page {} returns rows {}..{} of {}. Fetch every page and reconcile `total_count`; this response is not a complete trade list",
            page.number,
            window.start,
            window.end,
            rows.len()
        )
    });
    let _ = std::fmt::Write::write_fmt(
        &mut out,
        format_args!(
            r#","count":{},"total_count":{},"page":{},"limit":{},"page_complete":true,"complete":{},"next_page":{},"periods_scope":"complete-result","refusal":{}}}"#,
            page_rows.len(),
            rows.len(),
            page.number,
            page.limit,
            window.complete,
            window
                .next_page
                .map_or_else(|| "null".to_owned(), |next| next.to_string()),
            partial.map_or_else(|| "null".to_owned(), |why| crate::render::json_string(&why))
        ),
    );
    if out.len() > crate::detail::MAX_RESPONSE_BYTES {
        return too_large(
            json,
            &identity,
            u64::try_from(rows.len()).unwrap_or(u64::MAX),
            &format!(
                "the bounded trade response encoded to {} bytes; the hard response ceiling is {}. No oversized JSON was returned",
                out.len(),
                crate::detail::MAX_RESPONSE_BYTES
            ),
        );
    }
    let status = if window.complete {
        axum::http::StatusCode::OK
    } else {
        axum::http::StatusCode::PARTIAL_CONTENT
    };
    (status, json, out)
}

/// Writes every period key, including eight empty arrays for an empty block.
///
/// The missing-zero-child response uses this same writer as a present file, so
/// those two honest empty states cannot drift into different JSON schemas.
fn append_periods(out: &mut String, rows: &[cli::trades::Row]) {
    out.push('{');
    // EVERY CALENDAR PERIOD, BUILT IN ONE PASS AND SERVED WITH THE ROWS.
    //
    // The operator's question: *"every day how many wins how many loss every
    // week every month every quarter every half every year ... we need to always
    // know whether this combination made how much of all these"*.
    //
    // Served ALONGSIDE the rows rather than behind a second route, because the
    // page needs both together and two routes would let them disagree — a
    // reader could hold last week's buckets beside this week's trades and have
    // no way to tell. One response, one pass, one run.
    //
    // The aggregation lives in `cli::trades::by_period` rather than here for the
    // reason `/frontier.json` gives about `Row::derived`: `api` must not become
    // a second place that decides what a "win" is.
    for (at, (period, buckets)) in cli::trades::by_period(rows).iter().enumerate() {
        if at > 0 {
            out.push(',');
        }
        let _ = std::fmt::Write::write_fmt(&mut *out, format_args!(r#""{}":["#, period.name()));
        for (n, b) in buckets.iter().enumerate() {
            if n > 0 {
                out.push(',');
            }
            let _ = std::fmt::Write::write_fmt(
                &mut *out,
                format_args!(
                    r#"{{"key":{},"trades":{},"wins":{},"worst_wins":{},"best_paisa":{},"worst_paisa":{},"largest_win":{},"largest_loss":{}}}"#,
                    b.key,
                    b.trades,
                    b.wins,
                    b.worst_wins,
                    b.best_paisa,
                    b.worst_paisa,
                    b.largest_win,
                    b.largest_loss,
                ),
            );
        }
        out.push(']');
    }
    out.push('}');
}

/// Distinguishes an unswept store from a committed child deleted afterwards.
///
/// A missing path is a complete empty answer only when the sealed manifest says
/// this run committed zero trade rows. A positive committed count is corruption
/// and therefore receives the same null-list refusal as a damaged row.
fn missing_file_response(
    root: &std::path::Path,
    identity: &[u8; 32],
    receipt: Option<cli::result_set::Receipt>,
    page: crate::detail::Page,
    json: JsonHeaders,
    why: &str,
) -> (axum::http::StatusCode, JsonHeaders, String) {
    let child = cli::trades::Trades::path(root);
    match std::fs::metadata(&child) {
        Err(missing) if missing.kind() == std::io::ErrorKind::NotFound => {}
        Ok(_) => {
            return refuse(
                json,
                &format!(
                    "{why}. {} still exists, so its open failure is corruption or unreadability rather than an absent zero-row child",
                    child.display()
                ),
            );
        }
        Err(unreadable) => {
            return refuse(
                json,
                &format!(
                    "{why}. {} could not be checked for absence: {unreadable}",
                    child.display()
                ),
            );
        }
    }
    match receipt {
        Some(receipt) if receipt.trade_rows == 0 => {
            if let Err(range) = crate::detail::window(0, page) {
                return range_refusal(json, identity, 0, &range);
            }
            let mut out = format!(
                r#"{{"identity":"{}","policy":"{}","direction":"{}","trades":[],"periods":"#,
                crate::server::hex32(*identity),
                receipt.trade_policy.as_str(),
                receipt.direction
            );
            append_periods(&mut out, &[]);
            let _ = std::fmt::Write::write_fmt(
                &mut out,
                format_args!(
                    r#","count":0,"total_count":0,"page":{},"limit":{},"page_complete":true,"complete":true,"next_page":null,"periods_scope":"complete-result","refusal":null}}"#,
                    page.number, page.limit
                ),
            );
            (axum::http::StatusCode::OK, json, out)
        }
        Some(receipt) => refuse(
            json,
            &format!(
                "run {} is committed for {} chosen-trade row(s), but chosen-trades.bin is missing: {why}. No empty or partial answer is exposed",
                crate::server::hex32(*identity),
                receipt.trade_rows
            ),
        ),
        None => absent_response(json, identity, page, why),
    }
}

/// An explicitly uncommitted/absent child, never a clean committed empty set.
fn absent_response(
    json: JsonHeaders,
    identity: &[u8; 32],
    page: crate::detail::Page,
    why: &str,
) -> (axum::http::StatusCode, JsonHeaders, String) {
    if let Err(range) = crate::detail::window(0, page) {
        return range_refusal(json, identity, 0, &range);
    }
    let mut out = format!(
        r#"{{"identity":"{}","policy":null,"direction":null,"trades":[],"periods":"#,
        crate::server::hex32(*identity)
    );
    append_periods(&mut out, &[]);
    let _ = std::fmt::Write::write_fmt(
        &mut out,
        format_args!(
            r#","count":0,"total_count":0,"page":{},"limit":{},"page_complete":true,"complete":true,"next_page":null,"periods_scope":"complete-result","refusal":{}}}"#,
            page.number,
            page.limit,
            crate::render::json_string(why)
        ),
    );
    (axum::http::StatusCode::OK, json, out)
}

/// A refusal that names its cause, in the shape every other route here uses.
fn refuse(json: JsonHeaders, why: &str) -> (axum::http::StatusCode, JsonHeaders, String) {
    (
        axum::http::StatusCode::BAD_REQUEST,
        json,
        format!(
            r#"{{"policy":null,"direction":null,"trades":null,"periods":null,"count":0,"total_count":null,"page_complete":false,"complete":false,"refusal":{}}}"#,
            crate::render::json_string(why)
        ),
    )
}

fn json_headers() -> JsonHeaders {
    [(
        axum::http::header::CONTENT_TYPE,
        "application/json; charset=utf-8",
    )]
}

fn unavailable(
    status: axum::http::StatusCode,
    why: &str,
) -> (axum::http::StatusCode, JsonHeaders, String) {
    (
        status,
        json_headers(),
        format!(
            r#"{{"policy":null,"direction":null,"trades":null,"periods":null,"count":0,"total_count":null,"page_complete":false,"complete":false,"refusal":{}}}"#,
            crate::render::json_string(why)
        ),
    )
}

fn too_large(
    json: JsonHeaders,
    identity: &[u8; 32],
    total: u64,
    why: &str,
) -> (axum::http::StatusCode, JsonHeaders, String) {
    (
        axum::http::StatusCode::PAYLOAD_TOO_LARGE,
        json,
        format!(
            r#"{{"identity":"{}","policy":null,"direction":null,"trades":null,"periods":null,"count":0,"total_count":null,"claimed_total":"{}","page_complete":false,"complete":false,"refusal":{}}}"#,
            crate::server::hex32(*identity),
            total,
            crate::render::json_string(why)
        ),
    )
}

fn range_refusal(
    json: JsonHeaders,
    identity: &[u8; 32],
    total: usize,
    why: &str,
) -> (axum::http::StatusCode, JsonHeaders, String) {
    (
        axum::http::StatusCode::RANGE_NOT_SATISFIABLE,
        json,
        format!(
            r#"{{"identity":"{}","policy":null,"direction":null,"trades":null,"periods":null,"count":0,"total_count":{},"page_complete":false,"complete":false,"refusal":{}}}"#,
            crate::server::hex32(*identity),
            total,
            crate::render::json_string(why)
        ),
    )
}

/// Sixty-four hex characters as thirty-two bytes, or `None`.
///
/// Shared with `crate::frontierjson`, which keys on the same identity: two hex
/// decoders would be two chances to disagree about what a run is called.
///
/// `#[must_use]` because the whole value is the answer: this has no side
/// effect, so a call whose result is dropped decoded a run identity and threw
/// it away — which is a bug at every call site rather than a style preference.
#[must_use]
pub fn from_hex_public(text: &str) -> Option<[u8; 32]> {
    from_hex(text)
}

/// Sixty-four hex characters as thirty-two bytes, or `None`.
///
/// Length is checked BEFORE the digits, because a short identity is the common
/// mistake — a truncated copy-paste — and it deserves the same refusal as a
/// malformed one rather than a partial match against a run it is a prefix of.
fn from_hex(text: &str) -> Option<[u8; 32]> {
    let text = text.trim();
    if text.len() != 64 {
        return None;
    }
    let bytes = text.as_bytes();
    let mut out = [0_u8; 32];
    for (at, slot) in out.iter_mut().enumerate() {
        let hi = nibble(*bytes.get(at.checked_mul(2)?)?)?;
        let lo = nibble(*bytes.get(at.checked_mul(2)?.checked_add(1)?)?)?;
        *slot = hi.checked_shl(4)?.checked_add(lo)?;
    }
    Some(out)
}

/// One hex digit, upper or lower case.
const fn nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]
mod tests {
    use super::{from_hex, respond, trades_json};

    fn assert_body_identity(body: &str, expected: &str) {
        let prefix = format!(r#"{{"identity":"{expected}","#);
        assert!(
            body.starts_with(&prefix),
            "the top-level response must bind itself to {expected}: {body}"
        );
        assert_eq!(
            body.matches(r#""identity":"#).count(),
            1,
            "one response carries one canonical identity: {body}"
        );
    }

    fn trade_row(identity: [u8; 32], seq: u32) -> cli::trades::Row {
        cli::trades::Row {
            identity,
            seq,
            direction: cli::trades::Direction::Short,
            signal_bar: u64::from(seq),
            entry_bar: u64::from(seq).saturating_add(1),
            exit_bar: u64::from(seq).saturating_add(2),
            best: 100,
            worst: -50,
            entry_micros: 60_000_000,
            exit_micros: 120_000_000,
            adverse_ppm: 2_500,
            adverse_paisa: 25,
            favourable_ppm: 12_500,
            favourable_paisa: 125,
        }
    }

    fn commit_trade_fixture(dir: &std::path::Path, identity: [u8; 32], trade_rows: u64) {
        cli::result_set::Receipts::open(dir)
            .expect("receipt store")
            .append_exact(cli::result_set::Receipt {
                identity,
                frontier_rows: 0,
                trade_rows,
                direction: cli::trades::Direction::Short,
                trade_policy: cli::result_set::TradePolicy::ChosenGridV1,
            })
            .expect("receipt");
        cli::results::Results::open(dir)
            .expect("ledger")
            .append(&cli::results::Record {
                identity,
                finished_micros: 1,
                feed: cli::results::field("zerodha"),
                underlying: cli::results::field("NIFTY"),
                timeframe: cli::results::field("15min"),
                from_year: 2026,
                from_month: 1,
                to_year: 2026,
                to_month: 1,
                months_asked: 1,
                months_found: 1,
                bars: 10,
                min_hits: 1,
                combinations: 0,
                depth: 0,
                halted: 0,
                trades: 999,
                pessimistic: 0,
                optimistic: 0,
                worst_trade: 0,
                max_drawdown: 0,
                winner_mae: 0,
                winner_mfe: 0,
                all_mae: 0,
                exit_rungs: [-1; 5],
                mask_words: [0; 6],
            })
            .expect("parent last");
    }

    #[test]
    fn an_identity_must_be_sixty_four_hex_characters() {
        assert!(from_hex(&"a".repeat(64)).is_some(), "64 lowercase hex");
        assert!(from_hex(&"A".repeat(64)).is_some(), "upper case too");
        assert_eq!(from_hex(&"a".repeat(63)), None, "one short is refused");
        assert_eq!(from_hex(&"a".repeat(65)), None, "one long is refused");
        assert_eq!(from_hex(&"z".repeat(64)), None, "non-hex is refused");
        assert_eq!(from_hex(""), None, "absent is refused");
    }

    /// The bytes must come back in the order they were written, not reversed.
    #[test]
    fn hex_decodes_in_order() {
        let mut text = String::new();
        for byte in 0..32_u8 {
            let _ = std::fmt::Write::write_fmt(&mut text, format_args!("{byte:02x}"));
        }
        let got = from_hex(&text).expect("32 bytes of hex");
        assert_eq!(got.first(), Some(&0));
        assert_eq!(got.get(31), Some(&31), "the last byte is the last pair");
    }

    /// A STORE THAT HAS NEVER BEEN SWEPT IS AN EMPTY ANSWER, NOT A 500.
    ///
    /// A broken server and an empty store must not look alike to the page, and
    /// the direction matters: reporting "no trades" for a server fault would
    /// hide it, so the refusal travels beside the empty list rather than
    /// replacing it.
    #[test]
    fn an_absent_file_answers_empty_and_names_why() {
        let dir = std::env::temp_dir().join(format!("brutex-api-trades-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let query = format!("identity={}", "a".repeat(64));
        let (status, _, body) = respond(Ok(dir.clone()), &query);
        assert_eq!(status, axum::http::StatusCode::OK);
        assert_body_identity(&body, &"a".repeat(64));
        assert!(body.contains(r#""trades":[]"#), "an empty list: {body}");
        assert!(body.contains(r#""count":0"#));
        for period in [
            "day", "week", "month", "quarter", "half", "year", "weekday", "hour",
        ] {
            assert!(
                body.contains(&format!(r#""{period}":[]"#)),
                "empty period {period}: {body}"
            );
        }
        assert!(
            !body.contains(r#""refusal":null"#),
            "and it says why: {body}"
        );
        assert!(
            !cli::trades::Trades::path(&dir).exists(),
            "and reading must not have created the file"
        );
    }

    #[test]
    fn a_present_child_without_the_canonical_receipt_is_never_clean_empty() {
        let dir = std::env::temp_dir().join(format!(
            "brutex-api-trades-uncommitted-empty-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        cli::trades::Trades::open(&dir).expect("present chosen-trade header");

        let query = format!("identity={}", "6a".repeat(32));
        let (status, _, body) = respond(Ok(dir.clone()), &query);
        assert_eq!(status, axum::http::StatusCode::OK);
        assert_body_identity(&body, &"6a".repeat(32));
        assert!(body.contains(r#""trades":[]"#), "explicit empty: {body}");
        assert!(body.contains(r#""policy":null"#), "not committed: {body}");
        assert!(
            body.contains(r#""direction":null"#),
            "not committed: {body}"
        );
        assert!(
            !body.contains(r#""refusal":null"#),
            "absence must never masquerade as a committed clean empty block: {body}"
        );
        assert!(body.contains("no committed result-set receipt"), "{body}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_receipt_is_read_once_before_the_fresh_child_and_never_inside_missing_handling() {
        let source = include_str!("trades.rs");
        let respond = source
            .split_once("fn respond(")
            .and_then(|(_, after)| after.split_once("fn append_periods(").map(|(body, _)| body))
            .expect("respond body");
        assert_eq!(
            respond.matches("committed_receipt_bounded(").count(),
            1,
            "one request owns one canonical receipt snapshot"
        );
        let receipt = respond
            .find("committed_receipt_bounded(")
            .expect("canonical receipt read");
        let child = respond
            .find("Trades::open_read_bounded")
            .expect("fresh child open");
        let reconcile = respond
            .find("of_run_against_receipt(&identity, receipt)")
            .expect("exact snapshot reconciliation");
        assert!(
            receipt < child && child < reconcile,
            "receipt → child → rows"
        );

        let missing = source
            .split_once("fn missing_file_response(")
            .and_then(|(_, after)| {
                after
                    .split_once("fn absent_response(")
                    .map(|(body, _)| body)
            })
            .expect("missing-child body");
        assert!(
            !missing.contains("committed_receipt"),
            "the error path must retain the same snapshot too"
        );
    }

    /// A missing or malformed identity is refused rather than guessed at.
    #[test]
    fn a_request_without_an_identity_is_refused() {
        // Named for this process, for the reason given in `frontierjson`'s twin
        // of this test: gate 23 clause C polices the NAME, not whether this
        // particular assertion reaches the filesystem.
        let dir =
            std::env::temp_dir().join(format!("brutex-api-trades-none-{}", std::process::id()));
        let (status, _, body) = respond(Ok(dir), "");
        assert_eq!(status, axum::http::StatusCode::BAD_REQUEST);
        assert!(
            body.contains("64 hex characters"),
            "names the shape: {body}"
        );
        assert!(body.contains(r#""trades":null"#), "null, not empty: {body}");
    }

    #[test]
    fn a_committed_chosen_trade_exposes_policy_direction_and_both_excursions() {
        let dir = std::env::temp_dir().join(format!(
            "brutex-api-trades-chosen-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let identity = [0x6c; 32];
        cli::trades::Trades::open(&dir)
            .expect("chosen trade store")
            .append_all(&[trade_row(identity, 0)])
            .expect("chosen row");
        commit_trade_fixture(&dir, identity, 1);
        let query = format!("identity={}", "6c".repeat(32));
        let (status, _, body) = respond(Ok(dir.clone()), &query);
        assert_eq!(status, axum::http::StatusCode::OK);
        assert_body_identity(&body, &"6c".repeat(32));
        for fact in [
            r#""policy":"chosen-grid-v1""#,
            r#""direction":"short""#,
            r#""adverse_ppm":2500"#,
            r#""adverse_paisa":25"#,
            r#""favourable_ppm":12500"#,
            r#""favourable_paisa":125"#,
            r#""count":1"#,
        ] {
            assert!(body.contains(fact), "missing {fact}: {body}");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_receipt_direction_that_disagrees_with_its_rows_exposes_nothing() {
        let dir = std::env::temp_dir().join(format!(
            "brutex-api-trades-direction-mismatch-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let identity = [0x6d; 32];
        cli::trades::Trades::open(&dir)
            .expect("chosen trade store")
            .append_all(&[trade_row(identity, 0)])
            .expect("short row");
        cli::result_set::Receipts::open(&dir)
            .expect("receipt store")
            .append_exact(cli::result_set::Receipt {
                identity,
                frontier_rows: 0,
                trade_rows: 1,
                direction: cli::trades::Direction::Long,
                trade_policy: cli::result_set::TradePolicy::ChosenGridV1,
            })
            .expect("opposite receipt direction");
        let mut record = cli::results::Results::open(&dir).expect("ledger");
        let parent = cli::results::Record {
            identity,
            finished_micros: 1,
            feed: cli::results::field("zerodha"),
            underlying: cli::results::field("NIFTY"),
            timeframe: cli::results::field("15min"),
            from_year: 2026,
            from_month: 1,
            to_year: 2026,
            to_month: 1,
            months_asked: 1,
            months_found: 1,
            bars: 10,
            min_hits: 1,
            combinations: 0,
            depth: 0,
            halted: 0,
            trades: 1,
            pessimistic: 0,
            optimistic: 0,
            worst_trade: 0,
            max_drawdown: 0,
            winner_mae: 0,
            winner_mfe: 0,
            all_mae: 0,
            exit_rungs: [-1; 5],
            mask_words: [0; 6],
        };
        record.append(&parent).expect("parent last");
        let query = format!("identity={}", "6d".repeat(32));
        let (status, _, body) = respond(Ok(dir.clone()), &query);
        assert_eq!(status, axum::http::StatusCode::BAD_REQUEST);
        assert!(body.contains(r#""trades":null"#), "{body}");
        assert!(body.contains("selected direction long"), "{body}");
        assert!(body.contains("row 0 says short"), "{body}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_committed_corrupt_trade_block_exposes_no_valid_prefix() {
        let dir = std::env::temp_dir().join(format!(
            "brutex-api-trades-corrupt-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let identity = [0x7c; 32];
        cli::trades::Trades::open(&dir)
            .expect("trade store")
            .append_all(&[
                trade_row(identity, 0),
                trade_row(identity, 1),
                trade_row(identity, 2),
            ])
            .expect("three rows");
        commit_trade_fixture(&dir, identity, 3);

        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .open(cli::trades::Trades::path(&dir))
            .expect("trade bytes");
        let stride = u64::try_from(trade_row(identity, 0).to_bytes().len()).expect("row stride");
        std::io::Seek::seek(&mut file, std::io::SeekFrom::Start(16 + stride)).expect("second row");
        std::io::Write::write_all(&mut file, &[0xff]).expect("corrupts seal");
        file.sync_all().expect("durable fixture");

        let query = format!("identity={}", "7c".repeat(32));
        let (status, _, body) = respond(Ok(dir.clone()), &query);
        assert_eq!(status, axum::http::StatusCode::BAD_REQUEST);
        assert!(body.contains(r#""trades":null"#), "no prefix: {body}");
        assert!(body.contains("No partial trade"), "cause: {body}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A WHOLE CHILD FILE LOST AFTER COMMIT IS CORRUPTION, NOT ABSENCE.
    ///
    /// The explicit zero-count receipt is the only case in which that same
    /// missing pathname is a complete empty answer.
    #[test]
    fn a_deleted_committed_trade_file_refuses_unless_its_receipted_count_is_zero() {
        let dir = std::env::temp_dir().join(format!(
            "brutex-api-trades-deleted-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);

        let populated = [0x7d; 32];
        cli::trades::Trades::open(&dir)
            .expect("trade store")
            .append_all(&[trade_row(populated, 0)])
            .expect("one row");
        commit_trade_fixture(&dir, populated, 1);
        std::fs::remove_file(cli::trades::Trades::path(&dir)).expect("delete child");

        let query = format!("identity={}", "7d".repeat(32));
        let (status, _, body) = respond(Ok(dir.clone()), &query);
        assert_eq!(status, axum::http::StatusCode::BAD_REQUEST);
        assert!(body.contains(r#""trades":null"#), "not empty: {body}");
        assert!(
            body.contains("committed for 1 chosen-trade row"),
            "cause: {body}"
        );
        assert!(body.contains(&"7d".repeat(32)), "identity: {body}");

        let empty_dir = dir.join("empty");
        let empty = [0x7e; 32];
        cli::trades::Trades::open(&empty_dir).expect("empty trade header");
        commit_trade_fixture(&empty_dir, empty, 0);
        std::fs::remove_file(cli::trades::Trades::path(&empty_dir)).expect("delete empty child");

        let query = format!("identity={}", "7e".repeat(32));
        let (status, _, body) = respond(Ok(empty_dir), &query);
        assert_eq!(status, axum::http::StatusCode::OK);
        assert_body_identity(&body, &"7e".repeat(32));
        assert!(body.contains(r#""trades":[]"#), "empty: {body}");
        for period in [
            "day", "week", "month", "quarter", "half", "year", "weekday", "hour",
        ] {
            assert!(
                body.contains(&format!(r#""{period}":[]"#)),
                "empty period {period}: {body}"
            );
        }
        assert!(
            body.contains(r#""policy":"chosen-grid-v1""#),
            "policy: {body}"
        );
        assert!(body.contains(r#""direction":"short""#), "direction: {body}");
        assert!(body.contains(r#""refusal":null"#), "committed: {body}");

        std::fs::write(cli::trades::Trades::path(&dir.join("empty")), b"bad")
            .expect("malformed child");
        let query = format!("identity={}", "7e".repeat(32));
        let (status, _, body) = respond(Ok(dir.join("empty")), &query);
        assert_eq!(status, axum::http::StatusCode::BAD_REQUEST);
        assert!(body.contains(r#""trades":null"#), "not absent: {body}");
        assert!(body.contains("still exists"), "cause: {body}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_large_valid_result_is_explicitly_paged_and_never_presented_as_complete() {
        let dir = std::env::temp_dir().join(format!(
            "brutex-api-trades-page-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let identity = [0x81; 32];
        let rows: Vec<_> = (0..300).map(|seq| trade_row(identity, seq)).collect();
        cli::trades::Trades::open(&dir)
            .expect("trade store")
            .append_all(&rows)
            .expect("large valid block");
        commit_trade_fixture(&dir, identity, 300);

        let first_query = format!("identity={}&limit=256", "81".repeat(32));
        let (first_status, _, first) = respond(Ok(dir.clone()), &first_query);
        assert_eq!(first_status, axum::http::StatusCode::PARTIAL_CONTENT);
        let _: serde_json::Value = serde_json::from_str(&first).expect("valid partial JSON");
        assert!(first.len() <= crate::detail::MAX_RESPONSE_BYTES);
        for fact in [
            r#""count":256"#,
            r#""total_count":300"#,
            r#""page":0"#,
            r#""complete":false"#,
            r#""next_page":1"#,
            "partial page only",
        ] {
            assert!(first.contains(fact), "missing {fact}: {first}");
        }

        let second_query = format!("identity={}&page=1&limit=256", "81".repeat(32));
        let (second_status, _, second) = respond(Ok(dir.clone()), &second_query);
        assert_eq!(second_status, axum::http::StatusCode::PARTIAL_CONTENT);
        let _: serde_json::Value = serde_json::from_str(&second).expect("valid partial JSON");
        assert!(second.contains(r#""count":44"#), "{second}");
        assert!(second.contains(r#""total_count":300"#), "{second}");
        assert!(second.contains(r#""next_page":null"#), "{second}");
        assert!(second.contains(r#""complete":false"#), "{second}");

        let out_of_range = format!("identity={}&page=2&limit=256", "81".repeat(32));
        let (status, _, body) = respond(Ok(dir.clone()), &out_of_range);
        assert_eq!(status, axum::http::StatusCode::RANGE_NOT_SATISFIABLE);
        assert!(body.contains(r#""trades":null"#), "{body}");
        assert!(body.contains(r#""complete":false"#), "{body}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_valid_result_beyond_the_hard_row_ceiling_refuses_without_a_prefix() {
        let dir = std::env::temp_dir().join(format!(
            "brutex-api-trades-row-ceiling-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let identity = [0x82; 32];
        let rows: Vec<_> = (0..=crate::detail::MAX_RESULT_ROWS)
            .map(|seq| trade_row(identity, u32::try_from(seq).expect("bounded sequence")))
            .collect();
        cli::trades::Trades::open(&dir)
            .expect("trade store")
            .append_all(&rows)
            .expect("large valid block");
        commit_trade_fixture(
            &dir,
            identity,
            crate::detail::MAX_RESULT_ROWS.saturating_add(1),
        );

        let query = format!("identity={}", "82".repeat(32));
        let (status, _, body) = respond(Ok(dir.clone()), &query);
        assert_eq!(status, axum::http::StatusCode::PAYLOAD_TOO_LARGE);
        assert!(body.contains(r#""trades":null"#), "{body}");
        assert!(body.contains(r#""complete":false"#), "{body}");
        assert!(body.contains("one request verifies at most 4096"), "{body}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_unknown_identity_beside_a_valid_run_exposes_no_foreign_rows() {
        let dir = std::env::temp_dir().join(format!(
            "brutex-api-trades-unknown-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let known = [0x83; 32];
        cli::trades::Trades::open(&dir)
            .expect("trade store")
            .append_all(&[trade_row(known, 0)])
            .expect("known block");
        commit_trade_fixture(&dir, known, 1);

        let query = format!("identity={}", "84".repeat(32));
        let (status, _, body) = respond(Ok(dir.clone()), &query);
        assert_eq!(status, axum::http::StatusCode::OK);
        assert_body_identity(&body, &"84".repeat(32));
        assert!(body.contains(r#""trades":[]"#), "{body}");
        assert!(body.contains("no committed result-set receipt"), "{body}");
        assert!(!body.contains(r#""seq":0"#), "foreign row leaked: {body}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn concurrent_detail_saturation_returns_429_without_queuing_work() {
        let held = crate::detail::hold_every_slot()
            .await
            .expect("the saturation test owns every slot");
        let uri: axum::http::Uri = format!("/trades.json?identity={}", "85".repeat(32))
            .parse()
            .expect("valid uri");
        let (status, _, body) = trades_json(uri).await;
        assert_eq!(status, axum::http::StatusCode::TOO_MANY_REQUESTS);
        assert!(body.contains("no blocking task was queued"), "{body}");
        assert!(body.contains(r#""complete":false"#), "{body}");
        drop(held);
    }
}

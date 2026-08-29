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
//! One hash probe to find the run's block, then one seek and one read per row —
//! `CLAUDE.md` §3 rule 4's constant per-operation cost, with the row count as
//! the only linear term and it is the answer itself. The index is rebuilt once
//! when the file is opened, which is the same open-time pass `results` and
//! `frontier` pay.

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
    respond(crate::server::store_dir(), uri.query().unwrap_or(""))
}

fn respond(
    root: Result<PathBuf, String>,
    query: &str,
) -> (axum::http::StatusCode, JsonHeaders, String) {
    let json: JsonHeaders = [(
        axum::http::header::CONTENT_TYPE,
        "application/json; charset=utf-8",
    )];

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

    // OPENED READ-ONLY, AND THAT IS THE POINT. A GET that creates its own empty
    // file answers "no trades" by MAKING that true, which is the failure §4
    // bans. `cli::frontier::open_read` exists for the same reason and had no
    // caller; this one does.
    let mut file = match cli::trades::Trades::open_read(&root) {
        Ok(file) => file,
        Err(why) => {
            // AN ABSENT FILE IS NOT AN ERROR, IT IS AN ANSWER. Nothing has been
            // swept yet, or nothing was recorded, and a 500 here would read as a
            // broken server rather than an empty store.
            return (
                axum::http::StatusCode::OK,
                json,
                format!(
                    r#"{{"trades":[],"count":0,"refusal":{}}}"#,
                    crate::render::json_string(&why)
                ),
            );
        }
    };

    let rows = match file.of_run(&identity) {
        Ok(rows) => rows,
        Err(why) => return refuse(json, &why),
    };

    let mut out = String::with_capacity(rows.len().saturating_mul(120).saturating_add(64));
    out.push_str(r#"{"trades":["#);
    for (at, row) in rows.iter().enumerate() {
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
                r#"{{"seq":{},"signal_bar":{},"entry_bar":{},"exit_bar":{},"best":{},"worst":{},"bars_held":{},"entry_micros":{},"exit_micros":{}}}"#,
                row.seq,
                row.signal_bar,
                row.entry_bar,
                row.exit_bar,
                row.best,
                row.worst,
                row.exit_bar.saturating_sub(row.entry_bar),
                row.entry_micros,
                row.exit_micros,
            ),
        );
    }
    out.push_str(r#"],"periods":{"#);
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
    for (at, (period, buckets)) in cli::trades::by_period(&rows).iter().enumerate() {
        if at > 0 {
            out.push(',');
        }
        let _ = std::fmt::Write::write_fmt(
            &mut out,
            format_args!(r#""{}":["#, period.name()),
        );
        for (n, b) in buckets.iter().enumerate() {
            if n > 0 {
                out.push(',');
            }
            let _ = std::fmt::Write::write_fmt(
                &mut out,
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
    let _ = std::fmt::Write::write_fmt(
        &mut out,
        format_args!(r#"}},"count":{},"refusal":null}}"#, rows.len()),
    );
    (axum::http::StatusCode::OK, json, out)
}

/// A refusal that names its cause, in the shape every other route here uses.
fn refuse(json: JsonHeaders, why: &str) -> (axum::http::StatusCode, JsonHeaders, String) {
    (
        axum::http::StatusCode::BAD_REQUEST,
        json,
        format!(
            r#"{{"trades":null,"count":0,"refusal":{}}}"#,
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
    use super::{from_hex, respond};

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
        assert!(body.contains(r#""trades":[]"#), "an empty list: {body}");
        assert!(body.contains(r#""count":0"#));
        assert!(
            !body.contains(r#""refusal":null"#),
            "and it says why: {body}"
        );
        assert!(
            !cli::trades::Trades::path(&dir).exists(),
            "and reading must not have created the file"
        );
    }

    /// A missing or malformed identity is refused rather than guessed at.
    #[test]
    fn a_request_without_an_identity_is_refused() {
        let dir = std::env::temp_dir().join("brutex-api-trades-none");
        let (status, _, body) = respond(Ok(dir), "");
        assert_eq!(status, axum::http::StatusCode::BAD_REQUEST);
        assert!(
            body.contains("64 hex characters"),
            "names the shape: {body}"
        );
        assert!(body.contains(r#""trades":null"#), "null, not empty: {body}");
    }
}

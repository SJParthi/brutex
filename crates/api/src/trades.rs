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
//! the only linear term and it is the answer itself.
//!
//! **This paragraph used to end "the index is rebuilt once when the file is
//! opened", and the file was opened ONCE PER REQUEST.** So the documented
//! once-per-process pass was a per-page-load walk of every row in the store —
//! one `read_exact` and one blake3 seal check each — to build an index used for
//! a single probe and then dropped. The sentence was true of `open_read` and
//! false of this route, which is the worst place for a cost claim to be wrong:
//! it described the primitive and not the caller.
//!
//! [`OPEN`] now holds the handle between requests, keyed on the file's path and
//! LENGTH. The store is append-only by §3 rule 8, so an unchanged length means
//! unchanged content and the index is rebuilt only when there is something new
//! in it. The sentence above is now true of the route as well as of the
//! primitive.

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

    let rows = match fetch(&root, &identity) {
        Fetched::Rows(rows) => rows,
        // AN ABSENT FILE IS NOT AN ERROR, IT IS AN ANSWER. Nothing has been
        // swept yet, or nothing was recorded, and a 500 here would read as a
        // broken server rather than an empty store.
        Fetched::NoFile(why) => {
            return (
                axum::http::StatusCode::OK,
                json,
                format!(
                    r#"{{"trades":[],"count":0,"refusal":{}}}"#,
                    crate::render::json_string(&why)
                ),
            );
        }
        Fetched::Unreadable(why) => return refuse(json, &why),
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
    // GROUPED ONCE AND READ TWICE. `by_period` is one pass over the rows with
    // eight maps; the consistency block below folds the buckets it already
    // produced rather than walking the rows again, so adding that block costs
    // O(buckets) and not a second O(trades).
    let grouped = cli::trades::by_period(&rows);
    write_periods(&mut out, &grouped);
    // HOW STEADILY IT EARNED, ONE ROW PER GRAIN.
    //
    // The buckets above answer "how much in each period"; this answers "how
    // many of them made money at all", which is the question a single headline
    // total cannot be asked. On the operator's own 60-minute run the two
    // readings disagree completely: the total is positive and SIX OF SEVEN
    // YEARS ARE NEGATIVE.
    //
    // A separate key rather than a field inside each period's array, because
    // that array is a list of buckets and the page already iterates it — adding
    // an object to a list of objects would change its shape for every existing
    // reader. This is purely additive.
    out.push_str(r#"},"consistency":{"#);
    write_consistency(&mut out, &grouped);

    // WHETHER ONE LUCKY BAR IS THE WHOLE RESULT.
    //
    // `without_best` is the field to read first. The operator's rule is that a
    // lucky trade must never be believed, and until this shipped there was no
    // number anywhere that could refuse one: a run carried by a single COVID
    // circuit-breaker session and a run with a real edge printed the same
    // `pessimistic`.
    //
    // NO THRESHOLD IS APPLIED HERE. `Robustness::survives` takes its bar as an
    // argument and this route does not supply one, because a bar baked into the
    // response is a static value the operator cannot move — the same objection
    // §6 makes to a depth parameter, one step earlier. The page compares.
    let r = cli::trades::Robustness::of(&rows);
    note_robustness(&r);

    let _ = std::fmt::Write::write_fmt(
        &mut out,
        format_args!(
            r#"}},"robustness":{{"trades":{},"wins":{},"total":{},"best_trade":{},"without_best":{},"gross_win":{},"top_share_ppm":{},"concentration_ppm":{}}},"count":{},"refusal":null}}"#,
            r.trades,
            // WINS IS SERVED BECAUSE THE BROWSER'S BAR NEEDS IT. `top_share_ppm`
            // is a share of the WINNINGS, so the share one winner carries at
            // perfect spread is `1/wins` — not `1/trades`, which is unreachable
            // the moment any trade loses.
            r.wins,
            r.total,
            r.best_trade,
            r.without_best,
            r.gross_win,
            r.top_share_ppm,
            r.concentration_ppm,
            rows.len(),
        ),
    );
    (axum::http::StatusCode::OK, json, out)
}

/// What one lookup found: the rows, an absent file, or a file that refused.
///
/// Three outcomes and not two, because the middle one is an ANSWER rather than
/// a failure and the route already distinguished them — an absent `trades.bin`
/// means nothing has been swept yet and answers `200` with an empty list, while
/// a corrupt one is a `400`. Folding them into a single `Result` would lose that
/// distinction at exactly the point where a reader most needs it.
enum Fetched {
    /// The run's round trips, possibly none if the identity is not in the file.
    Rows(Vec<cli::trades::Row>),
    /// No `trades.bin` at all, with the reason to echo.
    NoFile(String),
    /// The file exists and would not answer.
    Unreadable(String),
}

/// The open `trades.bin`, **held between requests**.
///
/// # This module's header already claimed it, and the code did the opposite
///
/// The header above says *"The index is rebuilt once when the file is opened"*.
/// `respond` called `Trades::open_read` **per request**, and `open_read` walks
/// the whole file — one `read_exact` and one blake3 seal check per row — to
/// build a block index that is then used for exactly one hash probe and thrown
/// away when the handler returns. So a documented O(1) find was an O(rows)
/// rebuild on every page load, and the linear term was the whole file rather
/// than the answer.
///
/// At the 3,306 rows on disk today that is invisible. One 81-month run records
/// **11,209** trades, so a hundred recorded runs is over a million seal checks
/// per request — and this is the route the backtest page calls on every
/// selection.
///
/// # Invalidated by LENGTH, which is sound only because the file is append-only
///
/// `cli::trades` never rewrites a row: `CLAUDE.md` §3 rule 8 makes the store
/// append-only, and `append_all` seeks to the end. So a file whose length is
/// unchanged has content that is unchanged, and length is a complete
/// invalidation key — no mtime granularity to worry about, no hash to compute.
/// A file that GREW is reopened, which rebuilds the index over the new rows as
/// well as the old; that is the same cost the old code paid every time, now paid
/// only when there is something new to learn.
///
/// The path is part of the key because `store_dir()` is configurable, and
/// serving one store's trades under another store's request is the class of
/// defect `calendar_of::cached` records fixing in its own key.
static OPEN: std::sync::OnceLock<std::sync::Mutex<Option<Held>>> = std::sync::OnceLock::new();

/// One cached open file, with the two facts that decide whether it is still good.
struct Held {
    /// Which `trades.bin` this is.
    path: PathBuf,
    /// Its length when the index was built.
    len: u64,
    /// The handle, index already rebuilt.
    file: cli::trades::Trades,
}

/// One run's rows, reusing the open file when the store has not grown.
fn fetch(root: &std::path::Path, identity: &[u8; 32]) -> Fetched {
    let path = cli::trades::Trades::path(root);
    let len = match std::fs::metadata(&path) {
        Ok(meta) => meta.len(),
        Err(why) => return Fetched::NoFile(format!("{} could not be read: {why}", path.display())),
    };

    let Ok(mut held) = OPEN.get_or_init(|| std::sync::Mutex::new(None)).lock() else {
        // A POISONED LOCK IS NOT A REASON TO SERVE NOTHING. Some other request
        // panicked while holding it; this one can still answer by opening its
        // own handle, which is precisely what the code did before the cache
        // existed. Falling back loudly-in-the-code and silently-on-the-wire is
        // acceptable here only because the fallback is the ORIGINAL behaviour
        // and not a degraded one.
        return match cli::trades::Trades::open_read(root) {
            Ok(mut file) => match file.of_run(identity) {
                Ok(rows) => Fetched::Rows(rows),
                Err(why) => Fetched::Unreadable(why),
            },
            Err(why) => Fetched::NoFile(why),
        };
    };

    let stale = held.as_ref().is_none_or(|h| h.len != len || h.path != path);
    if stale {
        match cli::trades::Trades::open_read(root) {
            Ok(file) => {
                *held = Some(Held {
                    path: path.clone(),
                    len,
                    file,
                });
            }
            Err(why) => return Fetched::NoFile(why),
        }
    }

    let Some(h) = held.as_mut() else {
        return Fetched::NoFile(format!("{} could not be opened", path.display()));
    };
    match h.file.of_run(identity) {
        Ok(rows) => Fetched::Rows(rows),
        Err(why) => Fetched::Unreadable(why),
    }
}

/// Every bucket of every calendar grain, as one JSON array per grain.
///
/// Lifted out of [`trades_json`] alongside [`write_consistency`] for the same
/// reason: the handler crossed the hundred-line lint and a hundred-line handler
/// is the shape that hides a defect. Extracting the three blocks is what the
/// lint is asking for; an `allow` would have been the fallback that hides a
/// failure `CLAUDE.md` §4 bans, applied to the tooling instead of to the data.
///
/// The bucket shape is unchanged from when this was inline — a page already
/// iterating these arrays reads the same bytes.
fn write_periods(out: &mut String, grouped: &[(cli::trades::Period, Vec<cli::trades::Bucket>)]) {
    for (at, (period, buckets)) in grouped.iter().enumerate() {
        if at > 0 {
            out.push(',');
        }
        let _ = std::fmt::Write::write_fmt(out, format_args!(r#""{}":["#, period.name()));
        for (n, b) in buckets.iter().enumerate() {
            if n > 0 {
                out.push(',');
            }
            let _ = std::fmt::Write::write_fmt(
                out,
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
}

/// One object per calendar grain, saying how steadily the run earned at it.
///
/// Split out of [`trades_json`] because that handler crossed the hundred-line
/// lint the moment this and [`note_robustness`] were added to it — and a
/// hundred-line handler is the shape that hides a defect, which is the lint's
/// whole point rather than a formality to be silenced with an `allow`.
///
/// O(buckets) and no second pass over the trades: the buckets arrive already
/// folded by `cli::trades::by_period`, and `Consistency::of` is one compare per
/// bucket.
fn write_consistency(
    out: &mut String,
    grouped: &[(cli::trades::Period, Vec<cli::trades::Bucket>)],
) {
    for (at, (period, buckets)) in grouped.iter().enumerate() {
        if at > 0 {
            out.push(',');
        }
        let c = cli::trades::Consistency::of(buckets);
        let _ = std::fmt::Write::write_fmt(
            out,
            format_args!(
                r#""{}":{{"buckets":{},"positive":{},"positive_ppm":{},"worst_bucket":{},"worst_bucket_key":{},"best_bucket":{},"best_bucket_key":{}}}"#,
                period.name(),
                c.buckets,
                c.positive,
                c.positive_ppm,
                c.worst_bucket,
                c.worst_bucket_key,
                c.best_bucket,
                c.best_bucket_key,
            ),
        );
    }
}

/// The audit trail for one run's robustness, and **its level is decided by a
/// sign and not by a number.**
///
/// `Warn` when the run is above water as it stands and BELOW it with its single
/// best trade struck out. That is a sign flip — an objective property of the
/// series — so there is no threshold anybody chose and no static value here to
/// be set wrongly and silently the way §6 describes. A run that survives its own
/// best trade logs at `Info` and reads as ordinary; a run that does not is
/// exactly the row an operator must never scroll past, and it now appears on
/// `/logs` beside the pull events.
///
/// ONE EVENT PER REQUEST, which is the granularity `note_request` already logs
/// at and far coarser than gate 17's concern: that gate silences `vocab engine
/// indicators runner` because they hold the loops, and `api` is on neither list.
/// Nothing here runs per trade — `Robustness::of` has already finished when this
/// fires.
pub(crate) fn note_robustness(r: &cli::trades::Robustness) {
    let carried_by_one = r.total > 0 && r.without_best <= 0;
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::new(
            if carried_by_one {
                telemetry::Level::Warn
            } else {
                telemetry::Level::Info
            },
            "api.trades",
            if carried_by_one {
                "the result rests on a single trade"
            } else {
                "robustness measured"
            },
        )
        .with("trades", telemetry::Value::Uint(r.trades))
        // `paisa` and not `Int` for the three money fields, which is this
        // workspace's own convention: the constructor exists "so a call site
        // reads as the thing it is and a reviewer can grep for every place a
        // price enters the log". The two ppm figures are ratios, not money, and
        // take `Int`.
        .with("total", telemetry::Value::paisa(r.total))
        .with("best_trade", telemetry::Value::paisa(r.best_trade))
        .with("without_best", telemetry::Value::paisa(r.without_best))
        .with("top_share_ppm", telemetry::Value::Int(r.top_share_ppm))
        .with(
            "concentration_ppm",
            telemetry::Value::Int(r.concentration_ppm),
        ),
    );
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

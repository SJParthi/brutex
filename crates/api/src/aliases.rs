//! `/aliases.json` -- the alias and symbol-lineage table (D-3332): every name
//! an index goes by, every dated symbol change, each with its sources or
//! marked UNVERIFIED; and, with `?symbol=`, what one name resolves to.
//!
//! # Why a route
//!
//! `/db` had no way to know that FINNIFTY is filed as `NIFTYFINSERVICE`, that
//! ZOMATO became ETERNAL on 9 April 2025, or that NIFTYIT is an index and not a
//! share (DB-040, DB-071, DB-G25, DB-G54). The table is
//! `brutex_core::alias`; this route serves it, so the search (WP-51), the
//! mapping page (WP-61) and anything else that names an instrument reads one
//! answer rather than keeping a copy that drifts -- the same reason
//! `/vocab.json` serves the condition table (D-0288).
//!
//! # What it answers
//!
//! - `GET /aliases.json`: the whole table. Every index row (F&O symbol, NSE
//!   name, `indexSymbol` = GDFL file stem, Kite tradingsymbol, store key, and
//!   whether today's F&O list names it), every symbol change (old, new, the day
//!   it applies, the newest symbol its chain reaches), every symbol deliberately
//!   not aliased, and each row's citations.
//! - `GET /aliases.json?symbol=X`: what `X` resolves to, under any spelling of
//!   an index name, an old or a new symbol: the store segment and key, the
//!   changes forward from it, the changes that led to it ("formerly"), or the
//!   refusal that keeps it on its own key. A name the table does not hold is
//!   answered `"matched":"none"`, its own key as a share -- never a guess.
//!
//! # Cost
//!
//! Nothing is read from disk. The whole-table body is rendered once per
//! process and reused; a `?symbol=` answer is at most four probes of
//! compile-time tables and one hop per applied change, bounded by the table
//! (`brutex_core::alias`'s own `# Cost`). The route is O(1) per request.

use super::{param, render};
use brutex_core::alias::{
    self, Citation, INDEX_ROWS, IndexRow, LINEAGE, NOT_ALIASED, NotAliased, SYMBOL_CHANGES, Status,
    SymbolChange,
};
use std::fmt::Write as _;

/// The longest `symbol` this route reads, in bytes. The table's longest name
/// is 24 bytes; anything past this names nothing it holds and is refused
/// rather than walked.
pub(super) const MAX_SYMBOL_BYTES: usize = alias::MAX_NAME_BYTES;

/// The response: status, content type, body.
type Answer = (
    axum::http::StatusCode,
    [(axum::http::HeaderName, &'static str); 1],
    String,
);

/// `GET /aliases.json[?symbol=X]`. 400 for a `symbol` longer than
/// [`MAX_SYMBOL_BYTES`]; 200 otherwise, including for a name the table does
/// not hold, which is answered by name as `"matched":"none"`.
pub(super) async fn aliases_json(uri: axum::http::Uri) -> Answer {
    let json = [(
        axum::http::header::CONTENT_TYPE,
        "application/json; charset=utf-8",
    )];
    let asked = param(uri.query().unwrap_or(""), "symbol");
    let asked = asked.trim();
    if asked.len() > MAX_SYMBOL_BYTES {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            json,
            format!(
                r#"{{"error":{}}}"#,
                render::json_string(&format!(
                    "symbol is {} bytes; the longest name this table holds is far shorter, \
                     and more than {MAX_SYMBOL_BYTES} is refused rather than read",
                    asked.len()
                ))
            ),
        );
    }
    if asked.is_empty() {
        return (axum::http::StatusCode::OK, json, table().to_owned());
    }
    (axum::http::StatusCode::OK, json, resolve(asked))
}

/// The whole table, rendered once.
fn table() -> &'static str {
    static TABLE: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    TABLE.get_or_init(render_table)
}

/// The whole table as JSON.
#[must_use]
pub(super) fn render_table() -> String {
    let unverified = INDEX_ROWS
        .iter()
        .filter(|row| !row.status.applies())
        .count()
        + SYMBOL_CHANGES
            .iter()
            .filter(|row| !row.status.applies())
            .count();
    let mut out = String::with_capacity(16 * 1024);
    let _ = write!(
        out,
        r#"{{"decision":"{}","charter":{},"keyed_feeds":["#,
        alias::DECISION,
        render::json_string(alias::CHARTER),
    );
    for (n, feed) in alias::KEYED_FEEDS.iter().enumerate() {
        if n > 0 {
            out.push(',');
        }
        out.push_str(&render::json_string(feed.as_str()));
    }
    let _ = write!(
        out,
        r#"],"counts":{{"index":{},"symbol_changes":{},"not_aliased":{},"unverified":{unverified}}},"index":["#,
        INDEX_ROWS.len(),
        SYMBOL_CHANGES.len(),
        NOT_ALIASED.len(),
    );
    for (n, row) in INDEX_ROWS.iter().enumerate() {
        if n > 0 {
            out.push(',');
        }
        write_index(&mut out, row);
    }
    out.push_str(r#"],"symbol_changes":["#);
    for (n, row) in SYMBOL_CHANGES.iter().enumerate() {
        if n > 0 {
            out.push(',');
        }
        write_change(&mut out, row, true);
    }
    out.push_str(r#"],"not_aliased":["#);
    for (n, row) in NOT_ALIASED.iter().enumerate() {
        if n > 0 {
            out.push(',');
        }
        write_not_aliased(&mut out, row);
    }
    out.push_str("]}");
    out
}

/// What one name resolves to, as JSON.
#[must_use]
pub(super) fn resolve(asked: &str) -> String {
    let upper = asked.to_ascii_uppercase();
    let mut out = String::with_capacity(2048);
    let _ = write!(
        out,
        r#"{{"symbol":{},"decision":"{}""#,
        render::json_string(asked),
        alias::DECISION
    );
    if let Some(row) = alias::index_by_name(&upper) {
        let _ = write!(
            out,
            r#","matched":"index","segment":"INDEX","store_key":{},"key":{},"index":"#,
            render::json_string(row.store_key),
            render::json_string(&key("INDEX", row.store_key)),
        );
        write_index(&mut out, row);
        out.push_str(r#","changes":[],"formerly":[],"not_aliased":null}"#);
        return out;
    }
    let current = LINEAGE.current(&upper);
    let changes: Vec<&SymbolChange> = LINEAGE.forward(&upper).collect();
    let formerly: Vec<&SymbolChange> = LINEAGE.backward(&upper).collect();
    let refused = alias::not_aliased(&upper);
    let matched = if !changes.is_empty() || !formerly.is_empty() {
        "symbol_change"
    } else if refused.is_some() {
        "not_aliased"
    } else {
        "none"
    };
    let held = brutex_core::symbol::Symbol::new(current).is_ok();
    let _ = write!(
        out,
        r#","matched":"{matched}","segment":"CASH","store_key":{},"key":{},"index":null,"changes":["#,
        if held {
            render::json_string(current)
        } else {
            "null".to_owned()
        },
        if held {
            render::json_string(&key("CASH", current))
        } else {
            "null".to_owned()
        },
    );
    for (n, row) in changes.iter().enumerate() {
        if n > 0 {
            out.push(',');
        }
        write_change(&mut out, row, false);
    }
    out.push_str(r#"],"formerly":["#);
    for (n, row) in formerly.iter().enumerate() {
        if n > 0 {
            out.push(',');
        }
        write_change(&mut out, row, false);
    }
    out.push_str(r#"],"not_aliased":"#);
    match refused {
        Some(row) => write_not_aliased(&mut out, row),
        None => out.push_str("null"),
    }
    if !held {
        let _ = write!(
            out,
            r#","why":{}"#,
            render::json_string(
                "no row names it, and it is not a symbol the store can hold (letters, digits, \
                 '-', '_' and '&', at most 24)"
            )
        );
    }
    out.push('}');
    out
}

/// `NSE-<SEGMENT>-<KEY>`, the store path with its separators changed, as
/// `api::census::Series` renders a spot series.
fn key(segment: &str, store_key: &str) -> String {
    format!("NSE-{segment}-{store_key}")
}

/// One index row.
fn write_index(out: &mut String, row: &IndexRow) {
    let fno = row
        .fno
        .map_or_else(|| "null".to_owned(), render::json_string);
    let _ = write!(
        out,
        concat!(
            r#"{{"fno":{},"nse_name":{},"index_symbol":{},"gdfl_stem":{},"kite":{},"#,
            r#""store_key":{},"key":{},"segment":"INDEX","reference_only":{},"#,
            r#""in_fno_universe":{},"#
        ),
        fno,
        render::json_string(row.nse_name),
        render::json_string(row.index_symbol),
        render::json_string(&format!("{}.NSE_IDX.csv", row.index_symbol)),
        render::json_string(row.kite),
        render::json_string(row.store_key),
        render::json_string(&key("INDEX", row.store_key)),
        row.reference_only,
        brutex_core::universe::FNO_INDEX.contains(row.listed_as()),
    );
    write_status(out, row.status);
    out.push('}');
}

/// One symbol change; `current` adds the newest symbol its chain reaches.
fn write_change(out: &mut String, row: &SymbolChange, current: bool) {
    let _ = write!(
        out,
        r#"{{"old":{},"new":{},"effective":"{}","#,
        render::json_string(row.old),
        render::json_string(row.new),
        row.effective,
    );
    if current {
        let newest = LINEAGE.current(row.old);
        let _ = write!(
            out,
            r#""current":{},"key":{},"#,
            render::json_string(newest),
            render::json_string(&key("CASH", newest)),
        );
    }
    write_status(out, row.status);
    out.push('}');
}

/// One symbol deliberately not aliased.
fn write_not_aliased(out: &mut String, row: &NotAliased) {
    let _ = write!(
        out,
        r#"{{"symbol":{},"key":{},"why":{}}}"#,
        render::json_string(row.symbol),
        render::json_string(&key("CASH", row.symbol)),
        render::json_string(row.why),
    );
}

/// `"status":…,"sources":[…]`, and for an unverified row what is missing.
fn write_status(out: &mut String, status: Status) {
    let _ = write!(out, r#""status":"{}","sources":["#, status.word());
    for (n, cited) in status.citations().iter().enumerate() {
        if n > 0 {
            out.push(',');
        }
        write_citation(out, cited);
    }
    out.push(']');
    if let Status::Unverified(missing) = status {
        let _ = write!(out, r#","unverified":{}"#, render::json_string(missing));
    }
}

/// One citation.
fn write_citation(out: &mut String, cited: &Citation) {
    let _ = write!(
        out,
        r#"{{"url":{},"read":{},"sha256":{},"says":{}}}"#,
        render::json_string(cited.url),
        render::json_string(cited.read),
        cited
            .sha256
            .map_or_else(|| "null".to_owned(), render::json_string),
        render::json_string(cited.says),
    );
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail, and these lints exist to \
              keep panics out of the crate rather than out of its tests"
)]
mod tests {
    use super::{MAX_SYMBOL_BYTES, render_table, resolve, write_status};
    use crate::server::Loaded;
    use crate::server::tests::{agreeing, body_of, get, served_over, site};
    use brutex_core::alias::Status;
    use serde_json::Value;

    fn parsed(body: &str) -> Value {
        serde_json::from_str(body).unwrap_or_else(|why| panic!("JSON ({why}): {body}"))
    }

    /// **THE WHOLE TABLE, WITH ITS SOURCES.** FINNIFTY's row names the key the
    /// feeds file it under and the product page it rests on, with the page's
    /// digest; ZOMATO's change cites NSE's file and its own circular; HDFC is
    /// listed as deliberately not aliased.
    #[test]
    fn the_table_carries_every_row_with_its_sources() {
        let table = parsed(&render_table());
        assert_eq!(table["decision"], "D-3332");
        assert_eq!(table["counts"]["index"], 9);
        assert_eq!(table["counts"]["symbol_changes"], 18);
        assert_eq!(table["counts"]["not_aliased"], 2);
        assert_eq!(table["counts"]["unverified"], 0);
        assert_eq!(table["keyed_feeds"], serde_json::json!(["zerodha", "gdfl"]));
        let index = table["index"].as_array().expect("index rows");
        let fin = index
            .iter()
            .find(|row| row["fno"] == "FINNIFTY")
            .expect("FINNIFTY");
        assert_eq!(fin["key"], "NSE-INDEX-NIFTYFINSERVICE");
        assert_eq!(fin["store_key"], "NIFTYFINSERVICE");
        assert_eq!(fin["nse_name"], "NIFTY FINANCIAL SERVICES");
        assert_eq!(fin["gdfl_stem"], "NIFTY FIN SERVICE.NSE_IDX.csv");
        assert_eq!(fin["kite"], "NIFTY FIN SERVICE");
        assert_eq!(fin["in_fno_universe"], true);
        assert_eq!(fin["status"], "sourced");
        assert_eq!(
            fin["sources"][0]["sha256"],
            "60cd6ddf540a2771c4af76353ad2c0ea60fcada964ee77a0c427c8d8dc6aef44"
        );
        let nifty_it = index
            .iter()
            .find(|row| row["fno"] == "NIFTYIT")
            .expect("NIFTYIT");
        assert_eq!(nifty_it["in_fno_universe"], false, "not in today's list");
        let vix = index
            .iter()
            .find(|row| row["store_key"] == "INDIAVIX")
            .expect("VIX");
        assert_eq!(vix["fno"], Value::Null);
        assert_eq!(vix["reference_only"], true);
        let nifty = index
            .iter()
            .find(|row| row["fno"] == "NIFTY")
            .expect("NIFTY");
        assert_eq!(nifty["key"], "NSE-INDEX-NIFTY");
        assert_eq!(nifty["nse_name"], "NIFTY 50");

        let changes = table["symbol_changes"].as_array().expect("changes");
        let zomato = changes
            .iter()
            .find(|row| row["old"] == "ZOMATO")
            .expect("ZOMATO");
        assert_eq!(zomato["new"], "ETERNAL");
        assert_eq!(zomato["effective"], "2025-04-09");
        assert_eq!(zomato["key"], "NSE-CASH-ETERNAL");
        assert_eq!(zomato["sources"].as_array().map(Vec::len), Some(2));
        assert!(
            zomato["sources"][1]["url"]
                .as_str()
                .is_some_and(|url| url.ends_with("CML67420.pdf"))
        );
        let lti = changes.iter().find(|row| row["old"] == "LTI").expect("LTI");
        assert_eq!(lti["new"], "LTIM");
        assert_eq!(lti["current"], "LTM", "the chain's newest symbol");
        let refused = table["not_aliased"].as_array().expect("not aliased");
        assert_eq!(refused[0]["symbol"], "HDFC");
        assert_eq!(refused[0]["key"], "NSE-CASH-HDFC");
        assert!(
            refused[1]["why"]
                .as_str()
                .is_some_and(|why| why.contains("D-3332"))
        );
    }

    /// **DB-G25: FINNIFTY RESOLVES TO THE KEY THE FEEDS FILE**, under every
    /// spelling a reader types, and an index option's segment is `INDEX`
    /// (DB-G54).
    #[test]
    fn an_index_name_resolves_to_its_store_key_under_any_spelling() {
        for (asked, key) in [
            ("FINNIFTY", "NSE-INDEX-NIFTYFINSERVICE"),
            ("finnifty", "NSE-INDEX-NIFTYFINSERVICE"),
            ("NIFTY FIN SERVICE", "NSE-INDEX-NIFTYFINSERVICE"),
            ("NIFTY 50", "NSE-INDEX-NIFTY"),
            ("NIFTY", "NSE-INDEX-NIFTY"),
            ("NIFTY BANK", "NSE-INDEX-BANKNIFTY"),
            ("MIDCPNIFTY", "NSE-INDEX-NIFTYMIDSELECT"),
            ("NIFTYNXT50", "NSE-INDEX-NIFTYNEXT50"),
            ("NIFTYIT", "NSE-INDEX-NIFTYIT"),
            ("NIFTYFPI", "NSE-INDEX-NIFTYFPI150"),
        ] {
            let answer = parsed(&resolve(asked));
            assert_eq!(answer["matched"], "index", "{asked}");
            assert_eq!(answer["segment"], "INDEX", "{asked}");
            assert_eq!(answer["key"], key, "{asked}");
            assert_eq!(answer["symbol"], asked);
            assert_eq!(answer["index"]["status"], "sourced");
        }
    }

    /// **DB-071: AN OLD SYMBOL RESOLVES TO THE CURRENT ONE, AND THE CURRENT
    /// ONE NAMES ITS FORMER SYMBOLS.** HDFC, with no row, keeps its own key
    /// and says why; a name with no row at all is its own key as a share.
    #[test]
    fn a_changed_symbol_resolves_both_ways_and_an_unknown_one_is_its_own_key() {
        let zomato = parsed(&resolve("zomato"));
        assert_eq!(zomato["matched"], "symbol_change");
        assert_eq!(zomato["segment"], "CASH");
        assert_eq!(zomato["key"], "NSE-CASH-ETERNAL");
        assert_eq!(zomato["changes"][0]["new"], "ETERNAL");
        assert_eq!(zomato["formerly"], serde_json::json!([]));

        let eternal = parsed(&resolve("ETERNAL"));
        assert_eq!(eternal["matched"], "symbol_change");
        assert_eq!(eternal["key"], "NSE-CASH-ETERNAL");
        assert_eq!(eternal["formerly"][0]["old"], "ZOMATO");
        assert_eq!(eternal["formerly"][0]["effective"], "2025-04-09");

        let ltim = parsed(&resolve("LTIM"));
        assert_eq!(ltim["key"], "NSE-CASH-LTM");
        assert_eq!(ltim["changes"][0]["new"], "LTM");
        assert_eq!(ltim["formerly"][0]["old"], "LTI");
        let newest = parsed(&resolve("LTM"));
        assert_eq!(newest["formerly"].as_array().map(Vec::len), Some(2));

        let hdfc = parsed(&resolve("HDFC"));
        assert_eq!(hdfc["matched"], "not_aliased");
        assert_eq!(hdfc["key"], "NSE-CASH-HDFC");
        assert!(hdfc["not_aliased"]["why"].as_str().is_some());

        let reliance = parsed(&resolve("RELIANCE"));
        assert_eq!(reliance["matched"], "none");
        assert_eq!(reliance["key"], "NSE-CASH-RELIANCE");
        assert_eq!(reliance["not_aliased"], Value::Null);
        assert!(reliance.get("why").is_none());

        let spaced = parsed(&resolve("NOT A SYMBOL"));
        assert_eq!(spaced["matched"], "none");
        assert_eq!(spaced["key"], Value::Null);
        assert_eq!(spaced["store_key"], Value::Null);
        assert!(spaced["why"].as_str().is_some());
    }

    #[test]
    fn an_unverified_status_says_what_is_missing() {
        let mut out = String::new();
        write_status(&mut out, Status::Unverified("no exchange source read"));
        assert_eq!(
            out,
            r#""status":"UNVERIFIED","sources":[],"unverified":"no exchange source read""#
        );
    }

    /// Over the real router: the table, one resolution, the bound, and an
    /// empty `symbol` that is the table.
    #[tokio::test]
    async fn the_route_serves_the_table_and_resolves_a_name() {
        let dir = agreeing("aliases-route");
        let built = Loaded::new(site("aliases-route", &dir));
        served_over("aliases-route", built, |addr| async move {
            let whole = get(addr, "/aliases.json").await;
            assert!(whole.starts_with("HTTP/1.1 200 "), "{whole}");
            assert!(
                whole
                    .to_ascii_lowercase()
                    .contains("content-type: application/json"),
                "{whole}"
            );
            assert_eq!(body_of(&whole), render_table());

            let empty = get(addr, "/aliases.json?symbol=%20").await;
            assert_eq!(body_of(&empty), render_table());

            let one = get(addr, "/aliases.json?symbol=NIFTY%20FIN%20SERVICE").await;
            assert!(one.starts_with("HTTP/1.1 200 "), "{one}");
            assert_eq!(
                parsed(body_of(&one))["key"],
                "NSE-INDEX-NIFTYFINSERVICE",
                "{one}"
            );
            let amp = get(addr, "/aliases.json?symbol=L%26TFH").await;
            assert_eq!(parsed(body_of(&amp))["key"], "NSE-CASH-LTF", "{amp}");

            let long = "A".repeat(MAX_SYMBOL_BYTES + 1);
            let refused = get(addr, &format!("/aliases.json?symbol={long}")).await;
            assert!(refused.starts_with("HTTP/1.1 400 "), "{refused}");
            assert!(
                parsed(body_of(&refused))["error"]
                    .as_str()
                    .is_some_and(|why| why.contains("refused rather than read")),
                "{refused}"
            );
            let at = "A".repeat(MAX_SYMBOL_BYTES);
            let held = get(addr, &format!("/aliases.json?symbol={at}")).await;
            assert!(held.starts_with("HTTP/1.1 200 "), "{held}");
        })
        .await;
    }
}

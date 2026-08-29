//! One run's ranked combinations, as structured JSON the page can sort.
//!
//! # Why this exists beside `/engine/top.json`
//!
//! `/engine/top.json` serves `cli::top_at`, which renders a TEXT report — a
//! `<pre>` block, already formatted, with the condition names resolved. That is
//! the right shape for reading and the wrong shape for ranking: nothing in a
//! rendered table can be re-sorted, filtered, or weighted by a reader who wants
//! *"the ten with the smallest drawdown and the highest win rate"*.
//!
//! The operator's ranking is exactly that question:
//!
//! > *"top 10 ranking should be calculated based on very less max drawdown,
//! > less max stop loss, less losing percentage, less losing trades, less
//! > losing ratio, and on the win side massive max profit, higher winning
//! > trades, higher winning percentage, higher winning ratio, average maximum
//! > profit, average less loss."*
//!
//! That is a WEIGHTED ordering over eleven quantities, and the weights are the
//! operator's to move. Baking a score into Rust would put one more number in the
//! binary that nobody can see — the failure this whole session has been
//! unpicking. So the engine serves the measurements and the page does the
//! ordering, where a slider changes it without a rebuild.
//!
//! # What is derived here and what is not
//!
//! `frontier::Row` v2 stores six RAW money fields and no derived ones. The
//! derived quantities — win rate, reward-to-risk, return over drawdown, average
//! win, average loss — are NOT computed here either. `cli::frontier::Row::derived`
//! rebuilds a `grid::Cell` from the six stored fields and asks IT, so there is
//! one definition of "win rate" in this repository and not a second in this file
//! or a third in JavaScript. `api` cannot reach `runner` at all — the arrow is
//! `api -> cli` — which is what forced the right shape rather than merely
//! suggesting it. That is the same argument `/vocab.json` makes for
//! serving the condition table once rather than copying it into the page.
//!
//! # Cost
//!
//! One hash probe to find the run's block, then one read of its rows —
//! `CLAUDE.md` §3 rule 4's constant per-operation cost with the row count as the
//! only linear term, and that count is the answer itself.

use std::path::PathBuf;

/// The headers every JSON route here answers with.
type JsonHeaders = [(axum::http::header::HeaderName, &'static str); 1];

/// One run's ranked combinations, with every measurement the operator ranks on.
pub async fn frontier_json(uri: axum::http::Uri) -> (axum::http::StatusCode, JsonHeaders, String) {
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
    let Some(identity) = crate::trades::from_hex_public(&crate::server::param(query, "identity"))
    else {
        return refuse(
            json,
            "`identity` must be the 64 hex characters `/backtest.json` prints on \
             every row. This file holds many runs, so which one is not a detail \
             it can infer.",
        );
    };

    // READ-ONLY, so a GET on a store that has never been swept cannot answer
    // "no combinations" by creating the file that makes it true.
    let mut file = match cli::frontier::Frontier::open_read(&root) {
        Ok(file) => file,
        Err(why) => {
            return (
                axum::http::StatusCode::OK,
                json,
                format!(
                    r#"{{"rows":[],"count":0,"refusal":{}}}"#,
                    crate::render::json_string(&why)
                ),
            );
        }
    };

    let (rows, partial) = match file.of_run(&identity) {
        Ok(pair) => pair,
        Err(why) => return refuse(json, &why),
    };

    let mut out = String::with_capacity(rows.len().saturating_mul(320).saturating_add(96));
    out.push_str(r#"{"rows":["#);
    for (at, row) in rows.iter().enumerate() {
        if at > 0 {
            out.push(',');
        }
        // ASKED OF THE ENGINE, NOT RECOMPUTED HERE. `Row::derived` rebuilds a
        // `grid::Cell` from the six stored fields and asks IT for the five
        // derived ones, so there is one definition of "win rate" and not a
        // second written in this file. `api` cannot reach `runner` at all --
        // the arrow is `api -> cli` -- which is what forced the right shape.
        let d = row.derived();
        let _ = std::fmt::Write::write_fmt(
            &mut out,
            format_args!(
                r#"{{"rank":{},"mask_words":[{},{},{},{},{},{}],"hits":{},"n":{},"mean_milli_paisa":{},"t_milli":{},"payoff_bp":{},"edge_wins":{},"priced":{},"trades":{},"wins":{},"losses":{},"pessimistic":{},"worst_trade":{},"max_drawdown":{},"min_win":{},"win_rate_bp":{},"reward_to_risk_bp":{},"return_over_drawdown":{},"avg_win":{},"avg_loss":{}}}"#,
                row.rank,
                row.mask_words[0],
                row.mask_words[1],
                row.mask_words[2],
                row.mask_words[3],
                row.mask_words[4],
                row.mask_words[5],
                row.hits,
                row.n,
                row.mean_milli_paisa,
                row.t_milli,
                row.payoff_bp,
                row.wins,
                d.priced,
                row.trades,
                row.cell_wins,
                d.losses,
                row.pessimistic,
                row.worst_trade,
                row.max_drawdown,
                row.min_win,
                d.win_rate_bp,
                d.reward_to_risk_bp,
                d.return_over_drawdown,
                d.avg_win,
                d.avg_loss,
            ),
        );
    }
    // A PARTIAL READ IS REPORTED, NOT ROUNDED OFF. `of_run` returns the rows it
    // could read AND why it stopped; dropping the second would turn a torn tail
    // into a shorter answer that looks complete.
    let _ = std::fmt::Write::write_fmt(
        &mut out,
        format_args!(
            r#"],"count":{},"refusal":{}}}"#,
            rows.len(),
            partial.map_or_else(|| "null".to_owned(), |why| crate::render::json_string(&why))
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
            r#"{{"rows":null,"count":0,"refusal":{}}}"#,
            crate::render::json_string(why)
        ),
    )
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
    use super::respond;

    #[test]
    fn a_request_without_an_identity_is_refused() {
        let dir = std::env::temp_dir().join("brutex-api-frontier-none");
        let (status, _, body) = respond(Ok(dir), "");
        assert_eq!(status, axum::http::StatusCode::BAD_REQUEST);
        assert!(
            body.contains("64 hex characters"),
            "names the shape: {body}"
        );
        assert!(body.contains(r#""rows":null"#), "null, not empty: {body}");
    }

    /// An unswept store answers empty AND says why, so "nothing recorded" and
    /// "could not be read" are never the same response.
    #[test]
    fn an_absent_file_answers_empty_and_names_why() {
        let dir = std::env::temp_dir().join(format!(
            "brutex-api-frontier-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let query = format!("identity={}", "b".repeat(64));
        let (status, _, body) = respond(Ok(dir.clone()), &query);
        assert_eq!(status, axum::http::StatusCode::OK);
        assert!(body.contains(r#""rows":[]"#), "an empty list: {body}");
        assert!(
            !body.contains(r#""refusal":null"#),
            "and it says why: {body}"
        );
        assert!(
            !cli::frontier::Frontier::path(&dir).exists(),
            "and reading must not have created it"
        );
    }
}

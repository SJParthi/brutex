//! What every RUNNING sweep has found so far.
//!
//! # The gap this closes, measured on the operator's own machine
//!
//! `crates/cli/src/live.rs` is 863 lines that write a run's top-N to
//! `results/live/<hex>.bin` on every improvement, and it ships `live::current`
//! to read the directory back. That reader had **zero callers**, and the only
//! occurrence of the string `live.json` anywhere in the tree was the doc comment
//! in `live.rs` naming the route that was supposed to call it.
//!
//! So the feature existed, wrote its files, and could not be looked at. Asked
//! *"what is the status of the current sweep"* five hours into an eight-rung
//! `range-all`, the honest answer required decoding the binary by hand with
//! `xxd` — offset 16 for `trials`, 24 for the bar, 40 plus a 208-byte stride for
//! the rows. That is what this route is for.
//!
//! # Why it does not judge the rows
//!
//! `/frontier.json` calls `Row::derived()` and `Row::verdict()` and serves a
//! `meets` block. This must not, and the reason is not style: a live row is
//! written by `publish_ranked` BEFORE the exit grid runs, so `trades`, `wins`,
//! `pessimistic`, `worst_trade`, `max_drawdown`, `min_win`, `gross_win` and
//! `gross_loss` are all structural zeros. A win rate computed from them is 0%, a
//! reward-to-risk is 0.00x, and a verdict is FAIL — three figures nobody
//! measured, wearing the shape of three that somebody did. `CLAUDE.md` §4 bans
//! exactly that.
//!
//! What is real at publish time is the SWEEP half — the mask, the hits, the
//! sample size, the mean, the `t`, the payoff — and the bar that `t` must clear.
//! Those are served, and `"ranked_only": true` says in the payload itself that
//! the rest is not here yet rather than leaving a reader to infer it from zeros.
//!
//! # The bar travels with the rows
//!
//! `Summary::bar_milli` is the `|t|` a row must reach to be distinguishable from
//! luck **at this run's current trial count**, and that count grows while the run
//! is in flight. A live view showing `t` without it invites the reading §4 bans.
//! It is on the run object, not the row, because it is one fact per run.

use std::path::PathBuf;

/// The headers every JSON route here answers with.
type JsonHeaders = [(axum::http::header::HeaderName, &'static str); 1];

/// Every run with a live file, newest measurement first within each.
pub async fn live_json() -> (axum::http::StatusCode, JsonHeaders, String) {
    respond(crate::server::store_dir())
}

/// [`live_json`], with the root passed in so a test can drive it.
///
/// The same split `frontierjson::respond` takes, and for the same reason: the
/// handler resolves the store from the environment and the body does not.
fn respond(root: Result<PathBuf, String>) -> (axum::http::StatusCode, JsonHeaders, String) {
    let json: JsonHeaders = [(
        axum::http::header::CONTENT_TYPE,
        "application/json; charset=utf-8",
    )];

    let root = match root {
        Ok(root) => root,
        Err(why) => {
            return (
                axum::http::StatusCode::OK,
                json,
                format!(
                    r#"{{"runs":[],"count":0,"refusal":{}}}"#,
                    crate::render::json_string(&why)
                ),
            );
        }
    };

    // SKIPPING IS STILL RIGHT, AND IT IS NOW COUNTED. This directory is
    // transient by design and a half-written file is an ordinary state, not a
    // corruption to report -- but reporting `count: 0` for a machine whose
    // three live files are one version old told the operator the sweep was not
    // running. `skipped` and `listed` are the two facts that were folded into
    // an empty list: `listed: false` is "I could not look", and any nonzero
    // `skipped` beside `count: 0` is "something is there and I cannot read it".
    let census = cli::live::census(&root);
    let runs = &census.runs;

    let mut out = String::with_capacity(runs.len().saturating_mul(2_400).saturating_add(64));
    out.push_str(r#"{"runs":["#);
    for (nth, (identity, summary, rows)) in runs.iter().enumerate() {
        if nth > 0 {
            out.push(',');
        }
        write_run(&mut out, *identity, summary, rows);
    }
    let _ = std::fmt::Write::write_fmt(
        &mut out,
        format_args!(
            r#"],"count":{},"skipped":{},"listed":{},"refusal":null}}"#,
            runs.len(),
            census.skipped,
            census.listed
        ),
    );
    (axum::http::StatusCode::OK, json, out)
}

/// One in-flight run: what it has weighed, what it must clear, and its top rows.
fn write_run(
    out: &mut String,
    identity: [u8; 32],
    summary: &cli::live::Summary,
    rows: &[cli::frontier::Row],
) {
    let _ = std::fmt::Write::write_fmt(
        out,
        format_args!(
            r#"{{"identity":"{}","trials":{},"bar_milli":{},"priced":{},"ranked_only":true,"rows":["#,
            crate::server::hex32(identity),
            summary.trials,
            summary.bar_milli,
            summary.priced,
        ),
    );
    for (nth, row) in rows.iter().enumerate() {
        if nth > 0 {
            out.push(',');
        }
        write_row(out, row, summary.bar_milli);
    }
    let _ = std::fmt::Write::write_fmt(out, format_args!(r#"],"kept":{}}}"#, rows.len()));
}

/// One ranked row, in the eight fields that are REAL before the grid runs.
fn write_row(out: &mut String, row: &cli::frontier::Row, bar_milli: i64) {
    let _ = std::fmt::Write::write_fmt(
        out,
        format_args!(
            // 64-BIT SAFE ON THE WIRE, the same rule `/frontier.json` follows: a
            // bare JSON number above 2^53 does not survive `JSON.parse`, and a
            // mask decoded from a rounded word names the WRONG conditions while
            // looking exactly like an answer.
            r#"{{"rank":{},"direction":"{}","mask_words":["{}","{}","{}","{}","{}","{}"],"hits":{},"n":{},"mean_milli_paisa":{},"t_milli":{},"payoff_bp":{},"edge_wins":{},"clears_bar":{}}}"#,
            row.rank,
            row.direction.as_str(),
            // DECIMAL STRINGS, one per word, for the reason above.
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
            // THE COMPARISON, MADE HERE RATHER THAN LEFT TO THE READER. Both
            // figures are on the wire beside it, so this adds no fact -- it
            // removes the chance of two integers on different scales being
            // eyeballed against each other. `t_milli` can be negative for a
            // short's evidence; the bar is on |t|.
            row.t_milli.saturating_abs() >= bar_milli,
        ),
    );
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]
mod tests {
    use super::respond;

    /// A store with no live directory answers empty, and creates nothing.
    #[test]
    fn a_store_with_nothing_in_flight_answers_empty() {
        let root = std::env::temp_dir().join(format!(
            "brutex-livejson-empty-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("a temp root");

        let (status, _, body) = respond(Ok(root.clone()));
        assert_eq!(status, axum::http::StatusCode::OK);
        assert_eq!(
            body, r#"{"runs":[],"count":0,"skipped":0,"listed":true,"refusal":null}"#,
            "nothing in flight is an empty list, not a refusal, and a live \
             directory that was never created is LISTED and empty rather than \
             unreadable: {body}"
        );
        assert!(
            !root.join("results").exists(),
            "asking what is running must not create a store"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// A published run is served with its bar, and the rows are NOT judged.
    ///
    /// The `meets` block `/frontier.json` carries would be computed from eight
    /// structural zeros here, because `publish_ranked` writes before the exit
    /// grid runs. Its absence is the assertion.
    #[test]
    fn a_run_in_flight_carries_its_bar_and_is_not_given_a_verdict() {
        let root = std::env::temp_dir().join(format!(
            "brutex-livejson-run-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("a temp root");

        let identity = [0x7c_u8; 32];
        let mut live = cli::live::Live::open(&root, &identity).expect("a live file opens");
        let row = cli::frontier::Row {
            identity,
            rank: 1,
            mask_words: [9, 0, 0, 0, 0, 0],
            hits: 4_395,
            n: 4_070,
            mean_milli_paisa: 12_300,
            t_milli: 1_802,
            payoff_bp: 140,
            wins: 2_100,
            // EVERYTHING THE GRID WOULD FILL IS ZERO, which is the state this
            // route exists to serve honestly.
            trades: 0,
            cell_wins: 0,
            pessimistic: 0,
            worst_trade: 0,
            max_drawdown: 0,
            min_win: 0,
            gross_win: 0,
            gross_loss: 0,
            direction: cli::frontier::Direction::Short,
            rules: cli::Rules::elite(400, 25),
        };
        live.publish(
            &[row],
            cli::live::Summary {
                trials: 3_572_851,
                bar_milli: 5_673,
                priced: 0,
            },
        )
        .expect("the rows publish");

        let (status, _, body) = respond(Ok(root.clone()));
        assert_eq!(status, axum::http::StatusCode::OK);

        assert!(
            body.contains(r#""trials":3572851"#),
            "the trial count: {body}"
        );
        assert!(
            body.contains(r#""bar_milli":5673"#),
            "and the bar those trials imply, beside the rows it judges: {body}"
        );
        assert!(
            body.contains(r#""t_milli":1802"#) && body.contains(r#""clears_bar":false"#),
            "1.802 against a bar of 5.673 does not clear, and the answer is on \
             the wire rather than left to be eyeballed: {body}"
        );
        assert!(
            body.contains(r#""ranked_only":true"#),
            "the payload says the grid half is absent: {body}"
        );
        assert!(
            !body.contains(r#""meets""#) && !body.contains(r#""win_rate_bp""#),
            "a verdict computed from eight structural zeros is the failure \
             wearing a measurement's clothes: {body}"
        );
        assert!(
            body.contains(r#""direction":"short""#),
            "and the side, without which no row is actionable: {body}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }
}

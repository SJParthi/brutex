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
//! `frontier::Row` v3 stores eight RAW money fields and no derived ones. The
//! derived quantities — win rate, reward-to-risk, return over drawdown, average
//! win, average loss — are NOT computed here either. `cli::frontier::Row::derived`
//! rebuilds a `grid::Cell` from the six stored fields and asks IT, so there is
//! one definition of "win rate" in this repository and not a second in this file
//! or a third in JavaScript. `api` cannot reach `runner` at all — the arrow is
//! `api -> cli` — which is what forced the right shape rather than merely
//! suggesting it. That is the same argument `/vocab.json` makes for
//! serving the condition table once rather than copying it into the page.
//!
//! # The verdict, and the one rule it refuses to answer
//!
//! `cli::record_frontier` writes `by_evidence.iter().take(top)` and consults NO
//! rule at all, so every row arrived here looking like a candidate when the list
//! is really *"the top `top` by ranking lens"*. Each row therefore carries
//! `meets`, from `cli::frontier::Row::verdict`, and the envelope echoes the
//! `rules` those verdicts were taken against.
//!
//! `meets` has no `stop` field. `Rules::max_mae_ppm` is checked against
//! `grid::Cell::worst_mae`, which a row does not store — calling `Rules::admits`
//! here would have reported the stop rule PASSED on every row on the strength of
//! a defaulted zero. `stop_unchecked` is `true` and the threshold is echoed, so
//! the page can say *"not checked"* where a tick would have been a lie.
//!
//! # `mask_words` are decimal STRINGS
//!
//! They were bare JSON numbers, and a `u64` above 2^53 does not survive
//! `JSON.parse` — the browser would decode a DIFFERENT condition set, silently.
//! `/backtest.json` already quotes them for exactly this reason and its own
//! comment says so; this route now matches it, and the page reads both with
//! `BigInt`.
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

    // THE RULES ARE READ ONCE, FROM THE CRATE THAT DEFINES THEM. `Rules::operator`
    // resolves its six numbers from the environment at call time, so a run
    // launched with `BRUTEX_MIN_WIN_RATE_BP=9000` is judged against 9,000 and not
    // against a threshold copied into this file or into JavaScript. That copy is
    // the failure `CLAUDE.md` §5 refuses -- two definitions of one fact, correct
    // the day they are written.
    let rules = cli::Rules::operator();

    let mut out = String::with_capacity(rows.len().saturating_mul(420).saturating_add(320));
    out.push_str(r#"{"rows":["#);
    let admitted = write_rows(&mut out, &rows, &rules);
    envelope(&mut out, rows.len(), admitted, &rules, partial);
    (axum::http::StatusCode::OK, json, out)
}

/// Every row of one run, and how many of them met the rules.
///
/// Split out of [`respond`] because that function crossed the hundred-line bar
/// clippy holds it to, and the loop is the half with one job.
fn write_rows(out: &mut String, rows: &[cli::frontier::Row], rules: &cli::Rules) -> usize {
    let mut admitted = 0_usize;
    for (at, row) in rows.iter().enumerate() {
        if at > 0 {
            out.push(',');
        }
        // WHICH OF THE OPERATOR'S RULES THIS ROW MEETS, asked of `cli` rather
        // than decided here. `record_frontier` writes the top `top` by ranking
        // lens and consults NO rule, so without this every row arrived looking
        // like a candidate. The verdict does not remove rows -- it labels them,
        // and a run where nothing passes now says so instead of rendering a
        // table that looks like a working answer.
        let v = row.verdict(rules);
        if v.admitted {
            admitted = admitted.saturating_add(1);
        }
        // ASKED OF THE ENGINE, NOT RECOMPUTED HERE. `Row::derived` rebuilds a
        // `grid::Cell` from the six stored fields and asks IT for the five
        // derived ones, so there is one definition of "win rate" and not a
        // second written in this file. `api` cannot reach `runner` at all --
        // the arrow is `api -> cli` -- which is what forced the right shape.
        let d = row.derived();
        let _ = std::fmt::Write::write_fmt(
            &mut *out,
            format_args!(
                r#"{{"rank":{},"mask_words":["{}","{}","{}","{}","{}","{}"],"hits":{},"n":{},"mean_milli_paisa":{},"t_milli":{},"payoff_bp":{},"edge_wins":{},"priced":{},"trades":{},"wins":{},"losses":{},"pessimistic":{},"worst_trade":{},"max_drawdown":{},"min_win":{},"win_rate_bp":{},"reward_to_risk_bp":{},"return_over_drawdown":{},"avg_win":{},"avg_loss":{},"gross_win":{},"gross_loss":{},"meets":{{"win_rate":{},"reward_to_risk":{},"return_over_drawdown":{},"trades":{},"assurance":{},"break_even":{},"all":{},"stop_unchecked":{}}}}}"#,
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
                measurable(d.reward_to_risk_bp),
                measurable(d.return_over_drawdown),
                d.avg_win,
                d.avg_loss,
                row.gross_win,
                row.gross_loss,
                v.win_rate,
                v.reward_to_risk,
                v.return_over_drawdown,
                v.trades,
                v.assurance,
                v.break_even,
                v.admitted,
                v.stop_unchecked,
            ),
        );
    }
    admitted
}

/// The envelope: how many rows, how many passed, what they were judged against.
///
/// A PARTIAL READ IS REPORTED, NOT ROUNDED OFF. `of_run` returns the rows it
/// could read AND why it stopped; dropping the second would turn a torn tail
/// into a shorter answer that looks complete.
///
/// THE THRESHOLDS TRAVEL WITH THE ANSWER. A `PASS` means nothing without the
/// bar it cleared, and an operator who set `BRUTEX_MIN_WIN_RATE_BP` on the
/// request needs to see the number the rows were actually judged against
/// rather than the one the form's placeholder claims -- two of those
/// placeholders were measured wrong, one of them by a factor of a hundred.
///
/// `max_mae_ppm` is echoed and is deliberately NOT in any row's verdict:
/// `Cell::worst_mae` is not a stored field, so the stop rule cannot be
/// answered from a row. Sending the threshold while sending no verdict for it
/// is what lets the page say "not checked" instead of drawing a tick.
fn envelope(
    out: &mut String,
    count: usize,
    admitted: usize,
    rules: &cli::Rules,
    partial: Option<String>,
) {
    let _ = std::fmt::Write::write_fmt(
        &mut *out,
        format_args!(
            r#"],"count":{},"admitted":{},"rules":{{"min_win_rate_bp":{},"min_rr_bp":{},"min_ret_over_dd_bp":{},"min_trades":{},"min_assurance_bp":{},"max_mae_ppm":{},"top":{}}},"refusal":{}}}"#,
            count,
            admitted,
            rules.min_win_rate_bp,
            rules.min_rr_bp,
            rules.min_ret_over_dd_bp,
            rules.min_trades,
            rules.min_assurance_bp,
            rules.max_mae_ppm,
            rules.top,
            partial.map_or_else(|| "null".to_owned(), |why| crate::render::json_string(&why))
        ),
    );
}

/// A ratio the data could not settle, as `null` rather than as `i64::MAX`.
///
/// # Why this exists, and what it was doing before
///
/// `Cell::reward_to_risk_bp` returns `i64::MAX` when `worst_trade` is zero —
/// **a combination that never lost a trade** — and `return_over_drawdown` does
/// the same when there was no drawdown. Both are honest answers to "divide by
/// nothing". Both were being written into the JSON as the bare literal
/// `9223372036854775807`.
///
/// MEASURED against the page this route exists to feed: the browser ranks by
/// normalising each measurement to its position within the set, which needs the
/// set's range. ONE unbeaten row sets that range to 9.2e18, every other row's
/// term collapses to approximately zero, and the criterion silently leaves the
/// ranking. The operator can move the weight all they like; the term is already
/// gone. The same row also rendered as `92233720368547758.00x`.
///
/// `null` is the shape this repository already uses for exactly this: the
/// `/store.json` change ratio is `null` beside a reason code, and its own doc
/// says *"a ratio with no base is undefined, never `0`, never `∞`"*. Zero would
/// have been worse than the sentinel — it reads as "measured, and bad".
fn measurable(value: i64) -> String {
    if value == i64::MAX {
        "null".to_owned()
    } else {
        value.to_string()
    }
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

    /// A ranked row carries its verdict, and the envelope carries the bar.
    ///
    /// # What this is guarding
    ///
    /// `cli::record_frontier` writes `by_evidence.iter().take(top)` and reads no
    /// rule at all, so every row reaching the browser looked like a candidate.
    /// The row seeded here is the one this operator's store actually held at
    /// rank 1 — 868 trades, 3 won, smallest win 295 paisa against a worst loss
    /// of 3,035 — and the assertion is that the wire now says FAIL on it.
    ///
    /// The envelope's `rules` matters as much as the verdict: a `PASS` with no
    /// threshold beside it is unreadable, and the form's own placeholder for one
    /// of these was wrong by a factor of a hundred.
    #[test]
    fn a_ranked_row_carries_its_verdict_and_the_rules_it_was_judged_against() {
        let dir = std::env::temp_dir().join(format!(
            "brutex-api-frontier-verdict-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let identity = [0x5a_u8; 32];
        let mut store = cli::frontier::Frontier::open(&dir).expect("a fresh frontier opens");
        store
            .append_all(&[cli::frontier::Row {
                identity,
                rank: 1,
                mask_words: [3, 0, 0, 0, 0, 0],
                hits: 1_194,
                n: 868,
                mean_milli_paisa: 345_939,
                t_milli: 2_706,
                payoff_bp: 129,
                wins: 445,
                trades: 868,
                cell_wins: 3,
                pessimistic: -272_749,
                worst_trade: -3_035,
                max_drawdown: 329_165,
                min_win: 295,
                gross_win: 900,
                gross_loss: -273_649,
            }])
            .expect("one row appends");

        let query = format!("identity={}", "5a".repeat(32));
        let (status, _, body) = respond(Ok(dir.clone()), &query);
        assert_eq!(status, axum::http::StatusCode::OK);

        assert!(
            body.contains(r#""meets":{"#),
            "the verdict is on the row: {body}"
        );
        // THE TWO TYPED FLOORS ARE GONE, so both of these now pass vacuously
        // against a zero default. The row is still refused, and the clause that
        // refuses it is the one nobody typed.
        assert!(
            body.contains(r#""win_rate":true"#),
            "a zero floor is cleared by any rate: {body}"
        );
        assert!(
            body.contains(r#""reward_to_risk":true"#),
            "a zero floor is cleared by any ratio: {body}"
        );
        assert!(
            body.contains(r#""break_even":false"#),
            "0.09x reward-to-risk needs 91.74% wins to break even; this won \
             0.34%: {body}"
        );
        assert!(
            body.contains(r#""all":false"#),
            "so the row is not admitted: {body}"
        );
        assert!(
            body.contains(r#""stop_unchecked":true"#),
            "and the stop rule is named as unchecked, never as passed: {body}"
        );
        assert!(body.contains(r#""admitted":0"#), "none passed: {body}");
        assert!(
            body.contains(r#""rules":{"min_win_rate_bp":"#),
            "the thresholds travel with the answer: {body}"
        );
        // 64-BIT SAFE ON THE WIRE. A bare JSON number above 2^53 does not
        // survive `JSON.parse`, and a mask decoded from a rounded word names the
        // WRONG conditions while looking exactly like an answer.
        assert!(
            body.contains(r#""mask_words":["3","0""#),
            "mask words are decimal strings: {body}"
        );
        let _ = std::fs::remove_dir_all(&dir);
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

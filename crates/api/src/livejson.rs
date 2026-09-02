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
//!
//! # `stale` is EVIDENCE, and the payload is shaped so it cannot be read as more
//!
//! `cli::live::Live::finish` is the only thing that removes a live file and it
//! runs only on the success path — there is no `Drop` — so a run killed by a
//! signal leaves a file that this route served as in-flight forever. MEASURED on
//! 2026-09-01: one 6,840-byte file, 25 rows, written at 21:41, still listed here
//! hours after the process was gone.
//!
//! What is served is what can be measured: `idle_secs`, seconds since the file
//! was last written, and `stale`, whether that crossed
//! `cli::live::STALE_AFTER_SECS` — a day. **Neither says the run is dead.** The
//! writer publishes ONCE, immediately before an exit grid that is 87.6% of a
//! run's wall clock, so an untouched file is the normal state for most of a
//! healthy run; only a heartbeat inside that grid could answer the real
//! question, and there is none. §3 rule 6.
//!
//! Three consequences for the shape:
//!
//! * `stale` is `true`, `false` or **`null`** — `null` when the file carries no
//!   usable modification time. A `false` there would be a measurement nobody
//!   took, served as a clean bill of health.
//! * `stale_after_secs` is on the wire beside the counts, so a reader who wants
//!   a tighter line has the threshold AND the raw `idle_secs` and does not have
//!   to take this route's word for it.
//! * `undated` counts the runs with no answer, so `"stale":0` cannot be read as
//!   "and every file was checked".

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

    let mut out = String::with_capacity(runs.len().saturating_mul(2_400).saturating_add(128));
    out.push_str(r#"{"runs":["#);
    for (nth, run) in runs.iter().enumerate() {
        if nth > 0 {
            out.push(',');
        }
        write_run(&mut out, run);
    }
    // `strays` AND `skipped` ARE TWO FACTS AND ARE SERVED AS TWO. `skipped`
    // means "something is there and I cannot read it" — a corruption alarm —
    // and a temp file from a writer killed between its flush and its rename is
    // a complete, valid live file under a name that is not a run's. Counting it
    // as corruption made this route cry wolf about the one thing it is
    // guaranteed to find beside the live files.
    let _ = std::fmt::Write::write_fmt(
        &mut out,
        format_args!(
            r#"],"count":{},"skipped":{},"strays":{},"stale":{},"undated":{},"stale_after_secs":{},"listed":{},"refusal":null}}"#,
            runs.len(),
            census.skipped,
            census.strays,
            census.stale,
            census.undated,
            cli::live::STALE_AFTER_SECS,
            census.listed
        ),
    );
    (axum::http::StatusCode::OK, json, out)
}

/// One in-flight run: what it has weighed, what it must clear, its top rows,
/// and when anything last wrote to its file.
///
/// `idle_secs` and `stale` are both `null` when the file could not be dated —
/// see this module's header. `stale: null` is not `stale: false`, deliberately:
/// one says "I could not tell", the other says "I checked, and it is fine".
fn write_run(out: &mut String, run: &cli::live::Run) {
    let _ = std::fmt::Write::write_fmt(
        out,
        format_args!(
            r#"{{"identity":"{}","trials":{},"bar_milli":{},"priced":{},"ranked_only":true,"idle_secs":{},"stale":{},"rows":["#,
            crate::server::hex32(run.identity),
            run.summary.trials,
            run.summary.bar_milli,
            run.summary.priced,
            run.freshness
                .idle_secs()
                .map_or_else(|| "null".to_owned(), |secs| secs.to_string()),
            run.freshness
                .is_stale()
                .map_or_else(|| "null".to_owned(), |yes| yes.to_string()),
        ),
    );
    for (nth, row) in run.rows.iter().enumerate() {
        if nth > 0 {
            out.push(',');
        }
        write_row(out, row, run.summary.bar_milli);
    }
    let _ = std::fmt::Write::write_fmt(out, format_args!(r#"],"kept":{}}}"#, run.rows.len()));
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
            body,
            r#"{"runs":[],"count":0,"skipped":0,"strays":0,"stale":0,"undated":0,"stale_after_secs":86400,"listed":true,"refusal":null}"#,
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

    /// A store that could not be resolved refuses, and carries NO counts.
    ///
    /// `strays`, `stale` and `undated` are absent here rather than zero.
    /// Nothing was opened, so a zero beside a non-null `refusal` would be three
    /// measurements nobody took — the shape the rest of this module exists to
    /// refuse.
    #[test]
    fn a_store_that_cannot_be_resolved_refuses_rather_than_reporting_zero() {
        let (status, _, body) = respond(Err("no HOME, so no store root".to_owned()));
        assert_eq!(
            status,
            axum::http::StatusCode::OK,
            "a refusal is still an answer"
        );
        assert!(
            body.contains(r#""refusal":"no HOME, so no store root""#),
            "the reason is named rather than rendered as an idle machine: {body}"
        );
        assert!(
            !body.contains(r#""stale""#) && !body.contains(r#""strays""#),
            "and no count is served for a directory that was never opened: {body}"
        );
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

        // WHEN THE FILE WAS LAST WRITTEN IS ON THE WIRE, and the threshold it
        // was judged against with it — a reader that disagrees with a day has
        // the raw seconds and does not have to take this route's word for it.
        assert!(
            body.contains(r#""idle_secs":"#) && !body.contains(r#""idle_secs":null"#),
            "a file this test just wrote has a modification time, so the idle \
             seconds are a number and not an absence: {body}"
        );
        assert!(
            body.contains(r#""stale":false"#) && body.contains(r#""stale_after_secs":86400"#),
            "and it has not crossed the threshold, which is published beside \
             it: {body}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// **A temp file from a killed writer is not a corrupt file, and this route
    /// used to report it as one.**
    ///
    /// `cli::live::Live::publish` writes `<hex>.<pid>.tmp` and renames it over
    /// the real name. A process killed in that window leaves a COMPLETE, VALID
    /// live file under a name that is not a run's — and the reader iterated
    /// every directory entry with no filter, so it either listed the same run
    /// twice (a temp file carrying rows, whose row-zero identity it trusted) or
    /// added to `skipped`, which this payload publishes as "something is there
    /// and I cannot read it".
    ///
    /// The empty temp file below is the second case, and it is the one this
    /// route can be wrong about on its own: `cli::live::Live::open` writes a
    /// header-only file, so a kill in the first instant of a run leaves exactly
    /// this. The double-listing half is pinned in `cli::live`'s own tests,
    /// where a `Row` can be built without repeating eighteen fields here.
    #[test]
    fn a_stray_temp_file_is_counted_apart_from_a_corrupt_one() {
        let root = std::env::temp_dir().join(format!(
            "brutex-livejson-stray-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("a temp root");

        let identity = [0x3d_u8; 32];
        let mut live = cli::live::Live::open(&root, &identity).expect("a live file opens");
        live.publish(&[], cli::live::Summary::default())
            .expect("the empty file publishes");

        // The killed writer's leftovers: a byte-for-byte copy of a good live
        // file under the temp name it would have been renamed from.
        let real = cli::live::Live::path(&root, &identity);
        let bytes = std::fs::read(&real).expect("readable");
        std::fs::write(
            real.with_extension(format!("{}.tmp", std::process::id())),
            &bytes,
        )
        .expect("writable");

        let (_, _, body) = respond(Ok(root.clone()));
        assert!(
            body.contains(r#""count":1"#),
            "one run has one file however many copies of it are beside it: {body}"
        );
        assert!(
            body.contains(r#""skipped":0"#),
            "and a stray is NOT a corruption alarm: {body}"
        );
        assert!(
            body.contains(r#""strays":1"#),
            "it is counted, and counted apart — an uncounted stray would be the \
             silent skip the census exists to refuse: {body}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }
}

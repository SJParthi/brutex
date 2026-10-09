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
//! `frontier::Row` v4 stores eight RAW money fields and no derived ones. The
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
//! the page can say *"not checked"* where a tick would have been a lie. The
//! protective-exit and fill-headroom rules are named the same way.
//!
//! **The rule names are served, not restated (D-1810).** `meets` is written
//! from `cli::frontier::Verdict::checked` and `unchecked`, and the envelope's
//! `admission` member lists `cli::frontier::VERDICT_CHECKED` and
//! `VERDICT_UNCHECKED`. The browser checks `meets.all` against that list and
//! recomputes no rule, as `/vocab.json` lets it decode masks without a copy of
//! the vocabulary (D-0288).
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
//! After indexes exist, one hash probe finds the run's block and reading it is
//! linear in the selected rows. Cold opens index historical frontier, ledger
//! and receipt rows. Warm requests refresh only appended rows, or first
//! re-hash every indexed ledger or receipt byte when either file grew
//! (O(indexed bytes + new rows), D-1560, D-3305, D-3318, D-3320), retaining
//! generation checks and a parent snapshot before touching children. These
//! refreshes and selected-row reads are not O(1) overall. The HTTP door caps
//! every indexed file at 64 MiB, one
//! verified result at 4,096 rows, one page at 256 rows and encoded JSON at 8
//! MiB. Four detail tasks across both routes may be queued or running. The
//! blocking file/index/encoding path runs on `spawn_blocking`; saturation
//! answers 429 before another task is queued. D-0435 and limits §125.
//!
//! **UNVERIFIED as a measured bound.** No bench in this workspace
//! times this, so the shape above is read from the source rather
//! than measured. `CLAUDE.md` §3 rule 6.

/// The headers every JSON route here answers with.
type JsonHeaders = [(axum::http::header::HeaderName, &'static str); 1];

/// One run's ranked combinations, with every measurement the operator ranks on.
pub async fn frontier_json(uri: axum::http::Uri) -> (axum::http::StatusCode, JsonHeaders, String) {
    let query = uri.query().unwrap_or("");
    if let Err(why) = crate::detail::query_is_bounded(query) {
        return refuse(json_headers(), &why);
    }
    // PARSED BEFORE ADMISSION, so a selector that can only be refused never
    // takes a detail slot and is never answered 429 in place of its 400.
    // D-0689; `detail::Selector` keeps the blocking path's order and words.
    let asked = match crate::detail::Selector::parse(crate::server::store_dir(), query) {
        Ok(asked) => asked,
        Err(why) => return refuse(json_headers(), &why),
    };
    match crate::detail::run(move || respond(asked)).await {
        Ok(response) => response,
        Err(crate::detail::RunError::Saturated) => unavailable(
            axum::http::StatusCode::TOO_MANY_REQUESTS,
            "detail read capacity is full; no blocking task was queued. Retry after another trade/frontier request finishes",
        ),
        Err(crate::detail::RunError::Join(why)) => unavailable(
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            &format!("the bounded frontier-detail task could not be joined: {why}"),
        ),
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "one ordered receipt-child-validation-page transaction keeps every early refusal beside the operation it protects"
)]
fn respond(asked: crate::detail::Selector) -> (axum::http::StatusCode, JsonHeaders, String) {
    let json = json_headers();
    let crate::detail::Selector {
        root,
        identity,
        page,
    } = asked;

    let ledger_path = cli::results::Results::path(&root);
    let receipt_path = cli::result_set::Receipts::path(&root);
    let frontier_path = cli::frontier::Frontier::path(&root);
    if let Err(why) = crate::detail::preflight([
        ("results ledger", ledger_path.as_path()),
        ("detail receipts", receipt_path.as_path()),
        ("frontier", frontier_path.as_path()),
    ]) {
        return unavailable(axum::http::StatusCode::SERVICE_UNAVAILABLE, &why);
    }

    let committed = match crate::detail::committed_receipt(&root, &identity) {
        Ok(committed) => committed,
        Err(why) => return refuse(json, &why),
    };
    // THESE ARE A RUN'S RANKED COMBINATIONS, AND A STOCK'S SAY WHAT THEY ARE
    // MADE OF. The ledger parent the receipt read confirms names the
    // instrument, so a RELIANCE run's frontier carries the gross label and the
    // corporate-action sentence `cli top` prints over the same rows. Decided
    // from this one snapshot; an index run's payload gains no key. AF-19.
    // AND EVERY RECORDED RUN'S ROWS ARE IN SAMPLE, validation unrecorded.
    // Carried in the same member string so every body that names the note
    // names this too. D-2792 (CE-93).
    let note = format!(
        r#"{},"in_sample":{}"#,
        crate::detail::equity_note_member(
            committed
                .as_ref()
                .map(|committed| committed.underlying.as_str()),
        ),
        crate::render::json_string(cli::LEDGER_IN_SAMPLE)
    );
    let receipt = committed.map(|committed| committed.receipt);
    if let Some(committed) = receipt
        && committed.frontier_rows > crate::detail::MAX_RESULT_ROWS
    {
        return too_large(
            json,
            &identity,
            committed.frontier_rows,
            &format!(
                "run {} commits {} frontier rows; one request verifies at most {}. No prefix was read or exposed",
                crate::server::hex32(identity),
                committed.frontier_rows,
                crate::detail::MAX_RESULT_ROWS
            ),
        );
    }

    // READ-ONLY, so a GET on a store that has never been swept cannot answer
    // "no combinations" by creating the file that makes it true -- the opener
    // is `open_read_bounded` and nothing else.
    //
    // Held for the process and REFRESHED per request rather than opened fresh:
    // a fresh open walked every frontier row ever written to return ten. See
    // `detail::Cached` for the ordering argument and the refusal path.
    let (rows, partial) = match crate::detail::FRONTIER.with_verified(
        &root,
        || cli::frontier::Frontier::open_read_bounded(&root, crate::detail::MAX_SCAN_BYTES),
        cli::frontier::Frontier::refresh,
        |file| file.of_run_against_receipt(&identity, receipt),
    ) {
        Ok(Ok(pair)) => pair,
        Ok(Err(why)) => return refuse(json, &why),
        Err(why) => {
            return missing_file_response(&root, &identity, receipt, &note, page, json, &why);
        }
    };
    if receipt.is_none() {
        return absent_response(
            json,
            &identity,
            page,
            &format!(
                "run {} has no committed result-set receipt at this request's canonical read. No clean empty or orphan frontier answer is exposed",
                crate::server::hex32(identity)
            ),
        );
    }
    let window = match crate::detail::window(rows.len(), page) {
        Ok(window) => window,
        Err(why) => return range_refusal(json, &identity, rows.len(), &why),
    };
    let Some(page_rows) = rows.get(window.start..window.end) else {
        return refuse(
            json,
            "the validated frontier page window did not fit its verified row vector; no fallback page is exposed",
        );
    };

    // THE RULES ARE THE RUN'S OWN, READ OFF THE ROW IT WROTE THEM ON.
    //
    // This read `cli::Rules::operator()` -- resolved from the environment AT
    // REQUEST TIME -- and the comment above it argued that this avoided a copied
    // threshold. It did avoid that. What it did instead was judge these rows
    // against a rule set NO RUN EVER APPLIED, and three separate mechanisms
    // guaranteed the two differed:
    //
    // * the run swept at `Rules::derived(&span.bars, horizon)`, which MEASURES
    //   four of its floors off the bars -- and this handler has an identity and
    //   a row, not a span, so they were unrecoverable here;
    // * `sweeprun::Applied::drop` calls `knobs::clear_all`, so a floor the
    //   operator typed into the form steered the sweep and was GONE before the
    //   browser fetched the result, leaving `operator()` reading its built-in
    //   defaults;
    // * `/backtest/descend` reaches `screen_range_inner`, which uses
    //   `Rules::elite(..)` -- differing from `operator()` on THREE of the five
    //   rules `verdict` checks, unconditionally, on every run it produces.
    //
    // Both directions were reachable. A row the run REFUSED rendered PASS and a
    // row it ADMITTED rendered FAIL, with the wrong threshold printed beside it
    // in the same response -- a fallback that hides a failure, §4's ban.
    //
    // ONE `Rules` PER RUN AND NOT PER ROW, even though the field is per row:
    // `append_all` writes a run's whole frontier in one call under one lock from
    // one value, so every row of a block carries the same set, and `policy_of`
    // folds all eight into the run identity, so two runs sharing an identity
    // shared their rules. `None` when there are no rows, which is the only case
    // where there is nothing to read it from and also the only case where
    // nothing needs judging.
    let rules = rows.first().map(|row| row.rules);

    let mut out = String::with_capacity(page_rows.len().saturating_mul(640).saturating_add(640));
    let _ = std::fmt::Write::write_fmt(
        &mut out,
        format_args!(
            r#"{{"identity":"{}"{note},"rows":["#,
            crate::server::hex32(identity)
        ),
    );
    let admitted = rules.map_or(0, |set| write_rows(&mut out, page_rows, &set));
    let total_admitted = rules.map_or(0, |set| {
        rows.iter().filter(|row| row.verdict(&set).admitted).count()
    });
    let page_refusal = (!window.complete).then(|| {
        format!(
            "partial page only: page {} returns rows {}..{} of {}. Fetch every page and reconcile `total_count`; this response is not a complete frontier",
            page.number,
            window.start,
            window.end,
            rows.len()
        )
    });
    let partial = partial.or(page_refusal);
    envelope(
        &mut out,
        page_rows.len(),
        rows.len(),
        admitted,
        total_admitted,
        rules.as_ref(),
        page,
        window,
        partial,
    );
    if out.len() > crate::detail::MAX_RESPONSE_BYTES {
        return too_large(
            json,
            &identity,
            u64::try_from(rows.len()).unwrap_or(u64::MAX),
            &format!(
                "the bounded frontier response encoded to {} bytes; the hard response ceiling is {}. No oversized JSON was returned",
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

/// Distinguishes an unswept store from a committed child deleted afterwards.
///
/// `note` is the committed run's [`crate::detail::equity_note_member`]: a
/// stock's committed empty frontier still says what its run is made of.
fn missing_file_response(
    root: &std::path::Path,
    identity: &[u8; 32],
    receipt: Option<cli::result_set::Receipt>,
    note: &str,
    page: crate::detail::Page,
    json: JsonHeaders,
    why: &str,
) -> (axum::http::StatusCode, JsonHeaders, String) {
    let child = cli::frontier::Frontier::path(root);
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
        Some(receipt) if receipt.frontier_rows == 0 => {
            if let Err(range) = crate::detail::window(0, page) {
                return range_refusal(json, identity, 0, &range);
            }
            (
                axum::http::StatusCode::OK,
                json,
                format!(
                    r#"{{"identity":"{}"{note},"rows":[],"count":0,"total_count":0,"admitted":0,"total_admitted":0,"rules":null,"page":{},"limit":{},"page_complete":true,"complete":true,"next_page":null,"refusal":null}}"#,
                    crate::server::hex32(*identity),
                    page.number,
                    page.limit
                ),
            )
        }
        Some(receipt) => refuse(
            json,
            &format!(
                "run {} is committed for {} frontier row(s), but frontier.bin is missing: {why}. No empty or partial answer is exposed",
                crate::server::hex32(*identity),
                receipt.frontier_rows
            ),
        ),
        None => absent_response(json, identity, page, why),
    }
}

fn absent_response(
    json: JsonHeaders,
    identity: &[u8; 32],
    page: crate::detail::Page,
    why: &str,
) -> (axum::http::StatusCode, JsonHeaders, String) {
    if let Err(range) = crate::detail::window(0, page) {
        return range_refusal(json, identity, 0, &range);
    }
    (
        axum::http::StatusCode::OK,
        json,
        format!(
            r#"{{"identity":"{}","rows":[],"count":0,"total_count":0,"admitted":0,"total_admitted":0,"rules":null,"page":{},"limit":{},"page_complete":true,"complete":true,"next_page":null,"refusal":{}}}"#,
            crate::server::hex32(*identity),
            page.number,
            page.limit,
            crate::render::json_string(why)
        ),
    )
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
                r#"{{"rank":{},"direction":"{}","mask_words":["{}","{}","{}","{}","{}","{}"],"hits":{},"n":{},"mean_milli_paisa":{},"t_milli":{},"payoff_bp":{},"edge_wins":{},"priced":{},"trades":{},"wins":{},"losses":{},"pessimistic":{},"worst_trade":{},"max_drawdown":{},"min_win":{},"win_rate_bp":{},"reward_to_risk_bp":{},"return_over_drawdown":{},"avg_win":{},"avg_loss":{},"gross_win":{},"gross_loss":{},"meets":{{"#,
                row.rank,
                row.direction.as_str(),
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
                payoff_on_wire(row.n, row.payoff_bp),
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
            ),
        );
        write_meets(out, &v);
    }
    admitted
}

/// One row's `meets` object, written from the verdict's own named lists.
///
/// THE NAMES ARE `cli`'s, NOT THIS FILE'S (D-1810). Each answered rule is
/// `cli::frontier::Verdict::checked`'s name and value, then `all`, then each
/// unanswerable rule as `<name>_unchecked`. A rule added to the verdict
/// reaches the wire, and the envelope's `admission` list, without an edit
/// here, which is what keeps the browser from carrying a copy of the rule
/// set (W2-cli5-4). Nine names and one conjunction per row, a fixed count
/// read from the source; UNVERIFIED as a timed bound (see the D-1810 entry
/// in `docs/06-limits.md`).
fn write_meets(out: &mut String, v: &cli::frontier::Verdict) {
    for (name, holds) in v.checked() {
        let _ = std::fmt::Write::write_fmt(&mut *out, format_args!(r#""{name}":{holds},"#));
    }
    let _ = std::fmt::Write::write_fmt(&mut *out, format_args!(r#""all":{}"#, v.admitted));
    for (name, unchecked) in v.unchecked() {
        let _ = std::fmt::Write::write_fmt(
            &mut *out,
            format_args!(r#","{name}_unchecked":{unchecked}"#),
        );
    }
    out.push_str("}}");
}

/// The envelope's `admission` member: which rules `meets.all` conjoins and
/// which it cannot answer, from `cli::frontier::VERDICT_CHECKED` and
/// `VERDICT_UNCHECKED`. Served once per response so the browser verifies
/// `all` against this list rather than a list of its own (D-1810).
fn admission_json() -> String {
    let quoted = |names: &[&str]| {
        names
            .iter()
            .map(|name| format!(r#""{name}""#))
            .collect::<Vec<_>>()
            .join(",")
    };
    format!(
        r#"{{"checked":[{}],"unchecked":[{}]}}"#,
        quoted(&cli::frontier::VERDICT_CHECKED),
        quoted(&cli::frontier::VERDICT_UNCHECKED)
    )
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
#[expect(
    clippy::too_many_arguments,
    reason = "the JSON envelope spells each reconciled count and page fact once at its single rendering boundary"
)]
fn envelope(
    out: &mut String,
    count: usize,
    total_count: usize,
    admitted: usize,
    total_admitted: usize,
    // `None` ONLY when there are no rows, because the rules are read off a row.
    // Served as `"rules":null` rather than as `operator()`'s defaults: a page
    // that printed thresholds for a run it has no rows from would be stating a
    // policy nothing applied, which is the defect this whole field exists to
    // close. Both render sites in `web/` are already guarded on the key.
    rules: Option<&cli::Rules>,
    page: crate::detail::Page,
    window: crate::detail::Window,
    partial: Option<String>,
) {
    let next = window
        .next_page
        .map_or_else(|| "null".to_owned(), |value| value.to_string());
    let refusal = partial.map_or_else(|| "null".to_owned(), |why| crate::render::json_string(&why));
    let Some(rules) = rules else {
        let _ = std::fmt::Write::write_fmt(
            &mut *out,
            format_args!(
                r#"],"count":{},"total_count":{},"admitted":{},"total_admitted":{},"rules":null,"admission":{},"page":{},"limit":{},"page_complete":true,"complete":{},"next_page":{},"refusal":{}}}"#,
                count,
                total_count,
                admitted,
                total_admitted,
                admission_json(),
                page.number,
                page.limit,
                window.complete,
                next,
                refusal
            ),
        );
        return;
    };
    let _ = std::fmt::Write::write_fmt(
        &mut *out,
        format_args!(
            r#"],"count":{},"total_count":{},"admitted":{},"total_admitted":{},"rules":{{"min_win_rate_bp":{},"min_rr_bp":{},"min_ret_over_dd_bp":{},"min_trades":{},"min_assurance_bp":{},"max_mae_ppm":{},"min_avg_rr_bp":{},"min_fill_headroom_bp":{},"top":{}}},"admission":{},"page":{},"limit":{},"page_complete":true,"complete":{},"next_page":{},"refusal":{}}}"#,
            count,
            total_count,
            admitted,
            total_admitted,
            rules.min_win_rate_bp,
            rules.min_rr_bp,
            rules.min_ret_over_dd_bp,
            rules.min_trades,
            rules.min_assurance_bp,
            rules.max_mae_ppm,
            rules.min_avg_rr_bp,
            rules.min_fill_headroom_bp,
            rules.top,
            admission_json(),
            page.number,
            page.limit,
            window.complete,
            next,
            refusal
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

/// `payoff_bp` as the wire carries it: `null` for each of its two absences
/// that the row can name exactly (p5num-5, D-2568).
///
/// `runner::outcome::Edge::payoff_bp` answers [`i64::MAX`] for a combination
/// that never gave anything back and `0` for fewer than two observations. Both
/// went out raw. The first is `2^63 - 1`, which `JSON.parse` rounds to `2^63`,
/// so `web/src/lib/frontier-analytics.js` refused the WHOLE frontier for one
/// honest unbounded row; the second is a refusal that read as a measured zero.
/// `n` is on the row, so "fewer than two observations" is decided here exactly.
///
/// What it cannot decide: the method's third absence -- no observation moved in
/// the position's favour -- is also stored as `0`, and the stored row keeps no
/// count that separates it from a payoff that truncated to zero. Telling those
/// apart needs the refusal persisted beside the value, which is a frontier
/// file-format change and is left to an owner decision rather than guessed at.
#[must_use]
pub(crate) fn payoff_on_wire(n: u64, payoff_bp: i64) -> String {
    if n < 2 {
        "null".to_owned()
    } else {
        measurable(payoff_bp)
    }
}

/// A refusal that names its cause, in the shape every other route here uses.
fn refuse(json: JsonHeaders, why: &str) -> (axum::http::StatusCode, JsonHeaders, String) {
    (
        axum::http::StatusCode::BAD_REQUEST,
        json,
        format!(
            r#"{{"rows":null,"count":0,"total_count":null,"admitted":0,"total_admitted":null,"page_complete":false,"complete":false,"refusal":{}}}"#,
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
            r#"{{"rows":null,"count":0,"total_count":null,"admitted":0,"total_admitted":null,"page_complete":false,"complete":false,"refusal":{}}}"#,
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
            r#"{{"identity":"{}","rows":null,"count":0,"total_count":null,"claimed_total":"{}","admitted":0,"total_admitted":null,"page_complete":false,"complete":false,"refusal":{}}}"#,
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
            r#"{{"identity":"{}","rows":null,"count":0,"total_count":{},"admitted":0,"total_admitted":null,"page_complete":false,"complete":false,"refusal":{}}}"#,
            crate::server::hex32(*identity),
            total,
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
    /// A whole request against a chosen root: the selector `frontier_json`
    /// parses before admission, then the blocking read it admits. Every
    /// fixture below drives both halves through this (D-0689).
    fn respond(
        root: Result<std::path::PathBuf, String>,
        query: &str,
    ) -> (axum::http::StatusCode, super::JsonHeaders, String) {
        match crate::detail::Selector::parse(root, query) {
            Ok(asked) => super::respond(asked),
            Err(why) => super::refuse(super::json_headers(), &why),
        }
    }

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

    #[test]
    fn a_request_without_an_identity_is_refused() {
        // Named for this process: gate 23 clause C. This path is never created
        // -- the request is refused before it is opened -- but a fixed `/tmp`
        // name is a shared name whether or not this test happens to write to it,
        // and the next edit that DOES write is the one that would collide.
        let dir =
            std::env::temp_dir().join(format!("brutex-api-frontier-none-{}", std::process::id()));
        let (status, _, body) = respond(Ok(dir), "");
        assert_eq!(status, axum::http::StatusCode::BAD_REQUEST);
        assert!(
            body.contains("64 hex characters"),
            "names the shape: {body}"
        );
        assert!(body.contains(r#""rows":null"#), "null, not empty: {body}");
    }

    fn verdict_row(identity: [u8; 32], rank: u16) -> cli::frontier::Row {
        cli::frontier::Row {
            identity,
            rank,
            direction: cli::frontier::Direction::Short,
            rules: cli::Rules::elite(400, 25),
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
        }
    }

    /// p5num-5 (D-2568): the payoff's two nameable absences are `null`, and
    /// every other value passes through unchanged. Exhaustive over the
    /// boundaries of both inputs, including values the method never produces,
    /// so the rule is pinned rather than the method's current range.
    #[test]
    fn an_unbounded_or_refused_payoff_is_null_on_the_wire() {
        for n in [0_u64, 1, 2, 3, u64::MAX] {
            for payoff in [i64::MIN, -1, 0, 1, 129, i64::MAX - 1, i64::MAX] {
                let expected = if n < 2 || payoff == i64::MAX {
                    "null".to_owned()
                } else {
                    payoff.to_string()
                };
                assert_eq!(
                    super::payoff_on_wire(n, payoff),
                    expected,
                    "n {n} payoff {payoff}"
                );
            }
        }
    }

    /// p5num-5 (D-2568): the rendered row, not just the helper. Before the fix
    /// the unbounded row carried a bare `9223372036854775807`, which the
    /// browser cannot hold exactly and which refused the whole frontier, and
    /// the one-observation row carried a measured-looking `0`.
    #[test]
    fn a_rendered_frontier_row_carries_a_null_payoff_where_the_method_has_none() {
        let identity = [0x5e_u8; 32];
        for (n, payoff, expected) in [
            (868_u64, 129_i64, r#""payoff_bp":129,"#),
            (868, i64::MAX, r#""payoff_bp":null,"#),
            (868, i64::MAX - 1, r#""payoff_bp":9223372036854775806,"#),
            (2, 0, r#""payoff_bp":0,"#),
            (1, 0, r#""payoff_bp":null,"#),
            (0, 0, r#""payoff_bp":null,"#),
        ] {
            let mut row = verdict_row(identity, 1);
            row.n = n;
            row.payoff_bp = payoff;
            let mut out = String::new();
            let admitted = super::write_rows(&mut out, &[row], &row.rules);
            assert!(admitted <= 1, "one row admits at most once: {out}");
            assert!(out.contains(expected), "n {n} payoff {payoff}: {out}");
            assert!(
                !out.contains("9223372036854775807"),
                "no bare i64::MAX reaches the wire: {out}"
            );
        }
    }

    fn ranked_row(identity: [u8; 32], rank: u16, top: usize) -> cli::frontier::Row {
        let mut row = verdict_row(identity, rank);
        row.rules.top = top;
        row
    }

    fn commit_frontier_fixture(dir: &std::path::Path, identity: [u8; 32], frontier_rows: u64) {
        commit_frontier_fixture_for(dir, identity, frontier_rows, "NIFTY");
    }

    /// [`commit_frontier_fixture`], with the ledger parent naming `underlying`.
    fn commit_frontier_fixture_for(
        dir: &std::path::Path,
        identity: [u8; 32],
        frontier_rows: u64,
        underlying: &str,
    ) {
        cli::result_set::Receipts::open(dir)
            .expect("a fresh detail-receipt store opens")
            .append_exact(cli::result_set::Receipt {
                identity,
                frontier_rows,
                trade_rows: 0,
                direction: cli::trades::Direction::Long,
                trade_policy: cli::result_set::TradePolicy::ChosenGridV1,
            })
            .expect("the child counts are receipted before the parent");
        cli::results::Results::open(dir)
            .expect("a fresh ledger opens")
            .append(&cli::results::Record {
                identity,
                finished_micros: 1,
                feed: cli::results::field("zerodha"),
                underlying: cli::results::field(underlying),
                timeframe: cli::results::field("15min"),
                from_year: 2026,
                from_month: 1,
                to_year: 2026,
                to_month: 1,
                months_asked: 1,
                months_found: 1,
                bars: 1_194,
                min_hits: 400,
                combinations: 1,
                depth: 1,
                halted: 0,
                trades: 868,
                pessimistic: -272_749,
                optimistic: -200_000,
                worst_trade: -3_035,
                max_drawdown: 329_165,
                winner_mae: 0,
                winner_mfe: 0,
                all_mae: 0,
                exit_rungs: [-1; 5],
                mask_words: [3, 0, 0, 0, 0, 0],
            })
            .expect("the parent commit marker appends last");
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
            // THE RUN'S OWN RULES, not `operator()`, and deliberately SHORT.
            // The positive mean would make a re-derived side answer LONG.
            .append_all(&[verdict_row(identity, 1)])
            .expect("one row appends");

        // The ledger row is the final marker, after a fixed-stride receipt that
        // proves the frontier has one row and the trade detail is legitimately
        // empty. The endpoint fixture therefore describes one complete set.
        commit_frontier_fixture(&dir, identity, 1);

        let query = format!("identity={}", "5a".repeat(32));
        let (status, _, body) = respond(Ok(dir.clone()), &query);
        assert_eq!(status, axum::http::StatusCode::OK);
        assert_body_identity(&body, &"5a".repeat(32));

        assert!(
            body.contains(r#""meets":{"#),
            "the verdict is on the row: {body}"
        );
        // THE DIRECTION IS ON THE WIRE, AND IT IS THE ROW'S OWN.
        //
        // This row's `mean_milli_paisa` is POSITIVE, so anything re-deriving the
        // side from the sign of the mean answers "long". The row was written
        // short. A surface that carries no direction at all -- which every one
        // of them did -- leaves an operator a win rate, a payoff and a drawdown
        // for a trade whose direction they cannot recover.
        assert!(
            body.contains(r#""direction":"short""#),
            "the side travels with the row and is not re-derived from the \
             mean's sign: {body}"
        );
        assert!(
            body.contains(r#""win_rate":false"#),
            "3 wins in 868 does not clear the floor: {body}"
        );
        assert!(
            body.contains(r#""reward_to_risk":false"#),
            "295 over 3,035 is 0.09x: {body}"
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
        // THE RULES ON THE WIRE ARE THE RUN'S, NOT THE ENVIRONMENT'S, AND THIS
        // IS THE ASSERTION THAT SEPARATES THEM.
        //
        // The row above was written under `Rules::elite(400, 25)`, whose win
        // rate floor is 8,000. `Rules::operator()` -- what this handler read
        // until the rules travelled on the row -- defaults to 5,000. A handler
        // that resolves them at request time answers 5,000 here and every
        // verdict beside it is computed against a policy no run applied.
        //
        // Asserted on the JSON text rather than through a parse, for the same
        // reason the mask-word assertion below is: the wire format is the
        // contract, and a number that reaches the browser as the wrong value is
        // wrong however cleanly it parses.
        assert!(
            body.contains(r#""rules":{"min_win_rate_bp":8000"#),
            "the run swept at elite's 8,000; a handler reading the environment \
             answers 5,000 and judges every row by it: {body}"
        );
        assert!(
            body.contains(r#""min_rr_bp":300"#),
            "and elite's payoff floor, not operator's 125: {body}"
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

    /// W2-cli5-4, D-1810: a row that clears the five rules the page used to
    /// show but not the average-payoff floor is served as NOT admitted, and
    /// the envelope names, from `cli`, exactly which rules `all` conjoins.
    ///
    /// Before D-1810 this row was served `"all":true`, with no `avg_payoff`
    /// member, no `fill_headroom_unchecked` flag and no `min_avg_rr_bp`
    /// threshold, while `cli::Rules::admits` refused it.
    #[test]
    fn a_row_failing_only_average_payoff_is_not_served_as_admitted() {
        let rules_at = |min_avg_rr_bp: i64| cli::Rules {
            max_mae_ppm: 1,
            min_rr_bp: 100,
            min_win_rate_bp: 5_000,
            min_trades: 10,
            min_assurance_bp: 0,
            min_weakest_bp: 0,
            min_ret_over_dd_bp: 100,
            require_protective_exits: true,
            min_fill_headroom_bp: 150,
            min_avg_rr_bp,
            top: 25,
        };
        // 60 wins averaging 200, 40 losses averaging 100: average payoff 2.00.
        let row_at = |identity: [u8; 32], min_avg_rr_bp: i64| cli::frontier::Row {
            rules: rules_at(min_avg_rr_bp),
            trades: 100,
            cell_wins: 60,
            gross_win: 12_000,
            gross_loss: -4_000,
            pessimistic: 8_000,
            min_win: 150,
            worst_trade: -100,
            max_drawdown: 500,
            ..verdict_row(identity, 1)
        };
        for (floor, admitted, byte) in [
            (300_i64, false, 0x6a_u8),
            (200, true, 0x6b),
            (100, true, 0x6c),
        ] {
            let dir = std::env::temp_dir().join(format!(
                "brutex-api-frontier-avg-payoff-{floor}-{}-{:?}",
                std::process::id(),
                std::thread::current().id()
            ));
            let _ = std::fs::remove_dir_all(&dir);
            let identity = [byte; 32];
            cli::frontier::Frontier::open(&dir)
                .expect("a fresh frontier opens")
                .append_all(&[row_at(identity, floor)])
                .expect("one row appends");
            commit_frontier_fixture(&dir, identity, 1);
            let hex = format!("{byte:02x}").repeat(32);
            let (status, _, body) = respond(Ok(dir.clone()), &format!("identity={hex}"));
            assert_eq!(status, axum::http::StatusCode::OK, "{body}");
            let meets = format!(
                r#""meets":{{"win_rate":true,"reward_to_risk":true,"return_over_drawdown":true,"trades":true,"assurance":true,"avg_payoff":{admitted},"all":{admitted},"stop_unchecked":true,"protective_exits_unchecked":true,"fill_headroom_unchecked":true}}}}"#
            );
            assert!(body.contains(&meets), "floor {floor}: {body}");
            assert!(
                body.contains(&format!(
                    r#""admitted":{},"total_admitted":{}"#,
                    u8::from(admitted),
                    u8::from(admitted)
                )),
                "floor {floor}: {body}"
            );
            assert!(
                body.contains(&format!(
                    r#""max_mae_ppm":1,"min_avg_rr_bp":{floor},"min_fill_headroom_bp":150,"top":25}}"#
                )),
                "every threshold `all` or an unchecked flag speaks for is echoed: {body}"
            );
            assert!(
                body.contains(r#""admission":{"checked":["win_rate","reward_to_risk","return_over_drawdown","trades","assurance","avg_payoff"],"unchecked":["stop","protective_exits","fill_headroom"]}"#),
                "the rule list is served from cli: {body}"
            );
            assert_eq!(
                row_at(identity, floor).verdict(&rules_at(floor)).admitted,
                admitted,
                "the wire says what cli's verdict says"
            );
            let _ = std::fs::remove_dir_all(&dir);
        }
    }

    /// **A stock run's ranked combinations say what their figures are made
    /// of; an index run's are the bytes they were.** AF-19.
    ///
    /// This route serves one run's RANKED combinations, the rows `cli top`
    /// prints under a stock's gross label and corporate-action sentence, and a
    /// RELIANCE run's were served with neither. The ledger parent the receipt
    /// read already confirms names the instrument, so a stock's payload now
    /// carries `equity_note` beside its identity, a committed empty frontier
    /// included, and an index run's gains no key.
    #[test]
    fn a_stock_runs_frontier_carries_the_equity_note_and_an_index_runs_is_unchanged() {
        let dir = std::env::temp_dir().join(format!(
            "brutex-api-frontier-equity-note-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let (index, stock) = ([0x96_u8; 32], [0x97_u8; 32]);
        let mut frontier = cli::frontier::Frontier::open(&dir).expect("a fresh frontier opens");
        frontier
            .append_all(&[verdict_row(index, 1)])
            .expect("the index run's row");
        commit_frontier_fixture(&dir, index, 1);
        frontier
            .append_all(&[verdict_row(stock, 1)])
            .expect("the stock run's row");
        commit_frontier_fixture_for(&dir, stock, 1, "RELIANCE");
        drop(frontier);

        let member = crate::detail::equity_note_member(Some("RELIANCE"));
        for part in ["GROSS OF EVERY CHARGE", "CORPORATE ACTIONS ARE UNCHECKED"] {
            assert!(member.contains(part), "premise, {part}: {member}");
        }
        let (status, _, index_body) =
            respond(Ok(dir.clone()), &format!("identity={}", "96".repeat(32)));
        assert_eq!(status, axum::http::StatusCode::OK, "{index_body}");
        assert!(!index_body.contains("equity_note"), "{index_body}");
        // CE-93 / D-2792: every recorded run's rows say they are in sample
        // and that their validation is not recorded, index or stock.
        let in_sample = format!(
            r#","in_sample":{}"#,
            crate::render::json_string(cli::LEDGER_IN_SAMPLE)
        );
        assert!(index_body.contains(&in_sample), "{index_body}");
        let (status, _, stock_body) =
            respond(Ok(dir.clone()), &format!("identity={}", "97".repeat(32)));
        assert_eq!(status, axum::http::StatusCode::OK, "{stock_body}");
        assert_body_identity(&stock_body, &"97".repeat(32));
        let _: serde_json::Value = serde_json::from_str(&stock_body).expect("valid JSON");
        assert!(
            stock_body.starts_with(&format!(
                r#"{{"identity":"{}"{member}{in_sample},"rows":[{{"rank":1,"#,
                "97".repeat(32)
            )),
            "beside the identity, before the ranked rows: {stock_body}"
        );
        assert_eq!(
            stock_body
                .replacen(&member, "", 1)
                .replace(&"97".repeat(32), &"96".repeat(32)),
            index_body,
            "the note is the only thing a stock's frontier adds"
        );

        // A stock's committed EMPTY frontier is still a statement about its run.
        let empty_dir = dir.join("empty");
        let empty = [0x98_u8; 32];
        cli::frontier::Frontier::open(&empty_dir).expect("empty frontier header");
        commit_frontier_fixture_for(&empty_dir, empty, 0, "RELIANCE");
        std::fs::remove_file(cli::frontier::Frontier::path(&empty_dir))
            .expect("delete empty child");
        let (status, _, body) = respond(Ok(empty_dir), &format!("identity={}", "98".repeat(32)));
        assert_eq!(status, axum::http::StatusCode::OK, "{body}");
        assert!(
            body.starts_with(&format!(
                r#"{{"identity":"{}"{member}{in_sample},"rows":[],"#,
                "98".repeat(32)
            )),
            "{body}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_committed_corrupt_frontier_exposes_no_valid_prefix() {
        let dir = std::env::temp_dir().join(format!(
            "brutex-api-frontier-corrupt-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let identity = [0x6b; 32];
        cli::frontier::Frontier::open(&dir)
            .expect("frontier")
            .append_all(&[
                verdict_row(identity, 1),
                verdict_row(identity, 2),
                verdict_row(identity, 3),
            ])
            .expect("three rows");
        commit_frontier_fixture(&dir, identity, 3);

        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .open(cli::frontier::Frontier::path(&dir))
            .expect("frontier bytes");
        std::io::Seek::seek(
            &mut file,
            std::io::SeekFrom::Start(16 + cli::frontier::STRIDE),
        )
        .expect("second row");
        std::io::Write::write_all(&mut file, &[0xff]).expect("corrupts seal");
        file.sync_all().expect("durable fixture");

        let query = format!("identity={}", "6b".repeat(32));
        let (status, _, body) = respond(Ok(dir.clone()), &query);
        assert_eq!(status, axum::http::StatusCode::BAD_REQUEST);
        assert!(body.contains(r#""rows":null"#), "no prefix: {body}");
        assert!(body.contains("No partial frontier"), "cause: {body}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A WHOLE CHILD FILE LOST AFTER COMMIT IS CORRUPTION, NOT ABSENCE.
    ///
    /// The same missing pathname is a legitimate empty set only when the
    /// sealed receipt says its cardinality was zero. This pair keeps the API
    /// from inferring either state from the filesystem alone.
    #[test]
    fn a_deleted_committed_frontier_refuses_unless_its_receipted_count_is_zero() {
        let dir = std::env::temp_dir().join(format!(
            "brutex-api-frontier-deleted-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);

        let populated = [0x6c; 32];
        cli::frontier::Frontier::open(&dir)
            .expect("frontier")
            .append_all(&[verdict_row(populated, 1)])
            .expect("one row");
        commit_frontier_fixture(&dir, populated, 1);
        std::fs::remove_file(cli::frontier::Frontier::path(&dir)).expect("delete child");

        let query = format!("identity={}", "6c".repeat(32));
        let (status, _, body) = respond(Ok(dir.clone()), &query);
        assert_eq!(status, axum::http::StatusCode::BAD_REQUEST);
        assert!(body.contains(r#""rows":null"#), "not empty: {body}");
        assert!(
            body.contains("committed for 1 frontier row"),
            "cause: {body}"
        );
        assert!(body.contains(&"6c".repeat(32)), "identity: {body}");

        let empty_dir = dir.join("empty");
        let empty = [0x6d; 32];
        cli::frontier::Frontier::open(&empty_dir).expect("empty frontier header");
        commit_frontier_fixture(&empty_dir, empty, 0);
        std::fs::remove_file(cli::frontier::Frontier::path(&empty_dir))
            .expect("delete empty child");

        let query = format!("identity={}", "6d".repeat(32));
        let (status, _, body) = respond(Ok(empty_dir), &query);
        assert_eq!(status, axum::http::StatusCode::OK);
        assert_body_identity(&body, &"6d".repeat(32));
        assert!(body.contains(r#""rows":[]"#), "empty: {body}");
        assert!(body.contains(r#""refusal":null"#), "committed: {body}");

        std::fs::write(cli::frontier::Frontier::path(&dir.join("empty")), b"bad")
            .expect("malformed child");
        let query = format!("identity={}", "6d".repeat(32));
        let (status, _, body) = respond(Ok(dir.join("empty")), &query);
        assert_eq!(status, axum::http::StatusCode::BAD_REQUEST);
        assert!(body.contains(r#""rows":null"#), "not absent: {body}");
        assert!(body.contains("still exists"), "cause: {body}");
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
        assert_body_identity(&body, &"b".repeat(64));
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

    #[test]
    fn one_bounded_receipt_snapshot_is_reconciled_without_a_parent_reopen() {
        let source = include_str!("frontierjson.rs");
        let respond = source
            .split_once("fn respond(")
            .and_then(|(_, after)| after.split_once("fn render_row(").map(|(body, _)| body))
            .expect("respond body");
        assert_eq!(
            respond.matches("crate::detail::committed_receipt(").count(),
            1,
            "one request owns one bounded canonical receipt snapshot"
        );
        let receipt = respond
            .find("crate::detail::committed_receipt(")
            .expect("bounded parent read");
        let child = respond
            .find("Frontier::open_read_bounded")
            .expect("bounded child open");
        let reconcile = respond
            .find("of_run_against_receipt(&identity, receipt)")
            .expect("exact snapshot reconciliation");
        assert!(
            receipt < child && child < reconcile,
            "receipt → child → rows"
        );
    }

    #[test]
    fn a_large_valid_frontier_is_explicitly_paged_and_never_called_complete() {
        let dir = std::env::temp_dir().join(format!(
            "brutex-api-frontier-page-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let identity = [0x91; 32];
        let rows: Vec<_> = (1..=300)
            .map(|rank| ranked_row(identity, rank, 300))
            .collect();
        cli::frontier::Frontier::open(&dir)
            .expect("frontier")
            .append_all(&rows)
            .expect("large valid frontier");
        commit_frontier_fixture(&dir, identity, 300);

        let query = format!("identity={}&limit=256", "91".repeat(32));
        let (status, _, body) = respond(Ok(dir.clone()), &query);
        assert_eq!(status, axum::http::StatusCode::PARTIAL_CONTENT);
        let _: serde_json::Value = serde_json::from_str(&body).expect("valid partial JSON");
        assert!(body.len() <= crate::detail::MAX_RESPONSE_BYTES);
        for fact in [
            r#""count":256"#,
            r#""total_count":300"#,
            r#""complete":false"#,
            r#""next_page":1"#,
            "partial page only",
        ] {
            assert!(body.contains(fact), "missing {fact}: {body}");
        }
        assert!(body.contains(r#""rank":1"#), "{body}");
        assert!(!body.contains(r#""rank":257"#), "page leaked: {body}");

        let query = format!("identity={}&page=1&limit=256", "91".repeat(32));
        let (status, _, body) = respond(Ok(dir.clone()), &query);
        assert_eq!(status, axum::http::StatusCode::PARTIAL_CONTENT);
        let _: serde_json::Value = serde_json::from_str(&body).expect("valid partial JSON");
        assert!(body.contains(r#""count":44"#), "{body}");
        assert!(body.contains(r#""rank":257"#), "{body}");
        assert!(body.contains(r#""next_page":null"#), "{body}");
        assert!(body.contains(r#""complete":false"#), "{body}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_valid_frontier_beyond_the_hard_row_ceiling_exposes_no_prefix() {
        let dir = std::env::temp_dir().join(format!(
            "brutex-api-frontier-row-ceiling-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let identity = [0x92; 32];
        let count = crate::detail::MAX_RESULT_ROWS.saturating_add(1);
        let rows: Vec<_> = (1..=count)
            .map(|rank| {
                ranked_row(
                    identity,
                    u16::try_from(rank).expect("bounded rank"),
                    usize::try_from(count).expect("bounded top"),
                )
            })
            .collect();
        cli::frontier::Frontier::open(&dir)
            .expect("frontier")
            .append_all(&rows)
            .expect("large valid frontier");
        commit_frontier_fixture(&dir, identity, count);

        let query = format!("identity={}", "92".repeat(32));
        let (status, _, body) = respond(Ok(dir.clone()), &query);
        assert_eq!(status, axum::http::StatusCode::PAYLOAD_TOO_LARGE);
        assert!(body.contains(r#""rows":null"#), "{body}");
        assert!(body.contains(r#""complete":false"#), "{body}");
        assert!(body.contains("one request verifies at most 4096"), "{body}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_unknown_identity_beside_a_valid_frontier_exposes_no_foreign_rows() {
        let dir = std::env::temp_dir().join(format!(
            "brutex-api-frontier-unknown-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let known = [0x93; 32];
        cli::frontier::Frontier::open(&dir)
            .expect("frontier")
            .append_all(&[verdict_row(known, 1)])
            .expect("known frontier");
        commit_frontier_fixture(&dir, known, 1);

        let query = format!("identity={}", "94".repeat(32));
        let (status, _, body) = respond(Ok(dir.clone()), &query);
        assert_eq!(status, axum::http::StatusCode::OK);
        assert_body_identity(&body, &"94".repeat(32));
        assert!(body.contains(r#""rows":[]"#), "{body}");
        assert!(body.contains("no committed result-set receipt"), "{body}");
        assert!(!body.contains(r#""rank":1"#), "foreign row leaked: {body}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The child half of the test below reads its store root from this.
    const ADMITTED_CHILD: &str = "BRUTEX_API_FRONTIER_ADMITTED_CHILD";

    /// A WELL-FORMED SELECTOR IS ADMITTED, AND THE READ IT NAMES ANSWERS IT.
    ///
    /// The fixtures above call `Selector::parse` and `respond` directly; no
    /// test sent a good selector through `frontier_json`, so its `Ok(asked)`
    /// arm and the admitted `move || respond(asked)` had never run. This one
    /// does, and requires the route's answer to be byte-identical to the two
    /// halves called directly, with the parsed `limit` still bounding the page
    /// the admitted read renders (D-0689).
    ///
    /// In a child process because `frontier_json` takes its root from
    /// `BRUTEX_STORE` and a test cannot set its own environment; see
    /// `crate::isolated`.
    #[tokio::test]
    async fn an_admitted_selector_is_answered_by_the_read_it_names() {
        if let Some(root) = std::env::var_os(ADMITTED_CHILD) {
            admitted_child(std::path::Path::new(&root)).await;
            return;
        }
        let root = crate::scratch::path("frontier-admitted-selector");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("the child's store root");
        let out = crate::isolated::rerun(
            "frontierjson::tests::an_admitted_selector_is_answered_by_the_read_it_names",
            &[
                (ADMITTED_CHILD, root.as_os_str()),
                ("BRUTEX_STORE", root.as_os_str()),
            ],
        );
        assert!(out.contains("FRONTIER-ADMITTED 206 rank1"), "{out}");
        let _ = std::fs::remove_dir_all(&root);
    }

    async fn admitted_child(root: &std::path::Path) {
        assert_eq!(
            crate::server::store_dir(),
            Ok(root.to_path_buf()),
            "the route reads the root this child was given"
        );
        let identity = [0x95; 32];
        cli::frontier::Frontier::open(root)
            .expect("frontier")
            .append_all(&[verdict_row(identity, 1), verdict_row(identity, 2)])
            .expect("two ranked rows of one run");
        commit_frontier_fixture(root, identity, 2);

        let query = format!("identity={}&limit=1", "95".repeat(32));
        let uri: axum::http::Uri = format!("/frontier.json?{query}").parse().expect("uri");
        let routed = super::frontier_json(uri).await;
        let direct = respond(Ok(root.to_path_buf()), &query);
        assert_eq!(
            routed.0,
            axum::http::StatusCode::PARTIAL_CONTENT,
            "one page of two rows is a partial answer: {}",
            routed.2
        );
        assert_eq!(routed, direct, "the route serves what the read answers");
        assert_body_identity(&routed.2, &"95".repeat(32));
        assert!(routed.2.contains(r#""rank":1"#), "page 0 row: {}", routed.2);
        assert!(
            !routed.2.contains(r#""rank":2"#),
            "`limit=1` was parsed before admission and still bounds the page: {}",
            routed.2
        );
        println!("FRONTIER-ADMITTED {} rank1", routed.0.as_u16());
    }
}

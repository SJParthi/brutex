//! Bounded top report reader. Cold selection indexes O(history); refresh folds
//! only appended ledger records. Selected frontier verification remains O(rows).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::detail::{Cached, MAX_RESULT_ROWS, MAX_SCAN_BYTES};

type Response = (
    axum::http::StatusCode,
    [(axum::http::header::HeaderName, &'static str); 1],
    String,
);

struct Selection {
    ledger: cli::results::Results,
    processed: u64,
    best: Option<cli::results::Record>,
    pairs: BTreeMap<(String, String), cli::results::Record>,
}

impl Selection {
    fn open(root: &Path) -> Result<Self, String> {
        let mut selected = Self {
            ledger: cli::results::Results::open_read_bounded(root, MAX_SCAN_BYTES)?,
            processed: 0,
            best: None,
            pairs: BTreeMap::new(),
        };
        selected.extend()?;
        Ok(selected)
    }

    fn refresh(&mut self) -> Result<(), String> {
        self.ledger.refresh()?;
        self.extend()
    }

    fn extend(&mut self) -> Result<(), String> {
        let end = self.ledger.len()?;
        for at in self.processed..end {
            let row = self.ledger.read(at)?;
            if !row.has_complete_trade_total() {
                continue;
            }
            if self
                .best
                .is_none_or(|old| row.pessimistic >= old.pessimistic)
            {
                self.best = Some(row);
            }
            let pair = (
                cli::results::read_field(&row.feed),
                cli::results::read_field(&row.underlying),
            );
            self.pairs
                .entry(pair)
                .and_modify(|old| {
                    if row.pessimistic >= old.pessimistic {
                        *old = row;
                    }
                })
                .or_insert(row);
        }
        self.processed = end;
        Ok(())
    }

    fn get(&self, pair: Option<&(String, String)>) -> Option<cli::results::Record> {
        pair.map_or(self.best, |pair| self.pairs.get(pair).copied())
    }
}

static SELECTION: Cached<Selection> = Cached::new();

/// Render the terminal's canonical top report without occupying an async worker.
pub async fn top_json(uri: axum::http::Uri) -> Response {
    let query = uri.query().unwrap_or_default();
    let pair = match parse(query) {
        Ok(pair) => pair,
        Err(why) => return refusal(axum::http::StatusCode::BAD_REQUEST, &why),
    };
    let root = crate::server::store_dir();
    match crate::detail::run(move || respond(root, pair.as_ref())).await {
        Ok(response) => response,
        Err(crate::detail::RunError::Saturated) => refusal(
            axum::http::StatusCode::TOO_MANY_REQUESTS,
            "top report capacity is full; no blocking task was queued; retry after another detail read finishes",
        ),
        Err(crate::detail::RunError::Join(why)) => {
            refusal(axum::http::StatusCode::SERVICE_UNAVAILABLE, &why)
        }
    }
}

fn parse(query: &str) -> Result<Option<(String, String)>, String> {
    crate::detail::query_is_bounded(query)?;
    let mut seen = std::collections::BTreeSet::new();
    for pair in query.split('&').filter(|part| !part.is_empty()) {
        let (key, _) = pair
            .split_once('=')
            .ok_or("top query requires key=value fields")?;
        if !matches!(key, "feed" | "underlying") || !seen.insert(key) {
            return Err(format!("unknown or repeated top query field {key:?}"));
        }
    }
    let feed = crate::server::param(query, "feed");
    let underlying = crate::server::param(query, "underlying");
    match (feed.is_empty(), underlying.is_empty()) {
        (true, true) if seen.is_empty() => Ok(None),
        (false, false) => Ok(Some((feed, underlying))),
        _ => Err(
            "`feed` and `underlying` filter together: give both nonempty values or neither"
                .to_owned(),
        ),
    }
}

fn respond(root: Result<PathBuf, String>, pair: Option<&(String, String)>) -> Response {
    match root.and_then(|root| report(&root, pair)) {
        Ok(report) => {
            let body = format!(
                r#"{{"report":{},"refusal":null}}"#,
                crate::render::json_string(&report)
            );
            if body.len() > crate::detail::MAX_RESPONSE_BYTES {
                return refusal(
                    axum::http::StatusCode::SERVICE_UNAVAILABLE,
                    "top report exceeds the bounded JSON response size; no prefix was exposed",
                );
            }
            (axum::http::StatusCode::OK, headers(), body)
        }
        Err(why) => refusal(axum::http::StatusCode::SERVICE_UNAVAILABLE, &why),
    }
}

fn report(root: &Path, pair: Option<&(String, String)>) -> Result<String, String> {
    crate::detail::preflight([
        (
            "results ledger",
            cli::results::Results::path(root).as_path(),
        ),
        (
            "detail receipts",
            cli::result_set::Receipts::path(root).as_path(),
        ),
        ("frontier", cli::frontier::Frontier::path(root).as_path()),
    ])?;
    let selected = SELECTION.with_verified(
        root,
        || Selection::open(root),
        Selection::refresh,
        |selection| selection.get(pair),
    );
    let row = match selected {
        Ok(Some(row)) => row,
        Ok(None) => return Ok("  NO COMPLETE RUN matches. Every matching row halted on a budget or traded nothing, or nothing has been recorded yet — `cli results` lists what is there.\n".to_owned()),
        Err(why) if why.contains("nothing was created") || why.contains("nothing was written") => return Ok("  NO RUN HAS BEEN RECORDED YET. The ledger does not exist or holds nothing — sweep something and it appears here.\n".to_owned()),
        Err(why) => return Err(why),
    };
    let receipt = crate::detail::committed_receipt(root, &row.identity)?;
    if receipt.is_some_and(|receipt| receipt.frontier_rows > MAX_RESULT_ROWS) {
        return Err("top result exceeds the 4096-row detail bound; no prefix was read".to_owned());
    }
    let Some(receipt) = receipt else {
        return Ok(cli::render_top_record(
            &row,
            &[],
            None,
            "No committed result-set receipt exists; no orphan frontier rows are displayed.",
        ));
    };
    let (found, partial) = crate::detail::FRONTIER.with_verified(
        root,
        || cli::frontier::Frontier::open_read_bounded(root, MAX_SCAN_BYTES),
        cli::frontier::Frontier::refresh,
        |frontier| frontier.of_run_against_receipt(&row.identity, Some(receipt)),
    )??;
    if let Some(why) = partial {
        return Err(why);
    }
    Ok(cli::render_top_record(&row, &found, None, ""))
}

fn headers() -> [(axum::http::header::HeaderName, &'static str); 1] {
    [(
        axum::http::header::CONTENT_TYPE,
        "application/json; charset=utf-8",
    )]
}

fn refusal(status: axum::http::StatusCode, why: &str) -> Response {
    (
        status,
        headers(),
        format!(
            r#"{{"report":null,"refusal":{}}}"#,
            crate::render::json_string(why)
        ),
    )
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "tests fail through assertions"
)]
mod tests {
    use super::{Selection, parse, report, respond};

    fn row(id: u8, symbol: &str, profit: i64) -> cli::results::Record {
        cli::results::Record {
            identity: [id; 32],
            finished_micros: i64::from(id),
            feed: cli::results::field("zerodha"),
            underlying: cli::results::field(symbol),
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
            pessimistic: profit,
            optimistic: profit,
            worst_trade: 0,
            max_drawdown: 0,
            winner_mae: 0,
            winner_mfe: 0,
            all_mae: 0,
            exit_rungs: [-1; 5],
            mask_words: [0; 6],
        }
    }

    #[test]
    fn top_selection_refreshes_incrementally_keeps_ties_and_excludes_halted_rows() {
        let dir = crate::scratch::path("top-incremental-selection");
        let _ = std::fs::remove_dir_all(&dir);
        let mut writer = cli::results::Results::open(&dir).expect("ledger");
        writer.append(&row(1, "NIFTY", 100)).expect("first");
        writer
            .append(&row(2, "BANKNIFTY", 50))
            .expect("other instrument");
        let mut cache = Selection::open(&dir).expect("bounded selection");
        assert_eq!(cache.processed, 2);
        assert_eq!(cache.get(None).expect("global best").identity, [1; 32]);
        let pair = ("zerodha".to_owned(), "BANKNIFTY".to_owned());
        assert_eq!(cache.get(Some(&pair)).expect("pair best").identity, [2; 32]);
        let mut halted = row(3, "NIFTY", 1_000);
        halted.halted = 1;
        writer.append(&halted).expect("partial run");
        writer
            .append(&row(4, "NIFTY", 100))
            .expect("newer tied result");
        cache.refresh().expect("incremental refresh");
        assert_eq!(cache.processed, 4);
        assert_eq!(cache.get(None).expect("newer tie wins").identity, [4; 32]);
        cache.refresh().expect("unchanged refresh");
        assert_eq!(cache.processed, 4);
        std::fs::OpenOptions::new()
            .write(true)
            .open(cli::results::Results::path(&dir))
            .expect("fixture")
            .set_len(16)
            .expect("truncate");
        assert!(
            cache.refresh().is_err(),
            "a damaged generation cannot reuse its old winners"
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn top_selection_never_ranks_zero_trade_totals_over_losses() {
        struct Owned(std::path::PathBuf);
        impl Drop for Owned {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let dir = crate::scratch::path("top-zero-trade-totals");
        std::fs::create_dir(&dir).expect("exclusively claim generated ledger root");
        let _owned = Owned(dir.clone());
        let mut ledger = cli::results::Results::open(&dir).expect("owned ledger");
        let unpriced = cli::results::Record {
            trades: 0,
            ..row(20, "NIFTY", 0)
        };
        ledger
            .append(&unpriced)
            .expect("recorded sweep without trades");
        let mut cache = Selection::open(&dir).expect("cold selection");
        let pair = ("zerodha".to_owned(), "NIFTY".to_owned());
        assert_eq!(cache.processed, 1);
        assert_eq!(cache.get(None), None);
        assert_eq!(cache.get(Some(&pair)), None);
        let empty_report = report(&dir, None).expect("unpriced report");
        assert_eq!(empty_report, cli::top_at(&dir, None, None));
        assert!(empty_report.contains("NO COMPLETE RUN"), "{empty_report}");
        assert!(empty_report.contains("traded nothing"), "{empty_report}");

        let loss = row(21, "NIFTY", -100);
        ledger.append(&loss).expect("completed losing trade");
        cache.refresh().expect("incremental loss");
        assert_eq!(cache.get(None), Some(loss));
        assert_eq!(cache.get(Some(&pair)), Some(loss));
        ledger
            .append(&cli::results::Record {
                identity: [22; 32],
                ..unpriced
            })
            .expect("later unpriced zero");
        ledger
            .append(&cli::results::Record {
                halted: 1,
                ..row(23, "NIFTY", 1_000)
            })
            .expect("later halted total");
        cache.refresh().expect("ineligible rows are processed");
        assert_eq!(cache.processed, 4);
        assert_eq!(cache.get(None), Some(loss));
        assert_eq!(cache.get(Some(&pair)), Some(loss));
        let newer = row(24, "NIFTY", -100);
        ledger.append(&newer).expect("later equal trade total");
        let path = cli::results::Results::path(&dir);
        let before = std::fs::read(&path).expect("original bytes");
        cache.refresh().expect("newest tie");
        assert_eq!(cache.get(None), Some(newer));
        assert_eq!(cache.get(Some(&pair)), Some(newer));
        assert_eq!(cache.processed, 5);
        assert_eq!(std::fs::read(path).expect("unchanged bytes"), before);
    }

    #[test]
    fn top_queries_and_unreadable_files_refuse_without_creating_a_store() {
        for query in [
            "feed=zerodha",
            "feed=&underlying=NIFTY",
            "feed=x&feed=y&underlying=z",
            "unknown=x",
            "feed",
        ] {
            assert!(parse(query).is_err(), "{query}");
        }
        assert!(parse(&"x".repeat(crate::detail::MAX_QUERY_BYTES + 1)).is_err());
        assert_eq!(parse(""), Ok(None));
        assert_eq!(
            parse("feed=zerodha&underlying=NIFTY"),
            Ok(Some(("zerodha".to_owned(), "NIFTY".to_owned())))
        );
        let dir = crate::scratch::path("top-absent-store");
        let _ = std::fs::remove_dir_all(&dir);
        let (status, _, body) = respond(Ok(dir.clone()), None);
        assert_eq!(status, axum::http::StatusCode::OK);
        assert!(body.contains("NO RUN HAS BEEN RECORDED YET"));
        assert!(!dir.exists());
        let (status, _, body) = respond(Err("unreadable root".to_owned()), None);
        assert_eq!(status, axum::http::StatusCode::SERVICE_UNAVAILABLE);
        assert!(body.contains(r#""report":null"#));
        assert!(body.contains("unreadable root"));
    }

    #[test]
    fn cached_http_report_uses_the_exact_cli_renderer_after_parent_commit() {
        let dir = crate::scratch::path("top-canonical-renderer");
        let _ = std::fs::remove_dir_all(&dir);
        let record = row(0x6a, "NIFTY", 0);
        cli::frontier::Frontier::open(&dir).expect("empty frontier header");
        cli::trades::Trades::open(&dir)
            .expect("chosen trade detail")
            .append_all(&[cli::trades::Row {
                identity: record.identity,
                seq: 0,
                direction: cli::trades::Direction::Long,
                signal_bar: 0,
                entry_bar: 1,
                exit_bar: 2,
                best: 0,
                worst: 0,
                entry_micros: 1_767_240_000_000_000,
                exit_micros: 1_767_240_060_000_000,
                adverse_ppm: 0,
                adverse_paisa: 0,
                favourable_ppm: 0,
                favourable_paisa: 0,
            }])
            .expect("one generated break-even trade");
        cli::result_set::Receipts::open(&dir)
            .expect("receipts")
            .append_exact(cli::result_set::Receipt {
                identity: record.identity,
                frontier_rows: 0,
                trade_rows: 1,
                direction: cli::trades::Direction::Long,
                trade_policy: cli::result_set::TradePolicy::ChosenGridV1,
            })
            .expect("receipt");
        cli::results::Results::open(&dir)
            .expect("parent")
            .append(&record)
            .expect("commit parent last");
        let canonical = cli::top_at(&dir, None, None);
        assert!(canonical.contains(&record.identity_hex()), "{canonical}");
        assert_eq!(report(&dir, None), Ok(canonical.clone()));
        assert_eq!(report(&dir, None), Ok(canonical));
        let _ = std::fs::remove_dir_all(dir);
    }
}

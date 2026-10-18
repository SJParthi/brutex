#![cfg(test)]
//! `/spot.json` over generated records (D-2807, D-3328). No row here is
//! market data.
#![allow(clippy::expect_used, clippy::indexing_slicing)]

use super::spot::{
    AsOf, MAX_SPOT_TIMES, Spot, in_month, ist_day, month_of, not_admitted, on_grid, respond,
    spot_json, underlying_segment, verdict,
};
use super::*;
use axum::http::StatusCode;
use serde_json::{Value, json};
use std::fs;
use store::format::{Bar, OI_NULL};
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

/// 2025-05-02 09:15 IST, in UTC seconds.
fn open_second() -> i64 {
    i64::from(
        Day::new(2025, 5, 2)
            .expect("generated day")
            .days_from_epoch(),
    ) * 86_400
        + 13_500
}

struct Fixture {
    root: PathBuf,
    site: Loaded,
    index_path: PathBuf,
}

fn write(root: &Path, segment: &str, symbol: &str, bars: &[(i64, i64)]) -> PathBuf {
    write_rung(root, segment, symbol, Timeframe::MINUTE_1, bars)
}

fn write_rung(
    root: &Path,
    segment: &str,
    symbol: &str,
    timeframe: Timeframe,
    bars: &[(i64, i64)],
) -> PathBuf {
    let path = StorePath::new(PathParts {
        vendor: Vendor::Dhan,
        exchange: "NSE",
        segment,
        symbol,
        contract: None,
        timeframe,
        month: YearMonth::new(2025, 5).expect("month"),
        file: FileKind::Bars,
    })
    .expect("generated path");
    let hash = brutex_core::universe::fnv1a(symbol).to_le_bytes();
    let id = u32::from_le_bytes(hash[..4].try_into().expect("low32"));
    let mut file =
        store::file::BarFile::open_or_create(root, path, id).expect("owned generated writer");
    let rows: Vec<Bar> = bars
        .iter()
        .map(|&(secs, close)| Bar {
            ts_micros: secs * 1_000_000,
            open: close,
            high: close + 5,
            low: close - 5,
            close,
            volume: 0,
            open_interest: OI_NULL,
        })
        .collect();
    file.append(&rows).expect("generated records");
    path.to_path_buf(root)
}

/// One answered instant, as the wire carries it.
fn hit(t: i64, close: i64, stamp: i64) -> Value {
    json!({"t": t, "close": close, "spot_t": stamp, "spot_age_s": t - stamp, "why": null})
}

/// One refused instant.
fn miss(t: i64, why: &str) -> Value {
    json!({"t": t, "close": null, "spot_t": null, "spot_age_s": null, "why": why})
}

impl Fixture {
    fn new(name: &str) -> Self {
        let root = crate::scratch::path(name);
        let masters = root.join("masters");
        fs::create_dir_all(&masters).expect("offline empty master root");
        let site = Loaded::new(Site::load(&masters, &root));
        let t0 = open_second();
        // A GAP AT +120 s, so "no bar at t" and "the bar before t" differ, and
        // a bar the DAY BEFORE, so "the bar before t" can be yesterday's.
        let index_path = write(
            &root,
            "INDEX",
            "NIFTY",
            &[
                (t0 - 86_400 + 22_440, 2_398_000),
                (t0, 2_400_000),
                (t0 + 60, 2_401_050),
                (t0 + 180, 2_399_900),
            ],
        );
        write(&root, "CASH", "RELIANCE", &[(t0, 141_025)]);
        Self {
            root,
            site,
            index_path,
        }
    }

    /// `/spot.json?{query}` through the handler's body: the content of every
    /// answer is checked here, and the handler's admission once, below.
    fn get(&self, query: &str) -> (StatusCode, Value) {
        let (status, headers, body) = respond(&self.root, query);
        assert_eq!(
            headers[axum::http::header::CONTENT_TYPE],
            "application/json; charset=utf-8"
        );
        (status, serde_json::from_str(&body).expect("spot JSON"))
    }

    fn at(&self, symbol: &str, times: &[i64]) -> (StatusCode, Value) {
        self.at_with(symbol, times, "")
    }

    fn at_with(&self, symbol: &str, times: &[i64], extra: &str) -> (StatusCode, Value) {
        let t: Vec<String> = times.iter().map(i64::to_string).collect();
        self.get(&format!(
            "feed=dhan&exchange=NSE&symbol={symbol}&timeframe=1min&month=2025-05&t={}{extra}",
            t.join(",")
        ))
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn an_index_underlying_reads_index_and_every_other_underlying_reads_cash() {
    for name in brutex_core::universe::FNO_INDEX_UNDERLYINGS {
        assert_eq!(underlying_segment(name), "INDEX", "{name}");
    }
    // DB-G54: NIFTYIT and NIFTYFPI are indices, not shares (D-3332).
    for name in ["NIFTYIT", "NIFTYFPI", "NIFTYCPSE"] {
        assert_eq!(underlying_segment(name), "INDEX", "{name}");
    }
    for name in ["RELIANCE", "BAJAJ-AUTO", "NIFTYBEES"] {
        assert_eq!(underlying_segment(name), "CASH", "{name}");
    }
}

#[test]
fn an_absence_is_stated_only_when_the_records_either_side_bracket_it() {
    let t = 1_000;
    assert_eq!(verdict(None, None, t), Spot::NoBar);
    assert_eq!(verdict(Some(999), Some(1_001), t), Spot::NoBar);
    assert_eq!(verdict(None, Some(1_001), t), Spot::NoBar);
    assert_eq!(verdict(Some(1), None, t), Spot::NoBar);
    // A ZERO STAMP IS THE ZERO-FILLED EXTENT, never a real bar.
    assert_eq!(verdict(Some(0), Some(1_001), t), Spot::OrderBroken);
    assert_eq!(verdict(Some(1_000), None, t), Spot::OrderBroken);
    assert_eq!(verdict(Some(1_001), None, t), Spot::OrderBroken);
    assert_eq!(verdict(Some(999), Some(1_000), t), Spot::OrderBroken);
    assert_eq!(verdict(None, Some(0), t), Spot::OrderBroken);
    assert_eq!(Spot::OrderBroken.why(), Some("spot_order_broken"));
    let at = Spot::At {
        close: 7,
        ts_micros: 5,
    };
    assert_eq!(at.why(), None);
    assert_eq!(at.close(), Some(7));
    assert_eq!(at.stamp_micros(), Some(5));
    assert_eq!(at.read(), Some((7, 5)));
    assert_eq!(Spot::NoBar.close(), None);
    assert_eq!(Spot::NoBar.stamp_micros(), None);
    assert_eq!(Spot::NoBar.read(), None);
    assert_eq!(Spot::NoBarInSession.why(), Some("no_spot_in_session"));
    assert_eq!(Spot::OutsideMonth.why(), Some("t_outside_month"));
    assert_eq!(Spot::OffGrid.why(), Some("t_off_grid"));
}

/// The month and session arithmetic the guards stand on: a month's span is
/// `[00:00 IST on the 1st, 00:00 IST on the next 1st)`, and an IST day turns
/// at 18:30 UTC.
#[test]
fn a_months_span_and_a_session_day_are_read_in_ist() {
    let may = YearMonth::new(2025, 5).expect("month");
    let first = i64::from(Day::new(2025, 5, 1).expect("day").days_from_epoch()) * 86_400 - 19_800;
    let june = i64::from(Day::new(2025, 6, 1).expect("day").days_from_epoch()) * 86_400 - 19_800;
    assert!(in_month(may, first * 1_000_000));
    assert!(!in_month(may, (first - 1) * 1_000_000));
    assert!(in_month(may, (june - 1) * 1_000_000));
    assert!(!in_month(may, june * 1_000_000));
    assert_eq!(month_of(first * 1_000_000), Some(may));
    assert_eq!(
        month_of((first - 1) * 1_000_000),
        YearMonth::new(2025, 4).ok()
    );
    assert_eq!(month_of(i64::MIN), None);
    assert_eq!(
        ist_day(first * 1_000_000),
        ist_day((first + 86_399) * 1_000_000)
    );
    assert_eq!(
        ist_day(first * 1_000_000) + 1,
        ist_day((first + 86_400) * 1_000_000)
    );
    assert_eq!(AsOf::parse("asof=prev"), Ok(AsOf::Prev));
    assert_eq!(AsOf::parse("asof=exact"), Ok(AsOf::Exact));
    assert_eq!(AsOf::parse(""), Ok(AsOf::Exact));
    assert!(
        AsOf::parse("asof=nearest")
            .expect_err("refused")
            .contains("prev")
    );
    assert_eq!(AsOf::Prev.as_str(), "prev");
    assert_eq!(AsOf::Exact.as_str(), "exact");
}

#[test]
fn the_close_is_the_bar_stamped_exactly_at_t_and_never_a_nearest_one() {
    let fixture = Fixture::new("spot-exact");
    let t0 = open_second();
    let (status, body) = fixture.at(
        "NIFTY",
        &[t0, t0 + 60, t0 + 90, t0 + 120, t0 + 180, t0 - 60, t0 + 240],
    );
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["segment"], "INDEX");
    assert_eq!(body["symbol"], "NIFTY");
    assert_eq!(body["exchange"], "NSE");
    assert_eq!(body["timeframe"], "1min");
    assert_eq!(body["month"], "2025-05");
    assert_eq!(body["asof"], "exact");
    assert!(body["faults"].is_null());
    assert_eq!(
        body["spots"],
        json!([
            hit(t0, 2_400_000, t0),
            hit(t0 + 60, 2_401_050, t0 + 60),
            // BETWEEN TWO BARS and AT THE GAP: neither the earlier close nor
            // the later one, a named absence.
            miss(t0 + 90, "no_spot_bar"),
            miss(t0 + 120, "no_spot_bar"),
            hit(t0 + 180, 2_399_900, t0 + 180),
            miss(t0 - 60, "no_spot_bar"),
            miss(t0 + 240, "no_spot_bar"),
        ])
    );
}

/// **DB-068, D-3328: `asof=prev` IS THE SESSION'S LAST BAR AT OR BEFORE `t`,
/// WITH ITS STAMP AND AGE** -- never a later bar, never yesterday's, never
/// for an instant off the rung's grid (whose bar before may close after it),
/// and an exact hit is age 0.
#[test]
fn as_of_prev_takes_the_sessions_last_bar_at_or_before_t_and_says_how_old() {
    let fixture = Fixture::new("spot-asof");
    let t0 = open_second();
    let (status, body) = fixture.at_with(
        "NIFTY",
        &[t0, t0 + 120, t0 + 240, t0 - 60, t0 + 90, t0 + 179],
        "&asof=prev",
    );
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["asof"], "prev");
    assert_eq!(
        body["spots"],
        json!([
            hit(t0, 2_400_000, t0),
            // THE GAP AT +120: the bar of +60, a minute old, never +180's.
            hit(t0 + 120, 2_401_050, t0 + 60),
            hit(t0 + 240, 2_399_900, t0 + 180),
            // Before the session's first bar the bar before is YESTERDAY's:
            // refused, never carried into today.
            miss(t0 - 60, "no_spot_in_session"),
            // OFF THE MINUTE GRID: the bar before +90 is the +60 bar, which
            // closes at +120, after the instant asked.
            miss(t0 + 90, "t_off_grid"),
            miss(t0 + 179, "t_off_grid"),
        ])
    );
    assert_eq!(body["spots"][1]["spot_age_s"], 60);
    // A DAILY RUNG: the session's one bar closes at the session's close, so
    // an instant later that day is not answered with it under `prev`.
    let midnight = t0 - 33_300;
    write_rung(
        &fixture.root,
        "INDEX",
        "NIFTY",
        Timeframe::DAY_1,
        &[(midnight, 2_401_234)],
    );
    let day = |t: i64| {
        fixture.get(&format!(
            "feed=dhan&exchange=NSE&symbol=NIFTY&timeframe=1day&month=2025-05&asof=prev&t={t}"
        ))
    };
    assert_eq!(
        day(midnight).1["spots"],
        json!([hit(midnight, 2_401_234, midnight)])
    );
    assert_eq!(day(t0).1["spots"], json!([miss(t0, "no_spot_bar")]));
    // Exact mode keeps D-2807's word for the same off-grid instant.
    let (_, body) = fixture.at("NIFTY", &[t0 + 90]);
    assert_eq!(body["spots"], json!([miss(t0 + 90, "no_spot_bar")]));
    // A misspelt opt-in is refused, never read as the default.
    let (status, body) = fixture.at_with("NIFTY", &[t0], "&asof=previous");
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert!(body["error"].as_str().expect("named").contains("asof"));
}

/// The grid an instant is checked against is the store's own: intraday rungs
/// on the 09:15 IST-anchored grid of their width, a daily rung any whole
/// second.
#[test]
fn the_grid_is_the_stores_own_admission_grid() {
    let t0 = open_second() * 1_000_000;
    assert!(on_grid(60, t0));
    assert!(on_grid(60, t0 + 60_000_000));
    assert!(!on_grid(60, t0 + 30_000_000));
    assert!(on_grid(300, t0 + 300_000_000));
    assert!(!on_grid(300, t0 + 60_000_000));
    // 09:15 + 375 minutes is 15:30, on the hour rung's 09:15-anchored grid.
    assert!(on_grid(3_600, t0 + 3_600_000_000));
    assert!(!on_grid(3_600, t0 + 2_700_000_000));
    assert!(on_grid(1, t0 + 7_000_000));
    assert!(!on_grid(1, t0 + 7_500_000));
    assert!(on_grid(86_400, t0 + 1_000_000));
    assert!(!on_grid(86_400, t0 + 1));
}

/// **DB-039, D-3328: AN INSTANT OUTSIDE THE ASKED MONTH IS A WRONG QUESTION**,
/// answered `t_outside_month` whether or not a file is held, and never the
/// held-absence `no_spot_bar` it was answered as before.
#[test]
fn an_instant_outside_the_asked_month_is_refused_by_name() {
    let fixture = Fixture::new("spot-outside");
    let t0 = open_second();
    let april = t0 - 2 * 86_400;
    let june = t0 + 31 * 86_400;
    for asof in ["", "&asof=prev"] {
        let (status, body) = fixture.at_with("NIFTY", &[april, t0, june], asof);
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(
            body["spots"],
            json!([
                miss(april, "t_outside_month"),
                hit(t0, 2_400_000, t0),
                miss(june, "t_outside_month"),
            ]),
            "{asof}"
        );
    }
    // NO FILE: inside the month that is the absence; outside it the question.
    let (_, body) = fixture.at("TCS", &[april, t0]);
    assert_eq!(
        body["spots"],
        json!([miss(april, "t_outside_month"), miss(t0, "no_spot_file")])
    );
}

#[test]
fn a_stock_option_reads_its_cash_series_and_an_unheld_one_says_so_by_name() {
    let fixture = Fixture::new("spot-cash");
    let t0 = open_second();
    let (status, body) = fixture.at("RELIANCE", &[t0]);
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["segment"], "CASH");
    assert_eq!(body["spots"], json!([hit(t0, 141_025, t0)]));
    // FINNIFTY is an index underlying with no file here; TCS a share with none.
    for (symbol, segment) in [("FINNIFTY", "INDEX"), ("TCS", "CASH")] {
        let (status, body) = fixture.at(symbol, &[t0, t0 + 60]);
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["segment"], segment);
        assert!(body["faults"].is_null());
        assert_eq!(
            body["spots"],
            json!([miss(t0, "no_spot_file"), miss(t0 + 60, "no_spot_file")])
        );
    }
}

/// **DB-G25, DB-G54: THE UNDERLYING IS READ WHERE THE ALIAS TABLE SAYS.**
/// FINNIFTY's spot is filed as `INDEX/NIFTYFINSERVICE`; NIFTYIT is an index,
/// not a share; TATAMOTORS's months are filed under `CASH/TMPV` (D-3332,
/// D-3333). Before, the first answered `no_spot_file` against
/// `INDEX/FINNIFTY` and the other two against `CASH/<symbol>`.
#[test]
fn an_underlying_is_read_under_the_key_the_alias_table_names() {
    let fixture = Fixture::new("spot-alias");
    let t0 = open_second();
    write(
        &fixture.root,
        "INDEX",
        "NIFTYFINSERVICE",
        &[(t0, 2_312_345)],
    );
    write(&fixture.root, "INDEX", "NIFTYIT", &[(t0, 3_456_700)]);
    write(&fixture.root, "CASH", "TMPV", &[(t0, 65_430)]);
    for (symbol, segment, key, close) in [
        ("FINNIFTY", "INDEX", "NIFTYFINSERVICE", 2_312_345),
        ("NIFTYFINSERVICE", "INDEX", "NIFTYFINSERVICE", 2_312_345),
        ("NIFTYIT", "INDEX", "NIFTYIT", 3_456_700),
        ("TATAMOTORS", "CASH", "TMPV", 65_430),
        ("NIFTY", "INDEX", "NIFTY", 2_400_000),
        ("RELIANCE", "CASH", "RELIANCE", 141_025),
    ] {
        let (status, body) = fixture.at(symbol, &[t0]);
        assert_eq!(status, StatusCode::OK, "{symbol}: {body}");
        assert_eq!(body["segment"], segment, "{symbol}");
        assert_eq!(body["symbol"], symbol, "{symbol}");
        assert_eq!(body["key"], key, "{symbol}");
        assert_eq!(body["spots"], json!([hit(t0, close, t0)]), "{symbol}");
    }
}

#[test]
fn malformed_questions_are_refused_by_name_before_any_file_is_read() {
    let fixture = Fixture::new("spot-refusals");
    let t0 = open_second();
    let base = format!("feed=dhan&exchange=NSE&symbol=NIFTY&timeframe=1min&month=2025-05&t={t0}");
    let too_many = vec![t0.to_string(); MAX_SPOT_TIMES + 1].join(",");
    let at_cap = vec![t0.to_string(); MAX_SPOT_TIMES].join(",");
    let t = format!("t={t0}");
    for (query, word) in [
        (base.replace("feed=dhan", "feed=nobody"), "feed"),
        (base.replace("exchange=NSE", "exchange="), "both required"),
        (base.replace("symbol=NIFTY", "symbol="), "both required"),
        (base.replace("timeframe=1min", "timeframe="), "timeframe"),
        (base.replace("timeframe=1min", "timeframe=1hour"), "rung"),
        (base.replace("month=2025-05", "month=2025-13"), "month"),
        (base.replace("month=2025-05", "month=May"), "month"),
        (base.replace(&format!("&{t}"), ""), "t is required"),
        (base.replace(&t, &format!("{t},x")), "\"x\""),
        (base.replace(&t, "t=9223372036855"), "UTC second"),
        (base.replace(&t, &format!("t={too_many}")), "chunks"),
        (base.replace("symbol=NIFTY", "symbol=A%2FB"), "symbol"),
        (format!("{base}&asof=later"), "asof"),
    ] {
        let (status, body) = fixture.get(&query);
        assert_eq!(status, StatusCode::BAD_REQUEST, "{query}: {body}");
        assert!(
            body["error"].as_str().expect("named").contains(word),
            "{query}: {body}"
        );
        assert!(body.get("spots").is_none());
    }
    let (status, body) = fixture.get(&base.replace(&t, &format!("t={at_cap}")));
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        body["spots"].as_array().expect("answers").len(),
        MAX_SPOT_TIMES
    );
}

#[test]
fn an_unreadable_file_or_record_is_partial_content_with_the_fault_named() {
    let fixture = Fixture::new("spot-unreadable");
    let t0 = open_second();
    let original = fs::read(&fixture.index_path).expect("generated file");
    // A RECORD'S OWN BYTES CHANGED: its block no longer matches its seal.
    let stamp = (t0 * 1_000_000).to_le_bytes();
    let at = original
        .windows(8)
        .rposition(|w| w == stamp)
        .expect("the record's stamp is in the file");
    let mut damaged = original.clone();
    damaged[at] ^= 0x01;
    fs::write(&fixture.index_path, &damaged).expect("damage owned record");
    let (status, body) = fixture.at("NIFTY", &[t0]);
    assert_eq!(status, StatusCode::PARTIAL_CONTENT, "{body}");
    assert_eq!(body["spots"], json!([miss(t0, "spot_unreadable")]));
    let fault = body["faults"].as_str().expect("named fault");
    assert!(
        fault.contains("2025-05.bin") && fault.contains(&format!("t={t0}:")),
        "{fault}"
    );
    // THE WHOLE FILE UNREADABLE: every answer inside the month says so, and
    // the reason is named; outside it, the question is still the wrong one.
    fs::write(&fixture.index_path, b"not a bar file").expect("truncate owned file");
    let (status, body) = fixture.at("NIFTY", &[t0, t0 + 60, t0 + 40 * 86_400]);
    assert_eq!(status, StatusCode::PARTIAL_CONTENT, "{body}");
    assert_eq!(
        body["spots"],
        json!([
            miss(t0, "spot_unreadable"),
            miss(t0 + 60, "spot_unreadable"),
            miss(t0 + 40 * 86_400, "t_outside_month"),
        ])
    );
    assert!(body["faults"].is_string());
    fs::write(&fixture.index_path, &original).expect("restore exact bytes");
    let (status, body) = fixture.at("NIFTY", &[t0]);
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["spots"][0]["close"], 2_400_000);
    assert_eq!(
        fs::read(&fixture.index_path).expect("a GET writes nothing"),
        original
    );
}

/// **DB-066, DB-G80: THE ROUTE READS ON THE BLOCKING POOL, ADMITTED, AND A
/// FULL POOL IS A NAMED 429 WITH `Retry-After`.** The handler's body, read
/// from source, hands the whole answer to `run_store_read`; through the
/// handler the answer is the same; and the refusal names what was refused.
#[tokio::test]
async fn the_route_answers_from_the_store_read_pool_and_a_full_pool_is_a_named_429() {
    let source = include_str!("spot.rs");
    let handler = source
        .split("pub async fn spot_json(")
        .nth(1)
        .expect("the handler");
    // Through the store-read pool, with the request's cancellation flag
    // (D-3349, DB-N15): `cancellable(false, ..)` is `run_store_read`.
    assert!(
        handler.contains(
            "super::bars_route::cancellable(false, move |cancel| respond_with(&root, &query, cancel))"
        ),
        "{handler}"
    );
    // THE NAMED STORE-READ POOL (R3-08): `cancellable` picks the pool by name
    // and hands the request's flag to `cancellable_in`.
    let bars_route = include_str!("bars_route.rs");
    assert!(
        bars_route.contains("crate::detail::Named::StoreRead")
            && bars_route.contains("crate::detail::cancellable_in(named, work).await"),
        "a cancellable read that is not heavy is admitted by the store-read pool"
    );
    let greeks = include_str!("optiongreeks.rs")
        .split("pub async fn greeks_json(")
        .nth(1)
        .expect("the greeks handler")
        .to_owned();
    assert!(
        greeks.contains("super::bars_route::cancellable(false, move |cancel| {")
            && greeks.contains("RateTable::current(std::env::var_os(RATE_ENV))"),
        "{greeks}"
    );

    let (status, headers, body) = not_admitted("a spot read", &crate::detail::RunError::Saturated);
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(headers[axum::http::header::RETRY_AFTER], "1");
    assert!(
        body.contains("a spot read") && body.contains("retry"),
        "{body}"
    );
    let (status, headers, _) = not_admitted(
        "a spot read",
        &crate::detail::RunError::Join("gone".to_owned()),
    );
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert!(headers.get(axum::http::header::RETRY_AFTER).is_none());

    let _apart = crate::detail::apart_from_slot_owners().await;
    let fixture = Fixture::new("spot-handler");
    let t0 = open_second();
    let uri = format!(
        "/spot.json?feed=dhan&exchange=NSE&symbol=NIFTY&timeframe=1min&month=2025-05&t={t0}"
    )
    .parse()
    .expect("URI");
    let (status, _, body) =
        spot_json(axum::extract::State(Loaded::clone(&fixture.site)), uri).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(body.contains(r#""close":2400000"#), "{body}");
}

/// **AN ABANDONED SPOT READ STOPS, AND A LIVE ONE ANSWERS AS BEFORE** (DB-N15,
/// D-3349). The handler hands the read the request's cancellation flag; once
/// the client is gone the lookups stop by name before the next instant
/// instead of running to the last of up to 400. Before, the read had no flag
/// to look at and always ran to its end.
#[test]
fn a_spot_read_whose_client_went_away_stops_by_name() {
    let fixture = Fixture::new("spot-cancel");
    let t0 = open_second();
    let query = format!(
        "feed=dhan&exchange=NSE&symbol=NIFTY&timeframe=1min&month=2025-05&t={t0},{}",
        t0 + 60
    );
    let (gone, guard) = crate::bars::Cancel::pair();
    drop(guard);
    let (status, _, body) = super::spot::respond_with(&fixture.root, &query, &gone);
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert!(body.contains("cancelled"), "{body}");
    let (live, guard) = crate::bars::Cancel::pair();
    let (status, _, body) = super::spot::respond_with(&fixture.root, &query, &live);
    guard.disarm();
    assert_eq!(status, StatusCode::OK, "{body}");
    let (plain_status, _, plain) = respond(&fixture.root, &query);
    assert_eq!(
        (status, body),
        (plain_status, plain),
        "the flag changes nothing it does not stop"
    );
}

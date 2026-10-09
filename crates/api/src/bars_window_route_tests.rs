#![cfg(test)]
//! Actual window responses over finite generated records and owned failures.
#![allow(clippy::expect_used, clippy::indexing_slicing)]

use super::*;
use axum::http::StatusCode;
use serde_json::Value;
use std::fs;
use store::format::{Bar, OI_NULL};
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

const QUERY: &str = "feed=dhan&exchange=NSE&segment=INDEX&symbol=NIFTY&from=2025-05&to=2025-07";

struct Fixture {
    root: PathBuf,
    site: Loaded,
    paths: Vec<PathBuf>,
}

impl Fixture {
    fn new(name: &str) -> Self {
        let root = crate::scratch::path(name);
        let masters = root.join("masters");
        fs::create_dir_all(&masters).expect("offline empty master root");
        let site = Loaded::new(Site::load(&masters, &root));
        let mut fixture = Self {
            root,
            site,
            paths: Vec::new(),
        };
        for (month, closes, interest) in [(5, [100, 110], [OI_NULL, 0]), (6, [200, 180], [100, 50])]
        {
            let date = Day::new(2025, month, 2).expect("generated date");
            let path = StorePath::new(PathParts {
                vendor: Vendor::Dhan,
                exchange: "NSE",
                segment: "INDEX",
                symbol: "NIFTY",
                contract: None,
                timeframe: Timeframe::MINUTE_1,
                month: YearMonth::new(2025, month).expect("month"),
                file: FileKind::Bars,
            })
            .expect("generated path");
            let hash = brutex_core::universe::fnv1a("NIFTY").to_le_bytes();
            let symbol = u32::from_le_bytes(hash[..4].try_into().expect("low32"));
            let mut file = store::file::BarFile::open_or_create(&fixture.root, path, symbol)
                .expect("owned generated bar writer");
            let rows: Vec<_> = closes
                .iter()
                .copied()
                .enumerate()
                .map(|(index, close)| {
                    let ordinal =
                        i64::from(month - 5) * 2 + i64::try_from(index).expect("two rows") + 1;
                    Bar {
                        ts_micros: i64::from(date.days_from_epoch()) * 86_400_000_000
                            + 13_500_000_000
                            + i64::try_from(index).expect("row") * 60_000_000,
                        open: close,
                        high: close + ordinal * 10,
                        low: close - ordinal * 10,
                        close,
                        volume: ordinal * 10,
                        open_interest: interest[index],
                    }
                })
                .collect();
            file.append(&rows).expect("complete generated records");
            fixture.paths.push(path.to_path_buf(&fixture.root));
        }
        fixture
    }

    async fn get_raw(&self, query: &str) -> (StatusCode, Value) {
        let uri = format!("/bars/window.json?{query}").parse().expect("URI");
        // ADMITTED THROUGH THE STORE-READ POOL (resources-4, P1-04-01,
        // D-2593), so this keeps apart from a test that holds every slot.
        let _apart = crate::detail::apart_from_slot_owners().await;
        let (status, headers, body) =
            bars_window_json(axum::extract::State(Loaded::clone(&self.site)), uri).await;
        assert_eq!(headers[0].1, "application/json; charset=utf-8");
        (status, serde_json::from_str(&body).expect("window JSON"))
    }

    async fn get(&self, options: &str) -> (StatusCode, Value) {
        self.get_raw(&format!("{QUERY}&{options}")).await
    }

    async fn get_month(&self, options: &str) -> (StatusCode, Value) {
        let uri = format!(
            "/bars.json?feed=dhan&exchange=NSE&segment=INDEX&symbol=NIFTY&month=2025-05&{options}"
        )
        .parse()
        .expect("month URI");
        let (status, headers, body) =
            bars_json(axum::extract::State(Loaded::clone(&self.site)), uri).await;
        assert_eq!(headers[0].1, "application/json; charset=utf-8");
        (status, serde_json::from_str(&body).expect("month JSON"))
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[tokio::test]
async fn single_month_day_bounds_refuse_typos_and_reversed_ranges_before_open() {
    let fixture = Fixture::new("month-date-refusals");
    for (options, field) in [
        ("from=not-a-day", "from"),
        ("to=not-a-day", "to"),
        ("from=2025-02-29", "from"),
        ("to=2025-04-31", "to"),
        ("from=2025-13-02", "from"),
        ("to=2025-00-02", "to"),
        ("from=2025-05-02&to=bad", "to"),
        ("from=bad&to=2025-05-02", "from"),
        ("from=2025-05-03&to=2025-05-02", "from"),
    ] {
        let (status, response) = fixture.get_month(options).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{options}: {response}");
        assert!(
            response["error"]
                .as_str()
                .expect("named refusal")
                .contains(field)
        );
        assert!(response.get("bars").is_none());
    }
    let path = &fixture.paths[0];
    let held = path.with_extension("held");
    fs::rename(path, &held).expect("withhold owned source temporarily");
    let (status, response) = fixture.get_month("from=bad").await;
    fs::rename(held, path).expect("restore exact source");
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(
        response["error"]
            .as_str()
            .expect("date refusal")
            .contains("from")
    );
    assert!(
        !response["error"]
            .as_str()
            .expect("date refusal")
            .contains("cannot open")
    );
}

#[tokio::test]
async fn single_month_day_windows_preserve_valid_rows_nulls_and_source_bytes() {
    let fixture = Fixture::new("month-date-windows");
    let originals: Vec<_> = fixture
        .paths
        .iter()
        .flat_map(|path| [path.clone(), path.with_extension("crc")])
        .map(|path| {
            let bytes = fs::read(&path).expect("owned source or checksum");
            (path, bytes)
        })
        .collect();
    let (status, full) = fixture.get_month("").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(full.as_array().expect("bars").len(), 2);
    assert!(full[0]["oi"].is_null());
    assert_eq!(full[1]["oi"], 0);
    assert_eq!(full[0]["o"], 100);
    assert_eq!(full[1]["c"], 110);
    for options in [
        "from=&to=",
        "from=2025-05-02&to=2025-05-02",
        "from=2025-05-01&to=2025-05-02",
        "from=2025-05-02",
        "to=2025-05-02",
    ] {
        let (status, response) = fixture.get_month(options).await;
        assert_eq!(status, StatusCode::OK, "{options}: {response}");
        assert_eq!(response, full, "{options}");
    }
    for options in [
        "from=2025-05-03",
        "to=2025-05-01",
        "from=2025-05-03&to=2025-05-04",
    ] {
        let (status, response) = fixture.get_month(options).await;
        assert_eq!(status, StatusCode::OK, "{options}: {response}");
        assert_eq!(response, serde_json::json!([]), "{options}");
    }
    for (path, bytes) in originals {
        assert_eq!(fs::read(path).expect("unchanged source"), bytes);
    }
}

#[test]
fn single_day_window_checks_both_ist_midnights_at_microsecond_precision() {
    // 2025-05-02 00:00 IST is 2025-05-01 18:30 UTC. These are literal UTC
    // instants, independent of the parser's civil-day conversion arithmetic.
    let first = 1_746_124_200_000_000;
    let end = 1_746_210_600_000_000;
    let bounds = day_window_bounds("from=2025-05-02&to=2025-05-02").expect("same day");
    assert_eq!(bounds, (Some(first), Some(end)));
    let rows: Vec<_> = [first - 1, first, end - 1, end]
        .into_iter()
        .map(|ts_micros| Bar {
            ts_micros,
            open: 100,
            high: 110,
            low: 90,
            close: 100,
            volume: 1,
            open_interest: OI_NULL,
        })
        .collect();
    let response: Value =
        serde_json::from_str(&bars_array(&rows, bounds.0, bounds.1).0).expect("exact window");
    assert_eq!(response.as_array().expect("bars").len(), 2);
    assert_eq!(response[0]["t"], first / 1_000_000);
    assert_eq!(response[1]["t"], (end - 1) / 1_000_000);
    assert!(response[0]["oi"].is_null());
}

#[tokio::test]
async fn window_pages_preserve_integer_values_month_gaps_and_change_provenance() {
    let fixture = Fixture::new("window-route-pages");
    let originals: Vec<_> = fixture
        .paths
        .iter()
        .map(|path| fs::read(path).expect("source"))
        .collect();
    let (status, page) = fixture.get("dir=asc&offset=1&limit=2").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(page["total"], 4);
    assert_eq!(page["months_read"], 2);
    assert_eq!(page["months_missing"], 1);
    assert_eq!(page["scanned"], false);
    assert!(page["faults"].is_null());
    assert!(page["extremes"].is_null());
    assert_eq!(page["bars"][0]["c"], 110);
    assert_eq!(page["bars"][0]["oi"], 0);
    assert_eq!(page["bars"][0]["chg"], 1_000);
    assert!(page["bars"][0]["chg_why"].is_null());
    assert_eq!(page["bars"][0]["oichg_why"], "oi_null_before");
    assert_eq!(page["bars"][1]["c"], 200);
    // June's first bar stands behind May's last, read in the same request
    // (Z1-slice11-F4, D-1762), so it has a change and May's zero OI names why
    // the OI change has none.
    assert_eq!(
        page["bars"][1]["chg"],
        crate::server::basis_points(110, 200).expect("a change")
    );
    assert!(page["bars"][1]["chg_why"].is_null());
    assert_eq!(page["bars"][1]["oichg_why"], "previous_oi_zero");
    let (_, opening) = fixture.get("dir=asc&offset=0&limit=1").await;
    assert_eq!(opening["bars"][0]["c"], 100);
    assert!(opening["bars"][0]["chg"].is_null());
    assert_eq!(opening["bars"][0]["chg_why"], "first_bar_in_file");
    let (_, scan) = fixture.get("sort=c&extremes=true&limit=2").await;
    assert_eq!(scan["scanned"], true);
    assert_eq!(scan["extremes"]["range"], 80);
    assert_eq!(scan["extremes"]["volume"], 40);
    assert_eq!(scan["bars"][0]["c"], 200);
    assert_eq!(scan["bars"][1]["c"], 180);
    assert_eq!(scan["bars"][1]["oichg"], -5_000);
    assert!(scan["bars"][1]["oichg_why"].is_null());
    for sort in ["ts", "o", "h", "l", "c", "v", "oi"] {
        let (status, sorted) = fixture.get(&format!("sort={sort}&limit=4")).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(sorted["scanned"], sort != "ts");
        let mut closes: Vec<_> = sorted["bars"]
            .as_array()
            .expect("rows")
            .iter()
            .map(|bar| bar["c"].as_i64().expect("integer close"))
            .collect();
        closes.sort_unstable();
        assert_eq!(closes, [100, 110, 180, 200]);
    }
    let (_, first) = fixture.get("dir=asc&limit=1").await;
    assert!(first["bars"][0]["oi"].is_null());
    assert_eq!(first["bars"][0]["oichg_why"], "oi_null");
    // `limit=0` is refused since D-1765 and is pinned with the other refusals.
    for options in ["offset=4", "sort=v&offset=99"] {
        let (status, empty) = fixture.get(options).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(empty["bars"], serde_json::json!([]));
        assert_eq!(empty["total"], 4);
    }
    let (status, contract) = fixture
        .get("contract=2025-06-26-2500000-CE&extremes=true")
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(contract["total"], 0);
    assert_eq!(contract["months_missing"], 3);
    assert_eq!(
        contract["extremes"],
        serde_json::json!({"range": 0, "volume": 0})
    );
    for (path, expected) in fixture.paths.iter().zip(originals) {
        assert_eq!(fs::read(path).expect("read-only route"), expected);
    }
}

#[tokio::test]
async fn window_corrupt_months_are_faults_instead_of_silent_missing_months() {
    let fixture = Fixture::new("window-route-corruption");
    let path = &fixture.paths[1];
    let original = fs::read(path).expect("original June authority");
    fs::write(path, b"unreadable generated month").expect("truncate owned authority");
    for options in ["dir=asc", "sort=c&extremes=1"] {
        let (status, partial) = fixture.get(options).await;
        assert_eq!(status, StatusCode::PARTIAL_CONTENT, "{partial}");
        assert_eq!(partial["months_read"], 1);
        assert_eq!(partial["months_missing"], 1);
        assert_eq!(partial["total"], 2);
        assert_eq!(partial["bars"].as_array().expect("healthy rows").len(), 2);
        assert!(
            partial["faults"]
                .as_str()
                .expect("named unreadable file")
                .contains("2025-06")
        );
    }
    let (status, all_unreadable) = fixture
        .get_raw(&QUERY.replace("from=2025-05", "from=2025-06"))
        .await;
    assert_eq!(status, StatusCode::PARTIAL_CONTENT);
    assert_eq!(all_unreadable["months_read"], 0);
    assert_eq!(all_unreadable["months_missing"], 1);
    assert_eq!(all_unreadable["bars"], serde_json::json!([]));
    assert!(all_unreadable["faults"].is_string());
    fs::write(path, &original).expect("restore exact bytes");
    let (status, recovered) = fixture.get("dir=asc").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(recovered["total"], 4);
    assert!(recovered["faults"].is_null());
    assert_eq!(fs::read(path).expect("read-only recovery"), original);
}

#[tokio::test]
async fn window_unavailable_store_roots_cannot_be_reported_as_empty_history() {
    let fixture = Fixture::new("window-route-root");
    let moved = fixture.root.with_extension("owned-detached-root");
    assert!(!moved.exists());
    fs::rename(&fixture.root, &moved).expect("detach owned generated store");
    let (status, absent) = fixture.get("").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(
        absent["error"]
            .as_str()
            .expect("refusal")
            .contains("store root")
    );
    fs::write(&fixture.root, b"owned obstruction").expect("store path is no longer a directory");
    let (status, blocked) = fixture.get("").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(
        blocked["error"]
            .as_str()
            .expect("refusal")
            .contains("not a directory")
    );
    fs::remove_file(&fixture.root).expect("remove only owned obstruction");
    fs::rename(moved, &fixture.root).expect("reattach same owned store");
    let (status, recovered) = fixture.get("").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(recovered["total"], 4);
}

#[tokio::test]
async fn window_malformed_inputs_refuse_instead_of_answering_another_page() {
    let fixture = Fixture::new("window-route-inputs");
    for (key, value) in [
        ("feed", "unknown"),
        ("from", "bad"),
        ("to", "2025-13"),
        ("from", "2025-08"),
        ("symbol", "bad-symbol"),
    ] {
        let query = QUERY
            .split('&')
            .map(|pair| {
                if pair.starts_with(&format!("{key}=")) {
                    format!("{key}={value}")
                } else {
                    pair.to_owned()
                }
            })
            .collect::<Vec<_>>()
            .join("&");
        let (status, refused) = fixture.get_raw(&query).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{query}: {refused}");
        assert!(
            refused["error"]
                .as_str()
                .is_some_and(|reason| !reason.is_empty())
        );
        assert!(refused.get("bars").is_none());
    }
    for options in [
        "offset=banana",
        "limit=-1",
        "sort=unknown",
        "contract=bad",
        "timeframe=17min",
    ] {
        let (status, refused) = fixture.get(options).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{options}: {refused}");
        assert!(
            refused["error"]
                .as_str()
                .is_some_and(|reason| !reason.is_empty())
        );
    }
}

/// An unknown `dir` or `extremes` is refused and named, never read as the
/// default: `dir=ASC` once answered newest first with 200, and `extremes=yes`
/// dropped the extremes silently (Z1-slice14-F3, D-1762).
#[test]
fn an_unknown_direction_or_extremes_flag_is_refused_not_defaulted() {
    let base = "feed=zerodha&from=2025-01&to=2025-02&timeframe=1min&sort=ts";
    for (extra, desc, extremes) in [
        ("", true, false),
        ("&dir=desc", true, false),
        ("&dir=asc", false, false),
        ("&extremes=0", true, false),
        ("&extremes=false", true, false),
        ("&extremes=1", true, true),
        ("&extremes=true", true, true),
    ] {
        let ask = WindowAsk::parse(&format!("{base}{extra}")).expect(extra);
        assert_eq!((ask.desc, ask.want_extremes), (desc, extremes), "{extra}");
    }
    // The page size is served as asked at both ends of its range (P1-01-01).
    for (extra, limit) in [("", 200), ("&limit=1", 1), ("&limit=1000", 1_000)] {
        let ask = WindowAsk::parse(&format!("{base}{extra}")).expect(extra);
        assert_eq!(ask.limit, limit, "{extra}");
    }
    for (extra, named) in [
        ("&dir=ASC", "\"ASC\" is not a direction"),
        ("&dir=ascending", "\"ascending\" is not a direction"),
        ("&extremes=yes", "\"yes\" is not an extremes flag"),
        ("&extremes=2", "\"2\" is not an extremes flag"),
        ("&limit=0", "0 is not a page size"),
        ("&limit=1001", "1001 is not a page size"),
        ("&limit=5000", "Accepted: 1 to 1000"),
    ] {
        let why = WindowAsk::parse(&format!("{base}{extra}"))
            .err()
            .expect("an unknown value must be refused");
        assert!(why.contains(named), "{extra}: {why}");
    }
}

/// **CE-60. A BAR VALUE A BROWSER'S JSON NUMBER CANNOT HOLD IS WITHHELD BY
/// NAME, NEVER SENT TO BE ROUNDED.**
///
/// The store admits any non-negative `i64` count and price, and both bar
/// routes wrote them as bare JSON numbers: `9007199254740993` parses in a
/// browser as `9007199254740992`, and `/db` and `/markets` drew the rounded
/// value with no refusal.
#[test]
fn a_bar_value_past_two_to_the_fifty_three_is_withheld_by_name_not_rounded() {
    let past = (1_i64 << 53) + 1;
    let at = 1_746_157_500_000_000;
    let bar = |ts_micros: i64, volume: i64| Bar {
        ts_micros,
        open: 100,
        high: 110,
        low: 90,
        close: 100,
        volume,
        open_interest: OI_NULL,
    };
    let rows = [bar(at, 1), bar(at + 60_000_000, past)];
    let (out, withheld) = bars_array(&rows, None, None);
    assert!(
        !out.contains(&past.to_string()),
        "never sent as a number: {out}"
    );
    let sent: Value = serde_json::from_str(&out).expect("the exact bar is still sent");
    assert_eq!(sent.as_array().expect("bars").len(), 1);
    assert_eq!(withheld.len(), 1, "{withheld:?}");
    assert!(
        withheld[0].contains("`v`") && withheld[0].contains(&past.to_string()),
        "named by field and value: {withheld:?}"
    );
    // EXACTLY 2^53 − 1 IS EXACT, and is sent.
    let (edge, none) = bars_array(&[bar(at, (1_i64 << 53) - 1)], None, None);
    assert!(
        none.is_empty() && edge.contains("9007199254740991"),
        "{edge}"
    );

    let window = bars::Window {
        total: 2,
        months_read: 1,
        months_missing: 0,
        bars: rows
            .iter()
            .map(|row| bars::WindowBar {
                bar: *row,
                chg: None,
                chg_why: "first bar",
                oichg: None,
                oichg_why: "first bar",
            })
            .collect(),
        faults: Vec::new(),
        extremes: Some(bars::Extremes {
            range: past,
            volume: 1,
        }),
    };
    let (body, withheld) = render_window(&window, false);
    let sent: Value = serde_json::from_str(&body).expect("a window body");
    assert_eq!(sent["bars"].as_array().expect("bars").len(), 1);
    assert!(sent["extremes"].is_null(), "{body}");
    assert_eq!(withheld, 2, "one bar and the extremes");
    assert!(
        sent["faults"].as_str().expect("named").contains("`v`"),
        "{body}"
    );
}

/// resources-4, P1-04-01, D-2593. On the old code `/bars/window.json` and
/// `/backtest.json` ran inline on an async worker and answered 200 however
/// many ran at once; with every store-read slot held they now answer 429
/// before reading anything, the backtest refusal in the body shape its page
/// parses, and a released slot admits the same window again (200, 4 bars).
#[tokio::test]
async fn bars_window_and_backtest_reads_run_in_the_store_read_pool_and_answer_429_when_it_is_full()
{
    let fixture = Fixture::new("window-route-admission");
    let ask = || {
        format!("/bars/window.json?{QUERY}")
            .parse::<axum::http::Uri>()
            .expect("URI")
    };
    let apart = crate::detail::apart_from_slot_owners().await;
    let mut held = crate::detail::take_every_store_read_slot(&apart);
    let (status, _, body) =
        bars_window_json(axum::extract::State(Loaded::clone(&fixture.site)), ask()).await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS, "{body}");
    assert!(body.contains("bars window read not admitted"), "{body}");
    assert!(!body.contains(r#""bars""#), "nothing was read: {body}");

    held.extend(crate::detail::take_every_store_read_slot(&apart));
    let (status, _, body) = crate::backtest::backtest_json(
        "/backtest.json?limit=1"
            .parse::<axum::http::Uri>()
            .expect("a uri"),
    )
    .await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS, "{body}");
    assert!(body.contains("was not admitted"), "{body}");
    assert!(
        body.contains(r#""runs":[]"#),
        "the page's own shape: {body}"
    );

    drop(held);
    let (status, _, body) =
        bars_window_json(axum::extract::State(Loaded::clone(&fixture.site)), ask()).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let sent: Value = serde_json::from_str(&body).expect("window JSON");
    assert_eq!(sent["total"], 4, "{body}");
    drop(apart);
    // AND BOTH HANDLERS CALL THE POOL, read off the source so a later inline
    // rewrite cannot keep this test green by luck.
    let server = include_str!("server.rs");
    let window = server
        .split_once("\nasync fn bars_window_json(")
        .and_then(|(_, rest)| rest.split_once("\n}\n"))
        .map_or("", |(body, _)| body);
    assert!(
        window.contains("crate::detail::run_store_read("),
        "{window}"
    );
    let backtest = include_str!("backtest.rs");
    let handler = backtest
        .split_once("\npub async fn backtest_json(")
        .and_then(|(_, rest)| rest.split_once("\n}\n"))
        .map_or("", |(body, _)| body);
    assert!(
        handler.contains("crate::detail::run_store_read("),
        "{handler}"
    );
}

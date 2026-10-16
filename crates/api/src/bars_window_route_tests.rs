#![cfg(test)]
//! Actual window responses over finite generated records and owned failures.
#![allow(clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

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
            // The run the writer kept before the small-month rule (D-4933).
            crate::scratch::plant_pre_rule_sort_index(
                &path.to_path_buf(&fixture.root),
                &store::sort_index::Shape::of(path.month(), 60, symbol),
                &rows,
                rows.len(),
            );
            fixture.paths.push(path.to_path_buf(&fixture.root));
        }
        fixture
    }

    async fn get_raw(&self, query: &str) -> (StatusCode, Value) {
        window_get(&self.site, query).await
    }

    async fn get(&self, options: &str) -> (StatusCode, Value) {
        self.get_raw(&format!("{QUERY}&{options}")).await
    }

    async fn get_month(&self, options: &str) -> (StatusCode, Value) {
        month_get(
            &self.site,
            &format!("feed=dhan&exchange=NSE&segment=INDEX&symbol=NIFTY&month=2025-05&{options}"),
        )
        .await
    }
}

/// `/bars/window.json?{query}` through the handler. The blocking pools are
/// shared by every test in this binary, so a `429` (a full pool, by name) is
/// asked again rather than read as the answer.
async fn window_get(site: &Loaded, query: &str) -> (StatusCode, Value) {
    for _ in 0..400 {
        let uri = format!("/bars/window.json?{query}").parse().expect("URI");
        let (status, headers, body) =
            bars_window_json(axum::extract::State(Loaded::clone(site)), uri).await;
        assert_eq!(headers[0].1, "application/json; charset=utf-8");
        if status != StatusCode::TOO_MANY_REQUESTS {
            return (status, serde_json::from_str(&body).expect("window JSON"));
        }
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
    panic!("the window pool stayed full for two seconds");
}

/// `/bars.json?{query}` through the handler, asked again on a full pool.
async fn month_get(site: &Loaded, query: &str) -> (StatusCode, Value) {
    for _ in 0..400 {
        let uri = format!("/bars.json?{query}").parse().expect("month URI");
        let (status, headers, body) =
            bars_json(axum::extract::State(Loaded::clone(site)), uri).await;
        assert_eq!(headers[0].1, "application/json; charset=utf-8");
        if status != StatusCode::TOO_MANY_REQUESTS {
            return (status, serde_json::from_str(&body).expect("month JSON"));
        }
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
    panic!("the store-read pool stayed full for two seconds");
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
    // AN INDEX'S STORED ZERO IS NOT A MEASUREMENT (D-3322): served null, with
    // the reason, on every row; its volume is named as no traded volume.
    assert!(full[1]["oi"].is_null());
    assert_eq!(full[1]["oichg_why"], "not_a_derivative");
    assert_eq!(full[1]["v_why"], "index_no_volume");
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
    assert_eq!(page["offset"], 1);
    assert_eq!(page["months_read"], 2);
    assert_eq!(page["months_missing"], 1);
    assert_eq!(
        page["months_outside_life"], 1,
        "July is after the series' last month"
    );
    assert_eq!(page["missing_months"], serde_json::json!([]));
    assert_eq!(page["scanned"], false);
    assert!(page["faults"].is_null());
    assert!(page["extremes"].is_null());
    assert!(page["at_index"].is_null());
    assert_eq!(
        page["series_totals"],
        serde_json::json!([{"key": "NSE-INDEX-NIFTY", "total": 4}])
    );
    assert_eq!(page["bars"][0]["c"], 110);
    assert_eq!(page["bars"][0]["month"], "2025-05");
    // AN INDEX HAS NO OPEN INTEREST AND NO TRADED VOLUME (D-3322).
    assert!(page["bars"][0]["oi"].is_null());
    assert_eq!(page["bars"][0]["oichg_why"], "not_a_derivative");
    assert_eq!(page["bars"][0]["v_why"], "index_no_volume");
    assert_eq!(page["bars"][0]["chg"], 1_000);
    assert!(page["bars"][0]["chg_why"].is_null());
    assert_eq!(page["bars"][0]["phase"], "continuous");
    assert!(
        page["bars"][0].get("key").is_none(),
        "one series names no key per bar"
    );
    assert_eq!(page["bars"][1]["c"], 200);
    // JUNE'S FIRST BAR STANDS BEHIND MAY'S LAST, READ IN THE SAME REQUEST,
    // AND THE CALENDAR SAYS TWENTY SESSIONS LIE BETWEEN THEM: the change is
    // refused by name rather than measured across the hole (D-3325 rule 4).
    assert!(page["bars"][1]["chg"].is_null());
    assert_eq!(page["bars"][1]["chg_why"], "previous_session_not_adjacent");
    let (_, opening) = fixture.get("dir=asc&offset=0&limit=1").await;
    assert_eq!(opening["bars"][0]["c"], 100);
    assert!(opening["bars"][0]["chg"].is_null());
    assert_eq!(opening["bars"][0]["chg_why"], "no_earlier_stored_bar");
    let (_, scan) = fixture.get("sort=c&extremes=true&limit=2").await;
    assert_eq!(scan["scanned"], true);
    // SERVED FROM THE MONTHS' SORT INDEXES (D-3349): no row is read beyond
    // the page, and the extremes are the runs' own summaries.
    assert_eq!(scan["scan_rows"], 0);
    assert_eq!(scan["sort_index"]["indexed_rows"], 4);
    assert_eq!(scan["sort_index"]["scanned_rows"], 0);
    assert_eq!(scan["extremes"]["range"], 80);
    assert_eq!(scan["extremes"]["volume"], 0, "an index trades no volume");
    assert!(scan["extremes"]["volume_max"].is_null());
    assert_eq!(scan["extremes"]["close_max"]["v"], 200);
    assert_eq!(scan["extremes"]["close_max"]["key"], "NSE-INDEX-NIFTY");
    assert_eq!(scan["extremes"]["close_min"]["v"], 100);
    assert_eq!(scan["extremes"]["range_max"]["v"], 80);
    assert_eq!(scan["bars"][0]["c"], 200);
    assert_eq!(scan["bars"][1]["c"], 180);
    assert_eq!(scan["bars"][1]["oichg_why"], "not_a_derivative");
    for sort in ["ts", "o", "h", "l", "c", "v", "oi", "tod"] {
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
    assert_eq!(contract["months_outside_life"], 3);
    assert_eq!(contract["extremes"]["range"], 0);
    assert_eq!(contract["extremes"]["volume"], 0);
    assert!(contract["extremes"]["close_max"].is_null());
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
        bars: rows
            .iter()
            .map(|row| bars::WindowBar {
                bar: *row,
                chg: None,
                chg_why: "first bar",
                oichg: None,
                oichg_why: "first bar",
                series: 0,
                phase: None,
                oi_applies: true,
                v_why: None,
            })
            .collect(),
        extremes: Some(bars::Extremes {
            range: past,
            volume: 1,
            ..bars::Extremes::default()
        }),
        series: vec![bars::SeriesTotal {
            key: "NSE-INDEX-NIFTY".to_owned(),
            total: 2,
        }],
        ..bars::Window::default()
    };
    let (body, withheld, faulted) = render_window(&window, false);
    assert!(!faulted, "no fault was named by the window itself");
    let sent: Value = serde_json::from_str(&body).expect("a window body");
    assert_eq!(sent["bars"].as_array().expect("bars").len(), 1);
    assert!(sent["extremes"].is_null(), "{body}");
    assert_eq!(withheld, 2, "one bar and the extremes");
    assert!(
        sent["faults"].as_str().expect("named").contains("`v`"),
        "{body}"
    );
}

// ---------------------------------------------------------------------------
// D-3325 and D-3326, WP-31: several series in one request, `at=`, day and
// time-of-day bounds, `sort=tod`, the change refusals and the extremes. Every
// fixture is invented: prices no exchange ever printed, on real calendar days
// so the calendar rules have something to judge.
// ---------------------------------------------------------------------------

/// A scratch store of invented bars, removed when dropped.
struct Store {
    root: PathBuf,
}

impl Store {
    fn new(name: &str) -> Self {
        let root = crate::scratch::path(name);
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("masters")).expect("offline empty master root");
        Self { root }
    }

    /// Appends `rows` (time ordered) to their IST months of one series.
    fn write(
        &self,
        vendor: Vendor,
        segment: &str,
        symbol: &str,
        contract: Option<&str>,
        timeframe: Timeframe,
        rows: &[Bar],
    ) {
        let contract =
            contract.map(|text| brutex_core::instrument::Contract::parse(text).expect("contract"));
        let hash = brutex_core::universe::fnv1a(symbol).to_le_bytes();
        let symbol_id = u32::from_le_bytes(hash[..4].try_into().expect("low32"));
        let mut at = 0;
        while at < rows.len() {
            let month = month_of(rows[at].ts_micros);
            let mut end = at;
            while end < rows.len() && month_of(rows[end].ts_micros) == month {
                end += 1;
            }
            let path = StorePath::new(PathParts {
                vendor,
                exchange: "NSE",
                segment,
                symbol,
                contract,
                timeframe,
                month,
                file: FileKind::Bars,
            })
            .expect("generated path");
            let mut file = store::file::BarFile::open_or_create(&self.root, path, symbol_id)
                .expect("owned generated bar writer");
            file.append(&rows[at..end])
                .expect("generated records append");
            // The run the writer kept before the small-month rule (D-4933),
            // for a month this call made; a month a planted run already
            // covers was kept in step by the writer itself.
            crate::scratch::plant_pre_rule_sort_index(
                &path.to_path_buf(&self.root),
                &store::sort_index::Shape::of(month, timeframe.secs(), symbol_id),
                &rows[at..end],
                end - at,
            );
            at = end;
        }
    }

    fn site(&self) -> Loaded {
        Loaded::new(Site::load(&self.root.join("masters"), &self.root))
    }

    fn window(&self, query: &bars::WindowQuery) -> bars::Window {
        bars::window(&self.root, query).expect("a legal window")
    }
}

impl Drop for Store {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// The IST month of a stamp.
fn month_of(ts: i64) -> YearMonth {
    let day = u32::try_from((ts / 1_000_000 + 19_800).div_euclid(86_400)).expect("a day");
    Day::from_days(day)
        .expect("a day")
        .year_month()
        .expect("a month")
}

/// UTC micros of an IST wall clock.
fn ist(year: u16, mon: u8, date: u8, hour: i64, minute: i64, sec: i64) -> i64 {
    let day = i64::from(Day::new(year, mon, date).expect("a date").days_from_epoch());
    (day * 86_400 + hour * 3_600 + minute * 60 + sec - 19_800) * 1_000_000
}

/// An invented bar: the close given, a ten-paisa range, OI as given.
fn bar(ts_micros: i64, close: i64, volume: i64, open_interest: i64) -> Bar {
    Bar {
        ts_micros,
        open: close,
        high: close + 5,
        low: close - 5,
        close,
        volume,
        open_interest,
    }
}

fn month(y: u16, m: u8) -> YearMonth {
    YearMonth::new(y, m).expect("month")
}

/// A query over `keys` at `timeframe`, time ordered, `limit` rows.
fn query(keys: &[&str], timeframe: Timeframe, from: YearMonth, to: YearMonth) -> bars::WindowQuery {
    bars::WindowQuery {
        vendor: Vendor::Gdfl,
        timeframe,
        series: keys
            .iter()
            .map(|key| bars::SeriesKey::parse(key).expect("a census key"))
            .collect(),
        from,
        to,
        sort: bars::SortKey::Ts,
        desc: false,
        offset: Some(0),
        limit: 1_000,
        want_extremes: false,
        at: None,
        filter: bars::Filter::default(),
        lookback: true,
        coverage: false,
    }
}

const CE: &str = "NSE-FNO-NIFTY-2019-03-28-1150000-CE";
const PE: &str = "NSE-FNO-NIFTY-2019-03-28-1150000-PE";

/// Two option series of one strike, one-second bars over two months, with
/// instants both share (ties) and instants only one holds.
fn pair(name: &str) -> (Store, Vec<(i64, usize, i64)>) {
    let store = Store::new(name);
    let mut all = Vec::new();
    for (series, contract, base, step) in [
        (0_usize, "2019-03-28-1150000-CE", 70_000_i64, 3_i64),
        (1, "2019-03-28-1150000-PE", 40_000, 5),
    ] {
        let mut rows = Vec::new();
        for (day_at, (m, d)) in [(2_u8, 27_u8), (2, 28), (3, 19), (3, 20)]
            .into_iter()
            .enumerate()
        {
            for k in 0..40_i64 {
                // Every `step`-th second from the open, so the two series share
                // some instants (ties) and not others.
                let ts = ist(2019, m, d, 9, 15, k * step);
                let close = base + i64::try_from(day_at).expect("small") * 1_000 + k * 7 % 97;
                rows.push(bar(ts, close, 75 * (k % 9 + 1), 150_000 + k * 75));
                all.push((ts, series, close));
            }
        }
        store.write(
            Vendor::Gdfl,
            "FNO",
            "NIFTY",
            Some(contract),
            Timeframe::SECOND_1,
            &rows,
        );
    }
    all.sort_unstable();
    (store, all)
}

/// **A CE+PE PAIR IS MERGED BY TIME, AND EVERY PAGE IS THE TRUE PAGE.**
/// DB-002, DB-G92: page 1 was the newest 50 rows of the CE alone and the PE's
/// newest bar sat 1,72,339 rows later. The merge is checked against a brute
/// force over every offset, both directions, at several page sizes; ties at
/// one instant come in the order the series were asked.
#[test]
fn several_series_merge_by_time_on_every_page_in_both_directions() {
    let (store, truth) = pair("merge-pair");
    let total = truth.len();
    for desc in [false, true] {
        let mut expected: Vec<(i64, usize, i64)> = truth.clone();
        if desc {
            expected.reverse();
        }
        for limit in [1_usize, 7, 50] {
            for offset in (0..=total + 3).step_by(limit) {
                let mut asked = query(
                    &[CE, PE],
                    Timeframe::SECOND_1,
                    month(2019, 2),
                    month(2019, 3),
                );
                asked.desc = desc;
                asked.offset = Some(offset);
                asked.limit = limit;
                let got = store.window(&asked);
                assert_eq!(got.total, u64::try_from(total).expect("fits"));
                let rows: Vec<(i64, usize, i64)> = got
                    .bars
                    .iter()
                    .map(|row| (row.bar.ts_micros, row.series, row.bar.close))
                    .collect();
                let want: Vec<_> = expected.iter().copied().skip(offset).take(limit).collect();
                assert_eq!(rows, want, "desc={desc} offset={offset} limit={limit}");
            }
        }
    }
    let got = store.window(&query(
        &[CE, PE],
        Timeframe::SECOND_1,
        month(2019, 2),
        month(2019, 3),
    ));
    assert_eq!(
        got.series,
        vec![
            bars::SeriesTotal {
                key: CE.to_owned(),
                total: 160
            },
            bars::SeriesTotal {
                key: PE.to_owned(),
                total: 160
            },
        ],
        "the total is the sum of the series' totals"
    );
}

/// **A PRICE SORT OVER SEVERAL SERIES IS THE GLOBAL RANKING.** DB-003,
/// DB-G82, DB-G93: page 2 of an 80-series close sort overlapped the true
/// page 2 on 22 of 50 rows, because each series was asked for its own
/// offset. Every sortable column, both directions, every page.
#[test]
fn a_sort_over_several_series_pages_through_the_global_ranking() {
    let (store, _) = pair("merge-sorted");
    for (sort, field) in [
        (bars::SortKey::Close, 2_usize),
        (bars::SortKey::Volume, 3),
        (bars::SortKey::OpenInterest, 4),
        (bars::SortKey::TimeOfDay, 5),
    ] {
        for desc in [false, true] {
            let mut asked = query(
                &[CE, PE],
                Timeframe::SECOND_1,
                month(2019, 2),
                month(2019, 3),
            );
            asked.sort = sort;
            asked.desc = desc;
            asked.limit = 1_000;
            let everything = store.window(&asked);
            assert_eq!(everything.bars.len(), 320);
            let value = |row: &bars::WindowBar| match field {
                2 => row.bar.close,
                3 => row.bar.volume,
                4 => row.bar.open_interest,
                _ => (row.bar.ts_micros / 1_000_000 + 19_800).rem_euclid(86_400),
            };
            for pair in everything.bars.windows(2) {
                let (a, b) = (&pair[0], &pair[1]);
                let ordered = if desc {
                    value(a) >= value(b)
                } else {
                    value(a) <= value(b)
                };
                assert!(
                    ordered,
                    "{sort:?} desc={desc}: {} then {}",
                    value(a),
                    value(b)
                );
                if value(a) == value(b) {
                    assert!(
                        (a.bar.ts_micros, a.series) < (b.bar.ts_micros, b.series),
                        "ties keep time, then series order"
                    );
                }
            }
            for offset in [0_usize, 50, 100, 270, 319, 320] {
                asked.offset = Some(offset);
                asked.limit = 50;
                let page = store.window(&asked);
                let want: Vec<_> = everything
                    .bars
                    .iter()
                    .skip(offset)
                    .take(50)
                    .map(|row| (row.bar.ts_micros, row.series))
                    .collect();
                let got: Vec<_> = page
                    .bars
                    .iter()
                    .map(|row| (row.bar.ts_micros, row.series))
                    .collect();
                assert_eq!(got, want, "{sort:?} desc={desc} offset={offset}");
                assert_eq!(page.total, 320);
            }
        }
    }
}

/// **`at=` IS THE GLOBAL INDEX OF AN INSTANT, IN THE REQUEST'S DIRECTION,
/// AND WITHOUT AN OFFSET THE PAGE THAT HOLDS IT IS SERVED.** DB-005, DB-G16:
/// Go to computed its page from the first FILE's offset, not the row's, and
/// landed some 17,450 bars early.
#[test]
fn at_names_the_global_index_of_an_instant_in_both_directions() {
    let (store, truth) = pair("at-index");
    let instants = [
        ist(2019, 2, 27, 9, 15, 0),
        ist(2019, 2, 28, 9, 15, 15),
        ist(2019, 2, 28, 9, 15, 16),
        ist(2019, 3, 19, 9, 15, 30),
        ist(2019, 3, 20, 9, 18, 0),
        ist(2019, 3, 21, 0, 0, 0),
        ist(2019, 1, 1, 0, 0, 0),
    ];
    for at in instants {
        for desc in [false, true] {
            let mut asked = query(
                &[CE, PE],
                Timeframe::SECOND_1,
                month(2019, 2),
                month(2019, 3),
            );
            asked.desc = desc;
            asked.at = Some(at);
            asked.offset = None;
            asked.limit = 50;
            let got = store.window(&asked);
            let want = if desc {
                truth.iter().filter(|row| row.0 > at).count()
            } else {
                truth.iter().filter(|row| row.0 < at).count()
            };
            assert_eq!(
                got.at_index,
                Some(u64::try_from(want).expect("fits")),
                "at={at} desc={desc}"
            );
            let page_start = want.min(truth.len() - 1) / 50 * 50;
            assert_eq!(
                got.offset, page_start,
                "the page holding the index is served"
            );
            if want < truth.len() {
                let mut order = truth.clone();
                if desc {
                    order.reverse();
                }
                let row = &got.bars[want - page_start];
                assert_eq!(
                    (row.bar.ts_micros, row.series),
                    (order[want].0, order[want].1)
                );
                assert!(if desc {
                    row.bar.ts_micros <= at
                } else {
                    row.bar.ts_micros >= at
                });
            }
        }
    }
}

/// **A TIME-OF-DAY BOUND APPLIES TO EVERY MONTH, AND THE TOTAL IS THE
/// FILTERED TOTAL.** DB-004, DB-G91: the bound filtered only the newest
/// month's file and the pager counted that subset. A day bound trims the
/// edge months. Both, together and apart, against a brute force.
#[test]
fn day_and_time_of_day_bounds_filter_every_month_on_the_server() {
    type Case = (Option<i64>, Option<i64>, Option<(u32, u32)>);
    let (store, truth) = pair("bounds");
    let tod = |ts: i64| (ts / 1_000_000 + 19_800).rem_euclid(86_400);
    let cases: [Case; 5] = [
        (
            None,
            None,
            Some((9 * 3_600 + 15 * 60 + 10, 9 * 3_600 + 15 * 60 + 40)),
        ),
        (
            Some(ist(2019, 2, 28, 0, 0, 0)),
            Some(ist(2019, 3, 20, 0, 0, 0)),
            None,
        ),
        (
            Some(ist(2019, 2, 28, 0, 0, 0)),
            Some(ist(2019, 3, 20, 0, 0, 0)),
            Some((33_310, 33_320)),
        ),
        (None, None, Some((0, 33_299))),
        (
            Some(ist(2019, 3, 20, 0, 0, 0)),
            Some(ist(2019, 3, 21, 0, 0, 0)),
            Some((33_300, 86_399)),
        ),
    ];
    for (from, to, window) in cases {
        let kept: Vec<_> = truth
            .iter()
            .copied()
            .filter(|row| from.is_none_or(|at| row.0 >= at) && to.is_none_or(|at| row.0 < at))
            .filter(|row| {
                window.is_none_or(|(lo, hi)| (i64::from(lo)..=i64::from(hi)).contains(&tod(row.0)))
            })
            .collect();
        for desc in [false, true] {
            let mut asked = query(
                &[CE, PE],
                Timeframe::SECOND_1,
                month(2019, 2),
                month(2019, 3),
            );
            asked.filter = bars::Filter {
                from,
                to,
                tod: window,
            };
            asked.desc = desc;
            asked.limit = 13;
            let mut seen = Vec::new();
            let mut offset = 0;
            loop {
                asked.offset = Some(offset);
                let page = store.window(&asked);
                assert_eq!(
                    page.total,
                    u64::try_from(kept.len()).expect("fits"),
                    "{from:?} {to:?} {window:?}"
                );
                if page.bars.is_empty() {
                    break;
                }
                seen.extend(
                    page.bars
                        .iter()
                        .map(|row| (row.bar.ts_micros, row.series, row.bar.close)),
                );
                offset += 13;
            }
            let mut want = kept.clone();
            if desc {
                want.reverse();
            }
            assert_eq!(seen, want, "{from:?} {to:?} {window:?} desc={desc}");
        }
    }
}

/// **THE FIRST BAR OF A DAY WINDOW IS MEASURED AGAINST THE BAR BEFORE IT IN
/// THE STORE, EVEN OUTSIDE THE WINDOW, AND THE FIRST BAR OF A MONTH AGAINST
/// THE MONTH BEFORE, EVEN OUTSIDE THE RANGE.** DB-011, DB-G65, DB-G90: the
/// first bar of every month in a one-month request said `first_bar_in_file`,
/// and a day window's first row said its predecessor was in a month the
/// request did not read when it was in the same file. With `lookback=0` the
/// codes say which bound hid it.
#[test]
fn the_previous_stored_bar_is_read_across_the_day_and_month_bounds() {
    let (store, truth) = pair("lookback");
    let ce: Vec<_> = truth.iter().copied().filter(|row| row.1 == 0).collect();
    // The CE's first bar of 2019-03-20 and the last bar of 2019-03-19.
    let first_of_day = ce
        .iter()
        .position(|row| row.0 >= ist(2019, 3, 20, 0, 0, 0))
        .expect("held");
    let want =
        crate::server::basis_points(ce[first_of_day - 1].2, ce[first_of_day].2).expect("a ratio");
    let mut asked = query(&[CE], Timeframe::SECOND_1, month(2019, 3), month(2019, 3));
    asked.filter.from = Some(ist(2019, 3, 20, 0, 0, 0));
    asked.filter.to = Some(ist(2019, 3, 21, 0, 0, 0));
    asked.limit = 2;
    let got = store.window(&asked);
    assert_eq!(got.bars[0].bar.close, ce[first_of_day].2);
    assert_eq!(
        (got.bars[0].chg, got.bars[0].chg_why),
        (Some(want), ""),
        "the day before, same file"
    );
    asked.lookback = false;
    let off = store.window(&asked);
    assert_eq!(
        (off.bars[0].chg, off.bars[0].chg_why),
        (None, "first_bar_in_window")
    );
    assert!(
        off.bars[1].chg.is_some(),
        "a row whose predecessor is kept is measured"
    );

    // ONE MONTH ASKED, AND ITS FIRST BAR STANDS BEHIND FEBRUARY'S LAST.
    let first_of_march = ce
        .iter()
        .position(|row| row.0 >= ist(2019, 3, 1, 0, 0, 0))
        .expect("held");
    let mut march = query(&[CE], Timeframe::SECOND_1, month(2019, 3), month(2019, 3));
    march.limit = 1;
    let got = store.window(&march);
    assert_eq!(got.bars[0].bar.close, ce[first_of_march].2);
    // 2019-02-28 to 2019-03-19 skips the sessions between: an index
    // option's change is refused across them, by name.
    assert_eq!(got.bars[0].chg_why, "previous_session_not_adjacent");
    march.lookback = false;
    let off = store.window(&march);
    assert_eq!(off.bars[0].chg_why, "earlier_month_not_read");
    // AND THE SERIES' VERY FIRST BAR HAS NONE AT ALL.
    let mut february = query(&[CE], Timeframe::SECOND_1, month(2019, 2), month(2019, 2));
    february.limit = 1;
    assert_eq!(
        store.window(&february).bars[0].chg_why,
        "no_earlier_stored_bar"
    );
    // ADJACENT SESSIONS ACROSS A MONTH ARE MEASURED: 2019-02-27 then 02-28.
    let first_of_28 = ce
        .iter()
        .position(|row| row.0 >= ist(2019, 2, 28, 0, 0, 0))
        .expect("held");
    february.offset = Some(first_of_28);
    let row = store.window(&february).bars[0];
    assert_eq!(
        (row.chg, row.chg_why),
        (
            Some(
                crate::server::basis_points(ce[first_of_28 - 1].2, ce[first_of_28].2)
                    .expect("ratio")
            ),
            ""
        ),
        "an index option across adjacent sessions is a number"
    );
}

/// One bar per instant of `stamps`, closes from `close`, written to one
/// series; the window over it, time ordered, every row.
fn series_of(
    store: &Store,
    vendor: Vendor,
    key: &str,
    timeframe: Timeframe,
    stamps: &[i64],
    open_interest: i64,
) -> bars::Window {
    let parsed = bars::SeriesKey::parse(key).expect("key");
    let rows: Vec<Bar> = stamps
        .iter()
        .enumerate()
        .map(|(n, &ts)| {
            bar(
                ts,
                50_000 + i64::try_from(n).expect("small") * 10,
                100,
                open_interest,
            )
        })
        .collect();
    store.write(
        vendor,
        &parsed.segment,
        &parsed.symbol,
        parsed
            .contract
            .as_ref()
            .map(brutex_core::instrument::Contract::as_str),
        timeframe,
        &rows,
    );
    let mut asked = query(
        &[key],
        timeframe,
        month_of(stamps[0]),
        month_of(*stamps.last().expect("one")),
    );
    asked.vendor = vendor;
    store.window(&asked)
}

/// The change codes of a window, oldest first.
fn codes(window: &bars::Window) -> Vec<&'static str> {
    window.bars.iter().map(|row| row.chg_why).collect()
}

/// **A SHARE'S FIRST BAR OF A SESSION IS REFUSED, ON EVERY FEED, AND A
/// STOCK OPTION'S; AN INDEX OPTION'S IS NOT.** D-3325 rule 3; DB-010,
/// DB-032, DB-G43, DB-G74: GDFL BAJFINANCE served −89.76% at its split, and
/// Zerodha NMDC +85.71% at a demerger, with no reason.
#[test]
fn a_rebaseable_series_refuses_its_first_bar_of_each_session() {
    let store = Store::new("rebase");
    let days = [
        ist(2019, 3, 19, 15, 29, 59),
        ist(2019, 3, 20, 9, 15, 0),
        ist(2019, 3, 20, 9, 15, 1),
    ];
    for (vendor, key) in [
        (Vendor::Gdfl, "NSE-CASH-RELIANCE"),
        (Vendor::Gdfl, "NSE-FNO-RELIANCE-2019-03-28-130000-CE"),
        (Vendor::Gdfl, "NSE-FNO-NIFTYIT-2019-03-28-1500000-CE"),
    ] {
        let window = series_of(&store, vendor, key, Timeframe::SECOND_1, &days, 9_000);
        let index = key.contains("NIFTYIT");
        assert_eq!(
            codes(&window),
            if index {
                vec!["no_earlier_stored_bar", "", ""]
            } else {
                vec!["no_earlier_stored_bar", "corporate_action_unverified", ""]
            },
            "{key}"
        );
    }
    // ZERODHA'S DAY BARS: every bar opens a session, so every change of a
    // share is refused (D-3325: "CASH on every feed").
    let day_bars = [ist(2022, 10, 25, 9, 15, 0), ist(2022, 10, 27, 9, 15, 0)];
    let window = series_of(
        &store,
        Vendor::Zerodha,
        "NSE-CASH-NMDC",
        Timeframe::DAY_1,
        &day_bars,
        0,
    );
    assert_eq!(
        codes(&window),
        vec!["no_earlier_stored_bar", "corporate_action_unverified"]
    );
    // AND ITS STORED ZERO IS NOT AN OPEN INTEREST (D-3322).
    assert!(
        window
            .bars
            .iter()
            .all(|row| !row.oi_applies && row.oichg_why == "not_a_derivative")
    );
    assert!(
        window.bars.iter().all(|row| row.phase.is_none()),
        "a day bar has no phase"
    );
}

/// **AN INDEX'S CHANGE IS REFUSED ACROSS A MISSING SESSION AND A MISSING
/// MINUTE, ACROSS A SUSPECTED HALT, AND ACROSS A PHASE BOUNDARY.** D-3325
/// rules 4 to 7; DB-012 (Zerodha NIFTY 1min measured +15.97% across a
/// two-month hole), DB-028 (the 2020-03-13 halt measured as trading).
#[test]
fn an_index_change_is_refused_across_holes_halts_and_phases() {
    let store = Store::new("index-rules");
    let stamps = [
        ist(2019, 3, 18, 15, 29, 0),
        // 2019-03-19 is missing: the session before 03-20 is not held.
        ist(2019, 3, 20, 9, 14, 0),
        ist(2019, 3, 20, 9, 15, 0),
        ist(2019, 3, 20, 9, 16, 0),
        ist(2019, 3, 20, 9, 18, 0),
        ist(2019, 3, 20, 15, 29, 0),
        ist(2019, 3, 22, 9, 15, 0),
    ];
    let window = series_of(
        &store,
        Vendor::Zerodha,
        "NSE-INDEX-NIFTY",
        Timeframe::MINUTE_1,
        &stamps,
        0,
    );
    assert_eq!(
        codes(&window),
        vec![
            "no_earlier_stored_bar",
            "previous_session_not_adjacent",
            "phase_boundary",
            "",
            "previous_bar_not_adjacent",
            "previous_bar_not_adjacent",
            // 2019-03-21 was a trading holiday in the sourced calendar, so
            // 03-22's session follows 03-20's.
            "",
        ]
    );
    assert_eq!(window.bars[1].phase, Some("pre_open"));
    assert_eq!(window.bars[2].phase, Some("continuous"));
    assert!(
        window
            .bars
            .iter()
            .all(|row| row.v_why == Some("index_no_volume"))
    );

    let halt = [
        ist(2020, 3, 13, 9, 20, 0),
        ist(2020, 3, 13, 9, 21, 0),
        ist(2020, 3, 13, 9, 22, 0),
        ist(2020, 3, 13, 10, 20, 0),
        ist(2020, 3, 13, 10, 21, 0),
        ist(2020, 3, 13, 10, 22, 0),
    ];
    let store = Store::new("index-halt");
    let window = series_of(
        &store,
        Vendor::Zerodha,
        "NSE-INDEX-NIFTY",
        Timeframe::MINUTE_1,
        &halt,
        0,
    );
    // A MINUTE BAR CLOSES AT THE END OF ITS MINUTE: the 09:21 bar closes in
    // the halt (the index froze at 09:21:10), the 10:21 bar after the 10:21:01
    // resume, so 10:22 is the first change measured again.
    assert_eq!(
        codes(&window),
        vec![
            "no_earlier_stored_bar",
            "halt_suspected",
            "halt_suspected",
            "halt_suspected",
            "halt_suspected",
            "",
        ]
    );
    assert_eq!(window.bars[2].phase, Some("halted"));
    assert_eq!(window.bars[3].phase, Some("reopening_auction"));
}

/// **A ONE-SECOND BAR'S PHASE IS ITS SECOND'S, NOT ITS MINUTE'S** (DB-B43,
/// WP-54). The index is halted from 09:21:10 on 2020-03-13 and resumes at
/// 10:21:01 (`pull::calendar::HALTS`): the 50 unchanged-price seconds after 09:21:09 and
/// the second 10:21:00 were served as `continuous` at minute resolution.
#[test]
fn a_one_second_bar_is_labelled_by_its_own_second_in_a_halt() {
    let store = Store::new("index-halt-1s");
    let stamps = [
        ist(2020, 3, 13, 9, 21, 9),
        ist(2020, 3, 13, 9, 21, 10),
        ist(2020, 3, 13, 9, 21, 59),
        ist(2020, 3, 13, 10, 10, 0),
        ist(2020, 3, 13, 10, 21, 0),
        ist(2020, 3, 13, 10, 21, 1),
    ];
    let window = series_of(
        &store,
        Vendor::Gdfl,
        "NSE-INDEX-NIFTY",
        Timeframe::SECOND_1,
        &stamps,
        0,
    );
    assert_eq!(
        window.bars.iter().map(|row| row.phase).collect::<Vec<_>>(),
        vec![
            Some("continuous"),
            Some("halted"),
            Some("halted"),
            Some("reopening_auction"),
            Some("halted"),
            Some("continuous"),
        ]
    );
}

/// **ACROSS TWO ADJACENT SESSIONS, AN INDEX'S MINUTE IS MEASURED ONLY FROM
/// THE LAST BUCKET OF ONE TO THE FIRST BUCKET OF THE NEXT.** D-3325 rule 7: a
/// session whose minute file stops early, or a session whose first minutes
/// are missing, would otherwise read an "overnight" change from a bar that is
/// not the previous bucket. Each edge is checked apart, and the ordinary
/// 15:29 -> 09:15 step is still a number.
#[test]
fn an_index_change_across_sessions_needs_the_closing_and_opening_buckets() {
    let store = Store::new("index-edges");
    let stamps = [
        // 2019-03-18 stops at 15:00: 15:01..15:29 are continuous and absent.
        ist(2019, 3, 18, 15, 0, 0),
        ist(2019, 3, 19, 9, 15, 0),
        ist(2019, 3, 19, 15, 29, 0),
        // 2019-03-20 opens at 09:20: 09:15..09:19 are absent.
        ist(2019, 3, 20, 9, 20, 0),
        ist(2019, 3, 20, 9, 21, 0),
        ist(2019, 3, 20, 15, 29, 0),
        // 2019-03-21 is a trading holiday; 03-22 opens at 09:15.
        ist(2019, 3, 22, 9, 15, 0),
    ];
    let window = series_of(
        &store,
        Vendor::Zerodha,
        "NSE-INDEX-NIFTY",
        Timeframe::MINUTE_1,
        &stamps,
        0,
    );
    assert_eq!(
        codes(&window),
        vec![
            "no_earlier_stored_bar",
            "previous_bar_not_adjacent",
            "previous_bar_not_adjacent",
            "previous_bar_not_adjacent",
            "",
            "previous_bar_not_adjacent",
            "",
        ]
    );
    // A FIVE-MINUTE RUNG'S LAST BUCKET IS 15:25 AND ITS FIRST 09:15.
    let store = Store::new("index-edges-5min");
    let five = [
        ist(2019, 3, 19, 15, 25, 0),
        ist(2019, 3, 20, 9, 15, 0),
        ist(2019, 3, 20, 15, 20, 0),
        ist(2019, 3, 22, 9, 20, 0),
    ];
    let window = series_of(
        &store,
        Vendor::Zerodha,
        "NSE-INDEX-NIFTY",
        Timeframe::MINUTE_5,
        &five,
        0,
    );
    assert_eq!(
        codes(&window),
        vec![
            "no_earlier_stored_bar",
            "",
            "previous_bar_not_adjacent",
            "previous_bar_not_adjacent",
        ]
    );
    // AN INDEX OPTION IS NOT HELD TO IT: a contract that did not trade in a
    // minute has no bar there, and its change is the move since it last did.
    let store = Store::new("option-edges");
    let window = series_of(
        &store,
        Vendor::Zerodha,
        "NSE-FNO-NIFTY-2019-03-28-1150000-CE",
        Timeframe::MINUTE_1,
        &stamps,
        9_000,
    );
    assert_eq!(
        codes(&window),
        vec!["no_earlier_stored_bar", "", "", "", "", "", ""]
    );
}

/// **A CONTRACT RENAMED AT AN EX-DATE IS NAMED AT ITS FIRST BAR AFTER, AND
/// THE VENDOR'S UNSCALED OPEN INTEREST UNTIL IT LANDED.** D-3331 decision 4;
/// DB-G71 (NTPC's post-ex 125PE read +900% against the old 150PE), DB-G77
/// (BAJFINANCE's OI carried unscaled for hours). The ledger is read from
/// `<store>/imports/gdfl.lineage`; a line that does not parse is a fault.
#[test]
fn a_lineage_ex_date_names_the_contract_adjusted_and_the_unscaled_open_interest() {
    let store = Store::new("lineage");
    let until = ist(2019, 3, 19, 10, 0, 0) / 1_000_000;
    fs::create_dir_all(store.root.join("imports")).expect("imports");
    fs::write(
        store.root.join("imports").join("gdfl.lineage"),
        format!(
            "alias NTPC stated=2019-03-28 settled=2019-03-28 kind=closed_day first_day=2019-03-01 contracts=1 v=1\n\
             lineage NSE-FNO-NTPC-2019-03-28-15000-PE NSE-FNO-NTPC-2019-03-28-12500-PE ex=2019-03-19 factor=6/5 lot_old=4000 lot_new=4800 oi_unadjusted_until={until} v=1\n"
        ),
    )
    .expect("an invented ledger");
    let stamps = [
        ist(2019, 3, 18, 15, 25, 0),
        ist(2019, 3, 19, 9, 15, 25),
        ist(2019, 3, 19, 9, 15, 30),
        ist(2019, 3, 19, 10, 0, 1),
    ];
    let window = series_of(
        &store,
        Vendor::Gdfl,
        "NSE-FNO-NTPC-2019-03-28-12500-PE",
        Timeframe::SECOND_1,
        &stamps,
        52_000,
    );
    assert_eq!(
        codes(&window),
        vec!["no_earlier_stored_bar", "contract_adjusted", "", ""]
    );
    let oi: Vec<_> = window.bars.iter().map(|row| row.oichg_why).collect();
    assert_eq!(
        oi,
        vec![
            "no_earlier_stored_bar",
            "contract_adjusted",
            "vendor_oi_unadjusted",
            ""
        ]
    );
    assert!(window.faults.is_empty(), "{:?}", window.faults);
    let (events, unreadable) = bars::test_access::lineage_ledger_counts(
        "lineage nonsense\nstitched NSE-FNO-NTPC-2019-03-28-15000-PE ex=2019-03-19 before=- after=- v=1\n",
    );
    assert_eq!(
        unreadable, 1,
        "a line that does not parse is counted, not applied"
    );
    assert_eq!(events, 1, "a stitched key labels its first post-ex bar");
    // AN UNREADABLE LINE IN THE LEDGER IS A NAMED FAULT ON THE ANSWER.
    fs::write(
        store.root.join("imports").join("gdfl.lineage"),
        "lineage nonsense\n",
    )
    .expect("a damaged ledger");
    let mut asked = query(
        &["NSE-FNO-NTPC-2019-03-28-12500-PE"],
        Timeframe::SECOND_1,
        month(2019, 3),
        month(2019, 3),
    );
    asked.limit = 1;
    let faulted = store.window(&asked);
    assert!(
        faulted
            .faults
            .iter()
            .any(|fault| fault.contains("do not parse as a lineage record")),
        "{:?}",
        faulted.faults
    );
}

/// **THE EXTREMES SAY WHERE, ACROSS EVERY SERIES, AND A TIE KEEPS THE
/// EARLIER BAR.** DB-G95: only the widest range and heaviest volume were
/// returned, with no instant and no series. Folded on the scan path and on
/// the time path (from the per-month memo the second time), equal.
#[test]
fn the_extremes_name_their_instant_and_series_and_agree_on_both_paths() {
    let (store, truth) = pair("extremes");
    let mut asked = query(
        &[CE, PE],
        Timeframe::SECOND_1,
        month(2019, 2),
        month(2019, 3),
    );
    asked.want_extremes = true;
    asked.limit = 5;
    let seek = store.window(&asked);
    let again = store.window(&asked);
    asked.sort = bars::SortKey::Close;
    let scan = store.window(&asked);
    let top = seek.extremes.expect("asked for");
    assert_eq!(
        Some(top),
        again.extremes,
        "the memo answers what the read did"
    );
    assert_eq!(again.scan_rows, 0, "the second fold reads nothing");
    assert_eq!(Some(top), scan.extremes, "both paths fold the same rows");
    let max = truth.iter().map(|row| row.2).max().expect("rows");
    let first_max = truth.iter().find(|row| row.2 == max).expect("held");
    let close_max = top.close_max.expect("rows exist");
    assert_eq!(
        (close_max.v, close_max.t, close_max.series),
        (max, first_max.0, first_max.1)
    );
    let min = truth.iter().map(|row| row.2).min().expect("rows");
    let first_min = truth.iter().find(|row| row.2 == min).expect("held");
    let close_min = top.close_min.expect("rows");
    assert_eq!(
        (close_min.v, close_min.t, close_min.series),
        (min, first_min.0, first_min.1)
    );
    assert_eq!(top.range, 10);
    assert_eq!(top.range_max.map(|e| e.v), Some(10));
    assert_eq!(top.volume, 675);
    assert_eq!(top.volume_max.map(|e| e.v), Some(675));
    assert_eq!(top.high_max.map(|e| e.v), Some(max + 5));
    assert_eq!(top.low_min.map(|e| e.v), Some(min - 5));
}

/// **MISSING MONTHS ARE COUNTED FROM THE DIRECTORY, HOLES NAMED, AND THE
/// DAYS EACH HELD MONTH COVERS REPORTED.** DB-065, DB-023, DB-G35: every
/// month of the range was opened and an absent one logged a warning per
/// request; derived months stopped early with `months_missing=0`.
#[test]
fn missing_months_are_counted_holes_named_and_days_covered_reported() {
    let store = Store::new("coverage");
    let stamps = [
        ist(2019, 1, 2, 9, 15, 0),
        ist(2019, 1, 3, 9, 15, 0),
        ist(2019, 3, 5, 9, 15, 0),
        ist(2019, 3, 5, 9, 16, 0),
        ist(2019, 3, 6, 9, 15, 0),
        ist(2019, 3, 7, 9, 15, 0),
    ];
    let _ = series_of(
        &store,
        Vendor::Zerodha,
        "NSE-INDEX-NIFTYIT",
        Timeframe::MINUTE_1,
        &stamps,
        0,
    );
    let mut asked = query(
        &["NSE-INDEX-NIFTYIT"],
        Timeframe::MINUTE_1,
        month(2018, 11),
        month(2019, 5),
    );
    asked.vendor = Vendor::Zerodha;
    asked.coverage = true;
    let got = store.window(&asked);
    assert_eq!(got.months_read, 2);
    assert_eq!(got.months_missing, 5);
    // AN INDEX NEVER EXPIRES, so a missing month after its newest file, inside
    // Zerodha's span (the operator's window, 2018-01 to 2026-09, D-3338), is a
    // hole the feed owes — not yet pulled — as much as one between files
    // (DB-B47). Before its first file it may not have been launched, so those
    // stay expected absences.
    assert_eq!(
        got.missing_months,
        ["2019-02", "2019-04", "2019-05"].map(str::to_owned),
        "every hole after the index's first file"
    );
    assert_eq!(got.months_outside_life, 2, "2018-11 and 2018-12");
    let covered: Vec<_> = got
        .coverage
        .iter()
        .map(|m| (m.month.to_string(), m.rows, m.days_covered))
        .collect();
    assert_eq!(
        covered,
        vec![("2019-01".to_owned(), 2, 2), ("2019-03".to_owned(), 4, 3)]
    );
}

/// **THE ROUTE READS THE NEW PARAMETERS, AND REFUSES EACH WRONG ONE BY
/// NAME.** D-3326.
#[tokio::test]
async fn the_route_reads_keys_at_day_and_time_bounds_and_refuses_each_by_name() {
    let (store, truth) = pair("route-new");
    let site = store.site();
    let base = "feed=gdfl&timeframe=1s&from=2019-02&to=2019-03";
    let (status, body) =
        window_get(&site, &format!("{base}&key={CE}&key={PE}&dir=asc&limit=3")).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["total"], 320);
    assert_eq!(body["series_totals"][1]["key"], PE);
    assert_eq!(
        body["bars"][0]["key"],
        if truth[0].1 == 0 { CE } else { PE }
    );
    assert_eq!(body["bars"][0]["month"], "2019-02");
    let at = ist(2019, 3, 19, 9, 15, 30) / 1_000_000;
    let (_, found) = window_get(
        &site,
        &format!("{base}&key={CE}&key={PE}&dir=asc&limit=10&at={at}"),
    )
    .await;
    let want = truth.iter().filter(|row| row.0 < at * 1_000_000).count();
    assert_eq!(found["at_index"], want);
    assert_eq!(found["offset"], want / 10 * 10);
    let (_, timed) = window_get(
        &site,
        &format!("{base}&key={CE}&key={PE}&tod_from=09:15:10&tod_to=09:15:40&from_day=2019-02-28&to_day=2019-03-19"),
    )
    .await;
    let tod = |ts: i64| (ts / 1_000_000 + 19_800).rem_euclid(86_400);
    let kept = truth
        .iter()
        .filter(|row| row.0 >= ist(2019, 2, 28, 0, 0, 0) && row.0 < ist(2019, 3, 20, 0, 0, 0))
        .filter(|row| (33_310..=33_340).contains(&tod(row.0)))
        .count();
    assert_eq!(timed["total"], kept);
    for (extra, named) in [
        (format!("&key={CE}&symbol=NIFTY"), "not both"),
        (format!("&key={CE}&key={CE}"), "named twice"),
        ("&key=NOTAKEY".to_owned(), "not a census instrument key"),
        (format!("&key={CE}&sort=c&at=1"), "needs sort=ts"),
        (format!("&key={CE}&at=soon"), "unix seconds"),
        (
            format!("&key={CE}&tod_from=15:00:00&tod_to=09:15:00"),
            "across midnight",
        ),
        (format!("&key={CE}&tod_from=9:15"), "tod_from"),
        (format!("&key={CE}&tod_to=24:00:00"), "tod_to"),
        (
            format!("&key={CE}&from_day=2019-03-20&to_day=2019-03-19"),
            "from_day",
        ),
        (format!("&key={CE}&lookback=maybe"), "lookback flag"),
        (format!("&key={CE}&coverage=2"), "coverage flag"),
        // A DEEP PAGE OF A TIME-OF-DAY ORDER, which streams every kept row,
        // is refused by name (D-3326). A deep VALUE page is served: from the
        // sort indexes, or, under a time-of-day bound, from a read order held
        // whole (D-3349, below).
        (format!("&key={CE}&sort=tod&offset=100000"), "sort_too_deep"),
    ] {
        let (status, refused) = window_get(&site, &format!("{base}{extra}")).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{extra}: {refused}");
        assert!(
            refused["error"]
                .as_str()
                .is_some_and(|why| why.contains(named)),
            "{extra}: {refused}"
        );
    }
    let (status, deep) = window_get(&site, &format!("{base}&key={CE}&sort=c&offset=100000")).await;
    assert_eq!(status, StatusCode::OK, "{deep}");
    assert_eq!(deep["bars"], serde_json::json!([]));
    assert_eq!(deep["sort_index"]["scanned_rows"], 0, "{deep}");
    let (status, deep) = window_get(
        &site,
        &format!("{base}&key={CE}&sort=c&offset=100000&tod_from=09:15:00&tod_to=15:30:00"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{deep}");
    assert_eq!(deep["bars"], serde_json::json!([]));
    assert!(
        deep["sort_index"]["scanned_why"][0]
            .as_str()
            .is_some_and(|why| why.contains("time-of-day"))
    );
    let many: String = (0..=bars::MAX_SERIES).fold(String::new(), |mut keys, n| {
        use std::fmt::Write as _;
        let _ = write!(
            keys,
            "&key=NSE-FNO-NIFTY-2019-03-28-{}-CE",
            1_000_000 + n * 5_000
        );
        keys
    });
    let (status, refused) = window_get(&site, &format!("{base}{many}")).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(
        refused["error"]
            .as_str()
            .is_some_and(|why| why.starts_with("too_many_series"))
    );
}

/// **`/bars.json` SERVES AN INDEX'S AND A SHARE'S OPEN INTEREST AS NOT
/// APPLICABLE, AND A DERIVATIVE'S UNCHANGED.** D-3322.
#[tokio::test]
async fn the_month_route_serves_open_interest_only_where_it_applies() {
    let store = Store::new("month-oi");
    let stamps = [ist(2019, 3, 20, 9, 15, 0), ist(2019, 3, 20, 9, 16, 0)];
    let _ = series_of(
        &store,
        Vendor::Zerodha,
        "NSE-CASH-RELIANCE",
        Timeframe::MINUTE_1,
        &stamps,
        0,
    );
    let _ = series_of(
        &store,
        Vendor::Zerodha,
        "NSE-FNO-NIFTY-2019-03-28-1150000-CE",
        Timeframe::MINUTE_1,
        &stamps,
        0,
    );
    let site = store.site();
    let (_, cash) = month_get(
        &site,
        "feed=zerodha&exchange=NSE&segment=CASH&symbol=RELIANCE&timeframe=1min&month=2019-03",
    )
    .await;
    assert!(cash[0]["oi"].is_null());
    assert_eq!(cash[0]["oichg_why"], "not_a_derivative");
    assert!(
        cash[0].get("v_why").is_none(),
        "a share's volume is traded volume"
    );
    let (_, option) = month_get(
        &site,
        "feed=zerodha&exchange=NSE&segment=FNO&symbol=NIFTY&contract=2019-03-28-1150000-CE&timeframe=1min&month=2019-03",
    )
    .await;
    assert_eq!(option[0]["oi"], 0, "a derivative's zero is a measured zero");
    assert!(
        option[0].get("oichg_why").is_none(),
        "and its row is unchanged"
    );
}

/// **`/bars/window.json` MAY REPEAT `key`, AND ONLY `key`, AND ITS TARGET MAY
/// CARRY A THOUSAND OF THEM.** D-3326: one request names every series it
/// merges. The server-wide guard refused any repeated query field, and capped
/// every target at 8 KiB, which holds about 190 option keys.
#[test]
fn the_window_route_alone_may_repeat_key_and_carry_a_thousand_of_them() {
    let headers = axum::http::HeaderMap::new();
    let refused = |target: &str| {
        let uri: axum::http::Uri = target.parse().expect("a target");
        super::request_bounds_refusal(&uri, &headers)
    };
    let keys: String = (0..bars::MAX_SERIES).fold(String::new(), |mut keys, n| {
        use std::fmt::Write as _;
        let _ = write!(
            keys,
            "&key=NSE-FNO-NIFTY-2019-03-28-{}-CE",
            1_000_000 + n * 5_000
        );
        keys
    });
    let window = format!("/bars/window.json?feed=gdfl{keys}");
    assert!(window.len() > super::MAX_REQUEST_TARGET_BYTES);
    assert!(window.len() <= super::MAX_WINDOW_TARGET_BYTES);
    assert_eq!(refused(&window), None, "a thousand keys are one request");
    // ANY OTHER FIELD OF THE WINDOW STILL REFUSES A REPEAT.
    let (code, why) =
        refused("/bars/window.json?feed=gdfl&key=A&key=B&feed=zerodha").expect("a repeated feed");
    assert_eq!(code, StatusCode::BAD_REQUEST);
    assert!(why.contains("\"feed\""), "{why}");
    // AND ANY OTHER ROUTE STILL REFUSES A REPEATED `key`, AND ITS LONG TARGET.
    assert!(refused("/bars.json?key=A&key=B").is_some());
    let long = format!("/bars.json?feed=gdfl{keys}");
    let (code, _) = refused(&long).expect("past 8 KiB");
    assert_eq!(code, StatusCode::URI_TOO_LONG);
    let past = format!(
        "/bars/window.json?p={}",
        "a".repeat(super::MAX_WINDOW_TARGET_BYTES)
    );
    assert_eq!(
        refused(&past).map(|(code, _)| code),
        Some(StatusCode::URI_TOO_LONG)
    );
}

// ---------------------------------------------------------------------------
// D-3331, WP-12c (DB-A17, DB-A18): what a reader says of a stored option key
// the current decoder files under another key, or refuses. Invented bars and
// an invented ledger; the keys are the shapes the assurance measured.
// ---------------------------------------------------------------------------

const STATED: &str = "NSE-FNO-NIFTY-2023-06-29-1000000-PE";
const SETTLED: &str = "NSE-FNO-NIFTY-2023-06-28-1000000-PE";
const IMPOSSIBLE: &str = "NSE-FNO-NIFTY-2020-06-25-191250000-CE";

/// The re-key's two records for the three keys above, as `cli gdfl-rekey
/// --apply` writes them.
fn rekey_ledger() -> String {
    let nifty = brutex_core::symbol::Symbol::new("NIFTY").expect("symbol");
    let contract = |key: &str| {
        pull::gdfl_lineage::parse_option_key(key)
            .expect("an option key")
            .1
    };
    let superseded = pull::gdfl_lineage::SupersededRecord {
        underlying: nifty,
        old: contract(STATED),
        new: contract(SETTLED),
        kind: pull::expiry_alias::AliasKind::VendorRedated,
        months: 1,
        bars: 2,
        digest: [3; 32],
    };
    let retired = pull::gdfl_lineage::RetiredRecord {
        underlying: nifty,
        key: contract(IMPOSSIBLE),
        why: pull::gdfl_lineage::RetiredWhy::StrikeImplausible,
        day: Day::new(2019, 5, 30).expect("day"),
        median: Some(1_200_000),
        days: 1,
        bars: 1,
    };
    format!("{}\n{}\n", superseded.line(), retired.line())
}

/// The three keys written as invented one-second bars: the impossible
/// strike on 2019-05-30, and the stated and the settled put on 2020-06-15.
fn rekey_store(name: &str) -> Store {
    let store = Store::new(name);
    let on = |y, m, d, s| ist(y, m, d, 9, 15, s);
    for (key, stamps) in [
        (IMPOSSIBLE, vec![on(2019, 5, 30, 1)]),
        (STATED, vec![on(2020, 6, 15, 1), on(2020, 6, 15, 2)]),
        (SETTLED, vec![on(2020, 6, 15, 1), on(2020, 6, 15, 2)]),
    ] {
        let parsed = bars::SeriesKey::parse(key).expect("key");
        let rows: Vec<Bar> = stamps.iter().map(|&ts| bar(ts, 77, 3, 30)).collect();
        store.write(
            Vendor::Gdfl,
            &parsed.segment,
            &parsed.symbol,
            parsed
                .contract
                .as_ref()
                .map(brutex_core::instrument::Contract::as_str),
            Timeframe::SECOND_1,
            &rows,
        );
    }
    store
}

/// The re-key's ledger lines written into `store`.
fn record_rekey(store: &Store) {
    fs::create_dir_all(store.root.join("imports")).expect("imports");
    fs::write(
        store.root.join("imports").join("gdfl.lineage"),
        rekey_ledger(),
    )
    .expect("an invented ledger");
}

/// **A KEY THE DECODER REFUSES IS REFUSED BY NAME, AND A KEY IT FILES
/// ELSEWHERE IS SERVED WITH THE KEY IT CONTINUES AS.** Before the re-key ran,
/// the impossible strike was served like any contract (DB-A18) and the
/// stated-expiry key carried no word of its decoded key (DB-A17). Before the
/// ledger records it, the window still says the decoder disagrees
/// (`recorded: false`); after, it names the recorded successor, and the
/// retired key is refused by the window and by `bars::open_classified`, the
/// one door every month read takes.
#[test]
fn a_retired_key_is_refused_by_name_and_a_superseded_key_names_its_successor() {
    let store = rekey_store("rekey-window");
    let window = |key: &str, from: YearMonth| {
        bars::window(&store.root, &query(&[key], Timeframe::SECOND_1, from, from))
    };
    // BEFORE: the impossible strike is served; the stated key is served and
    // the decoder's disagreement is said, unrecorded.
    assert_eq!(window(IMPOSSIBLE, month(2019, 5)).expect("served").total, 1);
    let pending = window(STATED, month(2020, 6)).expect("served");
    assert_eq!(pending.total, 2);
    let expected = |recorded| {
        vec![bars::SupersededBy {
            key: STATED.to_owned(),
            by: SETTLED.to_owned(),
            kind: "vendor_redated",
            recorded,
        }]
    };
    assert_eq!(pending.superseded, expected(false));
    let settled = window(SETTLED, month(2020, 6)).expect("served");
    assert!(settled.superseded.is_empty());

    // AFTER the re-key's ledger lines.
    record_rekey(&store);
    let refused = window(IMPOSSIBLE, month(2019, 5)).expect_err("refused");
    assert!(
        refused.starts_with(
            "contract_refused: NSE-FNO-NIFTY-2020-06-25-191250000-CE is refused by the \
             current decoder (D-3331): strike implausible"
        ),
        "{refused}"
    );
    let recorded = window(STATED, month(2020, 6)).expect("still readable");
    assert_eq!(recorded.total, 2, "the old key's bars are kept and served");
    assert_eq!(recorded.superseded, expected(true));
    assert!(recorded.supersedes.is_empty(), "{:?}", recorded.supersedes);
    // THE KEY THE BARS LANDED IN NAMES WHERE THEY CAME FROM, with the
    // ledger's count and digest, so the copy is audited from either end.
    let landed = window(SETTLED, month(2020, 6)).expect("served");
    assert!(landed.superseded.is_empty(), "{:?}", landed.superseded);
    assert_eq!(
        landed.supersedes,
        vec![bars::Supersedes {
            key: SETTLED.to_owned(),
            of: STATED.to_owned(),
            kind: "vendor_redated",
            bars: 2,
            blake3: "03".repeat(32),
        }]
    );
    assert!(recorded.faults.is_empty(), "{:?}", recorded.faults);
    let parts = PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment: "FNO",
        symbol: "NIFTY",
        contract: brutex_core::instrument::Contract::parse("2020-06-25-191250000-CE"),
        timeframe: Timeframe::SECOND_1,
        month: month(2019, 5),
        file: FileKind::Bars,
    };
    let door = bars::open_classified(&store.root, parts).expect_err("refused at the door");
    assert!(
        door.message.starts_with("contract_refused:"),
        "{}",
        door.message
    );
}

/// The same through the routes: `410` and the refusal for the retired key on
/// `/bars/window.json` and `/bars.json`, and the pointer on the wire for the
/// superseded one (`null` for every other series).
#[tokio::test]
async fn the_routes_answer_a_retired_key_gone_and_carry_the_successor() {
    let store = rekey_store("rekey-routes");
    record_rekey(&store);
    let site = store.site();
    let (status, body) = window_get(
        &site,
        &format!("feed=gdfl&timeframe=1s&key={IMPOSSIBLE}&from=2019-05&to=2019-05"),
    )
    .await;
    assert_eq!(status, StatusCode::GONE, "{body}");
    assert!(
        body["error"]
            .as_str()
            .expect("why")
            .starts_with("contract_refused:"),
        "{body}"
    );
    let (status, body) = month_get(
        &site,
        "feed=gdfl&exchange=NSE&segment=FNO&symbol=NIFTY&contract=2020-06-25-191250000-CE&timeframe=1s&month=2019-05",
    )
    .await;
    assert_eq!(status, StatusCode::GONE, "{body}");
    // `/greeks.json` reads the contract through the same door, and answers
    // the same 410 with the same sentence, not a 400 (it answered cells
    // before the ledger named the key).
    let greeks = |root: &std::path::Path| {
        optiongreeks::respond(
            root,
            &format!(
                "feed=gdfl&exchange=NSE&segment=FNO&symbol=NIFTY&contract=2020-06-25-191250000-CE&timeframe=1s&month=2019-05&t={}",
                ist(2019, 5, 30, 9, 15, 1) / 1_000_000
            ),
            &optiongreeks::RateTable::from_var(None),
        )
    };
    let unrecorded = rekey_store("rekey-greeks-before");
    let (status, _, body) = greeks(&unrecorded.root);
    assert_eq!(status, StatusCode::OK, "before the ledger names it: {body}");
    let (status, _, body) = greeks(&store.root);
    assert_eq!(status, StatusCode::GONE, "{body}");
    let said: Value = serde_json::from_str(&body).expect("json");
    assert!(
        pull::gdfl_lineage::is_contract_refused(said["error"].as_str().expect("why")),
        "{body}"
    );
    let (status, body) = window_get(
        &site,
        &format!("feed=gdfl&timeframe=1s&key={STATED}&from=2020-06&to=2020-06"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["superseded_by"][0]["by"], SETTLED);
    assert_eq!(body["superseded_by"][0]["recorded"], true);
    assert!(body["supersedes"].is_null(), "{body}");
    let (status, body) = window_get(
        &site,
        &format!("feed=gdfl&timeframe=1s&key={SETTLED}&from=2020-06&to=2020-06"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(body["superseded_by"].is_null(), "{body}");
    assert_eq!(body["supersedes"][0]["of"], STATED, "{body}");
    assert_eq!(body["supersedes"][0]["bars"], 2, "{body}");
}

/// The re-key's records are read by the change rules too: they are not
/// counted as lines that do not parse, and a superseded key's lineage events
/// label the key it continues as.
#[test]
fn the_rekey_records_parse_and_carry_lineage_events_to_the_successor() {
    let lineage = format!(
        "lineage {STATED} NSE-FNO-NIFTY-2023-06-29-1100000-PE ex=2020-06-16 factor=2/1 lot_old=75 lot_new=150 oi_unadjusted_until=0 v=1\n"
    );
    let text = format!("{}{lineage}", rekey_ledger());
    let (events, unreadable) = bars::test_access::lineage_ledger_counts(&text);
    assert_eq!(unreadable, 0, "both re-key lines parse");
    assert_eq!(events, 1);
    let (said, _) = bars::test_access::lineage_ledger_status(&text, STATED);
    assert_eq!(said, format!("superseded by {SETTLED} (vendor_redated)"));
    let (said, _) = bars::test_access::lineage_ledger_status(&text, IMPOSSIBLE);
    assert!(said.starts_with("retired: contract_refused:"), "{said}");
    // An event labelling the OLD key also labels its successor.
    let carried = format!(
        "{}lineage NSE-FNO-NIFTY-2023-06-29-1100000-PE {STATED} ex=2020-06-16 factor=2/1 lot_old=75 lot_new=150 oi_unadjusted_until=0 v=1\n",
        rekey_ledger()
    );
    let (_, on_old) = bars::test_access::lineage_ledger_status(&carried, STATED);
    let (_, on_new) = bars::test_access::lineage_ledger_status(&carried, SETTLED);
    assert_eq!((on_old, on_new), (1, 1));
}

/// `/provenance.json` names the re-key on all three kinds of key: the copied
/// month says whose import days its reconstruction shows, the superseded one
/// says where its bars went, and the retired one quotes its refusal. A key
/// the ledger does not name gains no note.
#[test]
fn provenance_names_the_rekey_for_a_copied_a_superseded_and_a_retired_key() {
    let store = rekey_store("rekey-provenance");
    let notes = |key: &str, month: &str| {
        let asked = provenance::asked(&format!(
            "feed=gdfl&instrument={key}&timeframe=1s&month={month}"
        ))
        .expect("a provenance question");
        let answer = provenance::answer(&store.root, &asked);
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        answer.body["notes"]
            .as_array()
            .expect("notes")
            .iter()
            .map(|note| note.as_str().expect("a sentence").to_owned())
            .collect::<Vec<_>>()
    };
    // Before the ledger names them, none of the three gains a note.
    for (key, month) in [
        (SETTLED, "2020-06"),
        (STATED, "2020-06"),
        (IMPOSSIBLE, "2019-05"),
    ] {
        assert!(
            !notes(key, month)
                .iter()
                .any(|note| note.contains("gdfl-rekey")
                    || pull::gdfl_lineage::is_contract_refused(note)),
            "{key}"
        );
    }
    record_rekey(&store);
    let copied = notes(SETTLED, "2020-06");
    assert!(
        copied.iter().any(|note| note.starts_with(&format!(
            "the bars of {STATED} were copied into {SETTLED} by `cli gdfl-rekey`"
        )) && note.contains(&"03".repeat(32))),
        "{copied:?}"
    );
    let moved = notes(STATED, "2020-06");
    assert!(
        moved
            .iter()
            .any(|note| note.starts_with(&format!("{STATED} is superseded by {SETTLED}"))),
        "{moved:?}"
    );
    let retired = notes(IMPOSSIBLE, "2019-05");
    assert!(
        retired
            .iter()
            .any(|note| pull::gdfl_lineage::is_contract_refused(note)),
        "{retired:?}"
    );
}

/// An `event` and a `stopped` record of the lineage ledger (D-3355) are read
/// by the change rules and label no bar: before they were known here each
/// would have counted as a line that does not parse and raised the window's
/// ledger fault. Any other unknown line still does.
#[test]
fn the_event_and_stopped_records_parse_and_label_no_bar() {
    let text = "event NSE-FNO-NIFTY ex=2020-06-16 kind=relisted shift=0 lot_old=75 lot_new=75 paired=0 stopped=1 v=1\n\
                stopped NSE-FNO-NIFTY-2020-06-25-1000000-CE ex=2020-06-16 v=1\n";
    assert_eq!(bars::test_access::lineage_ledger_counts(text), (0, 0));
    let torn = format!("{text}event NSE-FNO-NIFTY ex=2020-06-16 kind=relisted\n");
    assert_eq!(bars::test_access::lineage_ledger_counts(&torn), (0, 1));
}

//! The All sources routes over generated stores (D-3339). No row here is
//! market data: every price is invented, on a level no NSE instrument named
//! here has traded at, so none can be mistaken for a vendor row.
#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::many_single_char_names,
    clippy::panic,
    clippy::too_many_lines,
    clippy::unwrap_used,
    reason = "a test that cannot panic cannot fail, and these lints exist to keep panics out of \
              the crate rather than out of its tests"
)]

use super::*;
use crate::server::Site;
use pull::manifest::{EntryKey, Manifest, manifest_path};
use serde_json::Value;
use std::fs;

/// A JSON number with exactly these digits: the routes write decimals to a
/// fixed number of places, and `arbitrary_precision` keeps them.
fn n(text: &str) -> Value {
    serde_json::from_str(text).expect("a number")
}

/// Micros since the epoch of `h:m:s` IST on `day`.
fn t(day: Day, h: i64, m: i64, s: i64) -> i64 {
    (i64::from(day.days_from_epoch()) * 86_400 - 19_800 + h * 3_600 + m * 60 + s) * 1_000_000
}

fn day(y: u16, m: u8, d: u8) -> Day {
    Day::new(y, m, d).expect("a real date")
}

/// One bar.
const fn bar(ts_micros: i64, (o, h, l, c): (i64, i64, i64, i64), volume: i64, oi: i64) -> Bar {
    Bar {
        ts_micros,
        open: o,
        high: h,
        low: l,
        close: c,
        volume,
        open_interest: oi,
    }
}

/// The two days the fixtures trade on: a Thursday and a Friday.
fn day_a() -> Day {
    day(2019, 3, 28)
}
fn day_b() -> Day {
    day(2019, 3, 29)
}
/// A day of the next month, so a derived rung can trail its minutes.
fn day_c() -> Day {
    day(2019, 4, 1)
}

/// An option contract of the index: strike ₹1,100.
fn option() -> Contract {
    Contract::parse("2019-03-28-110000-CE").expect("a contract")
}

struct Fixture {
    root: PathBuf,
    site: Loaded,
    /// How many manifest publications this fixture has stamped.
    stamps: std::cell::Cell<u64>,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

impl Fixture {
    fn empty(name: &str) -> Self {
        let root = crate::scratch::path(&format!("sources-{name}"));
        let _ = fs::remove_dir_all(&root);
        let masters = root.join("masters");
        fs::create_dir_all(&masters).expect("empty offline masters");
        let site = Loaded::new(Site::load(&masters, &root));
        Self {
            root,
            site,
            stamps: std::cell::Cell::new(0),
        }
    }

    /// Appends `rows` (ascending, one month) to one series' month file.
    fn bars(
        &self,
        vendor: Vendor,
        segment: &str,
        symbol: &str,
        contract: Option<Contract>,
        rung: Timeframe,
        rows: &[Bar],
    ) {
        let first = rows.first().expect("at least one bar").ts_micros;
        let civil =
            Day::from_days(u32::try_from(day_number(first)).expect("a day")).expect("civil");
        let month = YearMonth::new(civil.year(), civil.month()).expect("a month");
        let path = StorePath::new(PathParts {
            vendor,
            exchange: "NSE",
            segment,
            symbol,
            contract,
            timeframe: rung,
            month,
            file: FileKind::Bars,
        })
        .expect("fixture path");
        let hash = brutex_core::universe::fnv1a(symbol).to_le_bytes();
        let id = u32::from_le_bytes(hash[..4].try_into().expect("low32"));
        let mut file =
            store::file::BarFile::open_or_create(&self.root, path, id).expect("generated file");
        file.append(rows).expect("generated rows");
    }

    /// Records one month of one series in its vendor's census.
    fn census(
        &self,
        vendor: Vendor,
        segment: Segment,
        symbol: &str,
        rung: Timeframe,
        rows: &[Bar],
    ) {
        let first = rows.first().expect("rows").ts_micros;
        let last = rows.last().expect("rows").ts_micros;
        let civil =
            Day::from_days(u32::try_from(day_number(first)).expect("a day")).expect("civil");
        let path = manifest_path(&self.root, vendor);
        let bytes = fs::read(&path).unwrap_or_default();
        let mut manifest = Manifest::open_image(vendor, &bytes).expect("manifest");
        manifest
            .record(Entry {
                key: EntryKey {
                    contract: None,
                    exchange: Exchange::Nse,
                    segment,
                    symbol: Symbol::new(symbol).expect("symbol"),
                    timeframe: rung,
                    month: YearMonth::new(civil.year(), civil.month()).expect("month"),
                },
                rows: u64::try_from(rows.len()).expect("rows"),
                first_ts_micros: first,
                last_ts_micros: last,
            })
            .expect("one month in the census");
        fs::create_dir_all(path.parent().expect("parent")).expect("manifest dir");
        fs::write(&path, manifest.image()).expect("publish the census");
        self.stamps.set(self.stamps.get() + 1);
        fs::File::options()
            .write(true)
            .open(&path)
            .expect("the manifest")
            .set_modified(
                std::time::SystemTime::UNIX_EPOCH
                    + std::time::Duration::from_secs(1_700_000_000 + 60 * self.stamps.get()),
            )
            .expect("a filesystem that carries modified times");
    }

    /// Bars and their census row together.
    fn counted(
        &self,
        vendor: Vendor,
        segment: Segment,
        symbol: &str,
        rung: Timeframe,
        rows: &[Bar],
    ) {
        self.bars(vendor, segment.as_str(), symbol, None, rung, rows);
        self.census(vendor, segment, symbol, rung, rows);
    }

    /// NIFTY on both feeds over days A and B, a day C minute, a 2017 day bar
    /// outside the operator's window, a derived 5min rung that stops at
    /// 2019-03, and GDFL seconds the census does not count.
    fn nifty(name: &str) -> Self {
        let fx = Self::empty(name);
        let (a, b, c) = (day_a(), day_b(), day_c());
        let z = Vendor::Zerodha;
        let g = Vendor::Gdfl;
        let idx = Segment::Index;
        // ZERODHA MINUTES: three on A, one on B, one on C.
        fx.counted(
            z,
            idx,
            "NIFTY",
            Timeframe::MINUTE_1,
            &[
                bar(
                    t(a, 9, 15, 0),
                    (100_000, 100_200, 99_900, 100_100),
                    0,
                    OI_NULL,
                ),
                bar(
                    t(a, 9, 16, 0),
                    (100_100, 100_300, 100_000, 100_250),
                    0,
                    OI_NULL,
                ),
                bar(
                    t(a, 9, 17, 0),
                    (100_250, 100_260, 100_050, 100_150),
                    0,
                    OI_NULL,
                ),
                bar(
                    t(b, 9, 15, 0),
                    (101_000, 101_500, 100_800, 101_200),
                    0,
                    OI_NULL,
                ),
            ],
        );
        fx.counted(
            z,
            idx,
            "NIFTY",
            Timeframe::MINUTE_1,
            &[bar(
                t(c, 9, 15, 0),
                (102_000, 102_100, 101_900, 102_050),
                0,
                OI_NULL,
            )],
        );
        // ZERODHA DAYS: the official close of A is not its last minute.
        fx.counted(
            z,
            idx,
            "NIFTY",
            Timeframe::DAY_1,
            &[bar(
                t(day(2017, 12, 29), 0, 0, 0),
                (90_000, 90_100, 89_900, 90_050),
                0,
                OI_NULL,
            )],
        );
        fx.counted(
            z,
            idx,
            "NIFTY",
            Timeframe::DAY_1,
            &[
                bar(
                    t(a, 0, 0, 0),
                    (100_000, 100_300, 99_900, 100_200),
                    0,
                    OI_NULL,
                ),
                bar(
                    t(b, 0, 0, 0),
                    (101_000, 101_400, 100_800, 101_250),
                    0,
                    OI_NULL,
                ),
            ],
        );
        // A DERIVED RUNG, 2019-03 ONLY: it trails its minutes by C.
        fx.counted(
            z,
            idx,
            "NIFTY",
            Timeframe::MINUTE_5,
            &[
                bar(
                    t(a, 9, 15, 0),
                    (100_000, 100_300, 99_900, 100_150),
                    0,
                    OI_NULL,
                ),
                bar(
                    t(b, 9, 15, 0),
                    (101_000, 101_500, 100_800, 101_200),
                    0,
                    OI_NULL,
                ),
            ],
        );
        // GDFL SECONDS, NOT IN ITS CENSUS: a pre-open second, then seconds
        // that fold to Zerodha's minutes except 09:16's close.
        fx.bars(
            g,
            "INDEX",
            "NIFTY",
            None,
            Timeframe::SECOND_1,
            &[
                bar(
                    t(a, 9, 7, 59),
                    (100_000, 100_000, 100_000, 100_000),
                    0,
                    OI_NULL,
                ),
                bar(
                    t(a, 9, 15, 0),
                    (100_000, 100_100, 99_900, 100_050),
                    0,
                    OI_NULL,
                ),
                bar(
                    t(a, 9, 15, 30),
                    (100_050, 100_200, 100_000, 100_100),
                    0,
                    OI_NULL,
                ),
                bar(
                    t(a, 9, 16, 10),
                    (100_100, 100_300, 100_000, 100_300),
                    0,
                    OI_NULL,
                ),
                bar(
                    t(a, 9, 17, 5),
                    (100_250, 100_260, 100_050, 100_150),
                    0,
                    OI_NULL,
                ),
                bar(
                    t(b, 9, 15, 0),
                    (101_000, 101_400, 100_800, 101_250),
                    0,
                    OI_NULL,
                ),
            ],
        );
        fx
    }

    /// A share, `SYNTHA`, Zerodha at about half of GDFL on A (vendor
    /// adjusted, on a coarser grid, volume not scaled) and equal on B.
    fn share(name: &str) -> Self {
        let fx = Self::empty(name);
        let (a, b) = (day_a(), day_b());
        let (z, g, cash) = (Vendor::Zerodha, Vendor::Gdfl, Segment::Cash);
        fx.counted(
            z,
            cash,
            "SYNTHA",
            Timeframe::MINUTE_1,
            &[
                bar(
                    t(a, 9, 15, 0),
                    (50_000, 50_100, 49_900, 50_000),
                    1_000,
                    OI_NULL,
                ),
                bar(
                    t(a, 9, 16, 0),
                    (50_000, 50_200, 50_000, 50_100),
                    500,
                    OI_NULL,
                ),
                bar(
                    t(b, 9, 15, 0),
                    (60_000, 60_100, 59_900, 60_050),
                    700,
                    OI_NULL,
                ),
            ],
        );
        fx.counted(
            z,
            cash,
            "SYNTHA",
            Timeframe::DAY_1,
            &[
                bar(
                    t(a, 0, 0, 0),
                    (50_000, 50_200, 49_900, 50_100),
                    1_600,
                    OI_NULL,
                ),
                bar(
                    t(b, 0, 0, 0),
                    (60_000, 60_100, 59_900, 60_050),
                    700,
                    OI_NULL,
                ),
            ],
        );
        fx.counted(
            g,
            cash,
            "SYNTHA",
            Timeframe::SECOND_1,
            &[
                bar(
                    t(a, 9, 15, 0),
                    (100_010, 100_210, 99_810, 100_010),
                    1_000,
                    OI_NULL,
                ),
                bar(
                    t(a, 9, 16, 0),
                    (100_010, 100_410, 100_010, 100_210),
                    500,
                    OI_NULL,
                ),
                bar(
                    t(b, 9, 15, 0),
                    (60_000, 60_100, 59_900, 60_050),
                    700,
                    OI_NULL,
                ),
            ],
        );
        fx
    }

    async fn get(&self, route: &str, query: &str) -> (axum::http::StatusCode, Value) {
        let uri: axum::http::Uri = format!("{route}?{query}").parse().expect("URI");
        let state = axum::extract::State(Loaded::clone(&self.site));
        let (status, headers, body) = match route {
            "/sources.json" => sources_json(state, uri).await,
            "/sources/bars.json" => sources_bars_json(state, uri).await,
            "/sources/day.json" => sources_day_json(state, uri).await,
            other => panic!("not a route here: {other}"),
        };
        assert_eq!(headers[0].1, JSON);
        (status, serde_json::from_str(&body).expect("JSON"))
    }
}

/// The row of `rows` with this tag and rung.
fn row<'a>(rows: &'a Value, tag: &str, rung: &str) -> &'a Value {
    rows.as_array()
        .expect("rows")
        .iter()
        .find(|row| row["tag"] == tag && row["rung"] == rung)
        .unwrap_or_else(|| panic!("no {tag} {rung} in {rows:#}"))
}

const NIFTY: &str = "exchange=NSE&segment=INDEX&symbol=NIFTY";
const SHARE: &str = "exchange=NSE&segment=CASH&symbol=SYNTHA";

// ══ The rung table ═══════════════════════════════════════════════════════════

/// **EVERY RUNG EACH FEED HOLDS, TAGGED, WITH ITS SPAN, MONTHS, CENSUS ROWS,
/// BASIS AND STATE** (DB-079, DB-G99, D-3339 item 1): GDFL's seconds and the
/// nine rungs they fold to, Zerodha's stored minutes and days and its derived
/// 5min — the day rung's 2017 month counted outside the window, its holes
/// named, the derived rung partial behind its minutes, and GDFL's month the
/// census does not count said so.
#[tokio::test]
async fn the_rung_table_lists_every_feed_and_rung_with_its_basis_and_state() {
    let fx = Fixture::nifty("rungs");
    let (status, body) = fx.get("/sources.json", NIFTY).await;
    assert_eq!(status, axum::http::StatusCode::OK, "{body:#}");
    let rows = &body["rungs"];
    assert_eq!(body["instrument"]["key"], "NSE-INDEX-NIFTY");

    let seconds = row(rows, "gdfl_1s", "1s");
    assert_eq!(seconds["kind"], "stored");
    assert_eq!(seconds["first_day"], "2019-03-28");
    assert_eq!(seconds["last_day"], "2019-03-29");
    assert_eq!(seconds["months_held"], 1);
    assert_eq!(seconds["census"]["months_uncounted"], 1, "{seconds:#}");
    assert_eq!(seconds["census"]["uncounted"], json!(["2019-03"]));
    assert_eq!(seconds["state"], "uncounted");
    assert_eq!(seconds["basis"]["price"], "as_published");
    assert_eq!(seconds["basis"]["volume"], "none");
    assert_eq!(seconds["basis"]["close"], "last_print");
    assert_eq!(seconds["archive"]["from"], "2018-09-03");

    for rung in [
        "1min", "2min", "3min", "5min", "10min", "15min", "30min", "60min", "1day",
    ] {
        let fold = row(rows, "gdfl_fold", rung);
        assert_eq!(fold["stored"], false, "{fold:#}");
        assert_eq!(fold["computed"], "on_request");
        assert_eq!(fold["derived_from"], "1s");
        assert_eq!(
            fold["basis"]["close"], "last_print",
            "a fold closes on its last print"
        );
    }

    let minutes = row(rows, "zerodha_stored", "1min");
    assert_eq!(minutes["months_held"], 2);
    assert_eq!(minutes["census"]["rows"], 5);
    assert_eq!(minutes["census"]["months_counted"], 2);
    assert_eq!(minutes["last_day"], "2019-04-01");
    assert_eq!(minutes["basis"]["close"], "last_print");
    assert_eq!(minutes["state"], "held");
    // THE DAY-LEVEL HOLES ARE /calendar.json's, for the series and rung.
    let sessions = &minutes["sessions"];
    assert_eq!(
        sessions["calendar"], "/calendar.json?feed=zerodha&symbol=NIFTY&rung=1min",
        "{sessions:#}"
    );
    assert!(sessions["owed"].is_u64(), "{sessions:#}");
    assert_eq!(sessions["not_held"], 0, "{sessions:#}");
    assert_eq!(
        row(rows, "zerodha_derived", "5min")["sessions"]["calendar"],
        "/calendar.json?feed=zerodha&symbol=NIFTY&rung=5min",
        "a derived rung's sessions are its own calendar's, set against its minutes'"
    );

    let days = row(rows, "zerodha_stored", "1day");
    assert_eq!(days["basis"]["close"], "official", "D-3324");
    assert_eq!(days["basis"]["close_label"], "official close");
    assert_eq!(days["first_month"], "2017-12");
    assert_eq!(days["months_absent"], 14, "2018-01 to 2019-02");
    assert_eq!(days["holes"][0], "2018-01");
    assert_eq!(days["outside_window"]["months"], 1, "{days:#}");
    assert_eq!(days["outside_window"]["first"], "2017-12");
    assert_eq!(days["outside_window"]["label"], "outside your window");
    assert!(
        days["outside_window"]["db"]
            .as_str()
            .expect("a link")
            .ends_with("&oow=1")
    );

    let derived = row(rows, "zerodha_derived", "5min");
    assert_eq!(derived["derived_from"], "1min");
    assert_eq!(derived["state"], "partial", "{derived:#}");
    assert_eq!(derived["behind_source"]["months_missing"], 1);
    assert_eq!(
        derived["behind_source"]["months_missing_named"],
        json!(["2019-04"])
    );
    assert_eq!(derived["behind_source"]["source_last_day"], "2019-04-01");
    assert_eq!(derived["behind_source"]["last_day"], "2019-03-29");
    assert_eq!(derived["behind_source"]["days_behind"], 3);

    let feeds = body["feeds"].as_array().expect("feeds");
    assert_eq!(feeds.len(), 2, "Zerodha and GDFL only, D-3338: {feeds:#?}");
    let zerodha = feeds
        .iter()
        .find(|f| f["feed"] == "zerodha")
        .expect("zerodha");
    assert_eq!(zerodha["window"]["from"], "2018-01-01");
    assert_eq!(zerodha["window"]["label_outside"], "outside your window");
    assert_eq!(zerodha["census"]["state"], "held");
    assert_eq!(body["tolerances"]["price_bps"], 1);
}

/// A symbol nobody holds answers its empty table with the reason, and a
/// malformed question is refused by name, never answered for a default.
#[tokio::test]
async fn an_unheld_symbol_says_so_and_a_malformed_question_is_refused_by_name() {
    let fx = Fixture::nifty("refusals");
    let (status, body) = fx
        .get("/sources.json", "exchange=NSE&segment=INDEX&symbol=NOSUCH")
        .await;
    assert_eq!(status, axum::http::StatusCode::OK);
    assert_eq!(body["rungs"], json!([]));
    assert!(
        body["notes"][0]
            .as_str()
            .expect("a note")
            .contains("no offered feed holds any rung of NSE-INDEX-NOSUCH")
    );
    for (route, query, words) in [
        ("/sources.json", "segment=INDEX&symbol=NIFTY", "exchange="),
        (
            "/sources.json",
            "exchange=NSE&segment=FNO&symbol=NIFTY",
            "names one contract",
        ),
        (
            "/sources.json",
            "exchange=NSE&segment=INDEX&symbol=NIFTY&contract=2019-03-28-FUT",
            "holds none",
        ),
        ("/sources/bars.json", NIFTY, "rung="),
        (
            "/sources/bars.json",
            &format!("{NIFTY}&rung=7min&from=2019-03-28"),
            "rung=",
        ),
        (
            "/sources/bars.json",
            &format!("{NIFTY}&rung=1min"),
            "from=YYYY-MM-DD is required",
        ),
        (
            "/sources/bars.json",
            &format!("{NIFTY}&rung=1min&from=2019-03-29&to=2019-03-28"),
            "is after",
        ),
        (
            "/sources/bars.json",
            &format!("{NIFTY}&rung=1min&from=2019-03-28&limit=201"),
            "1 to 200",
        ),
        (
            "/sources/bars.json",
            &format!("{NIFTY}&rung=1day&from=2019-03-28&limit=32"),
            "1 to 31",
        ),
        (
            "/sources/bars.json",
            &format!("{NIFTY}&rung=1min&from=2019-03-28&offset=1001"),
            "cursor=",
        ),
        (
            "/sources/bars.json",
            &format!("{NIFTY}&rung=1min&from=2019-03-28&dir=up"),
            "asc or desc",
        ),
        (
            "/sources/day.json",
            NIFTY,
            "from=YYYY-MM-DD or from=YYYY-MM is required",
        ),
        (
            "/sources/day.json",
            &format!("{NIFTY}&from=2019-13"),
            "not a day",
        ),
        (
            "/sources/day.json",
            &format!("{NIFTY}&from=2019-03&page=x"),
            "whole number",
        ),
    ] {
        let (status, body) = fx.get(route, query).await;
        assert_eq!(
            status,
            axum::http::StatusCode::BAD_REQUEST,
            "{route}?{query}: {body}"
        );
        assert!(
            body["error"]
                .as_str()
                .is_some_and(|why| why.contains(words)),
            "{route}?{query}: {body}"
        );
    }
}

// ══ The timeline ═════════════════════════════════════════════════════════════

/// **ONE TIMELINE AT 1MIN, EVERY ROW TAGGED BY SOURCE** (DB-079, D-3339
/// items 1 and 2): Zerodha's stored minutes beside GDFL's seconds folded by
/// `pull::fold`, the pre-open second a GDFL-only row, equal minutes within
/// tolerance and 09:16's close flagged at 4.99 bp.
#[tokio::test]
async fn the_minute_timeline_tags_every_cell_and_flags_a_close_past_one_bp() {
    let fx = Fixture::nifty("minutes");
    let (status, body) = fx
        .get(
            "/sources/bars.json",
            &format!("{NIFTY}&rung=1min&from=2019-03-28&dir=asc"),
        )
        .await;
    assert_eq!(status, axum::http::StatusCode::OK, "{body:#}");
    let rows = body["rows"].as_array().expect("rows");
    let clocks: Vec<&str> = rows
        .iter()
        .map(|r| r["ist"].as_str().expect("ist"))
        .collect();
    assert_eq!(
        clocks,
        [
            "2019-03-28 09:07:00",
            "2019-03-28 09:15:00",
            "2019-03-28 09:16:00",
            "2019-03-28 09:17:00"
        ]
    );
    let sources: Vec<&str> = body["sources"]
        .as_array()
        .expect("sources")
        .iter()
        .map(|s| s["tag"].as_str().expect("tag"))
        .collect();
    assert_eq!(sources, ["gdfl_fold", "zerodha_stored"]);

    // THE PRE-OPEN SECOND IS GDFL'S ALONE.
    assert_eq!(rows[0]["only"], "gdfl");
    assert!(rows[0]["cells"]["zerodha"].is_null());
    assert_eq!(rows[0]["cells"]["gdfl"]["seconds"], 1);

    // 09:15: TWO SECONDS FOLDED, EVERY FIELD EQUAL.
    let fold = &rows[1]["cells"]["gdfl"];
    assert_eq!(fold["tag"], "gdfl_fold");
    assert_eq!(
        (
            &fold["o"],
            &fold["h"],
            &fold["l"],
            &fold["c"],
            &fold["seconds"]
        ),
        (
            &json!(100_000),
            &json!(100_200),
            &json!(99_900),
            &json!(100_100),
            &json!(2)
        )
    );
    assert_eq!(rows[1]["cells"]["zerodha"]["tag"], "zerodha_stored");
    assert_eq!(rows[1]["cells"]["zerodha"]["oi"], Value::Null);
    assert_eq!(rows[1]["cells"]["zerodha"]["oi_why"], "not_a_derivative");
    assert_eq!(rows[1]["cmp"]["c"]["bps"], n("0.00"));
    assert_eq!(rows[1]["cmp"]["v"]["why"], "index_no_volume");
    assert_eq!(rows[1]["flags"], json!([]));
    assert!(rows[1]["only"].is_null());

    // 09:16: GDFL's last print 1,003.00 against Zerodha's 1,002.50.
    assert_eq!(rows[2]["cmp"]["c"]["bps"], n("4.99"), "{:#}", rows[2]);
    assert_eq!(rows[2]["cmp"]["c"]["flag"], true);
    assert_eq!(rows[2]["cmp"]["base"]["of"], "zerodha");
    assert_eq!(rows[2]["flags"], json!(["c"]));

    assert_eq!(rows[2]["cmp"]["c"]["paisa"], 50);
    assert_eq!(rows[2]["cmp"]["c"]["equal"], false);
    assert_eq!(rows[2]["cmp"]["o"]["equal"], true);

    assert_eq!(body["summary"]["both_feeds"], 3);
    assert_eq!(body["summary"]["one_feed"], 1);
    assert_eq!(body["summary"]["prices_disagree"], 1);
    // WITHIN TOLERANCE IS NOT EQUAL: the exact count is kept beside it.
    assert_eq!(body["summary"]["compared_on_one_basis"], 3);
    assert_eq!(
        body["summary"]["exactly_equal"],
        json!({ "o": 3, "h": 3, "l": 3, "c": 2 })
    );
    assert_eq!(body["next_cursor"], Value::Null);
    assert_eq!(body["complete"], true);
}

/// **A PAGE NEVER SKIPS OR REPEATS A BUCKET**: newest first two at a time by
/// `cursor` covers what one asc page holds, and `offset` lands where the
/// cursor does.
#[tokio::test]
async fn paging_by_cursor_and_by_offset_covers_every_bucket_once() {
    let fx = Fixture::nifty("paging");
    let base = format!("{NIFTY}&rung=1min&from=2019-03-28");
    let (_, first) = fx
        .get("/sources/bars.json", &format!("{base}&limit=2"))
        .await;
    let stamps = |body: &Value| -> Vec<i64> {
        body["rows"]
            .as_array()
            .expect("rows")
            .iter()
            .map(|r| r["t"].as_i64().expect("t"))
            .collect()
    };
    let a = day_a();
    let s = |h, m| t(a, h, m, 0) / 1_000_000;
    assert_eq!(
        stamps(&first),
        [s(9, 17), s(9, 16)],
        "newest first by default"
    );
    assert_eq!(first["next_cursor"], s(9, 15));
    assert_eq!(first["complete"], false);
    let (_, second) = fx
        .get(
            "/sources/bars.json",
            &format!("{base}&limit=2&cursor={}", s(9, 15)),
        )
        .await;
    assert_eq!(stamps(&second), [s(9, 15), s(9, 7)]);
    assert_eq!(second["next_cursor"], Value::Null);
    let (_, offset) = fx
        .get("/sources/bars.json", &format!("{base}&limit=2&offset=2"))
        .await;
    assert_eq!(stamps(&offset), stamps(&second));
    // AN INSTANT INSIDE A BUCKET IS ITS BUCKET.
    let (_, inside) = fx
        .get(
            "/sources/bars.json",
            &format!("{base}&limit=1&dir=asc&cursor={}", s(9, 16) + 30),
        )
        .await;
    assert_eq!(stamps(&inside), [s(9, 16)]);
    assert_eq!(inside["next_cursor"], s(9, 17));
}

/// **AT THE DAY RUNG THE CLOSE IS OFFICIAL AGAINST A LAST PRINT, LABELLED,
/// NEVER A DISAGREEMENT** (DB-035, DB-G99, D-3324): Zerodha's 1day against
/// GDFL's seconds folded to the IST day — open, high and low equal, the
/// close's 4.99 bp shown and not flagged.
#[tokio::test]
async fn the_day_rung_labels_official_against_last_print_and_never_flags_it() {
    let fx = Fixture::nifty("day-rung");
    let (status, body) = fx
        .get(
            "/sources/bars.json",
            &format!("{NIFTY}&rung=1day&from=2019-03-28&to=2019-03-29&dir=asc"),
        )
        .await;
    assert_eq!(status, axum::http::StatusCode::OK, "{body:#}");
    let rows = body["rows"].as_array().expect("rows");
    assert_eq!(rows.len(), 2);
    let a = &rows[0];
    assert_eq!(a["ist"], "2019-03-28 00:00:00");
    assert_eq!(a["cells"]["zerodha"]["close"], "official");
    assert_eq!(a["cells"]["gdfl"]["close"], "last_print");
    assert_eq!(a["cells"]["gdfl"]["seconds"], 5);
    assert_eq!(a["cmp"]["o"]["flag"], false);
    assert_eq!(a["cmp"]["h"]["flag"], false);
    assert_eq!(a["cmp"]["l"]["flag"], false);
    assert_eq!(a["cmp"]["c"]["semantics"], "official_vs_last_print");
    assert_eq!(a["cmp"]["c"]["flag"], false);
    assert_eq!(a["cmp"]["c"]["bps"], n("-4.99"), "{a:#}");
    assert_eq!(a["flags"], json!(["official_vs_last_print"]));
    assert_eq!(body["summary"]["prices_disagree"], 0);
}

/// A rung only one feed holds is that feed's alone, said so; an option is
/// GDFL's, and its fold carries the last open interest it carried.
#[tokio::test]
async fn one_source_is_not_compared_and_an_option_folds_its_last_carried_oi() {
    let fx = Fixture::nifty("option");
    let a = day_a();
    fx.bars(
        Vendor::Gdfl,
        "FNO",
        "NIFTY",
        Some(option()),
        Timeframe::SECOND_1,
        &[
            bar(t(a, 9, 15, 0), (1_000, 1_000, 1_000, 1_000), 75, 5_000),
            bar(t(a, 9, 15, 20), (1_050, 1_050, 1_050, 1_050), 150, OI_NULL),
            bar(t(a, 9, 16, 0), (1_100, 1_100, 1_100, 1_100), 75, 5_100),
        ],
    );
    let (status, body) = fx
        .get(
            "/sources/bars.json",
            "exchange=NSE&segment=FNO&symbol=NIFTY&contract=2019-03-28-110000-CE&rung=1min\
             &from=2019-03-28&dir=asc",
        )
        .await;
    assert_eq!(status, axum::http::StatusCode::OK, "{body:#}");
    let rows = body["rows"].as_array().expect("rows");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["cells"]["gdfl"]["oi"], 5_000, "the last carried");
    assert_eq!(rows[0]["cells"]["gdfl"]["v"], 225);
    assert_eq!(rows[1]["cells"]["gdfl"]["oi"], 5_100);
    assert!(rows[0]["cmp"].is_null());
    assert!(rows[0]["only"].is_null(), "one source is not 'only' a feed");
    let notes: Vec<&str> = body["notes"]
        .as_array()
        .expect("notes")
        .iter()
        .map(|note| note.as_str().expect("a note"))
        .collect();
    assert!(
        notes
            .iter()
            .any(|note| note.contains("one source holds this rung")),
        "{notes:#?}"
    );
    // THE FEED THAT GIVES NO SOURCE IS NAMED, AND WHY: exactly one note
    // speaks for Zerodha, and it names the contract. Every note is read, so
    // the one-source note above is read as the one that is not Zerodha's.
    let zerodha: Vec<&&str> = notes
        .iter()
        .filter(|note| note.starts_with("Zerodha"))
        .collect();
    assert_eq!(zerodha.len(), 1, "{notes:#?}");
    assert!(
        zerodha[0].contains("NSE-FNO-NIFTY-2019-03-28-110000-CE"),
        "{notes:#?}"
    );
    let (_, seconds) = fx
        .get(
            "/sources/bars.json",
            &format!("{NIFTY}&rung=1s&from=2019-03-28&dir=asc"),
        )
        .await;
    let tags: Vec<&str> = seconds["sources"]
        .as_array()
        .expect("sources")
        .iter()
        .map(|s| s["tag"].as_str().expect("tag"))
        .collect();
    assert_eq!(tags, ["gdfl_1s"], "Zerodha holds no second");
    assert_eq!(seconds["rows"].as_array().expect("rows").len(), 5);
    assert!(
        seconds["notes"][0]
            .as_str()
            .expect("a note")
            .starts_with("Zerodha neither serves nor derives a 1s rung"),
        "{:#}",
        seconds["notes"]
    );
}

/// **A SHARE'S FACTOR REPLACES A DISAGREEMENT WHERE THE BASES DIFFER**
/// (DB-G58, DB-G75, D-3339 item 3): Zerodha back-adjusted against GDFL raw
/// reads `factor` 2.0002 and `basis_differs`, never five price flags.
#[tokio::test]
async fn a_share_whose_bases_differ_shows_the_factor_not_a_disagreement() {
    let fx = Fixture::share("share-minutes");
    let (status, body) = fx
        .get(
            "/sources/bars.json",
            &format!("{SHARE}&rung=1min&from=2019-03-28&dir=asc"),
        )
        .await;
    assert_eq!(status, axum::http::StatusCode::OK, "{body:#}");
    let rows = body["rows"].as_array().expect("rows");
    assert_eq!(rows.len(), 2);
    let second = &rows[1];
    assert_eq!(second["cmp"]["factor"], n("2.0002"), "{second:#}");
    assert_eq!(second["cmp"]["basis"], "basis_differs");
    assert_eq!(second["cmp"]["c"]["why"], "basis_differs");
    assert!(second["cmp"]["c"].get("flag").is_none());
    assert_eq!(second["cmp"]["v"]["why"], "basis_differs");
    assert_eq!(second["flags"], json!(["basis_differs"]));
    assert_eq!(body["summary"]["basis_differs"], 2);
    assert_eq!(body["summary"]["prices_disagree"], 0);
    // ON B THE FACTOR IS 1: compared, and within tolerance.
    let (_, b) = fx
        .get(
            "/sources/bars.json",
            &format!("{SHARE}&rung=1min&from=2019-03-29&dir=asc"),
        )
        .await;
    let only = &b["rows"][0];
    assert_eq!(only["cmp"]["factor"], n("1.0000"));
    assert_eq!(only["cmp"]["basis"], "adjusted_but_equal_here");
    assert_eq!(only["cmp"]["v"]["percent"], n("0.00"));
    assert_eq!(only["flags"], json!([]));
}

// ══ The day table ════════════════════════════════════════════════════════════

/// **THE DAY RECONCILIATION** (DB-035, DB-G41, DB-G99, D-3339 item 4):
/// Zerodha's official day, its minutes folded and GDFL's seconds folded;
/// open, high and low equal on both days, the close differing on A and equal
/// on B, the first print the pre-open second, B's minute high above its day
/// high, and an index's volume never compared.
#[tokio::test]
async fn the_day_table_reconciles_the_official_day_its_minutes_and_the_seconds() {
    let fx = Fixture::nifty("day-table");
    let (status, body) = fx
        .get(
            "/sources/day.json",
            &format!("{NIFTY}&from=2019-03-28&to=2019-03-29"),
        )
        .await;
    assert_eq!(status, axum::http::StatusCode::OK, "{body:#}");
    let days = body["days"].as_array().expect("days");
    assert_eq!(days.len(), 2);
    let a = &days[0];
    assert_eq!(a["day"], "2019-03-28");
    assert_eq!(a["zerodha_day"]["c"], 100_200);
    assert_eq!(a["zerodha_day_close"], "official");
    assert_eq!(a["zerodha_minutes_fold"]["c"], 100_150);
    assert_eq!(a["zerodha_minutes_fold"]["minutes"], 3);
    assert_eq!(a["gdfl_seconds_fold"]["seconds"], 5);
    assert_eq!(a["gdfl_seconds_fold"]["first_at"], "09:07:59");
    assert_eq!(a["open"]["gdfl_first_print"], 100_000);
    assert_eq!(a["open"]["gdfl_first_print_at"], "09:07:59");
    assert_eq!(a["open"]["gdfl_vs_zerodha_day"]["flag"], false);
    assert_eq!(a["close"]["official"], 100_200);
    assert_eq!(a["close"]["gdfl_last_print"], 100_150);
    assert_eq!(
        a["close"]["official_vs_gdfl_last_print"]["semantics"],
        "official_vs_last_print"
    );
    assert_eq!(a["close"]["official_vs_gdfl_last_print"]["flag"], false);
    assert_eq!(a["close"]["equal"], false);
    assert_eq!(a["volume"]["why"], "index_no_volume");
    assert!(a["basis"].is_null(), "an index has no adjustment");
    assert_eq!(a["flags"], json!(["official_vs_last_print"]));
    assert!(
        a["marks"]["calendar"]["kind"].is_string(),
        "{:#}",
        a["marks"]
    );

    let b = &days[1];
    assert_eq!(b["close"]["equal"], true);
    assert_eq!(b["high"]["minute_above_day"], true);
    assert_eq!(
        b["flags"],
        json!(["minute_extremes_outside_day", "official_vs_last_print"])
    );

    let summary = &body["summary"];
    assert_eq!(
        (
            &summary["open_equal"],
            &summary["high_equal"],
            &summary["low_equal"],
            &summary["close_equal"],
            &summary["close_differs"],
            &summary["minute_extremes_outside_day"],
        ),
        (
            &json!(2),
            &json!(2),
            &json!(2),
            &json!(1),
            &json!(1),
            &json!(1)
        ),
        "{summary:#}"
    );
    assert_eq!(body["pages"], 1);
    assert_eq!(body["page_days"], 31);
}

/// **A SHARE'S DAY: THE FACTOR, ITS STEP, VOLUME ON ITS OWN BASIS AND A
/// COARSER GRID** (DB-G38, DB-G39, DB-G40, DB-G41, DB-G75): on A Zerodha sits
/// at GDFL ÷ 2.0002 on a 100-paisa grid against GDFL's 10, its day volume is
/// not GDFL's times the factor, and its minutes are 6.25% off its day;
/// on B the factor is 1, a step of more than 0.2%.
#[tokio::test]
async fn a_share_day_names_its_factor_step_volume_basis_and_grid() {
    let fx = Fixture::share("share-days");
    let (status, body) = fx
        .get("/sources/day.json", &format!("{SHARE}&from=2019-03"))
        .await;
    assert_eq!(status, axum::http::StatusCode::OK, "{body:#}");
    assert_eq!(body["page_from"], "2019-03-01");
    assert_eq!(body["page_to"], "2019-03-31");
    let days = body["days"].as_array().expect("days");
    let find = |on: &str| days.iter().find(|d| d["day"] == on).expect("the day");
    let a = find("2019-03-28");
    assert_eq!(a["basis"]["factor"], n("2.0002"), "{a:#}");
    assert_eq!(
        a["basis"]["factor_of"],
        "GDFL first print ÷ Zerodha day open (D-3323's measure)"
    );
    assert_eq!(a["basis"]["minute_factor"], n("2.0002"));
    assert_eq!(a["basis"]["minute_factor_at"], "09:16:00");
    assert_eq!(a["basis"]["minute_basis_differs_from_day"], false);
    assert_eq!(a["basis"]["bases"], "differ");
    assert_eq!(a["basis"]["basis_differs"], true);
    assert_eq!(a["basis"]["basis_step"], Value::Null);
    assert_eq!(a["basis"]["volume_factor"], n("1.0667"));
    assert_eq!(a["basis"]["volume_not_scaled_with_price"], true);
    assert_eq!(
        a["basis"]["grid_paisa"],
        json!({ "zerodha_minutes": 100, "gdfl_seconds": 10 })
    );
    assert_eq!(a["basis"]["quantised"], true);
    assert_eq!(a["open"]["gdfl_vs_zerodha_day"]["why"], "basis_differs");
    assert_eq!(a["volume"]["minute_over_day"], n("0.9375"));
    assert_eq!(a["volume"]["minute_vs_day"]["percent"], n("-6.25"));
    assert_eq!(a["volume"]["minute_vs_day"]["flag"], true);
    // THE OFFICIAL CLOSE EQUALS THE LAST MINUTE'S AND GDFL'S IS ON ANOTHER
    // BASIS: nothing official-against-last-print to name.
    assert_eq!(
        a["flags"],
        json!(["minute_volume_off_day", "basis_differs"])
    );
    let b = find("2019-03-29");
    assert_eq!(b["basis"]["factor"], n("1.0000"));
    assert_eq!(b["basis"]["basis_differs"], false);
    assert_eq!(b["basis"]["basis_step"], true);
    assert_eq!(b["basis"]["quantised"], false);
    assert_eq!(b["volume"]["gdfl_vs_zerodha_day"]["percent"], n("0.00"));
    assert_eq!(b["volume"]["gdfl_vs_zerodha_day"]["flag"], false);
    let summary = &body["summary"];
    assert_eq!(summary["basis_differs"], 1);
    assert_eq!(summary["basis_steps"], 1);
    assert_eq!(summary["volume_within_tolerance"], 1);
    assert_eq!(summary["both_feeds"], 2);
    assert_eq!(summary["neither"], 29);
    // THE AUCTION HAS NOT STARTED IN 2019.
    assert!(a["auction"].is_null());
}

/// **A CLOSING-AUCTION DAY COMPARES THE SAME MINUTE, AND THE AUCTION PRINT
/// IS NEVER READ AS A BASIS** (D-3320, D-3339 item 3): from 2026-08-03 a CAS
/// share's Zerodha minutes stop at 15:14 while GDFL's month, imported before
/// the auction rule, holds the 15:28 auction print as a bar. The day's last
/// prints are a quarter of an hour apart; the factor is read off the open,
/// the last-print comparison is made at 15:14 on both feeds, the official
/// close equals the auction print, and the missing sidecar is said.
#[tokio::test]
async fn a_closing_auction_day_compares_the_same_minute_and_reads_no_false_basis() {
    let fx = Fixture::empty("cas-day");
    let cas = day(2026, 9, 22);
    let (z, g, cash) = (Vendor::Zerodha, Vendor::Gdfl, Segment::Cash);
    fx.counted(
        z,
        cash,
        "SYNTHB",
        Timeframe::MINUTE_1,
        &[
            bar(
                t(cas, 9, 15, 0),
                (70_000, 70_100, 69_900, 70_050),
                300,
                OI_NULL,
            ),
            bar(
                t(cas, 15, 14, 0),
                (70_050, 70_200, 70_000, 70_150),
                200,
                OI_NULL,
            ),
        ],
    );
    fx.counted(
        z,
        cash,
        "SYNTHB",
        Timeframe::DAY_1,
        &[bar(
            t(cas, 0, 0, 0),
            (70_000, 70_300, 69_900, 70_300),
            800,
            OI_NULL,
        )],
    );
    fx.counted(
        g,
        cash,
        "SYNTHB",
        Timeframe::SECOND_1,
        &[
            bar(
                t(cas, 9, 15, 0),
                (70_000, 70_100, 69_900, 70_050),
                300,
                OI_NULL,
            ),
            bar(
                t(cas, 15, 14, 30),
                (70_050, 70_200, 70_000, 70_150),
                200,
                OI_NULL,
            ),
            bar(
                t(cas, 15, 28, 7),
                (70_300, 70_300, 70_300, 70_300),
                300,
                OI_NULL,
            ),
        ],
    );
    let (status, body) = fx
        .get(
            "/sources/day.json",
            "exchange=NSE&segment=CASH&symbol=SYNTHB&from=2026-09-22",
        )
        .await;
    assert_eq!(status, axum::http::StatusCode::OK, "{body:#}");
    let row = &body["days"][0];
    assert_eq!(row["basis"]["factor"], n("1.0000"), "{row:#}");
    assert_eq!(row["basis"]["basis_differs"], false);
    assert_eq!(row["close"]["last_common_minute"]["at"], "15:14:00");
    assert_eq!(row["close"]["last_common_minute"]["zerodha"], 70_150);
    assert_eq!(row["close"]["last_common_minute"]["gdfl"], 70_150);
    assert_eq!(
        row["close"]["last_common_minute"]["gdfl_vs_zerodha"]["flag"],
        false
    );
    assert_eq!(
        row["close"]["equal"], true,
        "the official close is the auction's"
    );
    assert_eq!(row["last_print"]["gdfl_last_print_at"], "15:28:07");
    assert_eq!(row["last_print"]["zerodha_last_minute_at"], "15:14:00");
    assert_eq!(row["volume"]["gdfl_vs_zerodha_day"]["flag"], false);
    assert_eq!(row["auction"]["state"], "no_sidecar");
    assert!(
        row["flags"]
            .as_array()
            .expect("flags")
            .iter()
            .all(|flag| flag != "basis_differs" && flag != "basis_step"),
        "{row:#}"
    );
}

/// **ZERODHA'S MINUTES ON ANOTHER BASIS THAN ITS DAY ARE NAMED** (DB-G38,
/// DB-G41): the day reads GDFL ÷ 2.0000 off the open while the last common
/// minute reads ÷ 2.0408 — a minute basis 2% from the day's, flagged.
#[tokio::test]
async fn a_minute_basis_apart_from_the_day_basis_is_named() {
    let fx = Fixture::empty("minute-basis");
    let on = day_a();
    let (z, g, cash) = (Vendor::Zerodha, Vendor::Gdfl, Segment::Cash);
    fx.counted(
        z,
        cash,
        "SYNTHC",
        Timeframe::MINUTE_1,
        &[bar(
            t(on, 9, 15, 0),
            (49_000, 49_000, 49_000, 49_000),
            100,
            OI_NULL,
        )],
    );
    fx.counted(
        z,
        cash,
        "SYNTHC",
        Timeframe::DAY_1,
        &[bar(
            t(on, 0, 0, 0),
            (50_000, 50_000, 50_000, 50_000),
            100,
            OI_NULL,
        )],
    );
    fx.counted(
        g,
        cash,
        "SYNTHC",
        Timeframe::SECOND_1,
        &[bar(
            t(on, 9, 15, 0),
            (100_000, 100_000, 100_000, 100_000),
            100,
            OI_NULL,
        )],
    );
    let (_, body) = fx
        .get(
            "/sources/day.json",
            "exchange=NSE&segment=CASH&symbol=SYNTHC&from=2019-03-28",
        )
        .await;
    let row = &body["days"][0];
    assert_eq!(row["basis"]["factor"], n("2.0000"), "{row:#}");
    assert_eq!(row["basis"]["minute_factor"], n("2.0408"));
    assert_eq!(row["basis"]["minute_basis_differs_from_day"], true);
    assert!(
        row["flags"]
            .as_array()
            .expect("flags")
            .iter()
            .any(|flag| flag == "minute_basis_differs_from_day"),
        "{row:#}"
    );
}

/// The last minute both feeds hold is found walking back from both ends, a
/// GDFL second after Zerodha's last minute never standing in for it.
#[test]
fn the_last_common_minute_is_the_same_minute_on_both_feeds() {
    let on = day_a();
    let minutes = [
        bar(t(on, 9, 15, 0), (100, 100, 100, 100), 0, OI_NULL),
        bar(t(on, 15, 14, 0), (110, 110, 110, 110), 0, OI_NULL),
    ];
    let seconds = [
        bar(t(on, 9, 15, 3), (200, 200, 200, 200), 0, OI_NULL),
        bar(t(on, 15, 14, 20), (219, 219, 219, 219), 0, OI_NULL),
        bar(t(on, 15, 14, 59), (220, 220, 220, 220), 0, OI_NULL),
        bar(t(on, 15, 28, 0), (999, 999, 999, 999), 0, OI_NULL),
    ];
    assert_eq!(
        common_minute(&minutes, &seconds),
        Some((t(on, 15, 14, 0), 110, 220))
    );
    // GDFL SILENT IN ZERODHA'S LAST MINUTE: the one before it.
    let sparse = [
        bar(t(on, 9, 15, 3), (200, 200, 200, 200), 0, OI_NULL),
        bar(t(on, 15, 28, 0), (999, 999, 999, 999), 0, OI_NULL),
    ];
    assert_eq!(
        common_minute(&minutes, &sparse),
        Some((t(on, 9, 15, 0), 100, 200))
    );
    assert_eq!(common_minute(&minutes, &[]), None);
    assert_eq!(common_minute(&[], &seconds), None);
}

/// **CACHED PER GENERATION**: a second read of an unchanged page is served
/// from the cache, and an append to one month file reads that day again —
/// never a stale fold.
#[tokio::test]
async fn the_day_table_is_cached_until_a_month_file_changes() {
    let fx = Fixture::nifty("day-cache");
    let query = format!("{NIFTY}&from=2019-03-28&to=2019-03-29");
    let (_, first) = fx.get("/sources/day.json", &query).await;
    assert_eq!(first["summary"]["days_from_cache"], 0);
    assert!(first["records_read"].as_u64().expect("count") > 0);
    let (_, again) = fx.get("/sources/day.json", &query).await;
    assert_eq!(again["summary"]["days_from_cache"], 2);
    assert_eq!(again["records_read"], 0);
    assert_eq!(again["days"], first["days"]);
    // ONE MORE SECOND ON B: the month file's stamp moves.
    fx.bars(
        Vendor::Gdfl,
        "INDEX",
        "NIFTY",
        None,
        Timeframe::SECOND_1,
        &[bar(
            t(day_b(), 9, 16, 0),
            (101_250, 101_300, 101_200, 101_300),
            0,
            OI_NULL,
        )],
    );
    let (_, after) = fx.get("/sources/day.json", &query).await;
    assert_eq!(
        after["summary"]["days_from_cache"], 0,
        "the month file changed"
    );
    assert_eq!(after["days"][1]["gdfl_seconds_fold"]["seconds"], 2);
    assert_eq!(after["days"][1]["close"]["gdfl_last_print"], 101_300);
    assert_eq!(after["days"][1]["close"]["equal"], false);
}

/// A VERSIONED REPAIR IS A NEW CACHE KEY, NEVER A STALE HIT (D-3350, WP-13c).
/// A repair publishes a new generation of the month and leaves the original
/// file byte for byte as it was, so a memo keyed by the original's stamp would
/// keep answering the bars from before the repair. Keyed by the generation
/// every reader opens, the next answer reads the repaired month.
#[tokio::test]
async fn a_repaired_month_is_read_from_its_new_generation_not_the_cache() {
    let fx = Fixture::nifty("day-repair");
    let query = format!("{NIFTY}&from=2019-03-28&to=2019-03-29");
    let (_, first) = fx.get("/sources/day.json", &query).await;
    assert_eq!(first["days"][1]["zerodha_minutes_fold"]["minutes"], 1);
    let (_, again) = fx.get("/sources/day.json", &query).await;
    assert_eq!(again["summary"]["days_from_cache"], 2);

    // A MINUTE ON B, BETWEEN HELD BARS: only a repair can add it.
    let path = StorePath::new(PathParts {
        vendor: Vendor::Zerodha,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "NIFTY",
        contract: None,
        timeframe: Timeframe::MINUTE_1,
        month: YearMonth::new(day_b().year(), day_b().month()).expect("a month"),
        file: FileKind::Bars,
    })
    .expect("fixture path");
    let hash = brutex_core::universe::fnv1a("NIFTY").to_le_bytes();
    let id = u32::from_le_bytes(hash[..4].try_into().expect("low32"));
    let original = fs::read(path.to_path_buf(&fx.root)).expect("the original month");
    let repaired = store::repair::repair_month(
        &fx.root,
        path,
        id,
        &[bar(
            t(day_b(), 9, 16, 0),
            (101_200, 101_300, 101_100, 101_250),
            0,
            OI_NULL,
        )],
        store::repair::Policy {
            open_interest_applies: false,
            basis_witness: false,
        },
    )
    .expect("the repair");
    assert!(
        matches!(
            repaired,
            store::repair::MonthRepair::Repaired { added: 1, .. }
        ),
        "{repaired:?}"
    );
    assert_eq!(
        fs::read(path.to_path_buf(&fx.root)).expect("the original month"),
        original,
        "the original's bytes, and so its stamp's size, are unchanged"
    );

    let (_, after) = fx.get("/sources/day.json", &query).await;
    assert_eq!(
        after["summary"]["days_from_cache"], 0,
        "the month every reader opens changed"
    );
    assert_eq!(after["days"][1]["zerodha_minutes_fold"]["minutes"], 2);
    assert_eq!(after["days"][1]["zerodha_minutes_fold"]["c"], 101_250);
}

/// Newest first and pages of 31 days: a quarter is three pages, page 1 the
/// middle one, and a page past the end is the last.
#[tokio::test]
async fn the_day_table_pages_by_thirty_one_days_either_way() {
    let fx = Fixture::nifty("day-pages");
    let base = format!("{NIFTY}&from=2019-01-01&to=2019-03-31");
    let (_, asc) = fx.get("/sources/day.json", &format!("{base}&page=1")).await;
    assert_eq!(asc["pages"], 3);
    assert_eq!(asc["page_from"], "2019-02-01");
    assert_eq!(asc["page_to"], "2019-03-03");
    let (_, desc) = fx
        .get("/sources/day.json", &format!("{base}&order=desc&page=9"))
        .await;
    assert_eq!(desc["page"], 2);
    assert_eq!(desc["page_from"], "2019-01-01");
    assert_eq!(desc["page_to"], "2019-01-28");
    assert_eq!(desc["days"][0]["day"], "2019-01-28", "newest first");
}

// ══ Fixes found on the operator's store, 10 Oct 2026 ═════════════════════════

/// **A DERIVED RUNG THAT LACKS A SESSION ITS MINUTES HOLD IS PARTIAL**, even
/// when its months and its last day match its minutes': NIFTY's derived
/// rungs lacked 2021-02-24, 2024-03-02, 2024-05-18 and 2025-10-21 on the
/// operator's store while every month matched, and read `held`.
#[tokio::test]
async fn a_derived_rung_missing_a_session_its_minutes_hold_is_partial() {
    let fx = Fixture::empty("derived-sessions");
    let (a, b, c) = (day_a(), day_b(), day_c());
    let (z, idx) = (Vendor::Zerodha, Segment::Index);
    let at_open =
        |on: Day, close: i64| bar(t(on, 9, 15, 0), (close, close, close, close), 0, OI_NULL);
    fx.counted(
        z,
        idx,
        "NIFTY",
        Timeframe::MINUTE_1,
        &[at_open(a, 100_000), at_open(b, 100_100)],
    );
    fx.counted(z, idx, "NIFTY", Timeframe::MINUTE_1, &[at_open(c, 100_200)]);
    fx.counted(z, idx, "NIFTY", Timeframe::MINUTE_5, &[at_open(a, 100_000)]);
    fx.counted(z, idx, "NIFTY", Timeframe::MINUTE_5, &[at_open(c, 100_200)]);
    let (status, body) = fx.get("/sources.json", NIFTY).await;
    assert_eq!(status, axum::http::StatusCode::OK, "{body:#}");
    let derived = row(&body["rungs"], "zerodha_derived", "5min");
    assert_eq!(derived["months_held"], 2);
    assert_eq!(derived["last_day"], "2019-04-01");
    assert_eq!(derived["state"], "partial", "{derived:#}");
    let behind = &derived["behind_source"];
    assert_eq!(behind["months_missing"], 0);
    assert_eq!(behind["days_behind"], 0);
    assert_eq!(behind["sessions_missing"], 1, "{behind:#}");
    assert_eq!(behind["sessions_missing_named"], json!(["2019-03-29"]));
    assert_eq!(behind["sessions_missing_exact"], true);
    assert_eq!(
        row(&body["rungs"], "zerodha_stored", "1min")["state"],
        "held"
    );
}

/// A derived rung whose sessions cannot be set against its minutes' (here
/// no minute rung is held to set them against) is never called `held`.
#[tokio::test]
async fn a_derived_rung_whose_sessions_cannot_be_checked_says_so() {
    let fx = Fixture::empty("derived-unchecked");
    let five = bar(
        t(day_a(), 9, 15, 0),
        (100_000, 100_000, 100_000, 100_000),
        0,
        OI_NULL,
    );
    fx.counted(
        Vendor::Zerodha,
        Segment::Index,
        "NIFTY",
        Timeframe::MINUTE_5,
        &[five],
    );
    let (status, body) = fx.get("/sources.json", NIFTY).await;
    assert_eq!(status, axum::http::StatusCode::OK, "{body:#}");
    let derived = row(&body["rungs"], "zerodha_derived", "5min");
    assert_eq!(derived["state"], "sessions_unchecked", "{derived:#}");
    assert!(derived["behind_source"].is_null());
}

/// Where either calendar cut its list of unheld sessions short, the sessions
/// a derived rung lacks are the difference of the counts, said to be a lower
/// bound; never a list read as whole.
#[test]
fn sessions_behind_is_exact_only_on_whole_lists() {
    let held = |count: u64, days: &[&str], whole: bool| NotHeld {
        count,
        days: days.iter().map(|day| (*day).to_owned()).collect(),
        whole,
    };
    assert_eq!(
        sessions_behind(
            &held(3, &["2021-02-24", "2024-03-02", "2024-11-01"], true),
            &held(1, &["2024-11-01"], true)
        ),
        (
            2,
            vec!["2021-02-24".to_owned(), "2024-03-02".to_owned()],
            true
        )
    );
    assert_eq!(
        sessions_behind(&held(70, &["2018-01-02"], false), &held(66, &[], true)),
        (4, Vec::new(), false)
    );
    assert_eq!(
        sessions_behind(
            &held(1, &["2024-11-01"], true),
            &held(1, &["2024-11-01"], true)
        ),
        (0, Vec::new(), true)
    );
}

/// A feed that holds nothing of the instrument is named in the rung table,
/// never left to an absent row.
#[tokio::test]
async fn a_feed_holding_nothing_of_the_instrument_is_named() {
    let fx = Fixture::empty("feed-holds-nothing");
    fx.bars(
        Vendor::Gdfl,
        "FNO",
        "NIFTY",
        Some(option()),
        Timeframe::SECOND_1,
        &[bar(
            t(day_a(), 9, 15, 0),
            (1_000, 1_000, 1_000, 1_000),
            75,
            5_000,
        )],
    );
    let (status, body) = fx
        .get(
            "/sources.json",
            "exchange=NSE&segment=FNO&symbol=NIFTY&contract=2019-03-28-110000-CE",
        )
        .await;
    assert_eq!(status, axum::http::StatusCode::OK, "{body:#}");
    let feeds = body["feeds"].as_array().expect("feeds");
    let rungs_of = |feed: &str| {
        feeds
            .iter()
            .find(|f| f["feed"] == feed)
            .map(|f| f["rungs_held"].clone())
    };
    assert_eq!(rungs_of("zerodha"), Some(json!(0)));
    assert_eq!(rungs_of("gdfl"), Some(json!(1)));
    assert_eq!(
        body["notes"],
        json!(["Zerodha holds no rung of NSE-FNO-NIFTY-2019-03-28-110000-CE in this store"])
    );
}

/// **A KEY THE RE-KEY RETIRED IS GONE ON EVERY ALL-SOURCES ROUTE** (D-3331;
/// WP-12c, DB-A18): `410` with the ledger's sentence, as the bar routes
/// answer it -- never a rung table that calls its kept month "uncounted",
/// and never a `503` that reads as a store fault.
#[tokio::test]
async fn a_key_the_rekey_retired_is_gone_on_every_route() {
    let fx = Fixture::empty("rekey-retired");
    fx.bars(
        Vendor::Gdfl,
        "FNO",
        "NIFTY",
        Some(option()),
        Timeframe::SECOND_1,
        &[bar(
            t(day_a(), 9, 15, 0),
            (1_000, 1_000, 1_000, 1_000),
            75,
            5_000,
        )],
    );
    let asked = "exchange=NSE&segment=FNO&symbol=NIFTY&contract=2019-03-28-110000-CE";
    let routes = [
        ("/sources.json", asked.to_owned()),
        (
            "/sources/bars.json",
            format!("{asked}&rung=1s&from=2019-03-28&dir=asc"),
        ),
        ("/sources/day.json", format!("{asked}&from=2019-03-28")),
    ];
    // BEFORE THE LEDGER NAMES IT, every route serves the key.
    for (route, query) in &routes {
        let (status, body) = fx.get(route, query).await;
        assert_eq!(status, axum::http::StatusCode::OK, "{route}: {body:#}");
    }
    let retired = pull::gdfl_lineage::RetiredRecord {
        underlying: Symbol::new("NIFTY").expect("a symbol"),
        key: option(),
        why: pull::gdfl_lineage::RetiredWhy::StrikeImplausible,
        day: day_a(),
        median: Some(10_000),
        days: 1,
        bars: 1,
    };
    let ledger = pull::gdfl_lineage::lineage_path(&fx.root);
    fs::create_dir_all(ledger.parent().expect("imports")).expect("imports dir");
    fs::write(&ledger, format!("{}\n", retired.line())).expect("the ledger");
    for (route, query) in &routes {
        let (status, body) = fx.get(route, query).await;
        assert_eq!(status, axum::http::StatusCode::GONE, "{route}: {body:#}");
        assert_eq!(body["error"], json!(retired.refusal()), "{route}");
    }
    // AN INDEX OVER THE SAME STORE IS NOT THE RETIRED KEY, and is not refused.
    let (status, _) = fx
        .get("/sources.json", "exchange=NSE&segment=INDEX&symbol=NIFTY")
        .await;
    assert_eq!(status, axum::http::StatusCode::OK);
}

/// A same-basis share whose official close is 0.5% from its last print on
/// day A, and whose GDFL file starts at 11:57:48 on day B.
fn same_basis_share(name: &str, with_minutes: bool) -> Fixture {
    let fx = Fixture::empty(name);
    let (a, b) = (day_a(), day_b());
    let (z, g, cash) = (Vendor::Zerodha, Vendor::Gdfl, Segment::Cash);
    fx.counted(
        z,
        cash,
        "SYNTHD",
        Timeframe::DAY_1,
        &[
            bar(
                t(a, 0, 0, 0),
                (80_000, 80_500, 79_900, 80_400),
                1_000,
                OI_NULL,
            ),
            bar(
                t(b, 0, 0, 0),
                (81_000, 81_300, 80_900, 81_150),
                900,
                OI_NULL,
            ),
        ],
    );
    fx.counted(
        g,
        cash,
        "SYNTHD",
        Timeframe::SECOND_1,
        &[
            bar(
                t(a, 9, 15, 0),
                (80_000, 80_500, 79_900, 80_100),
                600,
                OI_NULL,
            ),
            bar(
                t(a, 15, 29, 59),
                (80_000, 80_000, 80_000, 80_000),
                400,
                OI_NULL,
            ),
            bar(
                t(b, 11, 57, 48),
                (81_200, 81_200, 81_100, 81_100),
                300,
                OI_NULL,
            ),
            bar(
                t(b, 12, 0, 30),
                (81_150, 81_150, 81_150, 81_150),
                200,
                OI_NULL,
            ),
        ],
    );
    if with_minutes {
        fx.counted(
            z,
            cash,
            "SYNTHD",
            Timeframe::MINUTE_1,
            &[
                bar(
                    t(a, 9, 15, 0),
                    (80_000, 80_500, 79_900, 80_100),
                    600,
                    OI_NULL,
                ),
                bar(
                    t(a, 15, 29, 0),
                    (80_000, 80_000, 80_000, 80_000),
                    400,
                    OI_NULL,
                ),
                bar(
                    t(b, 9, 15, 0),
                    (81_000, 81_300, 80_900, 81_100),
                    400,
                    OI_NULL,
                ),
                bar(
                    t(b, 12, 0, 0),
                    (81_150, 81_150, 81_150, 81_150),
                    500,
                    OI_NULL,
                ),
            ],
        );
        // DAY C, MEASURED OFF THE OPEN AGAIN.
        let c = day_c();
        let open_bar = |at: i64| bar(at, (82_000, 82_100, 81_900, 82_050), 500, OI_NULL);
        fx.counted(
            z,
            cash,
            "SYNTHD",
            Timeframe::DAY_1,
            &[open_bar(t(c, 0, 0, 0))],
        );
        fx.counted(
            g,
            cash,
            "SYNTHD",
            Timeframe::SECOND_1,
            &[open_bar(t(c, 9, 15, 2))],
        );
        fx.counted(
            z,
            cash,
            "SYNTHD",
            Timeframe::MINUTE_1,
            &[open_bar(t(c, 9, 15, 0))],
        );
    }
    fx
}

/// **AT THE DAY RUNG A SHARE'S FACTOR IS READ OFF THE OPENING PRINT, NEVER
/// OFF AN OFFICIAL CLOSE AGAINST A LAST PRINT** (D-3339 item 3, D-3324):
/// RELIANCE read `basis_differs` on 2025-01-08, -10, -16 and -21 on the
/// operator's store, same basis, because its official close sat more than
/// 0.2% from GDFL's last print. And where GDFL's first print is after the
/// opening minute no pair is the same instant: unmeasured, said so.
#[tokio::test]
async fn a_share_day_rung_factor_is_the_opening_print_and_unmeasured_without_one() {
    let fx = same_basis_share("share-day-rung", false);
    let (status, body) = fx
        .get(
            "/sources/bars.json",
            "exchange=NSE&segment=CASH&symbol=SYNTHD&rung=1day&from=2019-03-28&to=2019-03-29\
             &dir=asc",
        )
        .await;
    assert_eq!(status, axum::http::StatusCode::OK, "{body:#}");
    let rows = body["rows"].as_array().expect("rows");
    assert_eq!(rows.len(), 2);
    let a = &rows[0];
    assert_eq!(a["cmp"]["factor"], n("1.0000"), "{a:#}");
    assert!(
        a["cmp"]["factor_of"]
            .as_str()
            .expect("words")
            .starts_with("GDFL first print ÷ Zerodha official open")
    );
    assert_eq!(a["cmp"]["basis"], "adjusted_but_equal_here");
    assert_eq!(a["cmp"]["c"]["semantics"], "official_vs_last_print");
    assert_eq!(a["cmp"]["c"]["flag"], false);
    assert_eq!(a["cmp"]["c"]["paisa"], -400);
    assert_eq!(a["cmp"]["o"]["equal"], true);
    assert_eq!(a["cmp"]["v"]["percent"], n("0.00"));
    assert_eq!(a["cells"]["gdfl"]["first_at"], "09:15:00");
    assert_eq!(a["flags"], json!(["official_vs_last_print"]));

    let b = &rows[1];
    assert_eq!(b["cells"]["gdfl"]["first_at"], "11:57:48");
    assert_eq!(b["cmp"]["factor"], Value::Null, "{b:#}");
    assert_eq!(b["cmp"]["basis"], "basis_unmeasured");
    assert_eq!(b["cmp"]["o"]["why"], "basis_unmeasured");
    assert_eq!(b["cmp"]["o"]["flag"], Value::Null);
    assert_eq!(b["cmp"]["v"]["why"], "basis_unmeasured");
    assert_eq!(b["flags"], json!(["basis_unmeasured"]));

    let summary = &body["summary"];
    assert_eq!(summary["basis_differs"], 0, "{summary:#}");
    assert_eq!(summary["basis_unmeasured"], 1);
    assert_eq!(summary["prices_within_tolerance"], 1);
    assert_eq!(summary["compared_on_one_basis"], 1);
    assert_eq!(
        summary["exactly_equal"],
        json!({ "o": 1, "h": 1, "l": 1, "c": 0 })
    );
}

/// **A DAY WHOSE GDFL FILE STARTS LATE COMPARES NO OPEN, AND ITS FACTOR IS
/// THE LAST COMMON MINUTE'S, NAMED** (RELIANCE 2019-05-30: GDFL's first
/// print at 11:57:48 was set against Zerodha's 09:15 open). A factor is
/// stepped only against the page's last day measured the same way; each
/// absent source says why.
#[tokio::test]
async fn a_late_gdfl_day_compares_no_open_and_measures_its_factor_on_a_common_minute() {
    let fx = same_basis_share("late-gdfl-day", true);
    let (status, body) = fx
        .get(
            "/sources/day.json",
            "exchange=NSE&segment=CASH&symbol=SYNTHD&from=2019-03-27&to=2019-04-01",
        )
        .await;
    assert_eq!(status, axum::http::StatusCode::OK, "{body:#}");
    let days = body["days"].as_array().expect("days");
    let find = |on: &str| days.iter().find(|d| d["day"] == on).expect("the day");

    let a = find("2019-03-28");
    assert_eq!(a["open"]["gdfl_first_print_is_open"], true);
    assert_eq!(a["open"]["gdfl_vs_zerodha_day"]["flag"], false);
    assert_eq!(
        a["basis"]["factor_of"],
        "GDFL first print ÷ Zerodha day open (D-3323's measure)"
    );
    assert_eq!(a["absent"], json!({}));

    let b = find("2019-03-29");
    assert_eq!(b["open"]["gdfl_first_print_at"], "11:57:48");
    assert_eq!(b["open"]["gdfl_first_print_is_open"], false, "{b:#}");
    assert_eq!(
        b["open"]["gdfl_vs_zerodha_day"],
        json!({ "why": "gdfl_first_print_after_open" })
    );
    assert_eq!(
        b["basis"]["factor_of"],
        "GDFL ÷ Zerodha close at the last minute both hold"
    );
    assert_eq!(b["basis"]["factor"], n("1.0000"));
    assert_eq!(b["basis"]["minute_factor_at"], "12:00:00");
    assert_eq!(b["basis"]["basis_step"], Value::Null);
    assert_eq!(
        b["basis"]["basis_step_why"], "no earlier day on this page measured the same way",
        "{b:#}"
    );
    assert_eq!(b["basis"]["minute_basis_differs_from_day"], Value::Null);
    let flags = b["flags"].as_array().expect("flags");
    assert!(flags.iter().any(|f| f == "gdfl_first_print_after_open"));
    assert!(flags.iter().all(|f| f != "open_mismatch"), "{flags:#?}");

    assert_eq!(
        find("2019-03-27")["absent"],
        json!({
            "zerodha_day": "not_held",
            "zerodha_minutes": "not_held",
            "gdfl_seconds": "not_held",
        })
    );
    assert_eq!(
        find("2019-03-30")["absent"]["gdfl_seconds"],
        "closed",
        "a Saturday the calendar of record closes"
    );
    // DAY C IS STEPPED AGAINST DAY A, THE PAGE'S LAST DAY MEASURED OFF THE
    // OPEN, NOT AGAINST B.
    let c = find("2019-04-01");
    assert_eq!(c["basis"]["factor_of"], a["basis"]["factor_of"]);
    assert_eq!(c["basis"]["basis_step"], false, "{c:#}");
    assert_eq!(c["basis"]["basis_step_why"], Value::Null);
    let summary = &body["summary"];
    assert_eq!(summary["gdfl_first_print_after_open"], 1, "{summary:#}");
    assert_eq!(summary["open_equal"], 2);
}

/// **AN INDEX DAY WHOSE GDFL FILE STARTS LATE SETS NO OPEN AGAINST THE
/// OFFICIAL ONE**: the fold's open is a 10:30 print, said so, never counted
/// as a disagreement; high, low and close are still compared.
#[tokio::test]
async fn an_index_day_rung_with_a_late_gdfl_file_judges_no_open() {
    let fx = Fixture::empty("index-late-open");
    let a = day_a();
    fx.counted(
        Vendor::Zerodha,
        Segment::Index,
        "NIFTY",
        Timeframe::DAY_1,
        &[bar(
            t(a, 0, 0, 0),
            (100_000, 100_300, 99_900, 100_200),
            0,
            OI_NULL,
        )],
    );
    fx.counted(
        Vendor::Gdfl,
        Segment::Index,
        "NIFTY",
        Timeframe::SECOND_1,
        &[
            bar(
                t(a, 10, 30, 0),
                (100_150, 100_300, 99_900, 100_150),
                0,
                OI_NULL,
            ),
            bar(
                t(a, 15, 29, 59),
                (100_200, 100_200, 100_200, 100_200),
                0,
                OI_NULL,
            ),
        ],
    );
    let (status, body) = fx
        .get(
            "/sources/bars.json",
            &format!("{NIFTY}&rung=1day&from=2019-03-28"),
        )
        .await;
    assert_eq!(status, axum::http::StatusCode::OK, "{body:#}");
    let row = &body["rows"][0];
    assert_eq!(row["cells"]["gdfl"]["first_at"], "10:30:00");
    assert_eq!(
        row["cmp"]["o"]["why"], "gdfl_first_print_after_open",
        "{row:#}"
    );
    assert_eq!(row["cmp"]["o"]["flag"], Value::Null);
    assert_eq!(row["cmp"]["h"]["flag"], false);
    assert_eq!(row["cmp"]["c"]["equal"], true);
    assert_eq!(row["flags"], json!(["gdfl_first_print_after_open"]));
    assert_eq!(body["summary"]["prices_disagree"], 0);
    assert_eq!(body["summary"]["prices_within_tolerance"], 1);
}

/// The opening minute is the calendar of record's, and a day it holds no
/// window for is answered with the regular open, said to be so.
#[test]
fn the_opening_print_is_judged_against_the_calendar_of_record() {
    let a = day_a();
    assert_eq!(
        opening_minute(i64::from(a.days_from_epoch())),
        (9 * 60 + 15, true)
    );
    assert!(
        at_open(t(a, 9, 7, 59)),
        "a pre-open print is the opening one"
    );
    assert!(at_open(t(a, 9, 15, 59)));
    assert!(!at_open(t(a, 9, 16, 0)));
    assert!(!at_open(t(a, 11, 57, 48)));
    let saturday = day(2019, 3, 30);
    assert_eq!(
        opening_minute(i64::from(saturday.days_from_epoch())),
        (9 * 60 + 15, false),
        "closed: the regular open, and not the record's"
    );
}

// ══ The parts ════════════════════════════════════════════════════════════════

/// The tolerance tests are exact on the integers: 1 bp is not past 1 bp,
/// one paisa more is; 0.5% is not past 0.5%; 0.2% is not a factor, more is.
#[test]
fn every_threshold_is_exact_on_the_integers_and_never_on_a_rounded_figure() {
    assert_eq!(price_cmp(10_001, 10_000), Some((n("1.00"), false)));
    assert_eq!(price_cmp(100_011, 100_000), Some((n("1.10"), true)));
    assert_eq!(price_cmp(99_989, 100_000), Some((n("-1.10"), true)));
    assert_eq!(price_cmp(1, 0), None, "a zero base is not a price");
    assert_eq!(volume_cmp(1_005, 1_000), Some((n("0.50"), false)));
    assert_eq!(volume_cmp(1_006, 1_000), Some((n("0.60"), true)));
    assert_eq!(volume_cmp(1, 0), None);
    assert!(!basis_differs(1_002, 1_000));
    assert!(basis_differs(1_003, 1_000));
    assert!(basis_differs(997, 1_000));
    assert!(
        !factor_stepped((2_000, 1_000), (2_004, 1_000)),
        "0.2% exactly"
    );
    assert!(factor_stepped((2_000, 1_000), (2_005, 1_000)));
    assert!(factor_stepped((2_000, 1_000), (1_000, 1_000)));
    assert_eq!(ratio4(20_981, 10_000), Some(n("2.0981")));
    assert_eq!(ratio4(1, 0), None);
    assert_eq!(decimal(-5, 2), n("-0.05"));
    assert_eq!(decimal(7, 0), json!(7));
    assert_eq!(div_round(5, 2), 3);
    assert_eq!(div_round(-5, 2), -3);
    assert_eq!(div_round(4, 3), 1);
    assert_eq!(gcd(50, 75), 25);
    assert_eq!(grid_of(&[bar(0, (500, 1_000, 250, 750), 0, OI_NULL)]), 250);
    assert_eq!(grid_of(&[]), 0);
}

/// The grid is `pull::fold`'s: an hour bucket opens at 09:15 and a day at
/// IST midnight, whatever instant inside it is asked.
#[test]
fn a_bucket_is_placed_on_pull_folds_own_grid() {
    let a = day_a();
    let hour = bucket_of(Timeframe::MINUTE_60).expect("an hour");
    assert_eq!(bucket_start(t(a, 10, 14, 59), hour), Ok(t(a, 9, 15, 0)));
    assert_eq!(bucket_start(t(a, 10, 15, 0), hour), Ok(t(a, 10, 15, 0)));
    let daily = bucket_of(Timeframe::DAY_1).expect("a day");
    assert_eq!(bucket_start(t(a, 15, 29, 59), daily), Ok(t(a, 0, 0, 0)));
}

/// Folding forwards and backwards gives the same buckets, each whole, and
/// reads one record past the last bucket asked for and no more.
#[test]
fn a_fold_reads_whole_buckets_either_way_and_stops_one_record_past() {
    let fx = Fixture::nifty("fold-parts");
    let series = Series {
        inst: Instrument::parse(NIFTY).expect("NIFTY"),
        vendor: Vendor::Gdfl,
        rung: Timeframe::SECOND_1,
    };
    let held = series.held(&fx.root).expect("held");
    let range = day_bounds(day_a());
    let minute = Bucket::MINUTE;
    let mut budget = Budget::new();
    let mut rows = Rows::new(&fx.root, &series, &held, range, false);
    let (asc, exhausted) = fold_buckets(&mut rows, minute, 2, &mut budget).expect("folds");
    assert!(!exhausted);
    assert_eq!(asc.len(), 2);
    assert_eq!((asc[0].rows, asc[1].rows), (1, 2));
    assert_eq!(
        budget.used(),
        4,
        "the third bucket's first record, and no more"
    );
    let mut budget = Budget::new();
    let mut rows = Rows::new(&fx.root, &series, &held, range, true);
    let (desc, exhausted) = fold_buckets(&mut rows, minute, 10, &mut budget).expect("folds");
    assert!(exhausted);
    let starts: Vec<i64> = desc.iter().map(|f| f.bar.ts_micros).collect();
    let a = day_a();
    assert_eq!(
        starts,
        [
            t(a, 9, 17, 0),
            t(a, 9, 16, 0),
            t(a, 9, 15, 0),
            t(a, 9, 7, 0)
        ]
    );
    assert_eq!(desc[2], asc[1], "the same bucket read either way");
}

/// A request past its read budget is refused by name, never cut short.
#[test]
fn the_read_budget_refuses_by_name() {
    let mut budget = Budget { left: 1 };
    assert_eq!(budget.take(), Ok(()));
    let refusal = budget.take().expect_err("past the budget");
    assert_eq!(refusal.status, axum::http::StatusCode::UNPROCESSABLE_ENTITY);
    assert!(refusal.why.starts_with("over_budget"), "{}", refusal.why);
}

/// Months a series lacks between its first and last are its holes; the
/// months a source has that a derived rung lacks are found in one walk.
#[test]
fn holes_and_missing_months_are_counted_and_named() {
    let m = |y, mo| YearMonth::new(y, mo).expect("month");
    let (count, named) = holes_of(&[m(2019, 11), m(2020, 2), m(2020, 3)]);
    assert_eq!(count, 2);
    assert_eq!(named, ["2019-12", "2020-01"]);
    assert_eq!(holes_of(&[]), (0, Vec::new()));
    assert_eq!(
        missing_from(&[m(2019, 1), m(2019, 2), m(2019, 3)], &[m(2019, 2)]),
        [m(2019, 1), m(2019, 3)]
    );
    assert_eq!(month_of_text("2019-3"), None);
    assert_eq!(month_of_text("2019-03"), Some(m(2019, 3)));
}

/// The tags are D-3339's four, from the descriptor: no table here spells them.
#[test]
fn the_tags_are_the_four_d3339_names() {
    assert_eq!(
        tag(Vendor::Zerodha, Kind::Stored, Timeframe::MINUTE_1),
        "zerodha_stored"
    );
    assert_eq!(
        tag(Vendor::Zerodha, Kind::Derived, Timeframe::MINUTE_5),
        "zerodha_derived"
    );
    assert_eq!(
        tag(Vendor::Gdfl, Kind::Stored, Timeframe::SECOND_1),
        "gdfl_1s"
    );
    assert_eq!(
        tag(Vendor::Gdfl, Kind::Fold, Timeframe::MINUTE_1),
        "gdfl_fold"
    );
    assert_eq!(kind_of(Feed::Zerodha, Timeframe::MINUTE_1), Kind::Stored);
    assert_eq!(kind_of(Feed::Zerodha, Timeframe::DAY_1), Kind::Stored);
    assert_eq!(kind_of(Feed::Zerodha, Timeframe::MINUTE_30), Kind::Derived);
    assert_eq!(kind_of(Feed::Gdfl, Timeframe::SECOND_1), Kind::Stored);
    assert_eq!(kind_of(Feed::Gdfl, Timeframe::MINUTE_1), Kind::Unexpected);
    assert_eq!(
        offered().map(|(_, vendor)| vendor).collect::<Vec<_>>(),
        [Vendor::Gdfl, Vendor::Zerodha],
        "the operator's two, D-3338"
    );
}

/// Times print in IST, to the second.
#[test]
fn stamps_print_in_ist_to_the_second() {
    let a = day_a();
    assert_eq!(ist_stamp(t(a, 9, 7, 59)), "2019-03-28 09:07:59");
    assert_eq!(ist_clock(t(a, 15, 29, 59)), "15:29:59");
    assert_eq!(ist_day_text(t(a, 0, 0, 0)), "2019-03-28");
    assert_eq!(day_param("2019-02", "from", true), Ok(day(2019, 2, 28)));
    assert!(day_param("2019-02-30", "from", false).is_err());
}

/// A month file name with a SIGNED field is not a held month
/// (cloud-attack-law-20261016, lens 8): `2024-+3.bin` passed the length
/// check and `str::parse` read it as March 2024.
#[test]
fn a_signed_month_file_name_is_not_listed_as_a_held_month() {
    assert_eq!(month_of_text("2024-03"), YearMonth::new(2024, 3).ok());
    for bad in ["2024-+3", "+024-03", "2024-3", "02024-03", "2024-003"] {
        assert_eq!(month_of_text(bad), None, "{bad}");
    }
}

/// **A SOURCES QUESTION THAT CANNOT BE READ IS A 400 THAT NAMES THE FIELD,
/// AND A FULL STORE-READ POOL IS A 429 THAT NAMES THE TABLE** (BASE-COV-API,
/// D-7346; widened to thirteen cases by BASE-COV-API-AS2). Extremes: (1) an
/// empty symbol, (2) one of 25 bytes, (3) a contract no store names, (4) a
/// cursor that is no whole number of seconds, (5) one that overflows
/// microseconds upward (`i64::MAX` seconds) and (6) one that overflows them
/// downward (`i64::MIN`), (7) a day of four fields, (8) a month field one
/// digit wide, (9) a timeline page of no rows (`limit=0`), (10) a day-table
/// order nobody names; combined: (11) a day table asked from after its own end
/// AND in an order nobody names at once, which names the range, the first
/// field it reads, and nothing else, and (12) every slot of the store-read pool
/// taken while the rung table, the timeline and the day table are each asked a
/// good question, the timeline's a month with no end day; out of the box:
/// (13) a SIGNED year field (`%2B024`): four bytes that `str::parse` alone
/// reads as the year 24 (the lens-8 attack of cloud-attack-law-20261016, here
/// on a day rather than a month file name). The crowd runs in a child process
/// ([`crate::isolated`]): a full pool in this one would refuse every other
/// sources test of the binary while it stood. The child proves it ran by a
/// file its parent reads, never by a print: gate 23 refuses a print site it
/// does not declare.
#[tokio::test]
async fn a_sources_question_it_cannot_read_or_a_full_pool_is_said_by_name() {
    const CROWD_CHILD: &str = "BRUTEX_TEST_SOURCES_CROWD_CHILD";
    let fx = Fixture::empty("unreadable-questions");
    if let Some(proof) = std::env::var_os(CROWD_CHILD) {
        crowded_sources_answer_429(&fx).await;
        fs::write(Path::new(&proof).join("child-ran"), b"crowded").expect("the child's proof");
        return;
    }
    let long = "S".repeat(25);
    let timeline = "exchange=NSE&segment=INDEX&symbol=NIFTY&rung=1min";
    for (route, query, said) in [
        (
            "/sources.json",
            "exchange=NSE&segment=CASH&symbol=".to_owned(),
            "1 to 24 bytes",
        ),
        (
            "/sources.json",
            format!("exchange=NSE&segment=CASH&symbol={long}"),
            "1 to 24 bytes",
        ),
        (
            "/sources.json",
            "exchange=NSE&segment=FNO&symbol=NIFTY&contract=not-a-contract".to_owned(),
            "is not a contract this store can name",
        ),
        (
            "/sources/bars.json",
            "exchange=NSE&segment=INDEX&symbol=NIFTY&rung=1min&from=2024-01&to=2024-01&cursor=soon"
                .to_owned(),
            "is not a whole number of unix seconds",
        ),
        (
            "/sources/bars.json",
            format!(
                "exchange=NSE&segment=INDEX&symbol=NIFTY&rung=1min&from=2024-01&to=2024-01&cursor={}",
                i64::MAX
            ),
            "is not a whole number of unix seconds",
        ),
        (
            "/sources/bars.json",
            format!("{timeline}&from=2024-01&cursor={}", i64::MIN),
            "is not a whole number of unix seconds",
        ),
        (
            "/sources/bars.json",
            format!("{timeline}&from=2024-01-02-03"),
            "from=\"2024-01-02-03\" is not a day YYYY-MM-DD or a month YYYY-MM",
        ),
        (
            "/sources/bars.json",
            format!("{timeline}&from=2024-1"),
            "from=\"2024-1\" is not a day YYYY-MM-DD or a month YYYY-MM",
        ),
        (
            "/sources/bars.json",
            format!("{timeline}&from=2024-01&limit=0"),
            "limit=0 is not a page this timeline serves at 1min: 1 to 200",
        ),
        (
            "/sources/day.json",
            "exchange=NSE&segment=INDEX&symbol=NIFTY&from=2024-01-02&order=sideways".to_owned(),
            "order=\"sideways\" is not asc or desc",
        ),
        (
            "/sources/day.json",
            "exchange=NSE&segment=INDEX&symbol=NIFTY&from=2024-02-01&to=2024-01-01&order=sideways"
                .to_owned(),
            "from=2024-02-01 is after to=2024-01-01",
        ),
        (
            "/sources/bars.json",
            format!("{timeline}&from=%2B024-01"),
            "from=\"+024-01\" is not a day YYYY-MM-DD or a month YYYY-MM",
        ),
    ] {
        let (status, body) = fx.get(route, &query).await;
        assert_eq!(
            status,
            axum::http::StatusCode::BAD_REQUEST,
            "{route}?{query}: {body}"
        );
        assert!(
            body["error"].as_str().is_some_and(|why| why.contains(said)),
            "{route}?{query}: {body}"
        );
    }
    let proof = crate::scratch::path("sources-crowd-proof");
    let _ = fs::remove_dir_all(&proof);
    fs::create_dir_all(&proof).expect("the child's proof folder");
    let _ = crate::isolated::rerun(
        "server::sources::tests::a_sources_question_it_cannot_read_or_a_full_pool_is_said_by_name",
        &[(CROWD_CHILD, proof.as_os_str())],
    );
    assert_eq!(
        fs::read(proof.join("child-ran")).ok().as_deref(),
        Some(&b"crowded"[..]),
        "the child crowded the store-read pool and heard three 429s"
    );
    let _ = fs::remove_dir_all(&proof);
}

/// **A ROUTE THE FIXTURE DOES NOT SERVE IS REFUSED LOUDLY**, never answered
/// as one of the three it does: a typo in a test's route would otherwise ask
/// the wrong handler and pass on its answer.
#[tokio::test]
#[should_panic(expected = "not a route here: /sources/nowhere.json")]
async fn a_route_the_fixture_does_not_serve_is_refused_loudly() {
    let fx = Fixture::empty("not-a-route");
    let _ = fx.get("/sources/nowhere.json", NIFTY).await;
}

/// The child half of the test above: every slot of the store-read pool taken,
/// and each sources door asked a question it can read.
async fn crowded_sources_answer_429(fx: &Fixture) {
    let crowd = crate::detail::crowd(crate::detail::Named::StoreRead);
    for (route, query, named) in [
        (
            "/sources.json",
            "exchange=NSE&segment=INDEX&symbol=NIFTY",
            "all sources rung table",
        ),
        (
            "/sources/bars.json",
            "exchange=NSE&segment=INDEX&symbol=NIFTY&rung=1min&from=2024-01",
            "all sources timeline",
        ),
        (
            "/sources/day.json",
            "exchange=NSE&segment=INDEX&symbol=NIFTY&from=2024-01-02",
            "all sources day table",
        ),
    ] {
        let (status, body) = fx.get(route, query).await;
        assert_eq!(
            status,
            axum::http::StatusCode::TOO_MANY_REQUESTS,
            "{route}: {body}"
        );
        assert!(body.to_string().contains(named), "{route}: {body}");
    }
    drop(crowd);
}

/// **A STORE ROOT THAT IS NO FOLDER, A RUNG FOLDER THAT CANNOT BE LOOKED AT,
/// A SYMBOL A LINK MUST ESCAPE AND A FULL MEMO ARE EACH SAID**
/// (BASE-COV-API-AS2). Extremes: (1) a store root that does not exist is a
/// 503 naming the root; (2) one that is a file is a 503 saying it is no
/// directory; (3) an instrument whose folder is a FILE where its rung
/// folders go: the rung table still answers, and a note names the folder it
/// could not read; (4) every other feed is still listed beside it; (5) a
/// share named `M&M` links its calendar with the name as one escaped value,
/// `M%26M`, never two fields; (6) a memo past its cap is emptied whole and
/// then holds the newest entry; (7) a memo made at run time holds nothing;
/// (8) the memo keeps the last of two values put under one key. Combined:
/// (9) a root that is a file AND a share whose name must be escaped: the
/// root's refusal, before any link is built; (10) the cap reached AND the
/// next key already held: still emptied whole, then holds that key. Out of
/// the box: (11) a key put once more than the cap allows, `u32::MAX`, is
/// the one entry left.
#[tokio::test]
async fn a_root_no_folder_an_unlisted_rung_an_escaped_symbol_and_a_full_memo() {
    let masters = crate::scratch::path("sources-as2-masters");
    fs::create_dir_all(&masters).expect("empty offline masters");
    let ask = |root: &Path| {
        let site = Loaded::new(Site::load(&masters, root));
        async move {
            let uri: axum::http::Uri = format!("/sources.json?{SHARE}").parse().expect("URI");
            sources_json(axum::extract::State(site), uri).await
        }
    };
    // (1): no root at all.
    let missing = crate::scratch::path("sources-as2-no-root");
    let _ = fs::remove_dir_all(&missing);
    let (status, _, body) = ask(&missing).await;
    assert_eq!(
        status,
        axum::http::StatusCode::SERVICE_UNAVAILABLE,
        "{body}"
    );
    assert!(body.contains("cannot read store root"), "{body}");
    // (2), (9): a root that is a file.
    let file = crate::scratch::path("sources-as2-root-file");
    fs::write(&file, b"not a store").expect("a file");
    let (status, _, body) = ask(&file).await;
    assert_eq!(
        status,
        axum::http::StatusCode::SERVICE_UNAVAILABLE,
        "{body}"
    );
    assert!(body.contains("is not a directory"), "{body}");
    assert!(!body.contains("calendar"), "{body}");
    let _ = fs::remove_file(&file);
    // (3), (4): the share's folder is a file where its rungs go.
    let fx = Fixture::empty("as2-rung-file");
    let folder = fx.root.join("bars/zerodha/NSE/CASH/SYNTHA");
    fs::create_dir_all(folder.parent().expect("a parent")).expect("the segment folder");
    fs::write(&folder, b"not a folder").expect("a file where the rungs go");
    let (status, body) = fx.get("/sources.json", SHARE).await;
    assert_eq!(status, axum::http::StatusCode::OK, "{body:#}");
    let notes: Vec<&str> = body["notes"]
        .as_array()
        .expect("notes")
        .iter()
        .filter_map(Value::as_str)
        .collect();
    assert!(
        notes
            .iter()
            .filter(|note| note.starts_with("cannot read "))
            .any(|note| note.contains("SYNTHA")),
        "{notes:#?}"
    );
    assert!(
        body["feeds"]
            .as_array()
            .is_some_and(|feeds| feeds.len() > 1),
        "{body:#}"
    );
    // (5): M&M's calendar link.
    let fx = Fixture::empty("as2-escaped");
    let a = day_a();
    fx.counted(
        Vendor::Zerodha,
        Segment::Cash,
        "M&M",
        Timeframe::MINUTE_1,
        &[bar(
            t(a, 9, 15, 0),
            (50_000, 50_100, 49_900, 50_000),
            10,
            OI_NULL,
        )],
    );
    let (status, body) = fx
        .get("/sources.json", "exchange=NSE&segment=CASH&symbol=M%26M")
        .await;
    assert_eq!(status, axum::http::StatusCode::OK, "{body:#}");
    assert_eq!(
        row(&body["rungs"], "zerodha_stored", "1min")["sessions"]["calendar"],
        "/calendar.json?feed=zerodha&symbol=M%26M&rung=1min"
    );
    // (6)-(8), (10), (11): a memo of its own.
    let memo: Memo<u32, u32> = Memo::new();
    assert_eq!(memo.get(&1), None);
    memo.put(1, 10);
    memo.put(1, 11);
    assert_eq!(memo.get(&1), Some(11));
    for key in 2..=u32::try_from(MEMO_CAP).expect("small") {
        memo.put(key, key);
    }
    assert_eq!(memo.get(&1), Some(11), "at the cap, nothing is dropped yet");
    memo.put(1, 12);
    assert_eq!(memo.get(&1), Some(12), "emptied whole, then the newest");
    assert_eq!(memo.get(&2), None);
    for key in 2..=u32::try_from(MEMO_CAP).expect("small") {
        memo.put(key, key);
    }
    memo.put(u32::MAX, 0);
    assert_eq!(memo.get(&u32::MAX), Some(0));
    assert_eq!(memo.get(&1), None, "the one entry left is the newest");
    let _ = fs::remove_dir_all(&masters);
}

/// **THE INTEGER COMPARISONS, THE MONTH WALKS AND THE FOLD GRID ANSWER AT
/// THEIR EDGES.** The route tests reach these helpers through stores whose
/// prices, volumes and months are all well formed, so the edges below were
/// never asked of them.
///
/// Extremes: (1) a negative price against a positive one is no comparison;
/// (2) a price against zero is none; (3) a negative volume is none; (4) a
/// ratio over a zero denominator, or of a negative numerator, is none; (5) a
/// factor against a zero or negative `z` never differs; (6) a factor step
/// from a zero or negative base never steps; (7) each tolerance's exact edge
/// (1 bp, 0.5 %, 0.2 %) is within it and one unit more is beyond; (8) holes
/// past `MAX_NAMED` are all counted and only the first `MAX_NAMED` named;
/// (9) months held that the source lacks are walked past, not reported.
/// Combined: (10) two adjacent months beside a 35-month hole count 35 and
/// name 24; (11) one source month held and one missing beside two held
/// months the source never names. Out of the box: (12) an instant at the end
/// of `i64` its grid's anchor points to (`i64::MIN` for a minute, whose grid
/// starts at the open; `i64::MAX` for a day, whose grid starts at IST
/// midnight) is one `pull::fold` cannot place: a 400 in its words.
#[test]
fn the_integer_comparisons_month_walks_and_fold_grid_answer_at_their_edges() {
    let month = |y: u16, m: u8| YearMonth::new(y, m).expect("a month");
    assert_eq!(price_cmp(-1, 10_000), None, "a negative price");
    assert_eq!(price_cmp(10_000, 0), None, "against no price");
    assert_eq!(volume_cmp(-1, 1_000), None, "a negative volume");
    assert_eq!(ratio4(1, 0), None, "over nothing");
    assert_eq!(ratio4(-1, 5), None, "of a negative count");
    assert_eq!(ratio4(1, 3), Some(n("0.3333")));
    assert!(!basis_differs(1_000, 0), "against a zero z");
    assert!(!basis_differs(1_000, -10), "against a negative z");
    assert!(!factor_stepped((0, 100), (100, 100)), "from a zero base");
    assert!(
        !factor_stepped((-5, 100), (100, 100)),
        "from a negative base"
    );
    // Each tolerance's edge is within it; one unit more is beyond.
    assert_eq!(price_cmp(10_001, 10_000), Some((n("1.00"), false)));
    assert_eq!(price_cmp(10_002, 10_000), Some((n("2.00"), true)));
    assert_eq!(volume_cmp(1_005, 1_000), Some((n("0.50"), false)));
    assert_eq!(volume_cmp(1_006, 1_000), Some((n("0.60"), true)));
    assert!(!basis_differs(1_002, 1_000) && basis_differs(1_003, 1_000));
    assert!(!factor_stepped((1_000, 1_000), (1_002, 1_000)));
    assert!(factor_stepped((1_000, 1_000), (1_003, 1_000)));

    let (count, named) = holes_of(&[month(2019, 12), month(2020, 1), month(2023, 1)]);
    assert_eq!(count, 35, "February 2020 to December 2022");
    assert_eq!(named.len(), MAX_NAMED);
    assert_eq!(named.first().map(String::as_str), Some("2020-02"));
    assert_eq!(named.last().map(String::as_str), Some("2022-01"));
    assert_eq!(
        missing_from(
            &[month(2020, 1), month(2020, 3)],
            &[month(2019, 11), month(2020, 1), month(2020, 2)],
        ),
        vec![month(2020, 3)],
        "a held month the source lacks is walked past"
    );

    for (ts, secs) in [(i64::MIN, 60), (i64::MAX, 86_400)] {
        let bucket = Bucket::of_secs(secs).expect("a bucket");
        let refused = bucket_start(ts, bucket).expect_err("no grid places this instant");
        assert_eq!(refused.status, axum::http::StatusCode::BAD_REQUEST);
        assert!(
            refused
                .why
                .starts_with(&format!("pull::fold refused the instant {ts}")),
            "{}",
            refused.why
        );
    }
}

/// **A ROW IS COMPARED BY ITS FEEDS' ROLES, AND TALLIED BY WHAT IT WAS.**
/// The timeline tests always set a non-base GDFL cell first against a base
/// Zerodha one; here `compare_row` and `Tally::count` meet every other order
/// and every edge of a cell, from the feeds' own declared bases (GDFL raw,
/// Zerodha vendor-adjusted with an official day close, Dhan unverified).
///
/// Extremes: (1) two cells of one vendor are not compared at all; (2) the
/// base on the first side is not swapped; (3) when neither feed is a base the
/// cells keep their order; (4) a zero price on the base side is "not a
/// price"; (5) a volume twice the base's is flagged `v`; (6) a base with no
/// volume leaves none to compare; (7) feeds of no measured price basis are
/// "unverified"; (8) a share whose bases differ but whose closes are equal is
/// "adjusted but equal here". Combined: (9) a zero price beside a doubled
/// volume in one row names the price and flags only the volume; (10) a
/// one-vendor row and compared rows tallied together all count as rows of
/// both feeds, and none disagrees. Out of the box: (11) every price at
/// `i64::MAX` on both sides compares exactly, in integers, as equal.
#[test]
fn a_row_is_compared_by_its_feeds_roles_and_tallied_by_what_it_was() {
    let inst = Instrument {
        exchange: Exchange::Nse,
        segment: Segment::Cash,
        symbol: "INVENTEDCO".to_owned(),
        contract: None,
    };
    let source = |feed: Feed| Source {
        feed,
        kind: Kind::Stored,
        series: Series {
            inst: inst.clone(),
            vendor: feed.store_vendor().expect("every feed has a prefix"),
            rung: Timeframe::MINUTE_1,
        },
        held: Arc::from(Vec::new()),
    };
    let at = t(day_a(), 10, 0, 0);
    let cell = |prices: (i64, i64, i64, i64), volume: i64| Folded {
        bar: bar(at, prices, volume, OI_NULL),
        rows: 1,
        first: at,
    };
    let compare =
        |present: &[(&Source, Folded)]| compare_row(present, Timeframe::MINUTE_1, Segment::Cash);
    let (zerodha, gdfl, dhan) = (
        source(Feed::Zerodha),
        source(Feed::Gdfl),
        source(Feed::Dhan),
    );
    let level = (52_100, 52_300, 52_000, 52_200);

    let one_vendor = [(&zerodha, cell(level, 10)), (&zerodha, cell(level, 20))];
    let not_compared = compare(&one_vendor);
    assert_eq!(not_compared.basis, RowBasis::NotCompared);
    assert_eq!(not_compared.json, Value::Null);
    assert_eq!(not_compared.equal, None);

    let base_first = [(&zerodha, cell(level, 10)), (&gdfl, cell(level, 10))];
    let adjusted = compare(&base_first);
    assert_eq!(
        adjusted.json["base"],
        json!({ "of": "zerodha", "against": "gdfl" }),
        "the base on the first side stays the base"
    );
    assert_eq!(adjusted.basis, RowBasis::One);
    assert_eq!(adjusted.json["basis"], "adjusted_but_equal_here");

    let no_base = [
        (&gdfl, cell((0, 52_300, 52_000, 52_200), 1_000)),
        (&dhan, cell(level, 2_000)),
    ];
    let unverified = compare(&no_base);
    assert_eq!(
        unverified.json["base"],
        json!({ "of": "gdfl", "against": "dhan" }),
        "with no base the order stands"
    );
    assert_eq!(unverified.json["basis"], "unverified");
    assert_eq!(unverified.json["o"], json!({ "why": "not_a_price" }));
    assert_eq!(
        unverified.json["v"],
        json!({ "percent": n("100.00"), "flag": true, "equal": false })
    );
    assert_eq!(unverified.flags, vec!["v"], "only the volume is flagged");
    assert_eq!(unverified.disagree, 0);

    let no_volume = [(&gdfl, cell(level, 0)), (&dhan, cell(level, 2_000))];
    assert_eq!(
        compare(&no_volume).json["v"],
        json!({ "why": "no_volume_to_compare" })
    );

    let top = (i64::MAX, i64::MAX, i64::MAX, i64::MAX);
    let at_the_top = [(&gdfl, cell(top, 1)), (&dhan, cell(top, 1))];
    let exact = compare(&at_the_top);
    assert_eq!(
        exact.json["c"],
        json!({ "bps": n("0.00"), "flag": false, "equal": true, "paisa": 0 })
    );
    assert_eq!(exact.equal, Some([true; 4]));

    let mut tally = Tally::default();
    tally.count(&not_compared, &one_vendor);
    tally.count(&adjusted, &base_first);
    tally.count(&unverified, &no_base);
    let summed = tally.json();
    assert_eq!(summed["rows"], 3);
    assert_eq!(summed["both_feeds"], 3);
    assert_eq!(summed["prices_within_tolerance"], 3);
    assert_eq!(summed["prices_disagree"], 0);
}

/// **THE DAY TABLE'S QUESTION, A BASIS NO ONE MEASURED AND A MISSING DAY'S
/// REASON ARE SAID AT THEIR EDGES.** The route tests ask well-formed days of
/// held GDFL and Zerodha series, so the refusals and the reasons below were
/// never reached.
///
/// Extremes: (1) a year of two digits is no day; (2) four dash-separated
/// fields are none; (3) a month alone at `to` is its last day, the 29th of
/// a leap February; (4) February 30th is refused; (5) a thirteenth month is
/// refused; (6) `from` after `to` is refused by name; (7) an order that is
/// neither `asc` nor `desc` is refused; (8) a feed whose basis was never
/// measured says null and why; (9) no feed storing the rung, a series of no
/// month, GDFL before and after its archive, Zerodha before and after the
/// operator's window, a Sunday inside it and a weekday inside it each get
/// their own word. Combined: (10) a Sunday before GDFL's archive is "before
/// the archive" (the archive is asked before the calendar); (11) a Sunday
/// before Zerodha's window is "outside your window" (the window is asked
/// before the calendar). Out of the box: (12) the year 0000 and a year in
/// full-width digits are no days.
#[test]
fn the_day_questions_an_unmeasured_basis_and_a_missing_days_reason_are_said_at_their_edges() {
    let refused = |text: &str| day_param(text, "from", false).expect_err(text);
    assert_eq!(
        refused("24-03"),
        "from=\"24-03\" is not a day YYYY-MM-DD or a month YYYY-MM"
    );
    assert!(refused("2024-03-01-05").contains("is not a day YYYY-MM-DD"));
    assert_eq!(day_param("2024-02", "to", true), Ok(day(2024, 2, 29)));
    for text in [
        "2024-02-30",
        "2024-13",
        "0000-01",
        "\u{ff12}\u{ff10}\u{ff12}\u{ff14}-03",
    ] {
        assert!(refused(text).contains("is not a day YYYY-MM-DD"), "{text}");
    }
    let base = "exchange=NSE&segment=CASH&symbol=INVENTEDCO";
    assert_eq!(
        DayAsk::parse(&format!("{base}&from=2024-03-05&to=2024-03-04")).err(),
        Some("from=2024-03-05 is after to=2024-03-04".to_owned())
    );
    assert_eq!(
        DayAsk::parse(&format!("{base}&from=2024-03-04&order=sideways")).err(),
        Some("order=\"sideways\" is not asc or desc".to_owned())
    );

    let basis = basis_json(Feed::Dhan, Segment::Cash, Timeframe::MINUTE_1, Kind::Stored);
    for field in ["price", "volume", "adjusted_as_of", "close", "close_label"] {
        assert_eq!(basis[field], Value::Null, "{field}");
    }
    assert!(
        basis["unverified"]
            .as_str()
            .is_some_and(|why| !why.is_empty())
    );

    let inst = Instrument {
        exchange: Exchange::Nse,
        segment: Segment::Cash,
        symbol: "INVENTEDCO".to_owned(),
        contract: None,
    };
    let held = |vendor: Vendor, months: Vec<YearMonth>| -> HeldSeries {
        let series = Series {
            inst: inst.clone(),
            vendor,
            rung: Timeframe::MINUTE_1,
        };
        (series, Arc::from(months))
    };
    let march = vec![YearMonth::new(2024, 3).expect("a month")];
    let gdfl = held(Vendor::Gdfl, march.clone());
    let zerodha = held(Vendor::Zerodha, march);
    let empty = held(Vendor::Zerodha, Vec::new());
    for (series, when, want) in [
        (None, day(2024, 3, 4), "no_feed_stores_this_rung"),
        (Some(&empty), day(2024, 3, 4), "none_of_this_instrument"),
        (Some(&gdfl), day(2018, 8, 31), "before_archive"),
        (Some(&gdfl), day(2026, 9, 25), "after_archive"),
        (Some(&zerodha), day(2017, 12, 29), "outside_your_window"),
        (Some(&zerodha), day(2026, 10, 1), "outside_your_window"),
        (Some(&zerodha), day(2024, 3, 3), "closed"),
        (Some(&zerodha), day(2024, 3, 4), "not_held"),
        (Some(&gdfl), day(2018, 9, 2), "before_archive"),
        (Some(&zerodha), day(2017, 12, 31), "outside_your_window"),
    ] {
        assert_eq!(absent_why(series, when), want, "{when}");
    }
}

/// **A RETIRED KEY READ PAST THE ROUTE'S OWN CHECK IS STILL GONE, AND A
/// CHAIN IS NEVER THE RETIRED KEY** (BASE-COV-API-AS2). Every route asks
/// `refuse_retired` first, so the reader's own refusal of a retired option
/// month arrives only when the ledger lands between that check and the read;
/// `Refusal::store` must then answer the ledger's `410`, never a `503` that
/// reads as a store fault. Extremes: (1) a retired option's month read
/// through `Rows` is refused `410`, never `503`; (2) with the ledger's
/// sentence word for word; (3) before any record is read; (4) its answer is
/// the `410` JSON a page reads; (5) `contract_refused` with no colon is a
/// store fault, `503`; (6) a bare `contract_refused:` is the re-key's, `410`;
/// (7) the word anywhere but at the start is a store fault; (8) an empty
/// refusal is a `503`. Combined: (9) the ledger naming the underlying's
/// option AND an `FNO` question naming no contract: never the retired key,
/// `Ok`; (10) the retired contract asked as its own key: `refuse_retired`
/// answers the same `410`. Out of the box: (11) an index carrying the retired
/// contract, which no query can build: not refused, the segment decides
/// first.
#[test]
fn a_retired_key_read_past_the_route_is_gone_and_a_chain_is_never_the_retired_key() {
    let fx = Fixture::empty("as2-retired-read");
    let a = day_a();
    fx.bars(
        Vendor::Gdfl,
        "FNO",
        "NIFTY",
        Some(option()),
        Timeframe::SECOND_1,
        &[bar(t(a, 9, 15, 0), (1_000, 1_000, 1_000, 1_000), 75, 5_000)],
    );
    let retired = pull::gdfl_lineage::RetiredRecord {
        underlying: Symbol::new("NIFTY").expect("a symbol"),
        key: option(),
        why: pull::gdfl_lineage::RetiredWhy::StrikeImplausible,
        day: a,
        median: Some(10_000),
        days: 1,
        bars: 1,
    };
    let ledger = pull::gdfl_lineage::lineage_path(&fx.root);
    fs::create_dir_all(ledger.parent().expect("imports")).expect("imports dir");
    fs::write(&ledger, format!("{}\n", retired.line())).expect("the ledger");
    let fno = |contract: Option<Contract>| Instrument {
        exchange: Exchange::Nse,
        segment: Segment::Fno,
        symbol: "NIFTY".to_owned(),
        contract,
    };
    // (1)-(4): the reader's own refusal, the route's check passed by.
    let series = Series {
        inst: fno(Some(option())),
        vendor: Vendor::Gdfl,
        rung: Timeframe::SECOND_1,
    };
    let held = series.held(&fx.root).expect("its files are kept");
    assert_eq!(held.len(), 1);
    let mut budget = Budget::new();
    let refused = Rows::new(&fx.root, &series, &held, day_bounds(a), false)
        .all(&mut budget)
        .expect_err("the reader refuses a retired key too");
    assert_eq!(refused.status, axum::http::StatusCode::GONE);
    assert_eq!(refused.why, retired.refusal());
    assert_eq!(budget.used(), 0, "refused before any record was read");
    let (status, _, body) = refused.answer();
    assert_eq!(status, axum::http::StatusCode::GONE);
    assert_eq!(
        serde_json::from_str::<Value>(&body).expect("JSON"),
        json!({ "error": retired.refusal() })
    );
    // (5)-(8): the prefix and its colon, at their edges.
    for (why, status) in [
        ("contract_refused", 503_u16),
        ("contract_refused:", 410),
        ("contract_refused: a key the re-key retired", 410),
        (" contract_refused: a leading space", 503),
        ("", 503),
    ] {
        assert_eq!(Refusal::store(why).status.as_u16(), status, "{why:?}");
    }
    // (9)-(11): what `refuse_retired` refuses.
    assert_eq!(refuse_retired(&fx.root, &fno(None)), Ok(()));
    assert_eq!(
        refuse_retired(&fx.root, &fno(Some(option()))),
        Err(Refusal::gone(retired.refusal()))
    );
    let index_with_contract = Instrument {
        exchange: Exchange::Nse,
        segment: Segment::Index,
        symbol: "NIFTY".to_owned(),
        contract: Some(option()),
    };
    assert_eq!(refuse_retired(&fx.root, &index_with_contract), Ok(()));
}

/// Records one month of `contract` on `symbol`'s `FNO` series at `rung` in
/// `vendor`'s census, as [`Fixture::census`] records a spot series.
fn census_of_contract(
    fx: &Fixture,
    vendor: Vendor,
    (symbol, contract): (&str, Contract),
    rung: Timeframe,
    rows: &[Bar],
) {
    let first = rows.first().expect("rows").ts_micros;
    let last = rows.last().expect("rows").ts_micros;
    let civil = Day::from_days(u32::try_from(day_number(first)).expect("a day")).expect("civil");
    let path = manifest_path(&fx.root, vendor);
    let bytes = fs::read(&path).unwrap_or_default();
    let mut manifest = Manifest::open_image(vendor, &bytes).expect("manifest");
    manifest
        .record(Entry {
            key: EntryKey {
                contract: Some(contract),
                exchange: Exchange::Nse,
                segment: Segment::Fno,
                symbol: Symbol::new(symbol).expect("symbol"),
                timeframe: rung,
                month: YearMonth::new(civil.year(), civil.month()).expect("month"),
            },
            rows: u64::try_from(rows.len()).expect("rows"),
            first_ts_micros: first,
            last_ts_micros: last,
        })
        .expect("one contract month in the census");
    fs::create_dir_all(path.parent().expect("parent")).expect("manifest dir");
    fs::write(&path, manifest.image()).expect("publish the census");
    fx.stamps.set(fx.stamps.get() + 1);
    fs::File::options()
        .write(true)
        .open(&path)
        .expect("the manifest")
        .set_modified(
            std::time::SystemTime::UNIX_EPOCH
                + std::time::Duration::from_secs(1_700_000_000 + 60 * fx.stamps.get()),
        )
        .expect("a filesystem that carries modified times");
}

/// **EVERY RUNG STATE IS SAID, AND A CALENDAR THAT REFUSED IS QUOTED, NEVER
/// READ AS SESSIONS** (BASE-COV-API-AS2). The route tests' stores held only
/// rungs their feeds serve or derive, a derived rung behind by whole months,
/// censuses that were either whole or absent, and calendars that answered.
/// Extremes: (1) a Zerodha one-second folder, a rung its descriptor neither
/// serves nor derives, is tagged `zerodha_unexpected` and kind `unexpected`;
/// (2) its state is `unexpected` before any census word, though the census
/// counts none of it; (3) no calendar is asked for it: `sessions` is null;
/// (4) a one-second folder that is not stored folds to nothing: no
/// `zerodha_fold` row; (5) a derived rung a day behind its minutes inside one
/// month is `partial`, `days_behind` 1, no month missing; (6) its sessions
/// match its minutes' exactly: `sessions_missing` 0, exact; (7) a held census
/// that lacks one of a stored rung's months is `uncounted`, naming 2019-04;
/// (8) the stored rung with every month counted beside it is `held`; (9) a
/// feed whose census will not read has its rung's calendar refusal quoted
/// with its `503`, and no unheld session named; (10) that feed is
/// `unreadable` in the feed head. Combined: (11) the unreadable census AND a
/// stored one-second rung: the rung is `uncounted` and its fold rows are
/// still listed; (12) a future contract on a derived rung with its census
/// month counted: no calendar owes it a session, so `sessions_unchecked`,
/// and with no minutes beside it no `behind_source`. Out of the box: (13) a
/// census file holding twelve bytes of text.
#[tokio::test]
async fn every_rung_state_is_said_and_a_refused_calendar_is_quoted() {
    let fx = Fixture::empty("as2-rung-states");
    let (a, b, c) = (day_a(), day_b(), day_c());
    let (z, idx) = (Vendor::Zerodha, Segment::Index);
    let flat = |at: i64, price: i64| bar(at, (price, price, price, price), 0, OI_NULL);
    fx.counted(
        z,
        idx,
        "NIFTY",
        Timeframe::MINUTE_1,
        &[flat(t(a, 9, 15, 0), 100_000), flat(t(b, 9, 15, 0), 100_100)],
    );
    fx.counted(
        z,
        idx,
        "NIFTY",
        Timeframe::MINUTE_5,
        &[flat(t(a, 9, 15, 0), 100_000)],
    );
    fx.counted(
        z,
        idx,
        "NIFTY",
        Timeframe::DAY_1,
        &[flat(t(a, 0, 0, 0), 100_000)],
    );
    fx.bars(
        z,
        "INDEX",
        "NIFTY",
        None,
        Timeframe::DAY_1,
        &[flat(t(c, 0, 0, 0), 100_200)],
    );
    fx.bars(
        z,
        "INDEX",
        "NIFTY",
        None,
        Timeframe::SECOND_1,
        &[flat(t(a, 9, 15, 0), 100_000)],
    );
    fx.bars(
        Vendor::Gdfl,
        "INDEX",
        "NIFTY",
        None,
        Timeframe::SECOND_1,
        &[flat(t(a, 9, 15, 0), 100_000)],
    );
    let gdfl_census = manifest_path(&fx.root, Vendor::Gdfl);
    fs::create_dir_all(gdfl_census.parent().expect("a parent")).expect("the census folder");
    fs::write(&gdfl_census, b"not a census").expect("a census that will not read");
    let future = Contract::parse("2019-03-28-FUT").expect("a future");
    let five = [flat(t(a, 9, 15, 0), 100_500)];
    fx.bars(
        z,
        "FNO",
        "BANKNIFTY",
        Some(future),
        Timeframe::MINUTE_5,
        &five,
    );
    census_of_contract(&fx, z, ("BANKNIFTY", future), Timeframe::MINUTE_5, &five);

    let (status, body) = fx.get("/sources.json", NIFTY).await;
    assert_eq!(status, axum::http::StatusCode::OK, "{body:#}");
    let rows = &body["rungs"];
    // (1)-(4).
    let unexpected = row(rows, "zerodha_unexpected", "1s");
    assert_eq!(unexpected["kind"], "unexpected");
    assert_eq!(unexpected["state"], "unexpected", "{unexpected:#}");
    assert_eq!(unexpected["census"]["months_uncounted"], 1);
    assert_eq!(unexpected["sessions"], Value::Null);
    assert!(
        rows.as_array()
            .expect("rows")
            .iter()
            .all(|rung| rung["tag"] != "zerodha_fold"),
        "{rows:#}"
    );
    // (5), (6).
    let derived = row(rows, "zerodha_derived", "5min");
    assert_eq!(derived["state"], "partial", "{derived:#}");
    let behind = &derived["behind_source"];
    assert_eq!(behind["months_missing"], 0);
    assert_eq!(behind["days_behind"], 1);
    assert_eq!(behind["sessions_missing"], 0);
    assert_eq!(behind["sessions_missing_exact"], true);
    // (7), (8).
    let days = row(rows, "zerodha_stored", "1day");
    assert_eq!(days["state"], "uncounted", "{days:#}");
    assert_eq!(days["census"]["months_counted"], 1);
    assert_eq!(days["census"]["uncounted"], json!(["2019-04"]));
    assert_eq!(row(rows, "zerodha_stored", "1min")["state"], "held");
    // (9)-(11), (13).
    let seconds = row(rows, "gdfl_1s", "1s");
    let sessions = &seconds["sessions"];
    assert_eq!(sessions["status"], 503, "{sessions:#}");
    assert!(
        sessions["why"]
            .as_str()
            .is_some_and(|why| why.contains("census could not be read")),
        "{sessions:#}"
    );
    assert!(sessions.get("not_held").is_none(), "{sessions:#}");
    assert_eq!(seconds["state"], "uncounted");
    assert_eq!(row(rows, "gdfl_fold", "1day")["state"], "on_request");
    let gdfl = body["feeds"]
        .as_array()
        .expect("feeds")
        .iter()
        .find(|feed| feed["feed"] == "gdfl")
        .expect("gdfl");
    assert_eq!(gdfl["census"]["state"], "unreadable");
    // (12).
    let (status, contract) = fx
        .get(
            "/sources.json",
            "exchange=NSE&segment=FNO&symbol=BANKNIFTY&contract=2019-03-28-FUT",
        )
        .await;
    assert_eq!(status, axum::http::StatusCode::OK, "{contract:#}");
    let traded = row(&contract["rungs"], "zerodha_derived", "5min");
    assert_eq!(traded["state"], "sessions_unchecked", "{traded:#}");
    assert_eq!(traded["census"]["months_counted"], 1);
    assert!(traded["behind_source"].is_null());
    assert!(
        traded["sessions"]["why"]
            .as_str()
            .is_some_and(|why| why.starts_with("trade_driven")),
        "{traded:#}"
    );
    assert_eq!(
        contract["notes"],
        json!(["Global Datafeeds holds no rung of NSE-FNO-BANKNIFTY-2019-03-28-FUT in this store"])
    );
}

/// **A MINUTE RUNG WHOSE UNHELD SESSIONS PASS THE CALENDAR'S LIST IS COUNTED
/// WHOLE AND SET AGAINST A DERIVED RUNG AS A LOWER BOUND** (BASE-COV-API-AS2).
/// `/calendar.json` names at most `MAX_LISTED` unheld sessions and says when
/// it cut the list; every route test held a few days, so no list was ever cut.
/// Extremes: (1) a stored rung of 36 consecutive months, one bar each: every
/// month counted, `held`; (2) its unheld sessions outnumber what the calendar
/// lists; (3) the count is the calendar's whole count, never the list's
/// length; (4) 24 of them are named; (5) a derived rung holding only the
/// first month: 35 months missing, 24 named; (6) 1,065 days behind; (7) its
/// own calendar lists every unheld session it has, none; (8) set against a
/// cut list, the sessions it lacks are the difference of the counts, said not
/// exact and naming no day. Combined: (9) a whole derived list AND a cut
/// minute list at once: `sessions_missing_exact` false; (10) 35 months
/// missing AND `sessions_missing` 0: `partial` on the months alone. Out of the
/// box: (11) a derived unheld count far below its minutes': saturated at 0,
/// never a wrap.
#[tokio::test]
async fn a_minute_rung_whose_unheld_sessions_pass_the_calendars_list_is_a_lower_bound() {
    let fx = Fixture::empty("as2-cut-short");
    let (z, idx) = (Vendor::Zerodha, Segment::Index);
    let first_day = |month: u8| day(2019 + u16::from(month / 12), month % 12 + 1, 1);
    let flat = |on: Day| {
        bar(
            t(on, 9, 15, 0),
            (100_000, 100_000, 100_000, 100_000),
            0,
            OI_NULL,
        )
    };
    for month in 0..36_u8 {
        fx.counted(
            z,
            idx,
            "NIFTY",
            Timeframe::MINUTE_1,
            &[flat(first_day(month))],
        );
    }
    fx.counted(z, idx, "NIFTY", Timeframe::MINUTE_5, &[flat(first_day(0))]);
    let (status, body) = fx.get("/sources.json", NIFTY).await;
    assert_eq!(status, axum::http::StatusCode::OK, "{body:#}");
    // (1)-(4).
    let minutes = row(&body["rungs"], "zerodha_stored", "1min");
    assert_eq!(minutes["months_held"], 36);
    assert_eq!(minutes["census"]["months_counted"], 36);
    assert_eq!(minutes["state"], "held", "{minutes:#}");
    let sessions = &minutes["sessions"];
    let listed = u64::try_from(crate::calendar_of::record::MAX_LISTED).expect("small");
    let unheld = sessions["not_held"].as_u64().expect("a whole count");
    assert!(
        unheld > listed,
        "{unheld} unheld against a list of {listed}: {sessions:#}"
    );
    assert_eq!(
        sessions["not_held_named"].as_array().map(Vec::len),
        Some(MAX_NAMED)
    );
    // (5)-(11).
    let derived = row(&body["rungs"], "zerodha_derived", "5min");
    assert_eq!(derived["state"], "partial", "{derived:#}");
    let behind = &derived["behind_source"];
    assert_eq!(behind["months_missing"], 35);
    assert_eq!(
        behind["months_missing_named"].as_array().map(Vec::len),
        Some(MAX_NAMED)
    );
    assert_eq!(behind["days_behind"], 1_065);
    assert_eq!(behind["sessions_missing"], 0);
    assert_eq!(behind["sessions_missing_named"], json!([]));
    assert_eq!(behind["sessions_missing_exact"], false);
}

/// **A CUT-SHORT MINUTE LIST AND A LONG UNCOUNTED RUN ARE SAID AS FAR AS
/// THEY ARE KNOWN** (BASE-COV-API-AS2). Extremes: (1) a whole derived list
/// against a cut minute list is the difference of the counts, not exact,
/// naming no day; (2) both lists cut: the same; (3) minutes lacking more
/// sessions than the derived rung: 0; (4) 24 uncounted months are all named;
/// (5) 25 are all counted and the first 24 named, 2019-01 to 2020-12; (6) a
/// run of 25 months with no hole: no month absent; (7) no census at all:
/// every month uncounted, `uncounted`, 0 rows; (8) a rung row given no
/// calendar answer carries `sessions` null and no window label. Combined:
/// (9) a whole EMPTY derived list against a cut list of three: 0, not exact;
/// (10) a derived rung with no minutes beside it AND no calendar answer: no
/// `behind_source`, and `uncounted` before `sessions_unchecked`. Out of the
/// box: (11) a whole derived count of `u64::MAX` against a cut list of 0:
/// `u64::MAX`, never wrapped.
#[test]
fn a_cut_short_minute_list_and_a_long_uncounted_run_are_said_as_far_as_they_are_known() {
    let held = |count: u64, days: &[&str], whole: bool| NotHeld {
        count,
        days: days.iter().map(|day| (*day).to_owned()).collect(),
        whole,
    };
    let five = [
        "2024-01-01",
        "2024-01-02",
        "2024-01-03",
        "2024-01-04",
        "2024-01-05",
    ];
    // (1)-(3), (9), (11).
    assert_eq!(
        sessions_behind(&held(5, &five, true), &held(3, &five[..1], false)),
        (2, Vec::new(), false)
    );
    assert_eq!(
        sessions_behind(&held(600, &five, false), &held(590, &five, false)),
        (10, Vec::new(), false)
    );
    assert_eq!(
        sessions_behind(&held(1, &five[..1], true), &held(4, &five[..4], false)),
        (0, Vec::new(), false)
    );
    assert_eq!(
        sessions_behind(&held(0, &[], true), &held(3, &[], false)),
        (0, Vec::new(), false)
    );
    assert_eq!(
        sessions_behind(&held(u64::MAX, &[], true), &held(0, &[], false)),
        (u64::MAX, Vec::new(), false)
    );
    // (4)-(8), (10).
    let inst = Instrument::parse(NIFTY).expect("NIFTY");
    let run = |count: u8| -> Arc<[YearMonth]> {
        (0..count)
            .map(|n| YearMonth::new(2019 + u16::from(n / 12), n % 12 + 1).expect("a month"))
            .collect::<Vec<_>>()
            .into()
    };
    let rung = |kind: Kind, rung: Timeframe, months: Arc<[YearMonth]>| Held {
        feed: Feed::Zerodha,
        vendor: Vendor::Zerodha,
        rung,
        kind,
        months,
        span: None,
    };
    let none = MinuteSource {
        months_and_span: None,
        not_held: None,
    };
    let quiet = Sessions::default();
    let all_named = rung_row(
        &inst,
        &rung(Kind::Stored, Timeframe::MINUTE_1, run(24)),
        None,
        &none,
        &quiet,
    );
    assert_eq!(all_named["census"]["months_uncounted"], 24);
    assert_eq!(
        all_named["census"]["uncounted"].as_array().map(Vec::len),
        Some(24)
    );
    let past = rung_row(
        &inst,
        &rung(Kind::Stored, Timeframe::MINUTE_1, run(25)),
        None,
        &none,
        &quiet,
    );
    let census = &past["census"];
    assert_eq!(census["months_uncounted"], 25, "{past:#}");
    assert_eq!(census["months_counted"], 0);
    assert_eq!(census["rows"], 0);
    let named = census["uncounted"].as_array().expect("named months");
    assert_eq!(named.len(), MAX_NAMED);
    assert_eq!(named.first(), Some(&json!("2019-01")));
    assert_eq!(named.last(), Some(&json!("2020-12")));
    assert_eq!(past["months_held"], 25);
    assert_eq!(past["months_absent"], 0);
    assert_eq!(past["last_month"], "2021-01");
    assert_eq!(past["state"], "uncounted");
    assert_eq!(past["sessions"], Value::Null);
    assert!(past["outside_window"].is_null());
    let derived = rung_row(
        &inst,
        &rung(Kind::Derived, Timeframe::MINUTE_5, run(1)),
        None,
        &none,
        &quiet,
    );
    assert_eq!(derived["derived_from"], "1min");
    assert!(derived["behind_source"].is_null(), "{derived:#}");
    assert_eq!(derived["state"], "uncounted");
}

/// **A TIMELINE NOBODY HOLDS, OR A DAY NO SOURCE HOLDS, READS NOTHING AND
/// SAYS WHY** (BASE-COV-API-AS2). Every timeline test asked for days its
/// sources held. Extremes: (1) a symbol no feed holds, at `1min`: GDFL is
/// named as neither serving nor deriving the rung, with no second to fold;
/// (2) Zerodha as holding no month, with no finer rung that folds; (3) then
/// the note that no offered feed holds it; (4) at `1s`, the finest rung, the
/// roles swap: GDFL holds no month, Zerodha neither serves nor derives; (5)
/// at `1day`, the widest, GDFL again has no second to fold; (6) each answers
/// no source, no row, complete, no cursor and nothing read; (7) on a day the
/// store holds nothing of, a held fold source reads no bucket and is
/// exhausted; (8) a held stored source beside it likewise. Combined: (9) both
/// sources held AND both empty on one day: two sources, zero rows, no
/// one-source note, nothing read; (10) an index with no source at all still
/// says its volume note, after the absence notes. Out of the box: (11) a
/// symbol of 24 bytes, the longest the store files, named whole.
#[tokio::test]
async fn a_timeline_nobody_holds_or_a_day_no_source_holds_reads_nothing_and_says_why() {
    let fx = Fixture::nifty("as2-empty-timelines");
    let nobody = "exchange=NSE&segment=INDEX&symbol=NOSUCH";
    let neither = |feed: &str, rung: &str| {
        format!(
            "{feed} neither serves nor derives a {rung} rung, and holds no one-second month \
             of NSE-INDEX-NOSUCH to fold to it"
        )
    };
    let no_month = |feed: &str, rung: &str| {
        format!(
            "{feed} holds no {rung} month of NSE-INDEX-NOSUCH, and no finer stored rung of \
             it folds to {rung}"
        )
    };
    let (gdfl, zerodha) = ("Global Datafeeds", "Zerodha");
    // (1)-(6), (10).
    for (rung, from_gdfl, from_zerodha) in [
        ("1min", neither(gdfl, "1min"), no_month(zerodha, "1min")),
        ("1s", no_month(gdfl, "1s"), neither(zerodha, "1s")),
        ("1day", neither(gdfl, "1day"), no_month(zerodha, "1day")),
    ] {
        let (status, body) = fx
            .get(
                "/sources/bars.json",
                &format!("{nobody}&rung={rung}&from=2019-03-28"),
            )
            .await;
        assert_eq!(status, axum::http::StatusCode::OK, "{rung}: {body:#}");
        assert_eq!(
            body["notes"],
            json!([
                from_gdfl,
                from_zerodha,
                format!(
                    "no offered feed holds NSE-INDEX-NOSUCH at {rung} or a finer rung that folds to it"
                ),
                "an index has no traded volume (D-2802, D-3322): volume is never compared",
            ]),
            "{rung}"
        );
        assert_eq!(body["sources"], json!([]), "{rung}");
        assert_eq!(body["rows"], json!([]), "{rung}");
        assert_eq!(body["complete"], true, "{rung}");
        assert_eq!(body["next_cursor"], Value::Null, "{rung}");
        assert_eq!(body["records_read"], 0, "{rung}");
        assert_eq!(body["summary"]["rows"], 0, "{rung}");
    }
    // (11).
    let longest = "ABCDEFGHIJKLMNOPQRSTUVWX";
    let (status, body) = fx
        .get(
            "/sources/bars.json",
            &format!("exchange=NSE&segment=INDEX&symbol={longest}&rung=1min&from=2019-03-28"),
        )
        .await;
    assert_eq!(status, axum::http::StatusCode::OK, "{body:#}");
    assert_eq!(body["instrument"]["key"], format!("NSE-INDEX-{longest}"));
    assert_eq!(
        body["notes"][2],
        format!(
            "no offered feed holds NSE-INDEX-{longest} at 1min or a finer rung that folds to it"
        )
    );
    // (7)-(9): both sources held, neither holding 2019-03-27.
    let (status, body) = fx
        .get(
            "/sources/bars.json",
            &format!("{NIFTY}&rung=1min&from=2019-03-27"),
        )
        .await;
    assert_eq!(status, axum::http::StatusCode::OK, "{body:#}");
    let read: Vec<(&str, &Value, &Value)> = body["sources"]
        .as_array()
        .expect("sources")
        .iter()
        .map(|source| {
            (
                source["tag"].as_str().expect("a tag"),
                &source["buckets_read"],
                &source["exhausted"],
            )
        })
        .collect();
    assert_eq!(
        read,
        [
            ("gdfl_fold", &json!(0), &json!(true)),
            ("zerodha_stored", &json!(0), &json!(true)),
        ]
    );
    assert_eq!(body["rows"], json!([]));
    assert_eq!(body["complete"], true);
    assert_eq!(body["records_read"], 0);
    assert_eq!(
        body["notes"],
        json!(["an index has no traded volume (D-2802, D-3322): volume is never compared"])
    );
}

/// **A RECORD THAT FAILS ITS CHECKSUM, OR A MONTH CUT UNDER ITS READER, IS A
/// STORE REFUSAL THAT NAMES THE FILE** (BASE-COV-API-AS2). Every route test
/// read whole, well-formed months. Extremes: (1) a record whose bytes no
/// longer match its block's checksum: the timeline answers `503`; (2) naming
/// the file and the instant it could not locate; (3) the day table over the
/// same month answers the same refusal; (4) the rung table, which reads only
/// headers, still answers `200` with the month's first day; (5) a month cut to
/// its header under an open reader: the 73 records of the block it had
/// verified are still served; (6) the first record of the next block is
/// refused `503`; (7) naming the file and row 73; (8) every record read before
/// the refusal is counted: 74 taken from the budget. Combined: (9) the
/// corrupted record is the month's only record AND the first thing a lookup
/// touches: refused at the day's very first instant; (10) the cut leaves no
/// record on disk AND the reader still serves 73 verified ones. Out of the
/// box: (11) a month cut short while its reader holds the shared lock: the
/// lock never stops the cut.
#[tokio::test]
async fn a_record_that_fails_its_checksum_or_a_month_cut_under_its_reader_is_a_store_refusal() {
    use std::io::{Read as _, Seek as _, SeekFrom, Write as _};
    let a = day_a();
    let march = YearMonth::new(2019, 3).expect("a month");
    let minutes = Series {
        inst: Instrument::parse(NIFTY).expect("NIFTY"),
        vendor: Vendor::Zerodha,
        rung: Timeframe::MINUTE_1,
    };
    // (1)-(4), (9): one record, one byte flipped inside it.
    let fx = Fixture::empty("as2-unlocatable");
    fx.bars(
        Vendor::Zerodha,
        "INDEX",
        "NIFTY",
        None,
        Timeframe::MINUTE_1,
        &[bar(
            t(a, 9, 15, 0),
            (100_000, 100_000, 100_000, 100_000),
            0,
            OI_NULL,
        )],
    );
    let path = minutes.path(&fx.root, march).expect("the month's file");
    {
        let at = store::format::HEADER_LEN + 9;
        let mut file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .expect("the month");
        let mut byte = [0_u8; 1];
        file.seek(SeekFrom::Start(at)).expect("seek");
        file.read_exact(&mut byte).expect("read");
        byte[0] ^= 0x5A;
        file.seek(SeekFrom::Start(at)).expect("seek");
        file.write_all(&byte).expect("write");
    }
    let unlocatable = format!("{}: cannot locate {}: ", path.display(), t(a, 0, 0, 0));
    for (route, query) in [
        (
            "/sources/bars.json",
            format!("{NIFTY}&rung=1min&from=2019-03-28"),
        ),
        ("/sources/day.json", format!("{NIFTY}&from=2019-03-28")),
    ] {
        let (status, body) = fx.get(route, &query).await;
        assert_eq!(
            status,
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            "{route}: {body:#}"
        );
        assert!(
            body["error"]
                .as_str()
                .is_some_and(|why| why.starts_with(&unlocatable)),
            "{route}: {body:#}"
        );
    }
    let (status, table) = fx.get("/sources.json", NIFTY).await;
    assert_eq!(status, axum::http::StatusCode::OK, "{table:#}");
    assert_eq!(
        row(&table["rungs"], "zerodha_stored", "1min")["first_day"],
        "2019-03-28"
    );
    // (5)-(8), (10), (11): a hundred records, cut to the header under the reader.
    let cut = Fixture::empty("as2-cut-under-reader");
    let minute = |n: i64| {
        bar(
            t(a, 9, 15, 0) + n * 60_000_000,
            (100_000, 100_000, 100_000, 100_000),
            0,
            OI_NULL,
        )
    };
    let hundred: Vec<Bar> = (0..100).map(minute).collect();
    cut.bars(
        Vendor::Zerodha,
        "INDEX",
        "NIFTY",
        None,
        Timeframe::MINUTE_1,
        &hundred,
    );
    let cut_path = minutes.path(&cut.root, march).expect("the month's file");
    let held = minutes.held(&cut.root).expect("held");
    let mut budget = Budget::new();
    let mut rows = Rows::new(&cut.root, &minutes, &held, day_bounds(a), false);
    assert_eq!(
        rows.next(&mut budget).expect("the first record reads"),
        hundred.first().copied()
    );
    fs::OpenOptions::new()
        .write(true)
        .open(&cut_path)
        .expect("the month")
        .set_len(store::format::HEADER_LEN)
        .expect("cut to its header under the reader");
    let refused = rows
        .all(&mut budget)
        .expect_err("a record past the verified block is refused");
    assert_eq!(refused.status, axum::http::StatusCode::SERVICE_UNAVAILABLE);
    assert!(
        refused
            .why
            .starts_with(&format!("{} record 73: ", cut_path.display())),
        "{}",
        refused.why
    );
    assert_eq!(
        budget.used(),
        74,
        "73 records served, the 74th taken and refused"
    );
}

/// **THE DAY TABLE'S PARTS REFUSE A MISLABELLED SERIES AND SAY A SYMBOL THEY
/// CANNOT FILE** (BASE-COV-API-AS2). Extremes: (1) a day slot handed two
/// records of one IST day refuses `503`, naming the instrument, the count and
/// the day; (2) both records were read first; (3) a series no feed stores
/// reads no row and spends no budget; (4) two records one second apart across
/// IST midnight fold to two day bars: refused by name; (5) two records either
/// side of UTC midnight (05:30 IST) fold to one day bar, stamped at IST
/// midnight; (6) a 25-byte symbol's sidecar cannot be filed: unreadable, no
/// file, no ordinal, the symbol named; (7) its JSON says so and it doubles no
/// print; (8) an empty symbol likewise; (9) the same 25-byte symbol before the
/// auction began is never asked: not applicable; (10) day notes with no
/// dayflags list: three for a share, two for an index, one for a contract.
/// Combined: (11) a share's notes with an EMPTY dayflags list stay three, and
/// with one note it comes last. Out of the box: (12) a MINUTE series handed
/// to the day slot, which no route builds.
#[test]
fn the_day_tables_parts_refuse_a_mislabelled_series_and_say_an_unfiled_symbol() {
    let fx = Fixture::empty("as2-day-parts");
    let (a, b) = (day_a(), day_b());
    let flat = |at: i64, price: i64| bar(at, (price, price, price, price), 1, OI_NULL);
    fx.bars(
        Vendor::Zerodha,
        "INDEX",
        "NIFTY",
        None,
        Timeframe::MINUTE_1,
        &[flat(t(a, 9, 15, 0), 100_000), flat(t(a, 9, 16, 0), 100_100)],
    );
    let inst = Instrument::parse(NIFTY).expect("NIFTY");
    let minutes = Series {
        inst: inst.clone(),
        vendor: Vendor::Zerodha,
        rung: Timeframe::MINUTE_1,
    };
    let held = minutes.held(&fx.root).expect("held");
    // (1), (2), (12).
    let mislabelled = DaySeries {
        zerodha_day: Some((minutes.clone(), Arc::clone(&held))),
        zerodha_minutes: None,
        gdfl_seconds: None,
    };
    let mut budget = Budget::new();
    let refused = day_facts(&fx.root, &inst, &mislabelled, a, &mut budget)
        .expect_err("two records in the day slot");
    assert_eq!(refused.status, axum::http::StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(
        refused.why,
        "NSE-INDEX-NIFTY holds 2 day records on 2019-03-28; a day rung holds one"
    );
    assert_eq!(
        budget.used(),
        2,
        "both records were read before the refusal"
    );
    // (3).
    let mut untouched = Budget::new();
    assert_eq!(day_rows(&fx.root, None, a, &mut untouched), Ok(Vec::new()));
    assert_eq!(untouched.used(), 0);
    // (4), (5).
    let across = DayFold::of(
        &[
            flat(t(a, 23, 59, 59), 100_000),
            flat(t(b, 0, 0, 0), 100_100),
        ],
        Bucket::SECOND,
    )
    .expect_err("two IST days in one day fold");
    assert_eq!(across.status, axum::http::StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(
        across.why,
        "pull::fold made 2 day bars of one IST day's records"
    );
    let utc_midnight = DayFold::of(
        &[
            flat(t(a, 5, 29, 59), 100_000),
            flat(t(a, 5, 30, 0), 100_100),
        ],
        Bucket::SECOND,
    )
    .expect("one IST day")
    .expect("a fold");
    assert_eq!(
        (utc_midnight.records, utc_midnight.bar.ts_micros),
        (2, t(a, 0, 0, 0))
    );
    // (6)-(9).
    let auction_day = day(2026, 9, 22);
    let share = |symbol: &str| Instrument {
        exchange: Exchange::Nse,
        segment: Segment::Cash,
        symbol: symbol.to_owned(),
        contract: None,
    };
    let long = "S".repeat(25);
    let said = format!("{long} is not a symbol the store keys");
    let unfiled = auction_of(&fx.root, &share(&long), auction_day, &[]);
    assert_eq!(
        unfiled,
        Auction::Unreadable {
            file: String::new(),
            ordinal: None,
            why: said.clone(),
        }
    );
    assert_eq!(
        unfiled.json(),
        json!({ "state": "unreadable", "file": "", "ordinal": null, "why": said })
    );
    assert_eq!(unfiled.also_in_bars(), 0);
    assert_eq!(
        auction_of(&fx.root, &share(""), auction_day, &[]),
        Auction::Unreadable {
            file: String::new(),
            ordinal: None,
            why: " is not a symbol the store keys".to_owned(),
        }
    );
    assert_eq!(
        auction_of(&fx.root, &share(&long), day(2026, 7, 31), &[]),
        Auction::NotApplicable
    );
    // (10), (11).
    let future = Instrument {
        exchange: Exchange::Nse,
        segment: Segment::Fno,
        symbol: "NIFTY".to_owned(),
        contract: Some(option()),
    };
    let count = |of: &Instrument, flagged: Option<&Vec<Value>>| day_notes(of, flagged).len();
    assert_eq!(
        (
            count(&share("INVENTEDCO"), None),
            count(&inst, None),
            count(&future, None)
        ),
        (3, 2, 1)
    );
    let none_said: Vec<Value> = Vec::new();
    assert_eq!(count(&share("INVENTEDCO"), Some(&none_said)), 3);
    let one_said = vec![json!("a dayflags note")];
    let with = day_notes(&share("INVENTEDCO"), Some(&one_said));
    assert_eq!(with.len(), 4);
    assert_eq!(with.last(), Some(&json!("a dayflags note")));
}

/// **A DAY ROW IS JUDGED BY ITS BASES AND ITS SEGMENT AT EVERY EDGE**
/// (BASE-COV-API-AS2). Through `/sources/day.json` a share's bases always
/// differ and its measured table is always read, and no stored index day
/// carried volume, so these edges are asked of `day_row` directly.
/// Extremes: (1) a day Zerodha alone holds is counted `zerodha_only`; (2) the
/// sources a hand-built context does not hold say `no_feed_stores_this_rung`;
/// (3) a same-basis share pair whose factor is exactly 2: no
/// `basis_differs`, factor 2.0000; (4) against an earlier factor 1% away: no
/// `basis_step`, and no reason needed; (5) its open set against the official
/// open is flagged `open_mismatch` at 10000.00 bp; (6) GDFL's high above the
/// day's high is `gdfl_extremes_outside_day`; (7) a share read with no
/// measured basis table has `measured_run` null; (8) a share whose bases
/// differ and whose GDFL day traded nothing: no volume factor,
/// `volume_not_scaled_with_price` false; (9) an index day whose minutes hold
/// half its day volume never carries `minute_volume_off_day`, and the page
/// counts none. Combined: (10) bases `same` AND a factor of 2 AND a 1% move
/// on one day; (11) an index day carrying traded volume AND minutes half of
/// it. Out of the box: (12) a GDFL first print of 0 paisa: factor 0.0000,
/// `basis_differs`, never `quantised` though both grids are held, and its
/// volume never called unscaled.
#[test]
fn a_day_row_is_judged_by_its_bases_and_its_segment_at_every_edge() {
    let a = day_a();
    let none = DaySeries {
        zerodha_day: None,
        zerodha_minutes: None,
        gdfl_seconds: None,
    };
    let context = |segment: Segment, bases: Option<bool>| DayContext {
        segment,
        bases,
        series: &none,
        measured: None,
    };
    let facts = |zerodha_day: Option<Bar>,
                 zerodha_minutes: Option<DayFold>,
                 gdfl_seconds: Option<DayFold>| DayFacts {
        zerodha_day,
        zerodha_minutes,
        gdfl_seconds,
        common_minute: None,
        auction: Auction::NotApplicable,
        band: None,
        restart: None,
        records_read: 0,
    };
    let midnight =
        |prices: (i64, i64, i64, i64), volume: i64| bar(t(a, 0, 0, 0), prices, volume, OI_NULL);
    let mut tally = DayTally::default();

    // (1), (2): an index day Zerodha alone holds.
    let index_day = midnight((100_000, 100_300, 99_900, 100_200), 0);
    let alone = day_row(
        a,
        &facts(Some(index_day), None, None),
        &context(Segment::Index, None),
        None,
        &mut PreviousFactor::default(),
        &mut tally,
    );
    assert_eq!(
        alone["absent"],
        json!({
            "zerodha_minutes": "no_feed_stores_this_rung",
            "gdfl_seconds": "no_feed_stores_this_rung",
        }),
        "{alone:#}"
    );
    assert_eq!(alone["flags"], json!([]));

    // (3)-(7), (10): a same-basis pair two times apart.
    let share_day = midnight((50_000, 50_100, 49_900, 50_050), 1_000);
    let doubled = DayFold {
        bar: midnight((100_000, 100_100, 99_900, 100_050), 1_000),
        records: 2,
        first: bar(
            t(a, 9, 15, 0),
            (100_000, 100_100, 99_900, 100_000),
            400,
            OI_NULL,
        ),
        last: bar(
            t(a, 15, 29, 59),
            (100_050, 100_050, 100_050, 100_050),
            600,
            OI_NULL,
        ),
        grid: 50,
    };
    let mut earlier = PreviousFactor {
        open: Some((99_000, 50_000)),
        minute: None,
    };
    let same = day_row(
        a,
        &facts(Some(share_day), None, Some(doubled)),
        &context(Segment::Cash, Some(false)),
        None,
        &mut earlier,
        &mut tally,
    );
    let basis = &same["basis"];
    assert_eq!(basis["bases"], "same", "{same:#}");
    assert_eq!(basis["factor"], n("2.0000"));
    assert_eq!(basis["basis_differs"], false);
    assert_eq!(basis["basis_step"], false);
    assert_eq!(basis["basis_step_why"], Value::Null);
    assert_eq!(basis["measured_run"], Value::Null);
    assert_eq!(
        same["open"]["gdfl_vs_zerodha_day"],
        json!({ "bps": n("10000.00"), "flag": true })
    );
    assert_eq!(
        same["flags"],
        json!([
            "open_mismatch",
            "gdfl_extremes_outside_day",
            "official_vs_last_print"
        ])
    );
    assert_eq!(
        earlier.open,
        Some((100_000, 50_000)),
        "the day's pair is the next base"
    );

    // (8): the bases differ and GDFL traded nothing.
    let silent = DayFold {
        bar: midnight((100_000, 100_100, 99_900, 100_100), 0),
        records: 2,
        first: bar(
            t(a, 9, 15, 0),
            (100_000, 100_100, 99_900, 100_000),
            0,
            OI_NULL,
        ),
        last: bar(
            t(a, 15, 29, 59),
            (100_100, 100_100, 100_100, 100_100),
            0,
            OI_NULL,
        ),
        grid: 100,
    };
    let quiet = day_row(
        a,
        &facts(Some(share_day), None, Some(silent)),
        &context(Segment::Cash, Some(true)),
        None,
        &mut PreviousFactor::default(),
        &mut tally,
    );
    assert_eq!(quiet["basis"]["basis_differs"], true, "{quiet:#}");
    assert_eq!(quiet["basis"]["volume_factor"], Value::Null);
    assert_eq!(quiet["basis"]["volume_not_scaled_with_price"], false);
    assert_eq!(
        quiet["volume"]["gdfl_vs_zerodha_day"],
        json!({ "factor": null, "why": "basis_differs" })
    );
    assert_eq!(quiet["flags"], json!(["basis_differs"]));

    // (12): a first print of 0 paisa, with both grids held.
    let minutes = DayFold {
        bar: share_day,
        records: 1,
        first: bar(
            t(a, 9, 15, 0),
            (50_000, 50_100, 49_900, 50_050),
            1_000,
            OI_NULL,
        ),
        last: bar(
            t(a, 9, 15, 0),
            (50_000, 50_100, 49_900, 50_050),
            1_000,
            OI_NULL,
        ),
        grid: 50,
    };
    let zero_print = bar(t(a, 9, 15, 0), (0, 100_100, 0, 100_000), 5, OI_NULL);
    let free = DayFold {
        bar: midnight((0, 100_100, 0, 100_000), 5),
        records: 1,
        first: zero_print,
        last: zero_print,
        grid: 100,
    };
    let nought = day_row(
        a,
        &facts(Some(share_day), Some(minutes), Some(free)),
        &context(Segment::Cash, Some(true)),
        None,
        &mut PreviousFactor::default(),
        &mut tally,
    );
    assert_eq!(nought["basis"]["factor"], n("0.0000"), "{nought:#}");
    assert_eq!(nought["basis"]["basis_differs"], true);
    assert_eq!(nought["basis"]["quantised"], false);
    assert_eq!(nought["basis"]["volume_not_scaled_with_price"], false);
    assert_eq!(
        nought["basis"]["grid_paisa"],
        json!({ "zerodha_minutes": 50, "gdfl_seconds": 100 })
    );
    assert_eq!(nought["flags"], json!(["basis_differs"]));

    // (9), (11): an index day that traded, its minutes half of it.
    let traded = midnight((100_000, 100_300, 99_900, 100_200), 1_000);
    let half_print = bar(
        t(a, 9, 15, 0),
        (100_000, 100_300, 99_900, 100_200),
        500,
        OI_NULL,
    );
    let half = DayFold {
        bar: midnight((100_000, 100_300, 99_900, 100_200), 500),
        records: 1,
        first: half_print,
        last: half_print,
        grid: 100,
    };
    let index_volume = day_row(
        a,
        &facts(Some(traded), Some(half), None),
        &context(Segment::Index, None),
        None,
        &mut PreviousFactor::default(),
        &mut tally,
    );
    assert_eq!(
        index_volume["volume"],
        json!({ "why": "index_no_volume", "zerodha_day": 1_000, "gdfl_sum": null }),
        "{index_volume:#}"
    );
    assert_eq!(index_volume["flags"], json!([]));

    let summary = day_summary(&tally);
    assert_eq!(summary["days"], 5, "{summary:#}");
    assert_eq!(summary["zerodha_only"], 2);
    assert_eq!(summary["both_feeds"], 3);
    assert_eq!(summary["basis_differs"], 2);
    assert_eq!(summary["minute_volume_off_day"], 0);
}

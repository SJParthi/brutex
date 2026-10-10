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
    // THE FEED THAT GIVES NO SOURCE IS NAMED, AND WHY.
    assert!(
        notes.iter().any(|note| note.starts_with("Zerodha")
            && note.contains("NSE-FNO-NIFTY-2019-03-28-110000-CE")),
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

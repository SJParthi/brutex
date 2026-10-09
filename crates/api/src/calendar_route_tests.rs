#![cfg(test)]
//! `/calendar.json` through its real handler, over generated stores: the
//! calendar of record, held per feed and rung (D-3336).
#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "finite generated fixtures and exact route assertions"
)]

use super::*;
use brutex_core::instrument::{Exchange, Segment};
use brutex_core::symbol::Symbol;
use pull::manifest::{Entry, EntryKey, Manifest, manifest_path};
use serde_json::Value;
use std::fs;
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

struct Fixture {
    root: PathBuf,
    site: Loaded,
    /// How many manifest publications this fixture has stamped.
    stamps: std::cell::Cell<u64>,
}

/// Days since the epoch of a civil date.
fn epoch(year: u16, month: u8, day: u8) -> i64 {
    i64::from(
        Day::new(year, month, day)
            .expect("a real date")
            .days_from_epoch(),
    )
}

/// Micros since the epoch of `second` seconds past 09:15 IST on `day`.
fn at(day: i64, second: i64) -> i64 {
    (day * 86_400 - 19_800 + 33_300 + second) * 1_000_000
}

impl Fixture {
    fn new(name: &str) -> Self {
        let root = crate::scratch::path(name);
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

    /// Write `stamps` (one month's, ascending) for one series at `rung`, and
    /// record the month in the census with the rows the file holds.
    fn series(
        &self,
        vendor: Vendor,
        segment: Segment,
        symbol: &str,
        rung: Timeframe,
        stamps: &[i64],
    ) {
        let first = stamps.first().copied().expect("at least one bar");
        let day = Day::from_days(
            u32::try_from((first / 1_000_000 + 19_800).div_euclid(86_400)).expect("a day"),
        )
        .expect("a civil day");
        let month = YearMonth::new(day.year(), day.month()).expect("a month");
        let path = StorePath::new(PathParts {
            vendor,
            exchange: "NSE",
            segment: segment.as_str(),
            symbol,
            contract: None,
            timeframe: rung,
            month,
            file: FileKind::Bars,
        })
        .expect("fixture path");
        let hash = brutex_core::universe::fnv1a(symbol).to_le_bytes();
        let symbol_id = u32::from_le_bytes(hash[..4].try_into().expect("low32"));
        let mut file = store::file::BarFile::open_or_create(&self.root, path, symbol_id)
            .expect("generated bar file");
        let rows: Vec<store::format::Bar> = stamps
            .iter()
            .map(|&ts_micros| store::format::Bar {
                ts_micros,
                open: 100,
                high: 110,
                low: 90,
                close: 105,
                volume: 1,
                open_interest: i64::MIN,
            })
            .collect();
        file.append(&rows).expect("generated observations");
        let held = file.records();
        drop(file);
        self.census(
            vendor,
            segment,
            symbol,
            rung,
            month,
            held,
            first,
            stamps.last().copied().unwrap_or(first),
        );
    }

    /// Record one census entry, and stamp the manifest at a time of its own.
    #[expect(clippy::too_many_arguments, reason = "one census entry's fields")]
    fn census(
        &self,
        vendor: Vendor,
        segment: Segment,
        symbol: &str,
        timeframe: Timeframe,
        month: YearMonth,
        rows: u64,
        first: i64,
        last: i64,
    ) {
        let path = manifest_path(&self.root, vendor);
        let bytes = if path.exists() {
            fs::read(&path).expect("existing manifest")
        } else {
            Vec::new()
        };
        let mut manifest = Manifest::open_image(vendor, &bytes).expect("manifest");
        manifest
            .record(Entry {
                key: EntryKey {
                    contract: None,
                    exchange: Exchange::Nse,
                    segment,
                    symbol: Symbol::new(symbol).expect("symbol"),
                    timeframe,
                    month,
                },
                rows,
                first_ts_micros: first,
                last_ts_micros: last,
            })
            .expect("one month in the census");
        fs::create_dir_all(path.parent().expect("manifest parent")).expect("manifest directory");
        fs::write(&path, manifest.image()).expect("publish the census");
        // STAMPED EXPLICITLY: two publications inside one tick of the file
        // system's clock would share a stamp, and the answer kept under the
        // first would be served for the second.
        self.stamps.set(self.stamps.get() + 1);
        fs::File::options()
            .write(true)
            .open(&path)
            .expect("the manifest just published")
            .set_modified(
                std::time::SystemTime::UNIX_EPOCH
                    + std::time::Duration::from_secs(1_700_000_000)
                    + std::time::Duration::from_mins(self.stamps.get()),
            )
            .expect("a filesystem that carries modified times");
    }

    /// One daily bar per day, one month at a time.
    fn daily(&self, vendor: Vendor, segment: Segment, symbol: &str, days: &[i64]) {
        let mut by_month: std::collections::BTreeMap<(u16, u8), Vec<i64>> =
            std::collections::BTreeMap::new();
        for day in days {
            let civil = Day::from_days(u32::try_from(*day).expect("a day")).expect("civil");
            by_month
                .entry((civil.year(), civil.month()))
                .or_default()
                .push(at(*day, 0));
        }
        for stamps in by_month.values() {
            self.series(vendor, segment, symbol, Timeframe::DAY_1, stamps);
        }
    }

    async fn get(&self, query: &str) -> (axum::http::StatusCode, Value) {
        let uri = format!("/calendar.json?{query}").parse().expect("uri");
        let (status, headers, body) =
            calendar_json(axum::extract::State(Loaded::clone(&self.site)), uri).await;
        assert_eq!(headers[0].1, "application/json; charset=utf-8");
        (status, serde_json::from_str(&body).expect("calendar JSON"))
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// The names in a JSON list of strings.
fn names(list: &Value) -> Vec<&str> {
    list.as_array()
        .expect("a list")
        .iter()
        .map(|name| name.as_str().expect("a name"))
        .collect()
}

/// **THE EXCHANGE CALENDAR IS THE RECORD, HELD BY NIFTY AND BANKNIFTY, AND A
/// G-SEC INDEX VOTES FOR NOTHING.** DB-015: `NIFTYGS10YR` publishes on
/// weekends and its bars alone made 132 weekend days of 2015-16 sessions.
#[tokio::test]
async fn the_exchange_calendar_is_the_record_and_a_g_sec_index_votes_for_nothing() {
    let fixture = Fixture::new("calendar-route-record");
    let (fri, sat, sun, mon, tue) = (
        epoch(2025, 5, 2),
        epoch(2025, 5, 3),
        epoch(2025, 5, 4),
        epoch(2025, 5, 5),
        epoch(2025, 5, 6),
    );
    fixture.daily(Vendor::Zerodha, Segment::Index, "NIFTY", &[fri, mon, tue]);
    fixture.daily(
        Vendor::Zerodha,
        Segment::Index,
        "NIFTYGS10YR",
        &[sat, sun, mon],
    );

    let (status, answer) = fixture.get("feed=zerodha").await;
    assert_eq!(status, axum::http::StatusCode::OK, "{answer}");
    assert_eq!(answer["status"], "ok");
    assert_eq!(answer["rung"], "1day", "Zerodha's default: its widest rung");
    assert_eq!(answer["from"], fri);
    assert_eq!(answer["to"], tue);
    assert_eq!(
        answer["days"], "R..RR",
        "the weekend is closed whoever publishes on it"
    );
    assert_eq!(answer["held"], "10011");
    assert_eq!(answer["sessions"], 3);
    assert_eq!(answer["calendarSessions"], 3);
    assert_eq!(names(&answer["derivedFrom"]), ["NIFTY"]);
    assert_eq!(names(&answer["heldBy"]), ["NIFTY"]);
    assert_eq!(answer["owed"]["regular"], 1);
    assert_eq!(answer["minuteOwed"]["regular"], 375);

    let (status, unknown) = fixture.get("feed=notafeed").await;
    assert_eq!(status, axum::http::StatusCode::BAD_REQUEST);
    assert!(unknown.to_string().contains("notafeed"));
    for query in [
        "feed=zerodha&rung=7min",
        "feed=zerodha&rung=1day&timeframe=1min",
    ] {
        let (status, refused) = fixture.get(query).await;
        assert_eq!(
            status,
            axum::http::StatusCode::BAD_REQUEST,
            "{query}: {refused}"
        );
        assert!(refused["error"].is_string(), "{query}: {refused}");
    }
}

/// **OUTSIDE THE RECORD ONLY THE REFERENCES OBSERVE.** 2015 is before the
/// calendar of record: NIFTY's bars make each of its days a session of
/// unrecorded length, its Saturday is named, and a G-sec Sunday is closed.
#[tokio::test]
async fn outside_the_record_only_the_references_observe() {
    let fixture = Fixture::new("calendar-route-observed");
    let (fri, sat, sun, mon) = (
        epoch(2015, 2, 27),
        epoch(2015, 2, 28),
        epoch(2015, 3, 1),
        epoch(2015, 3, 2),
    );
    fixture.daily(Vendor::Zerodha, Segment::Index, "NIFTY", &[fri, sat, mon]);
    fixture.daily(Vendor::Zerodha, Segment::Index, "NIFTYGS10YR", &[sun]);
    let (status, answer) = fixture.get("feed=zerodha").await;
    assert_eq!(status, axum::http::StatusCode::OK, "{answer}");
    assert_eq!(answer["days"], "UU.U", "{answer}");
    assert_eq!(
        answer["observedWeekendSessions"]["days"][0]["date"],
        "2015-02-28"
    );
    assert_eq!(answer["observedWeekendSessions"]["count"], 1);
    assert_eq!(answer["basis"]["observed"]["days"], 3);
}

/// **GDFL ANSWERS ITS ONE-SECOND CALENDAR, NAMES A SESSION ITS IMPORT REFUSED,
/// AND WITHHOLDS A RUNG IT DOES NOT HOLD WITH THE REASON.** DB-G85: GDFL's
/// calendar was `{"sessions":0,"days":[]}` over 97 held months.
#[tokio::test]
async fn gdfl_answers_its_one_second_calendar_and_names_a_session_it_holds_no_bar_on() {
    let fixture = Fixture::new("calendar-route-gdfl");
    let (fri, tue) = (epoch(2022, 3, 4), epoch(2022, 3, 8));
    for symbol in ["NIFTY", "BANKNIFTY"] {
        let stamps: Vec<i64> = [fri, tue]
            .iter()
            .flat_map(|day| (0..30).map(move |second| at(*day, second)))
            .collect();
        fixture.series(
            Vendor::Gdfl,
            Segment::Index,
            symbol,
            Timeframe::SECOND_1,
            &stamps,
        );
    }
    let (status, answer) = fixture.get("feed=gdfl").await;
    assert_eq!(status, axum::http::StatusCode::OK, "{answer}");
    assert_eq!(answer["rung"], "1s", "GDFL holds nothing coarser");
    assert_eq!(answer["fromDate"], "2022-03-04");
    assert_eq!(answer["toDate"], "2022-03-08");
    assert_eq!(answer["days"], "R..RR");
    assert_eq!(
        answer["held"], "20002",
        "2022-03-07: a session neither holds"
    );
    assert_eq!(answer["sessionsNotHeld"]["days"][0]["date"], "2022-03-07");
    assert_eq!(answer["sessions"], 2);
    assert_eq!(answer["calendarSessions"], 3);
    assert_eq!(answer["owed"]["regular"], 22_500);
    assert!(answer["owed"]["casEligible"].is_null());
    assert_eq!(names(&answer["derivedFrom"]), ["BANKNIFTY", "NIFTY"]);
    for rung in ["1day", "1min"] {
        let (status, withheld) = fixture.get(&format!("feed=gdfl&rung={rung}")).await;
        assert_eq!(status, axum::http::StatusCode::OK, "{withheld}");
        assert_eq!(withheld["status"], "withheld", "{withheld}");
        let reason = withheld["reason"].as_str().expect("a reason");
        assert!(reason.contains(rung) && reason.contains("1s"), "{reason}");
        assert_eq!(
            withheld["days"], "",
            "never a list that reads as no sessions"
        );
    }
    let (_, seconds) = fixture.get("feed=gdfl&timeframe=1s").await;
    assert_eq!(
        seconds["days"], answer["days"],
        "rung and timeframe are one parameter"
    );
}

/// **A SERIES' CALENDAR IS HELD BY THE SERIES AND ATTESTED BY THE REFERENCES,
/// AND A NAME THE CENSUS DOES NOT HOLD IS ANSWERED, WITHHELD, AND NOT KEPT.**
#[tokio::test]
async fn a_series_calendar_is_held_by_the_series_and_an_unheld_name_is_not_kept() {
    let fixture = Fixture::new("calendar-route-series");
    fixture.daily(
        Vendor::Dhan,
        Segment::Cash,
        "ADANIENT",
        &[epoch(2025, 5, 2)],
    );
    fixture.daily(
        Vendor::Dhan,
        Segment::Index,
        "NIFTY",
        &[epoch(2025, 5, 2), epoch(2025, 5, 5)],
    );
    let (status, cash) = fixture.get("feed=dhan&symbol=ADANIENT").await;
    assert_eq!(status, axum::http::StatusCode::OK, "{cash}");
    assert_eq!(
        cash["target"]["segment"], "CASH",
        "read from cash, never from index"
    );
    assert_eq!(cash["sessions"], 1);
    assert_eq!(cash["days"], "R");
    assert_eq!(names(&cash["heldBy"]), ["ADANIENT"]);
    assert_eq!(names(&cash["derivedFrom"]), ["NIFTY"]);
    let (_, lower) = fixture.get("feed=dhan&symbol=adanient").await;
    assert_eq!(lower, cash, "a stored name's case is the census's");

    let kept = fixture.site.calendars.kept_answers();
    for name in ["NOSUCH", "NIFTY%20", "JUNK1"] {
        let (status, body) = fixture.get(&format!("feed=dhan&symbol={name}")).await;
        assert_eq!(status, axum::http::StatusCode::OK, "{name}: {body}");
        assert_eq!(body["status"], "withheld", "{name}: {body}");
        assert_eq!(body["sessions"], 0, "{name}: {body}");
        assert_eq!(
            fixture.site.calendars.kept_answers(),
            kept,
            "{name}: kept nothing"
        );
    }
}

/// **AN ANSWER IS BUILT ONCE PER FEED GENERATION.** The second request is the
/// kept answer, byte for byte, and opens nothing: it is served with the bars
/// moved aside. A landing moves the manifest's stamp, and the next request
/// builds again and sees the new day. DB-G29, DB-G48, DB-G85.
#[tokio::test]
async fn an_answer_is_kept_under_the_feeds_stamp_and_built_again_when_it_moves() {
    let fixture = Fixture::new("calendar-route-kept");
    fixture.daily(
        Vendor::Zerodha,
        Segment::Index,
        "NIFTY",
        &[epoch(2025, 5, 2)],
    );
    let (_, first) = fixture.get("feed=zerodha").await;
    assert_eq!(first["sessions"], 1);
    let bars = fixture.root.join("bars");
    let aside = fixture.root.join("bars-aside");
    fs::rename(&bars, &aside).expect("move the bars aside");
    let (status, again) = fixture.get("feed=zerodha").await;
    fs::rename(&aside, &bars).expect("put the bars back");
    assert_eq!(status, axum::http::StatusCode::OK);
    assert_eq!(again, first, "the kept answer, with nothing opened");
    fixture.daily(
        Vendor::Zerodha,
        Segment::Index,
        "BANKNIFTY",
        &[epoch(2025, 6, 2)],
    );
    let (_, landed) = fixture.get("feed=zerodha").await;
    assert_eq!(landed["toDate"], "2025-06-02", "{landed}");
    assert_eq!(names(&landed["derivedFrom"]), ["BANKNIFTY", "NIFTY"]);
}

/// **A HELD FILE THAT DOES NOT OPEN REFUSES THE ANSWER AND KEEPS NOTHING, AT
/// EVERY RUNG.** D-0695: a calendar built without a file its census holds
/// would put a holiday in the calendar that never happened.
#[tokio::test]
async fn a_held_one_second_file_that_does_not_open_refuses_the_answer() {
    let fixture = Fixture::new("calendar-route-unopened");
    let day = epoch(2022, 3, 4);
    fixture.series(
        Vendor::Gdfl,
        Segment::Index,
        "NIFTY",
        Timeframe::SECOND_1,
        &[at(day, 0), at(day, 1)],
    );
    let bars = fixture.root.join("bars");
    let aside = fixture.root.join("bars-aside");
    fs::rename(&bars, &aside).expect("move the bars aside");
    let (status, refused) = fixture.get("feed=gdfl").await;
    fs::rename(&aside, &bars).expect("put the bars back");
    assert_eq!(
        status,
        axum::http::StatusCode::SERVICE_UNAVAILABLE,
        "{refused}"
    );
    assert_eq!(refused["bars"], "unopened");
    assert_eq!(
        fixture.site.calendars.kept_answers(),
        0,
        "a refusal is not kept"
    );
    let (status, answer) = fixture.get("feed=gdfl").await;
    assert_eq!(status, axum::http::StatusCode::OK, "{answer}");
    assert_eq!(answer["sessions"], 1);
}

#[tokio::test]
async fn calendar_refuses_a_symbol_held_under_multiple_identities() {
    let fixture = Fixture::new("calendar-route-ambiguous");
    fixture.daily(Vendor::Dhan, Segment::Cash, "NIFTY", &[epoch(2025, 5, 2)]);
    fixture.daily(Vendor::Dhan, Segment::Index, "NIFTY", &[epoch(2025, 5, 5)]);
    let (status, body) = fixture.get("feed=dhan&symbol=NIFTY").await;
    assert_eq!(status, axum::http::StatusCode::CONFLICT);
    assert_eq!(body["status"], "refused");
    assert!(
        body["refusal"]
            .as_str()
            .expect("reason")
            .contains("ambiguous")
    );
    assert!(body.get("sessions").is_none());
}

/// **THE CALENDAR ROUTES DERIVE OFF THE ASYNC WORKERS, BEHIND ADMISSION.**
/// W1-api2-11, D-1443.
///
/// `calendar_json` held no `.await`: a cache miss derived every spot series on
/// a Tokio worker, and `gaps_json` derived its peer vote the same way. Read off
/// the source, because what is claimed is where the call sits.
#[test]
fn the_calendar_routes_derive_on_the_blocking_pool_behind_admission() {
    // Both handlers left `server.rs` for their own modules in WP-01.
    let source = concat!(
        include_str!("calendar_route.rs"),
        include_str!("gaps_route.rs")
    );
    let body_of = |head: &str| {
        let at = source.find(head).expect("the handler exists");
        let rest = &source[at..];
        rest[..rest.find("\n}\n").expect("the handler ends")].to_owned()
    };
    let calendar = body_of("async fn calendar_json(");
    assert!(
        calendar.contains("crate::detail::run_calendar(move ||")
            && calendar.contains("calendar_json_reading(&site, &uri, census_now_stamped)"),
        "/calendar.json derives inside the admitted blocking pool:\n{calendar}"
    );
    let gaps = body_of("async fn gaps_json(");
    let call = gaps
        .find("peer_calendar(&peers_site, &asked)")
        .expect("the peer vote, when asked for, is derived");
    let admitted = gaps
        .find("crate::detail::run_calendar(move ||")
        .expect("inside the calendar pool");
    assert!(admitted < call, "the peer vote is derived inside the pool");
    assert!(
        !gaps.contains("peer_calendar(&site, &asked)"),
        "and never inline on the worker"
    );
}

/// **A REFUSED ADMISSION IS ANSWERED, NAMED AND RETRYABLE.** Saturation is
/// 429, a join failure 503, and both say why. W1-api2-11, D-1443.
#[test]
fn a_refused_calendar_admission_names_why_and_the_bound() {
    let (status, _, body) = calendar_admission_refused(&crate::detail::RunError::Saturated);
    assert_eq!(status, axum::http::StatusCode::TOO_MANY_REQUESTS);
    assert!(
        body.contains("Saturated") && body.contains("at most 8"),
        "{body}"
    );
    let (status, _, body) =
        calendar_admission_refused(&crate::detail::RunError::Join("shut down".to_owned()));
    assert_eq!(status, axum::http::StatusCode::SERVICE_UNAVAILABLE);
    assert!(body.contains("shut down"), "{body}");
    let parsed: Value = serde_json::from_str(&body).expect("JSON");
    assert!(parsed["error"].is_string());
}

/// **THE REST OF W1-api2-11: NO STORE OR MASTERS READ ON AN ASYNC WORKER.**
/// D-1508.
///
/// `/gaps.json`'s month audits, `/folder.json`'s folder walk and
/// `/indexmap.json`'s catalogue read ran inline in their handlers. Read off the
/// source, as the calendar half is, because what is claimed is where each
/// call sits: inside the admitted closure, and nowhere outside it.
#[test]
fn the_gap_audit_folder_and_index_map_read_on_the_blocking_pool() {
    let server = include_str!("server.rs");
    // `gaps_json` and `audit_span` left `server.rs` for `gaps_route.rs` in WP-01.
    let gaps_route = include_str!("gaps_route.rs");
    let folder = include_str!("folder.rs");
    let body_of = |source: &str, head: &str| {
        let at = source.find(head).expect("the handler exists");
        let rest = &source[at..];
        rest[..rest.find("\n}\n").expect("the handler ends")].to_owned()
    };

    let handler = body_of(gaps_route, "async fn gaps_json(");
    assert!(
        handler.contains("audit_span(&site, asked, span.clone(), denominator, min_run).await"),
        "/gaps.json audits its span through the admitted helper:\n{handler}"
    );
    for call in ["audit_one(", "audit_cash_schedule("] {
        assert!(!handler.contains(call), "and calls no {call} inline");
    }
    let gaps = body_of(gaps_route, "async fn audit_span(");
    let pool = gaps
        .rfind("crate::detail::run_calendar(move ||")
        .expect("the audits are admitted to the calendar pool");
    for call in ["audit_one(", "audit_cash_schedule("] {
        let at: Vec<usize> = gaps.match_indices(call).map(|(i, _)| i).collect();
        assert!(!at.is_empty(), "/gaps.json calls {call}");
        assert!(
            at.iter().all(|&i| i > pool),
            "every {call} in /gaps.json sits inside the admitted closure:\n{gaps}"
        );
    }
    assert!(
        gaps.contains("runtime.block_on(audit_cash_schedule("),
        "the evidence reader is driven on the blocking thread, not awaited on a worker"
    );
    assert!(
        !gaps.contains("audit_cash_schedule(&site, &asked, *month).await"),
        "and is never awaited inline"
    );

    let walk = body_of(folder, "pub async fn folder_json(");
    assert!(
        walk.contains("crate::detail::run_store_read(move || answer(feed, shape, &root))"),
        "/folder.json walks the folder in the store-read pool:\n{walk}"
    );
    assert_eq!(walk.matches("answer(").count(), 1, "and nowhere else");

    let map = body_of(server, "async fn indexmap_json(");
    assert!(
        map.contains("crate::detail::run_store_read(move || indexmap_reading(&site, feed))"),
        "/indexmap.json reads its catalogue in the store-read pool:\n{map}"
    );
    assert!(
        !map.contains("Published::read") && !map.contains("masters_dir()"),
        "and never inline in the handler"
    );
}

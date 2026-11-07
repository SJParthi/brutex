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
    // SINCE WP-36 (DB-N19) the handler hands out a confirmed answer itself
    // and leads a build through `lead`, which runs the reading in the pool;
    // the handler never reads the census or builds inline.
    //
    // SINCE WP-61 (DB-B42) `calendar_json` reads the `span` asked, refuses an
    // unknown one and widens a finished answer to the record's span; the
    // build is `calendar_json_held`, which is what this pins. The wrapper
    // reads no census and builds nothing itself.
    let spanned = body_of("async fn calendar_json(");
    assert!(
        spanned.contains("calendar_json_held(site, uri).await")
            && !spanned.contains("census")
            && !spanned.contains("lead("),
        "/calendar.json's span wrapper only awaits the held answer:\n{spanned}"
    );
    let calendar = body_of("async fn calendar_json_held(");
    assert!(
        calendar.contains("lead(&site, uri, &asked, &build, now)")
            && !calendar.contains("calendar_json_reading(")
            && !calendar.contains("census_now_stamped"),
        "/calendar.json builds only through `lead`:\n{calendar}"
    );
    let lead = body_of("fn lead(");
    assert!(
        lead.contains("crate::detail::run_calendar(move ||")
            && lead.contains("calendar_json_reading(&site, &uri, census_now_stamped)"),
        "a build is read inside the admitted blocking pool:\n{lead}"
    );
    // SINCE WP-36 (DB-N19) the peer vote is kept, or built by `lead_vote`
    // inside the pool; `gaps_json` asks `voted` and never derives inline.
    let gaps = body_of("async fn gaps_json(");
    assert!(
        gaps.contains("voted(&site, &asked).await")
            && !gaps.contains("peer_calendar(")
            && !gaps.contains("peer_vote_reading("),
        "/gaps.json derives no vote inline:\n{gaps}"
    );
    let vote = body_of("fn lead_vote(");
    let admitted = vote
        .find("crate::detail::run_calendar(move ||")
        .expect("inside the calendar pool");
    let call = vote
        .find("peer_vote_reading(&site, &addressed, census_now_stamped)")
        .expect("the peer vote, when asked for, is derived");
    assert!(admitted < call, "the peer vote is derived inside the pool");
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
        .rfind("crate::detail::cancellable_in(crate::detail::Named::Calendar, move |cancel|")
        .expect(
            "the audits are admitted to the calendar pool, with the reader's cancel flag (R3-08)",
        );
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

// ══ WP-36: the cold start (DB-N19) ══════════════════════════════════════════

impl Fixture {
    /// Bound every request's wait, as the served process does.
    fn served(&self, wait: std::time::Duration) {
        self.site.calendars.builds.bound_waits(wait);
    }

    /// Hold the census cache's lock on another thread until the returned
    /// sender is dropped or sent to: anything that reads the census blocks
    /// meanwhile, and anything that does not, does not.
    fn census_held(&self) -> std::sync::mpsc::Sender<()> {
        let (release, released) = std::sync::mpsc::channel::<()>();
        let (taken, held) = std::sync::mpsc::channel::<()>();
        let site = Loaded::clone(&self.site);
        std::thread::spawn(move || {
            let _guard = site
                .census
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            taken.send(()).expect("the test waits for the lock");
            let _ = released.recv();
        });
        held.recv().expect("the census lock is taken");
        release
    }

    /// `/calendar.json` with `query`, asked first while `held` keeps the
    /// census, and so the build, from finishing, then again with the census
    /// let go, until it is answered in full: the answer, which must be a `200`.
    ///
    /// The first ask always meets `"building"`, so every host runs both paths
    /// of the loop; before, only a host whose build was slower than the
    /// request's wait did (BASE-COV-API-AS2).
    #[expect(clippy::panic, reason = "a build that never finishes fails the test")]
    async fn settled(&self, query: &str, held: std::sync::mpsc::Sender<()>) -> Value {
        let mut held = Some(held);
        for _ in 0..1_000 {
            let (status, answer) = self.get(query).await;
            if status == axum::http::StatusCode::OK {
                return answer;
            }
            assert_eq!(
                status,
                axum::http::StatusCode::ACCEPTED,
                "{query}: {answer}"
            );
            assert_eq!(answer["status"], "building", "{query}: {answer}");
            drop(held.take());
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        panic!("{query}: still building after 1,000 asks");
    }

    /// GDFL one-second bars of NIFTY and BANKNIFTY on two days of March 2022.
    fn gdfl_seconds(&self) {
        for symbol in ["NIFTY", "BANKNIFTY"] {
            let stamps: Vec<i64> = [epoch(2022, 3, 4), epoch(2022, 3, 8)]
                .iter()
                .flat_map(|day| (0..30).map(move |second| at(*day, second)))
                .collect();
            self.series(
                Vendor::Gdfl,
                Segment::Index,
                symbol,
                Timeframe::SECOND_1,
                &stamps,
            );
        }
    }
}

/// **A KEPT ANSWER IS HANDED OUT WITHOUT READING THE CENSUS, AND ONLY WHILE
/// THE ASKED FEED'S STAMP STANDS STILL.** DB-N19.
///
/// After one request, the census cache's lock is held by another thread: a
/// request that read the census would block. The next request is answered at
/// once, the same bytes. Another feed's landing does not move this feed's
/// answer either. This feed's landing does, and its next request reads again.
#[tokio::test]
async fn a_confirmed_answer_reads_no_census_and_moves_with_its_feeds_stamp() {
    let fixture = Fixture::new("calendar-route-confirmed");
    fixture.gdfl_seconds();
    let (status, first) = fixture.get("feed=gdfl&symbol=NIFTY").await;
    assert_eq!(status, axum::http::StatusCode::OK, "{first}");
    assert_eq!(fixture.site.calendars.builds.confirmed_answers(), 1);

    let release = fixture.census_held();
    let started = std::time::Instant::now();
    let (status, again) = fixture.get("feed=gdfl&symbol=NIFTY").await;
    assert_eq!(status, axum::http::StatusCode::OK);
    assert_eq!(again, first, "the confirmed answer, byte for byte");
    assert!(
        started.elapsed() < std::time::Duration::from_secs(5),
        "answered while the census lock was held: no census was read"
    );
    drop(release);

    // ANOTHER FEED'S LANDING moves only that feed's manifest.
    fixture.daily(
        Vendor::Zerodha,
        Segment::Index,
        "NIFTY",
        &[epoch(2025, 5, 2)],
    );
    let release = fixture.census_held();
    let (status, still) = fixture.get("feed=gdfl&symbol=NIFTY").await;
    drop(release);
    assert_eq!(status, axum::http::StatusCode::OK);
    assert_eq!(still, first, "another feed's landing does not move it");

    // THIS FEED'S LANDING moves it: the next request reads and sees the day.
    fixture.series(
        Vendor::Gdfl,
        Segment::Index,
        "NIFTY",
        Timeframe::SECOND_1,
        &[at(epoch(2022, 4, 4), 0)],
    );
    let (status, landed) = fixture.get("feed=gdfl&symbol=NIFTY").await;
    assert_eq!(status, axum::http::StatusCode::OK, "{landed}");
    assert_eq!(landed["toDate"], "2022-04-04", "{landed}");
}

/// **PAST THE BUDGET A REQUEST IS ANSWERED "BUILDING", AT ONCE, AND THE BUILD
/// GOES ON; THE NEXT REQUEST IS ANSWERED IN FULL.** DB-N19: the first
/// `/calendar.json?feed=gdfl&symbol=NIFTY` after a start took 6.55 s.
///
/// The census lock is held, so the build a request leads cannot finish. The
/// request is answered `202` inside a bound far below a build, says what it
/// is waiting for and when to ask again, and claims no day. Once the lock is
/// let go the build finishes, and the next request is answered in full.
#[tokio::test]
async fn past_the_budget_a_request_is_answered_building_and_the_next_in_full() {
    let fixture = Fixture::new("calendar-route-building");
    fixture.gdfl_seconds();
    fixture.served(std::time::Duration::from_millis(20));
    let release = fixture.census_held();
    let started = std::time::Instant::now();
    let (status, building) = fixture.get("feed=gdfl&symbol=NIFTY").await;
    let took = started.elapsed();
    assert_eq!(status, axum::http::StatusCode::ACCEPTED, "{building}");
    assert!(
        took < std::time::Duration::from_secs(2),
        "answered within its budget, not after the build: {took:?}"
    );
    assert_eq!(building["status"], "building");
    assert_eq!(building["feed"], "gdfl");
    assert_eq!(building["symbol"], "NIFTY");
    assert_eq!(building["rung"], Value::Null, "the feed's default");
    assert_eq!(building["by"], "request");
    assert_eq!(building["retry_after_ms"], 250);
    assert!(building.get("days").is_none(), "no day is claimed");
    assert!(
        building["reason"]
            .as_str()
            .is_some_and(|why| why.contains("ask again")),
        "{building}"
    );

    // A SECOND REQUEST joins the same build: one build, not two.
    let (status, joined) = fixture.get("feed=gdfl&symbol=NIFTY").await;
    assert_eq!(status, axum::http::StatusCode::ACCEPTED, "{joined}");
    assert_eq!(fixture.site.calendars.builds.in_flight(), 1);

    let full = fixture.settled("feed=gdfl&symbol=NIFTY", release).await;
    assert_eq!(full["status"], "ok", "{full}");
    assert_eq!(full["days"], "R..RR");
    assert_eq!(full["held"], "10001", "held by NIFTY alone");
}

/// **WHILE THE CENSUS IS READ AT START, A REQUEST FOR A KEY NOT QUEUED IS
/// ANSWERED "BUILDING" WITHOUT READING IT BESIDE IT, AND A QUEUED KEY'S
/// REQUEST WAITS ON ITS START-UP BUILD.** DB-N19.
#[tokio::test]
async fn while_the_census_is_read_at_start_a_request_reads_nothing_beside_it() {
    let fixture = Fixture::new("calendar-route-census-at-start");
    fixture.gdfl_seconds();
    fixture.served(std::time::Duration::from_millis(5));
    let builds = &fixture.site.calendars.builds;
    let queued = builds
        .queue_at_start(&(Vendor::Gdfl, None, "NIFTY".to_owned()))
        .expect("room");
    {
        let _reading = builds.reading_census_at_start();
        let (status, census) = fixture.get("feed=gdfl&symbol=BANKNIFTY").await;
        assert_eq!(status, axum::http::StatusCode::ACCEPTED, "{census}");
        assert_eq!(census["phase"], "census");
        assert_eq!(census["by"], "start-up");
        let (status, waiting) = fixture.get("feed=gdfl&symbol=NIFTY").await;
        assert_eq!(status, axum::http::StatusCode::ACCEPTED, "{waiting}");
        assert_eq!(waiting["phase"], "queued");
        assert_eq!(waiting["by"], "start-up");
    }
    // THE START-UP BUILD RUNS under the feed's stamp, as `warm` runs it, and
    // finishes with its answer.
    let entered = queued.enter(Some(calendar_route::feed_stamps(
        &fixture.root,
        Vendor::Gdfl,
    )));
    drop(entered);
    builds.finish(
        &(Vendor::Gdfl, None, "NIFTY".to_owned()),
        &queued,
        crate::calendar_of::KeptAnswer {
            status: 200,
            body: r#"{"status":"ok","from":"the start-up build"}"#.to_owned(),
        },
    );
    let (status, parked) = fixture.get("feed=gdfl&symbol=NIFTY").await;
    assert_eq!(status, axum::http::StatusCode::OK, "{parked}");
    assert_eq!(
        parked["from"], "the start-up build",
        "the request told to come back is handed the start-up build's answer"
    );
    let banknifty = fixture
        .settled("feed=gdfl&symbol=BANKNIFTY", fixture.census_held())
        .await;
    assert_eq!(banknifty["days"], "R..RR", "once read, it is built");
}

/// **THE START-UP WARM READS THE CENSUS ONCE AND BUILDS NIFTY, THE EXCHANGE
/// AND BANKNIFTY AT EVERY RUNG THEY ARE HELD AT, SO THE FIRST CLICK AFTER A
/// RESTART IS HANDED A KEPT ANSWER.** DB-N19.
#[tokio::test]
async fn the_start_up_warm_builds_what_the_first_clicks_ask_for() {
    let fixture = Fixture::new("calendar-route-warm");
    fixture.gdfl_seconds();
    fixture.daily(
        Vendor::Zerodha,
        Segment::Index,
        "NIFTY",
        &[epoch(2025, 5, 2), epoch(2025, 5, 5)],
    );
    fixture.series(
        Vendor::Zerodha,
        Segment::Index,
        "NIFTY",
        Timeframe::MINUTE_1,
        &[at(epoch(2025, 5, 2), 0)],
    );
    let warmed = calendar_route::warm(&fixture.site, &[Vendor::Zerodha, Vendor::Gdfl]);
    // Zerodha: NIFTY, the exchange and BANKNIFTY at the default rung and at
    // 1min and 1day; GDFL: the same three at the default rung and at 1s.
    assert_eq!(warmed.built, 3 + 3 * 2 + 3 + 3, "{warmed:?}");
    assert_eq!(warmed.refused, 0, "{warmed:?}");
    assert_eq!(warmed.unqueued, 0, "{warmed:?}");
    assert_eq!(warmed.journal, Ok((0, 0)), "no pull journal in the fixture");
    assert_eq!(fixture.site.calendars.builds.in_flight(), 0, "all finished");
    assert_eq!(
        fixture.site.calendars.builds.confirmed_answers(),
        15 - 3,
        "Zerodha holds no BANKNIFTY: those three are answered, withheld, and not kept"
    );

    // THE FIRST CLICK reads no census: its answer was built at start.
    fixture.served(std::time::Duration::from_millis(5));
    let release = fixture.census_held();
    for query in [
        "feed=gdfl&symbol=NIFTY",
        "feed=gdfl",
        "feed=gdfl&symbol=BANKNIFTY&rung=1s",
        "feed=zerodha&symbol=NIFTY&rung=1min",
        "feed=zerodha",
    ] {
        let (status, answer) = fixture.get(query).await;
        assert_eq!(status, axum::http::StatusCode::OK, "{query}: {answer}");
        assert_eq!(answer["status"], "ok", "{query}: {answer}");
    }
    drop(release);
}

/// **A FEED THE OPERATOR IS NOT OFFERED IS NOT WARMED.** D-3338.
#[test]
fn only_the_offered_feeds_are_warmed() {
    let feeds = calendar_route::offered_feeds();
    assert!(feeds.contains(&Vendor::Gdfl) && feeds.contains(&Vendor::Zerodha));
    assert!(!feeds.contains(&Vendor::Dhan), "{feeds:?}");
}

/// **A REQUEST'S OWN BUILD IS WAITED FOR UP TO THE BUDGET, AND A START-UP
/// BUILD STILL QUEUED BEHIND OTHERS IS NOT WAITED FOR AT ALL.** DB-N19.
///
/// With a budget far longer than either takes: the request that leads a quick
/// build is answered in full, and the request that finds its key queued at
/// start is answered `"queued"` long before that budget runs out.
#[tokio::test]
async fn a_requests_own_build_is_waited_for_and_a_queued_start_up_build_is_not() {
    let fixture = Fixture::new("calendar-route-waits");
    fixture.gdfl_seconds();
    fixture.served(std::time::Duration::from_secs(30));
    let (status, built) = fixture.get("feed=gdfl&symbol=NIFTY").await;
    assert_eq!(status, axum::http::StatusCode::OK, "{built}");
    assert_eq!(
        built["days"], "R..RR",
        "answered in full, inside its budget"
    );

    let _queued = fixture
        .site
        .calendars
        .builds
        .queue_at_start(&(Vendor::Gdfl, None, "BANKNIFTY".to_owned()))
        .expect("room");
    let started = std::time::Instant::now();
    let (status, waiting) = fixture.get("feed=gdfl&symbol=BANKNIFTY").await;
    assert_eq!(status, axum::http::StatusCode::ACCEPTED, "{waiting}");
    assert_eq!(waiting["phase"], "queued");
    assert!(
        started.elapsed() < std::time::Duration::from_secs(10),
        "answered without waiting out a 30 s budget"
    );
}

/// **A FULL BUILD MAP IS REFUSED BY NAME AND IS RETRYABLE.** DB-N19.
#[test]
fn a_full_build_map_is_refused_by_name() {
    let (status, headers, body) = calendar_route::builds_full();
    assert_eq!(status, axum::http::StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(headers[0].1, "application/json; charset=utf-8");
    let parsed: Value = serde_json::from_str(&body).expect("JSON");
    let why = parsed["error"].as_str().expect("a reason");
    assert!(
        why.contains(&crate::calendar_of::build::MAX_BUILDS.to_string()) && why.contains("retry"),
        "{why}"
    );
}

/// **THE RECORD SPAN WIDENS A HELD ANSWER AND MARKS ONE IT CANNOT, ONE RUNG
/// NAMED TWICE ALIKE IS ONE RUNG, AND A CONTRACT OR ANOTHER EXCHANGE'S
/// LISTING IS NO SERIES OF THE CALENDAR** (BASE-COV-API-AS2). Extremes:
/// (1) `span=record` over a held answer widens it past the days the feed
/// holds; (2) the widened answer still says it is ok; (3) `span=record` over
/// a WITHHELD rung is marked as not widened, never widened; (4) `span=held`
/// is the held answer byte for byte; (5) `rung` and `timeframe` naming the
/// same rung are one rung; (6) naming two rungs is refused by name; (7) an
/// option contract's month in the census is no series of the calendar;
/// (8) a BSE listing of a reference index's name is no reference. Combined:
/// (9) `span=record` AND `rung=1s&timeframe=1s` at once: widened; (10) a
/// contract AND a BSE listing beside the held references: the answer is the
/// references' alone, unchanged; (12) a feed nobody names AND the record
/// span: the 400 stands, never widened, marked as no finished answer. Out
/// of the box:
/// (11) `span=RECORD` in
/// capitals is no span this route answers.
#[tokio::test]
#[allow(
    clippy::too_many_lines,
    reason = "one test walks every span and rung its Extremes: line names, in order"
)]
async fn the_record_span_widens_or_marks_and_a_rung_named_twice_alike_is_one() {
    let fixture = Fixture::new("as2-calendar-record-span");
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
    let (status, held) = fixture.get("feed=gdfl").await;
    assert_eq!(status, axum::http::StatusCode::OK, "{held}");
    // (7), (8), (10): a contract's month and a BSE NIFTY in the census.
    let path = manifest_path(&fixture.root, Vendor::Gdfl);
    let mut manifest =
        Manifest::open_image(Vendor::Gdfl, &fs::read(&path).expect("the census")).expect("read");
    let month = YearMonth::new(2022, 3).expect("a month");
    for (exchange, segment, contract) in [
        (
            Exchange::Nse,
            Segment::Fno,
            Some(
                brutex_core::instrument::Contract::parse("2022-03-31-1700000-CE")
                    .expect("a contract"),
            ),
        ),
        (Exchange::Bse, Segment::Index, None),
    ] {
        manifest
            .record(Entry {
                key: EntryKey {
                    contract,
                    exchange,
                    segment,
                    symbol: Symbol::new("NIFTY").expect("symbol"),
                    timeframe: Timeframe::SECOND_1,
                    month,
                },
                rows: 9,
                first_ts_micros: at(fri, 0),
                last_ts_micros: at(fri, 8),
            })
            .expect("one more month");
    }
    fs::write(&path, manifest.image()).expect("publish");
    fs::File::options()
        .write(true)
        .open(&path)
        .expect("the census")
        .set_modified(std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_hours(500_000))
        .expect("a stamp");
    let (status, again) = fixture.get("feed=gdfl").await;
    assert_eq!(status, axum::http::StatusCode::OK, "{again}");
    for field in ["days", "held", "fromDate", "toDate"] {
        assert_eq!(again[field], held[field], "{field}: {again}");
    }
    assert_eq!(names(&again["derivedFrom"]), ["BANKNIFTY", "NIFTY"]);
    // (4)
    let (_, by_held) = fixture.get("feed=gdfl&span=held").await;
    assert_eq!(by_held["days"], held["days"]);
    // (1), (2), (9)
    for query in [
        "feed=gdfl&span=record",
        "feed=gdfl&span=record&rung=1s&timeframe=1s",
    ] {
        let (status, record) = fixture.get(query).await;
        assert_eq!(status, axum::http::StatusCode::OK, "{query}: {record}");
        assert_eq!(record["status"], "ok", "{record}");
        assert!(
            record["days"].as_str().map_or(0, str::len) > held["days"].as_str().map_or(0, str::len),
            "{query}: widened past the held days: {record}"
        );
    }
    // (3): a withheld rung is marked, not widened.
    let (status, marked) = fixture.get("feed=gdfl&rung=1day&span=record").await;
    assert_eq!(status, axum::http::StatusCode::OK, "{marked}");
    assert_eq!(marked["status"], "withheld", "{marked}");
    assert_eq!(marked["span"], "held", "{marked}");
    assert!(
        marked["spanNote"].as_str().is_some_and(
            |note| note.starts_with("span=record was asked and this answer is NOT widened")
        ),
        "{marked}"
    );
    assert_eq!(marked["days"], "", "never widened: {marked}");
    // (5), (6), (11)
    let (_, one) = fixture.get("feed=gdfl&rung=1s&timeframe=1s").await;
    assert_eq!(one["days"], held["days"]);
    let (status, two) = fixture.get("feed=gdfl&rung=1s&timeframe=1min").await;
    assert_eq!(status, axum::http::StatusCode::BAD_REQUEST, "{two}");
    let (status, capital) = fixture.get("feed=gdfl&span=RECORD").await;
    assert_eq!(status, axum::http::StatusCode::BAD_REQUEST, "{capital}");
    // (12): a question refused before any calendar, asked with the record
    // span: the 400 stands, never widened, and is marked as the held span
    // with the note that it is no finished answer.
    let (status, refused) = fixture.get("feed=nobody&span=record").await;
    assert_eq!(status, axum::http::StatusCode::BAD_REQUEST, "{refused}");
    assert_eq!(refused["span"], "held", "{refused}");
    assert!(
        refused["spanNote"]
            .as_str()
            .is_some_and(|note| note.contains("and this one is \"no status\"")),
        "{refused}"
    );
    assert!(refused.get("days").is_none(), "never widened: {refused}");
}

// ══ BASE-COV-API-AS2, shard G2: the warm's queue, the build map, a month that
// opened and would not be looked up, the record span's raw bodies ════════════

/// Flips one byte inside record `index` of `feed`'s `NSE/INDEX/<symbol>` file
/// at `rung` for `month`, so its checksum block no longer verifies: every
/// record of that block then refuses to read, and the file still opens (as
/// `calendar_of::tests::corrupt_record` does for NIFTY).
fn damage_record(
    root: &std::path::Path,
    feed: Vendor,
    symbol: &str,
    rung: Timeframe,
    month: YearMonth,
    index: u64,
) {
    use std::io::{Read as _, Seek as _, SeekFrom, Write as _};
    let path = StorePath::new(PathParts {
        vendor: feed,
        exchange: "NSE",
        segment: "INDEX",
        symbol,
        contract: None,
        timeframe: rung,
        month,
        file: FileKind::Bars,
    })
    .expect("a legal path")
    .to_path_buf(root);
    let at = store::format::HEADER_LEN + index * store::format::RECORD_STRIDE + 9;
    let mut file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&path)
        .expect("the bar file exists");
    let mut byte = [0_u8; 1];
    file.seek(SeekFrom::Start(at)).expect("seek");
    file.read_exact(&mut byte).expect("read");
    if let Some(one) = byte.first_mut() {
        *one ^= 0x5A;
    }
    file.seek(SeekFrom::Start(at)).expect("seek");
    file.write_all(&byte).expect("write");
}

/// One daily bar per day of `days` (one month) for `symbol` on BSE's index
/// segment, under Zerodha, and its census entry, stamped as `Fixture::census`
/// stamps one.
fn bse_daily(fixture: &Fixture, symbol: &str, days: &[i64]) {
    let stamps: Vec<i64> = days.iter().map(|day| at(*day, 0)).collect();
    let first = stamps.first().copied().expect("at least one day");
    let last = stamps.last().copied().unwrap_or(first);
    let day = Day::from_days(
        u32::try_from((first / 1_000_000 + 19_800).div_euclid(86_400)).expect("a day"),
    )
    .expect("a civil day");
    let month = YearMonth::new(day.year(), day.month()).expect("a month");
    let path = StorePath::new(PathParts {
        vendor: Vendor::Zerodha,
        exchange: "BSE",
        segment: "INDEX",
        symbol,
        contract: None,
        timeframe: Timeframe::DAY_1,
        month,
        file: FileKind::Bars,
    })
    .expect("a legal path");
    let hash = brutex_core::universe::fnv1a(symbol).to_le_bytes();
    let symbol_id = u32::from_le_bytes(hash[..4].try_into().expect("low32"));
    let mut file =
        store::file::BarFile::open_or_create(&fixture.root, path, symbol_id).expect("a bar file");
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
    file.append(&rows).expect("legal daily bars");
    let held = file.records();
    drop(file);
    let census = manifest_path(&fixture.root, Vendor::Zerodha);
    let mut manifest =
        Manifest::open_image(Vendor::Zerodha, &fs::read(&census).expect("the census"))
            .expect("read");
    manifest
        .record(Entry {
            key: EntryKey {
                contract: None,
                exchange: Exchange::Bse,
                segment: Segment::Index,
                symbol: Symbol::new(symbol).expect("symbol"),
                timeframe: Timeframe::DAY_1,
                month,
            },
            rows: held,
            first_ts_micros: first,
            last_ts_micros: last,
        })
        .expect("one more month");
    fs::write(&census, manifest.image()).expect("publish the census");
    fixture.stamps.set(fixture.stamps.get() + 1);
    fs::File::options()
        .write(true)
        .open(&census)
        .expect("the census just published")
        .set_modified(
            std::time::SystemTime::UNIX_EPOCH
                + std::time::Duration::from_secs(1_700_000_000)
                + std::time::Duration::from_mins(fixture.stamps.get()),
        )
        .expect("a filesystem that carries modified times");
}

/// **THE START-UP WARM QUEUES A KEY ONCE HOWEVER OFTEN ITS FEED IS GIVEN,
/// COUNTS EVERY ANSWER THAT IS NOT A 200, AND NAMES A PULL JOURNAL IT COULD NOT
/// WALK** (BASE-COV-API-AS2; DB-N19).
///
/// Extremes: (1) GDFL given twice: its three first answers are queued once,
/// three built, not six; (2) each, over a census that will not decode, is a
/// refusal, and all three are counted; (3) none is counted unqueued; (4) none
/// is kept or confirmed; (5) no build is left in flight; (6) the journal's walk
/// is refused naming the journal's path; (7) the feeds warmed are listed as
/// given; (8) the next request is refused the same way, naming the census.
/// Combined: (9) a feed given twice AND a census that will not decode: three
/// refusals, never six; (10) a journal that cannot be walked AND three refused
/// builds: each is reported and neither hides the other. Out of the box:
/// (11) a regular file where the journal's folder belongs.
#[tokio::test]
async fn the_start_up_warm_queues_a_key_once_counts_refusals_and_names_an_unwalked_journal() {
    let fixture = Fixture::new("as2-calendar-warm-refusals");
    let census = manifest_path(&fixture.root, Vendor::Gdfl);
    fs::create_dir_all(census.parent().expect("manifest parent")).expect("manifest directory");
    fs::write(&census, b"not a census").expect("a census that will not decode");
    // (11)
    let _ = fs::remove_dir_all(fixture.root.join("audit"));
    fs::write(
        fixture.root.join("audit"),
        b"a file where the journal's folder belongs",
    )
    .expect("a file at audit/");
    let warmed = calendar_route::warm(&fixture.site, &[Vendor::Gdfl, Vendor::Gdfl]);
    // (7)
    assert_eq!(warmed.feeds, [Vendor::Gdfl, Vendor::Gdfl]);
    // (1), (2), (3), (9)
    assert_eq!(
        (warmed.built, warmed.refused, warmed.unqueued),
        (3, 3, 0),
        "{warmed:?}"
    );
    // (6), (10)
    assert!(
        warmed
            .journal
            .as_ref()
            .is_err_and(|why| why.contains("pull.journal")),
        "{:?}",
        warmed.journal
    );
    let builds = &fixture.site.calendars.builds;
    // (5)
    assert_eq!(builds.in_flight(), 0);
    // (4)
    assert_eq!(builds.confirmed_answers(), 0);
    assert_eq!(fixture.site.calendars.kept_answers(), 0);
    // (8)
    let (status, refused) = fixture.get("feed=gdfl").await;
    assert_eq!(
        status,
        axum::http::StatusCode::SERVICE_UNAVAILABLE,
        "{refused}"
    );
    assert_eq!(refused["census"], "unreadable", "{refused}");
}

/// **A BUILD THAT HAS COUNTED PART OF WHAT IT OWES SAYS HOW FAR IT HAS GOT,
/// AND ONE THAT OWES NOTHING YET SAYS IT IS READING THE CENSUS**
/// (BASE-COV-API-AS2; DB-N19).
///
/// The build is led by the test, entered under the feed's stamp and told what
/// it owes and has counted on this thread, exactly as `Evidence::read` tells
/// it: nothing finishes it, so every request finds it running.
///
/// Extremes: (1) a running build is answered `202`; (2) phase `counting`;
/// (3) by a request; (4) one of three months counted is said in words; (5) and
/// as numbers; (6) the reading's time is given; (7) no day is claimed; (8) every
/// owed month counted and the build unfinished: three of three, still `202`.
/// Combined: (9) the symbol asked in lower case AND the same running build: the
/// same progress; (10) the build finished after every request was told to come
/// back: the next request is handed its answer. Out of the box: (11) a month
/// counted before any is owed: "it is reading the census", never "1 of 0".
#[tokio::test]
#[allow(clippy::panic, reason = "a map that will not lead fails the test")]
async fn a_build_that_has_counted_part_of_its_months_says_how_far_it_has_got() {
    use crate::calendar_of::build::{Joined, counted, owe};
    let fixture = Fixture::new("as2-calendar-counting");
    fixture.served(crate::calendar_of::build::REQUEST_WAIT);
    let builds = &fixture.site.calendars.builds;
    let key = (Vendor::Gdfl, None, "NIFTY".to_owned());
    let now = calendar_route::feed_stamps(&fixture.root, Vendor::Gdfl);
    let Joined::Lead(build) = builds.join(&key, Some(&now)) else {
        panic!("an empty map is led");
    };
    // (11)
    {
        let _entered = build.enter(Some(now.clone()));
        counted();
    }
    let (status, early) = fixture.get("feed=gdfl&symbol=NIFTY").await;
    assert_eq!(status, axum::http::StatusCode::ACCEPTED, "{early}");
    assert_eq!(early["months_counted"], 1, "{early}");
    assert_eq!(early["months_owed"], 0, "{early}");
    assert!(
        early["reason"]
            .as_str()
            .is_some_and(|why| why.contains("it is reading the census")),
        "{early}"
    );
    {
        let _entered = build.enter(Some(now.clone()));
        owe(3);
    }
    let (status, part) = fixture.get("feed=gdfl&symbol=NIFTY").await;
    // (1)
    assert_eq!(status, axum::http::StatusCode::ACCEPTED, "{part}");
    // (2), (3)
    assert_eq!(part["status"], "building");
    assert_eq!(part["phase"], "counting");
    assert_eq!(part["by"], "request");
    // (4)
    assert!(
        part["reason"].as_str().is_some_and(|why| {
            why.contains("it has counted 1 of the 3 month file(s) it reads so far")
        }),
        "{part}"
    );
    // (5)
    assert_eq!(part["months_counted"], 1);
    assert_eq!(part["months_owed"], 3);
    // (6)
    assert!(part["reading_ms"].is_u64(), "{part}");
    // (7)
    assert!(part.get("days").is_none(), "no day is claimed");
    assert_eq!(part["retry_after_ms"], 250);
    // (8)
    {
        let _entered = build.enter(Some(now.clone()));
        counted();
        counted();
    }
    let (status, whole) = fixture.get("feed=gdfl&symbol=NIFTY").await;
    assert_eq!(status, axum::http::StatusCode::ACCEPTED, "{whole}");
    assert!(
        whole["reason"]
            .as_str()
            .is_some_and(|why| why.contains("it has counted 3 of the 3")),
        "{whole}"
    );
    // (9)
    let (status, lower) = fixture.get("feed=gdfl&symbol=nifty").await;
    assert_eq!(status, axum::http::StatusCode::ACCEPTED, "{lower}");
    assert_eq!(lower["months_counted"], whole["months_counted"]);
    assert_eq!(lower["symbol"], "NIFTY");
    // (10)
    builds.finish(
        &key,
        &build,
        crate::calendar_of::KeptAnswer {
            status: 200,
            body: r#"{"status":"ok","from":"the test's build"}"#.to_owned(),
        },
    );
    let (status, parked) = fixture.get("feed=gdfl&symbol=NIFTY").await;
    assert_eq!(status, axum::http::StatusCode::OK, "{parked}");
    assert_eq!(parked["from"], "the test's build");
}

/// **A MONTH THAT OPENS AND WILL NOT BE LOOKED UP IS NAMED, AND THE ANSWER IS
/// BUILT FROM WHAT DID READ** (BASE-COV-API-AS2; D-3336, D-0695).
///
/// Extremes: (1) one reference's month opens and will not be looked up: the
/// exchange answer is still `ok`, from the other; (2) the month is named with
/// its series, month and the day whose lookup failed; (3) that series is still
/// among those the answer was derived from; (4) its month count is not
/// remembered, the healthy one is; (5) the answer is kept and handed again byte
/// for byte; (6) the days are the record's; (7) nothing is unopened: no `503`;
/// (8) asked for the damaged series alone: withheld, never a calendar of no
/// sessions, and the reason names the month that would not read rather than
/// saying the feed holds no bar. Combined: (9) the damaged series asked by name AND the healthy
/// reference attesting: still withheld, the holder decides the span; (10) the
/// rung named explicitly AND the same damage: a new key, read again, named
/// again. Out of the box: (11) one flipped byte in the first record makes every
/// lookup in the month refuse, since its checksum block holds every record.
#[tokio::test]
async fn a_month_that_opens_and_will_not_be_looked_up_is_named_and_the_rest_is_answered() {
    let fixture = Fixture::new("as2-calendar-unreadable-month");
    let (fri, tue) = (epoch(2022, 3, 4), epoch(2022, 3, 8));
    let stamps: Vec<i64> = [fri, tue]
        .iter()
        .flat_map(|day| (0..30).map(move |second| at(*day, second)))
        .collect();
    for symbol in ["NIFTY", "BANKNIFTY"] {
        fixture.series(
            Vendor::Gdfl,
            Segment::Index,
            symbol,
            Timeframe::SECOND_1,
            &stamps,
        );
    }
    let month = YearMonth::new(2022, 3).expect("a month");
    // (11)
    damage_record(
        &fixture.root,
        Vendor::Gdfl,
        "BANKNIFTY",
        Timeframe::SECOND_1,
        month,
        0,
    );
    let (status, answer) = fixture.get("feed=gdfl").await;
    // (1), (7)
    assert_eq!(status, axum::http::StatusCode::OK, "{answer}");
    assert_eq!(answer["status"], "ok", "{answer}");
    // (2)
    let unreadable = answer["unreadable"].as_array().expect("a list");
    assert_eq!(unreadable.len(), 1, "{answer}");
    let named = unreadable[0].as_str().expect("a sentence");
    assert!(
        named.starts_with(&format!(
            "BANKNIFTY: {month}: the lookup for day {} failed: ",
            epoch(2022, 3, 1)
        )),
        "{named}"
    );
    // (3)
    assert_eq!(names(&answer["derivedFrom"]), ["BANKNIFTY", "NIFTY"]);
    // (6)
    assert_eq!(answer["days"], "R..RR");
    // (4)
    assert_eq!(fixture.site.calendars.remembered_months(), 1);
    // (5)
    assert_eq!(fixture.site.calendars.kept_answers(), 1);
    let (_, again) = fixture.get("feed=gdfl").await;
    assert_eq!(again, answer);
    // (10)
    let (status, explicit) = fixture.get("feed=gdfl&rung=1s").await;
    assert_eq!(status, axum::http::StatusCode::OK, "{explicit}");
    assert_eq!(explicit["unreadable"], answer["unreadable"]);
    assert_eq!(explicit["rungAsked"], true);
    // (8), (9)
    let (status, alone) = fixture.get("feed=gdfl&symbol=BANKNIFTY").await;
    assert_eq!(status, axum::http::StatusCode::OK, "{alone}");
    assert_eq!(alone["status"], "withheld", "{alone}");
    assert_eq!(alone["days"], "", "never a list that reads as no sessions");
    let why = alone["reason"].as_str().unwrap_or_default();
    assert!(why.contains("BANKNIFTY"), "{alone}");
    // (8) THE READ FAILURE IS SAID, NEVER "NO BAR": the month that would not
    // read is named in the reason.
    assert!(
        why.contains("holds no readable 1s bar for BANKNIFTY: 1 month(s) could not be read (")
            && why.contains("BANKNIFTY: 2022-03: the lookup for day "),
        "{alone}"
    );
    assert!(!why.contains("holds no 1s bar"), "{alone}");
}

/// **THE RECORD SPAN ANSWERS A KEPT BODY IT CANNOT READ AS AN OBJECT
/// UNCHANGED, AND MARKS ONE IT CAN** (BASE-COV-API-AS2; DB-B42).
///
/// Every body the route builds is an object; a kept answer is handed out as
/// kept, so the bodies below are confirmed directly, under the feed's stamp.
///
/// Extremes: (1) a JSON array, status `200`: unchanged; (2) a body no JSON
/// parser reads, status `503`: unchanged; (3) a bare JSON string: unchanged;
/// (4) JSON `null`: unchanged; (5) the status stays as kept; (6) the content
/// type is JSON's; (7) `span=held` over the array: unchanged too; (8) an object
/// with status ok and no days: marked as not widened, never widened. Combined:
/// (9) `span=record` AND a rung asked AND an array body: unchanged; (10) a
/// `200` array: the widening refused AND the marking refused. Out of the box:
/// (11) an empty body.
#[tokio::test]
async fn the_record_span_answers_a_kept_body_it_cannot_read_as_an_object_unchanged() {
    let fixture = Fixture::new("as2-calendar-record-raw-bodies");
    let builds = &fixture.site.calendars.builds;
    let now = calendar_route::feed_stamps(&fixture.root, Vendor::Gdfl);
    for (symbol, rung, status, body) in [
        ("", None, 200, "[1,2]"),
        ("NIFTY", None, 503, "not json"),
        ("BANKNIFTY", None, 200, "\"ok\""),
        ("NULL", None, 200, "null"),
        ("EMPTY", None, 200, ""),
        ("", Some(Timeframe::SECOND_1), 200, "[3]"),
        ("OBJECT", None, 200, r#"{"status":"ok"}"#),
    ] {
        builds.confirm(
            (Vendor::Gdfl, rung, symbol.to_owned()),
            now.clone(),
            std::sync::Arc::new(crate::calendar_of::KeptAnswer {
                status,
                body: body.to_owned(),
            }),
        );
    }
    let ask = |query: &str| {
        let uri: axum::http::Uri = format!("/calendar.json?{query}").parse().expect("uri");
        calendar_json(axum::extract::State(Loaded::clone(&fixture.site)), uri)
    };
    for (query, status, body) in [
        // (1), (10)
        ("feed=gdfl&span=record", 200, "[1,2]"),
        // (2), (5)
        ("feed=gdfl&symbol=NIFTY&span=record", 503, "not json"),
        // (3)
        ("feed=gdfl&symbol=BANKNIFTY&span=record", 200, "\"ok\""),
        // (4)
        ("feed=gdfl&symbol=NULL&span=record", 200, "null"),
        // (11)
        ("feed=gdfl&symbol=EMPTY&span=record", 200, ""),
        // (9)
        ("feed=gdfl&rung=1s&span=record", 200, "[3]"),
        // (7)
        ("feed=gdfl&span=held", 200, "[1,2]"),
    ] {
        let (answered, headers, said) = ask(query).await;
        assert_eq!(answered.as_u16(), status, "{query}");
        assert_eq!(said, body, "{query}: answered as kept");
        // (6)
        assert_eq!(headers[0].1, "application/json; charset=utf-8", "{query}");
    }
    // (8)
    let (_, _, marked) = ask("feed=gdfl&symbol=OBJECT&span=record").await;
    let marked: Value = serde_json::from_str(&marked).expect("JSON");
    assert_eq!(marked["span"], "held", "{marked}");
    assert!(
        marked["spanNote"]
            .as_str()
            .is_some_and(|note| note.contains("this one is \"ok\"")),
        "{marked}"
    );
    assert!(marked.get("days").is_none(), "never widened: {marked}");
}

/// **THE READING REFUSES A MALFORMED QUESTION BEFORE IT READS THE CENSUS, AND
/// KEEPS NOTHING ITS FEED'S STAMP CANNOT VOUCH FOR** (BASE-COV-API-AS2; D-0695,
/// D-3336).
///
/// Extremes: (1) a feed nobody names is refused, naming it; (2) a rung no
/// directory holds is refused, naming it; (3) two rungs named are refused;
/// (4) a rung no directory holds, spelled as `timeframe`, is refused the same
/// way (a question with no feed is not malformed: it asks the default feed);
/// (5) the census is read for none of
/// them; (6) a held census with no reference index is answered withheld,
/// naming both; (7) under a stamp that found no manifest it is not kept;
/// (8) and not confirmed. Combined: (9) the same reading under its own stamps
/// is kept AND confirmed, byte for byte the same body; (10) a feed nobody names
/// AND a bad rung: the feed is refused first. Out of the box: (11) a stamp that
/// says "no manifest" under a census that read one: a first manifest installed
/// inside the read.
#[test]
fn the_reading_refuses_a_malformed_question_itself_and_keeps_nothing_its_stamp_cannot_vouch_for() {
    let fixture = Fixture::new("as2-calendar-reading-direct");
    let day = epoch(2025, 5, 2);
    fixture.census(
        Vendor::Gdfl,
        Segment::Cash,
        "RELIANCE",
        Timeframe::DAY_1,
        YearMonth::new(2025, 5).expect("a month"),
        1,
        at(day, 0),
        at(day, 0),
    );
    let uri = |query: &str| -> axum::http::Uri {
        format!("/calendar.json?{query}").parse().expect("uri")
    };
    let asked = std::cell::Cell::new(0_u32);
    for (query, says, not) in [
        // (1)
        ("feed=nobody", "nobody", ""),
        // (2)
        ("feed=gdfl&rung=7min", "7min", ""),
        // (3)
        ("feed=gdfl&rung=1s&timeframe=1min", "two rungs", ""),
        // (4)
        ("feed=gdfl&timeframe=7min", "7min\\\" is not a rung", ""),
        // (10)
        ("feed=nobody&rung=7min", "nobody", "7min"),
    ] {
        let (status, _, body) = calendar_json_reading(&fixture.site, &uri(query), |site| {
            asked.set(asked.get() + 1);
            census_now_stamped(site)
        });
        assert_eq!(
            status,
            axum::http::StatusCode::BAD_REQUEST,
            "{query}: {body}"
        );
        assert!(body.contains(says), "{query}: {body}");
        assert!(not.is_empty() || !body.contains(not), "{query}: {body}");
    }
    // (5)
    assert_eq!(asked.get(), 0, "no census is read for a malformed question");
    // (11)
    let unvouched = |site: &Site| {
        let (mut stamps, now) = census_now_stamped(site);
        if let Some(at) = stamps.manifests.get_mut(Vendor::Gdfl as usize) {
            *at = ManifestStamp::Missing;
        }
        (stamps, now)
    };
    let (status, _, body) = calendar_json_reading(&fixture.site, &uri("feed=gdfl"), unvouched);
    // (6)
    assert_eq!(status, axum::http::StatusCode::OK, "{body}");
    let parsed: Value = serde_json::from_str(&body).expect("JSON");
    assert_eq!(parsed["status"], "withheld", "{parsed}");
    assert!(
        parsed["reason"]
            .as_str()
            .is_some_and(|why| why.contains("holds neither NIFTY nor BANKNIFTY")),
        "{parsed}"
    );
    // (7)
    assert_eq!(fixture.site.calendars.kept_answers(), 0);
    // (8)
    assert_eq!(fixture.site.calendars.builds.confirmed_answers(), 0);
    // (9)
    let (status, _, vouched) =
        calendar_json_reading(&fixture.site, &uri("feed=gdfl"), census_now_stamped);
    assert_eq!(status, axum::http::StatusCode::OK);
    assert_eq!(vouched, body);
    assert_eq!(fixture.site.calendars.kept_answers(), 1);
    assert_eq!(fixture.site.calendars.builds.confirmed_answers(), 1);
}

/// **A BSE SERIES ASKED BY NAME IS ITS OWN CALENDAR AND NO REFERENCE, AND A
/// REFERENCE THAT DOES NOT HOLD THE CHOSEN RUNG ATTESTS NOTHING**
/// (BASE-COV-API-AS2; D-3336).
///
/// Extremes: (1) the exchange's default rung is the daily one NIFTY holds;
/// (2) BANKNIFTY, held only in minutes, attests nothing at it; (3) and holds
/// nothing of the answer; (4) asked at `1min` the roles swap: BANKNIFTY holds;
/// (5) the daily NIFTY attests nothing at `1min`; (6) a BSE series asked by
/// name is the target, under its own exchange; (7) it is held by itself;
/// (8) it is attested by the NSE reference holding its rung, never by itself.
/// Combined: (9) a BSE series AND a reference without its rung: derived from
/// NIFTY alone; (10) the BSE series asked at a rung it does not hold: withheld,
/// naming the rung. Out of the box: (11) the BSE-only name asked in lower case
/// is the same answer.
#[tokio::test]
async fn a_bse_series_is_its_own_calendar_and_a_reference_without_the_rung_attests_nothing() {
    let fixture = Fixture::new("as2-calendar-rungs-and-bse");
    let (fri, mon) = (epoch(2025, 5, 2), epoch(2025, 5, 5));
    fixture.daily(Vendor::Zerodha, Segment::Index, "NIFTY", &[fri, mon]);
    fixture.series(
        Vendor::Zerodha,
        Segment::Index,
        "BANKNIFTY",
        Timeframe::MINUTE_1,
        &[at(fri, 0)],
    );
    let (status, exchange) = fixture.get("feed=zerodha").await;
    assert_eq!(status, axum::http::StatusCode::OK, "{exchange}");
    // (1)
    assert_eq!(exchange["rung"], "1day", "{exchange}");
    assert_eq!(exchange["days"], "R..R", "{exchange}");
    // (2), (3)
    assert_eq!(names(&exchange["derivedFrom"]), ["NIFTY"]);
    assert_eq!(names(&exchange["heldBy"]), ["NIFTY"]);
    let (status, minute) = fixture.get("feed=zerodha&rung=1min").await;
    assert_eq!(status, axum::http::StatusCode::OK, "{minute}");
    // (4), (5)
    assert_eq!(names(&minute["heldBy"]), ["BANKNIFTY"]);
    assert_eq!(names(&minute["derivedFrom"]), ["BANKNIFTY"]);
    bse_daily(&fixture, "SENSEX", &[fri]);
    let (status, sensex) = fixture.get("feed=zerodha&symbol=SENSEX").await;
    assert_eq!(status, axum::http::StatusCode::OK, "{sensex}");
    // (6)
    assert_eq!(sensex["target"]["exchange"], "BSE", "{sensex}");
    assert_eq!(sensex["target"]["segment"], "INDEX");
    assert_eq!(sensex["target"]["symbol"], "SENSEX");
    assert_eq!(sensex["rung"], "1day");
    // (7)
    assert_eq!(names(&sensex["heldBy"]), ["SENSEX"]);
    // (8), (9)
    assert_eq!(names(&sensex["derivedFrom"]), ["NIFTY"]);
    assert_eq!(sensex["days"], "R");
    // (10)
    let (status, unheld) = fixture.get("feed=zerodha&symbol=SENSEX&rung=1min").await;
    assert_eq!(status, axum::http::StatusCode::OK, "{unheld}");
    assert_eq!(unheld["status"], "withheld", "{unheld}");
    assert!(
        unheld["reason"]
            .as_str()
            .is_some_and(|why| why.contains("1min") && why.contains("SENSEX")),
        "{unheld}"
    );
    // (11)
    let (_, lower) = fixture.get("feed=zerodha&symbol=sensex").await;
    assert_eq!(lower, sensex);
}

/// **A FULL BUILD MAP UNQUEUES THE START-UP WARM'S ANSWERS AND ANSWERS A
/// REQUEST `429`, UNTIL A BUILD FINISHES** (BASE-COV-API-AS2; DB-N19).
///
/// Extremes: (1) with the map full, one more start-up build is refused;
/// (2) each of the warm's three first answers is counted unqueued; (3) the warm
/// builds nothing; (4) it still reads the census and walks the journal;
/// (5) a request finds no room: `429`; (6) the refusal names the bound;
/// (7) and asks for a retry; (8) a finished build makes room: the next request
/// is answered `200`. Combined: (9) a full map AND a feed with no census:
/// `429`, never the absent census's withheld answer; (10) a full map AND the
/// warm: an unqueued answer is no refusal. Out of the box: (11) the map filled
/// by builds of a feed no operator is offered, under names no census holds.
#[tokio::test]
async fn a_full_build_map_unqueues_the_warm_and_answers_a_request_429() {
    use crate::calendar_of::build::MAX_BUILDS;
    let fixture = Fixture::new("as2-calendar-full-map");
    let builds = &fixture.site.calendars.builds;
    // (11)
    let held: Vec<(
        crate::calendar_of::AnswerKey,
        std::sync::Arc<crate::calendar_of::build::Build>,
    )> = (0..MAX_BUILDS)
        .map(|n| {
            let key = (Vendor::Dhan, None, format!("HELD{n}"));
            let build = builds.queue_at_start(&key).expect("room for it");
            (key, build)
        })
        .collect();
    // (1)
    assert_eq!(builds.in_flight(), MAX_BUILDS);
    assert!(
        builds
            .queue_at_start(&(Vendor::Dhan, None, "ONEMORE".to_owned()))
            .is_none()
    );
    // (2), (3), (10)
    let warmed = calendar_route::warm(&fixture.site, &[Vendor::Gdfl]);
    assert_eq!(
        (warmed.built, warmed.unqueued, warmed.refused),
        (0, 3, 0),
        "{warmed:?}"
    );
    // (4)
    assert_eq!(warmed.journal, Ok((0, 0)));
    // (5), (9)
    let (status, full) = fixture.get("feed=gdfl").await;
    assert_eq!(status, axum::http::StatusCode::TOO_MANY_REQUESTS, "{full}");
    // (6), (7)
    let why = full["error"].as_str().expect("a reason");
    assert!(
        why.contains(&MAX_BUILDS.to_string()) && why.contains("retry"),
        "{why}"
    );
    // (8)
    let (key, build) = held.first().expect("a held build");
    builds.finish(
        key,
        build,
        crate::calendar_of::KeptAnswer {
            status: 200,
            body: "{}".to_owned(),
        },
    );
    assert_eq!(builds.in_flight(), MAX_BUILDS - 1);
    let (status, answered) = fixture.get("feed=gdfl").await;
    assert_eq!(status, axum::http::StatusCode::OK, "{answered}");
    assert_eq!(
        answered["status"], "withheld",
        "GDFL holds no census: {answered}"
    );
}

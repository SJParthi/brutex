//! `census_cache.rs`'s tests of the warm at start and the background re-read
//! (R3-09; DB-G80, RA-008, RA-032, RA-046, RA-093, RA-094).
//!
//! What they pin, each against a store of its own:
//!
//! * every cache [`start`] warms is listed `warming` or `ready` from the
//!   moment the server starts, comes ready, and the first request after it
//!   reads nothing -- over twenty restarts;
//! * a census change beside a thousand feed-list requests never makes one of
//!   them wait for the re-read: they are answered from the previous read,
//!   with its age, and no request thread reads the census;
//! * a /db tab that held the census across twenty imported option days (more
//!   than the tail floor) and a restart refreshes by a delta, read back from
//!   the file by position, with no whole read;
//! * no /db view, served by the real router, reads the census on a request
//!   thread -- warm, and after a change.
//!
//! "Reads" are counted, not inferred: `census::manifest_reads` records every
//! read of a manifest file (whole, by its tail, read back) with the name of
//! the thread that made it, and the only threads allowed to read once the
//! caches are warm are the warm (`brutex-warm-census`) and the background
//! re-reads (`brutex-census-<vendor>`).
//!
//! Every price, row count and contract below is invented.

use super::*;
use crate::server::store_route::{
    CENSUS_EPOCH_HEADER, CENSUS_PART_GENERATIONS_HEADER, store_response,
};
use crate::server::{Loaded, Site};
use brutex_core::instrument::{Contract, Exchange, Segment};
use brutex_core::symbol::Symbol;
use pull::manifest::{Closes, Entry, EntryKey, Held, Manifest, epoch_path, segment_dir};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use store::path::{Timeframe, YearMonth};

/// 2019-03-01 09:15:00 IST, in microseconds: inside the month every key is in.
const MARCH: i64 = 1_551_411_900_000_000;

/// The threads allowed to read a census once the caches are warm.
fn is_census_thread(name: &str) -> bool {
    name.starts_with("brutex-census-") || name == "brutex-warm-census"
}

/// One NIFTY option of March 2019 at 1s: contract `n` of an invented chain.
fn option_key(n: u64) -> EntryKey {
    let strike = 100 * (10_000 + n);
    let side = if n.is_multiple_of(2) { "CE" } else { "PE" };
    EntryKey {
        contract: Some(
            Contract::parse(&format!("2019-03-28-{strike}-{side}")).expect("a contract"),
        ),
        exchange: Exchange::Nse,
        segment: Segment::Fno,
        symbol: Symbol::new("NIFTY").expect("a symbol"),
        timeframe: Timeframe::SECOND_1,
        month: YearMonth::new(2019, 3).expect("a month"),
    }
}

/// A row of `key` holding `rows` bars, with invented closes.
fn row(key: EntryKey, rows: u64) -> Held {
    let span = i64::try_from(rows).expect("small");
    Held::new(
        Entry {
            key,
            rows,
            first_ts_micros: MARCH,
            last_ts_micros: MARCH + span * 1_000_000,
        },
        Closes::known(5_001, 5_000 + span).expect("a close"),
    )
}

/// NIFTY spot at 1s, March 2019.
fn spot_row() -> Held {
    row(
        EntryKey {
            contract: None,
            exchange: Exchange::Nse,
            segment: Segment::Index,
            symbol: Symbol::new("NIFTY").expect("a symbol"),
            timeframe: Timeframe::SECOND_1,
            month: YearMonth::new(2019, 3).expect("a month"),
        },
        22_500,
    )
}

/// A store root with GDFL's census migrated to partitions (D-3316): NIFTY
/// spot in INDEX, `options` NIFTY options in FNO, and the FNO epoch kept
/// open to append to the way the importer does.
struct Store {
    root: PathBuf,
    fno: Manifest,
    fno_path: PathBuf,
}

impl Store {
    fn new(name: &str, options: u64) -> Self {
        let root = crate::scratch::path(&format!("census-warm-{name}"));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("masters")).expect("mkdir");
        std::fs::create_dir_all(root.join("manifest")).expect("mkdir");
        let mut index = Manifest::partition_genesis(Vendor::Gdfl, Segment::Index, 1_048_576)
            .expect("a partition");
        let _spot = index.record_held(spot_row()).expect("records");
        std::fs::create_dir_all(segment_dir(&root, Vendor::Gdfl, Segment::Index)).expect("mkdir");
        std::fs::write(
            epoch_path(&root, Vendor::Gdfl, Segment::Index, 1),
            index.image(),
        )
        .expect("written");
        let mut fno = Manifest::partition_genesis(Vendor::Gdfl, Segment::Fno, 1_048_576)
            .expect("a partition");
        for n in 0..options {
            let _append = fno.record_held(row(option_key(n), 10)).expect("records");
        }
        std::fs::create_dir_all(segment_dir(&root, Vendor::Gdfl, Segment::Fno)).expect("mkdir");
        let fno_path = epoch_path(&root, Vendor::Gdfl, Segment::Fno, 1);
        std::fs::write(&fno_path, fno.image()).expect("written");
        Self {
            root,
            fno,
            fno_path,
        }
    }

    /// A server over this store, as a restart makes one.
    fn site(&self) -> Loaded {
        Loaded::new(Site::load(&self.root.join("masters"), &self.root))
    }

    /// Appends `rows` to the FNO epoch the way the importer does: each entry
    /// at its own offset, then the header slot that counts it.
    fn append(&mut self, rows: &[Held]) {
        use std::os::unix::fs::FileExt as _;
        let file = std::fs::OpenOptions::new()
            .write(true)
            .open(&self.fno_path)
            .expect("open");
        for one in rows {
            let append = self.fno.record_held(*one).expect("records");
            file.write_all_at(&append.bytes, append.offset)
                .expect("entry");
            file.write_all_at(&append.commit.bytes, append.commit.offset)
                .expect("slot");
        }
    }

    /// Every read of this store's manifests so far: path, kind, thread.
    fn reads(&self) -> Vec<(PathBuf, &'static str, String)> {
        census::manifest_reads::threads_under(&self.root)
    }
}

/// Waits until every cache `site` lists has finished: ready or failed.
fn wait_ready(site: &Site) -> Vec<Warmth> {
    let deadline = Instant::now() + Duration::from_mins(2);
    loop {
        let rows = warmth(site);
        if !rows.is_empty()
            && rows
                .iter()
                .all(|row| matches!(row.state, WarmState::Ready | WarmState::Failed))
        {
            return rows;
        }
        assert!(Instant::now() < deadline, "never warm: {rows:?}");
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// The row `name` of `vendor`.
fn row_of<'a>(rows: &'a [Warmth], name: &str, vendor: Vendor) -> &'a Warmth {
    rows.iter()
        .find(|row| row.name == name && row.vendor == vendor)
        .unwrap_or_else(|| panic!("{name} of {} is not listed: {rows:?}", vendor.as_str()))
}

/// GDFL's FNO partition generation as `site` serves it now, and how current.
fn fno_generation(site: &Site) -> (u64, Freshness) {
    let (_stamps, now) = vendor_census(site, Vendor::Gdfl);
    let census = now.census().expect("a census row");
    let census::Census::Held { ref manifest } = census.state else {
        panic!("not held: {}", census.note());
    };
    let fno = manifest
        .parts()
        .iter()
        .find(|part| part.segment() == Some(Segment::Fno))
        .expect("an FNO part");
    (fno.header().generation, now.freshness())
}

/// `GET /store.json?<query>` through the route's own function.
fn store_get(site: &Site, query: &str) -> (u16, axum::http::HeaderMap, Vec<u8>) {
    let uri: axum::http::Uri = format!("/store.json?{query}").parse().expect("a uri");
    let (code, headers, body) = store_response(
        site,
        &axum::http::HeaderMap::new(),
        &uri,
        &axum::http::Method::GET,
    );
    (code.as_u16(), headers, body.to_vec())
}

/// One header's text, or a failure naming it.
fn header(headers: &axum::http::HeaderMap, name: &str) -> String {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_else(|| panic!("no {name}: {headers:?}"))
        .to_owned()
}

/// `=` and `,` percent-encoded, as a browser sends a since and an epoch.
fn encoded(text: &str) -> String {
    text.replace('=', "%3D").replace(',', "%2C")
}

/// The rows a tuples body carries.
fn rows_in(body: &[u8]) -> usize {
    let value: serde_json::Value = serde_json::from_slice(body).expect("json");
    value["rows"]
        .as_array()
        .unwrap_or_else(|| panic!("no rows: {value}"))
        .len()
}

/// **EVERY CACHE IS WARM OR SAYS WARMING AFTER A RESTART** (R3-09; RA-046,
/// RA-094). From the instant a server starts, `/census/warm.json` lists the
/// census of every vendor and, for each offered feed, the facet index, the
/// index map and the import journal: each `warming` or `ready`, never
/// missing. Each comes ready, timed, standing on the census it was built
/// from, and the first request after that reads no manifest. Twenty
/// restarts, each a new server over the same store.
#[test]
fn every_cache_is_warm_or_says_warming_after_a_restart() {
    let store = Store::new("restart", 64);
    let feeds = [Vendor::Gdfl, Vendor::Zerodha];
    let expected: Vec<(&str, Vendor)> = Vendor::ALL
        .into_iter()
        .map(|vendor| (CENSUS, vendor))
        .chain(
            feeds
                .iter()
                .flat_map(|feed| WARMED.into_iter().skip(1).map(move |name| (name, *feed))),
        )
        .collect();
    // EVERY CENSUS READ UNDER THIS ROOT PAUSES, so the warm is caught warming.
    slow_read::set(&store.root, 300);
    let site = store.site();
    assert!(!in_background(&site), "not before the start");
    start(&site, &feeds).expect("the warm thread starts");
    let listed = warmth(&site);
    assert_eq!(listed.len(), expected.len(), "{listed:?}");
    for (name, vendor) in &expected {
        let row = row_of(&listed, name, *vendor);
        assert!(
            matches!(row.state, WarmState::Warming | WarmState::Ready),
            "{row:?}"
        );
    }
    assert_eq!(
        row_of(&listed, CENSUS, Vendor::Gdfl).state,
        WarmState::Warming,
        "GDFL's census read takes 300 ms"
    );
    let order: Vec<(&str, Vendor)> = listed.iter().map(|row| (row.name, row.vendor)).collect();
    assert_eq!(
        order.get(..WARMED.len() * feeds.len()),
        Some(
            feeds
                .iter()
                .flat_map(|feed| WARMED.into_iter().map(move |name| (name, *feed)))
                .collect::<Vec<_>>()
                .as_slice()
        ),
        "each offered feed's census and what is derived from it, in the order given, first"
    );
    let body: serde_json::Value = serde_json::from_str(&warm_json(&site)).expect("json");
    assert_eq!(body["background"], true);
    assert_eq!(
        body["caches"].as_array().expect("caches").len(),
        expected.len()
    );
    assert_eq!(body["caches"][0]["name"], CENSUS);
    assert_eq!(body["caches"][0]["feed"], "gdfl", "the offered feeds lead");
    assert_eq!(body["caches"][0]["state"], "warming");

    let ready = wait_ready(&site);
    slow_read::set(&store.root, 0);
    for (name, vendor) in &expected {
        let row = row_of(&ready, name, *vendor);
        assert_eq!(row.state, WarmState::Ready, "{row:?}");
        assert!(row.took_ms.is_some(), "{row:?}");
    }
    let gdfl = row_of(&ready, CENSUS, Vendor::Gdfl);
    assert!(
        gdfl.took_ms.is_some_and(|ms| ms >= 300),
        "the time is the read's own: {gdfl:?}"
    );
    let (_stamps, now) = vendor_census(&site, Vendor::Gdfl);
    assert_eq!(
        gdfl.generation,
        now.census().and_then(census::VendorCensus::generation)
    );
    assert_eq!(
        gdfl.entries,
        now.census()
            .and_then(census::VendorCensus::counters)
            .map(|(_, _, entries)| entries)
    );
    assert_eq!(
        row_of(&ready, CENSUS, Vendor::Dhan).generation,
        None,
        "an absent census stood on no generation, and says none rather than zero"
    );
    let body: serde_json::Value = serde_json::from_str(&warm_json(&site)).expect("json");
    assert_eq!(body["caches"][0]["state"], "ready");
    assert!(body["caches"][0]["took_ms"].as_u64().is_some(), "{body}");

    // THE FIRST REQUEST AFTER THE WARM READS NOTHING.
    let mark = store.reads().len();
    let (_stamps, now) = vendor_census(&site, Vendor::Gdfl);
    assert!(now.census().is_some());
    assert_eq!(now.freshness(), Freshness::Current);
    assert_eq!(store.reads().len(), mark, "{:?}", &store.reads()[mark..]);

    twenty_restarts(&store, &feeds, expected.len());
}

/// [`every_cache_is_warm_or_says_warming_after_a_restart`]'s twenty
/// restarts: each a new server over `store`, every one of `caches` listed
/// warming or ready at once, all ready, and the first requests read nothing.
fn twenty_restarts(store: &Store, feeds: &[Vendor], caches: usize) {
    for restart in 1..=20 {
        let again = store.site();
        start(&again, feeds).expect("the warm thread starts");
        let listed = warmth(&again);
        assert_eq!(listed.len(), caches, "restart {restart}");
        assert!(
            listed
                .iter()
                .all(|row| matches!(row.state, WarmState::Warming | WarmState::Ready)),
            "restart {restart}: {listed:?}"
        );
        let ready = wait_ready(&again);
        assert!(
            ready.iter().all(|row| row.state == WarmState::Ready),
            "restart {restart}: {ready:?}"
        );
        let mark = store.reads().len();
        let (_stamps, now) = vendor_census(&again, Vendor::Gdfl);
        assert_eq!(now.freshness(), Freshness::Current, "restart {restart}");
        let (_, _, body) = store_get(&again, "feed=gdfl&encoding=census-tuples-v1");
        assert_eq!(rows_in(&body), 65, "restart {restart}: every row");
        assert_eq!(
            store.reads().len(),
            mark,
            "restart {restart}: the first requests read nothing"
        );
        let every = store.reads();
        assert!(
            every
                .iter()
                .all(|(_, _, thread)| is_census_thread(thread) || thread.contains("every_cache")),
            "only the warm and each server's own start-up snapshot: {every:?}"
        );
    }
}

/// **A CENSUS CHANGE NEVER BLOCKS A REQUEST** (R3-09; DB-G80, RA-032). The
/// census changes beside 1,000 feed-list requests while every census read
/// under the store pauses five seconds: no request waits for it. Each is
/// answered from the previous read and says so, none takes 50 ms, no request
/// thread reads the census, and once the background re-read lands the
/// change is served, current.
#[tokio::test(flavor = "current_thread")]
async fn a_census_change_never_blocks_a_request() {
    let mut store = Store::new("never-blocks", 64);
    let site = store.site();
    start(&site, &[Vendor::Gdfl]).expect("the warm thread starts");
    let _ready = wait_ready(&site);
    let (before, freshness) = fno_generation(&site);
    assert_eq!(freshness, Freshness::Current);

    slow_read::set(&store.root, 5_000);
    let mark = store.reads().len();
    let mut took: Vec<Duration> = Vec::with_capacity(1_000);
    let mut stale = 0_usize;
    for n in 0..1_000 {
        if n == 100 {
            store.append(&[row(option_key(10_000), 7)]);
        }
        let began = Instant::now();
        let (code, _headers, body) =
            crate::server::feeds_route::feeds_json(axum::extract::State(Loaded::clone(&site)))
                .await;
        let (_stamps, now) = vendor_census(&site, Vendor::Gdfl);
        took.push(began.elapsed());
        assert_eq!(code, axum::http::StatusCode::OK, "{body}");
        assert!(body.starts_with('['), "{body}");
        if now.freshness().is_stale() {
            stale += 1;
            assert!(n >= 100, "stale before the change, at {n}");
        }
    }
    slow_read::set(&store.root, 0);
    took.sort_unstable();
    let p99 = took[989];
    let max = took[999];
    assert!(
        max < Duration::from_millis(50),
        "a request waited: p50 {:?}, p99 {p99:?}, max {max:?}",
        took[499]
    );
    assert!(
        stale >= 800,
        "the requests after the change were answered from the previous read: {stale}"
    );
    let reads = store.reads();
    assert!(
        reads[mark..]
            .iter()
            .all(|(_, _, thread)| is_census_thread(thread)),
        "a request thread read the census: {:?}",
        &reads[mark..]
    );

    // THE CHANGE LANDS: the background re-read is served, current.
    let deadline = Instant::now() + Duration::from_mins(1);
    loop {
        let (generation, freshness) = fno_generation(&site);
        if freshness == Freshness::Current && generation == before + 1 {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "never current: {generation} {freshness:?}"
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let rows = warmth(&site);
    assert_eq!(row_of(&rows, CENSUS, Vendor::Gdfl).state, WarmState::Ready);
    assert_eq!(
        row_of(&rows, CENSUS, Vendor::Gdfl).generation,
        vendor_census(&site, Vendor::Gdfl)
            .1
            .census()
            .and_then(census::VendorCensus::generation)
    );
}

/// **TWENTY IMPORTED OPTION DAYS REFRESH WITHOUT A WHOLE READ** (R3-09;
/// RA-008, RA-042). A /db tab read the census whole and kept its
/// generations. Twenty options days are imported -- 5,000 new contracts and
/// 10 grown ones each, 100,200 entries, past the tail floor so the running
/// server reads the census whole again -- and the server is restarted. The
/// tab's refresh is answered, by the running server and by the restarted
/// one: a delta of exactly the 100,010 changed keys, read back from the file
/// by position, never a whole read and never a `410` that sends the tab to
/// download the whole list. The option lists' refresh is answered the same.
#[test]
fn twenty_imported_option_days_refresh_without_a_whole_read() {
    let mut store = Store::new("twenty-days", 100);
    let site = store.site();
    start(&site, &[Vendor::Gdfl]).expect("the warm thread starts");
    let _ready = wait_ready(&site);
    let (code, headers, body) = store_get(&site, "feed=gdfl&encoding=census-tuples-v1");
    assert_eq!(code, 200, "{}", String::from_utf8_lossy(&body));
    assert_eq!(rows_in(&body), 101);
    let since = header(&headers, CENSUS_PART_GENERATIONS_HEADER);
    let epoch = header(&headers, CENSUS_EPOCH_HEADER);
    let (fno_since, _) = fno_generation(&site);

    for day in 0..20_u64 {
        let mut rows: Vec<Held> = (0..5_000)
            .map(|n| row(option_key(100 + day * 5_000 + n), 10))
            .collect();
        rows.extend((0..10).map(|n| row(option_key(n), 11 + day)));
        store.append(&rows);
    }
    let newest = fno_since + 100_200;
    // THE RUNNING SERVER: noticed by a request, re-read in the background.
    let deadline = Instant::now() + Duration::from_mins(5);
    while fno_generation(&site) != (newest, Freshness::Current) {
        assert!(
            Instant::now() < deadline,
            "the running server never caught up"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    // AND A RESTART.
    let restarted = store.site();
    start(&restarted, &[Vendor::Gdfl]).expect("the warm thread starts");
    let _ready = wait_ready(&restarted);
    assert_eq!(fno_generation(&restarted), (newest, Freshness::Current));

    for (served, which) in [(&site, "running"), (&restarted, "restarted")] {
        let mark = store.reads().len();
        let began = Instant::now();
        let (code, _headers, body) = store_get(
            served,
            &format!(
                "feed=gdfl&encoding=census-tuples-v1&since={}&epoch={}",
                encoded(&since),
                encoded(&epoch)
            ),
        );
        let took = began.elapsed();
        assert_eq!(code, 200, "{which}: {}", String::from_utf8_lossy(&body));
        assert_eq!(rows_in(&body), 100_010, "{which}: every changed key once");
        let reads = store.reads();
        let kinds: Vec<&str> = reads[mark..].iter().map(|(_, kind, _)| *kind).collect();
        assert_eq!(
            kinds,
            vec!["read-back"],
            "{which}: one read-back, no whole read, no tail"
        );
        assert_eq!(
            census::manifest_reads::read_backs_of(&store.fno_path).last(),
            Some(&100_200),
            "{which}: exactly the entries after the generation the tab held"
        );
        // THE OPTION LISTS' REFRESH, from the same generation.
        let (code, _headers, body) = crate::server::facets::answer(
            served,
            &format!("feed=gdfl&segment=FNO&underlying=NIFTY&since={fno_since}"),
        );
        assert_eq!(code, axum::http::StatusCode::OK, "{which}: {body}");
        eprintln!("R3-09 measurement: {which} server, refresh after 100,200 new entries: {took:?}");
    }
}

/// The routes [`no_db_view_reads_the_whole_census`] asks: the feed list, the
/// whole census and a refresh of it since `since` in `epoch`, the option
/// lists, the coverage roll-up (R3-09 goal 1), the index map, the warm list
/// and the audit.
fn db_views(since: &str, epoch: &str) -> Vec<String> {
    let mut views: Vec<String> = [
        "/feeds.json",
        "/store.json?feed=gdfl&encoding=census-tuples-v1",
        "/census/facets.json?feed=gdfl",
        "/census/facets.json?feed=gdfl&segment=FNO",
        "/census/facets.json?feed=gdfl&segment=FNO&underlying=NIFTY",
        "/census/facets.json?feed=gdfl&segment=FNO&underlying=NIFTY&expiry=2019-03-28",
        "/census/facets.json?feed=gdfl&segment=INDEX&underlying=NIFTY&rows=census-tuples-v1",
        "/census/rollup.json?feed=gdfl&segment=FNO",
        "/census/rollup.json?feed=gdfl&segment=FNO&underlying=NIFTY",
        "/indexmap.json?feed=gdfl",
        "/census/warm.json",
        "/audit.json?feed=gdfl",
    ]
    .map(str::to_owned)
    .to_vec();
    views.insert(
        2,
        format!("/store.json?feed=gdfl&encoding=census-tuples-v1&since={since}&epoch={epoch}"),
    );
    views
}

/// Whether `view` is census-backed, and so says how current it is.
fn census_backed(view: &str) -> bool {
    view.starts_with("/store.json") || view.contains("facets") || view.contains("rollup")
}

/// Whether `reply` answers `view`.
///
/// `/indexmap.json` reads the exchange's catalogue from the PROCESS's masters
/// directory (`server::masters_dir`: `BRUTEX_MASTERS`, else
/// `$HOME/.brutex/masters`), not the site's, and reads it BEFORE the census:
/// on a machine without one it is refused 500 naming the file (never an empty
/// map), with no census read. So the route is held to that, and its census
/// half is asked directly, over an invented catalogue.
fn answered(view: &str, reply: &str) -> bool {
    reply.starts_with("HTTP/1.1 200")
        || (view.starts_with("/indexmap.json")
            && reply.starts_with("HTTP/1.1 500")
            && reply.contains("nse_indices.csv"))
}

/// The real router over `site` on a port of its own: the address, the
/// sender that stops it, and its task.
async fn serve_site(
    site: &Loaded,
    name: &str,
) -> (
    std::net::SocketAddr,
    tokio::sync::oneshot::Sender<()>,
    tokio::task::JoinHandle<()>,
) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("a port");
    let addr = listener.local_addr().expect("an address");
    let app =
        crate::server::router_serving(Loaded::clone(site), crate::server::tests::front(name), addr);
    let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
    let server = tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = stopped.await;
            })
            .await
            .expect("serves");
    });
    (addr, stop, server)
}

/// Every one of `views` asked once over `addr`: each answered within 900 ms,
/// and every census-backed one saying `x-brutex-census-refreshing: <refreshing>`.
async fn ask_every(addr: std::net::SocketAddr, views: &[String], refreshing: u8) {
    for view in views {
        let began = Instant::now();
        let reply = crate::server::tests::get(addr, view).await;
        let took = began.elapsed();
        assert!(answered(view, &reply), "{view}: {reply:.400}");
        assert!(
            took < Duration::from_millis(900),
            "{view} waited for the re-read: {took:?}"
        );
        if census_backed(view) {
            assert!(
                reply.contains(&format!("x-brutex-census-refreshing: {refreshing}")),
                "{view} says how current it is: {reply:.600}"
            );
        }
    }
}

/// **NO /DB VIEW READS THE WHOLE CENSUS** (R3-09; DB-G80, NEW-07). Every
/// census-backed route the /db page calls, served by the real router over a
/// warm cache, reads no manifest on a request thread: not when nothing
/// changed, and not after the census changes, when each is answered from
/// the previous read (saying so) while the change is read in the background.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn no_db_view_reads_the_whole_census() {
    let _apart = crate::detail::apart_from_slot_owners().await;
    let mut store = Store::new("no-db-view", 64);
    let site = store.site();
    start(&site, &[Vendor::Gdfl]).expect("the warm thread starts");
    let _ready = wait_ready(&site);
    let (addr, stop, server) = serve_site(&site, "census-no-db-view").await;
    let (_, headers, _) = store_get(&site, "feed=gdfl&encoding=census-tuples-v1");
    let views = db_views(
        &encoded(&header(&headers, CENSUS_PART_GENERATIONS_HEADER)),
        &encoded(&header(&headers, CENSUS_EPOCH_HEADER)),
    );
    let catalogue = store.root.join("masters").join("nse_indices.csv");
    std::fs::write(&catalogue, "index_name,category\nNIFTY 50,broad\n").expect("invented");
    let nse = crate::indexmap::Published::read(&catalogue).expect("reads");
    let index_map = || {
        let began = Instant::now();
        let code = crate::indexmap::answer(&site, &nse, Vendor::Gdfl, Vec::new()).0;
        assert_eq!(
            code,
            axum::http::StatusCode::OK,
            "the index map's census half"
        );
        assert!(
            began.elapsed() < Duration::from_millis(900),
            "the index map waited"
        );
    };

    let mark = store.reads().len();
    ask_every(addr, &views, 0).await;
    index_map();
    assert_eq!(
        store.reads().len(),
        mark,
        "a warm /db view read the census: {:?}",
        &store.reads()[mark..]
    );

    // A CHANGE, AND A RE-READ THAT TAKES A SECOND.
    slow_read::set(&store.root, 1_000);
    store.append(&[row(option_key(500), 3)]);
    let mark = store.reads().len();
    ask_every(addr, &views, 1).await;
    index_map();
    let reads = store.reads();
    assert!(
        reads[mark..]
            .iter()
            .all(|(_, _, thread)| is_census_thread(thread)),
        "a /db view read the census on a request thread: {:?}",
        &reads[mark..]
    );
    slow_read::set(&store.root, 0);
    let deadline = Instant::now() + Duration::from_mins(1);
    loop {
        let reply = crate::server::tests::get(addr, &views[1]).await;
        if reply.contains("x-brutex-census-refreshing: 0") {
            break;
        }
        assert!(Instant::now() < deadline, "never current: {reply:.600}");
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    stop.send(()).expect("stops");
    server.await.expect("the server task");
}

/// A store with no manifest directory at all is warmed as absent, ready, and
/// a missing root is named: the warm never stops at the first vendor that
/// cannot be read, and never lists a cache it did not reach as ready.
#[test]
fn a_store_with_nothing_or_no_root_is_warmed_and_named() {
    let empty = crate::scratch::path("census-warm-empty");
    let _ = std::fs::remove_dir_all(&empty);
    std::fs::create_dir_all(empty.join("masters")).expect("mkdir");
    let site = Loaded::new(Site::load(&empty.join("masters"), &empty));
    start(&site, &[Vendor::Gdfl]).expect("starts");
    let rows = wait_ready(&site);
    for (name, state) in [
        (CENSUS, WarmState::Ready),
        (FACETS, WarmState::Ready),
        (JOURNAL, WarmState::Ready),
        // THE INDEX MAP IS REFUSED WITHOUT A CENSUS, as its route refuses:
        // failed, naming why, never ready over nothing.
        (INDEX_MAP, WarmState::Failed),
    ] {
        assert_eq!(
            row_of(&rows, name, Vendor::Gdfl).state,
            state,
            "{name}: {rows:?}"
        );
    }
    assert!(
        row_of(&rows, INDEX_MAP, Vendor::Gdfl)
            .reason
            .as_deref()
            .is_some_and(|why| why.contains("no manifest")),
        "{rows:?}"
    );
    assert!(rows.iter().all(|row| row.generation.is_none()), "{rows:?}");

    // THE STORE'S DRIVE GOES AWAY between the server loading and the warm
    // (the root renamed aside): the census is read and refused by name.
    let gone = crate::scratch::path("census-warm-no-root");
    let _ = std::fs::remove_dir_all(&gone);
    std::fs::create_dir_all(gone.join("masters")).expect("mkdir");
    let root = gone.join("not-here");
    std::fs::create_dir_all(root.join("manifest")).expect("mkdir");
    let site = Loaded::new(Site::load(&gone.join("masters"), &root));
    std::fs::rename(&root, gone.join("aside")).expect("moved aside");
    start(&site, &[Vendor::Gdfl]).expect("starts");
    let rows = wait_ready(&site);
    let census = row_of(&rows, CENSUS, Vendor::Gdfl);
    assert_eq!(census.state, WarmState::Failed, "{census:?}");
    assert!(
        census
            .reason
            .as_deref()
            .is_some_and(|why| why.contains("not-here")),
        "{census:?}"
    );
}

/// Unused-path guard: the fixture's own root is where every read is counted.
#[test]
fn the_counting_reader_names_the_thread_that_read() {
    let store = Store::new("counting", 2);
    let before = store.reads().len();
    let _read = census::read_vendor(&store.root, Vendor::Gdfl);
    let reads = store.reads();
    assert_eq!(reads.len(), before + 2, "both partitions, whole: {reads:?}");
    assert!(
        reads[before..]
            .iter()
            .all(|(path, kind, thread)| *kind == "whole"
                && path.starts_with(Path::new(&store.root))
                && thread.contains("the_counting_reader")),
        "{reads:?}"
    );
}

/// **THE CONTROL: WITHOUT THE START, THE REQUEST THAT MEETS A CHANGE READS
/// IT, AND WAITS FOR IT** -- what every request did before R3-09 (DB-G80:
/// `/feeds.json` stalled 550 ms behind a census miss), and what a one-shot
/// caller (the `cli`, a test) still does. The same change and the same slow
/// read as [`a_census_change_never_blocks_a_request`]: the first request
/// after it takes the read's time, on its own thread, and is current.
#[test]
fn without_the_start_the_request_that_meets_a_change_reads_it() {
    let mut store = Store::new("control", 8);
    let site = store.site();
    let (before, freshness) = fno_generation(&site);
    assert_eq!(freshness, Freshness::Current);
    assert!(!in_background(&site));
    slow_read::set(&store.root, 300);
    store.append(&[row(option_key(77), 4)]);
    let mark = store.reads().len();
    let began = Instant::now();
    let (generation, freshness) = fno_generation(&site);
    let took = began.elapsed();
    slow_read::set(&store.root, 0);
    assert_eq!((generation, freshness), (before + 1, Freshness::Current));
    assert!(
        took >= Duration::from_millis(300),
        "it waited for the read: {took:?}"
    );
    let reads = store.reads();
    assert!(
        reads[mark..]
            .iter()
            .any(|(_, _, thread)| thread.contains("without_the_start")),
        "the request's own thread read it: {:?}",
        &reads[mark..]
    );
}

// ══ The coverage roll-up per month (`census_rollup.rs`; R3-09) ══════════════

/// Contract `n` of the invented 2019-03-28 NIFTY chain, at `month` and
/// `timeframe`.
fn option_at(n: u64, month: (u16, u8), timeframe: Timeframe) -> EntryKey {
    EntryKey {
        month: YearMonth::new(month.0, month.1).expect("a month"),
        timeframe,
        ..option_key(n)
    }
}

/// A row of `key` holding `rows` bars from `first` (microseconds), one a
/// second, with invented closes.
fn row_from(key: EntryKey, rows: u64, first: i64) -> Held {
    let span = i64::try_from(rows).expect("small");
    Held::new(
        Entry {
            key,
            rows,
            first_ts_micros: first,
            last_ts_micros: first + span * 1_000_000,
        },
        Closes::known(5_001, 5_000 + span).expect("a close"),
    )
}

/// 2019-04-01 09:15:00 IST, in microseconds.
const APRIL: i64 = 1_554_090_300_000_000;

/// The roll-up as an independent walk of `manifest` (the fixture's own FNO
/// manifest, every key it recorded): per (month, timeframe), the keys, rows,
/// earliest first and latest last stamp.
fn walked(manifest: &Manifest) -> BTreeMap<(String, String), (u64, u64, i64, i64)> {
    let mut out: BTreeMap<(String, String), (u64, u64, i64, i64)> = BTreeMap::new();
    for key in manifest.held_keys() {
        let entry = manifest.entry(key).expect("an entry");
        let cell = out
            .entry((key.month.to_string(), key.timeframe.as_str().to_owned()))
            .or_insert((0, 0, i64::MAX, i64::MIN));
        cell.0 += 1;
        cell.1 += entry.rows;
        cell.2 = cell.2.min(entry.first_ts_micros);
        cell.3 = cell.3.max(entry.last_ts_micros);
    }
    out
}

/// The roll-up's months as the same map [`walked`] builds.
fn served(body: &serde_json::Value) -> BTreeMap<(String, String), (u64, u64, i64, i64)> {
    body["months"]
        .as_array()
        .unwrap_or_else(|| panic!("no months: {body}"))
        .iter()
        .map(|row| {
            (
                (
                    row["month"].as_str().expect("a month").to_owned(),
                    row["timeframe"].as_str().expect("a rung").to_owned(),
                ),
                (
                    row["instrument_months"].as_u64().expect("a count"),
                    row["rows"].as_u64().expect("rows"),
                    row["first_ts"].as_i64().expect("a first"),
                    row["last_ts"].as_i64().expect("a last"),
                ),
            )
        })
        .collect()
}

/// `GET /census/rollup.json?<query>` through the route's own function.
fn rollup_get(site: &Site, query: &str) -> (u16, axum::http::HeaderMap, serde_json::Value) {
    let (code, headers, body) = crate::server::census_rollup::answer(site, query);
    let value = serde_json::from_str(&body).unwrap_or_else(|why| panic!("{why}: {body}"));
    (code.as_u16(), headers, value)
}

/// Sessions in `first ..= last` (epoch days), by the calendar of record.
fn sessions_by_calendar(first: i64, last: i64) -> u64 {
    (first..=last)
        .filter(|&day| {
            matches!(
                pull::calendar::sourced_kind_of(day),
                pull::calendar::DayKind::Open(_) | pull::calendar::DayKind::OpenLengthUnmeasured
            )
        })
        .count() as u64
}

/// **THE ROLL-UP EQUALS A WALK OF THE CENSUS AND MOVES FORWARD WITHOUT A
/// WHOLE READ** (R3-09 goal 1). A NIFTY options census of one month at 1s
/// is rolled up per (month, rung) exactly as an independent walk of every
/// key counts it; then a new month, a new rung and a grown key are appended
/// (the importer's way) and the roll-up, once current, equals the walk
/// again -- read by the census's tail on the background thread and moved
/// forward, never read whole, and never read by the request. A restarted
/// server built cold over the same file serves the same bytes. The rows are
/// newest month first, finest rung first, and every day count is the
/// calendar of record's.
#[test]
fn the_rollup_equals_a_walk_and_moves_forward_without_a_whole_read() {
    let mut store = Store::new("rollup", 40);
    let site = store.site();
    start(&site, &[Vendor::Gdfl]).expect("the warm thread starts");
    let _ready = wait_ready(&site);
    let query = "feed=gdfl&segment=FNO&underlying=NIFTY";
    let mark = store.reads().len();
    let (code, headers, body) = rollup_get(&site, query);
    assert_eq!(code, 200, "{body}");
    assert_eq!(served(&body), walked(&store.fno), "{body}");
    assert_eq!(body["exchange"], "NSE", "resolved by probe: {body}");
    assert_eq!(header(&headers, CENSUS_REFRESHING_HEADER), "0");
    assert_eq!(store.reads().len(), mark, "a warm roll-up read the census");
    // THE DAYS: March 2019's sessions by the calendar of record, and the one
    // session every key's bars span (each is ten seconds of 2019-03-01).
    let march = &body["months"][0];
    let (first, last) = (
        pull::session::Day::new(2019, 3, 1).expect("a day"),
        pull::session::Day::new(2019, 3, 31).expect("a day"),
    );
    let owed = sessions_by_calendar(
        i64::from(first.days_from_epoch()),
        i64::from(last.days_from_epoch()),
    );
    assert!((15..=23).contains(&owed), "a month's sessions: {owed}");
    assert_eq!(march["days_owed"].as_u64(), Some(owed), "{body}");
    assert_eq!(march["days_held"].as_u64(), Some(1), "{body}");
    assert_eq!(body["rungs"][0]["months_held"].as_u64(), Some(1), "{body}");
    assert_eq!(body["rungs"][0]["days_owed"].as_u64(), Some(owed), "{body}");

    // APPENDED: a new month at the same rung, a new rung, and a key grown.
    let mark = store.reads().len();
    store.append(&[
        row_from(option_at(1, (2019, 4), Timeframe::SECOND_1), 30, APRIL),
        row_from(option_at(2, (2019, 4), Timeframe::SECOND_1), 20, APRIL),
        row_from(option_at(1, (2019, 4), Timeframe::MINUTE_1), 5, APRIL),
        row(option_key(3), 99),
    ]);
    let deadline = Instant::now() + Duration::from_mins(1);
    let body = loop {
        let (code, headers, body) = rollup_get(&site, query);
        assert_eq!(code, 200, "{body}");
        if header(&headers, CENSUS_REFRESHING_HEADER) == "0" && served(&body) == walked(&store.fno)
        {
            break body;
        }
        assert!(Instant::now() < deadline, "never current: {body}");
        std::thread::sleep(Duration::from_millis(10));
    };
    let reads = store.reads();
    assert!(
        reads[mark..]
            .iter()
            .all(|(_, kind, thread)| *kind != "whole" && is_census_thread(thread)),
        "the change was read whole, or by a request: {:?}",
        &reads[mark..]
    );
    let order: Vec<(String, String)> = body["months"]
        .as_array()
        .expect("months")
        .iter()
        .map(|row| {
            (
                row["month"].as_str().expect("m").to_owned(),
                row["timeframe"].as_str().expect("t").to_owned(),
            )
        })
        .collect();
    assert_eq!(
        order,
        [
            (
                "2019-04".to_owned(),
                Timeframe::SECOND_1.as_str().to_owned()
            ),
            (
                "2019-04".to_owned(),
                Timeframe::MINUTE_1.as_str().to_owned()
            ),
            (
                "2019-03".to_owned(),
                Timeframe::SECOND_1.as_str().to_owned()
            ),
        ],
        "newest month first, finest rung first: {body}"
    );
    let second = body["rungs"]
        .as_array()
        .expect("rungs")
        .iter()
        .find(|rung| rung["timeframe"] == Timeframe::SECOND_1.as_str())
        .expect("the 1s rung");
    assert_eq!(second["months_held"].as_u64(), Some(2), "{body}");

    // A RESTART BUILDS IT COLD OVER THE SAME FILE: the same bytes.
    let restarted = store.site();
    let moved = crate::server::census_rollup::answer(&site, query).2;
    let fresh = crate::server::census_rollup::answer(&restarted, query).2;
    assert_eq!(moved, fresh, "a moved roll-up equals a fresh build");

    // THE SEGMENT'S ROLL-UP sums every underlying: here the one.
    let (code, _, whole) = rollup_get(&site, "feed=gdfl&segment=FNO");
    assert_eq!(code, 200, "{whole}");
    assert_eq!(served(&whole), walked(&store.fno), "{whole}");
    assert!(whole["underlying"].is_null() && whole["exchange"].is_null());
}

/// **EVERY WRONG ASK IS REFUSED BY NAME, AND AN EMPTY ONE IS EMPTY** (the
/// operator's rule 4: garbage parameters, empty, missing): an unknown
/// parameter, no feed, an unknown feed, no segment, an exchange without an
/// underlying, an underlying the census does not hold (with and without its
/// exchange), and a census with nothing in the segment -- each its own code
/// and sentence, never a silent default and never a whole read.
#[test]
fn every_wrong_rollup_ask_is_refused_by_name() {
    let store = Store::new("rollup-refusals", 3);
    let site = store.site();
    start(&site, &[Vendor::Gdfl]).expect("starts");
    let _ready = wait_ready(&site);
    let mark = store.reads().len();
    for (query, code, error, says) in [
        ("feed=gdfl&segment=FNO&since=3", 400, "bad_request", "since"),
        ("segment=FNO", 400, "refused", ""),
        ("feed=nosuch&segment=FNO", 400, "refused", "nosuch"),
        ("feed=gdfl", 400, "bad_request", "segment"),
        ("feed=gdfl&segment=OPTIONS", 400, "bad_request", "OPTIONS"),
        (
            "feed=gdfl&segment=FNO&exchange=NSE",
            400,
            "bad_request",
            "exchange needs",
        ),
        (
            "feed=gdfl&segment=FNO&underlying=BANKNIFTY",
            404,
            "not_held",
            "BANKNIFTY",
        ),
        (
            "feed=gdfl&segment=FNO&exchange=BSE&underlying=NIFTY",
            404,
            "not_held",
            "BSE NIFTY",
        ),
        (
            "feed=gdfl&segment=FNO&underlying=nifty%20%2F",
            400,
            "bad_request",
            "underlying",
        ),
    ] {
        let (status, _, body) = rollup_get(&site, query);
        assert_eq!(status, code, "{query}: {body}");
        // A FEED REFUSAL IS THE SERVER'S ONE SHAPE (`no_such_feed_json`,
        // `refused`); every other is `error`, by code.
        let named = body["error"].as_str().map_or("refused", |code| code);
        assert!(named.contains(error), "{query}: {body}");
        assert!(
            body.to_string().contains(says),
            "{query} names {says:?}: {body}"
        );
    }
    // EMPTY: GDFL holds nothing in CASH; the answer is empty, and says so.
    let (status, _, body) = rollup_get(&site, "feed=gdfl&segment=CASH");
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["months"].as_array().map(Vec::len), Some(0), "{body}");
    assert_eq!(body["rungs"].as_array().map(Vec::len), Some(0), "{body}");
    // SINGLE: one spot month in INDEX.
    let (status, _, body) = rollup_get(&site, "feed=gdfl&segment=INDEX&underlying=NIFTY");
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["months"].as_array().map(Vec::len), Some(1), "{body}");
    assert_eq!(body["months"][0]["rows"].as_u64(), Some(22_500), "{body}");
    assert_eq!(store.reads().len(), mark, "a refusal read the census");
    // A FEED WITH NO CENSUS AT ALL is answered absent and empty, never a 500.
    let (status, _, body) = rollup_get(&site, "feed=zerodha&segment=INDEX");
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["state"], "absent", "{body}");
    assert_eq!(body["months"].as_array().map(Vec::len), Some(0), "{body}");
}

/// **A MONTH OUTSIDE THE CALENDAR OF RECORD OWES `null`, AND SO DOES ITS
/// RUNG'S TOTAL**: never a count that silently begins at the record's edge.
#[test]
fn a_month_outside_the_calendar_owes_null_never_a_partial_count() {
    let month = YearMonth::new(2017, 6).expect("a month");
    assert_eq!(crate::server::census_rollup::days_owed(month), None);
    let inside = YearMonth::new(2019, 3).expect("a month");
    assert!(crate::server::census_rollup::days_owed(inside).is_some());
    let cell = crate::server::facets::MonthCell {
        keys: 1,
        rows: 10,
        first: Some(MARCH),
        last: Some(MARCH + 10_000_000),
    };
    assert_eq!(
        crate::server::census_rollup::days_held(inside, &cell),
        Some(1)
    );
    let none = crate::server::facets::MonthCell::default();
    assert_eq!(crate::server::census_rollup::days_held(inside, &none), None);
}

/// **TWO ANSWERS OVER A CURRENT INDEX RENDER SIDE BY SIDE** (R3-09): the
/// facet index (and so every `/census/facets.json` and `/census/rollup.json`
/// answer) is read under a READ lock once it is current, so a second reader
/// never queues behind the first one's render. Measured before this on the
/// shadow of the real store: sixty clients on NIFTY's FNO underlying, max
/// 141 ms, every render behind one `Mutex`. Each reader here holds its lock
/// until it has seen the other inside, for at most two seconds: behind a
/// mutex the first sees only itself and the test fails.
#[test]
fn two_answers_over_a_current_index_render_side_by_side() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let store = Store::new("side-by-side", 4);
    let site = store.site();
    let (_stamps, now) = vendor_census(&site, Vendor::Gdfl);
    let slot = Arc::clone(now.slot());
    // BUILT ONCE, so both readers below find it current.
    let built = crate::server::facets::index(&site, Vendor::Gdfl, Segment::Fno, &slot, |index| {
        index.generation()
    });
    assert!(built > 0);
    let inside = Arc::new(AtomicUsize::new(0));
    let readers: Vec<_> = (0..2)
        .map(|_| {
            let (site, slot, inside) =
                (Loaded::clone(&site), Arc::clone(&slot), Arc::clone(&inside));
            std::thread::spawn(move || {
                crate::server::facets::index(&site, Vendor::Gdfl, Segment::Fno, &slot, |_| {
                    let _me = inside.fetch_add(1, Ordering::SeqCst);
                    let deadline = Instant::now() + Duration::from_secs(2);
                    while inside.load(Ordering::SeqCst) < 2 && Instant::now() < deadline {
                        std::thread::sleep(Duration::from_millis(1));
                    }
                    inside.load(Ordering::SeqCst)
                })
            })
        })
        .collect();
    for reader in readers {
        assert_eq!(
            reader.join().expect("the reader"),
            2,
            "a reader rendered alone: the other queued behind its lock"
        );
    }
}

/// **A REQUEST ON THE PREVIOUS READ NEVER MOVES THE INDEX BACK** (R3-09). A
/// request handed the previous read while the background re-read ran, and
/// reaching the facet index after that re-read moved it on, is answered from
/// the moved index: never a build back over the older census on a request
/// thread (a whole build, which the next current request would undo), and
/// never an answer older than the read it was handed.
#[test]
fn a_request_on_the_previous_read_never_moves_the_index_back() {
    let mut store = Store::new("never-back", 64);
    let site = store.site();
    start(&site, &[Vendor::Gdfl]).expect("the warm thread starts");
    let _ready = wait_ready(&site);
    let (_stamps, before) = vendor_census(&site, Vendor::Gdfl);
    let old = Arc::clone(before.slot());
    let (was, _) = fno_generation(&site);
    store.append(&[row(option_key(9_000), 4)]);
    // THE RE-READ LANDS, and the background build moves the index with it.
    let deadline = Instant::now() + Duration::from_mins(1);
    let now = loop {
        let (generation, freshness) = fno_generation(&site);
        let (_stamps, now) = vendor_census(&site, Vendor::Gdfl);
        let vendor_generation = now.census().and_then(census::VendorCensus::generation);
        let facets = row_of(&warmth(&site), FACETS, Vendor::Gdfl).generation;
        if freshness == Freshness::Current && generation == was + 1 && facets == vendor_generation {
            break now;
        }
        assert!(
            Instant::now() < deadline,
            "never moved: {generation} {freshness:?}"
        );
        std::thread::sleep(Duration::from_millis(5));
    };
    let index = |slot: &Arc<Slot>| {
        crate::server::facets::index(&site, Vendor::Gdfl, Segment::Fno, slot, |index| {
            index.generation()
        })
    };
    assert_eq!(index(now.slot()), was + 1, "the moved index");
    assert_eq!(
        index(&old),
        was + 1,
        "the request on the previous read was built the index back over it"
    );
    assert_eq!(
        index(now.slot()),
        was + 1,
        "and the index still reflects the current read"
    );
}

/// **EVERY WARM BUILD IS A LINE IN THE LOG** (R3-09; the operator's rule 3:
/// every state change saved and searchable). Each cache the warm builds for
/// a feed is a `cache built` line naming the cache, the feed and the census
/// it stood on, and the whole warm is a `caches warmed at start` line, both
/// read back from the installed sink through the reader `/logs` renders
/// from. Six entries (NIFTY spot and five options) single this store's lines
/// out of every other test's.
#[test]
fn every_warm_build_is_a_line_in_the_log() {
    let _shared = crate::emitted::sink();
    let from = crate::emitted::mark();
    let store = Store::new("logged", 5);
    let site = store.site();
    start(&site, &[Vendor::Gdfl]).expect("the warm thread starts");
    let _ready = wait_ready(&site);
    let deadline = Instant::now() + Duration::from_mins(1);
    loop {
        let built = crate::emitted::landed(from, "api.census", "cache built");
        let named = |cache: &str| {
            built.iter().any(|record| {
                crate::emitted::says(record, "cache", cache)
                    && crate::emitted::says(record, "feed", "gdfl")
                    && crate::emitted::says(record, "state", "ready")
                    && crate::emitted::counts(record, "entries", 6)
            })
        };
        let warmed = crate::emitted::landed(from, "api.census", "caches warmed at start");
        if WARMED.into_iter().all(named) && !warmed.is_empty() {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "the warm is not in the log: {built:?} {warmed:?}"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// `GET /census/facets.json?<query>` through the route's own function, the
/// body parsed.
fn facets_get(site: &Site, query: &str) -> (u16, axum::http::HeaderMap, serde_json::Value) {
    let (code, headers, body) = crate::server::facets::answer(site, query);
    let value = serde_json::from_str(&body).unwrap_or_else(|why| panic!("{why}: {body}"));
    (code.as_u16(), headers, value)
}

/// Waits until a whole build of GDFL's FNO index over `slot` is running.
fn until_building(site: &Site, slot: &Arc<Slot>) {
    let deadline = Instant::now() + Duration::from_mins(1);
    while crate::server::facets::building(site, Vendor::Gdfl, Segment::Fno, slot).is_none() {
        assert!(Instant::now() < deadline, "the build never began");
        std::thread::sleep(Duration::from_millis(2));
    }
}

/// **AN INDEX BEING BUILT IS ANSWERED "WARMING", NEVER WAITED FOR** (R3-09;
/// RA-046). A first build of the FNO facet index takes a second and a half;
/// the option lists, the segment's lists and the coverage roll-up asked
/// meanwhile are each answered at once: `503 index_warming`, naming the
/// cache, the feed, the segment and how long the build has run, with
/// `Retry-After`. Once it is built they are answered `200`. Before, each
/// waited the build out behind its lock (5.6 s on average for the first
/// roll-up over 20 restarts of the shadow of the real store).
#[test]
fn an_index_being_built_is_answered_warming_never_waited_for() {
    let store = Store::new("warming-answer", 64);
    let site = store.site();
    let (_stamps, now) = vendor_census(&site, Vendor::Gdfl);
    let slot = Arc::clone(now.slot());
    // THE OTHER SEGMENTS BUILT FIRST, so only FNO's build is slow.
    for segment in [Segment::Index, Segment::Cash] {
        crate::server::facets::index(&site, Vendor::Gdfl, segment, &slot, |_| ());
    }
    slow_read::set(
        &store.root.join(crate::server::facets::SLOW_BUILD_KEY),
        1_500,
    );
    let builder = {
        let (site, slot) = (Loaded::clone(&site), Arc::clone(&slot));
        std::thread::spawn(move || {
            crate::server::facets::index(&site, Vendor::Gdfl, Segment::Fno, &slot, |index| {
                index.generation()
            })
        })
    };
    until_building(&site, &slot);
    for query in [
        "feed=gdfl&segment=FNO&underlying=NIFTY",
        "feed=gdfl&segment=FNO",
        "feed=gdfl&segment=FNO&underlying=NIFTY&rows=census-tuples-v1",
        "feed=gdfl",
    ] {
        let began = Instant::now();
        let (code, headers, body) = facets_get(&site, query);
        assert!(
            began.elapsed() < Duration::from_millis(500),
            "{query} waited"
        );
        assert_eq!(code, 503, "{query}: {body}");
        assert_eq!(body["error"], "index_warming", "{query}: {body}");
        assert_eq!(body["cache"], FACETS);
        assert_eq!(body["feed"], "gdfl");
        assert_eq!(body["segment"], "FNO");
        assert!(body["since_ms"].as_u64().is_some(), "{body}");
        assert_eq!(header(&headers, "retry-after"), "1");
    }
    let began = Instant::now();
    let (code, headers, body) = rollup_get(&site, "feed=gdfl&segment=FNO&underlying=NIFTY");
    assert!(
        began.elapsed() < Duration::from_millis(500),
        "the roll-up waited"
    );
    assert_eq!(code, 503, "{body}");
    assert_eq!(body["error"], "index_warming");
    assert_eq!(header(&headers, "retry-after"), "1");
    // THE INDEX SEGMENT IS NOT BEING BUILT: answered as ever.
    assert_eq!(rollup_get(&site, "feed=gdfl&segment=INDEX").0, 200);
    let built = builder.join().expect("the builder");
    slow_read::set(&store.root.join(crate::server::facets::SLOW_BUILD_KEY), 0);
    let (code, headers, body) = rollup_get(&site, "feed=gdfl&segment=FNO&underlying=NIFTY");
    assert_eq!(code, 200, "{body}");
    assert_eq!(body["generation"], built);
    assert_eq!(header(&headers, CENSUS_REFRESHING_HEADER), "0");
    assert_eq!(
        facets_get(&site, "feed=gdfl&segment=FNO&underlying=NIFTY").0,
        200
    );
}

/// **A REBUILD IS ANSWERED FROM THE PREVIOUS INDEX, SAYING SO** (R3-09;
/// RA-032). A census file replaced by rename (what a compaction or a new
/// epoch leaves) cannot move the FNO index by its appended entries: the
/// background re-read builds it whole again, here for a second and a half.
/// Meanwhile the option lists and the roll-up are answered at once from the
/// PREVIOUS index, at its own generation, with
/// `x-brutex-census-refreshing: 1`; once the rebuild lands, from the new
/// one, current. Before, every answer of the segment waited for the rebuild
/// behind the index's write lock.
#[test]
fn a_rebuild_is_answered_from_the_previous_index_saying_so() {
    let mut store = Store::new("rebuild-previous", 64);
    let site = store.site();
    start(&site, &[Vendor::Gdfl]).expect("the warm thread starts");
    let _ready = wait_ready(&site);
    let (was, _) = fno_generation(&site);
    slow_read::set(
        &store.root.join(crate::server::facets::SLOW_BUILD_KEY),
        1_500,
    );
    // ONE MORE ENTRY, AND THE FILE REPLACED BY RENAME: a new inode.
    store.append(&[row(option_key(777), 2)]);
    let bytes = std::fs::read(&store.fno_path).expect("the epoch");
    let aside = store.fno_path.with_extension("renamed");
    std::fs::write(&aside, bytes).expect("written");
    std::fs::rename(&aside, &store.fno_path).expect("renamed over");
    let deadline = Instant::now() + Duration::from_mins(1);
    let now = loop {
        let (generation, freshness) = fno_generation(&site);
        if freshness == Freshness::Current && generation == was + 1 {
            break vendor_census(&site, Vendor::Gdfl).1;
        }
        assert!(Instant::now() < deadline, "never re-read: {generation}");
        std::thread::sleep(Duration::from_millis(2));
    };
    until_building(&site, now.slot());
    let began = Instant::now();
    let (code, headers, body) = facets_get(&site, "feed=gdfl&segment=FNO&underlying=NIFTY");
    assert!(
        began.elapsed() < Duration::from_millis(500),
        "the lists waited"
    );
    assert_eq!(code, 200, "{body}");
    assert_eq!(body["generation"], was, "the previous index");
    assert_eq!(header(&headers, CENSUS_REFRESHING_HEADER), "1");
    let (code, headers, body) = rollup_get(&site, "feed=gdfl&segment=FNO&underlying=NIFTY");
    assert!(
        began.elapsed() < Duration::from_millis(500),
        "the roll-up waited"
    );
    assert_eq!(code, 200, "{body}");
    assert_eq!(body["generation"], was);
    assert_eq!(header(&headers, CENSUS_REFRESHING_HEADER), "1");
    slow_read::set(&store.root.join(crate::server::facets::SLOW_BUILD_KEY), 0);
    let deadline = Instant::now() + Duration::from_mins(1);
    loop {
        let (code, headers, body) = rollup_get(&site, "feed=gdfl&segment=FNO&underlying=NIFTY");
        assert_eq!(code, 200, "{body}");
        if body["generation"] == was + 1 && header(&headers, CENSUS_REFRESHING_HEADER) == "0" {
            break;
        }
        assert!(Instant::now() < deadline, "never rebuilt: {body}");
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// Replaces `path` by rename with `bytes`: a new inode, as a compaction or a
/// new epoch leaves the file, so no index can be moved forward by a tail.
fn replace_by_rename(path: &Path, bytes: &[u8]) {
    let aside = path.with_extension("renamed");
    std::fs::write(&aside, bytes).expect("written");
    std::fs::rename(&aside, path).expect("renamed over");
}

/// Waits until `vendor_census` hands GDFL's FNO part at `generation`, the
/// census on disk: the background re-read has published its read.
fn until_published(site: &Site, generation: u64) -> VendorCensusNow {
    let deadline = Instant::now() + Duration::from_mins(1);
    loop {
        let (at, freshness) = fno_generation(site);
        if freshness == Freshness::Current && at == generation {
            return vendor_census(site, Vendor::Gdfl).1;
        }
        assert!(Instant::now() < deadline, "never published: {at}");
        std::thread::sleep(Duration::from_millis(2));
    }
}

/// Waits until the roll-up of NIFTY's options answers current at
/// `generation`: the rebuild landed.
fn until_rebuilt(site: &Site, generation: u64) {
    let deadline = Instant::now() + Duration::from_mins(1);
    loop {
        let (code, headers, body) = rollup_get(site, "feed=gdfl&segment=FNO&underlying=NIFTY");
        if code == 200
            && body["generation"] == generation
            && header(&headers, CENSUS_REFRESHING_HEADER) == "0"
        {
            return;
        }
        assert!(Instant::now() < deadline, "never rebuilt: {code} {body}");
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// **A REQUEST BETWEEN THE CENSUS LANDING AND ITS INDEX BUILD NEVER WAITS**
/// (R3-09 repair 1, blocking item 1). The background re-read publishes the
/// new read, then counts the journal and warms the index map (283 to 1,657 ms
/// on the shadow of the real store) before it builds the FNO facet index.
/// The build was marked pending only once it began, so a request handed the
/// new read inside that window found nothing pending and built the whole
/// index itself, or waited for the re-read's build: 7.55 s and 12.21 s
/// measured. Here the window is held open a second and a half and the
/// build takes as long: every list and roll-up asked inside it answers at
/// once, from the previous index, said to be previous, and
/// `/census/warm.json` lists the facet index `refreshing`, not `ready`.
#[test]
fn a_request_between_the_census_landing_and_its_index_build_never_waits() {
    let mut store = Store::new("landing-window", 64);
    let site = store.site();
    start(&site, &[Vendor::Gdfl]).expect("the warm thread starts");
    let _ready = wait_ready(&site);
    let (was, _) = fno_generation(&site);
    slow_read::set(&store.root.join(SLOW_DERIVED_KEY), 1_500);
    slow_read::set(
        &store.root.join(crate::server::facets::SLOW_BUILD_KEY),
        1_500,
    );
    store.append(&[row(option_key(778), 2)]);
    replace_by_rename(
        &store.fno_path,
        &std::fs::read(&store.fno_path).expect("the epoch"),
    );
    let _now = until_published(&site, was + 1);
    for query in [
        "feed=gdfl&segment=FNO&underlying=NIFTY",
        "feed=gdfl&segment=FNO",
        "feed=gdfl",
    ] {
        let began = Instant::now();
        let (code, headers, body) = facets_get(&site, query);
        assert!(
            began.elapsed() < Duration::from_millis(500),
            "{query} waited {:?}",
            began.elapsed()
        );
        assert_eq!(code, 200, "{query}: {body}");
        assert_eq!(header(&headers, CENSUS_REFRESHING_HEADER), "1", "{query}");
        let stale: u64 = header(&headers, CENSUS_STALE_HEADER)
            .parse()
            .expect("a count");
        let age: u64 = header(&headers, CENSUS_AGE_HEADER)
            .parse()
            .expect("a count");
        assert!(stale <= age, "{query}: stale {stale} ms, age {age} ms");
    }
    let began = Instant::now();
    let (code, headers, body) = rollup_get(&site, "feed=gdfl&segment=FNO&underlying=NIFTY");
    assert!(
        began.elapsed() < Duration::from_millis(500),
        "the roll-up waited {:?}",
        began.elapsed()
    );
    assert_eq!(code, 200, "{body}");
    assert_eq!(body["generation"], was, "the previous index");
    assert_eq!(header(&headers, CENSUS_REFRESHING_HEADER), "1");
    // THE ROWS ARE NAMED WARMING, never built on the request's thread.
    let began = Instant::now();
    let (code, _headers, body) = facets_get(
        &site,
        "feed=gdfl&segment=FNO&underlying=NIFTY&rows=census-tuples-v1",
    );
    assert!(
        began.elapsed() < Duration::from_millis(500),
        "the rows waited"
    );
    assert_eq!(code, 503, "{body}");
    assert_eq!(body["error"], "index_warming", "{body}");
    // AND THE PAGE SAYS SO while the rebuild runs.
    let deadline = Instant::now() + Duration::from_mins(1);
    while row_of(&warmth(&site), FACETS, Vendor::Gdfl).state != WarmState::Refreshing {
        assert!(Instant::now() < deadline, "{:?}", warmth(&site));
        std::thread::sleep(Duration::from_millis(5));
    }
    slow_read::set(&store.root.join(SLOW_DERIVED_KEY), 0);
    slow_read::set(&store.root.join(crate::server::facets::SLOW_BUILD_KEY), 0);
    until_rebuilt(&site, was + 1);
    let rows = wait_ready(&site);
    assert_eq!(row_of(&rows, FACETS, Vendor::Gdfl).state, WarmState::Ready);
}

/// **THE OLD INDEX IS DROPPED OFF THE LOCK: NO READER STALLS AT THE SWAP**
/// (R3-09 repair 1, blocking item 2). Dropping GDFL's 2.07 M-entry FNO index
/// takes 0.8 to 1.9 s, and the rebuild dropped it under the write lock while
/// every reader of the segment queued on the read lock. Here every index the
/// warm builds takes a second to drop; readers looping on the option lists
/// across a rebuild never wait more than a fraction of that.
#[test]
fn the_old_index_is_dropped_off_the_lock_and_no_reader_stalls_at_the_swap() {
    let mut store = Store::new("swap-drop", 64);
    let site = store.site();
    slow_read::set(
        &store.root.join(crate::server::facets::SLOW_DROP_KEY),
        1_000,
    );
    start(&site, &[Vendor::Gdfl]).expect("the warm thread starts");
    let _ready = wait_ready(&site);
    slow_read::set(&store.root.join(crate::server::facets::SLOW_DROP_KEY), 0);
    let (was, _) = fno_generation(&site);
    store.append(&[row(option_key(779), 2)]);
    replace_by_rename(
        &store.fno_path,
        &std::fs::read(&store.fno_path).expect("the epoch"),
    );
    let readers: Vec<_> = (0..3)
        .map(|n| {
            let site = Loaded::clone(&site);
            std::thread::spawn(move || {
                let query = if n == 0 {
                    "feed=gdfl&segment=FNO"
                } else {
                    "feed=gdfl&segment=FNO&underlying=NIFTY"
                };
                let deadline = Instant::now() + Duration::from_mins(1);
                let mut worst = Duration::ZERO;
                loop {
                    let began = Instant::now();
                    let (code, headers, body) = facets_get(&site, query);
                    worst = worst.max(began.elapsed());
                    if code == 200
                        && body["generation"] == was + 1
                        && header(&headers, CENSUS_REFRESHING_HEADER) == "0"
                    {
                        return worst;
                    }
                    assert!(Instant::now() < deadline, "never rebuilt: {code} {body}");
                }
            })
        })
        .collect();
    for reader in readers {
        let worst = reader.join().expect("a reader");
        assert!(
            worst < Duration::from_millis(400),
            "a reader stalled {worst:?} at the swap"
        );
    }
}

/// **AN INDEX OVER A REFUSED CENSUS IS NEVER THE PREVIOUS ANSWER** (R3-09
/// repair 1, blocking item 3). The census file truncated: refused, named
/// `census_unreadable`. The re-read moved the facet index to the EMPTY index
/// over the refused read, and once the file was restored that empty index
/// answered for the recovered census while it was rebuilt: 404 `not_held`
/// for NIFTY, `0 underlyings` for the segment, for 3 to 5 s on the shadow of
/// the real store. Now a refused census moves no index: while the recovered
/// read is rebuilt, the lists answer from the last read that loaded, said
/// to be previous, and never "not held".
#[test]
fn after_a_refused_census_recovers_the_lists_answer_from_the_last_read_that_loaded() {
    let store = Store::new("refused-recovers", 64);
    let site = store.site();
    start(&site, &[Vendor::Gdfl]).expect("the warm thread starts");
    let _ready = wait_ready(&site);
    let (was, _) = fno_generation(&site);
    let whole = std::fs::read(&store.fno_path).expect("the epoch");
    replace_by_rename(&store.fno_path, &whole[..whole.len() / 2]);
    let deadline = Instant::now() + Duration::from_mins(1);
    loop {
        let (code, _headers, body) = rollup_get(&site, "feed=gdfl&segment=FNO&underlying=NIFTY");
        let facets = row_of(&warmth(&site), FACETS, Vendor::Gdfl).state;
        if code == 503 && body["error"] == "census_unreadable" && facets == WarmState::Failed {
            break;
        }
        assert!(Instant::now() < deadline, "never refused: {code} {body}");
        std::thread::sleep(Duration::from_millis(5));
    }
    let rows = warmth(&site);
    // THE LISTS' REFUSAL SAYS HOW CURRENT IT IS, as the roll-up's does: it
    // carried no freshness header at all.
    let (code, headers, body) = facets_get(&site, "feed=gdfl&segment=FNO&underlying=NIFTY");
    assert_eq!(code, 503, "{body}");
    assert_eq!(body["error"], "census_unreadable", "{body}");
    assert!(matches!(
        header(&headers, CENSUS_REFRESHING_HEADER).as_str(),
        "0" | "1"
    ));
    let _stale: u64 = header(&headers, CENSUS_STALE_HEADER)
        .parse()
        .expect("a count");
    let refused = row_of(&rows, CENSUS, Vendor::Gdfl);
    assert_eq!(refused.state, WarmState::Failed, "{refused:?}");
    let stayed = row_of(&rows, FACETS, Vendor::Gdfl).clone();
    slow_read::set(
        &store.root.join(crate::server::facets::SLOW_BUILD_KEY),
        1_500,
    );
    replace_by_rename(&store.fno_path, &whole);
    let deadline = Instant::now() + Duration::from_mins(1);
    loop {
        let (_stamps, now) = vendor_census(&site, Vendor::Gdfl);
        let held = now
            .census()
            .is_some_and(|census| matches!(census.state, census::Census::Held { .. }));
        if held && now.freshness() == Freshness::Current {
            break;
        }
        assert!(Instant::now() < deadline, "never read again");
        std::thread::sleep(Duration::from_millis(2));
    }
    for _ in 0..3 {
        let began = Instant::now();
        let (code, headers, body) = rollup_get(&site, "feed=gdfl&segment=FNO&underlying=NIFTY");
        assert!(began.elapsed() < Duration::from_millis(500), "waited");
        assert_eq!(
            code, 200,
            "never 'not held' over a census that holds it: {body}"
        );
        assert_eq!(body["generation"], was, "the last read that loaded: {body}");
        assert_eq!(header(&headers, CENSUS_REFRESHING_HEADER), "1");
        let (code, headers, body) = facets_get(&site, "feed=gdfl&segment=FNO");
        assert_eq!(code, 200, "{body}");
        assert_eq!(
            body["underlyings"].as_array().map(Vec::len),
            Some(1),
            "never 0 underlyings over a census that holds NIFTY: {body}"
        );
        assert_eq!(header(&headers, CENSUS_REFRESHING_HEADER), "1");
        std::thread::sleep(Duration::from_millis(50));
    }
    slow_read::set(&store.root.join(crate::server::facets::SLOW_BUILD_KEY), 0);
    until_rebuilt(&site, was);
    // AND THE PAGE SAID SO while the census was refused.
    assert!(
        stayed
            .reason
            .as_deref()
            .is_some_and(|why| why.contains("last read that loaded")),
        "the page says the index stayed: {stayed:?}"
    );
}

/// **A REQUEST THAT FINDS THE INDEX BEHIND STARTS ITS BUILD, NEVER WAITS FOR
/// IT** (R3-09 repair 1). An index left behind the read the cell keeps with
/// nothing building it -- a re-read that died before its build, a read made
/// on a request's thread -- was built whole by the first request that met
/// it, on that request's thread. Now that request starts the build on a
/// thread of its own and answers from the previous index at once; the next
/// request after the build is current.
#[test]
fn a_request_that_finds_the_index_behind_starts_its_build_and_never_waits() {
    let store = Store::new("kick", 64);
    let twin = Store::new("kick-twin", 128);
    let site = store.site();
    start(&site, &[Vendor::Gdfl]).expect("the warm thread starts");
    let _ready = wait_ready(&site);
    let (generation, _) = fno_generation(&site);
    // LEFT BEHIND: the index built over another file's read, which no tail
    // continues and the cell does not keep (of a higher generation, so the
    // index is not taken to reflect a later read than it, `later`).
    let other = Slot::detached(census::read_vendor(&twin.root, Vendor::Gdfl));
    crate::server::facets::index(&site, Vendor::Gdfl, Segment::Fno, &other, |_| ());
    slow_read::set(
        &store.root.join(crate::server::facets::SLOW_BUILD_KEY),
        1_500,
    );
    let _shared = crate::emitted::sink();
    let from = crate::emitted::mark();
    let began = Instant::now();
    let (code, headers, body) = rollup_get(&site, "feed=gdfl&segment=FNO&underlying=NIFTY");
    assert!(
        began.elapsed() < Duration::from_millis(500),
        "the request built it: {:?}",
        began.elapsed()
    );
    assert_eq!(code, 200, "{body}");
    assert_eq!(header(&headers, CENSUS_REFRESHING_HEADER), "1");
    // THE START IS A LINE IN THE LOG, naming the feed and the segment.
    let started = crate::emitted::landed(
        from,
        "api.census",
        "facet index rebuild started by a request",
    );
    assert!(
        started.iter().any(|record| {
            crate::emitted::says(record, "feed", "gdfl")
                && crate::emitted::says(record, "segment", "FNO")
        }),
        "the request's build is not in the log: {started:?}"
    );
    // ONE BUILD, STARTED BY THAT REQUEST: the next asks are told it runs.
    let began = Instant::now();
    let (code, _headers, body) = facets_get(
        &site,
        "feed=gdfl&segment=FNO&underlying=NIFTY&rows=census-tuples-v1",
    );
    assert!(
        began.elapsed() < Duration::from_millis(500),
        "the rows waited"
    );
    assert_eq!(code, 503, "{body}");
    slow_read::set(&store.root.join(crate::server::facets::SLOW_BUILD_KEY), 0);
    until_rebuilt(&site, generation);
}

/// A value that takes `pause` to drop, and says which thread dropped it.
struct SlowToFree {
    pause: Duration,
    freed_on: std::sync::mpsc::Sender<Option<String>>,
}

impl Drop for SlowToFree {
    fn drop(&mut self) {
        std::thread::sleep(self.pause);
        let _sent = self
            .freed_on
            .send(std::thread::current().name().map(str::to_owned));
    }
}

/// **WHAT A READER LETS GO OF IS FREED OFF ITS THREAD AND OFF EVERY LOCK**
/// (R3-09 repair 1, blocking items 2 and 4). A replaced census read, its
/// sorted entries and a replaced facet index (two million entries at GDFL's
/// size, 0.8 to 1.9 s to free on the shadow of the real store) are handed to
/// [`retire`], which must return at once and free them on `brutex-retire`,
/// in order, never on the thread that let go.
#[test]
fn a_retired_value_is_freed_on_the_retire_thread_never_on_the_caller() {
    let (sender, freed) = std::sync::mpsc::channel();
    let began = Instant::now();
    for _ in 0..2 {
        retire(SlowToFree {
            pause: Duration::from_millis(400),
            freed_on: sender.clone(),
        });
    }
    retire_any(Some(SlowToFree {
        pause: Duration::ZERO,
        freed_on: sender.clone(),
    }));
    retire_any::<SlowToFree>(None);
    assert!(
        began.elapsed() < Duration::from_millis(200),
        "the caller paid for the free: {:?}",
        began.elapsed()
    );
    for _ in 0..3 {
        let on = freed
            .recv_timeout(Duration::from_secs(30))
            .expect("every retired value is freed");
        assert_eq!(on.as_deref(), Some("brutex-retire"));
    }
    assert!(
        freed.recv_timeout(Duration::from_millis(100)).is_err(),
        "None retires nothing"
    );
}

/// **THE SEGMENT IS CASE-FOLDED, AS THE FEED AND THE UNDERLYING ARE** (R3-09
/// repair 1, a follow-up of its check): `segment=fno` was refused with a 400
/// while `feed=GDFL` and `underlying=nifty` were read as `gdfl` and `NIFTY`.
/// A word that is no segment in any case is still refused, by name.
#[test]
fn the_segment_is_case_folded_as_the_feed_and_the_underlying_are() {
    let store = Store::new("segment-case", 5);
    let site = store.site();
    start(&site, &[Vendor::Gdfl]).expect("the warm thread starts");
    let _ready = wait_ready(&site);
    let (code, _headers, upper) = rollup_get(&site, "feed=gdfl&segment=FNO&underlying=NIFTY");
    assert_eq!(code, 200, "{upper}");
    let (code, _headers, lower) = rollup_get(&site, "feed=GDFL&segment=fno&underlying=nifty");
    assert_eq!(code, 200, "{lower}");
    assert_eq!(lower, upper);
    let (code, _headers, upper) = facets_get(&site, "feed=gdfl&segment=FNO&underlying=NIFTY");
    assert_eq!(code, 200, "{upper}");
    let (code, _headers, lower) = facets_get(&site, "feed=gdfl&segment=Fno&underlying=nifty");
    assert_eq!(code, 200, "{lower}");
    assert_eq!(lower, upper);
    for query in ["feed=gdfl&segment=fnox", "feed=gdfl&segment=f%20no"] {
        let (code, _headers, body) = rollup_get(&site, query);
        assert_eq!(code, 400, "{query}: {body}");
        let (code, _headers, body) = facets_get(&site, query);
        assert_eq!(code, 400, "{query}: {body}");
    }
    // NO SEGMENT: the roll-up needs one; the lists answer their root.
    let (code, _headers, body) = rollup_get(&site, "feed=gdfl&segment=");
    assert_eq!(code, 400, "{body}");
    let (code, _headers, body) = facets_get(&site, "feed=gdfl&segment=");
    assert_eq!(code, 200, "{body}");
    assert!(body["segments"].is_array(), "{body}");
}

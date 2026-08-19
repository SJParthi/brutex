//! The broker path, end to end, over a real socket — and never a real broker.
//!
//! # What this file is the proof of
//!
//! `docs/05-decisions.md` D-0035 stopped one function short of a working vendor
//! pull: the credential port was defined and nothing implemented it, and the
//! ingest page has said "what is missing is one join" ever since. D-0051 wrote
//! the credential read in Rust and `pull::ingest::from_window` is the join.
//!
//! This drives the whole of it: a socket answers with a broker-shaped JSON body,
//! `HttpSource::window_async` fetches and decodes it, and `ingest::from_window`
//! puts the bars in a store and counts them in a manifest. **The bytes on disk
//! at the end are produced by exactly the code a live pull would run**, with one
//! substitution — the descriptor's `base_url` points at localhost instead of at
//! `api.dhan.co`.
//!
//! # Why there is no live call here, and no credential
//!
//! Not caution for its own sake. A test that reaches a broker cannot run in CI,
//! costs rate-limit budget that belongs to the operator, and fails for reasons
//! that are not the code's — so it would be quarantined within a week and then
//! deleted. This runs in 20 ms, offline, every time, and fails only when
//! something here is actually broken.
//!
//! `crates/pull/src/ssm.rs` proves the credential read against AWS's own
//! published `SigV4` vectors, which is the same trade: the specification is a
//! better oracle than a network.
//!
//! # The one thing this does NOT prove
//!
//! That `api.dhan.co` answers in the shape `crates/pull/src/vendor.rs` declares.
//! `docs/06-limits.md` records it — this cited §35, which does not exist; the
//! file has 34 sections, and a citation to a section nobody wrote reads as a
//! claim someone checked. Every field the descriptor names is
//! **UNVERIFIED against a live body**, and the first real call is what verifies
//! it — which is exactly why `decode_body` refuses a wrong `envelope` by name
//! and lists the keys it did find (D-0049), instead of guessing.

#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail, and these lints exist to \
              keep panics out of the crate rather than out of its tests"
)]

use brutex_core::instrument::{Contract, Expiry, Kind, OptionSide};
use brutex_core::price::Paisa;
use std::io::{Read as _, Write as _};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use brutex_core::vendor::Vendor;
use pull::fetch::BarRequest;
use pull::http::{Credential, HttpSource};
use pull::ingest::{self, Ingested, Plan};
use pull::manifest::{EntryKey, Manifest, manifest_path};
use pull::session::{Day, Window};
use pull::vendor::{
    Auth, AuthScheme, DateFormat, FieldNames, HttpSpec, Method, Param, ParamValue, PathSegment,
    PriceScale, RangeEnd, ResponseShape, TimestampEncoding,
};
use store::path::{Timeframe, YearMonth};

/// Distinguishes two scratch trees taken in the same process.
static NEXT: AtomicU64 = AtomicU64::new(0);

/// A temporary tree that removes itself, as the other suites use.
struct Scratch {
    root: PathBuf,
}

impl Scratch {
    fn new(tag: &str) -> Self {
        let serial = NEXT.fetch_add(1, Ordering::Relaxed);
        let mut root = std::env::temp_dir();
        root.push(format!(
            "brutex-pull-broker-{}-{tag}-{serial}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).expect("a scratch root");
        Self { root }
    }

    fn store(&self) -> PathBuf {
        let dir = self.root.join("STORE");
        std::fs::create_dir_all(&dir).expect("a scratch store");
        dir
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// A broker that answers once, on loopback, and reports what it was asked.
///
/// Raw sockets rather than a test HTTP framework: `crates/pull` takes `tokio`
/// without the `net` feature, and a test is not a reason to widen a dependency
/// this workspace counts as carefully as this one does.
fn broker(body: &str) -> (String, std::sync::mpsc::Receiver<String>) {
    let socket = TcpListener::bind("127.0.0.1:0").expect("a loopback port");
    let addr = socket.local_addr().expect("an address");
    let answer = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\
         Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let Ok((mut stream, _)) = socket.accept() else {
            return;
        };
        let mut buf = [0u8; 8192];
        let n = stream.read(&mut buf).unwrap_or(0);
        let _ = tx.send(String::from_utf8_lossy(buf.get(..n).unwrap_or(&[])).into_owned());
        let _ = stream.write_all(answer.as_bytes());
        let _ = stream.flush();
    });
    (format!("http://{addr}"), rx)
}

/// The descriptor, pointed at a socket instead of at a broker.
///
/// Every other field is the shape `crates/pull/src/vendor.rs` gives Dhan —
/// `access-token` raw, dashed dates, an exclusive range end, parallel arrays at
/// the top level, rupee prices, epoch seconds. Changing only the URL is what
/// makes this a test of the vendor path rather than of a fixture.
fn spec(base_url: &'static str) -> HttpSpec {
    HttpSpec {
        // As the shipped Dhan row: no body-level error contract has been
        // read for this vendor. See `HttpSpec::error_names`.
        error_names: None,
        // ONE ENDPOINT FOR THIS FIXTURE. The per-rung split is exercised
        // against the SHIPPED descriptor, in
        // `http::tests::dhan_serves_the_two_rungs_from_two_endpoints_and_only_one_takes_an_interval`,
        // where it can be checked against the vendor's own pages.
        rung_routes: &[],
        // NO CONTRACT LOOKUP IN THIS FIXTURE. Groww's is exercised against the
        // shipped descriptor in
        // `groww_declares_the_expired_fno_lookup_and_the_others_declare_none`,
        // where it can be checked against the vendor's own pages.
        fno: pull::vendor::FnoAccess::None,
        // DHAN'S REAL REQUIRED FIELDS, read first-hand from
        // dhanhq.co/docs/v2/historical-data. This is what `DH-905 securityId
        // is required` was reporting the absence of.
        // A test spec: no floor, so the window is used as given.
        history_floor: pull::vendor::HistoryFloor::Unstated,
        // No published cap at any rung, and no rung field on the wire — which
        // is Dhan's own shape. The split happens in `api::server::fetch_chunks`
        // and this suite calls `HttpSource` directly, so neither is what it is
        // testing.
        window_caps: &[],
        granularity_tokens: &[],
        listings: &[
            pull::vendor::ListingWords {
                listing: pull::vendor::Listing::Index,
                segment: "IDX_I",
                kind: "INDEX",
            },
            pull::vendor::ListingWords {
                listing: pull::vendor::Listing::Equity,
                segment: "NSE_EQ",
                kind: "EQUITY",
            },
        ],
        params: &[
            Param {
                name: "securityId",
                value: ParamValue::InstrumentId,
            },
            Param {
                name: "exchangeSegment",
                value: ParamValue::Fixed("IDX_I"),
            },
            Param {
                name: "instrument",
                value: ParamValue::Fixed("INDEX"),
            },
            Param {
                name: "fromDate",
                value: ParamValue::From,
            },
            Param {
                name: "toDate",
                value: ParamValue::To,
            },
        ],
        extra_headers: &[],
        base_url,
        bars_path: &[
            PathSegment::Literal("v2"),
            PathSegment::Literal("charts"),
            PathSegment::Literal("historical"),
        ],
        method: Method::Post,
        auth: Auth {
            header: "access-token",
            scheme: AuthScheme::Raw,
            key_field: None,
        },
        date_format: DateFormat::DashedYmd,
        range_end: RangeEnd::Exclusive,
        response: ResponseShape::ParallelArrays { envelope: None },
        fields: FieldNames {
            open: "open",
            high: "high",
            low: "low",
            close: "close",
            volume: "volume",
            timestamp: "timestamp",
            open_interest: None,
        },
        timestamps: TimestampEncoding::EpochSecondsUtc,
        prices: PriceScale::Rupees,
        budget: pull::vendor::Budget {
            per_second: None,
            per_minute: None,
            per_day: None,
        },
        pooling: pull::vendor::Pooling::PerVendor,
    }
}

/// 2025-07-01, inclusive both ends: one trading day.
fn window() -> Window {
    Window::new(
        Day::new(2025, 7, 1).expect("a real day"),
        Day::new(2025, 7, 1).expect("a real day"),
    )
    .expect("forwards")
}

fn request() -> BarRequest {
    BarRequest {
        // NIFTY at Dhan, from their own worked example.
        instrument_id: "13".to_owned(),
        listing: pull::vendor::Listing::Equity,
        window: window(),
        granularity: pull::vendor::Granularity::Minute1,
    }
}

/// The plan a spot pull runs under.
/// The same plan, but for one option contract rather than the spot index.
///
/// Only two fields move: the segment, and the contract. That is the whole
/// difference between filing a bar under `NSE/INDEX/NIFTY/...` and filing it
/// under `NSE/FNO/NIFTY/<contract>/...`, and it is why the contract is carried
/// on the plan rather than derived from the vendor's answer — which names no
/// series at all.
fn option_plan(request: &BarRequest, contract: Contract) -> Plan<'_> {
    Plan {
        segment: "FNO",
        contract: Some(contract),
        ..plan(request)
    }
}

fn plan(request: &BarRequest) -> Plan<'_> {
    Plan {
        columns: pull::csv::Columns::Gdfl,
        request,
        encoding: TimestampEncoding::EpochSecondsUtc,
        // THE ONE FIELD THAT IS NOT THE DESCRIPTOR'S. `http::decode_body`
        // already converted rupees to paisa, so the plan must say `Paisa` or
        // every price is multiplied by 100 a second time — the trap
        // `pull::http::DECODED_PRICE_SCALE` exists to name.
        scale: PriceScale::Paisa,
        vendor: Vendor::Dhan,
        exchange: "NSE",
        segment: "INDEX",
        contract: None,
    }
}

/// Four one-minute bars inside the 2025-07-01 session, as a broker sends them.
///
/// 09:15, 09:16, 09:17 and 09:18 IST — 03:45 UTC onward. The first draft of
/// this fixture used 08:45 IST and every bar was dropped "before the session
/// open", which was the session filter being RIGHT about a test that was
/// wrong. Prices carry paise so
/// the conversion is exercised rather than assumed.
const BODY: &str = r#"{
  "open":  [24500.75, 24510.25, 24515.00, 24520.50],
  "high":  [24512.00, 24518.75, 24522.25, 24530.00],
  "low":   [24498.50, 24505.00, 24511.75, 24518.00],
  "close": [24510.25, 24515.00, 24520.50, 24528.75],
  "volume":[1200, 980, 1450, 1100],
  "timestamp":[1751341500, 1751341560, 1751341620, 1751341680]
}"#;

fn fetch(url: &str) -> pull::fetch::RawWindow {
    let source = HttpSource::new(
        spec(Box::leak(url.to_owned().into_boxed_str())),
        Credential::token("A-FAKE-TOKEN".to_owned()),
    )
    .expect("a client builds");
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("a runtime")
        .block_on(source.window_async(&request()))
        .expect("the broker answered")
}

/// The census key the bars should be filed under.
fn key(instrument: &str) -> EntryKey {
    EntryKey {
        contract: None,
        exchange: brutex_core::instrument::Exchange::Nse,
        segment: brutex_core::instrument::Segment::Index,
        symbol: brutex_core::symbol::Symbol::new(instrument).expect("a legal symbol"),
        timeframe: Timeframe::MINUTE_1,
        month: YearMonth::new(2025, 7).expect("July 2025"),
    }
}

/// Reads the census back off disk, exactly as `/store` does.
fn census(store_root: &Path) -> Manifest {
    let path = manifest_path(store_root, Vendor::Dhan);
    let bytes = std::fs::read(&path).expect("the census was written");
    let (header, entries) = bytes
        .split_at_checked(32_768)
        .expect("a full header region");
    Manifest::open(Vendor::Dhan, header, entries).expect("a readable census")
}

// ===========================================================================
// The join
// ===========================================================================

/// **THE WHOLE BROKER PATH, WITH NO BROKER.**
///
/// Socket answers → `window_async` fetches → `decode_body` converts →
/// `from_window` lands, folds, appends and counts. Every assertion below is
/// about bytes that reached the disk.
#[test]
fn a_window_fetched_from_a_broker_lands_in_the_store_and_is_counted() {
    let scratch = Scratch::new("lands");
    let store_root = scratch.store();
    let (url, seen) = broker(BODY);

    let raw = fetch(&url);
    assert_eq!(raw.rows.len(), 4, "four bars came back");

    let request = request();
    let done: Ingested = ingest::from_window(&raw, "NIFTY", &url, &store_root, plan(&request));

    // ── THE BOOKS BALANCE ───────────────────────────────────────────────
    // Every row read is stored, folded or dropped. This is the same identity
    // the ingest page prints, and it is the first thing that breaks when a
    // path is joined wrongly.
    assert_eq!(done.members, 1, "one window is one member");
    assert_eq!(done.rows_read, 4);
    assert_eq!(
        done.rows_read,
        done.bars_stored + done.rows_folded + done.census.total() as usize,
        "read = stored + folded + dropped: {done:?}"
    );
    assert_eq!(done.bars_stored, 4, "all four are inside the session");
    assert!(
        done.failures.is_empty(),
        "nothing failed: {:?}",
        done.failures
    );

    // ── THE BARS ARE ACTUALLY ON THE DISK ───────────────────────────────
    let bars = store_root
        .join("bars")
        .join("dhan")
        .join("NSE")
        .join("INDEX")
        .join("NIFTY")
        .join("1min")
        .join("2025-07.bin");
    assert!(
        bars.is_file(),
        "the month file exists at {}",
        bars.display()
    );
    assert!(
        std::fs::metadata(&bars).expect("readable").len() > 0,
        "and it is not empty"
    );

    // ── AND THE CENSUS COUNTS THEM ──────────────────────────────────────
    // A store that holds rows its counter denies is the worst outcome
    // `ingest` names, so this is asserted rather than assumed.
    let manifest = census(&store_root);
    let entry = manifest.entry(&key("NIFTY")).expect("the month is counted");
    assert_eq!(entry.rows, 4, "the counter agrees with the store");
    // THE MINUTE'S OWN ROW IS STILL FOUR. What changed is that it is no longer
    // the ONLY row: the same four bars are folded into every rung derived from
    // the minute, so the census holds one key per rung and its total is the sum
    // across them. Both are asserted against the rule that produced them —
    // `derived_count` — so a rung added to the store moves this with it.
    let rungs = 1_u64 + pull::ingest::derived_count(store::path::Timeframe::MINUTE_1) as u64;
    assert_eq!(
        manifest.keys(),
        rungs,
        "one census key per rung: the one fetched and each one derived"
    );
    assert!(
        manifest.total_rows() >= 4 && manifest.total_rows() <= 4 * rungs,
        "every derived rung folds four minute bars into at most four bars, \
         never more: {} rows over {rungs} rung(s)",
        manifest.total_rows()
    );

    // ── THE REQUEST WAS THE DESCRIPTOR'S ────────────────────────────────
    let sent = seen
        .recv_timeout(core::time::Duration::from_secs(5))
        .expect("the broker was contacted");
    assert!(sent.starts_with("POST /v2/charts/historical"), "{sent}");
    assert!(
        sent.contains("access-token: A-FAKE-TOKEN"),
        "the descriptor's own header carries the credential: {sent}"
    );
    assert!(sent.contains("2025-07-01"), "fromDate: {sent}");
    assert!(
        sent.contains("2025-07-02"),
        "toDate is EXCLUSIVE — the day after the operator's last day: {sent}"
    );

    // AND THE THREE FIELDS DHAN REFUSED THE REQUEST FOR.
    //
    // The body used to be `{"fromDate":…,"toDate":…}` and nothing else, which
    // is the entirety of `DH-905 securityId is required`. Asserting the dates
    // alone passed throughout that, because the dates were never the problem.
    assert!(
        sent.contains("\"securityId\":\"13\""),
        "the instrument must be NAMED, and 13 is NIFTY in Dhan's own worked \
         example: {sent}"
    );
    assert!(
        sent.contains("\"exchangeSegment\":\"IDX_I\""),
        "the segment Dhan's Annexure gives for an index: {sent}"
    );
    assert!(
        sent.contains("\"instrument\":\"INDEX\""),
        "the instrument type, also required: {sent}"
    );
}

/// **THE PAISE SURVIVE THE WHOLE PATH.**
///
/// `24500.75` is 2,450,075 paisa. The archive path has always got this right;
/// this asserts the broker path does too, all the way to the file — the defect
/// that reached the tree once already and would silently rewrite every
/// fractional price in the store.
#[test]
fn a_fractional_price_reaches_the_store_with_its_paise_intact() {
    let scratch = Scratch::new("paise");
    let store_root = scratch.store();
    let (url, _seen) = broker(BODY);

    let raw = fetch(&url);
    assert_eq!(
        raw.rows.first().map(|r| r.open),
        Some(2_450_075),
        "24500.75 rupees is 2450075 paisa BEFORE it is stored"
    );

    let request = request();
    let done = ingest::from_window(&raw, "NIFTY", &url, &store_root, plan(&request));
    assert_eq!(done.bars_stored, 4);

    // And read back off the disk, through the store's own reader.
    let path = store::path::StorePath::new(store::path::PathParts {
        vendor: Vendor::Dhan,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "NIFTY",
        contract: None,
        timeframe: Timeframe::MINUTE_1,
        month: YearMonth::new(2025, 7).expect("July 2025"),
        file: store::path::FileKind::Bars,
    })
    .expect("a legal path");
    let file = store::file::BarFile::open_or_create(
        &store_root,
        path,
        // The same folding `ingest` uses: the id is a CROSS-CHECK the store
        // stamps in the header and verifies on reopen, never an index, so any
        // 32 bits of the hash serve and the low half is the standard fold.
        #[expect(
            clippy::cast_possible_truncation,
            reason = "matches pull::ingest's own derivation, which the store \
                      verifies against on reopen — a different fold here would \
                      make this test open a file the ingest path cannot"
        )]
        {
            brutex_core::universe::fnv1a("NIFTY") as u32
        },
    )
    .expect("the month file reopens");
    assert_eq!(file.header().n_valid, 4, "four bars are in the file");
}

/// A window the broker answers with nothing stores nothing, and says so — it
/// does not report a successful pull of zero bars.
#[test]
fn an_empty_answer_stores_nothing_and_writes_no_census() {
    let scratch = Scratch::new("empty");
    let store_root = scratch.store();
    let (url, _seen) =
        broker(r#"{"open":[],"high":[],"low":[],"close":[],"volume":[],"timestamp":[]}"#);

    let raw = fetch(&url);
    assert!(raw.rows.is_empty(), "the broker sent no bars");

    let request = request();
    let done = ingest::from_window(&raw, "NIFTY", &url, &store_root, plan(&request));
    assert_eq!(done.rows_read, 0);
    assert_eq!(done.bars_stored, 0);
    assert_eq!(
        done.counted, 0,
        "nothing was counted, because nothing landed"
    );
    assert!(done.failures.is_empty(), "an empty window is not a failure");

    // AND NOTHING WAS WRITTEN. A census published for a run that stored no bar
    // would be a counter describing an install that did not happen.
    assert!(
        !manifest_path(&store_root, Vendor::Dhan).exists(),
        "no census is published when nothing changed"
    );
}

/// Two identical pulls leave the store byte for byte as one did.
///
/// `CLAUDE.md` §3 rule 5: same inputs, same outputs, reruns are safe. The
/// broker path must not be the one that breaks it — a scheduled pull that
/// re-fetches yesterday must not double-count it.
#[test]
fn pulling_the_same_window_twice_changes_nothing_the_second_time() {
    let scratch = Scratch::new("idempotent");
    let store_root = scratch.store();
    let request = request();

    let (url_a, _a) = broker(BODY);
    let first = ingest::from_window(&fetch(&url_a), "NIFTY", &url_a, &store_root, plan(&request));
    assert_eq!(first.bars_stored, 4);
    let after_first = std::fs::read(manifest_path(&store_root, Vendor::Dhan)).expect("a census");
    let first_total = census(&store_root).total_rows();

    let (url_b, _b) = broker(BODY);
    let second = ingest::from_window(&fetch(&url_b), "NIFTY", &url_b, &store_root, plan(&request));
    let after_second = std::fs::read(manifest_path(&store_root, Vendor::Dhan)).expect("a census");

    // `bars_stored` is bars OFFERED to the store after folding, not bars newly
    // written — the file's own append refuses a duplicate, so the second run
    // offers the same four and writes none of them. Worth pinning, because the
    // ingest page shows this number and "4 stored" on a rerun that stored
    // nothing new is exactly the sort of counter an operator would misread.
    assert_eq!(
        second.bars_stored, 4,
        "the same four are offered again; what changes is that none are written"
    );
    assert_eq!(
        after_first, after_second,
        "the census is byte-for-byte identical after a rerun"
    );
    // FOUR PER RUNG, AND THE RUNGS ARE COMPUTED. A one-minute window now
    // writes the minute it fetched plus every rung derived from it, so the
    // census totals four bars for the minute plus whatever each fold produced —
    // and the POINT of this assertion is unchanged: a rerun adds nothing.
    // Asserted against the first run's own total rather than a literal, which
    // is the only form that survives a rung being added to the store.
    assert_eq!(
        census(&store_root).total_rows(),
        first_total,
        "a rerun adds no rows to any rung, derived or fetched"
    );
}

/// The broker path and the folder path are the same code below the seam.
///
/// Not a claim about the source — a claim about the *counters*. Both are
/// `from_members`, so a window of four bars balances its books the same way
/// whichever side it arrived from, and a future edit that special-cases one of
/// them breaks this.
#[test]
fn the_broker_path_and_the_folder_path_are_one_implementation() {
    let scratch = Scratch::new("shared");
    let store_root = scratch.store();
    let (url, _seen) = broker(BODY);
    let request = request();

    let raw = fetch(&url);
    let done = ingest::from_window(&raw, "NIFTY", &url, &store_root, plan(&request));

    // Every field `Ingested` carries is filled by the shared loop, so a broker
    // pull reports the same shape a folder pull does — including the ones a
    // hand-rolled second implementation would have forgotten.
    assert_eq!(done.members, 1);
    // ONE COUNTER ROW PER FILE — the rung fetched and every rung derived from
    // it. Asserted against the RULE so a rung added to `Timeframe::KNOWN` moves
    // this expectation with it rather than breaking a literal.
    assert_eq!(
        done.counted,
        1 + pull::ingest::derived_count(store::path::Timeframe::MINUTE_1),
        "the counter row was recorded for the fetched rung and each derived one"
    );
    assert_eq!(done.rows_folded, 0, "one-minute bars, nothing to fold");
    assert_eq!(
        done.census.total(),
        0,
        "every row is inside the session and the window"
    );
    assert!(done.failures.is_empty());
}

// ===========================================================================
// The rung
// ===========================================================================

/// **A DAILY PULL LANDS UNDER `1day/`, AND NOT UNDER `1min/`.**
///
/// The same four one-minute bodies, the same socket, the same landing code —
/// one field different. `Granularity::Day1` is what `store_timeframe` converts
/// into `Timeframe::DAY_1`, and `Timeframe::DAY_1.as_str()` is the directory,
/// so this test is what stands between a day's bar and the minute directory it
/// would otherwise be indistinguishable inside.
///
/// The bar length lives in the PATH in this store, never in the row. That is
/// why the negative half matters as much as the positive one: a build that
/// filed daily bars under `1min/` would write a well-formed file with a valid
/// checksum and an accurate counter, and no later reader could tell those bars
/// from real one-minute data.
///
/// The FOLD is asserted with it, because the same field decides both. Four
/// one-minute bars inside one session become **one** daily bar whose open is
/// the first, high the maximum, low the minimum, close the last and volume the
/// sum — if this landed four rows in a `1day/` file, the directory would be
/// right and its contents would be minute bars.
#[test]
fn a_daily_pull_lands_under_the_day_directory_and_folds_the_session_into_one_bar() {
    let scratch = Scratch::new("daily");
    let store_root = scratch.store();
    let (url, _seen) = broker(BODY);

    // THE ONE CONVERSION, asserted before the run rather than inferred from
    // it: the rung the operator picks and the directory the store writes are
    // tied by this function and by nothing else.
    assert_eq!(
        pull::vendor::Granularity::Day1.store_timeframe(),
        Some(Timeframe::DAY_1),
        "the day rung must carry a store timeframe or nothing can be filed at it"
    );
    assert_eq!(Timeframe::DAY_1.as_str(), "1day");

    let raw = fetch(&url);
    let request = BarRequest {
        granularity: pull::vendor::Granularity::Day1,
        ..request()
    };
    let done: Ingested = ingest::from_window(&raw, "NIFTY", &url, &store_root, plan(&request));

    assert!(
        done.failures.is_empty(),
        "a daily pull is not a refusal: {:?}",
        done.failures
    );
    assert_eq!(done.rows_read, 4, "the broker still sent four rows");
    assert_eq!(done.bars_stored, 1, "one session is ONE daily bar");
    assert_eq!(done.rows_folded, 3, "and the other three folded into it");
    assert_eq!(
        done.rows_read,
        done.bars_stored + done.rows_folded + done.census.total() as usize,
        "read = stored + folded + dropped: {done:?}"
    );

    // ── THE DIRECTORY, BOTH WAYS ────────────────────────────────────────
    let at = |rung: &str| {
        store_root
            .join("bars")
            .join("dhan")
            .join("NSE")
            .join("INDEX")
            .join("NIFTY")
            .join(rung)
            .join("2025-07.bin")
    };
    assert!(
        at("1day").is_file(),
        "the daily month file exists at {}",
        at("1day").display()
    );
    assert!(
        !at("1min").exists(),
        "and NOTHING was written under 1min/ — a daily bar filed there is one \
         no reader can tell from a real one-minute bar"
    );

    // ── THE CENSUS KEYS IT AT THE DAY RUNG TOO ──────────────────────────
    // The counter carries the timeframe, so a bar filed at one rung and counted
    // at another is a store that disagrees with itself about what it holds.
    let manifest = census(&store_root);
    assert!(
        manifest
            .entry(&EntryKey {
                contract: None,
                timeframe: Timeframe::MINUTE_1,
                ..key("NIFTY")
            })
            .is_none(),
        "nothing is counted at the minute rung"
    );
    let entry = manifest
        .entry(&EntryKey {
            contract: None,
            timeframe: Timeframe::DAY_1,
            ..key("NIFTY")
        })
        .expect("the month is counted at the day rung");
    assert_eq!(entry.rows, 1, "one daily bar, counted once");

    // ── AND IT IS A REAL AGGREGATE, not the first minute wearing a date ──
    let path = store::path::StorePath::new(store::path::PathParts {
        vendor: Vendor::Dhan,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "NIFTY",
        contract: None,
        timeframe: Timeframe::DAY_1,
        month: YearMonth::new(2025, 7).expect("July 2025"),
        file: store::path::FileKind::Bars,
    })
    .expect("a legal path");
    let file = store::file::BarFile::open_or_create(&store_root, path, {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "matches pull::ingest's own derivation, which the store \
                      verifies against on reopen — a different fold here would \
                      make this test open a file the ingest path cannot"
        )]
        {
            brutex_core::universe::fnv1a("NIFTY") as u32
        }
    })
    .expect("the day month file reopens");
    assert_eq!(file.header().n_valid, 1);
    let bar = file.read_record(0).expect("the one daily bar");
    // Paisa, from BODY: open 24500.75, high 24530.00, low 24498.50,
    // close 24528.75, volume 1200+980+1450+1100.
    assert_eq!(bar.open, 2_450_075, "the FIRST minute's open");
    assert_eq!(bar.high, 2_453_000, "the session's maximum");
    assert_eq!(bar.low, 2_449_850, "the session's minimum");
    assert_eq!(bar.close, 2_452_875, "the LAST minute's close");
    assert_eq!(bar.volume, 4_730, "and the volumes summed");
}

/// **A RUNG THE STORE CANNOT CARRY IS REFUSED BY NAME, AND THE REFUSAL SAYS
/// WHICH RUNG.**
///
/// `Granularity::Second1` is a real rung of the ladder — both archive feeds
/// serve it — and `crates/store` has no directory for it.
/// `CLAUDE.md` §4 leaves two options there and only one of them is allowed:
/// refuse loudly, or substitute. A substitution would file one-second bars
/// under `1min/`, and the bar length is the path here.
///
/// The refusal must NAME the rung. "Invalid timeframe" would send an operator
/// to guess which of eleven rungs the run was at.
#[test]
fn a_rung_the_store_cannot_carry_is_refused_and_the_refusal_names_it() {
    let scratch = Scratch::new("unstorable");
    let store_root = scratch.store();
    let (url, _seen) = broker(BODY);

    // `Second5`, NOT `Second1`, AND THE SWAP IS THE POINT OF THE TEST.
    //
    // This used `Second1` as "the rung crates/store has not been widened to
    // carry", and that stopped being true: the store ships `Timeframe::SECOND_1`
    // now, because one second is the rung the two archive feeds' files actually
    // hold and an operator could ask for the data he had bought while nothing
    // could file it.
    //
    // The BEHAVIOUR under test is unchanged and still needs proving — a rung
    // with no directory must refuse by name and write nothing — so it moves to
    // a rung that genuinely has none. Five seconds is requestable, is on the
    // ladder, and `Timeframe::KNOWN` holds nothing for it.
    assert_eq!(
        pull::vendor::Granularity::Second5.store_timeframe(),
        None,
        "this test is about a rung crates/store has NOT been widened to carry"
    );
    assert!(
        pull::vendor::Granularity::Second5.is_requestable(),
        "and one a caller may actually ask for, so the refusal is reachable"
    );

    let raw = fetch(&url);
    let request = BarRequest {
        granularity: pull::vendor::Granularity::Second5,
        ..request()
    };
    let done: Ingested = ingest::from_window(&raw, "NIFTY", &url, &store_root, plan(&request));

    assert_eq!(done.bars_stored, 0, "nothing was written");
    assert_eq!(done.counted, 0, "and nothing was counted");
    assert_eq!(
        done.rows_read, 4,
        "the rows that arrived are still reported — reporting zero would blame \
         the vendor for a refusal that is ours"
    );
    assert_eq!(done.failures.len(), 1, "one refusal, not one per member");
    let why = &done.failures.first().expect("the refusal").why;
    assert!(
        why.contains("5s"),
        "the refusal must NAME the rung it refused: {why}"
    );
    assert!(
        why.contains("1min") && why.contains("1day"),
        "and say which rungs this build does store: {why}"
    );

    // AND NOT ONE BYTE ON THE DISK, at any rung. A refusal that still wrote
    // the file somewhere would be the substitution this exists to prevent.
    assert!(
        !store_root.join("bars").exists(),
        "no bar file was opened at any rung"
    );
    assert!(
        !manifest_path(&store_root, Vendor::Dhan).exists(),
        "and no census was published"
    );
}

/// GROWW DECLARES THE TWO-STEP CONTRACT LOOKUP; NOBODY ELSE DOES.
///
/// Read against the SHIPPED descriptor, because the thing under test is what
/// this build would put on a socket. The endpoints and field names are quoted
/// from groww.in/trade-api/docs/curl/backtesting.
#[test]
fn groww_declares_the_expired_fno_lookup_and_the_others_declare_none() {
    use pull::vendor::{Feed, PathSegment, Transport};

    let joined = |segs: &[PathSegment]| {
        segs.iter()
            .map(|s| match *s {
                PathSegment::Literal(w) => w,
                PathSegment::Value { placeholder, .. } => placeholder,
            })
            .collect::<Vec<_>>()
            .join("/")
    };

    let Transport::Http(groww) = Feed::Groww.descriptor().transport else {
        panic!("Groww is an HTTP broker");
    };
    let fno = groww
        .fno
        .by_name()
        .expect("Groww is addressed by contract name");
    assert_eq!(joined(fno.expiries_path), "v1/historical/expiries");
    assert_eq!(joined(fno.contracts_path), "v1/historical/contracts");
    assert_eq!(fno.expiries_field, "expiries");
    assert_eq!(fno.contracts_field, "contracts");
    assert_eq!(
        fno.from_year, 2020,
        "the vendor's own sentence: FNO data is available from 2020"
    );

    // THE CONTRACT LOOKUP IS FED BY THE EXPIRY LOOKUP, so it must carry an
    // expiry date and the expiry lookup must not.
    let names = |ps: &[pull::vendor::Param]| ps.iter().map(|p| p.name).collect::<Vec<_>>();
    assert_eq!(
        names(fno.expiries_params),
        vec!["exchange", "underlying_symbol", "year", "month"]
    );
    assert_eq!(
        names(fno.contracts_params),
        vec!["exchange", "underlying_symbol", "expiry_date"]
    );

    // AND DHAN DECLARES NONE, which is a fact about the vendor rather than a
    // gap in this build: `/v2/charts/rollingoption` answers an ATM-relative
    // series whose underlying contract changes every week, so there is no
    // contract name in it to discover.
    let Transport::Http(dhan) = Feed::Dhan.descriptor().transport else {
        panic!("Dhan is an HTTP broker");
    };
    // NO NAME LOOKUP, AND THAT IS NOT NO DATA. See D-0193: this assertion used
    // to read `dhan.fno.is_none()`, and the field could only say "no
    // discovery", so "no discovery" was read as "serves nothing" for months
    // while the vendor was serving five years of expired options by strike
    // offset.
    assert!(
        dhan.fno.by_name().is_none(),
        "Dhan publishes no contract NAME to discover"
    );
    assert!(
        dhan.fno.by_offset().is_some(),
        "and it does serve expired options, addressed by distance from the money"
    );
}

/// A DISCOVERY FIELD CANNOT REACH A BARS REQUEST.
///
/// The four discovery values have no meaning in a `BarRequest`, and the one
/// thing that must never happen is a value being invented for them — the field
/// would go on the wire and the answer would be filed as bars.
#[test]
fn a_discovery_parameter_in_a_bars_request_is_refused_by_name() {
    use pull::vendor::{Feed, Transport};

    // NO SHIPPED FEED CARRIES ONE, which is the invariant. The refusal exists
    // for the descriptor row somebody writes next.
    for feed in Feed::ALL {
        let Transport::Http(spec) = feed.descriptor().transport else {
            continue;
        };
        for p in spec
            .params
            .iter()
            .chain(spec.rung_routes.iter().flat_map(|r| r.params))
        {
            assert!(
                !matches!(
                    p.value,
                    pull::vendor::ParamValue::Underlying
                        | pull::vendor::ParamValue::Year
                        | pull::vendor::ParamValue::Month
                        | pull::vendor::ParamValue::ExpiryDate
                ),
                "{feed}'s bars request carries the discovery field {:?}",
                p.name
            );
        }
    }
}

// ===========================================================================
// The contract level
// ===========================================================================

/// **AN OPTION'S BARS LAND UNDER THE CONTRACT, NOT UNDER THE UNDERLYING.**
///
/// The store has always had a contract level — `store::path::PathParts` carries
/// `Option<Contract>` and renders one directory deeper when it is `Some`. What
/// did not exist was any way for a caller to reach it: `pull::ingest` wrote
/// `contract: None` at three fixed sites, one of them carrying a note that the
/// contract path was "not reachable from here yet".
///
/// The consequence was not a missing feature, it was a collision. Every
/// contract of every expiry would have filed into `NSE/FNO/NIFTY/<rung>/` —
/// the same file — so a month of two hundred contracts would have appended two
/// hundred series into one, and the next one's pull would have appended to
/// that. `CLAUDE.md` §3 rule 8 makes such a file unrenameable after the fact.
///
/// This asserts the two things that keep them apart: the bars are reachable at
/// the contract path, and two different contracts of the same underlying do not
/// share a file.
#[test]
fn two_contracts_of_one_underlying_do_not_share_a_file() {
    let scratch = Scratch::new("contractpath");
    let store_root = scratch.store();
    let (url, _seen) = broker(BODY);
    let raw = fetch(&url);
    let request = request();

    let expiry = Expiry::new(2026, 9, 24).expect("a real expiry");
    let call = Contract::of(Kind::Option {
        expiry,
        strike: Paisa::from_raw(2_465_000),
        side: OptionSide::Call,
    })
    .expect("it renders");
    let put = Contract::of(Kind::Option {
        expiry,
        strike: Paisa::from_raw(2_465_000),
        side: OptionSide::Put,
    })
    .expect("it renders");
    assert_ne!(call, put, "a call and a put are different contracts");

    let first = ingest::from_window(
        &raw,
        "NIFTY",
        &url,
        &store_root,
        option_plan(&request, call),
    );
    assert!(
        first.bars_stored > 0,
        "the call's bars reached disk: {first:?}"
    );

    let second = ingest::from_window(&raw, "NIFTY", &url, &store_root, option_plan(&request, put));
    assert!(
        second.bars_stored > 0,
        "and so did the put's, into its OWN file rather than appending to the \
         call's: {second:?}"
    );

    // ── EACH CONTRACT'S OWN DIRECTORY ───────────────────────────────────
    // Counting files would prove nothing: `derive_all` writes a file per
    // derived rung, so the tree holds several either way. What distinguishes a
    // threaded contract from a dropped one is whether the contract's own name
    // is a directory in the path -- with `None` both writes render the SAME
    // path and the second appends into the first's file.
    let mut bins = Vec::new();
    walk_bins(&store_root, &mut bins);
    for (what, contract) in [("call", call), ("put", put)] {
        assert!(
            bins.iter().any(|p| p.contains(contract.as_str())),
            "the {what}'s bars are filed under its own contract directory \
             `{}`; without it both contracts render one path and the second \
             appends into the first's file. Found: {bins:?}",
            contract.as_str()
        );
    }
    assert!(
        bins.iter().any(|p| p.contains("NIFTY")),
        "and the contract sits BELOW the underlying rather than replacing it: \
         {bins:?}"
    );
}

/// Every `.bin` under a root, as slash-joined strings.
fn walk_bins(dir: &Path, into: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_bins(&path, into);
        } else if path.extension().is_some_and(|e| e == "bin") {
            into.push(path.display().to_string());
        }
    }
}

// ===========================================================================
// Decoded rows, bar and overlay together
// ===========================================================================

/// **THE OPTION'S BARS AND ITS iv/spot LAND TOGETHER, UNDER ONE CONTRACT.**
///
/// This is the join `/pull/fno` needs for Dhan. Its answer arrives already
/// decoded — the spot and the implied volatility are lifted from the SAME
/// parallel arrays as the open and the close, so no raw-row shape in this build
/// can carry them — and `from_rows` is the door that takes them.
///
/// Three things are asserted because each fails differently:
/// the bars reach a `.bar` under the contract, the overlay reaches a `.ovl`
/// BESIDE it, and the census counts the month so `/store` can see it. A month
/// on disk the counter does not know about is one the ladder gate treats as
/// missing, so the next run refetches what is already there.
#[test]
fn a_bar_outside_the_session_is_dropped_and_counted_rather_than_stored() {
    // THE FIXTURE THAT CAUGHT IT. These two stamps are 08:00 and 08:01 IST on
    // 2025-07-01 — an hour and a quarter before the 09:15 open — and the test
    // below asserted, until today, that both reached the store. They did,
    // because `from_rows` never asked `Window::verdict` anything: it takes bars
    // that are already decoded and so never touched `fetch::land`, where every
    // other path's window and session checks live.
    let scratch = Scratch::new("outside-session");
    let store_root = scratch.store();
    let request = request();
    let bars = vec![
        store::format::Bar {
            ts_micros: 1_751_337_000_000_000,
            open: 100,
            high: 100,
            low: 100,
            close: 100,
            volume: 1,
            open_interest: store::format::OI_NULL,
        },
        store::format::Bar {
            ts_micros: 1_751_337_060_000_000,
            open: 100,
            high: 100,
            low: 100,
            close: 100,
            volume: 1,
            open_interest: store::format::OI_NULL,
        },
    ];

    let done = ingest::from_rows(
        &bars,
        &[],
        "NIFTY",
        "https://x",
        &store_root,
        plan(&request),
    );

    assert_eq!(
        done.bars_stored, 0,
        "a pre-open bar must not reach the store"
    );
    assert_eq!(done.rows_read, 2, "and both are still COUNTED as read");
    assert!(
        done.failures.is_empty(),
        "a declined bar is not a failure — it is a bar this engine refused, \
         which is a different answer: {:?}",
        done.failures
    );
    // AND THE BOOKS ARE NO LONGER VACUOUS. `balances()` was trivially true
    // while the census stayed empty, because there was nothing on either side.
    assert!(
        done.census.total() > 0,
        "the drop must be counted, or the run reports two rows read, none \
         stored, and no reason anywhere"
    );
}

#[test]
fn decoded_bars_and_their_overlays_are_filed_under_one_contract() {
    use brutex_core::instrument::{Expiry, Kind, OptionSide};
    use store::format::{Bar, OI_NULL, Overlay};

    let scratch = Scratch::new("fromrows");
    let store_root = scratch.store();
    let request = request();
    let expiry = Expiry::new(2025, 9, 25).expect("a real expiry");
    let contract = Contract::of(Kind::Option {
        expiry,
        strike: Paisa::from_raw(2_465_000),
        side: OptionSide::Call,
    })
    .expect("it renders");

    let bars = [
        Bar {
            ts_micros: 1_751_341_500_000_000,
            open: 10_000,
            high: 10_500,
            low: 9_500,
            close: 10_200,
            volume: 75,
            open_interest: 4200,
        },
        Bar {
            ts_micros: 1_751_341_560_000_000,
            open: 10_200,
            high: 10_600,
            low: 10_100,
            close: 10_400,
            volume: 50,
            open_interest: 4250,
        },
    ];
    let overlays = [
        Overlay {
            ts_micros: 1_751_341_500_000_000,
            spot: 2_465_005,
            iv_micros: 125_000,
        },
        // ONE THAT STATES ONLY A SPOT. Dhan answers a spot for a contract whose
        // iv it did not compute, and dropping the row would lose the spot.
        Overlay {
            ts_micros: 1_751_341_560_000_000,
            spot: 2_465_100,
            iv_micros: OI_NULL,
        },
    ];

    let plan = Plan {
        contract: Some(contract),
        segment: "FNO",
        ..plan(&request)
    };
    let done = ingest::from_rows(&bars, &overlays, "NIFTY", "https://x", &store_root, plan);
    assert!(
        done.failures.is_empty(),
        "nothing should refuse: {:?}",
        done.failures
    );
    assert_eq!(done.bars_stored, 2, "both bars reached the store");
    assert_eq!(done.counted, 1, "and the census counted the month");

    // ── THE TWO FILES, SIDE BY SIDE ─────────────────────────────────────
    let mut bins = Vec::new();
    walk_bins(&store_root, &mut bins);
    assert!(
        bins.iter().any(|p| p.contains(contract.as_str())),
        "the bars are filed under the contract: {bins:?}"
    );

    let mut ovls = Vec::new();
    walk_ext(&store_root, "ovl", &mut ovls);
    assert_eq!(ovls.len(), 1, "exactly one overlay file: {ovls:?}");
    assert!(
        ovls[0].contains(contract.as_str()),
        "and it sits BESIDE the bars, under the same contract: {ovls:?}"
    );
}

/// Every file with `ext` under a root, as slash-joined strings.
fn walk_ext(dir: &Path, ext: &str, into: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_ext(&path, ext, into);
        } else if path.extension().is_some_and(|e| e == ext) {
            into.push(path.display().to_string());
        }
    }
}

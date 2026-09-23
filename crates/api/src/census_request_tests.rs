#![cfg(test)]
//! Census-backed GET routes read an unchanged manifest once, and a rewritten
//! one again. D-0686.
//!
//! # What this pins
//!
//! `/calendar.json` (both branches), `/gaps.json` through `peer_calendar`, and
//! `/bars` through `locate_series` each called `census::read_all` directly, so
//! every request read every vendor's whole manifest and its cost grew with the
//! store. They now take the census through `census_now`, which is keyed on the
//! manifests' modified times.
//!
//! Two properties, and the second is the one that makes the first safe:
//!
//! * **a warm request reads no manifest bytes** — counted, not inferred, through
//!   `census::manifest_reads`, because a re-read of an unchanged manifest
//!   answers exactly what a cached census does and no response can tell them
//!   apart;
//! * **a rewritten manifest is read again on the next request** — D-0318's
//!   freshness, which a cache that never rebuilt would lose silently.
//!
//! The modified times are SET, not waited for. A filesystem's timestamp
//! resolution is not this test's to assume, and two writes inside one tick
//! would otherwise share a stamp and make the test's answer depend on the
//! clock rather than on the code.
#![expect(
    clippy::expect_used,
    reason = "finite owned fixtures and exact response assertions"
)]

use super::*;
use brutex_core::instrument::{Exchange, Segment};
use brutex_core::symbol::Symbol;
use pull::manifest::{Entry, EntryKey, Manifest, manifest_path};
use std::fs;
use std::time::{Duration, SystemTime};
use store::path::{Timeframe, YearMonth};

/// `minute` minutes after a fixed instant: every stamp here is SET, not waited
/// for.
fn at(minute: u64) -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000) + Duration::from_mins(minute)
}

/// One store root, one site over it, and Dhan's manifest as the only one.
struct Fixture {
    root: PathBuf,
    site: Loaded,
}

impl Fixture {
    fn new(name: &str) -> Self {
        let root = crate::scratch::path(name);
        let _ = fs::remove_dir_all(&root);
        let masters = root.join("masters");
        fs::create_dir_all(&masters).expect("empty offline masters");
        let site = Loaded::new(Site::load(&masters, &root));
        Self { root, site }
    }

    /// Dhan's manifest path: the one file every assertion here counts.
    fn manifest(&self) -> PathBuf {
        manifest_path(&self.root, Vendor::Dhan)
    }

    /// How many times this process has read the bytes of Dhan's manifest.
    fn reads(&self) -> usize {
        census::manifest_reads::of(&self.manifest())
    }

    /// Publish a Dhan manifest holding exactly `held`, stamped `minute` minutes
    /// after a fixed instant.
    ///
    /// Every call writes a whole new image, as a pull's rename does, and then
    /// sets the modified time explicitly so each publication has its own stamp
    /// whatever the filesystem's resolution.
    fn publish(&self, held: &[(Segment, &str)], minute: u64) {
        self.publish_for(Vendor::Dhan, held, minute);
    }

    /// A manifest image for `vendor` holding exactly `held`, one daily month
    /// each.
    fn image(vendor: Vendor, held: &[(Segment, &str)]) -> Vec<u8> {
        let month = YearMonth::new(2025, 5).expect("fixture month");
        let day = Day::new(2025, 5, 2).expect("fixture date");
        let ts = i64::from(day.days_from_epoch()) * 86_400_000_000 + 21_600_000_000;
        let mut manifest = Manifest::open(vendor, &[], &[]).expect("a genesis manifest");
        for (segment, symbol) in held {
            manifest
                .record(Entry {
                    key: EntryKey {
                        contract: None,
                        exchange: Exchange::Nse,
                        segment: *segment,
                        symbol: Symbol::new(symbol).expect("fixture symbol"),
                        timeframe: Timeframe::DAY_1,
                        month,
                    },
                    rows: 1,
                    first_ts_micros: ts,
                    last_ts_micros: ts,
                })
                .expect("one held month");
        }
        manifest.image()
    }

    /// [`Self::publish`] for any vendor's manifest.
    fn publish_for(&self, vendor: Vendor, held: &[(Segment, &str)], minute: u64) {
        self.publish_bytes(vendor, &Self::image(vendor, held), minute);
    }

    /// Install `bytes` as `vendor`'s manifest, stamped `minute`, the way a
    /// pull installs one: a whole temporary file, its stamp, then a rename.
    fn publish_bytes(&self, vendor: Vendor, bytes: &[u8], minute: u64) {
        let path = manifest_path(&self.root, vendor);
        fs::create_dir_all(path.parent().expect("manifest parent")).expect("manifest dir");
        let staged = path.with_extension("staged");
        fs::write(&staged, bytes).expect("stage the manifest");
        fs::File::options()
            .write(true)
            .open(&staged)
            .expect("the manifest just staged")
            .set_modified(at(minute))
            .expect("a filesystem that carries modified times");
        fs::rename(&staged, &path).expect("install the manifest");
    }

    /// Move `vendor`'s manifest stamp without touching its bytes.
    fn stamp(&self, vendor: Vendor, minute: u64) {
        fs::File::options()
            .write(true)
            .open(manifest_path(&self.root, vendor))
            .expect("an installed manifest")
            .set_modified(at(minute))
            .expect("a filesystem that carries modified times");
    }

    /// One real daily bar on 2025-05-`date`, so a calendar has a session and
    /// `/bars` has a file. The census is not touched.
    fn bar(&self, vendor: Vendor, segment: Segment, symbol: &str, date: u8) {
        let month = YearMonth::new(2025, 5).expect("fixture month");
        let day = Day::new(2025, 5, date).expect("fixture date");
        let ts = i64::from(day.days_from_epoch()) * 86_400_000_000 + 21_600_000_000;
        let path = store::path::StorePath::new(store::path::PathParts {
            vendor,
            exchange: "NSE",
            segment: segment.as_str(),
            symbol,
            contract: None,
            timeframe: Timeframe::DAY_1,
            month,
            file: store::path::FileKind::Bars,
        })
        .expect("fixture path");
        let hash = brutex_core::universe::fnv1a(symbol).to_le_bytes();
        let low = hash
            .first_chunk::<4>()
            .copied()
            .expect("an eight-byte hash has four low bytes");
        let mut file =
            store::file::BarFile::open_or_create(&self.root, path, u32::from_le_bytes(low))
                .expect("a daily bar file");
        file.append(&[store::format::Bar {
            ts_micros: ts,
            open: 100,
            high: 110,
            low: 90,
            close: 105,
            volume: 1,
            open_interest: i64::MIN,
        }])
        .expect("one daily bar");
    }

    /// `/calendar.json` with `query`: status and body.
    async fn calendar_answer(&self, query: &str) -> (axum::http::StatusCode, String) {
        let uri = format!("/calendar.json?{query}").parse().expect("uri");
        let (status, _, body) =
            calendar_json(axum::extract::State(Loaded::clone(&self.site)), uri).await;
        (status, body)
    }

    /// `/bars` with `query`: status and page.
    fn bars(&self, query: &str) -> (axum::http::StatusCode, String) {
        bars_html(&self.site, query)
    }

    /// Every vendor's census state as the cache serves it.
    fn states(&self) -> Vec<&'static str> {
        census_now(&self.site)
            .0
            .iter()
            .map(|census| census.state.name())
            .collect()
    }

    /// Dhan's census state as the cache serves it.
    fn dhan_state(&self) -> &'static str {
        census_now(&self.site)
            .0
            .iter()
            .find(|census| census.vendor == Vendor::Dhan)
            .map_or("no Dhan row", |census| census.state.name())
    }

    /// Every vendor's census state as a fresh read of the disk finds it.
    fn fresh_states(&self) -> Vec<&'static str> {
        census::read_all(&self.site.store_root)
            .iter()
            .map(|census| census.state.name())
            .collect()
    }

    /// `/calendar.json` with `query`, answered by the real handler.
    async fn calendar(&self, query: &str) -> axum::http::StatusCode {
        let uri = format!("/calendar.json?{query}").parse().expect("uri");
        let (status, _, _) =
            calendar_json(axum::extract::State(Loaded::clone(&self.site)), uri).await;
        status
    }

    /// The peer vote `/gaps.json` takes for a series the fixture does not hold.
    fn peers(&self) -> usize {
        let asked = Addressed::parse(
            "feed=zerodha&exchange=NSE&segment=CASH&symbol=SUBJECT&timeframe=1min&month=2025-05",
        )
        .expect("a well-formed address");
        peer_calendar(&self.site, &asked).from.len()
    }

    /// Where `/bars` would read `symbol` from, with no exchange or segment given.
    ///
    /// `None` means exactly "no census holds it and every census was read". An
    /// ambiguity or an unreadable census is a different answer, and folding it
    /// into `None` let a `== None` assertion here pass on a refusal it never
    /// meant. D-0695.
    fn locate(&self, symbol: &str) -> Option<(String, String)> {
        let located = locate_series(&self.site, &format!("symbol={symbol}"), symbol);
        assert!(
            located.is_ok()
                || matches!(located, Err(Unlocated::NotHeld(ref unreadable)) if unreadable.is_empty()),
            "{symbol} was refused, not merely unheld: {located:?}"
        );
        located.ok()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// Every census-backed GET route shares one read of an unchanged manifest, and
/// each of them is the one that re-reads it after a rewrite.
///
/// # Why each route takes a turn going first
///
/// A route that still called `read_all` directly would show up as an extra
/// read on its WARM call; a route that used a cache nobody invalidated would
/// show up as a MISSING read on its first call after a rewrite. Rotating which
/// route meets the rewrite first puts every one of them on both sides of that
/// line, so none can pass on the others' behaviour.
#[tokio::test]
#[expect(
    clippy::too_many_lines,
    reason = "one store walked through five publications, each route meeting a \
              rewrite first in turn; split, a route could pass on another's read"
)]
async fn census_backed_get_routes_read_an_unchanged_manifest_once_and_a_rewrite_again() {
    let fixture = Fixture::new("census-request-reads");
    fixture.publish(&[(Segment::Cash, "ADANIENT")], 0);
    let base = fixture.reads();

    // THE FIRST REQUEST READS ONCE -- the cache was never filled, and the
    // startup read saw no manifest. Exactly once, not once per route.
    assert_eq!(
        fixture.locate("ADANIENT"),
        Some(("NSE".to_owned(), "CASH".to_owned())),
        "the census names ADANIENT's own exchange and segment"
    );
    assert_eq!(
        fixture.reads(),
        base + 1,
        "a cold census reads the manifest once"
    );

    // WARM: every route answers from the shared census and reads nothing.
    assert_eq!(
        fixture.locate("RELIANCE"),
        None,
        "nothing holds RELIANCE yet"
    );
    assert_eq!(
        fixture.reads(),
        base + 1,
        "/bars re-read an unchanged manifest"
    );
    assert_eq!(
        fixture.calendar("feed=dhan").await,
        axum::http::StatusCode::OK
    );
    assert_eq!(
        fixture.reads(),
        base + 1,
        "/calendar.json (exchange) re-read an unchanged manifest"
    );
    assert_eq!(
        fixture.calendar("feed=dhan&symbol=ADANIENT").await,
        axum::http::StatusCode::OK,
        "one identity is an answer, not a conflict"
    );
    assert_eq!(
        fixture.reads(),
        base + 1,
        "/calendar.json (symbol) re-read an unchanged manifest"
    );
    assert_eq!(fixture.peers(), 0, "no bars, so no peer can vote");
    assert_eq!(
        fixture.reads(),
        base + 1,
        "/gaps.json's peer vote re-read an unchanged manifest"
    );

    // REWRITE, AND /bars MEETS IT FIRST. The new image adds RELIANCE and files
    // ADANIENT under a second segment; both are visible only if it is re-read.
    fixture.publish(
        &[
            (Segment::Cash, "ADANIENT"),
            (Segment::Index, "ADANIENT"),
            (Segment::Cash, "RELIANCE"),
        ],
        1,
    );
    assert_eq!(
        fixture.locate("RELIANCE"),
        Some(("NSE".to_owned(), "CASH".to_owned())),
        "a pull that added RELIANCE is visible on the next /bars request"
    );
    assert_eq!(
        fixture.reads(),
        base + 2,
        "/bars re-reads a rewritten manifest once"
    );
    assert_eq!(
        fixture.calendar("feed=dhan&symbol=ADANIENT").await,
        axum::http::StatusCode::CONFLICT,
        "the second identity the rewrite filed is seen by /calendar.json too"
    );
    assert_eq!(fixture.reads(), base + 2, "and it shares that one re-read");

    // REWRITE, AND /calendar.json's EXCHANGE BRANCH MEETS IT FIRST.
    fixture.publish(&[(Segment::Cash, "ADANIENT")], 2);
    assert_eq!(
        fixture.calendar("feed=dhan").await,
        axum::http::StatusCode::OK
    );
    assert_eq!(
        fixture.reads(),
        base + 3,
        "/calendar.json (exchange) re-reads a rewritten manifest once"
    );
    assert_eq!(
        fixture.calendar("feed=dhan").await,
        axum::http::StatusCode::OK
    );
    assert_eq!(
        fixture.reads(),
        base + 3,
        "and not again while it is unchanged"
    );

    // REWRITE, AND /calendar.json's SYMBOL BRANCH MEETS IT FIRST. The rewrite
    // put ADANIENT back under one identity, so the conflict clears.
    assert_eq!(
        fixture.calendar("feed=dhan&symbol=ADANIENT").await,
        axum::http::StatusCode::OK,
        "the rewrite that removed the second identity is seen as well"
    );
    fixture.publish(
        &[(Segment::Cash, "ADANIENT"), (Segment::Index, "ADANIENT")],
        3,
    );
    assert_eq!(
        fixture.calendar("feed=dhan&symbol=ADANIENT").await,
        axum::http::StatusCode::CONFLICT,
        "/calendar.json (symbol) sees a rewrite on its first request after it"
    );
    assert_eq!(
        fixture.reads(),
        base + 4,
        "/calendar.json (symbol) re-reads a rewritten manifest once"
    );

    // REWRITE, AND /gaps.json's PEER VOTE MEETS IT FIRST.
    fixture.publish(&[(Segment::Cash, "ADANIENT")], 4);
    assert_eq!(fixture.peers(), 0, "still no bars, so still no vote");
    assert_eq!(
        fixture.reads(),
        base + 5,
        "/gaps.json's peer vote re-reads a rewritten manifest once"
    );
    assert_eq!(fixture.peers(), 0);
    assert_eq!(
        fixture.reads(),
        base + 5,
        "and not again while it is unchanged"
    );
    assert_eq!(
        fixture.locate("RELIANCE"),
        None,
        "the rewrite that dropped RELIANCE reaches /bars through the same read"
    );
    assert_eq!(fixture.reads(), base + 5);
}

/// THE STAMP IS AN EQUALITY KEY, NOT AN ORDERING, AND ABSENCE IS A STAMP TOO.
///
/// A restore that brings back an older image with its older modified time moves
/// the stamp BACKWARDS; a cache that only re-read on a newer stamp would serve
/// the newer image for ever. A deleted manifest moves the stamp from a time to
/// `None`, and the next request must see an absent census rather than the last
/// one it read.
#[tokio::test]
async fn a_stamp_moved_backwards_or_a_deleted_manifest_is_read_again() {
    let fixture = Fixture::new("census-request-backwards");
    fixture.publish(
        &[(Segment::Cash, "ADANIENT"), (Segment::Cash, "RELIANCE")],
        1,
    );
    assert_eq!(
        fixture.locate("RELIANCE"),
        Some(("NSE".to_owned(), "CASH".to_owned()))
    );
    let base = fixture.reads();

    // AN OLDER IMAGE, AT AN OLDER STAMP.
    fixture.publish(&[(Segment::Cash, "ADANIENT")], 0);
    assert_eq!(
        fixture.locate("RELIANCE"),
        None,
        "the older image is served"
    );
    assert_eq!(
        fixture.reads(),
        base + 1,
        "read once, because the key moved"
    );

    // DELETED: `Some(stamp)` to `None`.
    fs::remove_file(fixture.manifest()).expect("delete the manifest");
    assert_eq!(fixture.locate("ADANIENT"), None, "nothing is held any more");
    assert_eq!(fixture.dhan_state(), "absent", "{:?}", fixture.states());
    let (status, body) = fixture.calendar_answer("feed=dhan&symbol=ADANIENT").await;
    assert_eq!(status, axum::http::StatusCode::OK, "{body}");
    assert!(body.contains(r#""sessions":0"#), "{body}");
}

/// THE DOCUMENTED LIMIT, PINNED: a rewrite that keeps the cached stamp is not
/// seen until the stamp moves.
///
/// `docs/06-limits.md`'s D-0686 section states it and D-0686 rejected widening
/// the key to close it. This proves both halves of that sentence: the stale
/// answer is served with no manifest read while the stamp is unchanged, and the
/// first request after the stamp moves reads once and answers from the new
/// image. A change that closes the gap fails the first half and must update
/// that section.
#[tokio::test]
async fn a_rewrite_that_keeps_the_stamp_is_served_stale_until_the_stamp_moves() {
    let fixture = Fixture::new("census-request-same-stamp");
    fixture.publish(&[(Segment::Cash, "ADANIENT")], 0);
    assert_eq!(fixture.locate("RELIANCE"), None);
    let base = fixture.reads();

    fixture.publish(
        &[
            (Segment::Cash, "ADANIENT"),
            (Segment::Index, "ADANIENT"),
            (Segment::Cash, "RELIANCE"),
        ],
        0,
    );
    assert_eq!(
        fixture.locate("RELIANCE"),
        None,
        "stale: the stamp did not move"
    );
    assert_eq!(
        fixture.calendar("feed=dhan&symbol=ADANIENT").await,
        axum::http::StatusCode::OK,
        "stale: the second identity is not seen"
    );
    assert_eq!(fixture.reads(), base, "and nothing was read to find out");

    fixture.stamp(Vendor::Dhan, 1);
    assert_eq!(
        fixture.locate("RELIANCE"),
        Some(("NSE".to_owned(), "CASH".to_owned()))
    );
    assert_eq!(
        fixture.calendar("feed=dhan&symbol=ADANIENT").await,
        axum::http::StatusCode::CONFLICT
    );
    assert_eq!(fixture.reads(), base + 1);
}

/// A STORE ROOT THAT GOES AWAY IS AN OUTAGE, NOT AN EMPTY STORE -- and one that
/// comes back empty is empty, not an outage.
///
/// `census::read_all` tells the two apart. The cache key did not: every
/// manifest is missing in both, so both stamped `[None; 5]`, and a census
/// cached in either state was served in the other until a manifest appeared.
/// `/store.json`'s 503 for an unreadable census could then report the older of
/// the two.
#[test]
fn a_store_root_that_goes_away_or_comes_back_is_seen_on_the_next_request() {
    let fixture = Fixture::new("census-request-root-away");
    assert!(fixture.states().iter().all(|state| *state == "absent"));
    fs::remove_dir_all(&fixture.root).expect("detach the store root");
    assert_eq!(fixture.fresh_states(), vec!["unreadable"; 5]);
    assert_eq!(
        fixture.states(),
        vec!["unreadable"; 5],
        "the outage is seen"
    );

    fs::create_dir_all(&fixture.root).expect("reattach it, empty");
    assert_eq!(fixture.fresh_states(), vec!["absent"; 5]);
    assert_eq!(fixture.states(), vec!["absent"; 5], "the recovery is seen");

    // A ROOT THAT IS A FILE is unreadable too, as `read_all` says.
    fs::remove_dir_all(&fixture.root).expect("detach again");
    fs::write(&fixture.root, b"not a directory").expect("a file where the root was");
    assert_eq!(fixture.states(), vec!["unreadable"; 5]);
    fs::remove_file(&fixture.root).expect("remove the file");
}

/// AN UNREADABLE CENSUS IS NAMED, NEVER ANSWERED AS AN EMPTY ONE.
///
/// `/calendar.json` answered `200 {"sessions":0,"days":[]}` for a feed whose
/// manifest would not decode, which the ingest page reads as "the store holds no
/// bars for this feed" -- a claim about the store made from a fact about its
/// counter, the fallback `CLAUDE.md` §4 bans and D-0124 already refused on
/// `/store.json`. `/bars` refused the same request as "no feed in this store
/// holds a spot series under that name", which is not the reason. Both now
/// name the unreadable census. A manifest past the reader's size bound is
/// refused before a byte of it is read, and is named the same way.
#[tokio::test]
async fn an_unreadable_census_is_named_by_calendar_and_bars() {
    let fixture = Fixture::new("census-request-unreadable");
    fixture.publish(&[(Segment::Index, "NIFTY")], 0);
    fixture.bar(Vendor::Dhan, Segment::Index, "NIFTY", 2);
    let (status, healthy) = fixture.calendar_answer("feed=dhan").await;
    assert_eq!(status, axum::http::StatusCode::OK);
    assert!(healthy.contains(r#""sessions":1"#), "{healthy}");

    fixture.publish_bytes(Vendor::Dhan, &[0xFF; 16], 1);
    assert_eq!(fixture.dhan_state(), "unreadable");
    for query in ["feed=dhan", "feed=dhan&symbol=NIFTY"] {
        let (status, body) = fixture.calendar_answer(query).await;
        assert_eq!(
            status,
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            "{query}: {body}"
        );
        assert!(body.contains(r#""census":"unreadable""#), "{query}: {body}");
        assert!(body.contains("dhan.man"), "names the file: {body}");
        assert!(!body.contains(r#""sessions""#), "no calendar: {body}");
    }
    let (status, page) = fixture.bars("symbol=NIFTY&vendor=dhan&month=2025-05");
    assert_eq!(status, axum::http::StatusCode::BAD_REQUEST);
    assert!(page.contains("UNREADABLE"), "{page}");
    assert!(
        !page.contains("no feed in this store holds"),
        "the stated reason must be the true one: {page}"
    );

    // AN ABSENT FEED IS STILL THE ORDINARY STATE.
    let (status, absent) = fixture.calendar_answer("feed=zerodha").await;
    assert_eq!(status, axum::http::StatusCode::OK, "{absent}");

    // PAST THE SIZE BOUND: refused by size, before any read.
    let reads = fixture.reads();
    fs::File::options()
        .write(true)
        .open(fixture.manifest())
        .expect("the manifest")
        .set_len(census::MAX_MANIFEST_BYTES + 1)
        .expect("a sparse oversized manifest");
    fixture.stamp(Vendor::Dhan, 2);
    assert_eq!(fixture.dhan_state(), "unreadable");
    assert_eq!(fixture.reads(), reads, "refused by its size, never read");
    let (status, body) = fixture.calendar_answer("feed=dhan").await;
    assert_eq!(
        status,
        axum::http::StatusCode::SERVICE_UNAVAILABLE,
        "{body}"
    );
}

/// AN UNREADABLE PEER CENSUS IS NAMED IN THE VOTE.
///
/// The `/gaps.json` peer vote skipped an unreadable census exactly as it skips a
/// feed that holds nothing, so a damaged counter fell back to the typed table
/// with `voted_by: []` -- the same answer as a store with no peers at all.
/// The vote now lists the feeds whose census could not be read.
#[test]
fn an_unreadable_peer_census_is_named_in_the_vote() {
    let fixture = Fixture::new("census-request-unreadable-peer");
    let asked = Addressed::parse(
        "feed=zerodha&exchange=NSE&segment=INDEX&symbol=NIFTY&timeframe=1min&month=2025-05",
    )
    .expect("a well-formed address");
    assert!(peer_calendar(&fixture.site, &asked).unreadable.is_empty());
    fixture.publish_bytes(Vendor::Dhan, &[0xFF; 16], 0);
    let vote = peer_calendar(&fixture.site, &asked);
    assert!(vote.calendar.is_none());
    assert!(vote.from.is_empty());
    assert_eq!(vote.unreadable, vec!["dhan".to_owned()]);
}

/// A POISONED CACHE LOCK IS READ THROUGH, AND A REWRITE STILL REACHES IT.
///
/// A panic while holding `site.census` means another request died; the census
/// is still a census. A hit through the poison reads nothing, and a rewrite
/// behind it is still read once and served.
#[test]
fn a_poisoned_census_lock_is_read_through_and_still_refreshed() {
    let fixture = Fixture::new("census-request-poisoned");
    fixture.publish(&[(Segment::Cash, "ADANIENT")], 0);
    assert!(fixture.locate("ADANIENT").is_some());
    std::thread::scope(|scope| {
        #[expect(
            clippy::panic,
            reason = "the poison is the fixture: a holder must die with the lock held"
        )]
        let died = scope
            .spawn(|| {
                let _held = fixture.site.census.lock();
                std::panic::panic_any("a request died holding the census lock");
            })
            .join();
        assert!(died.is_err(), "the holder panicked");
    });
    assert!(fixture.site.census.is_poisoned());
    let reads = fixture.reads();
    assert_eq!(
        fixture.locate("ADANIENT"),
        Some(("NSE".to_owned(), "CASH".to_owned()))
    );
    assert_eq!(
        fixture.reads(),
        reads,
        "a hit through the poison reads nothing"
    );
    fixture.publish(&[(Segment::Cash, "RELIANCE")], 1);
    assert!(fixture.locate("RELIANCE").is_some());
    assert_eq!(fixture.locate("ADANIENT"), None);
    assert_eq!(fixture.reads(), reads + 1);
}

/// THE KEY IS THE WHOLE SET, SO ANOTHER FEED'S REWRITE RE-READS THIS ONE.
///
/// The cost `docs/06-limits.md`'s D-0686 section states: the first request
/// after ANY vendor's stamp moves pays a whole `read_all`, the unchanged
/// manifests included. Pinned so a change to that cost is a visible one.
#[test]
fn another_feeds_rewrite_rereads_every_manifest_once() {
    let fixture = Fixture::new("census-request-other-feed");
    fixture.publish(&[(Segment::Cash, "ADANIENT")], 0);
    assert!(fixture.locate("ADANIENT").is_some());
    let reads = fixture.reads();
    assert!(fixture.locate("ADANIENT").is_some());
    assert_eq!(fixture.reads(), reads, "warm");
    fixture.publish_for(Vendor::Zerodha, &[(Segment::Index, "NIFTY")], 0);
    assert!(fixture.locate("NIFTY").is_some());
    assert_eq!(
        fixture.reads(),
        reads + 1,
        "Dhan's unchanged manifest re-read"
    );
    assert!(fixture.locate("NIFTY").is_some());
    assert_eq!(fixture.reads(), reads + 1, "and once only");
}

/// COLD, WARM AND COLD AGAIN ANSWER THE SAME BYTES. `CLAUDE.md` §3 rule 5.
///
/// "Cold" empties BOTH caches the route reads through: the census and the
/// per-series calendars. Emptying the census alone left every calendar cached,
/// so the second cold request was a calendar hit and could not tell a
/// derivation that differs on a miss from one that does not. D-0695.
#[tokio::test]
async fn cold_warm_and_cold_again_calendars_are_byte_identical() {
    let fixture = Fixture::new("census-request-idempotent");
    fixture.bar(Vendor::Dhan, Segment::Index, "NIFTY", 2);
    fixture.bar(Vendor::Dhan, Segment::Cash, "ADANIENT", 5);
    fixture.publish(&[(Segment::Index, "NIFTY"), (Segment::Cash, "ADANIENT")], 0);
    let chill = || {
        *fixture
            .site
            .census
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = None;
        fixture
            .site
            .calendars
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clear();
    };
    for query in [
        "feed=dhan",
        "feed=dhan&symbol=ADANIENT",
        "feed=dhan&symbol=NIFTY",
    ] {
        chill();
        let cold = fixture.calendar_answer(query).await;
        let warm = fixture.calendar_answer(query).await;
        chill();
        let again = fixture.calendar_answer(query).await;
        assert_eq!(cold.0, axum::http::StatusCode::OK, "{query}: {}", cold.1);
        assert!(!cold.1.contains(r#""sessions":0"#), "{query}: {}", cold.1);
        assert_eq!(cold, warm, "{query}");
        assert_eq!(cold, again, "{query}");
    }
}

/// CONCURRENT INSTALLS AND READERS NEVER SEE A TORN CENSUS, AND CONVERGE.
///
/// Each generation installs a manifest holding exactly one name, by rename, at
/// its own stamp, while four readers call `census_now` in a loop. A census
/// assembled from two images would hold two names; one read of a half-written
/// file would be unreadable. Neither may be observed, and once the writer
/// stops the cache must equal a fresh read of the disk.
#[test]
fn concurrent_installs_and_readers_never_see_a_torn_census() {
    let fixture = Fixture::new("census-request-concurrent");
    let names = ["RELIANCE", "TCS", "INFY", "SBIN", "ITC"];
    fixture.publish(&[(Segment::Cash, "HDFCBANK")], 0);
    let stop = std::sync::atomic::AtomicBool::new(false);
    let torn = std::sync::atomic::AtomicUsize::new(0);
    let looked = std::sync::atomic::AtomicUsize::new(0);
    std::thread::scope(|scope| {
        for _ in 0..4 {
            scope.spawn(|| {
                while !stop.load(std::sync::atomic::Ordering::Acquire) {
                    let (censuses, entries) = census_now(&fixture.site);
                    let dhan = censuses
                        .iter()
                        .find(|census| census.vendor == Vendor::Dhan)
                        .map(|census| census.state.name());
                    if entries.len() != 1 || dhan != Some("held") {
                        torn.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    }
                    looked.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
            });
        }
        scope.spawn(|| {
            for generation in 1..=100_u64 {
                let at = usize::try_from(generation).expect("small") % names.len();
                let name = names.get(at).copied().expect("in range");
                fixture.publish(&[(Segment::Cash, name)], generation);
            }
            stop.store(true, std::sync::atomic::Ordering::Release);
        });
    });
    assert_eq!(torn.load(std::sync::atomic::Ordering::Relaxed), 0);
    assert!(looked.load(std::sync::atomic::Ordering::Relaxed) > 0);
    let (_, cached) = census_now(&fixture.site);
    let disk = census::held_entries(&census::read_all(&fixture.site.store_root));
    assert_eq!(*cached, disk, "the cache converged on the disk");
    assert_eq!(cached.len(), 1);
}

/// `/bars` READS THE ASKED FEED'S OWN IDENTITY, AND REFUSES ONE HELD TWICE.
///
/// `bars_html` opens `?vendor=`'s file, so the exchange and segment must be the
/// ones THAT feed filed the name under. The walk took the first vendor in
/// `Vendor::ALL` order that held the name: with Groww holding `ADANIENT` as an
/// INDEX series and Dhan as CASH, `/bars?symbol=ADANIENT&vendor=dhan` probed
/// Dhan's INDEX path and answered 404 for bars sitting one directory over. And
/// a feed holding one name under two identities was resolved silently to one
/// of them, where `/calendar.json` refuses the same census as ambiguous.
#[tokio::test]
async fn bars_takes_the_asked_feeds_identity_and_refuses_one_held_twice() {
    let fixture = Fixture::new("census-request-asked-feed");
    fixture.publish_for(Vendor::Groww, &[(Segment::Index, "ADANIENT")], 0);
    fixture.publish_for(Vendor::Dhan, &[(Segment::Cash, "ADANIENT")], 0);
    fixture.bar(Vendor::Dhan, Segment::Cash, "ADANIENT", 2);
    let (status, page) = fixture.bars("symbol=ADANIENT&vendor=dhan&month=2025-05&timeframe=1day");
    assert_eq!(status, axum::http::StatusCode::OK, "{page}");
    assert!(page.contains("CASH"), "{page}");
    // Groww's own request still resolves Groww's identity.
    let (_, groww) = fixture.bars("symbol=ADANIENT&vendor=groww&month=2025-05&timeframe=1day");
    assert!(groww.contains("INDEX/ADANIENT"), "{groww}");

    fixture.publish_for(
        Vendor::Dhan,
        &[(Segment::Cash, "NIFTY"), (Segment::Index, "NIFTY")],
        1,
    );
    let (status, page) = fixture.bars("symbol=NIFTY&vendor=dhan&month=2025-05&timeframe=1day");
    assert_eq!(status, axum::http::StatusCode::BAD_REQUEST, "{page}");
    assert!(page.contains("ambiguous"), "{page}");
    assert!(
        page.contains("NSE/CASH") && page.contains("NSE/INDEX"),
        "{page}"
    );
    assert_eq!(
        fixture.calendar("feed=dhan&symbol=NIFTY").await,
        axum::http::StatusCode::CONFLICT,
        "the two routes agree"
    );
    // A HALF THE CALLER GAVE IS A CHOICE ALREADY MADE: with `?segment=CASH`
    // the census supplies only the exchange, and both identities agree on it.
    let (status, page) = fixture.bars("symbol=NIFTY&vendor=dhan&month=2025-05&segment=CASH");
    assert_ne!(status, axum::http::StatusCode::BAD_REQUEST, "{page}");
    assert!(page.contains("NSE/CASH/NIFTY"), "{page}");
    // AN EXPLICIT PAIR STILL WINS OUTRIGHT.
    let (status, _) =
        fixture.bars("symbol=NIFTY&vendor=dhan&month=2025-05&exchange=NSE&segment=CASH");
    assert_ne!(status, axum::http::StatusCode::BAD_REQUEST);
}

/// A NAME IS FOLDED TO ITS STORED CASE, AS `Symbol::new` FOLDS IT.
///
/// Every stored name went through `Symbol::new`, which upper-cases `a-z`, so
/// `adanient` and `ADANIENT` name one series. Both routes compared the raw
/// parameter byte for byte: `/bars?symbol=adanient` refused as held by no feed
/// and `/calendar.json?symbol=adanient` answered zero sessions for a name with
/// one.
#[tokio::test]
async fn a_symbol_is_folded_to_its_stored_case_on_bars_and_calendar() {
    let fixture = Fixture::new("census-request-case");
    fixture.bar(Vendor::Dhan, Segment::Cash, "ADANIENT", 2);
    fixture.publish(&[(Segment::Cash, "ADANIENT")], 0);
    let upper = fixture.calendar_answer("feed=dhan&symbol=ADANIENT").await;
    assert!(upper.1.contains(r#""sessions":1"#), "{}", upper.1);
    for raw in ["adanient", "AdAnIeNt"] {
        assert_eq!(
            fixture
                .calendar_answer(&format!("feed=dhan&symbol={raw}"))
                .await,
            upper,
            "{raw}"
        );
        let (status, page) = fixture.bars(&format!(
            "symbol={raw}&vendor=dhan&month=2025-05&timeframe=1day"
        ));
        assert_eq!(status, axum::http::StatusCode::OK, "{raw}: {page}");
    }
}

/// THE QUERY'S EDGES, PINNED: an empty name, the feed's spelling, a half pair
/// and a repeated name.
///
/// * An empty `symbol` is the exchange branch on `/calendar.json` and a
///   refusal on `/bars`, which has nothing to read.
/// * `feed` is matched without regard to case, a padded one is refused, and
///   an empty one is Dhan -- `ingest::parse_vendor`'s documented default.
/// * A half pair keeps the asked half and takes the other from the census.
/// * A repeated parameter takes its first value, as `param` documents.
#[tokio::test]
async fn query_edges_on_bars_and_calendar_are_pinned() {
    let fixture = Fixture::new("census-request-edges");
    fixture.bar(Vendor::Dhan, Segment::Cash, "ADANIENT", 2);
    fixture.publish(&[(Segment::Cash, "ADANIENT")], 0);

    let (status, exchange) = fixture.calendar_answer("feed=dhan&symbol=").await;
    assert_eq!(status, axum::http::StatusCode::OK);
    assert!(
        exchange.contains(r#""derivedFrom":["ADANIENT"]"#),
        "{exchange}"
    );
    let (status, page) = fixture.bars("symbol=&vendor=dhan&month=2025-05");
    assert_eq!(status, axum::http::StatusCode::BAD_REQUEST, "{page}");

    let dhan = fixture.calendar_answer("feed=dhan").await;
    for spelled in ["DHAN", "Dhan", ""] {
        assert_eq!(
            fixture.calendar_answer(&format!("feed={spelled}")).await,
            dhan,
            "{spelled:?}"
        );
    }
    for padded in ["%20dhan", "dhan%20"] {
        let (status, body) = fixture.calendar_answer(&format!("feed={padded}")).await;
        assert_eq!(
            status,
            axum::http::StatusCode::BAD_REQUEST,
            "{padded}: {body}"
        );
    }

    let (_, bse) = fixture.bars("symbol=ADANIENT&vendor=dhan&month=2025-05&exchange=BSE");
    assert!(bse.contains("BSE/CASH/ADANIENT"), "{bse}");
    let (_, index) = fixture.bars("symbol=ADANIENT&vendor=dhan&month=2025-05&segment=INDEX");
    assert!(index.contains("NSE/INDEX/ADANIENT"), "{index}");

    let (status, page) =
        fixture.bars("symbol=ADANIENT&symbol=X&vendor=dhan&month=2025-05&timeframe=1day");
    assert_eq!(
        status,
        axum::http::StatusCode::OK,
        "the first value wins: {page}"
    );
}

/// THE STAMPS ARE TAKEN BEFORE THE READ: A MANIFEST INSTALLED BETWEEN THE READ
/// AND THE CACHE WRITE IS SEEN ON THE NEXT REQUEST. D-0695.
///
/// `concurrent_installs_and_readers_never_see_a_torn_census` asks for this by
/// racing, and a race lands a rename in that window only sometimes: a review
/// took the stamps after the read and that test failed 7 of 20 runs. Here the
/// read itself installs the next image once it has read, so the window is hit
/// on every run.
#[test]
fn a_manifest_installed_after_the_read_is_seen_on_the_next_request() {
    let fixture = Fixture::new("census-request-stamp-before-read");
    fixture.publish(&[(Segment::Cash, "RELIANCE")], 0);
    let (_, entries) = census_now_reading(&fixture.site, |root| {
        let read = census::read_all(root);
        fixture.publish(&[(Segment::Cash, "ITC")], 1);
        read
    });
    let names: Vec<&str> = entries
        .iter()
        .map(|(series, _)| series.symbol.as_str())
        .collect();
    assert_eq!(
        names,
        ["RELIANCE"],
        "the read saw the image before the install"
    );
    assert_eq!(
        fixture.locate("ITC"),
        Some(("NSE".to_owned(), "CASH".to_owned())),
        "the next request reads the image installed after the read"
    );
    assert_eq!(fixture.locate("RELIANCE"), None);
}

/// A MANIFEST DIRECTORY THAT IS A FILE IS REFUSED, NOT MISSING -- and one put
/// back is missing again, each on the next request. D-0695.
///
/// The key folded every `stat` error into the stamp a missing manifest has, so
/// `manifest/` as a regular file (`ENOTDIR` for every manifest) keyed exactly
/// as an empty store: a census cached "absent" was served "absent" while a
/// fresh read said "unreadable", and `/calendar.json` answered 200 with no
/// sessions -- the empty answer AF-24 refuses, reached through the cache.
#[tokio::test]
async fn a_manifest_directory_that_is_a_file_is_not_served_as_absent() {
    let fixture = Fixture::new("census-request-manifest-file");
    assert_eq!(fixture.states(), vec!["absent"; 5]);
    let dir = fixture.root.join("manifest");
    fs::write(&dir, b"not a directory").expect("a file where the manifests go");
    assert_eq!(fixture.fresh_states(), vec!["unreadable"; 5]);
    assert_eq!(fixture.states(), vec!["unreadable"; 5], "the fault is seen");
    let (status, body) = fixture.calendar_answer("feed=dhan").await;
    assert_eq!(
        status,
        axum::http::StatusCode::SERVICE_UNAVAILABLE,
        "{body}"
    );
    assert!(body.contains(r#""census":"unreadable""#), "{body}");

    fs::remove_file(&dir).expect("take the file away");
    assert_eq!(fixture.fresh_states(), vec!["absent"; 5]);
    assert_eq!(fixture.states(), vec!["absent"; 5], "and so is the repair");
    let (status, body) = fixture.calendar_answer("feed=dhan").await;
    assert_eq!(status, axum::http::StatusCode::OK, "{body}");
}

/// Makes a directory unsearchable, and searchable again when dropped, so a
/// failed assertion still leaves a fixture its own `Drop` can remove.
#[cfg(unix)]
struct Unsearchable<'a>(&'a Path);

#[cfg(unix)]
impl<'a> Unsearchable<'a> {
    fn new(dir: &'a Path) -> Self {
        use std::os::unix::fs::PermissionsExt as _;
        fs::set_permissions(dir, fs::Permissions::from_mode(0o000))
            .expect("a directory with no permissions");
        Self(dir)
    }
}

#[cfg(unix)]
impl Drop for Unsearchable<'_> {
    fn drop(&mut self) {
        use std::os::unix::fs::PermissionsExt as _;
        let _restored = fs::set_permissions(self.0, fs::Permissions::from_mode(0o755));
    }
}

/// A MANIFEST DIRECTORY THIS PROCESS MAY NOT SEARCH IS REFUSED, NOT MISSING,
/// warm or cold, and its repair is seen on the next request. D-0695.
///
/// `EACCES` from every manifest `stat` keyed as an empty store, as `ENOTDIR`
/// did. Warm on "absent", the refusal was served "absent" and `/calendar.json`
/// answered 200. Cold on the refusal, "unreadable" was cached and outlived the
/// restored permission, answering 503 over an empty store. Both directions are
/// driven here. Like `folder`'s unreadable-folder test, this needs a process
/// the permission binds, which a root process is not.
#[cfg(unix)]
#[tokio::test]
async fn a_manifest_directory_this_process_may_not_search_is_not_served_as_absent() {
    let fixture = Fixture::new("census-request-manifest-unsearchable");
    let dir = fixture.root.join("manifest");
    fs::create_dir_all(&dir).expect("an empty manifest directory");
    assert_eq!(
        fixture.states(),
        vec!["absent"; 5],
        "warm on an empty store"
    );
    {
        let _denied = Unsearchable::new(&dir);
        assert_eq!(
            fixture.fresh_states(),
            vec!["unreadable"; 5],
            "the permission binds this process"
        );
        assert_eq!(fixture.states(), vec!["unreadable"; 5], "warm: seen");
        let (status, body) = fixture.calendar_answer("feed=dhan").await;
        assert_eq!(
            status,
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            "{body}"
        );
    }
    assert_eq!(fixture.states(), vec!["absent"; 5], "the repair is seen");

    // COLD ON THE REFUSAL, then repaired.
    *fixture
        .site
        .census
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = None;
    {
        let _denied = Unsearchable::new(&dir);
        assert_eq!(fixture.states(), vec!["unreadable"; 5], "cold: seen");
    }
    assert_eq!(fixture.fresh_states(), vec!["absent"; 5]);
    assert_eq!(
        fixture.states(),
        vec!["absent"; 5],
        "the cached refusal does not outlive the repair"
    );
    let (status, body) = fixture.calendar_answer("feed=dhan").await;
    assert_eq!(status, axum::http::StatusCode::OK, "{body}");
}

/// A ROOT THAT IS A FILE AND A MANIFEST DIRECTORY THAT IS A FILE ARE TWO KEYS,
/// and so are a missing root and a root that is a file. D-0695.
///
/// Each pair is one census STATE (unreadable) with two different notes, and
/// `read_all` words each its own way. Keyed alike, the note cached first was
/// served for the other: a root replaced by a file kept saying the root was
/// "unavailable: No such file or directory".
#[test]
fn a_root_and_a_manifest_directory_that_are_files_keep_their_own_notes() {
    let fixture = Fixture::new("census-request-root-kinds");
    let notes = || -> Vec<String> {
        census_now(&fixture.site)
            .0
            .iter()
            .map(census::VendorCensus::note)
            .collect()
    };
    let fresh = || -> Vec<String> {
        census::read_all(&fixture.site.store_root)
            .iter()
            .map(census::VendorCensus::note)
            .collect()
    };
    fs::remove_dir_all(&fixture.root).expect("detach the store root");
    assert_eq!(notes(), fresh(), "missing root");
    fs::write(&fixture.root, b"not a directory").expect("a file where the root was");
    assert_eq!(notes(), fresh(), "a root that is a file");
    fs::remove_file(&fixture.root).expect("take the file away");
    fs::create_dir_all(&fixture.root).expect("the root again");
    fs::write(fixture.root.join("manifest"), b"not a directory").expect("a manifest file");
    assert_eq!(notes(), fresh(), "a manifest directory that is a file");
}

/// `/bars` REFUSES WHEN THE ASKED FEED'S OWN CENSUS IS UNREADABLE, rather than
/// stepping over it to another feed's identity. D-0695.
///
/// With Dhan's manifest damaged and Groww holding `ADANIENT` as INDEX,
/// `/bars?symbol=ADANIENT&vendor=dhan` stepped over Dhan's census, took Groww's
/// identity and opened Dhan's file at `NSE/INDEX/ADANIENT` -- the other-feed
/// guess `locate_series` exists to refuse -- and dropped the note, because
/// notes surfaced only when nothing was found. The page said the month did not
/// exist with no word that Dhan's counter could not be read. A half the caller
/// gave still leaves the other half to that census, so it is refused too.
#[tokio::test]
async fn bars_refuses_when_the_asked_feeds_own_census_is_unreadable() {
    let fixture = Fixture::new("census-request-asked-unreadable");
    fixture.publish_for(Vendor::Groww, &[(Segment::Index, "ADANIENT")], 0);
    fixture.publish_bytes(Vendor::Dhan, &[0xFF; 16], 0);
    for query in [
        "symbol=ADANIENT&vendor=dhan&month=2025-05&timeframe=1day",
        "symbol=ADANIENT&month=2025-05&timeframe=1day",
        "symbol=ADANIENT&vendor=dhan&month=2025-05&segment=INDEX",
    ] {
        let (status, page) = fixture.bars(query);
        assert_eq!(
            status,
            axum::http::StatusCode::BAD_REQUEST,
            "{query}: {page}"
        );
        assert!(
            page.contains("own census could not be read"),
            "{query}: {page}"
        );
        assert!(page.contains("dhan.man"), "names the file: {query}: {page}");
        assert!(
            !page.contains("INDEX/ADANIENT"),
            "no path was opened at Groww's identity: {query}: {page}"
        );
    }
    // Groww's own request still resolves Groww's identity: its census is fine.
    let (_, groww) = fixture.bars("symbol=ADANIENT&vendor=groww&month=2025-05&timeframe=1day");
    assert!(groww.contains("INDEX/ADANIENT"), "{groww}");
    // An explicit pair consults no census and still wins outright.
    let (_, page) =
        fixture.bars("symbol=ADANIENT&vendor=dhan&month=2025-05&exchange=NSE&segment=CASH");
    assert!(page.contains("NSE/CASH/ADANIENT"), "{page}");
}

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
//!
//! Since D-0695 the key also carries each manifest's status-change time, which
//! cannot be set. The tests that need it to move wait for its tick through
//! `past_a_ctime_tick`; every other test reads the same with or without it.
//! And only a read those stamps could have made is kept under them. The tests
//! from `a_store_root_that_vanishes_during_the_read_is_not_cached` on pin which
//! reads are kept: a read faulted through `census_now_reading` in a way no
//! stamp records is read again on the next request, and a fault the stamps do
//! record is still kept. The next three move a bar directory or file aside
//! instead, and pin the same rule for the calendar cache behind the census: a
//! calendar derived without a bar file its census holds is refused to the
//! request that derived it, and derived again on the next. The rest pin the
//! calendar cache's key, through every caller that derives one: a month landed
//! after a census's stamps, whether after or inside its read, is not keyed
//! under them; a calendar is kept at all, under the asked feed's own modified
//! time and no other feed's; and a census row its stamp could not have read
//! keys nothing.
#![expect(
    clippy::expect_used,
    reason = "finite owned fixtures and exact response assertions"
)]

use super::*;
use brutex_core::instrument::{Exchange, Segment};
use brutex_core::symbol::Symbol;
use pull::manifest::{Entry, EntryKey, Manifest, manifest_path};
use std::fs;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use store::path::{Timeframe, YearMonth};

/// `minute` minutes after a fixed instant: every stamp here is SET, not waited
/// for.
fn at(minute: u64) -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000) + Duration::from_mins(minute)
}

/// 2025-05, the month every fixture census holds.
fn may() -> YearMonth {
    YearMonth::new(2025, 5).expect("fixture month")
}

/// 2025-06, the month a fixture adds when a test needs a second one.
fn june() -> YearMonth {
    YearMonth::new(2025, 6).expect("fixture month")
}

/// The micros stamp of 11:30 IST on `month`'s 2nd: where a fixture's daily bar
/// and its manifest entry sit.
fn on_the_2nd(month: YearMonth) -> i64 {
    let day = Day::new(month.year(), month.month(), 2).expect("fixture date");
    i64::from(day.days_from_epoch()) * 86_400_000_000 + 21_600_000_000
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
    /// each: 2025-05.
    fn image(vendor: Vendor, held: &[(Segment, &str)]) -> Vec<u8> {
        let may = [may()];
        let held: Vec<(Segment, &str, &[YearMonth])> = held
            .iter()
            .map(|(segment, symbol)| (*segment, *symbol, may.as_slice()))
            .collect();
        Self::image_of(vendor, &held)
    }

    /// A manifest image for `vendor` holding exactly `held`: each series at
    /// the daily rung, for each month given, one bar on the 2nd.
    fn image_of(vendor: Vendor, held: &[(Segment, &str, &[YearMonth])]) -> Vec<u8> {
        let mut manifest = Manifest::open(vendor, &[], &[]).expect("a genesis manifest");
        for (segment, symbol, months) in held {
            for month in *months {
                let ts = on_the_2nd(*month);
                manifest
                    .record(Entry {
                        key: EntryKey {
                            contract: None,
                            exchange: Exchange::Nse,
                            segment: *segment,
                            symbol: Symbol::new(symbol).expect("fixture symbol"),
                            timeframe: Timeframe::DAY_1,
                            month: *month,
                        },
                        rows: 1,
                        first_ts_micros: ts,
                        last_ts_micros: ts,
                    })
                    .expect("one held month");
            }
        }
        manifest.image()
    }

    /// [`Self::publish`] for any vendor's manifest.
    fn publish_for(&self, vendor: Vendor, held: &[(Segment, &str)], minute: u64) {
        self.publish_bytes(vendor, &Self::image(vendor, held), minute);
    }

    /// Publish Dhan's manifest holding NIFTY as INDEX for each of `months`,
    /// stamped `minute`: what a pull that lands a further month installs.
    fn publish_nifty(&self, months: &[YearMonth], minute: u64) {
        let image = Self::image_of(Vendor::Dhan, &[(Segment::Index, "NIFTY", months)]);
        self.publish_bytes(Vendor::Dhan, &image, minute);
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
        self.bar_in(vendor, segment, symbol, may(), date);
    }

    /// [`Self::bar`] in any month: one real daily bar on `month`'s `date`,
    /// and the path of the file it is in. The census is not touched.
    fn bar_in(
        &self,
        vendor: Vendor,
        segment: Segment,
        symbol: &str,
        month: YearMonth,
        date: u8,
    ) -> PathBuf {
        let day = Day::new(month.year(), month.month(), date).expect("fixture date");
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
        let file_path = path.to_path_buf(&self.root);
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
        file_path
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
    // THE BAR THE CENSUS NAMES IS ON DISK. Since D-0695 a calendar derived
    // without a file its census holds is refused, and what this test counts is
    // the census read each route takes, not that refusal. So Dhan's ADANIENT
    // also has a daily bar, and it votes on `/gaps.json`.
    fixture.bar(Vendor::Dhan, Segment::Cash, "ADANIENT", 2);
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
    assert_eq!(
        fixture.peers(),
        1,
        "Dhan's ADANIENT, whose bar is on disk, votes"
    );
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
    assert_eq!(fixture.peers(), 1, "still Dhan's ADANIENT alone");
    assert_eq!(
        fixture.reads(),
        base + 5,
        "/gaps.json's peer vote re-reads a rewritten manifest once"
    );
    assert_eq!(fixture.peers(), 1);
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

/// `stamps` with every existing manifest's status-change time cleared, and
/// every other term left as it was.
///
/// Written as an assignment through the variant's one named field, not as a
/// rebuilt `At`, so a term added to `ManifestStamp::At` later is KEPT here and
/// compared: the AF-23 tripwire exists to see exactly such a term.
fn without_status_change(mut stamps: CensusStamps) -> CensusStamps {
    for stamp in &mut stamps.manifests {
        if let ManifestStamp::At { changed, .. } = stamp {
            *changed = StatusChanged::default();
        }
    }
    stamps
}

/// THE DOCUMENTED LIMIT, PINNED: a rewrite that keeps the cached stamp is not
/// seen until the stamp moves.
///
/// `docs/06-limits.md`'s D-0686 section states it, D-0686 rejected widening
/// the key to close it, and D-0695 narrowed it to two changes inside one tick
/// of the filesystem's clock. This proves the three parts of that sentence, in
/// order:
///
/// 1. **The tripwire.** The key is taken before and after a rewrite at the
///    same modified time, and the two must be equal once each manifest's
///    status-change time is cleared. So the status-change time is the only
///    term of the key a rewrite moves, and a rewrite inside one tick of it
///    moves nothing. Widen the key with the length, the inode or any other term
///    a rewrite moves, and this fails, because that change closes the gap the
///    limits section states and must update it. Narrowing the key back to the
///    modified time alone passes here and fails
///    `a_rewrite_that_keeps_only_the_modified_time_is_read_again`, so between
///    them the two tests pin what the key contains, as far as a rewrite can
///    see it.
/// 2. **The stale answer.** While the key stays put, the older census is served
///    and nothing is read.
/// 3. **The recovery.** The first request after the key moves reads once and
///    answers from the new image.
///
/// # Why the stale state is planted, and why part 1 must come first
///
/// Every write moves the status-change time, and no call can set it, so a
/// rewrite that keeps the WHOLE stamp means two writes inside one tick. A test
/// cannot make that on demand on a filesystem that keeps nanoseconds. So this
/// rewrite keeps the modified time, and the cache is then given the state such
/// a rewrite leaves: the census read before the rewrite, under the stamp taken
/// after it. That planted key is what the production `manifest_stamps`
/// computes, so parts 2 and 3 alone hold under ANY key: a review measured this
/// test passing with the length, and with the inode, added to the key. Part 1
/// is what they lacked, and it is checked before anything is planted.
#[tokio::test]
async fn a_rewrite_that_keeps_the_stamp_is_served_stale_until_the_stamp_moves() {
    let fixture = Fixture::new("census-request-same-stamp");
    fixture.publish(&[(Segment::Cash, "ADANIENT")], 0);
    // The bar the census names, so the stale answer below is a calendar and
    // not D-0695's refusal of one derived without a held file.
    fixture.bar(Vendor::Dhan, Segment::Cash, "ADANIENT", 2);
    assert_eq!(fixture.locate("RELIANCE"), None);
    let (older_censuses, older_entries) = census_now(&fixture.site);
    let base = fixture.reads();
    let older_stamps = manifest_stamps(&fixture.site.store_root);

    fixture.publish(
        &[
            (Segment::Cash, "ADANIENT"),
            (Segment::Index, "ADANIENT"),
            (Segment::Cash, "RELIANCE"),
        ],
        0,
    );
    // THE TRIPWIRE: the status-change time is the ONLY term of the key this
    // rewrite moved. A key that also carries the length, the inode or anything
    // else a rewrite moves separates this rewrite from the older image by a
    // term that no tick can share, closes the gap this test pins, and fails
    // here, before the planted state could hide it.
    let newer_stamps = manifest_stamps(&fixture.site.store_root);
    assert_eq!(
        without_status_change(newer_stamps.clone()),
        without_status_change(older_stamps),
        "a rewrite at the same modified time moved a term of the census key \
         other than the status-change time; the gap docs/06-limits.md states \
         (\"two changes inside one tick\") is no longer the one the key has, \
         so that section and this test must be updated with the key"
    );
    // THE STATE A REWRITE INSIDE ONE TICK LEAVES: the older census, keyed on the
    // stamp the newer image answers with, which the assertion above has just
    // shown differs from the older one by the status-change time alone.
    *fixture
        .site
        .census
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) =
        Some((newer_stamps, older_censuses, older_entries));
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
    let (_, (_, entries)) = census_now_reading(&fixture.site, |root| {
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

/// Run `change`, again and again, until `path`'s status-change time has moved.
///
/// A mode change, a `set_modified` and a rename into place each move
/// `st_ctime`, but only to the tick of the clock the filesystem stamps with,
/// whose size is not this test's to assume. Two changes inside one tick share
/// a stamp, which the cache is right not to tell apart. The fixture SETS
/// modified times so that no answer here depends on the clock. A status-change
/// time cannot be set, so this waits the tick out instead, re-running
/// `change`, which must therefore be idempotent.
#[cfg(unix)]
fn past_a_ctime_tick(path: &Path, change: impl Fn()) {
    use std::os::unix::fs::MetadataExt as _;
    let changed = || {
        let meta = fs::metadata(path).expect("a manifest to stat");
        (meta.ctime(), meta.ctime_nsec())
    };
    let before = changed();
    change();
    for _ in 0..1_000 {
        if changed() != before {
            return;
        }
        std::thread::sleep(Duration::from_millis(1));
        change();
    }
    assert_ne!(
        changed(),
        before,
        "{} kept its status-change time through a thousand changes",
        path.display()
    );
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

/// ANOTHER FEED'S UNREADABLE CENSUS IS STEPPED OVER; ONLY THE ASKED FEED'S IS
/// REFUSED ON. D-0695.
///
/// `locate_series` refuses on an unreadable census only when it is the asked
/// feed's own, and D-0695 states the other half: an unreadable census of any
/// other feed is stepped over, and when a third feed places the name, that
/// identity is used and the note is not shown. Nothing drove that half.
/// `bars_refuses_when_the_asked_feeds_own_census_is_unreadable` checks it only
/// through Groww's own request, and Groww's census holds the name, so the walk
/// returns before it meets Dhan's damaged one. A `locate_series` that refused
/// on EVERY unreadable census it walked passed the whole api lib, and would
/// answer a Dhan request with "the asked feed's own census could not be read:
/// groww ...", which is false.
///
/// Here Groww's manifest is damaged, and Groww is walked before Zerodha
/// because it is first in `Vendor::ALL`. Dhan's census is read and does not
/// hold `ADANIENT`; Zerodha's holds it as CASH. Dhan's request, with
/// `?vendor=dhan`, with no vendor (Dhan by default) and with `?segment=CASH`,
/// opens Dhan's file at `NSE/CASH/ADANIENT` and says nothing of Groww's
/// census. The one bar is written straight into Dhan's file at that path,
/// and the census is left as published, so a 200 page showing Dhan's one CASH
/// bar proves the file was opened at that identity: the page names the feed
/// and segment, not the exchange.
#[tokio::test]
async fn bars_steps_over_another_feeds_unreadable_census_to_a_third_feeds_identity() {
    let fixture = Fixture::new("census-request-other-unreadable");
    fixture.publish_bytes(Vendor::Groww, &[0xFF; 16], 0);
    fixture.publish_for(Vendor::Dhan, &[(Segment::Index, "NIFTY")], 0);
    fixture.publish_for(Vendor::Zerodha, &[(Segment::Cash, "ADANIENT")], 0);
    fixture.bar(Vendor::Dhan, Segment::Cash, "ADANIENT", 2);
    assert_eq!(
        fixture.states(),
        ["unreadable", "held", "absent", "absent", "held"],
        "Groww damaged, Dhan read, Zerodha holding"
    );
    for query in [
        "symbol=ADANIENT&vendor=dhan&month=2025-05&timeframe=1day",
        "symbol=ADANIENT&month=2025-05&timeframe=1day",
        "symbol=ADANIENT&vendor=dhan&month=2025-05&timeframe=1day&segment=CASH",
    ] {
        let (status, page) = fixture.bars(query);
        assert_eq!(status, axum::http::StatusCode::OK, "{query}: {page}");
        assert!(
            page.contains("DHAN · CASH · 2025-05") && page.contains("<b>1</b> bar(s)"),
            "{query}: {page}"
        );
        assert!(
            !page.contains("own census could not be read"),
            "only the asked feed's census is refused on: {query}: {page}"
        );
        assert!(
            !page.contains("groww.man"),
            "and another feed's note is not shown: {query}: {page}"
        );
    }
}

/// A PERMISSION CHANGE ON A MANIFEST FILE IS SEEN ON THE NEXT REQUEST, warm or
/// cold, and so is its repair. D-0695.
///
/// A manifest that exists was keyed on its modified time alone, and `chmod`
/// moves no modified time. Cold on a file this process could not read -- one a
/// pull run wrote as another user -- "unreadable" was cached under a time the
/// repair left where it was, so `/calendar.json` answered 503 and `/bars`
/// refused over a readable store until the next pull or a restart. Warm on
/// "held", a file that stopped being readable was served held. The key now
/// carries the status-change time as well, which `chmod` moves. Both
/// directions are driven, and the cold one through both routes. Like the
/// directory tests, this needs a process the permission binds, which a root
/// process is not.
#[cfg(unix)]
#[tokio::test]
async fn a_permission_change_on_a_manifest_file_is_seen_on_the_next_request() {
    use std::os::unix::fs::PermissionsExt as _;
    let fixture = Fixture::new("census-request-manifest-file-mode");
    fixture.publish(&[(Segment::Index, "NIFTY")], 0);
    fixture.bar(Vendor::Dhan, Segment::Index, "NIFTY", 2);
    let manifest = fixture.manifest();
    let set_mode = |bits: u32| {
        past_a_ctime_tick(&manifest, || {
            fs::set_permissions(&manifest, fs::Permissions::from_mode(bits))
                .expect("a manifest whose mode this process may set");
        });
    };
    let bars = "symbol=NIFTY&vendor=dhan&month=2025-05&timeframe=1day";
    let unreadable = ["absent", "unreadable", "absent", "absent", "absent"];
    let held = ["absent", "held", "absent", "absent", "absent"];

    // WARM ON "HELD", then the file stops being readable.
    assert_eq!(fixture.states(), held, "warm on a readable manifest");
    set_mode(0o000);
    assert_eq!(
        fixture.fresh_states(),
        unreadable,
        "the permission binds this process"
    );
    assert_eq!(fixture.states(), unreadable, "warm: the fault is seen");
    let (status, body) = fixture.calendar_answer("feed=dhan").await;
    assert_eq!(
        status,
        axum::http::StatusCode::SERVICE_UNAVAILABLE,
        "{body}"
    );
    set_mode(0o644);
    assert_eq!(fixture.states(), held, "and so is the repair");

    // COLD ON THE FAULT, then repaired: the case that answered 503 for good.
    set_mode(0o000);
    *fixture
        .site
        .census
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = None;
    assert_eq!(fixture.states(), unreadable, "cold: the fault is seen");
    let (status, body) = fixture.calendar_answer("feed=dhan").await;
    assert_eq!(
        status,
        axum::http::StatusCode::SERVICE_UNAVAILABLE,
        "{body}"
    );
    let (status, page) = fixture.bars(bars);
    assert_eq!(status, axum::http::StatusCode::BAD_REQUEST, "{page}");
    assert!(page.contains("own census could not be read"), "{page}");

    set_mode(0o644);
    assert_eq!(fixture.fresh_states(), held);
    assert_eq!(
        fixture.states(),
        held,
        "the cached refusal does not outlive the repair"
    );
    let (status, body) = fixture.calendar_answer("feed=dhan").await;
    assert_eq!(status, axum::http::StatusCode::OK, "{body}");
    assert!(body.contains(r#""sessions":1"#), "{body}");
    let (status, page) = fixture.bars(bars);
    assert_eq!(status, axum::http::StatusCode::OK, "{page}");
}

/// A REWRITE THAT KEEPS ONLY THE MODIFIED TIME IS READ AGAIN. D-0695.
///
/// D-0686's gap was any rewrite that kept the cached modified time. Every
/// write also moves the status-change time, which the key now carries, so such
/// a rewrite is seen on the next request, read once. The gap left is a rewrite
/// that keeps both times, which `a_rewrite_that_keeps_the_stamp_is_served_stale_until_the_stamp_moves`
/// pins.
#[cfg(unix)]
#[tokio::test]
async fn a_rewrite_that_keeps_only_the_modified_time_is_read_again() {
    let fixture = Fixture::new("census-request-same-modified-time");
    fixture.publish(&[(Segment::Cash, "ADANIENT")], 0);
    assert_eq!(fixture.locate("RELIANCE"), None);
    let base = fixture.reads();

    let newer = Fixture::image(
        Vendor::Dhan,
        &[
            (Segment::Cash, "ADANIENT"),
            (Segment::Index, "ADANIENT"),
            (Segment::Cash, "RELIANCE"),
        ],
    );
    past_a_ctime_tick(&fixture.manifest(), || {
        fixture.publish_bytes(Vendor::Dhan, &newer, 0);
    });
    assert_eq!(
        fixture.locate("RELIANCE"),
        Some(("NSE".to_owned(), "CASH".to_owned())),
        "seen, though its modified time is the one cached"
    );
    assert_eq!(
        fixture.calendar("feed=dhan&symbol=ADANIENT").await,
        axum::http::StatusCode::CONFLICT,
        "the second identity is seen too"
    );
    assert_eq!(fixture.reads(), base + 1, "read once");
}

/// Empty the census cache, as a server that has not yet answered a request.
fn cold(fixture: &Fixture) {
    *fixture
        .site
        .census
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = None;
}

/// Each census's state, in the order given.
fn named(censuses: &[census::VendorCensus]) -> Vec<&'static str> {
    censuses.iter().map(|census| census.state.name()).collect()
}

/// A store root, or a directory or file in it, moved aside for one read, and
/// put back afterwards even when an assertion inside that read fails, so the
/// fixture's own `Drop` still finds it.
struct Aside<'a> {
    root: &'a Path,
    aside: PathBuf,
}

impl<'a> Aside<'a> {
    fn new(root: &'a Path) -> Self {
        let aside = root.with_extension("aside");
        fs::rename(root, &aside).expect("move it aside");
        Self { root, aside }
    }
}

impl Drop for Aside<'_> {
    fn drop(&mut self) {
        let _restored = fs::rename(&self.aside, self.root);
    }
}

/// A healthy store the tests below fault for one read: Dhan's manifest
/// holds NIFTY as INDEX and its one daily bar is on disk.
fn nifty_store(name: &str) -> Fixture {
    let fixture = Fixture::new(name);
    fixture.publish(&[(Segment::Index, "NIFTY")], 0);
    fixture.bar(Vendor::Dhan, Segment::Index, "NIFTY", 2);
    cold(&fixture);
    fixture
}

/// What `nifty_store` answers once nothing is faulted: held, one session on
/// `/calendar.json`, and a 200 page on `/bars`.
async fn serves_the_healthy_store(fixture: &Fixture, why: &str) {
    let held = ["absent", "held", "absent", "absent", "absent"];
    assert_eq!(fixture.fresh_states(), held, "{why}: the disk reads held");
    assert_eq!(
        fixture.states(),
        held,
        "{why}: the next request reads again"
    );
    let (status, body) = fixture.calendar_answer("feed=dhan").await;
    assert_eq!(status, axum::http::StatusCode::OK, "{why}: {body}");
    assert!(body.contains(r#""sessions":1"#), "{why}: {body}");
    let (status, page) = fixture.bars("symbol=NIFTY&vendor=dhan&month=2025-05&timeframe=1day");
    assert_eq!(status, axum::http::StatusCode::OK, "{why}: {page}");
    assert!(
        !page.contains("no feed in this store holds"),
        "{why}: {page}"
    );
}

/// A CENSUS THAT CONTRADICTS ITS OWN KEY IS SERVED TO THE REQUEST THAT READ IT
/// AND NOT CACHED: a store root gone when `read_all` asks for it, and back
/// before the next request. D-0695.
///
/// The stamps are taken before the read, which is right for installs. But the
/// read was then cached under them whatever it said. The root's absence moves
/// no manifest's stamp, so all five feeds were cached "unreadable" under stamps
/// that say Dhan's manifest exists: `/calendar.json` answered 503 and `/bars`
/// refused over a store that reads, until a manifest was next written or the
/// server restarted -- the stale 503 D-0695 removed for a permission change.
/// The read moves the root aside, reads, and puts it back.
#[tokio::test]
async fn a_store_root_that_vanishes_during_the_read_is_not_cached() {
    let fixture = nifty_store("census-request-root-vanishes");
    let (_, (served, _)) = census_now_reading(&fixture.site, |root| {
        let _aside = Aside::new(root);
        census::read_all(root)
    });
    assert_eq!(
        named(&served),
        vec!["unreadable"; 5],
        "the request that met the outage names it"
    );
    serves_the_healthy_store(&fixture, "after the outage").await;
}

/// A CENSUS THAT CONTRADICTS ITS OWN KEY IS NOT CACHED: a store root that
/// passes `read_all`'s own check and is gone for the manifest reads. D-0695.
///
/// Each `read_vendor` then meets `NotFound`, which it rightly calls absent, so
/// an all-"absent" census was cached under stamps that say Dhan's manifest
/// exists. `/calendar.json` then answered `200 {"sessions":0}` and `/bars` said
/// no feed holds the name: a claim about the store made from a read that did
/// not reach it, the silent fallback `CLAUDE.md` §4 bans, served until a
/// manifest was next written. The read here is `read_all`'s own two steps with
/// the root moved aside between them.
#[tokio::test]
async fn a_store_root_that_vanishes_after_its_check_is_not_cached_as_absent() {
    let fixture = nifty_store("census-request-root-after-check");
    let (_, (served, _)) = census_now_reading(&fixture.site, |root| {
        assert!(
            fs::metadata(root).is_ok_and(|meta| meta.is_dir()),
            "read_all's own check passes"
        );
        let _aside = Aside::new(root);
        Vendor::ALL
            .into_iter()
            .map(|vendor| census::read_vendor(root, vendor))
            .collect()
    });
    assert_eq!(
        named(&served),
        vec!["absent"; 5],
        "the request that raced the root answers what it read"
    );
    serves_the_healthy_store(&fixture, "after the race").await;
}

/// AN I/O ERROR NO TIME RECORDS IS NOT CACHED. D-0695.
///
/// A manifest whose `stat` answers and whose read then fails with `EIO` or
/// `EMFILE` has moved neither of its times, so its stamp is the one a clean
/// read has. Cached, "unreadable" outlived the fault: `/calendar.json` answered
/// 503 and `/bars` refused until the manifest was next written. No test can
/// make a disk return either on demand, so the read here is `read_all` with
/// Dhan's census replaced by what `read_vendor` makes of that error, through
/// the one mapping it uses, `Census::of_io_error`. The next request reads
/// again, once, and keeps what it read.
///
/// That mapping holds this test only while `read_vendor` goes through it, and
/// nothing here reads a disk that fails. So two tests read real faults through
/// `read_vendor` itself: `a_manifest_that_stats_but_will_not_open_is_not_cached`
/// an error of a kind no stamp decides, and `a_fault_its_stamp_can_see_is_cached`
/// the two kinds a stamp does. A `read_vendor` that labelled every I/O error
/// `Fault::Refused` passed this test and every other one before them.
#[cfg(unix)]
#[tokio::test]
async fn an_io_error_its_stamp_cannot_see_is_not_cached() {
    let held = ["absent", "held", "absent", "absent", "absent"];
    // `EIO` and `EMFILE`: 5 and 24 on Linux and on macOS alike.
    for errno in [5, 24] {
        let fixture = nifty_store(&format!("census-request-io-{errno}"));
        let (_, (served, _)) = census_now_reading(&fixture.site, |root| {
            let mut read = census::read_all(root);
            for census in &mut read {
                if census.vendor == Vendor::Dhan {
                    census.state =
                        census::Census::of_io_error(&std::io::Error::from_raw_os_error(errno));
                }
            }
            read
        });
        assert_eq!(
            named(&served),
            ["absent", "unreadable", "absent", "absent", "absent"],
            "errno {errno}: the request that met it names it"
        );
        let base = fixture.reads();
        assert_eq!(fixture.states(), held, "errno {errno}: read again");
        assert_eq!(fixture.reads(), base + 1, "errno {errno}: once");
        assert_eq!(fixture.states(), held, "errno {errno}: warm");
        assert_eq!(
            fixture.reads(),
            base + 1,
            "errno {errno}: and what it read is kept"
        );
        let (status, body) = fixture.calendar_answer("feed=dhan").await;
        assert_eq!(status, axum::http::StatusCode::OK, "errno {errno}: {body}");
        assert!(body.contains(r#""sessions":1"#), "errno {errno}: {body}");
        let (status, page) = fixture.bars("symbol=NIFTY&vendor=dhan&month=2025-05&timeframe=1day");
        assert_eq!(status, axum::http::StatusCode::OK, "errno {errno}: {page}");
    }
}

/// A MANIFEST THAT `stat` ANSWERS AND NO READ OPENS IS NOT CACHED, on a real
/// disk and through `read_vendor` itself. D-0695.
///
/// `an_io_error_its_stamp_cannot_see_is_not_cached` hands the cache the census
/// `Census::of_io_error` makes, so it cannot see whether `read_vendor` makes
/// the same one. This reads a real fault instead: a Unix socket bound at
/// Groww's manifest path. Its `stat` answers with both times, so its stamp is
/// the one a file's is, and opening it fails with a kind that is neither of the
/// two a stamp decides (`EOPNOTSUPP` on this macOS host, which `std` calls
/// `Unsupported`; the kind is not assumed here, only required to be neither).
/// `read_vendor` must label it `Fault::Io` of that kind, and the cache must
/// decline it: every request while the socket stays reads again, counted on
/// Dhan's healthy manifest beside it, and nothing is cached. Once the socket
/// is gone the next read is kept. A `read_vendor` that labelled every I/O
/// error `Fault::Refused` had this socket kept, under a stamp that cannot say
/// when the fault ends.
///
/// The socket's path must fit a socket address, so the scratch name is short.
#[cfg(unix)]
#[test]
fn a_manifest_that_stats_but_will_not_open_is_not_cached() {
    use std::io::ErrorKind;
    let fixture = Fixture::new("cr-sock");
    fixture.publish(&[(Segment::Index, "NIFTY")], 0);
    let groww = manifest_path(&fixture.root, Vendor::Groww);
    let socket = std::os::unix::net::UnixListener::bind(&groww)
        .expect("a socket at Groww's manifest path, whose path fits a socket address");
    let stamp = Vendor::ALL
        .into_iter()
        .zip(manifest_stamps(&fixture.root).manifests)
        .find_map(|(vendor, stamp)| (vendor == Vendor::Groww).then_some(stamp));
    assert!(
        matches!(stamp, Some(ManifestStamp::At { .. })),
        "the socket's `stat` answers with its times: {stamp:?}"
    );
    let state = census::read_vendor(&fixture.root, Vendor::Groww).state;
    assert!(
        matches!(
            state,
            census::Census::Unreadable {
                fault: census::Fault::Io(kind),
                ..
            } if !matches!(kind, ErrorKind::PermissionDenied | ErrorKind::IsADirectory)
        ),
        "`read_vendor` names an I/O fault of a kind no stamp decides: {state:?}"
    );

    let faulted = ["unreadable", "held", "absent", "absent", "absent"];
    cold(&fixture);
    let base = fixture.reads();
    for request in 1..=3 {
        assert_eq!(fixture.states(), faulted, "request {request}: served");
        assert_eq!(
            fixture.reads(),
            base + request,
            "request {request}: read again, not kept"
        );
    }
    assert!(
        fixture
            .site
            .census
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .is_none(),
        "nothing is cached while the socket stays"
    );

    drop(socket);
    fs::remove_file(&groww).expect("take the socket away");
    let repaired = ["absent", "held", "absent", "absent", "absent"];
    assert_eq!(fixture.states(), repaired, "its end is read");
    let after = fixture.reads();
    assert_eq!(fixture.states(), repaired, "warm");
    assert_eq!(fixture.reads(), after, "and what it read is kept");
}

/// A FAULT ITS STAMP CAN SEE IS STILL CACHED. D-0695.
///
/// A census the cache declines to keep is read again on every request, so the
/// rule must decline no more than it has to. Groww's manifest is, in turn,
/// bytes that do not decode, a file this process may not read, and a
/// directory. Each is unreadable, each is decided by what its stamp records --
/// the bytes by the modified and status-change times, the mode by the
/// status-change time a `chmod` moves, the directory by a replacement -- and
/// each must be kept: the warm request reads no manifest bytes, counted on
/// Dhan's healthy manifest beside it. Like the other permission tests, this
/// needs a process the permission binds, which a root process is not.
///
/// And `read_vendor` must say where each refusal came from: `Fault::Refused`
/// for the bytes, `Fault::Io` of `PermissionDenied` for the mode and of
/// `IsADirectory` for the directory. Those two kinds are what the cache keeps
/// under a stamp that found the manifest, so this pins which kinds reach that
/// list from a real disk, not only what the list says. A `read_vendor` that
/// labelled every I/O error `Refused` still had all three kept, and so passed
/// the counts below without them.
#[cfg(unix)]
#[test]
fn a_fault_its_stamp_can_see_is_cached() {
    use std::io::ErrorKind;
    use std::os::unix::fs::PermissionsExt as _;
    let fixture = Fixture::new("census-request-kept-faults");
    fixture.publish(&[(Segment::Index, "NIFTY")], 0);
    let groww = manifest_path(&fixture.root, Vendor::Groww);
    let undecodable = || fixture.publish_bytes(Vendor::Groww, &[0xFF; 16], 0);
    let unpermitted = || {
        fixture.publish_for(Vendor::Groww, &[(Segment::Cash, "ITC")], 0);
        fs::set_permissions(&groww, fs::Permissions::from_mode(0o000))
            .expect("a manifest whose mode this process may set");
    };
    let directory = || {
        fs::remove_file(&groww).expect("take the file away");
        fs::create_dir(&groww).expect("a directory where Groww's manifest goes");
    };
    let faults: [(&str, &dyn Fn(), census::Fault); 3] = [
        (
            "bytes that do not decode",
            &undecodable,
            census::Fault::Refused,
        ),
        (
            "a file this process may not read",
            &unpermitted,
            census::Fault::Io(ErrorKind::PermissionDenied),
        ),
        (
            "a directory",
            &directory,
            census::Fault::Io(ErrorKind::IsADirectory),
        ),
    ];
    let unreadable = ["unreadable", "held", "absent", "absent", "absent"];
    for (what, fault, from) in faults {
        fault();
        cold(&fixture);
        assert_eq!(
            fixture.fresh_states(),
            unreadable,
            "{what}: the fault binds this process"
        );
        let state = census::read_vendor(&fixture.root, Vendor::Groww).state;
        assert!(
            matches!(state, census::Census::Unreadable { fault, .. } if fault == from),
            "{what}: `read_vendor` names where it came from, {from:?}: {state:?}"
        );
        let base = fixture.reads();
        assert_eq!(fixture.states(), unreadable, "{what}: cold");
        assert_eq!(fixture.reads(), base + 1, "{what}: read once");
        assert_eq!(fixture.states(), unreadable, "{what}: warm");
        assert_eq!(fixture.reads(), base + 1, "{what}: and kept");
    }
}

/// EVERY LINE OF THE RULE, ONE ROW EACH: which census each stamp keeps.
/// D-0695.
///
/// `stamp_could_read` decides what the census cache keeps. The route tests
/// above reach its lines through real faults; this reaches every arm, each
/// from both sides, so a line dropped from it or widened fails here by name.
///
/// Then `read_as_stamped`'s two checks on the whole read, order and count,
/// each with a read that only it refuses. The short read here used to drop the
/// first row, which the order check refuses on its own, so deleting the count
/// check passed every test.
#[test]
fn a_census_is_kept_only_under_a_stamp_that_could_have_read_it() {
    use census::{Census, Fault};
    use std::io::ErrorKind;
    let exists = ManifestStamp::At {
        modified: at(0),
        changed: StatusChanged::default(),
    };
    let missing = ManifestStamp::Missing;
    let refused = ManifestStamp::Faulted(ErrorKind::PermissionDenied);
    let unreadable = |fault| Census::Unreadable {
        reason: String::from("fixture"),
        fault,
    };
    let held = || Census::Held {
        manifest: Box::new(Manifest::open(Vendor::Dhan, &[], &[]).expect("a genesis manifest")),
    };
    let io = |kind| unreadable(Fault::Io(kind));
    let not_a_directory = ManifestStamp::Faulted(ErrorKind::NotADirectory);
    // (the stamp, whether the root was a directory, what the read said, kept)
    let rows: [(ManifestStamp, bool, Census, bool); 19] = [
        // `read_all`'s root refusal: only under a root that was not a directory.
        (missing, false, unreadable(Fault::Root), true),
        (missing, true, unreadable(Fault::Root), false),
        (exists, true, unreadable(Fault::Root), false),
        // Any other census: only under a root that was.
        (missing, false, Census::Absent, false),
        (not_a_directory, false, io(ErrorKind::NotADirectory), false),
        // Absent: only where no manifest was found.
        (missing, true, Census::Absent, true),
        (exists, true, Census::Absent, false),
        (refused, true, Census::Absent, false),
        // Held, or refused by this reader: only where one was.
        (exists, true, held(), true),
        (missing, true, held(), false),
        (exists, true, unreadable(Fault::Refused), true),
        (missing, true, unreadable(Fault::Refused), false),
        // An I/O refusal of a stamped manifest: only the kinds its status decides.
        (exists, true, io(ErrorKind::PermissionDenied), true),
        (exists, true, io(ErrorKind::IsADirectory), true),
        (exists, true, io(ErrorKind::Interrupted), false),
        (exists, true, io(ErrorKind::Other), false),
        // An I/O refusal where the stamp failed: only of the same kind.
        (refused, true, io(ErrorKind::PermissionDenied), true),
        (refused, true, io(ErrorKind::NotADirectory), false),
        (missing, true, io(ErrorKind::PermissionDenied), false),
    ];
    for (stamp, root_is_dir, state, kept) in rows {
        assert_eq!(
            stamp_could_read(stamp, root_is_dir, &state),
            kept,
            "{stamp:?}, root a directory: {root_is_dir}, {state:?}"
        );
    }

    // THE WHOLE READ: one row per vendor, in `Vendor::ALL` order, or nothing.
    let stamps = CensusStamps {
        manifests: vec![missing; Vendor::ALL.len()],
        root_is_dir: true,
    };
    let absent: Vec<census::VendorCensus> = Vendor::ALL
        .into_iter()
        .map(|vendor| census::VendorCensus {
            vendor,
            path: PathBuf::from("fixture.man"),
            state: Census::Absent,
        })
        .collect();
    assert!(read_as_stamped(&stamps, &absent), "every row agrees");
    let mut reversed = absent.clone();
    reversed.reverse();
    assert!(!read_as_stamped(&stamps, &reversed), "out of order");
    // SHORT AT EITHER END, AND ONE OVER. A read missing its FIRST row puts
    // every row after it one vendor off, so the order check refuses it with or
    // without the count. Only the count refuses a read missing its LAST row,
    // whose rows still line up with `Vendor::ALL`, or one with a row past the
    // last vendor, which the zip never reaches.
    let (_, but_first) = absent.split_first().expect("five rows");
    let (_, but_last) = absent.split_last().expect("five rows");
    assert!(!read_as_stamped(&stamps, but_first), "the first row short");
    assert!(!read_as_stamped(&stamps, but_last), "the last row short");
    let mut one_over = absent.clone();
    one_over.push(absent.first().cloned().expect("five rows"));
    assert!(!read_as_stamped(&stamps, &one_over), "one row over");
    let mut one_off = absent;
    if let Some(dhan) = one_off.get_mut(1) {
        dhan.state = held();
    }
    assert!(!read_as_stamped(&stamps, &one_off), "one row disagrees");
}

/// A CALENDAR DERIVED WHILE A HELD BAR FILE WOULD NOT OPEN IS REFUSED TO ITS
/// REQUEST AND NOT KEPT, on both branches of `/calendar.json`. D-0695.
///
/// The census cache keeps only a read its stamps could have made. The calendar
/// cache behind it kept whatever `calendar_of::derive` returned. With
/// `bars/dhan` moved aside for one request, NIFTY's one held daily month did
/// not open, and `/calendar.json` answered `200 {"sessions":0}`. That calendar
/// was cached under the manifest's modified time, which the fault had not
/// moved, so it was still served after the directory came back, while `/bars`
/// served the month. A review measured both halves. Now the request that meets
/// the fault is refused with 503, naming the file it could not open, and the
/// next request derives again and answers the one session.
///
/// Each branch gets its own store, because each must meet the fault with the
/// calendar cache cold: a kept calendar is a hit, and a hit opens nothing.
#[tokio::test]
async fn a_calendar_derived_while_the_bars_were_away_is_refused_and_not_kept() {
    for (branch, query) in [
        ("symbol", "feed=dhan&symbol=NIFTY"),
        ("exchange", "feed=dhan"),
    ] {
        let fixture = nifty_store(&format!("census-request-bars-away-{branch}"));
        let bars = fixture.root.join("bars").join("dhan");
        let (status, body) = {
            let _aside = Aside::new(&bars);
            fixture.calendar_answer(query).await
        };
        assert_eq!(
            status,
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            "{branch}: the request that met the fault refuses: {body}"
        );
        assert!(body.contains(r#""bars":"unopened""#), "{branch}: {body}");
        assert!(body.contains(r#""unopened":1"#), "{branch}: {body}");
        assert!(
            body.contains("NIFTY") && body.contains("2025-05"),
            "{branch}: it names the file it could not open: {body}"
        );
        let (status, body) = fixture.calendar_answer(query).await;
        assert_eq!(status, axum::http::StatusCode::OK, "{branch}: {body}");
        assert!(
            body.contains(r#""sessions":1"#),
            "{branch}: the next request derives again, and nothing was kept: {body}"
        );
    }
}

/// A PEER DERIVED WHILE ITS BARS WERE AWAY IS NAMED, NOT COUNTED, AND NOT
/// KEPT. D-0695.
///
/// `/gaps.json`'s peer vote reads through the same calendar cache. Asked for
/// BANKNIFTY on Zerodha, Dhan's NIFTY is the one peer. With `bars/dhan` moved
/// aside, its derivation opens nothing its census holds: it votes for nothing,
/// and is named under `unreadable` as `dhan:NIFTY` beside the unreadable
/// censuses, where it was silently no vote at all. Once the directory is back,
/// the next vote derives again and counts it.
#[test]
fn a_peer_derived_while_its_bars_were_away_is_named_and_not_kept() {
    let fixture = nifty_store("census-request-peer-away");
    let asked = Addressed::parse(
        "feed=zerodha&exchange=NSE&segment=INDEX&symbol=BANKNIFTY&timeframe=1min&month=2025-05",
    )
    .expect("a well-formed address");
    let bars = fixture.root.join("bars").join("dhan");
    let away = {
        let _aside = Aside::new(&bars);
        peer_calendar(&fixture.site, &asked)
    };
    assert!(away.from.is_empty(), "no vote: {:?}", away.from);
    assert!(away.calendar.is_none(), "and so no agreed calendar");
    assert_eq!(away.unreadable, ["dhan:NIFTY"], "the peer is named");
    let back = peer_calendar(&fixture.site, &asked);
    assert_eq!(back.from, ["dhan:NIFTY"], "the next vote counts it");
    assert!(back.unreadable.is_empty(), "{:?}", back.unreadable);
}

/// A PEER THAT OPENED SOME OF ITS HELD FILES AND NOT ANOTHER IS NAMED, AND
/// COUNTS NO VOTE. D-0695.
///
/// `a_peer_derived_while_its_bars_were_away_is_named_and_not_kept` moves all
/// of `bars/dhan` aside, so the peer's derivation holds no session, and the
/// `sessions() > 0` filter drops its calendar whether or not a named peer is
/// skipped. A review deleted that skip and every test still passed. Here Dhan
/// holds NIFTY for May and June at the daily rung, both bars on disk, and only
/// June's file is moved aside. The derivation still carries May's session, so
/// only the skip keeps a peer named under `unreadable` from voting a calendar
/// short of June. It is the one witness here, and `agree` would take its
/// calendar as the agreed one. With June's file back, the next vote counts
/// both sessions.
#[test]
fn a_peer_that_opened_only_some_of_its_held_files_is_named_and_does_not_vote() {
    let fixture = nifty_store("census-request-peer-partial");
    let june_bar = fixture.bar_in(Vendor::Dhan, Segment::Index, "NIFTY", june(), 2);
    fixture.publish_nifty(&[may(), june()], 1);
    let asked = Addressed::parse(
        "feed=zerodha&exchange=NSE&segment=INDEX&symbol=BANKNIFTY&timeframe=1min&month=2025-05",
    )
    .expect("a well-formed address");
    let partial = {
        let _aside = Aside::new(&june_bar);
        peer_calendar(&fixture.site, &asked)
    };
    assert_eq!(partial.unreadable, ["dhan:NIFTY"], "the peer is named");
    assert!(
        partial.from.is_empty(),
        "a named peer counts no vote: {:?}",
        partial.from
    );
    assert!(
        partial.calendar.is_none(),
        "and its calendar, short of June, is not agreed: {:?}",
        partial.calendar.map(|calendar| calendar.sessions())
    );
    let back = peer_calendar(&fixture.site, &asked);
    assert_eq!(back.from, ["dhan:NIFTY"], "the next vote counts it");
    assert!(back.unreadable.is_empty(), "{:?}", back.unreadable);
    assert_eq!(
        back.calendar.map(|calendar| calendar.sessions()),
        Some(2),
        "with both months"
    );
}

/// Where June lands, relative to the census a caller derives from.
#[derive(Clone, Copy, Debug)]
enum Landing {
    /// After `census_now_stamped` has returned and before the caller derives:
    /// the stamps and the census both predate June.
    AfterTheCensus,
    /// Inside the census's own read, once the manifests are read: the census
    /// predates June, and so do the stamps only if they were taken BEFORE that
    /// read.
    InsideTheRead,
}

/// A census read the production way, with June landed as a pull lands it --
/// its daily bar, and a manifest naming May and June at a new modified time
/// -- where `landing` says.
///
/// What a caller that takes its census through this hook derives from is the
/// census as it was before June arrived, while the disk it derives from and
/// every manifest `stat` taken after the landing already show June.
fn landing_june(
    fixture: &Fixture,
    landing: Landing,
) -> impl FnOnce(&Site) -> (CensusStamps, CensusNow) + '_ {
    let land = move || {
        fixture.bar_in(Vendor::Dhan, Segment::Index, "NIFTY", june(), 2);
        fixture.publish_nifty(&[may(), june()], 7);
    };
    move |site| match landing {
        Landing::AfterTheCensus => {
            let read = census_now_stamped(site);
            land();
            read
        }
        Landing::InsideTheRead => census_now_reading(site, |root| {
            let read = census::read_all(root);
            land();
            read
        }),
    }
}

/// What the ingest path would land for Dhan's NIFTY at one minute: the
/// window `ingestion_observations` derives an observation for.
fn landed_nifty() -> BrokerWindow {
    let spec = match pull::vendor::Feed::Dhan.descriptor().transport {
        pull::vendor::Transport::Http(spec) => Some(spec),
        pull::vendor::Transport::LocalArchive(_) => None,
    }
    .expect("Dhan is an HTTP feed");
    let day = Day::new(2025, 5, 2).expect("fixture date");
    let window = pull::session::Window::new(day, day).expect("a one-day window");
    BrokerWindow {
        listing: pull::vendor::Listing::Index,
        contract: None,
        unfetched: None,
        instrument: "NIFTY".to_owned(),
        origin: "test only".to_owned(),
        spec,
        exchange: "NSE",
        segment: "INDEX",
        store_vendor: Vendor::Dhan,
        window,
        granularity: pull::vendor::Granularity::Minute1,
        bodies: Vec::new(),
    }
}

/// The series `/gaps.json` audits in these tests: Zerodha's BANKNIFTY, whose
/// one peer in `nifty_store` is Dhan's NIFTY.
fn zerodha_banknifty() -> Addressed {
    Addressed::parse(
        "feed=zerodha&exchange=NSE&segment=INDEX&symbol=BANKNIFTY&timeframe=1min&month=2025-05",
    )
    .expect("a well-formed address")
}

/// Each caller that derives a calendar from a census, and so keeps one under
/// `calendar_of::cached`'s key for Dhan's NIFTY as INDEX.
#[derive(Clone, Copy, Debug)]
enum Caller {
    /// `/calendar.json?feed=dhan&symbol=NIFTY`.
    Symbol,
    /// `/calendar.json?feed=dhan`: the exchange's calendar, agreed over every
    /// series Dhan holds.
    Exchange,
    /// `/gaps.json`'s peer vote for Zerodha's BANKNIFTY.
    Peer,
    /// The ingest path's observation for Dhan's NIFTY at one minute.
    Observation,
}

impl Caller {
    /// Every caller, in the order the tests below walk them.
    const ALL: [Self; 4] = [Self::Symbol, Self::Exchange, Self::Peer, Self::Observation];

    /// One call, over the census `census` hands it, and the sessions of the
    /// calendar it answered: `None` where it answered no calendar at all -- a
    /// peer vote nobody cast, or no observation.
    fn reading(
        self,
        site: &Site,
        census: impl FnOnce(&Site) -> (CensusStamps, CensusNow),
    ) -> Option<u32> {
        match self {
            Self::Symbol | Self::Exchange => {
                let query = if matches!(self, Self::Symbol) {
                    "feed=dhan&symbol=NIFTY"
                } else {
                    "feed=dhan"
                };
                let uri = format!("/calendar.json?{query}").parse().expect("uri");
                let (status, _, body) = calendar_json_reading(site, &uri, census);
                assert_eq!(status, axum::http::StatusCode::OK, "{self:?}: {body}");
                let answer: serde_json::Value =
                    serde_json::from_str(&body).expect("a calendar as JSON");
                let sessions = answer
                    .get("sessions")
                    .and_then(serde_json::Value::as_u64)
                    .expect("a session count");
                Some(u32::try_from(sessions).expect("a session count that fits"))
            }
            Self::Peer => peer_calendar_reading(site, &zerodha_banknifty(), census)
                .calendar
                .map(|calendar| calendar.sessions()),
            Self::Observation => ingestion_observations_reading(&landed_nifty(), site, census)
                .map(|calendar| calendar.sessions()),
        }
    }

    /// [`Self::reading`] over the census production takes: what each wrapper
    /// passes, `census_now_stamped`.
    fn sessions(self, site: &Site) -> Option<u32> {
        self.reading(site, census_now_stamped)
    }
}

/// What the calendar cache keeps for Dhan's NIFTY as INDEX: the modified time
/// it was kept under, and the calendar.
fn kept_nifty(fixture: &Fixture) -> Option<(SystemTime, Arc<pull::calendar::Calendar>)> {
    let key = (
        Vendor::Dhan,
        "NSE".to_owned(),
        "INDEX".to_owned(),
        "NIFTY".to_owned(),
    );
    fixture
        .site
        .calendars
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .get(&key)
        .map(|(at, calendar)| (*at, Arc::clone(calendar)))
}

/// Empty the calendar cache, as a server that has not yet derived one.
fn forget_calendars(fixture: &Fixture) {
    fixture
        .site
        .calendars
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clear();
}

/// A CALENDAR DERIVED FROM A CENSUS IS KEPT UNDER THAT CENSUS'S STAMP, NOT A
/// NEWER ONE, on every route that derives one. D-0695.
///
/// `calendar_of::cached` took its key itself: one `stat` of the manifest, after
/// the caller's census had supplied the months. A pull that installed a
/// manifest between the two had the calendar derived from the OLDER census's
/// months kept under the NEWER modified time. Every later request read the
/// newer census, hit that calendar, and answered it until the manifest was next
/// written, which can be the next day's pull. A review measured it on
/// `/calendar.json`: three requests after the install each answered one
/// session, where a cold derivation over both months answered two. Nothing
/// failed to open, so the rule that keeps only a derivation that opened every
/// held file keeps this one.
///
/// Each caller that derives from a census meets that interleaving here, over a
/// store holding NIFTY's May on Dhan with its calendar cache cold: both
/// branches of `/calendar.json`, `/gaps.json`'s peer vote, and the ingest
/// path's observation. Each takes its census through `landing_june`. The call
/// that met the install answers what its own census holds, one session. Every
/// later call reads the census naming June and must answer two.
///
/// # And June lands twice, because "before the census" has two halves
///
/// Landed after `census_now_stamped` returns, June is newer than the stamps
/// and the census alike, so a key taken after `census_now_stamped` has
/// returned is caught, and a key taken after the read but before it returns
/// is not. A review returned the
/// stamps from a `stat` taken AFTER the read, with the census cache itself
/// still keyed on the stamps before it, and every test passed: the case that
/// fails it is an install inside the read, which such stamps already show and
/// the census does not. So each caller also meets June landed inside
/// `census_now_reading`'s read.
#[test]
fn a_calendar_is_kept_under_the_stamp_of_the_census_it_was_derived_from() {
    for landing in [Landing::AfterTheCensus, Landing::InsideTheRead] {
        for caller in Caller::ALL {
            let fixture = nifty_store(&format!("census-request-june-{landing:?}-{caller:?}"));
            assert_eq!(
                caller.reading(&fixture.site, landing_june(&fixture, landing)),
                Some(1),
                "{landing:?}, {caller:?}: the call that met the install answers its own census"
            );
            for request in 1..=3 {
                assert_eq!(
                    caller.sessions(&fixture.site),
                    Some(2),
                    "{landing:?}, {caller:?}, request {request}: May and June, not the \
                     calendar kept before June"
                );
            }
        }
    }
}

/// EVERY CALLER KEEPS ITS CALENDAR, UNDER THE ASKED FEED'S OWN MODIFIED TIME,
/// AND ITS NEXT CALL MEETS IT. D-0695.
///
/// `a_calendar_is_kept_under_the_stamp_of_the_census_it_was_derived_from` pins
/// that nothing is kept under a NEWER stamp, and passes whether a calendar is
/// kept at all: a caller that hands `cached` no stamp derives on every call
/// and answers the same. A review made `CensusStamps::modified` answer `None`,
/// then answer another feed's time, and then had each caller in turn hand
/// `cached` `None`, and the `api` lib passed every time. The first turns the
/// calendar cache off -- 0.28 s per instrument on every request, and every
/// instrument on the exchange branch -- and the second keys Dhan's calendar on
/// another feed's manifest, which a pull of Dhan does not move: the stale
/// calendar the key exists to prevent.
///
/// Here every feed has a manifest, each at a modified time of its own, and
/// Dhan's holds NIFTY's May. Over a cold calendar cache, each caller must keep
/// NIFTY's calendar under Dhan's modified time as its `stat` reads it, and its
/// second call must meet that calendar rather than derive and keep another.
/// Then June lands on Dhan alone, every other manifest where it was, and each
/// caller must answer both months: a calendar kept under a feed that did not
/// move would still answer one.
#[test]
fn every_caller_keeps_its_calendar_under_the_asked_feeds_own_modified_time() {
    let fixture = nifty_store("census-request-kept-own-time");
    for (minute, vendor) in (1..).zip(Vendor::ALL) {
        if vendor != Vendor::Dhan {
            fixture.publish_for(vendor, &[], minute);
        }
    }
    let dhan = fs::metadata(fixture.manifest())
        .and_then(|meta| meta.modified())
        .expect("Dhan's manifest stamp");
    for caller in Caller::ALL {
        forget_calendars(&fixture);
        assert_eq!(caller.sessions(&fixture.site), Some(1), "{caller:?}: May");
        let first = kept_nifty(&fixture);
        assert_eq!(
            first.as_ref().map(|(at, _)| *at),
            Some(dhan),
            "{caller:?}: kept, and under Dhan's own modified time"
        );
        assert_eq!(caller.sessions(&fixture.site), Some(1), "{caller:?}: again");
        assert!(
            first
                .zip(kept_nifty(&fixture))
                .is_some_and(|((_, first), (_, second))| Arc::ptr_eq(&first, &second)),
            "{caller:?}: the second call met the kept calendar and kept no other"
        );
    }
    fixture.bar_in(Vendor::Dhan, Segment::Index, "NIFTY", june(), 2);
    fixture.publish_nifty(&[may(), june()], 7);
    for caller in Caller::ALL {
        assert_eq!(
            caller.sessions(&fixture.site),
            Some(2),
            "{caller:?}: May and June, not a calendar kept under a feed that did not move"
        );
    }
}

/// Race one `/calendar.json?feed=dhan&symbol=NIFTY` request against its store
/// root, gone after `read_all`'s own check, and back as the read ends or, with
/// `until_the_route_returns`, once the route has answered. Then require that
/// the request that raced it answered what its census read, and that every
/// caller after it answers NIFTY's one session.
///
/// Every feed then reads `NotFound`, which is "absent", while Dhan's stamp is
/// still the modified time of a manifest that holds NIFTY. The census cache
/// refuses to keep that read. But `census_now_stamped` still hands its stamps
/// to the request, and the symbol branch of `/calendar.json` derives even for a
/// name its census does not hold: over no months, under the default NSE/INDEX
/// identity, which for NIFTY is the real series' key. That empty calendar was
/// kept under Dhan's unmoved modified time, and once the root was back every
/// caller hit it until Dhan's manifest was next written: `/calendar.json` on
/// both branches answered no session, the peer vote silently lost Dhan, and
/// the ingest path observed nothing. A review measured all four.
fn race_the_root(name: &str, until_the_route_returns: bool) {
    let fixture = nifty_store(name);
    let mut away = None;
    let raced = Caller::Symbol.reading(&fixture.site, |site| {
        census_now_reading(site, |root| {
            assert!(
                fs::metadata(root).is_ok_and(|meta| meta.is_dir()),
                "read_all's own check passes"
            );
            let aside = Aside::new(&fixture.root);
            let read = Vendor::ALL
                .into_iter()
                .map(|vendor| census::read_vendor(root, vendor))
                .collect();
            if until_the_route_returns {
                away = Some(aside);
            }
            read
        })
    });
    drop(away);
    assert_eq!(
        raced,
        Some(0),
        "the request that raced the root answers what it read"
    );
    for caller in Caller::ALL {
        for request in 1..=3 {
            assert_eq!(
                caller.sessions(&fixture.site),
                Some(1),
                "{caller:?}, request {request}: NIFTY's May, not the empty calendar the \
                 race derived"
            );
        }
    }
}

/// A CALENDAR DERIVED FROM A CENSUS ROW ITS STAMP COULD NOT HAVE READ IS
/// ANSWERED TO ITS REQUEST AND NOT KEPT: a store root gone after `read_all`'s
/// own check and back before the route derives. D-0695.
///
/// This case predates D-0695's fifth repair: `cached` then took its own `stat`
/// after the census, and the root was back to answer it. See `race_the_root`.
#[test]
fn a_calendar_derived_from_a_row_its_stamp_could_not_have_read_is_not_kept() {
    race_the_root("census-request-root-back-after-the-read", false);
}

/// The same, with the root still gone while the route derives and back once
/// it has answered. D-0695.
///
/// This case the fifth repair opened. Before it, `cached`'s own `stat` of a
/// manifest under a root that was gone found no time, and nothing was kept.
/// Its key is now the stamp taken before the read, which the root's absence
/// does not reach. See `race_the_root`.
#[test]
fn a_calendar_derived_while_the_root_stays_away_is_not_kept() {
    race_the_root("census-request-root-back-after-the-route", true);
}

/// `CensusStamps::modified` HANDS EACH FEED'S ROW ITS OWN MANIFEST'S MODIFIED
/// TIME, AND ONLY WHEN THAT STAMP COULD HAVE READ THE ROW. D-0695.
///
/// It is the key every kept calendar is kept under, and nothing else asks it.
/// A review made it answer `None`, and then answer a neighbouring feed's time,
/// and the `api` lib passed both. Here every feed's manifest is on disk at a
/// modified time of its own, so a time taken from any other feed's stamp is a
/// different time. Then a row its stamp could not have read -- the rows
/// `stamp_could_read` refuses, a store root gone after its check among them --
/// is handed no time although its stamp has one, and a stamp with no time
/// hands none to a row it could have read.
#[test]
fn each_feed_is_handed_its_own_modified_time_only_for_a_row_its_stamp_could_read() {
    use census::{Census, Fault};
    use std::io::ErrorKind;
    let fixture = Fixture::new("census-request-modified-per-feed");
    for (minute, vendor) in (1..).zip(Vendor::ALL) {
        fixture.publish_for(vendor, &[], minute);
    }
    let stamps = manifest_stamps(&fixture.root);
    let read = census::read_all(&fixture.root);
    assert_eq!(named(&read), vec!["held"; 5], "every feed's manifest reads");
    for ((minute, vendor), row) in (1..).zip(Vendor::ALL).zip(&read) {
        assert_eq!(row.vendor, vendor, "one row per vendor, in order");
        assert_eq!(
            stamps.modified(row),
            Some(at(minute)),
            "{vendor:?}: its own manifest's modified time"
        );
    }

    let dhan = read
        .iter()
        .find(|row| row.vendor == Vendor::Dhan)
        .expect("Dhan's row");
    let unreadable = |fault| Census::Unreadable {
        reason: String::from("fixture"),
        fault,
    };
    for (state, why) in [
        (Census::Absent, "absent under a manifest stamped present"),
        (
            unreadable(Fault::Root),
            "the root refused under one that answered",
        ),
        (
            unreadable(Fault::Io(ErrorKind::Interrupted)),
            "an I/O error no time records",
        ),
    ] {
        let mut row = dhan.clone();
        row.state = state;
        assert_eq!(stamps.modified(&row), None, "{why}");
    }

    let untimed = CensusStamps {
        manifests: vec![ManifestStamp::Faulted(ErrorKind::PermissionDenied); Vendor::ALL.len()],
        root_is_dir: true,
    };
    let mut refused = dhan.clone();
    refused.state = unreadable(Fault::Io(ErrorKind::PermissionDenied));
    assert!(
        stamp_could_read(
            ManifestStamp::Faulted(ErrorKind::PermissionDenied),
            true,
            &refused.state
        ),
        "a row this stamp could have read"
    );
    assert_eq!(untimed.modified(&refused), None, "and no time to hand it");
    let missing = CensusStamps {
        manifests: vec![ManifestStamp::Missing; Vendor::ALL.len()],
        root_is_dir: true,
    };
    let mut absent = dhan.clone();
    absent.state = Census::Absent;
    assert_eq!(
        missing.modified(&absent),
        None,
        "a missing manifest has none"
    );
}

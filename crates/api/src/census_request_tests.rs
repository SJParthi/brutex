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
        let month = YearMonth::new(2025, 5).expect("fixture month");
        let day = Day::new(2025, 5, 2).expect("fixture date");
        let ts = i64::from(day.days_from_epoch()) * 86_400_000_000 + 21_600_000_000;
        let mut manifest = Manifest::open(Vendor::Dhan, &[], &[]).expect("a genesis manifest");
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
        let path = self.manifest();
        fs::create_dir_all(path.parent().expect("manifest parent")).expect("manifest dir");
        fs::write(&path, manifest.image()).expect("publish the manifest");
        fs::File::options()
            .write(true)
            .open(&path)
            .expect("the manifest just written")
            .set_modified(
                SystemTime::UNIX_EPOCH
                    + Duration::from_secs(1_700_000_000)
                    + Duration::from_mins(minute),
            )
            .expect("a filesystem that carries modified times");
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
    fn locate(&self, symbol: &str) -> Option<(String, String)> {
        locate_series(&self.site, &format!("symbol={symbol}"), symbol)
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

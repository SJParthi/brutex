#![cfg(test)]
//! The peer vote, asked for with `calendar=peers`, retains its full exchange
//! and segment identity; the default denominator is the calendar of record
//! (D-3336), which no peer moves.
#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "finite owned fixtures and exact response assertions"
)]

use super::*;
use brutex_core::instrument::{Exchange, Segment};
use pull::manifest::{Closes, Entry, EntryKey, Held};
use serde_json::Value;
use std::fs;
use store::format::{Bar, OI_NULL};
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

struct Fixture {
    root: PathBuf,
    site: Loaded,
    day: i64,
    originals: Vec<(PathBuf, Vec<u8>)>,
    /// How many manifest publications this fixture has stamped. See `write`.
    stamps: u64,
}

impl Fixture {
    fn new() -> Self {
        Self::named("gap-peer-venue-identity")
    }

    /// A fixture under its own scratch name, so two tests never share one.
    fn named(name: &str) -> Self {
        let root = crate::scratch::path(name);
        fs::create_dir(&root).expect("exclusively claim generated fixture");
        let masters = root.join("masters");
        fs::create_dir(&masters).expect("offline empty masters");
        let first = Day::new(2024, 1, 1).expect("fixture month");
        let day = (first.days_from_epoch()..=first.end_of_month().days_from_epoch())
            .map(i64::from)
            .find(|day| {
                matches!(
                    pull::calendar::kind_of(*day),
                    pull::calendar::DayKind::Open(_)
                )
            })
            .expect("a session named by the existing calendar");
        Self {
            site: Loaded::new(Site::load(&masters, &root)),
            root,
            day,
            originals: Vec::new(),
            stamps: 0,
        }
    }

    fn write(
        &mut self,
        vendor: Vendor,
        exchange: Exchange,
        segment: Segment,
        symbol: &str,
        hole: Option<u16>,
    ) {
        let month = YearMonth::new(2024, 1).expect("fixture month");
        let rows: Vec<_> = (pull::calendar::OPEN_MINUTE..=pull::calendar::LAST_MINUTE)
            .filter(|minute| Some(*minute) != hole)
            .map(|minute| Bar {
                ts_micros: (self.day * 86_400 - 19_800 + i64::from(minute) * 60) * 1_000_000,
                open: 100,
                high: 110,
                low: 90,
                close: 105,
                volume: 1,
                open_interest: OI_NULL,
            })
            .collect();
        let hash = brutex_core::universe::fnv1a(symbol).to_le_bytes();
        let id = u32::from_le_bytes(hash[..4].try_into().expect("low32 symbol identity"));
        for (timeframe, bars) in [
            (Timeframe::MINUTE_1, rows.as_slice()),
            (Timeframe::DAY_1, &rows[..1]),
        ] {
            let path = StorePath::new(PathParts {
                vendor,
                exchange: exchange.as_str(),
                segment: segment.as_str(),
                symbol,
                contract: None,
                timeframe,
                month,
                file: FileKind::Bars,
            })
            .expect("generated venue address");
            let mut file = store::file::BarFile::open_or_create(&self.root, path, id)
                .expect("owned generated bar writer");
            file.append(bars).expect("ordered generated records");
            drop(file);
            let held = Held::new(
                Entry {
                    key: EntryKey {
                        contract: None,
                        exchange,
                        segment,
                        symbol: brutex_core::symbol::Symbol::new(symbol).expect("generated symbol"),
                        timeframe,
                        month,
                    },
                    rows: u64::try_from(bars.len()).expect("finite rows"),
                    first_ts_micros: bars[0].ts_micros,
                    last_ts_micros: bars.last().expect("nonempty rows").ts_micros,
                },
                Closes::UNKNOWN,
            );
            assert_eq!(pull::ingest::record_held(&self.root, vendor, &[held]), None);
            let path = path.to_path_buf(&self.root);
            self.originals
                .push((path.clone(), fs::read(path).expect("original source")));
        }
        // STAMPED EXPLICITLY, NOT LEFT TO THE CLOCK. The peer vote reads the
        // census through `census_now`, keyed on this manifest's modified time
        // (D-0686), and a filesystem's timestamp resolution is not this test's
        // to assume: two publications inside one tick would share a stamp and
        // the audit after the second would not see it.
        self.stamps += 1;
        fs::File::options()
            .write(true)
            .open(pull::manifest::manifest_path(&self.root, vendor))
            .expect("the manifest just published")
            .set_modified(
                std::time::SystemTime::UNIX_EPOCH
                    + std::time::Duration::from_secs(1_700_000_000)
                    + std::time::Duration::from_mins(self.stamps),
            )
            .expect("a filesystem that carries modified times");
    }

    async fn get(&self) -> Value {
        self.get_with("&calendar=peers").await
    }

    /// The audit of Dhan's SUBJECT for January 2024, with `extra` appended.
    async fn get_with(&self, extra: &str) -> Value {
        let uri = format!(
            "/gaps.json?feed=dhan&exchange=NSE&segment=INDEX&symbol=SUBJECT&timeframe=1min&month=2024-01{extra}"
        )
        .parse()
        .expect("fixture URI");
        let (status, _, body) =
            gaps_json(axum::extract::State(Loaded::clone(&self.site)), uri).await;
        assert_eq!(status, axum::http::StatusCode::OK, "{body}");
        serde_json::from_str(&body).expect("actual gap response")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[tokio::test]
async fn gap_audit_peers_must_match_both_exchange_and_segment() {
    let mut fixture = Fixture::new();
    let hole = pull::calendar::OPEN_MINUTE + 10;
    fixture.write(
        Vendor::Dhan,
        Exchange::Nse,
        Segment::Index,
        "SUBJECT",
        Some(hole),
    );
    let baseline = fixture.get().await;
    assert_eq!(baseline["calendar"]["source"], "table");
    assert_eq!(baseline["calendar"]["voted_by"], serde_json::json!([]));
    assert_eq!(baseline["calendar"]["unreadable"], serde_json::json!([]));

    fixture.write(
        Vendor::Groww,
        Exchange::Bse,
        Segment::Index,
        "FOREIGN",
        None,
    );
    fixture.write(
        Vendor::Zerodha,
        Exchange::Nse,
        Segment::Cash,
        "OTHERSEG",
        None,
    );
    assert_eq!(
        fixture.get().await,
        baseline,
        "unrelated venue evidence cannot change the audit"
    );

    fixture.write(
        Vendor::Groww,
        Exchange::Nse,
        Segment::Index,
        "SUBJECT",
        None,
    );
    let audited = fixture.get().await;
    assert_eq!(audited["calendar"]["source"], "peers");
    assert_eq!(
        audited["calendar"]["voted_by"],
        serde_json::json!(["groww:SUBJECT"])
    );
    assert_eq!(audited["lost_minutes"], 1);
    assert_eq!(audited["held"], 374);
    assert_eq!(audited["expected"], 375);
    assert_eq!(
        fixture.get().await,
        audited,
        "warm-cache audit is identical"
    );
    fixture.write(
        Vendor::Dhan,
        Exchange::Nse,
        Segment::Index,
        "COMPARABLE",
        None,
    );
    let another = fixture.get().await;
    let mut voters: Vec<_> = another["calendar"]["voted_by"]
        .as_array()
        .expect("named voters")
        .iter()
        .map(|voter| voter.as_str().expect("voter identity"))
        .collect();
    voters.sort_unstable();
    assert_eq!(voters, ["dhan:COMPARABLE", "groww:SUBJECT"]);
    assert_eq!(another["lost_minutes"], 1);
    assert_eq!(fixture.get().await, another, "stable native voter order");

    // A PEER CENSUS THAT CANNOT BE READ IS NAMED, and the vote is otherwise
    // what it was: Zerodha held only an off-venue series, so it never voted.
    let zerodha = pull::manifest::manifest_path(&fixture.root, Vendor::Zerodha);
    fs::write(&zerodha, [0xFF_u8; 16]).expect("damage Zerodha's census");
    fs::File::options()
        .write(true)
        .open(&zerodha)
        .expect("the damaged manifest")
        .set_modified(
            std::time::SystemTime::UNIX_EPOCH
                + std::time::Duration::from_secs(1_700_000_000)
                + std::time::Duration::from_mins(fixture.stamps + 1),
        )
        .expect("a filesystem that carries modified times");
    let damaged = fixture.get().await;
    assert_eq!(
        damaged["calendar"]["unreadable"],
        serde_json::json!(["zerodha"])
    );
    assert_eq!(
        damaged["calendar"]["voted_by"],
        another["calendar"]["voted_by"]
    );
    assert_eq!(damaged["lost_minutes"], 1);
    for (path, original) in &fixture.originals {
        assert_eq!(&fs::read(path).expect("read-only source"), original);
    }
}

/// AN NSE CASH STOCK CONSULTS NO PEER, EVEN ONE THAT HOLDS THE SAME STOCK.
///
/// A peer's auction eligibility is not this stock's, so `gaps_json` audits an
/// NSE cash equity with no peer calendar: another vendor's copy of the same
/// stock never votes and is never named. The exchange half of that condition
/// had no test: a mutant that made `asked.exchange == "NSE"` a `!=` sent an
/// NSE stock to the peer vote and survived the whole suite (Gate 18 on
/// PR #19). The same two feeds holding an index do vote, which is the
/// control: the peer here is one the index audit counts.
#[tokio::test]
async fn an_nse_cash_stock_consults_no_peer_even_one_holding_it() {
    let mut fixture = Fixture::named("gap-peer-nse-cash-stock");
    let hole = pull::calendar::OPEN_MINUTE + 10;
    fixture.write(
        Vendor::Dhan,
        Exchange::Nse,
        Segment::Cash,
        "SUBJECT",
        Some(hole),
    );
    fixture.write(Vendor::Groww, Exchange::Nse, Segment::Cash, "SUBJECT", None);
    let uri = "/gaps.json?feed=dhan&exchange=NSE&segment=CASH&symbol=SUBJECT&timeframe=1min&month=2024-01"
        .parse()
        .expect("fixture URI");
    let (status, _, body) =
        gaps_json(axum::extract::State(Loaded::clone(&fixture.site)), uri).await;
    assert_eq!(status, axum::http::StatusCode::OK, "{body}");
    let stock: Value = serde_json::from_str(&body).expect("actual gap response");
    assert_eq!(
        stock["calendar"]["voted_by"],
        serde_json::json!([]),
        "{body}"
    );
    assert_eq!(
        stock["calendar"]["unreadable"],
        serde_json::json!([]),
        "{body}"
    );

    fixture.write(
        Vendor::Dhan,
        Exchange::Nse,
        Segment::Index,
        "SUBJECT",
        Some(hole),
    );
    fixture.write(
        Vendor::Groww,
        Exchange::Nse,
        Segment::Index,
        "SUBJECT",
        None,
    );
    let index = fixture.get().await;
    assert_eq!(
        index["calendar"]["voted_by"],
        serde_json::json!(["groww:SUBJECT"]),
        "the control: the same peer votes for an index"
    );
}

/// **THE DEFAULT DENOMINATOR IS THE CALENDAR OF RECORD, AND NO PEER MOVES
/// IT.** D-3336: coverage is measured against what the calendar owes. The
/// same store whose peer vote names Groww's SUBJECT answers, by default, from
/// the record: nobody voted, and every session the record owes is counted,
/// where the peer vote owed only the one day its peer held.
#[tokio::test]
async fn the_default_denominator_is_the_record_and_no_peer_moves_it() {
    let mut fixture = Fixture::named("gap-peer-default-record");
    let hole = pull::calendar::OPEN_MINUTE + 10;
    fixture.write(
        Vendor::Dhan,
        Exchange::Nse,
        Segment::Index,
        "SUBJECT",
        Some(hole),
    );
    let alone = fixture.get_with("").await;
    fixture.write(
        Vendor::Groww,
        Exchange::Nse,
        Segment::Index,
        "SUBJECT",
        None,
    );
    let beside = fixture.get_with("").await;
    assert_eq!(
        beside, alone,
        "a peer's bars change nothing in the default audit"
    );
    assert_eq!(beside["calendar"]["source"], "record");
    assert_eq!(beside["calendar"]["voted_by"], serde_json::json!([]));
    // THE RECORD OWES EVERY SESSION OF JANUARY 2024: 22, the Saturday the
    // 20th among them. Dhan holds one, short by its one missing minute.
    assert_eq!(beside["expected"], 22 * 375);
    assert_eq!(beside["lost_minutes"], 1 + 21 * 375);
    let voted = fixture.get().await;
    assert_eq!(voted["calendar"]["source"], "peers", "asked for by name");
    assert_eq!(voted["lost_minutes"], 1);
}

// ══ WP-36: the peer vote off the request path (DB-N19) ══════════════════════

impl Fixture {
    /// The audit of Dhan's SUBJECT for January 2024 with `extra` appended,
    /// whatever its status.
    async fn raw(&self, extra: &str) -> (axum::http::StatusCode, Value) {
        let uri = format!(
            "/gaps.json?feed=dhan&exchange=NSE&segment=INDEX&symbol=SUBJECT&timeframe=1min&month=2024-01{extra}"
        )
        .parse()
        .expect("fixture URI");
        let (status, _, body) =
            gaps_json(axum::extract::State(Loaded::clone(&self.site)), uri).await;
        (status, serde_json::from_str(&body).expect("a JSON answer"))
    }

    /// The peer-vote audit, asked again while it is `"building"`, until it is
    /// answered in full.
    #[expect(clippy::panic, reason = "a vote that never finishes fails the test")]
    async fn settled(&self) -> Value {
        for _ in 0..1_000 {
            let (status, answer) = self.raw("&calendar=peers").await;
            if status == axum::http::StatusCode::OK {
                return answer;
            }
            assert_eq!(status, axum::http::StatusCode::ACCEPTED, "{answer}");
            assert_eq!(answer["status"], "building", "{answer}");
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        panic!("the peer vote was still building after 1,000 asks");
    }

    /// Hold the census cache's lock on another thread until the returned
    /// sender is dropped: anything that reads the census blocks meanwhile.
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
}

/// **A PEER VOTE IS BUILT OFF THE REQUEST PATH, KEPT UNDER EVERY FEED'S CENSUS
/// STAMP AND HANDED OUT WITHOUT READING A CENSUS, AND BUILT AGAIN WHEN ANY
/// FEED LANDS.** DB-N19: the first `calendar=peers` audit after a start derived
/// every peer series on the request, 17.9 s for Zerodha NIFTY on a shadow of
/// the operator's store, and every later one walked and agreed them all again.
#[tokio::test]
async fn a_peer_vote_is_built_off_the_request_kept_and_built_again_on_a_landing() {
    let mut fixture = Fixture::named("gap-peer-vote-kept");
    let hole = pull::calendar::OPEN_MINUTE + 10;
    fixture.write(
        Vendor::Dhan,
        Exchange::Nse,
        Segment::Index,
        "SUBJECT",
        Some(hole),
    );
    fixture.write(
        Vendor::Groww,
        Exchange::Nse,
        Segment::Index,
        "SUBJECT",
        None,
    );
    fixture
        .site
        .calendars
        .builds
        .bound_waits(std::time::Duration::from_millis(20));

    // THE CENSUS CANNOT BE READ, so the vote cannot be built: the audit is
    // answered "building" within its budget, and audits nothing.
    let release = fixture.census_held();
    let started = std::time::Instant::now();
    let (status, building) = fixture.raw("&calendar=peers").await;
    assert_eq!(status, axum::http::StatusCode::ACCEPTED, "{building}");
    assert!(started.elapsed() < std::time::Duration::from_secs(2));
    assert_eq!(building["status"], "building");
    assert_eq!(building["calendar"], "peers");
    assert_eq!(building["symbol"], "SUBJECT");
    assert_eq!(building["retry_after_ms"], 250);
    assert!(
        building.get("lost_minutes").is_none(),
        "no minute is audited"
    );
    drop(release);
    let voted = fixture.settled().await;
    assert_eq!(voted["calendar"]["source"], "peers", "{voted}");
    assert_eq!(
        voted["calendar"]["voted_by"],
        serde_json::json!(["groww:SUBJECT"])
    );
    assert_eq!(voted["lost_minutes"], 1);

    // KEPT: with the census lock held again, the next audit is answered in
    // full at once, because the vote is not read again.
    let release = fixture.census_held();
    let (status, again) = fixture.raw("&calendar=peers").await;
    drop(release);
    assert_eq!(status, axum::http::StatusCode::OK, "{again}");
    assert_eq!(again, voted, "the kept vote, the same audit");

    // A LANDING OF ANY FEED moves the stamps the vote is kept under.
    fixture.write(
        Vendor::Zerodha,
        Exchange::Nse,
        Segment::Index,
        "SUBJECT",
        None,
    );
    let landed = fixture.settled().await;
    assert_eq!(
        landed["calendar"]["voted_by"],
        serde_json::json!(["groww:SUBJECT", "zerodha:SUBJECT"]),
        "{landed}"
    );
}

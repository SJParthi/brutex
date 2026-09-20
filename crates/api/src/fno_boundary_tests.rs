#![cfg(test)]
//! Generated named-contract refusals must remain failures on disk and on screen.
#![allow(clippy::expect_used, clippy::indexing_slicing)]

use super::*;
use axum::http::StatusCode;
use brutex_core::instrument::{Exchange, Expiry, Segment};
use pull::manifest::{Closes, Entry, EntryKey, Held};
use std::fs;
use std::sync::Arc;
use std::time::Duration;
use store::format::{Bar, OI_NULL};
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

struct Fixture {
    root: PathBuf,
    site: Arc<Site>,
    wire: Wire,
    asked: ingest::FnoRequest,
    today: Day,
    chain: pull::chain::Chain,
    originals: Vec<(PathBuf, Vec<u8>)>,
}

impl Fixture {
    fn new(count: usize) -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let ordinal = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let root = crate::scratch::path(&format!("fno-budget-boundary-{ordinal}"));
        fs::create_dir(&root).expect("exclusively claim generated root");
        let masters = root.join("masters");
        fs::create_dir(&masters).expect("offline empty masters");
        let site = Arc::new(Site::load(&masters, &root));
        let today = Day::new(2025, 8, 2).expect("closed fixture month");
        let asked = ingest::parse_fno(
            "underlying=NIFTY&series=opt&vendor=groww&from=2025-07-01&to=2025-07-01",
            today,
        )
        .expect("generated named-chain request");
        let shipped = match asked.feed.descriptor().transport {
            pull::vendor::Transport::Http(spec) => Some(spec),
            pull::vendor::Transport::LocalArchive(_) => None,
        }
        .expect("named-chain fixture requires an HTTP descriptor");
        let spec = pull::vendor::HttpSpec {
            // No real credential or vendor connection can enter this fixture.
            base_url: "http://127.0.0.1:0",
            ..shipped
        };
        let source = pull::http::HttpSource::new(
            spec,
            pull::http::Credential::token("generated-offline-fixture".to_owned()),
        )
        .expect("offline transport constructor");
        site.budgets.lock().expect("owned budget table")[asked.feed as usize] = None;
        let expiry = Expiry::new(2025, 7, 31).expect("generated expired contract");
        let chain = pull::chain::Chain {
            expiries: vec!["2025-07-31".to_owned()],
            contracts: (0..count)
                .map(|ordinal| {
                    let strike = 24_000 + ordinal * 50;
                    pull::fno::read_contract(&format!("NSE-NIFTY-31Jul25-{strike}-CE"), expiry)
                        .expect("distinct generated named contract")
                })
                .collect(),
            unreadable: Vec::new(),
        };
        Self {
            root,
            site,
            wire: Wire {
                source,
                spec,
                store_vendor: Vendor::Groww,
            },
            asked,
            today,
            chain,
            originals: Vec::new(),
        }
    }

    fn hold_prefix(&mut self, count: usize) {
        let month = YearMonth::new(2025, 7).expect("fixture month");
        let mut rows = Vec::new();
        for found in self.chain.contracts.iter().take(count) {
            let bar = Bar {
                ts_micros: i64::from(self.asked.window.to().days_from_epoch()) * 86_400_000_000
                    + 36_600_000_000,
                open: 100,
                high: 110,
                low: 90,
                close: 105,
                volume: 1,
                open_interest: OI_NULL,
            };
            let path = StorePath::new(PathParts {
                vendor: Vendor::Groww,
                exchange: "NSE",
                segment: "FNO",
                symbol: "NIFTY",
                contract: Some(found.contract),
                timeframe: Timeframe::MINUTE_1,
                month,
                file: FileKind::Bars,
            })
            .expect("owned contract path");
            let mut writer = store::file::BarFile::open_or_create(&self.root, path, 1)
                .expect("generated bar writer");
            writer.append(&[bar]).expect("generated held row");
            drop(writer);
            let path = path.to_path_buf(&self.root);
            self.originals
                .push((path.clone(), fs::read(path).expect("original bar bytes")));
            rows.push(Held::new(
                Entry {
                    key: EntryKey {
                        contract: Some(found.contract),
                        exchange: Exchange::Nse,
                        segment: Segment::Fno,
                        symbol: self.asked.underlying,
                        timeframe: Timeframe::MINUTE_1,
                        month,
                    },
                    rows: 1,
                    first_ts_micros: bar.ts_micros,
                    last_ts_micros: bar.ts_micros,
                },
                Closes::UNKNOWN,
            ));
        }
        if !rows.is_empty() {
            assert_eq!(
                pull::ingest::record_held(&self.root, Vendor::Groww, &rows),
                None
            );
        }
    }

    async fn serve(&mut self, body: String) -> MockVendor {
        self.serve_replies(vec![(StatusCode::OK, body)], None).await
    }

    async fn serve_replies(
        &mut self,
        replies: Vec<(StatusCode, String)>,
        halt_after: Option<usize>,
    ) -> MockVendor {
        use std::sync::atomic::AtomicUsize;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("owned loopback listener");
        let address = listener.local_addr().expect("loopback address");
        let seen = Arc::new(AtomicUsize::new(0));
        let requests = Arc::clone(&seen);
        let site = Arc::clone(&self.site);
        let feed = self.asked.feed;
        let app = axum::Router::new().fallback(move || {
            let index = requests.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            if halt_after == Some(index + 1) {
                site.budgets.lock().expect("owned fixture budget")[feed as usize] = None;
            }
            let (status, body) = replies
                .get(index)
                .or_else(|| replies.last())
                .expect("nonempty scripted responses")
                .clone();
            async move {
                (
                    status,
                    [(axum::http::header::CONTENT_TYPE, "application/json")],
                    body,
                )
            }
        });
        let task = tokio::spawn(async move {
            axum::serve(listener, app).await.expect("loopback fixture");
        });
        self.wire.spec.base_url = Box::leak(format!("http://{address}").into_boxed_str());
        self.wire.source = pull::http::HttpSource::new(
            self.wire.spec,
            pull::http::Credential::token("generated-offline-fixture".to_owned()),
        )
        .expect("loopback transport");
        *self.site.budgets.lock().expect("owned budgets") = feed_budgets();
        MockVendor { task, seen }
    }

    fn bar_path(&self, index: usize) -> StorePath<'_> {
        StorePath::new(PathParts {
            vendor: self.wire.store_vendor,
            exchange: "NSE",
            segment: "FNO",
            symbol: "NIFTY",
            contract: Some(self.chain.contracts[index].contract),
            timeframe: Timeframe::MINUTE_1,
            month: YearMonth::new(2025, 7).expect("fixture month"),
            file: FileKind::Bars,
        })
        .expect("owned named-contract address")
    }

    fn enable_pricing(&mut self) {
        self.asked = ingest::parse_fno(
            "underlying=NIFTY&series=opt&vendor=groww&from=2025-07-01&to=2025-07-01&rate=0",
            self.today,
        )
        .expect("generated operator-supplied zero rate");
        assert!(self.asked.rate.is_some());
    }

    fn enable_rolling_pricing(&mut self) {
        self.asked = ingest::parse_fno(
            "underlying=NIFTY&series=opt&vendor=dhan&from=2025-07-01&to=2025-07-01&rate=0",
            self.today,
        )
        .expect("generated rolling request with an operator-supplied rate");
        let spec = match self.asked.feed.descriptor().transport {
            pull::vendor::Transport::Http(spec) => Some(spec),
            pull::vendor::Transport::LocalArchive(_) => None,
        }
        .expect("the rolling fixture requires an HTTP descriptor");
        self.wire.spec = pull::vendor::HttpSpec {
            base_url: "http://127.0.0.1:0",
            ..spec
        };
        self.wire.store_vendor = Vendor::Dhan;
    }

    async fn roll(&self) -> Rolled {
        let rolling = self.wire.spec.fno.by_offset().expect("rolling descriptor");
        roll_one(
            &self.asked,
            &self.site,
            &self.wire,
            "13",
            "OPTIDX",
            "MONTH",
            "1",
            "ATM",
            "CALL",
            &format!("{}/owned-rolling", self.wire.spec.base_url),
            rolling,
            self.asked.window,
            self.today,
        )
        .await
        .expect("the fixture's complete rolling answer remains reportable")
    }

    fn prepare_rolling_receipt(&mut self) -> pull::vendor::RollingSpec {
        self.enable_rolling_pricing();
        let masters = self.root.join("masters");
        fs::write(
            masters.join("dhan_scrip.csv"),
            "EXCH_ID,SEGMENT,ISIN,INSTRUMENT,UNDERLYING_SYMBOL,SYMBOL_NAME,\
             INSTRUMENT_TYPE,SERIES,SM_EXPIRY_DATE,STRIKE_PRICE,OPTION_TYPE,SECURITY_ID\n\
             NSE,I,NA,INDEX,NIFTY,NIFTY,INDEX,NA,0001-01-01,,,13\n",
        )
        .expect("owned generated underlying identity");
        self.site = Arc::new(Site::load(&masters, &self.root));
        let original = self.wire.spec.fno.by_offset().expect("rolling descriptor");
        // A generated one-cell descriptor drives the same cross-product walk
        // without spending time repeating unrelated offset/side combinations.
        pull::vendor::RollingSpec {
            index_offsets: &["ATM"],
            stock_offsets: &["ATM"],
            sides: &[("CALL", "ce")],
            expiry_flags: &[("MONTH", pull::vendor::ExpiryCadence::Monthly)],
            expiry_codes: &["1"],
            ..original
        }
    }

    async fn rolling_report(
        &self,
        rolling: pull::vendor::RollingSpec,
    ) -> (StatusCode, String, audit::Record) {
        let journal = self.site.journal();
        let before = journal.look().records();
        let page = FnoPage {
            asked: &self.asked,
            now: std::time::UNIX_EPOCH + Duration::from_mins(29_234_895),
            today: self.today,
            journal: &journal,
            broker: Broker::Live,
        };
        let (status, body) = fno_roll(
            &page,
            Vec::new(),
            &self.asked,
            &self.site,
            &self.wire,
            rolling,
        )
        .await;
        assert_eq!(journal.look().records(), before + 1);
        let record = newest_record(&journal, &journal.look()).expect("durable rolling receipt");
        for (path, original) in &self.originals {
            assert_eq!(fs::read(path).expect("retained fixture bytes"), *original);
        }
        (status, body, record)
    }

    fn symbol_id() -> u32 {
        let hash = brutex_core::universe::fnv1a("NIFTY").to_le_bytes();
        u32::from_le_bytes(hash[..4].try_into().expect("canonical symbol slot"))
    }

    fn hold_spot(&mut self) -> PathBuf {
        let path = StorePath::new(PathParts {
            vendor: Vendor::Groww,
            exchange: "NSE",
            segment: Segment::Index.as_str(),
            symbol: "NIFTY",
            contract: None,
            timeframe: Timeframe::MINUTE_1,
            month: YearMonth::new(2025, 7).expect("generated month"),
            file: FileKind::Bars,
        })
        .expect("generated spot address");
        let first = i64::from(
            Day::new(2025, 7, 1)
                .expect("generated day")
                .days_from_epoch(),
        ) * 86_400_000_000
            + 13_500_000_000;
        let bars: Vec<_> = (0..375)
            .map(|minute| Bar {
                ts_micros: first + minute * 60_000_000,
                open: 2_400_000,
                high: 2_400_000,
                low: 2_400_000,
                close: 2_400_000,
                volume: 0,
                open_interest: OI_NULL,
            })
            .collect();
        let mut writer = store::file::BarFile::open_or_create(&self.root, path, Self::symbol_id())
            .expect("owned generated spot writer");
        writer
            .append(&bars)
            .expect("complete generated spot session");
        drop(writer);
        let path = path.to_path_buf(&self.root);
        self.originals
            .push((path.clone(), fs::read(&path).expect("spot bytes")));
        path
    }

    async fn report(&self) -> (StatusCode, String, audit::Record) {
        let journal = self.site.journal();
        let before = journal.look().records();
        let page = FnoPage {
            asked: &self.asked,
            now: std::time::UNIX_EPOCH + Duration::from_mins(29_234_895),
            today: self.today,
            journal: &journal,
            broker: Broker::Live,
        };
        let (status, body) = fno_report(
            &page,
            Vec::new(),
            &self.chain,
            &self.asked,
            &self.site,
            &self.wire,
        )
        .await;
        let records = journal.look().records();
        assert_eq!(records, before + 1);
        let record = newest_record(&journal, &journal.look()).expect("actual durable receipt");
        for (path, original) in &self.originals {
            assert_eq!(fs::read(path).expect("retained bar bytes"), *original);
        }
        (status, body, record)
    }
}

struct MockVendor {
    task: tokio::task::JoinHandle<()>,
    seen: std::sync::Arc<std::sync::atomic::AtomicUsize>,
}

impl Drop for MockVendor {
    fn drop(&mut self) {
        self.task.abort();
    }
}

fn complete_session() -> String {
    let mut rows = Vec::new();
    // 375 regular-session rows plus ten after-close rows: rows read and
    // bars committed must remain different quantities on the receipt.
    for minute in 9 * 60 + 15..15 * 60 + 40 {
        rows.push(format!(
            r#"["2025-07-01T{:02}:{:02}:00",100,110,90,105,1]"#,
            minute / 60,
            minute % 60
        ));
    }
    format!(r#"{{"payload":{{"candles":[{}]}}}}"#, rows.join(","))
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[tokio::test]
async fn named_chain_budget_halt_counts_the_current_contract_and_unreached_suffix() {
    for (contracts, held) in [(1, 0), (3, 0), (3, 1), (3, 2)] {
        let mut fixture = Fixture::new(contracts);
        fixture.hold_prefix(held);
        let (status, body, record) = fixture.report().await;
        assert_eq!(
            status,
            StatusCode::BAD_GATEWAY,
            "{contracts} contracts, {held} held"
        );
        assert_eq!(record.scope, audit::Scope::Fno);
        assert_eq!(record.outcome, audit::Outcome::Failed);
        assert_eq!(record.members, contracts as u64);
        assert_eq!(record.failures, (contracts - held) as u64);
        assert_eq!(record.rows_read, 0);
        assert_eq!(record.bars_stored, 0);
        assert!(body.contains(&format!("{} of {contracts}", contracts - held)));
        assert!(body.contains("no rate budget"));
        assert!(body.contains(">FAILED<"));
        assert!(!body.contains("every discovered contract fetched and filed"));
    }
}

#[tokio::test]
async fn named_chain_already_held_contracts_do_not_require_an_unused_budget() {
    let mut fixture = Fixture::new(3);
    fixture.hold_prefix(3);
    let (status, body, record) = fixture.report().await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(record.outcome, audit::Outcome::Empty);
    assert_eq!(record.members, 3);
    assert_eq!(record.failures, 0);
    assert_eq!(record.bars_stored, 0);
    assert!(body.contains("already held through its"));
    assert!(body.contains(">STORED NOTHING<"));
    assert!(body.contains("No new bars were stored. Existing stored data is retained."));
    assert!(!body.contains("before the failure"));
    assert!(!body.contains(">NOT STARTED<"));
    assert!(!body.contains("no rate budget"));
}

#[tokio::test]
async fn unreadable_discovery_is_never_an_empty_or_complete_named_chain() {
    for contracts in [0, 3] {
        let mut fixture = Fixture::new(contracts);
        fixture.hold_prefix(contracts);
        fixture
            .chain
            .unreadable
            .push("generated unreadable discovery evidence".to_owned());
        let (status, body, record) = fixture.report().await;
        assert_eq!(
            status,
            StatusCode::BAD_GATEWAY,
            "{contracts} discovered contracts"
        );
        assert_eq!(record.outcome, audit::Outcome::Failed);
        assert_eq!(record.members, contracts as u64);
        assert_eq!(record.failures, 1);
        assert_eq!(record.bars_stored, 0);
        assert!(body.contains("generated unreadable discovery evidence"));
        assert!(body.contains(">FAILED<"));
        assert!(!body.contains("the walk succeeded"));
        assert!(!body.contains("already held through its last owed day"));
    }
}

#[tokio::test]
async fn complete_discovery_of_no_contracts_remains_an_explicit_empty_result() {
    let fixture = Fixture::new(0);
    let (status, body, record) = fixture.report().await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(record.outcome, audit::Outcome::Empty);
    assert_eq!(record.members, 0);
    assert_eq!(record.failures, 0);
    assert_eq!(record.bars_stored, 0);
    assert!(body.contains(">STORED NOTHING<"));
    assert!(body.contains("No new bars were stored. Existing stored data is retained."));
    assert!(body.contains("the walk succeeded"));
    assert!(!body.contains("no rate budget"));
}

#[tokio::test]
async fn discovery_and_fetch_faults_accumulate_without_inventing_missing_contract_counts() {
    let mut fixture = Fixture::new(3);
    fixture.hold_prefix(1);
    fixture.chain.unreadable = vec![
        "generated expiry discovery refusal".to_owned(),
        "generated unreadable contract name".to_owned(),
    ];
    let (status, body, record) = fixture.report().await;
    assert_eq!(status, StatusCode::BAD_GATEWAY);
    assert_eq!(record.outcome, audit::Outcome::Failed);
    assert_eq!(record.members, 3, "only the known contracts are counted");
    assert_eq!(
        record.failures, 4,
        "two discovery faults and two unfetched contracts"
    );
    assert_eq!(record.bars_stored, 0);
    assert!(body.contains("2 of 3"));
    assert!(body.contains("generated expiry discovery refusal"));
    assert!(body.contains("generated unreadable contract name"));
}

#[test]
fn an_unstarted_fno_receipt_retains_its_specific_refusal_reason() {
    let fixture = Fixture::new(0);
    let journal = fixture.site.journal();
    let page = FnoPage {
        asked: &fixture.asked,
        now: std::time::UNIX_EPOCH,
        today: fixture.today,
        journal: &journal,
        broker: Broker::Refused,
    };
    let reason = "generated preflight refusal";
    let (status, body) = page.say(
        Vec::new(),
        StatusCode::SERVICE_UNAVAILABLE,
        audit::Outcome::NotStarted,
        reason,
    );
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert!(body.contains(reason));
    assert!(body.contains(">NOT STARTED<"));
    let record = newest_record(&journal, &journal.look()).expect("durable refusal");
    assert_eq!(record.outcome, audit::Outcome::NotStarted);
    assert_eq!(record.note, reason);
    assert_eq!(record.members, 0);
    assert_eq!(record.failures, 0);
    assert_eq!(record.bars_stored, 0);
}

#[tokio::test]
async fn a_named_pull_reports_committed_bars_even_when_its_census_cannot_publish() {
    let mut fixture = Fixture::new(1);
    let transport = fixture.serve(complete_session()).await;
    let manifest_directory = fixture.root.join("manifest");
    fs::write(&manifest_directory, b"owned obstruction").expect("block only fixture census");
    let (status, body, first) = fixture.report().await;
    assert_eq!(status, StatusCode::BAD_GATEWAY, "{body}");
    assert_eq!(first.outcome, audit::Outcome::Failed);
    assert_eq!(first.failures, 1);
    let path = fixture.bar_path(0);
    let hash = brutex_core::universe::fnv1a("NIFTY").to_le_bytes();
    let symbol = u32::from_le_bytes(hash[..4].try_into().expect("symbol identity"));
    let held = store::file::BarFile::open_existing(&fixture.root, path, symbol)
        .expect("bars reached disk");
    assert_eq!(held.records(), 375);
    assert_eq!(
        first.bars_stored,
        held.records(),
        "receipt must count actual writes"
    );
    assert_eq!(first.rows_read, 385);
    assert!(body.contains("375"), "{body}");
    drop(held);
    let original = fs::read(path.to_path_buf(&fixture.root)).expect("committed bytes");
    let (status, body, retry) = fixture.report().await;
    assert_eq!(status, StatusCode::BAD_GATEWAY, "{body}");
    assert_eq!(retry.outcome, audit::Outcome::Failed);
    assert_eq!(retry.bars_stored, 0, "a replay did not append bars again");
    assert_eq!(retry.rows_read, 385);
    assert_eq!(retry.failures, 1);
    assert_eq!(
        fs::read(path.to_path_buf(&fixture.root)).expect("retained bytes"),
        original
    );
    assert_eq!(
        fs::read(&manifest_directory).expect("obstruction retained"),
        b"owned obstruction"
    );
    assert_eq!(transport.seen.load(std::sync::atomic::Ordering::Relaxed), 2);
}

#[tokio::test]
async fn a_named_pull_reuses_exact_bars_without_claiming_a_second_write() {
    let mut fixture = Fixture::new(1);
    let transport = fixture.serve(complete_session()).await;
    let (status, body, first) = fixture.report().await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(first.outcome, audit::Outcome::Stored);
    assert_eq!(first.bars_stored, 375);
    assert!(
        body.contains("every owed contract window completed"),
        "{body}"
    );
    assert!(
        !body.contains("every discovered contract fetched"),
        "{body}"
    );
    assert_eq!(first.rows_read, 385);
    let path = fixture.bar_path(0).to_path_buf(&fixture.root);
    let original = fs::read(&path).expect("committed bytes");
    let manifest = pull::manifest::manifest_path(&fixture.root, Vendor::Groww);
    fs::remove_file(manifest).expect("hide only the fixture census to force exact replay");
    let (status, body, replay) = fixture.report().await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(replay.outcome, audit::Outcome::Empty);
    assert_eq!(replay.bars_stored, 0);
    assert_eq!(replay.rows_read, 385);
    assert_eq!(replay.failures, 0);
    assert!(body.contains(">STORED NOTHING<"), "{body}");
    assert_eq!(fs::read(path).expect("unchanged bar bytes"), original);
    assert_eq!(transport.seen.load(std::sync::atomic::Ordering::Relaxed), 2);
}

#[tokio::test]
async fn a_later_named_chunk_refusal_preserves_read_counts_without_filing_a_partial_contract() {
    let mut fixture = Fixture::new(1);
    fixture.asked.window = pull::session::Window::new(
        Day::new(2025, 7, 1).expect("from day"),
        Day::new(2025, 7, 2).expect("through day"),
    )
    .expect("two generated days");
    fixture.wire.spec.window_caps = &[(pull::vendor::Granularity::Minute1, 1)];
    let transport = fixture
        .serve_replies(
            vec![
                (StatusCode::OK, complete_session()),
                (
                    StatusCode::BAD_REQUEST,
                    "generated permanent refusal".to_owned(),
                ),
            ],
            None,
        )
        .await;
    let (status, body, record) = fixture.report().await;
    assert_eq!(status, StatusCode::BAD_GATEWAY, "{body}");
    assert_eq!(record.outcome, audit::Outcome::Failed);
    assert_eq!(
        record.rows_read, 385,
        "decoded first chunk remains accounted for"
    );
    assert_eq!(
        record.bars_stored, 0,
        "a partial fetch must not file the contract"
    );
    assert_eq!(record.failures, 1);
    assert!(!fixture.bar_path(0).to_path_buf(&fixture.root).exists());
    assert_eq!(transport.seen.load(std::sync::atomic::Ordering::Relaxed), 2);
    assert!(body.contains("generated permanent refusal"), "{body}");
}

#[tokio::test]
async fn a_named_contract_refusal_does_not_hide_the_following_contracts_committed_bars() {
    for first in [
        (
            StatusCode::BAD_REQUEST,
            "generated permanent refusal".to_owned(),
        ),
        (StatusCode::OK, r#"{"payload":{"candles":[]}}"#.to_owned()),
    ] {
        let mut fixture = Fixture::new(2);
        let transport = fixture
            .serve_replies(vec![first, (StatusCode::OK, complete_session())], None)
            .await;
        let (status, body, record) = fixture.report().await;
        assert_eq!(status, StatusCode::BAD_GATEWAY, "{body}");
        assert_eq!(record.outcome, audit::Outcome::Failed);
        assert_eq!(record.members, 2);
        assert_eq!(record.rows_read, 385);
        assert_eq!(record.bars_stored, 375);
        assert_eq!(record.failures, 1);
        assert!(!fixture.bar_path(0).to_path_buf(&fixture.root).exists());
        assert!(fixture.bar_path(1).to_path_buf(&fixture.root).is_file());
        assert_eq!(transport.seen.load(std::sync::atomic::Ordering::Relaxed), 2);
        assert!(body.contains("1 of 2"), "{body}");
    }
}

#[tokio::test]
async fn a_later_named_budget_halt_retains_answered_rows_and_leaves_the_contract_unfiled() {
    let mut fixture = Fixture::new(1);
    fixture.asked.window = pull::session::Window::new(
        Day::new(2025, 7, 1).expect("from day"),
        Day::new(2025, 7, 2).expect("through day"),
    )
    .expect("two generated days");
    fixture.wire.spec.window_caps = &[(pull::vendor::Granularity::Minute1, 1)];
    let transport = fixture
        .serve_replies(vec![(StatusCode::OK, complete_session())], Some(1))
        .await;
    let (status, body, record) = fixture.report().await;
    assert_eq!(status, StatusCode::BAD_GATEWAY, "{body}");
    assert_eq!(record.outcome, audit::Outcome::Failed);
    assert_eq!(record.rows_read, 385);
    assert_eq!(record.bars_stored, 0);
    assert_eq!(record.failures, 1);
    assert!(!fixture.bar_path(0).to_path_buf(&fixture.root).exists());
    assert_eq!(transport.seen.load(std::sync::atomic::Ordering::Relaxed), 1);
    assert!(body.contains("no rate budget"), "{body}");
}

#[tokio::test]
async fn named_receipts_bound_reasons_without_losing_failed_contract_counts() {
    let mut fixture = Fixture::new(7);
    let transport = fixture
        .serve(r#"{"payload":{"candles":[]}}"#.to_owned())
        .await;
    let (status, body, record) = fixture.report().await;
    assert_eq!(status, StatusCode::BAD_GATEWAY, "{body}");
    assert_eq!(record.outcome, audit::Outcome::Failed);
    assert_eq!(record.members, 7);
    assert_eq!(record.failures, 7);
    assert_eq!(record.rows_read, 0);
    assert_eq!(record.bars_stored, 0);
    for (index, found) in fixture.chain.contracts.iter().enumerate() {
        assert_eq!(
            body.contains(&found.vendor_symbol),
            index < 5,
            "{index}: {body}"
        );
        assert!(!fixture.bar_path(index).to_path_buf(&fixture.root).exists());
    }
    assert_eq!(transport.seen.load(std::sync::atomic::Ordering::Relaxed), 7);
}

#[tokio::test]
async fn mixed_held_and_replayed_contracts_do_not_claim_that_no_fetch_happened() {
    let mut fixture = Fixture::new(2);
    fixture.hold_prefix(1);
    let transport = fixture.serve(complete_session()).await;
    let (status, body, first) = fixture.report().await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(first.outcome, audit::Outcome::Stored);
    assert_eq!(first.bars_stored, 375);
    let path = fixture.bar_path(1).to_path_buf(&fixture.root);
    let original = fs::read(&path).expect("committed second contract");
    fs::remove_file(pull::manifest::manifest_path(&fixture.root, Vendor::Groww))
        .expect("hide only the fixture census");
    fixture.hold_prefix(1);
    let (status, body, replay) = fixture.report().await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(replay.outcome, audit::Outcome::Empty);
    assert_eq!(replay.rows_read, 385);
    assert_eq!(replay.bars_stored, 0);
    assert_eq!(replay.failures, 0);
    assert!(
        body.contains("the fetched bars were already stored byte for byte"),
        "{body}"
    );
    assert!(!body.contains("so no vendor was asked"), "{body}");
    assert_eq!(fs::read(path).expect("unchanged second contract"), original);
    assert_eq!(transport.seen.load(std::sync::atomic::Ordering::Relaxed), 2);
}

#[tokio::test]
async fn a_window_after_every_discovered_expiry_is_empty_without_claiming_a_fetch() {
    for also_held in [false, true] {
        let mut fixture = Fixture::new(usize::from(also_held));
        let day = Day::new(2025, 7, 20).expect("generated requested day");
        fixture.asked.window = pull::session::Window::new(day, day).expect("one closed day");
        fixture.hold_prefix(usize::from(also_held));
        let expiry = Expiry::new(2025, 7, 10).expect("earlier generated expiry");
        fixture.chain.expiries = vec!["2025-07-10".to_owned()];
        if also_held {
            fixture.chain.expiries.push("2025-07-31".to_owned());
        }
        fixture.chain.contracts.push(
            pull::fno::read_contract("NSE-NIFTY-10Jul25-24000-CE", expiry)
                .expect("generated already expired contract"),
        );
        let (status, body, record) = fixture.report().await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(record.outcome, audit::Outcome::Empty);
        assert_eq!(record.members, 1 + u64::from(also_held));
        assert_eq!(record.rows_read, 0);
        assert_eq!(record.bars_stored, 0);
        assert_eq!(record.failures, 0);
        if also_held {
            assert!(body.contains("1, resumed rather than refetched"), "{body}");
            assert!(
                body.contains("every contract-month with bars owed"),
                "{body}"
            );
        } else {
            assert!(
                body.contains("no contract has bars owed within this window"),
                "{body}"
            );
            assert!(!body.contains("Contract-months already held"), "{body}");
        }
        assert!(
            !body.contains("the fetched bars were already stored"),
            "{body}"
        );
        assert!(body.contains("no vendor was asked"), "{body}");
        assert!(
            !fixture
                .bar_path(usize::from(also_held))
                .to_path_buf(&fixture.root)
                .exists()
        );
    }
}

#[tokio::test]
async fn named_pricing_requires_a_complete_landing_even_when_source_bars_committed() {
    for blocked_census in [false, true] {
        let mut fixture = Fixture::new(1);
        fixture.asked = ingest::parse_fno(
            "underlying=NIFTY&series=opt&vendor=groww&from=2025-07-01&to=2025-07-01&rate=0.0655",
            fixture.today,
        )
        .expect("operator-supplied generated pricing input");
        assert!(fixture.asked.rate.is_some());
        let transport = fixture.serve(complete_session()).await;
        let obstruction = fixture.root.join("manifest");
        if blocked_census {
            fs::write(&obstruction, b"owned obstruction").expect("block fixture census");
        }
        let landed = fno_land(
            &fixture.chain.contracts,
            &fixture.asked,
            &fixture.site,
            &fixture.wire,
        )
        .await;
        assert_eq!(landed.rows_read, 385);
        assert_eq!(landed.stored, 375);
        assert_eq!(landed.failed, usize::from(blocked_census));
        if blocked_census {
            assert_eq!(landed.priced, PricedCount::default());
            assert_eq!(
                fs::read(&obstruction).expect("untouched obstruction"),
                b"owned obstruction"
            );
        } else {
            assert_eq!(landed.priced.rows, 0);
            assert!(
                landed
                    .priced
                    .why
                    .iter()
                    .any(|why| why.contains("index month could not be read")),
                "a completed landing must reach pricing and retain its actual missing-input reason: {:?}",
                landed.priced
            );
        }
        assert!(fixture.bar_path(0).to_path_buf(&fixture.root).is_file());
        assert_eq!(transport.seen.load(std::sync::atomic::Ordering::Relaxed), 1);
    }
}

#[tokio::test]
async fn named_pricing_receipt_preserves_missing_and_corrupt_spot_reasons() {
    for corrupt in [false, true] {
        let mut fixture = Fixture::new(1);
        fixture.enable_pricing();
        if corrupt {
            let path = fixture.hold_spot();
            let damaged = b"owned invalid spot file".to_vec();
            fs::write(&path, &damaged).expect("damage only the owned spot fixture");
            fixture.originals = vec![(path, damaged)];
        }
        let actual_reason = read_month_bars(
            &fixture.site,
            &fixture.wire,
            "NIFTY",
            Segment::Index.as_str(),
            None,
            Timeframe::MINUTE_1,
            YearMonth::new(2025, 7).expect("generated month"),
        )
        .expect_err("the fixture's spot input is absent or corrupt");
        let transport = fixture.serve(complete_session()).await;
        let (status, body, record) = fixture.report().await;
        assert_eq!(status, StatusCode::OK, "the source bars completed: {body}");
        assert_eq!(record.bars_stored, 375);
        assert_eq!(record.rows_read, 385);
        assert_eq!(record.failures, 0);
        assert!(body.contains("Pricing notes"), "{body}");
        assert!(body.contains("index month could not be read"), "{body}");
        assert!(body.contains(&render::escape(&actual_reason)), "{body}");
        assert!(!body.contains("index month is not on disk"), "{body}");
        assert!(
            body.contains("no Greek rows were confirmed written"),
            "{body}"
        );
        assert!(
            !fixture
                .bar_path(0)
                .with_file(FileKind::Greeks)
                .to_path_buf(&fixture.root)
                .exists()
        );
        assert_eq!(transport.seen.load(std::sync::atomic::Ordering::Relaxed), 1);
    }
}

fn rolling_session() -> String {
    rolling_rows(375)
}

fn rolling_rows(count: usize) -> String {
    let first = i64::from(
        Day::new(2025, 7, 1)
            .expect("generated trading day")
            .days_from_epoch(),
    ) * 86_400
        + 13_500;
    let stamps: Vec<_> = (0..count)
        .map(|minute| first + i64::try_from(minute).expect("small generated batch") * 60)
        .collect();
    serde_json::json!({"data": {"ce": {
        "timestamp": stamps,
        "open": vec![100; count], "high": vec![110; count],
        "low": vec![90; count], "close": vec![105; count],
        "volume": vec![1; count], "oi": vec![0; count],
        "iv": vec![0.2; count], "spot": vec![24_000; count], "strike": vec![24_000; count]
    }, "pe": null}})
    .to_string()
}

#[tokio::test]
async fn an_unnameable_rolling_answer_keeps_its_decoded_row_count() {
    let mut fixture = Fixture::new(1);
    let rolling = fixture.prepare_rolling_receipt();
    let mut answer: serde_json::Value =
        serde_json::from_str(&rolling_session()).expect("generated rolling answer");
    let removed = answer["data"]["ce"]
        .as_object_mut()
        .expect("generated side object")
        .remove("strike");
    assert!(removed.is_some());
    let transport = fixture.serve(answer.to_string()).await;
    let (status, body, record) = fixture.rolling_report(rolling).await;
    assert_eq!(status, StatusCode::BAD_GATEWAY, "{body}");
    assert_eq!(record.outcome, audit::Outcome::Failed);
    assert_eq!(
        record.rows_read, 375,
        "a missing contract key is after decoding"
    );
    assert_eq!(record.bars_stored, 0);
    assert_eq!(record.failures, 1);
    assert!(body.contains("the vendor sent no strike"), "{body}");
    assert!(!fixture.root.join("bars").exists());
    assert_eq!(transport.seen.load(std::sync::atomic::Ordering::Relaxed), 1);
}

#[tokio::test]
async fn rolling_plan_failure_is_explicit_before_any_request() {
    let mut fixture = Fixture::new(1);
    let rolling = pull::vendor::RollingSpec {
        max_days_per_call: 0,
        ..fixture.prepare_rolling_receipt()
    };
    let transport = fixture.serve(rolling_session()).await;
    let (status, body, record) = fixture.rolling_report(rolling).await;
    assert_eq!(status, StatusCode::BAD_GATEWAY, "{body}");
    assert_eq!(record.outcome, audit::Outcome::Failed);
    assert_eq!(record.rows_read, 0);
    assert_eq!(record.bars_stored, 0);
    assert_eq!(record.failures, 1);
    assert!(body.contains("the window could not be split"), "{body}");
    assert!(body.contains("no rolling request was sent"), "{body}");
    assert_eq!(transport.seen.load(std::sync::atomic::Ordering::Relaxed), 0);
    assert!(!fixture.root.join("bars").exists());
}

#[tokio::test]
async fn rolling_reason_limits_do_not_truncate_failures_or_committed_counts() {
    let mut fixture = Fixture::new(1);
    let rolling = pull::vendor::RollingSpec {
        index_offsets: &["ATM", "ATM+1", "ATM+2", "ATM+3", "ATM+4", "ATM+5", "ATM+6"],
        ..fixture.prepare_rolling_receipt()
    };
    fixture.asked.rate = None;
    let obstruction = fixture.root.join("manifest");
    fs::write(&obstruction, b"owned census obstruction").expect("block only owned census");
    let transport = fixture.serve(rolling_session()).await;
    let (status, body, record) = fixture.rolling_report(rolling).await;
    assert_eq!(status, StatusCode::BAD_GATEWAY, "{body}");
    assert_eq!(record.outcome, audit::Outcome::Failed);
    assert_eq!(record.rows_read, 7 * 375);
    assert_eq!(record.bars_stored, 375, "six exact replies append nothing");
    assert_eq!(record.failures, 7);
    assert_eq!(body.matches("OPTIDX MONTH/1 ").count(), 5, "{body}");
    assert!(!body.contains("OPTIDX MONTH/1 ATM+5"), "{body}");
    assert!(!body.contains("OPTIDX MONTH/1 ATM+6"), "{body}");
    assert_eq!(transport.seen.load(std::sync::atomic::Ordering::Relaxed), 7);
    assert_eq!(
        fs::read(obstruction).expect("unchanged obstruction"),
        b"owned census obstruction"
    );
}

#[tokio::test]
async fn rolling_receipt_keeps_decoded_rows_distinct_from_committed_bars() {
    let mut fixture = Fixture::new(1);
    let rolling = fixture.prepare_rolling_receipt();
    fixture.asked.rate = None;
    let transport = fixture.serve(rolling_rows(385)).await;
    let (status, body, record) = fixture.rolling_report(rolling).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(record.bars_stored, 375);
    assert_eq!(record.rows_read, 385, "after-close rows were decoded too");
    assert_eq!(record.failures, 0);
    assert_eq!(transport.seen.load(std::sync::atomic::Ordering::Relaxed), 1);
}

#[tokio::test]
async fn rolling_receipt_does_not_claim_an_exact_replay_as_new_source_writes() {
    let mut fixture = Fixture::new(1);
    let rolling = fixture.prepare_rolling_receipt();
    let transport = fixture.serve(rolling_session()).await;
    let (status, body, record) = fixture.rolling_report(rolling).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(record.bars_stored, 375);
    for kind in [FileKind::Bars, FileKind::Overlay, FileKind::Greeks] {
        let path = fixture
            .bar_path(0)
            .with_file(kind)
            .to_path_buf(&fixture.root);
        fixture
            .originals
            .push((path.clone(), fs::read(path).expect("first committed bytes")));
    }
    let (status, body, record) = fixture.rolling_report(rolling).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(record.bars_stored, 0, "the real bar file was unchanged");
    assert_eq!(record.rows_read, 375);
    assert_eq!(record.outcome, audit::Outcome::Empty);
    assert!(body.contains("no new source bars were committed"), "{body}");
    assert_eq!(transport.seen.load(std::sync::atomic::Ordering::Relaxed), 2);
}

#[tokio::test]
async fn rolling_receipt_retains_committed_bars_after_a_census_failure() {
    let mut fixture = Fixture::new(1);
    let rolling = fixture.prepare_rolling_receipt();
    fixture.asked.rate = None;
    let obstruction = fixture.root.join("manifest");
    fs::write(&obstruction, b"owned census obstruction").expect("block only owned census");
    let transport = fixture.serve(rolling_session()).await;
    let (status, body, record) = fixture.rolling_report(rolling).await;
    assert_eq!(status, StatusCode::BAD_GATEWAY, "{body}");
    let file = store::file::BarFile::open_existing(
        &fixture.root,
        fixture.bar_path(0),
        Fixture::symbol_id(),
    )
    .expect("the source append really happened");
    assert_eq!(file.records(), 375);
    assert_eq!(
        record.bars_stored, 375,
        "the later census cannot erase the append"
    );
    assert_eq!(record.rows_read, 375);
    assert!(record.failures > 0);
    assert_eq!(
        fs::read(obstruction).expect("unchanged obstruction"),
        b"owned census obstruction"
    );
    assert_eq!(transport.seen.load(std::sync::atomic::Ordering::Relaxed), 1);
}

#[tokio::test]
async fn rolling_greeks_only_describe_bars_the_source_session_accepts() {
    let mut fixture = Fixture::new(1);
    let rolling = fixture.prepare_rolling_receipt();
    let transport = fixture.serve(rolling_rows(385)).await;
    let (status, body, _) = fixture.rolling_report(rolling).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    for kind in [FileKind::Bars, FileKind::Overlay, FileKind::Greeks] {
        let file = store::file::BarFile::open_existing(
            &fixture.root,
            fixture.bar_path(0).with_file(kind),
            Fixture::symbol_id(),
        )
        .expect("the accepted rows are really filed");
        assert_eq!(
            file.records(),
            375,
            "{kind:?} cannot describe an unfiled after-close bar"
        );
    }
    assert!(body.contains("375 confirmed stored"), "{body}");
    assert!(!body.contains("385 confirmed stored"), "{body}");
    assert_eq!(transport.seen.load(std::sync::atomic::Ordering::Relaxed), 1);
}

#[tokio::test]
async fn rolling_pricing_receipt_counts_only_acknowledged_greek_files() {
    let mut combined = PricedCount::default();
    for obstruction in [None, Some(FileKind::Bars), Some(FileKind::Greeks)] {
        let mut fixture = Fixture::new(1);
        fixture.enable_rolling_pricing();
        if let Some(kind) = obstruction {
            fs::create_dir_all(
                fixture
                    .bar_path(0)
                    .with_file(kind)
                    .to_path_buf(&fixture.root),
            )
            .expect("block only an owned fixture file");
        }
        let transport = fixture.serve(rolling_session()).await;
        let done = fixture.roll().await;
        assert_eq!(
            done.priced.rows, 375,
            "every generated quote priced: {done:?}"
        );
        assert_eq!(done.priced.solved, 0, "the fixture supplied volatility");
        assert_eq!(done.failed, usize::from(obstruction.is_some()));
        let facts = greek_facts(&done.priced, false);
        let location = facts
            .iter()
            .find(|(name, _)| *name == "Where the greeks were stored")
            .expect("the receipt states the filing outcome");
        if obstruction.is_none() {
            let file = store::file::BarFile::open_existing(
                &fixture.root,
                fixture.bar_path(0).with_file(FileKind::Greeks),
                Fixture::symbol_id(),
            )
            .expect("the rolling Greek file was really committed");
            assert_eq!(file.records(), 375);
            assert_eq!(done.priced.filed, 375);
            assert!(location.1.contains("375 confirmed stored"), "{facts:?}");
        } else {
            assert_eq!(done.priced.filed, 0);
            assert!(
                location.1.contains("no Greek rows were confirmed written"),
                "{facts:?}"
            );
            assert!(!done.why.is_empty(), "the filing refusal is retained");
            if obstruction == Some(FileKind::Bars) {
                assert!(
                    !fixture
                        .bar_path(0)
                        .with_file(FileKind::Greeks)
                        .to_path_buf(&fixture.root)
                        .exists()
                );
            }
        }
        combined.absorb_count(&done.priced);
        assert_eq!(
            combined.filed, 375,
            "refused groups add no confirmed writes"
        );
        assert_eq!(transport.seen.load(std::sync::atomic::Ordering::Relaxed), 1);
    }
    assert_eq!(combined.rows, 1_125);
    assert_eq!(combined.filed, 375);
}

#[test]
fn greek_filing_refuses_an_unaddressable_rung_without_claiming_a_write() {
    let mut fixture = Fixture::new(1);
    fixture.asked.granularity = pull::vendor::Granularity::Tick;
    let why = file_the_greeks(
        &[],
        fixture.chain.contracts[0].contract,
        &fixture.asked,
        &fixture.site,
        &fixture.wire,
        fixture.asked.window,
    )
    .expect("an unaddressable Greek file must refuse rather than acknowledge");
    assert!(why.contains("no Greek-file timeframe"), "{why}");
    assert!(!fixture.root.join("dhan").exists());
    assert!(!fixture.root.join("groww").exists());
}

#[tokio::test]
async fn named_pricing_receipt_matches_the_greek_file_commit_or_its_refusal() {
    for block_greeks in [false, true] {
        let mut fixture = Fixture::new(1);
        fixture.enable_pricing();
        fixture.hold_spot();
        let greek_path = fixture
            .bar_path(0)
            .with_file(FileKind::Greeks)
            .to_path_buf(&fixture.root);
        if block_greeks {
            fs::create_dir_all(&greek_path).expect("block only the owned Greek file");
        }
        let transport = fixture.serve(complete_session()).await;
        let (status, body, record) = fixture.report().await;
        assert_eq!(status, StatusCode::OK, "the source bars completed: {body}");
        assert_eq!(record.bars_stored, 375);
        assert_eq!(record.failures, 0);
        if block_greeks {
            assert!(greek_path.is_dir());
            assert!(body.contains("Rows that could not be priced"), "{body}");
            assert!(body.contains("could not be filed"), "{body}");
            assert!(
                body.contains("no Greek rows were confirmed written"),
                "{body}"
            );
        } else {
            let file = store::file::BarFile::open_existing(
                &fixture.root,
                fixture.bar_path(0).with_file(FileKind::Greeks),
                Fixture::symbol_id(),
            )
            .expect("actual committed Greek file");
            assert_eq!(file.records(), 375);
            assert!(
                body.contains("375 (375 solved here, 0 sent by the vendor)"),
                "{body}"
            );
            assert!(
                body.contains("375 confirmed stored alongside their option bars in .grk files"),
                "{body}"
            );
            assert!(!body.contains("computed, counted and dropped"), "{body}");
        }
        assert_eq!(transport.seen.load(std::sync::atomic::Ordering::Relaxed), 1);
    }
}

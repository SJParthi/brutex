#![cfg(test)]
//! Generated named-contract refusals must remain failures on disk and on screen.
#![allow(clippy::expect_used, clippy::indexing_slicing)]

use super::*;
use axum::http::StatusCode;
use brutex_core::instrument::{Exchange, Expiry, Segment};
use pull::manifest::{Closes, Entry, EntryKey, Held};
use std::fs;
use std::time::Duration;
use store::format::{Bar, OI_NULL};
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

struct Fixture {
    root: PathBuf,
    site: Site,
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
        let site = Site::load(&masters, &root);
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

    async fn report(&self) -> (StatusCode, String, audit::Record) {
        let journal = self.site.journal();
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
        assert_eq!(records, 1);
        let record = journal.page(records, 0, 1).expect("actual durable receipt")[0]
            .decoded
            .clone()
            .expect("readable durable receipt");
        for (path, original) in &self.originals {
            assert_eq!(fs::read(path).expect("retained bar bytes"), *original);
        }
        (status, body, record)
    }
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
        assert!(!body.contains("every contract-month asked for was already held"));
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

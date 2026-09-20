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
            vendor: Vendor::Groww,
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
                    .any(|why| why.contains("index month is not on disk")),
                "a completed landing must reach pricing and retain its actual missing-input reason: {:?}",
                landed.priced
            );
        }
        assert!(fixture.bar_path(0).to_path_buf(&fixture.root).is_file());
        assert_eq!(transport.seen.load(std::sync::atomic::Ordering::Relaxed), 1);
    }
}

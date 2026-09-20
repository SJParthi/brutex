#![cfg(test)]
//! Calendar votes retain their full exchange and segment identity.
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
}

impl Fixture {
    fn new() -> Self {
        let root = crate::scratch::path("gap-peer-venue-identity");
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
    }

    async fn get(&self) -> Value {
        let uri = "/gaps.json?feed=dhan&exchange=NSE&segment=INDEX&symbol=SUBJECT&timeframe=1min&month=2024-01"
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
    for (path, original) in &fixture.originals {
        assert_eq!(&fs::read(path).expect("read-only source"), original);
    }
}

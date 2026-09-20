#![cfg(test)]
//! Generated finite storage fixtures; these are never presented as market research.
#![allow(clippy::expect_used, clippy::indexing_slicing)]
use super::*;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use store::file::BarFile;
use store::format::Bar;
use store::path::{FileKind, StorePath};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture {
    root: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "brutex-audited-input-fixture-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).expect("scratch");
        let fixture = Self { root };
        for (month, day) in [(4, 30), (5, 2)] {
            let rows = generated_session(month, day);
            assert!(!rows.is_empty());
            fixture.write(month, Timeframe::MINUTE_1, &rows);
            fixture.write(month, Timeframe::DAY_1, &rows[..1]);
            if month == 5 {
                fixture.write(
                    month,
                    Timeframe::MINUTE_5,
                    &rows.iter().step_by(5).copied().collect::<Vec<_>>(),
                );
            }
        }
        fixture
    }
    fn warmed() -> Self {
        let fixture = Self::new();
        for day in 5..=13 {
            let rows = generated_session(5, day);
            if rows.is_empty() {
                continue;
            }
            fixture.write(5, Timeframe::MINUTE_1, &rows);
            fixture.write(5, Timeframe::DAY_1, &rows[..1]);
            fixture.write(
                5,
                Timeframe::MINUTE_5,
                &rows.iter().step_by(5).copied().collect::<Vec<_>>(),
            );
        }
        fixture
    }
    fn path(&self, month: u8, timeframe: Timeframe) -> PathBuf {
        let key = stored::swept_index("NIFTY").expect("key");
        StorePath::for_key(
            Vendor::Zerodha,
            &key,
            timeframe,
            YearMonth::new(2025, month).expect("month"),
            FileKind::Bars,
        )
        .expect("path")
        .to_path_buf(&self.root)
    }
    fn write(&self, month: u8, timeframe: Timeframe, rows: &[Bar]) {
        let key = stored::swept_index("NIFTY").expect("key");
        let path = StorePath::for_key(
            Vendor::Zerodha,
            &key,
            timeframe,
            YearMonth::new(2025, month).expect("month"),
            FileKind::Bars,
        )
        .expect("path");
        let hash = brutex_core::universe::fnv1a("NIFTY").to_le_bytes();
        let symbol = u32::from_le_bytes(hash[..4].try_into().expect("low32"));
        let mut file = BarFile::open_or_create(&self.root, path, symbol).expect("writer");
        file.append(rows).expect("generated fixture rows");
    }
    fn request<'a>(&'a self, rung: &'a str) -> Request<'a> {
        Request {
            store_root: &self.root,
            vendor: Vendor::Zerodha,
            underlying: "NIFTY",
            rung,
            year: 2025,
            month: 5,
            receipt_root: &self.root,
            max_bytes: 1_048_576,
            max_records: 10_000,
        }
    }
}

fn generated_session(month: u8, date: u8) -> Vec<Bar> {
    let civil = pull::session::Day::new(2025, month, date).expect("date");
    let day = i64::from(civil.days_from_epoch());
    let pull::calendar::DayKind::Open(session) = pull::calendar::kind_of(day) else {
        return Vec::new();
    };
    session
        .windows
        .iter()
        .take(usize::from(session.count))
        .flat_map(|window| window.from..=window.to)
        .map(|minute| Bar {
            ts_micros: day * 86_400_000_000 + i64::from(minute) * 60_000_000
                - indicators::IST_OFFSET_MICROS,
            open: 100_000,
            high: 110_000,
            low: 90_000,
            close: 101_000,
            volume: 100,
            open_interest: i64::MIN,
        })
        .collect()
}

#[test]
fn audited_month_publication_is_idempotent_and_stale_inputs_cannot_publish() {
    let _knobs = crate::knobs::serially();
    crate::knobs::clear_all();
    for rung in ["1min", "5min"] {
        let fixture = Fixture::warmed();
        let input = Inputs::load(fixture.request(rung)).expect("audited generated inputs");
        let run = |inputs: &Inputs| {
            crate::stored_month_kernel(
                crate::StoredSweepRequest {
                    root: fixture.root.clone(),
                    vendor: Vendor::Zerodha,
                    underlying: "NIFTY",
                    rung,
                    year: 2025,
                    month: 5,
                    min_hits: u64::MAX,
                    // Explicit generated-test identity, never operator provenance.
                    commit: "generated-audited-month-fixture",
                },
                inputs.data(),
                Some(inputs),
            )
        };
        let first = run(&input).expect("bounded complete generated sweep");
        assert!(first.contains("RESULT RECORDED"));
        assert!(first.contains("six-role input binding"));
        assert!(!first.contains(crate::NOT_RECORDED));
        let path = crate::results::Results::path(&fixture.root);
        let saved = fs::read(&path).expect("published ledger");
        let second = run(&input).expect("exact rerun");
        assert!(second.contains("RESULT ALREADY RECORDED AND VERIFIED"));
        assert_eq!(fs::read(&path).expect("rerun ledger"), saved);
        let mut ledger = crate::results::Results::open_read(&fixture.root).expect("cold ledger");
        assert_eq!(ledger.len().expect("count"), 1);
        let row = ledger.read(0).expect("only acknowledged parent");
        assert_eq!(row.min_hits, u64::MAX);
        assert_eq!(row.trades, 0);
        assert_eq!(row.combinations, 0);
        assert_eq!(row.halted, 0);
        assert_eq!(crate::results::read_field(&row.timeframe), rung);
        drop(ledger);

        let source = fixture.path(5, Timeframe::MINUTE_1);
        let original = fs::read(&source).expect("source bytes");
        let mut changed = original.clone();
        changed[24] ^= 1;
        fs::write(&source, changed).expect("corrupt the generated source");
        assert!(run(&input).is_err());
        assert_eq!(fs::read(&path).expect("refused ledger"), saved);
        fs::write(&source, original).expect("restore the generated source");
        let reloaded = Inputs::load(fixture.request(rung)).expect("fresh source authentication");
        assert!(
            run(&reloaded)
                .expect("restored rerun")
                .contains("RESULT ALREADY RECORDED")
        );
        assert_eq!(fs::read(&path).expect("restored ledger"), saved);
    }
    crate::knobs::clear_all();
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn native_and_coarse_use_exact_audited_rows_and_the_same_calendar_converters() {
    let fixture = Fixture::new();
    for (rung, guards, records) in [("1min", 4, 752), ("5min", 5, 827)] {
        let input = Inputs::load(fixture.request(rung)).expect("strict inputs");
        assert_eq!(input.guards.len(), guards);
        assert_eq!(input.records, records);
        let ordinary = stored::load(&fixture.root, Vendor::Zerodha, "NIFTY", rung, 2025, 5)
            .expect("ordinary decoded fixture");
        assert_eq!(input.data.loaded.bars, ordinary.bars);
        let daily = stored::load_daily_context(
            &fixture.root,
            Vendor::Zerodha,
            "NIFTY",
            ((2025, 5), (2025, 5)),
            &ordinary.bars,
        )
        .expect("same daily converter");
        let minute = stored::load_exact_minute_context(
            &fixture.root,
            Vendor::Zerodha,
            "NIFTY",
            ((2025, 5), (2025, 5)),
            &ordinary.bars,
        )
        .expect("same minute converter");
        assert_eq!(input.data.daily.bars, daily.bars);
        assert_eq!(input.data.exact_minute.bars, minute.bars);
        assert_eq!(input.data.execution_bars.is_some(), rung == "5min");
        assert_ne!(input.bind_digest([1; 32]), [1; 32]);
        assert_ne!(input.bind_digest([1; 32]), input.bind_digest([2; 32]));
        assert_eq!(input.roles[1], input.roles[5]);
        assert_eq!(input.roles[0] == input.roles[1], rung == "1min");
        assert!(input.note().contains("six-role input binding"));
        input.require_current().expect("all live guards");
    }
}

#[test]
fn every_context_source_and_the_saved_role_binding_remain_required_after_loading() {
    for (month, timeframe) in [
        (5, Timeframe::MINUTE_5),
        (5, Timeframe::MINUTE_1),
        (4, Timeframe::MINUTE_1),
        (4, Timeframe::DAY_1),
        (5, Timeframe::DAY_1),
    ] {
        let fixture = Fixture::new();
        let input = Inputs::load(fixture.request("5min")).expect("strict inputs");
        let path = fixture.path(month, timeframe);
        let bytes = fs::read(&path).expect("exact source");
        fs::remove_file(&path).expect("replace scratch source");
        fs::write(&path, bytes).expect("same bytes new inode");
        assert!(input.require_current().is_err());
    }
    let fixture = Fixture::new();
    let input = Inputs::load(fixture.request("1min")).expect("strict inputs");
    let path = fixture.root.join("audited-inputs-v1").join(format!(
        "{}.bin",
        crate::identity_hex(&input.binding_identity)
    ));
    let mut bytes = fs::read(&path).expect("binding");
    assert_eq!(bytes.len(), 512);
    assert_eq!(&bytes[..8], b"BRHIN001");
    bytes[24] ^= 1;
    fs::write(path, bytes).expect("corrupt scratch relationship");
    assert!(input.require_current().is_err());
}

#[test]
fn strict_input_caps_and_missing_prior_context_refuse_without_fallback() {
    let fixture = Fixture::new();
    for cap in [0, 1, 374, 751] {
        let mut request = fixture.request("1min");
        request.max_records = cap;
        assert!(Inputs::load(request).is_err());
    }
    let mut request = fixture.request("1min");
    request.max_records = 752;
    Inputs::load(request).expect("exact raw-source cap");
    let mut request = fixture.request("1min");
    request.max_bytes = 1;
    assert!(Inputs::load(request).is_err());
    fs::remove_file(fixture.path(4, Timeframe::DAY_1)).expect("remove required prior daily source");
    assert!(Inputs::load(fixture.request("1min")).is_err());
}

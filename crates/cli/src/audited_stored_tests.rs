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

pub(crate) fn with_warmed_store<R>(run: impl FnOnce(&std::path::Path) -> R) -> R {
    let fixture = Fixture::warmed();
    run(&fixture.root)
}

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

    fn audit(&self, rung: &str) -> Result<String, String> {
        crate::audit_stored_kernel(crate::StoredSweepRequest {
            root: self.root.clone(),
            vendor: Vendor::Zerodha,
            underlying: "NIFTY",
            rung,
            year: 2025,
            month: 5,
            min_hits: u64::MAX,
            commit: "generated-stored-audit-fixture",
        })
    }

    fn audit_range(&self, rung: &str, to: (u16, u8)) -> Result<String, String> {
        crate::audit_range_kernel(crate::StoredRangeAuditRequest {
            root: self.root.clone(),
            vendor: Vendor::Zerodha,
            underlying: "NIFTY",
            rung,
            from: (2025, 5),
            to,
            min_hits: u64::MAX,
            attempt: Some(7),
            commit: "generated-stored-range-audit-fixture",
        })
    }

    fn screen(&self, rung: &str, support_ppm: u64) -> Result<String, String> {
        crate::screen_range_kernel(crate::StoredScreenRequest {
            root: self.root.clone(),
            vendor: Vendor::Zerodha,
            underlying: "NIFTY",
            rung,
            span: ((2025, 5), (2025, 5)),
            support_ppm,
            policy: crate::Policy {
                rules: crate::Rules::BASELINE,
                lens: runner::rank::Lens::Detectability,
                validate: false,
            },
            attempt: Some(13),
            commit: "generated-stored-screen-fixture",
        })
    }

    fn omit_owned_minutes(&self, holed_days: &[u8]) {
        let path = self.path(5, Timeframe::MINUTE_1);
        fs::remove_file(&path).expect("replace owned generated minute file");
        fs::remove_file(path.with_extension("crc")).expect("replace its owned proof");
        for day in [2, 5, 6, 7, 8, 9, 12, 13] {
            let mut rows = generated_session(5, day);
            if holed_days.contains(&day) {
                rows.remove(150);
            }
            self.write(5, Timeframe::MINUTE_1, &rows);
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
fn stored_screens_scale_support_to_retained_bars_and_disclose_holed_sessions() {
    let _knobs = crate::knobs::serially();
    crate::knobs::clear_all();
    // An explicit finite operator budget also keeps a generated constant-price
    // vocabulary from consuming a machine if new always-true bits are added.
    crate::knobs::set("BRUTEX_CEILING", "256");
    for (rung, retained, withheld) in [("1min", 2_625, 374), ("5min", 525, 75)] {
        let fixture = Fixture::warmed();
        fixture.omit_owned_minutes(&[5]);
        let source = fixture.path(5, Timeframe::MINUTE_1);
        let source_bytes = fs::read(&source).expect("owned incomplete minute source");
        let report = fixture.screen(rung, 1_000_000).expect("loaded screen");
        assert!(report.contains("RESULT RECORDED"), "{report}");
        let mut ledger = crate::results::Results::open_read(&fixture.root).expect("screen ledger");
        assert_eq!(ledger.len().expect("parent count"), 1);
        let row = ledger.read(0).expect("screen parent");
        assert_eq!(row.bars, retained);
        assert_eq!(
            row.min_hits, retained,
            "100% support must use the retained sample"
        );
        assert_eq!(crate::results::read_field(&row.timeframe), rung);
        assert!(report.contains("MINUTE-GAP SESSIONS WITHHELD"), "{report}");
        assert!(report.contains("2025-05-05"), "{report}");
        assert!(
            report.contains(&format!("{withheld} signal bar(s)")),
            "{report}"
        );
        assert_eq!(fs::read(&source).expect("unmodified source"), source_bytes);
    }
    crate::knobs::clear_all();
}

#[test]
fn stored_screen_support_and_exact_retries_bind_the_actual_sample() {
    let _knobs = crate::knobs::serially();
    crate::knobs::clear_all();
    crate::knobs::set("BRUTEX_CEILING", "256");
    for (rung, bars) in [("1min", 3_000), ("5min", 600)] {
        let fixture = Fixture::warmed();
        let mut identities = Vec::new();
        for (index, support) in [20_000, 50_000, 1_000_000].into_iter().enumerate() {
            let report = fixture.screen(rung, support).expect("screen request");
            assert!(report.contains("RESULT RECORDED"), "{report}");
            assert!(!report.contains("MINUTE-GAP SESSIONS WITHHELD"), "{report}");
            let path = crate::results::Results::path(&fixture.root);
            let saved = fs::read(&path).expect("parent ledger");
            let mut ledger =
                crate::results::Results::open_read(&fixture.root).expect("screen ledger");
            assert_eq!(
                ledger.len().expect("distinct support count"),
                index as u64 + 1
            );
            let row = ledger.read(index as u64).expect("exact screen parent");
            assert_eq!(row.bars, bars);
            assert_eq!(row.min_hits, bars * support / 1_000_000);
            assert_eq!((row.months_asked, row.months_found), (1, 1));
            assert!(!identities.contains(&row.identity));
            identities.push(row.identity);
            drop(ledger);
            let retry = fixture.screen(rung, support).expect("same screen again");
            assert!(retry.contains("RESULT ALREADY RECORDED"), "{retry}");
            assert_eq!(fs::read(&path).expect("retry bytes"), saved);
            let attempt = crate::sweep_evidence::latest(&fixture.root, 1_048_576)
                .expect("read durable attempt")
                .expect("screen attempt exists");
            assert_eq!(attempt.identity, row.identity);
            assert_eq!(
                attempt.completion == crate::sweep_evidence::Completion::Completed,
                row.halted == 0
            );
        }
    }
    crate::knobs::clear_all();
}

#[test]
fn stored_screens_refuse_missing_or_corrupt_authorities_before_recording() {
    let _knobs = crate::knobs::serially();
    crate::knobs::clear_all();
    for (month, timeframe, corrupt) in [
        (5, Timeframe::MINUTE_1, false),
        (4, Timeframe::DAY_1, false),
        (4, Timeframe::MINUTE_1, false),
        (5, Timeframe::MINUTE_5, true),
    ] {
        let fixture = Fixture::warmed();
        let path = fixture.path(month, timeframe);
        let original = fs::read(&path).expect("owned authority");
        let damaged = if corrupt {
            let mut bytes = original;
            bytes[24] ^= 1;
            fs::write(&path, &bytes).expect("damage owned authority");
            Some(bytes)
        } else {
            fs::remove_file(&path).expect("remove owned required source");
            None
        };
        let refusal = fixture
            .screen("5min", 1_000_000)
            .expect_err("source must refuse");
        assert!(!refusal.is_empty());
        assert!(!crate::results::Results::path(&fixture.root).exists());
        assert_eq!(
            crate::sweep_evidence::latest(&fixture.root, 1_048_576).expect("no premature attempt"),
            None
        );
        assert_eq!(
            fs::read(&path).ok(),
            damaged,
            "no source repair or creation"
        );
    }
    crate::knobs::clear_all();
}

#[test]
fn stored_screens_refuse_when_every_signal_session_is_withheld() {
    let _knobs = crate::knobs::serially();
    crate::knobs::clear_all();
    let fixture = Fixture::warmed();
    fixture.omit_owned_minutes(&[2, 5, 6, 7, 8, 9, 12, 13]);
    for rung in ["1min", "5min"] {
        let refusal = fixture
            .screen(rung, 20_000)
            .expect_err("no retained session");
        assert_eq!(
            refusal,
            "every signal session has a minute gap; no screenable bars remain"
        );
        assert!(!crate::results::Results::path(&fixture.root).exists());
        assert_eq!(
            crate::sweep_evidence::latest(&fixture.root, 1_048_576).expect("no empty attempt"),
            None
        );
    }
    crate::knobs::clear_all();
}

#[test]
fn public_screen_admission_preserves_rung_and_build_or_feed_refusals() {
    let policy = crate::Policy {
        rules: crate::Rules::BASELINE,
        lens: runner::rank::Lens::Detectability,
        validate: false,
    };
    for rung in ["", "2min", "1day", "1min"] {
        let plain = crate::screen_range(
            "unknown-generated-feed",
            "NIFTY",
            rung,
            (2025, 5),
            (2025, 5),
            20_000,
            policy,
        );
        let attempted = crate::screen_range_for_attempt(
            "unknown-generated-feed",
            "NIFTY",
            rung,
            ((2025, 5), (2025, 5)),
            20_000,
            policy,
            Some(13),
        );
        assert_eq!(plain, attempted);
        assert!(plain.starts_with("refused: "), "{plain}");
        assert!(!plain.contains(crate::STORED_PROVENANCE), "{plain}");
        assert!(!plain.contains("RESULT RECORDED"), "{plain}");
        if rung == "1min" && crate::commit_stamp().is_none() {
            assert!(plain.contains("no verified commit stamp"), "{plain}");
        }
    }
}

#[test]
fn monthly_audits_publish_empty_extinction_and_exact_retry_identity_at_both_resolutions() {
    let _knobs = crate::knobs::serially();
    crate::knobs::clear_all();
    for rung in ["1min", "5min"] {
        let fixture = Fixture::warmed();
        let report = fixture.audit(rung).expect("actual stored audit");
        assert!(report.starts_with(crate::STORED_PROVENANCE), "{report}");
        assert!(report.contains("generated-stored-audit-fixture"));
        assert!(report.contains("RESULT RECORDED"), "{report}");
        assert!(!report.contains(crate::NOT_RECORDED), "{report}");
        assert!(!report.contains("NOTHING MEASURED"), "{report}");
        let path = crate::results::Results::path(&fixture.root);
        let original = fs::read(&path).expect("actual parent ledger");
        let mut ledger = crate::results::Results::open_read(&fixture.root).expect("cold ledger");
        assert_eq!(ledger.len().expect("parent count"), 1);
        let row = ledger.read(0).expect("saved audit parent");
        assert!(row.bars > 0);
        assert_eq!((row.trades, row.combinations, row.halted), (0, 0, 0));
        assert_eq!(row.min_hits, u64::MAX);
        assert_eq!(crate::results::read_field(&row.timeframe), rung);
        drop(ledger);
        let first = crate::sweep_evidence::latest(&fixture.root, 1_048_576)
            .expect("read first audit evidence")
            .expect("durable audit attempt");
        assert_eq!(first.identity, row.identity);
        assert_eq!(
            first.completion,
            crate::sweep_evidence::Completion::Completed
        );

        let retry = fixture.audit(rung).expect("exact audit retry");
        assert!(
            retry.contains("RESULT ALREADY RECORDED AND VERIFIED"),
            "{retry}"
        );
        assert_eq!(fs::read(&path).expect("retry ledger"), original);
        let second = crate::sweep_evidence::latest(&fixture.root, 1_048_576)
            .expect("read retry evidence")
            .expect("durable retry");
        assert!(second.attempt > first.attempt);
        assert_eq!(second.identity, first.identity);
        assert_eq!(
            second.completion,
            crate::sweep_evidence::Completion::Completed
        );

        let source = fixture.path(5, Timeframe::MINUTE_1);
        let saved = fs::read(&source).expect("original minutes");
        let mut corrupt = saved.clone();
        corrupt[24] ^= 1;
        fs::write(&source, corrupt).expect("corrupt owned minute authority");
        assert!(fixture.audit(rung).is_err());
        assert_eq!(fs::read(&path).expect("refused ledger"), original);
        fs::write(&source, saved).expect("restore owned authority");
        assert!(
            fixture
                .audit(rung)
                .expect("restored audit")
                .contains("RESULT ALREADY RECORDED")
        );
        assert_eq!(fs::read(&path).expect("restored ledger"), original);
    }
    crate::knobs::clear_all();
}

#[test]
fn monthly_audit_missing_execution_or_prior_context_refuses_before_an_attempt() {
    for (month, timeframe) in [(5, Timeframe::MINUTE_1), (4, Timeframe::DAY_1)] {
        let fixture = Fixture::warmed();
        fs::remove_file(fixture.path(month, timeframe)).expect("remove owned required context");
        let refusal = fixture
            .audit("5min")
            .expect_err("required context is absent");
        assert!(!refusal.is_empty());
        if timeframe == Timeframe::MINUTE_1 {
            assert!(refusal.contains("1min execution series"), "{refusal}");
            assert!(refusal.contains("no coarse fallback"), "{refusal}");
        }
        assert!(fixture.audit_range("5min", (2025, 5)).is_err());
        assert!(!crate::results::Results::path(&fixture.root).exists());
        assert_eq!(
            crate::sweep_evidence::latest(&fixture.root, 1_048_576).expect("no attempt ledger"),
            None
        );
    }
}

#[test]
fn range_audits_record_exact_requested_and_found_months_without_inventing_missing_bars() {
    let _knobs = crate::knobs::serially();
    crate::knobs::clear_all();
    for (rung, end) in [
        ("1min", (2025, 5)),
        ("5min", (2025, 5)),
        ("5min", (2025, 6)),
    ] {
        let fixture = Fixture::warmed();
        if end.1 == 6 {
            let refusal = fixture
                .audit_range(rung, end)
                .expect_err("required June context is absent");
            assert!(refusal.contains("1day reference stream is incomplete: missing 2025-06"));
            assert!(!crate::results::Results::path(&fixture.root).exists());
            assert_eq!(
                crate::sweep_evidence::latest(&fixture.root, 1_048_576)
                    .expect("no premature attempt"),
                None
            );
            // Supply the mandatory reference and execution authority while
            // leaving June's five-minute signal file absent.
            let context = generated_session(6, 2);
            assert!(!context.is_empty());
            fixture.write(6, Timeframe::MINUTE_1, &context);
            fixture.write(6, Timeframe::DAY_1, &context[..1]);
        }
        let report = fixture
            .audit_range(rung, end)
            .expect("actual stored range audit");
        assert!(report.starts_with(crate::STORED_PROVENANCE), "{report}");
        assert!(report.contains("RESULT RECORDED"), "{report}");
        assert!(!report.contains(crate::NOT_RECORDED), "{report}");
        assert!(!report.contains("NOTHING MEASURED"), "{report}");
        assert_eq!(
            report.contains("MONTHS MISSING FROM THIS SPAN (1): 2025-06"),
            end.1 == 6
        );
        let mut ledger =
            crate::results::Results::open_read(&fixture.root).expect("cold range ledger");
        assert_eq!(ledger.len().expect("one parent"), 1);
        let row = ledger.read(0).expect("recorded range");
        assert_eq!((row.from_year, row.from_month), (2025, 5));
        assert_eq!((row.to_year, row.to_month), end);
        assert_eq!(
            (row.months_asked, row.months_found),
            (u32::from(end.1) - 4, 1)
        );
        assert_eq!(crate::results::read_field(&row.timeframe), rung);
        assert!(row.bars > 0);
        assert_eq!((row.trades, row.combinations, row.halted), (0, 0, 0));
        drop(ledger);
        let first = crate::sweep_evidence::latest(&fixture.root, 1_048_576)
            .expect("read range attempt")
            .expect("range evidence");
        assert_eq!(first.identity, row.identity);
        assert_eq!(
            first.completion,
            crate::sweep_evidence::Completion::Completed
        );
        let path = crate::results::Results::path(&fixture.root);
        let saved = fs::read(&path).expect("saved parent bytes");
        let retry = fixture.audit_range(rung, end).expect("range retry");
        assert!(
            retry.contains("RESULT ALREADY RECORDED AND VERIFIED"),
            "{retry}"
        );
        assert_eq!(fs::read(path).expect("retry parent bytes"), saved);
        let second = crate::sweep_evidence::latest(&fixture.root, 1_048_576)
            .expect("read retry")
            .expect("range retry evidence");
        assert!(second.attempt > first.attempt);
        assert_eq!(second.identity, first.identity);
        assert_eq!(
            second.completion,
            crate::sweep_evidence::Completion::Completed
        );
    }
    crate::knobs::clear_all();
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

//! Finite generated fixtures for retained multi-month checksum authority.
#![allow(clippy::expect_used, clippy::indexing_slicing)]
use super::*;
use std::fs::{self, File};
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
        Self::with_sessions(false)
    }
    fn warmed() -> Self {
        Self::with_sessions(true)
    }
    fn with_sessions(warmed: bool) -> Self {
        let root = std::env::temp_dir().join(format!(
            "brutex-audited-range-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).expect("unique generated fixture");
        let result = Self { root };
        let may: &[u8] = if warmed {
            &[2, 5, 6, 7, 8, 30]
        } else {
            &[2, 30]
        };
        for (month, days) in [(4, &[30][..]), (5, may), (6, &[2][..])] {
            let mut minutes = Vec::new();
            let mut daily = Vec::new();
            for &day in days {
                let rows = session_rows(month, day);
                daily.push(aggregate(&rows));
                minutes.extend(rows);
            }
            result.write(month, Timeframe::MINUTE_1, &minutes);
            result.write(month, Timeframe::DAY_1, &daily);
            if month != 4 {
                result.write(
                    month,
                    Timeframe::MINUTE_5,
                    &minutes.chunks(5).map(aggregate).collect::<Vec<_>>(),
                );
            }
        }
        result
    }
    fn path(&self, month: u8, timeframe: Timeframe, kind: FileKind) -> PathBuf {
        let key = stored::swept_index("NIFTY").expect("fixture key");
        StorePath::for_key(
            Vendor::Zerodha,
            &key,
            timeframe,
            YearMonth::new(2025, month).expect("month"),
            kind,
        )
        .expect("source path")
        .to_path_buf(&self.root)
    }
    fn write(&self, month: u8, timeframe: Timeframe, rows: &[Bar]) {
        let key = stored::swept_index("NIFTY").expect("fixture key");
        let path = StorePath::for_key(
            Vendor::Zerodha,
            &key,
            timeframe,
            YearMonth::new(2025, month).expect("month"),
            FileKind::Bars,
        )
        .expect("source path");
        let hash = brutex_core::universe::fnv1a("NIFTY").to_le_bytes();
        let symbol = u32::from_le_bytes(hash[..4].try_into().expect("low32"));
        let mut file = BarFile::open_or_create(&self.root, path, symbol).expect("fixture writer");
        file.append(rows).expect("exact generated records");
    }
    fn request<'a>(&'a self, rung: &'a str) -> RangeRequest<'a> {
        RangeRequest {
            store_root: &self.root,
            vendor: Vendor::Zerodha,
            underlying: "NIFTY",
            rung,
            from: (2025, 5),
            to: (2025, 6),
            receipt_root: &self.root,
            max_bytes: 1_048_576,
            max_records: 10_000,
        }
    }
}

fn aggregate(rows: &[Bar]) -> Bar {
    let first = rows.first().expect("nonempty generated aggregation");
    Bar {
        ts_micros: first.ts_micros,
        open: first.open,
        high: rows.iter().map(|row| row.high).max().expect("high"),
        low: rows.iter().map(|row| row.low).min().expect("low"),
        close: rows.last().expect("close").close,
        volume: rows.iter().map(|row| row.volume).sum(),
        open_interest: i64::MIN,
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn session_rows(month: u8, day: u8) -> Vec<Bar> {
    let day = i64::from(
        pull::session::Day::new(2025, month, day)
            .expect("date")
            .days_from_epoch(),
    );
    let session = match pull::calendar::kind_of(day) {
        pull::calendar::DayKind::Open(session) => Some(session),
        _ => None,
    }
    .expect("fixture must be canonical open day");
    session
        .windows
        .iter()
        .take(usize::from(session.count))
        .flat_map(|window| window.from..=window.to)
        .map(|minute| {
            let open = 100_000 + i64::from(minute % 31) * 100;
            Bar {
                ts_micros: day * 86_400_000_000 + i64::from(minute) * 60_000_000
                    - indicators::IST_OFFSET_MICROS,
                open,
                high: open + 1000,
                low: open - 900,
                close: open + 50,
                volume: 100,
                open_interest: i64::MIN,
            }
        })
        .collect()
}

#[test]
fn native_and_coarse_spans_match_shared_decoders_without_duplicate_source_charges() {
    let fixture = Fixture::new();
    for (rung, sources, records, bars) in [("1min", 6, 1504, 1125), ("5min", 8, 1729, 225)] {
        let (data, guard) = RangeInputs::load(fixture.request(rung))
            .expect("strict range")
            .into_parts();
        let ordinary = stored::load_span(
            &fixture.root,
            Vendor::Zerodha,
            "NIFTY",
            rung,
            (2025, 5),
            (2025, 6),
        )
        .expect("ordinary range decoder");
        assert_eq!(data.signal.bars, ordinary.bars);
        assert_eq!(data.signal.bars.len(), bars);
        assert_eq!((data.signal.asked, data.signal.found), (2, 2));
        assert!(data.signal.complete());
        let daily = stored::load_daily_context(
            &fixture.root,
            Vendor::Zerodha,
            "NIFTY",
            ((2025, 5), (2025, 6)),
            &ordinary.bars,
        )
        .expect("same causal daily conversion");
        let minute = stored::load_exact_minute_context(
            &fixture.root,
            Vendor::Zerodha,
            "NIFTY",
            ((2025, 5), (2025, 6)),
            &ordinary.bars,
        )
        .expect("same exact-minute conversion");
        assert_eq!(data.daily.bars, daily.bars);
        assert_eq!(data.exact_minute.bars, minute.bars);
        assert_eq!(data.execution.is_some(), rung == "5min");
        if let Some(execution) = data.execution {
            assert_eq!(execution.bars.len(), 1125);
        }
        assert_eq!(guard.sources.len(), sources);
        assert_eq!(guard.records, records);
        assert_eq!(guard.bindings.len(), 2);
        assert_ne!(guard.bind_digest([1; 32]), guard.bind_digest([2; 32]));
        assert!(guard.note().contains("2 chronological signal months"));
        guard.require_current().expect("retained authority");
    }
}

#[test]
fn each_unique_source_is_required_even_after_decoded_bars_are_moved() {
    for (month, timeframe) in [
        (4, Timeframe::MINUTE_1),
        (4, Timeframe::DAY_1),
        (5, Timeframe::MINUTE_1),
        (5, Timeframe::DAY_1),
        (5, Timeframe::MINUTE_5),
        (6, Timeframe::MINUTE_1),
        (6, Timeframe::DAY_1),
        (6, Timeframe::MINUTE_5),
    ] {
        let fixture = Fixture::new();
        let (data, guard) = RangeInputs::load(fixture.request("5min"))
            .expect("strict range")
            .into_parts();
        assert_eq!(data.signal.bars.len(), 225);
        let lock = File::open(fixture.path(month, timeframe, FileKind::Lock)).expect("source lock");
        assert!(
            lock.try_lock().is_err(),
            "moving data must retain original shared source locks"
        );
        let path = fixture.path(month, timeframe, FileKind::Bars);
        let bytes = fs::read(&path).expect("exact original bytes");
        fs::remove_file(&path).expect("scratch replacement");
        fs::write(&path, bytes).expect("same bytes, new inode");
        assert!(guard.require_current().is_err());
    }
}

#[test]
fn every_required_month_missing_or_corrupt_refuses_without_a_shorter_span() {
    for (month, timeframe) in [
        (4, Timeframe::MINUTE_1),
        (4, Timeframe::DAY_1),
        (5, Timeframe::MINUTE_1),
        (5, Timeframe::DAY_1),
        (5, Timeframe::MINUTE_5),
        (6, Timeframe::MINUTE_1),
        (6, Timeframe::DAY_1),
        (6, Timeframe::MINUTE_5),
    ] {
        for corrupt in [false, true] {
            let fixture = Fixture::new();
            let path = fixture.path(month, timeframe, FileKind::Bars);
            if corrupt {
                let mut bytes = fs::read(&path).expect("source");
                bytes[32768 + 8] ^= 1;
                fs::write(path, bytes).expect("scratch CRC mismatch");
            } else {
                fs::remove_file(path).expect("missing required month");
            }
            assert!(RangeInputs::load(fixture.request("5min")).is_err());
            assert!(
                !fixture.root.join("audited-spans-v1").exists(),
                "no range binding after incomplete input"
            );
        }
    }
}

#[test]
fn exact_record_ceiling_counts_shared_context_once_and_bad_ranges_fail_before_io() {
    let fixture = Fixture::new();
    for (rung, count) in [("1min", 1504), ("5min", 1729)] {
        let mut request = fixture.request(rung);
        request.max_records = count;
        RangeInputs::load(request).expect("exact unique-source ceiling");
        request.max_records = count - 1;
        assert!(RangeInputs::load(request).is_err());
    }
    for (from, to) in [
        ((2025, 6), (2025, 5)),
        ((2025, 0), (2025, 6)),
        ((1900, 1), (2025, 6)),
        ((2025, 5), (2025, 13)),
    ] {
        let fresh = Fixture::new();
        let mut request = fresh.request("1min");
        request.from = from;
        request.to = to;
        assert!(RangeInputs::load(request).is_err());
        assert!(!fresh.root.join("checksum-receipts-v1").exists());
    }
    for zero_records in [false, true] {
        let fresh = Fixture::new();
        let mut request = fresh.request("1min");
        if zero_records {
            request.max_records = 0;
        } else {
            request.max_bytes = 0;
        }
        assert!(RangeInputs::load(request).is_err());
        assert!(!fresh.root.join("checksum-receipts-v1").exists());
    }
}

#[test]
fn linked_role_nodes_are_exact_reusable_and_each_predecessor_stays_required() {
    for ordinal in 0..2 {
        for fault in 0..3 {
            let fixture = Fixture::new();
            let (_, first) = RangeInputs::load(fixture.request("5min"))
                .expect("first")
                .into_parts();
            let (_, second) = RangeInputs::load(fixture.request("5min"))
                .expect("identical read reuse")
                .into_parts();
            assert_eq!(first.identity, second.identity);
            assert_eq!(first.bind_digest([3; 32]), second.bind_digest([3; 32]));
            let nodes = fs::read_dir(fixture.root.join("audited-spans-v1"))
                .expect("node directory")
                .map(|entry| entry.expect("node").path())
                .collect::<Vec<_>>();
            assert_eq!(nodes.len(), 2);
            let path = nodes
                .into_iter()
                .find(|path| {
                    let bytes = fs::read(path).expect("node ordinal");
                    u64::from_le_bytes(bytes[48..56].try_into().expect("word")) == ordinal
                })
                .expect("exact chronological node");
            let mut bytes = fs::read(&path).expect("immutable node");
            assert_eq!(bytes.len(), 512);
            assert_eq!(&bytes[..8], b"BRHSP001");
            assert!(bytes[312..480].iter().all(|byte| *byte == 0));
            first
                .require_current()
                .expect("unpoisoned positive control");
            match fault {
                0 => {
                    bytes[80] ^= 1;
                    fs::write(&path, bytes).expect("role corruption");
                }
                1 => {
                    fs::remove_file(&path).expect("missing predecessor");
                }
                _ => {
                    fs::remove_file(&path).expect("replace predecessor");
                    fs::write(&path, bytes).expect("same bytes new inode");
                }
            }
            assert!(
                first.require_current().is_err(),
                "node {ordinal}, fault {fault}"
            );
            assert!(second.require_current().is_err());
        }
    }
}

#[test]
fn changing_role_order_or_range_predecessor_rekeys_the_binding() {
    let fixture = Fixture::new();
    let roles = [[1; 32], [2; 32], [3; 32], [4; 32], [5; 32], [6; 32]];
    let (_, first) = crate::checksum_receipts::publish_span_binding(
        &fixture.root,
        [0; 32],
        0,
        (2025, 5),
        &roles,
    )
    .expect("first link");
    let (_, next) =
        crate::checksum_receipts::publish_span_binding(&fixture.root, first, 1, (2025, 6), &roles)
            .expect("second link");
    let mut swapped = roles;
    swapped.swap(2, 3);
    let (_, other) = crate::checksum_receipts::publish_span_binding(
        &fixture.root,
        first,
        1,
        (2025, 6),
        &swapped,
    )
    .expect("changed role");
    assert_ne!(next, other);
    let (_, other) = crate::checksum_receipts::publish_span_binding(
        &fixture.root,
        [9; 32],
        1,
        (2025, 6),
        &roles,
    )
    .expect("changed predecessor");
    assert_ne!(next, other);
    assert!(
        crate::checksum_receipts::publish_span_binding(
            &fixture.root,
            [0; 32],
            1,
            (2025, 6),
            &roles
        )
        .is_err()
    );
    assert!(
        crate::checksum_receipts::publish_span_binding(&fixture.root, first, 0, (2025, 6), &roles)
            .is_err()
    );
}

#[test]
fn actual_strict_range_kernel_publishes_and_reuses_native_and_coarse_evidence() {
    let _serial = crate::knobs::serially();
    for rung in ["1min", "5min"] {
        let fixture = Fixture::warmed();
        let first = crate::audited_range_command::run_for_test(fixture.request(rung), 2000)
            .expect("generated strict kernel");
        assert!(!crate::carries_refusal(&first), "{first}");
        assert!(first.contains("STRICT HISTORICAL SPAN CHECKSUMS V1"));
        let mut saved =
            crate::results::Results::open_read(&fixture.root).expect("real parent ledger");
        assert_eq!(saved.len().expect("parent rows"), 1);
        let row = saved.read(0).expect("exact parent");
        assert_eq!(crate::results::read_field(&row.timeframe), rung);
        assert_eq!((row.months_asked, row.months_found), (2, 2));
        assert_eq!(
            (row.from_year, row.from_month, row.to_year, row.to_month),
            (2025, 5, 2025, 6)
        );
        assert_eq!(row.halted, 0);
        assert_eq!(row.combinations, 0);
        assert_eq!(row.trades, 0);
        let evidence = crate::sweep_evidence::read(&fixture.root, row.identity, 1_048_576)
            .expect("strict attempt")
            .expect("durable computation");
        assert_eq!(evidence.operation, crate::sweep_evidence::Operation::Audit);
        assert_eq!(
            evidence.completion,
            crate::sweep_evidence::Completion::Completed
        );
        assert_eq!(evidence.depth_rows, 1);
        assert!(evidence.ranked_available);
        assert_eq!(evidence.ranked_rows, 0);
        let path = crate::results::Results::path(&fixture.root);
        let exact = fs::read(&path).expect("saved parent bytes");
        drop(saved);
        let retry = crate::audited_range_command::run_for_test(fixture.request(rung), 2000)
            .expect("strict retry");
        assert!(!crate::carries_refusal(&retry), "{retry}");
        assert_eq!(fs::read(path).expect("parent reused"), exact);
        let retried = crate::sweep_evidence::read(&fixture.root, row.identity, 1_048_576)
            .expect("retry attempt")
            .expect("retry evidence");
        assert_eq!(
            retried.completion,
            crate::sweep_evidence::Completion::Completed
        );
        assert!(retried.attempt > evidence.attempt);
        assert_eq!(retried.depth_rows, evidence.depth_rows);
        assert_eq!(retried.ranked_rows, evidence.ranked_rows);
    }
}

#[test]
fn actual_strict_range_terminal_refuses_changed_source_binding_or_acknowledged_rank() {
    use std::cell::Cell;
    use std::rc::Rc;
    let _serial = crate::knobs::serially();
    for rung in ["1min", "5min"] {
        for fault in 0..3 {
            let fixture = Fixture::warmed();
            let identity = Rc::new(Cell::new(None));
            let seen = Rc::clone(&identity);
            let root = fixture.root.clone();
            let source = fixture.path(6, Timeframe::MINUTE_1, FileKind::Bars);
            let _hook = crate::audit_publication_tests::FaultGuard::install(move |attempt| {
                seen.set(Some(attempt.identity()));
                let live = crate::sweep_evidence::read(&root, attempt.identity(), 1_048_576)
                    .expect("actual acknowledged evidence")
                    .expect("running audit");
                assert_eq!(live.completion, crate::sweep_evidence::Completion::Running);
                assert!(live.depth_rows > 0);
                assert!(live.ranked_available);
                inject_terminal_fault(&root, &source, attempt, fault);
            });
            let report = crate::audited_range_command::run_for_test(fixture.request(rung), 2000)
                .expect("kernel returns an explicit refused report");
            let identity = identity.get().expect("actual finalization hook must run");
            assert!(crate::carries_refusal(&report), "{report}");
            assert!(!report.contains("RESULT RECORDED"), "{report}");
            assert!(!crate::results::Results::path(&fixture.root).exists());
            let evidence = crate::sweep_evidence::read(&fixture.root, identity, 1_048_576);
            if fault < 2 {
                assert_eq!(
                    evidence
                        .expect("read source refusal")
                        .expect("attempt")
                        .completion,
                    crate::sweep_evidence::Completion::Refused
                );
            } else {
                assert!(!matches!(
                    evidence,
                    Ok(Some(crate::sweep_evidence::Evidence {
                        completion: crate::sweep_evidence::Completion::Completed,
                        ..
                    }))
                ));
            }
        }
    }
}

#[test]
fn strict_api_adapter_reuses_real_parent_and_returns_late_rank_loss_as_error() {
    use std::cell::Cell;
    use std::rc::Rc;
    let _serial = crate::knobs::serially();
    for rung in ["1min", "5min"] {
        let fixture = Fixture::warmed();
        let first = crate::audited_range_command::api_for_test(fixture.request(rung), 2000, 91)
            .expect("actual shared API kernel");
        assert!(!crate::carries_refusal(&first));
        let parent = crate::results::Results::path(&fixture.root);
        let exact = fs::read(&parent).expect("actual parent bytes");
        let retry = crate::audited_range_command::api_for_test(fixture.request(rung), 2000, 92)
            .expect("actual API retry");
        assert!(retry.contains("RESULT ALREADY RECORDED AND VERIFIED"));
        assert_eq!(fs::read(&parent).expect("reused bytes"), exact);
        let seen = Rc::new(Cell::new(false));
        let invoked = Rc::clone(&seen);
        let root = fixture.root.clone();
        let source = fixture.path(6, Timeframe::MINUTE_1, FileKind::Bars);
        let _fault = crate::audit_publication_tests::FaultGuard::install(move |attempt| {
            let evidence = crate::sweep_evidence::read(&root, attempt.identity(), 1_048_576)
                .expect("read running evidence")
                .expect("actual attempt");
            assert!(evidence.ranked_available);
            assert_eq!(
                evidence.completion,
                crate::sweep_evidence::Completion::Running
            );
            invoked.set(true);
            inject_terminal_fault(&root, &source, attempt, 2);
        });
        let failure = crate::audited_range_command::api_for_test(fixture.request(rung), 2000, 93)
            .expect_err("lost acknowledged rank cannot be a successful API result");
        assert!(seen.get());
        assert!(crate::carries_refusal(&failure), "{failure}");
        assert_eq!(fs::read(&parent).expect("prior parent retained"), exact);
    }
}

#[test]
fn strict_preparation_source_change_seals_refusal_before_returning_error() {
    use std::cell::Cell;
    use std::rc::Rc;
    let _serial = crate::knobs::serially();
    for rung in ["1min", "5min"] {
        let fixture = Fixture::warmed();
        let observed = Rc::new(Cell::new(None));
        let seen = Rc::clone(&observed);
        let root = fixture.root.clone();
        let source = fixture.path(6, Timeframe::MINUTE_1, FileKind::Bars);
        let _fault = crate::audited_range_command::PreparationFault::install(move |attempt| {
            let evidence = crate::sweep_evidence::read(&root, attempt.identity(), 1_048_576)
                .expect("running preparation")
                .expect("actual durable start");
            assert_eq!(
                evidence.operation,
                crate::sweep_evidence::Operation::Preparation
            );
            assert_eq!(
                evidence.completion,
                crate::sweep_evidence::Completion::Running
            );
            seen.set(Some(attempt.identity()));
            let bytes = fs::read(&source).expect("actual held source");
            fs::write(&source, bytes).expect("same bytes change retained generation");
        });
        let failure = crate::audited_range_command::api_for_test(fixture.request(rung), 2000, 94)
            .expect_err("post-fold source changed");
        assert!(failure.contains("preparation source refused"), "{failure}");
        let identity = observed.get().expect("fault must run after real fold");
        let terminal = crate::sweep_evidence::read(&fixture.root, identity, 1_048_576)
            .expect("terminal readable")
            .expect("preparation retained");
        assert_eq!(
            terminal.completion,
            crate::sweep_evidence::Completion::Refused
        );
        assert!(!crate::results::Results::path(&fixture.root).exists());
    }
}

#[test]
fn strict_invalid_runtime_settings_refuse_before_real_source_admission_or_preparation() {
    const CHILD: &str = "BRUTEX_STRICT_RUNTIME_REFUSAL_TEST_CHILD";
    let cases = [
        ("BRUTEX_HORIZON_BARS", "4294967296"),
        ("BRUTEX_TOP", "9223372036854775808"),
        ("BRUTEX_SCREEN_CAP", "10000001"),
        ("BRUTEX_GRID_RUNGS", "1"),
        ("BRUTEX_GRID_RESOLUTION", "bad"),
        ("BRUTEX_MAX_STOP_POINTS", "bad"),
        ("BRUTEX_PROTECTED_EXITS", "bad"),
        ("BRUTEX_MIN_FILL_HEADROOM_BP", "bad"),
        ("BRUTEX_MIN_AVG_RR_BP", "bad"),
        ("BRUTEX_VALIDATE", "false"),
    ];
    if let Ok(name) = std::env::var(CHILD) {
        let fixture = Fixture::new();
        let why = crate::audited_range_command::api_for_test(fixture.request("1min"), 2000, 95)
            .expect_err("invalid explicit setting must not become a fallback result");
        assert!(why.contains(&name), "{why}");
        assert!(why.contains("before computation"), "{why}");
        assert!(!fixture.root.join("results").exists());
        assert!(!fixture.root.join("audited-spans-v1").exists());
        assert!(!fixture.root.join("checksum-receipts-v1").exists());
        return;
    }
    for (name, raw) in cases {
        let status = std::process::Command::new(std::env::current_exe().expect("test binary"))
            .args(["--exact", "audited_stored::range::tests::strict_invalid_runtime_settings_refuse_before_real_source_admission_or_preparation", "--test-threads=1"])
            .env(CHILD, name)
            .env(name, raw)
            .status()
            .expect("isolated malformed environment child");
        assert!(status.success(), "{name}");
    }
}

fn inject_terminal_fault(
    root: &Path,
    source: &Path,
    attempt: &crate::sweep_evidence::Attempt,
    fault: u8,
) {
    match fault {
        0 => {
            let bytes = fs::read(source).expect("acknowledged source image");
            fs::remove_file(source).expect("replace exact held source");
            fs::write(source, bytes).expect("same bytes, different inode");
        }
        1 => {
            let node = fs::read_dir(root.join("audited-spans-v1"))
                .expect("saved role chain")
                .next()
                .expect("one of two actual nodes")
                .expect("node")
                .path();
            let mut bytes = fs::read(&node).expect("acknowledged exact node");
            assert_eq!(bytes.len(), 512);
            bytes[80] ^= 1;
            fs::write(node, bytes).expect("corrupt exact role binding");
        }
        _ => {
            let path = root
                .join("results/sweep-evidence-v1")
                .join(crate::identity_hex(&attempt.identity()))
                .join(format!("{}-ranked.bin", attempt.token()));
            assert!(
                fs::metadata(&path)
                    .expect("acknowledged ranking header")
                    .len()
                    >= 16
            );
            fs::remove_file(path).expect("remove acknowledged ranking");
        }
    }
}

#![cfg(test)]
//! Generated scratch files exercise the actual stored-rung audit boundary.
#![allow(clippy::expect_used, clippy::indexing_slicing)]

use super::*;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Fixture {
    root: PathBuf,
    key: InstrumentKey,
    month: YearMonth,
    minutes: Vec<Bar>,
}

impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "brutex-fold-audit-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).expect("claim private fixture directory");
        let fixture = Self {
            root,
            key: InstrumentKey::index(brutex_core::instrument::Exchange::Nse, "NIFTY")
                .expect("index key"),
            month: YearMonth::new(2024, 6).expect("month"),
            minutes: (0..60)
                .map(|minute| Bar {
                    ts_micros: 1_717_386_300_000_000 + minute * 60_000_000,
                    open: 100 + minute,
                    high: 110 + minute,
                    low: 90 + minute,
                    close: 105 + minute,
                    volume: 10,
                    open_interest: i64::MIN,
                })
                .collect(),
        };
        fixture.write(Timeframe::MINUTE_1, &fixture.minutes);
        for rung in DERIVED_RUNGS {
            let folded = pull::fold::fold(&fixture.minutes, store_bucket(rung).expect("bucket"))
                .expect("generated fold");
            fixture.write(rung, &folded);
        }
        fixture
    }

    fn path(&self, rung: Timeframe) -> StorePath<'_> {
        StorePath::for_key(Vendor::Zerodha, &self.key, rung, self.month, FileKind::Bars)
            .expect("fixture path")
    }

    fn write(&self, rung: Timeframe, bars: &[Bar]) {
        let hash = brutex_core::universe::fnv1a("NIFTY").to_le_bytes();
        let symbol = u32::from_le_bytes([hash[0], hash[1], hash[2], hash[3]]);
        BarFile::open_or_create(&self.root, self.path(rung), symbol)
            .expect("fixture file")
            .append(bars)
            .expect("fixture append");
    }

    fn audit(&self) -> Result<Vec<Result<RungVerdict, String>>, String> {
        audit_month(&self.root, Vendor::Zerodha, &self.key, self.month)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn stored_audit_reads_all_seven_derived_rungs_without_changing_any_source() {
    let fixture = Fixture::new();
    let snapshots: Vec<_> = std::iter::once(Timeframe::MINUTE_1)
        .chain(DERIVED_RUNGS)
        .map(|rung| {
            let path = fixture.path(rung).to_path_buf(&fixture.root);
            let bytes = fs::read(&path).expect("snapshot");
            (path, bytes)
        })
        .collect();
    for _ in 0..2 {
        let verdicts = fixture.audit().expect("stored audit");
        assert_eq!(verdicts.len(), DERIVED_RUNGS.len());
        for (rung, result) in DERIVED_RUNGS.into_iter().zip(verdicts) {
            let verdict = result.expect("derived rung is readable");
            assert_eq!(verdict.rung, rung.as_str());
            assert_eq!(verdict.stored_bars, u64::from(3_600 / rung.secs()));
            assert_eq!(verdict.folded_bars, verdict.stored_bars);
            assert!(verdict.agrees());
            assert!(verdict.disagreements.is_empty());
            assert_eq!(verdict.elided, 0);
        }
        for (path, bytes) in &snapshots {
            assert_eq!(fs::read(path).expect("read after audit"), *bytes);
        }
    }
    let coarse = read_month(
        &fixture.root,
        Vendor::Zerodha,
        &fixture.key,
        Timeframe::MINUTE_5,
        fixture.month,
    )
    .expect("actual coarse rows");
    assert_eq!(coarse.len(), 12);
    assert_eq!(
        coarse[0],
        Bar {
            ts_micros: fixture.minutes[0].ts_micros,
            open: 100,
            high: 114,
            low: 90,
            close: 109,
            volume: 50,
            open_interest: i64::MIN,
        }
    );
}

#[test]
fn unreadable_derived_rungs_are_named_and_do_not_hide_later_verdicts() {
    let fixture = Fixture::new();
    fs::remove_file(fixture.path(Timeframe::MINUTE_2).to_path_buf(&fixture.root))
        .expect("remove only owned coarse fixture");
    fs::write(
        fixture.path(Timeframe::MINUTE_5).to_path_buf(&fixture.root),
        b"not a bar file",
    )
    .expect("corrupt only owned coarse fixture");
    let verdicts = fixture.audit().expect("minute authority remains readable");
    assert_eq!(verdicts.len(), 7);
    for (index, (rung, result)) in DERIVED_RUNGS.into_iter().zip(verdicts).enumerate() {
        if index == 0 || index == 2 {
            let why = result.expect_err("unreadable coarse file must be named");
            assert!(why.contains(rung.as_str()), "{why}");
            assert!(why.contains("could not be read"), "{why}");
        } else {
            assert!(result.expect("later rung still audited").agrees());
        }
    }
}

#[test]
fn missing_or_corrupt_minute_authority_refuses_the_entire_audit() {
    let fixture = Fixture::new();
    let path = fixture.path(Timeframe::MINUTE_1).to_path_buf(&fixture.root);
    let original = fs::read(&path).expect("minute authority bytes");
    fs::remove_file(&path).expect("remove owned minute fixture");
    let missing = fixture.audit().expect_err("missing authority");
    assert!(missing.contains("one-minute file is the authority"));
    assert!(missing.contains("1min"));
    fs::write(&path, b"not a bar file").expect("malformed owned authority");
    let corrupt = fixture.audit().expect_err("corrupt authority");
    assert!(corrupt.contains("one-minute file is the authority"));
    assert!(corrupt.contains("could not be read"));
    fs::write(path, original).expect("exact authority restoration");
    assert!(
        fixture
            .audit()
            .expect("restored authority")
            .into_iter()
            .all(|result| result.expect("restored rung").agrees())
    );
}

fn assert_fold_command_case(case: &str) {
    let mut text = String::new();
    let through = if case == "missing-month" { "7" } else { "6" };
    let code = crate::fold_audit_arm(
        &mut text,
        "zerodha",
        "NIFTY",
        ("2024", "6"),
        ("2024", through),
    );
    let (expected, counts, detail) = match case {
        "clean" => (
            crate::OK,
            "7 rung-month(s) agree, 0 DISAGREE, 0 unreadable",
            "Every stored coarse bar equals",
        ),
        "missing-rung" => (
            crate::FAILED,
            "6 rung-month(s) agree, 0 DISAGREE, 1 unreadable",
            "RUNG UNREADABLE",
        ),
        "bad-minutes" => (
            crate::FAILED,
            "0 rung-month(s) agree, 0 DISAGREE, 1 unreadable",
            "MINUTE FILE UNREADABLE",
        ),
        "missing-month" => (
            crate::FAILED,
            "7 rung-month(s) agree, 0 DISAGREE, 1 unreadable",
            "2024-07  MINUTE FILE UNREADABLE",
        ),
        "disagreement" => (
            crate::FAILED,
            "6 rung-month(s) agree, 1 DISAGREE, 0 unreadable",
            "4 more not named",
        ),
        _ => unreachable!("unknown generated command case"),
    };
    assert!(text.contains(counts), "{case}: {text}");
    assert!(text.contains(detail), "{case}: {text}");
    assert_eq!(code, expected, "{case}: {text}");
    for (feed, symbol, from, to) in [
        ("unknown", "NIFTY", ("2024", "6"), ("2024", "6")),
        ("zerodha", "UNSWEPT", ("2024", "6"), ("2024", "6")),
        ("zerodha", "NIFTY", ("2024", "7"), ("2024", "6")),
        ("zerodha", "NIFTY", ("2024", "0"), ("2024", "6")),
    ] {
        let mut report = String::new();
        assert_eq!(
            crate::fold_audit_arm(&mut report, feed, symbol, from, to),
            crate::FAILED,
            "{report}"
        );
        assert!(report.starts_with("refused:"), "{report}");
    }
}

#[test]
fn fold_command_fails_on_unreadable_authority_and_retains_every_later_verdict() {
    const CASE: &str = "BRUTEX_TEST_FOLD_AUDIT_CASE";
    if let Ok(case) = std::env::var(CASE) {
        assert_fold_command_case(&case);
        return;
    }
    let fixture = Fixture::new();
    let run = |case: &str| {
        let result = std::process::Command::new(std::env::current_exe().expect("test binary"))
            .args([
                "--exact",
                "fold_audit::io_tests::fold_command_fails_on_unreadable_authority_and_retains_every_later_verdict",
                "--nocapture",
                "--test-threads=1",
            ])
            .env(CASE, case)
            .env("BRUTEX_STORE", &fixture.root)
            .output()
            .expect("isolated command environment");
        assert!(
            result.status.success(),
            "{case}: {}{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(String::from_utf8_lossy(&result.stdout).contains("1 passed"));
    };
    run("clean");
    let coarse = fixture.path(Timeframe::MINUTE_2).to_path_buf(&fixture.root);
    let saved_coarse = fs::read(&coarse).expect("saved coarse bytes");
    fs::remove_file(&coarse).expect("remove owned generated coarse file");
    run("missing-rung");
    fs::write(&coarse, &saved_coarse).expect("restore generated coarse file");

    let minute = fixture.path(Timeframe::MINUTE_1).to_path_buf(&fixture.root);
    let saved_minutes = fs::read(&minute).expect("saved minute bytes");
    fs::write(&minute, b"unreadable generated source").expect("owned corruption");
    run("bad-minutes");
    fs::write(&minute, &saved_minutes).expect("restore generated minutes");
    run("missing-month");

    let five = fixture.path(Timeframe::MINUTE_5).to_path_buf(&fixture.root);
    let saved_five = fs::read(&five).expect("saved five-minute bytes");
    let checksums = fixture
        .path(Timeframe::MINUTE_5)
        .with_file(FileKind::Checksums)
        .to_path_buf(&fixture.root);
    let saved_checksums = fs::read(&checksums).expect("saved five-minute checksums");
    let mut changed = pull::fold::fold(
        &fixture.minutes,
        store_bucket(Timeframe::MINUTE_5).expect("five-minute bucket"),
    )
    .expect("generated fold");
    for bar in &mut changed {
        bar.close += 1;
    }
    fs::remove_file(&five).expect("replace only generated five-minute source");
    fixture.write(Timeframe::MINUTE_5, &changed);
    run("disagreement");
    fs::write(&five, &saved_five).expect("restore exact five-minute bytes");
    fs::write(&checksums, &saved_checksums).expect("restore matching five-minute checksums");
    run("clean");
    assert_eq!(fs::read(coarse).expect("coarse after audit"), saved_coarse);
    assert_eq!(
        fs::read(minute).expect("minutes after audit"),
        saved_minutes
    );
    assert_eq!(fs::read(five).expect("five-minute after audit"), saved_five);
    assert_eq!(
        fs::read(checksums).expect("checksums after audit"),
        saved_checksums
    );
}

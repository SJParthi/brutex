//! Operator-facing limits and disclosures, exercised without market data.

use crate::{
    FAILED, MISUSED, OK, cap_within_budget, command_report, dispatch, exhausted_walk, grouped,
    min_hits_for, retention_note, unsourceable_minute, validation_note,
};

struct ScratchCleanup(std::path::PathBuf);
impl Drop for ScratchCleanup {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn operator_self_check_rejects_all_malformed_requests_before_claiming_provenance() {
    for feed in ["zerodha", "dhan", "groww"] {
        let check = crate::refusal_surface(feed, "NIFTY");
        assert!(check.held, "{feed}: {}", check.evidence);
        assert_eq!(check.evidence, "6 of 6 refused cleanly");
    }
}

#[test]
fn series_self_checks_require_observations_and_a_real_suffix()
-> Result<(), Box<dyn std::error::Error>> {
    let _knobs = crate::knobs::serially();
    crate::knobs::clear_all();
    let mut span = crate::stored::Span {
        bars: Vec::new(),
        vendor: brutex_core::vendor::Vendor::Zerodha,
        key: crate::stored::swept_index("NIFTY")?,
        timeframe: "1min",
        asked: 1,
        found: 1,
        missing: Vec::new(),
        excluded: crate::stored::CalendarExclusion::none(),
    };
    for bars in [Vec::new(), vec![runner::synthetic::bar(0, 0)]] {
        span.bars = bars;
        let [repeatability, causality] = crate::measured_series_checks(&span)?;
        assert!(!repeatability.held, "{}", repeatability.evidence);
        assert!(!causality.held, "{}", causality.evidence);
        assert!(repeatability.evidence.contains("complete false vs false"));
        assert!(causality.evidence.contains("observable prefix rows 0"));
    }
    // Seven generated short sessions warm the 200-bar indicator and the
    // five-session ladder. Fewer than 120 observable bars makes the sweep
    // finish by extinction, rather than exhaust a candidate budget.
    span.bars = runner::synthetic::session_of(7, 40);
    let [repeatability, causality] = crate::measured_series_checks(&span)?;
    assert!(repeatability.held, "{}", repeatability.evidence);
    assert!(repeatability.evidence.contains("complete true vs true"));
    assert!(causality.held, "{}", causality.evidence);
    assert!(causality.evidence.contains("suffix bars 1; 0 differ"));
    assert!(!causality.evidence.contains("observable prefix rows 0;"));
    Ok(())
}

#[test]
fn stored_self_check_reports_partial_history_and_missing_feed_as_failures()
-> Result<(), Box<dyn std::error::Error>> {
    const CHILD: &str = "BRUTEX_TEST_STORED_SELF_CHECK";
    if std::env::var_os(CHILD).is_some() {
        let root = crate::store_root()?;
        let key = crate::stored::swept_index("NIFTY")?;
        let mut originals = Vec::new();
        for month in [4, 5] {
            let path = store::path::StorePath::for_key(
                brutex_core::vendor::Vendor::Zerodha,
                &key,
                store::path::Timeframe::DAY_1,
                store::path::YearMonth::new(2025, month)?,
                store::path::FileKind::Bars,
            )?
            .to_path_buf(&root);
            originals.push((path.clone(), std::fs::read(path)?));
        }
        let mut report = String::new();
        assert_eq!(crate::verify_arm(&mut report, "zerodha", "NIFTY"), FAILED);
        for text in [
            "2 of 81 months",
            "two runs of one slice agree byte for byte",
            "identical",
            "a bar's conditions do not change because later bars exist",
            "0 observable bars vs 0; complete false vs false",
            "observable prefix rows 0; suffix bars 1; 0 differ",
            "3 of 6 checks passed",
            "A FAILURE ABOVE",
        ] {
            assert!(report.contains(text), "missing {text}: {report}");
        }
        assert!(!report.contains("Every property above was MEASURED"));
        let mut missing = String::new();
        assert_eq!(crate::verify_arm(&mut missing, "dhan", "NIFTY"), FAILED);
        assert!(missing.contains("the span loads at all"), "{missing}");
        assert!(missing.contains("2 of 3 checks passed"), "{missing}");
        let mut unknown = String::new();
        assert_eq!(crate::verify_arm(&mut unknown, "unknown", "NIFTY"), FAILED);
        assert!(unknown.starts_with("refused:"), "{unknown}");
        for (path, bytes) in originals {
            assert_eq!(std::fs::read(path)?, bytes);
        }
        return Ok(());
    }
    crate::audited_stored::with_warmed_store(|root| {
        let result = std::process::Command::new(std::env::current_exe()?)
            .args([
                "--exact",
                "operator_boundary_tests::stored_self_check_reports_partial_history_and_missing_feed_as_failures",
                "--nocapture",
                "--test-threads=1",
            ])
            .env(CHILD, "generated")
            .env("BRUTEX_STORE", root)
            .output()?;
        assert!(
            result.status.success(),
            "{}{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(String::from_utf8_lossy(&result.stdout).contains("1 passed"));
        Ok(())
    })
}

#[test]
fn ledger_self_checks_do_not_delete_an_existing_directory_and_can_run_concurrently()
-> Result<(), Box<dyn std::error::Error>> {
    use std::fs;
    use std::sync::{Arc, Barrier};
    let legacy = std::env::temp_dir().join(format!("brutex-verify-ledger-{}", std::process::id()));
    fs::create_dir(&legacy)?;
    let _owned = ScratchCleanup(legacy.clone());
    let marker = legacy.join("held-by-another-check");
    fs::write(&marker, b"keep these bytes")?;
    let check = crate::ledger_round_trip();
    assert!(check.held, "{}", check.evidence);
    assert_eq!(fs::read(&marker)?, b"keep these bytes");
    let start = Arc::new(Barrier::new(8));
    let checks = std::thread::scope(|scope| {
        let workers: Vec<_> = (0..8)
            .map(|_| {
                let start = Arc::clone(&start);
                scope.spawn(move || {
                    start.wait();
                    crate::ledger_round_trip()
                })
            })
            .collect();
        workers
            .into_iter()
            .map(std::thread::ScopedJoinHandle::join)
            .collect::<Vec<_>>()
    });
    for check in checks {
        let check = check.map_err(|_| "self-check worker panicked")?;
        assert!(check.held, "{}", check.evidence);
        assert!(check.evidence.contains("all equal"));
    }
    assert_eq!(fs::read(marker)?, b"keep these bytes");

    let first = crate::verification_scratch()?;
    let _first = ScratchCleanup(first.clone());
    let name = first
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("scratch name")?;
    let (prefix, serial) = name.rsplit_once('-').ok_or("scratch serial")?;
    let serial = serial.parse::<u64>()?;
    let mut collisions = Vec::new();
    for offset in 1..=16 {
        let next = serial
            .checked_add(offset)
            .ok_or("scratch serial overflow")?;
        let path = first.with_file_name(format!("{prefix}-{next}"));
        fs::create_dir(&path)?;
        collisions.push(ScratchCleanup(path.clone()));
        fs::write(path.join("owner"), b"already held")?;
    }
    let refused = crate::ledger_round_trip();
    assert!(!refused.held);
    assert!(refused.evidence.contains("16 collisions"));
    for held in &collisions {
        assert_eq!(fs::read(held.0.join("owner"))?, b"already held");
    }
    let recovered = crate::ledger_round_trip();
    assert!(recovered.held, "{}", recovered.evidence);
    Ok(())
}

#[test]
fn calibrated_caps_keep_measured_work_and_never_exceed_available_candidates() {
    for (sampled, nanos, budget, offered, expected) in [
        (0, 1, 0, 100, 100),
        (10, 0, 0, 100, 100),
        (10, 10_000_000, 0, 100, 10),
        (10, 10_000_000, 20, 100, 16),
        (10, 10_000_000, 31, 100, 16),
        (10, 10_000_000, 32, 100, 32),
        (10, 10_000_000, 33, 100, 32),
        (10, 10_000_000, 32, 20, 20),
        (25, 10_000_000, 0, 10, 10),
        (1, 1, u64::MAX, 0, 0),
        (usize::MAX, 1, u64::MAX, 31, 31),
        (1, u128::MAX, 1, 100, 1),
    ] {
        assert_eq!(cap_within_budget(sampled, nanos, budget, offered), expected);
    }
}

#[test]
fn support_is_scaled_in_ppm_and_cannot_disable_extinction() {
    for (bars, support, expected) in [
        (0, 20_000, 1),
        (200, 0, 1),
        (49, 20_000, 1),
        (200, 20_000, 4),
        (200, 50_000, 10),
        (200, 1_000_000, 200),
        (623_546, 20_000, 12_470),
    ] {
        assert_eq!(min_hits_for(bars, support), expected);
    }
}

#[test]
fn missing_minute_diagnostics_do_not_invent_a_timestamp() {
    for (text, expected) in [
        ("missing minute", None),
        ("expected_ts_micros:", None),
        ("expected_ts_micros: no timestamp", None),
        ("expected_ts_micros: -1", None),
        ("expected_ts_micros: 9223372036854775808", None),
        ("expected_ts_micros: 0", Some(0)),
        ("expected_ts_micros: 9223372036854775807", Some(i64::MAX)),
        (
            "gap { expected_ts_micros: 1780000000000000, actual: 7 }",
            Some(1_780_000_000_000_000),
        ),
    ] {
        assert_eq!(unsourceable_minute(text), expected);
    }
}

#[test]
fn validation_disclosures_keep_complete_sections_and_their_qualifications() {
    assert_eq!(validation_note("mentions BOOTSTRAP in prose only"), None);
    let report = "AUDIT\n  costs excluded\n\nSIGNIFICANCE\n  threshold 17\n\nFINDINGS\n  nothing passed\n\nWALK-FORWARD\n  fold one\n\n  fold two\n\nOVERFITTING\n  NON-AUTHORITATIVE\n\nBOOTSTRAP\n  short sample\n\nWINDOW SHAPE -- did the edge survive on RECENT history too\n  not measured\n\nNEXT SECTION\n  unrelated";
    let expected = "AUDIT\n  costs excluded\nSIGNIFICANCE\n  threshold 17\nFINDINGS\n  nothing passed\nWALK-FORWARD\n  fold one\n\n  fold two\nOVERFITTING\n  NON-AUTHORITATIVE\nBOOTSTRAP\n  short sample\nWINDOW SHAPE -- did the edge survive on RECENT history too\n  not measured";
    assert_eq!(validation_note(report).as_deref(), Some(expected));
    assert_eq!(
        validation_note("paragraph about OVERFITTING\n\nOVERFITTING\n  not measured").as_deref(),
        Some("OVERFITTING\n  not measured")
    );
}

#[test]
fn retention_disclosure_preserves_the_reason_no_candidate_traded() {
    assert_eq!(retention_note("no retention was supplied"), None);
    assert_eq!(
        retention_note("STREAMED RETENTION\n  kept 25").as_deref(),
        Some("STREAMED RETENTION\n  kept 25")
    );
    assert_eq!(
        retention_note("STREAMED RETENTION\n  kept 25\n\nNEXT\n  ignored\nno final screened candidate was admitted: strict rules\nother detail").as_deref(),
        Some("STREAMED RETENTION\n  kept 25\n  no final screened candidate was admitted: strict rules")
    );
}

#[test]
fn count_rendering_preserves_every_digit_at_group_boundaries() {
    for (value, expected) in [
        (0, "0"),
        (999, "999"),
        (1_000, "1,000"),
        (999_999, "999,999"),
        (1_000_000, "1,000,000"),
        (u64::MAX, "18,446,744,073,709,551,615"),
    ] {
        assert_eq!(grouped(value), expected);
    }
}

#[test]
fn command_reports_preserve_the_failure_and_the_requested_result() {
    let mut out = String::from("prior\n");
    assert_eq!(
        command_report(&mut out, Ok("measured\n".to_owned()), "AUDIT"),
        OK
    );
    assert_eq!(out, "prior\nmeasured\n");
    assert_eq!(
        command_report(&mut out, Err("missing receipt".to_owned()), "AUDIT"),
        FAILED
    );
    assert_eq!(out, "prior\nmeasured\nAUDIT REFUSED: missing receipt\n");
    assert_eq!(command_report(&mut out, Ok(String::new()), "AUDIT"), OK);
    assert_eq!(out, "prior\nmeasured\nAUDIT REFUSED: missing receipt\n");
}

#[test]
fn an_exhausted_search_keeps_its_scope_and_last_failed_page() {
    let mut without = String::new();
    exhausted_walk(&mut without, 20_000, None);
    assert!(without.contains("NOTHING PASSED AT ANY SUPPORT, down to 20000 ppm"));
    assert!(without.contains("under THESE rules"));
    let mut with = String::new();
    exhausted_walk(&mut with, 20_000, Some("candidate A: failed drawdown"));
    assert_eq!(with, format!("{without}\ncandidate A: failed drawdown"));
}

#[test]
fn malformed_stored_command_numbers_refuse_before_any_store_access() {
    let cases: &[(&[&str], &[usize])] = &[
        (
            &["fold-audit", "zerodha", "NIFTY", "2026", "8", "2026", "8"],
            &[3, 4, 5, 6],
        ),
        (
            &["pool", "zerodha", "1min", "2026", "8", "2026", "8", "200"],
            &[3, 4, 5, 6, 7],
        ),
        (
            &[
                "range-rung",
                "zerodha",
                "NIFTY",
                "1min",
                "2026",
                "8",
                "2026",
                "8",
                "20000",
            ],
            &[4, 5, 6, 7, 8],
        ),
        (
            &[
                "screen", "zerodha", "NIFTY", "1min", "2026", "8", "2026", "8", "20000", "20", "2",
                "10",
            ],
            &[4, 5, 6, 7, 8, 9, 10, 11],
        ),
        (
            &[
                "elite", "zerodha", "NIFTY", "1min", "2026", "8", "2026", "8", "20", "10",
            ],
            &[4, 5, 6, 7, 8, 9],
        ),
        (
            &[
                "descend", "zerodha", "NIFTY", "1min", "2026", "8", "2026", "8", "20000", "1",
            ],
            &[4, 5, 6, 7, 8, 9],
        ),
        (&["sweep-all", "zerodha", "1min", "200"], &[3]),
    ];
    for (words, positions) in cases {
        for &position in *positions {
            let mut args: Vec<String> = words.iter().map(|word| (*word).to_owned()).collect();
            if let Some(argument) = args.get_mut(position) {
                "not-a-number".clone_into(argument);
            } else {
                assert!(position < args.len());
            }
            let mut report = String::new();
            assert_eq!(dispatch(&args, &mut report), MISUSED);
            assert!(report.contains("refused:"));
            assert!(!report.contains("REAL MARKET DATA"));
        }
    }
}

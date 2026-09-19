#![cfg(test)]
//! Explicit strict-command parsing, admission and terminal status regressions.
#![allow(clippy::expect_used)]
use super::*;
use cli::audited_range_command::StrictConfig;
use std::cell::Cell;

const STAMP: &str = "0123456789abcdef0123456789abcdef01234567";
const BODY: &str = r#"{"command":"audit-audited-range","feed":"zerodha","underlying":"NIFTY","rung":"5min","from_year":2025,"from_month":4,"to_year":2025,"to_month":5,"min_hits":1000000}"#;

#[test]
fn exact_status_selector_requires_one_canonical_positive_u64_without_aliases() {
    assert_eq!(requested_attempt(None), Ok(None));
    assert_eq!(requested_attempt(Some("")), Ok(None));
    for value in [1, (1_u64 << 53) + 1, u64::MAX] {
        assert_eq!(
            requested_attempt(Some(&format!("attempt={value}"))),
            Ok(Some(value))
        );
    }
    for query in [
        "attempt",
        "attempt=",
        "attempt=0",
        "attempt=01",
        "attempt=-1",
        "attempt=+1",
        "attempt=1.0",
        "attempt=1e3",
        "attempt=18446744073709551616",
        "attempt=1&attempt=1",
        "attempt=1&attempt=2",
        "attempt=1&",
        "attempt=%31",
        "at%74empt=1",
        "attempt_key=1",
        "other=1&attempt=1",
        "attempt=1&other=1",
    ] {
        assert!(requested_attempt(Some(query)).is_err(), "{query}");
    }
}

#[tokio::test(flavor = "current_thread")]
async fn exact_status_retains_browser_terminal_despite_active_or_unknown_external_status() {
    let site = site("exact-terminal");
    let attempt = (1_u64 << 53) + 1;
    let mut local = Progress::started("zerodha", "NIFTY", (2025, 4), (2025, 5), None, 1, attempt)
        .of_kind(Kind::Command);
    settle_strict_result(
        &mut local,
        Ok("exact browser terminal report".to_owned()),
        2,
    );
    *site.sweep.lock().expect("local status lock") = Some(local.clone());
    let dir = crate::scratch::path("strict-status-external-active");
    let sink = telemetry::Sink::open(&telemetry::Config::new(&dir)).expect("scratch external log");
    assert_eq!(
        sink.emit_for_run(
            44,
            &telemetry::Event::info("cli.lifecycle", "command started")
                .with("phase", "running")
                .with("command", "sweep-stored"),
        ),
        telemetry::Emitted::Written
    );
    for external in [Some(dir.as_path()), None] {
        let global = observed_status(Some(&local), external_observation(external, 0));
        assert!(!global.contains("exact browser terminal report"));
        assert!(global.contains(r#""where":"cli""#));
        let uri = format!("/backtest/run.json?attempt={attempt}")
            .parse()
            .expect("exact URI");
        let (status, _, body) = run_json(axum::extract::State(site.clone()), uri).await;
        assert_eq!(status, axum::http::StatusCode::OK);
        assert_eq!(body, format!(r#"{{"running":{}}}"#, local.to_json()));
        assert!(body.contains(&format!(r#""attempt_key":"{attempt}""#)));
        assert!(!body.contains(r#""where":"cli""#));
    }
    drop(sink);
    std::fs::remove_dir_all(dir).expect("remove scratch external log");
}

#[tokio::test(flavor = "current_thread")]
async fn exact_status_keeps_running_and_refused_states_and_never_substitutes_another_attempt() {
    let site = site("exact-missing");
    let mut local = Progress::started("zerodha", "NIFTY", (2025, 4), (2025, 5), None, 1, 91)
        .of_kind(Kind::Command);
    for terminal in [false, true] {
        if terminal {
            settle_strict_result(&mut local, Err("required receipt changed".to_owned()), 2);
        }
        *site.sweep.lock().expect("local status lock") = Some(local.clone());
        let (status, _, body) = run_json(
            axum::extract::State(site.clone()),
            "/backtest/run.json?attempt=91".parse().expect("exact URI"),
        )
        .await;
        assert_eq!(status, axum::http::StatusCode::OK);
        assert_eq!(body, format!(r#"{{"running":{}}}"#, local.to_json()));
        assert_eq!(local.in_flight(), !terminal);
    }
    for retained in [Some(local), None] {
        *site.sweep.lock().expect("local status lock") = retained;
        let (status, _, body) = run_json(
            axum::extract::State(site.clone()),
            "/backtest/run.json?attempt=92"
                .parse()
                .expect("missing URI"),
        )
        .await;
        assert_eq!(status, axum::http::StatusCode::OK);
        let parsed: serde_json::Value = serde_json::from_str(&body).expect("unknown JSON");
        let run = parsed.get("running").expect("unknown run");
        assert_eq!(
            run.get("status").and_then(serde_json::Value::as_str),
            Some("unknown")
        );
        assert_eq!(
            run.get("requested_attempt")
                .and_then(serde_json::Value::as_str),
            Some("92")
        );
        assert!(
            run.get("attempt_key").is_none(),
            "an absent attempt has no actual token"
        );
        assert!(run.get("report").is_some_and(serde_json::Value::is_null));
        assert!(
            !body.contains("required receipt changed"),
            "foreign refusal cannot substitute"
        );
    }
    for query in ["attempt=", "attempt=0", "attempt=91&attempt=92"] {
        let uri = format!("/backtest/run.json?{query}")
            .parse()
            .expect("invalid-selector URI");
        let (status, _, body) = run_json(axum::extract::State(site.clone()), uri).await;
        assert_eq!(status, axum::http::StatusCode::BAD_REQUEST);
        assert!(body.contains(r#""status":"unknown""#));
    }
}

#[test]
fn command_acceptance_and_status_preserve_exact_attempts_beyond_browser_integer_precision() {
    let mut previous = None;
    for attempt in [
        1,
        (1_u64 << 53) - 1,
        1_u64 << 53,
        (1_u64 << 53) + 1,
        u64::MAX,
    ] {
        let accepted = command_acceptance(attempt);
        assert!(accepted.contains(r#""accepted":true"#));
        assert!(accepted.contains(r#""refusal":null"#));
        assert!(accepted.contains(&format!(r#""attempt":"{attempt}""#)));
        assert_ne!(
            previous.as_ref(),
            Some(&accepted),
            "adjacent u64 keys cannot alias"
        );
        previous = Some(accepted);
        let mut progress =
            Progress::started("zerodha", "NIFTY", (2025, 4), (2025, 5), None, 1, attempt)
                .of_kind(Kind::Command);
        for finished in [false, true] {
            if finished {
                settle_strict_result(&mut progress, Ok("saved evidence".to_owned()), 2);
            }
            let json = progress.to_json();
            assert!(json.contains(r#""where":"browser""#));
            assert!(json.contains(&format!(r#""attempt_key":"{attempt}""#)));
            assert!(json.contains(&format!(r#""attempt":{attempt},"#)));
            assert_eq!(progress.in_flight(), !finished);
        }
    }
    let source = include_str!("sweeprun.rs");
    let actual = source
        .split("fn command_with_configuration(")
        .nth(1)
        .and_then(|tail| tail.split("fn command_acceptance(").next())
        .expect("actual command acceptance body");
    assert!(actual.contains("let (started, attempt, audit, launch, lease) ="));
    assert!(actual.contains("command_acceptance(attempt)"));
    assert!(actual.contains("conduct_strict_command(&asked, started, attempt"));
}

fn site(name: &str) -> crate::server::Loaded {
    let masters = crate::scratch::path(&format!("strict-command-masters-{name}"));
    let store = crate::scratch::path(&format!("strict-command-store-{name}"));
    std::sync::Arc::new(crate::server::Site::load(&masters, &store))
}

fn config() -> StrictConfig {
    let root = crate::scratch::path("strict-command-receipt-config");
    std::fs::create_dir_all(&root).expect("private receipt fixture");
    StrictConfig::from_values(
        Some(root.into()),
        Some("33554432".into()),
        Some("1000000".into()),
    )
    .expect("explicit physical configuration")
}

#[test]
fn strict_command_parser_and_typed_adapter_preserve_exact_request_and_attempt() {
    let asked = command_from(BODY).expect("strict command");
    assert_eq!(asked.word(), "audit-audited-range");
    assert_eq!(asked.feed(), "zerodha");
    assert_eq!(asked.underlying(), "NIFTY");
    assert_eq!(asked.window(), ((2025, 4), (2025, 5)));
    let root = std::path::Path::new("/server/owned/store");
    let exact = strict_request(&asked, 41, root).expect("typed actual adapter");
    assert_eq!(exact.store_root, root);
    assert_eq!(
        (exact.vendor, exact.underlying, exact.rung),
        ("zerodha", "NIFTY", "5min")
    );
    assert_eq!(
        (exact.from, exact.to, exact.min_hits, exact.attempt),
        ((2025, 4), (2025, 5), 1_000_000, Some(41))
    );
    for invalid in [
        BODY.replace("1000000", "0"),
        BODY.replace("5min", "1day"),
        BODY.replace("2025,\"to_month\":5", "2024,\"to_month\":5"),
    ] {
        assert!(command_from(&invalid).is_err(), "{invalid}");
    }
}

#[test]
fn request_paths_and_limits_cannot_replace_server_strict_configuration() {
    let mut injected = BODY.trim_end_matches('}').to_owned();
    injected.push_str(r#", "receipt_root":"/tmp/injected", "max_bytes":1, "max_records":1, "BRUTEX_CHECKSUM_RECEIPTS":"/tmp/injected", "store_root":"/tmp/injected"}"#);
    assert_eq!(command_from(&injected), command_from(BODY));
    let resolved = config();
    assert_eq!(
        resolved.receipt_root(),
        crate::scratch::path("strict-command-receipt-config")
    );
    assert_eq!(
        (resolved.max_bytes(), resolved.max_records()),
        (33_554_432, 1_000_000)
    );
}

#[test]
fn strict_command_preserves_supported_knobs_and_refuses_irrelevant_support_controls() {
    let mut body = BODY.trim_end_matches('}').to_owned();
    body.push_str(r#", "ceiling":1234, "screen_cap":7, "validate":false, "min_win_rate_bp":6500}"#);
    let asked = command_from(&body).expect("existing meaningful settings");
    let context = strict_knob_context(&asked).expect("same Applied guard context");
    assert_eq!(context.rungs, ["5min"]);
    assert_eq!(
        context.knobs,
        [
            ("BRUTEX_CEILING", "1234".to_owned()),
            ("BRUTEX_SCREEN_CAP", "7".to_owned()),
            ("BRUTEX_VALIDATE", "0".to_owned()),
            ("BRUTEX_MIN_WIN_RATE_BP", "6500".to_owned()),
        ]
    );
    for name in ["support_ppm", "sizing_rate_bp"] {
        let body = format!("{},\"{name}\":123}}", BODY.trim_end_matches('}'));
        let why = command_from(&body).expect_err("absolute support route");
        assert!(why.why().contains(name));
        assert!(why.why().contains("No setting was ignored"));
    }
    for (name, value) in [
        ("validate", "unknown"),
        ("ceiling", "0"),
        ("screen_cap", "-1"),
        ("min_win_rate_bp", "6500bp"),
        ("grid_resolution", "0"),
        ("top", " "),
    ] {
        let body = format!("{},\"{name}\":\"{value}\"}}", BODY.trim_end_matches('}'));
        let why = command_from(&body).expect_err("unusable settings cannot fall back");
        assert!(why.why().contains(name));
        assert!(why.why().contains("No setting was ignored"));
    }
}

#[test]
fn missing_strict_configuration_is_structured_503_before_attempt_or_slot() {
    let site = site("missing");
    let calls = Cell::new(0);
    let mark = crate::emitted::mark();
    let (status, headers, body) = command_with_configuration(&site, BODY, Some(STAMP), || {
        calls.set(calls.get() + 1);
        StrictConfig::from_values(None, None, None)
    });
    assert_eq!(calls.get(), 1);
    assert_eq!(status, axum::http::StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(headers, json_headers());
    let response: serde_json::Value = serde_json::from_str(&body).expect("structured refusal");
    assert_eq!(
        response.get("code").and_then(serde_json::Value::as_str),
        Some("strict_input_configuration_missing_or_invalid")
    );
    assert_eq!(
        response
            .get("accepted")
            .and_then(serde_json::Value::as_bool),
        Some(false)
    );
    assert_eq!(
        response.get("started").and_then(serde_json::Value::as_bool),
        Some(false)
    );
    assert_eq!(
        response.get("missing"),
        Some(&serde_json::json!([
            "BRUTEX_CHECKSUM_RECEIPTS",
            "BRUTEX_CHECKSUM_MAX_BYTES",
            "BRUTEX_CHECKSUM_MAX_RECORDS"
        ]))
    );
    assert_eq!(response.get("invalid"), Some(&serde_json::json!([])));
    assert!(site.sweep.lock().expect("slot").is_none());
    assert!(!site.store_root.join("results").exists());
    let events = crate::emitted::landed(
        mark,
        "api.sweep",
        "an engine command was refused before it started",
    );
    assert!(
        events
            .iter()
            .any(|event| event.level == telemetry::Level::Warn
                && crate::emitted::says(event, "why", "BRUTEX_CHECKSUM_RECEIPTS")
                && crate::emitted::says(event, "why", "BRUTEX_CHECKSUM_MAX_BYTES")
                && crate::emitted::says(event, "why", "BRUTEX_CHECKSUM_MAX_RECORDS")
                && event.field("attempt").is_none())
    );
}

#[test]
fn invalid_limits_are_refused_and_commit_gate_precedes_configuration() {
    let site = site("invalid");
    let (status, _, body) = command_with_configuration(&site, BODY, Some(STAMP), || {
        StrictConfig::from_values(
            Some(config().receipt_root().as_os_str().to_owned()),
            Some("0".into()),
            Some("overflow".into()),
        )
    });
    assert_eq!(status, axum::http::StatusCode::SERVICE_UNAVAILABLE);
    let response: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(
        response.get("invalid"),
        Some(&serde_json::json!([
            "BRUTEX_CHECKSUM_MAX_BYTES",
            "BRUTEX_CHECKSUM_MAX_RECORDS"
        ]))
    );
    let called = Cell::new(false);
    let (status, _, body) = command_with_configuration(&site, BODY, None, || {
        called.set(true);
        Ok(config())
    });
    assert_eq!(status, axum::http::StatusCode::SERVICE_UNAVAILABLE);
    assert!(body.contains("BRUTEX_COMMIT"));
    assert!(!called.get());
    assert!(site.sweep.lock().expect("slot").is_none());
}

#[test]
fn strict_and_ordinary_commands_keep_the_existing_shared_busy_gate() {
    let site = site("busy");
    *site.sweep.lock().expect("slot") = Some(Progress::started(
        "zerodha",
        "NIFTY",
        (2025, 4),
        (2025, 5),
        None,
        1,
        41,
    ));
    for (body, expected_calls) in [
        (BODY.to_owned(), 1),
        (BODY.replace("audit-audited-range", "audit-range"), 0),
    ] {
        let calls = Cell::new(0);
        let (status, _, _) = command_with_configuration(&site, &body, Some(STAMP), || {
            calls.set(calls.get() + 1);
            Ok(config())
        });
        assert_eq!(status, axum::http::StatusCode::CONFLICT);
        assert_eq!(calls.get(), expected_calls);
        let held = site.sweep.lock().expect("slot");
        assert_eq!(held.as_ref().expect("old run").attempt, 41);
        assert!(held.as_ref().expect("old run").in_flight());
    }
}

#[test]
fn strict_late_refusal_and_abnormal_end_release_the_slot_without_success() {
    let site = site("terminal");
    let mut progress = Progress::started("zerodha", "NIFTY", (2025, 4), (2025, 5), None, 1, 41)
        .of_kind(Kind::Command);
    settle_strict_result(&mut progress, Ok("completed computation".to_owned()), 2);
    settle_strict_result(
        &mut progress,
        Err("REAL MARKET DATA\nNOT RECORDED: child changed".to_owned()),
        3,
    );
    assert_eq!(progress.attempt, 41);
    assert!(progress.report.is_none());
    assert!(
        progress
            .refusal
            .as_ref()
            .is_some_and(|why| why.contains("child changed"))
    );
    assert!(!progress.in_flight());
    assert_eq!(completion_audit(&progress).outcome, "refused");
    *site.sweep.lock().expect("slot") = Some(
        Progress::started("zerodha", "NIFTY", (2025, 4), (2025, 5), None, 4, 42)
            .of_kind(Kind::Command),
    );
    drop(TaskFinisher::new(std::sync::Arc::clone(&site)));
    let held = site.sweep.lock().expect("slot");
    let terminal = held.as_ref().expect("refused attempt retained");
    assert_eq!(terminal.attempt, 42);
    assert!(!terminal.in_flight());
    assert_eq!(terminal.refusal.as_deref(), Some(ABNORMAL_END));
    assert!(terminal.report.is_none());
}

#[test]
fn strict_worker_retains_real_identity_gate_or_refuses_missing_source_without_parent() {
    let site = site("real-adapter");
    let asked = command_from(BODY).expect("command");
    let output = conduct_strict_command(&asked, 5, 43, &config(), &site.store_root);
    assert_eq!(output.attempt, 43);
    assert!(!output.in_flight());
    assert!(output.report.is_none());
    let why = output.refusal.expect("real build gate or missing input");
    if cli::commit_stamp().is_none() {
        assert!(why.contains("verified clean build identity"), "{why}");
    } else {
        assert!(
            why.contains("No such file") || why.contains("missing") || why.contains("absent"),
            "{why}"
        );
    }
    assert!(!site.store_root.join("results/runs.bin").exists());
}

#[test]
fn strict_out_of_domain_request_settings_refuse_before_configuration_slot_or_start() {
    let site = site("scalar-bounds");
    for (name, value) in [
        ("horizon_bars", "4294967296"),
        ("top", "9223372036854775808"),
        ("screen_cap", "10000001"),
        ("grid_rungs", "1"),
        ("grid_rungs", "18446744073709551615"),
        ("ceiling", "18446744073709551615"),
        ("min_rr_bp", "9223372036854775808"),
    ] {
        let body = format!("{},\"{name}\":\"{value}\"}}", BODY.trim_end_matches('}'));
        let configured = Cell::new(false);
        let (status, _, response) = command_with_configuration(&site, &body, Some(STAMP), || {
            configured.set(true);
            Ok(config())
        });
        assert_eq!(status, axum::http::StatusCode::BAD_REQUEST, "{response}");
        assert!(response.contains(name), "{response}");
        assert!(!configured.get());
        assert!(site.sweep.lock().expect("slot").is_none());
        assert!(!site.store_root.join("results").exists());
    }
    for (name, value) in [
        ("horizon_bars", "4294967295"),
        ("top", "9223372036854775807"),
        ("screen_cap", "10000000"),
    ] {
        let body = format!("{},\"{name}\":\"{value}\"}}", BODY.trim_end_matches('}'));
        assert!(command_from(&body).is_ok(), "{name}={value}");
    }
}

#[test]
fn strict_invalid_server_environment_refuses_before_configuration_slot_or_start() {
    const CHILD: &str = "BRUTEX_STRICT_ENVIRONMENT_TEST_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let status = std::process::Command::new(std::env::current_exe().expect("test binary"))
            .arg("--exact")
            .arg("sweeprun::strict_tests::strict_invalid_server_environment_refuses_before_configuration_slot_or_start")
            .arg("--test-threads=1")
            .env(CHILD, "1")
            .env("BRUTEX_MAX_STOP_POINTS", "not-an-integer")
            .status()
            .expect("isolated environment test child");
        assert!(status.success());
        return;
    }
    let site = site("invalid-server-setting");
    let configured = Cell::new(false);
    let (status, _, body) = command_with_configuration(&site, BODY, Some(STAMP), || {
        configured.set(true);
        Ok(config())
    });
    assert_eq!(status, axum::http::StatusCode::SERVICE_UNAVAILABLE);
    let response: serde_json::Value = serde_json::from_str(&body).expect("structured refusal");
    assert_eq!(
        response.get("code"),
        Some(&serde_json::json!("strict_runtime_settings_invalid"))
    );
    assert_eq!(response.get("started"), Some(&serde_json::json!(false)));
    assert_eq!(
        response.get("invalid"),
        Some(&serde_json::json!(["BRUTEX_MAX_STOP_POINTS"]))
    );
    assert!(!body.contains("not-an-integer"));
    assert!(!configured.get());
    assert!(site.sweep.lock().expect("slot").is_none());
    assert!(!site.store_root.join("results").exists());
}

#[test]
fn strict_terminal_audit_requires_written_and_preserves_original_evidence_text() {
    for emitted in [
        telemetry::Emitted::Written,
        telemetry::Emitted::Filtered,
        telemetry::Emitted::Dropped,
        telemetry::Emitted::NotInstalled,
    ] {
        for original in [
            Ok("saved exact result".to_owned()),
            Err("source changed".to_owned()),
        ] {
            let mut progress =
                Progress::started("zerodha", "NIFTY", (2025, 4), (2025, 5), None, 1, 96);
            settle_strict_result(&mut progress, original, 2);
            let prior_report = progress.report.clone();
            let prior_refusal = progress.refusal.clone();
            strict_terminal_audit(&mut progress, emitted);
            assert_eq!(progress.attempt, 96);
            assert_eq!(progress.finished_micros, Some(2));
            assert!(!progress.in_flight());
            if emitted == telemetry::Emitted::Written {
                assert_eq!(progress.report, prior_report);
                assert_eq!(progress.refusal, prior_refusal);
            } else {
                assert!(progress.report.is_none());
                let why = progress.refusal.as_deref().expect("visible audit refusal");
                assert!(why.contains("terminal audit is missing"));
                assert!(
                    why.contains(
                        prior_report
                            .as_deref()
                            .or(prior_refusal.as_deref())
                            .expect("original outcome")
                    )
                );
                assert!(why.contains("cannot assert a durable terminal event"));
                assert_eq!(completion_audit(&progress).outcome, "refused");
            }
        }
    }
}

#[test]
fn strict_and_ordinary_commands_share_one_guarded_task_and_terminal_finish() {
    let production = include_str!("sweeprun.rs")
        .split_once("\n#[cfg(test)]")
        .expect("production boundary")
        .0;
    let command = production
        .split_once("fn command_with_configuration(")
        .expect("actual configured command entry")
        .1
        .split_once("\nfn strict_terminal_audit(")
        .expect("end of the command producer")
        .0;
    let (before, queued) = command
        .split_once("tokio::task::spawn_blocking(move || {")
        .expect("one queued command task");
    assert_eq!(before.matches("TaskFinisher::audited").count(), 1);
    assert_eq!(queued.matches("TaskFinisher::audited").count(), 0);
    assert_eq!(queued.matches("match strict.as_ref()").count(), 1);
    assert!(
        queued.contains("conduct_strict_command(&asked, started, attempt, config, &store_root)")
    );
    assert!(queued.contains("None => conduct_command(&asked, started, attempt),"));
    assert!(queued.contains(
        "crate::booleanlaunch::conduct(request, admission, started, attempt, &launch_site)"
    ));
    let (completed, _) = queued
        .split_once("guard.finish(done);")
        .expect("both command branches reach the same normal disarm");
    assert_eq!(queued.matches("guard.finish(done);").count(), 1);
    assert!(completed.contains("command_terminal_audit(&asked, &mut done, emitted);"));
    assert!(completed.contains("done.finished_micros = Some(now_micros());"));
}

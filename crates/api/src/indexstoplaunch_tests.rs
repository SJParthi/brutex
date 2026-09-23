#![cfg(test)]
//! Exact launch protocol and shared writer/audit boundary regressions.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "private protocol fixtures must assert their exact values"
)]
use super::*;
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

fn body() -> Value {
    json!({"command":COMMAND,"feed":"zerodha","index":"NSE-NIFTY","timeframes":["1min","3min"],
        "from_year":2025,"from_month":4,"to_year":2025,"to_month":5,
        "later_from_year":2025,"later_from_month":6,"later_to_year":2025,"later_to_month":8,
        "bits":"52,53","max_loss_points":"5","batch_programs":"2","node_allowance":"128","batch_allowance":"1",
        "expected_policy_digest":hex(&[7;32])})
}
struct Scratch(std::path::PathBuf);
impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "brutex-index-stop-launch-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&root).expect("private fixture");
        Self(root)
    }
    fn site(&self) -> crate::server::Loaded {
        Arc::new(crate::server::Site::load(
            &self.0.join("absent-masters"),
            &self.0,
        ))
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _cleanup = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn every_selected_subset_and_both_indices_preserve_exact_native_scope() {
    for index in ["NSE-NIFTY", "NSE-BANKNIFTY"] {
        for mask in 1_u16..=255 {
            let timeframes: Vec<_> = cli::EVERY_RUNG
                .iter()
                .enumerate()
                .filter(|(slot, _)| mask & (1 << slot) != 0)
                .map(|(_, label)| *label)
                .collect();
            let mut value = body();
            value["index"] = json!(index);
            value["timeframes"] = json!(timeframes);
            let asked = parse(&value.to_string()).unwrap();
            let input = asked.input().unwrap();
            assert_eq!(input.timeframes, timeframes);
            assert_eq!(input.index, index);
            assert_eq!(input.training, ((2025, 4), (2025, 5)));
            assert_eq!(input.later, ((2025, 6), (2025, 8)));
            assert_eq!(input.max_loss_points, 5);
            assert_eq!(Status::new(&asked).fields()["request"], value);
            let command = crate::sweeprun::command_from(&value.to_string()).unwrap();
            assert_eq!(command.word(), COMMAND);
            assert_eq!(command.underlying(), index);
            assert_eq!(command.window(), asked.window());
        }
    }
    let asked = parse(&body().to_string()).unwrap();
    let refused = crate::sweeprun::conduct_command(
        &crate::sweeprun::command_from(&body().to_string()).unwrap(),
        1,
        2,
    );
    assert!(
        refused
            .refusal
            .unwrap()
            .contains("prepared audited browser dispatch")
    );
    assert_eq!(asked.input().unwrap().expected_policy_digest, [7; 32]);
}

#[test]
fn missing_ambiguous_and_grid_fields_never_become_single_stop_execution() {
    let initial = body();
    for field in initial.as_object().unwrap().keys() {
        let mut missing = initial.clone();
        missing.as_object_mut().unwrap().remove(field);
        assert!(parse(&missing.to_string()).is_err(), "missing {field}");
        let mut null = initial.clone();
        null[field] = Value::Null;
        assert!(parse(&null.to_string()).is_err(), "null {field}");
    }
    for unknown in [
        "symbols",
        "horizon_bars",
        "max_points",
        "stop_points",
        "targets",
        "trail_points",
        "capital",
        "rung",
        "support_ppm",
        "policy_file",
        "root",
        "output",
        "costs",
        "limit",
    ] {
        let mut changed = initial.clone();
        changed[unknown] = json!("1");
        assert!(parse(&changed.to_string()).is_err(), "{unknown}");
    }
    assert!(parse(&initial.to_string().replacen('{', "{\"bits\":\"all\",", 1)).is_err());
    assert!(parse(&format!("{initial}{}", " ".repeat(REQUEST_BYTES))).is_err());
    for index in [
        "NIFTY",
        "BANKNIFTY",
        "NSE-RELIANCE",
        "NSE-INDIAVIX",
        "BSE-SENSEX",
        "NIFTY-FUT",
        " NSE-NIFTY",
        "NSE-NIFTY,NSE-BANKNIFTY",
    ] {
        let mut changed = initial.clone();
        changed["index"] = json!(index);
        assert!(parse(&changed.to_string()).is_err(), "{index}");
    }
    for value in [
        json!([]),
        json!(["3min", "1min"]),
        json!(["1min", "1min"]),
        json!(["1day"]),
        json!(["all"]),
        json!(["4min"]),
        json!([null]),
        json!("1min"),
    ] {
        let mut changed = initial.clone();
        changed["timeframes"] = value;
        assert!(parse(&changed.to_string()).is_err(), "{changed}");
    }
}

#[test]
fn unsafe_numbers_spans_alphabets_feeds_and_policy_fingerprints_refuse() {
    for field in [
        "max_loss_points",
        "batch_programs",
        "node_allowance",
        "batch_allowance",
    ] {
        for invalid in [
            json!(0),
            json!(1),
            json!(false),
            json!("0"),
            json!("01"),
            json!("+1"),
            json!("1.0"),
            json!("1e3"),
            json!("1 "),
            json!("18446744073709551616"),
        ] {
            let mut changed = body();
            changed[field] = invalid;
            assert!(parse(&changed.to_string()).is_err(), "{field}: {changed}");
        }
    }
    for (field, raw) in [
        ("max_loss_points", "184467440737095517"),
        ("bits", "53,52"),
        ("bits", "52,52"),
        ("bits", "052"),
        ("bits", "999"),
        ("bits", "52,"),
        ("bits", "All"),
        ("feed", "ZERODHA"),
        ("feed", "../zerodha"),
    ] {
        let mut changed = body();
        changed[field] = json!(raw);
        assert!(parse(&changed.to_string()).is_err(), "{field}: {raw}");
    }
    for (field, value) in [
        ("from_year", 0),
        ("from_month", 0),
        ("to_month", 13),
        ("later_from_month", 5),
        ("later_to_year", 2024),
    ] {
        let mut changed = body();
        changed[field] = json!(value);
        assert!(parse(&changed.to_string()).is_err(), "{field}: {value}");
    }
    for invalid in [
        "0".repeat(64),
        "A".repeat(64),
        "g".repeat(64),
        "f".repeat(63),
        String::new(),
    ] {
        let mut changed = body();
        changed["expected_policy_digest"] = json!(invalid);
        assert!(parse(&changed.to_string()).is_err());
    }
    let mut maximum = body();
    maximum["node_allowance"] = json!(u64::MAX.to_string());
    assert_eq!(
        parse(&maximum.to_string())
            .unwrap()
            .input()
            .unwrap()
            .node_allowance,
        u64::MAX
    );
}

fn observed(batches: u64, exhausted: bool) -> cli::index_stop_search::Progress {
    cli::index_stop_search::Progress {
        identity: [9; 32],
        completed_batches: batches,
        exhausted,
        qualifications: [None; 8],
        completed_programs: batches * 2,
        completed_work: batches * 100,
        current_batch: None,
        rung_stages: [None; 8],
        latest_saved: None,
    }
}
#[test]
fn status_is_atomic_exact_and_never_turns_invocation_completion_into_exhaustion() {
    let asked = parse(&body().to_string()).unwrap();
    let mut status = Status::new(&asked);
    assert!(status.fields()["exhausted"].is_null());
    assert!(status.fields()["search_identity"].is_null());
    status.observe(observed(0, false)).unwrap();
    let mut next = observed(1, false);
    next.qualifications[0] = Some(([1; 32], [2; 32]));
    next.qualifications[2] = Some(([3; 32], [4; 32]));
    next.rung_stages[0] = Some(cli::index_stop_search::RungStage::Saved);
    next.rung_stages[2] = Some(cli::index_stop_search::RungStage::Saved);
    status.observe(next.clone()).unwrap();
    let prior = status.clone();
    for bad in 0..8 {
        let mut changed = next.clone();
        match bad {
            0 => changed.identity = [0; 32],
            1 => changed.identity = [8; 32],
            2 => changed.completed_batches = 0,
            3 => changed.qualifications[0] = None,
            4 => changed.qualifications[1] = Some(([1; 32], [2; 32])),
            5 => changed.qualifications[0] = Some(([0; 32], [2; 32])),
            6 => changed.qualifications[0] = Some(([1; 32], [0; 32])),
            _ => changed.exhausted = true,
        }
        assert!(status.observe(changed).is_err());
        assert_eq!(status, prior);
    }
    let mut run = crate::sweeprun::Progress::started(
        "zerodha",
        "NSE-NIFTY",
        asked.window().0,
        asked.window().1,
        None,
        1,
        u64::MAX,
    );
    run.index_stop = Some(Box::new(status.clone()));
    run.finished_micros = Some(2);
    run.report = Some("bounded invocation finished".into());
    let wire: Value = serde_json::from_str(&run.to_json()).unwrap();
    assert_eq!(wire["command"], COMMAND);
    assert_eq!(wire["index_stop"]["exhausted"], false);
    assert_eq!(wire["index_stop"]["completed_batches"], "1");
    assert_eq!(wire["attempt_key"], u64::MAX.to_string());
    assert_eq!(wire["index_stop"]["elapsed_micros"], "1");
    assert_eq!(wire["index_stop"]["qualifications"][1]["timeframe"], "3min");
    assert_eq!(wire["index_stop"]["latest_saved_batch"]["batch"], "0");
    assert!(wire.get("boolean_search").is_none());
    status.observe(observed(2, true)).unwrap();
    assert!(status.observe(observed(3, false)).is_err());
    assert!(Status::new(&asked).observe(observed(0, true)).is_err());
}

#[test]
fn next_pending_and_empty_complete_batches_retain_the_exact_prior_saved_table_label() {
    use cli::index_stop_search::RungStage;
    let asked = parse(&body().to_string()).unwrap();
    let mut status = Status::new(&asked);
    let mut complete = observed(1, false);
    for slot in [0, 2] {
        complete.qualifications[slot] = Some(([1; 32], [2; 32]));
        complete.rung_stages[slot] = Some(RungStage::Saved);
    }
    status.observe(complete).unwrap();
    let saved = status.fields()["latest_saved_batch"].clone();
    assert_eq!(saved["batch"], "0");
    assert_eq!(saved["qualifications"].as_array().unwrap().len(), 2);
    let mut pending = observed(1, false);
    pending.current_batch = Some(1);
    pending.rung_stages[0] = Some(RungStage::Preparing);
    pending.rung_stages[2] = Some(RungStage::Preparing);
    status.observe(pending).unwrap();
    assert_eq!(status.fields()["latest_saved_batch"], saved);
    assert!(status.fields()["qualifications"][0]["identity"].is_null());
    status.observe(observed(2, true)).unwrap();
    assert_eq!(status.fields()["latest_saved_batch"], saved);
    assert!(status.fields()["current_batch"].is_null());
}

#[test]
fn fresh_pending_observation_restores_only_the_verified_complete_prior_table() {
    use cli::index_stop_search::{CompletedBatch, RungStage};
    let asked = parse(&body().to_string()).unwrap();
    let mut status = Status::new(&asked);
    let mut pending = observed(1, false);
    pending.current_batch = Some(1);
    let mut saved = CompletedBatch {
        batch: 0,
        qualifications: [None; 8],
    };
    for rung in [0, 2] {
        pending.rung_stages[rung] = Some(RungStage::Preparing);
        saved.qualifications[rung] = Some(([1; 32], [2; 32]));
    }
    pending.latest_saved = Some(saved.clone());
    status.observe(pending.clone()).unwrap();
    assert_eq!(status.fields()["latest_saved_batch"]["batch"], "0");
    assert!(status.fields()["qualifications"][0]["identity"].is_null());
    let prior = status.clone();
    for malformed in 0..4 {
        let mut changed = pending.clone();
        let saved = changed.latest_saved.as_mut().unwrap();
        match malformed {
            0 => saved.batch = 1,
            1 => saved.qualifications[0] = None,
            2 => saved.qualifications[1] = Some(([1; 32], [2; 32])),
            _ => saved.qualifications[0] = Some(([3; 32], [4; 32])),
        }
        assert!(status.observe(changed).is_err());
        assert_eq!(status, prior);
    }
}

#[test]
fn stages_advance_without_claiming_new_completed_counts_or_a_preparation_identity() {
    use cli::index_stop_search::{Observation, RungStage};
    let asked = parse(&body().to_string()).unwrap();
    let mut status = Status::new(&asked);
    status
        .observe_event(Observation::Preparing {
            rung: 0,
            stage: RungStage::Preparing,
        })
        .unwrap();
    assert!(status.fields()["search_identity"].is_null());
    assert_eq!(status.fields()["qualifications"][0]["stage"], "preparing");
    assert!(
        status
            .observe_event(Observation::Preparing {
                rung: 1,
                stage: RungStage::Preparing
            })
            .is_err()
    );
    assert!(
        status
            .observe_event(Observation::Preparing {
                rung: 0,
                stage: RungStage::Training
            })
            .is_err()
    );
    status.observe(observed(0, false)).unwrap();
    let mut active = observed(0, false);
    active.current_batch = Some(0);
    active.rung_stages[0] = Some(RungStage::Preparing);
    active.rung_stages[2] = Some(RungStage::Preparing);
    status.observe(active.clone()).unwrap();
    for stage in [
        RungStage::Training,
        RungStage::Later,
        RungStage::Institutional,
        RungStage::Saved,
    ] {
        active.rung_stages[0] = Some(stage);
        status.observe(active.clone()).unwrap();
        assert_eq!(status.fields()["completed_batches"], "0");
        assert_eq!(status.fields()["completed_programs"], "0");
        assert_eq!(status.fields()["current_batch"], "0");
        assert!(status.fields()["qualifications"][0]["identity"].is_null());
    }
    let prior = status.clone();
    active.rung_stages[0] = Some(RungStage::Training);
    assert!(status.observe(active.clone()).is_err());
    assert_eq!(status, prior);
    active.rung_stages[0] = Some(RungStage::Saved);
    active.completed_work = 1;
    assert!(status.observe(active).is_err());
    assert_eq!(status, prior);
    assert!(
        status
            .observe_event(Observation::Preparing {
                rung: 0,
                stage: RungStage::Preparing
            })
            .is_err()
    );
    let mut run = crate::sweeprun::Progress::started(
        "zerodha",
        "NSE-NIFTY",
        asked.window().0,
        asked.window().1,
        None,
        3,
        9,
    );
    run.finished_micros = Some(2);
    run.index_stop = Some(Box::new(status));
    let wire: Value = serde_json::from_str(&run.to_json()).unwrap();
    assert!(wire["index_stop"]["elapsed_micros"].is_null());
}

#[test]
fn new_command_keeps_existing_build_worker_and_external_lease_gates() {
    let root = Scratch::new();
    let site = root.site();
    let (status, _, reply) = crate::sweeprun::command_with(&site, &body().to_string(), None);
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert!(reply.contains("commit"));
    assert!(site.sweep.lock().unwrap().is_none());
    assert!(!root.0.join("audit/invocations-v1").exists());
    *site.sweep.lock().unwrap() = Some(crate::sweeprun::Progress::started(
        "zerodha",
        "NIFTY",
        (2025, 4),
        (2025, 5),
        None,
        1,
        89,
    ));
    for _ in 0..2 {
        let (code, _, reply) = crate::sweeprun::command_with(
            &site,
            &body().to_string(),
            Some("0123456789abcdef0123456789abcdef01234567"),
        );
        assert_eq!(code, StatusCode::CONFLICT);
        assert!(reply.contains("already running"));
        assert_eq!(site.sweep.lock().unwrap().as_ref().unwrap().attempt, 89);
    }
    *site.sweep.lock().unwrap() = None;
    let lease = cli::execution_lease::Lease::acquire(&root.0).unwrap();
    let (code, _, _) = crate::sweeprun::command_with(
        &site,
        &body().to_string(),
        Some("0123456789abcdef0123456789abcdef01234567"),
    );
    assert_eq!(code, StatusCode::CONFLICT);
    assert!(site.sweep.lock().unwrap().is_none());
    assert!(!root.0.join("audit/invocations-v1").exists());
    drop(lease);
}

#[test]
fn missing_terminal_audit_refuses_success_but_keeps_index_observations() {
    let asked = parse(&body().to_string()).unwrap();
    let command = crate::sweeprun::command_from(&body().to_string()).unwrap();
    for emitted in [
        telemetry::Emitted::Written,
        telemetry::Emitted::Filtered,
        telemetry::Emitted::Dropped,
        telemetry::Emitted::NotInstalled,
    ] {
        let mut run = crate::sweeprun::Progress::started(
            "zerodha",
            "NSE-NIFTY",
            asked.window().0,
            asked.window().1,
            None,
            1,
            9,
        );
        run.index_stop = Some(Box::new(Status::new(&asked)));
        run.report = Some("bounded research invocation finished".into());
        crate::sweeprun::command_terminal_audit(&command, &mut run, emitted);
        assert_eq!(
            run.refusal.is_some(),
            emitted != telemetry::Emitted::Written
        );
        assert_eq!(run.report.is_none(), emitted != telemetry::Emitted::Written);
        assert!(run.index_stop.is_some());
        if let Some(why) = run.refusal {
            assert!(why.contains("terminal audit is missing"));
        }
    }
}

const CHILD: &str = "BRUTEX_INDEX_STOP_LAUNCH_TEST_CHILD";
const CHILD_TEST: &str = "indexstoplaunch::tests::configured_launch_admits_only_exact_requests_and_records_missing_source_refusal";

#[test]
fn configured_launch_admits_only_exact_requests_and_records_missing_source_refusal()
-> Result<(), String> {
    if let Some(root) = std::env::var_os(CHILD) {
        configuration_child(Path::new(&root));
        return Ok(());
    }
    let root = Scratch::new();
    let policy = root.0.join("policy.toml");
    std::fs::write(
        &policy,
        include_str!("../../../config/intraday-research-v1.toml"),
    )
    .unwrap();
    let log = root.0.join("child.log");
    let output = std::fs::File::create_new(&log).unwrap();
    let mut command = std::process::Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", CHILD_TEST, "--nocapture", "--test-threads=1"])
        .env_clear();
    if let Some(profile) = std::env::var_os("LLVM_PROFILE_FILE") {
        command.env("LLVM_PROFILE_FILE", profile);
    }
    for (name, value) in [
        ("BRUTEX_CHECKSUM_MAX_BYTES", "67108864"),
        ("BRUTEX_CHECKSUM_MAX_RECORDS", "3000000"),
        ("BRUTEX_BOOLEAN_OBSERVATION_BYTES", "268435456"),
        ("BRUTEX_BOOLEAN_SEARCH_REPLAY_NODES", "6000000"),
        ("BRUTEX_INDEX_STOP_MAX_LOSS_POINTS", "5"),
        ("BRUTEX_INDEX_STOP_BATCH_PROGRAMS", "2"),
        ("BRUTEX_INDEX_STOP_NODE_ALLOWANCE", "128"),
        ("BRUTEX_INDEX_STOP_BATCH_ALLOWANCE", "1"),
    ] {
        command.env(name, value);
    }
    let mut child = command
        .env(CHILD, &root.0)
        .env("BRUTEX_STORE", &root.0)
        .env("BRUTEX_CHECKSUM_RECEIPTS", &root.0)
        .env("BRUTEX_ADMISSION_POLICY_FILE", &policy)
        .stdout(output.try_clone().unwrap())
        .stderr(output)
        .spawn()
        .unwrap();
    let start = std::time::Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if start.elapsed() > std::time::Duration::from_secs(45)
            || std::fs::metadata(&log).unwrap().len() > 1024 * 1024
        {
            child.kill().unwrap();
            child.wait().unwrap();
            return Err("private launch child exceeded45seconds/1MiB".into());
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    };
    let output = std::fs::read_to_string(log).unwrap();
    assert!(status.success(), "{output}");
    Ok(())
}

fn configuration_child(root: &Path) {
    let metadata = configuration::value(root, None);
    assert_eq!(metadata["ready"], true, "{metadata}");
    assert_eq!(metadata["execution_policy"], "signal_candle_stop_v1");
    assert!(metadata["work_model"]["estimated_seconds"].is_null());
    assert!(metadata["configured"].get("horizon_bars").is_none());
    assert_ne!(
        metadata["policy"]["digest"],
        configuration::value(root, Some("max_loss_points=6"))["policy"]["digest"]
    );
    assert_eq!(
        configuration::value(root, Some("max_loss_points=5&max_loss_points=6"))["ready"],
        false
    );
    let mismatch = parse(&body().to_string()).unwrap();
    assert!(
        prepare(&mismatch, root)
            .err()
            .unwrap()
            .contains("policy changed")
    );
    let site = Arc::new(crate::server::Site::load(
        &root.join("absent-masters"),
        root,
    ));
    let mut exact = body();
    exact["expected_policy_digest"] = metadata["policy"]["digest"].clone();
    let asked = parse(&exact.to_string()).unwrap();
    let admission = prepare(&asked, root).unwrap();
    assert_eq!(admission.input(), &asked.input().unwrap());
    assert!(!root.join("index-stop-search-v1").exists());
    assert!(!root.join("audit/invocations-v1").exists());
    let mut altered = asked.clone();
    altered.index = "NSE-BANKNIFTY".into();
    let refused = conduct(&altered, admission, 1, 4, &site);
    assert!(
        refused
            .refusal
            .unwrap()
            .contains("request changed after admission")
    );
    assert!(!root.join("index-stop-search-v1").exists());
    telemetry::install(&telemetry::Config::new(root.join("logs"))).unwrap();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let (code, _, reply) = crate::sweeprun::command_with(
            &site,
            &exact.to_string(),
            Some("0123456789abcdef0123456789abcdef01234567"),
        );
        assert_eq!(code, StatusCode::ACCEPTED, "{reply}");
        let accepted: Value = serde_json::from_str(&reply).unwrap();
        let attempt = accepted["attempt"]
            .as_str()
            .unwrap()
            .parse::<u64>()
            .unwrap();
        let start = std::time::Instant::now();
        loop {
            let progress = site.sweep.lock().unwrap().clone().unwrap();
            if !progress.in_flight() {
                assert_eq!(progress.attempt, attempt);
                assert!(progress.refusal.is_some());
                assert!(progress.report.is_none());
                assert!(progress.index_stop.unwrap().fields()["search_identity"].is_null());
                break;
            }
            assert!(start.elapsed() < std::time::Duration::from_secs(15));
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        let saved = cli::operation_audit::read(root, attempt).unwrap().unwrap();
        assert_eq!(saved.phase, cli::operation_audit::Phase::Refused);
        assert_eq!(saved.label, COMMAND);
        assert!(!root.join("index-stop-search-v1").exists());
    });
}

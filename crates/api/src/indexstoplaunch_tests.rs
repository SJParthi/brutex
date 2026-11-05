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
/// Built from `module_path!()` so a renamed or moved module cannot strand it
/// (P1-12-03); the crate's own name is dropped because the harness omits it.
const CHILD_PATH: &str = concat!(
    module_path!(),
    "::configured_launch_admits_only_exact_requests_and_records_missing_source_refusal"
);

#[test]
fn configured_launch_admits_only_exact_requests_and_records_missing_source_refusal()
-> Result<(), String> {
    if let Some(root) = std::env::var_os(CHILD) {
        configuration_child(Path::new(&root));
        return Ok(());
    }
    let root = Scratch::new();
    run_child(CHILD_PATH, &launch_env(&root.0, CHILD, "268435456", "128"));
    Ok(())
}

/// The environment of a configured launch child over `root`: the checksum,
/// observation, replay and single-stop settings, the admission policy file
/// written into `root`, and `marker` naming the child to run, set to `root`.
/// `observation_bytes` is `BRUTEX_BOOLEAN_OBSERVATION_BYTES` and
/// `node_allowance` is `BRUTEX_INDEX_STOP_NODE_ALLOWANCE`.
fn launch_env(
    root: &Path,
    marker: &'static str,
    observation_bytes: &'static str,
    node_allowance: &'static str,
) -> Vec<(&'static str, std::ffi::OsString)> {
    let policy = root.join("policy.toml");
    std::fs::write(
        &policy,
        include_str!("../../../config/intraday-research-v1.toml"),
    )
    .unwrap();
    let mut env: Vec<(&'static str, std::ffi::OsString)> = [
        ("BRUTEX_CHECKSUM_MAX_BYTES", "67108864"),
        ("BRUTEX_CHECKSUM_MAX_RECORDS", "3000000"),
        ("BRUTEX_BOOLEAN_OBSERVATION_BYTES", observation_bytes),
        ("BRUTEX_BOOLEAN_SEARCH_REPLAY_NODES", "6000000"),
        ("BRUTEX_INDEX_STOP_MAX_LOSS_POINTS", "5"),
        ("BRUTEX_INDEX_STOP_BATCH_PROGRAMS", "2"),
        ("BRUTEX_INDEX_STOP_NODE_ALLOWANCE", node_allowance),
        ("BRUTEX_INDEX_STOP_BATCH_ALLOWANCE", "1"),
    ]
    .into_iter()
    .map(|(name, value)| (name, value.into()))
    .collect();
    for name in [marker, "BRUTEX_STORE", "BRUTEX_CHECKSUM_RECEIPTS"] {
        env.push((name, root.into()));
    }
    env.push(("BRUTEX_ADMISSION_POLICY_FILE", policy.into()));
    env
}

/// Runs the test at `path` (a `module_path!()` path, crate name first) alone
/// in a child process of this binary with exactly `env` (and the coverage
/// profile, when one is written), its output in a log beside the marker's
/// root. The child is killed when this returns or panics, so one that
/// outlives 45 seconds or writes past 1 MiB fails the assertion and is gone.
fn run_child(path: &str, env: &[(&'static str, std::ffi::OsString)]) {
    struct Reaped(std::process::Child);
    impl Drop for Reaped {
        fn drop(&mut self) {
            let _killed = self.0.kill();
            let _reaped = self.0.wait();
        }
    }
    let root = Path::new(
        &env.iter()
            .find(|(name, _)| *name == "BRUTEX_STORE")
            .unwrap()
            .1,
    );
    let log = root.join("child.log");
    let output = std::fs::File::create_new(&log).unwrap();
    let child_test = path.split_once("::").map_or(path, |(_, path)| path);
    let mut command = std::process::Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", child_test, "--nocapture", "--test-threads=1"])
        .env_clear()
        .envs(std::env::var_os("LLVM_PROFILE_FILE").map(|profile| ("LLVM_PROFILE_FILE", profile)))
        .envs(env.iter().map(|(name, value)| (name, value)))
        .stdout(output.try_clone().unwrap())
        .stderr(output);
    let mut child = Reaped(command.spawn().unwrap());
    let start = std::time::Instant::now();
    let status = loop {
        if let Some(status) = child.0.try_wait().unwrap() {
            break status;
        }
        assert!(
            start.elapsed() <= std::time::Duration::from_secs(45)
                && std::fs::metadata(&log).unwrap().len() <= 1024 * 1024,
            "private launch child exceeded 45 seconds or 1 MiB"
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
    };
    let output = std::fs::read_to_string(log).unwrap();
    assert!(status.success(), "{output}");
    // THE CHILD MUST PROVE IT RAN (P1-12-03): `--exact` on a name that
    // matches nothing exits zero, which a status check alone accepts.
    assert!(
        output.contains("test result: ok. 1 passed;"),
        "the child must run exactly this one test:\n{output}"
    );
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

/// THE OBSERVER REFUSES EVERY INCOHERENT STEP BY NAME AND KEEPS NOTHING OF
/// IT, AND ACCEPTS THE COHERENT STEPS AT ITS EDGES: a worker that reports out
/// of order is stopped at the step it got wrong, never recorded.
///
/// Extremes: singles — a search reported through the event channel
/// (accepted), a refused rung asked to prepare again (refused) and refused
/// twice (accepted), fewer programs or less work than acknowledged, more
/// programs in the same batch count, an exhausted search reported again
/// unchanged (accepted), an initial declaration claiming links, programs or
/// work, a reservation that is not the next batch, a link on a rung nobody
/// selected, a zero link identity, a zero link pin, a stage on an unselected
/// rung, a reserved selected rung with no stage, a stage with no link, a link
/// whose rung is not saved, a retained table holding a zero link, an older
/// retained table, a newer one (accepted); combinations — links and a
/// retained table at once whose ordinal is not the last batch, then whose
/// tables differ, then that agree (accepted), and a refused rung that moves
/// back to training inside its own reservation (two stage ranks at once);
/// out of the box — the search itself delivered as an observation event,
/// where stages are expected.
#[test]
fn every_incoherent_observation_is_refused_by_name_and_its_coherent_edges_pass() {
    use cli::index_stop_search::{CompletedBatch, Observation, Progress, RungStage};
    let asked = parse(&body().to_string()).unwrap();
    // Rungs 0 and 2 are the selected `1min` and `3min`; rung 1 is not.
    let link = |seed: u8| Some(([seed; 32], [seed + 1; 32]));
    let saved_pair = |next: &mut Progress| {
        for (rung, seed) in [(0, 1), (2, 3)] {
            next.qualifications[rung] = link(seed);
            next.rung_stages[rung] = Some(RungStage::Saved);
        }
    };
    let retained = |batch: u64, seed: u8| {
        let mut table = CompletedBatch {
            batch,
            qualifications: [None; 8],
        };
        table.qualifications[0] = link(seed);
        table.qualifications[2] = link(seed + 2);
        table
    };
    let refuses = |status: &Status, next: Progress, why: &str| {
        let mut tried = status.clone();
        let said = tried.observe(next).unwrap_err();
        assert!(said.contains(why), "{said}");
        assert_eq!(&tried, status, "a refused step keeps nothing");
    };
    let regressed = "single-stop identity or acknowledged progress changed or regressed";
    let initial = "single-stop initial declaration cannot claim completed research";
    let foreign = "single-stop progress omitted a selected timeframe or contains a foreign/invalid qualification";
    let scope = "single-stop stage does not match its exact selected scope";
    let boundary = "single-stop saved boundary does not match its qualification link";
    let pinned = "single-stop retained batch identity or pinned qualifications changed";

    let mut searched = Status::new(&asked);
    searched
        .observe_event(Observation::Search(observed(1, false)))
        .unwrap();
    assert_eq!(searched.fields()["completed_batches"], "1");

    let mut preparing = Status::new(&asked);
    for _ in 0..2 {
        preparing
            .observe_event(Observation::Preparing {
                rung: 0,
                stage: RungStage::Refused,
            })
            .unwrap();
    }
    assert_eq!(preparing.fields()["qualifications"][0]["stage"], "refused");
    let restarted = preparing
        .observe_event(Observation::Preparing {
            rung: 0,
            stage: RungStage::Preparing,
        })
        .unwrap_err();
    assert_eq!(
        restarted,
        "single-stop refused source cannot silently restart"
    );

    let mut base = Status::new(&asked);
    base.observe(observed(1, false)).unwrap();
    let mut fewer = observed(1, false);
    fewer.completed_programs = 1;
    refuses(&base, fewer, regressed);
    let mut less = observed(1, false);
    less.completed_work = 99;
    refuses(&base, less, regressed);
    let mut more = observed(1, false);
    more.completed_programs = 3;
    refuses(&base, more, regressed);

    let mut done = Status::new(&asked);
    done.observe(observed(2, true)).unwrap();
    let before = done.clone();
    done.observe(observed(2, true)).unwrap();
    assert_eq!(
        done, before,
        "an unchanged exhausted search is the same status"
    );

    let fresh = Status::new(&asked);
    let mut linked = observed(0, false);
    saved_pair(&mut linked);
    refuses(&fresh, linked, initial);
    let mut programs = observed(0, false);
    programs.completed_programs = 1;
    refuses(&fresh, programs, initial);
    let mut work = observed(0, false);
    work.completed_work = 1;
    refuses(&fresh, work, initial);
    let mut ahead = observed(1, false);
    ahead.current_batch = Some(3);
    refuses(
        &fresh,
        ahead,
        "single-stop current batch is not its next acknowledged reservation",
    );

    let mut unselected = observed(1, false);
    saved_pair(&mut unselected);
    unselected.qualifications[1] = link(5);
    refuses(&fresh, unselected, foreign);
    for zero in [Some(([0; 32], [2; 32])), Some(([1; 32], [0; 32]))] {
        let mut zeroed = observed(1, false);
        saved_pair(&mut zeroed);
        zeroed.qualifications[0] = zero;
        refuses(&fresh, zeroed, foreign);
    }
    let mut stray = observed(1, false);
    stray.rung_stages[1] = Some(RungStage::Training);
    refuses(&fresh, stray, scope);
    let mut unstaged = observed(1, false);
    unstaged.current_batch = Some(1);
    unstaged.rung_stages[2] = Some(RungStage::Training);
    refuses(&fresh, unstaged, scope);
    let mut unlinked = observed(1, false);
    unlinked.rung_stages[0] = Some(RungStage::Saved);
    refuses(&fresh, unlinked, boundary);
    let mut unsaved = observed(1, false);
    saved_pair(&mut unsaved);
    unsaved.rung_stages[0] = Some(RungStage::Institutional);
    refuses(&fresh, unsaved, boundary);

    let mut reserved = observed(1, false);
    reserved.current_batch = Some(1);
    reserved.rung_stages[0] = Some(RungStage::Refused);
    reserved.rung_stages[2] = Some(RungStage::Preparing);
    let mut running = Status::new(&asked);
    running.observe(reserved.clone()).unwrap();
    reserved.rung_stages[0] = Some(RungStage::Training);
    refuses(
        &running,
        reserved,
        "single-stop timeframe stage regressed within its reserved batch",
    );

    let pending = |batches: u64, table: CompletedBatch| {
        let mut next = observed(batches, false);
        next.current_batch = Some(batches);
        next.rung_stages[0] = Some(RungStage::Preparing);
        next.rung_stages[2] = Some(RungStage::Preparing);
        next.latest_saved = Some(table);
        next
    };
    let mut zero_kept = retained(0, 1);
    zero_kept.qualifications[2] = Some(([0; 32], [9; 32]));
    refuses(
        &fresh,
        pending(2, zero_kept),
        "single-stop retained table is incomplete or has foreign timeframe links",
    );
    let mut kept = Status::new(&asked);
    let mut complete = observed(2, false);
    saved_pair(&mut complete);
    kept.observe(complete).unwrap();
    assert_eq!(kept.fields()["latest_saved_batch"]["batch"], "1");
    refuses(&kept, pending(2, retained(0, 1)), pinned);
    kept.observe(pending(3, retained(2, 1))).unwrap();
    assert_eq!(kept.fields()["latest_saved_batch"]["batch"], "2");

    let mut both = observed(3, false);
    saved_pair(&mut both);
    both.latest_saved = Some(retained(0, 1));
    refuses(&fresh, both.clone(), pinned);
    both.latest_saved = Some(retained(2, 5));
    refuses(&fresh, both.clone(), pinned);
    both.latest_saved = Some(retained(2, 1));
    let mut agreed = Status::new(&asked);
    agreed.observe(both).unwrap();
    assert_eq!(agreed.fields()["latest_saved_batch"]["batch"], "2");
}

/// A FOREIGN COMMAND AND A NUMBER WITH NO DIGITS OR TOO MANY ARE REFUSED
/// BEFORE ANY CONFIGURATION IS READ.
///
/// Extremes: singles — the grid command, an empty command, an empty count,
/// a count of 21 digits, the largest count of 20 digits (accepted), the
/// smallest count (accepted); combinations — an empty count and a foreign
/// command at once (the command is named first), 21 digits in every count
/// at once; out of the box — a command that is the native one with a
/// trailing space.
#[test]
fn a_foreign_command_and_a_count_of_no_or_too_many_digits_are_refused_before_any_read() {
    let command = "single-stop launch requires its exact native command";
    for foreign in ["index-stop-grid", "", &format!("{COMMAND} ")] {
        let mut changed = body();
        changed["command"] = json!(foreign);
        changed["batch_programs"] = json!("");
        assert_eq!(parse(&changed.to_string()).unwrap_err(), command);
    }
    for (raw, accepted) in [
        ("", false),
        ("123456789012345678901", false),
        ("18446744073709551615", true),
        ("1", true),
    ] {
        let mut changed = body();
        changed["node_allowance"] = json!(raw);
        let parsed = parse(&changed.to_string());
        assert_eq!(parsed.is_ok(), accepted, "{raw}: {parsed:?}");
        if !accepted {
            let said = parsed.unwrap_err();
            assert_eq!(
                said, "node_allowance requires a canonical positive decimal string",
                "{raw}"
            );
        }
    }
    let mut every = body();
    for field in [
        "max_loss_points",
        "batch_programs",
        "node_allowance",
        "batch_allowance",
    ] {
        every[field] = json!("123456789012345678901");
    }
    assert_eq!(
        parse(&every.to_string()).unwrap_err(),
        "batch_programs requires a canonical positive decimal string"
    );
}

/// THE CONFIGURATION PREVIEW DOOR answers JSON with its refusal when nothing
/// is configured, never an empty or plain answer.
#[tokio::test]
async fn the_configuration_preview_door_answers_json_when_nothing_is_configured() {
    let root = Scratch::new();
    let _apart = crate::detail::apart_from_slot_owners().await;
    let uri: Uri = "/index-stop/launch.json?max_loss_points=5".parse().unwrap();
    let (status, headers, reply) = metadata(axum::extract::State(root.site()), uri).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers[0].1, "application/json; charset=utf-8");
    let value: Value = serde_json::from_str(&reply).unwrap();
    assert_eq!(value["ready"], false, "{value}");
    assert_eq!(value["configured"]["max_loss_points"], "5", "{value}");
    assert!(
        value["refusal"]
            .as_str()
            .unwrap()
            .contains("batch_programs is not configured; set BRUTEX_INDEX_STOP_BATCH_PROGRAMS"),
        "{value}"
    );
}

const BOUNDS_CHILD: &str = "BRUTEX_INDEX_STOP_BOUNDS_TEST_CHILD";
const BOUNDS_CHILD_PATH: &str = concat!(
    module_path!(),
    "::a_preview_past_its_observation_records_or_store_bounds_names_each_one"
);

/// A PREVIEW PAST ITS BOUNDS NAMES EACH ONE: an observation admission below
/// what the saved comparison needs, a node allowance past the record bound,
/// and a serving store that is relative or a file. Run in a child, since the
/// bounds come from the environment.
///
/// Extremes: singles — 1,024 observation bytes against the 268,435,456 the
/// comparison needs, a node allowance one past the 3,000,000-record bound, a
/// batch whose two directions fill that bound exactly (no refusal of its
/// own), a batch one program past it (refused before any bound is read), a
/// relative store, a store that is a file; combinations — the small admission
/// and the oversized allowance at once (both named, in order), a relative
/// store with both at once (three named); out of the box — a store path that
/// is the empty string.
#[test]
fn a_preview_past_its_observation_records_or_store_bounds_names_each_one() {
    if let Some(root) = std::env::var_os(BOUNDS_CHILD) {
        bounds_child(Path::new(&root));
        return;
    }
    let root = Scratch::new();
    run_child(
        BOUNDS_CHILD_PATH,
        &launch_env(&root.0, BOUNDS_CHILD, "1024", "3000001"),
    );
}

fn bounds_child(root: &Path) {
    let small = "Saved single-stop comparison needs observation admission of at least 268435456 bytes; BRUTEX_BOOLEAN_OBSERVATION_BYTES currently admits 1024";
    let records = "node_allowance exceeds the admitted native record bound";
    let store = "The serving store is not an existing absolute directory";
    let refusal = |root: &Path, query: Option<&str>| {
        let value = configuration::value(root, query);
        assert_eq!(value["ready"], false, "{value}");
        value["refusal"].as_str().unwrap().to_owned()
    };
    let both = format!("{records}; {small}");
    assert_eq!(refusal(root, None), both);
    assert_eq!(refusal(root, Some("batch_programs=1500000")), both);
    assert_eq!(
        refusal(root, Some("batch_programs=1500001")),
        "single-stop batch must admit both directions for every program"
    );
    let file = root.join("a-file");
    std::fs::write(&file, b"not a store").unwrap();
    for wrong in [Path::new("relative-store"), file.as_path(), Path::new("")] {
        assert_eq!(
            refusal(wrong, None),
            format!("{both}; {store}"),
            "{wrong:?}"
        );
    }
}

const MOVED_CHILD: &str = "BRUTEX_INDEX_STOP_MOVED_TEST_CHILD";
const MOVED_CHILD_PATH: &str = concat!(
    module_path!(),
    "::a_launch_whose_slot_moved_and_a_preview_that_cannot_run_are_refused_by_name"
);

/// A LAUNCH WHOSE SLOT MOVED, AND A PREVIEW THAT CANNOT RUN, ARE REFUSED BY
/// NAME: the replay bounds a configuration must fit, the active slot a
/// running launch reports into (another attempt, or the same attempt for
/// another request), and the preview door with its pool full or its work
/// lost. Run in a child: the bounds come from the environment, and a crowded
/// pool or lost work must not reach another test.
///
/// Extremes: singles — the exact configuration (accepted), bootstrap work,
/// split work and numeric memory each one past the replay admission, a slot
/// holding another attempt, a slot holding this attempt for the other index,
/// a full pool, admitted work that dies; combinations — all three replay
/// bounds past at once (named once), a full pool asked twice in a row; out of
/// the box — a slot that holds this attempt with no single-stop status at
/// all.
#[test]
fn a_launch_whose_slot_moved_and_a_preview_that_cannot_run_are_refused_by_name() {
    if let Some(root) = std::env::var_os(MOVED_CHILD) {
        moved_child(Path::new(&root));
        return;
    }
    let root = Scratch::new();
    run_child(
        MOVED_CHILD_PATH,
        &launch_env(&root.0, MOVED_CHILD, "268435456", "128"),
    );
}

fn moved_child(root: &Path) {
    let exact = cli::index_stop_search::configuration(5, 2).unwrap();
    configuration::observation_limits(&exact).unwrap();
    let replay = "Saved single-stop comparison must fit independent cold numerical replay admission before launch: bootstrap work 6000000, CSCV work 6000000, estimated numeric-memory bytes 268435456; these are work/byte bounds, not latency or RSS guarantees";
    for past in 0..4 {
        let mut config = cli::index_stop_search::configuration(5, 2).unwrap();
        let bounds = &mut config.qualification;
        match past {
            0 => bounds.bootstrap_work = 6_000_001,
            1 => bounds.split_work = 6_000_001,
            2 => bounds.memory_bytes = 268_435_457,
            _ => {
                bounds.bootstrap_work = u64::MAX;
                bounds.split_work = u64::MAX;
                bounds.memory_bytes = u64::MAX;
            }
        }
        assert_eq!(
            configuration::observation_limits(&config).unwrap_err(),
            replay,
            "{past}"
        );
    }

    let site = Arc::new(crate::server::Site::load(
        &root.join("absent-masters"),
        root,
    ));
    let mut exact_body = body();
    exact_body["expected_policy_digest"] =
        configuration::value(root, None)["policy"]["digest"].clone();
    let asked = parse(&exact_body.to_string()).unwrap();
    let mut other_index = asked.clone();
    other_index.index = "NSE-BANKNIFTY".into();
    for (holder, expected) in [
        (
            7,
            None,
            "single-stop invocation lost its exact active status; no further batch starts",
        ),
        (
            8,
            Some(other_index),
            "single-stop active request changed; no further batch starts",
        ),
        (
            8,
            None,
            "single-stop active request changed; no further batch starts",
        ),
    ]
    .map(|(attempt, request, why)| {
        let mut held = crate::sweeprun::Progress::started(
            "zerodha",
            "NSE-NIFTY",
            asked.window().0,
            asked.window().1,
            None,
            1,
            attempt,
        );
        held.index_stop = request.map(|request| Box::new(Status::new(&request)));
        (held, why)
    }) {
        *site.sweep.lock().unwrap() = Some(holder);
        let moved = conduct(&asked, prepare(&asked, root).unwrap(), 1, 8, &site);
        let why = moved.refusal.unwrap();
        let named = cli::commit_stamp()
            .map_or("single-stop search requires a clean build identity", |_| {
                expected
            });
        assert!(why.contains(named), "{why}");
        assert!(moved.report.is_none());
    }

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let ask = || {
            metadata(
                axum::extract::State(site.clone()),
                Uri::from_static("/index-stop/launch.json"),
            )
        };
        let crowd = crate::detail::crowd(crate::detail::Named::Detail);
        for _ in 0..2 {
            let (status, _, full) = ask().await;
            assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
            let value: Value = serde_json::from_str(&full).unwrap();
            assert_eq!(value["ready"], false);
            assert_eq!(
                value["refusal"],
                "bounded configuration read unavailable: Saturated"
            );
        }
        drop(crowd);
        crate::detail::lost_work::lose_admitted_work(true);
        let (status, _, lost) = ask().await;
        crate::detail::lost_work::lose_admitted_work(false);
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        let value: Value = serde_json::from_str(&lost).unwrap();
        assert!(
            value["refusal"]
                .as_str()
                .unwrap()
                .starts_with("bounded configuration read unavailable: Join("),
            "{value}"
        );
        let (status, _, answered) = ask().await;
        assert_eq!(status, StatusCode::OK, "{answered}");
    });
}

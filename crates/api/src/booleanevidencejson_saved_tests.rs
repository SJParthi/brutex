#![cfg(test)]
//! SAVED BOOLEAN EVIDENCE, MADE BY THE PRODUCTION LAUNCH OVER A GENERATED STORE,
//! READ BACK THROUGH THE THREE EVIDENCE READERS (L-BCA-5).
//!
//! The generated NIFTY months are `indexstopqualificationjson`'s; the search,
//! its statistics, admission and qualification are what `/backtest/command`
//! starts for `boolean-qualified-search-stored` (`sweeprun::command_with`).
//! Nothing here writes an artifact by hand. A build with no commit stamp cannot
//! run that path, which is the one thing this test asserts there.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "a generated fixture and an exact projection must fail the test by name"
)]

use super::*;
use std::sync::Arc;

/// Set in the child that runs the launch, over the store it names.
const CHILD: &str = "BRUTEX_SAVED_BOOLEAN_EVIDENCE_TEST_CHILD";
const CHILD_PATH: &str = concat!(
    module_path!(),
    "::saved_boolean_evidence_is_served_by_every_reader_that_reads_it"
);

/// The environment of the launch child over `root`.
fn launch_env(root: &Path) -> Vec<(&'static str, std::ffi::OsString)> {
    let policy = root.join("policy.toml");
    std::fs::write(
        &policy,
        include_str!("../../../config/intraday-research-v1.toml"),
    )
    .expect("the policy file");
    let mut env: Vec<(&'static str, std::ffi::OsString)> = [
        ("BRUTEX_CHECKSUM_MAX_BYTES", "2147483648"),
        ("BRUTEX_CHECKSUM_MAX_RECORDS", "3000000"),
        ("BRUTEX_BOOLEAN_OBSERVATION_BYTES", "8589934592"),
        ("BRUTEX_BOOLEAN_SEARCH_REPLAY_NODES", "6000000"),
        ("BRUTEX_BOOLEAN_HORIZON_BARS", "5"),
        ("BRUTEX_BOOLEAN_MAX_POINTS", "5"),
        ("BRUTEX_BOOLEAN_BATCH_PROGRAMS", "2"),
        ("BRUTEX_BOOLEAN_NODE_ALLOWANCE", "128"),
        ("BRUTEX_BOOLEAN_BATCH_ALLOWANCE", "1"),
        // The search recurses deeper than a 2 MiB test thread at opt-level 0.
        ("RUST_MIN_STACK", "268435456"),
    ]
    .into_iter()
    .map(|(name, value)| (name, value.into()))
    .collect();
    for name in [CHILD, "BRUTEX_STORE", "BRUTEX_CHECKSUM_RECEIPTS"] {
        env.push((name, root.into()));
    }
    env.push(("BRUTEX_ADMISSION_POLICY_FILE", policy.into()));
    env
}

/// The launch request: NIFTY at one minute, May 2025 for training and June
/// for the later days, over the bullish and bearish bar bits (30, 31), which
/// the generated minutes, closing 50 paisa above or below their open, keep
/// firing.
fn launch_body(digest: &str) -> String {
    serde_json::json!({"command":crate::booleanlaunch::COMMAND,"feed":"zerodha","symbols":["NIFTY"],
        "timeframes":["1min"],"from_year":2025,"from_month":5,"to_year":2025,"to_month":5,
        "later_from_year":2025,"later_from_month":6,"later_to_year":2025,"later_to_month":6,
        "bits":"30,31","horizon_bars":"5","max_points":"5","batch_programs":"2",
        "node_allowance":"128","batch_allowance":"1","expected_policy_digest":digest})
    .to_string()
}

/// The one saved artifact under `namespace` in `root`: its identity, in hex.
fn saved_in(root: &Path, namespace: &str) -> String {
    let mut found: Vec<String> = std::fs::read_dir(root.join(namespace))
        .expect("the namespace")
        .map(|entry| {
            entry
                .expect("an entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .filter(|name| name.len() == 64)
        .collect();
    found.sort();
    found.first().cloned().expect("one saved artifact")
}

/// One page of an evidence reader, as JSON with its status.
async fn page(model: &str, query: &str) -> (StatusCode, Value) {
    let uri: Uri = format!("/boolean/{model}.json?{query}")
        .parse()
        .expect("a URI");
    let (status, _, body) = match model {
        "statistics" => statistics_json(uri).await,
        "admission" => admission_json(uri).await,
        _ => qualification_json(uri).await,
    };
    (status, serde_json::from_str(&body).expect("JSON"))
}

/// **SAVED BOOLEAN EVIDENCE IS SERVED BY EVERY READER THAT READS IT.** Run in
/// a child, since every bound comes from the environment.
#[test]
fn saved_boolean_evidence_is_served_by_every_reader_that_reads_it() {
    let Some(root) = std::env::var_os(CHILD) else {
        let root = crate::scratch::path("saved-boolean-evidence");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("a scratch root");
        crate::indexstopqualificationjson::saved_tests::generated_nifty(&root);
        let log = root.join("child.log");
        let output = std::fs::File::create_new(&log).expect("a log");
        let child_test = CHILD_PATH
            .split_once("::")
            .map_or(CHILD_PATH, |(_, path)| path);
        let status = std::process::Command::new(std::env::current_exe().expect("this binary"))
            .args(["--exact", child_test, "--nocapture", "--test-threads=1"])
            .env_clear()
            .envs(
                std::env::var_os("LLVM_PROFILE_FILE").map(|profile| ("LLVM_PROFILE_FILE", profile)),
            )
            .envs(launch_env(&root))
            .stdout(output.try_clone().expect("a second handle"))
            .stderr(output)
            .status()
            .expect("the child runs");
        let printed = std::fs::read_to_string(&log).expect("the child's log");
        assert!(status.success(), "{printed}");
        assert!(printed.contains("test result: ok. 1 passed;"), "{printed}");
        let _ = std::fs::remove_dir_all(&root);
        return;
    };
    let root = PathBuf::from(root);
    let site = Arc::new(crate::server::Site::load(
        &root.join("absent-masters"),
        &root,
    ));
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("a runtime");
    runtime.block_on(async {
        let (status, _, preview) = crate::booleanlaunch::metadata(
            axum::extract::State(Arc::clone(&site)),
            Uri::from_static("/boolean/launch.json"),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{preview}");
        let preview: Value = serde_json::from_str(&preview).expect("JSON");
        let digest = preview["policy"]["digest"].as_str().expect("a digest");
        telemetry::install(&telemetry::Config::new(root.join("logs"))).expect("a log sink");
        let body = launch_body(digest);
        let (code, _, reply) = crate::sweeprun::command_with(&site, &body, cli::commit_stamp());
        let Some(_stamp) = cli::commit_stamp() else {
            return assert_eq!(code, StatusCode::SERVICE_UNAVAILABLE, "{reply}");
        };
        assert_eq!(code, StatusCode::ACCEPTED, "{reply}");
        let run = loop {
            let progress = site.sweep.lock().unwrap().clone().expect("the run's slot");
            if !progress.in_flight() {
                break progress;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        };
        assert!(run.refusal.is_none(), "{:?}", run.refusal);
        let qualification = saved_in(&root, "boolean-qualification-v1");
        let (status, settings) = page("qualification", &format!("identity={qualification}")).await;
        assert_eq!(status, StatusCode::OK, "{settings}");
    });
}

#![cfg(test)]
//! A SAVED NATIVE QUALIFICATION, MADE BY THE PRODUCTION LAUNCH OVER A GENERATED
//! STORE, READ BACK THROUGH THE READERS THAT SERVE IT (L-BCA-5).
//!
//! Generated prices only, never market evidence: a NIFTY index month whose
//! minutes cycle through a fixed pattern around 20,000.00, written with the
//! store's own writer. The search, the strict source load, the qualification
//! and its daily assessments are all the production path `/backtest/command`
//! starts (`sweeprun::command_with`); nothing here writes an artifact by hand.
//! A build with no commit stamp cannot run that path at all, which is the one
//! thing this test asserts there.
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
const CHILD: &str = "BRUTEX_SAVED_QUALIFICATION_TEST_CHILD";
const CHILD_PATH: &str = concat!(
    module_path!(),
    "::a_saved_qualification_is_served_by_every_page_that_reads_it"
);

/// Generated NIFTY minutes and days of April to June 2025 under `zerodha`,
/// every open session of the shared calendar, written by the store's writer.
fn generated_nifty(root: &Path) {
    let hash = brutex_core::universe::fnv1a("NIFTY").to_le_bytes();
    let symbol = u32::from_le_bytes(hash[..4].try_into().expect("low32"));
    for month in [4_u8, 5, 6] {
        let mut minutes = Vec::new();
        let mut daily = Vec::new();
        for day in 1..=31_u8 {
            let Ok(date) = pull::session::Day::new(2025, month, day) else {
                continue;
            };
            let stamp = i64::from(date.days_from_epoch());
            let pull::calendar::DayKind::Open(session) = pull::calendar::kind_of(stamp) else {
                continue;
            };
            let start = minutes.len();
            for window in session.windows.iter().take(usize::from(session.count)) {
                for minute in window.from..=window.to {
                    let open = 2_000_000 + i64::from((minute + u16::from(day)) % 31) * 100;
                    minutes.push(store::format::Bar {
                        ts_micros: stamp * 86_400_000_000 + i64::from(minute) * 60_000_000
                            - pull::session::IST_OFFSET_SECS * 1_000_000,
                        open,
                        high: open + 1_000,
                        low: open - 900,
                        close: open + if minute % 3 == 0 { -50 } else { 50 },
                        volume: 100,
                        open_interest: i64::MIN,
                    });
                }
            }
            let session_bars = &minutes[start..];
            daily.push(store::format::Bar {
                ts_micros: session_bars[0].ts_micros,
                open: session_bars[0].open,
                high: session_bars
                    .iter()
                    .map(|bar| bar.high)
                    .max()
                    .expect("a high"),
                low: session_bars.iter().map(|bar| bar.low).min().expect("a low"),
                close: session_bars[session_bars.len() - 1].close,
                volume: session_bars.iter().map(|bar| bar.volume).sum(),
                open_interest: i64::MIN,
            });
        }
        for (timeframe, bars) in [
            (store::path::Timeframe::MINUTE_1, &minutes),
            (store::path::Timeframe::DAY_1, &daily),
        ] {
            let path = store::path::StorePath::new(store::path::PathParts {
                vendor: brutex_core::vendor::Vendor::Zerodha,
                exchange: "NSE",
                segment: "INDEX",
                symbol: "NIFTY",
                contract: None,
                timeframe,
                month: store::path::YearMonth::new(2025, month).expect("a month"),
                file: store::path::FileKind::Bars,
            })
            .expect("a generated path");
            store::file::BarFile::open_or_create(root, path, symbol)
                .expect("an owned generated writer")
                .append(bars)
                .expect("generated bars append");
        }
    }
}

/// The environment of the launch child over `root`.
fn launch_env(root: &Path) -> Vec<(&'static str, std::ffi::OsString)> {
    let policy = root.join("policy.toml");
    std::fs::write(
        &policy,
        include_str!("../../../config/intraday-research-v1.toml"),
    )
    .expect("the policy file");
    let mut env: Vec<(&'static str, std::ffi::OsString)> = [
        ("BRUTEX_CHECKSUM_MAX_BYTES", "67108864"),
        ("BRUTEX_CHECKSUM_MAX_RECORDS", "3000000"),
        ("BRUTEX_BOOLEAN_OBSERVATION_BYTES", "268435456"),
        ("BRUTEX_BOOLEAN_SEARCH_REPLAY_NODES", "6000000"),
        ("BRUTEX_INDEX_STOP_MAX_LOSS_POINTS", "5"),
        ("BRUTEX_INDEX_STOP_BATCH_PROGRAMS", "2"),
        ("BRUTEX_INDEX_STOP_NODE_ALLOWANCE", "128"),
        ("BRUTEX_INDEX_STOP_BATCH_ALLOWANCE", "1"),
        ("BRUTEX_INDEX_STOP_BOOTSTRAP_DRAWS", "63"),
        ("BRUTEX_INDEX_STOP_WORKERS", "1"),
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
/// for the later days, over two live bits.
fn launch_body(digest: &str) -> String {
    serde_json::json!({"command":crate::indexstoplaunch::COMMAND,"feed":"zerodha","index":"NSE-NIFTY",
        "timeframes":["1min"],"from_year":2025,"from_month":5,"to_year":2025,"to_month":5,
        "later_from_year":2025,"later_from_month":6,"later_to_year":2025,"later_to_month":6,
        "bits":"52,53","max_loss_points":"5","batch_programs":"2","node_allowance":"128",
        "batch_allowance":"1","expected_policy_digest":digest})
    .to_string()
}

/// One page of the qualification reader, as JSON with its status.
async fn page(query: &str) -> (StatusCode, Value) {
    let uri: Uri = format!("/index-stop-qualification.json?{query}")
        .parse()
        .expect("a URI");
    let (status, _, body) = index_stop_qualification_json(uri).await;
    (status, serde_json::from_str(&body).expect("JSON"))
}

/// **A SAVED QUALIFICATION IS SERVED BY EVERY PAGE THAT READS IT.** Run in a
/// child, since every bound comes from the environment and the launch is
/// process-wide.
#[test]
fn a_saved_qualification_is_served_by_every_page_that_reads_it() {
    let Some(root) = std::env::var_os(CHILD) else {
        let root = crate::scratch::path("saved-qualification");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("a scratch root");
        generated_nifty(&root);
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
        let (status, _, preview) = crate::indexstoplaunch::metadata(
            axum::extract::State(Arc::clone(&site)),
            Uri::from_static("/index-stop/launch.json"),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{preview}");
        let preview: Value = serde_json::from_str(&preview).expect("JSON");
        assert_eq!(preview["ready"], true, "{preview}");
        telemetry::install(&telemetry::Config::new(root.join("logs"))).expect("a log sink");
        let body = launch_body(preview["policy"]["digest"].as_str().expect("a digest"));
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
        let fields = run.index_stop.expect("the single-stop status").fields();
        let link = &fields["qualifications"][0];
        let (identity, completion) = (
            link["identity"].as_str().expect("an identity"),
            link["completion"].as_str().expect("a completion"),
        );
        let (status, settings) = page(&format!("identity={identity}")).await;
        assert_eq!(status, StatusCode::OK, "{settings}");
        assert_eq!(settings["status"], "saved", "{settings}");
        assert!(!completion.is_empty());
    });
}

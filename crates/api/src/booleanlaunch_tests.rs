#![cfg(test)]
//! Private protocol, status and production-router launch-boundary regressions.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "exact private fixture assertions fail loudly"
)]
use super::*;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

fn body() -> Value {
    json!({"command":COMMAND,"feed":"zerodha","symbols":["NSE-NIFTY","NSE-RELIANCE"],
        "from_year":2025,"from_month":4,"to_year":2025,"to_month":5,
        "later_from_year":2025,"later_from_month":6,"later_to_year":2025,"later_to_month":8,
        "bits":"52,53","horizon_bars":"5","max_points":"5",
        "batch_programs":"2","node_allowance":"128","batch_allowance":"1",
        "expected_policy_digest":"0707070707070707070707070707070707070707070707070707070707070707"})
}
struct Scratch(std::path::PathBuf);
impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "brutex-boolean-launch-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&root).unwrap();
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
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn declared_search_dispatch_preserves_all_families_periods_and_exact_work_allowances() {
    let mut raw = body();
    raw["node_allowance"] = Value::String(u64::MAX.to_string());
    let command = crate::sweeprun::command_from(&raw.to_string()).unwrap();
    assert_eq!(command.word(), COMMAND);
    assert_eq!(command.feed(), "zerodha");
    assert_eq!(command.window(), ((2025, 4), (2025, 5)));
    let asked = parse(&raw.to_string()).unwrap();
    let arguments = asked.arguments(Path::new("/server-owned-only")).unwrap();
    assert_eq!(arguments.len(), 17);
    assert_eq!(arguments[1], "NIFTY,RELIANCE");
    assert_eq!(arguments[10], u64::MAX.to_string());
    assert_eq!(arguments[12], "/server-owned-only");
    assert_eq!(&arguments[13..], ["2025", "6", "2025", "8"]);
    assert!(!arguments.iter().any(|value| value == "1day"));
}

#[test]
fn every_nonempty_canonical_timeframe_subset_is_preserved_without_widening() {
    let legacy = parse(&body().to_string()).unwrap();
    assert_eq!(legacy.timeframes, all_timeframes());
    for mask in 1_u16..(1 << cli::EVERY_RUNG.len()) {
        let selected: Vec<_> = cli::EVERY_RUNG
            .iter()
            .enumerate()
            .filter(|(index, _)| mask & (1 << index) != 0)
            .map(|(_, rung)| *rung)
            .collect();
        let mut raw = body();
        raw["symbols"] = json!(["NSE-NIFTY"]);
        raw["timeframes"] = json!(selected);
        let asked = parse(&raw.to_string()).unwrap();
        assert_eq!(asked.timeframes, selected, "subset {mask}");
        let fields = Status::new(&asked).fields();
        assert_eq!(fields["timeframes"], raw["timeframes"]);
        assert_eq!(fields["symbols"], raw["symbols"]);
        assert_eq!(
            asked.arguments(Path::new("/server-owned-only")).unwrap()[1],
            "NIFTY"
        );
    }
    let mut explicit = body();
    explicit["timeframes"] = json!(cli::EVERY_RUNG);
    assert_eq!(parse(&explicit.to_string()).unwrap(), legacy);
}

#[test]
fn malformed_or_noncanonical_timeframes_are_refused_instead_of_defaulting_to_all() {
    for invalid in [
        Value::Null,
        json!([]),
        json!("1min"),
        json!([null]),
        json!([1]),
        json!(["1min", "1min"]),
        json!(["3min", "1min"]),
        json!(["1day"]),
        json!(["1min", "1day"]),
        json!(["4min"]),
        json!(["1MIN"]),
        json!(["1min "]),
        json!([" 1min"]),
        json!(["1min,3min"]),
        json!(["all"]),
        json!([
            "1min", "2min", "3min", "5min", "10min", "15min", "30min", "60min", "1day"
        ]),
    ] {
        let mut raw = body();
        raw["timeframes"] = invalid;
        assert!(parse(&raw.to_string()).is_err(), "{raw}");
    }
    let mut raw = body();
    raw["timeframes"] = json!(["1min"]);
    let duplicate = raw
        .to_string()
        .replacen('{', "{\"timeframes\":[\"3min\"],", 1);
    assert!(parse(&duplicate).is_err());
}

#[test]
fn launch_refuses_absence_null_duplicates_unknown_paths_and_noncanonical_or_unsafe_numbers() {
    let original = body();
    for key in original.as_object().unwrap().keys() {
        let mut missing = original.clone();
        missing.as_object_mut().unwrap().remove(key);
        assert!(parse(&missing.to_string()).is_err(), "missing {key}");
        let mut null = original.clone();
        null[key] = Value::Null;
        assert!(parse(&null.to_string()).is_err(), "null {key}");
    }
    for key in [
        "output",
        "root",
        "store_root",
        "policy_file",
        "rungs",
        "underlying",
        "support_ppm",
        "min_trades",
    ] {
        let mut unknown = original.clone();
        unknown[key] = json!("not-accepted");
        assert!(parse(&unknown.to_string()).is_err(), "unknown {key}");
    }
    for key in [
        "horizon_bars",
        "max_points",
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
            let mut altered = original.clone();
            altered[key] = invalid;
            assert!(parse(&altered.to_string()).is_err(), "{key}: {altered}");
        }
    }
    for (key, value) in [
        ("horizon_bars", "4294967296"),
        ("max_points", "184467440737095517"),
    ] {
        let mut altered = original.clone();
        altered[key] = json!(value);
        assert!(parse(&altered.to_string()).is_err());
    }
    let duplicate = original.to_string().replacen('{', "{\"bits\":\"all\",", 1);
    assert!(parse(&duplicate).is_err());
    let oversized = format!("{}{}", original, " ".repeat(REQUEST_BYTES));
    assert!(parse(&oversized).is_err());
    for digest in [
        json!(false),
        json!(""),
        json!("0".repeat(64)),
        json!("A".repeat(64)),
        json!("f".repeat(63)),
        json!("g".repeat(64)),
    ] {
        let mut altered = original.clone();
        altered["expected_policy_digest"] = digest;
        assert!(parse(&altered.to_string()).is_err());
    }
}

#[test]
fn launch_scope_never_turns_references_derivatives_alias_duplicates_or_wrong_spans_into_work() {
    for symbols in [
        json!([]),
        json!(["NSE-INDIAVIX"]),
        json!(["BSE-SENSEX"]),
        json!(["NIFTY-FUT"]),
        json!(["NIFTY", "NSE-NIFTY"]),
        json!(["NIFTY,RELIANCE"]),
        json!([" NIFTY"]),
        json!(["X".repeat(65)]),
    ] {
        let mut altered = body();
        altered["symbols"] = symbols;
        assert!(parse(&altered.to_string()).is_err(), "{altered}");
    }
    for bits in [
        "",
        "53,52",
        "52,52",
        "052",
        "999",
        "4294967296",
        "52,53,",
        "All",
    ] {
        let mut altered = body();
        altered["bits"] = json!(bits);
        assert!(parse(&altered.to_string()).is_err(), "{bits}");
    }
    for (key, value) in [
        ("from_month", 0),
        ("to_month", 13),
        ("later_from_month", 5),
        ("later_to_year", 2024),
    ] {
        let mut altered = body();
        altered[key] = json!(value);
        assert!(parse(&altered.to_string()).is_err(), "{altered}");
    }
}

#[test]
fn exact_status_separates_durable_identity_paused_work_and_exhaustion() {
    let asked = parse(&body().to_string()).unwrap();
    let mut status = Status::new(&asked);
    assert!(status.fields()["search_identity"].is_null());
    assert!(status.fields()["exhausted"].is_null());
    status
        .observe(cli::boolean_search_launch::Progress {
            identity: [7; 32],
            exhausted: None,
            completed_batches: None,
        })
        .unwrap();
    let declared = status.clone();
    assert!(
        status
            .observe(cli::boolean_search_launch::Progress {
                identity: [8; 32],
                exhausted: None,
                completed_batches: None
            })
            .is_err()
    );
    assert_eq!(status, declared);
    status
        .observe(cli::boolean_search_launch::Progress {
            identity: [7; 32],
            exhausted: Some(false),
            completed_batches: Some(1),
        })
        .unwrap();
    let paused = status.clone();
    for (exhausted, completed_batches) in
        [(Some(false), Some(0)), (None, None), (Some(false), None)]
    {
        assert!(
            status
                .observe(cli::boolean_search_launch::Progress {
                    identity: [7; 32],
                    exhausted,
                    completed_batches
                })
                .is_err()
        );
        assert_eq!(status, paused);
    }
    let mut run = crate::sweeprun::Progress::started(
        "zerodha",
        "SELECTED",
        asked.window().0,
        asked.window().1,
        None,
        1,
        (1_u64 << 63) + 1,
    )
    .of_kind(crate::sweeprun::Kind::Command);
    run.boolean_search = Some(Box::new(status.clone()));
    run.report = Some("invocation complete; grammar remains paused".into());
    run.finished_micros = Some(2);
    let actual: Value = serde_json::from_str(&run.to_json()).unwrap();
    assert_eq!(actual["command"], COMMAND);
    assert_eq!(
        actual["expected_policy_digest"],
        body()["expected_policy_digest"]
    );
    assert_eq!(actual["symbols"], body()["symbols"]);
    assert_eq!(actual["timeframes"], json!(cli::EVERY_RUNG));
    assert_eq!(actual["attempt_key"], ((1_u64 << 63) + 1).to_string());
    assert_eq!(actual["search_identity"], hex(&[7; 32]));
    assert_eq!(actual["exhausted"], false);
    assert_eq!(actual["completed_batches"], "1");
    assert_eq!(actual["later_from_month"], 6);
    status
        .observe(cli::boolean_search_launch::Progress {
            identity: [7; 32],
            exhausted: Some(true),
            completed_batches: Some(2),
        })
        .unwrap();
    assert!(
        status
            .observe(cli::boolean_search_launch::Progress {
                identity: [7; 32],
                exhausted: Some(false),
                completed_batches: Some(3)
            })
            .is_err()
    );
}

#[test]
fn repeated_launch_cannot_bypass_the_shared_active_worker_gate() {
    let root = Scratch::new();
    let site = root.site();
    let attempt = (1_u64 << 63) + 19;
    *site.sweep.lock().unwrap() = Some(crate::sweeprun::Progress::started(
        "zerodha",
        "NIFTY",
        (2025, 4),
        (2025, 5),
        None,
        1,
        attempt,
    ));
    for _ in 0..2 {
        let (code, _, reply) = crate::sweeprun::command_with(
            &site,
            &body().to_string(),
            Some("0123456789abcdef0123456789abcdef01234567"),
        );
        assert_eq!(code, StatusCode::CONFLICT);
        assert!(reply.contains("already running"));
        assert_eq!(
            site.sweep.lock().unwrap().as_ref().unwrap().attempt,
            attempt
        );
        assert!(!root.0.join("audit/invocations-v1").exists());
    }
}

#[test]
fn boolean_terminal_audit_failure_refuses_success_without_erasing_saved_evidence() {
    let command = crate::sweeprun::command_from(&body().to_string()).unwrap();
    for emitted in [
        telemetry::Emitted::Written,
        telemetry::Emitted::Filtered,
        telemetry::Emitted::Dropped,
        telemetry::Emitted::NotInstalled,
    ] {
        for refused in [false, true] {
            let mut progress = crate::sweeprun::Progress::started(
                "zerodha",
                "SELECTED",
                (2025, 4),
                (2025, 5),
                None,
                1,
                55,
            );
            progress.finished_micros = Some(2);
            if refused {
                progress.refusal = Some("source evidence refused".into());
            } else {
                progress.report = Some("saved research remains incomplete".into());
            }
            let prior = progress
                .refusal
                .clone()
                .or_else(|| progress.report.clone())
                .unwrap();
            crate::sweeprun::command_terminal_audit(&command, &mut progress, emitted);
            if emitted == telemetry::Emitted::Written {
                assert_eq!(progress.refusal.is_some(), refused);
                assert_eq!(
                    progress
                        .refusal
                        .as_ref()
                        .or(progress.report.as_ref())
                        .unwrap(),
                    &prior
                );
            } else {
                assert!(progress.report.is_none());
                let why = progress.refusal.unwrap();
                assert!(why.contains("terminal audit is missing"));
                assert!(why.contains(&*prior));
            }
        }
    }
}

const CONFIG_CHILD: &str = "BRUTEX_BOOLEAN_LAUNCH_CONFIG_CHILD";
const CONFIG_TEST: &str = "booleanlaunch::tests::explicit_configuration_accepts_the_declared_launch_and_preserves_refused_terminal_status";

/// A child process killed and reaped when this guard drops: on every way
/// out of a test, a pass and a failing assert alike.
struct KillOnDrop(std::process::Child);
impl Drop for KillOnDrop {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn explicit_configuration_accepts_the_declared_launch_and_preserves_refused_terminal_status() {
    if let Some(root) = std::env::var_os(CONFIG_CHILD) {
        configuration_child(Path::new(&root));
        return;
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
    command.args(["--exact", CONFIG_TEST, "--nocapture", "--test-threads=1"]);
    command.env_clear();
    // A coverage run's profile path reaches the child; an ordinary run has none.
    command
        .envs(std::env::var_os("LLVM_PROFILE_FILE").map(|profile| ("LLVM_PROFILE_FILE", profile)));
    let child = command
        .env(CONFIG_CHILD, &root.0)
        .env("BRUTEX_STORE", &root.0)
        .env("BRUTEX_CHECKSUM_RECEIPTS", &root.0)
        .env("BRUTEX_CHECKSUM_MAX_BYTES", "67108864")
        .env("BRUTEX_CHECKSUM_MAX_RECORDS", "3000000")
        .env("BRUTEX_ADMISSION_POLICY_FILE", &policy)
        .env("BRUTEX_BOOLEAN_OBSERVATION_BYTES", "268435456")
        .env("BRUTEX_BOOLEAN_SEARCH_REPLAY_NODES", "3000000")
        .stdout(output.try_clone().unwrap())
        .stderr(output)
        .spawn()
        .unwrap();
    // Killed on every way out, so a child past its bound never outlives the test.
    let mut child = KillOnDrop(child);
    let start = std::time::Instant::now();
    let status = loop {
        if let Some(status) = child.0.try_wait().unwrap() {
            break status;
        }
        assert!(
            start.elapsed() <= std::time::Duration::from_secs(45)
                && std::fs::metadata(&log).unwrap().len() <= 1024 * 1024,
            "private configuration child exceeded 45s/1MiB"
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
    };
    assert!(std::fs::metadata(&log).unwrap().len() <= 1024 * 1024);
    let output = std::fs::read_to_string(log).unwrap();
    assert!(status.success(), "{output}");
    let metadata = output
        .lines()
        .find_map(|line| {
            line.split_once("BRUTEX_BOOLEAN_LAUNCH_METADATA=")
                .map(|(_, json)| json)
        })
        .unwrap();
    println!("BRUTEX_BOOLEAN_LAUNCH_METADATA={metadata}");
}

fn configuration_child(root: &Path) {
    let metadata = metadata_value(root, Some("max_points=5"));
    assert_eq!(metadata["ready"], true, "{metadata}");
    assert_eq!(metadata["selected_timeframes_supported"], true);
    println!("BRUTEX_BOOLEAN_LAUNCH_METADATA={metadata}");
    assert_eq!(metadata["configured"]["max_points"], "5");
    for key in [
        "horizon_bars",
        "batch_programs",
        "node_allowance",
        "batch_allowance",
    ] {
        assert!(metadata["configured"][key].is_null());
    }
    let values = metadata["policy"]["values"].as_array().unwrap();
    assert_eq!(values.len(), 39);
    assert_eq!(
        values
            .iter()
            .filter(|row| row["value"].is_boolean())
            .count(),
        2
    );
    assert!(
        values
            .iter()
            .all(|row| row["value"].is_string() || row["value"].is_boolean())
    );
    assert_ne!(metadata["policy"]["digest"], "0".repeat(64));
    let other = metadata_value(root, Some("max_points=6"));
    assert_eq!(other["ready"], true);
    assert_eq!(other["configured"]["max_points"], "6");
    assert_ne!(metadata["policy"]["digest"], other["policy"]["digest"]);
    assert_eq!(
        metadata_value(root, Some("max_points=5&max_points=6"))["ready"],
        false
    );
    let mismatch = parse(&body().to_string()).unwrap();
    let why = prepare(&mismatch, root).err().unwrap();
    assert!(why.contains("displayed research policy changed"));
    let fail_site = Arc::new(crate::server::Site::load(
        &root.join("absent-masters"),
        root,
    ));
    let (code, _, reply) = crate::sweeprun::command_with(
        &fail_site,
        &body().to_string(),
        Some("0123456789abcdef0123456789abcdef01234567"),
    );
    assert_eq!(code, StatusCode::SERVICE_UNAVAILABLE);
    assert!(reply.contains("displayed research policy changed"));
    assert!(fail_site.sweep.lock().unwrap().is_none());
    let mut exact = body();
    exact["expected_policy_digest"] = metadata["policy"]["digest"].clone();
    let asked = parse(&exact.to_string()).unwrap();
    let admission = prepare(&asked, root).unwrap();
    assert_eq!(
        metadata["policy"]["digest"],
        hex(&admission.configuration.policy_digest)
    );
    assert!(!root.join("audit/invocations-v1").exists());
    assert!(!root.join("boolean-qualified-search-v1").exists());
    drop(admission);
    let mut selected = exact.clone();
    selected["timeframes"] = json!(["3min", "15min"]);
    let selected = parse(&selected.to_string()).unwrap();
    let admission = prepare(&selected, root).unwrap();
    assert_eq!(admission.rungs(), ["3min", "15min"]);
    let mut altered = selected.clone();
    altered.timeframes = vec!["1min".into()];
    let refused = conduct(&altered, admission, 1, 1, &fail_site);
    assert!(
        refused
            .refusal
            .as_deref()
            .is_some_and(|why| why.contains("timeframe selection changed after admission"))
    );
    assert!(refused.report.is_none());
    assert!(!root.join("boolean-qualified-search-v1").exists());
    telemetry::install(&telemetry::Config::new(root.join("logs"))).unwrap();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(accepted_child(root));
}

async fn accepted_child(root: &Path) {
    let site = Arc::new(crate::server::Site::load(
        &root.join("absent-masters"),
        root,
    ));
    let mut exact = body();
    exact["timeframes"] = json!(["3min", "15min"]);
    exact["expected_policy_digest"] =
        metadata_value(root, Some("max_points=5"))["policy"]["digest"].clone();
    // The API's existing pure admission stamp seam does not bypass the native
    // executor's real compiler stamp or admit any absent historical source.
    let (code, _, reply) = crate::sweeprun::command_with(
        &site,
        &exact.to_string(),
        Some("0123456789abcdef0123456789abcdef01234567"),
    );
    assert_eq!(code, StatusCode::ACCEPTED, "{reply}");
    let value: Value = serde_json::from_str(&reply).unwrap();
    let attempt = value["attempt"].as_str().unwrap().parse::<u64>().unwrap();
    assert!(attempt > cli::operation_audit::ID_BASE);
    let started = std::time::Instant::now();
    loop {
        let run = site.sweep.lock().unwrap().clone().unwrap();
        assert_eq!(run.attempt, attempt);
        if !run.in_flight() {
            break;
        }
        assert!(started.elapsed() < std::time::Duration::from_secs(10));
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    let uri = format!("/backtest/run.json?attempt={attempt}")
        .parse()
        .unwrap();
    let (code, _, status) =
        crate::sweeprun::run_json(axum::extract::State(site.clone()), uri).await;
    assert_eq!(code, StatusCode::OK);
    let status: Value = serde_json::from_str(&status).unwrap();
    let run = &status["running"];
    assert_eq!(run["attempt_key"], attempt.to_string());
    assert_eq!(run["command"], COMMAND);
    assert_eq!(run["symbols"], body()["symbols"]);
    assert_eq!(run["timeframes"], exact["timeframes"]);
    assert_eq!(
        run["expected_policy_digest"],
        exact["expected_policy_digest"]
    );
    assert_eq!(run["in_flight"], false);
    assert!(run["report"].is_null());
    assert!(run["refusal"].as_str().is_some_and(|why| !why.is_empty()));
    assert!(run["search_identity"].is_null());
    assert!(run["exhausted"].is_null());
    let saved = cli::operation_audit::read(root, attempt).unwrap().unwrap();
    assert_eq!(saved.phase, cli::operation_audit::Phase::Refused);
    assert_eq!(saved.label, COMMAND);
    assert!(!root.join("boolean-qualified-search-v1").exists());
}

#[tokio::test]
async fn production_metadata_route_audits_only_its_http_outcome_without_launching_work() {
    use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
    // THE AUDITED ROUTER JOURNALS THIS REQUEST THROUGH A DETAIL SLOT, so it is
    // kept apart from the tests that hold every slot. A probe that held every
    // slot got 429 from this same request on each of three runs, and 200 once
    // the slots were released. D-0695.
    let _apart = crate::detail::apart_from_slot_owners().await;
    let root = Scratch::new();
    let site = root.site();
    let sentinel = root.0.join("retained-source.bin");
    std::fs::write(&sentinel, b"private unchanged sentinel").unwrap();
    let front = Arc::new(crate::assets::Assets::new(&root.0.join("web")));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let app = crate::server::audited_router_serving(Arc::clone(&site), front, address);
    let (stop, stopped) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = stopped.await;
            })
            .await
            .unwrap();
    });
    let mut socket = tokio::net::TcpStream::connect(address).await.unwrap();
    socket.write_all(format!("GET /engine/boolean-launch.json?max_points=0 HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n\r\n").as_bytes()).await.unwrap();
    let mut reply = String::new();
    socket.read_to_string(&mut reply).await.unwrap();
    assert!(reply.starts_with("HTTP/1.1 200"), "{reply}");
    assert!(reply.contains("x-brutex-request-audit:"));
    let value: Value = serde_json::from_str(reply.split_once("\r\n\r\n").unwrap().1).unwrap();
    assert_eq!(value["ready"], false);
    assert_eq!(value["timeframes"], json!(cli::EVERY_RUNG));
    assert_eq!(value["selected_timeframes_supported"], true);
    assert!(value["refusal"].as_str().unwrap().contains("max_points"));
    assert!(site.sweep.lock().unwrap().is_none());
    let rows = cli::operation_audit::page(&root.0, None, 32).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].label, "GET /engine/boolean-launch.json");
    assert_eq!(rows[0].phase, cli::operation_audit::Phase::Completed);
    assert_eq!(
        std::fs::read(&sentinel).unwrap(),
        b"private unchanged sentinel"
    );
    assert!(!root.0.join("boolean-qualified-search-v1").exists());
    stop.send(()).unwrap();
    server.await.unwrap();
}

/// **EACH LAUNCH FIELD AT ITS OWN EDGE IS REFUSED IN ITS OWN WORDS, THE
/// WHOLE ALPHABET IS ONE WORD, AND AN EXHAUSTED SEARCH MAY BE SEEN AGAIN.**
/// The refusal tests above assert only that a body is refused; these name
/// the sentence each edge earns, and the two acceptances beside them.
///
/// Extremes: (1) another command; (2) one symbol past `max_symbols`; (3) an
/// empty symbol; (4) a symbol holding a control character; (5) an alphabet
/// one bit list past 4,096 bytes; (6) a body one byte past `REQUEST_BYTES`;
/// (7) a digest of 64 zeros; (8) a digest in upper case; (9) the alphabet
/// `all` is accepted and kept as `all`; (10) a count of no digits and one of
/// 21 digits are no positive numbers, while 20 digits are; (11) a relative
/// store root and an absolute one that does not exist are each said to be
/// no store by the metadata route; (12) a metadata query naming anything
/// but `max_points` is refused by its own sentence. Combined: (13) another
/// command beside an empty symbol is refused once, by the symbol list's
/// sentence; (14) an exhausted search observed again, with the same
/// identity and count, is kept as it was, while the same search seen
/// unexhausted after it is refused. Out of the box: (15) a body of exactly
/// `REQUEST_BYTES` bytes of space is past the size check and refused by the
/// schema instead.
#[test]
fn each_launch_field_at_its_edge_is_refused_in_its_own_words() {
    const SYMBOLS: &str = "Boolean launch requires a bounded exact research symbol list";
    const NO_STORE: &str = "The serving store is not an existing absolute directory";
    let refused = |altered: &Value| parse(&altered.to_string()).expect_err("refused");
    let mut other = body();
    other["command"] = json!("boolean-qualified-search-elsewhere");
    assert_eq!(refused(&other), SYMBOLS);
    let mut crowd = body();
    crowd["symbols"] = json!(
        (0..=cli::boolean_search_launch::max_symbols())
            .map(|n| format!("NSE-INVENTED{n}"))
            .collect::<Vec<_>>()
    );
    assert_eq!(refused(&crowd), SYMBOLS);
    for symbol in ["", "NSE-NIF\u{7}TY"] {
        let mut odd = body();
        odd["symbols"] = json!(["NSE-NIFTY", symbol]);
        assert_eq!(refused(&odd), SYMBOLS, "{symbol:?}");
    }
    let mut both = other.clone();
    both["symbols"] = json!([""]);
    assert_eq!(refused(&both), SYMBOLS);

    let mut long = body();
    long["bits"] = json!(
        (0..1_100)
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join(",")
    );
    assert_eq!(
        refused(&long),
        "Boolean alphabet exceeds its bounded protocol size"
    );
    let oversized = " ".repeat(REQUEST_BYTES + 1);
    assert_eq!(
        parse(&oversized).expect_err("oversized"),
        format!("Boolean launch request exceeds {REQUEST_BYTES} bytes; no prefix accepted")
    );
    let spaces = " ".repeat(REQUEST_BYTES);
    assert!(
        parse(&spaces)
            .expect_err("no schema")
            .starts_with("Boolean launch schema refused: "),
        "a body of exactly the bound passes the size check"
    );
    for digest in ["0".repeat(64), "0a".repeat(32).to_uppercase()] {
        let mut altered = body();
        altered["expected_policy_digest"] = json!(digest);
        assert_eq!(
            refused(&altered),
            "expected_policy_digest requires the displayed nonzero lowercase policy fingerprint"
        );
    }
    let mut whole = body();
    whole["bits"] = json!("all");
    let asked = parse(&whole.to_string()).expect("the whole alphabet");
    assert_eq!(asked.bits, "all");

    let mut status = Status::new(&asked);
    let done = cli::boolean_search_launch::Progress {
        identity: [9; 32],
        exhausted: Some(true),
        completed_batches: Some(3),
    };
    status.observe(done).expect("an exhausted search");
    let exhausted = status.clone();
    status
        .observe(done)
        .expect("the same exhaustion seen again");
    assert_eq!(status, exhausted);
    assert_eq!(
        status.observe(cli::boolean_search_launch::Progress {
            identity: [9; 32],
            exhausted: Some(false),
            completed_batches: Some(3),
        }),
        Err("declared search identity or terminal evidence changed".to_owned())
    );
    assert_eq!(status, exhausted);

    assert_eq!(positive(""), None);
    assert_eq!(positive(&"1".repeat(21)), None);
    assert_eq!(positive(&"1".repeat(20)), Some(11_111_111_111_111_111_111));
    let refusal = |root: &Path, query: Option<&str>| {
        metadata_value(root, query)["refusal"]
            .as_str()
            .unwrap_or_default()
            .to_owned()
    };
    assert!(refusal(Path::new("relative/store"), None).contains(NO_STORE));
    assert!(refusal(Path::new("/brutex-no-such-store-directory"), None).contains(NO_STORE));
    let queried = refusal(&std::env::temp_dir(), Some("limit=5"));
    assert!(
        queried.contains("metadata accepts only one exact max_points query")
            && !queried.contains(NO_STORE),
        "{queried}"
    );
}

const LIMITS_PROOF: &str = "BRUTEX_BOOLEAN_LAUNCH_LIMITS_PROOF";
const LIMITS_WANT: &str = "BRUTEX_BOOLEAN_LAUNCH_LIMITS_WANT";
const LIMITS_NOT: &str = "BRUTEX_BOOLEAN_LAUNCH_LIMITS_NOT";
const LIMITS_TEST: &str = "booleanlaunch::tests::observation_limits_too_small_or_unreadable_are_said_by_the_metadata_route";

/// **OBSERVATION LIMITS TOO SMALL FOR THE STRICT RANGE, OR UNREADABLE, ARE
/// SAID BY THE METADATA ROUTE, AND A LAUNCH WITH NO POLICY FILE IS REFUSED
/// BEFORE ITS STORE.** Each limit is read from the process environment, so
/// each case runs in a child of its own (`crate::isolated`), with the strict
/// range of 64 MiB and 3,000,000 records, and proves it ran by a file.
///
/// Extremes: (1) an observation budget of 1,024 bytes against four strict
/// windows; (2) a replay allowance of one node against 3,000,000 records;
/// (3) a budget one byte short of four windows; (4) a budget of exactly four
/// windows with replay of exactly the records says no limits sentence; (5) a
/// budget that is no number; (6) a launch with no policy file is refused in
/// `configuration`'s words before its store is touched; (7) each child's
/// store is an existing absolute folder, so no store sentence is said; (8)
/// every case's refusal names what it lacks and not what it has. Combined:
/// (9) a small budget beside a one-node replay is one limits sentence; (10)
/// an unreadable budget beside a sufficient replay is the budget's sentence
/// and no limits sentence. Out of the box: (11) a budget of `u64::MAX` bytes
/// is refused as unreadable, because the pool's aggregate would overflow.
#[test]
fn observation_limits_too_small_or_unreadable_are_said_by_the_metadata_route() {
    const LIMITS: &str =
        "Boolean dashboard observation/replay limits cannot read the admitted search";
    const BUDGET: &str =
        "BRUTEX_BOOLEAN_OBSERVATION_BYTES must be a canonical positive decimal byte count";
    const POLICY: &str = "BRUTEX_ADMISSION_POLICY_FILE is required for browser Boolean research";
    if let Some(proof) = std::env::var_os(LIMITS_PROOF) {
        let proof = std::path::PathBuf::from(proof);
        let folder = proof.parent().expect("the proof's folder");
        let want = std::env::var(LIMITS_WANT).expect("the sentence owed");
        let not = std::env::var(LIMITS_NOT).expect("the sentence not owed");
        let said = metadata_value(folder, Some("max_points=5"))["refusal"]
            .as_str()
            .unwrap_or_default()
            .to_owned();
        assert!(said.contains(&want), "owed {want:?}: {said}");
        assert!(!said.contains(&not), "not owed {not:?}: {said}");
        assert!(!said.contains(NO_STORE_SENTENCE), "{said}");
        let asked = parse(&body().to_string()).expect("the declared launch");
        assert_eq!(
            prepare(&asked, Path::new("/brutex-no-such-store-directory")).err(),
            Some(POLICY.to_owned())
        );
        std::fs::write(&proof, b"ran").expect("the child's proof");
        return;
    }
    let root = Scratch::new();
    for (case, observation, replay, want, not) in [
        ("small", "1024", "3000000", LIMITS, BUDGET),
        ("replay", "268435456", "1", LIMITS, BUDGET),
        ("short", "268435455", "3000000", LIMITS, BUDGET),
        ("exact", "268435456", "3000000", POLICY, LIMITS),
        ("unreadable", "nope", "3000000", BUDGET, LIMITS),
        ("both", "1024", "1", LIMITS, BUDGET),
        ("huge", "18446744073709551615", "3000000", BUDGET, LIMITS),
    ] {
        let proof = root.0.join(case);
        let _out = crate::isolated::rerun_unsetting(
            LIMITS_TEST,
            &[
                (LIMITS_PROOF, proof.as_os_str()),
                (LIMITS_WANT, std::ffi::OsStr::new(want)),
                (LIMITS_NOT, std::ffi::OsStr::new(not)),
                ("BRUTEX_CHECKSUM_RECEIPTS", root.0.as_os_str()),
                (
                    "BRUTEX_CHECKSUM_MAX_BYTES",
                    std::ffi::OsStr::new("67108864"),
                ),
                (
                    "BRUTEX_CHECKSUM_MAX_RECORDS",
                    std::ffi::OsStr::new("3000000"),
                ),
                (
                    "BRUTEX_BOOLEAN_OBSERVATION_BYTES",
                    std::ffi::OsStr::new(observation),
                ),
                (
                    "BRUTEX_BOOLEAN_SEARCH_REPLAY_NODES",
                    std::ffi::OsStr::new(replay),
                ),
            ],
            &["BRUTEX_ADMISSION_POLICY_FILE"],
        );
        assert_eq!(
            std::fs::read(&proof).ok().as_deref(),
            Some(&b"ran"[..]),
            "{case}: the child ran its case"
        );
    }
}

const NO_STORE_SENTENCE: &str = "The serving store is not an existing absolute directory";

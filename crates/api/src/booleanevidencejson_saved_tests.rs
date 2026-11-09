#![cfg(test)]
//! SAVED BOOLEAN EVIDENCE, MADE BY THE PRODUCTION LAUNCH OVER A GENERATED STORE,
//! READ BACK THROUGH THE THREE EVIDENCE READERS AND THE SEARCH PAGE (L-BCA-5).
//!
//! Generated prices only, never market evidence: `indexstopqualificationjson`'s
//! NIFTY minutes, and their 60min bars folded from them on the store's 09:15
//! IST grid. The search, its statistics, admission and qualification are what
//! `/backtest/command` starts for `boolean-qualified-search-stored`
//! (`sweeprun::command_with`); nothing here writes an artifact by hand. A build
//! with no commit stamp cannot run that path, which is the one thing this test
//! asserts there.
//!
//! **Why 60min and not 1min.** A saved candidate body holds a record for every
//! signal bar, and every currency check of the chain rereads and rehashes it
//! whole. Over the same forty sessions at 1min one batch saved a 550 MB body
//! and spent minutes at opt-level 3 rehashing it; at 60min a session is seven
//! signal bars, not 375.
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

/// The months of 2025 the store holds: March (read before the training
/// window's first session), April and May (training, forty sessions), June
/// and July (the later window, 44 sessions: at seven 60min bars a session,
/// the first two months whose bars outlast the 200-bar warm-up of the slow
/// trend average, so the later window holds warm signal rows; June alone,
/// 147 bars, holds none and is refused).
const MONTHS: [u8; 5] = [3, 4, 5, 6, 7];

/// The environment of the launch child over `root`.
fn launch_env(root: &Path) -> Vec<(&'static str, std::ffi::OsString)> {
    let policy = root.join("policy.toml");
    std::fs::write(
        &policy,
        include_str!("../../../config/intraday-research-v1.toml"),
    )
    .expect("the policy file");
    let mut env: Vec<(&'static str, std::ffi::OsString)> = [
        ("BRUTEX_CHECKSUM_MAX_BYTES", "268435456"),
        ("BRUTEX_CHECKSUM_MAX_RECORDS", "3000000"),
        ("BRUTEX_BOOLEAN_OBSERVATION_BYTES", "2147483648"),
        ("BRUTEX_BOOLEAN_SEARCH_REPLAY_NODES", "3000000"),
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

/// Folds each month's `minutes` into the 60min bars of the store's 09:15 IST
/// grid and writes them beside the minutes, NIFTY under `zerodha`.
fn hour_months(root: &Path, minutes: &[Vec<store::format::Bar>]) {
    const HOUR: i64 = 3_600_000_000;
    // `(stamp + anchor) mod HOUR == 0` exactly at 09:15 IST.
    let anchor = pull::session::IST_OFFSET_SECS * 1_000_000 - 555 * 60_000_000;
    let hash = brutex_core::universe::fnv1a("NIFTY").to_le_bytes();
    let symbol = u32::from_le_bytes(hash[..4].try_into().expect("low32"));
    for (&month, minutes) in MONTHS.iter().zip(minutes) {
        let mut hours: Vec<store::format::Bar> = Vec::new();
        for minute in minutes {
            let start = minute.ts_micros - (minute.ts_micros + anchor).rem_euclid(HOUR);
            match hours.last_mut() {
                Some(hour) if hour.ts_micros == start => {
                    hour.high = hour.high.max(minute.high);
                    hour.low = hour.low.min(minute.low);
                    hour.close = minute.close;
                    hour.volume += minute.volume;
                }
                _ => hours.push(store::format::Bar {
                    ts_micros: start,
                    ..*minute
                }),
            }
        }
        let path = store::path::StorePath::new(store::path::PathParts {
            vendor: brutex_core::vendor::Vendor::Zerodha,
            exchange: "NSE",
            segment: "INDEX",
            symbol: "NIFTY",
            contract: None,
            timeframe: store::path::Timeframe::MINUTE_60,
            month: store::path::YearMonth::new(2025, month).expect("a month"),
            file: store::path::FileKind::Bars,
        })
        .expect("a generated path");
        store::file::BarFile::open_or_create(root, path, symbol)
            .expect("an owned generated writer")
            .append(&hours)
            .expect("generated hours append");
    }
}

/// The launch request: NIFTY at 60min, April and May 2025 for training and
/// June and July for the later days, over the bullish and bearish bar bits.
fn launch_body(digest: &str) -> String {
    serde_json::json!({"command":crate::booleanlaunch::COMMAND,"feed":"zerodha","symbols":["NIFTY"],
        "timeframes":["60min"],"from_year":2025,"from_month":4,"to_year":2025,"to_month":5,
        "later_from_year":2025,"later_from_month":6,"later_to_year":2025,"later_to_month":7,
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
    assert_eq!(found.len(), 1, "{namespace}: {found:?}");
    found.remove(0)
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

/// One page of the qualified search reader, as JSON with its status.
async fn search(query: &str) -> (StatusCode, Value) {
    let uri: Uri = format!("/boolean/search.json?{query}")
        .parse()
        .expect("a URI");
    let (status, _, body) = crate::booleansearchjson::search_json(uri).await;
    (status, serde_json::from_str(&body).expect("JSON"))
}

/// The parent half: a fresh scratch store holding [`MONTHS`] of NIFTY at one
/// minute, one day and 60min, then the child run alone over it.
fn in_a_launch_child() {
    let root = crate::scratch::path("saved-boolean-evidence");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("a scratch root");
    let minutes = crate::indexstopqualificationjson::saved_tests::index_months(
        &root,
        "NIFTY",
        &MONTHS,
        crate::indexstopqualificationjson::saved_tests::generated_bar,
    );
    hour_months(&root, &minutes);
    let log = root.join("child.log");
    let output = std::fs::File::create_new(&log).expect("a log");
    let child_test = CHILD_PATH
        .split_once("::")
        .map_or(CHILD_PATH, |(_, path)| path);
    let status = std::process::Command::new(std::env::current_exe().expect("this binary"))
        .args(["--exact", child_test, "--nocapture", "--test-threads=1"])
        .env_clear()
        .envs(std::env::var_os("LLVM_PROFILE_FILE").map(|profile| ("LLVM_PROFILE_FILE", profile)))
        .envs(launch_env(&root))
        .stdout(output.try_clone().expect("a second handle"))
        .stderr(output)
        .status()
        .expect("the child runs");
    let printed = std::fs::read_to_string(&log).expect("the child's log");
    assert!(status.success(), "{printed}");
    assert!(printed.contains("test result: ok. 1 passed;"), "{printed}");
    let _ = std::fs::remove_dir_all(&root);
}

/// **SAVED BOOLEAN EVIDENCE IS SERVED BY EVERY READER THAT READS IT**
/// (BASE-COV-API-AS3, D-8664). One production launch of the qualified
/// Boolean search over the generated store saves statistics, admission, a
/// later qualification and the search's acknowledged batch; the statistics,
/// admission and qualification readers serve each page kind of them, and the
/// search reader serves its overview and the rung it qualified, whose detail
/// projects the qualification through `qualification_observation` and each
/// row through `search_comparison`. Run in a child, since every bound comes
/// from the environment.
///
/// Extremes: singles — a 60min rung (seven bars a session, so every saved
/// body is small enough to rehash on each read), a later window of two
/// months (the shortest whose 60min bars outlast the 200-bar warm-up), a
/// second page one row in, the same first page asked again (the held reader
/// answers it), the largest page (limit 256), a page starting exactly at the
/// end (served empty, no next page), a page starting one past the end
/// (refused by name), a pin that is not the saved completion on statistics
/// and on admission, each statistics page kind (candidates, splits, sources),
/// each index period of one setting (training, later, full) and its weeks;
/// combinations — a pinned second page with its offset and its limit at once,
/// an index page with its setting, its period and its pin at once; out of
/// the box — the saved statistics asked under another root (an empty folder):
/// refused naming that root, no other folder searched.
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one production launch, then every page that reads what it saved, in the order a reader opens them"
)]
fn saved_boolean_evidence_is_served_by_every_reader_that_reads_it() {
    let Some(root) = std::env::var_os(CHILD) else {
        return in_a_launch_child();
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
        let (code, _, reply) =
            crate::sweeprun::command_with(&site, &launch_body(digest), cli::commit_stamp());
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
        let fields = run.boolean_search.expect("the search status").fields();

        // STATISTICS: every page kind, a second page, a pin it is not.
        let statistics = saved_in(&root, "boolean-statistics-v1");
        let (status, first) = page("statistics", &format!("identity={statistics}")).await;
        assert_eq!(status, StatusCode::OK, "{first}");
        assert_eq!(
            (&first["status"], &first["model"], &first["kind"]),
            (&json!("saved"), &json!("statistics"), &json!("candidates")),
            "{first}"
        );
        let pin = first["completion"].as_str().expect("a pin").to_owned();
        let total: usize = first["total"]
            .as_str()
            .expect("a total")
            .parse()
            .expect("a count");
        assert!(total > 1, "{first}");
        assert_eq!(
            first["rows"].as_array().expect("rows").len(),
            total.min(16),
            "{first}"
        );
        assert_eq!(first["summary"]["candidates"], total.to_string(), "{first}");
        let pinned = format!("identity={statistics}&completion={pin}");
        let (status, second) = page("statistics", &format!("{pinned}&offset=1&limit=1")).await;
        assert_eq!(status, StatusCode::OK, "{second}");
        assert_eq!(second["rows"][0], first["rows"][1], "{second}");
        let (status, again) = page("statistics", &format!("identity={statistics}")).await;
        assert_eq!(status, StatusCode::OK, "{again}");
        assert_eq!(
            again["rows"], first["rows"],
            "the held reader answers again"
        );
        let (status, largest) = page("statistics", &format!("{pinned}&limit=256")).await;
        assert_eq!(status, StatusCode::OK, "{largest}");
        assert_eq!(
            largest["rows"].as_array().expect("rows").len(),
            total.min(256),
            "{largest}"
        );
        let (status, end) = page("statistics", &format!("{pinned}&offset={total}")).await;
        assert_eq!(status, StatusCode::OK, "{end}");
        assert_eq!(
            (&end["rows"], &end["next"]),
            (&json!([]), &Value::Null),
            "{end}"
        );
        let (status, past) = page("statistics", &format!("{pinned}&offset={}", total + 1)).await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{past}");
        assert_eq!(past["refusal"], "Boolean evidence page refused", "{past}");
        let elsewhere = render_with_budget(
            &root.join("elsewhere"),
            &Asked::parse(&format!("identity={statistics}"), Model::Statistics).expect("asked"),
            crate::detail::BooleanObservationBudget::load().expect("a budget"),
        )
        .expect_err("nothing is saved under another root");
        assert!(
            elsewhere.contains("unavailable under configured root")
                && elsewhere.contains(&root.join("elsewhere").display().to_string()),
            "{elsewhere}"
        );
        for kind in ["splits", "sources"] {
            let (status, body) = page("statistics", &format!("{pinned}&kind={kind}")).await;
            assert_eq!(status, StatusCode::OK, "{body}");
            assert_eq!(body["kind"], kind, "{body}");
            assert_ne!(body["total"], "0", "{body}");
        }
        let other = "ab".repeat(32);
        let (status, refused) = page(
            "statistics",
            &format!("identity={statistics}&completion={other}"),
        )
        .await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{refused}");
        assert_eq!(
            refused["refusal"], "statistics completion pin differs",
            "{refused}"
        );

        // ADMISSION, beside the statistics it links.
        let admission = saved_in(&root, "boolean-admission-v1");
        let (status, admitted) = page("admission", &format!("identity={admission}")).await;
        assert_eq!(status, StatusCode::OK, "{admitted}");
        assert_eq!(admitted["model"], "admission", "{admitted}");
        assert_eq!(admitted["statistics_identity"], statistics, "{admitted}");
        assert_eq!(admitted["total"], first["total"], "{admitted}");
        let (status, refused) = page(
            "admission",
            &format!("identity={admission}&completion={other}"),
        )
        .await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{refused}");
        assert_eq!(
            refused["refusal"], "admission completion pin differs",
            "{refused}"
        );

        // QUALIFICATION: its settings, and each index page of one setting.
        let qualification = saved_in(&root, "boolean-qualification-v1");
        let (status, settings) = page("qualification", &format!("identity={qualification}")).await;
        assert_eq!(status, StatusCode::OK, "{settings}");
        assert_eq!(settings["model"], "qualification", "{settings}");
        let completion = settings["completion"].as_str().expect("a completion");
        let pinned = format!("identity={qualification}&completion={completion}");
        for (kind, period) in [
            ("index-days", "training"),
            ("index-days", "later"),
            ("index-days", "full"),
            ("index-weeks", "full"),
        ] {
            let (status, body) = page(
                "qualification",
                &format!("{pinned}&kind={kind}&setting=0&period={period}"),
            )
            .await;
            assert_eq!(status, StatusCode::OK, "{kind} {period}: {body}");
            assert_eq!(body["kind"], kind, "{body}");
        }

        // THE SEARCH: its overview, then the rung it qualified.
        let identity = fields["search_identity"].as_str().expect("a search");
        let (status, overview) = search(&format!("identity={identity}")).await;
        assert_eq!(status, StatusCode::OK, "{overview}");
        let rung = overview["rows"]
            .as_array()
            .expect("rows")
            .iter()
            .find(|rung| !rung["qualification"].is_null())
            .unwrap_or_else(|| panic!("no qualified rung: {overview}"));
        assert_eq!(rung["rung"], "60min", "{overview}");
        let (status, detail) = search(&format!(
            "identity={identity}&pin={}&batch={}&rung={}",
            overview["pin"].as_str().expect("a pin"),
            overview["batch"].as_str().expect("a batch"),
            rung["rung_index"].as_str().expect("a rung")
        ))
        .await;
        assert_eq!(status, StatusCode::OK, "{detail}");
        assert_eq!(detail["kind"], "rung", "{detail}");
        assert_eq!(detail["source"]["identity"], qualification, "{detail}");
        assert!(
            detail["rows"]
                .as_array()
                .expect("rows")
                .iter()
                .all(|row| !row["comparison"]["index_consistency"].is_null()),
            "{detail}"
        );
    });
}

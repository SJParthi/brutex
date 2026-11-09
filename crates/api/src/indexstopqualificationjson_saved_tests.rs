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

/// Generated NIFTY minutes and days of the months of 2025 named in `months`
/// under `zerodha`, every open session of the shared calendar, written by the
/// store's writer: minutes that cycle through a fixed pattern around
/// 20,000.00.
fn generated_nifty_months(root: &Path, months: &[u8]) {
    nifty_months(root, months, generated_bar);
}

/// The `(open, high, low, close)` of [`generated_nifty_months`]' minute of
/// the day `minute` on `day`: a fixed pattern around 20,000.00.
pub(crate) fn generated_bar(_month: u8, day: u8, minute: u16, _place: i64) -> (i64, i64, i64, i64) {
    let open = 2_000_000 + i64::from((minute + u16::from(day)) % 31) * 100;
    // Each minute reaches up and down by its own amount, from 100 to
    // 3,700 paisa, so the excursions an exit grid ranks spread wide enough
    // on both sides for a 3:1 target pair.
    let up = 100 + i64::from((minute * 7 + u16::from(day)) % 13) * 300;
    let down = 100 + i64::from((minute * 11 + u16::from(day)) % 13) * 300;
    let close = open + if minute.is_multiple_of(3) { -50 } else { 50 };
    (open, open + up, open - down, close)
}

/// The sessions of the trending months whose 09:16 minute dips 10 paisa
/// below the 09:15 low, four in the training window and three in the later.
const LOSS_DAYS: [(u8, u8); 7] = [(4, 15), (4, 22), (5, 6), (5, 20), (6, 10), (6, 17), (6, 24)];

/// Generated NIFTY months that rise through every session from 20,000.00, by
/// a slope of its own each day (no two days pay alike), every seventh minute
/// a bearish candle and every other a bullish one; on [`LOSS_DAYS`] the 09:16
/// minute dips 10 paisa below the 09:15 low, the only losses a long setting
/// meets.
fn trending_nifty_months(root: &Path, months: &[u8]) {
    nifty_months(root, months, |month, day, _minute, place| {
        let slope = 80 + i64::from(u16::from(day) * 7 % 41) + i64::from(month) * 3;
        let price = 2_000_000 + slope * place;
        let (open, close) = if place % 7 == 3 {
            (price + slope + 50, price)
        } else {
            (price, price + slope)
        };
        let low = if place == 1 && LOSS_DAYS.contains(&(month, day)) {
            1_999_970
        } else {
            open.min(close) - 20
        };
        (open, open.max(close) + 20, low, close)
    });
}

/// Writes the NIFTY minutes `bar` prices and their days: [`index_months`] for
/// NIFTY.
fn nifty_months(
    root: &Path,
    months: &[u8],
    bar: impl Fn(u8, u8, u16, i64) -> (i64, i64, i64, i64),
) {
    index_months(root, "NIFTY", months, bar);
}

/// Writes the minutes `bar` prices — `(open, high, low, close)` of the month,
/// day, minute of the day and place in its session — and their days for the
/// index `index` under `zerodha`, for every open session of the months of 2025
/// named in `months`; answers each month's minutes, in the order of `months`.
pub(crate) fn index_months(
    root: &Path,
    index: &str,
    months: &[u8],
    bar: impl Fn(u8, u8, u16, i64) -> (i64, i64, i64, i64),
) -> Vec<Vec<store::format::Bar>> {
    let hash = brutex_core::universe::fnv1a(index).to_le_bytes();
    let symbol = u32::from_le_bytes(hash[..4].try_into().expect("low32"));
    let mut written = Vec::new();
    for &month in months {
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
                    let place = i64::try_from(minutes.len() - start).expect("a session's minute");
                    let (open, high, low, close) = bar(month, day, minute, place);
                    minutes.push(store::format::Bar {
                        ts_micros: stamp * 86_400_000_000 + i64::from(minute) * 60_000_000
                            - pull::session::IST_OFFSET_SECS * 1_000_000,
                        open,
                        high,
                        low,
                        close,
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
                symbol: index,
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
        written.push(minutes);
    }
    written
}

/// The shipped admission policy profile.
const SHIPPED_POLICY: &str = include_str!("../../../config/intraday-research-v1.toml");

/// The admission policy at its floors: every structural count at 1 (the least
/// `runner::admission` admits: a 0 there proves no sample and is refused),
/// every other floor at 0 or its domain's bottom, every ceiling at its
/// domain's top, and neither rejection required. The operator's own rules
/// that no file sets (the smallest win at least three times the largest
/// loss, the largest loss within the risk) still bind.
const FLOOR_POLICY: &str = "policy_version = 1
min_support_hits = 1
min_independent_sessions = 1
min_trades = 1
max_mae_paisa = 18446744073709551615
min_win_rate_ppm = 0
min_wilson_win_rate_ppm = 0
min_return_drawdown_ppm = 0
min_weakest_period_return_paisa = -9223372036854775808
max_pbo_ppm = 1000000
max_fwer_p_value_ppm = 1000000
max_spa_p_value_ppm = 1000000
min_decided_folds = 1
max_ambiguous_fill_rate_ppm = 1000000
max_gap_affected_rate_ppm = 1000000
max_session_concentration_ppm = 1000000
max_largest_trade_profit_share_ppm = 1000000
max_drawdown_paisa = 18446744073709551615
max_losing_trade_rate_ppm = 1000000
max_losing_trades = 18446744073709551615
min_pessimistic_profit_paisa = -9223372036854775808
min_winning_trades = 0
min_average_win_paisa = 0
max_average_loss_paisa = 18446744073709551615
min_profit_factor_ppm = 0
max_consecutive_losing_streak = 18446744073709551615
min_consecutive_winning_streak = 0
min_bootstrap_draws = 1
min_bootstrap_strategies = 1
min_bootstrap_periods = 1
min_pbo_contributing_folds = 1
max_pbo_unrankable_folds = 18446744073709551615
min_profitable_oos_folds = 1
min_oos_pessimistic_return_paisa = -9223372036854775808
max_white_reality_p_value_ppm = 1000000
require_white_reality_rejection = false
max_romano_wolf_p_value_ppm = 1000000
require_romano_wolf_rejection = false
";

/// The environment of the launch child over `root`, `child` naming it, with
/// `policy` the admission policy file's text.
fn launch_env(
    root: &Path,
    child: &'static str,
    policy: &str,
) -> Vec<(&'static str, std::ffi::OsString)> {
    let policy_file = root.join("policy.toml");
    std::fs::write(&policy_file, policy).expect("the policy file");
    let policy = policy_file;
    let mut env: Vec<(&'static str, std::ffi::OsString)> = [
        ("BRUTEX_CHECKSUM_MAX_BYTES", "67108864"),
        ("BRUTEX_CHECKSUM_MAX_RECORDS", "3000000"),
        ("BRUTEX_BOOLEAN_OBSERVATION_BYTES", "268435456"),
        ("BRUTEX_BOOLEAN_SEARCH_REPLAY_NODES", "6000000"),
        ("BRUTEX_INDEX_STOP_MAX_LOSS_POINTS", "5"),
        ("BRUTEX_INDEX_STOP_BATCH_PROGRAMS", "2"),
        ("BRUTEX_INDEX_STOP_NODE_ALLOWANCE", "128"),
        ("BRUTEX_INDEX_STOP_BATCH_ALLOWANCE", "1"),
        ("BRUTEX_INDEX_STOP_BOOTSTRAP_DRAWS", "319"),
        ("BRUTEX_INDEX_STOP_WORKERS", "1"),
    ]
    .into_iter()
    .map(|(name, value)| (name, value.into()))
    .collect();
    for name in [child, "BRUTEX_STORE", "BRUTEX_CHECKSUM_RECEIPTS"] {
        env.push((name, root.into()));
    }
    env.push(("BRUTEX_ADMISSION_POLICY_FILE", policy.into()));
    env
}

/// The parent half of a launch test: a fresh scratch root named `name` that
/// `store` fills, then `child_path` run alone in a child over it (`child` set
/// to the root, `policy` its admission policy), which must pass.
fn in_a_launch_child(
    child: &'static str,
    child_path: &str,
    name: &str,
    store: impl FnOnce(&Path),
    policy: &str,
) {
    let root = crate::scratch::path(name);
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("a scratch root");
    store(&root);
    let log = root.join("child.log");
    let output = std::fs::File::create_new(&log).expect("a log");
    let child_test = child_path
        .split_once("::")
        .map_or(child_path, |(_, path)| path);
    let status = std::process::Command::new(std::env::current_exe().expect("this binary"))
        .args(["--exact", child_test, "--nocapture", "--test-threads=1"])
        .env_clear()
        .envs(std::env::var_os("LLVM_PROFILE_FILE").map(|profile| ("LLVM_PROFILE_FILE", profile)))
        .envs(launch_env(&root, child, policy))
        .stdout(output.try_clone().expect("a second handle"))
        .stderr(output)
        .status()
        .expect("the child runs");
    let printed = std::fs::read_to_string(&log).expect("the child's log");
    assert!(status.success(), "{printed}");
    assert!(printed.contains("test result: ok. 1 passed;"), "{printed}");
    let _ = std::fs::remove_dir_all(&root);
}

/// The launch request: NIFTY at one minute, May 2025 for training and June
/// for the later days, over the bullish and bearish bar bits (30, 31), which
/// the generated minutes, closing 50 paisa above or below their open, keep
/// firing.
fn launch_body(digest: &str) -> String {
    launch_body_over(digest, (5, 5), (6, 6))
}

/// [`launch_body`] with the training and later windows given as the first
/// and last month of 2025 of each.
fn launch_body_over(digest: &str, training: (u8, u8), later: (u8, u8)) -> String {
    serde_json::json!({"command":crate::indexstoplaunch::COMMAND,"feed":"zerodha","index":"NSE-NIFTY",
        "timeframes":["1min"],"from_year":2025,"from_month":training.0,"to_year":2025,"to_month":training.1,
        "later_from_year":2025,"later_from_month":later.0,"later_to_year":2025,"later_to_month":later.1,
        "bits":"30,31","max_loss_points":"5","batch_programs":"2","node_allowance":"128",
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
///
/// Extremes: singles — a build with no commit stamp (a 503 before any
/// launch), a page one row in (offset 1, limit 1), a setting one past the
/// saved extent on the fold page and on the day page, an offset one past the
/// extent, a pin that is not the saved completion, an identity nothing saved,
/// a ranking given no budget and one its admitted bytes already fill, a trade
/// past the saved record, a source query that names a trade, a candle query
/// with no context; combinations — a saved body changed under the kept reader
/// (refused as stale) and restored (served again by a fresh open) at once,
/// the later source asked under the first source's pin at once, a source body
/// changed under the kept candle reader and restored at once; out of the box
/// — the same identity asked under another root (the kept reader dropped and
/// that root's open refused by name), the same source and pin asked under
/// another root through the candle reader (refused by its code).
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one production launch, then every page that reads what it saved, in the order a reader opens them"
)]
fn a_saved_qualification_is_served_by_every_page_that_reads_it() {
    let Some(root) = std::env::var_os(CHILD) else {
        return in_a_launch_child(
            CHILD,
            CHILD_PATH,
            "saved-qualification",
            |root| generated_nifty_months(root, &[4, 5, 6]),
            SHIPPED_POLICY,
        );
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
        assert_eq!(settings["kind"], "settings", "{settings}");
        assert_eq!(settings["completion"], completion, "{settings}");
        let total: usize = settings["total"]
            .as_str()
            .expect("a total")
            .parse()
            .expect("a count");
        assert_eq!(
            settings["rows"].as_array().expect("rows").len(),
            total.min(16),
            "{settings}"
        );
        let pinned = format!("identity={identity}&completion={completion}");
        let (status, again) = page(&pinned).await;
        assert_eq!(status, StatusCode::OK, "{again}");
        assert_eq!(
            again["rows"], settings["rows"],
            "the kept reader serves the same page"
        );
        let (status, second) = page(&format!("{pinned}&offset=1&limit=1")).await;
        assert_eq!(status, StatusCode::OK, "{second}");
        assert_eq!(second["rows"][0], settings["rows"][1], "{second}");
        let (status, folds) = page(&format!("{pinned}&kind=folds&setting=0")).await;
        assert_eq!(status, StatusCode::OK, "{folds}");
        assert_eq!(folds["kind"], "folds", "{folds}");
        for period in ["training", "later", "full"] {
            let (status, days) = page(&format!(
                "{pinned}&kind=index-days&setting=0&period={period}"
            ))
            .await;
            assert_eq!(status, StatusCode::OK, "{days}");
            assert_eq!(days["period"], period, "{days}");
        }
        let (status, weeks) = page(&format!("{pinned}&kind=index-weeks&setting=0")).await;
        assert_eq!(status, StatusCode::OK, "{weeks}");
        assert_eq!(weeks["kind"], "index-weeks", "{weeks}");
        let other = "ab".repeat(32);
        for (query, why) in [
            (
                format!("identity={identity}&completion={other}"),
                "native qualification pin differs",
            ),
            (
                format!("{pinned}&kind=folds&setting={total}"),
                "native fold setting outside saved extent",
            ),
            (
                format!("{pinned}&kind=index-days&setting={total}"),
                "native day/week setting outside saved extent",
            ),
            (
                format!("{pinned}&offset={}", total + 1),
                "native qualification page outside saved extent",
            ),
            (format!("identity={other}"), "Native qualification"),
        ] {
            let (status, refused) = page(&query).await;
            assert_eq!(
                status,
                StatusCode::SERVICE_UNAVAILABLE,
                "{query}: {refused}"
            );
            let said = refused["refusal"].as_str().expect("a refusal");
            assert!(said.contains(why), "{query}: {refused}");
        }

        // A SAVED BODY CHANGED UNDER THE KEPT READER is stale: refused, never
        // served from the copy it kept; the original bytes restored are
        // admitted again by a fresh open. The pinned read first keeps the
        // reader, so the changed body is met by the kept reader's own check.
        let (status, kept) = page(&pinned).await;
        assert_eq!(status, StatusCode::OK, "{kept}");
        let body = root
            .join(cli::index_stop_qualification::NAMESPACE)
            .join(identity)
            .join("body.bin");
        let bytes = std::fs::read(&body).expect("the saved body");
        let swap = |with: &[u8]| {
            let copy = body.with_extension("tmp");
            std::fs::write(&copy, with).expect("the replacement");
            std::fs::rename(&copy, &body).expect("the replacement lands");
        };
        swap(&[bytes.as_slice(), &[0]].concat());
        let (status, stale) = page(&pinned).await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{stale}");
        swap(&bytes);
        let (status, readmitted) = page(&pinned).await;
        assert_eq!(status, StatusCode::OK, "{readmitted}");
        assert_eq!(readmitted["rows"], settings["rows"], "{readmitted}");

        // A KEPT READER ANSWERS ONLY ITS OWN ROOT (BASE-COV-API-AS3): the same
        // identity asked under another root drops it, and that root's open
        // refuses by name rather than serving the kept rows.
        let asked = super::Asked::parse(&format!("identity={identity}")).expect("an ask");
        let elsewhere = super::render(&root.join("another-root"), &asked)
            .expect_err("no qualification is saved under another root");
        assert!(
            elsewhere.contains("Native qualification") && elsewhere.contains("another-root"),
            "{elsewhere}"
        );
        // THE RANKING HAS A BUDGET OF ITS OWN (BASE-COV-API-AS3): over the
        // saved qualification, a budget its admitted bytes already fill, and
        // none at all, are refused by name before any row is ranked.
        let budget = crate::detail::BooleanObservationBudget::load().expect("the child's budget");
        let reader = super::Reader::open_with_replay_bounds(
            &root,
            asked.identity,
            budget.bytes(),
            budget.bytes() / 64,
            super::replay_bounds(budget.bytes()).expect("the replay bounds"),
        )
        .expect("the saved qualification opens");
        for tight in [0, reader.admitted_bytes()] {
            assert_eq!(
                super::rank(&reader, tight).map(|(ranked, _)| ranked.len()),
                Err("native ranking plus retained evidence exceeds read admission".to_owned()),
                "a budget of {tight}"
            );
        }

        let original = &settings["original"];
        let source = original["identity"].as_str().expect("a source");
        let source_pin = original["completion"].as_str().expect("its completion");
        let (status, catalog) = stop(&format!("identity={source}")).await;
        assert_eq!(status, StatusCode::OK, "{catalog}");
        for kind in ["trades", "events", "days"] {
            let (status, rows) = stop(&format!(
                "identity={source}&completion={source_pin}&kind={kind}&setting=0"
            ))
            .await;
            assert_eq!(status, StatusCode::OK, "{kind}: {rows}");
            assert_eq!(rows["kind"], kind, "{rows}");
        }
        let (status, meta) = candles(&format!(
            "identity={source}&pin={source_pin}&setting=0&kind=source"
        ))
        .await;
        assert_eq!(status, StatusCode::OK, "{meta}");
        assert_eq!(
            meta["provenance_status"], "verified_original_source",
            "{meta}"
        );
        let (status, context) = candles(&format!(
            "identity={source}&pin={source_pin}&setting=0&kind=candles&trade=0&before=2&after=2"
        ))
        .await;
        assert_eq!(status, StatusCode::OK, "{context} (catalog: {catalog})");
        assert_eq!(context["direction"], "long", "{context}");

        // The second setting is the same program read short.
        let (status, short) = candles(&format!(
            "identity={source}&pin={source_pin}&setting=1&kind=source"
        ))
        .await;
        assert_eq!(status, StatusCode::OK, "{short}");
        assert_eq!(short["direction"], "short", "{short}");
        // A trade past the saved record refuses, and the kept reader stays.
        let (status, past) = candles(&format!(
            "identity={source}&pin={source_pin}&setting=1&kind=candles&trade=999999&before=1&after=1"
        ))
        .await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{past}");
        // A source query that names a trade, and a candle query with no
        // context, are refused before any read.
        for wrong in [
            format!("identity={source}&pin={source_pin}&setting=0&kind=source&trade=0"),
            format!("identity={source}&pin={source_pin}&setting=0&kind=candles&trade=0"),
        ] {
            let (status, refused) = candles(&wrong).await;
            assert_eq!(status, StatusCode::BAD_REQUEST, "{wrong}: {refused}");
        }
        // Another source, and the first source under a wrong pin, are not
        // the kept reader: the first is opened, the second refused.
        let later = &settings["later"];
        let (later_id, later_pin) = (
            later["identity"].as_str().expect("a later source"),
            later["completion"].as_str().expect("its completion"),
        );
        let (status, other) = candles(&format!(
            "identity={later_id}&pin={later_pin}&setting=0&kind=source"
        ))
        .await;
        assert_eq!(status, StatusCode::OK, "{other}");
        let (status, wrong_pin) = candles(&format!(
            "identity={later_id}&pin={source_pin}&setting=0&kind=source"
        ))
        .await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{wrong_pin}");
        // The single-stop reader keeps one source: a pinned page of the
        // other is opened afresh.
        let (status, later_trades) = stop(&format!(
            "identity={later_id}&completion={later_pin}&kind=trades&setting=0"
        ))
        .await;
        assert_eq!(status, StatusCode::OK, "{later_trades}");
        // A source body changed under the kept candle reader is stale, and
        // the original bytes restored are admitted again.
        let (status, kept) = candles(&format!(
            "identity={source}&pin={source_pin}&setting=0&kind=source"
        ))
        .await;
        assert_eq!(status, StatusCode::OK, "{kept}");
        let source_body = root
            .join(cli::index_stop_store::NAMESPACE)
            .join(source)
            .join("body.bin");
        let source_bytes = std::fs::read(&source_body).expect("the source body");
        let replace = |with: &[u8]| {
            let copy = source_body.with_extension("tmp");
            std::fs::write(&copy, with).expect("the replacement");
            std::fs::rename(&copy, &source_body).expect("the replacement lands");
        };
        replace(&[source_bytes.as_slice(), &[0]].concat());
        let (status, stale) = candles(&format!(
            "identity={source}&pin={source_pin}&setting=0&kind=source"
        ))
        .await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{stale}");
        replace(&source_bytes);
        let (status, again) = candles(&format!(
            "identity={source}&pin={source_pin}&setting=0&kind=source"
        ))
        .await;
        assert_eq!(status, StatusCode::OK, "{again}");
        assert_eq!(again["source_id"], kept["source_id"], "{again}");
        // A KEPT CANDLE READER ANSWERS ONLY ITS OWN ROOT (BASE-COV-API-AS3):
        // the same source and pin asked under another root drop it, and that
        // root's open refuses rather than serving the kept source.
        let elsewhere = crate::indexstopcandlesjson::render_at(
            &root.join("another-root"),
            &format!("identity={source}&pin={source_pin}&setting=0&kind=source"),
        )
        .expect_err("no source is saved under another root");
        assert!(
            elsewhere.starts_with("original_context_refused: ") && elsewhere.contains("(os error 2)"),
            "{elsewhere}"
        );
    });
}

/// The child half of a launch test over the store at `root`: the preview, the
/// launch of the `training` and `later` windows (the first and last month of
/// 2025 of each), its end, and the settings page of the qualification it
/// saved. `None` in a build with no commit stamp, whose launch is refused
/// with a 503 before anything runs.
fn launched_settings(root: &Path, training: (u8, u8), later: (u8, u8)) -> Option<Value> {
    let site = Arc::new(crate::server::Site::load(
        &root.join("absent-masters"),
        root,
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
        let body = launch_body_over(
            preview["policy"]["digest"].as_str().expect("a digest"),
            training,
            later,
        );
        let (code, _, reply) = crate::sweeprun::command_with(&site, &body, cli::commit_stamp());
        let Some(_stamp) = cli::commit_stamp() else {
            assert_eq!(code, StatusCode::SERVICE_UNAVAILABLE, "{reply}");
            return None;
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
        let identity = fields["qualifications"][0]["identity"]
            .as_str()
            .expect("an identity")
            .to_owned();
        let (status, settings) = page(&format!("identity={identity}")).await;
        assert_eq!(status, StatusCode::OK, "{settings}");
        Some(settings)
    })
}

/// Every institutional check of a settings row that did not pass, as
/// `name:state`, in the row's order.
fn not_passed(row: &Value) -> Vec<String> {
    row["institutional"]["checks"]
        .as_array()
        .expect("checks")
        .iter()
        .filter(|check| check["state"] != "passed")
        .map(|check| {
            format!(
                "{}:{}",
                check["name"].as_str().expect("a name"),
                check["state"].as_str().expect("a state")
            )
        })
        .collect()
}

/// Set in the child that runs the forty-session launch, over the store it
/// names.
const REJECTED_CHILD: &str = "BRUTEX_REJECTED_QUALIFICATION_TEST_CHILD";
const REJECTED_CHILD_PATH: &str = concat!(
    module_path!(),
    "::a_forty_session_training_window_is_measured_whole_and_ranked_rejected"
);

/// **A FORTY-SESSION TRAINING WINDOW IS MEASURED WHOLE AND RANKED REJECTED**
/// (BASE-COV-API-AS3, D-8664). The saved probe trains on May alone: 21 open
/// sessions, an odd count, so its split statistics leave values unmeasured
/// and every setting is ranked UNMEASURED. April and May hold forty sessions,
/// so every value of every check is measured, and the generated minutes'
/// signal-candle stops, which lose more than the five-point risk the policy
/// allows, fail at least one check: every setting is ranked REJECTED, none
/// unmeasured, refused or admitted (`rank`'s rejected arm). March is
/// generated too, the month before the window, which the launch reads before
/// its first session (without it the launch is refused for a file it cannot
/// find). Run in a child, since every bound comes from the environment.
///
/// Extremes: singles — a training window of two months (forty sessions,
/// even), a later window of one month, every institutional check measured
/// (no check is `unmeasured`), at least one check failed in every setting;
/// combinations — every setting measured and rejected at once (`rejected` =
/// `total`, every other count 0), and no setting qualifying by its days
/// beside it (`combined_qualified` 0) at once; out of the box — a store
/// holding a month neither window names (March), which the launch reads
/// before the training window's first session and is not refused for.
#[test]
fn a_forty_session_training_window_is_measured_whole_and_ranked_rejected() {
    let Some(root) = std::env::var_os(REJECTED_CHILD) else {
        return in_a_launch_child(
            REJECTED_CHILD,
            REJECTED_CHILD_PATH,
            "rejected-qualification",
            |root| generated_nifty_months(root, &[3, 4, 5, 6]),
            SHIPPED_POLICY,
        );
    };
    let Some(settings) = launched_settings(Path::new(&root), (4, 5), (6, 6)) else {
        return;
    };
    let total = settings["total"].as_str().expect("a total");
    assert_ne!(total, "0", "{settings}");
    assert_eq!(
        settings["summary"],
        serde_json::json!({"institutional_admitted":"0","rejected":total,"refused":"0",
            "unmeasured":"0","combined_qualified":"0"}),
        "{settings}"
    );
    for row in settings["rows"].as_array().expect("rows") {
        let checks = not_passed(row);
        assert!(
            !checks.iter().any(|check| check.ends_with(":unmeasured")),
            "{row}"
        );
        assert!(
            checks.iter().any(|check| check.ends_with(":failed")),
            "{row}"
        );
    }
}

/// Set in the child that runs the launch an admitted setting comes out of,
/// over the store it names.
const ADMITTED_CHILD: &str = "BRUTEX_ADMITTED_QUALIFICATION_TEST_CHILD";
const ADMITTED_CHILD_PATH: &str = concat!(
    module_path!(),
    "::a_setting_that_passes_every_check_is_ranked_admitted"
);

/// **A SETTING THAT PASSES EVERY CHECK IS RANKED ADMITTED** (BASE-COV-API-AS3,
/// D-8664). The forty-session launch over months that rise through every
/// session ([`trending_nifty_months`]), under the policy at its floors
/// ([`FLOOR_POLICY`]): the long setting of the bullish bar trades once a
/// session from 09:16 to 15:10 and loses only on the seven sessions whose
/// 09:16 minute dips 10 paisa past its stop, so it passes every check, the
/// operator's own rules no file sets among them (the smallest win at least
/// three times the largest loss, the largest loss within the five-point
/// risk), and is ranked ADMITTED (`rank`'s admitted arm). The long setting of
/// the bearish bar never loses, so the ratios a loss divides stay unmeasured;
/// the short settings never win. Run in a child, since every bound comes from
/// the environment.
///
/// Extremes: singles — a policy at its floors (every structural count 1,
/// every other floor 0 or its domain's bottom, every ceiling at its domain's
/// top, neither rejection required), sessions that only rise, each by a slope
/// of its own, a long setting whose only losses are seven stops 10 paisa
/// deep, a long setting with no loss at all (worst reward to risk, average
/// loss and profit factor unmeasured), short settings that never win
/// (average win unmeasured); combinations — one setting admitted while the
/// other three are unmeasured at once, and the admitted setting qualifying by
/// its days beside it (`combined_qualified` 1) at once; out of the box — a
/// policy file that relaxes every threshold it may while the operator's
/// fixed rules still bind and are met (`worst_reward_risk` and
/// `worst_trade_loss` passed).
#[test]
fn a_setting_that_passes_every_check_is_ranked_admitted() {
    let Some(root) = std::env::var_os(ADMITTED_CHILD) else {
        return in_a_launch_child(
            ADMITTED_CHILD,
            ADMITTED_CHILD_PATH,
            "admitted-qualification",
            |root| trending_nifty_months(root, &[3, 4, 5, 6]),
            FLOOR_POLICY,
        );
    };
    let Some(settings) = launched_settings(Path::new(&root), (4, 5), (6, 6)) else {
        return;
    };
    assert_eq!(settings["total"], "4", "{settings}");
    assert_eq!(
        settings["summary"],
        serde_json::json!({"institutional_admitted":"1","rejected":"0","refused":"0",
            "unmeasured":"3","combined_qualified":"1"}),
        "{settings}"
    );
    let rows = settings["rows"].as_array().expect("rows");
    let by_index = |index: &str| {
        let row = rows
            .iter()
            .find(|row| row["index"] == index)
            .unwrap_or_else(|| panic!("setting {index}: {settings}"));
        not_passed(row)
    };
    assert_eq!(by_index("0"), Vec::<String>::new(), "the admitted setting");
    assert_eq!(
        by_index("2"),
        [
            "worst_reward_risk:unmeasured",
            "average_loss:unmeasured",
            "profit_factor:unmeasured"
        ],
        "a long setting that never loses"
    );
    for short in ["1", "3"] {
        assert!(
            by_index(short).contains(&"average_win:unmeasured".to_owned()),
            "a short setting that never wins: {settings}"
        );
    }
}

/// One page of the single-stop source reader, as JSON with its status.
async fn stop(query: &str) -> (StatusCode, Value) {
    let uri: Uri = format!("/index-stop.json?{query}").parse().expect("a URI");
    let (status, _, body) = crate::indexstopjson::index_stop_json(uri).await;
    (status, serde_json::from_str(&body).expect("JSON"))
}

/// One page of the original-source candle reader, as JSON with its status.
async fn candles(query: &str) -> (StatusCode, Value) {
    let uri: Uri = format!("/index-stop/candles.json?{query}")
        .parse()
        .expect("a URI");
    let (status, _, body) = crate::indexstopcandlesjson::index_stop_candles_json(uri).await;
    (status, serde_json::from_str(&body).expect("JSON"))
}

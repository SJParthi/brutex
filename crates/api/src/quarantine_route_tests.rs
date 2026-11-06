#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used
)]
//! The quarantine's routes and its readers by name: `/bars.json` answers a
//! month quarantined whole by name, `/bars/window.json` names it and serves
//! the rest, `/instruments.json` flags the series, and the plan and the press
//! run the one verb. R4-18. Every price is invented.

use std::fs;
use std::path::{Path, PathBuf};

use axum::http::StatusCode;
use brutex_core::vendor::Vendor;
use serde_json::Value;
use store::format::{Bar, OI_NULL};
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};
use store::quarantine::{Cause, Record, Scope};

use super::super::{Loaded, Site};
use super::{arguments, quarantine_apply, quarantine_plan_json};

const SYMBOL: &str = "QFIXQ";

/// The press is one per PROCESS, and every test of this binary shares the
/// process: the tests that press take turns.
static PRESS_TESTS: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

struct Fixture {
    root: PathBuf,
    site: Loaded,
}

impl Fixture {
    fn new(name: &str) -> Self {
        let root = crate::scratch::path(name);
        let masters = root.join("masters");
        fs::create_dir_all(&masters).expect("offline empty master root");
        let site = Loaded::new(Site::load(&masters, &root));
        Self { root, site }
    }

    fn month(&self, month: YearMonth, level: i64, n: i64) -> Vec<Bar> {
        let rows = rows(month, level, n);
        let mut file = store::file::BarFile::open_or_create(
            &self.root,
            path(month),
            pull::ingest::repair_plan::symbol_id(SYMBOL),
        )
        .expect("a month");
        file.append(&rows).expect("rows");
        rows
    }

    fn quarantine_whole(&self, month: YearMonth, bars: u64) {
        store::quarantine::record(
            &self.root,
            &Record {
                path: path(month),
                scope: Scope::WholeMonth,
                cause: Cause::BeforeListing,
                stamps: &[],
                bars,
                revision: 0,
                source_digest: [7; 32],
                who: "test",
                reason: "invented: before listing",
            },
        )
        .expect("recorded");
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _cleanup = fs::remove_dir_all(&self.root);
    }
}

fn path(month: YearMonth) -> StorePath<'static> {
    StorePath::new(PathParts {
        vendor: Vendor::Zerodha,
        exchange: "NSE",
        segment: "CASH",
        symbol: SYMBOL,
        contract: None,
        timeframe: Timeframe::MINUTE_1,
        month,
        file: FileKind::Bars,
    })
    .expect("a path")
}

fn ym(year: u16, month: u8) -> YearMonth {
    YearMonth::new(year, month).expect("a month")
}

/// `n` minutes from 09:15 IST on the 5th of `month`.
fn rows(month: YearMonth, level: i64, n: i64) -> Vec<Bar> {
    let day = pull::session::Day::new(month.year(), month.month(), 5).expect("a day");
    let open = i64::from(day.days_from_epoch()) * 86_400_000_000 + 13_500_000_000;
    (0..n)
        .map(|i| Bar {
            ts_micros: open + i * 60_000_000,
            open: level,
            high: level + 4,
            low: level - 4,
            close: level + 1,
            volume: 10 + i,
            open_interest: OI_NULL,
        })
        .collect()
}

async fn get(site: &Loaded, uri: &str) -> (StatusCode, Value) {
    for _ in 0..400 {
        let parsed: axum::http::Uri = uri.parse().expect("URI");
        let (status, _, body) = if uri.starts_with("/bars/window.json") {
            super::super::bars_route::bars_window_json(
                axum::extract::State(Loaded::clone(site)),
                parsed,
            )
            .await
        } else if uri.starts_with("/bars.json") {
            super::super::bars_route::bars_json(axum::extract::State(Loaded::clone(site)), parsed)
                .await
        } else {
            quarantine_plan_json(axum::extract::State(Loaded::clone(site)), parsed).await
        };
        if status != StatusCode::TOO_MANY_REQUESTS {
            return (status, serde_json::from_str(&body).expect("JSON"));
        }
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
    panic!("the pool stayed full for two seconds");
}

fn files_under(dir: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut out = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries {
            let path = entry.expect("entry").path();
            if path.is_dir() {
                out.extend(files_under(&path));
            } else {
                out.push((path.clone(), fs::read(&path).expect("bytes")));
            }
        }
    }
    out.sort();
    out
}

#[tokio::test]
async fn bars_json_answers_a_quarantined_month_by_name() {
    let fixture = Fixture::new("r418-bars-by-name");
    fixture.month(ym(2018, 3), 77_713, 3);
    let uri =
        format!("/bars.json?feed=zerodha&exchange=NSE&segment=CASH&symbol={SYMBOL}&month=2018-03");
    let (status, before) = get(&fixture.site, &uri).await;
    assert_eq!(status, StatusCode::OK, "{before}");
    assert_eq!(before.as_array().map(Vec::len), Some(3));
    fixture.quarantine_whole(ym(2018, 3), 3);
    let (status, body) = get(&fixture.site, &uri).await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(body["code"], "month_quarantined");
    assert_eq!(body["quarantine"]["scope"], "whole_month");
    assert_eq!(body["quarantine"]["cause"], "before_listing");
    assert_eq!(body["quarantine"]["bars"], 3);
    assert_eq!(
        body["quarantine"]["ledger"],
        "audit/zerodha.quarantine.ledger"
    );
    assert!(
        body["quarantine"]["ledger_at"]
            .as_u64()
            .is_some_and(|at| at > 0)
    );
    assert!(body.get("bars").is_none(), "never its bars: {body}");
    // A torn index answers by name, never the bars.
    let index = store::quarantine::index_path(&fixture.root, Vendor::Zerodha);
    let bytes = fs::read(&index).expect("index");
    fs::write(&index, &bytes[..bytes.len() / 2]).expect("torn");
    let (status, torn) = get(&fixture.site, &uri).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "{torn}");
    assert_eq!(torn["code"], "quarantine_index_torn");
}

#[tokio::test]
async fn a_window_names_its_quarantined_month_and_serves_the_rest() {
    let fixture = Fixture::new("r418-window");
    let february = fixture.month(ym(2018, 2), 410, 2);
    fixture.month(ym(2018, 3), 77_713, 3);
    fixture.month(ym(2018, 4), 420, 2);
    fixture.quarantine_whole(ym(2018, 3), 3);
    let uri = format!(
        "/bars/window.json?feed=zerodha&exchange=NSE&segment=CASH&symbol={SYMBOL}&from=2018-02&to=2018-04&limit=50"
    );
    let (status, body) = get(&fixture.site, &uri).await;
    assert!(status.is_success(), "{status} {body}");
    let text = body.to_string();
    assert!(text.contains("2018-03: quarantined, not served"), "{text}");
    assert!(
        !text.contains("77713"),
        "no quarantined bar is served: {text}"
    );
    // FEBRUARY IS SERVED: its bars open at the invented level 410. (This was
    // `stamp || 410`, and repair r2's Route B found the stamp's side never
    // true: only the level ever answered, so only the level is asked.)
    assert!(!february.is_empty());
    assert!(text.contains("410"), "{text}");
}

#[test]
fn instruments_json_flags_a_quarantined_series() {
    let fixture = Fixture::new("r418-instruments");
    fixture.month(ym(2018, 3), 77_713, 3);
    fixture.quarantine_whole(ym(2018, 3), 3);
    let key = brutex_core::instrument::InstrumentKey::cash(
        brutex_core::instrument::Exchange::Nse,
        SYMBOL,
    )
    .expect("a share");
    let summary = store::quarantine::Summary::load(&fixture.root, Vendor::Zerodha);
    let flag = super::super::instruments_route::quarantine_of(&summary, &key);
    let value: Value = serde_json::from_str(&flag).expect("JSON");
    assert_eq!(value["months"], 1);
    assert_eq!(value["whole_months"], 1);
    assert_eq!(value["bars"], 3);
    assert_eq!(value["causes"], serde_json::json!(["before_listing"]));
    let other = brutex_core::instrument::InstrumentKey::cash(
        brutex_core::instrument::Exchange::Nse,
        "QFIXR",
    )
    .expect("a share");
    assert_eq!(
        super::super::instruments_route::quarantine_of(&summary, &other),
        "null"
    );
    let unreadable = Err(store::quarantine::QuarantineError::Reason("x".to_owned()));
    assert_eq!(
        super::super::instruments_route::quarantine_of(&unreadable, &key),
        r#"{"unknown":"quarantine_reason"}"#
    );
}

#[test]
fn the_question_is_refused_by_name_before_the_verb_runs() {
    assert!(
        arguments(
            "symbol=IRFC&from=2018-01-01&to=2020-12-31&cause=before_listing",
            false
        )
        .is_ok()
    );
    for bad in [
        "",
        "symbol=IRFC&from=2018-01-01&to=2020-12-31",
        "symbol=IRFC&from=2018-01-01&cause=before_listing",
        "symbol=IRFC&from=2018-01-01&to=2020-12-31&cause=before_listing&bogus=1",
        "symbol=IRFC&from=2018-01-01&to=2020-12-31&cause=before_listing&reason=a%09b",
    ] {
        assert!(arguments(bad, false).is_err(), "{bad:?}");
    }
    let long = format!(
        "symbol=IRFC&from=2018-01-01&to=2020-12-31&cause=before_listing&reason={}",
        "x".repeat(201)
    );
    assert!(arguments(&long, true).is_err(), "a reason past 200 bytes");
    assert!(
        arguments(
            "symbol=IRFC&from=2018-01-01&to=2020-12-31&cause=before_listing",
            true
        )
        .is_err(),
        "a press names its reason"
    );
    let args = arguments(
        "symbol=IRFC&from=2018-01-01&to=2020-12-31&cause=zero_price&reason=r&segment=CASH",
        true,
    )
    .expect("ok");
    assert_eq!(args.last().map(String::as_str), Some("--apply"));
}

fn plan_file(root: &Path) -> PathBuf {
    let dir = root.join("audit").join("series-identity");
    fs::create_dir_all(&dir).expect("dir");
    dir.join(format!("zerodha-CASH-{SYMBOL}-00aa00aa00aa00aa.plan"))
}

fn plan_body(stamps: &[i64]) -> String {
    let listed = stamps
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "brutex series-identity quarantine plan v2\nkey\trung\tmonth\tscope\tcause\taction\tbars\ttimestamps_utc_micros\nNSE-CASH-{SYMBOL}\t1min\t2018-03\ttimestamps\tforeign_level\tquarantine\t{}\t{listed}\n",
        stamps.len()
    )
}

fn plan(root: &Path, stamps: &[i64]) {
    fs::write(plan_file(root), plan_body(stamps)).expect("plan");
}

#[tokio::test]
async fn the_plan_writes_nothing_and_the_press_applies_once() {
    let _serial = PRESS_TESTS.lock().await;
    let fixture = Fixture::new("r418-press");
    let held = fixture.month(ym(2018, 3), 520, 5);
    plan(&fixture.root, &[held[1].ts_micros]);
    let before = files_under(&fixture.root);
    let (status, body) = get(
        &fixture.site,
        &format!("/pull/quarantine.json?symbol={SYMBOL}&from=2018-03-01&to=2018-03-31&cause=foreign_level"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["applied"], false);
    assert!(
        body["report"].as_str().unwrap().contains("plan: 1 bar(s)"),
        "{body}"
    );
    assert_eq!(files_under(&fixture.root), before, "the plan wrote nothing");
    let form = format!(
        "symbol={SYMBOL}&from=2018-03-01&to=2018-03-31&cause=foreign_level&reason=invented"
    );
    let (status, _, text) = quarantine_apply(
        axum::extract::State(Loaded::clone(&fixture.site)),
        form.clone(),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{text}");
    let body: Value = serde_json::from_str(&text).expect("JSON");
    assert_eq!(body["applied"], true);
    assert!(
        body["report"].as_str().unwrap().contains("DROPPED 1 of 5"),
        "{body}"
    );
    let (_, _, again) =
        quarantine_apply(axum::extract::State(Loaded::clone(&fixture.site)), form).await;
    assert!(again.contains("already_dropped"), "{again}");
    // A SECOND PRESS WHILE ONE RUNS is refused by name, never queued.
    super::PRESSING.store(true, std::sync::atomic::Ordering::Release);
    let (status, _, busy) = quarantine_apply(
        axum::extract::State(Loaded::clone(&fixture.site)),
        format!("symbol={SYMBOL}&from=2018-03-01&to=2018-03-31&cause=foreign_level&reason=r"),
    )
    .await;
    super::PRESSING.store(false, std::sync::atomic::Ordering::Release);
    assert_eq!(status, StatusCode::CONFLICT, "{busy}");
    assert!(busy.contains("quarantine_running"));
}

/// R4-18 repair 1: a press whose client LEAVES keeps the press until its
/// blocking work ends. The plan is a named pipe, so the work blocks reading
/// it: the test knows the work is running (its writer's open returns only
/// once the work has opened the pipe), aborts the client's task, and a
/// second press is then refused by name. Only after that does the test feed
/// the pipe, and the first press's work applies once and frees the press.
/// Before the repair the guard lived in the request's future, so aborting it
/// freed the press while `spawn_blocking` kept running, and the second press
/// ran beside the first.
///
/// ONCE THE FIRST WORK HOLDS THE PIPE, the pipe is moved aside and an
/// ordinary plan file takes its name, so a second press that a regression
/// admits reads a plain file and answers at once: the test then FAILS on its
/// status instead of hanging. With the pipe left at its name, the admitted
/// second press's work read the pipe too, and this test run against the
/// pre-repair route never ended: it was still running after 16 minutes and
/// was killed.
#[tokio::test]
async fn a_press_whose_client_leaves_holds_the_press_until_its_work_ends() {
    let _serial = PRESS_TESTS.lock().await;
    let fixture = Fixture::new("r418-press-left");
    let held = fixture.month(ym(2018, 3), 640, 4);
    let pipe = plan_file(&fixture.root);
    let named = pipe.clone();
    let made = std::process::Command::new("mkfifo")
        .arg(&pipe)
        .status()
        .expect("mkfifo runs");
    assert!(made.success(), "mkfifo made {}", pipe.display());
    let body = plan_body(&[held[2].ts_micros]);
    let plain = body.clone();
    let (opened_tx, opened_rx) = std::sync::mpsc::channel();
    let (go_tx, go_rx) = std::sync::mpsc::channel::<()>();
    let feeder = std::thread::spawn(move || {
        // Blocks until the press's work opens the pipe to read the plan.
        let mut file = fs::OpenOptions::new()
            .write(true)
            .open(&pipe)
            .expect("the pipe opens for writing");
        let _told = opened_tx.send(());
        // Fed when the test says go, or at once if the test ended first (its
        // sender dropped): either way the blocked work reads a whole plan.
        let _go = go_rx.recv();
        std::io::Write::write_all(&mut file, body.as_bytes()).expect("the plan is fed");
    });
    let form =
        format!("symbol={SYMBOL}&from=2018-03-01&to=2018-03-31&cause=foreign_level&reason=left");
    let first = tokio::spawn(quarantine_apply(
        axum::extract::State(Loaded::clone(&fixture.site)),
        form.clone(),
    ));
    let mut running = false;
    for _ in 0..600 {
        if opened_rx.try_recv().is_ok() {
            running = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    assert!(running, "the first press's work opened its plan");
    // The first work holds the pipe open; its name now goes to a plain plan.
    fs::rename(&named, named.with_extension("fifo")).expect("the pipe moves aside");
    fs::write(&named, plain).expect("a plain plan takes the pipe's name");
    // THE CLIENT LEAVES while the work is blocked reading the plan.
    first.abort();
    let left = first.await;
    assert!(
        left.as_ref()
            .is_err_and(tokio::task::JoinError::is_cancelled),
        "the client's task was cancelled: {left:?}"
    );
    let second = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        quarantine_apply(
            axum::extract::State(Loaded::clone(&fixture.site)),
            form.clone(),
        ),
    )
    .await;
    // Feed the pipe whatever the second press answered, so no thread waits.
    let _fed = go_tx.send(());
    let (status, _, busy) = second.expect("the second press answers at once, by name");
    assert_eq!(status, StatusCode::CONFLICT, "{busy}");
    assert!(busy.contains("quarantine_running"), "{busy}");
    assert!(feeder.join().is_ok(), "the feeder wrote the plan");
    // The first press's work finishes on its own and only then frees it.
    let mut free = false;
    for _ in 0..600 {
        if !super::PRESSING.load(std::sync::atomic::Ordering::Acquire) {
            free = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    assert!(free, "the press is freed when its work ends");
    let ledger = fs::read_to_string(fixture.root.join("audit").join("zerodha.quarantine.ledger"))
        .expect("the ledger");
    assert_eq!(
        ledger.matches("\tdropped\t").count(),
        1,
        "the left press's work applied once: {ledger}"
    );
}

/// THE SELF-CHECK OF REPAIR 2: while a writer held the feed's quarantine
/// lock, `/bars.json` answered from an index behind its ledger — the month
/// served, its bars and all, whatever the unindexed act said of it. Now the
/// read waits for the writer's image within its patience and answers "being
/// written, ask again" (503, `"being_written":true`) when it does not land,
/// and with no writer at work the lag is named stale.
#[tokio::test]
async fn bars_json_asks_again_while_a_quarantine_writer_brings_its_index_level() {
    let fixture = Fixture::new("r418-being-written");
    fixture.month(ym(2018, 3), 77_713, 3);
    // The lock, the ledger and the index exist.
    fixture.quarantine_whole(ym(2018, 2), 3);
    // THE LEDGER AHEAD OF ITS INDEX: an append in flight.
    let ledger = store::quarantine::ledger_path(&fixture.root, Vendor::Zerodha);
    let mut text = fs::read(&ledger).expect("the ledger");
    text.extend_from_slice(b"1792\tzerodha\tNSE/CASH/QFIXQ/1min\t2018-03\twhole_mo");
    fs::write(&ledger, &text).expect("a ledger ahead of its index");
    // A WRITER HOLDS THE FEED'S QUARANTINE LOCK.
    let lock = fs::File::open(fixture.root.join("audit").join("zerodha.quarantine.lock"))
        .expect("the quarantine lock");
    lock.lock().expect("held by a writer");
    let uri =
        format!("/bars.json?feed=zerodha&exchange=NSE&segment=CASH&symbol={SYMBOL}&month=2018-03");
    let (status, body) = get(&fixture.site, &uri).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{body}");
    assert_eq!(body["being_written"], true, "{body}");
    assert!(
        body["error"]
            .as_str()
            .is_some_and(|said| said.contains("quarantine_being_written")),
        "{body}"
    );
    assert!(
        !body.to_string().contains("77713"),
        "never its bars: {body}"
    );
    // THE WRITER GONE, THE INDEX STILL BEHIND: named stale.
    lock.unlock().expect("let go");
    let (status, stale) = get(&fixture.site, &uri).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "{stale}");
    assert_eq!(stale["code"], "quarantine_index_stale");
}

/// A MONTH DROPPED FOR ONE CAUSE AND THEN RECORDED WHOLE FOR ANOTHER (R4-18
/// repair 2, the second check's follow-up on `merged`): `/bars.json` names
/// every bar the two acts set aside and both causes, and the listing counts
/// them all. Before, the whole record kept only its own bars and cause, so
/// the answer said "all 2 of its bars are `zero_price`" of a month that set
/// aside 6, and `/instruments.json` under-counted by the bars dropped first.
#[tokio::test]
async fn bars_json_names_every_bar_and_every_cause_a_month_set_aside() {
    let fixture = Fixture::new("r418-every-cause");
    let held = fixture.month(ym(2018, 3), 77_713, 6);
    let id = pull::ingest::repair_plan::symbol_id(SYMBOL);
    let why = |cause| store::quarantine::Why {
        cause,
        who: "test",
        reason: "invented: two causes",
    };
    let foreign: Vec<i64> = held[..4].iter().map(|bar| bar.ts_micros).collect();
    let dropped = store::quarantine::apply_month(
        &fixture.root,
        path(ym(2018, 3)),
        id,
        &foreign,
        why(Cause::ForeignLevel),
    )
    .expect("four dropped");
    assert!(
        matches!(dropped, store::quarantine::Applied::Dropped { .. }),
        "{dropped:?}"
    );
    let rest: Vec<i64> = held[4..].iter().map(|bar| bar.ts_micros).collect();
    let whole = store::quarantine::apply_month(
        &fixture.root,
        path(ym(2018, 3)),
        id,
        &rest,
        why(Cause::ZeroPrice),
    )
    .expect("the rest recorded whole");
    assert!(
        matches!(
            whole,
            store::quarantine::Applied::WholeMonth { bars: 2, .. }
        ),
        "{whole:?}"
    );
    let uri =
        format!("/bars.json?feed=zerodha&exchange=NSE&segment=CASH&symbol={SYMBOL}&month=2018-03");
    let (status, body) = get(&fixture.site, &uri).await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(body["quarantine"]["scope"], "whole_month", "{body}");
    assert_eq!(body["quarantine"]["bars"], 6, "{body}");
    assert_eq!(body["quarantine"]["cause"], "zero_price", "{body}");
    assert_eq!(
        body["quarantine"]["causes"],
        serde_json::json!(["foreign_level", "zero_price"]),
        "{body}"
    );
    assert!(
        body["error"]
            .as_str()
            .is_some_and(|said| said.contains("all 6 of its bars are foreign_level and zero_price")),
        "{body}"
    );
    let key = brutex_core::instrument::InstrumentKey::cash(
        brutex_core::instrument::Exchange::Nse,
        SYMBOL,
    )
    .expect("a share");
    let summary = store::quarantine::Summary::load(&fixture.root, Vendor::Zerodha);
    let flag: Value = serde_json::from_str(&super::super::instruments_route::quarantine_of(
        &summary, &key,
    ))
    .expect("JSON");
    assert_eq!(
        (&flag["months"], &flag["whole_months"], &flag["bars"]),
        (
            &serde_json::json!(1),
            &serde_json::json!(1),
            &serde_json::json!(6)
        ),
        "{flag}"
    );
    assert_eq!(
        flag["causes"],
        serde_json::json!(["foreign_level", "zero_price"])
    );
}

/// **THE VERB'S OWN MISUSE IS A 400, ITS REFUSAL A 422, AND A QUESTION
/// EITHER DOOR CANNOT READ IS A 400 BEFORE THE VERB** (BASE-COV-API-AS2).
/// Extremes: (1) a plan whose FROM is no day is the verb's misuse, 400 with
/// exit 2; (2) a plan whose cause nobody names is the same; (3) a plan over a
/// store that holds no plan file is the verb's refusal, 422 with exit 1, and
/// applies nothing; (4) the plan door asked a field it does not know is a
/// 400 `quarantine_question` and the verb never runs; (5) the press door
/// asked the same; (6) a press that names no reason is refused at the door;
/// (7) an empty question names the first field it lacks; (8) a press with a
/// reason over a store with no plan file is the verb's refusal, 422, said as
/// applied. Combined: (9) a press with an unknown field AND no reason is
/// refused for the unknown field, the first thing read; (10) a plan whose
/// FROM is after its TO AND whose cause is unknown: the dates are read
/// first, so the misuse names them. Out of the box: (11) a symbol of 300
/// bytes: the verb refuses it as no symbol, a misuse, never a store read.
#[tokio::test]
async fn the_verbs_misuse_and_refusal_and_an_unread_question_are_said_apart() {
    let _serial = PRESS_TESTS.lock().await;
    let fixture = Fixture::new("as2-quarantine-codes");
    let plan_of = |query: &str| format!("/pull/quarantine.json?{query}");
    // (1), (2), (10), (11): the verb's misuse.
    for (query, said) in [
        (
            format!("symbol={SYMBOL}&from=2018-13-01&to=2018-03-31&cause=foreign_level"),
            "refused: ",
        ),
        (
            format!("symbol={SYMBOL}&from=2018-03-01&to=2018-03-31&cause=nobody_names_it"),
            "refused: ",
        ),
        (
            format!("symbol={SYMBOL}&from=2018-04-01&to=2018-03-31&cause=nobody_names_it"),
            "FROM 2018-04-01 is after TO 2018-03-31",
        ),
        (
            format!(
                "symbol={}&from=2018-03-01&to=2018-03-31&cause=foreign_level",
                "S".repeat(300)
            ),
            "refused: ",
        ),
    ] {
        let (status, body) = get(&fixture.site, &plan_of(&query)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{query}: {body}");
        assert_eq!(body["exit"], u64::from(cli::MISUSED), "{body}");
        assert_eq!(body["applied"], false, "{body}");
        assert!(body["report"].as_str().unwrap().contains(said), "{body}");
    }
    // (3): no plan file: the verb refuses.
    let (status, body) = get(
        &fixture.site,
        &plan_of(&format!(
            "symbol={SYMBOL}&from=2018-03-01&to=2018-03-31&cause=foreign_level"
        )),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(body["exit"], u64::from(cli::FAILED), "{body}");
    assert!(
        body["report"]
            .as_str()
            .unwrap()
            .contains("ZERODHA QUARANTINE REFUSED"),
        "{body}"
    );
    // (4), (7): the plan door's own refusals.
    for (query, said) in [
        (
            format!("symbol={SYMBOL}&from=2018-03-01&to=2018-03-31&cause=foreign_level&bogus=1"),
            "`bogus` is not a parameter of the quarantine",
        ),
        (String::new(), "name `symbol`"),
    ] {
        let (status, body) = get(&fixture.site, &plan_of(&query)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{query}: {body}");
        assert_eq!(body["code"], "quarantine_question", "{body}");
        assert_eq!(body["error"], said, "{body}");
    }
    // (5), (6), (9): the press door's own refusals.
    for (form, said) in [
        (
            format!(
                "symbol={SYMBOL}&from=2018-03-01&to=2018-03-31&cause=foreign_level&reason=r&bogus=1"
            ),
            "`bogus` is not a parameter of the quarantine",
        ),
        (
            format!("symbol={SYMBOL}&from=2018-03-01&to=2018-03-31&cause=foreign_level"),
            "",
        ),
        (
            format!("symbol={SYMBOL}&from=2018-03-01&to=2018-03-31&cause=foreign_level&bogus=1"),
            "`bogus` is not a parameter of the quarantine",
        ),
    ] {
        let (status, _, text) =
            quarantine_apply(axum::extract::State(Loaded::clone(&fixture.site)), form).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{text}");
        let body: Value = serde_json::from_str(&text).expect("JSON");
        assert_eq!(body["code"], "quarantine_question", "{body}");
        assert!(body["error"].as_str().unwrap().contains(said), "{body}");
    }
    // (8): a press over a store with no plan file.
    let before = files_under(&fixture.root);
    let (status, _, text) = quarantine_apply(
        axum::extract::State(Loaded::clone(&fixture.site)),
        format!("symbol={SYMBOL}&from=2018-03-01&to=2018-03-31&cause=foreign_level&reason=r"),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{text}");
    let body: Value = serde_json::from_str(&text).expect("JSON");
    assert_eq!(body["exit"], u64::from(cli::FAILED), "{body}");
    assert_eq!(body["applied"], true, "{body}");
    assert_eq!(
        files_under(&fixture.root),
        before,
        "a refused press wrote nothing"
    );
    // A folder that is not there holds no file.
    assert!(files_under(&fixture.root.join("not-there")).is_empty());
    let _ = fs::remove_dir_all(&fixture.root);
}

/// **A PRESS REFUSED AT A FULL OPERATIONS POOL IS A 429 THAT NAMES THAT
/// POOL'S BOUND, AND THE REFUSED PRESS LEAVES THE PRESS FREE**
/// (BASE-COV-API-AS2): the press runs in the operations pool, so its refusal
/// names the slots that pool has. It named the store-read pool's, eight on a
/// machine of eight cores or more where the operations pool has four (on this
/// host's four cores the two agree). The press guard moved into the refused
/// work is dropped with it, so the next press runs.
///
/// Extremes: (1) every operations slot counted taken is a 429; (2) the body
/// names the quarantine; (3) and the operations pool's slots on this
/// machine; (4) and says to retry; (5) nothing in the store changes; (6) the
/// press is free once the refusal is answered; (7) the pool freed, the same
/// press applies; (8) and drops exactly the planned bar. Combined: (9) the
/// operations pool full and the store-read pool free: the plan is answered
/// while the press is refused; (10) the operations pool full and a press
/// whose question is unreadable: the 400 comes first, before the pool. Out of
/// the box: (11) the operations pool full and another press marked running:
/// the one-writer 409 is asked before the pool, and the refused press does
/// not clear the other's mark.
#[tokio::test]
async fn a_press_refused_by_a_full_operations_pool_names_its_bound_and_frees_the_press() {
    let _serial = PRESS_TESTS.lock().await;
    let _apart = crate::detail::apart_from_slot_owners().await;
    let fixture = Fixture::new("r418-press-crowded");
    let held = fixture.month(ym(2018, 3), 520, 5);
    plan(&fixture.root, &[held[1].ts_micros]);
    let form = format!(
        "symbol={SYMBOL}&from=2018-03-01&to=2018-03-31&cause=foreign_level&reason=invented"
    );
    let press =
        |form: String| quarantine_apply(axum::extract::State(Loaded::clone(&fixture.site)), form);
    let before = files_under(&fixture.root);

    let crowd = crate::detail::crowd(crate::detail::Named::OpsRead);
    let (status, _, refused) = press(form.clone()).await;
    let pressing_after = super::PRESSING.load(std::sync::atomic::Ordering::Acquire);
    let (plan_status, plan_body) = get(
        &fixture.site,
        &format!("/pull/quarantine.json?symbol={SYMBOL}&from=2018-03-01&to=2018-03-31&cause=foreign_level"),
    )
    .await;
    let (unread_status, _, unread) = press("symbol=".to_owned()).await;
    super::PRESSING.store(true, std::sync::atomic::Ordering::Release);
    let (running_status, _, running) = press(form.clone()).await;
    let still_marked = super::PRESSING.load(std::sync::atomic::Ordering::Acquire);
    super::PRESSING.store(false, std::sync::atomic::Ordering::Release);
    drop(crowd);

    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS, "(1) {refused}");
    let said: Value = serde_json::from_str(&refused).expect("JSON");
    let why = said["error"].as_str().unwrap_or_default();
    assert!(why.starts_with("the quarantine not admitted"), "(2) {why}");
    let slots = crate::detail::Named::OpsRead.pool().max();
    assert!(
        why.contains(&format!("at most {slots} run at once")),
        "(3) {why}"
    );
    assert!(why.ends_with("retry"), "(4) {why}");
    assert_eq!(
        files_under(&fixture.root),
        before,
        "(5) the store is as it was"
    );
    assert!(!pressing_after, "(6) the refused press left the press free");
    assert_eq!(plan_status, StatusCode::OK, "(9) {plan_body}");
    assert_eq!(plan_body["applied"], false, "(9) {plan_body}");
    assert_eq!(unread_status, StatusCode::BAD_REQUEST, "(10) {unread}");
    assert!(unread.contains("quarantine_question"), "(10) {unread}");
    assert_eq!(running_status, StatusCode::CONFLICT, "(11) {running}");
    assert!(running.contains("quarantine_running"), "(11) {running}");
    assert!(still_marked, "(11) the other press's mark stands");

    let (status, _, text) = press(form).await;
    assert_eq!(status, StatusCode::OK, "(7) {text}");
    let body: Value = serde_json::from_str(&text).expect("JSON");
    assert_eq!(body["applied"], true, "(7) {body}");
    assert!(
        body["report"]
            .as_str()
            .unwrap_or_default()
            .contains("DROPPED 1 of 5"),
        "(8) {body}"
    );
}

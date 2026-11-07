#![cfg(test)]
//! Tests of `/provenance.json` (WP-38b; D-3334; DB-078, DB-G56).
//!
//! EVERY FIXTURE HERE IS INVENTED. Prices are a few rupees, journal counts
//! are small round numbers, archive paths and run ids are made up: the shapes
//! are the importer's and the pull journal's, the numbers are not the
//! market's or the operator's.

#![allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "a test that cannot panic cannot fail, and these lints exist to keep panics out \
              of the crate rather than out of its tests"
)]

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use axum::http::StatusCode;
use brutex_core::instrument::Contract;
use brutex_core::vendor::Vendor;
use pull::session::{Day, Window};
use serde_json::Value;
use store::file::{Appended, BarFile};
use store::format::{Bar, OI_NULL};
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};
use store::provenance::{Landing, Source, SourceKind};

use super::*;

/// A store root of its own.
fn root_of(name: &str) -> PathBuf {
    let root = crate::scratch::path(&format!("provenance-{name}"));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("a scratch root");
    root
}

fn day(text: &str) -> Day {
    super::super::import_journal::parse_day(text).expect("a day")
}

/// IST midnight opening `on`, in UNIX seconds.
fn ist_midnight(on: &str) -> i64 {
    i64::from(day(on).days_from_epoch()) * 86_400 - pull::session::IST_OFFSET_SECS
}

/// One invented bar at `secs`, priced a few rupees.
fn bar(secs: i64, close: i64) -> Bar {
    Bar {
        ts_micros: secs * 1_000_000,
        open: close,
        high: close + 3,
        low: close - 3,
        close,
        volume: 0,
        open_interest: OI_NULL,
    }
}

/// `n` one-second bars from 09:15:00 IST on `on`.
fn seconds(on: &str, n: i64) -> Vec<Bar> {
    let open = ist_midnight(on) + 9 * 3_600 + 15 * 60;
    (0..n).map(|i| bar(open + i, 7_77 + i)).collect()
}

/// The symbol id `pull::ingest` stamps, folded the way the store checks it.
fn symbol_id(symbol: &str) -> u32 {
    u32::try_from(brutex_core::universe::fnv1a(symbol) & 0xFFFF_FFFF).unwrap()
}

/// One month's bar path.
fn path_of<'a>(
    feed: Vendor,
    segment: &'a str,
    symbol: &'a str,
    contract: Option<Contract>,
    rung: Timeframe,
    month: &str,
) -> StorePath<'a> {
    let (year, mm) = month.split_once('-').unwrap();
    StorePath::new(PathParts {
        vendor: feed,
        exchange: "NSE",
        segment,
        symbol,
        contract,
        timeframe: rung,
        month: YearMonth::new(year.parse().unwrap(), mm.parse().unwrap()).unwrap(),
        file: FileKind::Bars,
    })
    .unwrap()
}

/// Appends each batch as its own landing and closes the file, so its lock is
/// released as a finished landing's is.
fn land(root: &Path, path: StorePath<'_>, symbol: &str, batches: &[Vec<Bar>]) -> Vec<Appended> {
    let mut file = BarFile::open_or_create(root, path, symbol_id(symbol)).expect("a bar file");
    let mut appended = Vec::new();
    for batch in batches {
        appended.push(file.append(batch).expect("the bars append"));
    }
    appended
}

/// Writes `text` as the store's GDFL import journal.
fn gdfl_journal(root: &Path, text: &str) {
    let path = pull::gdfl_import::journal_path(root);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

/// A month with no sidecar has none of these lines' run; one has its run line.
const NIFTY_MARCH: &str = "\
begin indices 2019-03-01 *
done indices 2019-03-01 * files=2 seconds=3 failures=0 late=0 late_unresolved=0 max_back_s=0
begin stocks 2019-03-01 *
done stocks 2019-03-01 * definition=2 files=5 seconds=9 failures=0 late=0 late_unresolved=0 max_back_s=0
run 1791300000 source=zip archive=/archives/cm%20outer.zip archive_bytes=7 commit=abc started=1791300000 archive_cd_blake3=00
begin indices 2019-03-05 *
done indices 2019-03-05 * definition=2 files=2 seconds=2 failures=0 late=0 late_unresolved=0 max_back_s=0 run=1791300000
begin indices 2019-04-01 *
done indices 2019-04-01 * definition=2 files=2 seconds=4 failures=0 late=0 late_unresolved=0 max_back_s=0
begin indices 2019-03-06 BANKNIFTY
done indices 2019-03-06 BANKNIFTY definition=2 files=1 seconds=1 failures=0 late=0 late_unresolved=0 max_back_s=0
";

/// GDFL NIFTY 1s 2019-03: three bars on the 1st, two on the 5th, each day
/// one landing.
fn nifty_march(root: &Path) -> Vec<Appended> {
    gdfl_journal(root, NIFTY_MARCH);
    let path = path_of(
        Vendor::Gdfl,
        "INDEX",
        "NIFTY",
        None,
        Timeframe::SECOND_1,
        "2019-03",
    );
    land(
        root,
        path,
        "NIFTY",
        &[seconds("2019-03-01", 3), seconds("2019-03-05", 2)],
    )
}

fn ask(query: &str) -> Asked {
    asked(query).unwrap_or_else(|why| panic!("{query}: {why}"))
}

fn get(root: &Path, query: &str) -> Answer {
    answer(root, &ask(query))
}

const GDFL_NIFTY: &str = "feed=gdfl&instrument=NSE-INDEX-NIFTY&timeframe=1s&month=2019-03";

/// A landing as a GDFL importer would state it.
fn member_landing<'a>(name: &'a str, on: &str, line: u64) -> Landing<'a> {
    Landing {
        source: Source {
            kind: SourceKind::ArchiveMember,
            name,
            bytes: 1_234,
            crc32: 0xDEAD_BEEF,
        },
        day: store::provenance::Day::from_yyyymmdd(on.replace('-', "").parse().unwrap()).unwrap(),
        bar_definition: 2,
        journal_ordinal: line,
        commit: Some("0123456789abcdef0123456789abcdef01234567"),
        landed_at_micros: 1_791_300_000_000_000,
    }
}

// ══ The request ═════════════════════════════════════════════════════════════

/// **EVERY PARAMETER BUT `t` IS REQUIRED, AND EACH REFUSAL NAMES ITS
/// PARAMETER**: nothing is defaulted, a contract belongs to FNO only, an FNO
/// month needs its contract, and `t` must be a whole number of seconds inside
/// the asked month's IST days.
#[test]
fn every_parameter_is_required_and_each_refusal_names_it() {
    for (query, names) in [
        (
            "instrument=NSE-INDEX-NIFTY&timeframe=1s&month=2019-03",
            "feed=",
        ),
        ("feed=gdfl&timeframe=1s&month=2019-03", "instrument="),
        (
            "feed=gdfl&instrument=NSE-INDEX-NIFTY&month=2019-03",
            "timeframe=",
        ),
        (
            "feed=gdfl&instrument=NSE-INDEX-NIFTY&timeframe=1s",
            "month=",
        ),
        (
            "feed=nse&instrument=NSE-INDEX-NIFTY&timeframe=1s&month=2019-03",
            "feed=\"nse\"",
        ),
        (
            "feed=gdfl&instrument=NIFTY&timeframe=1s&month=2019-03",
            "census key",
        ),
        (
            "feed=gdfl&instrument=NSE-ETF-NIFTY&timeframe=1s&month=2019-03",
            "INDEX, CASH, FNO",
        ),
        (
            "feed=gdfl&instrument=NSE-INDEX-NIFTY-2019-03-28-1150000-CE&timeframe=1s&month=2019-03",
            "FNO only",
        ),
        (
            "feed=gdfl&instrument=NSE-FNO-NIFTY&timeframe=1s&month=2019-03",
            "names no contract",
        ),
        (
            "feed=gdfl&instrument=NSE-INDEX-NIFTY&timeframe=7s&month=2019-03",
            "\"7s\"",
        ),
        (
            "feed=gdfl&instrument=NSE-INDEX-NIFTY&timeframe=1s&month=2019-3x",
            "YYYY-MM",
        ),
        (&format!("{GDFL_NIFTY}&t=12.5"), "whole number"),
        (&format!("{GDFL_NIFTY}&t=0012"), "whole number"),
        // 2019-04-01 09:15 IST is April's.
        (
            &format!("{GDFL_NIFTY}&t={}", ist_midnight("2019-04-01") + 33_300),
            "not in month=2019-03",
        ),
    ] {
        let why = asked(query).expect_err(query);
        assert!(why.contains(names), "{query}: {why}");
    }
}

/// **A DAY BAR IS STAMPED AT IST MIDNIGHT, WHICH IS THE PREVIOUS UTC DAY**:
/// 2019-03-01 00:00 IST is 2019-02-28 18:30 UTC, and it is March's bar. An
/// option key parses whole, its contract inside it.
#[test]
fn a_day_bar_stamped_at_ist_midnight_belongs_to_its_ist_month() {
    let midnight = ist_midnight("2019-03-01");
    let got = ask(&format!(
        "feed=zerodha&instrument=NSE-INDEX-NIFTY&timeframe=1day&month=2019-03&t={midnight}"
    ));
    let t = got.t.expect("t");
    assert_eq!(
        (t.secs, t.day.to_string()),
        (midnight, "2019-03-01".to_owned())
    );
    assert_eq!(t.micros, midnight * 1_000_000);
    let option =
        ask("feed=GDFL&instrument=NSE-FNO-NIFTY-2019-03-28-1150000-CE&timeframe=1s&month=2019-03");
    assert_eq!(option.feed, Vendor::Gdfl);
    assert_eq!(option.symbol, "NIFTY");
    assert_eq!(option.key(), "NSE-FNO-NIFTY-2019-03-28-1150000-CE");
    assert_eq!(option.segment, Segment::Fno);
    let late = asked(&format!(
        "feed=zerodha&instrument=NSE-INDEX-NIFTY&timeframe=1day&month=2019-02&t={midnight}"
    ))
    .expect_err("IST March, UTC February");
    assert!(late.contains("in 2019-03"), "{late}");
}

// ══ GDFL ════════════════════════════════════════════════════════════════════

/// **AN ABSENT MONTH IS A 404 NAMING THE FILE LOOKED FOR**, never an empty
/// provenance.
#[test]
fn an_absent_month_is_a_404_naming_its_path() {
    let root = root_of("absent");
    let got = get(&root, GDFL_NIFTY);
    assert_eq!(got.status, StatusCode::NOT_FOUND, "{}", got.body);
    assert_eq!(got.body["held"], false);
    assert_eq!(got.body["path"], "bars/gdfl/NSE/INDEX/NIFTY/1s/2019-03.bin");
    assert!(got.body["error"].as_str().unwrap().contains("not held"));
}

/// **NIFTY FIRST: A GDFL MONTH WITH NO SIDECAR IS REBUILT FROM ITS IMPORT
/// DAYS AND LABELLED SO** (DB-078, DB-G56; D-3334 item 3). The month's index
/// import lines -- never a stock line, never April's, never another filter's --
/// with their counts; the run line a close names, with its archive; and for
/// a line written before D-3317 the plain statement that its archive was not
/// recorded, with only where this build's reader finds the day.
#[test]
fn a_gdfl_month_with_no_sidecar_is_rebuilt_from_its_import_days() {
    let root = root_of("gdfl-rebuilt");
    let _month = nifty_march(&root);
    let got = get(&root, GDFL_NIFTY);
    assert_eq!(got.status, StatusCode::OK, "{}", got.body);
    let body = &got.body;
    assert_eq!(body["reconstructed"], true);
    assert_eq!(body["label"], RECONSTRUCTED);
    assert_eq!(body["sidecar"]["state"], "absent");
    assert_eq!(
        body["sidecar"]["path"],
        "bars/gdfl/NSE/INDEX/NIFTY/1s/2019-03.prv"
    );
    assert_eq!(body["bars"]["n_valid"], 5);
    assert_eq!(body["bars"]["generation"], 2);
    assert_eq!(
        body["unrecorded"],
        serde_json::json!([{ "first_index": 0, "end": 5 }])
    );
    assert_eq!(body["recorded"], serde_json::json!([]));
    assert!(body["window"].is_null(), "GDFL has no operator window");
    let rebuilt = &body["reconstruction"];
    assert_eq!(rebuilt["source"], "imports/gdfl.journal");
    assert_eq!(rebuilt["kind"], "indices");
    assert_eq!(rebuilt["member_digest"], "unknown");
    let days = rebuilt["days"].as_array().unwrap();
    let shown: Vec<(&str, u64, u64)> = days
        .iter()
        .map(|row| {
            (
                row["day"].as_str().unwrap(),
                row["line"].as_u64().unwrap(),
                row["seconds"].as_u64().unwrap(),
            )
        })
        .collect();
    assert_eq!(shown, [("2019-03-01", 2, 3), ("2019-03-05", 7, 2)]);
    assert_eq!(days[0]["outcome"], "done");
    assert_eq!(days[0]["files"], 2);
    assert!(days[0]["run"].is_null());
    assert_eq!(days[1]["run"], 1_791_300_000_u64);
    assert_eq!(days[1]["definition"], 2);
    let files = &days[0]["vendor_files"];
    assert_eq!(files["day_folder"], "GFDLCM_INDICES_TICK_01032019");
    assert_eq!(
        files["zip_day"],
        "INDICES/2019/MAR_2019/GFDLCM_INDICES_TICK_01032019.zip"
    );
    assert_eq!(
        files["tickstore_day"],
        "cm/INDICES/2019/MAR_2019/GFDLCM_INDICES_TICK_01032019.bts"
    );
    assert_eq!(files["stem"], "NIFTY 50.NSE_IDX");
    assert_eq!(
        days[0]["bars"],
        serde_json::json!({ "first_index": 0, "end": 3, "count": 3 })
    );
    assert_eq!(
        days[1]["bars"],
        serde_json::json!({ "first_index": 3, "end": 5, "count": 2 })
    );
    assert_eq!(rebuilt["journal_days"], 2);
    assert_eq!(rebuilt["journal_days_done"], 2);
    assert_eq!(rebuilt["journal_days_with_bars"], 2);
    assert_eq!(rebuilt["bars_on_journal_days"], 5);
    assert_eq!(rebuilt["bars_on_no_journal_day"], 0);
    assert_eq!(rebuilt["bar_lookup_errors"], 0);
    assert_eq!(rebuilt["bar_file_commits"], 2);
    assert_eq!(rebuilt["commits_agree"], true);
    let archive = &rebuilt["archive"];
    assert_eq!(archive["recorded"], false, "one day names no run");
    assert_eq!(archive["days_naming_no_run"], 1);
    assert!(archive["why"].as_str().unwrap().contains("D-3317"));
    let runs = archive["runs"].as_array().unwrap();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0]["line"], 5);
    assert_eq!(runs[0]["fields"]["archive"], "/archives/cm outer.zip");
    assert_eq!(runs[0]["fields"]["source"], "zip");
}

/// **WITH `t`, THE ONE IMPORT DAY THAT LANDED `t`'s BAR**, the bar's index,
/// and no month-wide commit comparison; a `t` no bar is stamped at is a 404
/// naming the bar after it.
#[test]
fn with_t_the_answer_is_the_import_day_of_that_bar() {
    let root = root_of("gdfl-t");
    let _month = nifty_march(&root);
    let fifth = ist_midnight("2019-03-05") + 9 * 3_600 + 15 * 60;
    let got = get(&root, &format!("{GDFL_NIFTY}&t={}", fifth + 1));
    assert_eq!(got.status, StatusCode::OK, "{}", got.body);
    assert_eq!(got.body["bar"]["index"], 4);
    let rebuilt = &got.body["reconstruction"];
    let days: Vec<&str> = rebuilt["days"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["day"].as_str().unwrap())
        .collect();
    assert_eq!(days, ["2019-03-05"]);
    assert_eq!(rebuilt["archive"]["recorded"], true);
    assert!(rebuilt["commits_agree"].is_null());
    let missing = get(&root, &format!("{GDFL_NIFTY}&t={}", fifth + 9));
    assert_eq!(missing.status, StatusCode::NOT_FOUND, "{}", missing.body);
    assert!(
        missing.body["error"]
            .as_str()
            .unwrap()
            .contains("every one of its 5 bars is earlier"),
        "{}",
        missing.body
    );
    let between = get(&root, &format!("{GDFL_NIFTY}&t={}", fifth - 60));
    assert_eq!(between.status, StatusCode::NOT_FOUND);
    assert!(
        between.body["error"].as_str().unwrap().contains("bar 3"),
        "{}",
        between.body
    );
}

/// **A RECORDED MONTH SERVES ITS SIDECAR, AND THE JOURNAL LINE EACH RECORD
/// NAMES IS READ BESIDE IT** (D-3334 item 3). Nothing is reconstructed.
#[test]
fn a_recorded_month_serves_its_records_and_their_journal_rows() {
    let root = root_of("gdfl-recorded");
    gdfl_journal(&root, NIFTY_MARCH);
    let path = path_of(
        Vendor::Gdfl,
        "INDEX",
        "NIFTY",
        None,
        Timeframe::SECOND_1,
        "2019-03",
    );
    let mut bars = BarFile::open_or_create(&root, path, symbol_id("NIFTY")).unwrap();
    let names = [
        "cm.zip!INDICES/2019/MAR_2019/GFDLCM_INDICES_TICK_01032019.zip!x/NIFTY 50.NSE_IDX.csv",
        "cm.zip!INDICES/2019/MAR_2019/GFDLCM_INDICES_TICK_05032019.zip!x/NIFTY 50.NSE_IDX.csv",
    ];
    // Each landing is recorded right after its own append, as a writer does:
    // `record` refuses an append that is not the handle's newest commit.
    for ((batch, name), (on, line)) in [seconds("2019-03-01", 3), seconds("2019-03-05", 2)]
        .iter()
        .zip(names)
        .zip([("2019-03-01", 2), ("2019-03-05", 7)])
    {
        let landed = bars.append(batch).unwrap();
        store::provenance::record(&root, &bars, path, landed, &member_landing(name, on, line))
            .unwrap();
    }
    drop(bars);
    let got = get(&root, GDFL_NIFTY);
    assert_eq!(got.status, StatusCode::OK, "{}", got.body);
    let body = &got.body;
    assert_eq!(body["reconstructed"], false);
    assert_eq!(body["label"], "recorded");
    assert!(body["reconstruction"].is_null());
    assert_eq!(body["sidecar"]["state"], "present");
    assert_eq!(body["sidecar"]["records"], 2);
    assert_eq!(body["unrecorded"], serde_json::json!([]));
    let records = body["recorded"].as_array().unwrap();
    assert_eq!(records.len(), 2);
    let second = &records[1];
    assert_eq!(second["source_kind"], "archive_member");
    assert_eq!(second["source_name"], names[1]);
    assert_eq!(second["source_crc32"], "deadbeef");
    assert_eq!(second["source_bytes"], 1_234);
    assert_eq!(second["day"], "2019-03-05");
    assert_eq!(
        (second["first_index"].as_u64(), second["end"].as_u64()),
        (Some(3), Some(5))
    );
    assert_eq!(second["bar_file_generation"], 2);
    assert_eq!(second["commit"], "0123456789abcdef0123456789abcdef01234567");
    assert_eq!(second["journal"]["line_named"], 7);
    assert_eq!(second["journal"]["day_rows"][0]["line"], 7);
    assert_eq!(second["journal"]["day_rows"][0]["outcome"], "done");

    let fifth = ist_midnight("2019-03-05") + 9 * 3_600 + 15 * 60;
    let one = get(&root, &format!("{GDFL_NIFTY}&t={fifth}"));
    assert_eq!(one.status, StatusCode::OK, "{}", one.body);
    let shown = one.body["recorded"].as_array().unwrap();
    assert_eq!(shown.len(), 1);
    assert_eq!(shown[0]["ordinal"], 1);
    assert_eq!(one.body["reconstructed"], false);
}

/// **BARS LANDED BEFORE THE SIDECAR EXISTED ARE REBUILT, AND THE RECORDED
/// ONES ARE NOT**: the month names its unrecorded range and says it is both;
/// `t` in the unrecorded day is reconstructed alone, `t` in the recorded day
/// is recorded alone.
#[test]
fn bars_landed_before_the_sidecar_are_rebuilt_and_recorded_ones_are_not() {
    let root = root_of("gdfl-mixed");
    gdfl_journal(&root, NIFTY_MARCH);
    let path = path_of(
        Vendor::Gdfl,
        "INDEX",
        "NIFTY",
        None,
        Timeframe::SECOND_1,
        "2019-03",
    );
    let mut bars = BarFile::open_or_create(&root, path, symbol_id("NIFTY")).unwrap();
    let _unrecorded = bars.append(&seconds("2019-03-01", 3)).unwrap();
    let landed = bars.append(&seconds("2019-03-05", 2)).unwrap();
    store::provenance::record(
        &root,
        &bars,
        path,
        landed,
        &member_landing("m.zip!a.csv", "2019-03-05", 7),
    )
    .unwrap();
    drop(bars);
    let got = get(&root, GDFL_NIFTY);
    let body = &got.body;
    assert_eq!(
        body["unrecorded"],
        serde_json::json!([{ "first_index": 0, "end": 3 }])
    );
    assert_eq!(body["reconstructed"], true);
    assert!(
        body["label"].as_str().unwrap().starts_with("recorded for"),
        "{body}"
    );
    assert!(body["label"].as_str().unwrap().contains(RECONSTRUCTED));
    assert_eq!(body["recorded"].as_array().unwrap().len(), 1);
    assert_eq!(body["reconstruction"]["days"].as_array().unwrap().len(), 2);

    let first = ist_midnight("2019-03-01") + 9 * 3_600 + 15 * 60;
    let old = get(&root, &format!("{GDFL_NIFTY}&t={}", first + 2));
    assert_eq!(old.body["reconstructed"], true);
    assert_eq!(old.body["label"], RECONSTRUCTED);
    assert_eq!(old.body["recorded"], serde_json::json!([]));
    assert_eq!(old.body["reconstruction"]["days"][0]["day"], "2019-03-01");
    let fifth = ist_midnight("2019-03-05") + 9 * 3_600 + 15 * 60;
    let new = get(&root, &format!("{GDFL_NIFTY}&t={fifth}"));
    assert_eq!(new.body["reconstructed"], false);
    assert_eq!(new.body["label"], "recorded");
}

/// **A SIDECAR THE STORE REFUSES IS NAMED AND NEVER SERVED**: a damaged
/// record before a valid one is refused by name, no record is shown, and the
/// month is reconstructed under that refusal.
#[test]
fn a_damaged_sidecar_is_named_and_never_served() {
    let root = root_of("gdfl-damaged");
    gdfl_journal(&root, NIFTY_MARCH);
    let path = path_of(
        Vendor::Gdfl,
        "INDEX",
        "NIFTY",
        None,
        Timeframe::SECOND_1,
        "2019-03",
    );
    let mut bars = BarFile::open_or_create(&root, path, symbol_id("NIFTY")).unwrap();
    for (batch, on) in [
        (seconds("2019-03-01", 3), "2019-03-01"),
        (seconds("2019-03-05", 2), "2019-03-05"),
    ] {
        let landed = bars.append(&batch).unwrap();
        store::provenance::record(
            &root,
            &bars,
            path,
            landed,
            &member_landing("m.zip!a.csv", on, 2),
        )
        .unwrap();
    }
    drop(bars);
    let prv = path.with_file(FileKind::Provenance).to_path_buf(&root);
    let mut bytes = std::fs::read(&prv).unwrap();
    // Byte 70 lies inside record 0's `first_index`.
    bytes[64 + 70] ^= 0xFF;
    std::fs::write(&prv, bytes).unwrap();
    let got = get(&root, GDFL_NIFTY);
    let body = &got.body;
    assert_eq!(got.status, StatusCode::OK);
    assert_eq!(body["sidecar"]["state"], "refused");
    assert!(
        body["sidecar"]["error"]
            .as_str()
            .unwrap()
            .contains("record 0 fails its checksum"),
        "{body}"
    );
    assert_eq!(body["recorded"], serde_json::json!([]));
    assert_eq!(body["reconstructed"], true);
    assert_eq!(body["label"], RECONSTRUCTED);
}

/// **A STOCK'S DAY IS NAMED BY ITS STOCK FILE, AN OPTION'S BY ITS DAY FOLDER
/// ALONE**: the same reconstruction over the shared code path for a share and
/// an illiquid strike, each from its own import kind's lines.
#[test]
fn a_share_and_an_option_are_rebuilt_from_their_own_import_kind() {
    let root = root_of("gdfl-kinds");
    gdfl_journal(
        &root,
        "begin stocks 2019-03-01 *\n\
         done stocks 2019-03-01 * definition=2 files=5 seconds=9 failures=0 late=0 late_unresolved=0 max_back_s=0\n\
         begin options 2019-03-01 *\n\
         done options 2019-03-01 * definition=2 files=40 seconds=90 failures=0 late=0 late_unresolved=0 max_back_s=0\n",
    );
    let share = path_of(
        Vendor::Gdfl,
        "CASH",
        "RELIANCE",
        None,
        Timeframe::SECOND_1,
        "2019-03",
    );
    let _share = land(&root, share, "RELIANCE", &[seconds("2019-03-01", 2)]);
    let got = get(
        &root,
        "feed=gdfl&instrument=NSE-CASH-RELIANCE&timeframe=1s&month=2019-03",
    );
    let rebuilt = &got.body["reconstruction"];
    assert_eq!(rebuilt["kind"], "stocks", "{}", got.body);
    assert_eq!(rebuilt["days"][0]["line"], 2);
    assert_eq!(rebuilt["days"][0]["vendor_files"]["stem"], "RELIANCE.NSE");
    assert_eq!(
        rebuilt["days"][0]["vendor_files"]["day_folder"],
        "GFDLCM_STOCK_TICK_01032019"
    );

    let contract = Contract::parse("2019-03-28-1975000-PE").unwrap();
    let option = path_of(
        Vendor::Gdfl,
        "FNO",
        "NIFTY",
        Some(contract),
        Timeframe::SECOND_1,
        "2019-03",
    );
    let _option = land(&root, option, "NIFTY", &[seconds("2019-03-01", 1)]);
    let got = get(
        &root,
        "feed=gdfl&instrument=NSE-FNO-NIFTY-2019-03-28-1975000-PE&timeframe=1s&month=2019-03",
    );
    let rebuilt = &got.body["reconstruction"];
    assert_eq!(rebuilt["kind"], "options", "{}", got.body);
    assert_eq!(rebuilt["days"][0]["line"], 4);
    assert_eq!(
        rebuilt["days"][0]["vendor_files"]["day_folder"],
        "GFDLNFO_TICK_01032019"
    );
    assert!(rebuilt["days"][0]["vendor_files"]["stem"].is_null());
}

/// **A CLOSE NAMING A RUN THE JOURNAL DOES NOT HOLD IS SAID**, never shown
/// as some other run's archive.
#[test]
fn a_close_naming_an_absent_run_line_is_said() {
    let root = root_of("gdfl-norun");
    gdfl_journal(
        &root,
        "begin indices 2019-03-01 *\n\
         done indices 2019-03-01 * definition=2 files=2 seconds=3 failures=0 late=0 late_unresolved=0 max_back_s=0 run=42\n",
    );
    let path = path_of(
        Vendor::Gdfl,
        "INDEX",
        "NIFTY",
        None,
        Timeframe::SECOND_1,
        "2019-03",
    );
    let _month = land(&root, path, "NIFTY", &[seconds("2019-03-01", 3)]);
    let got = get(&root, GDFL_NIFTY);
    let run = &got.body["reconstruction"]["archive"]["runs"][0];
    assert_eq!(run["id"], 42, "{}", got.body);
    assert!(run["line"].is_null());
    assert!(run["error"].as_str().unwrap().contains("no run line"));
    let archive = &got.body["reconstruction"]["archive"];
    assert_eq!(
        archive["recorded"], false,
        "a named run that is absent is not a record"
    );
    assert_eq!(archive["runs_named_and_absent"], 1);
    assert_eq!(archive["days_naming_no_run"], 0);
}

// ══ A pulled feed ═══════════════════════════════════════════════════════════

/// When the invented runs below start: an instant in October 2026.
const T0: i64 = 1_791_200_000;

const KITE: &str = "https://api.kite.trade/instruments/historical/:instrument_token/day";

fn at(secs: i64) -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(u64::try_from(secs).unwrap())
}

fn window(from: &str, to: &str) -> Window {
    Window::new(day(from), day(to)).unwrap()
}

/// A run record: started at `start`, took `took` seconds, offered `bars`.
fn run(
    scope: audit::Scope,
    outcome: audit::Outcome,
    start: i64,
    took: u64,
    source: &str,
    span: Option<(&str, &str)>,
    bars: u64,
) -> audit::Record {
    let mut record = audit::Record::refused(scope, outcome, at(start), source, "invented note");
    if let Some((from, to)) = span {
        record = record.with_window(window(from, to));
    }
    record = record.with_counts(3, bars, bars, 0);
    record.elapsed_micros = took * 1_000_000;
    record
}

/// The pull journal of the Zerodha fixtures, in this order:
///
/// 0. refused with no window;
/// 1. not started, naming no vendor, window 2015..2020;
/// 2. **the 2015..2020 Kite run**, started `T0`, 193 s;
/// 3. a member failure of that run;
/// 4. a Kite run over the operator's window, started `T0 + 210`;
/// 5. a cash-identity run over the operator's window;
/// 6. an expired-F&O run over 2015..2016;
/// 7. a Kite run over 2015-01..2015-02 that offered nothing.
fn pull_journal(root: &Path) {
    use audit::{Outcome, Scope};
    let journal = audit::Journal::at(root);
    let identity = "zerodha_nse_checked NOT vendor-ISIN; invented";
    for record in [
        run(
            Scope::Spot,
            Outcome::Refused,
            T0 - 900,
            0,
            "indices",
            None,
            0,
        ),
        run(
            Scope::Spot,
            Outcome::NotStarted,
            T0 - 800,
            0,
            "Reference indices",
            Some(("2015-01-01", "2020-12-31")),
            0,
        ),
        run(
            Scope::Spot,
            Outcome::Failed,
            T0,
            193,
            KITE,
            Some(("2015-01-01", "2020-12-31")),
            600,
        ),
        audit::Record::member_failure(
            Scope::Spot,
            at(T0),
            "NIFTY",
            window("2015-01-01", "2020-12-31"),
            "invented",
        ),
        run(
            Scope::Spot,
            Outcome::Failed,
            T0 + 210,
            211,
            KITE,
            Some(("2018-01-01", "2026-09-30")),
            900,
        ),
        run(
            Scope::Spot,
            Outcome::Stored,
            T0 + 4_000,
            500,
            identity,
            Some(("2018-01-01", "2026-09-30")),
            40,
        ),
        run(
            Scope::Fno,
            Outcome::Stored,
            T0 + 9_000,
            50,
            KITE,
            Some(("2015-01-01", "2016-12-31")),
            70,
        ),
        run(
            Scope::Spot,
            Outcome::Empty,
            T0 + 9_500,
            10,
            KITE,
            Some(("2015-01-01", "2015-02-28")),
            0,
        ),
    ] {
        journal.append(&record).unwrap();
    }
}

/// Zerodha NIFTY 1day `month`: one bar a day for `days`, landed in `landings`
/// commits, the file last written at `written`.
fn zerodha_days(root: &Path, month: &str, days: &[&str], landings: usize, written: i64) {
    let path = path_of(
        Vendor::Zerodha,
        "INDEX",
        "NIFTY",
        None,
        Timeframe::DAY_1,
        month,
    );
    let bars: Vec<Bar> = days.iter().map(|on| bar(ist_midnight(on), 5_55)).collect();
    let cut = bars.len() / landings.max(1);
    let batches: Vec<Vec<Bar>> = if landings > 1 {
        vec![bars[..cut].to_vec(), bars[cut..].to_vec()]
    } else {
        vec![bars]
    };
    drop(land(root, path, "NIFTY", &batches));
    std::fs::File::options()
        .write(true)
        .open(path.to_path_buf(root))
        .unwrap()
        .set_modified(at(written))
        .unwrap();
}

const ZERODHA_2015: &str = "feed=zerodha&instrument=NSE-INDEX-NIFTY&timeframe=1day&month=2015-01";

/// **NIFTY FIRST: A ZERODHA MONTH OUTSIDE THE OPERATOR'S WINDOW NAMES THE
/// EARLIER PULL RUN THAT WROTE IT, AND SAYS "OUTSIDE YOUR WINDOW"** (DB-078;
/// D-3334, D-3338). Of the eight records, only the 2015..2020 Kite run is a
/// candidate: the others are refused, unwindowed, another feed's, another
/// form's, offered nothing, a member failure, or miss the month; and since
/// the file committed once and was last written inside that run, it is
/// named the writer.
#[test]
fn a_zerodha_month_outside_the_window_names_its_earlier_pull_run() {
    let root = root_of("zerodha-2015");
    pull_journal(&root);
    zerodha_days(&root, "2015-01", &["2015-01-01", "2015-01-02"], 1, T0 + 7);
    let got = get(&root, ZERODHA_2015);
    assert_eq!(got.status, StatusCode::OK, "{}", got.body);
    let body = &got.body;
    assert_eq!(body["reconstructed"], true);
    assert_eq!(body["label"], RECONSTRUCTED);
    assert_eq!(body["window"]["label"], OUTSIDE_WINDOW);
    assert_eq!(body["window"]["inside"], false);
    assert_eq!(body["window"]["from"], "2018-01-01");
    assert_eq!(body["window"]["standing"], "operator");
    let rebuilt = &body["reconstruction"];
    assert_eq!(rebuilt["source"], "audit/pull.journal");
    let candidates: Vec<u64> = rebuilt["candidates"]
        .as_array()
        .unwrap()
        .iter()
        .map(|run| run["ordinal"].as_u64().unwrap())
        .collect();
    assert_eq!(candidates, [2]);
    let earlier = &rebuilt["candidates"][0];
    assert_eq!(earlier["window"]["from"], "2015-01-01");
    assert_eq!(earlier["window"]["to"], "2020-12-31");
    assert_eq!(earlier["feed"], "zerodha");
    assert_eq!(earlier["scope"], "spot");
    assert_eq!(earlier["bars_offered"], 600);
    assert_eq!(earlier["started_unix"], T0);
    assert_eq!(earlier["running_at_last_write"], true);
    assert_eq!(
        rebuilt["excluded"],
        serde_json::json!({
            "other_feed": 0,
            "names_no_feed": 1,
            "other_scope": 1,
            "window_misses_t": 0,
            "refused_or_not_started": 0,
            "offered_no_bar": 1,
        })
    );
    assert_eq!(rebuilt["attributed"]["run"], 2);
    assert_eq!(rebuilt["last_write"]["inside_runs"], serde_json::json!([2]));
    assert_eq!(rebuilt["bar_file_commits"], 1);
    let journal = &rebuilt["journal"];
    assert_eq!(journal["records"], 8);
    assert_eq!(journal["runs"], 7);
    assert_eq!(journal["runs_without_window"], 1);
    assert_eq!(journal["undecodable"], 0);
}

/// **A MONTH INSIDE THE WINDOW CARRIES NO LABEL, AND ITS SEVERAL CANDIDATES
/// ARE LISTED OLDEST FIRST**; the file was last written inside the earlier
/// run, so that run is named.
#[test]
fn a_month_inside_the_window_lists_every_candidate_oldest_first() {
    let root = root_of("zerodha-2019");
    pull_journal(&root);
    zerodha_days(&root, "2019-03", &["2019-03-01", "2019-03-05"], 1, T0 + 8);
    let got = get(
        &root,
        "feed=zerodha&instrument=NSE-INDEX-NIFTY&timeframe=1day&month=2019-03",
    );
    let body = &got.body;
    assert_eq!(body["window"]["inside"], true, "{body}");
    assert!(body["window"]["label"].is_null());
    let candidates: Vec<u64> = body["reconstruction"]["candidates"]
        .as_array()
        .unwrap()
        .iter()
        .map(|run| run["ordinal"].as_u64().unwrap())
        .collect();
    assert_eq!(candidates, [2, 4, 5]);
    assert_eq!(body["reconstruction"]["attributed"]["run"], 2);
    assert_eq!(
        body["reconstruction"]["candidates"][1]["running_at_last_write"],
        false
    );
    assert!(
        body["reconstruction"]["candidates"][2]["source"]
            .as_str()
            .unwrap()
            .starts_with("zerodha_nse_checked"),
        "{body}"
    );
}

/// **ONE RUN IS NAMED THE WRITER ONLY ON BOTH COUNTS**: a month written
/// outside every candidate's run, or committed twice, names none.
#[test]
fn a_writer_is_named_only_when_one_commit_falls_in_one_run() {
    let root = root_of("zerodha-unnamed");
    pull_journal(&root);
    zerodha_days(
        &root,
        "2015-01",
        &["2015-01-01", "2015-01-02"],
        1,
        T0 + 20_000,
    );
    let outside = get(&root, ZERODHA_2015);
    let rebuilt = &outside.body["reconstruction"];
    assert!(rebuilt["attributed"].is_null(), "{}", outside.body);
    assert_eq!(rebuilt["last_write"]["inside_runs"], serde_json::json!([]));
    assert_eq!(rebuilt["candidates"][0]["running_at_last_write"], false);

    let root = root_of("zerodha-twice");
    pull_journal(&root);
    zerodha_days(&root, "2015-01", &["2015-01-01", "2015-01-02"], 2, T0 + 7);
    let twice = get(&root, ZERODHA_2015);
    let rebuilt = &twice.body["reconstruction"];
    assert_eq!(rebuilt["bar_file_commits"], 2, "{}", twice.body);
    assert_eq!(rebuilt["last_write"]["inside_runs"], serde_json::json!([2]));
    assert!(rebuilt["attributed"].is_null());
}

/// **WITH `t`, A RUN WHOSE WINDOW MISSES `t`'s DAY IS NOT A CANDIDATE**,
/// though it covers part of the month.
#[test]
fn with_t_a_run_whose_window_misses_its_day_is_excluded() {
    let root = root_of("zerodha-t");
    let journal = audit::Journal::at(&root);
    for (start, from, to) in [
        (T0, "2015-01-01", "2015-01-01"),
        (T0 + 100, "2015-01-02", "2015-01-31"),
    ] {
        journal
            .append(&run(
                audit::Scope::Spot,
                audit::Outcome::Stored,
                start,
                10,
                KITE,
                Some((from, to)),
                1,
            ))
            .unwrap();
    }
    zerodha_days(&root, "2015-01", &["2015-01-01", "2015-01-02"], 1, T0 + 5);
    let got = get(
        &root,
        &format!("{ZERODHA_2015}&t={}", ist_midnight("2015-01-02")),
    );
    assert_eq!(got.status, StatusCode::OK, "{}", got.body);
    let rebuilt = &got.body["reconstruction"];
    assert_eq!(rebuilt["candidates"].as_array().unwrap().len(), 1);
    assert_eq!(rebuilt["candidates"][0]["ordinal"], 1);
    assert_eq!(rebuilt["excluded"]["window_misses_t"], 1);
    assert!(
        rebuilt["attributed"].is_null(),
        "last written inside run 0, not run 1"
    );
}

/// **A RECORDED ZERODHA MONTH READS THE PULL RECORD ITS SIDECAR NAMES**: one
/// positional read, and an ordinal past the journal is said.
#[test]
fn a_recorded_zerodha_month_reads_the_pull_record_it_names() {
    let root = root_of("zerodha-recorded");
    pull_journal(&root);
    let path = path_of(
        Vendor::Zerodha,
        "INDEX",
        "NIFTY",
        None,
        Timeframe::DAY_1,
        "2015-01",
    );
    let mut bars = BarFile::open_or_create(&root, path, symbol_id("NIFTY")).unwrap();
    for (on, ordinal) in [("2015-01-01", 2), ("2015-01-02", 99)] {
        let landed = bars.append(&[bar(ist_midnight(on), 5_55)]).unwrap();
        let landing = Landing {
            source: Source {
                kind: SourceKind::VendorEndpoint,
                name: "/instruments/historical/999999/day",
                bytes: 99,
                crc32: 0,
            },
            journal_ordinal: ordinal,
            ..member_landing("unused", on, 0)
        };
        store::provenance::record(&root, &bars, path, landed, &landing).unwrap();
    }
    drop(bars);
    let got = get(&root, ZERODHA_2015);
    let records = got.body["recorded"].as_array().unwrap();
    assert_eq!(records.len(), 2, "{}", got.body);
    assert_eq!(records[0]["source_kind"], "vendor_endpoint");
    assert!(records[0]["source_crc32"].is_null());
    assert_eq!(records[0]["journal"]["record"]["ordinal"], 2);
    assert_eq!(
        records[0]["journal"]["record"]["window"]["from"],
        "2015-01-01"
    );
    assert!(
        records[1]["journal"]["error"]
            .as_str()
            .unwrap()
            .contains("past the journal's 8 records"),
        "{}",
        got.body
    );
    assert_eq!(got.body["window"]["label"], OUTSIDE_WINDOW);
}

/// **THE PULL JOURNAL IS READ THROUGH ITS TAIL**: the first read decodes
/// every record, the next reads nothing, an append reads one, and a replaced
/// file is read again from the start. A torn tail is counted, not read.
#[test]
fn the_pull_journal_is_read_through_its_tail() {
    let root = root_of("pull-tail");
    pull_journal(&root);
    let first = read_runs(&root).unwrap();
    assert_eq!(
        (first.records, first.read_now, first.index.runs.len()),
        (8, 8, 7)
    );
    let again = read_runs(&root).unwrap();
    assert_eq!(again.read_now, 0);
    audit::Journal::at(&root)
        .append(&run(
            audit::Scope::Spot,
            audit::Outcome::Stored,
            T0 + 20_000,
            1,
            KITE,
            Some(("2020-01-01", "2020-01-31")),
            1,
        ))
        .unwrap();
    let tail = read_runs(&root).unwrap();
    assert_eq!(
        (tail.records, tail.read_now, tail.index.runs.len()),
        (9, 1, 8)
    );
    let path = audit::journal_path(&root);
    {
        use std::io::Write as _;
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap();
        file.write_all(b"torn").unwrap();
    }
    let torn = read_runs(&root).unwrap();
    assert_eq!((torn.records, torn.read_now, torn.torn), (9, 0, 4));
    std::fs::remove_file(&path).unwrap();
    audit::Journal::at(&root)
        .append(&run(
            audit::Scope::Spot,
            audit::Outcome::Stored,
            T0,
            1,
            KITE,
            Some(("2020-01-01", "2020-01-31")),
            1,
        ))
        .unwrap();
    let replaced = read_runs(&root).unwrap();
    assert_eq!(
        (
            replaced.records,
            replaced.read_now,
            replaced.index.runs.len()
        ),
        (1, 1, 1)
    );
    let absent = read_runs(&root_of("pull-absent")).unwrap();
    assert!(!absent.present);
}

/// **A RUN TOO WIDE TO FILE BY MONTH IS STILL A CANDIDATE**, walked on every
/// request and counted; a record with a kind byte this reader does not know
/// is counted undecodable, never read as a run.
#[test]
fn a_run_too_wide_to_file_is_still_considered_and_counted() {
    let root = root_of("pull-wide");
    let journal = audit::Journal::at(&root);
    let mut wide = run(
        audit::Scope::Spot,
        audit::Outcome::Stored,
        T0,
        10,
        KITE,
        Some(("2015-01-01", "2015-01-31")),
        5,
    );
    wide.to_days = day("2125-12-31").days_from_epoch();
    journal.append(&wide).unwrap();
    let mut image = run(
        audit::Scope::Spot,
        audit::Outcome::Stored,
        T0,
        10,
        KITE,
        Some(("2015-01-01", "2015-01-31")),
        5,
    )
    .image();
    image[KIND_AT] = 9;
    {
        use std::io::Write as _;
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(audit::journal_path(&root))
            .unwrap();
        file.write_all(&image).unwrap();
    }
    zerodha_days(&root, "2015-01", &["2015-01-01"], 1, T0 + 1);
    let got = get(&root, ZERODHA_2015);
    let rebuilt = &got.body["reconstruction"];
    assert_eq!(
        rebuilt["journal"]["runs_walked_on_every_request"], 1,
        "{}",
        got.body
    );
    assert_eq!(rebuilt["journal"]["undecodable"], 1);
    assert_eq!(rebuilt["candidates"][0]["ordinal"], 0);
    assert_eq!(rebuilt["attributed"]["run"], 0);
}

/// The kind byte this reader looks at first is the byte the writer puts the
/// kind in.
#[test]
fn the_kind_byte_is_where_the_writer_puts_it() {
    let member = audit::Record::member_failure(
        audit::Scope::Spot,
        at(T0),
        "X",
        window("2015-01-01", "2015-01-31"),
        "y",
    );
    assert_eq!(member.image()[KIND_AT], audit::Kind::MemberFailure.code());
    let whole = run(
        audit::Scope::Spot,
        audit::Outcome::Stored,
        T0,
        1,
        KITE,
        None,
        1,
    );
    assert_eq!(whole.image()[KIND_AT], audit::Kind::Run.code());
    assert_ne!(audit::Kind::Run.code(), audit::Kind::MemberFailure.code());
}

/// Two ascending position lists come out ascending, each position once.
#[test]
fn ascending_merges_two_ordered_lists() {
    let got: Vec<usize> = ascending(&[1, 4, 9], &[0, 5, 6, 12]).collect();
    assert_eq!(got, [0, 1, 4, 5, 6, 9, 12]);
    assert_eq!(ascending(&[], &[3]).collect::<Vec<_>>(), [3]);
    assert_eq!(ascending(&[2], &[]).collect::<Vec<_>>(), [2]);
    assert!(ascending(&[], &[]).next().is_none());
}

/// A journal filter takes a symbol only by its exact name.
#[test]
fn a_filter_takes_a_symbol_by_its_exact_name() {
    assert!(filter_holds("*", "NIFTY"));
    assert!(filter_holds("BANKNIFTY,NIFTY", "NIFTY"));
    assert!(!filter_holds("NIFTYIT", "NIFTY"));
    assert!(!filter_holds("BANKNIFTY", "NIFTY"));
}

/// The unrecorded ranges: before the first record, between two, after the
/// last; none for a month every record covers; the whole month for none.
#[test]
fn unrecorded_ranges_are_the_gaps_around_the_records() {
    let root = root_of("unrecorded");
    let path = path_of(
        Vendor::Gdfl,
        "INDEX",
        "NIFTY",
        None,
        Timeframe::SECOND_1,
        "2019-03",
    );
    let mut bars = BarFile::open_or_create(&root, path, symbol_id("NIFTY")).unwrap();
    let _ = bars.append(&seconds("2019-03-01", 2)).unwrap();
    for on in ["2019-03-05", "2019-03-06"] {
        let landed = bars.append(&seconds(on, 2)).unwrap();
        store::provenance::record(
            &root,
            &bars,
            path,
            landed,
            &member_landing("m.zip!a.csv", on, 1),
        )
        .unwrap();
    }
    let _ = bars.append(&seconds("2019-03-07", 3)).unwrap();
    drop(bars);
    let Found::Present(sidecar) = store::provenance::read(&root, path).unwrap() else {
        panic!("a sidecar");
    };
    assert_eq!(unrecorded(sidecar.records(), 9), [(0, 2), (6, 9)]);
    assert_eq!(unrecorded(sidecar.records(), 6), [(0, 2)]);
    assert_eq!(unrecorded(&[], 4), [(0, 4)]);
    assert!(unrecorded(&[], 0).is_empty());
}

/// **THE ROUTE IS REGISTERED AND BUILT**, over the real route table: a `200`
/// JSON answer for a held month, a `400` naming the parameter it could not
/// read, and a `404` for a month the store does not hold.
#[tokio::test]
async fn the_route_answers_over_the_real_route_table() {
    use crate::server::tests::{agreeing, body_of, front, get as fetch, site};
    let dir = agreeing("provenance-route");
    let mut built = site("provenance-route", &dir);
    let root = root_of("route");
    let _month = nifty_march(&root);
    built.store_root = root;
    let loaded: Loaded = std::sync::Arc::new(built);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
    let served = tokio::spawn(
        axum::serve(
            listener,
            super::super::route_table(front("provenance-route")).with_state(loaded),
        )
        .with_graceful_shutdown(async move {
            let _closed = stopped.await;
        })
        .into_future(),
    );
    let ok = fetch(addr, &format!("/provenance.json?{GDFL_NIFTY}")).await;
    assert!(ok.starts_with("HTTP/1.1 200 "), "{ok}");
    assert!(
        ok.to_ascii_lowercase()
            .contains("content-type: application/json"),
        "{ok}"
    );
    let body: Value = serde_json::from_str(body_of(&ok)).unwrap();
    assert_eq!(body["label"], RECONSTRUCTED);
    assert_eq!(body["instrument"], "NSE-INDEX-NIFTY");
    let bad = fetch(
        addr,
        "/provenance.json?feed=gdfl&instrument=NSE-INDEX-NIFTY&month=2019-03",
    )
    .await;
    assert!(bad.starts_with("HTTP/1.1 400 "), "{bad}");
    assert!(body_of(&bad).contains("timeframe="), "{bad}");
    let absent = fetch(
        addr,
        "/provenance.json?feed=gdfl&instrument=NSE-INDEX-BANKNIFTY&timeframe=1s&month=2019-03",
    )
    .await;
    assert!(absent.starts_with("HTTP/1.1 404 "), "{absent}");
    let _gone = stop.send(());
    served.await.unwrap().unwrap();
}

/// **A MONTH BEING WRITTEN IS THE BEING-WRITTEN 503 THAT SAYS SO**: a landing
/// holds the month's lock while it appends and the store refuses a reader
/// meanwhile, which is neither an absent month nor a damaged one. It was a 409
/// with no `being_written`, which `count_route` gives no `Retry-After` and the
/// page never asks again on (R3-08 repair 2, NEW-21); it is now the 503 every
/// store read answers, its body small enough for `count_route` to read.
#[test]
fn a_month_being_written_is_the_being_written_503_naming_the_writer() {
    let root = root_of("locked");
    gdfl_journal(&root, NIFTY_MARCH);
    let path = path_of(
        Vendor::Gdfl,
        "INDEX",
        "NIFTY",
        None,
        Timeframe::SECOND_1,
        "2019-03",
    );
    let mut writer = BarFile::open_or_create(&root, path, symbol_id("NIFTY")).unwrap();
    let _landed = writer.append(&seconds("2019-03-01", 2)).unwrap();
    let held = get(&root, GDFL_NIFTY);
    assert_eq!(
        held.status,
        StatusCode::SERVICE_UNAVAILABLE,
        "{}",
        held.body
    );
    assert_eq!(held.body["being_written"], true, "{}", held.body);
    assert!(
        held.body["error"]
            .as_str()
            .unwrap()
            .contains(store::flock::BEING_WRITTEN),
        "{}",
        held.body
    );
    let wire = held.body.to_string();
    assert!(wire.contains(crate::detail::BEING_WRITTEN_FIELD), "{wire}");
    assert!(
        wire.len() <= crate::detail::BEING_WRITTEN_BODY_MAX,
        "{}",
        wire.len()
    );
    assert!(
        held.body["error"]
            .as_str()
            .unwrap()
            .contains("being written right now"),
        "{}",
        held.body
    );
    drop(writer);
    assert_eq!(get(&root, GDFL_NIFTY).status, StatusCode::OK);
}

/// The symbol id this route opens a month with is the fold `pull::ingest`
/// stamps in: the low half of the 64-bit FNV-1a, which ingest writes as a
/// truncating cast.
#[test]
fn the_symbol_id_is_the_fold_ingest_stamps() {
    for symbol in ["NIFTY", "BANKNIFTY", "RELIANCE", "BAJAJ-AUTO", "M&M"] {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "the truncating cast is what pull::ingest writes, and is what is compared"
        )]
        let ingest = brutex_core::universe::fnv1a(symbol) as u32;
        assert_eq!(super::symbol_id(symbol), ingest, "{symbol}");
    }
}

/// **ANOTHER FEED'S RUN IS NEVER A CANDIDATE**, though its window covers the
/// month: a Zerodha month whose only covering run is Dhan's has no
/// candidate and names no writer, and says so.
#[test]
fn a_run_of_another_feed_is_never_a_candidate() {
    let root = root_of("other-feed");
    audit::Journal::at(&root)
        .append(&run(
            audit::Scope::Spot,
            audit::Outcome::Stored,
            T0,
            10,
            "https://api.dhan.co/v2/charts/historical",
            Some(("2015-01-01", "2015-01-31")),
            4,
        ))
        .unwrap();
    zerodha_days(&root, "2015-01", &["2015-01-01"], 1, T0 + 1);
    let got = get(&root, ZERODHA_2015);
    let rebuilt = &got.body["reconstruction"];
    assert_eq!(rebuilt["candidates"], serde_json::json!([]), "{}", got.body);
    assert_eq!(rebuilt["excluded"]["other_feed"], 1);
    assert!(rebuilt["attributed"].is_null());
    assert_eq!(got.body["label"], RECONSTRUCTED);
}

/// **A BAR ON A DAY THE JOURNAL DOES NOT NAME IS COUNTED, NEVER HIDDEN**,
/// and a journal day on which this instrument has no bar says `count: 0`:
/// the commits then disagree with the days holding bars, by name.
#[test]
fn bars_on_a_day_the_journal_does_not_name_are_counted() {
    let root = root_of("gdfl-unjournalled");
    gdfl_journal(
        &root,
        "begin indices 2019-03-01 *\n\
         done indices 2019-03-01 * files=2 seconds=3 failures=0 late=0 late_unresolved=0 max_back_s=0\n\
         begin indices 2019-03-04 *\n\
         done indices 2019-03-04 * files=2 seconds=3 failures=0 late=0 late_unresolved=0 max_back_s=0\n",
    );
    let path = path_of(
        Vendor::Gdfl,
        "INDEX",
        "NIFTY",
        None,
        Timeframe::SECOND_1,
        "2019-03",
    );
    let _landed = land(
        &root,
        path,
        "NIFTY",
        &[seconds("2019-03-01", 3), seconds("2019-03-07", 4)],
    );
    let got = get(&root, GDFL_NIFTY);
    let rebuilt = &got.body["reconstruction"];
    assert_eq!(rebuilt["days"][1]["day"], "2019-03-04", "{}", got.body);
    assert_eq!(rebuilt["days"][1]["bars"]["count"], 0);
    assert_eq!(rebuilt["journal_days_with_bars"], 1);
    assert_eq!(rebuilt["bars_on_journal_days"], 3);
    assert_eq!(rebuilt["bars_on_no_journal_day"], 4);
    assert_eq!(rebuilt["bar_file_commits"], 2);
    assert_eq!(rebuilt["commits_agree"], false);
}

/// **A DAY'S START IS LOOKED UP WHEN BARS LIE BETWEEN IT AND THE PREVIOUS
/// JOURNAL DAY**: the previous day's end is taken for this day's start only
/// when no bar lies between, so the bars of an unjournalled day in between
/// are counted apart and never given to either neighbour.
#[test]
fn a_journal_day_after_unjournalled_bars_starts_at_its_own_first_bar() {
    let root = root_of("gdfl-between");
    gdfl_journal(
        &root,
        "begin indices 2019-03-01 *\n\
         done indices 2019-03-01 * files=2 seconds=3 failures=0 late=0 late_unresolved=0 max_back_s=0\n\
         begin indices 2019-03-07 *\n\
         done indices 2019-03-07 * files=2 seconds=4 failures=0 late=0 late_unresolved=0 max_back_s=0\n",
    );
    let path = path_of(
        Vendor::Gdfl,
        "INDEX",
        "NIFTY",
        None,
        Timeframe::SECOND_1,
        "2019-03",
    );
    let _landed = land(
        &root,
        path,
        "NIFTY",
        &[
            seconds("2019-03-01", 3),
            seconds("2019-03-05", 2),
            seconds("2019-03-07", 4),
        ],
    );
    let got = get(&root, GDFL_NIFTY);
    let days = &got.body["reconstruction"]["days"];
    assert_eq!(
        days[0]["bars"],
        serde_json::json!({ "first_index": 0, "end": 3, "count": 3 }),
        "{}",
        got.body
    );
    assert_eq!(
        days[1]["bars"],
        serde_json::json!({ "first_index": 5, "end": 9, "count": 4 })
    );
    assert_eq!(got.body["reconstruction"]["bars_on_no_journal_day"], 2);
    assert_eq!(got.body["reconstruction"]["commits_agree"], false);
}

/// **A RUN'S OPENING RECORD IS PAIRED WITH ITS CLOSE, AND ONE NEVER CLOSED
/// IS LISTED AND KEEPS A WRITER FROM BEING NAMED** (D-3341): an opened run
/// whose process stopped may have landed bars and said nothing of them, so
/// once it began before the file's last write no run is named; one opened
/// after that write changes nothing.
#[test]
fn an_opened_run_is_paired_with_its_close_and_an_unclosed_one_is_listed() {
    use audit::{Outcome, Scope};
    let window = || window("2015-01-01", "2020-12-31");
    let span = Some(("2015-01-01", "2020-12-31"));
    // 0: opened; 1: its close, which was under way when 0 was opened.
    let paired = |root: &Path| {
        let journal = audit::Journal::at(root);
        journal
            .append(&audit::Record::opened(
                Scope::Spot,
                at(T0 + 1),
                KITE,
                window(),
                3,
                0,
            ))
            .unwrap();
        journal
            .append(&run(Scope::Spot, Outcome::Failed, T0, 1_000, KITE, span, 9))
            .unwrap();
    };

    let root = root_of("opened-closed");
    paired(&root);
    zerodha_days(&root, "2015-01", &["2015-01-01"], 1, T0 + 700);
    let got = get(&root, ZERODHA_2015);
    let rebuilt = &got.body["reconstruction"];
    assert_eq!(rebuilt["candidates"][0]["ordinal"], 1, "{}", got.body);
    assert_eq!(rebuilt["candidates"].as_array().unwrap().len(), 1);
    assert_eq!(rebuilt["opened_and_closed"], 1);
    assert_eq!(rebuilt["opened_never_closed"], serde_json::json!([]));
    assert_eq!(rebuilt["attributed"]["run"], 1);

    // 2: a second run opened at T0 + 500 and never closed.
    let root = root_of("opened-unclosed");
    paired(&root);
    audit::Journal::at(&root)
        .append(&audit::Record::opened(
            Scope::Spot,
            at(T0 + 500),
            KITE,
            window(),
            3,
            0,
        ))
        .unwrap();
    zerodha_days(&root, "2015-01", &["2015-01-01"], 1, T0 + 700);
    let got = get(&root, ZERODHA_2015);
    let rebuilt = &got.body["reconstruction"];
    let unclosed = rebuilt["opened_never_closed"].as_array().unwrap();
    assert_eq!(unclosed.len(), 1, "{}", got.body);
    assert_eq!(unclosed[0]["ordinal"], 2);
    assert_eq!(unclosed[0]["outcome"], "OPENED");
    assert_eq!(rebuilt["last_write"]["inside_runs"], serde_json::json!([1]));
    assert!(
        rebuilt["attributed"].is_null(),
        "an unclosed run opened before the last write may be the writer"
    );

    // The same run opened after the file's last write blocks nothing.
    let root = root_of("opened-after");
    paired(&root);
    audit::Journal::at(&root)
        .append(&audit::Record::opened(
            Scope::Spot,
            at(T0 + 900),
            KITE,
            window(),
            3,
            0,
        ))
        .unwrap();
    zerodha_days(&root, "2015-01", &["2015-01-01"], 1, T0 + 700);
    let got = get(&root, ZERODHA_2015);
    let rebuilt = &got.body["reconstruction"];
    assert_eq!(rebuilt["opened_never_closed"].as_array().unwrap().len(), 1);
    assert_eq!(rebuilt["attributed"]["run"], 1, "{}", got.body);
}

/// **A RUN'S OPENING RECORD WRITTEN AGAIN AFTER ANOTHER WRITER IS THE SAME
/// RUN**, never an unclosed second one: the journal's own interleaving shape,
/// written by `audit::Journal::append_failure`, pairs with the run's one close.
#[test]
fn an_opening_record_written_again_is_the_same_run() {
    use audit::{Outcome, Scope};
    let root = root_of("opened-continues");
    let journal = audit::Journal::at(&root);
    let head = audit::Record::opened(
        Scope::Spot,
        at(T0 + 1),
        KITE,
        window("2015-01-01", "2020-12-31"),
        3,
        0,
    );
    let mut thread = journal.open_run(&head).unwrap();
    // Another writer appends between this run's records.
    journal
        .append(&run(
            Scope::Fno,
            Outcome::Refused,
            T0 + 2,
            0,
            "other",
            None,
            0,
        ))
        .unwrap();
    journal
        .append_failure(
            &mut thread,
            &audit::Record::member_failure(
                Scope::Spot,
                at(T0 + 3),
                "NIFTY",
                window("2015-01-01", "2015-01-31"),
                "invented",
            ),
            "invented",
        )
        .unwrap();
    journal
        .append(&run(
            Scope::Spot,
            Outcome::Failed,
            T0,
            1_000,
            KITE,
            Some(("2015-01-01", "2020-12-31")),
            9,
        ))
        .unwrap();
    zerodha_days(&root, "2015-01", &["2015-01-01"], 1, T0 + 700);
    let got = get(&root, ZERODHA_2015);
    let rebuilt = &got.body["reconstruction"];
    assert_eq!(
        rebuilt["opened_again_after_another_writer"], 1,
        "{}",
        got.body
    );
    assert_eq!(rebuilt["opened_and_closed"], 1);
    assert_eq!(rebuilt["opened_never_closed"], serde_json::json!([]));
    assert_eq!(rebuilt["attributed"]["run"], 4);
}

// ══ A month served from a repair revision (D-3350; DB-N26, WP-12d) ══════════

/// The repair door's policy for an index: open interest is no measurement,
/// and the feeds here are not back-adjusted.
const INDEX_REPAIR: store::repair::Policy = store::repair::Policy {
    open_interest_applies: false,
    basis_witness: false,
};

/// **A BACK-FILLED GDFL MONTH IS ANSWERED FROM THE GENERATION EVERY READER
/// OPENS, AND SAYS SO.** NIFTY 1s 2019-03 holds the 1st and the 5th in two
/// commits; the import then lands the 4th, a day inside what it holds, as
/// repair revision 1. The answer names the revision's path and its sidecar
/// path, the original's path beside it, the revision's header, and ties the
/// 4th's bars to its journal line between the others; the revision's one
/// commit is not compared with the import days, and the reason is given in
/// its place and at the top. The original's bytes are untouched.
#[test]
fn a_repaired_gdfl_month_is_answered_from_its_served_revision() {
    let root = root_of("gdfl-repaired");
    let _month = nifty_march(&root);
    let path = path_of(
        Vendor::Gdfl,
        "INDEX",
        "NIFTY",
        None,
        Timeframe::SECOND_1,
        "2019-03",
    );
    let original = std::fs::read(path.to_path_buf(&root)).unwrap();
    let repaired = store::repair::repair_month(
        &root,
        path,
        symbol_id("NIFTY"),
        &seconds("2019-03-04", 4),
        INDEX_REPAIR,
    )
    .unwrap();
    assert!(
        matches!(repaired, store::repair::MonthRepair::Repaired { .. }),
        "{repaired:?}"
    );
    gdfl_journal(
        &root,
        &format!(
            "{NIFTY_MARCH}begin indices 2019-03-04 NIFTY\n\
             done indices 2019-03-04 NIFTY definition=3 files=1 seconds=4 failures=0 late=0 \
             late_unresolved=0 max_back_s=0\n"
        ),
    );

    let got = get(&root, GDFL_NIFTY);

    assert_eq!(got.status, StatusCode::OK, "{}", got.body);
    let body = &got.body;
    let bars = &body["bars"];
    assert_eq!(
        bars["path"],
        "bar-revisions-v1/1/bars/gdfl/NSE/INDEX/NIFTY/1s/2019-03.bin"
    );
    assert_eq!(bars["served_generation"], 1);
    assert_eq!(
        bars["original_path"],
        "bars/gdfl/NSE/INDEX/NIFTY/1s/2019-03.bin"
    );
    assert_eq!(bars["n_valid"], 9);
    assert_eq!(bars["generation"], 1, "the revision's one commit");
    assert_eq!(
        body["sidecar"]["path"],
        "bar-revisions-v1/1/bars/gdfl/NSE/INDEX/NIFTY/1s/2019-03.prv"
    );
    assert_eq!(body["sidecar"]["state"], "absent");
    assert_eq!(body["label"], RECONSTRUCTED);
    let note = body["notes"][0].as_str().unwrap();
    assert!(note.contains("repair revision 1 (D-3350)"), "{note}");
    assert!(note.contains("audit/gdfl.repairs"), "{note}");
    let rebuilt = &body["reconstruction"];
    assert!(rebuilt["commits_agree"].is_null(), "{rebuilt}");
    assert!(
        rebuilt["commits_why"]
            .as_str()
            .unwrap()
            .contains("repair revision"),
        "{rebuilt}"
    );
    let days: Vec<(&str, Value)> = rebuilt["days"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| (row["day"].as_str().unwrap(), row["bars"].clone()))
        .collect();
    assert_eq!(
        days,
        [
            (
                "2019-03-01",
                serde_json::json!({ "first_index": 0, "end": 3, "count": 3 })
            ),
            (
                "2019-03-04",
                serde_json::json!({ "first_index": 3, "end": 7, "count": 4 })
            ),
            (
                "2019-03-05",
                serde_json::json!({ "first_index": 7, "end": 9, "count": 2 })
            ),
        ]
    );
    assert_eq!(rebuilt["bars_on_journal_days"], 9);
    assert_eq!(rebuilt["bars_on_no_journal_day"], 0);
    assert_eq!(std::fs::read(path.to_path_buf(&root)).unwrap(), original);
}

/// **NO PULL RUN IS NAMED THE WRITER OF A MONTH SERVED FROM A REPAIR
/// REVISION.** Zerodha NIFTY 1day 2015-01 committed once inside the
/// 2015..2020 Kite run, which is named its writer; a repair then lands the
/// 2nd as revision 1, also written inside that run's span. A revision's one
/// commit and modification time are the repair's, so the rule "committed
/// once, last written inside one run" no longer names anyone, and the last
/// write reported is the revision's.
#[test]
fn a_month_served_from_a_repair_revision_names_no_pull_run_its_writer() {
    let root = root_of("zerodha-repaired");
    pull_journal(&root);
    zerodha_days(&root, "2015-01", &["2015-01-01", "2015-01-05"], 1, T0 + 7);
    let before = get(&root, ZERODHA_2015);
    assert_eq!(
        before.body["reconstruction"]["attributed"]["run"], 2,
        "{}",
        before.body
    );
    let path = path_of(
        Vendor::Zerodha,
        "INDEX",
        "NIFTY",
        None,
        Timeframe::DAY_1,
        "2015-01",
    );
    let repaired = store::repair::repair_month(
        &root,
        path,
        symbol_id("NIFTY"),
        &[bar(ist_midnight("2015-01-02"), 5_55)],
        INDEX_REPAIR,
    )
    .unwrap();
    assert!(
        matches!(repaired, store::repair::MonthRepair::Repaired { .. }),
        "{repaired:?}"
    );
    std::fs::File::options()
        .write(true)
        .open(path.to_path_buf(&root.join("bar-revisions-v1").join("1")))
        .unwrap()
        .set_modified(at(T0 + 9))
        .unwrap();

    let got = get(&root, ZERODHA_2015);

    assert_eq!(got.status, StatusCode::OK, "{}", got.body);
    let body = &got.body;
    assert_eq!(body["bars"]["served_generation"], 1);
    assert_eq!(body["bars"]["n_valid"], 3);
    assert_eq!(body["bars"]["modified_unix"], T0 + 9);
    let rebuilt = &body["reconstruction"];
    assert_eq!(rebuilt["last_write"]["modified_unix"], T0 + 9);
    assert_eq!(rebuilt["last_write"]["inside_runs"], serde_json::json!([2]));
    assert!(rebuilt["attributed"].is_null(), "{rebuilt}");
    assert!(
        body["notes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|note| note.as_str().unwrap().contains("audit/zerodha.repairs")),
        "{body}"
    );
}

/// **EVERY STORE STATE BETWEEN "NOT HELD" AND "SERVED" IS SAID BY WHAT IT IS**
/// (BASE-COV-API-AS2). Extremes: (1) a month path whose feed folder is a FILE
/// is the filesystem's refusal, a 500 naming the path, never "not held";
/// (2) a month file whose header is not a header is a 500 naming the path;
/// (3) a month file that holds no committed bar is served with the label
/// that says there is nothing to attribute; (4) its sidecar, absent, is said
/// absent; (5) a sidecar of no bytes is UNBORN: its header never reached the
/// disk, so it holds no record; (6) a sidecar path that is a FOLDER is
/// REFUSED by the sidecar reader and named, never read as records; (7) every refusal echoes the
/// request; (8) the unborn sidecar's answer is still a 200 over the bars.
/// Combined: (9) a month that holds no bar AND an unborn sidecar beside it:
/// both said at once; (10) a garbage month header AND a request for one bar
/// by `t`: refused before any bar is looked for; (12) a recorded sidecar
/// AND a month written again shorter beside it: the sidecar is refused for
/// the bars it names that the month no longer holds. Out of the box: (11) a
/// FOLDER at the month's own path is refused, never opened as a month.
#[test]
#[allow(
    clippy::too_many_lines,
    reason = "one test walks every store state its Extremes: line names, in order"
)]
fn every_store_state_between_not_held_and_served_is_said_by_what_it_is() {
    let path = path_of(
        Vendor::Gdfl,
        "INDEX",
        "NIFTY",
        None,
        Timeframe::SECOND_1,
        "2019-03",
    );
    let relative = "bars/gdfl/NSE/INDEX/NIFTY/1s/2019-03.bin";
    // (1): the feed's folder is a file.
    let root = root_of("as2-feed-file");
    std::fs::create_dir_all(root.join("bars")).unwrap();
    std::fs::write(root.join("bars/gdfl"), b"not a folder").unwrap();
    let got = get(&root, GDFL_NIFTY);
    assert_eq!(
        got.status,
        StatusCode::INTERNAL_SERVER_ERROR,
        "{}",
        got.body
    );
    assert_eq!(got.body["path"], relative);
    assert!(
        got.body["error"].as_str().unwrap().starts_with(relative),
        "{}",
        got.body
    );
    assert_eq!(
        got.body["feed"], "gdfl",
        "the request is echoed: {}",
        got.body
    );
    // (2), (10): a month file whose header is not one.
    let root = root_of("as2-garbage-month");
    let month = path.to_path_buf(&root);
    std::fs::create_dir_all(month.parent().unwrap()).unwrap();
    std::fs::write(&month, vec![0xA5_u8; 4_096]).unwrap();
    for query in [
        GDFL_NIFTY.to_owned(),
        format!(
            "{GDFL_NIFTY}&t={}",
            ist_midnight("2019-03-01") + 9 * 3_600 + 15 * 60
        ),
    ] {
        let got = get(&root, &query);
        assert_eq!(
            got.status,
            StatusCode::INTERNAL_SERVER_ERROR,
            "{}",
            got.body
        );
        assert_eq!(got.body["path"], relative);
        assert!(got.body.get("bar").is_none(), "{}", got.body);
    }
    // (3), (4): a month that holds no committed bar.
    let root = root_of("as2-empty-month");
    drop(BarFile::open_or_create(&root, path, symbol_id("NIFTY")).unwrap());
    let got = get(&root, GDFL_NIFTY);
    assert_eq!(got.status, StatusCode::OK, "{}", got.body);
    assert_eq!(
        got.body["label"],
        "the month file holds no committed bar, so there is nothing to attribute"
    );
    assert_eq!(got.body["sidecar"]["state"], "absent");
    // (5), (8), (9): an unborn sidecar beside it.
    let prv = path.with_file(FileKind::Provenance).to_path_buf(&root);
    std::fs::write(&prv, b"").unwrap();
    let got = get(&root, GDFL_NIFTY);
    assert_eq!(got.status, StatusCode::OK, "{}", got.body);
    assert_eq!(got.body["sidecar"]["state"], "unborn", "{}", got.body);
    assert_eq!(got.body["sidecar"]["records"], 0);
    assert_eq!(got.body["sidecar"]["bytes"], 0);
    assert!(
        got.body["label"]
            .as_str()
            .unwrap()
            .contains("no committed bar")
    );
    // (6): a folder where the sidecar goes.
    let root = root_of("as2-damaged-sidecar-header");
    land(&root, path, "NIFTY", &[seconds("2019-03-01", 2)]);
    let prv = path.with_file(FileKind::Provenance).to_path_buf(&root);
    std::fs::create_dir_all(&prv).unwrap();
    let got = get(&root, GDFL_NIFTY);
    assert_eq!(got.status, StatusCode::OK, "{}", got.body);
    assert_eq!(got.body["sidecar"]["state"], "refused", "{}", got.body);
    assert!(!got.body["sidecar"]["error"].as_str().unwrap().is_empty());
    assert_eq!(got.body["recorded"], serde_json::json!([]));
    // (12): a sidecar whose records outrun the month beside it: the month
    // was written again shorter, its old sidecar kept.
    let root = root_of("as2-sidecar-outruns");
    let mut bars = BarFile::open_or_create(&root, path, symbol_id("NIFTY")).unwrap();
    for (batch, on) in [
        (seconds("2019-03-01", 3), "2019-03-01"),
        (seconds("2019-03-05", 2), "2019-03-05"),
    ] {
        let landed = bars.append(&batch).unwrap();
        store::provenance::record(
            &root,
            &bars,
            path,
            landed,
            &member_landing("m.zip!a.csv", on, 2),
        )
        .unwrap();
    }
    drop(bars);
    std::fs::remove_file(path.to_path_buf(&root)).unwrap();
    land(&root, path, "NIFTY", &[seconds("2019-03-01", 1)]);
    let got = get(&root, GDFL_NIFTY);
    assert_eq!(got.status, StatusCode::OK, "{}", got.body);
    assert_eq!(got.body["sidecar"]["state"], "refused", "{}", got.body);
    assert_eq!(got.body["recorded"], serde_json::json!([]));
    // (11): a folder at the month's own path.
    let root = root_of("as2-month-folder");
    std::fs::create_dir_all(path.to_path_buf(&root)).unwrap();
    let got = get(&root, GDFL_NIFTY);
    assert_eq!(
        got.status,
        StatusCode::INTERNAL_SERVER_ERROR,
        "{}",
        got.body
    );
    assert_eq!(got.body["path"], relative);
    assert!(got.body.get("bars").is_none(), "never served: {}", got.body);
}

/// **EVERY REFUSAL BEFORE THE MONTH IS SERVED NAMES WHAT REFUSED**
/// (BASE-COV-API-AS2): a symbol no store path can hold is the path's own
/// refusal; a promoted revision whose bar file is gone is "not held", naming
/// the file looked for; and a generation pointer damaged after the month opened
/// is the pointer's refusal, from the step that resolves the served generation.
/// Extremes: 12 (8 single, 2 combined, 2 out of the box): (1) a symbol in lower
/// case is a 400 naming its first wrong-case byte; (2) a symbol of 25 bytes is
/// a 400 naming its length and the 24 a segment holds; (3) each of those echoes
/// the request and carries no `path` and no `held`, since no file was looked
/// for; (4) a promoted revision whose `.bin` is gone is a 404 with `held`
/// false; (5) that 404 names the revision's missing file in `error` and the
/// original's path under `path`; (6) a pointer damaged after the month opened
/// is a 500 naming the pointer and that none of its records verifies; (7) that
/// 500 carries the original's path and no `bars`; (8) the same pointer met by a
/// fresh request is refused at the open, a 500 in the same words. Combined:
/// (9) a symbol in lower case AND too long is refused for its length, the check
/// that runs first; (10) a revision that is gone AND a request for one bar by
/// `t`: the same 404, and no bar is looked for. Out of the box: (11) `..` as a
/// symbol is refused as a traversal, never resolved; (12) the vendor's own file
/// stem `NIFTY 50` as a symbol is refused for its space byte.
#[test]
#[allow(
    clippy::too_many_lines,
    reason = "one test walks every refusal its Extremes: line names, in order"
)]
fn every_refusal_before_the_month_is_served_names_what_refused() {
    // (1), (2), (3), (9), (11), (12): symbols no store path can hold.
    let root = root_of("as2-g2-unpathable");
    for (instrument, key, said) in [
        (
            "NSE-INDEX-nifty",
            "NSE-INDEX-nifty",
            "path segment symbol holds byte 0x6e in the wrong case",
        ),
        (
            "NSE-INDEX-ABCDEFGHIJKLMNOPQRSTUVWXY",
            "NSE-INDEX-ABCDEFGHIJKLMNOPQRSTUVWXY",
            "path segment symbol is 25 bytes, max 24",
        ),
        (
            "NSE-INDEX-abcdefghijklmnopqrstuvwxy",
            "NSE-INDEX-abcdefghijklmnopqrstuvwxy",
            "path segment symbol is 25 bytes, max 24",
        ),
        (
            "NSE-INDEX-..",
            "NSE-INDEX-..",
            "path segment symbol is a traversal and would escape",
        ),
        (
            "NSE-INDEX-NIFTY%2050",
            "NSE-INDEX-NIFTY 50",
            "path segment symbol holds byte 0x20",
        ),
    ] {
        let got = get(
            &root,
            &format!("feed=gdfl&instrument={instrument}&timeframe=1s&month=2019-03"),
        );
        assert_eq!(got.status, StatusCode::BAD_REQUEST, "{}", got.body);
        assert_eq!(got.body["error"], format!("{key} 2019-03: {said}"));
        assert_eq!(got.body["instrument"], key);
        assert_eq!(got.body["feed"], "gdfl");
        assert!(
            got.body.get("path").is_none() && got.body.get("held").is_none(),
            "no file was looked for: {}",
            got.body
        );
    }
    // (4), (5), (10): a promoted revision whose bar file is gone.
    let relative = "bars/gdfl/NSE/INDEX/NIFTY/1s/2019-03.bin";
    let path = path_of(
        Vendor::Gdfl,
        "INDEX",
        "NIFTY",
        None,
        Timeframe::SECOND_1,
        "2019-03",
    );
    let root = root_of("as2-g2-revision-gone");
    let _month = nifty_march(&root);
    let repaired = store::repair::repair_month(
        &root,
        path,
        symbol_id("NIFTY"),
        &seconds("2019-03-04", 4),
        INDEX_REPAIR,
    )
    .unwrap();
    assert!(
        matches!(repaired, store::repair::MonthRepair::Repaired { .. }),
        "{repaired:?}"
    );
    let revision = path.to_path_buf(&root.join("bar-revisions-v1").join("1"));
    std::fs::remove_file(&revision).unwrap();
    let fifth = ist_midnight("2019-03-05") + 9 * 3_600 + 15 * 60;
    for query in [GDFL_NIFTY.to_owned(), format!("{GDFL_NIFTY}&t={fifth}")] {
        let got = get(&root, &query);
        assert_eq!(got.status, StatusCode::NOT_FOUND, "{}", got.body);
        assert_eq!(got.body["held"], false);
        assert_eq!(got.body["path"], relative);
        assert_eq!(
            got.body["error"],
            format!(
                "{} does not exist, opening it: this month is not held",
                revision.display()
            )
        );
        assert!(got.body.get("bar").is_none(), "{}", got.body);
    }
    // (6), (7), (8): a generation pointer damaged after the month opened.
    let root = root_of("as2-g2-pointer-after-open");
    land(&root, path, "NIFTY", &[seconds("2019-03-01", 2)]);
    let bars = BarFile::open_existing(&root, path, symbol_id("NIFTY")).unwrap();
    let meta = std::fs::metadata(path.to_path_buf(&root)).unwrap();
    let pointer = store::generation::pointer_path(&root, path);
    std::fs::create_dir_all(pointer.parent().unwrap()).unwrap();
    std::fs::write(&pointer, [0_u8; 64]).unwrap();
    let refused = format!(
        "{}: the month's generation pointer refused: none of its 1 record(s) verifies, so \
         which generation the month serves cannot be told; nothing is served in its place",
        pointer.display()
    );
    let served = served_answer(
        &root,
        &ask(GDFL_NIFTY),
        (path, relative.to_owned(), &meta),
        &bars,
        None,
    );
    drop(bars);
    assert_eq!(
        served.status,
        StatusCode::INTERNAL_SERVER_ERROR,
        "{}",
        served.body
    );
    assert_eq!(served.body["error"], refused);
    assert_eq!(served.body["path"], relative);
    assert!(served.body.get("bars").is_none(), "{}", served.body);
    let fresh = get(&root, GDFL_NIFTY);
    assert_eq!(
        fresh.status,
        StatusCode::INTERNAL_SERVER_ERROR,
        "{}",
        fresh.body
    );
    assert_eq!(fresh.body["error"], refused);
    assert_eq!(fresh.body["path"], relative);
}

/// **AN IMPORT JOURNAL THAT WILL NOT READ, OR A RECORD DATED ON A DAY NO
/// CALENDAR NAMES, IS SAID WHERE ITS JOURNAL ROW WOULD BE** (BASE-COV-API-AS2).
/// Extremes: 11 (7 single, 2 combined, 2 out of the box): (1) an import journal
/// whose folder is a file: a recorded landing's `journal` is the journal's own
/// refusal, under its source; (2) the reconstruction of the month's unrecorded
/// bars is the same refusal, and lists no day; (3) the month is still a 200
/// over its bars; (4) its label says recorded and reconstructed at once; (5) a
/// landing dated 1969-12-31, a day the sidecar keeps and the calendar refuses,
/// is "not a day the calendar can name"; (6) each record still serves its own
/// `day` as stored; (7) a month every record covers is `recorded`, with no
/// reconstruction. Combined: (8) a pre-epoch day AND an import journal that
/// reads: the day is refused, never the journal, and no journal row is shown;
/// (9) no journal read at all (a direct call): the reconstruction says the
/// import journal was not read AND a record's journal is `null`. Out of the
/// box: (10) a landing dated 0001-01-01, year one, is written in four digits
/// and refused the same way; (11) the refusal names the path the store would
/// read, below a folder that is a file.
#[test]
#[allow(
    clippy::too_many_lines,
    reason = "one test walks every journal state its Extremes: line names, in order"
)]
fn an_import_journal_that_will_not_read_or_a_day_no_calendar_names_is_said() {
    let path = path_of(
        Vendor::Gdfl,
        "INDEX",
        "NIFTY",
        None,
        Timeframe::SECOND_1,
        "2019-03",
    );
    // (1), (2), (3), (4), (11): the import journal's folder is a file.
    let root = root_of("as2-g2-imports-file");
    std::fs::write(root.join("imports"), b"not a folder").unwrap();
    let mut bars = BarFile::open_or_create(&root, path, symbol_id("NIFTY")).unwrap();
    let _unrecorded = bars.append(&seconds("2019-03-01", 3)).unwrap();
    let landed = bars.append(&seconds("2019-03-05", 2)).unwrap();
    store::provenance::record(
        &root,
        &bars,
        path,
        landed,
        &member_landing("m.zip!a.csv", "2019-03-05", 7),
    )
    .unwrap();
    drop(bars);
    let got = get(&root, GDFL_NIFTY);
    assert_eq!(got.status, StatusCode::OK, "{}", got.body);
    let journal = &got.body["recorded"][0]["journal"];
    assert_eq!(journal["source"], "imports/gdfl.journal", "{}", got.body);
    let why = journal["error"].as_str().unwrap();
    assert!(
        why.starts_with(&format!(
            "{}: ",
            pull::gdfl_import::journal_path(&root).display()
        )) && why.contains("Not a directory"),
        "{why}"
    );
    assert_eq!(
        got.body["reconstruction"],
        serde_json::json!({ "source": "imports/gdfl.journal", "error": why })
    );
    assert!(
        got.body["label"]
            .as_str()
            .unwrap()
            .starts_with("recorded for the bars the sidecar's records hold"),
        "{}",
        got.body
    );
    // (5), (6), (7), (8), (10): landings dated on days no calendar names.
    let root = root_of("as2-g2-pre-epoch-days");
    gdfl_journal(&root, NIFTY_MARCH);
    let mut bars = BarFile::open_or_create(&root, path, symbol_id("NIFTY")).unwrap();
    for (batch, on) in [
        (seconds("2019-03-01", 3), "1969-12-31"),
        (seconds("2019-03-05", 2), "0001-01-01"),
    ] {
        let landed = bars.append(&batch).unwrap();
        store::provenance::record(
            &root,
            &bars,
            path,
            landed,
            &member_landing("m.zip!a.csv", on, 2),
        )
        .unwrap();
    }
    drop(bars);
    let got = get(&root, GDFL_NIFTY);
    assert_eq!(got.status, StatusCode::OK, "{}", got.body);
    assert_eq!(got.body["label"], "recorded");
    assert!(got.body["reconstruction"].is_null(), "{}", got.body);
    let records = got.body["recorded"].as_array().unwrap();
    assert_eq!(records.len(), 2, "{}", got.body);
    for (record, on) in records.iter().zip(["1969-12-31", "0001-01-01"]) {
        assert_eq!(record["day"], on);
        assert_eq!(
            record["journal"],
            serde_json::json!({
                "source": "imports/gdfl.journal",
                "error": format!("{on} is not a day the calendar can name"),
            })
        );
    }
    // (9): no journal read at all.
    let unread = Journals {
        gdfl: None,
        pull: None,
    };
    let asked = ask(GDFL_NIFTY);
    let month = BarFile::open_existing(&root, path, symbol_id("NIFTY")).unwrap();
    assert_eq!(
        gdfl_reconstruction(&unread, &asked, &month, 0),
        serde_json::json!({
            "source": "imports/gdfl.journal",
            "error": "the import journal was not read",
        })
    );
    let Found::Present(sidecar) = store::provenance::read(&root, path).unwrap() else {
        panic!("the sidecar is present");
    };
    assert!(journal_of_record(&sidecar.records()[0], &unread, &asked).is_null());
}

/// One GDFL index month's journal: two filters closing the 1st under run 11,
/// the 5th under run 22, and a third run line that no close names.
const TWO_FILTERS: &str = "\
run 11 source=zip archive=/archives/a.zip
run 22 source=tickstore archive=/ticks/cm
run 33 source=zip archive=/archives/unused.zip
begin indices 2019-03-01 *
done indices 2019-03-01 * definition=2 files=2 seconds=3 failures=0 late=0 late_unresolved=0 max_back_s=0 run=11
begin indices 2019-03-01 NIFTY
done indices 2019-03-01 NIFTY definition=2 files=1 seconds=3 failures=0 late=0 late_unresolved=0 max_back_s=0 run=11
begin indices 2019-03-05 *
done indices 2019-03-05 * definition=2 files=2 seconds=2 failures=0 late=0 late_unresolved=0 max_back_s=0 run=22
";

/// **A RUN NAMED BY TWO CLOSES IS LISTED ONCE, AND TWO FILTERS CLOSING ONE DAY
/// SHARE ONE LOOKUP OF ITS BARS** (BASE-COV-API-AS2).
/// Extremes: 11 (7 single, 2 combined, 2 out of the box): (1) one day closed
/// under two filters is two rows, in filter order; (2) the second row's bars
/// are the first's span, answered from the day already looked up; (3) a run
/// named by both closes of that day is listed once; (4) the later day's run
/// follows it, in the order the rows first name them; (5) the archive is
/// recorded: every row names a run the journal holds; (6) the rows count three
/// journal days while two days hold bars, and the five bars are counted once;
/// (7) the commits agree with the days holding bars, two and two. Combined:
/// (8) the same day AND the same run on two rows: one span and one run line for
/// both; (9) the run-id index built for one snapshot answers a later id from
/// the same build AND an id the journal never wrote is `None`. Out of the box:
/// (10) a filter naming the instrument itself (`NIFTY`) beside the whole
/// tree's `*` on one day; (11) a run line no close names is never listed as an
/// archive, though the index finds it.
#[test]
fn a_run_named_by_two_closes_is_listed_once_and_one_days_filters_share_its_bars() {
    let root = root_of("as2-g2-two-filters");
    gdfl_journal(&root, TWO_FILTERS);
    let path = path_of(
        Vendor::Gdfl,
        "INDEX",
        "NIFTY",
        None,
        Timeframe::SECOND_1,
        "2019-03",
    );
    land(
        &root,
        path,
        "NIFTY",
        &[seconds("2019-03-01", 3), seconds("2019-03-05", 2)],
    );
    let got = get(&root, GDFL_NIFTY);
    assert_eq!(got.status, StatusCode::OK, "{}", got.body);
    let rebuilt = &got.body["reconstruction"];
    // (1), (10)
    let by_day: Vec<(&str, u64, u64)> = rebuilt["days"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            (
                row["filter"].as_str().unwrap(),
                row["line"].as_u64().unwrap(),
                row["run"].as_u64().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        by_day,
        [("*", 5, 11), ("NIFTY", 7, 11), ("*", 9, 22)],
        "{rebuilt}"
    );
    // (2), (8)
    let first = serde_json::json!({ "first_index": 0, "end": 3, "count": 3 });
    assert_eq!(rebuilt["days"][0]["bars"], first);
    assert_eq!(rebuilt["days"][1]["bars"], first);
    assert_eq!(
        rebuilt["days"][2]["bars"],
        serde_json::json!({ "first_index": 3, "end": 5, "count": 2 })
    );
    // (6), (7)
    assert_eq!(rebuilt["journal_days"], 3);
    assert_eq!(rebuilt["journal_days_with_bars"], 2);
    assert_eq!(rebuilt["bars_on_journal_days"], 5);
    assert_eq!(rebuilt["bars_on_no_journal_day"], 0);
    assert_eq!(rebuilt["commits_agree"], true);
    // (3), (4), (5), (11)
    let archive = &rebuilt["archive"];
    let listed: Vec<(u64, u64)> = archive["runs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|named| {
            (
                named["id"].as_u64().unwrap(),
                named["line"].as_u64().unwrap(),
            )
        })
        .collect();
    assert_eq!(listed, [(11, 1), (22, 2)], "{archive}");
    assert_eq!(
        archive["runs"][1]["fields"],
        serde_json::json!({ "source": "tickstore", "archive": "/ticks/cm" })
    );
    assert_eq!(archive["recorded"], true);
    assert_eq!(archive["days_naming_no_run"], 0);
    assert_eq!(archive["runs_named_and_absent"], 0);
    // (9), (11): the run-id index of this snapshot, asked again.
    let parsed = super::super::import_journal::snapshot(&root)
        .unwrap()
        .parsed;
    let found: Vec<Option<(u64, u64)>> = [22, 33, 11, 44]
        .into_iter()
        .map(|id| run_line(&parsed, id).map(|line| (line.id, line.line)))
        .collect();
    assert_eq!(found, [Some((22, 2)), Some((33, 3)), Some((11, 1)), None]);
}

/// **A DAY WHOSE BARS WILL NOT READ IS A LOOKUP ERROR NAMED BY ITS DAY, NEVER A
/// DAY HOLDING NO BAR** (BASE-COV-API-AS2). One byte of bar 0 is flipped, so the
/// checksum block that holds all five bars of the month no longer verifies.
/// Extremes: 11 (8 single, 2 combined, 1 out of the box): (1) the 1st's lookup
/// is an error named `2019-03-01: `; (2) the 5th's is too, though the 1st
/// failed: a failed day lends no start to the next; (3) both name block 0 and
/// the bytes that were not committed; (4) `bar_lookup_errors` counts the two
/// days; (5) no bar is counted on a journal day; (6) no journal day is said to
/// hold a bar; (7) the two commits disagree with the zero days holding bars;
/// (8) the month is still a 200, labelled reconstructed, its sidecar absent.
/// Combined: (9) the damage AND a request for one bar by `t`: a 404 naming `t`
/// and the block, before any journal is read; (10) the damage AND the whole
/// month: the five bars no lookup could place are never claimed to lie on no
/// journal day: the figure is null, beside the two errors that say why. Out
/// of the box: (11) one flipped byte in
/// bar 0 withholds the 5th's bars too, which share its 73-record block.
#[test]
fn a_day_whose_bars_will_not_read_is_a_lookup_error_named_by_its_day() {
    let root = root_of("as2-g2-damaged-block");
    let _month = nifty_march(&root);
    let path = path_of(
        Vendor::Gdfl,
        "INDEX",
        "NIFTY",
        None,
        Timeframe::SECOND_1,
        "2019-03",
    );
    // Byte 9 of bar 0: inside its open price, under block 0's checksum.
    let offset = store::format::HEADER_LEN + 9;
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path.to_path_buf(&root))
        .unwrap();
    let mut byte = [0_u8; 1];
    std::os::unix::fs::FileExt::read_exact_at(&file, &mut byte, offset).unwrap();
    byte[0] ^= 0x5A;
    std::os::unix::fs::FileExt::write_all_at(&file, &byte, offset).unwrap();
    drop(file);
    // (8)
    let got = get(&root, GDFL_NIFTY);
    assert_eq!(got.status, StatusCode::OK, "{}", got.body);
    assert_eq!(got.body["label"], RECONSTRUCTED);
    assert_eq!(got.body["sidecar"]["state"], "absent");
    // (1), (2), (3), (11)
    let rebuilt = &got.body["reconstruction"];
    let errors: Vec<&str> = rebuilt["days"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["bars"]["error"].as_str().unwrap())
        .collect();
    assert_eq!(errors.len(), 2, "{rebuilt}");
    for (why, on) in errors.iter().zip(["2019-03-01: ", "2019-03-05: "]) {
        assert!(
            why.starts_with(on)
                && why.contains("block 0 was sealed")
                && why.ends_with("those are not the bytes that were committed"),
            "{why}"
        );
    }
    // (4), (5), (6), (7), (10)
    assert_eq!(rebuilt["bar_lookup_errors"], 2);
    assert_eq!(rebuilt["bars_on_journal_days"], 0);
    assert_eq!(rebuilt["journal_days_with_bars"], 0);
    assert_eq!(rebuilt["bar_file_commits"], 2);
    assert_eq!(rebuilt["commits_agree"], false);
    assert_eq!(
        rebuilt["bars_on_no_journal_day"],
        serde_json::Value::Null,
        "(10) bars of a day whose lookup failed are never claimed off the journal"
    );
    // (9)
    let fifth = ist_midnight("2019-03-05") + 9 * 3_600 + 15 * 60;
    let one = get(&root, &format!("{GDFL_NIFTY}&t={fifth}"));
    assert_eq!(one.status, StatusCode::NOT_FOUND, "{}", one.body);
    let why = one.body["error"].as_str().unwrap();
    assert!(
        why.starts_with(&format!("t={fifth}: ")) && why.contains("block 0 was sealed"),
        "{why}"
    );
    assert!(one.body.get("reconstruction").is_none(), "{}", one.body);
}

/// **A DAY IS LOOKED UP AFRESH WHEN IT COMES BEFORE THE LAST DAY ASKED, AND IS
/// ANSWERED FROM THE LAST ENTRY WHEN IT IS THAT DAY** (BASE-COV-API-AS2), over
/// `DaySpans` directly: the route asks days in day order, a direct caller need
/// not.
/// Extremes: 10 (6 single, 2 combined, 2 out of the box): (1) the 5th asked
/// first is its two bars; (2) the 1st asked after it is looked up afresh, never
/// given the 5th's end, and is its three bars; (3) the 1st asked again is the
/// last entry, not entered twice; (4) the 3rd, a day with no bar, is an empty
/// span at bar 3; (5) the 31st, past every bar, is empty at the bar count;
/// (6) the totals count each bar once and no error, and two days hold bars.
/// Combined: (7) a day before the last asked AND the same day at once: one
/// lookup, two equal answers; (8) the 3rd after the 1st: the 1st's end is the
/// 3rd's start AND its end, an empty day settled by one bar read. Out of the
/// box: (9) days asked backwards; (10) a day past the month's last bar answers
/// the bar count twice, never a bar.
#[test]
fn day_spans_asked_out_of_day_order_look_each_earlier_day_up_afresh() {
    let root = root_of("as2-g2-day-spans");
    let path = path_of(
        Vendor::Gdfl,
        "INDEX",
        "NIFTY",
        None,
        Timeframe::SECOND_1,
        "2019-03",
    );
    land(
        &root,
        path,
        "NIFTY",
        &[seconds("2019-03-01", 3), seconds("2019-03-05", 2)],
    );
    let bars = BarFile::open_existing(&root, path, symbol_id("NIFTY")).unwrap();
    let mut spans = DaySpans::with_capacity(4);
    // (1)
    assert_eq!(spans.of(&bars, day("2019-03-05")), Ok((3, 5)));
    // (2), (9)
    assert_eq!(
        spans.of(&bars, day("2019-03-01")),
        Ok((0, 3)),
        "a day before the last one asked is looked up afresh"
    );
    // (3), (7)
    assert_eq!(
        spans.of(&bars, day("2019-03-01")),
        Ok((0, 3)),
        "the same day again is the last entry"
    );
    // (4), (8)
    assert_eq!(spans.of(&bars, day("2019-03-03")), Ok((3, 3)));
    // (5), (10)
    assert_eq!(spans.of(&bars, day("2019-03-31")), Ok((5, 5)));
    // (3), (6)
    assert_eq!(spans.seen.len(), 4, "the repeated day is entered once");
    assert_eq!(spans.totals(), (5, 0));
    assert_eq!(spans.days_with_bars(), 2);
}

/// **A CONTRACT IS NAMED BY NO CAPITAL-MARKET STEM, WHATEVER KIND OR SCOPE IT
/// IS ASKED WITH** (BASE-COV-API-AS2), over `vendor_files` directly: the route
/// asks an option only of the options kind, a direct caller may ask it of any.
/// Extremes: 11 (8 single, 2 combined, 1 out of the box): (1) a contract asked
/// of the indices kind names the indices day folder and no stem; (2) of the
/// stocks kind, the stock day folder and no stem; (3) its day zip and
/// tick-store day are the indices kind's; (4) it is named by the
/// capital-market readers; (5) the contract asked of its own kind names the
/// options day folder, no stem and no zip; (6) the index asked of its kind with
/// its scope names its stem; (7) the index with no scope names none; (8) a
/// contract has no capital-market scope of its own. Combined: (9) a contract
/// AND the index's scope passed: still no stem, of either kind; (10) a contract
/// AND every capital-market kind AND no scope: no stem. Out of the box: (11) the
/// index asked of the stocks kind keeps its own stem beside the stock day
/// folder: the stem is the instrument's, the folder the kind's.
#[test]
fn a_contract_asked_of_a_capital_market_kind_names_no_stem() {
    let on = day("2019-03-01");
    let option =
        ask("feed=gdfl&instrument=NSE-FNO-NIFTY-2019-03-28-1150000-CE&timeframe=1s&month=2019-03");
    let index = ask(GDFL_NIFTY);
    let index_scope = cm_scope(&index);
    // (8)
    assert!(index_scope.is_some());
    assert!(cm_scope(&option).is_none());
    // (1), (2), (4), (9), (10)
    for (kind, folder) in [
        (ImportKind::Indices, "GFDLCM_INDICES_TICK_01032019"),
        (ImportKind::Stocks, "GFDLCM_STOCK_TICK_01032019"),
    ] {
        for scope in [None, index_scope.as_ref()] {
            let files = vendor_files(&option, kind, on, scope);
            assert_eq!(files["day_folder"], folder, "{files}");
            assert!(files["stem"].is_null(), "{files}");
            assert!(
                files["named_by"]
                    .as_str()
                    .unwrap()
                    .contains("capital-market readers"),
                "{files}"
            );
        }
    }
    // (3)
    let indices = vendor_files(&option, ImportKind::Indices, on, None);
    assert_eq!(
        indices["zip_day"],
        "INDICES/2019/MAR_2019/GFDLCM_INDICES_TICK_01032019.zip"
    );
    assert_eq!(
        indices["tickstore_day"],
        "cm/INDICES/2019/MAR_2019/GFDLCM_INDICES_TICK_01032019.bts"
    );
    // (5)
    let own = vendor_files(&option, ImportKind::Options, on, None);
    assert_eq!(own["day_folder"], "GFDLNFO_TICK_01032019");
    assert!(
        own["stem"].is_null() && own.get("zip_day").is_none(),
        "{own}"
    );
    // (6), (7)
    assert_eq!(
        vendor_files(&index, ImportKind::Indices, on, index_scope.as_ref())["stem"],
        "NIFTY 50.NSE_IDX"
    );
    assert!(vendor_files(&index, ImportKind::Indices, on, None)["stem"].is_null());
    // (11)
    let crossed = vendor_files(&index, ImportKind::Stocks, on, index_scope.as_ref());
    assert_eq!(crossed["day_folder"], "GFDLCM_STOCK_TICK_01032019");
    assert_eq!(crossed["stem"], "NIFTY 50.NSE_IDX");
}

/// **A PULL RECORD THE JOURNAL CANNOT GIVE IS SAID BY WHY** (BASE-COV-API-AS2):
/// a record that will not decode is named with its ordinal and its fault, a
/// page the journal cannot read is the read's refusal, and no journal at all is
/// `null`.
/// Extremes: 11 (8 single, 2 combined, 1 out of the box): (1) a sidecar record
/// naming a journal record of zeros: its `journal` names that ordinal and "not
/// an audit record"; (2) nothing else is put in its place: no run's fields;
/// (3) the record's own `journal_ordinal` is served beside it; (4) the walk
/// over the same journal counts the zero record undecodable; (5) nine records,
/// seven of them runs; (6) a page past the bytes the journal holds is the
/// read's refusal, naming the journal; (7) an ordinal at the count given is
/// past the journal; (8) no pull journal at all is `null`. Combined: (9) an
/// unrecorded bar AND an undecodable named record: the reconstruction and the
/// record both speak of the one zero record; (10) a count of 100 AND a journal
/// of nine: the page is refused by its read, never answered empty. Out of the
/// box: (11) a record of 256 zero bytes, whose kind byte reads as a run.
#[test]
#[allow(
    clippy::too_many_lines,
    reason = "one test walks every journal fault its Extremes: line names, in order"
)]
fn a_pull_record_the_journal_cannot_give_is_said_by_why() {
    let root = root_of("as2-g2-pull-undecodable");
    pull_journal(&root);
    // (11): record 8 is 256 zero bytes.
    {
        use std::io::Write as _;
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(audit::journal_path(&root))
            .unwrap();
        file.write_all(&[0_u8; audit::RECORD_LEN]).unwrap();
    }
    let path = path_of(
        Vendor::Zerodha,
        "INDEX",
        "NIFTY",
        None,
        Timeframe::DAY_1,
        "2015-01",
    );
    let mut bars = BarFile::open_or_create(&root, path, symbol_id("NIFTY")).unwrap();
    let _unrecorded = bars
        .append(&[bar(ist_midnight("2015-01-01"), 5_55)])
        .unwrap();
    let landed = bars
        .append(&[bar(ist_midnight("2015-01-02"), 5_55)])
        .unwrap();
    let landing = Landing {
        source: Source {
            kind: SourceKind::VendorEndpoint,
            name: "/instruments/historical/999999/day",
            bytes: 99,
            crc32: 0,
        },
        journal_ordinal: 8,
        ..member_landing("unused", "2015-01-02", 0)
    };
    store::provenance::record(&root, &bars, path, landed, &landing).unwrap();
    drop(bars);
    // (1), (2), (3), (9)
    let got = get(&root, ZERODHA_2015);
    assert_eq!(got.status, StatusCode::OK, "{}", got.body);
    let records = got.body["recorded"].as_array().unwrap();
    assert_eq!(records.len(), 1, "{}", got.body);
    assert_eq!(records[0]["journal_ordinal"], 8);
    assert_eq!(
        records[0]["journal"],
        serde_json::json!({
            "source": "audit/pull.journal",
            "ordinal": 8,
            "error": "not an audit record",
        })
    );
    // (4), (5)
    let walked = &got.body["reconstruction"]["journal"];
    assert_eq!(walked["records"], 9, "{}", got.body);
    assert_eq!(walked["runs"], 7);
    assert_eq!(walked["undecodable"], 1);
    // (6), (7), (8), (10): the journal of a record, asked directly.
    let asked = ask(ZERODHA_2015);
    let Found::Present(sidecar) = store::provenance::read(&root, path).unwrap() else {
        panic!("the sidecar is present");
    };
    let mut named = sidecar.records()[0].clone();
    named.journal_ordinal = 50;
    let inflated = Journals {
        gdfl: None,
        pull: Some((audit::Journal::at(&root), 100)),
    };
    let unread = journal_of_record(&named, &inflated, &asked);
    assert_eq!(unread["source"], "audit/pull.journal", "{unread}");
    assert!(
        unread["error"].as_str().unwrap().starts_with(&format!(
            "{}: cannot read 1 record(s)",
            audit::journal_path(&root).display()
        )),
        "{unread}"
    );
    named.journal_ordinal = 100;
    assert!(
        journal_of_record(&named, &inflated, &asked)["error"]
            .as_str()
            .unwrap()
            .contains("record 100 is past the journal's 100 records")
    );
    let none = Journals {
        gdfl: None,
        pull: None,
    };
    assert!(journal_of_record(&named, &none, &asked).is_null());
}

/// **THE PULL JOURNAL'S WALK SAYS A LONG FIRST READ, REFUSES A JOURNAL IT
/// CANNOT MEASURE, AND READS ONE THAT SHRANK IN PLACE AGAIN FROM ITS START**
/// (BASE-COV-API-AS2).
/// Extremes: 11 (8 single, 2 combined, 1 out of the box): (1) a journal of
/// 100,001 records is a first walk the answer names in `notes`, with the count;
/// (2) and with the time it took, in ms; (3) every zero record is counted
/// undecodable and none is a run; (4) the next request reads nothing and names
/// nothing; (5) a journal whose folder is a file: the walk's refusal stands in
/// for the reconstruction, under its source and nothing else; (6) that refusal
/// names the journal's path; (7) the month is still a 200, reconstructed;
/// (8) a journal truncated in place to one record is read again from its
/// start: one record, one run, and that run windowless. Combined: (9) a sparse
/// file of zeros AND a month with no sidecar: the note and the counts at once;
/// (10) a file where the journal's folder goes AND a walk asked directly: the
/// same words. Out of the box: (11) a 25.6 MB hole standing for the journal.
#[test]
#[allow(
    clippy::too_many_lines,
    reason = "one test walks every journal shape its Extremes: line names, in order"
)]
fn the_pull_journal_walk_says_a_long_first_read_and_a_journal_it_cannot_measure() {
    // (1), (2), (3), (4), (9), (11): 100,001 records of zeros, a sparse file.
    let root = root_of("as2-g2-pull-long-walk");
    let journal = audit::journal_path(&root);
    std::fs::create_dir_all(journal.parent().unwrap()).unwrap();
    let whole = FIRST_WALK_RECORDS + 1;
    std::fs::File::create(&journal)
        .unwrap()
        .set_len(whole * audit::RECORD_LEN_U64)
        .unwrap();
    zerodha_days(&root, "2015-01", &["2015-01-01"], 1, T0 + 1);
    let first = get(&root, ZERODHA_2015);
    assert_eq!(first.status, StatusCode::OK, "{}", first.body);
    let walked = &first.body["reconstruction"]["journal"];
    assert_eq!(walked["records"], whole, "{}", first.body);
    assert_eq!(walked["read_now"], whole);
    assert_eq!(walked["undecodable"], whole);
    assert_eq!(walked["runs"], 0);
    let notes: Vec<&str> = first.body["notes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|note| note.as_str().unwrap())
        .collect();
    let lead = format!(
        "the pull journal keeps no index of its runs, so this process's first read of it \
         decoded {whole} records in "
    );
    assert_eq!(notes.len(), 1, "{notes:?}");
    assert!(
        notes[0].starts_with(&lead)
            && notes[0].ends_with(" ms; later requests read only what is appended"),
        "{notes:?}"
    );
    let again = get(&root, ZERODHA_2015);
    assert_eq!(again.body["reconstruction"]["journal"]["read_now"], 0);
    assert_eq!(again.body["notes"], serde_json::json!([]), "{}", again.body);
    // (5), (6), (7), (10): a file where the journal's folder goes.
    let root = root_of("as2-g2-audit-file");
    std::fs::write(root.join("audit"), b"not a folder").unwrap();
    zerodha_days(&root, "2015-01", &["2015-01-01"], 1, T0 + 1);
    let refused = get(&root, ZERODHA_2015);
    assert_eq!(refused.status, StatusCode::OK, "{}", refused.body);
    assert_eq!(refused.body["label"], RECONSTRUCTED);
    let why = refused.body["reconstruction"]["error"].as_str().unwrap();
    assert!(
        why.starts_with(&format!("{}: ", audit::journal_path(&root).display()))
            && why.contains("Not a directory"),
        "{why}"
    );
    assert_eq!(
        refused.body["reconstruction"],
        serde_json::json!({ "source": "audit/pull.journal", "error": why })
    );
    let Err(direct) = read_runs(&root) else {
        panic!("a journal below a file is refused");
    };
    assert_eq!(direct, why);
    // (8): truncated in place to one record.
    let root = root_of("as2-g2-pull-shrunk");
    pull_journal(&root);
    assert_eq!(read_runs(&root).unwrap().records, 8);
    std::fs::OpenOptions::new()
        .write(true)
        .open(audit::journal_path(&root))
        .unwrap()
        .set_len(audit::RECORD_LEN_U64)
        .unwrap();
    let shrunk = read_runs(&root).unwrap();
    assert_eq!(
        (
            shrunk.records,
            shrunk.read_now,
            shrunk.index.runs.len(),
            shrunk.index.unwindowed
        ),
        (1, 1, 1, 1)
    );
}

/// **A PULL JOURNAL THAT ENDS BEFORE THE LENGTH ITS HOST GIVES IS REFUSED BY
/// THE RECORD IT ENDED BEFORE** (BASE-COV-API-AS2). A kernel attribute file
/// reports a page of length and holds a few bytes, so one linked at the
/// journal's name is a journal whose measure promises records its bytes do not
/// hold: the shape of a journal cut short between its measure and its read.
/// Extremes: 11 (8 single, 2 combined, 1 out of the box): (1) a journal whose
/// length promises whole records and whose bytes end inside the first is
/// refused; (2) the refusal names record 0 and the whole count the length
/// promised; (3) it names the journal by the path the store reads, the link's
/// own name; (4) a second walk refuses the same, since the failed walk kept
/// nothing; (5) the answer over it carries the refusal as its reconstruction,
/// under its source; (6) and nothing else: no candidate, no count; (7) the
/// month is still a 200; (8) labelled reconstructed. Combined: (9) a journal
/// cut short AND a month with no sidecar: the refusal stands in for the whole
/// reconstruction; (10) a journal cut short AND two walks in a row: the same
/// words both times. Out of the box: (11) a kernel attribute file standing at
/// the journal's name.
#[cfg(target_os = "linux")]
#[test]
fn a_pull_journal_that_ends_before_its_length_is_refused_by_the_record_it_ended_before() {
    // (11)
    let attribute = Path::new("/sys/devices/system/cpu/online");
    let claimed = std::fs::metadata(attribute)
        .expect("sysfs is mounted")
        .len();
    let held = std::fs::read(attribute).expect("the attribute reads").len();
    assert!(
        claimed >= audit::RECORD_LEN_U64 && held < audit::RECORD_LEN,
        "the attribute claims {claimed} bytes and holds {held}"
    );
    let root = root_of("as2-g2-pull-cut-short");
    let journal = audit::journal_path(&root);
    std::fs::create_dir_all(journal.parent().unwrap()).unwrap();
    std::os::unix::fs::symlink(attribute, &journal).unwrap();
    // (1), (2), (3), (4), (10)
    let said = format!(
        "{}: the file ended before record 0 of {}",
        journal.display(),
        claimed / audit::RECORD_LEN_U64
    );
    for _ in 0..2 {
        let Err(why) = read_runs(&root) else {
            panic!("a journal that ends before its length is refused");
        };
        assert_eq!(why, said);
    }
    // (5), (6), (7), (8), (9)
    zerodha_days(&root, "2015-01", &["2015-01-01"], 1, T0 + 1);
    let got = get(&root, ZERODHA_2015);
    assert_eq!(got.status, StatusCode::OK, "{}", got.body);
    assert_eq!(got.body["label"], RECONSTRUCTED);
    assert_eq!(
        got.body["reconstruction"],
        serde_json::json!({ "source": "audit/pull.journal", "error": said })
    );
}

/// **EVERY RUN COVERING THE MONTH IS SORTED BY WHAT IT CAN SAY OF ITS BARS**
/// (BASE-COV-API-AS2): refused, never started or offering no bar is counted and
/// never a candidate; a window that misses `t`'s day, starts at the epoch, or
/// ends before it begins is filed by what it names; an option's month takes
/// only its feed's expired-F&O runs.
/// Extremes: 11 (8 single, 2 combined, 1 out of the box): (1) a refused run of
/// this feed and form covering the month is counted refused, never a
/// candidate; (2) a run that never started is counted with it; (3) a run that
/// stored and offered no bar is counted as offering none; (4) a run whose
/// window starts after `t`'s day misses it, though it covers the month; (5) a
/// window starting on the epoch's own day is filed under the month and is a
/// candidate, its start shown as no day; (6) a reversed window is walked on
/// every request and misses every day; (7) the epoch-day candidate is named the
/// writer: one commit, written inside its run alone; (8) an option's month
/// takes only the expired-F&O run, its feed's spot runs being the other form.
/// Combined: (9) the whole month AND the late-starting run: a candidate there,
/// excluded only by `t`; (10) a direct walk AND a position no run holds: passed
/// over and counted nowhere. Out of the box: (11) a window that ends before it
/// begins.
#[test]
#[allow(
    clippy::too_many_lines,
    reason = "one test walks every run shape its Extremes: line names, in order"
)]
fn every_run_covering_the_month_is_sorted_by_what_it_can_say_of_its_bars() {
    use audit::{Outcome, Scope};
    let january = Some(("2015-01-01", "2015-01-31"));
    let ordinals = |runs: &Value| -> Vec<u64> {
        runs.as_array()
            .unwrap()
            .iter()
            .map(|one| one["ordinal"].as_u64().unwrap())
            .collect()
    };
    // (1)-(7), (9), (11): six runs of one feed and form over January 2015.
    let root = root_of("as2-g2-sorted");
    let mut epoch = run(Scope::Spot, Outcome::Stored, T0, 10, KITE, january, 4);
    epoch.from_days = 0;
    let mut reversed = run(Scope::Spot, Outcome::Stored, T0 + 60, 5, KITE, january, 4);
    reversed.from_days = day("2015-02-01").days_from_epoch();
    reversed.to_days = day("2015-01-01").days_from_epoch();
    let journal = audit::Journal::at(&root);
    for record in [
        run(Scope::Spot, Outcome::Refused, T0 - 300, 0, KITE, january, 0),
        run(
            Scope::Spot,
            Outcome::NotStarted,
            T0 - 200,
            0,
            KITE,
            january,
            0,
        ),
        run(Scope::Spot, Outcome::Stored, T0 - 100, 5, KITE, january, 0),
        run(
            Scope::Spot,
            Outcome::Stored,
            T0 + 50,
            5,
            KITE,
            Some(("2015-01-05", "2015-01-31")),
            4,
        ),
        epoch,
        reversed,
    ] {
        journal.append(&record).unwrap();
    }
    zerodha_days(&root, "2015-01", &["2015-01-01", "2015-01-02"], 1, T0 + 5);
    let month = get(&root, ZERODHA_2015);
    assert_eq!(month.status, StatusCode::OK, "{}", month.body);
    let rebuilt = &month.body["reconstruction"];
    assert_eq!(ordinals(&rebuilt["candidates"]), [3, 4], "{rebuilt}");
    assert_eq!(
        rebuilt["excluded"],
        serde_json::json!({
            "other_feed": 0,
            "names_no_feed": 0,
            "other_scope": 0,
            "window_misses_t": 1,
            "refused_or_not_started": 2,
            "offered_no_bar": 1,
        })
    );
    assert_eq!(rebuilt["journal"]["runs"], 6);
    assert_eq!(rebuilt["journal"]["runs_without_window"], 0);
    assert_eq!(rebuilt["journal"]["runs_walked_on_every_request"], 1);
    assert_eq!(
        rebuilt["candidates"][1]["window"],
        serde_json::json!({ "from": null, "to": "2015-01-31" })
    );
    assert_eq!(rebuilt["attributed"]["run"], 4);
    let second = get(
        &root,
        &format!("{ZERODHA_2015}&t={}", ist_midnight("2015-01-02")),
    );
    assert_eq!(second.status, StatusCode::OK, "{}", second.body);
    let rebuilt = &second.body["reconstruction"];
    assert_eq!(ordinals(&rebuilt["candidates"]), [4], "{rebuilt}");
    assert_eq!(rebuilt["excluded"]["window_misses_t"], 2);
    assert_eq!(rebuilt["excluded"]["refused_or_not_started"], 2);
    assert_eq!(rebuilt["excluded"]["offered_no_bar"], 1);
    assert_eq!(rebuilt["attributed"]["run"], 4);
    // (8): an option's month over the pull-journal fixture.
    let root = root_of("as2-g2-sorted-option");
    pull_journal(&root);
    let contract = Contract::parse("2015-01-29-850000-CE").unwrap();
    let option = path_of(
        Vendor::Zerodha,
        "FNO",
        "NIFTY",
        Some(contract),
        Timeframe::DAY_1,
        "2015-01",
    );
    land(
        &root,
        option,
        "NIFTY",
        &[vec![bar(ist_midnight("2015-01-01"), 5_55)]],
    );
    std::fs::File::options()
        .write(true)
        .open(option.to_path_buf(&root))
        .unwrap()
        .set_modified(at(T0 + 9_010))
        .unwrap();
    let got = get(
        &root,
        "feed=zerodha&instrument=NSE-FNO-NIFTY-2015-01-29-850000-CE&timeframe=1day&month=2015-01",
    );
    assert_eq!(got.status, StatusCode::OK, "{}", got.body);
    let rebuilt = &got.body["reconstruction"];
    assert_eq!(ordinals(&rebuilt["candidates"]), [6], "{rebuilt}");
    assert_eq!(rebuilt["candidates"][0]["scope"], "expired F&O");
    assert_eq!(rebuilt["excluded"]["other_scope"], 2);
    assert_eq!(rebuilt["excluded"]["names_no_feed"], 1);
    assert_eq!(rebuilt["attributed"]["run"], 6);
    // (10): a direct walk over a position no run holds.
    let mut index = RunIndex::empty();
    index.push(PullRun {
        ordinal: 0,
        vendor: Some(Vendor::Zerodha),
        record: run(Scope::Spot, Outcome::Stored, T0, 10, KITE, january, 4),
    });
    let asked = ask(ZERODHA_2015);
    let walked = sorted(
        &index,
        &[0, 7],
        &asked,
        (day("2015-01-01"), day("2015-01-31")),
    );
    assert_eq!(
        walked
            .candidates
            .iter()
            .map(|one| one.ordinal)
            .collect::<Vec<_>>(),
        [0]
    );
    assert_eq!(
        (
            walked.closing.len(),
            walked.opened.len(),
            walked.continuations
        ),
        (1, 0, 0)
    );
    let excluded = &walked.excluded;
    assert_eq!(
        [
            excluded.other_feed,
            excluded.unattributed,
            excluded.other_scope,
            excluded.other_day,
            excluded.not_run,
            excluded.offered_nothing,
        ],
        [0; 6],
        "a position no run holds is passed over and counted nowhere"
    );
}

/// **AN OPENING RECORD IS ANSWERED ONLY BY A LATER CLOSE OF ITS OWN WINDOW THAT
/// WAS UNDER WAY WHEN IT OPENED** (BASE-COV-API-AS2; D-3341).
/// Extremes: 11 (8 single, 2 combined, 1 out of the box): (1) a close written
/// before the open never answers it; (2) a later close whose window starts a
/// day later does not; (3) a later close whose window ends a day earlier does
/// not; (4) a later close of the same window that began after the open does
/// not; (5) the later close of the same window under way at the open answers
/// it; (6) the answered open is counted opened and closed and listed nowhere
/// else; (7) every close of the feed and form that offered bars is a candidate,
/// oldest first; (8) the close under way at the last write is named the writer.
/// Combined: (9) four closes that do not answer AND one that does, in one
/// journal: the open is answered by the fifth alone; (10) a close written
/// before its open AND under way when it opened: the open stays unclosed and
/// keeps that close from being named the writer. Out of the box: (11) the other
/// form's close of the very same window, under way at the opening: never its
/// answer.
#[test]
#[allow(
    clippy::too_many_lines,
    reason = "one test walks every pairing its Extremes: line names, in order"
)]
fn an_open_is_answered_only_by_a_later_close_of_its_window_under_way_when_it_opened() {
    use audit::{Outcome, Scope};
    let span = Some(("2015-01-01", "2020-12-31"));
    let opened = || {
        audit::Record::opened(
            Scope::Spot,
            at(T0 + 1),
            KITE,
            window("2015-01-01", "2020-12-31"),
            3,
            0,
        )
    };
    let candidates = |body: &Value| -> Vec<u64> {
        body["reconstruction"]["candidates"]
            .as_array()
            .unwrap()
            .iter()
            .map(|one| one["ordinal"].as_u64().unwrap())
            .collect()
    };
    // (1)-(9): four closes that do not answer the open, then the one that does.
    let root = root_of("as2-g2-pairing");
    let journal = audit::Journal::at(&root);
    for record in [
        run(Scope::Spot, Outcome::Failed, T0 - 5_000, 10, KITE, span, 9),
        opened(),
        run(
            Scope::Spot,
            Outcome::Failed,
            T0 - 4_000,
            10,
            KITE,
            Some(("2015-01-02", "2020-12-31")),
            9,
        ),
        run(
            Scope::Spot,
            Outcome::Failed,
            T0 - 3_000,
            10,
            KITE,
            Some(("2015-01-01", "2020-12-30")),
            9,
        ),
        run(Scope::Spot, Outcome::Failed, T0 + 100, 10, KITE, span, 9),
        run(Scope::Spot, Outcome::Failed, T0, 1_000, KITE, span, 9),
    ] {
        journal.append(&record).unwrap();
    }
    zerodha_days(&root, "2015-01", &["2015-01-01"], 1, T0 + 700);
    let got = get(&root, ZERODHA_2015);
    assert_eq!(got.status, StatusCode::OK, "{}", got.body);
    let rebuilt = &got.body["reconstruction"];
    assert_eq!(candidates(&got.body), [0, 2, 3, 4, 5], "{rebuilt}");
    assert_eq!(rebuilt["opened_never_closed"], serde_json::json!([]));
    assert_eq!(rebuilt["opened_and_closed"], 1);
    assert_eq!(rebuilt["last_write"]["inside_runs"], serde_json::json!([5]));
    assert_eq!(rebuilt["attributed"]["run"], 5);
    // (10): a close written before its open, though under way when it opened.
    let root = root_of("as2-g2-pairing-before");
    let journal = audit::Journal::at(&root);
    for record in [
        run(Scope::Spot, Outcome::Failed, T0, 1_000, KITE, span, 9),
        opened(),
    ] {
        journal.append(&record).unwrap();
    }
    zerodha_days(&root, "2015-01", &["2015-01-01"], 1, T0 + 700);
    let got = get(&root, ZERODHA_2015);
    let rebuilt = &got.body["reconstruction"];
    let unclosed = rebuilt["opened_never_closed"].as_array().unwrap();
    assert_eq!(unclosed.len(), 1, "{}", got.body);
    assert_eq!(unclosed[0]["ordinal"], 1);
    assert_eq!(rebuilt["opened_and_closed"], 0);
    assert_eq!(rebuilt["last_write"]["inside_runs"], serde_json::json!([0]));
    assert!(rebuilt["attributed"].is_null(), "{rebuilt}");
    // (11): the other form's close of the same window.
    let root = root_of("as2-g2-pairing-other-form");
    let journal = audit::Journal::at(&root);
    for record in [
        opened(),
        run(Scope::Fno, Outcome::Failed, T0, 1_000, KITE, span, 9),
    ] {
        journal.append(&record).unwrap();
    }
    zerodha_days(&root, "2015-01", &["2015-01-01"], 1, T0 + 700);
    let got = get(&root, ZERODHA_2015);
    let rebuilt = &got.body["reconstruction"];
    assert_eq!(
        rebuilt["opened_never_closed"][0]["ordinal"], 0,
        "{}",
        got.body
    );
    assert_eq!(rebuilt["excluded"]["other_scope"], 1);
    assert!(candidates(&got.body).is_empty(), "{rebuilt}");
}

/// **THE OPERATOR'S WINDOW LABELS A WHOLE MONTH, INSIDE IT OR OUTSIDE IT ON
/// EITHER SIDE, AND A FEED WITH NO WINDOW CARRIES NONE** (BASE-COV-API-AS2;
/// D-3338), over `window_json` directly.
/// Extremes: 11 (8 single, 2 combined, 1 out of the box): (1) 1970-01, the
/// first month a store path can name, is outside; (2) 2017-12, the last month
/// before the window, is outside; (3) 2018-01, its first month, is inside with
/// no label; (4) 2026-09, its last month, is inside; (5) 2026-10, the first
/// month after it, is outside; (6) 9999-12, the last month a store path can
/// name, is outside; (7) a GDFL month carries no window; (8) a Dhan month
/// carries none either. Combined: (9) 2026-10 AND `t` on its first day:
/// labelled by the whole month; (10) 2017-12 AND `t` on its last day: outside.
/// Out of the box: (11) a futures contract's month: the feed's window holds for
/// every segment.
#[test]
fn the_operators_window_labels_whole_months_on_either_side_of_it() {
    let zerodha = |month: &str, rest: &str| {
        window_json(&ask(&format!(
            "feed=zerodha&instrument=NSE-INDEX-NIFTY&timeframe=1day&month={month}{rest}"
        )))
    };
    let after = format!("&t={}", ist_midnight("2026-10-01"));
    let before = format!("&t={}", ist_midnight("2017-12-31"));
    // (1)-(6), (9), (10)
    for (month, rest, inside) in [
        ("1970-01", "", false),
        ("2017-12", "", false),
        ("2018-01", "", true),
        ("2026-09", "", true),
        ("2026-10", "", false),
        ("9999-12", "", false),
        ("2026-10", after.as_str(), false),
        ("2017-12", before.as_str(), false),
    ] {
        let span = zerodha(month, rest);
        assert_eq!(span["inside"], inside, "{month}{rest}: {span}");
        let label = if inside {
            Value::Null
        } else {
            Value::from(OUTSIDE_WINDOW)
        };
        assert_eq!(span["label"], label, "{month}{rest}: {span}");
        assert_eq!(span["from"], "2018-01-01");
        assert_eq!(span["to"], "2026-09-30");
        assert_eq!(span["standing"], "operator");
        assert!(
            span["source"].as_str().unwrap().contains("D-3338"),
            "{span}"
        );
    }
    // (11)
    let future = window_json(&ask(
        "feed=zerodha&instrument=NSE-FNO-NIFTY-2026-10-29-FUT&timeframe=1day&month=2026-10",
    ));
    assert_eq!(future["label"], OUTSIDE_WINDOW, "{future}");
    // (7), (8)
    assert!(window_json(&ask(GDFL_NIFTY)).is_null());
    assert!(
        window_json(&ask(
            "feed=dhan&instrument=NSE-INDEX-NIFTY&timeframe=1min&month=2026-10"
        ))
        .is_null()
    );
}

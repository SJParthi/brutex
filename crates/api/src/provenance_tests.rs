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

/// **A MONTH BEING WRITTEN IS A 409 THAT SAYS SO**: a landing holds the
/// month's lock while it appends and the store refuses a reader meanwhile,
/// which is neither an absent month nor a damaged one.
#[test]
fn a_month_being_written_is_a_409_naming_the_writer() {
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
    assert_eq!(held.status, StatusCode::CONFLICT, "{}", held.body);
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

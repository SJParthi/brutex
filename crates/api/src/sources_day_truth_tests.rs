//! AG-04: `/sources/day.json` says what the store can prove about a day.
//! The auction sidecar's version and whether a checksum vouched for it; a
//! refused sidecar named by file and ordinal; a record outside its month
//! named; a print also stored as a one-second bar (bars-1); an eligible
//! share day's seconds inside the auction band (bars-1); and the restart
//! carry on a day the calendar names a restart (stocks-2). Labels only:
//! nothing is subtracted.
//!
//! Every value is invented: the prices cannot be mistaken for real ones and
//! no vendor row is quoted.

#![expect(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail"
)]

use std::fs;
use std::path::PathBuf;

use brutex_core::instrument::{Exchange, InstrumentKey};
use pull::gdfl_cm::auction;

use super::*;

/// A fresh store root, its own per test.
fn root(name: &str) -> PathBuf {
    let root = crate::scratch::path(&format!("ag04-{name}"));
    fs::create_dir_all(&root).unwrap();
    root
}

fn day(y: u16, m: u8, d: u8) -> Day {
    Day::new(y, m, d).unwrap()
}

/// The UTC microsecond of `h:m:s` IST on `on`.
fn t(on: Day, h: i64, m: i64, s: i64) -> i64 {
    (i64::from(on.days_from_epoch()) * 86_400 + h * 3_600 + m * 60 + s
        - pull::session::IST_OFFSET_SECS)
        * 1_000_000
}

/// A flat one-second bar at `price` with `volume`.
fn second(ts: i64, price: i64, volume: i64) -> Bar {
    Bar {
        ts_micros: ts,
        open: price,
        high: price,
        low: price,
        close: price,
        volume,
        open_interest: OI_NULL,
    }
}

/// Appends `rows` (ascending, one month) to GDFL's CASH seconds of `symbol`.
fn seconds(root: &Path, symbol: &str, rows: &[Bar]) {
    let first = rows.first().unwrap().ts_micros;
    let civil = Day::from_days(u32::try_from(day_number(first)).unwrap()).unwrap();
    let month = YearMonth::new(civil.year(), civil.month()).unwrap();
    let path = StorePath::new(PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment: "CASH",
        symbol,
        contract: None,
        timeframe: Timeframe::SECOND_1,
        month,
        file: FileKind::Bars,
    })
    .unwrap();
    let hash = brutex_core::universe::fnv1a(symbol).to_le_bytes();
    let id = u32::from_le_bytes(hash[..4].try_into().unwrap());
    let mut file = BarFile::open_or_create(root, path, id).unwrap();
    file.append(rows).unwrap();
}

fn key(symbol: &str) -> InstrumentKey {
    InstrumentKey::cash(Exchange::Nse, symbol).unwrap()
}

fn print(sod: u32, price: i64, quantity: u64) -> auction::Print {
    auction::Print {
        sod,
        price,
        quantity,
        rows: 1,
        line: 2,
    }
}

const fn at(h: u32, m: u32, s: u32) -> u32 {
    h * 3_600 + m * 60 + s
}

/// A gzip of `text` in one stored block, as an NSE master arrives.
fn stored_gzip(text: &[u8]) -> Vec<u8> {
    let mut crc = !0u32;
    for &byte in text {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    let len = u16::try_from(text.len()).unwrap();
    let mut out = vec![0x1f, 0x8b, 8, 0, 0, 0, 0, 0, 0, 0xff, 1];
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(&(!len).to_le_bytes());
    out.extend_from_slice(text);
    out.extend_from_slice(&(!crc).to_le_bytes());
    out.extend_from_slice(&u32::try_from(text.len()).unwrap().to_le_bytes());
    out
}

/// Installs `on`'s dated NSE master flagging `symbol` with `flag`.
fn master(root: &Path, on: Day, symbol: &str, flag: u8) {
    let masters = root.join("session-masters");
    fs::create_dir_all(&masters).unwrap();
    let isin = brutex_core::universe::nse_isin(symbol).expect("an F&O share");
    let csv = format!(
        "FinInstrmId,TckrSymb,SctySrs,ISIN,ElgbltyClsgAuctnSsn\n7,{symbol},EQ,{},{flag}\n",
        isin.as_str()
    );
    pull::cash_session_cache::install(&masters, on, &stored_gzip(csv.as_bytes())).unwrap();
}

/// `/sources/day.json`'s answer for one CASH day of `symbol`.
fn day_of(root: &Path, symbol: &str, on: Day) -> Value {
    let asked = DayAsk::parse(&format!(
        "exchange=NSE&segment=CASH&symbol={symbol}&from={on}"
    ))
    .unwrap();
    let body = day_table(root, &asked).unwrap();
    assert_eq!(body["days"].as_array().unwrap().len(), 1, "{body:#}");
    body
}

fn flags(row: &Value) -> Vec<String> {
    row["flags"]
        .as_array()
        .unwrap()
        .iter()
        .map(|flag| flag.as_str().unwrap().to_owned())
        .collect()
}

#[test]
fn sources_day_names_a_print_also_in_a_one_second_bar() {
    let root = root("doubled");
    let on = day(2026, 9, 22);
    // The match at 15:29:06 is also a stored second (a definition-2 month);
    // the straggler at 15:29:07 is not; 15:29:08 is a second at the print's
    // price with another volume, which is not the print.
    seconds(
        &root,
        "SYNTHDBL",
        &[
            second(t(on, 9, 15, 0), 444_600, 900),
            second(t(on, 15, 29, 6), 444_665, 8_473),
            second(t(on, 15, 29, 8), 444_665, 1),
        ],
    );
    auction::append(
        &root,
        &key("SYNTHDBL"),
        on,
        &[
            print(at(15, 29, 6), 444_665, 8_473),
            print(at(15, 29, 7), 444_665, 12),
            print(at(15, 29, 8), 444_665, 13),
        ],
    )
    .unwrap();
    let body = day_of(&root, "SYNTHDBL", on);
    let row = &body["days"][0];
    let auc = &row["auction"];
    assert_eq!(auc["state"], "kept_apart", "{auc:#}");
    assert_eq!(auc["version"], 2);
    assert_eq!(auc["checked"], true);
    assert_eq!(auc["unchecked_why"], Value::Null);
    assert_eq!(auc["also_in_bars"]["count"], 1);
    assert_eq!(auc["also_in_bars"]["seconds"], json!(["15:29:06"]));
    let doubled: Vec<bool> = auc["prints"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["also_in_bars"].as_bool().unwrap())
        .collect();
    assert_eq!(doubled, vec![true, false, false]);
    assert_eq!(auc["prints"][0]["ordinal"], 0);
    assert_eq!(auc["prints"][0]["in_band"], true);
    assert!(flags(row).contains(&"auction_print_also_in_bars".to_owned()));
    assert_eq!(body["summary"]["auction_print_also_in_bars"], 1);
    // NOT SUBTRACTED: GDFL's day sum still holds the doubled print.
    assert_eq!(row["volume"]["gdfl_sum"], 900 + 8_473 + 1);
}

#[test]
fn a_version_1_sidecar_is_served_and_labelled_unchecked() {
    let root = root("v1");
    let on = day(2026, 9, 22);
    seconds(&root, "SYNTHVONE", &[second(t(on, 9, 15, 0), 333_300, 5)]);
    let path = auction::path(&root, &key("SYNTHVONE"), on).unwrap();
    let mut bytes = auction::header_of(auction::VERSION_1).to_vec();
    bytes.extend_from_slice(&auction::Record::of(on, &print(at(15, 29, 1), 333_333, 7)).to_bytes());
    // A record whose day the file's month does not hold: after the last,
    // in order, so only the month can name it.
    let outside = auction::Record {
        t_micros: t(day(2026, 10, 1), 15, 29, 1),
        price_paisa: 333_333,
        quantity: 7,
        rows: 1,
    };
    bytes.extend_from_slice(&outside.to_bytes());
    fs::write(&path, &bytes).unwrap();
    let body = day_of(&root, "SYNTHVONE", on);
    let auc = &body["days"][0]["auction"];
    assert_eq!(auc["state"], "kept_apart", "{auc:#}");
    assert_eq!(auc["version"], 1);
    assert_eq!(auc["checked"], false);
    assert!(
        auc["unchecked_why"]
            .as_str()
            .unwrap()
            .contains("version 1 carries no checksum")
    );
    assert_eq!(auc["prints"].as_array().unwrap().len(), 1);
    assert_eq!(
        auc["outside_month"],
        json!([{ "ordinal": 1, "at": "2026-10-01 15:29:01" }]),
        "named, never filtered away"
    );
}

#[test]
fn a_refused_sidecar_names_its_file_and_ordinal() {
    let root = root("refused");
    let on = day(2026, 9, 22);
    seconds(&root, "SYNTHBAD", &[second(t(on, 9, 15, 0), 333_300, 5)]);
    auction::append(
        &root,
        &key("SYNTHBAD"),
        on,
        &[
            print(at(15, 29, 1), 333_333, 7),
            print(at(15, 29, 2), 333_333, 9),
        ],
    )
    .unwrap();
    let path = auction::path(&root, &key("SYNTHBAD"), on).unwrap();
    let mut bytes = fs::read(&path).unwrap();
    bytes[auction::HEADER_LEN + auction::RECORD_LEN + 8] ^= 0x04; // record 1's price
    fs::write(&path, &bytes).unwrap();
    let body = day_of(&root, "SYNTHBAD", on);
    let auc = &body["days"][0]["auction"];
    assert_eq!(auc["state"], "unreadable", "{auc:#}");
    assert_eq!(auc["file"], path.display().to_string());
    assert_eq!(auc["ordinal"], 1);
    assert!(auc["why"].as_str().unwrap().contains("checksum"), "{auc:#}");
    // Garbage at the path: refused by name, no ordinal.
    fs::write(&path, b"not a sidecar at all").unwrap();
    let body = day_of(&root, "SYNTHBAD", on);
    let auc = &body["days"][0]["auction"];
    assert_eq!(auc["state"], "unreadable");
    assert_eq!(auc["ordinal"], Value::Null);
    assert!(auc["why"].as_str().unwrap().contains("magic"), "{auc:#}");
}

#[test]
fn sources_day_reads_a_sidecar_filed_after_the_day_was_cached() {
    let root = root("late-sidecar");
    let on = day(2026, 9, 22);
    seconds(&root, "SYNTHLATE", &[second(t(on, 9, 15, 0), 333_300, 5)]);
    let before = day_of(&root, "SYNTHLATE", on);
    assert_eq!(before["days"][0]["auction"]["state"], "no_sidecar");
    auction::append(
        &root,
        &key("SYNTHLATE"),
        on,
        &[print(at(15, 29, 1), 333_333, 7)],
    )
    .unwrap();
    let after = day_of(&root, "SYNTHLATE", on);
    assert_eq!(
        after["days"][0]["auction"]["state"], "kept_apart",
        "the sidecar's stamp is in the day's key"
    );
}

#[test]
fn sources_day_counts_the_auction_band_bars_of_an_eligible_day() {
    let on = day(2026, 9, 22);
    let rows = [
        second(t(on, 9, 15, 0), 931_300, 100),
        second(t(on, 15, 14, 59), 931_300, 1),
        second(t(on, 15, 15, 0), 931_310, 20),
        second(t(on, 15, 28, 7), 931_313, 4_000),
        second(t(on, 15, 34, 59), 931_313, 300),
        second(t(on, 15, 35, 0), 931_313, 7),
    ];
    // ELIGIBLE: three seconds inside [15:15:00, 15:35:00), labelled.
    let root = root("band-eligible");
    seconds(&root, "RELIANCE", &rows);
    master(&root, on, "RELIANCE", 1);
    let body = day_of(&root, "RELIANCE", on);
    let row = &body["days"][0];
    let band = &row["auction"]["auction_band_bars"];
    assert_eq!(row["auction"]["state"], "no_sidecar", "{row:#}");
    assert_eq!(band["count"], 3, "{band:#}");
    assert_eq!(band["volume"], 20 + 4_000 + 300);
    assert_eq!(
        band["band"],
        json!({ "from": "15:15:00", "to": "15:35:00" })
    );
    assert_eq!(band["eligibility"], "eligible");
    assert!(band["says"].as_str().unwrap().contains("never subtracted"));
    assert!(flags(row).contains(&"auction_band_bars".to_owned()));
    assert_eq!(body["summary"]["auction_band_bars"], 1);
    assert_eq!(row["volume"]["gdfl_sum"], 100 + 1 + 20 + 4_000 + 300 + 7);
    // NOT ELIGIBLE: the same seconds are continuous trades; no flag.
    let root = self::root("band-ineligible");
    seconds(&root, "RELIANCE", &rows);
    master(&root, on, "RELIANCE", 0);
    let row = &day_of(&root, "RELIANCE", on)["days"][0];
    assert_eq!(
        row["auction"]["auction_band_bars"]["eligibility"],
        "not_eligible"
    );
    assert!(!flags(row).contains(&"auction_band_bars".to_owned()));
    // NO MASTER: unverified with the reason, never read as not eligible.
    let root = self::root("band-unverified");
    seconds(&root, "RELIANCE", &rows);
    let row = &day_of(&root, "RELIANCE", on)["days"][0];
    let band = &row["auction"]["auction_band_bars"];
    assert_eq!(band["eligibility"], "unverified");
    assert!(
        band["why"].as_str().unwrap().contains("UNVERIFIED"),
        "{band:#}"
    );
    assert_eq!(band["count"], 3, "still counted");
    assert!(!flags(row).contains(&"auction_band_bars".to_owned()));
    // AND THE MASTER LANDS: the next request reads it (AF-03's rule), with
    // the day facts themselves served from the memo.
    master(&root, on, "RELIANCE", 1);
    let body = day_of(&root, "RELIANCE", on);
    assert_eq!(body["summary"]["days_from_cache"], 1);
    assert_eq!(
        body["days"][0]["auction"]["auction_band_bars"]["eligibility"],
        "eligible"
    );
    // A SHARE WITH NO ISIN: unverified, named.
    let root = self::root("band-no-isin");
    seconds(&root, "SYNTHNOISIN", &rows);
    let band = &day_of(&root, "SYNTHNOISIN", on)["days"][0]["auction"]["auction_band_bars"];
    assert!(
        band["why"].as_str().unwrap().contains("no ISIN"),
        "{band:#}"
    );
}

/// An auction-era share day whose stored seconds all sit outside the band
/// (an eligible share imported after D-3320's diversion) has nothing to
/// label, so its eligibility is not read and no master is loaded for it:
/// the first touch of such a day never pays L-AG04-02's master load.
#[test]
fn a_day_with_no_second_inside_the_band_loads_no_master() {
    let on = day(2026, 9, 23);
    let rows = [
        second(t(on, 9, 15, 0), 931_300, 100),
        second(t(on, 15, 14, 59), 931_300, 1),
        second(t(on, 15, 35, 0), 931_313, 7),
    ];
    // NO MASTER ON DISK: reading it would say `unverified`; not read says so.
    let root = root("band-empty-no-master");
    seconds(&root, "RELIANCE", &rows);
    let band = &day_of(&root, "RELIANCE", on)["days"][0]["auction"]["auction_band_bars"];
    assert_eq!(band["count"], 0, "{band:#}");
    assert_eq!(band["volume"], 0);
    assert_eq!(band["eligibility"], "not_read");
    assert_eq!(band["why"], Value::Null);
    assert!(
        band["says"]
            .as_str()
            .unwrap()
            .contains("no master was loaded")
    );
    // A MASTER ON DISK: still not loaded, and the day carries no band flag.
    let root = self::root("band-empty-master");
    seconds(&root, "RELIANCE", &rows);
    master(&root, on, "RELIANCE", 1);
    let body = day_of(&root, "RELIANCE", on);
    let row = &body["days"][0];
    assert_eq!(
        row["auction"]["auction_band_bars"]["eligibility"],
        "not_read"
    );
    assert!(!flags(row).contains(&"auction_band_bars".to_owned()));
    assert_eq!(body["summary"]["auction_band_bars"], 0);
    let memo = (root.join("session-masters"), on.days_from_epoch());
    assert!(MASTERS.get(&memo).is_none(), "no master was loaded");
    // ONE SECOND INSIDE THE BAND: now the master is read, and kept.
    let root = self::root("band-one-master");
    let mut one = rows.to_vec();
    one.insert(2, second(t(on, 15, 15, 0), 931_310, 2));
    seconds(&root, "RELIANCE", &one);
    master(&root, on, "RELIANCE", 1);
    let band = &day_of(&root, "RELIANCE", on)["days"][0]["auction"]["auction_band_bars"];
    assert_eq!(band["count"], 1, "{band:#}");
    assert_eq!(band["eligibility"], "eligible");
    let memo = (root.join("session-masters"), on.days_from_epoch());
    assert!(MASTERS.get(&memo).is_some(), "the day's master is kept");
}

#[test]
fn a_special_session_and_a_day_before_the_auction_have_no_band() {
    // 2026-11-08: a Muhurat of no recorded length (NSE/CMTR/71775). The
    // master says eligible; the calendar holds no closing-auction phase.
    let muhurat = day(2026, 11, 8);
    let root = root("band-muhurat");
    seconds(
        &root,
        "RELIANCE",
        &[second(t(muhurat, 15, 20, 0), 931_300, 3)],
    );
    master(&root, muhurat, "RELIANCE", 1);
    let band = &day_of(&root, "RELIANCE", muhurat)["days"][0]["auction"]["auction_band_bars"];
    assert_eq!(band["eligibility"], "no_closing_auction", "{band:#}");
    // A DAY GDFL HOLDS NO SECOND OF (here 2026-11-09, the month's file
    // holding only 2026-11-08): no band to label and no master asked.
    let row = &day_of(&root, "RELIANCE", day(2026, 11, 9))["days"][0];
    assert_eq!(row["auction"]["state"], "no_sidecar", "{row:#}");
    assert_eq!(row["auction"]["auction_band_bars"], Value::Null);
    // 2026-07-31: before the auction began, no auction fact and no band.
    let before = day(2026, 7, 31);
    let root = self::root("band-before");
    seconds(
        &root,
        "RELIANCE",
        &[second(t(before, 15, 20, 0), 931_300, 3)],
    );
    let row = &day_of(&root, "RELIANCE", before)["days"][0];
    assert_eq!(row["auction"], Value::Null);
}

/// A halt day's share: `pre` seconds before the halt and `first` the first
/// second after the cash market reopens at 10:57:00 (2020-03-23).
fn halt_day(name: &str, pre: &[i64], first: i64) -> Value {
    let on = day(2020, 3, 23);
    let root = root(name);
    let mut rows: Vec<Bar> = pre
        .iter()
        .enumerate()
        .map(|(n, volume)| {
            let n = i64::try_from(n).unwrap();
            second(t(on, 9, 15, n), 222_200, *volume)
        })
        .collect();
    rows.push(second(t(on, 10, 57, 0), 222_250, first));
    rows.push(second(t(on, 11, 0, 0), 222_260, 10));
    seconds(&root, "SYNTHHALT", &rows);
    day_of(&root, "SYNTHHALT", on)
}

#[test]
fn sources_day_names_the_restart_carry_on_a_halt_day() {
    // P = 1,000 before the halt, F = 1,250 at the reopen: labelled.
    let body = halt_day("carry", &[400, 600], 1_250);
    let row = &body["days"][0];
    let restart = &row["restart"];
    assert_eq!(restart["source"], "market-wide halt", "{restart:#}");
    assert_eq!(restart["suspected"], true);
    assert_eq!(restart["reopen_at"], "10:57:00");
    assert_eq!(restart["pre_restart_volume"], 1_000);
    assert_eq!(restart["first_bar_at"], "10:57:00");
    assert_eq!(restart["first_bar_volume"], 1_250);
    assert_eq!(restart["carry"], true);
    assert!(flags(row).contains(&"restart_carry".to_owned()));
    assert_eq!(row["volume"]["gdfl_sum_verified"], false);
    assert!(
        row["volume"]["gdfl_sum_why"]
            .as_str()
            .unwrap()
            .starts_with("UNVERIFIED")
    );
    assert_eq!(body["summary"]["restart_carry"], 1);
    // NOT SUBTRACTED.
    assert_eq!(row["volume"]["gdfl_sum"], 400 + 600 + 1_250 + 10);
}

#[test]
fn sources_day_names_the_restart_carry_on_a_drill_saturday() {
    // 2024-05-18: two windows, the second opening 11:30 (the calendar's own
    // table, never a day list here).
    let on = day(2024, 5, 18);
    let root = root("drill");
    seconds(
        &root,
        "SYNTHDRILL",
        &[
            second(t(on, 9, 15, 0), 222_200, 700),
            second(t(on, 9, 59, 59), 222_200, 300),
            second(t(on, 11, 30, 0), 222_210, 1_000),
            second(t(on, 11, 30, 1), 222_210, 5),
        ],
    );
    let row = &day_of(&root, "SYNTHDRILL", on)["days"][0];
    let restart = &row["restart"];
    assert_eq!(restart["source"], "disaster-recovery drill", "{restart:#}");
    assert_eq!(restart["reopen_at"], "11:30:00");
    assert_eq!(restart["pre_restart_volume"], 1_000);
    assert_eq!(restart["first_bar_volume"], 1_000, "F = P is a carry");
    assert_eq!(restart["carry"], true);
    assert!(flags(row).contains(&"restart_carry".to_owned()));
}

#[test]
fn an_ordinary_day_and_a_restart_day_with_a_small_first_print_carry_no_restart_label() {
    // A SMALL FIRST PRINT: F = 999 < P = 1,000.
    let row = &halt_day("small", &[400, 600], 999)["days"][0];
    assert_eq!(row["restart"]["carry"], false, "{row:#}");
    assert!(!flags(row).contains(&"restart_carry".to_owned()));
    assert_eq!(
        row["volume"]["gdfl_sum_verified"],
        Value::Null,
        "never true"
    );
    assert_eq!(row["volume"]["gdfl_sum_why"], Value::Null);
    // NO TRADE BEFORE THE HALT: P = 0, whatever F is.
    let row = &halt_day("no-pre", &[], 5_000)["days"][0];
    assert_eq!(row["restart"]["pre_restart_volume"], 0);
    assert_eq!(row["restart"]["carry"], false);
    assert!(!flags(row).contains(&"restart_carry".to_owned()));
    // NO TRADE AFTER IT: no first bar, no carry.
    let on = day(2020, 3, 23);
    let root = root("no-after");
    seconds(&root, "SYNTHHALT", &[second(t(on, 9, 15, 0), 222_200, 50)]);
    let row = &day_of(&root, "SYNTHHALT", on)["days"][0];
    assert_eq!(row["restart"]["first_bar_volume"], Value::Null);
    assert_eq!(row["restart"]["carry"], false);
    // A RESTART DAY THE STORE HOLDS NO SECOND OF (the drill Saturdays on the
    // real store): not measured, never "no carry".
    let drill = day(2024, 3, 2);
    let root = self::root("not-measured");
    seconds(
        &root,
        "SYNTHHALT",
        &[second(t(day(2024, 3, 1), 9, 15, 0), 222_200, 50)],
    );
    let row = &day_of(&root, "SYNTHHALT", drill)["days"][0];
    assert_eq!(row["restart"]["reopen_at"], "11:30:00", "{row:#}");
    assert_eq!(row["restart"]["carry"], Value::Null);
    assert!(
        row["restart"]["says"]
            .as_str()
            .unwrap()
            .contains("not measured")
    );
    assert!(!flags(row).contains(&"restart_carry".to_owned()));
    // AN ORDINARY DAY: no restart at all, even with a first-print shape.
    let ordinary = day(2020, 3, 20);
    let root = self::root("ordinary");
    seconds(
        &root,
        "SYNTHHALT",
        &[
            second(t(ordinary, 9, 15, 0), 222_200, 10),
            second(t(ordinary, 10, 57, 0), 222_200, 5_000),
        ],
    );
    let row = &day_of(&root, "SYNTHHALT", ordinary)["days"][0];
    assert_eq!(row["restart"], Value::Null);
    assert!(!flags(row).contains(&"restart_carry".to_owned()));
    assert_eq!(
        row["volume"]["gdfl_sum_verified"],
        Value::Null,
        "never true"
    );
}

/// The calendar's restart table is read, not copied: every recorded halt
/// and every special session of two windows names a reopen, and nothing
/// else does.
#[test]
fn every_restart_comes_from_the_calendar_of_record() {
    for halt in pull::calendar::HALTS {
        let on = Day::from_days(u32::try_from(halt.day).unwrap()).unwrap();
        let (reopen, source, _) = restart_on(on).unwrap();
        assert_eq!(reopen, halt.cash.resume);
        assert_eq!(source, "market-wide halt");
    }
    for special in pull::calendar::SPECIAL_SESSIONS {
        let on = Day::from_days(u32::try_from(special.day).unwrap()).unwrap();
        let two = special.session.count >= 2;
        let halted = pull::calendar::halt_on(special.day).is_some();
        assert_eq!(restart_on(on).is_some(), two || halted, "{on}");
    }
    assert_eq!(restart_on(day(2020, 3, 20)), None);
}

/// A RENAMED SHARE: the sidecar is keyed by the same symbol as the seconds
/// it sits beside, so the old name's prints are served under the old name
/// and never borrowed by the new one, whose month has none (`no_sidecar`,
/// said, not an empty `kept_apart`). Lineage across a rename is the
/// lineage owner's; this route never guesses it.
#[test]
fn a_renamed_share_never_borrows_the_old_names_sidecar() {
    let root = root("renamed");
    let on = day(2026, 9, 22);
    seconds(&root, "SYNTHOLD", &[second(t(on, 9, 15, 0), 777_700, 40)]);
    seconds(&root, "SYNTHNEW", &[second(t(on, 9, 15, 0), 777_700, 50)]);
    auction::append(
        &root,
        &key("SYNTHOLD"),
        on,
        &[print(at(15, 29, 6), 777_765, 61)],
    )
    .unwrap();
    let old = &day_of(&root, "SYNTHOLD", on)["days"][0]["auction"];
    assert_eq!(old["state"], "kept_apart", "{old:#}");
    assert_eq!(old["prints"][0]["quantity"], 61);
    let new = &day_of(&root, "SYNTHNEW", on)["days"][0]["auction"];
    assert_eq!(new["state"], "no_sidecar", "{new:#}");
    assert_eq!(new["prints"], Value::Null);
}

/// NOTHING HERE VERIFIES GDFL'S SUM: the volume never says `verified: true`.
/// A carried cash day is `false` with its reason; a restart day of a segment
/// whose carry is not measured (an option or an index on a halt day) says
/// so; an ordinary day of any segment says nothing.
#[test]
fn the_gdfl_sum_is_never_said_verified_and_a_derivative_restart_day_is_named() {
    let halt = day(2020, 3, 23);
    let drill = day(2024, 5, 18);
    let ordinary = day(2020, 3, 20);
    assert!(
        gdfl_sum_why(true, Segment::Cash, halt)
            .unwrap()
            .starts_with("UNVERIFIED")
    );
    assert_eq!(gdfl_sum_why(false, Segment::Cash, halt), None);
    for segment in [Segment::Fno, Segment::Index] {
        for on in [halt, drill] {
            assert!(
                gdfl_sum_why(false, segment, on)
                    .unwrap()
                    .contains("cash segment only"),
                "{segment:?} {on}"
            );
        }
        assert_eq!(gdfl_sum_why(false, segment, ordinary), None);
    }
}

/// **A SIDECAR'S RECORDS OF OTHER DAYS STAY THEIRS, AND ONLY THOSE OUTSIDE
/// ITS MONTH ARE NAMED** (BASE-COV-API-AS2). The sidecar tests held one day,
/// or one record after the month. Extremes: (1) a record one second before the
/// file's month is named outside it; (2) one at the month's first instant is
/// inside it, another day's, neither a print nor named; (3) one on the day
/// before is skipped; (4) one at the day's first instant is the day's print,
/// outside the band; (5) one at the day's last second is the day's print; (6)
/// one at a stored second whose close is another price is not also in the
/// bars; (7) one on the day after is skipped; (8) one at the month's last
/// second is inside it, skipped, not named; (9) one at the next month's first
/// instant is named outside. Combined: (10) a stored second at the print's own
/// instant with its volume equal to the quantity but another close: not
/// counted twice, the price decides; (11) the day's last-second print is the
/// day's AND outside the band. Out of the box: (12) a version-1 file carrying
/// records outside its month at BOTH ends at once, still in day order: both
/// named, with ordinals 0 and 8.
#[test]
fn a_sidecars_records_of_other_days_stay_theirs_and_only_those_outside_its_month_are_named() {
    let root = root("as2-other-days");
    let on = day(2026, 9, 22);
    seconds(
        &root,
        "SYNTHSPAN",
        &[
            second(t(on, 9, 15, 0), 555_000, 10),
            second(t(on, 15, 29, 3), 555_500, 5),
        ],
    );
    let record = |when: Day, sod: u32, quantity: u64| {
        auction::Record::of(when, &print(sod, 555_555, quantity))
    };
    let records = [
        auction::Record {
            t_micros: t(day(2026, 9, 1), 0, 0, 0) - 1_000_000,
            price_paisa: 555_555,
            quantity: 1,
            rows: 1,
        },
        record(day(2026, 9, 1), at(0, 0, 0), 2),
        record(day(2026, 9, 21), at(15, 29, 2), 3),
        record(on, at(0, 0, 0), 4),
        record(on, at(15, 29, 3), 5),
        record(on, at(23, 59, 59), 6),
        record(day(2026, 9, 23), at(15, 29, 4), 7),
        record(day(2026, 9, 30), at(23, 59, 59), 8),
        record(day(2026, 10, 1), at(0, 0, 0), 9),
    ];
    let mut bytes = auction::header_of(auction::VERSION_1).to_vec();
    for kept in &records {
        bytes.extend_from_slice(&kept.to_bytes());
    }
    fs::write(auction::path(&root, &key("SYNTHSPAN"), on).unwrap(), &bytes).unwrap();
    let body = day_of(&root, "SYNTHSPAN", on);
    let row = &body["days"][0];
    let auc = &row["auction"];
    assert_eq!(auc["state"], "kept_apart", "{auc:#}");
    assert_eq!(auc["version"], 1);
    let placed: Vec<(u64, &str, u64, bool, bool)> = auc["prints"]
        .as_array()
        .unwrap()
        .iter()
        .map(|kept| {
            (
                kept["ordinal"].as_u64().unwrap(),
                kept["at"].as_str().unwrap(),
                kept["quantity"].as_u64().unwrap(),
                kept["in_band"].as_bool().unwrap(),
                kept["also_in_bars"].as_bool().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        placed,
        vec![
            (3, "00:00:00", 4, false, false),
            (4, "15:29:03", 5, true, false),
            (5, "23:59:59", 6, false, false),
        ]
    );
    assert_eq!(auc["also_in_bars"]["count"], 0);
    assert_eq!(
        auc["outside_month"],
        json!([
            { "ordinal": 0, "at": "2026-08-31 23:59:59" },
            { "ordinal": 8, "at": "2026-10-01 00:00:00" },
        ])
    );
    assert!(!flags(row).contains(&"auction_print_also_in_bars".to_owned()));
    assert_eq!(body["summary"]["auction_print_also_in_bars"], 0);
}

/// **A MASTER THAT LOADED BUT NAMES ANOTHER SHARE IS UNVERIFIED, AND ITS
/// ANSWER COMES BACK FROM MEMORY** (BASE-COV-API-AS2). Every master the band
/// tests installed named the share asked, and none was asked twice. Extremes:
/// (1) a dated master that loaded but lists another share: `unverified`,
/// never `not_eligible`; (2) the reason says UNVERIFIED and names the share
/// asked; (3) the band's seconds are still counted, one second, volume 20;
/// (4) no `auction_band_bars` flag, and the page counts none; (5) a master that
/// loaded is kept, whatever it said of this share; (6) with the masters folder
/// deleted, the next request answers from the kept master, word for word;
/// (7) and the day's facts come from the day memo; (8) a master of the day
/// before is not this day's: `unverified` with the day's own load refusal,
/// and nothing is kept for the day. Combined: (9) a kept master AND its folder
/// gone AND the day's facts cached: the same answer with nothing read from
/// disk; (10) the eve's master on disk AND the day asked: never borrowed.
/// Out of the box: (11) the whole `session-masters` folder deleted under a
/// reader that already loaded from it.
#[test]
fn a_loaded_master_that_names_another_share_is_unverified_and_answers_again_from_memory() {
    let on = day(2026, 9, 22);
    let rows = [
        second(t(on, 9, 15, 0), 931_300, 100),
        second(t(on, 15, 15, 0), 931_310, 20),
    ];
    let root = root("as2-master-other-share");
    seconds(&root, "RELIANCE", &rows);
    master(&root, on, "TCS", 1);
    // (1)-(5).
    let first = day_of(&root, "RELIANCE", on);
    let row = &first["days"][0];
    let band = &row["auction"]["auction_band_bars"];
    assert_eq!(band["eligibility"], "unverified", "{band:#}");
    let why = band["why"].as_str().unwrap().to_owned();
    assert!(
        why.contains("UNVERIFIED") && why.contains("no EQ mapping") && why.contains("RELIANCE"),
        "{why}"
    );
    assert_eq!((&band["count"], &band["volume"]), (&json!(1), &json!(20)));
    assert!(!flags(row).contains(&"auction_band_bars".to_owned()));
    assert_eq!(first["summary"]["auction_band_bars"], 0);
    let memo = (root.join("session-masters"), on.days_from_epoch());
    assert!(MASTERS.get(&memo).is_some(), "a master that loaded is kept");
    // (6), (7), (9), (11).
    fs::remove_dir_all(root.join("session-masters")).unwrap();
    let again = day_of(&root, "RELIANCE", on);
    assert_eq!(again["summary"]["days_from_cache"], 1);
    assert_eq!(
        again["days"][0]["auction"]["auction_band_bars"]["why"],
        json!(why),
        "read from the kept master, never from the disk"
    );
    // (8), (10).
    let eve = self::root("as2-master-eve");
    seconds(&eve, "RELIANCE", &rows);
    master(&eve, day(2026, 9, 21), "RELIANCE", 1);
    let band = &day_of(&eve, "RELIANCE", on)["days"][0]["auction"]["auction_band_bars"];
    assert_eq!(band["eligibility"], "unverified", "{band:#}");
    assert!(
        band["why"].as_str().unwrap().contains("2026-09-22"),
        "{band:#}"
    );
    assert!(
        MASTERS
            .get(&(eve.join("session-masters"), on.days_from_epoch()))
            .is_none()
    );
}

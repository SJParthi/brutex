#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::too_many_lines,
    clippy::type_complexity,
    clippy::many_single_char_names,
    reason = "a test that cannot panic cannot fail; a scenario test reads top to bottom"
)]
//! `cli tick-to-second-check` on invented archives and tick stores: every
//! status, every source, every bad-row reason, the fill-worsening measure,
//! the minute match, resumption, the fold and the index, against a store the
//! importer itself wrote (so every landing is the importer's own `.prv`
//! record). Every ticker (`ZZALPHA` ...) and every price (7777.00 ...) is
//! made up here; no vendor row is quoted (the repository is public). The one
//! real symbol, `RELIANCE` in the closing-auction test, is there for the ISIN
//! `core::universe` records (the decoder asks a share's auction eligibility
//! by it); its prices are invented ones it never traded at.

use std::collections::HashMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use brutex_core::instrument::{Exchange, InstrumentKey};
use brutex_core::vendor::Vendor;
use pull::gdfl_cm::{CmKind, HEADER_OPEN_INTEREST};
use pull::session::Day;
use store::file::BarFile;
use store::format::{Bar, OI_NULL};
use store::path::{FileKind, StorePath, Timeframe, YearMonth};

use super::index::{NotChecked, SecondChecks, SecondStatus, Source, Status};
use super::*;

/// A fresh, empty directory unique to `tag`.
fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "brutex-t2s-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn le16(out: &mut Vec<u8>, v: usize) {
    out.extend_from_slice(&u16::try_from(v).unwrap().to_le_bytes());
}

fn le32(out: &mut Vec<u8>, v: usize) {
    out.extend_from_slice(&u32::try_from(v).unwrap().to_le_bytes());
}

/// A plain zip of stored members, in order.
fn zip(members: &[(String, Vec<u8>)]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut central = Vec::new();
    for (name, data) in members {
        let offset = out.len();
        let crc = pull::gdfl_archive::crc32(data) as usize;
        le32(&mut out, 0x0403_4b50);
        le16(&mut out, 20);
        le16(&mut out, 0);
        le16(&mut out, 0);
        le32(&mut out, 0);
        le32(&mut out, crc);
        le32(&mut out, data.len());
        le32(&mut out, data.len());
        le16(&mut out, name.len());
        le16(&mut out, 0);
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(data);
        le32(&mut central, 0x0201_4b50);
        le16(&mut central, 20);
        le16(&mut central, 20);
        le16(&mut central, 0);
        le16(&mut central, 0);
        le32(&mut central, 0);
        le32(&mut central, crc);
        le32(&mut central, data.len());
        le32(&mut central, data.len());
        le16(&mut central, name.len());
        le16(&mut central, 0);
        le16(&mut central, 0);
        le16(&mut central, 0);
        le16(&mut central, 0);
        le32(&mut central, 0);
        le32(&mut central, offset);
        central.extend_from_slice(name.as_bytes());
    }
    let at = out.len();
    out.extend_from_slice(&central);
    le32(&mut out, 0x0605_4b50);
    le16(&mut out, 0);
    le16(&mut out, 0);
    le16(&mut out, members.len());
    le16(&mut out, members.len());
    le32(&mut out, central.len());
    le32(&mut out, at);
    le16(&mut out, 0);
    out
}

fn day(y: u16, m: u8, d: u8) -> Day {
    Day::new(y, m, d).unwrap()
}

fn hms(sod: u32) -> String {
    format!("{:02}:{:02}:{:02}", sod / 3_600, sod / 60 % 60, sod % 60)
}

fn at(h: u32, m: u32, s: u32) -> u32 {
    h * 3_600 + m * 60 + s
}

/// One invented row: `stem`, the day, the time, LTP, bid, ask and LTQ
/// text; the quantities and open interest zero.
fn row(stem: &str, d: Day, sod: u32, ltp: &str, (bid, ask): (&str, &str), ltq: &str) -> String {
    format!(
        "{stem}.NSE,{:02}/{:02}/{:04},{},{ltp},{bid},0,{ask},0,{ltq},0",
        d.day(),
        d.month(),
        d.year(),
        hms(sod)
    )
}

/// A trade row with no quote.
fn trade(stem: &str, d: Day, sod: u32, ltp: &str, ltq: u64) -> String {
    row(stem, d, sod, ltp, ("0", "0"), &ltq.to_string())
}

fn csv(rows: &[String]) -> Vec<u8> {
    let mut text = String::from(HEADER_OPEN_INTEREST);
    text.push_str("\r\n");
    for r in rows {
        text.push_str(r);
        text.push_str("\r\n");
    }
    text.into_bytes()
}

/// One stocks day zip holding `(stem, rows)` files.
fn day_zip(d: Day, files: &[(&str, Vec<String>)]) -> Vec<u8> {
    let folder = pull::gdfl_cm::day_folder_name(CmKind::Stocks, d);
    let inner: Vec<(String, Vec<u8>)> = files
        .iter()
        .map(|(stem, rows)| (format!("{folder}/{stem}.NSE.csv"), csv(rows)))
        .collect();
    zip(&inner)
}

/// An outer archive of stocks day zips.
fn archive(days: &[(Day, Vec<(&str, Vec<String>)>)]) -> Vec<u8> {
    let outer: Vec<(String, Vec<u8>)> = days
        .iter()
        .map(|(d, files)| {
            (
                pull::gdfl_archive::day_zip_name(CmKind::Stocks, *d),
                day_zip(*d, files),
            )
        })
        .collect();
    zip(&outer)
}

const D1: (u16, u8, u8) = (2025, 3, 7);
const D2: (u16, u8, u8) = (2025, 3, 10);
const D4: (u16, u8, u8) = (2025, 3, 11);
const MUHURAT: (u16, u8, u8) = (2025, 10, 21);

fn d(t: (u16, u8, u8)) -> Day {
    day(t.0, t.1, t.2)
}

/// ZZALPHA's first day: one row of every reason (see the test below for
/// the count each one earns).
fn alpha_d1() -> Vec<String> {
    let d1 = d(D1);
    let s = "ZZALPHA";
    vec![
        trade(s, d1, at(9, 14, 59), "7770.00", 1),
        trade(s, d1, at(9, 15, 0), "7777.00", 10),
        trade(s, d1, at(9, 15, 0), "7778.00", 0),
        trade(s, d1, at(9, 15, 1), "7776.50", 5),
        trade(s, d1, at(9, 15, 1), "7776.50", 5),
        trade(s, d1, at(9, 15, 2), "7790.00", 0),
        trade(s, d1, at(9, 15, 3), "0.00", 1),
        trade(s, d1, at(9, 15, 4), "7777.005", 1),
        format!("{s}.NSE,07/03/2025,{},7777.00,0,0,0,0,1", hms(at(9, 15, 5))),
        row(s, d1, at(9, 15, 6), "7777.00", ("0", "0"), "-5"),
        row(s, d1, at(9, 15, 7), "-7777.00", ("0", "0"), "1"),
        row(s, d1, at(9, 15, 8), "7777.25", ("7778.00", "7777.50"), "2"),
        row(s, d1, at(9, 15, 9), "7777.10", ("7777.00", "0"), "1"),
        trade(s, d1, at(9, 15, 5), "7777.20", 1),
        trade(s, d1, at(9, 16, 0), "7775.00", 4),
        trade(s, d1, at(15, 30, 5), "7779.00", 1),
    ]
}

fn beta_d1(stem_of_bad_row: &str) -> Vec<String> {
    let d1 = d(D1);
    vec![
        trade("ZZBETA", d1, at(10, 0, 0), "6666.00", 7),
        trade(stem_of_bad_row, d1, at(10, 0, 1), "6666.50", 3),
    ]
}

fn alpha_d2() -> Vec<String> {
    vec![
        trade("ZZALPHA", d(D2), at(9, 30, 0), "7700.00", 3),
        trade("ZZALPHA", d(D2), at(9, 30, 1), "7701.00", 4),
    ]
}

fn beta_d2(price: &str) -> Vec<String> {
    vec![trade("ZZBETA", d(D2), at(11, 0, 0), price, 2)]
}

fn delta_d2() -> Vec<String> {
    vec![
        trade("ZZDELTA", d(D2), at(10, 0, 0), "5555.00", 3),
        trade("ZZDELTA", d(D2), at(10, 0, 0), "5556.00", 0),
    ]
}

fn alpha_muhurat() -> Vec<String> {
    let m = d(MUHURAT);
    vec![
        trade("ZZALPHA", m, at(13, 45, 0), "8888.00", 1),
        trade("ZZALPHA", m, at(14, 0, 0), "8888.50", 2),
        trade("ZZALPHA", m, at(14, 50, 0), "8889.00", 3),
    ]
}

/// What the importer reads: v1.
fn v1() -> Vec<u8> {
    archive(&[
        (
            d(D1),
            vec![("ZZALPHA", alpha_d1()), ("ZZBETA", beta_d1("ZZBETA"))],
        ),
        (
            d(D2),
            vec![("ZZALPHA", alpha_d2()), ("ZZBETA", beta_d2("6667.00"))],
        ),
        (
            d(D4),
            vec![(
                "ZZBETA",
                vec![trade("ZZBETA", d(D4), at(10, 0, 0), "6600.00", 1)],
            )],
        ),
        (d(MUHURAT), vec![("ZZALPHA", alpha_muhurat())]),
    ])
}

/// v1 changed: ZZBETA's two files differ, its 2025-03-11 day zip is gone,
/// ZZGAMMA and ZZDELTA are new.
fn v2() -> Vec<u8> {
    archive(&[
        (
            d(D1),
            vec![("ZZALPHA", alpha_d1()), ("ZZBETA", beta_d1("ZZBETAX"))],
        ),
        (
            d(D2),
            vec![
                ("ZZALPHA", alpha_d2()),
                ("ZZBETA", beta_d2("6668.00")),
                (
                    "ZZGAMMA",
                    vec![trade("ZZGAMMA", d(D2), at(12, 0, 0), "4444.00", 9)],
                ),
                ("ZZDELTA", delta_d2()),
            ],
        ),
        (d(MUHURAT), vec![("ZZALPHA", alpha_muhurat())]),
    ])
}

fn env_of(pairs: Vec<(&'static str, OsString)>) -> impl Fn(&str) -> Option<OsString> {
    move |var| {
        pairs
            .iter()
            .find(|(k, _)| *k == var)
            .map(|(_, v)| v.clone())
    }
}

fn key(symbol: &str) -> InstrumentKey {
    InstrumentKey::cash(Exchange::Nse, symbol).unwrap()
}

/// Appends `bars` to `symbol`'s month of `vendor` at `timeframe`, by hand:
/// no landing is recorded.
fn write_bars(
    root: &Path,
    (vendor, timeframe): (Vendor, Timeframe),
    (symbol, on): (&str, Day),
    bars: &[Bar],
) {
    let k = key(symbol);
    let ym = YearMonth::new(on.year(), on.month()).unwrap();
    let path = StorePath::for_key(vendor, &k, timeframe, ym, FileKind::Bars).unwrap();
    let id = u32::try_from(brutex_core::universe::fnv1a(symbol) & 0xFFFF_FFFF).unwrap();
    let mut file = BarFile::open_or_create(root, path, id).unwrap();
    file.append(bars).unwrap();
}

/// The UTC microsecond of `sod` on IST day `on`.
fn micros(on: Day, sod: u32) -> i64 {
    day_span(on).0 + i64::from(sod) * 1_000_000
}

fn bar(ts: i64, (o, h, l, c): (i64, i64, i64, i64), v: i64) -> Bar {
    Bar {
        ts_micros: ts,
        open: o,
        high: h,
        low: l,
        close: c,
        volume: v,
        open_interest: OI_NULL,
    }
}

/// Imports `kind`'s `source` days `spans` into `store` through the import
/// verb itself.
fn import(
    store: &Path,
    env: &dyn Fn(&str) -> Option<OsString>,
    source: &str,
    spans: &[(&str, &str)],
) {
    let held = store.to_path_buf();
    for (from, to) in spans {
        let mut out = String::new();
        let code = crate::gdfl_import_command::command_with(
            &["stocks", source, from, to],
            env,
            &|| Ok(held.clone()),
            &mut out,
        );
        assert_eq!(code, OK, "{out}");
    }
}

/// A store the importer filled from v1 at `<root>/land.zip`, plus ZZDELTA's
/// rule-A second and ZZALPHA's Zerodha minutes written by hand.
fn imported_store(tag: &str) -> (PathBuf, PathBuf, PathBuf) {
    let root = scratch(tag);
    let store = root.join("store");
    for symbol in ["ZZALPHA", "ZZBETA", "ZZGAMMA", "ZZDELTA"] {
        std::fs::create_dir_all(store.join("bars/gdfl/NSE/FNO").join(symbol)).unwrap();
    }
    let land = root.join("land.zip");
    std::fs::write(&land, v1()).unwrap();
    let env = env_of(vec![
        (
            crate::gdfl_import_command::CM_ARCHIVE_VAR,
            land.clone().into_os_string(),
        ),
        (crate::gdfl_import_command::ALLOW_UNSTAMPED_VAR, "1".into()),
    ]);
    import(
        &store,
        &env,
        "zip",
        &[("2025-03-07", "2025-03-11"), ("2025-10-21", "2025-10-21")],
    );
    // ZZDELTA's one second as rule A has it, written by hand: no landing.
    write_bars(
        &store,
        (Vendor::Gdfl, Timeframe::SECOND_1),
        ("ZZDELTA", d(D2)),
        &[bar(
            micros(d(D2), at(10, 0, 0)),
            (555_500, 555_600, 555_500, 555_500),
            3,
        )],
    );
    // ZZALPHA's Zerodha minutes on D1: 09:15 at rule A's high and every
    // rule's low, 09:16 at every rule's extremes, 09:17 with no GDFL second.
    let z = |m: u32, h: i64, l: i64| bar(micros(d(D1), at(9, m, 0)), (l, h, l, l), 1);
    write_bars(
        &store,
        (Vendor::Zerodha, Timeframe::MINUTE_1),
        ("ZZALPHA", d(D1)),
        &[
            z(15, 777_800, 777_650),
            z(16, 777_720, 777_500),
            z(17, 1, 1),
        ],
    );
    (root, store, land)
}

/// Runs the check with `args` (after `--out <root>/out`) and `env`.
fn check(
    root: &Path,
    store: &Path,
    env: &dyn Fn(&str) -> Option<OsString>,
    args: &[&str],
) -> (u8, String, PathBuf) {
    let out_dir = root.join("out");
    let out_text = out_dir.to_str().unwrap().to_owned();
    let mut all: Vec<&str> = args.to_vec();
    all.extend(["--out", out_text.as_str()]);
    let held = store.to_path_buf();
    let mut out = String::new();
    let code = command_with(&all, env, &|| Ok(held.clone()), &mut out);
    (code, out, out_dir)
}

/// The rows of a CSV, keyed by (symbol, day, role), each a header-name map.
fn rows_of(text: &str) -> HashMap<(String, String, String), HashMap<String, String>> {
    let mut lines = text.lines();
    let header: Vec<&str> = lines.next().unwrap().split(',').collect();
    lines
        .map(|l| {
            let cells: HashMap<String, String> = header
                .iter()
                .zip(l.split(','))
                .map(|(h, c)| ((*h).to_owned(), c.to_owned()))
                .collect();
            (
                (
                    cells["symbol"].clone(),
                    cells["day"].clone(),
                    cells["role"].clone(),
                ),
                cells,
            )
        })
        .collect()
}

fn read(dir: &Path, name: &str) -> String {
    std::fs::read_to_string(dir.join(name)).unwrap()
}

fn none(_: &str) -> Option<OsString> {
    None
}

#[test]
fn every_status_and_source_is_named_against_the_landing_the_store_recorded() {
    let (root, store, land) = imported_store("statuses");
    // The landing source changes under its recorded name.
    std::fs::write(&land, v2()).unwrap();
    let (code, out, dir) = check(&root, &store, &none, &["2025"]);
    assert_eq!(code, FAILED, "not every share-day is exact: {out}");
    let rows = rows_of(&read(&dir, "tick-to-second-2025.v001.csv"));
    let get = |s: &str, on: &str| &rows[&(s.to_owned(), on.to_owned(), "landing".to_owned())];
    let land_text = land.display().to_string();
    // The importer's own fold of the recorded member, read back through the
    // store: rule T, bit for bit, the member named and its landing named.
    let alpha = get("ZZALPHA", "2025-03-07");
    assert_eq!(alpha["status"], "exact_T");
    assert_eq!(alpha["source"], "archive_member");
    assert!(
        alpha["source_name"].starts_with(&format!(
            "{land_text}!STOCKS/2025/MAR_2025/GFDLCM_STOCK_TICK_07032025.zip!"
        )),
        "{alpha:?}"
    );
    assert!(alpha["landing"].starts_with("prv:"), "{alpha:?}");
    assert_eq!(alpha["outside_day_band"], "UNVERIFIED");
    // Every bad-row reason, counted by name.
    for (field, want) in [
        ("raw_rows", "16"),
        ("kept_rows", "11"),
        ("zero_ltp", "1"),
        ("off_grid", "1"),
        ("malformed", "3"),
        ("refused_whole", "0"),
        ("negative_ltp", "1"),
        ("negative_qty", "1"),
        ("crossed_quote", "1"),
        ("one_sided_quote", "1"),
        ("identical_consecutive", "1"),
        ("out_of_order", "1"),
        ("late_traded", "1"),
        ("ltq0_rows", "2"),
        ("ltq0_new_price", "2"),
        ("trades_in_session", "7"),
        ("seconds_before_open", "1"),
        ("seconds_after_close", "1"),
        ("t_seconds", "5"),
        ("t_volume", "28"),
        ("a_seconds", "5"),
        ("a_high_widened", "1"),
        ("a_low_widened", "0"),
        ("store_seconds", "5"),
        ("fill_compared", "5"),
        ("fill_high_worse", "1"),
        ("fill_low_worse", "0"),
        ("fill_either_worse", "1"),
        ("zerodha_basis", "raw_unadjusted"),
        ("zerodha_minutes", "3"),
        ("t_compared", "2"),
        ("t_high_match", "1"),
        ("t_low_match", "2"),
        ("a_compared", "2"),
        ("a_high_match", "2"),
        ("a_low_match", "2"),
        ("r_compared", "2"),
        ("r_high_match", "1"),
        ("r_low_match", "2"),
    ] {
        assert_eq!(alpha[field], want, "ZZALPHA 2025-03-07 {field}");
    }
    assert_eq!(alpha["t_hash"], alpha["store_hash"]);
    assert_ne!(alpha["t_hash"], alpha["a_hash"]);
    assert_eq!(get("ZZALPHA", "2025-03-10")["status"], "exact_T");
    // A member whose bytes changed under its recorded name: refused before a
    // byte of it is read.
    for on in ["2025-03-07", "2025-03-10"] {
        let beta = get("ZZBETA", on);
        assert_eq!(beta["status"], "read_failed:source_changed", "{beta:?}");
        assert!(beta["detail"].contains("CRC-32"), "{beta:?}");
        assert!(beta["landing"].starts_with("prv:"), "{beta:?}");
    }
    // A day zip gone from the recorded archive.
    let gone = get("ZZBETA", "2025-03-11");
    assert_eq!(gone["status"], "read_failed:source_changed");
    assert!(
        gone["detail"].contains("holds no file for this day"),
        "{gone:?}"
    );
    // In the recorded day's listing, never landed.
    assert_eq!(get("ZZGAMMA", "2025-03-10")["status"], "raw_only");
    // Seconds no landing recorded, and no archive given to compare with.
    let delta = get("ZZDELTA", "2025-03-10");
    assert_eq!(delta["status"], "read_failed:source_unrecorded");
    assert_eq!(delta["source"], "unrecorded");
    // ... and the run says how many stored share-days had no landing record,
    // so a run over such months never reads as a landing-source check.
    assert!(
        out.contains("  1 of the stored share-days have no .prv landing record"),
        "{out}"
    );
    // The Muhurat: the trade after its window is dropped by the session
    // authority, counted, and the day is labelled.
    let muhurat = get("ZZALPHA", "2025-10-21");
    assert_eq!(muhurat["status"], "special_session");
    assert_eq!(muhurat["compare"], "exact_T");
    assert_eq!(muhurat["session"], "special:Muhurat");
    assert_eq!(muhurat["seconds_after_close"], "1");
    assert_eq!(muhurat["t_seconds"], "2");
    assert_eq!(rows.len(), 8, "{rows:?}");
    let summary = read(&dir, "tick-to-second-2025.v001.summary.txt");
    assert!(summary.contains("read_failed:source_changed"), "{summary}");
    assert!(summary.contains("fill worsening"), "{summary}");
    assert!(
        summary.contains("outside_day_band          not counted: UNVERIFIED"),
        "{summary}"
    );
    let timings = read(&dir, "tick-to-second-2025.v001.timings.txt");
    assert!(timings.contains("ns per status_of lookup"), "{timings}");
    // The index answers what the rows say.
    let checks = SecondChecks::open(&dir).unwrap();
    let alpha = checks.status_of("ZZALPHA", d(D1)).unwrap();
    assert_eq!(
        (
            alpha.status,
            alpha.source,
            alpha.store_seconds,
            alpha.fill_compared,
            alpha.fill_worse
        ),
        (Status::ExactT, Source::ArchiveMember, 5, 5, 1)
    );
    assert!(alpha.trusted());
    let beta = checks.status_of("ZZBETA", d(D2)).unwrap();
    assert_eq!(beta.status, Status::ReadFailed);
    assert!(!beta.trusted());
    assert_eq!(
        checks.status_of("ZZALPHA", day(2025, 3, 8)),
        Err(NotChecked::NotChecked { version: 1 })
    );
    assert_eq!(
        checks.status_of("ZZNOBODY", d(D1)),
        Err(NotChecked::SymbolUnknown { year: 2025 })
    );
    assert_eq!(
        checks.status_of("ZZALPHA", day(2019, 3, 7)),
        Err(NotChecked::NoVersion { year: 2019 })
    );
}

#[test]
fn an_extra_archive_is_compared_beside_the_landing_and_labelled_not_the_landing_source() {
    let (root, store, land) = imported_store("extra");
    std::fs::write(&land, v2()).unwrap();
    let (_, _, dir) = check(&root, &store, &none, &["2025"]);
    let other = root.join("other.zip");
    std::fs::write(&other, v2()).unwrap();
    let other_text = other.to_str().unwrap().to_owned();
    let (code, out, _) = check(&root, &store, &none, &["2025", "--archive", &other_text]);
    assert_eq!(code, FAILED, "{out}");
    assert!(out.contains("tick-to-second-2025.v002.csv"), "{out}");
    let rows = rows_of(&read(&dir, "tick-to-second-2025.v002.csv"));
    let get = |s: &str, on: &str| &rows[&(s.to_owned(), on.to_owned(), "extra".to_owned())];
    for row in rows.values().filter(|r| r["role"] == "extra") {
        assert_eq!(row["source"], "archive_not_the_landing_source", "{row:?}");
    }
    assert_eq!(get("ZZALPHA", "2025-03-07")["status"], "exact_T");
    assert_eq!(get("ZZALPHA", "2025-10-21")["status"], "special_session");
    // The decoder's own refusal of the other archive's file, by its word.
    assert_eq!(
        get("ZZBETA", "2025-03-07")["status"],
        "read_failed:TickerMismatch"
    );
    assert_eq!(
        get("ZZBETA", "2025-03-10")["status"],
        "differs:open+high+low+close+hash"
    );
    let only = get("ZZBETA", "2025-03-11");
    assert_eq!(only["status"], "store_only");
    assert_eq!(only["store_seconds"], "1");
    assert!(
        only["detail"].contains("no day zip for 2025-03-11"),
        "{only:?}"
    );
    assert_eq!(get("ZZGAMMA", "2025-03-10")["status"], "raw_only");
    // A store holding the every-row rule, compared where no landing was
    // recorded.
    let delta = get("ZZDELTA", "2025-03-10");
    assert_eq!(delta["status"], "exact_A");
    assert_eq!(delta["fill_either_worse"], "0", "the store IS rule A");
    // The landing rows of the first run are kept, not checked again.
    assert_eq!(
        rows[&(
            "ZZBETA".to_owned(),
            "2025-03-10".to_owned(),
            "landing".to_owned()
        )]["status"],
        "read_failed:source_changed"
    );
    // The index answers the landing row, and the extra row where no landing
    // was recorded.
    let checks = SecondChecks::open(&dir).unwrap();
    assert_eq!(checks.versions(), vec![(2025, 2)]);
    let delta = checks.status_of("ZZDELTA", d(D2)).unwrap();
    assert_eq!(
        (delta.status, delta.source),
        (Status::ExactA, Source::NotTheLandingSource)
    );
    assert!(!delta.trusted(), "not the landing source: never trusted");
    let beta = checks.status_of("ZZBETA", d(D2)).unwrap();
    assert_eq!(
        (beta.status, beta.source),
        (Status::ReadFailed, Source::ArchiveMember)
    );
    // Nothing new: the read-failed share-days are read again and fail the
    // same way, so no row is filed, no segment and no version is made, and
    // the run still says they are not exact (they are not).
    let segments_before = segments_in(&dir, 2025).unwrap();
    let (code, out, _) = check(&root, &store, &none, &["2025", "--archive", &other_text]);
    assert_eq!(code, FAILED, "{out}");
    assert!(out.contains("4 already answered"), "{out}");
    assert!(out.contains("2025 nothing new to fold"), "{out}");
    assert!(!dir.join("tick-to-second-2025.v003.csv").exists());
    assert_eq!(segments_in(&dir, 2025).unwrap(), segments_before);
}

#[test]
fn a_tick_store_landing_is_checked_against_that_tick_store_day_and_a_changed_day_is_refused() {
    let root = scratch("tickstore");
    let store = root.join("store");
    std::fs::create_dir_all(store.join("bars/gdfl/NSE/FNO/ZZALPHA")).unwrap();
    let ts = root.join("ts");
    std::fs::create_dir_all(&ts).unwrap();
    for (on, rows) in [(d(D1), alpha_d1()), (d(D2), alpha_d2())] {
        let name = format!("{}.zip", pull::gdfl_cm::day_folder_name(CmKind::Stocks, on));
        let zip_path = root.join(name);
        std::fs::write(&zip_path, day_zip(on, &[("ZZALPHA", rows)])).unwrap();
        let run = pull::tickstore_write::write_day(
            &zip_path,
            &ts,
            &pull::tickstore_write::WriteOptions::new("t2s-test"),
        );
        assert!(run.result.is_ok(), "{run:?}");
    }
    // The importer's door takes a tick store only with its FORMAT.md at the
    // root; the words are invented.
    std::fs::write(ts.join("FORMAT.md"), b"synthetic tick store\n").unwrap();
    let env = env_of(vec![
        (
            crate::gdfl_import_command::TICKSTORE_VAR,
            ts.clone().into_os_string(),
        ),
        (crate::gdfl_import_command::ALLOW_UNSTAMPED_VAR, "1".into()),
    ]);
    import(&store, &env, "tickstore", &[("2025-03-07", "2025-03-10")]);
    // The second day's file grows after its landing.
    let grown = ts.join(
        TickStore::new(Path::new(""))
            .day_path(CmKind::Stocks, d(D2))
            .display()
            .to_string(),
    );
    let mut bytes = std::fs::read(&grown).unwrap();
    bytes.push(0);
    std::fs::write(&grown, bytes).unwrap();
    let (code, out, dir) = check(&root, &store, &env, &["2025"]);
    assert_eq!(code, FAILED, "{out}");
    // Every landing here is recorded: no unrecorded line is printed.
    assert!(!out.contains("no .prv landing record"), "{out}");
    let rows = rows_of(&read(&dir, "tick-to-second-2025.v001.csv"));
    let first = &rows[&(
        "ZZALPHA".to_owned(),
        "2025-03-07".to_owned(),
        "landing".to_owned(),
    )];
    assert_eq!(first["status"], "exact_T", "{first:?}");
    assert_eq!(first["source"], "tickstore_day");
    assert_eq!(
        first["source_name"],
        "cm/STOCKS/2025/MAR_2025/GFDLCM_STOCK_TICK_07032025.bts"
    );
    let second = &rows[&(
        "ZZALPHA".to_owned(),
        "2025-03-10".to_owned(),
        "landing".to_owned(),
    )];
    assert_eq!(second["status"], "read_failed:source_changed", "{second:?}");
    assert_eq!(second["source"], "tickstore_day");
    // Without the tick store's root, its landings are refused by name.
    let other_root = scratch("tickstore-unset");
    let (_, _, other_dir) = check(&other_root, &store, &none, &["2025"]);
    let rows = rows_of(&read(&other_dir, "tick-to-second-2025.v001.csv"));
    let unset = &rows[&(
        "ZZALPHA".to_owned(),
        "2025-03-07".to_owned(),
        "landing".to_owned(),
    )];
    assert_eq!(unset["status"], "read_failed:tickstore_unset", "{unset:?}");
}

#[test]
fn no_worker_count_and_no_resumption_changes_a_byte_of_the_version() {
    let (root, store, _) = imported_store("bytes");
    let one = root.join("one");
    let two = root.join("two");
    let adaptive = root.join("adaptive");
    let resumed = root.join("resumed");
    let run = |dir: &Path, args: &[&str]| {
        let mut all: Vec<&str> = vec!["2025", "--out", dir.to_str().unwrap()];
        all.extend_from_slice(args);
        let held = store.clone();
        let mut out = String::new();
        let code = command_with(&all, &none, &|| Ok(held.clone()), &mut out);
        (code, out)
    };
    let workers = host_cores().min(2).to_string();
    run(&one, &["--workers", "1"]);
    run(&two, &["--workers", &workers]);
    run(&adaptive, &[]);
    // A run cut short after one share, then the rest.
    let (_, out) = run(&resumed, &["--symbols", "ZZALPHA"]);
    assert!(out.contains("v001.csv"), "{out}");
    assert!(!out.contains("no such share"), "ZZALPHA is held: {out}");
    let (_, out) = run(&resumed, &[]);
    assert!(out.contains("v002.csv"), "{out}");
    assert!(
        out.contains("3 already answered"),
        "ZZALPHA's three days are not read again: {out}"
    );
    let file = |dir: &Path, v: &str, kind: &str| {
        std::fs::read(dir.join(format!("tick-to-second-2025.{v}.{kind}"))).unwrap()
    };
    for kind in ["csv", "summary.txt", "idx"] {
        let want = file(&one, "v001", kind);
        assert_eq!(file(&two, "v001", kind), want, "{kind}: workers");
        assert_eq!(
            file(&adaptive, "v001", kind),
            want,
            "{kind}: the doubling rule"
        );
    }
    // Resumed: the same rows, byte for byte, and the same index records; the
    // index header differs only in the segments it folded.
    assert_eq!(file(&resumed, "v002", "csv"), file(&one, "v001", "csv"));
    let (a, b) = (file(&resumed, "v002", "idx"), file(&one, "v001", "idx"));
    assert_eq!(a[index::HEADER_LEN..], b[index::HEADER_LEN..]);
    assert_eq!(a[48..80], b[48..80], "the same CSV digest");
    let leftovers: Vec<_> = std::fs::read_dir(&resumed)
        .unwrap()
        .filter_map(Result::ok)
        .filter(|e| e.file_name().to_string_lossy().ends_with(".tmp"))
        .collect();
    assert!(leftovers.is_empty(), "no staging name is left behind");
    // A run that learns nothing: the one read-failed share-day (ZZDELTA's
    // unrecorded landing) is read again, fails the same way and is not filed
    // again; the other six are answered; no segment and no version is made.
    let segments = segments_in(&one, 2025).unwrap();
    let (code, out) = run(&one, &[]);
    assert_eq!(code, FAILED, "the failure is still there: {out}");
    assert!(
        out.contains("7 stored share-days, 6 already answered, 1 days checked, 1 rows made"),
        "{out}"
    );
    assert!(out.contains("2025 nothing new to fold"), "{out}");
    assert_eq!(segments_in(&one, 2025).unwrap(), segments);
    assert!(!one.join("tick-to-second-2025.v002.csv").exists());
    // v001 of the resumed directory is never touched by the fold.
    let first = rows_of(&String::from_utf8(file(&resumed, "v001", "csv")).unwrap());
    assert!(first.keys().all(|(s, _, _)| s == "ZZALPHA"));
}

#[test]
fn a_new_landing_of_a_checked_day_is_checked_again() {
    let (root, store, _) = imported_store("relanded");
    let (_, _, dir) = check(&root, &store, &none, &["2025", "--symbols", "ZZALPHA"]);
    let mut ledger = load_year(&dir, 2025).unwrap();
    assert_eq!(ledger.version, Some(1));
    // Rewrite one landing row as if it had been checked under another
    // landing: the next run checks that day again, and only that day.
    let k = (
        "2025-03-10".to_owned(),
        "ZZALPHA".to_owned(),
        "landing".to_owned(),
    );
    let old = ledger.lines.remove(&k).unwrap();
    let cells = Cells::of(&old).unwrap();
    let stale = old.replacen(cells.get("landing"), "prv:999@g1:def4", 1);
    let mut text = String::from(HEADER);
    text.push('\n');
    text.push_str(&stale);
    text.push('\n');
    publish_new(
        &dir,
        &segment_name(2025, ledger.last_segment + 1),
        text.as_bytes(),
    )
    .unwrap();
    let (_, out, _) = check(&root, &store, &none, &["2025", "--symbols", "ZZALPHA"]);
    assert!(out.contains("2 already answered, 1 days checked"), "{out}");
    let rows = rows_of(&read(&dir, "tick-to-second-2025.v002.csv"));
    assert_eq!(
        rows[&(
            "ZZALPHA".to_owned(),
            "2025-03-10".to_owned(),
            "landing".to_owned()
        )]["landing"],
        cells.get("landing")
    );
}

#[test]
fn a_file_the_reader_refuses_whole_still_prints_its_counts_and_the_two_way_verdict() {
    // 70 bad rows of 100: refused whole, and PREC-5's two-way rule refuses
    // it too; 65 bad rows of 1,000: refused whole today, admitted by the
    // two-way rule.
    assert!(
        two_way_words(70, 100)
            .contains("two-way rule would refuse it (70 malformed against 30 well-formed)")
    );
    assert!(
        two_way_words(65, 1_000).contains("would admit it (65 malformed against 935 well-formed)")
    );
    assert!(two_way_words(0, 0).contains("would admit"));
    let root = scratch("bad-rows");
    let store = root.join("store");
    std::fs::create_dir_all(store.join("bars/gdfl/NSE/FNO/ZZEPS")).unwrap();
    let on = d(D1);
    let mut rows: Vec<String> = (0..30)
        .map(|i| trade("ZZEPS", on, at(10, 0, i), "5000.00", 1))
        .collect();
    rows.extend((0..70).map(|i| trade("ZZEPS", on, at(10, 1, i % 60), "0.00", 1)));
    let other = root.join("bad.zip");
    std::fs::write(&other, archive(&[(on, vec![("ZZEPS", rows)])])).unwrap();
    let (code, out, dir) = check(
        &root,
        &store,
        &none,
        &["2025", "--archive", other.to_str().unwrap()],
    );
    assert_eq!(code, FAILED, "{out}");
    let rows = rows_of(&read(&dir, "tick-to-second-2025.v001.csv"));
    let eps = &rows[&(
        "ZZEPS".to_owned(),
        "2025-03-07".to_owned(),
        "extra".to_owned(),
    )];
    assert_eq!(eps["status"], "read_failed:TooManyBadRows", "{eps:?}");
    assert_eq!(eps["refused_whole"], "70");
    assert_eq!(
        eps["raw_rows"], "100",
        "the census counts every line either way"
    );
    assert!(eps["detail"].contains("would refuse it"), "{eps:?}");
}

/// The auction era: a share whose security master flags it eligible on
/// 2026-08-03, its continuous trade and one closing-auction print (zero bid
/// and ask, LTQ > 0, inside the band), invented prices; the store's bars
/// written by hand as bar definition 2 filed them, so no landing is recorded
/// and the archive is compared as what it is.
#[test]
fn a_store_day_holding_the_folded_auction_print_is_named_by_that_one_difference() {
    let root = scratch("auction");
    let store = root.join("store");
    let on = day(2026, 8, 3);
    let isin = brutex_core::universe::nse_isin("RELIANCE").expect("RELIANCE has an ISIN");
    let master = format!(
        "FinInstrmId,TckrSymb,SctySrs,ISIN,ElgbltyClsgAuctnSsn\n2885,RELIANCE,EQ,{},1\n",
        isin.as_str()
    );
    let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
    std::io::Write::write_all(&mut gz, master.as_bytes()).unwrap();
    pull::cash_session_cache::install(&store.join("session-masters"), on, &gz.finish().unwrap())
        .unwrap();
    let s = "RELIANCE";
    let rows = vec![
        row(s, on, at(10, 0, 0), "7777.00", ("7776.00", "7778.00"), "5"),
        row(s, on, at(10, 0, 0), "7777.50", ("7776.00", "7778.00"), "2"),
        trade(s, on, at(15, 29, 0), "7780.00", 40),
        trade(s, on, at(15, 29, 0), "7780.00", 40),
    ];
    let zip_bytes = archive(&[(on, vec![(s, rows)])]);
    let ten = bar(
        micros(on, at(10, 0, 0)),
        (777_700, 777_750, 777_700, 777_750),
        7,
    );
    let print = bar(
        micros(on, at(15, 29, 0)),
        (778_000, 778_000, 778_000, 778_000),
        80,
    );
    write_bars(
        &store,
        (Vendor::Gdfl, Timeframe::SECOND_1),
        (s, on),
        &[ten, print],
    );
    let later = day(2026, 8, 5);
    write_bars(
        &store,
        (Vendor::Gdfl, Timeframe::SECOND_1),
        (s, later),
        &[bar(
            micros(later, at(10, 0, 0)),
            (777_000, 777_000, 777_000, 777_000),
            1,
        )],
    );
    let zip_path = root.join("auction.zip");
    std::fs::write(&zip_path, zip_bytes).unwrap();
    let (code, out, dir) = check(
        &root,
        &store,
        &none,
        &[
            "2026",
            "--symbols",
            s,
            "--archive",
            zip_path.to_str().unwrap(),
        ],
    );
    assert_eq!(
        code, FAILED,
        "a store day the importer would restate is not clean: {out}"
    );
    let rows = rows_of(&read(&dir, "tick-to-second-2026.v001.csv"));
    let it = &rows[&(s.to_owned(), "2026-08-03".to_owned(), "extra".to_owned())];
    for (field, want) in [
        ("status", "differs:auction_prints_folded"),
        ("compare", "exact_T_with_auction_prints"),
        ("source", "archive_not_the_landing_source"),
        ("auction_prints", "1"),
        ("auction_volume", "80"),
        ("t_seconds", "1"),
        ("t_volume", "7"),
        ("folded_seconds", "2"),
        ("folded_volume", "87"),
        ("store_seconds", "2"),
        ("store_volume", "87"),
    ] {
        assert_eq!(it[field], want, "RELIANCE 2026-08-03 {field}: {it:?}");
    }
    assert_eq!(it["folded_hash"], it["store_hash"]);
    assert_ne!(it["t_hash"], it["store_hash"]);
    let after = &rows[&(s.to_owned(), "2026-08-05".to_owned(), "extra".to_owned())];
    assert_eq!(after["status"], "store_only");
    assert!(
        after["detail"].contains("no day zip for 2026-08-05"),
        "{after:?}"
    );
    assert_eq!(rows.len(), 2, "{rows:?}");
    let checks = SecondChecks::open(&dir).unwrap();
    assert_eq!(
        checks.status_of(s, on).unwrap().status,
        Status::AuctionPrintsFolded
    );
}

/// A day with two closing-auction prints whose store filed ONE in its `.auc`
/// sidecar and folded the OTHER into the seconds (the shape bar definition 3
/// left on tick-store days of 2026-09): neither rule T nor the every-print
/// fold, but rule T with exactly the print the sidecar does not hold. The
/// label is earned by equality with that rebuild, and the sidecar's count is
/// on the row. Invented prices, the store and the sidecar written by hand.
#[test]
fn a_store_day_that_filed_one_print_and_folded_the_other_is_named_by_its_sidecar() {
    let root = scratch("unfiled");
    let store = root.join("store");
    let on = day(2026, 8, 3);
    let isin = brutex_core::universe::nse_isin("RELIANCE").expect("RELIANCE has an ISIN");
    let master = format!(
        "FinInstrmId,TckrSymb,SctySrs,ISIN,ElgbltyClsgAuctnSsn\n2885,RELIANCE,EQ,{},1\n",
        isin.as_str()
    );
    let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
    std::io::Write::write_all(&mut gz, master.as_bytes()).unwrap();
    pull::cash_session_cache::install(&store.join("session-masters"), on, &gz.finish().unwrap())
        .unwrap();
    let s = "RELIANCE";
    let rows = vec![
        row(s, on, at(10, 0, 0), "7777.00", ("7776.00", "7778.00"), "5"),
        row(s, on, at(10, 0, 0), "7777.50", ("7776.00", "7778.00"), "2"),
        trade(s, on, at(15, 20, 0), "7781.00", 3),
        trade(s, on, at(15, 29, 0), "7780.00", 40),
    ];
    let zip_path = root.join("unfiled.zip");
    std::fs::write(&zip_path, archive(&[(on, vec![(s, rows)])])).unwrap();
    let ten = bar(
        micros(on, at(10, 0, 0)),
        (777_700, 777_750, 777_700, 777_750),
        7,
    );
    let folded = bar(
        micros(on, at(15, 29, 0)),
        (778_000, 778_000, 778_000, 778_000),
        40,
    );
    write_bars(
        &store,
        (Vendor::Gdfl, Timeframe::SECOND_1),
        (s, on),
        &[ten, folded],
    );
    let filed = pull::gdfl_cm::auction::Print {
        sod: at(15, 20, 0),
        price: 778_100,
        quantity: 3,
        rows: 1,
        line: 4,
    };
    pull::gdfl_cm::auction::append(&store, &key(s), on, &[filed]).unwrap();
    let (code, out, dir) = check(
        &root,
        &store,
        &none,
        &[
            "2026",
            "--symbols",
            s,
            "--archive",
            zip_path.to_str().unwrap(),
        ],
    );
    assert_eq!(code, FAILED, "a restated day is not clean: {out}");
    let rows = rows_of(&read(&dir, "tick-to-second-2026.v001.csv"));
    let it = &rows[&(s.to_owned(), "2026-08-03".to_owned(), "extra".to_owned())];
    for (field, want) in [
        ("status", "differs:auction_prints_folded"),
        ("compare", "exact_T_with_unfiled_auction_prints"),
        ("auction_prints", "2"),
        ("auction_volume", "43"),
        ("auction_in_sidecar", "1"),
        ("t_seconds", "1"),
        ("folded_seconds", "3"),
        ("folded_volume", "50"),
        ("unfiled_seconds", "2"),
        ("unfiled_volume", "47"),
        ("store_seconds", "2"),
        ("store_volume", "47"),
        ("detail", ""),
    ] {
        assert_eq!(it[field], want, "RELIANCE 2026-08-03 {field}: {it:?}");
    }
    assert_eq!(it["unfiled_hash"], it["store_hash"]);
    assert_ne!(it["folded_hash"], it["store_hash"]);
    // The same store with the sidecar holding both prints: nothing unfiled is
    // put back, so no unfiled signature, and the day stays a bare difference
    // (its bars hold a print the sidecar also holds: filed twice).
    let both = root.join("both");
    let store2 = both.join("store");
    pull::cash_session_cache::install(&store2.join("session-masters"), on, &{
        let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        std::io::Write::write_all(&mut gz, master.as_bytes()).unwrap();
        gz.finish().unwrap()
    })
    .unwrap();
    write_bars(
        &store2,
        (Vendor::Gdfl, Timeframe::SECOND_1),
        (s, on),
        &[ten, folded],
    );
    let other = pull::gdfl_cm::auction::Print {
        sod: at(15, 29, 0),
        price: 778_000,
        quantity: 40,
        rows: 1,
        line: 5,
    };
    pull::gdfl_cm::auction::append(&store2, &key(s), on, &[filed, other]).unwrap();
    let (code, out, dir) = check(
        &both,
        &store2,
        &none,
        &[
            "2026",
            "--symbols",
            s,
            "--archive",
            zip_path.to_str().unwrap(),
        ],
    );
    assert_eq!(code, FAILED, "{out}");
    let rows = rows_of(&read(&dir, "tick-to-second-2026.v001.csv"));
    let it = &rows[&(s.to_owned(), "2026-08-03".to_owned(), "extra".to_owned())];
    assert_eq!(it["auction_in_sidecar"], "2", "{it:?}");
    assert_eq!(it["unfiled_seconds"], "", "{it:?}");
    assert!(it["status"].starts_with("differs:seconds"), "{it:?}");
    // A sidecar that cannot be read is said on the row, never taken as empty.
    let auc = pull::gdfl_cm::auction::path(&store2, &key(s), on).unwrap();
    std::fs::write(&auc, b"not a sidecar").unwrap();
    let bad = root.join("bad");
    std::fs::create_dir_all(&bad).unwrap();
    let (_, out, dir) = check(
        &bad,
        &store2,
        &none,
        &[
            "2026",
            "--symbols",
            s,
            "--archive",
            zip_path.to_str().unwrap(),
        ],
    );
    let rows = rows_of(&read(&dir, "tick-to-second-2026.v001.csv"));
    let it = &rows[&(s.to_owned(), "2026-08-03".to_owned(), "extra".to_owned())];
    assert!(
        it["detail"].contains(".auc sidecar could not be read"),
        "{out} {it:?}"
    );
    assert_eq!(it["auction_in_sidecar"], "0", "{it:?}");
    assert_eq!(it["unfiled_seconds"], "", "{it:?}");
}

#[test]
fn an_out_directory_inside_the_store_and_a_held_lock_are_refused_before_anything_is_read() {
    let root = scratch("inside");
    let store = root.join("store");
    std::fs::create_dir_all(&store).unwrap();
    let inside = store.join("reports");
    let mut out = String::new();
    let code = command_with(
        &["2025", "--out", inside.to_str().unwrap()],
        &none,
        &|| Ok(store.clone()),
        &mut out,
    );
    assert_eq!(code, FAILED);
    assert!(out.contains("never writes the store"), "{out}");
    assert!(!inside.exists(), "nothing was made inside the store");
    let out_dir = root.join("out");
    std::fs::create_dir_all(&out_dir).unwrap();
    let held = std::fs::File::create(out_dir.join(LOCK_NAME)).unwrap();
    held.lock().unwrap();
    let mut out = String::new();
    let code = command_with(
        &["2025", "--out", out_dir.to_str().unwrap()],
        &none,
        &|| Ok(store.clone()),
        &mut out,
    );
    assert_eq!(code, FAILED);
    assert!(
        out.contains("another tick-to-second-check writes this directory"),
        "{out}"
    );
    let mut out = String::new();
    let code = command_with(
        &[
            "2025",
            "--out",
            root.join("o2").to_str().unwrap(),
            "--archive",
            root.join("missing.zip").to_str().unwrap(),
        ],
        &none,
        &|| Ok(store.clone()),
        &mut out,
    );
    assert_eq!(code, FAILED);
    assert!(
        out.contains("--archive"),
        "an archive asked for by name is refused by name: {out}"
    );
}

#[test]
fn a_version_whose_rows_are_not_the_ones_its_index_names_is_refused() {
    let (root, store, _) = imported_store("tamper");
    let (_, _, dir) = check(&root, &store, &none, &["2025"]);
    let csv_path = dir.join("tick-to-second-2025.v001.csv");
    let mut text = std::fs::read_to_string(&csv_path).unwrap();
    text.push_str(&text.lines().nth(1).unwrap().replace("exact_T", "exact_A"));
    text.push('\n');
    std::fs::write(&csv_path, text).unwrap();
    let why = load_year(&dir, 2025).unwrap_err();
    assert!(why.contains("is not the CSV its index"), "{why}");
    let (code, out, _) = check(&root, &store, &none, &["2025"]);
    assert_eq!(code, FAILED);
    assert!(out.contains("REFUSED"), "{out}");
    // A file of another layout is refused, never read as this one.
    let other = scratch("layout");
    std::fs::write(
        other.join(segment_name(2025, 1)),
        "symbol,day,status\nZZ,2025-03-07,exact_T\n",
    )
    .unwrap();
    assert!(
        load_year(&other, 2025)
            .unwrap_err()
            .contains("not a file of this check's layout")
    );
}

#[test]
fn the_index_round_trips_and_refuses_what_is_not_its_format() {
    let on = d(D1);
    let answer = |status, source, row| SecondStatus {
        status,
        source,
        store_seconds: 20_148,
        fill_compared: 20_000,
        fill_worse: 123,
        row,
    };
    let a = answer(Status::ExactT, Source::ArchiveMember, 1);
    let b = answer(Status::StoreOnly, Source::NotTheLandingSource, 2);
    let image = index::encode(
        2025,
        &[
            ("ZZB", on, b),
            ("ZZA", on, a),
            ("ZZA", day(2025, 12, 31), b),
        ],
        7,
        [9; 32],
    )
    .unwrap();
    let path = Path::new("x.idx");
    let idx = index::YearIndex::decode(&image, path, 3).unwrap();
    assert_eq!(idx.symbols(), ["ZZA".to_owned(), "ZZB".to_owned()]);
    assert_eq!(idx.status_of("ZZA", on), Ok(a));
    assert_eq!(idx.status_of("ZZB", on), Ok(b));
    assert_eq!(idx.status_of("ZZA", day(2025, 12, 31)), Ok(b));
    assert_eq!(
        idx.status_of("ZZB", day(2025, 1, 1)),
        Err(NotChecked::NotChecked { version: 3 })
    );
    assert_eq!(
        idx.status_of("ZZA", day(2026, 1, 1)),
        Err(NotChecked::NotChecked { version: 3 })
    );
    assert_eq!(
        (idx.segments_through(), idx.csv_digest(), idx.year()),
        (7, [9; 32], 2025)
    );
    assert_eq!(idx.file_digest(), brutex_core::blake3::hash(&image));
    for code in 1..=8 {
        assert_eq!(Status::from_code(code).unwrap().code(), code);
    }
    for code in 1..=4 {
        let s = Source::from_code(code).unwrap();
        assert_eq!(Source::of_word(s.word()), Some(s));
    }
    assert_eq!(Status::from_code(0), None);
    assert_eq!(
        Status::of_word(FOLDED_STATUS),
        Some(Status::AuctionPrintsFolded)
    );
    assert_eq!(Status::of_word("differs:hash"), Some(Status::Differs));
    assert_eq!(Status::of_word("bogus"), None);
    // Every refusal names what is wrong.
    let mut bad = image.clone();
    bad[0] = b'X';
    assert!(
        index::YearIndex::decode(&bad, path, 1)
            .unwrap_err()
            .contains("magic")
    );
    let mut bad = image.clone();
    bad[28] = 8;
    assert!(
        index::YearIndex::decode(&bad, path, 1)
            .unwrap_err()
            .contains("stride")
    );
    let mut bad = image.clone();
    bad.pop();
    assert!(
        index::YearIndex::decode(&bad, path, 1)
            .unwrap_err()
            .contains("records")
    );
    let mut bad = image.clone();
    let offset = usize::try_from(on.days_from_epoch() - day(2025, 1, 1).days_from_epoch()).unwrap();
    let first_record = index::HEADER_LEN + "ZZA\nZZB\n".len() + offset * index::STRIDE;
    bad[first_record] = 99;
    assert!(
        index::YearIndex::decode(&bad, path, 1)
            .unwrap_err()
            .contains("status code 99")
    );
    assert!(index::encode(2025, &[("ZZA", day(2024, 12, 31), a)], 0, [0; 32]).is_err());
    assert!(index::encode(2025, &[("A\nB", on, a)], 0, [0; 32]).is_err());
    // Two versions: the digest every answer came from changes with them.
    let dir = scratch("digest");
    std::fs::write(dir.join("tick-to-second-2025.v001.idx"), &image).unwrap();
    let first = SecondChecks::open(&dir).unwrap();
    let again = index::encode(2025, &[("ZZA", on, b)], 8, [1; 32]).unwrap();
    std::fs::write(dir.join("tick-to-second-2025.v002.idx"), &again).unwrap();
    let second = SecondChecks::open(&dir).unwrap();
    assert_eq!(second.versions(), vec![(2025, 2)]);
    assert_ne!(first.file_digest(), second.file_digest());
    assert_eq!(second.status_of("ZZA", on), Ok(b));
    assert_eq!(second.year(2025).unwrap().version(), 2);
    assert_eq!(
        SecondChecks::open(&dir.join("absent")).unwrap(),
        SecondChecks::default()
    );
}

#[test]
fn the_landing_of_a_day_is_its_one_recorded_source_or_named_as_not_one() {
    let record = |ordinal: u64, name: &str| store::provenance::Record {
        ordinal,
        vendor: Vendor::Gdfl,
        source_kind: store::provenance::SourceKind::ArchiveMember,
        source_bytes: 100,
        source_crc32: 7,
        source_name: name.to_owned(),
        source_name_truncated: false,
        source_name_blake3: brutex_core::blake3::hash(name.as_bytes()),
        day: store::provenance::Day::new(2025, 3, 7).unwrap(),
        first_index: ordinal * 10,
        n_bars: 10,
        bar_definition: 4,
        journal_ordinal: 1,
        bar_file_generation: 5,
        commit: None,
        landed_at_micros: 0,
    };
    let name = "/x/a.zip!STOCKS/2025/MAR_2025/GFDLCM_STOCK_TICK_07032025.zip!F/ZZA.NSE.csv";
    assert_eq!(day_source(None), DaySource::Unrecorded);
    assert_eq!(day_source(Some(&Vec::new())), DaySource::Unrecorded);
    let DaySource::Recorded(landed) = day_source(Some(&vec![record(0, name), record(3, name)]))
    else {
        panic!("one source");
    };
    assert_eq!(landed.key, "prv:0+3@g5:def4");
    assert_eq!(landed.archive(d(D1)), Some("/x/a.zip"));
    assert_eq!(
        landed.archive(d(D2)),
        None,
        "another day's zip is not in the name"
    );
    // A name the sidecar kept as its first bytes (D-3334 keeps 128): the
    // archive is still found when the cut falls inside the day zip's name or
    // right after the archive's `!`, and only then. The whole name's BLAKE3
    // then decides (`check_day`'s namer).
    let cut = |keep: usize| Landed {
        name: name[..keep].to_owned(),
        truncated: true,
        ..landed.clone()
    };
    let zip_at = name.find("!STOCKS/").unwrap();
    for keep in [zip_at + 1, zip_at + 9, zip_at + 46, name.len() - 4] {
        assert_eq!(cut(keep).archive(d(D1)), Some("/x/a.zip"), "kept {keep}");
    }
    assert_eq!(
        cut(zip_at + 46).archive(d(D2)),
        None,
        "a cut that has passed this day's zip name is another day's"
    );
    assert_eq!(cut(zip_at + 9).archive(d(D2)), Some("/x/a.zip"));
    assert_eq!(
        cut(zip_at).archive(d(D1)),
        None,
        "the cut is inside the path"
    );
    assert_eq!(
        Landed {
            name: name[..zip_at + 9].to_owned(),
            ..landed.clone()
        }
        .archive(d(D1)),
        None,
        "a name kept whole must hold the day zip whole"
    );
    assert!(matches!(
        day_source(Some(&vec![record(0, name), record(1, "/y/b.zip!z")])),
        DaySource::Ambiguous(_)
    ));
}

#[test]
fn the_doubling_rule_starts_at_one_and_stops_where_the_work_per_second_stops_rising() {
    let nap = || std::thread::sleep(std::time::Duration::from_millis(30));
    // Every job naps the same: more workers, more work per second.
    let (got, rounds) = schedule(
        12,
        None,
        4,
        |w: &u64| *w,
        |i| {
            nap();
            u64::try_from(i).unwrap() + 1
        },
    )
    .unwrap();
    assert_eq!(got, (1..=12).collect::<Vec<u64>>(), "answers in job order");
    let workers: Vec<usize> = rounds.iter().map(|r| r.workers).collect();
    assert_eq!(&workers[..3], &[1, 2, 4], "{rounds:?}");
    assert_eq!(rounds.iter().map(|r| r.jobs).sum::<usize>(), 12);
    // A ceiling that is not a power of two is reached, never skipped: the
    // doubling from 4 goes to the ceiling 6, not to 8 and not back to 4.
    let (got, rounds) = schedule(
        20,
        None,
        6,
        |w: &u64| *w,
        |i| {
            nap();
            u64::try_from(i).unwrap() + 1
        },
    )
    .unwrap();
    assert_eq!(got, (1..=20).collect::<Vec<u64>>(), "answers in job order");
    let workers: Vec<usize> = rounds.iter().map(|r| r.workers).collect();
    assert_eq!(&workers[..4], &[1, 2, 4, 6], "{rounds:?}");
    assert!(workers.iter().all(|w| *w <= 6), "{rounds:?}");
    assert_eq!(rounds.iter().map(|r| r.jobs).sum::<usize>(), 20);
    // The two-worker round does almost no work: back to one worker.
    let (_, rounds) = schedule(
        8,
        None,
        8,
        |w: &u64| *w,
        |i| {
            nap();
            if i == 0 { 1_000_000 } else { 1 }
        },
    )
    .unwrap();
    let workers: Vec<usize> = rounds.iter().map(|r| r.workers).collect();
    assert_eq!(workers, vec![1, 2, 1], "{rounds:?}");
    // A fixed count is one round; nothing to do is no round.
    let (_, rounds) = schedule(5, Some(3), 8, |_: &u8| 1, |_| 0_u8).unwrap();
    assert_eq!(rounds.len(), 1);
    assert_eq!((rounds[0].workers, rounds[0].jobs), (3, 5));
    assert!(
        schedule(0, None, 8, |_: &u8| 1, |_| 0_u8)
            .unwrap()
            .1
            .is_empty()
    );
    let r = |work, wall_ns| Round {
        workers: 1,
        jobs: 1,
        work,
        wall_ns,
    };
    assert!(faster(&r(3, 10), &r(2, 10)));
    assert!(!faster(&r(2, 10), &r(2, 10)));
    assert!(
        faster(&r(1, 0), &r(0, 0)),
        "a zero wall time is taken as one nanosecond"
    );
}

#[test]
fn every_misuse_is_refused_before_anything_is_read() {
    let cores = host_cores();
    let too_many = (cores + 1).to_string();
    for (args, word) in [
        (vec!["20x5", "--out", "o"], "is not a year"),
        (vec!["1969", "--out", "o"], "is not a year"),
        (vec!["2025"], "--out DIR is required"),
        (vec![], "--out DIR is required"),
        (vec!["2025", "--out"], "--out takes a directory"),
        (vec!["--out", "o", "--archive"], "--archive takes a zip"),
        (
            vec!["2025", "--out", "o", "--workers"],
            "--workers takes a count",
        ),
        (
            vec!["2025", "--out", "o", "--workers", "0"],
            "is not a worker count",
        ),
        (
            vec!["2025", "--out", "o", "--workers", too_many.as_str()],
            "is not a worker count",
        ),
        (
            vec!["2025", "--out", "o", "--workers", "x"],
            "is not a worker count",
        ),
        (
            vec!["2025", "--out", "o", "--symbols"],
            "at least one symbol",
        ),
        (vec!["2025", "--out", "o", "--frobnicate"], "is not a flag"),
    ] {
        let mut out = String::new();
        let code = command_with(&args, &none, &|| Err("unread".into()), &mut out);
        assert_eq!(code, MISUSED, "{args:?}: {out}");
        assert!(out.contains(word), "{args:?}: {out}");
    }
    let asked = ask(&[
        "2025",
        "--symbols",
        "A,B",
        "C",
        "--out",
        "o",
        "--workers",
        "1",
    ])
    .unwrap();
    assert_eq!(asked.workers, Some(1));
    assert_eq!(asked.symbols.unwrap().len(), 3);
    let every = ask(&["--out", "o", "--archive", "z.zip"]).unwrap();
    assert_eq!((every.year, every.workers), (None, None));
    assert_eq!(every.archive, Some(PathBuf::from("z.zip")));
}

#[test]
fn the_census_is_one_pass_and_its_work_grows_with_the_rows_alone() {
    let line = |i: usize| format!("ZZ.NSE,07/03/2025,10:00:00,{}.00,0,0,0,0,1,0", 1000 + i % 7);
    for n in [1_usize, 10, 10_000] {
        let rows: Vec<String> = (0..n).map(line).collect();
        let c = census(&csv(&rows));
        assert_eq!(
            c.steps,
            u64::try_from(n).unwrap(),
            "one step per line, n={n}"
        );
        assert_eq!(c.lines, c.steps);
        assert_eq!(c.identical, 0);
    }
    let rows = vec![
        "ZZ.NSE,07/03/2025,10:00:00,5.00,0,0,0,0,1,0".to_owned(),
        "ZZ.NSE,07/03/2025,10:00:00,5.00,0,0,0,0,1,0".to_owned(),
        "ZZ.NSE,07/03/2025,10:00:01,-5.00,6.00,1,5.00,1,1,0".to_owned(),
        "ZZ.NSE,07/03/2025,10:00:01,5.00,0,-1,4.00,0,1,0".to_owned(),
        "ZZ.NSE,07/03/2025,10:00:02,5.00,0,0,0".to_owned(),
    ];
    let c = census(&csv(&rows));
    assert_eq!(
        (
            c.lines,
            c.identical,
            c.negative_ltp,
            c.negative_qty,
            c.crossed,
            c.one_sided
        ),
        (5, 1, 1, 1, 1, 1)
    );
    let mut cut = csv(&rows[..1]);
    cut.extend_from_slice(rows[0].as_bytes());
    assert_eq!(
        census(&cut).lines,
        2,
        "an unterminated last line is a line too"
    );
    assert_eq!(census(&cut).identical, 1);
}

#[test]
fn compare_names_each_status_and_every_differing_field() {
    let t = sig_of(&[bar(1_000_000, (5, 6, 4, 5), 2)]);
    let a = sig_of(&[bar(1_000_000, (5, 7, 4, 5), 2)]);
    let empty = Sig {
        hash: SigBuilder::new().finish().hash,
        ..Sig::default()
    };
    assert_eq!(compare(&t, &a, &t), "exact_T");
    assert_eq!(compare(&t, &a, &a), "exact_A");
    assert_eq!(compare(&t, &a, &empty), "raw_only");
    assert_eq!(compare(&empty, &empty, &empty), "exact_T");
    let other = sig_of(&[bar(2_000_000, (5, 6, 4, 5), 3)]);
    assert_eq!(compare(&t, &a, &other), "differs:volume+span+hash");
    assert_eq!(
        compare(&empty, &empty, &t),
        "differs:seconds+open+high+low+close+volume+span+hash"
    );
    assert_eq!(status_word("exact_T", "special:Muhurat"), "special_session");
    assert_eq!(
        status_word("exact_A", "special:BudgetWeekend"),
        "special_session"
    );
    assert_eq!(
        status_word("differs:hash", "special:Muhurat"),
        "differs:hash"
    );
    assert_eq!(status_word("exact_T", "regular"), "exact_T");
    assert_eq!(status_word(FOLDED_COMPARE, "regular"), FOLDED_STATUS);
    let one = bar(1_000_000, (5, 6, 4, 5), 2);
    let two = bar(2_000_000, (5, 6, 4, 5), 2);
    assert_ne!(
        sig_of(&[one, two]).hash,
        sig_of(&[two, one]).hash,
        "the order of seconds is part of the digest"
    );
}

#[test]
fn rule_a_widens_only_traded_seconds_and_the_fill_measure_counts_each_worse_second_once() {
    let t = [
        bar(1_000_000, (10, 10, 10, 10), 1),
        bar(3_000_000, (20, 21, 19, 20), 2),
    ];
    let every = [
        bar(1_000_000, (10, 12, 9, 11), 0),
        bar(2_000_000, (50, 50, 50, 50), 0),
        bar(3_000_000, (20, 21, 19, 20), 0),
    ];
    let mut c = Counts::default();
    let a = rule_a(&t, &every, &mut c);
    assert_eq!(a.len(), 2, "no bar for the untraded second");
    assert_eq!(
        (a[0].open, a[0].high, a[0].low, a[0].close, a[0].volume),
        (10, 12, 9, 10, 1)
    );
    assert_eq!(a[1], t[1]);
    assert_eq!((c.a_high_widened, c.a_low_widened), (1, 1));
    // Against the stored rule-T bars: one second worse both ways (counted
    // once), one equal; a stored second rule A lacks is not compared.
    let stored = [t[0], bar(2_500_000, (1, 1, 1, 1), 1), t[1]];
    assert_eq!(
        fill_worse(&stored, &a),
        Worse {
            compared: 2,
            high: 1,
            low: 1,
            either: 1
        }
    );
    assert_eq!(
        fill_worse(&a, &a),
        Worse {
            compared: 2,
            ..Worse::default()
        }
    );
    assert_eq!(fill_worse(&[], &a), Worse::default());
}

#[test]
fn minutes_merge_in_one_pass_and_count_only_shared_minutes() {
    let s = |sec: i64, h: i64, l: i64| bar(sec * 1_000_000, (l, h, l, l), 1);
    let ours = minutes_of(&[s(60, 5, 4), s(61, 7, 3), s(180, 9, 9)]);
    assert_eq!(ours.len(), 2);
    assert_eq!((ours[0].high, ours[0].low), (7, 3));
    let zerodha = [s(0, 1, 1), s(60, 7, 4), s(120, 1, 1), s(180, 9, 9)];
    let m = match_minutes(&zerodha, &ours);
    assert_eq!((m.compared, m.high, m.low), (2, 2, 1));
    assert_eq!(match_minutes(&[], &ours), Match::default());
}

#[test]
fn the_histogram_answers_exact_percentiles_without_a_sort() {
    let mut h = Histogram::default();
    assert_eq!((h.at(500), h.max(), h.len()), (0, 0, 0));
    for v in (1..=1_000_u64).rev() {
        h.add(v);
    }
    assert_eq!(
        (h.at(500), h.at(990), h.max(), h.len()),
        (500, 990, 1_000, 1_000)
    );
    assert_eq!(per_mille(1, 3), 333);
    assert_eq!(per_mille(5, 0), 0);
    assert_eq!(h.line(), "n=1000 p50=500 p99=990 max=1000");
}

#[test]
fn words_and_fields_are_one_csv_cell_each() {
    assert_eq!(word_of("TickerMismatch { line: 3 }"), "TickerMismatch");
    assert_eq!(word_of("(x)"), "unnamed");
    assert_eq!(clean("a,b\r\nc"), "a;b  c");
    let label = Label {
        role: Role::Extra,
        source: Source::NotTheLandingSource,
        source_name: "/a,b.zip!x".to_owned(),
        landing: String::new(),
    };
    let line = csv_line(&Row::blank("X", d(D1), "f,g", label), "unstamped");
    assert_eq!(HEADER.split(',').count(), line.split(',').count());
    let cells = Cells::of(&line).unwrap();
    assert_eq!(
        (cells.get("role"), cells.get("source_name")),
        ("extra", "/a;b.zip!x")
    );
    assert_eq!(cells.get("no_such_column"), "");
    assert!(Cells::of("a,b").is_err());
    assert_eq!(session_word(d(D1)), "regular");
    assert_eq!(session_word(d(MUHURAT)), "special:Muhurat");
    assert_eq!(days_of_month(YearMonth::new(2024, 2).unwrap()).len(), 29);
    assert_eq!(parse_day("2025-03-07"), Some(d(D1)));
    assert_eq!(parse_day("2025-02-30"), None);
    assert_eq!(parse_day("x"), None);
    assert_eq!(index::year_span(2024).unwrap().1, 366);
}

#[test]
fn parallel_answers_in_index_order_whatever_the_worker_count() {
    for workers in [1, 2, 7, 40] {
        let got = parallel(25, workers, |i| i * 3).unwrap();
        assert_eq!(
            got,
            (0..25).map(|i| i * 3).collect::<Vec<_>>(),
            "workers={workers}"
        );
    }
    assert!(parallel(0, 4, |i| i).unwrap().is_empty());
}

#[test]
fn a_store_that_holds_nothing_makes_no_version_and_says_so() {
    let root = scratch("empty");
    let store = root.join("store");
    std::fs::create_dir_all(&store).unwrap();
    let (code, out, dir) = check(&root, &store, &none, &["2019"]);
    assert_eq!(code, OK, "{out}");
    assert!(out.contains("0 stored share-days"), "{out}");
    assert!(out.contains("2019 nothing new to fold"), "{out}");
    assert!(!dir.join("tick-to-second-2019.v001.csv").exists());
    // A share the store does not hold is named, never a silent empty run.
    let (code, out, _) = check(
        &root,
        &store,
        &none,
        &["2019", "--symbols", "ZZNOPE,ZZNADA"],
    );
    assert_eq!(code, OK, "{out}");
    assert!(
        out.contains(
            "  --symbols ZZNADA: the store's GDFL share tree holds no such share\n  --symbols \
             ZZNOPE: the store's GDFL share tree holds no such share"
        ),
        "named, in order: {out}"
    );
}

#[test]
fn the_auction_volume_is_every_identical_row_of_every_print() {
    let print = |quantity: u64, rows: u32| pull::gdfl_cm::auction::Print {
        sod: at(15, 30, 0),
        price: 999_900,
        quantity,
        rows,
        line: 2,
    };
    assert_eq!(auction_volume(&[]), 0);
    assert_eq!(auction_volume(&[print(7, 1), print(5, 3)]), 22);
    assert_eq!(
        auction_volume(&[print(u64::MAX, 2)]),
        u64::MAX,
        "saturates, never wraps"
    );
}

#[test]
fn the_restored_prints_rejoin_the_rows_in_file_order_each_identical_row_counted() {
    let kept = |line: u32, sod: u32, ltp: i64, ltq: u64| pull::gdfl_cm::CmRow {
        line,
        sod,
        ltp,
        ltq,
    };
    let print = |line: u32, sod: u32, rows: u32| pull::gdfl_cm::auction::Print {
        sod,
        price: 999_900,
        quantity: 4,
        rows,
        line,
    };
    let rows = [
        kept(2, at(10, 0, 0), 777_700, 1),
        kept(3, at(10, 0, 1), 777_800, 0),
        kept(6, at(15, 29, 2), 777_900, 2),
    ];
    let ticks = with_prints(
        &rows,
        &[print(4, at(15, 29, 0), 2), print(7, at(15, 29, 5), 1)],
    );
    let seen: Vec<(u32, i64, u64)> = ticks.iter().map(|t| (t.sod, t.ltp, t.ltq)).collect();
    assert_eq!(
        seen,
        vec![
            (at(10, 0, 0), 777_700, 1),
            (at(10, 0, 1), 777_800, 0),
            (at(15, 29, 0), 999_900, 4),
            (at(15, 29, 0), 999_900, 4),
            (at(15, 29, 2), 777_900, 2),
            (at(15, 29, 5), 999_900, 4),
        ]
    );
    assert!(
        ticks.iter().all(|t| t.oi.is_none()),
        "a share carries no open interest"
    );
    assert_eq!(
        with_prints(&rows, &[]).len(),
        rows.len(),
        "no print, the rows alone"
    );
}

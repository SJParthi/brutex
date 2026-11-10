//! Extreme cases of the GDFL capital-market import, end to end from zip bytes
//! to stored one-second bars, for the cloud extremes round of 10 Nov 2026
//! (`EXTREMES-REPORT.md` at the repository root).
//!
//! Public API only: an outer archive is built in memory, read through
//! `pull::gdfl_archive::Archive`, imported by `pull::gdfl_import::run_cm`, and
//! the bars are read back through the store's reader door. What the existing
//! tests stop short of, and this file adds:
//!
//! * a stock day with a trade in EVERY second of the session, and a file one
//!   row past the reader's row cap;
//! * a day zip with no member, an empty member, a header-only member;
//! * a member cut inside its deflate stream, and one whose stated CRC lies,
//!   each followed by the resume that must land the clean store;
//! * the Budget Saturday with a duplicate row inside a cut member, the 2024
//!   Muhurat evening with out-of-order rows, the DR drill's two windows;
//! * a weekend and a holiday whose vendor zip holds rows;
//! * the IST year boundary and the leap day through the importer;
//! * a rerun after the import journal is lost;
//! * every spelling of a last traded price the reader may meet;
//! * a stock day of quotes alone;
//! * the chained rename `LTI -> LTIM -> LTM` on the days either side of it.
//!
//! Every value is invented at run time; no vendor row is quoted (rows are
//! built by `row`, so `gdfl_cm`'s pasted-row scan finds none here). Tickers
//! are real share and index names only because the import admits nothing
//! else. A test named `finding_*` is `#[ignore]`d because it FAILS on the
//! branch it was written against; its doc comment is the finding.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]

use std::collections::BTreeMap;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use brutex_core::instrument::{Exchange, InstrumentKey};
use brutex_core::vendor::Vendor;
use pull::gdfl_archive::Archive;
use pull::gdfl_import::{ImportKind, Report, Run, journal_path, run_cm};
use pull::session::Day;
use store::file::{BarFile, StoreError};
use store::format::Bar;
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

// ===========================================================================
// Fixtures
// ===========================================================================

/// A scratch directory unique to `tag` and this process, empty.
fn scratch(tag: &str) -> PathBuf {
    static SERIAL: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let serial = SERIAL.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("brutex-xg-{tag}-{}-{serial}", std::process::id()));
    let _absent = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn day(y: u16, m: u8, d: u8) -> Day {
    Day::new(y, m, d).unwrap()
}

/// `HH:MM:SS` of a second of the day.
fn hms(sod: u32) -> String {
    format!("{:02}:{:02}:{:02}", sod / 3_600, sod / 60 % 60, sod % 60)
}

/// The second of the day of `h:m:s`.
const fn at(h: u32, m: u32, s: u32) -> u32 {
    h * 3_600 + m * 60 + s
}

/// 09:15:00 and 15:30:00, the regular session's edges.
const OPEN: u32 = at(9, 15, 0);
const CLOSE: u32 = at(15, 30, 0);

/// One invented row: `stem,DD/MM/YYYY,HH:MM:SS,ltp,0,0,0,0,ltq,0`.
fn row(stem: &str, d: Day, sod: u32, ltp: &str, ltq: u64) -> String {
    format!(
        "{stem},{:02}/{:02}/{:04},{},{ltp},0,0,0,0,{ltq},0",
        d.day(),
        d.month(),
        d.year(),
        hms(sod)
    )
}

/// A whole file: the modern header and `rows`, CRLF, ending in one.
fn csv(rows: &[String]) -> Vec<u8> {
    let mut text = format!("{}\r\n", pull::gdfl_cm::HEADER_OPEN_INTEREST);
    for r in rows {
        text.push_str(r);
        text.push_str("\r\n");
    }
    text.into_bytes()
}

/// Paisa as the vendor's two-decimal text.
fn rupees(paisa: i64) -> String {
    format!("{}.{:02}", paisa / 100, paisa % 100)
}

/// One zip member as written: its packed bytes and what the directory says.
struct Member {
    name: String,
    packed: Vec<u8>,
    method: u16,
    crc: u32,
    size: u32,
}

impl Member {
    /// A well-formed member of `data`, deflated when `deflate`.
    fn of(name: &str, data: &[u8], deflate: bool) -> Self {
        let packed = if deflate {
            let mut enc =
                flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
            enc.write_all(data).unwrap();
            enc.finish().unwrap()
        } else {
            data.to_vec()
        };
        Self {
            name: name.to_owned(),
            packed,
            method: if deflate { 8 } else { 0 },
            crc: pull::gdfl_archive::crc32(data),
            size: u32::try_from(data.len()).unwrap(),
        }
    }
}

fn le16(out: &mut Vec<u8>, v: u16) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn le32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}

/// A plain zip of `members`, each header saying exactly what the member says
/// about itself, true or not.
fn zip_of(members: &[Member]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut central = Vec::new();
    for m in members {
        let offset = u32::try_from(out.len()).unwrap();
        let packed = u32::try_from(m.packed.len()).unwrap();
        let name = u16::try_from(m.name.len()).unwrap();
        le32(&mut out, 0x0403_4b50);
        le16(&mut out, 20);
        le16(&mut out, 0);
        le16(&mut out, m.method);
        le32(&mut out, 0);
        le32(&mut out, m.crc);
        le32(&mut out, packed);
        le32(&mut out, m.size);
        le16(&mut out, name);
        le16(&mut out, 0);
        out.extend_from_slice(m.name.as_bytes());
        out.extend_from_slice(&m.packed);
        le32(&mut central, 0x0201_4b50);
        le16(&mut central, 20);
        le16(&mut central, 20);
        le16(&mut central, 0);
        le16(&mut central, m.method);
        le32(&mut central, 0);
        le32(&mut central, m.crc);
        le32(&mut central, packed);
        le32(&mut central, m.size);
        le16(&mut central, name);
        for _ in 0..4 {
            le16(&mut central, 0);
        }
        le32(&mut central, 0);
        le32(&mut central, offset);
        central.extend_from_slice(m.name.as_bytes());
    }
    let at = u32::try_from(out.len()).unwrap();
    let count = u16::try_from(members.len()).unwrap();
    let size = u32::try_from(central.len()).unwrap();
    out.extend_from_slice(&central);
    le32(&mut out, 0x0605_4b50);
    le16(&mut out, 0);
    le16(&mut out, 0);
    le16(&mut out, count);
    le16(&mut out, count);
    le32(&mut out, size);
    le32(&mut out, at);
    le16(&mut out, 0);
    out
}

const MONTHS: [&str; 12] = [
    "JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC",
];

/// Which capital-market tree.
#[derive(Clone, Copy)]
enum Tree {
    Stocks,
    Indices,
}

impl Tree {
    const fn folder_prefix(self) -> &'static str {
        match self {
            Self::Stocks => "GFDLCM_STOCK_TICK_",
            Self::Indices => "GFDLCM_INDICES_TICK_",
        }
    }
    const fn top(self) -> &'static str {
        match self {
            Self::Stocks => "STOCKS",
            Self::Indices => "INDICES",
        }
    }
}

/// The day folder of `tree` on `d`.
fn folder(tree: Tree, d: Day) -> String {
    format!(
        "{}{:02}{:02}{:04}",
        tree.folder_prefix(),
        d.day(),
        d.month(),
        d.year()
    )
}

/// One day of `tree`: its member files, each `(file name, member)`.
struct DayZip {
    tree: Tree,
    day: Day,
    members: Vec<Member>,
}

impl DayZip {
    /// A day whose files are well-formed deflated members.
    fn of(tree: Tree, d: Day, files: &[(String, Vec<u8>)]) -> Self {
        let folder = folder(tree, d);
        Self {
            tree,
            day: d,
            members: files
                .iter()
                .map(|(name, bytes)| Member::of(&format!("{folder}/{name}"), bytes, true))
                .collect(),
        }
    }
}

/// An outer archive holding one stored day zip per `DayZip`.
fn archive(days: Vec<DayZip>) -> Archive<Vec<u8>> {
    let outer: Vec<Member> = days
        .into_iter()
        .map(|dz| {
            let folder = folder(dz.tree, dz.day);
            let inner = zip_of(&dz.members);
            let month = MONTHS[usize::from(dz.day.month()) - 1];
            let name = format!(
                "{}/{}/{month}_{}/{folder}.zip",
                dz.tree.top(),
                dz.day.year(),
                dz.day.year()
            );
            Member::of(&name, &inner, false)
        })
        .collect();
    let bytes = zip_of(&outer);
    let len = u64::try_from(bytes.len()).unwrap();
    Archive::from_source(bytes, len).unwrap()
}

/// A stocks run over `from..=to` of `archive`, filtered to `only`.
fn import(
    archive: &Archive<Vec<u8>>,
    kind: ImportKind,
    (from, to): (Day, Day),
    store: &Path,
    only: &[String],
) -> Report {
    let run = Run {
        kind,
        from,
        to,
        only,
        store_root: store,
    };
    run_cm(archive, &run).unwrap()
}

fn only(symbol: &str) -> Vec<String> {
    vec![symbol.to_owned()]
}

/// The one-second bars the store holds for `symbol` in `segment` in `ym`.
fn stored(store: &Path, segment: &str, symbol: &str, ym: (u16, u8)) -> Vec<Bar> {
    let path = StorePath::new(PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment,
        symbol,
        contract: None,
        timeframe: Timeframe::SECOND_1,
        month: YearMonth::new(ym.0, ym.1).unwrap(),
        file: FileKind::Bars,
    })
    .unwrap();
    match BarFile::open_existing(store, path, pull::ingest::repair_plan::symbol_id(symbol)) {
        Ok(file) => (0..file.records())
            .map(|i| file.read_record(i).unwrap())
            .collect(),
        Err(StoreError::Missing { .. }) => Vec::new(),
        Err(other) => panic!("the store refused {symbol} {ym:?}: {other}"),
    }
}

/// Every bar file under `store`, by relative path, with its bytes.
fn bar_files(store: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut out = BTreeMap::new();
    let mut stack = vec![store.join("bars")];
    while let Some(at) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&at) else {
            continue;
        };
        for entry in entries {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "bin" || e == "crc") {
                let rel = path.strip_prefix(store).unwrap().to_path_buf();
                out.insert(rel, std::fs::read(&path).unwrap());
            }
        }
    }
    out
}

fn journal(store: &Path) -> String {
    std::fs::read_to_string(journal_path(store)).unwrap_or_default()
}

/// Whether the journal's last word on `kind d` is a `done` close.
fn closed_done(store: &Path, kind: &str, d: Day) -> bool {
    let key = format!(" {kind} {d} ");
    journal(store)
        .lines()
        .rfind(|l| l.contains(&key))
        .is_some_and(|l| l.starts_with("done "))
}

/// The IST epoch microseconds of second `sod` of `d`.
fn micros(d: Day, sod: u32) -> i64 {
    (i64::from(d.days_from_epoch()) * 86_400 + i64::from(sod) - 19_800) * 1_000_000
}

/// What the import must make of one stock's rows: a bar per second holding a
/// traded row (LTQ > 0), open the first, close the last, volume summed.
fn expected_bars(d: Day, rows: &[(u32, i64, u64)]) -> Vec<Bar> {
    let mut by_second: BTreeMap<u32, Vec<(i64, u64)>> = BTreeMap::new();
    for &(sod, ltp, ltq) in rows {
        if ltq > 0 && (OPEN..CLOSE).contains(&sod) {
            by_second.entry(sod).or_default().push((ltp, ltq));
        }
    }
    by_second
        .into_iter()
        .map(|(sod, trades)| Bar {
            ts_micros: micros(d, sod),
            open: trades[0].0,
            high: trades.iter().map(|t| t.0).max().unwrap(),
            low: trades.iter().map(|t| t.0).min().unwrap(),
            close: trades[trades.len() - 1].0,
            volume: i64::try_from(trades.iter().map(|t| t.1).sum::<u64>()).unwrap(),
            open_interest: store::format::OI_NULL,
        })
        .collect()
}

/// The rows of a RELIANCE day: a trade every `every` seconds of the session
/// at a price that walks, and a quote (LTQ 0) in every other second.
fn reliance_rows(d: Day, every: u32) -> (Vec<String>, Vec<(u32, i64, u64)>) {
    let mut text = Vec::new();
    let mut plain = Vec::new();
    for sod in OPEN..CLOSE {
        let n = sod - OPEN;
        let price = 250_000 + i64::from(n % 997) * 5;
        let (ltq, ltp) = if n.is_multiple_of(every) {
            (1 + u64::from(n % 5), price)
        } else {
            (0, price)
        };
        text.push(row("RELIANCE.NSE", d, sod, &rupees(ltp), ltq));
        plain.push((sod, ltp, ltq));
    }
    (text, plain)
}

fn reliance_file(d: Day, every: u32) -> (String, Vec<u8>) {
    (
        "RELIANCE.NSE.csv".to_owned(),
        csv(&reliance_rows(d, every).0),
    )
}

// ===========================================================================
// Size: every second, and one row past the cap
// ===========================================================================

/// A stock day with a trade in every second of the session and a second trade
/// in every tenth (47,250 rows) imports exactly 22,500 one-second bars, each
/// built as the rule says.
#[test]
fn a_stock_day_with_a_trade_in_every_second_imports_every_second_exactly() {
    let root = scratch("EverySecond");
    let d = day(2024, 6, 4);
    let mut text = Vec::new();
    let mut plain = Vec::new();
    for sod in OPEN..CLOSE {
        let n = sod - OPEN;
        let price = 120_000 + i64::from(n % 401) * 5;
        text.push(row("RELIANCE.NSE", d, sod, &rupees(price), 2));
        plain.push((sod, price, 2));
        if n.is_multiple_of(10) {
            text.push(row("RELIANCE.NSE", d, sod, &rupees(price + 15), 3));
            plain.push((sod, price + 15, 3));
        }
        text.push(row("RELIANCE.NSE", d, sod, &rupees(price + 5), 0));
    }
    assert_eq!(text.len(), 47_250);
    let files = [("RELIANCE.NSE.csv".to_owned(), csv(&text))];
    let source = archive(vec![DayZip::of(Tree::Stocks, d, &files)]);
    let store = root.join("S");
    let got = import(
        &source,
        ImportKind::Stocks,
        (d, d),
        &store,
        &only("RELIANCE"),
    );
    assert!(got.failures.is_empty(), "{:?}", got.failures);
    assert_eq!(got.seconds, 22_500);
    let bars = stored(&store, "CASH", "RELIANCE", (2024, 6));
    assert_eq!(bars, expected_bars(d, &plain));
    assert!(closed_done(&store, "stocks", d), "{}", journal(&store));
    std::fs::remove_dir_all(root).unwrap();
}

/// A file one row past the reader's row cap (1,000,001 rows) is refused by
/// name, and not one of its bars is stored.
#[test]
fn a_file_one_row_past_the_row_cap_is_refused_by_name_and_stores_nothing() {
    let root = scratch("RowCap");
    let d = day(2024, 6, 5);
    let mut text = String::with_capacity(60_000_000);
    text.push_str(pull::gdfl_cm::HEADER_OPEN_INTEREST);
    text.push_str("\r\n");
    let date = format!("{:02}/{:02}/{:04}", d.day(), d.month(), d.year());
    for i in 0..1_000_001_u32 {
        let sod = OPEN + i % (CLOSE - OPEN);
        let _infallible = std::fmt::Write::write_fmt(
            &mut text,
            format_args!("RELIANCE.NSE,{date},{},1234.50,0,0,0,0,1,0\r\n", hms(sod)),
        );
    }
    let files = [("RELIANCE.NSE.csv".to_owned(), text.into_bytes())];
    let source = archive(vec![DayZip::of(Tree::Stocks, d, &files)]);
    let store = root.join("S");
    let got = import(
        &source,
        ImportKind::Stocks,
        (d, d),
        &store,
        &only("RELIANCE"),
    );
    assert!(
        got.failures
            .iter()
            .any(|f| f.instrument.contains("RELIANCE")),
        "the file is refused by name: {:?}",
        got.failures
    );
    assert!(stored(&store, "CASH", "RELIANCE", (2024, 6)).is_empty());
    std::fs::remove_dir_all(root).unwrap();
}

// ===========================================================================
// Empty containers and members
// ===========================================================================

/// FINDING (GDFL import, a day zip with no member).
///
/// Input: the stocks day zip of 2024-06-06 is a valid zip holding NO entry
/// (a vendor export that wrote nothing, or a re-zip that dropped the folder).
///
/// Expected: the day is not closed `done`: no file was read, so the day is
/// refused or left open and read again by the next run, and its absence is
/// named.
///
/// Actual: see the assertion. The day closes `done files=0 ... entries=0`
/// (`docs/06-limits.md` names this as L-IA16-08): `cm_jobs` makes no job and
/// `day_close_refusals` needs at least one entry to refuse, so `close_of`
/// closes an empty day as a clean one (`crates/pull/src/gdfl_import.rs`
/// 2333-2335 and 2364-2373). Every later run skips the day as done, and the
/// store holds no bar of it with nothing saying why.
///
/// Proposed fix: close a day whose listing holds no entry as `refused
/// reason=day_zip_empty` (an open cause, like `day_not_held`), so it is read
/// again and named in the run's refusals.
#[test]
#[ignore = "finding: an empty day zip closes the day done (EXTREMES-REPORT.md)"]
fn finding_a_day_zip_with_no_member_closes_its_day_done() {
    let root = scratch("EmptyDayZip");
    let d = day(2024, 6, 6);
    let source = archive(vec![DayZip::of(Tree::Stocks, d, &[])]);
    let store = root.join("S");
    let got = import(
        &source,
        ImportKind::Stocks,
        (d, d),
        &store,
        &only("RELIANCE"),
    );
    let done = closed_done(&store, "stocks", d);
    assert!(
        !done && got.days_imported == 0,
        "an empty day zip must not close the day done: imported {}, journal:\n{}",
        got.days_imported,
        journal(&store)
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// A zero-byte member is refused by name, its day is left open, and the rerun
/// over a good archive lands exactly the store a clean import lands.
#[test]
fn an_empty_member_is_refused_by_name_and_the_rerun_lands_the_clean_store() {
    let root = scratch("EmptyMember");
    let d = day(2024, 6, 7);
    let bad = archive(vec![DayZip::of(
        Tree::Stocks,
        d,
        &[("RELIANCE.NSE.csv".to_owned(), Vec::new())],
    )]);
    let store = root.join("S");
    let got = import(&bad, ImportKind::Stocks, (d, d), &store, &only("RELIANCE"));
    assert!(!got.failures.is_empty(), "the empty member is named");
    assert!(!closed_done(&store, "stocks", d), "{}", journal(&store));
    assert!(stored(&store, "CASH", "RELIANCE", (2024, 6)).is_empty());

    let good = archive(vec![DayZip::of(Tree::Stocks, d, &[reliance_file(d, 3)])]);
    let again = import(&good, ImportKind::Stocks, (d, d), &store, &only("RELIANCE"));
    assert!(again.failures.is_empty(), "{:?}", again.failures);
    let clean = root.join("Clean");
    let _clean = import(&good, ImportKind::Stocks, (d, d), &clean, &only("RELIANCE"));
    assert_eq!(bar_files(&store), bar_files(&clean));
    assert!(!bar_files(&clean).is_empty());
    std::fs::remove_dir_all(root).unwrap();
}

/// A member holding the header and no row stores no bar and names the day's
/// refusal; nothing is invented for it.
#[test]
fn a_header_only_member_stores_nothing_and_names_its_refusal() {
    let root = scratch("HeaderOnly");
    let d = day(2024, 6, 10);
    let source = archive(vec![DayZip::of(
        Tree::Stocks,
        d,
        &[("RELIANCE.NSE.csv".to_owned(), csv(&[]))],
    )]);
    let store = root.join("S");
    let got = import(
        &source,
        ImportKind::Stocks,
        (d, d),
        &store,
        &only("RELIANCE"),
    );
    assert!(stored(&store, "CASH", "RELIANCE", (2024, 6)).is_empty());
    let named = !got.failures.is_empty()
        || got.closed_with_refusals.contains(&d)
        || got.closed_refused.contains(&d);
    assert!(named, "the empty file is named: {got:?}");
    std::fs::remove_dir_all(root).unwrap();
}

// ===========================================================================
// Damaged members, and the resume after them
// ===========================================================================

/// A deflated member cut inside its stream, whose directory states the cut
/// length and the true CRC and size, is refused before any bar lands; the
/// rerun over the good archive lands the clean store.
#[test]
fn a_member_cut_inside_its_deflate_stream_is_refused_and_the_rerun_lands_clean() {
    let root = scratch("CutDeflate");
    let d = day(2024, 6, 11);
    let (name, bytes) = reliance_file(d, 2);
    let folder = folder(Tree::Stocks, d);
    let mut cut = Member::of(&format!("{folder}/{name}"), &bytes, true);
    let half = cut.packed.len() / 2;
    cut.packed.truncate(half);
    let bad = archive(vec![DayZip {
        tree: Tree::Stocks,
        day: d,
        members: vec![cut],
    }]);
    let store = root.join("S");
    let got = import(&bad, ImportKind::Stocks, (d, d), &store, &only("RELIANCE"));
    assert!(!got.failures.is_empty(), "the cut member is named");
    assert!(stored(&store, "CASH", "RELIANCE", (2024, 6)).is_empty());
    assert!(!closed_done(&store, "stocks", d), "{}", journal(&store));

    let good = archive(vec![DayZip::of(Tree::Stocks, d, &[(name, bytes)])]);
    let again = import(&good, ImportKind::Stocks, (d, d), &store, &only("RELIANCE"));
    assert!(again.failures.is_empty(), "{:?}", again.failures);
    let clean = root.join("Clean");
    let _clean = import(&good, ImportKind::Stocks, (d, d), &clean, &only("RELIANCE"));
    assert_eq!(bar_files(&store), bar_files(&clean));
    std::fs::remove_dir_all(root).unwrap();
}

/// A member whose stated CRC-32 is one off is refused before decode, at the
/// import level and from a zip (the existing import-level CRC test reads a
/// tick store); the rerun lands the clean store.
#[test]
fn a_member_whose_stated_crc_lies_is_refused_and_the_rerun_lands_clean() {
    let root = scratch("CrcLies");
    let d = day(2024, 6, 12);
    let (name, bytes) = reliance_file(d, 4);
    let folder = folder(Tree::Stocks, d);
    let mut lying = Member::of(&format!("{folder}/{name}"), &bytes, true);
    lying.crc = lying.crc.wrapping_add(1);
    let bad = archive(vec![DayZip {
        tree: Tree::Stocks,
        day: d,
        members: vec![lying],
    }]);
    let store = root.join("S");
    let got = import(&bad, ImportKind::Stocks, (d, d), &store, &only("RELIANCE"));
    assert!(!got.failures.is_empty(), "the lying member is named");
    assert!(stored(&store, "CASH", "RELIANCE", (2024, 6)).is_empty());
    assert!(!closed_done(&store, "stocks", d));

    let good = archive(vec![DayZip::of(Tree::Stocks, d, &[(name, bytes)])]);
    let again = import(&good, ImportKind::Stocks, (d, d), &store, &only("RELIANCE"));
    assert!(again.failures.is_empty(), "{:?}", again.failures);
    let clean = root.join("Clean");
    let _clean = import(&good, ImportKind::Stocks, (d, d), &clean, &only("RELIANCE"));
    assert_eq!(bar_files(&store), bar_files(&clean));
    std::fs::remove_dir_all(root).unwrap();
}

/// An index file cut at a ROW BOUNDARY before the close (every row whole, the
/// zip consistent: to the reader a complete file that stops at 12:00) is
/// refused by name and stores nothing, because an index publishes to the
/// close; the whole file, read again under `retry_refused`, then imports
/// every second of the session, and the 15:30:00 row that proves the close
/// is not a bar.
#[test]
fn an_index_file_cut_at_a_row_boundary_is_refused_then_the_whole_file_lands() {
    let root = scratch("IndexCut");
    let d = day(2024, 6, 27);
    let stem = "NIFTY 50.NSE_IDX";
    let prices: Vec<(u32, i64)> = (OPEN..=CLOSE)
        .map(|sod| (sod, 2_300_000 + i64::from((sod - OPEN) % 113) * 5))
        .collect();
    let file_upto = |upto: u32| {
        let rows: Vec<String> = prices
            .iter()
            .filter(|p| p.0 <= upto)
            .map(|&(sod, paisa)| row(stem, d, sod, &rupees(paisa), 0))
            .collect();
        (format!("{stem}.csv"), csv(&rows))
    };
    let store = root.join("S");
    let cut = archive(vec![DayZip::of(
        Tree::Indices,
        d,
        &[file_upto(at(12, 0, 0))],
    )]);
    let got = import(&cut, ImportKind::Indices, (d, d), &store, &only("NIFTY"));
    assert!(
        !got.failures.is_empty(),
        "the short index file is named: {got:?}"
    );
    assert!(stored(&store, "INDEX", "NIFTY", (2024, 6)).is_empty());

    let whole = archive(vec![DayZip::of(Tree::Indices, d, &[file_upto(CLOSE)])]);
    let run = Run {
        kind: ImportKind::Indices,
        from: d,
        to: d,
        only: &only("NIFTY"),
        store_root: &store,
    };
    let options = pull::gdfl_import::RunOptions {
        retry_refused: true,
        ..pull::gdfl_import::RunOptions::default()
    };
    let again = pull::gdfl_import::run_cm_from(&whole, &run, options).unwrap();
    assert!(again.failures.is_empty(), "{:?}", again.failures);
    let bars = stored(&store, "INDEX", "NIFTY", (2024, 6));
    assert_eq!(bars.len(), 22_500, "09:15:00 to 15:29:59, one bar a second");
    assert!(
        bars.iter().all(|b| b.volume == 0),
        "an index trades no volume"
    );
    assert_eq!(bars[0].ts_micros, micros(d, OPEN));
    assert_eq!(bars[22_499].ts_micros, micros(d, CLOSE - 1));
    std::fs::remove_dir_all(root).unwrap();
}

// ===========================================================================
// Special sessions, with permutations
// ===========================================================================

/// The Budget Saturday 2025-02-01, permuted: its member carries a duplicated
/// row AND is cut inside its deflate stream. The cut file is refused and the
/// day stays open; the whole file then imports, every bar inside the
/// session, the duplicated second's volume summed as the rule says.
#[test]
fn a_budget_saturday_with_a_duplicate_inside_a_cut_member_is_refused_then_lands_whole() {
    let root = scratch("BudgetCutDup");
    let d = day(2025, 2, 1);
    let (mut text, mut plain) = reliance_rows(d, 7);
    let dup_at = 700;
    text.insert(dup_at, text[dup_at].clone());
    plain.insert(dup_at, plain[dup_at]);
    let bytes = csv(&text);
    let folder = folder(Tree::Stocks, d);
    let name = "RELIANCE.NSE.csv".to_owned();
    let mut cut = Member::of(&format!("{folder}/{name}"), &bytes, true);
    let keep = cut.packed.len() * 2 / 3;
    cut.packed.truncate(keep);
    let store = root.join("S");
    let bad = archive(vec![DayZip {
        tree: Tree::Stocks,
        day: d,
        members: vec![cut],
    }]);
    let got = import(&bad, ImportKind::Stocks, (d, d), &store, &only("RELIANCE"));
    assert!(!got.failures.is_empty());
    assert!(!closed_done(&store, "stocks", d));
    assert!(stored(&store, "CASH", "RELIANCE", (2025, 2)).is_empty());

    let good = archive(vec![DayZip::of(Tree::Stocks, d, &[(name, bytes)])]);
    let again = import(&good, ImportKind::Stocks, (d, d), &store, &only("RELIANCE"));
    assert!(again.failures.is_empty(), "{:?}", again.failures);
    assert!(closed_done(&store, "stocks", d), "{}", journal(&store));
    let bars = stored(&store, "CASH", "RELIANCE", (2025, 2));
    assert_eq!(bars, expected_bars(d, &plain));
    let dup_second = micros(d, plain[dup_at].0);
    let doubled = bars.iter().find(|b| b.ts_micros == dup_second).unwrap();
    assert_eq!(
        doubled.volume,
        2 * i64::try_from(plain[dup_at].2).unwrap(),
        "a repeated row is two trades of that second"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// The recorded session of a special day, as `(from, to]` seconds of the day.
fn windows_of(d: Day) -> Vec<(u32, u32)> {
    let special = pull::calendar::special_session(i64::from(d.days_from_epoch()))
        .expect("a recorded special session");
    special
        .session
        .windows
        .iter()
        .take(usize::from(special.session.count))
        .map(|w| (u32::from(w.from) * 60, u32::from(w.to) * 60 + 59))
        .collect()
}

/// The 2024 Muhurat evening: rows before, inside and after its recorded
/// window, one of them out of order. Only bars inside the window are stored,
/// and nothing outside it is invented.
///
/// Two placements of the late row, both by the rule `place_all` states (a
/// late row is placed at the next in-order row's stamp, never back in time,
/// `CLAUDE.md` section 3 rule 7): followed by an in-window row it is counted
/// at that row's second; followed only by the post-window row it is placed
/// after the window and dropped with it, COUNTED as a late row, never
/// silently lost.
#[test]
fn the_2024_muhurat_keeps_only_its_window_with_rows_out_of_order() {
    let d = day(2024, 11, 1);
    let windows = windows_of(d);
    assert_eq!(windows.len(), 1, "one evening window: {windows:?}");
    let (from, to) = windows[0];
    let stem = "RELIANCE.NSE";
    for late_inside in [true, false] {
        let root = scratch("Muhurat2024");
        let mut text = vec![row(stem, d, from - 600, "1300.00", 4)];
        for i in 0..30 {
            text.push(row(
                stem,
                d,
                from + i * 60,
                &rupees(130_000 + i64::from(i) * 5),
                2,
            ));
            if late_inside && i == 25 {
                // Late: stamped a minute before the row ahead of it.
                text.push(row(stem, d, from + 24 * 60 - 60, "1301.25", 1));
            }
        }
        if !late_inside {
            text.push(row(stem, d, from + 25 * 60 - 60, "1301.25", 1));
        }
        text.push(row(stem, d, to + 600, "1302.00", 9));
        let store = root.join("S");
        let source = archive(vec![DayZip::of(
            Tree::Stocks,
            d,
            &[("RELIANCE.NSE.csv".to_owned(), csv(&text))],
        )]);
        let got = import(
            &source,
            ImportKind::Stocks,
            (d, d),
            &store,
            &only("RELIANCE"),
        );
        assert!(got.failures.is_empty(), "{:?}", got.failures);
        assert_eq!(got.late_rows, 1, "the late row is counted: {got:?}");
        let bars = stored(&store, "CASH", "RELIANCE", (2024, 11));
        assert!(!bars.is_empty());
        for bar in &bars {
            assert!(
                bar.ts_micros >= micros(d, from) && bar.ts_micros <= micros(d, to),
                "a bar outside the recorded window: {bar:?}"
            );
        }
        let volume: i64 = bars.iter().map(|b| b.volume).sum();
        let want = if late_inside { 30 * 2 + 1 } else { 30 * 2 };
        assert_eq!(volume, want, "late inside the window: {late_inside}");
        assert!(got.outside_session >= 1, "the post-window row is counted");
        std::fs::remove_dir_all(root).unwrap();
    }
}

/// The 2024-03-02 DR drill: rows in each of its two recorded windows are
/// stored, rows in the gap between them and after the second are not.
#[test]
fn the_dr_drill_keeps_its_two_windows_and_drops_the_gap_between_them() {
    let root = scratch("DrDrill");
    let d = day(2024, 3, 2);
    let windows = windows_of(d);
    assert_eq!(windows.len(), 2, "{windows:?}");
    let stem = "RELIANCE.NSE";
    let inside = [
        windows[0].0 + 60,
        windows[0].1 - 60,
        windows[1].0 + 60,
        windows[1].1 - 60,
    ];
    let outside = [windows[0].1 + 600, windows[1].1 + 600];
    let mut sods: Vec<u32> = inside.iter().chain(outside.iter()).copied().collect();
    sods.sort_unstable();
    let text: Vec<String> = sods
        .iter()
        .map(|&s| row(stem, d, s, "1250.00", 3))
        .collect();
    let store = root.join("S");
    let source = archive(vec![DayZip::of(
        Tree::Stocks,
        d,
        &[("RELIANCE.NSE.csv".to_owned(), csv(&text))],
    )]);
    let got = import(
        &source,
        ImportKind::Stocks,
        (d, d),
        &store,
        &only("RELIANCE"),
    );
    assert!(got.failures.is_empty(), "{:?}", got.failures);
    let held: Vec<i64> = stored(&store, "CASH", "RELIANCE", (2024, 3))
        .iter()
        .map(|b| b.ts_micros)
        .collect();
    let want: Vec<i64> = inside.iter().map(|&s| micros(d, s)).collect();
    assert_eq!(held, want);
    std::fs::remove_dir_all(root).unwrap();
}

/// FINDING (GDFL import, vendor rows on a day the calendar calls closed).
///
/// Input: stocks day zips for Saturday 2024-06-01 (no special session is
/// recorded for it) and for the Independence Day holiday 2024-08-15, each
/// holding a RELIANCE file of in-session trades.
///
/// Expected: the import says the vendor shipped rows for a day the calendar
/// calls closed: a journal line, a failure or a refusal naming the day, so an
/// unrecorded special session (or a calendar error) is seen rather than lost.
///
/// Actual: see the assertion. `admit` counts the day in `days_closed` and
/// returns before the source is opened: no journal line, no failure, no
/// event (`crates/pull/src/gdfl_import_day.rs:614-621`), while both sources
/// file the day's zip (`crates/pull/src/gdfl_dayzips.rs:181-185`). The
/// import cannot tell "closed and empty" from "closed but the vendor traded".
///
/// Proposed fix: on a `Closed` day, ask the source whether it holds the day
/// (a listing is O(1) per day) and, when it does, name it: a warning event and
/// a journal note (`closed_with_vendor_rows`), never an import.
#[test]
#[ignore = "finding: vendor rows on a closed day are skipped without a word (EXTREMES-REPORT.md)"]
fn finding_vendor_rows_on_a_day_the_calendar_calls_closed_leave_no_trace() {
    let root = scratch("ClosedWithRows");
    let saturday = day(2024, 6, 1);
    let holiday = day(2024, 8, 15);
    let mut seen = Vec::new();
    for d in [saturday, holiday] {
        let store = root.join(format!("S{}", d.day()));
        let source = archive(vec![DayZip::of(Tree::Stocks, d, &[reliance_file(d, 5)])]);
        let got = import(
            &source,
            ImportKind::Stocks,
            (d, d),
            &store,
            &only("RELIANCE"),
        );
        let in_journal = journal(&store).contains(&d.to_string());
        let in_report = got.failures.iter().any(|f| f.day == Some(d))
            || got.closed_refused.contains(&d)
            || got.closed_with_refusals.contains(&d)
            || got.days_missing.contains(&d);
        seen.push((d, got.days_closed, in_journal, in_report));
    }
    assert!(
        seen.iter()
            .all(|&(_, _, journal, report)| journal || report),
        "vendor rows on a closed day must be named somewhere: (day, days_closed, \
         journal names it, report names it) = {seen:?}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

// ===========================================================================
// The year boundary and the leap day
// ===========================================================================

/// One run over 2024-12-31 and 2025-01-01 files each session under its own
/// IST month, and every bar of each in the month it belongs to.
#[test]
fn the_year_boundary_files_each_session_under_its_own_ist_month() {
    let root = scratch("YearEnd");
    let last = day(2024, 12, 31);
    let first = day(2025, 1, 1);
    let source = archive(vec![
        DayZip::of(Tree::Stocks, last, &[reliance_file(last, 6)]),
        DayZip::of(Tree::Stocks, first, &[reliance_file(first, 6)]),
    ]);
    let store = root.join("S");
    let got = import(
        &source,
        ImportKind::Stocks,
        (last, first),
        &store,
        &only("RELIANCE"),
    );
    assert!(got.failures.is_empty(), "{:?}", got.failures);
    assert_eq!(got.days_imported, 2);
    let december = stored(&store, "CASH", "RELIANCE", (2024, 12));
    let january = stored(&store, "CASH", "RELIANCE", (2025, 1));
    assert_eq!(december, expected_bars(last, &reliance_rows(last, 6).1));
    assert_eq!(january, expected_bars(first, &reliance_rows(first, 6).1));
    std::fs::remove_dir_all(root).unwrap();
}

/// One run over 2024-02-29 and 2024-03-01 files the leap day in February and
/// the next day in March.
#[test]
fn the_leap_day_session_lands_in_february_and_the_next_in_march() {
    let root = scratch("LeapDay");
    let leap = day(2024, 2, 29);
    let next = day(2024, 3, 1);
    let source = archive(vec![
        DayZip::of(Tree::Stocks, leap, &[reliance_file(leap, 8)]),
        DayZip::of(Tree::Stocks, next, &[reliance_file(next, 8)]),
    ]);
    let store = root.join("S");
    let got = import(
        &source,
        ImportKind::Stocks,
        (leap, next),
        &store,
        &only("RELIANCE"),
    );
    assert!(got.failures.is_empty(), "{:?}", got.failures);
    assert_eq!(
        stored(&store, "CASH", "RELIANCE", (2024, 2)),
        expected_bars(leap, &reliance_rows(leap, 8).1)
    );
    assert_eq!(
        stored(&store, "CASH", "RELIANCE", (2024, 3)),
        expected_bars(next, &reliance_rows(next, 8).1)
    );
    std::fs::remove_dir_all(root).unwrap();
}

// ===========================================================================
// The same input twice
// ===========================================================================

/// A rerun skips every done day and changes no byte; a rerun after the import
/// journal is LOST reads every day again and still changes no byte of any bar
/// file: the store, not the journal, is what keeps a bar from landing twice.
#[test]
fn a_rerun_and_a_rerun_after_the_journal_is_lost_change_no_bar_byte() {
    let root = scratch("JournalLost");
    let days = [day(2024, 6, 24), day(2024, 6, 25), day(2024, 6, 26)];
    let source = archive(
        days.iter()
            .map(|&d| DayZip::of(Tree::Stocks, d, &[reliance_file(d, 3)]))
            .collect(),
    );
    let store = root.join("S");
    let span = (days[0], days[2]);
    let first = import(&source, ImportKind::Stocks, span, &store, &only("RELIANCE"));
    assert!(first.failures.is_empty(), "{:?}", first.failures);
    let before = bar_files(&store);
    assert!(!before.is_empty());

    let again = import(&source, ImportKind::Stocks, span, &store, &only("RELIANCE"));
    assert_eq!(again.days_skipped, 3, "{again:?}");
    assert_eq!(bar_files(&store), before);

    std::fs::remove_file(journal_path(&store)).unwrap();
    let lost = import(&source, ImportKind::Stocks, span, &store, &only("RELIANCE"));
    assert!(lost.failures.is_empty(), "{:?}", lost.failures);
    assert_eq!(lost.seconds_committed, 0, "no second is committed twice");
    assert_eq!(bar_files(&store), before);
    std::fs::remove_dir_all(root).unwrap();
}

// ===========================================================================
// Prices as text, and a day of quotes alone
// ===========================================================================

/// Every spelling of a last traded price the reader may meet is either read
/// to the exact paisa or refused: never read as another number. Valid ones
/// are kept at their value; zero, signs, exponents, words, a third decimal
/// and foreign digits are refused.
#[test]
fn every_ltp_spelling_is_read_exactly_or_refused() {
    let d = day(2024, 6, 20);
    let kept: [(&str, i64); 6] = [
        ("1234.50", 123_450),
        ("1234.5", 123_450),
        ("1234", 123_400),
        ("0.05", 5),
        ("99999.95", 9_999_995),
        ("1.10", 110),
    ];
    let e = 'e';
    let refused: Vec<String> = vec![
        "0".to_owned(),
        "0.00".to_owned(),
        "-1.00".to_owned(),
        "-0.00".to_owned(),
        "+5.00".to_owned(),
        " 5.00".to_owned(),
        "5.00 ".to_owned(),
        "5.001".to_owned(),
        format!("1{e}3"),
        "1E3".to_owned(),
        "NaN".to_owned(),
        format!("in{}", 'f'),
        String::new(),
        "5,00".to_owned(),
        "\u{967}\u{968}.00".to_owned(),
        "5..0".to_owned(),
        ".5".to_owned(),
        "5.".to_owned(),
        format!("0{}10", 'x'),
        "1_000.00".to_owned(),
    ];
    let stem = "RELIANCE.NSE";
    let mut text = Vec::new();
    let mut sod = OPEN + 60;
    for (ltp, _) in &kept {
        text.push(row(stem, d, sod, ltp, 1));
        sod += 1;
    }
    for ltp in &refused {
        text.push(row(stem, d, sod, ltp, 1));
        sod += 1;
    }
    let source = archive(vec![DayZip::of(
        Tree::Stocks,
        d,
        &[("RELIANCE.NSE.csv".to_owned(), csv(&text))],
    )]);
    let key = InstrumentKey::cash(Exchange::Nse, "RELIANCE").unwrap();
    let read = pull::gdfl_cm::read_day(&source, &key, d).unwrap().unwrap();
    let got: Vec<(u32, i64)> = read.file.rows.iter().map(|r| (r.sod, r.ltp)).collect();
    let want: Vec<(u32, i64)> = kept
        .iter()
        .enumerate()
        .map(|(i, &(_, paisa))| (OPEN + 60 + u32::try_from(i).unwrap(), paisa))
        .collect();
    assert_eq!(got, want, "only the valid spellings are kept, each exactly");
    assert_eq!(
        read.file.refused.total(),
        u32::try_from(refused.len()).unwrap(),
        "every other spelling is refused"
    );
}

/// A stock day whose rows are all quotes (LTQ 0) stores no bar: a quote is
/// not a trade and nothing is invented from it.
#[test]
fn a_stock_day_of_quotes_alone_stores_no_bar() {
    let root = scratch("QuotesOnly");
    let d = day(2024, 6, 21);
    let text: Vec<String> = (0..600)
        .map(|i| row("RELIANCE.NSE", d, OPEN + i * 30, "1240.00", 0))
        .collect();
    let source = archive(vec![DayZip::of(
        Tree::Stocks,
        d,
        &[("RELIANCE.NSE.csv".to_owned(), csv(&text))],
    )]);
    let store = root.join("S");
    let got = import(
        &source,
        ImportKind::Stocks,
        (d, d),
        &store,
        &only("RELIANCE"),
    );
    assert_eq!(got.seconds_committed, 0, "{got:?}");
    assert!(stored(&store, "CASH", "RELIANCE", (2024, 6)).is_empty());
    std::fs::remove_dir_all(root).unwrap();
}

// ===========================================================================
// A chained rename on the days either side of it
// ===========================================================================

/// `LTI -> LTIM` (2022-12-05) then `LTIM -> LTM` (2026-02-27), through the
/// whole import under the STORED scope (the swept scope files no old stem):
/// a stem is filed under the newest symbol on a day the exchange used it,
/// and refused by name on a day it did not: `LTI` on the day of its own
/// change, `LTIM` the trading day before its birth and on the day of its own
/// change. `gdfl_cm_admission_tests.rs` asks the scope's `filing` the same
/// question away from the edges; this asks the run, on the edges.
#[test]
fn a_chained_rename_files_each_stem_under_ltm_only_on_its_own_days() {
    let root = scratch("ChainedRename");
    let scope = std::sync::Arc::new(pull::gdfl_cm::CmScope::stored(["LTI"]).unwrap());
    let cases = [
        (day(2022, 12, 2), "LTI", true),
        (day(2022, 12, 5), "LTI", false),
        (day(2022, 12, 2), "LTIM", false),
        (day(2022, 12, 5), "LTIM", true),
        (day(2026, 2, 26), "LTIM", true),
        (day(2026, 2, 27), "LTIM", false),
        (day(2026, 2, 27), "LTM", true),
    ];
    for (n, &(d, stem, files)) in cases.iter().enumerate() {
        let store = root.join(format!("S{n}"));
        let name = format!("{stem}.NSE.csv");
        let rows: Vec<String> = (0..50)
            .map(|i| row(&format!("{stem}.NSE"), d, OPEN + i * 60, "5000.00", 2))
            .collect();
        let source = archive(vec![DayZip::of(Tree::Stocks, d, &[(name, csv(&rows))])])
            .with_scope(std::sync::Arc::clone(&scope));
        let got = import(&source, ImportKind::Stocks, (d, d), &store, &[]);
        let held = stored(&store, "CASH", "LTM", (d.year(), d.month())).len();
        let old: usize = ["LTI", "LTIM"]
            .iter()
            .map(|s| stored(&store, "CASH", s, (d.year(), d.month())).len())
            .sum();
        assert_eq!(old, 0, "{d} {stem}: nothing is filed under an old key");
        if files {
            assert_eq!(held, 50, "{d} {stem}: filed under LTM: {:?}", got.failures);
        } else {
            assert_eq!(
                held, 0,
                "{d} {stem}: a stem the exchange did not use that day"
            );
            assert!(
                got.failures.iter().any(|f| f.instrument.contains(stem)),
                "{d} {stem}: refused by name: {:?}",
                got.failures
            );
        }
    }
    std::fs::remove_dir_all(root).unwrap();
}

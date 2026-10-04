//! ATTACK, INGEST: the Zerodha (Kite) candle path, end to end, under hostile
//! permutations — decoded by `pull::http::decode_body` with the shipped Zerodha
//! descriptor, filed by `pull::ingest::from_window`, and checked against the
//! bytes on disk and the census.
//!
//! Every property test here draws from a fixed-seed splitmix64, so a rerun is
//! byte-identical (`CLAUDE.md` §3 rule 5).

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::too_many_lines,
    clippy::float_arithmetic
)]

use std::fs;
use std::path::{Path, PathBuf};

use brutex_core::vendor::Vendor;
use pull::fetch::{BarRequest, RawRow, RawWindow};
use pull::ingest::{Ingested, Plan};
use pull::session::{Day, DropReason, Window};
use pull::vendor::{Granularity, Listing, PriceScale, TimestampEncoding};
use store::path::Timeframe;

// ---------------------------------------------------------------------------
// fixtures
// ---------------------------------------------------------------------------

/// 2025-07-01 09:15:00 IST as a UTC epoch second. A Tuesday, a full session.
const OPEN_UTC: i64 = 1_751_341_500;
/// The index session before 2026-08-03: 09:15 to 15:30, 375 minutes.
const SESSION_MINUTES: i64 = 375;

struct Scratch(PathBuf);
impl Scratch {
    fn new(tag: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("brutex-attack-ingest-{tag}-{}", std::process::id()));
        let _best_effort = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("a scratch root");
        Self(dir)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _best_effort = fs::remove_dir_all(&self.0);
    }
}

/// splitmix64, fixed seed: the only randomness in this file.
struct Mix(u64);
impl Mix {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

fn zerodha() -> pull::vendor::HttpSpec {
    let pull::vendor::Transport::Http(spec) = pull::vendor::Feed::Zerodha.descriptor().transport
    else {
        panic!("Zerodha is an HTTP feed");
    };
    spec
}

fn request(from: Day, to: Day, granularity: Granularity) -> BarRequest {
    BarRequest {
        instrument_id: String::new(),
        listing: Listing::Index,
        window: Window::new(from, to).expect("a legal window"),
        granularity,
    }
}

fn day(y: u16, m: u8, d: u8) -> Day {
    Day::new(y, m, d).expect("a real day")
}

fn july_first(granularity: Granularity) -> BarRequest {
    request(day(2025, 7, 1), day(2025, 7, 1), granularity)
}

/// The plan the Zerodha descriptor implies: zone-carrying text stamps, rupees.
fn plan(request: &BarRequest) -> Plan<'_> {
    Plan {
        calendar: pull::calendar::Runtime::default(),
        cash_schedule: None,
        columns: pull::csv::Columns::TrueDataIndex,
        request,
        encoding: TimestampEncoding::IsoDateTimeOffset,
        scale: PriceScale::Rupees,
        vendor: Vendor::Zerodha,
        exchange: "NSE",
        segment: "INDEX",
        contract: None,
    }
}

/// `2025-07-01T09:15:00+0530`-shaped text for a UTC epoch second, in IST.
fn ist_text(utc: i64, zone: &str) -> String {
    let local = utc + 19_800;
    let days = local.div_euclid(86_400);
    let secs = local.rem_euclid(86_400);
    let d = Day::from_days(u32::try_from(days).unwrap()).unwrap();
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}{zone}",
        d.year(),
        d.month(),
        d.day(),
        secs / 3_600,
        (secs / 60) % 60,
        secs % 60
    )
}

/// One candle as Kite spells it: stamp text and five already-rendered cells.
fn candle(stamp: &str, open: &str, high: &str, low: &str, close: &str, volume: &str) -> String {
    format!("[\"{stamp}\",{open},{high},{low},{close},{volume}]")
}

fn body(candles: &[String]) -> String {
    format!(
        "{{\"status\":\"success\",\"data\":{{\"candles\":[{}]}}}}",
        candles.join(",")
    )
}

/// A whole clean session of Kite minute candles, deterministic prices.
fn session_candles(open_utc: i64) -> Vec<String> {
    (0..SESSION_MINUTES)
        .map(|m| {
            let base = 24_000 + (m * 37) % 211;
            candle(
                &ist_text(open_utc + m * 60, "+0530"),
                &format!("{base}.05"),
                &format!("{}.75", base + 4),
                &format!("{}.10", base - 3),
                &format!("{}.40", base + 1),
                "0",
            )
        })
        .collect()
}

fn decode(text: &str) -> Result<RawWindow, pull::fetch::FetchError> {
    pull::http::decode_body(text, &zerodha(), Listing::Index)
}

fn month_file(root: &Path, tf: Timeframe, ym: &str) -> PathBuf {
    root.join("bars")
        .join("zerodha")
        .join("NSE")
        .join("INDEX")
        .join("NIFTY")
        .join(tf.as_str())
        .join(format!("{ym}.bin"))
}

fn census_bytes(root: &Path) -> Vec<u8> {
    fs::read(pull::manifest::manifest_path(root, Vendor::Zerodha)).unwrap_or_default()
}

fn census_has(root: &Path, tf: Timeframe, year: u16, month: u8) -> Option<u64> {
    let bytes = census_bytes(root);
    let census = pull::manifest::Manifest::open_image(Vendor::Zerodha, &bytes).ok()?;
    let key = pull::manifest::EntryKey {
        contract: None,
        exchange: brutex_core::instrument::Exchange::Nse,
        segment: brutex_core::instrument::Segment::Index,
        symbol: brutex_core::symbol::Symbol::new("NIFTY").unwrap(),
        timeframe: tf,
        month: store::path::YearMonth::new(year, month).unwrap(),
    };
    census.entry(&key).map(|entry| entry.rows)
}

/// Every committed bar of one NIFTY month file, or none when it is absent.
fn committed(root: &Path, tf: Timeframe, year: u16, month: u8) -> Vec<store::format::Bar> {
    let path = store::path::StorePath::new(store::path::PathParts {
        vendor: Vendor::Zerodha,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "NIFTY",
        contract: None,
        timeframe: tf,
        month: store::path::YearMonth::new(year, month).unwrap(),
        file: store::path::FileKind::Bars,
    })
    .unwrap();
    let Ok(file) = store::file::BarFile::open_existing(
        root,
        path,
        brutex_core::universe::fnv1a("NIFTY") as u32,
    ) else {
        return Vec::new();
    };
    (0..file.header().n_valid)
        .map(|i| file.read_record(i).unwrap())
        .collect()
}

fn land(raw: &RawWindow, root: &Path, req: &BarRequest) -> Ingested {
    pull::ingest::from_window(raw, "NIFTY", "attack", root, plan(req))
}

/// Every file under `root`, path and bytes, sorted: the whole store image.
fn image(root: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    fn walk(dir: &Path, out: &mut Vec<(PathBuf, Vec<u8>)>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().and_then(|e| e.to_str()) != Some("lock") {
                out.push((path.clone(), fs::read(&path).unwrap()));
            }
        }
    }
    let mut out = Vec::new();
    walk(root, &mut out);
    out.sort();
    out.into_iter()
        .map(|(p, b)| (p.strip_prefix(root).unwrap().to_path_buf(), b))
        .collect()
}

// ---------------------------------------------------------------------------
// timestamps
// ---------------------------------------------------------------------------

/// `+0530`, `+05:30` and `Z` naming the same instant decode to the same epoch
/// second; a stamp with no zone, a malformed zone, or a zone past 14 h is
/// refused rather than read as IST.
#[test]
fn every_zone_spelling_of_one_instant_decodes_to_one_epoch_and_no_zone_is_refused() {
    let at = OPEN_UTC + 17 * 60;
    let utc = {
        let local = at;
        let days = local.div_euclid(86_400);
        let secs = local.rem_euclid(86_400);
        let d = Day::from_days(u32::try_from(days).unwrap()).unwrap();
        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
            d.year(),
            d.month(),
            d.day(),
            secs / 3_600,
            (secs / 60) % 60,
            secs % 60
        )
    };
    let spellings = [
        ist_text(at, "+0530"),
        ist_text(at, "+05:30"),
        utc,
        // the same instant written in a different zone entirely
        ist_text(at - 19_800 + 3_600, "+0100"),
    ];
    let mut seen = Vec::new();
    for stamp in &spellings {
        let raw = decode(&body(&[candle(stamp, "1", "1", "1", "1", "0")]))
            .unwrap_or_else(|e| panic!("{stamp}: {e}"));
        seen.push(raw.rows[0].timestamp);
    }
    // The +0100 spelling was built from `at - 19_800 - 3_600` read as IST,
    // i.e. local 1 h ahead of UTC, so it is the same instant.
    assert!(
        seen.iter().all(|&t| t == at),
        "{spellings:?} decoded to {seen:?}, want {at}"
    );

    let bad = [
        ist_text(at, ""),
        ist_text(at, "+530"),
        ist_text(at, "+05:3"),
        ist_text(at, "+1500"),
        ist_text(at, "+0560"),
        ist_text(at, "z"),
        ist_text(at, " +0530"),
        ist_text(at, "+0530 "),
        ist_text(at, "+05\u{2236}30"),
        ist_text(at, "\u{FF0B}0530"),
        "2025-07-01 09:15:00".to_owned(),
        "2025-07-01T9:15:00+0530".to_owned(),
        "2025-13-01T09:15:00+0530".to_owned(),
        "2025-02-29T09:15:00+0530".to_owned(),
        "2025-07-01T24:00:00+0530".to_owned(),
        "2025-07-01T09:60:00+0530".to_owned(),
        "2025-07-01T09:15:60+0530".to_owned(),
        "+025-07-01T09:15:00+0530".to_owned(),
        String::new(),
    ];
    for stamp in &bad {
        let got = decode(&body(&[candle(stamp, "1", "1", "1", "1", "0")]));
        assert!(got.is_err(), "{stamp:?} decoded to {got:?}");
    }
    // A leap day that exists is read.
    let leap = decode(&body(&[candle(
        "2024-02-29T09:15:00+0530",
        "1",
        "1",
        "1",
        "1",
        "0",
    )]))
    .expect("2024-02-29 is a day");
    assert_eq!(leap.rows[0].timestamp, 1_709_178_300);
}

/// A minute candle whose seconds are not zero is off the broker grid: the
/// whole window is refused by name, nothing reaches the disk, and the tally
/// says so. Exhaustive over every second 1..=59 at three minutes of the day.
#[test]
fn a_minute_candle_off_the_whole_minute_refuses_the_window_and_writes_nothing() {
    let req = july_first(Granularity::Minute1);
    let mut tried = 0;
    for minute in [0_i64, 187, 374] {
        for second in 1..60_i64 {
            let scratch = Scratch::new("offgrid");
            let mut candles = session_candles(OPEN_UTC);
            candles[minute as usize] = candle(
                &ist_text(OPEN_UTC + minute * 60 + second, "+0530"),
                "1",
                "1",
                "1",
                "1",
                "0",
            );
            let raw = decode(&body(&candles)).unwrap();
            let done = land(&raw, &scratch.0, &req);
            assert_eq!(done.bars_committed, 0);
            assert!(!done.balances(), "a refused window must not balance");
            assert!(
                done.failures[0].why.contains("off-grid"),
                "{:?}",
                done.failures
            );
            assert!(image(&scratch.0).is_empty() || census_bytes(&scratch.0).is_empty());
            assert!(!month_file(&scratch.0, Timeframe::MINUTE_1, "2025-07").exists());
            tried += 1;
        }
    }
    assert_eq!(tried, 177);
}

/// 09:14 is before the open, 15:29 is the last minute, 15:30 is AT the close
/// and is dropped. Each drop is counted under its own reason and the books
/// balance.
#[test]
fn the_session_edges_are_kept_or_dropped_by_name_and_the_tally_balances() {
    let scratch = Scratch::new("edges");
    let req = july_first(Granularity::Minute1);
    let mut candles = session_candles(OPEN_UTC);
    // 09:14 and 15:30, and 15:45 and 00:00 the same day, and 08:59:00
    let extra = [-60_i64, SESSION_MINUTES * 60, SESSION_MINUTES * 60 + 900];
    for e in extra {
        candles.push(candle(
            &ist_text(OPEN_UTC + e, "+0530"),
            "1",
            "1",
            "1",
            "1",
            "0",
        ));
    }
    // vendor order is chronological; the 09:14 goes first
    let before = candles.remove(SESSION_MINUTES as usize);
    candles.insert(0, before);
    let raw = decode(&body(&candles)).unwrap();
    let done = land(&raw, &scratch.0, &req);
    assert!(done.failures.is_empty(), "{:?}", done.failures);
    assert_eq!(done.rows_read, 378);
    assert_eq!(done.bars_stored, 375);
    assert_eq!(done.bars_committed, 375);
    assert_eq!(done.census.of(DropReason::BeforeSessionOpen), 1);
    assert_eq!(done.census.of(DropReason::AtOrAfterSessionClose), 2);
    assert!(done.balances());
    assert_eq!(
        census_has(&scratch.0, Timeframe::MINUTE_1, 2025, 7),
        Some(375)
    );
}

/// Candles from the day after the window (the inclusive `to` a broker may
/// answer past) and from the day before are dropped as window drops.
#[test]
fn candles_outside_the_requested_days_are_window_drops_not_bars() {
    let scratch = Scratch::new("window");
    let req = july_first(Granularity::Minute1);
    let mut candles = vec![candle(
        &ist_text(OPEN_UTC - 86_400, "+0530"),
        "1",
        "1",
        "1",
        "1",
        "0",
    )];
    candles.extend(session_candles(OPEN_UTC));
    candles.push(candle(
        &ist_text(OPEN_UTC + 86_400, "+0530"),
        "1",
        "1",
        "1",
        "1",
        "0",
    ));
    let raw = decode(&body(&candles)).unwrap();
    let done = land(&raw, &scratch.0, &req);
    assert!(done.failures.is_empty(), "{:?}", done.failures);
    assert_eq!(done.census.of(DropReason::BeforeWindow), 1);
    assert_eq!(done.census.of(DropReason::AfterWindow), 1);
    assert_eq!(done.bars_committed, 375);
    assert!(done.balances());
}

/// Out-of-order minutes: refused, named, and the disk is untouched. Swaps a
/// random adjacent pair 200 times over a fixed seed.
#[test]
fn out_of_order_minutes_are_refused_and_write_nothing() {
    let req = july_first(Granularity::Minute1);
    let mut mix = Mix(0x1A2B_3C4D);
    for _ in 0..200 {
        let scratch = Scratch::new("order");
        let mut candles = session_candles(OPEN_UTC);
        let at = mix.below(SESSION_MINUTES as u64 - 1) as usize;
        candles.swap(at, at + 1);
        let raw = decode(&body(&candles)).unwrap();
        let done = land(&raw, &scratch.0, &req);
        assert_eq!(done.bars_committed, 0);
        assert!(!done.failures.is_empty());
        assert!(!done.balances());
        assert!(!month_file(&scratch.0, Timeframe::MINUTE_1, "2025-07").exists());
        assert_eq!(census_has(&scratch.0, Timeframe::MINUTE_1, 2025, 7), None);
    }
}

/// Exact duplicates count once (volume not doubled); a conflicting duplicate
/// refuses the window.
#[test]
fn duplicate_minutes_fold_once_and_a_conflicting_twin_refuses() {
    let req = july_first(Granularity::Minute1);
    let clean = Scratch::new("dup-clean");
    let twin = Scratch::new("dup-twin");
    let candles = session_candles(OPEN_UTC);
    let reference = land(&decode(&body(&candles)).unwrap(), &clean.0, &req);
    assert!(reference.balances());

    let mut doubled = Vec::new();
    for c in &candles {
        doubled.push(c.clone());
        doubled.push(c.clone());
    }
    let done = land(&decode(&body(&doubled)).unwrap(), &twin.0, &req);
    assert!(done.failures.is_empty(), "{:?}", done.failures);
    assert_eq!(done.rows_read, 750);
    assert_eq!(done.rows_folded, 375);
    assert!(done.balances());
    assert_eq!(image(&clean.0), image(&twin.0), "duplicates change no byte");

    let conflict = Scratch::new("dup-conflict");
    let mut bad = candles.clone();
    bad.insert(
        10,
        candle(
            &ist_text(OPEN_UTC + 9 * 60, "+0530"),
            "1",
            "1",
            "1",
            "1",
            "0",
        ),
    );
    let done = land(&decode(&body(&bad)).unwrap(), &conflict.0, &req);
    assert!(!done.balances());
    assert!(done.failures[0].why.contains("conflicting"));
    assert!(
        image(&conflict.0)
            .iter()
            .all(|(p, _)| !p.ends_with("2025-07.bin"))
    );
}

// ---------------------------------------------------------------------------
// prices
// ---------------------------------------------------------------------------

/// Half-up to paisa exactly once, on the vendor's own text: exhaustive over
/// every third-and-fourth decimal on a few rupee values, plus the hostile
/// spellings.
#[test]
fn prices_snap_half_up_once_and_hostile_numbers_are_refused() {
    let stamp = ist_text(OPEN_UTC, "+0530");
    let one = |p: &str| decode(&body(&[candle(&stamp, p, p, p, p, "0")]));
    let mut tried = 0;
    for rupees in [0_i64, 1, 99, 24_653, 999_999] {
        for paisa in 0..100_i64 {
            for tail in 0..100_i64 {
                let text = format!("{rupees}.{paisa:02}{tail:02}");
                // the exact rational in units of 1/10_000 rupee
                let exact = rupees * 10_000 + paisa * 100 + tail;
                let want = (exact + 50) / 100; // half-up, in paisa
                let got = one(&text);
                if want == 0 && exact != 0 {
                    assert!(got.is_err(), "{text} snaps to zero and must refuse");
                } else {
                    let row = got.unwrap_or_else(|e| panic!("{text}: {e}")).rows[0];
                    assert_eq!(row.close, want, "{text}");
                    assert_eq!((row.open, row.high, row.low), (want, want, want));
                }
                tried += 1;
            }
        }
    }
    assert_eq!(tried, 50_000);
    // 0.005 rupees is half a paisa and rounds UP to one.
    assert_eq!(one("0.005").unwrap().rows[0].close, 1);
    assert_eq!(one("0.0049").map(|r| r.rows[0].close).ok(), None);
    // exponent spellings are the same numbers
    assert_eq!(one("2.465e4").unwrap().rows[0].close, 2_465_000);
    assert_eq!(one("24650E-3").unwrap().rows[0].close, 2_465);
    // hostile: too large, not numbers, negative, non-ASCII
    let huge = format!("1{}", "0".repeat(308));
    for p in [
        "1e308",
        "1E400",
        huge.as_str(),
        "92233720368547758.08",
        "-1",
        "-0.01",
        "\"NaN\"",
        "\"Infinity\"",
        "\"24653.05\"",
        "true",
        "[]",
        "{}",
    ] {
        assert!(one(p).is_err(), "{p} decoded to {:?}", one(p));
    }
    // A null price is the documented untraded-interval skip: no row, never a
    // zero price.
    assert!(one("null").unwrap().rows.is_empty());
    for p in [
        "NaN",
        "Infinity",
        "-Infinity",
        "0x10",
        "1_000",
        "+1",
        ".5",
        "1.",
    ] {
        assert!(one(p).is_err(), "bare {p} is not JSON and must refuse");
    }
}

/// Bars whose OHLC cannot have happened never reach the disk.
#[test]
fn impossible_ohlc_never_reaches_the_disk() {
    let req = july_first(Granularity::Minute1);
    let cases = [
        ("10", "9", "11", "10"),  // high < low
        ("12", "11", "9", "10"),  // open above high
        ("8", "11", "9", "10"),   // open below low
        ("10", "11", "9", "12"),  // close above high
        ("10", "11", "9", "8"),   // close below low
        ("10", "10", "10", "-1"), // negative close
    ];
    for (o, h, l, c) in cases {
        let scratch = Scratch::new("ohlc");
        let mut candles = session_candles(OPEN_UTC);
        candles[100] = candle(&ist_text(OPEN_UTC + 6_000, "+0530"), o, h, l, c, "0");
        let done = match decode(&body(&candles)) {
            Ok(raw) => land(&raw, &scratch.0, &req),
            Err(_) => continue,
        };
        // Whatever the decoder did, the file on disk holds no impossible bar:
        // either the window refused or the minute is missing and NAMED.
        assert!(!done.balances() || done.bars_committed == 374, "{done:?}");
        assert!(!done.failures.is_empty(), "a hole in the session is named");
        if let Ok(file) = store::file::BarFile::open_existing(
            &scratch.0,
            store::path::StorePath::new(store::path::PathParts {
                vendor: Vendor::Zerodha,
                exchange: "NSE",
                segment: "INDEX",
                symbol: "NIFTY",
                contract: None,
                timeframe: Timeframe::MINUTE_1,
                month: store::path::YearMonth::new(2025, 7).unwrap(),
                file: store::path::FileKind::Bars,
            })
            .unwrap(),
            brutex_core::universe::fnv1a("NIFTY") as u32,
        ) {
            for i in 0..file.header().n_valid {
                let bar = file.read_record(i).unwrap();
                assert!(bar.ohlc_is_sane(), "{bar:?}");
            }
        }
    }
}

/// Volume edges: zero is zero; `i64::MAX` is held; past `i64` is refused; a
/// negative is not stored.
#[test]
fn volume_edges_are_held_exactly_or_refused() {
    let stamp = ist_text(OPEN_UTC, "+0530");
    let one = |v: &str| {
        pull::http::decode_body(
            &body(&[candle(&stamp, "1", "1", "1", "1", v)]),
            &zerodha(),
            Listing::Equity,
        )
    };
    assert_eq!(one("0").unwrap().rows[0].volume, 0);
    assert_eq!(one("9223372036854775807").unwrap().rows[0].volume, i64::MAX);
    for v in [
        "9223372036854775808",
        "18446744073709551615",
        "18446744073709551616",
        "1e30",
        "1.5",
        "\"5\"",
    ] {
        assert!(one(v).is_err(), "{v} decoded to {:?}", one(v));
    }
    // a negative volume never decodes into a row
    let got = one("-1");
    assert!(got.as_ref().map_or(true, |w| w.rows.is_empty()), "{got:?}");
}

/// Malformed bodies refuse; none panics. Truncation at every byte of a real
/// two-candle body, plus the hostile shapes.
#[test]
fn every_truncation_and_hostile_body_refuses_without_panicking() {
    let full = body(&session_candles(OPEN_UTC)[..2]);
    let mut refused = 0;
    for cut in 0..full.len() {
        let Some(part) = full.get(..cut) else {
            continue;
        };
        assert!(decode(part).is_err(), "prefix {cut} decoded: {part}");
        refused += 1;
    }
    assert_eq!(refused, full.len());
    let hostile = [
        String::new(),
        " ".to_owned(),
        "{}".to_owned(),
        "[]".to_owned(),
        "null".to_owned(),
        "{\"status\":\"success\"}".to_owned(),
        "{\"status\":\"success\",\"data\":{}}".to_owned(),
        "{\"status\":\"success\",\"data\":{\"candles\":{}}}".to_owned(),
        "{\"status\":\"success\",\"data\":{\"candles\":[[]]}}".to_owned(),
        "{\"status\":\"success\",\"data\":{\"candles\":[[1,2,3,4,5]]}}".to_owned(),
        "{\"status\":\"success\",\"data\":{\"candles\":[[\"x\",1,1,1,1,0,0,0]]}}".to_owned(),
        "{\"status\":\"error\",\"error_type\":\"TokenException\",\"message\":\"x\"}".to_owned(),
        format!("{full}\u{0}"),
        full.replace("candles", "candl\u{e9}s"),
        format!("{}{}", "[".repeat(200_000), "]".repeat(200_000)),
    ];
    for h in &hostile {
        let got = decode(h);
        assert!(
            got.is_err(),
            "{:?} decoded to {got:?}",
            &h[..h.len().min(80)]
        );
    }
    // the empty candle array is a legal, empty answer
    let empty = decode(&body(&[])).expect("an empty answer is an answer");
    assert!(empty.rows.is_empty());
}

// ---------------------------------------------------------------------------
// the ladder: day first, then minute, re-derive the rest
// ---------------------------------------------------------------------------

fn day_candles(days: &[Day]) -> Vec<String> {
    days.iter()
        .enumerate()
        .map(|(i, d)| {
            let p = 24_000 + i as i64 * 7;
            candle(
                &format!(
                    "{:04}-{:02}-{:02}T00:00:00+0530",
                    d.year(),
                    d.month(),
                    d.day()
                ),
                &format!("{p}.00"),
                &format!("{}.50", p + 90),
                &format!("{}.25", p - 80),
                &format!("{}.75", p + 10),
                "0",
            )
        })
        .collect()
}

/// The operator's order — the whole range at one day first, then one minute —
/// lands the day file untouched by the minute pass, the minute file and seven
/// derived rungs, and a census row for every one of the nine. Re-running both
/// passes in the same order changes no byte anywhere.
#[test]
fn the_day_pass_then_the_minute_pass_is_nine_files_and_rerunning_changes_no_byte() {
    let scratch = Scratch::new("ladder");
    let trading = [
        day(2025, 7, 1),
        day(2025, 7, 2),
        day(2025, 7, 3),
        day(2025, 7, 4),
    ];
    let day_req = request(trading[0], trading[3], Granularity::Day1);
    let day_raw = decode(&body(&day_candles(&trading))).unwrap();
    let day_done = land(&day_raw, &scratch.0, &day_req);
    assert!(day_done.failures.is_empty(), "{:?}", day_done.failures);
    assert!(day_done.balances());
    assert_eq!(day_done.bars_committed, 4);
    let day_bytes = fs::read(month_file(&scratch.0, Timeframe::DAY_1, "2025-07")).unwrap();

    let min_req = july_first(Granularity::Minute1);
    let min_raw = decode(&body(&session_candles(OPEN_UTC))).unwrap();
    let min_done = land(&min_raw, &scratch.0, &min_req);
    assert!(min_done.failures.is_empty(), "{:?}", min_done.failures);
    assert!(min_done.balances());
    assert_eq!(min_done.derived_files, 7);
    assert_eq!(
        fs::read(month_file(&scratch.0, Timeframe::DAY_1, "2025-07")).unwrap(),
        day_bytes,
        "the minute pass never rewrites the vendor's day bar"
    );
    let mut rungs = 0;
    for &tf in Timeframe::KNOWN {
        if tf.secs() < 60 {
            continue;
        }
        assert!(
            census_has(&scratch.0, tf, 2025, 7).is_some(),
            "{} has no census row",
            tf.as_str()
        );
        rungs += 1;
    }
    assert_eq!(rungs, 9, "1day + 1min + seven derived");

    let before = image(&scratch.0);
    let again_day = land(&day_raw, &scratch.0, &day_req);
    let again_min = land(&min_raw, &scratch.0, &min_req);
    assert_eq!(again_day.bars_committed + again_min.bars_committed, 0);
    assert!(again_day.balances() && again_min.balances());
    assert_eq!(image(&scratch.0), before, "a re-run changes no byte");
}

/// The minute pass first and the day pass second land the same nine files,
/// byte for byte, as the operator's order. Order of passes is not identity.
#[test]
fn the_two_passes_commute() {
    let a = Scratch::new("commute-a");
    let b = Scratch::new("commute-b");
    let trading = [day(2025, 7, 1)];
    let day_req = request(trading[0], trading[0], Granularity::Day1);
    let day_raw = decode(&body(&day_candles(&trading))).unwrap();
    let min_req = july_first(Granularity::Minute1);
    let min_raw = decode(&body(&session_candles(OPEN_UTC))).unwrap();
    assert!(land(&day_raw, &a.0, &day_req).balances());
    assert!(land(&min_raw, &a.0, &min_req).balances());
    assert!(land(&min_raw, &b.0, &min_req).balances());
    assert!(land(&day_raw, &b.0, &day_req).balances());
    let strip = |img: Vec<(PathBuf, Vec<u8>)>| {
        img.into_iter()
            .filter(|(p, _)| !p.to_string_lossy().contains("manifest"))
            .collect::<Vec<_>>()
    };
    assert_eq!(strip(image(&a.0)), strip(image(&b.0)));
}

// ---------------------------------------------------------------------------
// a window across a month boundary
// ---------------------------------------------------------------------------

/// A day pass spanning June into July lands one file per month and one census
/// row per file.
#[test]
fn a_day_window_across_a_month_boundary_lands_one_file_and_one_row_per_month() {
    let scratch = Scratch::new("month-span");
    let trading = [day(2025, 6, 27), day(2025, 6, 30), day(2025, 7, 1)];
    let req = request(trading[0], trading[2], Granularity::Day1);
    let done = land(
        &decode(&body(&day_candles(&trading))).unwrap(),
        &scratch.0,
        &req,
    );
    assert!(done.failures.is_empty(), "{:?}", done.failures);
    assert!(done.balances());
    assert_eq!(census_has(&scratch.0, Timeframe::DAY_1, 2025, 6), Some(2));
    assert_eq!(census_has(&scratch.0, Timeframe::DAY_1, 2025, 7), Some(1));
}

/// THE FINDING. A batch spanning two months whose SECOND month the store
/// refuses: the first month's file was already written (and its derived rungs
/// with it), so the census must count it. Before the fix, `one` returned the
/// second month's error through `?` and the first month's census rows were
/// dropped on the floor — bars on disk the counter denies, which is the
/// outcome the module header calls worse than refusing outright.
#[test]
fn a_month_the_store_refuses_does_not_uncount_the_month_already_written() {
    let scratch = Scratch::new("month-refused");
    let trading = [day(2025, 6, 30), day(2025, 7, 1)];
    let req = request(trading[0], trading[1], Granularity::Day1);
    let mut candles = day_candles(&trading);
    // July's bar is impossible: high below low.
    candles[1] = candle("2025-07-01T00:00:00+0530", "10", "9", "11", "10", "0");
    let raw = RawWindow {
        skipped: pull::fetch::DecodeSkips::default(),
        rows: {
            let mut rows = decode(&body(&candles[..1])).unwrap().rows;
            rows.push(RawRow {
                timestamp: rows[0].timestamp + 86_400,
                open: 1_000,
                high: 900,
                low: 1_100,
                close: 1_000,
                volume: 0,
                open_interest: None,
            });
            rows
        },
    };
    let done = land(&raw, &scratch.0, &req);
    assert!(
        !done.balances(),
        "July was refused, so the books cannot balance"
    );
    assert!(
        done.failures.iter().any(|f| f.why.contains("impossible")),
        "{:?}",
        done.failures
    );
    assert!(
        month_file(&scratch.0, Timeframe::DAY_1, "2025-06").exists(),
        "June was written before July refused"
    );
    assert_eq!(
        census_has(&scratch.0, Timeframe::DAY_1, 2025, 6),
        Some(1),
        "June's bars are on disk, so the census must count them"
    );
    assert_eq!(
        committed(&scratch.0, Timeframe::DAY_1, 2025, 7).len(),
        0,
        "July's impossible bar never landed"
    );
    assert_eq!(census_has(&scratch.0, Timeframe::DAY_1, 2025, 7), None);
    assert_eq!(
        done.bars_committed, 1,
        "one bar was written, and it is said"
    );
}

// ---------------------------------------------------------------------------
// weekend and holiday
// ---------------------------------------------------------------------------

/// A minute candle on a Saturday the calendar says is closed is never stored
/// as an ordinary session bar without a named reason.
#[test]
fn a_weekend_minute_candle_is_named_not_silently_stored() {
    let scratch = Scratch::new("weekend");
    let saturday = day(2025, 7, 5);
    let req = request(saturday, saturday, Granularity::Minute1);
    let sat_open = OPEN_UTC + 4 * 86_400;
    let raw = decode(&body(&[candle(
        &ist_text(sat_open, "+0530"),
        "1",
        "1",
        "1",
        "1",
        "0",
    )]))
    .unwrap();
    let done = land(&raw, &scratch.0, &req);
    let stored = done.bars_committed;
    let named = !done.failures.is_empty() || done.census.total() > 0;
    assert!(
        stored == 0 || named,
        "a Saturday candle was stored with no named reason: {done:?}"
    );
}

// ---------------------------------------------------------------------------
// the tally
// ---------------------------------------------------------------------------

/// `absorb` sums every counter, and a sum of balanced runs balances.
#[test]
fn absorbing_balanced_runs_balances_and_loses_no_counter() {
    let req = july_first(Granularity::Minute1);
    let a = Scratch::new("absorb-a");
    let mut total = Ingested::default();
    assert!(total.balances(), "the empty receipt balances");
    let raw = decode(&body(&session_candles(OPEN_UTC))).unwrap();
    let first = land(&raw, &a.0, &req);
    let second = land(&raw, &a.0, &req);
    total.absorb(first.clone());
    total.absorb(second.clone());
    assert!(total.balances());
    assert_eq!(total.members, first.members + second.members);
    assert_eq!(total.rows_read, first.rows_read + second.rows_read);
    assert_eq!(total.bars_stored, first.bars_stored + second.bars_stored);
    assert_eq!(total.bars_committed, first.bars_committed);
    assert_eq!(
        total.derived_files,
        first.derived_files + second.derived_files
    );
    assert_eq!(total.counted, first.counted + second.counted);
}

/// Property: for a random hostile permutation of one session (drops, dups,
/// off-session extras, window extras), the receipt either balances exactly —
/// rows read = stored + folded + dropped-by-reason — or names a failure.
/// 400 cases, fixed seed.
#[test]
fn every_receipt_balances_or_names_a_failure() {
    let req = july_first(Granularity::Minute1);
    let mut mix = Mix(0xC0FF_EE00);
    let mut balanced = 0;
    let mut named = 0;
    for case in 0..400 {
        let scratch = Scratch::new(&format!("prop-{case}"));
        let mut candles = session_candles(OPEN_UTC);
        let extra_before = mix.below(3) as i64;
        let extra_after = mix.below(3) as i64;
        let mut pre = Vec::new();
        for k in 0..extra_before {
            pre.push(candle(
                &ist_text(OPEN_UTC - 60 * (extra_before - k), "+0530"),
                "1",
                "1",
                "1",
                "1",
                "0",
            ));
        }
        for k in 0..extra_after {
            candles.push(candle(
                &ist_text(OPEN_UTC + (SESSION_MINUTES + k) * 60, "+0530"),
                "1",
                "1",
                "1",
                "1",
                "0",
            ));
        }
        pre.extend(candles);
        let mut candles = pre;
        if mix.below(2) == 0 {
            let at = mix.below(candles.len() as u64) as usize;
            let c = candles[at].clone();
            candles.insert(at, c);
        }
        if mix.below(4) == 0 {
            let at = mix.below(candles.len() as u64) as usize;
            candles.remove(at);
        }
        let raw = decode(&body(&candles)).unwrap();
        let done = land(&raw, &scratch.0, &req);
        let sum = done.bars_stored + done.rows_folded + done.census.total() as usize;
        if done.balances() {
            assert_eq!(done.rows_read, sum);
            assert_eq!(done.rows_read, candles.len(), "every offered row is read");
            balanced += 1;
        } else {
            assert!(!done.failures.is_empty());
            for f in &done.failures {
                assert!(!f.why.trim().is_empty(), "an empty refusal names nothing");
            }
            named += 1;
        }
    }
    assert_eq!(balanced + named, 400);
    assert!(balanced > 0 && named > 0, "{balanced} / {named}");
}

// ---------------------------------------------------------------------------
// per-row cost
// ---------------------------------------------------------------------------

/// Per-row ingest cost at 10^3..10^6 rows. Reported, with a loose flatness
/// bound: the per-row MEDIAN at the largest size is within 8× of the smallest
/// (the store write and census install are per call, so small calls carry
/// relatively more fixed cost; a scan per row would be 1000×).
#[test]
fn per_row_ingest_cost_is_flat_from_a_thousand_to_a_million() {
    // One minute bar per minute, continuous, across as many sessions as needed.
    let mut medians = Vec::new();
    for &n in &[1_000_usize, 10_000, 100_000, 1_000_000] {
        let sessions = n.div_ceil(SESSION_MINUTES as usize);
        let first = day(2015, 1, 1);
        let last = Day::from_days(first.days_from_epoch() + sessions as u32 * 2).unwrap();
        let req = request(first, last, Granularity::Minute1);
        let mut rows = Vec::with_capacity(n);
        let mut d = first;
        while rows.len() < n {
            let open = i64::from(d.days_from_epoch()) * 86_400 + 9 * 3_600 + 15 * 60 - 19_800;
            for m in 0..SESSION_MINUTES {
                if rows.len() == n {
                    break;
                }
                let p = 2_400_000 + (m * 37) % 211;
                rows.push(RawRow {
                    timestamp: open + m * 60,
                    open: p,
                    high: p + 10,
                    low: p - 10,
                    close: p,
                    volume: 0,
                    open_interest: None,
                });
            }
            d = d.succ().unwrap();
        }
        let mut samples = Vec::new();
        let reps = match n {
            1_000 => 21,
            10_000 => 11,
            100_000 => 5,
            _ => 3,
        };
        for rep in 0..reps {
            let scratch = Scratch::new(&format!("cost-{n}-{rep}"));
            let raw = RawWindow {
                rows: rows.clone(),
                skipped: pull::fetch::DecodeSkips::default(),
            };
            let mut p = plan(&req);
            p.encoding = TimestampEncoding::EpochSecondsUtc;
            p.scale = PriceScale::Paisa;
            let started = std::time::Instant::now();
            let done = pull::ingest::from_window(&raw, "NIFTY", "attack", &scratch.0, p);
            let took = started.elapsed();
            assert_eq!(done.rows_read, n);
            samples.push(took.as_nanos() as f64 / n as f64);
        }
        samples.sort_by(f64::total_cmp);
        let p50 = samples[samples.len() / 2];
        let p99 = samples[(samples.len() * 99 / 100).min(samples.len() - 1)];
        let max = samples[samples.len() - 1];
        println!("INGEST_COST n={n} reps={reps} per-row ns p50={p50:.1} p99={p99:.1} max={max:.1}");
        medians.push(p50);
    }
    let small = medians[0];
    let big = medians[medians.len() - 1];
    assert!(
        big <= small * 8.0,
        "per-row cost grew {small:.1} -> {big:.1} ns"
    );
}

// ---------------------------------------------------------------------------
// the decoded-bar door
// ---------------------------------------------------------------------------

fn flat(ts_micros: i64) -> store::format::Bar {
    store::format::Bar {
        ts_micros,
        open: 100,
        high: 100,
        low: 100,
        close: 100,
        volume: 1,
        open_interest: store::format::OI_NULL,
    }
}

/// THE FINDING. `from_rows` used to count a bar whose stamp the calendar
/// cannot read as `BeforeWindow` — a named reason that is false: the bar is
/// not before the window, its instant is not one this build can place at all.
/// `fetch::land` refuses that case as the vendor or the decoder being wrong;
/// this door now does the same, by name, and stores nothing.
#[test]
fn an_unreadable_stamp_on_the_decoded_door_is_refused_not_called_before_window() {
    let req = july_first(Granularity::Minute1);
    for ts in [i64::MAX, i64::MIN, i64::MAX - 1, i64::MIN + 1] {
        let scratch = Scratch::new("unreadable");
        let bars = [flat(OPEN_UTC * 1_000_000), flat(ts)];
        let done = pull::ingest::from_rows(&bars, &[], "NIFTY", "attack", &scratch.0, plan(&req));
        assert_eq!(
            done.census.of(DropReason::BeforeWindow),
            0,
            "{ts}: an unreadable instant is not 'before the requested window'"
        );
        assert!(!done.balances(), "{ts}: {done:?}");
        assert!(
            done.failures
                .iter()
                .any(|f| f.why.contains(&ts.to_string())),
            "{ts}: the refusal names the stamp: {:?}",
            done.failures
        );
        assert_eq!(done.bars_committed, 0);
        assert!(committed(&scratch.0, Timeframe::MINUTE_1, 2025, 7).is_empty());
    }
}

/// The decoded door at the session edges: 09:14 and 15:30 are named drops,
/// 09:15 and 15:29 are kept, and re-offering is byte-identical.
#[test]
fn the_decoded_door_names_its_session_drops_and_reruns_change_no_byte() {
    let req = july_first(Granularity::Minute1);
    let scratch = Scratch::new("rows-edges");
    let at = |m: i64| flat((OPEN_UTC + m * 60) * 1_000_000);
    let bars = [at(-1), at(0), at(SESSION_MINUTES - 1), at(SESSION_MINUTES)];
    let done = pull::ingest::from_rows(&bars, &[], "NIFTY", "attack", &scratch.0, plan(&req));
    assert!(done.failures.is_empty(), "{:?}", done.failures);
    assert_eq!(done.census.of(DropReason::BeforeSessionOpen), 1);
    assert_eq!(done.census.of(DropReason::AtOrAfterSessionClose), 1);
    assert_eq!(done.bars_committed, 2);
    assert!(done.balances());
    let pending: Vec<_> = done.pending.into_iter().collect();
    assert!(pull::ingest::record_held(&scratch.0, Vendor::Zerodha, &pending).is_none());
    let before = image(&scratch.0);
    let again = pull::ingest::from_rows(&bars, &[], "NIFTY", "attack", &scratch.0, plan(&req));
    assert_eq!(again.bars_committed, 0);
    let pending: Vec<_> = again.pending.into_iter().collect();
    assert!(pull::ingest::record_held(&scratch.0, Vendor::Zerodha, &pending).is_none());
    assert_eq!(image(&scratch.0), before);
}

/// One decoder-skip case: o, h, l, c, v, listing, and the reason that must count it.
type SkipCase = (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    Listing,
    fn(&pull::fetch::DecodeSkips) -> usize,
);

/// THE DECODER'S SKIPS AND THE RECEIPT. A Kite day window of three candles,
/// one of which the decoder drops (impossible OHLC, a negative volume on a
/// traded listing, a null price). The receipt must not claim every offered
/// row is accounted for while one of them is nowhere in it.
#[test]
fn a_candle_the_decoder_skips_is_still_on_the_receipt() {
    let trading = [day(2025, 7, 1), day(2025, 7, 2), day(2025, 7, 3)];
    let req = request(trading[0], trading[2], Granularity::Day1);
    // (o, h, l, c, v, listing, which reason must count it)
    let cases: [SkipCase; 3] = [
        ("10", "9", "11", "10", "0", Listing::Index, |s| {
            s.impossible_ohlc
        }),
        ("null", "1", "1", "1", "0", Listing::Index, |s| s.null_price),
        ("1", "1", "1", "1", "-5", Listing::Equity, |s| {
            s.negative_volume
        }),
    ];
    for (o, h, l, c, v, listing, reason) in cases {
        let scratch = Scratch::new("decoder-skip");
        let mut candles = day_candles(&trading);
        candles[1] = candle("2025-07-02T00:00:00+0530", o, h, l, c, v);
        let raw = pull::http::decode_body(&body(&candles), &zerodha(), listing).unwrap();
        assert_eq!(
            raw.rows.len(),
            2,
            "{o},{h},{l},{c},{v}: the decoder skips it"
        );
        assert_eq!(raw.skipped.total(), 1, "and the window carries the skip");
        assert_eq!(reason(&raw.skipped), 1, "under its own reason");
        let mut req = req.clone();
        req.listing = listing;
        let mut into = plan(&req);
        if listing == Listing::Equity {
            into.segment = "CASH";
        }
        let done = pull::ingest::from_window(&raw, "NIFTY", "attack", &scratch.0, into);
        let offered = candles.len();
        assert_eq!(
            done.rows_read, offered,
            "{o},{h},{l},{c},{v}: the vendor offered {offered} candles: {done:?}"
        );
        assert_eq!(done.decoder_skips.total(), 1);
        assert_eq!(reason(&done.decoder_skips), 1);
        assert_eq!(done.bars_committed, 2);
        assert!(done.balances(), "{done:?}");
        // and a receipt that claims fewer skips than it read cannot balance
        let mut lie = done.clone();
        lie.decoder_skips = pull::fetch::DecodeSkips::default();
        assert!(!lie.balances());
        // absorb keeps the reason
        let mut total = Ingested::default();
        total.absorb(done.clone());
        total.absorb(done);
        assert_eq!(reason(&total.decoder_skips), 2);
    }
}

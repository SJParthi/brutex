//! REPAIR 1 red vectors (state/sniper-req76-20261113/check-build-feat-v1.md, blocking items B1-B3), written against the
//! v1 API BEFORE the fixes, so each one FAILED on the v1 code (logs in state/sniper-req76-20261113/repair-feat-v1/) and
//! passes on the repaired code. Every vector reads the frozen store read-only through the same public API as
//! `feat-run readings`.
//!
//! - B1 (K13 span breach): no higher-timeframe column of any family reads a candle from before the 2025-11-17 rights
//!   reset: on 2025-11-17 every HTF column is NA until that HTF's first post-ex-date candle has ENDED.
//! - B2 (SNIPER s13 / plan 2.1 "the grid is Zerodha's candles"): a Zerodha candle whose interval overlaps a segment of
//!   the dated session table is KEPT even when it starts before the session open or in a DR gap; its k comes from the
//!   session open (the first candle that overlaps the segment is k = 1).
//! - B3 (plan 2.1 HALTS; K11; `pull::calendar::HALTS`): no decision candle whose END lies in a market-wide halt
//!   `[from, resume)` can enter (`decision_in_halt`, entry 0).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic, clippy::many_single_char_names, missing_docs)]

use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;

use common_feat::cal::{ist_day, ist_sod, parse_ymd};
use feat_io::host::{self, Ctx, Stream};
use feat_io::run::{self, Htf};
use feat_io::{defaults, load_ctx, minute_stream};

fn ctx() -> Ctx {
    load_ctx(Path::new(defaults::DATES), Path::new(defaults::BHAV), parse_ymd("2018-09-07").unwrap()).unwrap()
}

/// Keeps the header and the rows of the given dates only (a full 1-minute TSV would be ~450 MB in memory).
struct Keep {
    days: Vec<String>,
    buf: Vec<u8>,
    header: Option<Vec<String>>,
    rows: Vec<Vec<String>>,
}

impl Keep {
    fn new(days: &[&str]) -> Self {
        Self { days: days.iter().map(|d| (*d).to_owned()).collect(), buf: Vec::new(), header: None, rows: Vec::new() }
    }
    fn line(&mut self, l: &str) {
        if self.header.is_none() {
            self.header = Some(l.split('\t').map(str::to_owned).collect());
            return;
        }
        if self.days.iter().any(|d| l.starts_with(d.as_str())) {
            self.rows.push(l.split('\t').map(str::to_owned).collect());
        }
    }
    fn col(&self, name: &str) -> usize {
        self.header.as_ref().unwrap().iter().position(|h| h == name).unwrap_or_else(|| panic!("no column {name}"))
    }
}

impl Write for Keep {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        self.buf.extend_from_slice(b);
        while let Some(p) = self.buf.iter().position(|x| *x == b'\n') {
            let l: Vec<u8> = self.buf.drain(..=p).collect();
            let s = String::from_utf8_lossy(l.get(..l.len() - 1).unwrap_or(&[])).into_owned();
            self.line(&s);
        }
        Ok(b.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn readings(c: &Ctx, tf: &str, htf: &[&str], days: &[&str]) -> Keep {
    let store = Path::new(defaults::STORE);
    let (s, _) = minute_stream(store, "ADANIENT", tf, c);
    let (s1, _) = minute_stream(store, "ADANIENT", "1min", c);
    let lv = host::levels(c, &host::day_facts(&s1, c));
    let mut h: Vec<Htf> = htf.iter().map(|t| Htf::new(t, minute_stream(store, "ADANIENT", t, c).0).unwrap()).collect();
    let mut k = Keep::new(days);
    run::run(c, &s, &lv, &mut h, Some(&mut k), None).unwrap();
    k
}

fn hms(s: &str) -> i64 {
    let v: Vec<i64> = s.split(':').map(|x| x.parse().unwrap()).collect();
    v[0] * 3600 + v[1] * 60 + v[2]
}

fn tf_secs(tf: &str) -> i64 {
    tf.trim_end_matches("min").parse::<i64>().unwrap() * 60
}

// ---------------- B1: no HTF reads across the 2025-11-17 CONT reset (every family) ----------------

/// The END (IST seconds of day) of the first candle of `stream` on `day`.
fn first_end_on(stream: &Stream, day: i32) -> i64 {
    let c = stream.candles.iter().map(|x| x.1).find(|c| ist_day(c.start) == day).expect("a candle on the day");
    ist_sod(c.end)
}

#[test]
fn b1_htf_never_reads_a_candle_from_before_the_rights_reset() {
    let c = ctx();
    let store = Path::new(defaults::STORE);
    let e = parse_ymd("2025-11-17").unwrap();
    // the families and their HTF dials as feat-v1-run-2 ran them (runs2.sh)
    let fams: [(&str, &[&str]); 9] = [
        ("3min", &["15min", "60min"]),
        ("1min", &["5min", "15min", "60min"]),
        ("2min", &["5min", "15min", "60min"]),
        ("4min", &["15min", "60min"]),
        ("5min", &["15min", "60min"]),
        ("10min", &["30min", "60min"]),
        ("15min", &["30min", "60min"]),
        ("30min", &["60min"]),
        ("60min", &["30min"]),
    ];
    let mut breaches = Vec::new();
    let mut checked = 0u64;
    for (tf, htf) in fams {
        let k = readings(&c, tf, htf, &["2025-11-17"]);
        let (dc, sc) = (k.col("date"), k.col("start"));
        for h in htf {
            let first = first_end_on(&minute_stream(store, "ADANIENT", h, &c).0, e);
            let cols: Vec<usize> = ["rsi14_u", "cci20_u", "wr14_u", "st10x3_up", "ema200_s"].iter().map(|x| k.col(&format!("{h}_{x}"))).collect();
            for r in &k.rows {
                assert_eq!(r[dc], "2025-11-17");
                // the decision instant S = the candle END (clamped to the segment: 15:30 here)
                let s = (hms(&r[sc]) + tf_secs(tf)).min(15 * 3600 + 30 * 60);
                if s < first {
                    checked += 1;
                    for &ci in &cols {
                        if r[ci] != "NA" {
                            breaches.push(format!("{tf} {} {h} col {} = {}", r[sc], k.header.as_ref().unwrap()[ci], r[ci]));
                        }
                    }
                }
            }
        }
    }
    assert!(checked > 0);
    assert!(breaches.is_empty(), "{} HTF reads across the K13 reset, e.g. {:?}", breaches.len(), breaches.iter().take(6).collect::<Vec<_>>());
}

#[test]
fn b1_red_vector_15min_rsi_on_2025_11_17_0915_is_na() {
    let k = readings(&ctx(), "3min", &["15min", "60min"], &["2025-11-17"]);
    let (sc, r15, e60, r60) = (k.col("start"), k.col("15min_rsi14_u"), k.col("60min_ema200_s"), k.col("60min_rsi14_u"));
    let r = k.rows.iter().find(|r| r[sc] == "09:15:00").expect("the 09:15 row");
    assert_eq!(r[r15], "NA", "15min_rsi14 on 2025-11-17 09:15");
    assert_eq!(r[e60], "NA", "60min_ema200 on 2025-11-17 09:15");
    assert_eq!(r[r60], "NA", "60min_rsi14 on 2025-11-17 09:15");
}

// ---------------- B2: real Zerodha candles straddling a segment open are kept, k from the session open ----------------

#[test]
fn b2_no_zerodha_candle_overlapping_a_segment_is_dropped() {
    let c = ctx();
    let store = Path::new(defaults::STORE);
    let mut dropped = BTreeMap::new();
    for tf in ["1min", "2min", "3min", "5min", "10min", "15min", "30min", "60min"] {
        let (_, st) = minute_stream(store, "ADANIENT", tf, &c);
        if st.outside_segment != 0 {
            dropped.insert(tf, st.outside_segment);
        }
    }
    assert!(dropped.is_empty(), "Zerodha candles dropped as outside_segment: {dropped:?}");
}

#[test]
fn b2_red_vector_2018_11_07_muhurat_60min_1715_is_kept_as_k1() {
    let c = ctx();
    let store = Path::new(defaults::STORE);
    let (s, _) = minute_stream(store, "ADANIENT", "60min", &c);
    let d = parse_ymd("2018-11-07").unwrap();
    let got: Vec<(String, i64)> = s
        .candles
        .iter()
        .map(|x| x.1)
        .filter(|x| ist_day(x.start) == d)
        .map(|x| (format!("{:02}:{:02}", ist_sod(x.start) / 3600, (ist_sod(x.start) / 60) % 60), x.end - x.start))
        .collect();
    assert_eq!(got.first(), Some(&("17:15".to_owned(), 3600)), "the 17:15-18:15 Muhurat candle (trades 17:30-18:15) is kept: {got:?}");
    let k = readings(&c, "60min", &[], &["2018-11-07"]);
    let (sc, kc) = (k.col("start"), k.col("k"));
    let ks: Vec<(String, String)> = k.rows.iter().map(|r| (r[sc].clone(), r[kc].clone())).collect();
    assert_eq!(ks.first(), Some(&("17:15:00".to_owned(), "1".to_owned())), "{ks:?}");
    assert_eq!(ks.get(1), Some(&("18:15:00".to_owned(), "2".to_owned())), "{ks:?}");
    // the DR Saturday's second segment opens at 11:30: Zerodha's 11:15-12:15 candle is kept (k from the 09:15 open)
    let d2 = parse_ymd("2024-03-02").unwrap();
    let dr: Vec<(i64, i64)> = s.candles.iter().map(|x| x.1).filter(|x| ist_day(x.start) == d2).map(|x| (ist_sod(x.start), ist_sod(x.end))).collect();
    assert!(dr.contains(&(11 * 3600 + 15 * 60, 12 * 3600 + 15 * 60)), "{dr:?}");
}

// ---------------- B3: no decision candle whose END lies inside a market-wide halt enters ----------------

/// The halt windows `[from, resume)`, IST seconds of day: `pull::calendar::HALTS` (branch ag02-r2-wip, commit
/// 7c77f1823894c3b3, file sha256 16 f43ad9cfadee3cab), the union of the cash and index seconds.
type Hms = (u32, u32, u32);

const HALTS: [(&str, Hms, Hms); 3] =
    [("2020-03-13", (9, 21, 9), (10, 21, 1)), ("2020-03-23", (9, 58, 8), (10, 57, 1)), ("2021-02-24", (11, 40, 0), (15, 45, 0))];

fn sec(x: Hms) -> i64 {
    i64::from(x.0 * 3600 + x.1 * 60 + x.2)
}

#[test]
fn b3_red_vectors_the_checker_named() {
    let c = ctx();
    let k1 = readings(&c, "1min", &[], &["2020-03-13", "2020-03-23"]);
    let k3 = readings(&c, "3min", &[], &["2020-03-13", "2020-03-23"]);
    let entry = |k: &Keep, d: &str, s: &str| {
        let (dc, sc, ec) = (k.col("date"), k.col("start"), k.col("entry"));
        k.rows.iter().find(|r| r[dc] == d && r[sc] == s).map(|r| r[ec].clone())
    };
    assert_eq!(entry(&k3, "2020-03-13", "09:21:00").as_deref(), Some("0"), "3min 09:21 (END 09:24) is inside the halt");
    assert_eq!(entry(&k1, "2020-03-13", "09:21:00").as_deref(), Some("0"), "1min 09:21 (END 09:22) is inside the halt");
    assert_eq!(entry(&k3, "2020-03-13", "10:21:00").as_deref(), Some("1"), "3min 10:21 (END 10:24) is after the resumption");
    assert_eq!(entry(&k3, "2020-03-13", "09:18:00").as_deref(), Some("1"), "3min 09:18 (END 09:21:00) is before the halt");
    assert_eq!(entry(&k1, "2020-03-23", "09:58:00").as_deref(), Some("0"), "1min 09:58 (END 09:59) is inside the halt");
    assert_eq!(entry(&k3, "2020-03-23", "09:57:00").as_deref(), Some("0"), "3min 09:57 (END 10:00) is inside the halt");
}

#[test]
fn b3_every_family_refuses_exactly_the_candles_ending_inside_a_halt() {
    let c = ctx();
    let days: Vec<&str> = HALTS.iter().map(|h| h.0).collect();
    let mut bad = Vec::new();
    let mut refused = 0u64;
    for tf in ["1min", "2min", "3min", "4min", "5min", "10min", "15min", "30min", "60min"] {
        let k = readings(&c, tf, &[], &days);
        let (dc, sc, ec, kc) = (k.col("date"), k.col("start"), k.col("entry"), k.col("kind"));
        for r in &k.rows {
            let (_, from, resume) = HALTS.iter().find(|h| h.0 == r[dc]).copied().unwrap();
            let seg_end = if r[dc] == "2021-02-24" { if hms(&r[sc]) < 15 * 3600 { 11 * 3600 + 43 * 60 + 1 } else { 17 * 3600 } } else { 15 * 3600 + 30 * 60 };
            let end = (hms(&r[sc]) + tf_secs(tf)).min(seg_end);
            let in_halt = end >= sec(from) && end < sec(resume);
            let want = if r[kc] == "REGULAR" && !in_halt { "1" } else { "0" };
            refused += u64::from(in_halt);
            if r[ec] != want {
                bad.push(format!("{tf} {} {} entry {} want {want}", r[dc], r[sc], r[ec]));
            }
        }
    }
    assert!(refused > 0);
    assert!(bad.is_empty(), "{} rows, e.g. {:?}", bad.len(), bad.iter().take(8).collect::<Vec<_>>());
}

#[test]
fn b3_halt_rows_are_loaded_from_the_session_table() {
    let c = ctx();
    let with: Vec<(i32, Vec<(i64, i64)>)> = c.sessions.iter().filter(|s| !s.halts.is_empty()).map(|s| (s.day, s.halts.clone())).collect();
    assert_eq!(with.len(), HALTS.len(), "every halt of pull::calendar::HALTS is a row");
    for (d, from, resume) in HALTS {
        let day = parse_ymd(d).unwrap();
        let s = c.sessions.iter().find(|s| s.day == day).unwrap();
        let w = s.halts.first().copied().unwrap();
        assert_eq!((ist_sod(w.0), ist_sod(w.1)), (sec(from), sec(resume)), "{d}");
        assert!(s.in_halt(w.0) && !s.in_halt(w.1) && !s.in_halt(w.0 - 1));
        assert_eq!(s.refusal(w.0), Some(if s.entry { "decision_in_halt" } else { "session_no_entry+decision_in_halt" }));
    }
    let reg = c.sessions.iter().find(|s| s.day == parse_ymd("2020-03-12").unwrap()).unwrap();
    assert!(reg.halts.is_empty() && reg.entry_at(reg.open() + 180));
}

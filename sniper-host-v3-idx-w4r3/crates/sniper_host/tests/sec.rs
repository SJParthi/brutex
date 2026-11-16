//! SECONDS BUILDER (2026-11-14): the seconds families' red vectors on the REAL ADANIENT data (the frozen store's Zerodha minute
//! candles, the GDFL tick store, the pinned common_feat repair 1's session table), each caught by its named mutant:
//! - `sec_eq_minutes` (mutant `m_sec_eq`, a GE boundary byte one level short): the compact kept set of EVERY value of
//!   `cands::outer_values` on the 60-minute family, both sides, every XE context, equals the full family's bit for bit.
//! - `sec_eq_seconds_two_days`: the same on the 30-second family of two sessions, the full family built the minutes driver's way
//!   from a WHOLE-RANGE fold (one `fold_stream`, `cands::build`, `htf_reads` + `attach_htf` over 3 / 5 / 10 / 15 s) against
//!   `sec::build_family` (day by day, the streaming H-* cursor, compact rows).
//! - `day_by_day_fold_equals_whole_range`: the per-day `fold_stream` of the corrected seconds equals one fold over the range.
//! - `htf_pack_round_trip`: the 16-byte H-* read packs and unpacks to itself.
//! - `n1_halt_window_indicative_seconds_absent` (REPAIR ROUND, check-build-sec.md B1; mutant `m_n1`): no halt-window volume-0 second is
//!   a candle (common_feat repair 2, `feat_io::host::session_seconds`), the outage day's traded in-window seconds stay.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic, missing_docs)]

use std::path::{Path, PathBuf};

use common_feat::cal::parse_ymd;
use feat_io::host::{Ctx, Stream};
use feat_io::{defaults, load_ctx};
use sniper_host::esn1::load_inputs;
use sniper_host::sec;

fn data() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("data")
}

fn ctx() -> Ctx {
    load_ctx(Path::new(defaults::DATES), Path::new(defaults::BHAV), parse_ymd("2018-09-07").unwrap()).unwrap()
}

#[test]
fn sec_eq_minutes() {
    let inp = load_inputs(&data()).unwrap();
    let out = sniper_host::secrun::sec_eq_run(&inp, "60min", None, 2).unwrap();
    assert!(out[0].contains("mismatches 0"), "{out:?}");
    assert!(out[0].contains("PASS"), "{out:?}");
}

#[test]
fn sec_eq_seconds_two_days() {
    let inp = load_inputs(&data()).unwrap();
    let p = Some((parse_ymd("2026-03-31").unwrap(), parse_ymd("2026-04-01").unwrap()));
    let out = sniper_host::secrun::sec_eq_run(&inp, "30s", p, 2).unwrap();
    assert!(out[0].contains("mismatches 0"), "{out:?}");
}

#[test]
fn day_by_day_fold_equals_whole_range() {
    let c = ctx();
    let ticks = Path::new(defaults::TICKS);
    let (from, to) = (parse_ymd("2026-03-27").unwrap(), parse_ymd("2026-04-01").unwrap());
    let mut one = Vec::new();
    let mut per_day: Vec<Vec<(usize, common_feat::fold::Candle)>> = Vec::new();
    for (i, s) in c.sessions.iter().enumerate().filter(|(_, s)| s.day >= from && s.day <= to) {
        let d = sec::day_seconds(ticks, "ADANIENT", i, s).unwrap().unwrap();
        one.extend(d.one.iter().copied());
        per_day.push(d.one);
    }
    assert!(per_day.len() >= 3, "three sessions at least");
    for tf in [3_i64, 5, 10, 15, 30] {
        let whole = feat_io::host::fold_stream(&Stream { tf_s: 1, source: "x".into(), candles: one.clone() }, tf, &c, "w").candles;
        let mut days = Vec::new();
        for d in &per_day {
            days.extend(feat_io::host::fold_stream(&Stream { tf_s: 1, source: "x".into(), candles: d.clone() }, tf, &c, "d").candles);
        }
        assert_eq!(days, whole, "{tf} s");
        assert!(!whole.is_empty());
    }
}

#[test]
fn htf_pack_round_trip() {
    let lad = sniper_host::cands::htf_ladders(&load_inputs(&data()).unwrap().dials).unwrap();
    let _ = lad;
    let mut st = 7_u64;
    for _ in 0..2_000 {
        st = st.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        let g = |k: u32, m: u64| u16::try_from((st >> k) % m).unwrap();
        let h = sniper_host::cands::HtfRead {
            rsi: [g(1, 32), g(6, 32)],
            cci: [g(11, 568), g(21, 568)],
            wr: [g(31, 22), g(36, 22)],
            bb: [[g(41, 22), g(46, 22)], [g(51, 22), g(56, 22)]],
            st: [0, 1, 2, 0, 1, 2, 0, 1, u8::try_from(g(60, 3)).unwrap()],
            ema: [3, 2, 1, 0, u8::try_from(g(3, 4)).unwrap()],
        };
        let b = sec::htf_pack_for_test(Some(h));
        assert_eq!(sec::htf_unpack(&b), Some(h));
    }
    assert_eq!(sec::htf_unpack(&sec::htf_pack_for_test(None)), None);
}

/// An hour:minute:second triple.
type Hms = (i64, i64, i64);
/// One N1 case: day, window from, window resume, the absent 3 s / 5 s candle starts, in-window 1-s bars, in-window 3 s candles.
type N1Case = (&'static str, Hms, Hms, Option<(Hms, Hms)>, usize, Option<usize>);

/// REPAIR ROUND (check-build-sec.md B1 = check-build-feat-v1-r2 N1; plan 2.1 HALTS, K11): the re-opening auction's volume-0
/// indicative print inside a market-wide halt `[from, resume)` is never a candle. Red vectors: 3 s 2020-03-13 10:13:21 and 2020-03-23
/// 10:49:21, 5 s 10:13:20 and 10:49:20 are absent, and no 1 / 3 / 5 / 10 / 15 / 30 s candle inside either window has volume 0; on
/// 2021-02-24 (window 11:40:00-15:45:00) the 68 in-window 1-s bars are traded prints and stay, so the 48 in-window 3 s candles stay.
/// Written red first: it FAILED on binary a's source (build-host-v1-sec/r0-red.log) before the re-pin; mutant `m_n1` (= feat_io
/// `m_halt0_folded`) must fail it.
#[test]
fn n1_halt_window_indicative_seconds_absent() {
    let c = ctx();
    let ticks = Path::new(defaults::TICKS);
    let at = common_feat::cal::at;
    // (day, window from, window resume, the 3 s and 5 s candle starts that must be absent, in-window 1-s bars, in-window 3 s candles:
    // on the March 2020 days none is counted, since the only in-window candle a fold can make there is one that STARTS before the
    // resume and holds only seconds traded at or after it, which is asserted instead)
    let cases: [N1Case; 3] = [
        ("2020-03-13", (9, 21, 9), (10, 21, 1), Some(((10, 13, 21), (10, 13, 20))), 0, None),
        ("2020-03-23", (9, 58, 8), (10, 57, 1), Some(((10, 49, 21), (10, 49, 20))), 0, None),
        ("2021-02-24", (11, 40, 0), (15, 45, 0), None, 68, Some(48)),
    ];
    for (day, w0, w1, absent, n1s, n3s) in cases {
        let d = parse_ymd(day).unwrap();
        let (i, s) = c.sessions.iter().enumerate().find(|(_, s)| s.day == d).unwrap();
        let (from, resume) = (at(d, w0.0, w0.1, w0.2), at(d, w1.0, w1.1, w1.2));
        assert_eq!(s.halts, vec![(from, resume)], "{day}: the session table's HALT row");
        let r = sec::day_seconds(ticks, "ADANIENT", i, s).unwrap().unwrap();
        // one indicative second dropped on each March 2020 halt day (2020-03-13 10:13:22, 2020-03-23 10:49:21), none on 2021-02-24
        assert_eq!(r.halt_dropped, u64::from(absent.is_some()), "{day}: halt-window volume-0 seconds dropped");
        let inw: Vec<_> = r.one.iter().filter(|x| x.1.start >= from && x.1.start < resume).collect();
        assert_eq!(inw.len(), n1s, "{day}: in-window 1-s bars");
        assert!(inw.iter().all(|x| x.1.volume > 0), "{day}: an in-window 1-s bar with volume 0");
        let s1 = Stream { tf_s: 1, source: "n1".into(), candles: r.one.clone() };
        for tf in [3_i64, 5, 10, 15, 30] {
            let f = feat_io::host::fold_stream(&s1, tf, &c, "n1").candles;
            let w: Vec<_> = f.iter().filter(|x| x.1.start >= from && x.1.start < resume).collect();
            assert!(w.iter().all(|x| x.1.volume > 0), "{day} {tf} s: an in-window candle with volume 0");
            match n3s {
                Some(n) if tf == 3 => assert_eq!(w.len(), n, "{day}: in-window 3 s candles"),
                None => assert!(w.iter().all(|x| x.1.end > resume), "{day} {tf} s: an in-window candle that ends inside the halt"),
                Some(_) => {}
            }
            if let Some((a3, a5)) = absent {
                let gone = match tf {
                    3 => Some(at(d, a3.0, a3.1, a3.2)),
                    5 => Some(at(d, a5.0, a5.1, a5.2)),
                    _ => None,
                };
                if let Some(t) = gone {
                    assert!(f.iter().all(|x| x.1.start != t), "{day} {tf} s: the indicative candle at {t} is present");
                }
            }
        }
    }
}

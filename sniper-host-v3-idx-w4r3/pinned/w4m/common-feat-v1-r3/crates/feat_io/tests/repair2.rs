//! REPAIR 2 red vectors (state/sniper-req76-20261113/check-build-sec.md, blocking item B1 = check-build-feat-v1-r2 N1; plan 2.1
//! HALTS "a candle spanning a halt folds its traded seconds only", K11 "halt-window rows are the re-opening auction's INDICATIVE
//! values ... never read", SN-X35 "the grid continues across the halt"). Written BEFORE the drop: on the lifted v1 read (the old
//! `feat-run seconds` loop moved into `host::session_seconds` with no drop) the two halt vectors FAILED; they pass on the repair
//! and FAIL again under the named mutant `m_halt0_folded` (halt rows folded). Every vector reads the GDFL tick store read-only
//! through the same public API as `feat-run seconds`.
//!
//! - `n1_red_vectors_halt_indicative_candles_absent`: 3 s 2020-03-13 10:13:21 and 2020-03-23 10:49:21, 5 s 10:13:20 and 10:49:20
//!   are absent; no 1 / 3 / 5 / 10 / 15 / 30 s candle inside either window has volume 0; one second dropped on each day.
//! - `n1_2021_02_24_in_window_rows_unchanged`: the outage day's 68 in-window 1-s bars are traded prints: the read equals the v1
//!   read byte for byte (0 dropped), so its 48 in-window 3 s candles are unchanged.
//! - `n1_drop_is_exactly_the_halt_volume0_seconds`: on the halt days the read equals the v1 read minus exactly the in-window
//!   volume-0 seconds (every traded second keeps its place); on days without a halt it equals the v1 read.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic, clippy::many_single_char_names, missing_docs)]

use std::path::Path;

use common_feat::cal::{at, parse_ymd};
use common_feat::fold::Candle;
use feat_io::data::Session;
use feat_io::host::{self, Ctx, Stream};
use feat_io::{defaults, load_ctx};

fn ctx() -> Ctx {
    load_ctx(Path::new(defaults::DATES), Path::new(defaults::BHAV), parse_ymd("2018-09-07").unwrap()).unwrap()
}

fn session<'a>(c: &'a Ctx, day: &str) -> (usize, &'a Session) {
    let d = parse_ymd(day).unwrap();
    c.sessions.iter().enumerate().find(|(_, s)| s.day == d).unwrap()
}

/// The v1 read (`feat-run seconds` before REPAIR 2, written out here as the reference): every in-segment corrected second.
fn v1_read(s: &Session) -> Vec<Candle> {
    let (y, m, d) = common_feat::cal::civil(s.day);
    let (rows, _) = feat_io::ticks::read_member(&feat_io::ticks::day_path(Path::new(defaults::TICKS), y, m, d), "ADANIENT.NSE.csv").unwrap().unwrap();
    let (secs, _) = feat_io::ticks::fold_seconds(&rows, 0, 86_400);
    let d0 = at(s.day, 0, 0, 0);
    secs.into_iter()
        .map(|x| (d0 + i64::from(x.sod), x))
        .filter(|(t, _)| s.seg_end_of(*t).is_some())
        .map(|(t, x)| Candle { start: t, end: t + 1, open: x.open, high: x.high, low: x.low, close: x.close, volume: x.volume })
        .collect()
}

fn fold(c: &Ctx, i: usize, one: &[Candle], tf: i64) -> Vec<Candle> {
    let s1 = Stream { tf_s: 1, source: "r2".into(), candles: one.iter().map(|x| (i, *x)).collect() };
    host::fold_stream(&s1, tf, c, "r2").candles.into_iter().map(|x| x.1).collect()
}

#[test]
fn n1_red_vectors_halt_indicative_candles_absent() {
    let c = ctx();
    // (day, the HALT row [from, resume), the 3 s and the 5 s indicative candle starts)
    let cases = [
        ("2020-03-13", ((9, 21, 9), (10, 21, 1)), (10, 13, 21), (10, 13, 20)),
        ("2020-03-23", ((9, 58, 8), (10, 57, 1)), (10, 49, 21), (10, 49, 20)),
    ];
    for (day, (w0, w1), a3, a5) in cases {
        let (i, s) = session(&c, day);
        let d = s.day;
        let (from, resume) = (at(d, w0.0, w0.1, w0.2), at(d, w1.0, w1.1, w1.2));
        assert_eq!(s.halts, vec![(from, resume)], "{day}: the session table's HALT row");
        let r = host::session_seconds(Path::new(defaults::TICKS), "ADANIENT", s).unwrap().unwrap();
        assert_eq!(r.halt_dropped, 1, "{day}: halt-window volume-0 seconds dropped");
        for tf in [1_i64, 3, 5, 10, 15, 30] {
            let f = if tf == 1 { r.candles.clone() } else { fold(&c, i, &r.candles, tf) };
            assert!(f.iter().filter(|x| x.start >= from && x.start < resume).all(|x| x.volume > 0), "{day} {tf} s: an in-window candle with volume 0");
            let gone = match tf {
                3 => Some(at(d, a3.0, a3.1, a3.2)),
                5 => Some(at(d, a5.0, a5.1, a5.2)),
                _ => None,
            };
            if let Some(t) = gone {
                assert!(f.iter().all(|x| x.start != t), "{day} {tf} s: the indicative candle starting {t} is present");
            }
        }
    }
}

#[test]
fn n1_2021_02_24_in_window_rows_unchanged() {
    let c = ctx();
    let (i, s) = session(&c, "2021-02-24");
    let d = s.day;
    let (from, resume) = (at(d, 11, 40, 0), at(d, 15, 45, 0));
    assert_eq!(s.halts, vec![(from, resume)], "the session table's HALT row");
    let r = host::session_seconds(Path::new(defaults::TICKS), "ADANIENT", s).unwrap().unwrap();
    let v1 = v1_read(s);
    assert_eq!(r.halt_dropped, 0);
    assert_eq!(r.candles, v1, "the outage day's read equals the v1 read");
    let inw = |v: &[Candle]| v.iter().filter(|x| x.start >= from && x.start < resume).copied().collect::<Vec<_>>();
    assert_eq!(inw(&r.candles).len(), 68, "in-window 1-s bars");
    assert!(inw(&r.candles).iter().all(|x| x.volume > 0), "every in-window 1-s bar is a traded print");
    let (f, f1) = (fold(&c, i, &r.candles, 3), fold(&c, i, &v1, 3));
    assert_eq!(inw(&f).len(), 48, "in-window 3 s candles");
    assert_eq!(inw(&f), inw(&f1), "the 48 in-window 3 s candles are unchanged");
}

#[test]
fn n1_drop_is_exactly_the_halt_volume0_seconds() {
    let c = ctx();
    for day in ["2020-03-13", "2020-03-23", "2021-02-24", "2019-03-15", "2024-06-04"] {
        let (_, s) = session(&c, day);
        let r = host::session_seconds(Path::new(defaults::TICKS), "ADANIENT", s).unwrap().unwrap();
        let v1 = v1_read(s);
        let want: Vec<Candle> = v1.iter().filter(|x| !(x.volume == 0 && s.in_halt(x.start))).copied().collect();
        assert_eq!(r.candles, want, "{day}: the read is the v1 read minus exactly the halt-window volume-0 seconds");
        assert_eq!(r.halt_dropped, u64::try_from(v1.len() - want.len()).unwrap(), "{day}: the drop count");
        assert_eq!(r.price_only, u64::try_from(want.iter().filter(|x| x.volume == 0).count()).unwrap(), "{day}: kept price-only seconds");
        if s.halts.is_empty() {
            assert_eq!(r.halt_dropped, 0, "{day}: no halt, nothing dropped");
        }
    }
}

//! S4 (r3, wave 4; SNIPER-IDXOPT-PLAN-v3.1 s8 S4): the dials lane's 9 K11 tests RE-RUN ON BOTH INDICES (NIFTY, BANKNIFTY), on the real
//! data (the frozen store's Zerodha index candles, the index session table, the index official rows, S1's DP-IDXCAL), through the one
//! dial code path (`feat_io::dials`, `feat_io::idxdials`). Each test asserts K11 the dials lane's way: a datum that arrives AFTER S
//! (later candles, later sessions, the decision session's own official row, a date first listed after the session's file) never
//! moves a reading; a datum that arrived by S moves it. Under `--features m_k11` (every reading one step past S) all nine are red.
//!
//! The nine, mapped to the dials lane's (CHANGES-dials-final: lib `k11_x_nifty_and_x_vix`, `k11_d_exp`, `k11_sn_x34_d_mend`,
//! `k11_g_drng`; dials `k11_g52`, `k11_daily_atr`, `sn_x09_b5`, `x_nifty_and_x_vix_units`, `the_seven_dials...`):
//! 1 `k11_x_peer_and_x_vix_read_the_last_completed_minute`, 2 `k11_d_exp_and_d_dte_read_the_calendar_as_known`,
//! 3 `k11_sn_x34_d_mend_on_the_index_table`, 4 `k11_g_drng_reads_the_session_so_far_on_the_index_stream`,
//! 5 `k11_g52_on_the_index_official_rows`, 6 `k11_daily_atr_on_the_index_official_rows`, 7 `sn_x09_b5_index_g52_unknown_then_ready`,
//! 8 `x_peer_and_x_vix_units_on_the_real_store`, 9 `the_index_dials_wire_on_the_60min_stream`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic, missing_docs)]

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::OnceLock;

use common_feat::cal::{at, parse_ymd, ymd};
use common_feat::fold::Candle;
use feat_io::host::{Ctx, Stream};
use feat_io::idxdials::IdxPrep;
use feat_io::instr::{self, InstrRow};
use feat_io::{defaults, dials, minute_stream_read};

const IXS: [&str; 2] = ["NIFTY", "BANKNIFTY"];

fn rows() -> &'static BTreeMap<String, InstrRow> {
    static R: OnceLock<BTreeMap<String, InstrRow>> = OnceLock::new();
    R.get_or_init(|| instr::load_default().unwrap())
}

struct Ix {
    c: Ctx,
    p: IdxPrep,
    s60: Stream,
}

fn ix(name: &str) -> &'static Ix {
    static M: OnceLock<BTreeMap<String, Ix>> = OnceLock::new();
    let m = M.get_or_init(|| {
        let mut m = BTreeMap::new();
        for n in IXS {
            let r = &rows()[n];
            let c = r.load_ctx().unwrap();
            let store = Path::new(defaults::STORE);
            let p = IdxPrep::prepare(r, &rows()[&r.peer], &c, store).unwrap();
            let (s60, _) = minute_stream_read(store, &r.zread(), "60min", &c);
            m.insert(n.to_owned(), Ix { c, p, s60 });
        }
        m
    });
    &m[name]
}

fn idx(c: &Ctx, d: &str) -> usize {
    c.index_of(parse_ymd(d).unwrap()).unwrap_or_else(|| panic!("{d} is not a session"))
}

/// (1) X-PEER and X-VIX read the peer's / VIX's last COMPLETED 1-minute candle of the decision session only.
#[test]
fn k11_x_peer_and_x_vix_read_the_last_completed_minute() {
    for n in IXS {
        let x = ix(n);
        let d = parse_ymd("2024-03-05").unwrap();
        let s = at(d, 10, 15, 0);
        for bars in [&x.p.peer1, &x.p.vix1] {
            let want = bars.iter().rev().find(|b| b.end <= s && common_feat::cal::ist_day(b.start) == d).map(|b| b.close);
            assert!(want.is_some(), "{n}: a completed minute by 10:15");
            assert_eq!(dials::last_close_at(bars, d, s), want, "{n}: the candle ending exactly at S is completed and read");
            let mut m: Vec<Candle> = bars.clone();
            for b in m.iter_mut().filter(|b| b.end > s) {
                b.close += 777;
            }
            assert_eq!(dials::last_close_at(&m, d, s), want, "{n}: a bar ending after S is never read");
            let k = m.iter().rposition(|b| b.end <= s).unwrap();
            m[k].close += 5;
            assert_eq!(dials::last_close_at(&m, d, s), want.map(|w| w + 5), "{n}: the bar ending by S moves the reading");
            assert_eq!(dials::last_close_at(bars, d, at(d, 9, 15, 30)), None, "{n}: before the first completed minute: unknown");
        }
        // X-PEER's previous close is the PEER's previous session's official close (never the decision session's own row)
        let i = idx(&x.c, "2024-03-05");
        let peer = &rows()[&rows()[n].peer];
        let pb = feat_io::data::bhav_index(&peer.bhav).unwrap();
        assert_eq!(x.p.peer_prev[i], pb.get(&x.c.sessions[i - 1].day).map(|b| b.close), "{n}: the previous session's official close");
    }
}

/// (2) D-EXP and D-DTE read DP-IDXCAL AS KNOWN before the session: the holiday shift 2023-06-28 is an expiry day as known; on the
/// stale windows D-DTE counts to the stale as-known date (MAIN Q6), never to a date first listed after the session's file.
#[test]
fn k11_d_exp_and_d_dte_read_the_calendar_as_known() {
    for n in IXS {
        let x = ix(n);
        let d = |s: &str| parse_ymd(s).unwrap();
        assert_eq!(x.p.cal.d_exp(d("2023-06-28")), Some(true), "{n}: 2023-06-28 is the expiry day (holiday rule)");
        assert_eq!(x.p.cal.d_exp(d("2023-06-29")), None, "{n}: 2023-06-29 is no session");
        assert_eq!(x.p.cal.dte(d("2023-06-27")).value, Some(1));
        assert_eq!(x.p.cal.dte(d("2023-06-28")).value, Some(0));
    }
    let b = ix("BANKNIFTY");
    let d = |s: &str| parse_ymd(s).unwrap();
    // BANKNIFTY 2023-09-01: as known 09-07 (4 sessions: 09-04, 05, 06, 07), the true 09-06 would give 3
    let t = b.p.cal.dte(d("2023-09-01"));
    assert_eq!((t.value, ymd(t.known), t.stale), (Some(4), "2023-09-07".to_owned(), true), "BANKNIFTY 2023-09-01 counts to the stale 09-07");
    assert_eq!(b.p.cal.dte(d("2023-09-04")).value, Some(3));
    // BANKNIFTY 2024-12-26..2025-01-02: as known 2025-01-29
    let t2 = b.p.cal.dte(d("2025-01-01"));
    assert_eq!(ymd(t2.known), "2025-01-29");
    assert_eq!(t2.value, Some(5));
    // the window's last session 2025-01-02 still reads 01-29 (the re-date is first listed in the file OF 01-02); 01-03 reads 01-30
    assert_eq!(ymd(b.p.cal.dte(d("2025-01-02")).known), "2025-01-29");
    assert_eq!(ymd(b.p.cal.dte(d("2025-01-03")).known), "2025-01-30");
    // NIFTY 2019-02-11: the held monthly 02-28 as known (5+); the first weekly 02-14 is listed only from the file of 02-11
    let nf = ix("NIFTY");
    let t3 = nf.p.cal.dte(d("2019-02-11"));
    assert_eq!((t3.value, ymd(t3.known)), (Some(5), "2019-02-28".to_owned()), "NIFTY 2019-02-11 counts to the monthly as known");
    // the data end: no later session in the table, the causal weekday count (labelled)
    let t4 = nf.p.cal.dte(d("2026-10-30"));
    assert!(t4.data_end && t4.value == Some(u8::try_from(t4.weekdays.min(5)).unwrap()), "{t4:?}");
}

/// (3) D-MEND on the index session table: the calendar as known on the prior evening (SN-X34), the same rule and days as ADANIENT.
#[test]
fn k11_sn_x34_d_mend_on_the_index_table() {
    for n in IXS {
        let x = ix(n);
        for (d, want) in [("2024-03-28", Some(true)), ("2026-03-30", Some(true)), ("2026-10-30", Some(true)), ("2024-03-27", Some(false)), ("2019-09-30", Some(true))] {
            assert_eq!(x.p.mend[idx(&x.c, d)], want, "{n}: D-MEND on {d}");
        }
        // a closure of the month's next-to-last day announced ON the realised last session's eve is not known before
        let s = |v: &str| parse_ymd(v).unwrap();
        let days = vec![s("2024-03-26"), s("2024-03-27"), s("2024-04-01")];
        assert_eq!(dials::month_end_flags(&days, &[(s("2024-03-28"), s("2024-03-27"))]).get(1).copied().flatten(), Some(false), "{n}");
        assert_eq!(dials::month_end_flags(&days, &[(s("2024-03-28"), s("2024-03-26"))]).get(1).copied().flatten(), Some(true), "{n}");
    }
}

/// (4) G-DRNG's range so far on the real index stream: the candles of the same session ending by S only; a later candle never moves it.
#[test]
fn k11_g_drng_reads_the_session_so_far_on_the_index_stream() {
    for n in IXS {
        let x = ix(n);
        let i = idx(&x.c, "2024-03-05");
        let base = dials::day_range_so_far(&x.s60);
        let k = base.iter().position(|r| r.0 == i).unwrap();
        let mut s2 = x.s60.clone();
        // the session's later candles get wild extremes: the first candle's range does not move
        for c in s2.candles.iter_mut().filter(|c| c.0 == i).skip(1) {
            c.1.high += 100_000;
            c.1.low -= 50_000;
        }
        assert_eq!(dials::day_range_so_far(&s2)[k], base[k], "{n}: a later candle never moves the range so far");
        // the decision candle itself (END = S) is in it
        s2.candles[k].1.high += 7;
        assert_eq!(dials::day_range_so_far(&s2)[k].2, base[k].2.max(s2.candles[k].1.high));
        // the previous session's candles never enter
        assert_eq!(base[k].2, x.s60.candles[k].1.high, "{n}: the first candle's range is its own");
    }
}

/// (5) G-52H / G-52L over the index official rows: no datum of session t or later moves it; a session inside the 252-session window
/// moves it; the 253rd session before t is outside.
#[test]
fn k11_g52_on_the_index_official_rows() {
    for n in IXS {
        let mut c = rows()[n].load_ctx().unwrap();
        let i = idx(&c, "2024-03-05");
        let base = dials::hl52(&c, i).unwrap();
        let later: Vec<i32> = c.sessions.iter().skip(i).map(|s| s.day).collect();
        for d in &later {
            if let Some(b) = c.bhav.get_mut(d) {
                b.high *= 3;
                b.low /= 3;
            }
        }
        assert_eq!(dials::hl52(&c, i), Some(base), "{n}: session t's own row and later rows never move G-52H / G-52L");
        let w = c.sessions[i - 10].day;
        c.bhav.get_mut(&w).unwrap().high = base.0 * 2;
        assert_eq!(dials::hl52(&c, i).map(|x| x.0), Some(base.0 * 2), "{n}: a window session moves it");
        let mut c2 = rows()[n].load_ctx().unwrap();
        let beyond = c2.sessions[i - dials::WEEKS_52 - 1].day;
        c2.bhav.get_mut(&beyond).unwrap().high = base.0 * 5;
        assert_eq!(dials::hl52(&c2, i), Some(base), "{n}: the 253rd session before t is outside the window");
    }
}

/// (6) The daily ATR of the index official rows known at t never reads session t or later; its only hole is the first session
/// (2018-09-03: S1 has no official close for it).
#[test]
fn k11_daily_atr_on_the_index_official_rows() {
    for n in IXS {
        let mut c = rows()[n].load_ctx().unwrap();
        let i = idx(&c, "2024-03-05");
        let (base, holes) = dials::daily_atr(&c);
        let a = base[i].expect("ATR ready on 2024-03-05");
        assert_eq!(holes.iter().map(|d| ymd(*d)).collect::<Vec<_>>(), vec!["2018-09-03".to_owned()], "{n}");
        let later: Vec<i32> = c.sessions.iter().skip(i).map(|s| s.day).collect();
        for d in &later {
            if let Some(b) = c.bhav.get_mut(d) {
                b.high *= 2;
            }
        }
        assert_eq!(dials::daily_atr(&c).0[i], Some(a), "{n}: session t's own row and later rows never move the ATR known at t");
        let p = c.sessions[i - 1].day;
        c.bhav.get_mut(&p).unwrap().high *= 2;
        assert_ne!(dials::daily_atr(&c).0[i], Some(a), "{n}: the previous session's row moves it");
        assert_eq!(base[idx(&c, "2018-09-14")], None, "{n}: unready right after the CONT start");
    }
}

/// (7) SN-X09 B5 on the index: G-52H is unknown while its window still reaches the session without an official row (2018-09-03) and
/// ready from the 253rd session of the table.
#[test]
fn sn_x09_b5_index_g52_unknown_then_ready() {
    for n in IXS {
        let c = rows()[n].load_ctx().unwrap();
        let first = c.sessions.iter().position(|s| c.bhav.contains_key(&s.day)).unwrap();
        assert_eq!(first, 1, "{n}: the first official row is the second session");
        let ready = first + dials::WEEKS_52;
        assert_eq!(dials::hl52(&c, ready - 1), None, "{n}: unknown on {}", ymd(c.sessions[ready - 1].day));
        assert!(dials::hl52(&c, ready).is_some(), "{n}: ready on {}", ymd(c.sessions[ready].day));
    }
}

/// (8) X-PEER and X-VIX units on the real store: 2020-03-23 (the COVID crash: NIFTY about -13%, NIFTY BANK about -18%, INDIA VIX
/// above 60) and 2024-03-05 (VIX about 15); the reading at the first 60-minute decision equals the last completed minute by S.
#[test]
fn x_peer_and_x_vix_units_on_the_real_store() {
    for n in IXS {
        let x = ix(n);
        let all = x.p.read_stream(&x.c, &x.s60);
        let at_day = |d: &str| all.iter().find(|(i, _, _)| x.c.sessions[*i].day == parse_ymd(d).unwrap()).copied().unwrap();
        let (_, _, r) = at_day("2020-03-23");
        let pb = r.ctx.nifty_bp.map(|(a, b)| a / b).unwrap();
        assert!(pb < -500, "{n}: X-PEER on 2020-03-23: {pb} bp");
        assert!(r.ctx.vix.unwrap() > 5_000, "{n}: X-VIX x100 on 2020-03-23: {:?}", r.ctx.vix);
        let (_, s2, r2) = at_day("2024-03-05");
        let v2 = r2.ctx.vix.unwrap();
        assert!((1_000..2_500).contains(&v2), "{n}: X-VIX x100 on 2024-03-05: {v2}");
        let d = parse_ymd("2024-03-05").unwrap();
        assert!(s2 >= at(d, 10, 15, 0));
        assert_eq!(Some(v2), x.p.vix1.iter().rev().find(|b| b.end <= s2 && common_feat::cal::ist_day(b.start) == d).map(|b| b.close), "{n}");
    }
}

/// (9) The index dials wire on the whole 60-minute stream: every candle is read; D-DTE always has a value 0..5 (as known or the
/// data-end weekday count); D-EXP is ON exactly on the candles of true expiry days; the first G-52H reading is not before the
/// 253rd session; X-PEER is known almost everywhere (the 35 Zerodha-gap sessions of the peer are unknown, never true).
#[test]
fn the_index_dials_wire_on_the_60min_stream() {
    for n in IXS {
        let x = ix(n);
        let all = x.p.read_stream(&x.c, &x.s60);
        assert_eq!(all.len(), x.s60.candles.len(), "{n}");
        // every session with a calendar row reads D-DTE; only 2018-09-03 has none (S1's first file read is the one of 09-03)
        assert!(all.iter().filter(|(i, _, _)| x.p.cal.row(x.c.sessions[*i].day).is_some()).all(|(_, _, r)| r.dte.value.is_some_and(|v| v <= 5)), "{n}: D-DTE");
        let no_row: std::collections::BTreeSet<String> = all.iter().filter(|(i, _, _)| x.p.cal.row(x.c.sessions[*i].day).is_none()).map(|(i, _, _)| ymd(x.c.sessions[*i].day)).collect();
        assert_eq!(no_row.into_iter().collect::<Vec<_>>(), vec!["2018-09-03".to_owned()], "{n}");
        let on = all.iter().filter(|(_, _, r)| r.ctx.exp == Some(true)).count();
        let truth = all.iter().filter(|(i, _, _)| x.p.cal.row(x.c.sessions[*i].day).is_some_and(|w| w.d_exp_true)).count();
        assert_eq!(on, truth, "{n}: D-EXP ON on the true expiry days' candles");
        assert!(on > 0);
        let ready = 1 + dials::WEEKS_52;
        let first = all.iter().find(|(_, _, r)| r.ctx.h52.is_some()).map(|(i, _, _)| *i).unwrap();
        assert!(first >= ready, "{n}: first G-52H on session {first} ({}), ready from {ready}", ymd(x.c.sessions[first].day));
        let known = all.iter().filter(|(_, _, r)| r.ctx.nifty_bp.is_some()).count();
        assert!(known * 100 >= all.len() * 95, "{n}: X-PEER known on {known} of {}", all.len());
        // G-DRNG: the daily ATR is ready `need` (108) official rows after the CONT start, then known on every candle
        let late = parse_ymd("2019-03-01").unwrap();
        assert!(all.iter().filter(|(i, _, _)| x.c.sessions[*i].day >= late).all(|(_, _, r)| r.ctx.drng.is_some()), "{n}: G-DRNG after the ATR warm-up");
        assert!(all.iter().filter(|(i, _, _)| x.c.sessions[*i].day < parse_ymd("2018-10-01").unwrap()).all(|(_, _, r)| r.ctx.drng.is_none()), "{n}: G-DRNG unready right after the CONT start");
    }
}

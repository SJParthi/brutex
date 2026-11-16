//! Red-first cells of the pure module: the ladder edges (OBF34 / CAL-11), the higher-timeframe read (OBF35, m27),
//! same-session patterns (K15, m19), the candle index from the session table's open (K14, m-S9), the causal factor
//! read (OBF31, m20), relative volume without the decision candle (F1), pivots and CPR (CAL-4), the Bollinger
//! exact comparisons, the undo of a back-adjustment, and the step-count bounds.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic, clippy::many_single_char_names, missing_docs)]

use common_feat::S;
use common_feat::anatomy::{AnatomyState, PatCfg, pat};
use common_feat::ca::{CaBook, Event, Kind, Status, through, undo_factor};
use common_feat::cal::{at, candle_index, parse_ymd, weekday, ymd};
use common_feat::fold::{Candle, Fold, Pushed};
use common_feat::ind::{Boll, Cci, Ema, EmaSeed, Rsi, SuperTrend, WilliamsR};
use common_feat::levels::{Hlc, PivZone, Pivots};
use common_feat::mtf::MtfSlot;
use common_feat::num::{Ratio, isqrt, rupees_s};
use common_feat::rvol::Rvol;
use common_feat::series::{CandleCtx, Series, SeriesCfg, need_recursive};
use common_feat::steps::StepHist;

fn c(o: i64, h: i64, l: i64, x: i64) -> Candle {
    Candle { start: 0, end: 60, open: o, high: h, low: l, close: x, volume: 100 }
}

// ---------------- OBF34: CCI 20 reaches 666.67 with one deviating typical price, never 667 ----------------
#[test]
fn cci_limit_666_true_667_never() {
    let mut cci = Cci::new(20);
    for _ in 0..19 {
        cci.fold(&c(100, 100, 100, 100));
    }
    cci.fold(&c(200, 200, 200, 200));
    let (r, _) = cci.reading();
    let r = r.expect("ready");
    assert!(r.ge_int(666), "level 666 must be true at the bound");
    assert!(!r.ge_int(667), "level 667 is never reachable");
    // exactly 2000/3
    assert_eq!(r.num * 3, r.den * 2000);
}

#[test]
fn cci_zero_deviation_is_unknown() {
    let mut cci = Cci::new(20);
    for _ in 0..20 {
        cci.fold(&c(100, 100, 100, 100));
    }
    assert_eq!(cci.reading().0, None, "cci_zero_dev: unknown, never 0");
}

#[test]
fn rsi_100_no_loss_and_0_no_gain() {
    let mut up = Rsi::new(14);
    for i in 0..30 {
        up.fold(100 + i);
    }
    let r = up.reading().expect("ready");
    assert!(r.ge_int(100) && r.le_int(100));
    let mut dn = Rsi::new(14);
    for i in 0..30 {
        dn.fold(1000 - i);
    }
    let r = dn.reading().expect("ready");
    assert!(r.le_int(0) && r.ge_int(0));
}

#[test]
fn rsi_flat_window_is_not_ready() {
    let mut f = Rsi::new(14);
    for _ in 0..30 {
        f.fold(500);
    }
    assert_eq!(f.reading(), None, "a flat window sets neither the RSI 100 nor the RSI 0 bit (3b-F6)");
}

#[test]
fn wr_zero_and_minus_100_and_zero_range() {
    let mut w = WilliamsR::new(14);
    for i in 0..14 {
        w.fold(&c(100 + i, 110 + i, 90 + i, 105 + i));
    }
    w.fold(&c(200, 300, 150, 300));
    let r = w.reading().unwrap();
    assert!(r.ge_int(0) && r.le_int(0), "%R 0 at the high");
    let mut w = WilliamsR::new(14);
    for i in 0..14 {
        w.fold(&c(1000 - i, 1010 - i, 990 - i, 995 - i));
    }
    w.fold(&c(100, 150, 50, 50));
    let r = w.reading().unwrap();
    assert!(r.le_int(-100) && r.ge_int(-100), "%R -100 at the low");
    let mut z = WilliamsR::new(3);
    for _ in 0..3 {
        z.fold(&c(100, 100, 100, 100));
    }
    assert_eq!(z.reading(), None, "wr_zero_range: unknown");
}

#[test]
fn ladder_step_changes_only_at_integer_crossings() {
    // RSI with G = 87.345 x S-ish: bits for 87 true, 88 false; floor bucket 87.
    let r = Ratio::new(100 * 87_345, 100_000).unwrap();
    assert!(r.ge_int(87) && !r.ge_int(88));
    assert_eq!(r.floor_s(), 87_345_000);
    let n = Ratio::new(-25, 100).unwrap(); // %R -0.25
    assert!(n.ge_int(-1) && !n.ge_int(0));
    assert_eq!(n.floor_s(), -250_000);
    assert_eq!(n.ceil_s(), -250_000);
}

// ---------------- OBF35: the higher timeframe's forming candle is never read (m27) ----------------
#[test]
fn mtf_reads_last_completed_candle_only() {
    let mut m: MtfSlot<i32> = MtfSlot::new();
    let s = at(20_000, 9, 24, 0); // a 3-minute decision at 09:24 (the 09:21 candle's end)
    m.put(at(19_999, 15, 30, 0), 7); // yesterday's last 15-min candle
    m.put(at(20_000, 9, 30, 0), 9); // today's 09:15-09:30 candle: still forming at 09:24
    assert_eq!(m.at(s), Some(7), "the forming 09:15-09:30 candle is never read");
    assert_eq!(m.forming_refused(), 1);
    assert_eq!(m.at(at(20_000, 9, 30, 0)), Some(9), "at 09:30 it has ended");
}

// ---------------- K15 / m19: patterns read same-session candles only ----------------
#[test]
fn patterns_are_cold_on_the_first_candle_of_a_session() {
    let mut a = AnatomyState::new(PatCfg::default());
    a.fold(&c(100, 101, 99, 100));
    a.fold(&c(100, 200, 50, 150)); // outside bar inside the session
    a.session();
    let (an, _) = a.fold(&c(100, 300, 10, 200));
    assert_ne!(an.pat_cold & pat::OUTSIDE, 0, "two-candle pattern on a session's first candle is cold");
    assert_eq!(an.pat_long & pat::OUTSIDE, 0);
    let (an2, _) = a.fold(&c(200, 210, 190, 205));
    assert_ne!(an2.pat_long & pat::INSIDE, 0);
}

#[test]
fn engulfing_hammer_marubozu_doji_nr() {
    let mut a = AnatomyState::new(PatCfg::default());
    a.fold(&c(110, 112, 99, 100)); // red
    let (e, _) = a.fold(&c(99, 113, 98, 112)); // green engulfing
    assert_ne!(e.pat_long & pat::ENGULFING, 0);
    let mut a = AnatomyState::new(PatCfg::default());
    let (m, _) = a.fold(&c(100, 200, 100, 200));
    assert_ne!(m.pat_long & pat::MARUBOZU, 0);
    let (d, _) = a.fold(&c(150, 200, 100, 151));
    assert_ne!(d.pat_long & pat::DOJI, 0);
    let (h, _) = a.fold(&c(190, 200, 100, 199)); // hammer: lower wick 90, body 9, upper 1
    assert_ne!(h.pat_long & pat::HAMMER, 0);
    let mut a = AnatomyState::new(PatCfg::default());
    for w in [50, 40, 30] {
        a.fold(&c(100, 100 + w, 100 - w, 100));
    }
    let (n4, _) = a.fold(&c(100, 101, 99, 100));
    assert_ne!(n4.pat_long & pat::NR4, 0);
    assert_ne!(n4.pat_cold & pat::NR7, 0);
}

// ---------------- K14 / m-S9: the candle index counts from the session table's open ----------------
#[test]
fn candle_index_from_the_session_open_not_0915() {
    let day = parse_ymd("2025-10-21").unwrap(); // the afternoon Muhurat, open 13:45
    let open = at(day, 13, 45, 0);
    assert_eq!(candle_index(at(day, 13, 45, 0), open, 180), 1);
    assert_eq!(candle_index(at(day, 13, 51, 0), open, 180), 3);
    let reg = parse_ymd("2026-04-01").unwrap();
    assert_eq!(candle_index(at(reg, 9, 15, 0), at(reg, 9, 15, 0), 180), 1);
    assert_eq!(ymd(reg), "2026-04-01");
    assert_eq!(weekday(reg), 3, "2026-04-01 is a Wednesday");
}

// ---------------- OBF31 / m20: a factor dated after the decision session is never applied ----------------
#[test]
fn ca_future_factor_never_applied() {
    let e = parse_ymd("2025-11-17").unwrap();
    let book = CaBook::new(vec![Event {
        day: e,
        kind: Kind::Rights,
        f: (1525, 1573),
        g: (1, 1),
        status: Status::Proven,
        kite: Some((1525, 1573)),
        kite_vol: Some((1573, 1525)),
    }]);
    let t_before = parse_ymd("2025-11-14").unwrap();
    let u = parse_ymd("2025-11-13").unwrap();
    assert_eq!(book.factor(u, t_before), Ratio::new(1, 1), "decision before e: no factor");
    let f = book.factor(t_before, e).unwrap();
    assert_eq!((f.num, f.den), (1525, 1573));
    assert_eq!(through(251_680, f), 244_000, "2025-11-14 close read on 2025-11-17 through f");
    assert!(book.cont_reset(e));
    // Kite: a 2025-11-14 Zerodha price is re-based; a 2025-11-17 one is not.
    assert_eq!(book.kite_undo(t_before), Some((1525, 1573)));
    assert_eq!(book.kite_undo(e), None);
    assert_eq!(undo_factor(244_000, (1525, 1573), 10), 251_680);
    assert_eq!(book.kite_vol_undo(t_before), Some((1573, 1525)));
    assert_eq!(common_feat::ca::undo_volume(1573, (1573, 1525)), 1525);
    // an unproven event gives no factor across it
    let b2 = CaBook::new(vec![Event { status: Status::ExDateUnknown, ..book.events()[0] }]);
    assert_eq!(b2.factor(t_before, e), None);
    assert_eq!(b2.unknown_events().count(), 1);
}

// ---------------- F1: relative volume never holds the decision candle ----------------
#[test]
fn rvol_excludes_the_decision_candle_and_specials() {
    let mut r = Rvol::new(3, 3, 5);
    for v in [100, 100, 100] {
        r.fold(1, v, true);
    }
    let (x, _) = r.fold(1, 300, true);
    let x = x.unwrap();
    assert!(x.ge_int(3) && x.le_int(3), "300 against the mean 100 of the three sessions before");
    let (y, _) = r.fold(1, 999, false);
    assert_eq!(y, None, "a special session is neither read");
    let (z, _) = r.fold(1, 100, true);
    // ring now 100, 100, 300 (the 999 special was not pushed): 100 x 3 / 500
    assert_eq!(z.unwrap(), Ratio::new(300, 500).unwrap());
}

// ---------------- CAL-4: pivots and CPR of 2026-04-01 from 2026-03-30's official H / L / C ----------------
#[test]
fn cal4_pivots_and_cpr() {
    let p = Pivots::of(Hlc { h: 183_780, l: 175_300, c: 175_880 });
    assert_eq!(rupees_s(Pivots::at_s(p.p6)), "1783.20");
    assert_eq!(rupees_s(Pivots::at_s(p.r1_6)), "1813.40");
    assert_eq!(rupees_s(Pivots::at_s(p.r2_6)), "1868.00");
    assert_eq!(rupees_s(Pivots::at_s(p.s1_6)), "1728.60");
    assert_eq!(rupees_s(Pivots::at_s(p.s2_6)), "1698.40");
    assert_eq!(rupees_s(Pivots::at_s(p.bc6)), "1795.40");
    assert_eq!(rupees_s(Pivots::at_s(p.tc6)), "1771.00");
    assert_eq!(p.zone(183_150), PivZone::R1R2, "the 09:15 candle closes above R1");
    let w = p.cpr_width_bp().unwrap();
    assert!(w.ge(1368, 10) && !w.ge(1369, 10), "CPR width 1.368% of P = 136.8 bp"); // DATA-SCOUT 4
    let last = Pivots::of(Hlc { h: 183_780, l: 175_300, c: 176_400 });
    assert_eq!(rupees_s(Pivots::at_s(last.p6)), "1784.93");
    assert_eq!(rupees_s(Pivots::at_s(last.r1_6)), "1816.87");
}

// ---------------- Bollinger exactness, isqrt, the EMA seeds ----------------
#[test]
fn bollinger_exact_and_display() {
    let mut b = Boll::new(4, 2000);
    for x in [100, 102, 98, 100] {
        b.fold(x);
    }
    let r = b.reading().0.unwrap();
    // mean 100, population var = (0 + 4 + 4 + 0) / 4 = 2, sd = 1.41421..., upper = 102.828...
    assert_eq!(r.basis, 100 * S);
    assert_eq!(r.upper, 100 * S + 2_828_427);
    b.fold(103);
    let r = b.reading().0.unwrap(); // 102, 98, 100, 103: mean 100.75
    assert!(!r.above_upper_by(0) || r.close > 103);
    assert_eq!(isqrt(1_000_000), 1000);
    assert_eq!(isqrt(999_999), 999);
}

#[test]
fn ema_seeds_converge() {
    let mut a = Ema::new(20, EmaSeed::Sma);
    let mut b = Ema::new(20, EmaSeed::First);
    for i in 0..2000 {
        let x = 10_000 + (i % 37) * 3;
        a.fold(x);
        b.fold(x);
    }
    let d = (a.value().unwrap() - b.value().unwrap()).abs();
    assert!(d <= 200, "two seeds converge (differ by {d} at S)");
}

// ---------------- Supertrend: a flip on a close beyond the band ----------------
#[test]
fn supertrend_flips_up_then_down() {
    let mut st = SuperTrend::new(3, 1000);
    for x in [100, 101, 100, 99, 100] {
        st.fold(&c(x, x + 1, x - 1, x));
    }
    let mut flips = 0;
    for x in [110, 120, 130, 90, 80, 70] {
        st.fold(&c(x, x + 1, x - 1, x));
        if st.reading().unwrap().flip {
            flips += 1;
        }
    }
    assert_eq!(flips, 2);
    assert!(!st.reading().unwrap().up);
}

// ---------------- the fold anchors at the open; the step bounds ----------------
#[test]
fn fold_anchors_at_the_session_open_and_refuses_outside() {
    let day = parse_ymd("2026-04-01").unwrap();
    let open = at(day, 9, 15, 0);
    let mut f = Fold::new(240);
    f.segment(open, open, at(day, 15, 30, 0));
    let bar = |m: i64| Candle { start: open + m * 60, end: open + m * 60 + 60, open: 10, high: 11, low: 9, close: 10, volume: 1 };
    assert!(matches!(f.push(&Candle { start: open - 60, ..bar(0) }).0, Pushed::Refused));
    for m in 0..4 {
        assert!(matches!(f.push(&bar(m)).0, Pushed::Folded));
    }
    let (p, steps) = f.push(&bar(4));
    assert!(steps <= 3);
    match p {
        Pushed::Completed(c4) => {
            assert_eq!(c4.start, open);
            assert_eq!(c4.end, open + 240);
            assert_eq!(c4.volume, 4);
        }
        _ => panic!("expected a completed 4-minute candle"),
    }
}

#[test]
fn need_rule_and_step_hist() {
    // EMA 200, SMA seed 200: weight (199/201)^t <= 1e-3 at t = 691 -> need 891
    let n = need_recursive(200, 2, 201, 1000);
    assert!((880..=900).contains(&n), "need {n}");
    let mut h = StepHist::new();
    for v in [5, 5, 5, 7, 9] {
        h.add(v);
    }
    assert_eq!(h.summary(), (5, 9, 9));
}

#[test]
fn series_step_counts_are_bounded_per_candle() {
    let mut s = Series::new(SeriesCfg::sniper(180)).unwrap();
    let day = parse_ymd("2026-04-01").unwrap();
    let open = at(day, 9, 15, 0);
    let mut max = 0;
    for k in 0..400 {
        let x = 100_000 + (k * 7919) % 5000;
        let cd = Candle { start: open + k * 180, end: open + k * 180 + 180, open: x, high: x + 300, low: x - 200, close: x + 50, volume: 1000 };
        let ctx = CandleCtx { session_open: open, day, new_session: k == 0, regular_full: true, cont_reset: false };
        let (_, st) = s.fold(&cd, &ctx);
        max = max.max(st.total());
    }
    // RSI 6 + CCI (2 + 22) + %R (<= 2 + 2 + 28) + BB 2 x 8 + ST 3 x 10 + EMA 5 x 3 + ATR 4 + anatomy 12 + rvol 4 + 2
    assert!(max <= 160, "per-candle steps bounded, got {max}");
}

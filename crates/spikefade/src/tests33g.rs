//! v3.3 fix:v33 NAMED TESTS (MAIN-RULINGS-ENGINE-v22 Addenda 24-28): one per BLOCKING item of the v3.3 gates and the re-check that code
//! closes (STANDING-SCORECARD F1 / F2 / F4 / F5 / F6, EXITS-SCORECARD F-B2, RECHECK-V33 F1). Every case is synthetic and integer; each named
//! mutant (Cargo features in `Cargo.toml`, fix:v33 block) must make its test FAIL. The switches are per test thread; every test starts with
//! all off.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic, clippy::too_many_lines, clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::cast_possible_wrap, clippy::cast_precision_loss)]

use crate::acct::{Sum, Tops};
use crate::cal::{NM, at_sod, days_from_civil, ymd_of};
use crate::cube::{Contract, Cube, Sec, SecTape, Und};
use crate::fixg;
use crate::sw;

fn reset() {
    for s in 0..sw::COUNT {
        sw::set(s, false);
    }
    for s in 0..crate::sw33::COUNT {
        crate::sw33::set(s, false);
    }
    for s in 0..crate::fix33::COUNT {
        crate::fix33::set(s, false);
    }
    for s in 0..fixg::COUNT {
        fixg::set(s, false);
    }
}

/// A stock cube of 2022-07-28 with TATASTEEL (cube lot 425, the stale lot of the 1:10 split: every traded second a multiple of 4,250) and
/// INFY (lot 300, its seconds whole lots of 300, one a single lot), and the seconds tape.
fn split_day() -> (Cube, SecTape) {
    let d = days_from_civil(2022, 7, 28);
    let mut c = Cube { ymd: ymd_of(d), day: d, ..Cube::default() };
    c.unds.push(Und { name: "TATASTEEL".to_owned(), kind: 1, spot_seg: 1, spot_sid: 3499, nearest: 2022_08_25, next: 0 });
    c.unds.push(Und { name: "INFY".to_owned(), kind: 1, spot_seg: 1, spot_sid: 1594, nearest: 2022_08_25, next: 0 });
    let k = |sid: u32, und: u16, lot: u32, strike: i64| Contract { sid, und, kind: 1, right: 0, expiry: 2022_08_25, lot, strike, first_min: 0, first_open: 500, symbol: format!("C{sid}"), n_sec: 0 };
    c.cons = vec![k(1, 0, 425, 10_000), k(2, 0, 425, 10_500), k(3, 1, 300, 150_000), k(4, 1, 300, 151_000)];
    let mut t = SecTape { off: vec![0], rows: Vec::new(), lim: None };
    let sec = |sod: u32, vol: u32| Sec { sod, high: 600, low: 590, vol };
    // TATASTEEL: 4,250 / 8,500 / 12,750 units (1 / 2 / 3 real lots of 4,250), a zero-volume print ignored
    for (i, v) in [4_250_u32, 8_500, 0, 12_750].iter().enumerate() {
        t.rows.push(sec(33_300 + u32::try_from(i).unwrap(), *v));
    }
    t.off.push(u64::try_from(t.rows.len()).unwrap());
    for (i, v) in [8_500_u32, 4_250].iter().enumerate() {
        t.rows.push(sec(34_000 + u32::try_from(i).unwrap(), *v));
    }
    t.off.push(u64::try_from(t.rows.len()).unwrap());
    // INFY: 600 / 300 / 900 units: the GCD is the lot itself
    for (i, v) in [600_u32, 300, 900].iter().enumerate() {
        t.rows.push(sec(33_300 + u32::try_from(i).unwrap(), *v));
    }
    t.off.push(u64::try_from(t.rows.len()).unwrap());
    // INFY's second contract: only 2-lot prints (600 / 1,200); its group still trades single lots
    for (i, v) in [600_u32, 1_200].iter().enumerate() {
        t.rows.push(sec(35_000 + u32::try_from(i).unwrap(), *v));
    }
    t.off.push(u64::try_from(t.rows.len()).unwrap());
    (c, t)
}

/// STANDING F1 (out-of-box G1): a contract whose (underlying, cube lot) group trades only in whole multiples k >= 2 of its cube lot is REFUSED
/// by the sizing (`lot_below_traded_quantum`), never sized with a lot no order could carry; a group trading in single lots is untouched;
/// with the switch off nothing is marked (v3.3). Mutant m_oob_lot_guard_off fails it.
#[test]
fn oob_lot_equals_traded_quantum_on_adjusted_series() {
    reset();
    let (c0, t) = split_day();
    let q = fixg::quanta(&c0, &t);
    assert_eq!(q.get(&(0, 425)).map(|x| x.0), Some(4_250), "TATASTEEL's traded quantum on the ex-date: 4,250 (the real lot), the cube says 425");
    assert_eq!(q.get(&(1, 300)).map(|x| x.0), Some(300), "INFY's traded quantum = its lot");
    let var = std::collections::BTreeMap::new();
    let t10 = 36_000_u32;
    // switch off: nothing marked, the sizing never refuses for the lot (v3.3)
    let mut c = c0.clone();
    fixg::lot_mark(&mut c, &t);
    assert!(c.lotbad.is_empty(), "off: no mark (v3.3 byte for byte)");
    assert_ne!(crate::trade::size_event(&c, 0, t10, &var, true).refuse, crate::trade::Refuse::BadLot);
    // switch on: both TATASTEEL contracts marked x10 and refused; INFY untouched
    fixg::set(fixg::LOT, true);
    let mut c = c0.clone();
    fixg::lot_mark(&mut c, &t);
    assert_eq!(c.lotbad, vec![10, 10, 0, 0], "TATASTEEL x10 (425 -> 4,250), INFY 0");
    assert_eq!(crate::trade::size_event(&c, 0, t10, &var, true).refuse, crate::trade::Refuse::BadLot, "a lot below the traded quantum is refused");
    assert_eq!(crate::trade::size_event(&c, 1, t10, &var, true).refuse, crate::trade::Refuse::BadLot);
    assert_eq!(crate::trade::Refuse::BadLot.code(), "lot_below_traded_quantum");
    assert_ne!(crate::trade::size_event(&c, 2, t10, &var, true).refuse, crate::trade::Refuse::BadLot, "a lot equal to the traded quantum is sized as before");
    assert!(fixg::lot_line().contains("TATASTEEL cube lot 425 x10"), "the group is named in the counts line: {}", fixg::lot_line());
    // the guard reads the GROUP: INFY's second contract trades only 2-lot prints, but its (underlying, lot) group trades single lots
    assert_ne!(crate::trade::size_event(&c, 3, t10, &var, true).refuse, crate::trade::Refuse::BadLot, "a contract of a group trading single lots is sized as before");
    reset();
}

/// STANDING F4 = EXITS F-B2 (dynamic-common D12): GST 18% on the SEBI fee from 2022-07-18 (NSE FATAX52993), on both sides of a sold-first
/// round trip: on Rs 1 crore bought and Rs 1 crore sold the SEBI fee is Rs 10 a side and its GST Rs 1.80 a side, exactly; not before
/// 2022-07-18; not with the switch off (v3.3). Mutant m_v33f_gst_no_sebi fails it.
#[test]
fn v33f_gst_on_sebi_from_20220718() {
    reset();
    let tr = crate::charges::Trip { qty: 100, buy_value: 1_000_000_000, sell_value: 1_000_000_000, entry_orders: 1, exit_orders: 1, broker_orders: 0, rem_kind: 0, settle_value: 0, settle_notional: 0 };
    let on18 = days_from_civil(2022, 7, 18);
    let off15 = days_from_civil(2022, 7, 15);
    let r_off = crate::charges::rates_on(on18).unwrap();
    assert!(!r_off.gst_on_sebi, "switch off: v3.3");
    let (b0, s0) = crate::charges::charges_est_sell(&r_off, &tr);
    fixg::set(fixg::GST, true);
    let r_on = crate::charges::rates_on(on18).unwrap();
    assert!(r_on.gst_on_sebi, "from 2022-07-18");
    assert!(!crate::charges::rates_on(off15).unwrap().gst_on_sebi, "not before 2022-07-18");
    let (b1, s1) = crate::charges::charges_est_sell(&r_on, &tr);
    let sebi_paisa = i64::try_from(r_on.sebi).unwrap(); // Rs 10 per crore = 1,000 paisa on Rs 1 crore
    assert_eq!(sebi_paisa, 1_000, "SEBI Rs 10 per crore in 2022");
    assert_eq!((b1 - b0, s1 - s0), (180, 180), "18% of Rs 10 on each side");
    // the Order Estimator reading (charges_est, buy-first trips) carries the same term
    let (eb0, es0) = crate::charges::charges_est(&r_off, &tr);
    let (eb1, es1) = crate::charges::charges_est(&r_on, &tr);
    assert_eq!((eb1 - eb0, es1 - es0), (180, 180));
    // 2022-07-15: unchanged with the switch on
    let r15 = crate::charges::rates_on(off15).unwrap();
    fixg::set(fixg::GST, false);
    let r15_off = crate::charges::rates_on(off15).unwrap();
    assert_eq!(crate::charges::charges_est_sell(&r15, &tr), crate::charges::charges_est_sell(&r15_off, &tr));
    reset();
}

/// STANDING F5 (dynamic-common D17; Addendum 4): index exposure from the DATED table: on 2018-10-01 (no dated evidence of 2%) NIFTY's ELM is
/// the CONSERVATIVE 3% = Rs 24,593.51 a lot of 75 on the Rs 10,930.45 near-month future, with 2% printed beside = Rs 16,395.68; from the
/// VERIFIED row (2026-11-13: the 14 index Dhan screens) 2%; the 2024-11-20 expiry-day add-on stays on top (3% + 2%); a session the table does
/// not cover is refused; off: the crate's flat 2% (v3.3). Mutant m_v33f_idx_elm_flat_2pct fails it.
#[test]
fn v33f_idx_exposure_dated_conservative() {
    reset();
    let text = "from\tto\trate_h\tbeside_h\tclass\tsource\n2018-01-01\t2026-11-12\t300\t200\tCONSERVATIVE\tAddendum 4\n2026-11-13\t2099-12-31\t200\t200\tVERIFIED\tCALIBRATION-DHAN\n";
    let rows = fixg::IdxElm::parse(text).unwrap();
    let _ = fixg::IDX.set(fixg::IdxElm { rows, src: "test".to_owned() });
    assert!(fixg::IdxElm::parse("from\tto\trate_h\tbeside_h\tclass\tsource\n2018-01-01\t2026-11-12\t300\t200\tC\tx\n2026-11-01\t2099-12-31\t200\t200\tV\ty\n").is_err(), "overlapping rows refused");
    let base = 1_093_045_i64; // Rs 10,930.45, the near-month future in the file
    let r = common_fill::margin::Row { idx: true, expiry: 2018_10_25, right: 0, strike: 1_100_000, span_unit: 100_000, price: 5_000, rate_h: 200, ..common_fill::margin::Row::default() };
    let p = common_fill::margin::short_margin_base_v3(&r, 75, base, base, 2018_10_01).unwrap();
    assert_eq!((p.rate_h, p.elm), (200, 1_639_568), "the crate's flat 2%: Rs 16,395.68");
    // off: untouched (v3.3)
    assert_eq!(crate::mtable::idx_dated(true, p, base, 75, 2018_10_01).unwrap(), (p, 0, 0));
    fixg::set(fixg::IDXELM, true);
    let (q, c, e2) = crate::mtable::idx_dated(true, p, base, 75, 2018_10_01).unwrap();
    assert_eq!((q.rate_h, q.elm, c, e2), (300, 2_459_351, 1, 1_639_568), "3% CONSERVATIVE = Rs 24,593.51 a lot; 2% beside Rs 16,395.68");
    assert_eq!(q.exact, 7_500_000 + 375_000 + 2_459_351, "exact = SPAN + premium + the dated ELM");
    assert_eq!(q.nse_only, 7_500_000 + 2_459_351);
    // a stock row is never touched
    assert_eq!(crate::mtable::idx_dated(false, p, base, 75, 2018_10_01).unwrap(), (p, 0, 0));
    // the VERIFIED row: 2%
    let (v, cv, _) = crate::mtable::idx_dated(true, p, base, 75, 2026_11_16).unwrap();
    assert_eq!((v.rate_h, v.elm, cv), (200, 1_639_568, 2));
    // the expiry-day add-on (from 2024-11-20) on top of the dated base: 3% + 2% = 5%, beside 2% + 2% = 4%
    let rx = common_fill::margin::Row { expiry: 2024_11_21, ..r };
    let px = common_fill::margin::short_margin_base_v3(&rx, 75, base, base, 2024_11_21).unwrap();
    assert_eq!(px.rate_h, 400);
    let (x, _, ex2) = crate::mtable::idx_dated(true, px, base, 75, 2024_11_21).unwrap();
    assert_eq!((x.rate_h, x.elm, ex2), (500, 4_098_919, 3_279_135));
    // a session the table does not cover: refused, never guessed
    assert!(crate::mtable::idx_dated(true, p, base, 75, 2017_06_01).is_err());
    assert!(fixg::idx_cover(&[days_from_civil(2017, 6, 1)]).is_err());
    assert!(fixg::idx_cover(&[days_from_civil(2018, 10, 1), days_from_civil(2026, 11, 16)]).unwrap().contains("CONSERVATIVE 3.00%"));
    reset();
}

/// STANDING F6 (matrix X22, port of round C's v32c_cap_mark_is_minute_high): the running-MTM mark of a minute is its HIGH when the bar is
/// present (a re-spike inside the minute is the mark), else the forward-filled close; the per-minute margin + MTM reading prices that
/// minute at the HIGH. Mutant m_v33f_mark_close fails it.
#[test]
fn xm_mtm_mark_is_minute_high() {
    reset();
    let d = days_from_civil(2026, 11, 4);
    let mut c = Cube { ymd: ymd_of(d), day: d, ..Cube::default() };
    c.unds.push(Und { name: "NIFTY".to_owned(), kind: 0, spot_seg: 0, spot_sid: 1, nearest: 2026_11_10, next: 0 });
    c.cons.push(Contract { sid: 7, und: 0, kind: 0, right: 0, expiry: 2026_11_10, lot: 75, strike: 2_400_000, first_min: 0, first_open: 1_000, symbol: "NIFTY-X-CE".to_owned(), n_sec: 0 });
    c.flags = (0..NM).map(|m| if m % 3 == 0 { 5 } else { 4 }).collect();
    c.high = (0..NM).map(|m| 1_000 + i32::try_from(m).unwrap() * 2 + if m == 60 { 500 } else { 0 }).collect();
    c.close = (0..NM).map(|m| 900 + i32::try_from(m - m % 3).unwrap()).collect();
    let mk = fixg::minute_marks(&c, 0);
    for m in 0..NM {
        let want = if m % 3 == 0 { c.high[m] } else { c.close[m] };
        assert_eq!(mk[m], want, "minute {m}: HIGH when the bar is present, else the forward-filled close");
    }
    assert_eq!(mk[60], 1_000 + 120 + 500, "the re-spike inside minute 60 (09:15 + 60) is the mark");
    // the per-minute cap prices that minute at the HIGH: 100 units sold for Rs 10.00; Rs 19.99 L of margin
    let tr = vec![(at_sod(d, 36_000), at_sod(d, 39_600), 199_900_000_i64, crate::v32::PMark { units: 100, sold: 100_000, mark1: 1 })];
    let pk = crate::v32::margin_mtm_peak(&tr, &mk, &[d]);
    assert_eq!(pk.2, 60, "the peak minute is the re-spike's");
    assert_eq!(pk.0, 199_900_000 + (1_620 * 100 - 100_000), "peak = margin + the loss at the minute's HIGH");
    reset();
}

/// STANDING F2 (dynamic-common D6, out-of-box G2): checklist row 12 follows its counters: the GDFL 2022-07 stock run's dated 0 / UNDATED
/// 10,498 / no limit 1,846 reads NOT COVERED with both counts (it printed COVERED); UNDATED reads alone read PARTIAL; all dated reads stay
/// COVERED; a NOT COVERED switch reading is never raised. Row 8 names GST on the SEBI fee, and without it reads PARTIAL on a run holding a
/// session from 2022-07-18. Row 14 is PARTIAL (cube contents not sha-checked). Mutant m_oob_item12_static fails it.
#[test]
fn oob_checklist_rows_follow_evidence() {
    reset();
    let (s, w) = fixg::item12("COVERED", 10_498, 1_846);
    assert_eq!(s, "NOT COVERED", "contract-days with no limit at all: unsliced orders");
    assert!(w.contains("10498") && w.contains("1846"), "both counts named: {w}");
    assert_eq!(fixg::item12("COVERED", 0, 0), ("COVERED", String::new()), "every read dated");
    assert_eq!(fixg::item12("NOT COVERED", 5, 0).0, "NOT COVERED", "never raised");
    assert_eq!(fixg::item8("COVERED", false, 12).0, "PARTIAL", "GST on SEBI missing on 12 sessions from 2022-07-18");
    assert_eq!(fixg::item8("COVERED", false, 0), ("COVERED", String::new()), "a run before 2022-07-18");
    let (s8, w8) = fixg::item8("COVERED", true, 12);
    assert_eq!(s8, "COVERED");
    assert!(w8.contains("FATAX52993"));
    assert_eq!(fixg::ITEM14.0, "PARTIAL");
    reset();
}

/// STANDING F2 (round C's WORDS_C rule, check-engine-1r S1r, ported): row 12 is PARTIAL when some contract-day read the UNDATED stock file and
/// none lacked a limit. Mutant m_oob_item12_static fails it.
#[test]
fn v33f_item12_partial_when_undated() {
    reset();
    let (s, w) = fixg::item12("COVERED", 7, 0);
    assert_eq!(s, "PARTIAL");
    assert!(w.contains("UNDATED"));
    reset();
}

/// RECHECK-V33 F1 (BLOCKING): under WIN the sweep evaluates only the 6 GROWING rows: an account whose sliding rows a fresh start would
/// decide differently (r0 > 0 and a passed cash check below it) is neither flagged nor left PENDING on them (no unbounded per-account row
/// is held or checkpointed); the growing rows read it as before; with the switch off the sliding rows flag it (v3.3). The candidate set is
/// the distinct SEARCH accounts with a trade that the groups and views keep (bounded). Mutant m_v33g_win_sliding_swept fails it.
#[test]
fn v33g_win_sliding_candidates_bounded_exact() {
    reset();
    let rows = crate::wf::rows21();
    let mut p = vec![crate::kern::Piece { trades: 1, net: 100, ..crate::kern::Piece::default() }; crate::wf::NPIECE];
    p[0].net = 1_000;
    for x in p.iter_mut().skip(1) {
        x.min_pass = 0;
    }
    let sm = Sum { trades: 1, ..Sum::default() };
    let fresh = |on: bool| {
        fixg::set(fixg::WIN, on);
        let mut win: Vec<crate::report3::WinRes> = rows.iter().map(|r| crate::report3::WinRes { row: Some(r.clone()), run: true, ..crate::report3::WinRes::default() }).collect();
        crate::report3::offer_windows(&mut win, &rows, &p, sm, None);
        win
    };
    let off = fresh(false);
    let on = fresh(true);
    for ((r, a), b) in rows.iter().zip(&off).zip(&on) {
        if r.growing {
            assert_eq!((a.checked, a.pending, b.checked, b.pending), (1, 0, 1, 0), "growing row {}: read exactly in both", r.id);
            assert!(b.best.is_some());
        } else {
            assert_eq!(a.pending, 1, "off: sliding row {} flags the account (v3.3)", r.id);
            assert_eq!((b.checked, b.pending, b.rewalked), (0, 0, 0), "on: sliding row {} untouched by the sweep", r.id);
            assert!(b.best.is_none());
        }
    }
    assert!(fixg::skip_sliding(false) && !fixg::skip_sliding(true));
    // the candidate set: SEARCH accounts with a trade, distinct, from every group and view
    let mut t1 = Tops::new(3);
    let mut t2 = Tops::new(3);
    let mk = |list: u32, x: u16, n: u8, net: i64| Sum { list, x, n, net, trades: 2, ndays: 1, days_pos: 1, ..Sum::default() };
    for s in [mk(1, 0, 0, 50), mk(1, 1, 2, 40), mk(2, crate::trade::NL as u16, 1, 30)] {
        t1.offer(s);
        t2.offer(s);
    }
    t2.offer(mk(3, 0, 3, 99));
    let search = vec![true, false];
    let c = fixg::candidates(&[t1, t2], &search, crate::trade::NL);
    let keys: Vec<(u32, u32, u8, u8)> = c.iter().map(|x| (x.0, x.1, x.2, x.3)).collect();
    assert_eq!(keys, vec![(1, 0, 0, 0), (1, 0, 1, 2), (3, 0, 0, 3)], "distinct SEARCH accounts only (setting 1 is COMPARISON), each once");
    reset();
}

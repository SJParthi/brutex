//! CRATE v2.4 (spikefade v3.2, 15 Nov 2026): the named tests of change M3 (margin rule v3, MAIN-RULINGS-ENGINE-v22 addendum 11) and change
//! O1 (the executed-order count, addendum 10). M3 is checked against the operator's 46 Dhan sell screens (margin-calibration
//! `calibration-stocks.tsv`, copied byte for byte as `vectors/dhan-46-screens-calibration-stocks.tsv`, sha256 1ea63d3fca420ad9...): every
//! screen rebuilt to Rs 0.00. O1 is checked on hand-built tapes against its definition (every filling block x every freeze slice).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic, clippy::too_many_lines, clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::cast_possible_wrap, clippy::similar_names, clippy::many_single_char_names, clippy::type_complexity, clippy::cast_lossless, reason = "tests")]

use crate::margin::{ORDER_ADDON_PAISA, Row, parse_paisa, parse_pct_h, short_margin_base, short_margin_base_v3};
use crate::tests_v2::{BASE, rows, t};
use crate::*;

const VEC46: &str = include_str!("../vectors/dhan-46-screens-calibration-stocks.tsv");

/// One screen: (screen, index?, right 0 CALL / 1 PUT, strike paisa, expiry yyyymmdd, qty, SPAN per unit, file price, OTH h, OTM h, base
/// paisa, Dhan's margin paisa, the v2 rebuild paisa).
fn screens() -> Vec<(String, bool, u8, i64, u32, u64, i64, i64, i32, i32, i64, i64, i64)> {
    let mut lines = VEC46.lines();
    let head: Vec<&str> = lines.next().unwrap().split('\t').collect();
    let col = |name: &str| head.iter().position(|h| *h == name).unwrap_or_else(|| panic!("column {name}"));
    let (c_scr, c_grp, c_con, c_qty, c_scan, c_px, c_oth, c_otm, c_base, c_dhan, c_v2) = (col("screen"), col("group"), col("contract"), col("qty"), col("scan_loss_per_unit"), col("option_price_in_file"), col("ael_oth_total_pct"), col("ael_otm_total_pct"), col("exposure_base_value"), col("dhan_margin"), col("v2_rebuilt"));
    let mut out = Vec::new();
    for l in lines.filter(|l| !l.trim().is_empty()) {
        let f: Vec<&str> = l.split('\t').collect();
        let con: Vec<&str> = f[c_con].split(' ').collect();
        let idx = f[c_grp].starts_with("index");
        let pct = |s: &str| if s.is_empty() { -1 } else { i32::try_from(parse_pct_h(s.as_bytes()).unwrap()).unwrap() };
        out.push((
            f[c_scr].to_owned(),
            idx,
            u8::from(con[3] == "PE"),
            parse_paisa(con[2].as_bytes()).unwrap(),
            con[1].parse::<u32>().unwrap(),
            f[c_qty].parse::<u64>().unwrap(),
            parse_paisa(f[c_scan].as_bytes()).unwrap(),
            parse_paisa(f[c_px].as_bytes()).unwrap(),
            pct(f[c_oth]),
            pct(f[c_otm]),
            parse_paisa(f[c_base].as_bytes()).unwrap(),
            parse_paisa(f[c_dhan].as_bytes()).unwrap(),
            parse_paisa(f[c_v2].as_bytes()).unwrap(),
        ));
    }
    out
}

/// ADDENDUM 11: margin rule v3 rebuilds ALL 46 Dhan screens (14 index + 32 stock, 1 lot each, file nsccl.20261116.i1, trade date 16 Nov
/// 2026) to Rs 0.00: SPAN x qty + file price x qty + exposure (index 2% of the near-month future in the file; stock the AEL OTH rate x NSE's
/// close) rounded once + Rs 20. Rule v2 (the OTM row when out of the money) still gives the lane's v2 column, 21 screens too high.
#[test]
fn v32_margin_rule_v3_46_dhan_screens() {
    let v = screens();
    assert_eq!(v.len(), 46, "46 screens in the vector");
    assert_eq!(v.iter().filter(|x| x.1).count(), 14, "14 index screens");
    let (mut v2_over, mut v3_exact) = (0, 0);
    for (scr, idx, right, strike, expiry, qty, span, px, oth, otm, base, dhan, v2) in &v {
        let r = Row { idx: *idx, expiry: *expiry, right: *right, strike: *strike, span_unit: *span, price: *px, oth_h: *oth, otm_h: *otm, ..Row::default() };
        let m3 = short_margin_base_v3(&r, *qty, *base, *base, 2026_11_16).unwrap_or_else(|e| panic!("{scr}: {e}"));
        assert_eq!(m3.exact + ORDER_ADDON_PAISA, *dhan, "{scr}: rule v3 must rebuild Dhan's screen to Rs 0.00");
        v3_exact += 1;
        let m2 = short_margin_base(&r, *qty, *base, *base, 2026_11_16).unwrap_or_else(|e| panic!("{scr}: {e}"));
        assert_eq!(m2.exact + ORDER_ADDON_PAISA, *v2, "{scr}: rule v2 = the lane's v2 rebuild");
        if m2.exact + ORDER_ADDON_PAISA != *dhan {
            v2_over += 1;
            assert!(m2.exact + ORDER_ADDON_PAISA > *dhan, "{scr}: rule v2 only ever overstated");
            assert!(!*idx && m2.otm, "{scr}: only out-of-the-money stock screens differ");
        }
    }
    assert_eq!(v3_exact, 46);
    assert_eq!(v2_over, 21, "rule v2 overstated 21 of the 31 new stock screens (RULE-CHANGE-v3)");
    // the named vectors of RULE-CHANGE-v3: S3 PAYTM 3,24,404.21; S17 GVT&D 87,091.25; S4 ASTRAL 62,465.25; S10 POWERINDIA 2,25,171.63
    for (scr, want) in [("S3", 32_440_421_i64), ("S17", 8_709_125), ("S4", 6_246_525), ("S10", 22_517_163), ("S1", 14_101_250), ("E2", 19_477_235), ("E8", 20_347_754)] {
        let x = v.iter().find(|x| x.0 == scr).unwrap();
        assert_eq!(x.11, want, "{scr}");
    }
}

/// M3: the stock rate is the OTH row whatever the moneyness; an index row is rule v2's; a stock without its OTH rate is refused.
#[test]
fn v32_margin_v3_oth_row_every_moneyness() {
    let r = Row { idx: false, right: 0, strike: 190_000, span_unit: 10_000, price: 500, oth_h: 350, otm_h: 525, ..Row::default() };
    // a CALL at 1,900 is OTM at 1,876: rule v2 reads 5.25%, rule v3 3.50%
    assert_eq!(short_margin_base(&r, 1, 187_600, 187_600, 2026_11_05).map(|x| (x.rate_h, x.otm)), Ok((525, true)));
    assert_eq!(short_margin_base_v3(&r, 1, 187_600, 187_600, 2026_11_05).map(|x| (x.rate_h, x.otm)), Ok((350, true)));
    assert_eq!(short_margin_base_v3(&r, 1, 195_000, 195_000, 2026_11_05).map(|x| (x.rate_h, x.otm)), Ok((350, false)));
    let na = Row { oth_h: -1, ..r };
    assert_eq!(short_margin_base_v3(&na, 1, 187_600, 187_600, 2026_11_05), Err(crate::margin::MarginErr::NoElmRate));
    let ix = Row { idx: true, expiry: 2026_11_03, right: 0, strike: 2_350_000, span_unit: 169_032, price: 665, ..Row::default() };
    assert_eq!(short_margin_base_v3(&ix, 65, 2_326_695, 2_326_695, 2026_11_03).map(|x| x.rate_h), Ok(400), "index expiry day: rule v2's 4%");
    assert_eq!(short_margin_base_v3(&ix, 65, 2_326_695, 2_326_695, 2026_11_02), short_margin_base(&ix, 65, 2_326_695, 2_326_695, 2026_11_02));
}

fn tape_of<'a>(r: &'a [SecIn], ix: &'a SecIndex) -> Tape<'a> {
    Tape::new(r, ix, None).unwrap().with_block(2).unwrap()
}

/// ADDENDUM 10 (O1): every filling block x every freeze slice is an executed order. Three prints of 1,000 units, lot 25, the 100% level:
/// three filling blocks of 1,000. Slice 400 (16 lots): 3 slices a block = 9 orders; a 2,500-unit view: 1,000 / 1,000 / 500 = 3 + 3 + 2 = 8;
/// a cut at 15 s: the two head blocks = 6; no freeze known (0) or a slice holding the whole fill: one order a block.
#[test]
fn v32_sliced_orders_every_block_and_slice() {
    let r = rows(&[(0, 10_000, 10_000, 1_000), (10, 9_900, 9_800, 1_000), (20, 9_700, 9_600, 1_000)]);
    let ix = SecIndex::new(&r, BASE, t(100)).unwrap();
    let tape = tape_of(&r, &ix);
    let d = Dial::new(&[1_000, 2_500, 5_000, 7_500, 10_000]).unwrap();
    let o = Order { side: Side::Sell, anchor: t(0), stop: t(100), unit: 25, limit: None, kind: Kind::Entry };
    let fp = FillPath::record(&tape, &o, &d, LevelMask::all(5), &[3_000; MAXL], None).unwrap();
    let mut st = 0_u32;
    let f = fp.view(4, 3_000, &mut st).unwrap();
    assert_eq!((f.units, f.blocks_filled), (3_000, 3));
    assert_eq!(fp.sliced_orders(4, &f, 400), Ok(9), "3 blocks x ceil(1,000 / 400) = 9 executed orders");
    assert_eq!(fp.sliced_orders(4, &f, 0), Ok(3), "no freeze limit known: one order a filling block");
    assert_eq!(fp.sliced_orders(4, &f, 3_000), Ok(3), "the whole fill fits one slice: one order a block");
    assert_eq!(fp.sliced_orders(4, &f, 1_000), Ok(3), "a slice exactly the block: one order a block");
    assert_eq!(fp.sliced_orders(4, &f, 975), Ok(6), "39 lots a slice: 2 slices a block");
    let f2 = fp.view(4, 2_500, &mut st).unwrap();
    assert_eq!((f2.units, f2.blocks_filled), (2_500, 3));
    assert_eq!(fp.sliced_orders(4, &f2, 400), Ok(8), "1,000 / 1,000 / 500 at 400 a slice: 3 + 3 + 2");
    let f3 = fp.cut_at(&tape, 4, 3_000, t(15), &mut st).unwrap();
    assert_eq!((f3.units, f3.blocks_filled), (2_000, 2));
    assert_eq!(fp.sliced_orders(4, &f3, 400), Ok(6), "a cut at 15 s keeps the two head blocks: 3 + 3");
    let f0 = fp.view(4, 0, &mut st).unwrap();
    assert_eq!(fp.sliced_orders(4, &f0, 400), Ok(0), "nothing filled: no order");
    // the 10% level fills 100 units a block (4 lots): the slices never bind
    let f10 = fp.view(0, 300, &mut st).unwrap();
    assert_eq!(fp.sliced_orders(0, &f10, 400), Ok(f10.blocks_filled));
    assert_eq!(fp.sliced_orders(7, &f10, 400), Err(FillErr::DialInvalid), "a level not recorded is refused");
}

/// O1 against a brute-force count on random tapes: the view's blocks re-walked by `walk` with a recorder, each block's units sliced.
#[test]
fn v32_sliced_orders_equal_a_recorded_walk() {
    let mut g = crate::tests_v2::Rng(0x0032_5EED);
    for case in 0..200_u64 {
        let r = crate::tests_v2::random_day(&mut g, 600, case % 4);
        let ix = SecIndex::new(&r, BASE, t(700)).unwrap();
        let tape = tape_of(&r, &ix);
        let d = Dial::new(&[1_000, 2_500, 5_000, 7_500, 10_000]).unwrap();
        let unit = [1_u64, 25, 75, 15][usize::try_from(case % 4).unwrap()];
        let want = unit * (1 + g.below(400));
        let o = Order { side: Side::Sell, anchor: t(5), stop: t(650), unit, limit: None, kind: Kind::Entry };
        let fp = FillPath::record(&tape, &o, &d, LevelMask::all(5), &[want; MAXL], None).unwrap();
        for l in 0..5 {
            let qty = unit * (1 + g.below(want / unit));
            let mut st = 0;
            let f = fp.view(l, qty, &mut st).unwrap();
            // brute force: a fresh walk of exactly qty units, every filled block sliced
            let slice = unit * (1 + g.below(30));
            let mut w = [0_u64; MAXL];
            w[l] = qty;
            let mut out = [LevelFill::default(); MAXL];
            let mut n = 0_u64;
            let mut rec = |lv: usize, b: &BlockFill| {
                if lv == l && b.why == Why::Filled {
                    n += crate::legs::freeze_slices(b.units, slice);
                }
            };
            walk(&tape, &o, &d, LevelMask::all(5), &mut w, None, None, &mut out, Some(&mut rec)).unwrap();
            assert_eq!(out[l].units, f.units, "case {case} level {l}: the view = a fresh walk");
            assert_eq!(u64::from(fp.sliced_orders(l, &f, slice).unwrap()), n, "case {case} level {l} slice {slice}");
        }
    }
}

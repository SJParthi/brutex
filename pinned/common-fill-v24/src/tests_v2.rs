//! Crate v2 tests beside crate v1's (`tests.rs`, kept and moved with each change): step A0 (section 4's API completed with NO reading
//! changed) first, then each change C1-C8 adds its own red-first tests below its banner. Every figure is an integer (paisa, microseconds,
//! units); synthetic cases say so.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic, clippy::too_many_lines, clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::cast_possible_wrap, clippy::similar_names, clippy::many_single_char_names, clippy::too_many_arguments, clippy::needless_range_loop, clippy::single_match_else, clippy::assertions_on_constants, clippy::type_complexity, clippy::cast_lossless, clippy::items_after_statements, reason = "tests")]

use crate::gaps::GapTables;
use crate::legs::walk_pair;
use crate::money::{Draw, FreeCash, excess, money_in, pay_exposure, reserve};
use crate::rows::{Rec, T1, T2, T3, T4, T5, T6, T7, header, ist};
use crate::rules::Cfg;
use crate::*;

/// A whole second (a hand-built day; offsets below are seconds after it).
pub(crate) const BASE: Us = 1_793_000_000 * US_S;
const LV: [u16; 5] = [1_000, 2_500, 5_000, 7_500, 10_000];

pub(crate) const fn t(s: i64) -> Us {
    BASE + s * US_S
}

pub(crate) fn rows(v: &[(i64, Paisa, Paisa, u64)]) -> Vec<SecIn> {
    v.iter().map(|&(s, high, low, vol)| SecIn { ts: t(s), high, low, vol }).collect()
}

/// A deterministic generator (64-bit LCG, Knuth's constants; the high bits used).
pub(crate) struct Rng(pub(crate) u64);

impl Rng {
    pub(crate) fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 33
    }
    pub(crate) fn below(&mut self, n: u64) -> u64 {
        if n == 0 { 0 } else { self.next() % n }
    }
}

/// A random day of `n` seconds from offset 0 in one of five styles: 0 dense, 1 sparse, 2 THIN (long runs of 1-9 shares: zero capacity at
/// 10%), 3 SPIKY (long runs priced far above / below: past a protection limit), 4 duplicates (repeated stamps).
pub(crate) fn random_day(g: &mut Rng, n: i64, style: u64) -> Vec<SecIn> {
    let mut v = Vec::new();
    let mut px: i64 = 100_000;
    let mut s = 0_i64;
    let mut phase_thin = false;
    let mut phase_hi = false;
    while s < n {
        if g.below(40) == 0 {
            phase_thin = !phase_thin;
        }
        if g.below(30) == 0 {
            phase_hi = !phase_hi;
        }
        let print = match style {
            1 => g.below(4) == 0,
            2 => g.below(3) != 0,
            _ => g.below(10) != 0,
        };
        if print {
            px = (px + g.below(41) as i64 - 20).max(1_000);
            let spread = g.below(3) as i64 * 5;
            let (mut hi, mut lo) = (px + spread, px);
            if style == 3 && phase_hi {
                hi += 2_000;
                lo += 2_000;
            }
            let vol = if style == 2 && phase_thin { 1 + g.below(9) } else { 1 + g.below(400) };
            v.push(SecIn { ts: t(s), high: hi, low: lo, vol });
            if style == 4 && g.below(5) == 0 {
                v.push(SecIn { ts: t(s), high: hi + 5, low: lo - 5, vol: 1 + g.below(50) });
            }
        }
        s += 1;
    }
    v
}

pub(crate) fn index_of(r: &[SecIn], n: i64) -> SecIndex {
    SecIndex::new(r, BASE, t(n)).unwrap()
}

/// One walk of `o` at the dial `levels`, every level wanting `want`; the gap tables when `gaps` is given.
pub(crate) fn run(tape: &Tape<'_>, o: &Order, dial: &Dial, want: Units, money: Option<Paisa>, cap: Option<Paisa>) -> ([LevelFill; MAXL], [Units; MAXL]) {
    let mut w = [0_u64; MAXL];
    for x in w.iter_mut().take(dial.len()) {
        *x = want;
    }
    let mut f = [LevelFill::default(); MAXL];
    let pay = money.map(|p| [Some(p); MAXL]);
    walk(tape, o, dial, LevelMask::all(dial.len()), &mut w, pay.as_ref(), cap, &mut f, None).unwrap();
    (f, w)
}

/// A LevelFill without its work counters (the skips change only the steps).
pub(crate) fn no_steps(f: &LevelFill) -> LevelFill {
    LevelFill { steps: Steps::default(), ..*f }
}

// =====================================================================================================================================
// STEP A0: the gap skips change no fill and no printed counter (design 4.3); first_fill; capacity_between; FillPath; legs; money;
// limits; rules; rows; the old-rule core
// =====================================================================================================================================

#[test]
fn a0_gap_skips_give_the_same_fills_and_counters_as_the_block_by_block_walk_on_4000_random_orders() {
    let mut g = Rng(7);
    let mut skips = 0_u64;
    let mut cases = 0_u64;
    for day in 0..80_u64 {
        let n = 3_000 + g.below(4_000) as i64;
        let r = random_day(&mut g, n, day % 5);
        let ix = index_of(&r, n);
        let plain = Tape::new(&r, &ix, None).unwrap();
        for &unit in &[1_u64, 25] {
            let dial = Dial::new(&LV).unwrap();
            let gt = GapTables::new(&plain, &dial, unit).unwrap();
            let fast = plain.with_gaps(&gt).unwrap();
            for _ in 0..25 {
                let side = if g.below(2) == 0 { Side::Buy } else { Side::Sell };
                let a = g.below(n as u64 / 2) as i64;
                let stop = a + 1 + g.below((n - a) as u64) as i64;
                let lim = match g.below(3) {
                    0 => None,
                    1 => Some(100_000 + g.below(1_500) as i64),
                    _ => Some(99_000 + g.below(3_000) as i64),
                };
                let o = Order { side, anchor: t(a), stop: t(stop), unit, limit: lim, kind: Kind::Entry };
                let want = (1 + g.below(4_000)) / unit * unit + unit;
                let (f1, w1) = run(&plain, &o, &dial, want, None, None);
                let (f2, w2) = run(&fast, &o, &dial, want, None, None);
                assert_eq!(w1, w2, "rests, day {day} order {o:?}");
                for l in 0..5 {
                    assert_eq!(no_steps(&f1[l]), no_steps(&f2[l]), "day {day} level {l} order {o:?}");
                    skips += u64::from(f2[l].steps.skips);
                }
                cases += 1;
            }
        }
    }
    assert!(cases == 4_000 && skips > 200, "the skips must have run: {skips} skips over {cases} orders");
}

#[test]
fn a0_a_thin_contract_1_lot_through_11000_zero_capacity_blocks_skips_with_bounded_steps_ob39() {
    // SYNTHETIC (OB39): 22,500 seconds; every second trades 5 units except one liquid block at the end; a 1-lot (75) order at 10%.
    let mut v: Vec<(i64, Paisa, Paisa, u64)> = (0..22_400).map(|s| (s, 10_000, 10_000, 5)).collect();
    v.push((22_400, 10_010, 10_010, 2_000));
    v.push((22_401, 10_010, 10_010, 2_000));
    let r = rows(&v);
    let ix = index_of(&r, 22_500);
    let plain = Tape::new(&r, &ix, None).unwrap();
    let dial = Dial::new(&[1_000]).unwrap();
    let gt = GapTables::new(&plain, &dial, 75).unwrap();
    let fast = plain.with_gaps(&gt).unwrap();
    let o = Order { side: Side::Buy, anchor: t(0), stop: t(22_500), unit: 75, limit: None, kind: Kind::Entry };
    let (f1, _) = run(&plain, &o, &dial, 75, None, None);
    let (f2, _) = run(&fast, &o, &dial, 75, None, None);
    assert_eq!(no_steps(&f1[0]), no_steps(&f2[0]));
    assert_eq!((f2[0].units, f2[0].zero_take_blocks, f2[0].first.unwrap().block.at), (75, 11_200, t(22_400)));
    assert!(f1[0].steps.blocks > 11_000, "block by block: {} blocks read", f1[0].steps.blocks);
    assert!(f2[0].steps.blocks <= 2 * SKIP_AFTER_T + 2 && f2[0].steps.skips >= 1, "skipped: {:?}", f2[0].steps);
}
const SKIP_AFTER_T: u32 = 16;

#[test]
fn a0_a_run_past_the_protection_limit_is_skipped_by_descent_and_counted_exactly() {
    // SYNTHETIC: a buy with limit 100.00; 5,000 seconds priced 101.00 (past), then 99.00.
    let mut v: Vec<(i64, Paisa, Paisa, u64)> = (0..5_000).map(|s| (s, 10_100, 10_100, 500)).collect();
    v.extend((5_000..5_010).map(|s| (s, 9_900, 9_900, 500)));
    let r = rows(&v);
    let ix = index_of(&r, 6_000);
    let plain = Tape::new(&r, &ix, None).unwrap();
    let dial = Dial::new(&LV).unwrap();
    let gt = GapTables::new(&plain, &dial, 1).unwrap();
    let fast = plain.with_gaps(&gt).unwrap();
    let o = Order { side: Side::Buy, anchor: t(0), stop: t(6_000), unit: 1, limit: Some(10_000), kind: Kind::Entry };
    let (f1, _) = run(&plain, &o, &dial, 100, None, None);
    let (f2, _) = run(&fast, &o, &dial, 100, None, None);
    for l in 0..5 {
        assert_eq!(no_steps(&f1[l]), no_steps(&f2[l]));
        assert_eq!((f2[l].past_limit, f2[l].units, f2[l].first.unwrap().px), (2_500, 100, 9_900));
        assert!(f2[l].steps.blocks < 40 && f2[l].steps.skips == 1, "{:?}", f2[l].steps);
    }
}

#[test]
fn a0_a_want_that_is_not_a_whole_number_of_lots_is_refused_ob8() {
    let r = rows(&[(0, 10_000, 10_000, 4_000)]);
    let ix = index_of(&r, 10);
    let tape = Tape::new(&r, &ix, None).unwrap();
    let o = Order { side: Side::Buy, anchor: t(0), stop: t(10), unit: 75, limit: None, kind: Kind::Entry };
    let mut w = [100, 0, 0, 0, 0, 0, 0, 0];
    let mut f = [LevelFill::default(); MAXL];
    assert_eq!(walk(&tape, &o, &Dial::new(&[2_500]).unwrap(), LevelMask::all(1), &mut w, None, None, &mut f, None), Err(FillErr::LotMismatch));
    let mut w = [150, 0, 0, 0, 0, 0, 0, 0];
    walk(&tape, &o, &Dial::new(&[2_500]).unwrap(), LevelMask::all(1), &mut w, None, None, &mut f, None).unwrap();
    assert_eq!(f[0].units, 150, "whole lots fill");
}

#[test]
fn a0_first_fill_is_the_first_block_fill_of_the_walk_at_every_level() {
    let mut g = Rng(11);
    for day in 0..40_u64 {
        let n = 2_000;
        let r = random_day(&mut g, n, day % 5);
        let ix = index_of(&r, n);
        let tape = Tape::new(&r, &ix, None).unwrap();
        let dial = Dial::new(&LV).unwrap();
        for _ in 0..20 {
            let side = if g.below(2) == 0 { Side::Buy } else { Side::Sell };
            let a = g.below(1_500) as i64;
            let o = Order { side, anchor: t(a), stop: t(n), unit: 1, limit: None, kind: Kind::Entry };
            let want = 1 + g.below(3_000);
            let (f, _) = run(&tape, &o, &dial, want, None, None);
            let mut w = [0_u64; MAXL];
            for x in w.iter_mut().take(5) {
                *x = want;
            }
            let ff = first_fill(&tape, &o, &dial, LevelMask::all(5), &w).unwrap();
            for l in 0..5 {
                assert_eq!(ff[l].map(|b| (b.block, b.px)), f[l].first.map(|b| (b.block, b.px)), "level {l}");
                if let (Some(a), Some(b)) = (ff[l], f[l].first) {
                    assert_eq!(a.units, b.units, "the first block's units");
                }
            }
        }
    }
}

#[test]
fn a0_capacity_between_by_the_prefix_sums_equals_the_block_by_block_sum() {
    let mut g = Rng(13);
    for day in 0..30_u64 {
        let n = 3_000;
        let r = random_day(&mut g, n, day % 5);
        let ix = index_of(&r, n);
        let plain = Tape::new(&r, &ix, None).unwrap();
        let dial = Dial::new(&LV).unwrap();
        let gt = GapTables::new(&plain, &dial, 1).unwrap();
        let fast = plain.with_gaps(&gt).unwrap();
        for _ in 0..40 {
            let a = g.below(2_000) as i64;
            let from = a + g.below(300) as i64;
            let to = (from + g.below(900) as i64).min(n);
            for &bp in &LV {
                let (c1, s1) = capacity_between(&plain, t(a), t(from), t(to), bp, 1).unwrap();
                let (c2, s2) = capacity_between(&fast, t(a), t(from), t(to), bp, 1).unwrap();
                // By hand: the lattice blocks of anchor a wholly inside [from, to).
                let mut c3 = 0_u64;
                let mut at = a;
                while at < from {
                    at += 2;
                }
                while at + 2 <= to {
                    let v: u64 = r.iter().filter(|s| s.ts >= t(at) && s.ts < t(at + 2)).map(|s| s.vol).sum();
                    c3 += v * u64::from(bp) / 10_000;
                    at += 2;
                }
                assert_eq!((c1, c2), (c3, c3), "day {day} a {a} [{from}, {to}) bp {bp}");
                assert!(s2 <= 2, "O(1): {s2} reads (the scan took {s1})");
            }
        }
    }
}

#[test]
fn a0_fillpath_view_payable_and_cut_at_equal_the_walk_and_stay_within_14_probes() {
    let mut g = Rng(17);
    let mut max_probe = 0_u32;
    for day in 0..60_u64 {
        let n = 2_500;
        let r = random_day(&mut g, n, day % 5);
        let ix = index_of(&r, n);
        let tape = Tape::new(&r, &ix, None).unwrap();
        let dial = Dial::new(&LV).unwrap();
        for _ in 0..10 {
            let side = if g.below(2) == 0 { Side::Buy } else { Side::Sell };
            let a = g.below(1_500) as i64;
            let stop = a + 2 + g.below((n - a - 2) as u64) as i64;
            let lim = if g.below(2) == 0 { None } else { Some(100_000 + g.below(500) as i64) };
            let o = Order { side, anchor: t(a), stop: t(stop), unit: 1, limit: lim, kind: Kind::Entry };
            let upto_q = 1 + g.below(5_000);
            let mut upto = [0_u64; MAXL];
            for x in upto.iter_mut().take(5) {
                *x = upto_q;
            }
            let fp = FillPath::record(&tape, &o, &dial, LevelMask::all(5), &upto, None).unwrap();
            for l in 0..5 {
                let one = Dial::new(&[LV[l]]).unwrap();
                for _ in 0..4 {
                    let q = 1 + g.below(upto_q);
                    // view(q) == walk(q)
                    let mut probes = 0;
                    let v = fp.view(l, q, &mut probes).unwrap();
                    max_probe = max_probe.max(probes);
                    let (f, w) = run(&tape, &o, &one, q, None, None);
                    let key = |x: &LevelFill| (x.units, x.value, x.blocks_filled, x.first.map(|b| (b.block.at, b.units, b.px)), x.last.map(|b| (b.block.at, b.units, b.px)), x.e0, x.e1, x.known_at, x.rest_at_stop, x.filled_secs, x.one_price_secs);
                    assert_eq!(key(&v), key(&f[0]), "view day {day} level {l} q {q}");
                    assert_eq!(v.rest_at_stop, w[0]);
                    // payable(P) == the units of an in-walk cut at P
                    if f[0].value > 0 {
                        let pay = g.below(f[0].value as u64 + 10) as i64;
                        let mut probes = 0;
                        let pu = fp.payable(l, q, pay, &mut probes).unwrap();
                        max_probe = max_probe.max(probes);
                        let (fc, _) = run(&tape, &o, &one, q, Some(pay), None);
                        assert_eq!(pu, fc[0].units, "payable day {day} level {l} q {q} pay {pay}");
                    }
                    // cut_at(c) == the walk with stop c
                    let c = a + 1 + g.below((stop - a) as u64) as i64;
                    let mut probes = 0;
                    let cu = fp.cut_at(&tape, l, q, t(c), &mut probes).unwrap();
                    max_probe = max_probe.max(probes);
                    let oc = Order { stop: t(c).min(o.stop), ..o };
                    let (fcut, _) = run(&tape, &oc, &one, q, None, None);
                    assert_eq!(key(&cu), key(&fcut[0]), "cut_at day {day} level {l} q {q} at {c}");
                }
            }
        }
    }
    assert!(max_probe <= 14, "one search a call (D27): {max_probe} probes");
}

#[test]
fn a0_fillpath_probes_are_at_most_14_a_call_at_11250_blocks_d27() {
    // SYNTHETIC: 22,500 seconds, one unit a block at 100% (vol 1 each even second): 11,250 recorded blocks for one level.
    let v: Vec<(i64, Paisa, Paisa, u64)> = (0..22_500).filter(|s| s % 2 == 0).map(|s| (s, 10_000 + s % 7, 10_000, 1)).collect();
    let r = rows(&v);
    let ix = index_of(&r, 22_500);
    let tape = Tape::new(&r, &ix, None).unwrap();
    let dial = Dial::new(&[10_000]).unwrap();
    let o = Order { side: Side::Buy, anchor: t(0), stop: t(22_500), unit: 1, limit: None, kind: Kind::Entry };
    let fp = FillPath::record(&tape, &o, &dial, LevelMask::all(1), &[11_250, 0, 0, 0, 0, 0, 0, 0], None).unwrap();
    assert_eq!(fp.blocks(0), 11_250);
    let mut worst = [0_u32; 3];
    for q in [1_u64, 2, 777, 5_555, 5_556, 5_625, 11_249, 11_250] {
        let mut p = 0;
        let _ = fp.view(0, q, &mut p).unwrap();
        worst[0] = worst[0].max(p);
        for pay in [0_i64, 12_345_678, 55_555_555, i64::MAX] {
            let mut p = 0;
            let _ = fp.payable(0, q, pay, &mut p).unwrap();
            worst[1] = worst[1].max(p);
        }
        let mut p = 0;
        let c = fp.cut_at(&tape, 0, q, t(11_111), &mut p).unwrap();
        worst[2] = worst[2].max(p);
        // cut at 11,111 (off the lattice): 5,555 head blocks; a fill past them re-reads the straddling block [11,110, 11,111): ONE O(1)
        // block read, counted in its Steps (blocks, table_reads), never in the search probes.
        let reread = u32::from(q > 5_555);
        assert_eq!((c.steps.blocks, c.steps.table_reads, c.units), (reread, reread, q.min(5_556)), "cut_at q {q}");
    }
    // ceil(log2(11,251)) = 14: one search a call, each of the three (D27: at most 14 steps a call, step-asserted per call).
    assert!(worst.iter().all(|&w| w <= 14), "FillPath probes view / payable / cut_at at 11,250 blocks: {worst:?}");
    assert_eq!(worst, [14, 14, 14], "the bound is reached: the counter counts every probe");
}

#[test]
fn a0_a_hedge_that_never_binds_gives_two_independent_walks_and_e9_pairs_block_by_block() {
    // E9 (design 6, SYNTHETIC, D14): sell CE A 300 units (4 lots x 75), hedge buy CE B 300 units, T = 09:20:00 (offset 300 here), 25%.
    let hedge = rows(&[(300, 2_000, 2_000, 300), (301, 2_000, 2_000, 300), (302, 2_010, 2_010, 300), (306, 2_020, 2_020, 900)]);
    let sold = rows(&[(300, 5_000, 5_000, 1_200), (301, 5_000, 5_000, 1_200), (302, 4_990, 4_990, 1_200), (304, 4_980, 4_980, 800), (306, 4_970, 4_970, 500)]);
    let (ih, is) = (index_of(&hedge, 1_000), index_of(&sold, 1_000));
    let (th, ts) = (Tape::new(&hedge, &ih, None).unwrap(), Tape::new(&sold, &is, None).unwrap());
    let oh = Order { side: Side::Buy, anchor: t(300), stop: t(400), unit: 75, limit: None, kind: Kind::Leg { leg: 0, open: true } };
    let os = Order { side: Side::Sell, anchor: t(300), stop: t(400), unit: 75, limit: None, kind: Kind::Leg { leg: 1, open: true } };
    let dial = Dial::new(&[2_500]).unwrap();
    let mut want = [300, 0, 0, 0, 0, 0, 0, 0];
    let (mut fh, mut fs) = ([LevelFill::default(); MAXL], [LevelFill::default(); MAXL]);
    let mut got: Vec<(u8, Us, Units, Units, Why)> = Vec::new();
    {
        let mut rec = |_l: usize, leg: u8, b: &BlockFill| got.push((leg, b.block.at, b.units, b.cum, b.why));
        walk_pair((&th, &oh), (&ts, &os), &dial, LevelMask::all(1), &mut want, &mut fh, &mut fs, Some(&mut rec)).unwrap();
    }
    // k0 hedge VOL 600 -> 150 (150); sold VOL 2,400 -> 600 cap, held to 150 (150). k1 hedge VOL 300 -> 75 (225); sold VOL 1,200 -> 75 (225).
    // k2 hedge empty; sold VOL 800 -> 0, paired (225). k3 hedge VOL 900 -> 75 (300); sold VOL 500 -> 125 -> 1 lot 75 (300).
    assert_eq!(got, vec![(0, t(300), 150, 150, Why::Filled), (1, t(300), 150, 150, Why::Filled), (0, t(302), 75, 225, Why::Filled), (1, t(302), 75, 225, Why::Filled), (1, t(304), 0, 225, Why::Paired), (0, t(306), 75, 300, Why::Filled), (1, t(306), 75, 300, Why::Filled)]);
    assert_eq!((fh[0].units, fs[0].units, want[0]), (300, 300, 0));
    assert_eq!((fh[0].value, fs[0].value), (150 * 2_000 + 75 * 2_010 + 75 * 2_020, 150 * 5_000 + 75 * 4_990 + 75 * 4_970));
    // A hedge that never binds (it fills everything in the first block): the pair = two independent walks.
    let mut g = Rng(19);
    for _ in 0..200 {
        let n = 1_500;
        let mut lead = vec![SecIn { ts: t(100), high: 3_000, low: 3_000, vol: 10_000_000 }];
        lead.extend(random_day(&mut g, n, 0).into_iter().filter(|s| s.ts > t(100)));
        let style = g.below(5);
        let follow: Vec<SecIn> = random_day(&mut g, n, style).into_iter().filter(|s| s.ts >= t(100)).collect();
        let (il, ifo) = (index_of(&lead, n), index_of(&follow, n));
        let (tl, tf) = (Tape::new(&lead, &il, None).unwrap(), Tape::new(&follow, &ifo, None).unwrap());
        let ol = Order { side: Side::Buy, anchor: t(100), stop: t(n), unit: 1, limit: None, kind: Kind::Leg { leg: 0, open: true } };
        let of = Order { side: Side::Sell, anchor: t(100), stop: t(n), unit: 1, limit: None, kind: Kind::Leg { leg: 1, open: true } };
        let dial = Dial::new(&LV).unwrap();
        let q = 1 + g.below(2_000);
        let mut want = [0_u64; MAXL];
        for x in want.iter_mut().take(5) {
            *x = q;
        }
        let (mut fl, mut ff) = ([LevelFill::default(); MAXL], [LevelFill::default(); MAXL]);
        walk_pair((&tl, &ol), (&tf, &of), &dial, LevelMask::all(5), &mut want, &mut fl, &mut ff, None).unwrap();
        let (gl, _) = run(&tl, &ol, &dial, q, None, None);
        let (gf, _) = run(&tf, &of, &dial, q, None, None);
        for l in 0..5 {
            let key = |x: &LevelFill| (x.units, x.value, x.blocks_filled, x.e0, x.e1, x.known_at, x.rest_at_stop);
            assert_eq!(key(&fl[l]), key(&gl[l]), "lead level {l}");
            assert_eq!(key(&ff[l]), key(&gf[l]), "follow level {l}");
        }
    }
}

#[test]
fn a0_money_e6_a_to_d_excess_paid_from_free_cash_never_borrowed() {
    // E6 (design 6, SYNTHETIC): slot Rs 1,00,000 margin, leverage 5 (exposure Rs 5,00,000), qty 500, reserve Rs 1,00,000.
    let res = reserve(10_000_000, 500, 100_000, 5, 1);
    assert_eq!(res, 10_000_000, "min(slot, ceil(500 x 1,000.00 / 5))");
    let mi = money_in(50_200_000, 5, 1);
    assert_eq!((mi, excess(mi, res)), (10_040_000, 40_000), "(a) value 5,02,000.00: money in 1,00,400.00, excess Rs 400.00");
    // (a) free cash Rs 400 at the entry's known_at: paid.
    let mut fc = FreeCash::new(10_000_000 + 40_000).unwrap();
    fc.admit(res).unwrap();
    assert_eq!(fc.known(mi, res, 5, 1).unwrap(), Draw::Paid(40_000));
    assert_eq!((fc.free(), fc.paid_n, fc.paid_max), (0, 1, 40_000));
    // (b) free cash 0: cut to P = 5 x (1,00,000 + 0) = 5,00,000.00 -> 498 units at 1,004.00 (4,99,992.00), 2 unpaid.
    let mut fc = FreeCash::new(10_000_000).unwrap();
    fc.admit(res).unwrap();
    let Draw::Cut { pay } = fc.known(mi, res, 5, 1).unwrap() else { panic!("cut") };
    assert_eq!(pay, 50_000_000);
    let r = rows(&[(0, 100_400, 100_400, 2_000), (1, 100_400, 100_400, 2_000)]);
    let ix = index_of(&r, 100);
    let tape = Tape::new(&r, &ix, None).unwrap();
    let o = Order { side: Side::Buy, anchor: t(0), stop: t(100), unit: 1, limit: None, kind: Kind::Entry };
    let (f, _) = run(&tape, &o, &Dial::new(&[2_500]).unwrap(), 500, Some(pay), None);
    assert_eq!((f[0].units, f[0].value, f[0].unpaid), (498, 49_999_200, 2));
    // (d) TWO such orders admitted at one bell, both known at one instant, free cash Rs 400: the first pays, the second is cut to 498.
    let mut fc = FreeCash::new(2 * 10_000_000 + 40_000).unwrap();
    fc.admit(res).unwrap();
    fc.admit(res).unwrap();
    assert_eq!(fc.known(mi, res, 5, 1).unwrap(), Draw::Paid(40_000));
    let Draw::Cut { pay } = fc.known(mi, res, 5, 1).unwrap() else { panic!("the second entry must be cut (OB33)") };
    let (f, _) = run(&tape, &o, &Dial::new(&[2_500]).unwrap(), 500, Some(pay), None);
    assert_eq!((f[0].units, f[0].unpaid, fc.free()), (498, 2, 0), "free cash never below 0; v1's decision snapshot would let both draw");
    assert_eq!(money_in(f[0].value, 5, 1), 9_999_840);
    fc.settle_cut(money_in(f[0].value, 5, 1), res);
    assert_eq!(fc.free(), 0);
    // OB25 / OB26: negative free cash refused; money in equal to the reserve: excess 0, not counted.
    assert_eq!(FreeCash::new(-1), Err(FillErr::NegativeFreeCash));
    assert_eq!(pay_exposure(res, -1, 5, 1), Err(FillErr::NegativeFreeCash));
    let mut fc = FreeCash::new(10_000_000).unwrap();
    fc.admit(res).unwrap();
    assert_eq!(fc.known(res, res, 5, 1).unwrap(), Draw::Paid(0));
    assert_eq!(fc.paid_n, 0);
}

#[test]
fn a0_limits_are_sample_6s_formulas_protection_cap_size_and_the_seed_reference() {
    // Protection (sample-6 protect_t): ltp 1,461.90, 1,000 bp = 10%, tick 10 paise: band ceil(146190 x 1000 / 10000) = 14,619; buy limit
    // round_down(146190 + 14619, 10) = 160,800; a sale round_up(146190 - 14619, 10) = 131,580.
    assert_eq!(limits::protection(146_190, Side::Buy, 1_000, 10), 160_800);
    assert_eq!(limits::protection(146_190, Side::Sell, 1_000, 10), 131_580);
    // The cap price: a long floor(5,00,000.00 / 342) rounded down to 10 paise = 146,190; a short the value cap 146,198.
    assert_eq!(limits::cap_price(10_000_000, 5, 1, 342, Side::Buy, 10), 146_190);
    assert_eq!(limits::cap_price(10_000_000, 5, 1, 342, Side::Sell, 10), 146_198);
    // S11's quantity: floor(5,00,000.00 / 4,016.10) = 124 (E1); floor(5,00,000.00 / 1,461.90) = 342 (E2); lots of 75 at 1,000 units' worth.
    assert_eq!(limits::size_units(10_000_000, 5, 1, 401_610, 1), 124);
    assert_eq!(limits::size_units(10_000_000, 5, 1, 146_190, 1), 342);
    assert_eq!(limits::size_units(10_000_000, 5, 1, 50_000, 75), 975);
    // The seed reference (sample-6 expected_h): the last printed row strictly before t, plus / less one full tick, HALF-PAISA.
    let r = rows(&[(4, 401_610, 401_610, 1_122), (5, 400_020, 400_020, 605)]);
    let ix = index_of(&r, 100);
    let tape = Tape::new(&r, &ix, None).unwrap();
    assert_eq!(limits::size_ref_seed(&tape, t(5), Side::Sell, 10), Some(2 * 401_610 - 20));
    assert_eq!(limits::size_ref_seed(&tape, t(5), Side::Buy, 10), Some(2 * 401_610 + 20));
    assert_eq!(limits::size_ref_seed(&tape, t(4), Side::Buy, 10), None);
    assert_eq!(tape.last_traded_before(t(5)).map(|s| (s.ts, s.high, s.low, s.vol)), Some((t(4), 401_610, 401_610, 1_122)));
    assert_eq!(tape.next_traded(t(5)), Some(t(5)));
    assert_eq!(tape.next_traded(t(6)), None);
}

#[test]
fn a0_the_rules_parser_fails_closed_on_every_fill_row() {
    let ok = [("fill_law", "req70-2s"), ("fill_dial_bp", "1000,2500,5000,7500,10000"), ("fill_timing", "bell"), ("fill_crate_version", "v2"), ("other_key", "x")];
    let c = Cfg::from_rows(&ok).unwrap();
    assert_eq!((c.cap_law, c.dial.levels()), (false, &LV[..]));
    let cap = [("fill_law", "req70-2s-cap"), ("fill_dial_bp", "2500"), ("fill_timing", "bell"), ("fill_crate_version", "v2")];
    assert!(Cfg::from_rows(&cap).unwrap().cap_law);
    for bad in [
        vec![("fill_law", "req58b"), ("fill_dial_bp", "2500"), ("fill_timing", "bell"), ("fill_crate_version", "v2")],
        vec![("fill_dial_bp", "2500"), ("fill_timing", "bell"), ("fill_crate_version", "v2")],
        vec![("fill_law", "req70-2s"), ("fill_dial_bp", "2500,2500"), ("fill_timing", "bell"), ("fill_crate_version", "v2")],
        vec![("fill_law", "req70-2s"), ("fill_dial_bp", "2500"), ("fill_timing", "bell+w"), ("fill_crate_version", "v2")],
        vec![("fill_law", "req70-2s"), ("fill_dial_bp", "2500"), ("fill_timing", "bell"), ("fill_crate_version", "v1")],
        vec![("fill_law", "req70-2s"), ("fill_dial_bp", "2500"), ("fill_timing", "bell"), ("fill_crate_version", "v2"), ("fill_share_bp", "2500")],
        vec![("fill_law", "req70-2s"), ("fill_law", "req70-2s"), ("fill_dial_bp", "2500"), ("fill_timing", "bell"), ("fill_crate_version", "v2")],
        vec![("fill_law", "req70-2s"), ("fill_dial_bp", "25%"), ("fill_timing", "bell"), ("fill_crate_version", "v2")],
        vec![("fill_law", "req70-2s"), ("fill_dial_bp", "2500"), ("fill_timing", "bell"), ("fill_crate_version", "v2"), ("fill_session_table_sha256", "XYZ")],
    ] {
        assert!(Cfg::from_rows(&bad).is_err(), "{bad:?}");
    }
}

#[test]
fn a0_the_db_rows_keep_the_frozen_field_orders_of_design_section_12() {
    assert_eq!(header(&T1), "run_id\tschema\ttool\tlane\tround\tlaw_headline\tlaw_compiled\tdial_bp\ttiming_headline\tcomparisons\tcrate_version\tcommon_fill_sha\tvectors_sha\tmatrix_sha\tlive_timing_sha\trules_sha\tsession_table_sha\tstore_manifest_sha\tbinary_sha\tstarted_ist\tfinished_ist\tload_at_start\tfilled_secs\tone_price_secs\tholes\thole_span_orders\tinblock_stop_trades\tsupersedes_run_id");
    assert_eq!(header(&T2), "run_id\tlaw\tlevel_bp\tsession_date\tsymbol\tcontract_token\texpiry\tstrike\tright\tunit\ttrade_id\torder_id\torder_kind\tside\tanchor_ist\tk\tblock_at_ist\tblock_to_ist\tclipped\tn_traded\tone_price_secs\tblock_high\tblock_low\tblock_vol\tpx_pre\tcap_units\tledger_charge\ttake_units\tprice_paisa\tcum_units\trest_after\twhy\tknown_at_ist");
    assert_eq!(T3.fields.len(), 48);
    assert_eq!(T4.fields.len(), 46);
    assert_eq!(T5.fields.len(), 11);
    assert_eq!(header(&T6), "seq\tist\trun_id\taction\tpath\tsha256\trows\tbytes\twriter\tbinary_sha\tprev_row_sha256\tnote");
    assert_eq!(header(&T7), "run_id\tcombo_id\tlevel_bp\ttrades\tunits\tvalue_paisa\tgross_paisa\tcharges_paisa\tnet_paisa\tunpaid_units\texcess_n\texcess_max_paisa\tclipped_blocks\tzero_take_blocks\tholes_spanned");
    let mut r = Rec::new(T7);
    assert!(r.tsv().is_err(), "a row with a hole is refused");
    for f in T7.fields {
        r.set(f, 1).unwrap();
    }
    assert!(r.set("run_id", 2).is_err(), "a field set twice is refused");
    assert!(Rec::new(T7).set("nope", 1).is_err());
    assert!(Rec::new(T7).set("run_id", "a\tb").is_err());
    assert_eq!(r.tsv().unwrap(), vec!["1"; 15].join("\t"));
    // IST: 2026-10-22 09:15:05 IST = 03:45:05 UTC.
    let us = 1_792_640_705 * US_S;
    assert_eq!(ist(us), "2026-10-22 09:15:05");
    assert_eq!(ist(us + 250_000), "2026-10-22 09:15:05.250000");
    assert_eq!(rows::ist_date(us), "2026-10-22");
    assert_eq!(rows::avg_4dp(49_443_760, 124), "398740.0000");
}

// =====================================================================================================================================
// CHANGE C1 (D5, D29; MAIN 2026-11-12 23:10:48 IST): a block clipped by its order's stop: VOLUME before the stop, PRICE over its full two
// seconds, known at the full block's end (S9, S10). Red first: these fail on step A0 (crate v1's reading: price and volume before the stop,
// known at the clip).
// =====================================================================================================================================

#[test]
fn c1_e4_a_block_clipped_by_its_stop_is_priced_on_both_seconds_and_known_at_the_full_end_ob32() {
    // E4 (design 6, SYNTHETIC): a long entry BUY of 500 from 09:15:05 (offset 5 here), its stop the exit anchor 09:15:10, at 25%.
    // [05, 07) VOL 400 HIGH 100.20 -> 100 @ 100.20; [07, 09) VOL 600 HIGH 100.30 -> 150 @ 100.30; [09, 11) CLIPPED at 10: VOLUME second 09
    // alone (200 -> 50), PRICE max(100.40, 100.70) = 100.70 (px_pre 100.40), known 09:15:11. Value 10,020 + 15,045 + 5,035 = Rs 30,100.00;
    // 200 cancelled at 09:15:10. Crate v1 (sample-6) priced it at 100.40, known 09:15:10: Rs 15.00 kinder on these 50 shares.
    let r = rows(&[(5, 10_020, 10_020, 250), (6, 10_015, 10_015, 150), (7, 10_030, 10_030, 300), (8, 10_025, 10_025, 300), (9, 10_040, 10_040, 200), (10, 10_070, 10_070, 300)]);
    let ix = index_of(&r, 100);
    let tape = Tape::new(&r, &ix, None).unwrap();
    let o = Order { side: Side::Buy, anchor: t(5), stop: t(10), unit: 1, limit: None, kind: Kind::Entry };
    let dial = Dial::new(&[2_500]).unwrap();
    let mut w = [500, 0, 0, 0, 0, 0, 0, 0];
    let mut f = [LevelFill::default(); MAXL];
    let mut got = Vec::new();
    {
        let mut rec = |_l: usize, b: &BlockFill| got.push((b.block.at, b.block.to, b.units, b.px, b.block.high_pre, b.block.known, b.block.clipped));
        walk(&tape, &o, &dial, LevelMask::all(1), &mut w, None, None, &mut f, Some(&mut rec)).unwrap();
    }
    assert_eq!(got, vec![(t(5), t(7), 100, 10_020, 10_020, t(7), false), (t(7), t(9), 150, 10_030, 10_030, t(9), false), (t(9), t(10), 50, 10_070, 10_040, t(11), true)]);
    assert_eq!((f[0].units, f[0].value, f[0].rest_at_stop, f[0].known_at, f[0].clipped), (300, 3_010_000, 200, t(11), 1));
}

#[test]
fn c1_the_first_entry_block_clipped_by_its_stop_units_at_the_clip_price_e1_and_known_at_at_the_full_end_ob23() {
    // SYNTHETIC (OB23): an entry decided at the same bell + 1 s as its exit: its only block [0, 2) clipped at 1: 25% of second 0's 800 = 200
    // units (the exit's quantity, known at the stop); price max(LOW 0, LOW 1) for a sale = min(500.00, 499.00) = 499.00; E1 = known = 2.
    let r = rows(&[(0, 50_000, 50_000, 800), (1, 49_900, 49_900, 4_000)]);
    let ix = index_of(&r, 100);
    let tape = Tape::new(&r, &ix, None).unwrap();
    let o = Order { side: Side::Sell, anchor: t(0), stop: t(1), unit: 1, limit: None, kind: Kind::Entry };
    let (f, w) = run(&tape, &o, &Dial::new(&[2_500]).unwrap(), 1_000, None, None);
    assert_eq!((f[0].units, f[0].value, w[0]), (200, 200 * 49_900, 800));
    assert_eq!((f[0].e0, f[0].e1, f[0].known_at), (t(0), t(2), t(2)));
    assert_eq!(f[0].first.map(|b| (b.block.to, b.block.low_pre, b.block.low)), Some((t(1), 50_000, 49_900)));
}

#[test]
fn c1_a_clipped_block_whose_two_second_price_is_past_the_protection_limit_fills_nothing() {
    // SYNTHETIC: a buy limit 100.50; the clipped block's second before the stop at 100.40 (within), the second at the stop 100.70 (past):
    // crate v1 filled at 100.40; crate v2 prices the block at 100.70, past the limit: nothing fills there (past_limit 1).
    let r = rows(&[(0, 10_040, 10_040, 1_000), (1, 10_070, 10_070, 1_000)]);
    let ix = index_of(&r, 100);
    let tape = Tape::new(&r, &ix, None).unwrap();
    let o = Order { side: Side::Buy, anchor: t(0), stop: t(1), unit: 1, limit: Some(10_050), kind: Kind::Entry };
    let (f, w) = run(&tape, &o, &Dial::new(&[2_500]).unwrap(), 100, None, None);
    assert_eq!((f[0].units, f[0].past_limit, w[0]), (0, 1, 100));
}

#[test]
fn c1_a_resting_target_limit_never_fills_on_a_trade_after_its_cancel() {
    // SYNTHETIC (C1 repair 1): a resting sale limit at 101.00 cancelled at 10:00:01; its block [0, 2) clipped at 1: second 0 trades at
    // 100.50 (not beyond), second 1 (after the cancel) at 102.00 (beyond): nothing fills (the two-second price is a market order's worst,
    // never a resting limit's fill condition). Its known end is the full block's (C1).
    let r = rows(&[(0, 10_050, 10_050, 1_000), (1, 10_200, 10_200, 1_000)]);
    let ix = index_of(&r, 100);
    let tape = Tape::new(&r, &ix, None).unwrap();
    let o = Order { side: Side::Sell, anchor: t(0), stop: t(1), unit: 1, limit: None, kind: Kind::TargetLimit { px: 10_100 } };
    let (f, w) = run(&tape, &o, &Dial::new(&[2_500]).unwrap(), 100, None, None);
    assert_eq!((f[0].units, w[0]), (0, 100));
    let o2 = Order { stop: t(5), ..o };
    let (f2, _) = run(&tape, &o2, &Dial::new(&[2_500]).unwrap(), 100, None, None);
    assert_eq!((f2[0].units, f2[0].value, f2[0].known_at), (100, 100 * 10_100, t(2)), "uncut, the same block traded beyond fills AT the limit");
}

#[test]
fn c1_fillpath_cut_at_re_reads_the_straddling_block_with_the_two_second_price() {
    // SYNTHETIC: a record to 10:00:10; cut at 10:00:03: blocks [0, 2) full, [2, 4) clipped at 3 (volume of second 2, price over 2 and 3).
    let r = rows(&[(0, 10_000, 10_000, 400), (2, 10_010, 10_010, 400), (3, 10_090, 10_090, 400), (4, 10_020, 10_020, 400)]);
    let ix = index_of(&r, 100);
    let tape = Tape::new(&r, &ix, None).unwrap();
    let o = Order { side: Side::Buy, anchor: t(0), stop: t(10), unit: 1, limit: None, kind: Kind::Entry };
    let dial = Dial::new(&[2_500]).unwrap();
    let fp = FillPath::record(&tape, &o, &dial, LevelMask::all(1), &[1_000, 0, 0, 0, 0, 0, 0, 0], None).unwrap();
    let mut p = 0;
    let c = fp.cut_at(&tape, 0, 1_000, t(3), &mut p).unwrap();
    assert_eq!((c.units, c.value, c.known_at), (200, 100 * 10_000 + 100 * 10_090, t(4)));
}

// =====================================================================================================================================
// CHANGE C2 (S2; MAIN 2026-11-12 23:10:48 IST): a row with volume > 0 and an invalid price refuses the instrument-day (bad_print, fail
// closed); a row with volume 0 is no print anywhere. Red first: these fail on step C1 (crate v1's reading: such rows ignored, counted).
// =====================================================================================================================================

#[test]
fn c2_a_bad_print_refuses_the_instrument_day_ob6() {
    for (h, l) in [(0, 0), (9_000, 9_500), (10_000, -5), (-1, -2), (10_000, 0)] {
        let r = rows(&[(0, 10_000, 9_990, 100), (1, h, l, 50), (2, 10_010, 10_000, 100)]);
        assert_eq!(SecIndex::new(&r, BASE, t(100)).err(), Some(FillErr::BadPrint), "HIGH {h} LOW {l} with volume");
    }
    let ok = rows(&[(0, 10_000, 9_990, 100), (1, 9_000, 9_500, 0), (2, 10_010, 10_000, 100)]);
    assert!(SecIndex::new(&ok, BASE, t(100)).is_ok(), "the same prices with no volume: no print, the day stands");
}

#[test]
fn c2_a_row_with_no_volume_is_no_print_for_the_walk_the_extremes_the_sizing_and_the_next_print_ob5() {
    // SYNTHETIC (OB5): second 3 holds HIGH 110.00 / LOW 90.00 with no volume.
    let r = rows(&[(0, 10_000, 10_000, 100), (3, 11_000, 9_000, 0), (5, 10_010, 10_010, 100)]);
    let ix = index_of(&r, 100);
    let mm = RangeMm::new(&r);
    for tape in [Tape::new(&r, &ix, None).unwrap(), Tape::new(&r, &ix, Some(&mm)).unwrap()] {
        assert_eq!(tape.extreme(t(0), t(10), true), Some((10_010, t(5))));
        assert_eq!(tape.extreme(t(0), t(10), false), Some((10_000, t(0))));
        assert_eq!(tape.extreme(t(3), t(4), true), None, "a span of only no-volume rows has no price");
        assert_eq!(tape.last_traded_before(t(5)).map(|s| s.ts), Some(t(0)));
        assert_eq!(tape.next_traded(t(1)), Some(t(5)));
        assert_eq!(runs(&tape, t(0), t(10), true, 100 * 10_000, 100), ((1_000, t(5)), (0, t(0))));
    }
}

// =====================================================================================================================================
// CHANGE C3 (S5; MAIN 2026-11-12 23:10:48 IST): a block's n counts its traded SECONDS, not its rows (a printed field only). Red first:
// fails on step C2 (crate v1's reading: rows).
// =====================================================================================================================================

#[test]
fn c3_a_blocks_n_counts_traded_seconds_and_a_repeated_stamp_counts_once_b16() {
    // SYNTHETIC (B16): second 0 holds three rows, second 1 two: n = 2 seconds (crate v1: 5 rows); the price and volume merge all rows.
    let r = rows(&[(0, 10_000, 9_990, 100), (0, 10_020, 9_980, 50), (0, 10_010, 10_000, 25), (1, 10_030, 10_000, 200), (1, 10_000, 9_970, 25), (2, 10_000, 10_000, 10)]);
    let ix = index_of(&r, 100);
    let tape = Tape::new(&r, &ix, None).unwrap();
    let b = tape.block(t(0), t(100)).unwrap();
    assert_eq!((b.n, b.n_secs, b.high, b.low, b.vol, b.one_price), (2, 2, 10_030, 9_970, 400, 0));
    let b = tape.block(t(2), t(100)).unwrap();
    assert_eq!((b.n, b.n_secs, b.one_price), (1, 1, 1));
}

// =====================================================================================================================================
// CHANGE C4 (S11, D4; MAIN 2026-11-12 23:10:48 IST): the size reference is the last traded second strictly before T, its HIGH for a buy,
// its LOW for a sale, NO tick (crate v1 / sample-6: expected_h, REQ 58b's decision reference plus or less one dated tick). Red first:
// fails on step C3 (the stand-in carries the seed reading).
// =====================================================================================================================================

#[test]
fn c4_e1_kei_and_e2_hcltech_size_on_the_last_traded_second_before_the_bell_with_no_tick() {
    // E1 (REAL, design 6): KEI 2026-10-22, a SHORT decided at the bell 09:15:05 (offset 5 here): 09:15:04 traded 1,122 at 4016.10:
    // reference 4016.10 (no tick); 5,00,000 / 4016.10 = 124.498 -> 124. The seed's 4016.00 (LOW less one 10-paise tick) also gives 124.
    let kei = rows(&[(4, 401_610, 401_610, 1_122), (5, 400_020, 400_020, 605), (6, 398_740, 398_740, 1_294)]);
    let ix = index_of(&kei, 100);
    let tape = Tape::new(&kei, &ix, None).unwrap();
    assert_eq!(limits::size_ref(&tape, t(5), Side::Sell), Some(401_610));
    assert_eq!(limits::size_units(10_000_000, 5, 1, 401_610, 1), 124);
    // E2 (REAL, design 6): HCLTECH 2026-10-23, a LONG at the bell 09:15:05: 09:15:04 traded 2,693 at 1461.90: reference 1461.90 (no tick);
    // 5,00,000 / 1461.90 floors to 342. The seed (1461.90 + one 10-paise tick = 1462.00) gave 341.
    let hcl = rows(&[(4, 146_190, 146_190, 2_693), (5, 146_170, 146_170, 5_521), (6, 146_120, 146_120, 6_309)]);
    let ix = index_of(&hcl, 100);
    let tape = Tape::new(&hcl, &ix, None).unwrap();
    let r = limits::size_ref(&tape, t(5), Side::Buy).unwrap();
    assert_eq!((r, limits::size_units(10_000_000, 5, 1, r, 1)), (146_190, 342));
    assert_eq!(limits::size_units(10_000_000, 5, 1, i64::try_from(limits::size_ref_seed(&tape, t(5), Side::Buy, 10).unwrap() / 2).unwrap(), 1), 341, "crate v1's 341");
}

#[test]
fn c4_the_size_reference_merges_a_repeated_stamp_and_reads_nothing_at_or_after_t() {
    // SYNTHETIC: second 3 has two rows (HIGH 100.30 and 100.50); second 5 = T is never read; no trade before T: no reference.
    let r = rows(&[(3, 10_030, 10_000, 10), (3, 10_050, 9_980, 10), (5, 11_000, 9_000, 10)]);
    let ix = index_of(&r, 100);
    let tape = Tape::new(&r, &ix, None).unwrap();
    assert_eq!((limits::size_ref(&tape, t(5), Side::Buy), limits::size_ref(&tape, t(5), Side::Sell)), (Some(10_050), Some(9_980)));
    assert_eq!(limits::size_ref(&tape, t(3), Side::Buy), None, "nothing traded strictly before T: no_reference");
}

// =====================================================================================================================================
// CHANGE C5 (D11; MAIN 2026-11-12 23:10:48 IST): a target is a DECIDED exit: it triggers in the first second whose FAVOURABLE extreme
// REACHES the target level; the exit order is anchored at max(that second's end, E1) and fills by blocks at their worst (crate v1 /
// sample-6: a resting limit filled AT its level in a block traded strictly BEYOND it). Red first: fails on step C4 (the stand-in).
// =====================================================================================================================================

#[test]
fn c5_a_target_triggers_on_the_first_second_that_reaches_it_and_its_exit_is_anchored_at_that_seconds_end() {
    // SYNTHETIC: a long, entry known at 10:00:02 (E1); target 101.00: second :05 HIGH 101.00 (it REACHES, not beyond), :07 HIGH 101.20.
    // Decided: trigger :05, anchor :06. (A resting limit would have filled in the block holding :07, at 101.00.)
    let r = rows(&[(0, 10_000, 10_000, 500), (3, 10_050, 10_040, 500), (5, 10_100, 10_060, 500), (6, 10_070, 10_030, 800), (7, 10_120, 10_090, 500)]);
    let ix = index_of(&r, 100);
    let mm = RangeMm::new(&r);
    for tape in [Tape::new(&r, &ix, None).unwrap(), Tape::new(&r, &ix, Some(&mm)).unwrap()] {
        let mut st = 0;
        let tg = path::target_trigger(&tape, true, 10_100, t(2), t(60), t(2), &mut st).unwrap();
        assert_eq!((tg.sec, tg.anchor, tg.level), (t(5), t(6), 10_100));
        // A short's target 100.40 reached by :03's LOW 100.40: trigger :03, anchor :04.
        let tg = path::target_trigger(&tape, false, 10_040, t(2), t(60), t(2), &mut st).unwrap();
        assert_eq!((tg.sec, tg.anchor), (t(3), t(4)));
        // Watched from E1 on (the entry's known instant); the anchor is never before E1.
        let tg = path::target_trigger(&tape, true, 10_000, t(0), t(60), t(2), &mut st).unwrap();
        assert_eq!((tg.sec, tg.anchor), (t(0), t(2)), "a reach inside the entry's block: anchor = E1");
        assert_eq!(path::target_trigger(&tape, true, 10_200, t(2), t(60), t(2), &mut st), None);
    }
    // The exit it decides is a MARKET exit by blocks from the anchor: a sale at the block's lowest LOW (100.30), not AT 101.00.
    let tape = Tape::new(&r, &ix, None).unwrap();
    let o = Order { side: Side::Sell, anchor: t(6), stop: t(60), unit: 1, limit: None, kind: Kind::Target };
    let (f, _) = run(&tape, &o, &Dial::new(&[2_500]).unwrap(), 300, None, None);
    assert_eq!(f[0].first.map(|b| (b.block.at, b.px, b.units)), Some((t(6), 10_030, 300)), "block [6, 8): 25% of 1,300 = 325 covers 300");
}

#[test]
fn c5_the_target_level_is_sample_6s_target_price_and_first_reach_equals_a_scan() {
    // 100 shares at an average 100.00 (value 2 x 10,000.00 half-paisa); +0.5% on a 5-paise tick: 100.50; a short's -0.5%: 99.50.
    assert_eq!(path::target_level(2 * 1_000_000, 100, true, 50, 5), 10_050);
    assert_eq!(path::target_level(2 * 1_000_000, 100, false, 50, 5), 9_950);
    assert_eq!(path::target_level(2 * 1_000_007, 100, true, 50, 5), 10_055, "rounded up to the next tick for a long");
    let mut g = Rng(23);
    for day in 0..30 {
        let r = random_day(&mut g, 1_500, day % 5);
        let ix = index_of(&r, 1_500);
        let mm = RangeMm::new(&r);
        let (fast, slow) = (Tape::new(&r, &ix, Some(&mm)).unwrap(), Tape::new(&r, &ix, None).unwrap());
        for _ in 0..50 {
            let a = g.below(1_400) as i64;
            let b = a + g.below(400) as i64;
            let lv = 99_000 + g.below(3_000) as i64;
            for high in [true, false] {
                let (mut s1, mut s2) = (0, 0);
                assert_eq!(fast.first_reach(t(a), t(b), high, lv, &mut s1), slow.first_reach(t(a), t(b), high, lv, &mut s2), "day {day} [{a}, {b}) {high} {lv}");
                assert!(s1 <= 13, "O(log n): {s1} reads");
            }
        }
    }
}

// =====================================================================================================================================
// CHANGE C6 (S3; MAIN 2026-11-12 23:10:48 IST): blocks exist only inside the dated session segments; a block crossing a segment end is
// clipped there and known there; the call-auction print second of an auction-opened segment is in no block; after ANY clip the walk
// continues on its own lattice (at + 2 s); a halt gap holds no block; feed holes are found and the orders spanning one flagged. Red first:
// fails on step C5 (crate v1's reading: no segments, no holes).
// =====================================================================================================================================

/// A two-segment SYNTHETIC day (the 2021-02-24 pattern): segment A opens with an auction at 0 and ends after its last second 100; tail
/// rows priced at 101 and 102 (no orders); segment B re-opens with an auction at 1,000 and runs to 2,000.
fn two_segments() -> (Vec<SecIn>, [Seg; 2]) {
    let mut v: Vec<(i64, Paisa, Paisa, u64)> = vec![(0, 10_000, 10_000, 5_000)];
    v.extend((1..=100).map(|s| (s, 10_000 + s, 10_000 + s, 100)));
    v.push((101, 15_000, 15_000, 50_000));
    v.push((102, 15_000, 15_000, 50_000));
    v.push((1_000, 12_000, 12_000, 9_000));
    v.push((1_001, 11_000, 11_000, 400));
    v.push((1_002, 11_010, 11_010, 400));
    v.push((1_003, 11_020, 11_020, 400));
    let segs = [Seg { open: t(0), last_sec: t(100), auction_open: true }, Seg { open: t(1_000), last_sec: t(2_000), auction_open: true }];
    (rows(&v), segs)
}

#[test]
fn c6_ob1_ob11_ob37_a_block_clipped_at_a_segment_end_is_known_there_and_the_lattice_keeps_its_alignment_across_the_halt() {
    let (r, segs) = two_segments();
    let ix = index_of(&r, 3_000);
    let tape = Tape::new(&r, &ix, None).unwrap().with_segments(&segs).unwrap();
    // A buy anchored at the segment's last second 100 (OB11): its first block [100, 102) is clipped at the segment end 101 (one second:
    // the tail row 101 is no print of a block), known at 101; the next block is on its own lattice at 102 (OB37: never at 101), in the
    // halt gap (no block), so the walk jumps to the lattice block holding the re-open, [1,000, 1,002): its auction second 1,000 is in no
    // block (OB2), so it holds 1,001 alone; then [1,002, 1,004).
    let o = Order { side: Side::Buy, anchor: t(100), stop: t(3_000), unit: 1, limit: None, kind: Kind::Entry };
    let dial = Dial::new(&[10_000]).unwrap();
    let mut w = [700, 0, 0, 0, 0, 0, 0, 0];
    let mut f = [LevelFill::default(); MAXL];
    let mut got = Vec::new();
    {
        let mut rec = |_l: usize, b: &BlockFill| got.push((b.block.at, b.block.to, b.units, b.px, b.block.known, b.block.n));
        walk(&tape, &o, &dial, LevelMask::all(1), &mut w, None, None, &mut f, Some(&mut rec)).unwrap();
    }
    assert_eq!(got, vec![(t(100), t(101), 100, 10_100, t(101), 1), (t(1_000), t(1_002), 400, 11_000, t(1_002), 1), (t(1_002), t(1_004), 200, 11_020, t(1_004), 2)]);
    assert_eq!((f[0].units, f[0].e1, f[0].known_at, f[0].clipped), (700, t(101), t(1_004), 1));
}

#[test]
fn c6_an_order_stops_at_the_session_end_and_never_fills_on_tail_rows() {
    // One segment [0, 100]; tail rows at 101..150 (priced, no orders) with large volume: a square-off from 98 with its stop at 5,000 fills
    // only in [98, 100] and keeps its rest at the session end (unclosable in session: the host values it).
    let mut v: Vec<(i64, Paisa, Paisa, u64)> = (0..=100).map(|s| (s, 10_000, 10_000, 40)).collect();
    v.extend((101..=150).map(|s| (s, 9_000, 9_000, 100_000)));
    let r = rows(&v);
    let ix = index_of(&r, 3_000);
    let segs = [Seg { open: t(0), last_sec: t(100), auction_open: false }];
    let tape = Tape::new(&r, &ix, None).unwrap().with_segments(&segs).unwrap();
    let o = Order { side: Side::Sell, anchor: t(98), stop: t(5_000), unit: 1, limit: None, kind: Kind::SquareOff };
    let (f, w) = run(&tape, &o, &Dial::new(&[10_000]).unwrap(), 1_000, None, None);
    assert_eq!((f[0].units, w[0], f[0].known_at, tape.session_end()), (120, 880, t(101), t(101)));
}

#[test]
fn c6_ob40_a_suspected_feed_hole_is_found_and_an_order_spanning_it_is_flagged() {
    // A dense day (every second traded) with a 60-second silence after 500: one hole (500, 561). An order from 450 to 700 spans it; one
    // from 600 does not. A sparse day (a print every 3 s: median gap 3 s) has no suspected hole.
    let v: Vec<(i64, Paisa, Paisa, u64)> = (0..1_000).filter(|s| !(501..561).contains(s)).map(|s| (s, 10_000, 10_000, 10)).collect();
    let r = rows(&v);
    let ix = index_of(&r, 1_000);
    let segs = [Seg { open: t(0), last_sec: t(999), auction_open: false }];
    let base = Tape::new(&r, &ix, None).unwrap().with_segments(&segs).unwrap();
    let holes = find_holes(&base);
    assert_eq!(holes, vec![(t(500), t(561))]);
    let tape = base.with_holes(&holes);
    let dial = Dial::new(&[1_000]).unwrap();
    let (a, _) = run(&tape, &Order { side: Side::Buy, anchor: t(450), stop: t(700), unit: 1, limit: None, kind: Kind::Entry }, &dial, 1_000_000, None, None);
    let (b, _) = run(&tape, &Order { side: Side::Buy, anchor: t(600), stop: t(700), unit: 1, limit: None, kind: Kind::Entry }, &dial, 1_000_000, None, None);
    assert_eq!((a[0].hole_span, b[0].hole_span), (true, false));
    let sparse: Vec<(i64, Paisa, Paisa, u64)> = (0..1_000).filter(|s| s % 3 == 0 && !(501..561).contains(s)).map(|s| (s, 10_000, 10_000, 10)).collect();
    let rs = rows(&sparse);
    let ixs = index_of(&rs, 1_000);
    assert!(find_holes(&Tape::new(&rs, &ixs, None).unwrap().with_segments(&segs).unwrap()).is_empty());
}

#[test]
fn c6_the_size_reference_reads_only_the_segment_holding_t() {
    let (r, segs) = two_segments();
    let ix = index_of(&r, 3_000);
    let tape = Tape::new(&r, &ix, None).unwrap().with_segments(&segs).unwrap();
    assert_eq!(limits::size_ref(&tape, t(1_000), Side::Buy), None, "T at the re-open: nothing traded since the segment opened");
    assert_eq!(limits::size_ref(&tape, t(1_002), Side::Buy), Some(11_000));
    assert_eq!(limits::size_ref(&tape, t(1_001), Side::Buy), Some(12_000), "the re-open's auction print is a traded second of the segment");
}

#[test]
fn c6_the_gap_skips_stay_exact_on_segmented_days_with_auction_seconds_and_tail_rows() {
    let mut g = Rng(29);
    let mut skips = 0_u64;
    for day in 0..40_u64 {
        let n = 3_000;
        let mut r = random_day(&mut g, n, 2 + day % 2);
        // Tail rows after the session end (no orders) and an auction print at each open.
        r.retain(|s| s.ts < t(2_000) || s.ts >= t(2_400));
        let segs = [Seg { open: t(0), last_sec: t(1_499), auction_open: true }, Seg { open: t(1_700), last_sec: t(2_399), auction_open: true }];
        let ix = index_of(&r, n);
        let plain = Tape::new(&r, &ix, None).unwrap().with_segments(&segs).unwrap();
        let dial = Dial::new(&LV).unwrap();
        let gt = GapTables::new(&plain, &dial, 1).unwrap();
        let fast = plain.with_gaps(&gt).unwrap();
        for _ in 0..25 {
            let side = if g.below(2) == 0 { Side::Buy } else { Side::Sell };
            let a = g.below(2_200) as i64;
            let stop = a + 1 + g.below(3_000 - a as u64) as i64;
            let lim = if g.below(2) == 0 { None } else { Some(99_000 + g.below(3_000) as i64) };
            let o = Order { side, anchor: t(a), stop: t(stop), unit: 1, limit: lim, kind: Kind::Entry };
            let want = 1 + g.below(4_000);
            let (f1, w1) = run(&plain, &o, &dial, want, None, None);
            let (f2, w2) = run(&fast, &o, &dial, want, None, None);
            assert_eq!(w1, w2);
            for l in 0..5 {
                assert_eq!(no_steps(&f1[l]), no_steps(&f2[l]), "day {day} level {l} {o:?}");
                skips += u64::from(f2[l].steps.skips);
                // Nothing fills on the auction seconds, in the halt gap or after the session end.
                if let Some(x) = f1[l].last {
                    assert!(x.block.at < t(2_400) && !(t(1_500)..t(1_699)).contains(&x.block.at), "{o:?}");
                }
            }
        }
    }
    assert!(skips > 20, "{skips}");
}

// =====================================================================================================================================
// CHANGE C7 (S16, S19; D18; MAIN 2026-11-12 23:10:48 IST): REQ 71's rules and path facts watched from E0 (the START of the entry's first
// filling block), each price trigger anchored at max(its end, E1), R(s) from the fills. Red first: fails on the stand-in (design v1's
// E1 window: it misses a stop touched inside the entry block, OB31, and anchors the go-positive timer at E1 + t).
// =====================================================================================================================================

/// E7's SYNTHETIC tape (design 6), offsets from 10:01:00: a long 500 filled at 200.00 in [0, 2).
fn e7() -> Vec<SecIn> {
    let mut v: Vec<(i64, Paisa, Paisa, u64)> = vec![(0, 20_000, 20_000, 3_000), (1, 20_000, 19_970, 1_000), (2, 19_990, 19_980, 500), (3, 19_980, 19_960, 500), (4, 19_970, 19_945, 500), (5, 19_960, 19_940, 1_500), (6, 19_970, 19_955, 1_500)];
    v.extend((7..59).map(|s| (s, 20_040, 19_995, 300)));
    v.push((59, 20_010, 19_990, 300));
    v.extend((60..70).map(|s| (s, 20_080, 20_030, 300)));
    v.push((70, 20_120, 20_050, 300));
    v.push((71, 20_090, 20_055, 300));
    v.extend((72..80).map(|s| (s, 20_070, 20_040, 300)));
    rows(&v)
}

fn entry_of(tape: &Tape<'_>, o: &Order, want: Units) -> (LevelFill, Vec<BlockFill>) {
    let mut w = [want, 0, 0, 0, 0, 0, 0, 0];
    let mut f = [LevelFill::default(); MAXL];
    let mut fills = Vec::new();
    {
        let mut rec = |_l: usize, b: &BlockFill| {
            if b.why == Why::Filled {
                fills.push(*b);
            }
        };
        walk(tape, o, &Dial::new(&[2_500]).unwrap(), LevelMask::all(1), &mut w, None, None, &mut f, Some(&mut rec)).unwrap();
    }
    (f[0], fills)
}

#[test]
fn c7_e7_stops_timers_and_the_trail_watched_from_e0_each_anchored_at_max_its_end_and_e1() {
    let r = e7();
    let ix = index_of(&r, 200);
    let mm = RangeMm::new(&r);
    let tape = Tape::new(&r, &ix, Some(&mm)).unwrap();
    let (f, fills) = entry_of(&tape, &Order { side: Side::Buy, anchor: t(0), stop: t(200), unit: 1, limit: None, kind: Kind::Entry }, 500);
    assert_eq!((f.units, f.value, f.e0, f.e1), (500, 500 * 20_000, t(0), t(2)));
    let e = path::EntryView { e0: f.e0, e1: f.e1, fills: &fills };
    let rules = [path::Rule::Stop { x: 50 }, path::Rule::GoPositive { secs: 60 }, path::Rule::Trail { act: 100, dist: 60 }, path::Rule::Target { x: 100 }, path::Rule::TimeExit { secs: 30 }];
    let mut st = 0;
    let (tg, fc) = path::scan_one(&tape, &e, t(80), true, &rules, &[50, 100], &mut st);
    // Stop 199.50: the LOWs from E0 200.00, 199.70, 199.80, 199.60, 199.45: trigger 10:01:04, anchor max(10:01:05, E1) = 10:01:05.
    assert_eq!(tg[0].map(|x| (x.sec, x.anchor, x.level)), Some((t(4), t(5), 19_950)));
    // Go-positive 60 s: the clock at E0 + 60 s = 10:02:00; 10:01:59's LOW 199.90 < R 200.00: not positive: the exit at 10:02:00.
    assert_eq!(tg[1].map(|x| (x.sec, x.anchor)), Some((t(60), t(60))));
    // Trail (1.00, 0.60): 10:02:10's HIGH 201.20 activates it at 200.60; its own LOW 200.50 is tested against the level BEFORE it
    // (inactive: no trigger); 10:02:11's LOW 200.55 <= 200.60 triggers: anchor 10:02:12 (OB28, adverse-first).
    assert_eq!(tg[2].map(|x| (x.sec, x.anchor, x.level)), Some((t(71), t(72), 20_060)));
    // Target +1.00 reached at 10:02:10 (HIGH 201.20 >= 201.00): anchor 10:02:11. Time exit 30 s: 10:01:30.
    assert_eq!(tg[3].map(|x| (x.sec, x.anchor)), Some((t(70), t(71))));
    assert_eq!(tg[4].map(|x| (x.sec, x.anchor)), Some((t(30), t(30))));
    // Facts: +0.50 first at 10:02:10 (200.50 needs HIGH >= 200.50: :60 has 200.80), -0.50 at :04 (LOW 199.45); best 201.20 at :70,
    // worst 199.40 at :05; run-up 500 x 1.20 = Rs 600.00, run-down 500 x 0.60 = Rs 300.00.
    assert_eq!((fc.up_at.clone(), fc.down_at.clone()), (vec![Some(t(60)), Some(t(70))], vec![Some(t(4)), None]));
    assert_eq!(fc.first, vec![path::FirstSide::Down, path::FirstSide::Up]);
    assert_eq!((fc.best, fc.worst, fc.run_up, fc.run_down), ((20_120, t(70)), (19_940, t(5)), 60_000, 30_000));
    // The stop's exit fills by blocks from its anchor: SELL [05, 07) at min(199.40, 199.55) = 199.40, 25% of 3,000 = 750 >= 500.
    let (x, _) = entry_of(&tape, &Order { side: Side::Sell, anchor: t(5), stop: t(200), unit: 1, limit: None, kind: Kind::Stop }, 500);
    assert_eq!((x.units, x.first.map(|b| b.px)), (500, Some(19_940)));
}

// =====================================================================================================================================
// G0 REPAIR 2 (OB7: explicit checked arithmetic, never overflow-checks, design 4 and S1): a price level, a tick rounding or a balance
// whose paisa arithmetic leaves i64 never wraps. Red first: on the step before (raw + / -) these PANIC under this crate's release profile
// (overflow-checks on) and would WRAP under a host profile without it (a far target becoming a negative level that triggers at once).
// =====================================================================================================================================

#[test]
fn g0r2_ob7_far_levels_saturate_unreached_and_a_balance_never_wraps() {
    let r = e7();
    let ix = index_of(&r, 200);
    let mm = RangeMm::new(&r);
    let tape = Tape::new(&r, &ix, Some(&mm)).unwrap();
    for side in [Side::Buy, Side::Sell] {
        let long = side == Side::Buy;
        let (f, fills) = entry_of(&tape, &Order { side, anchor: t(0), stop: t(200), unit: 1, limit: None, kind: Kind::Entry }, 500);
        assert!(f.units > 0);
        let e = path::EntryView { e0: f.e0, e1: f.e1, fills: &fills };
        let far = i64::MAX;
        let rules = [path::Rule::Stop { x: far }, path::Rule::Target { x: far }, path::Rule::QuickCut { x: far }, path::Rule::Trail { act: far, dist: 1 }, path::Rule::Trail { act: 1, dist: far }];
        let mut st = 0;
        let (tg, fc) = path::scan_one(&tape, &e, t(150), long, &rules, &[far], &mut st);
        assert_eq!(tg, vec![None; 5], "a level beyond i64 is never reached (side {side:?})");
        assert_eq!((fc.up_at.clone(), fc.down_at.clone(), fc.first.clone()), (vec![None], vec![None], vec![path::FirstSide::Neither]));
    }
    // A tick rounding up at the top of i64 saturates (the step before: r + tk overflowed).
    assert_eq!(limits::round_up(i64::MAX - 1, 5), i64::MAX);
    assert_eq!(limits::round_up(10_001, 5), 10_005);
    // An admission whose reserve would carry the balance past i64 refuses (the step before: free - reserve wrapped).
    let mut fc = FreeCash::new(10_000_000).unwrap();
    assert_eq!(fc.admit(i64::MIN + 1), Err(FillErr::Overflow));
    assert_eq!(fc.free(), 10_000_000);
    assert_eq!(fc.admit(4_000_000), Ok(()));
    assert_eq!(fc.free(), 6_000_000);
}

#[test]
fn c7_ob31_a_stop_touched_inside_the_entry_block_after_the_price_paid_triggers_with_anchor_e1() {
    // SYNTHETIC (OB31): block [0, 2): second 0 at 200.00 (VOL 1,500), second 1 at 199.30 (VOL 1,000): BUY 500 at the block HIGH 200.00.
    // Stop 199.50: second 1 is a second held, after the price we paid: it triggers; anchor max(10:01:02, E1 = 10:01:02). (v1, from E1:
    // missed.)
    let r = rows(&[(0, 20_000, 20_000, 1_500), (1, 19_930, 19_930, 1_000), (2, 19_990, 19_980, 500), (3, 19_995, 19_985, 500)]);
    let ix = index_of(&r, 200);
    let tape = Tape::new(&r, &ix, None).unwrap();
    let (f, fills) = entry_of(&tape, &Order { side: Side::Buy, anchor: t(0), stop: t(200), unit: 1, limit: None, kind: Kind::Entry }, 500);
    assert_eq!((f.units, f.first.map(|b| b.px)), (500, Some(20_000)));
    let e = path::EntryView { e0: f.e0, e1: f.e1, fills: &fills };
    let mut st = 0;
    let (tg, _) = path::scan_one(&tape, &e, t(100), true, &[path::Rule::Stop { x: 50 }], &[], &mut st);
    assert_eq!(tg[0].map(|x| (x.sec, x.anchor)), Some((t(1), t(2))));
}

#[test]
fn c7_ob29_a_second_touching_both_sides_is_counted_apart_and_read_adverse_first_and_levels_share_one_pass() {
    let r = rows(&[(0, 20_000, 20_000, 4_000), (1, 20_000, 20_000, 4_000), (5, 20_060, 19_940, 300), (6, 20_000, 20_000, 300)]);
    let ix = index_of(&r, 200);
    let tape = Tape::new(&r, &ix, None).unwrap();
    let (f, fills) = entry_of(&tape, &Order { side: Side::Sell, anchor: t(0), stop: t(200), unit: 1, limit: None, kind: Kind::Entry }, 500);
    let e = path::EntryView { e0: f.e0, e1: f.e1, fills: &fills };
    let mut st = 0;
    let (_, fc) = path::scan_one(&tape, &e, t(100), false, &[], &[50], &mut st);
    assert_eq!((fc.up_at.clone(), fc.down_at.clone(), fc.first.clone(), fc.both_secs), (vec![Some(t(5))], vec![Some(t(5))], vec![path::FirstSide::BothSameSecondAdverse], 1));
    // Five levels with the same entry share ONE pass (design 4.6 step 2).
    let mut ent: [Option<path::EntryView<'_>>; MAXL] = [None; MAXL];
    for x in ent.iter_mut().take(5) {
        *x = Some(e);
    }
    let mut trig: [Vec<Option<path::Trigger>>; MAXL] = Default::default();
    let mut facts: [path::Facts; MAXL] = Default::default();
    let groups = path::scan(&tape, &ent, LevelMask::all(5), &[t(100); MAXL], false, &[path::Rule::Stop { x: 50 }], &[50], &mut trig, &mut facts, &mut st);
    assert_eq!((groups, trig[4].clone(), facts[4].both_secs), (1, vec![Some(path::Trigger { sec: t(5), anchor: t(6), level: 20_050 })], 1));
}

// =====================================================================================================================================
// CHANGE C8 (S17, D13; MAIN 2026-11-12 23:10:48 IST): the shared-volume ledger. Red first: fails on the stand-in (crate v1: no ledger,
// every charge 0).
// =====================================================================================================================================

#[test]
fn c8_ob15_ob27_another_live_orders_take_is_charged_on_both_its_seconds_and_the_total_never_passes_the_levels_share() {
    // SYNTHETIC: seconds 0..3 traded 400, 600, 500, 500 at 100.00. Order A (anchor 0, 100%) takes 600 in its block [0, 2): charged on
    // seconds 0 AND 1. Order B (anchor 1, its own odd lattice, 100%): block [1, 3) traded 1,100, the largest charge on its seconds is 600
    // (second 1; second 2 holds no take of A): capacity 500; block [3, 5): 500 uncharged. B = 1,000 (without the ledger: 1,600). On the
    // block [1, 3) A's 600 + B's 500 = 1,100 = its whole volume: never above 100% (OB27).
    let r = rows(&[(0, 10_000, 10_000, 400), (1, 10_000, 10_000, 600), (2, 10_000, 10_000, 500), (3, 10_000, 10_000, 500)]);
    let ix = index_of(&r, 100);
    let tape = Tape::new(&r, &ix, None).unwrap();
    let dial = Dial::new(&[10_000]).unwrap();
    let mut led = Ledger::new();
    let (mut wa, mut fa) = ([600, 0, 0, 0, 0, 0, 0, 0], [LevelFill::default(); MAXL]);
    walk_ledger(&tape, &Order { side: Side::Buy, anchor: t(0), stop: t(100), unit: 1, limit: None, kind: Kind::SignalExit }, &dial, LevelMask::all(1), &mut wa, None, None, &mut led, &mut fa, None).unwrap();
    let (mut wb, mut fb) = ([2_000, 0, 0, 0, 0, 0, 0, 0], [LevelFill::default(); MAXL]);
    walk_ledger(&tape, &Order { side: Side::Buy, anchor: t(1), stop: t(100), unit: 1, limit: None, kind: Kind::SquareOff }, &dial, LevelMask::all(1), &mut wb, None, None, &mut led, &mut fb, None).unwrap();
    assert_eq!((fa[0].units, fb[0].units, fb[0].first.map(|b| b.units), led.charged_blocks, led.max_charge, led.live()), (600, 1_000, Some(500), 1, 600, 2));
    // A third live order (OB15): anchored at 2 (even, A's lattice), 100%: block [2, 4): charges A 0 (A's block [2, 4) took nothing), B
    // 500 (B's block [1, 3) on second 2) and B's 500 (block [3, 5) on second 3): the largest 500: capacity 1,000 - 500 = 500.
    let (mut wc, mut fc) = ([2_000, 0, 0, 0, 0, 0, 0, 0], [LevelFill::default(); MAXL]);
    walk_ledger(&tape, &Order { side: Side::Buy, anchor: t(2), stop: t(100), unit: 1, limit: None, kind: Kind::Entry }, &dial, LevelMask::all(1), &mut wc, None, None, &mut led, &mut fc, None).unwrap();
    assert_eq!(fc[0].units, 500);
}

#[test]
fn c8_one_trades_entry_and_exit_never_share_a_seconds_volume_s13() {
    // An entry cut at its exit's anchor 3 (its block [2, 4) clipped: volume second 2 alone); the exit from 3 is charged NOTHING: the
    // entry's clipped block charges only its seconds before the stop (S9, S17).
    let r = rows(&[(0, 10_000, 10_000, 400), (1, 10_000, 10_000, 400), (2, 10_000, 10_000, 400), (3, 10_000, 10_000, 400), (4, 10_000, 10_000, 400)]);
    let ix = index_of(&r, 100);
    let tape = Tape::new(&r, &ix, None).unwrap();
    let dial = Dial::new(&[10_000]).unwrap();
    let mut led = Ledger::new();
    let (mut we, mut fe) = ([5_000, 0, 0, 0, 0, 0, 0, 0], [LevelFill::default(); MAXL]);
    walk_ledger(&tape, &Order { side: Side::Buy, anchor: t(0), stop: t(3), unit: 1, limit: None, kind: Kind::Entry }, &dial, LevelMask::all(1), &mut we, None, None, &mut led, &mut fe, None).unwrap();
    let (mut wx, mut fx) = ([1_200, 0, 0, 0, 0, 0, 0, 0], [LevelFill::default(); MAXL]);
    walk_ledger(&tape, &Order { side: Side::Sell, anchor: t(3), stop: t(100), unit: 1, limit: None, kind: Kind::SignalExit }, &dial, LevelMask::all(1), &mut wx, None, None, &mut led, &mut fx, None).unwrap();
    assert_eq!((fe[0].units, fx[0].units, led.charged_blocks), (1_200, 800, 0));
}

#[test]
fn c8_ob36_a_two_phase_overlap_is_found_by_the_spans_and_re_priced_by_the_ledger_walk_never_refused() {
    let r = rows(&[(0, 10_000, 10_000, 400), (1, 10_000, 10_000, 600), (2, 10_000, 10_000, 500), (3, 10_000, 10_000, 500)]);
    let ix = index_of(&r, 100);
    let tape = Tape::new(&r, &ix, None).unwrap();
    let dial = Dial::new(&[10_000]).unwrap();
    let oa = Order { side: Side::Buy, anchor: t(0), stop: t(100), unit: 1, limit: None, kind: Kind::Entry };
    let ob = Order { anchor: t(1), ..oa };
    let pa = FillPath::record(&tape, &oa, &dial, LevelMask::all(1), &[600, 0, 0, 0, 0, 0, 0, 0], None).unwrap();
    let pb = FillPath::record(&tape, &ob, &dial, LevelMask::all(1), &[2_000, 0, 0, 0, 0, 0, 0, 0], None).unwrap();
    let ((a0, a1), (b0, b1)) = (pa.span(0), pb.span(0));
    assert!(a0 < b1 && b0 < a1, "the busy spans overlap: a re-price, not a refusal");
    let mut led = Ledger::new();
    let (mut wa, mut fa) = ([600, 0, 0, 0, 0, 0, 0, 0], [LevelFill::default(); MAXL]);
    walk_ledger(&tape, &oa, &dial, LevelMask::all(1), &mut wa, None, None, &mut led, &mut fa, None).unwrap();
    let (mut wb, mut fb) = ([2_000, 0, 0, 0, 0, 0, 0, 0], [LevelFill::default(); MAXL]);
    walk_ledger(&tape, &ob, &dial, LevelMask::all(1), &mut wb, None, None, &mut led, &mut fb, None).unwrap();
    let mut p = 0;
    assert_eq!((pb.view(0, 2_000, &mut p).unwrap().units, fb[0].units), (1_600, 1_000), "ledger-free view 1,600; re-priced 1,000");
}

#[test]
fn a0_the_law_door_carries_the_standard_the_cap_and_the_old_rule_core_and_a_green_build_is_std70_v2() {
    assert!(!RED);
    assert_eq!((law_compiled(), VERSION, BLOCK_S), ("std70", "v2", 2));
    assert_eq!(refuse_if_red(), Ok(()));
    // The old-rule core on two seconds: a buy at max(HIGH s, HIGH s') + 5 paise, the dial's share of each SECOND's volume.
    let r = rows(&[(0, 10_000, 10_000, 400), (1, 10_020, 10_020, 400), (2, 10_010, 10_010, 400)]);
    let ix = index_of(&r, 100);
    let tape = Tape::new(&r, &ix, None).unwrap();
    let o = Order { side: Side::Buy, anchor: t(0), stop: t(100), unit: 1, limit: None, kind: Kind::Entry };
    let dial = Dial::new(&[2_500]).unwrap();
    let mut w = [250, 0, 0, 0, 0, 0, 0, 0];
    let mut f = [LevelFill::default(); MAXL];
    let law = Law::Req58bCore(cmp::CmpIn::RED);
    walk_law(&law, &tape, &o, &dial, LevelMask::all(1), &mut w, None, &mut f, None).unwrap();
    assert_eq!((f[0].units, f[0].value, law.name()), (250, 100 * 10_025 + 100 * 10_025 + 50 * 10_015, "req58b-core@cf2"));
    let mut w = [250, 0, 0, 0, 0, 0, 0, 0];
    let mut f = [LevelFill::default(); MAXL];
    walk_law(&Law::Std70, &tape, &o, &dial, LevelMask::all(1), &mut w, None, &mut f, None).unwrap();
    assert_eq!((f[0].units, f[0].value), (250, 200 * 10_020 + 50 * 10_010), "the standard: block [0, 2) at 100.20, block [2, 4) at 100.10");
}

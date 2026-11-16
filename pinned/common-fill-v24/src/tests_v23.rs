//! CRATE v2.3 (spikefade v3.1, 15 Nov 2026): the carried fill entitlement (K1, MAIN-RULINGS-ENGINE-v22 item 2) against its definition, on
//! hand-built and random tapes; the margin module's own tests sit in `margin.rs`. Every case is synthetic; every figure an integer.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic, clippy::too_many_lines, clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::cast_possible_wrap, clippy::similar_names, clippy::many_single_char_names, clippy::too_many_arguments, clippy::needless_range_loop, clippy::type_complexity, clippy::cast_lossless, reason = "tests")]

use crate::gaps::GapTables;
use crate::legs::walk_pair;
use crate::tests_v2::{BASE, Rng, random_day, rows, t};
use crate::*;

const LV: [u16; 5] = [1_000, 2_500, 5_000, 7_500, 10_000];

fn dial() -> Dial {
    Dial::new(&LV).unwrap()
}

fn index(r: &[SecIn], n: i64) -> SecIndex {
    SecIndex::new(r, BASE, t(n)).unwrap()
}

fn sell(anchor: Us, stop: Us, unit: Units, limit: Option<Paisa>) -> Order {
    Order { side: Side::Sell, anchor, stop, unit, limit, kind: Kind::Entry }
}

/// Every level's filled blocks (block start, units, price) of one walk wanting `want` units at every level.
fn fills(tape: &Tape<'_>, o: &Order, want: Units) -> (Vec<Vec<(Us, Units, Paisa)>>, [LevelFill; MAXL]) {
    let d = dial();
    let mut w = [0_u64; MAXL];
    for x in w.iter_mut().take(LV.len()) {
        *x = want;
    }
    let mut out = [LevelFill::default(); MAXL];
    let mut got: Vec<Vec<(Us, Units, Paisa)>> = vec![Vec::new(); LV.len()];
    let mut rec = |l: usize, b: &BlockFill| {
        if b.why == Why::Filled {
            got[l].push((b.block.at, b.units, b.px));
        }
    };
    walk(tape, o, &d, LevelMask::all(LV.len()), &mut w, None, None, &mut out, Some(&mut rec)).unwrap();
    (got, out)
}

fn units(f: &[(Us, Units, Paisa)]) -> Units {
    f.iter().map(|x| x.1).sum()
}

/// THE SPARSE-TAPE CASE of the v2.1 checker's S-2: a lot of 75, every print 50 units: the per-block floor never fills a lot; the carried
/// entitlement fills one lot in the block whose print brings the carry to 75 or more (100 at the second print), at that block's price.
#[test]
fn v31_carry_sparse_tape_fills_whole_lot_across_blocks() {
    let r = rows(&[(0, 10_000, 10_000, 50), (10, 9_900, 9_800, 50), (20, 9_700, 9_600, 50), (30, 9_500, 9_400, 50), (40, 9_300, 9_200, 50)]);
    let ix = index(&r, 100);
    let base = Tape::new(&r, &ix, None).unwrap().with_block(2).unwrap();
    let o = sell(t(0), t(100), 75, None);
    let (per_block, _) = fills(&base, &o, 150);
    assert_eq!(units(&per_block[4]), 0, "v2.2: 50 units a block never reach a lot of 75");
    let carry = base.with_carry(true);
    assert!(carry.carry() && !base.carry());
    let (c, out) = fills(&carry, &o, 150);
    // 100%: carry 50, 100 (one lot at the 10 s block, price 98.00), 150 (75 + 75 = 150: the second lot at the 20 s block, price 96.00)
    assert_eq!(c[4], vec![(t(10), 75, 9_800), (t(20), 75, 9_600)]);
    assert_eq!(out[4].units, 150);
    // 50%: 25 a block: 75 at the 20 s block (one lot), 150 at the 40 s block would need 6 prints: 125 at 40 s -> still one lot
    assert_eq!(c[2], vec![(t(20), 75, 9_600)]);
    // 10%: 5 a block: 25 by 40 s: nothing
    assert!(c[0].is_empty());
}

/// The carried entitlement against its definition on random tapes (every style, both block lengths, every level, several lots): after
/// each block a level's filled units = min(its want, floor(sum of the block shares in whole units / lot) x lot), never more than share x
/// cumulative usable volume, and never less than the per-block (v2.2) fills up to that block.
#[test]
fn v31_carry_never_more_than_share_of_cumulative_never_less_than_per_block() {
    let mut g = Rng(0x5eed_3101);
    for case in 0..1_200 {
        let style = g.below(5);
        let n = 120 + i64::try_from(g.below(400)).unwrap();
        let r = random_day(&mut g, n, style);
        if r.is_empty() {
            continue;
        }
        let ix = SecIndex::new(&r, BASE, t(n + 10)).unwrap();
        let blk = 1 + i64::try_from(g.below(2)).unwrap();
        let base = Tape::new(&r, &ix, None).unwrap().with_block(blk).unwrap();
        let lot = [1_u64, 25, 65, 75, 250][usize::try_from(g.below(5)).unwrap()];
        let want = lot * (1 + g.below(30));
        let a0 = i64::try_from(g.below(30)).unwrap();
        let lim = if g.below(3) == 0 { Some(9_000 + i64::try_from(g.below(2_000)).unwrap()) } else { None };
        let o = sell(t(a0), t(n + 5), lot, lim);
        let mut w = [0_u64; MAXL];
        for x in w.iter_mut().take(LV.len()) {
            *x = want;
        }
        // the carry walk with every block row recorded
        let d = dial();
        let mut out = [LevelFill::default(); MAXL];
        let mut rows_c: Vec<Vec<BlockFill>> = vec![Vec::new(); LV.len()];
        let mut rec = |l: usize, b: &BlockFill| rows_c[l].push(*b);
        walk(&base.with_carry(true), &o, &d, LevelMask::all(LV.len()), &mut w.clone(), None, None, &mut out, Some(&mut rec)).unwrap();
        let (pb, _) = fills(&base, &o, want);
        for l in 0..LV.len() {
            // replay the definition over the blocks the walk read (Filled / ZeroTake rows; past-limit rows add nothing)
            let mut carried = 0_u64;
            let mut filled = 0_u64;
            let mut exact_num = 0_u128; // share x usable as exact sums of the per-block floors are <= the exact share
            for b in &rows_c[l] {
                if b.why == Why::PastLimit || b.why == Why::PastCap {
                    assert_eq!(b.units, 0, "case {case}: a block past the limit fills nothing");
                    continue;
                }
                let share_units = if b.block.cap_den == 0 { u128::from(b.block.vol) * u128::from(LV[l]) / 10_000 } else { b.block.cap_num * u128::from(LV[l]) / (b.block.cap_den * 10_000) };
                carried += u64::try_from(share_units).unwrap();
                exact_num += u128::from(b.block.vol) * u128::from(LV[l]);
                let ent = (carried / lot * lot).min(want);
                filled += b.units;
                assert_eq!(filled, ent, "case {case} level {l} block {}: filled {filled} != min(want, carried lots) {ent}", b.block.at);
                assert!(u128::from(filled) * 10_000 <= exact_num, "case {case}: never more than share x cumulative usable volume");
                // never less than the per-block fills up to this block
                let pb_upto: u64 = pb[l].iter().filter(|x| x.0 <= b.block.at).map(|x| x.1).sum();
                assert!(filled >= pb_upto, "case {case} level {l}: carry {filled} < per-block {pb_upto}");
            }
            assert_eq!(out[l].units, filled);
        }
    }
}

/// THE TWO-PHASE HOST under the carry: a recorded path's VIEW of qty units equals a fresh walk wanting qty, and its CUT at any instant t
/// equals a fresh walk stopped at t (the straddling block re-read against the entitlement carried to it), on random tapes.
#[test]
fn v31_carry_record_view_and_cut_equal_fresh_walks() {
    let mut g = Rng(0x5eed_3102);
    let d = dial();
    let mut checked = 0;
    for case in 0..600 {
        let style = g.below(5);
        let n = 120 + i64::try_from(g.below(300)).unwrap();
        let r = random_day(&mut g, n, style);
        if r.is_empty() {
            continue;
        }
        let ix = SecIndex::new(&r, BASE, t(n + 10)).unwrap();
        let blk = 1 + i64::try_from(g.below(2)).unwrap();
        let tape = Tape::new(&r, &ix, None).unwrap().with_block(blk).unwrap().with_carry(true);
        let lot = [1_u64, 25, 65, 75][usize::try_from(g.below(4)).unwrap()];
        let upto = lot * (5 + g.below(40));
        let a0 = i64::try_from(g.below(20)).unwrap();
        let o = sell(t(a0), t(n + 5), lot, None);
        let fp = FillPath::record(&tape, &o, &d, LevelMask::all(LV.len()), &[upto; MAXL], None).unwrap();
        for _ in 0..6 {
            let l = usize::try_from(g.below(5)).unwrap();
            let qty = lot * (1 + g.below(upto / lot));
            let mut st = 0_u32;
            let v = fp.view(l, qty, &mut st).unwrap();
            let mut w = [0_u64; MAXL];
            w[l] = qty;
            let mut out = [LevelFill::default(); MAXL];
            walk(&tape, &o, &d, LevelMask::one(l), &mut w, None, None, &mut out, None).unwrap();
            assert_eq!((v.units, v.value), (out[l].units, out[l].value), "case {case}: view {qty} at level {l}");
            // a cut at a random instant (on or off the lattice)
            let cut = t(a0 + i64::try_from(g.below(u64::try_from(n).unwrap())).unwrap());
            let c = fp.cut_at(&tape, l, qty, cut, &mut st).unwrap();
            let oc = Order { stop: cut, ..o };
            let mut w2 = [0_u64; MAXL];
            w2[l] = qty;
            let mut out2 = [LevelFill::default(); MAXL];
            walk(&tape, &oc, &d, LevelMask::one(l), &mut w2, None, None, &mut out2, None).unwrap();
            assert_eq!((c.units, c.value), (out2[l].units, out2[l].value), "case {case}: cut at {} level {l} qty {qty}", (cut - BASE) / US_S);
            checked += 1;
        }
    }
    assert!(checked > 2_000);
}

/// Off (the default) the view reads crate v2.2 byte for byte: the same fills as a view never told about the carry.
#[test]
fn v31_carry_off_is_crate_v22() {
    let mut g = Rng(0x5eed_3103);
    for _ in 0..300 {
        let n = 200;
        let st = g.below(5);
        let r = random_day(&mut g, n, st);
        if r.is_empty() {
            continue;
        }
        let ix = SecIndex::new(&r, BASE, t(n + 10)).unwrap();
        let tape = Tape::new(&r, &ix, None).unwrap();
        let o = sell(t(3), t(n), 25, None);
        assert_eq!(fills(&tape, &o, 500).0, fills(&tape.with_carry(false), &o, 500).0);
    }
}

/// The carry is refused by the parts that read per-block capacities (gap tables, paired legs, the shared-volume ledger), by name.
#[test]
fn v31_carry_refused_by_gaps_legs_ledger() {
    let r = rows(&[(0, 10_000, 10_000, 500), (5, 9_900, 9_800, 500)]);
    let ix = index(&r, 50);
    let tc = Tape::new(&r, &ix, None).unwrap().with_carry(true);
    let d = dial();
    assert_eq!(GapTables::new(&tc, &d, 1).err(), Some(FillErr::CarryNotServed));
    let o = sell(t(0), t(40), 1, None);
    let mut w = [100_u64; MAXL];
    let (mut a, mut b) = ([LevelFill::default(); MAXL], [LevelFill::default(); MAXL]);
    assert_eq!(walk_pair((&tc, &o), (&tc, &o), &d, LevelMask::all(5), &mut w, &mut a, &mut b, None).err(), Some(FillErr::CarryNotServed));
    let mut led = Ledger::default();
    let mut out = [LevelFill::default(); MAXL];
    assert_eq!(walk_ledger(&tc, &o, &d, LevelMask::all(5), &mut w, None, None, &mut led, &mut out, None).err(), Some(FillErr::CarryNotServed));
}

/// A block priced past the protection limit adds nothing to the carry (its prints were not tradable for this order).
#[test]
fn v31_carry_past_limit_blocks_add_nothing() {
    // a sale with a limit of Rs 95: the 0 s and 10 s prints are at 90 (past the limit), the 20 s print at 96 (within): carry 50 only
    let r = rows(&[(0, 9_000, 9_000, 50), (10, 9_000, 9_000, 50), (20, 9_600, 9_600, 50), (30, 9_600, 9_600, 50)]);
    let ix = index(&r, 60);
    let tape = Tape::new(&r, &ix, None).unwrap().with_block(2).unwrap().with_carry(true);
    let (f, _) = fills(&tape, &sell(t(0), t(60), 75, Some(9_500)), 75);
    // 100%: 50 at 20 s (no lot), 100 at 30 s: one lot at 30 s
    assert_eq!(f[4], vec![(t(30), 75, 9_600)]);
}

/// capacity_between on a carrying view = the carried entitlement of the blocks inside the span.
#[test]
fn v31_carry_capacity_between() {
    let r = rows(&[(0, 10_000, 10_000, 50), (2, 10_000, 10_000, 50), (4, 10_000, 10_000, 50)]);
    let ix = index(&r, 20);
    let base = Tape::new(&r, &ix, None).unwrap().with_block(2).unwrap();
    assert_eq!(capacity_between(&base, t(0), t(0), t(6), 10_000, 75).unwrap().0, 0, "v2.2: no block alone reaches a lot");
    assert_eq!(capacity_between(&base.with_carry(true), t(0), t(0), t(6), 10_000, 75).unwrap().0, 150, "carry: 150 units carried -> 2 lots of 75");
}

/// CHANGE K2 (EXTREME-MATRIX-SELLING X05 / X06): a stop's buy-back whose price runs away past its first limit is RE-SENT every block (the
/// limit re-priced from the last print before the block) and fills; crate v2.2 waits at the trigger's limit and never fills (the short
/// rides). Entries are not re-priced.
#[test]
fn v31_resend_buyback_chases_a_runaway_price() {
    // last print before the anchor at Rs 100 (limit 110); the price jumps to Rs 115 at 10 s and then climbs 6% a second (121.90, 129.21 ...)
    let mut v = vec![(0_i64, 10_000_i64, 10_000_i64, 1_000_u64), (10, 11_500, 11_500, 1_000)];
    let mut px = 11_500_i64;
    for s in 11..40 {
        px = px * 106 / 100;
        v.push((s, px, px, 1_000));
    }
    let r = rows(&v);
    let ix = index(&r, 100);
    let base = Tape::new(&r, &ix, None).unwrap().with_block(1).unwrap();
    let lim = crate::limits::protection(10_000, Side::Buy, 1_000, 5);
    let o = Order { side: Side::Buy, anchor: t(5), stop: t(60), unit: 50, limit: Some(lim), kind: Kind::SignalExit };
    let d = dial();
    let walk1 = |tape: &Tape<'_>| {
        let mut w = [0_u64; MAXL];
        w[4] = 50;
        let mut out = [LevelFill::default(); MAXL];
        walk(tape, &o, &d, LevelMask::one(4), &mut w, None, None, &mut out, None).unwrap();
        out[4]
    };
    let fixed = walk1(&base);
    assert_eq!(fixed.units, 0, "v2.2: the limit stays at Rs 110 and the price never comes back");
    let rs = base.with_resend(Some((1_000, 5)));
    assert_eq!(rs.resend(), Some((1_000, 5)));
    let f = walk1(&rs);
    // the 10 s block (Rs 115) is past the Rs 110 limit set from the Rs 100 print; the 11 s block (Rs 121.90) is within 10% of the 10 s print
    assert_eq!(f.units, 50);
    assert_eq!(f.first.map(|b| b.block.at), Some(t(11)), "re-sent at the next block, filled at its HIGH");
    // an ENTRY is never re-priced on the same kind of view: a sale whose price falls 15% past its limit (Rs 90) and keeps falling never fills
    let mut fv = vec![(0_i64, 10_000_i64, 10_000_i64, 1_000_u64), (10, 8_500, 8_500, 1_000)];
    let mut fp0 = 8_500_i64;
    for s in 11..40 {
        fp0 = fp0 * 94 / 100;
        fv.push((s, fp0, fp0, 1_000));
    }
    let fr = rows(&fv);
    let fix = index(&fr, 100);
    let ftape = Tape::new(&fr, &fix, None).unwrap().with_block(1).unwrap().with_resend(Some((1_000, 5)));
    let e = Order { side: Side::Sell, kind: Kind::Entry, limit: Some(crate::limits::protection(10_000, Side::Sell, 1_000, 5)), ..o };
    let mut w = [0_u64; MAXL];
    w[4] = 50;
    let mut out = [LevelFill::default(); MAXL];
    walk(&ftape, &e, &d, LevelMask::one(4), &mut w, None, None, &mut out, None).unwrap();
    assert_eq!(out[4].units, 0, "a sale is not chased down: its limit stays at Rs 90");
    // the recorded path's cut equals the fresh walk under the re-send too
    let fp = FillPath::record(&rs, &o, &d, LevelMask::all(5), &[50; MAXL], None).unwrap();
    let mut st = 0_u32;
    let c = fp.cut_at(&rs, 4, 50, t(12), &mut st).unwrap();
    let oc = Order { stop: t(12), ..o };
    let mut w2 = [0_u64; MAXL];
    w2[4] = 50;
    let mut out2 = [LevelFill::default(); MAXL];
    walk(&rs, &oc, &d, LevelMask::one(4), &mut w2, None, None, &mut out2, None).unwrap();
    assert_eq!((c.units, c.value), (out2[4].units, out2[4].value));
}

/// CHANGE K1 (matrix X24): a segment gap (a halt) drops the carried, unused entitlement: 50 units before the halt and 50 after do NOT make a
/// lot of 75 across it.
#[test]
fn v31_carry_resets_at_a_halt() {
    let r = rows(&[(0, 10_000, 10_000, 50), (100, 10_000, 10_000, 50), (102, 10_000, 10_000, 50)]);
    let ix = index(&r, 200);
    let segs = [Seg { open: t(0), last_sec: t(10), auction_open: false }, Seg { open: t(100), last_sec: t(199), auction_open: false }];
    let tape = Tape::new(&r, &ix, None).unwrap().with_segments(&segs).unwrap().with_block(1).unwrap().with_carry(true);
    let (f, _) = fills(&tape, &sell(t(0), t(190), 75, None), 75);
    // before the halt: 50 (no lot); the carry is dropped at the halt; after: 50 at 100 s, 100 at 102 s -> one lot at 102 s
    assert_eq!(f[4], vec![(t(102), 75, 10_000)]);
}

//! CHANGE FA1 (spikefade v2.1 engine, 15 Nov 2026; checker F-A of `state/spikefade-20261114/check-spikefade-diag-v2.md`): the sparse-tape
//! capacity reading. A row printed after a silence carries every unit traded since the previous print; under [`CapMode::Prorate`] only the
//! seconds of that silence inside the block (and at or after the order's anchor) count, under [`CapMode::Anchor`] only those at or after the
//! order's anchor; [`CapMode::Row`] is crate v2.1. Every figure is an integer (paisa, microseconds, units); every case is synthetic.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic, clippy::too_many_lines, clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::cast_possible_wrap, clippy::similar_names, clippy::many_single_char_names, clippy::too_many_arguments, clippy::needless_range_loop, clippy::type_complexity, clippy::cast_lossless, reason = "tests")]

use crate::gaps::GapTables;
use crate::legs::walk_pair;
use crate::tests_v2::{BASE, Rng, rows, t};
use crate::*;

const LV: [u16; 5] = [1_000, 2_500, 5_000, 7_500, 10_000];

fn dial() -> Dial {
    Dial::new(&LV).unwrap()
}

/// Each row's previous VALID print (the host's reading: the first print's previous print is `first_prev`).
fn prevs(r: &[SecIn], first_prev: Us) -> Vec<Us> {
    let mut out = Vec::with_capacity(r.len());
    let mut last: Option<Us> = None;
    for s in r {
        out.push(last.unwrap_or(first_prev));
        if s.vol > 0 && s.high > 0 && s.low > 0 && s.high >= s.low && last != Some(s.ts) {
            last = Some(s.ts);
        }
    }
    out
}

fn index(r: &[SecIn], n: i64) -> SecIndex {
    SecIndex::new(r, BASE, t(n)).unwrap()
}

/// One sale (or buy) at every level, everything wanting `want`: each level's (block start, units, price) fills.
fn fills(tape: &Tape<'_>, o: &Order, want: Units) -> Vec<Vec<(Us, Units, Paisa)>> {
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
    got
}

fn units(f: &[(Us, Units, Paisa)]) -> Units {
    f.iter().map(|x| x.1).sum()
}

fn sell(anchor: Us, stop: Us, unit: Units) -> Order {
    Order { side: Side::Sell, anchor, stop, unit, limit: None, kind: Kind::Entry }
}

/// THE GAP RULE: a print at second 30 whose previous print is at second 0 carries 3,000 units traded over the 30 seconds (0, 30].
#[test]
fn fa1_gap_row_counts_only_the_block_seconds_or_the_seconds_after_the_order() {
    // second 0: the day's first print (its previous print one second before: g = 1); second 30: 3,000 units after a 30-second silence
    let r = rows(&[(0, 10_000, 10_000, 100), (30, 9_000, 9_000, 3_000), (200, 9_500, 9_500, 50)]);
    let ix = index(&r, 300);
    let prev = prevs(&r, t(-1));
    for blk in [1_i64, 2] {
        let base = Tape::new(&r, &ix, None).unwrap().with_block(blk).unwrap();
        let row = base.with_cap(&prev, CapMode::Row).unwrap();
        let anc = base.with_cap(&prev, CapMode::Anchor).unwrap();
        let pro = base.with_cap(&prev, CapMode::Prorate).unwrap();
        // the order is sent at second 10 (inside the silence), 100% level (index 4), lot 1, wanting 5,000 units, to second 100
        let o = sell(t(10), t(100), 1);
        let (fr, fa, fp) = (fills(&row, &o, 5_000), fills(&anc, &o, 5_000), fills(&pro, &o, 5_000));
        // crate v2.1: the whole row in its second; the minimal REQ 56 reading: the 21 seconds (9, 30] after the order's send; THE FULL
        // F-A READING: only the block's own second(s): second 30 is the FIRST second of its block from anchor 10 (both lattices), n = 1
        assert_eq!(units(&fr[4]), 3_000, "row reading, block {blk} s");
        assert_eq!(units(&fa[4]), 2_100, "anchor reading: 3,000 x 21 / 30, block {blk} s");
        assert_eq!(units(&fp[4]), 100, "prorate reading: 3,000 x 1 / 30, block {blk} s");
        // the price is the block's recorded LOW in every reading (prices never change)
        assert_eq!(fp[4].first().map(|x| (x.0, x.2)), Some((t(30), 9_000)));
        assert_eq!(fa[4].first().map(|x| (x.0, x.2)), Some((t(30), 9_000)));
        // the 10% level: floor(100 x 10%) = 10 (prorate), floor(2,100 x 10%) = 210 (anchor)
        assert_eq!(units(&fp[0]), 10);
        assert_eq!(units(&fa[0]), 210);
    }
    // the print as the SECOND second of a 2-second block (anchor 11: blocks [29, 31)): n = 2 of g = 30 -> 200; anchor reading: (10, 30] = 20
    let tape2 = Tape::new(&r, &ix, None).unwrap().with_block(2).unwrap();
    let o = sell(t(11), t(100), 1);
    assert_eq!(units(&fills(&tape2.with_cap(&prev, CapMode::Prorate).unwrap(), &o, 5_000)[4]), 200, "second second of its block: 3,000 x 2 / 30");
    assert_eq!(units(&fills(&tape2.with_cap(&prev, CapMode::Anchor).unwrap(), &o, 5_000)[4]), 2_000, "anchor reading: 3,000 x 20 / 30");
    // an order sent AFTER the silence began but whose block starts at the print itself, 1-second blocks: anchor 30: n = 1 either way
    let tape1 = Tape::new(&r, &ix, None).unwrap().with_block(1).unwrap();
    let o = sell(t(30), t(100), 1);
    assert_eq!(units(&fills(&tape1.with_cap(&prev, CapMode::Prorate).unwrap(), &o, 5_000)[4]), 100);
    assert_eq!(units(&fills(&tape1.with_cap(&prev, CapMode::Anchor).unwrap(), &o, 5_000)[4]), 100);
    // the second-200 print follows the second-30 print (g = 170); an order sent at second 150: prorate n = 1, anchor n = 51 (149, 200]
    let o = sell(t(150), t(300), 1);
    assert_eq!(units(&fills(&tape1.with_cap(&prev, CapMode::Prorate).unwrap(), &o, 5_000)[4]), 0, "50 x 1 / 170 floors to 0 units");
    assert_eq!(units(&fills(&tape1.with_cap(&prev, CapMode::Anchor).unwrap(), &o, 5_000)[4]), 15, "50 x 51 / 170 = 15");
}

/// The usable volume is an exact fraction, floored ONCE at the share (never per row): 260 units over g = 3 seconds, n = 1, at 75% in lots
/// of 65: 260 / 3 x 75% = 65 exactly = 1 lot (a per-row floor, 86 x 75% = 64.5, would give 0 lots).
#[test]
fn fa1_exact_fraction_floored_once() {
    let r = rows(&[(0, 10_000, 10_000, 65), (3, 9_000, 9_000, 260)]);
    let ix = index(&r, 100);
    let prev = prevs(&r, t(-1));
    let tape = Tape::new(&r, &ix, None).unwrap().with_block(1).unwrap().with_cap(&prev, CapMode::Prorate).unwrap();
    let f = fills(&tape, &sell(t(1), t(50), 65), 650);
    assert_eq!(units(&f[3]), 65, "75%: 260/3 x 3/4 = 65 = 1 lot of 65");
    assert_eq!(units(&f[4]), 65, "100%: floor(86.67) = 86 -> 1 lot");
    assert_eq!(units(&f[2]), 0, "50%: 43.3 -> 0 lots");
    // the block reports its exact fraction and the floor of it beside the recorded volume
    let b = tape.for_order(t(1)).block(t(3), t(50)).unwrap();
    assert_eq!((b.vol, b.cap_num, b.cap_den, b.usable_floor()), (260, 260, 3, 86));
}

/// A block clipped by the order's stop takes the usable volume of its rows BEFORE the stop only; its price still comes from the full block.
#[test]
fn fa1_clipped_block_counts_rows_before_the_stop() {
    // seconds 30 (after a 30-second silence) and 31 (1 second after it)
    let r = rows(&[(0, 10_000, 10_000, 100), (30, 9_000, 9_000, 3_000), (31, 8_000, 8_000, 1_000)]);
    let ix = index(&r, 100);
    let prev = prevs(&r, t(-1));
    let tape = Tape::new(&r, &ix, None).unwrap().with_block(2).unwrap().with_cap(&prev, CapMode::Prorate).unwrap();
    // anchor 10: block [30, 32); stop at 31: only second 30's usable volume (100), the price the full block's lowest LOW (80.00)
    let f = fills(&tape, &sell(t(10), t(31), 1), 5_000);
    assert_eq!(f[4], vec![(t(30), 100, 8_000)]);
    // no stop: both rows: 100 + 1,000 (second 31's previous print is second 30: g = 1)
    let f = fills(&tape, &sell(t(10), t(100), 1), 5_000);
    assert_eq!(f[4], vec![(t(30), 1_100, 8_000)]);
}

/// FillPath's views and its cut at an earlier stop read the same capacities as a walk to that stop (the two-phase property holds under
/// every reading), the straddling block re-read for the RECORDED order's anchor.
#[test]
fn fa1_fillpath_cut_at_equals_the_walk_to_the_stop() {
    let r = rows(&[(0, 10_000, 10_000, 100), (30, 9_000, 9_000, 3_000), (31, 8_500, 8_500, 40), (61, 8_000, 8_000, 6_000), (90, 7_000, 7_000, 900)]);
    let ix = index(&r, 200);
    let prev = prevs(&r, t(-1));
    let d = dial();
    for mode in [CapMode::Row, CapMode::Anchor, CapMode::Prorate] {
        for blk in [1_i64, 2] {
            let tape = Tape::new(&r, &ix, None).unwrap().with_block(blk).unwrap().with_cap(&prev, mode).unwrap();
            // anchor 20 sits inside the 30-second silence: a cut at 31 re-reads the block [30, 32) holding the gap row, for THIS anchor
            for anc in [11_i64, 20] {
            let o = sell(t(anc), t(150), 1);
            let up = [5_000_u64; MAXL];
            let fp = FillPath::record(&tape, &o, &d, LevelMask::all(LV.len()), &up, None).unwrap();
            for cut in [12_i64, 21, 30, 31, 32, 60, 61, 62, 90, 91, 149] {
                if cut <= anc {
                    continue;
                }
                for l in 0..LV.len() {
                    let mut st = 0_u32;
                    let c = fp.cut_at(&tape, l, 5_000, t(cut), &mut st).unwrap();
                    let w = fills(&tape, &sell(t(anc), t(cut), 1), 5_000);
                    let (wu, wv): (Units, i64) = (units(&w[l]), w[l].iter().map(|x| x.1 as i64 * x.2).sum());
                    assert_eq!((c.units, c.value), (wu, wv), "{} reading, block {blk} s, cut {cut}, level {l}", mode.name());
                }
            }
            for l in 0..LV.len() {
                let mut st = 0_u32;
                let v = fp.view(l, 5_000, &mut st).unwrap();
                assert_eq!(v.units, units(&fills(&tape, &o, 5_000)[l]));
            }
            }
        }
    }
    // the anchor-20 cut at 31 under the anchor reading: 3,000 x 11 / 30 = 1,100 (not the whole 3,000)
    let tape = Tape::new(&r, &ix, None).unwrap().with_block(2).unwrap().with_cap(&prev, CapMode::Anchor).unwrap();
    let fp = FillPath::record(&tape, &sell(t(20), t(150), 1), &d, LevelMask::all(LV.len()), &[5_000_u64; MAXL], None).unwrap();
    let mut st = 0_u32;
    assert_eq!(fp.cut_at(&tape, 4, 5_000, t(31), &mut st).unwrap().units, 1_100);
}

/// Without a capacity reading, or with one on a tape where every print follows its previous one by one second, every reading is crate
/// v2.1's (randomized days, every level, both block lengths, several anchors).
#[test]
fn fa1_dense_tape_and_row_mode_equal_crate_v21() {
    let mut g = Rng(0xFA1_0001);
    for _ in 0..40 {
        let n = 400_i64;
        let mut v = Vec::new();
        let mut px = 100_000_i64;
        for s in 0..n {
            px = (px + i64::try_from(g.below(201)).unwrap() - 100).max(100);
            let lo = px - i64::try_from(g.below(50)).unwrap();
            v.push((s, px, lo.max(1), 1 + g.below(5_000)));
        }
        let r = rows(&v);
        let ix = index(&r, n + 10);
        let prev = prevs(&r, t(-1));
        for blk in [1_i64, 2] {
            let plain = Tape::new(&r, &ix, None).unwrap().with_block(blk).unwrap();
            for a in [0_i64, 1, 7, 100, 333] {
                let o = sell(t(a), t(n), 25);
                let f0 = fills(&plain, &o, 1_000_000);
                for mode in [CapMode::Row, CapMode::Anchor, CapMode::Prorate] {
                    assert_eq!(fills(&plain.with_cap(&prev, mode).unwrap(), &o, 1_000_000), f0, "{} reading on a dense tape", mode.name());
                }
            }
        }
    }
}

/// The naive oracle, written apart from the walk: per block of `blk_s` seconds on the order's lattice, the usable volume of its valid rows
/// before the stop as an exact fraction (n / g of each row), floored once at the share, whole units; price over the full block.
fn oracle(r: &[SecIn], prev: &[Us], o: &Order, blk_s: i64, bp: u16, want: Units, mode: CapMode) -> Vec<(Us, Units, Paisa)> {
    let blk = blk_s * US_S;
    let unit = o.unit.max(1);
    let valid = |s: &SecIn| s.vol > 0 && s.high > 0 && s.low > 0 && s.high >= s.low;
    let (mut rest, mut out, mut a) = (want, Vec::new(), o.anchor);
    while a < o.stop && rest > 0 {
        let to = (a + blk).min(o.stop);
        let (mut num, mut den) = (0_u128, 1_u128);
        let mut any = false;
        for (i, s) in r.iter().enumerate() {
            if s.ts < a || s.ts >= to || !valid(s) {
                continue;
            }
            any = true;
            let from = match mode {
                CapMode::Prorate => a.max(o.anchor),
                CapMode::Anchor => o.anchor,
                CapMode::Row => i64::MIN / 2,
            };
            let p = prev[i];
            let gsec = ((s.ts - p) / US_S) as u128;
            let nsec = ((s.ts - p.max(from - US_S)) / US_S) as u128;
            let (cn, cd) = if mode == CapMode::Row { (s.vol as u128, 1) } else { (s.vol as u128 * nsec, gsec) };
            num = num * cd + cn * den;
            den *= cd;
        }
        if any {
            let w = r.iter().filter(|s| s.ts >= a && s.ts < a + blk && valid(s));
            let px = w.map(|s| s.low).min().unwrap();
            let cap = (num * u128::from(bp) / (den * 10_000)) as u64 / unit * unit;
            let take = cap.min(rest);
            if take > 0 {
                out.push((a, take, px));
                rest -= take;
            }
        }
        a += blk;
    }
    out
}

/// Randomized SPARSE days (silences of 1-90 seconds, rows without volume between prints): the walk under each reading equals the oracle at
/// every level, both block lengths, anchors inside and outside silences, with and without a stop inside a block.
#[test]
fn fa1_sparse_days_equal_the_oracle() {
    let mut g = Rng(0xFA1_0002);
    let mut differ = 0_u64;
    for _ in 0..60 {
        let mut v = Vec::new();
        let mut s = 0_i64;
        let mut px = 5_000_i64;
        while s < 2_000 {
            px = (px + i64::try_from(g.below(41)).unwrap() - 20).max(5);
            let vol = if g.below(9) == 0 { 0 } else { 1 + g.below(20_000) };
            v.push((s, px, (px - i64::try_from(g.below(10)).unwrap()).max(1), vol));
            s += 1 + i64::try_from(if g.below(3) == 0 { g.below(90) } else { g.below(4) }).unwrap();
        }
        let r = rows(&v);
        let ix = index(&r, 2_100);
        let prev = prevs(&r, t(-1) - US_S * i64::try_from(g.below(30)).unwrap());
        for blk in [1_i64, 2] {
            let plain = Tape::new(&r, &ix, None).unwrap().with_block(blk).unwrap();
            for _ in 0..6 {
                let a = i64::try_from(g.below(1_500)).unwrap();
                let stop = a + 1 + i64::try_from(g.below(400)).unwrap();
                let unit = [1_u64, 15, 65, 75, 650][usize::try_from(g.below(5)).unwrap()];
                let o = sell(t(a), t(stop), unit);
                let want = unit * (1 + g.below(400));
                for mode in [CapMode::Row, CapMode::Anchor, CapMode::Prorate] {
                    let f = fills(&plain.with_cap(&prev, mode).unwrap(), &o, want);
                    for (l, &bp) in LV.iter().enumerate() {
                        assert_eq!(f[l], oracle(&r, &prev, &o, blk, bp, want, mode), "{} reading, block {blk} s, anchor {a}, stop {stop}, unit {unit}, level {l}", mode.name());
                    }
                    if mode == CapMode::Prorate && f != fills(&plain, &o, want) {
                        differ += 1;
                    }
                }
            }
        }
    }
    assert!(differ > 50, "the prorate reading must change many sparse-day fills ({differ})");
}

/// Refusals: a `prev` of another length; a previous print not before its row; gap tables, paired legs and the shared-volume ledger over a
/// tape with a capacity reading.
#[test]
fn fa1_refusals() {
    let r = rows(&[(0, 10_000, 10_000, 100), (30, 9_000, 9_000, 3_000)]);
    let ix = index(&r, 100);
    let plain = Tape::new(&r, &ix, None).unwrap();
    assert_eq!(plain.with_cap(&[t(-1)], CapMode::Prorate).err(), Some(FillErr::IndexMismatch));
    assert!(plain.with_cap(&[], CapMode::Row).is_ok(), "the row reading needs no prev");
    let bad = [t(-1), t(30)];
    let tb = plain.with_cap(&bad, CapMode::Prorate).unwrap();
    let d = dial();
    let mut w = [100_u64; MAXL];
    let mut out = [LevelFill::default(); MAXL];
    assert_eq!(walk(&tb, &sell(t(10), t(100), 1), &d, LevelMask::all(5), &mut w, None, None, &mut out, None).err(), Some(FillErr::Unsorted));
    let prev = prevs(&r, t(-1));
    let tc = plain.with_cap(&prev, CapMode::Prorate).unwrap();
    assert_eq!(GapTables::new(&tc, &d, 1).err(), Some(FillErr::CapReadNotServed));
    let o = sell(t(10), t(100), 1);
    let mut w = [100_u64; MAXL];
    let (mut a, mut b) = ([LevelFill::default(); MAXL], [LevelFill::default(); MAXL]);
    assert_eq!(walk_pair((&tc, &o), (&tc, &o), &d, LevelMask::all(5), &mut w, &mut a, &mut b, None).err(), Some(FillErr::CapReadNotServed));
    let mut led = Ledger::default();
    let mut w = [100_u64; MAXL];
    let mut out = [LevelFill::default(); MAXL];
    assert_eq!(walk_ledger(&tc, &o, &d, LevelMask::all(5), &mut w, None, None, &mut led, &mut out, None).err(), Some(FillErr::CapReadNotServed));
    // gap tables built on the plain tape are not served to a capacity view (the walk reads block by block, the same answers)
    let gt = GapTables::new(&plain, &d, 1).unwrap();
    let with_g = plain.with_gaps(&gt).unwrap().with_cap(&prev, CapMode::Prorate).unwrap();
    assert!(with_g.gaps().is_none());
    assert_eq!(fills(&with_g, &o, 5_000), fills(&tc, &o, 5_000));
}

//! CHANGE C9 (the block-length dial of REQ 73b; idx1s lane build, main bba1eb42, 2026-11-13): red-first tests. Written before the change
//! (the red stub validates the length and ignores it, so every view stays at 2 s): the one-second readings below must fail on it and pass
//! after it; the 2-second equalities (G-CHG) pass on both. Every figure is an integer (paisa, microseconds, units); synthetic cases say so.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic, clippy::too_many_lines, clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::cast_possible_wrap, clippy::similar_names, clippy::many_single_char_names, clippy::too_many_arguments, clippy::needless_range_loop, clippy::type_complexity, clippy::cast_lossless, reason = "tests")]

use crate::legs::walk_pair;
use crate::tests_v2::{Rng, index_of, random_day, rows, t};
use crate::*;

const LV: [u16; 5] = [1_000, 2_500, 5_000, 7_500, 10_000];

fn dial() -> Dial {
    Dial::new(&LV).unwrap()
}

fn order(side: Side, anchor: Us, stop: Us, unit: Units, limit: Option<Paisa>) -> Order {
    Order { side, anchor, stop, unit, limit, kind: Kind::Entry }
}

/// One order at every level: (each level's summary, every recorded row as (level, block start, block end, known, units, price, why)).
fn run(tape: &Tape<'_>, o: &Order, want: Units, money: Option<Paisa>) -> ([LevelFill; MAXL], Vec<(usize, BlockFill)>) {
    let d = dial();
    let mut w = [0_u64; MAXL];
    for x in w.iter_mut().take(LV.len()) {
        *x = want;
    }
    let m: [Option<Paisa>; MAXL] = std::array::from_fn(|l| if l < LV.len() { money } else { None });
    let mut out = [LevelFill::default(); MAXL];
    let mut got: Vec<(usize, BlockFill)> = Vec::new();
    let mut rec = |l: usize, b: &BlockFill| got.push((l, *b));
    walk(tape, o, &d, LevelMask::all(LV.len()), &mut w, money.map(|_| &m), None, &mut out, Some(&mut rec)).unwrap();
    (out, got)
}

fn filled(got: &[(usize, BlockFill)], l: usize) -> Vec<(Us, Units, Paisa)> {
    got.iter().filter(|(x, b)| *x == l && b.why == Why::Filled).map(|(_, b)| (b.block.at, b.units, b.px)).collect()
}

fn valid_row(s: &SecIn) -> bool {
    s.vol > 0 && s.high > 0 && s.low > 0 && s.high >= s.low
}

/// The naive oracle, written apart from the walk: one order at one level over blocks of `blk_s` whole seconds from the anchor to the
/// stop: the volume of the block's valid rows before the stop, the price over the FULL block (C1), nothing past the protection limit,
/// floor(share x volume) in whole units, the payable cut (the rest cancelled). Returns (fills, units, value, unpaid).
fn oracle(r: &[SecIn], o: &Order, blk_s: i64, bp: u16, want: Units, pay: Option<Paisa>) -> (Vec<(Us, Units, Paisa)>, Units, Paisa, Units) {
    let unit = o.unit.max(1);
    let blk = blk_s * US_S;
    let (mut rest, mut value, mut unpaid) = (want, 0_i64, 0_u64);
    let mut out = Vec::new();
    let mut a = o.anchor;
    while a < o.stop && rest > 0 {
        let to = (a + blk).min(o.stop);
        let vol: u64 = r.iter().filter(|s| s.ts >= a && s.ts < to && valid_row(s)).map(|s| s.vol).sum();
        if vol > 0 {
            let w = r.iter().filter(|s| s.ts >= a && s.ts < a + blk && valid_row(s));
            let px = match o.side {
                Side::Buy => w.map(|s| s.high).max().unwrap(),
                Side::Sell => w.map(|s| s.low).min().unwrap(),
            };
            let past = o.limit.is_some_and(|l| match o.side {
                Side::Buy => px > l,
                Side::Sell => px < l,
            });
            if !past {
                let cap = vol * u64::from(bp) / 10_000 / unit * unit;
                let mut take = cap.min(rest);
                let mut cut = false;
                if let Some(p) = pay
                    && take > 0
                {
                    let fit = ((p - value).max(0) / px) as u64 / unit * unit;
                    if fit < take {
                        take = fit;
                        cut = true;
                    }
                }
                if take > 0 {
                    out.push((a, take, px));
                    rest -= take;
                    value += take as i64 * px;
                }
                if cut {
                    unpaid = rest;
                    rest = 0;
                }
            }
        }
        a += blk;
    }
    (out, want - rest - unpaid, value, unpaid)
}

/// The default block of every tape is 2 s (crate v2), and the law names follow the dial.
#[test]
fn c9_default_is_two_seconds_and_law_names() {
    let r = rows(&[(5, 10_000, 9_900, 400)]);
    let ix = index_of(&r, 60);
    let tape = Tape::new(&r, &ix, None).unwrap();
    assert_eq!(tape.block_us(), 2 * US_S);
    assert_eq!(BLOCK_US, 2 * US_S, "crate v2's default block, unchanged");
    assert_eq!(law_of_block(1), "req73b-1s");
    assert_eq!(law_of_block(2), LAW_STD);
    // CRATE v2.1 (was the lane pin ("v2", "C9", "G0+C9")): the default law is crate v2's; the changes are named.
    assert_eq!((VERSION, LANE_CHANGES, STEP), ("v2", "C9+C10+C11", "G0+C9+C10+C11"), "the default law is crate v2's; the changes are named");
    assert_eq!((RELEASE, CHANGES_SINCE_V2), ("v2.1", "C9+C10+C11"));
}

/// The dial accepts whole seconds 1..=60 and refuses anything else by name.
#[test]
fn c9_with_block_refuses_outside_one_to_sixty() {
    let r = rows(&[(5, 10_000, 9_900, 400)]);
    let ix = index_of(&r, 60);
    let tape = Tape::new(&r, &ix, None).unwrap();
    for bad in [0, -1, 61, i64::MIN, i64::MAX] {
        assert_eq!(tape.with_block(bad).err(), Some(FillErr::BlockInvalid), "length {bad}");
    }
    for good in [1, 2, 3, 60] {
        assert!(tape.with_block(good).is_ok(), "length {good}");
    }
}

/// REQ 73b: at 1 s the block from T = 09:15:05 (synthetic offsets) is [T.000, T.999] ONLY: a buy at THAT second's HIGH (100.00), never
/// the next second's (101.50, which the 2-second block would take); known at T + 1 s; block number 0.
#[test]
fn c9_one_second_block_reads_t000_to_t999_only() {
    let r = rows(&[(4, 9_000, 8_000, 100_000), (5, 10_000, 9_900, 400), (6, 10_150, 9_950, 400)]);
    let ix = index_of(&r, 60);
    let two = Tape::new(&r, &ix, None).unwrap();
    let one = two.with_block(1).unwrap();
    assert_eq!(one.block_us(), US_S);
    let o = order(Side::Buy, t(5), t(60), 50, None);
    let (out, got) = run(&one, &o, 50, None);
    let f = got.iter().find(|(l, b)| *l == 4 && b.why == Why::Filled).unwrap().1;
    assert_eq!((f.block.at, f.block.to, f.block.known, f.px, f.units, f.k), (t(5), t(6), t(6), 10_000, 50, 0));
    assert_eq!(f.block.vol, 400, "the second's own volume only");
    assert_eq!(out[4].known_at, t(6));
    // The same order at 2 s reads 05-06 and pays 101.50 (crate v2, unchanged).
    let (_, g2) = run(&two, &o, 50, None);
    let f2 = g2.iter().find(|(l, b)| *l == 4 && b.why == Why::Filled).unwrap().1;
    assert_eq!((f2.block.at, f2.block.to, f2.px), (t(5), t(7), 10_150));
}

/// REQ 73b + 70a: volume alone splits second by second: floor(share x that second's volume) in whole lots, the rest moves to the next
/// second at ITS own HIGH.
#[test]
fn c9_rest_moves_to_the_next_second_at_its_own_price() {
    let r = rows(&[(5, 10_000, 9_900, 300), (6, 10_100, 9_900, 500), (7, 10_400, 10_200, 1_000), (8, 10_600, 10_500, 2_000), (9, 10_700, 10_650, 40_000)]);
    let ix = index_of(&r, 60);
    let one = Tape::new(&r, &ix, None).unwrap().with_block(1).unwrap();
    let o = order(Side::Buy, t(5), t(60), 25, None);
    let (out, got) = run(&one, &o, 1_500, None);
    // 25%: 75 of 300 -> 75 at 100.00; 125 of 500 -> 125 at 101.00; 250 of 1,000 -> 250 at 104.00; 500 of 2,000 -> 500 at 106.00; rest 550 at 107.00.
    assert_eq!(filled(&got, 1), vec![(t(5), 75, 10_000), (t(6), 125, 10_100), (t(7), 250, 10_400), (t(8), 500, 10_600), (t(9), 550, 10_700)]);
    assert_eq!(out[1].units, 1_500);
    let ks: Vec<u32> = got.iter().filter(|(l, b)| *l == 1 && b.why == Why::Filled).map(|(_, b)| b.k).collect();
    assert_eq!(ks, vec![0, 1, 2, 3, 4], "one block a second");
    // 10%: 30 of 300 = 1 lot (25), 50 of 500 = 2 lots, ...
    assert_eq!(filled(&got, 0)[0], (t(5), 25, 10_000));
    assert_eq!(filled(&got, 0)[1], (t(6), 50, 10_100));
}

/// A second with no trade fills nothing; the walk jumps (O(1)) to the next traded second, on the 1-second lattice (block numbers count
/// the seconds from the anchor).
#[test]
fn c9_no_trade_seconds_jumped_on_the_one_second_lattice() {
    let r = rows(&[(5, 10_000, 9_900, 40), (9, 10_300, 10_000, 4_000)]);
    let ix = index_of(&r, 60);
    let one = Tape::new(&r, &ix, None).unwrap().with_block(1).unwrap();
    let o = order(Side::Buy, t(5), t(60), 50, None);
    let (out, got) = run(&one, &o, 100, None);
    assert_eq!(filled(&got, 4), vec![(t(9), 100, 10_300)], "05 has 40 units: no whole lot; 06-08 no trade; 09 fills");
    let b = got.iter().find(|(l, b)| *l == 4 && b.why == Why::Filled).unwrap().1;
    assert_eq!(b.k, 4);
    assert_eq!(out[4].no_trade, 3, "06, 07, 08 jumped");
    assert_eq!(out[4].zero_take_blocks, 1);
}

/// The walk equals the naive oracle at 1, 2 and 3 seconds on random days of every style, both sides, with and without protection limits
/// and payable cuts, stops on and off the lattice, every level.
#[test]
fn c9_walk_equals_the_oracle_at_one_two_three_seconds() {
    let mut g = Rng(0x00c9_b10c_1a2b_3c4d);
    let mut cases = 0;
    for case in 0..240 {
        let style = case % 5;
        let n = 300 + g.below(600) as i64;
        let r = random_day(&mut g, n, style);
        if r.is_empty() {
            continue;
        }
        let ix = index_of(&r, n + 5);
        let base = Tape::new(&r, &ix, None).unwrap();
        for blk in [1_i64, 2, 3] {
            let tape = base.with_block(blk).unwrap();
            let side = if g.below(2) == 0 { Side::Buy } else { Side::Sell };
            let anchor = t(g.below(60) as i64);
            let stop = anchor + (1 + g.below(n as u64) as i64) * US_S + if g.below(3) == 0 { g.below(999_999) as i64 } else { 0 };
            let unit = [1_u64, 15, 25, 50, 75][g.below(5) as usize];
            let want = unit * (1 + g.below(60));
            let limit = if g.below(2) == 0 { None } else { Some(99_000 + g.below(4_000) as i64) };
            let pay = if g.below(3) == 0 { Some(want as i64 * (90_000 + g.below(30_000) as i64)) } else { None };
            let o = order(side, anchor, stop, unit, limit);
            let (out, got) = run(&tape, &o, want, pay);
            for (l, &bp) in LV.iter().enumerate() {
                let (want_f, units, value, unpaid) = oracle(&r, &o, blk, bp, want, pay);
                assert_eq!(filled(&got, l), want_f, "case {case} blk {blk} level {l}");
                assert_eq!((out[l].units, out[l].value, out[l].unpaid), (units, value, unpaid), "case {case} blk {blk} level {l}");
                for (lx, b) in &got {
                    if *lx == l && b.why == Why::Filled {
                        assert_eq!(b.block.known, b.block.at + blk * US_S, "known at the full block's end");
                        assert_eq!((b.block.at - anchor) % (blk * US_S), 0, "on the lattice");
                        assert_eq!(u64::from(b.k), u64::try_from((b.block.at - anchor) / (blk * US_S)).unwrap());
                    }
                }
            }
            cases += 1;
        }
    }
    assert!(cases > 600, "{cases} cases walked");
}

/// G-CHG (C9 changes nothing at 2 s): a tape set to 2 s gives exactly the default tape's answers (every summary field and every row,
/// with and without the gap tables, where the gap skips run).
#[test]
fn c9_two_seconds_set_equals_the_default_tape_g_chg() {
    let mut g = Rng(0x0002_5eec_0d00_0c9e);
    for case in 0..120 {
        let n = 900 + g.below(900) as i64;
        let r = random_day(&mut g, n, case % 5);
        if r.is_empty() {
            continue;
        }
        let ix = index_of(&r, n + 5);
        let d = dial();
        let unit = [1_u64, 25, 50][g.below(3) as usize];
        let plain = Tape::new(&r, &ix, None).unwrap();
        let gt = GapTables::new(&plain, &d, unit).unwrap();
        let side = if g.below(2) == 0 { Side::Buy } else { Side::Sell };
        let anchor = t(g.below(30) as i64);
        let o = order(side, anchor, t(n), unit, if g.below(2) == 0 { None } else { Some(100_000) });
        let want = unit * (1 + g.below(400));
        for tables in [false, true] {
            let a = if tables { plain.with_gaps(&gt).unwrap() } else { plain };
            let b = a.with_block(2).unwrap();
            let (oa, ga) = run(&a, &o, want, None);
            let (ob, gb) = run(&b, &o, want, None);
            assert_eq!(format!("{oa:?}"), format!("{ob:?}"), "case {case} tables {tables}");
            assert_eq!(ga, gb, "case {case} tables {tables}");
            // Without a recorder the gap skips run (the tables served at 2 s): the same fills.
            let mut wa = [0_u64; MAXL];
            let mut wb = [0_u64; MAXL];
            for l in 0..LV.len() {
                wa[l] = want;
                wb[l] = want;
            }
            let (mut xa, mut xb) = ([LevelFill::default(); MAXL], [LevelFill::default(); MAXL]);
            walk(&a, &o, &d, LevelMask::all(LV.len()), &mut wa, None, None, &mut xa, None).unwrap();
            walk(&b, &o, &d, LevelMask::all(LV.len()), &mut wb, None, None, &mut xb, None).unwrap();
            assert_eq!(format!("{xa:?}"), format!("{xb:?}"), "case {case} tables {tables}, no recorder");
        }
    }
}

/// The session segments at 1 s: no block inside a halt; the lattice jumps to the next segment's open on the 1-second lattice; a block is
/// known at its own end inside the segment.
#[test]
fn c9_segments_halt_jump_at_one_second() {
    let r = rows(&[(5, 10_000, 9_900, 4), (100, 10_000, 9_900, 400), (301, 10_500, 10_400, 400)]);
    let ix = index_of(&r, 400);
    let segs = [Seg { open: t(0), last_sec: t(99), auction_open: false }, Seg { open: t(300), last_sec: t(399), auction_open: false }];
    let one = Tape::new(&r, &ix, None).unwrap().with_segments(&segs).unwrap().with_block(1).unwrap();
    let o = order(Side::Buy, t(5), t(400), 25, None);
    let (out, got) = run(&one, &o, 50, None);
    assert_eq!(filled(&got, 4), vec![(t(301), 50, 10_500)], "the 100 s print lies in the halt gap; 301 fills");
    let b = got.iter().find(|(l, b)| *l == 4 && b.why == Why::Filled).unwrap().1;
    assert_eq!((b.block.to, b.block.known), (t(302), t(302)));
    assert!(out[4].steps.jumps >= 1);
}

/// The gap tables are built on the 2-second lattice: at 1 s they are not served (no gap skip), and the answers equal the walk without
/// them.
#[test]
fn c9_gap_tables_not_served_at_one_second() {
    let mut g = Rng(0x0001_0005_ca9e_0c9e);
    let r = random_day(&mut g, 3_000, 2);
    let ix = index_of(&r, 3_005);
    let d = dial();
    let plain = Tape::new(&r, &ix, None).unwrap();
    let gt = GapTables::new(&plain, &d, 50).unwrap();
    let with = plain.with_gaps(&gt).unwrap().with_block(1).unwrap();
    assert!(with.gaps().is_none(), "not served at 1 s");
    let without = plain.with_block(1).unwrap();
    let o = order(Side::Buy, t(0), t(3_000), 50, None);
    let mut wa = [0_u64; MAXL];
    let mut wb = [0_u64; MAXL];
    for l in 0..LV.len() {
        wa[l] = 50_000;
        wb[l] = 50_000;
    }
    let (mut xa, mut xb) = ([LevelFill::default(); MAXL], [LevelFill::default(); MAXL]);
    walk(&with, &o, &d, LevelMask::all(LV.len()), &mut wa, None, None, &mut xa, None).unwrap();
    walk(&without, &o, &d, LevelMask::all(LV.len()), &mut wb, None, None, &mut xb, None).unwrap();
    assert_eq!(format!("{xa:?}"), format!("{xb:?}"));
    assert!(xa.iter().all(|f| f.steps.skips == 0));
    assert!(plain.with_gaps(&gt).unwrap().gaps().is_some(), "served at 2 s");
}

/// CRATE v2.1: C10 SUPERSEDES this C9 test's refusals (the lane build refused the paired legs and the shared-volume ledger at any length
/// but 2 s, `FillErr::BlockNotServed`); since C10 both serve the 1-second lattice (their readings: tests_c10.rs), and at 2 s both still
/// serve. The lane's two refusal assertions are now the two acceptances below.
#[test]
fn c9_pair_and_ledger_at_other_lengths_served_since_c10() {
    let r = rows(&[(5, 10_000, 9_900, 4_000), (6, 10_100, 9_900, 4_000)]);
    let ix = index_of(&r, 60);
    let two = Tape::new(&r, &ix, None).unwrap();
    let one = two.with_block(1).unwrap();
    let d = dial();
    let o = order(Side::Buy, t(5), t(60), 50, None);
    let mut w = [0_u64; MAXL];
    w[0] = 100;
    let (mut a, mut b) = ([LevelFill::default(); MAXL], [LevelFill::default(); MAXL]);
    assert!(walk_pair((&one, &o), (&one, &o), &d, LevelMask::one(0), &mut w.clone(), &mut a, &mut b, None).is_ok());
    let mut led = Ledger::new();
    let mut out = [LevelFill::default(); MAXL];
    assert!(walk_ledger(&one, &o, &d, LevelMask::one(0), &mut w.clone(), None, None, &mut led, &mut out, None).is_ok());
    // At 2 s both still serve.
    assert!(walk_pair((&two, &o), (&two, &o), &d, LevelMask::one(0), &mut w.clone(), &mut a, &mut b, None).is_ok());
    assert!(walk_ledger(&two, &o, &d, LevelMask::one(0), &mut w, None, None, &mut led, &mut out, None).is_ok());
}

/// The rest of the walk family follows the dial: the first filling block and the capacity over a span are read on the 1-second lattice.
#[test]
fn c9_first_fill_and_capacity_follow_the_block() {
    let r = rows(&[(5, 10_000, 9_900, 400), (6, 10_150, 9_950, 800), (7, 10_200, 10_000, 1_200)]);
    let ix = index_of(&r, 60);
    let one = Tape::new(&r, &ix, None).unwrap().with_block(1).unwrap();
    let d = dial();
    let o = order(Side::Buy, t(5), t(60), 50, None);
    let mut w = [0_u64; MAXL];
    w[4] = 1_000;
    let ff = first_fill(&one, &o, &d, LevelMask::one(4), &w).unwrap();
    let b = ff[4].unwrap();
    assert_eq!((b.block.at, b.block.to, b.px, b.units), (t(5), t(6), 10_000, 400));
    // 100% capacity over [05, 08) in whole lots of 50: 400 + 800 + 1,200 (three 1-second blocks); at 2 s only the block 05-06 fits.
    assert_eq!(capacity_between(&one, t(5), t(5), t(8), 10_000, 50).unwrap().0, 2_400);
    let two = Tape::new(&r, &ix, None).unwrap();
    assert_eq!(capacity_between(&two, t(5), t(5), t(8), 10_000, 50).unwrap().0, 1_200);
}

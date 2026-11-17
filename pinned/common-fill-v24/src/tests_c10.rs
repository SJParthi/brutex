//! CHANGE C10 (crate v2.1; SNIPER-IDXOPT-PLAN v2 s9.2 N1, MAIN Q10 signed): the gap tables, the paired legs and the shared-volume ledger
//! on EVERY block length (1 s for index options, REQ 73b), so the one-second walk keeps its O(1) gap skips (O(1) at p99 on a sparse
//! contract) and EVERY-mode re-entries (the S17 ledger) can run. Red-first: written before the change, against a stub that keeps C9's
//! behaviour (tables on the 2-second lattice only, the pair and the ledger refusing another length): the one-second readings below fail
//! on it and pass after it; the 2-second equalities (G-CHG) pass on both. Every figure is an integer; synthetic cases say so.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic, clippy::too_many_lines, clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::cast_possible_wrap, clippy::similar_names, clippy::many_single_char_names, clippy::too_many_arguments, clippy::needless_range_loop, clippy::type_complexity, clippy::cast_lossless, reason = "tests")]

use crate::legs::{lattice_at_or_after, lattice_at_or_after_blk, walk_pair};
use crate::tests_v2::{Rng, index_of, no_steps, random_day, rows, t};
use crate::*;

const LV: [u16; 5] = [1_000, 2_500, 5_000, 7_500, 10_000];

fn dial() -> Dial {
    Dial::new(&LV).unwrap()
}

fn order(side: Side, anchor: Us, stop: Us, unit: Units, limit: Option<Paisa>) -> Order {
    Order { side, anchor, stop, unit, limit, kind: Kind::Entry }
}

/// One walk with NO recorder (so the gap skips may run), every level of `d` wanting `want`.
fn run_fast(tape: &Tape<'_>, o: &Order, d: &Dial, want: Units) -> ([LevelFill; MAXL], [Units; MAXL]) {
    let mut w = [0_u64; MAXL];
    for x in w.iter_mut().take(d.len()) {
        *x = want;
    }
    let mut f = [LevelFill::default(); MAXL];
    walk(tape, o, d, LevelMask::all(d.len()), &mut w, None, None, &mut f, None).unwrap();
    (f, w)
}

/// The gap tables follow the tape's block length (1, 2, 3, 60 s): one lattice residue per second of the block, served only to a tape of
/// the same length; tables of another length are refused by name at `with_gaps` and never served after `with_block` changes the length.
#[test]
fn c10_gap_tables_follow_the_tape_block_length() {
    let mut g = Rng(0x0c10_0001);
    let r = random_day(&mut g, 3_000, 1);
    let ix = index_of(&r, 3_005);
    let d = dial();
    let plain = Tape::new(&r, &ix, None).unwrap();
    for blk in [1_i64, 2, 3, 60] {
        let tape = plain.with_block(blk).unwrap();
        let gt = GapTables::new(&tape, &d, 50).unwrap();
        assert_eq!(gt.block_us(), blk * US_S, "blk {blk}");
        assert_eq!(gt.tables(), blk as usize * LV.len(), "one residue a second of the block, every level: blk {blk}");
        let with = tape.with_gaps(&gt).unwrap();
        assert!(with.gaps().is_some(), "served at its own length {blk}");
    }
    let g1 = GapTables::new(&plain.with_block(1).unwrap(), &d, 50).unwrap();
    let g2 = GapTables::new(&plain, &d, 50).unwrap();
    assert_eq!(plain.with_gaps(&g1).err(), Some(FillErr::BlockNotServed), "1-s tables on a 2-s tape");
    assert_eq!(plain.with_block(1).unwrap().with_gaps(&g2).err(), Some(FillErr::BlockNotServed), "2-s tables on a 1-s tape");
    assert!(plain.with_block(1).unwrap().with_gaps(&g1).unwrap().with_block(2).unwrap().gaps().is_none(), "the length changed after: not served");
    // G-CHG: at 2 s the tables of a tape set to 2 s are the default tape's, field by field.
    assert_eq!(GapTables::new(&plain.with_block(2).unwrap(), &d, 50).unwrap(), g2);
}

/// The one-second (and three-second) gap skips change no fill and no printed counter, only the steps, on random days of every style
/// (the A0 equality of crate v2, at 1 s and 3 s), and they do run.
#[test]
fn c10_gap_skips_at_one_and_three_seconds_give_the_same_fills_and_counters() {
    let mut g = Rng(0x0c10_0002);
    let d = dial();
    for blk in [1_i64, 3] {
        let mut skips = 0_u64;
        let (mut steps_plain, mut steps_fast) = (0_u64, 0_u64);
        let mut cases = 0;
        for day in 0..40_u64 {
            let n = 2_000 + g.below(3_000) as i64;
            let r = random_day(&mut g, n, day % 5);
            let ix = index_of(&r, n);
            let plain = Tape::new(&r, &ix, None).unwrap().with_block(blk).unwrap();
            for &unit in &[1_u64, 25] {
                let gt = GapTables::new(&plain, &d, unit).unwrap();
                let fast = plain.with_gaps(&gt).unwrap();
                for _ in 0..15 {
                    let side = if g.below(2) == 0 { Side::Buy } else { Side::Sell };
                    let a = g.below(n as u64 / 2) as i64;
                    let stop = a + 1 + g.below((n - a) as u64) as i64;
                    let lim = match g.below(3) {
                        0 => None,
                        1 => Some(100_000 + g.below(1_500) as i64),
                        _ => Some(99_000 + g.below(3_000) as i64),
                    };
                    let o = order(side, t(a), t(stop), unit, lim);
                    let want = (1 + g.below(4_000)) / unit * unit + unit;
                    let (f1, w1) = run_fast(&plain, &o, &d, want);
                    let (f2, w2) = run_fast(&fast, &o, &d, want);
                    assert_eq!(w1, w2, "rests, blk {blk} day {day} order {o:?}");
                    for l in 0..LV.len() {
                        assert_eq!(no_steps(&f1[l]), no_steps(&f2[l]), "blk {blk} day {day} level {l} order {o:?}");
                        skips += u64::from(f2[l].steps.skips);
                        steps_plain += u64::from(f1[l].steps.blocks);
                        steps_fast += u64::from(f2[l].steps.blocks);
                    }
                    cases += 1;
                }
            }
        }
        assert!(cases >= 1_200, "{cases} orders at {blk} s");
        assert!(skips > 100, "the {blk}-s skips ran: {skips}");
        assert!(steps_fast < steps_plain, "fewer blocks read with the {blk}-s tables: {steps_fast} vs {steps_plain}");
    }
}

/// O(1) at p99 on a sparse contract at 1 s (the reason for C10): a thin option day (5 units a second, a liquid second every 600; 22,500 s),
/// a one-lot (75) order at 10% through 599 zero-capacity one-second blocks: with the 1-s tables every order reads at most 40 steps
/// (blocks + table reads + skips; p99 and max asserted, the same at every start); without them it reads hundreds of blocks.
#[test]
fn c10_one_second_walk_on_a_sparse_contract_is_o1_at_p99() {
    let day = 22_500_i64;
    let thin: Vec<SecIn> = (0..day).map(|s| SecIn { ts: t(s), high: 5_000, low: 5_000, vol: if s % 600 == 599 { 4_000 } else { 5 } }).collect();
    let ix = index_of(&thin, day);
    let d10 = Dial::new(&[1_000]).unwrap();
    let one = Tape::new(&thin, &ix, None).unwrap().with_block(1).unwrap();
    let gt = GapTables::new(&one, &d10, 75).unwrap();
    let fast = one.with_gaps(&gt).unwrap();
    let mut steps: Vec<u64> = Vec::new();
    let mut slow: Vec<u64> = Vec::new();
    for k in 0..200_i64 {
        let a = (k % 36) * 600 + (k * 7) % 599;
        let o = order(Side::Buy, t(a), t(a + 1_200), 75, None);
        let (f, _) = run_fast(&fast, &o, &d10, 75);
        assert_eq!(f[0].units, 75, "start {a}");
        let dist = (599 - a % 600).rem_euclid(600);
        if dist > 16 {
            assert!(f[0].steps.skips >= 1, "start {a}: {dist} zero-capacity blocks ahead, a skip ran");
        }
        steps.push(u64::from(f[0].steps.blocks + f[0].steps.table_reads + f[0].steps.skips));
        let (fs, _) = run_fast(&one, &o, &d10, 75);
        assert_eq!(no_steps(&fs[0]), no_steps(&f[0]), "start {a}: the same fill without the tables");
        slow.push(u64::from(fs[0].steps.blocks));
    }
    steps.sort_unstable();
    slow.sort_unstable();
    let p99 = steps[steps.len() * 99 / 100];
    let mx = *steps.last().unwrap();
    assert!(p99 <= 40 && mx <= 40, "1-s walk with tables: p99 {p99}, max {mx} steps (bound 40)");
    assert!(slow[slow.len() / 2] > 100, "without the tables the median walk reads {} blocks", slow[slow.len() / 2]);
}

/// capacity_between at 1 s and 3 s through the tables' prefix sums equals the block-by-block sum, in at most 2 steps inside the span.
#[test]
fn c10_capacity_between_by_prefix_sums_at_one_and_three_seconds() {
    let mut g = Rng(0x0c10_0004);
    let r = random_day(&mut g, 4_000, 0);
    let ix = index_of(&r, 4_000);
    let d = dial();
    for blk in [1_i64, 3] {
        let plain = Tape::new(&r, &ix, None).unwrap().with_block(blk).unwrap();
        let gt = GapTables::new(&plain, &d, 25).unwrap();
        let fast = plain.with_gaps(&gt).unwrap();
        let mut served = 0;
        for _ in 0..300 {
            let a = g.below(1_000) as i64;
            let from = a + g.below(1_500) as i64;
            let to = (from + g.below(2_400) as i64).min(3_990);
            let bp = LV[g.below(5) as usize];
            let (c1, s1) = capacity_between(&plain, t(a), t(from), t(to), bp, 25).unwrap();
            let (c2, s2) = capacity_between(&fast, t(a), t(from), t(to), bp, 25).unwrap();
            assert_eq!(c1, c2, "blk {blk} a {a} [{from}, {to}) bp {bp}");
            assert!(s2 <= 2, "blk {blk}: {s2} steps inside the span");
            if s1 > 2 {
                served += 1;
            }
        }
        assert!(served > 100, "blk {blk}: {served} spans needed more than 2 block reads without the tables");
    }
}

/// The paired legs on the 1-second lattice (synthetic): each leg at ITS second's worst, the follow leg never above the lead's cumulative
/// units at the end of the same second; the same pair at 2 s reads the two seconds together. Legs on different block lengths: refused.
#[test]
fn c10_pair_walks_on_the_one_second_lattice() {
    let ra = rows(&[(5, 10_000, 9_900, 400), (6, 10_100, 9_950, 400)]);
    let rb = rows(&[(5, 5_050, 5_000, 100), (6, 5_150, 5_100, 1_000)]);
    let (ixa, ixb) = (index_of(&ra, 60), index_of(&rb, 60));
    let (ta2, tb2) = (Tape::new(&ra, &ixa, None).unwrap(), Tape::new(&rb, &ixb, None).unwrap());
    let (ta1, tb1) = (ta2.with_block(1).unwrap(), tb2.with_block(1).unwrap());
    let d = dial();
    let lead = order(Side::Buy, t(5), t(60), 50, None);
    let follow = order(Side::Sell, t(5), t(60), 50, None);
    let mut got: Vec<(usize, u8, BlockFill)> = Vec::new();
    let mut w = [0_u64; MAXL];
    w[4] = 300;
    let (mut a, mut b) = ([LevelFill::default(); MAXL], [LevelFill::default(); MAXL]);
    {
        let mut rec = |l: usize, leg: u8, f: &BlockFill| got.push((l, leg, *f));
        walk_pair((&ta1, &lead), (&tb1, &follow), &d, LevelMask::one(4), &mut w, &mut a, &mut b, Some(&mut rec)).unwrap();
    }
    let fills: Vec<(u8, Us, Us, Units, Paisa, u32)> = got.iter().filter(|x| x.2.why == Why::Filled).map(|x| (x.1, x.2.block.at, x.2.block.to, x.2.units, x.2.px, x.2.k)).collect();
    // Second 05: the lead takes 300 at 100.00; the follow 100 (all 100% of 100) at 50.00. Second 06: the follow 200 at 51.00 (bound 300 - 100).
    assert_eq!(fills, vec![(0, t(5), t(6), 300, 10_000, 0), (1, t(5), t(6), 100, 5_000, 0), (1, t(6), t(7), 200, 5_100, 1)]);
    assert_eq!((a[4].units, b[4].units, w[4]), (300, 300, 0));
    assert_eq!((a[4].known_at, b[4].known_at), (t(6), t(7)));
    // At 2 s: one block 05-06: the lead at the HIGH of both seconds (101.00), the follow at the LOW of both (50.00).
    let mut w2 = [0_u64; MAXL];
    w2[4] = 300;
    let (mut a2, mut b2) = ([LevelFill::default(); MAXL], [LevelFill::default(); MAXL]);
    walk_pair((&ta2, &lead), (&tb2, &follow), &d, LevelMask::one(4), &mut w2, &mut a2, &mut b2, None).unwrap();
    assert_eq!((a2[4].value, b2[4].value), (300 * 10_100, 300 * 5_000));
    // Legs on two block lengths share no lattice: refused by name.
    let mut w3 = [0_u64; MAXL];
    w3[4] = 300;
    assert_eq!(walk_pair((&ta1, &lead), (&tb2, &follow), &d, LevelMask::one(4), &mut w3, &mut a2, &mut b2, None).err(), Some(FillErr::PairMismatch));
    assert_eq!(walk_pair((&ta2, &lead), (&tb1, &follow), &d, LevelMask::one(4), &mut w3, &mut a2, &mut b2, None).err(), Some(FillErr::PairMismatch));
}

/// The lead leg of a pair is the walk's own take (no limit, no money, no session table): at 1, 2 and 3 s on random days, the lead leg's
/// units and value equal [`walk`]'s at every level, and the follow leg never ends a block above the lead.
#[test]
fn c10_pair_lead_leg_equals_the_walk_at_every_block_length() {
    let mut g = Rng(0x0c10_0006);
    let d = dial();
    for case in 0..60 {
        let n = 400 + g.below(800) as i64;
        let ra = random_day(&mut g, n, case % 5);
        let rb = random_day(&mut g, n, (case + 2) % 5);
        if ra.is_empty() || rb.is_empty() {
            continue;
        }
        let (ixa, ixb) = (index_of(&ra, n + 5), index_of(&rb, n + 5));
        for blk in [1_i64, 2, 3] {
            let ta = Tape::new(&ra, &ixa, None).unwrap().with_block(blk).unwrap();
            let tb = Tape::new(&rb, &ixb, None).unwrap().with_block(blk).unwrap();
            let unit = [1_u64, 25, 50][g.below(3) as usize];
            let anchor = t(g.below(40) as i64);
            let stop = anchor + (1 + g.below(n as u64) as i64) * US_S;
            let lead = order(Side::Buy, anchor, stop, unit, None);
            let follow = order(Side::Sell, anchor, stop, unit, None);
            let want = unit * (1 + g.below(200));
            let mut w = [0_u64; MAXL];
            for l in 0..LV.len() {
                w[l] = want;
            }
            let (mut a, mut b) = ([LevelFill::default(); MAXL], [LevelFill::default(); MAXL]);
            let mut ends: Vec<(usize, u8, u32, Units)> = Vec::new();
            {
                let mut rec = |l: usize, leg: u8, f: &BlockFill| ends.push((l, leg, f.k, f.cum));
                walk_pair((&ta, &lead), (&tb, &follow), &d, LevelMask::all(LV.len()), &mut w, &mut a, &mut b, Some(&mut rec)).unwrap();
            }
            let (fw, _) = run_fast(&ta, &lead, &d, want);
            for l in 0..LV.len() {
                assert_eq!((a[l].units, a[l].value), (fw[l].units, fw[l].value), "case {case} blk {blk} level {l}");
                assert!(b[l].units <= a[l].units, "case {case} blk {blk} level {l}: follow above lead");
            }
            // Per block: the follow's cumulative units at a block's end never above the lead's cumulative units by then.
            for &(l, leg, k, cum) in ends.iter().filter(|x| x.1 == 1) {
                let lead_cum = ends.iter().filter(|x| x.0 == l && x.1 == 0 && x.2 <= k).map(|x| x.3).max().unwrap_or(0);
                assert!(cum <= lead_cum, "case {case} blk {blk} level {l} leg {leg} block {k}: follow {cum} > lead {lead_cum}");
            }
        }
    }
}

/// The shared-volume ledger on the 1-second lattice (synthetic; 25%: 250 of each second's 1,000): order A takes 250 in second 05 and
/// 50 in 06; order B's capacity in each second is lowered by A's take in THAT second (05: 0; 06: 200; 07: 250). At 2 s the ledger reads
/// A's take of block 05-06 on both seconds (C8's reading, unchanged).
#[test]
fn c10_ledger_charges_each_second_on_the_one_second_lattice() {
    let r = rows(&[(5, 10_000, 9_900, 1_000), (6, 10_100, 9_950, 1_000), (7, 10_200, 10_000, 1_000), (8, 10_250, 10_050, 1_000)]);
    let ix = index_of(&r, 60);
    let two = Tape::new(&r, &ix, None).unwrap();
    let one = two.with_block(1).unwrap();
    let d = dial();
    let lv = LevelMask::one(1);
    let fills = |tape: &Tape<'_>, led: &mut Ledger, want: Units| -> (u32, Vec<(Us, Units)>, LevelFill) {
        let mut w = [0_u64; MAXL];
        w[1] = want;
        let mut out = [LevelFill::default(); MAXL];
        let mut got: Vec<(Us, Units)> = Vec::new();
        let id = {
            let mut rec = |_l: usize, b: &BlockFill| {
                if b.why == Why::Filled {
                    got.push((b.block.at, b.units));
                }
            };
            walk_ledger(tape, &order(Side::Buy, t(5), t(60), 1, None), &d, lv, &mut w, None, None, led, &mut out, Some(&mut rec)).unwrap()
        };
        (id, got, out[1])
    };
    let mut led = Ledger::new();
    let (_, ga, _) = fills(&one, &mut led, 300);
    assert_eq!(ga, vec![(t(5), 250), (t(6), 50)]);
    let (_, gb, fb) = fills(&one, &mut led, 400);
    assert_eq!(gb, vec![(t(6), 200), (t(7), 200)], "05 fully charged; 06 250 - 50; 07 free");
    assert_eq!((fb.units, led.max_charge, led.charged_blocks), (400, 250, 2));
    // At 2 s (crate v2's C8 reading): A takes 300 of block 05-06 (cap 500), charged to both seconds; B gets 500 - 300 = 200, then 07-08.
    let mut led2 = Ledger::new();
    let (_, ga2, _) = fills(&two, &mut led2, 300);
    assert_eq!(ga2, vec![(t(5), 300)]);
    let (_, gb2, _) = fills(&two, &mut led2, 400);
    assert_eq!(gb2, vec![(t(5), 200), (t(7), 200)]);
    // Mixed lengths on one instrument: A at 2 s (300 in block 05-06, on both seconds), B at 1 s: 05 and 06 fully charged, 07 free.
    let mut led3 = Ledger::new();
    let _ = fills(&two, &mut led3, 300);
    let (_, gb3, _) = fills(&one, &mut led3, 400);
    assert_eq!(gb3, vec![(t(7), 250), (t(8), 150)]);
}

/// The ledger's invariant at 1 s on random days: for every second, the takes of all live orders at one level never exceed the level's share
/// of that second's volume (S17); EVERY mode: an entry, its exit and a re-entry anchored at the exit's known instant (1-s lattice) run in
/// one ledger with the retired orders no longer charging.
#[test]
fn c10_ledger_never_above_the_share_of_a_second_and_every_mode_reentry() {
    let mut g = Rng(0x0c10_0008);
    let d = dial();
    for case in 0..80 {
        let n = 300 + g.below(500) as i64;
        let r = random_day(&mut g, n, case % 5);
        if r.is_empty() {
            continue;
        }
        let ix = index_of(&r, n + 5);
        let one = Tape::new(&r, &ix, None).unwrap().with_block(1).unwrap();
        let mut led = Ledger::new();
        let mut per_sec: std::collections::BTreeMap<(usize, Us), Units> = std::collections::BTreeMap::new();
        let mut ids = Vec::new();
        for _ in 0..3 {
            let side = if g.below(2) == 0 { Side::Buy } else { Side::Sell };
            let a = t(g.below(n as u64 / 3) as i64);
            let o = order(side, a, a + (1 + g.below(n as u64)) as i64 * US_S, 1, None);
            let mut w = [0_u64; MAXL];
            for l in 0..LV.len() {
                w[l] = 1 + g.below(3_000);
            }
            let mut out = [LevelFill::default(); MAXL];
            let mut got: Vec<(usize, Us, Units)> = Vec::new();
            let id = {
                let mut rec = |l: usize, b: &BlockFill| {
                    if b.why == Why::Filled {
                        got.push((l, b.block.at, b.units));
                    }
                };
                walk_ledger(&one, &o, &d, LevelMask::all(LV.len()), &mut w, None, None, &mut led, &mut out, Some(&mut rec)).unwrap()
            };
            ids.push(id);
            for (l, at, u) in got {
                *per_sec.entry((l, at)).or_insert(0) += u;
            }
        }
        for (&(l, at), &u) in &per_sec {
            let vol: u64 = r.iter().filter(|s| s.ts == at && s.vol > 0 && s.high > 0 && s.low > 0 && s.high >= s.low).map(|s| s.vol).sum();
            let cap = vol * u64::from(LV[l]) / 10_000;
            assert!(u <= cap, "case {case} level {l} second {at}: {u} taken > share {cap}");
        }
        assert_eq!(led.live(), 3);
        for id in ids {
            led.retire(id);
        }
        assert_eq!(led.live(), 0);
    }
    // EVERY mode (synthetic): entry, exit, re-entry at the exit's known instant, one ledger, one-second lattice.
    let r = rows(&[(5, 10_000, 9_900, 1_000), (6, 10_100, 9_950, 1_000), (9, 10_300, 10_200, 1_000), (10, 10_400, 10_300, 1_000), (12, 10_500, 10_400, 1_000)]);
    let ix = index_of(&r, 60);
    let one = Tape::new(&r, &ix, None).unwrap().with_block(1).unwrap();
    let mut led = Ledger::new();
    let leg = |led: &mut Ledger, side: Side, at: Us, want: Units| -> (u32, LevelFill) {
        let mut w = [0_u64; MAXL];
        w[4] = want;
        let mut out = [LevelFill::default(); MAXL];
        let id = walk_ledger(&one, &order(side, at, t(60), 1, None), &d, LevelMask::one(4), &mut w, None, None, led, &mut out, None).unwrap();
        (id, out[4])
    };
    let (e1, f1) = leg(&mut led, Side::Buy, t(5), 1_500);
    assert_eq!((f1.units, f1.known_at), (1_500, t(7)), "1,000 in 05, 500 in 06, known at 06's end");
    led.retire(e1);
    let (x1, fx) = leg(&mut led, Side::Sell, t(9), 1_500);
    assert_eq!((fx.units, fx.known_at), (1_500, t(11)));
    led.retire(x1);
    let (_, f2) = leg(&mut led, Side::Buy, fx.known_at, 800);
    assert_eq!((f2.units, f2.first.unwrap().block.at, f2.known_at), (800, t(12), t(13)), "the re-entry at the exit's known instant, the retired orders charging nothing");
    assert_eq!(led.max_charge, 0);
}

/// The ledger's block length is validated: whole seconds 1..=60 (BlockInvalid otherwise); `open` keeps crate v2's 2-second lattice.
#[test]
fn c10_ledger_open_blk_validates_the_length() {
    let mut led = Ledger::new();
    for bad in [0_i64, -US_S, 61 * US_S, US_S + 1, US_S / 2, i64::MIN, i64::MAX] {
        assert_eq!(led.open_blk(t(0), t(10), bad).err(), Some(FillErr::BlockInvalid), "length {bad}");
    }
    for good in [US_S, 2 * US_S, 60 * US_S] {
        assert!(led.open_blk(t(0), t(10), good).is_ok(), "length {good}");
    }
    let id = led.open(t(0), t(10));
    assert_eq!(id, 3, "ids count every registered order");
}

/// The lattice helper on every block length: the first lattice instant of `anchor` at or after `t`; at 2 s it equals crate v2's helper.
#[test]
fn c10_lattice_at_or_after_on_every_block_length() {
    let a = t(5);
    assert_eq!(lattice_at_or_after_blk(a, t(5) + 1, US_S), Some(t(6)));
    assert_eq!(lattice_at_or_after_blk(a, t(7), US_S), Some(t(7)));
    assert_eq!(lattice_at_or_after_blk(a, t(7) + 1, 3 * US_S), Some(t(8)));
    assert_eq!(lattice_at_or_after_blk(a, t(1), US_S), Some(a), "before the anchor: the anchor");
    for bad in [0_i64, -US_S] {
        assert_eq!(lattice_at_or_after_blk(a, t(9), bad), None);
    }
    assert_eq!(lattice_at_or_after_blk(a, i64::MAX, US_S), None, "overflow refused, never wrapped");
    let mut g = Rng(0x0c10_000a);
    for _ in 0..2_000 {
        let x = t(5) + g.below(600_000_000) as i64;
        assert_eq!(lattice_at_or_after_blk(a, x, BLOCK_US), Some(lattice_at_or_after(a, x)));
    }
}

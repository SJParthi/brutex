//! Crate v1 tests (kept in crate v2; a test pinning a reading a change C1-C8 replaces is moved with that change, named by it). RED FIRST
//! in crate v1: under `--features red` every law function returned its stub; in crate v2 the red build carries the OLD rule instead;
//! then green on the same sources. Each test names what it proves. Worked numbers are in the comments; every figure is an integer (paisa,
//! microseconds, units). E1 (KEI 2026-10-22) and E2 (HCLTECH 2026-10-23) use the REAL seconds of the design's section 6 tables (read from
//! the frozen store shadow/fib-frozen-2 by critic A, confirmed by the Q70 lane); every other case is synthetic and says so.
//!
//! The independent oracle `oracle_walk` below is a second, deliberately naive implementation of crate v1's readings (every lattice block
//! visited, no index, no jump): the property and matrix tests compare the crate's one-pass walk with it field by field.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic, clippy::too_many_lines, clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::cast_possible_wrap, clippy::similar_names, clippy::many_single_char_names, clippy::too_many_arguments, clippy::needless_range_loop, clippy::single_match_else, clippy::assertions_on_constants, clippy::type_complexity, reason = "tests")]

use crate::*;

/// The day's first second in these tests (a whole second; 09:15:00 of a hand-built day). Offsets below are seconds after it.
const BASE: Us = 1_793_000_000 * US_S;
/// 15:10:00 and 15:30:00 as offsets from 09:15:00.
const SQ: i64 = 21_300;
const END: i64 = 22_500;
const LV: [u16; 5] = [1_000, 2_500, 5_000, 7_500, 10_000];

const fn t(s: i64) -> Us {
    BASE + s * US_S
}

fn rows(v: &[(i64, Paisa, Paisa, u64)]) -> Vec<SecIn> {
    v.iter().map(|&(s, high, low, vol)| SecIn { ts: t(s), high, low, vol }).collect()
}

fn index(r: &[SecIn]) -> SecIndex {
    SecIndex::new(r, BASE, t(END)).expect("the day index")
}

fn ord(side: Side, anchor_s: i64, stop_s: i64) -> Order {
    Order { side, anchor: t(anchor_s), stop: t(stop_s), unit: 1, limit: None, kind: Kind::Entry }
}

/// What one walk gave: per level its summary, its FILLED block rows, every row (all `Why`s) and the rest.
struct Got {
    f: [LevelFill; MAXL],
    fills: Vec<Vec<BlockFill>>,
    all: Vec<Vec<BlockFill>>,
    rest: [Units; MAXL],
}

fn go_with(r: &[SecIn], ix: &SecIndex, o: &Order, levels: &[u16], want: Units, money: Option<Paisa>, cap: Option<Paisa>) -> Result<Got, FillErr> {
    let tape = Tape::new(r, ix, None)?;
    let dial = Dial::new(levels)?;
    let mask = LevelMask::all(dial.len());
    let mut w = [0_u64; MAXL];
    for x in w.iter_mut().take(dial.len()) {
        *x = want;
    }
    let mut f = [LevelFill::default(); MAXL];
    let mut fills: Vec<Vec<BlockFill>> = vec![Vec::new(); MAXL];
    let mut all: Vec<Vec<BlockFill>> = vec![Vec::new(); MAXL];
    let pay = money.map(|p| [Some(p); MAXL]);
    {
        let mut rec = |l: usize, b: &BlockFill| {
            all[l].push(*b);
            if b.why == Why::Filled {
                fills[l].push(*b);
            }
        };
        walk(&tape, o, &dial, mask, &mut w, pay.as_ref(), cap, &mut f, Some(&mut rec))?;
    }
    Ok(Got { f, fills, all, rest: w })
}

fn go(r: &[SecIn], ix: &SecIndex, o: &Order, levels: &[u16], want: Units) -> Got {
    go_with(r, ix, o, levels, want, None, None).expect("the walk")
}

fn ups(v: &[BlockFill]) -> Vec<(Us, Units, Paisa)> {
    v.iter().map(|b| (b.block.at, b.units, b.px)).collect()
}

// =====================================================================================================================================
// The independent oracle: crate v1's readings, naively (every block of the lattice from the anchor to the stop; no index; no jump)
// =====================================================================================================================================

#[derive(Clone, Debug, PartialEq, Eq)]
struct OFill {
    at: Us,
    to: Us,
    n: u32,
    high: Paisa,
    low: Paisa,
    vol: Units,
    units: Units,
    px: Paisa,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct OOut {
    fills: Vec<OFill>,
    units: Units,
    value: Paisa,
    past_limit: u32,
    past_cap: u32,
    unpaid: Units,
    rest: Units,
    known: Us,
}

/// sample-6's req70-2s at ONE level, as plainly as it can be written: block k = [anchor + 2k s, min(anchor + 2k s + 2 s, stop)); its
/// VALID rows (volume > 0, 0 < LOW <= HIGH) price it (a buy the highest HIGH, a sell the lowest LOW) and give its volume; a target limit
/// fills only in a block traded strictly beyond it, at the limit; protection and the money cap skip a block; the payable exposure stops
/// the order at the units it can pay (the rest cancelled).
#[allow(clippy::too_many_arguments)]
fn oracle_walk(r: &[SecIn], o: &Order, bp: u16, want: Units, money: Option<Paisa>, cap: Option<Paisa>) -> OOut {
    let mut out = OOut { rest: want, ..OOut::default() };
    let unit = o.unit.max(1);
    let mut at = o.anchor;
    let mut j = 0_usize;
    while at < o.stop && out.rest > 0 {
        let to = (at + 2 * US_S).min(o.stop);
        // C1 (S9): a block clipped by the stop takes its PRICE over both seconds (the volume stays its seconds before the stop).
        let full = at + 2 * US_S;
        while j < r.len() && r[j].ts < at {
            j += 1;
        }
        let (mut n, mut hi, mut lo, mut vol) = (0_u32, i64::MIN, i64::MAX, 0_u64);
        let mut k = j;
        let mut last_sec = i64::MIN;
        while k < r.len() && r[k].ts < to {
            let s = r[k];
            if s.vol > 0 && s.high > 0 && s.low > 0 && s.high >= s.low {
                // C3 (S5): n counts traded SECONDS (a repeated stamp counts once).
                if s.ts != last_sec {
                    n += 1;
                    last_sec = s.ts;
                }
                hi = hi.max(s.high);
                lo = lo.min(s.low);
                vol += s.vol;
            }
            k += 1;
        }
        // A resting target limit reads only the seconds before its cancel (C1 repair 1).
        let (hi_pre, lo_pre) = (hi, lo);
        if n > 0 && to < full {
            let mut q = k;
            while q < r.len() && r[q].ts < full {
                let s = r[q];
                if s.vol > 0 && s.high > 0 && s.low > 0 && s.high >= s.low {
                    hi = hi.max(s.high);
                    lo = lo.min(s.low);
                }
                q += 1;
            }
        }
        if n > 0 {
            let (px, past) = match o.kind {
                Kind::TargetLimit { px } => (px, !(if o.side == Side::Sell { hi_pre > px } else { lo_pre < px })),
                _ => {
                    let px = if o.side == Side::Buy { hi } else { lo };
                    (px, o.limit.is_some_and(|l| if o.side == Side::Buy { px > l } else { px < l }))
                }
            };
            let target = matches!(o.kind, Kind::TargetLimit { .. });
            let capped = !past && cap.is_some_and(|c| px > c);
            if past {
                if !target {
                    out.past_limit += 1;
                }
            } else if capped {
                out.past_cap += 1;
            } else {
                let capu = (u128::from(vol) * u128::from(bp) / 10_000) as u64 / unit * unit;
                let mut take = capu.min(out.rest);
                let mut cut = false;
                if let Some(p) = money
                    && take > 0
                    && px > 0
                {
                    let room = (p - out.value).max(0);
                    let fit = (room / px) as u64 / unit * unit;
                    if fit < take {
                        take = fit;
                        cut = true;
                    }
                }
                if take > 0 {
                    out.units += take;
                    out.value += take as i64 * px;
                    out.rest -= take;
                    out.known = out.known.max(full);
                    out.fills.push(OFill { at, to, n, high: hi, low: lo, vol, units: take, px });
                }
                if cut {
                    out.unpaid += out.rest;
                    out.rest = 0;
                }
            }
        }
        at += 2 * US_S;
    }
    out
}

fn same_as_oracle(name: &str, r: &[SecIn], g: &Got, o: &Order, levels: &[u16], want: Units, money: Option<Paisa>, cap: Option<Paisa>) -> usize {
    let mut filled_levels = 0;
    for (l, &bp) in levels.iter().enumerate() {
        let x = oracle_walk(r, o, bp, want, money, cap);
        let mine: Vec<OFill> = g.fills[l].iter().map(|b| OFill { at: b.block.at, to: b.block.to, n: b.block.n, high: b.block.high, low: b.block.low, vol: b.block.vol, units: b.units, px: b.px }).collect();
        assert_eq!(mine, x.fills, "{name}: level {bp}: the block fills");
        let f = &g.f[l];
        assert_eq!((f.units, f.value, f.past_limit, f.past_cap, f.unpaid, g.rest[l], f.known_at), (x.units, x.value, x.past_limit, x.past_cap, x.unpaid, x.rest, x.known), "{name}: level {bp}: the summary");
        assert_eq!(f.rest_at_stop, x.rest, "{name}: level {bp}: rest_at_stop");
        assert_eq!(u64::from(f.blocks_filled), x.fills.len() as u64, "{name}: level {bp}: blocks filled");
        if x.units > 0 {
            filled_levels += 1;
            let first = x.fills.first().unwrap();
            assert_eq!((f.e0, f.e1), (first.at, first.at + 2 * US_S), "{name}: level {bp}: E0 / E1 (C1: E1 the full block's end)");
        }
        // The units of each filling block = min(rest, floor(share x volume) in whole units) (the checker's F3: exact, not a bound).
        let mut rest = want;
        for b in &g.fills[l] {
            let capu = (u128::from(b.block.vol) * u128::from(bp) / 10_000) as u64 / o.unit.max(1) * o.unit.max(1);
            if money.is_none() {
                assert_eq!(b.units, capu.min(rest), "{name}: level {bp}: units = min(rest, share)");
            }
            rest -= b.units;
            assert_eq!(b.cum, want - rest, "{name}: level {bp}: cum");
        }
    }
    filled_levels
}

// =====================================================================================================================================
// The design's worked examples
// =====================================================================================================================================

#[test]
fn e3_one_pass_five_levels_rest_across_blocks_and_an_empty_block() {
    // SYNTHETIC (design E3): a SALE of 1,000 from 10:00:00 (offset 0 here) to 10:00:30. Blocks: k0 00-01 2,000 traded LOW 499.60; k1 02-03
    // empty; k2 04-05 1,000 LOW 499.20; k3 06-07 6,000 LOW 498.90; k4 08-09 4,000 LOW 498.50.
    let r = rows(&[(0, 50_000, 50_000, 1_200), (1, 49_960, 49_960, 800), (5, 49_920, 49_920, 1_000), (6, 49_890, 49_890, 4_000), (7, 49_900, 49_900, 2_000), (8, 49_850, 49_850, 3_000), (9, 49_870, 49_870, 1_000)]);
    let ix = index(&r);
    let g = go(&r, &ix, &ord(Side::Sell, 0, 30), &LV, 1_000);
    assert_eq!(ups(&g.fills[0]), vec![(t(0), 200, 49_960), (t(4), 100, 49_920), (t(6), 600, 49_890), (t(8), 100, 49_850)], "10%");
    assert_eq!((g.f[0].value, g.f[0].known_at, g.f[0].no_trade), (49_903_000, t(10), 1), "10%: Rs 4,99,030.00, known 10:00:10");
    assert_eq!(ups(&g.fills[1]), vec![(t(0), 500, 49_960), (t(4), 250, 49_920), (t(6), 250, 49_890)], "25%");
    assert_eq!((g.f[1].value, g.f[1].known_at, g.f[1].no_trade), (49_932_500, t(8), 1), "25%: Rs 4,99,325.00, known 10:00:08");
    for l in 2..5 {
        assert_eq!(ups(&g.fills[l]), vec![(t(0), 1_000, 49_960)], "level {}", LV[l]);
        assert_eq!((g.f[l].value, g.f[l].known_at, g.f[l].no_trade), (49_960_000, t(2), 0));
    }
    for l in 0..5 {
        assert_eq!((g.f[l].units, g.rest[l], g.f[l].e0, g.f[l].e1), (1_000, 0, t(0), t(2)), "one path group: E0 / E1 equal at every level");
    }
    assert_eq!(g.fills[0].iter().map(|b| b.k).collect::<Vec<_>>(), vec![0, 2, 3, 4], "block numbers on the order's own lattice");
    assert_eq!(g.fills[0].iter().map(|b| b.cum).collect::<Vec<_>>(), vec![200, 300, 900, 1_000]);
}

#[test]
fn e1_kei_2026_10_22_real_seconds_entry_0915_05_06_and_exit_0917_40_at_all_five_levels() {
    // REAL (design E1; offsets from 09:15:00): 09:15:04 4016.10 x 1,122; :05 4000.20 x 605; :06 3987.40 x 1,294; :07 3987.50 x 937; :08
    // 3996.10 x 1,440; 09:17:40 3946.40 x 378; :41 3946.80 x 326; :42 3937.10 x 1,674; :43 3934.50 x 277 (one price a second).
    let r = rows(&[(4, 401_610, 401_610, 1_122), (5, 400_020, 400_020, 605), (6, 398_740, 398_740, 1_294), (7, 398_750, 398_750, 937), (8, 399_610, 399_610, 1_440), (160, 394_640, 394_640, 378), (161, 394_680, 394_680, 326), (162, 393_710, 393_710, 1_674), (163, 393_450, 393_450, 277)]);
    let ix = index(&r);
    // The short entry: 124 SOLD from the candle's END 09:15:05.000, block 09:15:05-06 at its LOWEST LOW 3987.40 (1,899 traded): every level.
    let e = go(&r, &ix, &ord(Side::Sell, 5, 160), &LV, 124);
    for l in 0..5 {
        assert_eq!(ups(&e.fills[l]), vec![(t(5), 124, 398_740)], "entry level {}", LV[l]);
        let b = e.fills[l][0].block;
        assert_eq!((b.to, b.n, b.vol, b.n_secs, b.one_price, b.clipped), (t(7), 2, 1_899, 2, 2, false));
        assert_eq!(e.f[l].value, 49_443_760, "Rs 4,94,437.60");
    }
    // The buy-back from the Supertrend candle's END 09:17:40.000: block 09:17:40-41 HIGH 3946.80 (704 traded): 25-100% all 124 there; 10% 70
    // there and the rest 54 in 09:17:42-43 at ITS highest HIGH 3937.10 (1,951 traded).
    let x = go(&r, &ix, &Order { kind: Kind::SignalExit, ..ord(Side::Buy, 160, SQ) }, &LV, 124);
    assert_eq!(ups(&x.fills[0]), vec![(t(160), 70, 394_680), (t(162), 54, 393_710)], "exit 10%");
    assert_eq!(x.f[0].value, 48_887_940, "Rs 4,88,879.40: gross Rs 5,558.20");
    for l in 1..5 {
        assert_eq!(ups(&x.fills[l]), vec![(t(160), 124, 394_680)], "exit level {}", LV[l]);
        assert_eq!(x.f[l].value, 48_940_320, "Rs 4,89,403.20: gross Rs 5,034.40");
    }
    assert_eq!(e.f[1].value - x.f[1].value, 503_440);
    assert_eq!(e.f[0].value - x.f[0].value, 555_820);
}

#[test]
fn e2_hcltech_2026_10_23_real_seconds_entry_0915_05_06_and_exit_block_0916_25_26_without_the_09_16_24_print() {
    // REAL (design E2): 09:15:04 1461.90 x 2,693; :05 1461.70 x 5,521; :06 1461.20 x 6,309; 09:16:24 1457.50 x 11,959; 09:16:25 NO ROW;
    // 09:16:26 1459.00 x 9,355. Long 342: BUY block 09:15:05-06 at 1461.70 (11,830 traded), SELL block 09:16:25-26 = 09:16:26 alone at 1459.00.
    let r = rows(&[(4, 146_190, 146_190, 2_693), (5, 146_170, 146_170, 5_521), (6, 146_120, 146_120, 6_309), (84, 145_750, 145_750, 11_959), (86, 145_900, 145_900, 9_355)]);
    let ix = index(&r);
    let e = go(&r, &ix, &ord(Side::Buy, 5, 85), &LV, 342);
    let x = go(&r, &ix, &Order { kind: Kind::SignalExit, ..ord(Side::Sell, 85, SQ) }, &LV, 342);
    for l in 0..5 {
        assert_eq!(ups(&e.fills[l]), vec![(t(5), 342, 146_170)], "entry level {}", LV[l]);
        assert_eq!(e.f[l].value, 49_990_140, "Rs 4,99,901.40");
        assert_eq!(ups(&x.fills[l]), vec![(t(85), 342, 145_900)], "exit level {}", LV[l]);
        assert_eq!((x.fills[l][0].block.n, x.fills[l][0].block.vol), (1, 9_355), "09:16:24 (the signal candle's) is never in the exit block");
        assert_eq!(x.f[l].value - e.f[l].value, -92_340, "gross -Rs 923.40");
    }
}

// =====================================================================================================================================
// sample-6's checked block law, case by case (SYNTHETIC; the same numbers as o5s_r70_tests r70_04 .. r70_21)
// =====================================================================================================================================

#[test]
fn a_fill_splits_across_two_blocks_each_at_its_own_price() {
    let r = rows(&[(2_700, 10_100, 10_090, 400), (2_701, 10_120, 10_100, 600), (2_702, 10_150, 10_120, 1_000), (2_703, 10_130, 10_110, 1_000)]);
    let ix = index(&r);
    let g = go(&r, &ix, &ord(Side::Buy, 2_700, SQ), &[2_500], 500);
    assert_eq!(ups(&g.fills[0]), vec![(t(2_700), 250, 10_120), (t(2_702), 250, 10_150)]);
}

#[test]
fn empty_blocks_fill_nothing_and_the_lattice_jumps_to_the_next_row_in_o1() {
    let r = rows(&[(2_700, 10_000, 9_990, 100), (2_710, 9_900, 9_880, 1_000)]);
    let ix = index(&r);
    let g = go(&r, &ix, &ord(Side::Buy, 2_700, SQ), &[2_500], 100);
    assert_eq!(ups(&g.fills[0]), vec![(t(2_700), 25, 10_000), (t(2_710), 75, 9_900)]);
    assert_eq!((g.f[0].blocks_filled, g.f[0].no_trade, g.f[0].steps.jumps), (2, 4, 1), "four empty blocks in one jump");
}

#[test]
fn the_money_cap_comparison_skips_a_block_priced_above_it_on_both_sides() {
    // Long: 10:00:00-01 HIGH 100.40 (above the cap 100.00), 10:00:02-03 HIGH 99.90. Short: LOW 100.20 above the cap, then LOW 99.80.
    let r = rows(&[(2_700, 10_040, 10_000, 1_000), (2_702, 9_990, 9_980, 1_000)]);
    let ix = index(&r);
    let o = ord(Side::Buy, 2_700, SQ);
    let g = go(&r, &ix, &o, &[2_500], 100);
    assert_eq!((g.f[0].units, g.f[0].first.map(|b| b.px), g.f[0].past_cap), (100, Some(10_040), 0), "req70-2s: no cap");
    let c = go_with(&r, &ix, &o, &[2_500], 100, None, Some(10_000)).unwrap();
    assert_eq!((c.f[0].units, c.f[0].first.map(|b| b.px), c.f[0].past_cap), (100, Some(9_990), 1), "req70-2s-cap");
    let rs = rows(&[(2_700, 10_030, 10_020, 1_000), (2_702, 9_990, 9_980, 1_000)]);
    let ixs = index(&rs);
    let os = ord(Side::Sell, 2_700, SQ);
    let s = go_with(&rs, &ixs, &os, &[2_500], 100, None, Some(10_000)).unwrap();
    assert_eq!((s.f[0].units, s.f[0].first.map(|b| b.px), s.f[0].past_cap), (100, Some(9_980), 1), "a short above the cap is skipped too");
    let s2 = go(&rs, &ixs, &os, &[2_500], 100);
    assert_eq!(s2.f[0].first.map(|b| b.px), Some(10_020));
    assert_eq!(c.all[0].first().map(|b| (b.why, b.units)), Some((Why::PastCap, 0)), "the skipped block's row");
}

#[test]
fn the_1510_square_off_fills_in_blocks_from_its_clock_instant() {
    let r = rows(&[(SQ - 1, 10_000, 9_990, 5_000), (SQ, 9_950, 9_900, 400), (SQ + 2, 9_900, 9_850, 2_000)]);
    let ix = index(&r);
    let g = go(&r, &ix, &Order { kind: Kind::SquareOff, ..ord(Side::Sell, SQ, END) }, &[2_500], 300);
    assert_eq!(ups(&g.fills[0]), vec![(t(SQ), 100, 9_900), (t(SQ + 2), 200, 9_850)], "nothing from 15:09:59");
}

#[test]
fn an_option_leg_fills_whole_lots_only() {
    let r = rows(&[(2_700, 5_000, 4_990, 1_000), (2_702, 5_010, 5_000, 600)]);
    let ix = index(&r);
    let g = go(&r, &ix, &Order { unit: 75, ..ord(Side::Buy, 2_700, SQ) }, &[2_500], 375);
    assert_eq!(g.fills[0].iter().map(|b| b.units).collect::<Vec<_>>(), vec![225, 150]);
    assert_eq!(g.rest[0], 0);
}

#[test]
fn a_halt_the_lattice_jumps_to_the_resumed_block_and_keeps_its_alignment() {
    let r = rows(&[(2_700, 10_000, 9_990, 40), (3_421, 10_500, 10_400, 10_000)]);
    let ix = index(&r);
    let g = go(&r, &ix, &ord(Side::Buy, 2_700, SQ), &[2_500], 100);
    assert_eq!(g.fills[0].iter().map(|b| (b.block.at, b.block.to, b.units, b.px)).collect::<Vec<_>>(), vec![(t(2_700), t(2_702), 10, 10_000), (t(3_420), t(3_422), 90, 10_500)]);
    assert_eq!(g.f[0].no_trade, 359);
}

#[test]
fn dhans_protection_a_block_past_the_limit_fills_nothing_and_a_later_block_fills_at_its_own_price() {
    let r = rows(&[(2_700, 10_150, 10_100, 5_000), (2_702, 10_080, 10_050, 5_000)]);
    let ix = index(&r);
    let g = go(&r, &ix, &Order { limit: Some(10_100), ..ord(Side::Buy, 2_700, SQ) }, &[2_500], 100);
    assert_eq!(g.f[0].past_limit, 1);
    assert_eq!(ups(&g.fills[0]), vec![(t(2_702), 100, 10_080)]);
    assert_eq!(g.all[0].iter().map(|b| (b.why, b.k)).collect::<Vec<_>>(), vec![(Why::PastLimit, 0), (Why::Filled, 1)]);
}

#[test]
fn c1_reading_a_block_crossing_the_stop_takes_its_volume_before_the_stop_and_its_price_over_both_seconds() {
    // MOVED WITH CHANGE C1 (crate v1's test of sample-6's reading, rewritten to S9 / S10, D5 / D29): the entry cancelled at 10:00:03: the
    // block 10:00:02-03 is CLIPPED: its VOLUME is 10:00:02's alone (100), its PRICE the worst of BOTH seconds (10:00:03's HIGH 105.00, not
    // 10:00:02's 100.10, which prints beside as px_pre), known at the FULL block's end 10:00:04.
    let r = rows(&[(2_700, 10_000, 9_990, 100), (2_702, 10_010, 10_000, 100), (2_703, 10_500, 10_400, 100_000)]);
    let ix = index(&r);
    let g = go(&r, &ix, &ord(Side::Buy, 2_700, 2_703), &[2_500], 1_000);
    assert_eq!(g.fills[0].iter().map(|b| (b.block.at, b.block.to, b.px, b.block.vol, b.block.clipped)).collect::<Vec<_>>(), vec![(t(2_700), t(2_702), 10_000, 100, false), (t(2_702), t(2_703), 10_500, 100, true)]);
    assert_eq!((g.rest[0], g.f[0].rest_at_stop, g.f[0].known_at, g.f[0].clipped, g.f[0].value), (950, 950, t(2_704), 1, 25 * 10_000 + 25 * 10_500));
    let b = g.fills[0][1].block;
    assert_eq!((b.high_pre, b.low_pre, b.high, b.low, b.known), (10_010, 10_000, 10_500, 10_000, t(2_704)), "C1: the 1-second price beside");
}

#[test]
fn c2_reading_a_row_with_volume_and_an_invalid_price_refuses_the_day_and_a_row_with_no_volume_is_no_print() {
    // MOVED WITH CHANGE C2 (crate v1's test of sample-6's reading, rewritten to S2): HIGH 0, a HIGH below its LOW, a negative LOW, each
    // with volume: the instrument-day is REFUSED (bad_print, fail closed). A row with volume 0 and prices is no print (counted): the day
    // stands, and nothing reads its price.
    for bad in [(2_700, 0, 0, 500), (2_701, 9_000, 9_500, 500), (2_702, 10_000, -5, 500)] {
        let r = rows(&[bad, (2_704, 10_020, 10_010, 400)]);
        assert_eq!(SecIndex::new(&r, BASE, t(END)).err(), Some(FillErr::BadPrint), "{bad:?}");
    }
    let r = rows(&[(2_703, 12_000, 8_000, 0), (2_704, 10_020, 10_010, 400), (2_705, 10_020, 10_010, 0)]);
    let ix = index(&r);
    assert_eq!((ix.bad_print_rows(), ix.zero_volume_rows()), (0, 2));
    let g = go(&r, &ix, &ord(Side::Buy, 2_702, SQ), &[10_000], 300);
    assert_eq!(ups(&g.fills[0]), vec![(t(2_704), 300, 10_020)]);
    assert_eq!(g.f[0].steps.blocks, 1, "a block with no valid print is not a step of a market order");
    let tape = Tape::new(&r, &ix, None).unwrap();
    assert_eq!((tape.extreme(t(2_700), t(2_710), true), tape.extreme(t(2_700), t(2_710), false)), (Some((10_020, t(2_704))), Some((10_010, t(2_704)))), "the no-volume row's 120.00 / 80.00 is no price");
    let mm = RangeMm::new(&r);
    let tm = Tape::new(&r, &ix, Some(&mm)).unwrap();
    assert_eq!((tm.extreme(t(2_700), t(2_710), true), tm.extreme(t(2_703), t(2_704), true)), (Some((10_020, t(2_704))), None));
}

#[test]
fn every_level_in_one_pass_equals_each_level_walked_alone() {
    let v: Vec<(i64, Paisa, Paisa, u64)> = (0..240_i64).map(|k| (2_700 + k, 10_000 + (k * 37) % 90, 9_950 + (k * 23) % 40, (50 + (k * 71) % 400) as u64)).collect();
    let r = rows(&v);
    let ix = index(&r);
    for side in [Side::Buy, Side::Sell] {
        let o = ord(side, 2_700, 2_880);
        let all = go(&r, &ix, &o, &LV, 5_000);
        for (l, &bp) in LV.iter().enumerate() {
            let one = go(&r, &ix, &o, &[bp], 5_000);
            assert_eq!(all.f[l], one.f[0], "level {bp} {side:?}");
            assert_eq!(all.all[l], one.all[0]);
            assert!(all.f[l].units > 0 && all.f[l].blocks_filled > 0, "level {bp} {side:?}: the stretch fills");
        }
    }
}

#[test]
fn the_payable_exposure_stops_an_entry_at_the_units_it_can_pay_and_cancels_the_rest() {
    // REQ 70a: payable exposure Rs 25,050 at 100.20: 250 units; the order wanted 400 (the block allows 1,000): 250 fill, 150 cancelled.
    let r = rows(&[(2_700, 10_020, 10_000, 4_000)]);
    let ix = index(&r);
    let o = ord(Side::Buy, 2_700, SQ);
    let g = go_with(&r, &ix, &o, &[2_500], 400, Some(2_505_000), None).unwrap();
    assert_eq!((g.f[0].units, g.f[0].unpaid, g.rest[0]), (250, 150, 0));
    let z = go_with(&r, &ix, &o, &[2_500], 400, Some(0), None).unwrap();
    assert_eq!((z.f[0].units, z.f[0].unpaid, z.all[0].first().map(|b| b.why)), (0, 400, Some(Why::Unpaid)), "nothing payable: nothing fills");
    let ok = go_with(&r, &ix, &o, &[2_500], 400, Some(4_008_000), None).unwrap();
    assert_eq!((ok.f[0].units, ok.f[0].unpaid), (400, 0), "exactly payable: no cut");
}

#[test]
fn the_range_table_equals_a_scan_with_ties_to_the_earliest_second() {
    let v: Vec<(i64, Paisa, Paisa, u64)> = (0..500_i64).map(|k| (7_200 + k, 10_000 + (k * 7_919) % 13 * 5, 9_900 + (k * 104_729) % 11 * 5, 10)).collect();
    let r = rows(&v);
    let ix = index(&r);
    let mm = RangeMm::new(&r);
    let fast = Tape::new(&r, &ix, Some(&mm)).unwrap();
    let slow = Tape::new(&r, &ix, None).unwrap();
    let first12 = (0..500_i64).find(|k| (k * 7_919) % 13 == 12).unwrap();
    assert_eq!(fast.extreme(t(7_200), t(7_700), true), Some((10_060, t(7_200 + first12))));
    let mut n = 0;
    for a in (0..500_i64).step_by(17) {
        for len in [1_i64, 2, 3, 5, 64, 129, 400] {
            for high in [true, false] {
                assert_eq!(fast.extreme(t(7_200 + a), t(7_200 + a + len), high), slow.extreme(t(7_200 + a), t(7_200 + a + len), high), "a {a} len {len} high {high}");
                n += 1;
            }
        }
    }
    assert!(n > 400);
    assert_eq!(fast.extreme(t(100), t(200), true), None, "no row in the span");
}

#[test]
fn the_second_index_equals_the_binary_search_inside_and_outside_its_span() {
    let v: Vec<(i64, Paisa, Paisa, u64)> = (0..3_000_i64).filter(|k| k % 3 != 1).map(|k| (k * 2, 10_000, 9_990, 5)).collect();
    let r = rows(&v);
    let ix = index(&r);
    let tape = Tape::new(&r, &ix, None).unwrap();
    let bs = |ts: Us| r.partition_point(|s| s.ts < ts);
    for k in (0..22_600_i64).step_by(7) {
        for off in [0_i64, 1, 500_000, 999_999] {
            let ts = t(k) + off;
            assert_eq!(tape.first_at(ts), bs(ts), "k {k} off {off}");
        }
    }
    for ts in [BASE - 7 * US_S, BASE - 1, t(END + 5), t(1_000_000)] {
        assert_eq!(tape.first_at(ts), bs(ts), "outside the span {ts}");
    }
}

#[test]
fn known_at_is_the_end_of_the_last_filling_block() {
    let r = rows(&[(2_701, 10_000, 9_990, 100), (2_705, 10_010, 10_000, 100)]);
    let ix = index(&r);
    let g = go(&r, &ix, &ord(Side::Buy, 2_700, SQ), &[5_000], 100);
    assert_eq!(ups(&g.fills[0]), vec![(t(2_700), 50, 10_000), (t(2_704), 50, 10_010)]);
    assert_eq!((g.f[0].known_at, g.f[0].e0, g.f[0].e1), (t(2_706), t(2_700), t(2_702)));
}

#[test]
fn order_size_tiny_equal_double_and_100x_the_block_volume() {
    let v: Vec<(i64, Paisa, Paisa, u64)> = (0..400_i64).map(|k| (2_700 + k, 10_000, 9_990, 500)).collect();
    let r = rows(&v);
    let ix = index(&r);
    for (want, blocks) in [(1_u64, 1_u32), (1_000, 1), (2_000, 2), (100_000, 100)] {
        let g = go(&r, &ix, &ord(Side::Buy, 2_700, SQ), &[10_000], want);
        assert_eq!((g.f[0].units, g.f[0].blocks_filled), (want, blocks), "want {want}");
    }
}

#[test]
fn a_target_limit_fills_only_in_a_block_traded_beyond_it_at_the_limit_and_the_levels_share() {
    // sample-6's target leg (crate v1; change C5 makes it a decided exit in crate v2). A long's target 101.00 placed at 10:00:02: 10:00:02-03
    // HIGH 100.50 (no), 10:00:04-05 HIGH 101.00 (a TOUCH: no), 10:00:06-07 HIGH 101.05 (beyond: 25% of 1,000 = 250 AT 101.00), 10:00:08-09
    // HIGH 101.20 (the rest 4,747 at 101.00).
    let r = rows(&[(2_700, 10_000, 9_990, 1_000_000), (2_702, 10_050, 10_000, 1_000_000), (2_704, 10_100, 10_050, 1_000_000), (2_706, 10_105, 10_090, 1_000), (2_708, 10_120, 10_100, 1_000_000), (4_500, 10_000, 9_990, 1_000_000)]);
    let ix = index(&r);
    let o = Order { kind: Kind::TargetLimit { px: 10_100 }, ..ord(Side::Sell, 2_702, SQ) };
    let g = go(&r, &ix, &o, &[2_500], 4_997);
    assert_eq!(ups(&g.fills[0]), vec![(t(2_706), 250, 10_100), (t(2_708), 4_747, 10_100)]);
    assert_eq!((g.f[0].steps.blocks, g.f[0].past_limit), (4, 0), "four blocks read; a block not beyond is not a protection skip");
    // A short's target (a buy-back limit 99.00): fills only where a LOW is strictly below it.
    let rb = rows(&[(2_702, 9_950, 9_900, 1_000), (2_704, 9_950, 9_890, 1_000)]);
    let ixb = index(&rb);
    let ob = Order { kind: Kind::TargetLimit { px: 9_900 }, ..ord(Side::Buy, 2_702, SQ) };
    let gb = go(&rb, &ixb, &ob, &[10_000], 300);
    assert_eq!(ups(&gb.fills[0]), vec![(t(2_704), 300, 9_900)]);
}

#[test]
fn a_target_counts_every_block_it_reads_and_a_market_order_only_blocks_with_a_print() {
    // sample-6's counters, kept exactly: the target loop counted every block it read; walk_blocks counted blocks with a valid print.
    // MOVED WITH C2: the block's non-print row is a NO-VOLUME row (with volume, an invalid price now refuses the day, S2).
    let r = rows(&[(2_702, 0, 0, 0), (2_704, 10_200, 10_150, 1_000)]);
    let ix = index(&r);
    let tg = go(&r, &ix, &Order { kind: Kind::TargetLimit { px: 10_100 }, ..ord(Side::Sell, 2_702, SQ) }, &[2_500], 100);
    assert_eq!((tg.f[0].steps.blocks, tg.f[0].units), (2, 100));
    let mk = go(&r, &ix, &ord(Side::Sell, 2_702, SQ), &[2_500], 100);
    assert_eq!((mk.f[0].steps.blocks, mk.f[0].units), (1, 100));
}

#[test]
fn a_level_whose_first_block_floors_to_zero_lots_starts_later_with_its_own_e0() {
    // Lot 75: block 0 traded 600: 10% = 60 units = 0 lots; 100% = 600 = 8 lots. 10% first fills in block 1 (1,500 traded: 150 = 2 lots).
    let r = rows(&[(2_700, 5_000, 4_990, 600), (2_702, 5_010, 5_000, 1_500)]);
    let ix = index(&r);
    let g = go(&r, &ix, &Order { unit: 75, ..ord(Side::Buy, 2_700, SQ) }, &[1_000, 10_000], 150);
    assert_eq!(ups(&g.fills[0]), vec![(t(2_702), 150, 5_010)]);
    assert_eq!(ups(&g.fills[1]), vec![(t(2_700), 150, 5_000)]);
    assert_eq!((g.f[0].e0, g.f[1].e0, g.f[0].zero_take_blocks), (t(2_702), t(2_700), 1));
    assert_eq!(g.all[0].first().map(|b| (b.why, b.k)), Some((Why::ZeroTake, 0)));
}

#[test]
fn per_level_steps_count_the_blocks_read_while_the_level_still_had_units_to_fill() {
    let v: Vec<(i64, Paisa, Paisa, u64)> = (0..20_i64).map(|k| (2_700 + k, 10_000, 9_990, 100)).collect();
    let r = rows(&v);
    let ix = index(&r);
    let g = go(&r, &ix, &Order { limit: Some(20_000), ..ord(Side::Buy, 2_700, SQ) }, &[1_000, 10_000], 100);
    assert_eq!((g.f[1].steps.blocks, g.f[1].units), (1, 100), "100% done in block 0");
    assert_eq!((g.f[0].steps.blocks, g.f[0].units), (5, 100), "10%: 20 a block, five blocks");
    assert_eq!(g.f[0].steps.table_reads, 1, "one index lookup for the order");
}

#[test]
fn the_extremes_runs_and_the_worst_between_read_as_sample_6_reads() {
    // Tape::extreme reads every PRINT (C2: a no-volume row is no price; crate v1 read every stored row); worst_between reads only valid
    // prints. MOVED WITH C2: the row 7_203 has no volume (with volume its invalid price would refuse the day).
    let r = rows(&[(7_200, 10_050, 10_000, 10), (7_201, 10_100, 9_950, 10), (7_202, 10_100, 9_960, 10), (7_203, 0, 0, 0), (7_204, 10_080, 9_980, 10)]);
    let ix = index(&r);
    let mm = RangeMm::new(&r);
    let tape = Tape::new(&r, &ix, Some(&mm)).unwrap();
    assert_eq!(tape.extreme(t(7_200), t(7_205), true), Some((10_100, t(7_201))), "the earliest of equal highs");
    assert_eq!(tape.extreme(t(7_200), t(7_205), false), Some((9_950, t(7_201))), "C2: the no-volume row's LOW 0 is no price (crate v1 read it)");
    assert_eq!(tape.extreme(t(7_200), t(7_203), false), Some((9_950, t(7_201))));
    assert_eq!(tape.worst_between(t(7_200), t(7_205), true), Some(9_950), "a sale's worst valid LOW");
    assert_eq!(tape.worst_between(t(7_200), t(7_205), false), Some(10_100), "a buy's worst valid HIGH");
    // Run-up / run-down of a long of 100 entered at 100.00 (value 10,000.00) over 7_200..7_203: best HIGH 101.00, worst LOW 99.50.
    assert_eq!(runs(&tape, t(7_200), t(7_203), true, 1_000_000, 100), ((10_000, t(7_201)), (5_000, t(7_201))));
    assert_eq!(runs(&tape, t(7_200), t(7_203), false, 1_000_000, 100), ((5_000, t(7_201)), (10_000, t(7_201))), "a short: run-up on the LOW");
    assert_eq!(runs(&tape, t(7_200), t(7_203), true, 1_000_000, 0), ((0, 0), (0, 0)));
}

#[test]
fn the_law_call_gives_the_headline_and_the_cap_comparison_through_one_door() {
    let r = rows(&[(2_700, 10_040, 10_000, 1_000), (2_702, 9_990, 9_980, 1_000)]);
    let ix = index(&r);
    let tape = Tape::new(&r, &ix, None).unwrap();
    let dial = Dial::new(&[2_500]).unwrap();
    let o = ord(Side::Buy, 2_700, SQ);
    for (law, px, name) in [(Law::Std70, 10_040, LAW_STD), (Law::Std70Cap { cap: 10_000 }, 9_990, LAW_STD_CAP)] {
        let mut w = [100_u64, 0, 0, 0, 0, 0, 0, 0];
        let mut f = [LevelFill::default(); MAXL];
        walk_law(&law, &tape, &o, &dial, LevelMask::all(1), &mut w, None, &mut f, None).unwrap();
        assert_eq!((f[0].units, f[0].first.map(|b| b.px), law.name()), (100, Some(px), name));
    }
}

#[test]
fn the_dial_refuses_levels_that_do_not_rise_or_leave_1_to_10000_bp_and_the_walk_uses_the_global_level_index() {
    assert!(Dial::new(&LV).is_ok());
    for bad in [&[2_500_u16, 1_000][..], &[0], &[10_001], &[], &[5_000, 5_000], &[1, 2, 3, 4, 5, 6, 7, 8, 9]] {
        assert_eq!(Dial::new(bad).err(), Some(FillErr::DialInvalid), "{bad:?}");
    }
    // A mask naming only the 3rd level: only that level's (GLOBAL) slots are touched.
    let r = rows(&[(2_700, 10_000, 9_990, 1_000)]);
    let ix = index(&r);
    let tape = Tape::new(&r, &ix, None).unwrap();
    let dial = Dial::new(&LV).unwrap();
    let mut w = [100_u64; MAXL];
    let mut f = [LevelFill::default(); MAXL];
    walk(&tape, &ord(Side::Buy, 2_700, SQ), &dial, LevelMask(0b100), &mut w, None, None, &mut f, None).unwrap();
    assert_eq!((f[2].units, w[2]), (100, 0), "level 3 (50%) filled");
    assert!(f.iter().enumerate().all(|(l, x)| l == 2 || x.units == 0) && w.iter().enumerate().all(|(l, &x)| l == 2 || x == 100), "no other level touched");
    let mut w2 = [100_u64; MAXL];
    let mut f2 = [LevelFill::default(); MAXL];
    assert_eq!(walk(&tape, &ord(Side::Buy, 2_700, SQ), &dial, LevelMask(0b10_0000), &mut w2, None, None, &mut f2, None), Err(FillErr::DialInvalid), "a mask beyond the dial");
}

#[test]
fn the_day_index_refuses_unsorted_rows_and_stamps_not_on_whole_seconds_and_a_tape_refuses_another_days_index() {
    let good = rows(&[(2_700, 10_000, 9_990, 1_000), (2_700, 10_010, 9_990, 10), (2_701, 10_000, 9_990, 1_000)]);
    assert!(SecIndex::new(&good, BASE, t(END)).is_ok(), "a repeated stamp is allowed (merged into its second)");
    let mut uns = good.clone();
    uns.swap(0, 2);
    assert_eq!(SecIndex::new(&uns, BASE, t(END)).err(), Some(FillErr::Unsorted));
    let mut half = good.clone();
    half[1].ts += 500_000;
    assert_eq!(SecIndex::new(&half, BASE, t(END)).err(), Some(FillErr::NotWholeSecond));
    assert_eq!(SecIndex::new(&good, BASE + 1, t(END)).err(), Some(FillErr::NotWholeSecond), "the span starts on a whole second");
    let ix = index(&good);
    let other = rows(&[(2_700, 10_000, 9_990, 1_000)]);
    assert_eq!(Tape::new(&other, &ix, None).err(), Some(FillErr::IndexMismatch));
    let mm_other = RangeMm::new(&other);
    assert_eq!(Tape::new(&good, &ix, Some(&mm_other)).err(), Some(FillErr::IndexMismatch));
    // The good day walks (so this case fails on the stubs): the block 10:00:00-01 holds three rows of two seconds, 2,010 traded: 25% = 502
    // covers 300, at the block's highest HIGH 100.10 (the repeated stamp's).
    let g = go(&good, &ix, &ord(Side::Buy, 2_700, SQ), &[2_500], 300);
    // MOVED WITH CHANGE C3 (S5): n counts traded SECONDS (crate v1 counted ROWS: 3 here).
    assert_eq!(g.fills[0].first().map(|b| (b.units, b.px, b.block.n, b.block.n_secs, b.block.vol)), Some((300, 10_010, 2, 2, 2_010)), "C3: n counts traded seconds");
}

#[test]
fn an_anchor_not_on_a_whole_second_and_an_overflowing_block_volume_are_refused_never_wrapped() {
    let r = rows(&[(2_700, 10_000, 9_990, u64::MAX / 2 + 1), (2_701, 10_000, 9_990, u64::MAX / 2 + 1)]);
    let ix = index(&r);
    let tape = Tape::new(&r, &ix, None).unwrap();
    let dial = Dial::new(&[2_500]).unwrap();
    let mut w = [100_u64, 0, 0, 0, 0, 0, 0, 0];
    let mut f = [LevelFill::default(); MAXL];
    assert_eq!(walk(&tape, &ord(Side::Buy, 2_700, SQ), &dial, LevelMask::all(1), &mut w, None, None, &mut f, None), Err(FillErr::Overflow));
    let o = Order { anchor: t(2_700) + 1, ..ord(Side::Buy, 2_700, SQ) };
    assert_eq!(walk(&tape, &o, &dial, LevelMask::all(1), &mut w, None, None, &mut f, None), Err(FillErr::NotWholeSecond));
}

#[test]
fn the_red_guard_a_green_build_says_std70_and_lets_a_headline_run() {
    assert!(!RED);
    assert_eq!(refuse_if_red(), Ok(()));
    assert_eq!(law_compiled(), "std70");
    assert_eq!(VERSION, "v2");
}

// =====================================================================================================================================
// The extreme-case matrix and the property test: the crate's one-pass walk = the naive oracle, field by field
// =====================================================================================================================================

/// The block states of the matrix (each a SYNTHETIC stretch from 11:00:00, offset 6,300 s): the rows and the order's (anchor, stop).
fn state(si: usize) -> (&'static str, Vec<(i64, Paisa, Paisa, u64)>, i64, i64) {
    let a = 6_300;
    let mut v: Vec<(i64, Paisa, Paisa, u64)> = Vec::new();
    let (mut anchor, mut stop) = (a, SQ);
    let name = match si {
        0 => {
            (0..600).for_each(|k| v.push((a + k, 10_000 + k % 7 * 5, 9_990 - k % 5 * 5, 500)));
            "both seconds traded"
        }
        1 => {
            (0..600).filter(|k| k % 2 == 0).for_each(|k| v.push((a + k, 10_000 + k % 7 * 5, 9_990, 1_000)));
            "only the first second traded"
        }
        2 => {
            (0..600).filter(|k| k % 2 == 1).for_each(|k| v.push((a + k, 10_000, 9_990 - k % 5 * 5, 1_000)));
            "only the second second traded"
        }
        3 => {
            (300..900).for_each(|k| v.push((a + k, 10_100, 10_050, 500)));
            "no trade for minutes, then prints"
        }
        4 => {
            v.push((a, 10_000, 9_990, 500));
            (401..1_001).for_each(|k| v.push((a + k, 10_200, 10_150, 500)));
            "a halt inside a block, resumed on an odd second"
        }
        5 => {
            (0..600).for_each(|k| {
                v.push((a + k, 10_000, 9_990, 300));
                if k % 3 == 0 {
                    v.push((a + k, 10_050, 9_950, 200));
                }
            });
            "a repeated stamp (two rows of one second)"
        }
        6 => {
            (0..600).for_each(|k| v.push((a + k, if k % 4 == 0 { 0 } else { 10_000 }, if k % 4 == 1 { -10 } else { 9_990 }, 500)));
            "zero and negative prices"
        }
        7 => {
            (0..600).for_each(|k| v.push((a + k, if k % 3 == 0 { 9_000 } else { 10_000 }, if k % 3 == 0 { 9_500 } else { 9_990 }, 500)));
            "a HIGH below its LOW"
        }
        8 => {
            (0..600).for_each(|k| v.push((a + k, 10_000 + k % 3 * 5, 9_990, 500)));
            stop = a + 5;
            "the order's stop inside a block (clipped)"
        }
        9 => {
            anchor = END - 2;
            stop = END;
            v.push((END - 2, 10_000, 9_990, 300));
            v.push((END - 1, 10_010, 9_980, 300));
            "the session's last block"
        }
        10 => {
            (0..600).for_each(|k| v.push((a + k, 10_000, 9_990, u64::from(u32::MAX) - 7)));
            "volume near the 32-bit ceiling"
        }
        11 => {
            (0..600).for_each(|k| v.push((a + k, if k % 2 == 0 { 10_000 } else { 11_000 }, if k % 2 == 0 { 9_990 } else { 10_990 }, 500)));
            "a price-band jump between the seconds"
        }
        12 => {
            (0..600).for_each(|k| v.push((a + k, 10_000 + k % 11 * 7, 9_990 - k % 13 * 3, (k % 9) as u64)));
            "thin seconds: many blocks floor to zero"
        }
        _ => {
            stop = a;
            (0..10).for_each(|k| v.push((a + k, 10_000, 9_990, 500)));
            "the stop at the anchor: nothing"
        }
    };
    v.sort_by_key(|x| x.0);
    (name, v, anchor, stop)
}

#[test]
fn extreme_matrix_state_x_side_x_order_kind_x_unit_x_size_x_level_x_limit_x_cap_x_money_equals_the_oracle() {
    let sizes: [Units; 6] = [0, 1, 75, 1_000, 2_000, 100_000];
    let kinds = [Kind::Entry, Kind::SignalExit, Kind::SquareOff, Kind::TargetLimit { px: 10_010 }];
    let mut cases = 0_usize;
    let mut filled = 0_usize;
    let mut refused = 0_usize;
    for si in 0..14 {
        let (sname, v, anchor_s, stop_s) = state(si);
        let r = rows(&v);
        // C2 (S2): a day holding a row with volume and an invalid price (states 6 and 7) is REFUSED: every case of it is a refusal.
        let ix = match SecIndex::new(&r, BASE, t(END)) {
            Ok(ix) => ix,
            Err(e) => {
                assert_eq!((e, si == 6 || si == 7), (FillErr::BadPrint, true), "{sname}: refused");
                refused += 1;
                cases += 2 * 4 * 2 * 6 * 2 * 2 * 2;
                continue;
            }
        };
        for side in [Side::Buy, Side::Sell] {
            for kind in kinds {
                for unit in [1_u64, 75] {
                    for &want in &sizes {
                        // Crate v2 step A0 refuses a want that is not a whole number of units (OB8; crate v1 filled a partial lot): whole lots here.
                        let want = want / unit * unit;
                        for (limit, lname) in [(None, "no limit"), (Some(if side == Side::Buy { 10_020 } else { 9_975 }), "protection")] {
                            for (cap, cname) in [(None, "no cap"), (Some(10_005), "cap 100.05")] {
                                for (money, mname) in [(None, "uncapped money"), (Some(5_000_000), "pay Rs 50,000")] {
                                    let o = Order { side, anchor: t(anchor_s), stop: t(stop_s), unit, limit, kind };
                                    let name = format!("{sname} / {side:?} / {kind:?} / unit {unit} / want {want} / {lname} / {cname} / {mname}");
                                    let g = go_with(&r, &ix, &o, &LV, want, money, cap).unwrap();
                                    filled += same_as_oracle(&name, &r, &g, &o, &LV, want, money, cap);
                                    // Lattice, stop and anchor: every row on the order's own 2-second lattice, inside [anchor, stop).
                                    for l in 0..LV.len() {
                                        for b in &g.all[l] {
                                            assert!(b.block.at >= o.anchor && b.block.to <= o.stop && (b.block.at - o.anchor) % (2 * US_S) == 0, "{name}: the lattice");
                                            assert!(b.block.to - b.block.at == 2 * US_S || b.block.to == o.stop, "{name}: a short block only at the stop");
                                        }
                                        // Monotone in the share: a higher level never fills fewer units in total.
                                        if l > 0 && money.is_none() {
                                            assert!(g.f[l].units >= g.f[l - 1].units, "{name}: a higher share filled fewer");
                                        }
                                    }
                                    cases += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    assert_eq!((cases, refused), (14 * 2 * 4 * 2 * 6 * 2 * 2 * 2, 2));
    assert!(filled > cases, "many (case, level)s fill: {filled} of {}", cases * 5);
}

/// A small deterministic generator (no dependency, no float).
struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 33
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n.max(1)
    }
}

#[test]
fn property_random_days_and_orders_one_pass_equals_the_oracle_and_each_level_alone() {
    let mut g = Lcg(0x5eed_7025);
    let mut filled = 0_usize;
    for case in 0..400 {
        // A day of up to 900 rows from 10:00:00 (gaps, repeated stamps, a few invalid rows, thin and thick seconds).
        let mut v: Vec<(i64, Paisa, Paisa, u64)> = Vec::new();
        let mut s = 2_700_i64;
        let n = 50 + g.below(850);
        for _ in 0..n {
            s += match g.below(10) {
                0 => 0,
                1..=6 => 1,
                7 | 8 => 2 + g.below(5) as i64,
                _ => 20 + g.below(200) as i64,
            };
            let base = 10_000 + g.below(400) as i64;
            let spread = g.below(4) as i64 * 5;
            let (hi, lo) = if g.below(60) == 0 { (base - 10, base) } else { (base + spread, base) };
            let vol = if g.below(8) == 0 { g.below(10) } else { 1 + g.below(3_000) };
            // C2 (S2): an invalid price with volume refuses the day, so the generated invalid rows are NO prints (volume 0).
            let vol = if hi < lo { 0 } else { vol };
            v.push((s, hi, lo, vol));
        }
        let r = rows(&v);
        let ix = index(&r);
        let anchor = 2_700 + g.below(300) as i64;
        let stop = anchor + 1 + g.below(1_200) as i64;
        let side = if g.below(2) == 0 { Side::Buy } else { Side::Sell };
        let unit = [1_u64, 1, 25, 75][g.below(4) as usize];
        // Crate v2 step A0 refuses a want that is not a whole number of units (OB8): whole lots here.
        let want = [1_u64, 100, 1_000, 5_000, 20_000][g.below(5) as usize] / unit * unit;
        let limit = (g.below(3) == 0).then(|| 10_000 + g.below(400) as i64);
        let kind = if g.below(5) == 0 { Kind::TargetLimit { px: 10_000 + g.below(400) as i64 } } else { Kind::Entry };
        let cap = (g.below(4) == 0).then(|| 10_000 + g.below(400) as i64);
        let money = (g.below(4) == 0).then(|| (g.below(5_000) as i64) * 10_000);
        let o = Order { side, anchor: t(anchor), stop: t(stop), unit, limit, kind };
        let name = format!("case {case}: {side:?} {kind:?} unit {unit} want {want} limit {limit:?} cap {cap:?} money {money:?} rows {n}");
        let all = go_with(&r, &ix, &o, &LV, want, money, cap).unwrap();
        filled += same_as_oracle(&name, &r, &all, &o, &LV, want, money, cap);
        for (l, &bp) in LV.iter().enumerate() {
            let one = go_with(&r, &ix, &o, &[bp], want, money, cap).unwrap();
            assert_eq!((all.f[l], &all.all[l]), (one.f[0], &one.all[0]), "{name}: level {bp}: one pass = alone");
        }
    }
    assert!(filled > 400, "the generated days fill: {filled} (case, level)s");
}

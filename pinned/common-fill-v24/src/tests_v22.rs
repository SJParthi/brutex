//! CRATE v2.2 tests (EXITS-DYNAMIC-DESIGN B1 + critic R3; red first): the extended exit rules ([`crate::xrule`]) against THE LAW (a plain
//! walk per traded second) and against crate v2.1's own `path::scan_one` on every v2.1 rule; the designer's 14 hand-built extreme cases
//! with their expected seconds; the critic's probe-D repeated-stamp tapes (R3); the adversarial chain; bounded and store-independent step
//! counters; the node reference vectors. Every case is SYNTHETIC; every figure an integer (paisa, microseconds).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic, clippy::too_many_lines, clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::cast_possible_wrap, clippy::similar_names, clippy::many_single_char_names, clippy::too_many_arguments, clippy::needless_range_loop, clippy::type_complexity, clippy::cast_lossless, clippy::items_after_statements, reason = "tests")]

use crate::path::{self, EntryView, Rule, Trigger};
use crate::tests_v2::{BASE, Rng, t};
use crate::xrule::{TrailIndex, XRule, scan_fast, scan_law};
use crate::*;

struct Day {
    rows: Vec<SecIn>,
    ix: SecIndex,
    mm: RangeMm,
}

fn day(rows: Vec<SecIn>) -> Day {
    let last = rows.last().map_or(BASE, |r| r.ts);
    let ix = SecIndex::new(&rows, BASE, last + 10 * US_S).unwrap();
    let mm = RangeMm::new(&rows);
    Day { rows, ix, mm }
}

fn tape(d: &Day) -> Tape<'_> {
    Tape::new(&d.rows, &d.ix, Some(&d.mm)).unwrap()
}

/// Rows from (second, HIGH, LOW, volume).
fn mk(v: &[(i64, Paisa, Paisa, u64)]) -> Vec<SecIn> {
    v.iter().map(|&(s, h, l, vol)| SecIn { ts: t(s), high: h, low: l, vol }).collect()
}

fn fill(at_s: i64, units: u64, px: Paisa) -> BlockFill {
    BlockFill { k: 0, block: Block { at: t(at_s), to: t(at_s + 2), known: t(at_s + 2), ..Block::default() }, units, px, cum: 0, why: Why::Filled }
}

fn ev(f: &[BlockFill]) -> EntryView<'_> {
    EntryView { e0: f[0].block.at, e1: f[0].block.known, fills: f }
}

/// The trigger second (offset from BASE in seconds) of one rule on a hand-built tape, by the fast path AND by the law (both asserted equal;
/// -1 = none). The watch window ends at `to_s`.
fn sec_of(d: &Day, fills: &[BlockFill], rule: XRule, to_s: i64, long: bool) -> i64 {
    let tp = tape(d);
    let ti = TrailIndex::new(&tp, long);
    let e = ev(fills);
    let mut st = 0_u32;
    let f = scan_fast(&tp, &ti, &e, t(to_s), long, &[rule], &mut st);
    let l = scan_law(&tp, &e, t(to_s), long, &[rule]);
    let s = |x: &Option<Trigger>| x.map_or(-1, |tr| (tr.sec - BASE) / US_S);
    assert_eq!(s(&f[0]), s(&l[0]), "fast vs law on {rule:?}");
    s(&f[0])
}

const SQ: i64 = 54_600; // 15:10:00 as a second of the day (the hand cases use seconds of the day as offsets)

fn one(at: i64, px: Paisa) -> Vec<BlockFill> {
    vec![fill(at, 10, px)]
}

// ---------------------------------------------------------------------------------------------- the designer's 14 extreme cases (6.2)

#[test]
fn sf3_be_wake_second_not_tested() {
    let d = day(mk(&[(35_999, 10_000, 10_000, 100), (36_000, 10_000, 9_900, 100), (36_001, 10_100, 7_900, 100), (36_002, 9_000, 8_800, 100), (36_003, 10_050, 9_500, 100)]));
    assert_eq!(sec_of(&d, &one(36_000, 10_000), XRule::ArmedStop { act: 2_000, off: 0 }, SQ, false), 36_003);
    // critic probe D (R3): the wake row and a row touching the stop in the SAME second 36001: the law reads the second once (merged):
    // the wake second is never tested: no trigger
    let d2 = day(mk(&[(36_000, 100, 100, 100), (36_001, 95, 89, 100), (36_001, 101, 95, 100), (36_002, 96, 92, 100)]));
    assert_eq!(sec_of(&d2, &one(36_000, 100), XRule::ArmedStop { act: 10, off: 0 }, SQ, false), -1);
}

#[test]
fn sf3_tsl_wake_second_not_tested_against_own_level() {
    let d = day(mk(&[(36_000, 10_000, 9_950, 100), (36_001, 9_600, 8_900, 100), (36_002, 9_300, 9_000, 100), (36_003, 9_450, 9_100, 100)]));
    assert_eq!(sec_of(&d, &one(36_000, 10_000), XRule::Trail { act: 1_000, dist: 500 }, SQ, false), 36_003);
    // a wake second whose own HIGH reaches its own new level, a piece that does not fire, then a later piece that does
    let d2 = day(mk(&[(36_000, 10_000, 10_000, 100), (36_001, 9_600, 9_000, 100), (36_002, 9_300, 8_900, 100), (36_003, 9_400, 9_000, 100)]));
    assert_eq!(sec_of(&d2, &one(36_000, 10_000), XRule::Trail { act: 1_000, dist: 500 }, SQ, false), 36_003);
    // the same for the relative trail (bp 500 of 9,000 = 450; of 8,900 = 445: level 9,345); bp 600 (of 8,900 = 534: level 9,434) never fires
    assert_eq!(sec_of(&d2, &one(36_000, 10_000), XRule::TrailPct { act: 1_000, bp: 500 }, SQ, false), 36_003);
    assert_eq!(sec_of(&d2, &one(36_000, 10_000), XRule::TrailPct { act: 1_000, bp: 600 }, SQ, false), -1);
}

#[test]
fn sf3_gapped_stop_trigger_is_the_gap_second() {
    let d = day(mk(&[(36_000, 10_000, 9_950, 100), (36_001, 10_500, 10_000, 100), (36_002, 21_000, 19_000, 100)]));
    assert_eq!(sec_of(&d, &one(36_000, 10_000), XRule::Stop { x: 2_000 }, SQ, false), 36_002);
    assert_eq!(sec_of(&d, &one(36_000, 10_000), XRule::StopAt { level: 12_000 }, SQ, false), 36_002);
}

#[test]
fn sf3_tp_reaching_is_enough() {
    let d = day(mk(&[(36_000, 10_000, 9_950, 100), (36_001, 9_000, 8_000, 100)]));
    assert_eq!(sec_of(&d, &one(36_000, 10_000), XRule::Target { x: 2_000 }, SQ, false), 36_001);
    assert_eq!(sec_of(&d, &one(36_000, 10_000), XRule::TargetAt { level: 8_000 }, SQ, false), 36_001);
}

#[test]
fn sf3_trail_fires_inside_1508_1510_is_seen() {
    let d = day(mk(&[(54_000, 10_000, 9_980, 100), (54_100, 9_500, 8_900, 100), (54_200, 9_200, 9_000, 100), (54_510, 9_500, 9_300, 100)]));
    assert_eq!(sec_of(&d, &one(54_000, 10_000), XRule::Trail { act: 1_000, dist: 500 }, SQ, false), 54_510);
    // a watch ending at 15:08:00 (the v2 window) does not see it
    assert_eq!(sec_of(&d, &one(54_000, 10_000), XRule::Trail { act: 1_000, dist: 500 }, 54_480, false), -1);
}

#[test]
fn sf3_hole_then_gap() {
    let d = day(mk(&[(36_000, 10_000, 9_900, 100), (36_010, 10_200, 10_000, 100), (36_056, 11_800, 11_500, 100)]));
    assert_eq!(sec_of(&d, &one(36_000, 10_000), XRule::Stop { x: 1_000 }, SQ, false), 36_056);
}

#[test]
fn sf3_rs005_floor() {
    let d = day(mk(&[(36_000, 5, 5, 100), (36_001, 5, 5, 100), (36_002, 10, 5, 100)]));
    assert_eq!(sec_of(&d, &one(36_000, 5), XRule::Stop { x: 1 }, SQ, false), 36_002);
    assert_eq!(sec_of(&d, &one(36_000, 5), XRule::Target { x: 4 }, SQ, false), -1);
}

#[test]
fn sf3_spike_reverses_before_first_fill() {
    let d = day(mk(&[(35_999, 10_000, 10_000, 100), (36_004, 5_200, 5_000, 100), (36_005, 5_500, 5_100, 100), (36_010, 6_100, 5_800, 100)]));
    assert_eq!(sec_of(&d, &one(36_004, 5_000), XRule::Stop { x: 1_000 }, SQ, false), 36_010);
}

#[test]
fn sf3_partial_fills_r_pieces() {
    let d = day(mk(&[(36_000, 10_000, 10_000, 100), (36_005, 10_400, 10_100, 100), (36_010, 9_100, 9_000, 100), (36_012, 9_800, 9_600, 100)]));
    let f = vec![fill(36_000, 2, 10_000), fill(36_010, 8, 9_000)];
    assert_eq!(sec_of(&d, &f, XRule::Stop { x: 500 }, SQ, false), 36_012);
    // R from 36010 = min(10,000, (2 x 10,000 + 8 x 9,000) / 10 = 9,200) = 9,200: the time-armed stop at R reads it too
    assert_eq!(sec_of(&d, &f, XRule::TimedStop { after: 10, off: 500 }, SQ, false), 36_012);
}

#[test]
fn sf3_go_positive_strict() {
    let d = day(mk(&[(36_000, 10_000, 9_900, 100), (36_059, 10_000, 9_800, 100)]));
    assert_eq!(sec_of(&d, &one(36_000, 10_000), XRule::GoPositive { secs: 60 }, SQ, false), 36_060);
    // strictly in the money: no exit
    let d2 = day(mk(&[(36_000, 10_000, 9_900, 100), (36_059, 9_995, 9_800, 100)]));
    assert_eq!(sec_of(&d2, &one(36_000, 10_000), XRule::GoPositive { secs: 60 }, SQ, false), -1);
}

#[test]
fn sf3_go_no_traded_second_since_e0() {
    // F9: nothing traded in [E0 + 1 s, E0 + 60 s): the last traded second before the timer is E0's own (HIGH = R: not in the money)
    let d = day(mk(&[(36_000, 10_000, 10_000, 100), (36_200, 9_000, 9_000, 100)]));
    assert_eq!(sec_of(&d, &one(36_000, 10_000), XRule::GoPositive { secs: 60 }, SQ, false), 36_060);
}

#[test]
fn sf3_price_only_rows_never_trigger() {
    let d = day(mk(&[(36_000, 10_000, 9_900, 100), (36_001, 20_000, 20_000, 0), (36_002, 10_100, 10_000, 100)]));
    assert_eq!(sec_of(&d, &one(36_000, 10_000), XRule::Stop { x: 1_000 }, SQ, false), -1);
    // a price-only row far BELOW never becomes a trail's new best
    let d2 = day(mk(&[(36_000, 10_000, 10_000, 100), (36_001, 5_000, 4_000, 0), (36_002, 10_400, 9_900, 100), (36_003, 10_400, 10_000, 100)]));
    assert_eq!(sec_of(&d2, &one(36_000, 10_000), XRule::Trail { act: 0, dist: 500 }, SQ, false), 36_003);
    assert_eq!(sec_of(&d2, &one(36_000, 10_000), XRule::TrailPct { act: 0, bp: 500 }, SQ, false), 36_003);
}

#[test]
fn sf3_tsl_relative_follows_running_minimum() {
    let d = day(mk(&[(36_000, 10_000, 9_950, 100), (36_001, 9_000, 8_000, 100), (36_002, 8_200, 6_000, 100), (36_003, 7_400, 7_000, 100), (36_004, 7_600, 7_100, 100)]));
    assert_eq!(sec_of(&d, &one(36_000, 10_000), XRule::TrailPct { act: 1_000, bp: 2_500 }, SQ, false), 36_004);
    // the distance follows the new minimum: 10% of 8,000 = 800 then of 6,000 = 600 (the wake price's 10% would be 800 throughout)
    let d2 = day(mk(&[(36_000, 10_000, 10_000, 100), (36_001, 9_000, 8_000, 100), (36_002, 6_500, 6_000, 100), (36_003, 6_600, 6_100, 100), (36_004, 6_700, 6_200, 100)]));
    assert_eq!(sec_of(&d2, &one(36_000, 10_000), XRule::TrailPct { act: 1_000, bp: 1_000 }, SQ, false), 36_003);
}

#[test]
fn sf3_tt_touch_before_arm_ignored() {
    let d = day(mk(&[(36_000, 10_000, 9_900, 100), (36_100, 10_500, 10_000, 100), (36_301, 10_000, 9_900, 100)]));
    assert_eq!(sec_of(&d, &one(36_000, 10_000), XRule::TimedStop { after: 300, off: 0 }, SQ, false), 36_301);
    // armed exactly at E0 + 300 s (inclusive)
    let d2 = day(mk(&[(36_000, 10_000, 9_900, 100), (36_300, 10_000, 9_900, 100)]));
    assert_eq!(sec_of(&d2, &one(36_000, 10_000), XRule::TimedStop { after: 300, off: 0 }, SQ, false), 36_300);
}

// ---------------------------------------------------------------------------------------------- more extreme cases (critic F9, R3)

#[test]
fn sf3_repeated_stamp_trail_law_merges_the_second() {
    // critic probe D: second 36001 has rows (H100 L90) then (H110 L80): merged (H110 L80) is tested against 95 + 18 = 113 first: no trigger
    let d = day(mk(&[(36_000, 100, 95, 100), (36_001, 100, 90, 100), (36_001, 110, 80, 100), (36_002, 85, 84, 100)]));
    assert_eq!(sec_of(&d, &one(36_000, 100), XRule::Trail { act: 0, dist: 18 }, SQ, false), -1);
    // the merged second sets the new low 80 (level 98): the next second's HIGH 99 fires; a row-wise chain would fire inside the repeated
    // second (row H100 L90 then row H110 L80: 110 >= 90 + 18) and then find no second in that empty piece
    let d3 = day(mk(&[(36_000, 100, 95, 100), (36_001, 100, 90, 100), (36_001, 110, 80, 100), (36_002, 99, 85, 100)]));
    assert_eq!(sec_of(&d3, &one(36_000, 100), XRule::Trail { act: 0, dist: 18 }, SQ, false), 36_002);
    // and crate v2.1's own trail_hit reads it the same way
    let tp = tape(&d);
    let f = one(36_000, 100);
    let mut st = 0;
    let (tg, _) = path::scan_one(&tp, &ev(&f), t(SQ), false, &[Rule::Trail { act: 0, dist: 18 }], &[], &mut st);
    assert_eq!(tg[0], None);
}

#[test]
fn sf3_tsl_act0_wakes_at_or_below_r() {
    let d = day(mk(&[(36_000, 10_000, 10_000, 100), (36_001, 10_400, 10_000, 100), (36_002, 10_500, 10_100, 100)]));
    // act 0: the entry second (LOW = R) wakes it; level 10,000 + 500: 36002's HIGH 10,500 fires
    assert_eq!(sec_of(&d, &one(36_000, 10_000), XRule::Trail { act: 0, dist: 500 }, SQ, false), 36_002);
}

#[test]
fn sf3_be_lock_level_below_entry() {
    // BE 30% with a 10% lock: wake at LOW <= 7,000; then a stop at R - 1,000 = 9,000
    let d = day(mk(&[(36_000, 10_000, 10_000, 100), (36_001, 7_500, 6_900, 100), (36_002, 8_900, 8_000, 100), (36_003, 9_000, 8_500, 100)]));
    assert_eq!(sec_of(&d, &one(36_000, 10_000), XRule::ArmedStop { act: 3_000, off: -1_000 }, SQ, false), 36_003);
}

#[test]
fn sf3_trail_wakes_in_a_later_piece() {
    // R falls from 10,000 to 9,000 at the second block (36010): act 1,000 is reached against the NEW R only at 36011 (LOW 8,000)
    let d = day(mk(&[(36_000, 10_000, 9_100, 100), (36_010, 9_200, 9_000, 100), (36_011, 8_500, 8_000, 100), (36_012, 8_400, 8_100, 100), (36_013, 8_600, 8_200, 100)]));
    let f = vec![fill(36_000, 10, 10_000), fill(36_010, 10, 8_000)];
    assert_eq!(sec_of(&d, &f, XRule::Trail { act: 1_000, dist: 500 }, SQ, false), 36_013);
}

#[test]
fn sf3_long_side_mirror() {
    // a LONG: adverse = LOW down, favourable = HIGH up
    let d = day(mk(&[(36_000, 10_000, 10_000, 100), (36_001, 11_100, 10_400, 100), (36_002, 11_000, 10_700, 100), (36_003, 10_800, 10_550, 100)]));
    assert_eq!(sec_of(&d, &one(36_000, 10_000), XRule::Trail { act: 1_000, dist: 500 }, SQ, true), 36_003);
    assert_eq!(sec_of(&d, &one(36_000, 10_000), XRule::TrailPct { act: 1_000, bp: 400 }, SQ, true), 36_003);
    assert_eq!(sec_of(&d, &one(36_000, 10_000), XRule::ArmedStop { act: 1_000, off: -600 }, SQ, true), 36_003);
    assert_eq!(sec_of(&d, &one(36_000, 10_000), XRule::ArmedStop { act: 1_000, off: -500 }, SQ, true), -1);
    assert_eq!(sec_of(&d, &one(36_000, 10_000), XRule::TargetAt { level: 11_000 }, SQ, true), 36_001);
    assert_eq!(sec_of(&d, &one(36_000, 10_000), XRule::StopAt { level: 10_550 }, SQ, true), 36_000); // a long's absolute stop ABOVE the entry fires at once (the entry second's LOW)
}

#[test]
fn sf3_adversarial_chain_is_bounded() {
    // critic probe C: 21,000 seconds, each a new strictly lower LOW, give-back 9 paisa until the very last second
    let n = 21_000_i64;
    let mut v = Vec::with_capacity(n as usize);
    for i in 0..n {
        let lo = 60_000 - i;
        let hi = if i == n - 1 { 70_000 } else { lo + 9 };
        v.push((33_300 + i, hi, lo, 100_u64));
    }
    let d = day(mk(&v));
    let tp = tape(&d);
    let ti = TrailIndex::new(&tp, false);
    let f = one(33_300, 60_000);
    let mut st = 0_u32;
    let r = scan_fast(&tp, &ti, &ev(&f), t(33_300 + n + 10), false, &[XRule::Trail { act: 0, dist: 50 }], &mut st);
    assert_eq!(r[0].map(|x| (x.sec - BASE) / US_S), Some(33_300 + n - 1));
    assert!(st <= 80, "steps {st}: bounded by about 4 x log2 n, not the 21,000-second walk");
    let l = scan_law(&tp, &ev(&f), t(33_300 + n + 10), false, &[XRule::Trail { act: 0, dist: 50 }]);
    assert_eq!(r, l);
}

#[test]
fn sf3_steps_identical_1x_10x_100x() {
    // the same queries on one day, alone and inside stores of 10 and 100 days: identical per-query step counts (each query reads only its
    // own contract-day's tables)
    let mut g = Rng(77);
    let base_day = rand_day(&mut g, 3_000, 0, 10_000);
    let queries: Vec<(Vec<BlockFill>, XRule, bool)> = (0..200).map(|_| { let f = rand_fills(&mut g, &base_day, 10_000); let r = rand_rule(&mut g, 10_000); (f, r, g.below(2) == 0) }).collect();
    let mut prints: Vec<Vec<u32>> = Vec::new();
    for copies in [1_usize, 10, 100] {
        let days: Vec<Day> = (0..copies).map(|k| if k == 0 { day(base_day.clone()) } else { day(rand_day(&mut Rng(k as u64), 3_000, k as u64 % 6, 10_000)) }).collect();
        let tp = tape(&days[0]);
        let (ts, tl) = (TrailIndex::new(&tp, false), TrailIndex::new(&tp, true));
        let mut v = Vec::new();
        for (f, r, long) in &queries {
            let mut st = 0_u32;
            let _ = scan_fast(&tp, if *long { &tl } else { &ts }, &ev(f), t(4_000), *long, &[*r], &mut st);
            v.push(st);
        }
        prints.push(v);
    }
    assert_eq!(prints[0], prints[1]);
    assert_eq!(prints[1], prints[2]);
}

// ---------------------------------------------------------------------------------------------- randomized equality (seeded)

/// A random tape of `n` seconds from offset 0: style 0 dense, 1 sparse, 2 holes (31-120 s silences), 3 spikes (1.5x-21x), 4 repeated
/// stamps, 5 price-only rows mixed in. `px0` the starting price (paisa).
fn rand_day(g: &mut Rng, n: i64, style: u64, px0: i64) -> Vec<SecIn> {
    let mut v = Vec::new();
    let mut px = px0;
    let mut s = 0_i64;
    while s < n {
        if style == 2 && g.below(200) == 0 {
            s += 31 + g.below(90) as i64;
            continue;
        }
        let print = if style == 1 { g.below(5) == 0 } else { g.below(8) != 0 };
        if print {
            let step = (px / 50).max(1);
            px = (px + g.below(2 * step as u64 + 1) as i64 - step).max(5);
            let mut hi = px + g.below((px / 40).max(1) as u64 + 1) as i64;
            let lo = px;
            if style == 3 && g.below(150) == 0 {
                hi = px * (3 + g.below(40) as i64) / 2;
            }
            let vol = 1 + g.below(300);
            v.push(SecIn { ts: t(s), high: hi, low: lo, vol });
            if style == 4 && g.below(4) == 0 {
                let (h2, l2) = (hi + g.below(30) as i64 - 10, (lo - g.below(30) as i64).max(1));
                v.push(SecIn { ts: t(s), high: h2.max(l2), low: l2, vol: 1 + g.below(50) });
            }
            if style == 5 && g.below(6) == 0 {
                v.push(SecIn { ts: t(s), high: px * 3, low: px / 3 + 1, vol: 0 });
            }
        } else if style == 5 && g.below(3) == 0 {
            v.push(SecIn { ts: t(s), high: px * 2, low: (px / 2).max(1), vol: 0 });
        }
        s += 1;
    }
    if v.is_empty() {
        v.push(SecIn { ts: t(0), high: px0, low: px0, vol: 1 });
    }
    v
}

/// 1-4 filling blocks from a random traded second, each priced at its block's lowest LOW (or near the price), units 1-50.
fn rand_fills(g: &mut Rng, rows: &[SecIn], px0: i64) -> Vec<BlockFill> {
    let traded: Vec<&SecIn> = rows.iter().filter(|r| r.vol > 0).collect();
    let r0 = traded[g.below(traded.len() as u64) as usize];
    let mut at = (r0.ts - BASE) / US_S;
    let nb = 1 + g.below(4);
    let mut out = Vec::new();
    for _ in 0..nb {
        let lo = rows.iter().filter(|r| r.vol > 0 && r.ts >= t(at) && r.ts < t(at + 2)).map(|r| r.low).min().unwrap_or(px0);
        out.push(fill(at, 1 + g.below(50), lo));
        at += 2 + 2 * g.below(5) as i64;
    }
    out
}

fn rand_rule(g: &mut Rng, px: i64) -> XRule {
    let sc = (px / 3).max(2) as u64;
    let x = |g: &mut Rng| g.below(sc) as i64;
    match g.below(11) {
        0 => XRule::Stop { x: x(g) },
        1 => XRule::QuickCut { x: x(g) },
        2 => XRule::Target { x: x(g) },
        3 => XRule::TimeExit { secs: g.below(3_000) as u32 },
        4 => XRule::GoPositive { secs: g.below(600) as u32 },
        5 => XRule::Trail { act: x(g), dist: x(g) - 3 },
        6 => XRule::StopAt { level: px + x(g) - (sc as i64) / 2 },
        7 => XRule::TargetAt { level: px - x(g) + (sc as i64) / 3 },
        8 => XRule::TrailPct { act: x(g), bp: g.below(10_001) as u32 },
        9 => XRule::ArmedStop { act: x(g), off: x(g) - (sc as i64) / 2 },
        _ => XRule::TimedStop { after: g.below(1_500) as u32, off: x(g) - (sc as i64) / 2 },
    }
}

fn to_v21(r: XRule) -> Option<Rule> {
    match r {
        XRule::Stop { x } => Some(Rule::Stop { x }),
        XRule::QuickCut { x } => Some(Rule::QuickCut { x }),
        XRule::Target { x } => Some(Rule::Target { x }),
        XRule::TimeExit { secs } => Some(Rule::TimeExit { secs }),
        XRule::GoPositive { secs } => Some(Rule::GoPositive { secs }),
        XRule::Trail { act, dist } => Some(Rule::Trail { act, dist }),
        _ => None,
    }
}

/// Runs `cases` random (tape, entry, rule, side, window) queries: fast == law on every rule; fast == crate v2.1 `scan_one` and law ==
/// `scan_one` on every v2.1 rule. Returns (queries, v2.1 queries).
fn random_equality(seed: u64, tapes: usize, per: usize) -> (usize, usize) {
    let mut g = Rng(seed);
    let (mut nq, mut nv) = (0_usize, 0_usize);
    for k in 0..tapes {
        let px0 = [10_000_i64, 5, 37, 250, 100_000][k % 5];
        let n = 300 + g.below(2_500) as i64;
        let rows = rand_day(&mut g, n, (k as u64) % 6, px0);
        let d = day(rows);
        let tp = tape(&d);
        let (ts, tl) = (TrailIndex::new(&tp, false), TrailIndex::new(&tp, true));
        for _ in 0..per {
            let f = rand_fills(&mut g, &d.rows, px0);
            let long = g.below(2) == 0;
            let e = ev(&f);
            let to = e.e0 + (1 + g.below(n as u64 + 200) as i64) * US_S;
            let rules: Vec<XRule> = (0..6).map(|_| rand_rule(&mut g, px0)).collect();
            let mut st = 0_u32;
            let fast = scan_fast(&tp, if long { &tl } else { &ts }, &e, to, long, &rules, &mut st);
            let law = scan_law(&tp, &e, to, long, &rules);
            assert_eq!(fast, law, "seed {seed} tape {k}: fast vs law, rules {rules:?}, long {long}");
            nq += rules.len();
            let old: Vec<(usize, Rule)> = rules.iter().enumerate().filter_map(|(i, r)| to_v21(*r).map(|x| (i, x))).collect();
            if !old.is_empty() {
                let rs: Vec<Rule> = old.iter().map(|x| x.1).collect();
                let mut s2 = 0_u32;
                let (one, _) = path::scan_one(&tp, &e, to, long, &rs, &[], &mut s2);
                for (j, (i, _)) in old.iter().enumerate() {
                    assert_eq!(fast[*i], one[j], "seed {seed} tape {k}: fast vs crate v2.1 scan_one on {:?}", rules[*i]);
                    assert_eq!(law[*i], one[j], "seed {seed} tape {k}: law vs crate v2.1 scan_one on {:?}", rules[*i]);
                }
                nv += old.len();
            }
        }
    }
    (nq, nv)
}

#[test]
fn sf3_fast_scan_equals_scan_one() {
    let (nq, nv) = random_equality(2_026_111_501, 600, 25);
    assert!(nq > 80_000 && nv > 40_000, "queries {nq}, v2.1 queries {nv}");
}

#[test]
fn sf3_trailindex_equals_trail_hit() {
    // property over 10,000 seeded tapes: every act / dist of a small grid (and every bp of the relative trail) from random entries
    let mut g = Rng(4_242);
    let mut n = 0_usize;
    for k in 0..10_000_usize {
        let px0 = [10_000_i64, 5, 120, 900][k % 4];
        let nn = 120 + g.below(400) as i64;
        let rows = rand_day(&mut g, nn, (k as u64) % 6, px0);
        let d = day(rows);
        let tp = tape(&d);
        let long = k % 3 == 0;
        let ti = TrailIndex::new(&tp, long);
        let f = rand_fills(&mut g, &d.rows, px0);
        let e = ev(&f);
        let to = t(700);
        let sc = (px0 / 4).max(2);
        let mut old = Vec::new();
        let mut new = Vec::new();
        for act in [0, 1, sc / 3, sc] {
            for dist in [0, 1, 2, sc / 5, sc / 2, sc] {
                old.push(Rule::Trail { act, dist });
                new.push(XRule::Trail { act, dist });
            }
            for bp in [0_u32, 1, 100, 1_000, 2_500, 5_000, 10_000] {
                new.push(XRule::TrailPct { act, bp });
            }
        }
        let mut st = 0_u32;
        let fast = scan_fast(&tp, &ti, &e, to, long, &new, &mut st);
        let law = scan_law(&tp, &e, to, long, &new);
        assert_eq!(fast, law, "tape {k}");
        let mut s2 = 0_u32;
        let (one, _) = path::scan_one(&tp, &e, to, long, &old, &[], &mut s2);
        let fast_old: Vec<Option<Trigger>> = new.iter().zip(fast.iter()).filter(|(r, _)| matches!(r, XRule::Trail { .. })).map(|(_, x)| *x).collect();
        assert_eq!(fast_old, one, "tape {k}: TrailIndex vs crate v2.1 trail_hit");
        n += new.len();
    }
    assert!(n >= 10_000 * 52);
}

#[test]
fn sf3_fast_equals_law_every_rule_more_seeds() {
    let (nq, _) = random_equality(7, 400, 20);
    assert!(nq > 40_000);
}

// ---------------------------------------------------------------------------------------------- node reference vectors (G0 extended)

/// `vectors/XRULE-VECTORS-v22.tsv` (written by the node reference `vectors/xrule-reference.mjs`, a plain walk per merged traded second
/// sharing no code with Rust): every vector's expected trigger second equals the fast path's and the law's.
#[test]
fn sf3_node_reference_vectors() {
    let txt = include_str!("../vectors/XRULE-VECTORS-v22.tsv");
    let mut tapes: Vec<Day> = Vec::new();
    let mut n = 0_usize;
    for line in txt.lines() {
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        let f: Vec<&str> = line.split('\t').collect();
        match f[0] {
            "T" => {
                // T <rows: s,h,l,v;...>
                let rows: Vec<SecIn> = f[1].split(';').filter(|x| !x.is_empty()).map(|x| { let p: Vec<i64> = x.split(',').map(|y| y.parse().unwrap()).collect(); SecIn { ts: t(p[0]), high: p[1], low: p[2], vol: p[3] as u64 } }).collect();
                tapes.push(day(rows));
            }
            "Q" => {
                // Q tape long to_s fills(at,units,px;..) kind p1 p2 want_sec
                let d = &tapes[f[1].parse::<usize>().unwrap()];
                let long = f[2] == "1";
                let to_s: i64 = f[3].parse().unwrap();
                let fills: Vec<BlockFill> = f[4].split(';').filter(|x| !x.is_empty()).map(|x| { let p: Vec<i64> = x.split(',').map(|y| y.parse().unwrap()).collect(); fill(p[0], p[1] as u64, p[2]) }).collect();
                let (p1, p2): (i64, i64) = (f[6].parse().unwrap(), f[7].parse().unwrap());
                let rule = match f[5] {
                    "stop" => XRule::Stop { x: p1 },
                    "target" => XRule::Target { x: p1 },
                    "time" => XRule::TimeExit { secs: p1 as u32 },
                    "goPos" => XRule::GoPositive { secs: p1 as u32 },
                    "trail" => XRule::Trail { act: p1, dist: p2 },
                    "stopAt" => XRule::StopAt { level: p1 },
                    "targetAt" => XRule::TargetAt { level: p1 },
                    "trailPct" => XRule::TrailPct { act: p1, bp: p2 as u32 },
                    "armed" => XRule::ArmedStop { act: p1, off: p2 },
                    "timed" => XRule::TimedStop { after: p1 as u32, off: p2 },
                    o => panic!("unknown kind {o}"),
                };
                let want: i64 = f[8].parse().unwrap();
                assert_eq!(sec_of(d, &fills, rule, to_s, long), want, "vector {line}");
                n += 1;
            }
            o => panic!("unknown vector row {o}"),
        }
    }
    assert!(n >= 5_000, "vectors {n}");
}

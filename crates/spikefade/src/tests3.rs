//! v3 host tests (EXITS-DYNAMIC-DESIGN 6.1 / 6.3, red first): the first-hit order and its tie labels, the 15:08 cut with B2 (a held anchor
//! cancels the sale's unfilled rest and sends no order), the gap-through fills (a stop and a target pay the next block's worst, never a
//! level), the trailing take-profit replacing the target's close, exact event skipping, exact setting dedup, the day-major mode. SYNTHETIC.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic, clippy::too_many_lines, clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::cast_possible_wrap)]

use common_fill::xrule::XRule;

use crate::acct::SEv;
use crate::cal::{CUT_S, SQ_S, at_sod, days_from_civil};
use crate::cube::Sec;
use crate::kern::{CSet, dedup_groups, finish, fnv, list_keys, walk_account, walk_day};
use crate::menu::{Menu, rule_at};
use crate::stage::{ABSENT, EvX, NEVER3, NR, Tab3, XKey, first_hit, key_of, trade3_once, trip3};
use crate::trade::{BASE_S, DayTape, EvT, NL, Out, Refuse, dial, record_base};

fn day() -> i64 {
    days_from_civil(2026, 11, 4)
}

fn sec(s: i64, h: i32, l: i32, v: u32) -> Sec {
    Sec { sod: u32::try_from(s).unwrap_or(0), high: h, low: l, vol: v }
}

fn off(s: i64) -> u16 {
    u16::try_from(s - BASE_S).unwrap_or(0)
}

#[test]
fn sf3_first_hit_min_anchor_and_tie_order() {
    // the earliest anchor wins; on a tie the FIRST rule in tie order (SL before TP) takes the label; NEVER / ABSENT never win
    assert_eq!(first_hit(&[100, 50, 70, NEVER3]), Some((1, 50)));
    assert_eq!(first_hit(&[50, 50]), Some((0, 50)), "a same-second SL and TP is labelled SL");
    assert_eq!(first_hit(&[ABSENT, NEVER3, 80]), Some((2, 80)));
    assert_eq!(first_hit(&[NEVER3, ABSENT]), None);
}

#[test]
fn sf3_trail_fires_inside_1508_1510_is_held() {
    // an anchor at 15:07:59 sends the own order; 15:08:00 and later (to 15:10:00) is HELD: no own order (S14), the sale cut there (B2)
    assert_eq!(key_of(off(CUT_S - 1), 5, false), XKey::Own(CUT_S - 1, 5));
    assert_eq!(key_of(off(CUT_S), 5, false), XKey::Held(CUT_S));
    assert_eq!(key_of(off(CUT_S + 30), 5, false), XKey::Held(CUT_S + 30));
    assert_eq!(key_of(off(SQ_S), 5, false), XKey::None, "at or after 15:10:00: the square-off itself");
    // the v2 compatibility cut (P0): every anchor at or after 15:08 is the square-off
    assert_eq!(key_of(off(CUT_S + 30), 5, true), XKey::None);
    assert_eq!(key_of(NEVER3, 0, false), XKey::None);
}

#[test]
fn sf3_held_anchor_cancels_unfilled_sale() {
    let d = day();
    let t = CUT_S - 120; // T = 15:06:00
    // a slow sale: 50 units (1 lot) per 2-second block from 15:06:00 (100 units printed each second at 100%: 2 lots a block... use 30 a second)
    let mut rows = vec![sec(t - 1, 10_000, 10_000, 1_000)];
    for s in 0..600 {
        rows.push(sec(t + s, 10_000, 10_000, 30));
    }
    let dt = DayTape::new(d, &rows, 2).unwrap();
    let tape = dt.tape().unwrap();
    let dl = dial().unwrap();
    let qty = 50 * 200; // 200 lots of 50: never filled before 15:10 at 60 units a block
    let mut p = record_base(&tape, d, t, 50, qty, &dl).ok().unwrap();
    let none = trip3(&mut p, &tape, d, 50, 4, qty, XKey::None, &dl).unwrap();
    let held = trip3(&mut p, &tape, d, 50, 4, qty, XKey::Held(CUT_S + 30), &dl).unwrap();
    assert!(held.units < none.units, "the held anchor cancels the sale's rest at 15:08:30 ({} < {})", held.units, none.units);
    assert_eq!(held.bb_units, 0, "no own order from 15:08");
    assert_eq!(held.sq_units + held.unclosed, held.units, "every filled unit waits for the 15:10 square-off");
    assert!(held.sale_known <= at_sod(d, CUT_S + 32));
}

#[test]
fn sf3_gapped_stop_pays_block_worst() {
    let d = day();
    let t = 36_000_i64;
    let mut rows = vec![sec(t - 1, 10_000, 10_000, 1_000)];
    for s in 0..10 {
        rows.push(sec(t + s, 10_000, 10_000, 1_000));
    }
    rows.push(sec(t + 10, 20_000, 19_000, 1_000));
    rows.push(sec(t + 11, 21_000, 20_000, 1_000));
    rows.push(sec(t + 12, 21_000, 20_500, 1_000));
    let dt = DayTape::new(d, &rows, 2).unwrap();
    let tape = dt.tape().unwrap();
    let dl = dial().unwrap();
    // SL 20% of the premium (level 12,000), one lot of 50 at share 100%
    let rules = vec![Ok((XRule::Stop { x: 2_000 }, false))];
    let (r, pos, anc, key, tsec, level) = trade3_once(&tape, d, u32::try_from(t).unwrap(), 50, 50, 4, &rules, false, &dl).unwrap().unwrap();
    assert_eq!((pos, tsec, level), (Some(0), t + 10, 12_000));
    assert_eq!(key, XKey::Own(t + 11, 12_000));
    assert_eq!(anc, off(t + 11));
    assert_eq!(r.bb_first_px, 21_000, "the buy-back pays the block's highest HIGH, never the stop level Rs 120");
    assert!(r.net < 0);
}

#[test]
fn sf3_gapped_target_buys_back_at_block_worst() {
    let d = day();
    let t = 36_000_i64;
    // the premium collapses through the 30% target (level 7,000) at 10:00:10 (LOW 5,000) and rebounds: the buy-back from 10:00:11 pays that
    // block's highest HIGH 8,000, ABOVE the target level
    let mut rows = vec![sec(t - 1, 10_000, 10_000, 1_000)];
    for s in 0..10 {
        rows.push(sec(t + s, 10_000, 10_000, 1_000));
    }
    rows.push(sec(t + 10, 9_000, 5_000, 1_000));
    rows.push(sec(t + 11, 8_000, 7_500, 1_000));
    rows.push(sec(t + 12, 7_900, 7_600, 1_000));
    let dt = DayTape::new(d, &rows, 2).unwrap();
    let tape = dt.tape().unwrap();
    let dl = dial().unwrap();
    let rules = vec![Ok((XRule::Stop { x: 5_000 }, false)), Ok((XRule::Target { x: 3_000 }, false))];
    let (r, pos, _, key, tsec, level) = trade3_once(&tape, d, u32::try_from(t).unwrap(), 50, 50, 4, &rules, false, &dl).unwrap().unwrap();
    assert_eq!((pos, tsec, level), (Some(1), t + 10, 7_000));
    assert!(matches!(key, XKey::Own(_, _)));
    assert_eq!(r.bb_first_px, 8_000, "the target's buy-back pays the next block's worst, above the level");
}

#[test]
fn sf3_ttp_replaces_tp_close() {
    let d = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("data");
    let menu = Menu::parse(&std::fs::read_to_string(d.join("EXITS-DYN-MENU-v1.tsv")).unwrap()).unwrap();
    let row = menu.rules.iter().find(|r| r.id == "TTP.P.50.5").cloned().unwrap();
    // with TTP, the 50% touch WAKES a trail (5% of p0 behind the running low); it does not close at the touch
    assert_eq!(rule_at(&row, &crate::menu::Rd::default(), 10_000), Ok((XRule::Trail { act: 5_000, dist: 500 }, false)));
    let rel = menu.rules.iter().find(|r| r.id == "TTP.R.50.25").cloned().unwrap();
    assert_eq!(rule_at(&rel, &crate::menu::Rd::default(), 10_000), Ok((XRule::TrailPct { act: 5_000, bp: 2_500 }, false)));
}

/// Synthetic events for the kernel: `n` events per day on `days` days, each on its own contract, one lot of margin `m`, each event's single
/// rule (menu row 0, the SL) firing per `fire` with out (net, exit after `dur` s).
fn kernel_fixture(days: u8, per_day: u32, dur: &dyn Fn(u8, u32) -> u32, net: &dyn Fn(u8, u32) -> i32) -> (Vec<SEv>, Vec<EvX>, Tab3) {
    let mut evs = Vec::new();
    let mut ex = Vec::new();
    let mut tab = Tab3::default();
    for d in 0..days {
        for k in 0..per_day {
            let t = 36_000 + k * 60;
            let et = EvT { refuse: Refuse::None, lots: [1; 4], q_of: [0; 4], qs: vec![1], margin_lot: 1_000_000, filled: [true; NL], ..EvT::default() };
            let x = EvX { r0: u32::try_from(tab.anc.len()).unwrap(), t0: 0, nq: 1, tf: Vec::new(), fill: Vec::new(), mark1: 0 };
            for l in 0..NL {
                let mut a = [ABSENT; NR];
                // rule 0 (SL) fires at T + 10 (out slot 1); rule 1 (a TP) fires at T + 5 on odd events (out slot 2), else never
                a[0] = off(i64::from(t) + 10);
                a[1] = if k % 2 == 1 { off(i64::from(t) + 5) } else { NEVER3 };
                let mut o = [0_u8; NR];
                o[0] = 1;
                o[1] = 2;
                tab.anc.push(a);
                tab.os.push(o);
                tab.obase.push(u32::try_from(tab.outs.len()).unwrap());
                let nt = net(d, k) + i32::try_from(l).unwrap();
                let ex0 = u16::try_from(i64::from(t + dur(d, k)) - BASE_S).unwrap();
                tab.outs.push(Out { net: 0, exit: ex0, flag: 0, ..Out::default() });
                tab.outs.push(Out { net: nt, exit: ex0, flag: 1, ..Out::default() });
                tab.outs.push(Out { net: nt / 2, exit: u16::try_from(i64::from(t + dur(d, k) / 2) - BASE_S).unwrap(), flag: 1, ..Out::default() });
            }
            evs.push(SEv { di: u16::from(d), ci: k + 1_000 * u32::from(d), t, skey: k + 1_000 * u32::from(d), tab: et });
            ex.push(x);
        }
    }
    (evs, ex, tab)
}

#[test]
fn sf3_event_skip_exact() {
    // positions last exactly 120 s: the event at T + 120 s lands ON the exit second (the slot frees there and the event books)
    let (evs, ex, tab) = kernel_fixture(3, 40, &|_, k| if k % 3 == 0 { 120 } else { 300 + (k % 7) * 60 }, &|d, k| i32::try_from((k * 37 + u32::from(d) * 11) % 200).unwrap() * 1_000 - 90_000);
    let list: Vec<u32> = (0..u32::try_from(evs.len()).unwrap()).collect();
    let keys = list_keys(&evs, &list);
    let sets = [CSet { rules: vec![(false, 0)], sl: 0 }, CSet { rules: vec![(false, 0), (false, 1)], sl: 0 }];
    let mut fewer = 0;
    for cs in &sets {
        for l in 0..NL {
            for ni in 0..4 {
                let (a, _, sa) = walk_account(&evs, &ex, &tab, &list, &keys, cs, l, ni, None, true, true, &[], 0, 3);
                let (b, _, sb) = walk_account(&evs, &ex, &tab, &list, &keys, cs, l, ni, None, true, false, &[], 0, 3);
                assert_eq!((a.net, a.trades, a.wins, a.mdd, a.worst, a.worst_day, a.days_pos, a.days_neg, a.pk_margin), (b.net, b.trades, b.wins, b.mdd, b.worst, b.worst_day, b.days_pos, b.days_neg, b.pk_margin), "skip vs plain: setting {cs:?} l {l} N {ni}");
                assert!(sa <= sb);
                if sa < sb {
                    fewer += 1;
                }
            }
        }
    }
    assert!(fewer > 0, "skipping must actually skip somewhere (N 5 slots fill up)");
}

#[test]
fn sf3_setting_dedup_exact() {
    let (evs, ex, tab) = kernel_fixture(2, 30, &|_, k| 60 + (k % 5) * 90, &|_, k| i32::try_from(k % 9).unwrap() * 3_000 - 12_000);
    let list: Vec<u32> = (0..u32::try_from(evs.len()).unwrap()).collect();
    let keys = list_keys(&evs, &list);
    // four settings: SL alone, SL + TP, the SL alone again (identical outcomes), SL + TP again
    let sets = vec![CSet { rules: vec![(false, 0)], sl: 0 }, CSet { rules: vec![(false, 0), (false, 1)], sl: 0 }, CSet { rules: vec![(false, 0), (false, 2)], sl: 0 }, CSet { rules: vec![(false, 0), (false, 1), (false, 3)], sl: 0 }];
    // a hash that collides EVERYTHING: only the full compare keeps different settings apart
    let collide: fn(&[u32]) -> u64 = |_| 7;
    for hash in [fnv as fn(&[u32]) -> u64, collide] {
        let rep = dedup_groups(&evs, &ex, &tab, &list, &sets, 0, 0, None, hash);
        assert_eq!(rep, vec![0, 1, 0, 1], "rule 2 / rule 3 never exist (ABSENT): the SL alone and SL + TP again");
        for (k, cs) in sets.iter().enumerate() {
            let (a, _, _) = walk_account(&evs, &ex, &tab, &list, &keys, cs, 0, 0, None, true, true, &[], 0, 2);
            let (b, _, _) = walk_account(&evs, &ex, &tab, &list, &keys, &sets[rep[k]], 0, 0, None, true, true, &[], 0, 2);
            assert_eq!((a.net, a.trades, a.mdd), (b.net, b.trades, b.mdd));
        }
    }
}

#[test]
fn sf3_day_major_equals_whole() {
    let (evs, ex, tab) = kernel_fixture(4, 50, &|d, k| 30 + ((k + u32::from(d)) % 11) * 45, &|d, k| i32::try_from((k * 53 + u32::from(d) * 7) % 300).unwrap() * 700 - 100_000);
    let list: Vec<u32> = (0..u32::try_from(evs.len()).unwrap()).collect();
    let keys = list_keys(&evs, &list);
    let cs = CSet { rules: vec![(false, 0), (false, 1)], sl: 0 };
    for l in 0..NL {
        for ni in 0..4 {
            let (w, _, _) = walk_account(&evs, &ex, &tab, &list, &keys, &cs, l, ni, None, true, true, &[], 0, 4);
            let mut ac = crate::kern::Acc::new();
            let mut pc = crate::kern::Pieces::default();
            for d in 0..4_u32 {
                let (lo, hi) = (d * 86_400, (d + 1) * 86_400);
                let (a, b) = (keys.partition_point(|&k| k < lo), keys.partition_point(|&k| k < hi));
                let _ = walk_day(&evs, &ex, &tab, &list[a..b], &keys[a..b], &cs, l, ni, None, true, true, &mut ac, &mut pc, &[]);
            }
            let s = finish(&mut ac, ni, 4);
            assert_eq!((w.net, w.trades, w.wins, w.mdd, w.worst, w.worst_day, w.days_pos, w.days_neg, w.days_traded, w.pk_margin), (s.net, s.trades, s.wins, s.mdd, s.worst, s.worst_day, s.days_pos, s.days_neg, s.days_traded, s.pk_margin), "l {l} N {ni}");
        }
    }
}

#[test]
fn sf3_chunk_kernel_equals_walk_account() {
    let (evs, ex, tab) = kernel_fixture(3, 45, &|d, k| 40 + ((k * 7 + u32::from(d)) % 13) * 50, &|d, k| i32::try_from((k * 31 + u32::from(d) * 5) % 250).unwrap() * 900 - 100_000);
    let list: Vec<u32> = (0..u32::try_from(evs.len()).unwrap()).collect();
    let keys = list_keys(&evs, &list);
    let sets = vec![CSet { rules: vec![(false, 0)], sl: 0 }, CSet { rules: vec![(false, 0), (false, 1)], sl: 0 }];
    for skip in [true, false] {
        let (rows, _) = crate::kern::walk_chunk(&evs, &ex, &tab, &list, &keys, &sets, None, true, skip, &[], 0, 3);
        for (a, (s, _)) in rows.iter().enumerate() {
            let (si, l, ni) = (a / (NL * 4), (a % (NL * 4)) / 4, a % 4);
            let (w, _, _) = walk_account(&evs, &ex, &tab, &list, &keys, &sets[si], l, ni, None, true, false, &[], 0, 3);
            assert_eq!((s.net, s.trades, s.wins, s.mdd, s.worst, s.worst_day, s.days_pos, s.days_neg, s.days_traded, s.pk_margin), (w.net, w.trades, w.wins, w.mdd, w.worst, w.worst_day, w.days_pos, w.days_neg, w.days_traded, w.pk_margin), "account {a} skip {skip}");
        }
    }
}

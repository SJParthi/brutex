//! SWEEP-EQ against `common_wf`'s OWN `Acc` (plan 8.3; linked since common_wf v1's SHA256SUMS, 2026-11-13 17:10:25 IST): for
//! random trade sets inserted and removed in ANY order, every key part of `SweepAcc` (K5 TRADE, K7, K6, K8, K9, K10, K4) equals
//! `common_wf::views::crit_val` on `Acc::learn` fed the same trades in time order, and E1 + E2 agree, for WF and TT, on the
//! standard WINDOWS / VIEWS files of common_wf v1.

use std::collections::BTreeSet;

use common_wf::views::{Base, Crit, Views, crit_val};
use common_wf::windows::{Windows, parse_windows};
use common_wf::{Acc, CalRow, Calendar, DayKind, RunRules, Trade, WinId};

use crate::acc::{Layout, Outcome, SweepAcc};
use crate::{MAXC, mix};

const RULES: &str = "period=2018-09..2026-10&signed=2026-11&paper_from=2026-12&order=SEQ_LOSSES_FIRST&holes=lane&grid=1000:SEARCH:LATTICE&proof=proof step: test&data=data: complete&run=1";
const WF_DATA: &str = "/Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/scratch/sniper-host-v1/pinned/common-wf-v1/data";

fn kv(s: &str) -> Vec<(String, String)> {
    s.split('&').filter_map(|x| x.split_once('=')).map(|(a, b)| (a.to_string(), b.to_string())).collect()
}

#[test]
fn sweep_eq_against_common_wf_acc() {
    let r = RunRules::from_rows(&kv(RULES)).unwrap();
    let d0 = common_wf::date::parse_day("2018-09-01").unwrap();
    let d1 = common_wf::date::parse_day("2026-10-31").unwrap();
    let days: Vec<i32> = (d0..=d1).filter(|z| (1..=5).contains(&((z % 7 + 7 + 4) % 7))).collect();
    let cal = Calendar::from_rows(&days.iter().map(|&z| CalRow { day: z, kind: DayKind::Regular, walked: true, ruling: false }).collect::<Vec<_>>()).unwrap();
    let wrows = parse_windows(&std::fs::read_to_string(format!("{WF_DATA}/WINDOWS-STD-v1.tsv")).unwrap()).unwrap();
    let views = Views::from_text(&std::fs::read_to_string(format!("{WF_DATA}/VIEWS-STD-v1.tsv")).unwrap(), &wrows, &r, &[]).unwrap();
    let ws = Windows::sets_from_rows(&wrows, &cal, r.period).unwrap();
    let set = ws.sets.iter().find(|w| w.has(WinId(0)) && w.has(WinId(1))).unwrap();
    let mk = |z: i32| common_wf::date::mk_of_day(z);
    let hull = (mk(common_wf::date::parse_day("2018-09-03").unwrap()), mk(common_wf::date::parse_day("2022-12-30").unwrap()));
    let sess_mk: Vec<i32> = days.iter().map(|&z| mk(z)).collect();
    let lay = Layout::new(&sess_mk, hull, &[("WF".to_string(), hull.0, hull.0 + 15), ("TT".to_string(), hull.0, hull.1)]).unwrap();
    let crits = [Crit::K5(Base::Trade, None), Crit::K7, Crit::K6, Crit::K8, Crit::K9, Crit::K10, Crit::K4];
    let learn_sess: Vec<usize> = (0..days.len()).filter(|&i| sess_mk[i] >= hull.0 && sess_mk[i] <= hull.1).collect();
    let mut compared = 0;
    for seed in 1..=40_u64 {
        let mut st = seed * 0x9e37;
        let mut next = || {
            st = mix(st.wrapping_add(0x9e37_79b9_7f4a_7c15));
            st
        };
        // A sniper-shaped subject: a few trades (some seeds dense), all-positive in some seeds (K7 = 0, K9 tier).
        let n_tr = if seed % 3 == 0 { 400 } else { 2 + usize::try_from(next() % 12).unwrap() };
        let mut chosen: BTreeSet<usize> = BTreeSet::new();
        while chosen.len() < n_tr {
            chosen.insert(learn_sess[usize::try_from(next() % learn_sess.len() as u64).unwrap()]);
        }
        let positive_only = seed % 4 == 1;
        let out: Vec<Outcome> = (0..days.len())
            .map(|i| {
                let x = i64::try_from(next() % 40_001).unwrap() - if positive_only { -1 } else { 15_000 };
                let mut net = [0_i64; MAXC];
                net[0] = x * 10;
                let mut rd = [0_u16; MAXC];
                rd[0] = u16::try_from(next() % 900).unwrap();
                Outcome { sess: u32::try_from(i).unwrap(), mk: sess_mk[i], id: 7 + i as u64, net, rd_bp: rd }
            })
            .collect();
        // SweepAcc: inserted in a scrambled order, with removals and re-inserts.
        let mut sw = SweepAcc::new(&lay, &out, 1, days.len()).unwrap();
        let mut order: Vec<usize> = chosen.iter().copied().collect();
        order.sort_by_key(|&i| mix(i as u64 ^ seed));
        for &i in &order {
            sw.insert(u32::try_from(i).unwrap()).unwrap();
        }
        for &i in order.iter().take(order.len() / 2) {
            sw.remove(u32::try_from(i).unwrap()).unwrap();
        }
        for &i in order.iter().take(order.len() / 2).rev() {
            sw.insert(u32::try_from(i).unwrap()).unwrap();
        }
        // common_wf's Acc in time order.
        let mut acc = Acc::new(set, &views, &r);
        acc.reset();
        for &i in &chosen {
            let o = out[i];
            acc.add(&Trade { day: days[i], exit_day: days[i], close_s: 40_000, net: o.net[0], net_alt: 0, rd_bp: i32::from(o.rd_bp[0]), hole: false, id: o.id, mark_prev: 0 }).unwrap();
        }
        acc.finish().unwrap();
        for w in 0..2_usize {
            let l = acc.learn(WinId(u8::try_from(w).unwrap())).unwrap();
            let wf_elig = l.cell.trades > 0 && (l.booked & l.units) == l.units;
            let (k, _, _) = sw.key(w, 0, 1).unwrap();
            assert_eq!(k.is_ok(), wf_elig, "seed {seed} window {w}: E1 + E2 differ");
            if let Ok(key) = k {
                let vals: Vec<i64> = crits.iter().map(|c| crit_val(c, &l, &acc, 200_000_000, false, &[]).unwrap()).collect();
                assert_eq!(key.0[..vals.len()].to_vec(), vals, "seed {seed} window {w}: key parts K5 K7 K6 K8 K9 K10 K4");
                compared += 1;
            }
        }
    }
    assert!(compared >= 10, "only {compared} eligible keys compared");
}

/// SWEEP-EQ for the BESIDE `-N` and the `-MISS` views (P17): `SweepAcc::view_key` against `common_wf::views::crit_val` on `Acc`
/// fed the same trades in time order: -N = (K4, K6, K5) on E1 + E2 cells; -MISS = (K1, K16, K17, K5, K7, K6, K8, K9, K10, K4) on
/// E1 cells failing E2 (E7). Mutant `m_k17` (no +1 paisa for an empty learning unit) fails it.
#[test]
fn sweep_eq_miss_and_n_against_common_wf() {
    use crate::acc::{KeyKind, NotEligible};
    let r = RunRules::from_rows(&kv(RULES)).unwrap();
    let d0 = common_wf::date::parse_day("2018-09-01").unwrap();
    let d1 = common_wf::date::parse_day("2026-10-31").unwrap();
    let days: Vec<i32> = (d0..=d1).filter(|z| (1..=5).contains(&((z % 7 + 7 + 4) % 7))).collect();
    let cal = Calendar::from_rows(&days.iter().map(|&z| CalRow { day: z, kind: DayKind::Regular, walked: true, ruling: false }).collect::<Vec<_>>()).unwrap();
    let wrows = parse_windows(&std::fs::read_to_string(format!("{WF_DATA}/WINDOWS-STD-v1.tsv")).unwrap()).unwrap();
    let views = Views::from_text(&std::fs::read_to_string(format!("{WF_DATA}/VIEWS-STD-v1.tsv")).unwrap(), &wrows, &r, &[]).unwrap();
    let ws = Windows::sets_from_rows(&wrows, &cal, r.period).unwrap();
    let set = ws.sets.iter().find(|w| w.has(WinId(0)) && w.has(WinId(1))).unwrap();
    let mk = |z: i32| common_wf::date::mk_of_day(z);
    let hull = (mk(common_wf::date::parse_day("2018-09-03").unwrap()), mk(common_wf::date::parse_day("2022-12-30").unwrap()));
    let sess_mk: Vec<i32> = days.iter().map(|&z| mk(z)).collect();
    let lay = Layout::new(&sess_mk, hull, &[("WF".to_string(), hull.0, hull.0 + 15), ("TT".to_string(), hull.0, hull.1)]).unwrap();
    let miss = [Crit::K1, Crit::K16, Crit::K17, Crit::K5(Base::Trade, None), Crit::K7, Crit::K6, Crit::K8, Crit::K9, Crit::K10, Crit::K4];
    let n_crits = [Crit::K4, Crit::K6, Crit::K5(Base::Trade, None)];
    let learn_sess: Vec<usize> = (0..days.len()).filter(|&i| sess_mk[i] >= hull.0 && sess_mk[i] <= hull.1).collect();
    let (mut n_miss, mut n_n) = (0, 0);
    for seed in 1..=60_u64 {
        let mut st = seed * 0x51ed;
        let mut next = || {
            st = mix(st.wrapping_add(0x9e37_79b9_7f4a_7c15));
            st
        };
        // Sparse subjects (whole empty learning years: K16 > 0) and dense ones; signs mixed so many cells miss E2.
        let n_tr = if seed % 5 == 0 { 300 } else { 1 + usize::try_from(next() % 6).unwrap() };
        let mut chosen: BTreeSet<usize> = BTreeSet::new();
        while chosen.len() < n_tr {
            chosen.insert(learn_sess[usize::try_from(next() % learn_sess.len() as u64).unwrap()]);
        }
        let out: Vec<Outcome> = (0..days.len())
            .map(|i| {
                let x = i64::try_from(next() % 30_001).unwrap() - 14_000;
                let mut net = [0_i64; MAXC];
                net[0] = x * 10;
                let mut rd = [0_u16; MAXC];
                rd[0] = u16::try_from(next() % 700).unwrap();
                Outcome { sess: u32::try_from(i).unwrap(), mk: sess_mk[i], id: 11 + i as u64, net, rd_bp: rd }
            })
            .collect();
        let mut sw = SweepAcc::new(&lay, &out, 1, days.len()).unwrap();
        let mut order: Vec<usize> = chosen.iter().copied().collect();
        order.sort_by_key(|&i| mix(i as u64 ^ (seed << 7)));
        for &i in &order {
            sw.insert(u32::try_from(i).unwrap()).unwrap();
        }
        let mut acc = Acc::new(set, &views, &r);
        acc.reset();
        for &i in &chosen {
            let o = out[i];
            acc.add(&Trade { day: days[i], exit_day: days[i], close_s: 40_000, net: o.net[0], net_alt: 0, rd_bp: i32::from(o.rd_bp[0]), hole: false, id: o.id, mark_prev: 0 }).unwrap();
        }
        acc.finish().unwrap();
        for w in 0..2_usize {
            let l = acc.learn(WinId(u8::try_from(w).unwrap())).unwrap();
            let e1 = l.cell.trades > 0;
            let e2 = (l.booked & l.units) == l.units;
            let (km, _) = sw.view_key(w, 0, 1, KeyKind::Miss).unwrap();
            match km {
                Ok(k) => {
                    assert!(e1 && !e2, "seed {seed} window {w}: a MISS key for a cell common_wf reads E1 {e1} E2 {e2}");
                    let vals: Vec<i64> = miss.iter().map(|c| crit_val(c, &l, &acc, 200_000_000, false, &[]).unwrap()).collect();
                    assert_eq!(k.0.to_vec(), vals, "seed {seed} window {w}: MISS key parts K1 K16 K17 K5 K7 K6 K8 K9 K10 K4");
                    n_miss += 1;
                }
                Err(NotEligible::PassesE2) => assert!(e1 && e2, "seed {seed} window {w}: PassesE2"),
                Err(NotEligible::NoTrade) => assert!(!e1, "seed {seed} window {w}: NoTrade"),
                Err(e) => panic!("seed {seed} window {w}: MISS refused {e:?}"),
            }
            let (kn, _) = sw.view_key(w, 0, 1, KeyKind::N).unwrap();
            assert_eq!(kn.is_ok(), e1 && e2, "seed {seed} window {w}: -N eligibility");
            if let Ok(k) = kn {
                let vals: Vec<i64> = n_crits.iter().map(|c| crit_val(c, &l, &acc, 200_000_000, false, &[]).unwrap()).collect();
                assert_eq!(k.0[..3].to_vec(), vals, "seed {seed} window {w}: -N key parts K4 K6 K5");
                n_n += 1;
            }
        }
    }
    assert!(n_miss >= 20 && n_n >= 3, "too few compared: MISS {n_miss}, -N {n_n}");
}

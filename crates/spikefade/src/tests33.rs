//! v3.3 named tests (Addendum 28.2, job v33-fast): every exact speed switch of SPEED-PLAN S1 / S2 against round B's own code on synthetic
//! fixtures, field by field. Each test names the switch it guards; each named mutant (crates/spikefade/Cargo.toml features `m_v33_*`) must
//! make its test FAIL. The v3.3 switches are per test thread (`sw33`).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic, clippy::too_many_lines, clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::cast_possible_wrap)]

use crate::acct::SEv;
use crate::kern::{Acc, CSet, Pieces, Pos, list_keys, walk_day, walk_day_h};
use crate::report3::{Layout33, Seg3, append_cfg_lists, batch_order33, build_lists, build_lists_derived, day_items33, fp_v33, stage_a_chunks33};
use crate::stage::{ABSENT, EvX, NEVER3, NR, Tab3};
use crate::sw33;
use crate::trade::{BASE_S, EvT, NL, Out, Refuse};

fn reset33() {
    for s in 0..sw33::COUNT {
        sw33::set(s, false);
    }
}

fn off(s: i64) -> u16 {
    u16::try_from(s - BASE_S).unwrap()
}

/// The synthetic kernel fixture of tests3 / tests32r: per day `per_day` events a minute apart from 10:00 on their own contracts, one lot of
/// Rs 10,000 margin, rule 0 (SL) at T + 10 s (out slot 1), rule 1 (a TP) at T + 5 s on odd events (out slot 2), positions lasting
/// `dur(d, k)` seconds, net `net(d, k)`.
fn fixture(days: u8, per_day: u32, dur: &dyn Fn(u8, u32) -> u32, net: &dyn Fn(u8, u32) -> i32) -> (Vec<SEv>, Vec<EvX>, Tab3) {
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

// ------------------------------------------------------------------------------------------------ switches

/// v3.3: every v3.3 switch is OFF by default (proof P0: round B's code paths), `--v33` turns every one on, and each flag overrides.
#[test]
fn v33_switches_default_off_v33_all() {
    reset33();
    sw33::from_args(&["run3".to_owned()]).unwrap();
    assert!(!sw33::any(), "no flag: every v3.3 switch off");
    assert_eq!(sw33::words(), "", "no v3.3 words when every switch is off (RUN.log and the fingerprint stay round B's)");
    sw33::from_args(&["run3".to_owned(), "--v33".to_owned()]).unwrap();
    for s in 0..sw33::COUNT {
        assert!(sw33::on(s), "--v33 turns switch {s} on");
    }
    sw33::from_args(&["run3".to_owned(), "--v33".to_owned(), "--states".to_owned(), "full".to_owned()]).unwrap();
    assert!(!sw33::on(sw33::COMPACT) && sw33::on(sw33::DERIVED), "a flag overrides --v33 for its own switch only");
    assert!(sw33::from_args(&["run3".to_owned(), "--ul".to_owned(), "maybe".to_owned()]).is_err());
    reset33();
}

// ------------------------------------------------------------------------------------------------ C2 (NOFACTS)

/// C2: each day's per-config screener output is appended with the day's event offset, exactly as round B's one merge after all days
/// (`dst.push(off + ue)`); the facts are dropped. Mutant m_v33_c2_merge_offset (offset left out) must fail.
#[test]
fn v33_c2_append_cfg_lists_offsets() {
    let mut lists: Vec<Vec<u32>> = vec![Vec::new(); 3];
    // day 0: 4 events (offset 0); day 1: offset 4
    append_cfg_lists(&mut lists, vec![vec![(0, 10, 1), (2, 20, 1)], vec![], vec![(3, 5, 2)]], 0);
    append_cfg_lists(&mut lists, vec![vec![(1, 7, 1)], vec![(0, 9, 1), (2, 1, 1)], vec![]], 4);
    assert_eq!(lists, vec![vec![0, 2, 5], vec![4, 6], vec![3]], "round B's merge: off + ue per day, in day order");
}

// ------------------------------------------------------------------------------------------------ C1 (COMPACT)

/// C1: the compact layout maps every (setting, WALKED share level, N) of a list to a distinct state inside the list's `settings x walked
/// levels x 4` states, and the full layout is round B's `(setting x 5 + level) x 4 + N`. Mutant m_v33_c1_stride_full must fail.
#[test]
fn v33_c1_compact_layout_bijective() {
    let ns = 7;
    for mask in [0b00010_u8, 0b00011, 0b00100, 0b10101, 0b11111] {
        let lay = Layout33::with_mask(true, mask);
        let walked: Vec<usize> = (0..NL).filter(|&l| (mask >> l) & 1 == 1).collect();
        assert_eq!(lay.nlv, walked.len());
        let n = lay.per_list(ns);
        let mut seen = vec![false; n];
        for si in 0..ns {
            for &l in &walked {
                for ni in 0..4 {
                    let i = lay.idx(si, l, ni);
                    assert!(i < n, "mask {mask:05b}: ({si}, {l}, {ni}) -> {i} outside the list's {n} states");
                    assert!(!seen[i], "mask {mask:05b}: state {i} given twice");
                    seen[i] = true;
                }
            }
        }
        assert!(seen.iter().all(|&x| x), "every compact state belongs to exactly one walked account");
        let full = Layout33::with_mask(false, mask);
        for si in 0..ns {
            for l in 0..NL {
                for ni in 0..4 {
                    assert_eq!(full.idx(si, l, ni), (si * NL + l) * 4 + ni, "full layout = round B's");
                }
            }
        }
    }
}

// ------------------------------------------------------------------------------------------------ C3 (DERIVED)

/// A synthetic segment for the list builders: `nd` days of `per` events, liquidity readings that differ per event, and per-config lists
/// that coincide after some liquidity filters (merges through the FULL compare) and differ under others.
fn list_seg() -> Seg3 {
    let (nd, per) = (3_u16, 12_u32);
    let mut sr = Seg3::default();
    for d in 0..nd {
        for k in 0..per {
            sr.evs.push(SEv { di: d, ci: k, t: 36_000 + k * 30, skey: k, tab: EvT::default() });
            // traded seconds and the longest gap: every 3rd event thin, every 4th with a long gap
            sr.liq.push((if k % 3 == 0 { 20 } else { 58 }, if k % 4 == 0 { 12 } else { 1 }));
        }
    }
    let n = sr.evs.len() as u32;
    sr.lists = vec![Vec::new(); crate::screen::NCFG];
    // config 5: every event; config 9: every event but the thin ones (equal to config 5 under liquidity 1); config 17: the odd events;
    // config 40: the odd events again (equal to 17 under every value); config 41: every 4th event (empty under value 6)
    sr.lists[5] = (0..n).collect();
    sr.lists[9] = (0..n).filter(|&e| (e % 12) % 3 != 0).collect();
    sr.lists[17] = (0..n).filter(|&e| e % 2 == 1).collect();
    sr.lists[40] = (0..n).filter(|&e| e % 2 == 1).collect();
    sr.lists[41] = (0..n).filter(|&e| (e % 12) % 4 == 0).collect();
    sr
}

/// C3: the derived builder gives the same unique lists, numbering, pairs and removal counts as round B's `build_lists`, and every derived
/// list (whole and per day) equals the stored `ul_evs`. Mutant m_v33_c3_derived_wrong_rep must fail.
#[test]
fn v33_c3_derived_lists_equal() {
    let liqs: Vec<usize> = vec![0, 1, 6, 7];
    let mut a = list_seg();
    let mut b = list_seg();
    let mut log = String::new();
    build_lists(&mut a, &liqs, false, &mut log, "t");
    build_lists_derived(&mut b, &liqs, false, &mut log, "t");
    assert!(b.ul_derived && b.ul_evs.is_empty() && !a.ul_derived);
    assert_eq!(a.uls, b.uls, "same unique lists, first-seen numbering");
    assert_eq!(a.ul_cfgs, b.ul_cfgs, "same (config, liquidity) pairs per list");
    assert_eq!(a.removed, b.removed, "same removal counts");
    assert!(a.uls.len() >= 4 && a.ul_cfgs.iter().any(|c| c.len() >= 2), "the fixture merges pairs through the full compare");
    for u in 0..a.uls.len() {
        let want = a.ul_evs[u].clone();
        assert_eq!(b.ul(u).unwrap().to_vec(), want, "list {u}: derived = stored");
        assert_eq!(a.ul(u).unwrap().to_vec(), want, "list {u}: the stored accessor");
        let wk = list_keys(&a.evs, &want);
        for di in 0..4 {
            let (mut s1, mut k1, mut s2, mut k2) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
            a.ul_day(u, di, Some(&wk), &mut s1, &mut k1);
            b.ul_day(u, di, None, &mut s2, &mut k2);
            assert_eq!((s1, k1), (s2, k2), "list {u} day {di}: the day part derived = sliced");
        }
    }
}

// ------------------------------------------------------------------------------------------------ HOIST

/// HOIST: walk_day_h with ONE reused buffer (pre-filled with stale positions) gives every account the very state walk_day gives, day by day,
/// for every setting, share level and slot count (N 5 fills: slots_full fires). Mutant m_v33_hoist_whole_buffer must fail.
#[test]
fn v33_hoist_buffer_equals_walk_day() {
    let (evs, ex, tab) = fixture(3, 40, &|_, k| if k % 3 == 0 { 120 } else { 300 + (k % 7) * 60 }, &|d, k| i32::try_from((k * 37 + u32::from(d) * 11) % 200).unwrap() * 1_000 - 90_000);
    let list: Vec<u32> = (0..u32::try_from(evs.len()).unwrap()).collect();
    let keys = list_keys(&evs, &list);
    let sets = [CSet { rules: vec![(false, 0)], sl: 0 }, CSet { rules: vec![(false, 0), (false, 1)], sl: 0 }];
    let stale = Pos { ci: 7, skey: 7, exit: 1, margin: 99, net: -5, piece: 0, units: 1, sold: 1, mark1: 0, nc_units: 3, net_s2: -9, i0: 0 };
    let mut buf = vec![stale; crate::kern::MAXCAP];
    let mut full = 0;
    for skip in [true, false] {
        for cs in &sets {
            for l in 0..NL {
                for ni in 0..4 {
                    let (mut a1, mut p1) = (Acc::new(), Pieces::default());
                    let (mut a2, mut p2) = (Acc::new(), Pieces::default());
                    for d in 0..3_u32 {
                        let (lo, hi) = (d * 86_400, (d + 1) * 86_400);
                        let (a, b) = (keys.partition_point(|&k| k < lo), keys.partition_point(|&k| k < hi));
                        let s1 = walk_day(&evs, &ex, &tab, &list[a..b], &keys[a..b], cs, l, ni, None, true, skip, &mut a1, &mut p1, &[]);
                        let s2 = walk_day_h(&evs, &ex, &tab, &list[a..b], &keys[a..b], cs, l, ni, None, true, skip, &mut a2, &mut p2, &[], &mut buf);
                        assert_eq!(s1, s2, "steps");
                        assert_eq!(format!("{a1:?}{p1:?}"), format!("{a2:?}{p2:?}"), "setting {cs:?} l {l} N {ni} day {d} skip {skip}");
                    }
                    if a1.pk_margin >= 5 * 1_000_000 && ni == 0 {
                        full += 1;
                    }
                }
            }
        }
    }
    assert!(full > 0, "N 5 must fill all its slots somewhere (the slots_full rule is exercised)");
}

// ------------------------------------------------------------------------------------------------ LARGEST

/// LARGEST: the day's work items are a permutation of the items with events (none dropped, none twice), largest first, ties by batch
/// position; off = batch position order (round B). Mutant m_v33_largest_drops_ties must fail.
#[test]
fn v33_largest_first_is_a_permutation() {
    let lens = [3_usize, 0, 7, 3, 7, 1, 0, 3];
    assert_eq!(day_items33(&lens, false), vec![0, 2, 3, 4, 5, 7], "round B: list order, empty lists skipped");
    assert_eq!(day_items33(&lens, true), vec![2, 4, 0, 3, 7, 5], "largest first, ties by position");
    let mut a = day_items33(&lens, true);
    a.sort_unstable();
    assert_eq!(a, day_items33(&lens, false), "the same items: each list-day walked exactly once");
}

// ------------------------------------------------------------------------------------------------ ORDER

/// ORDER: index batches take the unique lists config-sorted (config id, liquidity value, list id), stock keeps creation order, every list
/// keeps its id (a permutation), and the checkpoint fingerprint names the order. Mutant m_v33_order_not_in_fingerprint must fail.
#[test]
fn v33_batch_order_permutation_and_fingerprint() {
    reset33();
    sw33::set(sw33::ORDER, true);
    let uls: Vec<(u32, u8, u8)> = vec![(40, 0, 255), (5, 1, 255), (17, 0, 255), (5, 0, 255), (9, 6, 255), (5, 6, 255)];
    let o = batch_order33(&uls, 0);
    assert_eq!(o, vec![3, 1, 5, 4, 2, 0], "config id, then liquidity value, then list id");
    let mut p = o.clone();
    p.sort_unstable();
    assert_eq!(p, (0..uls.len()).collect::<Vec<_>>(), "a permutation: every list once");
    assert_eq!(batch_order33(&uls, 1), (0..uls.len()).collect::<Vec<_>>(), "stock: creation order");
    assert!(fp_v33(true).contains("batch order config-sorted") && fp_v33(false).contains("batch order creation"), "the fingerprint names the order");
    assert_ne!(fp_v33(true), fp_v33(false), "two orders never share a checkpoint");
    sw33::set(sw33::ORDER, false);
    assert_eq!(batch_order33(&uls, 0), (0..uls.len()).collect::<Vec<_>>(), "ORDER off: round B's creation order");
    reset33();
}

// ------------------------------------------------------------------------------------------------ PREFETCH

/// PREFETCH: the prefetch loads day d's OWN files into day d's slot (the same bytes `ensure` would load). Mutant m_v33_prefetch_wrong_day
/// must fail.
#[test]
fn v33_prefetch_slot_is_its_day() {
    let days: Vec<String> = vec!["2022-07-01".to_owned(), "2022-07-04".to_owned(), "2022-07-05".to_owned()];
    let store = crate::report3::Store::new(std::path::Path::new("/nonexistent-cube-dir"), &days, true).unwrap();
    for d in 0..2 {
        let (slot, (dir, name)) = crate::report3::pf_src33(&store, d).unwrap();
        assert_eq!((slot, dir.to_string_lossy().into_owned(), name), (d, "/nonexistent-cube-dir".to_owned(), days[d].clone()));
    }
}

// ------------------------------------------------------------------------------------------------ D2-1 (DAYCLASS)

/// The D2-1 fixture: `days` days of `per` events 45 s apart from 10:00 on distinct contracts, Rs 4.5 L a lot (the Rs 20 L cap binds at 4
/// open positions and the cash rule binds once the realized net is negative), nets that swing (large losses early in the day: the day's own
/// realized change goes negative), exits 30 s to 10 min after the event, rule 1 (a TP at T + 5 s) on odd events, and marks that rise after
/// each sale (unrealised losses: MTM refusals, some counted as margin_mtm_short).
fn d2_fixture(days: u8, per: u32) -> (Vec<SEv>, Vec<EvX>, Tab3) {
    let mut evs = Vec::new();
    let mut ex = Vec::new();
    let mut tab = Tab3::default();
    for d in 0..days {
        for k in 0..per {
            let t = 36_000 + k * 45;
            let et = EvT { refuse: Refuse::None, lots: [1; 4], q_of: [0; 4], qs: vec![1], margin_lot: 45_000_000, filled: [true; NL], ..EvT::default() };
            let m0 = u32::try_from(tab.marks.len()).unwrap();
            let b0 = usize::try_from((i64::from(t) - crate::cal::OPEN_S) / 60).unwrap();
            // the option's minute marks: Rs 100 until the sale's minute, then Rs 150 (a Rs 5,000 loss on 100 units) or, on every 5th event,
            // Rs 4,000 (a Rs 3.9 L loss)
            let hi = if k % 5 == 0 { 400_000 } else { 15_000 };
            tab.marks.extend((0..crate::cal::NM).map(|m| if m > b0 { hi } else { 10_000 }));
            let x = EvX { r0: u32::try_from(tab.anc.len()).unwrap(), t0: 0, nq: 1, tf: Vec::new(), fill: Vec::new(), mark1: m0 + 1 };
            let dur = 30 + ((k * 7 + u32::from(d) * 3) % 20) * 30;
            let net = if k < 6 { -8_000_000 + i32::try_from(k).unwrap() * 1_000 } else { i32::try_from((k * 37 + u32::from(d) * 11) % 200).unwrap() * 100_000 - 9_000_000 };
            for l in 0..NL {
                let mut a = [ABSENT; NR];
                a[0] = off(i64::from(t) + 10);
                a[1] = if k % 2 == 1 { off(i64::from(t) + 5) } else { NEVER3 };
                let mut o = [0_u8; NR];
                o[0] = 1;
                o[1] = 2;
                tab.anc.push(a);
                tab.os.push(o);
                tab.obase.push(u32::try_from(tab.outs.len()).unwrap());
                let nt = net + i32::try_from(l).unwrap();
                let ex0 = u16::try_from(i64::from(t + dur) - BASE_S).unwrap();
                tab.outs.push(Out { net: 0, exit: ex0, flag: 0, ..Out::default() });
                tab.outs.push(Out { net: nt, exit: ex0, flag: 1, units: 100, sold: 1_000_000, ..Out::default() });
                tab.outs.push(Out { net: nt / 2, exit: u16::try_from(i64::from(t + dur / 2) - BASE_S).unwrap(), flag: 1, units: 100, sold: 1_000_000, ..Out::default() });
            }
            evs.push(SEv { di: u16::from(d), ci: k + 1_000 * u32::from(d), t, skey: k + 1_000 * u32::from(d), tab: et });
            ex.push(x);
        }
    }
    (evs, ex, tab)
}

/// D2-1, THE replay test (DESIGN-D2 section 14 `d2_replay_equals_walk`, the segment series included): on every day of the fixture, for every
/// setting, share level and slot count, and start-of-day realized levels from deep negative to positive, the ghost trace replayed through the
/// real hooks (when R >= R*) leaves the account, its year pieces AND its split segment series exactly as walking the day does; below R* the
/// day is walked. MTM, the grouped drawdown and the split permutations are on (the headline engine). Mutants m_v33_d2_no_rstar,
/// m_v33_d2_ghost_reads_loss, m_v33_d2_rstar_mtm_term_dropped, m_v33_d2_mtm_ref_dropped and m_v33_d2_cash_offsets_dropped must fail.
#[test]
fn v33_d2_replay_equals_walk() {
    for s in 0..crate::sw::COUNT {
        crate::sw::set(s, false);
    }
    crate::sw::set(crate::sw::MTM, true);
    crate::sw::set(crate::sw::DD_GROUPED, true);
    crate::sw::set(crate::sw::SPLITS18, true);
    let days: Vec<i64> = [(2019, 12, 30), (2019, 12, 31), (2020, 1, 1), (2020, 1, 2)].iter().map(|&(y, m, d)| crate::cal::days_from_civil(y, m, d)).collect();
    crate::split18::set_cal(crate::split18::SCal::from_days(&days).unwrap());
    let (evs, ex, tab) = d2_fixture(4, 30);
    let list: Vec<u32> = (0..u32::try_from(evs.len()).unwrap()).collect();
    let keys = list_keys(&evs, &list);
    let of_day: Vec<u8> = vec![1, 1, 2, 2];
    let sets = [CSet { rules: vec![(false, 0)], sl: 0 }, CSet { rules: vec![(false, 0), (false, 1)], sl: 0 }];
    let deltas: [i64; 16] = [-300_000_000, -200_000_000, -120_000_000, -80_000_000, -50_000_000, -30_000_000, -20_000_000, -10_000_000, -5_000_000, -1, 0, 10_000_000, 50_000_000, 100_000_000, 300_000_000, 1_000_000_000];
    let mut buf = vec![Pos::default(); crate::kern::MAXCAP];
    let mut tr = crate::kern::Trace::default();
    let (mut replayed, mut fell, mut finite, mut mtm, mut cash) = (0, 0, 0, 0, 0);
    for cs in &sets {
        for l in [0_usize, 1, 3] {
            for ni in 0..4 {
                let cap = crate::kern::cap_at(0, ni);
                let mut base = (Acc::new(), Pieces::new(&of_day, crate::wf::NPIECE));
                assert!(base.1.ms.is_some(), "the split segment series is on");
                for d in 0..4_u32 {
                    let (lo, hi) = (d * 86_400, (d + 1) * 86_400);
                    let (a, b) = (keys.partition_point(|&k| k < lo), keys.partition_point(|&k| k < hi));
                    let (sub, subk) = (&list[a..b], &keys[a..b]);
                    crate::kern::ghost_day(&evs, &ex, &tab, sub, subk, cs, l, ni, None, true, &of_day, &mut buf, &mut tr);
                    assert!(tr.booked.iter().all(|(k, p)| p.exit > *k), "every exit strictly after its booking second (Lemma D's premise)");
                    finite += usize::from(tr.r_star != i64::MIN);
                    mtm += usize::from(tr.mtm_ref > 0);
                    cash += usize::from(tr.cash_min_pass != i64::MAX);
                    // start levels: the base account shifted by each delta, and exactly at the trace's R* and one paisa above (a threshold set
                    // too low replays an account whose walk differs)
                    let mut starts: Vec<(bool, i64)> = deltas.iter().map(|&x| (false, x)).collect();
                    if tr.r_star != i64::MIN {
                        starts.push((true, tr.r_star));
                        starts.push((true, tr.r_star + 1));
                    }
                    for &(abs, dl) in &starts {
                        let mut w = base.clone();
                        if abs {
                            w.0.realized = dl;
                        } else {
                            w.0.realized += dl;
                        }
                        let mut r = w.clone();
                        walk_day(&evs, &ex, &tab, sub, subk, cs, l, ni, None, true, true, &mut w.0, &mut w.1, &of_day);
                        if crate::kern::replay_day(&mut r.0, &mut r.1, &tr, u16::try_from(d).unwrap(), of_day[d as usize], &mut buf, cap) {
                            replayed += 1;
                        } else {
                            fell += 1;
                            walk_day_h(&evs, &ex, &tab, sub, subk, cs, l, ni, None, true, true, &mut r.0, &mut r.1, &of_day, &mut buf);
                        }
                        assert_eq!(format!("{:?}{:?}", w.0, w.1), format!("{:?}{:?}", r.0, r.1), "setting {cs:?} l {l} N {ni} day {d} delta {dl}: replay = walk");
                    }
                    // the base account walks on to the next day
                    walk_day(&evs, &ex, &tab, sub, subk, cs, l, ni, None, true, true, &mut base.0, &mut base.1, &of_day);
                }
            }
        }
    }
    assert!(replayed > 100 && fell > 100, "both paths exercised: {replayed} replays, {fell} walks below R*");
    assert!(finite > 10 && mtm > 5 && cash > 10, "the fixture reaches R* ({finite}), MTM refusals ({mtm}) and passed cash checks ({cash})");
    for s in 0..crate::sw::COUNT {
        crate::sw::set(s, false);
    }
}

/// D2-1 `d2_rstar_needed`: R* is tight: one paisa below it the walk differs from the trace on some day (so the threshold cannot be relaxed),
/// at it the replay equals the walk.
#[test]
fn v33_d2_rstar_needed() {
    for s in 0..crate::sw::COUNT {
        crate::sw::set(s, false);
    }
    crate::sw::set(crate::sw::MTM, true);
    let (evs, ex, tab) = d2_fixture(1, 30);
    let list: Vec<u32> = (0..u32::try_from(evs.len()).unwrap()).collect();
    let keys = list_keys(&evs, &list);
    let mut buf = vec![Pos::default(); crate::kern::MAXCAP];
    let mut tr = crate::kern::Trace::default();
    let mut tight = 0;
    for ni in 0..4 {
        let cs = CSet { rules: vec![(false, 0)], sl: 0 };
        crate::kern::ghost_day(&evs, &ex, &tab, &list, &keys, &cs, 0, ni, None, true, &[], &mut buf, &mut tr);
        if tr.r_star == i64::MIN {
            continue;
        }
        for (dl, eq) in [(0_i64, true), (-1, false)] {
            let mut w = (Acc::new(), Pieces::default());
            // a previous day closed at R = R* + dl (day 0 of the walk is the fixture's day 0: start at that realized level)
            w.0.realized = tr.r_star + dl;
            let mut r = w.clone();
            walk_day(&evs, &ex, &tab, &list, &keys, &cs, 0, ni, None, true, true, &mut w.0, &mut w.1, &[]);
            // force the replay (as if R* were not checked)
            r.0.realized = tr.r_star;
            let ok = crate::kern::replay_day(&mut r.0, &mut r.1, &tr, 0, 0, &mut buf, crate::kern::cap_at(0, ni));
            assert!(ok);
            r.0.realized += dl;
            let same = (w.0.trades, w.0.mtm_ref, w.0.pk_margin) == (r.0.trades, r.0.mtm_ref, r.0.pk_margin);
            if eq {
                assert!(same, "N {ni}: at R* the walk makes the trace's decisions");
            } else if !same {
                tight += 1;
            }
        }
    }
    assert!(tight > 0, "one paisa below R* some walk decides differently (R* is tight)");
    for s in 0..crate::sw::COUNT {
        crate::sw::set(s, false);
    }
}

/// D2-1 `d2_class_key_has_tf`: lists share a class only with the same day sub-list AND the same timeframe key (a tf-dependent stop reads its
/// timeframe's rows); different sub-lists never share one, even with a colliding length. Mutant m_v33_d2_class_key_without_tf must fail.
#[test]
fn v33_d2_class_key_has_tf() {
    let s = |v: &[u32]| (v.to_vec(), v.to_vec());
    let subs = vec![s(&[1, 2, 3]), s(&[1, 2, 3]), s(&[1, 2, 3]), s(&[]), s(&[1, 2, 4]), s(&[1, 2, 3])];
    let tfs = vec![None, None, Some(2), None, None, Some(2)];
    let c = crate::report3::day_classes33(&subs, &tfs);
    assert_eq!(c, vec![vec![0, 1], vec![2, 5], vec![4]], "same events + same tf = one class; an empty day is in none");
}

// ------------------------------------------------------------------------------------------------ C4a (TABLEL)

/// C4a: Table L summed online (each event's fill list added once, to every liquidity value it passes) equals round B's sum over every
/// event's fill list per liquidity value. Mutant m_v33_tablel_want_as_got must fail.
#[test]
fn v33_tablel_online_equals_events() {
    // three events: readings (traded seconds, longest gap) and fill lists (level, lot index, units filled, units wanted)
    let evs: Vec<((u32, u32), Vec<(u8, u8, u64, u64)>)> = vec![
        ((58, 1), vec![(0, 0, 50, 100), (1, 0, 100, 100), (1, 1, 150, 200)]),
        ((20, 12), vec![(0, 0, 0, 75), (4, 0, 75, 75)]),
        ((46, 4), vec![(1, 0, 25, 50), (2, 0, 10, 40)]),
    ];
    let mut on = vec![[(0_u128, 0_u128); NL]; crate::read::NLIQ];
    for ((c, g), f) in &evs {
        crate::report3::tl_add(&mut on, f, *c, *g);
    }
    for v in 0..crate::read::NLIQ {
        let mut got = [0_u128; NL];
        let mut want = [0_u128; NL];
        for ((c, g), f) in &evs {
            if !crate::read::liq_pass(v, *c, *g) {
                continue;
            }
            for &(l, _, u, w) in f {
                got[usize::from(l)] += u128::from(u);
                want[usize::from(l)] += u128::from(w);
            }
        }
        for l in 0..NL {
            assert_eq!(on[v][l], (got[l], want[l]), "liquidity value {v}, level {l}");
        }
    }
}

// ------------------------------------------------------------------------------------------------ C3 (DERIVED): the D3 flagged re-walk

/// C3: the D3 exact re-walk of flagged account-rows reads every flagged list DERIVED under C3 (C3 never stores `ul_evs`), equal to round
/// B's stored list for every unique list and for any subset; with stored lists the helper stays empty (the re-walk reads `ul_evs` itself,
/// round B's path). Mutant m_v33_c3_flag_lists_stored (the 23:37 plain binary's defect) must fail.
#[test]
fn v33_c3_flagged_lists_derived() {
    let liqs: Vec<usize> = vec![0, 1, 6, 7];
    let mut a = list_seg();
    let mut b = list_seg();
    let mut log = String::new();
    build_lists(&mut a, &liqs, false, &mut log, "t");
    build_lists_derived(&mut b, &liqs, false, &mut log, "t");
    assert!(b.ul_derived && b.ul_evs.is_empty() && !a.ul_derived && a.uls.len() >= 4);
    let all: Vec<u32> = (0..a.uls.len() as u32).collect();
    let got = b.flag_lists33(&all);
    assert_eq!(got.len(), a.uls.len(), "every flagged list found under C3");
    for &u in &all {
        assert_eq!(got.get(&u), a.ul_evs.get(u as usize), "list {u}: the flagged re-walk's derived list = round B's ul_evs");
    }
    // a subset in any order (the re-walk passes the sorted, deduplicated lists of one row start) and an id past the end (skipped)
    let some: Vec<u32> = vec![all[all.len() - 1], 0, u32::MAX];
    let got2 = b.flag_lists33(&some);
    assert_eq!(got2.len(), 2);
    assert_eq!(got2.get(&0), a.ul_evs.first());
    assert!(a.flag_lists33(&all).is_empty(), "stored lists: round B's path (ul_evs) is used, the helper stays empty");
    assert!(a.ul_evs.iter().any(|l| !l.is_empty()), "the fixture has non-empty lists");
}

// ------------------------------------------------------------------------------------------------ ACHUNK (stage A work items)

/// ACHUNK: stage A items of 1 or 4 contracts give, read in order, the SAME event sequence as round B's 64-contract items (the order the
/// tables, marks, counters and records are appended in); every contract-day's events sit in ONE item (its tape, trail index and marks are
/// built once, as in round B); no item holds more than `maxc` contracts; items keep the day order. Mutant m_v33_achunk_splits_contract (an
/// item cut inside a contract) must fail.
#[test]
fn v33_achunk_same_sequence_whole_contracts() {
    // 3 days x 9 contracts; contract k has 1 + (k % 4) events (2 with the very same second: the stable sort keeps their id order)
    let mut evs: Vec<SEv> = Vec::new();
    for d in 0..3_u16 {
        for k in 0..9_u32 {
            for j in 0..(1 + k % 4) {
                let t = 36_000 + (k % 3) * 5 + if j == 2 { 60 } else { j * 60 };
                evs.push(SEv { di: d, ci: 100 + (k * 7) % 9, t, skey: k, tab: EvT::default() });
            }
        }
    }
    let n = evs.len();
    let ids: Vec<usize> = (0..n).rev().collect();
    let flat = |c: &[(usize, Vec<usize>)]| -> Vec<usize> { c.iter().flat_map(|x| x.1.iter().copied()).collect() };
    let c64 = stage_a_chunks33(&evs, ids.clone(), 64);
    assert_eq!(c64.len(), 3, "round B: one item per day here (9 contracts < 64)");
    let mut distinct: std::collections::HashSet<(u16, u32)> = std::collections::HashSet::new();
    for e in &evs {
        distinct.insert((e.di, e.ci));
    }
    for maxc in [1_usize, 4] {
        let c = stage_a_chunks33(&evs, ids.clone(), maxc);
        assert_eq!(flat(&c), flat(&c64), "maxc {maxc}: the same event sequence as round B's 64-contract items");
        let mut seen: std::collections::HashSet<(u16, u32)> = std::collections::HashSet::new();
        let mut last_day = 0;
        for (di, item) in &c {
            assert!(*di >= last_day && !item.is_empty(), "items keep the day order");
            last_day = *di;
            assert!(item.iter().all(|&i| usize::from(evs[i].di) == *di), "an item holds one day");
            let mut cons: Vec<u32> = item.iter().map(|&i| evs[i].ci).collect();
            cons.dedup();
            assert!(cons.len() <= maxc, "maxc {maxc}: an item holds {} contracts", cons.len());
            for &ci in &cons {
                assert!(seen.insert((evs[item[0]].di, ci)), "maxc {maxc}: contract {ci} of day {di} in ONE item only");
            }
        }
        assert_eq!(seen, distinct, "every contract-day in an item");
        if maxc == 1 {
            assert_eq!(c.len(), distinct.len(), "one contract-day per item");
        }
    }
}

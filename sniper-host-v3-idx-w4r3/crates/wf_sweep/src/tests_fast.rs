//! Tests of the speed build (sniper-host-v1-fast, 2026-11-14): LAZY-EQ (the lazy trees, the memo and the gate give exactly the
//! eager accumulator's facts, keys, step counts and lists), the prepaid merge bound, and the exactness bound that keeps an
//! out-of-bound outcome table on the eager path.

use crate::acc::{KeyKind, LAZY_SUM_MAX, Layout, Outcome, SweepAcc};
use crate::lists::Lists;
use crate::views::View;
use crate::{MAXC, mix};

fn mk(y: i32, m: i32) -> i32 {
    y * 12 + m - 1
}

/// 5 sessions a month from 2018-09 to 2022-12 (every session a learning session).
fn calendar() -> Vec<i32> {
    let mut v = Vec::new();
    let mut m = mk(2018, 9);
    while m <= mk(2022, 12) {
        for _ in 0..5 {
            v.push(m);
        }
        m += 1;
    }
    v
}

fn layout(cal: &[i32]) -> Layout {
    let wins = vec![("WF".to_string(), mk(2018, 9), mk(2019, 12)), ("TT".to_string(), mk(2018, 9), mk(2022, 12))];
    Layout::new(cal, (mk(2018, 9), mk(2022, 12)), &wins).unwrap()
}

fn rng(x: u64) -> u64 {
    mix(x.wrapping_add(0x9e37_79b9_7f4a_7c15))
}

/// Up to three candidate rows per session (FIRST mode keeps one at a time), random nets in three columns, a positive drift so
/// that many cells pass E2 (and the R73 / -N lists fill) while many fail it (the -MISS lists fill).
fn outcomes(cal: &[i32], seed: u64) -> Vec<Outcome> {
    let mut st = seed;
    let mut v = Vec::new();
    for (s, &m) in cal.iter().enumerate() {
        st = rng(st);
        for _ in 0..1 + st % 3 {
            st = rng(st);
            let mut net = [0_i64; MAXC];
            let mut rd = [0_u16; MAXC];
            for c in 0..3 {
                st = rng(st);
                net[c] = (i64::try_from(st % 20_001).unwrap() - 8_000 - 1_000 * i64::try_from(c).unwrap()) * 100;
                rd[c] = u16::try_from((st >> 17) % 500).unwrap();
            }
            v.push(Outcome { sess: u32::try_from(s).unwrap(), mk: m, id: 7_000 + v.len() as u64, net, rd_bp: rd });
        }
    }
    v
}

fn views() -> Vec<View> {
    let mut v = Vec::new();
    for (w, name) in [(0_usize, "WF"), (1, "TT")] {
        for floor in [1_u32, 2, 4, 8] {
            v.push(View { id: format!("{name}-R73-F{floor}"), win: w, floor, the: floor == 1, kind: KeyKind::R73, k: Some(if floor == 1 { 6 } else { 3 }) });
        }
        v.push(View { id: format!("{name}-N"), win: w, floor: 1, the: false, kind: KeyKind::N, k: Some(3) });
        v.push(View { id: format!("{name}-MISS"), win: w, floor: 1, the: false, kind: KeyKind::Miss, k: Some(2) });
    }
    v
}

/// One random event on both accumulators (insert a free session's row, remove a held one, move a session's trade, or reset);
/// the step counts must be equal.
fn event(a: &mut SweepAcc<'_>, b: &mut SweepAcc<'_>, out: &[Outcome], held: &mut [Option<u32>], st: &mut u64) {
    *st = rng(*st);
    if (*st).is_multiple_of(97) {
        a.reset().unwrap();
        b.reset().unwrap();
        held.iter_mut().for_each(|h| *h = None);
        return;
    }
    let r = u32::try_from(*st % out.len() as u64).unwrap();
    let s = out[r as usize].sess as usize;
    match held[s] {
        None => {
            assert_eq!(a.insert(r).unwrap(), b.insert(r).unwrap(), "insert steps");
            held[s] = Some(r);
        }
        Some(h) if h == r => {
            assert_eq!(a.remove(r).unwrap(), b.remove(r).unwrap(), "remove steps");
            held[s] = None;
        }
        Some(h) => {
            assert_eq!(a.remove(h).unwrap(), b.remove(h).unwrap(), "move: remove steps");
            assert_eq!(a.insert(r).unwrap(), b.insert(r).unwrap(), "move: insert steps");
            held[s] = Some(r);
        }
    }
}

#[test]
fn lazy_eq_eager_every_fact_key_and_step() {
    // LAZY-EQ: after every event, every learning fact (pure read), every view key of every kind and floor (the memoised fast
    // read) and every step count of the lazy accumulator equals the eager one's (binary k's tree path).
    let cal = calendar();
    let lay = layout(&cal);
    for seed in 1..=8_u64 {
        let out = outcomes(&cal, seed);
        let mut lz = SweepAcc::new(&lay, &out, 3, cal.len()).unwrap();
        let mut eg = SweepAcc::new_eager(&lay, &out, 3, cal.len()).unwrap();
        assert!(lz.is_lazy() && !eg.is_lazy());
        let mut held = vec![None; cal.len()];
        let mut st = seed * 104_729;
        for step in 0..1_500 {
            event(&mut lz, &mut eg, &out, &mut held, &mut st);
            if step % 7 != 0 {
                continue;
            }
            for w in 0..2 {
                for col in 0..3 {
                    // the pure read first (dirty nodes composed, never written), then the fast reads, then the pure read again
                    assert_eq!(lz.learn(w, col).unwrap(), eg.learn(w, col).unwrap(), "seed {seed} step {step} learn w {w} col {col}");
                    for kind in [KeyKind::R73, KeyKind::N, KeyKind::Miss] {
                        for floor in [1_u32, 2, 4, 8] {
                            assert_eq!(lz.view_key(w, col, floor, kind).unwrap(), eg.view_key(w, col, floor, kind).unwrap(), "seed {seed} step {step} {kind:?} F{floor}");
                        }
                    }
                    assert_eq!(lz.key(w, col, 1).unwrap(), eg.key(w, col, 1).unwrap(), "seed {seed} step {step} key");
                    assert_eq!(lz.learn(w, col).unwrap(), eg.learn(w, col).unwrap(), "seed {seed} step {step} learn after reads");
                }
            }
            assert_eq!((lz.inserts, lz.removes, lz.key_reads), (eg.inserts, eg.removes, eg.key_reads), "counters");
        }
    }
}

#[test]
fn lazy_merges_never_exceed_the_charged_path() {
    // The prepaid charge: each event is charged its whole path (the eager count, unchanged step telemetry); every merge the lazy
    // reads perform is on a node some charged event dirtied, so merges performed <= merges charged; the eager tree performs
    // exactly what it is charged.
    let cal = calendar();
    let lay = layout(&cal);
    let out = outcomes(&cal, 99);
    let mut lz = SweepAcc::new(&lay, &out, 3, cal.len()).unwrap();
    let mut eg = SweepAcc::new_eager(&lay, &out, 3, cal.len()).unwrap();
    let mut held = vec![None; cal.len()];
    let mut st = 4_242_u64;
    for step in 0..20_000 {
        event(&mut lz, &mut eg, &out, &mut held, &mut st);
        if step % 3 == 0 {
            for col in 0..3 {
                assert_eq!(lz.view_key(1, col, 1, KeyKind::Miss).unwrap(), eg.view_key(1, col, 1, KeyKind::Miss).unwrap());
                assert_eq!(lz.key(1, col, 1).unwrap(), eg.key(1, col, 1).unwrap());
            }
        }
        assert!(lz.merges_done <= lz.merges_charged, "step {step}: {} > {}", lz.merges_done, lz.merges_charged);
    }
    assert_eq!(lz.merges_charged, eg.merges_charged, "the same charge");
    assert_eq!(eg.merges_done, eg.merges_charged, "the eager tree does what it is charged");
    assert!(lz.merges_done < lz.merges_charged, "the lazy tree defers work: {} of {}", lz.merges_done, lz.merges_charged);
}

#[test]
fn gated_lists_equal_ungated_lists() {
    // The gate: the host's offer loop with view_key_gated + Lists::rejects_prefix / count_rejected gives lists byte-equal to the
    // loop with view_key + offer_learn (held entries, counters, ties at #1), with the same step counts and refusal reasons;
    // and the gate really fires.
    let cal = calendar();
    let lay = layout(&cal);
    let vs = views();
    let mut fired = 0_u64;
    for seed in 1..=8_u64 {
        let out = outcomes(&cal, seed);
        let mut a = SweepAcc::new(&lay, &out, 3, cal.len()).unwrap();
        let mut b = SweepAcc::new_eager(&lay, &out, 3, cal.len()).unwrap();
        let mut la: Vec<Lists> = vs.iter().flat_map(|v| (0..3).map(|_| Lists::new(v.k.unwrap_or(5)))).collect();
        let mut lb = la.clone();
        let mut held = vec![None; cal.len()];
        let mut st = seed * 7_919;
        let mut code = 0_u64;
        for _ in 0..3_000 {
            event(&mut a, &mut b, &out, &mut held, &mut st);
            st = rng(st);
            let run = 1 + st % 4;
            for (vi, v) in vs.iter().enumerate() {
                for c in 0..3 {
                    let i = vi * 3 + c;
                    let l = &mut la[i];
                    let mut asked = false;
                    let (ka, sa) = a
                        .view_key_gated(v.win, c, v.floor, v.kind, |p| {
                            asked = true;
                            l.rejects_prefix(p)
                        })
                        .unwrap();
                    let (kb, sb) = b.view_key(v.win, c, v.floor, v.kind).unwrap();
                    assert_eq!(sa, sb, "steps");
                    match (ka, kb) {
                        (Ok(Some(x)), Ok(y)) => {
                            assert_eq!(x, y);
                            l.offer_learn(x, [code, 0, 0, 0], run);
                            lb[i].offer_learn(y, [code, 0, 0, 0], run);
                        }
                        (Ok(None), Ok(y)) => {
                            assert!(asked);
                            fired += 1;
                            l.count_rejected(run);
                            let before = lb[i].accepts;
                            lb[i].offer_learn(y, [code, 0, 0, 0], run);
                            assert_eq!(lb[i].accepts, before, "a gated key is a rejected key");
                        }
                        (Err(x), Err(y)) => assert_eq!(x, y),
                        (x, y) => panic!("gated {x:?} vs plain {y:?}"),
                    }
                    assert_eq!(la[i], lb[i], "seed {seed} view {} col {c}", v.id);
                }
            }
            code += run + st % 3;
        }
    }
    assert!(fired > 1_000, "the gate fired {fired} times");
}

#[test]
fn rejects_prefix_reads_only_a_full_list() {
    let mut l = Lists::new(2);
    assert!(!l.rejects_prefix(&[9]), "not full");
    l.offer(crate::acc::Key::of(&[1, 5]), [1, 0, 0, 0]);
    l.offer(crate::acc::Key::of(&[2, 5]), [2, 0, 0, 0]);
    assert!(l.rejects_prefix(&[3]), "3 > worst's 2");
    assert!(!l.rejects_prefix(&[2]), "a tie on the prefix needs the full key");
    assert!(!l.rejects_prefix(&[1]));
    assert!(!l.rejects_prefix(&[]), "an empty prefix never rejects");
    assert!(l.rejects_prefix(&[2, 6]));
    assert!(!l.rejects_prefix(&[2, 5]));
}

#[test]
fn an_out_of_bound_table_stays_eager() {
    // The exactness bound: a table whose sum of |net| exceeds LAZY_SUM_MAX in some column keeps binary k's eager path.
    let cal = calendar();
    let lay = layout(&cal);
    let mut out = outcomes(&cal, 3);
    assert!(SweepAcc::new(&lay, &out, 3, cal.len()).unwrap().is_lazy());
    out[0].net[2] = LAZY_SUM_MAX;
    assert!(!SweepAcc::new(&lay, &out, 3, cal.len()).unwrap().is_lazy(), "column 2 is out of bound");
    assert!(SweepAcc::new(&lay, &out, 2, cal.len()).unwrap().is_lazy(), "column 2 is not carried");
    out[0].net[2] = i64::MIN;
    assert!(!SweepAcc::new(&lay, &out, 3, cal.len()).unwrap().is_lazy());
}

#[test]
fn grouped_offer_all_equals_the_per_view_loop() {
    // OFFER-EQ: SweepAcc::offer_all (grouped by column and window, gate on) gives every list, every not-eligible count and the
    // key-read step histogram exactly as binary k's host loop (view_key per view and column on the EAGER accumulator, then
    // offer_learn), after every offer.
    let cal = calendar();
    let lay = layout(&cal);
    let vs = views();
    for seed in 1..=8_u64 {
        let out = outcomes(&cal, seed);
        let mut a = SweepAcc::new(&lay, &out, 3, cal.len()).unwrap();
        let mut b = SweepAcc::new_eager(&lay, &out, 3, cal.len()).unwrap();
        let mut la: Vec<Lists> = vs.iter().flat_map(|v| (0..3).map(|_| Lists::new(v.k.unwrap_or(5)))).collect();
        let mut lb = la.clone();
        let (mut nea, mut neb) = ([0_u64; 4], [0_u64; 4]);
        let (mut ta, mut tb) = (Vec::new(), vec![0_u64; 64]);
        let mut big: Vec<u64> = Vec::new();
        let mut held = vec![None; cal.len()];
        let mut st = seed * 31_337;
        let mut code = 0_u64;
        for _ in 0..3_000 {
            event(&mut a, &mut b, &out, &mut held, &mut st);
            st = rng(st);
            let run = 1 + st % 4;
            let mut sink = crate::acc::OfferSink { lists: &mut la, ne: &mut nea, tally: &mut ta, big: &mut big };
            a.offer_all(&vs, [code, 0, 0, 0], run, &mut sink).unwrap();
            for (vi, v) in vs.iter().enumerate() {
                for c in 0..3 {
                    let (k, s) = b.view_key(v.win, c, v.floor, v.kind).unwrap();
                    let si = usize::try_from(s).unwrap();
                    if si >= tb.len() {
                        tb.resize(si + 1, 0);
                    }
                    tb[si] += 1;
                    match k {
                        Ok(key) => {
                            lb[vi * 3 + c].offer_learn(key, [code, 0, 0, 0], run);
                        }
                        Err(e) => {
                            neb[match e {
                                crate::acc::NotEligible::NoTrade => 0,
                                crate::acc::NotEligible::PartNotPositive => 1,
                                crate::acc::NotEligible::BelowFloor => 2,
                                crate::acc::NotEligible::PassesE2 => 3,
                            }] += 1;
                        }
                    }
                }
            }
            assert_eq!(la, lb, "seed {seed}: lists");
            assert_eq!(nea, neb, "seed {seed}: not eligible");
            let mut tal = ta.clone();
            for &s in &big {
                let si = usize::try_from(s).unwrap();
                if si >= tal.len() {
                    tal.resize(si + 1, 0);
                }
                tal[si] += 1;
            }
            let width = tal.len().max(tb.len());
            let tan: Vec<u64> = (0..width).map(|i| tal.get(i).copied().unwrap_or(0)).collect();
            let tbn: Vec<u64> = (0..width).map(|i| tb.get(i).copied().unwrap_or(0)).collect();
            assert_eq!(tan, tbn, "seed {seed}: key-read steps");
            assert_eq!(a.key_reads, b.key_reads, "seed {seed}: key reads");
            code += run + st % 3;
        }
        assert!(nea.iter().all(|&x| x > 0), "every reason seen: {nea:?}");
    }
}

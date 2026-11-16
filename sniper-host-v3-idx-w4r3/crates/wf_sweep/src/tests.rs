//! Red-first tests of the D-S14 additions: SWEEP-EQ (against an independent plain recomputation until `common_wf`'s `Acc` is
//! linked), OR-EQ, the test-month refusal (SN-X17), E2 (SN-X01), the per-event step gate (m-S14).

use std::collections::BTreeSet;

use crate::acc::{Key, Layout, NotEligible, Outcome, SweepAcc};
use crate::dd::Dd;
use crate::lists::Lists;
use crate::views;
use crate::{MAXC, SweepErr, mix};

fn mk(y: i32, m: i32) -> i32 {
    y * 12 + m - 1
}

/// A synthetic calendar: 6 sessions a month from 2018-09 to 2023-03 (the last 3 months are TEST months, outside the hull).
fn calendar() -> Vec<i32> {
    let mut v = Vec::new();
    let mut m = mk(2018, 9);
    while m <= mk(2023, 3) {
        for _ in 0..6 {
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

fn rng(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
    mix(x)
}

/// One outcome per session (row id = session index), random nets in two columns.
fn outcomes(cal: &[i32], seed: u64) -> Vec<Outcome> {
    let mut st = seed;
    cal.iter()
        .enumerate()
        .map(|(i, &m)| {
            st = rng(st);
            let a = i64::try_from(st % 20_001).unwrap() - 9_000;
            st = rng(st);
            let b = i64::try_from(st % 20_001).unwrap() - 11_000;
            let mut net = [0_i64; MAXC];
            net[0] = a * 100;
            net[1] = b * 100;
            let mut rd = [0_u16; MAXC];
            rd[0] = u16::try_from(st % 400).unwrap();
            rd[1] = u16::try_from((st >> 9) % 400).unwrap();
            Outcome { sess: u32::try_from(i).unwrap(), mk: m, id: 1000 + i as u64, net, rd_bp: rd }
        })
        .collect()
}

/// The plain recomputation of one window's learning facts from the held set, in time order (session order).
#[allow(clippy::type_complexity)]
fn plain(out: &[Outcome], held: &BTreeSet<u32>, to: i32, col: usize) -> (i64, u32, u32, i64, i64, u16, i64, u32, u32, u64, i64) {
    let mut dd = Dd::ZERO;
    let (mut net, mut n, mut wins, mut worst, mut rd, mut fp) = (0_i64, 0_u32, 0_u32, i64::MAX, 0_u16, 0_u64);
    let mut months: std::collections::BTreeMap<i32, (i64, u32)> = std::collections::BTreeMap::new();
    for &r in held {
        let o = out[r as usize];
        if o.mk > to {
            continue;
        }
        let x = o.net[col];
        // the naive monoid push, left to right, written out (never the module's combine)
        let (t, hi, lo, mdd) = (dd.total + x, dd.max_prefix.max(dd.total + x), dd.min_prefix.min(dd.total + x), 0);
        let peak = dd.max_prefix.max(0);
        let fall = peak - (dd.total + x);
        dd = Dd { total: t, max_prefix: hi, min_prefix: lo, max_dd: dd.max_dd.max(fall).max(mdd) };
        net += x;
        n += 1;
        wins += u32::from(x > 0);
        worst = worst.min(x);
        rd = rd.max(o.rd_bp[col]);
        fp = fp.wrapping_add(mix(o.id));
        let e = months.entry(o.mk).or_insert((0, 0));
        e.0 += x;
        e.1 += 1;
    }
    let losing: i64 = months.values().map(|(v, _)| (*v).min(0)).sum();
    let mt = u32::try_from(months.len()).unwrap();
    let mp = u32::try_from(months.values().filter(|(v, _)| *v > 0).count()).unwrap();
    (net, n, wins, dd.max_dd, if n == 0 { 0 } else { worst }, rd, losing, mt, mp, fp, dd.min_prefix)
}

#[test]
fn sweep_eq_against_the_plain_recomputation() {
    // SWEEP-EQ (red first; m-S6 composes right to left): random inserts and removals in ANY order; after each step every learning
    // fact of both windows and both columns equals the plain time-order recomputation.
    let cal = calendar();
    let lay = layout(&cal);
    for seed in 1..=6_u64 {
        let out = outcomes(&cal, seed);
        let mut acc = SweepAcc::new(&lay, &out, 2, cal.len()).unwrap();
        let mut held: BTreeSet<u32> = BTreeSet::new();
        let learn_rows: Vec<u32> = (0..cal.len()).filter(|&i| cal[i] <= mk(2022, 12)).map(|i| u32::try_from(i).unwrap()).collect();
        let mut st = seed * 7919;
        for step in 0..400 {
            st = rng(st);
            let r = learn_rows[usize::try_from(st % learn_rows.len() as u64).unwrap()];
            if held.contains(&r) {
                acc.remove(r).unwrap();
                held.remove(&r);
            } else {
                acc.insert(r).unwrap();
                held.insert(r);
            }
            if step % 37 == 0 {
                for (w, to) in [(0_usize, mk(2019, 12)), (1, mk(2022, 12))] {
                    for col in 0..2 {
                        let (l, _) = acc.learn(w, col).unwrap();
                        let p = plain(&out, &held, to, col);
                        assert_eq!((l.net, l.trades, l.wins, l.max_dd, l.worst, l.rd_bp, l.losing, l.months_traded, l.months_pos, l.fp, l.min_prefix), p, "seed {seed} step {step} w {w} col {col}");
                    }
                }
            }
        }
        acc.reset().unwrap();
        let (l, _) = acc.learn(1, 0).unwrap();
        assert_eq!((l.trades, l.net, l.max_dd), (0, 0, 0), "reset empties every cell");
    }
}

#[test]
fn a_test_month_trade_is_refused() {
    // SN-X17 / m-S1: a 2023 trade into the sweep accumulator is TestMonthInSweep.
    let cal = calendar();
    let lay = layout(&cal);
    let out = outcomes(&cal, 3);
    let mut acc = SweepAcc::new(&lay, &out, 2, cal.len()).unwrap();
    let test_row = u32::try_from(cal.iter().position(|&m| m == mk(2023, 1)).unwrap()).unwrap();
    assert_eq!(acc.insert(test_row), Err(SweepErr::TestMonthInSweep));
}

#[test]
fn e2_every_learning_year_part_positive() {
    // SN-X01: a combo with no trade in the 2018 learning part fails E2 (red drops the empty part and passes it).
    let cal = calendar();
    let lay = layout(&cal);
    let mut out = outcomes(&cal, 5);
    for o in &mut out {
        o.net[0] = 50_000;
    }
    let mut acc = SweepAcc::new(&lay, &out, 1, cal.len()).unwrap();
    let r2019 = u32::try_from(cal.iter().position(|&m| m == mk(2019, 3)).unwrap()).unwrap();
    acc.insert(r2019).unwrap();
    assert_eq!(acc.eligible(0, 0, 1), Err(NotEligible::PartNotPositive), "WF: 2018 has no trade");
    let r2018 = u32::try_from(cal.iter().position(|&m| m == mk(2018, 10)).unwrap()).unwrap();
    acc.insert(r2018).unwrap();
    assert_eq!(acc.eligible(0, 0, 1), Ok(()), "WF: both parts positive");
    assert_eq!(acc.eligible(0, 0, 2), Err(NotEligible::BelowFloor), "-F2 needs two trades in every part");
    assert_eq!(acc.eligible(1, 0, 1), Err(NotEligible::PartNotPositive), "TT: 2020-2022 have no trade");
    // A second trade in one session's leaf is refused (FIRST mode holds one trade a session and side).
    let other = out.iter().position(|o| o.sess == out[r2018 as usize].sess && o.id != out[r2018 as usize].id);
    assert!(other.is_none(), "one outcome per session in this calendar");
}

#[test]
fn key_read_steps_identical_at_1x_10x_100x() {
    // The HARD gate for the accumulator (F19 / m-S14): the per-event counts of inserts, removals and key reads do not move when
    // the churn between reads grows 10x and 100x.
    let cal = calendar();
    let lay = layout(&cal);
    let out = outcomes(&cal, 11);
    let rows: Vec<u32> = (0..cal.len()).filter(|&i| cal[i] <= mk(2022, 12)).map(|i| u32::try_from(i).unwrap()).collect();
    let mut seen = Vec::new();
    for times in [1_usize, 10, 100] {
        let mut acc = SweepAcc::new(&lay, &out, 2, cal.len()).unwrap();
        for &r in rows.iter().take(40) {
            acc.insert(r).unwrap();
        }
        let mut ev = BTreeSet::new();
        for _ in 0..times {
            for &r in rows.iter().take(40) {
                ev.insert(acc.remove(r).unwrap());
                ev.insert(acc.insert(r).unwrap());
            }
        }
        let (_, _, ks) = acc.key(0, 0, 1).unwrap();
        seen.push((ev, ks));
    }
    assert_eq!(seen[0].1, seen[1].1, "key read 1x vs 10x");
    assert_eq!(seen[0].1, seen[2].1, "key read 1x vs 100x");
    assert!(seen[0].1 <= 1 + 5 + 1, "SC-K5: at most 5 part compositions");
}

#[test]
fn or_eq_runs_equal_single_offers() {
    // OR-EQ (red first; m-S2 lives in the kernel): offer_learn(.., n) and n single offers give byte-equal lists and counters.
    for seed in 1..=20_u64 {
        let mut a = Lists::new(25);
        let mut b = Lists::new(25);
        let mut st = seed;
        let mut base = 0_u64;
        for _ in 0..60 {
            st = rng(st);
            let key = Key::of(&[i64::try_from(st % 7).unwrap(), 0, 0, 0, 0, i64::try_from((st >> 5) % 3).unwrap(), 0]);
            let n = 1 + (st >> 11) % 9;
            a.offer_learn(key, [base, 0, 0, 0], n);
            for i in 0..n {
                b.offer(key, [base + i, 0, 0, 0]);
            }
            base += n + (st >> 20) % 3;
        }
        assert_eq!(a.expanded(), b.expanded(), "seed {seed}");
        assert_eq!((a.offered, a.top, a.len()), (b.offered, b.top, b.len()), "seed {seed}");
        assert!(a.len() <= 25);
    }
}

#[test]
fn a_run_is_split_at_the_bound() {
    let mut l = Lists::new(3);
    l.offer_learn(Key::of(&[5]), [10, 0, 0, 0], 5);
    assert_eq!(l.expanded().iter().map(|e| e.1[0]).collect::<Vec<_>>(), vec![10, 11, 12], "the smallest codes are kept");
    l.offer(Key::of(&[1]), [99, 0, 0, 0]);
    assert_eq!(l.expanded().iter().map(|e| e.1[0]).collect::<Vec<_>>(), vec![99, 10, 11]);
    assert_eq!(l.offered, 6);
}

#[test]
fn views_rows_parse_fail_closed() {
    let w = vec!["WF".to_string(), "TT".to_string()];
    let ok = "id\twindow\tfloor\trole\nWF-R73\tWF\t1\tTHE\nTT-R73\tTT\t1\tTHE\nWF-R73-F2\tWF\t2\tBESIDE\n";
    assert_eq!(views::parse(ok, &w).unwrap().len(), 3);
    assert!(views::parse("WF-R73\tWF\t1\tTHE\n", &w).is_err(), "no THE view for TT");
    assert!(views::parse("WF-R73\tWF\t0\tTHE\nTT-R73\tTT\t1\tTHE\n", &w).is_err(), "floor 0");
    assert!(views::parse("WF-R73\tXX\t1\tTHE\nTT-R73\tTT\t1\tTHE\n", &w).is_err(), "unknown window");
}

#[test]
fn windows_must_be_year_units_inside_the_hull() {
    let cal = calendar();
    let bad = vec![("WF".to_string(), mk(2018, 9), mk(2019, 6))];
    assert!(Layout::new(&cal, (mk(2018, 9), mk(2022, 12)), &bad).is_err());
    let lay = layout(&cal);
    assert_eq!(lay.years, vec![2018, 2019, 2020, 2021, 2022]);
    assert!(lay.depth(1) <= 9, "a part's tree depth is fixed by the calendar");
}

#[test]
fn red_build_is_refused() {
    if crate::RED {
        assert!(crate::refuse_if_red().is_err());
    } else {
        assert!(crate::refuse_if_red().is_ok());
    }
}

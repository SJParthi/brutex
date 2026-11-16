//! Speed build (sniper-host-v1-fast, 2026-11-14): RUN-ALL-EQ, the shared walk of every N (`sweep::run_all`) against one
//! `sweep::run` per N (binary k's loop): lists, walk counts, step histograms, not-eligible counts and refusals equal, at 1 and
//! 3 worker threads, over several outer values with random kept sets, three FIRST-mode candles a session and random nets.
#![allow(clippy::indexing_slicing, clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

use sniper_host::cands::OuterValue;
use sniper_host::sweep::{CHUNK, Ctx, Out, run, run_all, run_all_kept, run_all_with, run_with};
use sniper_kernel::bits::Bits;
use sniper_kernel::line::{Core, Row};
use wf_sweep::acc::{Layout, Outcome};

fn mix(x: u64) -> u64 {
    wf_sweep::mix(x.wrapping_add(0x9e37_79b9_7f4a_7c15))
}

fn same(a: &Out, b: &Out, what: &str) {
    // Every list field the host reads (entries, counters, ties at #1); `accepts` counts inserts and depends on which worker a
    // value lands on (dynamic scheduling, in binary k too), and nothing prints it.
    let l = |o: &Out| o.lists.iter().map(|x| (x.entries().collect::<Vec<_>>(), x.offered, x.runs, x.top, x.len(), x.bound())).collect::<Vec<_>>();
    assert_eq!(l(a), l(b), "{what}: lists");
    assert_eq!(a.hist_acc, b.hist_acc, "{what}: accumulator steps");
    assert_eq!(a.hist_key, b.hist_key, "{what}: key-read steps");
    assert_eq!(a.not_eligible, b.not_eligible, "{what}: not eligible");
    assert_eq!((a.values_walked, a.values_empty), (b.values_walked, b.values_empty), "{what}: values");
    let w = |o: &Out| (o.walk.combos, o.walk.lines, o.walk.lines_empty, o.walk.cells, o.walk.empty_cells, o.walk.runs_offered, o.walk.cells_offered, o.walk.entered, o.walk.inserts, o.walk.removes, o.walk.hist_enter.clone(), o.walk.hist_line.clone());
    assert_eq!(w(a), w(b), "{what}: walk counts");
    let mut ra = a.refused.clone();
    let mut rb = b.refused.clone();
    ra.sort();
    rb.sort();
    assert_eq!(ra, rb, "{what}: refusals");
}

#[test]
fn run_all_equals_one_run_per_n() {
    let mk = |y: i32, m: i32| y * 12 + m - 1;
    let mut sess_mk = Vec::new();
    for m in mk(2018, 9)..=mk(2022, 12) {
        for _ in 0..4 {
            sess_mk.push(m);
        }
    }
    let n = sess_mk.len();
    let core = Core { nr: 3, nc: 9, nw: 2 };
    let mut rows = Vec::new();
    let mut st = 77_u64;
    for s in 0..n {
        for k in 1..=3_u16 {
            st = mix(st);
            rows.push(Row { sess: u32::try_from(s).unwrap(), k: u32::from(k), rb: u16::try_from(st % 3).unwrap(), cb: u16::try_from((st >> 8) % 9).unwrap(), wb: u16::try_from((st >> 16) % 2).unwrap() });
        }
    }
    // three outcome tables (three "N"), nets drifting so that every eligibility reason and the gate occur
    let outs: Vec<Vec<Outcome>> = (0..3_u64)
        .map(|t| {
            rows.iter()
                .enumerate()
                .map(|(j, r)| {
                    let mut net = [0_i64; wf_sweep::MAXC];
                    let mut rd = [0_u16; wf_sweep::MAXC];
                    for c in 0..5 {
                        let h = mix(j as u64 * 31 + t * 7 + c as u64);
                        net[c] = (i64::try_from(h % 20_001).unwrap() - 8_500 - 300 * i64::try_from(t).unwrap()) * 100;
                        rd[c] = u16::try_from((h >> 20) % 600).unwrap();
                    }
                    Outcome { sess: r.sess, mk: sess_mk[r.sess as usize], id: j as u64 + 1, net, rd_bp: rd }
                })
                .collect()
        })
        .collect();
    let wins = vec![("WF".to_string(), mk(2018, 9), mk(2019, 12)), ("TT".to_string(), mk(2018, 9), mk(2022, 12))];
    let lay = Layout::new(&sess_mk, (mk(2018, 9), mk(2022, 12)), &wins).unwrap();
    let vtext = "WF-R73\tWF\t1\tTHE\tR73\t6\nTT-R73\tTT\t1\tTHE\tR73\t6\nWF-R73-F2\tWF\t2\tBESIDE\tR73\t3\nTT-R73-F2\tTT\t2\tBESIDE\tR73\t3\nWF-R73-F4\tWF\t4\tBESIDE\tR73\t3\nTT-R73-F4\tTT\t4\tBESIDE\tR73\t3\nWF-N\tWF\t1\tBESIDE\tN\t3\nTT-N\tTT\t1\tBESIDE\tN\t3\nWF-MISS\tWF\t1\tMISS\tMISS\t2\nTT-MISS\tTT\t1\tMISS\tMISS\t2\n";
    let views = wf_sweep::views::parse(vtext, &["WF".to_string(), "TT".to_string()]).unwrap();
    // dials: M-RSI, M-CCI, M-WR (the core) and one outer dial with 6 values (value 5 EMPTY by construction)
    let radix = vec![u64::from(core.nr), u64::from(core.nc), u64::from(core.nw), 7];
    let values: Vec<OuterValue> = (1..=6_u64)
        .map(|v| {
            let mut kept = Bits::new(rows.len());
            for j in 0..rows.len() {
                if !mix(j as u64 * 13 + v).is_multiple_of(3) {
                    kept.set(j);
                }
            }
            OuterValue { dial: 3, value: v, label: format!("D={v}"), kept, empty: v == 5 }
        })
        .collect();
    let refs: Vec<&[Outcome]> = outs.iter().map(Vec::as_slice).collect();
    for threads in [1_usize, 3] {
        let ctx0 = Ctx { rows: &rows, out: refs[0], lay: &lay, views: &views, cols: 5, core, sessions: n, top: 1_000, radix: &radix, core_pos: [0, 1, 2], filter: None };
        let all = run_all(&ctx0, &refs, &values, threads).unwrap();
        assert_eq!(all.len(), 3);
        // small chunks cross every chunk boundary (a chunk of 1 hands each event on alone)
        let small: Vec<Vec<Out>> = [1_usize, 7, 1_000].iter().map(|&c| run_all_with(&ctx0, &refs, &values, threads, c).unwrap()).collect();
        const { assert!(CHUNK >= 1_000) };
        for (j, o) in refs.iter().enumerate() {
            let ctx = Ctx { out: o, ..ctx0 };
            let one = run(&ctx, &values, threads).unwrap();
            same(&all[j], &one, &format!("threads {threads} table {j}"));
            for (si, s) in small.iter().enumerate() {
                same(&s[j], &one, &format!("threads {threads} table {j} small chunk {si}"));
            }
            assert!(one.not_eligible.iter().all(|&x| x > 0), "every reason occurs: {:?}", one.not_eligible);
            assert!(one.lists.iter().all(|l| !l.is_empty()), "every list holds entries");
        }
        // SECONDS PORTER (secf): the shared walk with each value's kept set made when it is walked (`run_all_kept`, the seconds
        // driver's call) equals one `run_with` per table with the same `kept_of`, at every chunk size; the made sets differ from
        // `OuterValue::kept`, so the walk is shown to read `kept_of`; `None` equals `run_all` itself.
        let kf = |vi: usize| {
            let mut b = Bits::new(rows.len());
            for j in 0..rows.len() {
                if values[vi].kept.get(j) && !mix(j as u64 * 5 + vi as u64).is_multiple_of(4) {
                    b.set(j);
                }
            }
            b
        };
        let made: Vec<Vec<Out>> = [1_usize, 7, CHUNK].iter().map(|&c| run_all_kept(&ctx0, &refs, &values, Some(&kf), threads, c).unwrap()).collect();
        let none = run_all_kept(&ctx0, &refs, &values, None, threads, CHUNK).unwrap();
        for (j, o) in refs.iter().enumerate() {
            let ctx = Ctx { out: o, ..ctx0 };
            let one = run_with(&ctx, &values, Some(&kf), threads).unwrap();
            for (si, s) in made.iter().enumerate() {
                same(&s[j], &one, &format!("threads {threads} table {j} kept_of chunk {si}"));
            }
            same(&none[j], &all[j], &format!("threads {threads} table {j} kept_of None"));
            assert_ne!(one.walk.inserts, all[j].walk.inserts, "the made kept sets are walked (they differ from OuterValue::kept)");
        }
    }
}

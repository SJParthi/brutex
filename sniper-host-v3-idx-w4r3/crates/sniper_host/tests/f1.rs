//! HOST v3 (main session 5b507203, 2026-11-14): check-build-v2 F1, the test gap at host v2's two merge points, closed with the
//! two tests check-build-v2 section 7 names. Before these tests, one-line mutants of either merged hunk passed every test:
//! - mA (`sweep.rs`, the kept-set match of `run_with`): `(Some(_), Some(f)) => f(i - 1)` read as `(Some(x), Some(_f)) => x.kept.clone()`
//!   (the seconds sweep ignores `kept_of`). Host v3 also carries the same match in `run_all_kept` (the seconds porter's shared walk,
//!   CHANGES-secf item 3), so test 1 holds BOTH to an oracle that never reads `kept_of`.
//! - mB (`search.rs`, the swept N of the minutes driver's shared walk): `.map(|(ni, _)| ni).collect()` read with `.rev()` (each N's
//!   sweep read against another N). Test 2 holds a two-N search to the two one-N searches, row by row.
#![allow(clippy::indexing_slicing, clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

use std::path::{Path, PathBuf};

use sniper_host::cands::OuterValue;
use sniper_host::sweep::{CHUNK, Ctx, Out, run, run_all_kept, run_with};
use sniper_kernel::bits::Bits;
use sniper_kernel::line::{Core, Row};
use wf_sweep::acc::{Layout, Outcome};

fn mix(x: u64) -> u64 {
    wf_sweep::mix(x.wrapping_add(0x9e37_79b9_7f4a_7c15))
}

/// Every list field the host reads, the walk counts, the step histograms, the not-eligible counts and the refusals (as
/// `tests/fast.rs`; `accepts` depends on the worker a value lands on and nothing prints it).
fn same(a: &Out, b: &Out, what: &str) {
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

/// TEST 1 (check-build-v2 section 7 item 1): `run_with(ctx, values, Some(&kept_of), t)`, where `kept_of(i)` differs from
/// `values[i].kept`, equals `run(ctx, values', t)` with `values'[i].kept = kept_of(i)` (an oracle that never reads `kept_of`); the
/// same for every table of `run_all_kept` at chunks 1 / 7 / CHUNK. The fixture is `tests/fast.rs`' (three outcome tables, random
/// kept sets, three FIRST-mode candles a session, random nets); the made sets drop AND add rows, and the oracle's walk differs
/// from the walk of `OuterValue::kept`, so a sweep that ignores `kept_of` (mutant mA, either match) fails here.
#[test]
fn f1_kept_of_equals_run_on_values_carrying_those_kept_sets() {
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
    // the made kept sets: each row's membership flipped on about one row in five (rows dropped AND rows added)
    let kf = |vi: usize| {
        let mut b = Bits::new(rows.len());
        for j in 0..rows.len() {
            if values[vi].kept.get(j) != mix(j as u64 * 11 + vi as u64 * 7).is_multiple_of(5) {
                b.set(j);
            }
        }
        b
    };
    let values2: Vec<OuterValue> = values.iter().enumerate().map(|(vi, v)| OuterValue { dial: v.dial, value: v.value, label: v.label.clone(), kept: kf(vi), empty: v.empty }).collect();
    let (mut dropped, mut added) = (0_usize, 0_usize);
    for (v, v2) in values.iter().zip(&values2) {
        for j in 0..rows.len() {
            dropped += usize::from(v.kept.get(j) && !v2.kept.get(j));
            added += usize::from(!v.kept.get(j) && v2.kept.get(j));
        }
    }
    assert!(dropped > 0 && added > 0, "the made sets drop and add rows: dropped {dropped}, added {added}");
    let refs: Vec<&[Outcome]> = outs.iter().map(Vec::as_slice).collect();
    for threads in [1_usize, 3] {
        let ctx0 = Ctx { rows: &rows, out: refs[0], lay: &lay, views: &views, cols: 5, core, sessions: n, top: 1_000, radix: &radix, core_pos: [0, 1, 2], filter: None };
        let oracles: Vec<Out> = refs.iter().map(|o| run(&Ctx { out: o, ..ctx0 }, &values2, threads).unwrap()).collect();
        for (j, o) in refs.iter().enumerate() {
            let ctx = Ctx { out: o, ..ctx0 };
            let own = run(&ctx, &values, threads).unwrap();
            assert_ne!(own.walk.inserts, oracles[j].walk.inserts, "table {j}: the made sets change the walk (a sweep that ignores kept_of is seen)");
            let got = run_with(&ctx, &values, Some(&kf), threads).unwrap();
            same(&got, &oracles[j], &format!("threads {threads} table {j}: run_with(kept_of) vs run(values')"));
            assert!(oracles[j].not_eligible.iter().all(|&x| x > 0), "every reason occurs: {:?}", oracles[j].not_eligible);
        }
        for chunk in [1_usize, 7, CHUNK] {
            let all = run_all_kept(&ctx0, &refs, &values, Some(&kf), threads, chunk).unwrap();
            assert_eq!(all.len(), refs.len());
            for (j, o) in oracles.iter().enumerate() {
                same(&all[j], o, &format!("threads {threads} table {j} chunk {chunk}: run_all_kept(kept_of) vs run(values')"));
            }
        }
    }
}

fn data() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("data")
}

/// The SN4 rows of one context's table (header dropped), each with its N (column `n`).
fn sn4_rows(out: &Path, file: &str) -> Vec<(i64, String)> {
    let t = std::fs::read_to_string(out.join("db").join(file)).unwrap();
    let mut it = t.lines();
    let head: Vec<&str> = it.next().unwrap().split('\t').collect();
    let ni = head.iter().position(|h| *h == "n").unwrap();
    it.map(|l| (l.split('\t').nth(ni).unwrap().parse().unwrap(), l.to_string())).collect()
}

/// TEST 2 (check-build-v2 section 7 item 2): the minutes driver's shared walk of every swept N (`search.rs`, one
/// `crate::sweep::run_all` walk read in the grid order) on a REAL context: a `--n 20,5` search's SN4 row of each N equals the SN4
/// row of the one-N search at that N (`--n 20`, `--n 5`), byte for byte (same run id; the sweep counts, step histograms and
/// not-eligible counts of that N's own outcome table), and the two N's rows differ, so a swap of the swept N (mutant mB) fails here.
/// Context: ADANIENT 60min LONG XS-10-30, the one-dial slice C-COL ALL (a small walk), dials OFF, every N priced. Data: the tree's
/// own data rows copied into the test's folder with ONE row changed, `swap_free_min_mb` 0 (MAIN's post-boot ruling: macOS shows 0
/// MB free swap until it swaps), and the slice file written beside them. The launch gate still holds on memory pressure 4 or a
/// disk below the floor (the out folders are in the target's own tmp folder, on the data volume); such a hold fails this test,
/// named. Nothing is removed afterwards (the folders are small; never delete).
#[test]
fn f1_swept_n_order_a_two_n_search_equals_each_one_n_search() {
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos());
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("f1-swept-{}-{stamp}", std::process::id()));
    let d = root.join("data");
    std::fs::create_dir_all(&d).unwrap();
    for e in std::fs::read_dir(data()).unwrap() {
        let e = e.unwrap();
        if !e.file_type().unwrap().is_file() {
            continue;
        }
        let name = e.file_name();
        let text = std::fs::read(e.path()).unwrap();
        if name == "SNIPER-RUN-v6.tsv" {
            let t = String::from_utf8(text).unwrap();
            let mut hit = 0;
            let rows: Vec<String> = t
                .lines()
                .map(|l| {
                    if l.starts_with("swap_free_min_mb\t") {
                        hit += 1;
                        "swap_free_min_mb\t0\tF1 test (host v3): MAIN's post-boot ruling, macOS shows 0 MB free swap until it swaps".to_string()
                    } else {
                        l.to_string()
                    }
                })
                .collect();
            assert_eq!(hit, 1, "one swap_free_min_mb row");
            std::fs::write(d.join(&name), rows.join("\n") + "\n").unwrap();
        } else {
            std::fs::write(d.join(&name), text).unwrap();
        }
    }
    std::fs::write(d.join("SLICE-F1-v1.tsv"), "# SLICE-F1-v1.tsv (host v3 F1 test): the core plus one dial, C-COL ALL.\ndial\tlabel\tsource\nC-COL\tALL\tF1 test: candle colour only\n").unwrap();
    let inp = sniper_host::esn1::load_inputs(&d).unwrap();
    let swap_now = sniper_host::monitor::swap().map_or(0, |x| x.0);
    let search = |n: &[i64], name: &str| {
        let out = root.join(name);
        std::fs::create_dir_all(&out).unwrap();
        let spec = sniper_host::search::RunSpec {
            tf: "60min".to_string(),
            sides: vec![true],
            exits: vec!["XS-10-30".to_string()],
            slice: Some(d.join("SLICE-F1-v1.tsv")),
            out: out.clone(),
            run_id: "f1-swept-n".to_string(),
            threads: 4,
            swap_base_mb: swap_now,
            data: d.clone(),
            n_sweep: Some(n.to_vec()),
            period: None,
            context_dials: false,
        };
        sniper_host::search::run_family(&inp, &spec).unwrap_or_else(|e| panic!("search --n {n:?}: {e}"));
        sn4_rows(&out, "SN4-60min-LONG-XS-10-30.tsv")
    };
    let two = search(&[20, 5], "n20-5");
    let one20 = search(&[20], "n20");
    let one5 = search(&[5], "n5");
    assert_eq!(two.iter().map(|x| x.0).collect::<Vec<_>>(), vec![5, 20], "the two-N search's SN4 rows, in the grid order");
    assert_eq!(one20.len(), 1);
    assert_eq!(one5.len(), 1);
    assert_eq!(two[1], one20[0], "N = 20: the two-N search's row equals the one-N search's");
    assert_eq!(two[0], one5[0], "N = 5: the two-N search's row equals the one-N search's");
    // the two N's own sweeps differ beyond the N column, so a swap would be seen
    let body = |r: &str| r.split('\t').enumerate().filter(|(i, _)| *i != 3).map(|(_, c)| c.to_string()).collect::<Vec<_>>();
    assert_ne!(body(&two[0].1), body(&two[1].1), "the sweeps at N = 5 and N = 20 differ");
}

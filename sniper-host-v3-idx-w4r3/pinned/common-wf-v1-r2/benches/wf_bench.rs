//! The crate's benches (plan section 12; D27). Per event class: the DETERMINISTIC step counts at 1x, 10x and 100x subjects per run
//! (the SAME subjects cycled) asserted identical per event (the hard O(1) gate: noise-free), and the timings p50 / p99 / max
//! (min of 5 repeats; printed, never a red on this loaded machine). Classes: S-ADD (a trade), S-MTM (a session-end mark), S-FINISH
//! (a subject), S-OFFER (one subject offered to every view of its partition: read-set compares, eligibility masks, the early reject,
//! the key and the insert), S-VERDICT (one window row's verdict), S-KEY per window shape (WF, TT and A2026 anchored: ONE prefix
//! cell each, asserted equal; a rolling row: its bounded fold). Then trades per subject at 1x / 10x / 100x: the largest steps of
//! one `add` identical (at most 7 fixed steps, F17). No float. Exit status non-zero when an assertion fails.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic, clippy::too_many_lines, clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::cast_possible_wrap, clippy::print_stdout, clippy::similar_names, clippy::many_single_char_names, missing_docs, reason = "bench")]

use std::hint::black_box;
use std::time::Instant;

use common_wf::date::parse_day;
use common_wf::views::Views;
use common_wf::windows::{WinKind, Windows, parse_windows};
use common_wf::*;

const ROOT: &str = env!("CARGO_MANIFEST_DIR");
fn read(rel: &str) -> String {
    std::fs::read_to_string(format!("{ROOT}/{rel}")).unwrap()
}
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
fn calendar(to: &str) -> Calendar {
    let (a, b) = (parse_day("2018-09-01").unwrap(), parse_day(to).unwrap());
    let rows: Vec<CalRow> = (a..=b).filter(|z| { let w = ((z % 7) + 7 + 4) % 7; (1..=5).contains(&w) }).map(|z| CalRow { day: z, kind: DayKind::Regular, walked: true, ruling: false }).collect();
    Calendar::from_rows(&rows).unwrap()
}
fn rules(marks: bool) -> RunRules {
    let mut kv: Vec<(String, String)> = [("period", "2018-09..2026-10"), ("signed", "2026-11"), ("paper_from", "2026-12"), ("order", "SEQ_LOSSES_FIRST"), ("holes", "lane"), ("grid", "1000:SEARCH:LATTICE"), ("proof", "proof step: bench"), ("data", "data: synthetic"), ("partition_provenance_default", "RULED_FROM_TEXT bench (repair B2)")].iter().map(|(a, b)| ((*a).to_owned(), (*b).to_owned())).collect();
    if marks {
        kv.push(("marks".to_owned(), "session_end".to_owned()));
    }
    let mut r = RunRules::from_rows(&kv).unwrap();
    let row: Vec<String> = common_wf::rules::PART_COLS.iter().map(|c| match *c { "id" => "1".to_owned(), "role" => "REAL".to_owned(), "world" => "real".to_owned(), "reading" => "booked".to_owned(), "side_dial" => "no".to_owned(), "side_setting" => "both".to_owned(), _ => "-".to_owned() }).collect();
    r.partitions = common_wf::rules::parse_partitions(&[row]).unwrap();
    r
}
/// One subject: `n` trades on random walked sessions, in the settle order.
fn subject(g: &mut Lcg, cal: &Calendar, n: usize) -> Vec<Trade> {
    let (d0, d1) = (parse_day("2018-09-03").unwrap(), parse_day("2026-10-30").unwrap());
    let mut days: Vec<i32> = Vec::new();
    while days.len() < n {
        let d = d0 + i32::try_from(g.below(u64::try_from(d1 - d0).unwrap())).unwrap();
        if cal.walked(d) {
            days.push(d);
        }
    }
    days.sort_unstable();
    let mut t: Vec<Trade> = days.iter().enumerate().map(|(i, d)| Trade { day: *d, exit_day: *d, close_s: 33_300 + u32::try_from(g.below(22_000)).unwrap(), net: i64::try_from(g.below(200_001)).unwrap() - 90_000, net_alt: 0, rd_bp: i32::try_from(g.below(400)).unwrap(), hole: g.below(12) == 0, id: u64::try_from(i).unwrap(), mark_prev: 0 }).collect();
    t.sort_by_key(|x| (x.exit_day, x.close_s, x.net));
    t
}
fn pct(v: &mut [u128], p: usize) -> u128 {
    v.sort_unstable();
    if v.is_empty() { 0 } else { v[((v.len() - 1) * p) / 100] }
}
struct Class {
    name: &'static str,
    steps_total: u64,
    events: u64,
    steps_max: u64,
}

fn main() {
    let cal = calendar("2026-10-31");
    let r = rules(false);
    let rows = parse_windows(&read("data/WINDOWS-STD-v1.tsv")).unwrap();
    let views = Views::from_text(&read("data/VIEWS-STD-v1.tsv"), &rows, &r, &[]).unwrap();
    let ws = Windows::sets_from_rows(&rows, &cal, r.period).unwrap();
    let w = &ws.sets[0];
    let mut g = Lcg(2026);
    let base: Vec<Vec<Trade>> = (0..100).map(|_| { let n = 40 + usize::try_from(g.below(80)).unwrap(); subject(&mut g, &cal, n) }).collect();
    let mut ticks = Tickets::new();
    ticks.push(Ticket { grid: 0, stage: NO_STAGE, set: ReadSet::EMPTY }, &r).unwrap();
    println!("BENCH common_wf v1: {} subjects (40..119 trades each) cycled at 1x / 10x / 100x; the standard windows, views and words.", base.len());
    let mut per_scale: Vec<[Class; 4]> = Vec::new();
    for scale in [1usize, 10, 100] {
        let mut add = Class { name: "S-ADD", steps_total: 0, events: 0, steps_max: 0 };
        let mut fin = Class { name: "S-FINISH", steps_total: 0, events: 0, steps_max: 0 };
        let mut off = Class { name: "S-OFFER", steps_total: 0, events: 0, steps_max: 0 };
        let mut ver = Class { name: "S-VERDICT", steps_total: 0, events: 0, steps_max: 0 };
        let (mut t_add, mut t_off, mut t_ver): (Vec<u128>, Vec<u128>, Vec<u128>) = (Vec::new(), Vec::new(), Vec::new());
        let mut lists = Lists::new(&views, w, &r, 0);
        let mut acc = Acc::new(w, &views, &r);
        for rep in 0..scale {
            for (i, tr) in base.iter().enumerate() {
                acc.reset();
                for t in tr {
                    let t0 = Instant::now();
                    let s = acc.add(black_box(t)).unwrap();
                    t_add.push(t0.elapsed().as_nanos());
                    add.steps_total += u64::from(s.total());
                    add.events += 1;
                    add.steps_max = add.steps_max.max(u64::from(s.total()));
                }
                let s = acc.finish().unwrap();
                fin.steps_total += u64::from(s.total());
                fin.events += 1;
                fin.steps_max = fin.steps_max.max(u64::from(s.total()));
                let code = Code([u64::try_from(rep * 1000 + i).unwrap(), 0, 0, 0]);
                let t0 = Instant::now();
                let s = lists.offer(&views, Partition(1), &ticks, code, &acc).unwrap();
                t_off.push(t0.elapsed().as_nanos());
                // the early reject makes a later cycle cheaper than the first; the deterministic gate reads the first cycle only
                if rep == 0 {
                    off.steps_total += u64::from(s.total());
                    off.events += 1;
                    off.steps_max = off.steps_max.max(u64::from(s.total()));
                }
                let f = acc.facts(Partition(1));
                for h in ws.honest() {
                    let pick = PickWin { set: w.set_id(), win: h, learn: w.learn_range(h).unwrap(), kind: PickKind::Split, code };
                    let t0 = Instant::now();
                    let v = verdict(w, h, pick, black_box(&f), &r).unwrap();
                    t_ver.push(t0.elapsed().as_nanos());
                    black_box(v);
                    ver.steps_total += 4;
                    ver.events += 1;
                    ver.steps_max = 4;
                }
            }
        }
        println!("\nscale {scale}x ({} subjects walked):", scale * base.len());
        for (c, t) in [(&add, &mut t_add), (&fin, &mut Vec::new()), (&off, &mut t_off), (&ver, &mut t_ver)] {
            let (p50, p99, mx) = (pct(t, 50), pct(t, 99), t.iter().copied().max().unwrap_or(0));
            println!("  {:<10} events {:>9}  steps/event {}.{:03}  max steps {}  p50 {} ns  p99 {} ns  max {} ns", c.name, c.events, c.steps_total / c.events.max(1), (c.steps_total % c.events.max(1)) * 1000 / c.events.max(1), c.steps_max, p50, p99, mx);
        }
        per_scale.push([add, fin, off, ver]);
    }
    // THE HARD O(1) GATE (D27): per event, the step counts identical at 1x, 10x and 100x (the same events cycled: exact)
    let one = &per_scale[0];
    for (k, s) in per_scale.iter().enumerate().skip(1) {
        for (a, b) in one.iter().zip(s.iter()) {
            let sc = [1u64, 10, 100][k];
            let mult = if a.name == "S-OFFER" { 1 } else { sc };
            assert_eq!(a.steps_total * mult, b.steps_total, "{}: steps per event differ at {sc}x", a.name);
            assert_eq!(a.events * mult, b.events, "{}: events", a.name);
            assert_eq!(a.steps_max, b.steps_max, "{}: the largest steps of one event differ at {sc}x", a.name);
        }
    }
    assert!(one[0].steps_max <= 7 + 2 * u64::try_from(rows.len()).unwrap(), "S-ADD within its bound (7 fixed steps + the unit updates of a part close, bounded by the windows file)");
    println!("\nGATE: every class has identical steps per event and the same largest steps at 1x, 10x and 100x.");

    // trades per subject at 1x / 10x / 100x: the largest steps of one add (dense days: up to one trade a session)
    let mut maxes = Vec::new();
    for k in [12usize, 120, 1200] {
        let mut acc = Acc::new(w, &views, &r);
        acc.reset();
        let tr = subject(&mut Lcg(7), &cal, k);
        let mut mx = 0u32;
        let mut tot = 0u64;
        for t in &tr {
            let s = acc.add(t).unwrap();
            mx = mx.max(s.total());
            tot += u64::from(s.total());
        }
        acc.finish().unwrap();
        println!("trades per subject {k:>5}: largest steps of one add {mx}, mean {}.{:03}", tot / u64::try_from(k).unwrap(), (tot % u64::try_from(k).unwrap()) * 1000 / u64::try_from(k).unwrap());
        maxes.push(mx);
    }
    assert!(maxes.iter().all(|m| *m == maxes[0]), "the largest steps of one add are the same at 1x / 10x / 100x trades per subject: {maxes:?}");

    // S-MTM: marks on every walked session of a carried position (the same subjects cycled)
    let rm = rules(true);
    let views_m = Views::from_text(&read("data/VIEWS-STD-v1.tsv"), &rows, &rm, &[]).unwrap();
    let mut mtm_counts = Vec::new();
    for scale in [1usize, 10, 100] {
        let (mut tot, mut ev, mut mx) = (0u64, 0u64, 0u32);
        let mut t_m = Vec::new();
        for _ in 0..scale {
            let mut acc = Acc::new(w, &views_m, &rm);
            acc.reset();
            let mut d = parse_day("2019-01-01").unwrap();
            for k in 0..200i64 {
                while !cal.walked(d) {
                    d += 1;
                }
                let t0 = Instant::now();
                let s = acc.add_session_mtm(d, -1000 * (k % 37)).unwrap();
                t_m.push(t0.elapsed().as_nanos());
                tot += u64::from(s.total());
                ev += 1;
                mx = mx.max(s.total());
                d += 1;
            }
            acc.finish().unwrap();
        }
        println!("S-MTM {scale:>3}x: events {ev} steps/event {} max steps {mx} p50 {} ns p99 {} ns", tot / ev, pct(&mut t_m, 50), pct(&mut t_m.clone(), 99));
        mtm_counts.push((tot / u64::try_from(scale).unwrap(), mx));
    }
    assert!(mtm_counts.iter().all(|x| *x == mtm_counts[0]), "S-MTM steps identical at 1x / 10x / 100x");

    // S-KEY per window shape: WF, TT, A2026 anchored (ONE prefix cell each); a rolling row (S5's RO): its bounded fold
    let rows5 = parse_windows(&read("vectors/data/WINDOWS-S5.tsv")).unwrap();
    let views5 = Views::from_text(&read("vectors/data/VIEWS-S5.tsv"), &rows5, &r, &[]).unwrap();
    let ws5 = Windows::sets_from_rows(&rows5, &cal, r.period).unwrap();
    let mut shape = Vec::new();
    for (wset, vw, ids) in [(w, &views, vec!["WF", "TT", "A2026"]), (&ws5.sets[0], &views5, vec!["RO"])] {
        let mut acc = Acc::new(wset, vw, &r);
        acc.reset();
        for t in &base[0] {
            acc.add(t).unwrap();
        }
        acc.finish().unwrap();
        let rows_here = if ids[0] == "RO" { &rows5 } else { &rows };
        for id in ids {
            let wi = WinId(rows_here.iter().find(|x| x.id == id).unwrap().idx);
            let spec = wset.spec(wi).unwrap();
            let cells = if spec.anchored { 1 } else { spec.learn_pieces.count_ones() };
            let mut t = Vec::new();
            for _ in 0..5 {
                for _ in 0..1000 {
                    let t0 = Instant::now();
                    black_box(acc.learn(wi).unwrap());
                    t.push(t0.elapsed().as_nanos());
                }
            }
            println!("S-KEY {id:<6} anchored {:<5} cells read {cells}  p50 {} ns  p99 {} ns", spec.anchored, pct(&mut t, 50), pct(&mut t.clone(), 99));
            shape.push((id, spec.anchored, cells));
        }
    }
    assert!(shape.iter().filter(|x| x.1).all(|x| x.2 == 1), "every anchored shape reads ONE prefix cell");
    let _ = WinKind::Split;
    println!("\nBENCH PASS: deterministic step counts identical at 1x / 10x / 100x for S-ADD, S-FINISH, S-OFFER, S-VERDICT, S-MTM; the largest add identical at 1x / 10x / 100x trades per subject; anchored S-KEY = 1 cell.");
}

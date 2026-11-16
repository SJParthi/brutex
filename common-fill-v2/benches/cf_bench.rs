//! The crate's benches (design v2 section 11; D27). Per event class: the DETERMINISTIC step counts at 1x, 10x and 100x a day's events over
//! the SAME working set (the same tapes, more events), asserted identical per event (the hard O(1) gate: noise-free), and the timings
//! p50 / p99 / max (min of 5 repeats; printed, never a red on this loaded machine), with the load average and the profile; then the
//! printed checks of gates 2 and 3 (the timing ratio and the absolute budgets; never a red, D27). `FillPath`'s `view` / `payable` /
//! `cut_at` (the named O(log blocks) exception, D27): ONE search a call, at most 14 probes at 11,250 blocks, asserted per call;
//! `cut_at`'s one O(1) straddling-block re-read is a block read, counted in its `Steps` and asserted at most 1 a call. No float.
//! Then the DB rows of design section 12 for E1 (KEI 2026-10-22, the real seconds of the vectors file): every table's frozen header and
//! its rows (G0 evidence). Exit status non-zero when an assertion fails.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic, clippy::too_many_lines, clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::cast_possible_wrap, clippy::cast_precision_loss, clippy::similar_names, clippy::many_single_char_names, clippy::too_many_arguments, clippy::print_stdout, clippy::type_complexity, clippy::cast_lossless, clippy::missing_docs_in_private_items, clippy::needless_range_loop, missing_docs, reason = "bench")]

use std::hint::black_box;
use std::time::Instant;

use common_fill::legs::walk_pair;
use common_fill::rows::{self, Ctx, Rec, T1, T4, T5, T6, T7, header};
use common_fill::{BlockFill, Dial, FillPath, GapTables, Kind, LevelFill, LevelMask, MAXL, Order, RangeMm, SecIn, SecIndex, Side, Tape, US_S, Us, Why, path, walk};

const BASE: Us = 1_793_000_000 * US_S;
const DAY: i64 = 22_500;
const LV: [u16; 5] = [1_000, 2_500, 5_000, 7_500, 10_000];

fn t(s: i64) -> Us {
    BASE + s * US_S
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

/// A dense day (every second traded) and a thin option day (5 units a second, a liquid second every 600).
fn days() -> (Vec<SecIn>, Vec<SecIn>) {
    let mut g = Lcg(42);
    let mut px = 100_000_i64;
    let dense = (0..DAY)
        .map(|s| {
            px = (px + g.below(41) as i64 - 20).max(1_000);
            SecIn { ts: t(s), high: px + 5 * (s % 3), low: px, vol: 100 + g.below(900) }
        })
        .collect();
    let thin = (0..DAY).map(|s| SecIn { ts: t(s), high: 5_000, low: 5_000, vol: if s % 600 == 599 { 4_000 } else { 5 } }).collect();
    (dense, thin)
}

/// One class's printed checks (design 11 gates 2 and 3): its p99 at 1x and 100x, its budget (ns per unit) and its unit count per event.
struct Check {
    name: String,
    p99_1: u64,
    p99_100: u64,
    budget: Option<u64>,
    per: u64,
}

/// `x` in units of 10^-places, printed with `places` decimals (integer arithmetic only).
fn dec(x: u64, places: u32) -> String {
    let d = 10_u64.pow(places);
    format!("{}.{:0w$}", x / d, x % d, w = places as usize)
}

fn pct(v: &mut [u64], p: usize) -> u64 {
    v.sort_unstable();
    if v.is_empty() { 0 } else { v[((v.len() - 1) * p) / 100] }
}

/// One class at three scales: `run(k)` performs one event k (0-based, k < n x scale cycles the same 1x events) and returns its steps.
/// Asserts the per-event steps identical at 1x / 10x / 100x (unless `printed_only`), prints the steps and p50 / p99 / max timings, and
/// keeps the class's printed checks: `budget` (design 11's p99 budget, ns per unit; None = printed only) over `per` units an event.
fn class(name: &str, n: usize, printed_only: bool, (budget, per): (Option<u64>, u64), checks: &mut Vec<Check>, mut run: impl FnMut(usize) -> u64) {
    let mut per_scale = Vec::new();
    for scale in [1_usize, 10, 100] {
        let events = n * scale;
        let mut best: Option<Vec<u64>> = None;
        let mut steps_total = 0_u64;
        let mut steps_max = 0_u64;
        for rep in 0..5 {
            let mut ns = Vec::with_capacity(events);
            let mut tot = 0_u64;
            let mut mx = 0_u64;
            for e in 0..events {
                let t0 = Instant::now();
                let s = run(e % n);
                ns.push(u64::try_from(t0.elapsed().as_nanos()).unwrap_or(u64::MAX));
                tot += s;
                mx = mx.max(s);
            }
            if rep == 0 {
                steps_total = tot;
                steps_max = mx;
            } else {
                assert_eq!((tot, mx), (steps_total, steps_max), "{name}: the steps are deterministic run to run");
            }
            let p99 = pct(&mut ns.clone(), 99);
            if best.as_ref().is_none_or(|b| pct(&mut b.clone(), 99) > p99) {
                best = Some(ns);
            }
        }
        let mut b = best.unwrap();
        let (p50, p99, mx) = (pct(&mut b, 50), pct(&mut b, 99), b.iter().copied().max().unwrap_or(0));
        per_scale.push((scale, events, steps_total, steps_max, p50, p99, mx));
    }
    let (_, n1, s1, m1, _, p99_1, _) = per_scale[0];
    for &(scale, events, st, mx, p50, p99, maxns) in &per_scale {
        let per_ev = u64::try_from(u128::from(st) * 10_000 / u128::from(events as u64)).unwrap();
        println!("| {name} | {scale}x | {events} | {st} | {} | {mx} | {p50} | {p99} | {maxns} | {} |", dec(per_ev, 4), dec(p99 * 100 / p99_1.max(1), 2));
        if !printed_only {
            // THE HARD O(1) GATE (D27): per event, the step counts identical at 1x, 10x and 100x (the same events cycled: exact).
            assert_eq!(st * n1 as u64, s1 * events as u64, "{name}: steps per event differ at {scale}x");
            assert_eq!(mx, m1, "{name}: the largest steps of one event differ at {scale}x");
        }
    }
    checks.push(Check { name: name.to_owned(), p99_1, p99_100: per_scale[2].5, budget, per });
}

/// Design 11 gates 2 and 3, PRINTED (never a red, D27): p99 at 100x / p99 at 1x <= 1.50 (min of 5), and each class's p99 per unit
/// against its budget; a p99 above twice its budget is a named follow-up for MAIN. The flag reads the 100x p99 (the larger sample).
fn print_checks(checks: &[Check]) {
    println!("\n## Printed checks (design 11 gates 2 and 3; never a red, D27)\n");
    println!("| class | p99 at 1x ns | p99 at 100x ns | ratio 100x / 1x | ratio <= 1.50 | budget ns per unit | units an event | p99 at 1x per unit ns | p99 at 100x per unit ns | within budget at 100x | above twice the budget at 100x (named follow-up for MAIN) |\n|---|---|---|---|---|---|---|---|---|---|---|");
    for c in checks {
        let ratio = c.p99_100 * 100 / c.p99_1.max(1);
        let (u1, u100) = (c.p99_1 * 100 / c.per.max(1), c.p99_100 * 100 / c.per.max(1));
        let (b, within, above) = match c.budget {
            Some(b) => (b.to_string(), if u100 <= b * 100 { "yes" } else { "no" }, if u100 > 2 * b * 100 { "YES" } else { "no" }),
            None => ("printed only".to_owned(), "-", "-"),
        };
        println!("| {} | {} | {} | {} | {} | {b} | {} | {} | {} | {within} | {above} |", c.name, c.p99_1, c.p99_100, dec(ratio, 2), if ratio <= 150 { "yes" } else { "no" }, c.per, dec(u1, 2), dec(u100, 2));
    }
}

fn load() -> String {
    std::process::Command::new("/usr/sbin/sysctl").args(["-n", "vm.loadavg"]).output().ok().map_or_else(|| "-".to_owned(), |o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
}

fn main() {
    println!("# common_fill crate v2 benches (design v2 section 11): profile bench (release, overflow checks on); load average at the start {}", load());
    // The per-event timer: one tick of Instant, measured as the smallest positive step between back-to-back reads.
    let tick = (0..10_000)
        .filter_map(|_| {
            let a = Instant::now();
            let mut b = Instant::now();
            while b <= a {
                b = Instant::now();
            }
            u64::try_from((b - a).as_nanos()).ok()
        })
        .min()
        .unwrap_or(0);
    println!("timer: one tick of Instant = {tick} ns (measured); a per-event timing below one tick reads 0 or one tick, so a budget under one tick is not resolved per event");
    let (dense, thin) = days();
    let ixd = SecIndex::new(&dense, BASE, t(DAY)).unwrap();
    let ixt = SecIndex::new(&thin, BASE, t(DAY)).unwrap();
    let mmd = RangeMm::new(&dense);
    let dial = Dial::new(&LV).unwrap();
    let td0 = Tape::new(&dense, &ixd, Some(&mmd)).unwrap();
    let gd = GapTables::new(&td0, &dial, 1).unwrap();
    let td = td0.with_gaps(&gd).unwrap();
    let dial10 = Dial::new(&[1_000]).unwrap();
    let tt0 = Tape::new(&thin, &ixt, None).unwrap();
    let gt = GapTables::new(&tt0, &dial10, 75).unwrap();
    let tt = tt0.with_gaps(&gt).unwrap();
    let mut checks: Vec<Check> = Vec::new();
    println!("\n| class | scale | events | steps total | steps per event | max steps of one event | p50 ns | p99 ns | max ns | p99 / p99 at 1x |\n|---|---|---|---|---|---|---|---|---|---|");

    // B-TAPE: the day's index, range table and gap tables, per session second (amortised; one event = one day).
    class("B-TAPE (index + range table + gap tables, per day of 22,500 s)", 1, false, (Some(30), 22_500), &mut checks, |_| {
        let ix = SecIndex::new(&dense, BASE, t(DAY)).unwrap();
        let mm = RangeMm::new(&dense);
        let tp = Tape::new(&dense, &ix, Some(&mm)).unwrap();
        let g = GapTables::new(&tp, &dial, 1).unwrap();
        black_box((g.tables(), ix.rows()));
        u64::try_from(dense.len()).unwrap()
    });
    // B-BLOCK: Tape::block at 1,000 lattice instants: steps = the rows read (two seconds) + one index read.
    let mut g = Lcg(7);
    let at_b: Vec<i64> = (0..1_000).map(|_| g.below(DAY as u64 - 4) as i64).collect();
    class("B-BLOCK (Tape::block: <= 2 seconds' rows)", 1_000, false, (Some(40), 1), &mut checks, |k| {
        let b = td.block(t(at_b[k]), t(DAY)).unwrap();
        black_box(b.vol);
        1 + u64::from(b.n)
    });
    // B-JUMP: next_traded from 1,000 instants (one index read).
    class("B-JUMP (next_traded)", 1_000, false, (Some(20), 1), &mut checks, |k| {
        black_box(tt.next_traded(t(at_b[k])));
        1
    });
    // B-GAP: a 1-lot (75) order at 10% on the thin day: the zero-capacity runs skipped (steps = blocks read + table reads + skips).
    class("B-GAP (a 1-lot order through 299 zero-capacity blocks: one skip)", 200, false, (None, 1), &mut checks, |k| {
        let a = (k as i64 % 30) * 600;
        let o = Order { side: Side::Buy, anchor: t(a), stop: t(a + 1_200), unit: 75, limit: None, kind: Kind::Entry };
        let mut w = [75, 0, 0, 0, 0, 0, 0, 0];
        let mut f = [LevelFill::default(); MAXL];
        walk(&tt, &o, &dial10, LevelMask::all(1), &mut w, None, None, &mut f, None).unwrap();
        assert!(f[0].steps.skips >= 1 && f[0].units == 75);
        u64::from(f[0].steps.blocks + f[0].steps.table_reads + f[0].steps.skips)
    });
    // B-WALK-BLOCK: an order filling in ONE block at all 5 levels (steps = blocks read over the levels).
    class("B-WALK-BLOCK (one block, 5 levels, one pass)", 1_000, false, (Some(150), 1), &mut checks, |k| {
        let o = Order { side: Side::Sell, anchor: t(at_b[k]), stop: t(DAY), unit: 1, limit: None, kind: Kind::Entry };
        let mut w = [10, 10, 10, 10, 10, 0, 0, 0];
        let mut f = [LevelFill::default(); MAXL];
        walk(&td, &o, &dial, LevelMask::all(5), &mut w, None, None, &mut f, None).unwrap();
        f.iter().take(5).map(|x| u64::from(x.steps.blocks)).sum()
    });
    // B-TAKE: one level's take in one block (the dial step), money off and money on (REQ 70a's payable check). The take is private to the
    // walk, so one event is a 5-level walk of ONE block and its unit is one level (the block read and the set-up shared over the 5).
    let pay_on = [Some(i64::MAX / 4); MAXL];
    for (label, money) in [("money off", None), ("money on", Some(&pay_on))] {
        class(&format!("B-TAKE (one level's take in one block, {label}; a 5-level one-block walk, per level)"), 1_000, false, (Some(10), 5), &mut checks, |k| {
            let o = Order { side: Side::Buy, anchor: t(at_b[k]), stop: t(at_b[k] + 2), unit: 1, limit: None, kind: Kind::Entry };
            let mut w = [5, 5, 5, 5, 5, 0, 0, 0];
            let mut f = [LevelFill::default(); MAXL];
            walk(&td, &o, &dial, LevelMask::all(5), &mut w, money, None, &mut f, None).unwrap();
            assert!(f.iter().take(5).all(|x| x.units == 5));
            f.iter().take(5).map(|x| u64::from(x.steps.blocks)).sum()
        });
    }
    // B-WALK-ORDER: random orders (output-sensitive: printed only, with their block counts).
    let ord_w: Vec<(i64, i64, u64)> = (0..500).map(|_| (g.below(DAY as u64 - 3_000) as i64, 1 + g.below(2_000) as i64, 1 + g.below(20_000))).collect();
    class("B-WALK-ORDER (one order, 5 levels; output-sensitive)", 500, true, (None, 1), &mut checks, |k| {
        let (a, len, q) = ord_w[k];
        let o = Order { side: Side::Buy, anchor: t(a), stop: t(a + len), unit: 1, limit: None, kind: Kind::Entry };
        let mut w = [q; MAXL];
        let mut f = [LevelFill::default(); MAXL];
        walk(&td, &o, &dial, LevelMask::all(5), &mut w, None, None, &mut f, None).unwrap();
        f.iter().take(5).map(|x| u64::from(x.steps.blocks + x.steps.jumps + x.steps.skips)).sum()
    });
    // B-EXTREME: Tape::extreme through the range table (2 reads).
    class("B-EXTREME (Tape::extreme, the range table)", 1_000, false, (Some(40), 1), &mut checks, |k| {
        black_box(td.extreme(t(at_b[k]), t(at_b[k] + 1 + (k as i64 % 3_000)), k % 2 == 0));
        2
    });
    // B-PATH-BUILD: path::scan's build for one (trade, level group). Crate v2 builds no per-second arrays (the rules descend the tape's
    // range table, B-RULE); its build is R(s)'s pieces, one pass over the entry's filling blocks, plus the facts' two extremes. One event:
    // an entry of 10 filling blocks (100 units at 10% of a flat 50-a-second tape), no rule, no dial value; its unit is one filling block.
    let flat: Vec<SecIn> = (0..DAY).map(|s| SecIn { ts: t(s), high: 10_000 + s % 5, low: 10_000 - s % 3, vol: 50 }).collect();
    let ixf = SecIndex::new(&flat, BASE, t(DAY)).unwrap();
    let mmf = RangeMm::new(&flat);
    let tf = Tape::new(&flat, &ixf, Some(&mmf)).unwrap();
    let entries10: Vec<(Us, Us, Vec<BlockFill>)> = (0..1_000)
        .map(|k| {
            let a = at_b[k] % (DAY - 700);
            let o = Order { side: Side::Buy, anchor: t(a), stop: t(a + 600), unit: 1, limit: None, kind: Kind::Entry };
            let mut w = [100, 0, 0, 0, 0, 0, 0, 0];
            let mut f = [LevelFill::default(); MAXL];
            let mut fills = Vec::new();
            {
                let mut rec = |_l: usize, b: &BlockFill| {
                    if b.why == Why::Filled {
                        fills.push(*b);
                    }
                };
                walk(&tf, &o, &dial10, LevelMask::all(1), &mut w, None, None, &mut f, Some(&mut rec)).unwrap();
            }
            assert_eq!(fills.len(), 10);
            (f[0].e0, f[0].e1, fills)
        })
        .collect();
    class("B-PATH-BUILD (path::scan_one's build: R(s) pieces over 10 filling blocks + 2 extremes; per filling block)", 1_000, false, (Some(20), 10), &mut checks, |k| {
        let (e0, e1, fills) = &entries10[k];
        let e = path::EntryView { e0: *e0, e1: *e1, fills };
        let mut st = 0_u32;
        black_box(path::scan_one(&tf, &e, e1 + 600 * US_S, true, &[], &[], &mut st));
        u64::try_from(fills.len()).unwrap() + 2 + u64::from(st)
    });
    // B-RULE / B-PATH: path::scan_one with 5 rules and 2 dial x after a one-block entry (steps = its probes and seconds read).
    class("B-RULE (path::scan_one: 5 rules, 2 x; O(log n) descents, the trail one pass)", 300, false, (None, 1), &mut checks, |k| {
        let a = at_b[k] % (DAY - 700);
        let o = Order { side: Side::Buy, anchor: t(a), stop: t(DAY), unit: 1, limit: None, kind: Kind::Entry };
        let mut w = [50, 0, 0, 0, 0, 0, 0, 0];
        let mut f = [LevelFill::default(); MAXL];
        let mut fills = Vec::new();
        {
            let mut rec = |_l: usize, b: &BlockFill| {
                if b.why == Why::Filled {
                    fills.push(*b);
                }
            };
            walk(&td, &o, &Dial::new(&[10_000]).unwrap(), LevelMask::all(1), &mut w, None, None, &mut f, Some(&mut rec)).unwrap();
        }
        let e = path::EntryView { e0: f[0].e0, e1: f[0].e1, fills: &fills };
        let rules = [path::Rule::Stop { x: 300 }, path::Rule::Target { x: 300 }, path::Rule::TimeExit { secs: 300 }, path::Rule::GoPositive { secs: 60 }, path::Rule::Trail { act: 200, dist: 100 }];
        let mut st = 0_u32;
        let (tg, fc) = path::scan_one(&td, &e, t(a + 600), true, &rules, &[100, 300], &mut st);
        black_box((tg, fc));
        u64::from(st)
    });
    // B-PATH-VIEW: FillPath view / payable / cut_at over 11,250 recorded blocks (the named exception: <= 14 probes a call, asserted).
    let one: Vec<SecIn> = (0..DAY).filter(|s| s % 2 == 0).map(|s| SecIn { ts: t(s), high: 10_000 + s % 7, low: 10_000, vol: 1 }).collect();
    let ix1 = SecIndex::new(&one, BASE, t(DAY)).unwrap();
    let t1 = Tape::new(&one, &ix1, None).unwrap();
    let o1 = Order { side: Side::Buy, anchor: BASE, stop: t(DAY), unit: 1, limit: None, kind: Kind::Entry };
    let fp = FillPath::record(&t1, &o1, &Dial::new(&[10_000]).unwrap(), LevelMask::all(1), &[11_250, 0, 0, 0, 0, 0, 0, 0], None).unwrap();
    assert_eq!(fp.blocks(0), 11_250);
    let qs: Vec<u64> = (0..1_000).map(|_| 1 + g.below(11_250)).collect();
    let mut worst = (0_u32, 0_u32, 0_u32, 0_u32);
    class("B-PATH-VIEW (FillPath view + payable + cut_at at 11,250 blocks; O(log blocks), the named exception)", 1_000, true, (Some(200), 3), &mut checks, |k| {
        let q = qs[k];
        let (mut a, mut b, mut c) = (0_u32, 0_u32, 0_u32);
        black_box(fp.view(0, q, &mut a).unwrap());
        black_box(fp.payable(0, q, i64::try_from(q).unwrap() * 5_000, &mut b).unwrap());
        let cf = black_box(fp.cut_at(&t1, 0, q, t(i64::try_from(q).unwrap() + 1), &mut c).unwrap());
        let reread = cf.steps.blocks;
        assert!(a <= 14 && b <= 14 && c <= 14 && reread <= 1, "FillPath probes {a} / {b} / {c}, re-reads {reread}: at most 14 a call at 11,250 blocks (D27)");
        worst = (worst.0.max(a), worst.1.max(b), worst.2.max(c), worst.3.max(reread));
        u64::from(a + b + c + reread)
    });
    println!("\nFillPath probes, the largest of one call (one search a call, D27): view {} (<= 14), payable {} (<= 14), cut_at {} (<= 14); cut_at's O(1) block re-reads, the largest of one call: {} (<= 1, counted in its Steps)", worst.0, worst.1, worst.2, worst.3);
    // B-TRADE: one trade = the entry pass (5 levels, one walk) + the path groups (path::scan: one pass per distinct entry) + the exit
    // groups (one exit walk per distinct exit anchor); printed only, with its group counts (design 4.6; critic B B2).
    let rules5 = [path::Rule::Stop { x: 300 }, path::Rule::Target { x: 300 }, path::Rule::TimeExit { secs: 300 }, path::Rule::GoPositive { secs: 60 }, path::Rule::Trail { act: 200, dist: 100 }];
    let mut groups = (usize::MAX, 0_usize, usize::MAX, 0_usize);
    class("B-TRADE (entry pass + path groups + exit groups, 5 levels; printed with the groups)", 300, true, (None, 1), &mut checks, |k| {
        let a = at_b[k] % (DAY - 1_300);
        let o = Order { side: Side::Buy, anchor: t(a), stop: t(a + 600), unit: 1, limit: None, kind: Kind::Entry };
        let mut w = [100, 200, 400, 800, 1_600, 0, 0, 0];
        let mut f = [LevelFill::default(); MAXL];
        let mut fills: [Vec<BlockFill>; MAXL] = Default::default();
        {
            let mut rec = |l: usize, b: &BlockFill| {
                if b.why == Why::Filled {
                    fills[l].push(*b);
                }
            };
            walk(&td, &o, &dial, LevelMask::all(5), &mut w, None, None, &mut f, Some(&mut rec)).unwrap();
        }
        let entries: [Option<path::EntryView<'_>>; MAXL] = std::array::from_fn(|l| (f[l].units > 0).then_some(path::EntryView { e0: f[l].e0, e1: f[l].e1, fills: &fills[l] }));
        let to = [t(a + 1_200); MAXL];
        let mut trig: [Vec<Option<path::Trigger>>; MAXL] = Default::default();
        let mut facts: [path::Facts; MAXL] = Default::default();
        let mut st = 0_u32;
        let pg = path::scan(&td, &entries, LevelMask::all(5), &to, true, &rules5, &[100, 300], &mut trig, &mut facts, &mut st);
        // Each level's exit: its earliest trigger's anchor, else the signal exit at a + 1,200 s; one exit walk per distinct anchor.
        let mut exits: Vec<(Us, LevelMask)> = Vec::new();
        for l in 0..5 {
            if f[l].units == 0 {
                continue;
            }
            let an = trig[l].iter().flatten().map(|x| x.anchor).min().unwrap_or(t(a + 1_200)).min(t(a + 1_200));
            match exits.iter_mut().find(|e| e.0 == an) {
                Some(e) => e.1 = LevelMask(e.1.0 | LevelMask::one(l).0),
                None => exits.push((an, LevelMask::one(l))),
            }
        }
        let mut sum = f.iter().map(|x| u64::from(x.steps.blocks + x.steps.jumps + x.steps.skips)).sum::<u64>();
        for &(an, m) in &exits {
            let xo = Order { side: Side::Sell, anchor: an, stop: t(DAY), unit: 1, limit: None, kind: Kind::SignalExit };
            let mut xw = [0_u64; MAXL];
            for l in 0..MAXL {
                if m.has(l) {
                    xw[l] = f[l].units;
                }
            }
            let mut xf = [LevelFill::default(); MAXL];
            walk(&td, &xo, &dial, m, &mut xw, None, None, &mut xf, None).unwrap();
            sum += xf.iter().map(|x| u64::from(x.steps.blocks + x.steps.jumps + x.steps.skips)).sum::<u64>();
        }
        groups = (groups.0.min(pg), groups.1.max(pg), groups.2.min(exits.len()), groups.3.max(exits.len()));
        sum + u64::from(st)
    });
    println!("\nB-TRADE groups of one trade: path groups {}..{}, exit groups {}..{} (5 levels)", groups.0, groups.1, groups.2, groups.3);
    // B-PAIR: one block of both legs at 5 levels (E9's pattern).
    let hedge: Vec<SecIn> = (0..DAY).map(|s| SecIn { ts: t(s), high: 2_000, low: 2_000, vol: 300 }).collect();
    let sold: Vec<SecIn> = (0..DAY).map(|s| SecIn { ts: t(s), high: 5_000, low: 5_000, vol: 1_200 }).collect();
    let (ih, is) = (SecIndex::new(&hedge, BASE, t(DAY)).unwrap(), SecIndex::new(&sold, BASE, t(DAY)).unwrap());
    let (th, ts) = (Tape::new(&hedge, &ih, None).unwrap(), Tape::new(&sold, &is, None).unwrap());
    class("B-PAIR (walk_pair: one block of both legs, 5 levels)", 1_000, false, (Some(300), 1), &mut checks, |k| {
        let a = at_b[k];
        let oh = Order { side: Side::Buy, anchor: t(a), stop: t(a + 2), unit: 75, limit: None, kind: Kind::Leg { leg: 0, open: true } };
        let os = Order { side: Side::Sell, anchor: t(a), stop: t(a + 2), unit: 75, limit: None, kind: Kind::Leg { leg: 1, open: true } };
        let mut w = [300, 300, 300, 300, 300, 0, 0, 0];
        let (mut fh, mut fs) = ([LevelFill::default(); MAXL], [LevelFill::default(); MAXL]);
        walk_pair((&th, &oh), (&ts, &os), &dial, LevelMask::all(5), &mut w, &mut fh, &mut fs, None).unwrap();
        fh.iter().chain(fs.iter()).map(|x| u64::from(x.steps.blocks)).sum()
    });
    // B-ROW: building one T2 and one T3 row (no I/O).
    let ctx = Ctx { run_id: "bench".into(), law: "req70-2s".into(), symbol: "SYN".into(), contract_token: "-".into(), expiry: "-".into(), strike: "-".into(), right: "-".into(), trade_id: "1".into(), order_id: "1".into(), order_kind: "entry".into() };
    let bo = Order { side: Side::Buy, anchor: BASE, stop: t(100), unit: 1, limit: None, kind: Kind::Entry };
    let bb = td.block(BASE, t(100)).unwrap();
    let bf = BlockFill { k: 0, block: bb, units: 10, px: bb.high, cum: 10, why: Why::Filled };
    let lf = LevelFill { units: 10, value: 10 * bb.high, blocks_filled: 1, first: Some(bf), last: Some(bf), e0: bb.at, e1: bb.known, known_at: bb.known, ..LevelFill::default() };
    class("B-ROW (rows::block_row + rows::order_row, the host's I/O excluded; per row)", 1_000, false, (Some(100), 2), &mut checks, |_| {
        black_box(rows::block_row(&ctx, 2_500, &bo, &bf, 25, 0, 0).unwrap().tsv().unwrap());
        let mut r = rows::order_row(&ctx, 2_500, &bo, None, 10, &lf).unwrap();
        for f in ["cancel_reason", "money_in_paisa", "slot_paisa", "reserve_paisa", "excess_paisa", "free_cash_at_known_paisa", "paid_paisa", "cut", "hole_span", "unclosable_units", "unclosable_px", "unclosable_from_to", "settlement"] {
            r.set(f, "-").unwrap();
        }
        black_box(r.tsv().unwrap());
        2
    });
    println!("\nload average at the end {}", load());
    print_checks(&checks);
    db_rows();
    println!("\nBENCH: every asserted class has identical steps per event at 1x, 10x and 100x; FillPath within 14 probes a call (one search a call), cut_at re-reads at most 1 a call.");
}

/// G0 evidence: the DB rows of design section 12 for E1 (KEI 2026-10-22, the REAL seconds of the vectors file, 25%): T1, T2, T3 (the
/// entry and the exit), T4, T5, T6, T7, every one in its frozen field order.
fn db_rows() {
    let text = include_str!("../vectors/COMMON-FILL-VECTORS-v2.tsv");
    let line = text.lines().find(|l| l.starts_with("E1-exit\t")).unwrap();
    let c: Vec<&str> = line.split('\t').collect();
    let rows_kei: Vec<SecIn> = c[3]
        .split(',')
        .map(|r| {
            let v: Vec<i64> = r.split(':').map(|x| x.parse().unwrap()).collect();
            SecIn { ts: v[0] * US_S, high: v[1], low: v[2], vol: u64::try_from(v[3]).unwrap() }
        })
        .collect();
    let ix = SecIndex::new(&rows_kei, rows_kei[0].ts, rows_kei[rows_kei.len() - 1].ts + US_S).unwrap();
    let tape = Tape::new(&rows_kei, &ix, None).unwrap();
    // 2026-10-22 09:15:05 IST (the bell), 09:17:40 (the exit anchor), 15:10:00 (the exit's stop), in UTC seconds.
    let bell = 1_792_640_705 * US_S;
    let xa = 1_792_640_860 * US_S;
    let entry = Order { side: Side::Sell, anchor: bell, stop: xa, unit: 1, limit: None, kind: Kind::Entry };
    let exit = Order { side: Side::Buy, anchor: xa, stop: 1_792_662_000 * US_S, unit: 1, limit: None, kind: Kind::SignalExit };
    let d = Dial::new(&[2_500]).unwrap();
    let go = |o: &Order| {
        let mut w = [124, 0, 0, 0, 0, 0, 0, 0];
        let mut f = [LevelFill::default(); MAXL];
        let mut fl = Vec::new();
        {
            let mut rec = |_l: usize, b: &BlockFill| fl.push(*b);
            walk(&tape, o, &d, LevelMask::all(1), &mut w, None, None, &mut f, Some(&mut rec)).unwrap();
        }
        (f[0], fl)
    };
    let (fe, ble) = go(&entry);
    let (fx, blx) = go(&exit);
    println!("\n# DB rows of design section 12 (schema {}), E1 KEI 2026-10-22 at 25% (real seconds): every table's frozen header and its rows", rows::SCHEMA);
    let mut t1 = Rec::new(T1);
    for (f, v) in [("run_id", "g0-bench-e1"), ("schema", rows::SCHEMA), ("tool", "common_fill cf_bench"), ("lane", "common-fill-20261112"), ("round", "G0"), ("law_headline", "req70-2s"), ("law_compiled", common_fill::law_compiled()), ("dial_bp", "2500"), ("timing_headline", "bell (REQ 70c)"), ("comparisons", "req70-2s-cap,req58b-core@cf2"), ("crate_version", common_fill::VERSION), ("common_fill_sha", "see logs/common-fill-v2.pin"), ("vectors_sha", "see SHA256SUMS"), ("matrix_sha", "see SHA256SUMS"), ("live_timing_sha", "-"), ("rules_sha", "-"), ("session_table_sha", "44158a9830dab1b2"), ("store_manifest_sha", "-"), ("binary_sha", "-"), ("started_ist", "-"), ("finished_ist", "-"), ("load_at_start", "-")] {
        t1.set(f, v).unwrap();
    }
    t1.set("filled_secs", fe.filled_secs + fx.filled_secs).unwrap().set("one_price_secs", fe.one_price_secs + fx.one_price_secs).unwrap().set("holes", 0).unwrap().set("hole_span_orders", 0).unwrap().set("inblock_stop_trades", 0).unwrap().set("supersedes_run_id", "-").unwrap();
    println!("{}\n{}", header(&T1), t1.tsv().unwrap());
    let ctx = |oid: &str, kind: &str| Ctx { run_id: "g0-bench-e1".into(), law: "req70-2s".into(), symbol: "KEI".into(), contract_token: "-".into(), expiry: "-".into(), strike: "-".into(), right: "-".into(), trade_id: "E1".into(), order_id: oid.into(), order_kind: kind.into() };
    println!("{}", header(&rows::T2));
    for (o, bl, ce) in [(&entry, &ble, ctx("E1-entry", "entry")), (&exit, &blx, ctx("E1-exit", "signal exit"))] {
        let mut rest = 124_u64;
        for b in bl {
            rest -= b.units;
            let capu = b.block.vol * 2_500 / 10_000;
            println!("{}", rows::block_row(&ce, 2_500, o, b, capu, 0, rest).unwrap().tsv().unwrap());
        }
    }
    println!("{}", header(&rows::T3));
    for (o, f, ce) in [(&entry, &fe, ctx("E1-entry", "entry")), (&exit, &fx, ctx("E1-exit", "signal exit"))] {
        let mut r = rows::order_row(&ce, 2_500, o, None, 124, f).unwrap();
        for (k, v) in [("cancel_reason", "-"), ("money_in_paisa", "-"), ("slot_paisa", "10000000"), ("reserve_paisa", "-"), ("excess_paisa", "0"), ("free_cash_at_known_paisa", "-"), ("paid_paisa", "0"), ("cut", "0"), ("hole_span", "0"), ("unclosable_units", "0"), ("unclosable_px", "-"), ("unclosable_from_to", "-"), ("settlement", "-")] {
            r.set(k, v).unwrap();
        }
        println!("{}", r.tsv().unwrap());
    }
    let gross = fe.value - fx.value;
    let mut t4 = Rec::new(T4);
    for (k, v) in [("run_id", "g0-bench-e1".to_owned()), ("level_bp", "2500".into()), ("trade_id", "E1".into()), ("symbol", "KEI".into()), ("instrument_kind", "F&O stock, cash".into()), ("underlying", "KEI".into()), ("signal_source", "opening 5-s candle 1 (design E1)".into()), ("entry_order_id", "E1-entry".into()), ("exit_order_ids", "E1-exit".into()), ("sq_order_id", "-".into()), ("e0_ist", rows::ist(fe.e0)), ("e1_ist", rows::ist(fe.e1)), ("x_end_ist", rows::ist(fx.known_at)), ("first_fill_px", fe.first.unwrap().px.to_string()), ("avg_entry_px", rows::avg_4dp(fe.value, fe.units)), ("avg_exit_px", rows::avg_4dp(fx.value, fx.units)), ("gross_paisa", gross.to_string()), ("charges_paisa", "22211".into()), ("net_paisa", (gross - 22_211).to_string())] {
        t4.set(k, v).unwrap();
    }
    for k in ["first_side", "up_at", "down_at", "both_secs", "best_px", "best_at_ist", "worst_px", "worst_at_ist", "run_up_paisa", "run_down_paisa", "first_side_from_e1", "inblock_stop", "winner_went_down_first", "loser_went_up_first", "incomplete_at_bell", "whatif_rule_ids", "whatif_trigger_ist", "whatif_net_paisa", "whatif_saved_paisa", "whatif_hindsight", "old_rule_net_paisa", "cap_net_paisa", "precise_net_paisa_or_na", "block_touch_net_paisa_or_na", "grid_cells_ref"] {
        t4.set(k, "-").unwrap();
    }
    t4.set("fills_before_anchor", 0).unwrap().set("fills_inside_signal_candle", 0).unwrap();
    println!("{}\n{}", header(&T4), t4.tsv().unwrap());
    let mut t5 = Rec::new(T5);
    for (k, v) in [("run_id", "g0-bench-e1".to_owned()), ("level_bp", "2500".into()), ("trade_id", "E1".into()), ("std70_net", (gross - 22_211).to_string()), ("old_rule_net", "480469".into()), ("diff", (gross - 22_211 - 480_469).to_string()), ("verdict", "kinder".into()), ("units_worse_than_old_second_price", "-".into()), ("cap_diff", "-".into()), ("precise_diff_or_na", "n/a".into()), ("block_touch_diff_or_na", "n/a".into())] {
        t5.set(k, v).unwrap();
    }
    println!("{}\n{}", header(&T5), t5.tsv().unwrap());
    let mut t6 = Rec::new(T6);
    for (k, v) in [("seq", "1"), ("ist", "-"), ("run_id", "g0-bench-e1"), ("action", rows::AUDIT_ACTIONS[0]), ("path", "-"), ("sha256", "-"), ("rows", "0"), ("bytes", "0"), ("writer", "cf_bench"), ("binary_sha", "-"), ("prev_row_sha256", "-"), ("note", "the G0 evidence rows")] {
        t6.set(k, v).unwrap();
    }
    println!("{}\n{}", header(&T6), t6.tsv().unwrap());
    let mut t7 = Rec::new(T7);
    for (k, v) in [("run_id", "g0-bench-e1".to_owned()), ("combo_id", "E1".into()), ("level_bp", "2500".into()), ("trades", "1".into()), ("units", fe.units.to_string()), ("value_paisa", fe.value.to_string()), ("gross_paisa", gross.to_string()), ("charges_paisa", "22211".into()), ("net_paisa", (gross - 22_211).to_string()), ("unpaid_units", fe.unpaid.to_string()), ("excess_n", "0".into()), ("excess_max_paisa", "0".into()), ("clipped_blocks", (fe.clipped + fx.clipped).to_string()), ("zero_take_blocks", (fe.zero_take_blocks + fx.zero_take_blocks).to_string()), ("holes_spanned", "0".into())] {
        t7.set(k, v).unwrap();
    }
    println!("{}\n{}", header(&T7), t7.tsv().unwrap());
    assert_eq!((fe.units, fe.value, fx.value, gross), (124, 49_443_760, 48_940_320, 503_440), "E1 at 25%: 124 @ 3987.40, 124 @ 3946.80, gross Rs 5,034.40");
}

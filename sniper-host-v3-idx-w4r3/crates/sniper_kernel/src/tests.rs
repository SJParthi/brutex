//! Red-first tests of the kernel (written before the engine's code ran): each named check fails under `red` or its mutant.

use std::collections::{BTreeMap, BTreeSet};

use crate::bits::Bits;
use crate::code::Code;
use crate::ladder::{Cmp, Ladder};
use crate::line::{Cell, Core, LineEngine, Row, Sink, WalkStats};
use crate::scale::{replicate, splitmix};

/// A test sink that keeps the current trade set and records every offered run with that set.
#[derive(Default)]
struct RecSink {
    cur: BTreeSet<u32>,
    offers: Vec<(Cell, u32, Vec<u32>)>,
}

impl Sink for RecSink {
    type Err = ();
    fn reset(&mut self) {
        self.cur.clear();
    }
    fn insert(&mut self, row: u32) -> Result<(), ()> {
        if self.cur.insert(row) { Ok(()) } else { Err(()) }
    }
    fn remove(&mut self, row: u32) -> Result<(), ()> {
        if self.cur.remove(&row) { Ok(()) } else { Err(()) }
    }
    fn offer(&mut self, cell: Cell, run: u32) -> Result<(), ()> {
        self.offers.push((cell, run, self.cur.iter().copied().collect()));
        Ok(())
    }
}

/// The plain FIRST-mode answer of one cell, by brute force: per session the qualifying row with the smallest candle index.
fn brute(rows: &[Row], kept: &[usize], cell: Cell) -> Vec<u32> {
    let mut best: BTreeMap<u32, (u32, u32)> = BTreeMap::new();
    for &i in kept {
        let r = rows[i];
        if r.rb >= cell.r && r.cb >= cell.c && r.wb >= cell.w {
            let e = best.entry(r.sess).or_insert((u32::MAX, u32::MAX));
            if r.k < e.0 {
                *e = (r.k, u32::try_from(i).unwrap());
            }
        }
    }
    let mut v: Vec<u32> = best.values().map(|x| x.1).collect();
    v.sort_unstable();
    v
}

fn random_rows(seed: u64, sessions: u32, per: u16, core: Core) -> Vec<Row> {
    let mut st = seed;
    let mut rows = Vec::new();
    for s in 0..sessions {
        for k in 1..=per {
            st = splitmix(st);
            // Skewed buckets: most rows fail every rung, a few reach the extremes (the sniper's shape).
            let pick = |x: u64, n: u16| -> u16 {
                let n = u64::from(n);
                let v = if x.is_multiple_of(3) { x % n } else { (x % 4).min(n - 1) };
                u16::try_from(v).unwrap()
            };
            rows.push(Row { sess: s, k: u32::from(k), rb: pick(st, core.nr), cb: pick(st >> 7, core.nc), wb: pick(st >> 17, core.nw) });
        }
    }
    rows
}

#[test]
fn line_engine_equals_brute_force_first_mode() {
    // SN-X23 / m-S5 / B1 (red): every offered run's set equals the brute-force FIRST-mode set of every cell in the run, and every
    // cell outside the offered runs is empty. m-S2 (a run counted n - 1) breaks the coverage.
    let core = Core { nr: 5, nc: 9, nw: 4 };
    for seed in 1..=12_u64 {
        let rows = random_rows(seed, 7, 6, core);
        let kept: Vec<usize> = (0..rows.len()).filter(|i| !splitmix(seed ^ (*i as u64)).is_multiple_of(5)).collect();
        let mut eng = LineEngine::new(7, core);
        let mut sink = RecSink::default();
        let mut st = WalkStats::default();
        eng.walk(&rows, kept.iter().copied(), &mut sink, &mut st).unwrap();
        let mut covered: BTreeSet<(u16, u16, u16)> = BTreeSet::new();
        for (cell, run, set) in &sink.offers {
            for dc in 0..*run {
                let c = Cell { r: cell.r, c: cell.c + u16::try_from(dc).unwrap(), w: cell.w };
                assert_eq!(&brute(&rows, &kept, c), set, "seed {seed} cell {c:?}");
                assert!(covered.insert((c.r, c.c, c.w)), "cell offered twice {c:?}");
            }
        }
        for r in 0..core.nr {
            for c in 0..core.nc {
                for w in 0..core.nw {
                    if !covered.contains(&(r, c, w)) {
                        assert!(brute(&rows, &kept, Cell { r, c, w }).is_empty(), "seed {seed}: non-empty cell ({r},{c},{w}) not offered");
                    }
                }
            }
        }
        assert_eq!(st.cells, core.cells(), "every core cell covered once");
        assert_eq!(st.cells_offered + st.empty_cells, st.cells);
    }
}

#[test]
fn tie_runs_are_ranges_of_consecutive_cci_levels() {
    // SN-X04: identical kept sets across consecutive 1-point levels give ONE run; the run's cell is its LOWEST level (the
    // smallest code). Two rows of one session: CCI buckets 7 and 3; the set changes only at 7 and at 3.
    let core = Core { nr: 1, nc: 9, nw: 1 };
    let rows = vec![Row { sess: 0, k: 2, rb: 0, cb: 7, wb: 0 }, Row { sess: 0, k: 1, rb: 0, cb: 3, wb: 0 }];
    let mut eng = LineEngine::new(1, core);
    let mut sink = RecSink::default();
    let mut st = WalkStats::default();
    eng.walk(&rows, 0..2, &mut sink, &mut st).unwrap();
    let got: Vec<(u16, u32, Vec<u32>)> = sink.offers.iter().map(|(c, n, s)| (c.c, *n, s.clone())).collect();
    // c = 8: empty (1 cell); c = 7..4: row 0 (run of 4 from 4); c = 3..0: row 1 replaced row 0 (FIRST mode; run of 4 from 0).
    assert_eq!(got, vec![(4, 4, vec![0]), (0, 4, vec![1])]);
    assert_eq!(st.empty_cells, 1);
    assert_eq!(st.removes, 1, "the session's first qualifying candle moved earlier: one removal");
}

#[test]
fn ladder_buckets_are_exact_and_the_last_rung_is_open() {
    // F24 / CAL-11: exact on the rational state; G-OPEN (S8): 10x the last level passes the last rung (m-S19 fails it).
    let rsi = Ladder::long_of("M-RSI", "GE=range:70:100:1").unwrap();
    assert_eq!(rsi.values(), 32);
    assert_eq!(rsi.bucket(Some((70, 1))), 1, "exactly 70 passes 70");
    assert_eq!(rsi.bucket(Some((69_999_999, 1_000_000))), 0, "69.999999 fails 70");
    assert_eq!(rsi.bucket(Some((100, 1))), 31, "RSI exactly 100 passes every rung");
    assert_eq!(rsi.bucket(None), 0, "an unready reading passes only OFF");
    let cci = Ladder::long_of("M-CCI", "GE=range:100:666:1").unwrap();
    assert_eq!(cci.values(), 568);
    // CCI 666.67 (one deviating typical price among 20): 666 passes, there is no 667 rung.
    assert_eq!(cci.bucket(Some((200_000, 300))), 567);
    let wr = Ladder::long_of("M-WR", "GE=range:-20:0:1").unwrap();
    assert_eq!(wr.values(), 22);
    assert_eq!(wr.bucket(Some((0, 1))), 21, "%R exactly 0 passes every rung");
    let short = Ladder::short_of(&rsi, "LE 100-x (30..0)").unwrap();
    assert_eq!(short.cmp, Cmp::Le);
    assert_eq!(short.levels.first(), Some(&30));
    assert_eq!(short.levels.last(), Some(&0));
    assert_eq!(short.bucket(Some((0, 1))), 31, "RSI exactly 0 passes every short rung");
    let wr_s = Ladder::short_of(&wr, "LE -100-x (-80..-100)").unwrap();
    assert_eq!(wr_s.bucket(Some((-100, 1))), 21, "%R exactly -100 passes every short rung");
    let cci_s = Ladder::short_of(&cci, "LE -x (-100..-666)").unwrap();
    assert_eq!(cci_s.bucket(Some((-200_000, 300))), 567);
    // G-OPEN on an open distance ladder: 10x its last written level passes the last rung.
    let dist = Ladder::long_of("T-ST", "GE=range:0:300:10").unwrap();
    assert_eq!(dist.bucket(Some((3000, 1))), u16::try_from(dist.levels.len()).unwrap(), "G-OPEN: 10x the last level passes the last rung");
    assert!(Ladder::long_of("X", "GE=range:5:1:1").is_err());
    assert!(Ladder::long_of("X", "LE=range:1:5:1").is_err());
}

#[test]
fn step_counts_identical_at_1x_10x_100x() {
    // The HARD gate (8.4): the per-event kernel step counts (enter, line) are identical at 1x, 10x and 100x.
    let core = Core { nr: 4, nc: 12, nw: 3 };
    let base = random_rows(77, 20, 5, core);
    let mut maxima = Vec::new();
    for t in [1_u16, 10, 100] {
        let (rows, src) = replicate(&base, t, 20_261_113);
        assert_eq!(rows.len(), base.len() * usize::from(t));
        assert_eq!(src.len(), rows.len());
        let mut eng = LineEngine::new(20, core);
        let mut sink = RecSink::default();
        let mut st = WalkStats::default();
        eng.walk(&rows, 0..rows.len(), &mut sink, &mut st).unwrap();
        maxima.push((st.hist_enter.max(), st.hist_line.max(), st.hist_line.permille(500), st.hist_line.permille(990)));
    }
    assert_eq!(maxima[0], maxima[1], "1x vs 10x");
    assert_eq!(maxima[0], maxima[2], "1x vs 100x");
}

#[test]
fn codes_are_mixed_radix_with_cci_least_significant() {
    let a = Code::mixed(&[(412, 568), (3, 32), (5, 22)]).unwrap();
    let b = Code::mixed(&[(413, 568), (3, 32), (5, 22)]).unwrap();
    assert_eq!(a.plus(1), Some(b), "consecutive CCI levels have consecutive codes");
    assert!(a < b);
    assert!(Code::mixed(&[(568, 568)]).is_none(), "a digit at its radix is refused");
    // 3.99 x 10^75 < 2^256: 64 digits of radix 15 fit; 2^256 itself does not.
    let mut big = vec![(14_u64, 15_u64); 64];
    assert!(Code::mixed(&big).is_some());
    big.push((1, u64::MAX));
    big.push((1, u64::MAX));
    assert!(Code::mixed(&big).is_none());
    assert_eq!(Code::default().hex().len(), 64);
}

#[test]
fn bitsets_and_and_iterate() {
    let mut a = Bits::new(130);
    let mut b = Bits::new(130);
    for i in [0, 3, 64, 129] {
        a.set(i);
    }
    for i in [3, 64, 100] {
        b.set(i);
    }
    a.and_with(&b);
    assert_eq!(a.ones().collect::<Vec<_>>(), vec![3, 64]);
    assert_eq!(a.count(), 2);
    assert_eq!(Bits::full(70).count(), 70);
}

#[test]
fn law_tag_and_refusal() {
    if crate::RED {
        assert!(crate::refuse_if_red().is_err());
    } else {
        assert_eq!(crate::law(), "sniper-kernel-v1");
        assert!(crate::refuse_if_red().is_ok());
    }
}

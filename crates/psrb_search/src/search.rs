//! Phase B (SEARCH-SPEC 6): per timeframe, every unit (core, exit lane) and every
//! combination of the library over the unit's records, six accounts each; the streaming
//! top-20 heaps per (window, scope); the luck stream; the bare cores and singles; the
//! build-up of 4-filter stacks (ADDENDUM 3 (2)); the random-entry twin (8.3).

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use rayon::prelude::*;

use crate::acct::{C0, Fast, Heaps, Luck, N_SCOPE_TF, N_WIN, Row, flips, key_range};
use crate::common::{N_CORE, N_MONTHS, SIDE6, TFS, draw, lanes_of};
use crate::filters::{DEFS, FW, N_BASE, N_FILT, groups};
use crate::phase_a::Sym;
use crate::spool::{Hot, load_hot};

/// Rows tried so far (for the progress line).
pub(crate) static ROWS: AtomicU64 = AtomicU64::new(0);
/// Records whose gross or charges did not fit the compact view (printed by the run; never seen).
pub(crate) static MINI_REFUSED: AtomicU64 = AtomicU64::new(0);
/// Tasks finished so far.
pub(crate) static TASKS: AtomicU64 = AtomicU64::new(0);
/// What the run is doing now.
pub(crate) static STAGE: Mutex<String> = Mutex::new(String::new());

/// Set the progress stage.
pub(crate) fn stage(s: String) {
    if let Ok(mut g) = STAGE.lock() {
        *g = s;
    }
}

/// A combination: `n` 0..=4 fixed filters (ascending in `f`), or a candle-code sequence
/// of length `n - 4` (5, 6, 7 = lengths 1, 2, 3; codes in `f`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub(crate) struct Combo {
    pub(crate) n: u8,
    pub(crate) f: [u8; 4],
}

impl Combo {
    pub(crate) fn code(self) -> u64 {
        let [a, b, c, d] = self.f;
        (u64::from(self.n) << 32) | (u64::from(a) << 24) | (u64::from(b) << 16) | (u64::from(c) << 8) | u64::from(d)
    }

    pub(crate) fn of_code(x: u64) -> Self {
        let n = u8::try_from((x >> 32) & 0x7).unwrap_or(0);
        let f = [(x >> 24) & 0xFF, (x >> 16) & 0xFF, (x >> 8) & 0xFF, x & 0xFF].map(|v| u8::try_from(v).unwrap_or(0));
        Self { n, f }
    }

    pub(crate) fn filters(self) -> Vec<usize> {
        if self.n > 4 {
            return Vec::new();
        }
        self.f.iter().take(usize::from(self.n)).map(|x| usize::from(*x)).collect()
    }

    pub(crate) fn seq_len(self) -> usize {
        if self.n > 4 { usize::from(self.n - 4) } else { 0 }
    }

    pub(crate) fn with(self, extra: usize) -> Option<Self> {
        if self.n >= 4 {
            return None;
        }
        let mut v = self.filters();
        v.push(extra);
        v.sort_unstable();
        let mut f = [0_u8; 4];
        for (slot, x) in f.iter_mut().zip(v.iter()) {
            *slot = u8::try_from(*x).ok()?;
        }
        Some(Self { n: self.n + 1, f })
    }

    pub(crate) fn words(self) -> String {
        if self.n == 0 {
            return "the bare core (no filter)".to_owned();
        }
        if self.n > 4 {
            let codes: Vec<String> = self.f.iter().take(self.seq_len()).map(|c| crate::filters::code_words(*c)).collect();
            return format!("candle-code sequence ending at the signal candle: {}", codes.join(" then "));
        }
        self.filters()
            .iter()
            .map(|&i| DEFS.get(i).map_or_else(|| "?".to_owned(), |d| format!("{} {}", d.id, d.words)))
            .collect::<Vec<_>>()
            .join(" + ")
    }

    pub(crate) fn ids(self) -> String {
        if self.n == 0 {
            return "bare".to_owned();
        }
        if self.n > 4 {
            return format!("SEQ{}:{}", self.seq_len(), self.f.iter().take(self.seq_len()).map(u8::to_string).collect::<Vec<_>>().join("-"));
        }
        self.filters().iter().map(|&i| DEFS.get(i).map_or("?", |d| d.id)).collect::<Vec<_>>().join("+")
    }
}

/// The stable recipe id (SEARCH-SPEC 9.1 key 8): timeframe, core, lane, side, combination, option set.
pub(crate) fn pack(ti: usize, core: usize, lane: usize, side: usize, combo: Combo, opt: usize) -> u64 {
    let u = |x: usize| u64::try_from(x).unwrap_or(0);
    ((((((u(ti) * 8 + u(core)) * 8 + u(lane)) * 8 + u(side)) << 35) | combo.code()) << 11) | u(opt)
}

/// The parts of a recipe id.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Recipe {
    pub(crate) ti: usize,
    pub(crate) core: usize,
    pub(crate) lane: usize,
    pub(crate) side: usize,
    pub(crate) combo: Combo,
    pub(crate) opt: usize,
}

pub(crate) fn unpack(id: u64) -> Recipe {
    let opt = usize::try_from(id & 0x7FF).unwrap_or(0);
    let rest = id >> 11;
    let combo = Combo::of_code(rest & ((1_u64 << 35) - 1));
    let top = rest >> 35;
    let side = usize::try_from(top % 8).unwrap_or(0);
    let lane = usize::try_from((top / 8) % 8).unwrap_or(0);
    let core = usize::try_from((top / 64) % 8).unwrap_or(0);
    let ti = usize::try_from(top / 512).unwrap_or(0);
    Recipe { ti, core, lane, side, combo, opt }
}

/// The exact counts per cell (SEARCH-SPEC 6.3 and the ADDENDUM 3/4 extension).
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Counts63 {
    pub(crate) singles: u64,
    pub(crate) pairs: u64,
    pub(crate) triples: u64,
    pub(crate) l2_singles: u64,
    pub(crate) l2_base_pairs: u64,
    pub(crate) l2_pairs: u64,
    pub(crate) seqs: u64,
}

impl Counts63 {
    pub(crate) fn per_cell(&self) -> u64 {
        1 + self.singles + self.pairs + self.triples + self.l2_singles + self.l2_base_pairs + self.l2_pairs + self.seqs
    }
    pub(crate) fn spec_cell(&self) -> u64 {
        1 + self.singles + self.pairs + self.triples
    }
}

/// The library's enumeration (the same for every unit).
pub(crate) struct Lib {
    pub(crate) g: [u8; N_FILT],
    pub(crate) n_groups: usize,
    pub(crate) counts: Counts63,
}

impl Lib {
    pub(crate) fn new() -> Self {
        let (g, n_groups) = groups();
        let mut c = Counts63 { singles: 74, l2_singles: 97, seqs: 24 + 576 + 13_824, ..Counts63::default() };
        for i in 0..N_BASE {
            for j in i + 1..N_BASE {
                if g.get(i) == g.get(j) {
                    continue;
                }
                c.pairs += 1;
                for k in j + 1..N_BASE {
                    if g.get(k) != g.get(i) && g.get(k) != g.get(j) {
                        c.triples += 1;
                    }
                }
            }
        }
        for i in N_BASE..N_FILT {
            c.l2_base_pairs += 74;
            for j in i + 1..N_FILT {
                if g.get(i) != g.get(j) {
                    c.l2_pairs += 1;
                }
            }
        }
        Self { g, n_groups, counts: c }
    }

    fn grp(&self, i: usize) -> u8 {
        self.g.get(i).copied().unwrap_or(u8::MAX)
    }
}

/// One unit's data: the core's records of one lane and their filter bitsets.
pub(crate) struct Unit<'a> {
    pub(crate) ti: usize,
    pub(crate) core: usize,
    pub(crate) lane: usize,
    pub(crate) recs: &'a [Hot],
    /// Positions (into `recs`) of the core's records, in admission order.
    pub(crate) idx: Vec<u32>,
    pub(crate) words: usize,
    /// `N_FILT` bitsets of `words` words over `idx`.
    pub(crate) bits: Vec<u64>,
    /// The candle codes of each kept position (twin: shuffled across positions).
    pub(crate) codes: Vec<[u8; 3]>,
    /// The compact records of the hot loop, by position.
    pub(crate) mini: Vec<crate::acct::Mini>,
}

fn unit_seed(ti: usize, core: usize, lane: usize) -> u64 {
    u64::try_from((ti * 8 + core) * 8 + lane).unwrap_or(0)
}

/// Build a unit; `twin` replaces every filter bitset by a random one of the same size
/// (Knuth's selection sampling) and shuffles the candle codes (SEARCH-SPEC 8.3).
pub(crate) fn build_unit<'a>(recs: &'a [Hot], ti: usize, core: usize, lane: usize, twin: Option<u64>) -> Unit<'a> {
    let idx: Vec<u32> = recs
        .iter()
        .enumerate()
        .filter(|(_, r)| r.cores & (1 << core) != 0)
        .filter_map(|(i, _)| u32::try_from(i).ok())
        .collect();
    let n = idx.len();
    let words = n.div_ceil(64);
    let mut bits = vec![0_u64; N_FILT * words];
    let mut codes = Vec::with_capacity(n);
    for (p, &ri) in idx.iter().enumerate() {
        let Some(r) = recs.get(usize::try_from(ri).unwrap_or(usize::MAX)) else { continue };
        codes.push(r.codes);
        for (wi, w) in r.fbits.iter().enumerate() {
            let mut x = *w;
            while x != 0 {
                let b = usize::try_from(x.trailing_zeros()).unwrap_or(0);
                x &= x - 1;
                let f = wi * 64 + b;
                if f < N_FILT
                    && let Some(slot) = bits.get_mut(f * words + p / 64)
                {
                    *slot |= 1_u64 << (p % 64);
                }
            }
        }
    }
    if let Some(seed) = twin {
        let us = unit_seed(ti, core, lane);
        let nn = u64::try_from(n).unwrap_or(0);
        for f in 0..N_FILT {
            let Some(bs) = bits.get_mut(f * words..(f + 1) * words) else { continue };
            let k: u64 = bs.iter().map(|w| u64::from(w.count_ones())).sum();
            bs.iter_mut().for_each(|w| *w = 0);
            let mut kept = 0_u64;
            for i in 0..nn {
                if kept >= k {
                    break;
                }
                let x = draw(seed, us, u64::try_from(f).unwrap_or(0), i);
                if x % (nn - i) < k - kept {
                    let iu = usize::try_from(i).unwrap_or(0);
                    if let Some(slot) = bs.get_mut(iu / 64) {
                        *slot |= 1_u64 << (iu % 64);
                    }
                    kept += 1;
                }
            }
        }
        // Fisher-Yates over the code triples.
        for i in (1..n).rev() {
            let iu = u64::try_from(i).unwrap_or(0);
            let j = usize::try_from(draw(seed, us, 999, iu) % (iu + 1)).unwrap_or(0);
            codes.swap(i, j);
        }
    }
    let mini: Vec<crate::acct::Mini> = idx
        .iter()
        .filter_map(|&ri| recs.get(usize::try_from(ri).ok()?))
        .map(|h| {
            crate::acct::Mini::of(h).unwrap_or_else(|_| {
                MINI_REFUSED.fetch_add(1, Ordering::Relaxed);
                crate::acct::Mini::default()
            })
        })
        .collect();
    Unit { ti, core, lane, recs, idx, words, bits, codes, mini }
}

impl Unit<'_> {
    fn bitset(&self, f: usize) -> &[u64] {
        self.bits.get(f * self.words..(f + 1) * self.words).unwrap_or(&[])
    }

    #[inline]
    fn rec(&self, p: usize) -> Option<&Hot> {
        let ri = usize::try_from(*self.idx.get(p)?).ok()?;
        self.recs.get(ri)
    }

    /// The kept positions of a combination of fixed filters (sorted).
    pub(crate) fn kept(&self, combo: Combo) -> Vec<u32> {
        let n = self.idx.len();
        if combo.n > 4 {
            let l = combo.seq_len();
            return self
                .codes
                .iter()
                .enumerate()
                .filter(|(_, c)| {
                    let tail = c.get(3 - l..).unwrap_or(&[]);
                    let want = combo.f.get(..l).unwrap_or(&[]);
                    tail == want && !tail.contains(&u8::MAX)
                })
                .filter_map(|(p, _)| u32::try_from(p).ok())
                .collect();
        }
        let fs = combo.filters();
        let mut out = Vec::new();
        for w in 0..self.words {
            let mut x = u64::MAX;
            for &f in &fs {
                x &= self.bitset(f).get(w).copied().unwrap_or(0);
            }
            if w + 1 == self.words && n % 64 != 0 {
                x &= (1_u64 << (n % 64)) - 1;
            }
            while x != 0 {
                let b = usize::try_from(x.trailing_zeros()).unwrap_or(0);
                x &= x - 1;
                if let Ok(p) = u32::try_from(w * 64 + b) {
                    out.push(p);
                }
            }
        }
        out
    }

    /// The hot records of a kept list.
    pub(crate) fn records(&self, kept: &[u32]) -> Vec<Hot> {
        kept.iter().filter_map(|&p| self.rec(usize::try_from(p).unwrap_or(usize::MAX)).copied()).collect()
    }
}

/// A worker's state.
struct St {
    fast: Fast,
    one: crate::acct::FastAcc,
    heaps: Heaps,
    luck: Luck,
    tried: u64,
    lines_bare: Vec<String>,
    lines_single: Vec<String>,
    buf: Vec<u64>,
}

impl St {
    fn new() -> Self {
        Self { fast: Fast::new(), one: crate::acct::FastAcc::new(), heaps: Heaps::new(20), luck: Luck::new(), tried: 0, lines_bare: Vec::new(), lines_single: Vec::new(), buf: Vec::new() }
    }

    fn merge(mut self, o: Self) -> Self {
        self.heaps.merge(&o.heaps);
        self.luck.merge(&o.luck);
        self.tried += o.tried;
        self.lines_bare.extend(o.lines_bare);
        self.lines_single.extend(o.lines_single);
        self
    }
}

/// One summary line of a row (bare-cores.tsv, singles.tsv).
pub(crate) fn summary_line(ti: usize, core: usize, lane: usize, side: usize, combo: Combo, row: &Row) -> String {
    let k = key_range(row, 0, N_MONTHS, 0);
    let worst = if k.worst_d > 0 { (10_000 * k.worst_n).div_euclid(k.worst_d).to_string() } else { "NA".to_owned() };
    let hold_min = if k.trades > 0 { (k.hold / 60).div_euclid(i64::from(k.trades)) } else { 0 };
    let win_pm = if k.trades > 0 { (1_000 * i64::from(k.wins)).div_euclid(i64::from(k.trades)) } else { 0 };
    format!(
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        TFS.get(ti).copied().unwrap_or(0),
        crate::common::CORE_NAMES.get(core).copied().unwrap_or("?"),
        crate::common::LANE_NAMES.get(lane).copied().unwrap_or("?"),
        SIDE6.get(side).copied().unwrap_or("?"),
        combo.ids(),
        N_MONTHS - usize::from(k.below),
        worst,
        k.dd,
        k.fall,
        k.net,
        k.trades,
        win_pm,
        hold_min,
        k.gross,
        k.charges,
        (10_000 * i128::from(k.net)).div_euclid(C0)
    )
}

pub(crate) const SUMMARY_HEAD: &str = "timeframe_min\tcore\texit\tside\tfilters\tmonths_pass_of_98\tworst_month_bp\tmax_dd_tenths_pct_of_20L\tmax_fall_paisa\tnet_paisa\ttrades\twin_permille\tavg_hold_min\tgross_paisa\tcharges_paisa\tnet_bp_of_20L";

/// Offer the six accounts' rows of the combination just stepped.
fn offer_rows(st: &mut St, unit: &Unit<'_>, combo: Combo, flips: &[u128], only: Option<usize>, summary: u8) {
    st.fast.finish();
    for side in 0..6 {
        let Some(a) = st.fast.acc.get(side) else { continue };
        let id = pack(unit.ti, unit.core, unit.lane, side, combo, 0);
        st.heaps.offer_row(&a.row, id, side, only);
        st.luck.offer_row(flips, &a.row);
        match summary {
            1 => st.lines_bare.push(summary_line(unit.ti, unit.core, unit.lane, side, combo, &a.row)),
            2 => st.lines_single.push(summary_line(unit.ti, unit.core, unit.lane, side, combo, &a.row)),
            _ => {}
        }
    }
    st.tried += 6;
    ROWS.fetch_add(6, Ordering::Relaxed);
}

/// Step the six accounts over the set bits of `a AND b AND c` (dense words, in order).
#[allow(clippy::too_many_arguments, reason = "one evaluation, every input named")]
fn eval_bits(st: &mut St, unit: &Unit<'_>, combo: Combo, a: &[u64], b: Option<&[u64]>, c: Option<&[u64]>, flips: &[u128], only: Option<usize>, summary: u8) {
    st.fast.reset();
    let n = unit.idx.len();
    let words = a.len();
    for w in 0..words {
        let mut x = a[w];
        if let Some(b) = b {
            x &= b[w];
        }
        if let Some(c) = c {
            x &= c[w];
        }
        if w + 1 == words && n % 64 != 0 {
            x &= (1_u64 << (n % 64)) - 1;
        }
        while x != 0 {
            let bt = usize::try_from(x.trailing_zeros()).unwrap_or(0);
            x &= x - 1;
            if let Some(r) = unit.mini.get(w * 64 + bt) {
                st.fast.step(r);
            }
        }
    }
    offer_rows(st, unit, combo, flips, only, summary);
}

/// Step the six accounts over a sparse list of positions.
fn eval_list(st: &mut St, unit: &Unit<'_>, combo: Combo, list: &[u32], flips: &[u128], only: Option<usize>) {
    st.fast.reset();
    for &p in list {
        if let Some(r) = unit.mini.get(usize::try_from(p).unwrap_or(usize::MAX)) {
            st.fast.step(r);
        }
    }
    offer_rows(st, unit, combo, flips, only, 0);
}

/// One account (side) of one combination over the set bits of `acc`, offered to window `w` only
/// (the light timeframes' build-up).
fn eval_side(st: &mut St, unit: &Unit<'_>, combo: Combo, side: usize, acc: &[u64], flips: &[u128], w: usize) {
    st.one.reset();
    let n = unit.idx.len();
    let words = acc.len();
    let which = usize::from(side >= 3);
    let want: Option<u8> = match side {
        0 | 3 => Some(0),
        1 | 4 => Some(1),
        _ => None,
    };
    for wi in 0..words {
        let mut x = acc[wi];
        if wi + 1 == words && n % 64 != 0 {
            x &= (1_u64 << (n % 64)) - 1;
        }
        while x != 0 {
            let bt = usize::try_from(x.trailing_zeros()).unwrap_or(0);
            x &= x - 1;
            if let Some(r) = unit.mini.get(wi * 64 + bt)
                && want.is_none_or(|s| r.side == s)
            {
                st.one.step(r.t, r.stock, &r.tr[which]);
            }
        }
    }
    st.one.finish();
    let id = pack(unit.ti, unit.core, unit.lane, side, combo, 0);
    st.heaps.offer_row(&st.one.row, id, side, Some(w));
    st.luck.offer_row(flips, &st.one.row);
    st.tried += 1;
    ROWS.fetch_add(1, Ordering::Relaxed);
}

/// The six rows of one combination (materializing heap entries).
pub(crate) fn rows_of(unit: &Unit<'_>, combo: Combo) -> Vec<Row> {
    let kept = unit.kept(combo);
    let mut f = Fast::new();
    for &p in &kept {
        if let Some(r) = unit.mini.get(usize::try_from(p).unwrap_or(usize::MAX)) {
            f.step(r);
        }
    }
    f.finish();
    f.acc.iter().map(|a| a.row).collect()
}

#[derive(Clone, Copy, Debug)]
enum Task {
    Bare,
    Singles(usize, usize),
    Pair(usize, usize),
    L2(usize),
    Seq(usize, usize),
}

/// Every combination of one unit, in fixed interleaved chunks (one state per chunk).
fn run_unit(unit: &Unit<'_>, lib: &Lib, flips: &[u128], threads_done: &AtomicU64, light: bool) -> St {
    let n = unit.idx.len();
    let mut tasks: Vec<Task> = vec![Task::Bare];
    for s in (0..N_FILT).step_by(16) {
        tasks.push(Task::Singles(s, (s + 16).min(N_FILT)));
    }
    for i in 0..N_BASE {
        for j in i + 1..N_BASE {
            if lib.grp(i) != lib.grp(j) {
                tasks.push(Task::Pair(i, j));
            }
        }
    }
    if !light {
        for i in N_BASE..N_FILT {
            tasks.push(Task::L2(i));
        }
    }
    // candle-code sequences present in this unit, grouped
    let mut seq_groups: BTreeMap<(usize, [u8; 3]), Vec<u32>> = BTreeMap::new();
    for (p, c) in unit.codes.iter().enumerate() {
        for l in 1..=3_usize {
            let tail = c.get(3 - l..).unwrap_or(&[]);
            if tail.contains(&u8::MAX) {
                continue;
            }
            let mut key = [0_u8; 3];
            for (k, x) in key.iter_mut().zip(tail.iter()) {
                *k = *x;
            }
            if let Ok(pp) = u32::try_from(p) {
                seq_groups.entry((l, key)).or_default().push(pp);
            }
        }
    }
    let seq_list: Vec<((usize, [u8; 3]), Vec<u32>)> = seq_groups.into_iter().collect();
    let chunk = 64;
    for c in 0..seq_list.len().div_ceil(chunk) {
        tasks.push(Task::Seq(c * chunk, ((c + 1) * chunk).min(seq_list.len())));
    }
    let n_chunks = (rayon::current_num_threads() * 4).max(1);
    let all_ones: Vec<u64> = vec![u64::MAX; unit.words];
    let st = (0..n_chunks)
        .into_par_iter()
        .map(|ci| {
            let mut st = St::new();
            for task in tasks.iter().skip(ci).step_by(n_chunks) {
                match *task {
                    Task::Bare => eval_bits(&mut st, unit, Combo::default(), &all_ones, None, None, flips, None, 1),
                    Task::Singles(a, b) => {
                        for f in a..b {
                            let combo = Combo { n: 1, f: [u8::try_from(f).unwrap_or(0), 0, 0, 0] };
                            eval_bits(&mut st, unit, combo, unit.bitset(f), None, None, flips, None, 2);
                        }
                    }
                    Task::Pair(i, j) => {
                        let (a, b) = (unit.bitset(i), unit.bitset(j));
                        let mut pair = std::mem::take(&mut st.buf);
                        pair.clear();
                        pair.extend(a.iter().zip(b.iter()).map(|(x, y)| x & y));
                        let fi = u8::try_from(i).unwrap_or(0);
                        let fj = u8::try_from(j).unwrap_or(0);
                        eval_bits(&mut st, unit, Combo { n: 2, f: [fi, fj, 0, 0] }, &pair, None, None, flips, None, 0);
                        for k in (j + 1..N_BASE).filter(|_| !light) {
                            if lib.grp(k) == lib.grp(i) || lib.grp(k) == lib.grp(j) {
                                continue;
                            }
                            let fk = u8::try_from(k).unwrap_or(0);
                            eval_bits(&mut st, unit, Combo { n: 3, f: [fi, fj, fk, 0] }, &pair, Some(unit.bitset(k)), None, flips, None, 0);
                        }
                        st.buf = pair;
                    }
                    Task::L2(i) => {
                        let fi = u8::try_from(i).unwrap_or(0);
                        for b in 0..N_BASE {
                            let fb = u8::try_from(b).unwrap_or(0);
                            eval_bits(&mut st, unit, Combo { n: 2, f: [fb, fi, 0, 0] }, unit.bitset(b), Some(unit.bitset(i)), None, flips, None, 0);
                        }
                        for j in i + 1..N_FILT {
                            if lib.grp(j) == lib.grp(i) {
                                continue;
                            }
                            let fj = u8::try_from(j).unwrap_or(0);
                            eval_bits(&mut st, unit, Combo { n: 2, f: [fi, fj, 0, 0] }, unit.bitset(i), Some(unit.bitset(j)), None, flips, None, 0);
                        }
                    }
                    Task::Seq(a, b) => {
                        for ((l, key), ps) in seq_list.get(a..b).unwrap_or(&[]) {
                            let mut f = [0_u8; 4];
                            for (slot, x) in f.iter_mut().zip(key.iter().take(*l)) {
                                *slot = *x;
                            }
                            let combo = Combo { n: u8::try_from(4 + l).unwrap_or(5), f };
                            eval_list(&mut st, unit, combo, ps, flips, None);
                        }
                    }
                }
                threads_done.fetch_add(1, Ordering::Relaxed);
                TASKS.fetch_add(1, Ordering::Relaxed);
            }
            st
        })
        .reduce(St::new, St::merge);
    // sequences absent from the unit are tried too: empty kept sets, no-trade rows
    let present = u64::try_from(seq_list.len()).unwrap_or(0);
    let mut st = st;
    st.tried += 6 * (lib.counts.seqs - present);
    st
}

/// What one timeframe run produced.
pub(crate) struct TfOut {
    pub(crate) ti: usize,
    pub(crate) heaps: Heaps,
    pub(crate) luck: Luck,
    pub(crate) tried: u64,
    pub(crate) tried_buildup: u64,
    pub(crate) bare: Vec<String>,
    pub(crate) singles: Vec<String>,
    pub(crate) filters: Vec<String>,
    pub(crate) units: Vec<(usize, usize, usize)>,
    pub(crate) secs: u64,
}

/// Phase B of one timeframe: every unit, then the build-up.
#[allow(clippy::too_many_lines, reason = "one timeframe run")]
pub(crate) fn run_tf(out: &Path, syms: &[Sym], ti: usize, lib: &Lib, twin: Option<u64>, light: bool, log: &dyn Fn(String)) -> Result<TfOut, String> {
    let t0 = Instant::now();
    let fl = flips();
    let mut lanes: Vec<Vec<Hot>> = Vec::new();
    for lane in 0..lanes_of(ti) {
        let v = load_hot(out, ti, lane, syms)?;
        log(format!("tf {} min lane X{}: {} records loaded", TFS.get(ti).copied().unwrap_or(0), lane + 1, v.len()));
        lanes.push(v);
    }
    let mut total = St::new();
    let mut filters_lines: Vec<String> = Vec::new();
    let mut units: Vec<(usize, usize, usize)> = Vec::new();
    let done = AtomicU64::new(0);
    for (lane, recs) in lanes.iter().enumerate() {
        for core in 0..N_CORE {
            let tu = Instant::now();
            let unit = build_unit(recs, ti, core, lane, twin);
            let n = unit.idx.len();
            units.push((core, lane, n));
            for f in 0..N_FILT {
                let k: u64 = unit.bitset(f).iter().map(|w| u64::from(w.count_ones())).sum();
                let pm = if n > 0 { (1_000 * k).div_euclid(u64::try_from(n).unwrap_or(1)) } else { 0 };
                filters_lines.push(format!(
                    "{}\t{}\tX{}\t{}\t{}\t{}\t{}",
                    TFS.get(ti).copied().unwrap_or(0),
                    crate::common::CORE_NAMES.get(core).copied().unwrap_or("?"),
                    lane + 1,
                    DEFS.get(f).map_or("?", |d| d.id),
                    n,
                    k,
                    pm
                ));
            }
            stage(format!("phase B tf {} min {} X{} ({} records)", TFS.get(ti).copied().unwrap_or(0), crate::common::CORE_NAMES.get(core).copied().unwrap_or("?"), lane + 1, n));
            let st = run_unit(&unit, lib, &fl, &done, light);
            log(format!(
                "tf {} min {} X{}: {} records, {} rows tried in {} s (luck series evaluated {} of {})",
                TFS.get(ti).copied().unwrap_or(0),
                crate::common::CORE_NAMES.get(core).copied().unwrap_or("?"),
                lane + 1,
                n,
                st.tried,
                tu.elapsed().as_secs(),
                st.luck.evaluated,
                st.luck.offered
            ));
            total = total.merge(st);
        }
    }
    // The build-up (ADDENDUM 3 (2)): every heap entry of fewer than four fixed filters,
    // extended by one more filter of another group, offered to its own window only.
    let mut ext: BTreeMap<(usize, usize, Combo), u8> = BTreeMap::new();
    let mut ext1: BTreeMap<(usize, usize, Combo, usize), u8> = BTreeMap::new();
    for w in 0..N_WIN {
        for s in 0..N_SCOPE_TF {
            let Some(h) = total.heaps.get(w, s) else { continue };
            for e in h.v.iter().take(if light { 5 } else { 20 }) {
                let r = crate::search::unpack(e.key.id);
                if r.combo.n >= 4 {
                    continue;
                }
                let used: BTreeSet<u8> = r.combo.filters().iter().map(|&f| lib.grp(f)).collect();
                for f in 0..N_FILT {
                    if used.contains(&lib.grp(f)) {
                        continue;
                    }
                    if let Some(c) = r.combo.with(f) {
                        let all_base = c.filters().iter().all(|&x| x < N_BASE);
                        if light {
                            // light: singles and base pairs were tried; everything else is new
                            if c.n <= 1 || (c.n == 2 && all_base) {
                                continue;
                            }
                            let m = ext1.entry((r.core, r.lane, c, r.side)).or_insert(0);
                            *m |= 1 << w;
                            continue;
                        }
                        // pairs and base triples were all tried in the exhaustive pass
                        if c.n <= 2 || (c.n == 3 && all_base) {
                            continue;
                        }
                        let m = ext.entry((r.core, r.lane, c)).or_insert(0);
                        *m |= 1 << w;
                    }
                }
            }
        }
    }
    let mut by_unit: BTreeMap<(usize, usize), Vec<(Combo, u8)>> = BTreeMap::new();
    for ((core, lane, c), m) in &ext {
        by_unit.entry((*core, *lane)).or_default().push((*c, *m));
    }
    let mut by_unit1: BTreeMap<(usize, usize), Vec<(Combo, usize, u8)>> = BTreeMap::new();
    for ((core, lane, c, side), m) in &ext1 {
        by_unit1.entry((*core, *lane)).or_default().push((*c, *side, *m));
    }
    let mut tried_light = 0_u64;
    for ((core, lane), list) in &by_unit1 {
        let Some(recs) = lanes.get(*lane) else { continue };
        stage(format!("phase B tf {} min build-up (light) {} X{}: {} extensions", TFS.get(ti).copied().unwrap_or(0), crate::common::CORE_NAMES.get(*core).copied().unwrap_or("?"), lane + 1, list.len()));
        let unit = build_unit(recs, ti, *core, *lane, twin);
        let n_chunks = (rayon::current_num_threads() * 4).max(1);
        let st = (0..n_chunks)
            .into_par_iter()
            .map(|ci| {
                let mut st = St::new();
                for (c, side, m) in list.iter().skip(ci).step_by(n_chunks) {
                    let fs = c.filters();
                    let mut acc: Vec<u64> = unit.bitset(fs.first().copied().unwrap_or(0)).to_vec();
                    for &f in fs.iter().skip(1) {
                        for (a, b) in acc.iter_mut().zip(unit.bitset(f).iter()) {
                            *a &= *b;
                        }
                    }
                    for w in 0..N_WIN {
                        if m & (1 << w) != 0 {
                            eval_side(&mut st, &unit, *c, *side, &acc, &fl, w);
                        }
                    }
                }
                st
            })
            .reduce(St::new, St::merge);
        tried_light += st.tried;
        total.heaps.merge(&st.heaps);
        total.luck.merge(&st.luck);
    }
    let mut tried_buildup = tried_light;
    for ((core, lane), list) in &by_unit {
        let Some(recs) = lanes.get(*lane) else { continue };
        let unit = build_unit(recs, ti, *core, *lane, twin);
        let n = unit.idx.len();
        let _ = n;
        let n_chunks = (rayon::current_num_threads() * 4).max(1);
        let st = (0..n_chunks)
            .into_par_iter()
            .map(|ci| {
                let mut st = St::new();
                for (c, m) in list.iter().skip(ci).step_by(n_chunks) {
                    let fs = c.filters();
                    let mut acc: Vec<u64> = unit.bitset(fs.first().copied().unwrap_or(0)).to_vec();
                    for &f in fs.iter().skip(1) {
                        for (a, b) in acc.iter_mut().zip(unit.bitset(f).iter()) {
                            *a &= *b;
                        }
                    }
                    for w in 0..N_WIN {
                        if m & (1 << w) != 0 {
                            eval_bits(&mut st, &unit, *c, &acc, None, None, &fl, Some(w), 0);
                        }
                    }
                }
                st
            })
            .reduce(St::new, St::merge);
        tried_buildup += st.tried;
        total.heaps.merge(&st.heaps);
        total.luck.merge(&st.luck);
    }
    log(format!(
        "tf {} min build-up: {} extensions ({} rows) in {} s total so far",
        TFS.get(ti).copied().unwrap_or(0),
        ext.len() + ext1.len(),
        tried_buildup,
        t0.elapsed().as_secs()
    ));
    // Materialize the survivors' 98-month rows (the heaps kept keys only).
    stage(format!("phase B tf {} min: materializing the heaps' rows", TFS.get(ti).copied().unwrap_or(0)));
    let mut need: BTreeMap<(usize, usize), BTreeSet<Combo>> = BTreeMap::new();
    for h in &total.heaps.h {
        for e in &h.v {
            let r = unpack(e.key.id);
            need.entry((r.core, r.lane)).or_default().insert(r.combo);
        }
    }
    let mut rows: BTreeMap<(usize, usize, Combo), Vec<Row>> = BTreeMap::new();
    for ((core, lane), set) in &need {
        let Some(recs) = lanes.get(*lane) else { continue };
        let unit = build_unit(recs, ti, *core, *lane, twin);
        let list: Vec<Combo> = set.iter().copied().collect();
        let got: Vec<(Combo, Vec<Row>)> = list.par_iter().map(|c| (*c, rows_of(&unit, *c))).collect();
        for (c, r) in got {
            rows.insert((*core, *lane, c), r);
        }
    }
    for h in &mut total.heaps.h {
        for e in &mut h.v {
            let r = unpack(e.key.id);
            if let Some(row) = rows.get(&(r.core, r.lane, r.combo)).and_then(|v| v.get(r.side)) {
                e.row = Some(Box::new(*row));
            }
        }
    }
    Ok(TfOut {
        ti,
        heaps: total.heaps,
        luck: total.luck,
        tried: total.tried,
        tried_buildup,
        bare: total.lines_bare,
        singles: total.lines_single,
        filters: filters_lines,
        units,
        secs: t0.elapsed().as_secs(),
    })
}

/// Save one timeframe run (heaps with their rows, luck, counts, lines) as text.
pub(crate) fn save_tf(p: &Path, t: &TfOut) -> Result<(), String> {
    use std::fmt::Write as _;
    let mut s = String::new();
    let _ = writeln!(s, "TFOUT 1");
    let _ = writeln!(s, "{} {} {} {}", t.ti, t.tried, t.tried_buildup, t.secs);
    let _ = writeln!(s, "{} {} {}", t.luck.offered, t.luck.evaluated, t.luck.best.iter().map(u16::to_string).collect::<Vec<_>>().join(" "));
    let _ = writeln!(s, "{}", t.units.len());
    for (c, l, n) in &t.units {
        let _ = writeln!(s, "{c} {l} {n}");
    }
    let _ = writeln!(s, "{}", t.heaps.h.len());
    for h in &t.heaps.h {
        let _ = writeln!(s, "{}", h.v.len());
        for e in &h.v {
            let k = &e.key;
            let _ = write!(s, "{} {} {} {} {} {} {} {} {} {} {} {} {} {}", k.standing, k.below, k.worst_n, k.worst_d, k.dd, k.fall, k.net, k.wins, k.trades, k.hold, k.gross, k.charges, k.months, k.id);
            match &e.row {
                Some(r) => {
                    let _ = write!(s, " 1");
                    for m in r.iter() {
                        let _ = write!(s, " {} {} {} {} {} {} {} {} {} {}", m.trades, m.wins, m.net, m.gross, m.charges, m.hold, m.end, m.low, m.high, m.fall);
                    }
                }
                None => {
                    let _ = write!(s, " 0");
                }
            }
            let _ = writeln!(s);
        }
    }
    for lines in [&t.bare, &t.singles, &t.filters] {
        let _ = writeln!(s, "{}", lines.len());
        for l in lines {
            let _ = writeln!(s, "{l}");
        }
    }
    let _ = writeln!(s, "END");
    if let Some(d) = p.parent() {
        std::fs::create_dir_all(d).map_err(|e| format!("{}: {e}", d.display()))?;
    }
    std::fs::write(p, s).map_err(|e| format!("{}: {e}", p.display()))
}

/// Load a saved timeframe run.
pub(crate) fn load_tf(p: &Path) -> Result<TfOut, String> {
    let text = std::fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()))?;
    let mut it = text.lines();
    let bad = || format!("{}: not a whole TFOUT file", p.display());
    if it.next() != Some("TFOUT 1") || !text.ends_with("END
") {
        return Err(bad());
    }
    let nums = |l: Option<&str>| -> Vec<i128> { l.unwrap_or("").split(' ').filter_map(|x| x.parse().ok()).collect() };
    let h0 = nums(it.next());
    let g = |v: &[i128], i: usize| v.get(i).copied().unwrap_or(0);
    let lk = nums(it.next());
    let mut luck = Luck::new();
    luck.offered = u64::try_from(g(&lk, 0)).unwrap_or(0);
    luck.evaluated = u64::try_from(g(&lk, 1)).unwrap_or(0);
    for (i, b) in luck.best.iter_mut().enumerate() {
        *b = u16::try_from(g(&lk, 2 + i)).unwrap_or(0);
    }
    luck.merge(&Luck::new());
    let nu = usize::try_from(g(&nums(it.next()), 0)).unwrap_or(0);
    let mut units = Vec::with_capacity(nu);
    for _ in 0..nu {
        let v = nums(it.next());
        units.push((usize::try_from(g(&v, 0)).unwrap_or(0), usize::try_from(g(&v, 1)).unwrap_or(0), usize::try_from(g(&v, 2)).unwrap_or(0)));
    }
    let nh = usize::try_from(g(&nums(it.next()), 0)).unwrap_or(0);
    let mut heaps = Heaps::new(20);
    for hi in 0..nh {
        let ne = usize::try_from(g(&nums(it.next()), 0)).unwrap_or(0);
        for _ in 0..ne {
            let v = nums(it.next());
            let k = crate::acct::Key {
                standing: u8::try_from(g(&v, 0)).unwrap_or(1),
                below: u16::try_from(g(&v, 1)).unwrap_or(0),
                worst_n: g(&v, 2),
                worst_d: g(&v, 3),
                dd: i64::try_from(g(&v, 4)).unwrap_or(0),
                fall: i64::try_from(g(&v, 5)).unwrap_or(0),
                net: i64::try_from(g(&v, 6)).unwrap_or(0),
                wins: u32::try_from(g(&v, 7)).unwrap_or(0),
                trades: u32::try_from(g(&v, 8)).unwrap_or(0),
                hold: i64::try_from(g(&v, 9)).unwrap_or(0),
                gross: i64::try_from(g(&v, 10)).unwrap_or(0),
                charges: i64::try_from(g(&v, 11)).unwrap_or(0),
                months: u16::try_from(g(&v, 12)).unwrap_or(0),
                id: u64::try_from(g(&v, 13)).unwrap_or(0),
            };
            let row = if g(&v, 14) == 1 {
                let mut r = crate::acct::ROW0;
                for (mi, m) in r.iter_mut().enumerate() {
                    let b = 15 + mi * 10;
                    *m = crate::acct::MonthAcc {
                        trades: u32::try_from(g(&v, b)).unwrap_or(0),
                        wins: u32::try_from(g(&v, b + 1)).unwrap_or(0),
                        net: i64::try_from(g(&v, b + 2)).unwrap_or(0),
                        gross: i64::try_from(g(&v, b + 3)).unwrap_or(0),
                        charges: i64::try_from(g(&v, b + 4)).unwrap_or(0),
                        hold: i64::try_from(g(&v, b + 5)).unwrap_or(0),
                        end: i64::try_from(g(&v, b + 6)).unwrap_or(0),
                        low: i64::try_from(g(&v, b + 7)).unwrap_or(0),
                        high: i64::try_from(g(&v, b + 8)).unwrap_or(0),
                        fall: i64::try_from(g(&v, b + 9)).unwrap_or(0),
                    };
                }
                Some(Box::new(r))
            } else {
                None
            };
            if let Some(h) = heaps.h.get_mut(hi) {
                h.v.push(crate::acct::Entry { key: k, row });
            }
        }
    }
    let mut take_lines = |it: &mut std::str::Lines<'_>| -> Vec<String> {
        let n = it.next().and_then(|x| x.parse::<usize>().ok()).unwrap_or(0);
        (0..n).filter_map(|_| it.next().map(str::to_owned)).collect()
    };
    let bare = take_lines(&mut it);
    let singles = take_lines(&mut it);
    let filters = take_lines(&mut it);
    Ok(TfOut {
        ti: usize::try_from(g(&h0, 0)).unwrap_or(0),
        heaps,
        luck,
        tried: u64::try_from(g(&h0, 1)).unwrap_or(0),
        tried_buildup: u64::try_from(g(&h0, 2)).unwrap_or(0),
        bare,
        singles,
        filters,
        units,
        secs: u64::try_from(g(&h0, 3)).unwrap_or(0),
    })
}

/// The hot records of one recipe (its unit's kept list), in admission order.
pub(crate) fn recipe_records(recs: &[Hot], ti: usize, core: usize, lane: usize, combo: Combo, side: usize, twin: Option<u64>) -> Vec<Hot> {
    let unit = build_unit(recs, ti, core, lane, twin);
    let kept = unit.kept(combo);
    let mut v = unit.records(&kept);
    match side {
        0 | 3 => v.retain(|r| r.side == 0),
        1 | 4 => v.retain(|r| r.side == 1),
        _ => {}
    }
    let _ = FW;
    v
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, reason = "tests")]
mod tests {
    use super::*;

    #[test]
    fn the_enumeration_yields_the_spec_counts() {
        let lib = Lib::new();
        assert_eq!(lib.counts.pairs, 2_651);
        assert_eq!(lib.counts.triples, 61_260);
        assert_eq!(lib.counts.spec_cell(), 63_986);
        assert_eq!(lib.counts.l2_pairs, 4_368);
        assert_eq!(lib.counts.per_cell(), 63_986 + 97 + 7_178 + 4_368 + 14_424);
        // groups: 39 in the base library, 20 in the extension
        assert_eq!(lib.n_groups, 59);
    }

    #[test]
    fn a_recipe_id_round_trips() {
        let c = Combo { n: 3, f: [3, 40, 70, 0] };
        let id = pack(5, 4, 5, 5, c, 1_343);
        assert_eq!(unpack(id), Recipe { ti: 5, core: 4, lane: 5, side: 5, combo: c, opt: 1_343 });
        assert!(pack(0, 0, 0, 0, Combo { n: 1, f: [73, 0, 0, 0] }, 0) < pack(0, 0, 0, 0, Combo { n: 2, f: [0, 1, 0, 0] }, 0));
    }

    #[test]
    fn dense_and_list_kept_sets_agree() {
        let mut recs = Vec::new();
        for i in 0..1_000_u64 {
            let x = draw(11, i, 0, 0);
            let mut h = Hot { t: i64::try_from(i).unwrap(), cores: 1, ..Hot::default() };
            h.fbits = [x, draw(12, i, 0, 0), draw(13, i, 0, 0) & ((1 << 43) - 1)];
            recs.push(h);
        }
        let unit = build_unit(&recs, 0, 0, 0, None);
        let c = Combo { n: 3, f: [1, 7, 90, 0] };
        let kept = unit.kept(c);
        let want: Vec<u32> = (0..1_000_u32)
            .filter(|&p| {
                let r = &recs[usize::try_from(p).unwrap()];
                crate::filters::get(&r.fbits, 1) && crate::filters::get(&r.fbits, 7) && crate::filters::get(&r.fbits, 90)
            })
            .collect();
        assert_eq!(kept, want);
        let pair: Vec<u32> = unit.kept(Combo { n: 2, f: [1, 7, 0, 0] });
        let tri: Vec<u32> = pair.iter().copied().filter(|&p| crate::filters::get(&recs[usize::try_from(p).unwrap()].fbits, 90)).collect();
        assert_eq!(tri, want);
    }

    #[test]
    fn a_twin_seed_is_reproducible_and_keeps_every_size() {
        let mut recs = Vec::new();
        for i in 0..700_u64 {
            let mut h = Hot { t: i64::try_from(i).unwrap(), cores: 1, ..Hot::default() };
            h.fbits = [draw(1, i, 0, 0), draw(2, i, 0, 0), draw(3, i, 0, 0) & ((1 << 43) - 1)];
            h.codes = [u8::try_from(i % 24).unwrap(), 3, 5];
            recs.push(h);
        }
        let real = build_unit(&recs, 1, 0, 2, None);
        let a = build_unit(&recs, 1, 0, 2, Some(1));
        let b = build_unit(&recs, 1, 0, 2, Some(1));
        assert_eq!(a.bits, b.bits);
        assert_eq!(a.codes, b.codes);
        for f in 0..N_FILT {
            let k1: u32 = real.bitset(f).iter().map(|w| w.count_ones()).sum();
            let k2: u32 = a.bitset(f).iter().map(|w| w.count_ones()).sum();
            assert_eq!(k1, k2, "filter {f}");
        }
        let mut c1: Vec<[u8; 3]> = real.codes.clone();
        let mut c2: Vec<[u8; 3]> = a.codes.clone();
        c1.sort_unstable();
        c2.sort_unstable();
        assert_eq!(c1, c2);
    }
}

//! The gap-skip tables of design 4.3 (critic B F1), step A0: per instrument-day and per LATTICE PARITY (an order's anchor second is even
//! or odd from the index start), the full 2-second blocks read once by the tape's own block read ([`crate::Tape::block`]), then:
//!
//! - per dial level: `next_capable[j]` (the next block whose capacity is at least one unit) and the capacity prefix sums (O(1) range sums,
//!   [`crate::capacity_between`]);
//! - the traded-block and any-row-block prefix counts and `prev_any` (exact counters for a skipped run);
//! - LAZY sparse tables of block HIGH / LOW (built on the first descent): the first block past, or within, a protection limit in O(log S).
//!
//! The walk ([`crate::walk`]) skips an unproductive run only after reading 16 unproductive blocks in a row, and only when no block row is
//! recorded, no money cap is read and the order is a market order: a skip never changes a fill or a printed counter (no_trade, past_limit,
//! zero_take_blocks stay exact), only `Steps` (`skips` + 1 instead of the blocks read).

use std::sync::OnceLock;

use crate::tape::{Row, SecIndex, Tape};
use crate::{BLOCK_S, Dial, FillErr, MAXL, Paisa, Side, US_S, Units, Us};

/// "None" in the u32 block arrays.
pub(crate) const NONE: u32 = u32::MAX;

/// The units a block of `vol` allows at `bp` of its volume in whole `unit`s (sample-6's `block_take` through `options::avail`).
pub(crate) fn cap_units(vol: Units, bp: u16, unit: Units) -> Result<Units, FillErr> {
    let raw = vol.checked_mul(Units::from(bp)).ok_or(FillErr::Overflow)? / 10_000;
    let u = unit.max(1);
    Ok(raw / u * u)
}

/// A sparse table over the blocks of one parity: `t[k][j]` = the max (or min) of `v[j .. j + 2^k)`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Sparse {
    t: Vec<Vec<Paisa>>,
    max: bool,
}

impl Sparse {
    fn new(v: Vec<Paisa>, max: bool) -> Self {
        let n = v.len();
        let mut t = vec![v];
        let mut k = 1_usize;
        while (1_usize << k) <= n {
            let half = 1_usize << (k - 1);
            let prev = t.get(k - 1).map_or(&[][..], Vec::as_slice);
            let m = n + 1 - (1_usize << k);
            let mut nx = Vec::with_capacity(m);
            for j in 0..m {
                let (a, b) = (prev.get(j).copied().unwrap_or(0), prev.get(j + half).copied().unwrap_or(0));
                nx.push(if max { a.max(b) } else { a.min(b) });
            }
            t.push(nx);
            k += 1;
        }
        Self { t, max }
    }

    /// The first index >= `j` whose value passes `hit` (a max table: value > x when `strict`, else >= x; a min table: value < x when
    /// `strict`, else <= x), by binary lifting: O(log n) reads, counted in `steps`.
    fn first(&self, j: usize, x: Paisa, strict: bool, steps: &mut u32) -> Option<usize> {
        let n = self.t.first().map_or(0, Vec::len);
        if j >= n {
            return None;
        }
        let hit = |v: Paisa| match (self.max, strict) {
            (true, true) => v > x,
            (true, false) => v >= x,
            (false, true) => v < x,
            (false, false) => v <= x,
        };
        let mut pos = j;
        let top = self.t.len();
        for k in (0..top).rev() {
            let len = 1_usize << k;
            if pos + len <= n {
                *steps = steps.saturating_add(1);
                if let Some(&v) = self.t.get(k).and_then(|r| r.get(pos))
                    && !hit(v)
                {
                    pos += len;
                }
            }
        }
        *steps = steps.saturating_add(1);
        let v = *self.t.first()?.get(pos)?;
        hit(v).then_some(pos)
    }
}

/// One lattice parity's full blocks.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Parity {
    hi: Vec<Paisa>,
    lo: Vec<Paisa>,
    traded: Vec<bool>,
    /// Prefix count of traded blocks (len nb + 1).
    ntr: Vec<u32>,
    /// Prefix count of blocks holding ANY row (a print or not; len nb + 1): the empty blocks of a skipped run are the others.
    nany: Vec<u32>,
    /// `prev_any[j]` = 1 + the index of the last block holding a row before j (0 when none), len nb + 1.
    prev_any: Vec<u32>,
    /// Per level: the next block >= j whose capacity is at least one unit (len nb + 1; NONE when none).
    next_cap: Vec<Vec<u32>>,
    /// Per level: prefix sums of block capacity (len nb + 1).
    capsum: Vec<Vec<Units>>,
    maxh: OnceLock<Sparse>,
    minh: OnceLock<Sparse>,
    minl: OnceLock<Sparse>,
    maxl: OnceLock<Sparse>,
}

/// The gap tables of one instrument-day for one dial and one unit (design 4.3, 11.1). Built once per instrument-day by the host
/// (O(session seconds x levels)), shared read only (`&GapTables: Sync`; the lazy sparse tables are `OnceLock`s).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GapTables {
    t0: Us,
    span: usize,
    dial: Dial,
    unit: Units,
    par: [Parity; 2],
}

impl GapTables {
    /// The tables of `tape` (its index span) for `dial` and the unit `unit` (1 share, or the contract's lot).
    ///
    /// # Errors
    /// [`FillErr::Overflow`] when a block's volume leaves u64; [`FillErr::DialInvalid`] never (the dial is valid by construction).
    pub fn new<R: Row>(tape: &Tape<'_, R>, dial: &Dial, unit: Units) -> Result<Self, FillErr> {
        let ix = tape.index();
        let (t0, span) = (ix.t0(), ix.span_secs());
        let unit = unit.max(1);
        let mk = |p: usize| -> Result<Parity, FillErr> {
            let nb = span.saturating_sub(p).div_ceil(2);
            let mut hi = Vec::with_capacity(nb);
            let mut lo = Vec::with_capacity(nb);
            let mut traded = Vec::with_capacity(nb);
            let mut anyrow = Vec::with_capacity(nb);
            let mut vols = Vec::with_capacity(nb);
            for j in 0..nb {
                let off = i64::try_from(p + 2 * j).map_err(|_| FillErr::Overflow)?;
                let at = t0.checked_add(off.checked_mul(US_S).ok_or(FillErr::Overflow)?).ok_or(FillErr::Overflow)?;
                let b = tape.block(at, Us::MAX)?;
                let to = at.checked_add(2 * US_S).ok_or(FillErr::Overflow)?;
                anyrow.push(tape.first_at(to) > tape.first_at(at));
                hi.push(b.high);
                lo.push(b.low);
                traded.push(b.n > 0);
                vols.push(b.vol);
            }
            let mut ntr = Vec::with_capacity(nb + 1);
            let mut nany = Vec::with_capacity(nb + 1);
            let mut prev_any = Vec::with_capacity(nb + 1);
            ntr.push(0_u32);
            nany.push(0_u32);
            prev_any.push(0_u32);
            for (j, (&t, &a)) in traded.iter().zip(anyrow.iter()).enumerate() {
                let last = ntr.last().copied().unwrap_or(0);
                ntr.push(last.saturating_add(u32::from(t)));
                let la = nany.last().copied().unwrap_or(0);
                nany.push(la.saturating_add(u32::from(a)));
                let pa = prev_any.last().copied().unwrap_or(0);
                prev_any.push(if a { u32::try_from(j + 1).unwrap_or(NONE) } else { pa });
            }
            let mut next_cap = Vec::with_capacity(dial.len());
            let mut capsum = Vec::with_capacity(dial.len());
            for &bp in dial.levels() {
                let mut caps = Vec::with_capacity(nb);
                for &v in &vols {
                    caps.push(cap_units(v, bp, unit)?);
                }
                let mut nc = vec![NONE; nb + 1];
                for j in (0..nb).rev() {
                    let here = caps.get(j).copied().unwrap_or(0) >= unit;
                    let nxt = nc.get(j + 1).copied().unwrap_or(NONE);
                    if let Some(x) = nc.get_mut(j) {
                        *x = if here { u32::try_from(j).unwrap_or(NONE) } else { nxt };
                    }
                }
                let mut cs = Vec::with_capacity(nb + 1);
                cs.push(0_u64);
                for &c in &caps {
                    let last = cs.last().copied().unwrap_or(0);
                    cs.push(last.checked_add(c).ok_or(FillErr::Overflow)?);
                }
                next_cap.push(nc);
                capsum.push(cs);
            }
            Ok(Parity { hi, lo, traded, ntr, nany, prev_any, next_cap, capsum, maxh: OnceLock::new(), minh: OnceLock::new(), minl: OnceLock::new(), maxl: OnceLock::new() })
        };
        Ok(Self { t0, span, dial: *dial, unit, par: [mk(0)?, mk(1)?] })
    }

    pub(crate) fn fits(&self, ix: &SecIndex) -> bool {
        self.t0 == ix.t0() && self.span == ix.span_secs()
    }

    /// Whether these tables serve a walk of `dial` in units of `unit` (the skips need the same levels and unit; 2-second blocks only).
    pub(crate) fn serves(&self, dial: &Dial, unit: Units) -> bool {
        BLOCK_S == 2 && self.dial == *dial && self.unit == unit.max(1)
    }

    /// The lattice parity and block index of the block starting at `at` (None when `at` is before the index span or off whole seconds).
    pub(crate) fn locate(&self, at: Us) -> Option<(usize, usize)> {
        let off = at.checked_sub(self.t0)?;
        if off < 0 || off.rem_euclid(US_S) != 0 {
            return None;
        }
        let s = usize::try_from(off / US_S).ok()?;
        let p = s % 2;
        Some((p, (s - p) / 2))
    }

    /// The start of block `j` of parity `p`.
    pub(crate) fn start_of(&self, p: usize, j: usize) -> Option<Us> {
        let off = i64::try_from(p + 2 * j).ok()?.checked_mul(US_S)?;
        self.t0.checked_add(off)
    }

    /// The number of full blocks of parity `p`.
    pub(crate) fn blocks(&self, p: usize) -> usize {
        self.par.get(p).map_or(0, |x| x.traded.len())
    }

    /// The first block >= j whose capacity at dial level `l` is at least one unit (None when none).
    pub(crate) fn next_capable(&self, p: usize, l: usize, j: usize) -> Option<usize> {
        let v = *self.par.get(p)?.next_cap.get(l)?.get(j)?;
        (v != NONE).then(|| usize::try_from(v).ok()).flatten()
    }

    /// Traded blocks in [j0, j1).
    pub(crate) fn traded_between(&self, p: usize, j0: usize, j1: usize) -> u32 {
        let Some(x) = self.par.get(p) else { return 0 };
        let a = x.ntr.get(j0).copied().unwrap_or(0);
        let b = x.ntr.get(j1).copied().unwrap_or(a);
        b.saturating_sub(a)
    }

    /// Blocks holding any row in [j0, j1).
    pub(crate) fn any_between(&self, p: usize, j0: usize, j1: usize) -> u32 {
        let Some(x) = self.par.get(p) else { return 0 };
        let a = x.nany.get(j0).copied().unwrap_or(0);
        let b = x.nany.get(j1).copied().unwrap_or(a);
        b.saturating_sub(a)
    }

    /// 1 + the last block holding a row before `j` (0 when none).
    pub(crate) fn prev_any_end(&self, p: usize, j: usize) -> usize {
        self.par.get(p).and_then(|x| x.prev_any.get(j)).and_then(|&v| usize::try_from(v).ok()).unwrap_or(0)
    }

    /// The capacity sum of blocks [j0, j1) of parity `p` at dial level `l`.
    pub(crate) fn capacity(&self, p: usize, l: usize, j0: usize, j1: usize) -> Option<Units> {
        let cs = self.par.get(p)?.capsum.get(l)?;
        let a = *cs.get(j0)?;
        let b = *cs.get(j1)?;
        b.checked_sub(a)
    }

    /// The dial level index of `bp`, if these tables were built for it at `unit`.
    pub(crate) fn level_of(&self, bp: u16, unit: Units) -> Option<usize> {
        if self.unit != unit.max(1) || BLOCK_S != 2 {
            return None;
        }
        self.dial.levels().iter().position(|&b| b == bp)
    }

    /// The first TRADED block >= j that is PAST the limit `lim` for `side` (a buy: HIGH above it; a sale: LOW below it): O(log S).
    pub(crate) fn first_past(&self, p: usize, j: usize, side: Side, lim: Paisa, steps: &mut u32) -> Option<usize> {
        let x = self.par.get(p)?;
        match side {
            Side::Buy => x.maxh.get_or_init(|| Sparse::new(sent(&x.hi, &x.traded, Paisa::MIN), true)).first(j, lim, true, steps),
            Side::Sell => x.minl.get_or_init(|| Sparse::new(sent(&x.lo, &x.traded, Paisa::MAX), false)).first(j, lim, true, steps),
        }
    }

    /// The first TRADED block >= j WITHIN the limit `lim` for `side` (a buy: HIGH at or below it; a sale: LOW at or above it): O(log S).
    pub(crate) fn first_within(&self, p: usize, j: usize, side: Side, lim: Paisa, steps: &mut u32) -> Option<usize> {
        let x = self.par.get(p)?;
        match side {
            Side::Buy => x.minh.get_or_init(|| Sparse::new(sent(&x.hi, &x.traded, Paisa::MAX), false)).first(j, lim, false, steps),
            Side::Sell => x.maxl.get_or_init(|| Sparse::new(sent(&x.lo, &x.traded, Paisa::MIN), true)).first(j, lim, false, steps),
        }
    }

    /// The levels these tables were built for (bp), MAXL wide.
    #[must_use]
    pub fn levels(&self) -> &[u16] {
        self.dial.levels()
    }

    /// The unit these tables were built for.
    #[must_use]
    pub const fn unit(&self) -> Units {
        self.unit
    }

    /// The parity count (2) times the levels: for the host's memory line.
    #[must_use]
    pub fn tables(&self) -> usize {
        2 * self.dial.len().min(MAXL)
    }
}

/// The block prices with an untraded block replaced by `s` (so it never hits a descent).
fn sent(v: &[Paisa], traded: &[bool], s: Paisa) -> Vec<Paisa> {
    v.iter().zip(traded).map(|(&x, &t)| if t { x } else { s }).collect()
}

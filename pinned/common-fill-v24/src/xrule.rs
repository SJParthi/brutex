//! CRATE v2.2 (ADDITIVE; main 5b507203, 15 Nov 2026; EXITS-DYNAMIC-DESIGN B1 as signed in MAIN-SIGNED-EXITS-DYNAMIC-20261115 with the
//! critic's R3): the extended exit rules of the s25b menu (SL / TSL / TP / TTP, fixed and dynamic) and their bounded-step evaluation.
//!
//! Nothing of crate v2.1 (FA1 copy) changes: `path.rs` (`Rule`, `scan_one`, `trail_hit`), `tape.rs` and `std70.rs` are byte for byte the
//! FA1 copy's. This module only ADDS:
//! - [`XRule`]: crate v2.1's six rules plus five new ones: [`XRule::StopAt`], [`XRule::TargetAt`], [`XRule::TrailPct`],
//!   [`XRule::ArmedStop`], [`XRule::TimedStop`];
//! - [`scan_law`]: THE LAW of every rule: a plain walk, traded SECOND by traded second (a repeated stamp merged into its second: max
//!   HIGH, min LOW, critic R3), R(s) read per second, adverse-first; it shares no search code with the fast path;
//! - [`TrailIndex`]: per contract-day, over the per-second MERGED traded seconds (R3), the next-strictly-better (favourable) chain, each
//!   node's give-back `g` (the most adverse extreme over its piece minus its favourable extreme) and relative key `kap`, with binary
//!   lifting over both;
//! - [`scan_fast`]: every rule in a bounded number of steps: fixed levels by the crate's own piecewise `first_reach` (crate v2.1's
//!   `scan_one` reading), trails by the chain. It equals [`crate::path::scan_one`] on every crate v2.1 rule and [`scan_law`] on every rule
//!   (tests_v22.rs, the node vectors `vectors/XRULE-VECTORS-v22.tsv`, proof P3 on the real days).
//!
//! Sides: a SOLD option (short) loses when the premium rises (adverse = HIGH up, favourable = LOW down); a long the mirror. Every level is
//! saturating (a level beyond i64 is never reached by a print). Integers only.

use crate::path::{EntryView, Rule, Trigger};
use crate::tape::{Row, Tape};
use crate::{Paisa, US_S, Us};

/// One exit rule of the extended menu (distances in paisa from R(s); absolute levels in paisa).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum XRule {
    /// crate v2.1 `Rule::Stop`: adverse extreme reaches R(s) -/+ x.
    Stop {
        /// Distance (paisa).
        x: Paisa,
    },
    /// crate v2.1 `Rule::QuickCut` (a stop).
    QuickCut {
        /// Distance (paisa).
        x: Paisa,
    },
    /// crate v2.1 `Rule::GoPositive`.
    GoPositive {
        /// Seconds after E0.
        secs: u32,
    },
    /// crate v2.1 `Rule::TimeExit`.
    TimeExit {
        /// Seconds after E0.
        secs: u32,
    },
    /// crate v2.1 `Rule::Trail`: wakes when the favourable extreme reaches R(s) +/- act; then fires when the adverse extreme reaches the best
    /// favourable extreme since the wake -/+ dist (the level in force BEFORE the second: adverse-first).
    Trail {
        /// Activation distance (paisa, may be 0).
        act: Paisa,
        /// Trailing distance (paisa).
        dist: Paisa,
    },
    /// crate v2.1 `Rule::Target`: favourable extreme reaches R(s) +/- x (reaching is enough).
    Target {
        /// Distance (paisa).
        x: Paisa,
    },
    /// NEW: an ABSOLUTE stop level (a short: HIGH >= level; a long: LOW <= level).
    StopAt {
        /// The level (paisa).
        level: Paisa,
    },
    /// NEW: an ABSOLUTE target level (a short: LOW <= level; a long: HIGH >= level).
    TargetAt {
        /// The level (paisa).
        level: Paisa,
    },
    /// NEW: a RELATIVE trail: wakes as [`XRule::Trail`]; the level follows the running best m: a short's m + max(1, floor(m x bp / 10,000)),
    /// a long's m - max(1, floor(m x bp / 10,000)).
    TrailPct {
        /// Activation distance (paisa).
        act: Paisa,
        /// The give-back in basis points of m.
        bp: u32,
    },
    /// NEW: a breakeven move: wakes at the first traded second whose favourable extreme reaches R(s) +/- act; from the NEXT traded second
    /// a stop at R(s) + off (a short; a long R(s) - off; off may be negative: a locked profit).
    ArmedStop {
        /// Activation distance (paisa).
        act: Paisa,
        /// The stop's offset from R(s) (paisa, may be negative).
        off: Paisa,
    },
    /// NEW: a time-armed stop: from E0 + `after` seconds (inclusive), a stop at R(s) + off (a short; a long R(s) - off).
    TimedStop {
        /// Seconds after E0.
        after: u32,
        /// The stop's offset from R(s) (paisa).
        off: Paisa,
    },
}

impl From<Rule> for XRule {
    fn from(r: Rule) -> Self {
        match r {
            Rule::Stop { x } => Self::Stop { x },
            Rule::QuickCut { x } => Self::QuickCut { x },
            Rule::GoPositive { secs } => Self::GoPositive { secs },
            Rule::TimeExit { secs } => Self::TimeExit { secs },
            Rule::Trail { act, dist } => Self::Trail { act, dist },
            Rule::Target { x } => Self::Target { x },
        }
    }
}

/// R(s)'s pieces, exactly crate v2.1's `path::reference_pieces` (private there; copied here so `path.rs` stays byte for byte): (from
/// instant, R) in time order, a piece at each filling block's START, the running average rounded against the position.
#[must_use]
pub fn pieces(fills: &[crate::std70::BlockFill], long: bool) -> Vec<(Us, Paisa)> {
    let mut out: Vec<(Us, Paisa)> = Vec::with_capacity(fills.len());
    let first = fills.first().map_or(0, |b| b.px);
    if cfg!(feature = "m_sf3_r_from_p0") {
        // MUTANT m_sf3_r_from_p0: every level from p0 (the first fill), never the running R(s).
        if let Some(b) = fills.first() {
            out.push((b.block.at, first));
        }
        return out;
    }
    let (mut u, mut v) = (0_i128, 0_i128);
    for b in fills {
        u += i128::from(b.units);
        v += i128::from(b.units) * i128::from(b.px);
        let avg = if u == 0 { first } else if long { i64::try_from((v + u - 1).div_euclid(u)).unwrap_or(i64::MAX) } else { i64::try_from(v.div_euclid(u)).unwrap_or(0) };
        let r = if long { first.max(avg) } else { first.min(avg) };
        match out.last_mut() {
            Some(last) if last.0 == b.block.at => last.1 = r,
            _ => out.push((b.block.at, r)),
        }
    }
    out
}

/// R at instant `t` from the pieces (the first fill price before the first piece).
#[must_use]
pub fn r_at(pieces: &[(Us, Paisa)], first_px: Paisa, t: Us) -> Paisa {
    pieces.iter().rev().find(|p| p.0 <= t).map_or(first_px, |p| p.1)
}

#[inline]
const fn adv_level(long: bool, r: Paisa, x: Paisa) -> Paisa {
    if long { r.saturating_sub(x) } else { r.saturating_add(x) }
}

#[inline]
const fn fav_level(long: bool, r: Paisa, x: Paisa) -> Paisa {
    if long { r.saturating_add(x) } else { r.saturating_sub(x) }
}

/// The relative trail's distance at the running best `m` (a price): max(1, floor(m x bp / 10,000)).
#[inline]
#[must_use]
pub fn pct_dist(m: Paisa, bp: u32) -> Paisa {
    let d = i128::from(m.max(0)) * i128::from(bp) / 10_000;
    i64::try_from(d).unwrap_or(i64::MAX).max(1)
}

/// One traded second, merged (max HIGH, min LOW over its rows with volume and a valid price).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MSec {
    /// The second's start.
    pub ts: Us,
    /// Its merged HIGH.
    pub high: Paisa,
    /// Its merged LOW.
    pub low: Paisa,
}

/// The merged traded seconds stamped in [from, to) (THE LAW's view: a row with no volume is no print, crate C2).
#[must_use]
pub fn merged<R: Row>(tape: &Tape<'_, R>, from: Us, to: Us) -> Vec<MSec> {
    let i0 = tape.first_at(from);
    let mut out: Vec<MSec> = Vec::new();
    for s in tape.rows().get(i0..).unwrap_or(&[]) {
        let ts = s.ts();
        if ts >= to {
            break;
        }
        if !(s.vol() > 0 && s.high() > 0 && s.low() > 0 && s.high() >= s.low()) {
            continue;
        }
        match out.last_mut() {
            Some(m) if m.ts == ts => {
                m.high = m.high.max(s.high());
                m.low = m.low.min(s.low());
            }
            _ => out.push(MSec { ts, high: s.high(), low: s.low() }),
        }
    }
    out
}

/// THE LAW of one level's extended rules over [E0, `to`): a plain walk over the merged traded seconds, per rule, adverse-first. O(window)
/// per rule: the reference, never the engine.
#[must_use]
pub fn scan_law<R: Row>(tape: &Tape<'_, R>, e: &EntryView<'_>, to: Us, long: bool, rules: &[XRule]) -> Vec<Option<Trigger>> {
    let from = e.e0;
    let pcs = pieces(e.fills, long);
    let first_px = e.fills.first().map_or(0, |b| b.px);
    let secs = merged(tape, from, to);
    let r = |t: Us| r_at(&pcs, first_px, t);
    let adv = |m: &MSec| if long { m.low } else { m.high };
    let fav = |m: &MSec| if long { m.high } else { m.low };
    let adv_hit = |p: Paisa, level: Paisa| if long { p <= level } else { p >= level };
    let fav_hit = |p: Paisa, level: Paisa| if long { p >= level } else { p <= level };
    let price = |s: Us, level: Paisa| Some(Trigger { sec: s, anchor: (s + US_S).max(e.e1), level });
    let mut out = Vec::with_capacity(rules.len());
    for rule in rules {
        let t = match *rule {
            XRule::Stop { x } | XRule::QuickCut { x } => secs.iter().find(|m| adv_hit(adv(m), adv_level(long, r(m.ts), x))).and_then(|m| price(m.ts, adv_level(long, r(m.ts), x))),
            XRule::Target { x } => secs.iter().find(|m| fav_hit(fav(m), fav_level(long, r(m.ts), x))).and_then(|m| price(m.ts, fav_level(long, r(m.ts), x))),
            XRule::StopAt { level } => secs.iter().find(|m| adv_hit(adv(m), level)).and_then(|m| price(m.ts, level)),
            XRule::TargetAt { level } => secs.iter().find(|m| fav_hit(fav(m), level)).and_then(|m| price(m.ts, level)),
            XRule::TimeExit { secs: s } => {
                let at = from + i64::from(s) * US_S;
                (at < to).then_some(Trigger { sec: at, anchor: at.max(e.e1), level: 0 })
            }
            XRule::GoPositive { secs: s } => {
                let at = from + i64::from(s) * US_S;
                let last = secs.iter().rev().find(|m| m.ts < at);
                let positive = last.is_some_and(|m| if long { m.low > r(m.ts) } else { m.high < r(m.ts) });
                (at < to && !positive).then_some(Trigger { sec: at, anchor: at.max(e.e1), level: last.map_or(first_px, |m| r(m.ts)) })
            }
            XRule::Trail { act, dist } => {
                let mut best: Option<Paisa> = None;
                let mut hit = None;
                for m in &secs {
                    if let Some(b) = best {
                        let level = if long { b.saturating_sub(dist) } else { b.saturating_add(dist) };
                        if adv_hit(adv(m), level) {
                            hit = price(m.ts, level);
                            break;
                        }
                    }
                    if best.is_some() || fav_hit(fav(m), fav_level(long, r(m.ts), act)) {
                        best = Some(best.map_or(fav(m), |b| if long { b.max(fav(m)) } else { b.min(fav(m)) }));
                    }
                }
                hit
            }
            XRule::TrailPct { act, bp } => {
                let mut best: Option<Paisa> = None;
                let mut hit = None;
                for m in &secs {
                    if let Some(b) = best {
                        let d = pct_dist(b, bp);
                        let level = if long { b.saturating_sub(d) } else { b.saturating_add(d) };
                        if adv_hit(adv(m), level) {
                            hit = price(m.ts, level);
                            break;
                        }
                    }
                    if best.is_some() || fav_hit(fav(m), fav_level(long, r(m.ts), act)) {
                        best = Some(best.map_or(fav(m), |b| if long { b.max(fav(m)) } else { b.min(fav(m)) }));
                    }
                }
                hit
            }
            XRule::ArmedStop { act, off } => {
                let mut armed = false;
                let mut hit = None;
                for m in &secs {
                    if armed {
                        let level = adv_level(long, r(m.ts), off);
                        if adv_hit(adv(m), level) {
                            hit = price(m.ts, level);
                            break;
                        }
                    }
                    if !armed && fav_hit(fav(m), fav_level(long, r(m.ts), act)) {
                        armed = true;
                    }
                }
                hit
            }
            XRule::TimedStop { after, off } => {
                let at = from + i64::from(after) * US_S;
                secs.iter().filter(|m| m.ts >= at).find(|m| adv_hit(adv(m), adv_level(long, r(m.ts), off))).and_then(|m| price(m.ts, adv_level(long, r(m.ts), off)))
            }
        };
        out.push(t);
    }
    out
}

/// The sentinel "no node" of the chain.
const NONE: u32 = u32::MAX;

/// The trail lookup of one contract-day and one side (critic R3: over the per-second MERGED traded seconds). In the side's own space a
/// long's prices are negated, so one chain serves both: `f` = the favourable extreme (a short's LOW, a long's -HIGH), `a` = the adverse
/// extreme (a short's HIGH, a long's -LOW); `nxt[i]` = the next node with a strictly lower `f` (a new best); on the piece (c, nxt[c]] the
/// trail's level is constant (f[c] + dist), and the new best at nxt[c] is tested against the old level first (adverse-first, exactly
/// crate v2.1's `trail_hit`). `g[c]` = max a over (c, nxt[c]] - f[c] (no node after c: never); `kap[c]` = ceil(10,000 (g + 1) / |f[c]|) - 1
/// for g >= 1 (else never): a relative trail of bp fires on piece c iff bp <= kap[c]. Binary lifting `up` / `mg` / `mk` over the chain.
/// Build O(n log n), at most n x 15 x 20 bytes; every query O(log n) steps.
#[derive(Clone, Debug, Default)]
pub struct TrailIndex {
    long: bool,
    secs: Vec<Us>,
    f: Vec<Paisa>,
    base: Vec<Paisa>,
    g: Vec<i64>,
    kap: Vec<i64>,
    up: Vec<Vec<u32>>,
    mg: Vec<Vec<i64>>,
    mk: Vec<Vec<i64>>,
    end_ts: Us,
}

impl TrailIndex {
    /// The lookup of `tape` for a long (`long`) or a short position.
    #[must_use]
    #[allow(clippy::too_many_lines, reason = "one O(n log n) build: merge, chain, sparse table, give-backs, lifting")]
    pub fn new<R: Row>(tape: &Tape<'_, R>, long: bool) -> Self {
        // merged traded seconds (MUTANT m_sf3_trail_rows: one node per ROW; MUTANT m_sf3_vol0_prints: rows with no volume count)
        let mut ms: Vec<MSec> = Vec::with_capacity(tape.rows().len());
        for s in tape.rows() {
            let ok = if cfg!(feature = "m_sf3_vol0_prints") { s.high() > 0 && s.low() > 0 && s.high() >= s.low() } else { s.vol() > 0 && s.high() > 0 && s.low() > 0 && s.high() >= s.low() };
            if !ok {
                continue;
            }
            match ms.last_mut() {
                Some(m) if m.ts == s.ts() && !cfg!(feature = "m_sf3_trail_rows") => {
                    m.high = m.high.max(s.high());
                    m.low = m.low.min(s.low());
                }
                _ => ms.push(MSec { ts: s.ts(), high: s.high(), low: s.low() }),
            }
        }
        let n = ms.len();
        let secs: Vec<Us> = ms.iter().map(|m| m.ts).collect();
        let f: Vec<Paisa> = ms.iter().map(|m| if long { -m.high } else { m.low }).collect();
        let a: Vec<Paisa> = ms.iter().map(|m| if long { -m.low } else { m.high }).collect();
        let base: Vec<Paisa> = ms.iter().map(|m| if long { m.high } else { m.low }).collect();
        // next strictly lower f (monotonic stack from the right)
        let mut nxt = vec![NONE; n];
        let mut st: Vec<usize> = Vec::new();
        for i in (0..n).rev() {
            let fi = f.get(i).copied().unwrap_or(0);
            while st.last().is_some_and(|&j| f.get(j).copied().unwrap_or(0) >= fi) {
                st.pop();
            }
            if let (Some(x), Some(&j)) = (nxt.get_mut(i), st.last()) {
                *x = u32::try_from(j).unwrap_or(NONE);
            }
            st.push(i);
        }
        // sparse max table of a (for g)
        let mut sa: Vec<Vec<Paisa>> = vec![a.clone()];
        let mut k = 1_usize;
        while (1_usize << k) <= n {
            let half = 1_usize << (k - 1);
            let prev = sa.get(k - 1).cloned().unwrap_or_default();
            let row: Vec<Paisa> = (0..=(n - (1_usize << k))).map(|i| prev.get(i).copied().unwrap_or(i64::MIN).max(prev.get(i + half).copied().unwrap_or(i64::MIN))).collect();
            sa.push(row);
            k += 1;
        }
        let range_max = |lo: usize, hi: usize| -> Paisa {
            // inclusive [lo, hi]
            if hi < lo || hi >= n {
                return i64::MIN;
            }
            let len = hi - lo + 1;
            let kk = usize::try_from(len.ilog2()).unwrap_or(0);
            let t = sa.get(kk);
            let x = t.and_then(|r| r.get(lo)).copied().unwrap_or(i64::MIN);
            let y = t.and_then(|r| r.get(hi + 1 - (1_usize << kk))).copied().unwrap_or(i64::MIN);
            x.max(y)
        };
        let mut g = vec![i64::MIN; n];
        let mut kap = vec![i64::MIN; n];
        for c in 0..n {
            let end = nxt.get(c).copied().filter(|&x| x != NONE).map_or(n.saturating_sub(1), |x| x as usize);
            // MUTANT m_sf3_trail_own_level: the piece includes c itself (a second tested against the level it sets)
            let lo = if cfg!(feature = "m_sf3_trail_own_level") { c } else { c + 1 };
            let mx = range_max(lo, end);
            let fc = f.get(c).copied().unwrap_or(0);
            let gc = if mx == i64::MIN { i64::MIN } else { mx.saturating_sub(fc) };
            if let Some(x) = g.get_mut(c) {
                *x = gc;
            }
            let bc = base.get(c).copied().unwrap_or(1).max(1);
            if gc >= 1
                && let Some(x) = kap.get_mut(c)
            {
                let num = i128::from(gc).saturating_add(1).saturating_mul(10_000);
                let q = num.div_euclid(i128::from(bc));
                let ceil = if q * i128::from(bc) == num { q } else { q + 1 };
                *x = i64::try_from(ceil - 1).unwrap_or(i64::MAX);
            }
        }
        // binary lifting over the chain
        let mut up: Vec<Vec<u32>> = vec![nxt];
        let mut mg: Vec<Vec<i64>> = vec![g.clone()];
        let mut mk: Vec<Vec<i64>> = vec![kap.clone()];
        let mut lv = 1_usize;
        while (1_usize << lv) <= n.max(1) {
            let (pu, pg, pk) = (up.get(lv - 1).cloned().unwrap_or_default(), mg.get(lv - 1).cloned().unwrap_or_default(), mk.get(lv - 1).cloned().unwrap_or_default());
            let mut nu = vec![NONE; n];
            let mut ng = vec![i64::MIN; n];
            let mut nk = vec![i64::MIN; n];
            for i in 0..n {
                let j = pu.get(i).copied().unwrap_or(NONE);
                let (gi, ki) = (pg.get(i).copied().unwrap_or(i64::MIN), pk.get(i).copied().unwrap_or(i64::MIN));
                if j == NONE {
                    if let (Some(a1), Some(b1), Some(c1)) = (nu.get_mut(i), ng.get_mut(i), nk.get_mut(i)) {
                        *a1 = NONE;
                        *b1 = gi;
                        *c1 = ki;
                    }
                } else {
                    let ju = j as usize;
                    if let (Some(a1), Some(b1), Some(c1)) = (nu.get_mut(i), ng.get_mut(i), nk.get_mut(i)) {
                        *a1 = pu.get(ju).copied().unwrap_or(NONE);
                        *b1 = gi.max(pg.get(ju).copied().unwrap_or(i64::MIN));
                        *c1 = ki.max(pk.get(ju).copied().unwrap_or(i64::MIN));
                    }
                }
            }
            up.push(nu);
            mg.push(ng);
            mk.push(nk);
            lv += 1;
        }
        let end_ts = secs.last().map_or(0, |&t| t + US_S);
        Self { long, secs, f, base, g, kap, up, mg, mk, end_ts }
    }

    /// The side of this lookup.
    #[must_use]
    pub const fn long(&self) -> bool {
        self.long
    }

    /// The number of merged traded seconds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.secs.len()
    }

    /// No traded second.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.secs.is_empty()
    }

    /// The merged index of a traded second (None when the second is not a traded second).
    #[must_use]
    pub fn node_of(&self, ts: Us) -> Option<usize> {
        let i = self.secs.partition_point(|&s| s < ts);
        (self.secs.get(i) == Some(&ts)).then_some(i)
    }

    /// The first chain node from `c0` whose piece fires: a fixed distance `dist` (`pct` None) or a relative bp (`pct` Some(bp)). Counted
    /// in `steps` (one per lifting level read).
    #[must_use]
    pub fn first_firing(&self, c0: usize, dist: Paisa, pct: Option<u32>, steps: &mut u32) -> Option<usize> {
        let fires = |c: usize| match pct {
            None => self.g.get(c).is_some_and(|&g| g >= dist),
            Some(bp) => self.kap.get(c).is_some_and(|&k| k >= i64::from(bp)),
        };
        let mut c = c0;
        if c >= self.secs.len() {
            return None;
        }
        for k in (0..self.up.len()).rev() {
            *steps = steps.saturating_add(1);
            let nx = self.up.get(k).and_then(|r| r.get(c)).copied().unwrap_or(NONE);
            if nx == NONE {
                continue;
            }
            let quiet = match pct {
                // MUTANT m_sf3_lift_off_by_one: a block whose best equals the distance is jumped (a firing node skipped)
                None => self.mg.get(k).and_then(|r| r.get(c)).is_some_and(|&m| if cfg!(feature = "m_sf3_lift_off_by_one") { m <= dist } else { m < dist }),
                Some(bp) => self.mk.get(k).and_then(|r| r.get(c)).is_some_and(|&m| if cfg!(feature = "m_sf3_lift_off_by_one") { m <= i64::from(bp) } else { m < i64::from(bp) }),
            };
            if quiet {
                c = nx as usize;
            }
        }
        *steps = steps.saturating_add(1);
        fires(c).then_some(c)
    }

    /// The firing level and the time span (start, end) of node c's piece (c, nxt[c]] for a fixed `dist` or a relative `pct`.
    fn piece(&self, c: usize, dist: Paisa, pct: Option<u32>) -> (Paisa, Us, Us) {
        let fc = self.f.get(c).copied().unwrap_or(0);
        let d = match pct {
            None => dist,
            Some(bp) => pct_dist(self.base.get(c).copied().unwrap_or(0), bp),
        };
        let lvl_space = fc.saturating_add(d);
        let level = if self.long { lvl_space.saturating_neg() } else { lvl_space };
        let start = self.secs.get(c).map_or(0, |&t| t + US_S);
        let nx = self.up.first().and_then(|r| r.get(c)).copied().unwrap_or(NONE);
        let end = if nx == NONE { self.end_ts } else { self.secs.get(nx as usize).map_or(self.end_ts, |&t| t + US_S) };
        (level, start, end)
    }
}

/// The piecewise first reach of R(s) +/- x over [start, to) (the pieces of R; the first piece from the watch start `from`): crate v2.1's
/// `scan_one` `first_cross`, with a free start (ArmedStop, TimedStop).
#[allow(clippy::too_many_arguments, reason = "the scan's own state")]
fn cross<R: Row>(tape: &Tape<'_, R>, pcs: &[(Us, Paisa)], from: Us, start: Us, to: Us, long: bool, favourable: bool, x: Paisa, steps: &mut u32) -> Option<(Us, Paisa)> {
    for (k, &(p_at, r)) in pcs.iter().enumerate() {
        let a0 = if k == 0 { from } else { p_at.max(from) };
        let a = a0.max(start);
        let b = pcs.get(k + 1).map_or(to, |n| n.0.min(to));
        if a >= b {
            continue;
        }
        let (high, level) = if long == favourable { (true, r.saturating_add(x)) } else { (false, r.saturating_sub(x)) };
        // MUTANT m_sf3_reach_strict: a favourable level must be passed by one paisa (reaching is not enough)
        let level = if cfg!(feature = "m_sf3_reach_strict") && favourable { if high { level.saturating_add(1) } else { level.saturating_sub(1) } } else { level };
        if let Some(s) = tape.first_reach(a, b, high, level, steps) {
            return Some((s, level));
        }
    }
    None
}

/// Every extended rule of one level over [E0, `to`) in a bounded number of steps: fixed levels by the piecewise `first_reach` (crate
/// v2.1's `scan_one` reading), trails through `ti` (the contract-day's [`TrailIndex`] of the same side; a lookup of the other side is
/// refused: the trail rules then read None and `steps` is set to u32::MAX). Equal to [`scan_law`] on every rule and to
/// [`crate::path::scan_one`] on every crate v2.1 rule.
#[must_use]
pub fn scan_fast<R: Row>(tape: &Tape<'_, R>, ti: &TrailIndex, e: &EntryView<'_>, to: Us, long: bool, rules: &[XRule], steps: &mut u32) -> Vec<Option<Trigger>> {
    let from = e.e0;
    let pcs = pieces(e.fills, long);
    let first_px = e.fills.first().map_or(0, |b| b.px);
    let price = |hit: Option<(Us, Paisa)>| hit.map(|(s, level)| Trigger { sec: s, anchor: (s + US_S).max(e.e1), level });
    let mut out = Vec::with_capacity(rules.len());
    for rule in rules {
        let t = match *rule {
            XRule::Stop { x } | XRule::QuickCut { x } => price(cross(tape, &pcs, from, from, to, long, false, x, steps)),
            XRule::Target { x } => price(cross(tape, &pcs, from, from, to, long, true, x, steps)),
            XRule::StopAt { level } => price(tape.first_reach(from, to, !long, level, steps).map(|s| (s, level))),
            XRule::TargetAt { level } => {
                let lv = if cfg!(feature = "m_sf3_reach_strict") { if long { level.saturating_add(1) } else { level.saturating_sub(1) } } else { level };
                price(tape.first_reach(from, to, long, lv, steps).map(|s| (s, level)))
            }
            XRule::TimeExit { secs } => {
                let at = from + i64::from(secs) * US_S;
                (at < to).then_some(Trigger { sec: at, anchor: at.max(e.e1), level: 0 })
            }
            XRule::GoPositive { secs } => {
                let at = from + i64::from(secs) * US_S;
                *steps = steps.saturating_add(1);
                let last = tape.last_traded_before(at).filter(|s| s.ts >= from);
                let positive = last.is_some_and(|s| {
                    let r = r_at(&pcs, first_px, s.ts);
                    // MUTANT m_sf3_gopos_le: an extreme EQUAL to R counted as in the money
                    if cfg!(feature = "m_sf3_gopos_le") { if long { s.low >= r } else { s.high <= r } } else if long { s.low > r } else { s.high < r }
                });
                (at < to && !positive).then_some(Trigger { sec: at, anchor: at.max(e.e1), level: last.map_or(first_px, |s| r_at(&pcs, first_px, s.ts)) })
            }
            XRule::Trail { act, dist } => trail_fast(tape, ti, &pcs, from, to, long, act, dist, None, e.e1, steps),
            XRule::TrailPct { act, bp } => {
                if cfg!(feature = "m_sf3_trailpct_abs") {
                    // MUTANT m_sf3_trailpct_abs: the relative distance frozen at the wake price (a fixed trail)
                    let w = cross(tape, &pcs, from, from, to, long, true, act, steps);
                    let d = w.and_then(|(s, _)| tape.extreme(s, s + US_S, long)).map_or(1, |(p, _)| pct_dist(p, bp));
                    trail_fast(tape, ti, &pcs, from, to, long, act, d, None, e.e1, steps)
                } else {
                    trail_fast(tape, ti, &pcs, from, to, long, act, 0, Some(bp), e.e1, steps)
                }
            }
            XRule::ArmedStop { act, off } => {
                let wake = cross(tape, &pcs, from, from, to, long, true, act, steps);
                match wake {
                    None => None,
                    Some((w, _)) => {
                        // MUTANT m_sf3_be_same_second: the stop is watched from the wake second itself
                        let st = if cfg!(feature = "m_sf3_be_same_second") { w } else { w + US_S };
                        price(cross(tape, &pcs, from, st, to, long, false, off, steps))
                    }
                }
            }
            XRule::TimedStop { after, off } => {
                let at = from + i64::from(after) * US_S;
                price(cross(tape, &pcs, from, at, to, long, false, off, steps))
            }
        };
        out.push(t);
    }
    out
}

/// The trail through the chain: the wake (piecewise first reach of the favourable side), then the first firing node, then one
/// `first_reach` inside its piece.
#[allow(clippy::too_many_arguments, reason = "the scan's own state")]
fn trail_fast<R: Row>(tape: &Tape<'_, R>, ti: &TrailIndex, pcs: &[(Us, Paisa)], from: Us, to: Us, long: bool, act: Paisa, dist: Paisa, pct: Option<u32>, e1: Us, steps: &mut u32) -> Option<Trigger> {
    if ti.long() != long {
        *steps = u32::MAX;
        return None;
    }
    let (w, _) = cross(tape, pcs, from, from, to, long, true, act, steps)?;
    let c0 = ti.node_of(w)?;
    let c = ti.first_firing(c0, dist, pct, steps)?;
    let (level, a, b) = ti.piece(c, dist, pct);
    let s = tape.first_reach(a, b, !long, level, steps)?;
    (s < to).then_some(Trigger { sec: s, anchor: (s + US_S).max(e1), level })
}

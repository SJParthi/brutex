//! Path facts: sample-6's `runs70`, lifted (crate v1); change C5's decided target (`target_level`, `target_trigger`); REQ 71's rules
//! and first-touch facts (`path::scan`, change C7).

use crate::tape::{Row, Tape};
use crate::{LevelMask, MAXL, Paisa, US_S, Units, Us};

/// A decided exit's trigger (design 4.2 `path::Trigger`): the trigger second, the exit's anchor (max(its end, E1) for a price trigger) and
/// the level that fired.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Trigger {
    /// The trigger second's start.
    pub sec: Us,
    /// The exit order's anchor T (S4, S16).
    pub anchor: Us,
    /// The level.
    pub level: Paisa,
}

/// sample-6's target price (`o5s_dial::target_px`, lifted): a long's sale at the first tick at or above avg x (1 + bp / 10,000); a short's
/// buy-back at the first tick at or below avg x (1 - bp / 10,000); `entry_h` = the entry's value in HALF-PAISA over `units`. 0 without units.
#[must_use]
pub fn target_level(entry_h: i128, units: Units, long: bool, bp: u16, tick: Paisa) -> Paisa {
    if units == 0 {
        return 0;
    }
    let tk = i128::from(tick.max(1));
    let den = 2 * 10_000 * i128::from(units);
    let ceil = |a: i128, b: i128| { let q = a.div_euclid(b); if q * b == a { q } else { q + 1 } };
    let bp = i128::from(bp);
    let raw = if long { ceil(entry_h * (10_000 + bp), den) } else { (entry_h * (10_000 - bp)).div_euclid(den) };
    let px = if long { ceil(raw, tk) * tk } else { raw.div_euclid(tk) * tk };
    i64::try_from(px.max(tk)).unwrap_or(i64::MAX)
}

/// CHANGE C5 (D11, S16): a position's DECIDED target: the first traded second s stamped in [from, to) whose FAVOURABLE extreme REACHES
/// `level` (a long: HIGH >= level; a short: LOW <= level); the exit order's anchor is max(s + 1 s, `e1`) (the decision at the trigger
/// second's end, never before the entry is known), and the exit fills by blocks at their worst ([`crate::walk`], `Kind::Target`). Crate
/// v1 / sample-6 rested a limit AT the level, filled only in a block traded strictly beyond it (`Kind::TargetLimit`, now a labelled
/// comparison only). O(log n) through the tape's range table.
#[must_use]
pub fn target_trigger<R: Row>(tape: &Tape<'_, R>, long: bool, level: Paisa, from: Us, to: Us, e1: Us, steps: &mut u32) -> Option<Trigger> {
    let sec = tape.first_reach(from, to, long, level, steps)?;
    let end = sec.checked_add(US_S)?;
    Some(Trigger { sec, anchor: end.max(e1), level })
}

/// sample-6's `runs70`, lifted: the run-up and the run-down (paisa, and their seconds) of a position of `qty` units entered for
/// `entry_value` paisa, over the rows stamped in [from, to), through [`Tape::extreme`]: a long's highest HIGH / lowest LOW, a short's lowest
/// LOW / highest HIGH; each figure at least 0. Exact i128 intermediate products (a per-trade total); a figure above i64 reads i64::MAX.
#[must_use]
pub fn runs<R: Row>(tape: &Tape<'_, R>, from: Us, to: Us, long: bool, entry_value: Paisa, qty: Units) -> ((Paisa, Us), (Paisa, Us)) {
    // A data fact under every law (crate v1 stubbed it under its red feature; crate v2's red build carries the old FILL rule only).
    if qty == 0 {
        return ((0, 0), (0, 0));
    }
    let (Some((bv, bat)), Some((wv, wat))) = (tape.extreme(from, to, long), tape.extreme(from, to, !long)) else { return ((0, 0), (0, 0)) };
    let (ev, q) = (i128::from(entry_value), i128::from(qty));
    let up = if long { q * i128::from(bv) - ev } else { ev - q * i128::from(bv) };
    let down = if long { ev - q * i128::from(wv) } else { q * i128::from(wv) - ev };
    let p = |x: i128| i64::try_from(x.max(0)).unwrap_or(i64::MAX);
    ((p(up), bat), (p(down), wat))
}

// =====================================================================================================================================
// CHANGE C7 (S16, S19; D18, D30): REQ 71's rules and path facts, per level, watched from E0 (the START of the entry's first filling block),
// each price trigger's exit anchored at max(the trigger second's end, E1), the reference R(s) = the worse of the first fill price and the
// running average of the units filled in blocks STARTING at or before s. The sample's own time exit and give-back trail stay host
// decisions (D30); this is the crate's family (the operator's "activation and distance" trail).
// =====================================================================================================================================

/// One REQ 71 rule (design 4.2 `path::Rule`), distances in paisa from R(s).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Rule {
    /// A stop at distance `x` against the position.
    Stop {
        /// The distance (paisa).
        x: Paisa,
    },
    /// A quick cut: a stop with a tiny distance.
    QuickCut {
        /// The distance (paisa).
        x: Paisa,
    },
    /// A go-positive timer: at E0 + `secs`, exit unless the last second before it was already strictly in the money (its adverse extreme
    /// beyond R in our favour).
    GoPositive {
        /// Seconds after E0.
        secs: u32,
    },
    /// A time exit at E0 + `secs`.
    TimeExit {
        /// Seconds after E0.
        secs: u32,
    },
    /// A trailing stop: active once a second's favourable extreme reaches R + `act`; then `dist` behind the best extreme since; a second's
    /// adverse extreme at or beyond the level in force BEFORE that second triggers (adverse-first).
    Trail {
        /// The activation distance (paisa).
        act: Paisa,
        /// The trailing distance (paisa).
        dist: Paisa,
    },
    /// A decided target at distance `x` in the position's favour (C5's reading, measured from R(s)).
    Target {
        /// The distance (paisa).
        x: Paisa,
    },
}

/// One level's entry for [`scan`]: E0, E1 and its filling blocks (R(s) of S16 comes from these).
#[derive(Clone, Copy, Debug)]
pub struct EntryView<'f> {
    /// The first filling block's start.
    pub e0: Us,
    /// Its known end.
    pub e1: Us,
    /// The filling blocks, in time order.
    pub fills: &'f [crate::std70::BlockFill],
}

/// Which side the price touched first (S19), for one dial value x.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FirstSide {
    /// first-fill + x first.
    Up,
    /// first-fill - x first.
    Down,
    /// Both in the same second: read adverse-first (counted apart).
    BothSameSecondAdverse,
    /// Neither before the window's end.
    #[default]
    Neither,
}

/// The path facts of one level (S19): per dial value x the first seconds touching first-fill + x and first-fill - x and which came first;
/// the dial values whose two touches fell in one second (`both_secs`); the best and worst moments; the run-up and run-down on the filled
/// units at the average entry (paisa).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Facts {
    /// Per x (the `xs` order): the first second touching first-fill + x.
    pub up_at: Vec<Option<Us>>,
    /// Per x: the first second touching first-fill - x.
    pub down_at: Vec<Option<Us>>,
    /// Per x: which side came first.
    pub first: Vec<FirstSide>,
    /// The dial values whose first up and down touches fell in the same second (read adverse-first).
    pub both_secs: u32,
    /// The best moment (price, its second): a long's highest HIGH, a short's lowest LOW.
    pub best: (Paisa, Us),
    /// The worst moment: a long's lowest LOW, a short's highest HIGH.
    pub worst: (Paisa, Us),
    /// The run-up (paisa, at least 0).
    pub run_up: Paisa,
    /// The run-down (paisa, at least 0).
    pub run_down: Paisa,
}

/// R(s)'s pieces: (from instant, R) in time order; a piece starts at each filling block's START (the worst-case reading: our fill came
/// first in the block).
fn reference_pieces(fills: &[crate::std70::BlockFill], long: bool) -> Vec<(Us, Paisa)> {
    let mut out: Vec<(Us, Paisa)> = Vec::with_capacity(fills.len());
    let first = fills.first().map_or(0, |b| b.px);
    let (mut u, mut v) = (0_i128, 0_i128);
    for b in fills {
        u += i128::from(b.units);
        v += i128::from(b.units) * i128::from(b.px);
        // The running average, rounded against us: a long's up, a short's down.
        let avg = if u == 0 { first } else if long { i64::try_from((v + u - 1).div_euclid(u)).unwrap_or(i64::MAX) } else { i64::try_from(v.div_euclid(u)).unwrap_or(0) };
        let r = if long { first.max(avg) } else { first.min(avg) };
        match out.last_mut() {
            Some(last) if last.0 == b.block.at => last.1 = r,
            _ => out.push((b.block.at, r)),
        }
    }
    out
}

/// The watching start of the rules and the facts (CHANGE C7, D18: E0, the START of the entry's first filling block: an adverse second
/// inside the entry block, after the price paid, is a second held; mutant m7: E1, design v1's kinder window).
const fn watch_from(e: &EntryView<'_>) -> Us {
    if cfg!(feature = "m7") { e.e1 } else { e.e0 }
}

/// REQ 71's trail (C7): one pass from the watching start, second by second, adverse-first (the touch tested against the level in force
/// BEFORE the second): the first (second, level) where the position gives back `dist` from its best after moving `act` its way from R(s).
fn trail_hit<R: Row>(tape: &Tape<'_, R>, (from, to): (Us, Us), long: bool, (act, dist): (Paisa, Paisa), pieces: &[(Us, Paisa)], first_px: Paisa, steps: &mut u32) -> Option<(Us, Paisa)> {
    let mut best: Option<Paisa> = None;
    let mut hit = None;
    let i0 = tape.first_at(from);
    let w = tape.rows().get(i0..).unwrap_or(&[]);
    let mut j = 0_usize;
    // Second by second (a repeated stamp merged into its second: max HIGH, min LOW), so a second that activates or raises the
    // trail AND touches it is read adverse-first (C7 repair 1, found by the node reference at G0: rows of one stamp were read
    // one by one).
    while let Some(s0) = w.get(j) {
        let ts = s0.ts();
        if ts >= to {
            break;
        }
        let (mut hi, mut lo, mut any) = (Paisa::MIN, Paisa::MAX, false);
        while let Some(s) = w.get(j).filter(|s| s.ts() == ts) {
            if s.vol() > 0 {
                hi = hi.max(s.high());
                lo = lo.min(s.low());
                any = true;
            }
            j += 1;
        }
        if !any {
            continue;
        }
        *steps = steps.saturating_add(1);
        let r = pieces.iter().rev().find(|p| p.0 <= ts).map_or(first_px, |p| p.1);
        if let Some(bst) = best {
            // Saturating, never wrapping (OB7, G0 repair 2): a level beyond i64 is never reached by a print.
        let level = if long { bst.saturating_sub(dist) } else { bst.saturating_add(dist) };
            if (long && lo <= level) || (!long && hi >= level) {
                hit = Some((ts, level));
                break;
            }
        }
        let fav = if long { hi } else { lo };
        let active = best.is_some() || (long && fav >= r.saturating_add(act)) || (!long && fav <= r.saturating_sub(act));
        if active {
            best = Some(best.map_or(fav, |b| if long { b.max(fav) } else { b.min(fav) }));
        }
    }
    hit
}

/// One level's REQ 71 triggers (per rule, in `rules` order) and its path facts over [watch start, `to`), for a long (`long`) or short
/// position. O(rules x pieces x log n) through the tape's range table (the trail: one pass from its activation), O(1) timers.
#[must_use]
pub fn scan_one<R: Row>(tape: &Tape<'_, R>, e: &EntryView<'_>, to: Us, long: bool, rules: &[Rule], xs: &[Paisa], steps: &mut u32) -> (Vec<Option<Trigger>>, Facts) {
    let from = watch_from(e);
    let pieces = reference_pieces(e.fills, long);
    let first_px = e.fills.first().map_or(0, |b| b.px);
    // The first second s in [from, to) whose adverse (or favourable) extreme reaches R(s) -/+ x: piece by piece (R constant on a piece).
    let first_cross = |x: Paisa, favourable: bool, steps: &mut u32| -> Option<(Us, Paisa)> {
        for (k, &(p_at, r)) in pieces.iter().enumerate() {
            let a = if k == 0 { from } else { p_at.max(from) };
            let b = pieces.get(k + 1).map_or(to, |n| n.0.min(to));
            if a >= b {
                continue;
            }
            // A long's adverse side and a short's favourable side are below R (read on LOWs); the other two above it (on HIGHs).
            // Saturating, never wrapping (OB7, G0 repair 2): a level beyond i64 is never reached by a print.
            let (high, level) = if long == favourable { (true, r.saturating_add(x)) } else { (false, r.saturating_sub(x)) };
            if let Some(s) = tape.first_reach(a, b, high, level, steps) {
                return Some((s, level));
            }
        }
        None
    };
    let price_trigger = |hit: Option<(Us, Paisa)>| hit.map(|(s, level)| Trigger { sec: s, anchor: (s + US_S).max(e.e1), level });
    let mut trig = Vec::with_capacity(rules.len());
    for rule in rules {
        let t = match *rule {
            Rule::Stop { x } | Rule::QuickCut { x } => price_trigger(first_cross(x, false, steps)),
            Rule::Target { x } => price_trigger(first_cross(x, true, steps)),
            Rule::TimeExit { secs } => {
                let at = from + i64::from(secs) * US_S;
                (at < to).then_some(Trigger { sec: at, anchor: at.max(e.e1), level: 0 })
            }
            Rule::GoPositive { secs } => {
                let at = from + i64::from(secs) * US_S;
                *steps = steps.saturating_add(1);
                let last = tape.last_traded_before(at).filter(|s| s.ts >= from);
                let r_at = |t: Us| pieces.iter().rev().find(|p| p.0 <= t).map_or(first_px, |p| p.1);
                let positive = last.is_some_and(|s| if long { s.low > r_at(s.ts) } else { s.high < r_at(s.ts) });
                (at < to && !positive).then_some(Trigger { sec: at, anchor: at.max(e.e1), level: last.map_or(first_px, |s| r_at(s.ts)) })
            }
            Rule::Trail { act, dist } => price_trigger(trail_hit(tape, (from, to), long, (act, dist), &pieces, first_px, steps)),
        };
        trig.push(t);
    }
    // S19's facts, from the first fill price.
    let mut f = Facts::default();
    for &x in xs {
        let (up, down) = (tape.first_reach(from, to, true, first_px.saturating_add(x), steps), tape.first_reach(from, to, false, first_px.saturating_sub(x), steps));
        let side = match (up, down) {
            (None, None) => FirstSide::Neither,
            (Some(_), None) => FirstSide::Up,
            (None, Some(_)) => FirstSide::Down,
            (Some(u), Some(d)) if u == d => FirstSide::BothSameSecondAdverse,
            (Some(u), Some(d)) => if u < d { FirstSide::Up } else { FirstSide::Down },
        };
        if side == FirstSide::BothSameSecondAdverse {
            f.both_secs += 1;
        }
        f.up_at.push(up);
        f.down_at.push(down);
        f.first.push(side);
    }
    let (units, value): (i128, i128) = e.fills.iter().fold((0, 0), |(u, v), b| (u + i128::from(b.units), v + i128::from(b.units) * i128::from(b.px)));
    if let (Some(hi), Some(lo)) = (tape.extreme(from, to, true), tape.extreme(from, to, false)) {
        let (best, worst) = if long { (hi, lo) } else { (lo, hi) };
        f.best = best;
        f.worst = worst;
        let p = |x: i128| i64::try_from(x.max(0)).unwrap_or(i64::MAX);
        f.run_up = p(if long { units * i128::from(best.0) - value } else { value - units * i128::from(best.0) });
        f.run_down = p(if long { value - units * i128::from(worst.0) } else { units * i128::from(worst.0) - value });
    }
    (trig, f)
}

/// S16 + S19 for every masked level: ONE pass per DISTINCT (E0, E1, entry fills) across the levels (at most MAXL; the count returned:
/// design 4.6 step 2, B-TRADE). `entries[l]` None for a level that filled nothing; `to[l]` the level's window end; `trig[l]` / `facts[l]`
/// receive the level's triggers and facts.
#[allow(clippy::too_many_arguments, reason = "design 4.2's signature")]
pub fn scan<R: Row>(tape: &Tape<'_, R>, entries: &[Option<EntryView<'_>>; MAXL], mask: LevelMask, to: &[Us; MAXL], long: bool, rules: &[Rule], xs: &[Paisa], trig: &mut [Vec<Option<Trigger>>; MAXL], facts: &mut [Facts; MAXL], steps: &mut u32) -> usize {
    let mut groups = 0_usize;
    for l in 0..MAXL {
        if !mask.has(l) {
            continue;
        }
        let (Some(Some(e)), Some(&t_end)) = (entries.get(l), to.get(l)) else { continue };
        // A level whose (E0, E1, fills, window end) equals an earlier level's shares its pass.
        let same = (0..l).find(|&m| mask.has(m) && entries.get(m).and_then(|x| x.as_ref()).is_some_and(|x| x.e0 == e.e0 && x.e1 == e.e1 && x.fills == e.fills) && to.get(m) == Some(&t_end));
        let (tg, fc) = if let Some(m) = same {
            (trig.get(m).cloned().unwrap_or_default(), facts.get(m).cloned().unwrap_or_default())
        } else {
            groups += 1;
            scan_one(tape, e, t_end, long, rules, xs, steps)
        };
        if let (Some(a), Some(b)) = (trig.get_mut(l), facts.get_mut(l)) {
            *a = tg;
            *b = fc;
        }
    }
    groups
}

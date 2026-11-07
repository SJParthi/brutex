//! E1, the flag (box) continuation (PREVIEW-SPEC 7.1), on the close (E1C)
//! and on the first second beyond (E1S).
//!
//! Written for a long; a short is the long's rule on the price mirror
//! (`Candle::mirror`), so every price test mirrors exactly.
//!
//! * **Push and box**: the push extreme is the candle of the last strictly
//!   new session high. A box of K consecutive complete candles starts no
//!   earlier than the candle after it (headline: the K candles right before
//!   the break candidate; `BoxAnchor::Anchored` is the other reading). A new
//!   high ends the box and restarts the push. An incomplete candle voids the
//!   scope until the next new high.
//! * **Holding (S33)** on rung L: all lows above L (seed), all closes above
//!   L, no close below the first box candle's low, or none; running mode
//!   judges each box candle against the ladder before it, frozen mode
//!   against the ladder at the box's first candle.
//! * **Break (E1C)**: the first candle after the box whose close is above the
//!   box high; T = its close. **E1S**: armed at each close where a box
//!   exists, the first GDFL second from that close + 14 s to the next close
//!   whose printed high exceeds the R1-mapped box high; T = that second + 1.
//! * **Last entry (S42)**: a decision after 15:07:59, the dial's outer bound,
//!   refuses by name (`after_last_entry`), as in E2 and E3.
//! * **Stop** the box low minus the offset; **targets** T1 the session high
//!   known at T plus the offset, T2 1.272 and T3 1.618 of the target ladder.

use crate::bars::{Candle, Minute, Sec, first_at_or_after};
use crate::clock::{CANDLE_END, LAMBDA, LAST_ENTRY};
use crate::ladder::{Ladder, R1272, R1618, running};
use crate::model::{
    AfterBreak, BoxAnchor, Readings, Side, Status, TargetLadder, Toward, map_level, r1_at,
};

/// The holding rule S33.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Holding {
    /// Every box candle's low stays above L (seed).
    AllLows,
    /// Every box candle's close stays above L.
    AllCloses,
    /// No box close below the first box candle's low.
    NoCloseBelowFirstLow,
    /// No holding rule.
    Off,
}

impl Holding {
    /// Short name for the golden files.
    pub fn name(self) -> &'static str {
        match self {
            Holding::AllLows => "all_lows",
            Holding::AllCloses => "all_closes",
            Holding::NoCloseBelowFirstLow => "no_close_below_first_low",
            Holding::Off => "off",
        }
    }
}

/// The ladder mode S21.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LadderMode {
    /// A1, the running ladder (seed).
    Running,
    /// A2, frozen at the box's first candle.
    Frozen,
}

/// One E1 recipe's dials.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct E1Params {
    /// Side.
    pub side: Side,
    /// Box candles K (S31).
    pub k: u32,
    /// Holding rung L, thousandths (seed 382).
    pub rung: u32,
    /// Holding rule (seed all lows).
    pub holding: Holding,
    /// Ladder mode (seed running).
    pub mode: LadderMode,
    /// Tight: box height at most this many ppm of R; `None` = any.
    pub max_height_ppm: Option<u64>,
    /// The break candle (E1S: the arming candle) must be the side's colour.
    pub colour: bool,
    /// Stop offset beyond the box, paise (seed one measured step).
    pub stop_offset: i64,
    /// T1 offset beyond the session extreme, paise (seed one measured step).
    pub t1_offset: i64,
}

/// Zerodha-space references fixed at a decision, in the real price space.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Levels {
    /// The pattern extreme the stop is placed from, paise.
    pub pattern_extreme: i64,
    /// The stop reference less the offset (a short: plus), milli-paise.
    pub stop_milli: i128,
    /// T1, the session extreme known at T plus the offset, milli-paise.
    pub t1_milli: i128,
    /// T2, the 1.272 rung, or `None` (no usable target ladder).
    pub t2_milli: Option<i128>,
    /// T3, the 1.618 rung, or `None`.
    pub t3_milli: Option<i128>,
}

impl Levels {
    /// The mirror of mirrored-space levels back to real prices.
    fn unmirror(self) -> Levels {
        Levels {
            pattern_extreme: -self.pattern_extreme,
            stop_milli: -self.stop_milli,
            t1_milli: -self.t1_milli,
            t2_milli: self.t2_milli.map(|x| -x),
            t3_milli: self.t3_milli.map(|x| -x),
        }
    }
}

/// One E1C break (or void) of one K.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct E1Event {
    /// The candle: the break candle, or the incomplete candle of a void.
    pub candle: u32,
    /// Its close, the decision second.
    pub t: u32,
    /// The push extreme's candle.
    pub ext: u32,
    /// The box's first candle (equals `candle` on a void).
    pub box_first: u32,
    /// Box high, paise (a short: the box low).
    pub box_high: i64,
    /// Box low, paise (a short: the box high).
    pub box_low: i64,
    /// `Ok`, a clause or a refusal by name.
    pub status: Status,
    /// Fixed references (present on a break, absent on a void).
    pub levels: Option<Levels>,
}

/// One E1S arming and what came of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct E1sEvent {
    /// The arming candle.
    pub candle: u32,
    /// Its close.
    pub armed_at: u32,
    /// The R1-mapped box high the scan watches (a short: the box low), paise.
    pub trigger: Option<i64>,
    /// The trigger second, if one printed before the next close.
    pub trigger_s: Option<u32>,
    /// The decision, trigger second + 1.
    pub t: Option<u32>,
    /// REF: the trigger second's last print, paise.
    pub ref_px: Option<i64>,
    /// `Ok`, a clause or a refusal; an arming whose scan found nothing is
    /// not reported.
    pub status: Status,
    /// Fixed references at the decision.
    pub levels: Option<Levels>,
}

/// The day mirrored to a long when the side is short.
pub(crate) fn oriented(candles: &[Candle], side: Side) -> Vec<Candle> {
    if side.long() {
        candles.to_vec()
    } else {
        candles.iter().map(Candle::mirror).collect()
    }
}

/// Box bounds over candles `first..=last` (long space).
fn bounds(candles: &[Candle], first: u32, last: u32) -> (i64, i64) {
    candles[first as usize..=last as usize]
        .iter()
        .fold((i64::MIN, i64::MAX), |(h, l), c| (h.max(c.h), l.min(c.l)))
}

/// The holding rule over a box, long space.
fn holds(candles: &[Candle], ladders: &[Ladder], first: u32, last: u32, p: &E1Params) -> bool {
    let first_low = candles[first as usize].l;
    let frozen = ladders[first as usize];
    (first..=last).all(|j| {
        let c = &candles[j as usize];
        let ladder = match p.mode {
            LadderMode::Running => ladders[j as usize],
            LadderMode::Frozen => frozen,
        };
        let level = if ladder.usable() {
            ladder.rung_milli(p.rung)
        } else {
            None
        };
        match p.holding {
            Holding::Off => true,
            Holding::NoCloseBelowFirstLow => c.c >= first_low,
            Holding::AllLows => level.is_some_and(|l| i128::from(c.l) * 1000 > l),
            Holding::AllCloses => level.is_some_and(|l| i128::from(c.c) * 1000 > l),
        }
    })
}

/// The height clause, long space: `(high - low) x 1e6 <= max x R`.
fn tight(box_high: i64, box_low: i64, ladder: &Ladder, max: Option<u64>) -> bool {
    max.is_none_or(|m| {
        i128::from(box_high - box_low) * 1_000_000 <= i128::from(m) * i128::from(ladder.range())
    })
}

/// The references at a decision whose candles closed through `closed`
/// (long space): `after` is the ladder after that candle, `judging` the one
/// it was judged against.
fn levels_at(box_low: i64, after: &Ladder, judging: &Ladder, p: &E1Params, r: &Readings) -> Levels {
    let target = match r.target_ladder {
        TargetLadder::AfterFold => after,
        TargetLadder::Judging => judging,
    };
    let rung = |x: u32| {
        if target.usable() {
            target.rung_milli(x)
        } else {
            None
        }
    };
    Levels {
        pattern_extreme: box_low,
        stop_milli: i128::from(box_low - p.stop_offset) * 1000,
        t1_milli: i128::from(after.hi + p.t1_offset) * 1000,
        t2_milli: rung(R1272),
        t3_milli: rung(R1618),
    }
}

/// The box whose break candidate is candle `i`, long space, or `None` when
/// no box can precede it.
fn box_for(i: u32, floor: u32, k: u32, anchor: BoxAnchor) -> Option<(u32, u32)> {
    let k = k.max(1);
    match anchor {
        BoxAnchor::Sliding => (i >= floor + k).then(|| (i - k, i - 1)),
        BoxAnchor::Anchored => (i >= floor + k).then(|| (floor, floor + k - 1)),
    }
}

/// E1C over one day for one recipe. `top[i]` is the rank clause at candle
/// `i`'s close for the recipe's side.
pub fn e1c(candles: &[Candle], top: &[bool], p: &E1Params, r: &Readings) -> Vec<E1Event> {
    let long = oriented(candles, p.side);
    let ladders = running(&long, r.ties, r.taint);
    let mut out = Vec::new();
    let mut ext: Option<u32> = None;
    let mut armed = false;
    let mut floor = 0u32;
    for (ix, c) in long.iter().enumerate() {
        let i = c.i;
        if let (true, Some(e)) = (armed, ext)
            && let Some((first, last)) = box_for(i, floor, p.k, r.box_anchor)
        {
            let (box_high, box_low) = bounds(&long, first, last);
            if c.c > box_high {
                let judging = ladders[ix];
                let status = if c.incomplete {
                    Status::IncompleteCandle
                } else if judging.tainted {
                    Status::TaintedLadder
                } else if !judging.usable() {
                    Status::LadderUndetermined
                } else if !holds(&long, &ladders, first, last, p) {
                    Status::Holding
                } else if !tight(box_high, box_low, &judging, p.max_height_ppm) {
                    Status::Height
                } else if p.colour && !c.green() {
                    Status::Colour
                } else if !top.get(ix).copied().unwrap_or(false) {
                    Status::NotInTopN
                } else if c.close_t > LAST_ENTRY {
                    Status::AfterLastEntry
                } else {
                    Status::Ok
                };
                let levels = levels_at(box_low, &ladders[ix + 1], &judging, p, r);
                out.push(E1Event {
                    candle: i,
                    t: c.close_t,
                    ext: e,
                    box_first: first,
                    box_high,
                    box_low,
                    status,
                    levels: Some(levels),
                });
                match r.after_break {
                    AfterBreak::UntilNewExtreme => armed = false,
                    AfterBreak::NextBox => floor = i + 1,
                }
            }
        }
        // fold candle i: a strictly new high restarts the push
        if ix == 0 || c.h > ladders[ix].hi {
            ext = Some(i);
            armed = true;
            floor = i + 1;
        }
        if c.incomplete && armed {
            out.push(E1Event {
                candle: i,
                t: c.close_t,
                ext: ext.unwrap_or(i),
                box_first: i,
                box_high: c.h,
                box_low: c.l,
                status: Status::IncompleteCandle,
                levels: None,
            });
            armed = false;
        }
    }
    if !p.side.long() {
        for e in &mut out {
            let (h, l) = (e.box_high, e.box_low);
            e.box_high = -h;
            e.box_low = -l;
            e.levels = e.levels.map(Levels::unmirror);
        }
    }
    out
}

/// Inputs E1S reads beyond the candles: the day's minutes (R1), seconds and
/// the measured step and data start.
#[derive(Clone, Copy, Debug)]
pub struct SecondFeed<'a> {
    /// The day's 1-minute candles, real prices.
    pub minutes: &'a [Minute],
    /// The day's GDFL seconds, real prices, sorted.
    pub secs: &'a [Sec],
    /// Measured raw price step, paise.
    pub step: i64,
    /// The stock-day's GDFL data start, second of the day.
    pub data_start: u32,
}

/// The first second in `[from, to)` whose (long-space) high exceeds
/// `trigger`; `secs` are long-space.
fn scan(secs: &[Sec], from: u32, to: u32, trigger: i64) -> Option<Sec> {
    secs[first_at_or_after(secs, from)..]
        .iter()
        .take_while(|s| s.t < to)
        .find(|s| s.traded() && s.h > trigger)
        .copied()
}

/// E1S over one day for one recipe (see the module doc).
#[allow(
    clippy::too_many_lines,
    reason = "the module doc's E1S steps, in order"
)]
pub fn e1s(
    candles: &[Candle],
    top: &[bool],
    feed: &SecondFeed<'_>,
    p: &E1Params,
    r: &Readings,
) -> Vec<E1sEvent> {
    let long_side = p.side.long();
    let long = oriented(candles, p.side);
    let secs: Vec<Sec> = if long_side {
        feed.secs.to_vec()
    } else {
        feed.secs.iter().map(Sec::mirror).collect()
    };
    let ladders = running(&long, r.ties, r.taint);
    let mut out = Vec::new();
    let mut armed = false;
    let mut floor = 0u32;
    for (ix, c) in long.iter().enumerate() {
        let i = c.i;
        // fold candle i first: the arming reads the box ending at i
        if ix == 0 || c.h > ladders[ix].hi {
            armed = true;
            floor = i + 1;
        }
        if c.incomplete {
            armed = false;
        }
        if !armed {
            continue;
        }
        let k = p.k.max(1);
        let first = match r.box_anchor {
            BoxAnchor::Sliding => match (i + 1).checked_sub(k) {
                Some(f) if f >= floor => f,
                _ => continue,
            },
            BoxAnchor::Anchored => {
                if i < floor + k - 1 {
                    continue;
                }
                floor
            }
        };
        let last = match r.box_anchor {
            BoxAnchor::Sliding => i,
            BoxAnchor::Anchored => floor + k - 1,
        };
        let (box_high, box_low) = bounds(&long, first, last);
        let after = ladders[ix + 1];
        if !holds(&long, &ladders, first, last, p)
            || !tight(box_high, box_low, &after, p.max_height_ppm)
        {
            continue;
        }
        let armed_at = c.close_t;
        let next_close = long.get(ix + 1).map_or(CANDLE_END, |n| n.close_t);
        let mut event = E1sEvent {
            candle: i,
            armed_at,
            trigger: None,
            trigger_s: None,
            t: None,
            ref_px: None,
            status: Status::Ok,
            levels: None,
        };
        if armed_at + LAMBDA < feed.data_start {
            event.status = Status::NoFillDataWindow;
            out.push(event);
            continue;
        }
        let Some((z, g)) = r1_at(feed.minutes, armed_at) else {
            event.status = Status::NoTriggerRatio;
            out.push(event);
            continue;
        };
        // the trigger in real space, rounded so it is harder to reach
        let real_level = if long_side { box_high } else { -box_high };
        let toward = if long_side { Toward::Up } else { Toward::Down };
        let real_trigger = map_level(i128::from(real_level) * 1000, z, g, feed.step, toward);
        let trigger = if long_side {
            real_trigger
        } else {
            -real_trigger
        };
        event.trigger = Some(real_trigger);
        let Some(hit) = scan(&secs, armed_at + LAMBDA, next_close, trigger) else {
            continue;
        };
        event.trigger_s = Some(hit.t);
        event.t = Some(hit.t + 1);
        event.ref_px = Some(if long_side { hit.c } else { -hit.c });
        event.status = if after.tainted {
            Status::TaintedLadder
        } else if !after.usable() {
            Status::LadderUndetermined
        } else if p.colour && !c.green() {
            Status::Colour
        } else if !top.get(ix).copied().unwrap_or(false) {
            Status::NotInTopN
        } else if hit.t + 1 > LAST_ENTRY {
            Status::AfterLastEntry
        } else {
            Status::Ok
        };
        let levels = levels_at(box_low, &after, &ladders[ix], p, r);
        event.levels = Some(if long_side { levels } else { levels.unmirror() });
        out.push(event);
        match r.after_break {
            AfterBreak::UntilNewExtreme => armed = false,
            AfterBreak::NextBox => floor = i + 1,
        }
    }
    out
}

#[cfg(test)]
#[path = "e1_tests.rs"]
pub(crate) mod tests;

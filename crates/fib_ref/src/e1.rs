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
//! * **Stop** the box low minus the offset; **targets** T1 the session high
//!   known at T plus the offset, T2 1.272 and T3 1.618 of the target ladder.

use crate::bars::{Candle, Minute, Sec, first_at_or_after};
use crate::clock::{CANDLE_END, LAMBDA};
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
pub(crate) mod tests {
    use super::*;
    use crate::bars::fold;
    use crate::clock::{OPEN, hms};

    pub(crate) fn seed(side: Side, k: u32) -> E1Params {
        E1Params {
            side,
            k,
            rung: 382,
            holding: Holding::AllLows,
            mode: LadderMode::Running,
            max_height_ppm: None,
            colour: true,
            stop_offset: 5,
            t1_offset: 5,
        }
    }

    pub(crate) fn minute(m: u32, o: i64, h: i64, l: i64, c: i64) -> Minute {
        Minute {
            m,
            o,
            h,
            l,
            c,
            v: 100,
            held: true,
            padded: false,
            g: Some(c),
        }
    }

    /// The operator's flag on a synthetic session: a push to a new high at
    /// minute 3, a two-candle box holding above 0.382, a green break at 6.
    pub(crate) fn flag_day() -> Vec<Minute> {
        vec![
            minute(0, 10_000, 10_020, 9_950, 10_010),
            minute(1, 10_010, 10_080, 10_000, 10_070),
            minute(2, 10_070, 10_150, 10_060, 10_140),
            minute(3, 10_140, 10_200, 10_130, 10_190), // push extreme 10,200
            minute(4, 10_190, 10_195, 10_160, 10_170), // box
            minute(5, 10_170, 10_185, 10_165, 10_180), // box
            minute(6, 10_180, 10_210, 10_175, 10_205), // green break above 10,195
            minute(7, 10_205, 10_230, 10_200, 10_225),
        ]
    }

    fn mirror_day(day: &[Minute]) -> Vec<Minute> {
        // a real short day: prices reflected about 20,000 paise
        day.iter()
            .map(|m| Minute {
                o: 20_000 - m.o,
                h: 20_000 - m.l,
                l: 20_000 - m.h,
                c: 20_000 - m.c,
                g: m.g.map(|g| 20_000 - g),
                ..*m
            })
            .collect()
    }

    #[test]
    fn the_operators_flag_gives_exactly_one_signal_at_the_break_close() {
        let candles = fold(&flag_day(), 1);
        let top = vec![true; candles.len()];
        let events = e1c(&candles, &top, &seed(Side::Long, 2), &Readings::HEADLINE);
        let ok: Vec<&E1Event> = events.iter().filter(|e| e.status == Status::Ok).collect();
        assert_eq!(ok.len(), 1, "{events:?}");
        let e = ok[0];
        assert_eq!(e.candle, 6);
        assert_eq!(hms(e.t), "09:22:00");
        assert_eq!(
            (e.ext, e.box_first, e.box_high, e.box_low),
            (3, 4, 10_195, 10_160)
        );
        let lv = e.levels.unwrap_or(Levels {
            pattern_extreme: 0,
            stop_milli: 0,
            t1_milli: 0,
            t2_milli: None,
            t3_milli: None,
        });
        assert_eq!(lv.stop_milli, 10_155_000);
        assert_eq!(lv.t1_milli, 10_215_000); // new high 10,210 + 5
        // after fold: lo 9,950 hi 10,210, R 260: 1.272 -> 9,950 + 330.72
        assert_eq!(lv.t2_milli, Some(9_950_000 + 1272 * 260));
        assert_eq!(lv.t3_milli, Some(9_950_000 + 1618 * 260));
    }

    #[test]
    fn its_short_mirror_gives_the_mirrored_signal() {
        let candles = fold(&mirror_day(&flag_day()), 1);
        let top = vec![true; candles.len()];
        let events = e1c(&candles, &top, &seed(Side::Short, 2), &Readings::HEADLINE);
        let ok: Vec<&E1Event> = events.iter().filter(|e| e.status == Status::Ok).collect();
        assert_eq!(ok.len(), 1, "{events:?}");
        let e = ok[0];
        assert_eq!(e.candle, 6);
        assert_eq!((e.box_high, e.box_low), (20_000 - 10_195, 20_000 - 10_160));
        let lv = e.levels.map(|l| (l.stop_milli, l.t1_milli));
        assert_eq!(lv, Some((9_845_000, 9_785_000)));
    }

    #[test]
    fn clauses_refuse_by_name_in_their_order() {
        let day = flag_day();
        let p = seed(Side::Long, 2);
        let r = Readings::HEADLINE;
        let top_off = vec![false; 8];
        let candles = fold(&day, 1);
        let s = e1c(&candles, &top_off, &p, &r);
        assert_eq!(
            s.iter().find(|e| e.candle == 6).map(|e| e.status),
            Some(Status::NotInTopN)
        );
        // a red break candle
        let mut red = day.clone();
        red[6] = minute(6, 10_209, 10_212, 10_175, 10_200);
        let top = vec![true; 8];
        let s = e1c(&fold(&red, 1), &top, &p, &r);
        assert_eq!(
            s.iter().find(|e| e.candle == 6).map(|e| e.status),
            Some(Status::Colour)
        );
        // colour off accepts it
        let s = e1c(&fold(&red, 1), &top, &E1Params { colour: false, ..p }, &r);
        assert_eq!(
            s.iter().find(|e| e.candle == 6).map(|e| e.status),
            Some(Status::Ok)
        );
        // a tight clause the box fails: height 35 of R 250 is 140,000 ppm
        let s = e1c(
            &candles,
            &top,
            &E1Params {
                max_height_ppm: Some(100_000),
                ..p
            },
            &r,
        );
        assert_eq!(
            s.iter().find(|e| e.candle == 6).map(|e| e.status),
            Some(Status::Height)
        );
        let s = e1c(
            &candles,
            &top,
            &E1Params {
                max_height_ppm: Some(140_000),
                ..p
            },
            &r,
        );
        assert_eq!(
            s.iter().find(|e| e.candle == 6).map(|e| e.status),
            Some(Status::Ok)
        );
        // a deep box below 0.382 fails the holding rule
        let mut deep = day.clone();
        deep[4] = minute(4, 10_190, 10_195, 10_050, 10_170);
        let s = e1c(&fold(&deep, 1), &top, &p, &r);
        assert_eq!(
            s.iter().find(|e| e.candle == 6).map(|e| e.status),
            Some(Status::Holding)
        );
        let s = e1c(
            &fold(&deep, 1),
            &top,
            &E1Params {
                holding: Holding::Off,
                ..p
            },
            &r,
        );
        assert_eq!(
            s.iter().find(|e| e.candle == 6).map(|e| e.status),
            Some(Status::Ok)
        );
        let s = e1c(
            &fold(&deep, 1),
            &top,
            &E1Params {
                holding: Holding::AllCloses,
                ..p
            },
            &r,
        );
        assert_eq!(
            s.iter().find(|e| e.candle == 6).map(|e| e.status),
            Some(Status::Ok)
        );
        let s = e1c(
            &fold(&deep, 1),
            &top,
            &E1Params {
                holding: Holding::NoCloseBelowFirstLow,
                ..p
            },
            &r,
        );
        assert_eq!(
            s.iter().find(|e| e.candle == 6).map(|e| e.status),
            Some(Status::Ok)
        );
        // frozen mode reads the ladder at the box's first candle
        let s = e1c(
            &candles,
            &top,
            &E1Params {
                mode: LadderMode::Frozen,
                ..p
            },
            &r,
        );
        assert_eq!(
            s.iter().find(|e| e.candle == 6).map(|e| e.status),
            Some(Status::Ok)
        );
    }

    #[test]
    fn an_incomplete_candle_voids_the_scope_and_a_tainted_ladder_refuses() {
        let mut day = flag_day();
        day[5].held = false; // a Kite hole inside the box
        let candles = fold(&day, 1);
        let top = vec![true; 8];
        let s = e1c(&candles, &top, &seed(Side::Long, 2), &Readings::HEADLINE);
        assert!(
            s.iter().any(|e| e.candle == 5
                && e.status == Status::IncompleteCandle
                && e.levels.is_none())
        );
        assert!(s.iter().all(|e| e.status != Status::Ok));
        // under the extreme-candles taint a hole before the push leaves the
        // break signalling; under the headline taint it is refused by name
        let mut early = flag_day();
        early[1].held = false;
        let s = e1c(
            &fold(&early, 1),
            &top,
            &seed(Side::Long, 2),
            &Readings::HEADLINE,
        );
        assert_eq!(
            s.iter().find(|e| e.candle == 6).map(|e| e.status),
            Some(Status::TaintedLadder)
        );
        let mut r = Readings::HEADLINE;
        r.taint = crate::ladder::TaintScope::ExtremeCandles;
        let s = e1c(&fold(&early, 1), &top, &seed(Side::Long, 2), &r);
        assert_eq!(
            s.iter().find(|e| e.candle == 6).map(|e| e.status),
            Some(Status::Ok)
        );
        // an incomplete break candle
        let mut bad_break = flag_day();
        bad_break[6].padded = true; // padded with a GDFL trade
        let s = e1c(
            &fold(&bad_break, 1),
            &top,
            &seed(Side::Long, 2),
            &Readings::HEADLINE,
        );
        assert_eq!(
            s.iter().find(|e| e.candle == 6).map(|e| e.status),
            Some(Status::IncompleteCandle)
        );
    }

    #[test]
    fn readings_after_a_break_and_of_the_box_anchor_differ() {
        // a break at 6 without a new high, then a second box [7, 8] broken at
        // 9 by a close equal to the session high (not a new high)
        let mut day: Vec<Minute> = flag_day().into_iter().take(6).collect();
        day.push(minute(6, 10_180, 10_198, 10_175, 10_197));
        day.push(minute(7, 10_197, 10_199, 10_180, 10_185));
        day.push(minute(8, 10_185, 10_196, 10_182, 10_190));
        day.push(minute(9, 10_190, 10_200, 10_186, 10_200));
        let candles = fold(&day, 1);
        let top = vec![true; candles.len()];
        let breaks = |r: &Readings| -> Vec<u32> {
            e1c(&candles, &top, &seed(Side::Long, 2), r)
                .iter()
                .filter(|e| e.status == Status::Ok)
                .map(|e| e.candle)
                .collect()
        };
        assert_eq!(breaks(&Readings::HEADLINE), [6]);
        let mut r = Readings::HEADLINE;
        r.after_break = AfterBreak::NextBox;
        assert_eq!(breaks(&r), [6, 9]);
        // anchored against sliding: no break on the candle after the box,
        // then a close above the anchored box but not the slid one
        let mut slow: Vec<Minute> = flag_day().into_iter().take(6).collect();
        slow.push(minute(6, 10_180, 10_199, 10_170, 10_175)); // red, no break
        slow.push(minute(7, 10_175, 10_198, 10_172, 10_197)); // > 10,195, < 10,199
        let candles = fold(&slow, 1);
        let top = vec![true; candles.len()];
        let ok = |r: &Readings| -> Vec<(u32, u32)> {
            e1c(&candles, &top, &seed(Side::Long, 2), r)
                .iter()
                .filter(|e| e.status == Status::Ok)
                .map(|e| (e.candle, e.box_first))
                .collect()
        };
        assert!(ok(&Readings::HEADLINE).is_empty());
        r = Readings::HEADLINE;
        r.box_anchor = BoxAnchor::Anchored;
        assert_eq!(ok(&r), [(7, 4)]);
        assert_eq!(box_for(5, 4, 2, BoxAnchor::Sliding), None);
        assert_eq!(box_for(6, 4, 2, BoxAnchor::Sliding), Some((4, 5)));
        assert_eq!(box_for(9, 4, 2, BoxAnchor::Sliding), Some((7, 8)));
        assert_eq!(box_for(9, 4, 2, BoxAnchor::Anchored), Some((4, 5)));
        assert_eq!(box_for(9, 4, 0, BoxAnchor::Sliding), Some((8, 8)));
    }

    #[test]
    fn the_second_reading_triggers_on_the_first_second_beyond() {
        let day = flag_day();
        let candles = fold(&day, 1);
        let top = vec![true; candles.len()];
        // seconds of minute 6 (09:21): a print above 10,195 at 09:21:20
        let secs: Vec<Sec> = (0..60)
            .map(|k| {
                let t = OPEN + 6 * 60 + k;
                let px = if k >= 20 { 10_200 } else { 10_185 };
                Sec {
                    t,
                    o: px,
                    h: px,
                    l: px,
                    c: px,
                    v: 50,
                }
            })
            .collect();
        let feed = SecondFeed {
            minutes: &day,
            secs: &secs,
            step: 5,
            data_start: OPEN,
        };
        let ev = e1s(
            &candles,
            &top,
            &feed,
            &seed(Side::Long, 2),
            &Readings::HEADLINE,
        );
        let hit = ev.iter().find(|e| e.t.is_some()).copied();
        assert_eq!(
            hit.and_then(|e| e.trigger_s).map(hms),
            Some("09:21:20".to_owned())
        );
        assert_eq!(hit.and_then(|e| e.t).map(hms), Some("09:21:21".to_owned()));
        assert_eq!(hit.map(|e| e.candle), Some(5));
        assert_eq!(hit.and_then(|e| e.trigger), Some(10_195));
        assert_eq!(hit.map(|e| e.status), Some(Status::Ok)); // the arming candle 5 is green
        // data start after the arming close + 14 s refuses by name
        let late = SecondFeed {
            data_start: OPEN + 23 * 60,
            ..feed
        };
        let ev = e1s(
            &candles,
            &top,
            &late,
            &seed(Side::Long, 2),
            &Readings::HEADLINE,
        );
        assert!(ev.iter().any(|e| e.status == Status::NoFillDataWindow));
        // no GDFL price in R1's minute refuses the trigger by name
        let mut quiet = day.clone();
        for m in &mut quiet {
            m.g = None;
        }
        let feed2 = SecondFeed {
            minutes: &quiet,
            ..feed
        };
        let ev = e1s(
            &candles,
            &top,
            &feed2,
            &seed(Side::Long, 2),
            &Readings::HEADLINE,
        );
        assert!(ev.iter().all(|e| e.status == Status::NoTriggerRatio));
    }
}

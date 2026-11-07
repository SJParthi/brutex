//! E3, the opening range on top movers (PREVIEW-SPEC 25.2.1 with 25.9's
//! correction): E3C on the close beyond the range, E3S on the first second
//! beyond, for the durations 1, 2, 3, 5, 10, 15, 30 and 60 minutes.
//!
//! Written for a long; a short is the long's rule on the price mirror.
//!
//! * **The range**: `H_d`, `L_d` over the session's first d 1-minute candles;
//!   it closes at 09:15 + d minutes. A hole or padded-traded minute inside
//!   it: no E3 on that d that day (`orb_tainted`); `H_d = L_d`:
//!   `orb_no_width`.
//! * **No look-ahead**: a candle whose close falls on or before the range's
//!   close writes no record (headline; `OrbBoundary::Arms` lets the candle
//!   closing exactly at the range's close arm E3S).
//! * **E3C**: a close above `H_d` while no close since the range's close was
//!   above it, or since the last close at or below it: every fresh cross is a
//!   break, numbered by its ordinal today (ORBB; seed: the first).
//! * **E3S**: armed at each close at or inside `H_d`; the first GDFL second
//!   from that close + 14 s to the next close whose high exceeds the
//!   R1-mapped `H_d`; T = that second + 1; the trigger consumes the arming.
//! * **Stop** (S50 seed) `L_d` less the offset; targets as E1's lanes.

use crate::bars::{Candle, Minute, Sec, first_at_or_after};
use crate::clock::{CANDLE_END, LAMBDA, LAST_ENTRY, OPEN};
use crate::e1::{Levels, SecondFeed, oriented};
use crate::ladder::{Ladder, R1272, R1618, running};
use crate::model::{OrbBoundary, Readings, Side, Status, TargetLadder, Toward, map_level, r1_at};

/// The durations of the opening range, minutes (requirement 27).
pub const DURATIONS: [u32; 8] = [1, 2, 3, 5, 10, 15, 30, 60];

/// E3's reading.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum E3Reading {
    /// The close beyond the range (E3C).
    Close,
    /// The first second beyond (E3S).
    Second,
}

/// ORBB, which break of the range today.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BreakClause {
    /// The first (seed).
    First,
    /// The second.
    Second,
    /// The second or later.
    SecondOrLater,
    /// Any.
    Any,
}

impl BreakClause {
    /// Whether break ordinal `k` (1 = the first) passes.
    pub fn passes(self, k: u32) -> bool {
        match self {
            BreakClause::First => k == 1,
            BreakClause::Second => k == 2,
            BreakClause::SecondOrLater => k >= 2,
            BreakClause::Any => true,
        }
    }

    /// Name for the golden files.
    pub fn name(self) -> &'static str {
        match self {
            BreakClause::First => "first",
            BreakClause::Second => "second",
            BreakClause::SecondOrLater => "second_or_later",
            BreakClause::Any => "any",
        }
    }
}

/// One E3 recipe's dials.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct E3Params {
    /// Side.
    pub side: Side,
    /// Duration d, minutes.
    pub d: u32,
    /// Reading.
    pub reading: E3Reading,
    /// ORBB clause.
    pub clause: BreakClause,
    /// Stop offset beyond the far edge, paise (seed one step).
    pub stop_offset: i64,
    /// T1 offset beyond the session extreme, paise (seed one step).
    pub t1_offset: i64,
}

/// The opening range of one duration, real prices.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OrbRange {
    /// Duration, minutes.
    pub d: u32,
    /// `H_d`, paise.
    pub h: i64,
    /// `L_d`, paise.
    pub l: i64,
    /// The second it closes.
    pub close_t: u32,
    /// `Ok`, `orb_tainted` or `orb_no_width`.
    pub status: Status,
}

/// The opening range of `d` minutes from a day's minutes (real prices).
pub fn range(minutes: &[Minute], d: u32) -> OrbRange {
    let d = d.max(1);
    let span = minutes.get(..d as usize).unwrap_or(&[]);
    let mut out = OrbRange {
        d,
        h: i64::MIN,
        l: i64::MAX,
        close_t: OPEN + 60 * d,
        status: Status::Ok,
    };
    if span.len() < d as usize || span.iter().any(Minute::hole) {
        out.status = Status::OrbTainted;
    }
    for m in span.iter().filter(|m| m.held) {
        out.h = out.h.max(m.h);
        out.l = out.l.min(m.l);
    }
    if out.status == Status::Ok && out.h == out.l {
        out.status = Status::OrbNoWidth;
    }
    out
}

/// One E3 record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct E3Event {
    /// Close or second.
    pub reading: E3Reading,
    /// The break candle (close) or the arming candle (second).
    pub candle: u32,
    /// Which break today, 1 = the first.
    pub ordinal: u32,
    /// The arming close (second reading).
    pub armed_at: Option<u32>,
    /// The R1-mapped `H_d` (a short: `L_d`), real paise.
    pub trigger: Option<i64>,
    /// The trigger second.
    pub trigger_s: Option<u32>,
    /// The decision.
    pub t: Option<u32>,
    /// REF (second reading), real paise.
    pub ref_px: Option<i64>,
    /// `Ok`, a clause or a refusal by name.
    pub status: Status,
    /// References at the decision.
    pub levels: Option<Levels>,
}

/// E3's outputs for one day, one duration and one recipe.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct E3Out {
    /// The range.
    pub range: OrbRange,
    /// Every record.
    pub events: Vec<E3Event>,
}

/// The references at a decision whose last closed candle is `ix` (long
/// space).
fn levels_at(far: i64, ladders: &[Ladder], ix: usize, p: &E3Params, r: &Readings) -> Levels {
    let target = match r.target_ladder {
        TargetLadder::AfterFold => ladders[ix + 1],
        TargetLadder::Judging => ladders[ix],
    };
    let rung = |x: u32| {
        if target.usable() {
            target.rung_milli(x)
        } else {
            None
        }
    };
    Levels {
        pattern_extreme: far,
        stop_milli: i128::from(far - p.stop_offset) * 1000,
        t1_milli: i128::from(ladders[ix + 1].hi + p.t1_offset) * 1000,
        t2_milli: rung(R1272),
        t3_milli: rung(R1618),
    }
}

/// E3 over one day for one recipe. `top[i]` is the rank clause at candle
/// `i`'s close for the recipe's side.
pub fn e3(
    candles: &[Candle],
    top: &[bool],
    feed: &SecondFeed<'_>,
    p: &E3Params,
    r: &Readings,
) -> E3Out {
    let long_side = p.side.long();
    let real = range(feed.minutes, p.d);
    let mut out = E3Out {
        range: real,
        events: Vec::new(),
    };
    if real.status != Status::Ok {
        return out;
    }
    let long = oriented(candles, p.side);
    let secs: Vec<Sec> = if long_side {
        feed.secs.to_vec()
    } else {
        feed.secs.iter().map(Sec::mirror).collect()
    };
    let ladders = running(&long, r.ties, r.taint);
    let (h, far) = if long_side {
        (real.h, real.l)
    } else {
        (-real.l, -real.h)
    };
    let unmirror = |lv: Levels| -> Levels {
        if long_side {
            lv
        } else {
            Levels {
                pattern_extreme: -lv.pattern_extreme,
                stop_milli: -lv.stop_milli,
                t1_milli: -lv.t1_milli,
                t2_milli: lv.t2_milli.map(|x| -x),
                t3_milli: lv.t3_milli.map(|x| -x),
            }
        }
    };
    let mut prev_above: Option<bool> = None;
    let mut ordinal = 0u32;
    for (ix, c) in long.iter().enumerate() {
        let after = c.close_t > real.close_t;
        let boundary = c.close_t == real.close_t && r.orb_boundary == OrbBoundary::Arms;
        if !after && !boundary {
            continue;
        }
        let above = c.c > h;
        match p.reading {
            E3Reading::Close => {
                if after && above && prev_above != Some(true) {
                    ordinal += 1;
                    let status = if c.incomplete {
                        Status::IncompleteCandle
                    } else if !p.clause.passes(ordinal) {
                        Status::BreakOrdinal
                    } else if !top.get(ix).copied().unwrap_or(false) {
                        Status::NotInTopN
                    } else if c.close_t > LAST_ENTRY {
                        Status::AfterLastEntry
                    } else {
                        Status::Ok
                    };
                    out.events.push(E3Event {
                        reading: E3Reading::Close,
                        candle: c.i,
                        ordinal,
                        armed_at: None,
                        trigger: None,
                        trigger_s: None,
                        t: Some(c.close_t),
                        ref_px: None,
                        status,
                        levels: Some(unmirror(levels_at(far, &ladders, ix, p, r))),
                    });
                }
            }
            E3Reading::Second => {
                if !above && !c.incomplete {
                    let next = long.get(ix + 1).map_or(CANDLE_END, |n| n.close_t);
                    if let Some(ev) = arm(
                        c, ix, next, h, far, &secs, &ladders, feed, p, r, top, ordinal,
                    ) {
                        if ev.t.is_some() {
                            ordinal += 1;
                        }
                        let mut ev = ev;
                        ev.levels = ev.levels.map(unmirror);
                        if ev.status != Status::Ok || ev.t.is_some() {
                            out.events.push(ev);
                        }
                    }
                }
            }
        }
        prev_above = Some(above);
    }
    out
}

/// One E3S arming at candle `ix` (long space); `None` when nothing printed
/// before the next close.
#[allow(
    clippy::too_many_arguments,
    reason = "an arming reads the whole day's state"
)]
fn arm(
    c: &Candle,
    ix: usize,
    next: u32,
    h: i64,
    far: i64,
    secs: &[Sec],
    ladders: &[Ladder],
    feed: &SecondFeed<'_>,
    p: &E3Params,
    r: &Readings,
    top: &[bool],
    done: u32,
) -> Option<E3Event> {
    let long_side = p.side.long();
    let mut ev = E3Event {
        reading: E3Reading::Second,
        candle: c.i,
        ordinal: done + 1,
        armed_at: Some(c.close_t),
        trigger: None,
        trigger_s: None,
        t: None,
        ref_px: None,
        status: Status::Ok,
        levels: None,
    };
    if c.close_t + LAMBDA < feed.data_start {
        ev.status = Status::NoFillDataWindow;
        return Some(ev);
    }
    let Some((z, g)) = r1_at(feed.minutes, c.close_t) else {
        ev.status = Status::NoTriggerRatio;
        return Some(ev);
    };
    let real_level = if long_side { h } else { -h };
    let toward = if long_side { Toward::Up } else { Toward::Down };
    let real_trigger = map_level(i128::from(real_level) * 1000, z, g, feed.step, toward);
    ev.trigger = Some(real_trigger);
    let trigger = if long_side {
        real_trigger
    } else {
        -real_trigger
    };
    let hit = secs[first_at_or_after(secs, c.close_t + LAMBDA)..]
        .iter()
        .take_while(|s| s.t < next)
        .find(|s| s.traded() && s.h > trigger)
        .copied()?;
    let t = hit.t + 1;
    ev.trigger_s = Some(hit.t);
    ev.t = Some(t);
    ev.ref_px = Some(if long_side { hit.c } else { -hit.c });
    ev.status = if !p.clause.passes(done + 1) {
        Status::BreakOrdinal
    } else if !top.get(ix).copied().unwrap_or(false) {
        Status::NotInTopN
    } else if t > LAST_ENTRY {
        Status::AfterLastEntry
    } else {
        Status::Ok
    };
    ev.levels = Some(levels_at(far, ladders, ix, p, r));
    Some(ev)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bars::fold;
    use crate::clock::hms;
    use crate::e1::tests::minute;

    fn seed(side: Side, d: u32) -> E3Params {
        E3Params {
            side,
            d,
            reading: E3Reading::Close,
            clause: BreakClause::First,
            stop_offset: 5,
            t1_offset: 5,
        }
    }

    /// A range of 2 minutes (10,000 - 10,100), then a break at minute 4, a
    /// fall back inside at 5 and a second break at 6.
    fn orb_day() -> Vec<Minute> {
        vec![
            minute(0, 10_050, 10_100, 10_000, 10_060),
            minute(1, 10_060, 10_080, 10_020, 10_070),
            minute(2, 10_070, 10_095, 10_050, 10_090), // inside
            minute(3, 10_090, 10_100, 10_080, 10_100), // at H: not above
            minute(4, 10_100, 10_130, 10_095, 10_120), // first break
            minute(5, 10_120, 10_125, 10_090, 10_095), // back inside
            minute(6, 10_095, 10_140, 10_090, 10_135), // second break
            minute(7, 10_135, 10_150, 10_130, 10_145),
        ]
    }

    #[test]
    fn the_range_and_its_refusals() {
        let day = orb_day();
        let r = range(&day, 2);
        assert_eq!((r.h, r.l, r.status), (10_100, 10_000, Status::Ok));
        assert_eq!(hms(r.close_t), "09:17:00");
        let mut holed = day.clone();
        holed[1].held = false;
        assert_eq!(range(&holed, 2).status, Status::OrbTainted);
        assert_eq!(range(&holed, 1).status, Status::Ok);
        let mut padded = day.clone();
        padded[0].padded = true; // padded with a GDFL trade
        assert_eq!(range(&padded, 1).status, Status::OrbTainted);
        let flat = vec![minute(0, 100, 100, 100, 100)];
        assert_eq!(range(&flat, 1).status, Status::OrbNoWidth);
        assert_eq!(range(&flat, 2).status, Status::OrbTainted); // not enough data
        assert_eq!(range(&day, 0).d, 1);
        assert_eq!(DURATIONS.len(), 8);
    }

    #[test]
    fn every_fresh_cross_is_a_break_numbered_by_its_ordinal() {
        let day = orb_day();
        let candles = fold(&day, 1);
        let top = vec![true; candles.len()];
        let feed = SecondFeed {
            minutes: &day,
            secs: &[],
            step: 5,
            data_start: OPEN,
        };
        let out = e3(
            &candles,
            &top,
            &feed,
            &seed(Side::Long, 2),
            &Readings::HEADLINE,
        );
        let got: Vec<(u32, u32, Status)> = out
            .events
            .iter()
            .map(|e| (e.candle, e.ordinal, e.status))
            .collect();
        assert_eq!(got, [(4, 1, Status::Ok), (6, 2, Status::BreakOrdinal)]);
        let lv = out.events[0]
            .levels
            .map(|l| (l.pattern_extreme, l.stop_milli, l.t1_milli));
        assert_eq!(lv, Some((10_000, 9_995_000, 10_135_000)));
        let any = E3Params {
            clause: BreakClause::Any,
            ..seed(Side::Long, 2)
        };
        let out = e3(&candles, &top, &feed, &any, &Readings::HEADLINE);
        assert!(out.events.iter().all(|e| e.status == Status::Ok));
        assert!(BreakClause::Second.passes(2) && !BreakClause::Second.passes(3));
        assert!(BreakClause::SecondOrLater.passes(3) && !BreakClause::SecondOrLater.passes(1));
        assert_eq!(BreakClause::Any.name(), "any");
    }

    #[test]
    fn no_record_inside_the_range_and_the_short_mirror() {
        let day = orb_day();
        let candles = fold(&day, 1);
        let top = vec![true; candles.len()];
        let feed = SecondFeed {
            minutes: &day,
            secs: &[],
            step: 5,
            data_start: OPEN,
        };
        // d = 5: the range is minutes 0..4 (H 10,130); the first break is at 6
        let out = e3(
            &candles,
            &top,
            &feed,
            &seed(Side::Long, 5),
            &Readings::HEADLINE,
        );
        assert_eq!(out.events.iter().map(|e| e.candle).collect::<Vec<_>>(), [6]);
        // the mirrored day breaks the mirrored low
        let mirrored: Vec<Minute> = day
            .iter()
            .map(|m| Minute {
                o: 30_000 - m.o,
                h: 30_000 - m.l,
                l: 30_000 - m.h,
                c: 30_000 - m.c,
                g: m.g.map(|g| 30_000 - g),
                ..*m
            })
            .collect();
        let mc = fold(&mirrored, 1);
        let mf = SecondFeed {
            minutes: &mirrored,
            ..feed
        };
        let out = e3(&mc, &top, &mf, &seed(Side::Short, 2), &Readings::HEADLINE);
        assert_eq!(
            out.events
                .iter()
                .map(|e| (e.candle, e.ordinal))
                .collect::<Vec<_>>(),
            [(4, 1), (6, 2)]
        );
        assert_eq!(
            out.events[0].levels.map(|l| l.stop_milli),
            Some((30_000 - 9_995) * 1000)
        );
        let tainted = SecondFeed {
            minutes: &[],
            ..feed
        };
        assert!(
            e3(
                &candles,
                &top,
                &tainted,
                &seed(Side::Long, 2),
                &Readings::HEADLINE
            )
            .events
            .is_empty()
        );
    }

    #[test]
    fn the_second_reading_triggers_after_the_blind_14_seconds() {
        let day = orb_day();
        let candles = fold(&day, 1);
        let top = vec![true; candles.len()];
        // during minute 4 a print above 10,100 at 09:19:10 (inside the 14 s
        // after the 09:19:00 close: unseen) and at 09:19:20
        let secs: Vec<Sec> = (0..60)
            .map(|k| {
                let t = OPEN + 4 * 60 + k;
                let px = if k == 10 || k >= 20 { 10_110 } else { 10_090 };
                Sec {
                    t,
                    o: px,
                    h: px,
                    l: px,
                    c: px,
                    v: 5,
                }
            })
            .collect();
        let feed = SecondFeed {
            minutes: &day,
            secs: &secs,
            step: 5,
            data_start: OPEN,
        };
        let p = E3Params {
            reading: E3Reading::Second,
            ..seed(Side::Long, 2)
        };
        let out = e3(&candles, &top, &feed, &p, &Readings::HEADLINE);
        let hit: Vec<(u32, Option<u32>)> =
            out.events.iter().map(|e| (e.candle, e.trigger_s)).collect();
        assert_eq!(hit, [(3, Some(OPEN + 4 * 60 + 20))]);
        assert_eq!(out.events[0].t, Some(OPEN + 4 * 60 + 21));
        assert_eq!(out.events[0].trigger, Some(10_100));
        // the boundary reading lets the candle closing at 09:17:00 arm
        let mut r = Readings::HEADLINE;
        r.orb_boundary = OrbBoundary::Arms;
        let secs2: Vec<Sec> = (0..60)
            .map(|k| {
                let t = OPEN + 2 * 60 + k;
                let px = if k >= 30 { 10_110 } else { 10_090 };
                Sec {
                    t,
                    o: px,
                    h: px,
                    l: px,
                    c: px,
                    v: 5,
                }
            })
            .collect();
        let f2 = SecondFeed {
            secs: &secs2,
            ..feed
        };
        let head = e3(&candles, &top, &f2, &p, &Readings::HEADLINE);
        assert!(head.events.is_empty());
        let arms = e3(&candles, &top, &f2, &p, &r);
        assert_eq!(
            arms.events.iter().map(|e| e.candle).collect::<Vec<_>>(),
            [1]
        );
        // a late data start and a minute with no GDFL price refuse by name
        let late = SecondFeed {
            data_start: OPEN + 3600,
            ..feed
        };
        assert!(
            e3(&candles, &top, &late, &p, &Readings::HEADLINE)
                .events
                .iter()
                .all(|e| e.status == Status::NoFillDataWindow)
        );
        let mut quiet = day.clone();
        for m in &mut quiet[2..] {
            m.g = None;
        }
        let qf = SecondFeed {
            minutes: &quiet,
            ..feed
        };
        assert!(
            e3(&candles, &top, &qf, &p, &Readings::HEADLINE)
                .events
                .iter()
                .all(|e| e.status == Status::NoTriggerRatio)
        );
    }
}

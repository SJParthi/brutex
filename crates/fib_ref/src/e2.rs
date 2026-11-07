//! E2, the second touch (PREVIEW-SPEC 7.2): the touch book, its states and
//! counts, the close reading (E2C) and the second reading (E2S).
//!
//! Written for a long; a short is the long's rule on the price mirror.
//!
//! * **Level and signature**: one rung of one ladder; the same level is the
//!   same rung on the same `(lo, hi, leg)`. A signature change or a new
//!   session restarts the count. Running mode reads the ladder each candle
//!   was judged against; frozen mode freezes that ladder at the first touch
//!   (the ladder the touch was judged against, so the count keeps the
//!   touched level) and releases it when the count ends.
//! * **Relations** at tolerance t: touch `low - t <= L <= high + t`, clear
//!   above, through below.
//! * **States** Idle, At, Away, Ended. The first touch must come from the
//!   favourable side: the last non-touching candle before it was clear. The
//!   bounce is the best high of clear candles since the touch's last candle
//!   (the touch's last candle and the next touching candle never count); a
//!   through candle wipes it; it passes at `bounce >= the m-th rung above L`
//!   with no tolerance. The return is the first candle of a later touch once
//!   the bounce passed: n advances by one, gap = candles strictly between. A
//!   hole or incomplete candle ends the count (`touch_count_unknown`); S29b's
//!   labelled reading ends it on a close beyond the level by more than t.
//! * **E2C**: the touch candle's close; market + 14 s. **E2S**: armed at the
//!   close where the bounce passes, for the next touch number; the first
//!   traded second from that close + 14 s whose low reaches the R1-mapped
//!   `L + t` decides at that second + 1 s. Its ends: a signature change, a
//!   hole minute's close, a through candle's close, the close of the touch
//!   candle it is armed for, S42; at a tie the end wins. Resting is refused
//!   by name, `touch_order_unsourced`.

use crate::bars::{Candle, Minute, Sec, first_at_or_after};
use crate::clock::{LAMBDA, LAST_ENTRY, minute_start};
use crate::e1::{LadderMode, SecondFeed, oriented};
use crate::ladder::{Ladder, Leg, R1272, R1618, Relation, relate, running};
use crate::model::{Readings, Retouch, Side, Status, TargetLadder, Toward, map_level, r1_at};

/// The entry reading S40 for a return.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reading {
    /// The touch candle's close (seed).
    Close,
    /// The first GDFL second reaching the trigger.
    Second,
    /// A resting order: refused by name, `touch_order_unsourced`.
    Resting,
}

/// The touch-number clause S27.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TouchClause {
    /// Exactly the n-th touch.
    Eq(u32),
    /// The n-th touch or later.
    AtLeast(u32),
}

impl TouchClause {
    /// Whether touch number `n` passes.
    pub fn passes(self, n: u32) -> bool {
        match self {
            TouchClause::Eq(x) => n == x,
            TouchClause::AtLeast(x) => n >= x,
        }
    }

    /// `=2`, `>=3` for the golden files.
    pub fn name(self) -> String {
        match self {
            TouchClause::Eq(x) => format!("={x}"),
            TouchClause::AtLeast(x) => format!(">={x}"),
        }
    }
}

/// One E2 recipe's dials.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct E2Params {
    /// Side.
    pub side: Side,
    /// The rung, thousandths (seed 786).
    pub rung: u32,
    /// Tolerance S24, milli-paise (seed 0).
    pub tol_milli: i128,
    /// Bounce S28: m rungs further (seed 1).
    pub bounce_m: u32,
    /// Touch-number clause S27 (seed `= 2`).
    pub clause: TouchClause,
    /// Ladder mode (seed running).
    pub mode: LadderMode,
    /// S29b's labelled reading: a close beyond the level by more than t ends
    /// the count (seed: the count goes on).
    pub close_beyond_ends: bool,
    /// Entry reading (seed close).
    pub reading: Reading,
    /// Stop offset beyond the touches' extreme, paise (seed one step).
    pub stop_offset: i64,
    /// T1 offset beyond the session extreme, paise (seed one step).
    pub t1_offset: i64,
    /// XJ offset beyond the bounce high, paise (seed 0).
    pub xj_offset: i64,
}

/// The touch book's state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// No counted touch yet on this signature.
    Idle,
    /// In a touch.
    At,
    /// After a touch, waiting for the return.
    Away,
    /// The count ended for this signature.
    Ended,
}

impl Phase {
    /// Short name for the golden files.
    pub fn name(self) -> &'static str {
        match self {
            Phase::Idle => "idle",
            Phase::At => "at",
            Phase::Away => "away",
            Phase::Ended => "ended",
        }
    }
}

/// The touch book after one candle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Trace {
    /// Candle index.
    pub candle: u32,
    /// The level, milli-paise (real prices), when the signature is usable.
    pub level_milli: Option<i128>,
    /// The candle's relation to the level.
    pub rel: Option<Relation>,
    /// State after the candle.
    pub phase: Phase,
    /// Touch number of the current or last touch.
    pub n: u32,
    /// The bounce so far (real price), if any.
    pub bounce: Option<i64>,
    /// The bounce has passed.
    pub passed: bool,
    /// What this candle did: `first_touch`, `return`, `merge`, `ended`, or
    /// empty.
    pub event: &'static str,
    /// Gap of a return, candles strictly between.
    pub gap: Option<u32>,
}

/// References fixed at an E2 decision, real prices.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct E2Levels {
    /// The level L, milli-paise.
    pub level_milli: i128,
    /// The touches' extreme so far (a long: lowest low), paise.
    pub pattern_extreme: i64,
    /// The stop reference, milli-paise.
    pub stop_milli: i128,
    /// T1, milli-paise.
    pub t1_milli: i128,
    /// T2 1.272, or `None`.
    pub t2_milli: Option<i128>,
    /// T3 1.618, or `None`.
    pub t3_milli: Option<i128>,
    /// XJ, the bounce high plus the offset, or `None` (a first touch).
    pub xj_milli: Option<i128>,
}

impl E2Levels {
    fn unmirror(self) -> E2Levels {
        E2Levels {
            level_milli: -self.level_milli,
            pattern_extreme: -self.pattern_extreme,
            stop_milli: -self.stop_milli,
            t1_milli: -self.t1_milli,
            t2_milli: self.t2_milli.map(|x| -x),
            t3_milli: self.t3_milli.map(|x| -x),
            xj_milli: self.xj_milli.map(|x| -x),
        }
    }
}

/// One E2 signal record (close or second reading).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct E2Signal {
    /// Close, second or resting.
    pub reading: Reading,
    /// Touch number (a second reading: the number it is armed for).
    pub n: u32,
    /// The touch candle (close) or the arming candle (second).
    pub candle: u32,
    /// The arming close (second reading).
    pub armed_at: Option<u32>,
    /// The R1-mapped trigger `L + t` (second reading), real paise.
    pub trigger: Option<i64>,
    /// The second the arming ended, and why (second reading).
    pub end: Option<(u32, &'static str)>,
    /// The trigger second (second reading).
    pub trigger_s: Option<u32>,
    /// The decision second.
    pub t: Option<u32>,
    /// REF at the decision (second reading: the trigger second's last
    /// print), real paise.
    pub ref_px: Option<i64>,
    /// `Ok`, a clause or a refusal by name.
    pub status: Status,
    /// References at the decision.
    pub levels: Option<E2Levels>,
}

/// E2's outputs for one day and one recipe.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct E2Out {
    /// The touch book after every candle.
    pub trace: Vec<Trace>,
    /// Every signal record.
    pub signals: Vec<E2Signal>,
}

/// An active second-reading arming.
#[derive(Clone, Copy, Debug)]
struct Arm {
    candle: usize,
    at: u32,
    n_for: u32,
    level: i128,
    extreme: i64,
    sig: Option<(i64, i64, Leg)>,
}

fn sig_of(l: &Ladder) -> Option<(i64, i64, Leg)> {
    l.usable().then_some((l.lo, l.hi, l.leg))
}

/// The touch book's running state (long space).
#[derive(Clone, Copy, Debug)]
struct Book {
    sig: Option<(i64, i64, Leg)>,
    frozen: Option<Ladder>,
    phase: Phase,
    n: u32,
    prev: Option<Relation>,
    touch_last: u32,
    extreme: Option<i64>,
    bounce: Option<i64>,
    passed: bool,
}

impl Book {
    const START: Book = Book {
        sig: None,
        frozen: None,
        phase: Phase::Idle,
        n: 0,
        prev: None,
        touch_last: 0,
        extreme: None,
        bounce: None,
        passed: false,
    };

    fn restart(&mut self, sig: Option<(i64, i64, Leg)>) {
        let frozen = self.frozen;
        *self = Book {
            sig,
            frozen,
            ..Book::START
        };
    }

    fn widen(&mut self, low: i64) {
        self.extreme = Some(self.extreme.map_or(low, |e| e.min(low)));
    }
}

/// What one candle did to the book.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Step {
    rel: Option<Relation>,
    event: &'static str,
    gap: Option<u32>,
    /// The bounce high a return closed (XJ's reference).
    returned_bounce: Option<i64>,
}

/// One candle through the book (long space): the transitions of 7.2.
fn step(
    book: &mut Book,
    c: &Candle,
    lad: &Ladder,
    level: i128,
    p: &E2Params,
    r: &Readings,
) -> Step {
    let mut out = Step {
        rel: None,
        event: "",
        gap: None,
        returned_bounce: None,
    };
    if c.incomplete {
        if book.phase != Phase::Ended {
            book.phase = Phase::Ended;
            out.event = "ended";
        }
        book.frozen = None;
        return out;
    }
    let rr = relate(c.h, c.l, level, p.tol_milli, true);
    out.rel = Some(rr);
    let target = lad.rung_beyond(p.rung, p.bounce_m, true);
    let pass = |b: Option<i64>| match (b, target) {
        (Some(b), Some(t)) => i128::from(b) * 1000 >= t,
        _ => false,
    };
    match (book.phase, rr) {
        (Phase::Idle, Relation::Touch) => {
            if book.prev == Some(Relation::Clear) {
                book.n = 1;
                book.phase = Phase::At;
                book.touch_last = c.i;
                book.extreme = Some(c.l);
                out.event = "first_touch";
                if p.mode == LadderMode::Frozen {
                    book.frozen = Some(*lad);
                }
            }
        }
        (Phase::Idle, other) => book.prev = Some(other),
        (Phase::At, Relation::Touch) => {
            book.touch_last = c.i;
            book.widen(c.l);
        }
        (Phase::At, Relation::Clear) => {
            book.phase = Phase::Away;
            book.bounce = Some(c.h);
            book.passed = pass(book.bounce);
        }
        (Phase::At | Phase::Away, Relation::Through) => {
            book.phase = Phase::Away;
            book.bounce = None;
            book.passed = false;
        }
        (Phase::Away, Relation::Clear) => {
            book.bounce = Some(book.bounce.map_or(c.h, |b| b.max(c.h)));
            book.passed = pass(book.bounce);
        }
        (Phase::Away, Relation::Touch) => {
            if book.passed {
                book.n += 1;
                out.gap = Some(c.i - book.touch_last - 1);
                out.returned_bounce = book.bounce;
                book.phase = Phase::At;
                book.touch_last = c.i;
                book.widen(c.l);
                book.bounce = None;
                book.passed = false;
                out.event = "return";
            } else if r.retouch == Retouch::Merge {
                book.phase = Phase::At;
                book.touch_last = c.i;
                book.widen(c.l);
                book.bounce = None;
                out.event = "merge";
            }
        }
        (Phase::Ended, _) => {}
    }
    if p.close_beyond_ends
        && matches!(book.phase, Phase::At | Phase::Away)
        && i128::from(c.c) * 1000 < level - p.tol_milli
    {
        book.phase = Phase::Ended;
        book.frozen = None;
        // a touch on this candle still happened: its record stands, and the
        // count ends at its close (S29b, labelled)
        if !matches!(out.event, "first_touch" | "return") {
            out.event = "ended";
        }
    }
    out
}

/// E2 over one day for one recipe. `top[i]` is the rank clause at candle
/// `i`'s close for the recipe's side.
pub fn e2(
    candles: &[Candle],
    top: &[bool],
    feed: &SecondFeed<'_>,
    p: &E2Params,
    r: &Readings,
) -> E2Out {
    let long_side = p.side.long();
    let long = oriented(candles, p.side);
    let minutes: Vec<Minute> = if long_side {
        feed.minutes.to_vec()
    } else {
        feed.minutes.iter().map(Minute::mirror).collect()
    };
    let secs: Vec<Sec> = if long_side {
        feed.secs.to_vec()
    } else {
        feed.secs.iter().map(Sec::mirror).collect()
    };
    let ladders = running(&long, r.ties, r.taint);
    let ctx = ArmCtx {
        secs: &secs,
        feed,
        p,
        top,
        ladders: &ladders,
        long: &long,
        r,
    };
    let flip = |x: i128| if long_side { x } else { -x };

    let mut trace = Vec::with_capacity(long.len());
    let mut signals = Vec::new();
    let mut bounces: Vec<Option<i64>> = Vec::with_capacity(long.len());
    let mut book = Book::START;
    let mut arm: Option<Arm> = None;

    for (ix, c) in long.iter().enumerate() {
        let lad = level_ladder(&book, p, &ladders, ix);
        let s = sig_of(&lad);
        if s != book.sig {
            book.restart(s);
        }
        let level = if book.sig.is_some() {
            lad.rung_milli(p.rung)
        } else {
            None
        };
        let was_passed = book.passed;
        let st = match level {
            Some(lv) => step(&mut book, c, &lad, lv, p, r),
            None => Step {
                rel: None,
                event: "",
                gap: None,
                returned_bounce: None,
            },
        };
        bounces.push(book.bounce);

        if matches!(st.event, "first_touch" | "return") && p.reading != Reading::Second {
            signals.push(close_record(&book, &st, c, ix, level.unwrap_or(0), &ctx));
        }
        if p.reading == Reading::Second {
            if let Some(a) = arm
                && let Some(e) = arm_end(
                    &a,
                    &book,
                    &st,
                    c,
                    long[ix.saturating_sub(1)].close_t,
                    &minutes,
                )
            {
                signals.push(finish_arm(&a, e, &minutes, &bounces, &ctx));
                arm = None;
            }
            if book.phase == Phase::Away && book.passed && !was_passed {
                // an Away book always has its level and its touches' extreme
                arm = Some(Arm {
                    candle: ix,
                    at: c.close_t,
                    n_for: book.n + 1,
                    level: level.unwrap_or_default(),
                    extreme: book.extreme.unwrap_or(c.l),
                    sig: book.sig,
                });
            }
        }
        trace.push(Trace {
            candle: c.i,
            level_milli: level.map(flip),
            rel: st.rel,
            phase: book.phase,
            n: book.n,
            bounce: book.bounce.map(|b| if long_side { b } else { -b }),
            passed: book.passed,
            event: st.event,
            gap: st.gap,
        });
    }
    if let Some(a) = arm {
        signals.push(finish_arm(
            &a,
            (LAST_ENTRY, "day_end"),
            &minutes,
            &bounces,
            &ctx,
        ));
    }
    E2Out { trace, signals }
}

/// The ladder the level is read on for candle `ix`.
fn level_ladder(book: &Book, p: &E2Params, ladders: &[Ladder], ix: usize) -> Ladder {
    match (p.mode, book.frozen) {
        (LadderMode::Frozen, Some(f)) => f,
        _ => ladders[ix],
    }
}

/// Whether candle `c` ends an active arming, and at which second: a
/// signature change at the previous close (the arming's own signature is
/// gone), the first hole minute's close, the touch candle's close, a through
/// candle's close.
fn arm_end(
    a: &Arm,
    book: &Book,
    st: &Step,
    c: &Candle,
    prev_close: u32,
    minutes: &[Minute],
) -> Option<(u32, &'static str)> {
    if book.sig != a.sig {
        return Some((prev_close, "signature"));
    }
    if let Some(m) =
        (c.first_minute..=c.last_minute).find(|&m| minutes.get(m as usize).is_none_or(Minute::hole))
    {
        return Some((minute_start(m + 1), "hole"));
    }
    match st.rel {
        Some(Relation::Touch) => Some((c.close_t, "touch_candle")),
        Some(Relation::Through) => Some((c.close_t, "through")),
        _ => None,
    }
}

/// The day-wide inputs an arming and a close record read.
struct ArmCtx<'a> {
    secs: &'a [Sec],
    feed: &'a SecondFeed<'a>,
    p: &'a E2Params,
    top: &'a [bool],
    ladders: &'a [Ladder],
    long: &'a [Candle],
    r: &'a Readings,
}

/// The rung targets of a target ladder.
fn rung_targets(l: &Ladder) -> (Option<i128>, Option<i128>) {
    if l.usable() {
        (l.rung_milli(R1272), l.rung_milli(R1618))
    } else {
        (None, None)
    }
}

/// The E2C record of a first touch or a return at candle `ix`.
fn close_record(
    book: &Book,
    st: &Step,
    c: &Candle,
    ix: usize,
    level: i128,
    ctx: &ArmCtx<'_>,
) -> E2Signal {
    let p = ctx.p;
    let status = if p.reading == Reading::Resting {
        Status::TouchOrderUnsourced
    } else if !p.clause.passes(book.n) {
        Status::TouchNumber
    } else if !ctx.top.get(ix).copied().unwrap_or(false) {
        Status::NotInTopN
    } else if c.close_t > LAST_ENTRY {
        Status::AfterLastEntry
    } else {
        Status::Ok
    };
    let ex = book.extreme.unwrap_or(c.l);
    let tl = match ctx.r.target_ladder {
        TargetLadder::AfterFold => ctx.ladders[ix + 1],
        TargetLadder::Judging => ctx.ladders[ix],
    };
    let (t2, t3) = rung_targets(&tl);
    let levels = E2Levels {
        level_milli: level,
        pattern_extreme: ex,
        stop_milli: i128::from(ex - p.stop_offset) * 1000,
        t1_milli: i128::from(ctx.ladders[ix + 1].hi + p.t1_offset) * 1000,
        t2_milli: t2,
        t3_milli: t3,
        xj_milli: st
            .returned_bounce
            .map(|b| i128::from(b + p.xj_offset) * 1000),
    };
    E2Signal {
        reading: p.reading,
        n: book.n,
        candle: c.i,
        armed_at: None,
        trigger: None,
        end: None,
        trigger_s: None,
        t: Some(c.close_t),
        ref_px: None,
        status,
        levels: Some(if p.side.long() {
            levels
        } else {
            levels.unmirror()
        }),
    }
}

/// Scans an arming's window and writes its record.
fn finish_arm(
    a: &Arm,
    end: (u32, &'static str),
    minutes: &[Minute],
    bounces: &[Option<i64>],
    ctx: &ArmCtx<'_>,
) -> E2Signal {
    let p = ctx.p;
    let long_side = p.side.long();
    let mut out = E2Signal {
        reading: Reading::Second,
        n: a.n_for,
        candle: ctx.long[a.candle].i,
        armed_at: Some(a.at),
        trigger: None,
        end: Some(end),
        trigger_s: None,
        t: None,
        ref_px: None,
        status: Status::Ok,
        levels: None,
    };
    if a.at + LAMBDA < ctx.feed.data_start {
        out.status = Status::NoFillDataWindow;
        return out;
    }
    let Some((z, g)) = r1_at(ctx.feed.minutes, a.at) else {
        out.status = Status::NoTriggerRatio;
        return out;
    };
    let _ = minutes;
    let real_level = if long_side {
        a.level + p.tol_milli
    } else {
        -(a.level + p.tol_milli)
    };
    let toward = if long_side { Toward::Down } else { Toward::Up };
    let real_trigger = map_level(real_level, z, g, ctx.feed.step, toward);
    out.trigger = Some(real_trigger);
    let trigger = if long_side {
        real_trigger
    } else {
        -real_trigger
    };
    let stop_at = end.0.min(LAST_ENTRY);
    let Some(hit) = ctx.secs[first_at_or_after(ctx.secs, a.at + LAMBDA)..]
        .iter()
        .take_while(|s| s.t < stop_at)
        .find(|s| s.traded() && s.l <= trigger)
        .copied()
    else {
        return out;
    };
    let t = hit.t + 1;
    out.trigger_s = Some(hit.t);
    out.t = Some(t);
    out.ref_px = Some(if long_side { hit.c } else { -hit.c });
    let q = ctx
        .long
        .iter()
        .rposition(|c| c.close_t <= t)
        .unwrap_or(a.candle);
    out.status = if !p.clause.passes(a.n_for) {
        Status::TouchNumber
    } else if !ctx.top.get(a.candle).copied().unwrap_or(false) {
        Status::NotInTopN
    } else {
        Status::Ok
    };
    let tl = match ctx.r.target_ladder {
        TargetLadder::AfterFold => ctx.ladders[q + 1],
        TargetLadder::Judging => ctx.ladders[q],
    };
    let (t2, t3) = rung_targets(&tl);
    let levels = E2Levels {
        level_milli: a.level,
        pattern_extreme: a.extreme,
        stop_milli: i128::from(a.extreme - p.stop_offset) * 1000,
        t1_milli: i128::from(ctx.ladders[q + 1].hi + p.t1_offset) * 1000,
        t2_milli: t2,
        t3_milli: t3,
        xj_milli: bounces
            .get(q)
            .copied()
            .flatten()
            .map(|b| i128::from(b + p.xj_offset) * 1000),
    };
    out.levels = Some(if long_side { levels } else { levels.unmirror() });
    out
}

#[cfg(test)]
#[path = "e2_tests.rs"]
pub(crate) mod tests;

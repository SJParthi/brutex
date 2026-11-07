//! The fill walk, lane S (PREVIEW-SPEC 11.1, 9.1, 9.2): one order through
//! its exit, second by second.
//!
//! * **Latency**: a decision at T starts its order at the first traded second
//!   at or after T + 14 s. A resting stop or target acts with no latency from
//!   the second its trigger prints. A move of a resting order (XD, XE) is an
//!   order action decided at the end of the second that triggered it (that
//!   second + 1, as the second reading decides) and takes effect at the first
//!   traded second at or after its decision + 14 s.
//! * **Volume and price**: at most 10% of each second's traded volume
//!   (floor), shared by the entry and the exit; every slice at that second's
//!   worst traded price, a buy at its high, a sell at its low; a zero-volume
//!   second fills nothing.
//! * **Protection**: every market order carries a limit at LTP x 1.03 for a
//!   buy and x 0.97 for a sell (Rs 1.50 at or below Rs 50), LTP the last
//!   print before the order's start; a second whose worst print lies beyond
//!   it fills nothing for that order. A buy limit floors and a sell limit
//!   ceils: never kinder.
//! * **Sizing**: whole shares of the amount at the last print strictly
//!   before the start; zero shares is `price_above_position`.
//! * **Events while filling**: a stop, target or exit while the entry is part
//!   filled cancels the remainder; the exit works on the filled part. A stop
//!   and a target in one second: the stop.
//! * **Session end**: the entry's remainder is cancelled at 15:08:00; a
//!   strategy market exit starting at or after 15:08:00 is rejected; from
//!   15:10:00 the broker squares off whatever is open at the worst print over
//!   15:10:00 to the session's last second, or, with none there, the worst
//!   since the entry, flagged `unclosable_in_session`.
//!
//! A short walks as the long on the price mirror; protection limits are
//! computed on real prices first.

use std::collections::VecDeque;

use crate::bars::{Sec, first_at_or_after, last_print_before};
use crate::clock::{CUT, SQUARE_OFF};
use crate::model::{RestingAfterCut, Side, Status, WithinSecond};
use crate::money::{ceil_div, floor_div};

/// Flag bits a walk carries (PREVIEW-SPEC 11.4's per-rule bits, in part).
pub mod rule {
    /// The first fill's second printed at or through the stop.
    pub const STOP_THROUGH_AT_FILL: u32 = 1 << 0;
    /// The entry filled only part of its quantity.
    pub const PARTIAL: u32 = 1 << 1;
    /// A second's worst print was beyond an order's protection limit.
    pub const PROTECTION_LIMITED: u32 = 1 << 2;
    /// A strategy market exit starting at or after 15:08:00 was rejected.
    pub const CUT_REJECTED: u32 = 1 << 3;
    /// The broker squared off at 15:10.
    pub const BROKER_SQUARE_OFF: u32 = 1 << 4;
    /// No traded second from 15:10:00: closed at the worst since the entry.
    pub const UNCLOSABLE: u32 = 1 << 5;
    /// An order filled over more than one second.
    pub const VOLUME_WALKED: u32 = 1 << 6;
    /// The entry's remainder was cancelled at 15:08:00.
    pub const ENTRY_CUT: u32 = 1 << 7;

    /// The flags' names in bit order.
    pub const NAMES: [&str; 8] = [
        "stop_through_at_fill",
        "partial",
        "protection_limited",
        "cut_rejected",
        "broker_square_off",
        "unclosable_in_session",
        "volume_walked",
        "entry_cut",
    ];
}

/// The exit beside the stop (the lanes of 9.1, prices already R1-mapped).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Exit {
    /// XA, XB, XC, XJ: a resting target.
    Target(i64),
    /// XD2, XD3: at `trigger` the stop moves to the entry price and the
    /// target becomes `then_target`.
    MoveAtBreak {
        /// The day-extreme break price.
        trigger: i64,
        /// 1.272 (XD2) or 1.618 (XD3).
        then_target: i64,
    },
    /// XE: a trailing stop armed at `arm`, trailing the best print by
    /// `distance`.
    Trail {
        /// The arming price (the day-extreme break).
        arm: i64,
        /// Trail distance, paise.
        distance: i64,
    },
    /// XG, XH: a market exit decided at this second.
    MarketAt(u32),
    /// The stop alone.
    StopOnly,
}

/// One order through its exit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Plan {
    /// Side.
    pub side: Side,
    /// The decision T.
    pub decided_at: u32,
    /// The walked amount, paise.
    pub amount: i64,
    /// The stop, R1-mapped, paise.
    pub stop: i64,
    /// The exit beside the stop.
    pub exit: Exit,
    /// Latency, seconds (14 in lane S).
    pub lambda: u32,
    /// Volume share, ppm (100,000 = 10% in lane S).
    pub share_ppm: u32,
    /// The measured raw price step, paise (XD's entry-price stop).
    pub step: i64,
    /// The session's last continuous-trading second.
    pub session_end: u32,
    /// The stock-day's GDFL data start.
    pub data_start: u32,
}

/// Why the position ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reason {
    /// No fill: the entry never filled (or was refused).
    NoFill,
    /// The stop.
    Stop,
    /// The target.
    Target,
    /// The market exit (time or condition).
    Market,
    /// The broker's square-off at 15:10.
    SquareOff,
}

impl Reason {
    /// Name for the golden files.
    pub fn name(self) -> &'static str {
        match self {
            Reason::NoFill => "no_fill",
            Reason::Stop => "stop",
            Reason::Target => "target",
            Reason::Market => "market",
            Reason::SquareOff => "square_off",
        }
    }
}

/// One fill.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fill {
    /// Second.
    pub t: u32,
    /// Shares.
    pub qty: i64,
    /// Price, paise (real).
    pub px: i64,
    /// 0 the entry order, 1 the strategy exit order, 2 the broker's.
    pub order: u8,
}

/// A walk's outcome.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WalkOut {
    /// `Ok`, or the refusal that stopped the order before it started.
    pub status: Status,
    /// The entry order's start second.
    pub start: Option<u32>,
    /// The sizing price (the last print before the start), real paise.
    pub sizing_px: Option<i64>,
    /// Shares the entry asked for.
    pub qty: i64,
    /// Every fill in time order.
    pub fills: Vec<Fill>,
    /// Why it ended.
    pub reason: Reason,
    /// The second the entry stopped working, when it did not fill whole.
    pub entry_end: Option<u32>,
    /// Applied stop moves `(effective second, new stop)`, real paise.
    pub moves: Vec<(u32, i64)>,
    /// Flag bits of [`rule`].
    pub flags: u32,
}

impl WalkOut {
    /// Shares bought by the entry (a short: sold).
    pub fn filled(&self) -> i64 {
        self.fills
            .iter()
            .filter(|f| f.order == 0)
            .map(|f| f.qty)
            .sum()
    }

    /// The first fill's second.
    pub fn first_fill(&self) -> Option<u32> {
        self.fills.first().map(|f| f.t)
    }

    /// The last fill's second (the exit's last fill).
    pub fn last_fill(&self) -> Option<u32> {
        self.fills.last().map(|f| f.t)
    }

    /// The flags' names, `|`-joined, or `-`.
    pub fn flag_names(&self) -> String {
        let names: Vec<&str> = rule::NAMES
            .iter()
            .enumerate()
            .filter(|(i, _)| self.flags & (1 << i) != 0)
            .map(|(_, n)| *n)
            .collect();
        if names.is_empty() {
            "-".to_owned()
        } else {
            names.join("|")
        }
    }
}

/// A buy order's protection limit on real prices: LTP x 1.03 floored, or
/// LTP + Rs 1.50 at or below Rs 50.
pub fn buy_limit(ltp: i64) -> i64 {
    if ltp <= 5_000 {
        ltp + 150
    } else {
        i64::try_from(floor_div(i128::from(ltp) * 103, 100)).unwrap_or(i64::MAX)
    }
}

/// A sell order's protection limit on real prices: LTP x 0.97 ceiled, or
/// LTP - Rs 1.50 at or below Rs 50.
pub fn sell_limit(ltp: i64) -> i64 {
    if ltp <= 5_000 {
        ltp - 150
    } else {
        i64::try_from(ceil_div(i128::from(ltp) * 97, 100)).unwrap_or(i64::MIN)
    }
}

/// A queued move of the resting orders, long space.
#[derive(Clone, Copy, Debug)]
struct Move {
    at: u32,
    stop: i64,
    target: Option<i64>,
}

/// The working exit order, long space.
#[derive(Clone, Copy, Debug)]
struct ExitOrder {
    limit: i64,
    reason: Reason,
}

/// The walk's running state, long space.
struct Run {
    qty: i64,
    filled: i64,
    exited: i64,
    bvalue: i128,
    entry_working: bool,
    entry_limit: i64,
    stop: i64,
    target: Option<i64>,
    exit: Option<ExitOrder>,
    pending: VecDeque<Move>,
    xd_armed: bool,
    best: Option<i64>,
    market_done: bool,
    entry_secs: u32,
    exit_secs: u32,
}

impl Run {
    /// The entry's slice in second `s` (long space), from `budget`.
    fn entry_fill(&mut self, s: &Sec, budget: &mut i64, long: bool, out: &mut WalkOut) {
        if !self.entry_working {
            return;
        }
        if s.t >= CUT {
            self.entry_working = false;
            out.entry_end = Some(CUT);
            out.flags |= rule::ENTRY_CUT;
            return;
        }
        if s.h > self.entry_limit {
            out.flags |= rule::PROTECTION_LIMITED;
            return;
        }
        let x = (self.qty - self.filled).min(*budget);
        if x <= 0 {
            return;
        }
        if self.filled == 0 && s.l <= self.stop {
            out.flags |= rule::STOP_THROUGH_AT_FILL;
        }
        self.filled += x;
        self.bvalue += i128::from(x) * i128::from(s.h);
        *budget -= x;
        self.entry_secs += 1;
        out.fills.push(Fill {
            t: s.t,
            qty: x,
            px: if long { s.h } else { -s.h },
            order: 0,
        });
        if self.filled == self.qty {
            self.entry_working = false;
        }
    }

    /// The resting orders' events and the lane's actions in second `s` on a
    /// held quantity: the exit's reason when one starts.
    fn events(
        &mut self,
        s: &Sec,
        plan: &Plan,
        long: bool,
        resting: RestingAfterCut,
        out: &mut WalkOut,
    ) -> Option<Reason> {
        let m = |x: i64| if long { x } else { -x };
        let resting_ok = s.t < CUT || resting == RestingAfterCut::Acts;
        if resting_ok && s.l <= self.stop {
            return Some(Reason::Stop);
        }
        if resting_ok && self.target.is_some_and(|t| s.h >= t) {
            return Some(Reason::Target);
        }
        if let Exit::MarketAt(d) = plan.exit
            && !self.market_done
            && s.t >= d + plan.lambda
        {
            self.market_done = true;
            if s.t >= CUT {
                out.flags |= rule::CUT_REJECTED;
                return None;
            }
            return Some(Reason::Market);
        }
        match plan.exit {
            Exit::MoveAtBreak {
                trigger,
                then_target,
            } if !self.xd_armed && s.h >= m(trigger) => {
                self.xd_armed = true;
                let avg = floor_div(self.bvalue, i128::from(self.filled.max(1)));
                let step = i128::from(plan.step.max(1));
                let entry_px = i64::try_from(floor_div(avg, step) * step).unwrap_or(self.stop);
                self.pending.push_back(Move {
                    at: s.t + 1 + plan.lambda,
                    stop: entry_px,
                    target: Some(m(then_target)),
                });
            }
            Exit::Trail { arm, distance } => {
                let armed_now = self.best.is_none() && s.h >= m(arm);
                if armed_now || self.best.is_some_and(|b| s.h > b) {
                    self.best = Some(s.h);
                    let candidate = s.h - distance;
                    let floor = self.pending.back().map_or(self.stop, |p| p.stop);
                    if candidate > floor {
                        self.pending.push_back(Move {
                            at: s.t + 1 + plan.lambda,
                            stop: candidate,
                            target: None,
                        });
                    }
                }
            }
            _ => {}
        }
        None
    }
}

/// Walks one order. `secs` are the stock-day's seconds, real prices, sorted.
#[allow(
    clippy::too_many_lines,
    reason = "one second-by-second state machine, kept whole so it reads as 11.1"
)]
pub fn walk(secs: &[Sec], plan: &Plan, within: WithinSecond, resting: RestingAfterCut) -> WalkOut {
    let long = plan.side.long();
    let m = |x: i64| if long { x } else { -x };
    let mut out = WalkOut {
        status: Status::Ok,
        start: None,
        sizing_px: None,
        qty: 0,
        fills: Vec::new(),
        reason: Reason::NoFill,
        entry_end: None,
        moves: Vec::new(),
        flags: 0,
    };
    let begin = plan.decided_at + plan.lambda;
    if begin < plan.data_start {
        out.status = Status::NoFillDataWindow;
        return out;
    }
    let Some(start) = crate::bars::first_traded_at_or_after(secs, begin) else {
        return out;
    };
    out.start = Some(start.t);
    let Some(ltp) = last_print_before(secs, start.t) else {
        out.status = Status::NoSizingPrice;
        return out;
    };
    out.sizing_px = Some(ltp);
    let qty =
        i64::try_from(floor_div(i128::from(plan.amount), i128::from(ltp.max(1)))).unwrap_or(0);
    out.qty = qty;
    if qty <= 0 {
        out.status = Status::PriceAbovePosition;
        return out;
    }
    if start.t >= CUT {
        out.entry_end = Some(CUT);
        out.flags |= rule::ENTRY_CUT;
        return out;
    }
    let mut run = Run {
        qty,
        filled: 0,
        exited: 0,
        bvalue: 0,
        entry_working: true,
        entry_limit: if long {
            buy_limit(ltp)
        } else {
            -sell_limit(ltp)
        },
        stop: m(plan.stop),
        target: match plan.exit {
            Exit::Target(t) => Some(m(t)),
            _ => None,
        },
        exit: None,
        pending: VecDeque::new(),
        xd_armed: false,
        best: None,
        market_done: false,
        entry_secs: 0,
        exit_secs: 0,
    };
    let first = first_at_or_after(secs, start.t);
    for raw in &secs[first..] {
        if raw.t >= SQUARE_OFF || (!run.entry_working && run.filled == 0) {
            break;
        }
        if !raw.traded() {
            continue;
        }
        let s = if long { *raw } else { raw.mirror() };
        let mut budget =
            i64::try_from(i128::from(s.v) * i128::from(plan.share_ppm) / 1_000_000).unwrap_or(0);
        while let Some(mv) = run.pending.front().copied() {
            if s.t < mv.at {
                break;
            }
            run.pending.pop_front();
            run.stop = mv.stop;
            if mv.target.is_some() {
                run.target = mv.target;
            }
            out.moves.push((s.t, m(run.stop)));
        }
        let held_before = run.filled - run.exited;
        if within == WithinSecond::EntryFirst {
            run.entry_fill(&s, &mut budget, long, &mut out);
        }
        let held = if within == WithinSecond::EntryFirst {
            run.filled - run.exited
        } else {
            held_before
        };
        if run.exit.is_none() && held > 0 {
            if let Some(reason) = run.events(&s, plan, long, resting, &mut out) {
                let ltp_exit = last_print_before(secs, s.t).unwrap_or(ltp);
                let limit = if long {
                    sell_limit(ltp_exit)
                } else {
                    -buy_limit(ltp_exit)
                };
                run.exit = Some(ExitOrder { limit, reason });
                if run.entry_working {
                    run.entry_working = false;
                    out.entry_end = Some(s.t);
                }
            }
        } else if run.exit.is_none()
            && run.entry_working
            && let Exit::MarketAt(d) = plan.exit
            && s.t >= d + plan.lambda
        {
            // a market exit decided while nothing is held cancels the entry
            // (nothing is held here: the first branch took every held case;
            // and it was never decided before: that cancel ends the loop)
            run.market_done = true;
            run.entry_working = false;
            out.entry_end = Some(s.t);
        }
        if within == WithinSecond::ExitFirst && run.exit.is_none() {
            run.entry_fill(&s, &mut budget, long, &mut out);
        }
        if let Some(ex) = run.exit {
            if s.l < ex.limit {
                out.flags |= rule::PROTECTION_LIMITED;
            } else {
                let x = (run.filled - run.exited).min(budget);
                if x > 0 {
                    run.exited += x;
                    run.exit_secs += 1;
                    out.fills.push(Fill {
                        t: s.t,
                        qty: x,
                        px: m(s.l),
                        order: 1,
                    });
                }
            }
            if run.exited == run.filled {
                out.reason = ex.reason;
                break;
            }
        }
    }
    if run.entry_secs > 1 || run.exit_secs > 1 {
        out.flags |= rule::VOLUME_WALKED;
    }
    if run.filled == 0 {
        out.reason = Reason::NoFill;
        out.fills.clear();
        return out;
    }
    if run.filled < qty {
        out.flags |= rule::PARTIAL;
    }
    if run.exited < run.filled {
        square_off(
            secs,
            plan,
            long,
            start.t,
            ltp,
            run.filled - run.exited,
            &mut out,
        );
    }
    out
}

/// The broker's 15:10 square-off of `open` shares: the remainder rule.
fn square_off(
    secs: &[Sec],
    plan: &Plan,
    long: bool,
    start: u32,
    ltp: i64,
    open: i64,
    out: &mut WalkOut,
) {
    let worst = |from: u32| -> Option<i64> {
        let span = secs[first_at_or_after(secs, from)..]
            .iter()
            .filter(|s| s.traded() && s.t <= plan.session_end);
        if long {
            span.map(|s| s.l).min()
        } else {
            span.map(|s| s.h).max()
        }
    };
    let px = worst(SQUARE_OFF).unwrap_or_else(|| {
        out.flags |= rule::UNCLOSABLE;
        worst(out.first_fill().unwrap_or(start)).unwrap_or(ltp)
    });
    out.flags |= rule::BROKER_SQUARE_OFF;
    out.fills.push(Fill {
        t: SQUARE_OFF,
        qty: open,
        px,
        order: 2,
    });
    out.reason = Reason::SquareOff;
}

#[cfg(test)]
#[path = "walk_tests.rs"]
mod tests;

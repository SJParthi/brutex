//! F28 `signal::ready_gate` and F29 `signal::evaluate` (PREVIEW-SPEC section 3.1).
//!
//! A reading decides only when ready: every reading the rule set uses is warm and
//! untainted, the candle is in the trading span and not a hole. The rule set's
//! conjunction is then read per side; a candle never yields both sides.

use crate::cond::{
    Side, c1_narrow_cpr, c2_beyond_r1_s1, c3_supertrend_side, c4_bollinger_beyond, c5_rsi_beyond, c6_willr_level,
    c7_cci_beyond, c8_ema_side,
};
use crate::dayref::AcceptedBar;
use crate::readings::{Bank, KINDS, Kind};
use crate::ruleset::{Cond, RuleSet};

/// What one candle offers a rule set.
#[derive(Clone, Copy, Debug)]
pub struct Context<'a> {
    /// The stock and timeframe's reading bank, the candle folded.
    pub bank: &'a Bank,
    /// The candle's close.
    pub close: i64,
    /// The previous session's accepted bar, if any.
    pub prev: Option<AcceptedBar>,
    /// The stock-day may trade and the day is inside the trading span.
    pub in_span: bool,
    /// The candle is a hole.
    pub hole: bool,
}

/// Why a candle cannot decide for a rule set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Blocker {
    /// Outside the trading span, or the stock-day may not trade.
    OutsideSpan,
    /// A hole candle.
    Hole,
    /// A reading the rule set uses is not warm.
    Cold(Kind),
    /// A reading the rule set uses is tainted by a hole.
    Tainted(Kind),
}

/// The readings a rule set uses, in table order, each once.
#[must_use]
pub fn readings_used(set: &RuleSet) -> Vec<Kind> {
    KINDS.iter().copied().filter(|k| set.conds.iter().any(|c| c.reading() == Some(*k))).collect()
}

/// F28: every blocker, in order (span, hole, then each used reading cold or tainted);
/// empty when the gate is ready.
#[must_use]
pub fn ready_gate(set: &RuleSet, ctx: &Context<'_>) -> Vec<Blocker> {
    let mut out = Vec::new();
    if !ctx.in_span {
        out.push(Blocker::OutsideSpan);
    }
    if ctx.hole {
        out.push(Blocker::Hole);
    }
    for kind in readings_used(set) {
        if !ctx.bank.warm(kind) {
            out.push(Blocker::Cold(kind));
        } else if !ctx.bank.taint.untainted(kind) {
            out.push(Blocker::Tainted(kind));
        }
    }
    out
}

/// One condition on one side.
#[must_use]
pub fn condition(cond: &Cond, ctx: &Context<'_>, side: Side) -> bool {
    let bank = ctx.bank;
    match *cond {
        Cond::NarrowCpr { bp } => ctx.prev.is_some_and(|p| c1_narrow_cpr(&p, bp)),
        Cond::BeyondR1S1 => ctx.prev.is_some_and(|p| c2_beyond_r1_s1(&p, ctx.close, side)),
        Cond::Supertrend { factor_milli } => {
            let st = if factor_milli == 2_000 { &bank.st2 } else { &bank.st3 };
            c3_supertrend_side(st.reading().as_ref(), side)
        }
        Cond::Bollinger { width_milli } => c4_bollinger_beyond(bank.bb.window().as_ref(), width_milli, side),
        Cond::Rsi { long_ppm, short_ppm } => c5_rsi_beyond(bank.rsi.reading().as_ref(), long_ppm, short_ppm, side),
        Cond::WilliamsR { long_ppm, short_ppm } => {
            c6_willr_level(bank.wr.reading().as_ref(), long_ppm, short_ppm, side)
        }
        Cond::Cci { long_milli, short_milli } => {
            c7_cci_beyond(bank.cci.reading().as_ref(), long_milli, short_milli, side)
        }
        Cond::Ema => c8_ema_side(&bank.ema, ctx.close, side),
    }
}

/// F29: the side whose every condition holds, when the gate is ready (long first; the
/// conditions make both impossible).
#[must_use]
pub fn evaluate(set: &RuleSet, ctx: &Context<'_>) -> Option<Side> {
    if !ready_gate(set, ctx).is_empty() {
        return None;
    }
    Side::BOTH.into_iter().find(|side| set.conds.iter().all(|c| condition(c, ctx, *side)))
}

#[cfg(test)]
#[path = "signal_tests.rs"]
mod tests;

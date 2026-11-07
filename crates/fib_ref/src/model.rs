//! Shared vocabulary: sides, the named statuses and refusals, the readings
//! this crate pins where the kit leaves a choice, and R1's level mapping.

use crate::bars::{Minute, last_minute_closed_by};
use crate::ladder::{ExtremeTies, TaintScope};
use crate::money::{ceil_div, floor_div};
use crate::rank::MoverSign;

/// The side a signal trades.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Side {
    /// Buy then sell; gainer ranks.
    Long,
    /// Sell then buy; loser ranks.
    Short,
}

impl Side {
    /// Short name for the golden files.
    pub fn name(self) -> &'static str {
        match self {
            Side::Long => "long",
            Side::Short => "short",
        }
    }

    /// `true` for a long.
    pub fn long(self) -> bool {
        self == Side::Long
    }

    /// The mirror side.
    #[must_use]
    pub fn mirror(self) -> Side {
        match self {
            Side::Long => Side::Short,
            Side::Short => Side::Long,
        }
    }
}

/// Every named status a rule can end in. A signal fires only at `Ok`; every
/// other value is a refusal by name (the kit's words where the kit names one;
/// a clause's name where the kit names a clause and no refusal).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Status {
    /// Every clause holds.
    Ok,
    /// A candle holding a hole minute: no decision on it (3.4).
    IncompleteCandle,
    /// The ladder at the decision is tainted for the session (3.4).
    TaintedLadder,
    /// The ladder at the decision is Undetermined (FIB-I3; a clause).
    LadderUndetermined,
    /// The holding rule S33 fails (a clause).
    Holding,
    /// The box height clause fails (a clause).
    Height,
    /// The colour clause S37 fails (a clause).
    Colour,
    /// Not inside top N at the decision (S11/S12; a clause).
    NotInTopN,
    /// The touch-number clause S27 fails (a clause).
    TouchNumber,
    /// A hole or incomplete candle ended the touch count (7.2 item 6).
    TouchCountUnknown,
    /// The order's start would be before the stock-day's GDFL data start.
    NoFillDataWindow,
    /// R1's minute has no price for a trigger (7.2 item 9).
    NoTriggerRatio,
    /// R1's minute has no price for a stop or target (6.1, DV-29).
    NoLevelRatio,
    /// No touch before the second reading's arming (7.2 item 10).
    NoPatternExtreme,
    /// The resting reading is refused by name (7.2 item 8).
    TouchOrderUnsourced,
    /// A hole or padded-traded minute inside the opening range (25.2.1).
    OrbTainted,
    /// The opening range has no width (25.2.1).
    OrbNoWidth,
    /// A rung target with no usable ladder at the decision (25.2.1).
    NoLadderTarget,
    /// The price is at or through the stop at the decision (24.1).
    SnStopThroughAtDecision,
    /// REF is above the walk's amount: no whole share (24.13).
    SnNoQuantity,
    /// A lane with no price at the decision for S-RR (24.1).
    SRrNoTarget,
    /// The no-stop comparison lane refuses both gates (24.1).
    SnNoStop,
    /// Decision after S42's 15:07:59 (9.2).
    AfterLastEntry,
    /// The 6th signal of a session (REQ 25).
    DayCap,
    /// The 101st signal of a calendar month (REQ 25).
    MonthCap,
    /// No walked amount's margin fits the ceiling, or the margin check fails.
    MarginCap,
    /// The exposure check fails (10.3).
    ExposureCap,
    /// Below the half step (10.2, 25.1).
    SizeRefused,
    /// One position per stock: the stock is locked (10.4).
    WhileOpen,
    /// All N slots are taken (10.4; the kit names no refusal for it, this
    /// crate's name).
    SlotsFull,
    /// The sizing price is above the amount: zero shares (10.2).
    PriceAbovePosition,
    /// No traded print before the order's start to size it (this crate's
    /// name; the kit names none).
    NoSizingPrice,
    /// One option lot costs more than the premium amount (25.2.5).
    LotAbovePremium,
    /// The S-STOP band clause fails (a clause).
    StopBand,
    /// The S-RR clause fails (a clause).
    RrBand,
    /// The opening-range break ordinal clause fails (a clause).
    BreakOrdinal,
}

impl Status {
    /// The name printed in the golden files.
    pub fn name(self) -> &'static str {
        match self {
            Status::Ok => "ok",
            Status::IncompleteCandle => "incomplete_candle",
            Status::TaintedLadder => "tainted_ladder",
            Status::LadderUndetermined => "ladder_undetermined",
            Status::Holding => "holding",
            Status::Height => "height",
            Status::Colour => "colour",
            Status::NotInTopN => "not_in_top_n",
            Status::TouchNumber => "touch_number",
            Status::TouchCountUnknown => "touch_count_unknown",
            Status::NoFillDataWindow => "no_fill_data_window",
            Status::NoTriggerRatio => "no_trigger_ratio",
            Status::NoLevelRatio => "no_level_ratio",
            Status::NoPatternExtreme => "no_pattern_extreme",
            Status::TouchOrderUnsourced => "touch_order_unsourced",
            Status::OrbTainted => "orb_tainted",
            Status::OrbNoWidth => "orb_no_width",
            Status::NoLadderTarget => "no_ladder_target",
            Status::SnStopThroughAtDecision => "sn_stop_through_at_decision",
            Status::SnNoQuantity => "sn_no_quantity",
            Status::SRrNoTarget => "s_rr_no_target",
            Status::SnNoStop => "sn_no_stop",
            Status::AfterLastEntry => "after_last_entry",
            Status::DayCap => "day cap",
            Status::MonthCap => "month cap",
            Status::MarginCap => "margin_cap",
            Status::ExposureCap => "exposure_cap",
            Status::SizeRefused => "size_refused",
            Status::WhileOpen => "while_open",
            Status::SlotsFull => "slots_full",
            Status::PriceAbovePosition => "price_above_position",
            Status::NoSizingPrice => "no_sizing_price",
            Status::LotAbovePremium => "lot_above_premium",
            Status::StopBand => "stop_band",
            Status::RrBand => "rr_band",
            Status::BreakOrdinal => "break_ordinal",
        }
    }

    /// Every status, in declaration order (for the coverage of names).
    pub const ALL: [Status; 36] = [
        Status::Ok,
        Status::IncompleteCandle,
        Status::TaintedLadder,
        Status::LadderUndetermined,
        Status::Holding,
        Status::Height,
        Status::Colour,
        Status::NotInTopN,
        Status::TouchNumber,
        Status::TouchCountUnknown,
        Status::NoFillDataWindow,
        Status::NoTriggerRatio,
        Status::NoLevelRatio,
        Status::NoPatternExtreme,
        Status::TouchOrderUnsourced,
        Status::OrbTainted,
        Status::OrbNoWidth,
        Status::NoLadderTarget,
        Status::SnStopThroughAtDecision,
        Status::SnNoQuantity,
        Status::SRrNoTarget,
        Status::SnNoStop,
        Status::AfterLastEntry,
        Status::DayCap,
        Status::MonthCap,
        Status::MarginCap,
        Status::ExposureCap,
        Status::SizeRefused,
        Status::WhileOpen,
        Status::SlotsFull,
        Status::PriceAbovePosition,
        Status::NoSizingPrice,
        Status::LotAbovePremium,
        Status::StopBand,
        Status::RrBand,
        Status::BreakOrdinal,
    ];
}

/// E1 after a break or a void: what re-arms the box machine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AfterBreak {
    /// Consumed until the next new session extreme (headline).
    UntilNewExtreme,
    /// Re-arms on the next box of K candles after the break candle.
    NextBox,
}

/// Where E1's box sits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoxAnchor {
    /// The K candles right before the break candidate, starting no earlier
    /// than the candle after the push extreme (headline).
    Sliding,
    /// The K candles right after the push extreme; the break is the first
    /// later candle closing beyond it.
    Anchored,
}

/// E2: a touch while Away whose bounce has not passed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Retouch {
    /// It joins the current touch; the next gap counts from its last candle
    /// (headline).
    Merge,
    /// It is not a touch at all for the count; the gap counts from the
    /// counted touch's last candle.
    Ignore,
}

/// E3: a candle closing exactly at the range's close.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrbBoundary {
    /// It writes no record: "on or before the range's close" (headline).
    NoRecord,
    /// It may arm the second reading: "at or after the range's close".
    Arms,
}

/// Within one second, the entry's fill against the stop and target check.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WithinSecond {
    /// The entry fills first, then the stop and target read the held
    /// quantity, then the exit fills from what is left of the second's
    /// budget (headline; it is what lets a stop be through at the first fill).
    EntryFirst,
    /// Stop and target read only what was held before the second.
    ExitFirst,
}

/// A resting stop or target that prints between 15:08:00 and 15:10:00.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RestingAfterCut {
    /// It acts: it rests at the broker and is not a new order (headline).
    Acts,
    /// Its exit starts after 15:08:00 and is rejected; the broker squares off.
    Rejected,
}

/// A zero-fill entry's day and month count.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ZeroFillCount {
    /// Released at its cancel: it held the count "while it lives" (headline).
    Released,
    /// Kept: counted at entry for good.
    Kept,
}

/// An amount below the half step reached by the leverage quantisation alone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BelowHalf {
    /// Allowed: any of the five walked amounts (headline).
    Allowed,
    /// Refused by name, `size_refused`.
    Refused,
}

/// Max drawdown's unit, tenths of a percent of Rs 20 lakh.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DdRound {
    /// Rounded down (headline).
    Floor,
    /// Rounded up.
    Ceil,
}

/// When the month's data charge enters the closed-equity curve.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DataChargeAt {
    /// At the month's first fill (headline).
    FirstFill,
    /// Kept out of the curve and the drawdown; still in the month's net.
    OutOfCurve,
}

/// The pre-2020-07-01 state stamp duty's "worst listed state".
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StateStamp {
    /// Telangana up to Rs 16.67 lakh of turnover a day, Tamil Nadu above it
    /// (the words of COSTS.md section 3; headline).
    Stated,
    /// The highest of every listed state, Himachal Pradesh's flat Rs 50 a day
    /// included.
    Literal,
}

/// Brokerage on an option order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OptionBrokerage {
    /// Rs 20 an order (REQ 26's words; headline).
    Flat20,
    /// `min(Rs 20, 0.03%)`, the equity rule.
    Min20,
}

/// Which ladder prices a rung target at the decision.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TargetLadder {
    /// The ladder after the decision candle folds (headline: every candle
    /// read has closed by the decision).
    AfterFold,
    /// The ladder the decision candle was judged against.
    Judging,
}

/// Where a lock or slot ends.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LockEnd {
    /// The exit's last fill second still holds it (headline).
    Inclusive,
    /// It is free in the exit's last fill second.
    Exclusive,
}

/// An interim report's line 2.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InterimLine {
    /// Always PENDING (headline).
    AlwaysPending,
    /// PENDING only where the verdict would be YES; a NO prints.
    NoCanPrint,
}

/// Where a timeframe's candle grid starts (A04): the spec gives a candle
/// labelled t as [t, t + tf), closing at min(t + tf, session end), and no
/// origin for the labels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CandleGrid {
    /// Labels from the session's first minute, 09:15, 09:15 + tf, ... (headline).
    Session,
    /// Labels on the clock, multiples of tf minutes from midnight: the first
    /// candle runs from 09:15 to the first such multiple.
    Clock,
}

/// The verdict's rules 5 and 6 when both could speak (A20): twin seeds 2
/// and 3 not all run, and one that ran at or above the headline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuleOrder {
    /// Rule 5 (fewer than three twins: PENDING) before rule 6 (headline).
    FiveFirst,
    /// Rule 6 (a twin 2 or 3 at or above: NO) before rule 5.
    SixFirst,
}

/// Every reading this crate pins. `HEADLINE` holds the headline reading of
/// each; a scenario that runs another prints it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Readings {
    /// A02.
    pub ties: ExtremeTies,
    /// A04.
    pub grid: CandleGrid,
    /// A03.
    pub taint: TaintScope,
    /// A01.
    pub mover_sign: MoverSign,
    /// A05.
    pub after_break: AfterBreak,
    /// A06.
    pub box_anchor: BoxAnchor,
    /// A07.
    pub retouch: Retouch,
    /// A08.
    pub orb_boundary: OrbBoundary,
    /// A09.
    pub within_second: WithinSecond,
    /// A10.
    pub resting_after_cut: RestingAfterCut,
    /// A11.
    pub zero_fill_count: ZeroFillCount,
    /// A12.
    pub below_half: BelowHalf,
    /// A13.
    pub dd_round: DdRound,
    /// A14.
    pub data_charge_at: DataChargeAt,
    /// A15.
    pub state_stamp: StateStamp,
    /// A16.
    pub option_brokerage: OptionBrokerage,
    /// A17.
    pub target_ladder: TargetLadder,
    /// A18.
    pub lock_end: LockEnd,
    /// A19.
    pub interim: InterimLine,
    /// A20.
    pub rule_order: RuleOrder,
}

impl Readings {
    /// The headline reading of every choice.
    pub const HEADLINE: Readings = Readings {
        ties: ExtremeTies::Strict,
        grid: CandleGrid::Session,
        taint: TaintScope::AnyFolded,
        mover_sign: MoverSign::RankOnly,
        after_break: AfterBreak::UntilNewExtreme,
        box_anchor: BoxAnchor::Sliding,
        retouch: Retouch::Merge,
        orb_boundary: OrbBoundary::NoRecord,
        within_second: WithinSecond::EntryFirst,
        resting_after_cut: RestingAfterCut::Acts,
        zero_fill_count: ZeroFillCount::Released,
        below_half: BelowHalf::Allowed,
        dd_round: DdRound::Floor,
        data_charge_at: DataChargeAt::FirstFill,
        state_stamp: StateStamp::Stated,
        option_brokerage: OptionBrokerage::Flat20,
        target_ladder: TargetLadder::AfterFold,
        lock_end: LockEnd::Inclusive,
        interim: InterimLine::AlwaysPending,
        rule_order: RuleOrder::FiveFirst,
    };

    /// The non-headline readings, as `Axx=name` tokens, for a scenario's
    /// head line; `headline` when every reading is the headline's.
    pub fn label(&self) -> String {
        let h = Readings::HEADLINE;
        let mut out = Vec::new();
        let mut push = |on: bool, s: &str| {
            if on {
                out.push(s.to_owned());
            }
        };
        push(self.mover_sign != h.mover_sign, "A01=signed_only");
        push(self.ties != h.ties, "A02=latest");
        push(self.taint != h.taint, "A03=extreme_candles");
        push(self.grid != h.grid, "A04=clock");
        push(self.after_break != h.after_break, "A05=next_box");
        push(self.box_anchor != h.box_anchor, "A06=anchored");
        push(self.retouch != h.retouch, "A07=ignore");
        push(self.orb_boundary != h.orb_boundary, "A08=arms");
        push(self.within_second != h.within_second, "A09=exit_first");
        push(
            self.resting_after_cut != h.resting_after_cut,
            "A10=rejected",
        );
        push(self.zero_fill_count != h.zero_fill_count, "A11=kept");
        push(self.below_half != h.below_half, "A12=refused");
        push(self.dd_round != h.dd_round, "A13=ceil");
        push(self.data_charge_at != h.data_charge_at, "A14=out_of_curve");
        push(self.state_stamp != h.state_stamp, "A15=literal");
        push(self.option_brokerage != h.option_brokerage, "A16=min20");
        push(self.target_ladder != h.target_ladder, "A17=judging");
        push(self.lock_end != h.lock_end, "A18=exclusive");
        push(self.interim != h.interim, "A19=no_can_print");
        push(self.rule_order != h.rule_order, "A20=six_first");
        if out.is_empty() {
            "headline".to_owned()
        } else {
            out.join(",")
        }
    }
}

/// R1 at a decision `t`: `(Z, G)` of the last 1-minute candle closed by `t`,
/// or `None` when that minute has no price of its own.
pub fn r1_at(minutes: &[Minute], t: u32) -> Option<(i64, i64)> {
    let m = last_minute_closed_by(t)?;
    minutes.get(usize::try_from(m).unwrap_or(usize::MAX))?.r1()
}

/// Rounding of a mapped level to the measured raw step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Toward {
    /// Down to the step.
    Down,
    /// Up to the step.
    Up,
}

/// R1's mapping of a Zerodha level (milli-paise) to a GDFL price in paise,
/// `L_g = L_z x G / Z`, rounded `toward` the measured raw `step` (paise).
/// "Against the position": a long's stop and a short's target round down, a
/// long's target and a short's stop round up (PREVIEW-SPEC 6.1).
pub fn map_level(level_milli: i128, z: i64, g: i64, step: i64, toward: Toward) -> i64 {
    let num = level_milli * i128::from(g);
    let den = i128::from(z) * 1000 * i128::from(step.max(1));
    let steps = match toward {
        Toward::Down => floor_div(num, den),
        Toward::Up => ceil_div(num, den),
    };
    i64::try_from(steps * i128::from(step.max(1))).unwrap_or(i64::MAX)
}

/// The rounding of a stop on `side`: against the position.
pub fn stop_toward(side: Side) -> Toward {
    if side.long() {
        Toward::Down
    } else {
        Toward::Up
    }
}

/// The rounding of a target on `side`: against the position.
pub fn target_toward(side: Side) -> Toward {
    if side.long() {
        Toward::Up
    } else {
        Toward::Down
    }
}

#[cfg(test)]
#[path = "model_tests.rs"]
mod tests;

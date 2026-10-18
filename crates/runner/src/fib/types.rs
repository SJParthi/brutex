//! Shared value types of the Fib walker, lattice and gate, with no behaviour
//! (FIB-1; D-6300).
//!
//! APPEND-ONLY. Later packages only ADD fields and variants, in dependency
//! order (FIB-5 first: the walker's readings, the MFE and last-peak seconds,
//! the record strides). No type here has a `Default`: every value is named and
//! recorded in the run identity. Prices are paisa (`i64`, `i128` for a
//! product); a statistic is never stored here.

use vocab::fib_cube::{TRADE_SIZE_LARGE_RUPEES, TRADE_SIZE_SMALL_RUPEES};

/// Which bars a walk reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Resolution {
    /// The Zerodha one-minute bars of the day (stage A).
    Minute,
    /// The GDFL one-second bars of the continuous phase (stage DEEP).
    Second,
}

/// How a decision-basis (Zerodha) level maps to the execution feed's raw
/// prices.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PriceMap {
    /// The same basis: stage A.
    Identity,
    /// DATA-1's per-stock-day factor pair: a raw price `g` is compared with a
    /// level `z` as `g x zerodha_close` against `z x gdfl_close`, so no level
    /// is ever rounded.
    Factor {
        /// The Zerodha close of the pair, in paisa.
        zerodha_close: i64,
        /// The GDFL close of the pair, in paisa.
        gdfl_close: i64,
    },
}

/// Every named reading of one walk. No `Default`: FIB-5 adds each reading the
/// walker needs (the exit fill, the snapshot edge and the rest), each with no
/// default either.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct WalkProfile {
    /// Which bars the walk reads.
    pub resolution: Resolution,
    /// How levels map to the bars' prices.
    pub price_map: PriceMap,
    /// How a stop, target, trail or time exit leaves a one-second bar
    /// (FM-F035; FIB-5). Minute bars always exit the whole quantity.
    pub exit_fill: ExitFill,
    /// What a GDFL second's stamp means (FM-F043; FIB-5).
    pub snapshot_edge: SnapshotEdge,
    /// What an open position does when a recorded halt starts (FIB-15 RQ-5;
    /// FIB-5).
    pub halt_rule: HaltRule,
    /// Whether a converted Zerodha minute may touch a deep stop (FM-F046;
    /// FIB-5).
    pub cross_feed: CrossFeed,
    /// What a fill at or beyond the frozen target does to the take-profit
    /// (FM-F038; FIB-5).
    pub beyond_target: BeyondTarget,
    /// What a measured non-zero GDFL clock lag does (FM-F043; FIB-5).
    pub clock_rule: ClockRule,
}

impl WalkProfile {
    /// The profile's readings as identity terms, in this fixed order:
    /// resolution, exit fill, snapshot edge, halt rule, cross feed, beyond
    /// target, clock rule. The price map is not a term: it is DATA-1's
    /// per-stock-day authority, a leaf of the data tree.
    #[must_use]
    pub const fn identity_terms(&self) -> [u64; WALK_PROFILE_TERMS] {
        [
            self.resolution as u64,
            self.exit_fill as u64,
            self.snapshot_edge as u64,
            self.halt_rule as u64,
            self.cross_feed as u64,
            self.beyond_target as u64,
            self.clock_rule as u64,
        ]
    }
}

/// How many identity terms a [`WalkProfile`] contributes.
pub const WALK_PROFILE_TERMS: usize = [
    "resolution",
    "exit_fill",
    "snapshot_edge",
    "halt_rule",
    "cross_feed",
    "beyond_target",
    "clock_rule",
]
.len();

/// How an exit order leaves a one-second bar (FM-F035). Both readings run;
/// neither is a default.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ExitFill {
    /// The touching second fills `min(remaining, volume)` at its worst price
    /// and the remainder continues second by second at each second's worst
    /// price (the operator's FILLS rule).
    SpillByVolume,
    /// The whole quantity leaves at the touching second's worst price: a
    /// labelled comparison.
    FirstTouchWhole,
}

impl ExitFill {
    /// The reading's canonical name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::SpillByVolume => "spill_by_volume",
            Self::FirstTouchWhole => "first_touch_whole",
        }
    }
}

/// What a GDFL second's stamp means (FM-F043). PREC-11 sources the edge; until
/// then both readings run under their own identities.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SnapshotEdge {
    /// A second stamped `s` covers `[s, s + 1)`: the second stamped at the
    /// decision instant is after it.
    StampOpensInterval,
    /// A second stamped `s` covers `(s - 1, s]`: the second stamped at the
    /// decision instant is before it and never fills.
    StampClosesInterval,
}

impl SnapshotEdge {
    /// The reading's canonical name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::StampOpensInterval => "stamp_opens_interval",
            Self::StampClosesInterval => "stamp_closes_interval",
        }
    }
}

/// What an open position does when a recorded halt starts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HaltRule {
    /// FIB-15 RQ-5, section 4.10, the deciding rule: an entry before the halt
    /// start stays in the record, unpriced, and is priced `AT_WORST_SEEN` through
    /// the halt (FM-F078); an entry after the halt start is not tradable.
    WorstSeenThroughHalt,
    /// The 17:15 IST item (6) reading, labelled: the rules resume at the first
    /// real print after the halt, and a gapped stop fills at that print's
    /// worst price.
    ResumeAtFirstPrint,
}

impl HaltRule {
    /// The rule's canonical name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::WorstSeenThroughHalt => "worst_seen_through_halt",
            Self::ResumeAtFirstPrint => "resume_at_first_print",
        }
    }
}

/// Whether a converted Zerodha minute may touch a deep stop (FM-F046).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CrossFeed {
    /// Only GDFL seconds touch a level.
    SecondsOnly,
    /// A stop touched only by the converted minute also exits (PREC-15's
    /// `runner::cross_feed`, called, never copied). Refused by name until that
    /// module exists.
    WorseOfBoth,
}

impl CrossFeed {
    /// The reading's canonical name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::SecondsOnly => "seconds_only",
            Self::WorseOfBoth => "worse_of_both",
        }
    }
}

/// What a fill at or beyond the frozen target does (FM-F038). The deciding
/// reading is the operator's choice; both run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BeyondTarget {
    /// The target fires by its own touch rule on the first bar.
    FiresOnFirstBar,
    /// The take-profit is void and the slot walks on its other components.
    TargetVoid,
}

impl BeyondTarget {
    /// The reading's canonical name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::FiresOnFirstBar => "fires_on_first_bar",
            Self::TargetVoid => "target_void",
        }
    }
}

/// What a measured non-zero GDFL clock lag does to a deep walk (FM-F043).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ClockRule {
    /// A stock-day with a non-zero lag is refused `clock_shifted:<L>`.
    RefuseShifted,
    /// The recorded re-stamp: fills start at `c + max(0, L)`.
    Restamp,
}

impl ClockRule {
    /// The rule's canonical name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::RefuseShifted => "refuse_shifted",
            Self::Restamp => "restamp",
        }
    }
}

/// One exit variant of the run's generated exit set (FIB-5): the set's version
/// and the variant's value index in the exit row.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ExitRule {
    /// The exit set's version.
    pub exit_set_version: u32,
    /// The variant, by its value index in the exit row.
    pub variant: u16,
}

/// A trade size the Winner Gate is judged at (`vocab::fib_cube::FIXED_NUMBERS`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TradeSize {
    /// Rs 1,00,000.
    Small,
    /// Rs 5,00,000.
    Large,
}

impl TradeSize {
    /// The size in rupees, from the fixed-number ledger.
    #[must_use]
    pub const fn rupees(self) -> u64 {
        match self {
            Self::Small => TRADE_SIZE_SMALL_RUPEES,
            Self::Large => TRADE_SIZE_LARGE_RUPEES,
        }
    }
}

/// One outcome slot of a decision: an exit variant at a trade size.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SlotKey {
    /// The exit variant.
    pub exit: ExitRule,
    /// The trade size.
    pub size: TradeSize,
}

/// Why a trade ended. The declaration order IS the fixed, append-only
/// precedence (SL > TSL > TTP > TP > time > forced): when several fire in one
/// bar the earliest variant here is the reason, and every fired one is flagged.
/// The own exits 15:07:00 and 15:08:00 are gone (17:15 IST): a time exit is a
/// generated variant.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ExitReason {
    /// The stop loss.
    StopLoss,
    /// The trailing stop loss.
    TrailingStopLoss,
    /// The trailing take-profit.
    TrailingTakeProfit,
    /// The take-profit.
    TakeProfit,
    /// A generated time exit.
    Time,
    /// The forced exit in `[15:09:00, 15:10:00)`.
    Forced,
}

impl ExitReason {
    /// The reason's canonical name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::StopLoss => "stop_loss",
            Self::TrailingStopLoss => "trailing_stop_loss",
            Self::TrailingTakeProfit => "trailing_take_profit",
            Self::TakeProfit => "take_profit",
            Self::Time => "time",
            Self::Forced => "forced",
        }
    }
}

/// One flag a trade can carry. The declaration order is the bit position,
/// append-only.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TradeFlag {
    /// The bar gapped through the stop: R below minus one.
    GapThroughStop,
    /// A target was booked at the bar's worst price.
    TargetBookedAtWorst,
    /// The exit was Dhan's auto square-off (its fee applies).
    DhanSquareOff,
    /// A gap between real seconds inside the trade longer than every gap of
    /// the stock's committed past.
    SilenceInTrade,
    /// The entry window closed with part of the quantity unfilled; the rest
    /// was cancelled (FIB-5).
    PartialEntry,
    /// An exit fired while the entry was still spilling; the unfilled
    /// remainder was cancelled in that bar (FM-F036).
    EntryCancelledByExit,
    /// An entry filled at or beyond the stop and was stopped in that bar
    /// (FM-F033). Kept in statistics.
    EntryBeyondStop,
    /// The average fill was at or beyond the frozen target (FM-F038).
    EntryBeyondTarget,
    /// Part of an exit order was still open at 15:09:00 and went to the forced
    /// rules (FM-F035).
    ExitSpillToForced,
    /// A fill or exit at the first print after a recorded halt (17:15 item 6).
    CircuitHalt,
    /// A fill at a print PREC-3 flagged (FM-F047). Kept, counted.
    FlaggedPrintFill,
    /// A fill outside the same minute's converted Zerodha range (FM-F047).
    CrossFeedOutlier,
    /// A level touched only by a GDFL second beyond the converted minute's
    /// range (FM-F041).
    TouchUnwitnessed,
    /// A stop touched only by the converted Zerodha minute (FM-F046).
    CrossFeedTouch,
    /// A second whose needed book side was empty supplied nothing (the
    /// one-sided-book rule, FIB-15 RQ-6).
    OneSidedBook,
    /// The position was open when a recorded halt started (FIB-15 RQ-5).
    Halt,
    /// A stage-A minute with high == low at the running day extreme was
    /// skipped as a suspected band lock (FM-F042, UNVERIFIED proxy).
    LockedMinuteSuspected,
    /// A swing was refused because a neighbour candle was withheld (FM-F045).
    TrailHole,
    /// The stop loss fired in the exit bar (FM-F037: every fired component is
    /// flagged).
    FiredStopLoss,
    /// The trailing stop fired in the exit bar.
    FiredTrailingStopLoss,
    /// The trailing take-profit fired in the exit bar.
    FiredTrailingTakeProfit,
    /// The take-profit fired in the exit bar.
    FiredTakeProfit,
    /// The time exit fired in the exit bar.
    FiredTime,
}

impl TradeFlag {
    /// Every flag, in bit order.
    pub const ALL: &[Self] = &[
        Self::GapThroughStop,
        Self::TargetBookedAtWorst,
        Self::DhanSquareOff,
        Self::SilenceInTrade,
        Self::PartialEntry,
        Self::EntryCancelledByExit,
        Self::EntryBeyondStop,
        Self::EntryBeyondTarget,
        Self::ExitSpillToForced,
        Self::CircuitHalt,
        Self::FlaggedPrintFill,
        Self::CrossFeedOutlier,
        Self::TouchUnwitnessed,
        Self::CrossFeedTouch,
        Self::OneSidedBook,
        Self::Halt,
        Self::LockedMinuteSuspected,
        Self::TrailHole,
        Self::FiredStopLoss,
        Self::FiredTrailingStopLoss,
        Self::FiredTrailingTakeProfit,
        Self::FiredTakeProfit,
        Self::FiredTime,
    ];

    /// The flag's canonical name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::GapThroughStop => "gap_through_stop",
            Self::TargetBookedAtWorst => "target_booked_at_worst",
            Self::DhanSquareOff => "dhan_square_off",
            Self::SilenceInTrade => "silence_in_trade",
            Self::PartialEntry => "partial_entry",
            Self::EntryCancelledByExit => "entry_cancelled_by_exit",
            Self::EntryBeyondStop => "entry_beyond_stop",
            Self::EntryBeyondTarget => "entry_beyond_target",
            Self::ExitSpillToForced => "exit_spill_to_forced",
            Self::CircuitHalt => "circuit_halt",
            Self::FlaggedPrintFill => "flagged_print_fill",
            Self::CrossFeedOutlier => "cross_feed_outlier",
            Self::TouchUnwitnessed => "touch_unwitnessed",
            Self::CrossFeedTouch => "cross_feed_touch",
            Self::OneSidedBook => "one_sided_book",
            Self::Halt => "halt",
            Self::LockedMinuteSuspected => "locked_minute_suspected",
            Self::TrailHole => "trail_hole",
            Self::FiredStopLoss => "fired_stop_loss",
            Self::FiredTrailingStopLoss => "fired_trailing_stop_loss",
            Self::FiredTrailingTakeProfit => "fired_trailing_take_profit",
            Self::FiredTakeProfit => "fired_take_profit",
            Self::FiredTime => "fired_time",
        }
    }

    /// The flag a fired exit component sets (FM-F037). The forced exit is not
    /// a component: it is always last and carries [`Self::DhanSquareOff`].
    #[must_use]
    pub const fn fired(reason: ExitReason) -> Option<Self> {
        match reason {
            ExitReason::StopLoss => Some(Self::FiredStopLoss),
            ExitReason::TrailingStopLoss => Some(Self::FiredTrailingStopLoss),
            ExitReason::TrailingTakeProfit => Some(Self::FiredTrailingTakeProfit),
            ExitReason::TakeProfit => Some(Self::FiredTakeProfit),
            ExitReason::Time => Some(Self::FiredTime),
            ExitReason::Forced => None,
        }
    }
}

/// What became of a slot (FIB-5). APPEND-ONLY: a new status or reason is a new
/// variant at the end. Only [`Self::Priced`] enters statistics as a fill;
/// every other status is counted and shown by name, and FIB-7's `AT_WORST_SEEN`
/// reading (FM-F078) prices the entered-but-unpriced ones from their MAE to
/// 15:09:59.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SlotStatus {
    /// Entered and exited at real prices; [`SlotOutcome::exit_reason`] says
    /// how.
    Priced,
    /// Nothing filled.
    EntryUnfilled(UnfilledReason),
    /// Something filled and the exit could not be priced.
    Unpriceable(UnpricedReason),
    /// The slot was refused before any bar was read.
    Refused(SlotRefusal),
    /// The plan's entry falls after a recorded halt start (FIB-15 RQ-5).
    NotTradable,
}

/// Why nothing filled. APPEND-ONLY.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum UnfilledReason {
    /// The entry window closed with nothing filled.
    WindowExpired,
    /// The target was touched before the first fill (FM-F036).
    TargetBeforeFill,
    /// The window ran over a GDFL feed hole (FM-F041).
    FeedHole,
    /// A BREAK or RETEST plan was never triggered inside its window.
    NeverTriggered,
}

/// Why a filled slot could not be priced. APPEND-ONLY.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum UnpricedReason {
    /// No GDFL second in `[15:09:00, 15:10:00)`, or too little volume in it
    /// (FM-F048 `unpriceable_thin`).
    ForcedThin,
    /// The forced window had prints but no bid for a long to sell into
    /// (FM-F048).
    ForcedLockedLong,
    /// The forced window had prints but no ask for a short to buy from
    /// (FM-F048).
    ForcedLockedShort,
    /// Stage A: no 15:09 Zerodha minute, or a locked one
    /// (`forced_exit_unpriceable_cheap`).
    ForcedCheap,
    /// A silence over a traded Zerodha minute inside the trade (FM-F041).
    ExecutionFeedHole,
    /// A zero or inverted bar inside the entry window or the open trade
    /// (FM-F044).
    CorruptBarInTrade,
    /// The position was open when a recorded halt started, under
    /// [`HaltRule::WorstSeenThroughHalt`].
    Halt,
}

/// Why a slot was refused before any bar was read. APPEND-ONLY.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SlotRefusal {
    /// The size buys less than one share at the entry reference (FM-F034).
    SizeBelowOneShare,
    /// The entry reference is not beyond the planned stop (FM-F033).
    NonpositivePlannedRisk,
}

impl SlotStatus {
    /// The status's canonical name, with its reason.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Priced => "priced",
            Self::EntryUnfilled(why) => match why {
                UnfilledReason::WindowExpired => "entry_unfilled",
                UnfilledReason::TargetBeforeFill => "target_before_fill",
                UnfilledReason::FeedHole => "entry_unfilled_feed_hole",
                UnfilledReason::NeverTriggered => "entry_never_triggered",
            },
            Self::Unpriceable(why) => match why {
                UnpricedReason::ForcedThin => "unpriceable_thin",
                UnpricedReason::ForcedLockedLong => "unpriceable_locked_long",
                UnpricedReason::ForcedLockedShort => "unpriceable_locked_short",
                UnpricedReason::ForcedCheap => "forced_exit_unpriceable_cheap",
                UnpricedReason::ExecutionFeedHole => "execution_feed_hole",
                UnpricedReason::CorruptBarInTrade => "corrupt_bar_in_trade",
                UnpricedReason::Halt => "halt_worst_seen",
            },
            Self::Refused(why) => match why {
                SlotRefusal::SizeBelowOneShare => "size_below_one_share",
                SlotRefusal::NonpositivePlannedRisk => "nonpositive_planned_risk",
            },
            Self::NotTradable => "not_tradable_after_halt_start",
        }
    }

    /// Whether something filled: an entered slot, priced or not.
    #[must_use]
    pub const fn entered(self) -> bool {
        matches!(self, Self::Priced | Self::Unpriceable(_))
    }
}

/// One exit order of a slot (FM-F125): its own exit or stop (a spill inside it
/// stays one order), or the forced remainder. In exact integers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ExitOrder {
    /// The order's last fill second after IST midnight.
    pub last_fill_second: u32,
    /// Shares the order filled.
    pub filled_qty: u64,
    /// The order's value, in paisa.
    pub exit_value_paisa: i128,
}

/// An exact R multiple: `num / den` with `den > 0` (FM-F033). The walker never
/// rounds R; a reader converts it once to a full-precision statistic.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RMultiple {
    /// The signed numerator: the trade's money in raw paisa times the risk's
    /// denominator.
    pub num: i128,
    /// The positive denominator: filled quantity times the planned risk's
    /// numerator.
    pub den: i128,
}

impl RMultiple {
    /// Compares two R multiples exactly, by cross-multiplication, or `None`
    /// when a product leaves `i128`.
    #[must_use]
    pub fn cmp_exact(self, other: Self) -> Option<core::cmp::Ordering> {
        let left = self.num.checked_mul(other.den)?;
        let right = other.num.checked_mul(self.den)?;
        Some(left.cmp(&right))
    }

    /// The whole number `n` as an R multiple.
    #[must_use]
    pub const fn whole(n: i128) -> Self {
        Self { num: n, den: 1 }
    }
}

/// A trade's flags, as bits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TradeFlags(pub u32);

impl TradeFlags {
    /// No flag.
    pub const NONE: Self = Self(0);

    /// These flags with `flag` set.
    #[must_use]
    pub const fn with(self, flag: TradeFlag) -> Self {
        Self(self.0 | (1 << flag as u32))
    }

    /// Whether `flag` is set.
    #[must_use]
    pub const fn contains(self, flag: TradeFlag) -> bool {
        self.0 & (1 << flag as u32) != 0
    }
}

/// One slot's outcome, in exact integers. R is computed from these by the
/// reader and never stored rounded.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SlotOutcome {
    /// Shares filled.
    pub filled_qty: u64,
    /// The entry's value, in paisa.
    pub entry_value_paisa: i128,
    /// The exit's value, in paisa.
    pub exit_value_paisa: i128,
    /// The planned risk per share, in paisa.
    pub risk_per_share_paisa: i64,
    /// The first entry fill's second after IST midnight.
    pub first_entry_second: u32,
    /// The last entry fill's second after IST midnight.
    pub last_entry_second: u32,
    /// The exit's second after IST midnight.
    pub exit_second: u32,
    /// Why the trade ended.
    pub exit_reason: ExitReason,
    /// The most favourable excursion per share up to the exit, in paisa.
    pub mfe_to_exit_paisa: i64,
    /// The most adverse excursion per share up to the exit, in paisa.
    pub mae_to_exit_paisa: i64,
    /// The most favourable excursion per share up to 15:09:59, in paisa.
    pub mfe_to_horizon_paisa: i64,
    /// The most adverse excursion per share up to 15:09:59, in paisa.
    pub mae_to_horizon_paisa: i64,
    /// The trade's flags.
    pub flags: TradeFlags,
    /// What became of the slot (FIB-5). Only [`SlotStatus::Priced`] is a fill
    /// a statistic may use as one.
    pub status: SlotStatus,
    /// The quantity the size buys on the DECISION basis,
    /// `floor(size / entry_ref)` (FM-F034).
    pub decision_qty: u64,
    /// The quantity the walk tried to fill, in the execution feed's raw
    /// shares: [`Self::decision_qty`] at stage A,
    /// `floor(size x zerodha_close / (entry_ref x gdfl_close))` in deep.
    pub planned_qty: u64,
    /// The planned risk per share in the execution feed's raw paisa, as the
    /// exact rational `risk_num / risk_den` (FM-F033): the decision risk times
    /// `gdfl_close`, over `zerodha_close`. Never zero, never rounded.
    pub risk_num: i128,
    /// The planned risk's positive denominator (`zerodha_close`; one at stage
    /// A).
    pub risk_den: i64,
    /// The first entry fill's price, in raw paisa: the base the four
    /// excursions are measured from.
    pub first_entry_price_paisa: i64,
    /// The first second at which the most favourable price up to the exit
    /// printed (the reference walk's MFE second, 17:15 item 0).
    pub mfe_second: u32,
    /// The last second at which that most favourable price was equalled: the
    /// clock second at which open profit last peaked (17:15 item 0).
    pub last_peak_second: u32,
    /// Seconds waited from the entry window's first admitted second to the
    /// first fill (missing seconds are waited for).
    pub entry_wait_seconds: u32,
    /// Seconds an exit order spilled past its touching second (FM-F035).
    pub exit_spill_seconds: u32,
    /// The largest share of one second's volume the entry took, in permille
    /// of that second's GDFL volume (17:15 item 4; zero at stage A).
    pub max_participation_permille: u32,
    /// The slot's own exit order: its stop, target, trail or time exit, a
    /// spill inside it being one order (FM-F125).
    pub own_exit: Option<ExitOrder>,
    /// The forced remainder's order, Dhan's square-off (FM-F125).
    pub forced_exit: Option<ExitOrder>,
}

impl SlotOutcome {
    /// The trade's R multiple from its exact integers (FM-F033): its money
    /// over `filled_qty x planned risk`, a fill's slippage in the numerator
    /// only. `None` for a slot with nothing filled, or when a product leaves
    /// `i128`.
    #[must_use]
    pub fn r_multiple(&self, direction: vocab::fib_cube::Search) -> Option<RMultiple> {
        if self.filled_qty == 0 || self.risk_num <= 0 || self.risk_den <= 0 {
            return None;
        }
        let money = match direction {
            vocab::fib_cube::Search::Long => {
                self.exit_value_paisa.checked_sub(self.entry_value_paisa)?
            }
            vocab::fib_cube::Search::Short => {
                self.entry_value_paisa.checked_sub(self.exit_value_paisa)?
            }
        };
        let num = money.checked_mul(i128::from(self.risk_den))?;
        let den = i128::from(self.filled_qty).checked_mul(self.risk_num)?;
        Some(RMultiple { num, den })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sizes_come_from_the_ledger_and_the_flags_are_bits() {
        assert_eq!(TradeSize::Small.rupees(), 100_000);
        assert_eq!(TradeSize::Large.rupees(), 500_000);
        let flags = TradeFlags::NONE
            .with(TradeFlag::GapThroughStop)
            .with(TradeFlag::SilenceInTrade);
        assert_eq!(flags.0, 0b1001);
        assert!(flags.contains(TradeFlag::GapThroughStop));
        assert!(!flags.contains(TradeFlag::TargetBookedAtWorst));
        assert!(!flags.contains(TradeFlag::DhanSquareOff));
        assert!(flags.contains(TradeFlag::SilenceInTrade));
        assert_eq!(TradeFlags::NONE.with(TradeFlag::DhanSquareOff).0, 0b100);
        let twice = TradeFlags::NONE
            .with(TradeFlag::DhanSquareOff)
            .with(TradeFlag::DhanSquareOff);
        assert_eq!(twice.0, 0b100, "setting a flag again keeps it set");
    }

    #[test]
    fn the_exit_reasons_are_in_precedence_order_and_named() {
        let all = [
            ExitReason::StopLoss,
            ExitReason::TrailingStopLoss,
            ExitReason::TrailingTakeProfit,
            ExitReason::TakeProfit,
            ExitReason::Time,
            ExitReason::Forced,
        ];
        assert!(all.windows(2).all(|w| matches!(w, [a, b] if a < b)));
        let names: Vec<&str> = all.iter().map(|r| r.name()).collect();
        assert_eq!(
            names,
            [
                "stop_loss",
                "trailing_stop_loss",
                "trailing_take_profit",
                "take_profit",
                "time",
                "forced"
            ]
        );
    }
}

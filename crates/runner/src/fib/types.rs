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

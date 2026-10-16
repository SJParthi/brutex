//! Every refusal the CHG-1 charge stack can return, and what it says.
//!
//! The stack refuses where a contract note could not be reproduced honestly,
//! and **never prints a zero in place of an answer**. The cases Dhan's own
//! calculator shows as `0` or `NaN` — a non-positive price, a quantity of
//! zero, a breakeven that cannot be computed — are refusals here, named.
//!
//! [`CostError`] still carries the refusals this crate already had: a
//! non-positive quantity, an overflow, and an unverified rate window. The
//! variants below are the ones only a charge stack can meet.

use std::fmt;

use crate::error::CostError;

/// Why a charge stack refused to price.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ChargeError {
    /// A refusal this crate already names: a non-positive input, an
    /// overflow, or a rate window that was never verified.
    Cost(CostError),
    /// An intraday client-security-day whose bought and sold quantities
    /// differ. An intraday position is squared off the same day — by the
    /// client, or by the broker at the ceiling — so the exit order is
    /// missing, not optional.
    OpenIntradayPosition {
        /// Units bought that day.
        bought: i64,
        /// Units sold that day.
        sold: i64,
    },
    /// A delivery or MTF client-security-day with both buys and sells of
    /// unequal size. Splitting it into a non-delivery and a delivery part
    /// needs the first-in-first-out order sequence (stamp duty is on the
    /// FIFO buy value of the non-deliverable quantity, `NCL/CMPT/43115`),
    /// and the totals alone cannot give it.
    PartlySquaredDay {
        /// Units bought that day.
        bought: i64,
        /// Units sold that day.
        sold: i64,
    },
    /// A client-security-day with no executed order and no exercise.
    EmptyDay,
    /// A broker square-off fee was asked for on a trade kind that has no
    /// ceiling square-off — a delivery or MTF holding is not squared off.
    SquareOffNotApplicable {
        /// The trade kind, as `crate::schedule::TradeKind::as_str` spells it.
        kind: &'static str,
    },
    /// An exercise was recorded on something that is not an option.
    ExerciseNotApplicable {
        /// The trade kind, as `crate::schedule::TradeKind::as_str` spells it.
        kind: &'static str,
    },
    /// A calculator quote with neither a buy nor a sell price: no order was
    /// executed, so there is nothing to price.
    NoOrder,
    /// A figure that needs both legs was asked of a one-sided quote.
    OneSided {
        /// The figure asked for.
        figure: &'static str,
    },
    /// A combination the calculator page was never read for, so its
    /// constants are unknown.
    NotOnPage {
        /// What was asked for.
        what: &'static str,
    },
    /// A rate the selector cannot supply, without a dated window to name.
    NoRate {
        /// The charge.
        charge: &'static str,
        /// Why there is no rate.
        reason: &'static str,
    },
}

impl fmt::Display for ChargeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cost(cost) => write!(f, "{cost}"),
            Self::OpenIntradayPosition { bought, sold } => write!(
                f,
                "an intraday client-security-day bought {bought} and sold {sold}: the position \
                 is open, so its exit order is missing (a broker square-off at the ceiling is \
                 an order too) — refused rather than priced as delivery"
            ),
            Self::PartlySquaredDay { bought, sold } => write!(
                f,
                "a delivery client-security-day bought {bought} and sold {sold}: splitting it \
                 into non-delivery and delivery quantity needs the FIFO order sequence, which \
                 day totals do not carry — refused rather than guessed"
            ),
            Self::EmptyDay => f.write_str(
                "a client-security-day with no executed order and no exercise has no charges \
                 to report — refused rather than printed as zero",
            ),
            Self::SquareOffNotApplicable { kind } => write!(
                f,
                "a broker square-off fee does not apply to {kind}: only an intraday position \
                 is squared off at the ceiling"
            ),
            Self::ExerciseNotApplicable { kind } => write!(
                f,
                "an exercise was recorded on {kind}; only an option is exercised"
            ),
            Self::NoOrder => f.write_str(
                "neither a buy nor a sell price was given: no order was executed, so there is \
                 nothing to price — refused rather than printed as zero",
            ),
            Self::OneSided { figure } => write!(
                f,
                "{figure} needs both a buy and a sell; this quote has one side only — refused \
                 rather than printed as zero"
            ),
            Self::NotOnPage { what } => write!(
                f,
                "{what} was never read from the calculator page, so its constants are \
                 unknown — refused rather than borrowed from another segment"
            ),
            Self::NoRate { charge, reason } => write!(f, "no rate for {charge}: {reason}"),
        }
    }
}

impl std::error::Error for ChargeError {}

impl From<CostError> for ChargeError {
    fn from(cost: CostError) -> Self {
        Self::Cost(cost)
    }
}

impl From<crate::error::Refusal> for ChargeError {
    fn from(refusal: crate::error::Refusal) -> Self {
        Self::Cost(CostError::Unverified(refusal))
    }
}

#[cfg(test)]
#[allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::inconsistent_digit_grouping
)]
mod tests {
    use super::*;
    use crate::day::TradeDay;
    use crate::error::Refusal;

    #[test]
    fn every_charge_refusal_renders_its_own_reason() {
        let cases: [(ChargeError, &str); 10] = [
            (
                ChargeError::Cost(CostError::MalformedDate),
                "not a real calendar date",
            ),
            (
                ChargeError::OpenIntradayPosition {
                    bought: 50,
                    sold: 0,
                },
                "an intraday client-security-day bought 50 and sold 0: the position is open, \
                 so its exit order is missing (a broker square-off at the ceiling is an order \
                 too) — refused rather than priced as delivery",
            ),
            (
                ChargeError::PartlySquaredDay {
                    bought: 100,
                    sold: 60,
                },
                "a delivery client-security-day bought 100 and sold 60: splitting it into \
                 non-delivery and delivery quantity needs the FIFO order sequence, which day \
                 totals do not carry — refused rather than guessed",
            ),
            (
                ChargeError::EmptyDay,
                "a client-security-day with no executed order and no exercise has no charges \
                 to report — refused rather than printed as zero",
            ),
            (
                ChargeError::SquareOffNotApplicable {
                    kind: "equity delivery",
                },
                "a broker square-off fee does not apply to equity delivery: only an intraday \
                 position is squared off at the ceiling",
            ),
            (
                ChargeError::ExerciseNotApplicable { kind: "futures" },
                "an exercise was recorded on futures; only an option is exercised",
            ),
            (
                ChargeError::NoOrder,
                "neither a buy nor a sell price was given: no order was executed, so there is \
                 nothing to price — refused rather than printed as zero",
            ),
            (
                ChargeError::OneSided { figure: "net P&L" },
                "net P&L needs both a buy and a sell; this quote has one side only — refused \
                 rather than printed as zero",
            ),
            (
                ChargeError::NotOnPage {
                    what: "BSE futures",
                },
                "BSE futures was never read from the calculator page, so its constants are \
                 unknown — refused rather than borrowed from another segment",
            ),
            (
                ChargeError::NoRate {
                    charge: "futures exchange charge (BSE)",
                    reason: "no source is recorded",
                },
                "no rate for futures exchange charge (BSE): no source is recorded",
            ),
        ];
        for (error, text) in cases {
            assert_eq!(error.to_string(), text);
            // Usable as a std error, with the same words.
            let boxed: Box<dyn std::error::Error> = Box::new(error);
            assert_eq!(boxed.to_string(), text);
        }
    }

    #[test]
    fn a_cost_error_and_a_refusal_convert_without_losing_their_words() {
        let cost = CostError::NotPositive {
            quantity: "quantity",
            value: 0,
        };
        assert_eq!(ChargeError::from(cost), ChargeError::Cost(cost));
        assert_eq!(ChargeError::from(cost).to_string(), cost.to_string());

        let refusal = Refusal::new(
            "a charge",
            None,
            TradeDay::MIN,
            TradeDay::MIN,
            None,
            "the gap",
        );
        let wrapped = ChargeError::from(refusal);
        assert_eq!(wrapped, ChargeError::Cost(CostError::Unverified(refusal)));
        assert_eq!(wrapped.to_string(), refusal.to_string());
    }
}

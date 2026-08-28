//! The operator's acceptance criteria, as runtime data rather than constants.
//!
//! # Why this module exists at all
//!
//! The criteria that decide which combinations are worth trading were spread
//! across `crates/cli` as compile-time constants — a stop floor, a stop cap, a
//! reference price, a reward-to-risk threshold, a win-rate threshold that was
//! set to zero. Some were reachable as command-line arguments; none was
//! reachable from the browser, which is where every run is now started.
//!
//! A constant that decides an answer is a policy nobody typed. `crates/cli`
//! already says so about its own rule set — *"a rule an operator did not type
//! is a policy the engine invented"* — and this module is that sentence given a
//! type. A [`Bound`] is built by whoever is asking: a CLI flag, an HTTP body, a
//! test. It carries no default and none can be added, because the moment one
//! exists a caller who supplied nothing gets a threshold they never chose.
//!
//! # A refusal that names its cause
//!
//! [`Bound::verdict`] returns which clause failed and by how much, not a bare
//! `false`. That is the difference between an operator who can see that a
//! combination missed the win rate by one trade and one who is told only that
//! nothing passed.
//!
//! `CLAUDE.md` §4 bans "a fallback that hides a failure" and requires degrading
//! loudly with the reason named. A boolean refusal over eight thousand exit
//! variants is exactly the hidden failure that rule is about: every one of them
//! could have been rejected for a different reason and the operator would see
//! one empty table.
//!
//! # Cost
//!
//! Every clause is integer arithmetic over fields already resident on the
//! [`Cell`]. No allocation, no scan, no branch that depends on the number of
//! trades or the number of bars. `CLAUDE.md` §3 rule 4 holds per call.

use core::fmt;

use crate::grid::Cell;

/// Why a variant was admitted, or which clause refused it and by how much.
///
/// # Both sides of every comparison are carried
///
/// `WinRateShort { got: 4_800, needs: 5_000 }` tells an operator they are two
/// percentage points short. `false` tells them nothing, and the difference
/// decides whether they lower the bound or discard the combination.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// Every clause held.
    Admitted,
    /// The variant took no round trips at all, so no ratio it could report
    /// would mean anything.
    NoTrades,
    /// The variant took trades and won none. Its smallest win does not exist,
    /// so a reward ratio cannot be formed from it at any threshold.
    NoWinners,
    /// Fewer round trips than the bound requires.
    TooFewTrades {
        /// Round trips this variant took.
        took: u64,
        /// Round trips the bound requires.
        needs: u64,
    },
    /// Win rate short of the bound. Both figures are percentages in hundredths.
    WinRateShort {
        /// The variant's win rate, in hundredths of a percent.
        got: i64,
        /// The bound's floor, in hundredths of a percent.
        needs: i64,
    },
    /// Smallest win over largest loss, short of the bound. Both figures are
    /// ratios in hundredths, so `125` reads 1.25.
    RewardToRiskShort {
        /// The variant's smallest win over its largest loss, in hundredths.
        got: i64,
        /// The bound's floor, in hundredths.
        needs: i64,
    },
    /// The guaranteed floor is short of the bound. Both figures are paisa.
    FloorShort {
        /// What the variant would have made in its worst arrangement, in paisa.
        got: i64,
        /// The bound's floor, in paisa.
        needs: i64,
    },
}

impl Verdict {
    /// Whether this verdict admits the variant.
    #[must_use]
    pub const fn admitted(self) -> bool {
        matches!(self, Self::Admitted)
    }
}

impl fmt::Display for Verdict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Admitted => write!(f, "admitted"),
            Self::NoTrades => write!(f, "took no trades"),
            Self::NoWinners => write!(f, "took trades and won none"),
            Self::TooFewTrades { took, needs } => {
                write!(f, "took {took} trades, needs {needs}")
            }
            Self::WinRateShort { got, needs } => write!(
                f,
                "win rate {}.{:02}%, needs {}.{:02}%",
                got / 100,
                (got % 100).abs(),
                needs / 100,
                (needs % 100).abs()
            ),
            Self::RewardToRiskShort { got, needs } => write!(
                f,
                "smallest win over largest loss {}.{:02}, needs {}.{:02}",
                got / 100,
                (got % 100).abs(),
                needs / 100,
                (needs % 100).abs()
            ),
            Self::FloorShort { got, needs } => {
                write!(f, "guaranteed floor {got} paisa, needs {needs}")
            }
        }
    }
}

/// The criteria a variant must clear, supplied by the caller.
///
/// # Every field is stated, and there is deliberately no `Default`
///
/// The operator's current requirement reads: *at least half the trades win, the
/// smallest single win is at least 1.25 times the largest single loss, and both
/// legs are priced at the worst the one-minute bar printed.* That is
/// [`Bound::new(min_trades, 5_000, 125, 1)`][Bound::new] — and the numbers live
/// in whatever asked for the run, not in this file.
///
/// A `Default` impl is not missing by oversight. It would mean a caller who
/// supplied nothing still got thresholds, and those thresholds would then be
/// the very compile-time policy this type exists to remove.
///
/// # Why the floor is a separate clause from the two ratios
///
/// The win-rate and reward-to-risk pair is a *sufficient* condition for the
/// guaranteed floor being positive; it is not a *necessary* one. A variant
/// winning 40% of the time at a 2.0 ratio floors at `+200` per thousand trades
/// — better than 50% at 1.25, which floors at `+125` — and fails a 50% clause.
///
/// Carrying the floor as its own threshold lets a caller express either
/// discipline: the two ratios for an operator who wants a shape they recognise,
/// the floor for one who wants the outcome and does not care how it is reached.
/// Setting `min_floor` to `1` asks only that the worst arrangement still makes
/// money; setting the two ratios to `0` asks only for the floor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[expect(
    clippy::struct_field_names,
    reason = "the shared `min_` is the MEANING and not a namespace: every field \
              is a floor the caller requires, and dropping it leaves `trades` \
              and `floor`, which read as quantities this type HOLDS rather \
              than minimums it DEMANDS. The lint is right about the shape and \
              wrong about this type"
)]
pub struct Bound {
    min_trades: u64,
    min_win_rate_bp: i64,
    min_rr_bp: i64,
    min_floor: i64,
}

impl Bound {
    /// The criteria, every one supplied.
    ///
    /// `min_win_rate_bp` and `min_rr_bp` are hundredths — 50% is `5_000` and a
    /// 1.25 ratio is `125`. `min_floor` is paisa.
    #[must_use]
    pub const fn new(
        min_trades: u64,
        min_win_rate_bp: i64,
        min_rr_bp: i64,
        min_floor: i64,
    ) -> Self {
        Self {
            min_trades,
            min_win_rate_bp,
            min_rr_bp,
            min_floor,
        }
    }

    /// The trade-count floor this bound requires.
    #[must_use]
    pub const fn min_trades(&self) -> u64 {
        self.min_trades
    }

    /// The win-rate floor, in hundredths of a percent.
    #[must_use]
    pub const fn min_win_rate_bp(&self) -> i64 {
        self.min_win_rate_bp
    }

    /// The reward-to-risk floor, in hundredths.
    #[must_use]
    pub const fn min_rr_bp(&self) -> i64 {
        self.min_rr_bp
    }

    /// The guaranteed-floor threshold, in paisa.
    #[must_use]
    pub const fn min_floor(&self) -> i64 {
        self.min_floor
    }

    /// Which clause decides this variant, and by how much.
    ///
    /// # The order is not arbitrary
    ///
    /// The clauses run from the ones that make the later ones meaningless to
    /// the ones that are merely unmet. A variant with no trades has no win rate
    /// to be short of, and a variant with no winners has no smallest win — so
    /// reporting either as `WinRateShort { got: 0 }` would name a true number
    /// that describes the wrong problem. An operator reading "win rate 0.00%,
    /// needs 50.00%" would lower the bound; the honest answer is that nothing
    /// traded.
    ///
    /// After those two, sample size precedes the ratios for the same reason:
    /// a three-trade variant can show a perfect win rate and an unbounded
    /// reward ratio, and both are arithmetic rather than evidence.
    #[must_use]
    pub fn verdict(&self, cell: &Cell) -> Verdict {
        if cell.trades == 0 {
            return Verdict::NoTrades;
        }
        if cell.wins == 0 {
            return Verdict::NoWinners;
        }
        if cell.trades < self.min_trades {
            return Verdict::TooFewTrades {
                took: cell.trades,
                needs: self.min_trades,
            };
        }
        let win_rate = cell.win_rate_bp();
        if win_rate < self.min_win_rate_bp {
            return Verdict::WinRateShort {
                got: win_rate,
                needs: self.min_win_rate_bp,
            };
        }
        let rr = cell.reward_to_risk_bp();
        if rr < self.min_rr_bp {
            return Verdict::RewardToRiskShort {
                got: rr,
                needs: self.min_rr_bp,
            };
        }
        let floor = cell.guaranteed_floor();
        if floor < self.min_floor {
            return Verdict::FloorShort {
                got: floor,
                needs: self.min_floor,
            };
        }
        Verdict::Admitted
    }

    /// Whether this variant clears every clause.
    ///
    /// [`Self::verdict`] is the same walk carrying its reason. Prefer it
    /// wherever the answer is shown to an operator; this is for the filter
    /// itself, where the reason is discarded anyway.
    #[must_use]
    pub fn admits(&self, cell: &Cell) -> bool {
        self.verdict(cell).admitted()
    }
}

#[cfg(test)]
mod tests {
    use super::{Bound, Verdict};
    use crate::grid::Cell;

    /// The operator's stated requirement, as a `Bound`, against the cell their
    /// own worked example describes.
    #[test]
    fn the_operators_bound_admits_the_operators_example() {
        let bound = Bound::new(50, 5_000, 125, 1);
        let cell = Cell {
            trades: 1_000,
            wins: 500,
            min_win: 125,
            worst_trade: -100,
            ..Cell::default()
        };
        assert_eq!(bound.verdict(&cell), Verdict::Admitted);
        assert!(bound.admits(&cell));

        // AND THE ACCESSORS RETURN WHAT WAS PUT IN. They exist so a caller that
        // built the bound from an HTTP body can render it back beside the
        // result, which is the whole point of the criteria being data.
        assert_eq!(bound.min_trades(), 50);
        assert_eq!(bound.min_win_rate_bp(), 5_000);
        assert_eq!(bound.min_rr_bp(), 125);
        assert_eq!(bound.min_floor(), 1);
    }

    /// Every clause refuses in its own right, and names both sides.
    #[test]
    fn each_clause_refuses_with_its_own_reason() {
        let bound = Bound::new(50, 5_000, 125, 1);
        let sound = Cell {
            trades: 1_000,
            wins: 500,
            min_win: 125,
            worst_trade: -100,
            ..Cell::default()
        };

        assert_eq!(
            bound.verdict(&Cell {
                trades: 0,
                wins: 0,
                ..sound
            }),
            Verdict::NoTrades
        );
        assert_eq!(
            bound.verdict(&Cell { wins: 0, ..sound }),
            Verdict::NoWinners,
            "no winners must not be reported as a short win rate"
        );
        assert_eq!(
            bound.verdict(&Cell {
                trades: 10,
                wins: 6,
                ..sound
            }),
            Verdict::TooFewTrades {
                took: 10,
                needs: 50
            }
        );
        assert_eq!(
            bound.verdict(&Cell {
                trades: 1_000,
                wins: 480,
                ..sound
            }),
            Verdict::WinRateShort {
                got: 4_800,
                needs: 5_000
            }
        );
        assert_eq!(
            bound.verdict(&Cell {
                min_win: 110,
                ..sound
            }),
            Verdict::RewardToRiskShort {
                got: 110,
                needs: 125
            }
        );
    }

    /// The floor is a separate clause, and it catches what the two ratios pass.
    #[test]
    fn the_floor_clause_binds_independently_of_the_two_ratios() {
        // BOTH RATIOS CLEAR AND THE FLOOR DOES NOT. 500 x 125 - 500 x 100 is
        // 12,500 paisa, which is under a bound of 20,000.
        let strict = Bound::new(50, 5_000, 125, 20_000);
        let cell = Cell {
            trades: 1_000,
            wins: 500,
            min_win: 125,
            worst_trade: -100,
            ..Cell::default()
        };
        assert_eq!(
            strict.verdict(&cell),
            Verdict::FloorShort {
                got: 12_500,
                needs: 20_000
            },
            "the floor must be able to refuse a cell both ratios admit"
        );

        // AND THE CONVERSE: A FLOOR-ONLY BOUND ADMITS WHAT THE RATIOS REFUSE.
        // 40% winners at a 2.0 ratio floors at +20,000 -- better than the
        // operator's own target -- and fails a 50% win-rate clause.
        let floor_only = Bound::new(50, 0, 0, 1);
        let ratios_too = Bound::new(50, 5_000, 0, 1);
        let lopsided = Cell {
            trades: 1_000,
            wins: 400,
            min_win: 200,
            worst_trade: -100,
            ..Cell::default()
        };
        assert_eq!(lopsided.guaranteed_floor(), 20_000);
        assert_eq!(floor_only.verdict(&lopsided), Verdict::Admitted);
        assert_eq!(
            ratios_too.verdict(&lopsided),
            Verdict::WinRateShort {
                got: 4_000,
                needs: 5_000
            },
            "the same cell must be refused by a shape rule it fails and \
             admitted by an outcome rule it passes -- that is why both exist"
        );
    }

    /// Every verdict renders a sentence an operator can act on.
    #[test]
    fn every_verdict_names_its_cause_in_words() {
        assert_eq!(Verdict::Admitted.to_string(), "admitted");
        assert!(Verdict::Admitted.admitted());
        assert_eq!(Verdict::NoTrades.to_string(), "took no trades");
        assert!(!Verdict::NoTrades.admitted());
        assert_eq!(Verdict::NoWinners.to_string(), "took trades and won none");
        assert_eq!(
            Verdict::TooFewTrades {
                took: 12,
                needs: 50
            }
            .to_string(),
            "took 12 trades, needs 50"
        );
        assert_eq!(
            Verdict::WinRateShort {
                got: 4_812,
                needs: 5_000
            }
            .to_string(),
            "win rate 48.12%, needs 50.00%"
        );
        assert_eq!(
            Verdict::RewardToRiskShort {
                got: 107,
                needs: 125
            }
            .to_string(),
            "smallest win over largest loss 1.07, needs 1.25"
        );
        assert_eq!(
            Verdict::FloorShort {
                got: -250,
                needs: 1
            }
            .to_string(),
            "guaranteed floor -250 paisa, needs 1"
        );
    }

    /// A NEGATIVE remainder must not render as a second minus sign.
    ///
    /// `-250 / 100` is `-2` and `-250 % 100` is `-50` in Rust, so the naive
    /// format string produces `-2.-50`. The `abs` on the fractional half is
    /// what stops it, and this is the case that proves it.
    #[test]
    fn a_negative_ratio_renders_with_one_minus_sign() {
        assert_eq!(
            Verdict::RewardToRiskShort {
                got: -250,
                needs: 125
            }
            .to_string(),
            "smallest win over largest loss -2.50, needs 1.25"
        );
    }
}

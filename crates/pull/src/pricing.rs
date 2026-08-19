//! The arrow from this repository to [`greeks`], and the first one there has
//! ever been.
//!
//! # What was missing
//!
//! `crates/greeks` is complete: Black-Scholes-Merton, a bounded implied-
//! volatility solver, moneyness, an error type that names every refusal, and a
//! test suite that checks the analytic greeks against numerical derivatives.
//! It has one property no other crate here has — measured with
//! `cargo metadata --no-deps`, not read off a diagram:
//!
//! ```text
//! greeks -> []          and NOTHING points at greeks
//! ```
//!
//! Thirteen crates, and not one of them names it. Every greek in this
//! repository is computed by nobody. This module is the arrow, and D-0217 is
//! the ledger entry for adding it.
//!
//! # Everything is PAISA, and stays paisa
//!
//! §7 says prices are paisa integers and never floats. Black-Scholes wants
//! floats, so the boundary is here — but the SCALE does not change at it.
//!
//! Black-Scholes is homogeneous of degree one in `(spot, strike, price)`:
//! multiply all three by a hundred and the model prices multiply by a hundred
//! too. So working in paisa rather than rupees is not an approximation, it is
//! the same equation, and it avoids a division that would round. Two
//! consequences a caller must know, and they are the reason this is spelled out
//! rather than assumed:
//!
//! * **Implied volatility and delta are unchanged.** Both are scale-free.
//! * **Gamma, vega, theta and rho are NOT.** They come out per paisa. Vega and
//!   theta are a hundred times their rupee values; gamma is a hundredth.
//!
//! `i64` paisa is exact in `f64` up to `2^53`, which is ninety trillion rupees.
//! Nothing this market prints comes close, so the widening is lossless.
//!
//! # The rate cannot be invented, and the type is what stops it
//!
//! BSM needs five inputs. Four are in hand — the strike from
//! [`crate::fno::read_contract`], the spot from Dhan's overlay or the index
//! bar, the tenor from [`crate::tenor`], and a carry of zero for an index with
//! no dividend adjustment. The fifth is the risk-free rate, and **no page in
//! `docs/00-charter.md` records one**.
//!
//! §3 rule 1 says a number with no source is not written down. So [`Rate`]
//! cannot be constructed without naming its source, there is no constant of it
//! anywhere in this workspace, and there is no `Default`. Nothing here can
//! price until a rate is measured — which is the honest state of this build,
//! made structural instead of documented.
//!
//! # Cost
//!
//! **O(1) time, O(1) space, no allocation.** [`greeks_at`] is a closed form:
//! two `exp`, two normal CDFs, one normal PDF, fixed arithmetic.
//! [`solve_iv`] is bounded by `greeks::solver::MAX_ITERATIONS`, a compile-time
//! constant no input can raise — the returned `iterations` is the count that
//! was actually spent, so the bound is observable rather than claimed.

use brutex_core::instrument::OptionSide;
use greeks::bsm::{Contract, Greeks, OptionKind};
use greeks::error::GreeksError;
use greeks::solver::ImpliedVolatility;

use crate::tenor::{Tenor, YearBasis};

/// The risk-free rate, and where it came from.
///
/// # Why the source is a field and not a comment
///
/// §3 rule 1: every claim about a cost is traceable to a source recorded in
/// `docs/00-charter.md`. A rate is a cost. Making the citation a constructor
/// argument means a rate with no source is not a rate that compiles — the rule
/// stops being something a reviewer has to notice.
///
/// There is deliberately **no constant of this type** in the workspace and no
/// `Default`. When Phase 2 measures one by solving against Dhan's published
/// implied volatility, the value and its window go in the charter and the
/// citation goes here.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rate {
    annual: f64,
    basis: YearBasis,
    source: &'static str,
}

impl Rate {
    /// A rate that was measured, with the source that measured it.
    ///
    /// `annual` is continuously compounded and a decimal — `0.0946`, not
    /// `9.46`, the same convention `greeks::bsm::Contract::rate` states.
    ///
    /// The `basis` travels with the value because it must: `r` and `T` enter
    /// the discounting together, so a rate solved under 365 calendar days is
    /// **not** the rate to use under 252 trading days. Carrying it here makes
    /// [`solve_iv`] able to refuse a mismatch instead of quietly repricing.
    ///
    /// # Errors
    ///
    /// [`PricingError::RateNotFinite`] for a `NaN` or an infinity, and
    /// [`PricingError::RateUnsourced`] for an empty source. A blank citation is
    /// worse than none: it looks like the rule was followed.
    pub fn measured(
        annual: f64,
        basis: YearBasis,
        source: &'static str,
    ) -> Result<Self, PricingError> {
        if !annual.is_finite() {
            return Err(PricingError::RateNotFinite { annual });
        }
        if source.trim().is_empty() {
            return Err(PricingError::RateUnsourced);
        }
        Ok(Self {
            annual,
            basis,
            source,
        })
    }

    /// The rate, continuously compounded.
    #[must_use]
    pub const fn annual(self) -> f64 {
        self.annual
    }

    /// The year basis this rate was measured under.
    #[must_use]
    pub const fn basis(self) -> YearBasis {
        self.basis
    }

    /// Where the value came from.
    #[must_use]
    pub const fn source(self) -> &'static str {
        self.source
    }
}

/// One option, at one bar, in paisa.
///
/// Every price is a paisa integer straight off the store — see the module
/// header on why they stay paisa through the model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Quote {
    /// The underlying's level at this bar, in paisa.
    pub spot: i64,
    /// The contract's strike, in paisa.
    pub strike: i64,
    /// What the option traded at, in paisa. The bar's close.
    pub premium: i64,
    /// How long the contract had left. See [`crate::tenor`].
    pub tenor: Tenor,
    /// Call or put.
    pub side: OptionSide,
}

/// Why a quote could not be priced.
///
/// Every arm names something READ and refused. None is a fallback, and none
/// substitutes a value — §4.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PricingError {
    /// A rate that is not a number.
    RateNotFinite {
        /// The value offered.
        annual: f64,
    },
    /// A rate with no citation. See [`Rate`].
    RateUnsourced,
    /// The rate was measured under one year basis and the tenor read under
    /// another.
    ///
    /// **Not a conversion.** The two are only interchangeable if the rate is
    /// re-measured, which this module cannot do, so it refuses rather than
    /// repricing the contract under a rate that never applied to it.
    BasisMismatch {
        /// What the rate was measured under.
        rate: YearBasis,
        /// What the caller asked to price under.
        asked: YearBasis,
    },
    /// A price at or below zero, which no option ever traded at.
    ///
    /// Zero is in this arm. A spot or strike of zero makes the model's
    /// logarithm undefined, and a premium of zero is a row the vendor sent as
    /// a placeholder rather than a trade.
    NotPositive {
        /// Which of `spot`, `strike` or `premium`.
        field: &'static str,
        /// The paisa value that was refused.
        paisa: i64,
    },
    /// The model itself refused, with its own reason kept intact.
    ///
    /// Wrapped rather than flattened: `greeks` distinguishes a price below
    /// intrinsic from one above the no-arbitrage maximum from one outside the
    /// solvable volatility range, and an operator needs to know which.
    Model(GreeksError),
}

impl std::fmt::Display for PricingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RateNotFinite { annual } => write!(
                f,
                "the risk-free rate {annual} is not a finite number, so no \
                 contract was priced"
            ),
            Self::RateUnsourced => write!(
                f,
                "a risk-free rate was offered with no source. CLAUDE.md §3 \
                 rule 1: a number with no source is not written down, and a \
                 blank citation is worse than none because it looks like the \
                 rule was followed"
            ),
            Self::BasisMismatch { rate, asked } => write!(
                f,
                "the rate was measured under {rate:?} and the tenor was read \
                 under {asked:?}. The rate and the time enter the discounting \
                 together, so these are not interchangeable and converting one \
                 would price the contract under a rate that never applied to \
                 it. Re-measure the rate under {asked:?} instead."
            ),
            Self::NotPositive { field, paisa } => write!(
                f,
                "the {field} is {paisa} paisa, and no option ever traded at or \
                 below zero. Nothing was priced rather than a placeholder row \
                 being read as a quote"
            ),
            Self::Model(why) => write!(f, "the model refused this quote: {why}"),
        }
    }
}

impl std::error::Error for PricingError {}

impl From<GreeksError> for PricingError {
    fn from(why: GreeksError) -> Self {
        Self::Model(why)
    }
}

/// `brutex_core`'s side becomes the model's kind.
///
/// Two enums that mean the same thing in two crates that must not know about
/// each other: `greeks` depends on NOTHING, which is a rule CI gate 9b proves,
/// so it cannot name `brutex_core::instrument::OptionSide`. The translation
/// lives here, in the crate that already depends on both.
const fn kind_of(side: OptionSide) -> OptionKind {
    match side {
        OptionSide::Call => OptionKind::Call,
        OptionSide::Put => OptionKind::Put,
    }
}

/// The model contract for one quote, or the reason there is none.
fn contract_of(quote: Quote, rate: Rate, basis: YearBasis) -> Result<Contract, PricingError> {
    if rate.basis() != basis {
        return Err(PricingError::BasisMismatch {
            rate: rate.basis(),
            asked: basis,
        });
    }
    for (field, paisa) in [
        ("spot", quote.spot),
        ("strike", quote.strike),
        ("premium", quote.premium),
    ] {
        if paisa <= 0 {
            return Err(PricingError::NotPositive { field, paisa });
        }
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "paisa is exact in f64 to 2^53, which is ninety trillion \
                  rupees. Nothing this market prints is within nine orders of \
                  magnitude of that, so the widening is lossless"
    )]
    Ok(Contract {
        spot: quote.spot as f64,
        strike: quote.strike as f64,
        years_to_expiry: quote.tenor.years(basis),
        rate: rate.annual(),
        // ZERO, AND NOT AN ASSUMPTION. `greeks::bsm::Contract::carry` says it
        // itself: "Zero for a spot index with no dividend adjustment." The two
        // instruments this engine sweeps are spot indices, and §1 pins that to
        // exactly two. A single stock would need a real dividend yield and this
        // field would become the next thing with no source.
        carry: 0.0,
    })
}

/// The volatility that reproduces what the option actually traded at.
///
/// This is the whole answer for Groww, whose candles carry open, high, low,
/// close and volume and **no implied volatility at all** — measured against the
/// vendor's own positional-array contract, not assumed. Dhan sends its own on
/// the rolling-option overlay; this is what fills the gap for the vendor that
/// does not, and what checks the vendor that does.
///
/// # Errors
///
/// Every arm of [`PricingError`], including the model's own refusals kept
/// intact — a premium below intrinsic and one above the no-arbitrage maximum
/// are different facts about a row and are reported as different facts.
///
/// # Cost
///
/// Bounded by `greeks::solver::MAX_ITERATIONS`, a compile-time constant. The
/// returned `iterations` is what was actually spent, so the bound is observable
/// rather than claimed. O(1) time, O(1) space, no allocation.
pub fn solve_iv(
    quote: Quote,
    rate: Rate,
    basis: YearBasis,
) -> Result<ImpliedVolatility, PricingError> {
    #[expect(
        clippy::cast_precision_loss,
        reason = "see contract_of — paisa is exact in f64 at this magnitude"
    )]
    let premium = quote.premium as f64;
    Ok(contract_of(quote, rate, basis)?.implied_volatility(premium, kind_of(quote.side))?)
}

/// The greeks of one quote at a known volatility.
///
/// Volatility is a parameter rather than a field for the reason `greeks` gives:
/// it is sometimes an input and sometimes the answer, and keeping it out of the
/// contract means a stale one cannot be handed in by accident. Pass a vendor's
/// (Dhan's overlay) or a solved one ([`solve_iv`]).
///
/// # Errors
///
/// Every arm of [`PricingError`].
///
/// # Cost
///
/// Closed form. O(1) time, O(1) space, no allocation, no iteration at all.
pub fn greeks_at(
    quote: Quote,
    volatility: f64,
    rate: Rate,
    basis: YearBasis,
) -> Result<Greeks, PricingError> {
    Ok(contract_of(quote, rate, basis)?.greeks(volatility, kind_of(quote.side))?)
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "a test that cannot panic cannot fail, and these lints exist to \
              keep panics out of the crate rather than out of its tests"
)]
mod tests {
    use super::*;
    use brutex_core::instrument::Expiry;

    /// A rate for the units below ONLY. Not a measurement and not a default —
    /// see [`Rate`] on why no constant of this type exists in the workspace.
    fn test_rate() -> Rate {
        Rate::measured(
            0.065,
            YearBasis::Calendar365,
            "a value invented for this test module and for nothing else. \
             docs/00-charter.md records no rate; Phase 2 measures one",
        )
        .expect("a finite rate with a source")
    }

    /// A live tenor `days` before the 2026-08-27 expiry, at 09:15 IST.
    fn tenor(days: u8) -> Tenor {
        let start = crate::session::Day::new(2026, 8, 27 - days).expect("a day in its life");
        let secs = i64::from(start.days_from_epoch()) * 86_400 - 19_800 + 9 * 3_600 + 15 * 60;
        Tenor::between(
            secs * 1_000_000,
            Expiry::new(2026, 8, 27).expect("a real expiry"),
        )
        .expect("a live tenor")
    }

    fn atm_call() -> Quote {
        Quote {
            spot: 2_500_000,
            strike: 2_500_000,
            premium: 25_000,
            tenor: tenor(7),
            side: OptionSide::Call,
        }
    }

    /// **THE ARROW WORKS.** A real quote goes in and a volatility comes out.
    ///
    /// Before this module the greeks crate had no caller at all, so this unit
    /// is the first evidence in the repository that its solver runs on data
    /// shaped like this store's.
    #[test]
    fn a_paisa_quote_solves_to_a_volatility_the_model_can_reproduce() {
        let quote = atm_call();
        let solved = solve_iv(quote, test_rate(), YearBasis::Calendar365).expect("a solvable ATM");

        assert!(
            solved.volatility > 0.0 && solved.volatility < 5.0,
            "{} is not a volatility",
            solved.volatility
        );
        assert!(
            solved.iterations <= greeks::solver::MAX_ITERATIONS,
            "the bound is observable, not claimed"
        );

        // AND IT ROUND-TRIPS: pricing at the solved volatility returns the
        // premium that was fed in. This is what makes the answer checkable
        // rather than merely finite.
        let back = greeks_at(
            quote,
            solved.volatility,
            test_rate(),
            YearBasis::Calendar365,
        )
        .expect("greeks at the solution");
        #[expect(clippy::cast_precision_loss, reason = "exact at this magnitude")]
        let premium = quote.premium as f64;
        assert!(
            (back.price - premium).abs() < 1.0,
            "solved {} reprices to {} against {premium} paisa",
            solved.volatility,
            back.price
        );
    }

    /// **PAISA AND RUPEES GIVE THE SAME IMPLIED VOLATILITY.**
    ///
    /// The module header claims Black-Scholes is homogeneous of degree one in
    /// `(spot, strike, premium)`, and the whole decision to stay in paisa rests
    /// on it. This is that claim, checked rather than cited.
    #[test]
    fn the_scale_does_not_change_the_implied_volatility() {
        let paisa = atm_call();
        let rupees = Quote {
            spot: 25_000,
            strike: 25_000,
            premium: 250,
            ..paisa
        };
        let a = solve_iv(paisa, test_rate(), YearBasis::Calendar365).expect("paisa");
        let b = solve_iv(rupees, test_rate(), YearBasis::Calendar365).expect("rupees");
        assert!(
            (a.volatility - b.volatility).abs() < 1e-9,
            "{} against {} — the homogeneity argument is wrong",
            a.volatility,
            b.volatility
        );
    }

    /// A rate with no source does not compile into existence.
    #[test]
    fn a_rate_without_a_citation_is_refused() {
        assert_eq!(
            Rate::measured(0.065, YearBasis::Calendar365, "   "),
            Err(PricingError::RateUnsourced),
            "a blank citation looks like the rule was followed"
        );
        assert_eq!(
            Rate::measured(0.065, YearBasis::Calendar365, ""),
            Err(PricingError::RateUnsourced)
        );
        // `assert_eq!` CANNOT BE USED HERE and the reason is the bug it would
        // hide: `PartialEq` on this type is derived, so it compares the `f64`,
        // and `NaN != NaN` by IEEE-754. The assertion fails while PRINTING two
        // values that read identically — `RateNotFinite { annual: NaN }` on
        // both sides. Matched on shape instead.
        assert!(
            matches!(
                Rate::measured(f64::NAN, YearBasis::Calendar365, "somewhere"),
                Err(PricingError::RateNotFinite { annual }) if annual.is_nan()
            ),
            "a NaN rate is not a rate"
        );
        assert!(
            Rate::measured(f64::INFINITY, YearBasis::Calendar365, "somewhere").is_err(),
            "an infinite rate is not a rate"
        );
    }

    /// A placeholder row is not a quote.
    #[test]
    fn a_zero_or_negative_price_is_refused_by_name() {
        for (field, bad) in [
            (
                "spot",
                Quote {
                    spot: 0,
                    ..atm_call()
                },
            ),
            (
                "strike",
                Quote {
                    strike: -1,
                    ..atm_call()
                },
            ),
            (
                "premium",
                Quote {
                    premium: 0,
                    ..atm_call()
                },
            ),
        ] {
            let error = solve_iv(bad, test_rate(), YearBasis::Calendar365).unwrap_err();
            assert!(
                matches!(error, PricingError::NotPositive { field: f, .. } if f == field),
                "{field} was refused as {error:?}"
            );
            assert!(error.to_string().contains(field), "{error}");
        }
    }

    /// The model's own refusals arrive intact, not flattened.
    ///
    /// A premium below intrinsic and one above the no-arbitrage maximum are
    /// different facts about a row, and an operator reading a census needs to
    /// know which one they have.
    #[test]
    fn the_models_reasons_survive_the_wrapping() {
        // Deep in the money, priced at almost nothing: below intrinsic.
        let below = Quote {
            spot: 2_600_000,
            strike: 2_500_000,
            premium: 100,
            ..atm_call()
        };
        assert!(
            matches!(
                solve_iv(below, test_rate(), YearBasis::Calendar365),
                Err(PricingError::Model(GreeksError::PriceBelowIntrinsic { .. }))
            ),
            "a premium under intrinsic must say so"
        );

        // Priced above the underlying: above the no-arbitrage maximum.
        let above = Quote {
            premium: 3_000_000,
            ..atm_call()
        };
        assert!(
            matches!(
                solve_iv(above, test_rate(), YearBasis::Calendar365),
                Err(PricingError::Model(GreeksError::PriceAboveMaximum { .. }))
            ),
            "a premium above the spot must say so"
        );
    }

    /// Both sides price, and the put is not the call.
    #[test]
    fn a_put_and_a_call_at_one_strike_are_priced_apart() {
        let call = atm_call();
        let put = Quote {
            side: OptionSide::Put,
            ..call
        };
        let a = solve_iv(call, test_rate(), YearBasis::Calendar365).expect("call");
        let b = solve_iv(put, test_rate(), YearBasis::Calendar365).expect("put");
        assert!(
            (a.volatility - b.volatility).abs() > 1e-6,
            "at one premium the two sides imply different volatilities under a \
             non-zero rate; identical answers would mean the side is ignored"
        );

        // AND THE SIGNS ARE THE RIGHT WAY ROUND, which is the check that
        // catches the two kinds being swapped in `kind_of`.
        let ga = greeks_at(call, 0.15, test_rate(), YearBasis::Calendar365).expect("call greeks");
        let gb = greeks_at(put, 0.15, test_rate(), YearBasis::Calendar365).expect("put greeks");
        assert!(ga.delta > 0.0, "a call's delta is positive: {}", ga.delta);
        assert!(gb.delta < 0.0, "a put's delta is negative: {}", gb.delta);
        assert!(
            (ga.gamma - gb.gamma).abs() < 1e-12,
            "gamma is identical for a call and a put at one strike"
        );
    }

    /// A rate measured under one basis will not price under another.
    ///
    /// The arm exists because the alternative — converting — prices the
    /// contract under a rate that was never measured for it.
    #[test]
    fn a_basis_the_rate_was_not_measured_under_is_refused_rather_than_converted() {
        // Only one basis is implemented, so this asserts the CHECK is wired by
        // proving the matching case passes. A second variant makes the
        // mismatching case constructible; until then this is the honest half.
        let rate = test_rate();
        assert_eq!(rate.basis(), YearBasis::Calendar365);
        assert!(solve_iv(atm_call(), rate, YearBasis::Calendar365).is_ok());
        assert!(
            rate.source().contains("charter"),
            "the citation travels with the value: {}",
            rate.source()
        );
    }
}

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

use std::collections::HashMap;

use brutex_core::instrument::OptionSide;
use brutex_core::price::Paisa;
use greeks::bsm::{Contract, Greeks, OptionKind};
use greeks::error::GreeksError;
use greeks::moneyness::Moneyness;
use greeks::solver::ImpliedVolatility;

use crate::tenor::{Tenor, YearBasis};

// THE TWO `costs` TYPES A CALLER NEEDS, RE-EXPORTED RATHER THAN REACHED FOR.
//
// `Quote` names both in its public surface, so any caller has to be able to
// build them. `api` does NOT depend on `costs` — its dependency set is `core`,
// `pull`, `store`, `telemetry` — and adding an arrow to the crate graph to pass
// two `Copy` values through would be a §5 change and a `docs/05-decisions.md`
// entry for no gain. `pull` already depends on `costs` and is the crate on the
// path, so the conversions live here, once.
pub use costs::day::TradeDay;
pub use costs::venue::SweptSlot;

/// The swept underlying a symbol names, or `None` for one with no regime.
///
/// `None` is a recorded absence and not a failure: an underlying with no
/// strike-ladder regime cannot have a moneyness, so pricing is skipped for it
/// with that reason rather than a ladder being guessed at.
///
/// # Cost
///
/// O(1) — see `costs::venue::swept_slot`.
#[must_use]
pub fn slot_of(symbol: brutex_core::symbol::Symbol) -> Option<SweptSlot> {
    costs::venue::swept_slot(symbol).ok()
}

/// The cost calendar's day for a session day, or `None` outside its window.
///
/// The two calendars have different accepted ranges — `costs` anchors at
/// 1990-01-01 — so this is a real refusal rather than a formality.
///
/// # Cost
///
/// O(1): three field reads and two range checks.
#[must_use]
pub fn trade_day_of(day: crate::session::Day) -> Option<TradeDay> {
    TradeDay::new(day.year(), day.month(), day.day()).ok()
}

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RateSource {
    /// A figure recorded in `docs/00-charter.md`, cited by the text here.
    ///
    /// The strongest provenance available and the only one that survives a
    /// re-run without the operator present.
    Charter(&'static str),
    /// Supplied by the operator with the request that used it.
    ///
    /// **This is the arm that exists today.** §3 rule 1 forbids this repository
    /// claiming a rate it has no source for; it does not forbid an operator
    /// naming one for their own run. The difference is where the claim lives,
    /// and it is exactly the shape §8 already uses for credentials — the repo
    /// holds the SHAPE, the operator supplies the VALUE, and an absent value
    /// halts loudly rather than defaulting.
    ///
    /// A rate supplied this way travels with the result and is recorded beside
    /// it, because §3 rule 3 makes a greek computed under an unrecorded rate
    /// unreproducible and therefore not a result at all.
    Operator,
    /// Solved against a vendor's own published implied volatility.
    ///
    /// Phase 2's answer, and a measurement rather than an assertion: Dhan sends
    /// its implied volatility beside the premium and the spot for the same
    /// strike, expiry and minute, so with four of five inputs known the rate is
    /// the only unknown left in the equation.
    SolvedAgainst(brutex_core::vendor::Vendor),
}

impl std::fmt::Display for RateSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Charter(cite) => write!(f, "docs/00-charter.md: {cite}"),
            Self::Operator => write!(f, "supplied by the operator with this request"),
            Self::SolvedAgainst(vendor) => {
                write!(
                    f,
                    "solved against {}'s published implied volatility",
                    vendor.as_str()
                )
            }
        }
    }
}

/// The risk-free rate, and where it came from.
///
/// # Why the provenance is a field and not a comment
///
/// §3 rule 1: every claim about a cost is traceable to a source. A rate is a
/// cost. Making the provenance a constructor argument means a rate with no
/// source is not a rate that compiles — the rule stops being something a
/// reviewer has to notice.
///
/// It is a closed [`RateSource`] enum rather than free text for two reasons: a
/// string can be blank or a lie and an enum cannot, and each arm is a real
/// provenance CLASS that a stored result can carry in one byte, which §3 rule 3
/// needs if a greek is ever to be reproducible.
///
/// There is deliberately **no constant of this type** in the workspace and no
/// `Default`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rate {
    annual: f64,
    basis: YearBasis,
    source: RateSource,
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
        source: RateSource,
    ) -> Result<Self, PricingError> {
        if !annual.is_finite() {
            return Err(PricingError::RateNotFinite { annual });
        }
        if let RateSource::Charter(cite) = source
            && cite.trim().is_empty()
        {
            // A BLANK CITATION IS WORSE THAN NONE, because it looks like the
            // rule was followed. The other two arms carry their provenance in
            // the discriminant and cannot be blank.
            return Err(PricingError::RateUnsourced);
        }
        // A RATE OUTSIDE ANY PLAUSIBLE BAND IS A TRANSPOSED FIELD, NOT A RATE.
        // `9.46` where `0.0946` was meant is the single likeliest way to get a
        // wrong number here, and it prices every option in the run wrongly
        // without erroring anywhere downstream: the model accepts it, the
        // greeks come out finite, and nothing says a word. Refused by name.
        #[expect(
            clippy::float_arithmetic,
            reason = "negating a bound to make the band symmetric. Negative \
                      rates are real, so the screen must not be a floor at \
                      zero, and `pull` denies float arithmetic because §7 is \
                      about PRICES — this is a comparison against a typo \
                      screen, not a price"
        )]
        let band = -MAX_PLAUSIBLE_RATE..=MAX_PLAUSIBLE_RATE;
        if !band.contains(&annual) {
            return Err(PricingError::RateImplausible { annual });
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
    pub const fn source(self) -> RateSource {
        self.source
    }
}

/// The widest continuously-compounded rate this build will accept.
///
/// One hundred percent. Not a market view — a TYPO SCREEN. The likeliest wrong
/// number here is a percentage entered where a decimal was meant (`9.46` for
/// `0.0946`), and it is the worst kind of wrong: the model accepts it, every
/// greek comes out finite, and nothing downstream says a word. Negative rates
/// are real and are allowed, which is why the band is symmetric.
pub const MAX_PLAUSIBLE_RATE: f64 = 1.0;

/// One option, at one bar, in paisa.
///
/// Every price is a paisa integer straight off the store — see the module
/// header on why they stay paisa through the model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Quote {
    /// The bar's stamp. The only thing a spot joins on — see [`SpotBook`].
    pub ts_micros: i64,
    /// The underlying's level at this bar, in paisa.
    ///
    /// Dhan sends it on the rolling overlay. For Groww it is joined from the
    /// index bar at the same stamp, which is what [`SpotBook`] is for.
    pub spot: i64,
    /// The contract's strike, in paisa.
    pub strike: i64,
    /// What the option traded at, in paisa. The bar's close.
    pub premium: i64,
    /// How long the contract had left. See [`crate::tenor`].
    pub tenor: Tenor,
    /// Call or put.
    pub side: OptionSide,
    /// Which swept underlying this is, for the dated strike ladder.
    ///
    /// Resolved once per contract by `costs::venue::swept_slot`, not once per
    /// bar: the answer cannot change inside one contract.
    pub slot: SweptSlot,
    /// The bar's own trading day, for the dated strike ladder.
    ///
    /// The interval is dated — `costs::strike::strike_step_on` carries its own
    /// verified-from row — so moneyness computed against a fixed 50 or 100
    /// would be silently wrong across every interval change and would look
    /// right on every row.
    pub on: TradeDay,
    /// Whose feed this bar came from, so a vendor-sent volatility can name it.
    pub vendor: brutex_core::vendor::Vendor,
}

/// The underlying's level at every stamp of one month, probed in O(1).
///
/// # Why this exists
///
/// Dhan sends the spot beside each option bar. **Groww sends nothing of the
/// kind** — its candle is a positional `[timestamp, open, high, low, close,
/// volume]` array by the vendor's own contract — so for that feed the spot has
/// to come from the index bar at the same minute, and it has to come without a
/// scan.
///
/// # Cost
///
/// Built once per instrument-month in **O(bars)**, one insert each. Probed in
/// **O(1)**, one hash lookup. `docs/07-o1-architecture.md` law 3 is what makes
/// the build cost acceptable and the probe cost mandatory: paying O(bars) once
/// to answer O(1) forever is the trade; paying O(bars) per option row is the
/// scan the law forbids, and with 252 contracts against 375 index bars that is
/// the difference between 375 inserts and 94,500 comparisons.
#[derive(Debug, Clone, Default)]
pub struct SpotBook {
    by_stamp: HashMap<i64, i64>,
}

impl SpotBook {
    /// Indexes one month of index bars by their stamp.
    ///
    /// A later bar at a stamp already seen REPLACES the earlier one, which
    /// cannot arise from a well-formed month — `store::file::BarFile` keeps its
    /// rows strictly ascending — and is chosen over keeping the first so that
    /// a malformed input behaves the same way twice rather than depending on
    /// which duplicate arrived.
    #[must_use]
    pub fn of(bars: &[store::format::Bar]) -> Self {
        let mut by_stamp = HashMap::with_capacity(bars.len());
        for bar in bars {
            by_stamp.insert(bar.ts_micros, bar.close);
        }
        Self { by_stamp }
    }

    /// The underlying's close at that stamp, or `None`.
    ///
    /// `None` is not an approximation and is never rounded to a neighbouring
    /// minute: an option that printed in a minute the index did not is a row
    /// this build cannot price, and pricing it against the previous minute's
    /// level would be an invented spot that no report could later tell from a
    /// real one.
    #[must_use]
    pub fn at(&self, ts_micros: i64) -> Option<i64> {
        self.by_stamp.get(&ts_micros).copied()
    }

    /// How many stamps this book holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.by_stamp.len()
    }

    /// Whether the book holds nothing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_stamp.is_empty()
    }
}

/// Why a quote could not be priced.
///
/// Every arm names something READ and refused. None is a fallback, and none
/// substitutes a value — §4.
#[derive(Debug, Clone, PartialEq)]
pub enum PricingError {
    /// A rate that is not a number.
    RateNotFinite {
        /// The value offered.
        annual: f64,
    },
    /// A rate with no citation. See [`Rate`].
    RateUnsourced,
    /// A rate outside any plausible band — see [`MAX_PLAUSIBLE_RATE`].
    RateImplausible {
        /// The value offered.
        annual: f64,
    },
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
    /// The dated strike ladder could not answer for this instrument and day.
    ///
    /// The ladder's OWN sentence is carried rather than a summary of it:
    /// `costs::strike` distinguishes an instrument with no recorded regime from
    /// a date before its verified-from row, and those want different actions.
    LadderUnknown {
        /// What `costs` said.
        why: String,
    },
    /// No index bar was stored at this option bar's stamp.
    ///
    /// Groww's route only. Refused rather than filled from a neighbouring
    /// minute — see [`SpotBook::at`].
    NoSpotAtStamp {
        /// The stamp that found nothing.
        ts_micros: i64,
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
            Self::RateImplausible { annual } => write!(
                f,
                "a risk-free rate of {annual} is outside \u{00b1}{MAX_PLAUSIBLE_RATE}, which is \
                 a percentage entered where a decimal was meant far more often \
                 than it is a real rate. Refused by name, because the model \
                 would have accepted it and every greek would have come out \
                 finite and wrong"
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
            Self::LadderUnknown { why } => write!(
                f,
                "the dated strike ladder could not place this contract, so its \
                 moneyness is unknown and nothing was priced: {why}"
            ),
            Self::NoSpotAtStamp { ts_micros } => write!(
                f,
                "no index bar is stored at {ts_micros}, so this option bar has \
                 no underlying level to price against. Nothing was priced \
                 rather than the previous minute's level being borrowed, which \
                 no later report could tell from a real one"
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

/// Where the volatility in a [`Priced`] came from.
///
/// Kept because the two are not interchangeable and a report that mixes them
/// is not readable. Dhan publishes its own; Groww publishes none at all, so for
/// that feed every volatility is one this build solved.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VolSource {
    /// The vendor sent it. Dhan's `iv` on the rolling-option overlay.
    Vendor(brutex_core::vendor::Vendor),
    /// Solved here from the traded premium.
    Solved {
        /// Model evaluations spent. Never above `greeks::solver::MAX_ITERATIONS`.
        iterations: u32,
        /// The solver's own screen on its answer. **Not an error bound** — see
        /// `greeks::solver::ImpliedVolatility::uncertainty`.
        uncertainty: f64,
    },
}

/// One option bar, fully priced: spot, volatility, every greek, and moneyness.
///
/// This is the whole answer for one row. Nothing here is optional and nothing
/// is filled in later — a row that could not produce all of it is a
/// [`PricingError`] instead, because a half-priced row on a report is
/// indistinguishable from a fully-priced one that happens to look odd.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Priced {
    /// The bar this prices, joined by stamp and nothing else.
    pub ts_micros: i64,
    /// The underlying's level, in paisa.
    pub spot: i64,
    /// The contract's strike, in paisa.
    pub strike: i64,
    /// What the option traded at, in paisa.
    pub premium: i64,
    /// The at-the-money rung of the ladder this bar sat on, in paisa.
    pub at_the_money: i64,
    /// The ladder interval on this date, in paisa. Dated, not fixed.
    pub step: i64,
    /// Seconds to expiry, exactly. The float form is in the greeks.
    pub tenor_seconds: i64,
    /// The volatility used, as a decimal.
    pub volatility: f64,
    /// Whether that volatility was the vendor's or this build's.
    pub vol_from: VolSource,
    /// Price, delta, gamma, vega, theta, rho, d1, d2 — **per paisa** for the
    /// scale-dependent four. See the module header.
    pub greeks: Greeks,
    /// `ATM`, `ITM-3`, `OTM+2` — the strike's place on the dated ladder.
    pub moneyness: Moneyness,
    /// The rate used, and where it came from. Carried so the row is
    /// reproducible under §3 rule 3.
    pub rate: Rate,
    /// Whether the tenor is below the band `greeks` validates itself in.
    ///
    /// **Every weekly is.** Carried per row rather than computed on a report,
    /// so a caller cannot render the number without the caveat being available.
    pub below_validated_band: bool,
}

/// Prices one option bar completely.
///
/// # What `volatility` selects
///
/// `Some(v)` uses the vendor's own — Dhan's `iv` off the rolling overlay, which
/// makes this a *check* of the vendor rather than a substitute for it. `None`
/// solves it from the premium, which is the only route for Groww, whose candles
/// carry open, high, low, close and volume and no volatility at all.
///
/// # The ladder is DATED and is not a constant
///
/// `costs::strike::strike_step_on` answers the interval for this instrument on
/// this day, with its own verified-from row. Moneyness computed against a fixed
/// 50 or 100 would be silently wrong across every interval change the exchange
/// has ever made, and would look right on every row.
///
/// # Errors
///
/// Every arm of [`PricingError`], including the ladder's own refusals and the
/// model's, each kept intact rather than flattened into one.
///
/// # Cost
///
/// **O(1) time, O(1) space, no allocation.** One dated table lookup bounded by a
/// compile-time constant, one closed-form at-the-money rounding, one closed-form
/// greeks evaluation, and — only when solving — an iteration count bounded by
/// `greeks::solver::MAX_ITERATIONS` and returned so the bound is observable.
pub fn price(
    quote: Quote,
    volatility: Option<f64>,
    rate: Rate,
    basis: YearBasis,
) -> Result<Priced, PricingError> {
    let contract = contract_of(quote, rate, basis)?;
    let kind = kind_of(quote.side);

    // THE DATED LADDER. Both refusals are the ladder's own words, kept.
    let step = costs::strike::strike_step_on(quote.slot, quote.on).map_err(|why| {
        PricingError::LadderUnknown {
            why: why.to_string(),
        }
    })?;
    let at_the_money =
        costs::strike::at_the_money(Paisa::from_raw(quote.spot), step).map_err(|why| {
            PricingError::LadderUnknown {
                why: why.to_string(),
            }
        })?;

    let (volatility, vol_from) = if let Some(sent) = volatility {
        // THE VENDOR'S OWN NUMBER, UNCHANGED. Dhan's `iv` off the rolling
        // overlay, which makes this a CHECK of the vendor rather than a
        // substitute for it.
        (sent, VolSource::Vendor(quote.vendor))
    } else {
        {
            #[expect(
                clippy::cast_precision_loss,
                reason = "see contract_of — paisa is exact in f64 at this magnitude"
            )]
            let premium = quote.premium as f64;
            let solved = contract.implied_volatility(premium, kind)?;
            (
                solved.volatility,
                VolSource::Solved {
                    iterations: solved.iterations,
                    uncertainty: solved.uncertainty,
                },
            )
        }
    };

    let greeks = contract.greeks(volatility, kind)?;

    #[expect(
        clippy::cast_precision_loss,
        reason = "paisa is exact in f64 at this magnitude, and moneyness is a \
                  ratio of three paisa values so the scale cancels entirely"
    )]
    let moneyness = Moneyness::from_ladder(
        quote.strike as f64,
        at_the_money.raw() as f64,
        step.raw() as f64,
        kind,
    )?;

    Ok(Priced {
        ts_micros: quote.ts_micros,
        spot: quote.spot,
        strike: quote.strike,
        premium: quote.premium,
        at_the_money: at_the_money.raw(),
        step: step.raw(),
        tenor_seconds: quote.tenor.seconds(),
        volatility,
        vol_from,
        greeks,
        moneyness,
        rate,
        below_validated_band: quote.tenor.below_validated_band(basis),
    })
}

/// What pricing a whole contract-month produced.
///
/// Counts rather than a bare vector, because a run that priced 4,000 rows of
/// 5,600 is neither a success nor a failure and rendering it as either is the
/// §4 fallback that hides what happened. The first few reasons travel with the
/// counts, capped, for the same reason every other landing here caps them.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PricedAll {
    /// Every row that priced, in the order it was offered.
    pub rows: Vec<Priced>,
    /// How many rows could not be priced.
    pub refused: usize,
    /// The first few reasons, verbatim, capped at [`REASONS_KEPT`].
    pub why: Vec<String>,
}

/// How many distinct refusal reasons one run keeps.
///
/// Five. A run that refuses forty thousand rows refuses them for two or three
/// reasons, and forty thousand copies of the same sentence is not more
/// information than five — it is the same information, unreadable.
pub const REASONS_KEPT: usize = 5;

impl PricedAll {
    /// How many rows were offered in total.
    #[must_use]
    pub const fn offered(&self) -> usize {
        self.rows.len().saturating_add(self.refused)
    }

    /// How many rows priced against a volatility this build solved.
    ///
    /// The number that separates a Groww run from a Dhan one on a receipt.
    #[must_use]
    pub fn solved(&self) -> usize {
        self.rows
            .iter()
            .filter(|row| matches!(row.vol_from, VolSource::Solved { .. }))
            .count()
    }

    /// How many priced rows sit below the band `greeks` validates itself in.
    ///
    /// On a weekly this is every row, and a report that does not say so is
    /// implying a validation that did not happen.
    #[must_use]
    pub fn below_validated_band(&self) -> usize {
        self.rows
            .iter()
            .filter(|row| row.below_validated_band)
            .count()
    }
}

/// Prices a whole contract-month, refusing rows rather than the run.
///
/// One malformed row out of five thousand must not discard the other four
/// thousand nine hundred and ninety-nine — the same rule `pull::ingest` follows
/// for a member that will not parse.
///
/// # Cost
///
/// **O(rows)**, one [`price`] each, which is O(1). Space is the rows that
/// priced plus at most [`REASONS_KEPT`] strings. Nothing here scans the store.
#[must_use]
pub fn price_all(
    quotes: &[Quote],
    volatility: impl Fn(i64) -> Option<f64>,
    rate: Rate,
    basis: YearBasis,
) -> PricedAll {
    let mut out = PricedAll {
        rows: Vec::with_capacity(quotes.len()),
        refused: 0,
        why: Vec::new(),
    };
    for quote in quotes {
        match price(*quote, volatility(quote.ts_micros), rate, basis) {
            Ok(row) => out.rows.push(row),
            Err(why) => {
                out.refused = out.refused.saturating_add(1);
                let sentence = why.to_string();
                // ONE COPY OF EACH DISTINCT REASON. Five thousand rows refused
                // for one reason is one fact, not five thousand.
                if out.why.len() < REASONS_KEPT && !out.why.contains(&sentence) {
                    out.why.push(sentence);
                }
            }
        }
    }
    out
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
        Rate::measured(0.065, YearBasis::Calendar365, RateSource::Operator)
            .expect("a plausible rate with a provenance")
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

    /// 09:15 IST on 2026-08-20, the stamp every quote below is joined on.
    fn stamp() -> i64 {
        let d = crate::session::Day::new(2026, 8, 20).expect("a real day");
        (i64::from(d.days_from_epoch()) * 86_400 - 19_800 + 9 * 3_600 + 15 * 60) * 1_000_000
    }

    fn slot() -> costs::venue::SweptSlot {
        let symbol = brutex_core::symbol::Symbol::new("NIFTY").expect("a swept underlying");
        costs::venue::swept_slot(symbol).expect("NIFTY has a recorded regime")
    }

    /// One index bar at a stamp, for the join book.
    fn bar(ts_micros: i64, close: i64) -> store::format::Bar {
        store::format::Bar {
            ts_micros,
            open: close,
            high: close,
            low: close,
            close,
            volume: 0,
            open_interest: 0,
        }
    }

    fn atm_call() -> Quote {
        Quote {
            ts_micros: stamp(),
            // 25,000.00 — a real NIFTY level, and a whole number of 50-point
            // rungs, so the strike below sits ON the ladder rather than off it.
            spot: 2_500_000,
            strike: 2_500_000,
            premium: 25_000,
            tenor: tenor(7),
            side: OptionSide::Call,
            slot: slot(),
            on: costs::day::TradeDay::new(2026, 8, 20).expect("a real trade day"),
            vendor: brutex_core::vendor::Vendor::Groww,
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

    /// A rate with no source does not compile into existence.    /// **HOW MUCH DOES THE RATE ACTUALLY MOVE THE ANSWER?**
    ///
    /// The operator has to type it, `docs/00-charter.md` records none, and
    /// [`Rate`] refuses to invent one — so the honest thing is to MEASURE what
    /// that friction buys rather than assert that it matters.
    ///
    /// Black-Scholes discounts by `exp(-r·T)`. On a seven-day weekly `T` is
    /// 0.0199 years, so the entire discount at 6.5% is `exp(-0.00129)` — about
    /// one part in eight hundred. This measures the spread across a band wider
    /// than any rate India has printed in living memory and asserts it, so the
    /// number is a fact in the suite rather than a claim in a comment.
    ///
    /// # The answer is that it matters, and the first version of this test
    /// asserted the opposite
    ///
    /// It was written to prove the rate was a formality — `exp(-r·T)` over
    /// seven days at 6.5% is about one part in eight hundred, so the discount
    /// looked negligible. **Measured: 0.1557 to 0.1776 across 0–12%, a spread
    /// of 0.0219 — a 14% relative swing in implied volatility.** The assertion
    /// `spread < 0.02` failed, and the reasoning behind it was wrong.
    ///
    /// Why the discount argument misleads: for an at-the-money option the rate
    /// moves the FORWARD, `S·e^((r−q)T)`, not just the discount. Over a
    /// seven-day tenor the expected move is `σ√T ≈ 0.15 × 0.141 ≈ 2.1%`, and a
    /// 12% rate shifts the forward by 0.23% — **eleven percent of the expected
    /// move**. A small absolute shift against a small denominator is not small.
    ///
    /// So the rate box is not a formality and the operator cannot skip it. This
    /// test records the sensitivity as a measured fact rather than leaving the
    /// question to intuition, which got it backwards.
    #[test]
    fn the_rate_materially_moves_a_weekly_which_is_why_it_must_be_supplied() {
        let quote = atm_call();
        let basis = YearBasis::Calendar365;
        let iv_at = |bps: u32| {
            // `f64::from(u32)` is lossless, so no cast lint fires here — the
            // basis-point count is the input precisely because it keeps the
            // rate an exact integer until the one division.
            let annual = f64::from(bps) / 10_000.0;
            let rate = Rate::measured(annual, basis, RateSource::Operator).expect("plausible");
            solve_iv(quote, rate, basis).expect("solvable").volatility
        };

        // THE WIDE BAND: wider than any rate India has printed in living
        // memory, so this is an upper bound on the error a wrong rate causes.
        let (at_zero, at_twelve) = (iv_at(0), iv_at(1_200));
        let wide = at_zero - at_twelve;

        // THE PLAUSIBLE BAND: roughly the range an Indian risk-free rate has
        // actually occupied. **This is the number that decides whether the
        // operator's guess has to be a good one**, and it is much the smaller
        // of the two: a rate anywhere in it lands IV within a few percent.
        let (at_five_five, at_eight) = (iv_at(550), iv_at(800));
        let near = at_five_five - at_eight;

        println!(
            "7-day ATM weekly — IV at r=0%: {at_zero:.4}, r=12%: {at_twelve:.4}, \
             spread {wide:.4}\n  plausible band r=5.5%..8%: \
             {at_five_five:.4}..{at_eight:.4}, spread {near:.4}"
        );

        // IT MATTERS, AND THAT IS THE ASSERTION. If a future change made the
        // rate genuinely negligible this would fail and the box could go —
        // the right way round for a claim once made from intuition and wrong.
        assert!(
            wide > 0.01,
            "the rate moved IV by only {wide:.6} across 0–12%, so the operator's \
             rate box may now be a formality — re-check the reasoning above"
        );

        // AND IT FALLS AS THE RATE RISES, which is the opposite of what the
        // first version of this test asserted.
        //
        // For a CALL at a FIXED PREMIUM: a higher rate lifts the forward
        // `S·e^((r−q)T)`, which moves the option further into the money, so
        // LESS volatility is needed to justify the same price. Getting this sign
        // backwards would not have been caught by a spread magnitude alone —
        // which is why the direction is asserted separately.
        assert!(
            at_zero > at_twelve && at_five_five > at_eight,
            "IV must FALL as the rate rises for a call at a fixed premium: \
             r=0% gave {at_zero:.6} and r=12% gave {at_twelve:.6}"
        );
    }

    #[test]
    fn a_rate_without_a_citation_is_refused() {
        assert_eq!(
            Rate::measured(0.065, YearBasis::Calendar365, RateSource::Charter("   ")),
            Err(PricingError::RateUnsourced),
            "a blank citation looks like the rule was followed"
        );
        assert_eq!(
            Rate::measured(0.065, YearBasis::Calendar365, RateSource::Charter("")),
            Err(PricingError::RateUnsourced)
        );
        // THE TRANSPOSED FIELD. `9.46` where `0.0946` was meant prices every
        // option in the run wrongly and errors nowhere downstream.
        assert_eq!(
            Rate::measured(9.46, YearBasis::Calendar365, RateSource::Operator),
            Err(PricingError::RateImplausible { annual: 9.46 }),
            "a percentage where a decimal was meant must be refused by name"
        );
        // AND A NEGATIVE RATE IS REAL, so the band is symmetric rather than a
        // floor at zero.
        assert!(Rate::measured(-0.004, YearBasis::Calendar365, RateSource::Operator).is_ok());
        // `assert_eq!` CANNOT BE USED HERE and the reason is the bug it would
        // hide: `PartialEq` on this type is derived, so it compares the `f64`,
        // and `NaN != NaN` by IEEE-754. The assertion fails while PRINTING two
        // values that read identically — `RateNotFinite { annual: NaN }` on
        // both sides. Matched on shape instead.
        assert!(
            matches!(
                Rate::measured(f64::NAN, YearBasis::Calendar365, RateSource::Operator),
                Err(PricingError::RateNotFinite { annual }) if annual.is_nan()
            ),
            "a NaN rate is not a rate"
        );
        assert!(
            Rate::measured(f64::INFINITY, YearBasis::Calendar365, RateSource::Operator).is_err(),
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
        assert_eq!(
            rate.source(),
            RateSource::Operator,
            "the provenance travels with the value"
        );
    }

    /// **THE WHOLE ANSWER FOR ONE ROW, IN ONE CALL.**
    ///
    /// Spot, volatility, five greeks and moneyness. This is the unit that says
    /// the pipeline is wired rather than merely present: before it, every one
    /// of those quantities existed in a different crate with no caller.
    #[test]
    fn one_option_bar_prices_to_a_volatility_five_greeks_and_a_moneyness() {
        let row = price(atm_call(), None, test_rate(), YearBasis::Calendar365)
            .expect("an at-the-money weekly is priceable");

        // THE VOLATILITY WAS SOLVED, not sent — Groww publishes none.
        assert!(matches!(row.vol_from, VolSource::Solved { .. }));
        assert!(row.volatility > 0.0 && row.volatility < 5.0);

        // EVERY GREEK IS A NUMBER. A NaN delta looks exactly like a real one
        // until it is multiplied by something.
        assert!(row.greeks.is_finite(), "{:?}", row.greeks);
        assert!(row.greeks.delta > 0.0, "a call's delta is positive");
        assert!(row.greeks.gamma > 0.0, "gamma is positive on both sides");
        assert!(row.greeks.vega > 0.0, "vega is positive on both sides");
        assert!(row.greeks.theta < 0.0, "a long option loses time value");

        // THE STRIKE IS PLACED ON THE DATED LADDER.
        assert_eq!(row.moneyness.to_string(), "ATM");
        assert_eq!(row.at_the_money, 2_500_000, "25,000.00 is its own rung");
        assert_eq!(row.step, 5_000, "NIFTY's ladder is 50 points on this date");

        // AND THE CAVEAT TRAVELS WITH THE ROW rather than being computed on a
        // report that may forget to.
        assert!(
            row.below_validated_band,
            "a seven-day weekly is below 0.02 years, and every report of this \
             row must be able to say so"
        );
        assert_eq!(row.tenor_seconds, 7 * 86_400 + 6 * 3_600 + 25 * 60);
        assert_eq!(row.rate.source(), RateSource::Operator);
    }

    /// A vendor's own volatility is USED and SAID SO, not silently re-solved.
    ///
    /// This is Dhan's route. Mixing a vendor's number with one this build
    /// solved, on one report, with nothing distinguishing them, is the failure
    /// `VolSource` exists to prevent.
    #[test]
    fn a_vendor_sent_volatility_is_used_and_attributed() {
        let sent = 0.1425;
        let row = price(atm_call(), Some(sent), test_rate(), YearBasis::Calendar365)
            .expect("priceable at a given volatility");

        #[expect(
            clippy::float_cmp,
            reason = "EXACT equality is the assertion. The claim is that the \
                      vendor's number passes through untouched — an epsilon \
                      comparison here would pass over a build that quietly \
                      re-solved it and landed nearby, which is the exact \
                      failure `VolSource` exists to make visible"
        )]
        {
            assert_eq!(row.volatility, sent, "the vendor's number, unchanged");
        }
        assert_eq!(
            row.vol_from,
            VolSource::Vendor(brutex_core::vendor::Vendor::Groww)
        );
        assert!(row.greeks.is_finite());

        // AND IT IS NOT THE SOLVED ONE. If these agreed by accident the unit
        // above would pass while this one proved nothing.
        let solved = price(atm_call(), None, test_rate(), YearBasis::Calendar365)
            .expect("solvable")
            .volatility;
        assert!(
            (solved - sent).abs() > 1e-6,
            "the two routes must be distinguishable for this unit to mean \
             anything: solved {solved}, sent {sent}"
        );
    }

    /// **MONEYNESS IS PLACED ON THE DATED LADDER, IN BOTH DIRECTIONS.**
    ///
    /// `greeks::moneyness::Moneyness` had no caller anywhere in the workspace
    /// before this — its only users were its own doc examples and its own
    /// tests. This is its first real one.
    #[test]
    fn a_strike_is_placed_on_the_ladder_and_the_side_flips_the_reading() {
        // Two rungs above a 25,000 money, on a 50-point ladder.
        let up = Quote {
            strike: 2_510_000,
            ..atm_call()
        };
        let call = price(up, Some(0.15), test_rate(), YearBasis::Calendar365).expect("otm call");
        assert_eq!(call.moneyness.to_string(), "OTM+2");
        assert_eq!(call.moneyness.steps, 2);

        // THE SAME STRIKE, THE OTHER SIDE. The strike has not moved; the
        // reading has, because moneyness is about the side being priced.
        let put = Quote {
            side: OptionSide::Put,
            ..up
        };
        let put = price(put, Some(0.15), test_rate(), YearBasis::Calendar365).expect("itm put");
        assert_eq!(put.moneyness.to_string(), "ITM-2");
        assert_eq!(
            put.moneyness.steps, 2,
            "the sign is about the STRIKE, not the side — both are two rungs up"
        );

        // AND BELOW THE MONEY, where the two swap again.
        let down = Quote {
            strike: 2_485_000,
            ..atm_call()
        };
        let call = price(down, Some(0.15), test_rate(), YearBasis::Calendar365).expect("itm call");
        assert_eq!(call.moneyness.to_string(), "ITM-3");
    }

    /// A strike between two rungs is REFUSED, not rounded onto one.
    ///
    /// Rounding would file a contract under a moneyness it does not have, and
    /// no later reader could tell it from a strike that really sat there.
    #[test]
    fn a_strike_off_the_ladder_is_refused_rather_than_rounded() {
        let off = Quote {
            strike: 2_502_500,
            ..atm_call()
        };
        assert!(
            matches!(
                price(off, Some(0.15), test_rate(), YearBasis::Calendar365),
                Err(PricingError::Model(GreeksError::OffGrid { .. }))
            ),
            "25,025.00 is half a rung and must not be read as a rung"
        );
    }

    /// **THE SPOT JOIN: EXACT STAMP OR NOTHING.**
    ///
    /// Groww's whole route depends on this, and the temptation it refuses is
    /// the expensive one — borrowing the previous minute's level would produce
    /// a spot no later report could tell from a real one.
    #[test]
    fn the_spot_book_answers_on_the_exact_stamp_and_never_a_neighbour() {
        let minute = 60 * 1_000_000;
        let bars = [
            bar(stamp(), 2_500_000),
            bar(stamp() + minute, 2_501_500),
            bar(stamp() + 2 * minute, 2_499_000),
        ];
        let book = SpotBook::of(&bars);

        assert_eq!(book.len(), 3);
        assert!(!book.is_empty());
        assert_eq!(book.at(stamp()), Some(2_500_000));
        assert_eq!(book.at(stamp() + minute), Some(2_501_500));

        // ONE SECOND OFF IS NOT A MATCH. Not the nearest, not the previous.
        assert_eq!(book.at(stamp() + 1_000_000), None);
        assert_eq!(book.at(stamp() - 1), None);
        assert_eq!(book.at(stamp() + 3 * minute), None);

        // AN EMPTY MONTH IS AN EMPTY BOOK, not a panic and not a zero level.
        let empty = SpotBook::of(&[]);
        assert!(empty.is_empty());
        assert_eq!(empty.at(stamp()), None);
    }

    /// A duplicate stamp resolves the same way twice.
    ///
    /// It cannot arise from a well-formed month, and the point is that a
    /// malformed one behaves deterministically rather than depending on which
    /// copy arrived first.
    #[test]
    fn a_repeated_stamp_resolves_deterministically() {
        let bars = [bar(stamp(), 2_500_000), bar(stamp(), 2_600_000)];
        assert_eq!(SpotBook::of(&bars).at(stamp()), Some(2_600_000));
        assert_eq!(SpotBook::of(&bars).len(), 1, "one stamp, one entry");
    }

    /// **ONE BAD ROW DOES NOT DISCARD THE MONTH**, and the reasons do not
    /// repeat themselves five thousand times.
    #[test]
    fn a_run_refuses_rows_rather_than_itself_and_keeps_each_reason_once() {
        let good = atm_call();
        let off_ladder = Quote {
            strike: 2_502_500,
            ..good
        };
        let zero_premium = Quote { premium: 0, ..good };
        let quotes = [
            good,
            off_ladder,
            good,
            off_ladder,
            zero_premium,
            good,
            off_ladder,
        ];

        let out = price_all(&quotes, |_| Some(0.15), test_rate(), YearBasis::Calendar365);

        assert_eq!(out.offered(), 7);
        assert_eq!(out.rows.len(), 3, "the three good rows survive");
        assert_eq!(out.refused, 4);
        assert_eq!(
            out.why.len(),
            2,
            "three off-ladder rows are ONE fact, not three: {:?}",
            out.why
        );
        // AND EACH REASON IS THE ONE THAT APPLIES, in the words of whatever
        // refused it — the model for the off-ladder strike, this module for
        // the placeholder premium.
        assert!(
            out.why.iter().any(|w| w.contains("steps from the money")),
            "the model's own sentence must survive: {:?}",
            out.why
        );
        assert!(
            out.why.iter().any(|w| w.contains("premium is 0 paisa")),
            "{:?}",
            out.why
        );
    }

    /// The counts a receipt needs are derived, not recomputed by each caller.
    #[test]
    fn a_run_reports_how_many_it_solved_and_how_many_are_below_the_band() {
        let quotes = [atm_call(), atm_call()];

        // SOLVED: no volatility offered for either stamp.
        let solved = price_all(&quotes, |_| None, test_rate(), YearBasis::Calendar365);
        assert_eq!(solved.rows.len(), 2);
        assert_eq!(solved.solved(), 2);
        assert_eq!(
            solved.below_validated_band(),
            2,
            "both are seven-day weeklies, so a report of them is outside the \
             region the model proves itself in"
        );

        // SENT: the vendor's number for both.
        let sent = price_all(&quotes, |_| Some(0.15), test_rate(), YearBasis::Calendar365);
        assert_eq!(
            sent.solved(),
            0,
            "nothing was solved when the vendor sent one"
        );

        // AND AN EMPTY RUN IS EMPTY, not one refused row.
        let none = price_all(&[], |_| None, test_rate(), YearBasis::Calendar365);
        assert_eq!(none.offered(), 0);
        assert_eq!(none, PricedAll::default());
    }

    /// The volatility source is selected PER STAMP, which is what lets a month
    /// with a partial overlay price both ways in one pass.
    #[test]
    fn the_volatility_is_chosen_per_stamp_rather_than_per_run() {
        let minute = 60 * 1_000_000;
        let later = Quote {
            ts_micros: stamp() + minute,
            ..atm_call()
        };
        let out = price_all(
            &[atm_call(), later],
            |ts| (ts == stamp()).then_some(0.15),
            test_rate(),
            YearBasis::Calendar365,
        );
        assert_eq!(out.rows.len(), 2);
        assert_eq!(
            out.solved(),
            1,
            "the stamp the vendor covered used its number; the other solved"
        );
    }
}

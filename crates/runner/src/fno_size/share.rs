//! SIZE-SPLIT-ALL's and SIZE-RISK's share arithmetic, exact in integers (D-8632, D-8633).
//!
//! For a batch with `e` eligible stocks:
//!
//! * the trade room is `trades_per_day` less `trades_today` less `queued`, saturating at 0;
//! * the divisor is `k = min(e, room)` when the room is above 0, and `k = e` otherwise;
//! * the free margin `F` is `min(capital, margin_cap) x 10^6` less `margin_in_use` less
//!   `reserved_margin`, floored to paise;
//! * the exposure room `X` is `exposure_cap` less `exposure` less `reserved_exposure`, paise;
//! * the equal shares are `m = floor(F / k)` and `x = floor(X / k)`; the remainders stay free.
//!
//! Every quantity is a floor, never SPEC 11.2's nearest integer: the share is a budget, and a
//! rounded-up quantity could pass the free margin or the exposure room. A share that buys no
//! whole share is refused and its money stays free: there is no re-split.
//!
//! Every overflow is found here, before a row is written: the free margin's product and
//! differences, the exposure room's differences, and an exposure share past `i64::MAX` paise,
//! the largest value a position can hold. A row's quantity is at most `floor(x / price)`, so
//! its value is at most `x` and the row arithmetic below is exact and cannot fail.

use super::{
    AccountView, Bound, Candidate, Invalid, Limits, Outcome, PPM, PolicyInput, RiskBudget,
    SizeRefusal, Sized,
};

/// How every eligible row of one batch is sized.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Rule {
    /// FNO-NET's quantity, the FIXED policies.
    Fixed,
    /// Every eligible row is refused with this word.
    Refuse(SizeRefusal),
    /// SIZE-SPLIT-ALL's equal shares.
    Split(Shares),
    /// SIZE-RISK's equal risk on equal shares.
    Risk {
        /// The equal shares.
        shares: Shares,
        /// Paise: the risk each, `b`, at least 1.
        each: u128,
    },
}

/// One batch's equal shares, as the row arithmetic reads them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Shares {
    /// Millionths of a paisa: `m x 10^6`.
    margin_micro: u128,
    /// Paise: `x`, at most `i64::MAX`.
    exposure: u64,
}

/// One batch's figures and the rule its rows are sized by.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Figures {
    /// The share divisor.
    pub(super) k: u16,
    /// Paise: `floor(F / k)`.
    pub(super) share_margin: i128,
    /// Paise: `floor(X / k)`.
    pub(super) share_exposure: i128,
    /// Paise: `b`.
    pub(super) risk_each: i128,
    /// How the eligible rows are sized.
    rule: Rule,
}

impl Figures {
    /// The FIXED policies' figures, and those of a batch with no eligible stock: no divisor,
    /// no shares.
    const fn none(rule: Rule) -> Self {
        Self {
            k: 0,
            share_margin: 0,
            share_exposure: 0,
            risk_each: 0,
            rule,
        }
    }

    /// The batch's figures under `input`.
    pub(super) fn of(
        input: &PolicyInput<'_>,
        view: &AccountView,
        limits: &Limits,
        eligible: u16,
    ) -> Result<Self, Invalid> {
        let budget = match input {
            PolicyInput::FixedSeeded | PolicyInput::FixedRanked { .. } => {
                return Ok(Self::none(Rule::Fixed));
            }
            PolicyInput::SplitAll => None,
            PolicyInput::Risk { budget } => Some(*budget),
        };
        if eligible == 0 {
            // Every row is busy and refused `while_open`; nothing is divided.
            return Ok(Self::none(Rule::Refuse(SizeRefusal::WhileOpen)));
        }
        let room = limits
            .trades_per_day
            .saturating_sub(view.trades_today)
            .saturating_sub(view.queued);
        let k = if room > 0 {
            eligible.min(room)
        } else {
            eligible
        };
        let divisor = i128::from(k);
        let free = view
            .capital
            .min(limits.margin_cap)
            .checked_mul(PPM)
            .and_then(|cap| cap.checked_sub(view.margin_in_use))
            .and_then(|left| left.checked_sub(view.reserved_margin))
            .ok_or(Invalid::Overflow("free margin"))?
            .div_euclid(PPM);
        let room_x = limits
            .exposure_cap
            .checked_sub(view.exposure)
            .and_then(|left| left.checked_sub(view.reserved_exposure))
            .ok_or(Invalid::Overflow("exposure room"))?;
        let share_margin = free.div_euclid(divisor);
        let share_exposure = room_x.div_euclid(divisor);
        let exposure = i64::try_from(share_exposure)
            .map_err(|_| Invalid::Overflow("exposure share"))?
            .unsigned_abs();
        let mut figures = Self {
            k,
            share_margin,
            share_exposure,
            risk_each: 0,
            rule: Rule::Refuse(SizeRefusal::NoFreeMargin),
        };
        if free <= 0 {
            return Ok(figures);
        }
        if room_x <= 0 {
            figures.rule = Rule::Refuse(SizeRefusal::NoExposureRoom);
            return Ok(figures);
        }
        // `m` is at most F, and F x 10^6 is at most the checked product above, so this is exact.
        let shares = Shares {
            margin_micro: share_margin.unsigned_abs() * PPM.unsigned_abs(),
            exposure,
        };
        (figures.rule, figures.risk_each) = match budget {
            None => (Rule::Split(shares), 0),
            Some(budget) => risk_rule(budget, view.open_risk, divisor, shares),
        };
        Ok(figures)
    }

    /// One eligible candidate's outcome. It cannot fail: every overflow was refused in
    /// [`Figures::of`] and in the candidate's own check.
    pub(super) fn size(&self, candidate: &Candidate) -> Outcome {
        match self.rule {
            Rule::Fixed => fixed(candidate),
            Rule::Refuse(word) => Outcome::Refused(word),
            Rule::Split(shares) => {
                let (qty, bound) = shares.quantity(candidate);
                sized(candidate, qty, bound, None)
            }
            Rule::Risk { shares, each } => risky(candidate, shares, each),
        }
    }
}

/// SIZE-RISK's rule once the shares are known, and its risk each: no learned risk, no risk
/// room, or `b = min(per_trade, floor(Rr / k))` with `Rr = open_cap - open_risk`. A room above
/// 0 but below `k` paise gives `b = 0`, and each row then buys no share after its stop checks.
fn risk_rule(
    budget: Option<RiskBudget>,
    open_risk: i128,
    divisor: i128,
    shares: Shares,
) -> (Rule, i128) {
    let Some(budget) = budget else {
        return (Rule::Refuse(SizeRefusal::NoLearnedRisk), 0);
    };
    // `open_cap` is above 0 and `open_risk` at least 0, so the difference is exact.
    let room = budget.open_cap() - open_risk;
    if room <= 0 {
        return (Rule::Refuse(SizeRefusal::NoRiskRoom), 0);
    }
    let each = budget.per_trade().min(room.div_euclid(divisor));
    (
        Rule::Risk {
            shares,
            each: each.unsigned_abs(),
        },
        each,
    )
}

impl Shares {
    /// `q = min(q_m, q_x)` with `q_m = floor(m x 10^6 / (rate x price))` and `q_x = floor(x /
    /// price)`, and which of the two bound it (a tie is the margin's).
    fn quantity(self, candidate: &Candidate) -> (u64, Bound) {
        let price = candidate.price.unsigned_abs();
        // A u32 times a u64 is exact in u128, and at least 200,000.
        let per_share_micro = u128::from(candidate.rate_ppm) * u128::from(price);
        let by_margin = self.margin_micro / per_share_micro;
        let by_exposure = self.exposure / price;
        let bound = if by_margin <= u128::from(by_exposure) {
            Bound::MarginShare
        } else {
            Bound::ExposureShare
        };
        // A `q_m` past u64 is past `q_x` too, so the minimum is `q_x`: exact, not a cap.
        let qty = u64::try_from(by_margin).map_or(by_exposure, |q| q.min(by_exposure));
        (qty, bound)
    }
}

/// The FIXED policies' row: FNO-NET's quantity, untouched.
fn fixed(candidate: &Candidate) -> Outcome {
    sized(candidate, candidate.fixed_qty, Bound::FixedTarget, None)
}

/// SIZE-RISK's row: the stop, then `q = min(q_r, q_m, q_x)` with `q_r = floor(b / d)`; a tie
/// between the risk and a share is the risk's.
fn risky(candidate: &Candidate, shares: Shares, each: u128) -> Outcome {
    let Some(stop) = candidate.stop else {
        return Outcome::Refused(SizeRefusal::NoStop);
    };
    let distance = candidate.side.distance(candidate.price, stop);
    if distance == 0 {
        return Outcome::Refused(SizeRefusal::ZeroStop);
    }
    if distance < 0 {
        return Outcome::Refused(SizeRefusal::StopThroughPrice);
    }
    let (by_share, share_bound) = shares.quantity(candidate);
    let by_risk = each / distance.unsigned_abs();
    let bound = if by_risk <= u128::from(by_share) {
        Bound::Risk
    } else {
        share_bound
    };
    let qty = u64::try_from(by_risk).map_or(by_share, |q| q.min(by_share));
    sized(candidate, qty, bound, Some(i128::from(qty) * distance))
}

/// A row of `qty` shares, or `size_below_one_share` when `qty` is 0. The value is at most the
/// exposure share or FNO-NET's checked value, so both products are exact.
fn sized(candidate: &Candidate, qty: u64, bound: Bound, risk: Option<i128>) -> Outcome {
    if qty == 0 {
        return Outcome::Refused(SizeRefusal::BelowOneShare);
    }
    let value = i128::from(qty) * i128::from(candidate.price);
    Outcome::Sized(Sized {
        qty,
        value,
        margin: value * i128::from(candidate.rate_ppm),
        risk,
        bound,
    })
}

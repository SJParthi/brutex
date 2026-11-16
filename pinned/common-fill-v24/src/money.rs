//! REQ 70a's amounts (design 4.2 `money`, S12): the payable exposure from the event's free cash, the money in (rounded against us), the
//! excess over the reserve, the reserve itself, and ONE running free-cash balance per level drawn IN EVENT ORDER ([`FreeCash`]: at one
//! instant releases first, then entries becoming known in admission order, then admissions; sample-6's `o5s_acct::walk_account`). The
//! common ACCOUNT module (D26) owns the account; this is the event rule's core that the fill tests and two-phase hosts need.

use crate::FillErr;
use crate::{Paisa, Units};
use crate::limits::ceil_div;

/// S12: the payable exposure P = leverage x (reserve + max(free cash now, 0)), floor, paisa. Free cash is the level's balance AT the
/// entry's known_at event, never a snapshot at the decision (critic A B3).
///
/// # Errors
/// [`FillErr::NegativeFreeCash`] when `free_now` is below 0 (it never is: OB25); [`FillErr::Overflow`] when P leaves i64.
pub fn pay_exposure(reserve: Paisa, free_now: Paisa, lev_num: u32, lev_den: u32) -> Result<Paisa, FillErr> {
    if free_now < 0 {
        return Err(FillErr::NegativeFreeCash);
    }
    if lev_den == 0 {
        return Err(FillErr::Overflow);
    }
    let p = (i128::from(reserve) + i128::from(free_now)) * i128::from(lev_num) / i128::from(lev_den);
    i64::try_from(p).map_err(|_| FillErr::Overflow)
}

/// The money in of a position worth `value`: ceil(value / leverage) paisa, rounded against us.
#[must_use]
pub fn money_in(value: Paisa, lev_num: u32, lev_den: u32) -> Paisa {
    if lev_num == 0 {
        return value;
    }
    i64::try_from(ceil_div(i128::from(value) * i128::from(lev_den), i128::from(lev_num))).unwrap_or(i64::MAX)
}

/// The excess of the money in over the reserve: max(0, money in - reserve).
#[must_use]
pub const fn excess(money_in: Paisa, reserve: Paisa) -> Paisa {
    let e = money_in.saturating_sub(reserve);
    if e > 0 { e } else { 0 }
}

/// S12's reserve (D12; sample-6's `dial70`): min(slot, ceil(qty x reserve price / leverage)), the reserve price a long's min(protection
/// limit, cap price) and a short's cap price (the host passes it).
#[must_use]
pub fn reserve(slot: Paisa, qty: Units, reserve_px: Paisa, lev_num: u32, lev_den: u32) -> Paisa {
    if lev_num == 0 {
        return slot;
    }
    let v = i128::from(qty) * i128::from(reserve_px) * i128::from(lev_den);
    i64::try_from(ceil_div(v, i128::from(lev_num))).unwrap_or(i64::MAX).min(slot)
}

/// What an entry's known_at event drew (S12).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Draw {
    /// The excess was paid from free cash (0 when there was none).
    Paid(Paisa),
    /// Free cash could not pay the excess: the entry is cut to the payable exposure `pay` (the host takes `FillPath::payable` or re-walks
    /// with `money` = `pay`; the rest is cancelled, `unpaid`), then calls [`FreeCash::settle_cut`] with the cut's money in.
    Cut {
        /// The payable exposure P (paisa).
        pay: Paisa,
    },
}

/// ONE running free-cash balance of one level (S12): base less the money held by open trades less their reserves. Calls in event order:
/// at one instant [`FreeCash::release`] first, then [`FreeCash::known`] in admission order, then [`FreeCash::admit`]. Free cash never goes
/// below 0 and lost money is never reused (the base is the host's, Q7).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct FreeCash {
    free: Paisa,
    /// Mutant m9 only: the free cash as it stood at the last admission (the decision), which m9 reads instead of the event's balance.
    at_decision: Paisa,
    /// Excess draws paid: count, largest, total (S12's counts).
    pub paid_n: u32,
    /// See `paid_n`.
    pub paid_max: Paisa,
    /// See `paid_n`.
    pub paid_total: Paisa,
    /// Entries cut by the payability rule.
    pub cut_n: u32,
}

impl FreeCash {
    /// A balance of `free` paisa.
    ///
    /// # Errors
    /// [`FillErr::NegativeFreeCash`] below 0.
    pub const fn new(free: Paisa) -> Result<Self, FillErr> {
        if free < 0 {
            return Err(FillErr::NegativeFreeCash);
        }
        Ok(Self { free, at_decision: free, paid_n: 0, paid_max: 0, paid_total: 0, cut_n: 0 })
    }

    /// The balance now.
    #[must_use]
    pub const fn free(&self) -> Paisa {
        self.free
    }

    /// An admission holds its reserve (the host admitted it: a slot was free and the reserve fits).
    ///
    /// # Errors
    /// [`FillErr::NegativeFreeCash`] when the reserve exceeds the balance (the host must not admit it).
    pub const fn admit(&mut self, reserve: Paisa) -> Result<(), FillErr> {
        if reserve > self.free {
            return Err(FillErr::NegativeFreeCash);
        }
        // Checked, never wrapping (OB7, G0 repair 2): a negative reserve near i64::MIN refuses.
        match self.free.checked_sub(reserve) {
            Some(v) => self.free = v,
            None => return Err(FillErr::Overflow),
        }
        self.at_decision = self.free;
        Ok(())
    }

    /// An entry becoming known with money in `money_in` over its `reserve`: its excess is paid now if this instant's free cash covers it,
    /// else the entry is cut to P = leverage x (reserve + free cash now) (S12).
    ///
    /// # Errors
    /// As [`pay_exposure`].
    pub fn known(&mut self, money_in: Paisa, reserve: Paisa, lev_num: u32, lev_den: u32) -> Result<Draw, FillErr> {
        let ex = excess(money_in, reserve);
        let free_seen = if cfg!(feature = "m9") { self.at_decision } else { self.free };
        if ex <= free_seen {
            self.free = self.free.saturating_sub(ex);
            if cfg!(feature = "m9") {
                // MUTANT m9 (design 10.4): the free cash read at the decision, so two entries known at one instant both draw it.
                self.free = self.free.max(0);
            }
            if ex > 0 {
                self.paid_n = self.paid_n.saturating_add(1);
                self.paid_max = self.paid_max.max(ex);
                self.paid_total = self.paid_total.saturating_add(ex);
            }
            return Ok(Draw::Paid(ex));
        }
        self.cut_n = self.cut_n.saturating_add(1);
        Ok(Draw::Cut { pay: pay_exposure(reserve, self.free.max(0), lev_num, lev_den)? })
    }

    /// After a cut: the cut entry's money in `money_in` over its `reserve` is drawn (it fits the balance by construction).
    pub fn settle_cut(&mut self, money_in: Paisa, reserve: Paisa) {
        let ex = excess(money_in, reserve).min(self.free.max(0));
        self.free -= ex;
        if ex > 0 {
            self.paid_n = self.paid_n.saturating_add(1);
            self.paid_max = self.paid_max.max(ex);
            self.paid_total = self.paid_total.saturating_add(ex);
        }
    }

    /// A trade's known end frees `amount` (its money in, or its reserve when it never filled).
    pub const fn release(&mut self, amount: Paisa) {
        self.free = self.free.saturating_add(amount);
    }
}

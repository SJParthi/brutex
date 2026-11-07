//! The exact account (PREVIEW-SPEC 10, 25.1): ceilings as data, admission,
//! the size step, the margin and exposure checks, net, months and drawdown.
//!
//! * **Ceilings** `{ day_cap, month_cap, amount, margin }` are data: the main
//!   account (5, 100, Rs 15 lakh, Rs 3 lakh margin) and ceiling B (10, 100,
//!   Rs 10 lakh, Rs 2 lakh margin).
//! * **Admission** in start-second order, then decision, then symbol (FIRST),
//!   with no look-ahead: one position per stock (`while_open`), the day cap
//!   (`day cap`), the month cap (`month cap`), N slots (`slots_full`).
//! * **The amount** (25.1's ruling): full step = `min(ceiling amount, Rs 1
//!   crore / N)`, quantised down to the largest of the five walked amounts
//!   (Rs 15, 10, 7.5, 5, 2.5 lakh) whose margin at the stock-day's dated
//!   leverage is at most the ceiling's margin (none: `margin_cap`). At the
//!   order's start the margin check (margin in use plus this order's at most
//!   the closed equity) and the exposure check (at most Rs 1 crore) may take
//!   it down one more walked step; below the half step `size_refused`.
//! * **Net** = money in - money out - charges; a win is net above 0. The data
//!   charge, Rs 499 + GST = Rs 588.82, is due in each calendar month with a
//!   trade.
//! * **Max drawdown**: the closed-equity fall from the running high, in
//!   tenths of a percent of Rs 20 lakh.

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet};

use crate::clock::Date;
use crate::model::{BelowHalf, DataChargeAt, DdRound, LockEnd, Readings, Status, ZeroFillCount};
use crate::money::{CRORE, LAKH, ceil_div, floor_div};

/// An account's ceilings, as data.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ceilings {
    /// Trades a session.
    pub day_cap: u32,
    /// Trades a calendar month.
    pub month_cap: u32,
    /// Largest position, paise.
    pub amount: i64,
    /// Largest margin a trade, paise.
    pub margin: i64,
}

/// The main account's ceilings (REQ 25).
pub const MAIN: Ceilings = Ceilings {
    day_cap: 5,
    month_cap: 100,
    amount: 15 * LAKH,
    margin: 3 * LAKH,
};
/// Ceiling B, the labelled second account (REQ 29).
pub const CEILING_B: Ceilings = Ceilings {
    day_cap: 10,
    month_cap: 100,
    amount: 10 * LAKH,
    margin: 2 * LAKH,
};
/// The capital, Rs 20 lakh of margin.
pub const CAPITAL: i64 = 20 * LAKH;
/// The open position value cap, Rs 1 crore.
pub const EXPOSURE: i64 = CRORE;
/// Rs 499 + 18% GST, paise.
pub const DATA_CHARGE: i64 = 58_882;
/// The five walked amounts, largest first.
pub const WALKED: [i64; 5] = [
    15 * LAKH,
    10 * LAKH,
    75 * LAKH / 10,
    5 * LAKH,
    25 * LAKH / 10,
];
/// The bar, Rs 2,50,000 net in a month.
pub const BAR: i64 = 250_000 * 100;

/// A stock-day's dated leverage as a margin fraction: margin =
/// `ceil(amount x num / den)`. `C_VAR1`'s reading is `num / den =
/// max(20, M) / 100` (25.3 D5); 5x is `1 / 5`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Leverage {
    /// Numerator.
    pub num: i64,
    /// Denominator.
    pub den: i64,
}

impl Leverage {
    /// Flat 5x.
    pub const FIVE: Leverage = Leverage { num: 1, den: 5 };

    /// The margin of an amount, paise, rounded up.
    pub fn margin(self, amount: i64) -> i128 {
        ceil_div(
            i128::from(amount) * i128::from(self.num),
            i128::from(self.den.max(1)),
        )
    }

    /// From `C_VAR1`'s `VaR` + extreme-loss margin `m_bp` (hundredths of a
    /// percent): `max(20%, M)`.
    pub fn from_var(m_bp: i64) -> Leverage {
        Leverage {
            num: m_bp.max(2_000),
            den: 10_000,
        }
    }
}

/// One signal's walk at one amount, priced (the caller walks it).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Outcome {
    /// `Ok`, or the walk's refusal before its start.
    pub status: Status,
    /// Shares the entry filled.
    pub filled: i64,
    /// The first fill's second.
    pub first_fill: Option<u32>,
    /// The second the position ended: the exit's last fill, or the entry's
    /// cancel when nothing filled.
    pub end: u32,
    /// Net = money in - money out - headline charges, paise.
    pub net: i128,
    /// Headline charges, paise.
    pub charges: i128,
}

/// One signal offered to the account.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Signal {
    /// Stable id.
    pub id: u32,
    /// Symbol order key.
    pub sym: u32,
    /// The session.
    pub date: Date,
    /// The decision second.
    pub decided_at: u32,
    /// The entry order's start second.
    pub start: u32,
    /// The stock-day's dated leverage.
    pub lev: Leverage,
    /// The walk at each walked amount (`WALKED` order).
    pub outcomes: [Outcome; 5],
}

/// What admission decided for one signal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Admission {
    /// Signal id.
    pub id: u32,
    /// `Ok` (admitted) or the refusal by name.
    pub status: Status,
    /// The amount, when admitted.
    pub amount: Option<i64>,
}

/// One closed trade (an admitted signal with a fill).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Trade {
    /// Signal id.
    pub id: u32,
    /// Symbol.
    pub sym: u32,
    /// Session.
    pub date: Date,
    /// Amount walked.
    pub amount: i64,
    /// Net, paise.
    pub net: i128,
    /// Headline charges, paise.
    pub charges: i128,
    /// Seconds from the order's start to its end (the search's holding).
    pub hold: u32,
}

/// One calendar month of an account.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MonthRow {
    /// Month index (`Date::month_index`).
    pub month: i32,
    /// Trades.
    pub trades: u32,
    /// Wins (net above 0).
    pub wins: u32,
    /// Net: trade nets less the data charge, paise.
    pub net: i128,
    /// Headline charges of the trades, paise (the cost section only).
    pub charges: i128,
    /// The worst trade's net, paise (0 with no trade).
    pub worst: i128,
}

/// The account's run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccountOut {
    /// Every admission decision, in processing order.
    pub admissions: Vec<Admission>,
    /// Every trade, in close order.
    pub trades: Vec<Trade>,
    /// Every month with a trade, ascending.
    pub months: Vec<MonthRow>,
    /// Max drawdown, paise.
    pub max_dd: i128,
    /// Max drawdown, tenths of a percent of Rs 20 lakh.
    pub max_dd_tenths: i128,
    /// Closed equity at the end, paise.
    pub end_equity: i128,
}

/// A queued account event.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Kind {
    /// The month's data charge is debited.
    Debit(i32),
    /// A position ends (by signal id).
    Close(u32),
}

/// An open position (or a pending entry).
#[derive(Clone, Copy, Debug)]
struct Open {
    id: u32,
    sym: u32,
    margin: i128,
    amount: i64,
    filled: bool,
    day: i64,
    month: i32,
    net: i128,
}

/// The amount after the leverage quantisation, or the refusal.
///
/// # Errors
/// `margin_cap` when no walked amount's margin fits the ceiling;
/// `size_refused` below the half step under that reading.
pub fn quantise(
    full: i64,
    lev: Leverage,
    ceil: &Ceilings,
    below_half: BelowHalf,
) -> Result<i64, Status> {
    let amount = WALKED
        .iter()
        .copied()
        .find(|&a| {
            a <= full
                && i128::from(a) * i128::from(lev.num)
                    <= i128::from(ceil.margin) * i128::from(lev.den)
        })
        .ok_or(Status::MarginCap)?;
    if below_half == BelowHalf::Refused && amount * 2 < full {
        return Err(Status::SizeRefused);
    }
    Ok(amount)
}

/// The full step `min(ceiling amount, Rs 1 crore / N)`.
pub fn full_step(n: u32, ceil: &Ceilings) -> i64 {
    let per =
        i64::try_from(floor_div(i128::from(EXPOSURE), i128::from(n.max(1)))).unwrap_or(EXPOSURE);
    ceil.amount.min(per)
}

/// Max drawdown in tenths of a percent of Rs 20 lakh.
pub fn dd_tenths(dd: i128, round: DdRound) -> i128 {
    let unit = i128::from(CAPITAL) / 1000;
    match round {
        DdRound::Floor => floor_div(dd, unit),
        DdRound::Ceil => ceil_div(dd, unit),
    }
}

/// The account's running state.
struct State<'r> {
    r: &'r Readings,
    equity: i128,
    peak: i128,
    max_dd: i128,
    open: Vec<Open>,
    day_count: HashMap<i64, u32>,
    month_count: HashMap<i32, u32>,
    debit_at: HashMap<i32, (i64, u32)>,
    debited: HashSet<i32>,
    heap: BinaryHeap<Reverse<((i64, u32), Kind)>>,
}

impl State<'_> {
    /// Applies one event.
    fn apply(&mut self, kind: Kind) {
        match kind {
            Kind::Debit(m) => {
                if self.debited.contains(&m) {
                    return;
                }
                self.debited.insert(m);
                if self.r.data_charge_at == DataChargeAt::FirstFill {
                    self.equity -= i128::from(DATA_CHARGE);
                }
            }
            Kind::Close(id) => {
                // every close event names a position its admission opened
                let ix = self
                    .open
                    .iter()
                    .position(|o| o.id == id)
                    .expect("a close event names an open position");
                let o = self.open.remove(ix);
                if o.filled {
                    self.equity += o.net;
                } else if self.r.zero_fill_count == ZeroFillCount::Released {
                    let day = self.day_count.entry(o.day).or_default();
                    *day = day.saturating_sub(1);
                    let month = self.month_count.entry(o.month).or_default();
                    *month = month.saturating_sub(1);
                }
            }
        }
        self.peak = self.peak.max(self.equity);
        self.max_dd = self.max_dd.max(self.peak - self.equity);
    }

    /// Applies every event due before an admission at `key`.
    fn drain_until(&mut self, key: (i64, u32)) {
        while let Some(Reverse((k, kind))) = self.heap.peek().copied() {
            let due = match self.r.lock_end {
                LockEnd::Inclusive => k < key,
                LockEnd::Exclusive => k <= key,
            };
            if !due {
                break;
            }
            self.heap.pop();
            self.apply(kind);
        }
    }

    /// The admission checks for one signal: the amount, or the refusal.
    fn check(&self, s: &Signal, n: u32, full: i64, ceil: &Ceilings) -> Result<i64, Status> {
        let day = s.date.days();
        let month = s.date.month_index();
        if self.open.iter().any(|o| o.sym == s.sym) {
            return Err(Status::WhileOpen);
        }
        if self.day_count.get(&day).copied().unwrap_or(0) >= ceil.day_cap {
            return Err(Status::DayCap);
        }
        if self.month_count.get(&month).copied().unwrap_or(0) >= ceil.month_cap {
            return Err(Status::MonthCap);
        }
        if self.open.len() >= n as usize {
            return Err(Status::SlotsFull);
        }
        let amount0 = quantise(full, s.lev, ceil, self.r.below_half)?;
        let margin_used: i128 = self.open.iter().map(|o| o.margin).sum();
        let exposure_used: i128 = self.open.iter().map(|o| i128::from(o.amount)).sum();
        let fits = |a: i64| -> Result<i64, Status> {
            if margin_used + s.lev.margin(a) > self.equity {
                Err(Status::MarginCap)
            } else if exposure_used + i128::from(a) > i128::from(EXPOSURE) {
                Err(Status::ExposureCap)
            } else {
                Ok(a)
            }
        };
        fits(amount0).or_else(|_| {
            let ix = WALKED
                .iter()
                .position(|&a| a == amount0)
                .unwrap_or(WALKED.len() - 1);
            match WALKED.get(ix + 1) {
                Some(&a1) if a1 * 2 >= full => fits(a1),
                _ => Err(Status::SizeRefused),
            }
        })
    }
}

/// The account, one signal at a time: offer signals in `(date, start,
/// decision, symbol, id)` order (FIRST admission; [`run`] sorts for you),
/// then `finish`.
pub struct Account<'r> {
    st: State<'r>,
    n: u32,
    ceil: Ceilings,
    full: i64,
    admissions: Vec<Admission>,
    trades: Vec<Trade>,
    months: HashMap<i32, MonthRow>,
}

impl<'r> Account<'r> {
    /// An empty account with N slots, its ceilings and its start equity.
    pub fn new(n: u32, ceil: &Ceilings, r: &'r Readings, start_equity: i128) -> Account<'r> {
        Account {
            st: State {
                r,
                equity: start_equity,
                peak: start_equity,
                max_dd: 0,
                open: Vec::new(),
                day_count: HashMap::new(),
                month_count: HashMap::new(),
                debit_at: HashMap::new(),
                debited: HashSet::new(),
                heap: BinaryHeap::new(),
            },
            n,
            ceil: *ceil,
            full: full_step(n, ceil),
            admissions: Vec::new(),
            trades: Vec::new(),
            months: HashMap::new(),
        }
    }

    /// Offers one signal: what admission decided.
    pub fn offer(&mut self, s: &Signal) -> Admission {
        let day = s.date.days();
        let month = s.date.month_index();
        self.st.drain_until((day, s.start));
        let admission = match self.st.check(s, self.n, self.full, &self.ceil) {
            Err(e) => Admission {
                id: s.id,
                status: e,
                amount: None,
            },
            Ok(amount) => {
                let ix = WALKED.iter().position(|&a| a == amount).unwrap_or(0);
                let o = s.outcomes[ix];
                if o.status == Status::Ok {
                    self.admit(s, amount, &o, day, month);
                }
                Admission {
                    id: s.id,
                    status: o.status,
                    amount: Some(amount),
                }
            }
        };
        self.admissions.push(admission);
        admission
    }

    /// Books an admitted signal.
    fn admit(&mut self, s: &Signal, amount: i64, o: &Outcome, day: i64, month: i32) {
        let st = &mut self.st;
        *st.day_count.entry(day).or_insert(0) += 1;
        *st.month_count.entry(month).or_insert(0) += 1;
        st.open.push(Open {
            id: s.id,
            sym: s.sym,
            margin: s.lev.margin(amount),
            amount,
            filled: o.filled > 0,
            day,
            month,
            net: o.net,
        });
        st.heap.push(Reverse(((day, o.end), Kind::Close(s.id))));
        if o.filled == 0 {
            return;
        }
        self.trades.push(Trade {
            id: s.id,
            sym: s.sym,
            date: s.date,
            amount,
            net: o.net,
            charges: o.charges,
            hold: o.end.saturating_sub(s.start),
        });
        let row = self.months.entry(month).or_insert(MonthRow {
            month,
            trades: 0,
            wins: 0,
            net: 0,
            charges: 0,
            worst: 0,
        });
        row.worst = if row.trades == 0 {
            o.net
        } else {
            row.worst.min(o.net)
        };
        row.trades += 1;
        row.wins += u32::from(o.net > 0);
        row.net += o.net;
        row.charges += o.charges;
        let f = (day, o.first_fill.unwrap_or(s.start));
        if !st.debited.contains(&month) && st.debit_at.get(&month).is_none_or(|&due| f < due) {
            st.debit_at.insert(month, f);
            st.heap.push(Reverse((f, Kind::Debit(month))));
        }
    }

    /// Applies every event left and returns the run.
    pub fn finish(mut self) -> AccountOut {
        while let Some(Reverse((_, kind))) = self.st.heap.pop() {
            self.st.apply(kind);
        }
        let mut rows: Vec<MonthRow> = self
            .months
            .into_values()
            .map(|mut m| {
                m.net -= i128::from(DATA_CHARGE);
                m
            })
            .collect();
        rows.sort_by_key(|m| m.month);
        AccountOut {
            admissions: self.admissions,
            trades: self.trades,
            months: rows,
            max_dd: self.st.max_dd,
            max_dd_tenths: dd_tenths(self.st.max_dd, self.st.r.dd_round),
            end_equity: self.st.equity,
        }
    }
}

/// Runs the account over `signals` (any order; sorted here) with N slots.
pub fn run(
    signals: &[Signal],
    n: u32,
    ceil: &Ceilings,
    r: &Readings,
    start_equity: i128,
) -> AccountOut {
    let mut order: Vec<&Signal> = signals.iter().collect();
    order.sort_by_key(|s| (s.date.days(), s.start, s.decided_at, s.sym, s.id));
    let mut acct = Account::new(n, ceil, r, start_equity);
    for s in order {
        acct.offer(s);
    }
    acct.finish()
}

#[cfg(test)]
#[path = "account_tests.rs"]
pub(crate) mod tests;

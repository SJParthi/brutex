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
    fn apply(&mut self, key: (i64, u32), kind: Kind) {
        match kind {
            Kind::Debit(m) => {
                if self.debited.contains(&m) || self.debit_at.get(&m) != Some(&key) {
                    return;
                }
                self.debited.insert(m);
                if self.r.data_charge_at == DataChargeAt::FirstFill {
                    self.equity -= i128::from(DATA_CHARGE);
                }
            }
            Kind::Close(id) => {
                let Some(ix) = self.open.iter().position(|o| o.id == id) else {
                    return;
                };
                let o = self.open.remove(ix);
                if o.filled {
                    self.equity += o.net;
                } else if self.r.zero_fill_count == ZeroFillCount::Released {
                    if let Some(c) = self.day_count.get_mut(&o.day) {
                        *c = c.saturating_sub(1);
                    }
                    if let Some(c) = self.month_count.get_mut(&o.month) {
                        *c = c.saturating_sub(1);
                    }
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
            self.apply(k, kind);
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
    let full = full_step(n, ceil);
    let mut st = State {
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
    };
    let mut admissions = Vec::with_capacity(order.len());
    let mut trades = Vec::new();
    let mut months: HashMap<i32, MonthRow> = HashMap::new();
    for s in order {
        let day = s.date.days();
        let month = s.date.month_index();
        st.drain_until((day, s.start));
        let amount = match st.check(s, n, full, ceil) {
            Ok(a) => a,
            Err(e) => {
                admissions.push(Admission {
                    id: s.id,
                    status: e,
                    amount: None,
                });
                continue;
            }
        };
        let ix = WALKED.iter().position(|&a| a == amount).unwrap_or(0);
        let o = s.outcomes[ix];
        if o.status != Status::Ok {
            admissions.push(Admission {
                id: s.id,
                status: o.status,
                amount: Some(amount),
            });
            continue;
        }
        admissions.push(Admission {
            id: s.id,
            status: Status::Ok,
            amount: Some(amount),
        });
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
            continue;
        }
        trades.push(Trade {
            id: s.id,
            sym: s.sym,
            date: s.date,
            amount,
            net: o.net,
            charges: o.charges,
            hold: o.end.saturating_sub(s.start),
        });
        let row = months.entry(month).or_insert(MonthRow {
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
    while let Some(Reverse((k, kind))) = st.heap.pop() {
        st.apply(k, kind);
    }
    let mut rows: Vec<MonthRow> = months
        .into_values()
        .map(|mut m| {
            m.net -= i128::from(DATA_CHARGE);
            m
        })
        .collect();
    rows.sort_by_key(|m| m.month);
    AccountOut {
        admissions,
        trades,
        months: rows,
        max_dd: st.max_dd,
        max_dd_tenths: dd_tenths(st.max_dd, r.dd_round),
        end_equity: st.equity,
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::clock::OPEN;

    pub(crate) fn outcome(net: i128, end: u32) -> Outcome {
        Outcome {
            status: Status::Ok,
            filled: 100,
            first_fill: Some(end.saturating_sub(60)),
            end,
            net,
            charges: 40_000,
        }
    }

    pub(crate) fn signal(id: u32, sym: u32, date: Date, start: u32, net: i128, end: u32) -> Signal {
        Signal {
            id,
            sym,
            date,
            decided_at: start - 14,
            start,
            lev: Leverage::FIVE,
            outcomes: [outcome(net, end); 5],
        }
    }

    fn d(day: u32) -> Date {
        Date {
            y: 2026,
            m: 10,
            d: day,
        }
    }

    fn statuses(out: &AccountOut) -> Vec<Status> {
        out.admissions.iter().map(|a| a.status).collect()
    }

    #[test]
    fn the_sixth_trade_of_a_day_is_refused_by_name() {
        let sigs: Vec<Signal> = (0..7)
            .map(|k| {
                signal(
                    k,
                    k,
                    d(5),
                    OPEN + 600 + 60 * k,
                    100_000,
                    OPEN + 620 + 60 * k,
                )
            })
            .collect();
        let out = run(&sigs, 5, &MAIN, &Readings::HEADLINE, i128::from(CAPITAL));
        assert_eq!(
            statuses(&out),
            [
                Status::Ok,
                Status::Ok,
                Status::Ok,
                Status::Ok,
                Status::Ok,
                Status::DayCap,
                Status::DayCap
            ]
        );
        assert_eq!(out.trades.len(), 5);
        let out_b = run(
            &sigs,
            10,
            &CEILING_B,
            &Readings::HEADLINE,
            i128::from(CAPITAL),
        );
        assert!(out_b.admissions.iter().all(|a| a.status == Status::Ok));
        assert_eq!(out_b.admissions[0].amount, Some(10 * LAKH));
    }

    #[test]
    fn the_101st_of_a_month_is_refused_by_name() {
        let mut sigs = Vec::new();
        let mut id = 0;
        for day in 1..=25u32 {
            for k in 0..5u32 {
                sigs.push(signal(
                    id,
                    id,
                    Date {
                        y: 2026,
                        m: 7,
                        d: day,
                    },
                    OPEN + 600 + 60 * k,
                    10_000,
                    OPEN + 620 + 60 * k,
                ));
                id += 1;
            }
        }
        let out = run(&sigs, 5, &MAIN, &Readings::HEADLINE, i128::from(CAPITAL));
        let ok = out
            .admissions
            .iter()
            .filter(|a| a.status == Status::Ok)
            .count();
        let capped = out
            .admissions
            .iter()
            .filter(|a| a.status == Status::MonthCap)
            .count();
        assert_eq!((ok, capped), (100, 25));
        assert_eq!(out.admissions[100].status, Status::MonthCap);
        assert_eq!(out.months.len(), 1);
        assert_eq!(out.months[0].trades, 100);
        assert_eq!(out.months[0].net, 100 * 10_000 - i128::from(DATA_CHARGE));
    }

    #[test]
    fn one_position_per_stock_slots_and_the_lock_end() {
        let sigs = [
            signal(1, 7, d(5), OPEN + 600, 1, OPEN + 900),
            signal(2, 7, d(5), OPEN + 700, 1, OPEN + 800),
            signal(3, 8, d(5), OPEN + 900, 1, OPEN + 950),
        ];
        let out = run(&sigs, 1, &MAIN, &Readings::HEADLINE, i128::from(CAPITAL));
        assert_eq!(
            statuses(&out),
            [Status::Ok, Status::WhileOpen, Status::SlotsFull]
        );
        let mut r = Readings::HEADLINE;
        r.lock_end = LockEnd::Exclusive;
        let out = run(&sigs, 1, &MAIN, &r, i128::from(CAPITAL));
        assert_eq!(statuses(&out), [Status::Ok, Status::WhileOpen, Status::Ok]);
    }

    #[test]
    fn leverage_quantises_the_amount_and_the_half_step_readings() {
        let three = Leverage { num: 1, den: 3 };
        assert_eq!(
            quantise(full_step(1, &MAIN), three, &MAIN, BelowHalf::Allowed),
            Ok(75 * LAKH / 10)
        );
        assert_eq!(
            quantise(
                full_step(1, &CEILING_B),
                three,
                &CEILING_B,
                BelowHalf::Allowed
            ),
            Ok(5 * LAKH)
        );
        assert_eq!(
            quantise(
                full_step(10, &MAIN),
                Leverage::FIVE,
                &MAIN,
                BelowHalf::Allowed
            ),
            Ok(10 * LAKH)
        );
        assert_eq!(
            quantise(
                full_step(20, &MAIN),
                Leverage::FIVE,
                &MAIN,
                BelowHalf::Allowed
            ),
            Ok(5 * LAKH)
        );
        let two = Leverage { num: 1, den: 2 };
        assert_eq!(
            quantise(full_step(1, &MAIN), two, &MAIN, BelowHalf::Allowed),
            Ok(5 * LAKH)
        );
        assert_eq!(
            quantise(full_step(1, &MAIN), two, &MAIN, BelowHalf::Refused),
            Err(Status::SizeRefused)
        );
        let one = Leverage { num: 1, den: 1 };
        assert_eq!(
            quantise(full_step(1, &MAIN), one, &MAIN, BelowHalf::Allowed),
            Ok(25 * LAKH / 10)
        );
        let none = Leverage { num: 2, den: 1 };
        assert_eq!(
            quantise(full_step(1, &MAIN), none, &MAIN, BelowHalf::Allowed),
            Err(Status::MarginCap)
        );
        assert_eq!(full_step(7, &MAIN), 142_857_142);
        assert_eq!(full_step(0, &MAIN), 15 * LAKH);
        assert_eq!(
            Leverage::from_var(2_345),
            Leverage {
                num: 2_345,
                den: 10_000
            }
        );
        assert_eq!(
            Leverage::from_var(1_000),
            Leverage {
                num: 2_000,
                den: 10_000
            }
        );
        assert_eq!(Leverage::FIVE.margin(15 * LAKH), i128::from(3 * LAKH));
        assert_eq!(Leverage { num: 1, den: 3 }.margin(10), 4);
    }

    #[test]
    fn the_margin_check_steps_down_once_then_refuses() {
        let sigs: Vec<Signal> = (0..5)
            .map(|k| signal(k, k, d(6), OPEN + 600 + 60 * k, 0, OPEN + 10_000))
            .collect();
        // four open use Rs 12 lakh; Rs 13 lakh equity: 15 lakh needs 3, steps to 10 lakh needing 2: 14 > 13
        let out = run(&sigs, 5, &MAIN, &Readings::HEADLINE, 130_000_000);
        assert_eq!(out.admissions[3].amount, Some(15 * LAKH));
        assert_eq!(out.admissions[4].status, Status::MarginCap);
        let out = run(&sigs, 5, &MAIN, &Readings::HEADLINE, 140_000_000);
        assert_eq!(
            (out.admissions[4].status, out.admissions[4].amount),
            (Status::Ok, Some(10 * LAKH))
        );
        let one = [signal(9, 9, d(6), OPEN + 600, 0, OPEN + 700)];
        let out = run(&one, 1, &MAIN, &Readings::HEADLINE, 20_000_000);
        assert_eq!(
            (out.admissions[0].status, out.admissions[0].amount),
            (Status::Ok, Some(10 * LAKH))
        );
        let out = run(&one, 1, &MAIN, &Readings::HEADLINE, 10_000_000);
        assert_eq!(out.admissions[0].status, Status::MarginCap);
        // N = 20's half step is Rs 2.5 lakh: below it nothing is walked
        let tiny = [Signal {
            lev: Leverage { num: 1, den: 1 },
            ..one[0]
        }];
        let out = run(&tiny, 20, &MAIN, &Readings::HEADLINE, 100_000);
        assert_eq!(out.admissions[0].status, Status::SizeRefused);
        // N = 1 at 15 lakh steps to 10 lakh, then 7.5 lakh is not tried: one step only
        let out = run(&one, 1, &MAIN, &Readings::HEADLINE, 16_000_000);
        assert_eq!(out.admissions[0].status, Status::MarginCap);
    }

    #[test]
    fn a_walk_refusal_admits_nothing_and_counts_nothing() {
        let refused = Signal {
            outcomes: [Outcome {
                status: Status::PriceAbovePosition,
                filled: 0,
                first_fill: None,
                end: OPEN + 700,
                net: 0,
                charges: 0,
            }; 5],
            ..signal(50, 50, d(8), OPEN + 600, 0, OPEN + 700)
        };
        let after = signal(51, 51, d(8), OPEN + 650, 10, OPEN + 900);
        let out = run(
            &[refused, after],
            1,
            &MAIN,
            &Readings::HEADLINE,
            i128::from(CAPITAL),
        );
        assert_eq!(statuses(&out), [Status::PriceAbovePosition, Status::Ok]);
        assert_eq!(out.trades.len(), 1);
    }

    #[test]
    fn exposure_binds_only_beside_a_larger_position() {
        // N slots of the full step never pass Rs 1 crore (floor(1 crore / N) x N),
        // so one recipe's account cannot reach the exposure check's refusal;
        // a merged account can, as this state shows.
        let r = Readings::HEADLINE;
        let st = State {
            r: &r,
            equity: i128::from(10 * CRORE),
            peak: 0,
            max_dd: 0,
            open: vec![Open {
                id: 1,
                sym: 1,
                margin: 0,
                amount: 90 * LAKH,
                filled: true,
                day: 0,
                month: 0,
                net: 0,
            }],
            day_count: HashMap::new(),
            month_count: HashMap::new(),
            debit_at: HashMap::new(),
            debited: HashSet::new(),
            heap: BinaryHeap::new(),
        };
        let s = signal(2, 2, d(16), OPEN + 600, 0, OPEN + 700);
        // Rs 15 lakh passes Rs 1 crore; one step down, Rs 10 lakh, fits exactly
        assert_eq!(st.check(&s, 5, full_step(5, &MAIN), &MAIN), Ok(10 * LAKH));
        let st2 = State {
            open: vec![Open {
                amount: 95 * LAKH,
                ..st.open[0]
            }],
            ..st
        };
        assert_eq!(
            st2.check(&s, 5, full_step(5, &MAIN), &MAIN),
            Err(Status::ExposureCap)
        );
        // N = 12 at Rs 8.33 lakh quantises to Rs 7.5 lakh: twelve fit Rs 90 lakh
        let wide = Ceilings {
            day_cap: 30,
            month_cap: 100,
            amount: 15 * LAKH,
            margin: 15 * LAKH,
        };
        let sigs: Vec<Signal> = (0..13)
            .map(|k| signal(k, k, d(9), OPEN + 600 + 10 * k, 0, OPEN + 20_000))
            .collect();
        let out = run(&sigs, 12, &wide, &r, i128::from(30 * CRORE));
        assert!(
            out.admissions
                .iter()
                .take(12)
                .all(|a| a.amount == Some(75 * LAKH / 10))
        );
        assert_eq!(out.admissions[12].status, Status::SlotsFull);
    }

    #[test]
    fn zero_fill_entries_release_their_count_under_the_headline() {
        let dead = Outcome {
            status: Status::Ok,
            filled: 0,
            first_fill: None,
            end: OPEN + 650,
            net: 0,
            charges: 0,
        };
        let mut sigs: Vec<Signal> = (0..5)
            .map(|k| Signal {
                outcomes: [dead; 5],
                ..signal(k, k, d(12), OPEN + 600 + k, 0, OPEN + 650)
            })
            .collect();
        sigs.push(signal(9, 9, d(12), OPEN + 700, 5_000, OPEN + 760));
        let head = run(&sigs, 5, &MAIN, &Readings::HEADLINE, i128::from(CAPITAL));
        assert_eq!(head.admissions[5].status, Status::Ok);
        assert_eq!(head.trades.len(), 1);
        let mut r = Readings::HEADLINE;
        r.zero_fill_count = ZeroFillCount::Kept;
        let kept = run(&sigs, 5, &MAIN, &r, i128::from(CAPITAL));
        assert_eq!(kept.admissions[5].status, Status::DayCap);
    }

    #[test]
    fn months_drawdown_and_the_data_charge_readings() {
        let sigs = [
            signal(
                1,
                1,
                Date {
                    y: 2026,
                    m: 1,
                    d: 5,
                },
                OPEN + 600,
                300_000,
                OPEN + 700,
            ),
            signal(
                2,
                2,
                Date {
                    y: 2026,
                    m: 1,
                    d: 6,
                },
                OPEN + 600,
                -500_000,
                OPEN + 700,
            ),
            signal(
                3,
                3,
                Date {
                    y: 2026,
                    m: 2,
                    d: 2,
                },
                OPEN + 600,
                100_000,
                OPEN + 700,
            ),
        ];
        let out = run(&sigs, 5, &MAIN, &Readings::HEADLINE, i128::from(CAPITAL));
        assert_eq!(out.months.len(), 2);
        assert_eq!(
            out.months[0].net,
            300_000 - 500_000 - i128::from(DATA_CHARGE)
        );
        assert_eq!(out.months[0].wins, 1);
        assert_eq!(out.months[0].worst, -500_000);
        // the curve: -58,882, +300,000, -500,000, -58,882, +100,000
        assert_eq!(out.max_dd, 500_000 + i128::from(DATA_CHARGE));
        assert_eq!(out.max_dd_tenths, 2);
        let mut r = Readings::HEADLINE;
        r.dd_round = DdRound::Ceil;
        assert_eq!(
            run(&sigs, 5, &MAIN, &r, i128::from(CAPITAL)).max_dd_tenths,
            3
        );
        r = Readings::HEADLINE;
        r.data_charge_at = DataChargeAt::OutOfCurve;
        let oc = run(&sigs, 5, &MAIN, &r, i128::from(CAPITAL));
        assert_eq!(oc.max_dd, 500_000);
        assert_eq!(oc.months[0].net, out.months[0].net);
        assert_eq!(
            out.end_equity,
            i128::from(CAPITAL) - 100_000 - 2 * i128::from(DATA_CHARGE)
        );
        assert_eq!(dd_tenths(0, DdRound::Ceil), 0);
        assert_eq!(dd_tenths(1, DdRound::Ceil), 1);
        assert_eq!(dd_tenths(199_999, DdRound::Floor), 0);
        assert_eq!(dd_tenths(200_000, DdRound::Floor), 1);
    }

    #[test]
    fn an_earlier_fill_moves_the_months_debit_forward() {
        // the second admitted trade fills before the first one does
        let mut a = signal(1, 1, d(20), OPEN + 600, 1_000, OPEN + 3_000);
        a.outcomes = [Outcome {
            first_fill: Some(OPEN + 2_000),
            ..a.outcomes[0]
        }; 5];
        let mut b = signal(2, 2, d(20), OPEN + 700, 2_000, OPEN + 800);
        b.outcomes = [Outcome {
            first_fill: Some(OPEN + 720),
            ..b.outcomes[0]
        }; 5];
        let out = run(&[a, b], 5, &MAIN, &Readings::HEADLINE, i128::from(CAPITAL));
        assert_eq!(
            out.end_equity,
            i128::from(CAPITAL) + 3_000 - i128::from(DATA_CHARGE)
        );
        assert_eq!(out.max_dd, i128::from(DATA_CHARGE));
    }
}

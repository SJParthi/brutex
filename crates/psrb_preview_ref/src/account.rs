//! F55 `account::MonthAgg` and F56 `account::DrawdownWalker` (PREVIEW-SPEC sections 3.8
//! and 3.9).

use crate::market::Window;
use crate::sizing::ACCOUNT;
use crate::time::{Day, Month};

/// One closed trade as the account reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Closed {
    /// The IST day of the exit fill (always the entry's day).
    pub day: Day,
    /// The exit fill's second.
    pub second: u32,
    /// The trade's net, paisa.
    pub net: i128,
}

/// One month of a row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MonthRow {
    /// The month.
    pub month: Month,
    /// Capital at the month's start: the account plus every earlier net (it compounds).
    pub capital_start: i128,
    /// The month's net.
    pub net: i128,
    /// Trades closed in the month.
    pub trades: u64,
    /// `(10_000 x net).div_euclid(capital_start)`; `None` when the capital is at or below 0.
    pub bp: Option<i128>,
    /// `10_000 x net >= 250 x capital_start` with positive capital.
    pub pass: bool,
}

/// F55: the window's months, each judged; trades attributed to their exit day's month.
#[must_use]
pub fn months(window: &Window, closed: &[Closed]) -> Vec<MonthRow> {
    let mut capital = i128::from(ACCOUNT);
    let mut out = Vec::new();
    for month in window.months() {
        let inside = closed.iter().filter(|c| c.day.month() == month);
        let net: i128 = inside.clone().map(|c| c.net).sum();
        let trades = u64::try_from(inside.count()).unwrap_or(u64::MAX);
        let positive = capital > 0;
        let bp = positive.then(|| (10_000 * net).div_euclid(capital));
        let pass = positive && 10_000 * net >= 250 * capital;
        out.push(MonthRow {
            month,
            capital_start: capital,
            net,
            trades,
            bp,
            pass,
        });
        capital += net;
    }
    out
}

/// The deepest fall of the account from a peak to a later low.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Drawdown {
    /// `(10_000 x (peak - equity)).div_euclid(peak)` at its deepest mark.
    pub bp: i128,
    /// `peak - equity` there, paisa.
    pub paisa: i128,
    /// The peak's day (`None`: the start).
    pub peak_day: Option<Day>,
    /// The low's day (`None`: no fall).
    pub low_day: Option<Day>,
}

/// F56: equity marked after every exit in time order (exit second, then symbol; the
/// caller's order); the peak includes the start; the first of two equal deepest falls
/// keeps its dates (Q9). Day-end marks repeat the day's last exit mark and change nothing.
#[must_use]
pub fn drawdown(closed: &[Closed]) -> Drawdown {
    let mut equity = i128::from(ACCOUNT);
    let mut peak = equity;
    let mut peak_day = None;
    let mut deepest = Drawdown {
        bp: 0,
        paisa: 0,
        peak_day: None,
        low_day: None,
    };
    for c in closed {
        equity += c.net;
        if equity > peak {
            peak = equity;
            peak_day = Some(c.day);
            continue;
        }
        let fall = peak - equity;
        let bp = (10_000 * fall).div_euclid(peak);
        if fall > 0 && (deepest.low_day.is_none() || bp > deepest.bp) {
            deepest = Drawdown {
                bp,
                paisa: fall,
                peak_day,
                low_day: Some(c.day),
            };
        }
    }
    deepest
}

#[cfg(test)]
#[path = "account_tests.rs"]
mod tests;

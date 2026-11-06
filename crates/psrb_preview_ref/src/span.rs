//! F04 `span::trading_span` and F05 `span::warmup_start` (PREVIEW-SPEC section 3.3).

use crate::market::{Calendar, Window};
use crate::time::{Day, Month};

/// Why a stock's month lists were refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// A month list out of order (the first month out of place).
    Unsorted(Month),
    /// One month listed twice (two served generations).
    Duplicate(Month),
    /// The trading start month has no Zerodha month (Q1, pinned: refused for the stock).
    StartMonthAbsent(Month),
}

/// A month list must rise strictly.
///
/// # Errors
/// [`Refusal::Duplicate`] for a repeated month, [`Refusal::Unsorted`] for a fall.
pub fn check_months(months: &[Month]) -> Result<(), Refusal> {
    for (a, b) in months.iter().zip(months.iter().skip(1)) {
        if a == b {
            return Err(Refusal::Duplicate(*b));
        }
        if a > b {
            return Err(Refusal::Unsorted(*b));
        }
    }
    Ok(())
}

/// The first and last full-session days of a window, `None` when it holds none.
#[must_use]
pub fn window_days(calendar: &Calendar, window: &Window) -> Option<(Day, Day)> {
    let first = window.first.first_day()?;
    let last = window.last.next().first_day()?.index() - 1;
    let days = calendar.full_days(first, Day::from_index(last));
    Some((*days.first()?, *days.last()?))
}

/// F04: the stock's trading span, first to last day inclusive: from the first full session
/// of the later of its first Zerodha and first GDFL month (never before the window's
/// first session) to the window's last session. `None` (counted "no trading span") when
/// either feed holds no month or the start falls after the window.
///
/// # Errors
/// A month list that is not strictly rising.
pub fn trading_span(
    zerodha: &[Month],
    gdfl: &[Month],
    window: &Window,
    calendar: &Calendar,
) -> Result<Option<(Day, Day)>, Refusal> {
    check_months(zerodha)?;
    check_months(gdfl)?;
    let (Some(z), Some(g)) = (zerodha.first(), gdfl.first()) else {
        return Ok(None);
    };
    let Some((window_first, window_last)) = window_days(calendar, window) else {
        return Ok(None);
    };
    let start_month = (*z).max(*g);
    let Some(month_first) = start_month.first_day() else {
        return Ok(None);
    };
    let from = month_first.max(window_first);
    Ok(calendar.full_days(from, window_last).first().map(|start| (*start, window_last)))
}

/// F05: the first month of the contiguous run of Zerodha months that holds the trading
/// start month; the warm-up folds every month of that run before the start.
///
/// # Errors
/// [`Refusal::StartMonthAbsent`] when the start month has no Zerodha month (Q1), and the
/// refusals of [`check_months`].
pub fn warmup_start(zerodha: &[Month], start: Month) -> Result<Month, Refusal> {
    check_months(zerodha)?;
    if zerodha.binary_search(&start).is_err() {
        return Err(Refusal::StartMonthAbsent(start));
    }
    let mut first = start;
    while zerodha.binary_search(&first.prev()).is_ok() {
        first = first.prev();
    }
    Ok(first)
}

#[cfg(test)]
#[path = "span_tests.rs"]
mod tests;

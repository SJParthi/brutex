//! F11 `dayref::PrevSession`: the previous full session's high, low and close for C1
//! and C2 (PREVIEW-SPEC section 3.4, "Previous session").

use crate::candles::session_close;
use crate::indicators::check_evaluable;
use crate::market::Minute;
use crate::time::{Day, OPEN};

/// A day bar that passed the evaluability check: the only form C1 and C2 accept.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AcceptedBar {
    high: i64,
    low: i64,
    close: i64,
}

impl AcceptedBar {
    /// A reference from a session's high, low and close, accepted only when
    /// `0 < low <= close <= high`.
    #[must_use]
    pub fn from_hlc(high: i64, low: i64, close: i64) -> Option<Self> {
        let bar = crate::market::Bar::new(close, high, low, close, 0);
        check_evaluable(&bar).ok()?;
        Some(Self { high, low, close })
    }

    /// The session high.
    #[must_use]
    pub const fn high(&self) -> i64 {
        self.high
    }
    /// The session low.
    #[must_use]
    pub const fn low(&self) -> i64 {
        self.low
    }
    /// The last traded minute's close.
    #[must_use]
    pub const fn close(&self) -> i64 {
        self.close
    }
}

/// The day bar of a full session (the fold at `Bucket::DAY`): the session's traded
/// minutes folded into one bar, accepted only when it passes the evaluability check.
/// `None` for a day with no traded minute (Q2) or a refused bar.
#[must_use]
pub fn day_bar(day: Day, minutes: &[Minute]) -> Option<AcceptedBar> {
    let close = session_close(day, minutes);
    let inside: Vec<Minute> = minutes
        .iter()
        .filter(|m| m.start >= OPEN && m.start < close)
        .copied()
        .collect();
    let bar = crate::candles::fold_bar(&inside)?;
    check_evaluable(&bar).ok()?;
    Some(AcceptedBar {
        high: bar.high,
        low: bar.low,
        close: bar.close,
    })
}

/// The previous-session reference, rolled once per full session of the candle span.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PrevSession {
    last: Option<AcceptedBar>,
}

impl PrevSession {
    /// The reference C1 and C2 read today: the previous full session's accepted bar, or
    /// `None` (first day of the span, a refused bar, or a day without one): no earlier
    /// bar is ever read.
    #[must_use]
    pub const fn reference(&self) -> Option<AcceptedBar> {
        self.last
    }

    /// Ends today: tomorrow reads today's bar.
    pub const fn close_day(&mut self, today: Option<AcceptedBar>) {
        self.last = today;
    }
}

#[cfg(test)]
#[path = "dayref_tests.rs"]
mod tests;

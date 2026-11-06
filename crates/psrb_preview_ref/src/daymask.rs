//! F06 `daymask::admit`: may a stock-day trade (PREVIEW-SPEC section 3.3, day mask)?

use crate::market::DayKind;
use crate::membership::Answer;
use crate::time::Day;

/// 2021-02-24: the NSE outage day (skip row 3). It never trades; its candles still fold.
#[must_use]
pub fn outage_day() -> Option<Day> {
    Day::from_ymd(2021, 2, 24)
}

/// 2019-05-30: GDFL's files start at 11:57:47 (skip row 4). Every stock fills on minutes.
#[must_use]
pub fn vendor_late_day() -> Option<Day> {
    Day::from_ymd(2019, 5, 30)
}

/// A failing check, in the fixed order M1..M5, then the skip row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fail {
    /// M1: the name is not in the universe.
    NotListed,
    /// M2: not a regular full session (the day's kind).
    NotFullSession(DayKind),
    /// M3: membership is not `Yes` (asked only on a full session).
    NotMember(Answer),
    /// M4: outside the window or the stock's trading span.
    OutsideSpan,
    /// M5: the stock has no Zerodha minute that day.
    NoMinutes,
    /// The market-wide skip row: 2021-02-24 never trades.
    SkipRow,
}

/// The day mask's verdict for one stock-day.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// Trades, filling on GDFL seconds where they exist.
    Trade,
    /// Trades, every fill on Zerodha minutes (2019-05-30).
    TradeFallback,
    /// Does not trade: every failing check, in order.
    NoTrade(Vec<Fail>),
}

/// The facts the mask reads for one stock-day.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Facts {
    /// M1: listed in the universe.
    pub listed: bool,
    /// M2: the calendar's kind of the day.
    pub kind: DayKind,
    /// M3: the membership answer (read only when the day is a full session).
    pub member: Answer,
    /// M4: inside the window and the stock's trading span.
    pub in_span: bool,
    /// M5: the stock has Zerodha minutes that day.
    pub has_minutes: bool,
}

/// F06: the verdict for `day` given its facts.
#[must_use]
pub fn admit(day: Day, facts: &Facts) -> Verdict {
    let mut fails = Vec::new();
    if !facts.listed {
        fails.push(Fail::NotListed);
    }
    if facts.kind == DayKind::Full {
        if facts.member != Answer::Yes {
            fails.push(Fail::NotMember(facts.member));
        }
    } else {
        fails.push(Fail::NotFullSession(facts.kind));
    }
    if !facts.in_span {
        fails.push(Fail::OutsideSpan);
    }
    if !facts.has_minutes {
        fails.push(Fail::NoMinutes);
    }
    if Some(day) == outage_day() {
        fails.push(Fail::SkipRow);
    }
    if !fails.is_empty() {
        return Verdict::NoTrade(fails);
    }
    if Some(day) == vendor_late_day() {
        Verdict::TradeFallback
    } else {
        Verdict::Trade
    }
}

#[cfg(test)]
#[path = "daymask_tests.rs"]
mod tests;

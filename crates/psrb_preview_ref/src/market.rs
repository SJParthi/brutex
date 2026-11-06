//! The in-memory market model every function reads: synthetic only.
//!
//! A stock carries Zerodha 1-minute bars (corporate-action ADJUSTED, the decision
//! basis) and GDFL 1-second bars (RAW, the fill basis), each per stock-day, plus the
//! months each vendor holds (PREVIEW-SPEC section 2). Prices are paisa.

use std::collections::{BTreeMap, BTreeSet};

use crate::time::{Day, Month};

/// One bar: open, high, low, close in paisa and the traded volume.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Bar {
    /// First price.
    pub open: i64,
    /// Highest price.
    pub high: i64,
    /// Lowest price.
    pub low: i64,
    /// Last price.
    pub close: i64,
    /// Traded quantity.
    pub volume: i64,
}

impl Bar {
    /// A bar whose four prices are all `price`.
    #[must_use]
    pub const fn flat(price: i64, volume: i64) -> Self {
        Self {
            open: price,
            high: price,
            low: price,
            close: price,
            volume,
        }
    }

    /// A bar from its five fields.
    #[must_use]
    pub const fn new(open: i64, high: i64, low: i64, close: i64, volume: i64) -> Self {
        Self {
            open,
            high,
            low,
            close,
            volume,
        }
    }
}

/// A stored Zerodha minute: its start second of the IST day and its bar.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Minute {
    /// Start of the minute, seconds of the IST day.
    pub start: u32,
    /// The minute's bar.
    pub bar: Bar,
}

impl Minute {
    /// A minute with no trade: the vendor's padded, flat, zero-volume record.
    #[must_use]
    pub const fn padded(&self) -> bool {
        self.bar.volume == 0
    }
}

/// A stored GDFL second: the second of the IST day and its bar (high and low are the
/// largest and smallest traded price; a second exists only if it traded, Rule T).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Second {
    /// The second of the IST day.
    pub at: u32,
    /// The second's bar.
    pub bar: Bar,
}

/// One stock's stored records for one day.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct StockDay {
    /// Zerodha minutes in file order.
    pub minutes: Vec<Minute>,
    /// GDFL seconds in file order (empty when the vendor holds none that day).
    pub seconds: Vec<Second>,
}

/// One stock: its spelling, its stored days and the months each vendor holds.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Stock {
    /// The universe spelling, e.g. `M&M`.
    pub name: String,
    /// Stored records by day.
    pub days: BTreeMap<Day, StockDay>,
    /// Months the Zerodha 1-minute series holds.
    pub zerodha_months: BTreeSet<Month>,
    /// Months the GDFL 1-second series holds.
    pub gdfl_months: BTreeSet<Month>,
    /// GDFL months that exist but refuse to open (the `open_existing` refusal).
    pub gdfl_refused: BTreeSet<Month>,
    /// Days the fold withholds (every scheduled bucket of such a day is a hole).
    pub withheld: BTreeSet<Day>,
}

/// What the exchange calendar says of a day.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DayKind {
    /// A regular full session (`DayKind::Open(Session::full())`, the only M2 day).
    Full,
    /// Open, but not a full session (a special session).
    Short,
    /// The Muhurat session (`OpenLengthUnmeasured`).
    Muhurat,
    /// No session.
    Closed,
}

/// The exchange calendar of a market: Monday to Friday are full sessions and the
/// weekend is closed, unless an override names the day.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Calendar {
    /// Days whose kind differs from the weekday rule.
    pub overrides: BTreeMap<Day, DayKind>,
}

impl Calendar {
    /// The kind of `day`.
    #[must_use]
    pub fn kind_of(&self, day: Day) -> DayKind {
        match self.overrides.get(&day) {
            Some(kind) => *kind,
            None if day.weekday() >= 5 => DayKind::Closed,
            None => DayKind::Full,
        }
    }

    /// The full-session days from `first` to `last` inclusive, in order.
    #[must_use]
    pub fn full_days(&self, first: Day, last: Day) -> Vec<Day> {
        let mut out = Vec::new();
        let mut d = first;
        while d <= last {
            if self.kind_of(d) == DayKind::Full {
                out.push(d);
            }
            d = d.next();
        }
        out
    }
}

/// The judged months, first to last inclusive.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Window {
    /// First judged month.
    pub first: Month,
    /// Last judged month.
    pub last: Month,
}

impl Window {
    /// Every month of the window, in order.
    #[must_use]
    pub fn months(&self) -> Vec<Month> {
        let mut out = Vec::new();
        let mut m = self.first;
        while m <= self.last {
            out.push(m);
            m = m.next();
        }
        out
    }
}

/// Every input of one run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Market {
    /// The exchange calendar.
    pub calendar: Calendar,
    /// The judged months.
    pub window: Window,
    /// The stocks, in any order (the run orders them by spelling).
    pub stocks: Vec<Stock>,
    /// The dated membership answers.
    pub membership: crate::membership::Membership,
}

#[cfg(test)]
#[path = "market_tests.rs"]
mod tests;

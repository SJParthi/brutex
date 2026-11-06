//! The synthetic market generator: fixed seeds, step-shaped price paths, and the
//! scenario catalogue whose golden files the Mac replays (TEST-PLAN section 2's extreme
//! cases; PREVIEW-SPEC sections 3.3..3.10).
//!
//! A day's Zerodha path is a step function of the minute index (0 = 09:15): flat runs
//! and jumps. A flat run makes sigma 0, an RSI of 100 or 0 after a jump, and a CCI at
//! its ceiling, so one jump after a quiet spell is a signal for every rule set whose
//! readings are warm: that is how signals are planted. GDFL seconds are drawn inside
//! each traded minute, raw (`factor` x the adjusted minute price).

use std::collections::{BTreeMap, BTreeSet};

use crate::market::{Bar, Calendar, DayKind, Market, Minute, Second, Stock, StockDay, Window};
use crate::membership::{Answer, Membership, Probe};
use crate::rng::SplitMix64;
use crate::time::{CLOSE_FULL, Day, Month, OPEN};

/// The four base seeds of TEST-PLAN section 0.1.
pub const SEEDS: [u64; 4] = [0x5053_5242_0000_0001, 0x5053_5242_0000_0002, 0x5053_5242_0000_0003, 0x5053_5242_0000_0004];

/// One named scenario: its inputs and what it plants.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scenario {
    /// The file stem, e.g. `s001-gap-up-time-exit`.
    pub name: String,
    /// The seed its random parts used (0 for a hand scenario with none).
    pub seed: u64,
    /// What it plants, in one line.
    pub about: String,
    /// The market.
    pub market: Market,
}

/// One minute of a step path: the minute index (0 = 09:15) and its bar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Step {
    /// Minute index from 09:15.
    pub index: u32,
    /// The minute's bar.
    pub bar: Bar,
}

/// How GDFL seconds are drawn for a day.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Seconds {
    /// One second at :01 of every traded minute, at the minute's close (x factor).
    OnePerMinute,
    /// No GDFL second at all that day (a fallback day if Zerodha has minutes).
    None,
}

/// A day under construction.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DayPlan {
    /// The minutes by index (later writes replace earlier ones).
    pub minutes: BTreeMap<u32, Bar>,
    /// Extra or replacing GDFL seconds by second of the day.
    pub seconds: BTreeMap<u32, Bar>,
    /// Seconds removed after drawing (by second of the day).
    pub drop_seconds: BTreeSet<u32>,
    /// Minutes removed (missing records: holes).
    pub drop_minutes: BTreeSet<u32>,
    /// GDFL raw price = factor x adjusted price.
    pub factor: i64,
    /// The seconds rule.
    pub rule: Option<Seconds>,
    /// No second at or after this second of the day (a halt).
    pub halt_from: Option<u32>,
}

/// The volume every synthetic traded minute carries.
pub const VOLUME: i64 = 1_000;

impl DayPlan {
    /// A day of 375 flat minutes at `price` (or 360 when `minutes` says so).
    #[must_use]
    pub fn flat(price: i64, count: u32) -> Self {
        let mut plan = Self { factor: 1, rule: Some(Seconds::OnePerMinute), ..Self::default() };
        for i in 0..count {
            plan.minutes.insert(i, Bar::flat(price, VOLUME));
        }
        plan
    }

    /// From minute `from` on, the price is `price` (flat minutes), up to `count` minutes.
    #[must_use]
    pub fn level(mut self, from: u32, count: u32, price: i64) -> Self {
        for i in from..count {
            self.minutes.insert(i, Bar::flat(price, VOLUME));
        }
        self
    }

    /// Sets one minute's bar.
    #[must_use]
    pub fn minute(mut self, index: u32, bar: Bar) -> Self {
        self.minutes.insert(index, bar);
        self
    }

    /// Sets one GDFL second's bar (raw prices, not multiplied by the factor).
    #[must_use]
    pub fn second(mut self, at: u32, bar: Bar) -> Self {
        self.seconds.insert(at, bar);
        self
    }

    /// Removes a minute record (a missing minute).
    #[must_use]
    pub fn drop_minute(mut self, index: u32) -> Self {
        self.drop_minutes.insert(index);
        self
    }

    /// Removes a drawn second.
    #[must_use]
    pub fn drop_second(mut self, at: u32) -> Self {
        self.drop_seconds.insert(at);
        self
    }

    /// Pads minutes `from..to`: flat at the last price, volume 0 (no trade).
    #[must_use]
    pub fn pad(mut self, from: u32, to: u32) -> Self {
        for i in from..to {
            let price = self.minutes.range(..i).next_back().map_or(100_000, |(_, b)| b.close);
            self.minutes.insert(i, Bar::flat(price, 0));
        }
        self
    }

    /// GDFL prices are `factor` x the adjusted minute prices (a split day: 5).
    #[must_use]
    pub const fn factor(mut self, factor: i64) -> Self {
        self.factor = factor;
        self
    }

    /// The seconds rule.
    #[must_use]
    pub const fn seconds_rule(mut self, rule: Seconds) -> Self {
        self.rule = Some(rule);
        self
    }

    /// No GDFL second from `at` on.
    #[must_use]
    pub const fn halt(mut self, at: u32) -> Self {
        self.halt_from = Some(at);
        self
    }

    /// The stored day.
    #[must_use]
    pub fn build(&self) -> StockDay {
        let minutes: Vec<Minute> = self
            .minutes
            .iter()
            .filter(|(i, _)| !self.drop_minutes.contains(i))
            .map(|(i, bar)| Minute { start: OPEN + 60 * i, bar: *bar })
            .collect();
        let mut seconds: BTreeMap<u32, Bar> = BTreeMap::new();
        if self.rule == Some(Seconds::OnePerMinute) {
            for m in minutes.iter().filter(|m| !m.padded() && m.start < CLOSE_FULL) {
                let p = m.bar.close * self.factor;
                seconds.insert(m.start + 1, Bar::flat(p, 10));
            }
        }
        for (at, bar) in &self.seconds {
            seconds.insert(*at, *bar);
        }
        let seconds: Vec<Second> = seconds
            .into_iter()
            .filter(|(at, _)| !self.drop_seconds.contains(at) && self.halt_from.is_none_or(|h| *at < h))
            .map(|(at, bar)| Second { at, bar })
            .collect();
        StockDay { minutes, seconds }
    }
}

/// A stock under construction.
#[derive(Clone, Debug, Default)]
pub struct StockPlan {
    /// The stock.
    pub stock: Stock,
}

impl StockPlan {
    /// A stock named `name` with no data.
    #[must_use]
    pub fn new(name: &str) -> Self {
        Self { stock: Stock { name: name.to_owned(), ..Stock::default() } }
    }

    /// Adds a day: its months join the Zerodha list (and the GDFL list when `gdfl`).
    #[must_use]
    pub fn day(mut self, day: Day, plan: &DayPlan, gdfl: bool) -> Self {
        self.stock.zerodha_months.insert(day.month());
        if gdfl {
            self.stock.gdfl_months.insert(day.month());
        }
        self.stock.days.insert(day, plan.build());
        self
    }

    /// Adds flat warm-up days (Zerodha only) on every full session of `months`.
    #[must_use]
    pub fn warmup(mut self, calendar: &Calendar, months: &[Month], price: i64) -> Self {
        let plan = DayPlan::flat(price, 375).seconds_rule(Seconds::None);
        for m in months {
            self.stock.zerodha_months.insert(*m);
            for d in calendar.full_days(m.start(), m.end()) {
                self.stock.days.insert(d, plan.build());
            }
        }
        self
    }

    /// Marks a GDFL month present (or refused).
    #[must_use]
    pub fn gdfl_month(mut self, month: Month, refused: bool) -> Self {
        self.stock.gdfl_months.insert(month);
        if refused {
            self.stock.gdfl_refused.insert(month);
        }
        self
    }

    /// Marks a Zerodha month present without days.
    #[must_use]
    pub fn zerodha_month(mut self, month: Month) -> Self {
        self.stock.zerodha_months.insert(month);
        self
    }

    /// Withholds a day.
    #[must_use]
    pub fn withhold(mut self, day: Day) -> Self {
        self.stock.withheld.insert(day);
        self
    }

    /// The stock.
    #[must_use]
    pub fn build(self) -> Stock {
        self.stock
    }
}

/// A market of `stocks` over `window`, every listed stock a member every day.
#[must_use]
pub fn market(window: Window, calendar: Calendar, stocks: Vec<Stock>) -> Market {
    let names: Vec<&str> = stocks.iter().map(|s| s.name.as_str()).collect();
    let membership = Membership::all_members(&names);
    Market { calendar, window, stocks, membership }
}

/// A day that must exist (the catalogue names only real dates).
#[must_use]
pub fn day(y: i32, m: u32, d: u32) -> Day {
    Day::from_ymd(y, m, d).unwrap_or_else(|| Day::from_index(0))
}

/// The membership of a market with one answer changed.
#[must_use]
pub fn with_answer(mut market: Market, name: &str, on: Day, answer: Answer) -> Market {
    market.membership.answers.insert((name.to_owned(), on), answer);
    market
}

/// The membership probe of a market replaced.
#[must_use]
pub const fn with_probe(mut market: Market, probe: Probe) -> Market {
    market.membership.probe = probe;
    market
}

/// A calendar with overrides.
#[must_use]
pub fn calendar(overrides: &[(Day, DayKind)]) -> Calendar {
    Calendar { overrides: overrides.iter().copied().collect() }
}

/// A random draw helper over the seeded generator.
#[derive(Clone, Debug)]
pub struct Draw {
    rng: SplitMix64,
}

impl Draw {
    /// A draw from `seed`.
    #[must_use]
    pub const fn new(seed: u64) -> Self {
        Self { rng: SplitMix64::new(seed) }
    }

    /// Uniform in `lo..=hi`.
    pub fn int(&mut self, lo: i64, hi: i64) -> i64 {
        self.rng.between(lo, hi)
    }

    /// Uniform in `lo..=hi` (u32).
    pub fn index(&mut self, lo: u32, hi: u32) -> u32 {
        u32::try_from(self.rng.between(i64::from(lo), i64::from(hi))).unwrap_or(lo)
    }

    /// True with probability `num / den`.
    pub fn chance(&mut self, num: u64, den: u64) -> bool {
        self.rng.chance(num, den)
    }
}

#[cfg(test)]
#[path = "generator_tests.rs"]
mod tests;

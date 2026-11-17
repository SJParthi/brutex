//! Rule set v2 (SNIPER s25b ruling, operator 14 Nov 2026 23:1x IST) and the NSE trading calendar used for the expiry facts.
//!
//! v2 is this binary's default; `run --rules v1` replays the v1 rules exactly (the regression run against `sf-20261114-215542`).
//! - R1: at most ONE open position per (underlying, side CE/PE, expiry) at any instant. A CE and a PE of the same underlying may be open
//!   together. Every other event of an already-held key is refused and counted: `same_side_dup` (another strike of the held key) and
//!   `contract_open` (the very same contract again, v1's rule, kept).
//! - R2: a rank-top-K list keeps only the best-ranked strike per (underlying, side, expiry) at each candle end; the next distinct key fills
//!   the K (a one-underlying index list therefore gives at most 2 events per candle end: its best CE and its best PE).
//! - R3: stock options never on their own expiry day: on that day the stock list is the underlying's NEXT expiry (the day's own instrument
//!   snapshot); when Tickvault has no next-expiry contract that day, the expiring list's events are refused and counted
//!   (`stock_expiry_no_next`).
//! - R4: hard cap (SNIPER s26 addendum 2): the margin in use (every open position's entry estimate, blocked from T to its last fill)
//!   never passes Rs 20 lakh at any instant; a trade that would pass it is refused and counted (`margin_cap_20L`). The v1 cash rule
//!   (lost money never reused: `margin_cash_short`) stays after it.
//!
//! The calendar (NSE's own trading-holiday list, `tools-work/calendar-ext/nse-holidays.tsv`, read only) is used for the printed expiry
//! facts only (is the expiry a trading day, the Tuesday it stands for, the holiday shift, sessions from the trade day to the expiry); no
//! decision reads it.

use std::collections::BTreeMap;
use std::path::Path;

use crate::cal::{civil, fmt_day, parse_day};

/// The rule set of one evaluation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rules {
    /// v2 (R1-R4) when true; v1 when false.
    pub v2: bool,
}

impl Rules {
    /// The printed name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        if self.v2 { "v2" } else { "v1" }
    }
}

/// Weekday of a day number (days since 1970-01-01, a Thursday): 0 Monday .. 6 Sunday.
#[must_use]
pub const fn weekday(day: i64) -> i64 {
    (day + 3).rem_euclid(7)
}

/// Weekday names.
pub const WD: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

/// The weekday name of a day.
#[must_use]
pub fn wd_name(day: i64) -> &'static str {
    WD.get(usize::try_from(weekday(day)).unwrap_or(0)).copied().unwrap_or("?")
}

/// NSE trading holidays (day -> name).
#[derive(Clone, Debug, Default)]
pub struct Calendar {
    pub hol: BTreeMap<i64, String>,
    pub source: String,
}

/// The facts of one snapshot expiry.
#[derive(Clone, Debug, Default)]
pub struct ExpFacts {
    /// The expiry is a regular trading day (Monday-Friday, not an NSE trading holiday).
    pub trading: bool,
    /// The Tuesday it stands for (the first Tuesday on or after it; NSE expiries are Tuesdays from 2025-09-01).
    pub nominal: i64,
    /// It is the trading day at or before that Tuesday (a Tuesday expiry, or the holiday shift of one).
    pub ok: bool,
    /// The holidays between it and that Tuesday (the shift), in words; empty when not shifted.
    pub shift: String,
    /// That Tuesday is the last Tuesday of its month (a monthly expiry).
    pub last_tue: bool,
}

impl Calendar {
    /// Reads NSE's holiday list (`YYYY-MM-DD<TAB>name` lines; `#` lines are notes).
    ///
    /// # Errors
    /// An unreadable file or no holiday row.
    pub fn load(p: &Path) -> Result<Self, String> {
        let t = std::fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()))?;
        let mut hol = BTreeMap::new();
        for l in t.lines() {
            if l.starts_with('#') || l.trim().is_empty() {
                continue;
            }
            let (d, n) = l.split_once('\t').unwrap_or((l, ""));
            if let Some(day) = parse_day(d) {
                hol.insert(day, n.trim().to_owned());
            }
        }
        if hol.is_empty() {
            return Err(format!("{}: no holiday row", p.display()));
        }
        Ok(Self { hol, source: p.display().to_string() })
    }

    /// A regular trading day: Monday-Friday and not an NSE trading holiday (a special session such as Muhurat is not regular).
    #[must_use]
    pub fn is_trading(&self, d: i64) -> bool {
        weekday(d) < 5 && !self.hol.contains_key(&d)
    }

    /// Regular trading days in (from, to]: 0 on the expiry day itself, 1 when `to` is the next regular trading day ("tomorrow's expiry").
    #[must_use]
    pub fn sessions(&self, from: i64, to: i64) -> i64 {
        if to <= from {
            return 0;
        }
        i64::try_from(((from + 1)..=to).filter(|&d| self.is_trading(d)).count()).unwrap_or(0)
    }

    /// The regular trading day at or before `d`.
    #[must_use]
    pub fn prev_trading(&self, d: i64) -> i64 {
        let mut x = d;
        for _ in 0..30 {
            if self.is_trading(x) {
                return x;
            }
            x -= 1;
        }
        x
    }

    /// The facts of a snapshot expiry.
    #[must_use]
    pub fn expiry_facts(&self, e: i64) -> ExpFacts {
        let nominal = e + (1 - weekday(e)).rem_euclid(7);
        let ok = self.is_trading(e) && self.prev_trading(nominal) == e;
        let mut sh: Vec<String> = Vec::new();
        for d in (e + 1)..=nominal {
            if let Some(n) = self.hol.get(&d) {
                sh.push(format!("{} {} ({n})", fmt_day(d), wd_name(d)));
            }
        }
        let (_, m0, _) = civil(nominal);
        let (_, m1, _) = civil(nominal + 7);
        ExpFacts { trading: self.is_trading(e), nominal, ok, shift: sh.join("; "), last_tue: m0 != m1 }
    }

    /// One line: "2026-11-09 Mon, trading day; stands for Tue 2026-11-10, shifted (holiday: 2026-11-10 Tue (Diwali-Balipratipada))".
    #[must_use]
    pub fn expiry_words(&self, e: i64) -> String {
        let f = self.expiry_facts(e);
        format!(
            "{} {}, {}; {}{}{}",
            fmt_day(e),
            wd_name(e),
            if f.trading { "a trading day" } else { "NOT a trading day" },
            if f.nominal == e { "a Tuesday expiry".to_owned() } else { format!("stands for Tue {}", fmt_day(f.nominal)) },
            if f.shift.is_empty() { String::new() } else { format!(", shifted to the trading day before (holiday {})", f.shift) },
            if f.ok { if f.last_tue { " (monthly: last Tuesday of the month)" } else { "" } } else { " — IRREGULAR: not the trading day at or before its Tuesday" }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cal::days_from_civil;

    fn cal() -> Calendar {
        let mut hol = BTreeMap::new();
        hol.insert(days_from_civil(2026, 11, 8), "Diwali Laxmi Pujan* (Muhurat trading)".to_owned());
        hol.insert(days_from_civil(2026, 11, 10), "Diwali-Balipratipada".to_owned());
        hol.insert(days_from_civil(2026, 11, 24), "Prakash Gurpurb Sri Guru Nanak Dev".to_owned());
        Calendar { hol, source: "test".to_owned() }
    }

    #[test]
    fn weekdays_and_holiday_shifted_expiries() {
        let c = cal();
        let d = |m: i64, dd: i64| days_from_civil(2026, m, dd);
        assert_eq!(wd_name(d(11, 3)), "Tue");
        assert_eq!(wd_name(d(11, 8)), "Sun");
        // NIFTY 03 Nov: a Tuesday expiry
        let f = c.expiry_facts(d(11, 3));
        assert!(f.ok && f.trading && f.shift.is_empty() && f.nominal == d(11, 3));
        // NIFTY 09 Nov: stands for Tue 10 Nov (holiday), shifted to Monday 09 Nov
        let f = c.expiry_facts(d(11, 9));
        assert!(f.ok && f.trading && f.nominal == d(11, 10) && f.shift.contains("Diwali-Balipratipada") && !f.last_tue);
        // stocks / BANKNIFTY 23 Nov: last Tuesday 24 Nov is a holiday -> Monday 23 Nov, monthly
        let f = c.expiry_facts(d(11, 23));
        assert!(f.ok && f.last_tue && f.nominal == d(11, 24));
        // a Monday that is NOT a holiday shift is irregular
        assert!(!c.expiry_facts(d(11, 16)).ok);
        // sessions to expiry: 02 Nov -> 03 Nov = 1 (tomorrow's expiry); 06 Nov (Fri) -> 09 Nov = 1 (the Muhurat Sunday is not regular);
        // 08 Nov (Muhurat) -> 09 Nov = 1; 09 Nov -> 09 Nov = 0 (expiry day); 11 Nov -> 17 Nov = 4
        assert_eq!(c.sessions(d(11, 2), d(11, 3)), 1);
        assert_eq!(c.sessions(d(11, 6), d(11, 9)), 1);
        assert_eq!(c.sessions(d(11, 8), d(11, 9)), 1);
        assert_eq!(c.sessions(d(11, 9), d(11, 9)), 0);
        assert_eq!(c.sessions(d(11, 11), d(11, 17)), 4);
        assert_eq!(c.sessions(d(11, 4), d(11, 9)), 3);
        assert!(!c.is_trading(d(11, 10)) && !c.is_trading(d(11, 8)) && c.is_trading(d(11, 9)));
    }
}

//! `cli series-identity` — whose bars a stored series holds. WP-13d; findings
//! DB-B07, DB-B08, DB-B10, DB-B22, DB-B23, DB-B60, DB-B64, DB-B66.
//!
//! # Usage
//!
//! ```text
//! cli series-identity INSTRUMENT [--segment INDEX|CASH] [--feed FEED]
//! cli series-identity ALL [--segment INDEX|CASH] [--feed FEED]
//! ```
//!
//! `--segment` is `INDEX` unless named; `--feed` is `zerodha` unless named.
//!
//! # What it does
//!
//! **It writes nothing.** Every stored 1min bar of the series is judged
//! against the series' own stored 1day bars, by the rule the landing door
//! applies to a broker's answer (`pull::series_identity`): a minute on a day
//! before the first served day bar, or wholly outside its own day's range by
//! more than 8%, is one a landing refuses today. Months already on disk hold
//! such minutes until a generation without them is published, so this verb
//! prints, per month, how many — the list a quarantine works from — and each
//! (day, cause). `ALL` prints one line per series of the segment, and names
//! every series that holds a day rung and no minute rung at all.
//!
//! # Exit
//!
//! [`OK`] when no stored minute would be refused and every month opened;
//! [`FAILED`] otherwise; [`MISUSED`] for arguments that name no series.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use brutex_core::vendor::Vendor;
use pull::ingest::identity_screen::{SeriesScan, scan_series};
use pull::series_identity::Tally;
use pull::session::Day;
use store::path::{FileKind, PathParts, Timeframe, YearMonth};

use crate::{FAILED, MISUSED, OK};

/// The verb's shape, printed with every misuse.
pub(crate) const USAGE: &str =
    "cli series-identity INSTRUMENT|ALL [--segment INDEX|CASH] [--feed FEED]";

/// What the arguments name.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Ask {
    /// `None` for `ALL`.
    instrument: Option<String>,
    segment: &'static str,
    feed: Vendor,
}

fn ask(arguments: &[&str]) -> Result<Ask, String> {
    let [first, flags @ ..] = arguments else {
        return Err("name an instrument, or ALL".to_owned());
    };
    let instrument = if *first == "ALL" {
        None
    } else {
        brutex_core::symbol::Symbol::new(first).map_err(|why| format!("`{first}`: {why}"))?;
        Some((*first).to_owned())
    };
    let (mut segment, mut feed) = ("INDEX", Vendor::Zerodha);
    let mut rest = flags.iter();
    while let Some(flag) = rest.next() {
        let value = rest.next().ok_or_else(|| format!("{flag} takes a value"))?;
        match *flag {
            "--segment" => {
                segment = match *value {
                    "INDEX" => "INDEX",
                    "CASH" => "CASH",
                    other => return Err(format!("--segment `{other}`: INDEX or CASH")),
                };
            }
            "--feed" => {
                feed = Vendor::ALL
                    .into_iter()
                    .find(|vendor| vendor.as_str() == *value)
                    .ok_or_else(|| format!("--feed `{value}`: not a feed this store names"))?;
            }
            other => return Err(format!("`{other}` is not a flag of this verb")),
        }
    }
    Ok(Ask {
        instrument,
        segment,
        feed,
    })
}

/// Runs the verb against the store root `store` names, so every arm is driven
/// by a test without the process environment.
pub(crate) fn command_with(
    arguments: &[&str],
    store: &dyn Fn() -> Result<PathBuf, String>,
    out: &mut String,
) -> u8 {
    let ask = match ask(arguments) {
        Ok(ask) => ask,
        Err(why) => {
            let _ = writeln!(out, "refused: {why}\nusage: {USAGE}");
            return MISUSED;
        }
    };
    let root = match store() {
        Ok(root) => root,
        Err(why) => {
            let _ = writeln!(out, "SERIES IDENTITY REFUSED: {why}. Nothing was read.");
            return FAILED;
        }
    };
    match &ask.instrument {
        Some(symbol) => one(&root, &ask, symbol, out),
        None => all(&root, &ask, out),
    }
}

/// `cli series-identity`: see the module header.
pub(crate) fn command(arguments: &[&str], out: &mut String) -> u8 {
    command_with(arguments, &crate::store_root, out)
}

fn scan(root: &Path, ask: &Ask, symbol: &str) -> Option<SeriesScan> {
    let month = YearMonth::new(2000, 1).ok()?;
    Some(scan_series(
        root,
        PathParts {
            vendor: ask.feed,
            exchange: "NSE",
            segment: ask.segment,
            symbol,
            contract: None,
            timeframe: Timeframe::MINUTE_1,
            month,
            file: FileKind::Bars,
        },
    ))
}

fn total(scan: &SeriesScan) -> Tally {
    let mut sum = Tally::default();
    for (_, tally) in &scan.months {
        let mut counts = tally.clone();
        counts.flagged.clear();
        sum.absorb(counts);
    }
    sum
}

fn span(months: &[YearMonth]) -> String {
    match (months.first(), months.last()) {
        (Some(first), Some(last)) => format!("{} months {first}..{last}", months.len()),
        _ => "none".to_owned(),
    }
}

fn day_text(day: i64) -> String {
    u32::try_from(day)
        .ok()
        .and_then(|days| Day::from_days(days).ok())
        .map_or_else(|| format!("IST day {day}"), |day| day.to_string())
}

/// Months holding at least one minute a landing refuses.
fn refused_months(scan: &SeriesScan) -> Vec<YearMonth> {
    scan.months
        .iter()
        .filter(|(_, tally)| tally.refused() > 0)
        .map(|(month, _)| *month)
        .collect()
}

fn one(root: &Path, ask: &Ask, symbol: &str, out: &mut String) -> u8 {
    let Some(scan) = scan(root, ask, symbol) else {
        return MISUSED;
    };
    let sum = total(&scan);
    let _ = writeln!(
        out,
        "SERIES IDENTITY (read-only; nothing is written) — {} NSE-{}-{symbol} under {}",
        ask.feed.as_str(),
        ask.segment,
        root.display()
    );
    let _ = writeln!(
        out,
        "  1day: {}; first served day {}",
        span(&scan.day_months),
        scan.first_day.map_or_else(|| "none".to_owned(), day_text)
    );
    let _ = writeln!(out, "  1min: {}", span(&scan.minute_months));
    let _ = writeln!(
        out,
        "  minutes {}: held {}, before_first_day_bar {}, foreign_level {}, suspect_level {}, \
         unjudged {} (no_day_rung {}, no_day_bar {}, day_bar_unusable {})",
        sum.minutes(),
        sum.held,
        sum.before_first_day_bar,
        sum.foreign_level,
        sum.suspect_level,
        sum.unchecked(),
        sum.no_day_rung,
        sum.no_day_bar,
        sum.day_bar_unusable
    );
    for (month, tally) in &scan.months {
        if tally.refused() + tally.suspect_level == 0 {
            continue;
        }
        let _ = writeln!(
            out,
            "  {month}: {} of {} minutes refused, {} suspect",
            tally.refused(),
            tally.minutes(),
            tally.suspect_level
        );
        for group in &tally.flagged {
            let _ = writeln!(
                out,
                "    {} {} {} minute(s)",
                day_text(group.day),
                group.code,
                group.minutes
            );
        }
    }
    for (month, why) in &scan.unread {
        let _ = writeln!(out, "  {month}: NOT READ: {why}");
    }
    let months = refused_months(&scan);
    if months.is_empty() && scan.unread.is_empty() {
        let _ = writeln!(out, "  no stored minute would be refused at landing");
        return OK;
    }
    if !months.is_empty() {
        let _ = writeln!(
            out,
            "  QUARANTINE NEEDED: {} month(s) hold {} minute(s) a landing refuses today: {}",
            months.len(),
            sum.refused(),
            months
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(" ")
        );
    }
    FAILED
}

/// The series directory names one segment directory lists, IN NAME ORDER so
/// two runs print the same table; a name that is not a symbol is not a series.
///
/// # Errors
///
/// An entry the listing could not read. This was `filter_map(Result::ok)`,
/// which dropped that entry without a word: the table then left a series
/// out, nothing marked the run degraded, and the verb could exit `OK` over a
/// store it had not walked (CLAUDE.md §4; cloud-attack-law-20261016). The
/// quarantine works from this list, so a missed series is a missed refusal.
pub(crate) fn series_names(
    dir: &Path,
    entries: impl IntoIterator<Item = std::io::Result<std::ffi::OsString>>,
) -> Result<std::collections::BTreeSet<String>, String> {
    let mut symbols = std::collections::BTreeSet::new();
    for entry in entries {
        let name = entry.map_err(|why| {
            format!(
                "an entry of {} could not be read: {why}; no partial table is printed",
                dir.display()
            )
        })?;
        if let Some(name) = name.to_str()
            && brutex_core::symbol::Symbol::new(name).is_ok()
        {
            symbols.insert(name.to_owned());
        }
    }
    Ok(symbols)
}

fn all(root: &Path, ask: &Ask, out: &mut String) -> u8 {
    let dir = root
        .join("bars")
        .join(ask.feed.as_str())
        .join("NSE")
        .join(ask.segment);
    let entries = match std::fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(why) => {
            let _ = writeln!(
                out,
                "SERIES IDENTITY REFUSED: cannot list {}: {why}",
                dir.display()
            );
            return FAILED;
        }
    };
    let symbols = match series_names(&dir, entries.map(|entry| entry.map(|e| e.file_name()))) {
        Ok(symbols) => symbols,
        Err(why) => {
            let _ = writeln!(out, "SERIES IDENTITY REFUSED: {why}");
            return FAILED;
        }
    };
    let _ = writeln!(
        out,
        "SERIES IDENTITY, EVERY SERIES (read-only) — {} NSE/{}: {} series under {}",
        ask.feed.as_str(),
        ask.segment,
        symbols.len(),
        root.display()
    );
    let _ = writeln!(
        out,
        "  instrument              minutes  before_first_day_bar  foreign_level  suspect_level  unjudged  refused-months  1day-from  1min-from"
    );
    let (mut sum, mut bad, mut no_minutes) = (Tally::default(), false, Vec::new());
    for symbol in &symbols {
        let Some(scan) = scan(root, ask, symbol) else {
            continue;
        };
        let from = |months: &[YearMonth]| {
            months
                .first()
                .map_or_else(|| "none".to_owned(), ToString::to_string)
        };
        if scan.minute_months.is_empty() {
            no_minutes.push(format!("{symbol} (1day {})", span(&scan.day_months)));
            continue;
        }
        let row = total(&scan);
        let months = refused_months(&scan).len();
        bad |= months > 0 || !scan.unread.is_empty();
        let _ = writeln!(
            out,
            "  {symbol:<20}  {:>9}  {:>20}  {:>13}  {:>13}  {:>8}  {:>14}  {:>9}  {:>9}{}",
            row.minutes(),
            row.before_first_day_bar,
            row.foreign_level,
            row.suspect_level,
            row.unchecked(),
            months,
            from(&scan.day_months),
            from(&scan.minute_months),
            if scan.unread.is_empty() {
                String::new()
            } else {
                format!("  NOT READ: {} month(s)", scan.unread.len())
            }
        );
        sum.absorb(row);
    }
    let _ = writeln!(
        out,
        "  TOTAL: minutes {}, before_first_day_bar {}, foreign_level {}, suspect_level {}, unjudged {}",
        sum.minutes(),
        sum.before_first_day_bar,
        sum.foreign_level,
        sum.suspect_level,
        sum.unchecked()
    );
    let _ = writeln!(
        out,
        "  DAY RUNG AND NO MINUTE RUNG ({}): {}",
        no_minutes.len(),
        if no_minutes.is_empty() {
            "none".to_owned()
        } else {
            no_minutes.join(", ")
        }
    );
    if bad { FAILED } else { OK }
}

#[cfg(test)]
#[path = "series_identity_tests.rs"]
mod tests;

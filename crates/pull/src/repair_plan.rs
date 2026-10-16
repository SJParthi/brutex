//! What a versioned repair of one series' months would do — read-only, and the
//! one answer the `/pull/repair.json` route and `cli zerodha-repair` both give.
//! D-3350, WP-13c; findings DB-N20, DB-N22.
//!
//! # What it answers
//!
//! For every month of the asked window: the generation every reader opens
//! (`0` is the original), the bars it holds, the sessions the calendar of
//! record owes inside the window ([`crate::calendar::exchange_kind_of`], the
//! authority `/calendar.json` reads), how many hold a bar, and each session
//! that does not, classed by what can fill it:
//!
//! - [`Missing::Front`] — before the month's first held bar;
//! - [`Missing::Interior`] — between its held bars;
//! - [`Missing::Tail`] — after its last held bar;
//! - [`Missing::Absent`] — the month holds nothing at all;
//! - [`Missing::BeforeFirst`] — before the first bar of the SERIES' first
//!   held month: a listing day, the start of the feed's history, or a hole an
//!   earlier refused answer left. The bars cannot tell which, so it is counted
//!   apart and never planned as a repair; a repair over that month lands
//!   whatever the feed serves.
//!
//! The first two are what the append-only store refuses for ever and only a
//! versioned repair fills (`POST /pull/repair`, `cli zerodha-repair`); tail and
//! absent an ordinary pull fills. A day the calendar cannot attest is named
//! apart, never counted as owed.
//!
//! # Cost
//!
//! Per month: one generation probe and one open; per day of the window: one
//! time-indexed lookup and one record read; per series: one listing of its
//! rung directory, to find its first held month. O(days asked + months held),
//! never the month's bars. Nothing is written, created or locked beyond a
//! reader's shared lock.

use std::path::Path;

use store::file::{BarFile, StoreError};
use store::path::{PathParts, StorePath, YearMonth};

use crate::calendar::{DayKind, exchange_kind_of};
use crate::session::{Day, IST_OFFSET_SECS, Window};

/// What can fill a session a month lacks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Missing {
    /// Before the month's first held bar: refused by the append-only store,
    /// filled only by a versioned repair.
    Front,
    /// Between the month's held bars: likewise.
    Interior,
    /// After the month's last held bar: an ordinary pull appends it.
    Tail,
    /// The month holds nothing: an ordinary pull creates it.
    Absent,
    /// Before the first bar of the series' first held month: a listing, the
    /// start of the feed's history, or a hole an earlier refused answer left.
    /// Counted apart and never planned as a repair.
    BeforeFirst,
}

impl Missing {
    /// The code a page and a terminal print.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Front => "front",
            Self::Interior => "interior",
            Self::Tail => "tail",
            Self::Absent => "absent",
            Self::BeforeFirst => "before_first",
        }
    }

    /// Whether only a versioned repair can fill it.
    #[must_use]
    pub const fn needs_repair(self) -> bool {
        matches!(self, Self::Front | Self::Interior)
    }
}

/// One month of the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Month {
    /// Which month.
    pub month: YearMonth,
    /// The generation served: `0` is the original month.
    pub generation: u32,
    /// Bars the served generation holds.
    pub bars: u64,
    /// Its first and last held stamps, UTC micros, when it holds any.
    pub span: Option<(i64, i64)>,
    /// Sessions of record inside the asked window.
    pub sessions: u32,
    /// Of those, the sessions holding at least one bar.
    pub held: u32,
    /// Each session holding none, and what can fill it.
    pub missing: Vec<(Day, Missing)>,
    /// Days inside the window the calendar of record cannot attest.
    pub unattested: Vec<Day>,
    /// Why the month could not be read, when it could not: nothing about it
    /// is then claimed.
    pub refused: Option<String>,
}

impl Month {
    /// Whether only a versioned repair can make this month whole.
    #[must_use]
    pub fn needs_repair(&self) -> bool {
        self.missing.iter().any(|(_, why)| why.needs_repair())
    }
}

/// The store's symbol id for `symbol`: the cross-check every bar header
/// carries, the low 32 bits of its FNV-1a (`ingest::identify`).
#[must_use]
pub fn symbol_id(symbol: &str) -> u32 {
    u32::try_from(brutex_core::universe::fnv1a(symbol) & u64::from(u32::MAX)).unwrap_or(u32::MAX)
}

/// The plan for `parts`' series over `window`, one entry per month the window
/// touches, oldest first. `parts.month` is ignored; every other part names the
/// series.
#[must_use]
pub fn plan(root: &Path, parts: PathParts<'_>, window: Window) -> Vec<Month> {
    let symbol_id = symbol_id(parts.symbol);
    let first_held = first_held_month(root, parts);
    let mut months = Vec::new();
    let mut at = window.from();
    while let Ok(month) = at.year_month() {
        let end = at.end_of_month().min(window.to());
        months.push(one(
            root,
            PathParts { month, ..parts },
            symbol_id,
            (at, end, first_held == Some(month)),
        ));
        if end >= window.to() {
            break;
        }
        let Ok(next) = Day::from_days(end.days_from_epoch().saturating_add(1)) else {
            break;
        };
        at = next;
    }
    months
}

/// The earliest month the series' rung directory holds a bar file for: one
/// listing, `None` when the directory is not there or names no month.
pub(crate) fn first_held_month(root: &Path, parts: PathParts<'_>) -> Option<YearMonth> {
    held_months(root, parts).first().copied()
}

/// Every month the series' rung directory holds a bar file for, ascending:
/// one listing, empty when the directory is not there or names no month.
/// `parts.month` only completes the path; every held month is answered.
#[must_use]
pub fn held_months(root: &Path, parts: PathParts<'_>) -> Vec<YearMonth> {
    let Some(dir) = StorePath::new(parts)
        .ok()
        .map(|path| path.to_path_buf(root))
        .and_then(|file| file.parent().map(Path::to_path_buf))
    else {
        return Vec::new();
    };
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut months: Vec<YearMonth> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name();
            let stem = name.to_str()?.strip_suffix(".bin")?;
            let (year, month) = stem.split_once('-')?;
            let digits = |text: &str, width: usize| {
                (text.len() == width && text.bytes().all(|b| b.is_ascii_digit()))
                    .then(|| text.parse::<u16>().ok())
                    .flatten()
            };
            YearMonth::new(digits(year, 4)?, u8::try_from(digits(month, 2)?).ok()?).ok()
        })
        .collect();
    months.sort_unstable();
    months.dedup();
    months
}

fn one(
    root: &Path,
    parts: PathParts<'_>,
    symbol_id: u32,
    (from, to, series_first): (Day, Day, bool),
) -> Month {
    let mut out = Month {
        month: parts.month,
        generation: 0,
        bars: 0,
        span: None,
        sessions: 0,
        held: 0,
        missing: Vec::new(),
        unattested: Vec::new(),
        refused: None,
    };
    let path = match StorePath::new(parts) {
        Ok(path) => path,
        Err(why) => {
            out.refused = Some(why.to_string());
            return out;
        }
    };
    let file = match store::generation::served(root, path)
        .and_then(|served| BarFile::open_existing(root, path, symbol_id).map(|file| (served, file)))
    {
        Ok((served, file)) => {
            out.generation = served.ordinal();
            out.bars = file.header().n_valid;
            if out.bars > 0 {
                out.span = Some((file.header().first_ts_micros, file.header().last_ts_micros));
            }
            Some(file)
        }
        Err(StoreError::Missing { .. }) => None,
        Err(why) => {
            out.refused = Some(why.to_string());
            return out;
        }
    };
    for number in from.days_from_epoch()..=to.days_from_epoch() {
        let Ok(day) = Day::from_days(number) else {
            continue;
        };
        match exchange_kind_of(i64::from(number)) {
            DayKind::Closed => continue,
            DayKind::Unmeasured => {
                out.unattested.push(day);
                continue;
            }
            DayKind::Open(_) | DayKind::OpenLengthUnmeasured => {}
        }
        out.sessions += 1;
        let start = (i64::from(number) * 86_400 - IST_OFFSET_SECS) * 1_000_000;
        let end = start + 86_400 * 1_000_000;
        let holds = file.as_ref().is_some_and(|file| {
            file.first_at_or_after(start)
                .ok()
                .filter(|&index| index < out.bars)
                .and_then(|index| file.read_record(index).ok())
                .is_some_and(|bar| bar.ts_micros < end)
        });
        if holds {
            out.held += 1;
            continue;
        }
        let why = match out.span {
            None => Missing::Absent,
            Some((first, _)) if end <= first && series_first => Missing::BeforeFirst,
            Some((first, _)) if end <= first => Missing::Front,
            Some((_, last)) if start > last => Missing::Tail,
            Some(_) => Missing::Interior,
        };
        out.missing.push((day, why));
    }
    out
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "test-only assertions"
)]
mod tests {
    use super::{Missing, plan};
    use crate::session::{Day, IST_OFFSET_SECS, Window};
    use brutex_core::vendor::Vendor;
    use store::format::{Bar, OI_NULL};
    use store::path::{FileKind, PathParts, Timeframe, YearMonth};

    fn parts() -> PathParts<'static> {
        PathParts {
            vendor: Vendor::Zerodha,
            exchange: "NSE",
            segment: "INDEX",
            symbol: "PLANIDX",
            contract: None,
            timeframe: Timeframe::MINUTE_1,
            month: YearMonth::new(2020, 12).unwrap(),
            file: FileKind::Bars,
        }
    }

    fn bar(day: Day, minute: i64) -> Bar {
        let secs = i64::from(day.days_from_epoch()) * 86_400 - IST_OFFSET_SECS + minute * 60;
        Bar {
            ts_micros: secs * 1_000_000,
            open: 500_000,
            high: 500_010,
            low: 499_990,
            close: 500_001,
            volume: 0,
            open_interest: OI_NULL,
        }
    }

    /// The DB-N20 month, on a fixture: November 2020 is held, and December
    /// 2020 holds the 16th on. The plan names December's eleven sessions
    /// before it as `front` — a repair's — and the window's later sessions it
    /// lacks as `tail`, an ordinary pull's.
    #[test]
    fn a_month_with_a_leading_hole_is_planned_as_a_repair() {
        let root = std::env::temp_dir().join(format!("brutex-PLAN-{}", std::process::id()));
        let _fresh = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let id = super::symbol_id("PLANIDX");
        let november = PathParts {
            month: YearMonth::new(2020, 11).unwrap(),
            ..parts()
        };
        let mut file = store::file::BarFile::open_or_create(
            &root,
            store::path::StorePath::new(november).unwrap(),
            id,
        )
        .unwrap();
        file.append(&[bar(Day::new(2020, 11, 27).unwrap(), 0)])
            .unwrap();
        drop(file);
        let path = store::path::StorePath::new(parts()).unwrap();
        let mut file = store::file::BarFile::open_or_create(&root, path, id).unwrap();
        let held: Vec<Bar> = [16u8, 17, 18, 21, 22, 23]
            .iter()
            .map(|&d| bar(Day::new(2020, 12, d).unwrap(), 0))
            .collect();
        file.append(&held).unwrap();
        drop(file);
        let window = Window::new(
            Day::new(2020, 12, 1).unwrap(),
            Day::new(2020, 12, 31).unwrap(),
        )
        .unwrap();
        let months = plan(&root, parts(), window);
        assert_eq!(months.len(), 1);
        let december = &months[0];
        assert_eq!((december.generation, december.bars), (0, 6));
        assert_eq!(december.sessions, 22, "December 2020's sessions of record");
        assert_eq!(december.held, 6);
        let front: Vec<String> = december
            .missing
            .iter()
            .filter(|(_, why)| *why == Missing::Front)
            .map(|(day, _)| day.to_string())
            .collect();
        assert_eq!(front.len(), 11);
        assert_eq!(front.first().map(String::as_str), Some("2020-12-01"));
        assert_eq!(front.last().map(String::as_str), Some("2020-12-15"));
        assert!(december.needs_repair());
        let tail = december
            .missing
            .iter()
            .filter(|(_, why)| *why == Missing::Tail)
            .count();
        assert_eq!(tail, 5, "24, 28, 29, 30 and 31 follow the last held bar");
        assert!(december.unattested.is_empty());

        // A month that holds nothing is an ordinary pull's, never a repair's.
        let january = Window::new(
            Day::new(2021, 1, 1).unwrap(),
            Day::new(2021, 1, 31).unwrap(),
        )
        .unwrap();
        let absent = &plan(&root, parts(), january)[0];
        assert!(!absent.missing.is_empty());
        assert!(
            absent
                .missing
                .iter()
                .all(|(_, why)| *why == Missing::Absent)
        );
        assert!(!absent.needs_repair());

        // THE SERIES' FIRST HELD MONTH: its sessions before November's first
        // bar may be a listing or the start of the feed's history, which the
        // bars cannot tell from a hole. Counted apart, never planned.
        let first = &plan(
            &root,
            parts(),
            Window::new(
                Day::new(2020, 11, 1).unwrap(),
                Day::new(2020, 11, 30).unwrap(),
            )
            .unwrap(),
        )[0];
        assert_eq!(first.held, 1);
        assert_eq!(
            first.missing.len(),
            19,
            "every session of November before the 27th"
        );
        assert!(
            first
                .missing
                .iter()
                .all(|(_, why)| *why == Missing::BeforeFirst)
        );
        assert!(!first.needs_repair());
        assert_eq!(Missing::BeforeFirst.code(), "before_first");
        let _cleanup = std::fs::remove_dir_all(&root);
    }

    /// One entry per month the window touches, each clipped to the window.
    #[test]
    fn the_plan_has_one_entry_per_month_clipped_to_the_window() {
        let root = std::env::temp_dir().join(format!("brutex-PLAN-SPAN-{}", std::process::id()));
        let _fresh = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let window = Window::new(
            Day::new(2020, 11, 25).unwrap(),
            Day::new(2021, 1, 5).unwrap(),
        )
        .unwrap();
        let months = plan(&root, parts(), window);
        let names: Vec<String> = months.iter().map(|m| m.month.to_string()).collect();
        assert_eq!(names, ["2020-11", "2020-12", "2021-01"]);
        assert_eq!(
            months[0].sessions, 3,
            "25, 26 and 27 November; the 30th was an exchange holiday"
        );
        assert_eq!(months[1].sessions, 22, "December 2020, less the 25th");
        assert_eq!(months[2].sessions, 3, "1, 4 and 5 January 2021");
        assert!(months.iter().all(|m| m.bars == 0 && m.refused.is_none()));
        let _cleanup = std::fs::remove_dir_all(&root);
    }
}

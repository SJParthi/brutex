//! `cli zerodha-repair` — the Zerodha month holes, planned and repaired.
//! D-3350, WP-13c; findings DB-N20, DB-N21, DB-N22.
//!
//! # Usage
//!
//! ```text
//! cli zerodha-repair INSTRUMENT FROM TO [--segment INDEX|CASH] [--rung RUNG]
//! cli zerodha-repair ALL FROM TO [--segment INDEX|CASH] [--rung RUNG]
//! cli zerodha-repair INSTRUMENT FROM TO --from-store ROOT --reason TEXT [--segment ..] [--rung ..]
//! ```
//!
//! `FROM` and `TO` are `YYYY-MM-DD`, inclusive. `--segment` is `INDEX` unless
//! named; `--rung` is `1min` unless named.
//!
//! # What it does
//!
//! **Without `--from-store` it writes nothing.** For every month of the window
//! it prints the generation every reader opens (`0` is the original month), the
//! sessions the calendar of record owes, how many hold a bar, and each that
//! does not, with what fills it: `front` and `interior` only a versioned repair
//! (the append-only store refuses a session before a month's last held bar for
//! ever), `tail` and `absent` an ordinary pull; at the minute rung, each
//! session holding a bar but fewer of its owed minutes than the calendar owes,
//! classed the same way (R3-18). Then the `POST /pull/repair`
//! body that fills each month a repair must — one whole month per request, so
//! the offered bars include held ones to witness the price basis. `ALL` does
//! the same for every Zerodha series of the segment, one line each: the count
//! of missing sessions per instrument WP-13c was asked to measure, and the
//! short sessions and their missing minutes (R3-18, CMP-19).
//!
//! **With `--from-store ROOT --reason TEXT` it repairs**, from the bars another
//! store holds for the same feed and series — a scratch store a fresh pull
//! landed into. Each month of the window goes through the repair door
//! `POST /pull/repair` lands through (`pull::ingest::repair_window`): appended
//! where the append door takes it, published as a NEW generation where the
//! month holds bars around the hole, every held bar kept and never changed,
//! every derived rung refolded the same way, the census counted, and each
//! repair journaled with the reason in `audit/zerodha.repairs`. A repair is
//! idempotent: a second run finds nothing new and writes nothing.
//!
//! No Kite request is made here. A pull needs a Kite access token valid on the
//! day; this repository never mints one (`CLAUDE.md` §8). `POST /pull/repair`
//! fetches and repairs in one step; this verb plans, measures, and applies a
//! fetch that is already on disk.
//!
//! # Exit
//!
//! Planning: [`OK`] when no month needs a repair, [`FAILED`] when one does or a
//! month will not open. Repairing: [`OK`] when every month landed with no
//! refusal, [`FAILED`] otherwise. [`MISUSED`] for arguments that do not name a
//! series and a window, before anything is opened.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use brutex_core::vendor::Vendor;
use pull::ingest::repair_plan::{self, Missing};
use pull::session::{Day, Window};
use store::format::{Bar, OI_NULL};
use store::path::{FileKind, PathParts, StorePath, Timeframe};

use crate::{FAILED, MISUSED, OK};

/// The verb's shape, printed with every misuse.
pub(crate) const USAGE: &str = "cli zerodha-repair INSTRUMENT|ALL FROM TO [--segment INDEX|CASH] \
     [--rung RUNG] [--from-store ROOT --reason TEXT]  (FROM and TO are YYYY-MM-DD, inclusive)";

/// The widest window one run answers, in days: ten years, as a pull form.
const MAX_DAYS: u32 = 3_653;

/// What the arguments name.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Ask {
    /// One store symbol, or `None` for every series of the segment.
    instrument: Option<String>,
    segment: &'static str,
    rung: Timeframe,
    window: Window,
    /// `(source root, reason)` when the run repairs.
    apply: Option<(PathBuf, String)>,
}

fn day(label: &str, word: &str) -> Result<Day, String> {
    let digits = |part: &str, width: usize| {
        (part.len() == width && part.bytes().all(|b| b.is_ascii_digit()))
            .then(|| part.parse::<u16>().ok())
            .flatten()
    };
    let mut parts = word.split('-');
    let parsed = match (parts.next(), parts.next(), parts.next(), parts.next()) {
        (Some(y), Some(m), Some(d), None) => digits(y, 4).zip(digits(m, 2)).zip(digits(d, 2)),
        _ => None,
    };
    let Some(((year, month), date)) = parsed else {
        return Err(format!(
            "{label} `{}` is not a YYYY-MM-DD day",
            crate::stored::clipped(word)
        ));
    };
    Day::new(
        year,
        u8::try_from(month).unwrap_or(u8::MAX),
        u8::try_from(date).unwrap_or(u8::MAX),
    )
    .map_err(|why| format!("{label} `{}`: {why}", crate::stored::clipped(word)))
}

/// The arguments, refused by name before anything is opened.
fn ask(arguments: &[&str]) -> Result<Ask, String> {
    let [instrument, from, to, flags @ ..] = arguments else {
        return Err(format!(
            "zerodha-repair takes INSTRUMENT FROM TO and flags; it was given {} argument(s)",
            arguments.len()
        ));
    };
    let mut segment = "INDEX";
    let mut rung = Timeframe::MINUTE_1;
    let mut source = None;
    let mut reason = None;
    let mut rest = flags.iter();
    while let Some(flag) = rest.next() {
        let value = rest.next().ok_or_else(|| format!("{flag} takes a value"))?;
        match *flag {
            "--segment" => {
                segment = match value.to_ascii_uppercase().as_str() {
                    "INDEX" => "INDEX",
                    "CASH" => "CASH",
                    _ => {
                        return Err(format!(
                            "--segment `{}`: INDEX or CASH",
                            crate::stored::clipped(value)
                        ));
                    }
                };
            }
            "--rung" => rung = crate::stored::rung(value)?,
            "--from-store" => source = Some(PathBuf::from(value)),
            "--reason" => reason = Some((*value).to_owned()),
            other => {
                return Err(format!(
                    "`{}` is not a flag of zerodha-repair",
                    crate::stored::clipped(other)
                ));
            }
        }
    }
    let window = Window::new(day("FROM", from)?, day("TO", to)?).map_err(|why| why.to_string())?;
    if window.days() > MAX_DAYS {
        return Err(format!(
            "{} days asked; one run answers at most {MAX_DAYS}",
            window.days()
        ));
    }
    let instrument = if instrument.eq_ignore_ascii_case("ALL") {
        None
    } else {
        let symbol = instrument.to_ascii_uppercase();
        brutex_core::symbol::Symbol::new(&symbol)
            .map_err(|why| format!("`{}`: {why}", crate::stored::clipped(instrument)))?;
        Some(symbol)
    };
    let apply = match (source, reason) {
        (None, None) => None,
        (Some(source), Some(reason)) => {
            let reason = reason.trim().to_owned();
            if reason.is_empty() || reason.len() > 200 || reason.chars().any(char::is_control) {
                return Err("--reason is one line of at most 200 bytes".to_owned());
            }
            if instrument.is_none() {
                return Err("a repair names one INSTRUMENT, never ALL".to_owned());
            }
            // A DERIVED RUNG IS FOLDED FROM THE MINUTES, NEVER LANDED (D-3321):
            // a repair at 1min refolds every derived rung of its months.
            if rung.secs() > Timeframe::MINUTE_1.secs() && rung.secs() < Timeframe::DAY_1.secs() {
                return Err(format!(
                    "--rung {} is derived from the minutes: repair at --rung 1min, which refolds it",
                    rung.as_str()
                ));
            }
            Some((source, reason))
        }
        _ => return Err(
            "--from-store and --reason go together: a repair says where its bars come from and why"
                .to_owned(),
        ),
    };
    Ok(Ask {
        instrument,
        segment,
        rung,
        window,
        apply,
    })
}

fn parts<'a>(ask: &Ask, symbol: &'a str) -> Result<PathParts<'a>, String> {
    let month = ask
        .window
        .from()
        .year_month()
        .map_err(|why| why.to_string())?;
    Ok(PathParts {
        vendor: Vendor::Zerodha,
        exchange: "NSE",
        segment: ask.segment,
        symbol,
        contract: None,
        timeframe: ask.rung,
        month,
        file: FileKind::Bars,
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
            let _ = writeln!(
                out,
                "ZERODHA REPAIR REFUSED: {why}. Nothing was read or written."
            );
            return FAILED;
        }
    };
    match (&ask.instrument, &ask.apply) {
        (Some(symbol), Some((source, reason))) => apply(&root, &ask, symbol, (source, reason), out),
        (Some(symbol), None) => plan_one(&root, &ask, symbol, out),
        (None, _) => plan_all(&root, &ask, out),
    }
}

/// `cli zerodha-repair`: see the module header.
pub(crate) fn command(arguments: &[&str], out: &mut String) -> u8 {
    // A REPAIR (`--from-store`) BY AN UNSTAMPED BUILD IS REFUSED (DB-B71,
    // WP-81); a plan writes nothing and is not.
    let store = crate::store_write::root_for("zerodha-repair", arguments);
    command_with(arguments, &store, out)
}

fn plan_one(root: &Path, ask: &Ask, symbol: &str, out: &mut String) -> u8 {
    let parts = match parts(ask, symbol) {
        Ok(parts) => parts,
        Err(why) => {
            let _ = writeln!(out, "refused: {why}");
            return MISUSED;
        }
    };
    let months = repair_plan::plan(root, parts, ask.window);
    let _ = writeln!(
        out,
        "ZERODHA REPAIR PLAN (read-only; nothing is written) — NSE-{}-{symbol} {}, {}..{} under {}",
        ask.segment,
        ask.rung.as_str(),
        ask.window.from(),
        ask.window.to(),
        root.display()
    );
    let mut needs = Vec::new();
    let mut refused = false;
    for month in &months {
        refused |= month.refused.is_some();
        let _ = writeln!(
            out,
            "  {}  generation {}  {:>6} bars  {:>2} of {:>2} sessions held{}",
            month.month,
            month.generation,
            month.bars,
            month.held,
            month.sessions,
            month
                .refused
                .as_deref()
                .map_or_else(String::new, |why| format!("  REFUSED: {why}")),
        );
        for (day, why) in &month.missing {
            let _ = writeln!(out, "      {day}  {:<12}  {}", why.code(), filled_by(*why));
        }
        short_lines(out, month);
        for day in &month.unattested {
            let _ = writeln!(
                out,
                "      {day}  unattested: the calendar of record cannot say whether it traded"
            );
        }
        if month.needs_repair() {
            needs.push(month.month);
        }
    }
    let _ = writeln!(out, "  {} month(s) need a versioned repair", needs.len());
    // A DERIVED RUNG IS NEVER PULLED (D-3321, D-3350): its months are refolded
    // from the minutes, so the request is the minute rung's, and where the
    // minutes are already held a refold from this store needs no vendor.
    let derived =
        ask.rung.secs() > Timeframe::MINUTE_1.secs() && ask.rung.secs() < Timeframe::DAY_1.secs();
    let pulled = if derived {
        Timeframe::MINUTE_1
    } else {
        ask.rung
    };
    for month in &needs {
        let first =
            Day::new(month.year(), month.month(), 1).map(|first| first.max(ask.window.from()));
        if let Ok(first) = first {
            let last = first.end_of_month().min(ask.window.to());
            let _ = writeln!(
                out,
                "  POST /pull/repair  target={}&vendor=zerodha&granularity={}&cash_identity=zerodha_nse_checked&member={symbol}&from={first}&to={last}&repair_reason=<why>",
                if ask.segment == "CASH" {
                    "equities"
                } else {
                    "indices"
                },
                pulled.as_str(),
            );
            if derived {
                let _ = writeln!(
                    out,
                    "    or, where the minutes are held: cli zerodha-repair {symbol} {first} {last} --segment {} --from-store {} --reason <why>",
                    ask.segment,
                    root.display()
                );
            }
        }
    }
    if !needs.is_empty() {
        let _ = writeln!(
            out,
            "  A Kite access token valid on the day is needed for the pull; this repository never mints one (CLAUDE.md section 8)."
        );
    }
    if needs.is_empty() && !refused {
        OK
    } else {
        FAILED
    }
}

/// A SESSION HOLDING A BAR IS NOT THEREFORE WHOLE (R3-18, CMP-19): each of
/// `month`'s sessions short of its owed minutes, one line with what fills it.
fn short_lines(out: &mut String, month: &repair_plan::Month) {
    for short in &month.short {
        let _ = writeln!(
            out,
            "      {}  {:<12}  short: {} of {} owed minutes held, {} missing; {}",
            short.day,
            short.fill.code(),
            short.held,
            short.owed,
            short.missing(),
            filled_by(short.fill)
        );
    }
}

/// What can fill a missing session, in the terminal's words.
const fn filled_by(why: Missing) -> &'static str {
    match why {
        Missing::Front | Missing::Interior => "only a versioned repair fills it",
        Missing::Tail | Missing::Absent => "an ordinary pull fills it",
        Missing::BeforeFirst => {
            "before the series' first held bar: a listing, the feed's history start, or a hole; not planned"
        }
    }
}

/// One series' counts over the window, inside its own held span.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Counts {
    sessions: u64,
    held: u64,
    front: u64,
    interior: u64,
    tail: u64,
    absent: u64,
    before_first: u64,
    /// Sessions holding a bar but short of their owed minutes (R3-18).
    short: u64,
    /// Their missing minutes.
    short_minutes: u64,
}

impl Counts {
    fn add(&mut self, other: Self) {
        self.sessions += other.sessions;
        self.held += other.held;
        self.front += other.front;
        self.interior += other.interior;
        self.tail += other.tail;
        self.absent += other.absent;
        self.before_first += other.before_first;
        self.short += other.short;
        self.short_minutes += other.short_minutes;
    }

    fn line(&self, name: &str, tail: &str) -> String {
        format!(
            "  {name:<20}  {:>8}  {:>6}  {:>5}  {:>8}  {:>4}  {:>6}  {:>12}  {:>5}  {:>9}  {tail}",
            self.sessions,
            self.held,
            self.front,
            self.interior,
            self.tail,
            self.absent,
            self.before_first,
            self.short,
            self.short_minutes
        )
    }
}

/// The counts of one series' plan. A series is owed only its own life: a
/// month before its first held month or after its last is not its hole.
fn counts(months: &[repair_plan::Month]) -> Counts {
    let (first, last) = held_span(months);
    let mut row = Counts::default();
    for month in months {
        if first.is_none_or(|first| month.month < first)
            || last.is_none_or(|last| month.month > last)
        {
            continue;
        }
        row.sessions += u64::from(month.sessions);
        row.held += u64::from(month.held);
        for (_, why) in &month.missing {
            match why {
                Missing::Front => row.front += 1,
                Missing::Interior => row.interior += 1,
                Missing::Tail => row.tail += 1,
                Missing::Absent => row.absent += 1,
                Missing::BeforeFirst => row.before_first += 1,
            }
        }
        row.short += u64::try_from(month.short.len()).unwrap_or(u64::MAX);
        row.short_minutes += month
            .short
            .iter()
            .map(|short| u64::from(short.missing()))
            .sum::<u64>();
    }
    row
}

fn plan_all(root: &Path, ask: &Ask, out: &mut String) -> u8 {
    let dir = root
        .join("bars")
        .join("zerodha")
        .join("NSE")
        .join(ask.segment);
    let entries = match std::fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(why) => {
            let _ = writeln!(
                out,
                "ZERODHA REPAIR PLAN REFUSED: cannot list {}: {why}",
                dir.display()
            );
            return FAILED;
        }
    };
    // IN NAME ORDER, so two runs print the same table.
    let mut symbols = std::collections::BTreeSet::new();
    for entry in entries.filter_map(Result::ok) {
        if let Some(name) = entry.file_name().to_str()
            && brutex_core::symbol::Symbol::new(name).is_ok()
        {
            symbols.insert(name.to_owned());
        }
    }
    let _ = writeln!(
        out,
        "ZERODHA REPAIR PLAN, EVERY SERIES (read-only) — NSE/{} {}, {}..{}: {} series under {}",
        ask.segment,
        ask.rung.as_str(),
        ask.window.from(),
        ask.window.to(),
        symbols.len(),
        root.display()
    );
    let _ = writeln!(
        out,
        "  instrument            sessions    held  front  interior  tail  absent  before_first  short  short-min  repair-months"
    );
    let mut total = Counts::default();
    let mut any = false;
    for symbol in &symbols {
        let Ok(parts) = parts(ask, symbol) else {
            continue;
        };
        let months = repair_plan::plan(root, parts, ask.window);
        if !months.iter().any(|m| m.bars > 0) {
            let _ = writeln!(
                out,
                "  {symbol:<20}  holds nothing at this rung in the window"
            );
            continue;
        }
        let row = counts(&months);
        let repair_months = months.iter().filter(|m| m.needs_repair()).count();
        any |= repair_months > 0;
        total.add(row);
        let _ = writeln!(out, "{}", row.line(symbol, &format!("{repair_months:>13}")));
    }
    let _ = writeln!(out, "{}", total.line("TOTAL", ""));
    if any { FAILED } else { OK }
}

/// The first and last month of `months` that hold a bar.
fn held_span(
    months: &[repair_plan::Month],
) -> (
    Option<store::path::YearMonth>,
    Option<store::path::YearMonth>,
) {
    let mut held = months.iter().filter(|m| m.bars > 0).map(|m| m.month);
    let first = held.next();
    (first, held.next_back().or(first))
}

/// The bars `source` holds for the series in `window`, as the raw rows a
/// broker answer carries: epoch seconds, prices already in paisa.
fn source_rows(
    source: &Path,
    parts: PathParts<'_>,
    window: Window,
) -> Result<Vec<pull::fetch::RawRow>, String> {
    let path = StorePath::new(parts).map_err(|why| why.to_string())?;
    let id = repair_plan::symbol_id(parts.symbol);
    let file = match store::file::BarFile::open_existing(source, path, id) {
        Ok(file) => file,
        Err(store::file::StoreError::Missing { .. }) => return Ok(Vec::new()),
        Err(why) => return Err(why.to_string()),
    };
    let (lo, hi) = window_micros(window);
    let mut rows = Vec::new();
    for index in 0..file.header().n_valid {
        let bar: Bar = file.read_record(index).map_err(|why| why.to_string())?;
        if bar.ts_micros < lo || bar.ts_micros >= hi {
            continue;
        }
        rows.push(pull::fetch::RawRow {
            timestamp: bar.ts_micros.div_euclid(1_000_000),
            open: bar.open,
            high: bar.high,
            low: bar.low,
            close: bar.close,
            volume: bar.volume,
            open_interest: (bar.open_interest != OI_NULL).then_some(bar.open_interest),
        });
    }
    Ok(rows)
}

/// `[from 00:00 IST, the day after to 00:00 IST)` in UTC micros.
fn window_micros(window: Window) -> (i64, i64) {
    let at = |day: u32| (i64::from(day) * 86_400 - pull::session::IST_OFFSET_SECS) * 1_000_000;
    (
        at(window.from().days_from_epoch()),
        at(window.to().days_from_epoch().saturating_add(1)),
    )
}

fn apply(
    root: &Path,
    ask: &Ask,
    symbol: &str,
    (source, reason): (&Path, &str),
    out: &mut String,
) -> u8 {
    let _ = writeln!(
        out,
        "ZERODHA REPAIR — NSE-{}-{symbol} {}, {}..{} under {}, from the bars {} holds; reason: {reason}",
        ask.segment,
        ask.rung.as_str(),
        ask.window.from(),
        ask.window.to(),
        root.display(),
        source.display()
    );
    let (Some(granularity), Ok(base)) = (granularity_of(ask.rung), parts(ask, symbol)) else {
        let _ = writeln!(
            out,
            "  refused: rung {} or the series is not one a pull lands",
            ask.rung.as_str()
        );
        return MISUSED;
    };
    let mut clean = true;
    for month in repair_plan::plan(root, base, ask.window) {
        let Ok(first) = Day::new(month.month.year(), month.month.month(), 1) else {
            continue;
        };
        let Ok(window) = Window::new(
            first.max(ask.window.from()),
            first.end_of_month().min(ask.window.to()),
        ) else {
            continue;
        };
        let parts = PathParts {
            month: month.month,
            ..base
        };
        clean &= repair_one(
            root,
            (source, parts, window),
            (ask, granularity, reason),
            out,
        );
    }
    let _ = writeln!(
        out,
        "  every repair is journaled in {}",
        root.join("audit").join("zerodha.repairs").display()
    );
    if clean { OK } else { FAILED }
}

/// One month of a repair from `source`'s bars, through the repair door, with
/// its receipt line. Whether it landed with no refusal.
fn repair_one(
    root: &Path,
    (source, parts, window): (&Path, PathParts<'_>, Window),
    (ask, granularity, reason): (&Ask, pull::vendor::Granularity, &str),
    out: &mut String,
) -> bool {
    let rows = match source_rows(source, parts, window) {
        Ok(rows) => rows,
        Err(why) => {
            let _ = writeln!(
                out,
                "  {}  REFUSED  the source month will not read: {why}; nothing written",
                parts.month
            );
            return false;
        }
    };
    if rows.is_empty() {
        let _ = writeln!(
            out,
            "  {}  skipped  the source store holds no bar in this month's window",
            parts.month
        );
        return true;
    }
    let request = pull::fetch::BarRequest {
        instrument_id: String::new(),
        listing: if ask.segment == "CASH" {
            pull::vendor::Listing::Equity
        } else {
            pull::vendor::Listing::Index
        },
        window,
        granularity,
    };
    let plan = pull::ingest::Plan {
        calendar: pull::calendar::Runtime::default(),
        cash_schedule: None,
        columns: pull::csv::Columns::Gdfl,
        request: &request,
        encoding: pull::vendor::TimestampEncoding::EpochSecondsUtc,
        scale: pull::vendor::PriceScale::Paisa,
        vendor: Vendor::Zerodha,
        exchange: "NSE",
        segment: ask.segment,
        contract: None,
    };
    let origin = format!("store {}", source.display());
    let done = pull::ingest::repair_window(
        &pull::fetch::RawWindow { rows },
        parts.symbol,
        &origin,
        root,
        plan,
        reason,
    );
    let _ = writeln!(
        out,
        "  {}  offered {:>6}  written {:>6}  census rows {:>2}  failures {}",
        parts.month,
        done.bars_stored,
        done.bars_committed,
        done.counted,
        done.failures.len()
    );
    for failure in &done.failures {
        let _ = writeln!(out, "      {}", failure.why);
    }
    done.failures.is_empty()
}

/// The pull rung that files under `rung`, when one does.
fn granularity_of(rung: Timeframe) -> Option<pull::vendor::Granularity> {
    pull::vendor::Granularity::ALL
        .into_iter()
        .rfind(|granularity| granularity.store_timeframe() == Some(rung))
}

#[cfg(test)]
#[path = "zerodha_repair_tests.rs"]
mod tests;

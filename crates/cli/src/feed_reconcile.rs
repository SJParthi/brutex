//! `cli feed-reconcile` — the three stored views of one cash share, side by
//! side, one row per stock-day, and the ONE authority for the per-stock-day
//! price factor and the Zerodha-to-GDFL clock alignment (DATA-1; D-5170 to
//! D-5175).
//!
//! # Usage
//!
//! ```text
//! cli feed-reconcile YYYY-MM --out DIR [--symbols A,B] [--workers N]
//! cli feed-reconcile factor SYMBOL YYYY-MM-DD --dir DIR [--repeat N]
//! ```
//!
//! # What it reads, and how
//!
//! The store, directly, through [`BarFile::open_existing`] — the door every
//! reader uses, so each month is read in its served generation and every
//! sealed block is checked. No HTTP, no server. Per share (`NSE CASH`), the
//! month's GDFL `1s` bars, Zerodha `1min` bars and Zerodha `1day` bars. Every
//! share is named by its stored directory, so a name holding `&` (`M&M`,
//! `GVT&D`, `ARE&M`, `M&MFIN`) is read by its own key.
//!
//! # The rules (operator 16 Oct 2026 17:15 IST, "nothing typed")
//!
//! - **Grid.** Each feed's grid `s` for a series-month is the gcd of the
//!   differences between its distinct paisa prints so far, updated per bar.
//!   Fewer than two distinct prints: no exchange tick is chartered, so the
//!   minute is counted `grid_unknown_minutes` and admits nothing.
//! - **Allowance.** A matched minute (Zerodha close `z`, GDFL close `g`)
//!   admits every factor in `[(2z − s_z − 2t)/(2g + s_g), (2z + s_z)/(2g − s_g)]`,
//!   `t` = 0 (no `vendor_truncated` proof is held for a minute series).
//!   Every comparison is an i128 cross-multiplication, never a float.
//! - **Factor.** The exact paisa pair `(z, g)` of the matched minute whose
//!   ratio is the LOWER median of the day's matched minutes, plus the min and
//!   max pairs. Empty, never 1.0, when no minute matched.
//! - `factor_not_constant`: the day's running intersection of intervals
//!   empties; the minute that emptied it and both pairs are printed.
//! - `range_not_constant` (D-5176, a labelled companion, not a replacement):
//!   the same running intersection over each matched minute's RANGES,
//!   `[(2·z_low − s_z)/(2·g_high + s_g), (2·z_high + s_z)/(2·g_low − s_g)]`.
//!   The two feeds' minute closes are often different trades of one minute,
//!   which empties the close intersection on days whose basis never moved;
//!   two ranges on one basis share their trades, so different trades of one
//!   minute do not empty this one and a basis change or a fault still does.
//!   The reader hands a `factor_not_constant` day's pairs and its range
//!   verdict to the consumer inside the refusal.
//! - `adjusted`: 1 lies outside the interval of the median pair.
//! - `ex_date`: two consecutive matched days have disjoint intervals. A
//!   day's interval is its close intersection, or where that emptied its
//!   range intersection (D-5177): on the real store the closes empty on
//!   almost every day, so an ex-date judged on them alone never fired. No
//!   corporate-action source is merged (R4-26), so every one is `unsourced`.
//! - **Alignment.** A GDFL minute close at lag `L` is the close of the last
//!   GDFL second in `[HH:MM:00 + L, HH:MM+1:00 + L)`. Every lag −59..=+59 s is
//!   measured per stock-day: at each lag the minutes whose Zerodha close
//!   EQUALS the factor-converted GDFL close (that lag's own lower-median ratio
//!   lies in the minute's allowance interval) are counted. Lag 0's share, the
//!   best lag (ties: smallest |L|, then the negative) and its share are
//!   stored. The lag APPLIED on day D comes only from earlier days: the
//!   share's most frequent earlier best lag (a 119-bin count), else every
//!   share's earlier days (`pooled`), else the day is `not_aligned`.
//! - **Volume.** `short = (day bar − minute sum) − explained`, `explained` =
//!   the GDFL volume in seconds outside the continuous-phase minutes Zerodha
//!   printed (pre-open, any non-continuous phase, omitted minutes), always
//!   reported in shares and as a pair over the day bar, on both bases on an
//!   adjusted day. The flag needs PREC-3's ENV-1 envelope, which is not
//!   merged: every computed day says `unjudged:no_env1_envelope`.
//! - `zerodha_minute_sum_exceeds_day_bar`: the minute sum exceeds the day bar
//!   by more than 0 shares (unadjusted) or `ceil((printed minutes + 1) / 2)`
//!   (adjusted); runs of flagged days are printed as spans.
//! - `zerodha_session_cut`: GDFL trades in a continuous-phase minute
//!   (`pull::calendar::phase_at`) after the day's last Zerodha minute.
//!
//! # What it writes
//!
//! One CSV per month under `--out`: `feed-reconcile-YYYY-MM.v<N>.csv`, `N`
//! one past the newest already there. Built whole, written to a temporary,
//! `fsync`ed, linked to its name with `hard_link` (which refuses rather than
//! replaces), the temporary removed, the directory synced. The bytes do not
//! depend on the worker count, the time or the version number. Carry lines
//! at the end hand the next month each share's lag history, last interval,
//! open span and session-cut onset.
//!
//! # Cost
//!
//! One constant-time update per bar into a slot addressed by the bar's own
//! offset; per stock-day a fixed walk of its 1,440 minutes, of at most 60
//! seconds per minute window, and of 119 lags per minute. Parallel over
//! shares; one telemetry event per share-month and one per run, emitted from
//! the coordinating thread.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use brutex_core::instrument::{Exchange, Segment};
use brutex_core::vendor::Vendor;
use pull::calendar::{CasEligibility, DayKind, Phase};
use pull::session::Day;
use pull::vendor::Venue;
use store::file::{BarFile, StoreError};
use store::format::Bar;
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

use crate::{FAILED, MISUSED, OK};

/// The verb's shape, printed with every misuse.
pub(crate) const USAGE: &str = "cli feed-reconcile YYYY-MM --out DIR [--symbols A,B] [--workers N]\n       \
cli feed-reconcile factor SYMBOL YYYY-MM-DD --dir DIR [--repeat N]\n       \
cli feed-reconcile months";

/// The first line of every file this verb writes. Version 2 is the exact-pair
/// factor, the allowance intervals and the lag alignment (D-5171); version 1
/// (the float ratio) is never read as version 2.
pub const FORMAT: &str = "# brutex feed-reconcile 2";

/// The column header, in output order.
pub(crate) const COLUMNS: &str = "symbol,day,day_kind,status,applied_lag,lag_source,\
gdfl_day_volume,zerodha_day_volume,zerodha_minute_volume,gdfl_vs_day_pct,zmin_vs_day_pct,\
zmin_vs_gdfl_pct,gdfl_seconds,gdfl_minutes,zerodha_minutes,matched_minutes,median_z,median_g,\
min_z,min_g,max_z,max_g,interval_lo,interval_hi,emptied_at,emptied_pair,opposing_pair,range_lo,\
range_hi,range_emptied_at,grid_z,grid_g,high_match,low_match,lag0_share,best_lag,\
best_lag_share,explained_volume,short_volume,\
short_of_day,explained_volume_zbasis,short_volume_zbasis,short_judgement,minute_sum_allowance,\
gdfl_unverified_phase_volume,auction_era_day_minus_minutes,session_cut_minute,ex_date,flags,source";

/// How many columns a row has.
const COLUMN_COUNT: usize = 50;

/// The reach of the lag search, in seconds either side (operator 17:15 IST,
/// item 7: "every lag from -59 to +59 s").
pub const LAG_REACH: i8 = 59;
/// The lag bins: −59..=+59.
const LAG_BINS: usize = 119;

const MINUTE_SECS: i64 = 60;
const SECOND_MICROS: i64 = 1_000_000;
const DAY_MICROS: i64 = 86_400_000_000;
const SECONDS_PER_DAY: usize = 86_400;
const MINUTES_PER_DAY: usize = 1_440;
const MAX_DAYS: usize = 31;

/// `t` of the allowance: 1 paisa only where `vendor_truncated` is PROVEN for
/// the series. `pull::daycheck` proves it for a Zerodha DAY bar against its
/// own minutes, never for a minute series, so it is 0 here (D-5171).
const TRUNCATION_PAISA: i128 = 0;

/// The flags, in the one order they are printed.
pub(crate) const FLAG_NAMES: [&str; 17] = [
    "factor_not_constant",
    "adjusted",
    "ex_date",
    "zerodha_minute_sum_exceeds_day_bar",
    "zerodha_session_cut",
    "zerodha_no_day_bar",
    "zerodha_no_minutes",
    "zero_day_volume",
    "zerodha_unaligned_minutes",
    "zerodha_duplicate_minutes",
    "zerodha_duplicate_day_bar",
    "nonpositive_price",
    "bars_outside_month",
    "grid_unknown_minutes",
    "interval_unbounded_minutes",
    "gdfl_unaligned_seconds",
    "range_not_constant",
];

const FACTOR_NOT_CONSTANT: u32 = 1;
const ADJUSTED: u32 = 1 << 1;
const EX_DATE: u32 = 1 << 2;
const SUM_EXCEEDS_DAY: u32 = 1 << 3;
const SESSION_CUT: u32 = 1 << 4;
const NO_DAY_BAR: u32 = 1 << 5;
const NO_MINUTES: u32 = 1 << 6;
const ZERO_DAY_VOLUME: u32 = 1 << 7;
const UNALIGNED: u32 = 1 << 8;
const DUPLICATE_MINUTES: u32 = 1 << 9;
const DUPLICATE_DAY_BAR: u32 = 1 << 10;
const NONPOSITIVE: u32 = 1 << 11;
const OUTSIDE_MONTH: u32 = 1 << 12;
const GRID_UNKNOWN: u32 = 1 << 13;
const UNBOUNDED: u32 = 1 << 14;
const GDFL_UNALIGNED: u32 = 1 << 15;
const RANGE_NOT_CONSTANT: u32 = 1 << 16;

// ---------------------------------------------------------------------------
// Arguments
// ---------------------------------------------------------------------------

/// What the arguments of a month run name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Ask {
    pub(crate) month: YearMonth,
    pub(crate) symbols: Option<Vec<String>>,
    pub(crate) out: PathBuf,
    /// `None`: generated (start at 1, double while throughput rises).
    pub(crate) workers: Option<usize>,
}

/// `YYYY-MM`, strictly.
pub(crate) fn parse_month(text: &str) -> Result<YearMonth, String> {
    let bytes = text.as_bytes();
    let shaped = bytes.len() == 7
        && bytes.iter().enumerate().all(|(at, byte)| {
            if at == 4 {
                *byte == b'-'
            } else {
                byte.is_ascii_digit()
            }
        });
    let year = text.get(..4).and_then(|y| y.parse::<u16>().ok());
    let month = text.get(5..).and_then(|m| m.parse::<u8>().ok());
    match (shaped, year, month) {
        (true, Some(year), Some(month)) => {
            YearMonth::new(year, month).map_err(|why| format!("month `{text}`: {why}"))
        }
        _ => Err(format!("month `{text}`: write it YYYY-MM")),
    }
}

/// `YYYY-MM-DD`, strictly.
pub(crate) fn parse_day(text: &str) -> Result<Day, String> {
    let bytes = text.as_bytes();
    let shaped = bytes.len() == 10
        && bytes.iter().enumerate().all(|(at, byte)| {
            if at == 4 || at == 7 {
                *byte == b'-'
            } else {
                byte.is_ascii_digit()
            }
        });
    let field = |range: std::ops::Range<usize>| text.get(range).and_then(|t| t.parse::<u16>().ok());
    match (shaped, field(0..4), field(5..7), field(8..10)) {
        (true, Some(year), Some(month), Some(day)) => {
            let (month, day) = (
                u8::try_from(month).map_err(|why| why.to_string())?,
                u8::try_from(day).map_err(|why| why.to_string())?,
            );
            Day::new(year, month, day).map_err(|why| format!("day `{text}`: {why}"))
        }
        _ => Err(format!("day `{text}`: write it YYYY-MM-DD")),
    }
}

/// Why `text` is not a stored share key, or `None` when it is one. A key is
/// its own canonical form: a lower-case word `Symbol::new` would upper-case
/// names a directory the store never wrote, so it is refused by name.
pub(crate) fn not_a_key(text: &str) -> Option<String> {
    match brutex_core::symbol::Symbol::new(text) {
        Err(why) => Some(why.to_string()),
        Ok(symbol) if symbol.as_str() != text => {
            Some(format!("write the stored key `{}`", symbol.as_str()))
        }
        Ok(_) => None,
    }
}

fn parse_symbols(value: &str) -> Result<Vec<String>, String> {
    // IN BYTE ORDER AND ONCE EACH, as the listing names them.
    let mut symbols = std::collections::BTreeSet::new();
    for word in value.split(',') {
        if let Some(why) = not_a_key(word) {
            return Err(format!("--symbols `{word}`: {why}"));
        }
        symbols.insert(word.to_owned());
    }
    Ok(symbols.into_iter().collect())
}

fn parse_count(flag: &str, value: &str) -> Result<usize, String> {
    match value.parse::<usize>() {
        Ok(n) if n >= 1 => Ok(n),
        _ => Err(format!("{flag} `{value}`: a whole number of at least 1")),
    }
}

/// The arguments of a month run, or the first thing wrong with them.
pub(crate) fn ask(arguments: &[&str]) -> Result<Ask, String> {
    let [first, flags @ ..] = arguments else {
        return Err("name a month, YYYY-MM".to_owned());
    };
    let month = parse_month(first)?;
    let (mut symbols, mut out, mut workers) = (None, None, None);
    let mut rest = flags.iter();
    while let Some(flag) = rest.next() {
        let value = rest.next().ok_or_else(|| format!("{flag} takes a value"))?;
        let fresh = match *flag {
            "--symbols" => symbols.replace(parse_symbols(value)?).is_none(),
            "--out" => out.replace(PathBuf::from(value)).is_none(),
            "--workers" => workers.replace(parse_count(flag, value)?).is_none(),
            other => return Err(format!("`{other}` is not a flag of this verb")),
        };
        if !fresh {
            return Err(format!("{flag} is named twice"));
        }
    }
    Ok(Ask {
        month,
        symbols,
        out: out.ok_or_else(|| "--out DIR is required".to_owned())?,
        workers,
    })
}

/// What the arguments of a factor lookup name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Lookup {
    pub(crate) symbol: String,
    pub(crate) day: Day,
    pub(crate) dir: PathBuf,
    pub(crate) repeat: usize,
}

/// `factor SYMBOL YYYY-MM-DD --dir DIR [--repeat N]`.
pub(crate) fn lookup_ask(arguments: &[&str]) -> Result<Lookup, String> {
    let [symbol, day, flags @ ..] = arguments else {
        return Err("factor takes SYMBOL YYYY-MM-DD --dir DIR".to_owned());
    };
    if let Some(why) = not_a_key(symbol) {
        return Err(format!("symbol `{symbol}`: {why}"));
    }
    let day = parse_day(day)?;
    let (mut dir, mut repeat) = (None, None);
    let mut rest = flags.iter();
    while let Some(flag) = rest.next() {
        let value = rest.next().ok_or_else(|| format!("{flag} takes a value"))?;
        let fresh = match *flag {
            "--dir" => dir.replace(PathBuf::from(value)).is_none(),
            "--repeat" => repeat.replace(parse_count(flag, value)?).is_none(),
            other => return Err(format!("`{other}` is not a flag of factor")),
        };
        if !fresh {
            return Err(format!("{flag} is named twice"));
        }
    }
    Ok(Lookup {
        symbol: (*symbol).to_owned(),
        day,
        dir: dir.ok_or_else(|| "--dir DIR is required".to_owned())?,
        repeat: repeat.unwrap_or(1),
    })
}

// ---------------------------------------------------------------------------
// The verb
// ---------------------------------------------------------------------------

/// `cli feed-reconcile`: see the module header.
pub(crate) fn command(arguments: &[&str], out: &mut String) -> u8 {
    let store = crate::store_write::root_for("feed-reconcile", arguments);
    command_with(arguments, &store, crate::commit_stamp(), out)
}

/// The verb against the store root `store` names and a stated build stamp, so
/// every arm is driven by a test without the process environment.
pub(crate) fn command_with(
    arguments: &[&str],
    store: &dyn Fn() -> Result<PathBuf, String>,
    commit: Option<&str>,
    out: &mut String,
) -> u8 {
    if let ["factor", rest @ ..] = arguments {
        return factor_command(rest, out);
    }
    if let ["months", rest @ ..] = arguments {
        return months_command(rest, store, out);
    }
    let ask = match ask(arguments) {
        Ok(ask) => ask,
        Err(why) => {
            let _ = writeln!(out, "refused: {why}\nusage: {USAGE}");
            return MISUSED;
        }
    };
    if !ask.out.is_dir() {
        let _ = writeln!(
            out,
            "FEED RECONCILE REFUSED: --out `{}` is not an existing directory; it was not \
             created and nothing was read.",
            ask.out.display()
        );
        return FAILED;
    }
    let root = match store() {
        Ok(root) => root,
        Err(why) => {
            let _ = writeln!(out, "FEED RECONCILE REFUSED: {why}. Nothing was read.");
            return FAILED;
        }
    };
    match run(&root, &ask, commit) {
        Ok(report) => report.print(out),
        Err(why) => {
            let _ = writeln!(out, "FEED RECONCILE FAILED: {why}");
            crate::note(
                &telemetry::Event::error("cli.feed_reconcile", "run failed")
                    .with("month", ask.month.to_string().as_str())
                    .with("reason", why.as_str()),
            );
            FAILED
        }
    }
}

/// `factor`: one answer from the newest version file, and when `--repeat`
/// asks, the time of that many sequential lookups (p50, p99, max).
fn factor_command(arguments: &[&str], out: &mut String) -> u8 {
    let lookup = match lookup_ask(arguments) {
        Ok(lookup) => lookup,
        Err(why) => {
            let _ = writeln!(out, "refused: {why}\nusage: {USAGE}");
            return MISUSED;
        }
    };
    let month = match YearMonth::new(lookup.day.year(), lookup.day.month()) {
        Ok(month) => month,
        Err(why) => {
            let _ = writeln!(out, "refused: {why}");
            return MISUSED;
        }
    };
    let index = match FactorIndex::load(&lookup.dir, month) {
        Ok(index) => index,
        Err(why) => {
            let _ = writeln!(out, "FACTOR REFUSED: {why}");
            return FAILED;
        }
    };
    let answer = index.factor_of(&lookup.symbol, lookup.day);
    let _ = writeln!(
        out,
        "file {} blake3 {}",
        index.file.display(),
        hex(&index.digest)
    );
    match &answer {
        Ok(pair) => {
            let _ = writeln!(out, "{} {} factor {pair}", lookup.symbol, lookup.day);
        }
        Err(refusal) => {
            let _ = writeln!(out, "{} {} refused {refusal}", lookup.symbol, lookup.day);
        }
    }
    if lookup.repeat > 1 {
        let mut nanos = Vec::with_capacity(lookup.repeat);
        for _ in 0..lookup.repeat {
            let started = Instant::now();
            let again = index.factor_of(&lookup.symbol, lookup.day);
            let took = u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX);
            nanos.push(took);
            if again != answer {
                let _ = writeln!(out, "FACTOR REFUSED: two lookups disagreed");
                return FAILED;
            }
        }
        nanos.sort_unstable();
        let _ = writeln!(
            out,
            "lookup timing (wall clock, one thread, sequential): p50 {} ns, p99 {} ns, max {} ns \
             (n={})",
            rank(&nanos, 50),
            rank(&nanos, 99),
            nanos.last().copied().unwrap_or(0),
            nanos.len()
        );
    }
    OK
}

/// `months`: every month the run spans, one per line, oldest first — the
/// order a full run must take, because each month carries from the one
/// before it.
fn months_command(
    arguments: &[&str],
    store: &dyn Fn() -> Result<PathBuf, String>,
    out: &mut String,
) -> u8 {
    if let Some(extra) = arguments.first() {
        let _ = writeln!(
            out,
            "refused: `{extra}`: months takes no argument\nusage: {USAGE}"
        );
        return MISUSED;
    }
    let held = store().and_then(|root| months_held(&root));
    match held {
        Ok(months) => {
            for month in &months {
                let _ = writeln!(out, "{month}");
            }
            OK
        }
        Err(why) => {
            let _ = writeln!(out, "FEED RECONCILE MONTHS REFUSED: {why}");
            FAILED
        }
    }
}

/// Whether a rung is one of a vendor's series this verb reads.
type RungTest = fn(Timeframe) -> bool;

/// Every month in which any NSE cash share holds a GDFL `1s`, Zerodha `1min`
/// or Zerodha `1day` month of at least one row, as the census records it
/// (`pull::ingest::census_store`, the one counter of what the store holds —
/// `pull::manifest`), oldest first. A partitioned census is read in its CASH
/// partition only, so no option series is loaded to list share months; a
/// single-file census is read whole, as it must be. A census that will not
/// load refuses by name; no directory is walked in its place.
///
/// # Errors
///
/// The census reader's refusal, naming the file.
pub(crate) fn months_held(root: &Path) -> Result<Vec<YearMonth>, String> {
    use pull::ingest::census_store::{self, Location};
    let mut months = std::collections::BTreeSet::new();
    // THE THREE SERIES THE VERB READS, one rung test per vendor.
    let series: [(Vendor, RungTest); 2] = [
        (Vendor::Gdfl, |rung| rung == Timeframe::SECOND_1),
        (Vendor::Zerodha, |rung| {
            rung == Timeframe::MINUTE_1 || rung == Timeframe::DAY_1
        }),
    ];
    for (vendor, read_rung) in series {
        let named = |why: String| format!("the {} census: {why}", vendor.as_str());
        let censuses = match census_store::locate(root, vendor).map_err(named)? {
            Location::Partitioned(_) => {
                match census_store::current_epoch(root, vendor, Segment::Cash).map_err(named)? {
                    Some(epoch) => {
                        let path = pull::manifest::epoch_path(root, vendor, Segment::Cash, epoch);
                        vec![
                            census_store::read_epoch(&path, vendor, Segment::Cash)
                                .map_err(named)?,
                        ]
                    }
                    None => Vec::new(),
                }
            }
            Location::Single(_) => census_store::read_vendor(root, vendor)
                .map_err(named)?
                .into_iter()
                .map(|read| read.census)
                .collect(),
        };
        for census in &censuses {
            for key in census.held_keys() {
                let share = key.contract.is_none()
                    && key.exchange == Exchange::Nse
                    && key.segment == Segment::Cash
                    && read_rung(key.timeframe);
                if share && census.held(key).is_some_and(|held| held.entry.rows > 0) {
                    months.insert(key.month);
                }
            }
        }
    }
    Ok(months.into_iter().collect())
}

// ---------------------------------------------------------------------------
// The store
// ---------------------------------------------------------------------------

/// One share's place in the run: its name and whether that name is a key.
struct Listed {
    symbol: String,
    refused: Option<String>,
}

const fn parts(
    vendor: Vendor,
    symbol: &str,
    timeframe: Timeframe,
    month: YearMonth,
) -> PathParts<'_> {
    PathParts {
        vendor,
        exchange: "NSE",
        segment: "CASH",
        symbol,
        contract: None,
        timeframe,
        month,
        file: FileKind::Bars,
    }
}

/// The share segment directory of a vendor and rung, rendered through
/// [`StorePath`] so the layout keeps one authority.
fn segment_dir(
    root: &Path,
    vendor: Vendor,
    timeframe: Timeframe,
    month: YearMonth,
) -> Option<PathBuf> {
    let probe = StorePath::new(parts(vendor, "PROBE", timeframe, month)).ok()?;
    let file = probe.to_path_buf(root);
    file.parent()?.parent()?.parent().map(Path::to_path_buf)
}

/// Every share directory either feed holds, by its stored name, in byte
/// order. A segment a feed does not hold lists nothing; one that cannot be
/// listed refuses the run.
fn listing(root: &Path, month: YearMonth) -> Result<Vec<Listed>, String> {
    let mut names = std::collections::BTreeSet::new();
    for (vendor, rung) in [
        (Vendor::Gdfl, Timeframe::SECOND_1),
        (Vendor::Zerodha, Timeframe::MINUTE_1),
        (Vendor::Zerodha, Timeframe::DAY_1),
    ] {
        let dir = segment_dir(root, vendor, rung, month)
            .ok_or_else(|| format!("the {} share directory has no path", vendor.as_str()))?;
        let entries = match std::fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(why) if why.kind() == std::io::ErrorKind::NotFound => continue,
            Err(why) => return Err(format!("listing `{}`: {why}", dir.display())),
        };
        for entry in entries {
            let entry = entry.map_err(|why| format!("listing `{}`: {why}", dir.display()))?;
            names.insert(entry.file_name().to_string_lossy().into_owned());
        }
    }
    Ok(names.into_iter().map(listed).collect())
}

fn listed(symbol: String) -> Listed {
    let refused = not_a_key(&symbol).map(|why| format!("not_a_share_key:{why}"));
    Listed { symbol, refused }
}

/// A month file of one share, as the read door answered.
enum Opened {
    Held(Box<BarFile>),
    Absent,
    Failed(String),
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "the id is a CROSS-CHECK the store verifies on open, never an index: the low \
              half of a 64-bit FNV-1a is what pull::ingest stamps in, and api::bars folds the \
              same way. A different fold would fail to open a file this repository wrote."
)]
fn symbol_id(symbol: &str) -> u32 {
    brutex_core::universe::fnv1a(symbol) as u32
}

fn open(
    root: &Path,
    vendor: Vendor,
    symbol: &str,
    timeframe: Timeframe,
    month: YearMonth,
) -> Opened {
    let path = match StorePath::new(parts(vendor, symbol, timeframe, month)) {
        Ok(path) => path,
        Err(why) => return Opened::Failed(why.to_string()),
    };
    match BarFile::open_existing(root, path, symbol_id(symbol)) {
        Ok(file) => Opened::Held(Box::new(file)),
        // ABSENT ONLY WHEN THE BARS THEMSELVES ARE NOT THERE. A missing
        // sidecar of a month that exists is the same variant with another
        // path, and that month is unreadable, not absent.
        Err(StoreError::Missing { path: missing, .. }) if missing == path.to_path_buf(root) => {
            Opened::Absent
        }
        Err(why) => Opened::Failed(why.to_string()),
    }
}

fn source_of(opened: &Opened) -> String {
    match opened {
        Opened::Held(file) => format!("{}@{}", file.records(), file.header().generation),
        Opened::Absent => "-".to_owned(),
        Opened::Failed(_) => "!".to_owned(),
    }
}

// ---------------------------------------------------------------------------
// Exact arithmetic
// ---------------------------------------------------------------------------

const fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let r = a % b;
        a = b;
        b = r;
    }
    a
}

/// A feed's price grid for one series-month: the gcd of the differences
/// between its distinct paisa prints so far. `0` until two distinct prints.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Grid {
    first: Option<i64>,
    gcd: u64,
}

impl Grid {
    /// One bar's four prices; whether one was not positive (never seen).
    pub(crate) fn see(&mut self, bar: &Bar) -> bool {
        let mut nonpositive = false;
        for price in [bar.open, bar.high, bar.low, bar.close] {
            if price <= 0 {
                nonpositive = true;
                continue;
            }
            match self.first {
                None => self.first = Some(price),
                Some(first) => self.gcd = gcd(self.gcd, price.abs_diff(first)),
            }
        }
        nonpositive
    }

    /// The step so far, `0` when fewer than two distinct prints were seen.
    pub(crate) const fn step(self) -> u64 {
        self.gcd
    }
}

/// An exact fraction with a positive denominator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Frac {
    /// Numerator.
    pub num: i128,
    /// Denominator, always positive.
    pub den: i128,
}

impl Frac {
    /// `self < other`, by cross-multiplication.
    #[must_use]
    pub const fn lt(self, other: Self) -> bool {
        self.num * other.den < other.num * self.den
    }

    /// `self <= other`, by cross-multiplication.
    #[must_use]
    pub const fn le(self, other: Self) -> bool {
        self.num * other.den <= other.num * self.den
    }
}

impl std::fmt::Display for Frac {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.num, self.den)
    }
}

/// One minute in both feeds: the minute of the IST day, the two closes in
/// paisa and each feed's grid step when it printed them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PairAt {
    pub(crate) minute: u16,
    pub(crate) z: i64,
    pub(crate) g: i64,
    pub(crate) sz: u64,
    pub(crate) sg: u64,
}

impl PairAt {
    /// The ratio `z/g` as a fraction.
    fn ratio(self) -> Frac {
        Frac {
            num: i128::from(self.z),
            den: i128::from(self.g),
        }
    }
}

/// `a` against `b` by ratio, then by minute: a total order, so every
/// selection over it is deterministic.
fn by_ratio(a: &PairAt, b: &PairAt) -> std::cmp::Ordering {
    let left = i128::from(a.z) * i128::from(b.g);
    let right = i128::from(b.z) * i128::from(a.g);
    left.cmp(&right).then(a.minute.cmp(&b.minute))
}

/// The allowance interval of one matched minute, or `None` when `2g ≤ s_g`
/// (no upper bound: the minute admits nothing).
pub(crate) fn bounds(pair: PairAt) -> Option<(Frac, Frac)> {
    let (z, g) = (i128::from(pair.z), i128::from(pair.g));
    let (sz, sg) = (i128::from(pair.sz), i128::from(pair.sg));
    let low_den = 2 * g + sg;
    let high_den = 2 * g - sg;
    if low_den <= 0 || high_den <= 0 || z <= 0 {
        return None;
    }
    Some((
        Frac {
            num: 2 * z - sz - 2 * TRUNCATION_PAISA,
            den: low_den,
        },
        Frac {
            num: 2 * z + sz,
            den: high_den,
        },
    ))
}

/// The companion allowance of one matched minute over its RANGES (D-5176):
/// every factor `f` for which Zerodha's `[low, high]` and GDFL's `[low, high]`
/// times `f` can hold one trade, each side widened by half its own grid step:
/// `[(2·z_low − s_z)/(2·g_high + s_g), (2·z_high + s_z)/(2·g_low − s_g)]`.
/// `None` when `2·g_low ≤ s_g` or a price is not positive (no bound).
fn range_bounds(z: ZMinute, w: Window) -> Option<(Frac, Frac)> {
    let (z_low, z_high) = (i128::from(z.low), i128::from(z.high));
    let (g_low, g_high) = (i128::from(w.low), i128::from(w.high));
    let (sz, sg) = (i128::from(z.step), i128::from(w.step));
    let high_den = 2 * g_low - sg;
    if z_low <= 0 || g_low <= 0 || high_den <= 0 {
        return None;
    }
    Some((
        Frac {
            num: 2 * z_low - sz - 2 * TRUNCATION_PAISA,
            den: 2 * g_high + sg,
        },
        Frac {
            num: 2 * z_high + sz,
            den: high_den,
        },
    ))
}

/// Whether `ratio` lies in `[low, high]`.
const fn holds(low: Frac, high: Frac, ratio: Frac) -> bool {
    low.le(ratio) && ratio.le(high)
}

/// The day's running intersection of allowance intervals, and the minute
/// that emptied it.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Running {
    low: Option<(Frac, PairAt)>,
    high: Option<(Frac, PairAt)>,
    /// The minute whose interval emptied the intersection, and the pair that
    /// holds the opposing bound.
    emptied: Option<(PairAt, PairAt)>,
}

impl Running {
    /// Admits one matched minute. O(1).
    pub(crate) fn admit(&mut self, pair: PairAt, low: Frac, high: Frac) {
        if self.emptied.is_some() {
            return;
        }
        if self.low.is_none_or(|(at, _)| at.lt(low)) {
            self.low = Some((low, pair));
        }
        if self.high.is_none_or(|(at, _)| high.lt(at)) {
            self.high = Some((high, pair));
        }
        if let (Some((l, lp)), Some((h, hp))) = (self.low, self.high)
            && h.lt(l)
        {
            let other = if lp == pair { hp } else { lp };
            self.emptied = Some((pair, other));
        }
    }

    /// The intersection, or `None` when empty or never admitted.
    pub(crate) fn interval(&self) -> Option<(Frac, Frac)> {
        if self.emptied.is_some() {
            return None;
        }
        self.low.zip(self.high).map(|((l, _), (h, _))| (l, h))
    }
}

/// The preference order of lags: 0, −1, +1, −2, +2, … (ties go to the
/// smallest |L|, then the negative).
fn lag_order() -> impl Iterator<Item = i8> {
    (0..=LAG_REACH).flat_map(|n| [-n, n].into_iter().take(if n == 0 { 1 } else { 2 }))
}

fn bin(lag: i8) -> usize {
    usize::try_from(i16::from(lag) + i16::from(LAG_REACH)).unwrap_or(0)
}

/// The most frequent lag of a 119-bin count (ties by [`lag_order`]), or
/// `None` when every bin is zero.
pub(crate) fn mode(counts: &[u32; LAG_BINS]) -> Option<i8> {
    let mut best: Option<(i8, u32)> = None;
    for lag in lag_order() {
        let count = counts.get(bin(lag)).copied().unwrap_or(0);
        if count > 0 && best.is_none_or(|(_, top)| count > top) {
            best = Some((lag, count));
        }
    }
    best.map(|(lag, _)| lag)
}

// ---------------------------------------------------------------------------
// Per-thread working memory
// ---------------------------------------------------------------------------

/// One Zerodha minute bar.
#[derive(Debug, Clone, Copy, Default)]
struct ZMinute {
    held: bool,
    high: i64,
    low: i64,
    close: i64,
    step: u64,
}

/// One IST day of Zerodha.
#[derive(Debug, Clone, Copy, Default)]
struct ZDay {
    minutes: u32,
    volume: i128,
    day_bars: u32,
    day_volume: i64,
    last_minute: Option<u16>,
    flags: u32,
}

/// One IST day of GDFL seconds, addressed by second of the day. A slot is
/// held when its tag is the current day's, so no slot is cleared per day.
struct Seconds {
    tag: Vec<u32>,
    close: Vec<i64>,
    high: Vec<i64>,
    low: Vec<i64>,
    volume: Vec<i64>,
    step: Vec<u64>,
    /// The latest held second at or before each second, after [`Self::fill`].
    last: Vec<Option<u32>>,
    current: u32,
    count: u32,
    volume_sum: i128,
    flags: u32,
}

impl Seconds {
    fn new() -> Self {
        Self {
            tag: vec![0; SECONDS_PER_DAY],
            close: vec![0; SECONDS_PER_DAY],
            high: vec![0; SECONDS_PER_DAY],
            low: vec![0; SECONDS_PER_DAY],
            volume: vec![0; SECONDS_PER_DAY],
            step: vec![0; SECONDS_PER_DAY],
            last: vec![None; SECONDS_PER_DAY],
            current: 0,
            count: 0,
            volume_sum: 0,
            flags: 0,
        }
    }

    fn begin(&mut self) {
        if let Some(next) = self.current.checked_add(1) {
            self.current = next;
        } else {
            self.tag.fill(0);
            self.current = 1;
        }
        self.count = 0;
        self.volume_sum = 0;
        self.flags = 0;
    }

    fn held(&self, second: usize) -> bool {
        self.tag.get(second) == Some(&self.current)
    }

    /// One GDFL `1s` bar. Constant time.
    fn put(&mut self, second: usize, bar: &Bar, step: u64) {
        let fresh = !self.held(second);
        if let (Some(tag), Some(close), Some(high), Some(low), Some(volume), Some(at)) = (
            self.tag.get_mut(second),
            self.close.get_mut(second),
            self.high.get_mut(second),
            self.low.get_mut(second),
            self.volume.get_mut(second),
            self.step.get_mut(second),
        ) {
            if fresh {
                *tag = self.current;
                (*high, *low, *volume) = (bar.high, bar.low, bar.volume);
                self.count = self.count.saturating_add(1);
            } else {
                *high = (*high).max(bar.high);
                *low = (*low).min(bar.low);
                *volume = volume.saturating_add(bar.volume);
            }
            *close = bar.close;
            *at = step;
            self.volume_sum += i128::from(bar.volume);
        }
    }

    /// Fills [`Self::last`] and counts the minutes holding a second. A fixed
    /// walk of the day's 86,400 slots.
    fn fill(&mut self) -> u32 {
        let mut latest = None;
        let mut minutes = 0_u32;
        let mut counted_minute = usize::MAX;
        for second in 0..SECONDS_PER_DAY {
            if self.held(second) {
                latest = u32::try_from(second).ok();
                if second / 60 != counted_minute {
                    counted_minute = second / 60;
                    minutes += 1;
                }
            }
            if let Some(slot) = self.last.get_mut(second) {
                *slot = latest;
            }
        }
        minutes
    }

    /// The last held second in `[from, to)` seconds of the day. O(1) after
    /// [`Self::fill`].
    fn last_in(&self, from: i64, to: i64) -> Option<usize> {
        let end = to.min(i64::try_from(SECONDS_PER_DAY).unwrap_or(i64::MAX));
        if end <= 0 || end <= from {
            return None;
        }
        let at = usize::try_from(end - 1).ok()?;
        let latest = self.last.get(at).copied().flatten()?;
        let latest = usize::try_from(latest).ok()?;
        (i64::try_from(latest).ok()? >= from).then_some(latest)
    }

    /// The fold of the held seconds in `[from, to)`: high, low, close, the
    /// close's grid step and the volume. At most 60 slots.
    fn window(&self, from: i64, to: i64) -> Option<Window> {
        let day = i64::try_from(SECONDS_PER_DAY).unwrap_or(i64::MAX);
        let (from, to) = (from.max(0), to.min(day));
        let mut window: Option<Window> = None;
        for second in from..to {
            let Ok(at) = usize::try_from(second) else {
                continue;
            };
            if !self.held(at) {
                continue;
            }
            let (Some(&high), Some(&low), Some(&close), Some(&step), Some(&volume)) = (
                self.high.get(at),
                self.low.get(at),
                self.close.get(at),
                self.step.get(at),
                self.volume.get(at),
            ) else {
                continue;
            };
            window = Some(match window {
                None => Window {
                    high,
                    low,
                    close,
                    step,
                    volume: i128::from(volume),
                    traded: volume > 0,
                },
                Some(w) => Window {
                    high: w.high.max(high),
                    low: w.low.min(low),
                    close,
                    step,
                    volume: w.volume + i128::from(volume),
                    traded: w.traded || volume > 0,
                },
            });
        }
        window
    }
}

/// GDFL seconds folded over one minute window.
#[derive(Debug, Clone, Copy)]
struct Window {
    high: i64,
    low: i64,
    close: i64,
    step: u64,
    volume: i128,
    traded: bool,
}

/// The per-thread working memory, sized once from the month's own shape.
pub(crate) struct Scratch {
    z: Vec<ZMinute>,
    zdays: [ZDay; MAX_DAYS],
    seconds: Seconds,
    pairs: Vec<PairAt>,
    start: i64,
    span: i64,
    outside: u64,
    z_grid: Grid,
    g_grid: Grid,
}

impl Scratch {
    pub(crate) fn new() -> Self {
        Self {
            z: vec![ZMinute::default(); MAX_DAYS * MINUTES_PER_DAY],
            zdays: [ZDay::default(); MAX_DAYS],
            seconds: Seconds::new(),
            pairs: Vec::with_capacity(MINUTES_PER_DAY),
            start: 0,
            span: 0,
            outside: 0,
            z_grid: Grid::default(),
            g_grid: Grid::default(),
        }
    }

    fn reset(&mut self, month: YearMonth) {
        let (start, end) = month.ist_bounds_micros();
        self.start = start;
        self.span = end.saturating_sub(start);
        self.outside = 0;
        self.z.fill(ZMinute::default());
        self.zdays = [ZDay::default(); MAX_DAYS];
        self.z_grid = Grid::default();
        self.g_grid = Grid::default();
    }

    fn days_in_month(&self) -> usize {
        usize::try_from(self.span / DAY_MICROS).map_or(0, |days| days.min(MAX_DAYS))
    }

    /// The month offset of a stamp, or `None` (counted) outside the month.
    fn offset(&mut self, ts: i64) -> Option<i64> {
        let offset = ts
            .checked_sub(self.start)
            .filter(|at| (0..self.span).contains(at));
        if offset.is_none() {
            self.outside = self.outside.saturating_add(1);
        }
        offset
    }

    /// One Zerodha minute. Constant time.
    fn add_minute(&mut self, bar: &Bar) {
        let Some(offset) = self.offset(bar.ts_micros) else {
            return;
        };
        let nonpositive = self.z_grid.see(bar);
        let step = self.z_grid.step();
        let minute_micros = MINUTE_SECS * SECOND_MICROS;
        let aligned = offset % minute_micros == 0;
        let Ok(slot) = usize::try_from(offset / minute_micros) else {
            return;
        };
        let mut duplicate = false;
        if aligned && let Some(minute) = self.z.get_mut(slot) {
            if minute.held {
                duplicate = true;
            } else {
                *minute = ZMinute {
                    held: true,
                    high: bar.high,
                    low: bar.low,
                    close: bar.close,
                    step,
                };
            }
        }
        let of_day = u16::try_from(slot % MINUTES_PER_DAY).unwrap_or(u16::MAX);
        if let Some(day) = self.zdays.get_mut(slot / MINUTES_PER_DAY) {
            day.minutes = day.minutes.saturating_add(1);
            day.volume += i128::from(bar.volume);
            if aligned && !duplicate {
                day.last_minute = Some(day.last_minute.map_or(of_day, |m| m.max(of_day)));
            }
            if nonpositive {
                day.flags |= NONPOSITIVE;
            }
            if !aligned {
                day.flags |= UNALIGNED;
            }
            if duplicate {
                day.flags |= DUPLICATE_MINUTES;
            }
        }
    }

    /// One Zerodha day bar. The first of a day is the one used.
    fn add_day(&mut self, bar: &Bar) {
        let Some(offset) = self.offset(bar.ts_micros) else {
            return;
        };
        let Ok(index) = usize::try_from(offset / DAY_MICROS) else {
            return;
        };
        if let Some(day) = self.zdays.get_mut(index) {
            if day.day_bars == 0 {
                day.day_volume = bar.volume;
            } else {
                day.flags |= DUPLICATE_DAY_BAR;
            }
            day.day_bars = day.day_bars.saturating_add(1);
        }
    }

    /// Every committed record of `file` into `add`; the number read.
    fn fold(&mut self, file: &BarFile, add: fn(&mut Self, &Bar)) -> Result<u64, String> {
        for index in 0..file.records() {
            let bar = file.read_record(index).map_err(|why| why.to_string())?;
            add(self, &bar);
        }
        Ok(file.records())
    }
}

/// The GDFL `1s` file streamed day by day, in stamp order.
struct Stream<'a> {
    file: &'a BarFile,
    next: u64,
    pending: Option<Bar>,
    previous: i64,
}

impl<'a> Stream<'a> {
    const fn new(file: &'a BarFile) -> Self {
        Self {
            file,
            next: 0,
            pending: None,
            previous: i64::MIN,
        }
    }

    fn peek(&mut self) -> Result<Option<Bar>, String> {
        if self.pending.is_none() && self.next < self.file.records() {
            let bar = self
                .file
                .read_record(self.next)
                .map_err(|why| format!("gdfl_1s:{why}"))?;
            if bar.ts_micros < self.previous {
                return Err(format!(
                    "gdfl_1s:record {} is stamped before the record ahead of it",
                    self.next
                ));
            }
            self.previous = bar.ts_micros;
            self.next += 1;
            self.pending = Some(bar);
        }
        Ok(self.pending)
    }

    /// Loads the seconds of day `day` into `scratch.seconds`; the number read.
    fn load_day(&mut self, scratch: &mut Scratch, day: usize) -> Result<u64, String> {
        scratch.seconds.begin();
        let offset = i64::try_from(day).map_err(|why| why.to_string())?;
        let day_start = scratch.start + offset * DAY_MICROS;
        let day_end = day_start + DAY_MICROS;
        let mut read = 0_u64;
        while let Some(bar) = self.peek()? {
            if bar.ts_micros >= day_end {
                break;
            }
            self.pending = None;
            read += 1;
            if bar.ts_micros < day_start {
                scratch.outside = scratch.outside.saturating_add(1);
                continue;
            }
            let within = bar.ts_micros - day_start;
            if within % SECOND_MICROS != 0 {
                scratch.seconds.flags |= GDFL_UNALIGNED;
            }
            let nonpositive = scratch.g_grid.see(&bar);
            if nonpositive {
                scratch.seconds.flags |= NONPOSITIVE;
            }
            let step = scratch.g_grid.step();
            if let Ok(second) = usize::try_from(within / SECOND_MICROS) {
                scratch.seconds.put(second, &bar, step);
            }
        }
        Ok(read)
    }

    /// Records stamped past the month, counted without being placed.
    fn rest(&mut self) -> Result<u64, String> {
        let mut left = u64::from(self.peek()?.is_some());
        self.pending = None;
        left += self.file.records().saturating_sub(self.next);
        Ok(left)
    }
}

// ---------------------------------------------------------------------------
// One stock-day
// ---------------------------------------------------------------------------

/// What matching at one lag found.
#[derive(Debug, Clone, Copy, Default)]
struct Matching {
    matched: u32,
    used: u32,
    high_eq: u32,
    low_eq: u32,
    grid_unknown: u32,
    unbounded: u32,
    median: Option<PairAt>,
    min: Option<PairAt>,
    max: Option<PairAt>,
    running: Running,
    /// The companion intersection over the minutes' RANGES (D-5176).
    range: Running,
    /// GDFL volume inside the continuous-phase minutes Zerodha printed.
    covered: i128,
}

/// The day being measured: its index in the month and its epoch day.
#[derive(Clone, Copy)]
struct DayAt {
    index: usize,
    epoch: u32,
}

fn phase(epoch: u32, minute: usize) -> Phase {
    pull::calendar::phase_at(
        Venue::NseCash,
        i64::from(epoch),
        u16::try_from(minute).unwrap_or(u16::MAX),
        CasEligibility::Unknown,
    )
}

/// The lower median, the minimum and the maximum of `pairs` by ratio.
fn select(pairs: &mut [PairAt]) -> (Option<PairAt>, Option<PairAt>, Option<PairAt>) {
    let min = pairs.iter().copied().min_by(by_ratio);
    let max = pairs.iter().copied().max_by(by_ratio);
    if pairs.is_empty() {
        return (None, min, max);
    }
    let middle = (pairs.len() - 1) / 2;
    let (_, median, _) = pairs.select_nth_unstable_by(middle, by_ratio);
    (Some(*median), min, max)
}

impl Scratch {
    /// The day's matching at `lag`: every held Zerodha minute against the
    /// GDFL seconds of its window shifted by `lag`. A fixed walk of 1,440
    /// minutes and at most 60 seconds each.
    fn matching(&mut self, day: DayAt, lag: i8) -> Matching {
        let mut found = Matching::default();
        self.pairs.clear();
        for minute in 0..MINUTES_PER_DAY {
            let Some(z) = self.z.get(day.index * MINUTES_PER_DAY + minute).copied() else {
                continue;
            };
            if !z.held {
                continue;
            }
            let from = i64::try_from(minute).unwrap_or(0) * MINUTE_SECS + i64::from(lag);
            let Some(window) = self.seconds.window(from, from + MINUTE_SECS) else {
                continue;
            };
            if phase(day.epoch, minute) == Phase::Continuous {
                found.covered += window.volume;
            }
            found.matched += 1;
            found.high_eq += u32::from(window.high == z.high);
            found.low_eq += u32::from(window.low == z.low);
            if z.step == 0 || window.step == 0 {
                found.grid_unknown += 1;
                continue;
            }
            let pair = PairAt {
                minute: u16::try_from(minute).unwrap_or(u16::MAX),
                z: z.close,
                g: window.close,
                sz: z.step,
                sg: window.step,
            };
            let Some((low, high)) = bounds(pair) else {
                found.unbounded += 1;
                continue;
            };
            found.running.admit(pair, low, high);
            if let Some((low, high)) = range_bounds(z, window) {
                found.range.admit(pair, low, high);
            }
            found.used += 1;
            self.pairs.push(pair);
        }
        (found.median, found.min, found.max) = select(&mut self.pairs);
        found
    }

    /// Every lag −59..=+59: the minutes whose Zerodha close equals the GDFL
    /// close at that lag converted by that lag's own lower-median ratio, and
    /// the number of Zerodha minutes asked (the shares' common denominator).
    fn scan(&mut self, day: DayAt) -> ([u32; LAG_BINS], u32) {
        let mut equal = [0_u32; LAG_BINS];
        let mut asked = 0_u32;
        for minute in 0..MINUTES_PER_DAY {
            if self
                .z
                .get(day.index * MINUTES_PER_DAY + minute)
                .is_some_and(|z| z.held && z.step > 0)
            {
                asked += 1;
            }
        }
        for lag in -LAG_REACH..=LAG_REACH {
            self.pairs.clear();
            for minute in 0..MINUTES_PER_DAY {
                let Some(z) = self.z.get(day.index * MINUTES_PER_DAY + minute).copied() else {
                    continue;
                };
                if !z.held || z.step == 0 {
                    continue;
                }
                let from = i64::try_from(minute).unwrap_or(0) * MINUTE_SECS + i64::from(lag);
                let Some(at) = self.seconds.last_in(from, from + MINUTE_SECS) else {
                    continue;
                };
                let (Some(&g), Some(&sg)) = (self.seconds.close.get(at), self.seconds.step.get(at))
                else {
                    continue;
                };
                if sg == 0 {
                    continue;
                }
                self.pairs.push(PairAt {
                    minute: u16::try_from(minute).unwrap_or(u16::MAX),
                    z: z.close,
                    g,
                    sz: z.step,
                    sg,
                });
            }
            let (median, _, _) = select(&mut self.pairs);
            let Some(median) = median else {
                continue;
            };
            let target = median.ratio();
            let count = self
                .pairs
                .iter()
                .filter(|pair| bounds(**pair).is_some_and(|(low, high)| holds(low, high, target)))
                .count();
            if let Some(slot) = equal.get_mut(bin(lag)) {
                *slot = u32::try_from(count).unwrap_or(u32::MAX);
            }
        }
        (equal, asked)
    }
}

/// The best lag of a scan and its count, `None` when no lag equalled.
fn best_of(equal: &[u32; LAG_BINS]) -> Option<(i8, u32)> {
    let mut best: Option<(i8, u32)> = None;
    for lag in lag_order() {
        let count = equal.get(bin(lag)).copied().unwrap_or(0);
        if count > 0 && best.is_none_or(|(_, top)| count > top) {
            best = Some((lag, count));
        }
    }
    best
}

/// Where the applied lag came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LagSource {
    Own,
    Pooled,
    None,
}

impl LagSource {
    const fn text(self) -> &'static str {
        match self {
            Self::Own => "own",
            Self::Pooled => "pooled",
            Self::None => "none",
        }
    }
}

// ---------------------------------------------------------------------------
// One share-month
// ---------------------------------------------------------------------------

/// A row's status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Status {
    Matched,
    GdflOnly,
    ZerodhaOnly,
    NoOverlap,
    NotAligned,
    ReadFailed(String),
}

const STATUS_NAMES: [&str; 6] = [
    "matched",
    "gdfl_only",
    "zerodha_only",
    "no_overlap",
    "not_aligned",
    "read_failed",
];

impl Status {
    fn text(&self) -> String {
        match self {
            Self::ReadFailed(why) => format!("read_failed:{}", clean(why)),
            other => STATUS_NAMES
                .get(other.index())
                .copied()
                .unwrap_or("read_failed")
                .to_owned(),
        }
    }

    const fn index(&self) -> usize {
        match self {
            Self::Matched => 0,
            Self::GdflOnly => 1,
            Self::ZerodhaOnly => 2,
            Self::NoOverlap => 3,
            Self::NotAligned => 4,
            Self::ReadFailed(_) => 5,
        }
    }
}

/// A reason made safe for one CSV field: no comma, no line break, no tab.
fn clean(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            ',' => ';',
            '\n' | '\r' | '\t' => ' ',
            other => other,
        })
        .collect()
}

/// What one share carries from one month to the next.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Carry {
    pub(crate) lags: [u32; LAG_BINS],
    /// The last matched day with a non-empty interval, and that interval.
    pub(crate) interval: Option<(u32, Frac, Frac)>,
    /// An open run of `zerodha_minute_sum_exceeds_day_bar`: first day, days.
    pub(crate) exceed_open: Option<(u32, u32)>,
    /// The first day `zerodha_session_cut` was set.
    pub(crate) cut_onset: Option<u32>,
}

impl Default for Carry {
    fn default() -> Self {
        Self {
            lags: [0; LAG_BINS],
            interval: None,
            exceed_open: None,
            cut_onset: None,
        }
    }
}

/// One share-month's outcome.
pub(crate) struct StockResult {
    pub(crate) symbol: String,
    lines: Vec<String>,
    statuses: Vec<usize>,
    flags: Vec<u32>,
    pub(crate) read_failed: bool,
    pub(crate) adjusted: bool,
    feeds: (bool, bool),
    bars: u64,
    micros: u64,
    best: [Option<i8>; MAX_DAYS],
    needs_pooled: bool,
    carry: Carry,
    spans: Vec<String>,
    onset: Option<u32>,
}

/// What every share of the run shares: the month, the carried state and,
/// in the second pass, the pooled lag counts before each day.
struct Context<'a> {
    root: &'a Path,
    month: YearMonth,
    first_day: u32,
    carried: &'a HashMap<String, Carry>,
    pooled: Option<&'a [[u32; LAG_BINS]; MAX_DAYS]>,
}

/// The three files of a share-month, folded; or the reasons they were not.
struct Folded {
    bars: u64,
    feeds: (bool, bool),
    source: String,
    failed: Vec<String>,
}

fn fold_zerodha(root: &Path, symbol: &str, month: YearMonth, scratch: &mut Scratch) -> Folded {
    let mut folded = Folded {
        bars: 0,
        feeds: (false, false),
        source: String::new(),
        failed: Vec::new(),
    };
    let mut sources = Vec::new();
    for (name, rung, add) in [
        (
            "zerodha_1min",
            Timeframe::MINUTE_1,
            Scratch::add_minute as fn(&mut Scratch, &Bar),
        ),
        ("zerodha_1day", Timeframe::DAY_1, Scratch::add_day),
    ] {
        let opened = open(root, Vendor::Zerodha, symbol, rung, month);
        sources.push(source_of(&opened));
        match opened {
            Opened::Held(file) => match scratch.fold(&file, add) {
                Ok(read) => {
                    folded.bars += read;
                    folded.feeds.1 |= read > 0;
                }
                Err(why) => folded.failed.push(format!("{name}:{why}")),
            },
            Opened::Absent => {}
            Opened::Failed(why) => folded.failed.push(format!("{name}:{why}")),
        }
    }
    folded.source = sources.join(";");
    folded
}

/// Rounds `num / den` half up (both positive). Volume conversion only.
const fn round_half_up(num: i128, den: i128) -> i128 {
    (2 * num + den) / (2 * den)
}

/// One stock-day, measured and rendered.
struct DayOut {
    line: String,
    status: Status,
    flags: u32,
    best: Option<i8>,
    adjusted: bool,
    exceed: Option<bool>,
    cut: bool,
}

/// Everything one day's row needs, gathered before it is rendered.
struct DayInputs<'a> {
    symbol: &'a str,
    day: DayAt,
    zday: ZDay,
    g_seconds: u32,
    g_volume: i128,
    g_minutes: u32,
    g_unverified: i128,
    g_flags: u32,
    source: &'a str,
}

#[expect(
    clippy::float_arithmetic,
    clippy::cast_precision_loss,
    reason = "a percentage difference of two volumes is a statistic at full precision \
              (CLAUDE.md §7); a day's volume is far below 2^53"
)]
fn percent(part: i128, whole: i128) -> Option<f64> {
    (whole != 0).then(|| 100.0 * (part - whole) as f64 / whole as f64)
}

fn number<T: std::fmt::Display>(value: Option<T>) -> String {
    value.map_or_else(String::new, |v| v.to_string())
}

fn day_label(epoch: u32) -> String {
    Day::from_days(epoch).map_or_else(|_| epoch.to_string(), |day| day.to_string())
}

fn clock(minute: u16) -> String {
    format!("{:02}:{:02}", minute / 60, minute % 60)
}

fn pair_text(pair: Option<PairAt>) -> String {
    pair.map_or_else(String::new, |p| format!("{}/{}", p.z, p.g))
}

/// What a day is in the calendar.
fn day_kind(epoch: u32) -> String {
    match pull::calendar::kind_of(i64::from(epoch)) {
        DayKind::Open(session) => format!("open_{}", session.bars()),
        DayKind::OpenLengthUnmeasured => "open_length_unmeasured".to_owned(),
        DayKind::Closed => "closed".to_owned(),
        DayKind::Unmeasured => "unmeasured".to_owned(),
    }
}

/// The cash closing-auction era (`docs/00-charter.md` §9, NSE/CMTR/73845),
/// asked of `pull::vendor`, the one holder of its first day, so this module
/// types no date of its own.
fn auction_era(epoch: u32) -> bool {
    Day::from_days(epoch).is_ok_and(pull::vendor::cash_auction_eligibility_required)
}

/// The volume columns: explained, short, its pair over the day bar, both on
/// Zerodha's basis on an adjusted day, and the judgement.
fn volume_columns(
    inputs: &DayInputs<'_>,
    matching: Option<&Matching>,
    adjusted: Option<PairAt>,
) -> [String; 6] {
    let zday = inputs.zday;
    let refuse = |why: &str| {
        [
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            format!("not_checked:{why}"),
        ]
    };
    if inputs.g_seconds == 0 {
        return refuse("no_gdfl_seconds");
    }
    if zday.day_bars == 0 {
        return refuse("no_day_bar");
    }
    if zday.minutes == 0 {
        return refuse("no_zerodha_minutes");
    }
    let Some(matching) = matching else {
        return refuse("not_aligned");
    };
    let explained = inputs.g_volume - matching.covered;
    let gap = i128::from(zday.day_volume) - zday.volume;
    let short = gap - explained;
    let (explained_z, short_z) = adjusted.map_or((String::new(), String::new()), |median| {
        let converted = round_half_up(explained * i128::from(median.g), i128::from(median.z));
        (converted.to_string(), (gap - converted).to_string())
    });
    [
        explained.to_string(),
        short.to_string(),
        format!("{short}/{}", zday.day_volume),
        explained_z,
        short_z,
        "unjudged:no_env1_envelope".to_owned(),
    ]
}

/// The row of a day both feeds or either feed holds.
#[expect(
    clippy::too_many_lines,
    reason = "one stock-day's whole row: every column is computed beside the rule that names it, \
              in the order the header prints them"
)]
fn measure_day(
    scratch: &mut Scratch,
    inputs: &DayInputs<'_>,
    applied: (Option<i8>, LagSource),
    carry: &mut Carry,
    ex_date_note: &str,
) -> DayOut {
    let zday = inputs.zday;
    let (g_any, z_any) = (inputs.g_seconds > 0, zday.minutes > 0 || zday.day_bars > 0);
    let both = g_any && zday.minutes > 0;
    let mut flags = zday.flags | inputs.g_flags;
    if zday.day_bars == 0 && zday.minutes > 0 {
        flags |= NO_DAY_BAR;
    }
    if zday.day_bars > 0 && zday.minutes == 0 {
        flags |= NO_MINUTES;
    }
    if zday.day_bars > 0 && zday.day_volume == 0 {
        flags |= ZERO_DAY_VOLUME;
    }
    // THE SCAN NEVER DEPENDS ON THE APPLIED LAG, so the best lag a day feeds
    // the history is the same in both passes.
    let (equal, asked) = if both {
        scratch.scan(inputs.day)
    } else {
        ([0; LAG_BINS], 0)
    };
    let best = best_of(&equal);
    let (lag, source) = applied;
    let matching = match (both, lag) {
        (true, Some(lag)) => Some(scratch.matching(inputs.day, lag)),
        _ => None,
    };
    let status = match (g_any, z_any, both, lag, matching.map(|m| m.used)) {
        (true, false, ..) => Status::GdflOnly,
        (false, ..) => Status::ZerodhaOnly,
        (true, true, true, None, _) => Status::NotAligned,
        (true, true, true, Some(_), Some(used)) if used > 0 => Status::Matched,
        _ => Status::NoOverlap,
    };
    let interval = matching.and_then(|m| m.running.interval());
    let median = matching.and_then(|m| m.median);
    let adjusted_pair = median.filter(|pair| {
        bounds(*pair).is_some_and(|(low, high)| !holds(low, high, Frac { num: 1, den: 1 }))
    });
    if let Some(m) = matching {
        if m.running.emptied.is_some() {
            flags |= FACTOR_NOT_CONSTANT;
        }
        if m.range.emptied.is_some() {
            flags |= RANGE_NOT_CONSTANT;
        }
        if adjusted_pair.is_some() {
            flags |= ADJUSTED;
        }
        if m.grid_unknown > 0 {
            flags |= GRID_UNKNOWN;
        }
        if m.unbounded > 0 {
            flags |= UNBOUNDED;
        }
    }
    // EX-DATE: two consecutive matched days with disjoint intervals. A day's
    // interval is its CLOSE intersection, or where that emptied its RANGE
    // intersection (D-5177): the closes of the two feeds are often different
    // trades of one minute, so on the real store the close intersection
    // empties on almost every day and an ex-date judged on it alone could
    // never fire. Each per-minute close interval lies inside that minute's
    // range interval, so either is a set of factors the whole day admits.
    let mut ex_date = String::new();
    let basis = interval.map(|span| (span, "close")).or_else(|| {
        matching
            .and_then(|m| m.range.interval())
            .map(|span| (span, "range"))
    });
    if let Some(((low, high), named)) = basis {
        ex_date = match carry.interval {
            Some((_, before_low, before_high)) => {
                if before_high.lt(low) || high.lt(before_low) {
                    flags |= EX_DATE;
                    format!("yes:unsourced:{named}")
                } else {
                    format!("no:{named}")
                }
            }
            None => format!("unchecked:{ex_date_note}:{named}"),
        };
        carry.interval = Some((inputs.day.epoch, low, high));
    } else if matching.is_some_and(|m| m.used > 0) {
        // Both intersections emptied: the day admits no one factor, so it is
        // judged against nothing and hands nothing to the next day.
        "unchecked:no_interval".clone_into(&mut ex_date);
    }
    // MINUTE SUM ABOVE THE DAY BAR, beyond the rounding allowance.
    let mut allowance = String::new();
    let mut exceed = None;
    if zday.day_bars > 0 && zday.minutes > 0 {
        let half = i128::from((zday.minutes + 1).div_ceil(2));
        let (room, basis) = match (median.is_some(), adjusted_pair.is_some()) {
            (true, false) => (0_i128, "unadjusted"),
            (true, true) => (half, "adjusted"),
            (false, _) => (half, "unknown"),
        };
        allowance = format!("{room}:{basis}");
        let over = zday.volume > i128::from(zday.day_volume) + room;
        if over {
            flags |= SUM_EXCEEDS_DAY;
        }
        exceed = Some(over);
    }
    // SESSION CUT: GDFL trades in a continuous minute after Zerodha's last.
    let mut cut_minute = String::new();
    let mut cut = false;
    if both {
        match (lag, zday.last_minute) {
            (Some(lag), Some(last)) => {
                let first_after = usize::from(last) + 1;
                let mut first_continuous = None;
                for minute in first_after..MINUTES_PER_DAY {
                    if phase(inputs.day.epoch, minute) != Phase::Continuous {
                        continue;
                    }
                    first_continuous.get_or_insert(minute);
                    let from = i64::try_from(minute).unwrap_or(0) * MINUTE_SECS + i64::from(lag);
                    if scratch
                        .seconds
                        .window(from, from + MINUTE_SECS)
                        .is_some_and(|w| w.traded)
                    {
                        cut = true;
                        break;
                    }
                }
                if cut {
                    flags |= SESSION_CUT;
                    cut_minute = first_continuous
                        .and_then(|m| u16::try_from(m).ok())
                        .map_or_else(String::new, clock);
                }
            }
            (None, _) => "not_checked:not_aligned".clone_into(&mut cut_minute),
            (Some(_), None) => "not_checked:no_aligned_minute".clone_into(&mut cut_minute),
        }
    } else if z_any && zday.minutes > 0 {
        "not_checked:no_gdfl_seconds".clone_into(&mut cut_minute);
    }
    let volumes = volume_columns(inputs, matching.as_ref(), adjusted_pair);
    let g_volume = g_any.then_some(inputs.g_volume);
    let day_bar_volume = (zday.day_bars > 0).then_some(i128::from(zday.day_volume));
    let z_minutes = (zday.minutes > 0).then_some(zday.volume);
    let both_pct = |a: Option<i128>, b: Option<i128>| a.zip(b).and_then(|(a, b)| percent(a, b));
    let lag0 = equal.get(bin(0)).copied().unwrap_or(0);
    let emptied = matching.and_then(|m| m.running.emptied);
    let range = matching.and_then(|m| m.range.interval());
    let range_emptied = matching.and_then(|m| m.range.emptied);
    let share = |count: u32, of: u32| (of > 0).then(|| format!("{count}/{of}"));
    let auction = (auction_era(inputs.day.epoch) && zday.day_bars > 0 && zday.minutes > 0)
        .then(|| i128::from(zday.day_volume) - zday.volume);
    let columns: [String; COLUMN_COUNT] = [
        inputs.symbol.to_owned(),
        day_label(inputs.day.epoch),
        day_kind(inputs.day.epoch),
        status.text(),
        if both { number(lag) } else { String::new() },
        if both {
            source.text().to_owned()
        } else {
            String::new()
        },
        number(g_volume),
        number(day_bar_volume),
        number(z_minutes),
        number(both_pct(g_volume, day_bar_volume)),
        number(both_pct(z_minutes, day_bar_volume)),
        number(both_pct(z_minutes, g_volume)),
        inputs.g_seconds.to_string(),
        inputs.g_minutes.to_string(),
        zday.minutes.to_string(),
        number(matching.map(|m| m.matched)),
        number(median.map(|p| p.z)),
        number(median.map(|p| p.g)),
        number(matching.and_then(|m| m.min).map(|p| p.z)),
        number(matching.and_then(|m| m.min).map(|p| p.g)),
        number(matching.and_then(|m| m.max).map(|p| p.z)),
        number(matching.and_then(|m| m.max).map(|p| p.g)),
        number(interval.map(|(low, _)| low)),
        number(interval.map(|(_, high)| high)),
        emptied.map_or_else(String::new, |(at, _)| clock(at.minute)),
        pair_text(emptied.map(|(at, _)| at)),
        pair_text(emptied.map(|(_, other)| other)),
        number(range.map(|(low, _)| low)),
        number(range.map(|(_, high)| high)),
        range_emptied.map_or_else(String::new, |(at, _)| clock(at.minute)),
        if z_any {
            step_text(scratch.z_grid.step())
        } else {
            String::new()
        },
        if g_any {
            step_text(scratch.g_grid.step())
        } else {
            String::new()
        },
        number(matching.and_then(|m| share(m.high_eq, m.matched))),
        number(matching.and_then(|m| share(m.low_eq, m.matched))),
        number(share(lag0, asked).filter(|_| both)),
        number(best.map(|(lag, _)| lag)),
        number(best.and_then(|(_, count)| share(count, asked))),
        volumes[0].clone(),
        volumes[1].clone(),
        volumes[2].clone(),
        volumes[3].clone(),
        volumes[4].clone(),
        volumes[5].clone(),
        allowance,
        number(g_any.then_some(inputs.g_unverified)),
        number(auction),
        cut_minute,
        ex_date,
        flag_text(flags),
        inputs.source.to_owned(),
    ];
    DayOut {
        line: columns.join(","),
        status,
        flags,
        best: best.map(|(lag, _)| lag),
        adjusted: adjusted_pair.is_some(),
        exceed,
        cut,
    }
}

fn step_text(step: u64) -> String {
    if step == 0 {
        "unknown".to_owned()
    } else {
        step.to_string()
    }
}

/// The flags of a bitmask, `|`-separated, in [`FLAG_NAMES`] order.
pub(crate) fn flag_text(flags: u32) -> String {
    let mut text = String::new();
    for (bit, name) in FLAG_NAMES.iter().enumerate() {
        if flags & (1 << bit) != 0 {
            if !text.is_empty() {
                text.push('|');
            }
            text.push_str(name);
        }
    }
    text
}

/// A row for a share or day that could not be read: the status, empty
/// measurements, the source.
fn failed_line(symbol: &str, day: &str, kind: &str, status: &Status, source: &str) -> String {
    let mut columns = vec![
        clean(symbol),
        day.to_owned(),
        kind.to_owned(),
        status.text(),
    ];
    columns.resize(COLUMN_COUNT - 1, String::new());
    columns.push(clean(source));
    columns.join(",")
}

/// One share's whole month.
#[expect(
    clippy::too_many_lines,
    reason = "one share-month in stamp order: the three files, the day loop and the read_failed \
              rows are one pass, so no day is measured from a half-read month"
)]
fn reconcile(context: &Context<'_>, listed: &Listed, scratch: &mut Scratch) -> StockResult {
    let started = Instant::now();
    let symbol = listed.symbol.as_str();
    let mut carry = context.carried.get(symbol).cloned().unwrap_or_default();
    let ex_date_note = if context.carried.contains_key(symbol) {
        "no_earlier_interval"
    } else {
        "no_carried_history"
    };
    let mut result = StockResult {
        symbol: listed.symbol.clone(),
        lines: Vec::new(),
        statuses: Vec::new(),
        flags: Vec::new(),
        read_failed: false,
        adjusted: false,
        feeds: (false, false),
        bars: 0,
        micros: 0,
        best: [None; MAX_DAYS],
        needs_pooled: false,
        carry: carry.clone(),
        spans: Vec::new(),
        onset: None,
    };
    if let Some(why) = &listed.refused {
        let status = Status::ReadFailed(why.clone());
        result
            .lines
            .push(failed_line(symbol, "-", "", &status, "-"));
        result.statuses.push(status.index());
        result.flags.push(0);
        result.read_failed = true;
        return result.timed(started);
    }
    scratch.reset(context.month);
    let mut folded = fold_zerodha(context.root, symbol, context.month, scratch);
    let gdfl = open(
        context.root,
        Vendor::Gdfl,
        symbol,
        Timeframe::SECOND_1,
        context.month,
    );
    let source = format!("{};{}", source_of(&gdfl), folded.source);
    let mut stream = match &gdfl {
        Opened::Held(file) => Some(Stream::new(file)),
        Opened::Absent => None,
        Opened::Failed(why) => {
            folded.failed.push(format!("gdfl_1s:{why}"));
            None
        }
    };
    let days = scratch.days_in_month();
    let mut loaded: Vec<(u32, i128, u32, i128, u32)> = Vec::with_capacity(days);
    // PASS OVER THE GDFL SECONDS, ONE DAY AT A TIME. The rows need the days
    // in order, so each day is measured as soon as its seconds are in.
    let mut outs: Vec<(DayAt, Option<DayOut>)> = Vec::with_capacity(days);
    let mut exceed_run = carry.exceed_open;
    for index in 0..days {
        let epoch = context
            .first_day
            .saturating_add(u32::try_from(index).unwrap_or(u32::MAX));
        let day = DayAt { index, epoch };
        let (mut g_seconds, mut g_volume, mut g_minutes, mut g_unverified, mut g_flags) =
            (0, 0, 0, 0, 0);
        if let Some(reader) = stream.as_mut()
            && folded.failed.is_empty()
        {
            match reader.load_day(scratch, index) {
                Ok(read) => {
                    folded.bars += read;
                    g_seconds = scratch.seconds.count;
                    g_volume = scratch.seconds.volume_sum;
                    g_flags = scratch.seconds.flags;
                    if g_seconds > 0 {
                        folded.feeds.0 = true;
                        g_minutes = scratch.seconds.fill();
                        g_unverified = unverified_volume(scratch, epoch);
                    }
                }
                Err(why) => folded.failed.push(why),
            }
        }
        loaded.push((g_seconds, g_volume, g_minutes, g_unverified, g_flags));
        let zday = scratch.zdays.get(index).copied().unwrap_or_default();
        if !folded.failed.is_empty() {
            outs.push((day, None));
            continue;
        }
        if g_seconds == 0 && zday.minutes == 0 && zday.day_bars == 0 {
            continue;
        }
        let both = g_seconds > 0 && zday.minutes > 0;
        let applied = if !both {
            (None, LagSource::None)
        } else if let Some(lag) = mode(&carry.lags) {
            (Some(lag), LagSource::Own)
        } else if let Some(pooled) = context.pooled {
            pooled
                .get(index)
                .and_then(mode)
                .map_or((None, LagSource::None), |lag| {
                    (Some(lag), LagSource::Pooled)
                })
        } else {
            result.needs_pooled = true;
            (None, LagSource::None)
        };
        let inputs = DayInputs {
            symbol,
            day,
            zday,
            g_seconds,
            g_volume,
            g_minutes,
            g_unverified,
            g_flags,
            source: &source,
        };
        let out = measure_day(scratch, &inputs, applied, &mut carry, ex_date_note);
        if let Some(lag) = out.best {
            if let Some(slot) = carry.lags.get_mut(bin(lag)) {
                *slot += 1;
            }
            if let Some(slot) = result.best.get_mut(index) {
                *slot = Some(lag);
            }
        }
        match out.exceed {
            Some(true) => {
                exceed_run = Some(exceed_run.map_or((epoch, 1), |(first, n)| (first, n + 1)));
            }
            Some(false) => {
                if let Some((first, n)) = exceed_run.take() {
                    result.spans.push(span_line(symbol, first, n, None));
                }
            }
            None => {}
        }
        if out.cut && carry.cut_onset.is_none() {
            carry.cut_onset = Some(epoch);
            result.onset = Some(epoch);
        }
        outs.push((day, Some(out)));
    }
    if let Some(reader) = stream.as_mut()
        && folded.failed.is_empty()
    {
        match reader.rest() {
            Ok(rest) => scratch.outside = scratch.outside.saturating_add(rest),
            Err(why) => folded.failed.push(why),
        }
    }
    carry.exceed_open = exceed_run;
    result.bars = folded.bars;
    result.feeds = folded.feeds;
    let outside = if scratch.outside > 0 {
        OUTSIDE_MONTH
    } else {
        0
    };
    if folded.failed.is_empty() {
        for (_, out) in outs {
            let Some(out) = out else { continue };
            let mut line = out.line;
            let mut flags = out.flags;
            if outside != 0 {
                flags |= outside;
                line = refl(&line, flags);
            }
            result.adjusted |= out.adjusted;
            result.statuses.push(out.status.index());
            result.flags.push(flags);
            result.lines.push(line);
        }
        if let Some((first, n)) = carry.exceed_open {
            result.spans.push(span_line(symbol, first, n, Some("open")));
        }
    } else {
        // A FILE THAT EXISTS AND CANNOT BE READ: every day any file holds is
        // `read_failed`, never a half-read number.
        let status = Status::ReadFailed(folded.failed.join(" + "));
        result.read_failed = true;
        result.spans.clear();
        for index in 0..days {
            let zday = scratch.zdays.get(index).copied().unwrap_or_default();
            let g_seconds = loaded.get(index).map_or(0, |l| l.0);
            if g_seconds == 0 && zday.minutes == 0 && zday.day_bars == 0 {
                continue;
            }
            let epoch = context
                .first_day
                .saturating_add(u32::try_from(index).unwrap_or(u32::MAX));
            result.lines.push(failed_line(
                symbol,
                &day_label(epoch),
                &day_kind(epoch),
                &status,
                &source,
            ));
            result.statuses.push(status.index());
            result.flags.push(0);
        }
        if result.lines.is_empty() {
            result
                .lines
                .push(failed_line(symbol, "-", "", &status, &source));
            result.statuses.push(status.index());
            result.flags.push(0);
        }
        // A share whose month could not be read carries forward unchanged.
        carry = context.carried.get(symbol).cloned().unwrap_or_default();
    }
    result.carry = carry;
    result.timed(started)
}

/// GDFL volume in seconds whose minute is in the `Unverified` phase (an
/// unknown closing-auction eligibility from 2026-08-03, or an unmeasured
/// day). A walk of the 1,440 minutes and their held seconds.
fn unverified_volume(scratch: &Scratch, epoch: u32) -> i128 {
    let mut total = 0_i128;
    for minute in 0..MINUTES_PER_DAY {
        if phase(epoch, minute) != Phase::Unverified {
            continue;
        }
        let from = i64::try_from(minute).unwrap_or(0) * MINUTE_SECS;
        if let Some(window) = scratch.seconds.window(from, from + MINUTE_SECS) {
            total += window.volume;
        }
    }
    total
}

/// Replaces a rendered row's flags column.
fn refl(line: &str, flags: u32) -> String {
    let mut fields: Vec<&str> = line.split(',').collect();
    let text = flag_text(flags);
    if let Some(slot) = fields.get_mut(COLUMN_COUNT - 2) {
        *slot = text.as_str();
    }
    fields.join(",")
}

fn span_line(symbol: &str, first: u32, days: u32, open: Option<&str>) -> String {
    format!(
        "# span,zerodha_minute_sum_exceeds_day_bar,{symbol},{},{days}{}",
        day_label(first),
        open.map_or_else(String::new, |o| format!(",{o}"))
    )
}

impl StockResult {
    fn timed(mut self, started: Instant) -> Self {
        self.micros = u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX);
        self
    }
}

// ---------------------------------------------------------------------------
// Carry lines
// ---------------------------------------------------------------------------

fn lags_text(lags: &[u32; LAG_BINS]) -> String {
    let mut parts = Vec::new();
    for lag in -LAG_REACH..=LAG_REACH {
        let count = lags.get(bin(lag)).copied().unwrap_or(0);
        if count > 0 {
            parts.push(format!("{lag}:{count}"));
        }
    }
    parts.join(";")
}

fn parse_lags(text: &str) -> Result<[u32; LAG_BINS], String> {
    let mut lags = [0_u32; LAG_BINS];
    for part in text.split(';').filter(|p| !p.is_empty()) {
        let (lag, count) = part
            .split_once(':')
            .ok_or_else(|| format!("lag count `{part}`"))?;
        let lag: i8 = lag.parse().map_err(|_| format!("lag `{lag}`"))?;
        if lag.unsigned_abs() > LAG_REACH.unsigned_abs() {
            return Err(format!("lag `{lag}` beyond the reach"));
        }
        let count: u32 = count.parse().map_err(|_| format!("count `{count}`"))?;
        if let Some(slot) = lags.get_mut(bin(lag)) {
            *slot = count;
        }
    }
    Ok(lags)
}

fn carry_line(symbol: &str, carry: &Carry) -> String {
    let interval = carry.interval.map_or_else(
        || "-".to_owned(),
        |(day, low, high)| format!("{}:{low}:{high}", day_label(day)),
    );
    let exceed = carry.exceed_open.map_or_else(
        || "-".to_owned(),
        |(first, n)| format!("{}:{n}", day_label(first)),
    );
    let onset = carry.cut_onset.map_or_else(|| "-".to_owned(), day_label);
    format!(
        "# carry,{symbol},lags={},interval={interval},exceed_open={exceed},cut_onset={onset}",
        lags_text(&carry.lags)
    )
}

fn parse_frac(text: &str) -> Result<Frac, String> {
    let (num, den) = text
        .split_once('/')
        .ok_or_else(|| format!("fraction `{text}`"))?;
    let num = num.parse().map_err(|_| format!("numerator `{num}`"))?;
    let den: i128 = den.parse().map_err(|_| format!("denominator `{den}`"))?;
    if den <= 0 {
        return Err(format!("denominator `{den}` is not positive"));
    }
    Ok(Frac { num, den })
}

fn parse_carry(line: &str) -> Result<(String, Carry), String> {
    let fields: Vec<&str> = line.split(',').collect();
    let [_, symbol, lags, interval, exceed, onset] = fields.as_slice() else {
        return Err(format!("carry line `{line}`"));
    };
    let value = |field: &str, key: &str| -> Result<String, String> {
        field
            .strip_prefix(key)
            .map(str::to_owned)
            .ok_or_else(|| format!("carry field `{field}` is not {key}"))
    };
    let lags = parse_lags(&value(lags, "lags=")?)?;
    let interval_text = value(interval, "interval=")?;
    let interval = if interval_text == "-" {
        None
    } else {
        let mut parts = interval_text.splitn(3, ':');
        let (Some(day), Some(low), Some(high)) = (parts.next(), parts.next(), parts.next()) else {
            return Err(format!("interval `{interval_text}`"));
        };
        Some((
            parse_day(day)?.days_from_epoch(),
            parse_frac(low)?,
            parse_frac(high)?,
        ))
    };
    let exceed_text = value(exceed, "exceed_open=")?;
    let exceed_open = if exceed_text == "-" {
        None
    } else {
        let (day, n) = exceed_text
            .split_once(':')
            .ok_or_else(|| format!("exceed_open `{exceed_text}`"))?;
        Some((
            parse_day(day)?.days_from_epoch(),
            n.parse().map_err(|_| format!("days `{n}`"))?,
        ))
    };
    let onset_text = value(onset, "cut_onset=")?;
    let cut_onset = if onset_text == "-" {
        None
    } else {
        Some(parse_day(&onset_text)?.days_from_epoch())
    };
    Ok((
        (*symbol).to_owned(),
        Carry {
            lags,
            interval,
            exceed_open,
            cut_onset,
        },
    ))
}

/// What the previous month's newest file hands this month.
#[derive(Debug)]
pub(crate) struct Carried {
    pub(crate) note: String,
    pub(crate) stocks: HashMap<String, Carry>,
    pub(crate) pooled: [u32; LAG_BINS],
}

impl Default for Carried {
    fn default() -> Self {
        Self {
            note: String::new(),
            stocks: HashMap::with_capacity(0),
            pooled: [0; LAG_BINS],
        }
    }
}

fn previous(month: YearMonth) -> Option<YearMonth> {
    if month.month() == 1 {
        YearMonth::new(month.year().checked_sub(1)?, 12).ok()
    } else {
        YearMonth::new(month.year(), month.month() - 1).ok()
    }
}

/// The previous month's newest version file in `dir`, parsed. Absent or of
/// another format: no carry, and the header says why.
pub(crate) fn carried(dir: &Path, month: YearMonth) -> Result<Carried, String> {
    let Some(before) = previous(month) else {
        return Ok(Carried {
            note: "none:no_previous_month".to_owned(),
            ..Carried::default()
        });
    };
    let Some((file, version)) = newest(dir, before)? else {
        return Ok(Carried {
            note: format!("none:no_file_for_{before}"),
            ..Carried::default()
        });
    };
    let text = std::fs::read_to_string(&file)
        .map_err(|why| format!("reading `{}`: {why}", file.display()))?;
    if text.lines().next() != Some(FORMAT) {
        return Ok(Carried {
            note: format!("none:{}_is_not_format_2", name(before, version)),
            ..Carried::default()
        });
    }
    let shares = text
        .lines()
        .filter(|line| line.starts_with("# carry,"))
        .count();
    let mut carried = Carried {
        note: name(before, version),
        stocks: HashMap::with_capacity(shares),
        pooled: [0; LAG_BINS],
    };
    for line in text.lines() {
        if line.starts_with("# carry,") {
            let (symbol, carry) = parse_carry(line)?;
            carried.stocks.insert(symbol, carry);
        } else if let Some(rest) = line.strip_prefix("# pooled,lags=") {
            carried.pooled = parse_lags(rest)?;
        }
    }
    Ok(carried)
}

// ---------------------------------------------------------------------------
// Workers
// ---------------------------------------------------------------------------

/// The rounds a run took: workers, items, bars and wall micros of each.
#[derive(Debug, Default, Clone)]
pub(crate) struct Rounds {
    pub(crate) log: Vec<(usize, usize, u64, u64)>,
    pub(crate) last: usize,
}

/// Runs `work` over `items` on worker threads. With `fixed`, that many from
/// the start; otherwise start at 1 and double while the throughput (bars per
/// microsecond) of the last full round still rises, up to `ceiling`. Results
/// come back in item order, so nothing depends on which thread did what.
pub(crate) fn schedule<T, F>(
    items: usize,
    fixed: Option<usize>,
    ceiling: usize,
    work: F,
) -> Result<(Vec<T>, Rounds), String>
where
    T: Send,
    F: Fn(usize, &mut Scratch) -> (T, u64) + Sync,
{
    let ceiling = ceiling.clamp(1, items.max(1));
    let mut workers = fixed.map_or(1, |n| n.clamp(1, items.max(1)));
    let mut adapting = fixed.is_none() && workers < ceiling;
    let mut best: Option<(u64, u64)> = None;
    let mut results: Vec<Option<T>> = std::iter::repeat_with(|| None).take(items).collect();
    let mut rounds = Rounds::default();
    let mut from = 0_usize;
    while from < items {
        let take = if adapting {
            workers.min(items - from)
        } else {
            items - from
        };
        let started = Instant::now();
        let next = AtomicUsize::new(from);
        let end = from + take;
        let threads = workers.min(take).max(1);
        let done: Vec<Vec<(usize, T, u64)>> = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..threads)
                .map(|_| {
                    scope.spawn(|| {
                        let mut scratch = Scratch::new();
                        let mut mine = Vec::new();
                        loop {
                            let at = next.fetch_add(1, Ordering::Relaxed);
                            if at >= end {
                                break;
                            }
                            let (value, bars) = work(at, &mut scratch);
                            mine.push((at, value, bars));
                        }
                        mine
                    })
                })
                .collect();
            handles
                .into_iter()
                .map(|handle| {
                    handle
                        .join()
                        .map_err(|_| "a worker thread panicked".to_owned())
                })
                .collect::<Result<_, _>>()
        })?;
        let mut bars = 0_u64;
        for (at, value, read) in done.into_iter().flatten() {
            bars = bars.saturating_add(read);
            if let Some(slot) = results.get_mut(at) {
                *slot = Some(value);
            }
        }
        let micros = u64::try_from(started.elapsed().as_micros())
            .unwrap_or(u64::MAX)
            .max(1);
        rounds.log.push((threads, take, bars, micros));
        if adapting {
            let full = take == workers;
            let rises = best.is_none_or(|(b, m)| {
                u128::from(bars) * u128::from(m) > u128::from(b) * u128::from(micros)
            });
            if full && rises && workers < ceiling {
                best = Some((bars, micros));
                workers = (workers * 2).min(ceiling);
            } else {
                adapting = false;
            }
        }
        from = end;
    }
    rounds.last = workers;
    let results = results
        .into_iter()
        .collect::<Option<Vec<T>>>()
        .ok_or_else(|| "a worker left an item unanswered".to_owned())?;
    Ok((results, rounds))
}

// ---------------------------------------------------------------------------
// The run
// ---------------------------------------------------------------------------

/// What a run produced.
pub(crate) struct Report {
    pub(crate) file: PathBuf,
    pub(crate) version: u32,
    pub(crate) summary: String,
    timing: String,
    pub(crate) failed: bool,
}

impl Report {
    fn print(&self, out: &mut String) -> u8 {
        let _ = writeln!(
            out,
            "FEED RECONCILE v{} -> {}\n{}\n{}",
            self.version,
            self.file.display(),
            self.summary,
            self.timing
        );
        if self.failed {
            let _ = writeln!(
                out,
                "FEED RECONCILE INCOMPLETE: at least one share-month could not be read; its rows \
                 say read_failed with the reason. The file was still written."
            );
            FAILED
        } else {
            OK
        }
    }
}

/// The run: list, carry in, reconcile (twice where a share needs the pooled
/// lag), render, publish, summarise.
#[expect(
    clippy::too_many_lines,
    reason = "the run's two passes, the pooled lag between them, the publish and the run \
              event read as one sequence"
)]
pub(crate) fn run(root: &Path, ask: &Ask, commit: Option<&str>) -> Result<Report, String> {
    let started = Instant::now();
    let listing = match &ask.symbols {
        Some(symbols) => symbols.iter().cloned().map(listed).collect(),
        None => listing(root, ask.month)?,
    };
    let carried = carried(&ask.out, ask.month)?;
    let first_day = Day::new(ask.month.year(), ask.month.month(), 1)
        .map_err(|why| why.to_string())?
        .days_from_epoch();
    let ceiling = std::thread::available_parallelism()
        .map_or(1, std::num::NonZeroUsize::get)
        .min(listing.len().max(1));
    let first = Context {
        root,
        month: ask.month,
        first_day,
        carried: &carried.stocks,
        pooled: None,
    };
    let (mut results, mut rounds) =
        schedule(listing.len(), ask.workers, ceiling, |at, scratch| {
            listing.get(at).map_or_else(
                || (empty_result(), 0),
                |item| {
                    let result = reconcile(&first, item, scratch);
                    let bars = result.bars;
                    (result, bars)
                },
            )
        })?;
    // THE POOLED LAG, BEFORE EACH DAY: the carried count plus every share's
    // best lags of the earlier days of this month.
    let mut pooled_before = [[0_u32; LAG_BINS]; MAX_DAYS];
    let mut running = carried.pooled;
    for (index, slot) in pooled_before.iter_mut().enumerate() {
        *slot = running;
        for result in &results {
            if let Some(Some(lag)) = result.best.get(index)
                && let Some(count) = running.get_mut(bin(*lag))
            {
                *count += 1;
            }
        }
    }
    let again: Vec<usize> = results
        .iter()
        .enumerate()
        .filter(|(_, r)| r.needs_pooled)
        .map(|(at, _)| at)
        .collect();
    if !again.is_empty() {
        let second = Context {
            pooled: Some(&pooled_before),
            ..first
        };
        let (redone, more) = schedule(again.len(), Some(rounds.last), ceiling, |at, scratch| {
            again.get(at).and_then(|i| listing.get(*i)).map_or_else(
                || (empty_result(), 0),
                |item| {
                    let result = reconcile(&second, item, scratch);
                    let bars = result.bars;
                    (result, bars)
                },
            )
        })?;
        for (at, result) in again.iter().zip(redone) {
            if let Some(slot) = results.get_mut(*at) {
                *slot = result;
            }
        }
        rounds.log.extend(more.log);
    }
    for result in &results {
        note_stock(result, ask.month);
    }
    let summary = summarise(ask.month, &results);
    let body = render(root, ask, commit, &carried, &results, &running, &summary);
    let (file, version) = publish(&ask.out, ask.month, body.as_bytes())?;
    let timing = timing(&results, &rounds, started);
    let line = format!(
        "{} v{version} {} | {summary} | {timing}\n",
        ask.month,
        file.display()
    );
    append_summary(&ask.out, &line)?;
    let failed = results.iter().any(|r| r.read_failed);
    crate::note(
        &telemetry::Event::new(
            if failed {
                telemetry::Level::Error
            } else {
                telemetry::Level::Info
            },
            "cli.feed_reconcile",
            "run published",
        )
        .with("month", ask.month.to_string().as_str())
        .with("file", file.display().to_string().as_str())
        .with("version", version)
        .with("stocks", results.len())
        .with("read_failed", failed),
    );
    Ok(Report {
        file,
        version,
        summary,
        timing,
        failed,
    })
}

fn empty_result() -> StockResult {
    StockResult {
        symbol: String::new(),
        lines: Vec::new(),
        statuses: Vec::new(),
        flags: Vec::new(),
        read_failed: true,
        adjusted: false,
        feeds: (false, false),
        bars: 0,
        micros: 0,
        best: [None; MAX_DAYS],
        needs_pooled: false,
        carry: Carry::default(),
        spans: Vec::new(),
        onset: None,
    }
}

/// The share-month's one event (gate 17's granularity: per instrument-month,
/// never per bar), from the coordinating thread after both passes.
fn note_stock(result: &StockResult, month: YearMonth) {
    let level = if result.read_failed {
        telemetry::Level::Error
    } else {
        telemetry::Level::Info
    };
    let matched = result.statuses.iter().filter(|s| **s == 0).count();
    crate::note(
        &telemetry::Event::new(level, "cli.feed_reconcile", "share-month reconciled")
            .with("symbol", result.symbol.as_str())
            .with("month", month.to_string().as_str())
            .with("days", result.lines.len())
            .with("matched", matched)
            .with("read_failed", result.read_failed)
            .with("bars", result.bars)
            .with("micros", result.micros),
    );
}

/// The one-line summary: shares, stock-days by status, adjusted shares,
/// ex-date days and every flag's count. A pure function of the rows.
pub(crate) fn summarise(month: YearMonth, results: &[StockResult]) -> String {
    let mut statuses = [0_usize; 6];
    let mut flags = [0_usize; FLAG_NAMES.len()];
    let (mut both, mut gdfl, mut zerodha, mut neither, mut unread, mut adjusted) =
        (0, 0, 0, 0, 0, 0);
    for result in results {
        for status in &result.statuses {
            if let Some(count) = statuses.get_mut(*status) {
                *count += 1;
            }
        }
        for row in &result.flags {
            for (bit, count) in flags.iter_mut().enumerate() {
                *count += usize::from(row & (1 << bit) != 0);
            }
        }
        match (result.read_failed, result.feeds) {
            (true, _) => unread += 1,
            (false, (true, true)) => both += 1,
            (false, (true, false)) => gdfl += 1,
            (false, (false, true)) => zerodha += 1,
            (false, (false, false)) => neither += 1,
        }
        adjusted += usize::from(result.adjusted);
    }
    let days: usize = statuses.iter().sum();
    let by_status: Vec<String> = STATUS_NAMES
        .iter()
        .zip(statuses)
        .map(|(name, count)| format!("{name} {count}"))
        .collect();
    let by_flag: Vec<String> = FLAG_NAMES
        .iter()
        .zip(flags)
        .filter(|(_, count)| *count > 0)
        .map(|(name, count)| format!("{name} {count}"))
        .collect();
    let ex_dates = flags.get(2).copied().unwrap_or(0);
    format!(
        "feed-reconcile {month}: {} shares ({both} in both feeds, {gdfl} GDFL only, {zerodha} \
         Zerodha only, {neither} with no day, {unread} unread); {days} stock-days: {}; adjusted \
         shares {adjusted}; ex-date days {ex_dates} (unsourced); flags: {}",
        results.len(),
        by_status.join(", "),
        if by_flag.is_empty() {
            "none".to_owned()
        } else {
            by_flag.join(", ")
        }
    )
}

/// Per share-month wall time and per-bar cost, p50, p99 and max, and the
/// worker rounds. Printed and logged, never written into the file.
fn timing(results: &[StockResult], rounds: &Rounds, started: Instant) -> String {
    let mut micros: Vec<u64> = results.iter().map(|r| r.micros).collect();
    let mut per_bar: Vec<u64> = results
        .iter()
        .filter(|r| r.bars > 0)
        .map(|r| r.micros.saturating_mul(1_000) / r.bars)
        .collect();
    micros.sort_unstable();
    per_bar.sort_unstable();
    let log: Vec<String> = rounds
        .log
        .iter()
        .map(|(workers, items, bars, micros)| format!("{workers}w/{items}i/{bars}b/{micros}us"))
        .collect();
    format!(
        "timing (wall clock, {} ms in all; rounds {}; final workers {}): per share-month p50 {} \
         us, p99 {} us, max {} us (n={}); per bar read and folded p50 {} ns, p99 {} ns, max {} ns \
         (n={})",
        started.elapsed().as_millis(),
        log.join(" "),
        rounds.last,
        rank(&micros, 50),
        rank(&micros, 99),
        micros.last().copied().unwrap_or(0),
        micros.len(),
        rank(&per_bar, 50),
        rank(&per_bar, 99),
        per_bar.last().copied().unwrap_or(0),
        per_bar.len()
    )
}

/// The nearest-rank percentile of sorted values; 0 for none.
pub(crate) fn rank(sorted: &[u64], percentile: usize) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    let at = (sorted.len() * percentile).div_ceil(100).max(1) - 1;
    sorted.get(at).copied().unwrap_or(0)
}

fn render(
    root: &Path,
    ask: &Ask,
    commit: Option<&str>,
    carried: &Carried,
    results: &[StockResult],
    pooled: &[u32; LAG_BINS],
    summary: &str,
) -> String {
    let symbols = ask
        .symbols
        .as_ref()
        .map_or_else(|| "all".to_owned(), |s| s.join(" "));
    let mut body = String::new();
    let _ = writeln!(body, "{FORMAT}");
    let _ = writeln!(
        body,
        "# month {}; store {}; commit {}; symbols {}; previous {}",
        ask.month,
        clean(&root.display().to_string()),
        commit.unwrap_or("unstamped"),
        clean(&symbols),
        carried.note
    );
    let _ = writeln!(
        body,
        "# feeds: gdfl NSE CASH 1s (raw prices; volume = sum of the 1s bars); zerodha NSE CASH \
         1min and 1day (corporate-action adjusted history); decisions on zerodha minutes, gdfl at \
         the seconds level only (D-5170)"
    );
    let _ = writeln!(
        body,
        "# rules (D-5171): grid = gcd of differences between a feed's distinct paisa prints so \
         far this series-month, unknown below two distinct prints (no exchange tick is \
         chartered); allowance of a matched minute = [(2z-s_z-2t)/(2g+s_g) .. (2z+s_z)/(2g-s_g)] \
         with t = 0; factor = the exact pair (z/g) of the lower-median ratio of the matched \
         minutes at the applied lag, with the min and max pairs, empty when none matched; \
         factor_not_constant = the running intersection emptied (emptied_at, emptied_pair, \
         opposing_pair); range = the same intersection over the minutes' ranges \
         [(2z_low-s_z)/(2g_high+s_g) .. (2z_high+s_z)/(2g_low-s_g)], range_not_constant when \
         it empties (D-5176); adjusted = 1 outside the median pair's interval; ex_date = disjoint \
         intervals of consecutive matched days, each day the close intersection or where it \
         emptied the range intersection (D-5177), unsourced (no corporate-action source \
         merged); \
         lag scan = every lag -59..+59 s, equal = the lag's own lower-median ratio inside the \
         minute's interval, shares over zerodha minutes with a known grid, best = most equal, \
         ties to the smallest |L| then the negative; applied lag = the share's most frequent \
         earlier best lag, else every share's earlier days (pooled), else not_aligned; \
         short = (day bar - minute sum) - explained, explained = gdfl volume outside the \
         continuous-phase minutes zerodha printed (phase_at, closing-auction eligibility not \
         read: 15:15-15:29 from 2026-08-03 is unverified), zerodha basis = explained x g/z \
         rounded half up (zerodha's rule UNVERIFIED), judgement needs PREC-3's ENV-1 envelope; \
         minute_sum_allowance = 0 unadjusted, ceil((printed minutes + 1)/2) adjusted or \
         unknown; session cut = gdfl trades in a continuous-phase minute after zerodha's last"
    );
    let _ = writeln!(body, "{COLUMNS}");
    for result in results {
        for line in &result.lines {
            let _ = writeln!(body, "{line}");
        }
    }
    let mut written = std::collections::HashSet::with_capacity(results.len());
    for result in results {
        if !result.symbol.is_empty() && not_a_key(&result.symbol).is_none() {
            let _ = writeln!(body, "{}", carry_line(&result.symbol, &result.carry));
            written.insert(result.symbol.as_str());
        }
    }
    // A SHARE THIS MONTH DID NOT READ KEEPS WHAT IT CARRIED: a month with no
    // file of a share (a suspension, a `--symbols` run) must not erase its lag
    // history or its last interval, so its next month is still judged against
    // its own earlier days. In byte order, so the bytes do not depend on the
    // map.
    let mut kept = std::collections::BTreeSet::new();
    for symbol in carried.stocks.keys() {
        if !written.contains(symbol.as_str()) && not_a_key(symbol).is_none() {
            kept.insert(symbol.as_str());
        }
    }
    for symbol in kept {
        if let Some(carry) = carried.stocks.get(symbol) {
            let _ = writeln!(body, "{}", carry_line(symbol, carry));
        }
    }
    let _ = writeln!(body, "# pooled,lags={}", lags_text(pooled));
    for result in results {
        for span in &result.spans {
            let _ = writeln!(body, "{span}");
        }
        if let Some(onset) = result.onset {
            let _ = writeln!(
                body,
                "# onset,zerodha_session_cut,{},{}",
                result.symbol,
                day_label(onset)
            );
        }
    }
    let _ = writeln!(body, "# summary: {summary}");
    body
}

fn name(month: YearMonth, version: u32) -> String {
    format!("feed-reconcile-{month}.v{version}.csv")
}

/// The newest version file of `month` in `dir`, and its number.
pub(crate) fn newest(dir: &Path, month: YearMonth) -> Result<Option<(PathBuf, u32)>, String> {
    let prefix = format!("feed-reconcile-{month}.v");
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(why) => return Err(format!("listing `{}`: {why}", dir.display())),
    };
    let mut best: Option<u32> = None;
    for entry in entries {
        let entry = entry.map_err(|why| format!("listing `{}`: {why}", dir.display()))?;
        let file = entry.file_name().to_string_lossy().into_owned();
        let version = file
            .strip_prefix(&prefix)
            .and_then(|rest| rest.strip_suffix(".csv"))
            .and_then(|n| n.parse::<u32>().ok());
        if let Some(version) = version
            && best.is_none_or(|b| version > b)
        {
            best = Some(version);
        }
    }
    Ok(best.map(|version| (dir.join(name(month, version)), version)))
}

/// Writes `bytes` as the next version of the month's file: a temporary, made
/// durable, then linked to its final name (a link never replaces a file),
/// the temporary removed and the directory made durable.
pub(crate) fn publish(
    dir: &Path,
    month: YearMonth,
    bytes: &[u8],
) -> Result<(PathBuf, u32), String> {
    let tmp = dir.join(format!(
        ".feed-reconcile-{month}.tmp-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |since| since.as_nanos())
    ));
    let failed = |what: &str, path: &Path, why: std::io::Error| {
        format!("{what} `{}`: {why}", path.display())
    };
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&tmp)
        .map_err(|why| failed("creating", &tmp, why))?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|why| failed("writing", &tmp, why))?;
    drop(file);
    let mut version = newest(dir, month)?.map_or(1, |(_, v)| v.saturating_add(1));
    let published = loop {
        let path = dir.join(name(month, version));
        match std::fs::hard_link(&tmp, &path) {
            Ok(()) => break (path, version),
            Err(why) if why.kind() == std::io::ErrorKind::AlreadyExists => {
                version = version
                    .checked_add(1)
                    .ok_or_else(|| format!("no version number is left for {month}"))?;
            }
            Err(why) => {
                let _ = std::fs::remove_file(&tmp);
                return Err(failed("linking", &path, why));
            }
        }
    };
    std::fs::remove_file(&tmp).map_err(|why| failed("removing the temporary", &tmp, why))?;
    std::fs::File::open(dir)
        .and_then(|handle| handle.sync_all())
        .map_err(|why| failed("syncing", dir, why))?;
    Ok(published)
}

/// Appends one line to the summary log: one `write` on a file opened to
/// append, then made durable.
fn append_summary(dir: &Path, line: &str) -> Result<(), String> {
    let path = dir.join("feed-reconcile.summary.log");
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(&path)
        .map_err(|why| format!("opening `{}`: {why}", path.display()))?;
    file.write_all(line.as_bytes())
        .and_then(|()| file.sync_all())
        .map_err(|why| format!("appending to `{}`: {why}", path.display()))
}

// ---------------------------------------------------------------------------
// The reader every consumer uses (FIB-10, OF-2): one authority
// ---------------------------------------------------------------------------

/// A stock-day's price factor, exact: the paisa pairs, the day's two
/// intersections and the lag the day was matched at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FactorPair {
    /// `(zerodha_close, gdfl_close)` of the lower-median matched minute.
    pub median: (i64, i64),
    /// The pair with the smallest ratio.
    pub min: (i64, i64),
    /// The pair with the largest ratio.
    pub max: (i64, i64),
    /// The day's intersection of the CLOSE allowances (D-5171 item 3), or
    /// `None` when it emptied (`factor_not_constant`).
    pub interval: Option<(Frac, Frac)>,
    /// The day's intersection of the RANGE allowances (D-5176), or `None`
    /// when it emptied (`range_not_constant`).
    pub range: Option<(Frac, Frac)>,
    /// The lag, in seconds, the day was matched at.
    pub lag: i8,
}

impl FactorPair {
    /// A Zerodha-basis paisa level on GDFL's raw basis, exactly:
    /// `level × g / z`, never rounded here (the consumer rounds at its own
    /// grid).
    #[must_use]
    pub fn to_gdfl(&self, zerodha_paisa: i64) -> Frac {
        Frac {
            num: i128::from(zerodha_paisa) * i128::from(self.median.1),
            den: i128::from(self.median.0),
        }
    }
}

fn span_text(span: Option<(Frac, Frac)>) -> String {
    span.map_or_else(
        || "empty".to_owned(),
        |(low, high)| format!("[{low} .. {high}]"),
    )
}

impl std::fmt::Display for FactorPair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "median {}/{} min {}/{} max {}/{} interval {} range {} lag {}",
            self.median.0,
            self.median.1,
            self.min.0,
            self.min.1,
            self.max.0,
            self.max.1,
            span_text(self.interval),
            span_text(self.range),
            self.lag
        )
    }
}

/// Why a stock-day has no factor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FactorRefusal {
    /// Both feeds held the day and no minute matched at the applied lag.
    NoOverlap,
    /// The day's close allowances do not intersect: the minute that emptied
    /// the intersection, and the day's pairs with its RANGE intersection, so
    /// a consumer decides by its own stated rule without recomputing.
    FactorNotConstant {
        /// `HH:MM` IST.
        minute: String,
        /// The day's pairs; `interval` is `None`, `range` says whether the
        /// minutes' ranges still hold one basis (D-5176).
        pair: Box<FactorPair>,
    },
    /// A file of the share-month could not be read; why.
    ReadFailed(String),
    /// The file holds no row for the share or the day, or another month.
    NotChecked,
    /// No lag could be applied: no earlier day of any share.
    NotAligned,
    /// Only one feed held the day (`gdfl_only` or `zerodha_only`).
    OneFeed(String),
}

impl std::fmt::Display for FactorRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoOverlap => write!(f, "no_overlap"),
            Self::FactorNotConstant { minute, pair } => {
                write!(f, "factor_not_constant at {minute} ({pair})")
            }
            Self::ReadFailed(why) => write!(f, "read_failed:{why}"),
            Self::NotChecked => write!(f, "not_checked"),
            Self::NotAligned => write!(f, "not_aligned"),
            Self::OneFeed(which) => write!(f, "one_feed:{which}"),
        }
    }
}

/// The newest version file of one month, indexed by (share slot × day of
/// month) at a fixed stride: a lookup is one hash of the symbol and one
/// multiply-add.
#[derive(Debug)]
pub struct FactorIndex {
    month: YearMonth,
    /// The file the index was built from.
    pub file: PathBuf,
    /// BLAKE3 of that file's bytes.
    pub digest: [u8; 32],
    slots: HashMap<String, usize>,
    cells: Vec<Option<Result<FactorPair, FactorRefusal>>>,
}

/// The newest version file of `month` in `dir` and the BLAKE3 of its bytes,
/// for a run identity that names exactly which factors it used.
///
/// # Errors
///
/// No version file, or one that cannot be read.
pub fn file_digest(dir: &Path, month: YearMonth) -> Result<(PathBuf, [u8; 32]), String> {
    let (file, _) = newest(dir, month)?.ok_or_else(|| {
        format!(
            "`{}` holds no feed-reconcile file for {month}",
            dir.display()
        )
    })?;
    let bytes =
        std::fs::read(&file).map_err(|why| format!("reading `{}`: {why}", file.display()))?;
    Ok((file, brutex_core::blake3::hash(&bytes)))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut text, byte| {
        let _ = write!(text, "{byte:02x}");
        text
    })
}

/// One cell of the index from one row.
fn cell_of(
    fields: &[&str],
    at: &HashMap<&str, usize>,
) -> Result<Result<FactorPair, FactorRefusal>, String> {
    let get = |name: &str| -> Result<&str, String> {
        at.get(name)
            .and_then(|i| fields.get(*i))
            .copied()
            .ok_or_else(|| format!("column `{name}` is missing"))
    };
    let int = |name: &str| -> Result<i64, String> {
        let text = get(name)?;
        text.parse()
            .map_err(|_| format!("column `{name}` holds `{text}`"))
    };
    // An empty pair of bounds is an intersection that emptied.
    let span = |low: &str, high: &str| -> Result<Option<(Frac, Frac)>, String> {
        match (get(low)?, get(high)?) {
            ("", "") => Ok(None),
            (l, h) => Ok(Some((parse_frac(l)?, parse_frac(h)?))),
        }
    };
    let status = get("status")?;
    if let Some(why) = status.strip_prefix("read_failed:") {
        return Ok(Err(FactorRefusal::ReadFailed(why.to_owned())));
    }
    Ok(match status {
        "gdfl_only" | "zerodha_only" => Err(FactorRefusal::OneFeed(status.to_owned())),
        "no_overlap" => Err(FactorRefusal::NoOverlap),
        "not_aligned" => Err(FactorRefusal::NotAligned),
        "matched" => {
            let pair = FactorPair {
                median: (int("median_z")?, int("median_g")?),
                min: (int("min_z")?, int("min_g")?),
                max: (int("max_z")?, int("max_g")?),
                interval: span("interval_lo", "interval_hi")?,
                range: span("range_lo", "range_hi")?,
                lag: get("applied_lag")?
                    .parse()
                    .map_err(|_| "column `applied_lag`".to_owned())?,
            };
            let emptied = get("emptied_at")?;
            match (emptied, pair.interval) {
                ("", Some(_)) => Ok(pair),
                ("", None) => {
                    return Err("a matched row with no interval names no minute".to_owned());
                }
                (minute, _) => Err(FactorRefusal::FactorNotConstant {
                    minute: minute.to_owned(),
                    pair: Box::new(pair),
                }),
            }
        }
        other => return Err(format!("status `{other}` is not one this format writes")),
    })
}

impl FactorIndex {
    /// Loads the newest version file of `month` from `dir`. One pass over
    /// its rows; every later lookup is O(1).
    ///
    /// # Errors
    ///
    /// No file, a file of another format, or a row that does not parse.
    pub fn load(dir: &Path, month: YearMonth) -> Result<Self, String> {
        let (file, digest) = file_digest(dir, month)?;
        let text = std::fs::read_to_string(&file)
            .map_err(|why| format!("reading `{}`: {why}", file.display()))?;
        let mut lines = text.lines();
        if lines.next() != Some(FORMAT) {
            return Err(format!(
                "`{}` is not `{FORMAT}`; an older format is never read as this one",
                file.display()
            ));
        }
        let header = lines
            .find(|line| !line.starts_with('#'))
            .ok_or_else(|| format!("`{}` has no column line", file.display()))?;
        let at: HashMap<&str, usize> = header.split(',').enumerate().map(|(i, n)| (n, i)).collect();
        // SIZED FROM THE FILE ITSELF: one carry line per share it read.
        let shares = text
            .lines()
            .filter(|line| line.starts_with("# carry,"))
            .count();
        let mut index = Self {
            month,
            file,
            digest,
            slots: HashMap::with_capacity(shares),
            cells: Vec::with_capacity(shares * MAX_DAYS),
        };
        let symbol_at = at.get("symbol").copied().unwrap_or(0);
        let day_at = at.get("day").copied().unwrap_or(1);
        for line in lines.take_while(|line| !line.starts_with('#')) {
            let fields: Vec<&str> = line.split(',').collect();
            let symbol = fields.get(symbol_at).copied().unwrap_or_default();
            let day = fields.get(day_at).copied().unwrap_or_default();
            let next = index.slots.len();
            let slot = *index.slots.entry(symbol.to_owned()).or_insert(next);
            if slot == next {
                index.cells.resize((next + 1) * MAX_DAYS, None);
            }
            let cell = cell_of(&fields, &at)?;
            if day == "-" {
                for d in 0..MAX_DAYS {
                    if let Some(place) = index.cells.get_mut(slot * MAX_DAYS + d) {
                        *place = Some(cell.clone());
                    }
                }
                continue;
            }
            let of_month = usize::from(parse_day(day)?.day());
            if let Some(place) = index.cells.get_mut(slot * MAX_DAYS + of_month - 1) {
                *place = Some(cell);
            }
        }
        Ok(index)
    }

    /// The factor of `symbol` on `day`, or why there is none. O(1): one hash
    /// and one fixed-stride index.
    ///
    /// # Errors
    ///
    /// [`FactorRefusal`], by name.
    pub fn factor_of(&self, symbol: &str, day: Day) -> Result<FactorPair, FactorRefusal> {
        if day.year() != self.month.year() || day.month() != self.month.month() {
            return Err(FactorRefusal::NotChecked);
        }
        let slot = self.slots.get(symbol).ok_or(FactorRefusal::NotChecked)?;
        let at = slot * MAX_DAYS + usize::from(day.day()).saturating_sub(1);
        self.cells
            .get(at)
            .cloned()
            .flatten()
            .unwrap_or(Err(FactorRefusal::NotChecked))
    }
}

#[cfg(test)]
#[path = "feed_reconcile_tests.rs"]
mod tests;

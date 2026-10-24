//! `cli srt-build` — each named month's `.srt` sort index written whole, as
//! one run, through the store's rebuild door. D-3349, WP-39; DB-N14.
//!
//! # Usage
//!
//! ```text
//! cli srt-build VENDOR EXCHANGE/SEGMENT/SYMBOL[/CONTRACT|/*] RUNG FROM TO [--dry-run]
//! cli srt-build gdfl NSE/INDEX/NIFTY 1s 2018-09 2026-09
//! cli srt-build gdfl NSE/FNO/NIFTY/2019-03-28-1150000-CE 1s 2019-01 2019-03
//! cli srt-build gdfl NSE/FNO/NIFTY/* 1s 2018-09 2020-09
//! ```
//!
//! `FROM` and `TO` are `YYYY-MM`, inclusive. The instrument is the store's own
//! path segments: a spot series, one contract, or `*` for every contract
//! directory under the symbol, walked in name order.
//!
//! # Why it exists
//!
//! `/bars/window.json` ordered by a price, the volume or the open interest
//! reads only the rows no `.srt` run orders (D-3349). A writer keeps every
//! month it writes indexed from D-3349 on; the months written before it have
//! no `.srt` until a writer opens them again, which an import that is done
//! with a month never does. This verb is that step for named months, and
//! nothing else, through `store::file::BarFile::sort_index_existing`:
//!
//! * a month that is not held is reported and NOTHING is created for it;
//! * the bar file, its checksums and its time index are opened read-only and
//!   never change;
//! * a month another process holds — a writer, or a reader mid-request — is
//!   refused by the store's own exclusive lock, named, and left as it was;
//! * a month already held as one run over every committed row is kept, after
//!   its headers are read, so a rerun is safe and resumes where one stopped;
//! * anything else is written whole into a NEW file renamed over the old
//!   name — never edited in place — and the store writes one `store.srt`
//!   line naming the month and why;
//! * each month built is then reopened through the READ side and checked to
//!   cover every committed row in one run.
//!
//! # The small-month rule (D-4933)
//!
//! A month that holds fewer than 1,024 bars
//! (`store::time_index::INDEX_MIN_ROWS`) and has no `.srt` is given none, by
//! the verb as by the writer: it is reported `small`, nothing is written, and
//! it counts as clean, because a sorted page orders its at most 1,023 rows
//! itself. The commit that brings such a month to the threshold writes its
//! index whole. A `.srt` already beside a small month is kept or rebuilt as
//! any other.
//!
//! `--dry-run` opens every month read-only, says which are indexed and which
//! would be built, and writes nothing.
//!
//! A `*` names what it does not walk: a symbolic link under the symbol is
//! never followed and nothing is built through one (a read-only shadow links
//! its contracts into the store it shadows), and a name that is not a
//! contract directory is not a series; the receipt counts both. A `*` that
//! finds no contract directory, or no held month in the span, says so and is
//! not a clean exit.
//!
//! # Cost
//!
//! Building one month reads every committed bar once through the verified
//! read path and sorts the six columns: `O(n log n)` for `n` bars, about 48
//! bytes a bar on disk (twelve for a column whose span passes four bytes).
//! The verb holds one month's lock at a time.
//!
//! # Exit
//!
//! [`OK`] only when every month in the span is held and a reader finds it
//! indexed (or, with `--dry-run`, held and readable); [`FAILED`] otherwise, so
//! a script cannot read a partial build — or a `*` that found nothing to
//! build — as a clean one; [`MISUSED`] for
//! arguments that do not name a series and a span, before anything is opened.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use brutex_core::instrument::Contract;
use brutex_core::vendor::Vendor;
use store::file::{BarFile, Resorted, StoreError};
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};
use store::sort_index::{Shape, SortIndex};
use store::time_index::{INDEX_MIN_ROWS, below_threshold};

use crate::{FAILED, MISUSED, OK};

/// The verb's shape, printed with every misuse.
pub(crate) const USAGE: &str = "cli srt-build VENDOR EXCHANGE/SEGMENT/SYMBOL[/CONTRACT|/*] RUNG \
     FROM TO [--dry-run]  (FROM and TO are YYYY-MM, inclusive; e.g. gdfl NSE/INDEX/NIFTY 1s \
     2018-09 2026-09)";

/// The most contracts one `*` walks: past it the run is refused before a
/// month is opened, rather than cut short.
const MAX_CONTRACTS: usize = 100_000;

/// Which contracts the arguments name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Which {
    /// The spot series.
    Spot,
    /// One contract.
    One(Contract),
    /// Every contract directory under the symbol.
    Every,
}

/// What the arguments name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Ask {
    pub(crate) vendor: Vendor,
    pub(crate) exchange: String,
    pub(crate) segment: String,
    pub(crate) symbol: String,
    pub(crate) which: Which,
    pub(crate) rung: Timeframe,
    pub(crate) months: Vec<YearMonth>,
    pub(crate) dry_run: bool,
}

impl Ask {
    /// The store path of one month of one series.
    pub(crate) fn path(
        &self,
        contract: Option<Contract>,
        month: YearMonth,
    ) -> Result<StorePath<'_>, String> {
        StorePath::new(PathParts {
            vendor: self.vendor,
            exchange: &self.exchange,
            segment: &self.segment,
            symbol: &self.symbol,
            contract,
            timeframe: self.rung,
            month,
            file: FileKind::Bars,
        })
        .map_err(|why| {
            format!(
                "{}/{}/{} {} {month} is not a store path: {why}",
                crate::stored::clipped(&self.exchange),
                crate::stored::clipped(&self.segment),
                crate::stored::clipped(&self.symbol),
                self.rung.as_str()
            )
        })
    }

    /// The header's symbol id, as every writer folds it.
    pub(crate) fn symbol_id(&self) -> u32 {
        symbol_id(&self.symbol)
    }

    /// The series the arguments name: the spot series, one contract, or
    /// every contract directory under the symbol, in name order.
    ///
    /// A `*` walk NAMES WHAT IT DOES NOT WALK: a symbolic link is never
    /// followed (the store's writer follows none below the root, so nothing
    /// could be built through it) and a name that is not a contract
    /// directory is not a series; both are counted for the receipt. A walk
    /// that finds no contract directory is refused, never an empty success.
    pub(crate) fn series(&self, root: &Path) -> Result<Walk, String> {
        self.series_within(root, MAX_CONTRACTS)
    }

    /// [`Self::series`] with the walk's bound named: past `bound` contract
    /// directories a `*` is refused before a month is opened. Every caller
    /// passes [`MAX_CONTRACTS`] through [`Self::series`]; the bound is a
    /// parameter only so a test reaches it with a handful of directories
    /// rather than 100,001 (`docs/06-limits.md`, "The cli import verbs").
    fn series_within(&self, root: &Path, bound: usize) -> Result<Walk, String> {
        match &self.which {
            Which::Spot => Ok(Walk::of(vec![None])),
            Which::One(contract) => Ok(Walk::of(vec![Some(*contract)])),
            Which::Every => {
                let dir = root
                    .join("bars")
                    .join(self.vendor.as_str())
                    .join(&self.exchange)
                    .join(&self.segment)
                    .join(&self.symbol);
                let entries = std::fs::read_dir(&dir)
                    .map_err(|why| format!("cannot list {}: {why}", dir.display()))?;
                let mut names = Vec::new();
                let (mut links, mut others) = (0_usize, 0_usize);
                for entry in entries {
                    let entry =
                        entry.map_err(|why| format!("cannot list {}: {why}", dir.display()))?;
                    // `file_type` does not follow a link, so a linked
                    // contract is seen as the link it is.
                    let kind = entry
                        .file_type()
                        .map_err(|why| format!("cannot list {}: {why}", dir.display()))?;
                    let name = entry.file_name();
                    let contract = name.to_str().and_then(Contract::parse);
                    if kind.is_symlink() {
                        links = links.saturating_add(1);
                    } else if let (true, Some(contract)) = (kind.is_dir(), contract) {
                        names.push((name.to_string_lossy().into_owned(), contract));
                    } else {
                        others = others.saturating_add(1);
                    }
                    if names.len() > bound {
                        return Err(format!(
                            "{} holds more than {bound} contract directories; name fewer",
                            dir.display()
                        ));
                    }
                }
                if names.is_empty() {
                    return Err(format!(
                        "{} holds no contract directory ({links} symbolic link(s), never \
                         followed, and {others} other entr(ies) that are not contract \
                         directories)",
                        dir.display()
                    ));
                }
                names.sort_unstable_by(|a, b| a.0.cmp(&b.0));
                Ok(Walk {
                    series: names
                        .into_iter()
                        .map(|(_, contract)| Some(contract))
                        .collect(),
                    links,
                    others,
                })
            }
        }
    }
}

/// The series a run walks, and what a `*` found beside them and left alone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Walk {
    /// The series, in name order.
    pub(crate) series: Vec<Option<Contract>>,
    /// Symbolic links under the symbol: never followed, never built through.
    pub(crate) links: usize,
    /// Entries that are not a contract directory by name or by kind.
    pub(crate) others: usize,
}

impl Walk {
    /// Named series: nothing was listed, so nothing was left alone.
    const fn of(series: Vec<Option<Contract>>) -> Self {
        Self {
            series,
            links: 0,
            others: 0,
        }
    }
}

/// A feed word, as the store names vendors.
pub(crate) fn vendor(word: &str) -> Result<Vendor, String> {
    Vendor::ALL
        .into_iter()
        .find(|vendor| vendor.as_str() == word)
        .ok_or_else(|| {
            let known: Vec<&str> = Vendor::ALL.iter().map(|vendor| vendor.as_str()).collect();
            format!(
                "`{}` is not a feed this build knows: {}",
                crate::stored::clipped(word),
                known.join(", ")
            )
        })
}

/// A `YYYY-MM` month, digits and widths exact.
pub(crate) fn month(label: &str, word: &str) -> Result<(u16, u8), String> {
    let digits = |part: &str, width: usize| {
        (part.len() == width && part.bytes().all(|b| b.is_ascii_digit()))
            .then(|| part.parse::<u16>().ok())
            .flatten()
    };
    let parsed = word
        .split_once('-')
        .and_then(|(y, m)| digits(y, 4).zip(digits(m, 2)));
    let Some((year, number)) = parsed else {
        return Err(format!(
            "{label} `{}` is not a YYYY-MM month",
            crate::stored::clipped(word)
        ));
    };
    let number = u8::try_from(number).unwrap_or(u8::MAX);
    YearMonth::new(year, number)
        .map(|_| (year, number))
        .map_err(|why| format!("{label} `{}`: {why}", crate::stored::clipped(word)))
}

/// The arguments, refused by name before anything is opened.
fn ask(arguments: &[&str]) -> Result<Ask, String> {
    ask_for("srt-build", arguments)
}

/// [`ask`] for `verb`, a store verb of the same shape (`msum-build`, R3-06):
/// one parser, so the two verbs name a series and a span the same way.
pub(crate) fn ask_for(verb: &str, arguments: &[&str]) -> Result<Ask, String> {
    let (positional, dry_run) = match arguments {
        [head @ .., "--dry-run"] => (head, true),
        all => (all, false),
    };
    let [feed, instrument, rung, from, to] = positional else {
        return Err(format!(
            "{verb} takes five arguments and an optional --dry-run; it was given {}",
            arguments.len()
        ));
    };
    let vendor = vendor(feed)?;
    let parts: Vec<&str> = instrument.split('/').collect();
    let (exchange, segment, symbol, which) = match parts.as_slice() {
        [exchange, segment, symbol] => (*exchange, *segment, *symbol, Which::Spot),
        [exchange, segment, symbol, "*"] => (*exchange, *segment, *symbol, Which::Every),
        [exchange, segment, symbol, contract] => (
            *exchange,
            *segment,
            *symbol,
            Which::One(Contract::parse(contract).ok_or_else(|| {
                format!(
                    "`{}` is not a contract this store can name",
                    crate::stored::clipped(contract)
                )
            })?),
        ),
        _ => {
            return Err(format!(
                "`{}` is not EXCHANGE/SEGMENT/SYMBOL[/CONTRACT|/*], the store's own path \
                 segments (e.g. NSE/INDEX/NIFTY)",
                crate::stored::clipped(instrument)
            ));
        }
    };
    if [exchange, segment, symbol]
        .iter()
        .any(|part| part.is_empty())
    {
        return Err(format!(
            "`{}` has an empty segment",
            crate::stored::clipped(instrument)
        ));
    }
    let rung = crate::stored::rung(rung)?;
    let (from, to) = (month("FROM", from)?, month("TO", to)?);
    let months = crate::stored::months_between(from, to)?
        .into_iter()
        .filter_map(|(year, number)| YearMonth::new(year, number).ok())
        .collect();
    let ask = Ask {
        vendor,
        exchange: exchange.to_owned(),
        segment: segment.to_owned(),
        symbol: symbol.to_owned(),
        which,
        rung,
        months,
        dry_run,
    };
    // EVERY PATH IS PROVEN BEFORE THE FIRST MONTH IS OPENED.
    let probe = match &ask.which {
        Which::One(contract) => Some(*contract),
        Which::Spot | Which::Every => None,
    };
    for &month in &ask.months {
        ask.path(probe, month)?;
    }
    Ok(ask)
}

/// What happened to one month.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Line {
    /// One run already ordered every row: nothing written.
    Kept { n_valid: u64 },
    /// Written whole, for the reason the store named.
    Built { n_valid: u64, because: String },
    /// No run can order these bars; nothing written.
    Unsortable { n_valid: u64, why: String },
    /// No committed rows to order.
    Empty,
    /// The step finished and a reader does not find one run over every row.
    NotIndexed { n_valid: u64, reader: String },
    /// The month is not in the store; nothing was created.
    NotHeld,
    /// The store refused the month, in its own words.
    Refused { why: String },
    /// `--dry-run`: one run orders every row.
    Indexed { n_valid: u64 },
    /// `--dry-run`: it would be built, for this reason.
    WouldBuild { n_valid: u64, why: String },
    /// Under the small-month threshold with no `.srt`: none is made, nothing
    /// was written, and a sorted page orders its rows itself (D-4933).
    Small { n_valid: u64 },
}

impl Line {
    /// Whether this month ends the run indexed (or, dry, held and readable).
    const fn clean(&self) -> bool {
        matches!(
            self,
            Self::Kept { .. }
                | Self::Built { .. }
                | Self::Empty
                | Self::Indexed { .. }
                | Self::WouldBuild { .. }
                | Self::Small { .. }
        )
    }
}

/// How a reader now sees the month's index: one run over every row, or why
/// not, and the `.srt` name it looked at.
///
/// `body` also reads the run's body against the checksum its header holds
/// (one read of the file): a reader that reaches a damaged fence table or
/// block refuses the month, so a receipt that looked at headers alone would
/// call `indexed` an index every reader sets aside. The build step passes
/// `false` only for a month the store has just kept, which made the same
/// check under the month's lock.
fn reader_view(
    root: &Path,
    path: StorePath<'_>,
    symbol_id: u32,
    body: bool,
) -> Result<(u64, Option<String>, PathBuf), StoreError> {
    // THE GENERATION EVERY READER OPENS (D-3350): `open_existing` resolves
    // it, and the `.srt` a reader reads sits beside the `.bin` it opened.
    let file = BarFile::open_existing(root, path, symbol_id)?;
    let header = file.header();
    let shape = Shape::of(path.month(), header.timeframe_secs, header.symbol_id);
    let srt = file.path().with_extension("srt");
    let verdict = match SortIndex::open(&srt, &shape) {
        // BYTES PAST THE LAST RUN ARE NOT ONE WHOLE RUN: the build step writes
        // such a month again, so a dry run that called it `indexed` promised
        // a real run that then built it.
        Ok(index) if index.torn().is_some() => index.torn().map(|(at, why)| {
            format!("bytes past its last run are not a run (at byte {at}: {why})")
        }),
        Ok(index) if index.runs().len() == 1 && index.coverage() == header.n_valid => {
            // BOUND TO THESE BARS: one run over every row is checked against
            // the header's own first and last stamps, which a reader checks
            // too, so the receipt never calls a stale index indexed.
            store::sort_index::bind(
                index.runs(),
                header.n_valid,
                header.first_ts_micros,
                header.last_ts_micros,
                |row| {
                    file.read_record(row)
                        .map(|bar| bar.ts_micros)
                        .map_err(|why| why.to_string())
                },
            )
            .and_then(|_| if body { index.verify() } else { Ok(()) })
            .err()
            .map(|why| why.to_string())
        }
        Ok(index) => Some(format!(
            "{} runs ordering rows 0..{} of {}",
            index.runs().len(),
            index.coverage(),
            header.n_valid
        )),
        Err(why) => Some(why.to_string()),
    };
    Ok((header.n_valid, verdict, srt))
}

/// The read side [`build`] checks a month against after its step: the
/// arguments of [`reader_view`], and its answer.
type ReadBack<'a> = &'a dyn Fn(
    &Path,
    StorePath<'_>,
    u32,
    bool,
) -> Result<(u64, Option<String>, PathBuf), StoreError>;

/// One month, built or confirmed, then checked from the read side.
fn build(root: &Path, path: StorePath<'_>, symbol_id: u32) -> Line {
    build_reading(root, path, symbol_id, &reader_view)
}

/// [`build`] with its read-back named. Every caller reads back through
/// [`reader_view`]; the read-back is a parameter only so a test can hand the
/// step a reader that disagrees with it, which no deterministic store state
/// reaches (the store's step and its reader decide from one header under the
/// month's lock), and see which body check the step asked for
/// (`docs/06-limits.md`, "The cli import verbs").
fn build_reading(
    root: &Path,
    path: StorePath<'_>,
    symbol_id: u32,
    read_back: ReadBack<'_>,
) -> Line {
    let done = match BarFile::sort_index_existing(root, path, symbol_id) {
        Ok(done) => done,
        Err(StoreError::Missing { .. }) => return Line::NotHeld,
        Err(StoreError::Locked { path }) => {
            return Line::Refused {
                why: format!(
                    "{} is held by another process (a writer, or a reader mid-request) and is \
                     not waited for",
                    path.display()
                ),
            };
        }
        Err(why) => {
            return Line::Refused {
                why: why.to_string(),
            };
        }
    };
    let n_valid = done.n_valid;
    // A KEPT MONTH'S BODY WAS JUST CHECKED BY THE STORE, under the lock; a
    // built one is read back whole here.
    let read_back_body = !matches!(done.outcome, Resorted::Kept);
    let line = match done.outcome {
        Resorted::Kept => Line::Kept { n_valid },
        Resorted::Built(because) => Line::Built { n_valid, because },
        Resorted::Empty => return Line::Empty,
        Resorted::Unsortable(why) => return Line::Unsortable { n_valid, why },
        // THE STORE'S DOOR FOUND NO `.srt` AND MADE NONE, under the month's
        // lock; there is nothing for a reader to find, so nothing to read
        // back (D-4933).
        Resorted::BelowThreshold => return Line::Small { n_valid },
        other => {
            return Line::Refused {
                why: format!("the store reported an outcome this build does not know: {other:?}"),
            };
        }
    };
    match read_back(root, path, symbol_id, read_back_body) {
        Ok((_, None, _)) => line,
        Ok((_, Some(reader), _)) => Line::NotIndexed { n_valid, reader },
        Err(why) => Line::Refused {
            why: format!("the step finished and the read-back was refused: {why}"),
        },
    }
}

/// Why the build step would refuse this month for what sits at `name` — the
/// month's `.srt` or its staging name — when that is there and is not a
/// regular file: a link, a directory, a pipe. The store's door leaves such a
/// name as it is and refuses the month, so a dry run says the same.
///
/// A name that could not be LOOKED AT is refused too, never read as absent:
/// `store::sort_index_writer` refuses every `lstat` failure but `NotFound`
/// on both names, so a dry run that said `would build` there would say
/// something other than what the build step does. This was `.ok()?`, which
/// turned an I/O error into "nothing is there" (cloud-attack-law-20261016).
fn name_refused(name: &Path) -> Option<String> {
    match std::fs::symlink_metadata(name) {
        Ok(meta) => (!meta.is_file()).then(|| {
            format!(
                "{} is not a regular file; the build step leaves it as it is and refuses the month",
                name.display()
            )
        }),
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => None,
        Err(why) => Some(format!(
            "{} could not be inspected: {why}; the build step refuses the month",
            name.display()
        )),
    }
}

/// One month as a reader sees it, writing nothing — and saying what the
/// build step WOULD do, never something else: a month whose `.srt` name or
/// `.new.srt` staging name is there and is not a regular file is `REFUSED`
/// here as it is there (the store's door refuses it whether it would keep
/// the month or build it).
fn inspect(root: &Path, path: StorePath<'_>, symbol_id: u32) -> Line {
    match reader_view(root, path, symbol_id, true) {
        Ok((0, _, _)) => Line::Empty,
        Ok((n_valid, verdict, srt)) => {
            let refused =
                name_refused(&srt).or_else(|| name_refused(&srt.with_extension("new.srt")));
            // SMALL ONLY WHEN THE NAME IS NOT THERE AT ALL, as the store's
            // door decides it: a damaged `.srt` beside a small month is still
            // built again (D-4933).
            let absent = matches!(
                std::fs::symlink_metadata(&srt),
                Err(why) if why.kind() == std::io::ErrorKind::NotFound
            );
            match (refused, verdict) {
                (Some(why), _) => Line::Refused { why },
                (None, Some(_)) if absent && below_threshold(n_valid) => Line::Small { n_valid },
                (None, None) => Line::Indexed { n_valid },
                (None, Some(why)) => Line::WouldBuild { n_valid, why },
            }
        }
        Err(StoreError::Missing { .. }) => Line::NotHeld,
        Err(why) => Line::Refused {
            why: why.to_string(),
        },
    }
}

/// The receipt line for one month.
fn render_line(out: &mut String, label: &str, line: &Line) {
    let _ = match line {
        Line::Kept { n_valid } => writeln!(
            out,
            "  {label}  kept          {n_valid:>9} bars  already one run, its body whole; nothing written"
        ),
        Line::Built { n_valid, because } => writeln!(
            out,
            "  {label}  built         {n_valid:>9} bars  because {because}; a reader now finds one run"
        ),
        Line::Unsortable { n_valid, why } => writeln!(
            out,
            "  {label}  UNSORTABLE    {n_valid:>9} bars  nothing written; a sort reads these rows: {why}"
        ),
        Line::Empty => writeln!(out, "  {label}  empty         no committed bar to order"),
        Line::NotIndexed { n_valid, reader } => writeln!(
            out,
            "  {label}  NOT INDEXED   {n_valid:>9} bars  the step finished and a reader finds {reader}"
        ),
        Line::NotHeld => writeln!(
            out,
            "  {label}  NOT HELD      no such month; nothing was created"
        ),
        Line::Refused { why } => writeln!(
            out,
            "  {label}  REFUSED       {why}; nothing was written to it"
        ),
        Line::Indexed { n_valid } => writeln!(out, "  {label}  indexed       {n_valid:>9} bars"),
        Line::WouldBuild { n_valid, why } => {
            writeln!(out, "  {label}  would build   {n_valid:>9} bars  {why}")
        }
        Line::Small { n_valid } => writeln!(
            out,
            "  {label}  small         {n_valid:>9} bars  under {INDEX_MIN_ROWS} bars: no .srt by the small-month rule (D-4933); a sort orders these rows itself; nothing written"
        ),
    };
}

/// Runs the verb against the store root `store` names.
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
                "SRT BUILD REFUSED: {why}. Nothing was read or written."
            );
            return FAILED;
        }
    };
    let walk = match ask.series(&root) {
        Ok(walk) => walk,
        Err(why) => {
            let _ = writeln!(out, "SRT BUILD REFUSED: {why}. Nothing was written.");
            return FAILED;
        }
    };
    let symbol_id = ask.symbol_id();
    let (first, last) = (
        ask.months
            .first()
            .map(ToString::to_string)
            .unwrap_or_default(),
        ask.months
            .last()
            .map(ToString::to_string)
            .unwrap_or_default(),
    );
    let _ = writeln!(
        out,
        "SRT BUILD{} — {} {}/{}/{} {}, {first}..{last}: {} series × {} month(s) under {}",
        if ask.dry_run {
            " (dry run: nothing is written)"
        } else {
            ""
        },
        ask.vendor.as_str(),
        ask.exchange,
        ask.segment,
        ask.symbol,
        ask.rung.as_str(),
        walk.series.len(),
        ask.months.len(),
        root.display()
    );
    // WHAT THE WALK LEFT ALONE IS SAID, never skipped silently.
    if walk.links > 0 || walk.others > 0 {
        let _ = writeln!(
            out,
            "  NOT WALKED: {} symbolic link(s) — never followed, nothing is built through a \
             link — and {} other entr(ies) that are not contract directories",
            walk.links, walk.others
        );
    }
    let mut receipt = Vec::with_capacity(
        walk.series
            .len()
            .saturating_mul(ask.months.len())
            .min(4_096),
    );
    for &contract in &walk.series {
        let name = contract.map_or_else(String::new, |contract| format!("{contract} "));
        for &month in &ask.months {
            let line = match ask.path(contract, month) {
                Ok(path) if ask.dry_run => inspect(&root, path, symbol_id),
                Ok(path) => build(&root, path, symbol_id),
                Err(why) => Line::Refused { why },
            };
            // A `*` WALK NAMES ONLY WHAT IT FOUND: a contract's life is a
            // few months, so the others are not held by design.
            if !(ask.which == Which::Every && line == Line::NotHeld) {
                render_line(out, &format!("{name}{month}"), &line);
                receipt.push(line);
            }
        }
    }
    // A `*` THAT FOUND NO HELD MONTH PROVED NOTHING: it is said, and it is
    // not a clean exit (only a `*` drops its not-held lines, so only a `*`
    // can end with none).
    if receipt.is_empty() {
        let _ = writeln!(
            out,
            "NO MONTH HELD: none of the {} contract(s) holds a month in {first}..{last}. \
             Nothing was indexed; nothing was written.",
            walk.series.len()
        );
        return FAILED;
    }
    summed(out, &receipt, ask.dry_run)
}

/// The receipt's last line and the exit: [`OK`] only when every month named
/// ended indexed (or, dry, held and readable).
fn summed(out: &mut String, receipt: &[Line], dry_run: bool) -> u8 {
    let count = |want: fn(&Line) -> bool| receipt.iter().filter(|line| want(line)).count();
    let clean = receipt.iter().all(Line::clean);
    if dry_run {
        let _ = writeln!(
            out,
            "Indexed {}, would build {}, empty {}, small {}, not held {}, refused {}. Nothing was written.",
            count(|line| matches!(line, Line::Indexed { .. })),
            count(|line| matches!(line, Line::WouldBuild { .. })),
            count(|line| matches!(line, Line::Empty)),
            count(|line| matches!(line, Line::Small { .. })),
            count(|line| matches!(line, Line::NotHeld)),
            count(|line| matches!(line, Line::Refused { .. })),
        );
    } else {
        let _ = writeln!(
            out,
            "Built {}, kept {}, empty {}, small {}, unsortable {}, not indexed {}, not held {}, refused {}: {}",
            count(|line| matches!(line, Line::Built { .. })),
            count(|line| matches!(line, Line::Kept { .. })),
            count(|line| matches!(line, Line::Empty)),
            count(|line| matches!(line, Line::Small { .. })),
            count(|line| matches!(line, Line::Unsortable { .. })),
            count(|line| matches!(line, Line::NotIndexed { .. })),
            count(|line| matches!(line, Line::NotHeld)),
            count(|line| matches!(line, Line::Refused { .. })),
            if clean && count(|line| matches!(line, Line::Small { .. })) > 0 {
                "every month in the span is indexed, or holds too few bars to need an index (D-4933)"
            } else if clean {
                "every month in the span is indexed"
            } else {
                "NOT every month in the span is indexed; rerun once the cause is gone (a rerun keeps what is built)"
            }
        );
    }
    if clean { OK } else { FAILED }
}

/// One month built as an item of an upkeep run (R3-17, D-6270): the step
/// this verb takes for it, and the receipt line this verb prints for it as the
/// run's outcome, so the two never say different things.
pub(crate) fn upkeep_month(
    root: &Path,
    path: StorePath<'_>,
    symbol_id: u32,
) -> pull::upkeep::Outcome {
    let line = build(root, path, symbol_id);
    let label = path.month().to_string();
    let mut text = String::new();
    render_line(&mut text, &label, &line);
    crate::upkeep_command::outcome(line.clean(), &label, &text)
}

/// The header's symbol id of `symbol`, as every writer folds it: the low 32
/// bits of its FNV-1a.
pub(crate) fn symbol_id(symbol: &str) -> u32 {
    let [a, b, c, d, ..] = brutex_core::universe::fnv1a(symbol).to_le_bytes();
    u32::from_le_bytes([a, b, c, d])
}

/// `cli srt-build`, against the CLI's store root (`BRUTEX_STORE`).
pub(crate) fn command(arguments: &[&str], out: &mut String) -> u8 {
    // AN UNSTAMPED BUILD BUILDS NO INDEX (DB-B71, WP-81): a `.srt` is a file
    // of the store that `/bars/window.json` serves sorted pages from.
    // `--dry-run` writes nothing and is not refused.
    let store = crate::store_write::root_for("srt-build", arguments);
    command_with(arguments, &store, out)
}

#[cfg(test)]
#[path = "srt_builder_tests.rs"]
mod tests;

/// THE CLOUD MUTATION PASS of 2026-10-16 (`docs/06-limits.md`, "The cli
/// import verbs"): each test pins the behaviour a mutant of this file broke
/// while every other test stayed green, or runs a line no test reached.
/// Invented stores only.
#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail"
)]
mod cloud_mutant_tests {
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    use store::format::{Bar, OI_NULL};

    use super::*;

    /// A store root that removes itself.
    struct Root(PathBuf);

    impl Root {
        fn new() -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let root = std::env::temp_dir().join(format!(
                "brutex-cli-srt-cloud-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            let _ = fs::remove_dir_all(&root);
            fs::create_dir_all(&root).unwrap();
            Self(root)
        }

        fn run(&self, arguments: &[&str]) -> (u8, String) {
            let mut out = String::new();
            let root = self.0.clone();
            let code = command_with(arguments, &move || Ok(root.clone()), &mut out);
            (code, out)
        }

        /// The store path of one month of `contract`, or of the spot series.
        fn path(contract: Option<&str>, month: YearMonth, file: FileKind) -> StorePath<'static> {
            StorePath::new(PathParts {
                vendor: Vendor::Gdfl,
                exchange: "NSE",
                segment: if contract.is_some() { "FNO" } else { "INDEX" },
                symbol: "NIFTY",
                contract: contract.and_then(Contract::parse),
                timeframe: Timeframe::SECOND_1,
                month,
                file,
            })
            .unwrap()
        }

        fn file(&self, contract: Option<&str>, month: YearMonth, file: FileKind) -> PathBuf {
            Self::path(contract, month, file).to_path_buf(&self.0)
        }

        /// One month of `count` invented bars under `symbol`'s header id, in
        /// `appends` appends (each past the first adds a run), its `.srt`
        /// taken away when `bare`.
        fn month(
            &self,
            contract: Option<&str>,
            month: YearMonth,
            (count, appends, bare): (i64, usize, bool),
            symbol: &str,
        ) {
            let mut writer = self.writer(contract, month, symbol);
            let rows = Self::rows(contract.is_some(), month, 0..count);
            if !rows.is_empty() {
                let cut = if appends == 1 { rows.len() } else { 1_024 };
                let (first, rest) = rows.split_at(cut);
                writer.append(first).unwrap();
                if appends > 1 {
                    for chunk in rest.chunks(rest.len().div_ceil(appends - 1)) {
                        writer.append(chunk).unwrap();
                    }
                }
            }
            drop(writer);
            let srt = self.file(contract, month, FileKind::SortIndex);
            if bare && srt.exists() {
                fs::remove_file(srt).unwrap();
            }
        }

        /// The writer of one month, filed under `symbol`'s header id.
        fn writer(&self, contract: Option<&str>, month: YearMonth, symbol: &str) -> BarFile {
            let symbol_id =
                u32::try_from(brutex_core::universe::fnv1a(symbol) & u64::from(u32::MAX)).unwrap();
            BarFile::open_or_create(
                &self.0,
                Self::path(contract, month, FileKind::Bars),
                symbol_id,
            )
            .unwrap()
        }

        /// Invented bars `ks` of a month: bar `k` one second after bar
        /// `k - 1`, from 09:15 IST on the month's second day.
        fn rows(contract: bool, month: YearMonth, ks: std::ops::Range<i64>) -> Vec<Bar> {
            let open = month.ist_bounds_micros().0 + (86_400 + 555 * 60) * 1_000_000;
            ks.map(|k| Bar {
                ts_micros: open + k * 1_000_000,
                open: 3_210_000 + k % 17,
                high: 3_210_050 + k % 17,
                low: 3_209_950 + k % 17,
                close: 3_210_010 + (k * 7) % 13,
                volume: k % 5,
                open_interest: if contract { 77 } else { OI_NULL },
            })
            .collect()
        }
    }

    impl Drop for Root {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn ym(year: u16, month: u8) -> YearMonth {
        YearMonth::new(year, month).unwrap()
    }

    const SPOT: [&str; 3] = ["gdfl", "NSE/INDEX/NIFTY", "1s"];

    fn span<'a>(from: &'a str, to: &'a str, dry: bool) -> Vec<&'a str> {
        let mut words = SPOT.to_vec();
        words.extend([from, to]);
        if dry {
            words.push("--dry-run");
        }
        words
    }

    /// The dispatched verb refuses a misuse with its usage, before the store
    /// root is asked for.
    #[test]
    fn the_dispatched_verb_refuses_a_misuse_with_its_usage() {
        let mut out = String::new();
        assert_eq!(command(&["gdfl"], &mut out), MISUSED);
        assert!(out.starts_with("refused: "), "{out}");
        assert!(out.contains(USAGE), "{out}");
    }

    /// A store root that cannot be proven fails the run, and a name the
    /// store cannot hold is a misuse refused before the store is asked.
    #[test]
    fn an_unproven_root_fails_and_an_unstorable_name_is_a_misuse() {
        let asked = std::cell::Cell::new(0_u32);
        let store = || {
            asked.set(asked.get() + 1);
            Err("no root".to_owned())
        };
        let mut out = String::new();
        let code = command_with(&span("2032-01", "2032-01", false), &store, &mut out);
        assert_eq!(code, FAILED);
        assert_eq!(
            out,
            "SRT BUILD REFUSED: no root. Nothing was read or written.\n"
        );
        let mut out = String::new();
        let code = command_with(
            &["gdfl", "NSE/INDEX/NI FTY", "1s", "2032-01", "2032-01"],
            &store,
            &mut out,
        );
        assert_eq!(code, MISUSED, "{out}");
        assert_eq!(asked.get(), 1, "a misuse never asks for the store root");
        assert!(out.contains("is not a store path"), "{out}");
        assert!(out.contains(USAGE), "{out}");
    }

    /// A `*` over a symbol the store has never listed is refused by name.
    #[test]
    fn a_star_over_a_symbol_with_no_directory_is_refused() {
        let root = Root::new();
        let (code, out) = root.run(&["gdfl", "NSE/FNO/NIFTY/*", "1s", "2032-01", "2032-01"]);
        assert_eq!(code, FAILED, "{out}");
        assert!(out.starts_with("SRT BUILD REFUSED: cannot list "), "{out}");
        assert!(out.ends_with(". Nothing was written.\n"), "{out}");
    }

    /// **WHAT A `*` LEFT ALONE IS SAID WHEN EITHER COUNT IS NOT ZERO, AND
    /// ONLY THEN.** A link alone is named, and so is a stray entry alone; a
    /// run that left nothing alone prints no `NOT WALKED` line.
    #[test]
    fn not_walked_is_said_for_a_link_alone_and_never_for_nothing() {
        let root = Root::new();
        let (ce, pe) = ("2032-02-26-1000000-CE", "2032-02-26-1000000-PE");
        root.month(Some(ce), ym(2032, 2), (1_200, 1, true), "NIFTY");
        let (code, out) = root.run(&[
            "gdfl",
            "NSE/FNO/NIFTY/*",
            "1s",
            "2032-02",
            "2032-02",
            "--dry-run",
        ]);
        assert_eq!(code, OK, "{out}");
        assert!(!out.contains("NOT WALKED"), "nothing was left alone: {out}");
        let symbol = root.0.join("bars/gdfl/NSE/FNO/NIFTY");
        std::os::unix::fs::symlink(symbol.join(ce), symbol.join(pe)).unwrap();
        let (code, out) = root.run(&[
            "gdfl",
            "NSE/FNO/NIFTY/*",
            "1s",
            "2032-02",
            "2032-02",
            "--dry-run",
        ]);
        assert_eq!(code, OK, "{out}");
        assert!(
            out.contains(
                "  NOT WALKED: 1 symbolic link(s) — never followed, nothing is built through a \
                 link — and 0 other entr(ies) that are not contract directories\n"
            ),
            "{out}"
        );
        fs::remove_file(symbol.join(pe)).unwrap();
        fs::create_dir(symbol.join("not-a-contract")).unwrap();
        let (code, out) = root.run(&[
            "gdfl",
            "NSE/FNO/NIFTY/*",
            "1s",
            "2032-02",
            "2032-02",
            "--dry-run",
        ]);
        assert_eq!(code, OK, "{out}");
        assert!(
            out.contains(
                "  NOT WALKED: 0 symbolic link(s) — never followed, nothing is built through a \
                 link — and 1 other entr(ies) that are not contract directories\n"
            ),
            "{out}"
        );
        // And a spot run lists nothing, so it leaves nothing alone.
        root.month(None, ym(2032, 2), (1_200, 1, false), "NIFTY");
        let (code, out) = root.run(&span("2032-02", "2032-02", true));
        assert_eq!(code, OK, "{out}");
        assert!(!out.contains("NOT WALKED"), "{out}");
    }

    /// **A DRY RUN CALLS A MONTH OF SEVERAL RUNS `would build`, NAMING THE
    /// RUNS**, never `indexed`: one run over every row is the only indexed
    /// shape.
    #[test]
    fn a_dry_run_names_a_month_of_several_runs_as_one_to_build() {
        let root = Root::new();
        root.month(None, ym(2032, 3), (1_200, 3, false), "NIFTY");
        let (code, out) = root.run(&span("2032-03", "2032-03", true));
        assert_eq!(code, OK, "{out}");
        assert!(
            out.contains(
                "  2032-03  would build        1200 bars  3 runs ordering rows 0..1200 of 1200\n"
            ),
            "{out}"
        );
    }

    /// **ONE RUN OVER FEWER ROWS THAN THE MONTH COMMITS IS NOT `indexed`.**
    /// The month's index as it stood at 1,024 rows is put back after 176
    /// more were appended: one run, but over rows 0..1024 of 1,200. The dry
    /// run says it would build it and names what the run covers; the build
    /// writes it whole.
    #[test]
    fn one_run_over_fewer_rows_than_committed_is_built_again() {
        let root = Root::new();
        let month = ym(2032, 9);
        root.month(None, month, (1_024, 1, false), "NIFTY");
        let srt = root.file(None, month, FileKind::SortIndex);
        let before_the_append = fs::read(&srt).unwrap();
        let mut writer = root.writer(None, month, "NIFTY");
        writer
            .append(&Root::rows(false, month, 1_024..1_200))
            .unwrap();
        drop(writer);
        fs::write(&srt, &before_the_append).unwrap();
        let (code, out) = root.run(&span("2032-09", "2032-09", true));
        assert_eq!(code, OK, "{out}");
        assert!(
            out.contains(
                "  2032-09  would build        1200 bars  1 runs ordering rows 0..1024 of 1200\n"
            ),
            "{out}"
        );
        let (code, out) = root.run(&span("2032-09", "2032-09", false));
        assert_eq!(code, OK, "{out}");
        assert!(out.contains("  2032-09  built          "), "{out}");
    }

    /// An empty month — a file that commits no rows — is `empty`, dry or
    /// not, writes nothing and is clean.
    #[test]
    fn an_empty_month_is_empty_dry_or_not() {
        let root = Root::new();
        root.month(None, ym(2032, 4), (0, 1, true), "NIFTY");
        for dry in [true, false] {
            let (code, out) = root.run(&span("2032-04", "2032-04", dry));
            assert_eq!(code, OK, "{out}");
            assert!(
                out.contains("  2032-04  empty         no committed bar to order\n"),
                "{out}"
            );
        }
        assert!(!root.file(None, ym(2032, 4), FileKind::SortIndex).exists());
    }

    /// **A MONTH NO RUN CAN ORDER IS `UNSORTABLE`.** The writer refuses a
    /// stamp off its rung's grid, so the bar here is moved half a second the
    /// way an append that bypassed the writer would leave it: row 500's stamp
    /// in place, and its block's `.crc` entry resealed over the new bytes
    /// (`docs/02-store-format.md`: 56-byte records from byte 32,768, 73 to a
    /// block, the entry for block `b` the CRC-32C of its committed bytes at
    /// `4 * b`). No sort-index entry can name half a second, so nothing is
    /// written and the run fails; the dry run says it would build.
    #[test]
    fn a_month_no_run_can_order_is_unsortable_and_fails_the_run() {
        let root = Root::new();
        let month = ym(2032, 5);
        root.month(None, month, (1_200, 1, true), "NIFTY");
        let (stride, per_block) = (
            usize::try_from(store::format::RECORD_STRIDE).unwrap(),
            usize::try_from(store::format::RECORDS_PER_BLOCK).unwrap(),
        );
        let start = usize::try_from(store::format::HEADER_LEN).unwrap();
        let bin = root.file(None, month, FileKind::Bars);
        let mut raw = fs::read(&bin).unwrap();
        let at = start + 500 * stride;
        let stamp = i64::from_le_bytes(raw[at..at + 8].try_into().unwrap());
        raw[at..at + 8].copy_from_slice(&(stamp + 500_000).to_le_bytes());
        fs::write(&bin, &raw).unwrap();
        let block = 500 / per_block;
        let from = start + block * per_block * stride;
        let seal = store::crc::crc32c(&raw[from..from + per_block * stride]);
        let crc = root.file(None, month, FileKind::Checksums);
        let mut entries = fs::read(&crc).unwrap();
        entries[4 * block..4 * block + 4].copy_from_slice(&seal.to_le_bytes());
        fs::write(&crc, &entries).unwrap();
        let (code, out) = root.run(&span("2032-05", "2032-05", true));
        assert_eq!(code, OK, "{out}");
        assert!(
            out.contains("  2032-05  would build        1200 bars  "),
            "{out}"
        );
        let (code, out) = root.run(&span("2032-05", "2032-05", false));
        assert_eq!(code, FAILED, "{out}");
        assert!(
            out.contains("  2032-05  UNSORTABLE         1200 bars  nothing written; a sort reads these rows: "),
            "{out}"
        );
        assert!(out.contains("unsortable 1, "), "{out}");
        assert!(!root.file(None, month, FileKind::SortIndex).exists());
    }

    /// A month filed under another symbol's header id is refused by the
    /// store, dry or not, and nothing is written to it.
    #[test]
    fn a_month_under_another_symbols_id_is_refused_dry_or_not() {
        let root = Root::new();
        root.month(None, ym(2032, 6), (1_200, 1, true), "BANKNIFTY");
        for dry in [true, false] {
            let (code, out) = root.run(&span("2032-06", "2032-06", dry));
            assert_eq!(code, FAILED, "{out}");
            assert!(out.contains("  2032-06  REFUSED       "), "{out}");
            assert!(out.contains("refused 1"), "{out}");
        }
        assert!(!root.file(None, ym(2032, 6), FileKind::SortIndex).exists());
    }

    /// The receipt lines no store state in a test reaches, word for word.
    #[test]
    fn the_receipt_line_for_a_month_a_reader_does_not_find_indexed() {
        let mut out = String::new();
        render_line(
            &mut out,
            "2032-07",
            &Line::NotIndexed {
                n_valid: 1_200,
                reader: "2 runs ordering rows 0..1200 of 1200".to_owned(),
            },
        );
        assert_eq!(
            out,
            "  2032-07  NOT INDEXED        1200 bars  the step finished and a reader finds 2 runs \
             ordering rows 0..1200 of 1200\n"
        );
        let mut out = String::new();
        assert_eq!(
            summed(
                &mut out,
                &[
                    Line::Built {
                        n_valid: 1,
                        because: String::new()
                    },
                    Line::NotIndexed {
                        n_valid: 1,
                        reader: String::new()
                    }
                ],
                false
            ),
            FAILED
        );
        assert!(out.starts_with("Built 1, kept 0, empty 0, small 0, unsortable 0, not indexed 1, not held 0, refused 0: NOT every"), "{out}");
    }

    /// **A `*` WALKS EXACTLY ITS BOUND AND REFUSES ONE MORE**, by name and
    /// before a month is opened; a link and a stray entry are counted for
    /// the receipt and never against the bound. The bound here is 3 rather
    /// than `MAX_CONTRACTS`, through the one walk every caller uses
    /// ([`Ask::series`] passes `MAX_CONTRACTS` to it), so `>` is told from
    /// `>=`, `==` and `<` with five directories; the documented figure is
    /// pinned beside it.
    #[test]
    fn a_star_walks_exactly_its_bound_and_refuses_one_more() {
        assert_eq!(
            MAX_CONTRACTS, 100_000,
            "the bound docs/06-limits.md and the refusal name"
        );
        let root = Root::new();
        let symbol = root.0.join("bars/gdfl/NSE/FNO/NIFTY");
        fs::create_dir_all(&symbol).unwrap();
        for k in 0..3 {
            fs::create_dir(symbol.join(format!("C{k:06}"))).unwrap();
        }
        std::os::unix::fs::symlink(symbol.join("C000000"), symbol.join("C900000")).unwrap();
        fs::create_dir(symbol.join("not-a-contract")).unwrap();
        let star = ask(&["gdfl", "NSE/FNO/NIFTY/*", "1s", "2032-01", "2032-01"]).unwrap();
        let walk = star.series_within(&root.0, 3).unwrap();
        assert_eq!(
            walk.series,
            ["C000000", "C000001", "C000002"]
                .map(Contract::parse)
                .to_vec(),
            "exactly the bound is walked, in name order"
        );
        assert_eq!((walk.links, walk.others), (1, 1));
        fs::create_dir(symbol.join("C000003")).unwrap();
        assert_eq!(
            star.series_within(&root.0, 3),
            Err(format!(
                "{} holds more than 3 contract directories; name fewer",
                symbol.display()
            ))
        );
        assert_eq!(
            star.series(&root.0).unwrap().series.len(),
            4,
            "the verb's own bound walks all four"
        );
    }

    /// **A `*` WALKS EXACTLY `MAX_CONTRACTS` CONTRACT DIRECTORIES AND REFUSES
    /// ONE MORE**, through the verb, before a month is opened. At the bound
    /// every contract is walked (none holds the month, so the run says so
    /// and fails); one past it the listing is refused by name, and nothing
    /// is written beside the directories either time.
    ///
    /// **Opt-in, because of what it costs.** It makes 100,001 empty
    /// directories in the temporary directory: on the operator's Mac under
    /// load one run took up to 19 minutes, and a process KILLED inside it (a
    /// mutation run's timeout, an interrupted `cargo test`) never runs
    /// [`Root`]'s `Drop` and leaves the tree behind; 14 such trees were found
    /// on 21 Oct 2026 (`docs/06-limits.md`, "The cli import verbs").
    /// [`a_star_walks_exactly_its_bound_and_refuses_one_more`] proves the
    /// comparison with five directories; this proves the real bound end to
    /// end: `cargo test -p cli --lib -- --ignored --exact
    /// srt_builder::cloud_mutant_tests::a_star_walks_exactly_the_contract_bound_and_refuses_one_more`.
    #[test]
    #[ignore = "makes 100,001 directories; the bound itself is proved by a_star_walks_exactly_its_bound_and_refuses_one_more"]
    fn a_star_walks_exactly_the_contract_bound_and_refuses_one_more() {
        let root = Root::new();
        let symbol = root.0.join("bars/gdfl/NSE/FNO/NIFTY");
        fs::create_dir_all(&symbol).unwrap();
        for k in 0..MAX_CONTRACTS {
            fs::create_dir(symbol.join(format!("C{k:06}"))).unwrap();
        }
        let star = [
            "gdfl",
            "NSE/FNO/NIFTY/*",
            "1s",
            "2032-01",
            "2032-01",
            "--dry-run",
        ];
        let (code, out) = root.run(&star);
        assert_eq!(code, FAILED, "{out}");
        assert!(
            out.contains(&format!("{MAX_CONTRACTS} series × 1 month(s)")),
            "{out}"
        );
        assert!(
            out.contains(&format!(
                "NO MONTH HELD: none of the {MAX_CONTRACTS} contract(s) holds a month"
            )),
            "{out}"
        );
        assert_eq!(
            fs::read_dir(&symbol).unwrap().count(),
            MAX_CONTRACTS,
            "a dry run at the bound wrote nothing beside the contracts"
        );
        fs::create_dir(symbol.join(format!("C{MAX_CONTRACTS:06}"))).unwrap();
        let (code, out) = root.run(&star);
        assert_eq!(code, FAILED, "{out}");
        assert_eq!(
            out,
            format!(
                "SRT BUILD REFUSED: {} holds more than {MAX_CONTRACTS} contract directories; \
                 name fewer. Nothing was written.\n",
                symbol.display()
            )
        );
        assert_eq!(
            fs::read_dir(&symbol).unwrap().count(),
            MAX_CONTRACTS + 1,
            "a refused listing wrote nothing beside the contracts"
        );
        assert_eq!(
            fs::read_dir(&root.0).unwrap().count(),
            1,
            "the store root holds the bars tree alone: no manifest, no log"
        );
    }

    /// The spot month's path and the symbol id the verb opens it under.
    fn spot(month: YearMonth) -> (StorePath<'static>, u32) {
        let id = ask(&span("2032-01", "2032-01", false)).unwrap().symbol_id();
        (Root::path(None, month, FileKind::Bars), id)
    }

    /// **A BUILT MONTH IS READ BACK WHOLE, A KEPT ONE BY ITS HEADERS.** The
    /// store keeps a month only after checking its run's body under the
    /// month's lock, and builds one from bars it has just verified; the
    /// step asks the read-back for the body check exactly when it built.
    /// Through the verb the two answers agree whichever is asked, so only the
    /// flag the step hands its read-back tells them apart (the cloud pass's
    /// equivalent mutant at `read_back_body`).
    #[test]
    fn the_read_back_checks_the_body_of_a_built_month_and_not_of_a_kept_one() {
        let root = Root::new();
        let month = ym(2032, 10);
        root.month(None, month, (1_200, 1, true), "NIFTY");
        let (path, id) = spot(month);
        let asked = std::cell::RefCell::new(Vec::new());
        let built = build_reading(&root.0, path, id, &|at, path, id, body| {
            asked.borrow_mut().push(body);
            reader_view(at, path, id, body)
        });
        assert!(
            matches!(built, Line::Built { n_valid: 1_200, .. }),
            "{built:?}"
        );
        let kept = build_reading(&root.0, path, id, &|at, path, id, body| {
            asked.borrow_mut().push(body);
            reader_view(at, path, id, body)
        });
        assert_eq!(kept, Line::Kept { n_valid: 1_200 });
        assert_eq!(
            *asked.borrow(),
            [true, false],
            "built: whole; kept: headers"
        );
        assert_eq!(
            build(&root.0, path, id),
            Line::Kept { n_valid: 1_200 },
            "the verb's own read-back agrees"
        );
    }

    /// **A READ-BACK THAT DISAGREES WITH THE STEP IS SAID, NEVER TRUSTED.**
    /// The step kept the month; a reader that finds two runs makes it `NOT
    /// INDEXED` with the reader's words, and a reader that is refused (here,
    /// one that opens the month under another symbol's id) makes it
    /// `REFUSED` naming the read-back. No store state a test can make reaches
    /// either through the verb: the step and the reader decide from one
    /// header under the month's lock.
    #[test]
    fn a_read_back_that_disagrees_with_the_step_is_not_indexed_or_refused() {
        let root = Root::new();
        let month = ym(2032, 11);
        root.month(None, month, (1_200, 1, false), "NIFTY");
        let (path, id) = spot(month);
        let two_runs = "2 runs ordering rows 0..1200 of 1200";
        assert_eq!(
            build_reading(&root.0, path, id, &|_, _, _, _| {
                Ok((1_200, Some(two_runs.to_owned()), PathBuf::new()))
            }),
            Line::NotIndexed {
                n_valid: 1_200,
                reader: two_runs.to_owned()
            }
        );
        let refused = build_reading(&root.0, path, id, &|at, path, id, body| {
            reader_view(at, path, id ^ 1, body)
        });
        assert!(
            matches!(&refused, Line::Refused { why }
                if why.starts_with("the step finished and the read-back was refused: ")),
            "{refused:?}"
        );
        assert_eq!(build(&root.0, path, id), Line::Kept { n_valid: 1_200 });
    }
}

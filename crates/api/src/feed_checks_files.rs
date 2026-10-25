//! The check files `/feed-checks.json` and `/tick-checks.json` read, held in
//! memory once per version as fixed-stride rows (DATA-1w, D-5190, D-5191).
//!
//! # What is read
//!
//! - **DATA-1's feed reconcile** (`cli feed-reconcile`): the newest FULL
//!   version of a month, `feed-reconcile-YYYY-MM.v<N>.csv`, through
//!   `cli::feed_reconcile::file_digest`, the one door its own reader uses: a
//!   file of another format, cut or damaged is refused there, by its words.
//!   A `--symbols` file is never a month's authority and is never read here.
//! - **DATA-2's tick-to-second check** (`cli tick-to-second-check`): the
//!   newest version of a year that has its index, `tick-to-second-YYYY.vNNN.idx`
//!   (linked last, so a version without one is not finished), through
//!   `cli::tick_second_check::index::newest`, and the version's CSV beside it,
//!   whose BLAKE3 must be the one the index records.
//!
//! Both are read only as regular files standing at their own names
//! (`index::read_regular`). Nothing here writes anywhere.
//!
//! # Fixed stride, and what it costs
//!
//! A version is parsed ONCE, in one pass over its bytes, into a vector of
//! fixed-size rows ([`FeedRow`], [`TickRow`]) and the tallies a page shows;
//! every word a row carries is interned to a small number. Each row keeps its
//! line's byte offset, length and FNV-1a, so the source row behind any number
//! is one positional read ([`source_row`]), checked against that hash.
//!
//! A request then costs one `stat` of the folder (its modification time says
//! whether the listing still stands), one map probe for the version, and
//! O(limit) for the rows it returns. The one-pass load is O(the version's
//! bytes), once per version per server process, and is named in
//! `docs/06-limits.md` (L-D1W-02). Only the newest version of each month or
//! year is held, so memory is bounded by the months and years the folder
//! holds (L-D1W-03).

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Instant, SystemTime};

use cli::tick_second_check::index;
use pull::session::Day;
use store::path::YearMonth;

/// Why a read has no answer, and so which status it is served with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Refusal {
    /// The question cannot be answered as asked: 400, naming the field.
    Asked(String),
    /// Nothing is there to answer it: 404, naming where it looked.
    Absent(String),
    /// A file is there and is refused: 500, naming the file and why.
    Refused(String),
}

impl Refusal {
    /// The words of the refusal.
    pub(crate) fn text(&self) -> &str {
        match self {
            Self::Asked(why) | Self::Absent(why) | Self::Refused(why) => why,
        }
    }

    /// A refused version's words as they are held (D-5196): the answer
    /// stands until a newer version appears or the server restarts.
    fn held(self) -> Self {
        match self {
            Self::Refused(why) => Self::Refused(format!(
                "{why} (held: this version is refused until a newer version appears or the \
                 server restarts)"
            )),
            other => other,
        }
    }
}

/// Words interned to small numbers, in the order first seen.
#[derive(Debug, Default)]
pub(crate) struct Words {
    names: Vec<String>,
    at: HashMap<String, u16>,
}

impl Words {
    /// The number of `word`, given one the first time it is seen.
    fn id(&mut self, word: &str) -> Result<u16, String> {
        if let Some(id) = self.at.get(word) {
            return Ok(*id);
        }
        let id = u16::try_from(self.names.len())
            .map_err(|_| format!("more than {} distinct words in one column", u16::MAX))?;
        self.names.push(word.to_owned());
        self.at.insert(word.to_owned(), id);
        Ok(id)
    }

    /// The word numbered `id`, empty for a number never given.
    pub(crate) fn name(&self, id: u16) -> &str {
        self.names.get(usize::from(id)).map_or("", String::as_str)
    }

    /// The number of `word`, when it was seen.
    pub(crate) fn find(&self, word: &str) -> Option<u16> {
        self.at.get(word).copied()
    }

    /// Every word, in number order.
    pub(crate) fn all(&self) -> &[String] {
        &self.names
    }
}

/// Adds one to `counts[id]`, growing it as needed.
fn bump(counts: &mut Vec<u32>, id: u16) {
    let at = usize::from(id);
    if counts.len() <= at {
        counts.resize(at + 1, 0);
    }
    if let Some(count) = counts.get_mut(at) {
        *count = count.saturating_add(1);
    }
}

/// The count at `id`, zero past the end.
pub(crate) fn count_at(counts: &[u32], id: u16) -> u32 {
    counts.get(usize::from(id)).copied().unwrap_or(0)
}

/// One line of a version file: where it is and what it hashed to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Place {
    /// The line's number in the file, 1 = its first line.
    pub(crate) line: u32,
    /// The byte its text starts at.
    pub(crate) at: u64,
    /// Its bytes, without the newline.
    pub(crate) len: u32,
    /// FNV-1a of its text, so a re-read proves it is the same line.
    pub(crate) hash: u64,
}

/// Every line of `text` with where it starts, in order, the newline cut off.
fn lines_of(text: &str) -> impl Iterator<Item = (usize, &str)> {
    let mut at = 0_usize;
    text.split_inclusive('\n').map(move |piece| {
        let start = at;
        at += piece.len();
        (start, piece.strip_suffix('\n').unwrap_or(piece))
    })
}

fn place_of(line: usize, start: usize, text: &str) -> Result<Place, String> {
    Ok(Place {
        line: u32::try_from(line).map_err(|_| "more than 2^32 lines".to_owned())?,
        at: u64::try_from(start).map_err(|_| "an offset past 2^64".to_owned())?,
        len: u32::try_from(text.len()).map_err(|_| format!("line {line} is over 4 GiB"))?,
        hash: brutex_core::universe::fnv1a(text),
    })
}

// ═══ Folder listings: one `stat` per request, one listing per change ═══

/// The newest version of every month or year one folder holds, and the
/// folder's modification time when it was listed.
#[derive(Debug)]
pub(crate) struct Listing<K> {
    stamp: Option<SystemTime>,
    /// Each key's newest version number.
    pub(crate) newest: BTreeMap<K, u32>,
    /// The same pairs in key order, so a page is one slice of it: O(limit),
    /// never a walk past `offset` keys.
    pub(crate) order: Vec<(K, u32)>,
}

impl<K> Listing<K> {
    /// Whether the folder existed when it was listed: a folder that does not
    /// exist is named as such, never answered as an empty check.
    pub(crate) const fn exists(&self) -> bool {
        self.stamp.is_some()
    }

    /// The pairs `range` covers, in key order.
    pub(crate) fn page(&self, range: (usize, usize)) -> &[(K, u32)] {
        self.order.get(range.0..range.1).unwrap_or(&[])
    }
}

type Listings<K> = Mutex<HashMap<PathBuf, Arc<Listing<K>>>>;

/// The listing of `dir`, re-read only when its modification time moved.
///
/// THE `stat` COMES FIRST: a name linked after it moves the time again, so
/// the next request lists again; a name linked before it is in this listing.
/// A folder that does not exist holds nothing, and is asked again each time.
fn listing<K: Ord + Copy>(
    cache: &Listings<K>,
    dir: &Path,
    parse: fn(&str) -> Option<(K, u32)>,
) -> Result<Arc<Listing<K>>, Refusal> {
    let stamp = match std::fs::metadata(dir) {
        Ok(meta) if meta.is_dir() => Some(meta.modified().map_err(|why| {
            Refusal::Refused(format!(
                "`{}` has no modification time: {why}",
                dir.display()
            ))
        })?),
        Ok(_) => {
            return Err(Refusal::Refused(format!(
                "`{}` is not a folder",
                dir.display()
            )));
        }
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => None,
        Err(why) => {
            return Err(Refusal::Refused(format!(
                "`{}` cannot be read: {why}",
                dir.display()
            )));
        }
    };
    if let Some(held) = cache
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get(dir)
        && held.stamp == stamp
        && stamp.is_some()
    {
        return Ok(Arc::clone(held));
    }
    let mut newest = BTreeMap::new();
    if stamp.is_some() {
        let entries = std::fs::read_dir(dir)
            .map_err(|why| Refusal::Refused(format!("listing `{}`: {why}", dir.display())))?;
        for entry in entries {
            let entry = entry
                .map_err(|why| Refusal::Refused(format!("listing `{}`: {why}", dir.display())))?;
            let Ok(name) = entry.file_name().into_string() else {
                continue;
            };
            if let Some((key, version)) = parse(&name) {
                let slot = newest.entry(key).or_insert(version);
                *slot = (*slot).max(version);
            }
        }
    }
    let order = newest
        .iter()
        .map(|(key, version)| (*key, *version))
        .collect();
    let listed = Arc::new(Listing {
        stamp,
        newest,
        order,
    });
    cache
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .insert(dir.to_path_buf(), Arc::clone(&listed));
    Ok(listed)
}

/// Digits that round-trip as a `u32`: `7`, never `+7` or `007`.
fn canonical(digits: &str) -> Option<u32> {
    digits
        .parse::<u32>()
        .ok()
        .filter(|n| n.to_string() == digits)
}

/// `YYYY-MM` that round-trips as a month.
pub(crate) fn month_of(text: &str) -> Option<YearMonth> {
    let (year, month) = text.split_once('-')?;
    if year.len() != 4 || month.len() != 2 {
        return None;
    }
    if !year
        .bytes()
        .chain(month.bytes())
        .all(|b| b.is_ascii_digit())
    {
        return None;
    }
    let parsed = YearMonth::new(year.parse().ok()?, month.parse().ok()?).ok()?;
    (parsed.to_string() == text).then_some(parsed)
}

/// A FULL feed-reconcile version's name, `feed-reconcile-YYYY-MM.v<N>.csv`,
/// exactly as `cli feed-reconcile` names one; a `.symbols.` file is not one.
pub(crate) fn feed_name(name: &str) -> Option<(YearMonth, u32)> {
    let rest = name.strip_prefix("feed-reconcile-")?;
    let (month, rest) = rest.split_at_checked(7)?;
    let version = rest.strip_prefix(".v")?.strip_suffix(".csv")?;
    Some((month_of(month)?, canonical(version)?))
}

/// A tick-to-second index's name, `tick-to-second-YYYY.v<digits>.idx`, read
/// as `cli::tick_second_check::index::versions_in` reads one.
pub(crate) fn tick_name(name: &str) -> Option<(u16, u32)> {
    let rest = name.strip_prefix("tick-to-second-")?;
    let (year, rest) = rest.split_at_checked(4)?;
    let digits = rest.strip_prefix(".v")?.strip_suffix(".idx")?;
    if !year.bytes().all(|b| b.is_ascii_digit())
        || digits.is_empty()
        || !digits.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    Some((year.parse().ok()?, digits.parse().ok()?))
}

static FEED_LISTINGS: std::sync::LazyLock<Listings<YearMonth>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));
static TICK_LISTINGS: std::sync::LazyLock<Listings<u16>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

/// The feed-reconcile months `dir` holds, each with its newest version.
pub(crate) fn feed_listing(dir: &Path) -> Result<Arc<Listing<YearMonth>>, Refusal> {
    listing(&FEED_LISTINGS, dir, feed_name)
}

/// The tick-to-second years `dir` holds an index of, each with its newest.
pub(crate) fn tick_listing(dir: &Path) -> Result<Arc<Listing<u16>>, Refusal> {
    listing(&TICK_LISTINGS, dir, tick_name)
}

// ═══ The feed-reconcile month ═══

/// The cells of the feed-reconcile CSV this reader keeps, by name.
const FEED_KEPT: [&str; 12] = [
    "symbol",
    "day",
    "day_kind",
    "status",
    "applied_lag",
    "lag_source",
    "median_z",
    "median_g",
    "short_volume",
    "short_judgement",
    "ex_date",
    "flags",
];

/// One stock-day row of a feed-reconcile month, fixed size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FeedRow {
    /// Where the row is in its file.
    pub(crate) place: Place,
    /// The day of the month, 1 to 31; 0 for a share's whole-month row (`-`).
    pub(crate) day: u8,
    /// The status's word (before any `:`), interned in `statuses`.
    pub(crate) status: u16,
    /// The whole status cell, interned in `status_texts`.
    pub(crate) status_text: u16,
    /// `day_kind`, interned.
    pub(crate) day_kind: u16,
    /// `lag_source`, interned.
    pub(crate) lag_source: u16,
    /// `short_judgement`, interned.
    pub(crate) judgement: u16,
    /// The `ex_date` verdict's word (`yes`, `no`, `unchecked`, or empty).
    pub(crate) ex_date: u16,
    /// The flags, one bit per interned flag name.
    pub(crate) flags: u64,
    /// `applied_lag`, in seconds.
    pub(crate) lag: Option<i8>,
    /// `(median_z, median_g)`: the exact paisa pair of the day's factor.
    pub(crate) median: Option<(i64, i64)>,
    /// `short_volume`: day bar minus minute sum minus what GDFL explains.
    pub(crate) short: Option<i128>,
}

/// What a set of feed-reconcile rows says, counted.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct FeedTally {
    /// Rows.
    pub(crate) rows: u32,
    /// Rows of one day (not a share's whole-month row).
    pub(crate) days: u32,
    /// Rows per status word.
    pub(crate) status: Vec<u32>,
    /// Rows per flag.
    pub(crate) flags: Vec<u32>,
    /// Rows per `ex_date` verdict word.
    pub(crate) ex_date: Vec<u32>,
    /// Rows per `short_judgement`.
    pub(crate) judgement: Vec<u32>,
    /// Rows whose `short_volume` is above zero.
    pub(crate) short_positive: u32,
}

impl FeedTally {
    fn add(&mut self, row: &FeedRow) {
        self.rows = self.rows.saturating_add(1);
        self.days = self.days.saturating_add(u32::from(row.day > 0));
        bump(&mut self.status, row.status);
        bump(&mut self.ex_date, row.ex_date);
        bump(&mut self.judgement, row.judgement);
        // AT MOST 64 BITS, one step each: the flag word is a fixed width.
        for bit in 0..64_u16 {
            if row.flags & (1 << bit) != 0 {
                bump(&mut self.flags, bit);
            }
        }
        self.short_positive = self
            .short_positive
            .saturating_add(u32::from(row.short.is_some_and(|short| short > 0)));
    }
}

/// One share's rows in a month: consecutive, as the writer prints them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FeedStock {
    /// The share's stored key.
    pub(crate) name: String,
    /// Its first row in [`FeedMonth::rows`].
    pub(crate) first: u32,
    /// Its rows.
    pub(crate) count: u32,
    /// What they say.
    pub(crate) tally: FeedTally,
}

/// One feed-reconcile month version, parsed once.
#[derive(Debug)]
pub(crate) struct FeedMonth {
    /// The month.
    pub(crate) month: YearMonth,
    /// The version read.
    pub(crate) version: u32,
    /// Its path.
    pub(crate) file: PathBuf,
    /// BLAKE3 of its bytes.
    pub(crate) digest: [u8; 32],
    /// Its size.
    pub(crate) bytes: u64,
    /// The comment lines before the column line (the run's own facts).
    pub(crate) header: Vec<String>,
    /// The writer's own `# summary:` line, as it wrote it.
    pub(crate) summary: Option<String>,
    /// The column names.
    pub(crate) columns: Vec<String>,
    /// Every row, in file order.
    pub(crate) rows: Vec<FeedRow>,
    /// Every share with a row, in file order.
    pub(crate) stocks: Vec<FeedStock>,
    slots: HashMap<String, u32>,
    /// Status words.
    pub(crate) statuses: Words,
    /// Whole status cells.
    pub(crate) status_texts: Words,
    /// Day kinds.
    pub(crate) day_kinds: Words,
    /// Lag sources.
    pub(crate) lag_sources: Words,
    /// Short judgements.
    pub(crate) judgements: Words,
    /// `ex_date` verdict words.
    pub(crate) ex_dates: Words,
    /// Flag names.
    pub(crate) flag_names: Words,
    /// What every row says.
    pub(crate) tally: FeedTally,
    /// Shares with at least one `adjusted` day.
    pub(crate) adjusted_stocks: u32,
    /// Shares with at least one `ex_date` day.
    pub(crate) ex_date_stocks: u32,
    /// How long the one-pass load took, in microseconds.
    pub(crate) load_micros: u64,
}

impl FeedMonth {
    /// The share named `symbol`, when it has rows.
    pub(crate) fn stock(&self, symbol: &str) -> Option<&FeedStock> {
        self.slots
            .get(symbol)
            .and_then(|slot| self.stocks.get(usize::try_from(*slot).ok()?))
    }

    /// The rows of `stock`.
    pub(crate) fn rows_of(&self, stock: &FeedStock) -> &[FeedRow] {
        let first = usize::try_from(stock.first).unwrap_or(usize::MAX);
        let count = usize::try_from(stock.count).unwrap_or(0);
        self.rows
            .get(first..first.saturating_add(count))
            .unwrap_or(&[])
    }

    /// The flag bit of `name`, when the month printed it.
    pub(crate) fn flag(&self, name: &str) -> Option<u16> {
        self.flag_names.find(name)
    }
}

/// A cell that must parse, or the row is refused naming it.
fn parsed<T: std::str::FromStr>(text: &str, column: &str) -> Result<Option<T>, String> {
    if text.is_empty() {
        return Ok(None);
    }
    text.parse()
        .map(Some)
        .map_err(|_| format!("column `{column}` holds `{text}`"))
}

/// One feed row's cells, by the kept names' positions.
fn feed_row(
    month: &mut FeedMonth,
    cells: &[&str],
    at: &[usize; FEED_KEPT.len()],
    place: Place,
) -> Result<(String, FeedRow), String> {
    let cell = |i: usize| -> &str {
        at.get(i)
            .and_then(|position| cells.get(*position))
            .copied()
            .unwrap_or_default()
    };
    let symbol = cell(0).to_owned();
    if symbol.is_empty() {
        return Err("an empty `symbol`".to_owned());
    }
    let day = match cell(1) {
        "-" => 0,
        text => {
            let civil = day_of_year(text, month.month.year())
                .ok()
                .filter(|civil| civil.month() == month.month.month());
            civil
                .ok_or_else(|| format!("`day` holds `{text}`, not a day of {}", month.month))?
                .day()
        }
    };
    let status_text = cell(3);
    let word = status_text.split(':').next().unwrap_or_default();
    let mut flags = 0_u64;
    for name in cell(11).split('|').filter(|name| !name.is_empty()) {
        let bit = month.flag_names.id(name)?;
        if bit >= 64 {
            return Err("more than 64 distinct flag names".to_owned());
        }
        flags |= 1 << bit;
    }
    let pair = match (
        parsed::<i64>(cell(6), "median_z")?,
        parsed::<i64>(cell(7), "median_g")?,
    ) {
        (None, None) => None,
        (Some(z), Some(g)) if z >= 1 && g >= 1 => Some((z, g)),
        (z, g) => {
            return Err(format!(
                "`median_z` {z:?} and `median_g` {g:?}: a factor is two paisa prices of at least 1"
            ));
        }
    };
    let row = FeedRow {
        place,
        day,
        status: month.statuses.id(word)?,
        status_text: month.status_texts.id(status_text)?,
        day_kind: month.day_kinds.id(cell(2))?,
        lag_source: month.lag_sources.id(cell(5))?,
        judgement: month.judgements.id(cell(9))?,
        ex_date: month
            .ex_dates
            .id(cell(10).split(':').next().unwrap_or_default())?,
        flags,
        lag: parsed(cell(4), "applied_lag")?,
        median: pair,
        short: parsed(cell(8), "short_volume")?,
    };
    Ok((symbol, row))
}

impl FeedMonth {
    /// A month of `version` with nothing parsed yet.
    fn empty(month: YearMonth, version: u32, file: PathBuf, text: &str, digest: [u8; 32]) -> Self {
        Self {
            month,
            version,
            file,
            digest,
            bytes: u64::try_from(text.len()).unwrap_or(u64::MAX),
            header: Vec::new(),
            summary: None,
            columns: Vec::new(),
            rows: Vec::new(),
            stocks: Vec::new(),
            slots: HashMap::new(),
            statuses: Words::default(),
            status_texts: Words::default(),
            day_kinds: Words::default(),
            lag_sources: Words::default(),
            judgements: Words::default(),
            ex_dates: Words::default(),
            flag_names: Words::default(),
            tally: FeedTally::default(),
            adjusted_stocks: 0,
            ex_date_stocks: 0,
            load_micros: 0,
        }
    }
}

/// Parses one feed-reconcile version's text.
fn parse_feed(
    month: YearMonth,
    version: u32,
    file: PathBuf,
    text: &str,
    digest: [u8; 32],
) -> Result<FeedMonth, String> {
    let mut parsed = FeedMonth::empty(month, version, file, text, digest);
    let mut lines = lines_of(text).enumerate();
    let mut at = [0_usize; FEED_KEPT.len()];
    for (_, (_, line)) in lines.by_ref() {
        if let Some(comment) = line.strip_prefix('#') {
            parsed.header.push(comment.trim_start().to_owned());
            continue;
        }
        if line != cli::feed_reconcile::COLUMNS {
            return Err(
                "its column line is not this build's `cli::feed_reconcile::COLUMNS`".to_owned(),
            );
        }
        parsed.columns = line.split(',').map(str::to_owned).collect();
        for (slot, name) in at.iter_mut().zip(FEED_KEPT) {
            *slot = parsed
                .columns
                .iter()
                .position(|column| column == name)
                .ok_or_else(|| format!("no column `{name}`"))?;
        }
        break;
    }
    if parsed.columns.is_empty() {
        return Err("it has no column line".to_owned());
    }
    let width = parsed.columns.len();
    let mut current: Option<String> = None;
    for (number, (start, line)) in lines {
        if let Some(comment) = line.strip_prefix('#') {
            if let Some(summary) = comment.strip_prefix(" summary: ") {
                parsed.summary = Some(summary.to_owned());
            }
            continue;
        }
        if parsed.summary.is_some() || line.is_empty() {
            return Err(format!(
                "line {}: a row after the comment block",
                number + 1
            ));
        }
        let cells: Vec<&str> = line.split(',').collect();
        if cells.len() != width {
            return Err(format!(
                "line {}: {} cells, the column line names {width}",
                number + 1,
                cells.len()
            ));
        }
        let place = place_of(number + 1, start, line)?;
        let (symbol, row) = feed_row(&mut parsed, &cells, &at, place)
            .map_err(|why| format!("line {}: {why}", number + 1))?;
        let index = u32::try_from(parsed.rows.len()).map_err(|_| "too many rows".to_owned())?;
        if current.as_deref() != Some(symbol.as_str()) {
            if parsed.slots.contains_key(&symbol) {
                return Err(format!(
                    "line {}: share `{symbol}` has rows in two places; the writer prints one \
                     share's rows together",
                    number + 1
                ));
            }
            let slot =
                u32::try_from(parsed.stocks.len()).map_err(|_| "too many shares".to_owned())?;
            parsed.slots.insert(symbol.clone(), slot);
            parsed.stocks.push(FeedStock {
                name: symbol.clone(),
                first: index,
                count: 0,
                tally: FeedTally::default(),
            });
            current = Some(symbol);
        }
        parsed.tally.add(&row);
        if let Some(stock) = parsed.stocks.last_mut() {
            stock.tally.add(&row);
            stock.count = stock.tally.rows;
        }
        parsed.rows.push(row);
    }
    if parsed.summary.is_none() {
        return Err("it has no `# summary:` line, the line the writer prints last".to_owned());
    }
    let adjusted = parsed.flag("adjusted");
    let ex_date = parsed.flag("ex_date");
    for stock in &parsed.stocks {
        let has = |bit: Option<u16>| bit.is_some_and(|bit| count_at(&stock.tally.flags, bit) > 0);
        parsed.adjusted_stocks = parsed
            .adjusted_stocks
            .saturating_add(u32::from(has(adjusted)));
        parsed.ex_date_stocks = parsed
            .ex_date_stocks
            .saturating_add(u32::from(has(ex_date)));
    }
    Ok(parsed)
}

/// The answer one version gave, held under the version the folder's listing
/// named for it (D-5196): the parsed version, or its refusal.
///
/// A REFUSAL IS HELD TOO. A version file is never rewritten (append-only), so
/// a version that was cut, damaged, of another format or not its index's is
/// refused the same way on every later ask; reading and parsing it again on
/// each request would make a listing page that names it O(its bytes) per
/// request, and would warn on `/logs` once per request instead of once per
/// version. A newer version (a new name, so a new listing) is read afresh;
/// a refusal that came from the host rather than the bytes stays until then
/// or until the server restarts, and its answer says so
/// (`docs/06-limits.md` L-D1W-06).
#[derive(Debug)]
struct Held<T> {
    /// The version the listing named when this was loaded.
    asked: u32,
    /// How many times this key was loaded in this process (1 the first).
    loads: u32,
    /// The parsed version, or why it was refused.
    answer: Result<Arc<T>, Refusal>,
}

/// Held answers, by folder and month or year.
type HeldMap<K, T> = Mutex<HashMap<(PathBuf, K), Held<T>>>;

/// The held answer of `key` if it was loaded for `asked`.
fn held<K: std::hash::Hash + Eq, T>(
    map: &HeldMap<K, T>,
    key: &(PathBuf, K),
    asked: u32,
) -> Option<Result<Arc<T>, Refusal>> {
    map.lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get(key)
        .filter(|held| held.asked == asked)
        .map(|held| held.answer.clone())
}

/// Holds `answer` for `key` at `asked`, replacing an older version's.
///
/// A refused load warns on `/logs` here and only here, so every refusal of
/// a version (host, bytes or parse) is one line, once per version.
fn hold<K: std::hash::Hash + Eq, T>(
    map: &HeldMap<K, T>,
    key: (PathBuf, K),
    asked: u32,
    check: &str,
    answer: Result<Arc<T>, Refusal>,
) -> Result<Arc<T>, Refusal> {
    if let Err(Refusal::Refused(why)) = &answer {
        note_refused(check, why);
    }
    let mut map = map.lock().unwrap_or_else(PoisonError::into_inner);
    let loads = map
        .get(&key)
        .map_or(1, |before| before.loads.saturating_add(1));
    let answer = answer.map_err(Refusal::held);
    map.insert(
        key,
        Held {
            asked,
            loads,
            answer: answer.clone(),
        },
    );
    answer
}

/// How many times `month` of `dir` was loaded (tests: a refusal is read once).
#[cfg(test)]
pub(crate) fn feed_loads(dir: &Path, month: YearMonth) -> u32 {
    FEED_MONTHS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get(&(dir.to_path_buf(), month))
        .map_or(0, |held| held.loads)
}

/// How many times `year` of `dir` was loaded (tests).
#[cfg(test)]
pub(crate) fn tick_loads(dir: &Path, year: u16) -> u32 {
    TICK_YEARS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get(&(dir.to_path_buf(), year))
        .map_or(0, |held| held.loads)
}

/// Every feed-reconcile month held, by folder and month: only the newest
/// version of each.
static FEED_MONTHS: std::sync::LazyLock<HeldMap<YearMonth, FeedMonth>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

/// The newest version of `month` in `dir`, parsed once per version.
///
/// `version` is the newest the folder's listing names. The first ask of a
/// version reads it through `cli::feed_reconcile::file_digest` (format,
/// cut and damage checked there), reads its bytes again as a regular file,
/// proves both reads were the same bytes by their BLAKE3, and parses it.
pub(crate) fn feed_month(
    dir: &Path,
    month: YearMonth,
    version: u32,
) -> Result<Arc<FeedMonth>, Refusal> {
    let key = (dir.to_path_buf(), month);
    if let Some(answer) = held(&FEED_MONTHS, &key, version) {
        return answer;
    }
    hold(
        &FEED_MONTHS,
        key,
        version,
        "feed-reconcile",
        load_feed(dir, month, version),
    )
}

/// One load of `month`'s newest version: the cold path, O(its bytes).
fn load_feed(dir: &Path, month: YearMonth, version: u32) -> Result<Arc<FeedMonth>, Refusal> {
    let started = Instant::now();
    let (file, digest) = cli::feed_reconcile::file_digest(dir, month).map_err(Refusal::Refused)?;
    let read_version = file
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(feed_name)
        .map_or(version, |(_, v)| v);
    let bytes = index::read_regular(&file).map_err(Refusal::Refused)?;
    if brutex_core::blake3::hash(&bytes) != digest {
        return Err(Refusal::Refused(format!(
            "`{}` changed between two reads of it: a version file is never rewritten, so this \
             one is refused",
            file.display()
        )));
    }
    let text = String::from_utf8(bytes)
        .map_err(|_| Refusal::Refused(format!("`{}` is not UTF-8 text", file.display())))?;
    let mut parsed = parse_feed(month, read_version, file.clone(), &text, digest)
        .map_err(|why| Refusal::Refused(format!("`{}` is refused: {why}", file.display())))?;
    parsed.load_micros = u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX);
    Ok(Arc::new(parsed))
}

/// One warning per refused version load: bounded by loads, never by rows or
/// requests that hit a held version.
fn note_refused(check: &str, why: &str) {
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::warn("api.feed_checks", "check file refused")
            .with("check", telemetry::Value::Str(check))
            .with("why", telemetry::Value::Str(why)),
    );
}

// ═══ The tick-to-second year ═══

/// The cells of the tick-to-second CSV this reader keeps, by name.
const TICK_KEPT: [&str; 20] = [
    "symbol",
    "day",
    "role",
    "status",
    "compare",
    "session",
    "zerodha_basis",
    "t_compared",
    "t_high_match",
    "t_low_match",
    "a_compared",
    "a_high_match",
    "a_low_match",
    "r_compared",
    "r_high_match",
    "r_low_match",
    "fill_compared",
    "fill_either_worse",
    "store_seconds",
    "source",
];

/// One row of a tick-to-second year, fixed size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TickRow {
    /// Where the row is in its file.
    pub(crate) place: Place,
    /// The share, numbered in [`TickYear::symbols`].
    pub(crate) symbol: u32,
    /// The day.
    pub(crate) day: Day,
    /// `role`, `status`, `compare`, `session`, `zerodha_basis` and `source`,
    /// interned.
    pub(crate) role: u16,
    /// See `role`.
    pub(crate) status: u16,
    /// See `role`.
    pub(crate) compare: u16,
    /// See `role`.
    pub(crate) session: u16,
    /// See `role`.
    pub(crate) basis: u16,
    /// See `role`.
    pub(crate) source: u16,
    /// Whether this is the row the year's index answers for the share-day
    /// (its last row; an `extra` row a `landing` row follows is not).
    pub(crate) counted: bool,
    /// Rule T, A and R Zerodha minutes compared, high matched, low matched.
    pub(crate) minutes: [[u32; 3]; 3],
    /// Stored seconds compared for the fill measure, and those worse under A.
    pub(crate) fill: [u32; 2],
    /// Seconds the store holds that day.
    pub(crate) store_seconds: u32,
}

/// What a set of tick-to-second rows says, counted over the COUNTED rows
/// (one per share-day); `rows` counts every row.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct TickTally {
    /// Every row, counted or not.
    pub(crate) rows: u32,
    /// Share-days: counted rows.
    pub(crate) share_days: u32,
    /// Share-days per status.
    pub(crate) status: Vec<u32>,
    /// Share-days per `compare`.
    pub(crate) compare: Vec<u32>,
    /// Share-days per Zerodha basis.
    pub(crate) basis: Vec<u32>,
    /// Rows per role (every row).
    pub(crate) role: Vec<u32>,
    /// Share-days per session.
    pub(crate) session: Vec<u32>,
    /// Rule T, A and R minutes compared, high matched, low matched.
    pub(crate) minutes: [[u64; 3]; 3],
    /// Fill seconds compared, and worse under A.
    pub(crate) fill: [u64; 2],
}

impl TickTally {
    fn add(&mut self, row: &TickRow) {
        self.rows = self.rows.saturating_add(1);
        bump(&mut self.role, row.role);
        if !row.counted {
            return;
        }
        self.share_days = self.share_days.saturating_add(1);
        bump(&mut self.status, row.status);
        bump(&mut self.compare, row.compare);
        bump(&mut self.basis, row.basis);
        bump(&mut self.session, row.session);
        for (sums, counts) in self.minutes.iter_mut().zip(row.minutes) {
            for (sum, count) in sums.iter_mut().zip(counts) {
                *sum = sum.saturating_add(u64::from(count));
            }
        }
        for (sum, count) in self.fill.iter_mut().zip(row.fill) {
            *sum = sum.saturating_add(u64::from(count));
        }
    }
}

/// One share's rows in one month of the year.
///
/// NOT consecutive in the CSV: the writer prints its rows in the order of
/// its key (day, symbol, role), so a month's shares interleave day by day.
/// The load groups them once, so a share's rows are one run of
/// [`TickYear::by_share`] (D-5197).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TickStock {
    /// The share, numbered in [`TickYear::symbols`].
    pub(crate) symbol: u32,
    /// Its first entry in [`TickYear::by_share`].
    pub(crate) first: u32,
    /// Its rows that month.
    pub(crate) count: u32,
    /// What they say.
    pub(crate) tally: TickTally,
}

/// One month of a tick-to-second year.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct TickMonth {
    /// The month, 1 to 12.
    pub(crate) month: u8,
    /// What its rows say.
    pub(crate) tally: TickTally,
    /// Its shares, in symbol byte order (the writer's order of a day's rows).
    pub(crate) stocks: Vec<TickStock>,
    slots: HashMap<u32, u32>,
    /// While loading: each share's row numbers, by slot; emptied by
    /// [`group_by_share`].
    members: Vec<Vec<u32>>,
}

impl TickMonth {
    /// The share numbered `symbol`, when it has rows this month.
    pub(crate) fn stock(&self, symbol: u32) -> Option<&TickStock> {
        self.slots
            .get(&symbol)
            .and_then(|slot| self.stocks.get(usize::try_from(*slot).ok()?))
    }
}

/// One tick-to-second year version, parsed once.
#[derive(Debug)]
pub(crate) struct TickYear {
    /// The year.
    pub(crate) year: u16,
    /// The version read.
    pub(crate) version: u32,
    /// The version's CSV.
    pub(crate) file: PathBuf,
    /// Its index.
    pub(crate) index_file: PathBuf,
    /// BLAKE3 of the CSV, as the index records it and as read.
    pub(crate) csv_digest: [u8; 32],
    /// BLAKE3 of the index file.
    pub(crate) index_digest: [u8; 32],
    /// The highest day segment the version folded.
    pub(crate) segments_through: u64,
    /// The CSV's size.
    pub(crate) bytes: u64,
    /// The column names.
    pub(crate) columns: Vec<String>,
    /// Every row, in file order.
    pub(crate) rows: Vec<TickRow>,
    /// Row numbers grouped by month, then share (symbol byte order), then
    /// file order: a share-month is one run of it.
    pub(crate) by_share: Vec<u32>,
    /// The shares, numbered in first-seen order.
    pub(crate) symbols: Words,
    /// The twelve months, January first; a month with no row has none.
    pub(crate) months: Vec<TickMonth>,
    /// Roles.
    pub(crate) roles: Words,
    /// Statuses.
    pub(crate) statuses: Words,
    /// `compare` words.
    pub(crate) compares: Words,
    /// Sessions.
    pub(crate) sessions: Words,
    /// Zerodha bases.
    pub(crate) bases: Words,
    /// Sources.
    pub(crate) sources: Words,
    /// What every row says.
    pub(crate) tally: TickTally,
    /// How long the one-pass load took, in microseconds.
    pub(crate) load_micros: u64,
}

impl TickYear {
    /// The month `month` (1 to 12), when it has rows.
    pub(crate) fn month(&self, month: u8) -> Option<&TickMonth> {
        self.months
            .get(usize::from(month).checked_sub(1)?)
            .filter(|held| held.tally.rows > 0)
    }

    /// The months that have rows, in order.
    pub(crate) fn held_months(&self) -> impl Iterator<Item = &TickMonth> {
        self.months.iter().filter(|held| held.tally.rows > 0)
    }

    /// The row numbers of `stock`, in file order: one slice.
    pub(crate) fn rows_of(&self, stock: &TickStock) -> &[u32] {
        let first = usize::try_from(stock.first).unwrap_or(usize::MAX);
        let count = usize::try_from(stock.count).unwrap_or(0);
        self.by_share
            .get(first..first.saturating_add(count))
            .unwrap_or(&[])
    }

    /// The row numbered `at`: one index.
    pub(crate) fn row(&self, at: u32) -> Option<&TickRow> {
        self.rows.get(usize::try_from(at).ok()?)
    }
}

/// A day of `year`, `YYYY-MM-DD`, or why not.
fn day_of_year(text: &str, year: u16) -> Result<Day, String> {
    let wrong = || format!("`day` holds `{text}`, not a day of {year}");
    let bytes = text.as_bytes();
    if text.len() != 10 || bytes.get(4) != Some(&b'-') || bytes.get(7) != Some(&b'-') {
        return Err(wrong());
    }
    let number = |range: std::ops::Range<usize>| -> Option<u16> {
        let part = text.get(range)?;
        part.bytes()
            .all(|b| b.is_ascii_digit())
            .then(|| part.parse().ok())?
    };
    let (Some(y), Some(m), Some(d)) = (number(0..4), number(5..7), number(8..10)) else {
        return Err(wrong());
    };
    if y != year {
        return Err(wrong());
    }
    Day::new(
        y,
        u8::try_from(m).map_err(|_| wrong())?,
        u8::try_from(d).map_err(|_| wrong())?,
    )
    .map_err(|_| wrong())
}

/// One tick row's cells.
fn tick_row(
    parsed: &mut TickYear,
    index: &index::YearIndex,
    cells: &[&str],
    at: &[usize; TICK_KEPT.len()],
    place: Place,
    row_number: u32,
) -> Result<TickRow, String> {
    let cell = |i: usize| -> &str {
        at.get(i)
            .and_then(|position| cells.get(*position))
            .copied()
            .unwrap_or_default()
    };
    let symbol = cell(0);
    if symbol.is_empty() {
        return Err("an empty `symbol`".to_owned());
    }
    let day = day_of_year(cell(1), parsed.year)?;
    let count = |i: usize| -> Result<u32, String> {
        let text = cell(i);
        let name = TICK_KEPT.get(i).copied().unwrap_or_default();
        if text.is_empty() {
            return Ok(0);
        }
        text.parse()
            .map_err(|_| format!("column `{name}` holds `{text}`"))
    };
    // THE INDEX IS THE AUTHORITY on which row stands for a share-day: its
    // record names the row's line, so this row is counted exactly when the
    // index points at it, and a row whose share-day the index does not hold
    // means the two files disagree.
    let answer = index.status_of(symbol, day).map_err(|why| {
        format!("the index answers {why:?} for `{symbol}` {day}, which this row names")
    })?;
    let id = u32::from(parsed.symbols.id(symbol)?);
    Ok(TickRow {
        place,
        symbol: id,
        day,
        role: parsed.roles.id(cell(2))?,
        status: parsed.statuses.id(cell(3))?,
        compare: parsed.compares.id(cell(4))?,
        session: parsed.sessions.id(cell(5))?,
        basis: parsed.bases.id(cell(6))?,
        source: parsed.sources.id(cell(19))?,
        counted: answer.row == row_number,
        minutes: [
            [count(7)?, count(8)?, count(9)?],
            [count(10)?, count(11)?, count(12)?],
            [count(13)?, count(14)?, count(15)?],
        ],
        fill: [count(16)?, count(17)?],
        store_seconds: count(18)?,
    })
}

/// Files `row` (number `index` in the rows) under its month and share.
fn file_tick(parsed: &mut TickYear, row: &TickRow, index: u32) -> Result<(), String> {
    let month = row.day.month();
    // ONE SLOT PER MONTH, found by its number; a share's slot in it by one
    // probe. The writer prints rows by (day, symbol, role), so a share's rows
    // of one month are spread over its days: each is filed under its share
    // here and grouped into one run by `group_by_share` after the last row.
    let held = parsed
        .months
        .get_mut(usize::from(month).saturating_sub(1))
        .ok_or_else(|| format!("month {month} is not a month"))?;
    held.tally.add(row);
    let slot = if let Some(slot) = held.slots.get(&row.symbol) {
        usize::try_from(*slot).map_err(|_| "too many shares".to_owned())?
    } else {
        let slot = held.stocks.len();
        held.slots.insert(
            row.symbol,
            u32::try_from(slot).map_err(|_| "too many shares".to_owned())?,
        );
        held.stocks.push(TickStock {
            symbol: row.symbol,
            first: 0,
            count: 0,
            tally: TickTally::default(),
        });
        held.members.push(Vec::new());
        slot
    };
    let (Some(stock), Some(members)) = (held.stocks.get_mut(slot), held.members.get_mut(slot))
    else {
        return Err(format!("month {month}: a share's slot {slot} is not held"));
    };
    stock.tally.add(row);
    stock.count = stock.tally.rows;
    members.push(index);
    Ok(())
}

/// After the last row: each month's shares in symbol byte order, and each
/// share-month one run of [`TickYear::by_share`]. One pass over the rows and
/// one sort of each month's shares (at most the F&O pool), once per load.
fn group_by_share(parsed: &mut TickYear) -> Result<(), String> {
    let mut by_share = Vec::with_capacity(parsed.rows.len());
    for held in &mut parsed.months {
        let mut order: Vec<usize> = (0..held.stocks.len()).collect();
        order.sort_by(|a, b| {
            let name = |slot: &usize| {
                held.stocks
                    .get(*slot)
                    .map_or("", |stock| {
                        parsed
                            .symbols
                            .name(u16::try_from(stock.symbol).unwrap_or(u16::MAX))
                    })
                    .as_bytes()
            };
            name(a).cmp(name(b))
        });
        let mut members = std::mem::take(&mut held.members);
        let mut stocks = Vec::with_capacity(order.len());
        held.slots.clear();
        for slot in order {
            let (Some(stock), Some(rows)) = (held.stocks.get(slot), members.get_mut(slot)) else {
                return Err(format!(
                    "month {}: a share's slot {slot} is not held",
                    held.month
                ));
            };
            let mut stock = stock.clone();
            stock.first = u32::try_from(by_share.len()).map_err(|_| "too many rows".to_owned())?;
            stock.count = u32::try_from(rows.len()).map_err(|_| "too many rows".to_owned())?;
            held.slots.insert(
                stock.symbol,
                u32::try_from(stocks.len()).map_err(|_| "too many shares".to_owned())?,
            );
            by_share.append(rows);
            stocks.push(stock);
        }
        held.stocks = stocks;
    }
    parsed.by_share = by_share;
    Ok(())
}

/// Parses one tick-to-second version's CSV against its index.
fn parse_tick(
    index: &index::YearIndex,
    file: PathBuf,
    text: &str,
    csv_digest: [u8; 32],
) -> Result<TickYear, String> {
    let mut parsed = TickYear {
        year: index.year(),
        version: index.version(),
        file,
        index_file: index.path().to_path_buf(),
        csv_digest,
        index_digest: index.file_digest(),
        segments_through: index.segments_through(),
        bytes: u64::try_from(text.len()).unwrap_or(u64::MAX),
        columns: Vec::new(),
        rows: Vec::new(),
        by_share: Vec::new(),
        symbols: Words::default(),
        months: (1..=12)
            .map(|month| TickMonth {
                month,
                ..TickMonth::default()
            })
            .collect(),
        roles: Words::default(),
        statuses: Words::default(),
        compares: Words::default(),
        sessions: Words::default(),
        bases: Words::default(),
        sources: Words::default(),
        tally: TickTally::default(),
        load_micros: 0,
    };
    let mut lines = lines_of(text).enumerate();
    let Some((_, (_, header))) = lines.next() else {
        return Err("it is empty".to_owned());
    };
    if header != cli::tick_second_check::HEADER {
        return Err("its header is not this build's `cli::tick_second_check::HEADER`".to_owned());
    }
    parsed.columns = header.split(',').map(str::to_owned).collect();
    let mut at = [0_usize; TICK_KEPT.len()];
    for (slot, name) in at.iter_mut().zip(TICK_KEPT) {
        *slot = parsed
            .columns
            .iter()
            .position(|column| column == name)
            .ok_or_else(|| format!("no column `{name}`"))?;
    }
    let width = parsed.columns.len();
    // THE WRITER'S ORDER, held to: `cli::tick_second_check` prints a version
    // from a map keyed by (day, symbol, role), so each row's key is above the
    // one before it. Anything else is not this writer's file (D-5197).
    let mut previous: Option<(&str, &str, &str)> = None;
    for (number, (start, line)) in lines {
        let cells: Vec<&str> = line.split(',').collect();
        if cells.len() != width {
            return Err(format!(
                "line {}: {} cells, the header names {width}",
                number + 1,
                cells.len()
            ));
        }
        let row_number = u32::try_from(number).map_err(|_| "too many rows".to_owned())?;
        let place = place_of(number + 1, start, line)?;
        let row = tick_row(&mut parsed, index, &cells, &at, place, row_number)
            .map_err(|why| format!("line {}: {why}", number + 1))?;
        let cell = |kept: usize| {
            at.get(kept)
                .and_then(|column| cells.get(*column))
                .copied()
                .unwrap_or("")
        };
        let key = (cell(1), cell(0), cell(2));
        if let Some(before) = previous
            && before >= key
        {
            return Err(format!(
                "line {}: ({}, {}, {}) is not above the row before it, ({}, {}, {}): the writer \
                 prints rows in (day, symbol, role) order, each key once",
                number + 1,
                key.0,
                key.1,
                key.2,
                before.0,
                before.1,
                before.2
            ));
        }
        previous = Some(key);
        let at_row = u32::try_from(parsed.rows.len()).map_err(|_| "too many rows".to_owned())?;
        parsed.tally.add(&row);
        file_tick(&mut parsed, &row, at_row)
            .map_err(|why| format!("line {}: {why}", number + 1))?;
        parsed.rows.push(row);
    }
    group_by_share(&mut parsed)?;
    Ok(parsed)
}

/// Every tick-to-second year held, by folder and year: only the newest.
static TICK_YEARS: std::sync::LazyLock<HeldMap<u16, TickYear>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

/// The newest version of `year` in `dir`, parsed once per version.
///
/// The index is read through `cli::tick_second_check::index::newest` (every
/// header field checked there); the CSV beside it must hash to the BLAKE3
/// the index records, so the two are one version.
pub(crate) fn tick_year(dir: &Path, year: u16, version: u32) -> Result<Arc<TickYear>, Refusal> {
    let key = (dir.to_path_buf(), year);
    if let Some(answer) = held(&TICK_YEARS, &key, version) {
        return answer;
    }
    hold(
        &TICK_YEARS,
        key,
        version,
        "tick-to-second",
        load_tick(dir, year),
    )
}

/// One load of `year`'s newest version: the cold path, O(its bytes).
fn load_tick(dir: &Path, year: u16) -> Result<Arc<TickYear>, Refusal> {
    let started = Instant::now();
    let index = index::newest(dir, year)
        .map_err(Refusal::Refused)?
        .ok_or_else(|| {
            Refusal::Absent(format!(
                "`{}` holds no tick-to-second index of {year}",
                dir.display()
            ))
        })?;
    let file = dir.join(format!("tick-to-second-{year}.v{:03}.csv", index.version()));
    let bytes = index::read_regular(&file).map_err(Refusal::Refused)?;
    let digest = brutex_core::blake3::hash(&bytes);
    if digest != index.csv_digest() {
        return Err(Refusal::Refused(format!(
            "`{}` is not the CSV its index `{}` records (BLAKE3 differs): the two are not one \
             version",
            file.display(),
            index.path().display()
        )));
    }
    let text = String::from_utf8(bytes)
        .map_err(|_| Refusal::Refused(format!("`{}` is not UTF-8 text", file.display())))?;
    let mut parsed = parse_tick(&index, file.clone(), &text, digest)
        .map_err(|why| Refusal::Refused(format!("`{}` is refused: {why}", file.display())))?;
    parsed.load_micros = u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX);
    Ok(Arc::new(parsed))
}

// ═══ The source row behind a number ═══

/// The line at `place` of `file`, read again by one positional read and
/// proved the same line by its FNV-1a, with each cell named by `columns`.
///
/// # Errors
///
/// A file that cannot be opened as a regular file, a short read, or a line
/// whose bytes are not the ones loaded (the file was rewritten, which the
/// append-only law forbids: refused, never served as the old row).
pub(crate) fn source_row(
    file: &Path,
    place: Place,
    columns: &[String],
) -> Result<Vec<(String, String)>, Refusal> {
    use std::os::unix::fs::FileExt as _;
    let handle = store::open_flags::open_read_no_follow(file)
        .map_err(|why| Refusal::Refused(format!("`{}`: {why}", file.display())))?;
    let mut bytes = vec![0_u8; usize::try_from(place.len).unwrap_or(0)];
    handle.read_exact_at(&mut bytes, place.at).map_err(|why| {
        Refusal::Refused(format!("`{}` line {}: {why}", file.display(), place.line))
    })?;
    let text = String::from_utf8(bytes)
        .ok()
        .filter(|text| brutex_core::universe::fnv1a(text) == place.hash);
    let Some(text) = text else {
        return Err(Refusal::Refused(format!(
            "`{}` line {} is not the line this server read from it: a version file is never \
             rewritten, so it is refused",
            file.display(),
            place.line
        )));
    };
    Ok(columns
        .iter()
        .cloned()
        .zip(text.split(',').map(str::to_owned))
        .collect())
}

/// Forgets every answer held for a folder under `under` (tests that rewrite
/// a version in place, which the law forbids, to prove the reader refuses
/// it). Only this test's own folders, so tests running beside it keep theirs.
#[cfg(test)]
pub(crate) fn forget_held(under: &Path) {
    FEED_MONTHS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .retain(|(dir, _), _| !dir.starts_with(under));
    TICK_YEARS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .retain(|(dir, _), _| !dir.starts_with(under));
}

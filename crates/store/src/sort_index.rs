//! The sort index — `<yyyy-mm>.srt` beside a month's `.bin` — and the reads
//! that page a value-ordered window without reading every bar. D-3349, WP-39;
//! DB-N14.
//!
//! # What it is for
//!
//! `/bars/window.json` ordered by a price, the volume or the open interest
//! answered by READING EVERY KEPT ROW: 43,942,544 bars for one-second NIFTY
//! over 97 months, 4.3 to 7.4 s and 8 to 12 GB a request (DB-N14), and since
//! D-3326 a refusal past 24 million rows. No index the store had could answer
//! "the fifty largest closes", because the store is indexed by time.
//!
//! This file is that index, one per month, at its own format version and
//! stride (`CLAUDE.md` §4: a new field is a new file version at its own
//! stride; §3 rule 8: the bar file is never touched).
//!
//! # The shape
//!
//! A 64-byte header binds the file to its month, rung and symbol. After it,
//! one or more RUNS, each covering a contiguous range of the month's committed
//! rows `[start, end)`; the first run starts at row 0 and each later run
//! starts where the one before it ended, so the runs together cover a PREFIX
//! of the month — its "coverage". A run holds, for each of the six sortable
//! columns ([`Column::ALL`]), the run's rows ordered by `(value, time)`, as
//! fixed-width entries `(value, second of the month)`, plus a fence table —
//! every [`FENCE`]-th entry, in one place, with each block's CRC-32C — and a
//! summary of the run's extremes, so a window's extremes cost one header read
//! per run and not one read per bar.
//!
//! Open interest's null ([`OI_NULL`]) sorts as no number: its entries are the
//! column's first `nulls`, in time order, and are never compared by value.
//!
//! # Append-only
//!
//! A run is written strictly AFTER the bars it orders are committed, and is
//! appended past the last valid run; the only bytes an append may overwrite
//! are those past the last valid run — a run that was being written when a
//! process died, which no reader ever used (the rule a bar file's next append
//! applies to the bytes past its counter, D-0189). A file is otherwise never
//! modified: a rebuild writes a NEW file beside it and renames it over the
//! name, so a reader holding the old one keeps reading the old one whole.
//! Committed bars never change, so a run stays true for as long as the bars it
//! was built from are the file's bars; a reader checks that binding (the
//! stamps of each run's first and last row) and checks every value it serves
//! against the bar it reads.
//!
//! # What this module is, and is not
//!
//! The byte codec, the run builder, the walk that finds the valid runs and the
//! read-only reader. The writer's half — appending a run after a commit, and
//! rebuilding a month whole — is `crate::file`'s, beside the bar I/O it must
//! stay ordered with.

use std::fmt;
use std::fs::File;
use std::io::{self, ErrorKind};
use std::os::unix::fs::FileExt;
use std::path::{Path, PathBuf};

use crate::crc::{crc32c, crc32c_extend, crc32c_split};
use crate::format::{Bar, OI_NULL};
use crate::path::YearMonth;

/// `b"BRUTEXS1"`: the bar family's `BRUTEX` prefix, `S` for sort, version 1.
pub const MAGIC: [u8; 8] = *b"BRUTEXS1";

/// The magic each run opens with.
pub const RUN_MAGIC: [u8; 8] = *b"BRUTEXSR";

/// The one version this build reads and writes.
pub const VERSION: u16 = 1;

/// The file header: 64 bytes at offset 0.
pub const HEADER_LEN: u64 = 64;

/// One run's header: 512 bytes, checksummed whole.
pub const RUN_HEADER_LEN: u64 = 512;

/// Entries per block, and the stride of the fence table: every `FENCE`-th
/// entry of a column is its block's fence. A block is one read of at most
/// `FENCE × 12` bytes.
pub const FENCE: u64 = 256;

/// The most runs a reader walks. A writer rebuilds a month whole well before
/// it ([`COMPACT_AT`]), so a file holding more is refused by name.
pub const MAX_RUNS: usize = 64;

/// A writer that finds this many runs rebuilds the month as one run, so a
/// reader's cost per month stays bounded however often the month was
/// appended to.
pub const COMPACT_AT: usize = 32;

/// An entry whose value is an offset from the column's base in four bytes,
/// then the second of the month in four.
pub const NARROW: u8 = 8;

/// An entry whose value is the whole `i64`, then the second of the month.
pub const WIDE: u8 = 12;

/// Microseconds in a second.
const MICROS: i64 = 1_000_000;

/// Bytes of one column's description in a run header.
const COLUMN_LEN: u64 = 48;

/// Where the column descriptions start in a run header.
const COLUMNS_AT: u64 = 48;

/// Where the extremes start in a run header.
const EXTREMES_AT: u64 = COLUMNS_AT + 6 * COLUMN_LEN;

/// Where a run header's checksum is.
const RUN_CRC_AT: u64 = RUN_HEADER_LEN - 4;

const _: () = assert!(EXTREMES_AT + 6 * 16 <= RUN_CRC_AT);

/// Where a run header holds its body's checksum.
const BODY_CRC_AT: usize = 432;

/// Where a run header holds its [`RunKind`].
const KIND_AT: usize = 436;

const _: () = assert!(EXTREMES_AT + 6 * 16 <= BODY_CRC_AT as u64);
const _: () = assert!((KIND_AT as u64) < RUN_CRC_AT);

/// Bytes one read of a body check takes.
const BODY_READ: usize = 1 << 20;

/// A column the grid sorts on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Column {
    /// The open.
    Open,
    /// The high.
    High,
    /// The low.
    Low,
    /// The close.
    Close,
    /// Traded volume.
    Volume,
    /// Open interest; [`OI_NULL`] is no number.
    OpenInterest,
}

impl Column {
    /// Every column, in the order a run stores them.
    pub const ALL: [Self; 6] = [
        Self::Open,
        Self::High,
        Self::Low,
        Self::Close,
        Self::Volume,
        Self::OpenInterest,
    ];

    /// Its place in [`Self::ALL`] and in a run.
    #[must_use]
    pub const fn index(self) -> usize {
        match self {
            Self::Open => 0,
            Self::High => 1,
            Self::Low => 2,
            Self::Close => 3,
            Self::Volume => 4,
            Self::OpenInterest => 5,
        }
    }

    /// Its place as one byte, for a checksum key.
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            Self::Open => 0,
            Self::High => 1,
            Self::Low => 2,
            Self::Close => 3,
            Self::Volume => 4,
            Self::OpenInterest => 5,
        }
    }

    /// The value this column orders `bar` by.
    #[must_use]
    pub const fn of(self, bar: &Bar) -> i64 {
        match self {
            Self::Open => bar.open,
            Self::High => bar.high,
            Self::Low => bar.low,
            Self::Close => bar.close,
            Self::Volume => bar.volume,
            Self::OpenInterest => bar.open_interest,
        }
    }

    /// Whether [`OI_NULL`] in this column is no number rather than a value.
    #[must_use]
    pub const fn nullable(self) -> bool {
        matches!(self, Self::OpenInterest)
    }

    /// The grid's own word for the column.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Open => "o",
            Self::High => "h",
            Self::Low => "l",
            Self::Close => "c",
            Self::Volume => "v",
            Self::OpenInterest => "oi",
        }
    }
}

/// One entry of a column: a value and the second of the month its bar opens
/// at. A null entry's `v` is [`OI_NULL`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entry {
    /// The column's value, paisa or a count.
    pub v: i64,
    /// Seconds from the month's start (00:00 IST on its first day).
    pub sec: u32,
}

impl Entry {
    /// The bar's stamp, UTC micros, given the month's start.
    #[must_use]
    pub fn ts(self, origin_micros: i64) -> i64 {
        origin_micros.saturating_add(i64::from(self.sec).saturating_mul(MICROS))
    }
}

/// What a sort index file is bound to: its rung, its symbol and its month.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Shape {
    /// The bar file's timeframe, from its header.
    pub timeframe_secs: u32,
    /// The bar file's symbol id, from its header.
    pub symbol_id: u32,
    /// 00:00 IST on the first of the month, UTC micros: second 0.
    pub origin_micros: i64,
}

impl Shape {
    /// The shape of `month` at `timeframe_secs` for `symbol_id`.
    #[must_use]
    pub const fn of(month: YearMonth, timeframe_secs: u32, symbol_id: u32) -> Self {
        Self {
            timeframe_secs,
            symbol_id,
            origin_micros: month.ist_bounds_micros().0,
        }
    }

    /// The 64-byte header this shape is written as.
    ///
    /// | Offset | Size | Field |
    /// |---|---|---|
    /// | 0 | 8 | magic `BRUTEXS1` |
    /// | 8 | 2 | version `1` |
    /// | 10 | 2 | columns `6` |
    /// | 12 | 4 | timeframe seconds |
    /// | 16 | 4 | symbol id |
    /// | 20 | 4 | fence stride `256` |
    /// | 24 | 8 | second 0, UTC micros |
    /// | 32 | 28 | reserved, zero |
    /// | 60 | 4 | CRC-32C over bytes `0..60` |
    #[must_use]
    pub fn encode(&self) -> [u8; 64] {
        let mut out = [0u8; 64];
        put(&mut out, 0, &MAGIC);
        put(&mut out, 8, &VERSION.to_le_bytes());
        put(&mut out, 10, &6u16.to_le_bytes());
        put(&mut out, 12, &self.timeframe_secs.to_le_bytes());
        put(&mut out, 16, &self.symbol_id.to_le_bytes());
        put(
            &mut out,
            20,
            &u32::try_from(FENCE).unwrap_or(u32::MAX).to_le_bytes(),
        );
        put(&mut out, 24, &self.origin_micros.to_le_bytes());
        let crc = crc32c(out.get(..60).unwrap_or_default());
        put(&mut out, 60, &crc.to_le_bytes());
        out
    }

    /// Whether `bytes` is the header this shape writes, and if not, why: the
    /// checksum first, then the magic and version, then every other byte.
    ///
    /// # Errors
    ///
    /// [`Why::Header`] naming the first thing that disagrees.
    pub fn check(&self, bytes: &[u8; 64]) -> Result<(), Why> {
        let crc = crc32c(bytes.get(..60).unwrap_or_default());
        if crc.to_le_bytes() != take::<4>(bytes, 60) {
            return Err(Why::Header("its header fails its checksum"));
        }
        if take::<8>(bytes, 0) != MAGIC {
            return Err(Why::Header("its magic is not BRUTEXS1"));
        }
        if u16::from_le_bytes(take::<2>(bytes, 8)) != VERSION {
            return Err(Why::Header("its version is not 1"));
        }
        if self.encode() != *bytes {
            return Err(Why::Header(
                "its rung, symbol, month or fence stride is not this bar file's",
            ));
        }
        Ok(())
    }
}

/// What wrote a run, which decides what a walk checks of it.
///
/// # Why a walk checks the body of the last appended run, and only that one
///
/// An appended run is written past the last valid run and synced once, at its
/// end; the writer appends the next one only after that sync returned. So a
/// crash can tear only the run being written, which is the LAST one, and it
/// can leave that run's header on disk without every byte after it — a header
/// that decodes over entries that are not there. Every block's own checksum
/// would refuse those entries when read, and a request would fail on them
/// long after the crash. The walk therefore checks the last appended run's
/// whole body against the checksum in its header, and a run that fails is
/// torn: never read, and appended over by the next writer. Every earlier run
/// was synced whole before it, and a whole image is synced before it is
/// renamed into place, so neither is read twice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RunKind {
    /// Appended past the last valid run after a commit.
    #[default]
    Appended,
    /// A whole month written into a new file, synced, and renamed over the
    /// name: whole by construction.
    Whole,
}

impl RunKind {
    /// Its byte in a run header.
    const fn code(self) -> u8 {
        match self {
            Self::Appended => 0,
            Self::Whole => 1,
        }
    }
}

/// One column of one run, as its run header describes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ColumnInfo {
    /// Bytes per entry: [`NARROW`] or [`WIDE`].
    pub width: u8,
    /// Null entries, which come first and are in time order.
    pub nulls: u64,
    /// The smallest non-null value; a narrow entry is an offset from it.
    pub base: i64,
    /// The largest non-null value.
    pub max: i64,
    /// Where the column's section starts, from the run's first byte.
    pub offset: u64,
    /// Blocks, and fences: `ceil(rows / FENCE)`.
    pub blocks: u64,
}

impl ColumnInfo {
    /// Where the block checksums start, from the run's first byte.
    const fn crcs_at(&self) -> u64 {
        self.offset + self.blocks * self.width as u64
    }

    /// Where the fence table's own checksum is.
    const fn table_crc_at(&self) -> u64 {
        self.crcs_at() + self.blocks * 4
    }

    /// Where the entries start, from the run's first byte.
    #[must_use]
    pub const fn entries_at(&self) -> u64 {
        self.table_crc_at() + 4
    }

    /// The section's length for a run of `rows`.
    const fn section_len(&self, rows: u64) -> u64 {
        self.blocks * (self.width as u64 + 4) + 4 + rows * self.width as u64
    }
}

/// One extreme of a run: the value and the stamp of the earliest bar that
/// holds it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Extreme {
    /// The value, paisa or a count.
    pub v: i64,
    /// The earliest bar's stamp, UTC micros.
    pub ts: i64,
}

/// A run's extremes, each with the EARLIEST bar that holds it — the tie rule
/// `/bars/window.json`'s extremes keep (D-3326).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RunExtremes {
    /// The largest close.
    pub close_max: Extreme,
    /// The smallest close.
    pub close_min: Extreme,
    /// The largest high.
    pub high_max: Extreme,
    /// The smallest low.
    pub low_min: Extreme,
    /// The widest `high - low` (saturating).
    pub range_max: Extreme,
    /// The heaviest volume, whether or not the series trades any.
    pub volume_max: Extreme,
}

impl RunExtremes {
    /// The six, in the order a run header stores them.
    const fn all(&self) -> [Extreme; 6] {
        [
            self.close_max,
            self.close_min,
            self.high_max,
            self.low_min,
            self.range_max,
            self.volume_max,
        ]
    }
}

/// One run, as its header describes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunInfo {
    /// Where the run starts in the file.
    pub offset: u64,
    /// Its length in bytes, header included.
    pub len: u64,
    /// The first bar row it orders.
    pub start: u64,
    /// One past the last.
    pub end: u64,
    /// The stamp of row `start`.
    pub ts_first: i64,
    /// The stamp of row `end - 1`.
    pub ts_last: i64,
    /// Its columns, in [`Column::ALL`] order.
    pub columns: [ColumnInfo; 6],
    /// Its extremes.
    pub extremes: RunExtremes,
    /// What wrote it.
    pub kind: RunKind,
    /// CRC-32C over every byte after its header, keyed by its first row.
    pub body_crc: u32,
}

impl RunInfo {
    /// Bar rows the run orders.
    #[must_use]
    pub const fn rows(&self) -> u64 {
        self.end - self.start
    }

    /// One column's description.
    #[must_use]
    pub const fn column(&self, column: Column) -> &ColumnInfo {
        let [open, high, low, close, volume, interest] = &self.columns;
        match column {
            Column::Open => open,
            Column::High => high,
            Column::Low => low,
            Column::Close => close,
            Column::Volume => volume,
            Column::OpenInterest => interest,
        }
    }
}

/// Why a month's sort index cannot serve a read, or a run cannot be built —
/// said, never hidden.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Why {
    /// No `.srt` beside the month.
    Absent,
    /// The file header is not the one this month implies.
    Header(&'static str),
    /// The run at this offset is refused.
    Run {
        /// Where it starts.
        offset: u64,
        /// What is wrong with it.
        why: &'static str,
    },
    /// More runs than a reader walks.
    TooManyRuns(usize),
    /// The runs do not describe the bar file's committed rows.
    Stale(String),
    /// A block of entries fails its checksum or would not read whole.
    Block {
        /// The run's first row.
        start: u64,
        /// The column.
        column: Column,
        /// The block.
        block: u64,
    },
    /// A column's fence table fails its checksum or would not read whole.
    Fences {
        /// The run's first row.
        start: u64,
        /// The column.
        column: Column,
    },
    /// A bar's stamp is not a whole second, so no entry can name it.
    NotWholeSecond {
        /// The row.
        row: u64,
        /// The stamp.
        ts_micros: i64,
    },
    /// A bar's stamp lies outside the month.
    Outside {
        /// The row.
        row: u64,
        /// The stamp.
        ts_micros: i64,
    },
    /// A run with no rows: never written.
    Empty,
    /// Reading or writing the file was refused.
    Unreadable(String),
    /// The body of a run — every byte after its header — fails the checksum
    /// the header holds: a fence table or a block of it is damaged, and a
    /// reader that reaches the damage refuses the month.
    Body {
        /// The run's first row.
        start: u64,
        /// One past its last.
        end: u64,
    },
}

impl fmt::Display for Why {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Absent => f.write_str("the month has no .srt sort index"),
            Self::Header(why) => write!(f, "the .srt sort index is refused: {why}"),
            Self::Run { offset, why } => {
                write!(f, "the .srt run at byte {offset} is refused: {why}")
            }
            Self::TooManyRuns(runs) => write!(
                f,
                "the .srt sort index holds more than {MAX_RUNS} runs ({runs} read); rebuild it"
            ),
            Self::Stale(why) => write!(
                f,
                "the .srt sort index does not describe the committed bars: {why}"
            ),
            Self::Block {
                start,
                column,
                block,
            } => write!(
                f,
                "the .srt block {block} of column {} in the run from row {start} fails its \
                 checksum or would not read",
                column.as_str()
            ),
            Self::Fences { start, column } => write!(
                f,
                "the .srt fence table of column {} in the run from row {start} fails its \
                 checksum or would not read",
                column.as_str()
            ),
            Self::NotWholeSecond { row, ts_micros } => write!(
                f,
                "bar {row} is stamped {ts_micros}, not a whole second, so no sort entry can \
                 name it"
            ),
            Self::Outside { row, ts_micros } => {
                write!(f, "bar {row} is stamped {ts_micros}, outside the month")
            }
            Self::Empty => f.write_str("a sort run of no rows is never written"),
            Self::Unreadable(why) => write!(f, "the .srt sort index would not read: {why}"),
            Self::Body { start, end } => write!(
                f,
                "the body of the .srt run ordering rows {start}..{end} fails the checksum its \
                 header holds: a fence table or a block of it is damaged"
            ),
        }
    }
}

impl std::error::Error for Why {}

/// The second of the month a bar at `ts_micros` opens at, or why it has none.
fn second_of(shape: &Shape, row: u64, ts_micros: i64) -> Result<u32, Why> {
    let offset = ts_micros
        .checked_sub(shape.origin_micros)
        .filter(|offset| *offset >= 0)
        .ok_or(Why::Outside { row, ts_micros })?;
    if offset % MICROS != 0 {
        return Err(Why::NotWholeSecond { row, ts_micros });
    }
    u32::try_from(offset / MICROS).map_err(|_| Why::Outside { row, ts_micros })
}

/// Folds one bar into the run's extremes; the earlier bar keeps a tie.
fn fold(held: &mut Option<RunExtremes>, bar: &Bar) {
    let at = |v: i64| Extreme {
        v,
        ts: bar.ts_micros,
    };
    let range = bar.high.saturating_sub(bar.low);
    let Some(held) = held.as_mut() else {
        *held = Some(RunExtremes {
            close_max: at(bar.close),
            close_min: at(bar.close),
            high_max: at(bar.high),
            low_min: at(bar.low),
            range_max: at(range),
            volume_max: at(bar.volume),
        });
        return;
    };
    // STRICTLY better only: the rows arrive in time order, so the earlier
    // bar already holds a tie.
    if bar.close > held.close_max.v {
        held.close_max = at(bar.close);
    }
    if bar.close < held.close_min.v {
        held.close_min = at(bar.close);
    }
    if bar.high > held.high_max.v {
        held.high_max = at(bar.high);
    }
    if bar.low < held.low_min.v {
        held.low_min = at(bar.low);
    }
    if range > held.range_max.v {
        held.range_max = at(range);
    }
    if bar.volume > held.volume_max.v {
        held.volume_max = at(bar.volume);
    }
}

/// One column's entries in run order: the nulls in time order, then every
/// value ordered by `(value, second)`. Seconds are unique within a month
/// (stamps strictly increase), so the order is total.
fn ordered(column: Column, bars: &[Bar], secs: &[u32]) -> (Vec<Entry>, u64) {
    let mut nulls: Vec<Entry> = Vec::new();
    let mut values: Vec<Entry> = Vec::with_capacity(bars.len());
    for (bar, &sec) in bars.iter().zip(secs) {
        let v = column.of(bar);
        if column.nullable() && v == OI_NULL {
            nulls.push(Entry { v: OI_NULL, sec });
        } else {
            values.push(Entry { v, sec });
        }
    }
    // THE ONE SORT THIS FORMAT EXISTS TO PAY ONCE, at write time, over the
    // rows of one run, so that no read ever pays it again (D-3349).
    values.sort_unstable_by_key(|entry| (entry.v, entry.sec));
    let count = nulls.len() as u64;
    nulls.extend(values);
    (nulls, count)
}

/// The bytes of one entry at `width`.
fn encode_entry(out: &mut Vec<u8>, entry: Entry, width: u8, base: i64, null: bool) {
    if width == NARROW {
        let offset = if null {
            0
        } else {
            u32::try_from(entry.v.saturating_sub(base)).unwrap_or(u32::MAX)
        };
        out.extend_from_slice(&offset.to_le_bytes());
    } else {
        out.extend_from_slice(&entry.v.to_le_bytes());
    }
    out.extend_from_slice(&entry.sec.to_le_bytes());
}

/// The checksum key binding a block to its run, column and place.
fn block_key(start: u64, column: Column, block: u64) -> [u8; 17] {
    let mut key = [0u8; 17];
    put(&mut key, 0, &start.to_le_bytes());
    put(&mut key, 8, &[column.code()]);
    put(&mut key, 9, &block.to_le_bytes());
    key
}

/// The checksum key binding a fence table to its run and column.
fn table_key(start: u64, column: Column) -> [u8; 9] {
    let mut key = [0u8; 9];
    put(&mut key, 0, &start.to_le_bytes());
    put(&mut key, 8, &[column.code()]);
    key
}

/// The bytes of one run over `bars`, whose first is row `start` of the month
/// `shape` names: its 512-byte header, then each column's fence table, block
/// checksums and entries.
///
/// # Cost
///
/// One pass over the bars, and one `O(n log n)` sort per column — six sorts
/// of the run's rows, paid once when the run is written. Memory: one column's
/// entries at a time, twelve bytes a bar, plus the run's bytes.
///
/// # Errors
///
/// [`Why::Empty`] for no bars, [`Why::Outside`] or [`Why::NotWholeSecond`]
/// for a stamp no entry can name.
pub fn build_run(shape: &Shape, start: u64, bars: &[Bar]) -> Result<Vec<u8>, Why> {
    build_run_as(shape, start, bars, RunKind::Appended)
}

/// [`build_run`], as a run of `kind`.
///
/// # Errors
///
/// What [`build_run`] refuses.
pub fn build_run_as(
    shape: &Shape,
    start: u64,
    bars: &[Bar],
    kind: RunKind,
) -> Result<Vec<u8>, Why> {
    let (Some(first), Some(last)) = (bars.first(), bars.last()) else {
        return Err(Why::Empty);
    };
    let rows = bars.len() as u64;
    let mut secs = Vec::with_capacity(bars.len());
    let mut extremes = None;
    for (at, bar) in bars.iter().enumerate() {
        secs.push(second_of(shape, start + at as u64, bar.ts_micros)?);
        fold(&mut extremes, bar);
    }
    let blocks = rows.div_ceil(FENCE);
    let mut sections: Vec<u8> = Vec::new();
    let mut columns = [ColumnInfo::default(); 6];
    for column in Column::ALL {
        let (entries, nulls) = ordered(column, bars, &secs);
        let values = entries.get(usize::try_from(nulls).unwrap_or(usize::MAX)..);
        let (base, max) = values
            .and_then(|values| values.first().zip(values.last()))
            .map_or((0, 0), |(lo, hi)| (lo.v, hi.v));
        let width = if max
            .checked_sub(base)
            .is_some_and(|span| u32::try_from(span).is_ok())
        {
            NARROW
        } else {
            WIDE
        };
        let mut body = Vec::with_capacity(entries.len() * usize::from(width));
        for (at, entry) in entries.iter().enumerate() {
            encode_entry(&mut body, *entry, width, base, (at as u64) < nulls);
        }
        let mut fences = Vec::with_capacity(usize::try_from(blocks).unwrap_or(0) * 16);
        let mut crcs = Vec::with_capacity(usize::try_from(blocks).unwrap_or(0) * 4);
        let stride = usize::from(width);
        for block in 0..blocks {
            let from = usize::try_from(block * FENCE).unwrap_or(usize::MAX) * stride;
            let until = usize::try_from(((block + 1) * FENCE).min(rows))
                .unwrap_or(usize::MAX)
                .saturating_mul(stride);
            let bytes = body.get(from..until).unwrap_or_default();
            fences.extend_from_slice(bytes.get(..stride).unwrap_or_default());
            crcs.extend_from_slice(
                &crc32c_split(bytes, &block_key(start, column, block)).to_le_bytes(),
            );
        }
        let table_crc = crc32c_split(
            &[fences.as_slice(), crcs.as_slice()].concat(),
            &table_key(start, column),
        );
        let info = ColumnInfo {
            width,
            nulls,
            base,
            max,
            offset: RUN_HEADER_LEN + sections.len() as u64,
            blocks,
        };
        sections.extend_from_slice(&fences);
        sections.extend_from_slice(&crcs);
        sections.extend_from_slice(&table_crc.to_le_bytes());
        sections.extend_from_slice(&body);
        if let Some(slot) = columns.get_mut(column.index()) {
            *slot = info;
        }
    }
    let len = RUN_HEADER_LEN + sections.len() as u64;
    let run = RunInfo {
        offset: 0,
        len,
        start,
        end: start + rows,
        ts_first: first.ts_micros,
        ts_last: last.ts_micros,
        columns,
        extremes: extremes.unwrap_or_default(),
        kind,
        body_crc: crc32c_split(&start.to_le_bytes(), &sections),
    };
    let mut out = Vec::with_capacity(usize::try_from(len).unwrap_or(0));
    out.extend_from_slice(&encode_run_header(&run));
    out.extend_from_slice(&sections);
    Ok(out)
}

/// A run's 512-byte header.
///
/// | Offset | Size | Field |
/// |---|---|---|
/// | 0 | 8 | magic `BRUTEXSR` |
/// | 8 | 8 | first row |
/// | 16 | 8 | one past the last row |
/// | 24 | 8 | stamp of the first row |
/// | 32 | 8 | stamp of the last row |
/// | 40 | 8 | the run's length in bytes |
/// | 48 | 6 × 48 | per column: width (1), 7 zero, nulls (8), base (8), max (8), section offset (8), blocks (8) |
/// | 336 | 6 × 16 | extremes: close max, close min, high max, low min, range max, volume max — each value (8) and stamp (8) |
/// | 432 | 4 | CRC-32C over every byte after the header, keyed by the first row |
/// | 436 | 1 | kind: `0` appended, `1` a whole image ([`RunKind`]) |
/// | 437 | 71 | reserved, zero |
/// | 508 | 4 | CRC-32C over bytes `0..508` |
fn encode_run_header(run: &RunInfo) -> [u8; 512] {
    let mut out = [0u8; 512];
    put(&mut out, 0, &RUN_MAGIC);
    put(&mut out, 8, &run.start.to_le_bytes());
    put(&mut out, 16, &run.end.to_le_bytes());
    put(&mut out, 24, &run.ts_first.to_le_bytes());
    put(&mut out, 32, &run.ts_last.to_le_bytes());
    put(&mut out, 40, &run.len.to_le_bytes());
    for (at, info) in run.columns.iter().enumerate() {
        let o = usize::try_from(COLUMNS_AT + at as u64 * COLUMN_LEN).unwrap_or(0);
        put(&mut out, o, &[info.width]);
        put(&mut out, o + 8, &info.nulls.to_le_bytes());
        put(&mut out, o + 16, &info.base.to_le_bytes());
        put(&mut out, o + 24, &info.max.to_le_bytes());
        put(&mut out, o + 32, &info.offset.to_le_bytes());
        put(&mut out, o + 40, &info.blocks.to_le_bytes());
    }
    for (at, extreme) in run.extremes.all().iter().enumerate() {
        let o = usize::try_from(EXTREMES_AT).unwrap_or(0) + at * 16;
        put(&mut out, o, &extreme.v.to_le_bytes());
        put(&mut out, o + 8, &extreme.ts.to_le_bytes());
    }
    put(&mut out, BODY_CRC_AT, &run.body_crc.to_le_bytes());
    put(&mut out, KIND_AT, &[run.kind.code()]);
    let crc = crc32c(
        out.get(..usize::try_from(RUN_CRC_AT).unwrap_or(0))
            .unwrap_or_default(),
    );
    put(
        &mut out,
        usize::try_from(RUN_CRC_AT).unwrap_or(0),
        &crc.to_le_bytes(),
    );
    out
}

/// The run a 512-byte header at `offset` describes, when it is whole, starts
/// at `expected_start`, and its columns tile its length exactly.
///
/// # Errors
///
/// [`Why::Run`] naming what is wrong with it.
pub fn decode_run_header(
    bytes: &[u8; 512],
    offset: u64,
    expected_start: u64,
) -> Result<RunInfo, Why> {
    let refuse = |why: &'static str| Why::Run { offset, why };
    let crc = crc32c(
        bytes
            .get(..usize::try_from(RUN_CRC_AT).unwrap_or(0))
            .unwrap_or_default(),
    );
    if crc.to_le_bytes() != take::<4>(bytes, usize::try_from(RUN_CRC_AT).unwrap_or(0)) {
        return Err(refuse("its header fails its checksum"));
    }
    if take::<8>(bytes, 0) != RUN_MAGIC {
        return Err(refuse("its magic is not BRUTEXSR"));
    }
    let word = |at: usize| u64::from_le_bytes(take::<8>(bytes, at));
    let signed = |at: usize| i64::from_le_bytes(take::<8>(bytes, at));
    let (start, end, len) = (word(8), word(16), word(40));
    if start != expected_start {
        return Err(refuse("it does not start where the run before it ended"));
    }
    if end <= start {
        return Err(refuse("it orders no rows"));
    }
    let rows = end - start;
    let blocks = rows.div_ceil(FENCE);
    let mut columns = [ColumnInfo::default(); 6];
    let mut next = RUN_HEADER_LEN;
    for column in Column::ALL {
        let o = usize::try_from(COLUMNS_AT + column.index() as u64 * COLUMN_LEN).unwrap_or(0);
        let info = ColumnInfo {
            width: take::<1>(bytes, o)[0],
            nulls: word(o + 8),
            base: signed(o + 16),
            max: signed(o + 24),
            offset: word(o + 32),
            blocks: word(o + 40),
        };
        if info.width != NARROW && info.width != WIDE {
            return Err(refuse("a column's entry width is neither 8 nor 12"));
        }
        if info.blocks != blocks || info.offset != next {
            return Err(refuse("a column's section is not where its run puts it"));
        }
        if info.nulls > rows || (info.nulls > 0 && !column.nullable()) {
            return Err(refuse("a column counts nulls it cannot hold"));
        }
        if info.max < info.base {
            return Err(refuse("a column's largest value is below its smallest"));
        }
        next = next
            .checked_add(info.section_len(rows))
            .ok_or_else(|| refuse("its columns overflow"))?;
        if let Some(slot) = columns.get_mut(column.index()) {
            *slot = info;
        }
    }
    if next != len {
        return Err(refuse("its columns do not fill its stated length"));
    }
    let kind = match take::<1>(bytes, KIND_AT)[0] {
        0 => RunKind::Appended,
        1 => RunKind::Whole,
        _ => return Err(refuse("its kind is neither appended nor whole")),
    };
    if bytes
        .get(KIND_AT + 1..usize::try_from(RUN_CRC_AT).unwrap_or(0))
        .is_none_or(|reserved| reserved.iter().any(|byte| *byte != 0))
    {
        return Err(refuse("its reserved bytes are not zero"));
    }
    let extreme = |at: usize| {
        let o = usize::try_from(EXTREMES_AT).unwrap_or(0) + at * 16;
        Extreme {
            v: signed(o),
            ts: signed(o + 8),
        }
    };
    Ok(RunInfo {
        offset,
        len,
        start,
        end,
        ts_first: signed(24),
        ts_last: signed(32),
        columns,
        extremes: RunExtremes {
            close_max: extreme(0),
            close_min: extreme(1),
            high_max: extreme(2),
            low_min: extreme(3),
            range_max: extreme(4),
            volume_max: extreme(5),
        },
        kind,
        body_crc: u32::from_le_bytes(take::<4>(bytes, BODY_CRC_AT)),
    })
}

/// Whether the body of `run` — every byte after its header — is whole: its
/// checksum, read in pieces of at most [`BODY_READ`] bytes.
///
/// # Errors
///
/// A read the host refused.
fn body_whole(
    read: &mut impl FnMut(u64, &mut [u8]) -> io::Result<()>,
    run: &RunInfo,
) -> io::Result<bool> {
    let mut crc = crc32c(&run.start.to_le_bytes());
    let mut at = run.offset + RUN_HEADER_LEN;
    let end = run.offset + run.len;
    let mut buf = vec![0u8; BODY_READ.min(usize::try_from(end - at).unwrap_or(BODY_READ))];
    while at < end {
        let size = usize::try_from(end - at).map_or(buf.len(), |left| left.min(buf.len()));
        let piece = buf.get_mut(..size).unwrap_or_default();
        read(at, piece)?;
        crc = crc32c_extend(crc, piece);
        at += size as u64;
    }
    Ok(crc == run.body_crc)
}

/// Checks the body of every run in `runs` against the checksum its header
/// holds: the whole file read once, in pieces of at most `BODY_READ` bytes.
///
/// [`walk`] reads headers only (and the body of a last appended run, the one
/// a crash can tear), so a flipped byte in a fence table or a block is found
/// by the reader that reaches it and by nothing sooner. This is the check a
/// REBUILD makes before it keeps a month as it is: O(file), about 20 MB for a
/// one-second month, and never run by a reader serving a page.
///
/// # Errors
///
/// [`Why::Body`] naming the first run whose body fails; [`Why::Unreadable`]
/// for a read the host refused.
pub fn verify_bodies(
    mut read: impl FnMut(u64, &mut [u8]) -> io::Result<()>,
    runs: &[RunInfo],
) -> Result<(), Why> {
    for run in runs {
        if !body_whole(&mut read, run).map_err(|why| Why::Unreadable(why.to_string()))? {
            return Err(Why::Body {
                start: run.start,
                end: run.end,
            });
        }
    }
    Ok(())
}

/// What a walk of a sort index file found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Walked {
    /// The valid runs, in row order from row 0.
    pub runs: Vec<RunInfo>,
    /// The end of the last valid run (the header's end when there is none):
    /// where the next run is appended.
    pub end: u64,
    /// Bytes past `end` that are not a valid run — a run being written, or
    /// one a process died writing — with why. Never read as a run.
    pub torn: Option<(u64, String)>,
}

impl Walked {
    /// The rows the runs cover: `[0, coverage)`.
    #[must_use]
    pub fn coverage(&self) -> u64 {
        self.runs.last().map_or(0, |run| run.end)
    }
}

/// Walks a sort index file of `file_len` bytes through `read`: the header,
/// then each run header until the bytes end or one does not decode.
///
/// `MAX_RUNS + 1` header reads at most, whatever the month holds.
///
/// # Errors
///
/// [`Why::Header`] for a header that is short or not this month's,
/// [`Why::TooManyRuns`] past [`MAX_RUNS`], [`Why::Unreadable`] for a read the
/// host refused.
pub fn walk(
    mut read: impl FnMut(u64, &mut [u8]) -> io::Result<()>,
    file_len: u64,
    shape: &Shape,
) -> Result<Walked, Why> {
    if file_len < HEADER_LEN {
        return Err(Why::Header("it is shorter than its header"));
    }
    let mut head = [0u8; 64];
    read(0, &mut head).map_err(|why| Why::Unreadable(why.to_string()))?;
    shape.check(&head)?;
    let mut runs: Vec<RunInfo> = Vec::new();
    let mut end = HEADER_LEN;
    let mut torn = None;
    while end < file_len {
        if runs.len() >= MAX_RUNS {
            return Err(Why::TooManyRuns(runs.len() + 1));
        }
        let expected = runs.last().map_or(0, |run| run.end);
        if file_len - end < RUN_HEADER_LEN {
            torn = Some((end, "shorter than a run header".to_owned()));
            break;
        }
        let mut bytes = [0u8; 512];
        read(end, &mut bytes).map_err(|why| Why::Unreadable(why.to_string()))?;
        match decode_run_header(&bytes, end, expected) {
            Ok(run) if run.len <= file_len - end => {
                end += run.len;
                runs.push(run);
            }
            Ok(run) => {
                torn = Some((
                    end,
                    format!("a run of {} bytes with {} on disk", run.len, file_len - end),
                ));
                break;
            }
            Err(why) => {
                torn = Some((end, why.to_string()));
                break;
            }
        }
    }
    // THE LAST APPENDED RUN IS THE ONLY ONE A CRASH CAN TEAR (see
    // [`RunKind`]): its body is checked whole, and a torn one is never read.
    if let Some(last) = runs.last().copied()
        && last.kind == RunKind::Appended
        && !body_whole(&mut read, &last).map_err(|why| Why::Unreadable(why.to_string()))?
    {
        runs.pop();
        end = last.offset;
        torn = Some((
            last.offset,
            "the last appended run's body fails its checksum: a writer stopped before it was whole"
                .to_owned(),
        ));
    }
    Ok(Walked { runs, end, torn })
}

/// Checks the runs against the bar file they claim to order: each run ends
/// at or before the committed count, and each run's first and last stamps
/// are the stamps of those rows. The header's own first and last stamps
/// answer the two ends without a read; every other boundary is one
/// `ts_at` call.
///
/// # Errors
///
/// [`Why::Stale`] naming the first disagreement, or the reason `ts_at`
/// would not read a row.
pub fn bind(
    runs: &[RunInfo],
    n_valid: u64,
    first_ts: i64,
    last_ts: i64,
    mut ts_at: impl FnMut(u64) -> Result<i64, String>,
) -> Result<u64, Why> {
    let mut coverage = 0;
    for run in runs {
        if run.end > n_valid {
            return Err(Why::Stale(format!(
                "a run orders rows {}..{} and the month commits {n_valid}",
                run.start, run.end
            )));
        }
        let first = if run.start == 0 {
            first_ts
        } else {
            ts_at(run.start).map_err(Why::Stale)?
        };
        let last = if run.end == n_valid {
            last_ts
        } else {
            ts_at(run.end - 1).map_err(Why::Stale)?
        };
        if first != run.ts_first || last != run.ts_last {
            return Err(Why::Stale(format!(
                "the run over rows {}..{} was built from bars stamped {}..{}, and the month's \
                 rows there are stamped {first}..{last}",
                run.start, run.end, run.ts_first, run.ts_last
            )));
        }
        coverage = run.end;
    }
    Ok(coverage)
}

/// A column's fence table: the first entry of every block, and every block's
/// checksum.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fences {
    /// The first entry of each block.
    pub entries: Vec<Entry>,
    /// Each block's CRC-32C.
    crcs: Vec<u32>,
}

impl Fences {
    /// Bytes this table holds in memory, for a caller's budget.
    #[must_use]
    pub fn weight(&self) -> usize {
        self.entries.len() * std::mem::size_of::<Entry>() + self.crcs.len() * 4
    }
}

/// The entry `bytes` hold at position `at` of `info`'s column.
fn decode_entry(bytes: &[u8], info: &ColumnInfo, at: u64) -> Entry {
    let sec = u32::from_le_bytes(take::<4>(bytes, usize::from(info.width) - 4));
    if at < info.nulls {
        return Entry { v: OI_NULL, sec };
    }
    let v = if info.width == NARROW {
        info.base
            .saturating_add(i64::from(u32::from_le_bytes(take::<4>(bytes, 0))))
    } else {
        i64::from_le_bytes(take::<8>(bytes, 0))
    };
    Entry { v, sec }
}

/// One month's sort index, opened for reading. Never writes, never creates.
#[derive(Debug)]
pub struct SortIndex {
    file: File,
    path: PathBuf,
    shape: Shape,
    walked: Walked,
}

impl SortIndex {
    /// Opens `path` read-only and walks its runs.
    ///
    /// # Errors
    ///
    /// [`Why::Absent`] when there is no file; whatever [`walk`] refuses.
    pub fn open(path: &Path, shape: &Shape) -> Result<Self, Why> {
        let file = match File::open(path) {
            Ok(file) => file,
            Err(why) if why.kind() == ErrorKind::NotFound => return Err(Why::Absent),
            Err(why) => return Err(Why::Unreadable(format!("{}: {why}", path.display()))),
        };
        let meta = file
            .metadata()
            .map_err(|why| Why::Unreadable(format!("{}: {why}", path.display())))?;
        if !meta.is_file() {
            return Err(Why::Unreadable(format!(
                "{} is not a regular file",
                path.display()
            )));
        }
        let walked = walk(|at, buf| file.read_exact_at(buf, at), meta.len(), shape)?;
        Ok(Self {
            file,
            path: path.to_path_buf(),
            shape: *shape,
            walked,
        })
    }

    /// Opens `path` read-only for block reads, trusting a walk a caller
    /// already made of the same file — a server that keeps the walk between
    /// requests, keyed by the file's own stamp, and re-walks when the stamp
    /// moves. The file must still hold every byte the walk read.
    ///
    /// # Errors
    ///
    /// [`Why::Absent`] when there is no file; [`Why::Unreadable`] when it is
    /// not a regular file or is shorter than the walk.
    pub fn open_walked(path: &Path, shape: &Shape, walked: Walked) -> Result<Self, Why> {
        let file = match File::open(path) {
            Ok(file) => file,
            Err(why) if why.kind() == ErrorKind::NotFound => return Err(Why::Absent),
            Err(why) => return Err(Why::Unreadable(format!("{}: {why}", path.display()))),
        };
        let meta = file
            .metadata()
            .map_err(|why| Why::Unreadable(format!("{}: {why}", path.display())))?;
        if !meta.is_file() || meta.len() < walked.end {
            return Err(Why::Unreadable(format!(
                "{} is not the file that was walked",
                path.display()
            )));
        }
        Ok(Self {
            file,
            path: path.to_path_buf(),
            shape: *shape,
            walked,
        })
    }

    /// What the walk found.
    #[must_use]
    pub const fn walked(&self) -> &Walked {
        &self.walked
    }

    /// Checks every run's body against its header's checksum: see
    /// [`verify_bodies`]. O(file) — a rebuild's check, not a page's.
    ///
    /// # Errors
    ///
    /// [`Why::Body`], or [`Why::Unreadable`].
    pub fn verify(&self) -> Result<(), Why> {
        verify_bodies(
            |at, buf| self.file.read_exact_at(buf, at),
            &self.walked.runs,
        )
    }

    /// The valid runs, in row order.
    #[must_use]
    pub fn runs(&self) -> &[RunInfo] {
        &self.walked.runs
    }

    /// The rows the runs cover, `[0, coverage)`.
    #[must_use]
    pub fn coverage(&self) -> u64 {
        self.walked.coverage()
    }

    /// Bytes past the last valid run that are not a run, with why.
    #[must_use]
    pub fn torn(&self) -> Option<&(u64, String)> {
        self.walked.torn.as_ref()
    }

    /// The file.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Its shape.
    #[must_use]
    pub const fn shape(&self) -> &Shape {
        &self.shape
    }

    /// Run `run` and its column `column`, or the refusal.
    fn info(&self, run: usize, column: Column) -> Result<(&RunInfo, &ColumnInfo), Why> {
        let at = self.walked.runs.get(run).ok_or(Why::Run {
            offset: 0,
            why: "no such run",
        })?;
        Ok((at, at.column(column)))
    }

    /// `run`'s fence table for `column`: one read, checked whole.
    ///
    /// # Errors
    ///
    /// [`Why::Fences`] when it fails its checksum or would not read.
    pub fn fences(&self, run: usize, column: Column) -> Result<Fences, Why> {
        let (at, info) = self.info(run, column)?;
        let refuse = || Why::Fences {
            start: at.start,
            column,
        };
        let width = u64::from(info.width);
        let table = usize::try_from(info.blocks * (width + 4) + 4).map_err(|_| refuse())?;
        let mut bytes = vec![0u8; table];
        self.file
            .read_exact_at(&mut bytes, at.offset + info.offset)
            .map_err(|_| refuse())?;
        let body = table - 4;
        let crc = crc32c_split(
            bytes.get(..body).unwrap_or_default(),
            &table_key(at.start, column),
        );
        if crc.to_le_bytes() != take::<4>(&bytes, body) {
            return Err(refuse());
        }
        let stride = usize::from(info.width);
        let fences_len = usize::try_from(info.blocks).map_err(|_| refuse())? * stride;
        let entries = (0..info.blocks)
            .map(|block| {
                let from = usize::try_from(block).unwrap_or(0) * stride;
                decode_entry(
                    bytes.get(from..from + stride).unwrap_or_default(),
                    info,
                    block * FENCE,
                )
            })
            .collect();
        let crcs = bytes
            .get(fences_len..body)
            .unwrap_or_default()
            .chunks_exact(4)
            .map(|chunk| u32::from_le_bytes(take::<4>(chunk, 0)))
            .collect();
        Ok(Fences { entries, crcs })
    }

    /// Block `block` of `run`'s `column`: at most [`FENCE`] entries, one read,
    /// checked against its CRC in `fences`.
    ///
    /// # Errors
    ///
    /// [`Why::Block`] when it fails its checksum or would not read.
    pub fn block(
        &self,
        run: usize,
        column: Column,
        block: u64,
        fences: &Fences,
    ) -> Result<Vec<Entry>, Why> {
        let (at, info) = self.info(run, column)?;
        let refuse = || Why::Block {
            start: at.start,
            column,
            block,
        };
        let rows = at.rows();
        if block >= info.blocks {
            return Err(refuse());
        }
        let first = block * FENCE;
        let count = FENCE.min(rows - first);
        let stride = u64::from(info.width);
        let mut bytes = vec![0u8; usize::try_from(count * stride).map_err(|_| refuse())?];
        self.file
            .read_exact_at(&mut bytes, at.offset + info.entries_at() + first * stride)
            .map_err(|_| refuse())?;
        let held = fences
            .crcs
            .get(usize::try_from(block).map_err(|_| refuse())?)
            .copied()
            .ok_or_else(refuse)?;
        if crc32c_split(&bytes, &block_key(at.start, column, block)) != held {
            return Err(refuse());
        }
        Ok(bytes
            .chunks_exact(usize::from(info.width))
            .enumerate()
            .map(|(i, chunk)| decode_entry(chunk, info, first + i as u64))
            .collect())
    }

    /// Every entry of `run`'s `column` that `keep` keeps, in the column's
    /// order (nulls first in time order, then by value and time), and how
    /// many of those are nulls. Each block is checked against its CRC in
    /// `fences` before any entry of it is decoded. D-3349.
    ///
    /// For a run a day bound cuts: the entries of the kept stretch are the
    /// run's entries whose instant lies inside the bound, already in order,
    /// so a page over them needs no bar read and no sort.
    ///
    /// # Cost
    ///
    /// The column's entries read once, in reads of at most
    /// [`Self::SECTION_READ_BLOCKS`] blocks: `rows × width` bytes, eight or
    /// twelve a row. Memory: the kept entries, sixteen bytes each, plus one
    /// read's buffer.
    ///
    /// # Errors
    ///
    /// [`Why::Block`] naming the first block that fails its checksum or would
    /// not read.
    pub fn entries_where(
        &self,
        run: usize,
        column: Column,
        fences: &Fences,
        keep: impl Fn(Entry) -> bool,
    ) -> Result<(Vec<Entry>, u64), Why> {
        let (at, info) = self.info(run, column)?;
        let rows = at.rows();
        let stride = u64::from(info.width);
        let mut out = Vec::new();
        let mut nulls = 0_u64;
        let mut block = 0_u64;
        while block < info.blocks {
            let blocks = Self::SECTION_READ_BLOCKS.min(info.blocks - block);
            let first = block * FENCE;
            let count = (blocks * FENCE).min(rows - first);
            let refuse = |block: u64| Why::Block {
                start: at.start,
                column,
                block,
            };
            let mut bytes = vec![0u8; usize::try_from(count * stride).map_err(|_| refuse(block))?];
            self.file
                .read_exact_at(&mut bytes, at.offset + info.entries_at() + first * stride)
                .map_err(|_| refuse(block))?;
            let per_block = usize::try_from(FENCE * stride).map_err(|_| refuse(block))?;
            for (i, chunk) in bytes.chunks(per_block).enumerate() {
                let this = block + i as u64;
                let held = fences
                    .crcs
                    .get(usize::try_from(this).map_err(|_| refuse(this))?)
                    .copied()
                    .ok_or_else(|| refuse(this))?;
                if crc32c_split(chunk, &block_key(at.start, column, this)) != held {
                    return Err(refuse(this));
                }
                for (j, raw) in chunk.chunks_exact(usize::from(info.width)).enumerate() {
                    let position = this * FENCE + j as u64;
                    let entry = decode_entry(raw, info, position);
                    if keep(entry) {
                        if position < info.nulls {
                            nulls += 1;
                        }
                        out.push(entry);
                    }
                }
            }
            block += blocks;
        }
        Ok((out, nulls))
    }

    /// Blocks one read of [`Self::entries_where`] takes: 1,024 blocks, at
    /// most 3 MiB.
    pub const SECTION_READ_BLOCKS: u64 = 1_024;
}

/// `N` bytes of `bytes` from `at`, zero-filled past the end. Every caller
/// asks inside a buffer whose length it fixed.
fn take<const N: usize>(bytes: &[u8], at: usize) -> [u8; N] {
    let mut out = [0u8; N];
    if let Some(src) = bytes.get(at..at + N) {
        out.copy_from_slice(src);
    }
    out
}

/// Copies `src` into `out` at `at`; a write past the end is dropped, and no
/// caller makes one.
fn put(out: &mut [u8], at: usize, src: &[u8]) {
    if let Some(dst) = out.get_mut(at..at + src.len()) {
        dst.copy_from_slice(src);
    }
}

#[cfg(test)]
#[path = "sort_index_tests.rs"]
mod tests;

//! The time index — `<yyyy-mm>.tix` beside a month's `.bin` — and the lookup
//! it makes constant. D-2329, D-2330.
//!
//! # What it is for
//!
//! `BarFile::first_at_or_after` answered "which committed bar is the first at
//! or after this instant" with a bisection over the records: up to
//! `ceil(log2(n_valid + 1))` record reads, each able to pay a cold block verify
//! (D-1434). `CLAUDE.md` §3 rule 4 names bar lookup as an operation that must be
//! constant, so this module turns time into a row in a FIXED number of reads,
//! whatever the month holds.
//!
//! # Why a sidecar and not arithmetic on the header
//!
//! A bar's slot on its rung's grid is pure arithmetic — the path names the IST
//! month and the header names the timeframe — but the slot is not the row: a
//! month has holidays, nights and missing minutes, so `slot == row` is false
//! after the first gap. The header carries no field that recovers the row
//! from the slot, and §3 rule 8 forbids giving it one in place. So the
//! slot-to-row table is a NEW FILE at its own version and stride
//! (`docs/02-store-format.md` §8.1, `CLAUDE.md` §4's "a new field is a new file
//! version at its own stride").
//!
//! # The shape
//!
//! The month is cut into slots of exactly one timeframe on the grid the
//! writer admits (D-0915): intraday slots start on the 09:15 IST-anchored grid,
//! daily slots at IST midnight. Every 64 slots share one 16-byte [`Entry`]:
//! a 64-bit occupancy word (bit `j` set when a bar lies in slot `64b + j`) and
//! the count of bars in every earlier slot. The row of the first bar at or
//! after slot `q` is then
//!
//! ```text
//! before[q / 64] + popcount(occupancy[q / 64] & ((1 << (q % 64)) - 1))
//! ```
//!
//! — one entry read and two instructions. Holding it requires AT MOST ONE BAR
//! PER SLOT, which the intraday grid gives for free (two strictly increasing
//! on-grid stamps are two slots) and a daily month usually has: a month with a
//! second bar on one IST day, which the daily rung admits, keeps no index and
//! bisects, said out loud (D-2330).
//!
//! # What this module is, and is not
//!
//! Pure: geometry, the byte codec and the lookup, each driven by closures so
//! the READ COUNT is asserted rather than described. The file I/O — opening,
//! building, extending and the loud fallback — is `crate::file`'s, beside the
//! bar I/O it must stay ordered with.

use std::fmt;

use crate::crc::{crc32c, crc32c_split};
use crate::file::{OPEN_ANCHOR_MICROS, StoreError};
use crate::header::Header;
use crate::path::{IST_OFFSET_SECS, YearMonth};

/// `b"BRUTEXT1"`: the bar family's `BRUTEX` prefix, `T` for time, version 1.
pub const MAGIC: [u8; 8] = *b"BRUTEXT1";

/// The one version this build reads and writes.
pub const VERSION: u16 = 1;

/// The header: 64 bytes at offset 0, written whole and checksummed whole.
pub const HEADER_LEN: u64 = 64;

/// One [`Entry`]: 8 occupancy bytes, 4 count bytes, 4 checksum bytes.
pub const ENTRY_LEN: u64 = 16;

/// Slots one [`Entry`] describes: the width of its occupancy word.
pub const SLOTS_PER_ENTRY: u64 = 64;

/// Microseconds in a second.
const MICROS: i64 = 1_000_000;

/// Where the slots of one month at one rung lie.
///
/// Derived from the path's month, the header's timeframe and the header's
/// symbol, and WRITTEN into the `.tix` header so a reader compares rather than
/// trusts: an index whose geometry is not the one this file's path implies is
/// refused as [`Why::Header`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Geometry {
    /// The start of slot 0, epoch microseconds: the last grid point at or
    /// before 00:00 IST on the first of the month.
    origin_micros: i64,
    /// One slot, in microseconds: the timeframe.
    slot_micros: i64,
    /// Slots from `origin_micros` to the end of the month.
    slot_count: u64,
    /// The file's timeframe, from its header.
    timeframe_secs: u32,
    /// The file's symbol, from its header.
    symbol_id: u32,
    /// Whether every bar the index holds is EXACTLY its slot's first instant:
    /// true for every intraday rung, whose admitted stamps are the grid
    /// itself (D-0915), false for the daily rung, whose bar may sit anywhere
    /// in its IST day. When true, [`extend`] refuses any other stamp and
    /// [`locate`] never needs a bar read.
    exact: bool,
}

impl Geometry {
    /// The slots of `month` at `timeframe_secs`.
    ///
    /// Intraday slots start on the grid `crate::file::Admission` admits —
    /// `(ts + anchor) mod width == 0` with the 09:15 IST anchor — so every
    /// admitted intraday stamp is EXACTLY a slot start. Daily slots are IST
    /// days. Every intraday width divides 86,400 s, so the grid is the same on
    /// every day of the month.
    ///
    /// The anchor is `crate::file`'s own `OPEN_ANCHOR_MICROS`, the constant
    /// `Admission` admits intraday stamps against, so a slot start and an
    /// admitted stamp are the same instants by construction.
    #[must_use]
    pub fn new(month: YearMonth, timeframe_secs: u32, symbol_id: u32) -> Self {
        let (from, until) = month.ist_bounds_micros();
        let width = i64::from(timeframe_secs) * MICROS;
        // A zero width cannot reach here from a real file — `validated`
        // refuses a header whose timeframe disagrees with the path and every
        // `Timeframe` is at least a second — but `max(1)` keeps the
        // arithmetic total rather than leaving a division a caller can trip.
        let width = width.max(1);
        let anchor = if timeframe_secs >= 86_400 {
            IST_OFFSET_SECS * MICROS
        } else {
            OPEN_ANCHOR_MICROS
        };
        let origin = from - (from + anchor).rem_euclid(width);
        let span = (until - 1 - origin) / width + 1;
        Self {
            origin_micros: origin,
            slot_micros: width,
            slot_count: span.unsigned_abs(),
            timeframe_secs,
            symbol_id,
            exact: timeframe_secs < 86_400,
        }
    }

    /// The start of slot 0.
    #[must_use]
    pub const fn origin_micros(&self) -> i64 {
        self.origin_micros
    }

    /// The width of one slot.
    #[must_use]
    pub const fn slot_micros(&self) -> i64 {
        self.slot_micros
    }

    /// Slots in the month.
    #[must_use]
    pub const fn slot_count(&self) -> u64 {
        self.slot_count
    }

    /// Whether every indexed bar is its slot's first instant: intraday rungs.
    #[must_use]
    pub const fn exact(&self) -> bool {
        self.exact
    }

    /// Entries a complete index of this month holds: one per 64 slots.
    #[must_use]
    pub const fn entry_count(&self) -> u64 {
        self.slot_count.div_ceil(SLOTS_PER_ENTRY)
    }

    /// The slot `ts_micros` lies in, or `None` outside the month's slots.
    #[must_use]
    pub fn slot_of(&self, ts_micros: i64) -> Option<u64> {
        let offset = ts_micros.checked_sub(self.origin_micros)?;
        let slot = u64::try_from(offset.div_euclid(self.slot_micros)).ok()?;
        (slot < self.slot_count).then_some(slot)
    }

    /// The first instant of `slot`.
    #[must_use]
    pub fn slot_start(&self, slot: u64) -> i64 {
        let slot = i64::try_from(slot).unwrap_or(i64::MAX);
        self.origin_micros
            .saturating_add(slot.saturating_mul(self.slot_micros))
    }

    /// The 64-byte header this geometry is written as.
    ///
    /// | Offset | Size | Field |
    /// |---|---|---|
    /// | 0 | 8 | magic `BRUTEXT1` |
    /// | 8 | 2 | version `1` |
    /// | 10 | 2 | entry stride `16` |
    /// | 12 | 4 | timeframe seconds |
    /// | 16 | 8 | origin, epoch microseconds |
    /// | 24 | 8 | slot width, microseconds |
    /// | 32 | 8 | slot count |
    /// | 40 | 4 | symbol id |
    /// | 44 | 16 | reserved, zero |
    /// | 60 | 4 | CRC-32C over bytes `0..60` |
    #[must_use]
    pub fn encode(&self) -> [u8; 64] {
        let mut out = [0u8; 64];
        put(&mut out, 0, &MAGIC);
        put(&mut out, 8, &VERSION.to_le_bytes());
        put(&mut out, 10, &16u16.to_le_bytes());
        put(&mut out, 12, &self.timeframe_secs.to_le_bytes());
        put(&mut out, 16, &self.origin_micros.to_le_bytes());
        put(&mut out, 24, &self.slot_micros.to_le_bytes());
        put(&mut out, 32, &self.slot_count.to_le_bytes());
        put(&mut out, 40, &self.symbol_id.to_le_bytes());
        let crc = crc32c(out.get(..60).unwrap_or_default());
        put(&mut out, 60, &crc.to_le_bytes());
        out
    }

    /// Whether `bytes` is the header this geometry writes, and if not, why.
    ///
    /// Checked in the order a reader can trust each field: the checksum
    /// first, so a torn or rotted header is called that and not a foreign
    /// one; then the magic and version, so a future version is refused by
    /// number; then everything else, byte for byte.
    ///
    /// # Errors
    ///
    /// [`Why::Header`] naming the first field that disagrees.
    pub fn check(&self, bytes: &[u8; 64]) -> Result<(), Why> {
        let crc = crc32c(bytes.get(..60).unwrap_or_default());
        if crc.to_le_bytes() != take::<4>(bytes, 60) {
            return Err(Why::Header("its header fails its checksum"));
        }
        if take::<8>(bytes, 0) != MAGIC {
            return Err(Why::Header("its magic is not BRUTEXT1"));
        }
        if u16::from_le_bytes(take::<2>(bytes, 8)) != VERSION {
            return Err(Why::Header("its version is not 1"));
        }
        if self.encode() != *bytes {
            return Err(Why::Header(
                "its geometry is not the one this file's path and header imply",
            ));
        }
        Ok(())
    }
}

/// One 16-byte entry: 64 slots' occupancy and the bars before them.
///
/// | Offset | Size | Field |
/// |---|---|---|
/// | 0 | 8 | occupancy, bit `j` = slot `64b + j` holds a bar |
/// | 8 | 4 | bars in every slot before `64b` |
/// | 12 | 4 | CRC-32C over bytes `0..12` followed by `b` as 8 LE bytes |
///
/// The checksum binds the entry to its POSITION: an entry copied to another
/// bucket, or a run shifted by a lost write, fails it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entry {
    /// Bit `j` set: a bar lies in slot `64b + j`.
    pub occupancy: u64,
    /// Bars in every slot before this entry's first.
    pub before: u32,
}

impl Entry {
    /// The entry of a bucket nothing precedes and nothing occupies.
    pub const EMPTY: Self = Self {
        occupancy: 0,
        before: 0,
    };

    /// Bars in every slot of this entry below `bit`, plus every earlier one:
    /// the row of the first bar at or after slot `64b + bit`.
    #[must_use]
    pub fn rank(self, bit: u32) -> u64 {
        let below = self.occupancy & (1u64 << (bit % 64)).wrapping_sub(1);
        u64::from(self.before) + u64::from(below.count_ones())
    }

    /// Whether slot `64b + bit` holds a bar.
    #[must_use]
    pub const fn occupied(self, bit: u32) -> bool {
        (self.occupancy >> (bit % 64)) & 1 == 1
    }

    /// Bars in this entry's slots and every earlier one.
    #[must_use]
    pub fn total(self) -> u64 {
        u64::from(self.before) + u64::from(self.occupancy.count_ones())
    }

    /// This entry with every slot above `bit` cleared.
    #[must_use]
    pub const fn through(self, bit: u32) -> Self {
        let keep = if bit % 64 == 63 {
            u64::MAX
        } else {
            (1u64 << (bit % 64 + 1)) - 1
        };
        Self {
            occupancy: self.occupancy & keep,
            before: self.before,
        }
    }

    /// The 16 bytes this entry is written as at `bucket`.
    #[must_use]
    pub fn encode(self, bucket: u64) -> [u8; 16] {
        let mut out = [0u8; 16];
        put(&mut out, 0, &self.occupancy.to_le_bytes());
        put(&mut out, 8, &self.before.to_le_bytes());
        let crc = crc32c_split(out.get(..12).unwrap_or_default(), &bucket.to_le_bytes());
        put(&mut out, 12, &crc.to_le_bytes());
        out
    }

    /// The entry `bytes` holds at `bucket`, or `None` when its checksum —
    /// which covers its position — does not match.
    #[must_use]
    pub fn decode(bytes: &[u8; 16], bucket: u64) -> Option<Self> {
        let crc = crc32c_split(bytes.get(..12).unwrap_or_default(), &bucket.to_le_bytes());
        (crc.to_le_bytes() == take::<4>(bytes, 12)).then(|| Self {
            occupancy: u64::from_le_bytes(take::<8>(bytes, 0)),
            before: u32::from_le_bytes(take::<4>(bytes, 8)),
        })
    }
}

/// Why a month's time lookup is NOT served by its index — said, never hidden.
///
/// Every value is a reason a caller can print. The lookup still answers, by
/// the D-1434 bisection, and the handle writes one `store.tix` warning naming
/// this reason (`CLAUDE.md` §4: degrade loudly and name the reason).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Why {
    /// No `.tix` beside the month: written before D-2329, or by a writer that
    /// does not go through `BarFile::append`.
    Absent,
    /// The `.tix` header is not the one this month's path and header imply.
    Header(&'static str),
    /// The entry at this bucket is missing or fails its checksum.
    Entry {
        /// The bucket whose entry could not be used.
        bucket: u64,
    },
    /// The index does not describe the committed bars: the bar named by the
    /// header's first or last stamp is not where the index puts it. An append
    /// that bypassed the store writer leaves exactly this.
    Stale,
    /// A committed bar lies outside the month's slots, so no index can hold it.
    Outside {
        /// The bar's row.
        index: u64,
        /// Its stamp.
        ts_micros: i64,
    },
    /// A committed intraday bar is not its slot's first instant: an off-grid
    /// stamp, which only a month written before D-0915 can hold.
    OffGrid {
        /// The bar's row.
        index: u64,
        /// Its stamp.
        ts_micros: i64,
    },
    /// A committed bar shares a slot with the bar before it — two daily bars on
    /// one IST day.
    SharedSlot {
        /// The bar's row.
        index: u64,
        /// Its stamp.
        ts_micros: i64,
    },
    /// The month's `.bin` path no longer names the file this handle holds:
    /// the month was replaced after the handle opened it, so the `.tix` beside
    /// it may describe other bars. The handle's own bars still answer, by the
    /// bisection. satk-2, D-4416.
    Replaced,
    /// Reading or writing the index, or a bar it is built from, was refused.
    Unreadable(StoreError),
    /// A file of another record kind; only bars are indexed.
    NotBars,
}

impl fmt::Display for Why {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Absent => f.write_str("the month has no .tix time index"),
            Self::Header(why) => write!(f, "the .tix time index is refused: {why}"),
            Self::Entry { bucket } => write!(
                f,
                "the .tix entry for bucket {bucket} is missing or fails its checksum"
            ),
            Self::Stale => f.write_str("the .tix time index does not describe the committed bars"),
            Self::Outside { index, ts_micros } => write!(
                f,
                "bar {index} is stamped {ts_micros}, outside the month's slots"
            ),
            Self::OffGrid { index, ts_micros } => {
                write!(f, "bar {index} is stamped {ts_micros}, off its rung's grid")
            }
            Self::SharedSlot { index, ts_micros } => write!(
                f,
                "bar {index} is stamped {ts_micros}, in the slot of the bar before it"
            ),
            Self::Replaced => f.write_str(
                "the month's .bin path no longer names the file this handle holds, so its \
                 .tix may describe other bars",
            ),
            Self::Unreadable(why) => write!(f, "the .tix time index could not be used: {why}"),
            Self::NotBars => f.write_str("only bar files carry a time index"),
        }
    }
}

/// What the bar header says a month holds, as the index needs it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Held {
    /// The commit counter.
    pub n_valid: u64,
    /// The stamp of record 0, meaningful when `n_valid > 0`.
    pub first_ts: i64,
    /// The stamp of record `n_valid - 1`, meaningful when `n_valid > 0`.
    pub last_ts: i64,
}

impl Held {
    /// The three facts, from the committed header.
    pub(crate) const fn of(header: &Header) -> Self {
        Self {
            n_valid: header.n_valid,
            first_ts: header.first_ts_micros,
            last_ts: header.last_ts_micros,
        }
    }
}

/// The bucket and bit of `slot`.
#[must_use]
pub fn split(slot: u64) -> (u64, u32) {
    (
        slot / SLOTS_PER_ENTRY,
        u32::try_from(slot % SLOTS_PER_ENTRY).unwrap_or(0),
    )
}

/// Byte offset of the entry for `bucket`.
#[must_use]
pub const fn entry_offset(bucket: u64) -> u64 {
    HEADER_LEN.saturating_add(bucket.saturating_mul(ENTRY_LEN))
}

/// Why a lookup through the index could not finish.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Fault {
    /// The index itself: the caller falls back to the bisection, loudly.
    Index(Why),
    /// A bar read, which the bisection would have refused as well.
    Bar(StoreError),
}

/// The first committed row stamped at or after `ts` — in at most ONE entry
/// read and ONE bar read, whatever `n_valid` is.
///
/// # The cases, in order
///
/// 1. No bars, or `ts` at or before the first: row 0. No read.
/// 2. `ts` after the last: `n_valid`. No read.
/// 3. Otherwise `ts` lies in slot `q` with `first < ts <= last`, both inside
///    the month, so `q` is a real slot no later than the last bar's. One entry
///    read gives `r`, the row of the first bar in slot `q` or later.
/// 4. When slot `q` is empty, or `ts` is its first instant, `r` is the answer:
///    every bar in slot `q` or later is at or after `ts`.
/// 5. Otherwise `ts` is strictly inside an occupied slot. An intraday bar IS
///    its slot's first instant (the index holds no other, see [`extend`]), so
///    it is before `ts` and the answer is `r + 1`, with no read.
/// 6. A daily bar may sit anywhere in its IST day, so for the daily rung one
///    bar read decides between `r` and `r + 1`.
///
/// `entry` is asked for ONE bucket and `stamp` for ONE row, which
/// `store::time_index::every_lookup_reads_at_most_one_entry_and_one_bar`
/// counts at every month size up to the one-second ceiling.
///
/// # Errors
///
/// [`Fault::Index`] when the entry cannot be read or is damaged — the caller
/// bisects instead and says so — and [`Fault::Bar`] when the bar read refuses.
pub(crate) fn locate(
    geometry: &Geometry,
    held: Held,
    ts: i64,
    entry: impl FnOnce(u64) -> Result<Entry, Why>,
    stamp: impl FnOnce(u64) -> Result<i64, StoreError>,
) -> Result<u64, Fault> {
    if held.n_valid == 0 || ts <= held.first_ts {
        return Ok(0);
    }
    if ts > held.last_ts {
        return Ok(held.n_valid);
    }
    let slot = geometry.slot_of(ts).ok_or(Fault::Index(Why::Stale))?;
    let (bucket, bit) = split(slot);
    let found = entry(bucket).map_err(Fault::Index)?;
    let row = found.rank(bit);
    if row >= held.n_valid {
        // `ts <= last`, so a correct index has the last bar at or after `ts`
        // and `row < n_valid`. Anything else is an index that disagrees with
        // the header it was confirmed against.
        return Err(Fault::Index(Why::Stale));
    }
    if !found.occupied(bit) || ts == geometry.slot_start(slot) {
        return Ok(row);
    }
    if geometry.exact {
        return Ok(row + 1);
    }
    let at = stamp(row).map_err(Fault::Bar)?;
    Ok(if at >= ts { row } else { row + 1 })
}

/// Whether an index whose header already checked describes these committed
/// bars: the bar the header stamps first is row 0 in its slot, and the bar it
/// stamps last is row `n_valid - 1` in its slot.
///
/// Two entry reads, whatever the month holds. It cannot prove every entry in
/// between — that would be the O(n) audit this file exists to avoid — and it
/// does not claim to: an index written by `BarFile::append` is right by
/// construction, and what this catches is an index left behind by a write that
/// did not maintain it, whose last bar is not where the index says.
/// `docs/06-limits.md` D-2329 states the residue.
///
/// # Errors
///
/// [`Why::Stale`] when either bar is not where the index puts it, and whatever
/// `entry` refuses.
pub(crate) fn confirm(
    geometry: &Geometry,
    held: Held,
    mut entry: impl FnMut(u64) -> Result<Entry, Why>,
) -> Result<(), Why> {
    if held.n_valid == 0 {
        return Ok(());
    }
    for (ts, row) in [(held.first_ts, 0), (held.last_ts, held.n_valid - 1)] {
        let slot = geometry.slot_of(ts).ok_or(Why::Stale)?;
        let (bucket, bit) = split(slot);
        let found = entry(bucket)?;
        if !found.occupied(bit) || found.rank(bit) != row {
            return Err(Why::Stale);
        }
    }
    Ok(())
}

/// The entry an append continues from: the one holding the last committed
/// bar, with every slot above that bar cleared — a crash between the index
/// write and the header commit may have set bits for bars that never
/// committed — or bucket 0, empty, for a month with nothing in it.
///
/// One entry read.
///
/// # Errors
///
/// [`Why::Stale`] when the entry does not put the last bar where the header
/// does, and whatever `entry` refuses.
pub(crate) fn resume(
    geometry: &Geometry,
    held: Held,
    entry: impl FnOnce(u64) -> Result<Entry, Why>,
) -> Result<(u64, Entry), Why> {
    if held.n_valid == 0 {
        return Ok((0, Entry::EMPTY));
    }
    let slot = geometry.slot_of(held.last_ts).ok_or(Why::Stale)?;
    let (bucket, bit) = split(slot);
    let found = entry(bucket)?;
    if !found.occupied(bit) || found.rank(bit) != held.n_valid - 1 {
        return Err(Why::Stale);
    }
    Ok((bucket, found.through(bit)))
}

/// The entries from `bucket` on, after the bars stamped `stamps` are added to
/// `start` — which holds every bar before them and has nothing set above the
/// last of those.
///
/// One pass over `stamps`, plus one entry per bucket the stamps cross, so a
/// gap of `g` slots costs `g / 64` entries. Each stamp must lie in a slot
/// AFTER every slot already set and, on an intraday rung, BE that slot's first
/// instant; the first that does not is refused with its offset among
/// `stamps`, `first_row` added so the offset is a row.
///
/// # Errors
///
/// [`Why::Outside`], [`Why::OffGrid`] or [`Why::SharedSlot`] naming the first
/// stamp the index cannot hold, or whatever `stamps` itself yields as an
/// error, as [`Why::Unreadable`].
pub(crate) fn extend(
    geometry: &Geometry,
    bucket: u64,
    start: Entry,
    first_row: u64,
    stamps: impl IntoIterator<Item = Result<i64, StoreError>>,
) -> Result<Vec<Entry>, Why> {
    let mut current = start;
    let mut at = bucket;
    let mut out = Vec::new();
    for (offset, stamp) in (0u64..).zip(stamps) {
        let ts_micros = stamp.map_err(Why::Unreadable)?;
        let index = first_row.saturating_add(offset);
        let slot = geometry
            .slot_of(ts_micros)
            .ok_or(Why::Outside { index, ts_micros })?;
        if geometry.exact && geometry.slot_start(slot) != ts_micros {
            return Err(Why::OffGrid { index, ts_micros });
        }
        let (target, bit) = split(slot);
        if target < at {
            return Err(Why::SharedSlot { index, ts_micros });
        }
        while at < target {
            out.push(current);
            current = Entry {
                occupancy: 0,
                before: u32::try_from(current.total())
                    .map_err(|_| Why::Outside { index, ts_micros })?,
            };
            at += 1;
        }
        // A bar at or above `bit` in this bucket is a bar in this slot or a
        // later one: either way this stamp does not follow every bar indexed.
        if current.occupancy >> bit != 0 {
            return Err(Why::SharedSlot { index, ts_micros });
        }
        current.occupancy |= 1u64 << bit;
    }
    out.push(current);
    Ok(out)
}

/// Writes `bytes` into `out` at `at`, a no-op past its end. Every call site
/// passes a fixed offset inside a fixed array, so the clamp never bites; it
/// exists so the codec carries no indexing that could panic.
fn put(out: &mut [u8], at: usize, bytes: &[u8]) {
    if let Some(dst) = out.get_mut(at..at.saturating_add(bytes.len())) {
        dst.copy_from_slice(bytes);
    }
}

/// `N` bytes of `bytes` from `at`, or zeros past its end — see [`put`].
fn take<const N: usize>(bytes: &[u8], at: usize) -> [u8; N] {
    let mut out = [0u8; N];
    if let Some(run) = bytes.get(at..at.saturating_add(N)) {
        out.copy_from_slice(run);
    }
    out
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::indexing_slicing,
        clippy::expect_used,
        clippy::unwrap_used,
        clippy::panic
    )]

    use super::*;
    use std::cell::Cell;

    /// July 2024: 31 days, so the one-second ceiling is the real one.
    fn month() -> YearMonth {
        YearMonth::new(2024, 7).expect("a real month")
    }

    /// 2024-07-01 00:00 IST.
    const MONTH_START: i64 = 1_719_772_200_000_000;

    /// 09:15 IST on day `day` (0-based) of July 2024, plus `secs`.
    fn at(day: i64, secs: i64) -> i64 {
        MONTH_START + day * 86_400 * MICROS + (555 * 60 + secs) * MICROS
    }

    /// Every stamp of a month with `per_day` bars `step` seconds apart from
    /// 09:15 IST on each of `days` weekdays.
    fn session_month(days: i64, per_day: i64, step: i64) -> Vec<i64> {
        sessions(days, per_day, step, true)
    }

    /// As [`session_month`], on every day when `weekdays` is false.
    fn sessions(days: i64, per_day: i64, step: i64, weekdays: bool) -> Vec<i64> {
        let mut out = Vec::new();
        let mut day = 0;
        let mut taken = 0;
        while taken < days {
            // 2024-07-01 is a Monday: skip days 5, 6, 12, 13, ...
            if !weekdays || day % 7 < 5 {
                for k in 0..per_day {
                    out.push(at(day, k * step));
                }
                taken += 1;
            }
            day += 1;
        }
        out
    }

    /// The whole index of `stamps`, built the way the writer builds it.
    fn index_of(geometry: &Geometry, stamps: &[i64]) -> Vec<Entry> {
        extend(
            geometry,
            0,
            Entry::EMPTY,
            0,
            stamps.iter().map(|&ts| Ok(ts)),
        )
        .expect("the stamps are indexable")
    }

    fn held(stamps: &[i64]) -> Held {
        Held {
            n_valid: stamps.len() as u64,
            first_ts: stamps.first().copied().unwrap_or(0),
            last_ts: stamps.last().copied().unwrap_or(0),
        }
    }

    /// One lookup, counted: the answer, entry reads and bar reads.
    fn counted(geometry: &Geometry, entries: &[Entry], stamps: &[i64], ts: i64) -> (u64, u32, u32) {
        let entry_reads = Cell::new(0u32);
        let bar_reads = Cell::new(0u32);
        let row = locate(
            geometry,
            held(stamps),
            ts,
            |bucket| {
                entry_reads.set(entry_reads.get() + 1);
                entries
                    .get(usize::try_from(bucket).expect("fits"))
                    .copied()
                    .ok_or(Why::Entry { bucket })
            },
            |row| {
                bar_reads.set(bar_reads.get() + 1);
                Ok(stamps[usize::try_from(row).expect("fits")])
            },
        )
        .expect("an in-memory index answers");
        (row, entry_reads.get(), bar_reads.get())
    }

    /// A deterministic spread of probe stamps over and around `stamps`:
    /// every bar's own stamp, one microsecond either side of it, the middle
    /// of the gap after it, and both ends of the month — sampled with a
    /// stride so the one-second ceiling stays fast.
    fn probes(stamps: &[i64], stride: usize) -> Vec<i64> {
        let (from, until) = month().ist_bounds_micros();
        let mut out = vec![i64::MIN, from - 1, from, until - 1, until, i64::MAX];
        for (k, pair) in stamps.windows(2).enumerate().step_by(stride.max(1)) {
            let _ = k;
            out.extend([
                pair[0] - 1,
                pair[0],
                pair[0] + 1,
                i64::midpoint(pair[0], pair[1]),
            ]);
        }
        if let Some(&last) = stamps.last() {
            out.extend([last - 1, last, last + 1]);
        }
        out
    }

    /// The answer by definition: how many bars are stamped before `ts`.
    fn truth(stamps: &[i64], ts: i64) -> u64 {
        stamps.partition_point(|&s| s < ts) as u64
    }

    #[test]
    fn every_lookup_reads_at_most_one_entry_and_one_bar() {
        // D-2329. The read count of `locate` is a constant: at most ONE entry
        // and ONE bar, at every month size from empty to the one-second
        // ceiling of 31 × 86,400 = 2,678,400 bars, and the answer is the
        // bisection's answer — the count of bars stamped before `ts`.
        let minute = Geometry::new(month(), 60, 7);
        let second = Geometry::new(month(), 1, 7);
        let cases: Vec<(&str, Geometry, Vec<i64>, usize)> = vec![
            ("empty", minute, Vec::new(), 1),
            ("one bar", minute, vec![at(3, 0)], 1),
            ("one bucket", minute, session_month(1, 64, 60), 1),
            ("a bucket and one", minute, session_month(1, 65, 60), 1),
            ("one session", minute, session_month(1, 375, 60), 1),
            ("1min, 31 x 375", minute, sessions(31, 375, 60, false), 1),
            (
                "1min, every minute",
                minute,
                {
                    let (from, until) = month().ist_bounds_micros();
                    (0..(until - from) / (60 * MICROS))
                        .map(|k| from + k * 60 * MICROS)
                        .collect()
                },
                3,
            ),
            ("1s, 23 x 22,500", second, session_month(23, 22_500, 1), 97),
            (
                "1s, every second",
                second,
                {
                    let (from, until) = month().ist_bounds_micros();
                    (0..(until - from) / MICROS)
                        .map(|k| from + k * MICROS)
                        .collect()
                },
                499,
            ),
        ];
        for (name, geometry, stamps, stride) in cases {
            let entries = index_of(&geometry, &stamps);
            let mut probed = 0u64;
            for ts in probes(&stamps, stride) {
                let (row, entry_reads, bar_reads) = counted(&geometry, &entries, &stamps, ts);
                assert_eq!(row, truth(&stamps, ts), "{name}: ts={ts}");
                assert!(entry_reads <= 1, "{name}: {entry_reads} entry reads");
                // Every case here is intraday, whose bars are slot starts.
                assert_eq!(bar_reads, 0, "{name}: an intraday lookup read a bar");
                probed += 1;
            }
            assert!(probed >= 6, "{name}: probed");
        }
        // THE CEILING IS REAL: the every-second month is the largest a
        // one-second file admits, and it is indexed in 41,850 entries.
        assert_eq!(second.slot_count(), 2_678_400);
        assert_eq!(second.entry_count(), 41_850);
        assert_eq!(minute.slot_count(), 44_640);
        assert_eq!(minute.entry_count(), 698);
    }

    #[test]
    fn an_intraday_lookup_never_reads_a_bar() {
        let geometry = Geometry::new(month(), 60, 7);
        let stamps = session_month(2, 375, 60);
        let entries = index_of(&geometry, &stamps);
        // A bar's own stamp is its slot's first instant: no bar read.
        let (row, entries_read, bars_read) = counted(&geometry, &entries, &stamps, stamps[100]);
        assert_eq!((row, entries_read, bars_read), (100, 1, 0));
        // A query 30 s into an occupied minute: its bar is the minute's first
        // instant, so it is before the query, and nothing is read for it.
        let (row, entries_read, bars_read) =
            counted(&geometry, &entries, &stamps, stamps[100] + 30 * MICROS);
        assert_eq!((row, entries_read, bars_read), (101, 1, 0));
        // Outside the bars: no read at all.
        assert_eq!(counted(&geometry, &entries, &stamps, stamps[0]), (0, 0, 0));
        assert_eq!(
            counted(&geometry, &entries, &stamps, stamps[749] + 1),
            (750, 0, 0)
        );
        // Inside an empty slot (overnight): the entry alone answers.
        let (row, _, bars_read) = counted(&geometry, &entries, &stamps, at(1, -3_600));
        assert_eq!((row, bars_read), (375, 0));
    }

    #[test]
    fn a_daily_bar_anywhere_in_its_day_is_found_by_one_bar_read() {
        // The daily rung admits any whole second (D-0915): a bar at the close,
        // at midnight or at the open. A query inside its day reads it once.
        let geometry = Geometry::new(month(), 86_400, 7);
        assert_eq!(
            geometry.origin_micros(),
            MONTH_START,
            "daily slots are IST days"
        );
        assert_eq!(geometry.slot_count(), 31);
        let stamps = vec![at(3, 22_500), at(4, -555 * 60), at(5, 0), at(6, 400)];
        let entries = index_of(&geometry, &stamps);
        assert_eq!(entries.len(), 1);
        for ts in probes(&stamps, 1) {
            let (row, entry_reads, bar_reads) = counted(&geometry, &entries, &stamps, ts);
            assert_eq!(row, truth(&stamps, ts), "ts={ts}");
            assert!(entry_reads <= 1 && bar_reads <= 1);
        }
        let (row, _, bar_reads) = counted(&geometry, &entries, &stamps, at(3, 0));
        assert_eq!(
            (row, bar_reads),
            (0, 0),
            "before the first bar answers from the header"
        );
        let (row, _, bar_reads) = counted(&geometry, &entries, &stamps, at(6, 0));
        assert_eq!(
            (row, bar_reads),
            (3, 1),
            "09:15 on day 6 precedes its 09:21:40 bar"
        );
        let (row, _, bar_reads) = counted(&geometry, &entries, &stamps, at(5, 1));
        assert_eq!((row, bar_reads), (3, 1), "a second after the day-5 bar");
    }

    #[test]
    fn the_slots_are_the_grid_the_writer_admits() {
        let (from, until) = month().ist_bounds_micros();
        // One, three, five and fifteen minutes divide 09:15: slot 0 starts at
        // IST midnight, exactly as the admission's grid does.
        for secs in [1, 60, 180, 300, 900] {
            let g = Geometry::new(month(), secs, 7);
            assert_eq!(g.origin_micros(), from, "{secs}s");
            assert_eq!(g.slot_micros(), i64::from(secs) * MICROS);
            assert_eq!(g.slot_of(from), Some(0));
            assert_eq!(g.slot_of(until - 1), Some(g.slot_count() - 1));
            assert_eq!(g.slot_of(until), None);
            assert_eq!(g.slot_of(from - 1), None);
            assert_eq!(g.slot_start(g.slot_of(at(4, 0)).expect("in")), at(4, 0));
        }
        // Two, ten, thirty and sixty minutes do not: 09:15 is on their grid
        // and IST midnight is not, so slot 0 starts before the month and a
        // 09:15 bar is still a slot's first instant.
        for secs in [120, 600, 1_800, 3_600] {
            let g = Geometry::new(month(), secs, 7);
            assert!(g.origin_micros() < from, "{secs}s");
            assert!(from - g.origin_micros() < g.slot_micros());
            assert_eq!(g.slot_of(from), Some(0));
            let slot = g.slot_of(at(9, 0)).expect("in the month");
            assert_eq!(g.slot_start(slot), at(9, 0), "{secs}s: 09:15 starts a slot");
            assert_eq!(g.slot_of(until - 1), Some(g.slot_count() - 1));
        }
        assert_eq!(
            Geometry::new(month(), 0, 7).slot_micros(),
            1,
            "never a zero width"
        );
        assert_eq!(split(0), (0, 0));
        assert_eq!(split(63), (0, 63));
        assert_eq!(split(64), (1, 0));
        assert_eq!(entry_offset(0), 64);
        assert_eq!(entry_offset(3), 64 + 48);
    }

    #[test]
    fn the_header_is_its_geometry_and_every_other_byte_string_is_refused() {
        let g = Geometry::new(month(), 60, 7);
        let bytes = g.encode();
        assert_eq!(&bytes[0..8], b"BRUTEXT1");
        assert_eq!(u16::from_le_bytes([bytes[8], bytes[9]]), 1);
        assert_eq!(u16::from_le_bytes([bytes[10], bytes[11]]), 16);
        assert_eq!(u32::from_le_bytes(bytes[12..16].try_into().unwrap()), 60);
        assert_eq!(
            i64::from_le_bytes(bytes[16..24].try_into().unwrap()),
            MONTH_START
        );
        assert_eq!(
            i64::from_le_bytes(bytes[24..32].try_into().unwrap()),
            60 * MICROS
        );
        assert_eq!(
            u64::from_le_bytes(bytes[32..40].try_into().unwrap()),
            44_640
        );
        assert_eq!(u32::from_le_bytes(bytes[40..44].try_into().unwrap()), 7);
        assert!(bytes[44..60].iter().all(|&b| b == 0));
        assert_eq!(g.check(&bytes), Ok(()));

        // Every single flipped bit is refused.
        for bit in 0..512 {
            let mut bad = bytes;
            bad[bit / 8] ^= 1 << (bit % 8);
            assert!(g.check(&bad).is_err(), "bit {bit}");
        }
        let resealed = |mut raw: [u8; 64]| {
            let crc = crc32c(&raw[..60]);
            raw[60..64].copy_from_slice(&crc.to_le_bytes());
            raw
        };
        let mut magic = bytes;
        magic[7] = b'2';
        assert_eq!(
            g.check(&resealed(magic)),
            Err(Why::Header("its magic is not BRUTEXT1"))
        );
        let mut version = bytes;
        version[8] = 2;
        assert_eq!(
            g.check(&resealed(version)),
            Err(Why::Header("its version is not 1"))
        );
        assert_eq!(
            g.check(&Geometry::new(month(), 60, 8).encode()),
            Err(Why::Header(
                "its geometry is not the one this file's path and header imply"
            )),
            "another symbol's index"
        );
        assert_eq!(
            g.check(&[0u8; 64]),
            Err(Why::Header("its header fails its checksum"))
        );
    }

    #[test]
    fn an_entry_is_bound_to_its_position() {
        let entry = Entry {
            occupancy: 0x8000_0000_0000_0005,
            before: 1_234,
        };
        let raw = entry.encode(9);
        assert_eq!(Entry::decode(&raw, 9), Some(entry));
        assert_eq!(Entry::decode(&raw, 8), None, "copied to another bucket");
        assert_eq!(Entry::decode(&[0u8; 16], 9), None, "a hole of zeros");
        for bit in 0..128 {
            let mut bad = raw;
            bad[bit / 8] ^= 1 << (bit % 8);
            assert_eq!(Entry::decode(&bad, 9), None, "bit {bit}");
        }
        assert_eq!(entry.rank(0), 1_234);
        assert_eq!(entry.rank(1), 1_235);
        assert_eq!(entry.rank(3), 1_236);
        assert_eq!(entry.rank(63), 1_236);
        assert!(entry.occupied(0) && entry.occupied(2) && entry.occupied(63));
        assert!(!entry.occupied(1) && !entry.occupied(62));
        assert_eq!(entry.total(), 1_237);
        assert_eq!(entry.through(0).occupancy, 1);
        assert_eq!(entry.through(2).occupancy, 5);
        assert_eq!(entry.through(63), entry);
        assert_eq!(entry.through(62).occupancy, 5);
    }

    #[test]
    fn extend_fills_the_gaps_and_refuses_a_bar_it_cannot_hold() {
        let g = Geometry::new(month(), 60, 7);
        let (from, _) = month().ist_bounds_micros();
        let minute = |k: i64| from + k * 60 * MICROS;
        // Slots 1, 2 and 200: entries 0..=3, the empty ones carrying counts.
        let built = index_of(&g, &[minute(1), minute(2), minute(200)]);
        assert_eq!(built.len(), 4);
        assert_eq!(
            built[0],
            Entry {
                occupancy: 0b110,
                before: 0
            }
        );
        assert_eq!(
            built[1],
            Entry {
                occupancy: 0,
                before: 2
            }
        );
        assert_eq!(
            built[2],
            Entry {
                occupancy: 0,
                before: 2
            }
        );
        assert_eq!(
            built[3],
            Entry {
                occupancy: 1 << 8,
                before: 2
            }
        );

        let refuse = |stamps: &[i64]| index_of_err(&g, stamps);
        let daily = Geometry::new(month(), 86_400, 7);
        assert!(!daily.exact() && g.exact());
        assert_eq!(
            index_of_err(&daily, &[at(2, 0), at(2, 60)]),
            Why::SharedSlot {
                index: 1,
                ts_micros: at(2, 60)
            }
        );
        assert_eq!(
            refuse(&[minute(70), minute(3)]),
            Why::SharedSlot {
                index: 1,
                ts_micros: minute(3)
            },
            "a slot in an earlier bucket"
        );
        assert_eq!(
            refuse(&[minute(5), minute(4)]),
            Why::SharedSlot {
                index: 1,
                ts_micros: minute(4)
            },
            "an earlier slot in the same bucket"
        );
        assert_eq!(
            refuse(&[0]),
            Why::Outside {
                index: 0,
                ts_micros: 0
            }
        );
        assert_eq!(
            refuse(&[minute(5), minute(6) + 1]),
            Why::OffGrid {
                index: 1,
                ts_micros: minute(6) + 1
            },
            "an intraday bar off its slot's first instant"
        );
        let read_failure = extend(
            &g,
            0,
            Entry::EMPTY,
            0,
            [Err(StoreError::NotCommitted {
                index: 0,
                n_valid: 0,
            })],
        );
        assert_eq!(
            read_failure,
            Err(Why::Unreadable(StoreError::NotCommitted {
                index: 0,
                n_valid: 0
            }))
        );
        // Continuing from bucket 3 with a later bar adds to that entry.
        let more = extend(&g, 3, built[3], 3, [Ok(minute(250))]).expect("later");
        assert_eq!(
            more,
            vec![Entry {
                occupancy: (1 << 8) | (1 << 58),
                before: 2
            }]
        );
    }

    fn index_of_err(g: &Geometry, stamps: &[i64]) -> Why {
        extend(g, 0, Entry::EMPTY, 0, stamps.iter().map(|&ts| Ok(ts))).expect_err("refused")
    }

    #[test]
    fn resume_masks_bits_an_uncommitted_append_set_and_confirm_spots_a_stale_index() {
        let g = Geometry::new(month(), 60, 7);
        let committed = session_month(1, 100, 60);
        let mut ahead = committed.clone();
        ahead.extend(session_month(2, 375, 60)[375..400].iter().copied());
        let entries = index_of(&g, &ahead);
        let read = |bucket: u64| {
            entries
                .get(usize::try_from(bucket).expect("fits"))
                .copied()
                .ok_or(Why::Entry { bucket })
        };
        // The committed hundred are described exactly, whatever lies past them.
        assert_eq!(confirm(&g, held(&committed), read), Ok(()));
        let (bucket, start) = resume(&g, held(&committed), read).expect("resumes");
        let (last_bucket, last_bit) = split(g.slot_of(committed[99]).expect("in"));
        assert_eq!(bucket, last_bucket);
        assert_eq!(
            start,
            entries[usize::try_from(bucket).unwrap()].through(last_bit)
        );
        assert_eq!(start.total(), 100, "nothing past the commit survives");
        assert_eq!(resume(&g, held(&[]), read), Ok((0, Entry::EMPTY)));
        assert_eq!(confirm(&g, held(&[]), read), Ok(()));

        // A month that grew without the index: its last bar is not the row
        // the index puts there.
        let grown = Held {
            n_valid: ahead.len() as u64 + 1,
            ..held(&ahead)
        };
        assert_eq!(confirm(&g, grown, read), Err(Why::Stale));
        assert_eq!(resume(&g, grown, read), Err(Why::Stale));
        // A month whose first bar moved.
        assert_eq!(confirm(&g, held(&committed[1..]), read), Err(Why::Stale));
        // A header stamp outside the month.
        let outside = Held {
            n_valid: 1,
            first_ts: 0,
            last_ts: 0,
        };
        assert_eq!(confirm(&g, outside, read), Err(Why::Stale));
        assert_eq!(resume(&g, outside, read), Err(Why::Stale));
        // An entry that cannot be read is carried as itself.
        let none = |bucket: u64| Err(Why::Entry { bucket });
        let first_bucket = split(g.slot_of(committed[0]).expect("in")).0;
        assert_eq!(
            confirm(&g, held(&committed), none),
            Err(Why::Entry {
                bucket: first_bucket
            })
        );
        assert_eq!(
            resume(&g, held(&committed), none),
            Err(Why::Entry {
                bucket: last_bucket
            })
        );
    }

    #[test]
    fn a_lookup_the_index_cannot_answer_is_a_fault_and_never_a_guess() {
        let g = Geometry::new(month(), 60, 7);
        let stamps = session_month(1, 375, 60);
        let h = held(&stamps);
        let mid = stamps[200];
        let damaged = locate(&g, h, mid, |bucket| Err(Why::Entry { bucket }), |_| Ok(0));
        let bucket = split(g.slot_of(mid).expect("in")).0;
        assert_eq!(damaged, Err(Fault::Index(Why::Entry { bucket })));
        // An entry that puts the bar past the commit is stale, not an answer.
        let wrong = locate(
            &g,
            h,
            mid,
            |_| {
                Ok(Entry {
                    occupancy: 0,
                    before: 375,
                })
            },
            |_| Ok(0),
        );
        assert_eq!(wrong, Err(Fault::Index(Why::Stale)));
        // A header whose stamps are outside the month cannot be slotted.
        let outside = Held {
            n_valid: 2,
            first_ts: 0,
            last_ts: i64::MAX,
        };
        assert_eq!(
            locate(&g, outside, 5, |_| Ok(Entry::EMPTY), |_| Ok(0)),
            Err(Fault::Index(Why::Stale))
        );
        // A bar read that refuses travels out as the bar's refusal. Only the
        // daily rung reads a bar.
        let daily = Geometry::new(month(), 86_400, 7);
        let days = vec![at(2, 0), at(3, 0), at(4, 0)];
        let entries = index_of(&daily, &days);
        let refusal = StoreError::NotCommitted {
            index: 1,
            n_valid: 0,
        };
        let failed = locate(
            &daily,
            held(&days),
            at(3, 1),
            |bucket| Ok(entries[usize::try_from(bucket).unwrap()]),
            |_| Err(refusal.clone()),
        );
        assert_eq!(failed, Err(Fault::Bar(refusal)));
    }

    #[test]
    fn every_reason_reads_as_a_sentence() {
        let cases = [
            (Why::Absent, "the month has no .tix time index"),
            (Why::Header("x"), "the .tix time index is refused: x"),
            (
                Why::Entry { bucket: 4 },
                "the .tix entry for bucket 4 is missing or fails its checksum",
            ),
            (
                Why::Stale,
                "the .tix time index does not describe the committed bars",
            ),
            (
                Why::Outside {
                    index: 2,
                    ts_micros: 9,
                },
                "bar 2 is stamped 9, outside the month's slots",
            ),
            (
                Why::OffGrid {
                    index: 4,
                    ts_micros: 7,
                },
                "bar 4 is stamped 7, off its rung's grid",
            ),
            (
                Why::SharedSlot {
                    index: 3,
                    ts_micros: 8,
                },
                "bar 3 is stamped 8, in the slot of the bar before it",
            ),
            (Why::NotBars, "only bar files carry a time index"),
            (
                Why::Replaced,
                "the month's .bin path no longer names the file this handle holds, so its \
                 .tix may describe other bars",
            ),
        ];
        for (why, text) in cases {
            assert_eq!(why.to_string(), text);
        }
        let inner = StoreError::NotCommitted {
            index: 1,
            n_valid: 0,
        };
        assert_eq!(
            Why::Unreadable(inner.clone()).to_string(),
            format!("the .tix time index could not be used: {inner}")
        );
    }
}

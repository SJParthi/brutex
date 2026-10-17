//! The closing auction's prints, kept apart from trades (D-3320 item 3).
//!
//! # What is an auction print
//!
//! From 2026-08-03 a closing-auction-eligible share's continuous trading
//! ends at 15:15 (charter §7, NSE/CMTR/73845, NSE/CMTR/74466). The auction
//! then runs through its order-matching phase to 15:35: that timing rests on
//! a broker's dated implementation notice and is UNVERIFIED against an NSE
//! page or circular (charter §9, "The auction's phases";
//! [`super::AUCTION_BAND_END`]). In the GDFL capital-market files such a
//! share carries, in that band, a row whose bid and ask are both zero and
//! whose LTQ is positive: the auction's print (measured on 7 Oct 2026; its
//! price equals Zerodha's day close, D-3320). The match itself is stamped
//! from 15:28 at one price, and is often stated with a book beside it: when
//! every traded row of the match band is at one price, the booked ones are
//! prints too (SC-CLOSE-6, [`super::AUCTION_MATCH_FROM`]). The reader
//! ([`super::decode_with`]) diverts every such row of an eligible share out
//! of the rows the fold sees and into [`super::CmFile::auction`]; this module
//! files them. A print is never folded into a one-second bar, and none is
//! dropped (the operator's answer, D-3320: a silent drop is the fault
//! `CLAUDE.md` §4 bans).
//!
//! Identical consecutive rows (one second, one price, one quantity) are one
//! [`Print`] with their count in `rows`: whether two such rows are two fills
//! or one stated twice is UNVERIFIED (charter; HDFCBANK carries two), so the
//! quantity is the row's own and is never summed.
//!
//! # The `.auc` sidecar, format version 1
//!
//! One file per instrument-month, beside the month's one-second bar file and
//! never inside it: `bars/gdfl/NSE/CASH/<SYM>/1s/<YYYY-MM>.auc` beside
//! `<YYYY-MM>.bin`. Every integer is little-endian.
//!
//! | Bytes | Field | Value |
//! |---|---|---|
//! | 0..8 | magic | ASCII `BRTXAUC1` |
//! | 8..10 | version | `u16`, 1 |
//! | 10..12 | record length | `u16`, 32 |
//! | 12..16 | reserved | `u32`, 0 |
//!
//! then zero or more 32-byte records, day after day, and within a day in the
//! order the vendor's file states them:
//!
//! | Bytes | Field | Type |
//! |---|---|---|
//! | 0..8 | `t_micros`: the UTC microsecond of the print's second | `i64` |
//! | 8..16 | `price_paisa` | `i64` |
//! | 16..24 | `quantity`: the row's LTQ | `u64` |
//! | 24..28 | `rows`: identical consecutive rows merged | `u32` |
//! | 28..32 | reserved | `u32`, 0 |
//!
//! A file whose length is not 16 plus a whole number of records, or whose
//! header is not this one, is refused by name and never repaired.
//! `docs/02-store-format.md` §8 is WP-99's to transcribe.
//!
//! # Append-only, idempotent (`CLAUDE.md` §3 rules 5 and 8)
//!
//! A day's prints are appended once, after every held record. Offered again,
//! byte for byte the records already held for that day, they write nothing.
//! A day whose records differ from the held ones, or a day older than one the
//! month already holds, is refused by name: it is a new generation of the
//! month (WP-80), never an in-place change or an insertion.
//!
//! # Cost
//!
//! One append reads the month's sidecar whole (a few records a day, at most
//! a month of days: bounded by the month, not by the store), takes the
//! file's exclusive advisory lock, writes the day's records once and syncs
//! them. Not a constant-time claim; it is O(records in the month) per
//! appended day, off every read path.

use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use brutex_core::instrument::InstrumentKey;
use brutex_core::vendor::Vendor;
use store::flock::Flock;
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

use super::{CmRefusal, CmRow};
use crate::session::{Day, IST_OFFSET_SECS};

/// The sidecar's magic, `BRTXAUC1`.
pub const MAGIC: [u8; 8] = *b"BRTXAUC1";

/// The format version this module writes and reads.
pub const VERSION: u16 = 1;

/// The header's length in bytes.
pub const HEADER_LEN: usize = 16;

/// One record's length in bytes.
pub const RECORD_LEN: usize = 32;

/// The sidecar's file extension.
pub const EXTENSION: &str = "auc";

/// One auction print as the reader diverted it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Print {
    /// The second of the IST day it is stamped at.
    pub sod: u32,
    /// Its price in paisa.
    pub price: i64,
    /// Its quantity: the row's LTQ, never a sum.
    pub quantity: u64,
    /// How many identical consecutive rows stated it.
    pub rows: u32,
    /// The one-based line of its first row in the file.
    pub line: u32,
}

/// Adds one diverted row: a row identical to the last print (one second, one
/// price, one quantity) raises its count; any other row is a new print.
pub(crate) fn merge(prints: &mut Vec<Print>, row: CmRow) {
    if let Some(last) = prints.last_mut()
        && last.sod == row.sod
        && last.price == row.ltp
        && last.quantity == row.ltq
    {
        last.rows = last.rows.saturating_add(1);
        return;
    }
    prints.push(Print {
        sod: row.sod,
        price: row.ltp,
        quantity: row.ltq,
        rows: 1,
        line: row.line,
    });
}

/// One record of a sidecar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Record {
    /// The UTC microsecond of the print's second.
    pub t_micros: i64,
    /// The price in paisa.
    pub price_paisa: i64,
    /// The row's LTQ.
    pub quantity: u64,
    /// Identical consecutive rows merged into it.
    pub rows: u32,
}

impl Record {
    /// `print` of `day` as a record.
    #[must_use]
    pub fn of(day: Day, print: &Print) -> Self {
        Self {
            t_micros: (i64::from(day.days_from_epoch()) * 86_400 + i64::from(print.sod)
                - IST_OFFSET_SECS)
                * 1_000_000,
            price_paisa: print.price,
            quantity: print.quantity,
            rows: print.rows,
        }
    }

    /// The record's 32 bytes.
    #[must_use]
    pub fn to_bytes(&self) -> [u8; RECORD_LEN] {
        let mut out = [0_u8; RECORD_LEN];
        let parts: [&[u8]; 4] = [
            &self.t_micros.to_le_bytes(),
            &self.price_paisa.to_le_bytes(),
            &self.quantity.to_le_bytes(),
            &self.rows.to_le_bytes(),
        ];
        let mut at = 0;
        for part in parts {
            if let Some(slot) = out.get_mut(at..at + part.len()) {
                slot.copy_from_slice(part);
            }
            at += part.len();
        }
        out
    }

    /// A record read back, or `None` when `bytes` is not one.
    #[must_use]
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let field = |at: usize| -> Option<[u8; 8]> { bytes.get(at..at + 8)?.try_into().ok() };
        let rows: [u8; 4] = bytes.get(24..28)?.try_into().ok()?;
        let reserved: [u8; 4] = bytes.get(28..32)?.try_into().ok()?;
        (bytes.len() == RECORD_LEN && reserved == [0; 4]).then_some(())?;
        Some(Self {
            t_micros: i64::from_le_bytes(field(0)?),
            price_paisa: i64::from_le_bytes(field(8)?),
            quantity: u64::from_le_bytes(field(16)?),
            rows: u32::from_le_bytes(rows),
        })
    }
}

/// The header's 16 bytes.
#[must_use]
pub fn header() -> [u8; HEADER_LEN] {
    let mut out = [0_u8; HEADER_LEN];
    if let Some(slot) = out.get_mut(..8) {
        slot.copy_from_slice(&MAGIC);
    }
    if let Some(slot) = out.get_mut(8..10) {
        slot.copy_from_slice(&VERSION.to_le_bytes());
    }
    if let Some(slot) = out.get_mut(10..12) {
        slot.copy_from_slice(&u16::try_from(RECORD_LEN).unwrap_or(0).to_le_bytes());
    }
    out
}

/// Every record of a sidecar's bytes.
///
/// # Errors
///
/// A header that is not version 1's, or a length that is not the header
/// plus whole records, named.
pub fn parse(bytes: &[u8]) -> Result<Vec<Record>, String> {
    let (head, body) = bytes
        .split_at_checked(HEADER_LEN)
        .ok_or_else(|| format!("{} bytes, shorter than the header", bytes.len()))?;
    if head != header() {
        return Err("the header is not BRTXAUC1 version 1 with 32-byte records".to_owned());
    }
    if body.len() % RECORD_LEN != 0 {
        return Err(format!(
            "{} bytes after the header, not a whole number of {RECORD_LEN}-byte records",
            body.len()
        ));
    }
    body.chunks(RECORD_LEN)
        .map(|chunk| Record::from_bytes(chunk).ok_or_else(|| "a record is not one".to_owned()))
        .collect()
}

/// The sidecar beside `key`'s one-second month file of `day`'s month.
///
/// # Errors
///
/// What the store's path builder refuses (an instrument or a month it cannot
/// file), named.
pub fn path(store_root: &Path, key: &InstrumentKey, day: Day) -> Result<PathBuf, String> {
    let month = YearMonth::new(day.year(), day.month()).map_err(|why| why.to_string())?;
    let bars = StorePath::new(PathParts {
        vendor: Vendor::Gdfl,
        exchange: key.exchange.as_str(),
        segment: key.segment.as_str(),
        symbol: key.underlying.as_str(),
        contract: None,
        timeframe: Timeframe::SECOND_1,
        month,
        file: FileKind::Bars,
    })
    .map_err(|why| why.to_string())?;
    Ok(bars.to_path_buf(store_root).with_extension(EXTENSION))
}

/// What one append did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Appended {
    /// Records offered for the day.
    pub offered: usize,
    /// Records written; 0 when the day's records were already held.
    pub written: usize,
}

/// Files `day`'s `prints` of `key` in the month's sidecar (see the module
/// header for the bytes and the rules).
///
/// # Errors
///
/// [`CmRefusal::AuctionSidecar`] naming why: the path, the file, its lock or
/// its bytes; a day older than one the month holds; a day whose records
/// differ from those held for it.
pub fn append(
    store_root: &Path,
    key: &InstrumentKey,
    day: Day,
    prints: &[Print],
) -> Result<Appended, CmRefusal> {
    let refuse = |why: String| CmRefusal::AuctionSidecar { why };
    let offered: Vec<Record> = prints.iter().map(|p| Record::of(day, p)).collect();
    let appended = Appended {
        offered: offered.len(),
        written: 0,
    };
    if offered.is_empty() {
        return Ok(appended);
    }
    let path = path(store_root, key, day).map_err(refuse)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|why| refuse(format!("{}: {why}", parent.display())))?;
    }
    let file = OpenOptions::new()
        .read(true)
        .append(true)
        .create(true)
        .open(&path)
        .map_err(|why| refuse(format!("{}: {why}", path.display())))?;
    let lock = Flock::try_lock(file, path.clone()).map_err(|why| {
        refuse(format!(
            "{} is busy or cannot be locked: {why}",
            path.display()
        ))
    })?;
    let written = write_day(&lock, &path, day, &offered).map_err(refuse);
    lock.release().map_err(|why| {
        refuse(format!(
            "{}: the lock was not released: {why}",
            path.display()
        ))
    })?;
    let written = written?;
    note_filed(key, day, &path, offered.len(), written);
    Ok(Appended {
        written,
        ..appended
    })
}

/// Under the lock: reads the held records, decides, and appends.
fn write_day(file: &File, path: &Path, day: Day, offered: &[Record]) -> Result<usize, String> {
    let mut bytes = Vec::new();
    let mut reader = file;
    reader
        .read_to_end(&mut bytes)
        .map_err(|why| format!("{}: {why}", path.display()))?;
    let fresh = bytes.is_empty();
    let held = if fresh {
        Vec::new()
    } else {
        parse(&bytes).map_err(|why| format!("{}: {why}", path.display()))?
    };
    let start = (i64::from(day.days_from_epoch()) * 86_400 - IST_OFFSET_SECS) * 1_000_000;
    let end = start + 86_400 * 1_000_000;
    let held_day: Vec<Record> = held
        .iter()
        .filter(|r| r.t_micros >= start && r.t_micros < end)
        .copied()
        .collect();
    if !held_day.is_empty() {
        if held_day == offered {
            return Ok(0);
        }
        return Err(format!(
            "{}: {day} holds {} records and {} different ones were offered; a new generation, \
             never an in-place change",
            path.display(),
            held_day.len(),
            offered.len()
        ));
    }
    if held.last().is_some_and(|last| last.t_micros >= end) {
        return Err(format!(
            "{}: the month holds a day after {day}; {day} is a new generation, never an \
             insertion",
            path.display()
        ));
    }
    let mut out = Vec::with_capacity(HEADER_LEN + offered.len() * RECORD_LEN);
    if fresh {
        out.extend_from_slice(&header());
    }
    for record in offered {
        out.extend_from_slice(&record.to_bytes());
    }
    let mut writer = file;
    writer
        .write_all(&out)
        .and_then(|()| file.sync_data())
        .map_err(|why| format!("{}: {why}", path.display()))?;
    Ok(offered.len())
}

/// One `info` line per filed day (D-3320): the share, the day, the prints
/// offered and written, and the sidecar.
fn note_filed(key: &InstrumentKey, day: Day, path: &Path, offered: usize, written: usize) {
    let day_text = day.to_string();
    let path_text = path.display().to_string();
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::info(crate::gdfl_import::TARGET, "auction prints filed")
            .with("symbol", key.underlying.as_str())
            .with("day", day_text.as_str())
            .with("prints", offered)
            .with("written", written)
            .with("sidecar", path_text.as_str()),
    );
}

#[cfg(test)]
#[allow(
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail"
)]
mod cloud_mutant_tests {
    //! Mutant kills for [`merge`] (cloud mutation round, 2026-10-16). Every
    //! price and quantity is invented.

    use super::{CmRow, Print, merge};

    const fn row(line: u32, sod: u32, ltp: i64, ltq: u64) -> CmRow {
        CmRow {
            line,
            sod,
            ltp,
            ltq,
        }
    }

    /// Only a row identical to the last print in ALL THREE of second, price
    /// and quantity raises its count; a row differing in any one of them is
    /// a new print, and the merged print keeps its FIRST row's line. Kills
    /// each `&&` of the merge test read as `||` and each `==` read as `!=`.
    #[test]
    fn a_row_merges_only_when_second_price_and_quantity_all_match() {
        let first = row(10, 55_687, 123_405, 7);
        for (other, what) in [
            (row(11, 55_688, 123_405, 7), "another second"),
            (row(11, 55_687, 123_410, 7), "another price"),
            (row(11, 55_687, 123_405, 8), "another quantity"),
        ] {
            let mut prints = Vec::new();
            merge(&mut prints, first);
            merge(&mut prints, other);
            assert_eq!(prints.len(), 2, "{what} is a new print");
            assert_eq!(
                prints[1],
                Print {
                    sod: other.sod,
                    price: other.ltp,
                    quantity: other.ltq,
                    rows: 1,
                    line: 11,
                },
                "{what}"
            );
            assert_eq!(prints[0].rows, 1, "{what} leaves the first alone");
        }
        let mut prints = Vec::new();
        merge(&mut prints, first);
        merge(&mut prints, row(11, 55_687, 123_405, 7));
        merge(&mut prints, row(12, 55_687, 123_405, 7));
        assert_eq!(
            prints,
            [Print {
                sod: 55_687,
                price: 123_405,
                quantity: 7,
                rows: 3,
                line: 10,
            }],
            "three identical rows are one print stated three times, its quantity never summed"
        );
    }

    /// The count saturates rather than wrapping to zero.
    #[test]
    fn a_merged_count_saturates() {
        let mut prints = vec![Print {
            sod: 1,
            price: 2,
            quantity: 3,
            rows: u32::MAX,
            line: 4,
        }];
        merge(&mut prints, row(5, 1, 2, 3));
        assert_eq!(prints[0].rows, u32::MAX);
        assert_eq!(prints.len(), 1);
    }

    /// A directory where the month's sidecar belongs is refused by the
    /// sidecar's own name, and nothing is written inside it. Invented share.
    #[test]
    #[allow(
        clippy::unwrap_used,
        clippy::panic,
        reason = "a test that cannot panic cannot fail"
    )]
    fn a_directory_at_the_sidecar_name_is_refused_by_name() {
        let root = crate::gdfl_fixtures::scratch("CloudSidecarDir");
        let key = super::InstrumentKey::cash(brutex_core::instrument::Exchange::Nse, "ZZAUCTION")
            .unwrap();
        let day = super::Day::new(2026, 8, 4).unwrap();
        let sidecar = super::path(&root, &key, day).unwrap();
        std::fs::create_dir_all(&sidecar).unwrap();
        let print = Print {
            sod: 55_687,
            price: 140_150,
            quantity: 400,
            rows: 1,
            line: 3,
        };
        let why = match super::append(&root, &key, day, &[print]) {
            Err(super::CmRefusal::AuctionSidecar { why }) => why,
            other => panic!("{other:?}"),
        };
        assert!(
            why.starts_with(&format!("{}: ", sidecar.display())),
            "{why}"
        );
        assert!(std::fs::read_dir(&sidecar).unwrap().next().is_none());
        std::fs::remove_dir_all(root).unwrap();
    }
}

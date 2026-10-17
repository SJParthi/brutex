//! `store::sealed` (SC-2): the `.sbar` version 1 file, built and read back.
//!
//! Every fixture is invented: prices are walks from 7,777.77 rupees (777,777
//! paisa), volumes and open interest are counters, and no row resembles a
//! vendor's. Months are built in memory and read through [`Positional`] on a
//! `Vec<u8>`; one test writes a file and reads it through a `File`. Every
//! test that makes a door refuse holds `crate::emits::hold_the_sink()`: each
//! refusal writes one `store.sealed` line, and the emit test counts the lines
//! of its own window.
#![cfg(test)]
#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::unwrap_used,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::too_many_lines,
    clippy::print_stdout
)]

use super::{
    BISECT_IN_ONE_READ, Builder, Built, ColumnRange, ENTRY_LEN, FLAG_PROVENANCE, FLAG_SUMMARY,
    FLAG_TIME_TABLE, HEAD_LEN, Head, KNOWN_FLAGS, MAGIC, MAX_PROVENANCE_LEN, MAX_RANGE_ROWS,
    MAX_SUMMARY_LEN, Positional, SealedError, SealedMonth, Summary, TABLE_AFTER_BLOCKS,
    TABLE_ENTRY_LEN, VERSION, build, entry_image, head_crc, pack_provenance, slot_crc, table_image,
    unpack_provenance,
};
use crate::block;
use crate::crc::crc32c;
use crate::format::{Bar, FLAG_CHECKSUMS, FormatError, OI_NULL, RECORD_LEN};
use crate::header::Header;
use crate::layout::Layout;
use crate::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};
use crate::provenance;
use crate::sealed_codec::{BLOCK_ROWS, FrameError};
use crate::sort_index::{self, Shape};
use brutex_core::instrument::Contract;
use brutex_core::vendor::Vendor;
use std::cell::Cell;
use std::io;
use std::time::Instant;

const SYMBOL: u32 = 4_242;
const GENERATION: u64 = 7;
const SECOND: i64 = 1_000_000;
const MINUTE: i64 = 60 * SECOND;
const DAY: i64 = 86_400 * SECOND;
/// 09:15 IST after the month's origin (00:00 IST).
const OPEN: i64 = (9 * 60 + 15) * MINUTE;

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
    fn signed(&mut self, magnitude: i64) -> i64 {
        (self.below(2 * magnitude as u64 + 1)) as i64 - magnitude
    }
}

fn june() -> YearMonth {
    YearMonth::new(2024, 6).unwrap()
}

fn january() -> YearMonth {
    YearMonth::new(2026, 1).unwrap()
}

fn path_of(
    vendor: Vendor,
    symbol: &'static str,
    timeframe: Timeframe,
    month: YearMonth,
) -> StorePath<'static> {
    StorePath::new(PathParts {
        vendor,
        exchange: "NSE",
        segment: "INDEX",
        symbol,
        contract: None,
        timeframe,
        month,
        file: FileKind::Bars,
    })
    .unwrap()
}

fn nifty(timeframe: Timeframe, month: YearMonth) -> StorePath<'static> {
    path_of(Vendor::Gdfl, "NIFTY", timeframe, month)
}

fn origin(month: YearMonth) -> i64 {
    month.ist_bounds_micros().0
}

fn header(timeframe: Timeframe, bars: &[Bar]) -> Header {
    Header {
        format_version: 3,
        record_stride: 56,
        flags: FLAG_CHECKSUMS,
        generation: GENERATION,
        n_valid: bars.len() as u64,
        first_ts_micros: bars.first().map_or(0, |bar| bar.ts_micros),
        last_ts_micros: bars.last().map_or(0, |bar| bar.ts_micros),
        symbol_id: SYMBOL,
        timeframe_secs: timeframe.secs(),
    }
}

/// `n` invented bars from `first`, `step` apart, a random walk of prices.
fn walk(n: usize, first: i64, step: i64, seed: u64, oi: bool) -> Vec<Bar> {
    let mut rng = Rng(seed);
    let mut close = 777_777_i64;
    (0..n)
        .map(|i| {
            let open = close + rng.signed(40);
            close = open + rng.signed(60);
            let high = open.max(close) + rng.below(30) as i64;
            let low = open.min(close) - rng.below(30) as i64;
            Bar {
                ts_micros: first + step * i as i64,
                open,
                high,
                low,
                close,
                volume: rng.below(5_000) as i64,
                open_interest: if oi { 10_000 + (i as i64 / 7) } else { OI_NULL },
            }
        })
        .collect()
}

/// One-second bars from 09:15 IST on the 1st, as many as asked.
fn seconds(n: usize, seed: u64) -> Vec<Bar> {
    walk(n, origin(june()) + OPEN, SECOND, seed, false)
}

fn sealed(path: StorePath<'_>, bars: &[Bar]) -> Built {
    let timeframe = path.timeframe();
    build(path, &header(timeframe, bars), bars, None, None).unwrap()
}

fn open(path: StorePath<'_>, bytes: Vec<u8>) -> SealedMonth<Vec<u8>> {
    SealedMonth::open(bytes, path, SYMBOL).unwrap()
}

fn refusal<T: std::fmt::Debug>(result: Result<T, SealedError>) -> SealedError {
    match result {
        Ok(value) => panic!("expected a refusal, got {value:?}"),
        Err(refusal) => refusal,
    }
}

/// Recomputes the slot checksum and then the head checksum of a file image.
fn reseal(bytes: &mut [u8]) {
    let slot = slot_crc(bytes);
    bytes[56..60].copy_from_slice(&slot.to_le_bytes());
    let whole = head_crc(bytes);
    bytes[508..512].copy_from_slice(&whole.to_le_bytes());
}

/// The first row at or after `ts`, by reading every row.
fn scan(bars: &[Bar], ts: i64) -> u64 {
    bars.partition_point(|bar| bar.ts_micros < ts) as u64
}

/// A `.prv` image for `path`: its header and `records` invented 256-byte
/// records that repeat most fields from one to the next, as real ones do.
fn prv_image(path: StorePath<'_>, records: usize) -> Vec<u8> {
    let mut image = provenance::Header::of(path, SYMBOL).image().to_vec();
    for record in 0..records {
        let mut bytes = [0u8; 256];
        bytes[0..8].copy_from_slice(&(record as u64 * 3_600).to_le_bytes());
        bytes[8..16].copy_from_slice(&((record as u64 + 1) * 3_600).to_le_bytes());
        bytes[16..48].copy_from_slice(b"invented-source-name-for-a-test!");
        bytes[200] = record as u8;
        let crc = crc32c(&bytes[..252]);
        bytes[252..256].copy_from_slice(&crc.to_le_bytes());
        image.extend_from_slice(&bytes);
    }
    image
}

/// Counts the reads made through it.
#[derive(Debug)]
struct Counting {
    bytes: Vec<u8>,
    reads: Cell<u64>,
    read_bytes: Cell<u64>,
}

impl Counting {
    fn new(bytes: Vec<u8>) -> Self {
        Self {
            bytes,
            reads: Cell::new(0),
            read_bytes: Cell::new(0),
        }
    }
    fn take(&self) -> (u64, u64) {
        (self.reads.replace(0), self.read_bytes.replace(0))
    }
}

impl Positional for Counting {
    fn read_exact_at(&self, buf: &mut [u8], offset: u64) -> io::Result<()> {
        self.reads.set(self.reads.get() + 1);
        self.read_bytes
            .set(self.read_bytes.get() + buf.len() as u64);
        self.bytes.read_exact_at(buf, offset)
    }
    fn byte_len(&self) -> io::Result<u64> {
        self.bytes.byte_len()
    }
}

/// Refuses the read that crosses `fail_at`, or measuring, on demand.
#[derive(Debug)]
struct Failing {
    bytes: Vec<u8>,
    fail_at: u64,
    fail_len: bool,
}

impl Positional for Failing {
    fn read_exact_at(&self, buf: &mut [u8], offset: u64) -> io::Result<()> {
        if offset <= self.fail_at && self.fail_at < offset + buf.len() as u64 {
            return Err(io::Error::other("an invented device error"));
        }
        self.bytes.read_exact_at(buf, offset)
    }
    fn byte_len(&self) -> io::Result<u64> {
        if self.fail_len {
            return Err(io::Error::other("an invented stat error"));
        }
        self.bytes.byte_len()
    }
}

/// Reads every row back and checks every lookup a month can be asked.
fn check_reads(month: &SealedMonth<Vec<u8>>, bars: &[Bar]) {
    let head = month.head();
    let rows = u64::from(head.block_rows);
    let mut all = Vec::new();
    for block in 0..u64::from(head.n_blocks) {
        let got = month.read_block(block, &mut all).unwrap();
        assert_eq!(got as u64, head.rows_in(block));
    }
    assert_eq!(all, bars);
    // Pages across every frame boundary, and single rows.
    let n = bars.len() as u64;
    for first in (0..n).step_by(97).chain([n.saturating_sub(1)]) {
        for count in [1, 2, rows - 1, rows, rows + 1, 1_000] {
            let count = count.min(n - first);
            let mut page = vec![Bar::default()];
            assert_eq!(
                month.read_rows(first, count, &mut page).unwrap() as u64,
                count
            );
            assert_eq!(&page[1..], &bars[first as usize..(first + count) as usize]);
        }
    }
    // Every stamp, one before and one after.
    for bar in bars.iter().step_by(13).chain(bars.last()) {
        for ts in [bar.ts_micros - 1, bar.ts_micros, bar.ts_micros + 1] {
            assert_eq!(
                month.first_at_or_after(ts).unwrap(),
                scan(bars, ts),
                "ts {ts}"
            );
        }
    }
    for ts in [i64::MIN, head.origin_micros, i64::MAX] {
        assert_eq!(month.first_at_or_after(ts).unwrap(), scan(bars, ts));
    }
    let audited = month.audit().unwrap();
    assert_eq!(audited.rows, n);
    assert_eq!(audited.frames, u64::from(head.n_blocks));
    assert_eq!(audited.bytes, head.file_len);
}

#[test]
fn an_empty_month_is_a_head_alone() {
    // Every refusal below is one `store.sealed` line: held off the sink
    // `crate::emits` reads, so its count is its own. Held on this thread,
    // so a body that never returns lets go of it at the deadline.
    let _sink = crate::emits::hold_the_sink();
    within_a_minute(empty_month_body);
}

/// The test above, on a worker with a deadline: an audit of an empty month
/// that never ends (`audited`'s `chunk < n_blocks` made `==`) fails it.
fn empty_month_body() {
    let path = nifty(Timeframe::SECOND_1, june());
    let built = sealed(path, &[]);
    let bytes = built.to_bytes();
    assert_eq!(bytes.len() as u64, HEAD_LEN);
    assert_eq!(built.file_len(), HEAD_LEN);
    let month = open(path, bytes);
    let head = *month.head();
    assert_eq!((head.n_valid, head.n_blocks, head.flags), (0, 0, 0));
    assert_eq!(head.blocks_len, 0);
    assert_eq!(head.columns, [ColumnRange::default(); 6]);
    assert_eq!(head.records_blake3, brutex_core::blake3::hash(&[]));
    assert_eq!(head.crc_image_crc, crc32c(&[]));
    for ts in [i64::MIN, 0, origin(june()), i64::MAX] {
        assert_eq!(month.first_at_or_after(ts).unwrap(), 0);
    }
    let mut out = vec![Bar::default()];
    assert_eq!(month.read_rows(0, 0, &mut out).unwrap(), 0);
    assert_eq!(refusal(month.read_rows(0, 1, &mut out)).reason(), "range");
    assert_eq!(refusal(month.read_block(0, &mut out)).reason(), "range");
    assert_eq!(out.len(), 1);
    assert_eq!(month.provenance().unwrap(), None);
    assert_eq!(month.summary().unwrap(), None);
    assert_eq!(month.audit().unwrap().rows, 0);

    // An empty raw month keeps whatever stamps its header carries, and a
    // provenance image alone follows the head.
    let mut raw = header(Timeframe::SECOND_1, &[]);
    raw.first_ts_micros = 5;
    raw.last_ts_micros = 9;
    let image = prv_image(path, 0);
    let built = build(path, &raw, &[], Some(&image), None).unwrap();
    let packed = pack_provenance(&image).len() as u64;
    assert_eq!(built.file_len(), HEAD_LEN + packed + 4);
    let month = open(path, built.to_bytes());
    assert_eq!(month.head().bar_header(), raw);
    assert_eq!(month.provenance().unwrap(), Some(image));
    assert_eq!(month.audit().unwrap().rows, 0);
}

#[test]
fn one_bar_is_one_frame_and_every_lookup_finds_it() {
    let path = nifty(Timeframe::SECOND_1, june());
    let bars = seconds(1, 1);
    let month = open(path, sealed(path, &bars).to_bytes());
    assert_eq!(month.head().n_blocks, 1);
    check_reads(&month, &bars);
}

#[test]
fn the_boundaries_of_a_frame_and_of_the_sort_threshold() {
    let path = nifty(Timeframe::SECOND_1, june());
    let block = usize::from(BLOCK_ROWS);
    let table_at = TABLE_AFTER_BLOCKS as usize * block;
    for n in [
        block - 1,
        block,
        block + 1,
        1_023,
        1_024,
        1_025,
        table_at,
        table_at + 1,
        table_at + block + 1,
    ] {
        let bars = seconds(n, n as u64);
        let built = sealed(path, &bars);
        let month = open(path, built.to_bytes());
        let head = *month.head();
        assert_eq!(u64::from(head.n_blocks), (n as u64).div_ceil(256), "{n}");
        let table = u64::from(head.n_blocks) > TABLE_AFTER_BLOCKS;
        assert_eq!(head.flags & FLAG_TIME_TABLE != 0, table, "{n}");
        assert_eq!(
            head.table_off,
            HEAD_LEN + ENTRY_LEN * u64::from(head.n_blocks)
        );
        if table {
            // A thirty-day month at one second: ceil(2,592,000 / 4,096).
            assert_eq!(head.table_entries, 633);
        } else {
            assert_eq!(head.table_entries, 0);
        }
        check_reads(&month, &bars);
    }
}

#[test]
fn the_largest_month_reads_at_the_cost_of_the_smallest() {
    // Every second of a thirty-one-day month: 2,678,400 bars.
    let month_of = january();
    let path = nifty(Timeframe::SECOND_1, month_of);
    let mut rng = Rng(99);
    let largest = walk(2_678_400, origin(month_of), SECOND, 5, false);
    let built = sealed(path, &largest);
    let head = *built.head();
    assert_eq!(head.n_blocks, 10_463);
    assert_eq!(head.table_off - HEAD_LEN, 10_463 * 24);
    assert_eq!(head.table_entries, 654);
    let bytes = built.to_bytes();
    println!(
        "largest month: {} bars, {} bytes, {} hundredths of a byte a bar",
        largest.len(),
        bytes.len(),
        bytes.len() * 100 / largest.len()
    );
    let big = SealedMonth::open(Counting::new(bytes), path, SYMBOL).unwrap();
    let small_bars = seconds(554, 3);
    let small = SealedMonth::open(
        Counting::new(sealed(nifty(Timeframe::SECOND_1, june()), &small_bars).to_bytes()),
        nifty(Timeframe::SECOND_1, june()),
        SYMBOL,
    )
    .unwrap();
    assert_eq!(big.source.take(), (1, 512));
    assert_eq!(small.source.take(), (1, 512));

    // A 100-row page: one read of directory entries, one of frames, at any
    // depth of either month.
    let mut timings = Vec::new();
    for _ in 0..2_000 {
        for (month, bars) in [(&big, &largest), (&small, &small_bars)] {
            let n = bars.len() as u64;
            let first = rng.below(n - 100);
            let mut page = Vec::new();
            let started = Instant::now();
            month.read_rows(first, 100, &mut page).unwrap();
            timings.push((n, started.elapsed()));
            assert_eq!(page, bars[first as usize..first as usize + 100]);
            let (reads, read_bytes) = month.source.take();
            assert_eq!(reads, 2, "page at {first} of {n}");
            assert!(read_bytes <= 3 * 24 + 2 * 14_508, "{read_bytes}");
        }
    }
    // A time lookup: one table read (when the month has one), one read of
    // directory entries, one frame's stamps.
    for _ in 0..2_000 {
        for (month, bars) in [(&big, &largest), (&small, &small_bars)] {
            let first = bars[0].ts_micros;
            let last = bars[bars.len() - 1].ts_micros;
            let ts = first + 1 + rng.below((last - first) as u64) as i64;
            assert_eq!(month.first_at_or_after(ts).unwrap(), scan(bars, ts));
            let (reads, read_bytes) = month.source.take();
            let table = month.head().flags & FLAG_TIME_TABLE != 0;
            assert!(reads <= 2 + u64::from(table), "{reads} reads");
            assert!(
                read_bytes <= 2 * TABLE_ENTRY_LEN + 65 * ENTRY_LEN + 14_508,
                "{read_bytes}"
            );
        }
    }
    for (label, n) in [("small", 554), ("largest", 2_678_400)] {
        let mut mine: Vec<u128> = timings
            .iter()
            .filter(|(rows, _)| *rows == n)
            .map(|(_, took)| took.as_nanos())
            .collect();
        mine.sort_unstable();
        println!(
            "100-row page, {label} month ({n} bars), in memory, n = {}: p50 {} ns, p99 {} ns, max {} ns",
            mine.len(),
            mine[mine.len() / 2],
            mine[mine.len() * 99 / 100],
            mine[mine.len() - 1]
        );
    }
}

#[test]
fn a_flipped_bit_anywhere_is_refused_by_name() {
    // Every refusal below is one `store.sealed` line: held off the sink
    // `crate::emits` reads, so its count is its own.
    let _sink = crate::emits::hold_the_sink();
    let path = nifty(Timeframe::SECOND_1, june());
    let bars = walk(300, origin(june()) + OPEN, SECOND, 11, true);
    let image = prv_image(path, 3);
    let summary = Summary {
        version: 1,
        bytes: b"an invented month summary",
    };
    let built = build(
        path,
        &header(Timeframe::SECOND_1, &bars),
        &bars,
        Some(&image),
        Some(summary),
    )
    .unwrap();
    let good = built.to_bytes();
    assert_eq!(built.head().flags, FLAG_PROVENANCE | FLAG_SUMMARY);
    let mut seen = std::collections::BTreeSet::new();
    for at in 0..good.len() {
        for bit in 0..8 {
            let mut bytes = good.clone();
            bytes[at] ^= 1 << bit;
            let answer = SealedMonth::open(bytes, path, SYMBOL).and_then(|month| {
                month.audit()?;
                Ok(())
            });
            let refusal = match answer {
                Ok(()) => panic!("byte {at} bit {bit} flipped and nothing refused"),
                Err(refusal) => refusal,
            };
            seen.insert(refusal.reason());
        }
    }
    let seen: Vec<_> = seen.into_iter().collect();
    for name in [
        "magic",
        "version",
        "head_checksum",
        "entry_checksum",
        "frame",
        "provenance",
        "summary",
    ] {
        assert!(seen.contains(&name), "{name} not among {seen:?}");
    }

    // A month with a time table: every byte of its directory and its table.
    let bars = seconds(16_641, 12);
    let built = sealed(path, &bars);
    let good = built.to_bytes();
    let head = *built.head();
    for at in 0..usize::try_from(head.prv_off).unwrap() {
        let mut bytes = good.clone();
        bytes[at] ^= 0x10;
        let answer = SealedMonth::open(bytes, path, SYMBOL).and_then(|month| month.audit());
        assert!(answer.is_err(), "byte {at} flipped and nothing refused");
    }
}

/// Every checksum in the file, computed here from the bytes as `docs/02`
/// §8.5 states it, not through the module's own helpers: a helper that wrote
/// and checked a wrong number on both sides would agree with itself.
#[test]
fn the_checksums_are_the_documented_ones() {
    use crate::crc::crc32c_split;
    let path = nifty(Timeframe::SECOND_1, june());
    let bars = walk(17_000, origin(june()) + OPEN, SECOND, 37, true);
    let image = prv_image(path, 3);
    let built = build(
        path,
        &header(Timeframe::SECOND_1, &bars),
        &bars,
        Some(&image),
        Some(Summary {
            version: 9,
            bytes: b"checked",
        }),
    )
    .unwrap();
    let bytes = built.to_bytes();
    let head = *built.head();
    let u32_at = |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
    assert_eq!(u32_at(508), crc32c(&bytes[..508]), "head");
    assert_eq!(
        u32_at(56),
        crc32c_split(&bytes[..56], &bytes[60..64]),
        "slot"
    );
    assert_eq!(&bytes[60..64], &[0; 4]);
    let mut start = 0;
    for k in 0..u64::from(head.n_blocks) {
        let at = 512 + 24 * k as usize;
        assert_eq!(
            u32_at(at + 20),
            crc32c_split(&bytes[at..at + 20], &k.to_le_bytes()),
            "entry {k}"
        );
        let end = u32_at(at) as usize;
        let frame = &bytes[head.blocks_off as usize + start..head.blocks_off as usize + end];
        assert_eq!(u32_at(at + 4), crc32c(frame), "frame {k}");
        let first = i64::from_le_bytes(bytes[at + 8..at + 16].try_into().unwrap());
        assert_eq!(first, bars[k as usize * 256].ts_micros);
        start = end;
    }
    assert_eq!(start as u64, head.blocks_len);
    for index in 0..=u64::from(head.table_entries) {
        let at = head.table_off as usize + 8 * index as usize;
        assert_eq!(
            u32_at(at + 4),
            crc32c_split(&bytes[at..at + 4], &index.to_le_bytes()),
            "table {index}"
        );
    }
    let prv = head.prv_off as usize;
    let packed = head.prv_packed_len as usize;
    assert_eq!(&bytes[prv..prv + packed], &pack_provenance(&image)[..]);
    assert_eq!(
        u32_at(prv + packed),
        crc32c(&bytes[prv..prv + packed]),
        "provenance"
    );
    let sum = head.sum_off as usize;
    let len = head.sum_len as usize;
    assert_eq!(&bytes[sum..sum + 4], &[9, 0, 0, 0]);
    assert_eq!(&bytes[sum + 4..sum + len - 4], b"checked");
    assert_eq!(
        u32_at(sum + len - 4),
        crc32c(&bytes[sum..sum + len - 4]),
        "summary"
    );
    assert_eq!(head.blocks_off as usize, sum + len);
}

#[test]
fn a_length_that_is_not_the_heads_is_refused() {
    // Every refusal below is one `store.sealed` line: held off the sink
    // `crate::emits` reads, so its count is its own.
    let _sink = crate::emits::hold_the_sink();
    let path = nifty(Timeframe::SECOND_1, june());
    let bars = seconds(600, 4);
    let good = sealed(path, &bars).to_bytes();
    let n = good.len();
    let mut longer = good.clone();
    longer.push(0);
    let mut doubled = good.clone();
    doubled.extend_from_slice(&good);
    let cases: [(Vec<u8>, &str); 6] = [
        (Vec::new(), "too_short"),
        (good[..511].to_vec(), "too_short"),
        (good[..512].to_vec(), "length"),
        (good[..n - 1].to_vec(), "length"),
        (longer, "length"),
        (doubled, "length"),
    ];
    for (bytes, reason) in cases {
        let len = bytes.len();
        let got = refusal(SealedMonth::open(bytes, path, SYMBOL));
        assert_eq!(got.reason(), reason, "{len} bytes: {got}");
    }
}

#[test]
fn an_entry_at_the_wrong_index_fails() {
    // Every refusal below is one `store.sealed` line: held off the sink
    // `crate::emits` reads, so its count is its own.
    let _sink = crate::emits::hold_the_sink();
    let path = nifty(Timeframe::SECOND_1, june());
    let bars = seconds(1_000, 5);
    let good = sealed(path, &bars).to_bytes();
    let entry = |k: usize| 512 + 24 * k..512 + 24 * (k + 1);
    // Entry 1 copied over entry 0: whole and well formed, at the wrong place.
    let mut bytes = good.clone();
    let one = bytes[entry(1)].to_vec();
    bytes[entry(0)].copy_from_slice(&one);
    let month = open(path, bytes);
    let mut out = Vec::new();
    match refusal(month.read_block(0, &mut out)) {
        SealedError::Entry { index } => assert_eq!(index, 0),
        other => panic!("{other}"),
    }
    assert!(out.is_empty());
    assert_eq!(refusal(month.audit()).reason(), "entry_checksum");
    // Two entries swapped.
    let mut bytes = good;
    let zero = bytes[entry(0)].to_vec();
    let two = bytes[entry(2)].to_vec();
    bytes[entry(0)].copy_from_slice(&two);
    bytes[entry(2)].copy_from_slice(&zero);
    let month = open(path, bytes);
    assert_eq!(
        refusal(month.read_rows(250, 300, &mut out)).reason(),
        "entry_checksum"
    );
    assert_eq!(
        refusal(month.first_at_or_after(bars[600].ts_micros)).reason(),
        "entry_checksum"
    );
    assert!(out.is_empty());
}

#[test]
fn off_grid_and_equal_stamps_round_trip_and_are_found() {
    let path = nifty(Timeframe::SECOND_1, june());
    let mut rng = Rng(21);
    let mut bars = seconds(5_000, 6);
    let mut ts = origin(june()) + OPEN + 123;
    for bar in &mut bars {
        // Runs of equal stamps, stamps off the second, and gaps.
        if rng.below(3) != 0 {
            ts += rng.below(3 * SECOND as u64) as i64;
        }
        bar.ts_micros = ts;
    }
    let month = open(path, sealed(path, &bars).to_bytes());
    check_reads(&month, &bars);
    for _ in 0..2_000 {
        let at = rng.below(bars.len() as u64) as usize;
        let ts = bars[at].ts_micros;
        let found = month.first_at_or_after(ts).unwrap();
        assert_eq!(found, scan(&bars, ts));
        assert!(
            found == 0 || bars[found as usize - 1].ts_micros < ts,
            "the first of equals"
        );
    }

    // Two daily bars in one IST day, at the one-day rung.
    let path = nifty(Timeframe::DAY_1, june());
    let mut daily = walk(20, origin(june()) + OPEN, DAY, 7, false);
    daily[5].ts_micros = daily[4].ts_micros;
    daily[6].ts_micros = daily[4].ts_micros + 3 * 3_600 * SECOND;
    let month = open(path, sealed(path, &daily).to_bytes());
    check_reads(&month, &daily);
    assert_eq!(month.first_at_or_after(daily[4].ts_micros).unwrap(), 4);
}

/// Runs `body` on a worker and fails if it has not returned in a minute,
/// where it takes under a second: a bisection that never ends under a
/// mutant (`last_before`'s `a < b` or its midpoint) fails the test instead
/// of holding up the binary. A panic in `body` fails it too, its message
/// printed by the worker.
fn within_a_minute(body: fn()) {
    let (done, finished) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        body();
        let _receiver_gone = done.send(());
    });
    finished
        .recv_timeout(std::time::Duration::from_mins(1))
        .expect("returned within a minute");
}

#[test]
fn a_widened_superblock_bisects_in_a_bounded_number_of_reads() {
    within_a_minute(widened_superblock_body);
}

/// The test above, on a worker with a deadline.
fn widened_superblock_body() {
    // Twenty thousand bars on one second (a month written before D-0915), so
    // one superblock spans every block and the lookup bisects.
    let path = nifty(Timeframe::SECOND_1, june());
    let mut bars = seconds(40_000, 8);
    let pile = bars[0].ts_micros;
    for bar in &mut bars[..20_000] {
        bar.ts_micros = pile;
    }
    for (i, bar) in bars[20_000..].iter_mut().enumerate() {
        bar.ts_micros = pile + SECOND + i as i64 * SECOND / 1_000;
    }
    let head = *sealed(path, &bars).head();
    assert!(u64::from(head.n_blocks) > BISECT_IN_ONE_READ);
    let month =
        SealedMonth::open(Counting::new(sealed(path, &bars).to_bytes()), path, SYMBOL).unwrap();
    month.source.take();
    let bound = u64::from(head.n_blocks + 1).ilog2() + 2;
    for at in (0..bars.len()).step_by(37) {
        for ts in [bars[at].ts_micros, bars[at].ts_micros + 1] {
            assert_eq!(month.first_at_or_after(ts).unwrap(), scan(&bars, ts));
            let (reads, _) = month.source.take();
            assert!(
                reads <= u64::from(bound) + 3,
                "{reads} reads, bound {bound}"
            );
        }
    }
    let plain = open(path, sealed(path, &bars).to_bytes());
    check_reads(&plain, &bars);
}

#[test]
fn a_lookup_at_the_edge_of_a_superblock_and_a_block_is_exact() {
    within_a_minute(superblock_edge_body);
}

/// The test above, on a worker with a deadline.
fn superblock_edge_body() {
    let path = nifty(Timeframe::SECOND_1, june());
    let superblock = |s: i64| origin(june()) + s * 4_096 * SECOND;
    let probe = |bars: &[Bar], ts: i64| {
        let month = open(path, sealed(path, bars).to_bytes());
        for ts in [ts - 1, ts, ts + 1] {
            assert_eq!(month.first_at_or_after(ts).unwrap(), scan(bars, ts), "{ts}");
        }
        month
    };
    // Row 16,896, the first of block 66, is stamped exactly at superblock
    // 10: the table names block 66 and the block before holds no later row.
    let bars = walk(20_000, superblock(10) - 16_896 * SECOND, SECOND, 34, false);
    let month = probe(&bars, superblock(10));
    assert!(month.head().flags & FLAG_TIME_TABLE != 0);
    assert_eq!(month.first_at_or_after(superblock(10)).unwrap(), 16_896);
    // One block, then forty thousand rows on superblock 9's first instant
    // and after it: a widened range whose first block is not block 0.
    let mut bars = walk(40_256, superblock(8) + 100 * SECOND, SECOND, 35, false);
    for (i, bar) in bars[256..].iter_mut().enumerate() {
        bar.ts_micros = superblock(9) + (i as i64) / 2;
    }
    let month = probe(&bars, superblock(9));
    assert_eq!(month.first_at_or_after(superblock(9)).unwrap(), 256);
    // Twenty thousand rows a microsecond apart inside superblock 8: a widened
    // range that starts at block 0, probed inside block 0.
    let bars = walk(20_000, superblock(8), 1, 36, false);
    probe(&bars, superblock(8) + 100);
    check_reads(&open(path, sealed(path, &bars).to_bytes()), &bars);
}

#[test]
fn a_misfiled_month_is_refused() {
    // Every refusal below is one `store.sealed` line: held off the sink
    // `crate::emits` reads, so its count is its own.
    let _sink = crate::emits::hold_the_sink();
    let path = nifty(Timeframe::SECOND_1, june());
    let bars = seconds(300, 9);
    let bytes = sealed(path, &bars).to_bytes();
    let cases = [
        (path, SYMBOL + 1, "symbol_id"),
        (nifty(Timeframe::MINUTE_1, june()), SYMBOL, "timeframe"),
        (
            nifty(Timeframe::SECOND_1, YearMonth::new(2024, 7).unwrap()),
            SYMBOL,
            "month",
        ),
        (
            nifty(Timeframe::SECOND_1, YearMonth::new(2023, 6).unwrap()),
            SYMBOL,
            "month",
        ),
        (
            path_of(Vendor::Gdfl, "BANKNIFTY", Timeframe::SECOND_1, june()),
            SYMBOL,
            "path",
        ),
        (
            path_of(Vendor::Zerodha, "NIFTY", Timeframe::SECOND_1, june()),
            SYMBOL,
            "path",
        ),
    ];
    for (other, symbol, field) in cases {
        match refusal(SealedMonth::open(bytes.clone(), other, symbol)) {
            SealedError::Misfiled { field: got } => assert_eq!(got, field),
            other => panic!("{field}: {other}"),
        }
    }
    // The sealed sibling's own path names the same month.
    assert!(SealedMonth::open(bytes, path.with_file(FileKind::SealedBars), SYMBOL).is_ok());
    // An option contract's month is bound to its contract.
    let contract = Contract::parse("2020-03-26-900000-CE").unwrap();
    let option = StorePath::new(PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment: "FNO",
        symbol: "NIFTY",
        contract: Some(contract),
        timeframe: Timeframe::SECOND_1,
        month: YearMonth::new(2020, 3).unwrap(),
        file: FileKind::Bars,
    })
    .unwrap();
    let other = Contract::parse("2020-03-26-900000-PE").unwrap();
    let put = StorePath::new(PathParts {
        contract: Some(other),
        ..PathParts {
            vendor: Vendor::Gdfl,
            exchange: "NSE",
            segment: "FNO",
            symbol: "NIFTY",
            contract: None,
            timeframe: Timeframe::SECOND_1,
            month: YearMonth::new(2020, 3).unwrap(),
            file: FileKind::Bars,
        }
    })
    .unwrap();
    let start = origin(YearMonth::new(2020, 3).unwrap()) + OPEN;
    let bars = walk(200, start, SECOND, 10, true);
    let bytes = sealed(option, &bars).to_bytes();
    let month = SealedMonth::open(bytes.clone(), option, SYMBOL).unwrap();
    check_reads(&month, &bars);
    assert_eq!(
        refusal(SealedMonth::open(bytes, put, SYMBOL)).reason(),
        "misfiled"
    );
}

#[test]
fn the_same_month_built_twice_is_the_same_bytes() {
    let path = nifty(Timeframe::SECOND_1, june());
    let bars = walk(17_000, origin(june()) + OPEN, SECOND, 13, true);
    let image = prv_image(path, 5);
    let raw = header(Timeframe::SECOND_1, &bars);
    let summary = Some(Summary {
        version: 3,
        bytes: &[1, 2, 3],
    });
    let once = build(path, &raw, &bars, Some(&image), summary)
        .unwrap()
        .to_bytes();
    let mut builder = Builder::new(path, &raw).unwrap();
    for bar in &bars {
        builder.push(bar).unwrap();
    }
    let mut streamed = Vec::new();
    builder
        .finish(Some(&image), summary)
        .unwrap()
        .write_to(&mut streamed)
        .unwrap();
    assert_eq!(streamed, once);
    let copies: Vec<Vec<u8>> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..4)
            .map(|_| {
                scope.spawn(|| {
                    build(path, &raw, &bars, Some(&image), summary)
                        .unwrap()
                        .to_bytes()
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });
    for copy in copies {
        assert_eq!(copy, once);
    }
}

/// A device with room for `room` bytes, then `ENOSPC`, as a full disk is.
struct Full {
    taken: Vec<u8>,
    room: usize,
}

impl io::Write for Full {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let room = self.room - self.taken.len();
        if room == 0 {
            return Err(io::Error::from_raw_os_error(28));
        }
        let wrote = room.min(buf.len());
        self.taken.extend_from_slice(&buf[..wrote]);
        Ok(wrote)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// A full disk at any byte of the file: `write_to` hands back the device's
/// own `ENOSPC`, what reached the device is exactly the file's prefix, and
/// that prefix (a torn file, as a sealer killed or starved mid-write leaves
/// it) is refused when opened, never read as a month. Every section present:
/// head, directory, table, provenance, summary, frames.
#[test]
fn a_full_disk_at_any_byte_is_the_writers_refusal_and_the_prefix_is_never_a_month() {
    let _sink = crate::emits::hold_the_sink();
    let path = nifty(Timeframe::SECOND_1, june());
    let bars = walk(17_000, origin(june()) + OPEN, SECOND, 23, true);
    let image = prv_image(path, 4);
    let built = build(
        path,
        &header(Timeframe::SECOND_1, &bars),
        &bars,
        Some(&image),
        Some(Summary {
            version: 2,
            bytes: b"summary",
        }),
    )
    .unwrap();
    let bytes = built.to_bytes();
    let head = *built.head();
    assert_eq!(
        head.flags, KNOWN_FLAGS,
        "the premise: every section present"
    );
    let len = bytes.len();
    let mut rng = Rng(28);
    let mut at: Vec<usize> = [
        0,
        1,
        511,
        512,
        513,
        head.table_off as usize,
        head.prv_off as usize,
        head.sum_off as usize,
        head.blocks_off as usize,
        head.blocks_off as usize + 1,
        len - 1,
    ]
    .into_iter()
    .chain((0..64).map(|_| rng.below(len as u64) as usize))
    .collect();
    at.sort_unstable();
    at.dedup();
    for room in at {
        let mut device = Full {
            taken: Vec::new(),
            room,
        };
        let refused = built
            .write_to(&mut device)
            .expect_err("a full disk is the writer's refusal");
        assert_eq!(refused.raw_os_error(), Some(28), "at {room}");
        assert_eq!(device.taken, bytes[..room], "at {room}: the file's prefix");
        let torn = refusal(SealedMonth::open(device.taken, path, SYMBOL));
        let expected = if room < HEAD_LEN as usize {
            "too_short"
        } else {
            "length"
        };
        assert_eq!(torn.reason(), expected, "a torn file of {room} bytes");
    }
    let mut device = Full {
        taken: Vec::new(),
        room: len,
    };
    built.write_to(&mut device).unwrap();
    assert_eq!(device.taken, bytes, "room for the whole file is the file");
    let month = open(path, device.taken);
    assert_eq!(month.audit().unwrap().rows, 17_000);
}

#[test]
fn many_threads_read_one_month_at_once() {
    let path = nifty(Timeframe::SECOND_1, june());
    let bars = seconds(30_000, 14);
    let month = open(path, sealed(path, &bars).to_bytes());
    std::thread::scope(|scope| {
        for seed in 0..8 {
            let (month, bars) = (&month, &bars);
            scope.spawn(move || {
                let mut rng = Rng(seed);
                for _ in 0..500 {
                    let first = rng.below(bars.len() as u64 - 200);
                    let mut page = Vec::new();
                    month.read_rows(first, 200, &mut page).unwrap();
                    assert_eq!(page, bars[first as usize..first as usize + 200]);
                    let ts = bars[first as usize].ts_micros;
                    assert_eq!(month.first_at_or_after(ts).unwrap(), first);
                }
            });
        }
    });
}

#[test]
fn a_build_that_predates_this_format_refuses_a_sealed_file_by_magic() {
    // Every refusal below is one `store.sealed` line: held off the sink
    // `crate::emits` reads, so its count is its own.
    let _sink = crate::emits::hold_the_sink();
    let path = nifty(Timeframe::SECOND_1, june());
    let bytes = sealed(path, &seconds(10, 15)).to_bytes();
    assert_eq!(&bytes[..8], &MAGIC);
    assert_eq!(Header::decode(&bytes[..64]), Err(FormatError::NotABarFile));
    // And the sealed reader refuses a bar file's header by its magic.
    let raw = header(Timeframe::SECOND_1, &seconds(10, 15));
    let slot = raw.commit().unwrap().bytes;
    let mut bar_file = vec![0u8; 512];
    bar_file[..64].copy_from_slice(&slot);
    assert_eq!(
        refusal(SealedMonth::open(bar_file, path, SYMBOL)).reason(),
        "magic"
    );
}

#[test]
fn the_head_is_the_raw_months_header_and_its_records_digest() {
    let path = nifty(Timeframe::MINUTE_1, june());
    let bars = walk(4_000, origin(june()) + OPEN, MINUTE, 16, true);
    let raw = header(Timeframe::MINUTE_1, &bars);
    let month = open(path, sealed(path, &bars).to_bytes());
    let head = *month.head();
    // `generation::Pointer::describes` reads exactly these four fields.
    assert_eq!(head.bar_header(), raw);
    assert_eq!(head.bar_header().generation, GENERATION);
    let mut records = Vec::new();
    for bar in &bars {
        records.extend_from_slice(&bar.image());
    }
    assert_eq!(records.len(), bars.len() * RECORD_LEN);
    assert_eq!(head.records_blake3, brutex_core::blake3::hash(&records));
    // The raw month's `.crc` image, regenerated block by block.
    let layout = Layout::CURRENT;
    let n = bars.len() as u64;
    let mut image = Vec::new();
    for b in 0..layout.blocks_for(n) {
        let from = (b * 73 * 56) as usize;
        let until = (((b + 1) * 73).min(n) * 56) as usize;
        let crc = block::seal(layout, n, b, &records[from..until]).unwrap();
        image.extend_from_slice(&crc.to_le_bytes());
    }
    assert_eq!(head.crc_image_crc, crc32c(&image));
    assert_eq!(
        head.path_digest,
        provenance::Header::of(path, SYMBOL).path_digest
    );
    assert_eq!(head.origin_micros, origin(june()));
    assert_eq!((head.year, head.month), (2024, 6));
}

#[test]
fn the_head_ranges_and_extremes_are_the_sort_runs() {
    let path = nifty(Timeframe::SECOND_1, june());
    let mut bars = walk(3_000, origin(june()) + OPEN, SECOND, 17, true);
    for bar in bars.iter_mut().step_by(5) {
        bar.open_interest = OI_NULL;
    }
    // Ties: the earliest bar keeps an extreme.
    bars[2_000].close = bars[1_000].close;
    let head = *sealed(path, &bars).head();
    let shape = Shape::of(june(), 1, SYMBOL);
    let run = sort_index::build_run(&shape, 0, &bars).unwrap();
    let info = sort_index::decode_run_header(&run[..512].try_into().unwrap(), 0, 0).unwrap();
    assert_eq!(head.extremes, info.extremes);
    // The run's columns are open, high, low, close, volume, open interest.
    for (mine, theirs) in head.columns.iter().zip(info.columns) {
        assert_eq!(
            (mine.nulls, mine.smallest, mine.largest),
            (theirs.nulls, theirs.base, theirs.max)
        );
    }
    assert_eq!(head.columns[5].nulls, 600);

    // Every open interest null: a range of zeros and every row counted.
    let mut nulls = seconds(100, 18);
    for bar in &mut nulls {
        bar.open_interest = OI_NULL;
    }
    let head = *sealed(path, &nulls).head();
    assert_eq!(
        head.columns[5],
        ColumnRange {
            nulls: 100,
            smallest: 0,
            largest: 0
        }
    );
}

#[test]
fn provenance_packs_and_unpacks_byte_for_byte() {
    let cases: [(&[u8], &[u8]); 7] = [
        (&[], &[]),
        (&[0], &[0, 0]),
        (&[0, 0, 0, 5], &[0, 2, 5]),
        (&[5, 0, 7], &[5, 0, 0, 7]),
        (&[0; 256], &[0, 255]),
        (&[0; 257], &[0, 255, 0, 0]),
        (&[9; 3], &[9, 9, 9]),
    ];
    for (raw, packed) in cases {
        assert_eq!(pack_provenance(raw), packed, "{raw:?}");
        assert_eq!(unpack_provenance(packed, raw.len()).unwrap(), raw);
    }
    // A byte 256 after an equal one is a zero.
    let mut raw = vec![3u8; 300];
    raw[260] = 4;
    let packed = pack_provenance(&raw);
    assert_eq!(&packed[..256], &[3u8; 256]);
    assert_eq!(&packed[256..], &[0, 3, 7, 0, 38]);
    assert_eq!(unpack_provenance(&packed, 300).unwrap(), raw);
    let path = nifty(Timeframe::SECOND_1, june());
    let mut rng = Rng(19);
    for records in [0, 1, 2, 9, 64, 1_024] {
        let image = prv_image(path, records);
        let packed = pack_provenance(&image);
        assert!(packed.len() <= 2 * image.len());
        assert_eq!(unpack_provenance(&packed, image.len()).unwrap(), image);
        if records >= 9 {
            assert!(
                packed.len() * 2 < image.len(),
                "{records}: {} of {}",
                packed.len(),
                image.len()
            );
        }
    }
    for _ in 0..200 {
        let len = rng.below(2_000) as usize;
        let raw: Vec<u8> = (0..len)
            .map(|_| {
                if rng.below(3) == 0 {
                    0
                } else {
                    rng.below(256) as u8
                }
            })
            .collect();
        let packed = pack_provenance(&raw);
        assert!(packed.len() <= 2 * raw.len());
        assert_eq!(unpack_provenance(&packed, raw.len()).unwrap(), raw);
    }
    let refused = |packed: &[u8], raw_len: usize| match unpack_provenance(packed, raw_len) {
        Err(SealedError::Provenance { why }) => why,
        other => panic!("{other:?}"),
    };
    assert_eq!(refused(&[0], 1), "truncated");
    assert_eq!(refused(&[0, 4], 4), "overrun");
    assert_eq!(refused(&[7, 7], 1), "overrun");
    assert_eq!(refused(&[0, 2], 4), "short");
}

#[test]
fn a_provenance_is_carried_byte_for_byte_and_a_foreign_one_is_refused() {
    // Every refusal below is one `store.sealed` line: held off the sink
    // `crate::emits` reads, so its count is its own.
    let _sink = crate::emits::hold_the_sink();
    let path = nifty(Timeframe::SECOND_1, june());
    let bars = seconds(400, 20);
    let raw = header(Timeframe::SECOND_1, &bars);
    for image in [
        prv_image(path, 2),
        prv_image(path, 1_024),
        vec![1, 2, 3],
        Vec::new(),
    ] {
        let built = build(path, &raw, &bars, Some(&image), None).unwrap();
        assert_eq!(built.head().prv_raw_len as usize, image.len());
        let month = open(path, built.to_bytes());
        assert_eq!(month.provenance().unwrap(), Some(image));
        month.audit().unwrap();
    }
    let foreign = [
        prv_image(
            path_of(Vendor::Gdfl, "BANKNIFTY", Timeframe::SECOND_1, june()),
            1,
        ),
        prv_image(nifty(Timeframe::MINUTE_1, june()), 1),
        {
            let mut image = prv_image(path, 1);
            image[16] ^= 1;
            image
        },
    ];
    for image in foreign {
        let got = refusal(build(path, &raw, &bars, Some(&image), None));
        assert_eq!(got.reason(), "provenance_foreign");
    }
    let mut long = prv_image(path, 1_024);
    long.push(0);
    assert_eq!(long.len() as u64, MAX_PROVENANCE_LEN + 1);
    match refusal(build(path, &raw, &bars, Some(&long), None)) {
        SealedError::ProvenanceTooLong { len } => assert_eq!(len, MAX_PROVENANCE_LEN + 1),
        other => panic!("{other}"),
    }
}

#[test]
fn the_month_summary_is_carried_with_its_version() {
    // Every refusal below is one `store.sealed` line: held off the sink
    // `crate::emits` reads, so its count is its own.
    let _sink = crate::emits::hold_the_sink();
    let path = nifty(Timeframe::SECOND_1, june());
    let bars = seconds(700, 22);
    let raw = header(Timeframe::SECOND_1, &bars);
    let biggest = vec![0xA5u8; MAX_SUMMARY_LEN];
    for (version, bytes) in [
        (1u16, &b""[..]),
        (2, &b"invented summary"[..]),
        (u16::MAX, &biggest[..]),
    ] {
        let built = build(path, &raw, &bars, None, Some(Summary { version, bytes })).unwrap();
        assert_eq!(built.head().sum_len as usize, bytes.len() + 8);
        let month = open(path, built.to_bytes());
        assert_eq!(month.summary().unwrap(), Some((version, bytes.to_vec())));
        month.audit().unwrap();
    }
    let too_long = vec![0u8; MAX_SUMMARY_LEN + 1];
    let got = refusal(build(
        path,
        &raw,
        &bars,
        None,
        Some(Summary {
            version: 1,
            bytes: &too_long,
        }),
    ));
    assert_eq!(got.reason(), "summary_too_long");
    let got = refusal(build(
        path,
        &raw,
        &bars,
        None,
        Some(Summary {
            version: 0,
            bytes: &[1],
        }),
    ));
    assert_eq!(got.reason(), "summary_version");

    // A section whose checksum holds but whose shape does not.
    let good = build(
        path,
        &raw,
        &bars,
        None,
        Some(Summary {
            version: 4,
            bytes: &[9; 10],
        }),
    )
    .unwrap()
    .to_bytes();
    let at = open(path, good.clone()).head().sum_off as usize;
    for (offset, value, why) in [
        (0, 0u8, "version"),
        (1, 0, "version"),
        (2, 1, "reserved"),
        (3, 1, "reserved"),
    ] {
        let mut bytes = good.clone();
        if offset < 2 {
            bytes[at..at + 2].copy_from_slice(&[0, 0]);
        }
        bytes[at + offset] = value;
        let body = bytes[at..at + 14].to_vec();
        bytes[at + 14..at + 18].copy_from_slice(&crc32c(&body).to_le_bytes());
        let month = open(path, bytes);
        match refusal(month.summary()) {
            SealedError::Summary { why: got } => assert_eq!(got, why),
            other => panic!("{other}"),
        }
    }
    let mut bytes = good;
    bytes[at + 5] ^= 1;
    assert_eq!(
        refusal(open(path, bytes).summary()).to_string(),
        "summary: the month summary section: checksum"
    );
}

/// One head field changed, both checksums recomputed, and the reason the
/// open refuses it with.
#[test]
fn every_head_field_is_checked() {
    // Every refusal below is one `store.sealed` line: held off the sink
    // `crate::emits` reads, so its count is its own.
    let _sink = crate::emits::hold_the_sink();
    let path = nifty(Timeframe::SECOND_1, june());
    let bars = walk(17_000, origin(june()) + OPEN, SECOND, 23, true);
    let image = prv_image(path, 2);
    let built = build(
        path,
        &header(Timeframe::SECOND_1, &bars),
        &bars,
        Some(&image),
        Some(Summary {
            version: 1,
            bytes: b"summary",
        }),
    )
    .unwrap();
    let good = built.to_bytes();
    let head = *built.head();
    assert_eq!(head.flags, KNOWN_FLAGS);
    let poke = |at: usize, value: &[u8], reseal_it: bool| {
        let mut bytes = good.clone();
        bytes[at..at + value.len()].copy_from_slice(value);
        if reseal_it {
            reseal(&mut bytes);
        }
        bytes
    };
    let mut cases: Vec<(Vec<u8>, &str, &str)> = vec![
        (poke(0, b"BRUTEXB3", true), "magic", ""),
        (poke(8, &2u16.to_le_bytes(), true), "version", ""),
        (poke(300, &[1], false), "head_checksum", ""),
        (poke(56, &[0, 0, 0, 0], false), "head_checksum", ""),
        (
            {
                let mut bytes = poke(56, &[1, 2, 3, 4], false);
                let whole = head_crc(&bytes);
                bytes[508..512].copy_from_slice(&whole.to_le_bytes());
                bytes
            },
            "slot_checksum",
            "",
        ),
        (poke(10, &57u16.to_le_bytes(), true), "stride", ""),
        (poke(12, &8u32.to_le_bytes(), true), "flags", ""),
        (poke(64, &0u16.to_le_bytes(), true), "block_rows", ""),
        (poke(64, &257u16.to_le_bytes(), true), "block_rows", ""),
        (poke(66, &2u16.to_le_bytes(), true), "codec", ""),
        (poke(68, &20u16.to_le_bytes(), true), "entry_len", ""),
        (poke(70, &2u16.to_le_bytes(), true), "raw_version", ""),
        (poke(76, &4_095u32.to_le_bytes(), true), "table_slots", ""),
        (
            poke(52, &0u32.to_le_bytes(), true),
            "geometry",
            "timeframe_secs",
        ),
        (poke(94, &[13], true), "geometry", "month"),
        (
            poke(136, &(head.origin_micros + 1).to_le_bytes(), true),
            "geometry",
            "origin_micros",
        ),
        (
            poke(72, &(head.n_blocks + 1).to_le_bytes(), true),
            "geometry",
            "n_blocks",
        ),
        (
            poke(12, &(KNOWN_FLAGS & !FLAG_TIME_TABLE).to_le_bytes(), true),
            "geometry",
            "time_table_flag",
        ),
        (
            poke(80, &(head.table_entries + 1).to_le_bytes(), true),
            "geometry",
            "table_entries",
        ),
        (
            poke(96, &(head.table_off + 1).to_le_bytes(), true),
            "geometry",
            "table_off",
        ),
        (
            poke(104, &(head.prv_off + 1).to_le_bytes(), true),
            "geometry",
            "prv_off",
        ),
        (
            poke(12, &(KNOWN_FLAGS & !FLAG_PROVENANCE).to_le_bytes(), true),
            "geometry",
            "prv_packed_len",
        ),
        (
            poke(88, &((MAX_PROVENANCE_LEN + 1) as u32).to_le_bytes(), true),
            "geometry",
            "prv_raw_len",
        ),
        (
            poke(84, &(2 * head.prv_raw_len + 1).to_le_bytes(), true),
            "geometry",
            "prv_packed_len",
        ),
        (
            poke(144, &(head.sum_off + 1).to_le_bytes(), true),
            "geometry",
            "sum_off",
        ),
        (poke(152, &7u32.to_le_bytes(), true), "geometry", "sum_len"),
        (
            poke(152, &((MAX_SUMMARY_LEN + 9) as u32).to_le_bytes(), true),
            "geometry",
            "sum_len",
        ),
        (
            poke(12, &(KNOWN_FLAGS & !FLAG_SUMMARY).to_le_bytes(), true),
            "geometry",
            "sum_off",
        ),
        (
            poke(112, &(head.blocks_off + 1).to_le_bytes(), true),
            "geometry",
            "blocks_off",
        ),
        (
            poke(120, &0u64.to_le_bytes(), true),
            "geometry",
            "blocks_len",
        ),
        (
            poke(
                120,
                &(u64::from(head.n_blocks) * 14_508 + 1).to_le_bytes(),
                true,
            ),
            "geometry",
            "blocks_len",
        ),
        (
            poke(128, &(head.file_len + 1).to_le_bytes(), true),
            "geometry",
            "file_len",
        ),
        (
            poke(32, &(head.last_ts_micros + 1).to_le_bytes(), true),
            "stamps",
            "",
        ),
        (
            poke(32, &(head.origin_micros - 1).to_le_bytes(), true),
            "stamps",
            "",
        ),
        (poke(40, &june_end().to_le_bytes(), true), "stamps", ""),
        (poke(224, &1u64.to_le_bytes(), true), "columns", "open"),
        (
            poke(224 + 5 * 24, &(head.n_valid + 1).to_le_bytes(), true),
            "columns",
            "open_interest",
        ),
        (
            poke(224 + 8, &(head.columns[0].largest + 1).to_le_bytes(), true),
            "columns",
            "open",
        ),
        (
            poke(368, &(head.extremes.close_max.v + 1).to_le_bytes(), true),
            "extremes",
            "close_max",
        ),
        (
            poke(368 + 8, &(head.last_ts_micros + 1).to_le_bytes(), true),
            "extremes",
            "close_max",
        ),
        (
            poke(
                368 + 16 + 8,
                &(head.first_ts_micros - 1).to_le_bytes(),
                true,
            ),
            "extremes",
            "close_min",
        ),
        (
            poke(
                368 + 32,
                &(head.extremes.high_max.v - 1).to_le_bytes(),
                true,
            ),
            "extremes",
            "high_max",
        ),
        (
            poke(368 + 48, &(head.extremes.low_min.v + 1).to_le_bytes(), true),
            "extremes",
            "low_min",
        ),
        (
            poke(368 + 64 + 8, &(head.last_ts_micros + 1).to_le_bytes(), true),
            "extremes",
            "range_max",
        ),
        (
            poke(
                368 + 80,
                &(head.extremes.volume_max.v + 1).to_le_bytes(),
                true,
            ),
            "extremes",
            "volume_max",
        ),
    ];
    for at in [60, 63, 95, 156, 159, 196, 199, 464, 507] {
        cases.push((poke(at, &[1], true), "reserved", ""));
    }
    for (bytes, reason, field) in cases {
        let got = refusal(SealedMonth::open(bytes, path, SYMBOL));
        assert_eq!(got.reason(), reason, "{field}: {got}");
        let named = match &got {
            SealedError::Geometry { field, .. } => *field,
            SealedError::Columns { column } => column,
            SealedError::Extremes { extreme } => extreme,
            SealedError::Reserved { offset } => {
                assert!(matches!(*offset, 60..64 | 95 | 156..160 | 196..200 | 464..508));
                ""
            }
            _ => "",
        };
        assert_eq!(named, field, "{got}");
    }

    // An empty month's columns and extremes are all zero.
    let empty = sealed(path, &[]).to_bytes();
    for (at, reason) in [
        (224 + 8, "columns"),
        (368, "extremes"),
        (368 + 88, "extremes"),
    ] {
        let mut bytes = empty.clone();
        bytes[at] = 1;
        reseal(&mut bytes);
        assert_eq!(
            refusal(SealedMonth::open(bytes, path, SYMBOL)).reason(),
            reason
        );
    }
    // A column of all nulls holds no range.
    let mut nulls = seconds(10, 24);
    for bar in &mut nulls {
        bar.open_interest = OI_NULL;
    }
    let mut bytes = sealed(path, &nulls).to_bytes();
    bytes[224 + 5 * 24 + 16] = 1;
    reseal(&mut bytes);
    assert_eq!(
        refusal(SealedMonth::open(bytes, path, SYMBOL)).reason(),
        "columns"
    );
    // `Head::encode` writes what `Head::decode` reads.
    assert_eq!(Head::decode(&head.encode()).unwrap(), head);
    assert_eq!(&head.encode()[..], &good[..512]);
    assert_eq!(refusal(Head::decode(&good[..511])).reason(), "too_short");
    assert_eq!(u16::from_le_bytes([good[8], good[9]]), VERSION);
}

fn june_end() -> i64 {
    june().ist_bounds_micros().1
}

#[test]
fn a_whole_month_decode_disagreeing_with_the_head_is_refused() {
    // Every refusal below is one `store.sealed` line: held off the sink
    // `crate::emits` reads, so its count is its own.
    let _sink = crate::emits::hold_the_sink();
    let path = nifty(Timeframe::SECOND_1, june());
    // Row 0 and the last row hold no extreme, so their stamps may move.
    let mut bars = walk(17_000, origin(june()) + OPEN, SECOND, 25, true);
    let middle = bars[8_000].close;
    for at in [0, 16_999] {
        let bar = &mut bars[at];
        bar.close = middle;
        bar.open = middle;
        bar.high = middle;
        bar.low = middle;
        bar.volume = 0;
    }
    let built = sealed(path, &bars);
    let good = built.to_bytes();
    let head = *built.head();
    let audit = |bytes: Vec<u8>| refusal(open(path, bytes).audit());
    let poke = |at: usize, value: &[u8]| {
        let mut bytes = good.clone();
        bytes[at..at + value.len()].copy_from_slice(value);
        reseal(&mut bytes);
        bytes
    };
    let cases = [
        (poke(160, &[0; 32]), "records_blake3"),
        (
            poke(192, &(head.crc_image_crc ^ 1).to_le_bytes()),
            "crc_image_crc",
        ),
        (
            poke(224 + 8, &(head.columns[0].smallest - 1).to_le_bytes()),
            "columns",
        ),
        (
            poke(368 + 64, &(head.extremes.range_max.v + 1).to_le_bytes()),
            "extremes",
        ),
        (
            poke(32, &(head.first_ts_micros + 1).to_le_bytes()),
            "first_ts_micros",
        ),
        (
            poke(40, &(head.last_ts_micros - 1).to_le_bytes()),
            "last_ts_micros",
        ),
    ];
    for (bytes, field) in cases {
        match audit(bytes) {
            SealedError::Audit { field: got } => assert_eq!(got, field),
            other => panic!("{field}: {other}"),
        }
    }
    // A table entry that is well formed and wrong.
    let mut bytes = good.clone();
    let entry = |index: usize| {
        let at = head.table_off as usize + 8 * index;
        u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap())
    };
    let index = (0..head.table_entries as usize)
        .find(|&i| entry(i) < entry(i + 1))
        .unwrap();
    let wrong = entry(index) + 1;
    let at = head.table_off as usize + 8 * index;
    bytes[at..at + 8].copy_from_slice(&table_image(index as u64, wrong));
    match audit(bytes) {
        SealedError::Audit { field } => assert_eq!(field, "time_table"),
        other => panic!("{other}"),
    }
    // A provenance section that fails its checksum, found by the audit.
    let image = prv_image(path, 2);
    let built = build(
        path,
        &header(Timeframe::SECOND_1, &bars),
        &bars,
        Some(&image),
        None,
    )
    .unwrap();
    let mut bytes = built.to_bytes();
    bytes[built.head().prv_off as usize + 1] ^= 1;
    match audit(bytes) {
        SealedError::Provenance { why } => assert_eq!(why, "checksum"),
        other => panic!("{other}"),
    }
}

/// A directory entry rewritten with its checksum recomputed.
fn rewrite_entry(bytes: &mut [u8], index: u64, end: u32, crc: u32, first: i64) {
    let at = 512 + 24 * index as usize;
    bytes[at..at + 24].copy_from_slice(&entry_image(index, end, crc, first));
}

fn entry_of(bytes: &[u8], index: u64) -> (u32, u32, i64) {
    let at = 512 + 24 * index as usize;
    (
        u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()),
        u32::from_le_bytes(bytes[at + 4..at + 8].try_into().unwrap()),
        i64::from_le_bytes(bytes[at + 8..at + 16].try_into().unwrap()),
    )
}

#[test]
fn each_malformed_entry_frame_or_table_is_refused_by_name() {
    // Every refusal below is one `store.sealed` line: held off the sink
    // `crate::emits` reads, so its count is its own.
    let _sink = crate::emits::hold_the_sink();
    let path = nifty(Timeframe::SECOND_1, june());
    // The last ten rows hold no extreme, so the head may count fewer rows.
    let mut bars = walk(17_000, origin(june()) + OPEN, SECOND, 26, true);
    let middle = bars[8_000].close;
    for bar in &mut bars[16_990..] {
        (bar.open, bar.high, bar.low, bar.close, bar.volume) = (middle, middle, middle, middle, 0);
    }
    let good = sealed(path, &bars).to_bytes();
    let head = *open(path, good.clone()).head();
    let last = u64::from(head.n_blocks) - 1;
    let (end0, crc0, first0) = entry_of(&good, 0);
    let (end1, crc1, first1) = entry_of(&good, 1);
    let (end_last, crc_last, first_last) = entry_of(&good, last);
    let mut out = Vec::new();
    let read = |bytes: Vec<u8>, block: u64| {
        let month = open(path, bytes);
        let mut out = Vec::new();
        let got = refusal(month.read_block(block, &mut out));
        assert!(out.is_empty());
        got
    };

    // A zero byte that is set, under a valid checksum.
    let mut bytes = good.clone();
    bytes[512 + 16] = 1;
    let crc = crate::crc::crc32c_split(&bytes[512..532], &0u64.to_le_bytes());
    bytes[532..536].copy_from_slice(&crc.to_le_bytes());
    assert_eq!(read(bytes, 0).reason(), "entry_checksum");

    // Spans: backwards, past the frames, past a frame's bound, a last frame
    // that stops short.
    let mut bytes = good.clone();
    rewrite_entry(&mut bytes, 1, end0 - 1, crc1, first1);
    assert!(matches!(
        read(bytes, 1),
        SealedError::EntryOrder { block: 1 }
    ));
    let mut bytes = good.clone();
    rewrite_entry(&mut bytes, 0, head.blocks_len as u32 + 1, crc0, first0);
    assert!(matches!(
        read(bytes, 0),
        SealedError::EntryOrder { block: 0 }
    ));
    let mut bytes = good.clone();
    rewrite_entry(&mut bytes, 1, end0 + 14_509, crc1, first1);
    assert!(matches!(
        read(bytes, 1),
        SealedError::EntryOrder { block: 1 }
    ));
    let mut bytes = good.clone();
    rewrite_entry(&mut bytes, last, end_last - 1, crc_last, first_last);
    assert!(matches!(read(bytes, last), SealedError::EntryOrder { .. }));

    // The frame's checksum, and a first stamp that is not the frame's.
    let mut bytes = good.clone();
    rewrite_entry(&mut bytes, 1, end1, crc1 ^ 1, first1);
    match read(bytes, 1) {
        SealedError::Frame { block, why } => {
            assert_eq!(block, 1);
            assert!(matches!(why, FrameError::Checksum { .. }));
        }
        other => panic!("{other}"),
    }
    let mut bytes = good.clone();
    rewrite_entry(&mut bytes, 1, end1, crc1, first1 + 1);
    assert!(matches!(
        read(bytes.clone(), 1),
        SealedError::FirstStamp { block: 1 }
    ));
    let month = open(path, bytes);
    assert!(matches!(
        refusal(month.first_at_or_after(bars[300].ts_micros)),
        SealedError::FirstStamp { block: 1 }
    ));
    assert_eq!(refusal(month.audit()).reason(), "first_stamp");

    // A frame that holds more rows than its block: the head says fewer.
    let n = head.n_valid - 10;
    let mut bytes = good.clone();
    bytes[24..32].copy_from_slice(&n.to_le_bytes());
    bytes[40..48].copy_from_slice(&bars[n as usize - 1].ts_micros.to_le_bytes());
    reseal(&mut bytes);
    let month = open(path, bytes);
    match refusal(month.read_block(last, &mut out)) {
        SealedError::FrameRows {
            block,
            expected,
            found,
        } => assert_eq!((block, found - expected), (last, 10)),
        other => panic!("{other}"),
    }
    assert_eq!(
        refusal(month.first_at_or_after(bars[n as usize - 2].ts_micros)).reason(),
        "frame_rows"
    );

    // Time-table entries: a checksum, a block past the month, a decrease.
    let table_at = |index: usize| head.table_off as usize + 8 * index;
    let ts = bars[9_000].ts_micros;
    let s = ((ts - head.origin_micros) / (4_096 * SECOND)) as u64;
    let mut bytes = good.clone();
    bytes[table_at(s as usize) + 4] ^= 1;
    assert!(
        matches!(refusal(open(path, bytes).first_at_or_after(ts)), SealedError::Table { index } if index == s)
    );
    let mut bytes = good.clone();
    bytes[table_at(s as usize)..table_at(s as usize) + 8]
        .copy_from_slice(&table_image(s, head.n_blocks + 1));
    assert!(
        matches!(refusal(open(path, bytes).first_at_or_after(ts)), SealedError::TableOrder { index } if index == s)
    );
    let mut bytes = good.clone();
    bytes[table_at(s as usize + 1)..table_at(s as usize + 1) + 8]
        .copy_from_slice(&table_image(s + 1, 0));
    assert!(
        matches!(refusal(open(path, bytes).first_at_or_after(ts)), SealedError::TableOrder { index } if index == s + 1)
    );
    let mut bytes = good.clone();
    bytes[table_at(s as usize)..table_at(s as usize) + 8]
        .copy_from_slice(&table_image(s, head.n_blocks));
    bytes[table_at(s as usize + 1)..table_at(s as usize + 1) + 8]
        .copy_from_slice(&table_image(s + 1, head.n_blocks));
    assert!(
        matches!(refusal(open(path, bytes).first_at_or_after(ts)), SealedError::TableOrder { index } if index == s)
    );

    // A refused frame inside a page leaves the caller's buffer as it was.
    let mut bytes = good.clone();
    let frame_at = head.blocks_off as usize + end0 as usize + 3;
    bytes[frame_at] ^= 0x40;
    let month = open(path, bytes);
    let mut page = vec![Bar::default(); 3];
    assert_eq!(
        refusal(month.read_rows(200, 400, &mut page)).reason(),
        "frame"
    );
    assert_eq!(page, vec![Bar::default(); 3]);
}

#[test]
fn read_rows_matches_the_rows_everywhere_and_refuses_past_the_month() {
    // Every refusal below is one `store.sealed` line: held off the sink
    // `crate::emits` reads, so its count is its own.
    let _sink = crate::emits::hold_the_sink();
    let path = nifty(Timeframe::SECOND_1, june());
    let bars = seconds(50_000, 27);
    let month = open(path, sealed(path, &bars).to_bytes());
    let mut rng = Rng(28);
    let n = bars.len() as u64;
    for _ in 0..500 {
        let first = rng.below(n);
        let count = rng.below((n - first).min(MAX_RANGE_ROWS) + 1);
        let mut page = Vec::new();
        assert_eq!(
            month.read_rows(first, count, &mut page).unwrap() as u64,
            count
        );
        assert_eq!(page, bars[first as usize..(first + count) as usize]);
    }
    let mut page = Vec::new();
    assert_eq!(
        month.read_rows(0, MAX_RANGE_ROWS, &mut page).unwrap() as u64,
        MAX_RANGE_ROWS
    );
    for (first, count) in [
        (0, MAX_RANGE_ROWS + 1),
        (n, 1),
        (n - 1, 2),
        (u64::MAX, 2),
        (n + 1, 0),
    ] {
        let mut page = vec![Bar::default()];
        match refusal(month.read_rows(first, count, &mut page)) {
            SealedError::Range {
                first: f,
                count: c,
                n_valid,
            } => assert_eq!((f, c, n_valid), (first, count, n)),
            other => panic!("{other}"),
        }
        assert_eq!(page.len(), 1);
    }
    assert_eq!(month.read_rows(n, 0, &mut page).unwrap(), 0);
    let mut out = Vec::new();
    assert_eq!(
        refusal(month.read_block(u64::from(month.head().n_blocks), &mut out)).reason(),
        "range"
    );
}

#[test]
fn first_at_or_after_matches_a_scan_on_every_rung_and_shape() {
    let mut rng = Rng(29);
    for (timeframe, step) in [
        (Timeframe::SECOND_1, SECOND),
        (Timeframe::MINUTE_1, MINUTE),
        (Timeframe::MINUTE_5, 5 * MINUTE),
        (Timeframe::DAY_1, DAY),
    ] {
        let path = nifty(timeframe, june());
        // Sessions of 22,500 seconds a day, on weekdays only, so the month
        // has gaps the size of nights and weekends.
        let session = 22_500 * SECOND / step;
        let mut bars = Vec::new();
        for day in 0..30 {
            if day % 7 >= 5 {
                continue;
            }
            let start = origin(june()) + day * DAY + OPEN;
            let mut today = walk(session.max(1) as usize, start, step, day as u64, true);
            bars.append(&mut today);
        }
        bars.truncate(400_000);
        let month = open(path, sealed(path, &bars).to_bytes());
        let first = bars[0].ts_micros;
        let last = bars[bars.len() - 1].ts_micros;
        for _ in 0..3_000 {
            let ts = first - DAY + rng.below((last - first + 2 * DAY) as u64) as i64;
            assert_eq!(
                month.first_at_or_after(ts).unwrap(),
                scan(&bars, ts),
                "{timeframe:?} {ts}"
            );
        }
        month.audit().unwrap();
    }
}

#[test]
fn builder_refusals_are_named() {
    // Every refusal below is one `store.sealed` line: held off the sink
    // `crate::emits` reads, so its count is its own.
    let _sink = crate::emits::hold_the_sink();
    let path = nifty(Timeframe::SECOND_1, june());
    let bars = seconds(300, 30);
    let raw = header(Timeframe::SECOND_1, &bars);
    let with = |edit: fn(&mut Header)| {
        let mut header = raw;
        edit(&mut header);
        header
    };
    let headers = [
        (with(|h| h.format_version = 2), "unverified_source"),
        (with(|h| h.format_version = 4), "raw_version"),
        (with(|h| h.format_version = 1), "raw_version"),
        (with(|h| h.record_stride = 24), "stride"),
        (with(|h| h.flags = 0), "raw_flags"),
        (with(|h| h.flags = 3), "raw_flags"),
        (with(|h| h.timeframe_secs = 60), "misfiled"),
        (with(|h| h.n_valid = u64::MAX), "too_large"),
    ];
    for (header, reason) in headers {
        assert_eq!(
            refusal(Builder::new(path, &header).map(|_| ())).reason(),
            reason
        );
    }
    // One row too many, one too few, rows out of order, rows outside the
    // month, rows that disagree with the header's stamps.
    let mut builder = Builder::new(path, &raw).unwrap();
    for bar in &bars {
        builder.push(bar).unwrap();
    }
    assert!(matches!(
        refusal(builder.push(&bars[0])),
        SealedError::TooManyRows { n_valid: 300 }
    ));
    let got = refusal(build(path, &raw, &bars[..299], None, None));
    assert!(
        matches!(
            got,
            SealedError::Count {
                expected: 300,
                found: 299
            }
        ),
        "{got}"
    );
    let mut backwards = bars.clone();
    backwards[150].ts_micros = backwards[149].ts_micros - 1;
    assert!(matches!(
        refusal(build(path, &raw, &backwards, None, None)),
        SealedError::OutOfOrder { row: 150 }
    ));
    for ts in [origin(june()) - 1, june_end(), i64::MIN, i64::MAX] {
        let mut outside = bars.clone();
        outside[0].ts_micros = ts;
        if ts >= june_end() {
            outside[299].ts_micros = ts;
            outside[0].ts_micros = bars[0].ts_micros;
            let got = refusal(build(path, &raw, &outside, None, None));
            assert!(
                matches!(got, SealedError::OutsideMonth { row: 299 }),
                "{got}"
            );
        } else {
            let got = refusal(build(path, &raw, &outside, None, None));
            assert!(matches!(got, SealedError::OutsideMonth { row: 0 }), "{got}");
        }
    }
    for edit in [
        (|h: &mut Header| h.first_ts_micros += 1) as fn(&mut Header),
        |h: &mut Header| h.last_ts_micros -= 1,
    ] {
        let got = refusal(build(path, &with(edit), &bars, None, None));
        assert_eq!(got.reason(), "header_stamps");
    }
    // The month's first and last microseconds and equal stamps are accepted.
    let mut edge = seconds(3, 31);
    edge[0].ts_micros = origin(june());
    edge[1].ts_micros = edge[0].ts_micros;
    edge[2].ts_micros = june_end() - 1;
    let month = open(path, sealed(path, &edge).to_bytes());
    check_reads(&month, &edge);
}

#[test]
fn a_file_on_disk_reads_the_same_as_its_bytes() {
    let path = nifty(Timeframe::SECOND_1, june());
    let bars = seconds(20_000, 32);
    let image = prv_image(path, 4);
    let built = build(
        path,
        &header(Timeframe::SECOND_1, &bars),
        &bars,
        Some(&image),
        None,
    )
    .unwrap();
    let file = std::env::temp_dir().join(format!(
        "brutex-sealed-unit-{}-{}.sbar",
        std::process::id(),
        line!()
    ));
    let mut out = std::fs::File::create(&file).unwrap();
    built.write_to(&mut out).unwrap();
    drop(out);
    let month = SealedMonth::open(std::fs::File::open(&file).unwrap(), path, SYMBOL).unwrap();
    assert_eq!(month.head(), built.head());
    let mut page = Vec::new();
    month.read_rows(1_000, 5_000, &mut page).unwrap();
    assert_eq!(page, bars[1_000..6_000]);
    assert_eq!(
        month.first_at_or_after(bars[7_777].ts_micros).unwrap(),
        7_777
    );
    assert_eq!(month.provenance().unwrap(), Some(image));
    assert_eq!(month.audit().unwrap().rows, 20_000);
    let held = built.to_bytes();
    let by_reference = SealedMonth::open(&held[..], path, SYMBOL).unwrap();
    assert_eq!(by_reference.head(), built.head());
    std::fs::remove_file(&file).unwrap();
}

#[test]
fn a_device_error_is_named_with_what_was_being_read() {
    // Every refusal below is one `store.sealed` line: held off the sink
    // `crate::emits` reads, so its count is its own.
    let _sink = crate::emits::hold_the_sink();
    let path = nifty(Timeframe::SECOND_1, june());
    let bars = seconds(20_000, 33);
    let image = prv_image(path, 2);
    let built = build(
        path,
        &header(Timeframe::SECOND_1, &bars),
        &bars,
        Some(&image),
        Some(Summary {
            version: 1,
            bytes: b"x",
        }),
    )
    .unwrap();
    let bytes = built.to_bytes();
    let head = *built.head();
    let failing = |fail_at: u64, fail_len: bool| Failing {
        bytes: bytes.clone(),
        fail_at,
        fail_len,
    };
    let action = |got: SealedError| match got {
        SealedError::Io { action, .. } => action,
        other => panic!("{other}"),
    };
    assert_eq!(
        action(refusal(SealedMonth::open(
            failing(u64::MAX, true),
            path,
            SYMBOL
        ))),
        "measuring the sealed month"
    );
    assert_eq!(
        action(refusal(SealedMonth::open(failing(10, false), path, SYMBOL))),
        "reading the head"
    );
    let month = SealedMonth::open(failing(512 + 24 * 3, false), path, SYMBOL).unwrap();
    let mut out = Vec::new();
    assert_eq!(
        action(refusal(month.read_block(3, &mut out))),
        "reading directory entries"
    );
    let month = SealedMonth::open(failing(head.blocks_off + 10, false), path, SYMBOL).unwrap();
    assert_eq!(
        action(refusal(month.read_block(0, &mut out))),
        "reading frames"
    );
    assert_eq!(
        action(refusal(month.first_at_or_after(bars[5].ts_micros))),
        "reading a frame's stamps"
    );
    assert_eq!(action(refusal(month.audit())), "reading a frame");
    let s = (bars[9_000].ts_micros - head.origin_micros) / (4_096 * SECOND);
    let month = SealedMonth::open(
        failing(head.table_off + 8 * s as u64 + 1, false),
        path,
        SYMBOL,
    )
    .unwrap();
    assert_eq!(
        action(refusal(month.first_at_or_after(bars[9_000].ts_micros))),
        "reading the time table"
    );
    let month = SealedMonth::open(failing(head.prv_off, false), path, SYMBOL).unwrap();
    assert_eq!(
        action(refusal(month.provenance())),
        "reading the provenance"
    );
    let month = SealedMonth::open(failing(head.sum_off, false), path, SYMBOL).unwrap();
    assert_eq!(
        action(refusal(month.summary())),
        "reading the month summary"
    );
    // Past the end of bytes held in memory.
    let mut buf = [0u8; 4];
    assert_eq!(
        bytes
            .read_exact_at(&mut buf, bytes.len() as u64 - 3)
            .unwrap_err()
            .kind(),
        io::ErrorKind::UnexpectedEof
    );
    assert_eq!(
        bytes.read_exact_at(&mut buf, u64::MAX).unwrap_err().kind(),
        io::ErrorKind::UnexpectedEof
    );
}

#[test]
fn every_refusal_has_its_own_name_and_says_what() {
    let io_error = || io::Error::other("invented");
    let all = [
        SealedError::TooShort { len: 1 },
        SealedError::Magic,
        SealedError::Version { found: 2 },
        SealedError::HeadChecksum {
            stored: 1,
            computed: 2,
        },
        SealedError::SlotChecksum {
            stored: 1,
            computed: 2,
        },
        SealedError::Stride { found: 1 },
        SealedError::Flags { found: 8 },
        SealedError::Reserved { offset: 60 },
        SealedError::BlockRows { found: 0 },
        SealedError::Codec { found: 2 },
        SealedError::EntryLen { found: 20 },
        SealedError::RawVersion { found: 4 },
        SealedError::UnverifiedSource,
        SealedError::RawFlags { found: 0 },
        SealedError::TableSlots { found: 1 },
        SealedError::Geometry {
            field: "n_blocks",
            head: 1,
            expected: 2,
        },
        SealedError::Length { head: 1, file: 2 },
        SealedError::Misfiled { field: "path" },
        SealedError::Stamps { first: 2, last: 1 },
        SealedError::Columns { column: "open" },
        SealedError::Extremes {
            extreme: "close_max",
        },
        SealedError::Entry { index: 3 },
        SealedError::EntryOrder { block: 3 },
        SealedError::Frame {
            block: 3,
            why: FrameError::Codec { found: 9 },
        },
        SealedError::FrameRows {
            block: 3,
            expected: 256,
            found: 255,
        },
        SealedError::FirstStamp { block: 3 },
        SealedError::Table { index: 4 },
        SealedError::TableOrder { index: 4 },
        SealedError::Provenance { why: "short" },
        SealedError::Summary { why: "version" },
        SealedError::Range {
            first: 1,
            count: 2,
            n_valid: 0,
        },
        SealedError::Io {
            action: "reading",
            offset: 9,
            source: io_error(),
        },
        SealedError::Audit { field: "columns" },
        SealedError::OutOfOrder { row: 5 },
        SealedError::OutsideMonth { row: 5 },
        SealedError::TooManyRows { n_valid: 5 },
        SealedError::Count {
            expected: 5,
            found: 4,
        },
        SealedError::HeaderStamps,
        SealedError::TooLarge,
        SealedError::ProvenanceTooLong { len: 9 },
        SealedError::ProvenanceForeign,
        SealedError::SummaryTooLong { len: 9 },
        SealedError::SummaryVersion,
    ];
    let mut names = std::collections::BTreeSet::new();
    for refusal in &all {
        let text = refusal.to_string();
        assert!(
            text.starts_with(&format!("{}: ", refusal.reason())),
            "{text}"
        );
        assert!(text.len() > refusal.reason().len() + 10, "{text}");
        assert!(names.insert(refusal.reason()), "{} twice", refusal.reason());
        let source = std::error::Error::source(refusal);
        assert_eq!(
            source.is_some(),
            matches!(refusal, SealedError::Io { .. } | SealedError::Frame { .. }),
            "{text}"
        );
    }
    assert_eq!(names.len(), all.len());
}

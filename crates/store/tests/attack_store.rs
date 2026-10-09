//! Adversarial round against `store::file::BarFile` and the bar format.
//!
//! Model-based: a seeded PRNG drives 100,000 operations over a few hundred
//! fresh month files and every answer is compared with an in-memory `Vec<Bar>`
//! that holds what the month should hold. Exhaustive: every byte of a small
//! sealed month is flipped in turn and the reader must refuse or serve the
//! original bytes, never a different bar. Hand-picked: the extreme stamps, the
//! open-interest sentinel, a bad row in the middle of a batch, a header from a
//! version this build does not know, the lock.
//!
//! Deterministic: one fixed seed, fixed case counts, so a rerun is the same
//! run (`CLAUDE.md` §3 rule 5). Scratch directories live under the host's temp
//! directory and remove themselves.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use brutex_core::vendor::Vendor;

use store::crc::crc32c;
use store::file::{Appended, BarFile, StoreError};
use store::format::{Bar, FormatError, HEADER_LEN, OI_NULL};
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

// ===========================================================================
// Scratch and fixtures
// ===========================================================================

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Scratch {
    root: PathBuf,
}

impl Scratch {
    fn new(tag: &str) -> Self {
        let serial = NEXT.fetch_add(1, Ordering::Relaxed);
        let mut root = std::env::temp_dir();
        root.push(format!(
            "brutex-attack-store-{}-{tag}-{serial}",
            std::process::id()
        ));
        drop(fs::remove_dir_all(&root));
        fs::create_dir_all(&root).expect("scratch root");
        Self { root }
    }

    fn root(&self) -> &Path {
        &self.root
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        drop(fs::remove_dir_all(&self.root));
    }
}

const MINUTE: i64 = 60_000_000;
const SECOND: i64 = 1_000_000;
const SYMBOL: u32 = 26_000;

fn june() -> YearMonth {
    YearMonth::new(2024, 6).expect("2024-06")
}

/// The first and one-past-last microsecond the June 2024 month admits.
fn bounds() -> (i64, i64) {
    june().ist_bounds_micros()
}

/// Minutes in the month.
fn minutes_in_month() -> i64 {
    let (from, until) = bounds();
    (until - from) / MINUTE
}

fn path_at(timeframe: Timeframe) -> StorePath<'static> {
    StorePath::new(PathParts {
        vendor: Vendor::Groww,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "NIFTY",
        contract: None,
        timeframe,
        month: june(),
        file: FileKind::Bars,
    })
    .expect("a legal path")
}

fn bars_path() -> StorePath<'static> {
    path_at(Timeframe::MINUTE_1)
}

fn open(root: &Path) -> Result<BarFile, StoreError> {
    BarFile::open_or_create(root, bars_path(), SYMBOL)
}

fn image(root: &Path) -> Vec<u8> {
    fs::read(bars_path().to_path_buf(root)).expect("the bar file")
}

fn sidecar(root: &Path) -> Vec<u8> {
    fs::read(bars_path().with_file(FileKind::Checksums).to_path_buf(root)).unwrap_or_default()
}

/// Both files' bytes, for "a refusal leaves the month byte-identical".
fn digest(root: &Path) -> (u32, u32) {
    (crc32c(&image(root)), crc32c(&sidecar(root)))
}

// ===========================================================================
// PRNG — splitmix64, fixed seed
// ===========================================================================

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in `0..n`, `n > 0`. The modulo bias is irrelevant to a test.
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }

    fn range(&mut self, lo: i64, hi_exclusive: i64) -> i64 {
        let span = u64::try_from(hi_exclusive - lo).expect("non-empty range");
        lo + i64::try_from(self.below(span)).expect("in range")
    }

    fn chance(&mut self, percent: u64) -> bool {
        self.below(100) < percent
    }
}

/// A sane bar at `ts`, values drawn from `rng` and including the extremes.
fn sane_bar(rng: &mut Rng, ts_micros: i64) -> Bar {
    let low = match rng.below(8) {
        0 => 0,
        1 => i64::MAX - 1,
        _ => rng.range(1, 10_000_000_000),
    };
    let high = match rng.below(8) {
        0 => i64::MAX,
        _ => low.saturating_add(rng.range(0, 1_000_000)),
    };
    let open = rng
        .range(low, high.saturating_add(1).max(low + 1))
        .min(high);
    let close = rng
        .range(low, high.saturating_add(1).max(low + 1))
        .min(high);
    let volume = match rng.below(6) {
        0 => 0,
        1 => i64::MAX,
        _ => rng.range(0, 5_000_000),
    };
    let open_interest = match rng.below(5) {
        0 | 1 => OI_NULL,
        2 => 0,
        3 => i64::MAX,
        _ => rng.range(1, 1_000_000_000),
    };
    Bar {
        ts_micros,
        open,
        high,
        low,
        close,
        volume,
        open_interest,
    }
}

fn minute(m: i64) -> i64 {
    bounds().0 + m * MINUTE
}

// ===========================================================================
// The oracle
// ===========================================================================

/// What `append` must answer for `batch` against a month holding `model`,
/// as a `Result` of the outcome kind and, on success, the outcome itself.
#[derive(Debug, PartialEq, Eq)]
enum Expect {
    Ok(Appended),
    Empty,
    Impossible,
    Count,
    Unordered,
    Outside,
    OffGrid,
    Overlap,
}

fn oracle(model: &[Bar], batch: &[Bar]) -> Expect {
    if batch.is_empty() {
        return Expect::Empty;
    }
    let mut previous: Option<i64> = None;
    for bar in batch {
        if !bar.ohlc_is_sane() {
            return Expect::Impossible;
        }
        if !bar.counts_are_sane() {
            return Expect::Count;
        }
        if let Some(earlier) = previous
            && bar.ts_micros <= earlier
        {
            return Expect::Unordered;
        }
        previous = Some(bar.ts_micros);
    }
    let (from, until) = bounds();
    for bar in batch {
        if bar.ts_micros < from || bar.ts_micros >= until {
            return Expect::Outside;
        }
        if (bar.ts_micros - from).rem_euclid(MINUTE) != 0 {
            return Expect::OffGrid;
        }
    }
    let len = u64::try_from(model.len()).unwrap();
    let count = u64::try_from(batch.len()).unwrap();
    let first = batch[0].ts_micros;
    let last_held = model.last().map(|bar| bar.ts_micros);
    if last_held.is_none_or(|last| first > last) {
        return Expect::Ok(Appended::Committed {
            first_index: len,
            n_valid: len + count,
        });
    }
    let at = model.partition_point(|bar| bar.ts_micros < first);
    if at + batch.len() <= model.len() && model[at..at + batch.len()] == *batch {
        return Expect::Ok(Appended::AlreadyPresent {
            first_index: u64::try_from(at).unwrap(),
            n_valid: len,
        });
    }
    let last = last_held.unwrap();
    let split = batch.partition_point(|bar| bar.ts_micros <= last);
    let suffix = batch.len() - split;
    if suffix > 0 && split <= model.len() && model[model.len() - split..] == batch[..split] {
        return Expect::Ok(Appended::Committed {
            first_index: len,
            n_valid: len + u64::try_from(suffix).unwrap(),
        });
    }
    Expect::Overlap
}

fn observed(result: &Result<Appended, StoreError>) -> Expect {
    match result {
        Ok(done) => Expect::Ok(*done),
        Err(StoreError::EmptyBatch) => Expect::Empty,
        Err(StoreError::ImpossibleBar { .. }) => Expect::Impossible,
        Err(StoreError::ImpossibleCount { .. }) => Expect::Count,
        Err(StoreError::BatchNotOrdered { .. }) => Expect::Unordered,
        Err(StoreError::OutsideMonth { .. }) => Expect::Outside,
        Err(StoreError::OffGrid { .. }) => Expect::OffGrid,
        Err(StoreError::OverlapDisagrees { .. }) => Expect::Overlap,
        Err(other) => panic!("an append refusal the oracle has no name for: {other:?}"),
    }
}

/// Applies an expected success to the model.
fn apply(model: &mut Vec<Bar>, batch: &[Bar], expect: &Expect) {
    if let Expect::Ok(Appended::Committed { first_index, .. }) = expect {
        let held_before = usize::try_from(*first_index).unwrap();
        assert_eq!(held_before, model.len());
        let last = model.last().map_or(i64::MIN, |bar| bar.ts_micros);
        for bar in batch {
            if bar.ts_micros > last {
                model.push(*bar);
            }
        }
    }
}

// ===========================================================================
// Batch generators
// ===========================================================================

/// A fresh run of `count` bars starting `gap` minutes after `after_minute`.
fn fresh_run(rng: &mut Rng, start_minute: i64, count: usize) -> Vec<Bar> {
    let mut out = Vec::with_capacity(count);
    let mut m = start_minute;
    for _ in 0..count {
        if m >= minutes_in_month() {
            break;
        }
        out.push(sane_bar(rng, minute(m)));
        m += rng.range(1, 4);
    }
    out
}

fn last_minute(model: &[Bar]) -> i64 {
    model
        .last()
        .map_or(-1, |bar| (bar.ts_micros - bounds().0) / MINUTE)
}

fn random_batch(rng: &mut Rng, model: &[Bar]) -> Vec<Bar> {
    let len = model.len();
    let next_minute = last_minute(model) + rng.range(1, 6);
    match rng.below(100) {
        // Extend the month.
        0..=44 => {
            let count = usize::try_from(rng.range(1, 60)).unwrap();
            fresh_run(rng, next_minute, count)
        }
        // Replay a held sub-run exactly.
        45..=54 if len > 0 => {
            let a = usize::try_from(rng.below(len as u64)).unwrap();
            let b = a + 1 + usize::try_from(rng.below((len - a) as u64)).unwrap();
            model[a..b].to_vec()
        }
        // A held tail, then new bars: the resumed backfill.
        55..=64 if len > 0 => {
            let a = usize::try_from(rng.below(len as u64)).unwrap();
            let mut out = model[a..].to_vec();
            let extra = 1 + usize::try_from(rng.below(20)).unwrap();
            out.extend(fresh_run(rng, next_minute, extra));
            out
        }
        // A held sub-run with one bar restated.
        65..=69 if len > 0 => {
            let a = usize::try_from(rng.below(len as u64)).unwrap();
            let b = a + 1 + usize::try_from(rng.below((len - a) as u64)).unwrap();
            let mut out = model[a..b].to_vec();
            let pick = a + usize::try_from(rng.below((b - a) as u64)).unwrap() - a;
            out[pick].volume = out[pick].volume.wrapping_add(1).max(0);
            if out[pick] == model[a + pick] {
                out[pick].close = out[pick].low;
                out[pick].open = out[pick].high;
            }
            if rng.chance(50) {
                out.extend(fresh_run(rng, next_minute, 3));
            }
            out
        }
        // A held run with one bar dropped (a gap over a held stamp).
        70..=73 if len > 2 => {
            let a = usize::try_from(rng.below((len - 2) as u64)).unwrap();
            let mut out = model[a..].to_vec();
            out.remove(1);
            if rng.chance(50) {
                out.extend(fresh_run(rng, next_minute, 2));
            }
            out
        }
        // A stamp the month never held, inside the held range.
        74..=76 if len > 1 => {
            let first = (model[0].ts_micros - bounds().0) / MINUTE;
            let probe = rng.range(first.max(1) - 1, last_minute(model) + 1);
            let ts = minute(probe);
            if model.iter().any(|bar| bar.ts_micros == ts) {
                vec![sane_bar(rng, ts)]
            } else {
                let mut out = vec![sane_bar(rng, ts)];
                out.extend(fresh_run(rng, next_minute, 2));
                out
            }
        }
        // Bad batches: one corrupt row anywhere in an otherwise good batch.
        77..=94 => {
            let count = usize::try_from(rng.range(1, 12)).unwrap();
            let mut out = fresh_run(rng, next_minute, count);
            if out.is_empty() {
                return out;
            }
            let at = usize::try_from(rng.below(out.len() as u64)).unwrap();
            match rng.below(9) {
                0 => out[at].high = out[at].low - 1,
                1 => out[at].volume = -1,
                2 => out[at].open_interest = -1,
                3 => {
                    let ts = out[at.saturating_sub(1)].ts_micros;
                    out[at].ts_micros = ts;
                }
                4 => out[at].ts_micros += SECOND,
                5 => out[at].ts_micros = i64::MIN,
                6 => out[at].ts_micros = i64::MAX,
                7 => out[at].ts_micros = bounds().1,
                _ => {
                    out[at].low = -1;
                    out[at].open = -1;
                    out[at].close = -1;
                }
            }
            out
        }
        _ => Vec::new(),
    }
}

// ===========================================================================
// 1. Model-based: 100,000 operations against a Vec
// ===========================================================================

/// One month file under test, the model of what it should hold, and the
/// writer currently held on it (`None` while a reader walk has it).
struct Session {
    scratch: Scratch,
    model: Vec<Bar>,
    file: Option<BarFile>,
}

/// How many of each operation a model run performed.
#[derive(Debug, Default)]
struct Tally {
    committed: u64,
    present: u64,
    refused: u64,
    reads: u64,
    lookups: u64,
    lock_probes: u64,
    reopens: u64,
    reader_walks: u64,
    held: u64,
}

impl Session {
    fn new() -> Self {
        let scratch = Scratch::new("model");
        let file = Some(open(scratch.root()).expect("create"));
        Self {
            scratch,
            model: Vec::new(),
            file,
        }
    }

    fn len(&self) -> u64 {
        u64::try_from(self.model.len()).unwrap()
    }

    fn writer(&mut self) -> &mut BarFile {
        self.file.as_mut().expect("a writer is held")
    }

    fn append(&mut self, rng: &mut Rng, op: u64, tally: &mut Tally) {
        let batch = random_batch(rng, &self.model);
        let expect = oracle(&self.model, &batch);
        let before = digest(self.scratch.root());
        let got = self.writer().append(&batch);
        assert_eq!(
            observed(&got),
            expect,
            "op {op}: batch {batch:?}\nmodel len {}",
            self.model.len()
        );
        match &expect {
            Expect::Ok(Appended::Committed { .. }) => tally.committed += 1,
            Expect::Ok(Appended::AlreadyPresent { .. }) => {
                tally.present += 1;
                assert_eq!(digest(self.scratch.root()), before, "op {op}: rerun wrote");
            }
            _ => {
                tally.refused += 1;
                assert_eq!(
                    digest(self.scratch.root()),
                    before,
                    "op {op}: refusal wrote"
                );
            }
        }
        apply(&mut self.model, &batch, &expect);
        let len = self.len();
        assert_eq!(self.writer().records(), len);
    }

    fn read(&mut self, rng: &mut Rng, op: u64) {
        let len = self.len();
        let index = match rng.below(6) {
            0 => len,
            1 => u64::MAX,
            2 => len.saturating_sub(1),
            3 => 0,
            _ => rng.below(len + 1),
        };
        match self.writer().read_record(index) {
            Ok(bar) => assert_eq!(bar, self.model[usize::try_from(index).unwrap()]),
            Err(StoreError::NotCommitted { index: i, n_valid }) => {
                assert!(index >= len, "op {op}: index {index} of {len} refused");
                assert_eq!((i, n_valid), (index, len));
            }
            Err(other) => panic!("op {op}: read_record({index}): {other:?}"),
        }
    }

    fn lookup(&mut self, rng: &mut Rng, op: u64) {
        let held = self.model.len() as u64;
        let ts = match rng.below(8) {
            0 => i64::MIN,
            1 => i64::MAX,
            2 if held > 0 => self.model[usize::try_from(rng.below(held)).unwrap()].ts_micros,
            3 if held > 0 => self.model[usize::try_from(rng.below(held)).unwrap()].ts_micros + 1,
            4 => bounds().0 - 1,
            _ => minute(rng.range(-5, minutes_in_month() + 5)),
        };
        let want = u64::try_from(self.model.partition_point(|bar| bar.ts_micros < ts)).unwrap();
        assert_eq!(
            self.writer().first_at_or_after(ts).expect("lookup"),
            want,
            "op {op}: first_at_or_after({ts})"
        );
    }

    /// The writer holds the month: a second writer and a reader are both
    /// refused by name.
    fn locks(&self) {
        let root = self.scratch.root();
        assert!(matches!(open(root), Err(StoreError::Locked { .. })));
        assert!(matches!(
            BarFile::open_existing(root, bars_path(), SYMBOL),
            Err(StoreError::Locked { .. })
        ));
    }

    /// Reopen as a writer: same records, same bytes.
    fn reopen(&mut self, op: u64) {
        let before = digest(self.scratch.root());
        drop(self.file.take());
        let reopened = open(self.scratch.root()).expect("reopen");
        assert_eq!(reopened.records(), self.len());
        assert_eq!(digest(self.scratch.root()), before, "op {op}: reopen wrote");
        self.file = Some(reopened);
    }

    /// Drop the writer, read the whole month through the reader door with two
    /// readers at once, then take the writer back.
    fn reader_walk(&mut self) {
        drop(self.file.take());
        let root = self.scratch.root();
        let reader = BarFile::open_existing(root, bars_path(), SYMBOL).expect("reader");
        let second =
            BarFile::open_existing(root, bars_path(), SYMBOL).expect("a second reader shares");
        // Refused by name as READERS, not as another writer (barflow-1,
        // D-2552; D-4612).
        assert!(matches!(open(root), Err(StoreError::ReaderHolds { .. })));
        for (index, want) in self.model.iter().enumerate() {
            let index = u64::try_from(index).unwrap();
            assert_eq!(reader.read_record(index).expect("read"), *want);
        }
        assert_eq!(second.records(), reader.records());
        drop((reader, second));
        self.file = Some(open(root).expect("writer again"));
    }
}

#[test]
fn one_hundred_thousand_random_operations_agree_with_an_in_memory_vec() {
    const OPS: u64 = 100_000;
    const OPS_PER_FILE: u64 = 400;
    let mut rng = Rng(0x00C0_FFEE_D00D_5EED);
    let mut tally = Tally::default();
    let mut files = 0u64;
    let mut op = 0u64;
    while op < OPS {
        let mut session = Session::new();
        files += 1;
        for _ in 0..OPS_PER_FILE.min(OPS - op) {
            op += 1;
            match rng.below(100) {
                0..=29 => session.append(&mut rng, op, &mut tally),
                30..=59 => {
                    tally.reads += 1;
                    session.read(&mut rng, op);
                }
                60..=91 => {
                    tally.lookups += 1;
                    session.lookup(&mut rng, op);
                }
                92..=95 => {
                    tally.lock_probes += 1;
                    session.locks();
                }
                96..=97 => {
                    tally.reopens += 1;
                    session.reopen(op);
                }
                _ => {
                    tally.reader_walks += 1;
                    session.reader_walk();
                }
            }
        }
        tally.held += session.len();
    }
    // The run exercised every branch it claims to.
    assert_eq!(op, OPS);
    assert!(tally.committed > 1_000, "{tally:?}");
    assert!(tally.present > 100, "{tally:?}");
    assert!(tally.refused > 1_000, "{tally:?}");
    assert!(tally.reads > 10_000 && tally.lookups > 10_000, "{tally:?}");
    assert!(tally.reopens > 100 && tally.reader_walks > 100, "{tally:?}");
    assert_eq!(files, OPS / OPS_PER_FILE);
    eprintln!("model run: {files} files, {tally:?}");
}

// ===========================================================================
// 2. Hand-picked edges
// ===========================================================================

#[test]
fn an_empty_month_answers_every_lookup_without_reading() {
    let scratch = Scratch::new("empty");
    let file = open(scratch.root()).expect("create");
    for ts in [i64::MIN, -1, 0, 1, bounds().0, bounds().1, i64::MAX] {
        assert_eq!(file.first_at_or_after(ts).expect("lookup"), 0, "ts {ts}");
    }
    for index in [0, 1, u64::MAX] {
        assert!(matches!(
            file.read_record(index),
            Err(StoreError::NotCommitted { n_valid: 0, .. })
        ));
    }
}

#[test]
fn a_bad_row_anywhere_in_a_batch_leaves_the_month_byte_identical() {
    let scratch = Scratch::new("middle");
    let mut file = open(scratch.root()).expect("create");
    let mut rng = Rng(7);
    let good: Vec<Bar> = (0..10).map(|m| sane_bar(&mut rng, minute(m))).collect();
    file.append(&good).expect("seed");
    let before = digest(scratch.root());
    let mut refused = 0;
    for at in 0..9usize {
        for corruption in 0..6 {
            let mut batch: Vec<Bar> = (10..19).map(|m| sane_bar(&mut rng, minute(m))).collect();
            match corruption {
                0 => batch[at].high = -5,
                1 => batch[at].volume = i64::MIN,
                2 => batch[at].open_interest = OI_NULL + 1,
                3 => batch[at].ts_micros = i64::MAX,
                4 => batch[at].ts_micros += 1,
                _ => batch[at].ts_micros = bounds().0 - MINUTE,
            }
            assert!(
                file.append(&batch).is_err(),
                "row {at} corruption {corruption}"
            );
            assert_eq!(digest(scratch.root()), before);
            assert_eq!(file.records(), 10);
            refused += 1;
        }
    }
    assert_eq!(refused, 54);
}

#[test]
fn the_open_interest_sentinel_round_trips_and_is_distinct_from_zero() {
    let scratch = Scratch::new("oi");
    let mut file = open(scratch.root()).expect("create");
    let mut rng = Rng(11);
    let mut batch: Vec<Bar> = (0..4).map(|m| sane_bar(&mut rng, minute(m))).collect();
    batch[0].open_interest = OI_NULL;
    batch[1].open_interest = 0;
    batch[2].open_interest = i64::MAX;
    batch[3].open_interest = 1;
    file.append(&batch).expect("append");
    drop(file);
    let reader = BarFile::open_existing(scratch.root(), bars_path(), SYMBOL).expect("reader");
    assert_eq!(reader.read_record(0).expect("0").oi(), None);
    assert_eq!(reader.read_record(1).expect("1").oi(), Some(0));
    assert_eq!(reader.read_record(2).expect("2").oi(), Some(i64::MAX));
    assert_eq!(reader.read_record(3).expect("3").oi(), Some(1));
}

#[test]
fn the_first_and_last_minute_of_the_month_are_admitted_and_their_neighbours_are_not() {
    let scratch = Scratch::new("edges");
    let mut file = open(scratch.root()).expect("create");
    let mut rng = Rng(13);
    let (from, until) = bounds();
    assert!(matches!(
        file.append(&[sane_bar(&mut rng, from - MINUTE)]),
        Err(StoreError::OutsideMonth { .. })
    ));
    file.append(&[sane_bar(&mut rng, from)])
        .expect("first minute");
    file.append(&[sane_bar(&mut rng, until - MINUTE)])
        .expect("last minute");
    assert!(matches!(
        file.append(&[sane_bar(&mut rng, until)]),
        Err(StoreError::OutsideMonth { .. })
    ));
    assert_eq!(file.first_at_or_after(from).expect("lookup"), 0);
    assert_eq!(file.first_at_or_after(from + 1).expect("lookup"), 1);
    assert_eq!(file.first_at_or_after(until - MINUTE).expect("lookup"), 1);
    assert_eq!(file.first_at_or_after(until).expect("lookup"), 2);
}

#[test]
fn a_timestamp_equal_to_the_last_held_with_different_bytes_is_refused() {
    let scratch = Scratch::new("equal");
    let mut file = open(scratch.root()).expect("create");
    let mut rng = Rng(17);
    let held = sane_bar(&mut rng, minute(100));
    file.append(&[held]).expect("seed");
    let before = digest(scratch.root());
    let mut restated = held;
    restated.volume = held.volume.wrapping_add(1).max(0);
    if restated == held {
        restated.volume = 0;
    }
    assert!(matches!(
        file.append(&[restated]),
        Err(StoreError::OverlapDisagrees { .. })
    ));
    assert_eq!(
        file.append(&[held]).expect("rerun"),
        Appended::AlreadyPresent {
            first_index: 0,
            n_valid: 1
        }
    );
    assert_eq!(digest(scratch.root()), before);
}

/// Rewrites the version field of both header slots to `version`, with the
/// magic's last byte to match and a correct slot checksum, so the only thing
/// wrong is that this build does not know the version.
fn forge_version(root: &Path, version: u16) {
    let path = bars_path().to_path_buf(root);
    let mut bytes = fs::read(&path).expect("bars");
    for slot in [0usize, 16_384] {
        let s = &mut bytes[slot..slot + 64];
        if s[..7] != *b"BRUTEXB" {
            continue;
        }
        // The slot checksum covers bytes 0..56 and 60..64.
        let check = |s: &[u8]| {
            let mut covered = s[..56].to_vec();
            covered.extend_from_slice(&s[60..64]);
            crc32c(&covered)
        };
        assert_eq!(
            check(s).to_le_bytes(),
            s[56..60],
            "the forger's checksum model matches the writer's"
        );
        s[7] = b'0' + u8::try_from(version % 10).unwrap();
        s[8..10].copy_from_slice(&version.to_le_bytes());
        let crc = check(s).to_le_bytes();
        s[56..60].copy_from_slice(&crc);
    }
    fs::write(&path, bytes).expect("forge");
}

#[test]
fn a_header_from_a_future_version_is_refused_by_both_doors_and_never_rewritten() {
    for version in [4u16, 9, 10, 99, u16::MAX] {
        let scratch = Scratch::new("future");
        let mut file = open(scratch.root()).expect("create");
        let mut rng = Rng(19);
        file.append(&[sane_bar(&mut rng, minute(0))])
            .expect("one bar");
        drop(file);
        forge_version(scratch.root(), version);
        let before = digest(scratch.root());
        assert!(
            matches!(open(scratch.root()), Err(StoreError::Format { .. })),
            "version {version}: writer door"
        );
        assert!(
            matches!(
                BarFile::open_existing(scratch.root(), bars_path(), SYMBOL),
                Err(StoreError::Format { .. })
            ),
            "version {version}: reader door"
        );
        assert_eq!(
            digest(scratch.root()),
            before,
            "version {version}: rewritten"
        );
    }
}

#[test]
fn a_zero_length_file_is_initialised_by_the_writer_and_refused_by_the_reader() {
    let scratch = Scratch::new("zero");
    let path = bars_path().to_path_buf(scratch.root());
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, b"").unwrap();
    // The reader opens nothing and creates nothing.
    let read = BarFile::open_existing(scratch.root(), bars_path(), SYMBOL);
    assert!(read.is_err(), "a zero-byte month is not a readable month");
    assert_eq!(fs::metadata(&path).unwrap().len(), 0, "the reader wrote");
    let file = open(scratch.root()).expect("the writer initialises it");
    assert_eq!(file.records(), 0);
    assert_eq!(fs::metadata(&path).unwrap().len(), HEADER_LEN);
}

#[test]
fn a_truncated_file_is_refused_at_every_cut_through_the_records() {
    let scratch = Scratch::new("trunc");
    let mut file = open(scratch.root()).expect("create");
    let mut rng = Rng(23);
    let batch: Vec<Bar> = (0..150).map(|m| sane_bar(&mut rng, minute(m))).collect();
    file.append(&batch).expect("append");
    drop(file);
    let full = image(scratch.root());
    let path = bars_path().to_path_buf(scratch.root());
    let mut cases = 0;
    let header = usize::try_from(HEADER_LEN).unwrap();
    for cut in (0..full.len()).step_by(7).chain([full.len() - 1]) {
        fs::write(&path, &full[..cut]).unwrap();
        cases += 1;
        let Ok(reader) = BarFile::open_existing(scratch.root(), bars_path(), SYMBOL) else {
            continue;
        };
        // An open that succeeded must serve the original bars, a prefix of
        // them at most, and never a different one.
        assert!(
            cut >= header,
            "cut {cut}: a file shorter than its header opened"
        );
        let n = usize::try_from(reader.records()).unwrap();
        assert!(n <= batch.len());
        for (index, want) in batch.iter().take(n).enumerate() {
            let Ok(bar) = reader.read_record(u64::try_from(index).unwrap()) else {
                break;
            };
            assert_eq!(bar, *want, "cut {cut} index {index}");
        }
    }
    assert!(cases > 1_000);
}

/// What a reader made of one corrupted month.
#[derive(Default, Debug)]
struct FlipTally {
    open_refused: u64,
    read_refused: u64,
    served_intact: u64,
    served_rolled_back: u64,
}

#[test]
fn every_single_byte_flip_is_refused_or_harmless_and_never_serves_a_different_bar() {
    let scratch = Scratch::new("flip");
    let mut file = open(scratch.root()).expect("create");
    let mut rng = Rng(29);
    let first: Vec<Bar> = (0..80).map(|m| sane_bar(&mut rng, minute(m))).collect();
    let second: Vec<Bar> = (80..150).map(|m| sane_bar(&mut rng, minute(m))).collect();
    file.append(&first).expect("first commit");
    file.append(&second).expect("second commit");
    drop(file);
    let all: Vec<Bar> = first.iter().chain(&second).copied().collect();
    let bars = bars_path().to_path_buf(scratch.root());
    let crc_path = bars_path()
        .with_file(FileKind::Checksums)
        .to_path_buf(scratch.root());
    let clean_bars = fs::read(&bars).unwrap();
    let clean_crc = fs::read(&crc_path).unwrap();
    let mut tally = FlipTally::default();
    let mut rolled_back_at: Vec<usize> = Vec::new();
    let targets: Vec<(bool, usize)> = (0..clean_bars.len())
        .map(|at| (false, at))
        .chain((0..clean_crc.len()).map(|at| (true, at)))
        .collect();
    for (in_sidecar, at) in targets {
        let mut b = clean_bars.clone();
        let mut c = clean_crc.clone();
        if in_sidecar {
            c[at] ^= 0x01;
        } else {
            b[at] ^= 0x01;
        }
        fs::write(&bars, &b).unwrap();
        fs::write(&crc_path, &c).unwrap();
        let Ok(reader) = BarFile::open_existing(scratch.root(), bars_path(), SYMBOL) else {
            tally.open_refused += 1;
            continue;
        };
        let n = usize::try_from(reader.records()).unwrap();
        assert!(n <= all.len(), "flip at {at}: more records than written");
        let mut refused = false;
        for (index, want) in all.iter().take(n).enumerate() {
            let Ok(bar) = reader.read_record(u64::try_from(index).unwrap()) else {
                refused = true;
                break;
            };
            assert_eq!(
                bar, *want,
                "flip at {at} (sidecar {in_sidecar}) served a different bar at {index}"
            );
        }
        if refused {
            tally.read_refused += 1;
        } else if n == all.len() {
            tally.served_intact += 1;
        } else {
            tally.served_rolled_back += 1;
            rolled_back_at.push(at);
        }
    }
    fs::write(&bars, &clean_bars).unwrap();
    fs::write(&crc_path, &clean_crc).unwrap();
    eprintln!("flip tally: {tally:?}; rolled back by flips at {rolled_back_at:?}");
    let total = u64::try_from(clean_bars.len() + clean_crc.len()).unwrap();
    assert_eq!(
        tally.open_refused + tally.read_refused + tally.served_intact + tally.served_rolled_back,
        total
    );
    // Every flip inside a committed record or the sidecar is caught.
    assert!(tally.read_refused >= 150 * 56);
}

// ===========================================================================
// 3. Measurement: lookup and read cost against the record count
// ===========================================================================

fn percentiles(mut samples: Vec<u128>) -> (u128, u128, u128) {
    samples.sort_unstable();
    let n = samples.len();
    (samples[n / 2], samples[(n * 99) / 100], samples[n - 1])
}

/// Builds a one-second month holding `n` records, every second from the open.
fn second_month(root: &Path, n: u64) -> BarFile {
    let mut file =
        BarFile::open_or_create(root, path_at(Timeframe::SECOND_1), SYMBOL).expect("create");
    let mut rng = Rng(31);
    let from = bounds().0;
    let mut i = 0u64;
    while i < n {
        let take = (n - i).min(20_000);
        let batch: Vec<Bar> = (i..i + take)
            .map(|k| sane_bar(&mut rng, from + i64::try_from(k).unwrap() * SECOND))
            .collect();
        file.append(&batch).expect("append");
        i += take;
    }
    file
}

/// Times `first_at_or_after` and `read_record` at 10^3..10^6 records.
///
/// Ignored by default: it writes 56 MB at the top size. Run with
/// `cargo test -p store --test attack_store -- --ignored --nocapture`.
/// It asserts every answer it times; the timings are printed, not asserted.
#[test]
#[ignore = "measurement: writes ~62 MB; run with --ignored --nocapture"]
fn lookup_and_read_cost_against_the_record_count() {
    const SAMPLES: u64 = 20_000;
    for exp in 3..=6u32 {
        let n = 10u64.pow(exp);
        let scratch = Scratch::new("bench");
        let file = second_month(scratch.root(), n);
        assert_eq!(file.records(), n);
        let from = bounds().0;
        let mut rng = Rng(37);
        let mut lookup = Vec::with_capacity(usize::try_from(SAMPLES).unwrap());
        let mut read = Vec::with_capacity(usize::try_from(SAMPLES).unwrap());
        for _ in 0..SAMPLES {
            let k = rng.below(n);
            let ts = from + i64::try_from(k).unwrap() * SECOND;
            let t = Instant::now();
            let got = file.first_at_or_after(ts).expect("lookup");
            lookup.push(t.elapsed().as_nanos());
            assert_eq!(got, k);
            let k2 = rng.below(n);
            let t = Instant::now();
            let bar = file.read_record(k2).expect("read");
            read.push(t.elapsed().as_nanos());
            assert_eq!(bar.ts_micros, from + i64::try_from(k2).unwrap() * SECOND);
        }
        let (lp50, lp99, lmax) = percentiles(lookup);
        let (rp50, rp99, rmax) = percentiles(read);
        eprintln!(
            "n={n:>8}  first_at_or_after p50={lp50}ns p99={lp99}ns max={lmax}ns  |  read_record p50={rp50}ns p99={rp99}ns max={rmax}ns"
        );
    }
}

#[test]
fn the_audited_door_refuses_every_flip_the_plain_reader_rolls_back_and_serves_no_different_bar() {
    let scratch = Scratch::new("audit-flip");
    let mut file = open(scratch.root()).expect("create");
    let mut rng = Rng(41);
    let first: Vec<Bar> = (0..80).map(|m| sane_bar(&mut rng, minute(m))).collect();
    let second: Vec<Bar> = (80..150).map(|m| sane_bar(&mut rng, minute(m))).collect();
    file.append(&first).expect("first commit");
    file.append(&second).expect("second commit");
    drop(file);
    let all: Vec<Bar> = first.iter().chain(&second).copied().collect();
    let bars = bars_path().to_path_buf(scratch.root());
    let crc_path = bars_path()
        .with_file(FileKind::Checksums)
        .to_path_buf(scratch.root());
    let clean_bars = fs::read(&bars).unwrap();
    let clean_crc = fs::read(&crc_path).unwrap();
    let limit = u64::MAX;
    // The clean month audits and serves every bar.
    {
        let audited = BarFile::open_existing_audited(scratch.root(), bars_path(), SYMBOL, limit)
            .expect("a clean month audits");
        for (index, want) in all.iter().enumerate() {
            assert_eq!(
                audited
                    .read_record(u64::try_from(index).unwrap())
                    .expect("row"),
                *want
            );
        }
        assert!(audited.read_record(150).is_err());
        assert!(audited.read_record(u64::MAX).is_err());
    }
    // A byte ceiling one short of the exact extent is refused.
    let exact = u64::try_from(clean_bars.len() + clean_crc.len()).unwrap();
    assert!(BarFile::open_existing_audited(scratch.root(), bars_path(), SYMBOL, exact).is_ok());
    assert!(
        BarFile::open_existing_audited(scratch.root(), bars_path(), SYMBOL, exact - 1).is_err()
    );
    let (mut refused, mut intact) = (0u64, 0u64);
    // Every byte of both header slots, every committed record byte, every
    // sidecar byte; the reserved header span is sampled.
    let targets: Vec<(bool, usize)> = (0..128)
        .chain((128..16_384).step_by(97))
        .chain(16_384..16_448)
        .chain((16_448..32_768).step_by(97))
        .chain(32_768..clean_bars.len())
        .map(|at| (false, at))
        .chain((0..clean_crc.len()).map(|at| (true, at)))
        .collect();
    for (in_sidecar, at) in targets {
        let mut b = clean_bars.clone();
        let mut c = clean_crc.clone();
        if in_sidecar {
            c[at] ^= 0x10;
        } else {
            b[at] ^= 0x10;
        }
        fs::write(&bars, &b).unwrap();
        fs::write(&crc_path, &c).unwrap();
        let Ok(audited) =
            BarFile::open_existing_audited(scratch.root(), bars_path(), SYMBOL, limit)
        else {
            refused += 1;
            continue;
        };
        // An audit that passed answers for the WHOLE committed month.
        assert_eq!(
            audited.evidence().header().n_valid,
            150,
            "flip at {at} (sidecar {in_sidecar}): the audit passed a rolled-back month"
        );
        for (index, want) in all.iter().enumerate() {
            let got = audited
                .read_record(u64::try_from(index).unwrap())
                .unwrap_or_else(|why| panic!("flip at {at}: audited read refused {why}"));
            assert_eq!(
                got, *want,
                "flip at {at}: audited door served a different bar"
            );
        }
        intact += 1;
    }
    fs::write(&bars, &clean_bars).unwrap();
    fs::write(&crc_path, &clean_crc).unwrap();
    eprintln!("audited flips: refused {refused}, harmless {intact}");
    // Every committed-record and sidecar flip and every live-slot flip refuses.
    assert!(refused >= 150 * 56 + 12 + 64);
}

#[test]
fn a_month_truncated_under_its_writer_cannot_be_appended_into_a_hole_that_reads_as_bars() {
    for held in [1i64, 72, 73, 74, 146] {
        let scratch = Scratch::new("hole");
        let mut file = open(scratch.root()).expect("create");
        let mut rng = Rng(43);
        let batch: Vec<Bar> = (0..held).map(|m| sane_bar(&mut rng, minute(m))).collect();
        file.append(&batch).expect("seed");
        // Another process ignores the advisory lock and cuts the records off.
        let path = bars_path().to_path_buf(scratch.root());
        fs::OpenOptions::new()
            .write(true)
            .open(&path)
            .unwrap()
            .set_len(HEADER_LEN)
            .unwrap();
        let next = sane_bar(&mut rng, minute(held));
        let appended = file.append(&[next]);
        // Either the writer refuses, or every read of the month afterwards
        // refuses rather than serving the zero-filled hole as bars.
        if appended.is_ok() {
            drop(file);
            let reader =
                BarFile::open_existing(scratch.root(), bars_path(), SYMBOL).expect("reader");
            for index in 0..u64::try_from(held).unwrap() {
                let got = reader.read_record(index);
                assert!(
                    got.is_err(),
                    "held {held}: index {index} served {got:?} out of a hole"
                );
            }
        }
    }
}

/// Rewrites one little-endian `i64` field of the newest committed slot and
/// re-seals the slot's checksum, so the slot decodes and validates and only
/// its CONTENT is wrong. `generation` picks the slot.
fn forge_slot_i64(root: &Path, generation: u64, offset: usize, value: i64) {
    let path = bars_path().to_path_buf(root);
    let mut bytes = fs::read(&path).expect("bars");
    let at = usize::try_from(generation % 2).unwrap() * 16_384;
    let s = &mut bytes[at..at + 64];
    assert_eq!(&s[..7], b"BRUTEXB");
    assert_eq!(
        u64::from_le_bytes(s[16..24].try_into().unwrap()),
        generation
    );
    s[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    let mut covered = s[..56].to_vec();
    covered.extend_from_slice(&s[60..64]);
    let crc = crc32c(&covered).to_le_bytes();
    s[56..60].copy_from_slice(&crc);
    fs::write(&path, bytes).expect("forge");
}

/// The header's `last_ts_micros` is what `append` trusts to decide that a
/// batch follows the month. A slot whose checksum is good and whose range
/// disagrees with the sealed records it counts must not steer a write that
/// leaves the month out of order.
#[test]
fn a_header_whose_last_stamp_disagrees_with_its_last_record_cannot_steer_an_append() {
    for forged in [
        minute(0),
        minute(5),
        minute(9) - 1,
        bounds().0 - MINUTE,
        i64::MIN,
    ] {
        let scratch = Scratch::new("forged-last");
        let mut file = open(scratch.root()).expect("create");
        let mut rng = Rng(47);
        let held: Vec<Bar> = (0..10).map(|m| sane_bar(&mut rng, minute(m))).collect();
        file.append(&held).expect("seed");
        drop(file);
        forge_slot_i64(scratch.root(), 1, 40, forged);
        let before = digest(scratch.root());
        // A writer may refuse to open it, or refuse the append; it may not
        // commit a bar stamped at or before a record it already holds.
        if let Ok(mut file) = open(scratch.root()) {
            // The first grid minute past the forged stamp, and never before
            // minute 6: always at or behind the held minute 9.
            let past = forged
                .saturating_sub(bounds().0)
                .div_euclid(MINUTE)
                .saturating_add(1);
            let intruder = sane_bar(&mut rng, minute(past.max(6)));
            assert!(intruder.ts_micros <= minute(9));
            let got = file.append(&[intruder]);
            assert!(
                matches!(
                    got,
                    Err(StoreError::Format {
                        source: FormatError::LastStampDisagrees { header, record },
                        ..
                    }) if header == forged && record == minute(9)
                ),
                "forged last_ts {forged}: append of {intruder:?} behind held records: {got:?}"
            );
            assert_eq!(file.records(), 10);
        }
        assert_eq!(
            digest(scratch.root()),
            before,
            "forged last_ts {forged}: month changed"
        );
    }
}

/// The same for the far end: a slot claiming a LATER last stamp than its last
/// record makes every honest extension look like an overlap. That is loud
/// rather than silent, but its diagnosis must not be a write.
#[test]
fn a_header_whose_last_stamp_runs_past_its_last_record_writes_nothing() {
    let scratch = Scratch::new("forged-later");
    let mut file = open(scratch.root()).expect("create");
    let mut rng = Rng(53);
    let held: Vec<Bar> = (0..10).map(|m| sane_bar(&mut rng, minute(m))).collect();
    file.append(&held).expect("seed");
    drop(file);
    forge_slot_i64(scratch.root(), 1, 40, minute(500));
    let before = digest(scratch.root());
    if let Ok(mut file) = open(scratch.root()) {
        let next = sane_bar(&mut rng, minute(10));
        assert!(file.append(&[next]).is_err());
    }
    assert_eq!(digest(scratch.root()), before);
}

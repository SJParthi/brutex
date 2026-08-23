//! Every run, on disk, keyed by the identity that names it.
//!
//! # What this closes
//!
//! A sweep printed its answer to stdout and the answer was gone. Nothing in the
//! workspace could say what had been run, when, on which span, or what it found
//! — so "which threshold did best on NIFTY 15-minute" was a question that could
//! only be answered by running everything again. `CLAUDE.md` §3 rule 3 requires
//! every computation to record its identity; it was computed and printed, and
//! printed is not recorded.
//!
//! # The shape, and why it is not a database
//!
//! One append-only file of FIXED-STRIDE records. §4 bans a query planner and a
//! dynamic schema, and the reason is the same for both: *the path is the index*,
//! and *a new field is a new file version at its own stride*. This follows
//! `store::format`'s layout exactly — a magic, a version, then records at a
//! constant stride, so the address of record *i* is `HEADER + i · STRIDE`: an
//! add and a multiply.
//!
//! | Operation | Cost | How |
//! |---|---|---|
//! | append | **O(1)** | seek to end, one write of one stride |
//! | read record *i* | **O(1)** | seek to `HEADER + i·STRIDE`, one read |
//! | count | **O(1)** | `(file_len - HEADER) / STRIDE`, no walk |
//! | duplicate check | **O(1)** amortised | a `HashSet` of identities, built once at open |
//!
//! The duplicate check is the only one that is not O(1) *outright*, and the cost
//! is stated rather than hidden: building the set is one pass over the file at
//! open. That pass is O(runs), runs is a few thousand after years of operating,
//! and it happens once per process — never per bar, never per candidate, so it
//! is not on the path §3 rule 4 governs.
//!
//! # Append-only, because §3 rule 8 says so
//!
//! No record is ever rewritten and no field is reused. A rerun of an identity
//! that is already present is REFUSED rather than overwritten: §3 rule 5 says
//! same inputs give same outputs, so a second run of one identity has nothing
//! new to say, and letting it overwrite would destroy the first run's timestamp
//! for no gain.

use std::fs::{File, OpenOptions};
use std::io::{Read as _, Seek as _, SeekFrom, Write as _};
use std::path::{Path, PathBuf};

/// Why a result could not be written or read, in the operator's words.
pub type Refusal = String;

/// `BRUTEXRS`, so a file that is not this one is refused before it is parsed.
const MAGIC: [u8; 8] = *b"BRUTEXRS";

/// Version one. A new field is a new version at its own stride, never a
/// widened record — `CLAUDE.md` §4 and §3 rule 8 together.
const VERSION: u32 = 1;

/// Magic, version, and four bytes reserved so the header is a round sixteen.
const HEADER: u64 = 16;

/// [`HEADER`] as a `usize`, for the header array. Same reason as
/// [`STRIDE_BYTES`]: a cast would be a narrowing on a 32-bit target.
const HEADER_BYTES: usize = 16;

const _: () = assert!(HEADER_BYTES as u64 == HEADER);

/// Bytes per record. Every field below is fixed-width; nothing here is a string
/// of unknown length, because a variable record has no stride and therefore no
/// O(1) address.
///
/// # 205, and it was 176 for about ten minutes
///
/// This constant was declared before the fields were counted, and
/// `the_stride_is_exactly_what_the_writer_writes` failed on its first run with
/// `left: 176, right: 205`. That is the entire reason the test exists: had the
/// writer emitted 205 bytes while every address was computed from 176, record 1
/// would have been read starting 29 bytes into record 0 -- and it would still
/// PARSE, because every byte pattern is a legal value of its type. The file
/// would have been silently and completely wrong, with no error anywhere.
///
/// Not padded to a round number: §4 says a new field is a new file version at
/// its own stride, so reserved space would be space for a change the format
/// does not permit.
pub const STRIDE: u64 = 205;

/// [`STRIDE`] as a `usize`, for the record arrays.
///
/// Declared rather than cast: `STRIDE as usize` is a narrowing on a 32-bit
/// target and clippy is right to refuse it. Two constants that must agree, and
/// a `const` assertion that they do -- which a cast could not give.
pub const STRIDE_BYTES: usize = 205;

const _: () = assert!(STRIDE_BYTES as u64 == STRIDE);

/// One completed run, as it is stored.
///
/// Sixteen-byte text fields hold the instrument and the rung. They are FIXED and
/// zero-padded rather than length-prefixed: a length prefix makes the record
/// variable, and a variable record cannot be addressed by multiplication.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Record {
    /// `blake3(mask ‖ direction ‖ instrument ‖ timeframe ‖ params ‖ digest ‖
    /// vocab ‖ commit ‖ feed)` — the nine terms §3 rule 3 names.
    pub identity: [u8; 32],
    /// When the run finished, microseconds since the epoch.
    pub finished_micros: i64,
    /// The feed's directory word, zero-padded.
    pub feed: [u8; 16],
    /// The instrument, zero-padded.
    pub underlying: [u8; 16],
    /// The SIGNAL rung, zero-padded. Execution is always one-minute.
    pub timeframe: [u8; 16],
    /// First month of the span.
    pub from_year: u16,
    /// First month of the span.
    pub from_month: u8,
    /// Last month of the span.
    pub to_year: u16,
    /// Last month of the span.
    pub to_month: u8,
    /// Months the span asked for.
    pub months_asked: u32,
    /// Months the store actually held. Below `months_asked` means a HOLE, and
    /// every figure in this record is over a shorter sample.
    pub months_found: u32,
    /// Signal bars swept.
    pub bars: u64,
    /// The support threshold applied.
    pub min_hits: u64,
    /// Combinations the ladder produced.
    pub combinations: u64,
    /// Deepest level reached.
    pub depth: u32,
    /// `1` when a budget stopped the walk short, so `depth` is PARTIAL and
    /// `combinations` covers less of the ladder than it appears to.
    pub halted: u8,
    /// Round trips the chosen combination took.
    pub trades: u64,
    /// Its total under worst-case fills, in paisa. **The figure selection ranks
    /// on**, so the one to compare runs by.
    pub pessimistic: i64,
    /// Its total under best-case fills, in paisa.
    pub optimistic: i64,
    /// Worst single round trip, in paisa.
    pub worst_trade: i64,
    /// Worst peak-to-trough, in paisa.
    pub max_drawdown: i64,
    /// Winners' adverse excursion, ppm — the tightest stop that would not have
    /// killed a winner.
    pub winner_mae: i64,
    /// Winners' favourable excursion, ppm.
    pub winner_mfe: i64,
    /// Every trade's adverse excursion, ppm.
    pub all_mae: i64,
    /// Chosen exit rungs, `-1` for "no rung": stop, target, TSL, TTP arm, TTP
    /// trail. Five axes, so five fields — a packed integer would be a schema
    /// inside a schema.
    pub exit_rungs: [i16; 5],
}

impl Record {
    /// The record as its exact `STRIDE` bytes, little-endian throughout.
    #[must_use]
    #[expect(
        clippy::indexing_slicing,
        reason = "every index below is a constant offset into a fixed-size \
                  array whose length is asserted against STRIDE by \
                  `the_stride_is_exactly_what_the_writer_writes`"
    )]
    pub fn to_bytes(&self) -> [u8; STRIDE_BYTES] {
        let mut out = [0_u8; STRIDE_BYTES];
        let mut at = 0_usize;
        let mut put = |bytes: &[u8], at: &mut usize| {
            out[*at..*at + bytes.len()].copy_from_slice(bytes);
            *at += bytes.len();
        };
        put(&self.identity, &mut at);
        put(&self.finished_micros.to_le_bytes(), &mut at);
        put(&self.feed, &mut at);
        put(&self.underlying, &mut at);
        put(&self.timeframe, &mut at);
        put(&self.from_year.to_le_bytes(), &mut at);
        put(&[self.from_month], &mut at);
        put(&self.to_year.to_le_bytes(), &mut at);
        put(&[self.to_month], &mut at);
        put(&self.months_asked.to_le_bytes(), &mut at);
        put(&self.months_found.to_le_bytes(), &mut at);
        put(&self.bars.to_le_bytes(), &mut at);
        put(&self.min_hits.to_le_bytes(), &mut at);
        put(&self.combinations.to_le_bytes(), &mut at);
        put(&self.depth.to_le_bytes(), &mut at);
        put(&[self.halted], &mut at);
        put(&self.trades.to_le_bytes(), &mut at);
        put(&self.pessimistic.to_le_bytes(), &mut at);
        put(&self.optimistic.to_le_bytes(), &mut at);
        put(&self.worst_trade.to_le_bytes(), &mut at);
        put(&self.max_drawdown.to_le_bytes(), &mut at);
        put(&self.winner_mae.to_le_bytes(), &mut at);
        put(&self.winner_mfe.to_le_bytes(), &mut at);
        put(&self.all_mae.to_le_bytes(), &mut at);
        for rung in self.exit_rungs {
            put(&rung.to_le_bytes(), &mut at);
        }
        out
    }

    /// The record back from its exact `STRIDE` bytes.
    ///
    /// Infallible by construction: every field is fixed-width and every byte
    /// pattern is a legal value of its type. A record whose CONTENT is
    /// nonsensical is a separate question and is not answered by pretending the
    /// parse can fail.
    #[must_use]
    #[expect(
        clippy::indexing_slicing,
        reason = "the same constant offsets `to_bytes` writes, over an array of \
                  the same fixed length"
    )]
    pub fn from_bytes(raw: &[u8; STRIDE_BYTES]) -> Self {
        let mut at = 0_usize;
        let take = |n: usize, at: &mut usize| {
            let slice = &raw[*at..*at + n];
            *at += n;
            slice
        };
        let mut id = [0_u8; 32];
        id.copy_from_slice(take(32, &mut at));
        let finished_micros = i64::from_le_bytes(take(8, &mut at).try_into().unwrap_or([0; 8]));
        let mut feed = [0_u8; 16];
        feed.copy_from_slice(take(16, &mut at));
        let mut underlying = [0_u8; 16];
        underlying.copy_from_slice(take(16, &mut at));
        let mut timeframe = [0_u8; 16];
        timeframe.copy_from_slice(take(16, &mut at));
        let from_year = u16::from_le_bytes(take(2, &mut at).try_into().unwrap_or([0; 2]));
        let from_month = take(1, &mut at).first().copied().unwrap_or(0);
        let to_year = u16::from_le_bytes(take(2, &mut at).try_into().unwrap_or([0; 2]));
        let to_month = take(1, &mut at).first().copied().unwrap_or(0);
        let months_asked = u32::from_le_bytes(take(4, &mut at).try_into().unwrap_or([0; 4]));
        let months_found = u32::from_le_bytes(take(4, &mut at).try_into().unwrap_or([0; 4]));
        let bars = u64::from_le_bytes(take(8, &mut at).try_into().unwrap_or([0; 8]));
        let min_hits = u64::from_le_bytes(take(8, &mut at).try_into().unwrap_or([0; 8]));
        let combinations = u64::from_le_bytes(take(8, &mut at).try_into().unwrap_or([0; 8]));
        let depth = u32::from_le_bytes(take(4, &mut at).try_into().unwrap_or([0; 4]));
        let halted = take(1, &mut at).first().copied().unwrap_or(0);
        let trades = u64::from_le_bytes(take(8, &mut at).try_into().unwrap_or([0; 8]));
        let pessimistic = i64::from_le_bytes(take(8, &mut at).try_into().unwrap_or([0; 8]));
        let optimistic = i64::from_le_bytes(take(8, &mut at).try_into().unwrap_or([0; 8]));
        let worst_trade = i64::from_le_bytes(take(8, &mut at).try_into().unwrap_or([0; 8]));
        let max_drawdown = i64::from_le_bytes(take(8, &mut at).try_into().unwrap_or([0; 8]));
        let winner_mae = i64::from_le_bytes(take(8, &mut at).try_into().unwrap_or([0; 8]));
        let mfe_winners = i64::from_le_bytes(take(8, &mut at).try_into().unwrap_or([0; 8]));
        let all_mae = i64::from_le_bytes(take(8, &mut at).try_into().unwrap_or([0; 8]));
        let mut exit_rungs = [0_i16; 5];
        for slot in &mut exit_rungs {
            *slot = i16::from_le_bytes(take(2, &mut at).try_into().unwrap_or([0; 2]));
        }
        Self {
            identity: id,
            finished_micros,
            feed,
            underlying,
            timeframe,
            from_year,
            from_month,
            to_year,
            to_month,
            months_asked,
            months_found,
            bars,
            min_hits,
            combinations,
            depth,
            halted,
            trades,
            pessimistic,
            optimistic,
            worst_trade,
            max_drawdown,
            winner_mae,
            winner_mfe: mfe_winners,
            all_mae,
            exit_rungs,
        }
    }

    /// The identity as lowercase hex, which is how a run is named everywhere
    /// else in this workspace.
    #[must_use]
    pub fn identity_hex(&self) -> String {
        let mut s = String::with_capacity(64);
        for byte in self.identity {
            let _ = write!(s, "{byte:02x}");
        }
        s
    }
}

use core::fmt::Write as _;

/// A fixed-width text field, truncated rather than refused.
///
/// Truncation is safe here and nowhere else: these fields are for a HUMAN
/// reading a listing, and the identity is what actually names the run. A refusal
/// would throw away a completed sweep because an instrument name was long.
#[must_use]
pub fn field(text: &str) -> [u8; 16] {
    let mut out = [0_u8; 16];
    for (slot, byte) in out.iter_mut().zip(text.bytes()) {
        *slot = byte;
    }
    out
}

/// A fixed-width text field, back to a string with its padding removed.
#[must_use]
pub fn read_field(raw: &[u8; 16]) -> String {
    let end = raw.iter().position(|&b| b == 0).unwrap_or(raw.len());
    String::from_utf8_lossy(raw.get(..end).unwrap_or(&[])).into_owned()
}

/// The append-only file of every run this store has recorded.
///
/// `Debug` shows the handle and the identity count, not the identities: a
/// listing of thousands of digests in a panic message helps nobody.
#[derive(Debug)]
pub struct Results {
    file: File,
    seen: std::collections::HashSet<[u8; 32]>,
}

impl Results {
    /// Opens, or creates, the results file beneath `root`.
    ///
    /// # Errors
    ///
    /// An unwritable directory, a file whose magic is not this format's, or a
    /// version this build does not know. Each refuses rather than being
    /// repaired: a file that is not this one must not be appended to.
    pub fn open(root: &Path) -> Result<Self, Refusal> {
        let dir = root.join("results");
        std::fs::create_dir_all(&dir)
            .map_err(|why| format!("the results directory could not be made: {why}"))?;
        let path = Self::path(root);
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)
            .map_err(|why| format!("{} could not be opened: {why}", path.display()))?;

        let len = file
            .metadata()
            .map_err(|why| format!("the results file could not be measured: {why}"))?
            .len();
        if len == 0 {
            let mut header = [0_u8; HEADER_BYTES];
            header
                .get_mut(..8)
                .ok_or_else(|| "the header is shorter than its magic".to_owned())?
                .copy_from_slice(&MAGIC);
            header
                .get_mut(8..12)
                .ok_or_else(|| "the header is shorter than its version".to_owned())?
                .copy_from_slice(&VERSION.to_le_bytes());
            file.write_all(&header)
                .map_err(|why| format!("the header could not be written: {why}"))?;
        } else {
            let mut header = [0_u8; HEADER_BYTES];
            file.seek(SeekFrom::Start(0))
                .and_then(|_| file.read_exact(&mut header))
                .map_err(|why| format!("the header could not be read: {why}"))?;
            if header.get(..8) != Some(&MAGIC) {
                return Err(format!(
                    "{} is not a brutex results file: its first eight bytes are \
                     not `BRUTEXRS`. Nothing was written.",
                    path.display()
                ));
            }
            let version = u32::from_le_bytes(
                header
                    .get(8..12)
                    .and_then(|s| s.try_into().ok())
                    .unwrap_or([0; 4]),
            );
            if version != VERSION {
                return Err(format!(
                    "{} is version {version} and this build writes version \
                     {VERSION}. A new field is a new file version at its own \
                     stride, never a widened record, so this file is not \
                     appended to.",
                    path.display()
                ));
            }
            // A TRAILING PART-RECORD IS A REFUSAL, NOT A ROUNDING.
            //
            // # The failure this closes, reproduced before it was written
            //
            // `len()` is `(file_len - HEADER) / STRIDE`, and integer division
            // DISCARDS the remainder. A machine that lost power partway through
            // an append left a part-record that no reader ever mentioned:
            // measured on a copy of a real ledger, appending 100 bytes and
            // listing it printed the same fourteen rows, said nothing, and
            // exited 0.
            //
            // Silence was only half of it. `append` seeks `SeekFrom::End(0)`,
            // so the NEXT record lands after the orphan bytes and every record
            // from then on is offset by however many they were — while `read`
            // still seeks `HEADER + index * STRIDE`. The same measurement showed
            // what that renders as: a row with `18,446,744,073,709,551,615`
            // combinations, no vendor, no rung, months `4294901760/0`, and a
            // total of -₹0.01. It was counted in the row total and it was
            // printed as data.
            //
            // Whether such a row can also WIN `BEST COMPLETE RUN` is decided by
            // whichever byte lands on `halted`. In that run it happened to be
            // non-zero and the row was skipped — luck, not design.
            //
            // Refused rather than healed. Truncating the orphan would be a
            // silent repair of a file whose history §3 rule 8 protects, and §4
            // bans a fallback that hides a failure. The count of intact records
            // is named so an operator can see exactly what survived.
            let payload = len.saturating_sub(HEADER);
            let orphan = payload % STRIDE;
            if orphan != 0 {
                return Err(format!(
                    "{} ends with {orphan} bytes that are not a whole record: \
                     {} complete records occupy {} bytes after the {HEADER}-byte \
                     header, and the file is {len}. A write was interrupted. \
                     Nothing here is repaired automatically — the intact records \
                     are readable and the orphan bytes are not, and truncating \
                     them is a decision about history that belongs to you.",
                    path.display(),
                    payload / STRIDE,
                    payload - orphan,
                ));
            }
        }

        // ONE PASS, ONCE, AT OPEN. Stated rather than hidden: this is O(runs)
        // and every other operation on this type is O(1). It is not on the
        // per-bar or per-candidate path §3 rule 4 governs.
        let mut seen = std::collections::HashSet::new();
        let mut at = HEADER;
        while at + STRIDE <= len {
            let mut raw = [0_u8; STRIDE_BYTES];
            file.seek(SeekFrom::Start(at))
                .and_then(|_| file.read_exact(&mut raw))
                .map_err(|why| format!("record at byte {at} could not be read: {why}"))?;
            seen.insert(Record::from_bytes(&raw).identity);
            at = at.saturating_add(STRIDE);
        }
        Ok(Self { file, seen })
    }

    /// Where the file lives: `<store>/results/runs.bin`.
    #[must_use]
    pub fn path(root: &Path) -> PathBuf {
        root.join("results").join("runs.bin")
    }

    /// How many runs are recorded. **O(1)** — a division, not a walk.
    ///
    /// # Errors
    ///
    /// An unreadable file handle.
    pub fn len(&self) -> Result<u64, Refusal> {
        let len = self
            .file
            .metadata()
            .map_err(|why| format!("the results file could not be measured: {why}"))?
            .len();
        Ok(len.saturating_sub(HEADER) / STRIDE)
    }

    /// Whether nothing has been recorded yet.
    ///
    /// # Errors
    ///
    /// Every error [`Self::len`] returns.
    pub fn is_empty(&self) -> Result<bool, Refusal> {
        Ok(self.len()? == 0)
    }

    /// Whether this identity is already recorded. **O(1).**
    #[must_use]
    pub fn holds(&self, identity: &[u8; 32]) -> bool {
        self.seen.contains(identity)
    }

    /// Appends one run. **O(1)** — a seek to the end and one write.
    ///
    /// # Errors
    ///
    /// A duplicate identity, or an unwritable file. A duplicate REFUSES rather
    /// than overwriting: §3 rule 5 makes a rerun byte-identical, so the second
    /// run has nothing to add, and overwriting would destroy the first one's
    /// timestamp for no gain.
    pub fn append(&mut self, record: &Record) -> Result<u64, Refusal> {
        if self.holds(&record.identity) {
            return Err(format!(
                "run {} is already recorded. Same inputs give same outputs \
                 (§3 rule 5), so this run has nothing new to add and the \
                 original's timestamp is kept.",
                record.identity_hex()
            ));
        }
        let at = self
            .file
            .seek(SeekFrom::End(0))
            .map_err(|why| format!("the results file could not be extended: {why}"))?;
        self.file
            .write_all(&record.to_bytes())
            .map_err(|why| format!("the record could not be written: {why}"))?;
        self.file
            .flush()
            .map_err(|why| format!("the record could not be flushed: {why}"))?;
        self.seen.insert(record.identity);
        Ok(at.saturating_sub(HEADER) / STRIDE)
    }

    /// Reads record `index`. **O(1)** — `HEADER + index · STRIDE`, one seek.
    ///
    /// # Errors
    ///
    /// An index past the end, or an unreadable file.
    pub fn read(&mut self, index: u64) -> Result<Record, Refusal> {
        let count = self.len()?;
        if index >= count {
            return Err(format!(
                "record {index} does not exist: this file holds {count}."
            ));
        }
        let at = HEADER.saturating_add(index.saturating_mul(STRIDE));
        let mut raw = [0_u8; STRIDE_BYTES];
        self.file
            .seek(SeekFrom::Start(at))
            .and_then(|_| self.file.read_exact(&mut raw))
            .map_err(|why| format!("record {index} could not be read: {why}"))?;
        Ok(Record::from_bytes(&raw))
    }
}

#[cfg(test)]
#[expect(
    clippy::expect_used,
    reason = "the exception every test module in this workspace takes: a test \
              that cannot panic cannot fail. `indexing_slicing` is NOT listed: \
              nothing here indexes, and an unfulfilled expectation is itself a \
              warning"
)]
mod tests {
    use super::{HEADER, HEADER_BYTES, Record, Results, STRIDE, field, read_field};

    fn root(tag: &str) -> std::path::PathBuf {
        let p = std::env::temp_dir().join(format!("brutex-results-{tag}"));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).expect("a temp root");
        p
    }

    fn record(n: u8) -> Record {
        Record {
            identity: [n; 32],
            finished_micros: 1_785_727_500_000_000 + i64::from(n),
            feed: field("zerodha"),
            underlying: field("NIFTY"),
            timeframe: field("15min"),
            from_year: 2019,
            from_month: 12,
            to_year: 2026,
            to_month: 8,
            months_asked: 81,
            months_found: 81,
            bars: 43_875,
            min_hits: 200,
            combinations: 54_895_691,
            depth: 11,
            halted: 1,
            trades: 412,
            pessimistic: -987_654_321,
            optimistic: 123_456_789,
            worst_trade: -87_654_321,
            max_drawdown: -76_543_210,
            winner_mae: 2_291,
            winner_mfe: 8_876,
            all_mae: 2_295,
            exit_rungs: [2, 3, -1, 1, 0],
        }
    }

    /// THE STRIDE IS WHAT THE WRITER WRITES, and nothing else may claim it.
    ///
    /// `STRIDE` is the constant every address is computed from: the address of
    /// record *i* is `HEADER + i·STRIDE`. If the writer emitted a different
    /// number of bytes, every record after the first would be read from the
    /// wrong offset — and it would still PARSE, because every byte pattern is a
    /// legal value of its type. The file would be silently, entirely wrong.
    #[test]
    fn the_stride_is_exactly_what_the_writer_writes() {
        // Field by field, so a change to the layout has to change this sum
        // deliberately rather than by a rebuild.
        const EXPECTED: u64 = 32   // identity
            + 8                    // finished_micros
            + 16 + 16 + 16         // feed, underlying, timeframe
            + 2 + 1 + 2 + 1        // from/to year and month
            + 4 + 4                // months asked, found
            + 8 + 8 + 8            // bars, min_hits, combinations
            + 4 + 1                // depth, halted
            + 8                    // trades
            + 8 + 8                // pessimistic, optimistic
            + 8 + 8                // worst_trade, max_drawdown
            + 8 + 8 + 8            // winner_mae, winner_mfe, all_mae
            + 2 * 5; // exit_rungs
        assert_eq!(
            STRIDE, EXPECTED,
            "the declared stride must be the sum of the fields"
        );
        assert_eq!(
            record(1).to_bytes().len() as u64,
            STRIDE,
            "and the writer must emit exactly that many bytes"
        );
    }

    #[test]
    fn every_field_survives_the_round_trip() {
        // Twenty-four fields, asserted one at a time. A `assert_eq!(a, b)` on
        // the whole struct would pass if two fields were swapped in BOTH
        // directions, which is exactly the bug a hand-written codec produces.
        let want = record(7);
        let got = Record::from_bytes(&want.to_bytes());
        assert_eq!(got.identity, want.identity, "identity");
        assert_eq!(got.finished_micros, want.finished_micros, "finished_micros");
        assert_eq!(read_field(&got.feed), "zerodha", "feed");
        assert_eq!(read_field(&got.underlying), "NIFTY", "underlying");
        assert_eq!(read_field(&got.timeframe), "15min", "timeframe");
        assert_eq!(got.from_year, 2019, "from_year");
        assert_eq!(got.from_month, 12, "from_month");
        assert_eq!(got.to_year, 2026, "to_year");
        assert_eq!(got.to_month, 8, "to_month");
        assert_eq!(got.months_asked, 81, "months_asked");
        assert_eq!(got.months_found, 81, "months_found");
        assert_eq!(got.bars, 43_875, "bars");
        assert_eq!(got.min_hits, 200, "min_hits");
        assert_eq!(got.combinations, 54_895_691, "combinations");
        assert_eq!(got.depth, 11, "depth");
        assert_eq!(got.halted, 1, "halted");
        assert_eq!(got.trades, 412, "trades");
        assert_eq!(got.pessimistic, -987_654_321, "pessimistic");
        assert_eq!(got.optimistic, 123_456_789, "optimistic");
        assert_eq!(got.worst_trade, -87_654_321, "worst_trade");
        assert_eq!(got.max_drawdown, -76_543_210, "max_drawdown");
        assert_eq!(got.winner_mae, 2_291, "winner_mae");
        assert_eq!(got.winner_mfe, 8_876, "winner_mfe");
        assert_eq!(got.all_mae, 2_295, "all_mae");
        assert_eq!(got.exit_rungs, [2, 3, -1, 1, 0], "exit_rungs");
        assert_eq!(got, want, "and the whole record agrees");
    }

    #[test]
    fn a_negative_exit_rung_survives_as_no_rung() {
        // `-1` means "no rung on this axis", and it is the one value that would
        // be destroyed by storing the five axes unsigned. A no-stop variant
        // reading as stop rung 65,535 would be a strategy nobody ran.
        let mut want = record(3);
        want.exit_rungs = [-1, -1, -1, -1, -1];
        assert_eq!(Record::from_bytes(&want.to_bytes()).exit_rungs, [-1; 5]);
    }

    #[test]
    fn records_are_addressed_by_multiplication_and_read_back_in_order() {
        let r = root("addr");
        let mut store = Results::open(&r).expect("a fresh store opens");
        assert!(store.is_empty().expect("measurable"));
        for n in 0..5_u8 {
            let at = store.append(&record(n)).expect("appends");
            assert_eq!(at, u64::from(n), "each append returns its own index");
        }
        assert_eq!(store.len().expect("measurable"), 5);
        for n in 0..5_u8 {
            let got = store.read(u64::from(n)).expect("reads");
            assert_eq!(got.identity, [n; 32], "record {n} is at index {n}");
        }
        // AND THE FILE IS EXACTLY THE SIZE ITS ARITHMETIC CLAIMS. A stride that
        // disagreed with the writer would show up here as a length that is not
        // a whole number of records.
        let bytes = std::fs::metadata(Results::path(&r))
            .expect("the file exists")
            .len();
        assert_eq!(bytes, HEADER + 5 * STRIDE);
    }

    #[test]
    fn a_reopened_store_appends_after_what_is_already_there() {
        // Append-only across process boundaries, which is the whole point: a run
        // recorded yesterday must still be there today, and today's must not
        // overwrite it.
        let r = root("reopen");
        {
            let mut store = Results::open(&r).expect("opens");
            store.append(&record(1)).expect("appends");
            store.append(&record(2)).expect("appends");
        }
        let mut store = Results::open(&r).expect("reopens");
        assert_eq!(store.len().expect("measurable"), 2, "nothing was lost");
        assert_eq!(
            store.append(&record(3)).expect("appends"),
            2,
            "and it appends after"
        );
        assert_eq!(
            store.read(0).expect("reads").identity,
            [1; 32],
            "the first is untouched"
        );
    }

    /// A WRITE CUT SHORT BY A DEAD MACHINE IS NAMED, NOT ROUNDED AWAY.
    ///
    /// # What this reproduces
    ///
    /// `len()` divides the payload by `STRIDE`, and integer division discards
    /// the remainder. Before this refusal existed, a ledger with a part-record
    /// on the end listed its intact rows, said nothing about the orphan, and
    /// exited 0 — so the operator's evidence that a run had been interrupted
    /// was a file size nobody looks at.
    ///
    /// The silence was the smaller half. `append` seeks to the END of the file,
    /// while `read` seeks `HEADER + index * STRIDE`, so one interrupted write
    /// puts every later record permanently out of phase with every later read.
    /// Reproduced on a copy of a real ledger, the first misaligned row rendered
    /// as `18,446,744,073,709,551,615` combinations with no vendor and no rung,
    /// and was counted in the row total as though it were a run.
    ///
    /// # Why every remainder is tried
    ///
    /// One orphan byte and `STRIDE - 1` orphan bytes are the two ends of the
    /// same defect, and a check written as `!= STRIDE` or `< STRIDE / 2` would
    /// pass one and fail the other. The loop is the cheapest way to say that the
    /// only acceptable remainder is zero.
    #[test]
    fn a_part_record_from_an_interrupted_write_is_refused_and_counted() {
        for orphan in [1_u64, 7, 100, STRIDE - 1] {
            let r = root(&format!("torn{orphan}"));
            {
                let mut store = Results::open(&r).expect("opens");
                store.append(&record(1)).expect("appends");
                store.append(&record(2)).expect("appends");
            }
            // The interrupted write itself: bytes that are not a whole record.
            let path = Results::path(&r);
            use std::io::Write as _;
            let mut f = std::fs::OpenOptions::new()
                .append(true)
                .open(&path)
                .expect("the ledger reopens for the torn write");
            f.write_all(&vec![0_u8; usize::try_from(orphan).expect("fits")])
                .expect("the partial write lands");
            drop(f);

            let why = Results::open(&r).expect_err("a torn ledger is refused");
            assert!(
                why.contains(&orphan.to_string()),
                "the refusal must name how many bytes are orphaned, or the \
                 operator cannot tell a one-byte tear from a near-whole one: \
                 {why}"
            );
            assert!(
                why.contains('2'),
                "and how many records survived, which is what makes it \
                 actionable rather than merely alarming: {why}"
            );
        }
    }

    /// The refusal is about the REMAINDER, so a whole number of records opens.
    ///
    /// Without this row the check above would pass just as well if `open`
    /// refused every non-empty ledger, which would be a far worse defect wearing
    /// the same test's approval.
    #[test]
    fn a_ledger_whose_length_is_a_whole_number_of_records_still_opens() {
        let r = root("whole");
        {
            let mut store = Results::open(&r).expect("opens");
            for i in 1..=3 {
                store.append(&record(i)).expect("appends");
            }
        }
        let store = Results::open(&r).expect("a ledger with no orphan bytes reopens");
        assert_eq!(store.len().expect("measurable"), 3);
    }

    #[test]
    fn the_same_identity_is_refused_rather_than_overwritten() {
        // §3 rule 5: same inputs, same outputs. A rerun has nothing new to say,
        // and overwriting would destroy the original's timestamp for no gain.
        let r = root("dup");
        let mut store = Results::open(&r).expect("opens");
        store.append(&record(9)).expect("the first is recorded");
        let why = store.append(&record(9)).expect_err("the second is refused");
        assert!(why.contains("already recorded"), "{why}");
        assert_eq!(
            store.len().expect("measurable"),
            1,
            "and nothing was appended"
        );
        // The duplicate check survives a reopen, because it is rebuilt from the
        // file rather than kept only in memory.
        drop(store);
        let mut store = Results::open(&r).expect("reopens");
        assert!(
            store.holds(&[9; 32]),
            "the identity is known after a reopen"
        );
        assert!(store.append(&record(9)).is_err(), "and still refused");
    }

    #[test]
    fn a_file_that_is_not_this_format_refuses_rather_than_being_appended_to() {
        let r = root("alien");
        std::fs::create_dir_all(r.join("results")).expect("the directory");
        std::fs::write(Results::path(&r), b"NOTBRUTEXand some more bytes here")
            .expect("an alien file");
        let why = Results::open(&r).expect_err("refuses");
        assert!(
            why.contains("not a brutex results file"),
            "the refusal must name what it found: {why}"
        );
    }

    #[test]
    fn a_version_this_build_does_not_write_refuses() {
        // §4: a new field is a new file version at its own stride. A build that
        // appended its own stride to another version's file would interleave two
        // layouts in one addressable space.
        let r = root("version");
        std::fs::create_dir_all(r.join("results")).expect("the directory");
        let mut header = [0_u8; HEADER_BYTES];
        header[..8].copy_from_slice(b"BRUTEXRS");
        header[8..12].copy_from_slice(&99_u32.to_le_bytes());
        std::fs::write(Results::path(&r), header).expect("a future file");
        let why = Results::open(&r).expect_err("refuses");
        assert!(why.contains("version 99"), "{why}");
    }

    #[test]
    fn an_index_past_the_end_is_refused_and_names_the_count() {
        let r = root("oob");
        let mut store = Results::open(&r).expect("opens");
        store.append(&record(1)).expect("appends");
        let why = store.read(7).expect_err("refuses");
        assert!(why.contains("this file holds 1"), "{why}");
    }

    #[test]
    fn a_text_field_longer_than_its_slot_is_truncated_and_never_bleeds() {
        // Truncation is safe HERE because the identity names the run and these
        // fields are for a human reading a listing. What must never happen is a
        // write past the slot, which would corrupt the next field.
        let long = field("an-instrument-name-far-longer-than-sixteen-bytes");
        assert_eq!(long.len(), 16, "the slot is exactly its declared width");
        assert_eq!(read_field(&long), "an-instrument-na");
    }
}

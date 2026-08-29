//! Every round trip a recorded run took, keyed by that run's identity.
//!
//! # Why this exists, and why it is here rather than in `crates/api`
//!
//! The backtest page has drawn a per-trade table since it was written — trade
//! number, entry and exit, price, net P&L, favourable and adverse excursion,
//! cumulative P&L, duration — and **every cell of it is a padlock**, each
//! carrying its own sentence: *"No trade number — no trade list is recorded."*
//! The display was built, the reader was built, and nothing ever wrote the file.
//!
//! `crates/api/src/trades.rs` held a store of exactly this shape — magic,
//! stride, `append`, `read_block`, a JSON serialiser — with **zero production
//! callers on either side**, and three defects that made it unusable where it
//! stood:
//!
//! * it lives in `api`, and the crate arrow runs `api -> cli` (`CLAUDE.md` §5).
//!   The trades exist inside `cli::audit_bars`, which cannot reach `api`;
//! * its block index was built as `HashMap::new()` on *both* open paths, so
//!   after any restart every lookup returned nothing — the module header claimed
//!   the opposite;
//! * its record carried no run identity, only a sequence number, so the index
//!   was **unreconstructible**: nothing on disk said which run a row belonged
//!   to.
//!
//! This module is modelled on [`crate::frontier`], which is the identity-keyed
//! detail file that already works end to end. Carrying the identity **on every
//! row** is what makes the index rebuildable by one pass at open, exactly as
//! `results::Results::open` rebuilds its duplicate set.
//!
//! # What is stored, and what is deliberately not
//!
//! These are the round trips of the **level-less walk** — `runner::trade::walk`,
//! the same `taken` the audit renders — not the chosen exit variant's. The walk
//! is what decides *when* a combination was in the market; the variant decides
//! what a stop would have done to it. Storing the walk keeps this file a record
//! of the run rather than of one cell of its grid, and it is the vector already
//! in hand at the recording site, so nothing is recomputed.
//!
//! Prices are not here. A `Trade` carries bar INDICES and the excursion in
//! paisa; turning an index into a price needs the bars, which the reader has and
//! this file should not duplicate — `CLAUDE.md` §5's argument against two copies
//! of one fact.
//!
//! # Cost
//!
//! | operation | cost | how |
//! |---|---|---|
//! | append a run's trades | O(rows) | one seek to the end, one write |
//! | find a run's trades | **O(1)** | one hash probe into the block index |
//! | read one row | **O(1)** | seek to `HEADER + index * STRIDE` |
//! | open | O(rows) | one pass to rebuild the index, once per process |
//!
//! The open-time pass is the only non-constant term and it is the same one
//! `results` and `frontier` pay, for the same reason: an index that is not on
//! disk must be rebuilt from what is.

use crate::results::Refusal;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

/// Names the format on disk. A file that does not begin with this is not one of
/// these and is refused rather than parsed.
const MAGIC: [u8; 8] = *b"BRUTEXTD";

/// The layout below. A reader that does not know this number refuses rather
/// than guessing at a stride it cannot verify.
const VERSION: u32 = 1;

/// Magic, version and a reserved word.
const HEADER: u64 = 16;
const HEADER_BYTES: usize = 16;
const _: () = assert!(HEADER_BYTES as u64 == HEADER);

/// One row. Fixed, so `index -> byte offset` is a multiply and the lookup is
/// O(1) — `CLAUDE.md` §3 rule 4.
const STRIDE: u64 = 88;
const STRIDE_BYTES: usize = 88;
const _: () = assert!(STRIDE_BYTES as u64 == STRIDE);

/// The last eight bytes of a row are a seal over the first eighty.
const SEAL_BYTES: usize = 8;
const PAYLOAD_BYTES: usize = STRIDE_BYTES - SEAL_BYTES;

/// THE PAYLOAD IS THE SUM OF ITS FIELDS, AND THIS ASSERTION IS NOT DECORATION.
///
/// The first draft of this file set `STRIDE` to 80, which left the payload at
/// 72 while the fields below sum to 80 — so `worst` was written *into the
/// seal's own eight bytes*, every seal then failed to match, and `index_of`
/// would have skipped every row it had just written. A file that indexes to
/// nothing looks exactly like a file nobody wrote to.
///
/// `32 + 4 + 4 + 8 + 8 + 8 + 8 + 8`.
const _: () = assert!(PAYLOAD_BYTES == 32 + 4 + 4 + 8 + 8 + 8 + 8 + 8);

/// One round trip, with the run it belongs to.
///
/// `identity` is repeated on every row and that repetition is the point: it is
/// what lets [`Trades::open`] rebuild the block index from the file alone. The
/// predecessor stored a bare sequence number and could not.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Row {
    /// The run this trade belongs to — the same 32 bytes `results::Record` and
    /// `frontier::Row` carry.
    pub identity: [u8; 32],
    /// Position within this run's trades, from zero. Kept so a reader can order
    /// rows without depending on file order.
    pub seq: u32,
    /// The bar the combination fired on.
    pub signal_bar: u64,
    /// The bar the position was entered on — `signal_bar + 1` for a
    /// same-timeframe column, the aligned execution bar otherwise.
    pub entry_bar: u64,
    /// The bar the position was closed on.
    pub exit_bar: u64,
    /// The most favourable excursion reached, in paisa.
    pub best: i64,
    /// The most adverse excursion reached, in paisa. Negative or zero.
    pub worst: i64,
}

impl Row {
    /// This row as the bytes that go on disk, sealed.
    #[must_use]
    #[allow(
        clippy::indexing_slicing,
        reason = "every write is at a compile-time offset into an array whose \
                  length is asserted above; `PAYLOAD_BYTES` is checked to be 72 \
                  and the writes below sum to exactly that."
    )]
    pub fn to_bytes(&self) -> [u8; STRIDE_BYTES] {
        let mut out = [0_u8; STRIDE_BYTES];
        let mut at = 0_usize;
        let mut put = |bytes: &[u8], at: &mut usize| {
            out[*at..*at + bytes.len()].copy_from_slice(bytes);
            *at += bytes.len();
        };
        put(&self.identity, &mut at); //  0  32
        put(&self.seq.to_le_bytes(), &mut at); // 32   4
        put(&[0_u8; 4], &mut at); // 36   4  reserved, keeps the i64s aligned
        put(&self.signal_bar.to_le_bytes(), &mut at); // 40   8
        put(&self.entry_bar.to_le_bytes(), &mut at); // 48   8
        put(&self.exit_bar.to_le_bytes(), &mut at); // 56   8
        put(&self.best.to_le_bytes(), &mut at); // 64   8
        put(&self.worst.to_le_bytes(), &mut at); // 72   8  -> payload ends at 80
        debug_assert_eq!(at, PAYLOAD_BYTES, "the fields must fill the payload");
        let seal = seal_of(&out);
        out[PAYLOAD_BYTES..].copy_from_slice(&seal);
        out
    }

    /// Whether a row's seal matches its payload.
    ///
    /// A torn write leaves a row whose bytes are plausible and whose seal is
    /// not, and this is what tells the two apart. `index_of` skips a row that
    /// fails rather than indexing a value it cannot trust.
    #[must_use]
    pub fn seal_matches(raw: &[u8; STRIDE_BYTES]) -> bool {
        let want = seal_of(raw);
        raw.get(PAYLOAD_BYTES..) == Some(&want[..])
    }

    /// A row from its bytes. The caller checks [`Self::seal_matches`] first.
    #[must_use]
    #[allow(
        clippy::indexing_slicing,
        reason = "the same compile-time offsets `to_bytes` writes at, over an \
                  array of the same asserted length."
    )]
    pub fn from_bytes(raw: &[u8; STRIDE_BYTES]) -> Self {
        let mut identity = [0_u8; 32];
        identity.copy_from_slice(&raw[0..32]);
        let mut four = [0_u8; 4];
        let mut eight = [0_u8; 8];
        four.copy_from_slice(&raw[32..36]);
        let seq = u32::from_le_bytes(four);
        eight.copy_from_slice(&raw[40..48]);
        let signal_bar = u64::from_le_bytes(eight);
        eight.copy_from_slice(&raw[48..56]);
        let entry_bar = u64::from_le_bytes(eight);
        eight.copy_from_slice(&raw[56..64]);
        let exit_bar = u64::from_le_bytes(eight);
        eight.copy_from_slice(&raw[64..72]);
        let best = i64::from_le_bytes(eight);
        eight.copy_from_slice(&raw[72..80]);
        let worst = i64::from_le_bytes(eight);
        Self {
            identity,
            seq,
            signal_bar,
            entry_bar,
            exit_bar,
            best,
            worst,
        }
    }
}

/// The first eight bytes of a blake3 over the payload.
///
/// Eight rather than thirty-two because this detects a torn write, which is a
/// question about accident and not about an adversary — the same argument
/// `frontier::seal_of` makes, and the same construction, so the two files fail
/// the same way.
fn seal_of(raw: &[u8; STRIDE_BYTES]) -> [u8; SEAL_BYTES] {
    let mut hasher = brutex_core::blake3::Hasher::new();
    hasher.update(raw.get(..PAYLOAD_BYTES).unwrap_or(&[]));
    let full = hasher.finalize();
    let mut out = [0_u8; SEAL_BYTES];
    out.copy_from_slice(full.get(..SEAL_BYTES).unwrap_or(&[0_u8; SEAL_BYTES]));
    out
}

/// Where one run's rows sit, so finding them is a hash probe and not a scan.
#[derive(Clone, Copy, Debug)]
pub struct Block {
    /// Index of this run's first row.
    pub first: u64,
    /// How many rows it has.
    pub count: u64,
}

/// The per-trade file.
pub struct Trades {
    file: File,
    path: PathBuf,
    blocks: std::collections::HashMap<[u8; 32], Block>,
}

impl Trades {
    /// Where the file lives, beside the ledger and the frontier.
    #[must_use]
    pub fn path(root: &Path) -> PathBuf {
        root.join("results").join("trades.bin")
    }

    /// Opens for reading and **never creates**.
    ///
    /// A GET that creates its own empty file answers "no trades" by making that
    /// true, which is the failure `CLAUDE.md` §4 bans. `frontier::open_read`
    /// exists for the same reason and, unlike this one, had no caller.
    ///
    /// # Errors
    ///
    /// Names the path and the cause when the file is absent, is not this format
    /// at this version, or cannot be read.
    pub fn open_read(root: &Path) -> Result<Self, Refusal> {
        let path = Self::path(root);
        let mut file = OpenOptions::new()
            .read(true)
            .open(&path)
            .map_err(|why| format!("{} could not be opened: {why}", path.display()))?;
        let len = file
            .metadata()
            .map_err(|why| format!("{} could not be measured: {why}", path.display()))?
            .len();
        check_header(&mut file, &path, len)?;
        let blocks = index_of(&mut file, len)?;
        Ok(Self { file, path, blocks })
    }

    /// Opens for append, creating the file and its header if absent.
    ///
    /// # Errors
    ///
    /// Names the path and the cause when the directory cannot be made, the file
    /// cannot be opened, or an existing file is not this format at this version.
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
            .map_err(|why| format!("{} could not be measured: {why}", path.display()))?
            .len();
        if len == 0 {
            write_fresh_header(&mut file, &path)?;
            return Ok(Self {
                file,
                path,
                blocks: std::collections::HashMap::new(),
            });
        }
        check_header(&mut file, &path, len)?;
        let blocks = index_of(&mut file, len)?;
        Ok(Self { file, path, blocks })
    }

    /// Where a run's rows are, or `None`. **O(1)** — one hash probe.
    #[must_use]
    pub fn block(&self, identity: &[u8; 32]) -> Option<Block> {
        self.blocks.get(identity).copied()
    }

    /// Whether this run's trades are already recorded. **O(1)**.
    ///
    /// Unlike its predecessor, this answers correctly after a restart, because
    /// the index is rebuilt from rows that carry their own identity.
    #[must_use]
    pub fn holds(&self, identity: &[u8; 32]) -> bool {
        self.blocks.contains_key(identity)
    }

    /// Appends one run's rows, refusing a run already recorded.
    ///
    /// Refused rather than appended twice for the reason `results::append` gives:
    /// same inputs give same outputs, so a second copy adds nothing and a reader
    /// asking for "this run's trades" would get two runs' worth.
    ///
    /// # Errors
    ///
    /// Refuses a run already recorded, and names the path and cause when the
    /// file cannot be measured, seeked, written or synced.
    pub fn append_all(&mut self, rows: &[Row]) -> Result<u64, Refusal> {
        let Some(first_row) = rows.first() else {
            return Ok(0);
        };
        if self.holds(&first_row.identity) {
            return Err(format!(
                "the trades for run {} are already recorded, so this write has \
                 nothing to add",
                hex32(&first_row.identity)
            ));
        }
        let len = self
            .file
            .metadata()
            .map_err(|why| format!("{} could not be measured: {why}", self.path.display()))?
            .len();
        let first = len.saturating_sub(HEADER) / STRIDE;
        self.file
            .seek(SeekFrom::Start(len))
            .map_err(|why| format!("{} could not be seeked: {why}", self.path.display()))?;
        // ONE WRITE, NOT ONE PER ROW. A run with fifty thousand trades is fifty
        // thousand syscalls otherwise, which is the shape `frontier::of_run`'s
        // doc records having removed elsewhere.
        let mut buffer: Vec<u8> = Vec::with_capacity(rows.len().saturating_mul(STRIDE_BYTES));
        for row in rows {
            buffer.extend_from_slice(&row.to_bytes());
        }
        self.file
            .write_all(&buffer)
            .map_err(|why| format!("{} could not be written: {why}", self.path.display()))?;
        self.file
            .sync_data()
            .map_err(|why| format!("{} could not be synced: {why}", self.path.display()))?;
        self.blocks.insert(
            first_row.identity,
            Block {
                first,
                count: rows.len() as u64,
            },
        );
        Ok(rows.len() as u64)
    }

    /// Every row of one run, in order. **O(count)** after an O(1) lookup.
    ///
    /// # Errors
    ///
    /// Names the cause when the file cannot be seeked or a row cannot be read.
    /// A row whose seal does not match is SKIPPED rather than refused: the rest
    /// of the run is still readable and a torn tail must not hide it.
    pub fn of_run(&mut self, identity: &[u8; 32]) -> Result<Vec<Row>, Refusal> {
        let Some(block) = self.block(identity) else {
            return Ok(Vec::new());
        };
        let at = HEADER.saturating_add(block.first.saturating_mul(STRIDE));
        self.file
            .seek(SeekFrom::Start(at))
            .map_err(|why| format!("{} could not be seeked: {why}", self.path.display()))?;
        let mut out = Vec::with_capacity(usize::try_from(block.count).unwrap_or(0));
        let mut raw = [0_u8; STRIDE_BYTES];
        for _ in 0..block.count {
            self.file
                .read_exact(&mut raw)
                .map_err(|why| format!("a trade row could not be read: {why}"))?;
            if Row::seal_matches(&raw) {
                out.push(Row::from_bytes(&raw));
            }
        }
        Ok(out)
    }
}

/// Refuses a file whose first bytes are not this format at this version.
fn check_header(file: &mut File, path: &Path, len: u64) -> Result<(), Refusal> {
    if len < HEADER {
        return Err(format!(
            "{} is {len} bytes, shorter than this format's {HEADER}-byte header, \
             so it is not one of these files",
            path.display()
        ));
    }
    file.seek(SeekFrom::Start(0))
        .map_err(|why| format!("{} could not be seeked: {why}", path.display()))?;
    let mut head = [0_u8; HEADER_BYTES];
    file.read_exact(&mut head)
        .map_err(|why| format!("{} could not be read: {why}", path.display()))?;
    if head.get(..8) != Some(&MAGIC[..]) {
        return Err(format!(
            "{} does not begin with this format's magic, so it is not one of \
             these files and is refused rather than parsed",
            path.display()
        ));
    }
    let mut four = [0_u8; 4];
    four.copy_from_slice(head.get(8..12).unwrap_or(&[0_u8; 4]));
    let found = u32::from_le_bytes(four);
    if found != VERSION {
        return Err(format!(
            "{} is version {found} and this build writes {VERSION}. Store format \
             versions are never mutated in place, so a reader that does not know \
             a version refuses rather than guessing at its stride",
            path.display()
        ));
    }
    Ok(())
}

/// Writes the header of a new file.
fn write_fresh_header(file: &mut File, path: &Path) -> Result<(), Refusal> {
    let mut head = [0_u8; HEADER_BYTES];
    head.get_mut(..8)
        .ok_or_else(|| "the header is shorter than its magic".to_owned())?
        .copy_from_slice(&MAGIC);
    head.get_mut(8..12)
        .ok_or_else(|| "the header is shorter than its version".to_owned())?
        .copy_from_slice(&VERSION.to_le_bytes());
    file.write_all(&head)
        .map_err(|why| format!("{} could not be written: {why}", path.display()))?;
    file.sync_data()
        .map_err(|why| format!("{} could not be synced: {why}", path.display()))?;
    Ok(())
}

/// Rebuilds the block index in one pass.
///
/// This is the whole reason the identity is on every row. Its predecessor set
/// `HashMap::new()` here and on the create path, so every lookup after a restart
/// answered "no such run" — and the module's own header claimed an O(1) find.
fn index_of(
    file: &mut File,
    len: u64,
) -> Result<std::collections::HashMap<[u8; 32], Block>, Refusal> {
    let count = len.saturating_sub(HEADER) / STRIDE;
    let mut blocks: std::collections::HashMap<[u8; 32], Block> =
        std::collections::HashMap::with_capacity(
            usize::try_from(count)
                .unwrap_or(0)
                .saturating_div(64)
                .max(8),
        );
    file.seek(SeekFrom::Start(HEADER))
        .map_err(|why| format!("the trades file could not be seeked: {why}"))?;
    let mut raw = [0_u8; STRIDE_BYTES];
    for index in 0..count {
        if file.read_exact(&mut raw).is_err() {
            // A SHORT TAIL IS A TORN WRITE, NOT A CORRUPT FILE. Everything
            // before it is whole and indexed; stopping here keeps those
            // readable rather than refusing the lot.
            break;
        }
        if !Row::seal_matches(&raw) {
            continue;
        }
        let identity = Row::from_bytes(&raw).identity;
        blocks
            .entry(identity)
            .and_modify(|block| {
                block.count = index.saturating_add(1).saturating_sub(block.first);
            })
            .or_insert(Block {
                first: index,
                count: 1,
            });
    }
    Ok(blocks)
}

/// A run identity as lowercase hex, for a message a reader can search the
/// ledger with.
fn hex32(bytes: &[u8; 32]) -> String {
    let mut out = String::with_capacity(64);
    for byte in bytes {
        let _ = std::fmt::Write::write_fmt(&mut out, format_args!("{byte:02x}"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{Block, Row, STRIDE_BYTES, Trades};

    fn row(identity: [u8; 32], seq: u32) -> Row {
        Row {
            identity,
            seq,
            signal_bar: 11,
            entry_bar: 12,
            exit_bar: 40,
            best: 4_250,
            worst: -1_375,
        }
    }

    /// EVERY FIELD, BOTH WAYS. The first draft of this file sized the stride so
    /// that `worst` was written into the seal's own bytes -- every seal then
    /// failed, `index_of` skipped every row, and a file full of trades looked
    /// exactly like a file nobody had written to. A round trip is the only test
    /// that would have caught it, because each field on its own looked right.
    #[test]
    fn every_field_survives_the_round_trip() {
        let want = row([7_u8; 32], 3);
        let raw = want.to_bytes();
        assert!(Row::seal_matches(&raw), "a row must seal its own payload");
        assert_eq!(Row::from_bytes(&raw), want, "every field, unchanged");
    }

    /// A row whose payload is edited after sealing must not pass.
    #[test]
    fn a_row_damaged_after_it_was_written_is_refused() {
        let mut raw = row([1_u8; 32], 0).to_bytes();
        assert!(Row::seal_matches(&raw));
        raw[40] ^= 0xFF;
        assert!(
            !Row::seal_matches(&raw),
            "a changed payload must not match the seal it was written with"
        );
    }

    /// THE INDEX IS REBUILT FROM THE FILE, WHICH IS THE WHOLE POINT.
    ///
    /// The store this replaces set `HashMap::new()` on both open paths, so after
    /// a restart every lookup answered "no such run" while its own header
    /// claimed an O(1) find. Reopening is the only test that separates the two.
    #[test]
    fn a_reopened_file_still_finds_the_run_it_was_given() {
        let dir = std::env::temp_dir().join(format!(
            "brutex-trades-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let one = [9_u8; 32];
        let two = [5_u8; 32];

        {
            let mut file = Trades::open(&dir).expect("a fresh file opens");
            let rows: Vec<Row> = (0..3).map(|s| row(one, s)).collect();
            assert_eq!(file.append_all(&rows).expect("appends"), 3);
            let more: Vec<Row> = (0..2).map(|s| row(two, s)).collect();
            assert_eq!(file.append_all(&more).expect("appends"), 2);
        }

        let mut reopened = Trades::open_read(&dir).expect("reopens read-only");
        assert!(reopened.holds(&one), "the index must survive a reopen");
        assert!(reopened.holds(&two), "both runs, not just the first");
        let Some(Block { first, count }) = reopened.block(&two) else {
            panic!("the second run must have a block");
        };
        assert_eq!(
            (first, count),
            (3, 2),
            "the second run starts after the first"
        );
        let back = reopened.of_run(&one).expect("reads the first run");
        assert_eq!(back.len(), 3);
        assert_eq!(back.first().map(|r| r.seq), Some(0));
        assert_eq!(
            reopened
                .of_run(&[0_u8; 32])
                .expect("a missing run is not an error"),
            Vec::new(),
            "a run with no rows is an empty answer, not a refusal"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Recording the same run twice would give a reader two runs' worth of rows
    /// under one identity, so it is refused the way the ledger refuses one.
    #[test]
    fn the_same_run_is_refused_rather_than_recorded_twice() {
        let dir = std::env::temp_dir().join(format!(
            "brutex-trades-dup-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let id = [3_u8; 32];
        let mut file = Trades::open(&dir).expect("opens");
        assert_eq!(file.append_all(&[row(id, 0)]).expect("first append"), 1);
        let again = file.append_all(&[row(id, 0)]);
        assert!(again.is_err(), "the second write must be refused");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// READING MUST NEVER CREATE. A GET on a store that has never been swept
    /// would otherwise answer "no trades" by making that true.
    #[test]
    fn open_read_does_not_create_the_file() {
        let dir = std::env::temp_dir().join(format!(
            "brutex-trades-noc-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        assert!(Trades::open_read(&dir).is_err(), "an absent file refuses");
        assert!(
            !Trades::path(&dir).exists(),
            "and the read must not have created it"
        );
    }

    /// An empty run writes nothing rather than an empty block.
    #[test]
    fn a_run_with_no_trades_writes_no_rows() {
        let dir = std::env::temp_dir().join(format!(
            "brutex-trades-empty-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let mut file = Trades::open(&dir).expect("opens");
        assert_eq!(file.append_all(&[]).expect("an empty write"), 0);
        assert!(!file.holds(&[0_u8; 32]));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The stride is what makes a lookup a multiply, so it is pinned.
    #[test]
    fn the_stride_is_what_the_layout_says() {
        assert_eq!(STRIDE_BYTES, 88);
        assert_eq!(row([0_u8; 32], 0).to_bytes().len(), STRIDE_BYTES);
    }
}

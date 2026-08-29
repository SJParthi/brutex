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
const VERSION: u32 = 2;

/// Magic, version and a reserved word.
const HEADER: u64 = 16;
const HEADER_BYTES: usize = 16;
const _: () = assert!(HEADER_BYTES as u64 == HEADER);

/// One row. Fixed, so `index -> byte offset` is a multiply and the lookup is
/// O(1) — `CLAUDE.md` §3 rule 4.
const STRIDE: u64 = 104;
const STRIDE_BYTES: usize = 104;
const _: () = assert!(STRIDE_BYTES as u64 == STRIDE);

/// The last eight bytes of a row are a seal over the first ninety-six.
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
/// `32 + 4 + 4 + 8 + 8 + 8 + 8 + 8 + 8 + 8`.
const _: () = assert!(PAYLOAD_BYTES == 32 + 4 + 4 + 8 + 8 + 8 + 8 + 8 + 8 + 8);

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
    /// Paisa per unit with both legs filled at the bar OPEN -- the BEST-CASE
    /// realised P&L of this round trip, not an excursion.
    ///
    /// THIS DOC SAID "the most favourable excursion reached" AND THAT WAS WRONG.
    /// `runner::trade::Trade::best` is a completed round trip priced at the most
    /// favourable fill that could have happened, and the companion field said
    /// "Negative or zero", which is false of a worst-case P&L on a winning
    /// trade. A reader trusting either sentence would have filtered on
    /// `worst <= 0` and found nothing, or read an excursion where a result was.
    pub best: i64,
    /// Paisa per unit with both legs filled at the bar PRINTED EXTREME -- the
    /// WORST-CASE realised P&L, and the figure selection actually ranks on.
    ///
    /// Never better than [`Self::best`], and the gap between them is the whole
    /// range a real fill can land in. Positive on a trade that wins even at the
    /// worst fill, which is the only kind worth having.
    pub worst: i64,
    /// When the position was ENTERED, in microseconds since the epoch, copied
    /// straight off `Candle::ts_micros` of the entry bar.
    ///
    /// # Why this is stored and not derived
    ///
    /// Because the three `*_bar` fields above are INDICES INTO A SPAN, and a
    /// reader of this file does not have the span. `/trades.json` would have to
    /// reopen the right instrument-month, seek to the right stride and read the
    /// bar back — which is possible, and O(1) per trade, and still the wrong
    /// shape: it makes the answer to *"which weekday made the money"* depend on
    /// the bar files still being there, unchanged, months later.
    ///
    /// The operator's question is exactly that one: *"as per this combination
    /// which days especially which particular time period made this massive
    /// success and profit"*. With the epoch on the row it is one pass over the
    /// run's own block and no lookup at all — weekday is
    /// `(days_since_epoch + 4) % 7` and the minute of the session is a divide.
    /// Without it the question could not be asked of this file.
    pub entry_micros: i64,
    /// When the position was CLOSED, same units and same reason.
    ///
    /// Kept alongside the entry rather than derived from `exit_bar - entry_bar`,
    /// because the two bars can sit either side of a weekend or a holiday and
    /// the difference in BARS is not the difference in TIME. A holding period
    /// measured in bars would call a Friday-to-Monday trade the same length as
    /// a Monday-to-Tuesday one.
    pub exit_micros: i64,
}

impl Row {
    /// This row as the bytes that go on disk, sealed.
    #[must_use]
    #[allow(
        clippy::indexing_slicing,
        reason = "every write is at a compile-time offset into an array whose \
                  length is asserted above; `PAYLOAD_BYTES` is checked to be 96 \
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
        put(&self.worst.to_le_bytes(), &mut at); // 72   8
        put(&self.entry_micros.to_le_bytes(), &mut at); // 80   8
        put(&self.exit_micros.to_le_bytes(), &mut at); // 88   8  -> payload ends at 96
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
        eight.copy_from_slice(&raw[80..88]);
        let entry_micros = i64::from_le_bytes(eight);
        eight.copy_from_slice(&raw[88..96]);
        let exit_micros = i64::from_le_bytes(eight);
        Self {
            identity,
            seq,
            signal_bar,
            entry_bar,
            exit_bar,
            best,
            worst,
            entry_micros,
            exit_micros,
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
    /// How far the block index has read, in bytes from the start of the file.
    ///
    /// `absorb_new_rows` resumes here rather than rescanning, so a writer that
    /// takes the lock and finds another process has appended pays only for what
    /// arrived since. Zero on the one-writer path, which is every path today.
    scanned: u64,
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
        // `index_of` walks every WHOLE row, so the cursor sits at the end of the
        // last one -- and a ragged tail cannot reach here, `check_header`
        // refuses it.
        let scanned =
            HEADER.saturating_add((len.saturating_sub(HEADER) / STRIDE).saturating_mul(STRIDE));
        Ok(Self {
            file,
            path,
            blocks,
            scanned,
        })
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
                // A FRESH FILE HOLDS ONLY ITS HEADER, so the index has read
                // everything there is.
                scanned: HEADER,
            });
        }
        check_header(&mut file, &path, len)?;
        let blocks = index_of(&mut file, len)?;
        // `index_of` walks every WHOLE row, so the cursor sits at the end of the
        // last one -- and a ragged tail cannot reach here, `check_header`
        // refuses it.
        let scanned =
            HEADER.saturating_add((len.saturating_sub(HEADER) / STRIDE).saturating_mul(STRIDE));
        Ok(Self {
            file,
            path,
            blocks,
            scanned,
        })
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
        // AN EXCLUSIVE FILE LOCK, AND THIS FILE WAS THE ONE OF THE THREE
        // WITHOUT ONE.
        //
        // `results::append` takes `lock()` and explains why at length; this
        // measured the end with a bare `metadata()` and seeked to it. Two
        // processes — two `cli` sweeps, or a sweep beside the server — both
        // measure the same length, both seek there, and the SECOND WRITE LANDS
        // ON TOP OF THE FIRST. Both blocks seal correctly, the count still
        // reads as one, and the losing process has already printed "N trade(s)
        // recorded for this run". Nothing downstream can see it.
        //
        // The `LEDGER` mutex above serialises the three record calls WITHIN one
        // process and does nothing across two, which is exactly the case this
        // store is reachable in: `range_over` runs eight rungs and the operator
        // runs the server alongside.
        self.file
            .lock()
            .map_err(|why| format!("{} could not be locked: {why}", self.path.display()))?;
        let out = self.append_locked(rows, first_row.identity);
        // RELEASED WHETHER OR NOT THE WRITE SUCCEEDED, and the two failures are
        // reported separately rather than one hiding the other — the same shape
        // `results::append` uses.
        let released = self
            .file
            .unlock()
            .map_err(|why| format!("{} could not be unlocked: {why}", self.path.display()));
        match (out, released) {
            (Ok(count), Ok(())) => Ok(count),
            (Err(why), _) | (Ok(_), Err(why)) => Err(why),
        }
    }

    /// [`Self::append_all`]'s body, with the file lock already held.
    ///
    /// Split out for the reason `results::append_locked` is: an early `return`
    /// inside the locked region would strand the lock until the process exits.
    fn append_locked(&mut self, rows: &[Row], identity: [u8; 32]) -> Result<u64, Refusal> {
        // THE DUPLICATE TEST NEEDS THE LOCK, and it was made before it.
        // `blocks` is built once at open and knows nothing about a block
        // another process appended since. Two processes both passed, and
        // `index_of`'s `and_modify` then made the FIRST block span every row
        // written between the two — so `of_run` returned both runs' trades as
        // one run's.
        self.absorb_new_rows()?;
        if self.holds(&identity) {
            return Err(format!(
                "the trades for run {} are already recorded, so this write has \
                 nothing to add -- and a second block would make the first span \
                 every row written between them.",
                hex32(&identity)
            ));
        }
        // FROM THE FILE UNDER THE LOCK, not from a length measured before it.
        let at = self
            .file
            .seek(SeekFrom::End(0))
            .map_err(|why| format!("{} could not be extended: {why}", self.path.display()))?;
        let first = at.saturating_sub(HEADER) / STRIDE;
        // ONE WRITE, NOT ONE PER ROW. A run with fifty thousand trades is fifty
        // thousand syscalls otherwise, which is the shape `frontier::of_run`'s
        // doc records having removed elsewhere.
        let mut buffer: Vec<u8> = Vec::with_capacity(rows.len().saturating_mul(STRIDE_BYTES));
        for row in rows {
            buffer.extend_from_slice(&row.to_bytes());
        }
        // A PARTIAL WRITE IS ROLLED BACK. `write_all` on a full filesystem can
        // put some bytes down before it fails, and those bytes are not a row --
        // leaving them ragged is what makes every LATER run's trades unreadable,
        // because `first` floors while the seek does not. The cause is known
        // here and `at` is where the file ended, so this removes only what this
        // call wrote.
        self.file
            .write_all(&buffer)
            .map_err(|why| match self.file.set_len(at) {
                Ok(()) => format!(
                    "{} could not be written: {why}. The partial write was rolled \
                     back, so the file still ends on a whole row.",
                    self.path.display()
                ),
                Err(and) => format!(
                    "{} could not be written: {why}. Rolling the partial write \
                     back ALSO failed: {and}. The file now ends mid-row and is \
                     refused on the next open until its tail is cut back to byte \
                     {at}.",
                    self.path.display()
                ),
            })?;
        // `sync_all`, NOT `sync_data`: this write EXTENDS the file, so the
        // LENGTH is part of what has to survive. A torn length here does not
        // cost "a detail row" -- it misaligns every run recorded afterwards.
        self.file
            .sync_all()
            .map_err(|why| format!("{} could not be synced: {why}", self.path.display()))?;
        self.blocks.insert(
            identity,
            Block {
                first,
                count: rows.len() as u64,
            },
        );
        self.scanned = at.saturating_add(buffer.len() as u64);
        Ok(rows.len() as u64)
    }

    /// Absorbs blocks another writer has appended since this handle last looked.
    ///
    /// O(rows appended by others), which is zero on the one-writer path.
    fn absorb_new_rows(&mut self) -> Result<(), Refusal> {
        let len = self
            .file
            .metadata()
            .map_err(|why| format!("{} could not be measured: {why}", self.path.display()))?
            .len();
        let mut raw = [0_u8; STRIDE_BYTES];
        while self.scanned.saturating_add(STRIDE) <= len {
            let at = self.scanned;
            self.file
                .seek(SeekFrom::Start(at))
                .and_then(|_| self.file.read_exact(&mut raw))
                .map_err(|why| format!("the trade row at byte {at} could not be read: {why}"))?;
            if Row::seal_matches(&raw) {
                let index = at.saturating_sub(HEADER) / STRIDE;
                self.blocks
                    .entry(Row::from_bytes(&raw).identity)
                    .and_modify(|held| {
                        held.count = index.saturating_add(1).saturating_sub(held.first);
                    })
                    .or_insert(Block {
                        first: index,
                        count: 1,
                    });
            }
            self.scanned = at.saturating_add(STRIDE);
        }
        Ok(())
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
        let mut foreign = 0_u64;
        for _ in 0..block.count {
            self.file
                .read_exact(&mut raw)
                .map_err(|why| format!("a trade row could not be read: {why}"))?;
            if Row::seal_matches(&raw) {
                // A ROW THAT IS NOT THIS RUN'S IS NOT THIS RUN'S, HOWEVER
                // CORRECTLY IT IS SEALED.
                //
                // The seal answers *were these bytes written whole*. It says
                // nothing about WHOSE they are, and this loop treated the two
                // as one question — so any block whose `first`/`count` is wrong
                // returned whatever sealed correctly at those offsets, under
                // this run's banner. `/trades.json` then feeds them to the
                // weekday and hour attribution, so another run's P&L is
                // reported as this one's shape.
                //
                // A wrong span is reachable: `index_of`'s `and_modify` sets
                // `count = last + 1 - first`, so a second block for one
                // identity makes the first SPAN every row written between them.
                // §3 rule 8 keeps such a file as it is, which is why the READ
                // filters rather than the file being repaired.
                //
                // `frontier::of_run` has made this check since it was written
                // and says the same thing; this file did not.
                let row = Row::from_bytes(&raw);
                if row.identity == *identity {
                    out.push(row);
                } else {
                    foreign = foreign.saturating_add(1);
                }
            }
        }
        if foreign > 0 {
            // NAMED, NOT SILENTLY SHORTENED. A list that is shorter than the
            // block claimed is a fact the reader needs; dropping the rows and
            // saying nothing is the same silence this whole guard exists to
            // end.
            return Err(format!(
                "the block for run {} spans {foreign} row(s) belonging to \
                 another run, so its recorded span is wrong. {} row(s) that are \
                 genuinely this run's were kept and are not returned here, \
                 because a partial answer under a whole answer's name is what \
                 §4 refuses. The file is unchanged.",
                hex32(identity),
                out.len()
            ));
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
    // A RAGGED TAIL IS NAMED RATHER THAN ABSORBED, and this file was the only
    // one of the three without the check.
    //
    // `append_all` computes `first = (len - HEADER) / STRIDE`, which FLOORS,
    // and then seeks to `len`. On a whole-row file those agree. After an
    // interrupted write they do not: the index names one offset and the write
    // goes to another, so every row of the new block seals against a window it
    // does not occupy, `of_run` finds no valid seal at the offsets its block
    // names, and `/trades.json` answers `{"trades":[],"count":0}` for a run
    // whose rows are physically on disk. Every run recorded afterwards inherits
    // the same shift.
    //
    // That is the plausible wrong answer §4 bans, and it is silent: the seals
    // are individually valid, so nothing downstream can see the misalignment.
    // `frontier::check_header` and `results::open_with` have both refused this
    // since they were written; the wording here is theirs.
    //
    // The remainder is left alone rather than truncated. Cutting a file back is
    // a decision about the operator's history and belongs to them — §3 rule 8 —
    // and the whole rows before the tear are still readable and still counted.
    let body = len.saturating_sub(HEADER);
    if !body.is_multiple_of(STRIDE) {
        let whole = body / STRIDE;
        let spare = body % STRIDE;
        return Err(format!(
            "{} has {spare} bytes past its last whole row — an append was \
             interrupted. {whole} whole rows are intact; the remainder is left \
             alone, because truncating it is a decision about your own history. \
             Nothing was written.",
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
    // ONE SYSCALL PER 8 KiB, NOT ONE PER 104-BYTE ROW.
    //
    // This loop read straight from the `File`, so `read_exact` was one `read(2)`
    // per row -- and `open_read` runs it on EVERY `/trades.json` request, which
    // means a request for one run's twenty trades walked every trade ever
    // recorded, one syscall at a time. An O(1) audit measured the shape as
    // "O(rows), 1 read_exact syscall/row".
    //
    // A `BufReader` does not change the O(rows) walk -- only a persisted or
    // cached index does that, and it is the right fix -- but it divides the
    // syscall count by the rows that fit in a buffer: at 104 bytes and the
    // default 8 KiB that is 78 rows per read. The borrow is scoped so the
    // `File`'s own cursor is free afterwards, and every later reader seeks
    // explicitly before it reads.
    let mut buffered = std::io::BufReader::new(&mut *file);
    let mut raw = [0_u8; STRIDE_BYTES];
    for index in 0..count {
        if buffered.read_exact(&mut raw).is_err() {
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
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]
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
            // 2024-01-15 09:20 IST as UTC micros -- a Monday, so the weekday
            // arithmetic below has a known answer to be wrong about.
            entry_micros: 1_705_290_600_000_000,
            exit_micros: 1_705_292_400_000_000,
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
        assert_eq!(STRIDE_BYTES, 104);
        assert_eq!(row([0_u8; 32], 0).to_bytes().len(), STRIDE_BYTES);
    }
}

/// One calendar bucket's result, under both fill readings at once.
///
/// # Why both readings live in one bucket
///
/// Because the operator's standing requirement is that *"in all these both
/// worst and best case will be always covered"*, and a bucket carrying one of
/// them would force the page to ask twice and hope the two answers were built
/// from the same trades. They are built from the same pass here.
///
/// A "win" is a round trip that ended above water UNDER THAT READING, so a
/// trade can be a win at the best fill and a loss at the worst — and the gap
/// between `wins` and `worst_wins` is exactly the number of trades whose
/// outcome the data cannot settle.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Bucket {
    /// Which bucket this is, in the units [`Period`] defines.
    pub key: i64,
    /// Round trips that started in this bucket.
    pub trades: u64,
    /// Of those, the ones that ended above water at the BEST fill.
    pub wins: u64,
    /// And at the WORST fill — never more than [`Self::wins`].
    pub worst_wins: u64,
    /// Total paisa per unit at the best fill.
    pub best_paisa: i64,
    /// Total paisa per unit at the worst fill.
    pub worst_paisa: i64,
    /// The single best round trip in this bucket, at the worst fill.
    pub largest_win: i64,
    /// The single worst round trip in this bucket, at the worst fill.
    pub largest_loss: i64,
}

impl Bucket {
    /// Fold one trade in. Every field is a compare or an add, so this is O(1)
    /// and the whole aggregation is one pass — `CLAUDE.md` §3 rule 4.
    fn take(&mut self, row: &Row) {
        self.trades = self.trades.saturating_add(1);
        if row.best > 0 {
            self.wins = self.wins.saturating_add(1);
        }
        if row.worst > 0 {
            self.worst_wins = self.worst_wins.saturating_add(1);
        }
        self.best_paisa = self.best_paisa.saturating_add(row.best);
        self.worst_paisa = self.worst_paisa.saturating_add(row.worst);
        // SEEDED FROM THE FIRST TRADE, not from zero. A bucket whose every trade
        // lost would report a `largest_win` of 0 if this started at zero, and
        // zero is a better result than every trade it actually holds -- the
        // operator's own rule is `min(win) >= 3x max(loss)`, which a phantom
        // zero would silently satisfy on the losing side and fail on the winning
        // one. `trades == 1` is the seed test because `take` has already
        // incremented it.
        if self.trades == 1 {
            self.largest_win = row.worst;
            self.largest_loss = row.worst;
        } else {
            self.largest_win = self.largest_win.max(row.worst);
            self.largest_loss = self.largest_loss.min(row.worst);
        }
    }
}

/// The calendar periods a run's trades can be grouped by.
///
/// # Why these six and why they are integers
///
/// The operator asked for *"every day how many wins how many loss every week
/// every month every quarter every half every year"*. Each is a pure integer
/// function of the entry timestamp — no date library, no allocation, no
/// formatting — so bucketing a trade is a handful of divides and one hash
/// probe, and the whole aggregation is one pass over the run's own block.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Period {
    /// Days since 1970-01-01.
    Day,
    /// Seven-day blocks since 1970-01-01. **Not ISO weeks**: the epoch was a
    /// Thursday, so block boundaries fall on Thursdays. Stated rather than
    /// hidden, because a reader comparing these to a broker's Monday-start
    /// weekly statement would otherwise find them mysteriously off by three
    /// days. `Weekday` below is what answers "which day of the week".
    Week,
    /// `year * 12 + (month - 1)`, so consecutive months are consecutive keys
    /// across a year boundary.
    Month,
    /// `year * 4 + (month - 1) / 3`.
    Quarter,
    /// `year * 2 + (month - 1) / 6`.
    Half,
    /// The civil year.
    Year,
    /// 0 = Monday through 6 = Sunday, across the whole span.
    ///
    /// This is the one that answers *"which days made this massive success"* —
    /// [`Self::Day`] says *which dates*, and with eighty-one months of data that
    /// is 1,700 rows nobody can read. Seven rows can be read at a glance.
    Weekday,
    /// Minutes from midnight UTC, rounded down to the hour.
    ///
    /// Answers *"which particular time period"*. UTC and not IST, deliberately:
    /// the bars carry UTC and converting here would put a second timezone
    /// opinion in the engine. The page adds the 5.5-hour offset once, where it
    /// is visible.
    Hour,
}

impl Period {
    /// Every period, so a caller can build the whole board without naming them.
    pub const ALL: [Self; 8] = [
        Self::Day,
        Self::Week,
        Self::Month,
        Self::Quarter,
        Self::Half,
        Self::Year,
        Self::Weekday,
        Self::Hour,
    ];

    /// The name this period answers to over HTTP and on the page.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Day => "day",
            Self::Week => "week",
            Self::Month => "month",
            Self::Quarter => "quarter",
            Self::Half => "half",
            Self::Year => "year",
            Self::Weekday => "weekday",
            Self::Hour => "hour",
        }
    }

    /// Which bucket a microsecond timestamp falls in.
    ///
    /// # Why the division is floored rather than truncated
    ///
    /// Rust's `/` truncates towards zero, so `-1 / 86_400_000_000` is 0 — which
    /// would put 1969-12-31 in the same bucket as 1970-01-01. No bar in this
    /// store predates 1970, so it cannot bite today; it is written correctly
    /// anyway because the alternative is a comment promising it will not.
    #[must_use]
    pub fn bucket(self, micros: i64) -> i64 {
        const DAY: i64 = 86_400_000_000;
        let days = micros.div_euclid(DAY);
        let (year, month, _day) = telemetry::civil_from_days(days);
        match self {
            Self::Day => days,
            Self::Week => days.div_euclid(7),
            Self::Month => year.saturating_mul(12).saturating_add(month - 1),
            Self::Quarter => year.saturating_mul(4).saturating_add((month - 1) / 3),
            Self::Half => year.saturating_mul(2).saturating_add((month - 1) / 6),
            Self::Year => year,
            // 1970-01-01 was a THURSDAY, so shifting by 3 puts Monday at 0.
            // `rem_euclid` and not `%`, for the negative side.
            Self::Weekday => days.saturating_add(3).rem_euclid(7),
            Self::Hour => micros.rem_euclid(DAY) / 3_600_000_000,
        }
    }
}

/// Group one run's trades into every calendar period at once.
///
/// # Cost
///
/// One pass over the rows, and for each row eight hash probes — one per period.
/// Every probe and every fold is O(1), so this is O(trades) overall with a
/// constant of eight, and it allocates one map per period rather than one per
/// trade. `CLAUDE.md` §3 rule 4.
///
/// Buckets come back SORTED BY KEY, because a calendar read out of order is not
/// a calendar, and because §3 rule 5 requires two runs over the same bytes to
/// produce the same output — a `HashMap` iteration would not.
#[must_use]
pub fn by_period(rows: &[Row]) -> Vec<(Period, Vec<Bucket>)> {
    Period::ALL
        .iter()
        .map(|&period| {
            let mut held: std::collections::HashMap<i64, Bucket> =
                std::collections::HashMap::with_capacity(rows.len().min(512));
            for row in rows {
                // A ROW WITH NO TIMESTAMP IS SKIPPED AND NOT BUCKETED AT ZERO.
                // `record_trades` writes 0 when a bar index is out of range, and
                // 0 micros is 1970-01-01 -- a bucket that would sit fifty years
                // before every real one and drag every "first trade" readout
                // with it.
                if row.entry_micros == 0 {
                    continue;
                }
                let key = period.bucket(row.entry_micros);
                held.entry(key)
                    .or_insert(Bucket {
                        key,
                        ..Bucket::default()
                    })
                    .take(row);
            }
            let mut out: Vec<Bucket> = held.into_values().collect();
            out.sort_unstable_by_key(|b| b.key);
            (period, out)
        })
        .collect()
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail, and an index that is out of \
              range in a fixture IS the failure."
)]
mod period_tests {
    use super::{Bucket, Period, Row, by_period};

    /// 2024-01-15 09:20 UTC. A **Monday**, chosen because a weekday
    /// calculation off by one still lands on a real day and looks fine.
    const MONDAY: i64 = 1_705_310_400_000_000;
    const DAY: i64 = 86_400_000_000;

    fn trade(entry: i64, best: i64, worst: i64) -> Row {
        Row {
            identity: [7_u8; 32],
            seq: 0,
            signal_bar: 0,
            entry_bar: 1,
            exit_bar: 2,
            best,
            worst,
            entry_micros: entry,
            exit_micros: entry + 3_600_000_000,
        }
    }

    fn of(rows: &[Row], want: Period) -> Vec<Bucket> {
        by_period(rows)
            .into_iter()
            .find(|(p, _)| *p == want)
            .map(|(_, b)| b)
            .expect("every period is present")
    }

    /// Monday is 0 and Sunday is 6, across the whole week.
    ///
    /// The epoch was a THURSDAY, so the `+3` shift is the entire content of
    /// this calculation and an off-by-one is invisible without a fixed date.
    #[test]
    fn the_weekday_of_a_known_monday_is_zero_and_the_week_runs_to_six() {
        for (offset, expect) in (0..7).map(|d| (d, d)) {
            let rows = [trade(MONDAY + offset * DAY, 100, 50)];
            let got = of(&rows, Period::Weekday);
            assert_eq!(got.len(), 1);
            assert_eq!(
                got[0].key, expect,
                "{offset} days after a Monday must be weekday {expect}"
            );
        }
    }

    /// A win at the best fill can be a loss at the worst, and the two counts
    /// must disagree when the data cannot settle the trade.
    #[test]
    fn a_trade_can_win_on_one_reading_and_lose_on_the_other() {
        let rows = [
            trade(MONDAY, 500, 200),   // wins on both
            trade(MONDAY, 300, -100),  // wins best, loses worst
            trade(MONDAY, -400, -900), // loses on both
        ];
        let got = of(&rows, Period::Day);
        assert_eq!(got.len(), 1, "all three are the same day");
        assert_eq!(got[0].trades, 3);
        assert_eq!(got[0].wins, 2, "two end above water at the best fill");
        assert_eq!(got[0].worst_wins, 1, "only one survives the worst fill");
        assert_eq!(got[0].best_paisa, 400);
        assert_eq!(got[0].worst_paisa, -800);
    }

    /// The extremes are seeded from the first trade, not from zero.
    ///
    /// A bucket where everything lost must not report a `largest_win` of zero —
    /// zero is better than every trade it holds, and the operator's rule is
    /// `min(win) >= 3x max(loss)`, which a phantom zero would quietly pass.
    #[test]
    fn a_bucket_where_everything_lost_reports_no_phantom_zero_win() {
        let rows = [trade(MONDAY, -100, -300), trade(MONDAY, -50, -700)];
        let got = of(&rows, Period::Day);
        assert_eq!(got[0].largest_win, -300, "the least bad loss, not zero");
        assert_eq!(got[0].largest_loss, -700);
        assert_eq!(got[0].worst_wins, 0);
    }

    /// Consecutive months are consecutive keys ACROSS a year boundary, which
    /// `month` alone would not give: December then January would go 12 -> 1.
    #[test]
    fn december_and_january_are_adjacent_month_keys() {
        // 2024-12-15 and 2025-01-15.
        let december = 1_734_220_800_000_000;
        let january = 1_736_899_200_000_000;
        let rows = [trade(december, 1, 1), trade(january, 1, 1)];
        let got = of(&rows, Period::Month);
        assert_eq!(got.len(), 2);
        assert_eq!(
            got[1].key - got[0].key,
            1,
            "one month apart must be one key apart: {:?}",
            got.iter().map(|b| b.key).collect::<Vec<_>>()
        );
        // And the same for the quarter and the half.
        assert_eq!(
            of(&rows, Period::Quarter)[1].key - of(&rows, Period::Quarter)[0].key,
            1
        );
        assert_eq!(
            of(&rows, Period::Half)[1].key - of(&rows, Period::Half)[0].key,
            1
        );
        assert_eq!(
            of(&rows, Period::Year)[1].key - of(&rows, Period::Year)[0].key,
            1
        );
    }

    /// A row whose bar index was out of range carries 0 micros, and 1970 must
    /// not appear as a bucket fifty years before every real trade.
    #[test]
    fn a_trade_with_no_timestamp_is_skipped_rather_than_filed_under_1970() {
        let rows = [trade(0, 100, 100), trade(MONDAY, 100, 100)];
        let got = of(&rows, Period::Day);
        assert_eq!(got.len(), 1, "only the stamped trade is bucketed: {got:?}");
        assert_eq!(got[0].trades, 1);
    }

    /// Sorted by key, so a calendar reads as a calendar and two runs over the
    /// same bytes list the same buckets in the same order — §3 rule 5.
    #[test]
    fn buckets_come_back_in_calendar_order() {
        let rows = [
            trade(MONDAY + 5 * DAY, 1, 1),
            trade(MONDAY, 1, 1),
            trade(MONDAY + 2 * DAY, 1, 1),
        ];
        let keys: Vec<i64> = of(&rows, Period::Day).iter().map(|b| b.key).collect();
        let mut sorted = keys.clone();
        sorted.sort_unstable();
        assert_eq!(keys, sorted, "insertion order must not reach the output");
    }

    /// Every period is answered, so a page asking for all eight gets all eight
    /// rather than silently missing one.
    #[test]
    fn all_eight_periods_come_back_even_when_a_run_has_one_trade() {
        let got = by_period(&[trade(MONDAY, 1, 1)]);
        assert_eq!(got.len(), Period::ALL.len());
        for (period, buckets) in &got {
            assert_eq!(buckets.len(), 1, "{} held nothing", period.name());
        }
    }

    /// No trades is an empty board, not a panic and not a phantom bucket.
    #[test]
    fn a_run_with_no_trades_answers_empty_for_every_period() {
        for (period, buckets) in by_period(&[]) {
            assert!(buckets.is_empty(), "{} invented a bucket", period.name());
        }
    }
}

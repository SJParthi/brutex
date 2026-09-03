//! Every round trip of the exact chosen exit-grid cell, keyed by run identity.
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
//! # What is stored, and the legacy file that is not reinterpreted
//!
//! These rows are a replay of the **exact selected [`runner::grid::Cell`]**.
//! That includes the cell's stop/target/trailing exits and its own exclusivity:
//! after an early stop the next eligible signal may differ from the level-less
//! walk. The row direction, both fill outcomes, timestamps and exact adverse
//! and favourable excursions are durable facts, so the API never has to infer
//! one from the run identity or from a bar file that may later be absent.
//!
//! `results/trades.bin` version 2 is deliberately left untouched. Its bytes are
//! level-less-walk rows and therefore cannot be relabelled as the selected
//! cell's rows. This module writes `results/chosen-trades.bin`; a reader that
//! finds only the legacy path refuses and asks for an exact rerun.
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
//!
//! A read-only [`Trades::of_run`] also opens/indexes `runs.bin` and
//! `detail-sets.bin` to prove the parent and exact cardinality. A fresh HTTP
//! lookup is therefore O(all trade rows + all ledger rows + all receipts) to
//! open, then one O(1) identity probe and O(selected trades) to read. It is not
//! an end-to-end O(1) latency claim.
//!
//! **UNVERIFIED as a measured bound.** No bench in this workspace
//! times this, so the shape above is read from the source rather
//! than measured. `CLAUDE.md` §3 rule 6.

use crate::results::Refusal;
pub use costs::fill::Direction;
use indicators::IST_OFFSET_MICROS;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

/// Names the format on disk. A file that does not begin with this is not one of
/// these and is refused rather than parsed.
const MAGIC: [u8; 8] = *b"BRUTEXCT";

/// The layout below. A reader that does not know this number refuses rather
/// than guessing at a stride it cannot verify.
const VERSION: u32 = 1;

/// Magic, version and a reserved word.
const HEADER: u64 = 16;
const HEADER_BYTES: usize = 16;
const _: () = assert!(HEADER_BYTES as u64 == HEADER);

/// One row. Fixed, so `index -> byte offset` is a multiply and the lookup is
/// O(1) — `CLAUDE.md` §3 rule 4.
///
/// **UNVERIFIED as a measured bound.** No bench in this workspace
/// times this, so the shape above is read from the source rather
/// than measured. `CLAUDE.md` §3 rule 6.
pub(crate) const STRIDE: u64 = 136;
const STRIDE_BYTES: usize = 136;
const _: () = assert!(STRIDE_BYTES as u64 == STRIDE);

/// The last eight bytes of a row are a seal over the first 128.
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
/// `32 + 4 + 1 + 3 + (11 * 8)`.
const _: () = assert!(PAYLOAD_BYTES == 32 + 4 + 1 + 3 + (11 * 8));

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
    /// The selected position direction. It is not inferred from the run hash:
    /// the present frequency-sweep identity is deliberately undirected.
    pub direction: costs::fill::Direction,
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
    ///
    /// **UNVERIFIED as a measured bound.** No bench in this workspace
    /// times this, so the shape above is read from the source rather
    /// than measured. `CLAUDE.md` §3 rule 6.
    pub entry_micros: i64,
    /// When the position was CLOSED, same units and same reason.
    ///
    /// Kept alongside the entry rather than derived from `exit_bar - entry_bar`,
    /// because the two bars can sit either side of a weekend or a holiday and
    /// the difference in BARS is not the difference in TIME. A holding period
    /// measured in bars would call a Friday-to-Monday trade the same length as
    /// a Monday-to-Tuesday one.
    pub exit_micros: i64,
    /// Maximum adverse excursion from entry, in parts per million.
    pub adverse_ppm: i64,
    /// The same adverse excursion in paisa per unit.
    pub adverse_paisa: i64,
    /// Maximum favourable excursion from entry, in parts per million.
    pub favourable_ppm: i64,
    /// The same favourable excursion in paisa per unit.
    pub favourable_paisa: i64,
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
        put(&[direction_byte(self.direction)], &mut at); // 36   1
        put(&[0_u8; 3], &mut at); // 37   3  reserved, keeps the i64s aligned
        put(&self.signal_bar.to_le_bytes(), &mut at); // 40   8
        put(&self.entry_bar.to_le_bytes(), &mut at); // 48   8
        put(&self.exit_bar.to_le_bytes(), &mut at); // 56   8
        put(&self.best.to_le_bytes(), &mut at); // 64   8
        put(&self.worst.to_le_bytes(), &mut at); // 72   8
        put(&self.entry_micros.to_le_bytes(), &mut at); // 80   8
        put(&self.exit_micros.to_le_bytes(), &mut at); // 88   8
        put(&self.adverse_ppm.to_le_bytes(), &mut at); // 96   8
        put(&self.adverse_paisa.to_le_bytes(), &mut at); // 104  8
        put(&self.favourable_ppm.to_le_bytes(), &mut at); // 112  8
        put(&self.favourable_paisa.to_le_bytes(), &mut at); // 120  8 -> payload 128
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
    ///
    /// # Errors
    ///
    /// Refuses an unknown direction or non-zero reserved byte rather than
    /// assigning either a meaning this format never defined.
    #[allow(
        clippy::indexing_slicing,
        reason = "the same compile-time offsets `to_bytes` writes at, over an \
                  array of the same asserted length."
    )]
    pub fn from_bytes(raw: &[u8; STRIDE_BYTES]) -> Result<Self, Refusal> {
        let mut identity = [0_u8; 32];
        identity.copy_from_slice(&raw[0..32]);
        let mut four = [0_u8; 4];
        let mut eight = [0_u8; 8];
        four.copy_from_slice(&raw[32..36]);
        let seq = u32::from_le_bytes(four);
        let direction = direction_of(raw[36])?;
        if raw[37..40] != [0_u8; 3] {
            return Err("a chosen-trade row has non-zero reserved bytes; this build will not invent their meaning".to_owned());
        }
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
        eight.copy_from_slice(&raw[96..104]);
        let adverse_ppm = i64::from_le_bytes(eight);
        eight.copy_from_slice(&raw[104..112]);
        let adverse_paisa = i64::from_le_bytes(eight);
        eight.copy_from_slice(&raw[112..120]);
        let favourable_ppm = i64::from_le_bytes(eight);
        eight.copy_from_slice(&raw[120..128]);
        let favourable_paisa = i64::from_le_bytes(eight);
        Ok(Self {
            identity,
            seq,
            direction,
            signal_bar,
            entry_bar,
            exit_bar,
            best,
            worst,
            entry_micros,
            exit_micros,
            adverse_ppm,
            adverse_paisa,
            favourable_ppm,
            favourable_paisa,
        })
    }
}

const fn direction_byte(direction: costs::fill::Direction) -> u8 {
    match direction {
        costs::fill::Direction::Long => 1,
        costs::fill::Direction::Short => 2,
    }
}

fn direction_of(byte: u8) -> Result<costs::fill::Direction, Refusal> {
    match byte {
        1 => Ok(costs::fill::Direction::Long),
        2 => Ok(costs::fill::Direction::Short),
        _ => Err(format!(
            "chosen-trade direction byte {byte} is unknown; only 1=long and 2=short are defined"
        )),
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

/// One open-time pass's unambiguous block map and first global integrity failure.
struct Indexed {
    blocks: std::collections::HashMap<[u8; 32], Block>,
    write_refusal: Option<Refusal>,
}

/// The per-trade file.
pub struct Trades {
    file: File,
    path: PathBuf,
    /// Store root, kept so a read-only detail lookup can prove its ledger
    /// parent exists before exposing the block.
    root: PathBuf,
    /// Writers may inspect prepared rows before their ledger commit exists;
    /// public readers may not.
    require_parent: bool,
    blocks: std::collections::HashMap<[u8; 32], Block>,
    /// How far the block index has read, in bytes from the start of the file.
    ///
    /// `absorb_new_rows` resumes here rather than rescanning, so a writer that
    /// takes the lock and finds another process has appended pays only for what
    /// arrived since. Zero on the one-writer path, which is every path today.
    scanned: u64,
    /// First integrity failure found while indexing whole rows.
    ///
    /// Unambiguous committed blocks remain readable. A writer, however, must not
    /// append beyond unexplained bytes or promote a prepared block beside them,
    /// so both append and durability confirmation refuse while this is present.
    write_refusal: Option<Refusal>,
}

impl Trades {
    /// Where the file lives, beside the ledger and the frontier.
    #[must_use]
    pub fn path(root: &Path) -> PathBuf {
        root.join("results").join("chosen-trades.bin")
    }

    /// The old level-less policy file. It is named only for an explicit refusal
    /// and is never opened or decoded by this module.
    #[must_use]
    pub fn legacy_path(root: &Path) -> PathBuf {
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
        Self::open_read_bounded(root, u64::MAX)
    }

    /// Opens read-only while refusing a file larger than `max_bytes` before
    /// rebuilding its identity index.
    ///
    /// [`Self::open_read`] keeps the unrestricted CLI contract. HTTP callers
    /// pass a finite ceiling so one request cannot turn an arbitrarily large
    /// history into an arbitrarily large blocking task.
    ///
    /// # Errors
    ///
    /// In addition to [`Self::open_read`]'s refusals, names the measured byte
    /// length when it exceeds `max_bytes`. No row is indexed in that case.
    pub fn open_read_bounded(root: &Path, max_bytes: u64) -> Result<Self, Refusal> {
        let path = Self::path(root);
        let mut file = match OpenOptions::new().read(true).open(&path) {
            Ok(file) => file,
            Err(why)
                if why.kind() == std::io::ErrorKind::NotFound
                    && Self::legacy_path(root).exists() =>
            {
                return Err(format!(
                    "{} is absent, while legacy {} exists. Legacy trade version 2 records the level-less walk, not the selected exit-grid variant, so it is never reinterpreted. Re-run the exact inputs to materialize chosen-grid-v1 rows",
                    path.display(),
                    Self::legacy_path(root).display()
                ));
            }
            Err(why) => return Err(format!("{} could not be opened: {why}", path.display())),
        };
        let len = file
            .metadata()
            .map_err(|why| format!("{} could not be measured: {why}", path.display()))?
            .len();
        if len > max_bytes {
            return Err(format!(
                "{} is {len} bytes; this reader's hard index ceiling is {max_bytes} bytes. No partial chosen-trade index was built",
                path.display()
            ));
        }
        check_header(&mut file, &path, len)?;
        let Indexed {
            blocks,
            write_refusal,
        } = index_of(&mut file, len)?;
        // `index_of` walks every WHOLE row, so the cursor sits at the end of the
        // last one -- and a ragged tail cannot reach here, `check_header`
        // refuses it.
        let scanned =
            HEADER.saturating_add((len.saturating_sub(HEADER) / STRIDE).saturating_mul(STRIDE));
        Ok(Self {
            file,
            path,
            root: root.to_path_buf(),
            require_parent: true,
            blocks,
            scanned,
            write_refusal,
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
                root: root.to_path_buf(),
                require_parent: false,
                blocks: std::collections::HashMap::new(),
                // A FRESH FILE HOLDS ONLY ITS HEADER, so the index has read
                // everything there is.
                scanned: HEADER,
                write_refusal: None,
            });
        }
        check_header(&mut file, &path, len)?;
        let Indexed {
            blocks,
            write_refusal,
        } = index_of(&mut file, len)?;
        // `index_of` walks every WHOLE row, so the cursor sits at the end of the
        // last one -- and a ragged tail cannot reach here, `check_header`
        // refuses it.
        let scanned =
            HEADER.saturating_add((len.saturating_sub(HEADER) / STRIDE).saturating_mul(STRIDE));
        Ok(Self {
            file,
            path,
            root: root.to_path_buf(),
            require_parent: false,
            blocks,
            scanned,
            write_refusal,
        })
    }

    /// Where a run's rows are, or `None`. **O(1)** — one hash probe.
    ///
    /// **UNVERIFIED as a measured bound.** No bench in this workspace
    /// times this, so the shape above is read from the source rather
    /// than measured. `CLAUDE.md` §3 rule 6.
    #[must_use]
    pub fn block(&self, identity: &[u8; 32]) -> Option<Block> {
        self.blocks.get(identity).copied()
    }

    /// Whether this run's trades are already recorded. **O(1)**.
    ///
    /// Unlike its predecessor, this answers correctly after a restart, because
    /// the index is rebuilt from rows that carry their own identity.
    ///
    /// **UNVERIFIED as a measured bound.** No bench in this workspace
    /// times this, so the shape above is read from the source rather
    /// than measured. `CLAUDE.md` §3 rule 6.
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
        for (at, row) in rows.iter().enumerate() {
            let expected_seq = u32::try_from(at).map_err(|_| {
                "a chosen-trade block has more rows than its u32 sequence field can name".to_owned()
            })?;
            if row.identity != first_row.identity
                || row.direction != first_row.direction
                || row.seq != expected_seq
            {
                return Err(format!(
                    "chosen-trade row {at} does not belong to one contiguous run/direction/sequence block (expected identity {}, direction {}, seq {expected_seq}; got identity {}, direction {}, seq {})",
                    hex32(&first_row.identity),
                    first_row.direction,
                    hex32(&row.identity),
                    row.direction,
                    row.seq
                ));
            }
        }
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
                count: u64::try_from(rows.len()).unwrap_or(u64::MAX),
            },
        );
        self.scanned = at.saturating_add(u64::try_from(buffer.len()).unwrap_or(u64::MAX));
        Ok(u64::try_from(rows.len()).unwrap_or(u64::MAX))
    }

    /// Repeats the child's durability barrier before an exact prepared block
    /// is promoted to a committed result set.
    ///
    /// # Errors
    ///
    /// Names a shared-lock, durability-barrier, or unlock failure. A caller
    /// must not promote the child to the public ledger after any such refusal.
    pub fn confirm_durable(&mut self) -> Result<(), Refusal> {
        self.file.lock_shared().map_err(|why| {
            format!(
                "{} could not be locked for syncing: {why}",
                self.path.display()
            )
        })?;
        let synced = self.absorb_new_rows().and_then(|()| {
            self.file
                .sync_all()
                .map_err(|why| format!("{} could not be synced: {why}", self.path.display()))
        });
        let released = self.file.unlock().map_err(|why| {
            format!(
                "{} could not be unlocked after syncing: {why}",
                self.path.display()
            )
        });
        match (synced, released) {
            (Ok(()), Ok(())) => Ok(()),
            (Err(why), _) | (Ok(()), Err(why)) => Err(why),
        }
    }

    /// Absorbs blocks another writer has appended since this handle last looked.
    ///
    /// O(rows appended by others), which is zero on the one-writer path.
    /// Bring a HELD handle up to date, in O(rows appended since it was opened).
    ///
    /// # The scan this exists to stop repeating
    ///
    /// `open_read_bounded` walks every row to build the identity → block index,
    /// and `/trades.json` opened a fresh handle on EVERY request — so a request
    /// for one run's twenty trades rebuilt the index over every trade ever
    /// recorded. `index_of`'s own comment measured the shape and named the fix:
    /// *"A `BufReader` does not change the O(rows) walk — only a persisted or
    /// cached index does that, and it is the right fix."*
    ///
    /// The incremental machinery was already here. [`Self::absorb_new_rows`]
    /// resumes from `self.scanned` rather than rescanning, precisely so a writer
    /// that appended since this handle opened costs O(delta) — it was simply
    /// never available to a reader, because a reader never kept its handle.
    ///
    /// # Why this is a refresh and not a reopen
    ///
    /// A caller holding this across requests sees rows another process appended
    /// without paying for the ones it already indexed. The bounded-open ceiling
    /// still applies at open; this only ever moves `scanned` forward, and a file
    /// that SHRANK is refused rather than reindexed — append-only history was
    /// replaced, and silently re-walking it would hide that.
    ///
    /// # Errors
    ///
    /// The same refusals `absorb_new_rows` makes: a shrunken file, a ragged
    /// tail, an unreadable row, or a duplicate identity whose blocks are not
    /// contiguous.
    pub fn refresh(&mut self) -> Result<(), Refusal> {
        self.absorb_new_rows()
    }

    fn absorb_new_rows(&mut self) -> Result<(), Refusal> {
        self.refuse_integrity_failure_for_write()?;
        let len = self
            .file
            .metadata()
            .map_err(|why| format!("{} could not be measured: {why}", self.path.display()))?
            .len();
        if len < self.scanned {
            return Err(format!(
                "{} shrank from byte {} to {len} while this handle was open. Append-only history was replaced and no chosen trades were written",
                self.path.display(),
                self.scanned
            ));
        }
        if len < HEADER || !len.saturating_sub(HEADER).is_multiple_of(STRIDE) {
            return Err(format!(
                "{} has length {len}, which is not a {HEADER}-byte header plus whole {STRIDE}-byte chosen-trade rows. A torn tail is never skipped or extended",
                self.path.display()
            ));
        }
        let mut raw = [0_u8; STRIDE_BYTES];
        while self.scanned.saturating_add(STRIDE) <= len {
            let at = self.scanned;
            self.file
                .seek(SeekFrom::Start(at))
                .and_then(|_| self.file.read_exact(&mut raw))
                .map_err(|why| format!("the trade row at byte {at} could not be read: {why}"))?;
            let index = at.saturating_sub(HEADER) / STRIDE;
            if !Row::seal_matches(&raw) {
                self.write_refusal.get_or_insert_with(|| {
                    format!("whole chosen-trade row {index} whose integrity seal failed")
                });
                return Err(format!(
                    "chosen-trade row {index}, appended after this handle opened, does not match its seal. No later row was absorbed and nothing was written"
                ));
            }
            let row = match Row::from_bytes(&raw) {
                Ok(row) => row,
                Err(why) => {
                    let refusal = format!(
                        "chosen-trade row {index} is sealed but its schema is invalid: {why}"
                    );
                    self.write_refusal.get_or_insert(refusal.clone());
                    return Err(format!(
                        "{refusal}. No later row was absorbed and nothing was written"
                    ));
                }
            };
            match self.blocks.get_mut(&row.identity) {
                Some(block) if block.first.saturating_add(block.count) == index => {
                    block.count = block.count.saturating_add(1);
                }
                Some(_) => {
                    let refusal = format!(
                        "{} gained a non-contiguous duplicate chosen-trade block for run {} while this handle was open. The index is ambiguous and nothing was written",
                        self.path.display(),
                        hex32(&row.identity)
                    );
                    self.write_refusal.get_or_insert(refusal.clone());
                    return Err(refusal);
                }
                None => {
                    self.blocks.insert(
                        row.identity,
                        Block {
                            first: index,
                            count: 1,
                        },
                    );
                }
            }
            self.scanned = at.saturating_add(STRIDE);
        }
        Ok(())
    }

    /// Refuses every writer transition after a bad seal, invalid schema or
    /// non-contiguous identity block. Read-only handles can still diagnose
    /// unaffected blocks.
    fn refuse_integrity_failure_for_write(&self) -> Result<(), Refusal> {
        if let Some(why) = &self.write_refusal {
            return Err(format!(
                "{} contains {why}. Append-only history is damaged or ambiguous, so no row may be appended and no prepared block may be promoted until a reviewed forensic repair installs a replacement",
                self.path.display(),
            ));
        }
        Ok(())
    }

    /// Every row of one run, in order. Inside an already-open writer handle this
    /// is **O(count)** after an O(1) lookup. A read-only handle additionally
    /// opens/indexes the complete parent ledger and receipt sidecar for commit
    /// proof, so a fresh end-to-end lookup is not O(1).
    ///
    /// **UNVERIFIED as a measured bound.** No bench in this workspace
    /// times this, so the shape above is read from the source rather
    /// than measured. `CLAUDE.md` §3 rule 6.
    ///
    /// # Errors
    ///
    /// Names commit/receipt, seek, read, foreign-identity, seal-damage and count
    /// failures. A selected block is all-or-nothing: valid rows accumulated
    /// before a bad one are retained only for the refusal's diagnostic count
    /// and are never returned as a complete trade list.
    pub fn of_run(&mut self, identity: &[u8; 32]) -> Result<Vec<Row>, Refusal> {
        let receipt = if self.require_parent {
            crate::result_set::committed_receipt(&self.root, identity)?
        } else {
            None
        };
        self.read_against_receipt(identity, receipt)
    }

    /// Reads one public block against the caller's already-verified receipt.
    ///
    /// This is the HTTP transaction door: the caller first obtains exactly one
    /// snapshot from [`crate::result_set::committed_receipt`], then opens this
    /// file afresh and passes that same value here for count, direction and row
    /// reconciliation. Re-reading the receipt after opening the child would let
    /// metadata from one instant label rows indexed at another.
    ///
    /// A `None` proof never produces a clean committed-empty answer. An orphan
    /// block is refused here; a genuinely absent block returns an empty vector
    /// so the caller can publish an explicit *uncommitted/absent* reason.
    ///
    /// # Errors
    ///
    /// Refuses writer handles, a foreign receipt identity, any receipt/index
    /// count mismatch, an orphan block, direction disagreement, foreign row,
    /// invalid schema or damaged seal. No prefix is returned.
    pub fn of_run_against_receipt(
        &mut self,
        identity: &[u8; 32],
        receipt: Option<crate::result_set::Receipt>,
    ) -> Result<Vec<Row>, Refusal> {
        if !self.require_parent {
            return Err(
                "an already-verified committed receipt can only be applied to a read-only chosen-trade handle"
                    .to_owned(),
            );
        }
        self.read_against_receipt(identity, receipt)
    }

    fn read_against_receipt(
        &mut self,
        identity: &[u8; 32],
        receipt: Option<crate::result_set::Receipt>,
    ) -> Result<Vec<Row>, Refusal> {
        let block = self.block(identity);

        if let Some(receipt) = receipt {
            if receipt.identity != *identity {
                return Err(format!(
                    "receipt for run {} was applied to chosen-trade run {}. No rows are exposed across identities",
                    hex32(&receipt.identity),
                    hex32(identity)
                ));
            }
            let actual = block.map_or(0, |held| held.count);
            if actual != receipt.trade_rows {
                return Err(format!(
                    "run {} commits a receipt for {} chosen-trade row(s), but chosen-trades.bin indexes {actual}. The result set is incomplete or damaged. No partial trade list is exposed",
                    hex32(identity),
                    receipt.trade_rows
                ));
            }
        }
        let Some(block) = block else {
            return Ok(Vec::new());
        };
        // THE LEDGER ROW IS THE COMMIT MARKER. A trade block is prepared and
        // synced first, then the parent is appended. Without this check a
        // direct `/trades.json?identity=...` request could expose an orphan left
        // by a killed writer. A writer handle skips the check so an exact rerun
        // can inspect and verify that prepared block before committing it.
        if self.require_parent && receipt.is_none() {
            return Err(format!(
                "run {} has prepared trade rows but no results-ledger row. The run did not commit; the rows are hidden. Re-run the exact same inputs to verify and finish it.",
                hex32(identity)
            ));
        }
        let at = HEADER.saturating_add(block.first.saturating_mul(STRIDE));
        self.file
            .seek(SeekFrom::Start(at))
            .map_err(|why| format!("{} could not be seeked: {why}", self.path.display()))?;
        let mut out = Vec::with_capacity(usize::try_from(block.count).unwrap_or(0));
        let mut raw = [0_u8; STRIDE_BYTES];
        let mut foreign = 0_u64;
        let mut damaged = 0_u64;
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
                let row = Row::from_bytes(&raw).map_err(|why| {
                    format!(
                        "run {} has a sealed but invalid chosen-trade row: {why}. No partial trade list is exposed",
                        hex32(identity)
                    )
                })?;
                if row.identity == *identity {
                    if let Some(receipt) = receipt
                        && row.direction != receipt.direction
                    {
                        return Err(format!(
                            "run {} commits selected direction {}, but chosen-trade row {} says {}. No partial trade list is exposed",
                            hex32(identity),
                            receipt.direction,
                            row.seq,
                            row.direction
                        ));
                    }
                    out.push(row);
                } else {
                    foreign = foreign.saturating_add(1);
                }
            } else {
                damaged = damaged.saturating_add(1);
            }
        }
        let own = u64::try_from(out.len()).unwrap_or(u64::MAX);
        if self.require_parent && (foreign > 0 || damaged > 0 || own != block.count) {
            return Err(format!(
                "run {} has a committed receipt for a {}-row trade block, but reading that block found {} valid own row(s), {foreign} foreign row(s), and {damaged} seal-damaged row(s). No partial trade list is exposed",
                hex32(identity),
                block.count,
                out.len()
            ));
        }
        if foreign > 0 || damaged > 0 {
            // NAMED, NOT SILENTLY SHORTENED. A list that is shorter than the
            // block claimed is a fact the reader needs; dropping the rows and
            // saying nothing is the same silence this whole guard exists to
            // end.
            return Err(format!(
                "the block for run {} spans {foreign} foreign row(s) and \
                 {damaged} seal-damaged row(s), so its recorded contents are \
                 not whole. {} row(s) that are genuinely this run's were kept \
                 in memory and are not returned here, because a partial answer \
                 under a whole answer's name is what §4 refuses. The file is unchanged.",
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
    if head.get(12..16) != Some(&[0_u8; 4]) {
        return Err(format!(
            "{} has non-zero reserved header bytes. This chosen-trade version assigns them no meaning and refuses rather than guessing",
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
    // `sync_all`, because a freshly-created empty detail has only this header.
    // Its length/metadata is part of the receipted zero-row state and must be
    // durable before the ledger marker can advertise it.
    file.sync_all()
        .map_err(|why| format!("{} could not be synced: {why}", path.display()))?;
    Ok(())
}

/// Rebuilds the block index in one pass.
///
/// This is the whole reason the identity is on every row. Its predecessor set
/// `HashMap::new()` here and on the create path, so every lookup after a restart
/// answered "no such run" — and the module's own header claimed an O(1) find.
/// A failed seal is omitted from the healthy block index but retained as a
/// writer-stopping row number, so read-only availability does not let a fresh
/// process append beyond unexplained history.
///
/// **UNVERIFIED as a measured bound.** No bench in this workspace
/// times this, so the shape above is read from the source rather
/// than measured. `CLAUDE.md` §3 rule 6.
fn index_of(file: &mut File, len: u64) -> Result<Indexed, Refusal> {
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
    // ONE SYSCALL PER 8 KiB, NOT ONE PER 136-BYTE ROW.
    //
    // This loop read straight from the `File`, so `read_exact` was one `read(2)`
    // per row -- and `open_read` runs it on EVERY `/trades.json` request, which
    // means a request for one run's twenty trades walked every trade ever
    // recorded, one syscall at a time. An O(1) audit measured the shape as
    // "O(rows), 1 read_exact syscall/row".
    //
    // A `BufReader` does not change the O(rows) walk -- only a persisted or
    // cached index does that, and it is the right fix -- but it divides the
    // syscall count by the rows that fit in a buffer: at 136 bytes and the
    // default 8 KiB that is about 60 rows per read. The borrow is scoped so the
    // `File`'s own cursor is free afterwards, and every later reader seeks
    // explicitly before it reads.
    let mut buffered = std::io::BufReader::new(&mut *file);
    let mut raw = [0_u8; STRIDE_BYTES];
    let mut write_refusal = None;
    for index in 0..count {
        buffered.read_exact(&mut raw).map_err(|why| {
            format!("chosen-trade row {index} could not be read while indexing: {why}")
        })?;
        if !Row::seal_matches(&raw) {
            write_refusal.get_or_insert_with(|| {
                format!("whole chosen-trade row {index} whose integrity seal failed")
            });
            continue;
        }
        let mut identity = [0_u8; 32];
        identity.copy_from_slice(raw.get(..32).unwrap_or(&[0_u8; 32]));
        if let Err(why) = Row::from_bytes(&raw) {
            write_refusal.get_or_insert_with(|| {
                format!("chosen-trade row {index} is sealed but its schema is invalid: {why}")
            });
        }

        match blocks.get_mut(&identity) {
            Some(block) if block.first.saturating_add(block.count) == index => {
                block.count = block.count.saturating_add(1);
            }
            Some(_) => {
                write_refusal.get_or_insert_with(|| {
                    format!(
                        "chosen-trade row {index} starts a non-contiguous duplicate block for run {}",
                        hex32(&identity)
                    )
                });
            }
            None => {
                blocks.insert(
                    identity,
                    Block {
                        first: index,
                        count: 1,
                    },
                );
            }
        }
    }
    Ok(Indexed {
        blocks,
        write_refusal,
    })
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
    use super::{Block, PAYLOAD_BYTES, Row, STRIDE_BYTES, Trades, seal_of};
    use crate::result_set::{Receipt, TradePolicy};

    fn row(identity: [u8; 32], seq: u32) -> Row {
        Row {
            identity,
            seq,
            direction: costs::fill::Direction::Short,
            signal_bar: 11,
            entry_bar: 12,
            exit_bar: 40,
            best: 4_250,
            worst: -1_375,
            // 2024-01-15 09:20 IST as UTC micros -- a Monday, so the weekday
            // arithmetic below has a known answer to be wrong about.
            entry_micros: 1_705_290_600_000_000,
            exit_micros: 1_705_292_400_000_000,
            adverse_ppm: 12_500,
            adverse_paisa: 125,
            favourable_ppm: 42_500,
            favourable_paisa: 425,
        }
    }

    fn reseal(raw: &mut [u8; STRIDE_BYTES]) {
        let seal = seal_of(raw);
        raw.get_mut(PAYLOAD_BYTES..)
            .expect("the row owns its seal suffix")
            .copy_from_slice(&seal);
        assert!(Row::seal_matches(raw), "the adversarial row is whole");
    }

    fn append_raw(path: &std::path::Path, rows: &[[u8; STRIDE_BYTES]]) {
        use std::io::Write as _;

        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(path)
            .expect("the adversarial peer opens chosen trades");
        for raw in rows {
            file.write_all(raw).expect("one whole row is appended");
        }
        file.sync_all().expect("the adversarial rows are durable");
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
        assert_eq!(
            Row::from_bytes(&raw).expect("valid row"),
            want,
            "every field, unchanged"
        );
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
    ///
    /// This test is the proof --
    /// `cli::trades::a_reopened_file_still_finds_the_run_it_was_given` -- and what
    /// it establishes is that the index SURVIVES a reopen, which is the half the
    /// replaced store got wrong. It says nothing about the probe's cost; no bench
    /// in this workspace times it.
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

        // A writer handle deliberately may inspect a PREPARED block before its
        // ledger parent exists: that is how an exact rerun resumes a killed
        // commit. This test is about rebuilding the index, so use that handle;
        // the public read-only refusal is pinned separately below.
        let mut reopened = Trades::open(&dir).expect("reopens for recovery");
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
        reopened
            .confirm_durable()
            .expect("healthy contiguous blocks carry no write poison after reopen");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_external_receipt_snapshot_is_read_only_and_cannot_cross_identities() {
        let dir = std::env::temp_dir().join(format!(
            "brutex-trades-receipt-snapshot-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let identity = [0x31; 32];
        let mut writer = Trades::open(&dir).expect("writer");
        writer
            .append_all(&[row(identity, 0)])
            .expect("prepared row");
        let proof = Receipt {
            identity,
            frontier_rows: 0,
            trade_rows: 1,
            direction: costs::fill::Direction::Short,
            trade_policy: TradePolicy::ChosenGridV1,
        };
        assert!(
            writer
                .of_run_against_receipt(&identity, Some(proof))
                .expect_err("a writer is not a public snapshot reader")
                .contains("read-only chosen-trade handle")
        );
        drop(writer);

        let mut reader = Trades::open_read(&dir).expect("fresh read-only child");
        let foreign = Receipt {
            identity: [0x32; 32],
            ..proof
        };
        let why = reader
            .of_run_against_receipt(&identity, Some(foreign))
            .expect_err("one identity cannot borrow another run's receipt");
        assert!(why.contains(&"32".repeat(32)), "foreign proof: {why}");
        assert!(why.contains(&"31".repeat(32)), "requested run: {why}");
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

    /// Duplicate rejection is repeated after refreshing disk state under the
    /// exclusive lock. Two handles opened before either write cannot both
    /// publish the same run.
    #[test]
    fn two_stale_handles_cannot_append_the_same_chosen_trade_identity() {
        let dir = std::env::temp_dir().join(format!(
            "brutex-trades-stale-duplicate-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let identity = [0x41; 32];
        let mut first = Trades::open(&dir).expect("first handle");
        let mut stale = Trades::open(&dir).expect("stale handle");

        first
            .append_all(&[row(identity, 0), row(identity, 1)])
            .expect("first block");
        let why = stale
            .append_all(&[row(identity, 0)])
            .expect_err("the stale handle must refresh before duplicate rejection");
        assert!(why.contains("already recorded"), "{why}");

        drop(first);
        drop(stale);
        let mut reopened = Trades::open(&dir).expect("whole file");
        assert_eq!(reopened.of_run(&identity).expect("one block").len(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A stale writer must not append after a torn tail left by a crashed peer.
    /// Doing so would shift every later fixed-stride row permanently.
    #[test]
    fn a_stale_handle_refuses_to_extend_a_ragged_chosen_trade_tail() {
        use std::io::Write as _;

        let dir = std::env::temp_dir().join(format!(
            "brutex-trades-stale-ragged-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let mut stale = Trades::open(&dir).expect("stale handle");
        let path = Trades::path(&dir);
        let before = std::fs::metadata(&path).expect("header").len();
        let mut crashed = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .expect("crashed peer handle");
        crashed.write_all(&[1, 2, 3]).expect("torn prefix");
        crashed
            .sync_all()
            .expect("make the adversarial tail visible");

        let why = stale
            .append_all(&[row([0x42; 32], 0)])
            .expect_err("a ragged tail is never skipped");
        assert!(why.contains("torn tail"), "{why}");
        assert_eq!(
            std::fs::metadata(&path).expect("unchanged tail").len(),
            before + 3,
            "the refusal neither extends nor silently repairs append-only history"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A whole-stride row with a bad seal is corruption, not padding a stale
    /// writer may walk past before appending another run.
    #[test]
    fn a_stale_handle_refuses_to_extend_past_a_bad_sealed_trade_row() {
        use std::io::Write as _;

        let dir = std::env::temp_dir().join(format!(
            "brutex-trades-stale-bad-seal-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let mut stale = Trades::open(&dir).expect("stale handle");
        let path = Trades::path(&dir);
        let mut corrupt = row([0x43; 32], 0).to_bytes();
        corrupt[40] ^= 0x80;
        let mut crashed = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .expect("peer handle");
        crashed.write_all(&corrupt).expect("whole corrupt row");
        crashed
            .sync_all()
            .expect("make the adversarial row visible");

        let why = stale
            .append_all(&[row([0x44; 32], 0)])
            .expect_err("a bad seal cannot be absorbed");
        assert!(why.contains("does not match its seal"), "{why}");
        assert_eq!(
            std::fs::metadata(&path).expect("no extension").len(),
            super::HEADER + super::STRIDE,
            "nothing was appended past the corrupt row"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A process restart must retain a whole bad-seal row as a writer-stopping
    /// fact rather than skipping it and appending a new block after it.
    #[test]
    fn a_reopened_writer_refuses_to_extend_past_a_bad_sealed_trade_row() {
        use std::io::Write as _;

        let dir = std::env::temp_dir().join(format!(
            "brutex-trades-reopened-bad-seal-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let path = Trades::path(&dir);
        let identity = [0x45; 32];
        let kept = row(identity, 0);
        {
            let mut store = Trades::open(&dir).expect("a fresh file opens");
            store
                .append_all(&[kept])
                .expect("the healthy prefix appends");
        }

        let mut corrupt = row([0x46; 32], 0).to_bytes();
        *corrupt.get_mut(40).expect("a sealed payload byte") ^= 0x80;
        let mut crashed = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .expect("the crashed peer opens");
        crashed
            .write_all(&corrupt)
            .expect("one whole corrupt row lands");
        crashed.sync_all().expect("the corrupt row is durable");
        drop(crashed);
        let before = std::fs::metadata(&path)
            .expect("the file has metadata")
            .len();

        let mut reopened = Trades::open(&dir).expect("healthy blocks remain indexable");
        assert_eq!(
            reopened
                .of_run(&identity)
                .expect("the healthy block remains readable"),
            vec![kept]
        );
        for why in [
            reopened
                .append_all(&[row([0x47; 32], 0)])
                .expect_err("a fresh writer cannot append around corruption"),
            reopened
                .confirm_durable()
                .expect_err("corrupt history cannot be promoted as durable"),
        ] {
            assert!(
                why.contains("row 1") && why.contains("integrity seal failed"),
                "{why}"
            );
        }
        assert_eq!(
            std::fs::metadata(&path)
                .expect("the refusal leaves metadata")
                .len(),
            before,
            "neither refusal extends or rewrites append-only history"
        );
        assert!(
            Trades::open_read(&dir).is_ok(),
            "a read-only handle still opens so unaffected committed runs can be diagnosed"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_sealed_invalid_trade_on_fresh_reopen_keeps_the_healthy_prefix_but_poisons_writes() {
        let dir = std::env::temp_dir().join(format!(
            "brutex-trades-reopened-schema-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let path = Trades::path(&dir);
        let kept = row([0x51; 32], 0);
        {
            let mut store = Trades::open(&dir).expect("a fresh file opens");
            store.append_all(&[kept]).expect("the healthy prefix");
        }

        let mut invalid = row([0x52; 32], 0).to_bytes();
        *invalid.get_mut(36).expect("the direction byte") = 9;
        reseal(&mut invalid);
        append_raw(&path, &[invalid]);
        let before = std::fs::read(&path).expect("the adversarial file is readable");

        let mut reopened = Trades::open(&dir).expect("a row defect does not hide its prefix");
        assert_eq!(
            reopened
                .of_run(&kept.identity)
                .expect("the verified prefix remains readable"),
            vec![kept]
        );
        for why in [
            reopened
                .append_all(&[row([0x53; 32], 0)])
                .expect_err("a different identity cannot append past invalid schema"),
            reopened
                .confirm_durable()
                .expect_err("invalid schema cannot be promoted as durable"),
        ] {
            assert!(
                why.contains("row 1")
                    && why.contains("schema is invalid")
                    && why.contains("direction byte 9"),
                "{why}"
            );
        }
        assert_eq!(
            std::fs::read(&path).expect("the refusal leaves the file readable"),
            before,
            "neither refusal rewrites or extends append-only bytes"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_a_b_a_trade_file_on_fresh_reopen_never_spans_b_and_poisons_writes() {
        let dir = std::env::temp_dir().join(format!(
            "brutex-trades-reopened-a-b-a-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let path = Trades::path(&dir);
        let first_a = row([0x61; 32], 0);
        let b = row([0x62; 32], 0);
        {
            let mut store = Trades::open(&dir).expect("a fresh file opens");
            store.append_all(&[first_a]).expect("first A block");
            store.append_all(&[b]).expect("B block");
        }
        append_raw(&path, &[row([0x61; 32], 0).to_bytes()]);
        let before = std::fs::read(&path).expect("the adversarial file is readable");

        let mut reopened = Trades::open(&dir).expect("the healthy prefix remains indexable");
        let Some(Block { first, count }) = reopened.block(&first_a.identity) else {
            panic!("the first A block remains indexed");
        };
        assert_eq!(
            (first, count),
            (0, 1),
            "the first A block must never widen across B"
        );
        let Some(Block { first, count }) = reopened.block(&b.identity) else {
            panic!("the B block remains indexed");
        };
        assert_eq!((first, count), (1, 1));
        assert_eq!(
            reopened.of_run(&first_a.identity).expect("first A reads"),
            vec![first_a],
            "the foreign B row and repeated A row are outside A's first block"
        );
        for why in [
            reopened
                .append_all(&[row([0x63; 32], 0)])
                .expect_err("ambiguous history cannot be extended"),
            reopened
                .confirm_durable()
                .expect_err("ambiguous history cannot be promoted"),
        ] {
            assert!(
                why.contains("row 2") && why.contains("non-contiguous"),
                "{why}"
            );
        }
        assert_eq!(
            std::fs::read(&path).expect("the refusal leaves bytes readable"),
            before,
            "the A,B,A history is diagnosed, never repaired or extended"
        );
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

    #[test]
    fn a_legacy_level_less_file_is_named_and_never_reinterpreted() {
        let dir = std::env::temp_dir().join(format!(
            "brutex-trades-legacy-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("results")).expect("results directory");
        std::fs::write(Trades::legacy_path(&dir), b"legacy-level-less-v2").expect("legacy fixture");

        let why = Trades::open_read(&dir)
            .err()
            .expect("legacy bytes cannot become chosen rows");
        assert!(why.contains("Legacy trade version 2"), "{why}");
        assert!(why.contains("level-less walk"), "{why}");
        assert!(why.contains("Re-run the exact inputs"), "{why}");
        assert!(
            !Trades::path(&dir).exists(),
            "a read must not create the replacement file"
        );
        let _ = std::fs::remove_dir_all(&dir);
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
        assert_eq!(STRIDE_BYTES, 136);
        assert_eq!(row([0_u8; 32], 0).to_bytes().len(), STRIDE_BYTES);
    }

    #[test]
    fn unknown_direction_and_reserved_bytes_are_never_given_a_meaning() {
        let mut unknown_direction = row([8_u8; 32], 0).to_bytes();
        unknown_direction[36] = 9;
        assert!(
            Row::from_bytes(&unknown_direction)
                .expect_err("unknown direction")
                .contains("direction byte 9")
        );

        let mut assigned_row_reserve = row([8_u8; 32], 0).to_bytes();
        assigned_row_reserve[37] = 1;
        assert!(
            Row::from_bytes(&assigned_row_reserve)
                .expect_err("assigned row reserve")
                .contains("reserved")
        );

        let dir = std::env::temp_dir().join(format!(
            "brutex-trades-reserve-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        drop(Trades::open(&dir).expect("fresh header"));
        let mut header = std::fs::read(Trades::path(&dir)).expect("header bytes");
        assert_eq!(header.get(12..16), Some(&[0_u8; 4][..]));
        *header.get_mut(12).expect("reserved header byte") = 1;
        std::fs::write(Trades::path(&dir), header).expect("assigned header fixture");
        assert!(
            Trades::open_read(&dir)
                .err()
                .expect("assigned header reserve")
                .contains("reserved header bytes")
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_bounded_reader_refuses_before_indexing_an_oversized_file() {
        let dir = std::env::temp_dir().join(format!(
            "brutex-trades-bounded-open-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("results")).expect("results directory");
        let file = std::fs::File::create(Trades::path(&dir)).expect("trade fixture");
        file.set_len(17).expect("sparse oversized fixture");
        let why = Trades::open_read_bounded(&dir, 16)
            .err()
            .expect("17 exceeds 16");
        assert!(why.contains("17 bytes"), "measured length: {why}");
        assert!(why.contains("16 bytes"), "hard ceiling: {why}");
        assert!(why.contains("No partial chosen-trade index"), "{why}");
        let _ = std::fs::remove_dir_all(&dir);
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
    ///
    /// **UNVERIFIED as a measured bound.** No bench in this workspace
    /// times this, so the shape above is read from the source rather
    /// than measured. `CLAUDE.md` §3 rule 6.
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
    /// Monday-to-Sunday blocks in the IST calendar.
    ///
    /// The Unix epoch was a Thursday, so adding three to the IST civil-day key
    /// before division moves the boundary to Monday. This matches the trading
    /// week an operator means by Monday through Friday while retaining weekend
    /// slots as ordinary, usually empty, calendar members.
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
    /// Minutes from midnight IST, rounded down to the clock hour.
    ///
    /// Epoch microseconds are the storage representation, not the trading
    /// calendar. Applying the shared IST offset here keeps hour, weekday and
    /// every civil period on the same exchange clock.
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
        const HOUR: i64 = 3_600_000_000;
        let ist_micros = micros.saturating_add(IST_OFFSET_MICROS);
        let days = ist_micros.div_euclid(DAY);
        let (year, month, _day) = telemetry::civil_from_days(days);
        match self {
            Self::Day => days,
            // 1970-01-01 was Thursday. Shift three days so each integer block
            // begins on Monday rather than at the epoch's Thursday.
            Self::Week => days.saturating_add(3).div_euclid(7),
            Self::Month => year.saturating_mul(12).saturating_add(month - 1),
            Self::Quarter => year.saturating_mul(4).saturating_add((month - 1) / 3),
            Self::Half => year.saturating_mul(2).saturating_add((month - 1) / 6),
            Self::Year => year,
            // 1970-01-01 was a THURSDAY, so shifting by 3 puts Monday at 0.
            // `rem_euclid` and not `%`, for the negative side.
            Self::Weekday => days.saturating_add(3).rem_euclid(7),
            Self::Hour => ist_micros.rem_euclid(DAY) / HOUR,
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
///
/// **UNVERIFIED as a measured bound.** No bench in this workspace
/// times this, so the shape above is read from the source rather
/// than measured. `CLAUDE.md` §3 rule 6.
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

    /// 2024-01-15 09:20 IST. A **Monday**, chosen because a weekday
    /// calculation off by one still lands on a real day and looks fine.
    const MONDAY: i64 = 1_705_290_600_000_000;
    const DAY: i64 = 86_400_000_000;

    fn trade(entry: i64, best: i64, worst: i64) -> Row {
        Row {
            identity: [7_u8; 32],
            seq: 0,
            direction: costs::fill::Direction::Long,
            signal_bar: 0,
            entry_bar: 1,
            exit_bar: 2,
            best,
            worst,
            entry_micros: entry,
            exit_micros: entry + 3_600_000_000,
            adverse_ppm: 0,
            adverse_paisa: 0,
            favourable_ppm: 0,
            favourable_paisa: 0,
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

    #[test]
    fn utc_sunday_evening_is_ist_monday_and_every_bucket_uses_ist() {
        // 2024-01-14 20:00 UTC == 2024-01-15 01:30 IST. Dividing the raw epoch
        // first calls this Sunday and hour 20; the exchange calendar calls it
        // Monday and hour 1.
        const SUNDAY_UTC_MONDAY_IST: i64 = 1_705_262_400_000_000;
        assert_eq!(
            Period::Day.bucket(SUNDAY_UTC_MONDAY_IST),
            Period::Day.bucket(MONDAY)
        );
        assert_eq!(Period::Weekday.bucket(SUNDAY_UTC_MONDAY_IST), 0);
        assert_eq!(Period::Hour.bucket(SUNDAY_UTC_MONDAY_IST), 1);
    }

    #[test]
    fn weekly_buckets_begin_on_monday_in_ist() {
        let monday = Period::Week.bucket(MONDAY);
        assert_eq!(Period::Week.bucket(MONDAY + 6 * DAY), monday);
        assert_eq!(Period::Week.bucket(MONDAY - DAY), monday - 1);
        assert_eq!(Period::Week.bucket(MONDAY + 7 * DAY), monday + 1);
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

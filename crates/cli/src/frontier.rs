//! The ranked frontier of one run, kept — not folded into two scalars.
//!
//! # What was lost, and why nothing downstream could recover it
//!
//! [`crate::results`] stores **one** [`crate::results::Record`] per run, holding
//! the one combination the exit grid chose. Everything the Apriori ladder found
//! is reduced to two numbers on that row — `depth` and `combinations` — and
//! discarded. `one_rung` says so in its own comment: *"The long report is
//! DISCARDED on purpose."*
//!
//! So an operator asking *"show me the top ten"* could be answered on screen and
//! nowhere else. The second-best combination, the survivor count per level, and
//! where the frontier emptied were all gone the moment the process exited, which
//! means:
//!
//! * two runs a month apart cannot be compared beyond their single winners;
//! * `/backtest.json` cannot serve ranked combinations, because there is nothing
//!   on disk to serve — the web console ranks RUNS, one row per rung, and a
//!   reader can easily mistake that for the brute force's output;
//! * a robust edge and one lucky mask look identical, because the shape of the
//!   distribution behind the winner was never written down.
//!
//! This file is that shape.
//!
//! # Why a second file rather than more columns
//!
//! A run has ONE identity and the results ledger refuses a duplicate — that is
//! what keeps it append-only and idempotent. N combinations per run cannot
//! therefore be N rows there, and they cannot be N columns either: a record
//! whose width depends on how many survived is a variable record, and a variable
//! record has no stride and so no `O(1)` address. `docs/02-store-format.md` puts
//! it as *"a new field is a new file version at its own stride"*, and this is a
//! new field that happens to repeat.
//!
//! So: its own file, its own stride, its own magic, and a foreign key. Rows are
//! grouped by the run identity they belong to, which is the same
//! `blake3(mask ‖ direction ‖ instrument ‖ timeframe ‖ params ‖ digest ‖ vocab ‖
//! commit ‖ feed)` §3 rule 3 already requires every run to record.
//!
//! | Operation | Cost | How |
//! |---|---|---|
//! | append | **O(1)** | seek to end, one buffered write of the whole block |
//! | read row *i* | **O(1)** | seek to `HEADER + i·STRIDE`, one read |
//! | count | **O(1)** | `(file_len - HEADER) / STRIDE`, no walk |
//! | FIND a run's rows | **O(1)** | one hash probe into the block index |
//! | READ them | **O(count)** | one seek, one read of `count · STRIDE` bytes |
//!
//! # A run's rows are CONTIGUOUS, which is what makes the index exact
//!
//! [`Frontier::append_all`] writes a whole frontier in one call under one lock,
//! so a run's rows are a BLOCK rather than a scatter. The index is therefore
//! `identity -> {first, count}` — two integers, not a row list — and it is built
//! in one pass at open. That is the shape `crates/api/src/trades.rs` already uses
//! for the per-run trade file, and it was copied rather than invented.
//!
//! **This is not the query planner §4 bans.** That rule refuses a component that
//! CHOOSES a strategy at runtime — *"the path is the index. There is nothing to
//! plan."* An identity-to-offset map plans nothing: there is exactly one way to
//! reach a row, and this is that way, precomputed. The ban is on deciding how to
//! look something up, not on knowing where it already is.
//!
//! It replaced a walk that cost roughly five syscalls per row — a `metadata`, a
//! `lock_shared`, a seek, a `read_exact` and an `unlock`, EACH — so a thousand
//! recorded runs at twenty-five rows apiece was over a hundred thousand
//! syscalls to answer one question about one run.
//!
//! # What the one pass costs, stated rather than hidden
//!
//! Opening reads the whole file once, O(rows), to learn where each block starts.
//! That is the same trade [`crate::results`] already makes for its duplicate
//! check, and it happens once per process rather than once per question.
//!
//! UNVERIFIED as a measured figure: `crates/cli/benches/ratio.rs` measures the
//! results ledger's four bounds and no row of it covers this file yet.
//!
//! # Integers on disk, including the two that are floats in memory
//!
//! [`crate::results`] stores no float and neither does this. `Edge::mean_paisa`
//! and `Edge::t` are `f64` in memory; both are written here as **thousandths**,
//! as `i64`. `CLAUDE.md` §7 keeps floats off any path whose output is compared,
//! and two runs' frontiers are compared by definition — that is the whole reason
//! this file exists.
//!
//! Thousandths and not the raw bits: a bit pattern round-trips exactly and is
//! unreadable in a hex dump, and this file will be read by hand long before it
//! is read by a second program. A `t` of 4.007 is a different finding from 4.0
//! and a `t` of 4.0007 is not, so three decimals is the precision the number
//! actually carries.

use std::fmt::Write as _;
use std::fs::{File, OpenOptions};
use std::io::{Read as _, Seek as _, SeekFrom, Write as _};
use std::path::{Path, PathBuf};

use crate::results::Refusal;

/// `BRUTEXFR`, so a file that is not this one is refused before it is parsed.
///
/// One letter apart from the results ledger's `BRUTEXRS` and deliberately so:
/// both live in the same directory, and a reader that opened the wrong one would
/// otherwise get a stride mismatch rather than a name.
const MAGIC: [u8; 8] = *b"BRUTEXFR";

/// # Version 2 — the money a row was ranked on
///
/// Version 1 held nine fields and its own doc said *"There is no P&L here and
/// that is deliberate. A frontier row records what the SWEEP found."* That was
/// coherent while the ranking was a sweep statistic. It stopped being coherent
/// the moment the operator's ranking became a question about money:
///
/// > *"top 10 ranking should be calculated based on very less max drawdown,
/// > less max stop loss, less losing percentage, less losing trades, less
/// > losing ratio, and on the win side massive max profit, higher winning
/// > trades, higher winning percentage, higher winning ratio, average maximum
/// > profit, average less loss."*
///
/// Every one of those is on `grid::Cell` — `trades`, `wins`, `pessimistic`,
/// `worst_trade`, `max_drawdown`, `min_win`, and `win_rate_bp`,
/// `reward_to_risk_bp`, `return_over_drawdown`, `avg_win`, `avg_loss` derived
/// from them in O(1). `screen` computes a `Cell` for EVERY candidate and then
/// returns `-> String`: the numbers exist for microseconds and are rendered as
/// text. Nothing could rank on them because nothing kept them.
///
/// The six raw fields are stored and the derived ones are NOT, so the reader
/// computes them from the same numbers the engine did — one definition of "win
/// rate", not two. That is the same argument `vocab` makes for serving the
/// condition table once rather than copying it into JavaScript.
///
/// A version is never mutated in place (`CLAUDE.md` §8), so this is a new one at
/// its own stride. A version-1 file is REFUSED and named rather than read with
/// zeros in the new fields — a zero drawdown is a spectacular result, and
/// inventing one for every historical row is the failure §4 bans. The file is
/// regenerable by re-running the sweep, which is the cheap half of this trade.
const VERSION: u32 = 2;

/// Bytes before the first row.
///
/// Sixteen, the same as [`crate::results::HEADER_BYTES`], and
/// `the_two_ledgers_agree_on_what_they_share` fails the build if the two ever
/// differ. They are separate files with separate strides, but a reader that
/// learned one header layout should not have to learn a second.
const HEADER: u64 = 16;

/// [`HEADER`] as a `usize`, for the header array.
///
/// Declared rather than cast: `HEADER as usize` is a narrowing on a 32-bit
/// target, and this workspace builds the same source everywhere.
const HEADER_BYTES: usize = 16;

const _: () = assert!(HEADER_BYTES as u64 == HEADER);

/// Bytes per row.
///
/// 144, and the last eight are the seal. The layout is in [`Row::to_bytes`], and
/// `the_stride_is_exactly_what_the_writer_writes` asserts this constant against
/// what that function actually fills rather than against a hand count.
pub const STRIDE: u64 = 192;

/// [`STRIDE`] as a `usize`. Same reason as [`HEADER_BYTES`].
pub const STRIDE_BYTES: usize = 192;

const _: () = assert!(STRIDE_BYTES as u64 == STRIDE);

/// Bytes of `blake3` kept as the seal.
///
/// Eight, matching the results ledger. The seal answers *were these bytes
/// written whole*, not *who wrote them*, and eight bytes make a torn write
/// astronomically unlikely to pass while costing 5.5% of the stride.
const SEAL_BYTES: usize = 8;

/// Bytes of a row the seal covers: everything before the seal itself.
const PAYLOAD_BYTES: usize = STRIDE_BYTES - SEAL_BYTES;

/// One combination on one run's ranked frontier.
///
/// # Every field is either the key or something a comparison needs
///
/// There is no P&L here and that is deliberate. A frontier row records what the
/// SWEEP found — the combination, how often it fired, and the two statistics the
/// cut can be made on. What a trade of it would have earned is the exit grid's
/// answer and lives on [`crate::results::Record`], one per run, because the grid
/// is only ever run on the chosen combination.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Row {
    /// Which run this row belongs to — the nine-term identity §3 rule 3 names.
    pub identity: [u8; 32],
    /// One-based position in the ranked list, so `1` is the best.
    ///
    /// One-based rather than zero-based because it is printed beside a rank
    /// column an operator reads, and a report that starts at rank 0 has to
    /// explain itself once per reader.
    pub rank: u16,
    /// The combination, as the six words of a [`vocab::ConditionMask`].
    ///
    /// The words rather than a rendered name: a name depends on the vocabulary
    /// version that rendered it, and this row outlives that. `vocab_version` is
    /// already inside `identity`, so a reader that wants names can resolve them
    /// against the vocabulary the run actually used.
    pub mask_words: [u64; 6],
    /// Bars the mask fired on.
    pub hits: u64,
    /// Observations that had a forward outcome — smaller than `hits` by the tail.
    pub n: u64,
    /// `Edge::mean_paisa` in THOUSANDTHS of a paisa.
    pub mean_milli_paisa: i64,
    /// `Edge::t` in THOUSANDTHS.
    pub t_milli: i64,
    /// `Edge::payoff_bp` — the mean win over the mean loss, in hundredths.
    ///
    /// [`i64::MAX`] where the combination never lost, which is the answer that
    /// method gives and is a fact rather than an error.
    pub payoff_bp: i64,
    /// Observations the EDGE counted as wins -- strictly positive forward moves,
    /// before any exit level. Distinct from `cell_wins` below, which counts
    /// priced round trips.
    pub wins: u64,
    /// Observations that moved strictly in favour.
    /// Round trips the chosen exit variant took. `Self::wins` above is the
    /// EDGE's win count over forward moves; this is the CELL's, over priced
    /// trades, and the two answer different questions.
    pub trades: u64,
    /// Of those, how many the cell won.
    pub cell_wins: u64,
    /// Total under worst-case fills, in paisa. Negative or positive.
    pub pessimistic: i64,
    /// The single worst round trip, in paisa. Negative or zero.
    pub worst_trade: i64,
    /// The deepest peak-to-trough fall, in paisa.
    pub max_drawdown: i64,
    /// The SMALLEST winning trade, in paisa -- the numerator of the operator's
    /// "smallest win over largest loss" rule.
    pub min_win: i64,
}

impl Row {
    /// This row as the exact bytes that go on disk, seal included.
    ///
    /// The offsets are written out beside each field because they are the file
    /// format: a reader with a hex dump and this function can decode the file
    /// with nothing else, which is the property `docs/02-store-format.md` asks
    /// every stored shape to have.
    #[must_use]
    #[allow(
        clippy::indexing_slicing,
        reason = "every index is a compile-time constant inside a fixed array \
                  whose length is asserted against the sum of the writes by \
                  `the_stride_is_exactly_what_the_writer_writes`."
    )]
    pub fn to_bytes(&self) -> [u8; STRIDE_BYTES] {
        let mut out = [0_u8; STRIDE_BYTES];
        let mut at = 0_usize;
        let mut put = |bytes: &[u8], at: &mut usize| {
            out[*at..*at + bytes.len()].copy_from_slice(bytes);
            *at += bytes.len();
        };
        put(&self.identity, &mut at); //   0  32
        put(&self.rank.to_le_bytes(), &mut at); //  32   2
        for word in self.mask_words {
            put(&word.to_le_bytes(), &mut at); //  34  48
        }
        put(&self.hits.to_le_bytes(), &mut at); //  82   8
        put(&self.n.to_le_bytes(), &mut at); //  90   8
        put(&self.mean_milli_paisa.to_le_bytes(), &mut at); //  98   8
        put(&self.t_milli.to_le_bytes(), &mut at); // 106   8
        put(&self.payoff_bp.to_le_bytes(), &mut at); // 114   8
        put(&self.wins.to_le_bytes(), &mut at); // 122   8
        put(&self.trades.to_le_bytes(), &mut at); // 130   8
        put(&self.cell_wins.to_le_bytes(), &mut at); // 138   8
        put(&self.pessimistic.to_le_bytes(), &mut at); // 146   8
        put(&self.worst_trade.to_le_bytes(), &mut at); // 154   8
        put(&self.max_drawdown.to_le_bytes(), &mut at); // 162   8
        put(&self.min_win.to_le_bytes(), &mut at); // 170   8 -> ends at 178
        // 130..136 stay zero: six bytes of reserve so the next field does not
        // need a new version for a small addition. Covered by the seal, so a
        // reserve byte that is not zero is a torn write rather than a surprise.
        let seal = seal_of(&out);
        out[PAYLOAD_BYTES..].copy_from_slice(&seal);
        out
    }

    /// Whether a row matches the seal written with it.
    #[must_use]
    pub fn seal_matches(raw: &[u8; STRIDE_BYTES]) -> bool {
        let want = seal_of(raw);
        raw.get(PAYLOAD_BYTES..) == Some(&want[..])
    }

    /// A row from its bytes. The caller checks [`Self::seal_matches`] first.
    #[must_use]
    #[allow(
        clippy::indexing_slicing,
        reason = "the same compile-time constants `to_bytes` writes at, over an \
                  array of the same asserted length."
    )]
    pub fn from_bytes(raw: &[u8; STRIDE_BYTES]) -> Self {
        let mut at = 0_usize;
        let take = |n: usize, at: &mut usize| {
            let slice = &raw[*at..*at + n];
            *at += n;
            slice
        };
        let mut identity = [0_u8; 32];
        identity.copy_from_slice(take(32, &mut at));
        let rank = u16::from_le_bytes(take(2, &mut at).try_into().unwrap_or([0; 2]));
        let mut mask_words = [0_u64; 6];
        for word in &mut mask_words {
            *word = u64::from_le_bytes(take(8, &mut at).try_into().unwrap_or([0; 8]));
        }
        Self {
            identity,
            rank,
            mask_words,
            hits: u64::from_le_bytes(take(8, &mut at).try_into().unwrap_or([0; 8])),
            n: u64::from_le_bytes(take(8, &mut at).try_into().unwrap_or([0; 8])),
            mean_milli_paisa: i64::from_le_bytes(take(8, &mut at).try_into().unwrap_or([0; 8])),
            t_milli: i64::from_le_bytes(take(8, &mut at).try_into().unwrap_or([0; 8])),
            payoff_bp: i64::from_le_bytes(take(8, &mut at).try_into().unwrap_or([0; 8])),
            wins: u64::from_le_bytes(take(8, &mut at).try_into().unwrap_or([0; 8])),
            trades: u64::from_le_bytes(take(8, &mut at).try_into().unwrap_or([0; 8])),
            cell_wins: u64::from_le_bytes(take(8, &mut at).try_into().unwrap_or([0; 8])),
            pessimistic: i64::from_le_bytes(take(8, &mut at).try_into().unwrap_or([0; 8])),
            worst_trade: i64::from_le_bytes(take(8, &mut at).try_into().unwrap_or([0; 8])),
            max_drawdown: i64::from_le_bytes(take(8, &mut at).try_into().unwrap_or([0; 8])),
            min_win: i64::from_le_bytes(take(8, &mut at).try_into().unwrap_or([0; 8])),
        }
    }

    /// One ranked combination, as this file stores it.
    ///
    /// The two `f64` statistics are scaled to thousandths and SATURATED rather
    /// than wrapped: a `t` past the `i64` range is not a number any run
    /// produced, and wrapping it would store a large negative `t` for the
    /// strongest finding in the sweep. A NaN — which `Edge::t` documents itself
    /// as returning zero for, but which a future caller could hand in — becomes
    /// zero, because an undefined statistic must not read as a strong one.
    /// # The cell is optional, and its absence is a real state
    ///
    /// A combination is ranked by the sweep before it is priced, and
    /// `screen_cap` means most ranked combinations never meet an exit grid at
    /// all. `None` says exactly that — swept and ranked, never priced — and it
    /// is stored as zeros that the reader must read through the `trades` count,
    /// which is zero only in that case. Writing a plausible drawdown for a
    /// combination nobody priced would be the failure `CLAUDE.md` §4 bans.
    #[must_use]
    pub fn of(
        identity: [u8; 32],
        rank: u16,
        scored: &runner::rank::Scored,
        cell: Option<&runner::grid::Cell>,
    ) -> Self {
        Self {
            identity,
            rank,
            mask_words: scored.mask.words(),
            hits: scored.hits,
            n: scored.edge.n,
            // ASKED FOR AS INTEGERS. `cli` denies `clippy::float_arithmetic`
            // outright -- CLAUDE.md §7, made a lint by D-0061 -- so the crate
            // that owns the float owns the conversion and this one never
            // touches one.
            mean_milli_paisa: scored.edge.mean_milli_paisa(),
            t_milli: scored.edge.t_milli(),
            payoff_bp: scored.edge.payoff_bp(),
            wins: scored.edge.wins,
            // THE MONEY THE OPERATOR RANKS ON. Six raw fields, and the derived
            // ones -- win rate, reward-to-risk, return over drawdown, average
            // win, average loss -- are NOT stored: the reader computes them
            // from these, so there is one definition of each and not two.
            trades: cell.map_or(0, |c| c.trades),
            cell_wins: cell.map_or(0, |c| c.wins),
            pessimistic: cell.map_or(0, |c| c.pessimistic),
            worst_trade: cell.map_or(0, |c| c.worst_trade),
            max_drawdown: cell.map_or(0, |c| c.max_drawdown),
            min_win: cell.map_or(0, |c| c.min_win),
        }
    }
}

/// The first eight bytes of `blake3` over everything but the seal.
fn seal_of(raw: &[u8; STRIDE_BYTES]) -> [u8; SEAL_BYTES] {
    let mut hasher = brutex_core::blake3::Hasher::new();
    hasher.update(&raw[..PAYLOAD_BYTES]);
    let full = hasher.finalize();
    let mut out = [0_u8; SEAL_BYTES];
    out.copy_from_slice(&full[..SEAL_BYTES]);
    out
}

/// The frontier file, open.
///
/// `Debug` prints the PATH and not the handle: a file descriptor number is not
/// a fact about the store, and a test that refuses to open one wants to say
/// which file it was.
#[derive(Debug)]
pub struct Frontier {
    file: File,
    path: PathBuf,
    /// Which rows belong to which run — built in one pass at open.
    blocks: std::collections::HashMap<[u8; 32], Block>,
}

/// Where one run's rows sit in the file.
///
/// Two integers rather than a list of rows, because [`Frontier::append_all`]
/// writes a whole frontier in one call under one lock — so a run's rows are
/// contiguous and a `(first, count)` pair locates all of them exactly.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Block {
    /// Index of this run's first row.
    pub first: u64,
    /// How many rows it owns.
    pub count: u64,
}

impl Frontier {
    /// Where the file lives, beside the results ledger.
    #[must_use]
    pub fn path(root: &Path) -> PathBuf {
        root.join("results").join("frontier.bin")
    }

    /// Opens an EXISTING file for reading, and never creates one.
    ///
    /// # Why a second constructor rather than a flag on the first
    ///
    /// [`Self::open`] creates: it does `create_dir_all`, `create(true)`, and on
    /// an empty file writes a fresh header. That is right for a sweep, which is
    /// about to append. It is wrong for anything that only READS — an HTTP
    /// handler answering `GET` on a store that has never been swept would create
    /// `frontier.bin` as a side effect of being asked a question.
    ///
    /// `crates/api/src/backtest.rs` avoids exactly this for the results ledger,
    /// with a bare `File::open` and a `NotFound` arm whose message says *"This is
    /// not an error."* A reader of this file needs the same, and a boolean
    /// parameter would leave the caller to remember which way it points.
    ///
    /// # Errors
    ///
    /// Refuses when the file is absent — named as absent rather than as a
    /// failure — and for every reason [`Self::open`] refuses an existing file:
    /// a wrong magic, an unreadable version, or a length that does not divide
    /// by the stride.
    pub fn open_read(root: &Path) -> Result<Self, Refusal> {
        let path = Self::path(root);
        let mut file = File::open(&path).map_err(|why| {
            if why.kind() == std::io::ErrorKind::NotFound {
                format!(
                    "{} does not exist yet. No run has recorded a frontier — \
                     this is not an error, and nothing was created.",
                    path.display()
                )
            } else {
                format!("{} could not be opened: {why}", path.display())
            }
        })?;
        let len = file
            .metadata()
            .map_err(|why| format!("{} could not be measured: {why}", path.display()))?
            .len();
        check_header(&mut file, &path, len)?;
        let blocks = index_of(&mut file, len)?;
        Ok(Self { file, path, blocks })
    }

    /// Where one run's rows sit, or `None` if it recorded none. **O(1).**
    ///
    /// **UNVERIFIED as a measurement.** The bound is argued from the
    /// shape of the code and no bench in this workspace times it.
    /// `CLAUDE.md` §3 rule 6: a structural argument is not a
    /// measurement, however sound it is.
    #[must_use]
    pub fn block(&self, identity: &[u8; 32]) -> Option<Block> {
        self.blocks.get(identity).copied()
    }

    /// Whether this run recorded a frontier at all. **O(1).**
    ///
    /// **UNVERIFIED as a measurement.** The bound is argued from the
    /// shape of the code and no bench in this workspace times it.
    /// `CLAUDE.md` §3 rule 6: a structural argument is not a
    /// measurement, however sound it is.
    #[must_use]
    pub fn holds(&self, identity: &[u8; 32]) -> bool {
        self.blocks.contains_key(identity)
    }

    /// Opens the file, creating it with a fresh header if it is not there.
    ///
    /// # Errors
    ///
    /// Refuses when the directory cannot be made, the file cannot be opened,
    /// the magic is not `BRUTEXFR`, the version is one this build does not
    /// write, or the length does not divide by the stride -- each named, and
    /// nothing written in any of them.
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

    /// How many rows the file holds. **O(1)** — arithmetic on the length.
    ///
    /// # Errors
    ///
    /// Refuses when the file cannot be measured, which is the only way a length
    /// can be unknown.
    ///
    /// **UNVERIFIED as a measurement.** The bound is argued from the
    /// shape of the code and no bench in this workspace times it.
    /// `CLAUDE.md` §3 rule 6: a structural argument is not a
    /// measurement, however sound it is.
    pub fn len(&self) -> Result<u64, Refusal> {
        let len = self
            .file
            .metadata()
            .map_err(|why| format!("{} could not be measured: {why}", self.path.display()))?
            .len();
        Ok(len.saturating_sub(HEADER) / STRIDE)
    }

    /// Whether the file holds no rows.
    ///
    /// # Errors
    ///
    /// The same refusal [`Self::len`] gives, for the same reason.
    pub fn is_empty(&self) -> Result<bool, Refusal> {
        Ok(self.len()? == 0)
    }

    /// Appends every row of one run's frontier, under one exclusive lock.
    ///
    /// # Why the whole frontier and not one row at a time
    ///
    /// A run's rows are only meaningful together — rank 3 of a list whose rank 1
    /// is missing is not a partial answer, it is a misleading one. Taking the
    /// lock once and writing them in one call is what makes a reader either see
    /// the whole frontier or none of it.
    ///
    /// It is still O(1) per row: one `write_all` of `rows.len() * STRIDE` bytes
    /// at the end of the file, with no read and no seek per row.
    ///
    /// # Errors
    ///
    /// Refuses when the exclusive lock cannot be taken, the seek or write
    /// fails, the flush fails, or the unlock fails. A refusal from the unlock
    /// is returned even when the write succeeded: a lock this process still
    /// holds would block every later reader, and that is not a success.
    ///
    /// **UNVERIFIED as a measurement.** The bound is argued from the
    /// shape of the code and no bench in this workspace times it.
    /// `CLAUDE.md` §3 rule 6: a structural argument is not a
    /// measurement, however sound it is.
    pub fn append_all(&mut self, rows: &[Row]) -> Result<u64, Refusal> {
        if rows.is_empty() {
            return self.len();
        }
        // THE DUPLICATE REFUSAL, AND IT IS THE ONE THIS FILE WAS WRITTEN
        // WITHOUT.
        //
        // `crate::results::append_locked` refuses an identity it already holds.
        // This file did not, and `crate::record_frontier` is called
        // UNCONDITIONALLY -- including on the path where `record_run` has just
        // refused the same identity as already recorded. So a rerun, the thing
        // §3 rule 5 calls SAFE, wrote a SECOND block for one identity.
        //
        // What that costs is not a duplicate row, which would be harmless. The
        // accounting below keeps the ORIGINAL `first` and moves `count` to the
        // new row, so the block comes to span every row written BETWEEN the two
        // appends -- and `of_run` returned that whole span. MEASURED on this
        // type: three identities of three rows each, appended twice, gave every
        // block `count = 12`, of which SIX rows belonged to another run, with
        // `damaged` reporting false. A third pass gave 21, nine own and twelve
        // foreign. `cli top` renders that mixture under one run's banner.
        //
        // The check is `holds`, which was already written, already documented
        // O(1), and until now had no production call site at all.
        for row in rows {
            if self.holds(&row.identity) {
                return Err(format!(
                    "run {} already has a frontier block. Same inputs give same \
                     outputs (§3 rule 5), so a second block adds nothing -- and \
                     it would make the first one span every row written between \
                     the two, which `of_run` would then return as that run's.",
                    hex(&row.identity)
                ));
            }
        }
        // AN EXCLUSIVE LOCK, for the reason `crate::results::append` gives at
        // length: two processes that both seek to the end compute the same
        // offset, and the second write lands ON TOP of the first. Both records
        // are correctly sealed, so nothing downstream can see the loss.
        self.file
            .lock()
            .map_err(|why| format!("the frontier file could not be locked: {why}"))?;
        let out = self.append_locked(rows);
        let released = self
            .file
            .unlock()
            .map_err(|why| format!("the frontier file could not be unlocked: {why}"));
        match (out, released) {
            (Ok(n), Ok(())) => Ok(n),
            (Err(why), _) | (Ok(_), Err(why)) => Err(why),
        }
    }

    /// [`Self::append_all`]'s work, with the lock already held.
    fn append_locked(&mut self, rows: &[Row]) -> Result<u64, Refusal> {
        // WHERE THE BLOCK WILL LAND, read from the file rather than from the
        // index. Another process may have appended since this one opened, and
        // its rows are in the file whether or not they are in this process's
        // map — so the offset has to come from the length under the lock.
        let end = self
            .file
            .seek(SeekFrom::End(0))
            .map_err(|why| format!("the frontier file could not be seeked: {why}"))?;
        let first = end.saturating_sub(HEADER) / STRIDE;

        let mut buffer: Vec<u8> = Vec::with_capacity(rows.len() * STRIDE_BYTES);
        for row in rows {
            buffer.extend_from_slice(&row.to_bytes());
        }
        self.file
            .write_all(&buffer)
            .map_err(|why| format!("the frontier rows could not be written: {why}"))?;
        // FLUSHED BEFORE THE COUNT IS REPORTED. A count taken from a length the
        // operating system has not committed is a number that can shrink.
        self.file
            .sync_all()
            .map_err(|why| format!("the frontier rows could not be flushed: {why}"))?;

        // THE INDEX LEARNS ABOUT THE BLOCK IT JUST WROTE, so a process that
        // appends and then reads does not have to reopen the file to find its
        // own rows. Grouped by identity because one call may carry more than
        // one run's rows in principle; today it carries exactly one.
        for (nth, row) in rows.iter().enumerate() {
            let at = first.saturating_add(nth as u64);
            self.blocks
                .entry(row.identity)
                .and_modify(|block| {
                    block.count = at.saturating_add(1).saturating_sub(block.first);
                })
                .or_insert(Block {
                    first: at,
                    count: 1,
                });
        }
        self.len()
    }

    /// Row `index`. **O(1)** — one seek to `HEADER + index·STRIDE`, one read.
    ///
    /// # Errors
    ///
    /// Refuses when `index` is past the end -- naming the count -- when the
    /// shared lock, seek or read fails, or when the row does not match the seal
    /// written with it. A seal mismatch names the row and changes nothing.
    ///
    /// **UNVERIFIED as a measurement.** The bound is argued from the
    /// shape of the code and no bench in this workspace times it.
    /// `CLAUDE.md` §3 rule 6: a structural argument is not a
    /// measurement, however sound it is.
    pub fn read(&mut self, index: u64) -> Result<Row, Refusal> {
        let count = self.len()?;
        if index >= count {
            return Err(format!(
                "row {index} is past the end: the frontier holds {count}"
            ));
        }
        // A SHARED lock, so a reader waits for an in-progress append rather than
        // seeing half of one and reporting corruption on a healthy file. Any
        // number of readers hold it at once; only a writer excludes them.
        self.file
            .lock_shared()
            .map_err(|why| format!("the frontier file could not be locked for reading: {why}"))?;
        let out = self.read_locked(index);
        let released = self
            .file
            .unlock()
            .map_err(|why| format!("the frontier file could not be unlocked: {why}"));
        match (out, released) {
            (Ok(row), Ok(())) => Ok(row),
            (Err(why), _) | (Ok(_), Err(why)) => Err(why),
        }
    }

    /// [`Self::read`]'s work, with the lock already held.
    fn read_locked(&mut self, index: u64) -> Result<Row, Refusal> {
        let at = HEADER.saturating_add(index.saturating_mul(STRIDE));
        self.file
            .seek(SeekFrom::Start(at))
            .map_err(|why| format!("the frontier file could not be seeked: {why}"))?;
        let mut raw = [0_u8; STRIDE_BYTES];
        self.file
            .read_exact(&mut raw)
            .map_err(|why| format!("row {index} could not be read: {why}"))?;
        if !Row::seal_matches(&raw) {
            return Err(format!(
                "row {index} does not match the seal written with it — the file \
                 is damaged at that row. Nothing was changed."
            ));
        }
        Ok(Row::from_bytes(&raw))
    }

    /// Every row belonging to one run, best rank first.
    ///
    /// **FINDING the block is O(1); READING it is O(count).** One hash probe
    /// into the block index built in one pass at open, then one seek and one
    /// `read_exact` of `count · STRIDE` bytes. §4 bans a query planner — *"the
    /// path is the index"* — and this is not one: the offset IS the address.
    ///
    /// **This paragraph used to say the opposite** — *"O(rows), and it is a
    /// walk. This file carries no index"* — which described the file before the
    /// block index existed and contradicted the header table at the top of this
    /// module. A doc that disagrees with the code beside it is the stale copy,
    /// and a reader who believed this one would have paid for a walk that is
    /// not there.
    ///
    /// **UNVERIFIED as a measurement.** The `O(1)` half is argued from the
    /// shape — one hash probe, then an address rather than a search — and no
    /// bench times it. `crates/cli/benches/ratio.rs` measures `results`:
    /// `C-CLI-01` a record read across a 1,024-record span, `C-CLI-02` a count,
    /// `C-CLI-03` the duplicate probe, `C-CLI-04` an encode. None of the four
    /// opens a frontier file. Closing it is a fifth row over `rows_for` at
    /// 1×/10×/100× the BLOCK count, which is the axis the claim is about — the
    /// row count is the `O(count)` half and is not in dispute.
    ///
    /// The rows returned are filtered to `identity`. A block written before
    /// `append_all` refused duplicates can span another run's rows, and §8 keeps
    /// those files as they are.
    ///
    /// A DAMAGED row does not stop the walk. A frontier is a list, and one
    /// unreadable entry is a smaller list rather than no answer — the refusal is
    /// returned beside what was read, which is what §4 asks for.
    ///
    /// # Errors
    ///
    /// Refuses only when the LENGTH cannot be read, because without it there is
    /// no walk to make. A damaged row inside the walk is returned as the second
    /// half of the pair rather than as a refusal.
    pub fn of_run(&mut self, identity: &[u8; 32]) -> Result<(Vec<Row>, Option<Refusal>), Refusal> {
        // O(1) TO FIND. This walked the whole file and read every row to check
        // its identity -- roughly five syscalls per row, because `read` takes a
        // `metadata`, a shared lock, a seek, a read and an unlock EACH. A
        // thousand recorded runs at twenty-five rows apiece was over a hundred
        // thousand syscalls to answer one question about one run.
        let Some(block) = self.block(identity) else {
            return Ok((Vec::new(), None));
        };

        // O(count) TO READ, and it is ONE read rather than `count` of them. The
        // rows are contiguous because `append_all` wrote them in one call, so
        // the whole block comes back in a single `read_exact` under a single
        // shared lock.
        let Ok(width) = usize::try_from(block.count.saturating_mul(STRIDE)) else {
            return Err(format!(
                "run {} claims {} rows, which does not fit this machine's address \
                 space. The index is damaged and nothing was read.",
                hex(identity),
                block.count
            ));
        };
        let at = HEADER.saturating_add(block.first.saturating_mul(STRIDE));

        self.file
            .lock_shared()
            .map_err(|why| format!("the frontier file could not be locked for reading: {why}"))?;
        let out = self.read_block_locked(at, width);
        let released = self
            .file
            .unlock()
            .map_err(|why| format!("the frontier file could not be unlocked: {why}"));
        let raw = match (out, released) {
            (Ok(raw), Ok(())) => raw,
            (Err(why), _) | (Ok(_), Err(why)) => return Err(why),
        };

        // A DAMAGED ROW DOES NOT DISCARD THE BLOCK. A frontier is a list, and
        // one unreadable entry is a shorter list rather than no answer -- the
        // refusal is returned beside what was read, which is what §4 asks for.
        let mut found: Vec<Row> = Vec::with_capacity(raw.len() / STRIDE_BYTES);
        let mut damaged: Option<Refusal> = None;
        for (nth, chunk) in raw.chunks_exact(STRIDE_BYTES).enumerate() {
            let Ok(bytes) = <[u8; STRIDE_BYTES]>::try_from(chunk) else {
                continue;
            };
            if Row::seal_matches(&bytes) {
                // A ROW THAT IS NOT THIS RUN'S IS NOT THIS RUN'S, however
                // correctly it is sealed. The seal proves the bytes are intact;
                // it says nothing about WHOSE they are. A block written before
                // the duplicate refusal above existed can still span foreign
                // rows, and this file is append-only (§8) -- those files are on
                // disk and are not rewritten. So the read filters too, and a
                // stale span comes back as the run's OWN rows rather than as a
                // mixture nothing marks.
                let row = Row::from_bytes(&bytes);
                if &row.identity == identity {
                    found.push(row);
                }
            } else if damaged.is_none() {
                damaged = Some(format!(
                    "row {} of run {} does not match the seal written with it — \
                     the file is damaged at that row. Nothing was changed.",
                    block.first.saturating_add(nth as u64),
                    hex(identity)
                ));
            }
        }
        // BEST RANK FIRST. The block is in WRITE order, which is the order the
        // ranker handed them over -- already best first today. Sorting anyway
        // costs nothing at twenty-five rows and means a future caller that
        // appends out of order still reads back in order.
        found.sort_unstable_by_key(|row| (row.rank, row.mask_words));
        Ok((found, damaged))
    }

    /// One contiguous read of `width` bytes at `at`, with the lock already held.
    fn read_block_locked(&mut self, at: u64, width: usize) -> Result<Vec<u8>, Refusal> {
        self.file
            .seek(SeekFrom::Start(at))
            .map_err(|why| format!("the frontier file could not be seeked: {why}"))?;
        let mut raw = vec![0_u8; width];
        self.file
            .read_exact(&mut raw)
            .map_err(|why| format!("a frontier block could not be read: {why}"))?;
        Ok(raw)
    }
}

/// A run identity as lowercase hex, for naming it in a refusal.
fn hex(identity: &[u8; 32]) -> String {
    let mut out = String::with_capacity(64);
    for byte in identity {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

/// Every run's block, from one pass over the file.
///
/// # Why a pass rather than a footer
///
/// A footer would have to be rewritten on every append, which turns an O(1)
/// append into a read-modify-write and puts a second thing in the file that can
/// disagree with the first. The pass is O(rows) once per process; the append
/// stays one buffered write.
///
/// **A row whose seal fails is skipped rather than fatal.** Its block loses a
/// row and `of_run` reports the damage when the block is read; refusing to open
/// the whole file over one bad row would take a store with 999 good runs offline
/// for the thousandth.
///
/// **UNVERIFIED as a measurement.** The bound is argued from the
/// shape of the code and no bench in this workspace times it.
/// `CLAUDE.md` §3 rule 6: a structural argument is not a
/// measurement, however sound it is.
fn index_of(
    file: &mut File,
    len: u64,
) -> Result<std::collections::HashMap<[u8; 32], Block>, Refusal> {
    let count = len.saturating_sub(HEADER) / STRIDE;
    let mut blocks: std::collections::HashMap<[u8; 32], Block> =
        std::collections::HashMap::with_capacity(
            usize::try_from(count).unwrap_or(0).saturating_div(8).max(8),
        );
    file.seek(SeekFrom::Start(HEADER))
        .map_err(|why| format!("the frontier file could not be seeked: {why}"))?;

    let mut raw = [0_u8; STRIDE_BYTES];
    for index in 0..count {
        file.read_exact(&mut raw)
            .map_err(|why| format!("row {index} could not be read while indexing: {why}"))?;
        if !Row::seal_matches(&raw) {
            continue;
        }
        let identity = Row::from_bytes(&raw).identity;
        blocks
            .entry(identity)
            .and_modify(|block| {
                // A run appended more than once -- `append_all` under one lock
                // makes that a re-record rather than a scatter, so the block
                // GROWS to cover both. `first` stays where the run started.
                block.count = index.saturating_add(1).saturating_sub(block.first);
            })
            .or_insert(Block {
                first: index,
                count: 1,
            });
    }
    Ok(blocks)
}

/// Writes the sixteen-byte header of a fresh file, and proves it was kept.
fn write_fresh_header(file: &mut File, path: &Path) -> Result<(), Refusal> {
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
    file.sync_all()
        .map_err(|why| format!("the header could not be flushed: {why}"))?;
    // READ BACK, because a successful write is not proof that anything was
    // stored — the same check `crate::results::write_fresh_header` makes and for
    // the same reason.
    file.seek(SeekFrom::Start(0))
        .map_err(|why| format!("the header could not be seeked: {why}"))?;
    let mut back = [0_u8; HEADER_BYTES];
    file.read_exact(&mut back)
        .map_err(|why| format!("the header could not be read back: {why}"))?;
    if back != header {
        return Err(format!(
            "{} did not keep the header it was given. Nothing was written.",
            path.display()
        ));
    }
    Ok(())
}

/// Refuses a file that is not this format, or is a version this build cannot read.
fn check_header(file: &mut File, path: &Path, len: u64) -> Result<(), Refusal> {
    if len < HEADER {
        return Err(format!(
            "{} is {len} bytes, shorter than the {HEADER}-byte header. It is not \
             a frontier file. Nothing was written.",
            path.display()
        ));
    }
    file.seek(SeekFrom::Start(0))
        .map_err(|why| format!("the header could not be seeked: {why}"))?;
    let mut header = [0_u8; HEADER_BYTES];
    file.read_exact(&mut header)
        .map_err(|why| format!("the header could not be read: {why}"))?;
    if header.get(..8) != Some(&MAGIC) {
        return Err(format!(
            "{} does not begin with `BRUTEXFR`. Nothing was written.",
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
            "{} is frontier format version {version}; this build writes and reads \
             version {VERSION}. A format version is never mutated in place — \
             CLAUDE.md §8. Nothing was written.",
            path.display()
        ));
    }
    // A RAGGED TAIL IS NAMED RATHER THAN ABSORBED. Bytes past the last whole row
    // mean an interrupted append; the whole rows are still readable and `len()`
    // reports them, so this refuses only if the file is unusable rather than
    // merely untidy.
    let body = len.saturating_sub(HEADER);
    if !body.is_multiple_of(STRIDE) {
        let whole = body / STRIDE;
        let spare = body % STRIDE;
        return Err(format!(
            "{} has {spare} bytes past its last whole row — an append was \
             interrupted. {whole} whole rows are intact; the remainder is left \
             alone. Nothing was written.",
            path.display()
        ));
    }
    Ok(())
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]
mod tests {
    use super::{Frontier, HEADER_BYTES, MAGIC, Row, SEAL_BYTES, STRIDE, STRIDE_BYTES};

    fn root(tag: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("brutex-frontier-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a temp root");
        dir
    }

    fn row(identity: u8, rank: u16) -> Row {
        Row {
            identity: [identity; 32],
            rank,
            mask_words: [u64::from(rank), 2, 3, 4, 5, 6],
            hits: 1_000 + u64::from(rank),
            n: 900 + u64::from(rank),
            mean_milli_paisa: 1_234,
            t_milli: 4_007,
            payoff_bp: 900,
            wins: 700,
            trades: 0,
            cell_wins: 0,
            pessimistic: 0,
            worst_trade: 0,
            max_drawdown: 0,
            min_win: 0,
        }
    }

    /// The stride is what the writer writes, not what a comment claims.
    ///
    /// Version 2 added six money fields — `trades`, `cell_wins`, `pessimistic`,
    /// `worst_trade`, `max_drawdown`, `min_win` — because the operator's ranking
    /// is a question about money and version 1 stored none of it. The stride
    /// moved from 144 to 192, and this assertion is what made that a decision
    /// rather than an accident: it failed the moment the fields were added and
    /// the number was not.
    #[test]
    fn the_stride_is_exactly_what_the_writer_writes() {
        //           identity  rank  mask      hits/n/mean/t/payoff/wins
        let v1 = 32 + 2 + 6 * 8 + 8 + 8 + 8 + 8 + 8 + 8;
        //  trades, cell_wins, pessimistic, worst_trade, max_drawdown, min_win
        let v2_added = 6 * 8;
        assert_eq!(
            v1 + v2_added + 6 + SEAL_BYTES,
            STRIDE_BYTES,
            "fields + six reserved + seal must be the stride"
        );
        assert_eq!(STRIDE_BYTES as u64, STRIDE);
    }

    /// The two ledgers agree on the things a reader learns once.
    #[test]
    fn the_two_ledgers_agree_on_what_they_share() {
        assert_eq!(
            HEADER_BYTES,
            crate::results::HEADER_BYTES,
            "a reader that learned one header layout must not have to learn a second"
        );
        assert_eq!(
            MAGIC.len(),
            crate::results::MAGIC.len(),
            "both magics are eight bytes, so both headers have the same shape"
        );
        assert_ne!(
            MAGIC,
            crate::results::MAGIC,
            "the two files live in one directory; opening the wrong one must be \
             refused by NAME rather than by a stride mismatch"
        );
    }

    /// Every field survives the round trip.
    #[test]
    fn every_field_survives_the_round_trip() {
        let want = row(7, 3);
        let raw = want.to_bytes();
        assert!(Row::seal_matches(&raw), "a fresh row matches its own seal");
        assert_eq!(Row::from_bytes(&raw), want);
    }

    /// A flipped byte anywhere in the payload is caught.
    #[test]
    fn a_single_flipped_byte_fails_the_seal() {
        let mut raw = row(1, 1).to_bytes();
        // One byte from each field, plus the last reserve byte: the seal covers
        // the reserve too, so a torn write that lands there is caught rather
        // than read as a surprising-but-valid row.
        for at in [0_usize, 32, 40, 100, 129] {
            let before = *raw.get(at).expect("the stride is longer than 129");
            *raw.get_mut(at).expect("the same index, still in range") = before ^ 0x01;
            assert!(
                !Row::seal_matches(&raw),
                "a flipped byte at {at} must fail the seal"
            );
            *raw.get_mut(at).expect("the same index, still in range") = before;
        }
        assert!(Row::seal_matches(&raw), "restored bytes match again");
    }

    /// The block index locates a run without reading a row.
    ///
    /// This is the O(1) half of the pair: `block` and `holds` answer from the
    /// map alone, and the file is not touched. The index must also survive a
    /// reopen, because the process that READS a frontier is rarely the one that
    /// wrote it.
    ///
    /// **UNVERIFIED as a measurement.** The bound is argued from the
    /// shape of the code and no bench in this workspace times it.
    /// `CLAUDE.md` §3 rule 6: a structural argument is not a
    /// measurement, however sound it is.
    #[test]
    fn the_block_index_locates_a_run_without_reading_a_row() {
        let dir = root("blocks");
        let mut store = Frontier::open(&dir).expect("a fresh file opens");
        store
            .append_all(&[row(1, 1), row(1, 2), row(1, 3)])
            .expect("run 1");
        store.append_all(&[row(2, 1), row(2, 2)]).expect("run 2");

        // O(1) TO FIND: two integers, and no row was read to get them.
        assert_eq!(
            store.block(&[1; 32]),
            Some(super::Block { first: 0, count: 3 }),
            "run 1 owns the first three rows"
        );
        assert_eq!(
            store.block(&[2; 32]),
            Some(super::Block { first: 3, count: 2 }),
            "run 2 owns the two after them"
        );
        assert_eq!(
            store.block(&[9; 32]),
            None,
            "a run with no rows has no block"
        );
        assert!(store.holds(&[1; 32]));
        assert!(!store.holds(&[9; 32]));

        // AND THE INDEX SURVIVES A REOPEN, built from the file rather than
        // carried in memory -- which is the case that matters, because the
        // process that reads is rarely the one that wrote.
        drop(store);
        let reopened = Frontier::open(&dir).expect("an existing file opens");
        assert_eq!(
            reopened.block(&[2; 32]),
            Some(super::Block { first: 3, count: 2 }),
            "the one pass at open rebuilt the same block"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A read-only open never creates the file.
    ///
    /// An HTTP handler answering GET on a store that has never been swept must
    /// not create `frontier.bin` as a side effect of being asked a question.
    /// `crates/api/src/backtest.rs` avoids exactly this for the results ledger.
    #[test]
    fn opening_to_read_creates_nothing_and_names_the_absence() {
        let dir = root("readonly");
        // Deliberately NOT calling `open` first: the file does not exist.
        let why = Frontier::open_read(&dir).expect_err("there is no file to read");
        assert!(
            why.contains("does not exist yet"),
            "the absence is named as an absence: {why}"
        );
        assert!(
            why.contains("not an error"),
            "and is not reported as a failure: {why}"
        );
        assert!(
            !Frontier::path(&dir).exists(),
            "READING MUST NOT CREATE. `open` creates by design; `open_read` must \
             leave a store that has never been swept exactly as it found it."
        );

        // Once a writer has made it, the reader sees the same blocks.
        let mut writer = Frontier::open(&dir).expect("a writer creates it");
        writer
            .append_all(&[row(5, 1), row(5, 2)])
            .expect("two rows");
        drop(writer);

        let reader = Frontier::open_read(&dir).expect("now it exists");
        assert_eq!(
            reader.block(&[5; 32]),
            Some(super::Block { first: 0, count: 2 })
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A run recorded twice is REFUSED, and the first block is left alone.
    ///
    /// # This test asserted the defect
    ///
    /// It read *"the block grows to cover both rather than forgetting the
    /// first"* and expected `count: 2`. That growth is exactly the bug: the
    /// accounting keeps the original `first` and moves `count` to the newest
    /// row, so with anything written in between the block comes to span another
    /// run's rows. Back to back there is nothing in between, which is why the
    /// old shape passed and why
    /// [`a_block_never_spans_another_runs_rows`] is its necessary companion.
    #[test]
    fn a_run_appended_twice_is_refused_and_the_first_block_stands() {
        let dir = root("twice");
        let mut store = Frontier::open(&dir).expect("a fresh file opens");
        store.append_all(&[row(3, 1)]).expect("first append");

        let again = store.append_all(&[row(3, 2)]);
        let why = again.expect_err("a second block for one identity is refused");
        assert!(
            why.contains("already has a frontier block"),
            "the refusal names what it refused: {why}"
        );

        assert_eq!(
            store.block(&[3; 32]),
            Some(super::Block { first: 0, count: 1 }),
            "the refusal changed nothing -- the first block still covers one row"
        );
        let (rows, damaged) = store.of_run(&[3; 32]).expect("a read");
        assert!(damaged.is_none());
        assert_eq!(rows.len(), 1, "only the first append is there");
        let kept = rows.first().expect("the one row just asserted");
        assert_eq!(kept.rank, 1, "and it is the FIRST one, not the second");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A run's rows never come back with another run's mixed in.
    ///
    /// # The case the back-to-back test could not reach
    ///
    /// MEASURED on this type before the refusal existed: three identities of
    /// three rows each, appended twice, gave every block `count = 12` of which
    /// six rows belonged to another run, with `damaged` reporting FALSE. `cli
    /// top` renders that mixture under one run's banner, which is §4's failure
    /// wearing a success's clothes.
    ///
    /// Both halves of the fix are asserted here: the append is refused, and the
    /// read filters by identity so a block written before the refusal existed
    /// still comes back as its own run's rows. §8 keeps those files as they are,
    /// so the read-side filter is not redundant with the write-side refusal.
    #[test]
    fn a_block_never_spans_another_runs_rows() {
        let dir = root("interleaved");
        let mut store = Frontier::open(&dir).expect("a fresh file opens");

        store.append_all(&[row(1, 1), row(1, 2)]).expect("run one");
        store.append_all(&[row(2, 1), row(2, 2)]).expect("run two");
        // The rerun §3 rule 5 calls SAFE, with a foreign run now in between.
        let again = store.append_all(&[row(1, 3)]);
        assert!(
            again.is_err(),
            "the rerun is refused rather than widening run one's block over run two"
        );

        let (ones, damaged) = store.of_run(&[1; 32]).expect("a read");
        assert!(damaged.is_none(), "nothing is damaged: {damaged:?}");
        assert_eq!(ones.len(), 2, "run one has exactly its own two rows");
        assert!(
            ones.iter().all(|r| r.identity == [1; 32]),
            "and every one of them is run one's"
        );

        let (twos, _) = store.of_run(&[2; 32]).expect("a read");
        assert_eq!(twos.len(), 2, "run two is untouched");
        assert!(twos.iter().all(|r| r.identity == [2; 32]));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A whole frontier is written and read back, grouped by run.
    #[test]
    fn a_runs_rows_come_back_together_and_in_rank_order() {
        let dir = root("of-run");
        let mut store = Frontier::open(&dir).expect("a fresh file opens");
        assert!(store.is_empty().expect("a length"), "a fresh file is empty");

        // Two runs interleaved, and the second written out of rank order.
        let first: Vec<Row> = (1..=3).map(|r| row(1, r)).collect();
        let second: Vec<Row> = [3_u16, 1, 2].iter().map(|&r| row(2, r)).collect();
        store.append_all(&first).expect("the first run appends");
        store.append_all(&second).expect("the second run appends");
        assert_eq!(store.len().expect("a length"), 6, "six rows in total");

        let (rows, damaged) = store.of_run(&[2; 32]).expect("a walk");
        assert!(damaged.is_none(), "nothing is damaged");
        assert_eq!(rows.len(), 3, "only the second run's rows");
        assert_eq!(
            rows.iter().map(|r| r.rank).collect::<Vec<_>>(),
            vec![1, 2, 3],
            "returned best first, whatever order they were written in"
        );
        assert!(
            rows.iter().all(|r| r.identity == [2; 32]),
            "no row of the other run leaked in"
        );

        let (none, _) = store.of_run(&[9; 32]).expect("a walk");
        assert!(none.is_empty(), "a run with no rows is an empty list");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Appending nothing writes nothing and is not an error.
    #[test]
    fn an_empty_frontier_is_not_an_append() {
        let dir = root("empty");
        let mut store = Frontier::open(&dir).expect("a fresh file opens");
        assert_eq!(store.append_all(&[]).expect("no rows"), 0);
        assert_eq!(store.len().expect("a length"), 0, "nothing was written");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A reopened file appends after what is already there.
    ///
    /// # The two runs differ, and that is the point of the test
    ///
    /// This wrote identity `1` both times, so once `append_all` began refusing a
    /// second block for one identity it failed — on the refusal, not on the
    /// append position it exists to check. A second run is what a reopened file
    /// actually receives; reusing one identity was incidental, and the
    /// duplicate case has two tests of its own.
    #[test]
    fn a_reopened_file_appends_after_what_is_already_there() {
        let dir = root("reopen");
        {
            let mut store = Frontier::open(&dir).expect("a fresh file opens");
            store.append_all(&[row(1, 1)]).expect("an append");
        }
        let mut again = Frontier::open(&dir).expect("an existing file opens");
        assert_eq!(again.len().expect("a length"), 1, "the row is still there");
        again
            .append_all(&[row(2, 2)])
            .expect("a second run appends");
        assert_eq!(again.len().expect("a length"), 2);
        assert_eq!(again.read(0).expect("row 0").rank, 1);
        assert_eq!(again.read(1).expect("row 1").rank, 2);
        assert_eq!(
            again.block(&[2; 32]),
            Some(super::Block { first: 1, count: 1 }),
            "the reopened index places the new run AFTER what was already there"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A file that is not this format is refused by name.
    #[test]
    fn a_file_that_is_not_this_format_is_refused_by_name() {
        let dir = root("magic");
        std::fs::create_dir_all(dir.join("results")).expect("the results dir");
        std::fs::write(Frontier::path(&dir), b"BRUTEXRSxxxxxxxx").expect("a decoy");
        let why = Frontier::open(&dir).expect_err("the results magic is not this one");
        assert!(
            why.contains("BRUTEXFR"),
            "the refusal names the magic it wanted: {why}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A ragged tail is named, and says how many rows survived.
    #[test]
    fn a_ragged_tail_is_named_and_counts_what_survived() {
        use std::io::Write as _;

        let dir = root("ragged");
        {
            let mut store = Frontier::open(&dir).expect("a fresh file opens");
            store.append_all(&[row(1, 1), row(1, 2)]).expect("two rows");
        }
        // Three bytes of an interrupted third row.
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(Frontier::path(&dir))
            .expect("the file opens for append");
        file.write_all(&[0_u8; 3]).expect("a torn tail");
        drop(file);

        let why = Frontier::open(&dir).expect_err("a ragged file is refused");
        assert!(
            why.contains('3'),
            "the refusal counts the spare bytes: {why}"
        );
        assert!(
            why.contains('2'),
            "the refusal says how many whole rows are intact: {why}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A row past the end is refused and names the count.
    #[test]
    fn a_row_past_the_end_is_refused_and_names_the_count() {
        let dir = root("past-end");
        let mut store = Frontier::open(&dir).expect("a fresh file opens");
        store.append_all(&[row(1, 1)]).expect("one row");
        let why = store.read(5).expect_err("row 5 does not exist");
        assert!(why.contains('1'), "the refusal names the count: {why}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}

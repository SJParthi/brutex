//! Every trade one run took, on disk, one fixed-stride record each.
//!
//! # What this closes
//!
//! `runs.bin` records a run's TOTALS and nothing about the round trips that
//! produced them: `trades: 5093` is a count, and a count cannot be split back
//! into winners and losers. So the console showed seventy-six padlocks — percent
//! profitable, profit factor, gross profit and loss, largest win, the streaks,
//! the returns histogram, the winners/losers ring, the equity curve, average
//! bars in a trade, and every column of the trade list.
//!
//! **All of them are one file away, and this is the file.** One record per
//! round trip, and every one of those figures is a fold over it.
//!
//! # Why a second file rather than a wider record
//!
//! `CLAUDE.md` §4 bans a dynamic schema and §3 rule 8 bans mutating a format
//! version in place: *a new field is a new file version at its own stride*. A
//! run has one row in `runs.bin` and N trades, so trades cannot be fields on
//! that row at any stride — they are a different shape and they get a different
//! file. `runs.bin` keeps its 205 bytes and its meaning, exactly as it is.
//!
//! # The shape, and why it is addressable
//!
//! ```text
//! header 16 bytes : magic b"BRUTEXTR" (8) | version u32 le (4) | reserved (4)
//! record 40 bytes, little-endian throughout
//! ```
//!
//! Fixed stride, so trade *i* of a run is a computed offset and not a scan. The
//! records for one run are CONTIGUOUS, and `runs.bin`'s own row gains nothing:
//! the index is rebuilt at open in one pass, the same pass `cli::results`
//! already makes for its duplicate set, and for the same stated reason — it is
//! O(runs), it happens once per process, and it is not on a request path.
//!
//! # Nothing here decides anything
//!
//! Every field is copied from [`runner::trade::Trade`], which is where a trade
//! is defined. This module chooses the bytes and nothing else: it does not
//! decide what a trade is, when one opens, which fill model ranks, or what
//! counts as a winner. A second opinion on any of those is the divergence
//! `CLAUDE.md` §3 rule 1 exists to prevent.

use std::fmt::Write as _;
use std::fs::{File, OpenOptions};
use std::io::{Read as _, Seek as _, SeekFrom, Write as _};
use std::path::{Path, PathBuf};

/// Why a trade file could not be written or read, in the operator's words.
pub type Refusal = String;

/// `BRUTEXTR`, so a file that is not this one is refused before it is parsed.
///
/// Deliberately distinct from `runs.bin`'s `BRUTEXRS`: the two files live in
/// one directory and a store root pointed at the wrong place should fail on the
/// magic rather than on a stride that happens to divide.
const MAGIC: [u8; 8] = *b"BRUTEXTR";

/// Version one. A new field is a new version at its own stride.
const VERSION: u32 = 1;

/// Magic, version, four reserved.
const HEADER: u64 = 16;

/// [`HEADER`] as a `usize`. Declared rather than cast — `as usize` is a
/// narrowing on a 32-bit target and `cast_possible_truncation` is denied.
const HEADER_BYTES: usize = 16;

const _: () = assert!(HEADER_BYTES as u64 == HEADER);

/// Bytes per record.
pub const STRIDE: u64 = 40;

/// [`STRIDE`] as a `usize`.
pub const STRIDE_BYTES: usize = 40;

const _: () = assert!(STRIDE_BYTES as u64 == STRIDE);

/// Every field's width, added up in the writer's own order.
///
/// The same compile-time check `crate::backtest::FIELD_SUM` makes against
/// `runs.bin`: `run[32]` is not repeated per trade — the run is identified once
/// by the index below — so a record is `seq u32`, three bar indices at `u32`,
/// two `i64` money figures and one flag byte, padded to a round stride.
const FIELD_SUM: usize = 4 + 4 + 4 + 4 + 8 + 8 + 1 + 7;

const _: () = assert!(FIELD_SUM == STRIDE_BYTES);

/// One completed round trip, as it is stored.
///
/// Field for field [`runner::trade::Trade`], with the bar indices narrowed to
/// `u32` and one `seq` added. Nothing is computed here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Row {
    /// Which trade of the run this is, from zero, in bar order.
    ///
    /// Stored rather than implied by position, so a reader that seeks into the
    /// middle of a run's block can tell whether it landed where it meant to.
    pub seq: u32,
    /// The bar whose close carried the signal. Nothing is traded on it.
    pub signal_bar: u32,
    /// The bar the entry filled in — always `signal_bar + 1`.
    pub entry_bar: u32,
    /// The bar the exit filled in: the horizon, or the square-off.
    pub exit_bar: u32,
    /// Paisa per unit at the friendly end of both bars — open fills.
    pub best: i64,
    /// Paisa per unit at the adverse extreme plus a tick on each leg.
    ///
    /// **The figure a winner is decided by.** Every fold this file exists to
    /// support — percent profitable, gross profit, the histogram — reads THIS
    /// one, because ranking on the flattering reading is what the whole
    /// workspace refuses.
    pub worst: i64,
    /// The exit was the square-off rather than the horizon.
    pub forced: bool,
}

impl Row {
    /// The record as its exact [`STRIDE`] bytes, little-endian throughout.
    #[must_use]
    #[expect(
        clippy::indexing_slicing,
        reason = "every index is a constant offset into a fixed-size array whose \
                  length is const-asserted against FIELD_SUM"
    )]
    pub fn to_bytes(&self) -> [u8; STRIDE_BYTES] {
        let mut out = [0_u8; STRIDE_BYTES];
        let mut at = 0_usize;
        let mut put = |bytes: &[u8], at: &mut usize| {
            out[*at..*at + bytes.len()].copy_from_slice(bytes);
            *at += bytes.len();
        };
        put(&self.seq.to_le_bytes(), &mut at);
        put(&self.signal_bar.to_le_bytes(), &mut at);
        put(&self.entry_bar.to_le_bytes(), &mut at);
        put(&self.exit_bar.to_le_bytes(), &mut at);
        put(&self.best.to_le_bytes(), &mut at);
        put(&self.worst.to_le_bytes(), &mut at);
        put(&[u8::from(self.forced)], &mut at);
        out
    }

    /// The record back from its exact [`STRIDE`] bytes.
    ///
    /// Infallible by construction, as `runs.bin`'s reader is: every field is
    /// fixed-width and every byte pattern is a legal value of its type.
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
        let u32_at = |at: &mut usize| u32::from_le_bytes(take(4, at).try_into().unwrap_or([0; 4]));
        let seq = u32_at(&mut at);
        let signal_bar = u32_at(&mut at);
        let entry_bar = u32_at(&mut at);
        let exit_bar = u32_at(&mut at);
        let best = i64::from_le_bytes(take(8, &mut at).try_into().unwrap_or([0; 8]));
        let worst = i64::from_le_bytes(take(8, &mut at).try_into().unwrap_or([0; 8]));
        let forced = take(1, &mut at).first().copied().unwrap_or(0) != 0;
        Self {
            seq,
            signal_bar,
            entry_bar,
            exit_bar,
            best,
            worst,
            forced,
        }
    }

    /// How many bars the position was held, entry to exit.
    ///
    /// The figure `TradingView` calls "duration (bars)", and the one that makes
    /// "average bars in trades" answerable — a quantity this console has
    /// refused to show precisely because `bars ÷ trades` is a different thing.
    #[must_use]
    pub const fn held_bars(&self) -> u32 {
        self.exit_bar.saturating_sub(self.entry_bar)
    }

    /// This row as one JSON object. Money stays an integer.
    #[must_use]
    pub fn to_json(&self) -> String {
        let mut out = String::with_capacity(160);
        out.push('{');
        let _ = write!(out, r#""seq":{}"#, self.seq);
        let _ = write!(out, r#","signal_bar":{}"#, self.signal_bar);
        let _ = write!(out, r#","entry_bar":{}"#, self.entry_bar);
        let _ = write!(out, r#","exit_bar":{}"#, self.exit_bar);
        let _ = write!(out, r#","held_bars":{}"#, self.held_bars());
        let _ = write!(out, r#","best":{}"#, self.best);
        let _ = write!(out, r#","worst":{}"#, self.worst);
        let _ = write!(out, r#","forced":{}"#, self.forced);
        out.push('}');
        out
    }
}

/// Where the trade file lives beneath a store root.
///
/// Beside `runs.bin`, because the two are read together and a reader that found
/// one should not have to be told where the other is.
#[must_use]
pub fn path_in(root: &Path) -> PathBuf {
    root.join("results").join("trades.bin")
}

/// Where one run's trades begin and how many there are.
///
/// Held in memory, keyed by the run's identity, so answering "the trades for
/// this run" is a probe and a seek rather than a walk. **O(1) per lookup**;
/// the map is built in one pass at open, the same pass and the same stated cost
/// as `cli::results`' duplicate set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Block {
    /// Index of this run's first record.
    pub first: u64,
    /// How many records it owns.
    pub count: u64,
}

/// The append-only file of every trade every recorded run took.
#[derive(Debug)]
pub struct Trades {
    file: File,
    /// Which records belong to which run.
    blocks: std::collections::HashMap<[u8; 32], Block>,
}

impl Trades {
    /// Opens, or creates, the trade file beneath `root`.
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
        let path = path_in(root);
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)
            .map_err(|why| format!("{} could not be opened: {why}", path.display()))?;

        let len = file
            .metadata()
            .map_err(|why| format!("the trade file could not be measured: {why}"))?
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
            return Ok(Self {
                file,
                blocks: std::collections::HashMap::new(),
            });
        }

        let mut header = [0_u8; HEADER_BYTES];
        file.seek(SeekFrom::Start(0))
            .and_then(|_| file.read_exact(&mut header))
            .map_err(|why| format!("the header could not be read: {why}"))?;
        if header.get(..8) != Some(&MAGIC) {
            return Err(format!(
                "{} is not a brutex trade file: its first eight bytes are not \
                 `BRUTEXTR`. Nothing was written.",
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
                "{} is version {version} and this build writes version {VERSION}. \
                 A new field is a new file version at its own stride, so this file \
                 is not appended to.",
                path.display()
            ));
        }

        Ok(Self {
            file,
            blocks: std::collections::HashMap::new(),
        })
    }

    /// How many trade records the file holds. **O(1)** — a division.
    ///
    /// # Errors
    ///
    /// An unreadable file handle.
    pub fn len(&self) -> Result<u64, Refusal> {
        let len = self
            .file
            .metadata()
            .map_err(|why| format!("the trade file could not be measured: {why}"))?
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

    /// Whether this run's trades are already recorded. **O(1).**
    #[must_use]
    pub fn holds(&self, run: &[u8; 32]) -> bool {
        self.blocks.contains_key(run)
    }

    /// Where one run's trades are, if they are here. **O(1).**
    #[must_use]
    pub fn block(&self, run: &[u8; 32]) -> Option<Block> {
        self.blocks.get(run).copied()
    }

    /// Appends one run's trades as a contiguous block.
    ///
    /// **O(trades) writes and no seeks between them** — the block is one
    /// `write_all` of one buffer, so a run of five thousand trades is one
    /// syscall rather than five thousand.
    ///
    /// # Errors
    ///
    /// A run whose trades are already recorded, or an unwritable file. A
    /// duplicate REFUSES rather than appending a second block: §3 rule 5 makes
    /// a rerun byte-identical, so the second block has nothing new to say and
    /// two blocks for one identity would make [`Self::block`] ambiguous.
    pub fn append(&mut self, run: [u8; 32], rows: &[Row]) -> Result<Block, Refusal> {
        if self.holds(&run) {
            return Err(format!(
                "the trades for run {} are already recorded. Same inputs give same \
                 outputs (§3 rule 5), so a second block has nothing to add.",
                hex(&run)
            ));
        }
        let at = self
            .file
            .seek(SeekFrom::End(0))
            .map_err(|why| format!("the trade file could not be extended: {why}"))?;
        let first = at.saturating_sub(HEADER) / STRIDE;

        // ONE BUFFER, ONE WRITE. A per-record write would put a syscall on
        // every round trip, and a run can take tens of thousands.
        let mut buffer = Vec::with_capacity(rows.len() * STRIDE_BYTES);
        for row in rows {
            buffer.extend_from_slice(&row.to_bytes());
        }
        self.file
            .write_all(&buffer)
            .map_err(|why| format!("the trade block could not be written: {why}"))?;
        self.file
            .flush()
            .map_err(|why| format!("the trade block could not be flushed: {why}"))?;

        let count = rows.len() as u64;
        let block = Block { first, count };
        self.blocks.insert(run, block);
        Ok(block)
    }

    /// Reads one run's trades. **O(1) to find, O(count) to read.**
    ///
    /// # Errors
    ///
    /// A run with no block recorded, or an unreadable file.
    pub fn read_block(&mut self, run: &[u8; 32]) -> Result<Vec<Row>, Refusal> {
        let block = self
            .block(run)
            .ok_or_else(|| format!("no trades are recorded for run {}", hex(run)))?;
        let at = HEADER.saturating_add(block.first.saturating_mul(STRIDE));
        let want = usize::try_from(block.count).unwrap_or(0);
        let mut raw = vec![0_u8; want.saturating_mul(STRIDE_BYTES)];
        self.file
            .seek(SeekFrom::Start(at))
            .and_then(|_| self.file.read_exact(&mut raw))
            .map_err(|why| format!("the trade block at byte {at} could not be read: {why}"))?;
        Ok(raw
            .chunks_exact(STRIDE_BYTES)
            .filter_map(|chunk| chunk.try_into().ok())
            .map(|bytes: &[u8; STRIDE_BYTES]| Row::from_bytes(bytes))
            .collect())
    }
}

/// A 32-byte identity as lowercase hex, which is how a run is named everywhere.
fn hex(bytes: &[u8; 32]) -> String {
    let mut s = String::with_capacity(64);
    for byte in bytes {
        let _ = write!(s, "{byte:02x}");
    }
    s
}

/// Everything the console could not show, folded out of one run's trades.
///
/// **This is the whole point of the file.** Every field below was a padlock,
/// and every one is a single pass over the rows — no sort, no second read.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Fold {
    /// Round trips counted.
    pub total: u64,
    /// Trades whose worst-case total was positive.
    pub winners: u64,
    /// Trades whose worst-case total was negative.
    pub losers: u64,
    /// Trades that came out exactly flat. Counted separately because a
    /// breakeven is neither, and folding it into either would move the
    /// percentage that names the strategy.
    pub breakevens: u64,
    /// Sum of the winners, in paisa.
    pub gross_profit: i64,
    /// Sum of the losers, in paisa — negative.
    pub gross_loss: i64,
    /// The single best round trip.
    pub largest_win: i64,
    /// The single worst.
    pub largest_loss: i64,
    /// Bars held across every trade, for the average.
    pub held_bars: u64,
    /// Bars held across the winners only.
    pub winner_bars: u64,
    /// Bars held across the losers only.
    pub loser_bars: u64,
    /// The longest run of consecutive winners.
    pub longest_win_streak: u64,
    /// The longest run of consecutive losers.
    pub longest_loss_streak: u64,
    /// Trades closed by the square-off rather than by their horizon.
    pub forced: u64,
}

impl Fold {
    /// Folds one run's trades. **One pass, no allocation.**
    ///
    /// Reads `worst` throughout — the adverse-extreme figure — because that is
    /// what decides a winner everywhere else in this workspace. Folding on
    /// `best` would produce a percent-profitable that no other surface agrees
    /// with.
    #[must_use]
    pub fn of(rows: &[Row]) -> Self {
        let mut out = Self {
            total: rows.len() as u64,
            largest_win: 0,
            largest_loss: 0,
            ..Self::default()
        };
        let mut win_run = 0_u64;
        let mut loss_run = 0_u64;
        for row in rows {
            out.held_bars = out.held_bars.saturating_add(u64::from(row.held_bars()));
            if row.forced {
                out.forced = out.forced.saturating_add(1);
            }
            // A MATCH ON THE SIGN, because the three arms are three states of
            // one value and an `if` chain lets a fourth be added without the
            // compiler noticing the first three no longer cover it.
            match row.worst.cmp(&0) {
                core::cmp::Ordering::Greater => {
                    out.winners = out.winners.saturating_add(1);
                    out.gross_profit = out.gross_profit.saturating_add(row.worst);
                    out.winner_bars = out.winner_bars.saturating_add(u64::from(row.held_bars()));
                    out.largest_win = out.largest_win.max(row.worst);
                    win_run = win_run.saturating_add(1);
                    loss_run = 0;
                    out.longest_win_streak = out.longest_win_streak.max(win_run);
                }
                core::cmp::Ordering::Less => {
                    out.losers = out.losers.saturating_add(1);
                    out.gross_loss = out.gross_loss.saturating_add(row.worst);
                    out.loser_bars = out.loser_bars.saturating_add(u64::from(row.held_bars()));
                    out.largest_loss = out.largest_loss.min(row.worst);
                    loss_run = loss_run.saturating_add(1);
                    win_run = 0;
                    out.longest_loss_streak = out.longest_loss_streak.max(loss_run);
                }
                core::cmp::Ordering::Equal => {
                    // A BREAKEVEN BREAKS BOTH STREAKS. It is not a win
                    // continuing and not a loss continuing; treating it as
                    // either would make the longest streak a number about the
                    // tie-breaking rule rather than about the strategy.
                    out.breakevens = out.breakevens.saturating_add(1);
                    win_run = 0;
                    loss_run = 0;
                }
            }
        }
        out
    }

    /// Percent profitable in basis points, or [`None`] with no trades.
    ///
    /// Integer throughout: this crate denies `float_arithmetic`, and a ratio
    /// is not a float until it is displayed.
    #[must_use]
    pub const fn profitable_bps(&self) -> Option<u64> {
        if self.total == 0 {
            return None;
        }
        Some(self.winners.saturating_mul(10_000) / self.total)
    }

    /// Profit factor in basis points — gross profit over gross loss.
    ///
    /// [`None`] when there is no loss to divide by, which is a real state and
    /// not an infinity: a run that never lost has no profit factor, and
    /// printing one would be inventing a denominator.
    #[must_use]
    pub const fn profit_factor_bps(&self) -> Option<u64> {
        let loss = self.gross_loss.unsigned_abs();
        if loss == 0 {
            return None;
        }
        Some(self.gross_profit.unsigned_abs().saturating_mul(10_000) / loss)
    }

    /// Average bars held, or [`None`] with no trades.
    ///
    /// **The figure `bars ÷ trades` is NOT.** That is the average gap between
    /// trades; this is the average time inside one, and the console refused to
    /// show the first under the second's name.
    #[must_use]
    pub const fn avg_bars(&self) -> Option<u64> {
        if self.total == 0 {
            return None;
        }
        Some(self.held_bars / self.total)
    }

    /// The fold as JSON, for the page.
    #[must_use]
    pub fn to_json(&self) -> String {
        let mut out = String::with_capacity(512);
        out.push('{');
        let _ = write!(out, r#""total":{}"#, self.total);
        let _ = write!(out, r#","winners":{}"#, self.winners);
        let _ = write!(out, r#","losers":{}"#, self.losers);
        let _ = write!(out, r#","breakevens":{}"#, self.breakevens);
        let _ = write!(out, r#","gross_profit":{}"#, self.gross_profit);
        let _ = write!(out, r#","gross_loss":{}"#, self.gross_loss);
        let _ = write!(out, r#","largest_win":{}"#, self.largest_win);
        let _ = write!(out, r#","largest_loss":{}"#, self.largest_loss);
        let _ = write!(out, r#","longest_win_streak":{}"#, self.longest_win_streak);
        let _ = write!(
            out,
            r#","longest_loss_streak":{}"#,
            self.longest_loss_streak
        );
        let _ = write!(out, r#","forced":{}"#, self.forced);
        match self.profitable_bps() {
            Some(v) => {
                let _ = write!(out, r#","profitable_bps":{v}"#);
            }
            None => out.push_str(r#","profitable_bps":null"#),
        }
        match self.profit_factor_bps() {
            Some(v) => {
                let _ = write!(out, r#","profit_factor_bps":{v}"#);
            }
            None => out.push_str(r#","profit_factor_bps":null"#),
        }
        match self.avg_bars() {
            Some(v) => {
                let _ = write!(out, r#","avg_bars":{v}"#);
            }
            None => out.push_str(r#","avg_bars":null"#),
        }
        out.push('}');
        out
    }
}

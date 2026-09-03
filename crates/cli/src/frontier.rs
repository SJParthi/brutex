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
//! | append a block of R rows | **O(R) time and O(R) buffer space** | validate R rows, encode R fixed strides, then one buffered write |
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
//! A read-only [`Frontier::of_run`] also opens/indexes `runs.bin` and
//! `detail-sets.bin` to prove public commit and exact cardinality. On a fresh
//! HTTP handle that proof is O(total ledger rows + total receipts), before the
//! in-memory O(1) probes and O(selected rows) block read. No end-to-end O(1)
//! latency claim is made.
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
/// Re-exported so `api` can NAME the direction it is served without gaining a
/// `costs` arrow the crate graph does not draw. `cli` already depends on
/// `costs`; `api` depends on `cli`. One arrow, not two.
pub use costs::fill::Direction;

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
///
/// **UNVERIFIED as a measured bound.** No bench in this workspace
/// times this, so the shape above is read from the source rather
/// than measured. `CLAUDE.md` §3 rule 6.
/// # Version 5 — the protective-exit rule joined the stored rule set
///
/// Byte 195, previously the first of five reserved bytes, now carries
/// `Rules::require_protective_exits`. The rules block exists so nothing
/// downstream can judge a row by rules other than the ones it was screened
/// under, and that guarantee is only as complete as the field list: a row that
/// omitted the rule deciding whether an unstopped variant may be selected did
/// not describe its own screen.
///
/// A version 4 file is REFUSED by `read_header`, not reread under version 5
/// meanings. Its byte 195 is a zero that means "reserved", and reading it as
/// `false` would make the row assert a rule was off when the format could not
/// say — §3 rule 8 forbids exactly that reinterpretation, and §4 forbids the
/// silent fallback that would hide it.
const VERSION: u32 = 5;

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
pub const STRIDE: u64 = 272;

/// [`STRIDE`] as a `usize`. Same reason as [`HEADER_BYTES`].
pub const STRIDE_BYTES: usize = 272;

const _: () = assert!(STRIDE_BYTES as u64 == STRIDE);

/// Bytes of `blake3` kept as the seal.
///
/// Eight, matching the results ledger. The seal answers *were these bytes
/// written whole*, not *who wrote them*, and eight bytes make a torn write
/// astronomically unlikely to pass while costing 5.5% of the stride.
const SEAL_BYTES: usize = 8;

/// Bytes of a row the seal covers: everything before the seal itself.
const PAYLOAD_BYTES: usize = STRIDE_BYTES - SEAL_BYTES;

/// Byte carrying the row direction inside the sealed payload.
const DIRECTION_AT: usize = 194;

/// Reserved row bytes after the protective-exit flag, before the stored rule
/// set.
///
/// Four, not five: byte 195 became `Rules::require_protective_exits` at format
/// version 5. See [`VERSION`].
const ROW_RESERVED: core::ops::Range<usize> = 196..200;

/// Reserved bytes in the fixed frontier-file header.
const HEADER_RESERVED: core::ops::Range<usize> = 12..HEADER_BYTES;

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
    /// Total paisa of every WINNING round trip in the chosen cell.
    ///
    /// # Why this is stored when `avg_win` could have been
    ///
    /// MEASURED, 2026-08-29: `Row::derived` rebuilt a `grid::Cell` from the six
    /// v2 fields and asked it for `avg_win` and `avg_loss`. `Cell::avg_win` is
    /// `gross_win / wins` and `Cell::avg_loss` is `gross_loss / losers`, and
    /// NEITHER sum was stored -- `..Default::default()` set both to zero, so
    /// both figures were **structurally always 0**. Two of the eleven criteria
    /// the operator ranks on were dead weight in every score, and a zero average
    /// loss is the BEST possible value, so the ranking was quietly rewarding
    /// rows for a number nobody had measured.
    ///
    /// The SUM and not the average, because an average cannot be re-derived
    /// into anything else while a sum can: `gross_win / wins` gives the average
    /// back, and the sum also answers "how much did the winners make in total",
    /// which an average alone cannot. §5's argument against storing a derived
    /// figure where the raw one fits.
    pub gross_win: i64,
    /// Total paisa of every LOSING round trip in the chosen cell. Negative.
    pub gross_loss: i64,
    /// WHICH WAY TO TRADE THIS COMBINATION.
    ///
    /// # It was computed on every path and reached no surface at all
    ///
    /// `side_of_evidence` picks a side for every row the screen prices, and the
    /// engine's whole pricing layer is symmetric about it — `fills_at`,
    /// `BarMoves::of`, `level_price` and `peak` each resolve the two cases with
    /// a comment explaining why. Then the sign was DROPPED: `struct Screened`
    /// computes a side and has no field for it, `report.rs` and `audit.rs` have
    /// no column, no `api` endpoint carries the key, the ledger has no byte for
    /// it, and the browser's Long/Short columns are every one a padlock.
    ///
    /// So an operator could read a ranked combination, its win rate, its payoff
    /// and its drawdown — and could not act on ANY of it, because nothing said
    /// which way round the trade goes. `traded_line` exists in this crate
    /// precisely because *"a number whose subject is unstated is a number that
    /// cannot be checked"*, and it printed hits, n, mean and t without the side.
    ///
    /// # Why it is stored rather than re-derived
    ///
    /// It IS derivable — `mean_paisa < 0.0` is the whole rule — and the sign of
    /// the mean does reach `/frontier.json`. But re-deriving it in the browser
    /// would be the second definition of one fact that `CLAUDE.md` §5 refuses,
    /// correct the day it is written and silently wrong the first time the rule
    /// gains a tie-break. Two of the three surfaces an operator selects from
    /// (the screened table and the results ledger) carry no mean at all, so
    /// there it is not merely a copy — it is unrecoverable.
    ///
    /// One byte, taken from the six-byte reserve rather than by widening the
    /// stride, which is exactly what that reserve was written for: *"so the next
    /// field does not need a new version for a small addition."*
    pub direction: Direction,
    /// The rules THIS RUN judged by.
    ///
    /// # A verdict computed from rules no run applied
    ///
    /// `api::frontierjson` read `cli::Rules::operator()` at REQUEST time and
    /// compared these rows against it, while the run that WROTE them swept at
    /// `Rules::derived(..)` or at `Rules::elite(..)`. Three separate mechanisms
    /// guaranteed the two differed:
    ///
    /// * `derived` MEASURES four of its floors off the bars — the EXECUTION
    ///   series' bars, per `cli::floors_measured_on` — and the handler has an
    ///   identity and a row, not a span, so they are unrecoverable there;
    /// * `sweeprun::Applied::drop` calls `knobs::clear_all`, so a floor the
    ///   operator typed into the form steers the sweep and is GONE before the
    ///   browser fetches the result;
    /// * `elite` differs from `operator` on three of the five rules `verdict`
    ///   checks, unconditionally, on every descend-produced run.
    ///
    /// Both directions were reachable. A row the run REFUSED rendered PASS and a
    /// row it ADMITTED rendered FAIL, with the wrong threshold printed beside
    /// it — a fallback that hides a failure, which `CLAUDE.md` §4 bans by name.
    ///
    /// # Why per row and not once per run
    ///
    /// The tidier home is `results::Record`, and it was rejected for a reason
    /// that is not tidiness: `record_all` calls `record_frontier` even when
    /// `record_run`'s ledger append refuses a duplicate, so a per-run lookup can
    /// legitimately MISS and the handler would be back to guessing. A row cannot
    /// miss. `identity` is already duplicated per row for the same reason.
    ///
    /// `append_all` writes a run's whole frontier in one call under one lock
    /// from one `Rules` value, so every row of a block carries the same set —
    /// and `policy_of` folds all eight into the run identity, so two runs
    /// sharing an identity necessarily shared their rules.
    pub rules: crate::Rules,
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
        put(&self.min_win.to_le_bytes(), &mut at); // 170   8
        put(&self.gross_win.to_le_bytes(), &mut at); // 178   8
        put(&self.gross_loss.to_le_bytes(), &mut at); // 186   8 -> ends at 194
        // WHICH WAY TO TRADE IT, one byte, taken from the reserve below rather
        // than by widening the stride -- which is what that reserve was
        // written for. Zero is long and one is short, and the two are named by
        // `Direction::as_str`, which until now had no production caller at all.
        put(
            &[match self.direction {
                Direction::Long => 0_u8,
                Direction::Short => 1,
            }],
            &mut at,
        ); // 194   1 -> 195
        // BYTE 195 IS NOW `require_protective_exits`, AT FORMAT VERSION 5.
        //
        // The rules block below records what a run judged by, so that nothing
        // downstream can judge by others — and a rule missing from it is that
        // guarantee quietly broken. `require_protective_exits` is the rule that
        // decides whether a variant with no stop may be selected at all, so a
        // row that omits it does not say what it was screened for.
        //
        // Taken from the reserve at a NEW version rather than reinterpreted at
        // version 4, which is what `CLAUDE.md` §3 rule 8 forbids: a v4 file has
        // a zero here meaning "reserved", and reading that as `false` would be
        // this row asserting a rule was off when the format simply could not
        // say. `read_header` refuses a version it does not know, loudly, which
        // is the §4-compliant outcome — v4 frontiers are not silently reread
        // under v5 meanings.
        //
        // 196..200 stay zero: four bytes remain reserved.
        put(&[u8::from(self.rules.require_protective_exits)], &mut at); // 195   1 -> 196
        // Advanced over EXPLICITLY now that something follows it. While the
        // reserve was the last thing on the row, leaving `at` short of it was
        // the same as skipping it; with the rules after it, a missing advance
        // would silently write them four bytes early.
        put(&[0_u8; 4], &mut at); // 196   4 -> 200
        // THE RULES THIS RUN JUDGED BY, so nothing downstream can judge by
        // others. Eight fields in declaration order, which is the order
        // `from_bytes` reads them back in.
        put(&self.rules.max_mae_ppm.to_le_bytes(), &mut at); // 200   8
        put(&self.rules.min_rr_bp.to_le_bytes(), &mut at); // 208   8
        put(&self.rules.min_win_rate_bp.to_le_bytes(), &mut at); // 216   8
        put(&self.rules.min_assurance_bp.to_le_bytes(), &mut at); // 224   8
        put(&self.rules.min_weakest_bp.to_le_bytes(), &mut at); // 232   8
        put(&self.rules.min_ret_over_dd_bp.to_le_bytes(), &mut at); // 240   8
        put(&self.rules.min_trades.to_le_bytes(), &mut at); // 248   8
        // `top` is a `usize`, which is not a width the disk can carry. Saturated
        // rather than truncated: a `top` past `u64` is not a number an operator
        // typed, and wrapping it would record a small one.
        put(
            &u64::try_from(self.rules.top)
                .unwrap_or(u64::MAX)
                .to_le_bytes(),
            &mut at,
        ); // 256   8 -> ends at 264
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
    ///
    /// # Errors
    ///
    /// Refuses a direction other than `0=long` or `1=short`, and any non-zero
    /// reserved byte. A valid seal proves those bytes were written together; it
    /// does not give an unknown value a meaning this format never assigned.
    #[allow(
        clippy::indexing_slicing,
        reason = "the same compile-time constants `to_bytes` writes at, over an \
                  array of the same asserted length."
    )]
    pub fn from_bytes(raw: &[u8; STRIDE_BYTES]) -> Result<Self, Refusal> {
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
        let hits = u64::from_le_bytes(take(8, &mut at).try_into().unwrap_or([0; 8]));
        let n = u64::from_le_bytes(take(8, &mut at).try_into().unwrap_or([0; 8]));
        let mean_milli_paisa = i64::from_le_bytes(take(8, &mut at).try_into().unwrap_or([0; 8]));
        let t_milli = i64::from_le_bytes(take(8, &mut at).try_into().unwrap_or([0; 8]));
        let payoff_bp = i64::from_le_bytes(take(8, &mut at).try_into().unwrap_or([0; 8]));
        let wins = u64::from_le_bytes(take(8, &mut at).try_into().unwrap_or([0; 8]));
        let trades = u64::from_le_bytes(take(8, &mut at).try_into().unwrap_or([0; 8]));
        let cell_wins = u64::from_le_bytes(take(8, &mut at).try_into().unwrap_or([0; 8]));
        let pessimistic = i64::from_le_bytes(take(8, &mut at).try_into().unwrap_or([0; 8]));
        let worst_trade = i64::from_le_bytes(take(8, &mut at).try_into().unwrap_or([0; 8]));
        let max_drawdown = i64::from_le_bytes(take(8, &mut at).try_into().unwrap_or([0; 8]));
        let min_win = i64::from_le_bytes(take(8, &mut at).try_into().unwrap_or([0; 8]));
        let gross_win = i64::from_le_bytes(take(8, &mut at).try_into().unwrap_or([0; 8]));
        let gross_loss = i64::from_le_bytes(take(8, &mut at).try_into().unwrap_or([0; 8]));

        let direction_byte = take(1, &mut at).first().copied().unwrap_or(u8::MAX);
        let direction = match direction_byte {
            0 => Direction::Long,
            1 => Direction::Short,
            other => {
                return Err(format!(
                    "frontier row direction byte {DIRECTION_AT} is {other}; only 0=long and 1=short are defined"
                ));
            }
        };
        // BYTE 195, THE PROTECTIVE-EXIT RULE. Two values are defined and a third
        // is refused rather than coerced: a byte this row does not understand
        // means the row was written by a schema this build does not know, and
        // reading `2` as `true` would be a guess wearing a value's clothes.
        let protective_byte = take(1, &mut at).first().copied().unwrap_or(0);
        let require_protective_exits = match protective_byte {
            0 => false,
            1 => true,
            other => {
                return Err(format!(
                    "frontier row protective-exit byte 195 is {other}; only 0=off and 1=required are defined for format version {VERSION}"
                ));
            }
        };
        let reserve = take(4, &mut at);
        if let Some((offset, byte)) = reserve
            .iter()
            .copied()
            .enumerate()
            .find(|(_, byte)| *byte != 0)
        {
            return Err(format!(
                "frontier row reserved byte {} is {byte}; every byte in 196..200 must be zero for format version {VERSION}",
                ROW_RESERVED.start.saturating_add(offset)
            ));
        }

        let rules = crate::Rules {
            max_mae_ppm: i64::from_le_bytes(take(8, &mut at).try_into().unwrap_or([0; 8])),
            min_rr_bp: i64::from_le_bytes(take(8, &mut at).try_into().unwrap_or([0; 8])),
            min_win_rate_bp: i64::from_le_bytes(take(8, &mut at).try_into().unwrap_or([0; 8])),
            min_assurance_bp: i64::from_le_bytes(take(8, &mut at).try_into().unwrap_or([0; 8])),
            min_weakest_bp: i64::from_le_bytes(take(8, &mut at).try_into().unwrap_or([0; 8])),
            min_ret_over_dd_bp: i64::from_le_bytes(take(8, &mut at).try_into().unwrap_or([0; 8])),
            min_trades: u64::from_le_bytes(take(8, &mut at).try_into().unwrap_or([0; 8])),
            // READ FROM BYTE 195 ABOVE, not defaulted. See the write side for
            // why this took a reserved byte at version 5 rather than being
            // reinterpreted at version 4.
            require_protective_exits,
            // Saturated back, matching the write. A stored `u64::MAX` means
            // "past what a `usize` holds", which on this target it also is.
            top: usize::try_from(u64::from_le_bytes(
                take(8, &mut at).try_into().unwrap_or([0; 8]),
            ))
            .unwrap_or(usize::MAX),
        };

        Ok(Self {
            identity,
            rank,
            mask_words,
            hits,
            n,
            mean_milli_paisa,
            t_milli,
            payoff_bp,
            wins,
            trades,
            cell_wins,
            pessimistic,
            worst_trade,
            max_drawdown,
            min_win,
            gross_win,
            gross_loss,
            direction,
            rules,
        })
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
        // The side `cell` was PRICED at, carried rather than re-derived. `None`
        // for an unpriced row, which has no side because it has no trades.
        direction: Option<Direction>,
        rules: crate::Rules,
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
            gross_win: cell.map_or(0, |c| c.gross_win),
            gross_loss: cell.map_or(0, |c| c.gross_loss),
            // THE SIDE THE MONEY ABOVE WAS PRICED AT, handed in beside the cell.
            //
            // This read `side_of_evidence(scored)` and its comment claimed that
            // "cannot drift from what the screen actually traded". It could, the
            // moment `screen` began pricing BOTH sides and keeping the better:
            // every money field on this row comes from the winning side's cell,
            // and the direction byte came from the raw unexited mean. A row
            // could store SHORT money under a LONG direction.
            //
            // This is the PERSISTED surface — `/frontier.json` and the browser
            // render this byte verbatim — and it contradicted its own sibling in
            // the same result set, since the detail receipt already stored the
            // priced direction. Two files, one run, opposite answers.
            //
            // Carried rather than derived, so there is no second definition to
            // drift. An UNPRICED row has no side to name and keeps the evidence
            // reading: it has no money either, and `trades == 0` is how a reader
            // tells the two apart.
            direction: direction
                .unwrap_or_else(|| crate::direction_of(crate::side_of_evidence(scored))),
            rules,
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
    /// Store root, kept so a read-only detail lookup can prove its ledger
    /// parent exists before exposing the block.
    root: PathBuf,
    /// Writers may inspect a prepared block before its ledger commit exists;
    /// public readers may not.
    require_parent: bool,
    /// Which rows belong to which run — built in one pass at open.
    blocks: std::collections::HashMap<[u8; 32], Block>,
    /// First byte not yet absorbed into `blocks`. A stale writer advances this
    /// under the file lock before it performs duplicate rejection.
    scanned: u64,
    /// First integrity failure found while this handle indexed whole rows.
    ///
    /// Read-only handles retain every unambiguous block range so one damaged orphan
    /// does not take every committed run offline. Writer operations are
    /// different: appending beyond unexplained history, or blessing an existing
    /// prepared block as durable beside it, would silently turn corruption into
    /// an accepted prefix. They refuse while this is present.
    write_refusal: Option<Refusal>,
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

/// One open-time pass's unambiguous block map and first global integrity failure.
struct Indexed {
    blocks: std::collections::HashMap<[u8; 32], Block>,
    write_refusal: Option<Refusal>,
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
        Self::open_read_bounded(root, u64::MAX)
    }

    /// Opens read-only while refusing a file larger than `max_bytes` before
    /// the open-time index walk begins.
    ///
    /// The ordinary CLI reader uses [`u64::MAX`] through [`Self::open_read`].
    /// HTTP callers use a finite ceiling because rebuilding an index is
    /// blocking O(rows) work and a request boundary must name its maximum.
    ///
    /// # Errors
    ///
    /// In addition to [`Self::open_read`]'s refusals, names the measured byte
    /// length when it exceeds `max_bytes`. No row is indexed in that case.
    pub fn open_read_bounded(root: &Path, max_bytes: u64) -> Result<Self, Refusal> {
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
        if len > max_bytes {
            return Err(format!(
                "{} is {len} bytes; this reader's hard index ceiling is {max_bytes} bytes. No partial frontier index was built",
                path.display()
            ));
        }
        check_header(&mut file, &path, len)?;
        let Indexed {
            blocks,
            write_refusal,
        } = index_of(&mut file, len)?;
        Ok(Self {
            file,
            path,
            root: root.to_path_buf(),
            require_parent: true,
            blocks,
            scanned: len,
            write_refusal,
        })
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
                root: root.to_path_buf(),
                require_parent: false,
                blocks: std::collections::HashMap::new(),
                scanned: HEADER,
                write_refusal: None,
            });
        }
        check_header(&mut file, &path, len)?;
        let Indexed {
            blocks,
            write_refusal,
        } = index_of(&mut file, len)?;
        Ok(Self {
            file,
            path,
            root: root.to_path_buf(),
            require_parent: false,
            blocks,
            scanned: len,
            write_refusal,
        })
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
        let Some(first_row) = rows.first() else {
            return self.len();
        };
        let identity = first_row.identity;
        if let Some((at, foreign)) = rows
            .iter()
            .enumerate()
            .find(|(_, row)| row.identity != identity)
        {
            return Err(format!(
                "frontier row {at} belongs to run {}, not run {}. One append must be one contiguous identity block, so no bytes were written",
                hex(&foreign.identity),
                hex(&identity)
            ));
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
        self.append_locked_with(rows, std::io::Write::write_all)
    }

    /// The append body with an injectable write used to prove partial-write
    /// rollback against a real file. Production passes [`Write::write_all`].
    fn append_locked_with(
        &mut self,
        rows: &[Row],
        write: impl FnOnce(&mut File, &[u8]) -> std::io::Result<()>,
    ) -> Result<u64, Refusal> {
        // This handle may have been opened before another process appended.
        // Refresh while holding the same exclusive lock that protects the
        // write, then repeat duplicate rejection against current disk state.
        self.absorb_new_rows()?;
        for row in rows {
            if self.holds(&row.identity) {
                return Err(format!(
                    "run {} already has a frontier block. Same inputs give same outputs (§3 rule 5), so a second block adds nothing",
                    hex(&row.identity)
                ));
            }
        }
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
        // `write_all` may extend a regular file and then fail (for example when
        // the filesystem fills). Those prefix bytes are not a row. Leaving
        // them behind makes `check_header` refuse the complete shared frontier
        // on every later open, hiding older committed blocks and preventing an
        // exact rerun from recovering. `end` was measured under the exclusive
        // lock, so truncating to it removes only this call's uncommitted bytes.
        write(&mut self.file, &buffer).map_err(|why| match self.file.set_len(end) {
            Ok(()) => format!(
                "the frontier rows could not be written: {why}. The partial write was rolled back to byte {end}, so every older whole row remains readable"
            ),
            Err(and) => format!(
                "the frontier rows could not be written: {why}. Rolling the partial write back to byte {end} ALSO failed: {and}. The file may now end mid-row and is refused until its tail is repaired"
            ),
        })?;
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
        self.scanned = end.saturating_add(buffer.len() as u64);
        self.len()
    }

    /// Absorbs whole rows appended since this handle opened.
    ///
    /// O(rows appended by other writers), which is zero on the ordinary
    /// result-set path because its outer lock serialises all four children.
    fn absorb_new_rows(&mut self) -> Result<(), Refusal> {
        self.refuse_integrity_failure_for_write()?;
        let len = self
            .file
            .metadata()
            .map_err(|why| format!("{} could not be measured: {why}", self.path.display()))?
            .len();
        if len < self.scanned {
            return Err(format!(
                "{} shrank from byte {} to {len} while this handle was open. Append-only history was replaced and no frontier was written",
                self.path.display(),
                self.scanned
            ));
        }
        if len < HEADER || !len.saturating_sub(HEADER).is_multiple_of(STRIDE) {
            return Err(format!(
                "{} has length {len}, which is not a {HEADER}-byte header plus whole {STRIDE}-byte frontier rows. A torn tail is never ignored or padded",
                self.path.display()
            ));
        }

        let mut raw = [0_u8; STRIDE_BYTES];
        while self.scanned.saturating_add(STRIDE) <= len {
            let at = self.scanned;
            let index = at.saturating_sub(HEADER) / STRIDE;
            self.file
                .seek(SeekFrom::Start(at))
                .and_then(|_| self.file.read_exact(&mut raw))
                .map_err(|why| format!("frontier row at byte {at} could not be read: {why}"))?;
            if !Row::seal_matches(&raw) {
                self.write_refusal.get_or_insert_with(|| {
                    format!("whole frontier row {index} whose integrity seal failed")
                });
                return Err(format!(
                    "frontier row {index}, appended after this handle opened, does not match its seal. No later row was absorbed and nothing was written"
                ));
            }
            let row = match Row::from_bytes(&raw) {
                Ok(row) => row,
                Err(why) => {
                    let refusal =
                        format!("frontier row {index} is sealed but its schema is invalid: {why}");
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
                        "{} gained a non-contiguous duplicate frontier block for run {} while this handle was open. The index is ambiguous and nothing was written",
                        self.path.display(),
                        hex(&row.identity)
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
            self.scanned = self.scanned.saturating_add(STRIDE);
        }
        Ok(())
    }

    /// A writer cannot extend or promote a file after a bad seal, invalid schema
    /// or non-contiguous identity block. Read-only callers retain unambiguous
    /// ranges and diagnose selected damaged rows at read time.
    fn refuse_integrity_failure_for_write(&self) -> Result<(), Refusal> {
        if let Some(why) = &self.write_refusal {
            return Err(format!(
                "{} contains {why}. Append-only history is damaged or ambiguous, so no row may be appended and no prepared block may be promoted until a reviewed forensic repair installs a replacement",
                self.path.display(),
            ));
        }
        Ok(())
    }

    /// Repeats the child's durability barrier before an exact prepared block
    /// is promoted to a committed result set.
    ///
    /// # Errors
    ///
    /// Names a shared-lock, durability-barrier, or unlock failure. A caller
    /// must not promote the child to the public ledger after any such refusal.
    pub fn confirm_durable(&mut self) -> Result<(), Refusal> {
        self.file
            .lock_shared()
            .map_err(|why| format!("the frontier file could not be locked for syncing: {why}"))?;
        let synced = self.absorb_new_rows().and_then(|()| {
            self.file
                .sync_all()
                .map_err(|why| format!("the prepared frontier could not be synced: {why}"))
        });
        let released = self
            .file
            .unlock()
            .map_err(|why| format!("the frontier file could not be unlocked after syncing: {why}"));
        match (synced, released) {
            (Ok(()), Ok(())) => Ok(()),
            (Err(why), _) | (Ok(()), Err(why)) => Err(why),
        }
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
        Row::from_bytes(&raw)
            .map_err(|why| format!("row {index} is sealed but its schema is invalid: {why}"))
    }

    /// Every row belonging to one run, best rank first.
    ///
    /// **Inside this already-open frontier handle, finding the block is O(1)
    /// and reading it is O(count).** One hash probe
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
    /// opens a frontier file. A read-only call additionally opens/indexes the
    /// whole parent ledger and receipt sidecar, so fresh-handle end-to-end
    /// lookup is O(total ledger rows + total receipts + selected rows), not
    /// O(1). Closing the local-block half is a fifth row over `rows_for` at
    /// 1×/10×/100× the BLOCK count, which is the axis the claim is about — the
    /// row count is the `O(count)` half and is not in dispute.
    ///
    /// The rows returned are filtered to `identity`. A block written before
    /// `append_all` refused duplicates can span another run's rows, and §8 keeps
    /// those files as they are.
    ///
    /// A writer/recovery handle keeps valid rows beside a damage diagnostic so
    /// an exact rerun can refuse with evidence. A public read-only handle has a
    /// committed receipt and is all-or-nothing: any damaged/foreign/missing row
    /// refuses the entire block and exposes no valid prefix.
    ///
    /// # Errors
    ///
    /// Refuses when commit/receipt proof, length, locking, seeking or reading
    /// fails. Public handles also refuse any damaged, foreign or short content.
    /// Recovery handles return their valid rows plus the damage diagnostic so
    /// the caller can compare/refuse the interrupted preparation.
    pub fn of_run(&mut self, identity: &[u8; 32]) -> Result<(Vec<Row>, Option<Refusal>), Refusal> {
        let receipt = if self.require_parent {
            crate::result_set::committed_receipt(&self.root, identity)?
        } else {
            None
        };
        self.read_against_receipt(identity, receipt)
    }

    /// Reads one public frontier block against one already-verified receipt.
    ///
    /// This is the bounded HTTP transaction door. The caller captures the
    /// parent/receipt snapshot once, then opens this child and reconciles that
    /// exact identity and row count without silently reopening either parent.
    /// A `None` proof can describe an absent run, but can never publish an
    /// orphan child block as a committed empty or complete result.
    ///
    /// # Errors
    ///
    /// Refuses writer handles, a foreign receipt identity, any receipt/index
    /// count mismatch, orphan/damaged/foreign child content, seek, lock or read
    /// failures. No valid prefix is returned by a public handle.
    pub fn of_run_against_receipt(
        &mut self,
        identity: &[u8; 32],
        receipt: Option<crate::result_set::Receipt>,
    ) -> Result<(Vec<Row>, Option<Refusal>), Refusal> {
        if !self.require_parent {
            return Err(
                "an already-verified committed receipt can only be applied to a read-only frontier handle"
                    .to_owned(),
            );
        }
        self.read_against_receipt(identity, receipt)
    }

    fn read_against_receipt(
        &mut self,
        identity: &[u8; 32],
        receipt: Option<crate::result_set::Receipt>,
    ) -> Result<(Vec<Row>, Option<Refusal>), Refusal> {
        let block = self.block(identity);
        if let Some(receipt) = receipt {
            if receipt.identity != *identity {
                return Err(format!(
                    "receipt for run {} was applied to frontier run {}. No rows are exposed across identities",
                    hex(&receipt.identity),
                    hex(identity)
                ));
            }
            let actual = block.map_or(0, |held| held.count);
            if actual != receipt.frontier_rows {
                return Err(format!(
                    "run {} commits a receipt for {} frontier row(s), but frontier.bin indexes {actual}. The result set is incomplete or damaged. No partial frontier is exposed",
                    hex(identity),
                    receipt.frontier_rows
                ));
            }
        }
        let parent_is_complete = receipt.is_some();

        // O(1) TO FIND. This walked the whole file and read every row to check
        // its identity -- roughly five syscalls per row, because `read` takes a
        // `metadata`, a shared lock, a seek, a read and an unlock EACH. A
        // thousand recorded runs at twenty-five rows apiece was over a hundred
        // thousand syscalls to answer one question about one run.
        let Some(block) = block else {
            return Ok((Vec::new(), None));
        };

        // THE LEDGER ROW IS THE COMMIT MARKER.
        //
        // A writer prepares this block before it appends `runs.bin`. That order
        // makes a crash recoverable, but it also means the bytes can exist for
        // a run that did not commit. A direct `/frontier.json?identity=...`
        // request knows the identity without visiting `/backtest.json`, so the
        // absence of a parent must be checked here rather than left to the UI.
        // Writer handles deliberately skip this check: an exact rerun has to
        // inspect and byte-verify the prepared block before it can finish the
        // missing ledger append.
        if self.require_parent && !parent_is_complete {
            return Err(format!(
                "run {} has a prepared frontier block but no results-ledger row. The run did not commit; the block is hidden. Re-run the exact same inputs to verify and finish it.",
                hex(identity)
            ));
        }

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

        // RECOVERY KEEPS THE DIAGNOSTIC BESIDE VALID ROWS; PUBLICATION DOES NOT.
        // A writer handle needs the prefix to explain why it cannot equal an
        // exact rerun. A read-only handle has promised one receipted result set
        // and refuses below before any prefix can escape.
        let mut found: Vec<Row> = Vec::with_capacity(raw.len() / STRIDE_BYTES);
        let mut damaged: Option<Refusal> = None;
        let mut foreign = 0_u64;
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
                match Row::from_bytes(&bytes) {
                    Ok(row) if &row.identity == identity => found.push(row),
                    Ok(_) => foreign = foreign.saturating_add(1),
                    Err(why) if damaged.is_none() => {
                        damaged = Some(format!(
                            "row {} of run {} is sealed but its schema is invalid: {why}. Nothing was changed.",
                            block.first.saturating_add(nth as u64),
                            hex(identity)
                        ));
                    }
                    Err(_) => {}
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
        let found_count = u64::try_from(found.len()).unwrap_or(u64::MAX);
        if self.require_parent && (damaged.is_some() || foreign > 0 || found_count != block.count) {
            return Err(format!(
                "run {} has a committed receipt for a {}-row frontier block, but reading that block found {} valid own row(s), {foreign} foreign row(s), and damage={}. No partial frontier is exposed{}",
                hex(identity),
                block.count,
                found.len(),
                damaged.is_some(),
                damaged
                    .as_deref()
                    .map_or(String::new(), |why| format!(": {why}"))
            ));
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
/// A footer would have to be rewritten on every append, adding a
/// read-modify-write and a second thing in the file that can disagree with the
/// first. The pass is O(rows) once per process. Appending R rows is already
/// O(R) time and O(R) buffer space even though it issues one buffered write;
/// syscall count does not make the bytes or validation constant-cost.
///
/// **A row whose seal fails is omitted from the block index rather than making
/// every other run unreadable, and its first index is retained separately.**
/// Public handles can still read unaffected committed blocks and diagnose the
/// selected damaged row. Writer handles refuse every append and durability
/// promotion while that marker exists; skipping a corrupt orphan must not make
/// it safe to extend append-only history. Refusing to open the whole file over
/// one bad row would take a store with 999 good runs offline for the thousandth.
///
/// **UNVERIFIED as a measurement.** The bound is argued from the
/// shape of the code and no bench in this workspace times it.
/// `CLAUDE.md` §3 rule 6: a structural argument is not a
/// measurement, however sound it is.
fn index_of(file: &mut File, len: u64) -> Result<Indexed, Refusal> {
    let count = len.saturating_sub(HEADER) / STRIDE;
    let mut blocks: std::collections::HashMap<[u8; 32], Block> =
        std::collections::HashMap::with_capacity(
            usize::try_from(count).unwrap_or(0).saturating_div(8).max(8),
        );
    file.seek(SeekFrom::Start(HEADER))
        .map_err(|why| format!("the frontier file could not be seeked: {why}"))?;

    // ONE SYSCALL PER 8 KiB, NOT ONE PER 208-BYTE ROW.
    //
    // This read straight from the `File`, so `read_exact` was one `read(2)` per
    // row -- and `open_read` runs it on EVERY `/frontier.json` request, so
    // asking for one run's twenty-five rows walked every frontier row ever
    // recorded, one syscall each. An O(1) audit measured the shape as "O(rows),
    // 1 read_exact syscall/row".
    //
    // A `BufReader` does not change the O(rows) walk -- only a persisted or
    // cached index does that -- but it divides the syscall count by the rows
    // that fit in a buffer: at 208 bytes and the default 8 KiB, 39 per read.
    // The borrow is scoped so the `File`'s cursor is free afterwards, and every
    // later reader seeks explicitly before it reads.
    let mut buffered = std::io::BufReader::new(&mut *file);
    let mut raw = [0_u8; STRIDE_BYTES];
    let mut write_refusal = None;
    for index in 0..count {
        buffered
            .read_exact(&mut raw)
            .map_err(|why| format!("row {index} could not be read while indexing: {why}"))?;
        if !Row::seal_matches(&raw) {
            write_refusal.get_or_insert_with(|| {
                format!("whole frontier row {index} whose integrity seal failed")
            });
            continue;
        }
        // A valid seal makes the identity bytes trustworthy, but it does not
        // make an unknown schema byte meaningful. Decode every sealed row so a
        // fresh writer inherits the same refusal a stale writer would have seen.
        let mut identity = [0_u8; 32];
        identity.copy_from_slice(raw.get(..32).unwrap_or(&[0_u8; 32]));
        if let Err(why) = Row::from_bytes(&raw) {
            write_refusal.get_or_insert_with(|| {
                format!("frontier row {index} is sealed but its schema is invalid: {why}")
            });
        }

        match blocks.get_mut(&identity) {
            Some(block) if block.first.saturating_add(block.count) == index => {
                block.count = block.count.saturating_add(1);
            }
            Some(_) => {
                write_refusal.get_or_insert_with(|| {
                    format!(
                        "frontier row {index} starts a non-contiguous duplicate block for run {}",
                        hex(&identity)
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
        // THE REMEDY IS THE OPERATOR'S OWN HAND, AND SAYING "RE-RUN THE SWEEP"
        // WAS A NO-OP DRESSED AS AN INSTRUCTION.
        //
        // This message used to end "The file is REGENERABLE -- re-run the sweep
        // and it is written afresh". Nothing in the tree renames or removes
        // this file: `grep remove_file|rename` across `crates/` finds one hit
        // and it is `live.rs`. So `open` refuses, the sweep never reaches a
        // write, and re-running produces the identical refusal FOREVER. The
        // operator was handed an action that cannot work, which is worse than
        // being handed none -- §4's banned fallback wearing an instruction's
        // clothes.
        //
        // It is not repaired automatically either, and that is deliberate: this
        // is the operator's recorded history, §3 rule 8 keeps it, and a build
        // that silently deletes a file it cannot read is a different and worse
        // failure. The path and the exact command are named instead, so the
        // action is one line and the decision stays theirs.
        let aside = path.with_extension(format!("v{version}.bin"));
        return Err(format!(
            "{} is frontier format version {version}; this build writes and reads \
             version {VERSION}. A format version is never mutated in place — §3 \
             rule 8, and it is not widened in place either: a row read with \
             zeroes in the new fields would carry a rule set of all-zero floors, \
             which every priced row passes — the fallback that hides a failure \
             §4 bans, wearing a migration's clothes.\n\
             \n\
             THIS BUILD WILL NOT WRITE A FRONTIER UNTIL THE FILE IS MOVED ASIDE, \
             and it does not move it for you. Run:\n\
             \n\
             \x20   mv {} {}\n\
             \n\
             then sweep again — the rows are regenerable from the bars, which is \
             why moving it is safe. Nothing was written.",
            path.display(),
            path.display(),
            aside.display()
        ));
    }
    if let Some((offset, byte)) = header
        .get(HEADER_RESERVED.clone())
        .unwrap_or_default()
        .iter()
        .copied()
        .enumerate()
        .find(|(_, byte)| *byte != 0)
    {
        return Err(format!(
            "{} has reserved header byte {} set to {byte}; bytes 12..16 must be zero for frontier format version {VERSION}. The file may belong to an unknown schema and nothing was written.",
            path.display(),
            HEADER_RESERVED.start.saturating_add(offset)
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
    use super::{
        DIRECTION_AT, Frontier, HEADER_BYTES, HEADER_RESERVED, MAGIC, PAYLOAD_BYTES, ROW_RESERVED,
        Row, SEAL_BYTES, STRIDE, STRIDE_BYTES, seal_of,
    };

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
            direction: costs::fill::Direction::Long,
            rules: crate::Rules::elite(400, 25),
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
            gross_win: 0,
            gross_loss: 0,
        }
    }

    /// The row this operator's own store actually holds, top-ranked.
    ///
    /// MEASURED from `/frontier.json` on 2026-08-29: rank 1 of the newest run
    /// took 868 trades, won 3 of them, and its smallest win was 295 paisa
    /// against a worst loss of 3,035. It sat at the TOP of a table headed
    /// "ranked on your criteria".
    fn measured_rank_one() -> Row {
        Row {
            trades: 868,
            cell_wins: 3,
            pessimistic: -272_749,
            worst_trade: -3_035,
            max_drawdown: 329_165,
            min_win: 295,
            gross_win: 900,
            gross_loss: -273_649,
            ..row(1, 1)
        }
    }

    /// The row the operator has been shown at rank 1 fails four of five rules.
    ///
    /// This is the test that would have caught it. `record_frontier` writes the
    /// top `top` by ranking lens and consults no rule at all, so nothing between
    /// the sweep and the browser ever asked whether the best row was any good.
    #[test]
    fn the_top_ranked_row_fails_the_operators_rules() {
        let v = measured_rank_one().verdict(&crate::Rules::operator());
        assert!(v.priced, "868 trades is priced");
        assert!(!v.win_rate, "3 wins in 868 is 0.34%, against a 50% floor");
        assert!(
            !v.reward_to_risk,
            "295 over 3,035 is 0.09x, against a 1.25x floor"
        );
        assert!(
            !v.return_over_drawdown,
            "a negative total cannot clear a 5x floor"
        );
        assert!(!v.admitted, "and so it is not admitted");
        assert!(
            v.stop_unchecked,
            "the stop rule is NOT claimed as passed -- the row has no worst_mae"
        );
    }

    /// A row that meets every checkable rule says so, and still reports the gap.
    ///
    /// Without this the verdict could be a function that returns `false`, and
    /// the table would read FAIL forever while looking exactly as correct.
    #[test]
    fn a_row_that_meets_every_rule_is_admitted_and_still_names_the_unchecked_stop() {
        let good = Row {
            trades: 100,
            cell_wins: 95,
            pessimistic: 500_000,
            worst_trade: -100,
            max_drawdown: 1_000,
            min_win: 400,
            gross_win: 505_000,
            gross_loss: -5_000,
            ..row(2, 1)
        };
        let v = good.verdict(&crate::Rules::operator());
        assert!(v.win_rate, "95% clears 50%");
        assert!(v.reward_to_risk, "400 over 100 is 4.00x, clears 1.25x");
        assert!(v.return_over_drawdown, "500,000 over 1,000 clears 5x");
        assert!(v.admitted, "every checkable rule is met");
        assert!(
            v.stop_unchecked,
            "and the stop is STILL unchecked -- passing the others does not \
             turn an unmeasured rule into a passed one"
        );
    }

    /// An unpriced row is not a perfect row, and the two must never render alike.
    ///
    /// `screen_cap` means most ranked combinations never meet an exit grid and
    /// store zeros. A zero drawdown is a spectacular result, so a reader that
    /// cannot tell "never priced" from "priced and lost nothing" reads the rows
    /// nobody measured as the best in the file.
    #[test]
    fn an_unpriced_row_fails_every_rule_and_is_marked_unpriced() {
        let v = row(3, 1).verdict(&crate::Rules::operator());
        assert!(!v.priced, "trades == 0 is the only value that says so");
        assert!(!v.admitted, "and it is not admitted on a zero drawdown");
        assert!(!v.win_rate && !v.reward_to_risk && !v.return_over_drawdown);
    }

    /// The stride is what the writer writes, not what a comment claims.
    ///
    /// Version 2 added six money fields — `trades`, `cell_wins`, `pessimistic`,
    /// `worst_trade`, `max_drawdown`, `min_win` — because the operator's ranking
    /// is a question about money and version 1 stored none of it. The stride
    /// moved from 144 to 192, and this assertion is what made that a decision
    /// rather than an accident: it failed the moment the fields were added and
    /// the number was not.
    ///
    /// IT DID IT AGAIN AT VERSION 3, which is the whole justification for keeping
    /// it. `gross_win` and `gross_loss` were appended and `STRIDE` was moved to
    /// 208 in the same edit; this line failed on the next `cargo test` because
    /// the SUM below had not been told. Two of the operator's eleven ranking
    /// criteria — average win and average loss — were structurally always zero
    /// without those two sums, since `Cell::avg_win` is `gross_win / wins` and
    /// `..Default::default()` had been supplying a zero for it.
    #[test]
    fn the_stride_is_exactly_what_the_writer_writes() {
        //           identity  rank  mask      hits/n/mean/t/payoff/wins
        let v1 = 32 + 2 + 6 * 8 + 8 + 8 + 8 + 8 + 8 + 8;
        //  trades, cell_wins, pessimistic, worst_trade, max_drawdown, min_win
        let v2_added = 6 * 8;
        //  gross_win, gross_loss -- without which avg_win and avg_loss are zero
        let v3_added = 2 * 8;
        //  THE EIGHT `Rules` FIELDS, so a reader judges these rows by the rules
        //  the run that wrote them applied and not by whatever the environment
        //  says at request time. `top` is written as a `u64` because a `usize`
        //  is not a width the disk can carry.
        let v4_added = 8 * 8;
        assert_eq!(
            v1 + v2_added + v3_added + 6 + v4_added + SEAL_BYTES,
            STRIDE_BYTES,
            "fields + six reserved + the rules + seal must be the stride"
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
        assert_eq!(Row::from_bytes(&raw).expect("valid schema"), want);
    }

    fn reseal(raw: &mut [u8; STRIDE_BYTES]) {
        let seal = seal_of(raw);
        raw.get_mut(PAYLOAD_BYTES..)
            .expect("the row owns a seal suffix")
            .copy_from_slice(&seal);
        assert!(Row::seal_matches(raw), "the mutated fixture is sealed");
    }

    fn append_raw(path: &std::path::Path, rows: &[[u8; STRIDE_BYTES]]) {
        use std::io::Write as _;

        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(path)
            .expect("the adversarial peer opens the frontier");
        for raw in rows {
            file.write_all(raw).expect("one whole row is appended");
        }
        file.sync_all().expect("the adversarial rows are durable");
    }

    /// A seal authenticates bytes, not the meaning of a future schema.
    #[test]
    fn sealed_unknown_direction_and_reserved_row_bytes_are_refused() {
        for (offset, value, named) in [
            (DIRECTION_AT, 2_u8, "direction"),
            (ROW_RESERVED.start, 1, "reserved"),
            (ROW_RESERVED.end.saturating_sub(1), u8::MAX, "reserved"),
        ] {
            let mut raw = row(7, 3).to_bytes();
            *raw.get_mut(offset).expect("a payload offset") = value;
            reseal(&mut raw);
            let why = Row::from_bytes(&raw).expect_err("unknown schema must refuse");
            assert!(why.contains(named), "offset {offset}: {why}");
            assert!(why.contains(&offset.to_string()), "offset {offset}: {why}");
        }
    }

    /// A sealed invalid row remains indexed so a rerun cannot append around it.
    #[test]
    fn a_sealed_invalid_row_blocks_read_and_duplicate_recovery() {
        let dir = root("sealed-invalid");
        let path = Frontier::path(&dir);
        {
            let mut store = Frontier::open(&dir).expect("a fresh file opens");
            store.append_all(&[row(7, 1)]).expect("one row");
        }
        let mut bytes = std::fs::read(&path).expect("the frontier is readable");
        let chunk = bytes
            .get_mut(HEADER_BYTES..HEADER_BYTES + STRIDE_BYTES)
            .expect("one complete row");
        let mut raw = <[u8; STRIDE_BYTES]>::try_from(&*chunk).expect("one row stride");
        *raw.get_mut(DIRECTION_AT).expect("the direction byte") = 2;
        reseal(&mut raw);
        chunk.copy_from_slice(&raw);
        std::fs::write(&path, bytes).expect("the fixture is rewritten");

        let mut reopened = Frontier::open(&dir).expect("the header and stride remain valid");
        assert!(
            reopened.holds(&[7; 32]),
            "a sealed invalid row's identity remains in the recovery index"
        );
        let why = reopened.read(0).expect_err("direct read must refuse");
        assert!(
            why.contains("schema is invalid") && why.contains("direction"),
            "{why}"
        );
        let (found, damaged) = reopened
            .of_run(&[7; 32])
            .expect("recovery returns evidence");
        assert!(found.is_empty(), "no invalid row is decoded");
        assert!(
            damaged
                .as_deref()
                .is_some_and(|why| why.contains("direction")),
            "the block carries its schema refusal: {damaged:?}"
        );
        let duplicate = reopened
            .append_all(&[row(7, 1)])
            .expect_err("recovery must not append around damaged history");
        assert!(
            duplicate.contains("already has a frontier block"),
            "{duplicate}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_sealed_invalid_row_on_fresh_reopen_keeps_the_healthy_prefix_but_poisons_writes() {
        let dir = root("reopened-sealed-invalid-tail");
        let path = Frontier::path(&dir);
        let kept = row(0x51, 1);
        {
            let mut store = Frontier::open(&dir).expect("a fresh file opens");
            store.append_all(&[kept]).expect("the healthy prefix");
        }

        let mut invalid = row(0x52, 1).to_bytes();
        *invalid.get_mut(DIRECTION_AT).expect("the direction byte") = 2;
        reseal(&mut invalid);
        append_raw(&path, &[invalid]);
        let before = std::fs::read(&path).expect("the adversarial file is readable");

        let mut reopened = Frontier::open(&dir).expect("a row defect does not hide its prefix");
        assert_eq!(
            reopened.block(&kept.identity),
            Some(super::Block { first: 0, count: 1 })
        );
        assert_eq!(
            reopened
                .read(0)
                .expect("the verified prefix remains readable"),
            kept
        );
        assert!(
            reopened
                .read(1)
                .expect_err("the invalid row is still diagnosed")
                .contains("schema is invalid")
        );
        for why in [
            reopened
                .append_all(&[row(0x53, 1)])
                .expect_err("a different identity cannot append past invalid schema"),
            reopened
                .confirm_durable()
                .expect_err("invalid schema cannot be promoted as durable"),
        ] {
            assert!(
                why.contains("row 1")
                    && why.contains("schema is invalid")
                    && why.contains("direction"),
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
    fn an_a_b_a_frontier_on_fresh_reopen_never_spans_b_and_poisons_writes() {
        let dir = root("reopened-a-b-a");
        let path = Frontier::path(&dir);
        let first_a = row(0x61, 1);
        let b = row(0x62, 1);
        {
            let mut store = Frontier::open(&dir).expect("a fresh file opens");
            store.append_all(&[first_a]).expect("first A block");
            store.append_all(&[b]).expect("B block");
        }
        append_raw(&path, &[row(0x61, 2).to_bytes()]);
        let before = std::fs::read(&path).expect("the adversarial file is readable");

        let mut reopened = Frontier::open(&dir).expect("the healthy prefix remains indexable");
        assert_eq!(
            reopened.block(&first_a.identity),
            Some(super::Block { first: 0, count: 1 }),
            "the first A block must never widen across B"
        );
        assert_eq!(
            reopened.block(&b.identity),
            Some(super::Block { first: 1, count: 1 })
        );
        assert_eq!(
            reopened.of_run(&first_a.identity).expect("first A reads").0,
            vec![first_a],
            "the foreign B row and repeated A row are outside A's first block"
        );
        for why in [
            reopened
                .append_all(&[row(0x63, 1)])
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

    /// Header reserve is schema, not padding a reader may silently ignore.
    #[test]
    fn a_nonzero_reserved_header_byte_refuses_the_file() {
        let dir = root("header-reserve");
        let path = Frontier::path(&dir);
        drop(Frontier::open(&dir).expect("a fresh file opens"));
        let mut bytes = std::fs::read(&path).expect("the header is readable");
        *bytes
            .get_mut(HEADER_RESERVED.start)
            .expect("the first reserved header byte") = 1;
        std::fs::write(&path, bytes).expect("the fixture is rewritten");

        let why = Frontier::open(&dir).expect_err("unknown header schema must refuse");
        assert!(why.contains("reserved header byte"), "{why}");
        assert!(why.contains("12..16"), "{why}");
        assert!(why.contains("nothing was written"), "{why}");
        let _ = std::fs::remove_dir_all(&dir);
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
        let mut reopened = Frontier::open(&dir).expect("an existing file opens");
        assert_eq!(
            reopened.block(&[2; 32]),
            Some(super::Block { first: 3, count: 2 }),
            "the one pass at open rebuilt the same block"
        );
        reopened
            .confirm_durable()
            .expect("healthy contiguous blocks carry no write poison after reopen");
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

    /// Duplicate rejection is refreshed under the file lock, not decided by
    /// the snapshot a handle happened to open with.
    #[test]
    fn two_stale_handles_cannot_append_the_same_frontier_identity() {
        let dir = root("stale-duplicate");
        let mut first = Frontier::open(&dir).expect("the first handle opens");
        let mut stale = Frontier::open(&dir).expect("the stale handle opens before the write");

        first
            .append_all(&[row(8, 1), row(8, 2)])
            .expect("the first block appends");
        let why = stale
            .append_all(&[row(8, 3)])
            .expect_err("the stale snapshot is refreshed under the lock");
        assert!(why.contains("already has a frontier block"), "{why}");

        drop(first);
        drop(stale);
        let mut reopened = Frontier::open(&dir).expect("the file remains whole");
        assert_eq!(reopened.len().expect("a count"), 2);
        assert_eq!(
            reopened.of_run(&[8; 32]).expect("one exact block").0,
            vec![row(8, 1), row(8, 2)]
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A stale handle must notice a whole corrupt row added after its index was
    /// built and stop before it writes a later block.
    #[test]
    fn a_stale_handle_refuses_to_extend_past_a_bad_sealed_frontier_row() {
        use std::io::Write as _;

        let dir = root("stale-bad-seal");
        let mut stale = Frontier::open(&dir).expect("the stale handle opens");
        let path = Frontier::path(&dir);
        let mut corrupt = row(9, 1).to_bytes();
        *corrupt.get_mut(40).expect("a sealed payload byte") ^= 0x80;
        let mut crashed = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .expect("the crashed peer opens");
        crashed
            .write_all(&corrupt)
            .expect("one whole corrupt row lands");
        crashed.sync_all().expect("the corrupt row is durable");
        let before = std::fs::metadata(&path)
            .expect("the file has metadata")
            .len();

        let why = stale
            .append_all(&[row(10, 1)])
            .expect_err("the stale writer must absorb and refuse the corrupt row");
        assert!(why.contains("does not match its seal"), "{why}");
        assert_eq!(
            std::fs::metadata(&path)
                .expect("the refusal leaves metadata")
                .len(),
            before,
            "nothing was appended after the corrupt row"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Reopening must not turn a whole corrupt orphan into padding. Healthy
    /// rows stay diagnosable, but no writer may extend or promote the file.
    #[test]
    fn a_reopened_writer_refuses_to_extend_past_a_bad_sealed_frontier_row() {
        use std::io::Write as _;

        let dir = root("reopened-bad-seal");
        let path = Frontier::path(&dir);
        let kept = row(1, 1);
        {
            let mut store = Frontier::open(&dir).expect("a fresh file opens");
            store
                .append_all(&[kept])
                .expect("the healthy prefix appends");
        }

        let mut corrupt = row(2, 1).to_bytes();
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

        let mut reopened = Frontier::open(&dir).expect("healthy blocks remain indexable");
        assert_eq!(
            reopened
                .read(0)
                .expect("the healthy prefix remains readable"),
            kept
        );
        assert!(
            reopened
                .read(1)
                .expect_err("the corrupt row is diagnosed")
                .contains("seal"),
            "a read names the damaged row"
        );
        for why in [
            reopened
                .append_all(&[row(3, 1)])
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
            Frontier::open_read(&dir).is_ok(),
            "a read-only handle still opens so unaffected committed runs can be diagnosed"
        );
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

    /// A failed buffered append cannot poison every block that preceded it.
    #[test]
    fn a_partial_frontier_write_rolls_back_to_the_last_whole_row() {
        use std::io::Write as _;

        let dir = root("partial-write-rollback");
        let mut store = Frontier::open(&dir).expect("a fresh file opens");
        store
            .append_all(&[row(1, 1), row(1, 2)])
            .expect("the committed prefix appends");
        let before = std::fs::metadata(Frontier::path(&dir))
            .expect("the prefix has metadata")
            .len();

        let why = store
            .append_locked_with(&[row(2, 1)], |file, bytes| {
                file.write_all(bytes.get(..3).unwrap_or_default())?;
                Err(std::io::Error::new(
                    std::io::ErrorKind::StorageFull,
                    "injected full filesystem after a three-byte prefix",
                ))
            })
            .expect_err("the injected partial write refuses");
        assert!(why.contains("rolled back"), "the recovery is named: {why}");
        assert_eq!(
            std::fs::metadata(Frontier::path(&dir))
                .expect("the rolled-back file has metadata")
                .len(),
            before,
            "only this call's three partial bytes were removed"
        );

        drop(store);
        let mut reopened = Frontier::open(&dir).expect("the whole prefix still opens");
        assert_eq!(reopened.len().expect("a whole-row count"), 2);
        assert_eq!(
            reopened.of_run(&[1; 32]).expect("the old block reads").0,
            vec![row(1, 1), row(1, 2)]
        );
        assert!(!reopened.holds(&[2; 32]), "the failed run gained no block");
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

    #[test]
    fn a_bounded_reader_refuses_before_indexing_an_oversized_file() {
        let dir = root("bounded-open");
        std::fs::create_dir_all(dir.join("results")).expect("results directory");
        let file = std::fs::File::create(Frontier::path(&dir)).expect("frontier fixture");
        file.set_len(17).expect("sparse oversized fixture");
        let why = Frontier::open_read_bounded(&dir, 16).expect_err("17 exceeds 16");
        assert!(why.contains("17 bytes"), "measured length: {why}");
        assert!(why.contains("16 bytes"), "hard ceiling: {why}");
        assert!(why.contains("No partial frontier index"), "{why}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}

/// The five figures a row implies, computed from the six it stores.
///
/// # Why these are derived and not stored
///
/// A win rate is `wins / trades`, a reward-to-risk is `min_win / -worst_trade`,
/// and both are already defined once — on [`runner::grid::Cell`], by the code
/// that priced the combination. Storing them too would put a second definition
/// on disk, and the two would agree until one of them was changed. That is the
/// shape `CLAUDE.md` §5 refuses and the shape that has caused four separate
/// defects in this repository.
///
/// So the row carries the raw six and this rebuilds a `Cell` to ask it. The
/// caller gets the engine's own answer, not a copy of the formula.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Derived {
    /// Trades that won, in basis points of trades taken.
    pub win_rate_bp: i64,
    /// The SMALLEST win over the LARGEST loss, in hundredths. `125` is the
    /// operator's stated 1.25x.
    pub reward_to_risk_bp: i64,
    /// Total made over the deepest fall, in basis points.
    pub return_over_drawdown: i64,
    /// Mean winning trade, in paisa.
    pub avg_win: i64,
    /// Mean losing trade, in paisa. Negative or zero.
    pub avg_loss: i64,
    /// Trades that did not win.
    pub losses: u64,
    /// Whether this combination was ever priced.
    ///
    /// `screen_cap` means most ranked combinations never meet an exit grid, and
    /// those store zeros. A zero drawdown is a SPECTACULAR result, so a reader
    /// that cannot tell "never priced" from "priced and lost nothing" will read
    /// the best rows in the file as the ones nobody measured. `trades == 0` is
    /// the only value that separates them.
    pub priced: bool,
}

impl Row {
    /// What this row implies, asked of the engine's own `Cell`.
    #[must_use]
    pub fn derived(&self) -> Derived {
        let cell = runner::grid::Cell {
            trades: self.trades,
            wins: self.cell_wins,
            pessimistic: self.pessimistic,
            worst_trade: self.worst_trade,
            max_drawdown: self.max_drawdown,
            min_win: self.min_win,
            gross_win: self.gross_win,
            gross_loss: self.gross_loss,
            ..Default::default()
        };
        Derived {
            win_rate_bp: cell.win_rate_bp(),
            reward_to_risk_bp: cell.reward_to_risk_bp(),
            return_over_drawdown: cell.return_over_drawdown(),
            avg_win: cell.avg_win(),
            avg_loss: cell.avg_loss(),
            losses: self.trades.saturating_sub(self.cell_wins),
            priced: self.trades > 0,
        }
    }
}

/// Which of the operator's rules a ranked row meets, and the one it cannot answer.
///
/// # Why this exists
///
/// [`crate::record_frontier`] writes `by_evidence.iter().take(top)` — the top
/// `top` combinations by the ranking lens — and reads NONE of
/// [`crate::Rules`]. Not `min_win_rate_bp`, not `min_rr_bp`, not `min_trades`,
/// not `min_ret_over_dd_bp`. The only knob that reaches the ledger is `top`, as
/// a count.
///
/// That is deliberate and it is correct: the frontier is the RANKING, and a
/// ranking that silently dropped everything failing a rule would answer *"what
/// did the sweep find"* with *"nothing"* and give the operator no way to see how
/// near the misses were. The defect was never that the rows are unfiltered — it
/// is that **nothing on the row said whether it passed**, so a list ranked on
/// unstopped forward payoff was read as a list of candidates.
///
/// So the verdict travels beside the row rather than deciding whether the row
/// exists. `PASS` and `FAIL` are shown, the operator sorts on them, and a run
/// where nothing passes says so out loud instead of rendering an empty table
/// that looks like a missing feature.
///
/// # The rule that is ABSENT rather than passing, and why that distinction is the point
///
/// `Rules::max_mae_ppm` is checked against [`runner::grid::Cell::worst_mae`] —
/// the maximum adverse excursion across every trade. **A row does not store
/// it.** [`Row::derived`] rebuilds its `Cell` with `..Default::default()`, which
/// sets `worst_mae` to `0`, and `Rules::admits` opens with
/// `self.max_mae_ppm == 0 || cell.worst_mae <= self.max_mae_ppm` — so calling
/// `admits` here would report the stop rule as PASSED on every row in the file,
/// on the strength of a zero nobody measured.
///
/// That is the failure `CLAUDE.md` §4 bans by name: a fallback that hides a
/// failure. The field is therefore not a `bool`. It is absent, and
/// [`Self::stop_unchecked`] says so, because "we did not measure this" and "this
/// passed" are the two things that must never render the same.
/// # Why six booleans rather than the enum clippy asks for
///
/// `struct_excessive_bools` fires at three, and its remedy — split the type into
/// variants — is right when the flags are a STATE that only some combinations of
/// which are legal. These are not a state. They are five independent rules and
/// their conjunction, every one of the thirty-two combinations is reachable, and
/// an operator reading `FAIL` needs to know WHICH rules failed. Collapsing them
/// into variants would either enumerate thirty-two names or throw away the
/// detail that makes the verdict worth showing.
#[expect(
    clippy::struct_excessive_bools,
    reason = "five independent rule outcomes and their conjunction; every \
              combination is reachable and the page renders which ones failed"
)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Verdict {
    /// `win_rate_bp >= Rules::min_win_rate_bp`.
    pub win_rate: bool,
    /// `reward_to_risk_bp >= Rules::min_rr_bp` — smallest win over largest loss.
    pub reward_to_risk: bool,
    /// `return_over_drawdown >= Rules::min_ret_over_dd_bp`.
    pub return_over_drawdown: bool,
    /// `trades >= Rules::min_trades` — is the rate evidence of anything.
    pub trades: bool,
    /// `assurance_bp >= Rules::min_assurance_bp` — the 95% lower bound on the
    /// rate, not the observed rate.
    pub assurance: bool,
    /// Every rule above holds. **Not "every rule holds"** — see
    /// [`Self::stop_unchecked`] and [`Self::protective_exits_unchecked`].
    pub admitted: bool,
    /// Always `true`. The stop rule was not evaluated because the row does not
    /// carry `worst_mae`, and a reader must be told that rather than shown a
    /// pass.
    pub stop_unchecked: bool,
    /// Always `true`. The PROTECTIVE-EXIT rule was not evaluated either, and
    /// for the same reason: [`Row`] does not carry the exit shape.
    ///
    /// # This flag exists because its absence was a lie on the page
    ///
    /// `Rules::require_protective_exits` refuses any cell without a stop and a
    /// trailing order. [`Row::of`] copies a cell's MONEY and drops `stop`,
    /// `target`, `tsl` and `ttp` entirely, so this type cannot answer the rule
    /// — and it did not say so. `/frontier.json` emitted `meets.all` and the
    /// browser rendered a green **PASS** pill for a row whose cell may have had
    /// no stop at all.
    ///
    /// The asymmetry was the defect, not the absence. `stop_unchecked` already
    /// says *"we did not measure this"* for `max_mae_ppm`, precisely so a
    /// missing measurement and a passed rule never render alike. This rule was
    /// missing in the second way — silently — which is the failure wearing a
    /// success's clothes `CLAUDE.md` §4 bans.
    ///
    /// Making it a FIELD rather than fixing the format is deliberate. Carrying
    /// the exit shape would be a version 6 and a wider row, and the rule
    /// governs SELECTION, which happened before the row was written. What a
    /// reader needs is not the shape but the knowledge that this verdict does
    /// not speak for it.
    pub protective_exits_unchecked: bool,
    /// Whether this combination was ever priced. An unpriced row fails every
    /// rule on zeros, which is honest but is not the same claim as "priced and
    /// it failed" — the reader needs both.
    pub priced: bool,
}

impl Row {
    /// This row judged against one rule set, with the unanswerable rule named.
    ///
    /// An UNPRICED row (`trades == 0`) returns every rule `false` and
    /// `priced: false`. It is not admitted, and the reason is that it never met
    /// an exit grid — `screen_cap` cut it — rather than that it was measured and
    /// found wanting.
    #[must_use]
    pub fn verdict(&self, rules: &crate::Rules) -> Verdict {
        let d = self.derived();
        if !d.priced {
            return Verdict {
                stop_unchecked: true,
                ..Verdict::default()
            };
        }
        let cell = runner::grid::Cell {
            trades: self.trades,
            wins: self.cell_wins,
            ..Default::default()
        };
        let win_rate = d.win_rate_bp >= rules.min_win_rate_bp;
        let reward_to_risk = d.reward_to_risk_bp >= rules.min_rr_bp;
        let return_over_drawdown = d.return_over_drawdown >= rules.min_ret_over_dd_bp;
        let trades = self.trades >= rules.min_trades;
        let assurance = cell.assurance_bp() >= rules.min_assurance_bp;
        Verdict {
            win_rate,
            reward_to_risk,
            return_over_drawdown,
            trades,
            assurance,
            // FIVE RULES, AND THE NAME SAYS FIVE. Two more exist and neither
            // can be answered from a `Row`: the stop ceiling needs `worst_mae`
            // and the protective-exit rule needs the exit shape, and this type
            // carries neither. Both are reported unchecked rather than folded
            // in as passes.
            admitted: win_rate && reward_to_risk && return_over_drawdown && trades && assurance,
            stop_unchecked: true,
            protective_exits_unchecked: true,
            priced: true,
        }
    }
}

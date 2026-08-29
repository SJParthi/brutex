//! The best rows a run has found SO FAR, written while it is still running.
//!
//! # The hole this fills, measured
//!
//! A sweep writes its ledger row, its frontier rows and its trades in
//! [`crate::record_all`], which runs once, at the very end, after everything.
//! MEASURED on 2026-08-29: a 60-minute `audit-range` over 81 months ran for
//! **48 minutes and wrote zero bytes**. Its results directory still held the
//! previous run's three files, untouched. Killed at minute 47 it would have left
//! nothing at all, and there was no surface anywhere that could say what it had
//! found so far — a browser polling `/frontier.json` sees the PREVIOUS run until
//! this one ends.
//!
//! The ranking that would have answered already existed and was simply never
//! written down: `runner::rank` keeps a bounded min-heap of the best `keep`
//! combinations by `|t|`, admitting in `O(log keep)` and rejecting with a single
//! comparison. This module is the other half — it puts that heap on disk
//! whenever it changes.
//!
//! **UNVERIFIED as a measurement.** Those per-candidate costs are `runner::rank`'s
//! and are argued from the shape of a bounded heap; that module's own doc marks
//! them unverified too, and no bench row in this workspace times either. What IS
//! arithmetic rather than assertion is the rewrite count below, and
//! [`expected_rewrites`] is that arithmetic as a function so it can be checked
//! rather than trusted.
//!
//! # Why it is cheap, and this is the number that decides it
//!
//! Not "whenever a candidate arrives" — **whenever the top-N actually moves.**
//! Over `N` candidates in arrival order the expected number of times the
//! `keep`-th place improves is about `keep * ln(N / keep)`, because a new
//! arrival only displaces the worst held row if it beats it and that gets
//! steadily harder. At `keep = 25`:
//!
//! | candidates weighed | expected rewrites |
//! |---|---|
//! | 10,000 | 174 |
//! | 1,000,000 | 289 |
//! | 84,000,000 | **400** |
//!
//! So an eighty-four-million-candidate run rewrites this file a few hundred
//! times, and each rewrite is `keep` rows of 208 bytes — about 2 MB of writes
//! for the whole run. The cost is logarithmic in the search and the heap settles
//! early, which is why this is affordable at a granularity that would be absurd
//! per candidate.
//!
//! Those figures include the `keep` rewrites that FILL the heap. The first table
//! here omitted them and read 149 / 264 / 375; the trigger is "the top-N moved",
//! and it moves on each of the first `keep` arrivals too.
//!
//! # It is written by RENAME, and the first version's ordering argument was wrong
//!
//! [`Live::publish`] writes a temp file and renames it over the real one.
//! It used to rewrite in place with the row count written last, arguing that a
//! reader arriving mid-write saw a whole older answer. **An adversarial pass
//! produced the interleaving that breaks it**: the reader walks the file in the
//! same direction the writer writes it, so a reader that had read the OLD
//! summary and was descheduled came back and read the NEW rows — rendering a
//! `|t|` against a bar it was no longer judged by, which is the fabricated
//! finding the summary block exists to prevent. `rename` is atomic, so a reader
//! gets one whole answer or the other and there is no third outcome.
//!
//! # It is TRANSIENT, and that is the whole reason it is a separate file
//!
//! `CLAUDE.md` §3 rule 8 makes history append-only, and `crate::record_all`
//! writes LEDGER, then FRONTIER, then TRADES so that *"a detail row whose run
//! has no ledger row is an orphan"*. The ledger row does not exist until the run
//! ends. Writing live rows into `frontier.bin` would therefore manufacture
//! exactly that orphan — every partial run leaving detail rows whose parent
//! never arrives — and it would do it by the one route the ordering cannot see.
//!
//! So this is not history. It is **replaced whole by rename**, it is not appended
//! to, and a run that completes leaves its permanent record through the ordinary
//! path unchanged. A file here is a statement about a run that is happening, not
//! a record of one that happened, and [`Live::finish`] removes it when the real
//! rows land.
//!
//! # One file per run identity, and therefore no lock
//!
//! `range_over` walks eight rungs at once. A single shared file would need the
//! results lock taken a few hundred times per rung, and the rungs would contend
//! for it while doing nothing useful. The run identity already encodes the
//! timeframe — it is `blake3` over nine terms of which one is the rung — so one
//! file per identity gives **top-N per timeframe for free**, with eight writers
//! touching eight paths and no shared state at all.
//!
//! # The row is `frontier::Row`, deliberately
//!
//! Not a new struct. A second layout for the same fact is two things to keep
//! true, and the drift shows up as a live view that disagrees with the finished
//! one — which is worse than no live view, because a reader cannot tell which is
//! lying. Reusing the row means the page renders both with one decoder, and a
//! field added to the frontier appears here without a second edit.

use std::fs::File;
use std::io::{Read as _, Seek as _, SeekFrom, Write as _};
use std::path::{Path, PathBuf};

use crate::frontier::{Row, STRIDE_BYTES};

/// What a refusal says. A `String` for the same reason the other stores use one:
/// the message names the path and the cause, and neither is known at compile
/// time.
pub type Refusal = String;

/// Eight bytes that say which format this is. One letter apart from the others
/// so a hex dump names the file: `BRUTEXRS` is the ledger, `BRUTEXFR` the
/// frontier, `BRUTEXTD` the trades, `BRUTEXLV` this.
const MAGIC: [u8; 8] = *b"BRUTEXLV";

/// The format this build writes and reads.
///
/// Unlike the three permanent stores there is no read-an-older-version path and
/// there never will be: a live file describes a run that is in flight, so a file
/// written by an older build belongs to a process that is no longer running. A
/// version this build does not know is DELETED rather than decoded — see
/// [`Live::open`] — which is safe here and would be a §3 rule 8 violation
/// anywhere else in this directory.
const VERSION: u32 = 1;

/// Magic, version, and four bytes that stay zero. The same sixteen the other
/// three stores use, so one reader can identify any file in this directory.
const HEADER_BYTES: usize = 16;

/// How many rows the header says are live. Sits in the reserved four bytes the
/// other stores leave zero, because a live file is rewritten whole and a reader
/// arriving mid-write must not read a stale tail as data.
const COUNT_AT: usize = 12;

/// The trial count and the bar are written in a SECOND header block rather than
/// on every row, because they are facts about the RUN and not about a row.
/// Repeating them 25 times would be 25 places for one number to disagree with
/// itself.
const SUMMARY_BYTES: usize = 24;

/// Where row zero starts.
const ROWS_AT: usize = HEADER_BYTES + SUMMARY_BYTES;

/// What a run has found so far, and what it must clear.
///
/// # Why the bar travels with the rows
///
/// A `|t|` of 3.42 is not a verdict on its own — it is a verdict against the
/// number of hypotheses the run has weighed, and that number GROWS while the run
/// is in flight. A live view showing rows without the bar they must clear invites
/// exactly the reading `CLAUDE.md` §4 bans: a figure that looks like a finding
/// because nothing beside it says otherwise.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Summary {
    /// Hypotheses weighed so far. The denominator the bar is computed from.
    pub trials: u64,
    /// The `|t|` a row must reach to be distinguishable from luck at this trial
    /// count, in thousandths — the same scale [`Row::t_milli`] uses, so a reader
    /// compares two integers rather than two conventions.
    pub bar_milli: i64,
    /// Candidates the run has priced. Not the same as `trials`: a candidate can
    /// be weighed by the ladder and never reach the exit grid.
    pub priced: u64,
}

impl Summary {
    fn to_bytes(self) -> [u8; SUMMARY_BYTES] {
        let mut out = [0_u8; SUMMARY_BYTES];
        out[0..8].copy_from_slice(&self.trials.to_le_bytes()); //  0  8
        out[8..16].copy_from_slice(&self.bar_milli.to_le_bytes()); //  8  8
        out[16..24].copy_from_slice(&self.priced.to_le_bytes()); // 16  8
        out
    }

    fn from_bytes(raw: &[u8; SUMMARY_BYTES]) -> Self {
        let word = |at: usize| -> [u8; 8] {
            let mut b = [0_u8; 8];
            b.copy_from_slice(raw.get(at..at + 8).unwrap_or(&[0; 8]));
            b
        };
        Self {
            trials: u64::from_le_bytes(word(0)),
            bar_milli: i64::from_le_bytes(word(8)),
            priced: u64::from_le_bytes(word(16)),
        }
    }
}

/// One run's live top-N, on disk.
pub struct Live {
    path: PathBuf,
}

impl Live {
    /// The directory live files live in, beside the permanent three.
    #[must_use]
    pub fn dir(root: &Path) -> PathBuf {
        root.join("results").join("live")
    }

    /// This run's file. Named by the identity so eight concurrent rungs cannot
    /// collide, and so a stale file names the run that abandoned it.
    #[must_use]
    pub fn path(root: &Path, identity: &[u8; 32]) -> PathBuf {
        let mut hex = String::with_capacity(64);
        for byte in identity {
            use core::fmt::Write as _;
            let _ = write!(hex, "{byte:02x}");
        }
        Self::dir(root).join(format!("{hex}.bin"))
    }

    /// Opens this run's live file, TRUNCATING whatever was there.
    ///
    /// # Why truncate rather than resume
    ///
    /// A file already at this path was written by a process that is no longer
    /// running — the identity is over nine terms including the commit, so a
    /// second process with the same identity is the same run being repeated. Its
    /// rows describe a search that was abandoned partway, and resuming from them
    /// would report a top-N assembled from two different walks. Starting empty is
    /// the honest reading, and it is why a version this build does not know can
    /// simply be overwritten.
    ///
    /// # Errors
    ///
    /// An unwritable results directory.
    pub fn open(root: &Path, identity: &[u8; 32]) -> Result<Self, Refusal> {
        let dir = Self::dir(root);
        std::fs::create_dir_all(&dir)
            .map_err(|why| format!("the live directory could not be made: {why}"))?;
        let mut live = Self {
            path: Self::path(root, identity),
        };
        // AN EMPTY PUBLISH RATHER THAN A HAND-WRITTEN HEADER, so the file is
        // born through the same atomic path every later update takes. The first
        // version wrote the header and the summary as two separate `write_all`s
        // with no barrier between them, so a reader landing in the gap saw a
        // valid 16-byte file with no summary and dropped the run from the
        // listing for that instant.
        live.publish(&[], Summary::default())?;
        Ok(live)
    }

    /// Replaces the whole file with `rows` and `summary`, ATOMICALLY.
    ///
    /// # Why the whole file and not an append
    ///
    /// The top-N is a SET that changes, not a sequence that grows: when a new
    /// row displaces the worst held one, the row that left is not part of the
    /// answer any more. Appending would make the file a history of every row
    /// that was ever briefly in the top 25, and a reader would have to replay it
    /// to find the current answer. `keep` rows is 5.2 KB at 25, so rewriting is
    /// cheaper than the bookkeeping that would avoid it.
    ///
    /// # Why a temp file and a rename, and what the first version got wrong
    ///
    /// This wrote in place and put the row COUNT down last, arguing that a
    /// reader arriving mid-write would see the previous count and therefore the
    /// previous rows — "stale, never torn". **That was false, and an adversarial
    /// pass produced the interleaving.** The reader walks the file in the SAME
    /// direction the writer writes it — summary first, then rows — so a reader
    /// that had already read the OLD summary and was then descheduled came back
    /// and read the NEW rows. It rendered a `|t|` of 5.000 against the bar of
    /// 4.560 it was no longer being judged by, and the row read as CLEARING a
    /// bar it does not clear. That is exactly the fabricated finding the summary
    /// block exists to prevent.
    ///
    /// Two further holes in the same scheme: a partial `write_all` on a full
    /// filesystem left a well-formed 25-row list spliced from two different
    /// walks, with ranks 1..25 intact and nothing downstream able to detect it;
    /// and the count-last ordering did no work anyway, because `read_one` clamps
    /// to the bytes actually present and that clamp decided every case.
    ///
    /// `rename` is atomic within a filesystem. A reader either opens the old
    /// inode and reads a complete old answer, or opens the new one and reads a
    /// complete new answer. There is no third outcome, no lock, and no ordering
    /// for a future change to get subtly wrong. The temp file carries the
    /// process id so two writers cannot collide on it.
    ///
    /// # Errors
    ///
    /// A write or rename that fails, which leaves the PREVIOUS file exactly as
    /// it was — the temp file is discarded and nothing partial is ever visible
    /// under the real name.
    pub fn publish(&mut self, rows: &[Row], summary: Summary) -> Result<(), Refusal> {
        let mut body = Vec::with_capacity(ROWS_AT + rows.len() * STRIDE_BYTES);
        let mut header = [0_u8; HEADER_BYTES];
        header[0..8].copy_from_slice(&MAGIC);
        header[8..12].copy_from_slice(&VERSION.to_le_bytes());
        // The count is still written, and is still checked against the bytes
        // present on read. It is no longer load-bearing for consistency -- the
        // rename is -- but it lets a reader refuse a file whose length disagrees
        // with its own header rather than trusting the length alone.
        let count = u32::try_from(rows.len()).unwrap_or(u32::MAX);
        header[COUNT_AT..COUNT_AT + 4].copy_from_slice(&count.to_le_bytes());
        body.extend_from_slice(&header);
        body.extend_from_slice(&summary.to_bytes());
        for row in rows {
            body.extend_from_slice(&row.to_bytes());
        }

        let temp = self
            .path
            .with_extension(format!("{}.tmp", std::process::id()));
        let write = || -> std::io::Result<()> {
            let mut file = File::create(&temp)?;
            file.write_all(&body)?;
            // `sync_data` and not `sync_all`: this file is not history, and a
            // torn live view costs a poll rather than a run. The ledger's own
            // barrier is `sync_all` for the opposite reason. Synced BEFORE the
            // rename so the name never points at bytes that are not down.
            file.sync_data()
        };
        if let Err(why) = write() {
            let _ = std::fs::remove_file(&temp);
            return Err(format!("{} could not be written: {why}", temp.display()));
        }
        std::fs::rename(&temp, &self.path).map_err(|why| {
            let _ = std::fs::remove_file(&temp);
            format!(
                "{} could not replace {}: {why}. The previous answer is still \
                 there and is still whole.",
                temp.display(),
                self.path.display()
            )
        })
    }

    /// Removes this run's live file, because the permanent rows now exist.
    ///
    /// Called after [`crate::record_all`] has written the ledger, the frontier
    /// and the trades. A live file that outlives its run is a claim that a search
    /// is still going when it is not, and `/live.json` cannot tell the difference
    /// from the file alone.
    ///
    /// # Errors
    ///
    /// Never. A file that cannot be removed is named in the returned text and
    /// the run stands — the same rule `record_all` follows, because detail about
    /// a completed run must not turn that run into a failure.
    #[must_use]
    pub fn finish(self) -> String {
        match std::fs::remove_file(&self.path) {
            Ok(()) => String::new(),
            Err(why) if why.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(why) => format!(
                "the live file {} could not be removed: {why}. It will report a \
                 finished run as still running until it is deleted by hand.\n",
                self.path.display()
            ),
        }
    }
}

/// Every run with a live file right now, in identity order.
///
/// NOT "newest activity first", which this said while sorting by identity. The
/// format carries no timestamp, no pid and no heartbeat, so there is nothing an
/// activity order could be computed from -- and a caller trusting that wording
/// would render a three-week-old abandoned file at the top because its identity
/// happens to start with a zero byte.
///
/// # Errors
///
/// Never. A directory that cannot be listed reads as no live runs, which is
/// the truth a reader can act on: there is nothing to show.
#[must_use]
pub fn current(root: &Path) -> Vec<([u8; 32], Summary, Vec<Row>)> {
    let Ok(entries) = std::fs::read_dir(Live::dir(root)) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for entry in entries.flatten() {
        if let Some(one) = read_one(&entry.path()) {
            out.push(one);
        }
    }
    // Deterministic order, because two runs finishing in the same millisecond
    // must not swap places between two polls of the same page. The identity is
    // the key nothing else shares.
    out.sort_by_key(|(identity, _, _)| *identity);
    out
}

/// One live file, or `None` if it is not one.
///
/// A file with the wrong magic, an unknown version, a short header or a count
/// larger than the bytes present is SKIPPED rather than refused: this directory
/// is transient by design and a half-written file is an ordinary state, not a
/// corruption to report.
fn read_one(path: &Path) -> Option<([u8; 32], Summary, Vec<Row>)> {
    let mut file = File::open(path).ok()?;
    let mut header = [0_u8; HEADER_BYTES];
    file.read_exact(&mut header).ok()?;
    if header.get(0..8)? != MAGIC {
        return None;
    }
    let mut four = [0_u8; 4];
    four.copy_from_slice(header.get(8..12)?);
    if u32::from_le_bytes(four) != VERSION {
        return None;
    }
    four.copy_from_slice(header.get(COUNT_AT..COUNT_AT + 4)?);
    let count = usize::try_from(u32::from_le_bytes(four)).ok()?;

    let mut summary_raw = [0_u8; SUMMARY_BYTES];
    file.read_exact(&mut summary_raw).ok()?;
    let summary = Summary::from_bytes(&summary_raw);

    // THE COUNT IS CHECKED AGAINST THE BYTES ACTUALLY PRESENT. It is written
    // last and is therefore never ahead of the rows -- but a file truncated by a
    // crashed process can be shorter than its own header claims, and reading
    // that as rows would decode whatever follows.
    let len = file.metadata().ok()?.len();
    let available = usize::try_from(len.saturating_sub(ROWS_AT as u64)).ok()? / STRIDE_BYTES;
    let readable = count.min(available);

    let mut rows = Vec::with_capacity(readable);
    let mut identity = [0_u8; 32];
    for nth in 0..readable {
        let at = ROWS_AT + nth * STRIDE_BYTES;
        let mut raw = [0_u8; STRIDE_BYTES];
        if file
            .seek(SeekFrom::Start(at as u64))
            .and_then(|_| file.read_exact(&mut raw))
            .is_err()
        {
            break;
        }
        // THE SEAL IS CHECKED, AND THIS MODULE WAS THE ONE READER THAT SKIPPED
        // IT.
        //
        // `Row::from_bytes`'s own doc says "The caller checks `seal_matches`
        // first", and `frontier.rs` and `trades.rs` both do at every read site.
        // This module declined -- while reading a file that is rewritten IN
        // PLACE by a live process, which is the single place in this store where
        // a torn row is most likely rather than least. A row torn mid-stride
        // decodes into an arbitrary `t_milli` and would be published to a
        // browser as a finding.
        //
        // A failed seal ends the read rather than skipping one row: rows are
        // ranked, so a gap in the middle would renumber everything after it and
        // the page would show rank 3 where rank 4 is. Stopping gives a shorter
        // but honest prefix of the ranking.
        if !Row::seal_matches(&raw) {
            break;
        }
        let row = Row::from_bytes(&raw);
        if nth == 0 {
            identity = row.identity;
        }
        rows.push(row);
    }
    if rows.is_empty() {
        // A started-but-empty file is a real state -- a run that has not yet
        // found anything -- and the identity comes from the FILENAME, which is
        // where it is written before any row exists.
        identity = identity_from_name(path)?;
    }
    Some((identity, summary, rows))
}

/// The identity a live file is named for.
fn identity_from_name(path: &Path) -> Option<[u8; 32]> {
    let stem = path.file_stem()?.to_str()?;
    if stem.len() != 64 {
        return None;
    }
    // EVERY CHARACTER MUST BE A HEX DIGIT, and `from_str_radix` is not that
    // check.
    //
    // `u8::from_str_radix` accepts a leading `+` on an unsigned type -- the
    // parser's `[b'+', rest @ ..]` arm carries no signedness guard -- so
    // `from_str_radix("+1", 16)` is `Ok(1)`. A file named `+1` sixty-four times
    // over therefore parsed to `[1u8; 32]`, which is a real identity another run
    // can legitimately own. Two different files would then report the SAME run,
    // and `sort_by_key` being stable means their order came from `read_dir` and
    // could change between two polls of one page -- defeating the determinism
    // the sort exists for.
    //
    // Found by an adversarial pass. Rejecting the name outright is right rather
    // than normalising it: this directory's filenames are written by
    // `Live::path` and nothing else, so a name that is not lowercase hex was not
    // written by this program.
    if !stem.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let mut out = [0_u8; 32];
    for (slot, pair) in out.iter_mut().zip(stem.as_bytes().chunks(2)) {
        let text = core::str::from_utf8(pair).ok()?;
        *slot = u8::from_str_radix(text, 16).ok()?;
    }
    Some(out)
}

/// How many rewrites a bounded top-`keep` is expected to need over `weighed`
/// candidates.
///
/// `keep * ln(weighed / keep)`, the standard record-count bound: the `m`-th
/// arrival improves the worst held row with probability `keep / m` once the heap
/// is full, and summing that is a harmonic series.
///
/// Exposed so the affordability claim in this module's header is a function
/// anyone can call rather than a table anyone must trust, and so a test can
/// check the figures rather than repeating them.
#[must_use]
pub fn expected_rewrites(keep: u64, weighed: u64) -> u64 {
    // KEEPING NOTHING REWRITES NOTHING. This returned `weighed`, and a test
    // asserted that under the words "degenerate inputs answer rather than divide
    // by zero" -- conflating "do not divide by zero" with "return the input". A
    // top-0 is always empty and can never move, so the answer is zero.
    if keep == 0 {
        return 0;
    }
    // FILLING THE HEAP IS `keep` REWRITES, and they are counted here rather than
    // waved at. The formula below counts DISPLACEMENTS once the heap is full;
    // the trigger this function models is "the top-N moved", and it also moves
    // on each of the first `keep` arrivals.
    //
    // Omitting them made the function non-monotonic at the seam: the old guard
    // returned `weighed` for `weighed <= keep`, while the formula at
    // `weighed == keep` gives `keep * ln(1) == 0`. So `f(25, 25)` was 25 and
    // `f(25, 26)` was 0 -- weighing one MORE candidate reported fewer rewrites --
    // and it stayed below 25 for every input up to about `25e`. The old test
    // stepped from `(25, 10)` straight to `(25, 10_000)` and over the whole
    // broken interval.
    if weighed <= keep {
        return weighed;
    }
    // The ratio is computed in f64 while the guard above compares in u64, so a
    // `weighed` too large to be represented exactly could round down to `keep`
    // and give `ln(1) == 0`. `max(1)` on the tail keeps the result at least the
    // fill cost, which is the floor the guard above establishes.
    #[expect(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::float_arithmetic,
        reason = "a logarithm, and this workspace holds MONEY in paisa integers -- \
                  which is what that ban is for. This is an expected COUNT, used to argue that a few hundred writes is \
                  affordable against a few hundred million candidates. A unit in \
                  the last place of a logarithm does not move that argument, and \
                  the inputs are counts that cannot be negative."
    )]
    let displacements = (keep as f64 * (weighed as f64 / keep as f64).ln()) as u64;
    keep.saturating_add(displacements)
}

#[cfg(test)]
#[expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "the exception every test module in this workspace takes. Indexing \
              is added because each index here follows an assertion on the \
              length, so the panic is unreachable and `.get(0).expect()` would \
              state the same fact twice."
)]
mod tests {
    use super::{Live, MAGIC, ROWS_AT, Summary, VERSION, current, expected_rewrites};
    use crate::frontier::Row;
    use crate::frontier::STRIDE_BYTES;

    /// A row with every field named, because `Row` has no `Default` -- it is
    /// built from a `Scored` and a grid `Cell` in production, and a test that
    /// wanted a partial one would be asking for a row that cannot exist.
    fn row(identity: [u8; 32], rank: u16, t_milli: i64) -> Row {
        Row {
            identity,
            rank,
            mask_words: [1, 0, 0, 0, 0, 0],
            hits: 250,
            n: 177,
            mean_milli_paisa: 965_000,
            t_milli,
            payoff_bp: 1_200,
            wins: 96,
            trades: 177,
            cell_wins: 96,
            pessimistic: 170_000,
            worst_trade: -17_845,
            max_drawdown: -42_000,
            min_win: 5_000,
            gross_win: 320_000,
            gross_loss: -150_000,
        }
    }

    /// A run's best rows are readable WHILE it is still running.
    ///
    /// The whole point: `record_all` writes nothing until a run ends, and a
    /// 60-minute `audit-range` measured on 2026-08-29 ran 48 minutes and wrote
    /// zero bytes. A browser polling the permanent files during that window sees
    /// the PREVIOUS run.
    #[test]
    fn a_running_sweep_publishes_its_top_rows_before_it_finishes() {
        let root = tempdir();
        let identity = [7_u8; 32];
        let mut live = Live::open(root.path(), &identity).expect("a writable root");

        // Nothing found yet: the run exists and has no answer, and those are
        // different from the run not existing.
        let started = current(root.path());
        assert_eq!(started.len(), 1, "the run is visible before it finds a row");
        assert!(started[0].2.is_empty(), "and it holds no rows yet");
        assert_eq!(
            started[0].0, identity,
            "an empty file still names its run, from the filename"
        );

        let first = vec![row(identity, 1, 3_420), row(identity, 2, 3_100)];
        live.publish(
            &first,
            Summary {
                trials: 1_000,
                bar_milli: 4_560,
                priced: 40,
            },
        )
        .expect("the file is writable");

        let seen = current(root.path());
        assert_eq!(seen.len(), 1);
        assert_eq!(seen[0].2.len(), 2, "both rows are readable mid-run");
        assert_eq!(seen[0].2[0].t_milli, 3_420);
        assert_eq!(
            seen[0].1.bar_milli, 4_560,
            "and the bar travels with them -- a |t| without the bar it must \
             clear is the reading CLAUDE.md §4 bans"
        );

        // A LATER PUBLISH REPLACES, IT DOES NOT APPEND. The top-N is a set that
        // changes; a file that accumulated every row ever briefly held would
        // make a reader replay it to find the current answer.
        let second = vec![row(identity, 1, 5_000)];
        live.publish(
            &second,
            Summary {
                trials: 90_000,
                bar_milli: 5_900,
                priced: 9_000,
            },
        )
        .expect("the file is writable");
        let after = current(root.path());
        assert_eq!(after[0].2.len(), 1, "one row replaced two, not three total");
        assert_eq!(after[0].2[0].t_milli, 5_000);
        assert_eq!(
            after[0].1.trials, 90_000,
            "and the trial count moved with them, because the bar is a verdict \
             against a denominator that grows while the run is in flight"
        );

        // AND IT IS REMOVED WHEN THE PERMANENT ROWS LAND. A live file that
        // outlives its run claims a search is still going when it is not.
        assert!(live.finish().is_empty(), "removal reports no problem");
        assert!(
            current(root.path()).is_empty(),
            "a finished run leaves no live file behind"
        );
    }

    /// Two runs at once do not collide, and each is its own timeframe.
    ///
    /// `range_over` walks eight rungs concurrently. The identity is over nine
    /// terms of which one is the rung, so one file per identity gives top-N per
    /// timeframe with no shared state and no lock.
    #[test]
    fn concurrent_rungs_each_keep_their_own_top_rows() {
        let root = tempdir();
        let (a, b) = ([1_u8; 32], [2_u8; 32]);
        let mut one = Live::open(root.path(), &a).expect("writable");
        let mut two = Live::open(root.path(), &b).expect("writable");

        one.publish(&[row(a, 1, 1_000)], Summary::default())
            .expect("writable");
        two.publish(&[row(b, 1, 2_000), row(b, 2, 1_500)], Summary::default())
            .expect("writable");

        let seen = current(root.path());
        assert_eq!(seen.len(), 2, "two runs, two files, no collision");
        let found_a = seen.iter().find(|(id, _, _)| *id == a).expect("run a");
        let found_b = seen.iter().find(|(id, _, _)| *id == b).expect("run b");
        assert_eq!(found_a.2.len(), 1);
        assert_eq!(found_b.2.len(), 2);
        assert_eq!(found_b.2[0].t_milli, 2_000);

        // Deterministic order, so two polls of the same page cannot swap them.
        assert!(
            seen.windows(2).all(|w| w[0].0 <= w[1].0),
            "runs come back in identity order"
        );
    }

    /// A file this build does not recognise is skipped, not decoded.
    ///
    /// This directory is transient by design: a half-written file, or one from
    /// an older build whose process is long gone, is an ordinary state. Refusing
    /// the whole listing over one would make a live view that a stale byte can
    /// switch off.
    #[test]
    fn a_foreign_or_short_file_is_skipped_and_the_rest_still_read() {
        let root = tempdir();
        let good = [9_u8; 32];
        let mut live = Live::open(root.path(), &good).expect("writable");
        live.publish(&[row(good, 1, 4_000)], Summary::default())
            .expect("writable");

        let dir = Live::dir(root.path());
        std::fs::write(
            dir.join("not-a-live-file.bin"),
            b"BRUTEXFR\x03\0\0\0\0\0\0\0",
        )
        .expect("writable");
        std::fs::write(dir.join("truncated.bin"), b"BRU").expect("writable");
        // Right magic, wrong version: a process from another build.
        let mut wrong = Vec::from(b"BRUTEXLV");
        wrong.extend_from_slice(&99_u32.to_le_bytes());
        wrong.extend_from_slice(&[0_u8; 4]);
        std::fs::write(dir.join("future.bin"), &wrong).expect("writable");

        let seen = current(root.path());
        assert_eq!(
            seen.len(),
            1,
            "three unreadable files are skipped and the real one still reads"
        );
        assert_eq!(seen[0].2[0].t_milli, 4_000);
    }

    /// A row that does not seal is not decoded, and a hex-looking name is not
    /// trusted.
    ///
    /// # Two defects an adversarial pass found, both latent
    ///
    /// This module was the ONLY reader in the workspace that skipped
    /// `Row::seal_matches`, whose own doc says the caller must check it first --
    /// while reading the one file in this store that is rewritten by a live
    /// process. A row torn mid-stride decodes to an arbitrary `t_milli` and
    /// would have been published to a browser as a finding.
    ///
    /// And `u8::from_str_radix` accepts a leading `+` on an unsigned type, so a
    /// file named `+1` thirty-two times over parsed to `[1u8; 32]` -- a real
    /// identity another run can own. Two files would then report one run, and
    /// the stable sort would order them by `read_dir`, which is what the sort
    /// exists to avoid.
    #[test]
    fn a_torn_row_and_a_forged_name_are_both_refused() {
        let root = tempdir();
        let identity = [3_u8; 32];
        let mut live = Live::open(root.path(), &identity).expect("writable");
        live.publish(
            &[row(identity, 1, 4_000), row(identity, 2, 3_000)],
            Summary::default(),
        )
        .expect("writable");

        // CORRUPT THE SECOND ROW'S PAYLOAD, leaving its seal stale. Byte 3 of
        // the row is inside `identity`, well clear of the seal's own eight.
        let path = Live::path(root.path(), &identity);
        let mut raw = std::fs::read(&path).expect("readable");
        let second = ROWS_AT + STRIDE_BYTES;
        raw[second + 3] ^= 0xFF;
        std::fs::write(&path, &raw).expect("writable");

        let seen = current(root.path());
        assert_eq!(seen.len(), 1);
        assert_eq!(
            seen[0].2.len(),
            1,
            "the torn row is refused by its seal and the read stops there -- a \
             skipped middle row would renumber the ranking and show rank 3 where \
             rank 4 is"
        );
        assert_eq!(seen[0].2[0].t_milli, 4_000, "the intact row still reads");

        // A NAME THAT PARSES BUT IS NOT HEX. `+1` x32 is 64 characters and
        // `from_str_radix` accepts every pair, yielding [1u8; 32].
        let forged = Live::dir(root.path()).join(format!("{}.bin", "+1".repeat(32)));
        let mut body = Vec::from(MAGIC);
        body.extend_from_slice(&VERSION.to_le_bytes());
        body.extend_from_slice(&[0_u8; 4]);
        body.extend_from_slice(&Summary::default().to_bytes());
        std::fs::write(&forged, &body).expect("writable");

        let after = current(root.path());
        assert_eq!(
            after.len(),
            1,
            "a filename that is not lowercase hex is not an identity, however \
             willingly `from_str_radix` parses it"
        );
    }

    /// The affordability claim is arithmetic, not a table to trust.
    #[test]
    fn the_rewrite_count_is_logarithmic_in_the_search() {
        // The figures this module's header quotes, recomputed. They include the
        // `keep` rewrites that FILL the heap: the trigger this models is "the
        // top-N moved", and it moves on each of the first `keep` arrivals too.
        // Omitting them understated the count by 25 and made the function
        // non-monotonic at the seam -- see `expected_rewrites`.
        assert_eq!(expected_rewrites(25, 10_000), 174);
        assert_eq!(expected_rewrites(25, 1_000_000), 289);
        assert_eq!(expected_rewrites(25, 84_000_000), 400);

        // MONOTONIC EVERYWHERE, which it was not. `f(25, 25)` was 25 and
        // `f(25, 26)` was 0 -- one more candidate reporting fewer rewrites --
        // and it stayed below 25 until about `25e`. The old test jumped from
        // `(25, 10)` to `(25, 10_000)` and stepped over the entire broken range,
        // so it passed while the function was wrong across sixty inputs.
        let mut previous = 0;
        for weighed in 0..200_u64 {
            let now = expected_rewrites(25, weighed);
            assert!(
                now >= previous,
                "weighing {weighed} candidates cannot need fewer rewrites than \
                 {} did: {previous} -> {now}",
                weighed.saturating_sub(1)
            );
            previous = now;
        }

        // GROWTH IS LOGARITHMIC: eight thousand times the candidates costs
        // roughly two and a half times the writes. That ratio is the whole
        // argument for publishing on every improvement rather than on a timer.
        let small = expected_rewrites(25, 10_000);
        let huge = expected_rewrites(25, 84_000_000);
        assert!(
            huge < small.saturating_mul(3),
            "8,400x the search must not cost 3x the writes: {small} -> {huge}"
        );

        // KEEPING NOTHING REWRITES NOTHING. This asserted 500 under the words
        // "degenerate inputs answer rather than divide by zero", which confused
        // not-dividing-by-zero with returning the input. A top-0 is always empty
        // and can never move.
        assert_eq!(expected_rewrites(0, 500), 0);
        assert_eq!(
            expected_rewrites(25, 10),
            10,
            "fewer candidates than the heap holds means every one is a rewrite"
        );
    }

    /// A scratch directory that removes itself.
    struct Dir(std::path::PathBuf);
    impl Dir {
        fn path(&self) -> &std::path::Path {
            &self.0
        }
    }
    impl Drop for Dir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// No `tempfile` dependency: gate 22 pins what this workspace may name, and
    /// a scratch directory is four lines.
    fn tempdir() -> Dir {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let nth = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let at = std::env::temp_dir().join(format!("brutex-live-{}-{nth}", std::process::id()));
        std::fs::create_dir_all(&at).expect("a writable temp directory");
        Dir(at)
    }
}

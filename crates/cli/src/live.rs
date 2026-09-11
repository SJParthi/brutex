//! The best rows a run has found SO FAR, written while it is still running.
//!
//! # The hole this fills, measured
//!
//! A sweep writes its ledger row, its frontier rows and its trades in
//! [`crate::record_all`], which runs once, at the very end, after everything.
//! MEASURED on 2026-08-29: a 60-minute `audit-range` over 81 months ran for
//! **48 minutes and wrote zero bytes**. Its results directory still held the
//! previous run's four durable files, untouched. Killed at minute 47 it would have left
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
//! writes FRONTIER, TRADES, a cardinality RECEIPT, and the LEDGER marker last.
//! A detail row whose run has no ledger row is an orphan, and the ledger row
//! does not exist until the run ends. Writing live rows into `frontier.bin` would therefore manufacture
//! exactly that orphan — every partial run leaving detail rows whose parent
//! never arrives — and it would do it by the one route the ordering cannot see.
//!
//! So this is not history. It is **replaced whole by rename**, it is not appended
//! to, and a run that completes leaves its permanent record through the ordinary
//! path unchanged. A file here is a statement about a run that is happening, not
//! a record of one that happened, and [`Live::finish`] removes it when the real
//! rows land.
//!
//! # A KILLED RUN LEAVES ITS FILE, AND THIS MODULE CANNOT REMOVE IT
//!
//! [`Live::finish`] is the only remover and it runs only on the success path.
//! There is no `Drop`, so SIGTERM, SIGKILL and a closed terminal each leave a
//! file behind that says a search is in flight forever. MEASURED on 2026-09-01:
//! one 6,840-byte file -- 25 rows, last written 21:41 -- was still being served
//! as a running sweep hours after the process that wrote it was gone.
//!
//! [`census`] now dates every file it reads and [`Freshness`] carries the
//! answer, but read [`STALE_AFTER_SECS`] before trusting it. The writer
//! publishes ONCE, immediately before the exit grid, so an untouched file is the
//! normal state for 87.6% of a healthy run's wall clock. mtime can say "nothing
//! has written here since yesterday". It cannot say "this run is dead", and
//! nothing below pretends otherwise.
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
const VERSION: u32 = 2;

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

/// How long a live file may sit untouched before [`census`] calls it STALE.
///
/// # The signal is mtime, and mtime is the WRONG signal
///
/// What follows is the honest half-answer, and the whole reason it is a day.
///
/// [`Live::finish`] is the only remover and it runs only on the success path.
/// A killed run therefore leaves a file claiming a search is in flight, forever.
///
/// The obvious remedy is to time the file out, and the obvious remedy is wrong,
/// because of what the writer actually does: **the file is written exactly twice
/// and then deleted.** [`Live::open`] publishes an empty one, `publish_ranked`
/// publishes the ranked rows ONCE immediately before the exit grid, and
/// [`Live::finish`] removes it at the end. This module's own header computes
/// ~400 rewrites for an 84-million-candidate run; that is what the FORMAT
/// affords, not what the caller does.
///
/// So the mtime of a perfectly healthy live file is pinned at the instant the
/// exit grid started -- and the exit grid is **87.6% of a run's wall clock**,
/// measured by sampling a real `range-all`: 18,068 samples in the grid against
/// 1,403 in the sweep and 887 everywhere else. Silence on this file is not a
/// symptom. It is the normal state for the overwhelming majority of every run,
/// by construction.
///
/// A threshold cannot separate "killed" from "pricing". It can only separate
/// "recently" from "long ago". Marking a healthy nine-hour grid dead is worse
/// than the bug it would be fixing, because a flag an operator learns to ignore
/// is a flag that is not there.
///
/// # Why a day, and what crossing it does NOT claim
///
/// The longest run recorded anywhere in this repository is a 60-minute
/// `audit-range` whose grid phase was about 42 minutes of that. A day is 24x
/// the longest measured run and ~34x the longest measured grid, so crossing it
/// is not a judgement about how long a sweep is allowed to take. It is the flat
/// statement *nothing has written here since yesterday*, which an operator can
/// act on without knowing anything about the run at all.
///
/// [`Freshness`] carries the measured idle seconds either way, so a caller that
/// wants a tighter line has the raw number and does not need this one.
///
/// # The correct fix is a heartbeat, and it is not in this module
///
/// Silence would mean something if the grid loop touched the file on a
/// schedule. That is a call inside the exit grid in `crates/cli/src/lib.rs`,
/// not a reader here, and until it exists NO reader can answer "is this run
/// alive" -- only "when was this file last written". §3 rule 6: the limit is
/// stated rather than papered over with a shorter timeout that would look
/// decisive and be wrong.
///
/// **UNVERIFIED as a measured bound.** No bench in this workspace
/// times this, so the shape above is read from the source rather
/// than measured. `CLAUDE.md` §3 rule 6.
pub const STALE_AFTER_SECS: u64 = 24 * 60 * 60;

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
    /// is still going when it is not.
    ///
    /// # It is the ONLY remover, and it runs only on the success path
    ///
    /// There is no `Drop` impl on [`Live`], so a run ended by SIGTERM, SIGKILL
    /// or a closed terminal never reaches this method and leaves its file
    /// behind permanently. MEASURED on 2026-09-01: a 6,840-byte file -- 25 rows,
    /// written at 21:41 -- was still served as an in-flight run hours later.
    ///
    /// [`census`] dates what it reads and [`Freshness`] reports it, which is a
    /// half-answer and is documented as one on [`STALE_AFTER_SECS`]: the file is
    /// published once and then sits untouched for the 87.6% of a run the exit
    /// grid occupies, so mtime cannot tell a killed run from a pricing one. This
    /// method staying the only remover is why that half-answer is needed at all.
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

/// How long ago a live file was last written, and what that does and does not
/// prove.
///
/// **No variant here says a process is alive.** A run killed one second ago
/// leaves a file that is [`Self::Touched`], and a healthy run deep in the exit
/// grid eventually leaves one that is [`Self::Stale`] -- see
/// [`STALE_AFTER_SECS`] for why mtime cannot do better than that, and for the
/// heartbeat that would. Each variant is a statement about the FILE.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Freshness {
    /// Written `secs` ago, which is less than [`STALE_AFTER_SECS`]. Something
    /// wrote here recently; it does not follow that anything is writing here
    /// now.
    Touched(u64),
    /// Not written for `secs`, which is at least [`STALE_AFTER_SECS`]. Evidence
    /// of an abandoned run, not proof of one.
    Stale(u64),
    /// No modification time could be read, or the file claims to have been
    /// written in the FUTURE -- a clock that moved, or a store on a filesystem
    /// that keeps no time.
    ///
    /// Its own state rather than folded into "recent", which is the distinction
    /// the rest of this module exists to preserve: "I could not look" and
    /// "there is nothing to see" are two facts, and rendering them identically
    /// is the failure `CLAUDE.md` §4 bans.
    #[default]
    Unknown,
}

impl Freshness {
    /// Seconds since the last write, or [`None`] when there is no answer.
    #[must_use]
    pub const fn idle_secs(self) -> Option<u64> {
        match self {
            Self::Touched(secs) | Self::Stale(secs) => Some(secs),
            Self::Unknown => None,
        }
    }

    /// Whether this file crossed [`STALE_AFTER_SECS`].
    ///
    /// [`None`] when the file could not be dated -- **not** `false`, which
    /// would report a measurement nobody took as a clean bill of health.
    #[must_use]
    pub const fn is_stale(self) -> Option<bool> {
        match self {
            Self::Touched(_) => Some(false),
            Self::Stale(_) => Some(true),
            Self::Unknown => None,
        }
    }
}

/// One live file, decoded, plus the one fact the format itself does not carry.
///
/// A bare `(identity, summary, rows)` tuple until 2026-09-01, and the fourth
/// member is why it is a struct now: four positions at a call site are four
/// things a reader has to count rather than read.
#[derive(Clone, Debug)]
pub struct Run {
    /// The nine-term run identity, from row zero or -- for a file that has
    /// found nothing yet -- from the filename.
    pub identity: [u8; 32],
    /// What the run has weighed so far and the bar its rows must clear.
    pub summary: Summary,
    /// The ranked prefix that decoded, best first.
    pub rows: Vec<Row>,
    /// When the file was last written, judged against [`STALE_AFTER_SECS`].
    pub freshness: Freshness,
}

/// Every run with a live file right now, in identity order.
///
/// NOT "newest activity first", which this said while sorting by identity. The
/// format carries no timestamp, no pid and no heartbeat, so there is nothing an
/// activity order could be computed from -- and a caller trusting that wording
/// would render a three-week-old abandoned file at the top because its identity
/// happens to start with a zero byte.
///
/// **It DROPS [`Freshness`].** The file's modification time is not in the tuple
/// and cannot be, so a caller here cannot tell a run that is being written from
/// one abandoned in April. Any surface an operator watches during a run wants
/// [`census`] instead, for this reason and for the two below.
///
/// # Errors
///
/// Never. A directory that cannot be listed reads as no live runs -- which is
/// NOT "the truth a reader can act on", as this sentence used to claim: it is
/// one of two very different facts rendered identically. Callers that need to
/// tell them apart, and any surface an operator watches during a run must,
/// should use [`census`] instead.
#[must_use]
pub fn current(root: &Path) -> Vec<([u8; 32], Summary, Vec<Row>)> {
    census(root)
        .runs
        .into_iter()
        .map(|run| (run.identity, run.summary, run.rows))
        .collect()
}

/// What could be read, and what could not.
///
/// **`current` returns only what it could decode, and an empty vector from it
/// is indistinguishable from a machine with nothing running.** That is the
/// shape §4 bans: a five-hour sweep whose live file is one version behind
/// reports an idle machine, and the operator watching `/live.json` has no way
/// to tell "nothing is running" from "I could not read three files".
///
/// Skipping is still right -- this directory is transient by design and a
/// half-written file is an ordinary state -- but it must be COUNTED. A
/// directory that cannot be listed is likewise not a directory that is empty.
#[derive(Clone, Debug, Default)]
pub struct Census {
    /// Every live file that decoded, in identity order.
    pub runs: Vec<Run>,
    /// Entries NAMED like a live file whose bytes are not one: wrong magic,
    /// unknown version, short header, or an unreadable count.
    ///
    /// This is the corruption count, and it is now only that. See
    /// [`Self::strays`] for what used to land here and should not have.
    pub skipped: u32,
    /// Directory entries that are not a live file's NAME at all -- anything
    /// but 64 hex characters and `.bin`.
    ///
    /// Counted apart from [`Self::skipped`] because they are a different fact
    /// and `/live.json` publishes `skipped` as *"something is there and I
    /// cannot read it"* -- a corruption alarm. The commonest entry here is not
    /// a corruption at all: [`Live::publish`] names its temp file
    /// `<hex>.<pid>.tmp`, so a run killed between the write and the rename
    /// leaves a COMPLETE, VALID live file under a name that is not a run's.
    ///
    /// Before the name filter that was worse than a false alarm. A `.tmp`
    /// carrying rows decoded fine, took its identity from row zero, and listed
    /// **the same run twice**; an empty one -- the header-only file
    /// [`Live::open`] writes -- failed [`identity_from_name`] and inflated the
    /// corruption count instead.
    pub strays: u32,
    /// Runs whose file has not been written for at least [`STALE_AFTER_SECS`].
    ///
    /// **Read that constant before reporting this number as dead runs.** It is
    /// "not written since yesterday", which is evidence and is not proof: the
    /// writer publishes once and the exit grid is 87.6% of a healthy run.
    pub stale: u32,
    /// Runs whose file carries no usable modification time, so the staleness
    /// question has no answer for them.
    ///
    /// Present so that `stale: 0` cannot be read as "and every file was
    /// checked". A count of unanswered questions beside a count of answers is
    /// the same discipline [`Self::skipped`] applies to the bytes.
    pub undated: u32,
    /// False when the live directory itself could not be listed. Distinct from
    /// an empty directory, which lists fine and yields no runs.
    pub listed: bool,
}

/// Reads the live directory, counting what it could not show.
#[must_use]
pub fn census(root: &Path) -> Census {
    census_at(root, std::time::SystemTime::now())
}

/// [`census`], with the instant staleness is judged against passed in.
///
/// **Split so a test can state a clock it cannot cause.** [`STALE_AFTER_SECS`]
/// is a day and a test that waited for it would not be a test; nothing in this
/// workspace may name `filetime`, so backdating a file is not on offer either.
/// Moving `now` is the same arithmetic seen from the other end, and it reaches
/// all three [`Freshness`] arms: forward for [`Freshness::Stale`], backward for
/// [`Freshness::Unknown`] -- a file written after the instant it is judged at is
/// exactly the clock-moved case that variant exists for.
///
/// The clock is read ONCE for the whole walk, deliberately. Two runs judged
/// against two different instants can be ordered by the reads rather than by
/// the files, and §3 rule 5 applies to a page's rows as much as to a total.
#[must_use]
pub fn census_at(root: &Path, now: std::time::SystemTime) -> Census {
    let entries = match std::fs::read_dir(Live::dir(root)) {
        Ok(entries) => entries,
        // A live directory that does not exist is a store where nothing has
        // ever run, and that IS "nothing in flight" -- it is listed, and it is
        // empty. Every other error is "I could not look", which is the fact
        // this type exists to keep separate: permissions, a broken symlink, a
        // volume that went away mid-sweep.
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => {
            return Census {
                listed: true,
                ..Census::default()
            };
        }
        Err(_) => {
            return Census {
                listed: false,
                ..Census::default()
            };
        }
    };
    let mut runs: Vec<Run> = Vec::new();
    let mut skipped: u32 = 0;
    let mut strays: u32 = 0;
    let mut stale: u32 = 0;
    let mut undated: u32 = 0;
    for entry in entries.flatten() {
        let path = entry.path();
        // A NAME THAT IS NOT A RUN'S IS NOT A CORRUPT RUN, and the reader used
        // to iterate every entry with no filter at all. See `Census::strays`.
        //
        // THE FILTER AND THE PARSE ARE ONE CALL, deliberately. A separate
        // `is_live_name` predicate beside `identity_from_name` would be two
        // spellings of one rule, free to disagree the first time either is
        // edited -- and `read_one` would then re-derive from the name a second
        // time. The name is decided here, once, and handed down.
        let Some(named) = identity_from_name(&path) else {
            strays = strays.saturating_add(1);
            continue;
        };
        if let Some(run) = read_one(&path, named, now) {
            match run.freshness {
                Freshness::Stale(_) => stale = stale.saturating_add(1),
                Freshness::Unknown => undated = undated.saturating_add(1),
                Freshness::Touched(_) => {}
            }
            runs.push(run);
        } else {
            skipped = skipped.saturating_add(1);
        }
    }
    // Deterministic order, because two runs finishing in the same millisecond
    // must not swap places between two polls of the same page. The identity is
    // the key nothing else shares.
    runs.sort_by_key(|run| run.identity);
    Census {
        runs,
        skipped,
        strays,
        stale,
        undated,
        listed: true,
    }
}

/// When a file was last written, judged against [`STALE_AFTER_SECS`].
fn freshness_of(meta: &std::fs::Metadata, now: std::time::SystemTime) -> Freshness {
    // BOTH FAILURES ARE ONE ANSWER, and they are collapsed with combinators
    // rather than two `else` arms so this function holds no branch a test
    // cannot reach -- `modified` is infallible on every platform this runs on.
    //
    // The second failure is the interesting one: `duration_since` refuses when
    // the file was written AFTER `now`, which is a clock that moved. Reading
    // that refusal as zero seconds would report the most suspicious file on the
    // disk as the healthiest one on it.
    match meta
        .modified()
        .ok()
        .and_then(|written| now.duration_since(written).ok())
        .map(|idle| idle.as_secs())
    {
        Some(secs) if secs >= STALE_AFTER_SECS => Freshness::Stale(secs),
        Some(secs) => Freshness::Touched(secs),
        None => Freshness::Unknown,
    }
}

/// One live file, or `None` if it is not one.
///
/// A file with the wrong magic, an unknown version, a short header or a count
/// larger than the bytes present is SKIPPED rather than refused: this directory
/// is transient by design and a half-written file is an ordinary state, not a
/// corruption to report.
///
/// `named` is the identity [`identity_from_name`] already parsed out of the
/// path -- passed in rather than re-derived, because the caller had to parse it
/// to decide this entry was a live file at all.
///
/// `now` is the instant [`Freshness`] is measured against; it is the caller's
/// so that every run in one census is dated by one clock read.
fn read_one(path: &Path, named: [u8; 32], now: std::time::SystemTime) -> Option<Run> {
    read_one_with_limit(path, named, now, None)
}

fn read_one_with_limit(
    path: &Path,
    named: [u8; 32],
    now: std::time::SystemTime,
    row_limit: Option<usize>,
) -> Option<Run> {
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
    //
    // ONE `metadata` CALL SERVES BOTH the length check and the modification
    // time. Two calls would be two answers about one file, and this module has
    // recorded what that costs.
    let meta = file.metadata().ok()?;
    let len = meta.len();
    let freshness = freshness_of(&meta, now);
    let available = usize::try_from(len.saturating_sub(ROWS_AT as u64)).ok()? / STRIDE_BYTES;
    if let Some(limit) = row_limit
        && (count > limit
            || len != u64::try_from(ROWS_AT.checked_add(count.checked_mul(STRIDE_BYTES)?)?).ok()?)
    {
        return None;
    }
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
        let Ok(row) = Row::from_bytes(&raw) else {
            // A sealed row can still carry an unknown direction or non-zero
            // reserved byte. Like a seal failure, that ends the ranked prefix:
            // skipping it would renumber every later row.
            break;
        };
        if row_limit.is_some()
            && (row.identity != named || usize::from(row.rank) != nth.checked_add(1)?)
        {
            return None;
        }
        if nth == 0 {
            identity = row.identity;
        }
        rows.push(row);
    }
    if row_limit.is_some() && rows.len() != count {
        return None;
    }
    if rows.is_empty() {
        // A started-but-empty file is a real state -- a run that has not yet
        // found anything -- and the identity comes from the FILENAME, which is
        // where it is written before any row exists. The caller parsed it.
        identity = named;
    }
    Some(Run {
        identity,
        summary,
        rows,
        freshness,
    })
}

/// Maximum directory entries inspected by one live dashboard refresh.
pub const LIVE_ENTRY_LIMIT: usize = 4_096;
/// Maximum independently named live runs admitted by one dashboard refresh.
pub const LIVE_RUN_LIMIT: usize = 128;
/// Maximum rows admitted from each live run, before any row buffer is allocated.
pub const LIVE_ROW_LIMIT: usize = 256;

#[derive(Clone, Copy, PartialEq, Eq)]
struct LiveStamp {
    len: u64,
    modified: std::time::SystemTime,
    created: Option<std::time::SystemTime>,
    #[cfg(unix)]
    generation: (u64, u64, i64, i64),
}

impl LiveStamp {
    fn of(path: &Path) -> Result<Self, Refusal> {
        let metadata = std::fs::metadata(path)
            .map_err(|why| format!("{} could not be measured: {why}", path.display()))?;
        if !metadata.is_file() {
            return Err(format!("{} is not a regular live file", path.display()));
        }
        Ok(Self {
            len: metadata.len(),
            modified: metadata.modified().map_err(|why| {
                format!(
                    "{} has no readable modification time: {why}",
                    path.display()
                )
            })?,
            created: metadata.created().ok(),
            #[cfg(unix)]
            generation: {
                use std::os::unix::fs::MetadataExt as _;
                (
                    metadata.dev(),
                    metadata.ino(),
                    metadata.ctime(),
                    metadata.ctime_nsec(),
                )
            },
        })
    }
}

/// Bounded live-file index. Every refresh checks directory membership and each
/// file's metadata; unchanged files reuse their previously verified rows.
///
/// Cost includes directory entries and returned rows; changed files additionally
/// decode their rows. Explicit entry/run/row ceilings bound each term. Cached
/// values are replaced only after one entire refresh succeeds.
#[derive(Default)]
pub struct CensusCache {
    root: Option<PathBuf>,
    runs: std::collections::BTreeMap<PathBuf, (LiveStamp, Run)>,
}

impl CensusCache {
    /// Refreshes the complete bounded live snapshot without creating files.
    ///
    /// # Errors
    /// Refuses unreadable entries, over-limit directories/files, incomplete or
    /// corrupt named live files, and a file changing while it is read.
    pub fn refresh(&mut self, root: &Path) -> Result<Census, Refusal> {
        self.refresh_at(root, std::time::SystemTime::now())
    }

    fn refresh_at(&mut self, root: &Path, now: std::time::SystemTime) -> Result<Census, Refusal> {
        let entries = match std::fs::read_dir(Live::dir(root)) {
            Ok(entries) => entries,
            Err(why) if why.kind() == std::io::ErrorKind::NotFound => {
                self.runs.clear();
                self.root = Some(root.to_path_buf());
                return Ok(Census {
                    listed: true,
                    ..Census::default()
                });
            }
            Err(why) => return Err(format!("live directory could not be listed: {why}")),
        };
        let mut next = std::collections::BTreeMap::new();
        let mut census = Census {
            listed: true,
            ..Census::default()
        };
        for (index, entry) in entries.enumerate() {
            if index >= LIVE_ENTRY_LIMIT {
                return Err(format!(
                    "live directory exceeds {LIVE_ENTRY_LIMIT} entries; no prefix snapshot is exposed"
                ));
            }
            let path = entry
                .map_err(|why| format!("live directory entry is unreadable: {why}"))?
                .path();
            let Some(identity) = identity_from_name(&path) else {
                census.strays = census.strays.saturating_add(1);
                continue;
            };
            if path != Live::path(root, &identity) {
                return Err(format!(
                    "{} is not a canonical live filename; no duplicate identity is inferred",
                    path.display()
                ));
            }
            if next.len() >= LIVE_RUN_LIMIT {
                return Err(format!(
                    "live directory exceeds {LIVE_RUN_LIMIT} runs; no prefix snapshot is exposed"
                ));
            }
            let stamp = LiveStamp::of(&path)?;
            let max_bytes = ROWS_AT.saturating_add(LIVE_ROW_LIMIT.saturating_mul(STRIDE_BYTES));
            if stamp.len > u64::try_from(max_bytes).unwrap_or(u64::MAX) {
                return Err(format!(
                    "{} exceeds the live snapshot limit of {LIVE_ROW_LIMIT} rows",
                    path.display()
                ));
            }
            let cached = (cfg!(unix) && self.root.as_deref() == Some(root))
                .then(|| self.runs.get(&path))
                .flatten()
                .filter(|(old, _)| *old == stamp);
            let mut run = if let Some((_, run)) = cached {
                run.clone()
            } else {
                read_one_with_limit(&path, identity, now, Some(LIVE_ROW_LIMIT)).ok_or_else(|| {
                    format!("{} is unreadable, incomplete, corrupt, or above the live row limit; no ranked prefix is exposed", path.display())
                })?
            };
            if LiveStamp::of(&path)? != stamp {
                return Err(format!(
                    "{} changed during the live refresh; retry for one coherent snapshot",
                    path.display()
                ));
            }
            run.freshness = now
                .duration_since(stamp.modified)
                .map_or(Freshness::Unknown, |age| {
                    if age.as_secs() >= STALE_AFTER_SECS {
                        Freshness::Stale(age.as_secs())
                    } else {
                        Freshness::Touched(age.as_secs())
                    }
                });
            match run.freshness {
                Freshness::Stale(_) => census.stale = census.stale.saturating_add(1),
                Freshness::Unknown => census.undated = census.undated.saturating_add(1),
                Freshness::Touched(_) => {}
            }
            census.runs.push(run.clone());
            next.insert(path, (stamp, run));
        }
        census.runs.sort_by_key(|run| run.identity);
        self.root = Some(root.to_path_buf());
        self.runs = next;
        Ok(census)
    }
}

/// The identity a live file is named for, and [`census_at`]'s name filter.
///
/// # It requires `.bin` now, and a killed writer is why
///
/// This read `file_stem` and so answered the same for `<hex>.bin` and for
/// `<hex>.tmp`. [`Live::publish`] writes `<hex>.<pid>.tmp` and renames it over
/// the real name, so a writer killed in that window leaves a **complete, valid
/// live file** under a temp name -- and the reader, which iterated every
/// directory entry with no filter at all, then either listed the same run TWICE
/// (when the temp file carried rows, whose row-zero identity it trusted) or
/// counted it as a corrupt file (when it was the header-only file
/// [`Live::open`] writes, whose `<hex>.<pid>` stem parses to nothing).
///
/// Neither is true. `<hex>.<pid>` is 64 characters plus a dot plus digits, so
/// requiring the whole NAME to be 64 hex characters and `.bin` refuses it on
/// its face -- and refuses `<hex>`, `<hex>.tmp`, `notes.bin` and a
/// subdirectory with it.
///
/// # Why the whole check lives here rather than beside the walk
///
/// [`census_at`] filters by calling this and keeping the answer, so there is
/// one rule and it is parsed once. A predicate beside a parser is two spellings
/// of one fact -- the shape this repository has recorded the cost of more than
/// once.
fn identity_from_name(path: &Path) -> Option<[u8; 32]> {
    let stem = path
        .file_name()
        .and_then(std::ffi::OsStr::to_str)?
        .strip_suffix(".bin")?;
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
    use super::{
        Freshness, Live, MAGIC, ROWS_AT, STALE_AFTER_SECS, Summary, VERSION, census, census_at,
        current, expected_rewrites,
    };
    use crate::frontier::Row;
    use crate::frontier::STRIDE_BYTES;

    /// One file name of the only shape the reader admits: sixty-four hex
    /// characters and `.bin`, the byte repeated thirty-two times.
    ///
    /// A test that wants a file `census` will actually OPEN has to name it like
    /// one. Before the name filter any string would do, which is why three
    /// fixtures below had to be renamed -- they were about to start proving
    /// something other than what they are named for.
    fn hex_name(byte: u8) -> String {
        use core::fmt::Write as _;
        let mut out = String::with_capacity(68);
        for _ in 0..32 {
            let _ = write!(out, "{byte:02x}");
        }
        out.push_str(".bin");
        out
    }

    /// A row with every field named, because `Row` has no `Default` -- it is
    /// built from a `Scored` and a grid `Cell` in production, and a test that
    /// wanted a partial one would be asking for a row that cannot exist.
    fn row(identity: [u8; 32], rank: u16, t_milli: i64) -> Row {
        Row {
            identity,
            rank,
            direction: costs::fill::Direction::Long,
            rules: crate::Rules::elite(400, 25),
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

    #[test]
    fn bounded_live_cache_observes_atomic_updates_removal_and_root_changes() {
        let root = tempdir();
        let identity = [91_u8; 32];
        let mut cache = super::CensusCache::default();
        assert!(
            cache
                .refresh(root.path())
                .expect("empty directory")
                .runs
                .is_empty()
        );
        let mut live = Live::open(root.path(), &identity).expect("live");
        live.publish(&[row(identity, 1, 100)], Summary::default())
            .expect("first");
        assert_eq!(
            cache.refresh(root.path()).expect("first read").runs[0].rows[0].t_milli,
            100
        );
        assert_eq!(
            cache.refresh(root.path()).expect("cached read").runs[0].rows[0].t_milli,
            100
        );
        live.publish(&[row(identity, 1, 200)], Summary::default())
            .expect("replacement");
        assert_eq!(
            cache.refresh(root.path()).expect("changed read").runs[0].rows[0].t_milli,
            200
        );
        let other = tempdir();
        assert!(
            cache
                .refresh(other.path())
                .expect("other root")
                .runs
                .is_empty()
        );
        assert_eq!(
            cache
                .refresh(root.path())
                .expect("original root")
                .runs
                .len(),
            1
        );
        assert!(live.finish().is_empty());
        assert!(cache.refresh(root.path()).expect("removed").runs.is_empty());
    }

    #[test]
    fn bounded_live_cache_refuses_torn_corrupt_and_misidentified_rows_without_prefixes() {
        let root = tempdir();
        let identity = [92_u8; 32];
        let mut live = Live::open(root.path(), &identity).expect("live");
        let mut cache = super::CensusCache::default();
        for rows in [vec![row([93; 32], 1, 100)], vec![row(identity, 2, 100)]] {
            live.publish(&rows, Summary::default())
                .expect("malformed fixture");
            assert!(cache.refresh(root.path()).is_err());
        }
        live.publish(
            &[row(identity, 1, 100), row(identity, 2, 200)],
            Summary::default(),
        )
        .expect("valid");
        assert_eq!(
            cache.refresh(root.path()).expect("valid read").runs.len(),
            1
        );
        let path = Live::path(root.path(), &identity);
        let mut bytes = std::fs::read(&path).expect("fixture");
        let original = bytes.clone();
        bytes[ROWS_AT + STRIDE_BYTES + 50] ^= 1;
        std::fs::write(&path, &bytes).expect("corruption");
        assert!(
            cache.refresh(root.path()).is_err(),
            "cached rows must not hide same-length corruption"
        );
        std::fs::write(&path, &original[..original.len() - 1]).expect("torn tail");
        assert!(
            cache.refresh(root.path()).is_err(),
            "no valid prefix on truncation"
        );
        std::fs::write(&path, &original).expect("restored");
        assert_eq!(
            cache.refresh(root.path()).expect("recovery").runs[0]
                .rows
                .len(),
            2
        );
    }

    #[test]
    fn bounded_live_cache_refuses_row_run_and_directory_limits() {
        let root = tempdir();
        let mut cache = super::CensusCache::default();
        for byte in 0..=u8::try_from(super::LIVE_RUN_LIMIT).expect("bound fits byte") {
            Live::open(root.path(), &[byte; 32]).expect("empty named run");
        }
        assert!(
            cache
                .refresh(root.path())
                .expect_err("run ceiling")
                .contains("runs")
        );
        let root = tempdir();
        let identity = [95_u8; 32];
        let mut live = Live::open(root.path(), &identity).expect("live");
        let rows: Vec<_> = (1..=super::LIVE_ROW_LIMIT + 1)
            .map(|rank| row(identity, u16::try_from(rank).expect("rank fits"), 100))
            .collect();
        live.publish(&rows, Summary::default())
            .expect("over-limit fixture");
        assert!(
            cache
                .refresh(root.path())
                .expect_err("row ceiling")
                .contains("rows")
        );
        let root = tempdir();
        std::fs::create_dir_all(Live::dir(root.path())).expect("directory");
        for index in 0..=super::LIVE_ENTRY_LIMIT {
            std::fs::write(Live::dir(root.path()).join(format!("stray-{index}")), [])
                .expect("stray");
        }
        assert!(
            cache
                .refresh(root.path())
                .expect_err("entry ceiling")
                .contains("entries")
        );
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

        // THE NAMES ARE HEX, and that is load-bearing rather than cosmetic.
        // These fixtures were called `not-a-live-file.bin`, `truncated.bin` and
        // `future.bin`, so after the name filter they would have been counted
        // as STRAYS and this test would have stopped exercising the corruption
        // path it is named for -- passing for the wrong reason.
        let dir = Live::dir(root.path());
        let foreign = dir.join(hex_name(0xa1));
        std::fs::write(&foreign, b"BRUTEXFR\x03\0\0\0\0\0\0\0").expect("writable");
        std::fs::write(dir.join(hex_name(0xb2)), b"BRU").expect("writable");
        // Right magic, wrong version: a process from another build.
        let mut wrong = Vec::from(b"BRUTEXLV");
        wrong.extend_from_slice(&99_u32.to_le_bytes());
        wrong.extend_from_slice(&[0_u8; 4]);
        std::fs::write(dir.join(hex_name(0xc3)), &wrong).expect("writable");

        let seen = current(root.path());
        assert_eq!(
            seen.len(),
            1,
            "three unreadable files are skipped and the real one still reads"
        );
        assert_eq!(seen[0].2[0].t_milli, 4_000);

        // AND THEY ARE COUNTED AS CORRUPTION, not as strays: each carries a
        // live file's name and does not carry a live file's bytes, which is
        // exactly the fact `skipped` publishes.
        let census = census(root.path());
        assert_eq!(census.skipped, 3, "three named files would not decode");
        assert_eq!(
            census.strays, 0,
            "and none of them is a stray -- every name here is one this reader \
             is right to have opened"
        );
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

        let after = census(root.path());
        assert_eq!(
            after.runs.len(),
            1,
            "a filename that is not lowercase hex is not an identity, however \
             willingly `from_str_radix` parses it"
        );
        assert_eq!(
            (after.strays, after.skipped),
            (1, 0),
            "and it is a STRAY rather than a corrupt file: the bytes are a \
             perfectly good live file, the name is not a run's"
        );
    }

    /// A temp file left by a killed writer is neither a run nor a corruption.
    ///
    /// # The two ways this went wrong, and both were silent
    ///
    /// [`Live::publish`] writes `<hex>.<pid>.tmp` and renames it over
    /// `<hex>.bin`. A process killed between the two leaves a **complete,
    /// valid live file** under the temp name, and the reader iterated every
    /// directory entry with no filter at all. So:
    ///
    /// * a temp file carrying ROWS decoded, took its identity from row zero,
    ///   and listed **the same run twice** -- two entries, one identity, and a
    ///   stable sort ordering them by `read_dir`;
    /// * an EMPTY one -- the header-only file [`Live::open`] writes -- failed
    ///   the name parse and landed in `skipped`, which `/live.json` publishes
    ///   as *"something is there and I cannot read it"*. A false corruption
    ///   alarm about a file that is not corrupt.
    #[test]
    fn a_temp_file_from_a_killed_writer_is_a_stray_and_not_a_second_run() {
        let root = tempdir();
        let identity = [0x5a_u8; 32];
        let mut live = Live::open(root.path(), &identity).expect("writable");
        live.publish(
            &[row(identity, 1, 4_000), row(identity, 2, 3_000)],
            Summary::default(),
        )
        .expect("writable");

        // KILLED BETWEEN THE FLUSH AND THE RENAME. The bytes are exactly the
        // real file's -- that is the point, and it is why "is it decodable"
        // cannot be the test.
        let real = Live::path(root.path(), &identity);
        let bytes = std::fs::read(&real).expect("readable");
        let temp = real.with_extension(format!("{}.tmp", std::process::id()));
        std::fs::write(&temp, &bytes).expect("writable");

        // AND A SECOND ONE THAT IS EMPTY, which is the shape `Live::open`
        // writes and the shape that used to inflate the corruption count.
        let other = Live::path(root.path(), &[0x6b_u8; 32]).with_extension("99999.tmp");
        let mut empty = Vec::from(MAGIC);
        empty.extend_from_slice(&VERSION.to_le_bytes());
        empty.extend_from_slice(&[0_u8; 4]);
        empty.extend_from_slice(&Summary::default().to_bytes());
        std::fs::write(&other, &empty).expect("writable");

        let census = census(root.path());
        assert_eq!(
            census.runs.len(),
            1,
            "one run has one file, whatever else is beside it -- a temp copy \
             listed the same identity twice"
        );
        assert_eq!(
            census.runs.first().map(|run| run.identity),
            Some(identity),
            "and it is the run the real name belongs to"
        );
        assert_eq!(
            census.skipped, 0,
            "a stray temp file is NOT a corrupt file, and `skipped` is the \
             alarm that says one is there"
        );
        assert_eq!(
            census.strays, 2,
            "both temp files are counted, and counted apart -- an uncounted \
             stray would be the silent skip this type exists to refuse"
        );
    }

    /// A file nothing has written for a day is marked, and an undatable one is
    /// marked UNKNOWN rather than healthy.
    ///
    /// # What this does not prove, and why the threshold is a day
    ///
    /// `publish_ranked` publishes ONCE, immediately before an exit grid that is
    /// 87.6% of a run's wall clock, so an untouched live file is the normal
    /// state for most of a healthy run. mtime can only say when the file was
    /// last written; it cannot say whether the process is alive. See
    /// [`STALE_AFTER_SECS`] for the heartbeat that could.
    #[test]
    fn a_file_untouched_for_a_day_is_stale_and_an_undatable_one_is_unknown() {
        let root = tempdir();
        let identity = [0x2b_u8; 32];
        let mut live = Live::open(root.path(), &identity).expect("writable");
        live.publish(&[row(identity, 1, 4_000)], Summary::default())
            .expect("writable");

        // A SECOND OF MARGIN, because a filesystem's timestamp granularity is
        // not this test's business. The claim being made is "well under a day",
        // not "exactly zero seconds", and a store whose mtime lands a tick
        // ahead of `SystemTime::now()` would otherwise read as a clock that
        // moved and fail here for a reason that is not the subject.
        let now = std::time::SystemTime::now()
            .checked_add(std::time::Duration::from_secs(1))
            .expect("a clock that can be moved a second forward");
        let fresh = census_at(root.path(), now);
        assert_eq!(fresh.runs.len(), 1);
        assert_eq!((fresh.stale, fresh.undated), (0, 0), "just written");
        let first = fresh.runs.first().expect("the run");
        assert_eq!(first.freshness.is_stale(), Some(false));
        assert!(
            first
                .freshness
                .idle_secs()
                .is_some_and(|s| s < STALE_AFTER_SECS),
            "the idle seconds are MEASURED and travel with the run, so a caller \
             wanting a tighter line than a day does not need this one: {:?}",
            first.freshness
        );

        // A DAY AND AN HOUR LATER. Moving `now` is the only spelling available:
        // nothing in this workspace may name `filetime`, and a test that waited
        // out `STALE_AFTER_SECS` would not be a test.
        let later = now
            .checked_add(std::time::Duration::from_secs(STALE_AFTER_SECS + 3_600))
            .expect("a clock that can be moved a day forward");
        let aged = census_at(root.path(), later);
        assert_eq!(aged.stale, 1, "nothing has written here since yesterday");
        assert_eq!(aged.undated, 0);
        let one = aged.runs.first().expect("the run");
        assert_eq!(one.freshness.is_stale(), Some(true));
        assert!(
            one.freshness
                .idle_secs()
                .is_some_and(|s| s >= STALE_AFTER_SECS),
            "{:?}",
            one.freshness
        );
        assert_eq!(
            one.rows.len(),
            1,
            "and the rows are untouched by any of it -- staleness is a fact \
             about the file, never a filter on what it holds"
        );

        // A CLOCK THAT MOVED. Judged against an instant BEFORE the write, the
        // file claims to have been written in the future -- which is the state
        // `Unknown` exists for, and the state a `false` would have hidden.
        let confused = census_at(root.path(), std::time::SystemTime::UNIX_EPOCH);
        assert_eq!(
            confused.undated, 1,
            "a file that cannot be dated is COUNTED, so `stale: 0` cannot be \
             read as 'and every file was checked'"
        );
        assert_eq!(
            confused.stale, 0,
            "and it is not counted stale on a question nobody answered"
        );
        let none = confused.runs.first().expect("the run");
        assert_eq!(
            none.freshness.is_stale(),
            None,
            "NOT `Some(false)`: that would publish a measurement nobody took as \
             a clean bill of health"
        );
        assert_eq!(none.freshness.idle_secs(), None);
    }

    /// The three answers, and the one that must stay an absence.
    #[test]
    fn freshness_answers_i_do_not_know_rather_than_fine() {
        // A DAY IS NOT AN ARBITRARY NUMBER, and a future edit that tightens it
        // fails here rather than in production. The writer publishes ONCE,
        // immediately before an exit grid that is 87.6% of wall clock, so the
        // threshold has to clear the longest run anyone in this repository has
        // measured -- a 60-minute `audit-range`, ~42 minutes of it grid -- by a
        // margin that makes crossing it a fact rather than a guess about how
        // long a sweep is allowed to take.
        const LONGEST_MEASURED_RUN_SECS: u64 = 60 * 60;
        assert_eq!(Freshness::Touched(60).idle_secs(), Some(60));
        assert_eq!(Freshness::Touched(60).is_stale(), Some(false));
        assert_eq!(
            Freshness::Stale(STALE_AFTER_SECS).idle_secs(),
            Some(STALE_AFTER_SECS)
        );
        assert_eq!(Freshness::Stale(STALE_AFTER_SECS).is_stale(), Some(true));
        assert_eq!(Freshness::Unknown.idle_secs(), None);
        assert_eq!(
            Freshness::Unknown.is_stale(),
            None,
            "the whole point of the variant"
        );
        assert_eq!(
            Freshness::default(),
            Freshness::Unknown,
            "and the DEFAULT answer is 'I do not know', never 'fine' -- a \
             `Census` built by `Default` must not describe healthy runs"
        );

        assert_eq!(STALE_AFTER_SECS, 86_400);
        assert!(
            STALE_AFTER_SECS >= LONGEST_MEASURED_RUN_SECS.saturating_mul(20),
            "a threshold near the length of a real run marks healthy sweeps \
             dead, and a flag an operator learns to ignore is not a flag"
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

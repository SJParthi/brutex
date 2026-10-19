//! The join: a vendor's files on one side, bars on disk on the other, and the
//! counter that says what landed.
//!
//! Every piece this calls already existed and was tested on its own —
//! [`crate::archive`] walks a folder, [`crate::csv`] decodes a row,
//! [`crate::fetch`] filters and converts, [`store::file`] writes,
//! [`crate::manifest`] counts. What did not exist was anything that ran them in
//! order, so nothing could actually be ingested — and then nothing that ran the
//! counter at all, so `/store` reported an em-dash over a store holding 194 bar
//! files.
//!
//! # The census is opened before the first bar is written, and committed once
//!
//! **One load before the member loop, one install after it.** A commit per
//! member would rewrite the whole file twelve thousand times for one folder,
//! and the counter's whole value is that reading it is one operation; paying
//! `O(census)` twelve thousand times to maintain an `O(1)` read is the shape
//! this layer exists to remove.
//!
//! That read is measured, not asserted: `pull::bench::entry_lookup_is_flat`
//! times one lookup at 1×, 10× and 100× the census on the map a loaded manifest
//! holds (`C-12`), and `pull::bench::census_beats_the_scan_it_replaces` times it
//! against re-deriving the same answer from the entries (`C-11`). The append
//! this loop pays instead is `pull::bench::append_after_load_is_flat` (`C-13`).
//!
//! The load happens **first**, before a single bar reaches the disk, because a
//! census this build cannot read is a run whose bars would land uncounted. That
//! is the one outcome worse than a run that refused outright: the bars are
//! real, the census says they are not, and the next run refetches them into a
//! store that correctly refuses the append. So a census that will not open
//! stops the run before it writes, and says which path and which refusal.
//!
//! # A counter row and a bar file describe the same slice, or neither does
//!
//! The [`crate::manifest::EntryKey`] is built from the same `(exchange,
//! segment, symbol, timeframe, month)` the [`StorePath`] is built from, in the
//! same function, from the same values — and the venue is parsed into
//! [`Exchange`] and [`Segment`] **before** the bar file is opened. A plan whose
//! segment is not one the census can key therefore stores nothing for that
//! member rather than storing bars nobody counts.
//!
//! `rows`, `first_ts_micros` and `last_ts_micros` are read back off the bar
//! file's own committed header after the append, not from the batch that was
//! offered. On a second window into a month that already holds bars the batch
//! is a suffix and the header is the whole file, and it is the file the census
//! is a census of.
//!
//! **The month's two closes are the same fact and get the same treatment.**
//! They come from the batch when the batch is provably the whole file — three
//! header comparisons, no read — and are read back off the disk otherwise, two
//! O(1) positional reads. Recording a suffix's first close as the month's first
//! close would put a fabricated base under every percentage computed from it.
//! See `closes_in_hand` and D-0067.
//!
//! # What this refuses to do
//!
//! **It does not decide which bars are wanted.** The window and the cadence
//! arrive from the caller and go straight to [`crate::fetch::land`], so the
//! session rule lives in one place. A second filter here would be a second
//! answer to "is this bar in the session", and the two would disagree the
//! first time either changed.
//!
//! **It does not group rows into a file.** One member becomes one append. A
//! member that spans a month boundary is the caller's problem, because the
//! store addresses per (instrument, timeframe, month) and splitting is a
//! decision about paths rather than about bars.
//!
//! # A member that fails does not stop the run
//!
//! One malformed contract out of twelve thousand should not discard the other
//! eleven thousand nine hundred and ninety-nine. So a member-level failure is
//! **counted and named** in [`Ingested::failures`] rather than returned, and
//! the run continues. That is the opposite of [`crate::archive::read_dir`]'s
//! rule, deliberately: a *decode* failure means the shape is wrong and every
//! member shares the shape, whereas a *write* failure is usually about one
//! path. The run reports both totals so neither hides in the other.
//!
//! **A run-level refusal is a failure too, and not an `Err`.** The error type
//! here belongs to [`crate::archive`] and every variant of it is about a
//! folder; a census that will not open is not about the folder, and dressing
//! it as one would name the wrong layer. Such a refusal is one entry in
//! [`Ingested::failures`] naming the census path, with nothing stored — loud,
//! and `Ingested::balances` is false for it.
//!
//! # Cost
//!
//! One pass per member, one append per member, one hash probe and one 128-byte
//! entry per member that stored anything. The append is
//! `base + header + index·stride` — arithmetic, not a search. Enumerating the
//! folder is O(members), which is inherent to a bulk import and is stated in
//! [`crate::archive`] rather than dressed up.
//!
//! The census install is `O(entries)` **once per run**: it re-images every
//! committed entry, including the ones this run did not touch. That is the
//! documented cost of [`Manifest::image`], taken deliberately in exchange for
//! an install that is one `rename`. The incremental alternative —
//! [`Manifest::record`]'s [`crate::manifest::Append`], one 128-byte positional
//! write per member and one commit — is what a per-member publish would use,
//! and this deliberately does not publish per member.
//!
//! **"Once per run" is true of [`from_dir`] and false of [`from_window`].** A
//! folder is one run over twelve thousand members and one install. A broker
//! window is one run over ONE member — [`from_window`] hands a single-element
//! slice to [`from_members`] — so the whole image is rewritten, `fsync`ed and
//! renamed for every window fetched. The batch bargain above is being paid at
//! per-member frequency, which is exactly the shape the last paragraph says
//! this module does not do.
//!
//! It costs nothing today: the broker route refuses every target but `Swept`,
//! and no window returns at all until `pull::vendor::HttpSpec` carries a
//! request-parameter map. It is measured and recorded against the scope that
//! makes it bite — `docs/06-limits.md` §34, **424 GB rewritten to maintain a
//! 5.75 MB file** across the operator's stated NIFTY option backfill. The fix
//! is the incremental path named two paragraphs up, and it belongs with the
//! work that first makes that backfill runnable, because a positional
//! installer that cannot be exercised under load is a durability change nobody
//! has watched fail.
//!
//! A run that changed nothing installs nothing, so a re-run leaves the census
//! byte for byte as it was. `CLAUDE.md` §3 rule 5 covers the counter as well as
//! the bars.
//!
//! **UNVERIFIED as a measurement.** The bound is argued from the
//! shape of the code and no bench in this workspace times it.
//! `CLAUDE.md` §3 rule 6: a structural argument is not a
//! measurement, however sound it is.

use std::fs;
/// The census on disk: single file or per-segment epochs, compaction into the
/// next epoch, migration and recount. D-3316 (WP-10).
#[path = "census_store.rs"]
pub mod census_store;
/// The gap ledger for months already stored, from the store and the calendar
/// with no vendor call (R3-18).
#[path = "gap_backfill.rs"]
pub mod gap_backfill;
#[path = "identity_screen.rs"]
pub mod identity_screen;
/// What a versioned repair of a series' months would do, read-only: the one
/// answer `/pull/repair.json` and `cli zerodha-repair` give. D-3350, WP-13c.
#[path = "repair_plan.rs"]
pub mod repair_plan;
#[path = "request_minutes.rs"]
mod request_minutes;
use std::io::{ErrorKind, Write as _};
use std::path::{Path, PathBuf};

use brutex_core::instrument::{Exchange, Segment};
use brutex_core::vendor::Vendor;
use store::file::BarFile;
use store::flock::Flock;
use store::format::Bar;
use store::header::Header;
use store::path::{FileKind, PathParts, StorePath, Timeframe};

use crate::archive::{self, Member};
use crate::csv::Columns;
use crate::fetch::{self, BarRequest};
use crate::manifest::{
    Append, Closes, ENTRY_STRIDE, Entry, EntryKey, HEADER_LEN, Held, MAX_ENTRIES, Manifest,
};
use crate::session::DropCensus;
use crate::vendor::{PriceScale, TimestampEncoding};

/// The largest census this reader will hold in memory.
///
/// Derived, not chosen: `HEADER_LEN + MAX_ENTRIES × ENTRY_STRIDE` is the
/// largest file [`Manifest::image`] can produce, because
/// `ManifestHeader::advance` refuses a counter past
/// [`MAX_ENTRIES`] and every entry occupies exactly [`ENTRY_STRIDE`] bytes. A
/// file larger than that is not a census this build could have written, and
/// reading it to find that out is an allocation the size of whatever somebody
/// left at that path — which is not a bound, it is the absence of one.
///
/// **It is the WIDEST stride this build knows, and that is deliberate.** A
/// version-1 census at the ceiling is 134,250,496 bytes and must still be
/// readable; a version-2 one is twice that. Bounding at version 1's stride
/// would refuse a legal version-2 file, and bounding per-version is impossible
/// here because the version is inside the file this bound decides whether to
/// read. D-0067.
const MAX_CENSUS_BYTES: u64 = HEADER_LEN + MAX_ENTRIES * ENTRY_STRIDE;

/// The derivation above, checked at compile time rather than in a comment.
const _: () = assert!(MAX_CENSUS_BYTES == 268_468_224);

/// What one member could not do, kept so the run can continue past it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure {
    /// Which instrument.
    ///
    /// Or, for the two refusals that are the **run's** rather than one
    /// member's — a census that will not open, and one that will not install —
    /// the census path, because that is the thing an operator has to go and
    /// look at. Both are named in the same list so a caller cannot count
    /// member failures and believe it has counted every failure.
    pub instrument: String,
    /// Why, in its own words.
    pub why: String,
    /// The month this failure is about, when it is about one. D-3341, DB-G45.
    ///
    /// **Carried, never derived from the request.** The pull's audit lines
    /// were all filed under `month=2018-01` — the first month of an 8.75-year
    /// window — because the caller only had the window to name: 48,396 "member
    /// did not land" lines, among them a gap on 2019-12-02, a Muhurat refusal
    /// on 2020-11-14 and a derived-rung failure for 2026-09. The failure knows
    /// which month it concerns; this is where it says so. [`None`] for a
    /// failure that is not about one month — a census, a whole member whose
    /// rows span several.
    pub month: Option<store::path::YearMonth>,
    /// The day this failure is about, when it is about one. D-3341, DB-G45.
    pub day: Option<crate::session::Day>,
}

impl Failure {
    /// A failure about no particular month or day.
    #[must_use]
    pub fn new(instrument: impl Into<String>, why: impl Into<String>) -> Self {
        Self {
            instrument: instrument.into(),
            why: why.into(),
            month: None,
            day: None,
        }
    }

    /// This failure, about `month`.
    #[must_use]
    pub fn in_month(self, month: store::path::YearMonth) -> Self {
        Self {
            month: Some(month),
            ..self
        }
    }

    /// This failure, about `day` — and so about the month `day` is in.
    #[must_use]
    pub fn on_day(self, day: crate::session::Day) -> Self {
        Self {
            month: day.year_month().ok().or(self.month),
            day: Some(day),
            ..self
        }
    }
}

/// What a run put on disk, and what it did not.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Ingested {
    /// Members read from the folder.
    pub members: usize,
    /// Rows the vendor's files held, before any filtering.
    pub rows_read: usize,
    /// Bars OFFERED to the store — not necessarily written.
    ///
    /// # The distinction, and why the name kept the wrong one
    ///
    /// This was documented as "bars written" and fed from the post-fold batch
    /// length. `BarFile::append` answers `Committed` or `AlreadyPresent`, and
    /// `write_and_count` discarded the discriminant — so re-running a pull the
    /// store already held reported *"Rows read 375, Bars stored 375"* with zero
    /// bytes written and the census untouched. `balances()` was true, the
    /// journal said `Stored`, and nothing had been.
    ///
    /// The arithmetic is not the bug: `balances()` genuinely needs the OFFERED
    /// count, because it reconciles rows read against rows accounted for. What
    /// was wrong is that the number an operator reads to decide whether a
    /// backfill is progressing was the wrong one. So this keeps its meaning and
    /// says so, and [`Self::bars_committed`] carries the other.
    pub bars_stored: usize,
    /// The census row this run produced and did NOT write.
    ///
    /// `None` once it has been recorded, or when nothing was filed. A caller
    /// filing several contracts from one vendor answer collects these and calls
    /// [`record_held`] once — see `record_all`'s cost note for what paying it
    /// per contract cost.
    pub pending: Option<crate::manifest::Held>,
    /// Bars this run actually WROTE — `Appended::Committed` only.
    ///
    /// Zero on a re-run of a month already held, which is the number that makes
    /// "nothing happened" distinguishable from "nothing was there". See
    /// [`Self::bars_stored`] for why both exist.
    pub bars_committed: usize,
    /// Month files written for a rung NOBODY ASKED FOR, folded from the minute
    /// bars this run landed.
    ///
    /// A one-minute pull writes eight files per member: the minute it was asked
    /// for and the seven coarser rungs derived from it. This counts the seven,
    /// so a receipt can say what a run built as well as what it fetched — and
    /// so `bars_stored` is never mistaken for the number of bars a vendor sent.
    ///
    /// Zero on every rung that is not the minute, because nothing is derived
    /// from a day and nothing is derived from a second.
    pub derived_files: usize,
    /// Rows folded into a bar that was already open.
    ///
    /// **Consumed, not discarded, and not bars.** Both archive vendors ship
    /// two to four snapshots per second with no sub-second field, so a
    /// one-minute bucket swallows up to 240 rows and emits one bar. Every one
    /// of those rows is *in* the bar — its volume is summed into it, its high
    /// and low widen it, the last one is its close — so counting them as drops
    /// would say they were declined, and leaving them uncounted was what made
    /// [`Ingested::balances`] answer NO on a run where nothing was lost:
    /// 354,675 rows read, 62,978 bars stored, 170 dropped, and 291,527 rows
    /// with nowhere to be.
    ///
    /// It is a **measured** count taken at the fold — the length before minus
    /// the length after — and not `rows_read - bars_stored - drops`. Derived
    /// from the other three it would balance by construction and
    /// [`Ingested::balances`] would be a tautology rather than a check.
    pub rows_folded: usize,
    /// Members whose slice the census holds after this run.
    ///
    /// Recorded by this run, or already recorded with exactly these numbers.
    /// A member that stored bars is either in this count or named in
    /// [`Ingested::failures`]; there is no third place for it to be.
    pub counted: usize,
    /// Every row that did not become a bar, by reason.
    pub census: DropCensus,
    /// Members that failed, named. The run continued past each.
    pub failures: Vec<Failure>,
}

impl Ingested {
    /// Add another run's counters to this one.
    ///
    /// A window wider than the vendor's per-request cap is several requests and
    /// several answers, each landed by its own [`from_window`] call. The receipt
    /// must report the WHOLE run rather than merely its last chunk, so the
    /// counters are summed and the failures concatenated.
    ///
    /// `members` is summed like the rest: one body is one member, so eighty-one
    /// chunks are eighty-one members, which is what the page should say.
    pub fn absorb(&mut self, other: Self) {
        self.members += other.members;
        self.rows_read += other.rows_read;
        self.bars_stored += other.bars_stored;
        self.bars_committed += other.bars_committed;
        // A FIELD ADDED TO THE RECEIPT AND NOT TO THIS LINE IS A COUNT THAT
        // SILENTLY RESETS on every window after the first — the whole reason
        // `absorb` exists is that a run is many calls and the receipt is one.
        self.derived_files += other.derived_files;
        self.rows_folded += other.rows_folded;
        self.counted += other.counted;
        self.census.absorb(other.census);
        self.failures.extend(other.failures);
    }

    /// Whether every row is accounted for: stored, folded into a bar that was
    /// already open, dropped, or in a member that failed.
    ///
    /// A row that vanished without landing in one of those four is
    /// indistinguishable from a row the vendor never sent, which is the
    /// failure this whole pipeline is shaped to prevent.
    #[must_use]
    pub fn balances(&self) -> bool {
        self.failures.is_empty()
            && self.rows_read == self.bars_stored + self.rows_folded + self.census.total() as usize
    }
}

/// Everything one run needs, as one value.
///
/// A struct rather than ten positional arguments, and not only to satisfy a
/// lint: THREE OF THEM ARE `&str` AND ADJACENT. `exchange` and `segment`
/// would transpose without a compiler complaint and the bars would land under
/// a path that looks plausible and is wrong. The same reasoning produced
/// `api::render::View` earlier in this codebase, for the same reason.
#[derive(Debug, Clone, Copy)]
pub struct Plan<'a> {
    /// Runtime schedule authority, shared by request audit and derivation.
    pub calendar: crate::calendar::Runtime<'a>,
    /// Per-security dated continuous close; never inferred from today's basket.
    pub cash_schedule: Option<&'a crate::cash_auction::Schedule>,
    /// Which column layout the files carry.
    pub columns: Columns,
    /// The window and the rung, passed straight to the filter.
    ///
    /// **The rung on it is also the timeframe these bars are filed under**, via
    /// [`crate::vendor::Granularity::store_timeframe`] — see [`Plan::timeframe`].
    /// This used to be two fields, a `granularity` here and a `timeframe`
    /// beside it, and nothing made them agree: a plan naming `Day1` and
    /// `MINUTE_1` exempts every bar from the session filter and then files the
    /// result under `1min/`, which is a directory of daily bars no reader can
    /// tell from minute bars.
    pub request: &'a BarRequest,
    /// How the vendor encodes its timestamps.
    pub encoding: TimestampEncoding,
    /// Whether prices arrive in rupees or paisa.
    pub scale: PriceScale,
    /// Which vendor prefix to write beneath.
    pub vendor: Vendor,
    /// The exchange segment of the path.
    ///
    /// Also parsed into an [`Exchange`] for the census key, so `NSE` and `BSE`
    /// are the only two this build stores under — a directory the census
    /// cannot name is a directory `/store` can never report.
    pub exchange: &'a str,
    /// The instrument segment of the path.
    ///
    /// Also parsed into a [`Segment`], so `INDEX`, `CASH` and `FNO` are the
    /// only three this build stores under, for the same reason.
    pub segment: &'a str,
    /// The contract these bars belong to, or `None` for spot.
    ///
    /// # Why the caller carries it rather than this deriving it
    ///
    /// Only the caller knows which contract it asked the broker for. The
    /// vendor's answer is a list of bars; nothing in it names the series, and
    /// deriving one from the shape of the request would be inventing an
    /// identity rather than reading one.
    ///
    /// `None` is spot, and it is what makes the path one level shallower —
    /// `store::path` branches on exactly this absence. A contract that failed
    /// to arrive here would file an option's bars in the underlying's own
    /// directory, where the next month's spot pull would append to them.
    pub contract: Option<brutex_core::instrument::Contract>,
}

impl Plan<'_> {
    /// THE WRITE BOUNDARY. The directory these bars are filed under, or the
    /// reason there is none.
    ///
    /// One conversion site, and it is here rather than in a caller because
    /// here is where a wrong answer becomes bytes on a disk. A rung
    /// `crates/store` has not been widened to carry has no directory, and
    /// `CLAUDE.md` §4 leaves exactly two options: refuse, or substitute. A
    /// substitution would file five-minute bars under `1min/` and no later
    /// reader could tell them from real one-minute data — the store carries the
    /// bar length in the PATH, not in the row.
    ///
    /// # Errors
    ///
    /// The rung, named, when it has no store timeframe.
    fn timeframe(&self) -> Result<Timeframe, String> {
        self.request.granularity.store_timeframe().ok_or_else(|| {
            format!(
                "{} has no directory in this store, so there is nowhere to file \
                 a bar of that length. Nothing was written rather than filing \
                 it under a rung it is not: the bar length lives in the PATH \
                 here, so a substituted bar is one no later reader can tell \
                 from a real one. crates/store ships {}.",
                self.request.granularity,
                Timeframe::KNOWN
                    .iter()
                    .map(|tf| tf.as_str())
                    .collect::<Vec<_>>()
                    .join(" and "),
            )
        })
    }
}

/// Reads every CSV in `dir`, writes the bars that survive into the store, and
/// publishes one census of what landed.
///
/// # Errors
///
/// Whatever [`archive::read_dir`] refuses — a missing folder, a wrong column
/// shape, a member that is not text. Those are **run-level**: the folder or the
/// declared shape is wrong, and every member shares both.
///
/// A failure on **one** member is counted in [`Ingested::failures`] and the run
/// continues. One malformed contract out of twelve thousand must not discard
/// the other eleven thousand nine hundred and ninety-nine. A census that will
/// not open or will not install is counted there too, named by its path — see
/// the module header for why it is not an `Err`.
pub fn from_dir(
    dir: &Path,
    store_root: &Path,
    plan: Plan<'_>,
) -> Result<Ingested, archive::ArchiveError> {
    // Only the column layout and the vendor are needed here; `one`
    // destructures the rest.
    let Plan { columns, .. } = plan;
    let members = archive::read_dir(dir, columns)?;
    Ok(from_members(&members, store_root, plan))
}

/// Every member, through the census, the store and the counter file.
///
/// # Why this is its own function
///
/// It was the back half of [`from_dir`], reachable only by naming a directory.
/// The broker path has no directory — [`crate::http::HttpSource`] answers with
/// a [`crate::fetch::RawWindow`] in memory — and it needs every line of this:
/// the census read before any bar is written, the degraded-census note, the
/// per-member land/fold/append, the folded drop counts, the counter row, the
/// one install after the loop, and each of the failures named along the way.
///
/// Copying it for the second caller would have been two implementations of
/// "what an ingest does", and the second one would have been wrong first. So
/// both callers build `Member`s and hand them here, and a bar pulled from a
/// broker takes byte-for-byte the same path to disk as one read from a folder.
///
/// THE TWO VERBS ARE DIFFERENT AND THIS FUNCTION IS THE ONE PLACE THEY MEET.
/// A broker's bars are PULLED — a request, a token, a quota, a floor. A folder
/// vendor's are READ — no request, no token, no quota, and a reach that is
/// whatever files the operator bought (`crate::folder`). Calling both a pull
/// sent every folder failure to the wrong question first, so the word each
/// caller uses comes from `crate::vendor::SourceKind::verb` and never from a
/// literal. What is shared is this function, which is downstream of both.
///
/// See [`from_window`] for the broker's side of that, and [`from_dir`] for the
/// folder's.
#[must_use]
pub fn from_members(members: &[Member], store_root: &Path, plan: Plan<'_>) -> Ingested {
    from_members_mode(members, store_root, plan, Mode::Append).0
}

/// [`from_members`] in either mode, with what the landing refused after the
/// vendor sent it — for the gap ledger, which audits the vendor's rows and
/// must still count a refused minute as missing from the store (D-3351).
fn from_members_mode(
    members: &[Member],
    store_root: &Path,
    plan: Plan<'_>,
    mode: Mode<'_>,
) -> (Ingested, request_minutes::Refusals) {
    let (done, refusals) = from_members_inner(members, store_root, plan, mode);
    note_run(&done, members, &plan);
    (done, refusals)
}

/// One line per completed run, at **`Info`** — the line this module's header
/// promised and nothing wrote.
///
/// # The defect this closes, measured
///
/// The header says *"`Info` carries the run; `Debug` carries the members, for
/// the one run an operator is actually diagnosing."* The `Debug` half was built.
/// **The `Info` half never existed.** Every emit on this path was `debug`,
/// `warn` or `error`, so a run that SUCCEEDED emitted nothing above the default
/// floor of `Level::Info`.
///
/// Measured on the operator's store, 2026-08-22: a pull of **1,871,491 rows
/// into 1,870,591 bars** across three instruments and nine rungs left
/// `logs/events.ndjson` at **0 bytes**. The audit journal recorded 502 records
/// and the telemetry log recorded nothing, so `/logs` covered a completed
/// backfill with a blank page and every question about it had to be answered by
/// decoding fixed-stride records by hand.
///
/// A silence that only breaks on failure is the worst possible reporting
/// contract: it is indistinguishable from a run that never happened, and
/// `CLAUDE.md` §4 asks for the opposite — say what happened, loudly, and name
/// the reason.
///
/// # Why one event and not one per member
///
/// The members already emit at `Debug`, which is where they belong: 800
/// instruments × 80 windows is 64,000 lines an operator does not want by
/// default and does want when diagnosing one run. This is the granularity
/// `CLAUDE.md` §5 calls affordable — one event per run — and it is emitted from
/// the single public entry point every run passes through, so a caller cannot
/// take a path that skips it.
fn note_run(done: &Ingested, members: &[Member], plan: &Plan<'_>) {
    // WHAT THIS EVENT IS ABOUT, WHICH IT DID NOT SAY.
    //
    // Measured on the operator's store, 2026-08-28: **884 `ingested` events in
    // one log**, each carrying `members / rows / bars / committed / folded /
    // dropped / failures` and **not one naming its instrument, month or rung**.
    // So the log recorded that something was ingested 884 times and could not
    // answer "which month came up short" — the question the counters exist for.
    //
    // A number nobody can attribute is not an audit trail. `CLAUDE.md` §4 asks
    // for the reason to be NAMED, and a count with no subject names nothing.
    //
    // THE NAME COMES FROM THE PLAN, AND THE FIRST DRAFT TOOK IT FROM THE WRONG
    // PLACE. That draft read `Ingested::pending`, whose `Entry::key` is exactly
    // the tuple wanted — and `pending` is set in `from_rows` alone, the rolling
    // contract path. **Every one of the 884 measured events comes through
    // `from_members`**, where it is `None` from first line to last, so the
    // naming was a no-op on the only path that emits. `emit_sites` refused it,
    // which is the whole reason that table exists.
    //
    // The plan carries the run's constants unconditionally, so they are named
    // unconditionally. The cost is the `with` calls below and one `YearMonth`
    // render — O(1) per RUN, not per bar, which is the granularity gate 17's
    // own comment prescribes as the affordable one.
    //
    // THE INSTRUMENT IS NAMED ONLY WHEN THERE IS ONE. A broker pull is always
    // `from_members(slice::from_ref(&member), ..)` — one member, one subject —
    // while a folder walk can carry many, and no single symbol is true of that
    // run. Picking the first would be a name that reads exactly like a fact and
    // is not one, which is the `CLAUDE.md` §4 failure-in-a-success's-clothes
    // shape. The per-member `pull.member` lines name those individually.
    //
    // THE STRINGS OUTLIVE THE EVENT, DELIBERATELY. `Value::Str` borrows, so
    // both rendered values are bound here, ahead of the builder, rather than
    // inline where each would be dropped at the end of its own statement.
    let span = plan.request.window;
    let month = match (span.from().year_month(), span.to().year_month()) {
        // ONE KEY, NOT TWO, AND A SPAN IS A SUBSTRING OF ITS OWN FIRST MONTH.
        // The log's field match is substring, so `month=2022-10` still finds a
        // run that spanned into November — which a separate `to_month` key
        // would have hidden from exactly the query an operator types.
        (Ok(first), Ok(last)) if first == last => first.to_string(),
        (Ok(first), Ok(last)) => format!("{first}..{last}"),
        // A day outside the store's month range has no month to name. Saying so
        // beats naming a month the run did not touch.
        _ => String::new(),
    };
    let rung = plan.timeframe().map_or("", Timeframe::as_str);
    let event = telemetry::Event::info("pull.run", "ingested")
        .with("vendor", telemetry::Value::Str(plan.vendor.as_str()))
        .with("exchange", telemetry::Value::Str(plan.exchange))
        .with("segment", telemetry::Value::Str(plan.segment))
        .with("rung", telemetry::Value::Str(rung))
        .with("month", telemetry::Value::Str(&month));
    let event = match members {
        [only] => event.with(
            "instrument",
            telemetry::Value::Str(only.instrument.as_str()),
        ),
        _ => event,
    };
    let _dropped_when_filtered = telemetry::emit(
        &event
            .with("members", telemetry::Value::Uint(done.members as u64))
            .with("rows", telemetry::Value::Uint(done.rows_read as u64))
            .with("bars", telemetry::Value::Uint(done.bars_stored as u64))
            .with(
                "committed",
                telemetry::Value::Uint(done.bars_committed as u64),
            )
            .with("folded", telemetry::Value::Uint(done.rows_folded as u64))
            // THE BOOKS, ON THE LINE ITSELF. `rows - bars - folded` is what
            // `Ingested::balances` reconciles, and an operator reading the log
            // should not have to subtract to learn a run dropped something.
            .with(
                "dropped",
                telemetry::Value::Uint(
                    (done.rows_read.saturating_sub(done.bars_stored))
                        .saturating_sub(done.rows_folded) as u64,
                ),
            )
            .with(
                "failures",
                telemetry::Value::Uint(done.failures.len() as u64),
            ),
    );
}

/// The fold ratio, once per member — never per snapshot.
///
/// **This is the number that would have named D-0070 on the day it landed.** A
/// daily pull folding N rows into N bars means the vendor already sent one bar
/// per day and the bucket is only ever re-stamping them, so `folded = 0` on a
/// `1day` rung is the signature of the whole defect. Nothing wrote it down, and
/// it took decoding bar files by hand to see it.
///
/// `bucket_secs` travels with it because the bucket WIDTH is the other half:
/// 86,400 against an IST offset of 19,800 is the misalignment itself.
fn note_fold(
    member: &Member,
    snapshots: usize,
    bars: usize,
    folded: usize,
    bucket: crate::fold::Bucket,
) {
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::debug("pull.fold", "folded")
            .with("instrument", telemetry::Value::Str(&member.instrument))
            .with("snapshots", telemetry::Value::Uint(snapshots as u64))
            .with("bars", telemetry::Value::Uint(bars as u64))
            .with("folded", telemetry::Value::Uint(folded as u64))
            .with(
                "bucket_secs",
                telemetry::Value::Uint(u64::from(bucket.secs())),
            ),
    );
}

/// One member landed, on the rolling log.
///
/// # Why the result is discarded, and why that is not the swallow §4 bans
///
/// [`telemetry::emit`] returns whether the event reached a file, and at the
/// call sites in `crates/api` that answer is asserted, because those events are
/// `Error` and an `Error` that cannot be written is the defect the event exists
/// to remove. This one is `Debug`. A `Debug` event is *supposed* to be dropped
/// on a normal run — the sink's `min_level` is `Info` unless the operator
/// raises it — so asserting it was written would fire on every clean pull.
///
/// # Why `Debug` and not `Info`
///
/// The sink keeps [`telemetry::DEFAULT_KEEP_FILES`] files of
/// [`telemetry::DEFAULT_MAX_FILE_BYTES`], a 64 MiB window. A one-minute
/// backfill is ~62,600 members, so writing one line each at `Info` would roll
/// the run's own first hour out of the window before the run finished — the
/// evidence would destroy itself. `Info` carries the run; `Debug` carries the
/// members, for the one run an operator is actually diagnosing.
fn note_landed(member: &Member, landed: &Landed) {
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::debug("pull.member", "landed")
            .with("instrument", telemetry::Value::Str(&member.instrument))
            .with("bars", telemetry::Value::Uint(landed.bars as u64))
            .with("folded", telemetry::Value::Uint(landed.folded as u64))
            .with("rows", telemetry::Value::Uint(member.rows.len() as u64))
            .with(
                "dropped",
                telemetry::Value::Uint(u64::from(landed.census.total())),
            )
            // A SUBSET OF `dropped`, NOT AN ADDEND TO IT. `dropped` says how
            // many rows were declined; this says how many of those the venue's
            // published hours declined. A reader who sums them double-counts.
            // It earns its slot because `dropped` alone cannot distinguish a
            // vendor sending rows past the close from one sending rows outside
            // the requested window, and a jump here means `crate::vendor`'s
            // session table has gone stale.
            .with(
                "outside_session",
                telemetry::Value::Uint(u64::from(landed.outside_session)),
            ),
    );
}

/// One member that did not land, on the rolling log — **at `Error`**.
///
/// This is the only surface that gets every reason. The audit journal keeps
/// `failures.first()` in a fixed 68-byte note, and the run's receipt shows
/// five; neither is the full set. Before this existed, a run that refused ten
/// members wrote nothing at all and the cause was recovered by decoding bar
/// files by hand.
fn note_not_landed(member: &Member, why: &str) {
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::error("pull.member", "did not land")
            .with("instrument", telemetry::Value::Str(&member.instrument))
            .with("rows", telemetry::Value::Uint(member.rows.len() as u64))
            .with("why", telemetry::Value::Str(why)),
    );
}

/// Records one instrument-level filing failure in the log, beside the receipt.
///
/// # The three that reached the receipt and never reached the log
///
/// `file_one` can give up at three points, and each pushed a `Failure` and
/// returned without emitting anything: the address could not be resolved, the
/// bar write failed, or the overlay write failed. All three reach the receipt,
/// which is read by whoever is watching the run. None reached `/logs`, which is
/// what gets handed to a diagnosis afterwards — so an operator arriving later
/// saw a quiet file for a run that had failed. Gate 19 exists for exactly that
/// gap, and named these three.
///
/// # Why `stage` is a parameter and not three helpers
///
/// The three differ only in WHERE they gave up, and that difference is the
/// whole diagnostic value: an address that will not resolve is a listing
/// problem, a bar write is a disk or a layout problem, and an overlay write is
/// a sidecar whose bars did land. One field carries it, and one helper means
/// the event shape cannot drift between the three.
///
/// At `error` rather than `warn`, for the reason [`note_derived_shortfall`]
/// gives: the run REPORTS these as failures, so a line below the default floor
/// would be the same silence in a different place.
pub(super) fn note_not_filed(instrument: &str, stage: &str, why: &str) {
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::error("pull.file", "not filed")
            .with("instrument", telemetry::Value::Str(instrument))
            .with("stage", telemetry::Value::Str(stage))
            .with("why", telemetry::Value::Str(why)),
    );
}

/// Records one derived-rung shortfall in the log, beside the receipt.
///
/// # Why both surfaces and not one
///
/// The `Failure` this accompanies reaches the receipt, and a receipt is read by
/// whoever is watching the run. `/logs` is what gets handed to a diagnosis
/// afterwards, and gate 19 exists because a failure visible on one and absent
/// from the other reads as a quiet file to the operator who arrives later.
///
/// At `error` rather than `warn`: this is a shortfall the run REPORTS as a
/// failure, so a log line that sat below the default floor would be the same
/// silence in a different place.
fn note_derived_shortfall(member: &Member, short: &Failure) {
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::error("pull.derived", "a folded rung did not land")
            .with("instrument", telemetry::Value::Str(&member.instrument))
            .with("why", telemetry::Value::Str(&short.why)),
    );
}

/// A member whose folded rungs did not all land, or [`None`] when they did.
///
/// # The failure this exists to stop being silent
///
/// `derive` returning `Err` once fed a telemetry line and nothing else — no
/// entry in `failures` at all. So a disk that filled, or a
/// permission lost, AFTER the pulled rung landed but before the folded ones did
/// produced a run that reported SUCCESS: `Outcome::Stored` journalled, the
/// receipt reading "every row accounted for", and seven of eight timeframes
/// simply absent. The books balanced because nothing had asked them this.
///
/// # The expected count is a rule, never a list
///
/// [`derived_count_in`] is what `derive_rungs` itself walks, so a rung added to
/// `Timeframe::KNOWN` moves both sides at once and this cannot go stale against
/// it. It already knows the one case where zero is correct: an option contract
/// folds into no rung at all.
///
/// # Cost
///
/// Two integers compared. O(1) per member, and it opens nothing: both counts
/// were established while the member was being written.
///
/// **UNVERIFIED as a measurement.** The bound is argued from the
/// shape of the code and no bench in this workspace times it.
/// `CLAUDE.md` §3 rule 6: a structural argument is not a
/// measurement, however sound it is.
fn derived_shortfall(member: &Member, landed: &Landed) -> Option<Failure> {
    if landed.derived == landed.derived_expected {
        return None;
    }
    let failure = Failure::new(
        member.instrument.clone(),
        format!(
            "{} of {} derived rung(s) landed. The pulled rung is on disk and the \
             folded ones are not, so this instrument-month reads as held at one \
             timeframe and missing at the rest. Why each rung failed is on the \
             log at `pull.derive`; this is the count that stops the run \
             reporting itself clean.",
            landed.derived, landed.derived_expected
        ),
    );
    // THE MONTH THE SHORTFALL IS IN, when the member wrote one (DB-G45).
    Some(match one_month(&landed.entries) {
        Some(month) => failure.in_month(month),
        None => failure,
    })
}

/// The one month every row in `entries` is about, or [`None`] when they span
/// several or there are none. O(entries), which is a member's rungs.
fn one_month(entries: &[Held]) -> Option<store::path::YearMonth> {
    let first = entries.first()?.entry.key.month;
    entries
        .iter()
        .all(|held| held.entry.key.month == first)
        .then_some(first)
}

fn note_not_derived(instrument: &str, rung: Timeframe, why: &str) {
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::warn("pull.derive", "rung not derived")
            .with("instrument", telemetry::Value::Str(instrument))
            .with("timeframe", telemetry::Value::Str(rung.as_str()))
            .with("why", telemetry::Value::Str(why)),
    );
}

/// [`from_members`]'s body, split only so the two `note_*` helpers can sit
/// beside it rather than between its documentation and its signature.
#[expect(
    clippy::too_many_lines,
    reason = "the census cycle — locate, lock, read, record per member, install — \
              is one read-modify-write under one lock, and splitting it would \
              hand the lock across a function boundary the type does not track"
)]
fn from_members_inner(
    members: &[Member],
    store_root: &Path,
    plan: Plan<'_>,
    mode: Mode<'_>,
) -> (Ingested, request_minutes::Refusals) {
    let Plan { vendor, .. } = plan;
    let mut done = Ingested {
        members: members.len(),
        ..Ingested::default()
    };
    let mut refusals = request_minutes::Refusals::default();

    // THE CENSUS FIRST, BEFORE ANY BAR IS WRITTEN. A bar that cannot be
    // counted must not be stored: the store would then hold rows the census
    // denies, a later run would refetch them, and the append would be refused
    // for bars that are already there.
    let census_path = crate::manifest::manifest_path(store_root, vendor);

    // THE RUNG IS RESOLVED ONCE, AND BEFORE THE LOCK IS TAKEN.
    //
    // `one` resolves it too — it is the function that opens the file — but
    // every member of a run shares the plan, so a rung the store cannot carry
    // is a RUN-level refusal wearing a per-member costume. Reported once here
    // rather than eight hundred identical times, and before anything is
    // locked, read or written.
    if let Err(why) = plan.timeframe() {
        return (refused_whole(members, &census_path, why), refusals);
    }

    // WHICH CENSUS THIS RUN'S ROWS GO TO (D-3316): the vendor's single file
    // until it is migrated, then the partition of the plan's segment — one run
    // is one plan and one plan is one segment, so a Zerodha cash pull never
    // reads, locks or rewrites another segment's census, let alone GDFL's.
    let target = match census_store::target_for_plan(store_root, vendor, plan.segment) {
        Ok(target) => target,
        Err(why) => return (refused_whole(members, &census_path, why), refusals),
    };
    let census_path = target.named();

    // THE LOCK, AND IT IS TAKEN BEFORE THE READ.
    //
    // The census cycle is read-whole-file, mutate in memory, write-whole-file.
    // Locking only the write serialises the two renames and leaves the two
    // reads racing: run A reads 20 entries, run B reads the same 20, A installs
    // 20+A, B installs 20+B on top, and A's entries are gone without either run
    // ever finding the lock contended. Holding it from here to the install is
    // what makes the read-modify-write atomic, which is the property the
    // incident in `CensusLock`'s docs actually needed.
    //
    // Declared before `census` so it drops after it — the lock outlives every
    // use of the thing it protects.
    let census_lock = match target.lock() {
        Ok(guard) => guard,
        Err(why) => return (refused_whole(members, &census_path, why), refusals),
    };
    if let Err(why) = target.still_where_located() {
        return (refused_whole(members, &census_path, why), refusals);
    }

    // Under the lock, so a partition's current epoch cannot move under it.
    let mut loaded = match target.load() {
        Ok(loaded) => loaded,
        Err(why) => return (refused_whole(members, &census_path, why), refusals),
    };
    // Set when a partition at its capacity is compacted mid-run (D-3316 item
    // 5): the appends below are then relative to an epoch not yet written,
    // and the install publishes the next epoch whole instead.
    let mut compacted = false;

    // A census that loaded degraded may still be appended to — recovering from
    // a torn commit *is* writing the next generation — but never silently.
    // What it recovered is a prefix: months a newer generation had committed
    // are not in it and cannot be got back from it, only from the bar files.
    let repairing = loaded.census.degraded_reason().is_some();
    if let Some(why) = loaded.census.degraded_reason() {
        note_census_degraded(&census_path, &why.to_string());
        done.failures.push(Failure::new(
            census_path.display().to_string(),
            format!(
                "the census loaded DEGRADED at generation {} after stepping \
                 over: {why}. This run's install is the repair, and any month \
                 a newer generation had committed is not in it — that is \
                 recoverable only by walking bars/ (`cli census-recount`, D-3316)",
                loaded.census.header().generation
            ),
        ));
    }
    // A REPAIR still publishes the whole image, because it may rewrite entries
    // the append region already holds and an append-only log cannot say that.
    let publish = repairing;
    // EVERY WRITE THIS RUN OWES THE CENSUS, IN ORDER.
    //
    // One 64-byte entry image and one header commit each, both already computed
    // by `Manifest::record`. Collecting them is what lets the install publish
    // what CHANGED rather than re-imaging what did not — see `install_census`.
    let mut appends: Vec<Append> = Vec::new();

    for member in members {
        done.rows_read += member.rows.len();
        match one(member, store_root, plan, mode) {
            Ok(mut landed) => {
                // WHAT THE LANDING REFUSED AFTER THE VENDOR SENT IT, for the
                // gap ledger (D-3351): each source a list in time order.
                refusals.seconds.append(&mut landed.refused_seconds);
                refusals.fronts.append(&mut landed.fronts);
                refusals.repaired.append(&mut landed.repaired_days);
                // ONE EVENT PER MEMBER, WHICH IS THE GRANULARITY THAT WAS
                // MISSING. Per-row would be millions on a one-minute backfill
                // and would roll the run's own beginning out of an 8 MB x 8
                // file window before it finished; per-run is what the receipt
                // already says. The member is the unit an operator resumes at.
                note_landed(member, &landed);
                done.bars_stored += landed.bars;
                // AND WHAT WAS ACTUALLY WRITTEN, beside what was offered. A
                // re-run of a month already held offers every bar and commits
                // none, and only this number can say so.
                done.bars_committed += landed.committed;
                done.rows_folded += landed.folded;
                // The census is folded so the totals describe the RUN. A
                // per-member census would answer "why did this contract drop
                // rows" and the operator is asking "why did this window".
                // FOUR ADDS, AND THIS WAS A LOOP PER DROPPED ROW.
                //
                // It walked the four reasons and then counted **one at a time**
                // — `for _ in 0..landed.census.of(reason)` — to compute four
                // sums that `DropCensus::absorb` expresses in four
                // instructions.
                //
                // Per MEMBER, and the multiplier is the whole point: a
                // one-second archive file covering 00:00–23:59 filtered to a
                // 22,500-second session leaves ~290,000 drops per contract, and
                // a GDFL day is ~12,132 contracts. **~3.5 billion loop
                // iterations to add four numbers.**
                //
                // Constant per dropped row, so no ratio gate could see it —
                // work that scales uniformly with the input does not change a
                // ratio. Same shape as `month_at` above and `read_row` in
                // `store`: constant work nobody needed.
                done.census.absorb(landed.census);
                // ONE COUNT PER FILE THIS MEMBER WROTE — the rung that was
                // pulled and every rung derived from it. A derived bar with no
                // census row is a bar `/store.json` cannot see, which is the
                // store disagreeing with its own counter in the direction that
                // makes a later run refetch a month already on disk.
                done.derived_files += landed.derived;
                // A DERIVED RUNG THAT DID NOT LAND IS A FAILURE, NOT A NOTE.
                if let Some(short) = derived_shortfall(member, &landed) {
                    // BOTH SURFACES, NEVER ONE. The receipt is for whoever is
                    // watching; the log is what a diagnosis is handed later.
                    note_derived_shortfall(member, &short);
                    done.failures.push(short);
                }
                done.failures.extend(landed.failures);
                for held in landed.entries {
                    // A PARTITION AT ITS CAPACITY IS COMPACTED AND THE ROW
                    // RETRIED (D-3316); a single file refuses as it always did.
                    match census_store::count_growing(&mut loaded.census, held, &mut compacted) {
                        Ok(changed) => {
                            done.counted += 1;
                            if let Some(append) = changed {
                                appends.push(append);
                            }
                        }
                        // THE WORST OUTCOME THERE IS, AND IT IS NAMED. The
                        // bars are on the disk and the census does not count
                        // them, so a later run refetches a month the store
                        // already holds and the append refuses it. Swallowing
                        // this would leave a store that disagrees with its own
                        // counter and no record of when it started.
                        Err(why) => {
                            // THE STORE DISAGREES WITH ITS OWN COUNTER. Named
                            // in the log as well as on the receipt, because a
                            // later run refetches a month already on disk.
                            note_bars_not_counted(&member.instrument, held.entry.rows, &why);
                            done.failures.push(
                                Failure::new(
                                    member.instrument.clone(),
                                    format!(
                                        "{} holds {} bar(s) the census does not count: {why}",
                                        member.instrument, held.entry.rows
                                    ),
                                )
                                .in_month(held.entry.key.month),
                            );
                        }
                    }
                }
            }
            Err(why) => {
                note_not_landed(member, &why);
                done.failures
                    .push(Failure::new(member.instrument.clone(), why));
            }
        }
    }

    // ONE INSTALL, AFTER THE LOOP — and none at all when nothing changed, so
    // a re-run of the same folder leaves the census byte for byte as it was.
    // A partition compacted mid-run, or loaded degraded, is published whole as
    // its next epoch, and the epoch it replaces is never written again
    // (D-3316 items 2 and 4).
    if let Err(why) =
        census_store::install_run(&census_lock, &target, &loaded, &appends, publish, compacted)
    {
        note_census_unpublished(&census_path, appends.len(), &why);
        done.failures.push(Failure::new(
            census_path.display().to_string(),
            format!(
                "{} slice(s) are on disk and the census that counts them was \
                 not published: {why}",
                done.counted
            ),
        ));
    }

    (done, refusals)
}

/// A run refused before a single bar was written, named against the file the
/// refusal is about.
///
/// **`rows_read` is the real count, not zero.** The members were read; what
/// failed is ours — an unwritable rung, a contended lock, an unreadable census
/// — and reporting zero rows would put the blame on a vendor that answered
/// perfectly. Three arms of [`from_members`] end this way and they were three
/// copies of it, which is three places for one of them to start reporting zero.
fn refused_whole(members: &[Member], about: &Path, why: String) -> Ingested {
    let rows_read = members.iter().map(|member| member.rows.len()).sum();
    // THE WHOLE RUN REFUSED, AND UNTIL NOW THE LOG SAID NOTHING.
    //
    // The receipt carried this and the log did not, so an operator reading
    // `/logs` after a backfill that landed nothing saw a quiet file. Three arms
    // of `from_members` end here — an unwritable rung, a contended lock, an
    // unreadable census — and each is the end of the run, not of one member.
    //
    // `Error`, so it clears the default `Info` floor, and once per run, so
    // D-0075's 64 MiB window arithmetic is untouched.
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::error("pull.run", "refused")
            .with("about", telemetry::Value::Str(&about.display().to_string()))
            .with("members", telemetry::Value::Uint(members.len() as u64))
            .with("rows_read", telemetry::Value::Uint(rows_read as u64))
            .with("why", telemetry::Value::Str(&why)),
    );
    Ingested {
        members: members.len(),
        rows_read,
        failures: vec![Failure::new(about.display().to_string(), why)],
        ..Ingested::default()
    }
}

/// The census loaded from a torn commit, and what loaded is a prefix.
///
/// `Warn` and once per run. Months a newer generation had committed are not in
/// it and can only be got back from the bar files.
fn note_census_degraded(census: &Path, why: &str) {
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::warn("pull.census", "loaded degraded")
            .with(
                "census",
                telemetry::Value::Str(&census.display().to_string()),
            )
            .with("why", telemetry::Value::Str(why)),
    );
}

/// Bars are on disk and the census that counts them was not published.
///
/// A later run refetches months this one already stored, and the append refuses
/// them. `Error`, once per run.
fn note_census_unpublished(census: &Path, slices: usize, why: &str) {
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::error("pull.census", "not published")
            .with(
                "census",
                telemetry::Value::Str(&census.display().to_string()),
            )
            .with("slices", telemetry::Value::Uint(slices as u64))
            .with("why", telemetry::Value::Str(why)),
    );
}

/// The store holds bars its own counter does not know about.
fn note_bars_not_counted(instrument: &str, bars: u64, why: &str) {
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::error("pull.census", "bars not counted")
            .with("instrument", telemetry::Value::Str(instrument))
            .with("bars", telemetry::Value::Uint(bars))
            .with("why", telemetry::Value::Str(why)),
    );
}

/// One window fetched from a broker, through the same path a folder takes.
///
/// # The join this repository was one function short of
///
/// `docs/05-decisions.md` D-0035 stopped at the credential port; the page's own
/// banner has said since then that "what is missing is one join". This is it.
///
/// [`crate::http::HttpSource::window_async`] answers with a
/// [`crate::fetch::RawWindow`] — the same seven parallel arrays a CSV member
/// decodes into — so the broker's rows are wrapped in a [`Member`] and handed to
/// [`from_members`]. **Nothing downstream can tell the two apart**, which is the
/// point: the session filter, the drop census, the fold, the append, the counter
/// row and every refusal are one implementation, not two.
///
/// `instrument` is what the store will file the bars under, and it comes from
/// the caller because only the caller knows which contract it asked the broker
/// for. `origin` is recorded on the member so a refusal downstream can name
/// where the rows came from — a URL here, where a folder path would be.
#[must_use]
pub fn from_window(
    raw: &crate::fetch::RawWindow,
    instrument: &str,
    origin: &str,
    store_root: &Path,
    plan: Plan<'_>,
) -> Ingested {
    from_window_mode(raw, instrument, origin, store_root, plan, Mode::Append)
}

/// One window fetched from a broker, landed as a VERSIONED REPAIR of every
/// month it touches that holds bars around a hole (D-3350, WP-13c; DB-N20,
/// DB-N22).
///
/// The same door as [`from_window`] — the same row screening, session filter,
/// fold, census, derived rungs and gap ledger — with one difference at the
/// write: a month the append-only store refuses because the offered bars lie
/// inside its held range, or a month already served from a revision, is
/// published as a NEW generation holding every bar it held plus the offered
/// ones ([`store::repair::repair_month`]) and promoted so every reader opens
/// it. A held bar is never dropped and never changed: an offered bar that
/// disagrees with one refuses that month by name. A month the append door
/// takes (a new month, a tail) is appended as usual. Every repair is
/// journaled in `<store>/audit/<vendor>.repairs` with `reason`.
#[must_use]
pub fn repair_window(
    raw: &crate::fetch::RawWindow,
    instrument: &str,
    origin: &str,
    store_root: &Path,
    plan: Plan<'_>,
    reason: &str,
) -> Ingested {
    from_window_mode(
        raw,
        instrument,
        origin,
        store_root,
        plan,
        Mode::Repair { reason },
    )
}

/// Whether a landing appends, or repairs a month the append door refuses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode<'a> {
    /// The append-only door, and nothing else.
    Append,
    /// A month the append door refuses is repaired to a new generation, for
    /// this reason (D-3350).
    Repair {
        /// Why, journaled with every repair.
        reason: &'a str,
    },
}

fn from_window_mode(
    raw: &crate::fetch::RawWindow,
    instrument: &str,
    origin: &str,
    store_root: &Path,
    plan: Plan<'_>,
    mode: Mode<'_>,
) -> Ingested {
    // This door receives vendor candles. Archive members bypass it and retain
    // all legitimate snapshots, including identical rows within one second.
    //
    // A CANDLE OFF THE GRID, OR TWO CANDLES AT ONE STAMP THAT DISAGREE, ARE
    // REFUSED BY NAME AND THE REST OF THE ANSWER LANDS (D-3351, DB-N23). Each
    // refused the whole answer, and a whole answer refused while the next one
    // landed is how a month was opened part-way for ever (DB-N20).
    let (rows, screened, duplicates) = screen_candles(raw, plan);
    // A MINUTE THAT IS NOT THIS SERIES ON THIS DAY IS REFUSED BY NAME AND THE
    // REST LANDS (WP-13d; DB-B07, DB-B08, DB-B10, DB-B60, DB-B64): before the
    // series' first served day bar, or wholly outside its own day's range.
    let (rows, foreign) = identity_screen::screen(raw, rows, instrument, store_root, plan);
    let member = Member {
        path: std::path::PathBuf::from(origin),
        instrument: instrument.to_owned(),
        rows,
    };
    let (mut done, mut refusals) =
        from_members_mode(std::slice::from_ref(&member), store_root, plan, mode);
    done.rows_read += screened.rows() + foreign.rows();
    done.failures
        .extend(refused_row_failures(instrument, &screened, plan.encoding));
    done.failures
        .extend(refused_row_failures(instrument, &foreign, plan.encoding));
    refusals.seconds.push(screened.local_seconds);
    // Its own time-ordered list: the gap ledger merges one list per source.
    refusals.seconds.push(foreign.local_seconds);
    // ONE EVENT AND ONE FAILURE PER (MONTH, REASON), AND ONE LEDGER LINE PER
    // DAY (D-3335, D-3341, WP-13b). It was one event and one failure per gap
    // (OD-2, D-2371), and each failure one `pull.spot` line besides. A minute
    // refused after the vendor sent it — a row named, or a month the store
    // refused — is a minute the store lacks, and its day says why (D-3351).
    request_minutes::report(
        &member.rows,
        instrument,
        store_root,
        plan,
        &mut done,
        &refusals,
    );
    done.rows_read += duplicates;
    done.rows_folded += duplicates;
    if duplicates > 0 {
        let _dropped_when_filtered = telemetry::emit(
            &telemetry::Event::warn(
                "pull.duplicate",
                "identical vendor candles deduplicated; volume counted once",
            )
            .with("instrument", telemetry::Value::Str(instrument))
            .with("duplicates", telemetry::Value::Uint(duplicates as u64)),
        );
    }
    done
}

/// The broker's candles a landing may take: each on the requested grid, and
/// one per stamp. A candle off the grid is refused; two candles at one stamp
/// that disagree are BOTH refused — neither can be chosen without inventing
/// which one the vendor meant; identical repeats are one candle. Answers the
/// kept rows in the vendor's order, the refusals in the vendor's order, and
/// how many identical repeats were folded (D-3351).
///
/// Two passes over the rows and one hash probe per row: O(rows).
fn screen_candles(
    raw: &crate::fetch::RawWindow,
    plan: Plan<'_>,
) -> (Vec<crate::fetch::RawRow>, crate::fetch::Refused, usize) {
    /// What a stamp has been seen as.
    #[derive(Clone, Copy)]
    enum Seen {
        /// One candle, or identical repeats of it, first at this row.
        One(usize),
        /// Candles that disagree: every row at this stamp is refused.
        Conflicting,
    }
    let on_grid = |row: &crate::fetch::RawRow| {
        broker_stamp_on_grid(row.timestamp, plan.encoding, plan.request.granularity)
    };
    let mut seen: std::collections::HashMap<i64, Seen> =
        std::collections::HashMap::with_capacity(raw.rows.len());
    for (i, row) in raw.rows.iter().enumerate() {
        if !on_grid(row) {
            continue;
        }
        match seen.get(&row.timestamp).copied() {
            None => {
                seen.insert(row.timestamp, Seen::One(i));
            }
            Some(Seen::One(first)) if raw.rows.get(first) != Some(row) => {
                seen.insert(row.timestamp, Seen::Conflicting);
            }
            Some(_) => {}
        }
    }
    let mut kept = Vec::with_capacity(raw.rows.len());
    let mut refused = crate::fetch::Refused::default();
    let mut duplicates = 0usize;
    for (i, row) in raw.rows.iter().enumerate() {
        let local = stamp_local_second(row.timestamp, plan.encoding);
        if !on_grid(row) {
            refused.note(crate::fetch::RowCause::OffGrid, (i, row.timestamp), local, || {
                format!(
                    "row {i}: off-grid broker candle at {} ({:?}) for {}; off-grid candle refused before fold or append, this row only",
                    row.timestamp,
                    plan.encoding,
                    plan.request.granularity.label()
                )
            });
            continue;
        }
        match seen.get(&row.timestamp).copied() {
            Some(Seen::One(first)) if first == i => kept.push(*row),
            Some(Seen::One(_)) => duplicates += 1,
            _ => refused.note(
                crate::fetch::RowCause::Conflicting,
                (i, row.timestamp),
                local,
                || {
                    format!(
                        "row {i}: conflicting vendor candles at {}; conflicting candles refused before append, every candle at that stamp",
                        row.timestamp
                    )
                },
            ),
        }
    }
    (kept, refused, duplicates)
}

/// The IST-local epoch second of a vendor stamp, in the arithmetic
/// [`stamp_day`] uses: the space the gap ledger counts minutes in.
fn stamp_local_second(timestamp: i64, encoding: TimestampEncoding) -> i64 {
    let offset = i128::from(crate::session::IST_OFFSET_SECS) * 1_000;
    let utc_millis = match encoding {
        TimestampEncoding::EpochMillisUtc => i128::from(timestamp),
        TimestampEncoding::EpochSecondsUtc | TimestampEncoding::IsoDateTimeOffset => {
            i128::from(timestamp) * 1_000
        }
        TimestampEncoding::IstDateTimeText | TimestampEncoding::IsoDateTimeText => {
            i128::from(timestamp) * 1_000 - offset
        }
    };
    i64::try_from((utc_millis + offset).div_euclid(1_000)).unwrap_or(i64::MAX)
}

/// One named failure per (cause, day) of refused rows, logged once each
/// (D-3351): the first row in its own words, the count, and that it is those
/// rows only.
fn refused_row_failures(
    instrument: &str,
    refused: &crate::fetch::Refused,
    encoding: TimestampEncoding,
) -> Vec<Failure> {
    refused
        .groups
        .iter()
        .map(|group| {
            let why = format!(
                "{}; {} row(s) refused for {} on this day, those rows only: the rest of the answer lands (D-3351)",
                group.why,
                group.rows,
                group.cause.code()
            );
            note_not_filed(instrument, group.cause.code(), &why);
            on_stamp_day(Failure::new(instrument, why), group.first_raw, encoding)
        })
        .collect()
}

/// `failure`, about the IST day a vendor stamp falls on, when that day is one
/// the calendar can name. DB-G45: the failure carries its own day rather than
/// the window's first month.
fn on_stamp_day(failure: Failure, timestamp: i64, encoding: TimestampEncoding) -> Failure {
    match stamp_day(timestamp, encoding) {
        Some(day) => failure.on_day(day),
        None => failure,
    }
}

/// The IST calendar day a vendor stamp falls on, in the same arithmetic
/// [`broker_stamp_on_grid`] uses, or [`None`] for a stamp outside the
/// calendar's range.
fn stamp_day(timestamp: i64, encoding: TimestampEncoding) -> Option<crate::session::Day> {
    let offset = i128::from(crate::session::IST_OFFSET_SECS) * 1_000;
    let utc_millis = match encoding {
        TimestampEncoding::EpochMillisUtc => i128::from(timestamp),
        TimestampEncoding::EpochSecondsUtc | TimestampEncoding::IsoDateTimeOffset => {
            i128::from(timestamp) * 1_000
        }
        TimestampEncoding::IstDateTimeText | TimestampEncoding::IsoDateTimeText => {
            i128::from(timestamp) * 1_000 - offset
        }
    };
    let days = (utc_millis + offset).div_euclid(86_400_000);
    crate::session::Day::from_days(u32::try_from(days).ok()?).ok()
}

/// Broker candles must already start on the requested grid. Archive snapshots
/// never enter this check. Work in milliseconds so subsecond offsets cannot be
/// hidden by the later epoch-millisecond conversion; no input stamp is changed.
fn broker_stamp_on_grid(
    timestamp: i64,
    encoding: TimestampEncoding,
    granularity: crate::vendor::Granularity,
) -> bool {
    let crate::vendor::Grid::Intraday(seconds) = granularity.grid() else {
        return true;
    };
    let offset = i128::from(crate::session::IST_OFFSET_SECS) * 1_000;
    let utc_millis = match encoding {
        TimestampEncoding::EpochMillisUtc => i128::from(timestamp),
        TimestampEncoding::EpochSecondsUtc | TimestampEncoding::IsoDateTimeOffset => {
            i128::from(timestamp) * 1_000
        }
        TimestampEncoding::IstDateTimeText | TimestampEncoding::IsoDateTimeText => {
            i128::from(timestamp) * 1_000 - offset
        }
    };
    let open = i128::from(Timeframe::OPEN_MINUTES_PAST_IST_MIDNIGHT) * 60_000;
    (utc_millis + offset - open).rem_euclid(i128::from(seconds) * 1_000) == 0
}

/// Records one written month in the census, through the same lock and install
/// the raw-row door uses.
///
/// # Why the counter is not optional
///
/// A month on disk the census does not know about is a month `/store` cannot
/// see and the ladder gate treats as missing, so the next run refetches what is
/// already there. `CLAUDE.md` §3 rule 5 makes that harmless and §3 rule 4 makes
/// it expensive.
///
/// # Cost — and the doc this replaces claimed the opposite of the truth
///
/// It read: *"One lock, one read, one incremental append. O(1) in the entries
/// the census already holds."* The middle clause is the whole cost.
/// `read_census` reads the entire manifest file and `Manifest::load` decodes
/// **every one of `n_valid` entries**, checksum-verifies each, inserts each
/// into a `HashMap`, and walks the values again for the row total. It is
/// Θ(entries the census already holds) — precisely the quantity it named itself
/// constant in.
///
/// That mattered little at one call per member. It matters now: splitting a
/// rolling answer into its several contracts made this run once per GROUP, so
/// one vendor answer could trigger thirty full manifest walks, against 252
/// requests a month, while the manifest grows by that same count. Total
/// backfill cost was quadratic in entries written.
///
/// **So it takes a SLICE.** One lock, one read, one decode and one install for
/// however many entries a caller has in hand — which is the shape
/// `from_members_inner` has always used for the spot path, and the reason that
/// path never had this problem.
///
/// Per call: Θ(entries) for the read, plus O(offered) to fold them in. Per
/// ENTRY offered it is now amortised, which is the property that was missing.
///
/// **UNVERIFIED as a measurement.** The bound is argued from the
/// shape of the code and no bench in this workspace times it.
/// `CLAUDE.md` §3 rule 6: a structural argument is not a
/// measurement, however sound it is.
///
/// # The census at its ceiling refuses UP FRONT (D-3316, DB-G10)
///
/// The batch is probed before a row is recorded. A single file without room
/// for it refuses with the FILE's `n_valid`, its capacity, the free slots and
/// the batch's size, and says nothing was published — where it used to record
/// rows in memory until the first one past the ceiling, and report that row's
/// in-memory counter (`n_valid is 2097153`) as if it were the file's. A
/// migrated vendor's partition compacts once into its next epoch instead, and
/// refuses with the same numbers only when one row per key still leaves no
/// room for the batch (D-3316 item 5). Rows go to one partition per segment,
/// each all-or-nothing. See [`census_store::record`].
fn record_all(store_root: &Path, vendor: Vendor, held: &[Held]) -> Option<String> {
    census_store::record(store_root, vendor, held).err()
}

/// Writes a batch of census rows in ONE cycle — one lock, one read, one install.
///
/// # Why a batch, and what one-at-a-time cost
///
/// `record_all` reads the entire manifest and decodes every entry in it. Paying
/// that per CONTRACT meant one rolling vendor answer — which spans four or five
/// weekly contracts and steps strike whenever spot crosses a boundary —
/// triggering thirty full manifest walks, against 252 requests a month, while
/// the manifest grows by that same count. Total backfill cost was quadratic in
/// entries written.
///
/// The spot path never had this shape: `from_members_inner` reads the census
/// once, accumulates, and installs once. This is that shape, made available
/// where the contract split made it necessary.
///
/// The public face of `record_all`. A caller that filed several contracts from
/// a single vendor answer collects each run's [`Ingested::pending`] and calls
/// this once, instead of paying a full manifest decode per contract.
///
/// Returns the reason when the census could not be published, exactly as the
/// single-row path reports it.
#[must_use]
pub fn record_held(store_root: &Path, vendor: Vendor, held: &[Held]) -> Option<String> {
    if held.is_empty() {
        return None;
    }
    record_all(store_root, vendor, held)
}

/// Files bars and their overlays that are ALREADY DECODED.
///
/// # Why this door exists beside [`from_window`]
///
/// That one takes RAW vendor rows and decodes them on the way in, which is
/// right for every bars endpoint in this build. Dhan's expired-options answer
/// cannot arrive that way: the spot and the implied volatility are lifted from
/// the SAME parallel arrays as the open and the close, so a raw-row shape that
/// carried them would have to be a second row type — and the reader that
/// splits those arrays is the only thing that knows which is which.
///
/// So `pull::rolling` decodes, and this files what it produced.
///
/// # What it does NOT do, and why
///
/// It does not fold and it does not derive. A derived rung of an option is not
/// a thing this store keeps — `derived_count_in` already answers zero for a
/// contract — and folding a minute series that arrived at minute resolution
/// would be a no-op wearing a cost.
///
/// # Errors
///
/// Reported through [`Ingested::failures`] rather than returned, so one
/// contract that will not file does not abandon the month's other two hundred.
///
/// # Cost
///
/// **Corrected by D-2372 (OD-3): this claimed two constant-cost appends and
/// that nothing scanned — neither half held.** `keep_in_session` walks
/// every bar and every overlay once, O(rows), before anything opens. Then two
/// opens and two appends, and each
/// `BarFile::append` is not O(1) in the file: it locates the batch against
/// what is already stored by bisection, `already_stored`, at most
/// `ceil(log2(n_valid + 1))` record reads plus O(batch) comparisons (D-1434);
/// it re-reads and verifies the old tail block before re-sealing it (D-0910);
/// and it re-reads every block the batch touches to seal its checksum.
/// So one call is `O(rows + log n_valid + blocks touched)` reads and work, not
/// constant. `docs/06-limits.md`, D-2372.
///
/// **UNVERIFIED as a measurement.** The bound is argued from the
/// shape of the code and no bench in this workspace times it.
/// `CLAUDE.md` §3 rule 6: a structural argument is not a
/// measurement, however sound it is.
#[must_use]
pub fn from_rows(
    bars: &[store::format::Bar],
    overlays: &[store::format::Overlay],
    instrument: &str,
    origin: &str,
    store_root: &Path,
    plan: Plan<'_>,
) -> Ingested {
    let (done, _appended) = rows_mode(
        bars,
        overlays,
        (instrument, origin),
        store_root,
        plan,
        RowsDoor::Append,
    );
    done
}

/// How [`repair_rows`] filed a month's bars (D-3350; DB-N26, WP-12d).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Filed {
    /// Through the append-only door, exactly as [`from_rows`] files them:
    /// appended, already held, or refused (the failures name why). No new
    /// generation of the month was published.
    Appended,
    /// As a NEW GENERATION of the month (`store::repair::repair_month`): the
    /// generation it served, every bar kept and none changed, plus the
    /// offered bars, of which it held none, published under
    /// `bar-revisions-v1/<n>`, promoted so every reader opens it, and
    /// journaled in `<store>/audit/<vendor>.repairs`. The original's files
    /// are untouched.
    NewGeneration,
}

/// Bars ALREADY DECODED, filed as [`from_rows`] files them — the same session
/// filter, the same census row handed back, the same refusals — but a batch
/// the served month holds NOTHING of and the append-only store cannot take
/// is BACK-FILLED as a versioned repair (D-3350) rather than refused: a day
/// the month lacks between, before or among the days it holds, or any day a
/// month already served from a revision lacks. DB-N26, WP-12d.
///
/// # Why the GDFL import needs it
///
/// `store::file::BarFile::append` takes a month's bars only after the last
/// one it holds: a batch inside the held range at a stamp the month never
/// held is refused (`store::file::Conflict::NotHeld`), and must be, since an
/// append-only file cannot insert. So a GDFL day that lies inside a month
/// already held past it — the 8 Muhurat sessions, the 2 DR drills,
/// 2021-02-24 and 2022-03-07, which the calendar refused when the months
/// were first imported and admits since D-3319 (WP-12a) — could never land:
/// the run of 10 Oct 2026 refused every NIFTY and BANKNIFTY file of the 12
/// days. This is the one repair authority WP-13c built for the Zerodha
/// holes ([`repair_window`]'s door, `repair_after`), reached from the
/// decoded-bar door; it is not a second one.
///
/// # What is back-filled, and what is not
///
/// The append door is tried first, so a tail, a new month and a batch
/// already held land, or answer, exactly as [`from_rows`] does. When it
/// refuses, the served month is read (`lack_of`: one generation probe, one
/// open, one time-indexed lookup and one record) and the batch is
/// back-filled only when BOTH hold:
///
/// * the month holds no bar anywhere in the batch's span, first stamp to
///   last — it is bars the month LACKS, never a held day completed, restated
///   or mixed with another reading of it; and
/// * the append door cannot ever take it: the span starts at or before the
///   month's last held stamp, or the month is served from a revision, which
///   is never appended to.
///
/// Every other refusal of the append door stands as it was, in its own
/// words, and nothing is repaired: a month another process holds locked, a
/// host error, a batch after an original month's tail that the store refused
/// for a reason of its own, and a batch that overlaps held bars and disagrees
/// with them. So this door, which every GDFL file is filed through, cannot
/// turn a passing lock or a full disk into a new generation of a month that
/// needed none.
///
/// # What it never does
///
/// It never writes a month in place: a repair publishes a new generation and
/// leaves the original's `.bin`, `.crc`, `.tix` and `.prv` byte-identical. It
/// never changes or drops a held bar: it adds bars at stamps the month holds
/// nothing near, and `store::repair::repair_month` refuses the month whole
/// if a held bar would differ. It files no overlay: inserting bars would
/// shift every row an overlay indexes, which is why `repair_month` refuses a
/// month that has one.
///
/// # Journaled before and after
///
/// One `repairing` line is appended to `<store>/audit/<vendor>.repairs`
/// BEFORE the repair (the month, the generation served, the bars offered and
/// their span, the reason), and the repair's own `repair` or `refused` line
/// after it. A process killed between the promotion and its `repair` line
/// leaves the `repairing` line: no generation is ever served that the
/// journal does not name. A `repairing` line that cannot be written refuses
/// the back-fill before anything is published.
///
/// # Errors
///
/// As [`from_rows`], through [`Ingested::failures`]; a refused repair names
/// the append door's refusal and the repair's, and is journaled `refused`.
///
/// # Cost
///
/// [`from_rows`]'s when the append door takes the bars. A back-fill reads
/// every bar the served generation holds, merges, writes and reads back the
/// new generation, all O(held + offered), bounded by
/// `store::repair::MAX_ROWS`, and appends one pointer record and two journal
/// lines: NOT O(1), and not on a read path. A month already served from a
/// revision pays that for EVERY later batch it lacks, one generation each (a
/// revision is never appended to), bounded by `store::repair::MAX_REVISIONS`
/// per month. A reader's cost is unchanged: one pointer resolution per open.
#[must_use]
pub fn repair_rows(
    bars: &[store::format::Bar],
    instrument: &str,
    origin: &str,
    store_root: &Path,
    plan: Plan<'_>,
    reason: &str,
) -> (Ingested, Filed) {
    rows_mode(
        bars,
        &[],
        (instrument, origin),
        store_root,
        plan,
        RowsDoor::BackFill { reason },
    )
}

/// Which door [`rows_mode`] writes through.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RowsDoor<'a> {
    /// The append-only door, and nothing else ([`from_rows`]).
    Append,
    /// The append-only door, and a versioned repair of a month that lacks
    /// the whole batch where no append can ever land it ([`repair_rows`]).
    BackFill {
        /// Why, journaled with every repair.
        reason: &'a str,
    },
}

/// [`from_rows`] and [`repair_rows`], one implementation: `door` decides only
/// the write.
fn rows_mode(
    bars: &[store::format::Bar],
    overlays: &[store::format::Overlay],
    (instrument, origin): (&str, &str),
    store_root: &Path,
    plan: Plan<'_>,
    door: RowsDoor<'_>,
) -> (Ingested, Filed) {
    let mut done = Ingested {
        members: 1,
        rows_read: bars.len(),
        ..Ingested::default()
    };
    // THE WINDOW AND THE SESSION, WHICH THIS PATH NEVER APPLIED AT ALL.
    // See [`keep_in_session`] for what that cost.
    let (bars, overlays, census) = keep_in_session(bars, overlays, &plan);
    done.census = census;
    let (bars, overlays) = (bars.as_slice(), overlays.as_slice());
    if bars.is_empty() {
        return (done, Filed::Appended);
    }
    let Some((first, last)) = bars.first().zip(bars.last()) else {
        return (done, Filed::Appended);
    };
    let addressed = month_of(first, last).and_then(|ym| {
        plan.timeframe().and_then(|timeframe| {
            identify(instrument, plan.exchange, plan.segment)
                .map(|identity| (ym, timeframe, identity))
        })
    });
    // THREE REFUSALS, ONE ARM. Each was its own `match` with an identical
    // four-line error branch, which is three chances to get the recording
    // wrong and, in this function, three copies of the same paragraph.
    let (ym, timeframe, identity) = match addressed {
        Ok(three) => three,
        Err(why) => {
            note_not_filed(instrument, "address", &why);
            done.failures.push(Failure::new(instrument, why));
            return (done, Filed::Appended);
        }
    };
    let Identity {
        symbol,
        exchange,
        segment,
        symbol_id,
    } = identity;

    let parts = PathParts {
        vendor: plan.vendor,
        exchange: exchange.as_str(),
        segment: segment.as_str(),
        symbol: symbol.as_str(),
        contract: plan.contract,
        timeframe,
        month: ym,
        file: FileKind::Bars,
    };
    let key = EntryKey {
        contract: plan.contract,
        exchange,
        segment,
        symbol,
        timeframe,
        month: ym,
    };
    // THE ONE DIFFERENCE BETWEEN THE TWO DOORS (D-3350; DB-N26). The
    // back-fill door tries the append door first, and repairs only a month
    // that lacks the whole batch where no append can ever land it.
    let held = match door {
        RowsDoor::Append => write_and_count(bars, store_root, symbol_id, parts, key)
            .map(|(one, committed)| (one, committed, Filed::Appended)),
        RowsDoor::BackFill { reason } => back_fill_or_append(
            bars,
            store_root,
            symbol_id,
            (parts, key),
            (repair_policy(plan), reason),
        ),
    };
    let (one, filed) = match held {
        Ok((one, committed, filed)) => {
            done.bars_stored = bars.len();
            done.bars_committed = committed;
            (one, filed)
        }
        Err(why) => {
            note_not_filed(instrument, "bars", &why);
            done.failures
                .push(Failure::new(instrument, why).in_month(ym));
            return (done, Filed::Appended);
        }
    };

    // THE CENSUS ROW IS HANDED BACK, NOT WRITTEN HERE.
    //
    // A caller filing SEVERAL contracts from one vendor answer — which is every
    // rolling answer, since one spans four or five weekly contracts and steps
    // strike with spot — would otherwise pay a full census cycle per contract:
    // lock, read the whole manifest, decode every entry, install, fsync. Thirty
    // groups is thirty walks, against 252 requests a month, while the manifest
    // grows by that same count. `from_rows` below records the one entry it has;
    // `roll_one` collects and records once.
    done.pending = Some(one);
    // COUNTED, BECAUSE IT WILL BE — **and the reason given here was wrong.**
    //
    // This said: *"the caller returns an error if the batch cannot be
    // published, so there is no path where this reports counted and the census
    // does not hold it."* The overlay write below WAS that path. It runs after
    // `pending` and `counted` are set; its failure went into `failures`; and
    // `land_rolling_group` returned `Err` on any failure, so the caller's `Err`
    // arm skipped `census_rows.extend(pending)` entirely. Bars on disk, census
    // row dropped, `counted: 1` a lie — and every retry repeated it, because
    // the bars come back `AlreadyPresent` and the overlay refuses again.
    //
    // The property holds now, and it holds because the CALLER was fixed rather
    // than because this line was right: `land_rolling_group` returns `Err` only
    // when `pending.is_none()` — which is the honest test for "nothing landed",
    // since the early return above leaves it unset when the BAR write fails —
    // and otherwise publishes the row with the reason travelling beside it.
    // D-0343.
    done.counted = 1;

    // THE OVERLAY AFTER THE BARS, NEVER BEFORE. If the bar write fails there is
    // nothing for an overlay row to be a column of, and a sidecar describing
    // bars that are not there is worse than no sidecar: the next reader joins
    // on a stamp that has no bar.
    if let Err(why) = write_overlay(overlays, store_root, symbol_id, parts) {
        note_not_filed(instrument, "overlay", &why);
        done.failures
            .push(Failure::new(instrument, why).in_month(ym));
    }
    name_the_origin(&mut done, origin);
    (done, filed)
}

/// Drops the bars this engine declines, and counts why.
///
/// The same question `fetch::land` asks of a raw vendor row, asked of a bar
/// that is already decoded: is it inside the operator's window, and is it
/// inside the venue's session? The venue comes from the request's listing, so
/// an expired derivative is judged against the derivatives clock (15:40 since
/// 2026-08-03) and not the cash one.
///
/// # Why this did not exist, and what its absence cost
///
/// `from_window` reaches `fetch::land`, which asks `Window::verdict` of every
/// row and counts what it declines. [`from_rows`] takes bars that are ALREADY
/// decoded and so never touched `land` — so the rolling driver stored whatever
/// the vendor sent: no before-window check, no after-window check, and no
/// session-hours check.
///
/// Two costs. A vendor that treats `toDate` as inclusive where this build reads
/// it exclusive appends one extra day past every chunk boundary with nothing to
/// catch it. And `Ingested::census` stayed empty, which makes
/// [`Ingested::balances`] trivially true — the books balanced because there was
/// nothing on either side of them.
///
/// The fixture that caught this asserted two bars stamped **08:00 IST** reached
/// the store. They are an hour and a quarter before the open, and storing them
/// is what this path did.
///
/// Filtered here rather than by routing through `land`: that function decodes
/// RAW vendor rows and these are `store::format::Bar` already. What is shared is
/// the QUESTION, and it is asked once in each path rather than answered twice.
///
/// # Why the overlays are filtered by the same question and not by the bars
///
/// An overlay carries its own stamp and joins on it, so asking the verdict of
/// each independently keeps the two consistent without a set of kept stamps to
/// probe against — one pass each, no hashing.
///
/// # Cost
///
/// One pass over the bars and one over the overlays, one verdict each. The
/// verdict is arithmetic against a fixed session table.
fn keep_in_session(
    bars: &[store::format::Bar],
    overlays: &[store::format::Overlay],
    plan: &Plan<'_>,
) -> (
    Vec<store::format::Bar>,
    Vec<store::format::Overlay>,
    DropCensus,
) {
    let mut census = DropCensus::default();
    let cadence = plan.request.granularity.cadence();
    let venue = plan.request.listing.venue();
    let verdict = |ts_micros: i64| {
        plan.request
            .window
            .verdict(ts_micros.div_euclid(1_000_000), cadence, venue)
    };

    let mut kept = Vec::with_capacity(bars.len());
    for bar in bars {
        // A TIMESTAMP THE CALENDAR CANNOT READ IS A DROP, NOT A HALT. `land`
        // refuses one because it is decoding the vendor and a stamp it cannot
        // read means the DECODER is wrong. Here the bar is already built, so
        // the same value is a bar this engine declines — counted, never stored,
        // and never silently kept.
        match verdict(bar.ts_micros) {
            Ok(None) => {
                // Kept on a day the calendar cannot classify: counted by name
                // (P-03, D-2673), never a silent keep.
                if crate::session::on_unclassified_day(bar.ts_micros.div_euclid(1_000_000)) {
                    census.count_unclassified_kept();
                }
                kept.push(*bar);
            }
            Ok(Some(reason)) => census.count(reason),
            Err(_) => census.count(crate::session::DropReason::BeforeWindow),
        }
    }
    let overlays = overlays
        .iter()
        .filter(|o| matches!(verdict(o.ts_micros), Ok(None)))
        .copied()
        .collect();
    (kept, overlays, census)
}

/// Puts the endpoint that produced these rows onto every refusal they caused.
///
/// # Why it was missing
///
/// `from_window` builds `Member { path: PathBuf::from(origin), .. }` for the
/// stated reason that "a refusal downstream can name where the rows came from —
/// a URL here, where a folder path would be." [`from_rows`] took the same
/// argument and threw it away with `let _ = origin`, so every refusal on the
/// rolling path was anonymous as to which of 252 requests produced it. No wrong
/// bytes reach disk from that; it costs the diagnosis when wrong bytes do.
///
/// Applied once at the end rather than threaded through three construction
/// sites — those differ in WHY they failed and agree entirely on where from.
fn name_the_origin(done: &mut Ingested, origin: &str) {
    for failure in &mut done.failures {
        failure.why = format!("{} (from {origin})", failure.why);
    }
}

/// What one member put on disk, and the counter row that describes it.
struct Landed {
    /// Bucket refusals accompany safe appends and must reach the receipt.
    failures: Vec<Failure>,
    /// Bars the member offered to the store.
    bars: usize,
    /// Bars actually WRITTEN — zero when the month already held this batch.
    committed: usize,
    /// Snapshots that merged into a bar which was already open.
    folded: usize,
    /// Why rows were declined.
    census: DropCensus,
    /// The counter rows for every month file this member wrote — one for the
    /// rung that was pulled, and one for each rung DERIVED from it.
    ///
    /// Empty when nothing was stored. A `Vec` rather than an `Option` because
    /// a one-minute member now writes eight files: the minute it was asked for
    /// and the seven coarser rungs folded from it. Each needs its own census
    /// row, keyed on its own timeframe — without one, the bars are on disk and
    /// `/store.json` cannot see them, which is the store disagreeing with its
    /// own counter.
    entries: Vec<Held>,
    /// How many of `entries` were derived rather than pulled.
    derived: usize,
    /// How many SHOULD have been, by the rule `derive_rungs` itself walks.
    ///
    /// Carried rather than recomputed by the caller because the contract and
    /// the source rung are known here and are on neither `Member` nor `Plan`.
    /// See `derived_shortfall` for what the difference means.
    derived_expected: usize,
    /// How many of `census`'s drops were for falling outside venue hours.
    ///
    /// **Computed since the session table existed and read by nobody.**
    /// [`fetch::Landed::outside_session`] counts these on the way out, and a
    /// jump in it after a session change is the signal that a row in
    /// `crate::vendor`'s session table has gone stale. That signal reached this
    /// function and stopped: `fetch::Landed` carried it, `ingest::Landed` had no
    /// field for it, and so no receipt, no event and no page ever showed it.
    ///
    /// **A SUBSET of `census`, never an addend to it.** Since the operator's
    /// rule of 2026-08-20 an out-of-session row is dropped, so it is counted
    /// once in `census` and again here, on purpose — `census` answers how many
    /// rows were declined, this answers how many of those the session table
    /// declined. `Ingested::balances` reconciles against `census` alone and
    /// must never learn about this field.
    outside_session: u32,
    /// What this member's landing refused after the vendor sent it, as
    /// IST-local seconds, one time-ordered list per source: rows refused by
    /// name, and rows of months the store refused (D-3351).
    refused_seconds: Vec<Vec<i64>>,
    /// IST day numbers the store refused before a month's first held bar, in
    /// order: the ledger's `refused_front` days.
    fronts: Vec<u32>,
    /// IST day numbers a repair added bars on, in order: the ledger's
    /// `repaired` days (D-3350).
    repaired_days: Vec<u32>,
}

/// The month's two closes, when the batch just appended **is** the whole file.
///
/// # Why this is a question and not an assumption
///
/// The counter row describes the FILE, not the batch — which is why `rows`,
/// `first_ts_micros` and `last_ts_micros` are all read back off the committed
/// header a few lines below rather than taken from `landed.bars`. The closes are
/// the same fact and get the same treatment: on a second window into a month
/// that already held bars, the batch is a *suffix*, and recording its first
/// close as the month's first close would put a fabricated base under every
/// percentage computed from it.
///
/// So the batch's closes are used only when the file is provably the batch: the
/// same count, and the same two instants at the same two ends. **All three are
/// header fields already in hand, so the test is three comparisons and no I/O**
/// — and when it passes, the two closes cost nothing at all. That is the
/// property `pull::unit::a_virgin_month_takes_its_closes_from_the_batch_it_just_wrote`
/// pins: the `None` arm is where the reads live, and a virgin month never
/// reaches it.
///
/// `None` means "read them off the disk" — two O(1) positional reads of 56 bytes
/// on a handle that is already open. Measured against the groww census's own
/// numbers, 43,422 committed months over 216,496,530 records written, that is
/// **86,844 extra 56-byte reads against 216.5 M record writes: +0.040%**. The
/// syscall latency itself is the device's and is UNVERIFIED here —
/// `store::file::BarFile::read_record` says so and this does not claim more.
fn closes_in_hand(header: &Header, batch: &[Bar]) -> Option<(i64, i64)> {
    let first = batch.first()?;
    let last = batch.last()?;
    let count = u64::try_from(batch.len()).ok()?;
    if header.n_valid != count
        || header.first_ts_micros != first.ts_micros
        || header.last_ts_micros != last.ts_micros
    {
        return None;
    }
    Some((first.close, last.close))
}

/// The month's two closes, read back off the file.
///
/// Two positional reads of [`store::format::RECORD_LEN`] bytes at computed
/// offsets on an already-open handle. No scan, no reopen, no allocation.
///
/// `saturating_sub` rather than a checked one: the last committed index of a
/// file holding `n_valid` records is `n_valid − 1`, and for `n_valid == 0` this
/// asks for record 0 of an empty file — which
/// [`BarFile::read_record`] refuses by name, with both numbers, rather than
/// being answered by an arm here that no successful append could reach.
fn closes_on_disk(file: &BarFile, n_valid: u64) -> Result<(i64, i64), String> {
    let first = file.read_record(0).map_err(|why| why.to_string())?;
    let last = file
        .read_record(n_valid.saturating_sub(1))
        .map_err(|why| why.to_string())?;
    Ok((first.close, last.close))
}

/// The month's closes: from the batch when that is free, off the file when it
/// is not, and refused when the pair is not a pair of prices.
///
/// # Errors
///
/// The host's own words for a read that failed, or
/// [`crate::manifest::EntryFault::CloseNotAPrice`] for a negative close — which
/// no tick grid produces, and which would otherwise become a not-recorded
/// sentinel that looks like a price.
///
/// # Why the negative arm can no longer fire from here, and stays anyway
///
/// This runs AFTER `BarFile::append`, and that ordering used to matter: a
/// negative close was refused here, with the bar already on disk. The month
/// then held it permanently — §3 rule 8 forbids a rewrite — and every retry
/// repeated the sequence exactly, appending nothing (`AlreadyPresent`) and
/// failing here again. One bad candle, and the month could never be completed.
///
/// D-0143 removed the trigger rather than the ordering. A negative price is now
/// refused twice before it reaches this line: at the vendor boundary by
/// `http::one_price`, where the value the vendor sent can still be quoted, and
/// at the write boundary by `store::format::Bar::ohlc_is_sane`, which `append`
/// consults through `survey` BEFORE it writes a byte. Both sources this
/// function reads — the batch just appended, and the records already on the
/// file — have therefore crossed that check, so `Closes::known` cannot see a
/// negative by this route.
///
/// The arm is kept because `Closes::known` is a constructor for a public type
/// and is called by things that are not this function. A guard that is
/// currently unreachable via one caller is not a guard that has stopped being
/// needed; deleting it would move the invariant from the type into a comment
/// about the order of two lines in this file.
///
/// What remains after the append is a failed READ — `closes_on_disk` returning
/// the host's error. That leaves the bars written and the manifest entry
/// unrecorded, which the next run resolves: the append answers `AlreadyPresent`
/// and the read is retried. Recoverable, and not the same shape of fault.
fn month_closes(file: &BarFile, header: &Header, batch: &[Bar]) -> Result<Closes, String> {
    let (first, last) = match closes_in_hand(header, batch) {
        Some(pair) => pair,
        None => closes_on_disk(file, header.n_valid)?,
    };
    Closes::known(first, last).map_err(|why| why.to_string())
}

/// One member: convert, fold, append, and describe the month file that
/// resulted.
/// A [`Landed`] that stored nothing, carrying only what is still true of it.
///
/// Both of [`one`]'s early returns built this literal field for field, and they
/// differed in exactly one value — `folded`, which is zero before the fold runs
/// and known after it. Eight duplicated lines twice over is where a field added
/// to `Landed` gets carried on one path and forgotten on the other, which is
/// how `outside_session` would have been lost on the empty-window path while
/// looking correct on the one anybody tests.
fn nothing_landed(
    census: DropCensus,
    folded: usize,
    outside_session: u32,
    (failures, refused_seconds): (Vec<Failure>, Vec<Vec<i64>>),
) -> Landed {
    Landed {
        failures,
        committed: 0,
        bars: 0,
        folded,
        census,
        entries: Vec::new(),
        derived: 0,
        derived_expected: 0,
        outside_session,
        refused_seconds,
        fronts: Vec::new(),
        repaired_days: Vec::new(),
    }
}

/// One month of a member, appended and counted, or its refusal as a named
/// failure that is that month's alone (D-3345).
///
/// The path is rendered from the values the key is built from, not from the
/// plan's strings a second time. `exchange.as_str()` is the parsed `Exchange`
/// speaking, so the directory a bar lands in and the code the census records
/// cannot disagree — there is only one value.
///
/// # Why a refusal is a value and not `?`
///
/// It was `?` in [`one`], so a month the store refused ended the member AFTER
/// the months before it were already appended, and their census rows were
/// dropped with the `Err`: bars on disk the census does not count, which is the
/// outcome `from_members_inner` calls the worst there is. It is also the shape
/// of every re-pull of a window whose last month is already held with its first
/// days missing (a prefix the append-only store cannot take): the months before
/// it would land on every attempt and be counted on none. The refused month is
/// named and left byte-identical, and [`one`] logs it, then lands, derives and
/// counts the others.
fn write_month(
    slice: &[store::format::Bar],
    month: store::path::YearMonth,
    timeframe: Timeframe,
    member: &Member,
    store_root: &Path,
    plan: Plan<'_>,
    (identity, mode): (&Identity, Mode<'_>),
) -> Result<(Held, usize, Vec<i64>), Failure> {
    let parts = month_parts(plan, identity, timeframe, month);
    let key = EntryKey {
        contract: plan.contract,
        exchange: identity.exchange,
        segment: identity.segment,
        symbol: identity.symbol,
        timeframe,
        month,
    };
    let written = match mode {
        Mode::Append => write_and_count(slice, store_root, identity.symbol_id, parts, key)
            .map(|(held, written)| (held, written, Vec::new())),
        Mode::Repair { reason } => repair_or_append(
            slice,
            store_root,
            identity.symbol_id,
            (parts, key),
            (repair_policy(plan), reason),
        ),
    };
    written.map_err(|why| {
        Failure::new(
            member.instrument.clone(),
            format!(
                "{}: {month}: {why}; this month only: the member's other months land and are counted",
                member.instrument
            ),
        )
        .in_month(month)
    })
}

/// The path parts of one month of this plan's instrument at `timeframe`.
fn month_parts<'a>(
    plan: Plan<'a>,
    identity: &'a Identity,
    timeframe: Timeframe,
    month: store::path::YearMonth,
) -> PathParts<'a> {
    PathParts {
        vendor: plan.vendor,
        exchange: identity.exchange.as_str(),
        segment: identity.segment.as_str(),
        symbol: identity.symbol.as_str(),
        contract: plan.contract,
        timeframe,
        month,
        file: FileKind::Bars,
    }
}

/// The days of the months a member's landing refused that the store does not
/// hold, for the gap ledger (D-3351): a refused month is a month whose days
/// the vendor sent and the store lacks, and a ledger that audits only the
/// vendor's rows read them as held (DB-N22).
#[derive(Debug, Default)]
struct NotHeld {
    /// IST-local seconds of the offered bars of those days, in time order.
    seconds: Vec<i64>,
    /// Those days that lie before their month's first held bar, in order.
    fronts: Vec<u32>,
    /// The IST days a repair added bars on, in order.
    repaired: Vec<u32>,
}

impl NotHeld {
    /// Count the IST days of `added_at`, the stamps a repair added.
    fn repaired(&mut self, added_at: &[i64]) {
        for &ts in added_at {
            if let Ok(day) = u32::try_from(local_day(ts))
                && self.repaired.last() != Some(&day)
            {
                self.repaired.push(day);
            }
        }
    }

    /// Count a refused month's days, log its refusal, and hand it back.
    fn note_month(
        &mut self,
        refused: Failure,
        slice: &[store::format::Bar],
        (store_root, parts): (&Path, PathParts<'_>),
        symbol_id: u32,
    ) -> Failure {
        note_not_filed(&refused.instrument, "bars", &refused.why);
        not_held_days(
            slice,
            store_root,
            symbol_id,
            parts,
            &mut self.seconds,
            &mut self.fronts,
        );
        refused
    }
}

/// The days of a refused month's `slice` the store does NOT hold a bar on,
/// as IST-local seconds of their offered bars (appended to `seconds` in time
/// order), and of those, the days before the month's first held bar (appended
/// to `fronts`): the ledger's `refused_day` and `refused_front` (D-3351).
///
/// One lookup per day of the slice — at most 31 — through the month's own
/// time index; a month that will not open holds nothing to ask, and every day
/// of the slice is then not held.
///
/// **A HELD DAY'S OFFERED BAR THE STORE LACKS IS NOT HELD EITHER (FM-166,
/// R3-18).** A day the store holds SHORT — 2019-09-23 without 15:20 — whose
/// missing minute the vendor now serves was read as held, so the re-pull's
/// audit counted the served minute held and wrote no line: the minute the
/// vendor now publishes and the append door refused was named nowhere. Each
/// held day's offered bars are now merged against the day's stored stamps
/// (one more lookup, and one read per stored bar of the day, in time order:
/// O(the day's bars), the order the landing already read them in), and an
/// offered bar the store does not hold is a refused row's second, so the
/// ledger line names it `refused_day` with its minute. A record that will
/// not read leaves the rest of the day not held, never held by guess.
fn not_held_days(
    slice: &[store::format::Bar],
    store_root: &Path,
    symbol_id: u32,
    parts: PathParts<'_>,
    seconds: &mut Vec<i64>,
    fronts: &mut Vec<u32>,
) {
    let file = StorePath::new(parts)
        .ok()
        .and_then(|path| BarFile::open_existing(store_root, path, symbol_id).ok());
    let held = file.as_ref().map_or(0, |file| file.header().n_valid);
    let first_day = file
        .as_ref()
        .filter(|_| held > 0)
        .map(|file| local_day(file.header().first_ts_micros));
    let mut at = 0usize;
    while let Some(first) = slice.get(at) {
        let day = local_day(first.ts_micros);
        let mut end = at;
        while slice
            .get(end)
            .is_some_and(|bar| local_day(bar.ts_micros) == day)
        {
            end += 1;
        }
        let day_start = (day * 86_400 - crate::session::IST_OFFSET_SECS) * 1_000_000;
        let holds_day = file.as_ref().is_some_and(|file| {
            file.first_at_or_after(day_start)
                .ok()
                .filter(|&index| index < held)
                .and_then(|index| file.read_record(index).ok())
                .is_some_and(|bar| bar.ts_micros < day_start + 86_400 * 1_000_000)
        });
        let offered = slice.get(at..end).unwrap_or(&[]);
        match file.as_ref().filter(|_| holds_day) {
            None => {
                seconds.extend(offered.iter().map(|bar| local_second_of(bar.ts_micros)));
                if first_day.is_some_and(|first| day < first)
                    && let Ok(number) = u32::try_from(day)
                {
                    fronts.push(number);
                }
            }
            Some(file) => offered_not_stored(file, (held, day_start), offered, seconds),
        }
        at = end;
    }
}

/// The IST-local seconds of `offered` (one held day's bars, ascending) that
/// `file` does not hold, appended to `seconds` in time order: one walk of the
/// day's stored bars beside the offered ones (FM-166, R3-18).
fn offered_not_stored(
    file: &BarFile,
    (held, day_start): (u64, i64),
    offered: &[store::format::Bar],
    seconds: &mut Vec<i64>,
) {
    let day_end = day_start + 86_400 * 1_000_000;
    let at = |ts: i64| file.first_at_or_after(ts).ok().map(|index| index.min(held));
    let (Some(mut index), Some(end)) = (at(day_start), at(day_end)) else {
        seconds.extend(offered.iter().map(|bar| local_second_of(bar.ts_micros)));
        return;
    };
    let mut stored = None;
    for bar in offered {
        while index < end {
            let ts = if let Some(ts) = stored {
                ts
            } else {
                let Ok(record) = file.read_record(index) else {
                    // UNREADABLE: the rest of the day is not held.
                    index = end;
                    break;
                };
                *stored.insert(record.ts_micros)
            };
            if ts >= bar.ts_micros {
                break;
            }
            index += 1;
            stored = None;
        }
        if index >= end || stored != Some(bar.ts_micros) {
            seconds.push(local_second_of(bar.ts_micros));
        }
    }
}

/// The IST-local epoch second of a bar's stamp.
const fn local_second_of(ts_micros: i64) -> i64 {
    ts_micros.div_euclid(1_000_000) + crate::session::IST_OFFSET_SECS
}

/// The IST day number of a bar's stamp.
const fn local_day(ts_micros: i64) -> i64 {
    local_second_of(ts_micros).div_euclid(86_400)
}

/// How a repair compares an offered bar with a held one, from what the
/// instrument IS (D-3350): open interest compared only where it is a
/// measurement (D-3322), and a held bar required in the offered window where
/// the feed adjusts its prices as of the pull date (D-3323). A feed whose
/// basis for the segment nobody recorded is held to the witness too.
fn repair_policy(plan: Plan<'_>) -> store::repair::Policy {
    let segment = plan.request.listing.segment();
    let mut adjusted = true;
    for feed in crate::vendor::Feed::ALL {
        if feed.store_vendor() == Some(plan.vendor)
            && let Some(row) = feed.descriptor().basis.segment(segment)
        {
            adjusted = row.price.adjusted_as_of_pull_date();
        }
    }
    store::repair::Policy {
        open_interest_applies: crate::vendor::open_interest_applies(segment),
        basis_witness: adjusted,
    }
}

/// THE REPAIR DOOR (D-3350): the append door first, and a versioned repair of
/// the month when the append door refuses it.
///
/// A new month, a tail and a batch already held go through
/// [`write_and_count`] exactly as an ordinary pull. A batch the append-only
/// month refuses — offered bars inside its held range at stamps it never held
/// (the DB-N20 holes), or a month already served from a revision — is handed
/// to [`store::repair::repair_month`], which publishes a new generation with
/// every held bar and every offered one and promotes it. The census row is
/// the served month's header. Each repair is journaled.
fn repair_or_append(
    bars: &[store::format::Bar],
    store_root: &Path,
    symbol_id: u32,
    (parts, key): (PathParts<'_>, EntryKey),
    (policy, reason): (store::repair::Policy, &str),
) -> Result<(Held, usize, Vec<i64>), String> {
    match write_and_count(bars, store_root, symbol_id, parts, key) {
        Ok((held, written)) => Ok((held, written, Vec::new())),
        Err(appended) => repair_after(
            &appended,
            bars,
            (store_root, symbol_id),
            (parts, key),
            (policy, reason),
        ),
    }
}

/// The repair half of [`repair_or_append`]: the month the append door
/// refused, in `appended`'s words, handed to
/// [`store::repair::repair_month`], journaled either way. Shared by the
/// repair door and the back-fill door ([`back_fill_or_append`]), so there is
/// one repair, one journal line and one refusal sentence.
fn repair_after(
    appended: &str,
    bars: &[store::format::Bar],
    (store_root, symbol_id): (&Path, u32),
    (parts, key): (PathParts<'_>, EntryKey),
    (policy, reason): (store::repair::Policy, &str),
) -> Result<(Held, usize, Vec<i64>), String> {
    let path = StorePath::new(parts).map_err(|why| why.to_string())?;
    match store::repair::repair_month(store_root, path, symbol_id, bars, policy) {
        Ok(store::repair::MonthRepair::NothingNew { .. }) => Ok((
            held_of_served(store_root, path, symbol_id, key)?,
            0,
            Vec::new(),
        )),
        Ok(store::repair::MonthRepair::Repaired {
            from,
            to,
            held,
            added,
            added_at,
            header,
            published,
            skipped,
            ..
        }) => {
            let journaled = journal_repair(
                store_root,
                &parts,
                (from, to.ordinal(), held, added),
                (&header, published, &skipped),
                reason,
            );
            let held_now = held_of_served(store_root, path, symbol_id, key)?;
            journaled.map(|()| {
                (
                    held_now,
                    usize::try_from(added).unwrap_or(usize::MAX),
                    added_at,
                )
            })
        }
        Err(why) => {
            let refused = format!("{appended}; the versioned repair refused it too: {why}");
            match journal_refusal(store_root, &parts, &why.to_string(), reason) {
                Ok(()) => Err(refused),
                Err(journal) => Err(format!(
                    "{refused}; and the repair journal line was not written: {journal}"
                )),
            }
        }
    }
}

/// The census row of the month as every reader now opens it: its served
/// generation's header and closes.
fn held_of_served(
    store_root: &Path,
    path: StorePath<'_>,
    symbol_id: u32,
    key: EntryKey,
) -> Result<Held, String> {
    let file =
        BarFile::open_existing(store_root, path, symbol_id).map_err(|why| why.to_string())?;
    let header = file.header();
    let closes = month_closes(&file, &header, &[])?;
    Ok(Held::new(
        Entry {
            key,
            rows: header.n_valid,
            first_ts_micros: header.first_ts_micros,
            last_ts_micros: header.last_ts_micros,
        },
        closes,
    ))
}

/// What the served month holds of an offered batch, as the back-fill door
/// asks it ([`lack_of`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Lack {
    /// The month holds no bar anywhere in the batch's span, and no append can
    /// ever land the batch: its span starts at or before the last stamp an
    /// original month holds, or the month is served from revision `served`
    /// (`0` the original), which is never appended to. Only a repair lands it.
    Lacked {
        /// The generation served now.
        served: u32,
    },
    /// The month holds a bar inside the batch's span, at `held_at`: the batch
    /// is another reading of bars the month has, not bars it lacks.
    Overlaps {
        /// The first held stamp at or after the batch's first.
        held_at: i64,
    },
    /// Nothing a repair answers: the month does not exist or holds nothing
    /// (the append door creates and fills it), the batch lies wholly after
    /// what an original month holds (the append door appends it), or the
    /// batch is not in ascending order (no door takes it). Whatever refused
    /// the append is not the month's shape.
    AppendDoors,
}

/// What the month at `path`, as every reader opens it, holds of `bars`.
///
/// # Cost
///
/// One pass over the batch's stamps (its order), one generation probe, one
/// read-only open, one time-indexed lookup (`BarFile::first_at_or_after`,
/// O(log n) at worst) and one record read. On the refusal path of the append
/// door only.
///
/// # Errors
///
/// The store's words for a pointer, an open or a read it refuses (a month a
/// writer holds locked among them): the month could not be judged.
fn lack_of(
    store_root: &Path,
    path: StorePath<'_>,
    symbol_id: u32,
    bars: &[store::format::Bar],
) -> Result<Lack, String> {
    let Some((first, last)) = bars.first().zip(bars.last()) else {
        return Ok(Lack::AppendDoors);
    };
    // A SPAN IS FIRST TO LAST ONLY OF AN ORDERED BATCH. One that is not is
    // the store's refusal, by name, at every door.
    if bars
        .iter()
        .zip(bars.iter().skip(1))
        .any(|(earlier, later)| earlier.ts_micros >= later.ts_micros)
    {
        return Ok(Lack::AppendDoors);
    }
    let served = store::generation::served(store_root, path).map_err(|why| why.to_string())?;
    let file = match BarFile::open_existing(store_root, path, symbol_id) {
        Ok(file) => file,
        Err(store::file::StoreError::Missing { .. }) => return Ok(Lack::AppendDoors),
        Err(why) => return Err(why.to_string()),
    };
    let header = file.header();
    if header.n_valid == 0 {
        return Ok(Lack::AppendDoors);
    }
    let original = served.ordinal() == 0;
    if original && first.ts_micros > header.last_ts_micros {
        return Ok(Lack::AppendDoors);
    }
    let at = file
        .first_at_or_after(first.ts_micros)
        .map_err(|why| why.to_string())?;
    if at < header.n_valid {
        let held = file.read_record(at).map_err(|why| why.to_string())?;
        if held.ts_micros <= last.ts_micros {
            return Ok(Lack::Overlaps {
                held_at: held.ts_micros,
            });
        }
    }
    Ok(Lack::Lacked {
        served: served.ordinal(),
    })
}

/// THE BACK-FILL DOOR (D-3350; DB-N26, WP-12d): the append door first, and a
/// versioned repair only of a month that LACKS the whole batch where no
/// append can ever land it ([`Lack::Lacked`]). [`repair_rows`] says why the
/// rule is this narrow; in short, every GDFL file comes through here, so a
/// refusal that is not the month's shape (a lock, the host, a held day read
/// differently) must stay a refusal and never become a generation.
///
/// The repair itself is [`repair_after`], the one WP-13c built; this door
/// adds the judgement before it and the `repairing` line.
fn back_fill_or_append(
    bars: &[store::format::Bar],
    store_root: &Path,
    symbol_id: u32,
    (parts, key): (PathParts<'_>, EntryKey),
    (policy, reason): (store::repair::Policy, &str),
) -> Result<(Held, usize, Filed), String> {
    let appended = match write_and_count(bars, store_root, symbol_id, parts, key) {
        Ok((held, written)) => return Ok((held, written, Filed::Appended)),
        Err(why) => why,
    };
    let path = StorePath::new(parts).map_err(|why| why.to_string())?;
    let served = match lack_of(store_root, path, symbol_id, bars) {
        Ok(Lack::Lacked { served }) => served,
        // THE APPEND DOOR'S REFUSAL, UNCHANGED: nothing about the month's
        // shape asks for a repair.
        Ok(Lack::AppendDoors) => return Err(appended),
        Ok(Lack::Overlaps { held_at }) => {
            return Err(format!(
                "{appended}; not back-filled: the month holds a bar at {held_at}, inside the offered span, and a back-fill adds only bars the month holds nothing of (D-3350): it never completes, restates or mixes a held day, so nothing was written"
            ));
        }
        Err(why) => {
            return Err(format!(
                "{appended}; not back-filled: the month as served could not be read to judge it ({why}), so nothing was written"
            ));
        }
    };
    // JOURNALED BEFORE ANYTHING IS PUBLISHED, so a generation is never served
    // that the journal does not name, whatever kills the process after it.
    journal_intent(store_root, &parts, served, bars, reason).map_err(|why| {
        format!(
            "{appended}; not back-filled: the repair journal would not take the `repairing` line ({why}), so nothing was published"
        )
    })?;
    repair_after(
        &appended,
        bars,
        (store_root, symbol_id),
        (parts, key),
        (policy, reason),
    )
    .map(|(held, added, added_at)| {
        let filed = if added_at.is_empty() {
            Filed::Appended
        } else {
            Filed::NewGeneration
        };
        (held, added, filed)
    })
}

/// One `repairing` line in `<store>/audit/<vendor>.repairs`, written by the
/// back-fill door BEFORE the repair: the month, the generation served, the
/// bars offered and the stamps they span, and why. The `repair` or `refused`
/// line that follows closes it; a `repairing` line with neither is a process
/// that died in between, and the month's pointer says whether its generation
/// was promoted.
fn journal_intent(
    store_root: &Path,
    parts: &PathParts<'_>,
    served: u32,
    bars: &[store::format::Bar],
    reason: &str,
) -> Result<(), String> {
    let (first, last) = bars
        .first()
        .zip(bars.last())
        .map_or((0, 0), |(first, last)| (first.ts_micros, last.ts_micros));
    let line = format!(
        "repairing {} {} {} v=1 from_generation={served} offered={} first={first} last={last} reason={} run={}",
        ledger_key(parts),
        parts.timeframe.as_str(),
        parts.month,
        bars.len(),
        ledger_field(reason),
        telemetry::global().map_or(0, telemetry::Sink::run)
    );
    request_minutes::append_named(store_root, parts.vendor, "repairs", &[line])
}

/// The census key text a ledger line names a month by.
fn ledger_key(parts: &PathParts<'_>) -> String {
    match parts.contract {
        Some(contract) => format!(
            "{}-{}-{}-{contract}",
            parts.exchange, parts.segment, parts.symbol
        ),
        None => format!("{}-{}-{}", parts.exchange, parts.segment, parts.symbol),
    }
}

/// A reason as one ledger field: every byte outside `[A-Za-z0-9_.:/-]`
/// becomes `_`, so the line keeps D-3317's one-space field rule. The whole
/// reason is on the telemetry event beside it.
fn ledger_field(text: &str) -> String {
    let field: String = text
        .chars()
        .take(160)
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | ':' | '/' | '-') {
                c
            } else {
                '_'
            }
        })
        .collect();
    if field.is_empty() {
        "-".to_owned()
    } else {
        field
    }
}

/// One `repair` line in `<store>/audit/<vendor>.repairs` and one `pull.repair`
/// event (D-3350): the month, the generation it replaced and the one now
/// served, the bars kept and added, the stamps it now spans, whether the
/// revision was written now or found complete (a resumed crash), the ordinals
/// stepped over, and why.
fn journal_repair(
    store_root: &Path,
    parts: &PathParts<'_>,
    (from, to, held, added): (u32, u32, u64, u64),
    (header, published, skipped): (&Header, store::repair::Published, &[(u32, String)]),
    reason: &str,
) -> Result<(), String> {
    let skipped_field = if skipped.is_empty() {
        "-".to_owned()
    } else {
        skipped
            .iter()
            .map(|(ordinal, _)| ordinal.to_string())
            .collect::<Vec<_>>()
            .join(",")
    };
    let published_word = match published {
        store::repair::Published::Created => "created",
        store::repair::Published::Reused => "reused",
    };
    let line = format!(
        "repair {} {} {} v=1 from_generation={from} to_generation={to} held={held} added={added} rows={} first={} last={} revision={published_word} skipped={skipped_field} reason={} run={}",
        ledger_key(parts),
        parts.timeframe.as_str(),
        parts.month,
        header.n_valid,
        header.first_ts_micros,
        header.last_ts_micros,
        ledger_field(reason),
        telemetry::global().map_or(0, telemetry::Sink::run)
    );
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::info("pull.repair", "month repaired to a new generation")
            .with("instrument", telemetry::Value::Str(parts.symbol))
            .with("feed", telemetry::Value::Str(parts.vendor.as_str()))
            .with("month", telemetry::Value::Str(&parts.month.to_string()))
            .with("rung", telemetry::Value::Str(parts.timeframe.as_str()))
            .with("from_generation", telemetry::Value::Uint(u64::from(from)))
            .with("to_generation", telemetry::Value::Uint(u64::from(to)))
            .with("held", telemetry::Value::Uint(held))
            .with("added", telemetry::Value::Uint(added))
            .with("skipped", telemetry::Value::Str(&skipped_field))
            .with("reason", telemetry::Value::Str(reason)),
    );
    request_minutes::append_named(store_root, parts.vendor, "repairs", &[line]).map_err(|why| {
        note_not_filed(parts.symbol, "repair journal", &why);
        format!(
            "{}: repaired to generation {to} and served, but its journal line was not written: {why}",
            parts.month
        )
    })
}

/// One `refused` line in the repair journal and one `pull.repair` error
/// event: a month the repair door would not take, and why.
fn journal_refusal(
    store_root: &Path,
    parts: &PathParts<'_>,
    why: &str,
    reason: &str,
) -> Result<(), String> {
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::error("pull.repair", "month repair refused")
            .with("instrument", telemetry::Value::Str(parts.symbol))
            .with("feed", telemetry::Value::Str(parts.vendor.as_str()))
            .with("month", telemetry::Value::Str(&parts.month.to_string()))
            .with("rung", telemetry::Value::Str(parts.timeframe.as_str()))
            .with("why", telemetry::Value::Str(why))
            .with("reason", telemetry::Value::Str(reason)),
    );
    let line = format!(
        "refused {} {} {} v=1 why={} reason={} run={}",
        ledger_key(parts),
        parts.timeframe.as_str(),
        parts.month,
        ledger_field(why),
        ledger_field(reason),
        telemetry::global().map_or(0, telemetry::Sink::run)
    );
    request_minutes::append_named(store_root, parts.vendor, "repairs", &[line])
}

/// Every refusal a landing made before any file was opened: each day of
/// unmeasured length (D-3344) and each (cause, day) of rows refused by name
/// (D-3351). Carried on every return of [`one`], so an answer that stored
/// nothing still says why.
fn landing_failures(
    instrument: &str,
    landed: &fetch::Landed,
    encoding: TimestampEncoding,
) -> Vec<Failure> {
    let mut failures = refused_day_failures(instrument, &landed.refused_days);
    failures.extend(refused_row_failures(instrument, &landed.refused, encoding));
    failures
}

/// One named failure per day [`fetch::land_rows`] refused, logged and returned
/// (D-3344).
///
/// The sentence says what was refused, why, that it was that day alone, and
/// what the remedy costs: the month goes on to hold the days after it, so
/// adding the refused minutes once the session is measured is a versioned store
/// repair, not a re-pull. One line per refused day, and at most five such days
/// exist (`calendar::DayKind::OpenLengthUnmeasured`).
fn refused_day_failures(instrument: &str, days: &[fetch::RefusedDay]) -> Vec<Failure> {
    days.iter()
        .map(|refused| {
            let civil = crate::session::Day::from_days(refused.day).ok();
            let day = civil.map_or_else(|| format!("day {}", refused.day), |day| day.to_string());
            let why = format!(
                "{day}: {} minute row(s) refused: the exchange calendar records a traded session on this day whose length was never measured (a Muhurat), so no bar on it can be judged in or out of session; this day only, the rest of the window lands; measure the session and record it in the calendar's irregular table, after which adding these minutes to a month that holds later days is a versioned store repair, not a re-pull",
                refused.rows
            );
            note_not_filed(instrument, "unmeasured session day", &why);
            // THE DAY IT IS ABOUT, carried (DB-G45): the Muhurat of 2020-11-14
            // was filed under the window's first month, 2018-01.
            let failure = Failure::new(instrument, why);
            match civil {
                Some(civil) => failure.on_day(civil),
                None => failure,
            }
        })
        .collect()
}

fn one(
    member: &Member,
    store_root: &Path,
    plan: Plan<'_>,
    mode: Mode<'_>,
) -> Result<Landed, String> {
    let Plan {
        request,
        encoding,
        scale,
        vendor,
        exchange,
        segment,
        ..
    } = plan;
    // THE ONE CONVERSION FROM RUNG TO DIRECTORY. See `Plan::timeframe`.
    let timeframe = plan.timeframe()?;
    // BORROWED, NOT CLONED. o1api-36, D-1203: this used to copy every row
    // into a `RawWindow` only to pass a reference to it.
    let mut landed = fetch::land_rows(&member.rows, request, encoding, scale, plan.cash_schedule)
        .map_err(|why| why.to_string())?;
    // Bound once so the three returns below cannot disagree. See the field.
    let outside_session = landed.outside_session;
    // EACH REFUSED DAY IS NAMED, AND THAT DAY IS ALL THAT IS LOST (D-3344).
    // Carried on every return below, so a window holding nothing but the
    // refused day still says why it stored nothing.
    let mut failures = landing_failures(&member.instrument, &landed, encoding);
    let mut refused_seconds = vec![std::mem::take(&mut landed.refused.local_seconds)];

    // THE FOLD. Without it a real run produced 354,675 rows and ZERO bars:
    // both archive vendors ship one-second snapshots with two to four rows per
    // second and no sub-second field, and the store correctly refuses two
    // records claiming the same instant.
    //
    // The bucket width comes from the TIMEFRAME the caller is filing under, so
    // a one-second feed, a one-minute feed and a daily feed all fold through
    // this same line. Nothing here knows which vendor it came from.
    let folded = fold_landed(&mut landed.bars, timeframe, member)?;

    // The month comes from the FIRST surviving bar. A member whose bars cross a
    // month boundary would need two files, and that split is a decision about
    // paths rather than about bars — so it is refused here by name rather than
    // silently filing December into November.
    //
    // THE FOLD RATIO, ONCE — never per snapshot. This is the number that
    // would have named D-0070 on the day it landed: a daily pull folding N
    // rows into N bars means the vendor already sent one bar per day, and the
    // bucket is only ever re-stamping them. `rows_folded = 0` on a 1day rung
    // is the signature, and nothing wrote it down.

    // NOTHING TO FILE — an answer of refused or dropped rows, or nothing at
    // all — still carries why: every refused day and row is on `failures`.
    // (`fold` returns at least one bar for an input that had at least one,
    // so a folded batch never reaches this arm.)
    if landed.bars.is_empty() {
        return Ok(nothing_landed(
            landed.census,
            folded,
            outside_session,
            (failures, refused_seconds),
        ));
    }
    // ONE RUN PER MONTH, RATHER THAN A REFUSAL IF THE BARS CROSS ONE.
    //
    // This was `month_of(first, last)?` — a batch spanning two months was an
    // error, so `session::split_window` clamped every fetch to a month end and
    // the day pass issued 81 requests where the vendor's 2,000-day cap needs 2.
    // See `months_in` for the measurement.
    let by_month = months_in(&landed.bars)?;
    // THE MEMBER'S IDENTITY, PARSED BEFORE ANY FILE IS OPENED. See `identify`.
    let identity = identify(&member.instrument, exchange, segment)?;
    let Identity {
        symbol,
        exchange,
        segment,
        symbol_id,
    } = identity;

    // ONE WRITE PER MONTH, in one pass over a batch that may now span many.
    let mut entries = Vec::new();
    let mut committed = 0usize;
    let mut months_written = 0usize;
    // Bars of months whose write was refused: offered, never stored.
    let mut refused_bars = 0usize;
    // The days of refused months the store does not hold, for the ledger.
    let mut not_held = NotHeld::default();
    for (ym, slice) in &by_month {
        // ONE MONTH'S REFUSAL IS THAT MONTH'S (D-3345). See `write_month`.
        let written = write_month(
            slice,
            *ym,
            timeframe,
            member,
            store_root,
            plan,
            (&identity, mode),
        );
        let (pulled, wrote) = match written {
            Ok((held, wrote, added_at)) => {
                // THE DAYS A REPAIR FILLED, for the ledger (D-3350): their
                // earlier `refused_*` lines are superseded by a `repaired` one.
                not_held.repaired(&added_at);
                (held, wrote)
            }
            Err(refused) => {
                refused_bars = refused_bars.saturating_add(slice.len());
                let at = (store_root, month_parts(plan, &identity, timeframe, *ym));
                failures.push(not_held.note_month(refused, slice, at, symbol_id));
                continue;
            }
        };
        entries.push(pulled);
        months_written = months_written.saturating_add(1);
        // SUMMED ACROSS MONTHS, because it is a COUNT of bars written and not
        // a flag. `Ingested::bars_stored` reads it, and a batch of eighty
        // months reporting one month's figure would under-report the run by
        // seventy-nine — a number an operator checks against the vendor's.
        committed = committed.saturating_add(wrote);

        // NO `if timeframe == MINUTE_1` GATE. Whatever rung was pulled,
        // everything derivable FROM it is derived — the rule decides, not a
        // condition here. A daily pull derives nothing because nothing coarser
        // is a whole multiple of it that is not itself the day. See
        // `derived_from`.
        //
        // Per month, because `DeriveInto::month` names one file and the
        // derived rungs of January must not land in February's.
        let into = DeriveInto {
            calendar: plan.calendar,
            cash_schedule: plan.cash_schedule,
            // THE CONTRACT THE BARS WERE FILED UNDER, which is what
            // `derive_all` reads its option/future line off. Leaving this
            // `None` would file an option's derived rungs in the
            // underlying's own directory.
            contract: plan.contract,
            vendor,
            exchange,
            segment,
            symbol,
            month: *ym,
        };
        failures.extend(derive_month(
            mode,
            (slice, &member.instrument),
            (store_root, symbol_id, timeframe),
            (into, plan),
            &mut entries,
        ));
    }
    refused_seconds.push(not_held.seconds);

    // PER MONTH ON BOTH SIDES. `entries` now holds one bars entry per month
    // plus that month's derived rungs, so the subtrahend is the number of
    // months written rather than the constant 1 it was when a batch was one
    // month — and the expectation is multiplied to match. Left as they were, an
    // eighty-month batch would report eighty times the derived rungs it
    // expected and every run would read as wrong.
    let derived = entries.len().saturating_sub(months_written);
    // THE RULE, NOT A LIST. `derived_count_in` is what `derive_rungs` walks, so
    // a rung added to `Timeframe::KNOWN` moves both sides at once and this
    // expectation cannot go stale against it. It already knows the one case
    // where zero is correct: an option contract folds into no rung.
    let derived_expected =
        derived_count_for(plan.vendor, plan.contract, timeframe).saturating_mul(months_written);
    Ok(Landed {
        failures,
        // OFFERED TO A MONTH THAT TOOK THEM: a refused month's bars are named
        // in `failures` and are not stored bars (D-3345).
        bars: landed.bars.len().saturating_sub(refused_bars),
        // WRITTEN, AS DISTINCT FROM OFFERED. See `Ingested::bars_stored`.
        committed,
        folded,
        census: landed.census,
        entries,
        derived,
        derived_expected,
        outside_session,
        refused_seconds,
        fronts: not_held.fronts,
        repaired_days: not_held.repaired,
    })
}

/// The month a batch of bars belongs to, or a refusal if they span two.
///
/// The month comes from the FIRST surviving bar. A member whose bars cross a
/// month boundary would need two files, and that split is a decision about
/// paths rather than about bars — so it is refused here by name rather than
/// silently filing December into November.
///
/// # Errors
///
/// A timestamp that is not a moment on this calendar, or a batch that spans two
/// months, naming both.
/// Splits a sorted batch into one contiguous run per calendar month.
///
/// # Why this exists, and what it is worth
///
/// The store addresses **one month per file**, so a write is per month. The
/// FETCH does not have to be, and coupling the two cost forty requests out of
/// every forty-one.
///
/// `session::split_window` ended every chunk at the earlier of the vendor's day
/// cap and the month's last day, because [`one`] refused a batch spanning two
/// months. Zerodha's documented cap is **2,000 days for `day` and 60 for
/// `minute`** (`Zerodha Docs/13-historical.md`, Appendix A). Over the
/// operator's 2,460-day window that is **2 requests** at day level and **41** at
/// minute level — and the month rule forced **81** at both.
///
/// Measured on the operator's own run: 0.35 s per instrument-month against a
/// 3-requests-per-second ceiling, 30 s per instrument, ~94 minutes for a day
/// pass that needs ~2.3. The local cost of splitting instead is 242 ms per
/// instrument, benchmarked over 81 fsynced month files.
///
/// # It splits DECODED bars, never raw rows
///
/// A raw row's timestamp is meaningless until [`crate::fetch::land`] has
/// dispatched on the vendor's [`crate::fetch::TimestampEncoding`]. Partitioning
/// before that would mean a second implementation of the conversion, and the
/// two disagreeing files bars under the wrong month — which is worse than the
/// requests it saves, and invisible until a backtest reads the wrong window.
///
/// # A run, not a group
///
/// The batch is already sorted by timestamp — `land` asserts it — so a month's
/// bars are contiguous and this is one pass with no map and no allocation per
/// bar. `CLAUDE.md` §3 rule 4.
///
/// # Errors
///
/// A timestamp that is not a moment on the IST calendar, from [`month_of`].
fn months_in(
    bars: &[store::format::Bar],
) -> Result<Vec<(store::path::YearMonth, &[store::format::Bar])>, String> {
    let mut out: Vec<(store::path::YearMonth, &[store::format::Bar])> = Vec::new();
    let mut start = 0usize;
    while start < bars.len() {
        let Some(first) = bars.get(start) else { break };
        let ym = month_at(first)?;
        let mut end = start;
        while let Some(bar) = bars.get(end) {
            if month_at(bar)? != ym {
                break;
            }
            end = end.saturating_add(1);
        }
        let Some(slice) = bars.get(start..end) else {
            break;
        };
        out.push((ym, slice));
        start = end;
    }
    Ok(out)
}

/// The month ONE bar falls in.
///
/// # Why this is separate from [`month_of`], which takes two
///
/// `months_in` groups a batch by month and asked `month_of(bar, bar)` for every
/// bar in it. That function exists to check a SPAN — it converts both ends and
/// compares them — so passing one bar twice did **two** `IstMoment`
/// conversions and two `year_month` calls to answer a question with one end,
/// and then compared the answer with itself.
///
/// Constant per bar either way, and constant work nobody needed: a backfill
/// that lands 1.6 million bars did 3.2 million conversions to make 1.6 million
/// decisions. The same shape as the per-record allocation D-0366 removed from
/// `store::file::read_row`, and invisible to the same gates for the same
/// reason — a ratio cannot see work that is doubled uniformly.
///
/// [`month_of`] keeps the span check, which is a real refusal and has real
/// callers: a batch straddling a month boundary needs two files and splitting
/// it is the caller's decision.
///
/// # Errors
///
/// A timestamp outside the range [`crate::session::IstMoment`] can name, or a
/// day with no month in the store's addressing.
fn month_at(bar: &store::format::Bar) -> Result<store::path::YearMonth, String> {
    crate::session::IstMoment::from_epoch_secs(bar.ts_micros.div_euclid(1_000_000))
        .map_err(|why| why.to_string())?
        .day()
        .year_month()
        .map_err(|why| why.to_string())
}

fn month_of(
    first: &store::format::Bar,
    last: &store::format::Bar,
) -> Result<store::path::YearMonth, String> {
    let at = crate::session::IstMoment::from_epoch_secs(first.ts_micros.div_euclid(1_000_000))
        .map_err(|why| why.to_string())?;
    let ym = at.day().year_month().map_err(|why| why.to_string())?;
    let end = crate::session::IstMoment::from_epoch_secs(last.ts_micros.div_euclid(1_000_000))
        .map_err(|why| why.to_string())?;
    let end_ym = end.day().year_month().map_err(|why| why.to_string())?;
    if end_ym != ym {
        return Err(format!(
            "bars span {ym} to {end_ym}; the store addresses one month per \
             file and splitting is the caller's decision, not this one's"
        ));
    }
    Ok(ym)
}

/// A member's identity, parsed once and used by every file it writes.
#[derive(Clone, Copy)]
struct Identity {
    symbol: brutex_core::symbol::Symbol,
    exchange: Exchange,
    segment: Segment,
    symbol_id: u32,
}

/// Actual committed cash-minute dates in the given source months. This is the
/// same identity/path/header check used by derivation, and reads no other rung
/// or instrument. Missing files contribute no dates; corrupt files refuse.
/// No calendar dates are synthesized and no file is created or changed.
///
/// # Errors
/// Refuses invalid identity, unreadable source records or a timestamp outside
/// its addressed month. O(committed source rows), once per affected month.
pub fn committed_cash_days(
    root: &Path,
    vendor: Vendor,
    instrument: &str,
    months: &[store::path::YearMonth],
) -> Result<Vec<crate::session::Day>, String> {
    let identity = identify(instrument, "NSE", "CASH")?;
    let mut months = months.to_vec();
    months.sort_unstable();
    months.dedup();
    let mut days = std::collections::BTreeSet::new();
    for month in months {
        let path = StorePath::new(PathParts {
            vendor,
            exchange: "NSE",
            segment: "CASH",
            symbol: identity.symbol.as_str(),
            contract: None,
            timeframe: Timeframe::MINUTE_1,
            month,
            file: FileKind::Bars,
        })
        .map_err(|why| why.to_string())?;
        let file = match BarFile::open_existing(root, path, identity.symbol_id) {
            Ok(file) => file,
            Err(store::file::StoreError::Missing { .. }) => continue,
            Err(why) => {
                return Err(format!(
                    "{instrument} {month}: committed cash source: {why}"
                ));
            }
        };
        for index in 0..file.header().n_valid {
            let row = file
                .read_record(index)
                .map_err(|why| format!("{instrument} {month}: {why}"))?;
            let day =
                crate::session::IstMoment::from_epoch_secs(row.ts_micros.div_euclid(1_000_000))
                    .map_err(|why| why.to_string())?
                    .day();
            if day.year_month().map_err(|why| why.to_string())? != month {
                return Err(format!(
                    "{instrument} {month}: committed source timestamp belongs to {day}"
                ));
            }
            if crate::vendor::cash_auction_eligibility_required(day) {
                days.insert(day);
            }
        }
    }
    Ok(days.into_iter().collect())
}

/// Parses a member's symbol and venue, and derives the id the store stamps.
///
/// # Why this happens BEFORE a bar file is opened
///
/// `StorePath` accepts any upper-case segment, so a plan naming one the census
/// cannot key would write bars under a directory `/store` can never report on.
/// Refusing here means such a member stores nothing, rather than storing bars
/// nobody counts.
///
/// # Errors
///
/// The symbol's refusal or the venue's, each naming the instrument.
fn identify(instrument: &str, exchange: &str, segment: &str) -> Result<Identity, String> {
    let symbol = brutex_core::symbol::Symbol::new(instrument)
        .map_err(|why| format!("{instrument}: {why}"))?;
    let exchange = Exchange::parse(exchange)
        .map_err(|why| format!("{instrument}: exchange {exchange:?}: {why}"))?;
    // NSE ONLY ON THE WRITE PATH (GAP2-41, D-0955). `CLAUDE.md` §1: BSE is
    // not swept and not pulled (D-0017). Every caller of this function is about
    // to write bars or read the NSE cash rung, so a plan naming BSE stores
    // nothing. BSE files already on disk are untouched: reading them does not
    // come through here.
    if exchange != Exchange::Nse {
        return Err(format!(
            "{instrument}: exchange {}: not pulled; D-0017 narrows ingest to \
             NSE, so nothing is written for this member",
            exchange.as_str()
        ));
    }
    let segment = Segment::parse(segment)
        .map_err(|why| format!("{instrument}: segment {segment:?}: {why}"))?;
    // The symbol id is a CROSS-CHECK the store stamps into the header and
    // verifies on every reopen -- never the index, which is arithmetic. Derived
    // from the name so the same instrument always yields the same id, because a
    // counter would give a different one on a rerun and CLAUDE.md §3 rule 5
    // requires the same inputs to give the same bytes.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the id is a CROSS-CHECK the store stamps in the header and \
                  verifies on reopen, never an index — any 32 bits of the hash \
                  serve, and taking the low half of a 64-bit FNV-1a is the \
                  standard folding. Derived from the name so a rerun yields the \
                  same id; a counter would not, and CLAUDE.md §3 rule 5 requires \
                  the same inputs to give the same bytes."
    )]
    let symbol_id = brutex_core::universe::fnv1a(symbol.as_str()) as u32;
    Ok(Identity {
        symbol,
        exchange,
        segment,
        symbol_id,
    })
}

/// Where a derived rung lands: everything about the member except its rung.
///
/// A struct rather than six parameters, because clippy's argument ceiling is
/// the same kind of rule as its line ceiling and six of these travel together
/// everywhere they go.
#[derive(Clone, Copy)]
struct DeriveInto<'a> {
    calendar: crate::calendar::Runtime<'a>,
    cash_schedule: Option<&'a crate::cash_auction::Schedule>,
    /// The contract these bars belong to, or `None` for spot.
    contract: Option<brutex_core::instrument::Contract>,
    vendor: Vendor,
    exchange: Exchange,
    segment: Segment,
    symbol: brutex_core::symbol::Symbol,
    month: store::path::YearMonth,
}

/// Folds the minute bars into every rung derived from them and appends each.
///
/// # THE OPERATOR'S REQUIREMENT, stated from his first message
///
/// One minute is what gets bought and pulled, and 2, 3, 5, 10, 15, 30 and 60
/// minutes are DERIVED from it internally so `/db` shows every one. Until this,
/// a one-minute pull wrote `1min/` and stopped — the store shipped directories
/// for the coarser rungs and nothing ever put a bar in one.
///
/// # Why it is sound, and none of these three is an assumption
///
/// **The arithmetic.** [`crate::fold`] already aggregates OHLCV correctly: open
/// is the first, close the last, high and low the extremes, volume the SUM and
/// open interest the LAST non-null. Volume is a flow and open interest is a
/// level — summing the second would report 187 million where 500,177 is true —
/// and the fold has had that right all along.
///
/// **The alignment.** Every intraday rung is anchored at the SESSION OPEN, so
/// each derived bar begins the day at 09:15 exactly. That is what made 2, 10,
/// 30 and 60 filable at all; see [`crate::fold`] and `pull::tests::anchor`.
///
/// **The multiple.** Every rung here is a whole multiple of one minute, so no
/// bucket ever straddles a source bar.
///
/// # Always from the minute, never from a coarser derived rung
///
/// Folding 5min into 15min gives the same answer today and makes every rung
/// depend on the one before it: an error compounds, and no rung can be rebuilt
/// without rebuilding its ancestors. One step from the source, every time.
///
/// # A derived rung that fails does not fail the member
///
/// The minute bars are already committed and are the ones that were paid for.
/// Refusing the whole member because a fold could not be filed would throw away
/// the pull to protect a copy of it. Each refusal is NAMED on the log instead —
/// `CLAUDE.md` §4 requires it said, not that it be fatal.
fn derive_all(
    source_bars: &[store::format::Bar],
    instrument: &str,
    store_root: &Path,
    symbol_id: u32,
    source: Timeframe,
    into: DeriveInto<'_>,
    entries: &mut Vec<Held>,
) -> Vec<Failure> {
    let mut failures = Vec::new();
    // DERIVED RUNGS ARE FOR THE UNDERLYING SPOT, AND ONLY FOR IT.
    //
    // The operator's rule, 15 Aug 2026: the internal timeframes "should be
    // fully one and only applicable for these underlying spots alone". A
    // derivative is pulled at the rungs the vendor serves and at no others.
    //
    // Why that is the right rule and not merely the stated one: a folded bar is
    // an ARITHMETIC claim about a continuous series, and an option contract is
    // not one. It is born at its listing, dies at its expiry, and is illiquid at
    // both ends — folding thirty one-minute bars of which four traded produces a
    // bar that looks like a half-hour of trading and was nothing of the kind.
    // The spot index has no such gaps, which is why the fold is honest there.
    //
    // SPOT AND EXPIRED FUTURES DERIVE. EXPIRED OPTIONS DO NOT.
    //
    // The line is drawn where the arithmetic stops holding. A folded bar is a
    // claim about a CONTINUOUS series. A spot index is one, and so is a futures
    // contract: one contract per expiry, the front month carries the volume,
    // and it trades every minute of every session it is alive for.
    //
    // An option chain is not. Each expiry has hundreds of strikes, and away
    // from the money most of them do not trade for minutes at a time — so
    // folding thirty one-minute bars of which four traded yields a bar that
    // looks like half an hour of trading and was nothing of the kind. The gaps
    // are the instrument, not a defect in it.
    //
    // Read off the CONTRACT rather than the segment, because both derivative
    // legs are `Segment::Fno` and the segment therefore cannot tell them apart
    // — the same reason `api::ladder::Leg` exists. The suffix is the test:
    // `-FUT` for a future, a strike and a side for an option.
    // GDFL IS ONE SECOND AND NOTHING COARSER (operator, 4 Oct 2026, D-2802).
    // Minutes and every higher rung come from the Zerodha one-minute pulls,
    // so no rung is ever folded from a `gdfl` bar. See `derives_for`.
    if !derives_for(into.vendor) {
        return failures;
    }
    if into.contract.is_some_and(|c| c.is_option()) {
        return failures;
    }
    // Resume from the committed source month, not just this request's suffix.
    // This is O(month rows) once per batch; record reads remain positional.
    let history = if source == Timeframe::MINUTE_1 {
        let read = || -> Result<Vec<Bar>, String> {
            let path = StorePath::new(PathParts {
                vendor: into.vendor,
                exchange: into.exchange.as_str(),
                segment: into.segment.as_str(),
                symbol: into.symbol.as_str(),
                contract: into.contract,
                timeframe: source,
                month: into.month,
                file: FileKind::Bars,
            })
            .map_err(|why| why.to_string())?;
            let file = BarFile::open_existing(store_root, path, symbol_id)
                .map_err(|why| why.to_string())?;
            (0..file.header().n_valid)
                .map(|i| file.read_record(i).map_err(|why| why.to_string()))
                .collect()
        };
        match read() {
            Ok(bars) => Some(bars),
            Err(why) => {
                for rung in derived_from(source) {
                    note_not_derived(instrument, rung, &why);
                }
                failures.push(Failure::new(instrument, why).in_month(into.month));
                return failures;
            }
        }
    } else {
        None
    };
    if let Some(minutes) = history.as_deref() {
        check_day(minutes, instrument, store_root, symbol_id, &into);
    }
    let source_bars = history.as_deref().unwrap_or(source_bars);
    let mut ledger: Vec<String> = Vec::with_capacity(derived_count(source)); // D-3335
    for rung in derived_from(source) {
        let parts = PathParts {
            vendor: into.vendor,
            exchange: into.exchange.as_str(),
            segment: into.segment.as_str(),
            symbol: into.symbol.as_str(),
            // THE CONTRACT THESE BARS WERE FOLDED FROM, and both of these read
            // the literal `None`.
            //
            // `DeriveInto::contract` was added and then consulted ONLY by the
            // `is_option()` line above — so an OPTION returned early and was
            // safe, and a FUTURE walked straight past two hardcoded `None`s.
            //
            // What that filed: `NSE-NIFTY-25Sep25-FUT`'s seven coarser rungs
            // went to `bars/groww/NSE/FNO/NIFTY/2min/2025-09.bin` — a directory
            // naming no contract — under `EntryKey { contract: None, … }`. So
            // did October's and November's, because September, October and
            // November futures all trade every minute of September. The first
            // contract won the path; the other two hit the same file with the
            // same stamps and different prices, and `BarFile::append` refused
            // them as out of order.
            //
            // Nothing caught it because `StorePath::new` checks the contract
            // segment only when one is PRESENT: `NSE/FNO/NIFTY/2min/` is a
            // legal path, so it was created, appended to, sealed and counted.
            // HTTP 200, `derived_files: 7`, `balances()` true — and one
            // contract's coarse bars sitting at an address that belongs to no
            // contract at all, unrewritable under §8.
            contract: into.contract,
            timeframe: rung,
            month: into.month,
            file: FileKind::Bars,
        };
        let key = EntryKey {
            contract: into.contract,
            exchange: into.exchange,
            segment: into.segment,
            symbol: into.symbol,
            timeframe: rung,
            month: into.month,
        };
        match derive(
            source_bars,
            source,
            rung,
            store_root,
            symbol_id,
            parts,
            (key, into.cash_schedule, into.calendar),
        ) {
            Ok((held, diagnostics, line)) => {
                entries.push(held);
                ledger.extend(line);
                failures.extend(diagnostics.into_iter().map(|why| {
                    Failure::new(instrument, format!("{}: {why}", rung.as_str()))
                        .in_month(into.month)
                }));
            }
            Err(why) => {
                note_not_derived(instrument, rung, &why);
                // THE MONTH THE RUNG WAS FOLDED FOR (DB-G45): a derived-rung
                // failure for 2026-09 was filed under 2018-01.
                failures.push(
                    Failure::new(instrument, format!("{}: {why}", rung.as_str()))
                        .in_month(into.month),
                );
            }
        }
    }
    failures.extend(request_minutes::record_derived(
        store_root,
        into.vendor,
        instrument,
        into.month,
        &ledger,
    ));
    failures
}

/// The derived rungs of one landed month, by the landing's mode: appended
/// ([`derive_all`]), or — for a repair — folded again from the month as now
/// served and repaired the same way ([`derive_repair`], D-3350): a derived
/// month held around the hole cannot be appended to either.
fn derive_month(
    mode: Mode<'_>,
    (slice, instrument): (&[Bar], &str),
    (store_root, symbol_id, timeframe): (&Path, u32, Timeframe),
    (into, plan): (DeriveInto<'_>, Plan<'_>),
    entries: &mut Vec<Held>,
) -> Vec<Failure> {
    match mode {
        Mode::Append => derive_all(
            slice, instrument, store_root, symbol_id, timeframe, into, entries,
        ),
        Mode::Repair { reason } => derive_repair(
            instrument,
            store_root,
            symbol_id,
            timeframe,
            (into, repair_policy(plan), reason),
            entries,
        ),
    }
}

/// [`derive_all`] for a REPAIR (D-3350): every derived rung is folded again
/// from the source month AS IT IS NOW SERVED — the repaired generation — and
/// each rung month goes through the same repair door as its source: appended
/// where the append door takes it, repaired to a new generation where the
/// month holds bars around the days the fold now fills. A held derived bar is
/// never dropped or changed; one the fold would change refuses that rung by
/// name, and the source month's repair stands.
///
/// Spot and futures only, minute source only, never GDFL — the rules
/// [`derive_all`] states.
fn derive_repair(
    instrument: &str,
    store_root: &Path,
    symbol_id: u32,
    source: Timeframe,
    (into, policy, reason): (DeriveInto<'_>, store::repair::Policy, &str),
    entries: &mut Vec<Held>,
) -> Vec<Failure> {
    let mut failures = Vec::new();
    if !derives_for(into.vendor)
        || into.contract.is_some_and(|c| c.is_option())
        || source != Timeframe::MINUTE_1
    {
        return failures;
    }
    let source_parts = PathParts {
        vendor: into.vendor,
        exchange: into.exchange.as_str(),
        segment: into.segment.as_str(),
        symbol: into.symbol.as_str(),
        contract: into.contract,
        timeframe: source,
        month: into.month,
        file: FileKind::Bars,
    };
    let (minutes, venue) = match served_minutes(store_root, symbol_id, source_parts) {
        Ok(read) => read,
        Err(why) => {
            for rung in derived_from(source) {
                note_not_derived(instrument, rung, &why);
            }
            failures.push(Failure::new(instrument, why).in_month(into.month));
            return failures;
        }
    };
    let mut ledger: Vec<String> = Vec::with_capacity(derived_count(source));
    for rung in derived_from(source) {
        let parts = PathParts {
            timeframe: rung,
            ..source_parts
        };
        let key = EntryKey {
            contract: into.contract,
            exchange: into.exchange,
            segment: into.segment,
            symbol: into.symbol,
            timeframe: rung,
            month: into.month,
        };
        let Some(bucket) = crate::fold::Bucket::of_secs(rung.secs()) else {
            continue;
        };
        let folded =
            crate::fold::derive_minutes(&minutes, bucket, venue, into.cash_schedule, into.calendar);
        let crate::fold::Derived {
            bars,
            tally,
            diagnostics,
        } = match folded {
            Ok(folded) => folded,
            Err(why) => {
                let why = why.to_string();
                note_not_derived(instrument, rung, &why);
                failures.push(
                    Failure::new(instrument, format!("{}: {why}", rung.as_str()))
                        .in_month(into.month),
                );
                continue;
            }
        };
        failures.extend(diagnostics.into_iter().map(|why| {
            Failure::new(instrument, format!("{}: {why}", rung.as_str())).in_month(into.month)
        }));
        if bars.is_empty() {
            continue;
        }
        match repair_or_append(&bars, store_root, symbol_id, (parts, key), (policy, reason)) {
            Ok((held, _, _)) => {
                entries.push(held);
                ledger.push(request_minutes::derive_line(&parts, tally));
            }
            Err(why) => {
                note_not_derived(instrument, rung, &why);
                failures.push(
                    Failure::new(instrument, format!("{}: {why}", rung.as_str()))
                        .in_month(into.month),
                );
            }
        }
    }
    failures.extend(request_minutes::record_derived(
        store_root,
        into.vendor,
        instrument,
        into.month,
        &ledger,
    ));
    failures
}

/// Every bar of a source month as it is now served, and the venue its
/// minutes are judged complete against.
fn served_minutes(
    store_root: &Path,
    symbol_id: u32,
    parts: PathParts<'_>,
) -> Result<(Vec<Bar>, crate::vendor::Venue), String> {
    let venue = Exchange::parse(parts.exchange)
        .ok()
        .zip(Segment::parse(parts.segment).ok())
        .and_then(|(exchange, segment)| crate::vendor::Venue::for_segment(exchange, segment))
        .ok_or_else(|| {
            format!(
                "no verified minute-completeness venue for {}/{}; derived rungs not repaired",
                parts.exchange, parts.segment
            )
        })?;
    let path = StorePath::new(parts).map_err(|why| why.to_string())?;
    let file =
        BarFile::open_existing(store_root, path, symbol_id).map_err(|why| why.to_string())?;
    let minutes = (0..file.header().n_valid)
        .map(|i| file.read_record(i).map_err(|why| why.to_string()))
        .collect::<Result<Vec<Bar>, String>>()?;
    Ok((minutes, venue))
}

/// Folds a member's bars to the rung being filed under, in place, and answers
/// how many rows the fold absorbed.
///
/// # Why the fold exists at all
///
/// Without it a real run produced 354,675 rows and ZERO bars: both archive
/// vendors ship one-second snapshots with two to four rows per second and no
/// sub-second field, and the store correctly refuses two records claiming the
/// same instant. The fold is what turns a snapshot stream into bars.
///
/// The absorbed count is measured HERE, where both lengths are in hand. `fold`
/// emits one bar per bucket that held anything and never invents one, so the
/// subtraction cannot go negative.
///
/// # Errors
///
/// A rung whose bucket is zero seconds, or a fold the bar set refuses.
// A FOLD AT THE VENDOR'S OWN RUNG IS DUPLICATE STAMPS, AND IT IS STILL ONLY
// `Debug`. RECORDED, NOT FIXED.
//
// Coarsening is this function's job: a one-second archive filed as `1min` folds
// by design. But if the VENDOR was asked for one minute and the bars are filed
// at one minute, there is nothing to coarsen — every reduction is two rows
// sharing a timestamp, and `fold` MERGES them: high widened, low widened, close
// from the last, and **volume summed**. A vendor repeating 09:15 twice at 1,000
// lots each stores one bar of 2,000, `Ingested::balances` stays true, and the
// receipt reads clean. `ingest`'s own comment calls this *"the signature, and
// nothing wrote it down"*.
//
// **The rule cannot be written from here.** It needs "the rung the VENDOR was
// asked for", and `Plan::request.granularity` is not that — on the archive path
// it is the rung the OPERATOR wants the file written at, while the source is a
// one-second folder. A rule keyed on it warns on every archive pull, which is
// the noise §4 exists to prevent and strictly worse than this `Debug` line.
//
// Closing it means threading the source rung through `from_dir` and
// `from_window`, which know it and `Plan` does not. Named here rather than
// half-fixed. D-0354.
/// [`fold_in_place`] for an answer that may have landed no bar at all — every
/// row refused or dropped (D-3351) — which folds nothing.
fn fold_landed(
    bars: &mut Vec<store::format::Bar>,
    timeframe: Timeframe,
    member: &Member,
) -> Result<usize, String> {
    if bars.is_empty() {
        return Ok(0);
    }
    fold_in_place(bars, timeframe, member)
}

fn fold_in_place(
    bars: &mut Vec<store::format::Bar>,
    timeframe: Timeframe,
    member: &Member,
) -> Result<usize, String> {
    let bucket = crate::fold::Bucket::of_secs(timeframe.secs())
        .ok_or("a timeframe of zero seconds has no bucket to fold into")?;
    let snapshots = bars.len();
    *bars = crate::fold::fold(bars, bucket).map_err(|why| why.to_string())?;
    let folded = snapshots.saturating_sub(bars.len());
    note_fold(member, snapshots, bars.len(), folded, bucket);
    Ok(folded)
}

/// WHY `derived_count_in` IS HANDED THE CONTRACT, AND WHAT THE LITERAL COST.
///
/// The call in `one` passed `None` directly beneath the sentence naming the
/// case it exists for. So an OPTION expected seven derived rungs and folds into
/// none: `derived` was 0, `derived_expected` was 7, `derived_shortfall` fired,
/// and every contract of the month landed in `Ingested::failures`.
///
/// From outside that looked like: bars, census row and closes all correctly on
/// disk, and `/pull/fno` rendering "Bars stored: 0 · Contracts that did not
/// land: 200 of 200" with HTTP 502 "the month is incomplete and must not be
/// read as held". A correct month reported as a total failure.
///
/// It also drowned any REAL derived shortfall in two hundred false ones, which
/// is the more expensive half: `note_derived_shortfall` exists to catch a disk
/// that filled between the pulled rung landing and the folded ones.
///
/// How many rungs a file at `source` is folded into — COMPUTED, never listed.
///
/// # Why this is a rule and not an array
///
/// It was `[MINUTE_2, MINUTE_3, MINUTE_5, MINUTE_10, MINUTE_15, MINUTE_30,
/// MINUTE_60]`, and a hardcoded list is a second place to remember: a rung
/// added to `Timeframe::KNOWN` and forgotten here is a directory the store can
/// file and nothing ever writes to, silently, with no test able to see the
/// omission because both sides agree with themselves.
///
/// The operator's requirement is that the ladder scales — seconds today,
/// whatever is needed later — **without anyone editing a list**. So the set is
/// derived from `KNOWN` on the two properties that make a fold correct, and
/// adding a rung to the store is the whole of adding it to the ladder.
///
/// # The two conditions, and there are only two
///
/// **Strictly coarser.** A rung cannot be folded into itself or into anything
/// finer — the information is not there.
///
/// **A whole multiple.** `target.secs % source.secs == 0`, so no bucket ever
/// straddles a source bar. Ten minutes from one minute is ten whole bars; ten
/// minutes from three would be three and a third, and the third bar would be
/// split between two buckets with no way to divide its volume or decide which
/// one owns its high.
///
/// # The day is excluded, and that is D-0077 rather than arithmetic
///
/// `DAY_1` passes both tests — 86,400 is a whole multiple of 60 — and is still
/// not derived. A daily bar is what a VENDOR serves under its own convention:
/// Dhan opens it at the session's first print, Groww at the previous session's
/// close, measured 181 points apart on one instrument on one day. Folding one
/// from minutes would silently pick a convention and file it beside the other
/// vendor's. The day is served, never derived.
#[must_use]
pub fn derived_count(source: Timeframe) -> usize {
    derived_from(source).count()
}

/// The same count, for an instrument holding `contract`.
///
/// Zero for an OPTION, and the full set for spot and for a future. See
/// [`derive_all`] for where that line is drawn and why the arithmetic stops
/// holding on an option chain and not on a futures contract.
#[must_use]
pub fn derived_count_in(
    contract: Option<brutex_core::instrument::Contract>,
    source: Timeframe,
) -> usize {
    if contract.is_some_and(|c| c.is_option()) {
        0
    } else {
        derived_count(source)
    }
}

/// Whether any coarser rung is ever derived from a bar of `vendor`.
///
/// False for `gdfl` ALONE, by the operator's rule of 4 Oct 2026 (D-2802): GDFL
/// supplies the one-second rung only, and minutes and every higher timeframe
/// come from the Zerodha one-minute pulls, derived by the app from those. A
/// named rule, not a missing arm: a GDFL bar filed through any ingest door
/// is filed at its own rung and folded into nothing.
#[must_use]
pub const fn derives_for(vendor: Vendor) -> bool {
    !matches!(vendor, Vendor::Gdfl)
}

/// [`derived_count_in`] for a bar of `vendor`: zero for `gdfl` ([`derives_for`]).
#[must_use]
pub fn derived_count_for(
    vendor: Vendor,
    contract: Option<brutex_core::instrument::Contract>,
    source: Timeframe,
) -> usize {
    if derives_for(vendor) {
        derived_count_in(contract, source)
    } else {
        0
    }
}

/// [`derived_count`]'s set. Private because the rungs themselves are this
/// module's business; the COUNT is what a caller can meaningfully assert on.
fn derived_from(source: Timeframe) -> impl Iterator<Item = Timeframe> {
    Timeframe::KNOWN.iter().copied().filter(move |target| {
        target.secs() > source.secs()
            && target.secs().is_multiple_of(source.secs())
            && target.secs() != Timeframe::DAY_1.secs()
    })
}

/// Checks Zerodha's pulled day bars for the month against the days its own
/// minute bars fold to, and says what it found on the log. D-3001.
///
/// # Zerodha only, and why
///
/// The operator's rule of 4 Oct 2026 is a Zerodha rule: the day pass over the
/// whole span first, then the minute pass, then everything rebuilt from the
/// minutes. Groww's day bar opens at the previous session's close, measured
/// 181 points from Dhan's on one instrument on one day (D-0077), so checking it
/// would log every day as different and drown a real gap in convention noise.
///
/// **Zerodha's two answers do NOT agree day for day either**, which this said
/// they were meant to: measured on the operator's store, the pulled close is
/// not the last minute's close on almost every day, and a share's pulled volume
/// is above its minutes' sum on almost every day. So "differs" is not news and
/// is an `info` line with the count of each kind; only a pulled day that its
/// own minutes EXCEED (a minute above its high, below its low, or more volume
/// than it) is a `warn` line. D-3347.
///
/// # Never a failure, never a write
///
/// The minute bars are already committed and the day bars are what the vendor
/// served, so nothing here can make either more correct. Agreement and a
/// difference that contradicts nothing are `info` lines; a contradiction, a
/// minute day with no pulled day, or an unreadable day file is a `warn` line,
/// all under `pull.daycheck` on `/logs`. See [`crate::daycheck`].
///
/// # Cost
///
/// One read of the month's day file, `O(days)`, then [`crate::daycheck::compare`],
/// `O(minutes + days)`. Once per instrument-month after its minute bars land.
fn check_day(
    minutes: &[Bar],
    instrument: &str,
    store_root: &Path,
    symbol_id: u32,
    into: &DeriveInto<'_>,
) {
    let Some(found) = day_check_of(minutes, store_root, symbol_id, into) else {
        return;
    };
    let month = into.month.to_string();
    let _dropped_when_filtered = telemetry::emit(&daycheck_event(&found, instrument, &month));
    // THE RULE `vendor_truncated` HAS ITS OWN LINE (WP-13d, DB-B19): the line
    // above is at `telemetry::MAX_FIELDS`, and a count that fell off its end
    // would be the silence this rule exists to end.
    if let Ok(report) = &found
        && report.vendor_truncated > 0
    {
        let _dropped_when_filtered = telemetry::emit(
            &telemetry::Event::warn(
                "pull.daycheck",
                "pulled 1day holds fields one paisa under its own 1min on the 5-paisa grid",
            )
            .with("instrument", telemetry::Value::Str(instrument))
            .with("month", telemetry::Value::Str(&month))
            .with("rule", telemetry::Value::Str("vendor_truncated"))
            .with(
                "fields",
                telemetry::Value::Uint(u64::try_from(report.vendor_truncated).unwrap_or(u64::MAX)),
            )
            .with(
                "why",
                telemetry::Value::Str(report.first_truncated.as_deref().unwrap_or("")),
            ),
        );
    }
}

/// The one `pull.daycheck` line for what [`check_day`] found (D-3347).
///
/// Four sentences, and the level is the finding's, not the field count's:
/// identical, or differing without contradiction, is `info`; a pulled day its
/// minutes exceed, a minute day with no pulled day, or an unreadable day file
/// is `warn`. Twelve fields at most, which is `telemetry::MAX_FIELDS`, so none
/// is counted and lost: `why` is the first contradiction when there is one and
/// the first difference otherwise. The report's `open_differs` is not on the
/// line for that reason.
fn daycheck_event<'a>(
    found: &'a Result<crate::daycheck::Report, String>,
    instrument: &'a str,
    month: &'a str,
) -> telemetry::Event<'a> {
    let (level, message) = match found {
        Ok(report) if report.clean() => (
            telemetry::Level::Info,
            "pulled 1day agrees with the days its 1min bars fold to",
        ),
        Ok(report) if report.consistent() => (
            telemetry::Level::Info,
            "pulled 1day differs from its 1min fold and is contradicted by none of it",
        ),
        Ok(_) => (
            telemetry::Level::Warn,
            "pulled 1day is contradicted by, or missing for, days its 1min bars fold to",
        ),
        Err(_) => (
            telemetry::Level::Warn,
            "pulled 1day could not be checked against its 1min bars",
        ),
    };
    let event = telemetry::Event::new(level, "pull.daycheck", message)
        .with("instrument", telemetry::Value::Str(instrument))
        .with("month", telemetry::Value::Str(month));
    let count = |n: usize| telemetry::Value::Uint(u64::try_from(n).unwrap_or(u64::MAX));
    match found {
        Ok(report) => event
            .with("agreed", count(report.agreed))
            .with("differed", count(report.differed))
            .with("contradicted", count(report.contradicted))
            .with("close_differs", count(report.close_differs))
            .with("range_wider", count(report.range_wider))
            .with("volume_above", count(report.volume_above))
            .with(
                "largest_breach",
                telemetry::Value::Int(report.largest_breach),
            )
            .with("day_absent", count(report.day_absent))
            .with("minute_absent", count(report.minute_absent))
            .with(
                "why",
                telemetry::Value::Str(
                    report
                        .first_contradiction
                        .as_deref()
                        .or(report.first.as_deref())
                        .unwrap_or(""),
                ),
            ),
        Err(why) => event.with("why", telemetry::Value::Str(why)),
    }
}

/// [`check_day`]'s finding: `None` for any vendor but Zerodha, otherwise the
/// comparison or the reason the pulled day file could not be read.
fn day_check_of(
    minutes: &[Bar],
    store_root: &Path,
    symbol_id: u32,
    into: &DeriveInto<'_>,
) -> Option<Result<crate::daycheck::Report, String>> {
    if into.vendor != Vendor::Zerodha {
        return None;
    }
    let read = || -> Result<Vec<Bar>, String> {
        let path = StorePath::new(PathParts {
            vendor: into.vendor,
            exchange: into.exchange.as_str(),
            segment: into.segment.as_str(),
            symbol: into.symbol.as_str(),
            contract: into.contract,
            timeframe: Timeframe::DAY_1,
            month: into.month,
            file: FileKind::Bars,
        })
        .map_err(|why| why.to_string())?;
        let file =
            BarFile::open_existing(store_root, path, symbol_id).map_err(|why| why.to_string())?;
        (0..file.header().n_valid)
            .map(|i| file.read_record(i).map_err(|why| why.to_string()))
            .collect()
    };
    Some(
        read().and_then(|days| {
            crate::daycheck::compare(minutes, &days).map_err(|why| why.to_string())
        }),
    )
}

/// Folds one month of minute bars into `rung` and appends them under its own
/// directory, answering the census row for what the file now holds.
///
/// # Errors
///
/// The fold's own refusal, the path's, or the store's — each in its own words,
/// and none of them fails the minute bars that are already committed.
fn derive(
    minutes: &[store::format::Bar],
    source: Timeframe,
    rung: Timeframe,
    store_root: &Path,
    symbol_id: u32,
    parts: PathParts<'_>,
    context: (
        EntryKey,
        Option<&crate::cash_auction::Schedule>,
        crate::calendar::Runtime<'_>,
    ),
) -> Result<(Held, Vec<String>, Option<String>), String> {
    let (key, cash_schedule, calendar) = context;
    // A coarse source cannot prove that every underlying minute existed.
    // Keep the paid-for source, but never publish an unchecked derived copy.
    if source != Timeframe::MINUTE_1 {
        return Err(format!(
            "{} source cannot attest minute completeness for {}; derived output withheld; ingest one-minute source bars",
            source.as_str(),
            rung.as_str()
        ));
    }
    let bucket = crate::fold::Bucket::of_secs(rung.secs())
        .ok_or("a timeframe of zero seconds has no bucket to fold into")?;
    let exchange = Exchange::parse(parts.exchange).map_err(|why| why.to_string())?;
    let segment = Segment::parse(parts.segment).map_err(|why| why.to_string())?;
    let venue = crate::vendor::Venue::for_segment(exchange, segment).ok_or_else(|| {
        format!(
            "no verified minute-completeness venue for {}/{}",
            parts.exchange, parts.segment
        )
    })?;
    // THE FOLD COUNTS ITS NOTES INSTEAD OF REPEATING THEM (D-3341, WP-13b):
    // one diagnostic per reason, so one `pull.derive` line and one receipt
    // failure per reason per rung-month, and the counts go to the ledger.
    let crate::fold::Derived {
        mut bars,
        tally,
        mut diagnostics,
    } = crate::fold::derive_minutes(minutes, bucket, venue, cash_schedule, calendar)
        .map_err(|why| why.to_string())?;
    for why in &mut diagnostics {
        // The fold knows source coverage, not whether a complete candidate is
        // missing behind a stored tail. Classify store remedies below instead.
        if let Some(coverage) =
            why.strip_suffix("; historical gap refill requires a versioned store repair")
        {
            *why = format!(
                "{coverage}; restore complete minute source and verified schedule evidence before retrying derivation"
            );
        }
        note_not_derived(parts.symbol, rung, why);
    }
    let historical = reconcile_derived(&mut bars, store_root, symbol_id, parts).map_err(|why| {
        if diagnostics.is_empty() {
            why
        } else {
            format!("{why}; {}", diagnostics.join("; "))
        }
    })?;
    let behind = !historical.is_empty();
    diagnostics.extend(historical);
    if bars.is_empty() {
        return Err(format!(
            "{} folded to no complete bars: {}",
            rung.as_str(),
            diagnostics.join("; ")
        ));
    }
    // ONLY THE CENSUS ROW. A derived rung's committed count is not reported
    // separately — the receipt's `derived` line already says how many rungs
    // landed, and a second number for the same fact would be a second thing to
    // keep in step.
    write_and_count(&bars, store_root, symbol_id, parts, key).map(|(held, written)| {
        // THE MONTH'S `derive` LINE (D-3321 item 3, D-3335), when the month
        // changed or still owes something: a re-derive that wrote nothing and
        // withheld nothing has nothing new to say.
        let line = (written > 0 || tally.withheld > 0 || behind)
            .then(|| request_minutes::derive_line(&parts, tally));
        (held, diagnostics, line)
    })
}

/// Compare historical derived bytes at the monthly boundary, O(source + held).
/// History is never replaced. Missing evidence and byte conflicts both refuse
/// the rung, but only a complete-source comparison proves a conflict. Unfillable holes
/// produce failures while complete buckets after the tail may still append.
fn reconcile_derived(
    bars: &mut Vec<Bar>,
    root: &Path,
    symbol_id: u32,
    parts: PathParts<'_>,
) -> Result<Vec<String>, String> {
    let mut diagnostics = Vec::new();
    let path = StorePath::new(parts).map_err(|why| why.to_string())?;
    if !path
        .to_path_buf(root)
        .try_exists()
        .map_err(|why| why.to_string())?
    {
        return Ok(diagnostics);
    }
    let file = BarFile::open_existing(root, path, symbol_id).map_err(|why| why.to_string())?;
    let mut candidates = bars.iter().peekable();
    let mut last = None;
    // COUNTED, NOT REPEATED (D-3341, WP-13b): one line for every candidate
    // behind the tail, where there was one line each.
    let mut gaps = 0_u64;
    let mut first_gap: Option<i64> = None;
    let mut last_gap = 0_i64;
    for index in 0..file.header().n_valid {
        let held = file.read_record(index).map_err(|why| why.to_string())?;
        while let Some(candidate) = candidates.peek().filter(|b| b.ts_micros < held.ts_micros) {
            gaps = gaps.saturating_add(1);
            first_gap = first_gap.or(Some(candidate.ts_micros));
            last_gap = candidate.ts_micros;
            candidates.next();
        }
        let matching = candidates
            .peek()
            .is_some_and(|b| b.ts_micros == held.ts_micros);
        if !matching {
            return Err(format!(
                "historical derived evidence incomplete at {}: no complete source candidate for stored bar; restore complete minute source and verified schedule evidence before retrying derivation; rung refused; bytes preserved, not repaired",
                held.ts_micros
            ));
        }
        if candidates.peek().is_some_and(|b| **b != held) {
            return Err(format!(
                "historical derived conflict at {}: stored bar differs from complete source; rung refused; bytes preserved, not repaired; explicit versioned correction required",
                held.ts_micros
            ));
        }
        candidates.next();
        last = Some(held);
    }
    if let Some(first) = first_gap {
        let why = if gaps == 1 {
            format!(
                "historical derived gap at {first}: complete source candidate is absent from stored history behind the tail; cannot be refilled append-only; versioned gapfill required; bytes preserved"
            )
        } else {
            format!(
                "historical derived gap at {first}: {gaps} folded source candidates from {first} to {last_gap} are absent from stored history behind the tail; cannot be refilled append-only; versioned gapfill required; bytes preserved"
            )
        };
        note_not_derived(parts.symbol, parts.timeframe, &why);
        diagnostics.push(why);
    }
    if let Some(last) = last {
        bars.retain(|bar| bar.ts_micros > last.ts_micros);
        // Re-offer an actual stored record to refresh the census on a no-op.
        if bars.is_empty() {
            bars.push(last);
        }
    }
    Ok(diagnostics)
}

/// Appends `bars` under `parts` and answers the census row for what the FILE
/// now holds — not for the batch that was offered.
///
/// # The path refusal here is a backstop, not a live check
///
/// With the venue a closed enum, the symbol a `Symbol` (whose byte set is a
/// subset of `store::path::check_segment`'s and whose capacity is the same 24),
/// the vendor a `Vendor`, the timeframe one of `Timeframe::KNOWN` and the month
/// a `YearMonth`, `StorePath::new` cannot fail from this caller. It stays
/// because this is the only thing that renders a path, and a caller that
/// skipped it would be ASSERTING the above rather than checking it — but
/// `docs/06-limits.md` should not be told the arm is exercised.
///
/// # It answers what it WROTE, beside what it was given
///
/// `BarFile::append` returns `Committed` or `AlreadyPresent`, and this
/// discarded the discriminant with a bare `?`. So re-running a pull the store
/// already held reported "Rows read 375, Bars stored 375" with zero bytes
/// written and the census untouched — `balances()` true, the journal saying
/// `Stored`, and nothing stored. The second return value is the number that
/// tells those two runs apart.
///
/// The one place a bar reaches the disk, shared by the rung that was pulled and
/// every rung derived from it. Two copies of this would be two chances for the
/// counter row to describe something different from the file it names, and the
/// `rows`, both timestamps and both closes are all read back off the header the
/// append just committed for exactly that reason.
///
/// # Errors
///
/// The path's refusal or the store's, in its own words.
fn write_and_count(
    bars: &[store::format::Bar],
    store_root: &Path,
    symbol_id: u32,
    parts: PathParts<'_>,
    key: EntryKey,
) -> Result<(Held, usize), String> {
    let path = StorePath::new(parts).map_err(|why| why.to_string())?;
    // A MONTH SERVED FROM A REPAIR REVISION (D-3350). The writer door refuses
    // it — an append to its original would never be served — so a batch it
    // already holds, bar for bar, is answered as the append door answers one:
    // nothing written. A batch holding anything new is refused by name; only
    // a further repair can add to the month.
    if let store::generation::Served::Revision(pointer) =
        store::generation::served(store_root, path).map_err(|why| why.to_string())?
    {
        return already_served(bars, store_root, path, symbol_id, key, pointer);
    }
    let mut file =
        BarFile::open_or_create(store_root, path, symbol_id).map_err(|why| why.to_string())?;
    // THE DISCRIMINANT IS KEPT, AND IT WAS THROWN AWAY. `Committed` and
    // `AlreadyPresent` are the difference between a run that wrote a month and
    // one that re-offered it, and `?` on its own erased that.
    // AND THE COUNT IS THE ONE THE APPEND REPORTS, NOT THE ONE WE OFFERED.
    //
    // This kept only the discriminant and then answered `bars.len()` — the
    // whole batch — whenever anything was written. On a PARTIAL overlap that is
    // wrong by the overlap: `BarFile::append`'s own doc gives the case, held
    // 2026-08-04 at 375 bars and offered 08-04..08-06 at 1,260, of which 885
    // were actually written. The receipt said 1,260.
    //
    // That is the ordinary shape of a resume, not an edge case, and it breaks
    // the one comparison two render surfaces use to spot a run that offered
    // everything and wrote nothing: `bars_stored` against `bars_committed`. The
    // signature only reads when the overlap is TOTAL; on a normal resume the
    // two were equal and the figure was inflated.
    //
    // `Committed { first_index, n_valid }` already carries the answer —
    // `n_valid` is the counter AFTER the append and `first_index` is where this
    // batch's first record landed, so the difference is what this call wrote.
    // On a partial overlap `append` recurses on the suffix, so both values
    // describe the suffix and the subtraction is right without knowing that.
    let written = match file.append(bars).map_err(|why| why.to_string())? {
        store::file::Appended::Committed {
            first_index,
            n_valid,
        } => usize::try_from(n_valid.saturating_sub(first_index)).unwrap_or(0),
        // NOTHING WAS WRITTEN AND NO GENERATION WAS SPENT. Zero is the number
        // that makes a re-run distinguishable from a first run on a receipt.
        store::file::Appended::AlreadyPresent { .. } => 0,
    };
    let header = file.header();
    let closes = month_closes(&file, &header, bars)?;
    Ok((
        Held::new(
            Entry {
                key,
                rows: header.n_valid,
                first_ts_micros: header.first_ts_micros,
                last_ts_micros: header.last_ts_micros,
            },
            closes,
        ),
        // WRITTEN, NOT MERELY OFFERED, AND NOT THE WHOLE BATCH EITHER.
        written,
    ))
}

/// [`write_and_count`] for a month served from a repair revision: every
/// offered bar must already be held there with the same values, and then the
/// answer is the served month's census row with nothing written. One lookup
/// per offered bar through the revision's own time index.
fn already_served(
    bars: &[store::format::Bar],
    store_root: &Path,
    path: StorePath<'_>,
    symbol_id: u32,
    key: EntryKey,
    pointer: store::generation::Pointer,
) -> Result<(Held, usize), String> {
    let file =
        BarFile::open_existing(store_root, path, symbol_id).map_err(|why| why.to_string())?;
    let n_valid = file.header().n_valid;
    for bar in bars {
        let held = file
            .first_at_or_after(bar.ts_micros)
            .ok()
            .filter(|&index| index < n_valid)
            .and_then(|index| file.read_record(index).ok());
        if held != Some(*bar) {
            return Err(format!(
                "the month is served from repair revision {} (D-3350) and does not hold the offered bar at {} as offered; an ordinary append cannot add to it, so nothing was written. Only a further versioned repair adds to such a month: POST /pull/repair or cli zerodha-repair for a pulled feed; a GDFL import back-fills a day the month lacks by itself",
                pointer.revision.ordinal(),
                bar.ts_micros
            ));
        }
    }
    drop(file);
    Ok((held_of_served(store_root, path, symbol_id, key)?, 0))
}

/// Writes the overlay records beside the bars they belong to.
///
/// # Why this is separate from [`write_and_count`]
///
/// The overlay is not counted. `write_and_count` returns a [`Held`] because a
/// bar file's row count is what `/store` and the ladder gate answer from; an
/// overlay row is a COLUMN of a bar that is already counted, and counting it
/// again would give a reader two numbers for one month and two answers when
/// they drifted.
///
/// # Why an empty batch is a success and not a write
///
/// A contract whose vendor stated neither a spot nor a volatility has nothing
/// to overlay, and a file of null rows costs a block per 170 bars to answer
/// what an absent file answers better. So the caller filters and this refuses
/// nothing.
///
/// # Errors
///
/// The store's own, as text — the same shape `write_and_count` returns, so a
/// caller reports both the same way.
///
/// # Cost
///
/// One open and one append — **not O(1) per call; corrected by D-2372 (OD-3)**.
/// The open reads the header; the
/// `BarFile::append` is not O(1) in the file: it locates the batch against
/// what is already stored by bisection, `already_stored`, at most
/// `ceil(log2(n_valid + 1))` record reads plus O(batch) comparisons (D-1434);
/// it re-reads and verifies the old tail block before re-sealing it (D-0910);
/// and it re-reads every block the batch touches to seal its checksum.
/// So one call is `O(log n_valid + rows + blocks touched)`. `docs/06-limits.md`,
/// D-2372.
///
/// **UNVERIFIED as a measurement.** The bound is argued from the
/// shape of the code and no bench in this workspace times it.
/// `CLAUDE.md` §3 rule 6: a structural argument is not a
/// measurement, however sound it is.
fn write_overlay(
    rows: &[store::format::Overlay],
    store_root: &Path,
    symbol_id: u32,
    parts: PathParts<'_>,
) -> Result<(), String> {
    if rows.is_empty() {
        return Ok(());
    }
    let path = StorePath::new(PathParts {
        file: FileKind::Overlay,
        ..parts
    })
    .map_err(|why| why.to_string())?;
    let mut file =
        BarFile::open_or_create(store_root, path, symbol_id).map_err(|why| why.to_string())?;
    file.append(rows).map_err(|why| why.to_string())?;
    Ok(())
}

/// Where one month's computed greeks are filed.
///
/// A struct rather than eight positional arguments, for the reason
/// [`store::path::PathParts`] gives about itself: `exchange`, `segment` and
/// `symbol` are three short strings, and swapping any two at a call site builds
/// a valid-looking path to the wrong place with nothing to catch it.
#[derive(Debug, Clone, Copy)]
pub struct GreekTarget<'a> {
    /// The store root.
    pub store_root: &'a Path,
    /// Whose feed the bars came from.
    pub vendor: brutex_core::vendor::Vendor,
    /// `NSE`.
    pub exchange: &'a str,
    /// `FNO`.
    pub segment: &'a str,
    /// The UNDERLYING, not the contract — `NIFTY`, never `NIFTY26AUG…CE`.
    pub symbol: &'a str,
    /// The contract segment, which for an option is always `Some`.
    pub contract: Option<brutex_core::instrument::Contract>,
    /// The rung these bars were stored at.
    pub timeframe: Timeframe,
    /// The month. One file per month, so every row must fall inside it.
    pub month: store::path::YearMonth,
}

/// Writes one month's computed greeks beside the bars they price.
///
/// # Why this is public and [`write_overlay`] is not
///
/// The overlay is a VENDOR-STATED column and arrives with the bars, so it is
/// filed by the same function that files them. Greeks are COMPUTED, by a caller
/// that holds the rate and the contract's strike and side — neither of which
/// survives into `Plan`, because `brutex_core::instrument::Contract` is a
/// rendered string that cannot be read back into its parts. So the computation
/// happens at the caller and the filing happens here, once, rather than the
/// path being rebuilt at every call site that wants to write one.
///
/// # An empty batch is a success and not a write
///
/// A month whose rows all refused to price has nothing to file, and a file of
/// zero rows costs a block to answer what an absent file answers better. The
/// same rule [`write_overlay`] follows.
///
/// # Ordering
///
/// **After the bars, never before.** A sidecar describing bars that are not
/// there is worse than no sidecar: the next reader joins on a stamp that has no
/// bar. This function does not enforce that — it cannot see the bar write — so
/// it is stated here and obeyed by the caller, exactly as the overlay's is.
///
/// # Errors
///
/// The store's own, as text: a path this store cannot name, a month that will
/// not open, or an append the file refuses — including a batch that is not a
/// suffix of what is already there, which is `CLAUDE.md` §8's append-only rule
/// doing its job rather than a fault.
///
/// # Cost
///
/// One `fnv1a` over the symbol, one open and one append. **Not "O(1) per call
/// and nothing else", which this said until D-2372 (OD-3).** The
/// `BarFile::append` is not O(1) in the file: it locates the batch against
/// what is already stored by bisection, `already_stored`, at most
/// `ceil(log2(n_valid + 1))` record reads plus O(batch) comparisons (D-1434);
/// it re-reads and verifies the old tail block before re-sealing it (D-0910);
/// and it re-reads every block the batch touches to seal its checksum.
/// So one call is `O(log n_valid + rows + blocks touched)`. `docs/06-limits.md`,
/// D-2372.
///
/// **UNVERIFIED as a measurement.** The bound is argued from the
/// shape of the code and no bench in this workspace times it.
/// `CLAUDE.md` §3 rule 6: a structural argument is not a
/// measurement, however sound it is.
pub fn write_greeks(rows: &[store::format::Greek], into: GreekTarget<'_>) -> Result<(), String> {
    if rows.is_empty() {
        return Ok(());
    }
    let path = StorePath::new(PathParts {
        vendor: into.vendor,
        exchange: into.exchange,
        segment: into.segment,
        symbol: into.symbol,
        contract: into.contract,
        timeframe: into.timeframe,
        month: into.month,
        file: FileKind::Greeks,
    })
    .map_err(|why| why.to_string())?;
    // THE SAME DERIVATION THE BAR PATH USES. A second way of computing this
    // would be a second answer to "which slot does this symbol occupy", and the
    // header records it — so a greeks file and its bars would disagree about
    // their own identity.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "`fnv1a` is a 64-bit hash narrowed to the header's 32-bit \
                  slot id. The narrowing is the identity this store uses \
                  everywhere; computing it differently here would be the \
                  disagreement the comment above names"
    )]
    let symbol_id = brutex_core::universe::fnv1a(into.symbol) as u32;
    let mut file =
        BarFile::open_or_create(into.store_root, path, symbol_id).map_err(|why| why.to_string())?;
    file.append(rows).map_err(|why| why.to_string())?;
    Ok(())
}

/// Records one month in the census, unless it is already recorded exactly.
///
/// Returns whether anything changed, which is what decides whether the file is
/// rewritten at all.
///
/// # The probe is not an optimisation
///
/// The manifest's entry region is an append-only log, so re-recording a month
/// whose numbers have not moved appends a second row saying what the first row
/// already said. Two identical runs would then leave two different files, and
/// `CLAUDE.md` §3 rule 5 is about the bytes. One hash probe answers "is this
/// already exactly what is recorded", and equality is over the whole row —
/// key, rows, both timestamps **and both closes** — so a month that grew by one
/// bar is a change and is recorded.
///
/// **The closes are in that comparison on purpose.** A version-1 census records
/// no close, so a month re-ingested against one differs in exactly that field
/// and is re-recorded — which is how the 43,422 entries already on disk fill in
/// as they are touched, rather than staying unknown forever. Once filled they
/// compare equal and the file stops moving, so the run after that writes
/// nothing.
///
/// # Errors
///
/// Whatever [`Manifest::record_held`] refuses, in its own words: a row count
/// that went backwards, timestamps that did, or a census at its ceiling.
fn count(census: &mut Manifest, held: Held) -> Result<Option<Append>, String> {
    if census.held(&held.entry.key) == Some(held) {
        return Ok(None);
    }
    // THE `Append` IS KEPT, NOT DROPPED. It carries the one 128-byte write and
    // the header commit that publishes it — everything an incremental install
    // needs, already computed. Discarding it here is what forced the caller to
    // re-image every committed entry to publish the few it had touched.
    census
        .record_held(held)
        .map(Some)
        .map_err(|why| why.to_string())
}

/// The vendor's census, or why this run must not write.
///
/// # An absent file is a first ingest, and only an absent file
///
/// [`ErrorKind::NotFound`] and [`ErrorKind::NotADirectory`] are the two shapes
/// of *there is nothing at that path*: no file, or no directory to hold one.
/// Both are the ordinary state before a vendor's first ingest, and both are
/// answered with the empty slice, which [`Manifest::open_image`] turns into a
/// genesis census.
///
/// **Every other refusal stops the run.** A census that exists and cannot be
/// read must never be treated as one that does not exist: starting a fresh
/// census over a file that already holds one is the defect D-0036 closed — the
/// stale slot wins on generation, the new commit lands in the other, and the
/// result loads clean while being wrong.
///
/// # Errors
///
/// The path and the refusal, in one sentence, for a file that cannot be
/// measured, is larger than this build could have written, cannot be read, or
/// is not a census this build accepts.
fn read_census(path: &Path, vendor: Vendor) -> Result<Manifest, String> {
    // ONE OPEN, ONE `fstat` OF THAT HANDLE, ONE CAPPED READ (CE-64, D-2684).
    //
    // This took `fs::metadata(path).len()` and then `fs::read(path)`. The length
    // `stat` gives a FIFO or a device is 0, so the ceiling passed, and the read
    // then blocked for ever on a FIFO with no writer, while this run held the
    // census lock and the vendor's seat, or read `/dev/zero` until memory ran
    // out. A file swapped or grown between the two calls was read whole. The
    // api's reader of this same file refuses all three by name (P-19).
    let bytes = match read_regular_capped(path, MAX_CENSUS_BYTES) {
        Ok(Ok(bytes)) => bytes,
        Ok(Err(why)) => {
            return Err(format!(
                "{} {why}; the largest census this build can write is \
                 {MAX_CENSUS_BYTES} bytes, and a census is a regular file",
                path.display()
            ));
        }
        Err(ref absent)
            if matches!(
                absent.kind(),
                ErrorKind::NotFound | ErrorKind::NotADirectory
            ) =>
        {
            Vec::new()
        }
        Err(why) => {
            return Err(format!(
                "{} could not be measured or read: {why}. A census that exists and \
                 cannot be read is not one that does not exist, and this run \
                 will not start a second census over it",
                path.display()
            ));
        }
    };
    Manifest::open_image(vendor, &bytes).map_err(|why| format!("{}: {why}", path.display()))
}

/// At most `cap` bytes of a **regular** file, read through one handle.
///
/// The path is opened once with no-follow and non-blocking flags, so a FIFO
/// with no writer opens at once instead of waiting and a final symlink is
/// refused by the host. That handle's `fstat` decides whether it is a regular
/// file, so nothing can be swapped in between the check and the read, and the
/// read is capped at one byte past `cap`, so a file that grows under it is
/// refused rather than held. The shape `api::census::sized` already has (P-19).
/// CE-64 and CE-65, D-2684; proved by
/// `pull::ingest::tests::a_census_path_that_is_not_a_regular_file_is_refused_unread_and_never_waits`.
///
/// # Errors
///
/// The host's own error for an open or `fstat` that fails (an absent path is
/// `NotFound`, a final symlink `FilesystemLoop`); `Ok(Err(sentence))` for a
/// path that is not a regular file or holds more than `cap` bytes.
pub(crate) fn read_regular_capped(
    path: &Path,
    cap: u64,
) -> std::io::Result<Result<Vec<u8>, Unbounded>> {
    use std::io::Read as _;
    let file = store::open_flags::open_read_no_follow(path)?;
    let meta = file.metadata()?;
    if !meta.is_file() {
        return Ok(Err(Unbounded::NotRegular));
    }
    // THE HANDLE'S OWN LENGTH FIRST, so a file already past the cap is refused
    // with nothing read; the capped read below catches one that grows.
    if meta.len() > cap {
        return Ok(Err(Unbounded::PastCap {
            cap,
            size: meta.len(),
        }));
    }
    let mut bytes = Vec::new();
    let _ = file.take(cap.saturating_add(1)).read_to_end(&mut bytes)?;
    let size = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    if size > cap {
        return Ok(Err(Unbounded::PastCap { cap, size }));
    }
    Ok(Ok(bytes))
}

/// Why [`read_regular_capped`] read nothing it would return.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Unbounded {
    /// The handle's `fstat` is not a regular file: a FIFO, a device or a
    /// directory. Nothing was read.
    NotRegular,
    /// The file holds more than the cap: its length said so before any byte
    /// was read, or it grew past the cap under a read that stopped one byte
    /// beyond it.
    PastCap {
        /// The cap.
        cap: u64,
        /// The bytes the handle's length, or the read, found.
        size: u64,
    },
}

impl std::fmt::Display for Unbounded {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotRegular => f.write_str(
                "is not a regular file (a FIFO, a device or a directory); refused \
                 unread, because this reader neither waits on a FIFO nor reads a device",
            ),
            Self::PastCap { cap, size } => {
                write!(
                    f,
                    "holds {size} bytes, more than the {cap} this reader takes"
                )
            }
        }
    }
}

/// Publishes a whole census: write a temporary, flush it, rename it into place.
///
/// # Why a rename and not a write
///
/// [`Manifest::image`] is atomic as a buffer and says so; its **installation**
/// is this function's problem and a rename within one filesystem is the only
/// step that is atomic against a crash. Overwriting the live file in place
/// publishes a prefix — a header counting entries that are not there yet — and
/// while that prefix is *detected* on the next load
/// (`CounterExceedsRegion`, or a slot that fails its own CRC-32C), being
/// detectably broken is worse than never being broken.
///
/// The temporary sits in the census's own directory so the rename cannot cross
/// a filesystem, and the directory is flushed afterwards so the rename itself
/// survives the power going out.
///
/// # Errors
///
/// The path and the host's own words, for a directory that cannot be made, a
/// temporary that cannot be written or flushed, or a rename that is refused.
/// The census lock, held for the whole read-modify-write.
///
/// # Why this is a guard and not a call inside `install`
///
/// It *was* a call inside `install`, and that closed half the hole. The census
/// cycle is: read the whole file, mutate the image in memory, write the whole
/// file back. Locking only the write serialises the two renames and does
/// nothing about the two reads that already happened — so run A reads 20
/// entries, run B reads the same 20, A installs 20+A, B installs 20+B over the
/// top, and A's entries are gone. Neither run ever sees the lock contended,
/// because neither holds it while the other is reading.
///
/// That is a lost update, and it produces the exact symptom the incident below
/// records. The fix is not a better lock; it is a lock held across the whole
/// cycle. This guard is taken before [`read_census`] and dropped after the
/// install, and [`install_locked`] takes a reference to it so the install
/// cannot be reached without it.
///
/// Dropping the guard is what releases the census, so the field is live for
/// its whole life and read exactly once, by the compiler-generated drop. The
/// guard unlocks explicitly rather than by closing the [`File`]: a descriptor
/// duplicated into a child another thread spawned would otherwise hold the
/// census lock past the run that took it, and the next run would be refused
/// as if one were still installing (D-0693).
struct CensusLock {
    _held: Option<Flock<fs::File>>,
}

impl CensusLock {
    /// Take the lock, or refuse naming the run that holds it.
    ///
    /// `Ok(None)` inside the guard means the lock file could not be opened for
    /// a reason that **also stops the install a moment later** — the path is
    /// unusable for the whole directory, not for this one file. That refusal is
    /// not this function's to word: `install_locked` fails on the same cause
    /// and says it better, and two tests in `crates/pull/tests/census.rs`
    /// expect that sentence rather than a worse-worded twin from here.
    ///
    /// **Every reason that leaves the census writable is an `Err`**, because a
    /// lock that quietly becomes no lock is the `CLAUDE.md` §4 fallback that
    /// hides a failure — see the arms below for which is which, and why the
    /// division is by what the census can still do rather than by errno.
    fn take(path: &Path) -> Result<Self, String> {
        // BEST EFFORT, and deliberately not `?`. The lock lives beside the
        // census and on a first ever run neither exists, so the directory is
        // attempted here — but a failure to create it is `install_locked`'s to
        // report, for the reason above.
        if let Some(dir) = path.parent() {
            let _ = fs::create_dir_all(dir);
        }
        Self::take_at(&path.with_extension("man.lock"))
    }

    /// [`CensusLock::take`] at an explicit lock path, creating no directory.
    ///
    /// A version-4 partition's lock is `manifest/v4/<vendor>/<SEGMENT>.lock`,
    /// beside the segment's directory of epochs rather than beside one epoch,
    /// because the epoch a cycle appends to can change under compaction while
    /// the partition it belongs to does not (D-3316 item 1). The vendor's
    /// directory is NOT created here: its existence is what marks the vendor
    /// as migrated, so only `census_store::migrate` may make it.
    fn take_at(lock_path: &Path) -> Result<Self, String> {
        let lock_path = lock_path.to_path_buf();
        // A LOCK THAT CANNOT BE TAKEN IS A REFUSAL, NEVER A RUN WITHOUT ONE.
        //
        // This arm used to be `else { return Ok(Self { _held: None }) }` — any
        // failure to OPEN the lock file yielded a guard holding nothing, and
        // the run proceeded to read-modify-write the census unserialised. A
        // root-owned `.man.lock` left by one `sudo` run, a restrictive ACL, an
        // immutable flag, a full disk: each turned the mutual exclusion off
        // and said nothing, while the sentence twelve lines below explains at
        // length why two concurrent installs are unacceptable. The guard
        // documented the hazard and then opened the door to it.
        //
        // IT BECAME LIVE TODAY. The ingest page now runs feeds CONCURRENTLY,
        // so two chains can reach the census at once on one machine — the
        // exact interleaving this lock exists to prevent, previously held off
        // only by the runs being serial.
        //
        // `CLAUDE.md` §4: degrade loudly and name the reason, or refuse. There
        // is no third option, and a silent un-locked run is the fallback that
        // hides a failure — the loser's receipt still reads "every row
        // accounted for", because its own books balanced.
        // READ AND WRITE, never write alone (R9-csr-cx-0, D-0955). A write-only
        // open of a FIFO blocks in open(2) until a reader appears, so a FIFO
        // misfiled at this name hung the ingest: it neither refused nor ran.
        // Read-write does not wait on a FIFO, and the type is refused below.
        let lock = match fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&lock_path)
        {
            Ok(lock) => lock,
            // AN ACCESS FAILURE REFUSES; A PATH FAILURE STILL DEFERS.
            //
            // The two causes are not the same, and the original best-effort
            // arm was right about one of them. If the PATH is wrong -- no such
            // directory, a file where the directory belongs, a name past the
            // host's limit -- the install fails on the same cause a moment
            // later and names it in the words an operator needs, and this
            // function must not pre-empt it with a worse-worded twin. That is
            // what `a_census_that_cannot_be_measured_stops_the_run` and
            // `a_census_that_cannot_be_installed_names_what_is_left_uncounted`
            // assert, and they are right to.
            //
            // A FAILURE OF THIS ONE FILE is the dangerous one, and the one that
            // was silent. The lock is a SIBLING of the census, not a parent of
            // it, so a refusal that belongs to the lock's own inode leaves
            // `<vendor>.man` perfectly creatable and the install perfectly
            // able to publish over another run's work. Two shapes reach it:
            //
            // * `PermissionDenied` -- a root-owned `.man.lock` from a `sudo`
            //   run, a restrictive ACL, an immutable flag. The census itself
            //   stays writable and only the LOCK is denied.
            // * `IsADirectory` -- a directory occupying the lock's name, which
            //   is what an interrupted tool or a mistyped `mkdir` leaves. It
            //   was still deferring: the install never touches this path, so
            //   nothing downstream reported it and the run went unserialised
            //   with nothing to say. That is the one case the two tests cited
            //   above do NOT cover, because they break the DIRECTORY and this
            //   breaks the file inside it.
            //
            // Both are the §4 fallback that hides a failure, and both went live
            // the day feeds began running concurrently. A name past the host's
            // limit is `InvalidFilename`, is the path's fault, and must keep
            // deferring — `a_census_that_cannot_be_measured_stops_the_run`
            // builds exactly that and expects the install's wording.
            //
            // Every OTHER kind now refuses too while the census directory
            // stands (pull2-1, D-2765): an EMFILE or an ENOSPC on the lock is
            // not a failure the install repeats. See [`CensusLock::unopened`].
            Err(why) => return Self::unopened(&lock_path, &why),
        };
        // A LOCK THAT OPENED BUT IS NOT A REGULAR FILE IS REFUSED BY NAME. A
        // FIFO or a device that opens at this name is a misfiled path, the same
        // class as the directory refused above, and nothing downstream reports
        // it. A socket never opens, so it is refused in the arm above instead.
        match lock.metadata() {
            Ok(found) if found.is_file() => {}
            Ok(found) => {
                return Err(format!(
                    "the census lock at {} is not a regular file ({}). \
                     Refused rather than run without it: the census beside it \
                     is still writable. Remove what occupies that name and try \
                     again.",
                    lock_path.display(),
                    entry_kind(found.file_type())
                ));
            }
            Err(why) => {
                return Err(format!(
                    "the census lock at {} opened but cannot be measured: {why}. \
                     Refused rather than run without it.",
                    lock_path.display()
                ));
            }
        }
        Flock::try_lock(lock, lock_path.clone())
            .map(|held| Self { _held: Some(held) })
            .map_err(|refusal| lock_refusal(&lock_path, refusal))
    }

    /// Why the lock file would not open, and whether that may defer.
    ///
    /// Split out of [`CensusLock::take`] so its arms can be driven with host
    /// error kinds a test cannot provoke on demand (EMFILE, ENOSPC). D-2765.
    fn unopened(lock_path: &Path, why: &std::io::Error) -> Result<Self, String> {
        if matches!(
            why.kind(),
            ErrorKind::PermissionDenied | ErrorKind::IsADirectory
        ) {
            return Err(format!(
                "the census lock at {} exists but cannot be opened: {why}. \
                 Refused rather than run without it -- the census beside it \
                 is still writable, so this run would have interleaved a \
                 read-modify-write with any other and silently discarded \
                 one of them, while the loser's receipt still read 'every \
                 row accounted for' because its own books balanced. Fix the \
                 ownership, the permissions or the type of that path and \
                 try again.",
                lock_path.display()
            ));
        }
        // SOMETHING THAT IS NOT A FILE OCCUPIES THE NAME (v3b-1, D-1480). A
        // UNIX socket at this name fails the open with ENXIO, which is neither
        // kind above. The kind cannot tell a socket from a path failure, but
        // the name can: `symlink_metadata` that SUCCEEDS on a non-regular entry
        // means the directory is sound and this one name is misfiled -- a
        // socket, a device that would not open, a symlink that leads nowhere or
        // loops. Refused, naming the type.
        if let Ok(found) = fs::symlink_metadata(lock_path)
            && !found.is_file()
        {
            return Err(format!(
                "the census lock at {} is not a regular file ({}) and \
                 cannot be opened: {why}. Refused rather than run \
                 without it: the census beside it is still writable. \
                 Remove what occupies that name and try again.",
                lock_path.display(),
                entry_kind(found.file_type())
            ));
        }
        // A REGULAR FILE THAT WOULD NOT OPEN, OR NO FILE AT ALL, IS NOT THE
        // PATH'S FAULT WHILE ITS DIRECTORY STANDS (pull2-1, D-2765). Both used
        // to defer to the install on the claim that it "fails on the same
        // host". It does not: EMFILE, ENFILE and ENOMEM pass by the time the
        // install runs, and a missing lock that cannot be created on an
        // inode-exhausted disk leaves an existing census appendable in place.
        // Each ran the census read-modify-write unserialised. Only a directory
        // that is not there to hold the census defers now -- absent, a file in
        // its place, or a name past the host's limit -- because the install
        // then fails on that same cause, as
        // `a_broken_directory_defers_because_the_install_fails_on_the_same_cause`
        // and `a_census_that_cannot_be_measured_stops_the_run` assert.
        match lock_path.parent().map(fs::metadata) {
            Some(Ok(dir)) if dir.is_dir() => Err(format!(
                "the census lock at {} could not be opened: {why}. Refused \
                 rather than run without it: the directory that holds the \
                 census stands, so the census may still be writable, and \
                 running unlocked would interleave this run's \
                 read-modify-write with any other and silently discard one \
                 of them. Try again once the host can open that file.",
                lock_path.display()
            )),
            Some(Err(stat))
                if !matches!(
                    stat.kind(),
                    ErrorKind::NotFound | ErrorKind::NotADirectory | ErrorKind::InvalidFilename
                ) =>
            {
                Err(format!(
                    "the census lock at {} could not be opened: {why}, and \
                     the directory that holds it could not be measured: \
                     {stat}. Refused rather than run without the lock.",
                    lock_path.display()
                ))
            }
            _ => Ok(Self { _held: None }),
        }
    }
}

/// What occupies a name, in words an operator can act on (v3b-1, D-1480).
///
/// `FileType`'s `Debug` prints only `is_file`, `is_dir` and `is_symlink`, so a
/// socket and a FIFO both read as three `false`s and the refusal named nothing.
fn entry_kind(found: fs::FileType) -> &'static str {
    #[cfg(unix)]
    {
        use std::os::unix::fs::FileTypeExt as _;
        if found.is_socket() {
            return "a UNIX socket";
        }
        if found.is_fifo() {
            return "a FIFO";
        }
        if found.is_char_device() {
            return "a character device";
        }
        if found.is_block_device() {
            return "a block device";
        }
    }
    if found.is_symlink() {
        "a symbolic link that does not lead to an openable file"
    } else if found.is_dir() {
        "a directory"
    } else if found.is_file() {
        "a regular file"
    } else {
        "an entry of a kind this host does not name"
    }
}

/// The sentence for a census lock that could not be taken.
///
/// Only [`TryLockError::WouldBlock`] means another run holds it. Every other
/// refusal is the host declining `flock` itself (`ENOLCK` on an NFS mount
/// without a lock daemon, `ENOTSUP` for a lock file the host cannot lock), and
/// waiting for "the other run" never helps when there is none. Both arms used
/// to say there was one (R9-csr-cx-1, D-0955).
fn lock_refusal(lock_path: &Path, refusal: std::fs::TryLockError) -> String {
    match refusal {
        std::fs::TryLockError::WouldBlock => format!(
            "another ingest holds the census lock at {}. Refused rather \
             than queued: two runs installing at once silently discard one, and \
             the loser's receipt still reads 'every row accounted for' \
             because its own books balanced. Wait for the other run and try \
             again.",
            lock_path.display()
        ),
        std::fs::TryLockError::Error(host) => format!(
            "the host refused to lock the census lock at {}: {host}. No other \
             run is implied, so waiting will not help; refused rather than run \
             without the lock. Put the store on a filesystem that supports \
             advisory locks.",
            lock_path.display()
        ),
    }
}

/// Publish what this run changed, without re-imaging what it did not.
///
/// # The 424 GB
///
/// The census was installed by [`Manifest::image`] and one `rename`: an
/// `O(entries)` write bought for an install that is atomic by construction.
/// That is a good bargain **once per run**, which is what `from_dir` is — one
/// folder, twelve thousand members, one install.
///
/// `from_window` is not that. It hands a SINGLE-ELEMENT slice to
/// [`from_members`], so the batch cost is paid at per-member frequency: the
/// whole file re-imaged, `fsync`ed and renamed for every window fetched.
/// Measured against the operator's stated backfill in `docs/06-limits.md` §34
/// — **424 GB rewritten to maintain a 5.75 MB file**, where the same work as
/// positional appends is 9.16 MB. The amplification is ~47,400×.
///
/// # Why an append is safe without the rename
///
/// The rename was buying atomicity. The manifest format already buys it
/// another way, and has from the start: **two header slots, written
/// alternately**, commit *g* landing in slot `g % 2`. A crash during a slot
/// write leaves the other slot holding the previous generation, and recovery
/// takes the newest slot that passes its own CRC-32C. So the sequence is
///
/// 1. write the entry at its own offset — beyond `n_valid`, so no reader can
///    see it yet whatever happens next;
/// 2. **barrier**, through [`Commit::durable_through`];
/// 3. write the header slot, which is the single act that makes the entry
///    real.
///
/// A crash before 3 leaves bytes past the counter, which the next run
/// overwrites; a crash during 3 leaves the other slot intact. Neither state is
/// a census that disagrees with itself, which is the property the rename was
/// there for.
///
/// # When the whole image is still written
///
/// **A repair**, because it may rewrite entries the append region already holds
/// and an append-only log cannot express that. **An upgrade**, because the file
/// on disk is at the stride it was written at and this build appends at the
/// stride it writes — against a version-1 census those two disagree from the
/// first entry onward, so the whole file is restated at version 2 instead. And
/// **the first write**, when there is no file or it is shorter than the header:
/// an append at `HEADER_LEN + 0` against a zero-length file would leave the
/// header region a sparse hole and one slot never written at all. All three are
/// stated conditions, not a fallback around a failure — any of them failing
/// still fails loudly.
///
/// The upgrade is checked **after** the "nothing moved" gate, so a run that
/// records nothing leaves a version-1 census exactly as it found it. The
/// conversion is paid once, by the first run that had something to say.
fn install_census(
    lock: &CensusLock,
    path: &Path,
    census: &Manifest,
    appends: &[Append],
    repairing: bool,
) -> Result<(), String> {
    // Nothing moved. A re-run of the same folder leaves the census byte for
    // byte as it was, which is `CLAUDE.md` §3 rule 5 about the bytes.
    if appends.is_empty() && !repairing {
        return Ok(());
    }

    let virgin = match fs::metadata(path) {
        Ok(meta) => meta.len() < HEADER_LEN,
        Err(_) => true,
    };
    if repairing || virgin || census.upgrading() {
        return install_locked(lock, path, &census.image());
    }
    append_locked(lock, path, appends)
}

/// The positional writes, kept apart from the sentence they fail with.
fn append_locked(_lock: &CensusLock, path: &Path, appends: &[Append]) -> Result<(), String> {
    write_appends(path, appends).map_err(|why| {
        format!(
            "{} could not be appended to after {} entry write(s): {why}",
            path.display(),
            appends.len()
        )
    })
}

/// Every entry of the batch, one barrier, then the newest commit of each slot,
/// the older slot first, each followed by its own barrier (WP-16; it was one
/// barrier per entry and one per slot, which is the same bytes at twice the
/// batch's length in flushes).
///
/// The barrier is not optional and not an optimisation to skip: issuing the
/// slot write before the entry bytes are on stable storage publishes a counter
/// over entries that may not exist, which is the one way this format can be
/// made to lie. [`Commit::durable_through`] names the offset, so a writer
/// cannot claim it did not know which one to flush.
fn write_appends(path: &Path, appends: &[Append]) -> std::io::Result<()> {
    write_appends_observed(path, appends, |_| {})
}

/// [`write_appends`], with a look at the file after its last slot is durable
/// and before its modification time is moved past it: the instant a lock-free
/// reader could stamp the census while the slot was still being copied.
///
/// # Why the time is moved at the end (census-1, D-2766)
///
/// An in-place `write` updates the inode's times BEFORE it copies the bytes,
/// and a buffered reader takes no lock to copy a page. A reader that stamped
/// the manifest inside that window and read it before the copy cached the
/// older census, or a torn and "degraded" one, under the FINAL stamp, because
/// nothing moved the times again; the api served it until the next manifest
/// write, possibly the next day. Setting the modification time once more,
/// after every byte is durable, to a value strictly past the one the writes
/// left, guarantees that any stamp taken during them differs from the final
/// one, so the next request reads again. The whole-image install needs none
/// of this: its rename publishes an inode whose bytes were complete first.
///
/// # One batch, three flushes (WP-16, DB-N25)
///
/// This flushed twice PER ENTRY, so an import day of 5,641 option files paid
/// 11,282 flushes for its census alone. The batch is now every entry, a
/// barrier; the newest commit of the slot that held the OLDER generation, a
/// barrier; the newest commit of the other slot, a barrier. The same bytes,
/// still no slot over an entry that is not durable, and still one slot write
/// in flight at a time, never onto the newest durable generation: a torn
/// slot write leaves the census where it was or further on, never behind.
/// [`census_store::write_batch`] carries the argument.
fn write_appends_observed(
    path: &Path,
    appends: &[Append],
    landed: impl FnMut(&fs::File),
) -> std::io::Result<()> {
    census_store::write_batch(path, appends, landed)
}

/// The install itself, once the census lock is held.
///
/// The guard is taken by reference and never read. That is the point: it makes
/// "the lock is held" a thing the compiler checks rather than a thing a comment
/// asserts, so no future caller can reach the publish without having gone
/// through [`CensusLock::take`] first.
fn install_locked(_lock: &CensusLock, path: &Path, image: &[u8]) -> Result<(), String> {
    // A rendered census path is `<root>/manifest/<vendor>.man`, so the parent
    // is always there; the fallback is the path itself, which cannot be
    // created as a directory and therefore fails loudly on the next line
    // rather than writing somewhere unexpected.
    let dir: &Path = path.parent().unwrap_or(path);
    let tmp: PathBuf = path.with_extension("man.writing");
    publish(dir, &tmp, path, image).map_err(|why| {
        format!(
            "{} could not be published through {}: {why}",
            path.display(),
            tmp.display()
        )
    })
}

/// The five calls [`install`] is, kept apart from the sentence it fails with.
fn publish(dir: &Path, tmp: &Path, path: &Path, image: &[u8]) -> std::io::Result<()> {
    fs::create_dir_all(dir)?;
    let mut file = fs::File::create(tmp)?;
    file.write_all(image)?;
    // The bytes before the rename, always. A rename that beats its own
    // contents to the disk publishes a name over nothing.
    file.sync_all()?;
    drop(file);
    fs::rename(tmp, path)?;
    fs::File::open(dir)?.sync_all()
}

/// The facts about this module no external test can reach.
///
/// Most of it is proved from outside, in `crates/pull/tests/census.rs`, because
/// that is where a caller stands — a folder goes in, bars and a counter come
/// out, and the two are checked against each other. These are here because one
/// needs a 268,468,224-byte file on the disk to reach from outside, one is about
/// a read that **does not happen**, which a caller cannot observe by definition,
/// and two are about [`CensusLock::take`]'s own arms — a private function whose
/// interesting outcome is precisely that the ingest around it never starts. A
/// boundary that is only ever tested one side of is a boundary nobody has
/// checked; `pull::manifest` makes the same argument about [`MAX_ENTRIES`] and
/// verifies it the same way.
#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]
mod tests {
    use store::format::Bar;
    use store::header::Header;

    use brutex_core::vendor::Vendor;
    use store::path::Timeframe;

    use super::{
        CensusLock, DeriveInto, EntryKey, MAX_CENSUS_BYTES, check_day, closes_in_hand,
        day_check_of, daycheck_event, install_locked, lock_refusal, write_and_count,
    };

    /// **A MEMBER'S ROWS ARE LANDED BORROWED, NOT CLONED.** o1api-36, D-1203.
    ///
    /// `one` copied every row of every member into a `RawWindow` only to pass
    /// a reference to it. The copy had no output of its own to assert on, so
    /// its absence is asserted on the source, with needles assembled at run
    /// time so this test cannot match its own text. The landing itself is
    /// proved equal through both entry points in
    /// `pull::pipeline::borrowed_rows_land_exactly_as_a_window_does`.
    #[test]
    fn a_member_is_landed_from_its_own_rows_without_a_copy() {
        let source = include_str!("ingest.rs");
        let start = source
            .find(&format!("{}{}", "fn one(\n    member: &", "Member,"))
            .expect("ingest::one exists");
        let body = &source[start..];
        let body = &body[..body.find("\n}\n").expect("ingest::one ends")];
        assert!(
            !body.contains(&format!("{}{}", "member.rows", ".clone()")),
            "ingest::one clones a member's rows again"
        );
        assert!(body.contains(&format!("{}{}", "land_rows(&", "member.rows,")));
    }

    /// **THE THREE STORE-WRITING DOORS DO NOT CLAIM CONSTANT COST.** OD-3,
    /// D-2372, invariant AFG-72.
    ///
    /// `from_rows`, `write_overlay` and `write_greeks` each called themselves
    /// "O(1) per call" with "nothing else" or "Nothing scans", while
    /// `BarFile::append` bisects what is stored (D-1434), verifies the old tail
    /// block (D-0910) and re-reads each touched block to seal it, and
    /// `from_rows` walks every row through `keep_in_session`. The needles are
    /// assembled at run time so this test cannot match its own text.
    /// The real cost stays UNVERIFIED as a measurement (`docs/06-limits.md`).
    #[test]
    fn the_store_writing_doors_state_their_real_cost() {
        let source = include_str!("ingest.rs");
        for false_claim in [
            format!("{}{}", "both O(1). Nothing ", "scans."),
            format!("{}{}", "One open and one append. ", "O(1) per call."),
            format!("{}{}", "O(rows) in the bytes written and ", "nothing else."),
        ] {
            assert!(
                !source.contains(&format!("/// {false_claim}")),
                "a false constant-cost claim is back: {false_claim}"
            );
        }
        for door in [
            "pub fn from_rows(",
            "fn write_overlay(",
            "pub fn write_greeks(",
        ] {
            let at = source.find(door).expect("the door exists");
            let doc = &source[..at];
            let doc = &doc[doc.rfind("/// # Cost").expect("a cost section")..];
            for cite in ["D-1434", "D-0910", "D-2372", "log2(n_valid + 1)"] {
                assert!(doc.contains(cite), "{door} cost omits {cite}");
            }
        }
    }

    /// A scratch directory of this test's own, named after the line that asked
    /// for it so two tests cannot collide in a shared `TMPDIR`.
    fn scratch(tag: &str) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!(
            "brutex-ingest-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |since| since.as_nanos())
        ));
        std::fs::create_dir_all(root.join("manifest")).expect("the manifest directory");
        root
    }

    /// **A PARTIAL-OVERLAP RESUME REPORTS WHAT IT WROTE, NOT WHAT IT OFFERED.**
    ///
    /// `write_and_count` kept only the `Appended` discriminant and then answered
    /// `bars.len()` — the whole batch — whenever anything was written. On a
    /// partial overlap that is wrong by the overlap, and `BarFile::append`'s own
    /// doc gives the case: 375 bars held, 1,260 offered, **885 written**. The
    /// receipt said 1,260.
    ///
    /// That is the ordinary shape of a resume, and it breaks the one comparison
    /// two render surfaces use to spot a run that offered everything and wrote
    /// nothing — `bars_stored` against `bars_committed`. The signature only
    /// reads when the overlap is TOTAL; on a normal resume the two were equal
    /// and the number was simply inflated.
    ///
    /// Three cases, because each is a different arm: a first write reports all
    /// of it, a partial overlap reports only the suffix, and a total re-offer
    /// reports **zero** — the number that makes a re-run distinguishable from a
    /// first run.
    #[test]
    fn a_partial_overlap_counts_the_suffix_it_wrote_not_the_batch_it_offered() {
        let root = scratch("partialcount");
        let month = store::path::YearMonth::new(2026, 1).expect("a legal month");
        let parts = || store::path::PathParts {
            vendor: brutex_core::vendor::Vendor::Dhan,
            exchange: "NSE",
            segment: "INDEX",
            symbol: "NIFTY",
            contract: None,
            timeframe: store::path::Timeframe::MINUTE_1,
            month,
            file: store::path::FileKind::Bars,
        };
        let entry = EntryKey {
            contract: None,
            exchange: brutex_core::instrument::Exchange::Nse,
            segment: brutex_core::instrument::Segment::Index,
            symbol: brutex_core::symbol::Symbol::new("NIFTY").expect("a symbol"),
            timeframe: store::path::Timeframe::MINUTE_1,
            month,
        };
        // Midday IST on 2026-01-01, one bar a minute, so every stamp is inside
        // the month the path names.
        let at = |n: i64| {
            let days = 20_454_i64; // 2026-01-01
            (days * 86_400 - 19_800 + 6 * 3_600 + n * 60) * 1_000_000
        };
        let bar = |n: i64| Bar {
            ts_micros: at(n),
            open: 100 + n,
            high: 110 + n,
            low: 90 + n,
            close: 105 + n,
            volume: 1,
            open_interest: i64::MIN,
        };
        #[expect(
            clippy::cast_possible_truncation,
            reason = "the id is the cross-check `open` folds; any 32 bits serve"
        )]
        let symbol_id = brutex_core::universe::fnv1a("NIFTY") as u32;

        // FIRST WRITE: all three land, all three are counted.
        let first: Vec<Bar> = (0..3).map(bar).collect();
        let (_, wrote) =
            write_and_count(&first, &root, symbol_id, parts(), entry).expect("a first write");
        assert_eq!(wrote, 3, "a first write reports every bar it wrote");

        // PARTIAL OVERLAP: five offered, the first three already held, so TWO
        // are written. This is the assertion the old code failed -- it said 5.
        let overlapping: Vec<Bar> = (0..5).map(bar).collect();
        let (_, wrote) = write_and_count(&overlapping, &root, symbol_id, parts(), entry)
            .expect("the suffix follows and is appended");
        assert_eq!(
            wrote, 2,
            "only the SUFFIX was written -- reporting 5 here is the defect, and \
             it is what a resume does every time it runs"
        );

        // TOTAL RE-OFFER: nothing is written, and the count says so.
        let (_, wrote) = write_and_count(&overlapping, &root, symbol_id, parts(), entry)
            .expect("re-offering the same batch is safe");
        assert_eq!(
            wrote, 0,
            "a re-run wrote nothing, which is what tells it apart from a first run"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    /// **ZERODHA'S PULLED DAY IS CHECKED AGAINST ITS OWN MINUTES, AND NO OTHER
    /// VENDOR'S IS.** No day file is a reason, not a silence; a day file that
    /// matches the fold agrees; a minute that moved the high is named; Dhan is
    /// not checked at all. Each case also drives `check_day`, so every arm of
    /// the `pull.daycheck` line runs. D-3001.
    #[test]
    fn zerodha_minutes_are_checked_against_the_pulled_day_and_no_other_vendor_is() {
        let root = scratch("DayCheck");
        let month = store::path::YearMonth::new(2026, 1).expect("a legal month");
        let parts = |timeframe| store::path::PathParts {
            vendor: Vendor::Zerodha,
            exchange: "NSE",
            segment: "INDEX",
            symbol: "NIFTY",
            contract: None,
            timeframe,
            month,
            file: store::path::FileKind::Bars,
        };
        let entry = |timeframe| EntryKey {
            contract: None,
            exchange: brutex_core::instrument::Exchange::Nse,
            segment: brutex_core::instrument::Segment::Index,
            symbol: brutex_core::symbol::Symbol::new("NIFTY").expect("a symbol"),
            timeframe,
            month,
        };
        let into = |vendor| DeriveInto {
            calendar: crate::calendar::Runtime::default(),
            cash_schedule: None,
            contract: None,
            vendor,
            exchange: brutex_core::instrument::Exchange::Nse,
            segment: brutex_core::instrument::Segment::Index,
            symbol: brutex_core::symbol::Symbol::new("NIFTY").expect("a symbol"),
            month,
        };
        // 09:15 IST on 2026-01-02 and the two minutes after it.
        let at = |n: i64| (20_455_i64 * 86_400 - 19_800 + 33_300 + n * 60) * 1_000_000;
        let minute = |n: i64, open, high, low, close| Bar {
            ts_micros: at(n),
            open,
            high,
            low,
            close,
            volume: 10,
            open_interest: i64::MIN,
        };
        let minutes = [
            minute(0, 100, 110, 95, 105),
            minute(1, 105, 120, 100, 115),
            minute(2, 115, 116, 90, 98),
        ];
        let day = Bar {
            ts_micros: at(0),
            open: 100,
            high: 120,
            low: 90,
            close: 98,
            volume: 30,
            open_interest: i64::MIN,
        };
        #[expect(
            clippy::cast_possible_truncation,
            reason = "the id is the cross-check `open` folds; any 32 bits serve"
        )]
        let symbol_id = brutex_core::universe::fnv1a("NIFTY") as u32;
        let zerodha = into(Vendor::Zerodha);

        // NO PULLED DAY FILE: a reason, never a silent pass.
        let found = day_check_of(&minutes, &root, symbol_id, &zerodha).expect("zerodha is checked");
        assert!(found.is_err(), "{found:?}");
        check_day(&minutes, "NIFTY", &root, symbol_id, &zerodha);

        // THE PULLED DAY MATCHES THE FOLD.
        let _ = write_and_count(
            &[day],
            &root,
            symbol_id,
            parts(Timeframe::DAY_1),
            entry(Timeframe::DAY_1),
        )
        .expect("the day bar lands");
        let report = day_check_of(&minutes, &root, symbol_id, &zerodha)
            .expect("zerodha is checked")
            .expect("the day file reads");
        assert_eq!((report.agreed, report.differed), (1, 0));
        assert!(report.clean());
        check_day(&minutes, "NIFTY", &root, symbol_id, &zerodha);

        // A MINUTE THAT MOVED THE HIGH is named with both values.
        let mut moved = minutes;
        if let Some(second) = moved.get_mut(1) {
            second.high = 125;
        }
        let report = day_check_of(&moved, &root, symbol_id, &zerodha)
            .expect("zerodha is checked")
            .expect("the day file reads");
        assert_eq!(report.differed, 1);
        let first = report
            .first
            .clone()
            .expect("the first disagreement is named");
        assert!(first.contains("high pulled 120 folded 125"), "{first}");
        check_day(&moved, "NIFTY", &root, symbol_id, &zerodha);

        // ANY OTHER VENDOR IS NOT CHECKED: its day bar follows its own
        // convention (D-0077).
        let dhan = into(Vendor::Dhan);
        assert!(day_check_of(&moved, &root, symbol_id, &dhan).is_none());
        check_day(&moved, "NIFTY", &root, symbol_id, &dhan);

        let _ = std::fs::remove_dir_all(&root);
    }

    /// **THE DAY CHECK'S LINE IS `info` FOR A DIFFERENCE AND `warn` ONLY FOR
    /// A CONTRADICTION (D-3347).**
    ///
    /// The pulled close differs from the last minute's on almost every day of
    /// the operator's store, so D-3001's `warn` on any difference warned on
    /// every month. Identical and differing-without-contradiction are `info`
    /// with their own sentences; a pulled day its minutes exceed, a minute day
    /// with no pulled day, and an unreadable file are `warn`. Every count rides
    /// on the line, at most `telemetry::MAX_FIELDS`, and `why` names the first
    /// contradiction ahead of the first difference.
    #[test]
    fn the_day_check_line_warns_only_on_a_contradiction() {
        use crate::daycheck::Report;
        use telemetry::Level;
        let clean: Result<Report, String> = Ok(Report {
            agreed: 20,
            ..Report::default()
        });
        let differs = Ok(Report {
            agreed: 1,
            differed: 19,
            close_differs: 19,
            range_wider: 7,
            volume_above: 19,
            first: Some("IST day 1: close pulled 2 folded 3".to_owned()),
            ..Report::default()
        });
        let contradicted = Ok(Report {
            differed: 2,
            contradicted: 1,
            largest_breach: 22_985,
            first: Some("IST day 1: close pulled 2 folded 3".to_owned()),
            first_contradiction: Some(
                "IST day 2: minute high 9 above the pulled high 8".to_owned(),
            ),
            ..Report::default()
        });
        let absent = Ok(Report {
            day_absent: 1,
            ..Report::default()
        });
        let unread: Result<Report, String> = Err("no day file".to_owned());
        for (found, level, says, why) in [
            (&clean, Level::Info, "agrees with the days", ""),
            (
                &differs,
                Level::Info,
                "contradicted by none of it",
                "IST day 1: close pulled 2 folded 3",
            ),
            (
                &contradicted,
                Level::Warn,
                "contradicted by, or missing for",
                "IST day 2: minute high 9 above the pulled high 8",
            ),
            (&absent, Level::Warn, "contradicted by, or missing for", ""),
            (&unread, Level::Warn, "could not be checked", "no day file"),
        ] {
            let event = daycheck_event(found, "NIFTY", "2026-01");
            assert_eq!(event.level(), level, "{says}");
            assert_eq!(event.target(), "pull.daycheck");
            assert!(event.message().contains(says), "{}", event.message());
            assert_eq!(event.dropped_fields(), 0, "{says}: no field lost");
            let field = |key: &str| {
                event
                    .fields()
                    .iter()
                    .find(|(name, _)| *name == key)
                    .map(|(_, value)| format!("{value:?}"))
            };
            assert!(
                field("why").is_some_and(|w| w.contains(why)),
                "{says}: {:?}",
                field("why")
            );
            if found.is_ok() {
                assert_eq!(event.fields().len(), telemetry::MAX_FIELDS, "{says}");
            }
        }
        let event = daycheck_event(&contradicted, "NIFTY", "2026-01");
        let breach = event
            .fields()
            .iter()
            .find(|(name, _)| *name == "largest_breach")
            .map(|(_, value)| format!("{value:?}"));
        assert!(breach.is_some_and(|b| b.contains("Int(22985)")));
    }

    /// **A LOCK THAT CANNOT BE OPENED IS A REFUSAL, NOT A RUN WITHOUT ONE.**
    ///
    /// The `else` arm this replaces returned a guard holding nothing for *any*
    /// open failure, and the run then read-modify-wrote the census with no
    /// mutual exclusion at all: run A reads 20 entries, run B reads the same
    /// 20, A installs 20+A, B installs 20+B over the top, and A's entries are
    /// gone while both receipts read "every row accounted for".
    ///
    /// # Why a directory and not a permission bit
    ///
    /// The dangerous states are the ones that break the lock's own inode and
    /// leave the census beside it writable. A root-owned `.man.lock` is the
    /// one an operator hits, and it cannot be built in a test that must pass
    /// unprivileged and must also pass **as** root, where a mode of `0` is not
    /// a refusal at all. A directory at the lock's name is the same fault with
    /// the same consequence and no such dependency: `open` says `IsADirectory`
    /// on every host, for every user.
    ///
    /// The second half is what makes it a defect rather than an inconvenience.
    /// The census is created here **after** the refusal, from this same test, to
    /// prove the install would have gone through — so the old arm was not
    /// deferring to a failure that reports itself, it was running unlocked.
    #[test]
    fn a_lock_whose_own_path_is_unopenable_refuses_while_the_census_stays_writable() {
        let root = scratch("lock-is-a-dir");
        let census = root.join("manifest").join("dhan.man");
        std::fs::create_dir_all(census.with_extension("man.lock"))
            .expect("a directory occupying the lock's name");

        // `let Err … else` rather than `expect_err`, which would need a `Debug`
        // on the guard — and the guard holds a `File` it is careful not to
        // render.
        let Err(why) = CensusLock::take(&census) else {
            panic!("a lock that cannot be opened must refuse, never run unlocked")
        };
        assert!(
            why.contains("man.lock"),
            "the refusal names the path an operator has to fix: {why}"
        );
        assert!(
            why.contains("interleaved"),
            "and says what it prevented, not merely that something failed: {why}"
        );

        // THE HALF THAT MAKES IT A DEFECT. Nothing downstream would have
        // reported this: the census is a sibling of the lock, so it writes.
        std::fs::write(&census, b"the install had nothing stopping it")
            .expect("the census beside the broken lock is writable, which is the whole point");

        std::fs::remove_dir_all(&root).ok();
    }

    /// **A PATH FAILURE STILL DEFERS, AND THIS IS WHY THAT IS SAFE.**
    ///
    /// A file where the manifest DIRECTORY belongs breaks the lock and the
    /// census together, so `take` hands back a guard holding nothing and lets
    /// [`install_locked`] word the refusal — which is what
    /// `a_census_that_cannot_be_installed_names_what_is_left_uncounted` in
    /// `crates/pull/tests/census.rs` asserts from the outside.
    ///
    /// That deferral is only defensible while the install really does fail on
    /// the same cause, so the coupling is asserted here rather than reasoned
    /// about in a comment: an install driven with the deferred guard refuses.
    /// If a future change ever made this path installable, this test fails and
    /// the arm above it becomes a silent unlocked run.
    #[test]
    fn a_broken_directory_defers_because_the_install_fails_on_the_same_cause() {
        let root = scratch("dir-is-a-file");
        std::fs::remove_dir_all(root.join("manifest")).expect("make room for the file");
        std::fs::write(root.join("manifest"), b"NOT A DIRECTORY").expect("a file in the way");
        let census = root.join("manifest").join("dhan.man");

        let deferred = CensusLock::take(&census)
            .expect("a path failure is the install's to report, in better words");
        let why = install_locked(&deferred, &census, b"an image")
            .expect_err("the install must fail on the same cause the lock did");
        assert!(
            why.contains("could not be published"),
            "and that is the sentence the outside test expects: {why}"
        );
        assert!(
            !census.exists(),
            "nothing was published at the live path either"
        );

        std::fs::remove_dir_all(&root).ok();
    }

    /// **A LOCK THAT WILL NOT OPEN FOR A TRANSIENT OR SPACE REASON REFUSES.**
    /// pull2-1, D-2765.
    ///
    /// EMFILE, ENFILE and ENOMEM on a regular lock file, and ENOSPC creating a
    /// missing one, used to hand back a guard holding nothing, and the run did
    /// its census read-modify-write unserialised: the install does not fail on
    /// those causes, so concurrent installs lost rows silently. The host's
    /// kinds are driven directly because no test can make the host return them
    /// on demand; `OutOfMemory` and `StorageFull` are the kinds std gives
    /// ENOMEM and ENOSPC.
    #[test]
    fn a_transient_or_space_failure_on_the_lock_refuses_while_the_directory_stands() {
        let root = scratch("lock-transient");
        let census = root.join("manifest").join("dhan.man");
        let lock_path = census.with_extension("man.lock");
        std::fs::write(&census, b"a census that stays writable").expect("a census");

        // NO LOCK FILE YET, and creating it hit a full disk.
        let full = std::io::Error::from(std::io::ErrorKind::StorageFull);
        let Err(why) = CensusLock::unopened(&lock_path, &full) else {
            panic!("a lock that could not be created must refuse, never run unlocked")
        };
        assert!(why.contains("man.lock") && why.contains("Refused"), "{why}");

        // A REGULAR LOCK FILE that would not open for want of memory.
        std::fs::write(&lock_path, b"").expect("a regular lock file");
        let starved = std::io::Error::from(std::io::ErrorKind::OutOfMemory);
        let Err(why) = CensusLock::unopened(&lock_path, &starved) else {
            panic!("a regular lock that would not open must refuse, never run unlocked")
        };
        assert!(
            why.contains("directory that holds the census stands"),
            "{why}"
        );

        // THE BOUNDARY: with no directory to hold the census, the same kind
        // still defers to the install, which fails on that cause.
        std::fs::remove_dir_all(root.join("manifest")).expect("make room for the file");
        std::fs::write(root.join("manifest"), b"NOT A DIRECTORY").expect("a file in the way");
        assert!(
            CensusLock::unopened(&lock_path, &full).is_ok(),
            "a broken directory is the install's to report"
        );
        std::fs::remove_dir_all(&root).ok();
    }

    /// **AN IN-PLACE APPEND ENDS UNDER A STAMP NO READER COULD HAVE TAKEN
    /// WHILE IT WROTE.** census-1, D-2766.
    ///
    /// The hook stamps the census after its last slot is durable, which is the
    /// latest instant a reader inside the write could have stamped it: the
    /// kernel moved the times before copying the bytes, and nothing after the
    /// slot moved them again. Before D-2766 the final stamp WAS that stamp, so
    /// the api's cache kept whatever that reader saw under it indefinitely.
    #[test]
    fn an_in_place_append_moves_the_census_stamp_past_its_own_writes() {
        let root = scratch("census-stamp");
        let census = root.join("manifest").join("dhan.man");
        std::fs::write(&census, vec![0_u8; 4096]).expect("a census file");
        let append = super::Append {
            ordinal: 0,
            offset: 1024,
            bytes: [7; crate::manifest::ENTRY_LEN],
            commit: crate::manifest::Commit {
                slot: 0,
                offset: 64,
                bytes: [9; crate::manifest::IMAGE_LEN],
                durable_through: 1024 + 128,
                header: crate::manifest::ManifestHeader::genesis(brutex_core::vendor::Vendor::Dhan),
            },
        };
        let mut seen = None;
        super::write_appends_observed(&census, std::slice::from_ref(&append), |file| {
            seen = Some(
                file.metadata()
                    .and_then(|meta| meta.modified())
                    .expect("a stamp inside the write"),
            );
        })
        .expect("the append lands");
        let inside = seen.expect("the hook saw the landed slot");
        let after = std::fs::metadata(&census)
            .and_then(|meta| meta.modified())
            .expect("the final stamp");
        assert!(
            after > inside,
            "the final stamp must differ from any stamp taken during the write: \
             {inside:?} then {after:?}"
        );
        let bytes = std::fs::read(&census).expect("the census");
        assert_eq!(
            bytes.get(1024..1152),
            Some(&[7_u8; 128][..]),
            "the entry landed"
        );
        assert_eq!(bytes.get(64..128), Some(&[9_u8; 64][..]), "the slot landed");
        std::fs::remove_dir_all(&root).ok();
    }

    /// **A DROPPED CENSUS LOCK IS RELEASED WHILE A DUPLICATE OF ITS DESCRIPTOR
    /// IS STILL OPEN.** D-0693.
    ///
    /// The duplicate is the model of the reference a child spawned by another
    /// thread holds until its exec. While closing the descriptor was the
    /// release, that reference kept the lock alive and the next run was refused
    /// as "another ingest holds the census lock" with no other ingest running.
    #[test]
    #[expect(
        clippy::used_underscore_binding,
        reason = "the duplicate is taken of the held lock itself, which is the \
                  field whose drop this test is about"
    )]
    fn a_dropped_census_lock_is_released_despite_a_duplicated_descriptor() {
        let root = scratch("census-lock-dup");
        let census = root.join("manifest").join("dhan.man");

        let first = CensusLock::take(&census).unwrap_or_else(|why| panic!("a free lock: {why}"));
        let child = first
            ._held
            .as_deref()
            .expect("an openable lock path holds the lock")
            .try_clone()
            .expect("the duplicate a spawned child would hold");
        let Err(busy) = CensusLock::take(&census) else {
            panic!("the premise: a held census lock refuses a second run")
        };
        assert!(busy.contains("another ingest holds"), "{busy}");
        drop(first);

        let second = CensusLock::take(&census).unwrap_or_else(|why| {
            panic!("the dropped lock was released despite the duplicate: {why}")
        });
        drop(second);
        drop(child);
        std::fs::remove_dir_all(&root).ok();
    }

    /// **A FIFO AT THE LOCK'S NAME IS REFUSED, NOT WAITED ON.** R9-csr-cx-0,
    /// D-0955.
    ///
    /// The lock was opened write-only, and a write-only open of a FIFO blocks
    /// in open(2) until some process opens it for reading. The ingest neither
    /// refused nor ran. `take` runs on a thread here so a regression is a
    /// failed assertion after five seconds rather than a hung suite; the FIFO
    /// is then opened for reading, which is what releases a blocked writer.
    ///
    /// `mkfifo` is the external program, run from a test only: `CLAUDE.md` §2
    /// forbids a binding, and `std` has no FIFO constructor.
    #[test]
    fn a_fifo_at_the_lock_path_is_refused_rather_than_waited_on() {
        let root = scratch("LOCK-FIFO");
        let census = root.join("manifest").join("dhan.man");
        let lock_path = census.with_extension("man.lock");
        let made = std::process::Command::new("/usr/bin/mkfifo")
            .arg(&lock_path)
            .status()
            .expect("the FIFO this test is about can be made");
        assert!(made.success(), "the premise: a FIFO at the lock's name");

        let (tell, heard) = std::sync::mpsc::channel();
        let asked = census.clone();
        std::thread::spawn(move || {
            let answer = CensusLock::take(&asked).map(drop);
            let _gone = tell.send(answer);
        });
        let answer = heard.recv_timeout(std::time::Duration::from_secs(5));
        if answer.is_err() {
            // Release the writer blocked in open(2) before failing.
            let _reader = std::fs::File::open(&lock_path);
        }
        let Ok(Err(why)) = answer else {
            panic!("a FIFO at the lock path must refuse promptly, got {answer:?}")
        };
        assert!(why.contains("man.lock"), "the path is named: {why}");
        assert!(why.contains("not a regular file"), "and so is why: {why}");
        assert!(why.contains("(a FIFO)"), "and the type, in words: {why}");

        // A regular lock file at the same name is taken as before.
        std::fs::remove_file(&lock_path).expect("the FIFO is removable");
        let held = CensusLock::take(&census).map_err(|why| why.clone());
        assert!(held.is_ok(), "{:?}", held.map(drop));
        std::fs::remove_dir_all(&root).ok();
    }

    /// A UNIX socket bound AT `path` itself, whatever `path`'s length.
    ///
    /// A socket address holds a short path (`sun_path`: 104 bytes on macOS,
    /// 108 on Linux), and a scratch tree under a long TMPDIR is longer. The
    /// bind goes through a symbolic link to `path`'s directory named in a
    /// short directory, the temporary root or else `/tmp`: the kernel follows
    /// the link, so the socket file is made in `path`'s own directory under
    /// its own name, and only the address is the short spelling. The link is
    /// removed at once; the socket file stays. A rename from a short
    /// directory was the first way (GF-1), and it fails across volumes with
    /// `EXDEV` whenever TMPDIR and `/tmp` differ. No short directory that can
    /// hold the link is a panic naming each one tried and why, never a skip.
    /// `socket_at_binds_where_a_direct_bind_cannot` pins it.
    fn socket_at(path: &std::path::Path, tag: &str) -> std::os::unix::net::UnixListener {
        let dir = path.parent().expect("a socket path inside a directory");
        let file = path.file_name().expect("a socket path naming a file");
        let mut tried = Vec::new();
        for base in [std::env::temp_dir(), std::path::PathBuf::from("/tmp")] {
            let link = base.join(format!("brutex-sock-{}-{tag}", std::process::id()));
            let _stale = std::fs::remove_file(&link);
            if let Err(why) = std::os::unix::fs::symlink(dir, &link) {
                tried.push(format!("{}: {why}", link.display()));
                continue;
            }
            let bound = std::os::unix::net::UnixListener::bind(link.join(file));
            let _unlinked = std::fs::remove_file(&link);
            match bound {
                Ok(listener) => return listener,
                Err(why) => tried.push(format!("{}: {why}", link.display())),
            }
        }
        panic!(
            "no short directory carries a socket to {}: {}",
            path.display(),
            tried.join("; ")
        )
    }

    /// **A SOCKET IS BOUND AT A PATH TOO LONG FOR A SOCKET ADDRESS.** GF-1.
    /// Bound directly, the path is refused with `InvalidInput` wherever
    /// TMPDIR is; `socket_at` makes the socket at that very path, and no
    /// link is left behind.
    #[test]
    fn socket_at_binds_where_a_direct_bind_cannot() {
        use std::os::unix::fs::FileTypeExt as _;
        let root = scratch("LONG-SOCKET");
        let dir = root.join("D".repeat(120));
        std::fs::create_dir_all(&dir).expect("a deep directory");
        let path = dir.join("dhan.man.lock");
        let direct = std::os::unix::net::UnixListener::bind(&path);
        assert!(
            matches!(direct, Err(ref why) if why.kind() == std::io::ErrorKind::InvalidInput),
            "the premise: this path does not fit a socket address: {direct:?}"
        );
        let listener = socket_at(&path, "LONG");
        let found = std::fs::symlink_metadata(&path).expect("something at the path itself");
        assert!(
            found.file_type().is_socket(),
            "the socket itself, not a link: {:?}",
            found.file_type()
        );
        for base in [std::env::temp_dir(), std::path::PathBuf::from("/tmp")] {
            let link = base.join(format!("brutex-sock-{}-LONG", std::process::id()));
            assert!(
                std::fs::symlink_metadata(&link).is_err(),
                "no link is left at {}",
                link.display()
            );
        }
        drop(listener);
        std::fs::remove_dir_all(&root).ok();
    }

    /// **A SOCKET AT THE LOCK'S NAME IS REFUSED, NOT RUN WITHOUT A LOCK.**
    /// v3b-1, D-1480. Opening a UNIX socket fails with ENXIO, a kind that is
    /// neither `PermissionDenied` nor `IsADirectory`, so it fell into the
    /// path-failure arm and the ingest ran unserialised in silence. A dangling
    /// symlink whose target directory is missing fails the open with
    /// `NotFound`, the path-failure kind itself, and is the same misfiled name.
    #[test]
    #[expect(
        clippy::used_underscore_binding,
        reason = "whether the guard holds a lock is the fact under test, and the \
                  field is underscored because production never reads it"
    )]
    fn a_socket_at_the_lock_path_is_refused_rather_than_run_unlocked() {
        let root = scratch("LOCK-SOCKET");
        let census = root.join("manifest").join("dhan.man");
        let lock_path = census.with_extension("man.lock");
        // A socket's name must fit `sun_path` (104 bytes on macOS, 108 on
        // Linux), and under a long TMPDIR the lock path is longer: the bind
        // failed there and the property was never reached (GF-1, gate 1e).
        let listener = socket_at(&lock_path, "LOCK");
        assert!(
            std::os::unix::fs::FileTypeExt::is_socket(
                &std::fs::symlink_metadata(&lock_path)
                    .expect("something is at the lock's name")
                    .file_type()
            ),
            "the premise: a UNIX socket sits at the lock's name"
        );
        let opened = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&lock_path);
        let premise = opened.map(drop).map_err(|why| why.kind());
        assert!(
            !matches!(
                premise,
                Ok(())
                    | Err(std::io::ErrorKind::PermissionDenied | std::io::ErrorKind::IsADirectory)
            ),
            "the premise: a socket fails the open with a kind the refusal arm \
             did not name, got {premise:?}"
        );
        let refused = CensusLock::take(&census).map(drop);
        let Err(why) = refused else {
            panic!("a socket at the lock path must refuse, got {refused:?}")
        };
        assert!(why.contains("man.lock"), "the path is named: {why}");
        assert!(why.contains("not a regular file"), "and so is why: {why}");
        assert!(why.contains("a UNIX socket"), "and the type: {why}");
        drop(listener);
        std::fs::remove_file(&lock_path).expect("the socket is removable");

        // A dangling symlink into a directory that does not exist.
        std::os::unix::fs::symlink(root.join("nested").join("elsewhere"), &lock_path)
            .expect("the dangling symlink");
        let refused = CensusLock::take(&census).map(drop);
        let Err(why) = refused else {
            panic!("a dangling symlink at the lock path must refuse, got {refused:?}")
        };
        assert!(why.contains("a symbolic link"), "{why}");
        std::fs::remove_file(&lock_path).expect("the symlink is removable");

        // A genuine path failure still defers: the directory is gone, so
        // nothing occupies the name and the install will say why.
        let elsewhere = root.join("dir-is-a-file").join("manifest").join("dhan.man");
        std::fs::write(
            root.join("dir-is-a-file"),
            b"a file where the directory belongs",
        )
        .expect("the blocking file");
        let deferred = CensusLock::take(&elsewhere).map(|held| held._held.is_none());
        assert_eq!(
            deferred,
            Ok(true),
            "a path failure is the install's to report"
        );

        // With the name cleared, the lock is taken as before.
        let held = CensusLock::take(&census).map(|held| held._held.is_some());
        assert_eq!(held, Ok(true));

        // Every kind the refusal can meet is named in words.
        let kind = |at: &std::path::Path| {
            std::fs::symlink_metadata(at).map(|found| super::entry_kind(found.file_type()))
        };
        assert_eq!(kind(&root).ok(), Some("a directory"));
        assert_eq!(kind(&lock_path).ok(), Some("a regular file"));
        assert_eq!(
            kind(std::path::Path::new("/dev/null")).ok(),
            Some("a character device")
        );
        std::fs::remove_dir_all(&root).ok();
    }

    /// **ONLY A HELD LOCK IS REPORTED AS ANOTHER RUN.** R9-csr-cx-1, D-0955.
    ///
    /// Both refusals of `try_lock` became "another ingest holds the census
    /// lock". When the host refuses `flock` itself (`ENOLCK` on NFS without a
    /// lock daemon, `ENOTSUP` for a lock file it cannot lock) no other run
    /// exists and waiting never helps. Linux gives no regular file that refuses
    /// `flock`, so the mapping is driven directly, with each host error.
    #[test]
    fn a_host_refusal_to_lock_is_not_reported_as_another_run() {
        let path = std::path::Path::new("/store/manifest/dhan.man.lock");
        let busy = lock_refusal(path, std::fs::TryLockError::WouldBlock);
        assert!(busy.contains("another ingest holds"), "{busy}");
        for host in [
            std::io::Error::from(std::io::ErrorKind::Unsupported),
            std::io::Error::other("No locks available"),
            std::io::Error::from(std::io::ErrorKind::PermissionDenied),
        ] {
            let words = host.to_string();
            let why = lock_refusal(path, std::fs::TryLockError::Error(host));
            assert!(
                !why.contains("another ingest") && !why.contains("Wait for the other run"),
                "a host refusal names no other run: {why}"
            );
            assert!(why.contains(&words), "the host's own words: {why}");
            assert!(why.contains("man.lock"), "and the path: {why}");
        }
    }

    /// A bar at one instant with one close. Every other field is zero, which
    /// the store's own format calls a legal bar.
    fn bar(ts_micros: i64, close: i64) -> Bar {
        Bar {
            ts_micros,
            open: close,
            high: close,
            low: close,
            close,
            volume: 0,
            open_interest: i64::MIN,
        }
    }

    /// A committed header describing `n_valid` records between two instants.
    fn header(n_valid: u64, first: i64, last: i64) -> Header {
        Header {
            format_version: 2,
            record_stride: 56,
            flags: 0,
            generation: 1,
            n_valid,
            first_ts_micros: first,
            last_ts_micros: last,
            symbol_id: 1,
            timeframe_secs: 60,
        }
    }

    /// M-34 — a virgin month takes its closes from the batch it just wrote, and
    /// pays for no read at all.
    ///
    /// **This is the "costs nothing extra" claim, and it is a claim about a
    /// branch rather than about a stopwatch.** `closes_in_hand` returning `Some`
    /// is the only way the `None` arm — the two positional reads — is not
    /// reached, so proving the arm is not taken proves the reads are not issued.
    /// A timing test could not prove it and a mock would only prove the mock.
    ///
    /// The `None` cases are the whole reason the function exists: on a second
    /// window into a month that already holds bars, the batch is a SUFFIX, and
    /// its first close is not the month's first close. Recording it as one puts
    /// a fabricated base under every percentage computed from it.
    #[test]
    fn a_virgin_month_takes_its_closes_from_the_batch_it_just_wrote() {
        let batch = [
            bar(1_000, 295_050),
            bar(2_000, 300_000),
            bar(3_000, 310_025),
        ];

        // THE FILE IS THE BATCH: same count, same two instants. Free.
        assert_eq!(
            closes_in_hand(&header(3, 1_000, 3_000), &batch),
            Some((295_050, 310_025)),
            "the month's first and last close, with no read issued"
        );

        // The file holds MORE than the batch — the batch is a suffix. The first
        // close in hand belongs to the suffix, not to the month.
        assert_eq!(closes_in_hand(&header(9, 500, 3_000), &batch), None);
        // The counts agree and the FIRST instant does not: the batch is not the
        // prefix it looks like.
        assert_eq!(closes_in_hand(&header(3, 900, 3_000), &batch), None);
        // The counts agree and the LAST instant does not.
        assert_eq!(closes_in_hand(&header(3, 1_000, 4_000), &batch), None);
        // A count below the batch is not a state a committed append leaves, and
        // it is still not "the file is the batch".
        assert_eq!(closes_in_hand(&header(2, 1_000, 3_000), &batch), None);
        // An empty batch has no ends to read.
        assert_eq!(closes_in_hand(&header(0, 0, 0), &[]), None);

        // One bar is both ends of its own file.
        assert_eq!(
            closes_in_hand(&header(1, 7, 7), &[bar(7, 0)]),
            Some((0, 0)),
            "and a close of zero comes back as a zero, not as unknown"
        );
    }

    /// The largest census this build can write is accepted; one byte more is
    /// not.
    ///
    /// **A file of exactly the cap is legal**: [`MAX_CENSUS_BYTES`] is a census
    /// at `MAX_ENTRIES`, which `ManifestHeader::advance` accepts and refuses
    /// only one past. Written as `>=` the bounded read would refuse the
    /// largest legal census, and materialising that file means a 268 MB
    /// allocation, so the boundary is proved on a small cap through the same
    /// reader the census uses (D-2684).
    #[test]
    fn the_ceiling_admits_the_largest_census_this_build_can_write() {
        assert_eq!(
            MAX_CENSUS_BYTES, 268_468_224,
            "32,768 bytes of header region and 2,097,152 entries of 128 bytes"
        );
        let dir = scratch("census-ceiling");
        let path = dir.join("manifest").join("four.man");
        std::fs::write(&path, b"1234").expect("a four-byte file");
        assert_eq!(
            super::read_regular_capped(&path, 4).expect("it opens"),
            Ok(b"1234".to_vec()),
            "a file of exactly the cap is read whole"
        );
        assert_eq!(
            super::read_regular_capped(&path, 3).expect("it opens"),
            Err(super::Unbounded::PastCap { cap: 3, size: 4 }),
            "and one byte past it is refused"
        );
        std::fs::write(&path, b"").expect("an empty file");
        assert_eq!(
            super::read_regular_capped(&path, 0).expect("it opens"),
            Ok(Vec::new()),
            "an empty file is a first ingest"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// **CE-64. A CENSUS PATH THAT IS NOT A REGULAR FILE IS REFUSED UNREAD,
    /// AND A FIFO THERE NEVER HOLDS THE PULL.**
    ///
    /// `read_census` took `fs::metadata(path).len()`, which is 0 for a FIFO or
    /// a device, and then `fs::read` by path: a FIFO with no writer blocked the
    /// open for ever while this run held the census lock and the vendor's
    /// seat, and `/dev/zero` read until memory ran out. The api's reader of the
    /// same file already refuses both by name (P-19, R9-api-cx-1).
    #[test]
    fn a_census_path_that_is_not_a_regular_file_is_refused_unread_and_never_waits() {
        let dir = scratch("census-not-regular");
        let device = dir.join("manifest").join("device.man");
        std::os::unix::fs::symlink("/dev/null", &device).expect("a link to a device");
        let why = super::read_census(&device, brutex_core::vendor::Vendor::Groww)
            .map(|_| ())
            .expect_err("a device is not a census, and not an absent one either");
        assert!(why.contains(&device.display().to_string()), "{why}");

        let fifo = dir.join("manifest").join("fifo.man");
        assert!(
            std::process::Command::new("mkfifo")
                .arg(&fifo)
                .status()
                .expect("the test host runs its FIFO fixture")
                .success()
        );
        let (sent, answer) = std::sync::mpsc::channel();
        let reader = {
            let fifo = fifo.clone();
            std::thread::spawn(move || {
                let _ = sent.send(
                    super::read_census(&fifo, brutex_core::vendor::Vendor::Groww).map(|_| ()),
                );
            })
        };
        let read = answer.recv_timeout(std::time::Duration::from_secs(2));
        if read.is_err() {
            // Release the stuck open so the suite does not hang, then fail.
            let _ = std::fs::OpenOptions::new().write(true).open(&fifo);
        }
        let _ = reader.join();
        let why = read
            .expect("a FIFO at the census path must not hold the read")
            .expect_err("and it is refused rather than read as a census");
        assert!(why.contains(&fifo.display().to_string()), "{why}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// **A FAILURE CARRIES ITS OWN MONTH AND DAY** (DB-G45, D-3341 item 1).
    ///
    /// Every "member did not land" line of a pull was filed under
    /// `month=2018-01`, the first month of an 8.75-year window, because the
    /// failure carried nothing better. These are the doors that set the fields:
    /// a day sets its month too, a month alone leaves the day unset, and a
    /// failure about no month says so with `None` rather than a guess.
    #[test]
    fn a_failure_carries_its_own_month_and_day_never_the_window_s() {
        use super::{Failure, refused_day_failures, stamp_day};
        use crate::session::Day;
        use crate::vendor::TimestampEncoding;
        use store::path::YearMonth;

        let bare = Failure::new("NIFTY", "why");
        assert_eq!((bare.month, bare.day), (None, None));
        let month = YearMonth::new(2026, 9).expect("a month");
        let in_month = bare.clone().in_month(month);
        assert_eq!((in_month.month, in_month.day), (Some(month), None));
        let muhurat = Day::new(2020, 11, 14).expect("a day");
        let on_day = in_month.on_day(muhurat);
        assert_eq!(
            on_day.month,
            Some(YearMonth::new(2020, 11).expect("a month"))
        );
        assert_eq!(
            on_day.day,
            Some(muhurat),
            "the day wins over a month set before"
        );
        assert_eq!(
            (on_day.instrument.as_str(), on_day.why.as_str()),
            ("NIFTY", "why")
        );

        // A Muhurat refusal names the Muhurat's day, whatever the window was.
        let refused = refused_day_failures(
            "NIFTY",
            &[crate::fetch::RefusedDay {
                day: muhurat.days_from_epoch(),
                rows: 60,
            }],
        );
        assert_eq!(refused.len(), 1);
        let first = refused.first().expect("one refusal");
        assert_eq!(first.day, Some(muhurat));
        assert_eq!(
            first.month,
            Some(YearMonth::new(2020, 11).expect("a month"))
        );
        assert!(
            first
                .why
                .starts_with("2020-11-14: 60 minute row(s) refused")
        );
        // A day number the calendar cannot name is said, never guessed.
        let unnamed = refused_day_failures(
            "NIFTY",
            &[crate::fetch::RefusedDay {
                day: u32::MAX,
                rows: 1,
            }],
        );
        let unnamed = unnamed.first().expect("one refusal");
        assert_eq!((unnamed.month, unnamed.day), (None, None));
        assert!(unnamed.why.starts_with(&format!("day {}", u32::MAX)));

        // A vendor stamp's IST day, in each encoding's own arithmetic: 18:29:59
        // UTC on 2 Dec 2019 is 23:59:59 IST that day, and a second later the
        // next day; an IST-text stamp is already IST.
        let utc = 1_575_311_399_i64; // 2019-12-02 18:29:59 UTC
        let dec2 = Day::new(2019, 12, 2).expect("a day");
        let dec3 = Day::new(2019, 12, 3).expect("a day");
        assert_eq!(
            stamp_day(utc, TimestampEncoding::EpochSecondsUtc),
            Some(dec2)
        );
        assert_eq!(
            stamp_day(utc + 1, TimestampEncoding::EpochSecondsUtc),
            Some(dec3)
        );
        assert_eq!(
            stamp_day(utc * 1_000, TimestampEncoding::EpochMillisUtc),
            Some(dec2)
        );
        assert_eq!(
            stamp_day(utc + 1, TimestampEncoding::IsoDateTimeOffset),
            Some(dec3)
        );
        let ist_text = utc + crate::session::IST_OFFSET_SECS;
        assert_eq!(
            stamp_day(ist_text, TimestampEncoding::IstDateTimeText),
            Some(dec2)
        );
        assert_eq!(
            stamp_day(ist_text + 1, TimestampEncoding::IsoDateTimeText),
            Some(dec3)
        );
        assert_eq!(
            stamp_day(i64::MIN / 2_000, TimestampEncoding::EpochSecondsUtc),
            None
        );
    }

    /// The one month a member's rows are about, or none when they span two.
    #[test]
    fn a_shortfall_names_its_month_only_when_its_rows_have_one() {
        use super::one_month;
        use crate::manifest::{Closes, Entry, Held};
        use store::path::YearMonth;

        let row = |month: u8| {
            Held::new(
                Entry {
                    key: EntryKey {
                        contract: None,
                        exchange: brutex_core::instrument::Exchange::Nse,
                        segment: brutex_core::instrument::Segment::Index,
                        symbol: brutex_core::symbol::Symbol::new("NIFTY").expect("a symbol"),
                        timeframe: Timeframe::MINUTE_1,
                        month: YearMonth::new(2026, month).expect("a month"),
                    },
                    rows: 1,
                    first_ts_micros: 1,
                    last_ts_micros: 1,
                },
                Closes::UNKNOWN,
            )
        };
        assert_eq!(one_month(&[]), None);
        assert_eq!(
            one_month(&[row(9), row(9)]),
            Some(YearMonth::new(2026, 9).expect("a month"))
        );
        assert_eq!(one_month(&[row(8), row(9)]), None);
    }
}

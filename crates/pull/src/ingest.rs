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

use std::fs;
use std::io::{ErrorKind, Write as _};
use std::path::{Path, PathBuf};

use brutex_core::instrument::{Exchange, Segment};
use brutex_core::vendor::Vendor;
use store::file::BarFile;
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
    let done = from_members_inner(members, store_root, plan);
    note_run(&done);
    done
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
fn note_run(done: &Ingested) {
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::info("pull.run", "ingested")
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
fn derived_shortfall(member: &Member, landed: &Landed) -> Option<Failure> {
    if landed.derived == landed.derived_expected {
        return None;
    }
    Some(Failure {
        instrument: member.instrument.clone(),
        why: format!(
            "{} of {} derived rung(s) landed. The pulled rung is on disk and the \
             folded ones are not, so this instrument-month reads as held at one \
             timeframe and missing at the rest. Why each rung failed is on the \
             log at `pull.derive`; this is the count that stops the run \
             reporting itself clean.",
            landed.derived, landed.derived_expected
        ),
    })
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
fn from_members_inner(members: &[Member], store_root: &Path, plan: Plan<'_>) -> Ingested {
    let Plan { vendor, .. } = plan;
    let mut done = Ingested {
        members: members.len(),
        ..Ingested::default()
    };

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
        return refused_whole(members, &census_path, why);
    }

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
    let census_lock = match CensusLock::take(&census_path) {
        Ok(guard) => guard,
        Err(why) => return refused_whole(members, &census_path, why),
    };

    let mut census = match read_census(&census_path, vendor) {
        Ok(census) => census,
        Err(why) => return refused_whole(members, &census_path, why),
    };

    // A census that loaded degraded may still be appended to — recovering from
    // a torn commit *is* writing the next generation — but never silently.
    // What it recovered is a prefix: months a newer generation had committed
    // are not in it and cannot be got back from it, only from the bar files.
    let repairing = census.degraded_reason().is_some();
    if let Some(why) = census.degraded_reason() {
        note_census_degraded(&census_path, &why.to_string());
        done.failures.push(Failure {
            instrument: census_path.display().to_string(),
            why: format!(
                "the census loaded DEGRADED at generation {} after stepping \
                 over: {why}. This run's install is the repair, and any month \
                 a newer generation had committed is not in it — that is \
                 recoverable only by walking bars/",
                census.header().generation
            ),
        });
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
        match one(member, store_root, plan) {
            Ok(landed) => {
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
                for reason in [
                    crate::session::DropReason::BeforeSessionOpen,
                    crate::session::DropReason::AtOrAfterSessionClose,
                    crate::session::DropReason::BeforeWindow,
                    crate::session::DropReason::AfterWindow,
                ] {
                    for _ in 0..landed.census.of(reason) {
                        done.census.count(reason);
                    }
                }
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
                for held in landed.entries {
                    match count(&mut census, held) {
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
                            done.failures.push(Failure {
                                instrument: member.instrument.clone(),
                                why: format!(
                                    "{} holds {} bar(s) the census does not count: {why}",
                                    member.instrument, held.entry.rows
                                ),
                            });
                        }
                    }
                }
            }
            Err(why) => {
                note_not_landed(member, &why);
                done.failures.push(Failure {
                    instrument: member.instrument.clone(),
                    why,
                });
            }
        }
    }

    // ONE INSTALL, AFTER THE LOOP — and none at all when nothing changed, so
    // a re-run of the same folder leaves the census byte for byte as it was.
    if let Err(why) = install_census(&census_lock, &census_path, &census, &appends, publish) {
        note_census_unpublished(&census_path, appends.len(), &why);
        done.failures.push(Failure {
            instrument: census_path.display().to_string(),
            why: format!(
                "{} slice(s) are on disk and the census that counts them was \
                 not published: {why}",
                done.counted
            ),
        });
    }

    done
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
        failures: vec![Failure {
            instrument: about.display().to_string(),
            why,
        }],
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
    let member = Member {
        path: std::path::PathBuf::from(origin),
        instrument: instrument.to_owned(),
        rows: raw.rows.clone(),
    };
    from_members(std::slice::from_ref(&member), store_root, plan)
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
fn record_all(store_root: &Path, vendor: Vendor, held: &[Held]) -> Option<String> {
    let census_path = crate::manifest::manifest_path(store_root, vendor);
    let lock = match CensusLock::take(&census_path) {
        Ok(lock) => lock,
        Err(why) => return Some(why.clone()),
    };
    let mut census = match read_census(&census_path, vendor) {
        Ok(census) => census,
        Err(why) => return Some(why),
    };
    // RESERVED FROM THE BOUND IN HAND — at most one append per entry offered.
    let mut appends: Vec<Append> = Vec::with_capacity(held.len());
    for one in held {
        match count(&mut census, *one) {
            Ok(Some(append)) => appends.push(append),
            Ok(None) => {}
            Err(why) => return Some(why),
        }
    }
    if let Err(why) = install_census(&lock, &census_path, &census, &appends, false) {
        note_census_unpublished(&census_path, appends.len(), &why);
        return Some(why);
    }
    None
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
/// Two opens and two appends per call, both O(1). Nothing scans.
#[must_use]
pub fn from_rows(
    bars: &[store::format::Bar],
    overlays: &[store::format::Overlay],
    instrument: &str,
    origin: &str,
    store_root: &Path,
    plan: Plan<'_>,
) -> Ingested {
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
        return done;
    }
    let Some((first, last)) = bars.first().zip(bars.last()) else {
        return done;
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
            done.failures.push(Failure {
                instrument: instrument.to_owned(),
                why,
            });
            return done;
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
    let held = write_and_count(
        bars,
        store_root,
        symbol_id,
        parts,
        EntryKey {
            contract: plan.contract,
            exchange,
            segment,
            symbol,
            timeframe,
            month: ym,
        },
    );
    let one = match held {
        Ok((one, committed)) => {
            done.bars_stored = bars.len();
            done.bars_committed = committed;
            one
        }
        Err(why) => {
            done.failures.push(Failure {
                instrument: instrument.to_owned(),
                why,
            });
            return done;
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
    // COUNTED, BECAUSE IT WILL BE. The row is going to the caller's batch and
    // the caller returns an error if the batch cannot be published, so there is
    // no path where this reports counted and the census does not hold it —
    // which is the property `Ingested::counted`'s own doc asserts: a member
    // that stored bars is either in this count or named in `failures`, with no
    // third place to be.
    done.counted = 1;

    // THE OVERLAY AFTER THE BARS, NEVER BEFORE. If the bar write fails there is
    // nothing for an overlay row to be a column of, and a sidecar describing
    // bars that are not there is worse than no sidecar: the next reader joins
    // on a stamp that has no bar.
    if let Err(why) = write_overlay(overlays, store_root, symbol_id, parts) {
        done.failures.push(Failure {
            instrument: instrument.to_owned(),
            why,
        });
    }
    name_the_origin(&mut done, origin);
    done
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
            Ok(None) => kept.push(*bar),
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
fn nothing_landed(census: DropCensus, folded: usize, outside_session: u32) -> Landed {
    Landed {
        committed: 0,
        bars: 0,
        folded,
        census,
        entries: Vec::new(),
        derived: 0,
        derived_expected: 0,
        outside_session,
    }
}

fn one(member: &Member, store_root: &Path, plan: Plan<'_>) -> Result<Landed, String> {
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
    let raw = fetch::RawWindow {
        rows: member.rows.clone(),
    };
    let mut landed = fetch::land(&raw, request, encoding, scale).map_err(|why| why.to_string())?;
    // Bound once so the three returns below cannot disagree. See the field.
    let outside_session = landed.outside_session;
    if landed.bars.is_empty() {
        return Ok(nothing_landed(landed.census, 0, outside_session));
    }

    // THE FOLD. Without it a real run produced 354,675 rows and ZERO bars:
    // both archive vendors ship one-second snapshots with two to four rows per
    // second and no sub-second field, and the store correctly refuses two
    // records claiming the same instant.
    //
    // The bucket width comes from the TIMEFRAME the caller is filing under, so
    // a one-second feed, a one-minute feed and a daily feed all fold through
    // this same line. Nothing here knows which vendor it came from.
    let folded = fold_in_place(&mut landed.bars, timeframe, member)?;

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

    // `fold` returns at least one bar for an input that had at least one, and
    // the empty input already returned above — but the type does not say so,
    // and the arm that says "this cannot happen" is a `Landed` with nothing in
    // it rather than a panic.
    let (Some(first), Some(last)) = (landed.bars.first(), landed.bars.last()) else {
        return Ok(nothing_landed(landed.census, folded, outside_session));
    };
    // THE MONTH, AND THE REFUSAL IF THE BARS CROSS ONE. See `month_of`.
    let ym = month_of(first, last)?;
    // THE MEMBER'S IDENTITY, PARSED BEFORE ANY FILE IS OPENED. See `identify`.
    let Identity {
        symbol,
        exchange,
        segment,
        symbol_id,
    } = identify(&member.instrument, exchange, segment)?;

    // THE PATH IS RENDERED FROM THE VALUES THE KEY IS BUILT FROM, not from the
    // plan's strings a second time. `exchange.as_str()` is the parsed
    // `Exchange` speaking, so the directory a bar lands in and the code the
    // census records cannot disagree — there is only one value.
    // See `write_and_count` for why the path refusal below is a backstop.
    let parts = PathParts {
        vendor,
        exchange: exchange.as_str(),
        segment: segment.as_str(),
        symbol: symbol.as_str(),
        contract: plan.contract,
        timeframe,
        month: ym,
        file: FileKind::Bars,
    };
    let (pulled, committed) = write_and_count(
        &landed.bars,
        store_root,
        symbol_id,
        parts,
        EntryKey {
            contract: plan.contract,
            exchange,
            segment,
            symbol,
            timeframe,
            month: ym,
        },
    )
    .map_err(|why| format!("{}: {why}", member.instrument))?;
    let mut entries = vec![pulled];

    // THE SEVEN THAT WERE NEVER PULLED. Split into its own function only to
    // stay under clippy's 100-line ceiling for `one`; the argument for it is
    // there.
    // NO `if timeframe == MINUTE_1` GATE. Whatever rung was pulled, everything
    // derivable FROM it is derived — the rule decides, not a condition here. A
    // daily pull derives nothing because nothing coarser is a whole multiple of
    // it that is not itself the day; a one-second pull would derive the whole
    // ladder the day `Timeframe::KNOWN` gains a second. Neither case needs a
    // line changed. See `derived_from`.
    derive_all(
        &landed.bars,
        &member.instrument,
        store_root,
        symbol_id,
        timeframe,
        DeriveInto {
            // THE CONTRACT THE BARS WERE FILED UNDER, which is what
            // `derive_all` reads its option/future line off. This was a
            // hardcoded `None` carrying a note that the contract path was not
            // reachable from here yet. It is now, and leaving the note in place
            // would have filed an option's derived rungs in the underlying's
            // own directory.
            contract: plan.contract,
            vendor,
            exchange,
            segment,
            symbol,
            month: ym,
        },
        &mut entries,
    );

    let derived = entries.len().saturating_sub(1);
    // THE RULE, NOT A LIST. `derived_count_in` is what `derive_rungs` walks, so
    // a rung added to `Timeframe::KNOWN` moves both sides at once and this
    // expectation cannot go stale against it. It already knows the one case
    // where zero is correct: an option contract folds into no rung.
    let derived_expected = derived_count_in(plan.contract, timeframe);
    Ok(Landed {
        bars: landed.bars.len(),
        // WRITTEN, AS DISTINCT FROM OFFERED. See `Ingested::bars_stored`.
        committed,
        folded,
        census: landed.census,
        entries,
        derived,
        derived_expected,
        outside_session,
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
struct Identity {
    symbol: brutex_core::symbol::Symbol,
    exchange: Exchange,
    segment: Segment,
    symbol_id: u32,
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
struct DeriveInto {
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
    into: DeriveInto,
    entries: &mut Vec<Held>,
) {
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
    if into.contract.is_some_and(|c| c.is_option()) {
        return;
    }
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
        match derive(source_bars, rung, store_root, symbol_id, parts, key) {
            Ok(held) => entries.push(held),
            Err(why) => note_not_derived(instrument, rung, &why),
        }
    }
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

/// [`derived_count`]'s set. Private because the rungs themselves are this
/// module's business; the COUNT is what a caller can meaningfully assert on.
fn derived_from(source: Timeframe) -> impl Iterator<Item = Timeframe> {
    Timeframe::KNOWN.iter().copied().filter(move |target| {
        target.secs() > source.secs()
            && target.secs().is_multiple_of(source.secs())
            && target.secs() != Timeframe::DAY_1.secs()
    })
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
    rung: Timeframe,
    store_root: &Path,
    symbol_id: u32,
    parts: PathParts<'_>,
    key: EntryKey,
) -> Result<Held, String> {
    let bucket = crate::fold::Bucket::of_secs(rung.secs())
        .ok_or("a timeframe of zero seconds has no bucket to fold into")?;
    let bars = crate::fold::fold(minutes, bucket).map_err(|why| why.to_string())?;
    if bars.is_empty() {
        return Err(format!("{} folded to no bars", rung.as_str()));
    }
    // ONLY THE CENSUS ROW. A derived rung's committed count is not reported
    // separately — the receipt's `derived` line already says how many rungs
    // landed, and a second number for the same fact would be a second thing to
    // keep in step.
    write_and_count(&bars, store_root, symbol_id, parts, key).map(|(held, _)| held)
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
    let mut file =
        BarFile::open_or_create(store_root, path, symbol_id).map_err(|why| why.to_string())?;
    // THE DISCRIMINANT IS KEPT, AND IT WAS THROWN AWAY. `Committed` and
    // `AlreadyPresent` are the difference between a run that wrote a month and
    // one that re-offered it, and `?` on its own erased that.
    let committed = matches!(
        file.append(bars).map_err(|why| why.to_string())?,
        store::file::Appended::Committed { .. }
    );
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
        // WRITTEN, NOT MERELY OFFERED. Zero when the month already held this
        // batch byte for byte — the number that makes a re-run distinguishable
        // from a first run on a receipt.
        if committed { bars.len() } else { 0 },
    ))
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
/// One open and one append. O(1) per call.
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
/// One `fnv1a` over the symbol, one open and one append. **O(1) per call**,
/// O(rows) in the bytes written and nothing else.
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

/// Whether a file of `len` bytes is larger than this build could have written.
///
/// A named predicate rather than the comparison written inline, because the
/// boundary is the whole content of the check and **a file of exactly
/// [`MAX_CENSUS_BYTES`] is legal**: it is a census at [`MAX_ENTRIES`], which
/// `ManifestHeader::advance` accepts and refuses only one past. Written as
/// `>=` it would refuse the largest legal census, and no test that could
/// afford to build one would ever notice — materialising that file means a
/// 134 MB allocation, which is not a thing a unit test should do. So the
/// ceiling is verified as **arithmetic** here, exactly as `pull::manifest`
/// verifies its own, and `pull::ingest::tests` is the only place that can
/// reach it without one.
const fn beyond_ceiling(len: u64) -> bool {
    len > MAX_CENSUS_BYTES
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
    let bytes = match fs::metadata(path) {
        // THE SIZE IS CHECKED BEFORE THE READ. `read` on a file this process
        // cannot hold is not an error it can report — it is an allocator
        // failure or an OOM kill, and neither reaches the operator as "that
        // census is too big".
        Ok(found) if beyond_ceiling(found.len()) => {
            return Err(format!(
                "{} is {} bytes; the largest census this build can write is \
                 {MAX_CENSUS_BYTES} and this reader refuses more",
                path.display(),
                found.len()
            ));
        }
        Ok(_) => {
            fs::read(path).map_err(|why| format!("{} could not be read: {why}", path.display()))?
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
                "{} could not be measured: {why}. A census that exists and \
                 cannot be read is not one that does not exist, and this run \
                 will not start a second census over it",
                path.display()
            ));
        }
    };
    Manifest::open_image(vendor, &bytes).map_err(|why| format!("{}: {why}", path.display()))
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
/// Dropping the [`File`] is what releases the month, so the field is live for
/// its whole life and read exactly once, by the compiler-generated drop.
struct CensusLock {
    _held: Option<fs::File>,
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
        let lock_path = path.with_extension("man.lock");
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
        let lock = match fs::OpenOptions::new()
            .create(true)
            .truncate(false)
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
            // the day feeds began running concurrently. Enumerated by kind
            // rather than inverted into "refuse unless the path is at fault"
            // because a name past the host's limit is `InvalidFilename`, is the
            // path's fault, and must keep deferring —
            // `a_census_that_cannot_be_measured_stops_the_run` builds exactly
            // that and expects the install's wording.
            Err(why)
                if matches!(
                    why.kind(),
                    ErrorKind::PermissionDenied | ErrorKind::IsADirectory
                ) =>
            {
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
            // Every other cause is the path's, and the install reports it.
            Err(_) => return Ok(Self { _held: None }),
        };
        if lock.try_lock().is_err() {
            return Err(format!(
                "another ingest holds the census lock at {}. Refused rather \
                 than queued: two runs installing at once silently discard one, and \
                 the loser's receipt still reads 'every row accounted for' \
                 because its own books balanced. Wait for the other run and try \
                 again.",
                lock_path.display()
            ));
        }
        Ok(Self { _held: Some(lock) })
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

/// One entry write, one barrier and one slot write per append, in that order.
///
/// The barrier is not optional and not an optimisation to skip: issuing the
/// slot write before the entry bytes are on stable storage publishes a counter
/// over entries that may not exist, which is the one way this format can be
/// made to lie. [`Commit::durable_through`] names the offset, so a writer
/// cannot claim it did not know which one to flush.
fn write_appends(path: &Path, appends: &[Append]) -> std::io::Result<()> {
    use std::io::{Seek, SeekFrom};

    let mut file = fs::OpenOptions::new().read(true).write(true).open(path)?;
    for append in appends {
        file.seek(SeekFrom::Start(append.offset))?;
        file.write_all(&append.bytes)?;

        // THE BARRIER. Everything through here must be durable before the slot
        // below is allowed to count it.
        debug_assert!(append.commit.durable_through <= append.offset + ENTRY_STRIDE);
        file.sync_data()?;

        file.seek(SeekFrom::Start(append.commit.offset))?;
        file.write_all(&append.commit.bytes)?;
        file.sync_all()?;
    }
    Ok(())
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

    use super::{CensusLock, MAX_CENSUS_BYTES, beyond_ceiling, closes_in_hand, install_locked};

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
    #[test]
    fn the_ceiling_admits_the_largest_census_this_build_can_write() {
        assert_eq!(
            MAX_CENSUS_BYTES, 268_468_224,
            "32,768 bytes of header region and 2,097,152 entries of 128 bytes"
        );
        assert!(
            !beyond_ceiling(MAX_CENSUS_BYTES),
            "a census at exactly MAX_ENTRIES is one this build writes, and \
             refusing it would refuse a legal file"
        );
        assert!(!beyond_ceiling(MAX_CENSUS_BYTES - 1));
        assert!(
            beyond_ceiling(MAX_CENSUS_BYTES + 1),
            "and one byte past it is not a census this build could have made"
        );
        assert!(beyond_ceiling(u64::MAX));
        assert!(!beyond_ceiling(0), "an empty file is a first ingest");
    }
}

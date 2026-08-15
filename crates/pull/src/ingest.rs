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
    /// Bars written to the store.
    pub bars_stored: usize,
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
    from_members_inner(members, store_root, plan)
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

/// A DERIVED RUNG THAT WOULD NOT FILE, at `warn` and never silently.
///
/// The minute bars are already committed when this fires, so the member did not
/// fail — a copy of it did. That distinction is the whole reason this is a log
/// line rather than a `Failure` on the receipt: a run that stored every minute
/// it was asked for and could not fold one of them into thirty is not a run
/// that failed, and reporting it as one would send an operator to re-pull data
/// he already has. `CLAUDE.md` §4 still requires it NAMED, which is this.
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

/// What one member put on disk, and the counter row that describes it.
struct Landed {
    /// Bars the member offered to the store.
    bars: usize,
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
    if landed.bars.is_empty() {
        return Ok(Landed {
            bars: 0,
            folded: 0,
            census: landed.census,
            entries: Vec::new(),
            derived: 0,
        });
    }

    // THE FOLD. Without it a real run produced 354,675 rows and ZERO bars:
    // both archive vendors ship one-second snapshots with two to four rows per
    // second and no sub-second field, and the store correctly refuses two
    // records claiming the same instant.
    //
    // The bucket width comes from the TIMEFRAME the caller is filing under, so
    // a one-second feed, a one-minute feed and a daily feed all fold through
    // this same line. Nothing here knows which vendor it came from.
    let bucket = crate::fold::Bucket::of_secs(timeframe.secs())
        .ok_or("a timeframe of zero seconds has no bucket to fold into")?;
    let snapshots = landed.bars.len();
    landed.bars = crate::fold::fold(&landed.bars, bucket).map_err(|why| why.to_string())?;
    // Measured at the fold, where the two lengths are both in hand. `fold`
    // emits one bar per bucket that held anything and never invents one, so
    // this subtraction cannot go negative.
    let folded = snapshots.saturating_sub(landed.bars.len());

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
    note_fold(member, snapshots, landed.bars.len(), folded, bucket);

    // `fold` returns at least one bar for an input that had at least one, and
    // the empty input already returned above — but the type does not say so,
    // and the arm that says "this cannot happen" is a `Landed` with nothing in
    // it rather than a panic.
    let (Some(first), Some(last)) = (landed.bars.first(), landed.bars.last()) else {
        return Ok(Landed {
            bars: 0,
            folded,
            census: landed.census,
            entries: Vec::new(),
            derived: 0,
        });
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
    //
    // A consequence worth naming: with the venue a closed enum, the symbol a
    // `Symbol` (whose byte set is a subset of `store::path::check_segment`'s
    // and whose capacity is the same 24), the vendor a `Vendor`, the timeframe
    // one of `Timeframe::KNOWN` and the month a `YearMonth`, this refusal is
    // no longer reachable from here. It stays because `StorePath::new` is the
    // only thing that renders a path and a caller that skipped it would be
    // asserting the above rather than checking it — but it is a backstop now,
    // and `docs/06-limits.md` should not be told it is exercised.
    let parts = PathParts {
        vendor,
        exchange: exchange.as_str(),
        segment: segment.as_str(),
        symbol: symbol.as_str(),
        contract: None,
        timeframe,
        month: ym,
        file: FileKind::Bars,
    };
    let mut entries = vec![
        write_and_count(
            &landed.bars,
            store_root,
            symbol_id,
            parts,
            EntryKey {
                contract: None,
                exchange,
                segment,
                symbol,
                timeframe,
                month: ym,
            },
        )
        .map_err(|why| format!("{}: {why}", member.instrument))?,
    ];

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
            // SPOT INGEST. The contract path is not reachable from here yet;
            // when it is, this carries the contract the bars were filed under
            // and `derive_all` reads the option/future line off it.
            contract: None,
            vendor,
            exchange,
            segment,
            symbol,
            month: ym,
        },
        &mut entries,
    );

    let derived = entries.len().saturating_sub(1);
    Ok(Landed {
        bars: landed.bars.len(),
        folded,
        census: landed.census,
        entries,
        derived,
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
            contract: None,
            timeframe: rung,
            month: into.month,
            file: FileKind::Bars,
        };
        let key = EntryKey {
            contract: None,
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
    write_and_count(&bars, store_root, symbol_id, parts, key)
}

/// Appends `bars` under `parts` and answers the census row for what the FILE
/// now holds — not for the batch that was offered.
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
) -> Result<Held, String> {
    let path = StorePath::new(parts).map_err(|why| why.to_string())?;
    let mut file =
        BarFile::open_or_create(store_root, path, symbol_id).map_err(|why| why.to_string())?;
    file.append(bars).map_err(|why| why.to_string())?;
    let header = file.header();
    let closes = month_closes(&file, &header, bars)?;
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
    /// `Ok(None)` inside the guard means the lock file could not be *opened* at
    /// all, which happens when the directory is not a directory. That is not
    /// this function's fault to report: the install fails on the same cause a
    /// moment later and names it in the words an operator needs, and a test
    /// that puts a file where the directory belongs expects that sentence
    /// rather than a worse-worded twin from here.
    fn take(path: &Path) -> Result<Self, String> {
        // BEST EFFORT, and deliberately not `?`. The lock lives beside the
        // census and on a first ever run neither exists, so the directory is
        // attempted here — but a failure to create it is `install_locked`'s to
        // report, for the reason above.
        if let Some(dir) = path.parent() {
            let _ = fs::create_dir_all(dir);
        }
        let lock_path = path.with_extension("man.lock");
        let Ok(lock) = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(&lock_path)
        else {
            return Ok(Self { _held: None });
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

/// The two facts about this module no external test can reach.
///
/// Most of it is proved from outside, in `crates/pull/tests/census.rs`, because
/// that is where a caller stands — a folder goes in, bars and a counter come
/// out, and the two are checked against each other. These two are here because
/// one needs a 268,468,224-byte file on the disk to reach from outside, and the
/// other is about a read that **does not happen**, which a caller cannot
/// observe by definition. A boundary that is only ever tested one side of is a
/// boundary nobody has checked; `pull::manifest` makes the same argument about
/// [`MAX_ENTRIES`] and verifies it the same way.
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

    use super::{MAX_CENSUS_BYTES, beyond_ceiling, closes_in_hand};

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

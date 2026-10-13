//! The GDFL import: indices, stocks and option contracts, read from the
//! vendor's zips or the verified tick store, folded to one-second OHLCV bars
//! and filed through the one ingest door. D-2802 to D-2806.
//!
//! # One second, and nothing coarser (operator, 4 Oct 2026, D-2802)
//!
//! GDFL supplies the one-second rung ONLY. This module writes `1s` and no
//! other rung, and `crate::ingest` derives no coarser rung from a `gdfl`
//! bar on any path (`ingest::derives_for`): minutes and every higher
//! timeframe come from the Zerodha one-minute pulls, derived by the app.
//!
//! # One runtime for every kind
//!
//! A source only LISTS a day and READS its files ([`crate::gdfl_cm`] for the
//! capital-market trees, [`crate::gdfl_nfo`] for options); everything after
//! that is this module, once, for all three kinds: the calendar and venue
//! gate, the journal, the row rule, the placement of late rows, the fold and
//! the write. Two sources and three kinds meet in [`drive`], so a bar from a
//! zip and a bar from the tick store take byte for byte the same path to
//! disk, and a run over either leaves the same store (`CM-GI-02`).
//!
//! # The bar (D-2802, the agreed definition of 4 Oct 2026)
//!
//! - **Stocks and options:** rows with LTQ > 0 only; open the first such
//!   row of the second, high and low the extremes, close the last (file
//!   order), volume the sum of LTQ. A second with no traded row has no bar.
//! - **Index spot:** every row (an index row carries no quantity), volume 0.
//! - **Open interest:** the options file's `OpenInterest` of the LAST row
//!   placed in the second, traded or not (D-3342, [`BAR_DEFINITION`] 3, the
//!   operator's answer: bar-close semantics; definition 2 took the last
//!   traded row's); `OI_NULL` for the capital-market trees, which carry none.
//! - The second is folded by [`crate::fold`], the one fold authority
//!   (`fold::fold_seconds_with_last_oi` for options).
//!
//! # A late row is deferred, never moved earlier (D-0805's rule, D-2802)
//!
//! [`crate::fold`] refuses a back-step, and the vendor's files hold them
//! (the design's §1.3 measures single late rows, replays and a forward
//! stamp). So before the fold every kept row is PLACED: a row stamped at or
//! after the running maximum is in order and keeps its stamp; a late row is
//! placed at the stamp of the next in-order row after it in the file, the
//! earliest second it is proven to have printed by. A late row with no
//! in-order row after it is dropped and counted (`late_unresolved`). A price
//! can be delayed, never advanced, so no bar holds a print from its future
//! (`CLAUDE.md` §3 rule 7). The design's UTC re-stamp is NOT applied: such a
//! row is deferred like any other late row, which is the conservative
//! direction, and is counted in `late_rows`.
//!
//! # Incremental, idempotent, resumable (D-2803)
//!
//! The store itself refuses a duplicate: `BarFile::append` answers
//! `AlreadyPresent` for bars it holds and appends only a following suffix.
//! On top of that a journal, `<store>/imports/gdfl.journal`, records `begin`
//! before a day's first write and one closing line after the day, `done`,
//! `incomplete` or `refused` (the lines and what decides them are below,
//! D-3317), keyed by kind, day and instrument filter. A re-run skips every
//! day done under this bar definition without
//! reading the source (O(1) per day), writes nothing and leaves the store
//! byte for byte as it was. A `begin` with no closing line is a crashed run,
//! and a day closed `incomplete` is an unfinished one: each is imported again
//! and named in [`Report::resumed`], the store keeping what it holds and
//! appending the rest, the census re-counted (D-2821).
//!
//! Every `done` and `incomplete` line carries `definition=<n>`, the
//! [`BAR_DEFINITION`] that built the day (D-2822). A `done` line with no
//! `definition=` whose stats are exactly the six fields D-2811 wrote
//! (`files= seconds= failures= late= late_unresolved= max_back_s=`) was
//! written by the code of D-2811 and D-2812, which landed together, and is
//! read as definition 2: a journal an earlier build of this branch wrote is
//! this store's own and its finished days stay finished. Any other `done`
//! line naming another definition, or none, is not done: the day is imported
//! again and named in `resumed` and [`Report::restated`]. The store is
//! append-only, so it answers `AlreadyPresent` for the same bars and refuses
//! the file by name for different ones (the repair below refuses them too:
//! it never changes a held bar); a day built under an old definition is
//! never rewritten in place and never passes as a clean result.
//!
//! # A day its month lacks, where no append can land it: a new generation (D-3350; DB-N26)
//!
//! A day the month lacks between, before or among the days it holds — the
//! 8 Muhurat sessions, the 2 DR drills, 2021-02-24 and 2022-03-07, refused by
//! the calendar when their months were first imported and admitted since
//! D-3319 (WP-12a) — cannot be appended: the append-only month refuses a
//! batch inside its held range at a stamp it never held, and until WP-12d
//! that refusal was the day's end (10 Oct 2026: every NIFTY and BANKNIFTY
//! file of the 12 days, which stayed `incomplete`). Every file is filed
//! through the append door first (`ingest::from_rows`, on whichever worker
//! takes it, WP-16), so a tail, a new month and a day already held land
//! exactly as before. A file that door refuses outright is filed again
//! through `ingest::repair_rows`, WP-13c's one repair authority reached from
//! this door, by the coordinating thread, in listing order and outside the
//! day's batch (`Door`, below): the append door once more, and when it
//! refuses, the month as every
//! reader opens it is asked one thing: does it hold any bar inside the
//! file's span, first second to last? Only a month that holds NONE, where no
//! append can ever land the file (the span starts at or before the month's
//! last held bar, or the month is served from a revision), is REPAIRED — a
//! new generation holding every bar the month served plus the day's,
//! published under `bar-revisions-v1/<n>` and promoted so every reader
//! (`BarFile::open_existing`, the census, `.tix` lookups, `/provenance.json`)
//! opens it. Never in place: the original's files stay byte-identical.
//!
//! Every other refusal of the append door stands, in its own words, and
//! nothing is repaired: a month another process holds locked, a host error,
//! and above all a day the month already holds read differently — other
//! values at a held second, or more seconds than it holds. A back-fill adds
//! a day the month lacks; it never completes, restates or mixes a held one,
//! so no month is ever left holding what no single reading of a day
//! produces, and a passing lock never becomes a generation.
//!
//! Journaled before and after, in `<store>/audit/gdfl.repairs`, each line
//! naming this run's id, whose `run` line names the archive: `repairing`
//! before anything is published (the month, the generation served, the bars
//! offered and their span), then `repair` (the generations, bars kept and
//! added, the new span) or `refused` (why). A run killed between the
//! promotion and its `repair` line leaves the `repairing` line, so no
//! generation is served that the journal does not name. Resumable: a crash
//! between publication and promotion is promoted by the next run, a revision
//! torn mid-write is stepped over and named, never reused, and a day re-read
//! after its repair finds every bar held and writes nothing
//! ([`Report::months_repaired`] counts the repairs; such a landing writes no
//! `.prv` record, below).
//!
//! Once a month is served from a revision, every later day it lacks is a
//! repair too — a revision is never appended to — so each costs one more
//! generation and O(month), bounded by `store::repair::MAX_REVISIONS` per
//! month; `docs/06-limits.md` is WP-99's to state it. The 12 index days'
//! months were complete, so each instrument-month of theirs is one
//! generation. A month that holds only a later day (the six alias indices'
//! special-session months, DB-B40) takes every other day of the month this
//! way, one generation a day.
//!
//! # Refusals
//!
//! A run-level refusal (a bad range, an unreadable journal) is an `Err`. A
//! day-level one (calendar, venue hours, a listing the source refuses) and a
//! file-level one (any reader refusal, a volume past `i64`) is one named
//! [`Failure`] in the report and in the log, and the run continues, but for
//! a census refusal, which stops it (below). A closed day is counted, a day
//! the source does not hold is counted and named.
//!
//! # A census refusal stops the run (D-3317 item 7; DB-G01, DB-G10, DB-058)
//!
//! The census is published once per day, after the day's bar files are
//! written, all or nothing ([`record_held`]). Until D-3317 a refusal was one
//! more failure of the day and the run went on. At the census's entry ceiling
//! every later day then failed the same way while its bars still landed: the
//! options run of 6 Oct 2026 closed 210 days `incomplete`, every options day
//! from 2019-11-14 to 2020-09-15 but 2019-12-27 (the store's journal, read 7
//! and 9 Oct 2026, after the operator stopped the run mid 2020-09-16), each
//! logged at `info` as "day imported" and counted in `days_imported`, under a
//! refusal that quoted the census's in-memory counter rather than the file's.
//!
//! Now the first census refusal ends the run. At the ceiling it is
//! [`ImportRefusal::CensusFull`], naming the census FILE's committed entries
//! (`n_valid`), its limit, the free slots, the day's batch and how many of
//! the batch are new to the census ([`CensusRoom`]), and that nothing of the
//! batch was published. Any other census refusal (a census that cannot be
//! read, installed or locked) is [`ImportRefusal::CensusRefused`] and stops
//! the run just the same: it leaves the day uncounted too, and continuing is
//! what let the drift grow without a bound. Either way the day closes
//! `incomplete ... stage=census reason=too_many_entries` (or
//! `reason=census_refused`), it is not counted in `days_imported`, it is
//! logged once at `error` ("census refused; run stopped") and never as
//! imported, and no later day begins. So the bars on disk run ahead of the
//! census by at most one day per run, and the journal names that day.
//!
//! # The journal's lines (D-3317, the field names it fixes)
//!
//! One line per event, fields split by one space, every field after the key
//! `name=value`; a free-text value (the archive's path) is percent-encoded,
//! every byte outside `A-Z a-z 0-9 - . _ ~ /` written `%XX`. `<key>` is
//! `<kind> <day> <filter>`.
//!
//! - `run <run> source=<zip|tickstore> archive=<path> archive_bytes=<n>
//!   commit=<sha> started=<unix_s>` and `archive_cd_blake3=<hex>` (a zip
//!   source) or `format_sha256=<hex>` (a tick store): the run's provenance
//!   ([`Origin`]; DB-G56). `<run>` is `started`, raised by one while an
//!   earlier `run` line holds it. Written before the run's first other line,
//!   so a run that skips every day writes nothing (D-2803).
//! - `begin <key>`: unchanged, before a day's first write.
//! - `done <key> <stats> run=<run>`: no failure.
//! - `done <key> <stats> vendor_refused=<n> reason=<code> run=<run>`: DONE
//!   WITH REFUSALS. A file of the day read clean, and every failure is a
//!   refusal of the vendor's own bytes by a named reader rule (2018-11-06's
//!   options price at line 13668; 2018-11-09's row at line 451, DB-G09), which
//!   the same member refuses the same way every time under the same reader.
//!   It is done: a
//!   re-run skips it, unless it is given `retry_refused`
//!   ([`RunOptions::retry_refused`]), which is how a day is read again after a
//!   reader learns to accept what it refused.
//! - `incomplete <key> <stats> stage=<stage> reason=<code> run=<run>`: a
//!   file of the day read clean and a failure is not the vendor's own bytes,
//!   or the census refused. Imported again by the next run.
//! - `refused <key> definition=<n> stage=<stage> reason=<code> failures=<n>
//!   run=<run>`: no file of the day read clean, so it wrote nothing — every
//!   wanted file refused (2022-03-07's index files), the listing refused, the
//!   calendar declined it (a special session), or the source does not hold it
//!   (`day_not_held`). Until D-3317 such a day left no line, and the journal
//!   went from `done indices 2022-03-04` straight to `begin indices
//!   2022-03-08` (DB-025). Imported again by the next run and named in
//!   `resumed`; refused again the same way (the same line but for `run=`),
//!   it is not written again, so a re-run over a hole that does not change
//!   writes nothing.
//! - `recounted <key> definition=<n> months=<n> run=<run>`: written by
//!   `cli census-recount` (D-3316, WP-10), never here; it closes the day.
//!
//! A `<key>` is `<kind> <day> <filter>`, or `<kind> <day> <filter>+<member>`
//! for one member of a day read again for a wider scope (D-4878,
//! [`scope`]). Each close tells one summary event ([`note_day`]): a day's is
//! "day imported", "day incomplete" or "day refused", once a day; a member
//! key's is "member imported", "member incomplete" or "member refused" and
//! names its member and its key.
//!
//! `<stats>` are the D-2822 fields in their order. `<stage>` is one of
//! [`Stage`]'s words and `<code>` the snake-case name of the refusal variant
//! that refused ([`Cause`]), of the day's decisive failure: the census's,
//! else the first that keeps the day from closing, else the first. A loader
//! built before D-3317 refuses the new verbs by name, loudly, rather than
//! misreads them.
//!
//! # What each day also records (WP-11b)
//!
//! - **The day ledger** (D-3335; [`quality`]): before its close, every
//!   admitted day the source held appends its constant-size quality lines to
//!   `<store>/imports/gdfl.dayflags`: per index, or per kind-day of stocks or
//!   options (only for an unfiltered run, since `*` is the whole kind-day):
//!   the day zip's size, late and deferred rows, refused rows, zero-quote
//!   files, leading LTQ = 1 marker rows, LTQ = 0 rows carrying a new price,
//!   index seconds and holes, shared share holes, same-second dual values,
//!   closing-auction prints. Bars are never altered by any of it. A ledger
//!   that will not take the lines leaves the day `incomplete`
//!   (`stage=journal reason=day_ledger`), so the next run writes them; a
//!   ledger that cannot be read refuses the run before any day.
//!   [`rescan_cm`] and [`rescan_nfo`] (`cli gdfl-dayflags`) write the same
//!   lines, `src=rescan`, for days already imported, read-only.
//! - **The provenance** (D-3334): when the run names its [`Origin`], each
//!   file that committed bars appends one record to its month's `.prv`
//!   sidecar (`store::provenance`): the archive member
//!   (`<archive>!<day zip>!<entry>`, its size and CRC-32) or the tick-store
//!   day file, the day, the bars appended, the bar definition, the journal's
//!   `begin` line and the build commit. The bar file is opened read-only and
//!   the record refused unless its commit counter is still the one the
//!   append left. A refused record is a named failure of the file. A file
//!   that repaired its month (above) writes none, and says so in a `warn`
//!   event: a record covers the tail one append committed, and a revision
//!   carries no `.prv` (`crates/store/REPAIR.md`), so `/provenance.json`
//!   answers that month from this journal's day lines, reconstructed.
//! - **The lineage** (D-3331, D-3343; [`lineage`]): an options run feeds
//!   every day's contract-days to `gdfl_lineage::detect` and appends the
//!   `alias`, `lineage` and `stitched` records to `<store>/imports/gdfl.lineage`
//!   and the inferred lots to `<store>/imports/gdfl.lots`. A renamed strike
//!   is flagged, never split: the store key has no form for two contracts.
//!
//! # Cost
//!
//! Per day: one listing (O(entries)), then per instrument file one fetch,
//! check and decode (O(bytes)), one placement (two passes, O(rows)), two
//! folds (O(rows)), and two `from_rows` appends (O(bars + log n_valid +
//! blocks touched), `docs/06-limits.md`), each through `repair_rows`, which
//! for a month the append door refuses reads, merges, writes and reads back
//! the whole month instead (O(held + offered), at most
//! `store::repair::MAX_ROWS`), once per repaired month: measured on 24 real
//! NIFTY and BANKNIFTY one-second months of 395,000 to 473,000 bars in a
//! scratch store, p50 142 ms and max 323 ms a month, the file's read and
//! fold included (WP-12d). The census is published once per
//! day ([`crate::ingest::record_held`], O(entries held)). None of this is a
//! constant-time claim; `docs/06-limits.md` states it. What IS constant per
//! lookup is the stored result: a bar by index is one positional read
//! (`store::file::BarFile::read_record`, C-28/C-29), a ticker in a listing is
//! one slot index or one hash probe (C-GI-01/02).
//!
//! WP-11b adds, per file, one more pass over its rows and its bars for the
//! day ledger (O(rows + bars)) and, when the run names its origin, one
//! read-only open of the month's bar file, as a reader opens it, and one
//! sidecar append with one `fdatasync` (measured by WP-38a at p50
//! 1.15 ms, p99 1.90 ms on WD_BLACK); per day one append and one `fsync` per
//! ledger, and for options the lineage's O(contracts) (its module says
//! more); per run one read of each ledger, O(its lines).
//!
//! # Workers, and when a day is durable (WP-16, DB-N25)
//!
//! Days run in order. The files of a day run on [`Workers`] (8 by default,
//! 1 to 12): each wanted file is one job, read, decoded, folded and filed
//! by whichever worker takes it, and every month file and sidecar it writes
//! is that file's alone. What the jobs did is folded into the day in listing
//! order on the coordinating thread, which alone writes journal lines, so
//! the store, the journal, the census, the day ledger and the lineage are
//! the bytes one worker writes (`gdfl_import_tests::
//! w_workers_build_the_same_store_journal_and_census_as_one`). The lineage
//! warm-up at a run's start reads its files on the same workers.
//!
//! A BACK-FILL IS NOT A WORKER'S, AND NOT THE BATCH'S (D-3350 beside D-3353).
//! A worker files through the append door alone. A file that door refuses
//! outright is filed again through the back-fill door when the day's workers
//! are done: by the coordinating thread, one file after the other in listing
//! order, with the day's batch suspended (`store::file::DurableBatch::suspend`).
//! The versioned repair flushes a revision's bars before the receipt that
//! publishes them and before their promotion, and inside a batch those
//! flushes would be held back past both; and its journal,
//! `<store>/audit/gdfl.repairs`, is one ledger whose lines stay in the order
//! one worker writes them (`gdfl_import_tests::
//! a_back_fill_is_published_outside_the_days_batch_and_leaves_one_workers_store`).
//! A refused file is therefore offered to the append door twice, and its
//! rows stay in memory until its day's workers are done: at most one day's
//! rows, when every file of the day is refused.
//!
//! A DAY IS DURABLE ONCE, TOGETHER (D-3353). By default every write of a
//! day's files (records, checksums, time index, header slot, provenance
//! record, new names) is left to one `store::file::DurableBatch`: the writes
//! are the same writes in the same order, each file opened `O_SYNC`, and no
//! flush is issued per append. When the day's last job is folded the batch is
//! made durable (one `F_FULLFSYNC` per directory that gained a name and one
//! per device); only then are the day's ledger lines written, its census
//! published as ONE batch of three flushes (it was two flushes per entry;
//! the entries, then each header slot on its own, the older slot first, so a
//! torn slot write never leaves the census behind where it was:
//! `census_store::write_batch`) and its closing line appended. So a day's
//! closing line is never written over writes that are not durable, and a day
//! that cannot be made durable stops the run with the day left open
//! ([`ImportRefusal::Durability`]).
//!
//! A KILL AT ANY POINT leaves each month at its last commit or its new one
//! (or, cut off inside its creation, holding no commit at all, which the
//! next open creates again, D-1521), and the day, begun and not closed, is
//! read again by the next run. That run finds the months the killed one
//! wrote already present and writes nothing to them, so it names each to
//! its own batch (`DurableBatch::confirm_month`), which flushes that month's
//! files and directories for themselves before the day is closed
//! (`gdfl_import_tests::a_kill_at_any_point_leaves_no_half_month_and_the_
//! resume_lands_the_same_store`). A POWER CUT inside an open day is weaker
//! than a kill and is said, not implied away: no barrier orders a month's
//! records before its header slot on the drive until the day's flush, so a
//! month the open day appended to can be left refused by its own checks
//! until it is repaired (`store::file`'s header; named for
//! `docs/06-limits.md`). [`Durability::EachWrite`] keeps the flush per
//! write for a caller that cannot accept that.
//!
//! Measured on the operator's drive, into scratch stores, 11 Oct 2026
//! (machine load 6 to 12): options 2020-09-15 alone, 5,641 files into an
//! empty store, 144.7 s wall and 41.8 s of CPU before, 10.8 s wall and
//! 52.9 s of CPU on 8 workers after, the bars, checksums, indexes, census,
//! day ledger and lineage byte for byte the same; five options days
//! 2020-09-14..18 in one run, 29,823 files, 529.5 s before (10 Oct) and
//! 16.7 s after, a day after the first 2.1 to 3.0 s; indices 2020-09-15,
//! 7 files, 0.27 s before and 0.06 s after.
//!
//! D-3317 adds three costs, each said here. Every closing line is one
//! append of a bounded number of fields, so the ledger is O(1) per day.
//! Measuring an [`Origin`] at run start reads metadata only, plus each outer
//! zip's central directory (bounded by [`CENTRAL_DIRECTORY_CAP`]) or the tick
//! store's `FORMAT.md`; it never reads a member. Measuring a refusing census's
//! room ([`CensusRoom`]) loads the census once more, Θ(entries), on the
//! refusal path only, which happens at most once per run because the run stops
//! there. Whether a `refused` close repeats the key's last line is one hash
//! probe into the lines the journal load already holds.

use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use brutex_core::instrument::Contract;
use brutex_core::vendor::Vendor;
use store::format::{Bar, OI_NULL};

use crate::fetch::BarRequest;
use crate::fold::{Bucket, fold};
use crate::gdfl_cm::{CmKind, CmRefusal, CmSource, read_listed, session_gate, split_name};
use crate::gdfl_nfo::{NfoRefusal, NfoSource, decode_ticker, read_file};
use crate::ingest::{
    Failure, Filed as Landed, Ingested, Plan, from_rows, record_held, repair_rows,
};
use crate::manifest::{ENTRY_STRIDE, HEADER_LEN, Held, MAX_ENTRIES, Manifest, manifest_path};
use crate::session::{Day, IST_OFFSET_SECS, Window};
use crate::vendor::{Granularity, Listing, PriceScale, TimestampEncoding};

/// The telemetry target every event of this module carries.
pub const TARGET: &str = "gdfl.import";

/// Which tree a run imports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImportKind {
    /// NIFTY 50 and NIFTY BANK, filed as `NSE/INDEX/NIFTY` and `.../BANKNIFTY`.
    Indices,
    /// The F&O shares' cash equities, filed as `NSE/CASH/<SYM>`.
    Stocks,
    /// Option contracts, filed as `NSE/FNO/<UNDERLYING>/<contract>`.
    Options,
}

impl ImportKind {
    /// Every kind, in import order.
    pub const ALL: [Self; 3] = [Self::Indices, Self::Stocks, Self::Options];

    /// The word for this kind, as the `cli` verb takes it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Indices => "indices",
            Self::Stocks => "stocks",
            Self::Options => "options",
        }
    }

    /// The kind `word` names.
    ///
    /// # Errors
    ///
    /// [`ImportRefusal::KindUnknown`] for any other word.
    pub fn parse(word: &str) -> Result<Self, ImportRefusal> {
        Self::ALL
            .into_iter()
            .find(|kind| kind.as_str() == word)
            .ok_or_else(|| ImportRefusal::KindUnknown {
                word: word.to_owned(),
            })
    }

    /// The store segment this kind is filed under.
    #[must_use]
    pub const fn segment(self) -> &'static str {
        match self {
            Self::Indices => "INDEX",
            Self::Stocks => "CASH",
            Self::Options => "FNO",
        }
    }

    /// The listing class, which decides the venue's hours.
    #[must_use]
    pub const fn listing(self) -> Listing {
        match self {
            Self::Indices => Listing::Index,
            Self::Stocks => Listing::Equity,
            Self::Options => Listing::Derivative,
        }
    }

    /// Whether every row is a price (an index level) or only a traded row is.
    #[must_use]
    pub const fn every_row_counts(self) -> bool {
        matches!(self, Self::Indices)
    }
}

/// Why a run, a day or a file was refused. Every refusal names itself.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ImportRefusal {
    /// The kind word is not `indices`, `stocks` or `options`.
    KindUnknown {
        /// The word.
        word: String,
    },
    /// The first day is after the last.
    RangeBackwards {
        /// The first day asked.
        from: Day,
        /// The last day asked.
        to: Day,
    },
    /// A traded quantity, or a second's sum of them, past `i64::MAX`.
    VolumeOverflow {
        /// The second of the day it happened at.
        sod: u32,
    },
    /// The fold refused, in its own words.
    Fold {
        /// Why.
        why: String,
    },
    /// The journal could not be read or written, or is not a journal.
    Journal {
        /// Why.
        why: String,
    },
    /// A filter name that is not a symbol as the store files it: the journal
    /// keys a day by the names, joined by `,` and split on ` `, and `*` is the
    /// whole tree, so a name with a space, a comma, a `*`, a lowercase letter
    /// or nothing would match no file and still record the day `done` under a
    /// key another filter shares (D-2811).
    FilterName {
        /// The name as given.
        name: String,
    },
    /// A row stamped at or past second 86,400 of its day (D-2825): filed,
    /// it would land on a later day, inside that day's session.
    StampPastTheDay {
        /// The stamp, in seconds from the day's midnight.
        sod: u32,
    },
    /// The census is full: the day's batch does not fit what its file has
    /// left, so nothing of it was published, and the run stopped at that day
    /// (D-3317 item 7; DB-G01, DB-G10, DB-058). The day's bar files are on
    /// disk and uncounted; it is journalled `incomplete ... stage=census
    /// reason=too_many_entries`, so a recount or a resumed run finds it, and
    /// every day after it was left unread.
    CensusFull {
        /// The kind the run imports.
        kind: ImportKind,
        /// The day whose census batch was refused.
        day: Day,
        /// The census file's room for the batch, measured after the refusal.
        room: CensusRoom,
        /// The census's own words. Since D-3316 item 5 the census refuses a
        /// batch up front, before recording any of it, naming its file's
        /// `n_valid`, capacity, free slots and the batch (WP-10); `room` is
        /// this module's own measurement of the same file, kept beside them.
        why: String,
        /// What the run did before it stopped. The refused day's files and
        /// bars are counted in it, because they are on disk; the day is not
        /// in `days_imported`, because the census does not count it.
        done: Box<Report>,
    },
    /// The census refused the day's batch for a reason other than room, or
    /// its room could not be measured; nothing of the batch was published and
    /// the run stopped at that day, as for [`ImportRefusal::CensusFull`]
    /// (D-3317; DB-G01). Journalled `incomplete ... stage=census
    /// reason=census_refused`.
    CensusRefused {
        /// The kind the run imports.
        kind: ImportKind,
        /// The day whose census batch was refused.
        day: Day,
        /// The census's room, measured from its file after the refusal, or
        /// why it could not be measured.
        room: Result<CensusRoom, String>,
        /// The census's own words.
        why: String,
        /// What the run did before it stopped, as for `CensusFull`.
        done: Box<Report>,
    },
    /// The ingest door refused to file a converted file's bars (a store write,
    /// a month the store will not take), in its own words. A failure of the
    /// file, never of the run (D-3317's code `filing`).
    Filing {
        /// Why, in the door's words.
        why: String,
    },
    /// The source holds no file of a trading day the calendar owes (D-3317's
    /// code `day_not_held`). Counted in [`Report::days_missing`] and closed
    /// `refused`; never a failure of the run.
    DayNotHeld {
        /// The day.
        day: Day,
    },
    /// A landing's provenance record (D-3334) was refused by the sidecar or
    /// could not be built: a failure of the file, never of the run. The bars
    /// stay committed, and unrecorded.
    Provenance {
        /// Why, in the store's words.
        why: String,
    },
    /// The GDFL day ledger (D-3335) could not be read at the start of a run,
    /// which then stops before any day, or would not take a day's lines, a
    /// failure that leaves the day `incomplete` so the next run writes them.
    DayLedger {
        /// Why.
        why: String,
    },
    /// The lineage or lot ledger (D-3331, D-3343) could not be read at the
    /// start of an options run, which then stops before any day, or would not
    /// take a day's records, or a renamed contract's file could not be read
    /// again: the day is left `incomplete` and its records are written by the
    /// next run.
    Lineage {
        /// Why.
        why: String,
    },
    /// A day's writes could not be made durable (WP-16, D-3353): the host
    /// refused a flush of the day's batch. The run stopped at that day, which
    /// is left begun and not closed, so the next run reads it again; every
    /// month the day wrote takes no further append in this process.
    Durability {
        /// The kind the run imports.
        kind: ImportKind,
        /// The day.
        day: Day,
        /// The store's words, and what was not written.
        why: String,
        /// What the run did before it stopped. The day's files are counted
        /// in it; the day is not in `days_imported`.
        done: Box<Report>,
    },
    /// An options filter names a symbol the lineage files under another key
    /// (D-3354): the decoder files `NIITTECH...` under `COFORGE`, so the
    /// name as given would match no file, and the day would be journaled
    /// `done` with nothing read.
    FilterRenamed {
        /// The name as given.
        name: String,
        /// The key its option files are filed under.
        filed: String,
    },
    /// The store holds live option series under a symbol the lineage files
    /// elsewhere (D-3354): the run would open a second key for contracts
    /// the store already holds under the old one, on a day the exchange
    /// changed nothing. Refused before any day is read. A directory whose
    /// every contract the ledger records superseded or retired is history
    /// and is not named.
    UnderlyingSplit {
        /// Each old symbol with a directory, and the key it is filed under.
        held: Vec<(String, String)>,
    },
}

impl ImportRefusal {
    /// The refusal's code, as a journal `reason=` writes it (D-3317 item 6):
    /// the snake-case name of the variant, but for a full census, whose code
    /// is the census's own refusal, `too_many_entries`
    /// (`ManifestError::TooManyEntries`, D-3317 item 7). A code is never
    /// renamed or reused.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::KindUnknown { .. } => "kind_unknown",
            Self::RangeBackwards { .. } => "range_backwards",
            Self::VolumeOverflow { .. } => "volume_overflow",
            Self::Fold { .. } => "fold",
            Self::Journal { .. } => "journal",
            Self::FilterName { .. } => "filter_name",
            Self::StampPastTheDay { .. } => "stamp_past_the_day",
            Self::CensusFull { .. } => "too_many_entries",
            Self::CensusRefused { .. } => "census_refused",
            Self::Filing { .. } => "filing",
            Self::DayNotHeld { .. } => "day_not_held",
            Self::Provenance { .. } => "provenance",
            Self::DayLedger { .. } => "day_ledger",
            Self::Lineage { .. } => "lineage",
            Self::Durability { .. } => "durability",
            Self::FilterRenamed { .. } => "filter_renamed",
            Self::UnderlyingSplit { .. } => "underlying_split",
        }
    }

    /// What the run did before a census refusal stopped it, or `None` for a
    /// refusal that stopped it before any day.
    #[must_use]
    pub fn report(&self) -> Option<&Report> {
        match self {
            Self::CensusFull { done, .. }
            | Self::CensusRefused { done, .. }
            | Self::Durability { done, .. } => Some(done),
            _ => None,
        }
    }
}

/// [`ImportRefusal::UnderlyingSplit`]'s sentence: every old-symbol directory
/// with the key its files are filed under, then the remedy.
fn split_said(f: &mut core::fmt::Formatter<'_>, held: &[(String, String)]) -> core::fmt::Result {
    write!(
        f,
        "the store holds option series under {} old symbol(s) the import now files elsewhere \
         (D-3354):",
        held.len()
    )?;
    for (old, filed) in held {
        write!(f, " {old} (filed under {filed});")?;
    }
    write!(
        f,
        " importing would open a second key for contracts the store already holds. Supersede \
         each bars/gdfl/NSE/FNO/<old symbol> first (its months re-imported under the filed key \
         as new generations and the old keys recorded superseded), then run again. Nothing was \
         read or written"
    )
}

impl core::fmt::Display for ImportRefusal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::KindUnknown { word } => write!(
                f,
                "{word:?} is not a GDFL import kind; the kinds are indices, stocks and options"
            ),
            Self::RangeBackwards { from, to } => write!(f, "the range {from}..{to} runs backwards"),
            Self::VolumeOverflow { sod } => write!(
                f,
                "the traded quantity at second {sod} of the day does not fit an i64; refused, never saturated"
            ),
            Self::Fold { why } => write!(f, "the fold refused: {why}"),
            Self::Journal { why } => write!(f, "the import journal: {why}"),
            Self::FilterName { name } => write!(
                f,
                "{name:?} is not a symbol as the store files it (A-Z, 0-9, -, _, &, at most 24); \
                 refused before the journal keys a day by it"
            ),
            Self::StampPastTheDay { sod } => write!(
                f,
                "a row stamped at second {sod} is not a second of the day (0 to 86,399); \
                 refused, never filed on a later day"
            ),
            Self::CensusFull {
                kind,
                day,
                room,
                why,
                done,
            } => write!(
                f,
                "the census is full and cannot take {} {day}: its file holds n_valid={} of the \
                 limit={} this build writes, free={}, and the day's batch={} entries (new={}) \
                 does not fit (the census's own words: {why}); {}",
                kind.as_str(),
                room.n_valid,
                room.limit,
                room.free(),
                room.batch,
                room.new,
                Stopped(done.days_imported),
            ),
            Self::CensusRefused {
                kind,
                day,
                room,
                why,
                done,
            } => {
                let kind = kind.as_str();
                match room {
                    Ok(room) => write!(
                        f,
                        "the census refused {kind} {day} although the day's batch fits \
                         (n_valid={} limit={} free={} batch={} new={}): {why}",
                        room.n_valid,
                        room.limit,
                        room.free(),
                        room.batch,
                        room.new,
                    )?,
                    Err(unread) => write!(
                        f,
                        "the census refused {kind} {day} and its room could not be measured \
                         ({unread}): {why}"
                    )?,
                }
                write!(f, "; {}", Stopped(done.days_imported))
            }
            Self::Filing { why } => write!(f, "the store refused to file the bars: {why}"),
            Self::DayNotHeld { day } => {
                write!(f, "the source holds no file of {day}, a trading day")
            }
            Self::Provenance { why } => write!(
                f,
                "the bars were filed and their provenance record was refused, so they stay \
                 unrecorded: {why}"
            ),
            Self::DayLedger { why } => write!(f, "the GDFL day ledger: {why}"),
            Self::Lineage { why } => write!(f, "the lineage ledger: {why}"),
            Self::Durability {
                kind,
                day,
                why,
                done,
            } => write!(
                f,
                "{} {day}: {why}; the run stopped there after {} day(s) imported",
                kind.as_str(),
                done.days_imported
            ),
            Self::FilterRenamed { name, filed } => write!(
                f,
                "{name} is filed under {filed} (D-3354: an option is filed under the newest \
                 symbol its name's dated changes reach); ask for {filed}, which reads the \
                 {name} files of the days before the change too. Refused: the name as given \
                 matches no file"
            ),
            Self::UnderlyingSplit { held } => split_said(f, held),
        }
    }
}

/// The tail every census refusal ends in: what was and was not published,
/// and where the run stopped.
struct Stopped(usize);

impl core::fmt::Display for Stopped {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "nothing of the day's batch was published, the day's bars are on disk and uncounted \
             (journal: incomplete, stage=census), and the run stopped there after {} day(s) \
             imported",
            self.0
        )
    }
}

impl core::error::Error for ImportRefusal {}

/// The stage a failure happened at, as a journal `stage=` writes it (D-3317
/// item 6). Ordered by how decisive it is for a day's close: a census failure
/// decides it before any other.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Stage {
    /// The census refused to publish the day's batch: its bars are on disk
    /// and uncounted. Always stops the run.
    Census,
    /// The journal would not take a line.
    Journal,
    /// The store refused to file a file's bars, or this import refused a file
    /// it had read: the fold, a quantity past `i64`, an underlying or a
    /// contract the store cannot file.
    File,
    /// The calendar declined the day or a file of it: a special session, a
    /// day it has not measured, an expiry it derives.
    Calendar,
    /// The vendor's listing, member or row: the vendor's own bytes refused by
    /// a named reader rule, or the copy of them (a CRC or length that does
    /// not match, a container that is not one, a read that failed).
    Vendor,
}

impl Stage {
    /// Every stage, in order.
    pub const ALL: [Self; 5] = [
        Self::Census,
        Self::Journal,
        Self::File,
        Self::Calendar,
        Self::Vendor,
    ];

    /// The word a journal `stage=` writes.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Census => "census",
            Self::Journal => "journal",
            Self::File => "file",
            Self::Calendar => "calendar",
            Self::Vendor => "vendor",
        }
    }
}

/// Why one failure happened, as the journal names it (D-3317): its stage, its
/// code, and whether it can close a day as done with refusals.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Cause {
    /// The stage.
    pub stage: Stage,
    /// The snake-case name of the refusal variant that refused.
    pub code: &'static str,
    /// Whether the failure is a refusal of the vendor's own bytes by a named
    /// reader rule, which the same member refuses the same way every time
    /// under the same reader: a day that wrote and whose every failure is one
    /// closes done with refusals (D-3317 item 2). A refusal of the copy (a CRC
    /// or a length that does not match, a member that does not inflate, a
    /// container or a tick-store file that is not one), an I/O fault, the
    /// calendar, the store, the journal and the census never close a day:
    /// another copy, another calendar or another try can change them.
    pub closes: bool,
}

impl Cause {
    /// A refusal of the vendor's own bytes by a named reader rule.
    #[must_use]
    pub const fn vendor(code: &'static str) -> Self {
        Self {
            stage: Stage::Vendor,
            code,
            closes: true,
        }
    }

    /// A failure at `stage` that does not close a day.
    #[must_use]
    pub const fn open(stage: Stage, code: &'static str) -> Self {
        Self {
            stage,
            code,
            closes: false,
        }
    }
}

/// The cause of a capital-market refusal (D-3317). Exhaustive on purpose: a
/// new refusal is a compile error here until someone names its code and
/// decides whether rereading the day could change it.
#[must_use]
pub const fn cm_cause(why: &CmRefusal) -> Cause {
    match why {
        CmRefusal::FolderUnknown => Cause::vendor("folder_unknown"),
        CmRefusal::ExtensionUnknown => Cause::vendor("extension_unknown"),
        CmRefusal::AmbiguousCaseTwins => Cause::vendor("ambiguous_case_twins"),
        CmRefusal::HeaderUnknown => Cause::vendor("header_unknown"),
        CmRefusal::TickerMismatch { .. } => Cause::vendor("ticker_mismatch"),
        CmRefusal::DateMismatch { .. } => Cause::vendor("date_mismatch"),
        CmRefusal::MalformedRow { .. } => Cause::vendor("malformed_row"),
        CmRefusal::PriceRefused { .. } => Cause::vendor("price_refused"),
        CmRefusal::RowsOverCap { .. } => Cause::vendor("rows_over_cap"),
        CmRefusal::NoSessionRows => Cause::vendor("no_session_rows"),
        CmRefusal::IndexEndsInSession { .. } => Cause::vendor("index_ends_in_session"),
        CmRefusal::ArchiveDuplicateDay { .. } => Cause::vendor("archive_duplicate_day"),
        CmRefusal::ArchiveMemberCompressed { .. } => Cause::vendor("archive_member_compressed"),
        CmRefusal::ArchiveMethodUnknown { .. } => Cause::vendor("archive_method_unknown"),
        CmRefusal::CalendarNotRegular { .. } => {
            Cause::open(Stage::Calendar, "calendar_not_regular")
        }
        CmRefusal::CalendarUnmeasured => Cause::open(Stage::Calendar, "calendar_unmeasured"),
        CmRefusal::NotSwept => Cause::open(Stage::File, "not_swept"),
        // The copy, which an operator can replace, or a read that can succeed.
        CmRefusal::ArchiveUnavailable { .. } => Cause::open(Stage::Vendor, "archive_unavailable"),
        CmRefusal::ArchiveMalformed { .. } => Cause::open(Stage::Vendor, "archive_malformed"),
        CmRefusal::SourceLengthMismatch { .. } => {
            Cause::open(Stage::Vendor, "source_length_mismatch")
        }
        CmRefusal::SourceCrcMismatch { .. } => Cause::open(Stage::Vendor, "source_crc_mismatch"),
        CmRefusal::ArchiveMemberCorrupt { .. } => {
            Cause::open(Stage::Vendor, "archive_member_corrupt")
        }
        CmRefusal::TickStoreMalformed { .. } => Cause::open(Stage::Vendor, "tick_store_malformed"),
        CmRefusal::TickStoreUnavailable { .. } => {
            Cause::open(Stage::Vendor, "tick_store_unavailable")
        }
        // D-3318, D-3333: the vendor's own bytes, refused the same way by
        // the same reader every time.
        CmRefusal::TooManyBadRows { .. } => Cause::vendor("too_many_bad_rows"),
        CmRefusal::SymbolNotInUse { .. } => Cause::vendor("symbol_not_in_use"),
        // D-3319, D-3320: a calendar, a security master or a sidecar another
        // build or another day can change.
        CmRefusal::CalendarWindowNotLanded { .. } => {
            Cause::open(Stage::Calendar, "calendar_window_not_landed")
        }
        CmRefusal::AuctionEligibilityUnknown { .. } => {
            Cause::open(Stage::Calendar, "auction_eligibility_unknown")
        }
        CmRefusal::AuctionSidecar { .. } => Cause::open(Stage::File, "auction_sidecar"),
    }
}

/// The cause of an options refusal (D-3317), as [`cm_cause`].
#[must_use]
pub const fn nfo_cause(why: &NfoRefusal) -> Cause {
    match why {
        NfoRefusal::TickerUnparsed { .. } => Cause::vendor("ticker_unparsed"),
        NfoRefusal::TickerAmbiguous { .. } => Cause::vendor("ticker_ambiguous"),
        NfoRefusal::ContractNamedTwice { .. } => Cause::vendor("contract_named_twice"),
        NfoRefusal::DuplicateTicker { .. } => Cause::vendor("duplicate_ticker"),
        NfoRefusal::HeaderUnknown => Cause::vendor("header_unknown"),
        NfoRefusal::TickerMismatch { .. } => Cause::vendor("ticker_mismatch"),
        NfoRefusal::DateMismatch { .. } => Cause::vendor("date_mismatch"),
        NfoRefusal::MalformedRow { .. } => Cause::vendor("malformed_row"),
        NfoRefusal::PriceRefused { .. } => Cause::vendor("price_refused"),
        NfoRefusal::OpenInterestRefused { .. } => Cause::vendor("open_interest_refused"),
        NfoRefusal::RowsOverCap { .. } => Cause::vendor("rows_over_cap"),
        // A strike past the day's plausible multiple of its underlying's
        // median (D-3331): the vendor's name, judged by a named rule over the
        // same day's listing, so the same member is refused the same way.
        NfoRefusal::StrikeImplausible { .. } => Cause::vendor("strike_implausible"),
        // The expiry is DERIVED from the calendar, so a calendar that learns
        // the day, or a rule that changes, can change the answer.
        NfoRefusal::MonthlyExpiryUnmeasured { .. } => {
            Cause::open(Stage::Calendar, "monthly_expiry_unmeasured")
        }
        NfoRefusal::ExpiryPassed { .. } => Cause::open(Stage::Calendar, "expiry_passed"),
        // Settled against the calendar (D-3331), which can learn the day.
        NfoRefusal::ExpiryUnsettled { .. } => Cause::open(Stage::Calendar, "expiry_unsettled"),
        NfoRefusal::UnderlyingRefused { .. } => Cause::open(Stage::File, "underlying_refused"),
        // Judged against the dated symbol-change table (D-3333), which can
        // learn a row (WP-12e).
        NfoRefusal::UnderlyingStale { .. } => Cause::open(Stage::File, "underlying_stale"),
        NfoRefusal::ContractUnrenderable { .. } => {
            Cause::open(Stage::File, "contract_unrenderable")
        }
        NfoRefusal::Source(inner) => cm_cause(inner),
    }
}

/// How a day closed, as the journal records it (D-3317).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Outcome {
    /// No failure: `done`.
    Done,
    /// The day wrote and every failure closes ([`Cause::closes`]): `done`
    /// with `vendor_refused=`.
    DoneWithRefusals,
    /// The day wrote and a failure does not close it, or the census refused:
    /// `incomplete`.
    Incomplete,
    /// The day wrote nothing and has a failure, or the source does not hold
    /// it: `refused`.
    Refused,
}

impl Outcome {
    /// The word a reader names this close by: `/dayflags.json`'s `outcome`
    /// (D-3335).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Done => "done",
            Self::DoneWithRefusals => "done_with_refusals",
            Self::Incomplete => "incomplete",
            Self::Refused => "refused",
        }
    }
}

/// A census's room for one day's batch, measured from the census FILE after it
/// refused the batch (D-3317; DB-G10). The census's own refusal quotes its
/// counter in memory part way through the batch; these are the file's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CensusRoom {
    /// Entries the census file has committed.
    pub n_valid: u64,
    /// The most entries this build lets one census hold
    /// ([`crate::manifest::MAX_ENTRIES`]).
    pub limit: u64,
    /// Entries the day offered.
    pub batch: u64,
    /// Entries of the batch the census does not already hold exactly as
    /// offered: at most the appends the batch needs.
    pub new: u64,
}

impl CensusRoom {
    /// Entries the census can still take.
    #[must_use]
    pub const fn free(&self) -> u64 {
        self.limit.saturating_sub(self.n_valid)
    }

    /// Whether the batch's new entries fit the free ones.
    #[must_use]
    pub const fn fits(&self) -> bool {
        self.new <= self.free()
    }
}

/// Which kind of source a run reads, as `source=` writes it (D-2804, D-3317).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceKind {
    /// The vendor's zips.
    Zip,
    /// The verified tick store.
    TickStore,
}

impl SourceKind {
    /// The word `cli gdfl-import` takes and the journal writes.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Zip => "zip",
            Self::TickStore => "tickstore",
        }
    }

    /// The name of the digest field a `run` line carries for this source
    /// (D-3317 item 4).
    #[must_use]
    pub const fn digest_field(self) -> &'static str {
        match self {
            Self::Zip => "archive_cd_blake3",
            Self::TickStore => "format_sha256",
        }
    }
}

/// The largest zip central directory [`Origin::measure`] reads, 64 MiB, as
/// the archive reader caps its own (`gdfl_archive`): the operator's
/// capital-market outer archive's is 493,028 bytes (charter, GDFL section).
pub const CENTRAL_DIRECTORY_CAP: u64 = 64 << 20;

/// The most directory entries [`Origin::measure`] visits under a tick store
/// before it stops and says so. A tick store holds one `.bts` per vendor day
/// zip, a few thousand; a root holding a million is not one.
pub const ORIGIN_WALK_CAP: usize = 1_000_000;

/// The largest `FORMAT.md` [`Origin::measure`] hashes, 1 MiB; the tick
/// store's specification is a few kilobytes.
pub const FORMAT_CAP: u64 = 1 << 20;

/// Where a run's data came from and what built it (D-3317 item 4; DB-G56),
/// written as the run's `run` line and in "run started".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Origin {
    /// The kind of source.
    pub source: SourceKind,
    /// The archive the source reads: the capital-market outer zip, the
    /// directory of yearly options zips, or the tick store's root.
    pub path: PathBuf,
    /// The archive's bytes: the outer zip's length, the yearly zips' summed,
    /// or every `.bts` file's under the tick store's root.
    pub archive_bytes: u64,
    /// For a zip source, the BLAKE3 of the outer zip's central directory,
    /// which records every member's size and CRC-32; for a directory of
    /// yearly zips, the BLAKE3 of each one's name, a zero byte, its central
    /// directory's length (eight bytes, little-endian) and that directory, in
    /// name order (this module's reading of D-3317 item 4, which names one
    /// outer zip). For a tick store, the SHA-256 of its `FORMAT.md`. `None`
    /// when it could not be measured; [`Origin::fault`] says why.
    pub digest: Option<[u8; 32]>,
    /// The commit the binary was built from, `None` when the build could not
    /// prove one (`cli::commit_stamp`).
    pub commit: Option<String>,
    /// When the run started, in UNIX seconds.
    pub started: u64,
    /// What could not be measured, named: a missing archive, an unreadable
    /// directory, a central directory that is not one. Logged at `warn` and
    /// written `unmeasured` in the `run` line; never a reason to read some
    /// other archive.
    pub fault: Option<String>,
}

impl Origin {
    /// Measures `path` for a run reading `source`, built at `commit`, starting
    /// now.
    ///
    /// Total: what cannot be measured is named in [`Origin::fault`], because
    /// the run's own source refuses a missing or unreadable archive by name a
    /// moment later and must not be pre-empted by a worse-worded twin. Cost:
    /// metadata reads, plus each outer zip's tail and central directory (at
    /// most [`CENTRAL_DIRECTORY_CAP`]) or one `FORMAT.md` (at most
    /// [`FORMAT_CAP`]); no member is read.
    #[must_use]
    pub fn measure(source: SourceKind, path: &Path, commit: Option<&str>) -> Self {
        let started = now_unix_s();
        let measured = match source {
            SourceKind::Zip => measure_zips(path),
            SourceKind::TickStore => measure_tick_store(path),
        };
        Self {
            source,
            path: path.to_path_buf(),
            archive_bytes: measured.bytes,
            digest: measured.digest,
            commit: commit.map(str::to_owned),
            started,
            fault: measured.fault,
        }
    }

    /// The digest as the `run` line writes it: 64 lower-case hex digits, or
    /// `unmeasured`.
    #[must_use]
    pub fn digest_hex(&self) -> String {
        self.digest.map_or_else(
            || "unmeasured".to_owned(),
            |digest| {
                digest
                    .iter()
                    .fold(String::with_capacity(64), |mut hex, byte| {
                        let _infallible = write!(hex, "{byte:02x}");
                        hex
                    })
            },
        )
    }

    /// The commit as the `run` line writes it, or `unstamped`.
    fn commit_text(&self) -> String {
        self.commit.as_deref().map_or_else(
            || "unstamped".to_owned(),
            |commit| encoded(commit.as_bytes()),
        )
    }
}

/// What [`Origin::measure`] found.
struct Measured {
    bytes: u64,
    digest: Option<[u8; 32]>,
    fault: Option<String>,
}

impl Measured {
    /// Nothing measured, for `why`.
    const fn failed(why: String) -> Self {
        Self {
            bytes: 0,
            digest: None,
            fault: Some(why),
        }
    }
}

/// `bytes` as one journal field value (D-3317): every byte outside
/// `A-Z a-z 0-9 - . _ ~ /` written `%XX`, upper-case hex, so a value never
/// holds a space or a newline and loses no byte.
fn encoded(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len());
    for &byte in bytes {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~' | b'/') {
            out.push(char::from(byte));
        } else {
            let _infallible = write!(out, "%{byte:02X}");
        }
    }
    out
}

/// The central directory of the zip at `path`, its bytes exactly as stored:
/// the end record found in the file's last 65,557 bytes (22 and the longest
/// comment), the ZIP64 end record when its locator sits before it (APPNOTE
/// 4.3.14-15), and the directory at the offset and size they state, at most
/// [`CENTRAL_DIRECTORY_CAP`]. The same records `gdfl_archive` reads; this
/// reads their bytes for an identity and decodes no entry.
///
/// # Errors
///
/// The path and what was wrong, in words.
fn central_directory_bytes(path: &Path) -> Result<Vec<u8>, String> {
    use crate::gdfl_archive::ReadAt as _;
    const END: u32 = 0x0605_4b50;
    const LOCATOR: u32 = 0x0706_4b50;
    const ZIP64_END: u32 = 0x0606_4b50;
    let shown = path.display();
    let file = std::fs::File::open(path).map_err(|why| format!("{shown}: {why}"))?;
    let len = file
        .metadata()
        .map_err(|why| format!("{shown}: {why}"))?
        .len();
    let read = |at: u64, n: u64| -> Result<Vec<u8>, String> {
        let mut buf = vec![0_u8; usize::try_from(n).map_err(|_| format!("{shown}: too long"))?];
        file.read_at(at, &mut buf)
            .map_err(|why| format!("{shown}: {why}"))?;
        Ok(buf)
    };
    let le = |bytes: &[u8], at: usize, width: usize| -> Option<u64> {
        let field = bytes.get(at..at.checked_add(width)?)?;
        Some(
            field
                .iter()
                .rev()
                .fold(0_u64, |value, byte| (value << 8) | u64::from(*byte)),
        )
    };
    let tail_len = len.min(65_557);
    let tail_at = len - tail_len;
    let tail = read(tail_at, tail_len)?;
    let end = (0..tail.len().saturating_sub(21))
        .rev()
        .find(|&at| {
            le(&tail, at, 4) == Some(u64::from(END))
                && le(&tail, at + 20, 2)
                    .and_then(|comment| usize::try_from(comment).ok())
                    .is_some_and(|comment| at + 22 + comment == tail.len())
        })
        .ok_or_else(|| format!("{shown}: no end-of-central-directory record ends it"))?;
    let cut = || format!("{shown}: a record is cut short");
    let mut size = le(&tail, end + 12, 4).ok_or_else(cut)?;
    let mut offset = le(&tail, end + 16, 4).ok_or_else(cut)?;
    let end_at = tail_at + u64::try_from(end).map_err(|_| cut())?;
    if let Some(locator_at) = end_at.checked_sub(20) {
        let locator = read(locator_at, 20)?;
        if le(&locator, 0, 4) == Some(u64::from(LOCATOR)) {
            let record_at = le(&locator, 8, 8).ok_or_else(cut)?;
            if record_at.checked_add(56).is_none_or(|stop| stop > len) {
                return Err(format!("{shown}: the ZIP64 end record lies past the end"));
            }
            let record = read(record_at, 56)?;
            if le(&record, 0, 4) != Some(u64::from(ZIP64_END)) {
                return Err(format!(
                    "{shown}: no ZIP64 end record where its locator says"
                ));
            }
            size = le(&record, 40, 8).ok_or_else(cut)?;
            offset = le(&record, 48, 8).ok_or_else(cut)?;
        }
    }
    if size > CENTRAL_DIRECTORY_CAP {
        return Err(format!(
            "{shown}: its central directory of {size} bytes is over the cap of \
             {CENTRAL_DIRECTORY_CAP}"
        ));
    }
    if offset.checked_add(size).is_none_or(|stop| stop > len) {
        return Err(format!("{shown}: its central directory lies past the end"));
    }
    read(offset, size)
}

/// A zip source's bytes and digest ([`Origin::digest`]): one outer zip, or a
/// directory of the yearly zips `gdfl_nfo::NfoZips` opens, `<yyyy>.zip`.
fn measure_zips(path: &Path) -> Measured {
    let shown = path.display();
    let meta = match std::fs::metadata(path) {
        Ok(meta) => meta,
        Err(why) => return Measured::failed(format!("{shown}: {why}")),
    };
    if meta.is_file() {
        let (digest, fault) = match central_directory_bytes(path) {
            Ok(directory) => (Some(brutex_core::blake3::hash(&directory)), None),
            Err(why) => (None, Some(why)),
        };
        return Measured {
            bytes: meta.len(),
            digest,
            fault,
        };
    }
    let entries = match std::fs::read_dir(path) {
        Ok(entries) => entries,
        Err(why) => return Measured::failed(format!("{shown}: {why}")),
    };
    let mut years: Vec<(String, PathBuf)> = Vec::new();
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(why) => return Measured::failed(format!("{shown}: {why}")),
        };
        let name = entry.file_name().to_string_lossy().into_owned();
        let year = name
            .strip_suffix(".zip")
            .is_some_and(|stem| stem.len() == 4 && stem.bytes().all(|b| b.is_ascii_digit()));
        if year {
            years.push((name, entry.path()));
        }
    }
    years.sort_unstable();
    let mut measured = Measured {
        bytes: 0,
        digest: None,
        fault: years
            .is_empty()
            .then(|| format!("{shown}: holds no yearly zip <yyyy>.zip")),
    };
    let mut hasher = brutex_core::blake3::Hasher::new();
    for (name, year) in &years {
        match std::fs::metadata(year) {
            Ok(meta) => measured.bytes = measured.bytes.saturating_add(meta.len()),
            Err(why) => {
                let _first = measured
                    .fault
                    .get_or_insert_with(|| format!("{}: {why}", year.display()));
            }
        }
        match central_directory_bytes(year) {
            Ok(directory) => {
                hasher.update(name.as_bytes());
                hasher.update(&[0]);
                hasher.update(
                    &u64::try_from(directory.len())
                        .unwrap_or(u64::MAX)
                        .to_le_bytes(),
                );
                hasher.update(&directory);
            }
            Err(why) => {
                let _first = measured.fault.get_or_insert(why);
            }
        }
    }
    if measured.fault.is_none() {
        measured.digest = Some(hasher.finalize());
    }
    measured
}

/// A tick store's bytes (every `.bts` file under its root, dot-entries
/// skipped and links not followed, so a link cycle cannot loop the walk) and
/// the SHA-256 of its `FORMAT.md` ([`Origin::digest`]).
fn measure_tick_store(root: &Path) -> Measured {
    use sha2::Digest as _;
    let mut measured = Measured {
        bytes: 0,
        digest: None,
        fault: None,
    };
    let mut stack = vec![root.to_path_buf()];
    let mut visited: usize = 0;
    'walk: while let Some(dir) = stack.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(why) => {
                measured.fault = Some(format!("{}: {why}", dir.display()));
                break;
            }
        };
        for entry in entries {
            visited += 1;
            if visited > ORIGIN_WALK_CAP {
                measured.fault = Some(format!(
                    "{}: more than {ORIGIN_WALK_CAP} entries; the walk stopped there",
                    root.display()
                ));
                break 'walk;
            }
            let found = entry.and_then(|entry| Ok((entry.file_type()?, entry)));
            let (kind, entry) = match found {
                Ok(found) => found,
                Err(why) => {
                    measured.fault = Some(format!("{}: {why}", dir.display()));
                    break 'walk;
                }
            };
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                continue;
            }
            if kind.is_dir() {
                stack.push(entry.path());
            } else if kind.is_file()
                && Path::new(&name)
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("bts"))
            {
                match entry.metadata() {
                    Ok(meta) => measured.bytes = measured.bytes.saturating_add(meta.len()),
                    Err(why) => {
                        measured.fault = Some(format!("{}: {why}", entry.path().display()));
                        break 'walk;
                    }
                }
            }
        }
    }
    let format = root.join("FORMAT.md");
    let text = std::fs::metadata(&format)
        .and_then(|meta| {
            if meta.len() > FORMAT_CAP {
                return Err(std::io::Error::other(format!(
                    "{} bytes, over the cap of {FORMAT_CAP}",
                    meta.len()
                )));
            }
            std::fs::read(&format)
        })
        .map_err(|why| format!("{}: {why}", format.display()));
    match text {
        Ok(text) => {
            if measured.fault.is_none() {
                measured.digest = Some(sha2::Sha256::digest(&text).into());
            }
        }
        Err(why) => {
            let _first = measured.fault.get_or_insert(why);
        }
    }
    measured
}

/// What a run is given beside its [`Run`] (D-3317): where its data came from,
/// and whether to read again the days closed done with refusals.
#[derive(Debug, Clone, Copy, Default)]
pub struct RunOptions<'a> {
    /// The source the caller measured, written as the run's `run` line and in
    /// "run started". `None` writes `unrecorded` for every part of it: named,
    /// never left out. `cli gdfl-import` always measures one.
    pub origin: Option<&'a Origin>,
    /// Read again every day of the range closed done with refusals
    /// (`cli gdfl-import ... --retry-refused`, D-3317 item 2): how a day is
    /// re-read after a reader learns to accept what it refused. Each is named
    /// in [`Report::resumed`].
    pub retry_refused: bool,
    /// How many workers read and file one day's files at once (WP-16,
    /// DB-N25). The bytes a run leaves do not depend on it
    /// (`gdfl_import_tests::w_workers_build_the_same_store_journal_and_census_as_one`).
    pub workers: Workers,
    /// When a day's writes are made durable (WP-16, D-3353). The bytes a run
    /// leaves do not depend on it either
    /// (`gdfl_import_tests::the_durability_unit_changes_no_byte_a_run_leaves`).
    pub durability: Durability,
}

/// When an import's writes are made durable (WP-16, DB-N25, D-3353).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Durability {
    /// Every append, every new name and every provenance record flushed as
    /// it is written, before the next: the store's own default, and every
    /// import before WP-16. A power cut at any point leaves every month at a
    /// commit.
    EachWrite,
    /// Every write of a day's files left to one [`store::file::DurableBatch`]
    /// of this kind, made durable when the day's last file is filed and
    /// before its ledger lines, its census batch and its closing line. A kill
    /// at any point leaves every month at a commit and the day open, so it is
    /// read again; a power cut can leave a month the open day wrote refused
    /// by its own checks until it is repaired (`store::file`'s header).
    Day(store::file::BatchFlush),
}

impl Durability {
    /// The default, chosen by measurement (WP-16): the day, through `O_SYNC`
    /// writes and one flush per device.
    pub const DEFAULT: Self = Self::Day(store::file::BatchFlush::OncePerDevice);

    /// The word `BRUTEX_GDFL_DURABILITY` and the run's report use.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::EachWrite => "each-write",
            Self::Day(store::file::BatchFlush::OncePerDevice) => "day",
            Self::Day(store::file::BatchFlush::EveryFile) => "day-each-file",
        }
    }

    /// The durability `word` names, or the words that name one.
    ///
    /// # Errors
    ///
    /// The words a caller shows, for any other word.
    pub fn parse(word: &str) -> Result<Self, String> {
        [
            Self::Day(store::file::BatchFlush::OncePerDevice),
            Self::Day(store::file::BatchFlush::EveryFile),
            Self::EachWrite,
        ]
        .into_iter()
        .find(|durability| durability.as_str() == word)
        .ok_or_else(|| {
            format!("{word:?} is not a durability: the words are day, day-each-file and each-write")
        })
    }

    /// A fresh batch for one day, or none. A run that flushes each write as
    /// it is written takes one only for a day an earlier run `left_open`,
    /// to confirm that day's months in: nothing enters it, so every writer
    /// still flushes for itself, and each confirmed file is flushed for
    /// itself when the day ends.
    fn batch(self, left_open: bool) -> Option<store::file::DurableBatch> {
        match self {
            Self::EachWrite => left_open
                .then(|| store::file::DurableBatch::new(store::file::BatchFlush::EveryFile)),
            Self::Day(flush) => Some(store::file::DurableBatch::new(flush)),
        }
    }
}

impl Default for Durability {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Threads that issue a day's deferred flushes (WP-16). They wait on the
/// drive, not on a core, and the drive serves more flushes a second the more
/// are asked at once: about 2,200 at 12 and 3,400 at 32 (the scratch probe of
/// `store::file`'s batch section). Only the directories that gained a name
/// and one file per device are flushed, so most days ask a handful.
const FLUSH_THREADS: usize = 32;

/// The workers that read and file one day's files at once (WP-16, DB-N25):
/// 1 to [`Workers::MAX`], [`Workers::for_host`] unless a caller asks.
///
/// Days still run in order, one after the other. Inside a day each wanted
/// file is one job, taken in listing order by whichever worker is free; one
/// file is one contract-month (an option contract, a share, an index), so
/// every month file, its checksum and index sidecars, its provenance record
/// and its closing-auction sidecar are written by exactly one worker. What
/// the workers did is folded into the day in listing order, so the report,
/// the day ledger, the census batch and the journal are the ones a single
/// worker writes. With one worker no thread is started.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Workers(usize);

impl Workers {
    /// The most a caller may ask for, on any host: the end of what was
    /// measured (WP-16). It bounds the threads one run starts and is not a
    /// count of cores: a worker waits on the drive more than on a core, so a
    /// caller may ask for more workers than the host has cores, up to this.
    /// Nothing past 12 was measured, so nothing past 12 is taken.
    pub const MAX: usize = 12;

    /// Where more workers stopped paying, by measurement (WP-16, the
    /// operator's drive, options 2020-09-14 and 15 one day per run): 4
    /// workers took 85.5 s and 52.1 s, 8 took 65.5 s and 45.9 s, 12 took
    /// 67.8 s and 46.0 s. Past eight the drive's flushes, not the cores, set
    /// the pace. With the day made durable together (D-3353; 11 Oct 2026,
    /// 2020-09-15 into an empty store, machine load 23 to 30) 4 workers took
    /// 18.7 s, 8 took 13.2 s and 12 took 12.4 s.
    const KNEE: usize = 8;

    /// Cores a default run leaves free, for the api and the rest of the
    /// machine.
    const SPARE: usize = 2;

    /// The workers a run takes when nobody asks, on a host of `cores`: every
    /// core but [`Self::SPARE`], never fewer than one and never past
    /// [`Self::KNEE`].
    #[must_use]
    pub const fn for_cores(cores: usize) -> Self {
        let free = cores.saturating_sub(Self::SPARE);
        Self(if free < 1 {
            1
        } else if free > Self::KNEE {
            Self::KNEE
        } else {
            free
        })
    }

    /// The workers a run takes when nobody asks, READ FROM THE HOST when the
    /// run starts (`std::thread::available_parallelism`, which honours a
    /// limit set on the process), not written down for one machine. A host
    /// that will not say is taken as one core, so one worker; a run prints
    /// the workers it used either way.
    #[must_use]
    pub fn for_host() -> Self {
        Self::for_cores(std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get))
    }

    /// `n` workers.
    ///
    /// # Errors
    ///
    /// The words a caller shows, for 0 or more than [`Self::MAX`].
    pub fn new(n: usize) -> Result<Self, String> {
        if matches!(n, 1..=Self::MAX) {
            Ok(Self(n))
        } else {
            Err(format!(
                "{n} workers: a run takes 1 to {} (WP-16)",
                Self::MAX
            ))
        }
    }

    /// How many.
    #[must_use]
    pub const fn get(self) -> usize {
        self.0
    }
}

impl Default for Workers {
    fn default() -> Self {
        Self::for_host()
    }
}

/// One row as both readers hand it on: its second, its price in paisa, its
/// traded quantity and, for options, its open interest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tick {
    /// Second of the IST day, as stamped.
    pub sod: u32,
    /// Last traded price, paisa.
    pub ltp: i64,
    /// Last traded quantity.
    pub ltq: u64,
    /// Open interest, `None` where the tree carries none.
    pub oi: Option<i64>,
}

/// What the row rule and the placement did to one file's rows.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Placement {
    /// Rows the file held.
    pub rows: usize,
    /// Rows dropped because they record no trade (LTQ = 0, stocks and options).
    pub ltq_zero_dropped: usize,
    /// Kept rows stamped below the running maximum, each deferred.
    pub late_rows: usize,
    /// Late rows with no in-order row after them, dropped.
    pub late_unresolved: usize,
    /// The largest back-step seen, in seconds.
    pub max_back_s: u32,
}

/// One file's bars.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Converted {
    /// One-second bars, one per second holding a kept row.
    pub seconds: Vec<Bar>,
    /// What the rule and the placement did.
    pub placement: Placement,
}

/// The UTC microsecond of second `sod` of IST day `day`.
fn micros_at(day: Day, sod: u32) -> i64 {
    (i64::from(day.days_from_epoch()) * 86_400 + i64::from(sod) - IST_OFFSET_SECS) * 1_000_000
}

/// One row of a file as the placement leaves it: its placed second, the row,
/// whether the row rule keeps it as a price (every index row; a traded share
/// or option row), and whether it was late.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PlacedRow {
    second: u32,
    tick: Tick,
    counts: bool,
    late: bool,
}

/// Every row of one file with the second it is placed at (module doc), in
/// file order, untraded rows included: a row with no in-order row after it
/// has no second and is not here. The counts are about kept rows only, as
/// before D-3342: `ltq_zero_dropped` names the untraded rows, which build no
/// price and since D-3342 still carry their second's open interest.
fn place_all(kind: ImportKind, ticks: &[Tick], placement: &mut Placement) -> Vec<PlacedRow> {
    // THE CLOCK IS EVERY ROW, THE BAR IS THE KEPT ROWS (D-2812). The running
    // maximum and the next in-order stamp are taken over every row, untraded
    // ones included: a trade written after a row stamped 10:00:05 was known
    // no earlier than 10:00:05 whatever that row's LTQ, so filtering first
    // would file it before the file had shown that time (CLAUDE.md §3 rule
    // 7). The counts are about kept rows only.
    placement.rows = ticks.len();
    let counts = |tick: &Tick| kind.every_row_counts() || tick.ltq > 0;
    // Forward: in order when at or above the running maximum of every row.
    let mut high: Option<u32> = None;
    let mut in_order = Vec::with_capacity(ticks.len());
    for tick in ticks {
        match high {
            Some(max) if tick.sod < max => {
                if counts(tick) {
                    placement.late_rows += 1;
                    placement.max_back_s = placement.max_back_s.max(max - tick.sod);
                }
                in_order.push(false);
            }
            _ => {
                high = Some(tick.sod);
                in_order.push(true);
            }
        }
    }
    // Backward: a late row lands at the next in-order row's stamp.
    let mut next: Option<u32> = None;
    let mut placed: Vec<Option<u32>> = Vec::with_capacity(ticks.len());
    for (tick, &ordered) in ticks.iter().zip(&in_order).rev() {
        if ordered {
            next = Some(tick.sod);
        }
        placed.push(next);
    }
    placed.reverse();
    let mut out = Vec::with_capacity(ticks.len());
    for ((tick, second), &ordered) in ticks.iter().copied().zip(placed).zip(&in_order) {
        let kept = counts(&tick);
        match (kept, second) {
            (false, Some(second)) => {
                placement.ltq_zero_dropped += 1;
                out.push(PlacedRow {
                    second,
                    tick,
                    counts: false,
                    late: !ordered,
                });
            }
            (false, None) => placement.ltq_zero_dropped += 1,
            (true, Some(second)) => out.push(PlacedRow {
                second,
                tick,
                counts: true,
                late: !ordered,
            }),
            (true, None) => placement.late_unresolved += 1,
        }
    }
    out
}

/// One file's rows as one-second bars under the agreed
/// definition (module doc).
///
/// # Errors
///
/// [`ImportRefusal::StampPastTheDay`] for a row stamped at or past second
/// 86,400, [`ImportRefusal::VolumeOverflow`] for a quantity or a second's sum
/// past `i64::MAX`, and [`ImportRefusal::Fold`] for anything else the fold
/// refuses.
pub fn convert(kind: ImportKind, day: Day, ticks: &[Tick]) -> Result<Converted, ImportRefusal> {
    convert_noted(kind, day, ticks).map(|(converted, _)| converted)
}

/// [`convert`], and what its placement deferred ([`quality::Deferred`]):
/// how many placed seconds took at least one late kept row (D-3335's
/// `deferred_s`) and the most seconds a kept row was deferred by.
///
/// Under [`BAR_DEFINITION`] 3 (D-3342) an option second's open interest is
/// the OI of the last row PLACED in it, traded or not
/// (`fold::fold_seconds_with_last_oi`); its prices and volume are the traded
/// rows' exactly as before, and a second with no traded row has no bar. An
/// index or a share second folds its kept rows with [`fold`], as before: the
/// capital-market trees carry no open interest.
fn convert_noted(
    kind: ImportKind,
    day: Day,
    ticks: &[Tick],
) -> Result<(Converted, quality::Deferred), ImportRefusal> {
    // Both readers hold a stamp to the day; this runtime is public and holds
    // it too, every row's, untraded ones included, since each moves the
    // placement (D-2825, D-2812).
    for tick in ticks {
        if tick.sod >= 86_400 {
            return Err(ImportRefusal::StampPastTheDay { sod: tick.sod });
        }
    }
    let mut placement = Placement::default();
    let placed = place_all(kind, ticks, &mut placement);
    let mut deferred = quality::Deferred::default();
    let mut last_deferred: Option<u32> = None;
    for row in placed.iter().filter(|row| row.counts && row.late) {
        // Placed seconds never fall in file order, so a new second is a
        // change from the last one counted.
        if last_deferred != Some(row.second) {
            deferred.seconds += 1;
            last_deferred = Some(row.second);
        }
        // A late row lands at or after its own stamp, never before it.
        deferred.max_s = deferred.max_s.max(row.second.saturating_sub(row.tick.sod));
    }
    let bar_of = |row: &PlacedRow| -> Result<Bar, ImportRefusal> {
        let volume = if kind.every_row_counts() || !row.counts {
            0
        } else {
            i64::try_from(row.tick.ltq)
                .map_err(|_| ImportRefusal::VolumeOverflow { sod: row.second })?
        };
        Ok(Bar {
            ts_micros: micros_at(day, row.second),
            open: row.tick.ltp,
            high: row.tick.ltp,
            low: row.tick.ltp,
            close: row.tick.ltp,
            volume,
            open_interest: row.tick.oi.unwrap_or(OI_NULL),
        })
    };
    let seconds = if kind == ImportKind::Options {
        let mut rows = Vec::with_capacity(placed.len());
        for row in &placed {
            rows.push(crate::fold::Placed {
                bar: bar_of(row)?,
                traded: row.counts,
            });
        }
        crate::fold::fold_seconds_with_last_oi(&rows).map_err(|why| fold_refusal(why, day))?
    } else {
        let mut rows = Vec::with_capacity(placed.len());
        for row in placed.iter().filter(|row| row.counts) {
            rows.push(bar_of(row)?);
        }
        fold(&rows, Bucket::SECOND).map_err(|why| fold_refusal(why, day))?
    };
    Ok((Converted { seconds, placement }, deferred))
}

/// What a fold refusal is to the import: a volume sum past `i64::MAX` names
/// its second; any other (none reachable once rows are placed in order with
/// non-negative quantities) is named in the fold's own words.
fn fold_refusal(why: crate::fold::FoldError, day: Day) -> ImportRefusal {
    match why {
        crate::fold::FoldError::VolumeOverflow { bucket, .. } => ImportRefusal::VolumeOverflow {
            sod: second_of(bucket, day),
        },
        other => ImportRefusal::Fold {
            why: format!("{other:?}"),
        },
    }
}

/// The second of `day` a bucket starting at `ts_micros` names.
fn second_of(ts_micros: i64, day: Day) -> u32 {
    let secs = ts_micros.div_euclid(1_000_000) + IST_OFFSET_SECS
        - i64::from(day.days_from_epoch()) * 86_400;
    u32::try_from(secs).unwrap_or(0)
}

/// One instrument's file of one day, as a source hands it on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TickFile {
    /// The symbol the bars are filed under: `NIFTY`, `RELIANCE`, the option's
    /// underlying.
    pub symbol: String,
    /// The option contract, `None` for spot.
    pub contract: Option<Contract>,
    /// The vendor's file name, for every refusal and event about it.
    pub name: String,
    /// Every row, in file order.
    pub ticks: Vec<Tick>,
}

/// What a source did with one day.
#[derive(Debug, Default)]
pub struct DayRead {
    /// Whether the source holds the day at all.
    pub held: bool,
    /// Entries of the day that are not wanted files (off the surface, outside
    /// the filter, not a file of the day).
    pub skipped: usize,
    /// Files the source or the reader refused, named, each with its
    /// [`Cause`] ([`cm_cause`], [`nfo_cause`]; D-3317).
    pub refused: Vec<Refused>,
}

/// One file the source or the reader refused, and why, as the journal names
/// it (D-3317). It reads as the [`Failure`] it carries, so `refused[i].why`
/// is the refusal's words, as before the cause was kept.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refused {
    /// The stage and code, which decide whether the day can close with it.
    pub cause: Cause,
    /// What was refused and why, in words.
    pub failure: Failure,
}

impl Refused {
    /// `failure`, refused for `cause`.
    #[must_use]
    pub const fn at(cause: Cause, failure: Failure) -> Self {
        Self { cause, failure }
    }

    /// `about`, refused by a capital-market reader or source.
    #[must_use]
    pub fn cm(about: &str, why: &CmRefusal) -> Self {
        Self::at(cm_cause(why), failure(about, why.to_string()))
    }

    /// `about`, refused by the options reader or source.
    #[must_use]
    pub fn nfo(about: &str, why: &NfoRefusal) -> Self {
        Self::at(nfo_cause(why), failure(about, why.to_string()))
    }
}

impl core::ops::Deref for Refused {
    type Target = Failure;

    fn deref(&self) -> &Failure {
        &self.failure
    }
}

/// A failure about `instrument`, in words: the one place this module builds
/// a [`Failure`] by its fields, so a field `pull::ingest` adds is added here.
/// A reader's refusal is not yet tied to a day here; [`note_failure`] gives
/// it the day it is reported under.
fn failure(instrument: &str, why: String) -> Failure {
    Failure {
        instrument: instrument.to_owned(),
        why,
        month: None,
        day: None,
    }
}

/// A failure of import day `day`, carrying that day and its month, so a log
/// or a ring files it under the day it concerns and never under the run's
/// first month (DB-G45, D-3341 item 1).
fn failure_on(day: Day, instrument: &str, why: String) -> Failure {
    Failure {
        month: day.year_month().ok(),
        day: Some(day),
        ..failure(instrument, why)
    }
}

/// What one run did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    /// Days closed `done` under a narrower scope whose listing this run read
    /// again for the members that close never covered (WP-12f, D-4878).
    pub days_widened: usize,
    /// Member keys this run read on such days, one per member with a file.
    pub members_reopened: usize,
    /// The days BEFORE the run's range it read a member of: done days of the
    /// range's first month that owed this run a member the day holds a file
    /// of, read first, so no member's month is made from a later day while an
    /// earlier one is owed (D-4878, DB-B76). Each is in `days_widened`, once
    /// per key that closed it; a day only listed, with no file to read, is
    /// in `days_widened` alone.
    pub days_before_range: usize,
    /// Days that were imported this run (clean or not): a day read whole,
    /// and a day read again for a wider scope on which the run closed at
    /// least one member key (that day is in `days_widened` too). One per
    /// day, never one per member key.
    pub days_imported: usize,
    /// Days the journal already records as done, skipped unread.
    pub days_skipped: usize,
    /// Days the calendar records as closed.
    pub days_closed: usize,
    /// Trading days the source does not hold, named.
    pub days_missing: Vec<Day>,
    /// Days refused before any file was read, named in `failures`.
    pub days_refused: usize,
    /// EVERY day the journal names that this run takes up again, in day
    /// order: one an earlier run began and never closed (a crash), one it
    /// closed `incomplete`, and one closed `done` under another bar
    /// definition (also in `restated`). Such a day is here whatever this run
    /// then finds (imported, or in `days_missing` when the source no longer
    /// holds it); a day the journal does not name is not here, nor is a day
    /// the calendar declines before the journal is asked (D-2821).
    pub resumed: Vec<Day>,
    /// The days of `resumed` whose last `done` line names a bar definition
    /// other than [`BAR_DEFINITION`], or none in a shape no definition-2 code
    /// wrote (D-2822).
    pub restated: Vec<Day>,
    /// Instrument files read.
    pub files: usize,
    /// Entries not wanted.
    pub files_skipped: usize,
    /// Files refused, named in `failures`.
    pub files_refused: usize,
    /// Rows those files held.
    pub rows: usize,
    /// Rows dropped as recording no trade.
    pub ltq_zero_dropped: usize,
    /// Late rows deferred.
    pub late_rows: usize,
    /// Late rows dropped with nothing after them.
    pub late_unresolved: usize,
    /// The largest back-step of any kept row, in seconds (D-2811): a forward
    /// spike collapses hours into one second, and this says how far.
    pub max_back_s: u32,
    /// One-second bars offered to the store.
    pub seconds: usize,
    /// One-second bars the store wrote this run.
    pub seconds_committed: usize,
    /// Bars the venue's session declined (pre-open, post-close).
    pub outside_session: usize,
    /// Every failure, named.
    pub failures: Vec<Failure>,
    /// The days closed done WITH refusals, in day order (D-3317 item 2): the
    /// day wrote and every failure was a refusal of the vendor's own bytes
    /// ([`Cause::closes`]), so a re-run skips them unless it is given
    /// [`RunOptions::retry_refused`]. Their failures are in `failures` like
    /// any other.
    pub closed_with_refusals: Vec<Day>,
    /// The days closed `incomplete`, in day order (D-3317): the day wrote and
    /// a failure does not close it. A census-refused day is not here: it ends
    /// the run, and the refusal names it.
    pub closed_incomplete: Vec<Day>,
    /// The days the source held that closed `refused`, in day order (D-3317
    /// item 3): every wanted file refused, or the listing. A day the calendar
    /// declines is counted in `days_refused`, one the source does not hold
    /// in `days_missing`; each is journalled `refused` too.
    pub closed_refused: Vec<Day>,
    /// Lines appended to the GDFL day ledger (D-3335): a line equal to its
    /// key's last one, `src` aside, is not written again.
    pub flags_lines: usize,
    /// Records appended to the lineage ledger (D-3331): aliases, renamed
    /// contracts and reused keys.
    pub lineage_lines: usize,
    /// Records appended to the inferred-lot ledger (D-3343).
    pub lot_lines: usize,
    /// Corporate-action adjustments the lineage detector found (D-3331).
    pub adjustments: usize,
    /// Provenance records written beside the months the run appended to
    /// (D-3334): one per file that committed bars, when the run names its
    /// origin.
    pub provenance_records: usize,
    /// Month files a day was back-filled into as a NEW GENERATION (D-3350;
    /// DB-N26, WP-12d): one per file whose day the append-only month refused
    /// because it lies inside what the month holds, and the versioned repair
    /// took. Each is journaled in `<store>/audit/gdfl.repairs`; its added
    /// bars are in [`Report::seconds_committed`], and it writes no `.prv`
    /// record (a revision carries none).
    pub months_repaired: usize,
}

/// What one run is asked to do.
#[derive(Debug, Clone, Copy)]
pub struct Run<'a> {
    /// Which tree.
    pub kind: ImportKind,
    /// The first day, inclusive.
    pub from: Day,
    /// The last day, inclusive.
    pub to: Day,
    /// Only these symbols (spot) or underlyings (options); empty is every one.
    pub only: &'a [String],
    /// The bar store's root.
    pub store_root: &'a Path,
}

impl Run<'_> {
    /// Refuses a filter name that is not a symbol as the store files it
    /// (D-2811): the journal key and the match are both by exact bytes.
    ///
    /// An OPTIONS filter is matched against the key a file is filed under,
    /// which for a renamed underlying is its newest symbol (D-3354). An old
    /// symbol there matched no file and still journaled the day `done` with
    /// `files=0`, so it is refused with the key to ask for. A capital-market
    /// filter is not refused: `cli`'s stored scope hands both names of a
    /// renamed share, and the day's key is those names as given.
    fn check_filter(&self) -> Result<(), ImportRefusal> {
        for name in self.only {
            let filed = brutex_core::symbol::Symbol::new(name)
                .is_ok_and(|symbol| symbol.as_str() == name.as_str());
            if !filed {
                return Err(ImportRefusal::FilterName { name: name.clone() });
            }
            let current = brutex_core::alias::LINEAGE.current(name);
            if self.kind == ImportKind::Options && current != name.as_str() {
                return Err(ImportRefusal::FilterRenamed {
                    name: name.clone(),
                    filed: current.to_owned(),
                });
            }
        }
        Ok(())
    }

    /// The old symbols the store holds LIVE option series under whose files
    /// this run would file under another key (D-3354), each with that key.
    /// Empty for a spot run.
    ///
    /// One directory open per applied row of `core::alias::SYMBOL_CHANGES`
    /// whose newest symbol the filter wants. A directory that is there is
    /// walked until its first contract the lineage ledger has neither
    /// superseded nor retired (the re-key's records, D-3331): that one
    /// contract is what the run would split. A directory whose every
    /// contract is withdrawn, or that holds none, is history and blocks
    /// nothing, so superseding the old keys is what lifts the refusal; the
    /// walk is then the directory's length, once per run. A name in it that
    /// is no contract segment is not a contract the store holds.
    ///
    /// A directory or an entry that cannot be looked at counts as held: that
    /// is not proof of absence.
    ///
    /// # Errors
    ///
    /// [`ImportRefusal::Lineage`] when an old-symbol directory exists and
    /// the ledger that would say whether it is superseded cannot be read.
    fn split_underlyings(&self) -> Result<Vec<(String, String)>, ImportRefusal> {
        use brutex_core::instrument::{Exchange, Segment};
        if self.kind != ImportKind::Options {
            return Ok(Vec::new());
        }
        let base = self
            .store_root
            .join(store::path::STORE_ROOT)
            .join(Vendor::Gdfl.as_str())
            .join(Exchange::Nse.as_str())
            .join(Segment::Fno.as_str());
        let mut held = Vec::new();
        let mut ledger: Option<crate::gdfl_rekey::Withdrawn> = None;
        for row in brutex_core::alias::LINEAGE.rows() {
            let filed = brutex_core::alias::LINEAGE.current(row.old);
            if !row.status.applies() || filed == row.old || !self.wants(filed) {
                continue;
            }
            let entries = match std::fs::read_dir(base.join(row.old)) {
                Ok(entries) => entries,
                Err(why) if why.kind() == std::io::ErrorKind::NotFound => continue,
                Err(_) => {
                    held.push((row.old.to_owned(), filed.to_owned()));
                    continue;
                }
            };
            let withdrawn = match &mut ledger {
                Some(read) => read,
                slot => slot.insert(
                    crate::gdfl_rekey::withdrawn(self.store_root)
                        .map_err(|why| ImportRefusal::Lineage { why })?,
                ),
            };
            let symbol = brutex_core::symbol::Symbol::new(row.old).ok();
            let live = entries.into_iter().any(|entry| {
                let Ok(entry) = entry else {
                    return true;
                };
                let name = entry.file_name();
                let contract = name
                    .to_str()
                    .and_then(|name| Contract::parse(name).filter(|c| c.as_str() == name));
                match (symbol, contract) {
                    (Some(symbol), Some(contract)) => {
                        withdrawn.superseded(symbol, contract).is_none()
                            && withdrawn.retired(symbol, contract).is_none()
                    }
                    // Not a contract the store files, or a table symbol the
                    // store cannot spell: nothing here to split.
                    _ => false,
                }
            });
            if live {
                held.push((row.old.to_owned(), filed.to_owned()));
            }
        }
        Ok(held)
    }

    /// Whether `symbol` passes the filter.
    fn wants(&self, symbol: &str) -> bool {
        self.only.is_empty() || self.only.iter().any(|only| only == symbol)
    }

    /// The journal key's filter part: the sorted names, or `*`.
    fn filter_key(&self) -> String {
        if self.only.is_empty() {
            return "*".to_owned();
        }
        // The operator's own few names, put in order and deduplicated so the
        // key does not depend on how they were typed.
        let mut names = std::collections::BTreeSet::new();
        for name in self.only {
            names.insert(name.as_str());
        }
        names.into_iter().collect::<Vec<_>>().join(",")
    }
}

/// The import journal's path under a store root.
#[must_use]
pub fn journal_path(store_root: &Path) -> PathBuf {
    store_root.join("imports").join("gdfl.journal")
}

/// The version of the one-second bar definition (the row rule, the
/// placement and the fold this module applies), written into every `done`
/// and `incomplete` line as `definition=<n>` (D-2822). Bump it whenever a
/// change makes one file build different bars: a day closed `done` under
/// another number is imported again, and the append-only store judges the
/// result, answering `AlreadyPresent` for the same bars and refusing the file
/// by name for different ones, never writing over them.
///
/// 1 is D-2802 as first written, whose placement took the running maximum
/// of traded rows only; 2 is D-2812, which takes it over every row; 3 is
/// D-3342 (the operator's answer, 7 Oct 2026): an option bar's open interest
/// is the OI of the LAST row placed in its second, traded or not
/// (`fold::fold_seconds_with_last_oi`), where 2 took the last traded row's.
pub const BAR_DEFINITION: u32 = 3;

/// The definition D-3342 replaced.
const D3342_PREVIOUS: u32 = 2;

/// Whether a day of `kind` built under `definition` holds the bars this
/// build would build (D-2822, D-3342): [`BAR_DEFINITION`], or for an index
/// or a share also 2. Definition 3 changed only the open interest of a
/// second, and the capital-market trees carry none (`OI_NULL`, D-3322), so
/// their bars under 2 and 3 are the same bars, byte for byte; reading those
/// days again would change nothing and is not asked of them. An option day
/// built under 2 is restated: its open interest may differ.
#[must_use]
pub fn definition_is_current(kind: ImportKind, definition: u64) -> bool {
    definition == u64::from(BAR_DEFINITION)
        || (definition == u64::from(D3342_PREVIOUS) && kind != ImportKind::Options)
}

/// [`definition_is_current`] for a journal key's kind word; a word that is
/// no kind is current only under [`BAR_DEFINITION`].
fn current_for(kind: &str, definition: u32) -> bool {
    ImportKind::parse(kind).map_or(definition == BAR_DEFINITION, |kind| {
        definition_is_current(kind, u64::from(definition))
    })
}

/// The definition the code of D-2811 and D-2812 built under. Both landed in
/// one commit, so every journal line that code wrote was written under 2.
const D2811_DEFINITION: u32 = 2;

/// The stats fields, in order, of every `done` and `incomplete` line the
/// journal wrote from D-2811 until it recorded `definition=` (D-2822).
/// Before D-2811 a line carried only the first three, under definition 1.
const D2811_STATS: [&str; 6] = [
    "files",
    "seconds",
    "failures",
    "late",
    "late_unresolved",
    "max_back_s",
];

/// The `definition=` field of a journal line's stats: `Ok(None)` when it
/// has none, `Err(())` when it has two or one that is not a number in the
/// form [`BAR_DEFINITION`] is written in.
fn definition_of(stats: &[&str]) -> Result<Option<u32>, ()> {
    let mut found = None;
    for field in stats {
        if let Some(text) = field.strip_prefix("definition=") {
            let value: u32 = text.parse().map_err(|_| ())?;
            if found.is_some() || value.to_string() != text {
                return Err(());
            }
            found = Some(value);
        }
    }
    Ok(found)
}

/// The bar definition a `done` line says built its day: its `definition=`
/// field, else [`D2811_DEFINITION`] when its stats are exactly
/// [`D2811_STATS`], each a decimal count, as the code of D-2811 wrote them
/// before the field existed; else `None`, a day no definition is known for
/// (D-2822). O(fields).
fn built_under(stats: &[&str]) -> Result<Option<u32>, ()> {
    if let Some(stated) = definition_of(stats)? {
        return Ok(Some(stated));
    }
    let d2811 = stats.len() == D2811_STATS.len()
        && stats.iter().zip(D2811_STATS).all(|(field, name)| {
            field
                .strip_prefix(name)
                .and_then(|rest| rest.strip_prefix('='))
                .is_some_and(|count| !count.is_empty() && count.bytes().all(|b| b.is_ascii_digit()))
        });
    Ok(d2811.then_some(D2811_DEFINITION))
}

/// The decimal count a field holds (`name=<n>`), `Ok(None)` when the line
/// has no such field, `Err(())` when it has two or one that is not a count as
/// this journal writes it (no sign, no leading zero but `0`). O(fields).
fn count_of(stats: &[&str], name: &str) -> Result<Option<u64>, ()> {
    let mut found = None;
    for field in stats {
        if let Some(text) = field
            .strip_prefix(name)
            .and_then(|rest| rest.strip_prefix('='))
        {
            let value: u64 = text.parse().map_err(|_| ())?;
            if found.is_some() || value.to_string() != text {
                return Err(());
            }
            found = Some(value);
        }
    }
    Ok(found)
}

/// The `run=` field of a closing line, refused when it is there and not a
/// count (D-3317 item 2). Lines written before D-3317 have none.
fn run_of(stats: &[&str]) -> Result<Option<u64>, ()> {
    count_of(stats, "run")
}

/// What closes a torn journal line, so every later load knows it for one
/// (D-2814). A line ending in it is skipped; no line the journal writes
/// whole can end in it, because a key is a kind, a day and symbol names.
const TORN: &str = " (torn)";

/// Whether `fragment`, a line cut short by a crash, can be the start of a
/// line the journal writes: a prefix of one of its [`VERBS`] and the space
/// after it, or text that starts with one (D-2814). Anything else at the end
/// of the file is not a torn journal line but a foreign one.
///
/// The close a load writes can itself be cut by a second crash, leaving `d`
/// followed by part of [`TORN`] (`d (t`), and the next load closes that
/// again. So every trailing piece of the mark, whole or cut, is taken off
/// before the verb rule is asked: each is a byte the journal wrote, and
/// judging `d (t` foreign refused every later run for ever (D-2817). The
/// mark's seven bytes are all different, so its last byte names the one
/// piece a fragment can end in. What is left is never empty: a close follows
/// only a fragment of at least a verb's first letter, which no piece of the
/// mark ends in. O(fragment length).
fn torn_fragment(fragment: &str) -> bool {
    let mut rest = fragment;
    'strip: loop {
        for cut in (1..=TORN.len()).rev() {
            if let Some(shorter) = TORN.get(..cut).and_then(|piece| rest.strip_suffix(piece)) {
                rest = shorter;
                continue 'strip;
            }
        }
        break;
    }
    !rest.is_empty()
        && VERBS
            .into_iter()
            .any(|verb| verb.starts_with(rest) || rest.starts_with(verb))
}

/// Every verb the journal holds, each with the space after it: `refused`,
/// `run` and `recounted` since D-3317 (`recounted` is written by `cli
/// census-recount`, D-3316).
const VERBS: [&str; 6] = [
    "begin ",
    "done ",
    "incomplete ",
    "refused ",
    "run ",
    "recounted ",
];

/// The journal, read once per run: the keys done under [`BAR_DEFINITION`]
/// (and which of them closed with refusals), the keys begun and not
/// finished, the keys closed `incomplete` or `refused`, the keys whose last
/// `done` line names another definition (D-2821, D-2822), and the run ids its
/// `run` lines hold (D-3317).
#[derive(Debug, Default)]
struct Journal {
    path: PathBuf,
    done: HashSet<String>,
    open: HashSet<String>,
    incomplete: HashSet<String>,
    restated: HashSet<String>,
    /// The done keys whose last `done` line carries `vendor_refused=` above
    /// zero: read again only under [`RunOptions::retry_refused`].
    with_refusals: HashSet<String>,
    /// Every id a `run` line holds, so a new run takes one no line holds.
    runs: HashSet<u64>,
    /// The keys whose last line is `refused`, each with that line less its
    /// `run=` field: a day refused again for the same stage, reason and count
    /// is not written again ([`Ctx::close`]), so a re-run over a special
    /// session or a day the source lacks writes nothing (D-2803).
    refused_close: HashMap<String, String>,
    /// The journal's length in bytes and in whole lines as this run last
    /// knew them, so the line a `begin` lands on can be named in a
    /// provenance record (D-3334's `journal_ordinal`). A journal another
    /// process appended to since is counted again before the next line.
    known: std::cell::Cell<(u64, u64)>,
    /// The `<kind> <day>` of every key in `open`, whatever its filter: the
    /// days an earlier run began and never closed (it was killed, or its day
    /// could not be made durable), so what it wrote of them was never made
    /// durable (WP-16, D-3353; [`Journal::left_open`]).
    open_days: HashSet<String>,
    /// The `scope=` of each key's last `done` line (WP-12f, D-4878): the
    /// digest of the members that close was asked to read. A key whose last
    /// `done` carries none was closed before scopes were recorded.
    scope_done: HashMap<String, String>,
    /// The members a base key (`<kind> <day> <filter>`) has keys of their own
    /// for (`<kind> <day> <filter>+<member>`), whatever closed them: the
    /// members a wider scope reopened that day (D-4878).
    member_keys: HashMap<String, std::collections::BTreeSet<String>>,
    /// Per `<kind> <day>`, the filters whose base key is closed `done`, in
    /// order (D-4878): every close a later run may ask what it owes
    /// ([`reach_back`]). Built once at the load's end, O(done keys); one
    /// hash probe to read.
    closed_by: HashMap<String, Vec<String>>,
}

impl Journal {
    /// Reads the journal at `path`; a missing one is empty. A torn last line
    /// (no newline: a crash mid-write) is ignored and closed before the next
    /// write. The lines are those of the module doc (D-3317): `run`, `begin`,
    /// `done`, `incomplete`, `refused` and `recounted`; the last closing line
    /// of a key wins. Any other line, or a `definition=`, `run=`,
    /// `vendor_refused=` or `months=` that is not one count as this journal
    /// writes it, refuses the run (D-2814).
    fn load(path: PathBuf) -> Result<Self, ImportRefusal> {
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(why) if why.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(why) => {
                return Err(ImportRefusal::Journal {
                    why: format!("{}: {why}", path.display()),
                });
            }
        };
        let mut journal = Self {
            path,
            ..Self::default()
        };
        journal.known.set((
            u64::try_from(text.len()).unwrap_or(u64::MAX),
            newlines(text.as_bytes()),
        ));
        let (whole, tail) = text.rsplit_once('\n').unwrap_or(("", text.as_str()));
        let shown = journal.path.display().to_string();
        let foreign = |line: &str| ImportRefusal::Journal {
            why: format!("{shown}: not a journal line: {line:?}"),
        };
        // Every whole line is judged BEFORE anything is written, and a torn
        // tail must be the start of a line the journal writes: a foreign file
        // is refused on every run and never touched. Closing first let a
        // one-line foreign file be refused once and then, its line now
        // "torn", read as an empty journal by every later run (D-2814).
        if !tail.is_empty() && !torn_fragment(tail) {
            return Err(foreign(tail));
        }
        for line in whole.split('\n').filter(|line| !line.is_empty()) {
            if let Some(fragment) = line.strip_suffix(TORN) {
                if torn_fragment(fragment) {
                    continue;
                }
                return Err(foreign(line));
            }
            let fields: Vec<&str> = line.split(' ').collect();
            journal
                .read_line(fields.as_slice())
                .map_err(|()| foreign(line))?;
        }
        if !tail.is_empty() {
            // Closed with a mark, never a bare newline: a bare one made the
            // torn fragment a whole line that the NEXT load refused as
            // foreign, so one crash mid-write stopped every later run
            // (D-2814).
            journal.write(None)?;
        }
        // Once, O(open keys): the days left open, whatever filter began them.
        journal.open_days = journal
            .open
            .iter()
            .filter_map(|key| key.rsplit_once(' ').map(|(day, _only)| day.to_owned()))
            .collect();
        // Once, O(done keys): which filters closed each day done. A member
        // key (`<filter>+<member>`) is its base key's and is not one.
        let mut closed_by: HashMap<String, Vec<String>> = HashMap::new();
        for key in &journal.done {
            if let Some((day, only)) = key.rsplit_once(' ')
                && !only.contains('+')
            {
                closed_by
                    .entry(day.to_owned())
                    .or_default()
                    .push(only.to_owned());
            }
        }
        for filters in closed_by.values_mut() {
            filters.sort_unstable();
        }
        journal.closed_by = closed_by;
        Ok(journal)
    }

    /// Whether an earlier run began `day` of `kind`, under any filter, and
    /// never closed it (WP-16, D-3353). Such a run was killed, or stopped at
    /// a day it could not make durable, after it had written some of the
    /// day's months: read again, those months answer "already present" and
    /// nothing is written, so this run must make them durable itself before
    /// it closes the day ([`store::file::DurableBatch::confirm_month`]). One
    /// hash probe.
    fn left_open(&self, kind: ImportKind, day: Day) -> bool {
        self.open_days.contains(&format!("{} {day}", kind.as_str()))
    }

    /// Folds one whole line into the journal's sets, or `Err(())` for a line
    /// this journal does not write. O(fields).
    fn read_line(&mut self, fields: &[&str]) -> Result<(), ()> {
        let key = |kind: &str, day: &str, only: &str| format!("{kind} {day} {only}");
        // A member key names its base key and its member (D-4878); `+` is
        // outside the symbol alphabet, so no filter name holds one.
        if let [_, kind, day, only, ..] = fields
            && let Some((base, member)) = only.split_once('+')
        {
            self.member_keys
                .entry(key(kind, day, base))
                .or_default()
                .insert(member.to_owned());
        }
        match fields {
            ["run", text, rest @ ..] => {
                let id: u64 = text.parse().map_err(|_| ())?;
                if id.to_string() != *text || rest.iter().any(|field| !field.contains('=')) {
                    return Err(());
                }
                self.runs.insert(id);
            }
            ["begin", kind, day, only] => {
                let key = key(kind, day, only);
                self.refused_close.remove(&key);
                self.open.insert(key);
            }
            ["done", kind, day, only, stats @ ..] => {
                let definition = built_under(stats)?;
                run_of(stats)?;
                let refused = count_of(stats, "vendor_refused")?.unwrap_or(0);
                let key = key(kind, day, only);
                self.refused_close.remove(&key);
                self.open.remove(&key);
                self.incomplete.remove(&key);
                // The LAST `done` of a key says which definition built what
                // the store holds (D-2822); a capital-market day built under
                // 2 holds the bars 3 builds (D-3342, [`definition_is_current`]).
                // The LAST `done` of a key also says which members it was
                // asked to read (D-4878); one that says nothing is a close
                // from before scopes were recorded.
                match stats.iter().find_map(|field| field.strip_prefix("scope=")) {
                    Some(digest) => self.scope_done.insert(key.clone(), digest.to_owned()),
                    None => self.scope_done.remove(&key),
                };
                if definition.is_some_and(|definition| current_for(kind, definition)) {
                    self.restated.remove(&key);
                    if refused > 0 {
                        self.with_refusals.insert(key.clone());
                    } else {
                        self.with_refusals.remove(&key);
                    }
                    self.done.insert(key);
                } else {
                    self.done.remove(&key);
                    self.with_refusals.remove(&key);
                    self.restated.insert(key);
                }
            }
            // Not done, whatever closed the key before: the last line wins
            // (D-3317). A `refused` day wrote nothing this time; each is
            // imported again and named in `resumed`.
            [
                verb @ ("incomplete" | "refused"),
                kind,
                day,
                only,
                stats @ ..,
            ] => {
                definition_of(stats)?;
                run_of(stats)?;
                let key = key(kind, day, only);
                if *verb == "refused" {
                    let unrun: Vec<&str> = fields
                        .iter()
                        .copied()
                        .filter(|field| !field.starts_with("run="))
                        .collect();
                    self.refused_close.insert(key.clone(), unrun.join(" "));
                } else {
                    self.refused_close.remove(&key);
                }
                self.open.remove(&key);
                self.done.remove(&key);
                self.with_refusals.remove(&key);
                self.incomplete.insert(key);
            }
            // A census-refused day the census now counts (D-3316's recount):
            // done, as a `done` line under its definition is.
            ["recounted", kind, day, only, stats @ ..] => {
                let definition = definition_of(stats)?;
                count_of(stats, "months")?;
                run_of(stats)?;
                let key = key(kind, day, only);
                self.refused_close.remove(&key);
                self.open.remove(&key);
                self.incomplete.remove(&key);
                self.with_refusals.remove(&key);
                if definition.is_some_and(|definition| current_for(kind, definition)) {
                    self.restated.remove(&key);
                    self.done.insert(key);
                } else {
                    self.done.remove(&key);
                    self.restated.insert(key);
                }
            }
            _ => return Err(()),
        }
        Ok(())
    }

    /// THE RUN'S ID (D-3317 item 4): its start, raised past every id a `run`
    /// line holds, so two runs started in one second are two runs. At most
    /// one step per `run` line in the journal.
    fn fresh_run(&self, started: u64) -> u64 {
        let mut id = started;
        while self.runs.contains(&id) {
            id = id.saturating_add(1);
        }
        id
    }

    /// Appends `line` and a newline, durably, and answers the line's number
    /// in the file, from 1.
    fn append(&self, line: &str) -> Result<u64, ImportRefusal> {
        self.write(Some(line))?;
        Ok(self.known.get().1)
    }

    /// Appends `line` and a newline, durably, after closing a torn last line
    /// with [`TORN`]. A write that failed part way earlier IN THIS RUN left
    /// such a line, and the run went on: written straight after it, the next
    /// line was glued to it, a foreign line every later load refused or a
    /// line that hid the one glued on (D-2826). So the file's last byte is
    /// read first, one seek and one byte, and a line that does not end in a
    /// newline is closed exactly as a load closes one. `None` only closes.
    fn write(&self, line: Option<&str>) -> Result<(), ImportRefusal> {
        use std::io::{Read as _, Seek as _};
        let refused = |why: std::io::Error| ImportRefusal::Journal {
            why: format!("{}: {why}", self.path.display()),
        };
        // The journal always sits in `imports/` under the store root.
        std::fs::create_dir_all(self.path.with_file_name("")).map_err(refused)?;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .open(&self.path)
            .map_err(refused)?;
        let mut text = String::new();
        let at = file.metadata().map_err(refused)?.len();
        if at != self.known.get().0 {
            // Another process appended since this run last wrote: its lines
            // are counted once, so the next line's number is the file's.
            let mut bytes = Vec::new();
            file.seek(std::io::SeekFrom::Start(0)).map_err(refused)?;
            file.read_to_end(&mut bytes).map_err(refused)?;
            self.known.set((
                u64::try_from(bytes.len()).unwrap_or(u64::MAX),
                newlines(&bytes),
            ));
        }
        if at > 0 {
            let mut last = [0_u8; 1];
            file.seek(std::io::SeekFrom::End(-1)).map_err(refused)?;
            file.read_exact(&mut last).map_err(refused)?;
            if last != *b"\n" {
                let shown = self.path.display().to_string();
                let _dropped_when_filtered = telemetry::emit(
                    &telemetry::Event::warn(TARGET, "closing a torn journal line")
                        .with("journal", shown.as_str()),
                );
                text.push_str(TORN);
                text.push('\n');
            }
        }
        if let Some(line) = line {
            text.push_str(line);
            text.push('\n');
        }
        if text.is_empty() {
            return Ok(());
        }
        file.write_all(text.as_bytes()).map_err(refused)?;
        file.sync_all().map_err(refused)?;
        let (bytes, lines) = self.known.get();
        self.known.set((
            bytes.saturating_add(u64::try_from(text.len()).unwrap_or(u64::MAX)),
            lines.saturating_add(newlines(text.as_bytes())),
        ));
        Ok(())
    }
}

/// The newlines in `bytes`: the whole lines of a journal.
fn newlines(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0_u64, |lines, &b| {
        lines.saturating_add(u64::from(b == b'\n'))
    })
}

/// Appends `lines`, each with a newline, to the ledger at `path`, durably:
/// one write and one `fsync`, after closing a last line a crash left torn
/// with a bare newline (that fragment is then one line a reader counts as a
/// fault, never glued to the next). Creates the file and its directory.
/// Nothing to append opens nothing. Shared by the day, lineage and lot
/// ledgers (D-3335, D-3331, D-3343).
///
/// # Errors
///
/// The path and why.
pub(crate) fn append_lines(path: &Path, lines: &[String]) -> Result<(), String> {
    use std::io::{Read as _, Seek as _};
    if lines.is_empty() {
        return Ok(());
    }
    let refused = |why: std::io::Error| format!("{}: {why}", path.display());
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(refused)?;
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .append(true)
        .open(path)
        .map_err(refused)?;
    let mut text = String::new();
    if file.metadata().map_err(refused)?.len() > 0 {
        let mut last = [0_u8; 1];
        file.seek(std::io::SeekFrom::End(-1)).map_err(refused)?;
        file.read_exact(&mut last).map_err(refused)?;
        if last != *b"\n" {
            text.push('\n');
        }
    }
    for line in lines {
        text.push_str(line);
        text.push('\n');
    }
    file.write_all(text.as_bytes()).map_err(refused)?;
    file.sync_all().map_err(refused)
}

/// Every day of `from..=to`.
fn days(from: Day, to: Day) -> Vec<Day> {
    (from.days_from_epoch()..=to.days_from_epoch())
        .filter_map(|n| Day::from_days(n).ok())
        .collect()
}

/// A failure about `about`, for `cause`, logged at `Error` with its stage and
/// code and returned for the report.
fn note_failure(kind: ImportKind, day: Day, about: &str, why: &str, cause: Cause) -> Failure {
    let day_text = day.to_string();
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::error(TARGET, "refused")
            .with("kind", kind.as_str())
            .with("day", day_text.as_str())
            .with("about", about)
            .with("why", why)
            .with("stage", cause.stage.as_str())
            .with("code", cause.code),
    );
    failure_on(day, &format!("{day} {about}"), why.to_owned())
}

/// One failure of a day and its cause (D-3317).
#[derive(Debug, Clone)]
struct Staged {
    cause: Cause,
    failure: Failure,
}

/// A day's failure about `about`, refused by `refusal` (an import refusal
/// that names its own code), at `stage`, logged.
fn staged(run: &Run<'_>, day: Day, about: &str, stage: Stage, refusal: &ImportRefusal) -> Staged {
    let cause = Cause::open(stage, refusal.code());
    Staged {
        cause,
        failure: note_failure(run.kind, day, about, &refusal.to_string(), cause),
    }
}

/// How a day closes, with the cause its line names (D-3317 items 2, 3, 7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Close {
    /// `done`, no failure.
    Done,
    /// `done ... vendor_refused=<n> reason=<code>`.
    WithRefusals(&'static str),
    /// `incomplete ... stage= reason=`.
    Incomplete(Cause),
    /// `refused ... stage= reason=`.
    Refused(Cause),
}

impl Close {
    /// The outcome a reader names this close by.
    const fn outcome(self) -> Outcome {
        match self {
            Self::Done => Outcome::Done,
            Self::WithRefusals(_) => Outcome::DoneWithRefusals,
            Self::Incomplete(_) => Outcome::Incomplete,
            Self::Refused(_) => Outcome::Refused,
        }
    }
}

/// The day's decisive failure: the census's, else the first that keeps the
/// day from closing, else the first (D-3317: its `stage=` and `reason=`).
fn decisive(failures: &[Staged]) -> Option<&Staged> {
    failures
        .iter()
        .find(|one| one.cause.stage == Stage::Census)
        .or_else(|| failures.iter().find(|one| !one.cause.closes))
        .or_else(|| failures.first())
}

/// How a day with `failures` closes, `read` when at least one of its files
/// was read clean: no failure is done; a census failure is incomplete; a day
/// none of whose files was read clean (every wanted file refused, or the
/// listing) is refused (D-3317 item 3); a day whose every failure closes is
/// done with refusals (item 2); any other is incomplete. A day whose clean
/// files held no traded second wrote nothing and still read them: it is not
/// "every file refused", so it closes by its failures like any other day.
/// O(failures).
fn close_of(failures: &[Staged], read: bool) -> Close {
    let Some(first) = decisive(failures) else {
        return Close::Done;
    };
    let cause = first.cause;
    if cause.stage == Stage::Census {
        Close::Incomplete(cause)
    } else if !read {
        Close::Refused(cause)
    } else if cause.closes {
        // The decisive failure is the first that does not close, when there
        // is one: this one closing means every one does.
        Close::WithRefusals(cause.code)
    } else {
        Close::Incomplete(cause)
    }
}

/// The counts of one closing line, the D-2822 fields in their order.
#[derive(Debug, Clone, Copy, Default)]
struct Counts {
    files: usize,
    seconds: usize,
    failures: usize,
    late: usize,
    late_unresolved: usize,
    max_back_s: u32,
}

/// The report's counters at a day's start, so the day's own counts are a
/// subtraction (the design's seam 3, WP-16): a `Copy` of seven numbers where
/// this was a clone of the whole report, its failures and day lists
/// included, once per day.
#[derive(Debug, Clone, Copy)]
struct Mark {
    files: usize,
    rows: usize,
    seconds: usize,
    seconds_committed: usize,
    late_rows: usize,
    late_unresolved: usize,
    failures: usize,
}

impl Mark {
    /// `report`'s counters now.
    const fn of(report: &Report) -> Self {
        Self {
            files: report.files,
            rows: report.rows,
            seconds: report.seconds,
            seconds_committed: report.seconds_committed,
            late_rows: report.late_rows,
            late_unresolved: report.late_unresolved,
            failures: report.failures.len(),
        }
    }
}

impl Counts {
    /// The day's counts: what `report` holds beyond `before`.
    const fn of_day(before: Mark, report: &Report, failures: usize, max_back_s: u32) -> Self {
        Self {
            files: report.files - before.files,
            seconds: report.seconds - before.seconds,
            failures,
            late: report.late_rows - before.late_rows,
            late_unresolved: report.late_unresolved - before.late_unresolved,
            max_back_s,
        }
    }

    /// The D-2822 stats, as `done` and `incomplete` write them.
    fn text(&self) -> String {
        format!(
            "definition={BAR_DEFINITION} files={} seconds={} failures={} late={} \
             late_unresolved={} max_back_s={}",
            self.files,
            self.seconds,
            self.failures,
            self.late,
            self.late_unresolved,
            self.max_back_s,
        )
    }
}

/// A day's closing line, all but its `run=` (D-3317 items 2 and 3). `scope`
/// is the ` scope=<digest>` a `done` or `incomplete` line of a recorded run
/// carries after its counts (D-4878), or nothing; a `refused` line carries
/// none.
fn closing_line(key: &str, close: Close, counts: &Counts, scope: &str) -> String {
    match close {
        Close::Done => format!("done {key} {}{scope}", counts.text()),
        Close::WithRefusals(code) => format!(
            "done {key} {}{scope} vendor_refused={} reason={code}",
            counts.text(),
            counts.failures
        ),
        Close::Incomplete(cause) => format!(
            "incomplete {key} {}{scope} stage={} reason={}",
            counts.text(),
            cause.stage.as_str(),
            cause.code
        ),
        Close::Refused(cause) => format!(
            "refused {key} definition={BAR_DEFINITION} stage={} reason={} failures={}",
            cause.stage.as_str(),
            cause.code,
            counts.failures
        ),
    }
}

/// The run's `run` line (D-3317 item 4): its source as the caller measured
/// it, or `unrecorded` for every part a caller did not hand on.
fn run_line(id: u64, started: u64, origin: Option<&Origin>) -> String {
    let Some(origin) = origin else {
        return format!(
            "run {id} source=unrecorded archive=unrecorded archive_bytes=unrecorded \
             commit=unrecorded started={started}"
        );
    };
    format!(
        "run {id} source={} archive={} archive_bytes={} commit={} started={started} {}={}",
        origin.source.as_str(),
        encoded(origin.path.as_os_str().as_encoded_bytes()),
        origin.archive_bytes,
        origin.commit_text(),
        origin.source.digest_field(),
        origin.digest_hex(),
    )
}

/// What every day of one run shares: the run, its journal, its options and
/// its `run` line, written before the run's first other line (D-3317 item 4).
struct Ctx<'a> {
    run: &'a Run<'a>,
    journal: &'a Journal,
    options: RunOptions<'a>,
    /// The run's id: `started`, raised past every id the journal holds.
    id: u64,
    started: u64,
    /// Whether the `run` line is in the journal.
    announced: std::cell::Cell<bool>,
    /// The scope this run records with every close and what a done day owes
    /// it (WP-12f, D-4878); `None` for a run whose members are not recorded
    /// (an options run, a test's bare reader).
    scoped: Option<Scoped<'a>>,
}

/// A run's recorded scope beside the scope ledger (WP-12f, D-4878).
struct Scoped<'a> {
    /// What this run is asked to read.
    record: &'a scope::Record,
    /// The swept surface of the run's kind: what a close with no `scope=`
    /// is read as built under, the scope every build filed at least.
    swept: std::sync::Arc<HashSet<String>>,
    /// The scope ledger, read at the run's start.
    ledger: std::cell::RefCell<scope::Ledger>,
    /// Per scope a done close names (`None`: it names none), the members
    /// this run's scope holds beyond it, computed once per run.
    beyond: std::cell::RefCell<HashMap<Option<String>, std::sync::Arc<HashSet<String>>>>,
}

/// What a day closed `done` still owes this run's scope (D-4878).
enum Owed {
    /// Nothing: its close covered every member, or the day was examined
    /// under this scope and every member key it opened is done.
    Nothing,
    /// The members whose own keys the day already has and are not done.
    Pending(Vec<String>),
    /// The members its close never covered: the day's listing is read once
    /// to find the ones that have a file.
    Probe(std::sync::Arc<HashSet<String>>),
}

impl Ctx<'_> {
    /// What `key`'s day, closed `done`, owes this run's scope: O(1) for a
    /// day that owes nothing (one compare of digests, two hash probes), and
    /// the members beyond a close's scope are computed once per scope.
    ///
    /// `remembered` is the key the scope ledger holds the examination under:
    /// `key` itself for the run's own key. For a key another filter closed,
    /// what it owes is the same, and whether this run's scope has examined
    /// it for this run's filter is remembered under a key of that pair
    /// ([`Asked::remembered`]), never under the key itself, whose own
    /// examination is its own filter's to write.
    fn owed(&self, key: &str, remembered: &str) -> Owed {
        let Some(scoped) = &self.scoped else {
            return Owed::Nothing;
        };
        let retried = self.options.retry_refused && self.journal.with_refusals.contains(key);
        if !self.journal.done.contains(key) || retried {
            return Owed::Nothing;
        }
        let built = self.journal.scope_done.get(key);
        let now = scoped.record.digest();
        if built.is_some_and(|digest| digest == now) {
            return Owed::Nothing;
        }
        let done = |member: &String| self.settled(&format!("{key}+{member}"));
        let ledger = scoped.ledger.borrow();
        if ledger.examined_under(remembered) == Some(now) {
            let pending: Vec<String> = self
                .journal
                .member_keys
                .get(key)
                .into_iter()
                .flatten()
                .filter(|member| scoped.record.set().contains(*member) && !done(member))
                .cloned()
                .collect();
            return if pending.is_empty() {
                Owed::Nothing
            } else {
                Owed::Pending(pending)
            };
        }
        let beyond = std::sync::Arc::clone(
            scoped
                .beyond
                .borrow_mut()
                .entry(built.cloned())
                .or_insert_with(|| {
                    let old = built.and_then(|digest| ledger.members_of(digest));
                    if built.is_some() && old.is_none() {
                        let _dropped_when_filtered = telemetry::emit(
                            &telemetry::Event::warn(
                                TARGET,
                                "a close names a scope the scope ledger does not hold; \
                                 it is read as the swept surface",
                            )
                            .with("kind", self.run.kind.as_str())
                            .with("code", "scope_unrecorded"),
                        );
                    }
                    let old = old.unwrap_or(&scoped.swept);
                    std::sync::Arc::new(
                        scoped
                            .record
                            .members()
                            .iter()
                            .filter(|member| !old.contains(*member))
                            .cloned()
                            .collect(),
                    )
                }),
        );
        if beyond.is_empty() {
            Owed::Nothing
        } else {
            Owed::Probe(beyond)
        }
    }

    /// Whether a member key needs no read by this run: closed `done`, and not
    /// one closed with refusals that the run was asked to read again
    /// ([`RunOptions::retry_refused`]), as a base key is in [`admit`]. Two
    /// hash probes.
    fn settled(&self, member_key: &str) -> bool {
        let retried = self.options.retry_refused && self.journal.with_refusals.contains(member_key);
        self.journal.done.contains(member_key) && !retried
    }

    /// Writes the run's `run` line when no line of this run is in the
    /// journal yet (D-3317 item 4): before its first other line, and before
    /// the scope ledger names the run's id (D-4878), so no id is written
    /// anywhere that the journal does not hold and the next run cannot take.
    fn announce(&self) -> Result<(), ImportRefusal> {
        if !self.announced.get() {
            self.journal
                .append(&run_line(self.id, self.started, self.options.origin))?;
            self.announced.set(true);
        }
        Ok(())
    }

    /// The `scope=` field a closing line of `key` carries: this run's digest
    /// on a base key's `done` or `incomplete`, after its member list is in
    /// the scope ledger. A member key names its one member and carries none.
    fn scope_field(&self, key: &str, close: Close) -> Result<String, ImportRefusal> {
        let Some(scoped) = &self.scoped else {
            return Ok(String::new());
        };
        if matches!(close, Close::Refused(_)) || key.contains('+') {
            return Ok(String::new());
        }
        scoped
            .ledger
            .borrow_mut()
            .record(self.run.kind.as_str(), scoped.record)
            .map_err(|why| ImportRefusal::Journal { why })?;
        Ok(format!(" scope={}", scoped.record.digest()))
    }

    /// Appends `line`, after the run's `run` line when it is the run's first,
    /// and answers its line number in the journal, from 1.
    fn append(&self, line: &str) -> Result<u64, ImportRefusal> {
        self.announce()?;
        self.journal.append(line)
    }

    /// Writes a day's one closing line and says how the day closed. A journal
    /// that will not take it is one more named failure of the day, in
    /// `failures`, and the day closed nowhere: `incomplete`, imported again by
    /// the next run.
    ///
    /// A `refused` close identical, but for its `run=`, to the key's last line
    /// is not written again: the journal already says exactly that, and a
    /// re-run over a day the calendar or the source refuses for good (a
    /// special session, a day the vendor never published) then writes
    /// nothing, as D-2803 asks of a re-run that changes nothing.
    fn close(
        &self,
        (key, day): (&str, Day),
        close: Close,
        counts: &Counts,
        failures: &mut Vec<Staged>,
    ) -> Outcome {
        // The scope's member list is in the scope ledger before the first
        // line that names it (D-4878); a ledger that will not take it leaves
        // the day closed nowhere, as a journal that will not take its line.
        let scope = match self.scope_field(key, close) {
            Ok(scope) => scope,
            Err(why) => {
                failures.push(staged(self.run, day, "scope ledger", Stage::Journal, &why));
                return Outcome::Incomplete;
            }
        };
        let unrun = closing_line(key, close, counts, &scope);
        if matches!(close, Close::Refused(_)) && self.journal.refused_close.get(key) == Some(&unrun)
        {
            return close.outcome();
        }
        match self.append(&format!("{unrun} run={}", self.id)) {
            Ok(_) => close.outcome(),
            Err(why) => {
                failures.push(staged(self.run, day, "journal", Stage::Journal, &why));
                Outcome::Incomplete
            }
        }
    }
}

/// The census a run publishes each day's batch through (D-3317). The store's
/// own is [`StoreCensus`]; a test hands a double, because a census at the
/// entry ceiling is a 268 MB file and not a thing a unit test builds (the
/// reason `pull::unit`'s ceiling checks give for their own arithmetic).
trait Census {
    /// Publishes `held`, or says why not, as [`record_held`] does: all or
    /// nothing.
    fn publish(&mut self, store_root: &Path, held: &[Held]) -> Option<String>;

    /// The census's room for `held`, measured from its file.
    fn room(&self, store_root: &Path, held: &[Held]) -> Result<CensusRoom, String>;
}

/// The store's census of the `gdfl` feed.
struct StoreCensus;

impl Census for StoreCensus {
    fn publish(&mut self, store_root: &Path, held: &[Held]) -> Option<String> {
        record_held(store_root, Vendor::Gdfl, held)
    }

    fn room(&self, store_root: &Path, held: &[Held]) -> Result<CensusRoom, String> {
        census_room(store_root, held)
    }
}

/// The largest census file this build can have written: the entry ceiling at
/// the widest stride it writes, after the header region. Derived, not chosen.
const CENSUS_CAP: u64 = HEADER_LEN + MAX_ENTRIES * ENTRY_STRIDE;

/// The `gdfl` census's room for `held`, from its version-3 file (D-3317;
/// DB-G10).
///
/// One capped read and one [`Manifest::open_image`] of the census, Θ(entries)
/// and on the refusal path only: the run stops after it. `new` counts the
/// batch's entries the census does not hold exactly as offered, by the same
/// comparison the census's own count makes, so it is at most the appends the
/// batch needs. An absent census is an empty one, as the census reads it.
///
/// A store that holds D-3316's version-4 census (`manifest/v4/<vendor>/`,
/// WP-10) is not measured here: its writer refuses up front in its own words,
/// naming the file's `n_valid`, the batch and the free slots, and the frozen
/// version-3 file beside it would give numbers that are no longer the
/// census's. Its room is then `Err`, naming why, and the refusal is
/// [`ImportRefusal::CensusRefused`] carrying those words.
///
/// # Errors
///
/// The path and why, for a census that cannot be read or does not load, or a
/// version-4 census.
fn census_room(store_root: &Path, held: &[Held]) -> Result<CensusRoom, String> {
    let path = manifest_path(store_root, Vendor::Gdfl);
    let v4 = path
        .parent()
        .map(|dir| dir.join("v4").join(Vendor::Gdfl.as_str()));
    if let Some(v4) = v4.filter(|dir| dir.exists()) {
        return Err(format!(
            "{} holds the version-4 census (D-3316); its own refusal names its room",
            v4.display()
        ));
    }
    let bytes = match crate::ingest::read_regular_capped(&path, CENSUS_CAP) {
        Ok(Ok(bytes)) => bytes,
        Ok(Err(why)) => return Err(format!("{} {why}", path.display())),
        Err(absent) if absent.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(why) => return Err(format!("{}: {why}", path.display())),
    };
    let census = Manifest::open_image(Vendor::Gdfl, &bytes)
        .map_err(|why| format!("{}: {why}", path.display()))?;
    let new = held
        .iter()
        .filter(|one| census.held(&one.entry.key) != Some(**one))
        .count();
    Ok(CensusRoom {
        n_valid: census.entries(),
        limit: MAX_ENTRIES,
        batch: u64::try_from(held.len()).unwrap_or(u64::MAX),
        new: u64::try_from(new).unwrap_or(u64::MAX),
    })
}

/// The code of the census refusal a `room` makes (D-3317 items 6 and 7): the
/// census's own `too_many_entries` when the room is measured and the batch
/// does not fit it, `census_refused` otherwise. [`census_refusal`] chooses its
/// variant by the same test, so the journal's `reason=` and the refusal's
/// [`ImportRefusal::code`] agree.
const fn census_code(room: &Result<CensusRoom, String>) -> &'static str {
    match room {
        Ok(room) if !room.fits() => "too_many_entries",
        _ => "census_refused",
    }
}

/// The census refusal of the day, from the census's words and its measured
/// room: full when the room is measured and the batch does not fit, refused
/// otherwise (D-3317 item 7).
fn census_refusal(
    (kind, day): (ImportKind, Day),
    why: String,
    room: Result<CensusRoom, String>,
    done: Report,
) -> ImportRefusal {
    match room {
        Ok(room) if !room.fits() => ImportRefusal::CensusFull {
            kind,
            day,
            room,
            why,
            done: Box::new(done),
        },
        room => ImportRefusal::CensusRefused {
            kind,
            day,
            room,
            why,
            done: Box::new(done),
        },
    }
}

/// The plan one rung of one file is filed under.
fn plan(kind: ImportKind, request: &BarRequest, contract: Option<Contract>) -> Plan<'_> {
    Plan {
        calendar: crate::calendar::Runtime::default(),
        cash_schedule: None,
        // Unused by `from_rows`, which takes bars already decoded; named for
        // the record. GDFL bytes never pass `csv::decode` (D-2802).
        columns: crate::csv::Columns::Gdfl,
        request,
        encoding: TimestampEncoding::EpochSecondsUtc,
        scale: PriceScale::Paisa,
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment: kind.segment(),
        contract,
    }
}

/// One file of a day as a source's listing names it (D-3334): the entry,
/// its uncompressed size and its CRC-32 as the archive's directory (or the
/// tick store's index, which copies it) states them.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Member {
    entry: String,
    bytes: u64,
    crc32: u32,
}

/// What a source hands on beside a file's rows: its member, for the
/// provenance record, and its reader's own counts, for the day ledger.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct FileFacts {
    member: Option<Member>,
    reader: quality::ReaderCounts,
}

/// One file as the runtime receives it.
#[derive(Debug, Clone)]
struct Handed {
    file: TickFile,
    facts: FileFacts,
}

/// What a source says about a day beyond its files: the day zip's size, the
/// lineage it wrote, and the failures that are no file's (a lineage ledger
/// that would not take its lines).
#[derive(Debug, Default)]
struct DayExtras {
    member_bytes: Option<u64>,
    lineage: lineage::Written,
    failures: Vec<Staged>,
    /// The members a probe asked for that the day's listing holds a file
    /// of, in listing order ([`Ask::Probe`]; WP-12f, D-4878).
    present: Vec<String>,
}

/// The symbols a filter names, as a set.
type Names = std::sync::Arc<HashSet<String>>;

/// Which files of a day a reader is asked for (WP-12f, D-4878).
#[derive(Debug, Clone, Default)]
enum Ask {
    /// Every file the run's scope and filter want: an ordinary day.
    #[default]
    All,
    /// None: only which of these members the day's listing holds a file of,
    /// answered in [`DayExtras::present`]. Nothing is read or filed. The
    /// second is the names of the filter that closed the day, when it is not
    /// the run's own and names any ([`Asked::names`]).
    Probe(std::sync::Arc<HashSet<String>>, Option<Names>),
    /// The files of this one member, under the member's own key.
    One(String),
}

impl DayExtras {
    /// Takes in what `early` gathered before the day's listing (the
    /// lineage's warm-up): its records counted, its failures first, as they
    /// arose before this day's.
    fn absorb(&mut self, early: Self) {
        self.lineage.absorb(early.lineage);
        let later = std::mem::replace(&mut self.failures, early.failures);
        self.failures.extend(later);
    }
}

/// Where one day's files came from, for their provenance records (D-3334).
#[derive(Debug, Clone, PartialEq, Eq)]
enum DaySource {
    /// Members of a zip: every record's name is this prefix (the archives
    /// that hold the member, outermost first, each followed by `!`) and the
    /// member's entry.
    Members {
        /// The prefix.
        prefix: String,
    },
    /// One tick-store day file: its path below the tick store's root, and
    /// its length.
    TickstoreDay {
        /// The path below the root.
        name: String,
        /// Its length in bytes.
        bytes: u64,
    },
}

/// The capital-market tree of a spot kind.
const fn cm_kind(kind: ImportKind) -> CmKind {
    match kind {
        ImportKind::Stocks => CmKind::Stocks,
        ImportKind::Indices | ImportKind::Options => CmKind::Indices,
    }
}

/// Where `day`'s files of `kind` came from, as the run's measured `origin`
/// names its source (D-3334's naming, `docs/02-store-format.md` §8.3): a
/// zip member by `<archive>!<day zip>!<entry>`, a tick-store day by its path
/// below the root. One `stat` for a tick-store day; nothing else is read.
///
/// # Errors
///
/// The tick-store day file's path and why it cannot be measured.
fn day_source(kind: ImportKind, day: Day, origin: &Origin) -> Result<DaySource, String> {
    let month = crate::gdfl_archive::MONTHS
        .get(usize::from(day.month()).saturating_sub(1))
        .copied()
        .unwrap_or_default();
    match (origin.source, kind) {
        (SourceKind::Zip, ImportKind::Options) => Ok(DaySource::Members {
            prefix: format!(
                "{}!{month}_{:04}/{}.zip!",
                origin.path.join(format!("{:04}.zip", day.year())).display(),
                day.year(),
                crate::gdfl_nfo::day_folder_name(day)
            ),
        }),
        (SourceKind::Zip, _) => Ok(DaySource::Members {
            prefix: format!(
                "{}!{}!",
                origin.path.display(),
                crate::gdfl_archive::day_zip_name(cm_kind(kind), day)
            ),
        }),
        (SourceKind::TickStore, _) => {
            let below = Path::new("");
            let rel = if kind == ImportKind::Options {
                crate::gdfl_nfo::NfoTickStore::new(below).day_path(day)
            } else {
                crate::gdfl_tickstore::TickStore::new(below).day_path(cm_kind(kind), day)
            };
            let full = origin.path.join(&rel);
            let bytes = std::fs::metadata(&full)
                .map_err(|why| format!("{}: {why}", full.display()))?
                .len();
            Ok(DaySource::TickstoreDay {
                name: rel.display().to_string(),
                bytes,
            })
        }
    }
}

/// The store's symbol id of `symbol`: the cross-check its bar header
/// carries, derived exactly as `ingest` derives it.
fn symbol_id_of(symbol: &str) -> u32 {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the store's own fold of the name into its 32-bit cross-check: \
                  the low half of the 64-bit FNV-1a, exactly as `ingest` takes it"
    )]
    let id = brutex_core::universe::fnv1a(symbol) as u32;
    id
}

/// The UTC microsecond now; 0 before 1970, which a record refuses by name.
fn now_micros() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| {
            i64::try_from(since.as_micros()).unwrap_or(i64::MAX)
        })
}

/// The bar file of the month `held` names, in the GDFL tree.
///
/// # Errors
///
/// The store's words for a key that is not a path.
fn month_path(held: &Held) -> Result<store::path::StorePath<'_>, String> {
    let key = &held.entry.key;
    store::path::StorePath::new(store::path::PathParts {
        vendor: Vendor::Gdfl,
        exchange: key.exchange.as_str(),
        segment: key.segment.as_str(),
        symbol: key.symbol.as_str(),
        contract: key.contract,
        timeframe: key.timeframe,
        month: key.month,
        file: store::path::FileKind::Bars,
    })
    .map_err(|why| why.to_string())
}

/// Names the month `held` names to `batch`, so every file it has and every
/// directory down to it is flushed for itself when the batch is made durable
/// (WP-16, D-3353; `store::file::DurableBatch::confirm_month`). `Ok(false)`
/// only for a batch already sealed, which the day's never is while it files.
///
/// # Errors
///
/// The path or the store's refusal, in its words.
fn confirm_month(
    store_root: &Path,
    held: &Held,
    batch: &store::file::DurableBatch,
) -> Result<bool, String> {
    batch
        .confirm_month(store_root, month_path(held)?)
        .map_err(|why| why.to_string())
}

/// Records one landing beside the month it appended to (D-3334): the month
/// `held` names, whose append committed its last `committed` bars. The bar
/// file is opened read-only, never written, and the record is refused unless
/// that file's commit counter is still `held`'s, so a record never names bars
/// another writer appended. `Ok(false)` when nothing was appended.
///
/// # Errors
///
/// The path or the store's refusal, in its words.
fn record_provenance(
    store_root: &Path,
    held: &Held,
    committed: usize,
    landing: &store::provenance::Landing<'_>,
) -> Result<bool, String> {
    let key = held.entry.key;
    let path = month_path(held)?;
    let bars =
        store::file::BarFile::open_existing(store_root, path, symbol_id_of(key.symbol.as_str()))
            .map_err(|why| why.to_string())?;
    let n_valid = held.entry.rows;
    let first_index = u64::try_from(committed)
        .ok()
        .and_then(|committed| n_valid.checked_sub(committed))
        .ok_or_else(|| format!("{committed} bars committed and {n_valid} held"))?;
    let appended = store::file::Appended::Committed {
        first_index,
        n_valid,
    };
    match store::provenance::record(store_root, &bars, path, appended, landing) {
        Ok(store::provenance::Recorded::Written(_)) => Ok(true),
        Ok(store::provenance::Recorded::NothingAppended) => Ok(false),
        Err(why) => Err(why.to_string()),
    }
}

// ═══ One day's files on W workers (WP-16, DB-N25) ═════════════════════════
//
// A day used to file its files one after another on one thread: an options
// day of 5,641 files took 94.5 s on the operator's drive for 36 s of CPU,
// most of the rest waiting on 46,373 flushes (2020-09-15 into a scratch
// store, measured by a counting build). The files of a day now run on
// [`Workers`]: each file is one job (read, verify and decode it; convert,
// fold and file its seconds; record its landing), taken in listing order by
// whichever worker is free, and what each job did comes back as a [`Filed`]
// that the coordinating thread folds into the day IN LISTING ORDER. So the
// report, its failures, the day ledger, the census batch and the lineage's
// contract-days are exactly what one worker builds.
//
// One contract-month per file: an option file is one contract, a stocks file
// one share, an index file one index, and a listing names each at most once
// (a second name is a skip or a refusal, D-2814), so every month file and
// every sidecar beside it is written by exactly one worker.
//
// ONE WRITER OF JOURNAL LINES (the design's seam 2). Workers never append to
// the journal. A day's `begin` is still written before its first write and
// only when the day has a second to write (D-2803): the first worker that
// has one asks the [`BeginGate`], the coordinating thread writes the line and
// answers its number, and every later file of the day reads it from the gate.

/// The day's `begin` line, written by the coordinating thread when the first
/// worker with a second to file asks for it (WP-16).
///
/// An attempt the journal refuses is answered to every worker that asked
/// before it finished, each of which names it as its own file's failure and
/// files nothing, exactly as a single worker did; the next worker that asks
/// makes a new attempt.
#[derive(Debug, Default)]
struct BeginGate {
    state: std::sync::Mutex<GateState>,
    changed: std::sync::Condvar,
}

/// What the [`BeginGate`] knows.
#[derive(Debug, Default)]
struct GateState {
    /// The `begin` line's number in the journal, once written.
    line: Option<u64>,
    /// A worker waits for an attempt.
    asked: bool,
    /// Attempts finished.
    attempts: u64,
    /// The last attempt's refusal.
    refused: Option<ImportRefusal>,
    /// Workers that have not finished.
    working: usize,
}

impl BeginGate {
    /// A gate for a day whose `begin` is `line` (written by an earlier batch
    /// of the same day), for `working` workers.
    fn new(line: Option<u64>, working: usize) -> Self {
        Self {
            state: std::sync::Mutex::new(GateState {
                line,
                working,
                ..GateState::default()
            }),
            changed: std::sync::Condvar::new(),
        }
    }

    /// The state, whatever a panicking worker left: every field is written
    /// whole under the lock, so a poisoned state is still a correct one.
    fn lock(&self) -> std::sync::MutexGuard<'_, GateState> {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// The `begin` line's number, asked by a worker with a second to file.
    ///
    /// # Errors
    ///
    /// The journal's refusal of the attempt this call waited for.
    fn line(&self) -> Result<u64, ImportRefusal> {
        let mut state = self.lock();
        if let Some(line) = state.line {
            return Ok(line);
        }
        let asked_at = state.attempts;
        state.asked = true;
        self.changed.notify_all();
        loop {
            if let Some(line) = state.line {
                return Ok(line);
            }
            if state.attempts > asked_at {
                return Err(state
                    .refused
                    .clone()
                    .unwrap_or_else(|| ImportRefusal::Journal {
                        why: "the day's begin line was not written".to_owned(),
                    }));
            }
            state = self
                .changed
                .wait(state)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
        }
    }

    /// The coordinating thread's side: writes `begin` through `write` each
    /// time a worker asks, until every worker has finished.
    fn serve(&self, write: &mut dyn FnMut() -> Result<u64, ImportRefusal>) {
        let mut state = self.lock();
        loop {
            if state.asked {
                state.asked = false;
                // Written under the lock: a worker that needs the line waits
                // for it anyway, and no other worker touches the gate.
                let written = write();
                state.attempts = state.attempts.saturating_add(1);
                match written {
                    Ok(line) => state.line = Some(line),
                    Err(why) => state.refused = Some(why),
                }
                self.changed.notify_all();
                continue;
            }
            if state.working == 0 {
                return;
            }
            state = self
                .changed
                .wait(state)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
        }
    }

    /// `count` workers finished, or will never start.
    fn finished(&self, count: usize) {
        let mut state = self.lock();
        state.working = state.working.saturating_sub(count);
        self.changed.notify_all();
    }

    /// The line, if it was written.
    fn written(&self) -> Option<u64> {
        self.lock().line
    }
}

/// Tells the gate a worker finished when it drops, so a worker that panics
/// still lets the coordinating thread stop serving and join it.
struct Finishing<'a>(&'a BeginGate);

impl Drop for Finishing<'_> {
    fn drop(&mut self) {
        self.0.finished(1);
    }
}

/// Runs `job` over every one of `jobs` on up to `workers` threads, the
/// coordinating thread serving the [`BeginGate`] meanwhile, and answers each
/// job's result at its index. `Err` when no thread could be started, with
/// why; a worker that panics is a panic of the run, resumed here.
fn on_workers<J: Sync, T: Send>(
    workers: usize,
    jobs: &[J],
    gate: &BeginGate,
    write_begin: &mut dyn FnMut() -> Result<u64, ImportRefusal>,
    job: &(dyn Fn(&J) -> T + Sync),
) -> Result<Vec<T>, String> {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let want = workers.min(jobs.len()).max(1);
    let next = AtomicUsize::new(0);
    let mut slots: Vec<Option<T>> = Vec::with_capacity(jobs.len());
    slots.resize_with(jobs.len(), || None);
    let refused = std::thread::scope(|scope| {
        let mut handles = Vec::with_capacity(want);
        let mut refused = None;
        for at in 0..want {
            let spawned = std::thread::Builder::new()
                .name(format!("gdfl-file-{at}"))
                .spawn_scoped(scope, || {
                    let _finishing = Finishing(gate);
                    let mut done = Vec::new();
                    loop {
                        let index = next.fetch_add(1, Ordering::Relaxed);
                        let Some(one) = jobs.get(index) else {
                            break;
                        };
                        done.push((index, job(one)));
                    }
                    done
                });
            match spawned {
                Ok(handle) => handles.push(handle),
                Err(why) => {
                    refused = Some(why.to_string());
                    break;
                }
            }
        }
        // A worker that never started never finishes: the gate is told now.
        gate.finished(want - handles.len());
        if handles.is_empty() {
            return refused.or_else(|| Some("no worker started".to_owned()));
        }
        if let Some(why) = refused.as_deref() {
            let _dropped_when_filtered = telemetry::emit(
                &telemetry::Event::warn(TARGET, "fewer workers than asked for")
                    .with("asked", want)
                    .with("started", handles.len())
                    .with("why", why),
            );
        }
        gate.serve(write_begin);
        for handle in handles {
            match handle.join() {
                Ok(done) => {
                    for (index, result) in done {
                        if let Some(slot) = slots.get_mut(index) {
                            *slot = Some(result);
                        }
                    }
                }
                Err(panicked) => std::panic::resume_unwind(panicked),
            }
        }
        None
    });
    match refused {
        Some(why) => Err(why),
        None => Ok(slots.into_iter().flatten().collect()),
    }
}

/// What every worker of a day shares while it files (WP-16): read-only.
struct Filing<'a> {
    run: &'a Run<'a>,
    day: Day,
    request: &'a BarRequest,
    /// The day's session, when it keeps a day ledger (D-3335).
    session: Option<&'a crate::calendar::Session>,
    /// The run's measured origin (D-3317 item 4).
    origin: Option<&'a Origin>,
    /// Where the day's files came from (D-3334), or why it is not known.
    source: Option<&'a Result<DaySource, String>>,
    /// The day's batch, when an earlier run left the day open (WP-16,
    /// D-3353): every month a file of the day is filed in is confirmed in
    /// it, so what that run wrote and never made durable is flushed for
    /// itself before this run closes the day.
    confirm: Option<&'a store::file::DurableBatch>,
    /// The run's id, which a back-fill's reason and event name (D-3350).
    id: u64,
}

/// What a file's landing left beside the bars it committed.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum Receipt {
    /// Nothing: no bar committed, or a run that names no origin.
    #[default]
    None,
    /// The provenance record of its append (D-3334): counted in
    /// [`Report::provenance_records`].
    Provenance,
    /// A new generation of its month (D-3350; DB-N26, WP-12d), journaled in
    /// `<store>/audit/gdfl.repairs`: counted in [`Report::months_repaired`].
    /// A revision carries no provenance record.
    NewGeneration,
}

/// One file as a worker filed it, folded into its day in listing order by
/// [`DayFiler`] (WP-16): every count the file adds and every failure it
/// named, in the order it named them.
#[derive(Debug, Default)]
struct Filed {
    /// The fold refused the file: counted refused, and never also read
    /// (D-2814).
    refused: bool,
    /// What its placement counted, for a file the fold took.
    placement: Option<Placement>,
    /// Its quality (D-3335) and the symbol it is filed under.
    quality: Option<(String, quality::FileQuality)>,
    /// Bars the venue's session declined.
    outside_session: usize,
    /// Bars offered to the store.
    seconds: usize,
    /// Bars the store wrote.
    committed: usize,
    /// What its landing left beside its bars.
    receipt: Receipt,
    /// Whether its month was confirmed in the day's batch (D-3353): the day
    /// had been left open by an earlier run.
    confirmed: bool,
    /// The census row of the month it filed.
    held: Option<Held>,
    /// Its failures, in the order they arose.
    failures: Vec<Staged>,
}

/// A file's quality (D-3335), from its rows, the bars it built and what its
/// placement counted.
fn measure(
    kind: ImportKind,
    (day, session): (Day, &crate::calendar::Session),
    handed: &Handed,
    converted: &Converted,
    deferred: quality::Deferred,
) -> quality::FileQuality {
    let seconds: Vec<u32> = converted
        .seconds
        .iter()
        .map(|bar| second_of(bar.ts_micros, day))
        .collect();
    quality::measure_file(
        kind,
        (session, quality::halt_window(kind, day)),
        &handed.file.ticks,
        &seconds,
        (converted.placement, deferred),
        handed.facts.reader,
    )
}

/// A file's quality (D-3335), folded into `ledger`.
fn take_quality(
    kind: ImportKind,
    ledger: &mut quality::DayLedger,
    handed: &Handed,
    converted: &Converted,
    deferred: quality::Deferred,
) {
    let measured = measure(
        kind,
        (ledger.day(), ledger.session()),
        handed,
        converted,
        deferred,
    );
    ledger.take(&handed.file.symbol, &measured);
}

/// The store's door a file is filed through (WP-16 with WP-12d).
///
/// A day's files run on workers inside the day's batch, where no writer
/// flushes for itself (D-3353). The versioned month repair (D-3350) is not
/// such a writer: its own flushes order a revision's bars before its receipt
/// and its promotion, it journals to one ledger, `<store>/audit/gdfl.repairs`,
/// and it costs O(month). So a worker files through the append door alone,
/// and a file that door refuses outright is filed again through the
/// back-fill door by the coordinating thread, in listing order and outside
/// the batch: the repair keeps every flush it was written with, and the
/// repair journal's lines are in the order one worker writes them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Door {
    /// `ingest::from_rows`: a tail, a new month, a day already held.
    Append,
    /// `ingest::repair_rows`: the append door again, then the repair for a
    /// day its month lacks where no append can land it.
    BackFill,
}

impl Filing<'_> {
    /// Converts and files one instrument file, measures its quality and
    /// records its landing: what one file of a day does, on whichever worker
    /// took it. `begin` answers the day's `begin` line, written the first time
    /// a file has a second to file.
    ///
    /// The flag is `true` only through [`Door::Append`], when the append door
    /// refused the file and committed nothing: the answer is then not the
    /// file's, nothing of it is to be counted, and the caller files it again
    /// through [`Door::BackFill`], whose answer is always the file's.
    fn file(
        &self,
        handed: &Handed,
        begin: &dyn Fn() -> Result<u64, ImportRefusal>,
        door: Door,
    ) -> (Filed, bool) {
        let run = self.run;
        let file = &handed.file;
        let mut filed = Filed::default();
        // A file the fold refuses is counted refused and never also read
        // (D-2814): files, skips and refusals add up to the day's entries.
        let (converted, deferred) = match convert_noted(run.kind, self.day, &file.ticks) {
            Ok(two) => two,
            Err(why) => {
                filed.refused = true;
                filed
                    .failures
                    .push(staged(run, self.day, &file.name, Stage::File, &why));
                return (filed, false);
            }
        };
        if let Some(session) = self.session {
            let measured = measure(run.kind, (self.day, session), handed, &converted, deferred);
            filed.quality = Some((file.symbol.clone(), measured));
        }
        let p = converted.placement;
        filed.placement = Some(p);
        if converted.seconds.is_empty() {
            return (filed, false);
        }
        let begin_line = match begin() {
            Ok(line) => line,
            Err(why) => {
                filed
                    .failures
                    .push(staged(run, self.day, "journal", Stage::Journal, &why));
                return (filed, false);
            }
        };
        let origin = format!("gdfl {} {} {}", run.kind.as_str(), self.day, file.name);
        // A DAY ITS MONTH LACKS, WHERE NO APPEND CAN LAND IT, LANDS AS A NEW
        // GENERATION OF THE MONTH (D-3350; DB-N26). The append-only month
        // refuses it, and was the end of it: the 12 index days WP-12a
        // admitted were refused for every NIFTY and BANKNIFTY file on 10 Oct
        // 2026. The back-fill door tries the append door first, so a tail, a
        // new month and a day already held land exactly as before; it
        // repairs only a month that holds no bar inside the file's span and
        // cannot be appended to, never in place, and every other refusal
        // stands as it was. The reason names the run, whose `run` line names
        // the archive.
        let Some((done, landed)) = self.through(door, file, &converted.seconds, &origin) else {
            return (filed, true);
        };
        filed.outside_session = usize::try_from(done.census.total()).unwrap_or(0);
        filed.seconds = done.bars_stored;
        filed.committed = done.bars_committed;
        if done.bars_committed > 0
            && let Some(held) = done.pending.as_ref()
        {
            match landed {
                Landed::Appended => {
                    match self.provenance(handed, held, done.bars_committed, begin_line) {
                        Ok(true) => filed.receipt = Receipt::Provenance,
                        Ok(false) => {}
                        Err(refused) => filed.failures.push(refused),
                    }
                }
                Landed::NewGeneration => {
                    filed.receipt = Receipt::NewGeneration;
                    self.repaired(handed, held, done.bars_committed);
                }
            }
        }
        // A DAY AN EARLIER RUN LEFT OPEN: whatever that run wrote of this
        // month was never made durable, and found present it is not written
        // again, so it is named to the day's batch, which flushes the month's
        // files and directories for themselves (D-3353). A month that cannot
        // be named keeps the day open, by name.
        if let (Some(batch), Some(held)) = (self.confirm, done.pending.as_ref()) {
            match confirm_month(run.store_root, held, batch) {
                Ok(confirmed) => filed.confirmed = confirmed,
                Err(why) => {
                    let refusal = ImportRefusal::Filing { why };
                    filed
                        .failures
                        .push(staged(run, self.day, &file.name, Stage::File, &refusal));
                }
            }
        }
        filed.held = done.pending;
        for failure in done.failures {
            let refusal = ImportRefusal::Filing { why: failure.why };
            filed
                .failures
                .push(staged(run, self.day, &file.name, Stage::File, &refusal));
        }
        let _dropped_when_filtered = telemetry::emit(
            &telemetry::Event::debug(TARGET, "filed")
                .with("file", file.name.as_str())
                .with("rows", p.rows)
                .with("seconds", converted.seconds.len()),
        );
        (filed, false)
    }

    /// Offers a file's bars to the store through `door`: what the store
    /// answered and how the bars landed, or `None` when [`Door::Append`]
    /// refused them and committed nothing.
    fn through(
        &self,
        door: Door,
        file: &TickFile,
        seconds: &[Bar],
        origin: &str,
    ) -> Option<(Ingested, Landed)> {
        let run = self.run;
        let month = || plan(run.kind, self.request, file.contract);
        match door {
            Door::Append => {
                let done = from_rows(seconds, &[], &file.symbol, origin, run.store_root, month());
                (done.bars_committed > 0 || done.failures.is_empty())
                    .then_some((done, Landed::Appended))
            }
            Door::BackFill => {
                let reason = format!(
                    "gdfl-import {} {} {} run:{}: a day the month lacks where no append can land it",
                    run.kind.as_str(),
                    self.day,
                    file.name,
                    self.id
                );
                // OUTSIDE THE DAY'S BATCH, whichever thread this is: the
                // repair's flushes are its own (D-3350), and so are those of
                // an append the door takes this second time.
                let _outside_the_batch = store::file::DurableBatch::suspend();
                Some(repair_rows(
                    seconds,
                    &file.symbol,
                    origin,
                    run.store_root,
                    month(),
                    &reason,
                ))
            }
        }
    }

    /// Writes the provenance record of a file's landing (D-3334) when the
    /// run names its origin and the source named the file's member, and says
    /// whether it wrote one. A record the sidecar refuses is a named failure
    /// of the day, never silence: the bars stay committed and stay
    /// unrecorded, which `/provenance.json` labels reconstructed.
    fn provenance(
        &self,
        handed: &Handed,
        held: &Held,
        committed: usize,
        begin_line: u64,
    ) -> Result<bool, Staged> {
        let run = self.run;
        let (Some(origin), Some(member), Some(source)) =
            (self.origin, handed.facts.member.as_ref(), self.source)
        else {
            return Ok(false);
        };
        let recorded = source.clone().and_then(|source| {
            let (kind, name, bytes, crc32) = match source {
                DaySource::Members { prefix } => (
                    store::provenance::SourceKind::ArchiveMember,
                    format!("{prefix}{}", member.entry),
                    member.bytes,
                    member.crc32,
                ),
                DaySource::TickstoreDay { name, bytes } => {
                    (store::provenance::SourceKind::TickstoreDay, name, bytes, 0)
                }
            };
            let day =
                store::provenance::Day::new(self.day.year(), self.day.month(), self.day.day())
                    .ok_or_else(|| format!("{} is not a provenance day", self.day))?;
            let landing = store::provenance::Landing {
                source: store::provenance::Source {
                    kind,
                    name: &name,
                    bytes,
                    crc32,
                },
                day,
                bar_definition: BAR_DEFINITION,
                journal_ordinal: begin_line,
                commit: origin.commit.as_deref(),
                landed_at_micros: now_micros(),
            };
            record_provenance(run.store_root, held, committed, &landing)
        });
        recorded.map_err(|why| {
            let refusal = ImportRefusal::Provenance { why };
            staged(run, self.day, &handed.file.name, Stage::File, &refusal)
        })
    }

    /// A file whose day its month lacked inside what it holds, landed as a
    /// NEW GENERATION of the month (D-3350; DB-N26): logged with the month's
    /// rows now served, and counted in [`Report::months_repaired`] when the
    /// file is folded into its day. The repair door journaled it in `<store>/audit/gdfl.repairs`
    /// with this run's id.
    ///
    /// **No `.prv` record is written, and the event says so.** A sidecar
    /// record covers the bars ONE append committed at the end of its file,
    /// at that file's commit generation (`store::provenance::record`); the
    /// added bars sit inside the new generation, and its records would index
    /// bars the original never held, so a revision carries no `.prv`
    /// (`crates/store/REPAIR.md`). `/provenance.json` reads beside the
    /// generation every reader opens and rebuilds the month's answer from
    /// this journal's day lines, labelled reconstructed; the original's
    /// `.prv`, if any, is left byte-identical. When the run names its origin
    /// — so an append would have been recorded — the event is a `warn`.
    fn repaired(&self, handed: &Handed, held: &Held, added: usize) {
        let unrecorded =
            self.origin.is_some() && handed.facts.member.is_some() && self.source.is_some();
        let (day, month) = (self.day.to_string(), held.entry.key.month.to_string());
        let event = if unrecorded {
            telemetry::Event::warn(
                TARGET,
                "day back-filled as a new generation of its month; no .prv record: a revision \
                 carries none (D-3350), the repair journal and this journal's day line name it",
            )
        } else {
            telemetry::Event::info(TARGET, "day back-filled as a new generation of its month")
        };
        let _dropped_when_filtered = telemetry::emit(
            &event
                .with("file", handed.file.name.as_str())
                .with("day", day.as_str())
                .with("month", month.as_str())
                .with("added", added)
                .with("rows", held.entry.rows)
                .with("run", self.id),
        );
    }
}

/// What reading one file of a day came to, answered to its source in listing
/// order (WP-16): read and filed, with the contract-day the lineage folds
/// when the source asked for it, or refused by the reader.
#[derive(Debug)]
enum JobRead {
    /// Read; filed by [`Filing::file`].
    Read(Option<lineage::DayContract>),
    /// The source or the reader refused it.
    Refused(Refused),
}

/// What a worker answers for one job.
#[derive(Debug)]
enum JobDone {
    /// Read and filed.
    Filed(Box<Filed>, Option<lineage::DayContract>),
    /// Read, and refused outright by the append door: the coordinating
    /// thread files it through the back-fill door ([`Door::BackFill`]).
    BackFill(Box<Handed>, Option<lineage::DayContract>),
    /// The source or the reader refused it.
    Refused(Refused),
}

/// One file read by a source for a day's job: its rows and facts, and the
/// contract-day the lineage folds, when asked.
type ReadJob = Result<(Handed, Option<lineage::DayContract>), Refused>;

/// One job of a day once every file is filed: the file as it was filed and
/// the contract-day the lineage folds, or the read's refusal.
type FiledJob = Result<(Box<Filed>, Option<lineage::DayContract>), Refused>;

/// THE BACK-FILL PASS (D-3350 under WP-16): every file the append door refused
/// outright, filed through the back-fill door ([`Door::BackFill`]), one after
/// the other in listing order on the calling thread. Each had asked for the
/// day's `begin` line before it reached a door, so `begun` holds it.
fn back_fill(filing: &Filing<'_>, begun: Option<u64>, done: Vec<JobDone>) -> Vec<FiledJob> {
    let begin_again = || {
        begun.ok_or_else(|| ImportRefusal::Journal {
            why: "the day's begin line was not written".to_owned(),
        })
    };
    done.into_iter()
        .map(|one| match one {
            JobDone::Filed(filed, contract) => Ok((filed, contract)),
            JobDone::BackFill(handed, contract) => {
                let (filed, _never_through_this_door) =
                    filing.file(&handed, &begin_again, Door::BackFill);
                Ok((Box::new(filed), contract))
            }
            JobDone::Refused(refused) => Err(refused),
        })
        .collect()
}

/// One admitted day's filing (WP-16): the source hands it the day's wanted
/// files, it files them on the run's workers, and it folds what each file
/// did into the day in listing order.
struct DayFiler<'a> {
    ctx: &'a Ctx<'a>,
    key: &'a str,
    day: Day,
    request: &'a BarRequest,
    /// Where the day's files came from (D-3334), when the run names an
    /// origin.
    source: Option<Result<DaySource, String>>,
    workers: usize,
    /// The day's `begin` line, once written.
    begin: Option<u64>,
    report: &'a mut Report,
    held: Vec<Held>,
    failures: Vec<Staged>,
    /// The day's largest back-step, for its journal line.
    max_back_s: u32,
    /// The day's quality fields (D-3335), when the day has a session.
    ledger: Option<quality::DayLedger>,
    /// The day's batch (WP-16, D-3353), which `drive_core` makes durable
    /// once the last file is filed: the one every write of the day's files
    /// is left to when the run makes a day durable together, and the one the
    /// day's months are confirmed in when an earlier run left the day open.
    batch: Option<store::file::DurableBatch>,
    /// Whether each job enters the batch on the thread that runs it, so its
    /// writers leave their flushes to it ([`Durability::Day`]). A run that
    /// flushes each write as it is written has a batch only to confirm in.
    enters: bool,
    /// Whether an earlier run left the day open ([`Journal::left_open`]):
    /// every month a file of the day is filed in is confirmed in the batch.
    confirm: bool,
    /// The months confirmed so far.
    confirmed: usize,
    /// Which files the day's reader is asked for (D-4878).
    ask: Ask,
}

impl<'a> DayFiler<'a> {
    /// The filer of one admitted day: its quality ledger when the day has a
    /// session, where its files came from when the run names an origin, and
    /// its batch.
    ///
    /// A DAY AN EARLIER RUN BEGAN AND NEVER CLOSED was cut off after it had
    /// written some of the day's months and before it made them durable.
    /// Read again they are found present and nothing is written, so this run
    /// confirms each month the day files in the day's batch (D-3353),
    /// whatever its own durability: a run that flushes each write as it is
    /// written takes a batch for the confirmations alone.
    fn of_day(
        ctx: &'a Ctx<'a>,
        (key, day): (&'a str, Day),
        request: &'a BarRequest,
        report: &'a mut Report,
    ) -> Self {
        let (kind, options) = (ctx.run.kind, ctx.options);
        let left_open = ctx.journal.left_open(kind, day);
        Self {
            ctx,
            key,
            day,
            request,
            source: options.origin.map(|origin| day_source(kind, day, origin)),
            workers: options.workers.get(),
            begin: None,
            report,
            held: Vec::new(),
            failures: Vec::new(),
            max_back_s: 0,
            ledger: crate::gdfl_cm::session_of(day)
                .ok()
                .map(|session| quality::DayLedger::new(kind, day, session)),
            batch: options.durability.batch(left_open),
            enters: options.durability != Durability::EachWrite,
            confirm: left_open,
            confirmed: 0,
            ask: Ask::All,
        }
    }
}

impl DayFiler<'_> {
    /// Reads every one of `jobs` through `read` and files what it reads, on
    /// the run's workers; folds each file into the day in `jobs` order and
    /// answers how each read went, in that order.
    ///
    /// With one worker, or one job, nothing is started: the jobs run on this
    /// thread, as before WP-16. A run that cannot start a single worker
    /// thread says so at `warn` and runs the day here too, never silently.
    fn file_all<J: Sync>(
        &mut self,
        jobs: &[J],
        read: &(dyn Fn(&J) -> ReadJob + Sync),
    ) -> Vec<JobRead> {
        let session = self.ledger.as_ref().map(|ledger| *ledger.session());
        let batch = self.batch.as_ref().filter(|_| self.enters);
        let filing = Filing {
            run: self.ctx.run,
            day: self.day,
            request: self.request,
            session: session.as_ref(),
            origin: self.ctx.options.origin,
            source: self.source.as_ref(),
            confirm: self.batch.as_ref().filter(|_| self.confirm),
            id: self.ctx.id,
        };
        let mut done: Option<Vec<JobDone>> = None;
        if self.workers > 1 && jobs.len() > 1 {
            let gate = BeginGate::new(self.begin, self.workers.min(jobs.len()));
            let (ctx, key) = (self.ctx, self.key);
            let threaded = on_workers(
                self.workers,
                jobs,
                &gate,
                &mut || ctx.append(&format!("begin {key}")),
                &|job| {
                    let _in_the_day = batch.map(store::file::DurableBatch::enter);
                    match read(job) {
                        Ok((handed, contract)) => {
                            match filing.file(&handed, &|| gate.line(), Door::Append) {
                                (_, true) => JobDone::BackFill(Box::new(handed), contract),
                                (filed, false) => JobDone::Filed(Box::new(filed), contract),
                            }
                        }
                        Err(refused) => JobDone::Refused(refused),
                    }
                },
            );
            self.begin = gate.written();
            match threaded {
                Ok(all) => done = Some(all),
                Err(why) => {
                    let day_text = self.day.to_string();
                    let _dropped_when_filtered = telemetry::emit(
                        &telemetry::Event::warn(
                            TARGET,
                            "no worker thread started; the day's files run on one thread",
                        )
                        .with("day", day_text.as_str())
                        .with("why", why.as_str()),
                    );
                }
            }
        }
        let done = done.unwrap_or_else(|| {
            let line = std::cell::Cell::new(self.begin);
            let (ctx, key) = (self.ctx, self.key);
            let begin = || {
                if let Some(written) = line.get() {
                    return Ok(written);
                }
                let written = ctx.append(&format!("begin {key}"))?;
                line.set(Some(written));
                Ok(written)
            };
            let _in_the_day = batch.map(store::file::DurableBatch::enter);
            let all = jobs
                .iter()
                .map(|job| match read(job) {
                    Ok((handed, contract)) => match filing.file(&handed, &begin, Door::Append) {
                        (_, true) => JobDone::BackFill(Box::new(handed), contract),
                        (filed, false) => JobDone::Filed(Box::new(filed), contract),
                    },
                    Err(refused) => JobDone::Refused(refused),
                })
                .collect();
            self.begin = line.get();
            all
        });
        let done = back_fill(&filing, self.begin, done);
        done.into_iter()
            .map(|one| match one {
                Ok((filed, contract)) => {
                    self.fold(*filed);
                    JobRead::Read(contract)
                }
                Err(refused) => JobRead::Refused(refused),
            })
            .collect()
    }

    /// Folds one filed file into the day: exactly what filing it did to the
    /// report, the ledger, the census batch and the failures when the day ran
    /// on one thread.
    fn fold(&mut self, filed: Filed) {
        if filed.refused {
            self.report.files_refused += 1;
        } else {
            self.report.files += 1;
        }
        if let (Some(ledger), Some((symbol, quality))) = (self.ledger.as_mut(), &filed.quality) {
            ledger.take(symbol, quality);
        }
        if let Some(p) = filed.placement {
            self.report.rows += p.rows;
            self.report.ltq_zero_dropped += p.ltq_zero_dropped;
            self.report.late_rows += p.late_rows;
            self.report.late_unresolved += p.late_unresolved;
            self.report.max_back_s = self.report.max_back_s.max(p.max_back_s);
            self.max_back_s = self.max_back_s.max(p.max_back_s);
        }
        self.report.outside_session += filed.outside_session;
        self.report.seconds += filed.seconds;
        self.report.seconds_committed += filed.committed;
        match filed.receipt {
            Receipt::None => {}
            Receipt::Provenance => self.report.provenance_records += 1,
            Receipt::NewGeneration => self.report.months_repaired += 1,
        }
        self.confirmed += usize::from(filed.confirmed);
        self.held.extend(filed.held);
        self.failures.extend(filed.failures);
    }
}

/// Writes an admitted day's one closing line and says how the day closed
/// (D-3317): `done`, done with refusals, `incomplete` or `refused`, by
/// [`close_of`]. Every admitted day the source holds gets one, begun or not:
/// until D-3317 a day whose every file was refused left no line at all.
fn close_day(
    ctx: &Ctx<'_>,
    (key, day): (&str, Day),
    (before, report, max_back_s): (Mark, &Report, u32),
    failures: &mut Vec<Staged>,
) -> Outcome {
    let counts = Counts::of_day(before, report, failures.len(), max_back_s);
    ctx.close(
        (key, day),
        close_of(failures, counts.files > 0),
        &counts,
        failures,
    )
}

/// Whether `day` is imported this run: not already done (a day done with
/// refusals is read again under [`RunOptions::retry_refused`]), a regular
/// session (a closed day is counted, any other calendar answer is a named
/// failure and a `refused` line), and named as resumed when the journal
/// sends it back.
fn admit(ctx: &Ctx<'_>, key: &str, day: Day, report: &mut Report) -> bool {
    let retried = ctx.options.retry_refused && ctx.journal.with_refusals.contains(key);
    if ctx.journal.done.contains(key) && !retried {
        report.days_skipped += 1;
        return false;
    }
    match session_gate(day) {
        Ok(()) => {}
        Err(CmRefusal::CalendarNotRegular {
            kind: crate::calendar::DayKind::Closed,
        }) => {
            report.days_closed += 1;
            return false;
        }
        Err(why) => {
            report.days_refused += 1;
            let cause = cm_cause(&why);
            let mut failures = vec![Staged {
                cause,
                failure: note_failure(ctx.run.kind, day, "calendar", &why.to_string(), cause),
            }];
            let counts = Counts {
                failures: 1,
                ..Counts::default()
            };
            let _refused = ctx.close((key, day), Close::Refused(cause), &counts, &mut failures);
            report
                .failures
                .extend(failures.into_iter().map(|one| one.failure));
            return false;
        }
    }
    name_retried(ctx.journal, ctx.run.kind, (key, day, retried), report);
    true
}

/// Names `day` in `resumed` when the journal sends it back, whatever closed
/// it (D-2821), or when it is a day done with refusals read again (D-3317),
/// and in `restated` too when its last `done` was under another bar
/// definition (D-2822), with a warning either way.
fn name_retried(
    journal: &Journal,
    kind: ImportKind,
    (key, day, retried): (&str, Day, bool),
    report: &mut Report,
) {
    let restated = journal.restated.contains(key);
    if !(retried || restated || journal.open.contains(key) || journal.incomplete.contains(key)) {
        return;
    }
    report.resumed.push(day);
    if restated {
        report.restated.push(day);
    }
    let day_text = day.to_string();
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::warn(
            TARGET,
            "importing again a day an earlier run did not finish under this bar definition",
        )
        .with("kind", kind.as_str())
        .with("day", day_text.as_str())
        .with("restated", restated)
        .with("retry_refused", retried)
        .with("definition", BAR_DEFINITION),
    );
}

/// The one runtime: every day of the run through the calendar, the venue,
/// the journal, the source's `read_day` and the common filing path. Its
/// `run` line records the provenance as `unrecorded`; [`drive_from`] is the
/// door for a caller that measured its [`Origin`].
///
/// # Errors
///
/// [`ImportRefusal::RangeBackwards`], [`ImportRefusal::FilterName`] and
/// [`ImportRefusal::Journal`] for the run, and [`ImportRefusal::CensusFull`]
/// or [`ImportRefusal::CensusRefused`] at the first day the census refuses,
/// where the run stops (D-3317); everything else is a named failure in the
/// report.
pub fn drive<F>(run: &Run<'_>, read_day: F) -> Result<Report, ImportRefusal>
where
    F: FnMut(Day, &mut dyn FnMut(TickFile)) -> DayRead,
{
    drive_with(run, RunOptions::default(), &mut StoreCensus, read_day)
}

/// [`drive`] with the run's [`RunOptions`]: the origin the caller measured,
/// written as the run's `run` line and in "run started" (D-3317; DB-G56), and
/// whether to read again the days done with refusals.
///
/// # Errors
///
/// As [`drive`].
pub fn drive_from<F>(
    run: &Run<'_>,
    options: RunOptions<'_>,
    read_day: F,
) -> Result<Report, ImportRefusal>
where
    F: FnMut(Day, &mut dyn FnMut(TickFile)) -> DayRead,
{
    drive_with(run, options, &mut StoreCensus, read_day)
}

/// "run started", with everything the caller said about where the data came
/// from, or `source=unrecorded` when it said nothing (D-3317; DB-G56). An
/// origin that could not be measured is logged at `warn`, naming why.
fn note_started(ctx: &Ctx<'_>, filter: &str) {
    let run = ctx.run;
    let (from_text, to_text) = (run.from.to_string(), run.to.to_string());
    let origin = ctx.options.origin;
    let (path, digest) = origin.map_or_else(Default::default, |origin| {
        (origin.path.display().to_string(), origin.digest_hex())
    });
    let event = started_event(
        (run.kind, from_text.as_str(), to_text.as_str(), filter),
        (ctx.id, ctx.options.retry_refused),
        origin.map(|origin| (origin, path.as_str(), digest.as_str())),
    );
    let _dropped_when_filtered = telemetry::emit(&event);
    let _dropped_when_filtered = telemetry::emit(&settings_event(ctx.id, &ctx.options));
}

/// "run settings" (WP-16, D-3353): the workers and the durability unit the
/// run was started with, beside its id, so the audit trail can say which
/// durability a day left open ran under without inferring it from the "day
/// made durable" counts (an `each-write` run leaves none of those). Its own
/// event because "run started" is already [`telemetry::MAX_FIELDS`] wide,
/// and not on the journal's `run` line because neither setting changes a
/// byte the run leaves, the journal included.
fn settings_event(id: u64, options: &RunOptions<'_>) -> telemetry::Event<'static> {
    telemetry::Event::info(TARGET, "run settings")
        .with("run", id)
        .with("workers", options.workers.get())
        .with("durability", options.durability.as_str())
}

/// "run started" (D-3317 item 4; DB-G56): what the run reads and where its
/// data came from, in at most [`telemetry::MAX_FIELDS`] fields so a fault is
/// never the field the sink counts instead of keeping (the bar definition is
/// on every closing line, not here). `warn` when the origin names a fault.
fn started_event<'a>(
    (kind, from, to, filter): (ImportKind, &'a str, &'a str, &'a str),
    (id, retry_refused): (u64, bool),
    origin: Option<(&'a Origin, &'a str, &'a str)>,
) -> telemetry::Event<'a> {
    let level = if origin.is_some_and(|(origin, ..)| origin.fault.is_some()) {
        telemetry::Event::warn(TARGET, "run started")
    } else {
        telemetry::Event::info(TARGET, "run started")
    };
    let event = level
        .with("kind", kind.as_str())
        .with("from", from)
        .with("to", to)
        .with("only", filter)
        .with("run", id)
        .with("retry_refused", retry_refused);
    let Some((origin, path, digest)) = origin else {
        return event.with("source", "unrecorded");
    };
    let event = event
        .with("source", origin.source.as_str())
        .with("archive", path)
        .with("archive_bytes", origin.archive_bytes)
        .with(origin.source.digest_field(), digest)
        .with("commit", origin.commit.as_deref().unwrap_or("unstamped"));
    match &origin.fault {
        Some(fault) => event.with("archive_fault", fault.as_str()),
        None => event,
    }
}

/// Whose close a summary event tells (D-3317, D-4878).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Closed<'c> {
    /// A day read whole, under its base key.
    Day,
    /// One member key of a day read again for a wider scope: the member and
    /// the journal key its closing line is written under.
    Member { member: &'c str, key: &'c str },
    /// A day read again for a wider scope, once, after every member key the
    /// run opened on it is closed: how many those were.
    Widened { members: usize },
}

/// The words of a summary event, by whose close it is and how it closed. A
/// member key's close is never "day imported": that message is the DAY's,
/// once a day, and the import's pace is measured from the gaps between its
/// stamps (`api`'s `/imports.json`), so one per member key made a day of six
/// members read as six days (WP-12f repair 1).
const fn summary_words(closed: Closed<'_>, outcome: Outcome) -> &'static str {
    match (closed, outcome) {
        (Closed::Member { .. }, Outcome::Done | Outcome::DoneWithRefusals) => "member imported",
        (Closed::Member { .. }, Outcome::Incomplete) => "member incomplete",
        (Closed::Member { .. }, Outcome::Refused) => "member refused",
        (_, Outcome::Done | Outcome::DoneWithRefusals) => "day imported",
        (_, Outcome::Incomplete) => "day incomplete",
        (_, Outcome::Refused) => "day refused",
    }
}

/// One close's summary event (D-3317; D-4878 for a member key and a widened
/// day). For a DAY, read whole or read again for a wider scope: "day
/// imported" at `info` for a day closed `done`, with or without refusals;
/// "day incomplete" or "day refused" at `warn` otherwise; a widened day says
/// `members=`, how many member keys the run closed on it. For a MEMBER KEY:
/// "member imported", "member incomplete" or "member refused", each naming
/// its `member=` and its journal `key=`, so a member's close is found in the
/// event log by its name. A census-refused day never reaches it.
fn note_day(
    kind: ImportKind,
    day: Day,
    (outcome, closed): (Outcome, Closed<'_>),
    (before, report): (Mark, &Report),
) {
    let day_text = day.to_string();
    let words = summary_words(closed, outcome);
    let event = match outcome {
        Outcome::Done | Outcome::DoneWithRefusals => telemetry::Event::info(TARGET, words),
        Outcome::Incomplete | Outcome::Refused => telemetry::Event::warn(TARGET, words),
    };
    let event = event
        .with("kind", kind.as_str())
        .with("day", day_text.as_str())
        .with("outcome", outcome.as_str())
        .with("files", report.files - before.files)
        .with("rows", report.rows - before.rows)
        .with("seconds", report.seconds - before.seconds)
        .with(
            "committed",
            report.seconds_committed - before.seconds_committed,
        )
        .with("failures", report.failures.len() - before.failures);
    let event = match closed {
        Closed::Day => event,
        Closed::Member { member, key } => event.with("member", member).with("key", key),
        Closed::Widened { members } => event.with("members", members),
    };
    #[cfg(test)]
    tests::summary_emitted(&event);
    let _dropped_when_filtered = telemetry::emit(&event);
}

/// The worse of two closes of one day's keys, `done` best and `refused`
/// worst: how a day read again for a wider scope closed as a whole.
const fn worse(a: Outcome, b: Outcome) -> Outcome {
    const fn rank(outcome: Outcome) -> u8 {
        match outcome {
            Outcome::Done => 0,
            Outcome::DoneWithRefusals => 1,
            Outcome::Incomplete => 2,
            Outcome::Refused => 3,
        }
    }
    if rank(b) > rank(a) { b } else { a }
}

/// Stops the run at a census refusal (D-3317 item 7; DB-G01, DB-G10). The
/// day's bars are on disk, so it is journalled `incomplete` with
/// `stage=census` and its files and seconds stay in the report; it is not
/// counted in `days_imported`; ONE `error` event names the census file's
/// room, the batch and that nothing was published; and the refusal carries
/// the report of what the run did before it stopped.
fn stop_at_census(
    ctx: &Ctx<'_>,
    (key, day): (&str, Day),
    (before, mut report, max_back_s): (Mark, Report, u32),
    mut failures: Vec<Staged>,
    (why, room): (String, Result<CensusRoom, String>),
) -> ImportRefusal {
    let kind = ctx.run.kind;
    let cause = Cause::open(Stage::Census, census_code(&room));
    let words = census_words(&why, &room);
    failures.push(Staged {
        cause,
        failure: failure_on(day, &format!("{day} census"), words.clone()),
    });
    let _incomplete = close_day(
        ctx,
        (key, day),
        (before, &report, max_back_s),
        &mut failures,
    );
    let day_text = day.to_string();
    let event = stopped_event(
        (kind, day_text.as_str(), cause),
        (words.as_str(), report.days_imported),
        room.as_ref().ok(),
    );
    let _dropped_when_filtered = telemetry::emit(&event);
    report
        .failures
        .extend(failures.into_iter().map(|one| one.failure));
    census_refusal((kind, day), why, room, report)
}

/// "census refused; run stopped" (D-3317 item 7; DB-G10), at `error`: the
/// day, the cause, the census's room when it was measured, that nothing was
/// published and how many days the run imported first, in at most
/// [`telemetry::MAX_FIELDS`] fields, so none of the room's numbers is the one
/// the sink counts instead of keeping.
fn stopped_event<'a>(
    (kind, day, cause): (ImportKind, &'a str, Cause),
    (why, days_imported): (&'a str, usize),
    room: Option<&CensusRoom>,
) -> telemetry::Event<'a> {
    let event = telemetry::Event::error(TARGET, "census refused; run stopped")
        .with("kind", kind.as_str())
        .with("day", day)
        .with("stage", cause.stage.as_str())
        .with("code", cause.code)
        .with("why", why)
        .with("days_imported", days_imported)
        .with("published", 0_u64);
    match room {
        Some(room) => event
            .with("n_valid", room.n_valid)
            .with("limit", room.limit)
            .with("free", room.free())
            .with("batch", room.batch)
            .with("new", room.new),
        None => event,
    }
}

/// The day's census failure in words that name the file's numbers (D-3317;
/// DB-G10): that nothing of the batch was published, the room and the batch
/// FIRST, the census's own words last, so a log value cut at its bound keeps
/// what matters.
fn census_words(why: &str, room: &Result<CensusRoom, String>) -> String {
    match room {
        Ok(room) if !room.fits() => format!(
            "the census is full, so nothing of the batch was published: n_valid={} limit={} \
             free={} batch={} new={} (the census's own words: {why})",
            room.n_valid,
            room.limit,
            room.free(),
            room.batch,
            room.new,
        ),
        Ok(room) => format!(
            "the census refused a batch that fits, and nothing of the batch was published \
             (n_valid={} limit={} free={} batch={} new={}): {why}",
            room.n_valid,
            room.limit,
            room.free(),
            room.batch,
            room.new,
        ),
        Err(unread) => format!(
            "the census refused, and nothing of the batch was published: {why}; its room could \
             not be measured: {unread}"
        ),
    }
}

/// A trading day the source does not hold: counted and named in
/// `days_missing`, warned, and closed `refused ... stage=vendor
/// reason=day_not_held failures=0` (D-3317 item 3, DB-025), so the hole has
/// a reason in the journal. It names no failure of its own.
fn close_missing(
    ctx: &Ctx<'_>,
    (key, day): (&str, Day),
    report: &mut Report,
    mut failures: Vec<Staged>,
) {
    report.days_missing.push(day);
    let day_text = day.to_string();
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::warn(TARGET, "the source holds no file of this trading day")
            .with("kind", ctx.run.kind.as_str())
            .with("day", day_text.as_str())
            .with("code", ImportRefusal::DayNotHeld { day }.code()),
    );
    let not_held = Cause::open(Stage::Vendor, ImportRefusal::DayNotHeld { day }.code());
    let cause = decisive(&failures).map_or(not_held, |one| one.cause);
    let counts = Counts {
        failures: failures.len(),
        ..Counts::default()
    };
    let _refused = ctx.close((key, day), Close::Refused(cause), &counts, &mut failures);
    report
        .failures
        .extend(failures.into_iter().map(|one| one.failure));
}

/// The UNIX second now; 0 for a clock set before 1970, which no run line can
/// then be mistaken for a real start by.
fn now_unix_s() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}

/// The runtime behind [`drive`] and [`drive_from`], with the census it
/// publishes through handed in (a test hands a double; [`Census`]), for a
/// source that hands on bare [`TickFile`]s: no member, so no provenance
/// record, and no reader counts. The day's files are gathered as the source
/// hands them on, then filed on the run's workers in that order (WP-16).
fn drive_with<F>(
    run: &Run<'_>,
    options: RunOptions<'_>,
    census: &mut dyn Census,
    mut read_day: F,
) -> Result<Report, ImportRefusal>
where
    F: FnMut(Day, &mut dyn FnMut(TickFile)) -> DayRead,
{
    drive_core(run, options, (census, None), |day, filer| {
        let mut files = Vec::new();
        let read = read_day(day, &mut |file| files.push(file));
        let handed: Vec<std::sync::Mutex<Option<Handed>>> = files
            .into_iter()
            .map(|file| {
                std::sync::Mutex::new(Some(Handed {
                    file,
                    facts: FileFacts::default(),
                }))
            })
            .collect();
        let _reads = filer.file_all(&handed, &|slot| {
            // Each slot is one job, taken once, so it is always full here.
            slot.lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .take()
                .map(|one| (one, None))
                .ok_or_else(|| {
                    Refused::at(
                        Cause::open(Stage::File, "file_taken_twice"),
                        failure("file", "a handed file was taken twice".to_owned()),
                    )
                })
        });
        (read, DayExtras::default())
    })
}

/// Says out loud each index of the day whose file is minute-sampled (WP-11c,
/// DB-A04): the day still closes `done` and its bars are filed as printed
/// (the operator's answer: flagged only), so the log is where the import
/// itself names it; the day ledger's own `rows`, `seconds`, `session_s` and
/// `ss_distinct` carry it to `/dayflags.json`. One warning an index, at most
/// the day's indices. The import says it of every day it files; a rescan
/// says it only of the lines it wrote ([`quality::sampled_among`]), so a
/// rescan that finds nothing new repeats nothing (D-4928 item 7).
fn note_sampled(kind: ImportKind, day: Day, sampled: &[quality::Sampled], src: quality::Src) {
    let day = day.to_string();
    for sampled in sampled {
        let _dropped_when_filtered = telemetry::emit(&sampled_event(kind, &day, sampled, src));
    }
}

/// The warning [`note_sampled`] emits for one index of a day: who, when, and
/// the day ledger's own numbers that make it sampled.
fn sampled_event<'a>(
    kind: ImportKind,
    day: &'a str,
    sampled: &'a quality::Sampled,
    src: quality::Src,
) -> telemetry::Event<'a> {
    telemetry::Event::warn(TARGET, "index day is minute-sampled")
        .with("kind", kind.as_str())
        .with("day", day)
        .with("scope", sampled.scope.as_str())
        .with("src", src.as_str())
        .with("rows", sampled.rows)
        .with("seconds", sampled.seconds)
        .with("session_s", sampled.session_s)
        .with("ss_distinct", sampled.ss_distinct)
}

/// The day's quality lines (D-3335), written before its close: a ledger that
/// will not take them is a failure of the day, which then closes
/// `incomplete` and is read again, so its lines are written by the next run.
fn write_flags(
    (run, whole): (&Run<'_>, bool),
    flags: &mut quality::Ledger,
    (day, ledger, member_bytes): (Day, Option<quality::DayLedger>, Option<u64>),
    report: &mut Report,
    failures: &mut Vec<Staged>,
) {
    let Some(mut ledger) = ledger else {
        return;
    };
    ledger.set_member_bytes(member_bytes);
    note_sampled(
        ledger.kind(),
        ledger.day(),
        &ledger.sampled(),
        quality::Src::Import,
    );
    let fresh = ledger.lines(whole);
    let news = flags.news(fresh, quality::Src::Import);
    match flags.append(&news) {
        Ok(()) => report.flags_lines += news.len(),
        Err(why) => failures.push(staged(
            run,
            day,
            "day ledger",
            Stage::Journal,
            &ImportRefusal::DayLedger { why },
        )),
    }
}

/// The runtime: every day of the run through the calendar, the venue, the
/// journal, the source's `read_day` and the common filing path, and each
/// day's quality lines (D-3335) and its files' provenance (D-3334).
///
/// Days run in order. A day's files are handed to its [`DayFiler`], which
/// files them on the run's [`Workers`] and folds them back in listing order
/// (WP-16); every file of the day is written and the day's batch made durable
/// ([`make_durable`], D-3353) before the day's ledger lines, its one census
/// batch and its closing line, in that order.
fn drive_core<F>(
    run: &Run<'_>,
    options: RunOptions<'_>,
    (census, scoped): (&mut dyn Census, Option<(&scope::Record, &scope::Record)>),
    mut read_day: F,
) -> Result<Report, ImportRefusal>
where
    F: FnMut(Day, &mut DayFiler<'_>) -> (DayRead, DayExtras),
{
    // ONE REQUEST FOR THE RUN: its window is the run's range, which is also
    // the range check, its rung the second, its listing the kind's venue. A
    // file, not a vendor request, is the source, so it carries no vendor id.
    // Every venue table holds verified hours from the epoch (`crate::vendor`
    // const-asserts their shipping shape), so the session filter in
    // `from_rows` cannot meet a day with no hours.
    let window = Window::new(run.from, run.to).map_err(|_| ImportRefusal::RangeBackwards {
        from: run.from,
        to: run.to,
    })?;
    let request = BarRequest {
        instrument_id: String::new(),
        listing: run.kind.listing(),
        window,
        granularity: Granularity::Second1,
    };
    run.check_filter()?;
    let journal = Journal::load(journal_path(run.store_root))?;
    let mut flags = quality::Ledger::load(quality::dayflags_path(run.store_root))
        .map_err(|why| ImportRefusal::DayLedger { why })?;
    // THE SCOPE LEDGER (WP-12f, D-4878), read before any day: one that
    // cannot be read refuses the run, as the journal does.
    let scoped = match scoped {
        Some((record, swept)) => Some(Scoped {
            record,
            swept: std::sync::Arc::clone(swept.set()),
            ledger: std::cell::RefCell::new(
                scope::Ledger::load(scope::scopes_path(run.store_root))
                    .map_err(|why| ImportRefusal::Journal { why })?,
            ),
            beyond: std::cell::RefCell::new(HashMap::new()),
        }),
        None => None,
    };
    let filter = run.filter_key();
    let started = options
        .origin
        .map_or_else(now_unix_s, |origin| origin.started);
    let ctx = Ctx {
        run,
        journal: &journal,
        options,
        id: journal.fresh_run(started),
        started,
        announced: std::cell::Cell::new(false),
        scoped,
    };
    note_started(&ctx, &filter);
    note_torn(&ctx);
    let mut report = Report::default();
    let state = (&mut report, &mut flags);
    reach_back(&ctx, &filter, state, &mut *census, &mut read_day)?;
    for day in days(run.from, run.to) {
        let key = format!("{} {day} {filter}", run.kind.as_str());
        let state = (&mut report, &mut flags);
        let asked = Asked {
            key: &key,
            day,
            remembered: &key,
            names: None,
            early: false,
        };
        if widen(&ctx, &request, &asked, state, &mut *census, &mut read_day)? {
            continue;
        }
        if !admit(&ctx, &key, day, &mut report) {
            continue;
        }
        let pass = Pass {
            key: &key,
            day,
            member: None,
        };
        let state = (&mut report, &mut flags);
        day_pass(&ctx, &request, &pass, state, &mut *census, &mut read_day)?;
    }
    note_finished(&ctx, &report);
    Ok(report)
}

/// The days of `from`'s month before it, in order: none when `from` is the
/// month's first day. At most 30.
fn before_range(from: Day) -> Vec<Day> {
    let first = from.days_from_epoch() - u32::from(from.day().saturating_sub(1));
    (first..from.days_from_epoch())
        .filter_map(|n| Day::from_days(n).ok())
        .collect()
}

/// One key that closed a day `done`, asked what it owes this run (WP-12f,
/// D-4878).
#[derive(Clone, Copy)]
struct Asked<'k> {
    /// The base key whose close is asked: `<kind> <day> <filter>`.
    key: &'k str,
    day: Day,
    /// The key the scope ledger remembers the examination under: `key` when
    /// it is the run's own, `<key>/<the run's filter>` when another filter
    /// closed it. `/` is in no symbol, so in no filter and no member.
    remembered: &'k str,
    /// The names of the filter that closed `key`, when it is not the run's
    /// own and names any: `*` names none, which is every one.
    names: Option<&'k Names>,
    /// Whether the day lies before the run's range ([`reach_back`]).
    early: bool,
}

impl Asked<'_> {
    /// Whether the key is the run's own.
    fn own(&self) -> bool {
        self.key == self.remembered
    }
}

/// The names a journal key's filter part lists: none for `*`, which names
/// every one.
fn names_of(filter: &str) -> Option<Names> {
    (filter != "*").then(|| std::sync::Arc::new(filter.split(',').map(str::to_owned).collect()))
}

/// THE RUN'S FIRST MONTH FROM ITS FIRST OWED DAY (D-4878, DB-B76): the days
/// of that month before the range that are closed `done` and owe this run a
/// member are read for it first, in order, so no member's month is made from
/// a later day while an earlier one is still owed. A day no run closed owes
/// nothing and is not read: the run was not asked for it.
///
/// WHOSE CLOSE IS ASKED (repair 2, 12 Oct 2026). A day the run's own key
/// closed is that key's. Any other day is asked of EVERY key that closed it
/// `done`, whatever its filter, the whole-day key `*` among them: such a key
/// owes the members its close's scope never covered that its own filter
/// names, and the run reads the ones its own filter wants, each under that
/// key's member key (`<its filter>+<member>`), so the key's own later run
/// finds them done. Asking only the run's own key let a run under a filter
/// (`cli`'s `--stored`, or one named symbol) make each new member's month
/// from one later day while the earlier days were closed done under `*` for
/// two indices: the 10 Oct shape, unsaid. That such a key was examined for
/// this run's filter is remembered under `<key>/<filter>` and never under
/// the key itself, so its own run still reads the members this filter did
/// not want.
///
/// THOSE DAYS HAVE A REQUEST OF THEIR OWN: a request's window is its range
/// check and a row outside it is dropped, so under the run's own request a
/// day before the range would file nothing and still close `done`.
///
/// At most 30 days: one hash probe for a day no run closed, and for a closed
/// one what [`Ctx::owed`] costs per key that closed it.
fn reach_back<F>(
    ctx: &Ctx<'_>,
    filter: &str,
    (report, flags): (&mut Report, &mut quality::Ledger),
    census: &mut dyn Census,
    read_day: &mut F,
) -> Result<(), ImportRefusal>
where
    F: FnMut(Day, &mut DayFiler<'_>) -> (DayRead, DayExtras),
{
    let run = ctx.run;
    let early = before_range(run.from);
    let (Some(&first), Some(&last)) = (early.first(), early.last()) else {
        return Ok(());
    };
    if ctx.scoped.is_none() {
        return Ok(());
    }
    let reach = BarRequest {
        instrument_id: String::new(),
        listing: run.kind.listing(),
        window: Window::new(first, last).map_err(|_| ImportRefusal::RangeBackwards {
            from: first,
            to: last,
        })?,
        granularity: Granularity::Second1,
    };
    // Each other filter's names, split once a run.
    let mut lenders: HashMap<&str, Option<Names>> = HashMap::new();
    for day in early {
        let of_day = format!("{} {day}", run.kind.as_str());
        let Some(closed_by) = ctx.journal.closed_by.get(&of_day) else {
            continue;
        };
        let own = format!("{of_day} {filter}");
        if ctx.journal.done.contains(&own) {
            let asked = Asked {
                key: &own,
                day,
                remembered: &own,
                names: None,
                early: true,
            };
            let state = (&mut *report, &mut *flags);
            widen(ctx, &reach, &asked, state, &mut *census, read_day)?;
            continue;
        }
        // THE DAY IS COUNTED AND TOLD ONCE, after the last key that closed
        // it, and a member two of them owe is read once.
        let mut widened = Widened::default();
        for lender in closed_by {
            let key = format!("{of_day} {lender}");
            let remembered = format!("{key}/{filter}");
            let names = lenders
                .entry(lender.as_str())
                .or_insert_with(|| names_of(lender));
            let asked = Asked {
                key: &key,
                day,
                remembered: &remembered,
                names: names.as_ref(),
                early: true,
            };
            let state = (&mut *report, &mut *flags);
            widen_key(
                ctx,
                &reach,
                &asked,
                state,
                &mut *census,
                read_day,
                &mut widened,
            )?;
        }
        widened.tell(ctx, day, report);
    }
    Ok(())
}

/// Says, at `warn` and once a run, that the run reads days before its range:
/// `asked` is the earliest key of the range's first month that is closed
/// `done` and owes this run a member the day holds a file of (D-4878,
/// DB-B76). The run was not asked for those days, so it never reads them
/// unsaid; a day it only lists, and reads nothing of, is not one.
fn note_reach_back(ctx: &Ctx<'_>, asked: &Asked<'_>) {
    let (first, from) = (asked.day.to_string(), ctx.run.from.to_string());
    let event = telemetry::Event::warn(
        TARGET,
        "done days before the range owe this scope; they are read first",
    )
    .with("kind", ctx.run.kind.as_str())
    .with("run", ctx.id)
    .with("code", "scope_reach_back")
    .with("first_owed", first.as_str())
    .with("key", asked.key)
    .with("range_from", from.as_str());
    #[cfg(test)]
    tests::summary_emitted(&event);
    let _dropped_when_filtered = telemetry::emit(&event);
}

/// Says, at `warn` and once a run, that the scope ledger holds a torn line:
/// the fragment a killed write left, stepped over by the load and written
/// again whole by the run that needs it (D-4878).
fn note_torn(ctx: &Ctx<'_>) {
    let Some(scoped) = &ctx.scoped else {
        return;
    };
    let torn = scoped.ledger.borrow().torn();
    if torn > 0 {
        let event = telemetry::Event::warn(
            TARGET,
            "the scope ledger holds a torn line; it is stepped over",
        )
        .with("kind", ctx.run.kind.as_str())
        .with("run", ctx.id)
        .with("code", "scope_ledger_torn")
        .with("lines", torn);
        #[cfg(test)]
        tests::summary_emitted(&event);
        let _dropped_when_filtered = telemetry::emit(&event);
    }
}

/// A DAY DONE UNDER A NARROWER SCOPE IS NOT DONE FOR THE MEMBERS ITS CLOSE
/// NEVER COVERED (WP-12f, D-4878): each such member with a file that day is
/// read under a key of its own, `<filter>+<member>`, so its hole never takes
/// the day's other members out of `done`; the base key is not read again.
/// Answers whether the day was such a day, and so is not read whole.
///
/// The day is counted and told ONCE, after its last member key: one "day
/// imported" (or "day incomplete", "day refused": the worst of its member
/// keys' closes) and one more in `days_imported`, when the run closed at
/// least one member key on it. Each member key tells its own close
/// ([`Closed::Member`]).
///
/// A KEY ANOTHER FILTER CLOSED ([`Asked::own`] is false; [`reach_back`]) is
/// read the same way under its own member keys, for the members this run's
/// filter wants. Which those are is the day's listing's to say, also for the
/// member keys that key already has: another run opened them and this run's
/// filter may want none.
fn widen<F>(
    ctx: &Ctx<'_>,
    request: &BarRequest,
    asked: &Asked<'_>,
    (report, flags): (&mut Report, &mut quality::Ledger),
    census: &mut dyn Census,
    read_day: &mut F,
) -> Result<bool, ImportRefusal>
where
    F: FnMut(Day, &mut DayFiler<'_>) -> (DayRead, DayExtras),
{
    let mut widened = Widened::default();
    let state = (&mut *report, &mut *flags);
    let was = widen_key(ctx, request, asked, state, census, read_day, &mut widened)?;
    widened.tell(ctx, asked.day, report);
    Ok(was)
}

/// What the keys of ONE day read again for a wider scope came to, so the day
/// is counted and told once however many keys closed it (D-4878).
#[derive(Default)]
struct Widened {
    /// Where the report stood before the day's first member key.
    before: Option<Mark>,
    /// The member keys closed, and the worst of their closes.
    closed: usize,
    worst: Option<Outcome>,
    /// The members read this day, under whichever key: a member two keys owe
    /// is read once.
    taken: HashSet<String>,
}

impl Widened {
    /// One "day imported" (or "day incomplete", "day refused") and one more
    /// in `days_imported`, when the run closed a member key on the day.
    fn tell(&self, ctx: &Ctx<'_>, day: Day, report: &mut Report) {
        if let (Some(before), Some(worst)) = (self.before, self.worst) {
            report.days_imported += 1;
            let told = (
                worst,
                Closed::Widened {
                    members: self.closed,
                },
            );
            note_day(ctx.run.kind, day, told, (before, report));
        }
    }
}

/// [`widen`] for one key of the day, its closes gathered in `widened`.
fn widen_key<F>(
    ctx: &Ctx<'_>,
    request: &BarRequest,
    asked: &Asked<'_>,
    (report, flags): (&mut Report, &mut quality::Ledger),
    census: &mut dyn Census,
    read_day: &mut F,
    widened: &mut Widened,
) -> Result<bool, ImportRefusal>
where
    F: FnMut(Day, &mut DayFiler<'_>) -> (DayRead, DayExtras),
{
    let (key, day) = (asked.key, asked.day);
    let (examined, members) = match ctx.owed(key, asked.remembered) {
        Owed::Nothing => return Ok(false),
        Owed::Pending(members) if asked.own() => (false, Some(members)),
        Owed::Pending(members) => {
            let pending = std::sync::Arc::new(members.into_iter().collect());
            let present = probe(ctx, asked, request, &pending, report, read_day);
            (false, present)
        }
        Owed::Probe(beyond) => (true, probe(ctx, asked, request, &beyond, report, read_day)),
    };
    let Some(mut members) = members else {
        return Ok(true);
    };
    members.retain(|member| !widened.taken.contains(member));
    report.days_widened += 1;
    if asked.early && !members.is_empty() && widened.taken.is_empty() {
        if report.days_before_range == 0 {
            note_reach_back(ctx, asked);
        }
        report.days_before_range += 1;
    }
    if widened.before.is_none() {
        widened.before = Some(Mark::of(report));
    }
    for member in &members {
        let pass = Pass {
            key: &format!("{key}+{member}"),
            day,
            member: Some(member),
        };
        let state = (&mut *report, &mut *flags);
        if let Some(outcome) = day_pass(ctx, request, &pass, state, &mut *census, read_day)? {
            widened.closed += 1;
            widened.worst = Some(widened.worst.map_or(outcome, |worst| worse(worst, outcome)));
        }
    }
    if examined {
        note_examined(ctx, asked, members.len(), report);
    }
    widened.taken.extend(members);
    Ok(true)
}

/// One key of one day: a base key and every file the run wants, or a member
/// key and that member's files (WP-12f, D-4878).
struct Pass<'p> {
    key: &'p str,
    day: Day,
    member: Option<&'p String>,
}

/// WHICH OF THE MEMBERS A DONE DAY'S CLOSE NEVER COVERED HAVE A FILE THAT DAY
/// (WP-12f, D-4878): the day's listing, read once and nothing of it filed.
/// A member whose own key is already done is left out. `None` when the
/// listing cannot be asked: the source no longer holds the day, or refuses
/// its listing, which is named as a failure of the run and leaves the day
/// unexamined, so the next run asks again.
fn probe<F>(
    ctx: &Ctx<'_>,
    asked: &Asked<'_>,
    request: &BarRequest,
    beyond: &std::sync::Arc<HashSet<String>>,
    report: &mut Report,
    read_day: &mut F,
) -> Option<Vec<String>>
where
    F: FnMut(Day, &mut DayFiler<'_>) -> (DayRead, DayExtras),
{
    let (key, day) = (asked.key, asked.day);
    let (read, extras) = {
        let mut filer = DayFiler::of_day(ctx, (key, day), request, report);
        filer.ask = Ask::Probe(std::sync::Arc::clone(beyond), asked.names.cloned());
        read_day(day, &mut filer)
    };
    if !read.held || !read.refused.is_empty() {
        let why = read.refused.first().map_or_else(
            || {
                "the journal closed this day done and the source holds no file of it now, \
                 so the members a wider scope added cannot be read"
                    .to_owned()
            },
            |refused| refused.why.clone(),
        );
        let cause = Cause::open(Stage::Vendor, "scope_reopen_unreadable");
        report
            .failures
            .push(note_failure(ctx.run.kind, day, "listing", &why, cause));
        return None;
    }
    Some(
        extras
            .present
            .into_iter()
            .filter(|member| !ctx.settled(&format!("{key}+{member}")))
            .collect(),
    )
}

/// Records that `asked`'s key was examined under this run's scope, once
/// every member key it opened is closed, and says so at `info`: under the key
/// itself when it is the run's own, under [`Asked::remembered`] when another
/// filter closed it. A ledger that will not take the line is a named failure
/// of the run; the day is then examined again by the next run, which finds
/// its member keys done and reads no file.
fn note_examined(ctx: &Ctx<'_>, asked: &Asked<'_>, opened: usize, report: &mut Report) {
    let Some(scoped) = &ctx.scoped else {
        return;
    };
    let (key, day) = (asked.remembered, asked.day);
    report.members_reopened += opened;
    let digest = scoped.record.digest();
    // The run's `run` line is in the journal before the ledger names its id:
    // a run that opened no member key wrote no other journal line, and its
    // id was then one the next run could take as its own.
    let written = ctx
        .announce()
        .map_err(|why| why.to_string())
        .and_then(|()| {
            let mut ledger = scoped.ledger.borrow_mut();
            ledger
                .record(ctx.run.kind.as_str(), scoped.record)
                .and_then(|()| ledger.examine(key, digest, (ctx.id, opened)))
        });
    let day_text = day.to_string();
    match written {
        Ok(()) => {
            let _dropped_when_filtered = telemetry::emit(
                &telemetry::Event::info(TARGET, "a done day was read again for a wider scope")
                    .with("kind", ctx.run.kind.as_str())
                    .with("day", day_text.as_str())
                    .with("key", key)
                    .with("scope", digest)
                    .with("members_reopened", opened),
            );
        }
        Err(why) => {
            let cause = Cause::open(Stage::Journal, "scope_ledger");
            report
                .failures
                .push(note_failure(ctx.run.kind, day, "scope ledger", &why, cause));
        }
    }
}

/// One key of one day through the source's `read_day` and the common filing
/// path: its files written, the day's batch made durable, its ledger lines,
/// its census batch and its closing line, in that order.
fn day_pass<F>(
    ctx: &Ctx<'_>,
    request: &BarRequest,
    pass: &Pass<'_>,
    (report, flags): (&mut Report, &mut quality::Ledger),
    census: &mut dyn Census,
    read_day: &mut F,
) -> Result<Option<Outcome>, ImportRefusal>
where
    F: FnMut(Day, &mut DayFiler<'_>) -> (DayRead, DayExtras),
{
    let (run, key, day) = (ctx.run, pass.key, pass.day);
    // A request's window is its range check: a day outside it would have
    // every row dropped and still close.
    debug_assert!(
        request.window.contains(day),
        "{day} is read under a request that does not hold it"
    );
    if pass.member.is_some() {
        let retried = ctx.options.retry_refused && ctx.journal.with_refusals.contains(key);
        name_retried(ctx.journal, run.kind, (key, day, retried), report);
    }
    let before = Mark::of(report);
    let mut files_of_day = DayFiler::of_day(ctx, (key, day), request, report);
    if let Some(member) = pass.member {
        files_of_day.ask = Ask::One(member.clone());
    }
    let left_open = files_of_day.confirm;
    let (read, extras) = read_day(day, &mut files_of_day);
    let DayFiler {
        held,
        mut failures,
        max_back_s: day_back,
        ledger,
        batch,
        confirmed,
        ..
    } = files_of_day;
    // EVERY FILE OF THE DAY IS WRITTEN; NOW IT IS MADE DURABLE, ONCE, and
    // only then are its day-ledger lines, its census batch and its
    // closing line written (WP-16, D-3353). A day whose writes cannot be
    // made durable stops the run with the day left open, so the next run
    // reads it again and confirms what this one wrote.
    if let Some(batch) = batch
        && let Err(why) = make_durable(&batch, (run.kind, day), left_open.then_some(confirmed))
    {
        return Err(stop_at_durability(
            ctx,
            day,
            (before, std::mem::take(report)),
            failures,
            &why,
        ));
    }
    let (held_day, member_bytes) = (read.held, extras.member_bytes);
    take_read((run.kind, day), (read, extras), report, &mut failures);
    if !held_day {
        close_missing(ctx, (key, day), report, failures);
        return Ok(None);
    }
    // A member key's close never writes the whole-day line of the day
    // ledger: that line is the base close's (D-4878, D-3335).
    write_flags(
        (run, run.only.is_empty() && pass.member.is_none()),
        flags,
        (day, ledger, member_bytes),
        report,
        &mut failures,
    );
    if let Some(why) = census.publish(run.store_root, &held) {
        let room = census.room(run.store_root, &held);
        return Err(stop_at_census(
            ctx,
            (key, day),
            (before, std::mem::take(report), day_back),
            failures,
            (why, room),
        ));
    }
    // A DAY IS COUNTED ONCE: here for a day read whole, and in [`widen`] for
    // a day read again for a wider scope, after its last member key.
    let closed = match pass.member {
        None => {
            report.days_imported += 1;
            Closed::Day
        }
        Some(member) => Closed::Member { member, key },
    };
    let outcome = close_day(ctx, (key, day), (before, report, day_back), &mut failures);
    record_close(run.kind, (day, outcome, closed), (before, report), failures);
    Ok(Some(outcome))
}

/// Makes the day's batch durable (WP-16, D-3353), on [`FLUSH_THREADS`], and
/// says so at `info`, once a day beside "day imported": what it held, the
/// flushes it took and how long they took, so the cost of a day's durability
/// is in the log the operator reads.
fn make_durable(
    batch: &store::file::DurableBatch,
    (kind, day): (ImportKind, Day),
    left_open: Option<usize>,
) -> Result<(), store::file::StoreError> {
    let started = std::time::Instant::now();
    let made = batch.make_durable_by(FLUSH_THREADS, &|file, path| {
        #[cfg(test)]
        tests::before_a_day_flush(path)?;
        let _ = path;
        file.sync_all()
    })?;
    let day_text = day.to_string();
    let ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    // A DAY AN EARLIER RUN LEFT OPEN SAYS SO, at `warn`: what that run wrote
    // and never made durable was flushed for itself here, and how much.
    if let Some(months) = left_open {
        let _dropped_when_filtered = telemetry::emit(
            &telemetry::Event::warn(
                TARGET,
                "an earlier run left this day open; its months were confirmed durable",
            )
            .with("kind", kind.as_str())
            .with("day", day_text.as_str())
            .with("months", months)
            .with("files", made.confirmed),
        );
    }
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::info(TARGET, "day made durable")
            .with("kind", kind.as_str())
            .with("day", day_text.as_str())
            .with("files", made.files)
            .with("directories", made.directories)
            .with("confirmed", made.confirmed)
            .with("flushes", made.flushes)
            .with("ms", ms),
    );
    Ok(())
}

/// A day whose writes could not be made durable (WP-16, D-3353): named in
/// the report, said at `error`, and the run stopped there. The day is left
/// begun and not closed, so the next run reads it again; nothing of its
/// ledger, census or closing line is written over writes that are not
/// durable.
fn stop_at_durability(
    ctx: &Ctx<'_>,
    day: Day,
    (before, mut report): (Mark, Report),
    failures: Vec<Staged>,
    why: &store::file::StoreError,
) -> ImportRefusal {
    let kind = ctx.run.kind;
    let words = format!(
        "the day's writes could not be made durable, so nothing of its ledger, census or \
         closing line was written and the day stays open: {why}"
    );
    let day_text = day.to_string();
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::error(TARGET, "day not made durable; run stopped")
            .with("kind", kind.as_str())
            .with("day", day_text.as_str())
            .with("why", words.as_str())
            .with("days_imported", report.days_imported)
            .with("files", report.files - before.files),
    );
    report
        .failures
        .extend(failures.into_iter().map(|one| one.failure));
    report
        .failures
        .push(failure_on(day, &format!("{day} durability"), words.clone()));
    ImportRefusal::Durability {
        kind,
        day,
        why: words,
        done: Box::new(report),
    }
}

/// Counts what a source did with a day beside its files: the entries it
/// skipped, the ones it refused (each a failure of the day, after the files'
/// own), and the lineage it wrote and the failures that are no file's.
fn take_read(
    (kind, day): (ImportKind, Day),
    (read, extras): (DayRead, DayExtras),
    report: &mut Report,
    failures: &mut Vec<Staged>,
) {
    report.files_skipped += read.skipped;
    report.files_refused += read.refused.len();
    for refused in read.refused {
        let Refused { cause, failure } = refused;
        let failure = note_failure(kind, day, &failure.instrument, &failure.why, cause);
        failures.push(Staged { cause, failure });
    }
    report.lineage_lines += extras.lineage.lineage;
    report.lot_lines += extras.lineage.lots;
    report.adjustments += extras.lineage.actions;
    failures.extend(extras.failures);
}

/// Files a closed day in the report, by how it closed, with its failures, and
/// logs its one summary event ([`note_day`]).
fn record_close(
    kind: ImportKind,
    (day, outcome, closed): (Day, Outcome, Closed<'_>),
    (before, report): (Mark, &mut Report),
    failures: Vec<Staged>,
) {
    // A DAY IS NAMED ONCE however many of its member keys closed that way
    // (D-4878): the days of a run ascend, so the day is the list's last.
    let named = |days: &mut Vec<Day>| {
        if days.last() != Some(&day) {
            days.push(day);
        }
    };
    match outcome {
        Outcome::Done => {}
        Outcome::DoneWithRefusals => named(&mut report.closed_with_refusals),
        Outcome::Incomplete => named(&mut report.closed_incomplete),
        Outcome::Refused => named(&mut report.closed_refused),
    }
    report
        .failures
        .extend(failures.into_iter().map(|one| one.failure));
    note_day(kind, day, (outcome, closed), (before, report));
}

/// "run finished", with how its days closed (D-3317).
fn note_finished(ctx: &Ctx<'_>, report: &Report) {
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::info(TARGET, "run finished")
            .with("kind", ctx.run.kind.as_str())
            .with("run", ctx.id)
            .with("days", report.days_imported)
            .with("skipped", report.days_skipped)
            .with("missing", report.days_missing.len())
            .with("done_with_refusals", report.closed_with_refusals.len())
            .with("incomplete", report.closed_incomplete.len())
            .with("refused", report.closed_refused.len() + report.days_refused)
            .with("files", report.files)
            .with("seconds", report.seconds)
            .with("failures", report.failures.len()),
    );
}

/// Whether a day is read to import it or rescanned for its quality only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// The import: the closing-auction prints are filed in their sidecar.
    Import,
    /// `cli gdfl-dayflags`: nothing under `bars/` is written, not even an
    /// auction sidecar.
    Rescan,
}

/// The run over a capital-market source (indices or stocks), its
/// provenance `unrecorded` ([`run_cm_from`] records it).
///
/// # Errors
///
/// As [`drive`].
pub fn run_cm<S>(source: &S, run: &Run<'_>) -> Result<Report, ImportRefusal>
where
    S: CmSource + Sync,
    S::Locator: Sync,
{
    run_cm_from(source, run, RunOptions::default())
}

/// [`run_cm`] with the run's [`RunOptions`]: the origin the caller measured
/// (D-3317; DB-G56), whether to read again the days done with refusals, and
/// the workers a day's files are read and filed on (WP-16).
///
/// # Errors
///
/// As [`drive`].
pub fn run_cm_from<S>(
    source: &S,
    run: &Run<'_>,
    options: RunOptions<'_>,
) -> Result<Report, ImportRefusal>
where
    S: CmSource + Sync,
    S::Locator: Sync,
{
    let tree = match run.kind {
        ImportKind::Indices => CmKind::Indices,
        ImportKind::Stocks => CmKind::Stocks,
        ImportKind::Options => {
            return Err(ImportRefusal::KindUnknown {
                word: "options read from a capital-market source".to_owned(),
            });
        }
    };
    // THE RUN'S SCOPE, RECORDED WITH EVERY CLOSE (WP-12f, D-4878): the
    // members the source's listings file, and the swept surface a close from
    // before scopes were recorded is read as built under.
    let record = scope::Record::of(source.scope().members(tree));
    let swept = scope::Record::of(crate::gdfl_cm::CmScope::swept().members(tree));
    // A day read again for several members lists once: the listing of the
    // day in hand is kept until another day is asked for.
    let mut cached: Option<(Day, crate::gdfl_cm::DayListing<S::Locator>)> = None;
    let scoped = Some((&record, &swept));
    drive_core(run, options, (&mut StoreCensus, scoped), |day, filer| {
        if cached.as_ref().is_none_or(|(held, _)| *held != day) {
            cached = None;
            let (listed, listing) = cm_list(source, (tree, day));
            match listing {
                Some(listing) => cached = Some((day, listing)),
                None => return listed.close(Vec::new()).0,
            }
        }
        let Some((_, listing)) = cached.as_ref() else {
            return Listed::default().close(Vec::new()).0;
        };
        let mut listed = Listed {
            held: true,
            member_bytes: listing.member_bytes(),
            ..Listed::default()
        };
        let member = match &filer.ask {
            Ask::All => None,
            Ask::One(member) => Some(member.clone()),
            Ask::Probe(wanted, lender) => {
                let (read, mut extras) = listed.close(Vec::new()).0;
                extras.present = cm_present((run, lender.as_deref()), listing, wanted);
                return (read, extras);
            }
        };
        let jobs = cm_jobs(run, listing, &mut listed, member.as_deref());
        let reads = filer.file_all(&jobs, &|job| {
            cm_read(source, run, (listing, day), job, Mode::Import).map(|one| (one, None))
        });
        listed
            .close(jobs.iter().map(|job| job.index).zip(reads).collect())
            .0
    })
}

/// What a day's listing decided before any file of it is read (WP-16): the
/// entries it skipped, the ones it refused by name, each at its place in the
/// listing, and the day zip's size. The wanted files are its jobs.
#[derive(Debug, Default)]
struct Listed {
    /// Whether the source holds the day at all.
    held: bool,
    /// Entries of the day that are not wanted files.
    skipped: usize,
    /// The refusals the listing decided, in listing order, each at its place.
    refused: Vec<(usize, Refused)>,
    /// The day zip's size, where the source read one.
    member_bytes: Option<u64>,
}

impl Listed {
    /// The day as its source reports it once its jobs are read: `reads` is
    /// each job's read at its place in the listing, in listing order. The
    /// listing's refusals and the readers' are merged in listing order, the
    /// order one pass over the listing named them in, in one linear merge of
    /// the two ordered lists. Answers the lineage's contract-days too, in
    /// listing order.
    fn close(
        self,
        reads: Vec<(usize, JobRead)>,
    ) -> ((DayRead, DayExtras), Vec<lineage::DayContract>) {
        let mut contracts = Vec::new();
        let mut refused = Vec::with_capacity(self.refused.len().saturating_add(reads.len()));
        let mut listed = self.refused.into_iter().peekable();
        for (at, read) in reads {
            while let Some((_, one)) = listed.next_if(|(place, _)| *place < at) {
                refused.push(one);
            }
            match read {
                JobRead::Read(contract) => contracts.extend(contract),
                JobRead::Refused(one) => refused.push(one),
            }
        }
        refused.extend(listed.map(|(_, one)| one));
        let read = DayRead {
            held: self.held,
            skipped: self.skipped,
            refused,
        };
        let extras = DayExtras {
            member_bytes: self.member_bytes,
            ..DayExtras::default()
        };
        ((read, extras), contracts)
    }
}

/// One wanted capital-market file of a day: its place in the listing, the
/// listed file, the key it is filed under and its symbol.
struct CmJob<'a, L> {
    index: usize,
    file: &'a crate::gdfl_cm::ListedFile<L>,
    key: brutex_core::instrument::InstrumentKey,
    symbol: String,
}

/// The listing of tree `tree` on `day`: none when the source does not hold
/// the day, and the listing refused by name when it cannot be read.
fn cm_list<S: CmSource>(
    source: &S,
    (tree, day): (CmKind, Day),
) -> (Listed, Option<crate::gdfl_cm::DayListing<S::Locator>>) {
    let mut listed = Listed::default();
    match source.day(tree, day) {
        Ok(Some(listing)) => {
            listed.held = true;
            listed.member_bytes = listing.member_bytes();
            (listed, Some(listing))
        }
        Ok(None) => (listed, None),
        Err(why) => {
            listed.held = true;
            listed.refused.push((0, Refused::cm("listing", &why)));
            (listed, None)
        }
    }
}

/// Every swept ticker's file of `listing`, once, in listing order: what the
/// day reads. Every other entry is a skip, or a refusal by name where the run
/// wants the series it would have been filed under (D-2814: every entry is a
/// file, a skip or a refusal).
fn cm_jobs<'a, L>(
    run: &Run<'_>,
    listing: &'a crate::gdfl_cm::DayListing<L>,
    listed: &mut Listed,
    member: Option<&str>,
) -> Vec<CmJob<'a, L>> {
    let mut seen = HashSet::with_capacity(listing.entries().len());
    let mut jobs = Vec::with_capacity(listing.entries().len());
    let folder = format!("{}/", listing.folder());
    for (index, file) in listing.entries().iter().enumerate() {
        let stem = file
            .entry
            .strip_prefix(&folder)
            .and_then(|name| split_name(name).ok())
            .map(|(stem, _)| stem);
        // A member key reads its one member's files (WP-12f, D-4878). Every
        // other entry is the base close's, counted there and not again.
        if let Some(asked) = member
            && stem.and_then(|stem| listing.member(stem)).as_deref() != Some(asked)
        {
            continue;
        }
        // What the listing's scope files the stem under this day (D-3333):
        // the swept surface under the default scope, as `resolve` answers.
        let filed = stem.map(|stem| listing.filing(stem));
        let key = match filed {
            Some(Ok(Some(key))) => key,
            // A stem refused for its symbol is refused only where the run
            // wants the series it would have been filed under.
            Some(Err(why)) if why.filed_as().is_none_or(|symbol| run.wants(symbol)) => {
                listed.refused.push((index, Refused::cm(file.name(), &why)));
                continue;
            }
            Some(Ok(None) | Err(_)) | None => {
                listed.skipped += 1;
                continue;
            }
        };
        // A second name for one ticker is a skip: its first name was refused
        // as twins, so every entry is a file, a skip or a refusal (D-2814).
        if !seen.insert(key) {
            listed.skipped += 1;
            continue;
        }
        let symbol = key.underlying.as_str().to_owned();
        if !run.wants(&symbol) {
            listed.skipped += 1;
            continue;
        }
        jobs.push(CmJob {
            index,
            file,
            key,
            symbol,
        });
    }
    jobs
}

/// Which of `wanted`'s members `listing` holds a file of that `run` wants,
/// each once, in listing order (WP-12f, D-4878): one pass over the day's
/// entries, a member name and one hash probe each. Nothing is read.
///
/// The run's filter decides here as it does for a base key ([`cm_jobs`]): a
/// member whose file is filed under a symbol the filter does not name is not
/// the run's to read, so a filtered key never opens a member key that would
/// close with no file.
///
/// `lender` is the names of the filter that closed the day, when another
/// filter than the run's did and it names any: that close was asked for
/// those names alone, so it owes no member filed under another.
fn cm_present<L>(
    (run, lender): (&Run<'_>, Option<&HashSet<String>>),
    listing: &crate::gdfl_cm::DayListing<L>,
    wanted: &HashSet<String>,
) -> Vec<String> {
    let asks =
        |symbol: &str| run.wants(symbol) && lender.is_none_or(|names| names.contains(symbol));
    let folder = format!("{}/", listing.folder());
    let mut seen = HashSet::new();
    let mut present = Vec::new();
    for file in listing.entries() {
        let stem = file
            .entry
            .strip_prefix(&folder)
            .and_then(|name| split_name(name).ok())
            .map(|(stem, _)| stem);
        let Some((stem, member)) =
            stem.and_then(|stem| listing.member(stem).map(|member| (stem, member)))
        else {
            continue;
        };
        if !wanted.contains(&member) {
            continue;
        }
        let asked = match listing.filing(stem) {
            Ok(Some(key)) => asks(key.underlying.as_str()),
            Ok(None) => false,
            Err(why) => why.filed_as().is_none_or(asks),
        };
        if asked && seen.insert(member.clone()) {
            present.push(member);
        }
    }
    present
}

/// Says, once per file, that a trade-for-trade series file was filed under
/// its share's key (WP-12f, DB-B06), so the day's series is on the log
/// beside the file's provenance record, which names the entry itself.
fn note_series(day: Day, symbol: &str, name: &str) {
    let Some(series) = crate::gdfl_cm::series_file(name) else {
        return;
    };
    let day_text = day.to_string();
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::info(
            TARGET,
            "a trade-for-trade series file is filed under its share's key",
        )
        .with("kind", ImportKind::Stocks.as_str())
        .with("day", day_text.as_str())
        .with("symbol", symbol)
        .with("series", series)
        .with("file", name)
        .with("code", "series_filed"),
    );
}

/// Reads one wanted capital-market file: fetched, checked and decoded, and
/// handed on with its member and its reader's counts. A key resolved from the
/// listing always has its file there, so a `None` is refused by name rather
/// than counted as skipped. The closing auction's prints (D-3320) go to the
/// month's sidecar before the trades are filed, and a file whose prints it
/// refuses is refused whole, so every entry stays one file, skip or refusal.
/// A rescan files nothing, so it never writes a sidecar. Runs on the worker
/// that files the share (WP-16), so one share's `.auc` has one writer.
fn cm_read<S: CmSource>(
    source: &S,
    run: &Run<'_>,
    (listing, day): (&crate::gdfl_cm::DayListing<S::Locator>, Day),
    job: &CmJob<'_, S::Locator>,
    mode: Mode,
) -> Result<Handed, Refused> {
    let read_file = read_listed(source, listing, &job.key)
        .and_then(|f| f.ok_or(CmRefusal::NotSwept))
        .and_then(|day_file| match mode {
            Mode::Import => crate::gdfl_cm::auction::append(
                run.store_root,
                &job.key,
                day,
                &day_file.file.auction,
            )
            .map(|_| day_file),
            Mode::Rescan => Ok(day_file),
        });
    let day_file = read_file.map_err(|why| Refused::cm(job.file.name(), &why))?;
    if mode == Mode::Import {
        note_series(day, &job.symbol, &day_file.name);
    }
    let decoded = &day_file.file;
    let reader = quality::ReaderCounts {
        rows_refused: u64::from(decoded.refused.total()),
        zero_ltp_rows: u64::from(decoded.refused.zero_ltp),
        dual: decoded.dual_spread,
        auction_prints: u64::try_from(decoded.auction.len()).unwrap_or(u64::MAX),
        quoted_rows: None,
    };
    let ticks = decoded
        .rows
        .iter()
        .map(|row| Tick {
            sod: row.sod,
            ltp: row.ltp,
            ltq: row.ltq,
            oi: None,
        })
        .collect();
    Ok(Handed {
        file: TickFile {
            symbol: job.symbol.clone(),
            contract: None,
            name: day_file.name,
            ticks,
        },
        facts: FileFacts {
            member: Some(Member {
                entry: job.file.entry.to_string(),
                bytes: job.file.len,
                crc32: job.file.crc32,
            }),
            reader,
        },
    })
}

/// One capital-market day as a bare reader hands it on: [`cm_day_with`] for
/// the import, its files' facts dropped.
#[cfg(test)]
fn cm_day<S: CmSource>(
    source: &S,
    run: &Run<'_>,
    tree: CmKind,
    day: Day,
    sink: &mut dyn FnMut(TickFile),
) -> DayRead {
    cm_day_with(
        source,
        run,
        (tree, day),
        &mut |handed| sink(handed.file),
        Mode::Import,
    )
    .0
}

/// One capital-market day, one file after another on this thread: every
/// swept ticker's file, once, in listing order, each handed on with its
/// member and its reader's counts. The rescan's door, and the one the
/// import's workers share every step of ([`cm_jobs`], [`cm_read`]).
fn cm_day_with<S: CmSource>(
    source: &S,
    run: &Run<'_>,
    (tree, day): (CmKind, Day),
    sink: &mut dyn FnMut(Handed),
    mode: Mode,
) -> (DayRead, DayExtras) {
    let (mut listed, listing) = cm_list(source, (tree, day));
    let Some(listing) = listing else {
        return listed.close(Vec::new()).0;
    };
    let jobs = cm_jobs(run, &listing, &mut listed, None);
    let mut reads = Vec::with_capacity(jobs.len());
    for job in &jobs {
        let read = match cm_read(source, run, (&listing, day), job, mode) {
            Ok(handed) => {
                sink(handed);
                JobRead::Read(None)
            }
            Err(refused) => JobRead::Refused(refused),
        };
        reads.push((job.index, read));
    }
    listed.close(reads).0
}

/// The run over an options source, its provenance `unrecorded`
/// ([`run_nfo_from`] records it).
///
/// # Errors
///
/// As [`drive`], and [`ImportRefusal::KindUnknown`] for a spot kind.
pub fn run_nfo<S>(source: &S, run: &Run<'_>) -> Result<Report, ImportRefusal>
where
    S: NfoSource + Sync,
    S::Locator: Sync,
{
    run_nfo_from(source, run, RunOptions::default())
}

/// [`run_nfo`] with the run's [`RunOptions`] (D-3317; DB-G56), as
/// [`run_cm_from`]. Every day's option files also feed the corporate-action
/// lineage (D-3331, D-3343; [`lineage`]), whose ledgers are read before the
/// first day. The lineage is folded on the coordinating thread, from the
/// contract-days the workers read, in listing order (WP-16).
///
/// # Errors
///
/// As [`run_nfo`], and [`ImportRefusal::Lineage`] when a lineage ledger
/// exists and cannot be read; [`ImportRefusal::FilterRenamed`] for a filter
/// name the lineage files under another key, and
/// [`ImportRefusal::UnderlyingSplit`] while the store holds option series
/// under an old symbol whose files the run would file elsewhere (D-3354),
/// both before any day is read.
pub fn run_nfo_from<S>(
    source: &S,
    run: &Run<'_>,
    options: RunOptions<'_>,
) -> Result<Report, ImportRefusal>
where
    S: NfoSource + Sync,
    S::Locator: Sync,
{
    if run.kind != ImportKind::Options {
        return Err(ImportRefusal::KindUnknown {
            word: format!("{} read from an options source", run.kind.as_str()),
        });
    }
    // BEFORE ANYTHING IS READ OR JOURNALED (D-3354): a name the lineage files
    // elsewhere, then an old-symbol directory the run would split from. Each
    // is logged as well as returned: a refused run leaves a line to find.
    let refused = match run.check_filter().and_then(|()| run.split_underlyings()) {
        Ok(held) if held.is_empty() => None,
        Ok(held) => Some(ImportRefusal::UnderlyingSplit { held }),
        Err(refusal) => Some(refusal),
    };
    if let Some(refusal) = refused {
        let why = refusal.to_string();
        let _dropped_when_filtered = telemetry::emit(
            &telemetry::Event::error(TARGET, "run refused")
                .with("kind", run.kind.as_str())
                .with("code", refusal.code())
                .with("why", why.as_str()),
        );
        return Err(refusal);
    }
    let mut lineage = lineage::LineageRun::load(run.store_root, run.from)
        .map_err(|why| ImportRefusal::Lineage { why })?;
    drive_core(run, options, (&mut StoreCensus, None), |day, filer| {
        let mut early = DayExtras::default();
        let workers = options.workers.get();
        warm_up(
            source,
            run,
            day,
            &mut lineage,
            (&mut early, &|listing, wanted| {
                warm_files_on(source, workers, listing, wanted)
            }),
        );
        let (mut listed, listing) = nfo_list(source, day);
        let Some(listing) = listing else {
            return with_early(listed.close(Vec::new()).0, early);
        };
        let jobs = nfo_jobs(run, day, &listing, &mut listed);
        let reads = filer.file_all(&jobs, &|job| nfo_read(source, &listing, job, true));
        let ((read, mut extras), contracts) =
            listed.close(jobs.iter().map(|job| job.index).zip(reads).collect());
        extras.absorb(early);
        fold_lineage(
            source,
            run,
            (day, &listing),
            &mut lineage,
            &contracts,
            &mut extras,
        );
        (read, extras)
    })
}

/// A day's read with what was gathered before its listing (the lineage's
/// warm-up) folded into its extras.
fn with_early((read, mut extras): (DayRead, DayExtras), early: DayExtras) -> (DayRead, DayExtras) {
    extras.absorb(early);
    (read, extras)
}

/// The longest name of `names` that `ticker` starts with.
fn longest_prefix<'a>(ticker: &str, names: impl IntoIterator<Item = &'a str>) -> Option<&'a str> {
    let mut best: Option<&'a str> = None;
    for name in names {
        if ticker.starts_with(name) && best.is_none_or(|b| name.len() > b.len()) {
            best = Some(name);
        }
    }
    best
}

/// Whether a filtered run wants a name that does not decode (D-2815): when
/// the longest underlying it starts with, among the F&O underlyings AND the
/// filter's own names, is a filter name. `NIFTYNXT50...` is `NIFTYNXT50`'s,
/// never `NIFTY`'s; `TV18BRDCST...`, a share outside today's F&O list, is a
/// `TV18BRDCST` filter's, which asked for it by name.
///
/// A name whose own shape spells its underlying (two digits and a month
/// before a tail with no letter) is that underlying's alone, by exact bytes:
/// `LTI06APR24...` is `LTI`'s, never `LT`'s, though `LTI` is no longer in
/// the F&O list and `LT` is (D-2818, D-2823). The longest-prefix rule is
/// only for a name with no such shape. O(213 + filter names) per
/// undecodable name, on the refusal path only.
fn claims_undecodable(run: &Run<'_>, ticker: &str) -> bool {
    if let Some(underlying) = crate::gdfl_nfo::shaped_underlying(ticker) {
        // By the key it would be FILED under (D-3354), as a name that does
        // decode is matched: an undecodable `NIITTECH...` is a `COFORGE`
        // filter's to refuse, never nobody's.
        return run.wants(brutex_core::alias::LINEAGE.current(underlying));
    }
    let fno = longest_prefix(ticker, brutex_core::universe::FNO_UNDERLYINGS);
    let asked = longest_prefix(ticker, run.only.iter().map(String::as_str));
    asked.is_some_and(|asked| fno.is_none_or(|fno| asked.len() >= fno.len()))
}

/// The `(second of day, OI)` rows of the file at `index` of `listing`, read
/// again for a renamed contract whose OI crossed unadjusted (D-3331).
fn reread_oi<S: NfoSource>(
    source: &S,
    listing: &crate::gdfl_nfo::NfoDay<S::Locator>,
    index: usize,
) -> Result<Vec<(u32, i64)>, String> {
    let file = listing
        .entries()
        .get(index)
        .ok_or_else(|| format!("no entry {index} in the listing of {}", listing.day()))?;
    read_file(source, listing, file)
        .map(|day_file| {
            day_file
                .file
                .rows
                .iter()
                .map(|row| (row.sod, row.oi))
                .collect()
        })
        .map_err(|why| format!("{}: {why}", file.entry))
}

/// The contract-day of a read option file, for the lineage.
fn contract_day_of(day_file: &crate::gdfl_nfo::NfoDayFile, file: usize) -> lineage::DayContract {
    lineage::DayContract {
        ticker: day_file.ticker.clone(),
        day: crate::gdfl_lineage::ContractDay::from_rows(
            &day_file.ticker,
            day_file
                .file
                .rows
                .iter()
                .map(|row| (row.sod, row.ltq, row.oi)),
        ),
        file,
    }
}

/// One read option file as the runtime receives it: its rows, its member
/// and its quote count (D-3334, D-3335).
fn option_handed<L>(
    day_file: crate::gdfl_nfo::NfoDayFile,
    file: &crate::gdfl_cm::ListedFile<L>,
) -> Handed {
    let reader = quality::ReaderCounts {
        quoted_rows: Some(u64::from(day_file.file.quoted_rows)),
        ..quality::ReaderCounts::default()
    };
    let ticks = day_file
        .file
        .rows
        .iter()
        .map(|row| Tick {
            sod: row.sod,
            ltp: row.ltp,
            ltq: row.ltq,
            oi: Some(row.oi),
        })
        .collect();
    Handed {
        file: TickFile {
            symbol: day_file.ticker.underlying.as_str().to_owned(),
            contract: Some(day_file.ticker.contract),
            name: day_file.name,
            ticks,
        },
        facts: FileFacts {
            member: Some(Member {
                entry: file.entry.to_string(),
                bytes: file.len,
                crc32: file.crc32,
            }),
            reader,
        },
    }
}

/// Folds `day`'s option `contracts`, read from `listing`, into `lineage`: a
/// ledger that will not take the records, or a renamed contract's file that
/// will not read again, is a failure of `run`'s day, so the day stays open
/// and the next run writes them.
fn fold_lineage<S: NfoSource>(
    source: &S,
    run: &Run<'_>,
    (day, listing): (Day, &crate::gdfl_nfo::NfoDay<S::Locator>),
    lineage: &mut lineage::LineageRun,
    contracts: &[lineage::DayContract],
    extras: &mut DayExtras,
) {
    match lineage.day(day, contracts, &mut |index| {
        reread_oi(source, listing, index)
    }) {
        Ok(written) => extras.lineage.absorb(written),
        Err(why) => extras.failures.push(staged(
            run,
            day,
            "lineage",
            Stage::Journal,
            &ImportRefusal::Lineage { why },
        )),
    }
}

/// Folds into `lineage`, read-only, the open sessions just before `day` it
/// holds no state for ([`lineage::warmup_days`]), so an adjustment on `day`
/// is detected against the contracts that traded before it. Files the run
/// does not want are not read; a file named twice on a day is left out, as
/// the import refuses it; a day or a file the source refuses is logged and
/// passed over, never a failure of `day` (the import of that day names it).
///
/// `read` reads the wanted files of one warm-up day, each at its place in
/// the listing, and answers the contract-day of each one it read, in listing
/// order: one after another for a rescan, on the run's workers for an import
/// (WP-16: three sessions of every option file, read again at every run's
/// start, took 20 s on one thread for 2020-09-15).
fn warm_up<S: NfoSource>(
    source: &S,
    run: &Run<'_>,
    day: Day,
    lineage: &mut lineage::LineageRun,
    (extras, read): (&mut DayExtras, WarmRead<'_, S::Locator>),
) {
    for prev in lineage::warmup_days(day, lineage.last_day()) {
        let listing = match source.day(prev) {
            Ok(Some(listing)) => listing,
            Ok(None) => continue,
            Err(why) => {
                let prev_text = prev.to_string();
                let why = why.to_string();
                let _dropped_when_filtered = telemetry::emit(
                    &telemetry::Event::warn(TARGET, "lineage warm-up day unread")
                        .with("day", prev_text.as_str())
                        .with("why", why.as_str()),
                );
                continue;
            }
        };
        let wanted: Vec<(usize, &crate::gdfl_cm::ListedFile<S::Locator>)> = listing
            .entries()
            .iter()
            .enumerate()
            .filter(|(_, file)| {
                listing
                    .ticker_of(file)
                    .and_then(|ticker| decode_ticker(ticker, prev).ok())
                    .is_some_and(|decoded| run.wants(decoded.underlying.as_str()))
            })
            .collect();
        let mut contracts = read(&listing, &wanted);
        let mut named: HashMap<Contract, usize> = HashMap::with_capacity(contracts.len());
        for c in &contracts {
            *named.entry(c.ticker.contract).or_default() += 1;
        }
        contracts.retain(|c| named.get(&c.ticker.contract) == Some(&1));
        // A failure here is the warm-up day's, and keeps `day` open too.
        fold_lineage(source, run, (prev, &listing), lineage, &contracts, extras);
        let prev_text = prev.to_string();
        let _dropped_when_filtered = telemetry::emit(
            &telemetry::Event::info(TARGET, "lineage warm-up day folded")
                .with("day", prev_text.as_str())
                .with("contracts", contracts.len()),
        );
    }
}

/// How a warm-up reads one day's wanted files: each at its place in the
/// listing, answering the contract-day of every file it read, in listing
/// order (a file the source refuses is passed over).
type WarmRead<'r, L> = &'r dyn Fn(
    &crate::gdfl_nfo::NfoDay<L>,
    &[(usize, &crate::gdfl_cm::ListedFile<L>)],
) -> Vec<lineage::DayContract>;

/// The warm-up's files read one after another on this thread.
fn warm_files<S: NfoSource>(
    source: &S,
    listing: &crate::gdfl_nfo::NfoDay<S::Locator>,
    wanted: &[(usize, &crate::gdfl_cm::ListedFile<S::Locator>)],
) -> Vec<lineage::DayContract> {
    wanted
        .iter()
        .filter_map(|&(index, file)| {
            read_file(source, listing, file)
                .ok()
                .map(|day_file| contract_day_of(&day_file, index))
        })
        .collect()
}

/// The warm-up's files read on `workers` (WP-16), answered in listing order,
/// exactly what [`warm_files`] answers. A run that cannot start a worker
/// reads them here, saying so at `warn`.
fn warm_files_on<S>(
    source: &S,
    workers: usize,
    listing: &crate::gdfl_nfo::NfoDay<S::Locator>,
    wanted: &[(usize, &crate::gdfl_cm::ListedFile<S::Locator>)],
) -> Vec<lineage::DayContract>
where
    S: NfoSource + Sync,
    S::Locator: Sync,
{
    if workers <= 1 || wanted.len() <= 1 {
        return warm_files(source, listing, wanted);
    }
    // Nothing here files a second, so no worker asks the gate for `begin`.
    let gate = BeginGate::new(None, workers.min(wanted.len()));
    let read = on_workers(
        workers,
        wanted,
        &gate,
        &mut || {
            Err(ImportRefusal::Journal {
                why: "a lineage warm-up writes no journal line".to_owned(),
            })
        },
        &|&(index, file): &(usize, &crate::gdfl_cm::ListedFile<S::Locator>)| {
            read_file(source, listing, file)
                .ok()
                .map(|day_file| contract_day_of(&day_file, index))
        },
    );
    match read {
        Ok(all) => all.into_iter().flatten().collect(),
        Err(why) => {
            let _dropped_when_filtered = telemetry::emit(
                &telemetry::Event::warn(
                    TARGET,
                    "no worker thread started; the warm-up's files run on one thread",
                )
                .with("why", why.as_str()),
            );
            warm_files(source, listing, wanted)
        }
    }
}

/// One options day as a bare reader hands it on: [`nfo_day_with`] with no
/// lineage, its files' facts dropped.
#[cfg(test)]
fn nfo_day<S: NfoSource>(
    source: &S,
    run: &Run<'_>,
    day: Day,
    sink: &mut dyn FnMut(TickFile),
) -> DayRead {
    nfo_day_with(source, run, day, &mut |handed| sink(handed.file), None).0
}

/// One wanted option file of a day: its place in the listing, the listed
/// file and its ticker.
struct NfoJob<'a, L> {
    index: usize,
    file: &'a crate::gdfl_cm::ListedFile<L>,
    ticker: &'a str,
}

/// The options listing of `day`: none when the source does not hold the
/// day, and the listing refused by name when it cannot be read.
fn nfo_list<S: NfoSource>(
    source: &S,
    day: Day,
) -> (Listed, Option<crate::gdfl_nfo::NfoDay<S::Locator>>) {
    let mut listed = Listed::default();
    match source.day(day) {
        Ok(Some(listing)) => {
            listed.held = true;
            listed.member_bytes = listing.member_bytes();
            (listed, Some(listing))
        }
        Ok(None) => (listed, None),
        Err(why) => {
            listed.held = true;
            listed.refused.push((0, Refused::cm("listing", &why)));
            (listed, None)
        }
    }
}

/// Every option file of `listing` whose underlying passes the filter, in
/// listing order: what the day reads. Two tickers of the day naming one
/// contract would file into one path, so every one of them is refused by
/// name; a name that does not decode is refused when it could be wanted;
/// every other entry is a skip.
fn nfo_jobs<'a, L>(
    run: &Run<'_>,
    day: Day,
    listing: &'a crate::gdfl_nfo::NfoDay<L>,
    listed: &mut Listed,
) -> Vec<NfoJob<'a, L>> {
    // Counted first, so every one of a contract's names is refused.
    let mut named: HashMap<(String, Contract), usize> =
        HashMap::with_capacity(listing.entries().len());
    for file in listing.entries() {
        if let Some(Ok(decoded)) = listing.ticker_of(file).map(|t| decode_ticker(t, day)) {
            *named
                .entry((decoded.underlying.as_str().to_owned(), decoded.contract))
                .or_default() += 1;
        }
    }
    let mut jobs = Vec::with_capacity(listing.entries().len());
    for (index, file) in listing.entries().iter().enumerate() {
        let Some(ticker) = listing.ticker_of(file) else {
            listed.skipped += 1;
            continue;
        };
        let wanted = match decode_ticker(ticker, day) {
            Ok(decoded)
                if run.wants(decoded.underlying.as_str())
                    && named.get(&(decoded.underlying.as_str().to_owned(), decoded.contract))
                        > Some(&1) =>
            {
                let why = NfoRefusal::ContractNamedTwice {
                    ticker: ticker.to_owned(),
                };
                listed.refused.push((index, Refused::nfo(ticker, &why)));
                continue;
            }
            Ok(decoded) => run.wants(decoded.underlying.as_str()),
            // A name that does not decode is refused when it could be wanted.
            Err(_) => run.only.is_empty() || claims_undecodable(run, ticker),
        };
        if !wanted {
            listed.skipped += 1;
            continue;
        }
        jobs.push(NfoJob {
            index,
            file,
            ticker,
        });
    }
    jobs
}

/// Reads one wanted option file, handed on with its member and its quote
/// count, and, when `lineage`, the contract-day the lineage folds.
fn nfo_read<S: NfoSource>(
    source: &S,
    listing: &crate::gdfl_nfo::NfoDay<S::Locator>,
    job: &NfoJob<'_, S::Locator>,
    lineage: bool,
) -> ReadJob {
    match read_file(source, listing, job.file) {
        Ok(day_file) => {
            let contract = lineage.then(|| contract_day_of(&day_file, job.index));
            Ok((option_handed(day_file, job.file), contract))
        }
        Err(why) => Err(Refused::nfo(job.ticker, &why)),
    }
}

/// One options day, one file after another on this thread: every option
/// file whose underlying passes the filter, handed on with its member and
/// its quote count, and, when `lineage` is given, folded into the
/// corporate-action lineage after the day's files. The rescan's door, and
/// the one the import's workers share every step of ([`nfo_jobs`],
/// [`nfo_read`]).
fn nfo_day_with<S: NfoSource>(
    source: &S,
    run: &Run<'_>,
    day: Day,
    sink: &mut dyn FnMut(Handed),
    mut lineage: Option<&mut lineage::LineageRun>,
) -> (DayRead, DayExtras) {
    let mut early = DayExtras::default();
    if let Some(lineage) = lineage.as_deref_mut() {
        warm_up(
            source,
            run,
            day,
            lineage,
            (&mut early, &|listing, wanted| {
                warm_files(source, listing, wanted)
            }),
        );
    }
    let (mut listed, listing) = nfo_list(source, day);
    let Some(listing) = listing else {
        return with_early(listed.close(Vec::new()).0, early);
    };
    let jobs = nfo_jobs(run, day, &listing, &mut listed);
    let mut reads = Vec::with_capacity(jobs.len());
    for job in &jobs {
        let read = match nfo_read(source, &listing, job, lineage.is_some()) {
            Ok((handed, contract)) => {
                sink(handed);
                JobRead::Read(contract)
            }
            Err(refused) => JobRead::Refused(refused),
        };
        reads.push((job.index, read));
    }
    let ((read, mut extras), contracts) = listed.close(reads);
    extras.absorb(early);
    if let Some(lineage) = lineage {
        fold_lineage(
            source,
            run,
            (day, &listing),
            lineage,
            &contracts,
            &mut extras,
        );
    }
    (read, extras)
}

// ═══ `cli gdfl-dayflags`: the day ledger of days already imported ══════════

/// What a rescan did (D-3335 §1, `cli gdfl-dayflags`): it reads the vendor's
/// files of days the journal records as imported, writes the day ledger and,
/// when asked, the lineage, and nothing else.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Rescan {
    /// Days read.
    pub days_read: usize,
    /// Days of the range the journal records no import of: not read.
    pub days_not_imported: usize,
    /// Days the journal records as imported that the source no longer
    /// holds, named.
    pub days_missing: Vec<Day>,
    /// Files read.
    pub files: usize,
    /// Files the reader or the placement refused, named in `failures`.
    pub files_refused: usize,
    /// Day-ledger lines written.
    pub flags_lines: usize,
    /// Day-ledger lines already there, `src` aside, and not written again.
    pub flags_unchanged: usize,
    /// Lineage-ledger records written (D-3331).
    pub lineage_lines: usize,
    /// Lot-ledger records written (D-3343).
    pub lot_lines: usize,
    /// Adjustments found (D-3331).
    pub adjustments: usize,
    /// Every failure, named.
    pub failures: Vec<Failure>,
}

/// The days of `kind` the import journal under `store_root` records as
/// imported, under any filter: a key whose last closing line is `done`,
/// `recounted` or `incomplete` (the day wrote bars). Read-only: the journal
/// is never closed, repaired or written here, and a line this reader cannot
/// key is passed over. O(the journal's lines).
///
/// # Errors
///
/// [`ImportRefusal::Journal`] for a journal that exists and cannot be read.
fn imported_days(store_root: &Path, kind: ImportKind) -> Result<HashSet<Day>, ImportRefusal> {
    let path = journal_path(store_root);
    let text = match std::fs::read(&path) {
        Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(why) => {
            return Err(ImportRefusal::Journal {
                why: format!("{}: {why}", path.display()),
            });
        }
    };
    let mut last: HashMap<(Day, &str), bool> = HashMap::new();
    for line in text.lines() {
        let mut words = line.split(' ');
        let verb = words.next().unwrap_or_default();
        let (Some(word), Some(day), Some(filter)) = (words.next(), words.next(), words.next())
        else {
            continue;
        };
        if word != kind.as_str() {
            continue;
        }
        let Some(day) = crate::gdfl_lineage::parse_day(day) else {
            continue;
        };
        match verb {
            "done" | "recounted" | "incomplete" => {
                last.insert((day, filter), true);
            }
            "refused" => {
                last.insert((day, filter), false);
            }
            _ => {}
        }
    }
    Ok(last
        .into_iter()
        .filter_map(|((day, _), imported)| imported.then_some(day))
        .collect())
}

/// The rescan of every imported day of `kind` in `from..=to`, each read
/// through `read_day` and counted into the day ledger with `src=rescan`.
fn rescan_core<F>(
    kind: ImportKind,
    (from, to): (Day, Day),
    store_root: &Path,
    mut read_day: F,
) -> Result<Rescan, ImportRefusal>
where
    F: FnMut(Day, &Run<'_>, &mut dyn FnMut(Handed)) -> (DayRead, DayExtras),
{
    if from > to {
        return Err(ImportRefusal::RangeBackwards { from, to });
    }
    let imported = imported_days(store_root, kind)?;
    let mut flags = quality::Ledger::load(quality::dayflags_path(store_root))
        .map_err(|why| ImportRefusal::DayLedger { why })?;
    let run = Run {
        kind,
        from,
        to,
        only: &[],
        store_root,
    };
    let mut done = Rescan::default();
    for day in days(from, to) {
        if !imported.contains(&day) {
            done.days_not_imported += 1;
            continue;
        }
        let session = match crate::gdfl_cm::session_of(day) {
            Ok(session) => session,
            Err(why) => {
                let cause = cm_cause(&why);
                done.failures
                    .push(note_failure(kind, day, "calendar", &why.to_string(), cause));
                continue;
            }
        };
        let mut ledger = quality::DayLedger::new(kind, day, session);
        let mut failures: Vec<Failure> = Vec::new();
        let (mut files, mut refused_files) = (0_usize, 0_usize);
        let (read, extras) = read_day(day, &run, &mut |handed| match convert_noted(
            kind,
            day,
            &handed.file.ticks,
        ) {
            Ok((converted, deferred)) => {
                files += 1;
                take_quality(kind, &mut ledger, &handed, &converted, deferred);
            }
            Err(why) => {
                refused_files += 1;
                let cause = Cause::open(Stage::File, why.code());
                failures.push(note_failure(
                    kind,
                    day,
                    &handed.file.name,
                    &why.to_string(),
                    cause,
                ));
            }
        });
        done.files += files;
        done.files_refused += refused_files + read.refused.len();
        for refused in read.refused {
            let Refused { cause, failure } = refused;
            failures.push(note_failure(
                kind,
                day,
                &failure.instrument,
                &failure.why,
                cause,
            ));
        }
        failures.extend(extras.failures.into_iter().map(|one| one.failure));
        done.lineage_lines += extras.lineage.lineage;
        done.lot_lines += extras.lineage.lots;
        done.adjustments += extras.lineage.actions;
        if !read.held {
            done.days_missing.push(day);
            done.failures.extend(failures);
            continue;
        }
        done.days_read += 1;
        ledger.set_member_bytes(extras.member_bytes);
        let written = rescan_flags(&mut flags, ledger, &mut done, &mut failures);
        let day_text = day.to_string();
        let _dropped_when_filtered = telemetry::emit(
            &telemetry::Event::info(TARGET, "day rescanned")
                .with("kind", kind.as_str())
                .with("day", day_text.as_str())
                .with("files", files)
                .with("lines", written)
                .with("failures", failures.len()),
        );
        done.failures.extend(failures);
    }
    Ok(done)
}

/// A rescanned day's lines, `src=rescan`, those that say something new
/// appended (D-3335 §1): counted in `done`, or a named failure. Answers how
/// many were written.
fn rescan_flags(
    flags: &mut quality::Ledger,
    mut ledger: quality::DayLedger,
    done: &mut Rescan,
    failures: &mut Vec<Failure>,
) -> usize {
    let sampled = ledger.sampled();
    let fresh = ledger.lines(true);
    let offered = fresh.len();
    let news = flags.news(fresh, quality::Src::Rescan);
    match flags.append(&news) {
        Ok(()) => {
            note_sampled(
                ledger.kind(),
                ledger.day(),
                &quality::sampled_among(ledger.kind(), ledger.day(), sampled, &news),
                quality::Src::Rescan,
            );
            done.flags_lines += news.len();
            done.flags_unchanged += offered - news.len();
            news.len()
        }
        Err(why) => {
            let refusal = ImportRefusal::DayLedger { why };
            failures.push(note_failure(
                ledger.kind(),
                ledger.day(),
                "day ledger",
                &refusal.to_string(),
                Cause::open(Stage::Journal, refusal.code()),
            ));
            0
        }
    }
}

/// `cli gdfl-dayflags indices|stocks`: the day ledger of every day of
/// `from..=to` the journal records as imported, read from `source`
/// read-only (D-3335 §1). Nothing under `bars/` is written, the journal is
/// read and never written, and only `imports/gdfl.dayflags` is appended to.
///
/// # Errors
///
/// [`ImportRefusal::KindUnknown`] for `options`, [`ImportRefusal::RangeBackwards`],
/// [`ImportRefusal::Journal`] for an unreadable journal and
/// [`ImportRefusal::DayLedger`] for an unreadable ledger; everything else is
/// a named failure in the answer.
pub fn rescan_cm<S: CmSource>(
    source: &S,
    kind: ImportKind,
    range: (Day, Day),
    store_root: &Path,
) -> Result<Rescan, ImportRefusal> {
    let tree = match kind {
        ImportKind::Indices => CmKind::Indices,
        ImportKind::Stocks => CmKind::Stocks,
        ImportKind::Options => {
            return Err(ImportRefusal::KindUnknown {
                word: "options read from a capital-market source".to_owned(),
            });
        }
    };
    rescan_core(kind, range, store_root, |day, run, sink| {
        cm_day_with(source, run, (tree, day), sink, Mode::Rescan)
    })
}

/// `cli gdfl-dayflags options`: as [`rescan_cm`], and with `with_lineage`
/// the corporate-action lineage too (D-3331, D-3343): the lineage and lot
/// ledgers are appended to, still nothing under `bars/`.
///
/// # Errors
///
/// As [`rescan_cm`], and [`ImportRefusal::Lineage`] for an unreadable
/// lineage ledger.
pub fn rescan_nfo<S: NfoSource>(
    source: &S,
    range: (Day, Day),
    store_root: &Path,
    with_lineage: bool,
) -> Result<Rescan, ImportRefusal> {
    let mut lineage = if with_lineage {
        Some(
            lineage::LineageRun::load(store_root, range.0)
                .map_err(|why| ImportRefusal::Lineage { why })?,
        )
    } else {
        None
    };
    rescan_core(ImportKind::Options, range, store_root, |day, run, sink| {
        nfo_day_with(source, run, day, sink, lineage.as_mut())
    })
}

#[path = "gdfl_import_quality.rs"]
pub mod quality;

#[path = "gdfl_import_lineage.rs"]
pub mod lineage;

#[path = "gdfl_import_scope.rs"]
pub mod scope;

#[cfg(test)]
#[path = "gdfl_wp12e_tests.rs"]
mod wp12e_tests;

#[cfg(test)]
#[path = "gdfl_import_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "gdfl_seconds_attack_tests.rs"]
mod seconds_attack_tests;

#[cfg(test)]
#[path = "gdfl_r2_attack_tests.rs"]
mod r2_attack_tests;

#[cfg(test)]
#[path = "gdfl_r3_attack_tests.rs"]
mod r3_attack_tests;

#[cfg(test)]
#[path = "gdfl_r4_attack_tests.rs"]
mod r4_attack_tests;

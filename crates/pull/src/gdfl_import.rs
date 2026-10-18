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
use std::path::{Path, PathBuf};

use brutex_core::instrument::Contract;
use brutex_core::vendor::Vendor;
use store::format::{Bar, OI_NULL};

use crate::fetch::BarRequest;
use crate::fold::{Bucket, fold};
use crate::gdfl_cm::{CmKind, CmRefusal, CmSource, read_listed, split_name};
use crate::gdfl_nfo::{NfoRefusal, NfoSource, decode_ticker, read_file};
use crate::ingest::{Failure, Plan};
#[cfg(test)]
use crate::manifest::{ENTRY_STRIDE, HEADER_LEN, Held, MAX_ENTRIES, Manifest, manifest_path};
#[cfg(test)]
use crate::session::Window;
use crate::session::{Day, IST_OFFSET_SECS};
#[cfg(test)]
use crate::vendor::Granularity;
use crate::vendor::{Listing, PriceScale, TimestampEncoding};

pub use self::causes::{Cause, ImportRefusal, Outcome, Stage, cm_cause, nfo_cause};
pub use self::census::CensusRoom;
use self::census::StoreCensus;
#[cfg(test)]
use self::census::{CENSUS_CAP, Census, census_code, census_refusal, census_room, stopped_event};
#[cfg(test)]
use self::day::{
    Asked, Close, Counts, Ctx, Owed, Scoped, before_range, close_of, drive_with,
    restated_past_append, settings_event, started_event,
};
use self::day::{Mode, Staged, days, drive_core, note_staged, now_unix_s};
pub use self::day::{drive, drive_from};
#[cfg(test)]
use self::filing::symbol_id_of;
use self::filing::{BeginGate, JobRead, ReadJob, on_workers, take_quality};
pub(crate) use self::journal::append_lines;
pub use self::journal::{BAR_DEFINITION, definition_is_current, journal_path};
#[cfg(test)]
use self::journal::{Journal, closing_line, count_of, torn_fragment};

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

/// Says out loud each index of the day whose file is minute-sampled (WP-11c,
/// DB-A04): the day still closes `done` and its bars are filed as printed
/// (the operator's answer: flagged only), so the log is where the import
/// itself names it; the day ledger's own `rows`, `seconds`, `session_s` and
/// `ss_distinct` carry it to `/dayflags.json`. One warning an index, at most
/// the day's indices. The import says it of every day it files; a rescan
/// says it only of the lines it wrote ([`quality::sampled_among`]), so a
/// rescan that finds nothing new repeats nothing (D-4928 item 7).
pub(crate) fn note_sampled(
    kind: ImportKind,
    day: Day,
    sampled: &[quality::Sampled],
    src: quality::Src,
) {
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
    let mut seen = HashSet::with_capacity(wanted.len());
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
        if !wanted.contains(member.as_str()) {
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

/// Names a file whose match band traded at two prices or more, so its booked
/// rows there stayed ordinary rows of phase `cas` (SC-CLOSE-6): never a
/// silent choice. Nothing to say for any other file.
fn note_unattributed(day: Day, symbol: &str, name: &str, booked: u32) {
    if booked == 0 {
        return;
    }
    let day_text = day.to_string();
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::warn(
            TARGET,
            "the auction's match band traded at two prices or more: its booked rows stay \
             ordinary rows of phase cas, none is called the auction's",
        )
        .with("kind", ImportKind::Stocks.as_str())
        .with("day", day_text.as_str())
        .with("symbol", symbol)
        .with("file", name)
        .with("booked_rows", u64::from(booked))
        .with("code", "auction_band_unattributed"),
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
        note_unattributed(
            day,
            &job.symbol,
            &day_file.name,
            day_file.file.auction_band_unattributed,
        );
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
        Err(why) => extras.failures.push(note_staged(
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

#[path = "gdfl_import_causes.rs"]
mod causes;

#[path = "gdfl_import_census.rs"]
mod census;

#[path = "gdfl_import_day.rs"]
mod day;

#[path = "gdfl_import_filing.rs"]
mod filing;

#[path = "gdfl_import_journal.rs"]
mod journal;

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

#[cfg(test)]
#[path = "gdfl_import_split_tests.rs"]
mod split_tests;

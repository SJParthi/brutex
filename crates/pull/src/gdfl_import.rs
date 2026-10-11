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
//! file of the 12 days, which stayed `incomplete`). Every file is now filed
//! through `ingest::repair_rows`, WP-13c's one repair authority reached from
//! this door: the append door first, so a tail, a new month and a day
//! already held land exactly as before. When it refuses, the month as every
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
use crate::ingest::{Failure, Filed, Ingested, Plan, record_held, repair_rows};
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
        }
    }

    /// What the run did before a census refusal stopped it, or `None` for a
    /// refusal that stopped it before any day.
    #[must_use]
    pub fn report(&self) -> Option<&Report> {
        match self {
            Self::CensusFull { done, .. } | Self::CensusRefused { done, .. } => Some(done),
            _ => None,
        }
    }
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
    /// Days that were imported this run (clean or not).
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
    fn check_filter(&self) -> Result<(), ImportRefusal> {
        for name in self.only {
            let filed = brutex_core::symbol::Symbol::new(name)
                .is_ok_and(|symbol| symbol.as_str() == name.as_str());
            if !filed {
                return Err(ImportRefusal::FilterName { name: name.clone() });
            }
        }
        Ok(())
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
        Ok(journal)
    }

    /// Folds one whole line into the journal's sets, or `Err(())` for a line
    /// this journal does not write. O(fields).
    fn read_line(&mut self, fields: &[&str]) -> Result<(), ()> {
        let key = |kind: &str, day: &str, only: &str| format!("{kind} {day} {only}");
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

impl Counts {
    /// The day's counts: what `report` holds beyond `before`.
    fn of_day(before: &Report, report: &Report, failures: usize, max_back_s: u32) -> Self {
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

/// A day's closing line, all but its `run=` (D-3317 items 2 and 3).
fn closing_line(key: &str, close: Close, counts: &Counts) -> String {
    match close {
        Close::Done => format!("done {key} {}", counts.text()),
        Close::WithRefusals(code) => format!(
            "done {key} {} vendor_refused={} reason={code}",
            counts.text(),
            counts.failures
        ),
        Close::Incomplete(cause) => format!(
            "incomplete {key} {} stage={} reason={}",
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
}

impl Ctx<'_> {
    /// Appends `line`, after the run's `run` line when it is the run's first,
    /// and answers its line number in the journal, from 1.
    fn append(&self, line: &str) -> Result<u64, ImportRefusal> {
        if !self.announced.get() {
            self.journal
                .append(&run_line(self.id, self.started, self.options.origin))?;
            self.announced.set(true);
        }
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
        let unrun = closing_line(key, close, counts);
        if matches!(close, Close::Refused(_)) && self.journal.refused_close.get(key) == Some(&unrun)
        {
            return close.outcome();
        }
        let line = format!("{unrun} run={}", self.id);
        match self.append(&line) {
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
    let path = store::path::StorePath::new(store::path::PathParts {
        vendor: Vendor::Gdfl,
        exchange: key.exchange.as_str(),
        segment: key.segment.as_str(),
        symbol: key.symbol.as_str(),
        contract: key.contract,
        timeframe: key.timeframe,
        month: key.month,
        file: store::path::FileKind::Bars,
    })
    .map_err(|why| why.to_string())?;
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

/// The day's per-file work and the census rows it collects.
struct DayWork<'a> {
    ctx: &'a Ctx<'a>,
    day: Day,
    request: &'a BarRequest,
    key: String,
    begun: bool,
    /// The `begin` line's number in the journal, once written: the line a
    /// provenance record names (D-3334's `journal_ordinal`).
    begin_line: u64,
    held: Vec<Held>,
    report: &'a mut Report,
    failures: Vec<Staged>,
    /// The day's largest back-step, for its journal line.
    max_back_s: u32,
    /// The day's quality fields (D-3335), when the day has a session.
    ledger: Option<quality::DayLedger>,
    /// Where the day's files came from, when the run names an origin
    /// (D-3334), or why it could not be measured.
    source: Option<Result<DaySource, String>>,
}

/// A file's quality (D-3335), from its rows, the bars it built and what its
/// placement counted, folded into `ledger`.
fn take_quality(
    kind: ImportKind,
    ledger: &mut quality::DayLedger,
    handed: &Handed,
    converted: &Converted,
    deferred: quality::Deferred,
) {
    let day = ledger.day();
    let seconds: Vec<u32> = converted
        .seconds
        .iter()
        .map(|bar| second_of(bar.ts_micros, day))
        .collect();
    let measured = quality::measure_file(
        kind,
        ledger.session(),
        &handed.file.ticks,
        &seconds,
        (converted.placement, deferred),
        handed.facts.reader,
    );
    ledger.take(&handed.file.symbol, &measured);
}

impl DayWork<'_> {
    /// Converts and files one instrument file, counts its quality and
    /// records its landing.
    fn file(&mut self, handed: &Handed) {
        let run = self.ctx.run;
        let file = &handed.file;
        // A file the fold refuses is counted refused and never also read
        // (D-2814): files, skips and refusals add up to the day's entries.
        let (converted, deferred) = match convert_noted(run.kind, self.day, &file.ticks) {
            Ok(converted) => {
                self.report.files += 1;
                converted
            }
            Err(why) => {
                self.report.files_refused += 1;
                let refused = staged(run, self.day, &file.name, Stage::File, &why);
                self.failures.push(refused);
                return;
            }
        };
        if let Some(ledger) = self.ledger.as_mut() {
            take_quality(run.kind, ledger, handed, &converted, deferred);
        }
        let p = converted.placement;
        self.report.rows += p.rows;
        self.report.ltq_zero_dropped += p.ltq_zero_dropped;
        self.report.late_rows += p.late_rows;
        self.report.late_unresolved += p.late_unresolved;
        self.report.max_back_s = self.report.max_back_s.max(p.max_back_s);
        self.max_back_s = self.max_back_s.max(p.max_back_s);
        if converted.seconds.is_empty() {
            return;
        }
        if !self.begun {
            match self.ctx.append(&format!("begin {}", self.key)) {
                Ok(line) => self.begin_line = line,
                Err(why) => {
                    let refused = staged(run, self.day, "journal", Stage::Journal, &why);
                    self.failures.push(refused);
                    return;
                }
            }
            self.begun = true;
        }
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
        let reason = format!(
            "gdfl-import {} {} {} run:{}: a day the month lacks where no append can land it",
            run.kind.as_str(),
            self.day,
            file.name,
            self.ctx.id
        );
        let (done, filed): (Ingested, Filed) = repair_rows(
            &converted.seconds,
            &file.symbol,
            &origin,
            run.store_root,
            plan(run.kind, self.request, file.contract),
            &reason,
        );
        self.report.outside_session += usize::try_from(done.census.total()).unwrap_or(0);
        self.report.seconds += done.bars_stored;
        self.report.seconds_committed += done.bars_committed;
        if done.bars_committed > 0
            && let Some(held) = done.pending.as_ref()
        {
            match filed {
                Filed::Appended => self.provenance(handed, held, done.bars_committed),
                Filed::NewGeneration => self.repaired(handed, held, done.bars_committed),
            }
        }
        self.held.extend(done.pending);
        for failure in done.failures {
            let refusal = ImportRefusal::Filing { why: failure.why };
            let refused = staged(run, self.day, &file.name, Stage::File, &refusal);
            self.failures.push(refused);
        }
        let _dropped_when_filtered = telemetry::emit(
            &telemetry::Event::debug(TARGET, "filed")
                .with("file", file.name.as_str())
                .with("rows", p.rows)
                .with("seconds", converted.seconds.len()),
        );
    }

    /// Writes the provenance record of a file's landing (D-3334) when the
    /// run names its origin and the source named the file's member. A record
    /// the sidecar refuses is a named failure of the day, never silence: the
    /// bars stay committed and stay unrecorded, which `/provenance.json`
    /// labels reconstructed.
    fn provenance(&mut self, handed: &Handed, held: &Held, committed: usize) {
        let run = self.ctx.run;
        let (Some(origin), Some(member), Some(source)) = (
            self.ctx.options.origin,
            handed.facts.member.as_ref(),
            self.source.as_ref(),
        ) else {
            return;
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
                journal_ordinal: self.begin_line,
                commit: origin.commit.as_deref(),
                landed_at_micros: now_micros(),
            };
            record_provenance(run.store_root, held, committed, &landing)
        });
        match recorded {
            Ok(true) => self.report.provenance_records += 1,
            Ok(false) => {}
            Err(why) => {
                let refusal = ImportRefusal::Provenance { why };
                let refused = staged(run, self.day, &handed.file.name, Stage::File, &refusal);
                self.failures.push(refused);
            }
        }
    }

    /// A file whose day its month lacked inside what it holds, landed as a
    /// NEW GENERATION of the month (D-3350; DB-N26): counted in
    /// [`Report::months_repaired`] and logged with the month's rows now
    /// served. The repair door journaled it in `<store>/audit/gdfl.repairs`
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
    fn repaired(&mut self, handed: &Handed, held: &Held, added: usize) {
        self.report.months_repaired += 1;
        let unrecorded = self.ctx.options.origin.is_some()
            && handed.facts.member.is_some()
            && self.source.is_some();
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
                .with("run", self.ctx.id),
        );
    }
}

/// Writes an admitted day's one closing line and says how the day closed
/// (D-3317): `done`, done with refusals, `incomplete` or `refused`, by
/// [`close_of`]. Every admitted day the source holds gets one, begun or not:
/// until D-3317 a day whose every file was refused left no line at all.
fn close_day(
    ctx: &Ctx<'_>,
    (key, day): (&str, Day),
    (before, report, max_back_s): (&Report, &Report, u32),
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

/// The day's one summary event (D-3317): "day imported" at `info` for a day
/// closed `done`, with or without refusals; "day incomplete" or "day
/// refused" at `warn` otherwise. A census-refused day never reaches it.
fn note_day(kind: ImportKind, day: Day, outcome: Outcome, (before, report): (&Report, &Report)) {
    let day_text = day.to_string();
    let event = match outcome {
        Outcome::Done | Outcome::DoneWithRefusals => telemetry::Event::info(TARGET, "day imported"),
        Outcome::Incomplete => telemetry::Event::warn(TARGET, "day incomplete"),
        Outcome::Refused => telemetry::Event::warn(TARGET, "day refused"),
    };
    let _dropped_when_filtered = telemetry::emit(
        &event
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
            .with("failures", report.failures.len() - before.failures.len()),
    );
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
    (before, mut report, max_back_s): (&Report, Report, u32),
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
/// record, and no reader counts.
fn drive_with<F>(
    run: &Run<'_>,
    options: RunOptions<'_>,
    census: &mut dyn Census,
    mut read_day: F,
) -> Result<Report, ImportRefusal>
where
    F: FnMut(Day, &mut dyn FnMut(TickFile)) -> DayRead,
{
    drive_core(run, options, census, |day, sink| {
        let read = read_day(day, &mut |file| {
            sink(Handed {
                file,
                facts: FileFacts::default(),
            });
        });
        (read, DayExtras::default())
    })
}

/// The day's quality lines (D-3335), written before its close: a ledger that
/// will not take them is a failure of the day, which then closes
/// `incomplete` and is read again, so its lines are written by the next run.
fn write_flags(
    run: &Run<'_>,
    flags: &mut quality::Ledger,
    (day, ledger, member_bytes): (Day, Option<quality::DayLedger>, Option<u64>),
    report: &mut Report,
    failures: &mut Vec<Staged>,
) {
    let Some(mut ledger) = ledger else {
        return;
    };
    ledger.set_member_bytes(member_bytes);
    let fresh = ledger.lines(run.only.is_empty());
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
fn drive_core<F>(
    run: &Run<'_>,
    options: RunOptions<'_>,
    census: &mut dyn Census,
    mut read_day: F,
) -> Result<Report, ImportRefusal>
where
    F: FnMut(Day, &mut dyn FnMut(Handed)) -> (DayRead, DayExtras),
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
    };
    note_started(&ctx, &filter);
    let mut report = Report::default();
    for day in days(run.from, run.to) {
        let key = format!("{} {day} {filter}", run.kind.as_str());
        if !admit(&ctx, &key, day, &mut report) {
            continue;
        }
        let before = report.clone();
        let ledger = crate::gdfl_cm::session_of(day)
            .ok()
            .map(|session| quality::DayLedger::new(run.kind, day, session));
        let source = options
            .origin
            .map(|origin| day_source(run.kind, day, origin));
        let mut work = DayWork {
            ctx: &ctx,
            day,
            request: &request,
            key: key.clone(),
            begun: false,
            begin_line: 0,
            held: Vec::new(),
            report: &mut report,
            failures: Vec::new(),
            max_back_s: 0,
            ledger,
            source,
        };
        let (read, extras) = read_day(day, &mut |handed| work.file(&handed));
        let (held, mut failures, day_back, ledger) =
            (work.held, work.failures, work.max_back_s, work.ledger);
        report.files_skipped += read.skipped;
        report.files_refused += read.refused.len();
        for refused in read.refused {
            let Refused { cause, failure } = refused;
            let failure = note_failure(run.kind, day, &failure.instrument, &failure.why, cause);
            failures.push(Staged { cause, failure });
        }
        report.lineage_lines += extras.lineage.lineage;
        report.lot_lines += extras.lineage.lots;
        report.adjustments += extras.lineage.actions;
        failures.extend(extras.failures);
        if !read.held {
            close_missing(&ctx, (&key, day), &mut report, failures);
            continue;
        }
        write_flags(
            run,
            &mut flags,
            (day, ledger, extras.member_bytes),
            &mut report,
            &mut failures,
        );
        if let Some(why) = census.publish(run.store_root, &held) {
            let room = census.room(run.store_root, &held);
            return Err(stop_at_census(
                &ctx,
                (&key, day),
                (&before, report, day_back),
                failures,
                (why, room),
            ));
        }
        report.days_imported += 1;
        let outcome = close_day(
            &ctx,
            (&key, day),
            (&before, &report, day_back),
            &mut failures,
        );
        record_close(run.kind, (day, outcome), (&before, &mut report), failures);
    }
    note_finished(&ctx, &report);
    Ok(report)
}

/// Files a closed day in the report, by how it closed, with its failures, and
/// logs its one summary event ([`note_day`]).
fn record_close(
    kind: ImportKind,
    (day, outcome): (Day, Outcome),
    (before, report): (&Report, &mut Report),
    failures: Vec<Staged>,
) {
    match outcome {
        Outcome::Done => {}
        Outcome::DoneWithRefusals => report.closed_with_refusals.push(day),
        Outcome::Incomplete => report.closed_incomplete.push(day),
        Outcome::Refused => report.closed_refused.push(day),
    }
    report
        .failures
        .extend(failures.into_iter().map(|one| one.failure));
    note_day(kind, day, outcome, (before, report));
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
pub fn run_cm<S: CmSource>(source: &S, run: &Run<'_>) -> Result<Report, ImportRefusal> {
    run_cm_from(source, run, RunOptions::default())
}

/// [`run_cm`] with the run's [`RunOptions`]: the origin the caller measured
/// (D-3317; DB-G56) and whether to read again the days done with refusals.
///
/// # Errors
///
/// As [`drive`].
pub fn run_cm_from<S: CmSource>(
    source: &S,
    run: &Run<'_>,
    options: RunOptions<'_>,
) -> Result<Report, ImportRefusal> {
    let tree = match run.kind {
        ImportKind::Indices => CmKind::Indices,
        ImportKind::Stocks => CmKind::Stocks,
        ImportKind::Options => {
            return Err(ImportRefusal::KindUnknown {
                word: "options read from a capital-market source".to_owned(),
            });
        }
    };
    drive_core(run, options, &mut StoreCensus, |day, sink| {
        cm_day_with(source, run, (tree, day), sink, Mode::Import)
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

/// One capital-market day: every swept ticker's file, once, in listing order,
/// each handed on with its member and its reader's counts.
fn cm_day_with<S: CmSource>(
    source: &S,
    run: &Run<'_>,
    (tree, day): (CmKind, Day),
    sink: &mut dyn FnMut(Handed),
    mode: Mode,
) -> (DayRead, DayExtras) {
    let mut read = DayRead::default();
    let mut extras = DayExtras::default();
    let listing = match source.day(tree, day) {
        Ok(Some(listing)) => listing,
        Ok(None) => return (read, extras),
        Err(why) => {
            read.held = true;
            read.refused.push(Refused::cm("listing", &why));
            return (read, extras);
        }
    };
    read.held = true;
    extras.member_bytes = listing.member_bytes();
    let mut seen = HashSet::with_capacity(listing.entries().len());
    let folder = format!("{}/", listing.folder());
    for file in listing.entries() {
        // What the listing's scope files the stem under this day (D-3333):
        // the swept surface under the default scope, as `resolve` answers.
        let filed = file
            .entry
            .strip_prefix(&folder)
            .and_then(|name| split_name(name).ok())
            .map(|(stem, _)| listing.filing(stem));
        let key = match filed {
            Some(Ok(Some(key))) => key,
            // A stem refused for its symbol is refused only where the run
            // wants the series it would have been filed under.
            Some(Err(why)) if why.filed_as().is_none_or(|symbol| run.wants(symbol)) => {
                read.refused.push(Refused::cm(file.name(), &why));
                continue;
            }
            Some(Ok(None) | Err(_)) | None => {
                read.skipped += 1;
                continue;
            }
        };
        // A second name for one ticker is a skip: its first name was refused
        // as twins, so every entry is a file, a skip or a refusal (D-2814).
        if !seen.insert(key) {
            read.skipped += 1;
            continue;
        }
        let symbol = key.underlying.as_str().to_owned();
        if !run.wants(&symbol) {
            read.skipped += 1;
            continue;
        }
        // A key resolved from this listing always has its file here, so a
        // `None` is refused by name rather than counted as skipped.
        // The closing auction's prints (D-3320) go to the month's sidecar
        // before the trades are filed, and a file whose prints it refuses is
        // refused whole, so every entry stays one file, skip or refusal. A
        // rescan files nothing, so it never writes a sidecar.
        let read_file = read_listed(source, &listing, &key)
            .and_then(|f| f.ok_or(CmRefusal::NotSwept))
            .and_then(|day_file| match mode {
                Mode::Import => crate::gdfl_cm::auction::append(
                    run.store_root,
                    &key,
                    day,
                    &day_file.file.auction,
                )
                .map(|_| day_file),
                Mode::Rescan => Ok(day_file),
            });
        match read_file {
            Ok(day_file) => {
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
                sink(Handed {
                    file: TickFile {
                        symbol,
                        contract: None,
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
                });
            }
            Err(why) => read.refused.push(Refused::cm(file.name(), &why)),
        }
    }
    (read, extras)
}

/// The run over an options source, its provenance `unrecorded`
/// ([`run_nfo_from`] records it).
///
/// # Errors
///
/// As [`drive`], and [`ImportRefusal::KindUnknown`] for a spot kind.
pub fn run_nfo<S: NfoSource>(source: &S, run: &Run<'_>) -> Result<Report, ImportRefusal> {
    run_nfo_from(source, run, RunOptions::default())
}

/// [`run_nfo`] with the run's [`RunOptions`] (D-3317; DB-G56), as
/// [`run_cm_from`]. Every day's option files also feed the corporate-action
/// lineage (D-3331, D-3343; [`lineage`]), whose ledgers are read before the
/// first day.
///
/// # Errors
///
/// As [`run_nfo`], and [`ImportRefusal::Lineage`] when a lineage ledger
/// exists and cannot be read.
pub fn run_nfo_from<S: NfoSource>(
    source: &S,
    run: &Run<'_>,
    options: RunOptions<'_>,
) -> Result<Report, ImportRefusal> {
    if run.kind != ImportKind::Options {
        return Err(ImportRefusal::KindUnknown {
            word: format!("{} read from an options source", run.kind.as_str()),
        });
    }
    let mut lineage = lineage::LineageRun::load(run.store_root, run.from)
        .map_err(|why| ImportRefusal::Lineage { why })?;
    drive_core(run, options, &mut StoreCensus, |day, sink| {
        nfo_day_with(source, run, day, sink, Some(&mut lineage))
    })
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
        return run.wants(underlying);
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
fn warm_up<S: NfoSource>(
    source: &S,
    run: &Run<'_>,
    day: Day,
    lineage: &mut lineage::LineageRun,
    extras: &mut DayExtras,
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
        let mut contracts = Vec::new();
        for (index, file) in listing.entries().iter().enumerate() {
            let wanted = listing
                .ticker_of(file)
                .and_then(|ticker| decode_ticker(ticker, prev).ok())
                .is_some_and(|decoded| run.wants(decoded.underlying.as_str()));
            if !wanted {
                continue;
            }
            if let Ok(day_file) = read_file(source, &listing, file) {
                contracts.push(contract_day_of(&day_file, index));
            }
        }
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

/// One options day: every option file whose underlying passes the filter,
/// handed on with its member and its quote count, and, when `lineage` is
/// given, folded into the corporate-action lineage after the day's files.
fn nfo_day_with<S: NfoSource>(
    source: &S,
    run: &Run<'_>,
    day: Day,
    sink: &mut dyn FnMut(Handed),
    mut lineage: Option<&mut lineage::LineageRun>,
) -> (DayRead, DayExtras) {
    let mut read = DayRead::default();
    let mut extras = DayExtras::default();
    if let Some(lineage) = lineage.as_deref_mut() {
        warm_up(source, run, day, lineage, &mut extras);
    }
    let listing = match source.day(day) {
        Ok(Some(listing)) => listing,
        Ok(None) => return (read, extras),
        Err(why) => {
            read.held = true;
            read.refused.push(Refused::cm("listing", &why));
            return (read, extras);
        }
    };
    read.held = true;
    extras.member_bytes = listing.member_bytes();
    // Two tickers of the day naming one contract would file into one path:
    // counted first, and every one of them refused by name.
    let mut named: HashMap<(String, Contract), usize> =
        HashMap::with_capacity(listing.entries().len());
    for file in listing.entries() {
        if let Some(Ok(decoded)) = listing.ticker_of(file).map(|t| decode_ticker(t, day)) {
            *named
                .entry((decoded.underlying.as_str().to_owned(), decoded.contract))
                .or_default() += 1;
        }
    }
    let mut contracts = Vec::new();
    for (index, file) in listing.entries().iter().enumerate() {
        let Some(ticker) = listing.ticker_of(file) else {
            read.skipped += 1;
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
                read.refused.push(Refused::nfo(ticker, &why));
                continue;
            }
            Ok(decoded) => run.wants(decoded.underlying.as_str()),
            // A name that does not decode is refused when it could be wanted.
            Err(_) => run.only.is_empty() || claims_undecodable(run, ticker),
        };
        if !wanted {
            read.skipped += 1;
            continue;
        }
        match read_file(source, &listing, file) {
            Ok(day_file) => {
                if lineage.is_some() {
                    contracts.push(contract_day_of(&day_file, index));
                }
                sink(option_handed(day_file, file));
            }
            Err(why) => read.refused.push(Refused::nfo(ticker, &why)),
        }
    }
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
    let fresh = ledger.lines(true);
    let offered = fresh.len();
    let news = flags.news(fresh, quality::Src::Rescan);
    match flags.append(&news) {
        Ok(()) => {
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

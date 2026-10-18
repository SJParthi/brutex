//! The importer's causes (R3-35, a move-only split of `gdfl_import.rs`):
//! why a run, a day or a file was refused ([`ImportRefusal`]), the stage
//! and cause a failure carries, the `cm_cause` and `nfo_cause` arms, and a
//! day's [`Outcome`].

use crate::gdfl_cm::CmRefusal;
use crate::gdfl_nfo::NfoRefusal;
use crate::session::Day;

use super::census::CensusRoom;
use super::{ImportKind, Report};

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

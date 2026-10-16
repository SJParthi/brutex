//! What every pull did, written where a restart cannot lose it.
//!
//! # Why this is a file and not a `Vec` behind a lock
//!
//! The operator asked for a record they can read afterwards. An in-memory ring
//! would have been three lines and one `Mutex`, and it would be empty in
//! exactly the situation the record is wanted in: the server fell over, or was
//! restarted, and the question is *what was the last thing it did*. A log that
//! vanishes on restart is a log that is missing when it is needed, so this one
//! is on disk, it is `fsync`-ed before the answer is rendered, and
//! [`Journal::path`] is printed on the page so the claim is checkable rather
//! than taken.
//!
//! It also has to be shared, and a shared mutable buffer inside the server is a
//! lock on the render path — the one thing `crates/api/src/server.rs` says it
//! does not have. The file itself is the lock boundary: every writer takes an
//! advisory exclusive lock on its own handle before measuring or appending,
//! while every reader opens its own handle. `O_APPEND` chooses the end offset;
//! the lock is what stops two cooperating processes from interleaving the
//! multi-call semantics of measure, write and sync.
//!
//! # Why the records are fixed stride
//!
//! Because the page must be able to show *the last twenty runs* without reading
//! the first twenty thousand. A newline-delimited text log cannot answer "where
//! does record `n - 20` start" without walking, so the tail of a text log costs
//! the whole log. Every record here is exactly [`RECORD_LEN`] bytes, so the
//! count is `len / 256` — one `metadata` call — and the offset of any record is
//! a multiplication. That is the same argument `docs/02-store-format.md` makes
//! for bars and `crates/pull/src/manifest.rs` makes for entries; this file is
//! the third instance of it and deliberately not a fourth idea.
//! `api::audit::the_tail_reads_only_the_records_it_shows` is the proof, and it
//! is written as a byte-offset assertion rather than as a timing.
//!
//! # What a torn write looks like
//!
//! One record is one `write_all` of 256 bytes to a handle opened `O_APPEND`,
//! followed by `sync_all`. A process killed between the two leaves the bytes
//! placed but unsynced; a process killed *inside* the write can in principle
//! leave a partial record. Both are visible rather than silent: the file length
//! stops being a multiple of [`RECORD_LEN`], which [`Log::Held::torn`] reports
//! by name, and any record whose bytes are wrong fails its CRC-32C and is
//! rendered as a refusal in its own row instead of as a number. A later writer
//! measures the file while holding the exclusive lock and refuses a ragged
//! length before writing one byte: appending after a partial record would move
//! every later record off its fixed-stride address. **Nothing here repairs or
//! truncates a damaged record.** `CLAUDE.md` §3 rule 8 and §4 — append-only,
//! then degrade loudly and name the reason, or refuse.
//!
//! **The one truncation is of this call's own failed bytes (D-1854).** A
//! `write_all` that fails part-way (ENOSPC, EIO) has put down part of a record
//! that no one committed, and left there it would make every later append
//! refuse the journal for good. So the writer cuts the file back to the length
//! it measured under the same exclusive lock, before releasing it, and refuses
//! naming the write error and the rollback. A record, whole or torn, that was
//! there before the call is never touched; a process killed inside the write
//! runs no rollback, and that tail is refused as above.
//!
//! # What is deliberately not here
//!
//! No rotation, no compaction, and no cap on the file. A pull is an operator
//! action and each one costs 256 bytes, so a hundred thousand pulls is 25.6 MB.
//! Rotation would mean a second file, an ordering rule between the two and a
//! deletion — and `CLAUDE.md` §3 rule 8 makes history append-only. The growth
//! is stated on the page instead, in bytes, so nobody has to guess.
//!
//! # What a record does NOT identify, said plainly
//!
//! `CLAUDE.md` §3 rule 3: a run is identified by `blake3(mask ‖ direction ‖
//! instrument ‖ timeframe ‖ params ‖ data_digest ‖ vocab_version ‖ commit ‖
//! feed)`. **No record written here carries that identity, and not one of the
//! nine terms is on disk.** That is written down under §3 rule 6 rather than left
//! for a reader to discover, because a file called an audit journal invites the
//! assumption that a run can be reproduced from it, and this one cannot.
//!
//! Two reasons, and they are different reasons.
//!
//! **This journal is not about a sweep.** It records a PULL — `/pull/spot` and
//! `/pull/fno`, the two forms [`Scope`] names. A pull has no mask, no
//! direction, no ladder parameters and no condition vocabulary, so four of the
//! eight terms do not exist for the thing being recorded and a hash over four
//! absences would be the same value for every pull. The eight-term identity
//! lives in `crates/runner/src/identity.rs` and it identifies a **sweep**.
//!
//! **The four that do apply are each blocked outside this file.**
//!
//! * `instrument` — [`Record::source`] holds what was ASKED FOR: a target name
//!   or a folder, which expands to many instruments. It is text cut at
//!   `SOURCE_CAPACITY`, not an `InstrumentKey`.
//! * `timeframe` — there is no field. [`crate::ladder`] names the same gap from
//!   the other side: nothing on a record distinguishes a day run from a minute
//!   one, which is why its gate reads the store rather than this journal.
//! * `data_digest` — nothing in the pull path computes one. [`Ingested`] carries
//!   counters, and a digest over counters would be a different quantity wearing
//!   the name, which §3 rule 1 is exactly about.
//! * `commit` — `crates/runner` takes it from its caller precisely because §2
//!   forbids a `build.rs` that shells out to `git`. Nothing hands one to this
//!   crate either.
//!
//! **And there is nowhere to put them.** The field map below allocates every
//! byte of the 256 but one: `OFF_KIND` is byte 118 and `OFF_SOURCE` begins at
//! 120, so byte 119 is the whole of the free space and a BLAKE3 output is 32
//! bytes. Recording an identity is therefore a **stride change** — a new
//! [`VERSION`] and a new record length, which §3 rule 8 makes a new format
//! rather than a mutation of this one — plus the `server` and `autopilot`
//! callers that would have to supply the terms, plus the
//! `docs/05-decisions.md` entry §9 requires for a locked choice.
//! `api::audit::the_record_has_exactly_one_free_byte` is the arithmetic, so
//! this paragraph cannot quietly go stale while the layout moves.
//!
//! Adding the fields anyway, filled with zeros no caller can populate, would be
//! an empty column that reads as compliance. §4 — never a fallback that hides
//! a failure.

use std::io::{Read as _, Seek as _};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use pull::ingest::Ingested;
use pull::session::{DropCensus, DropReason, Window};
use store::crc::crc32c;
use store::flock::Flock;

/// One record, in bytes. Every record is exactly this long, always.
pub const RECORD_LEN: usize = 256;

/// [`RECORD_LEN`] as the width an offset is computed in.
///
/// Written as a literal in both widths rather than converted, for the reason
/// `crates/api/src/census.rs` gives about its own pair: there is no cast to
/// deny and no fallible conversion whose failure arm no test could reach. The
/// assertion below keeps the two honest.
pub const RECORD_LEN_U64: u64 = 256;
const _: () = assert!(RECORD_LEN as u64 == RECORD_LEN_U64);

/// The four bytes a record starts with.
///
/// A file that is not this journal is refused by name rather than decoded into
/// plausible counters.
const MAGIC: [u8; 4] = *b"BXAU";

/// Which layout the record after it is in.
///
/// A version is per record and not per file, so a journal written by two builds
/// is readable by the newer one: an unknown version is refused for that record
/// alone and every other row still renders. `CLAUDE.md` §3 rule 8 — a format
/// version is never mutated in place.
///
/// **Version 2 (D-2673)** is the layout every record is now written in. It is
/// version 1 byte for byte up to [`OFF_NOTE_V1`]; there it carries two `u32`
/// counters version 1 had no room for — rows dropped on a day the exchange
/// calendar records closed, and rows kept on a day it cannot classify — and the
/// note follows them, eight bytes shorter. Same stride, so the journal stays
/// one fixed-stride file and a version-1 record beside a version-2 one still
/// reads ([`VERSION_ONE`]).
pub const VERSION: u16 = 2;

/// The first layout. Still read, never written. Its records predate the
/// closed-day filter, so their two newer counters are not zero but UNKNOWN,
/// and decode as `None`.
pub const VERSION_ONE: u16 = 1;

/// **Version 3 (D-3341, DB-G23): a member failure whose whole note is kept.**
///
/// Version 2's layout byte for byte, written only for a
/// [`Kind::MemberFailure`] whose reason was longer than [`NOTE_CAPACITY`] and
/// whose whole reason reached the notes file beside the journal
/// ([`Journal::notes_path`]). The eight bytes at [`OFF_COUNTED`], zero on every
/// member record by design (the counters belong to the run), hold the byte
/// offset of that whole note, so reading it is one seek and two reads however
/// long either file is.
///
/// Measured on the operator's journal on 9 Oct 2026: 4,217,015 member records,
/// and every one of them carries a reason longer than the field (2,909,596 of
/// 256 to 319 bytes; 45,928 of 1,280 or more), so every one was cut. A month
/// cell that wanted to say *why* Zerodha refused 2020-11-14 had 60 bytes of it.
///
/// Every other record is still written as [`VERSION`]: a build that knows only
/// versions 1 and 2 refuses these records by name and still reads every run.
pub const VERSION_WHOLE_NOTE: u16 = 3;

/// The longest source this record keeps, in bytes.
const SOURCE_CAPACITY: usize = 64;

/// The longest note a version-2 record keeps, in bytes.
pub const NOTE_CAPACITY: usize = 60;

/// The longest note a version-1 record kept, in bytes.
pub const NOTE_CAPACITY_V1: usize = 68;

// The field map. Every offset is a constant so the writer and the reader
// cannot disagree about one, which is the failure a hand-counted literal has.
const OFF_MAGIC: usize = 0;
const OFF_VERSION: usize = 4;
const OFF_SCOPE: usize = 6;
const OFF_OUTCOME: usize = 7;
const OFF_AT: usize = 8;
const OFF_ELAPSED: usize = 16;
const OFF_MEMBERS: usize = 24;
const OFF_ROWS_READ: usize = 32;
const OFF_BARS_STORED: usize = 40;
const OFF_ROWS_FOLDED: usize = 48;
const OFF_COUNTED: usize = 56;
const OFF_BEFORE_OPEN: usize = 64;
const OFF_AFTER_CLOSE: usize = 72;
const OFF_BEFORE_WINDOW: usize = 80;
const OFF_AFTER_WINDOW: usize = 88;
const OFF_FAILURES: usize = 96;
const OFF_FROM_DAYS: usize = 104;
const OFF_TO_DAYS: usize = 108;
const OFF_SOURCE_LEN: usize = 112;
const OFF_NOTE_LEN: usize = 114;
const OFF_SOURCE_KEPT: usize = 116;
const OFF_NOTE_KEPT: usize = 117;
/// Which shape this record is — see [`Kind`].
///
/// **Byte 118 was reserved and zero in every record ever written**, and
/// [`Kind::Run`] is code `0`, so every existing journal decodes unchanged and
/// the stride does not move. That is the whole reason a new record shape did
/// not need a new file version: `CLAUDE.md` §4 bans a dynamic schema, and this
/// is not one — it is a discriminator in space the format already had.
const OFF_KIND: usize = 118;
const OFF_SOURCE: usize = 120;
/// Where a version-1 record's note began, and where version 2's two new
/// counters begin.
const OFF_NOTE_V1: usize = 184;
/// Version 2: rows dropped on a day the exchange calendar records closed.
const OFF_CLOSED_DAY: usize = 184;
/// Version 2: rows kept on a day the exchange calendar cannot classify.
const OFF_UNCLASSIFIED_KEPT: usize = 188;
/// Version 2: the note.
const OFF_NOTE: usize = 192;
const OFF_CRC: usize = 252;

/// The layout above adds up to exactly one record, checked here rather than in
/// a comment.
const _: () = assert!(OFF_SOURCE + SOURCE_CAPACITY == OFF_NOTE_V1);
const _: () = assert!(OFF_NOTE_V1 + NOTE_CAPACITY_V1 == OFF_CRC);
const _: () = assert!(OFF_CLOSED_DAY == OFF_NOTE_V1);
const _: () = assert!(OFF_CLOSED_DAY + 4 == OFF_UNCLASSIFIED_KEPT);
const _: () = assert!(OFF_UNCLASSIFIED_KEPT + 4 == OFF_NOTE);
const _: () = assert!(OFF_NOTE + NOTE_CAPACITY == OFF_CRC);
const _: () = assert!(OFF_CRC + 4 == RECORD_LEN);

// THE NOTES FILE (D-3341, DB-G23): one entry per version-3 member failure, at
// the byte offset that record names. An entry is
//
//   0..4    `BXNT`
//   4..6    entry layout version, little-endian (`NOTE_ENTRY_VERSION`)
//   6..8    zero
//   8..16   the ordinal of the journal record it belongs to
//   16..20  the reason's length in bytes before any cut
//   20..24  the bytes kept, at most `MAX_WHOLE_NOTE`
//   24..    the kept bytes, UTF-8
//   then    CRC-32C of every byte before it, little-endian
//
// Variable length, so it is addressed by the offset its record carries and
// never by arithmetic; the ordinal inside it is what a reader checks, so an
// offset that lands on another record's note is refused, never shown.

/// The four bytes a whole-note entry starts with.
const NOTE_MAGIC: [u8; 4] = *b"BXNT";

/// The entry layout this build writes and reads.
const NOTE_ENTRY_VERSION: u16 = 1;

/// An entry's fixed head, in bytes, before its text.
const NOTE_HEAD: usize = 24;

/// The longest reason an entry keeps, in bytes: 16 KiB, cut on a character
/// boundary past it, with the whole length still stated in the head.
pub const MAX_WHOLE_NOTE: usize = 16 * 1024;

/// Which shape a record is: the run itself, or one member that did not land.
///
/// # Why a second shape and not a longer note
///
/// A run's note is [`NOTE_CAPACITY`] bytes and holds the **first** failure
/// only. A live run refused 10 of 16 members and the journal kept one reason;
/// the other nine were unrecoverable from disk, so nothing on the machine could
/// say which months had failed. Widening the note is not available — `CLAUDE.md`
/// §4 bans a dynamic schema, and the fixed stride is what makes
/// [`Journal::append`] O(1) and the file scannable at `ordinal * RECORD_LEN`
/// — proved by `api::audit::every_failed_member_gets_its_own_record_at_the_same_stride`,
/// which writes a run and its failures and asserts each lands one whole
/// `RECORD_LEN` after the last, and by
/// `api::audit::appending_creates_the_directory_and_the_count_is_the_length_divided`,
/// which recovers the count by dividing the file length rather than reading it
/// without an index.
///
/// So the run record is followed by **one more fixed-stride record per failed
/// member**. The stride is unchanged, the append is still one `write_all` of
/// [`RECORD_LEN`] bytes, and every failure is on disk. Bounded by the member
/// count, which is bounded by the chunk count — never by rows. See D-0073.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    /// The run as a whole. Code `0`, which is what the reserved byte already
    /// held, so every record written before this existed is one of these.
    Run,
    /// One member that reached the vendor and did not land. `source` carries
    /// the instrument, `note` its reason, and the day fields the month asked
    /// for.
    MemberFailure,
}

impl Kind {
    /// The byte written at [`OFF_KIND`].
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            Self::Run => 0,
            Self::MemberFailure => 1,
        }
    }

    /// The kind a byte names, or [`None`] for one this build does not know.
    ///
    /// Refused rather than defaulted to [`Self::Run`]: a record written by a
    /// later build carries fields this one cannot read, and rendering it as a
    /// run would put invented numbers on the operator's page. `CLAUDE.md` §4 —
    /// degrade loudly and name the reason.
    #[must_use]
    pub const fn of_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::Run),
            1 => Some(Self::MemberFailure),
            _ => None,
        }
    }

    /// What the page calls it.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Run => "run",
            Self::MemberFailure => "member",
        }
    }
}

/// Which form the run came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// `/pull/spot`.
    Spot,
    /// `/pull/fno`.
    Fno,
}

impl Scope {
    /// How many scopes this build knows and — because the codes are dense from
    /// zero — the first byte no scope uses.
    const COUNT: u8 = 2;

    /// Every scope, in stored-byte order.
    pub const ALL: [Self; Self::COUNT as usize] = [Self::Spot, Self::Fno];

    /// The byte this scope is stored as.
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            Self::Spot => 0,
            Self::Fno => 1,
        }
    }

    /// The scope a stored byte names.
    #[must_use]
    pub const fn of_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::Spot),
            1 => Some(Self::Fno),
            _ => None,
        }
    }

    /// What the page calls it.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Spot => "spot",
            Self::Fno => "expired F&O",
        }
    }
}

/// How the run ended.
///
/// Four states and not three: "it stored bars", "I would not accept the
/// request", "I accepted it and there is no transport to serve it" and "the
/// transport ran and something failed" are four different things to an
/// operator, and collapsing any pair of them loses the diagnosis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Bars reached the store and the census counted them.
    Stored,
    /// The run completed cleanly and stored NOTHING.
    ///
    /// Not a failure and not a success. Nothing broke — no member errored and
    /// the books balance — but no bar reached disk, so the window is still
    /// empty and the operator's next action is not "carry on".
    ///
    /// It had no variant, so it was recorded as [`Self::Stored`]: a run over
    /// zero rows has no failures and balances trivially (0 = 0 + 0 + 0). Across
    /// the ~11,200 windows of the stated backfill that is the difference
    /// between a complete history and a gap nobody finds, because the month
    /// file exists and the census counts it.
    Empty,
    /// The request was refused before anything ran.
    Refused,
    /// The request was understood and no transport exists to serve it.
    NotStarted,
    /// Something ran and did not finish clean.
    Failed,
    /// A broker run began: its source and window, written before its first
    /// member (D-3341, DB-055, DB-G46).
    ///
    /// Not a verdict. The run's member failures follow this record as each
    /// member lands, and its own closing record (one of the five above) follows
    /// them. Before it existed a run wrote nothing until it ended, so a stock
    /// pass that ran for hours had no durable trace until its last instrument,
    /// and one whose process stopped had none at all. An opened record with no
    /// closing record after its failures is that case, and the journal now
    /// shows it rather than nothing.
    Opened,
}

impl Outcome {
    /// How many outcomes this build knows and — because the codes are dense
    /// from zero — the first byte no outcome uses.
    ///
    /// Raising this is what forces a new variant into [`Self::ALL`]: the
    /// literal below stops being the right length until it is listed.
    const COUNT: u8 = 6;

    /// Every outcome, in stored-byte order — which is not declaration order,
    /// because [`Self::Empty`] was appended to the code space and reads best
    /// beside [`Self::Stored`]. Byte order is the order that matters here: it
    /// is what makes this list a dense map of the codes on disk, which is what
    /// the assertions under this block rest on.
    pub const ALL: [Self; Self::COUNT as usize] = [
        Self::Stored,
        Self::Refused,
        Self::NotStarted,
        Self::Failed,
        Self::Empty,
        Self::Opened,
    ];

    /// The byte this outcome is stored as.
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            Self::Stored => 0,
            Self::Refused => 1,
            Self::NotStarted => 2,
            Self::Failed => 3,
            // APPENDED, NOT INSERTED. These are the on-disk codes of an
            // append-only journal: renumbering would re-read every record
            // written before today as a different outcome.
            Self::Empty => 4,
            Self::Opened => 5,
        }
    }

    /// The outcome a stored byte names.
    #[must_use]
    pub const fn of_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::Stored),
            1 => Some(Self::Refused),
            2 => Some(Self::NotStarted),
            3 => Some(Self::Failed),
            4 => Some(Self::Empty),
            5 => Some(Self::Opened),
            _ => None,
        }
    }

    /// The one word at the top of a row.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Stored => "STORED",
            Self::Empty => "STORED NOTHING",
            Self::Refused => "REFUSED",
            Self::NotStarted => "NOT STARTED",
            Self::Failed => "FAILED",
            Self::Opened => "OPENED",
        }
    }

    /// Whether an operator has to be told about this row.
    #[must_use]
    pub const fn is_loud(self) -> bool {
        // `Empty` IS loud. It is not an error, but a window that stored nothing
        // is something the operator has to look at — and the whole reason it
        // needed its own variant is that reporting it quietly as `Stored` is
        // how a backfill gap survives.
        //
        // `Opened` is NOT loud: it states that a run began, and the run's
        // verdict is its closing record's to give.
        !matches!(self, Self::Stored | Self::Opened)
    }
}

// THE WRITER AND THE READER, PINNED TO EACH OTHER AT BUILD TIME.
//
// `code` is a `match self`, so the compiler forces a new variant to be given a
// byte. `of_code` is a `match code` ending in `_ => None`, which is total by
// construction — the compiler has nothing to say about it, and that asymmetry
// is how `Empty` came to be written as byte 4 and read back as
// `UnknownOutcome { code: 4 }`. Assertions, not review, close it.
//
// Read together with `ALL`'s length, these say the code space is dense and
// exactly `COUNT` wide: every byte below `COUNT` is a known outcome, and
// `COUNT` itself is not. Teaching `of_code` a new byte therefore fails the
// second assertion, and the only repair is to raise `COUNT` — which makes the
// `ALL` literal the wrong length, which is a build error until the new variant
// is listed there too, which is what the round-trip test walks.
//
// What they do NOT catch: a variant given an arm in `code` and put in neither
// place. Closing that needs the compiler to enumerate an enum's variants, which
// stable Rust will not do. `ALL` is the shortest list to keep honest.
const _: () = {
    let mut code = 0;
    while code < Scope::COUNT {
        assert!(Scope::of_code(code).is_some());
        code += 1;
    }
};
const _: () = assert!(Scope::of_code(Scope::COUNT).is_none());
const _: () = {
    let mut code = 0;
    while code < Outcome::COUNT {
        assert!(Outcome::of_code(code).is_some());
        code += 1;
    }
};
const _: () = assert!(Outcome::of_code(Outcome::COUNT).is_none());

/// Rows that did not become bars, by reason, in the width a record stores.
///
/// A separate type from [`DropCensus`] on purpose. `DropCensus` is a *counter*
/// — its fields are private and `count` is the only way in, so rebuilding one
/// from four totals would mean calling `count` once per drop, which is a walk
/// over a number that was already known. This is the same four facts as a
/// value, so a record read off disk becomes a rendered panel without a loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Drops {
    /// Before 09:15 IST.
    pub before_open: u64,
    /// At or after 15:30 IST.
    pub after_close: u64,
    /// Before the operator's window.
    pub before_window: u64,
    /// After the operator's window.
    pub after_window: u64,
    /// On a day the exchange calendar records closed (P-03, D-2673).
    ///
    /// `None` on a version-1 record: that build had no closed-day filter, so
    /// the count is unknown, and zero would claim no such bar arrived.
    pub on_closed_day: Option<u64>,
    /// NOT a drop: rows KEPT on a day the exchange calendar cannot classify,
    /// counted by name so the keep is not silent. Never part of [`Self::total`].
    /// `None` on a version-1 record, for the same reason.
    pub unclassified_kept: Option<u64>,
}

impl Default for Drops {
    /// Nothing dropped, under the current layout: every counter is a known
    /// zero, including the two version 1 could not carry.
    fn default() -> Self {
        Self {
            before_open: 0,
            after_close: 0,
            before_window: 0,
            after_window: 0,
            on_closed_day: Some(0),
            unclassified_kept: Some(0),
        }
    }
}

impl Drops {
    /// The same counts a filter recorded.
    #[must_use]
    pub fn of_census(census: DropCensus) -> Self {
        Self {
            before_open: u64::from(census.of(DropReason::BeforeSessionOpen)),
            after_close: u64::from(census.of(DropReason::AtOrAfterSessionClose)),
            before_window: u64::from(census.of(DropReason::BeforeWindow)),
            after_window: u64::from(census.of(DropReason::AfterWindow)),
            on_closed_day: Some(u64::from(census.of(DropReason::OnClosedDay))),
            unclassified_kept: Some(u64::from(census.unclassified_kept())),
        }
    }

    /// How many were dropped for one reason, or `None` when this record's
    /// version never counted that reason.
    #[must_use]
    pub const fn recorded(self, reason: DropReason) -> Option<u64> {
        match reason {
            DropReason::OnClosedDay => self.on_closed_day,
            _ => Some(self.of(reason)),
        }
    }

    /// How many were dropped for one reason; zero for a reason this record's
    /// version never counted (see [`Self::recorded`] for the honest answer).
    #[must_use]
    pub const fn of(self, reason: DropReason) -> u64 {
        match reason {
            DropReason::BeforeSessionOpen => self.before_open,
            DropReason::AtOrAfterSessionClose => self.after_close,
            DropReason::BeforeWindow => self.before_window,
            DropReason::AfterWindow => self.after_window,
            DropReason::OnClosedDay => match self.on_closed_day {
                Some(n) => n,
                None => 0,
            },
            // `DropReason` is `#[non_exhaustive]`, so a sixth reason added in
            // `pull` compiles here and reports zero rather than failing to
            // build. It is zero and not a dash because this type carries five
            // counters and a sixth reason has none — the honest answer is that
            // this build never counted it, and `DROP_REASONS` below is what
            // the page iterates, so a reason this build cannot count is also a
            // reason the page never names.
            //
            // NO TEST DRIVES THIS ARM AND NONE CAN. A sixth variant would have
            // to be constructed, and `pull::session::DropReason` is a foreign
            // `#[non_exhaustive]` enum with exactly five constructors. It is
            // named here rather than left for a coverage report to find.
            _ => 0,
        }
    }

    /// How many were dropped in total.
    #[must_use]
    pub const fn total(self) -> u64 {
        self.before_open
            .saturating_add(self.after_close)
            .saturating_add(self.before_window)
            .saturating_add(self.after_window)
            .saturating_add(self.of(DropReason::OnClosedDay))
    }

    /// The largest single reason, or one, so a share is never divided by zero.
    #[must_use]
    pub const fn peak(self) -> u64 {
        let mut top = self.before_open;
        if self.after_close > top {
            top = self.after_close;
        }
        if self.before_window > top {
            top = self.before_window;
        }
        if self.after_window > top {
            top = self.after_window;
        }
        if self.of(DropReason::OnClosedDay) > top {
            top = self.of(DropReason::OnClosedDay);
        }
        if top == 0 { 1 } else { top }
    }
}

/// Every reason this build counts, in the order a page shows them.
///
/// [`DropReason`] is `#[non_exhaustive]` so there is no `ALL` to iterate. The
/// four are listed once, here, and every surface reads this rather than writing
/// its own list — which is what stops a page and a record disagreeing about
/// which reasons exist.
pub const DROP_REASONS: [DropReason; 5] = [
    DropReason::BeforeWindow,
    DropReason::AfterWindow,
    DropReason::BeforeSessionOpen,
    DropReason::AtOrAfterSessionClose,
    DropReason::OnClosedDay,
];

/// One pull, as it is written down.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    /// The layout this record was decoded from, or [`VERSION`] for one built
    /// in memory. Decides only how long [`Self::note`] could have been.
    pub version: u16,
    /// Whether this is the run or one member that did not land.
    ///
    /// Every constructor but [`Record::member_failure`] makes a [`Kind::Run`],
    /// and code `0` is what the reserved byte already held — so a journal
    /// written before this field existed decodes as runs, unchanged.
    pub kind: Kind,
    /// When the answer was written, in epoch seconds.
    pub at_unix_secs: i64,
    /// How long the run took, measured across the call that did the work.
    pub elapsed_micros: u64,
    /// Which form it came from.
    pub scope: Scope,
    /// How it ended.
    pub outcome: Outcome,
    /// Members the folder held.
    pub members: u64,
    /// Rows read out of the vendor's files.
    pub rows_read: u64,
    /// Bars OFFERED to the store — not necessarily written.
    ///
    /// **The caption said "written" and this counts what was offered.** They
    /// diverge on every re-pull: a second run over a window offers every bar
    /// and writes none, because the store already holds them byte for byte,
    /// which `CLAUDE.md` §3 rule 5 requires. Measured on a real store:
    /// `bars_stored: 1643341, bars_committed: 0` — sixteen lakh offered, zero
    /// written, under a caption promising they were written.
    ///
    /// The FIGURE cannot be corrected here, only the claim: `Record` is a
    /// 256-byte fixed-stride image whose field map allocates every byte but
    /// one, and a `u64` needs eight. Carrying the committed count is a new
    /// `VERSION` at its own stride under §3 rule 8, not a field. The live pull
    /// receipt shows both numbers because it builds from `Ingested` directly.
    pub bars_stored: u64,
    /// Rows merged into a bar that was already open.
    pub rows_folded: u64,
    /// Slices the census holds after the run.
    pub counted: u64,
    /// Rows dropped, by reason.
    pub drops: Drops,
    /// Members that failed.
    pub failures: u64,
    /// The window's first day, as days from the epoch.
    pub from_days: u32,
    /// The window's last day, as days from the epoch.
    pub to_days: u32,
    /// What was asked for: the target, or the folder.
    pub source: String,
    /// How long `source` was before it was cut, in bytes.
    pub source_bytes: u16,
    /// Why it ended the way it did, in the refusal's own words.
    pub note: String,
    /// How long `note` was before it was cut, in bytes.
    pub note_bytes: u16,
    /// Where this member failure's whole reason is in the notes file beside
    /// the journal ([`Journal::notes_path`]), as a byte offset.
    ///
    /// `Some` only on a [`VERSION_WHOLE_NOTE`] member failure — one whose reason
    /// did not fit [`NOTE_CAPACITY`] and reached that file. [`None`] on every
    /// run, on every member failure whose reason fit, on every record older
    /// than D-3341, and on one whose whole reason could not be written (its
    /// `note_bytes` still says how much was cut). D-3341, DB-G23.
    pub note_at: Option<u64>,
}

impl Record {
    /// A record of a run that never ran.
    #[must_use]
    pub fn refused(
        scope: Scope,
        outcome: Outcome,
        at: SystemTime,
        source: &str,
        note: &str,
    ) -> Self {
        Self {
            version: VERSION,
            kind: Kind::Run,
            at_unix_secs: crate::ingest::epoch_secs(at),
            elapsed_micros: 0,
            scope,
            outcome,
            members: 0,
            rows_read: 0,
            bars_stored: 0,
            rows_folded: 0,
            counted: 0,
            drops: Drops::default(),
            failures: 0,
            from_days: 0,
            to_days: 0,
            source: keep(source, SOURCE_CAPACITY),
            source_bytes: saturating_len(source),
            note: keep(note, NOTE_CAPACITY),
            note_bytes: saturating_len(note),
            note_at: None,
        }
    }

    /// The window this record is about.
    #[must_use]
    pub fn with_window(mut self, window: Window) -> Self {
        self.from_days = window.from().days_from_epoch();
        self.to_days = window.to().days_from_epoch();
        self
    }

    /// What a run actually moved, for a path that has counts but no [`Ingested`].
    ///
    /// # The zeroes this exists to remove
    ///
    /// [`Record::refused`] hardcodes `members`, `rows_read`, `bars_stored`,
    /// `rows_folded`, `counted` and `failures` to zero, which is right for the
    /// thing it is named after — a run that never ran stored nothing. **The
    /// expired-derivative route stamps every one of its outcomes through it,
    /// including its successes**, because [`Record::of_run`] takes an
    /// [`Ingested`] and that path does not build one: it lands contract by
    /// contract and keeps its own tallies.
    ///
    /// The measured consequence, from the operator's own journal on 2026-08-20:
    /// two records whose note reads *"every discovered contract fetched and
    /// filed"* and whose `bars_stored` is `0` — not because nothing landed, but
    /// because no code path could ever write anything else there. Every numeric
    /// field of the F&O surface was structurally zero, so `/audit.json`, the
    /// audit page and any ladder reading the journal could not distinguish a
    /// working expired-derivative pull from a broken one.
    ///
    /// That is the shape `CLAUDE.md` §4 bans read backwards: not a failure
    /// wearing a success's clothes, but a success with no clothes at all — and
    /// it is worse, because the operator's only recourse was to count files on
    /// disk by hand.
    ///
    /// # Why a builder and not a second constructor
    ///
    /// Because the outcome, the note and the window are decided identically on
    /// every arm of that route and only the counts differ. A second constructor
    /// would duplicate the five fields that do not vary, and the copy that
    /// drifts is the one nobody remembers exists — the same argument
    /// [`with_window`] settles one method up.
    ///
    /// `rows_folded` and `counted` stay zero deliberately: the F&O path folds
    /// nothing into an open bar and does not re-count the census per contract,
    /// so writing a number there would be an invention rather than a
    /// measurement. `CLAUDE.md` §3 rule 6.
    ///
    /// # Cost
    ///
    /// Four field assignments. The record's stride is unchanged — these are
    /// fields the format has always had and this path has never filled.
    #[must_use]
    pub const fn with_counts(
        mut self,
        members: u64,
        rows_read: u64,
        bars_stored: u64,
        failures: u64,
    ) -> Self {
        self.members = members;
        self.rows_read = rows_read;
        self.bars_stored = bars_stored;
        self.failures = failures;
        self
    }

    /// A record of a run that ran.
    ///
    /// Every figure comes off [`Ingested`] rather than being recomputed here: a
    /// second arithmetic over the same counts is a second answer waiting to
    /// disagree with the first.
    #[must_use]
    pub fn of_run(
        scope: Scope,
        at: SystemTime,
        elapsed_micros: u64,
        source: &str,
        window: Window,
        done: &Ingested,
    ) -> Self {
        let note = done.failures.first().map_or_else(
            || {
                if done.balances() {
                    "every row accounted for: stored, folded into an open bar, or dropped"
                        .to_owned()
                } else {
                    "THE BOOKS DO NOT BALANCE — see the run's own page".to_owned()
                }
            },
            |first| format!("{} — {}", first.instrument, first.why),
        );
        let outcome = if !done.failures.is_empty() || !done.balances() {
            Outcome::Failed
        } else if done.bars_stored == 0 {
            // CLEAN, AND EMPTY. `balances()` is true here — every row read was
            // accounted for — but zero rows read balances just as trivially as
            // a thousand, so this arm is what stops a window that landed
            // nothing from reporting success.
            Outcome::Empty
        } else {
            Outcome::Stored
        };
        Self {
            version: VERSION,
            kind: Kind::Run,
            at_unix_secs: crate::ingest::epoch_secs(at),
            elapsed_micros,
            scope,
            outcome,
            members: as_u64(done.members),
            rows_read: as_u64(done.rows_read),
            bars_stored: as_u64(done.bars_stored),
            rows_folded: as_u64(done.rows_folded),
            counted: as_u64(done.counted),
            drops: Drops::of_census(done.census),
            failures: as_u64(done.failures.len()),
            from_days: window.from().days_from_epoch(),
            to_days: window.to().days_from_epoch(),
            source: keep(source, SOURCE_CAPACITY),
            source_bytes: saturating_len(source),
            note: keep(&note, NOTE_CAPACITY),
            note_bytes: saturating_len(&note),
            note_at: None,
        }
    }

    /// One member that reached the vendor and did not land, as its own record.
    ///
    /// # What this recovers, and what it still cannot
    ///
    /// [`Self::of_run`] keeps `done.failures.first()` and nothing else, because
    /// the note is [`NOTE_CAPACITY`] bytes and the stride is fixed. A run that
    /// refused 10 members left 9 reasons nowhere on disk, so no process on the
    /// machine could say which months had failed — it had to be worked out by
    /// decoding bar files.
    ///
    /// One of these per failure puts **every failed (instrument, month) pair**
    /// in the journal. The reason is still cut at [`NOTE_CAPACITY`] and
    /// `note_bytes` still says by how much — that half is answered by the
    /// `pull.spot` `member did not land` event, whose field is not stride-bound.
    /// Two surfaces, each doing what its format can actually do.
    ///
    /// The counters are zero on purpose: they belong to the run, and repeating
    /// them here would give a reader two places to add up the same numbers from
    /// and two answers when they drifted. `elapsed_micros` is zero for the same
    /// reason — a member is not separately timed.
    #[must_use]
    pub fn member_failure(
        scope: Scope,
        at: SystemTime,
        instrument: &str,
        window: Window,
        why: &str,
    ) -> Self {
        Self {
            version: VERSION,
            kind: Kind::MemberFailure,
            at_unix_secs: crate::ingest::epoch_secs(at),
            elapsed_micros: 0,
            scope,
            outcome: Outcome::Failed,
            members: 0,
            rows_read: 0,
            bars_stored: 0,
            rows_folded: 0,
            counted: 0,
            drops: Drops::default(),
            failures: 1,
            from_days: window.from().days_from_epoch(),
            to_days: window.to().days_from_epoch(),
            source: keep(instrument, SOURCE_CAPACITY),
            source_bytes: saturating_len(instrument),
            note: keep(why, NOTE_CAPACITY),
            note_bytes: saturating_len(why),
            note_at: None,
        }
    }

    /// The first record of a broker run, written before its first member.
    ///
    /// # Why a run now opens a record (D-3341, DB-055, DB-G46)
    ///
    /// A run wrote its record and its member failures only when it ended. The
    /// operator's swept 1min pass started at 19:31 IST and `pull.journal` was
    /// last written at 19:00, at the end of the indices run before it: for the
    /// hours a stock pass runs, nothing durable named a single refusal, and the
    /// rolling log that did wrapped in 6 to 10 minutes. Now the run's member
    /// failures are appended as each member lands, and this record goes first
    /// so that they are filed under the run they belong to.
    ///
    /// `source` is what the run is fetched from — the endpoint, carried through
    /// the cash-identity policy exactly as the closing record's is — because a
    /// member failure belongs to the run record before it, and that run's
    /// source is how `/audit.json` names its vendor. The counters are zero:
    /// nothing has landed yet, and the closing record carries what did.
    ///
    /// # The note names the run's ASK (DB-B34, repair 6; D-4950)
    ///
    /// `ask` is [`ask_digest`] of what the run asks the vendor for beyond its
    /// window and source: the feed, the identity policy, the target, the
    /// instruments named and the rung. The source cannot say it: it is the
    /// policy and the endpoint TEMPLATE, the same for every target and rung,
    /// and the 64-byte field keeps `zerodha_nse_checked NOT vendor-ISIN; NSE
    /// file same day; https://` of every Zerodha run. The restart scan reads a
    /// later run as having made a cut-short run's pull again only when both
    /// opening records name the same ask ([`ask_of`]).
    #[must_use]
    pub fn opened(
        scope: Scope,
        at: SystemTime,
        source: &str,
        window: Window,
        of: usize,
        ask: u64,
    ) -> Self {
        let note = ask_note(&format!("opened for {of} instruments"), ask);
        Self::refused(scope, Outcome::Opened, at, source, &note).with_window(window)
    }

    /// Whether the source on the page is shorter than the one that was asked
    /// for.
    #[must_use]
    pub fn source_was_cut(&self) -> bool {
        usize::from(self.source_bytes) > self.source.len()
    }

    /// The longest note this record's version could hold, in bytes.
    #[must_use]
    pub const fn note_capacity(&self) -> usize {
        if self.version == VERSION_ONE {
            NOTE_CAPACITY_V1
        } else {
            NOTE_CAPACITY
        }
    }

    /// Whether the note on the page is shorter than the one that was written.
    #[must_use]
    pub fn note_was_cut(&self) -> bool {
        usize::from(self.note_bytes) > self.note.len()
    }

    /// Exactly the bytes to append, checksum included.
    #[must_use]
    pub fn image(&self) -> [u8; RECORD_LEN] {
        let mut out = [0u8; RECORD_LEN];
        // VERSION 3 ONLY FOR A MEMBER FAILURE THAT NAMES ITS WHOLE NOTE; every
        // other record is version 2, byte for byte as before (D-3341).
        let whole = match (self.kind, self.note_at) {
            (Kind::MemberFailure, Some(at)) => Some(at),
            _ => None,
        };
        write_at(&mut out, OFF_MAGIC, MAGIC);
        write_at(
            &mut out,
            OFF_VERSION,
            whole.map_or(VERSION, |_| VERSION_WHOLE_NOTE).to_le_bytes(),
        );
        write_at(&mut out, OFF_SCOPE, [self.scope.code()]);
        write_at(&mut out, OFF_OUTCOME, [self.outcome.code()]);
        write_at(&mut out, OFF_KIND, [self.kind.code()]);
        write_at(&mut out, OFF_AT, self.at_unix_secs.to_le_bytes());
        write_at(&mut out, OFF_ELAPSED, self.elapsed_micros.to_le_bytes());
        write_at(&mut out, OFF_MEMBERS, self.members.to_le_bytes());
        write_at(&mut out, OFF_ROWS_READ, self.rows_read.to_le_bytes());
        write_at(&mut out, OFF_BARS_STORED, self.bars_stored.to_le_bytes());
        write_at(&mut out, OFF_ROWS_FOLDED, self.rows_folded.to_le_bytes());
        // A member failure's `counted` is zero by design, so version 3 keeps
        // the whole note's offset in those eight bytes.
        write_at(
            &mut out,
            OFF_COUNTED,
            whole.unwrap_or(self.counted).to_le_bytes(),
        );
        write_at(
            &mut out,
            OFF_BEFORE_OPEN,
            self.drops.before_open.to_le_bytes(),
        );
        write_at(
            &mut out,
            OFF_AFTER_CLOSE,
            self.drops.after_close.to_le_bytes(),
        );
        write_at(
            &mut out,
            OFF_BEFORE_WINDOW,
            self.drops.before_window.to_le_bytes(),
        );
        write_at(
            &mut out,
            OFF_AFTER_WINDOW,
            self.drops.after_window.to_le_bytes(),
        );
        // Version 2's two counters, as `u32` because the census that feeds
        // them is `u32`; a larger figure built by hand saturates rather than
        // wrapping to a smaller one.
        write_at(
            &mut out,
            OFF_CLOSED_DAY,
            narrow(self.drops.of(DropReason::OnClosedDay)).to_le_bytes(),
        );
        write_at(
            &mut out,
            OFF_UNCLASSIFIED_KEPT,
            narrow(self.drops.unclassified_kept.unwrap_or(0)).to_le_bytes(),
        );
        write_at(&mut out, OFF_FAILURES, self.failures.to_le_bytes());
        write_at(&mut out, OFF_FROM_DAYS, self.from_days.to_le_bytes());
        write_at(&mut out, OFF_TO_DAYS, self.to_days.to_le_bytes());
        write_at(&mut out, OFF_SOURCE_LEN, self.source_bytes.to_le_bytes());
        write_at(&mut out, OFF_NOTE_LEN, self.note_bytes.to_le_bytes());
        write_at(&mut out, OFF_SOURCE_KEPT, [kept_byte(&self.source)]);
        // A version-1 note decoded at 68 bytes is cut to version 2's 60 when it
        // is written again, so the kept byte never names more than the field.
        let note = keep(&self.note, NOTE_CAPACITY);
        write_at(&mut out, OFF_NOTE_KEPT, [kept_byte(&note)]);
        write_text(&mut out, OFF_SOURCE, &self.source, SOURCE_CAPACITY);
        write_text(&mut out, OFF_NOTE, &note, NOTE_CAPACITY);
        let crc = crc32c(&covered(&out));
        write_at(&mut out, OFF_CRC, crc.to_le_bytes());
        out
    }

    /// One record, decoded and checksum-verified.
    ///
    /// # Errors
    ///
    /// [`RecordFault`], checked in this order: magic, checksum, version, then
    /// each field. The checksum before the version, because a version refusal
    /// read out of bytes that already failed their checksum is a diagnosis of
    /// corruption as a schema problem.
    pub fn decode(image: &[u8; RECORD_LEN]) -> Result<Self, RecordFault> {
        if le4(image, OFF_MAGIC) != MAGIC {
            return Err(RecordFault::NotAnAuditRecord);
        }
        let stored = u32::from_le_bytes(le4(image, OFF_CRC));
        let computed = crc32c(&covered(image));
        if stored != computed {
            return Err(RecordFault::Checksum { stored, computed });
        }
        let version = u16::from_le_bytes(le2(image, OFF_VERSION));
        if version != VERSION && version != VERSION_ONE && version != VERSION_WHOLE_NOTE {
            return Err(RecordFault::UnknownVersion { version });
        }
        // Versions 2 and 3 share one layout; only version 1's note is longer.
        let current = version != VERSION_ONE;
        let scope_code = byte(image, OFF_SCOPE);
        let Some(scope) = Scope::of_code(scope_code) else {
            return Err(RecordFault::UnknownScope { code: scope_code });
        };
        let outcome_code = byte(image, OFF_OUTCOME);
        let Some(outcome) = Outcome::of_code(outcome_code) else {
            return Err(RecordFault::UnknownOutcome { code: outcome_code });
        };
        // REFUSED, NOT DEFAULTED TO `Run`. A record a later build wrote carries
        // fields this one cannot read; rendering it as a run would put invented
        // numbers on the operator's page, which is worse than a named refusal.
        let kind_code = byte(image, OFF_KIND);
        let Some(kind) = Kind::of_code(kind_code) else {
            return Err(RecordFault::UnknownKind { code: kind_code });
        };
        // A WHOLE-NOTE OFFSET IS A MEMBER FAILURE'S ALONE. A run carrying
        // version 3 would have its census count read as an offset, so it is
        // refused by name rather than decoded either way.
        let whole = version == VERSION_WHOLE_NOTE;
        if whole && kind != Kind::MemberFailure {
            return Err(RecordFault::WholeNoteOnARun);
        }
        let counted = u64::from_le_bytes(le8(image, OFF_COUNTED));
        let source = text(
            image,
            OFF_SOURCE,
            byte(image, OFF_SOURCE_KEPT),
            SOURCE_CAPACITY,
        )?;
        let note = if current {
            text(image, OFF_NOTE, byte(image, OFF_NOTE_KEPT), NOTE_CAPACITY)?
        } else {
            text(
                image,
                OFF_NOTE_V1,
                byte(image, OFF_NOTE_KEPT),
                NOTE_CAPACITY_V1,
            )?
        };
        let newer =
            |offset: usize| current.then(|| u64::from(u32::from_le_bytes(le4(image, offset))));
        Ok(Self {
            version,
            kind,
            at_unix_secs: i64::from_le_bytes(le8(image, OFF_AT)),
            elapsed_micros: u64::from_le_bytes(le8(image, OFF_ELAPSED)),
            scope,
            outcome,
            members: u64::from_le_bytes(le8(image, OFF_MEMBERS)),
            rows_read: u64::from_le_bytes(le8(image, OFF_ROWS_READ)),
            bars_stored: u64::from_le_bytes(le8(image, OFF_BARS_STORED)),
            rows_folded: u64::from_le_bytes(le8(image, OFF_ROWS_FOLDED)),
            counted: if whole { 0 } else { counted },
            drops: Drops {
                before_open: u64::from_le_bytes(le8(image, OFF_BEFORE_OPEN)),
                after_close: u64::from_le_bytes(le8(image, OFF_AFTER_CLOSE)),
                before_window: u64::from_le_bytes(le8(image, OFF_BEFORE_WINDOW)),
                after_window: u64::from_le_bytes(le8(image, OFF_AFTER_WINDOW)),
                on_closed_day: newer(OFF_CLOSED_DAY),
                unclassified_kept: newer(OFF_UNCLASSIFIED_KEPT),
            },
            failures: u64::from_le_bytes(le8(image, OFF_FAILURES)),
            from_days: u32::from_le_bytes(le4(image, OFF_FROM_DAYS)),
            to_days: u32::from_le_bytes(le4(image, OFF_TO_DAYS)),
            source_bytes: u16::from_le_bytes(le2(image, OFF_SOURCE_LEN)),
            source,
            note_bytes: u16::from_le_bytes(le2(image, OFF_NOTE_LEN)),
            note,
            note_at: whole.then_some(counted),
        })
    }
}

/// Why one record could not be believed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RecordFault {
    /// The first four bytes are not this journal's.
    NotAnAuditRecord,
    /// The stored checksum does not match the bytes.
    Checksum {
        /// What the record carries.
        stored: u32,
        /// What its bytes actually produce.
        computed: u32,
    },
    /// A layout this build does not know.
    UnknownVersion {
        /// The version the record claims.
        version: u16,
    },
    /// A scope byte outside the two this build writes.
    UnknownScope {
        /// The byte found.
        code: u8,
    },
    /// An outcome byte outside the four this build writes.
    UnknownOutcome {
        /// The byte found.
        code: u8,
    },
    /// A record-kind byte outside the two this build writes.
    ///
    /// Byte 118 was reserved and zero before [`Kind`] existed, and `0` is
    /// [`Kind::Run`], so this cannot fire on an older journal. It fires on one
    /// written by a LATER build, which is exactly when guessing would be worst.
    UnknownKind {
        /// The byte found.
        code: u8,
    },
    /// A kept length past the field it names.
    KeptPastCapacity {
        /// The length the record claims.
        kept: u8,
        /// The field it has.
        capacity: u8,
    },
    /// Text that is not UTF-8.
    NotText,
    /// A version-3 record that is not a member failure (D-3341).
    WholeNoteOnARun,
}

impl std::fmt::Display for RecordFault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::NotAnAuditRecord => f.write_str("not an audit record"),
            Self::Checksum { stored, computed } => {
                write!(f, "record checksum {stored:#010x} != {computed:#010x}")
            }
            Self::UnknownVersion { version } => {
                write!(
                    f,
                    "record version {version}; this build writes {VERSION} \
                     ({VERSION_WHOLE_NOTE} for a member failure whose whole note \
                     is kept) and reads {VERSION_ONE}, {VERSION} and \
                     {VERSION_WHOLE_NOTE}"
                )
            }
            Self::UnknownScope { code } => {
                write!(f, "scope byte {code} is not one this build writes")
            }
            Self::UnknownOutcome { code } => {
                write!(f, "outcome byte {code} is not one this build writes")
            }
            Self::UnknownKind { code } => {
                write!(
                    f,
                    "record-kind byte {code} is not one this build writes — a \
                     later build wrote this record and it is not read rather \
                     than being rendered as a run it is not"
                )
            }
            Self::KeptPastCapacity { kept, capacity } => {
                write!(f, "kept length {kept} past a {capacity}-byte field")
            }
            Self::NotText => f.write_str("a text field is not UTF-8"),
            Self::WholeNoteOnARun => f.write_str(
                "a version-3 record that is not a member failure: only a member \
                 failure carries a whole-note offset, so this record is not read",
            ),
        }
    }
}

impl std::error::Error for RecordFault {}

/// What the journal file was found to be.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Log {
    /// No file at that path. The ordinary state before the first pull.
    Absent,
    /// A file that exists and cannot be measured.
    Unreadable {
        /// What refused it, in the refusal's own words.
        reason: String,
    },
    /// A file that is there, with the number of whole records it holds.
    Held {
        /// How many whole records. `len / RECORD_LEN`, one field read.
        records: u64,
        /// How large the file is, so the growth is a number and not a worry.
        bytes: u64,
        /// The bytes past the last whole record, when there are any.
        ///
        /// `Some` is a torn write and is loud. It never changes what the whole
        /// records say — they are before it — so the page shows both.
        torn: Option<u64>,
    },
}

impl Log {
    /// How many whole records are readable.
    #[must_use]
    pub const fn records(&self) -> u64 {
        match *self {
            Self::Held { records, .. } => records,
            Self::Absent | Self::Unreadable { .. } => 0,
        }
    }

    /// Whether an operator has to be told about the file itself.
    #[must_use]
    pub const fn is_loud(&self) -> bool {
        match *self {
            Self::Unreadable { .. } => true,
            Self::Held { torn, .. } => torn.is_some(),
            Self::Absent => false,
        }
    }
}

/// One run's place in the journal: its opening record, that record's ordinal,
/// and the ordinal of the last record the run appended. Made by
/// [`Journal::open_run`] and moved on by [`Journal::append_failure`]. D-3341.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Thread {
    head: Record,
    first: u64,
    last: u64,
}

impl Thread {
    /// The ordinal of the run's opening record.
    #[must_use]
    pub const fn first(&self) -> u64 {
        self.first
    }

    /// The ordinal of the last record this run appended.
    #[must_use]
    pub const fn last(&self) -> u64 {
        self.last
    }
}

/// The journal file, and where it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Journal {
    /// The file every record is appended to.
    pub path: PathBuf,
}

/// The directory the journal lives in, beneath the store root.
///
/// Beside `manifest/`, not inside it: a manifest is a counter this build must
/// be able to rewrite, and this is a history it must never rewrite. Two
/// different rules, two different directories.
#[must_use]
pub fn journal_path(root: &Path) -> PathBuf {
    root.join("audit").join("pull.journal")
}

impl Journal {
    /// The journal beneath a store root.
    #[must_use]
    pub fn at(root: &Path) -> Self {
        Self {
            path: journal_path(root),
        }
    }

    /// What is there, from one `metadata` call and no read.
    ///
    /// Never fails: every outcome is a state the page renders, for the reason
    /// `crates/api/src/census.rs` gives about an absent manifest.
    #[must_use]
    pub fn look(&self) -> Log {
        let log = match std::fs::metadata(&self.path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Log::Absent,
            Err(e) => Log::Unreadable {
                reason: e.to_string(),
            },
            Ok(meta) => {
                let bytes = meta.len();
                let records = bytes / RECORD_LEN_U64;
                let spare = bytes % RECORD_LEN_U64;
                Log::Held {
                    records,
                    bytes,
                    torn: if spare == 0 { None } else { Some(spare) },
                }
            }
        };
        note_looked(&self.path, &log);
        log
    }

    /// Appends one record and makes it durable.
    ///
    /// # Errors
    ///
    /// The failure in its own words, prefixed with the path. A pull whose
    /// record could not be written is a pull the operator has to be told about
    /// on the answer page — silently losing the record of a run that wrote
    /// 9.8 MB of bars is exactly the fallback `CLAUDE.md` §4 forbids.
    ///
    /// # One arm here is a backstop and no test drives it
    ///
    /// `sync_all`: reaching it needs a failing `fsync`, which a developer's
    /// disk does not enter on request. The failed `write_all` is driven, through
    /// [`write_rolled_back`]'s injected write (D-1854). `CLAUDE.md` §3 rule 6:
    /// said, rather than left to be found.
    ///
    /// # A refused unlock after the sync is not a failed append
    ///
    /// Once `sync_all` has returned, the record is on disk, and an error from
    /// here says it is not: `recorded_fact` renders "NO — this run is NOT in
    /// the journal", and the spot path returns before it appends the run's
    /// member-failure records. So the lock is not released by
    /// `Flock::release`, whose refusal would be returned, but by the guard's
    /// `Drop`, whose refusal `store.flock` logs as a Warn and does not return,
    /// as `pull::masters::land_validated`'s `_lock` guard is released after
    /// its file is written (D-0693). A lock still held refuses the next append
    /// by name, as `a_held_journal_lock_refuses_a_second_writer_without_appending`
    /// pins for a lock another handle holds. No test here can refuse an unlock:
    /// the seam that does is `#[cfg(test)]` inside `store`. So
    /// `nothing_after_the_sync_can_refuse_an_append` pins the source instead.
    /// D-0695.
    pub fn append(&self, record: &Record) -> Result<(), String> {
        // ONE LINE WHEN THE RUN DOES NOT MAKE IT ONTO THE RECORD, and none when
        // it does. The refusal below is already returned to the caller and
        // rendered on the answer page — but the answer page is a browser tab
        // somebody has to still be looking at, and this is the surface whose
        // whole purpose is to survive the process. A pull that wrote 9.8 MB of
        // bars and lost its own receipt left nothing anywhere.
        //
        // Bounded by the append, which is bounded by the run and its failed
        // members — never by rows. See [`Kind::MemberFailure`].
        self.appended(record, None)
            .inspect_err(|why| note_append_failed(&self.path, record, why))
    }

    /// Appends one member failure, keeping its WHOLE reason beside it when the
    /// record's note field cut it (D-3341, DB-G23).
    ///
    /// `whole` is the reason `record` was built from. When it is longer than
    /// the note the record kept, it is appended to [`Self::notes_path`] first,
    /// under that file's own lock and while this journal's is held, and the
    /// record is written as [`VERSION_WHOLE_NOTE`] naming where it went. The
    /// note entry carries this record's ordinal, so a reader that follows the
    /// offset checks it landed on the right reason.
    ///
    /// # A whole note that cannot be written does not lose the record
    ///
    /// The record is the fact that the member failed; the whole note is its
    /// detail. When the note cannot be written the record is still appended,
    /// as version 2 with its cut note and `note_bytes` saying by how much, and
    /// the refusal is logged at `Error` (`api.audit` / `whole note not
    /// appended`), which the audit stream keeps. Losing the record because its
    /// detail failed would be the worse of the two losses.
    ///
    /// # Cost
    ///
    /// One more locked append and `fsync` than [`Self::append`] when the note
    /// was cut, none when it fit. Constant per failure, bounded by the member
    /// count, never by rows.
    ///
    /// # Errors
    ///
    /// As [`Self::append`]: the record itself not reaching the journal.
    pub fn append_member(&self, record: &Record, whole: &str) -> Result<(), String> {
        self.appended(record, Some(whole))
            .inspect_err(|why| note_append_failed(&self.path, record, why))
    }

    /// The file each version-3 member failure's whole reason is kept in: the
    /// journal's own name with `.notes` for `.journal`, beside it in `audit/`.
    #[must_use]
    pub fn notes_path(&self) -> PathBuf {
        self.path.with_extension("notes")
    }

    /// Opens a run in the journal: appends its [`Outcome::Opened`] record and
    /// answers the [`Thread`] its member failures are appended through.
    ///
    /// # Errors
    ///
    /// As [`Self::append`]: the opening record did not reach the journal.
    pub fn open_run(&self, head: &Record) -> Result<Thread, String> {
        self.appended_images(|_ordinal| head.image().to_vec())
            .map(|first| Thread {
                head: head.clone(),
                first,
                last: first,
            })
            .inspect_err(|why| note_append_failed(&self.path, head, why))
    }

    /// Appends one member failure of the run `thread` opened, with its whole
    /// reason kept as [`Self::append_member`] keeps it — and the run's opening
    /// record again first, whenever another writer appended since this run's
    /// last record (D-3341, DB-055).
    ///
    /// # Why the opening record is repeated
    ///
    /// A member failure carries no vendor and no run of its own: `/audit.json`
    /// and `/dayflags.json` file it under the run record before it. A run that
    /// appended its failures only when it ended wrote them in one burst behind
    /// its own record, so that held. Appended as each member lands, two runs
    /// of two feeds interleave, and a failure would be filed under the other
    /// run's feed. So the check is made under the journal's lock, where it is
    /// exact: if the record before this one is not this run's last, the
    /// opening record is written again ("continues ...") in the same write as
    /// the failure. A run that has the journal to itself never repeats it.
    ///
    /// # Errors
    ///
    /// As [`Self::append`]: the record did not reach the journal. Nothing of
    /// this call is then on disk, and `thread` is unchanged.
    pub fn append_failure(
        &self,
        thread: &mut Thread,
        record: &Record,
        whole: &str,
    ) -> Result<(), String> {
        self.threaded(thread, record, Some(whole))
    }

    /// Appends the closing record of the run `thread` opened, DIRECTLY behind a
    /// record of that run: behind its last record when nothing was appended
    /// since, else behind its opening record written again ("continues ..."),
    /// in the same write (DB-B34, repair 6; D-4950).
    ///
    /// # Why the close goes through the run's thread
    ///
    /// A closing record does not name its opening record, and the restart
    /// scan paired it with the newest open run of its window and source. Every
    /// Zerodha spot run of a window carries one source (the identity policy
    /// and the endpoint template, cut to 64 bytes), so two runs of one window
    /// in flight at once (a server run and a `cli` pull) were told apart by
    /// nothing, and the close of the older named the newer. Written this way,
    /// a close always follows a record of its own run, and the scan pairs it
    /// with that run (`crate::pullrun`), whoever else appended meanwhile.
    ///
    /// # Errors
    ///
    /// As [`Self::append`]: nothing of this call is then on disk, and
    /// `thread` is unchanged.
    pub fn close_run(&self, thread: &mut Thread, record: &Record) -> Result<(), String> {
        self.threaded(thread, record, None)
    }

    /// Appends the closing record of a run whose opening record `head` was
    /// REFUSED when the run began, behind that opening record, in one write:
    /// the opening record keeps its own time and gains `; late` in its note
    /// (DB-B34, repair 6; D-4950).
    ///
    /// # Why the opening record is written late rather than not at all
    ///
    /// A run whose opening record was refused (the journal's lock taken by
    /// another writer's append at that instant) still ran and still closes. Its
    /// close, alone, would be paired by the restart scan with the newest open
    /// run of its window and source: a run cut short of ANOTHER target or rung
    /// over that window would read as closed by a run that never asked for its
    /// instruments. Behind its own opening record the close pairs with its own
    /// run. Its member failures were not written during the run; the caller
    /// writes them after this ([`Self::append_member`]), as before D-3341.
    ///
    /// # Errors
    ///
    /// As [`Self::append`]: neither record is then on disk.
    pub fn close_unopened(&self, head: &Record, record: &Record) -> Result<(), String> {
        let mut late = head.clone();
        let note = format!("{}; late", head.note);
        late.note = keep(&note, NOTE_CAPACITY);
        late.note_bytes = saturating_len(&note);
        self.appended_images(|_ordinal| {
            let mut out = Vec::with_capacity(2 * RECORD_LEN);
            out.extend_from_slice(&late.image());
            out.extend_from_slice(&record.image());
            out
        })
        .map(|_first| ())
        .inspect_err(|why| note_append_failed(&self.path, record, why))
    }

    /// [`Self::append_failure`] and [`Self::close_run`]: `record` behind a
    /// record of `thread`'s run, its opening record written again first when
    /// another writer appended since the run's last record. `whole` is a
    /// member failure's whole reason ([`Self::append_member`]); `None` for a
    /// closing record.
    fn threaded(
        &self,
        thread: &mut Thread,
        record: &Record,
        whole: Option<&str>,
    ) -> Result<(), String> {
        let mut written = 1u64;
        let first = self
            .appended_images(|ordinal| {
                let mut out = Vec::with_capacity(2 * RECORD_LEN);
                let mut at = ordinal;
                if ordinal != thread.last.saturating_add(1) {
                    let mut again = thread.head.clone();
                    let note = format!("continues the run opened at record {}", thread.first);
                    again.note = keep(&note, NOTE_CAPACITY);
                    again.note_bytes = saturating_len(&note);
                    out.extend_from_slice(&again.image());
                    at = at.saturating_add(1);
                    written = 2;
                }
                out.extend_from_slice(&self.with_whole_note(record, whole, at).image());
                out
            })
            .inspect_err(|why| note_append_failed(&self.path, record, why))?;
        thread.last = first.saturating_add(written).saturating_sub(1);
        Ok(())
    }

    /// `record`, carrying where its whole reason went when it was a member
    /// failure whose note was cut and the reason reached [`Self::notes_path`]
    /// as the entry of record `ordinal`. A note that cannot be written leaves
    /// the record as it was and is logged ([`note_whole_failed`]).
    fn with_whole_note(&self, record: &Record, whole: Option<&str>, ordinal: u64) -> Record {
        let mut record = record.clone();
        if let Some(whole) = whole
            .filter(|whole| record.kind == Kind::MemberFailure && whole.len() > record.note.len())
        {
            match self.append_note(ordinal, whole) {
                Ok(at) => record.note_at = Some(at),
                Err(why) => note_whole_failed(&self.notes_path(), &record, &why),
            }
        }
        record
    }

    /// The append itself, so the refusal has exactly one place to be noticed.
    ///
    /// # Errors
    ///
    /// As [`Self::append`], which is the only caller and adds the line.
    fn appended(&self, record: &Record, whole: Option<&str>) -> Result<(), String> {
        // THE WHOLE NOTE FIRST, while this journal's lock fixes the ordinal the
        // record will have. Only for a member failure whose note was cut.
        self.appended_images(|ordinal| {
            self.with_whole_note(record, whole, ordinal)
                .image()
                .to_vec()
        })
        .map(|_first| ())
    }

    /// Writes the records `build` makes, whole, at the end of the journal and
    /// makes them durable; answers the ordinal of the first.
    ///
    /// `build` is called with that ordinal while this journal's exclusive lock
    /// is held, so what it decides from it (a whole note's owner, whether a run
    /// must restate its opening record) cannot be overtaken by another writer.
    /// Its bytes are one `write_all`: a failed or short write is rolled back to
    /// the length measured under the lock, all of it.
    ///
    /// # Errors
    ///
    /// As [`Self::append`].
    fn appended_images(&self, build: impl FnOnce(u64) -> Vec<u8>) -> Result<u64, String> {
        let named =
            |what: &str, e: &std::io::Error| format!("{}: {what} — {e}", self.path.display());
        // ONE LEVEL, NEVER THE ROOT. `create_dir_all` made every missing
        // ancestor, the store root included, so a root that vanished (an
        // unmounted volume whose mountpoint path is still writable) was
        // recreated on the wrong device and this run's record began a second,
        // split journal there, the case `autopilot`'s "NEVER RECREATE A MISSING
        // STORE ROOT" refuses. Only `audit/` itself is made; a missing parent
        // is refused by name, nothing created. One `mkdir`. W1-api1-9, D-0954.
        if let Some(dir) = self.path.parent() {
            match std::fs::create_dir(dir) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists && dir.is_dir() => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    return Err(named(
                        "the store root is missing, so the audit directory was not created; a missing store root is never recreated, because an unmounted volume would split this append-only journal onto another device",
                        &e,
                    ));
                }
                Err(e) => return Err(named("cannot create the audit directory", &e)),
            }
        }
        // Every path below releases the append lock through the guard's
        // explicit unlock in its `Drop`, a refusal by returning and the
        // success path once the record is synced: closing the descriptor would
        // leave the lock alive on any duplicate a child spawned by another
        // thread still holds (D-0693).
        let mut file = Flock::try_lock(
            std::fs::OpenOptions::new()
                .read(true)
                .append(true)
                .create(true)
                .open(&self.path)
                .map_err(|e| named("cannot open the journal", &e))?,
            self.path.as_path(),
        )
        .map_err(|e| {
            format!(
                "{}: cannot take the journal append lock; another writer may be appending, so this record was refused rather than interleaved — {e}",
                self.path.display()
            )
        })?;
        let meta = file
            .metadata()
            .map_err(|e| named("cannot measure the locked journal", &e))?;
        let bytes = meta.len();
        let torn = bytes % RECORD_LEN_U64;
        if torn != 0 {
            return Err(format!(
                "{}: the locked journal has {torn} trailing byte(s) after {} whole record(s); refused to append because those bytes are an interrupted record and appending after them would permanently misalign every later fixed-stride record. Nothing was truncated or repaired because audit history is append-only",
                self.path.display(),
                bytes / RECORD_LEN_U64,
            ));
        }
        let first = bytes / RECORD_LEN_U64;
        let images = build(first);
        write_rolled_back(
            &mut file,
            &self.path,
            bytes,
            &images,
            std::io::Write::write_all,
        )?;
        file.sync_all()
            .map_err(|e| named("the record was written and not synced", &e))?;
        // THE RECORD IS DURABLE FROM HERE, SO NOTHING BELOW MAY REFUSE THE
        // APPEND. The lock is released by the guard's `Drop`, whose refused
        // unlock `store.flock` logs as a Warn naming the journal. Returned as
        // this append's error, it said a synced record was not in the journal.
        // See `append`'s doc. D-0695.
        //
        // THE INDEX FOLLOWS, under the same lock (WP-33c; DB-A32, DB-A34). It
        // returns nothing, so it cannot refuse the append either.
        self.index_after_sync(&meta, first, &images);
        drop(file);
        Ok(first)
    }

    /// Files the records just appended at `first` into the journal's index
    /// and moves its cover, under the append lock the caller holds; builds the
    /// index first when it is absent or not this file's (WP-33c).
    ///
    /// Returns nothing on purpose: an index that cannot be written is logged
    /// at `Error` ([`note_index`]) and never refuses the append. The record is
    /// the fact and the index is derived from it; every reader then walks the
    /// journal and says why.
    fn index_after_sync(&self, meta: &std::fs::Metadata, first: u64, images: &[u8]) {
        use std::os::unix::fs::MetadataExt as _;
        let total = first.saturating_add(as_u64(images.len()) / RECORD_LEN_U64);
        let indexed = self.index_locked((meta.dev(), meta.ino()), first, total, images);
        if indexed.as_ref().map_or(true, |built| built.from_scratch) {
            note_index(&self.path, &indexed);
        }
    }

    /// Appends one whole-note entry for journal record `ordinal` and makes it
    /// durable, answering the byte offset it starts at.
    ///
    /// The caller holds the journal's append lock; this takes the notes file's
    /// own. A failed or short write is truncated back to the length measured
    /// under that lock, as [`write_rolled_back`] does for a record, so a later
    /// entry's offset is never preceded by this call's partial bytes.
    ///
    /// # Errors
    ///
    /// The file cannot be opened, locked, measured, written or synced, in the
    /// failure's own words.
    fn append_note(&self, ordinal: u64, whole: &str) -> Result<u64, String> {
        let path = self.notes_path();
        let named = |what: &str, e: &std::io::Error| format!("{}: {what} — {e}", path.display());
        let mut file = Flock::try_lock(
            std::fs::OpenOptions::new()
                .read(true)
                .append(true)
                .create(true)
                .open(&path)
                .map_err(|e| named("cannot open the notes file", &e))?,
            path.as_path(),
        )
        .map_err(|e| {
            format!(
                "{}: cannot take the notes append lock — {e}",
                path.display()
            )
        })?;
        let at = file
            .metadata()
            .map_err(|e| named("cannot measure the locked notes file", &e))?
            .len();
        write_rolled_back(
            &mut file,
            &path,
            at,
            &note_image(ordinal, whole),
            std::io::Write::write_all,
        )?;
        file.sync_all()
            .map_err(|e| named("the note was written and not synced", &e))?;
        drop(file);
        Ok(at)
    }

    /// The whole reason of journal record `ordinal`, read at `at` in the notes
    /// file: one seek and two reads.
    ///
    /// # Errors
    ///
    /// The file cannot be read there, or what is there is not an entry, fails
    /// its checksum, or belongs to another record — each by name. Nothing is
    /// guessed: an offset that lands on the wrong reason is refused, never shown.
    pub fn whole_note(&self, ordinal: u64, at: u64) -> Result<String, String> {
        let path = self.notes_path();
        let mut file =
            std::fs::File::open(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        read_note(&mut file, &path, ordinal, at)
    }

    /// One page of records, newest first.
    ///
    /// `skip` and `take` are counted from the **newest** record, so page zero
    /// is the last `take` runs. Only those records are read: the offset is
    /// `(records - skip - take) * RECORD_LEN`, which is a multiplication, and
    /// the read is `take * RECORD_LEN` bytes however long the file is.
    /// `api::audit::the_tail_reads_only_the_records_it_shows` pins the offset
    /// and the length.
    ///
    /// # Errors
    ///
    /// The failure in its own words. A record that will not decode is **not**
    /// an error here — it is an [`Entry`] carrying its own fault, so one
    /// damaged record does not blank the page around it.
    ///
    /// # One arm here is a backstop and no test drives it
    ///
    /// The `seek` refusal. `SeekFrom::Start` takes a `u64` and every value of
    /// it is a legal offset on a regular file — seeking past the end is
    /// defined and succeeds — so on the platforms this builds for there is no
    /// input that reaches it. It is written rather than unwrapped because a
    /// `?` that cannot fire costs nothing and an `unwrap` that cannot fire is
    /// still an `unwrap`. It is named here rather than left for a coverage
    /// report to find, per `CLAUDE.md` §3 rule 6.
    pub fn page(&self, records: u64, skip: u64, take: u64) -> Result<Vec<Entry>, String> {
        // A REFUSED READ, ONCE PER REQUEST — not once per record. A record that
        // will not decode is an `Entry` carrying its own fault and is deliberately
        // silent here: a page is up to `MAX_PAGE_RECORDS` of them, and a line
        // each would make the volume a function of how damaged the file is, which
        // is exactly when the earlier lines matter most.
        //
        // This arm is the whole read failing — the journal gone between the look
        // and the seek, or shorter than the count says. The page then shows no
        // runs at all, which reads on screen like a server that has never pulled.
        self.read_page(records, skip, take)
            .inspect_err(|why| note_page_refused(&self.path, records, skip, take, why))
    }

    /// The read itself, so the refusal has exactly one place to be noticed.
    ///
    /// # Errors
    ///
    /// As [`Self::page`], which is the only caller and adds the line.
    fn read_page(&self, records: u64, skip: u64, take: u64) -> Result<Vec<Entry>, String> {
        let take = take.min(MAX_PAGE_RECORDS);
        let end = records.saturating_sub(skip);
        let start = end.saturating_sub(take);
        let wanted = end.saturating_sub(start);
        if wanted == 0 {
            return Ok(Vec::new());
        }
        // `wanted` is capped at MAX_PAGE_RECORDS above, so this product is at
        // most 51,200 and the conversion cannot fail on any host with a 16-bit
        // `usize` or wider. The arm is a backstop and no test drives it.
        let span = usize::try_from(wanted.saturating_mul(RECORD_LEN_U64))
            .map_err(|e| format!("{wanted} records do not fit this machine's memory: {e}"))?;
        let mut file =
            std::fs::File::open(&self.path).map_err(|e| format!("{}: {e}", self.path.display()))?;
        file.seek(std::io::SeekFrom::Start(
            start.saturating_mul(RECORD_LEN_U64),
        ))
        .map_err(|e| {
            format!(
                "{}: cannot seek to record {start} — {e}",
                self.path.display()
            )
        })?;
        let mut buf = vec![0u8; span];
        file.read_exact(&mut buf).map_err(|e| {
            format!(
                "{}: cannot read {wanted} record(s) — {e}",
                self.path.display()
            )
        })?;
        let mut out = Vec::with_capacity(span / RECORD_LEN);
        // THE NOTES FILE IS OPENED ONCE PER PAGE, and only when a record on it
        // names a whole note. Each such record then costs one seek and two
        // reads, so a page stays bounded by `MAX_PAGE_RECORDS` (D-3341).
        let mut notes: Option<Result<std::fs::File, String>> = None;
        for (i, chunk) in buf.chunks_exact(RECORD_LEN).enumerate() {
            let mut image = [0u8; RECORD_LEN];
            for (dst, src) in image.iter_mut().zip(chunk.iter()) {
                *dst = *src;
            }
            let ordinal = start.saturating_add(as_u64(i));
            let decoded = Record::decode(&image);
            let whole_note = decoded
                .as_ref()
                .ok()
                .and_then(|record| record.note_at)
                .map(|at| {
                    let path = self.notes_path();
                    match notes.get_or_insert_with(|| {
                        std::fs::File::open(&path).map_err(|e| format!("{}: {e}", path.display()))
                    }) {
                        Ok(file) => read_note(file, &path, ordinal, at),
                        Err(why) => Err(why.clone()),
                    }
                });
            out.push(Entry {
                ordinal,
                decoded,
                whole_note,
            });
        }
        out.reverse();
        Ok(out)
    }
}

/// Writes `image` at the end of a journal that held exactly `at` bytes under
/// the exclusive append lock, and on a failed or short write truncates the
/// file back to `at` before the lock is released (D-1854). Only bytes this
/// call wrote are removed; when the truncation also fails both errors are
/// named and the next append refuses the torn tail.
fn write_rolled_back(
    file: &mut std::fs::File,
    path: &Path,
    at: u64,
    image: &[u8],
    write: impl FnOnce(&mut std::fs::File, &[u8]) -> std::io::Result<()>,
) -> Result<(), String> {
    let Err(e) = write(file, image) else {
        return Ok(());
    };
    Err(match file.set_len(at) {
        Ok(()) => format!(
            "{}: cannot append the record — {e}; its partial bytes were truncated back to {at} bytes, so every whole record before it stays readable",
            path.display()
        ),
        Err(and) => format!(
            "{}: cannot append the record — {e}; truncating its partial bytes back to {at} bytes also failed: {and}; the journal may now end torn and the next append refuses it",
            path.display()
        ),
    })
}

/// A run that is **not on the record at all**, said somewhere that is not the
/// record.
///
/// # Why this is the loudest line in the file
///
/// Everything else this module reports is a run whose receipt exists and cannot
/// be believed. This is the one state where the receipt does not exist: the
/// pull ran, bars reached the store, and the journal — the surface whose entire
/// argument is that it survives a restart — took nothing. The refusal is
/// returned and painted on the answer page, and that page is a browser tab
/// somebody has to still be looking at. Once it is closed, a full disk or a
/// file where the `audit/` directory has to be leaves **no trace on the machine
/// that a run happened**, while the store quietly disagrees with a journal that
/// never heard about it.
///
/// `Error`, and it is the only `Error` here. `CLAUDE.md` §4 — degrade loudly.
///
/// Bounded by the append: one per run, plus one per failed member, and never by
/// rows. `source` and `note` are already cut to the record's own field widths,
/// so no field here grows with anything.
fn note_append_failed(path: &Path, record: &Record, why: &str) {
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::error("api.audit", "record not appended")
            .with("path", telemetry::Value::Str(&path.display().to_string()))
            .with("kind", telemetry::Value::Str(record.kind.label()))
            .with("scope", telemetry::Value::Str(record.scope.label()))
            .with("outcome", telemetry::Value::Str(record.outcome.label()))
            .with("source", telemetry::Value::Str(&record.source))
            .with("at_unix_secs", telemetry::Value::Int(record.at_unix_secs))
            .with("bars_stored", telemetry::Value::Uint(record.bars_stored))
            .with("why", telemetry::Value::Str(why)),
    );
}

/// A journal that cannot be measured, or one whose tail is half a record.
///
/// # What was invisible
///
/// Both states are already named on the page — [`Log::is_loud`] exists for
/// exactly that. But the page is where an operator goes *after* they suspect
/// something, and a tear is a thing nobody suspects: the whole records before it
/// still read, every row renders, and the only symptom is that the newest run is
/// not there. This puts the byte count somewhere it can be found without anyone
/// having gone looking, and it dates the tear to the first read after it
/// happened rather than to whenever the page is next opened.
///
/// `Warn`, not `Error`: nothing that was written is lost, and the surface
/// continued. [`note_append_failed`] is the error case, because that one is a
/// record that never reached the file.
///
/// **Silent on the ordinary states.** An absent journal is the state before the
/// first pull, and a whole file is the state the other 99.99% of the time —
/// neither emits, so this costs one branch per look and nothing else. Bounded by
/// the request, since a look is one `metadata` call on the render path.
fn note_looked(path: &Path, log: &Log) {
    match *log {
        // A tear is the loud half of a `Held`: the records before it are fine,
        // which is why it can sit there unnoticed.
        Log::Held {
            records,
            bytes,
            torn: Some(spare),
        } => {
            let _dropped_when_filtered = telemetry::emit(
                &telemetry::Event::warn("api.audit", "journal tail is torn")
                    .with("path", telemetry::Value::Str(&path.display().to_string()))
                    .with("records", telemetry::Value::Uint(records))
                    .with("bytes", telemetry::Value::Uint(bytes))
                    .with("torn_bytes", telemetry::Value::Uint(spare))
                    .with("record_len", telemetry::Value::Uint(RECORD_LEN_U64)),
            );
        }
        Log::Unreadable { ref reason } => {
            let _dropped_when_filtered = telemetry::emit(
                &telemetry::Event::warn("api.audit", "journal unreadable")
                    .with("path", telemetry::Value::Str(&path.display().to_string()))
                    .with("why", telemetry::Value::Str(reason)),
            );
        }
        // ABSENT IS NOT A FAULT and a whole file is not news. Emitting either
        // would put a line on every render of every audit page, which is the
        // fastest way to make the two above unfindable.
        Log::Absent | Log::Held { torn: None, .. } => {}
    }
}

/// A page of the journal that could not be read at all.
///
/// Distinct from a record that will not decode, which stays silent because a
/// page holds up to [`MAX_PAGE_RECORDS`] of them and each already renders its
/// own fault in its own row. This is the read itself refusing — the file gone
/// between the look and the seek, or shorter than the count claimed — and its
/// symptom on screen is an audit surface showing no runs, which is
/// indistinguishable from a server that has never pulled.
///
/// The three numbers are the ones needed to tell a vanished file from a
/// disagreement between the count and the length. `Warn`: the request was
/// answered and named its refusal; nothing written was lost. One per refused
/// request.
fn note_page_refused(path: &Path, records: u64, skip: u64, take: u64, why: &str) {
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::warn("api.audit", "journal page refused")
            .with("path", telemetry::Value::Str(&path.display().to_string()))
            .with("records", telemetry::Value::Uint(records))
            .with("skip", telemetry::Value::Uint(skip))
            .with("take", telemetry::Value::Uint(take))
            .with("why", telemetry::Value::Str(why)),
    );
}

/// The most records one call will ever read.
///
/// A page shows what a person reads; this is the ceiling on the buffer
/// [`Journal::page`] allocates, so a caller that asks for the whole file gets
/// one page of it instead of an allocation the size of the disk.
/// `docs/07-o1-architecture.md` law 5 — bound every input at the boundary.
pub const MAX_PAGE_RECORDS: u64 = 200;

/// One row of the journal page: which record it is, and what it says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// Its position in the file, counted from the first record ever written.
    pub ordinal: u64,
    /// The record, or why it could not be believed.
    pub decoded: Result<Record, RecordFault>,
    /// The record's whole reason from the notes file, when it names one, or
    /// why it could not be read. [`None`] for a record that names none
    /// ([`Record::note_at`]). D-3341, DB-G23.
    pub whole_note: Option<Result<String, String>>,
}

/// One whole-note entry's bytes: the head, the reason (cut on a character
/// boundary past [`MAX_WHOLE_NOTE`]) and its checksum.
fn note_image(ordinal: u64, whole: &str) -> Vec<u8> {
    let kept = keep(whole, MAX_WHOLE_NOTE);
    let mut out = Vec::with_capacity(NOTE_HEAD.saturating_add(kept.len()).saturating_add(4));
    out.extend_from_slice(&NOTE_MAGIC);
    out.extend_from_slice(&NOTE_ENTRY_VERSION.to_le_bytes());
    out.extend_from_slice(&[0, 0]);
    out.extend_from_slice(&ordinal.to_le_bytes());
    out.extend_from_slice(&u32::try_from(whole.len()).unwrap_or(u32::MAX).to_le_bytes());
    out.extend_from_slice(&u32::try_from(kept.len()).unwrap_or(u32::MAX).to_le_bytes());
    out.extend_from_slice(kept.as_bytes());
    let crc = crc32c(&out);
    out.extend_from_slice(&crc.to_le_bytes());
    out
}

/// The entry at `at` in an open notes file, checked to belong to `ordinal`.
///
/// A reason the entry had to cut past [`MAX_WHOLE_NOTE`] comes back with the
/// cut said in words, never passed off as whole.
fn read_note(
    file: &mut std::fs::File,
    path: &Path,
    ordinal: u64,
    at: u64,
) -> Result<String, String> {
    let named = |what: &str| format!("{}: the note at byte {at} {what}", path.display());
    file.seek(std::io::SeekFrom::Start(at))
        .map_err(|e| named(&format!("cannot be reached — {e}")))?;
    let mut head = [0u8; NOTE_HEAD];
    file.read_exact(&mut head)
        .map_err(|e| named(&format!("cannot be read — {e}")))?;
    let field = |from: usize, to: usize| head.get(from..to).unwrap_or(&[]);
    if field(0, 4) != NOTE_MAGIC {
        return Err(named("is not a note entry"));
    }
    let version = <[u8; 2]>::try_from(field(4, 6)).map_or(0, u16::from_le_bytes);
    if version != NOTE_ENTRY_VERSION {
        return Err(named(&format!(
            "has entry layout {version}; this build reads {NOTE_ENTRY_VERSION}"
        )));
    }
    let owner = <[u8; 8]>::try_from(field(8, 16)).map_or(u64::MAX, u64::from_le_bytes);
    if owner != ordinal {
        return Err(named(&format!(
            "belongs to record {owner}, not record {ordinal}, so it is not shown"
        )));
    }
    let whole = <[u8; 4]>::try_from(field(16, 20)).map_or(0, u32::from_le_bytes);
    let kept = <[u8; 4]>::try_from(field(20, 24)).map_or(u32::MAX, u32::from_le_bytes);
    let kept = usize::try_from(kept)
        .ok()
        .filter(|kept| *kept <= MAX_WHOLE_NOTE)
        .ok_or_else(|| {
            named(&format!(
                "claims {kept} bytes, past the {MAX_WHOLE_NOTE} kept"
            ))
        })?;
    let mut rest = vec![0u8; kept.saturating_add(4)];
    file.read_exact(&mut rest)
        .map_err(|e| named(&format!("ends early — {e}")))?;
    let (text, stored) = rest.split_at(kept);
    let stored = <[u8; 4]>::try_from(stored).map_or(0, u32::from_le_bytes);
    let mut covered = head.to_vec();
    covered.extend_from_slice(text);
    let computed = crc32c(&covered);
    if stored != computed {
        return Err(named(&format!(
            "fails its checksum ({stored:#010x} != {computed:#010x})"
        )));
    }
    let text = std::str::from_utf8(text).map_err(|_ignored| named("is not UTF-8"))?;
    if u64::from(whole) > as_u64(kept) {
        return Ok(format!("{text}… (cut from {whole} bytes)"));
    }
    Ok(text.to_owned())
}

/// A whole note that could not be written, said where it survives: its
/// record still lands, cut, and this line names why the rest did not.
fn note_whole_failed(path: &Path, record: &Record, why: &str) {
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::error("api.audit", "whole note not appended")
            .with("path", telemetry::Value::Str(&path.display().to_string()))
            .with("instrument", telemetry::Value::Str(&record.source))
            .with(
                "note_bytes",
                telemetry::Value::Uint(u64::from(record.note_bytes)),
            )
            .with("why", telemetry::Value::Str(why)),
    );
}

// ---------------------------------------------------------------------------
// the run's ask (DB-B34, repair 6; D-4950)
// ---------------------------------------------------------------------------

/// What an opening record's note carries before its ask: `"; ask "`, then
/// sixteen lowercase hexadecimal digits ([`ask_note`]).
pub const ASK_MARK: &str = "; ask ";

/// The digest of what a run asks the vendor for, from `parts` in order:
/// 64-bit FNV-1a (offset basis `0xcbf29ce484222325`, prime `0x100000001b3`)
/// over each part's bytes followed by the unit separator `0x1F`, which no
/// part carries, so two different lists of parts are two different inputs.
///
/// # Why a digest and not the words
///
/// The note is [`NOTE_CAPACITY`] bytes, and the instruments a run names can
/// be hundreds of symbols: the words do not fit, and a cut list would make
/// two different asks read as one. The digest is compared, never read: equal
/// digests are taken as the same ask (two different asks share one with a
/// chance of about one in 2^64), and a digest that differs, including one a
/// later build computes over parts it spells differently, only ever keeps a
/// run cut short reported (`crate::pullrun`), the safe direction.
///
/// # Cost
///
/// One multiply and one xor per byte of the parts: bounded by the run's own
/// request, computed once when the run opens.
#[must_use]
pub fn ask_digest(parts: &[&str]) -> u64 {
    const BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    parts.iter().fold(BASIS, |hash, part| {
        part.bytes()
            .chain(core::iter::once(0x1f_u8))
            .fold(hash, |hash, byte| {
                (hash ^ u64::from(byte)).wrapping_mul(PRIME)
            })
    })
}

/// An opening record's note: `lead`, then [`ASK_MARK`] and `ask` as sixteen
/// lowercase hexadecimal digits.
#[must_use]
pub fn ask_note(lead: &str, ask: u64) -> String {
    format!("{lead}{ASK_MARK}{ask:016x}")
}

/// The ask an opening record's note names: the sixteen hexadecimal digits
/// after [`ASK_MARK`], and no seventeenth. `None` for a note that names none,
/// which is every opening record written before D-4950 and every restatement
/// of one ("continues the run opened at record N").
#[must_use]
pub fn ask_of(note: &str) -> Option<u64> {
    let (_, rest) = note.split_once(ASK_MARK)?;
    let digits = rest.get(..16)?;
    let whole = digits.bytes().all(|byte| byte.is_ascii_hexdigit())
        && !rest.as_bytes().get(16).is_some_and(u8::is_ascii_hexdigit);
    whole
        .then(|| u64::from_str_radix(digits, 16).ok())
        .flatten()
}

// ---------------------------------------------------------------------------
// bytes
// ---------------------------------------------------------------------------

/// `n` as a `u64`, saturating rather than wrapping.
///
/// A count that wrapped would report a smaller number than the truth, which is
/// the one direction a count must never be wrong in. On every host this builds
/// for `usize` is 64 bits and the conversion cannot fail, so the arm is a
/// backstop and not a live path.
pub(crate) fn as_u64(n: usize) -> u64 {
    u64::try_from(n).unwrap_or(u64::MAX)
}

/// A text's byte length, saturating at what the record's field can hold.
fn saturating_len(text: &str) -> u16 {
    u16::try_from(text.len()).unwrap_or(u16::MAX)
}

/// The longest prefix of `text` that fits `capacity` bytes and is still text.
///
/// Cut on a character boundary, never mid-sequence: a record holding half a
/// multi-byte character is a record that will not decode, and a page that
/// refuses its own writer's output is worse than a shorter line.
fn keep(text: &str, capacity: usize) -> String {
    if text.len() <= capacity {
        return text.to_owned();
    }
    let mut end = 0;
    for (at, c) in text.char_indices() {
        let next = at.saturating_add(c.len_utf8());
        if next > capacity {
            break;
        }
        end = next;
    }
    text.get(..end).unwrap_or("").to_owned()
}

/// A count as the `u32` a version-2 field holds, saturating.
fn narrow(n: u64) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// A kept text's length as the byte the record stores.
///
/// [`keep`] never returns more than a capacity, and both capacities are under
/// 255, so the saturation is a backstop. It is written rather than asserted
/// because an assertion here would be a panic on a path that renders a page.
fn kept_byte(text: &str) -> u8 {
    u8::try_from(text.len()).unwrap_or(u8::MAX)
}

/// The text at `offset`, `kept` bytes of it.
fn text(
    image: &[u8; RECORD_LEN],
    offset: usize,
    kept: u8,
    capacity: usize,
) -> Result<String, RecordFault> {
    let kept_usize = usize::from(kept);
    if kept_usize > capacity {
        return Err(RecordFault::KeptPastCapacity {
            kept,
            capacity: u8::try_from(capacity).unwrap_or(u8::MAX),
        });
    }
    let end = offset.saturating_add(kept_usize);
    let bytes = image.get(offset..end).unwrap_or(&[]);
    std::str::from_utf8(bytes)
        .map(str::to_owned)
        .map_err(|_ignored| RecordFault::NotText)
}

/// Writes `src` at `offset`.
fn write_at<const N: usize>(out: &mut [u8; RECORD_LEN], offset: usize, src: [u8; N]) {
    for (dst, byte) in out.iter_mut().skip(offset).zip(src) {
        *dst = byte;
    }
}

/// Writes `text` at `offset`, leaving the rest of its field zero.
fn write_text(out: &mut [u8; RECORD_LEN], offset: usize, text: &str, capacity: usize) {
    for (dst, byte) in out.iter_mut().skip(offset).take(capacity).zip(text.bytes()) {
        *dst = byte;
    }
}

/// One byte at `offset`, or zero past the end.
fn byte(image: &[u8; RECORD_LEN], offset: usize) -> u8 {
    image.get(offset).copied().unwrap_or(0)
}

/// Two bytes at `offset`.
fn le2(image: &[u8; RECORD_LEN], offset: usize) -> [u8; 2] {
    let mut out = [0u8; 2];
    for (dst, src) in out.iter_mut().zip(image.iter().skip(offset)) {
        *dst = *src;
    }
    out
}

/// Four bytes at `offset`.
fn le4(image: &[u8; RECORD_LEN], offset: usize) -> [u8; 4] {
    let mut out = [0u8; 4];
    for (dst, src) in out.iter_mut().zip(image.iter().skip(offset)) {
        *dst = *src;
    }
    out
}

/// Eight bytes at `offset`.
fn le8(image: &[u8; RECORD_LEN], offset: usize) -> [u8; 8] {
    let mut out = [0u8; 8];
    for (dst, src) in out.iter_mut().zip(image.iter().skip(offset)) {
        *dst = *src;
    }
    out
}

/// Every byte the checksum covers: `0..252`, one contiguous run.
///
/// **The domain is frozen.** Covering the four checksum bytes as well would be
/// a different number over a different domain, and every record already on disk
/// would fail its check — a change no round-trip test would notice, because a
/// checksum that agrees with itself always agrees with itself.
/// `api::audit::the_covered_domain_is_the_record_minus_its_checksum` pins it.
fn covered(image: &[u8; RECORD_LEN]) -> [u8; OFF_CRC] {
    let mut head = [0u8; OFF_CRC];
    for (dst, src) in head.iter_mut().zip(image.iter()) {
        *dst = *src;
    }
    head
}

// ===========================================================================
// THE JOURNAL'S INDEX (WP-33c; DB-A32, DB-A34)
// ===========================================================================
//
// # Why
//
// The journal keeps no index of its own, so the two readers that need to know
// which run a record belongs to (`/audit.json`'s feed scope) or which records
// refused one instrument (`/dayflags.json`'s Zerodha front) decoded every
// record the first time a process asked: 4,217,027 records, 1,079,558,912
// bytes on the operator's store on 9 Oct 2026, 1.33 s under a mutex on the
// first `/audit.json` request and the same walk again under another on the
// first `/dayflags.json`. Neither cost was a constant, and both grew with
// every pull.
//
// # What it is
//
// Beside the journal, `pull.journal.idx/`, written by the journal's own
// append under the journal's exclusive lock, AFTER the record is synced:
//
// * `runs` -- one 32-byte entry per run record: its ordinal, its stamp and
//   the vendor its source names (`crate::audit_json::vendor_of_source`).
// * `refusals/<source>.idx` -- one 16-byte entry per member failure whose note
//   may name a refusal ([`may_name_a_refusal`]), filed by its source (the
//   store symbol it pulled).
// * `faults` -- one 16-byte entry per record that would not decode.
// * `cover` -- how many of the journal's records all of the above cover: one
//   16-byte image, CRC-checked, rewritten in place after every append (a
//   counter, like a manifest's, not history). Each entry file is synced before
//   the cover moves past it, so a cover can never claim an entry the disk
//   does not hold; a cover lost to a crash is behind, never ahead, and the
//   next append re-indexes the gap. An entry written twice that way is the
//   same entry; readers keep one per ordinal.
// * `ident` -- the device and inode of the journal the index was built from:
//   an index of another file (a journal replaced) is refused, never read.
//
// # What it costs
//
// Per append: the record's kind byte and a decode of the records just
// written (one or two), at most one synced entry append, and one 16-byte
// write of the cover. Constant. A reader: one read of `ident` and `cover`,
// one read of `runs` (O(runs): 12 on the operator's store), one read of one
// `refusals` file (O(refusals of one instrument): 20 for NIFTY), and a decode
// of the records the cover does not reach yet (none, after an append).
//
// # Built once for a journal that predates it
//
// A journal written before this index exists has none. The first append under
// this build builds it ([`Journal::build_index`], under the lock it already
// holds), and so does `api serve` at startup, in the background, when the
// lock is free: one pass over the whole journal, the 1.33 s the readers used
// to pay per process, paid once, ever. Until then a reader says it walked the
// journal and why (`index: absent`); it never guesses.

/// What the index directory is called beside a journal: the journal's file
/// name and `.idx`.
pub const INDEX_SUFFIX: &str = "idx";

/// One `runs` entry.
const RUN_ENTRY: usize = 32;
/// One `refusals` or `faults` entry, and the cover.
const ORDINAL_ENTRY: usize = 16;
/// The marks closing each entry kind.
const MARK_RUN: [u8; 4] = *b"BXRI";
const MARK_REFUSAL: [u8; 4] = *b"BXRF";
const MARK_FAULT: [u8; 4] = *b"BXFT";
const MARK_COVER: [u8; 4] = *b"BXCV";
/// The rule a `runs` entry's vendor was attributed by. An entry of another
/// rule is re-attributed from its record, never trusted.
const ATTRIBUTION_RULE: u8 = 1;
/// How many records a backfill decodes per read.
const INDEX_CHUNK_RECORDS: u64 = 16_384;

/// The notes a member failure carries when its whole window was refused
/// before append. Moved here from `dayflags` so the index and its reader name
/// one list.
pub(crate) const WHOLE_WINDOW_NOTES: [&str; 2] = [
    "conflicting vendor candles at ",
    "off-grid broker candle at ",
];

/// Whether a member failure's note may name a refusal: it says `refused`, or
/// it opens with a whole-window refusal. A superset of what `/dayflags.json`
/// keeps (it then reads the note for the day); never a subset.
pub(crate) fn may_name_a_refusal(note: &str) -> bool {
    note.contains("refused") || WHOLE_WINDOW_NOTES.iter().any(|p| note.starts_with(p))
}

/// One record read by its ordinal: the ordinal, and the record or why it
/// would not decode.
pub(crate) type Decoded = (u64, Result<Record, RecordFault>);

/// What the index says about a journal's first `covered` records.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Indexed {
    /// Records the index covers, from the first.
    pub(crate) covered: u64,
    /// Every run record among them, `(ordinal, vendor)`, in file order.
    pub(crate) runs: Vec<(u64, Option<brutex_core::vendor::Vendor>)>,
    /// Records among them that would not decode.
    pub(crate) faults: u64,
    /// Which records those are, ascending, one per ordinal: a reader that
    /// walks the run records must know where a record it was never handed
    /// lies (DB-B34: a run record damaged before the index was built is
    /// filed here and not under `runs`).
    pub(crate) fault_ordinals: Vec<u64>,
    /// Index entries that would not decode, skipped.
    pub(crate) malformed: u64,
}

impl Indexed {
    /// The vendor of the run a record at `ordinal` belongs to: the run record
    /// before it. O(log runs).
    pub(crate) fn vendor_before(&self, ordinal: u64) -> Option<brutex_core::vendor::Vendor> {
        let at = self.runs.partition_point(|(run, _)| *run < ordinal);
        at.checked_sub(1)
            .and_then(|at| self.runs.get(at))
            .and_then(|(_, vendor)| *vendor)
    }
}

/// What one index build or catch-up did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IndexBuilt {
    /// Records decoded.
    pub records: u64,
    /// Entries written, by kind.
    pub runs: u64,
    /// See [`Self::runs`].
    pub refusals: u64,
    /// See [`Self::runs`].
    pub faults: u64,
    /// The index was built from the first record (it was absent, or of another
    /// file, or its cover was unreadable).
    pub from_scratch: bool,
    /// Why it was built from scratch, when it was.
    pub why: Option<String>,
    /// The cover written.
    pub covered: u64,
    /// The lock was held by a writer, so nothing was done: that writer builds
    /// it under the lock it holds.
    pub busy: bool,
    /// Wall time, microseconds.
    pub micros: u64,
}

/// One `runs` entry's image.
fn run_entry(
    ordinal: u64,
    at: i64,
    vendor: Option<brutex_core::vendor::Vendor>,
) -> [u8; RUN_ENTRY] {
    let mut out = [0u8; RUN_ENTRY];
    let code = vendor
        .and_then(|v| {
            brutex_core::vendor::Vendor::ALL
                .iter()
                .position(|held| *held == v)
        })
        .and_then(|n| u8::try_from(n.saturating_add(1)).ok())
        .unwrap_or(0);
    let parts: [&[u8]; 4] = [
        &ordinal.to_le_bytes(),
        &at.to_le_bytes(),
        &[code, ATTRIBUTION_RULE, 0, 0, 0, 0, 0, 0],
        &MARK_RUN,
    ];
    let mut at_byte = 0usize;
    for part in parts {
        for (dst, src) in out.iter_mut().skip(at_byte).zip(part) {
            *dst = *src;
        }
        at_byte = at_byte.saturating_add(part.len());
    }
    let crc = crc32c(out.get(..RUN_ENTRY - 4).unwrap_or(&[]));
    for (dst, src) in out.iter_mut().skip(RUN_ENTRY - 4).zip(crc.to_le_bytes()) {
        *dst = src;
    }
    out
}

/// A `runs` entry: `(ordinal, vendor code, rule)`, or `None` when it is not one.
fn read_run_entry(bytes: &[u8]) -> Option<(u64, u8, u8)> {
    if bytes.len() != RUN_ENTRY || bytes.get(24..28) != Some(MARK_RUN.as_slice()) {
        return None;
    }
    let crc = u32::from_le_bytes(<[u8; 4]>::try_from(bytes.get(28..32)?).ok()?);
    if crc != crc32c(bytes.get(..28)?) {
        return None;
    }
    let ordinal = u64::from_le_bytes(<[u8; 8]>::try_from(bytes.get(..8)?).ok()?);
    Some((ordinal, *bytes.get(16)?, *bytes.get(17)?))
}

/// The vendor a code names: `Ok(None)` for code 0 (no vendor), `Err` with the
/// code for one this build does not know.
fn vendor_of_code(code: u8) -> Result<Option<brutex_core::vendor::Vendor>, u8> {
    if code == 0 {
        return Ok(None);
    }
    brutex_core::vendor::Vendor::ALL
        .get(usize::from(code.saturating_sub(1)))
        .copied()
        .map(Some)
        .ok_or(code)
}

/// An ordinal entry's image, closed by `mark`.
fn ordinal_entry(ordinal: u64, mark: [u8; 4]) -> [u8; ORDINAL_ENTRY] {
    let mut out = [0u8; ORDINAL_ENTRY];
    for (dst, src) in out
        .iter_mut()
        .zip(ordinal.to_le_bytes().iter().chain(&mark))
    {
        *dst = *src;
    }
    let crc = crc32c(out.get(..12).unwrap_or(&[]));
    for (dst, src) in out.iter_mut().skip(12).zip(crc.to_le_bytes()) {
        *dst = src;
    }
    out
}

/// An ordinal entry closed by `mark`, or `None` when it is not one.
fn read_ordinal_entry(bytes: &[u8], mark: [u8; 4]) -> Option<u64> {
    if bytes.len() != ORDINAL_ENTRY || bytes.get(8..12) != Some(mark.as_slice()) {
        return None;
    }
    let crc = u32::from_le_bytes(<[u8; 4]>::try_from(bytes.get(12..16)?).ok()?);
    if crc != crc32c(bytes.get(..12)?) {
        return None;
    }
    Some(u64::from_le_bytes(
        <[u8; 8]>::try_from(bytes.get(..8)?).ok()?,
    ))
}

/// A source as a file name: every byte outside `A-Z a-z 0-9 - _` as `%XX`, so
/// no source names a path outside `refusals/`. `None` when empty.
fn source_file(source: &str) -> Option<String> {
    if source.is_empty() {
        return None;
    }
    let mut name = String::with_capacity(source.len().saturating_add(4));
    for byte in source.bytes() {
        if byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_' {
            name.push(char::from(byte));
        } else {
            name.push('%');
            for nibble in [byte >> 4, byte & 0x0f] {
                name.push(char::from(
                    b"0123456789ABCDEF"
                        .get(usize::from(nibble))
                        .copied()
                        .unwrap_or(b'0'),
                ));
            }
        }
    }
    name.push_str(".idx");
    Some(name)
}

/// The entries one batch of records adds, gathered so each file is written
/// once per batch.
#[derive(Debug, Default)]
struct Batch {
    runs: Vec<u8>,
    faults: Vec<u8>,
    refusals: std::collections::BTreeMap<String, Vec<u8>>,
    built: IndexBuilt,
}

impl Batch {
    /// Files one record image of ordinal `ordinal`.
    fn file(&mut self, ordinal: u64, image: &[u8; RECORD_LEN]) {
        self.built.records = self.built.records.saturating_add(1);
        match Record::decode(image) {
            Err(_) => {
                self.faults
                    .extend_from_slice(&ordinal_entry(ordinal, MARK_FAULT));
                self.built.faults = self.built.faults.saturating_add(1);
            }
            Ok(record) if record.kind == Kind::Run => {
                let vendor = crate::audit_json::vendor_of_source(&record.source);
                self.runs
                    .extend_from_slice(&run_entry(ordinal, record.at_unix_secs, vendor));
                self.built.runs = self.built.runs.saturating_add(1);
            }
            Ok(record) => {
                if may_name_a_refusal(&record.note)
                    && let Some(name) = source_file(&record.source)
                {
                    self.refusals
                        .entry(name)
                        .or_default()
                        .extend_from_slice(&ordinal_entry(ordinal, MARK_REFUSAL));
                    self.built.refusals = self.built.refusals.saturating_add(1);
                }
            }
        }
    }

    /// Writes every entry gathered into the index at `dir`, each file synced.
    fn write(&mut self, dir: &Path) -> Result<(), String> {
        append_synced(&dir.join("runs"), &self.runs)?;
        append_synced(&dir.join("faults"), &self.faults)?;
        for (name, bytes) in &self.refusals {
            append_synced(&dir.join("refusals").join(name), bytes)?;
        }
        self.runs.clear();
        self.faults.clear();
        self.refusals.clear();
        Ok(())
    }
}

/// Appends `bytes` to `path` and syncs it; nothing for nothing.
fn append_synced(path: &Path, bytes: &[u8]) -> Result<(), String> {
    use std::io::Write as _;
    if bytes.is_empty() {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("{}: cannot create — {e}", parent.display()))?;
    }
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(path)
        .map_err(|e| format!("{}: cannot open — {e}", path.display()))?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|e| format!("{}: cannot append an index entry — {e}", path.display()))
}

/// Writes the cover: `covered` records indexed. In place, one 16-byte write.
fn write_cover(dir: &Path, covered: u64) -> Result<(), String> {
    use std::os::unix::fs::FileExt as _;
    let path = dir.join("cover");
    let image = ordinal_entry(covered, MARK_COVER);
    std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(&path)
        .and_then(|file| file.write_all_at(&image, 0))
        .map_err(|e| format!("{}: cannot write the cover — {e}", path.display()))
}

/// The device and inode of the journal, as `ident` holds them.
fn ident_text(identity: (u64, u64)) -> String {
    format!("{} {}\n", identity.0, identity.1)
}

impl Journal {
    /// The journal's index directory: `pull.journal.idx` beside it.
    #[must_use]
    pub fn index_dir(&self) -> PathBuf {
        let name = self
            .path
            .file_name()
            .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
        self.path.with_file_name(format!("{name}.{INDEX_SUFFIX}"))
    }

    /// How many records the index at `dir` covers, when it is an index of
    /// the journal of `identity`.
    ///
    /// # Errors
    ///
    /// Absent, of another file, or a cover that will not read, by name.
    fn cover_of(dir: &Path, identity: (u64, u64)) -> Result<u64, String> {
        let ident = dir.join("ident");
        let held = match std::fs::read_to_string(&ident) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(format!("no index at {}", dir.display()));
            }
            Err(e) => return Err(format!("{}: {e}", ident.display())),
        };
        if held != ident_text(identity) {
            return Err(format!(
                "{} is the index of another file (device and inode {:?}, the journal's are \
                 {identity:?}): a journal replaced, never read through a stale index",
                dir.display(),
                held.trim()
            ));
        }
        let cover = dir.join("cover");
        let bytes = std::fs::read(&cover).map_err(|e| format!("{}: {e}", cover.display()))?;
        bytes
            .get(..ORDINAL_ENTRY)
            .and_then(|head| read_ordinal_entry(head, MARK_COVER))
            .ok_or_else(|| format!("{}: not a cover this build wrote", cover.display()))
    }

    /// The journal file's identity and whole records now.
    fn measured(&self) -> Result<((u64, u64), u64), String> {
        use std::os::unix::fs::MetadataExt as _;
        let meta =
            std::fs::metadata(&self.path).map_err(|e| format!("{}: {e}", self.path.display()))?;
        Ok(((meta.dev(), meta.ino()), meta.len() / RECORD_LEN_U64))
    }

    /// What the index says about the journal's first `total` records, read
    /// without a lock: `ident`, `cover`, `runs` and `faults`. See the section
    /// header for the cost.
    ///
    /// # Errors
    ///
    /// No index, an index of another file, a cover past `total` (a journal
    /// shorter than its index), or a file that is there and cannot be read,
    /// each by name. A reader then walks the journal and says so.
    pub(crate) fn read_index(&self, total: u64) -> Result<Indexed, String> {
        let (identity, _) = self.measured()?;
        let dir = self.index_dir();
        let covered = Self::cover_of(&dir, identity)?;
        if covered > total {
            return Err(format!(
                "{} covers {covered} records and the journal holds {total}: an index ahead of \
                 its journal is refused",
                dir.display()
            ));
        }
        let mut out = Indexed {
            covered,
            ..Indexed::default()
        };
        let runs = read_or_empty(&dir.join("runs"))?;
        for chunk in runs.chunks_exact(RUN_ENTRY) {
            match read_run_entry(chunk) {
                Some((ordinal, _, _)) if ordinal >= covered => {}
                Some((ordinal, code, rule)) => {
                    let vendor = if rule == ATTRIBUTION_RULE {
                        vendor_of_code(code)
                    } else {
                        Err(code)
                    };
                    match vendor {
                        Ok(vendor) => out.runs.push((ordinal, vendor)),
                        Err(_) => match self.records_at(&[ordinal])?.pop() {
                            Some((_, Ok(record))) => out.runs.push((
                                ordinal,
                                crate::audit_json::vendor_of_source(&record.source),
                            )),
                            _ => out.malformed = out.malformed.saturating_add(1),
                        },
                    }
                }
                None => out.malformed = out.malformed.saturating_add(1),
            }
        }
        // ONE PER ORDINAL: an entry a crash made the next append re-write is
        // the same entry.
        out.runs.sort_by_key(|run| run.0);
        out.runs.dedup_by_key(|run| run.0);
        let faults = read_or_empty(&dir.join("faults"))?;
        let mut fault_ordinals: Vec<u64> = faults
            .chunks_exact(ORDINAL_ENTRY)
            .filter_map(|chunk| read_ordinal_entry(chunk, MARK_FAULT))
            .filter(|ordinal| *ordinal < covered)
            .collect();
        fault_ordinals.sort_unstable();
        fault_ordinals.dedup();
        out.faults = as_u64(fault_ordinals.len());
        out.fault_ordinals = fault_ordinals;
        Ok(out)
    }

    /// The ordinals the index files under `source` below `covered`: every
    /// member failure of that source whose note may name a refusal. One read
    /// of one file, O(its entries).
    ///
    /// # Errors
    ///
    /// The file is there and cannot be read.
    pub(crate) fn refusal_ordinals(&self, source: &str, covered: u64) -> Result<Vec<u64>, String> {
        let Some(name) = source_file(source) else {
            return Ok(Vec::new());
        };
        let bytes = read_or_empty(&self.index_dir().join("refusals").join(name))?;
        let mut out: Vec<u64> = bytes
            .chunks_exact(ORDINAL_ENTRY)
            .filter_map(|chunk| read_ordinal_entry(chunk, MARK_REFUSAL))
            .filter(|ordinal| *ordinal < covered)
            .collect();
        out.sort_unstable();
        out.dedup();
        Ok(out)
    }

    /// The records at `ordinals`, each with its ordinal: one open and one
    /// positional read per record, whatever the journal's length.
    ///
    /// # Errors
    ///
    /// The journal cannot be opened, or a record cannot be read whole.
    pub(crate) fn records_at(&self, ordinals: &[u64]) -> Result<Vec<Decoded>, String> {
        use std::os::unix::fs::FileExt as _;
        if ordinals.is_empty() {
            return Ok(Vec::new());
        }
        let file =
            std::fs::File::open(&self.path).map_err(|e| format!("{}: {e}", self.path.display()))?;
        let mut out = Vec::with_capacity(ordinals.len());
        let mut image = [0u8; RECORD_LEN];
        for &ordinal in ordinals {
            file.read_exact_at(&mut image, ordinal.saturating_mul(RECORD_LEN_U64))
                .map_err(|e| {
                    format!(
                        "{}: cannot read record {ordinal} — {e}",
                        self.path.display()
                    )
                })?;
            out.push((ordinal, Record::decode(&image)));
        }
        Ok(out)
    }

    /// Builds or catches up the index, taking the journal's append lock for
    /// the time it takes. A lock another writer holds is no failure: that
    /// writer indexes under its own hold ([`IndexBuilt::busy`]).
    ///
    /// # Errors
    ///
    /// The journal or the index cannot be read or written, by name.
    pub fn build_index(&self) -> Result<IndexBuilt, String> {
        use std::os::unix::fs::MetadataExt as _;
        let file = match std::fs::OpenOptions::new().read(true).open(&self.path) {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(IndexBuilt::default());
            }
            Err(e) => return Err(format!("{}: {e}", self.path.display())),
        };
        let Ok(locked) = Flock::try_lock(file, self.path.as_path()) else {
            return Ok(IndexBuilt {
                busy: true,
                ..IndexBuilt::default()
            });
        };
        let meta = locked
            .metadata()
            .map_err(|e| format!("{}: {e}", self.path.display()))?;
        let total = meta.len() / RECORD_LEN_U64;
        let built = self.index_locked((meta.dev(), meta.ino()), total, total, &[])?;
        drop(locked);
        Ok(built)
    }

    /// The index brought to `first + images` records, under the journal's
    /// lock: rebuilt from scratch when absent or not this file's, caught up
    /// from the journal when behind `first`, then `images` (the records just
    /// appended at `first`) filed, then the cover moved.
    fn index_locked(
        &self,
        identity: (u64, u64),
        first: u64,
        total: u64,
        images: &[u8],
    ) -> Result<IndexBuilt, String> {
        let started = std::time::Instant::now();
        let dir = self.index_dir();
        let mut built = IndexBuilt::default();
        let covered = match Self::cover_of(&dir, identity) {
            Ok(covered) if covered <= first => covered,
            Ok(covered) => {
                let why =
                    format!("the index covered {covered} records and the journal held {first}");
                built = self.rebuild(identity, first, &why)?;
                first
            }
            Err(why) => {
                built = self.rebuild(identity, first, &why)?;
                first
            }
        };
        let mut batch = Batch::default();
        if covered < first {
            self.file_range(covered, first, &mut batch, &dir)?;
        }
        for (n, chunk) in images.chunks_exact(RECORD_LEN).enumerate() {
            let mut image = [0u8; RECORD_LEN];
            for (dst, src) in image.iter_mut().zip(chunk) {
                *dst = *src;
            }
            batch.file(first.saturating_add(as_u64(n)), &image);
        }
        batch.write(&dir)?;
        write_cover(&dir, total)?;
        built.records = built.records.saturating_add(batch.built.records);
        built.runs = built.runs.saturating_add(batch.built.runs);
        built.refusals = built.refusals.saturating_add(batch.built.refusals);
        built.faults = built.faults.saturating_add(batch.built.faults);
        built.covered = total;
        built.micros = u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX);
        Ok(built)
    }

    /// Files the journal's records `[from, to)` into `batch`, read in
    /// [`INDEX_CHUNK_RECORDS`] reads and written to `dir` after each.
    fn file_range(&self, from: u64, to: u64, batch: &mut Batch, dir: &Path) -> Result<(), String> {
        let mut file =
            std::fs::File::open(&self.path).map_err(|e| format!("{}: {e}", self.path.display()))?;
        file.seek(std::io::SeekFrom::Start(
            from.saturating_mul(RECORD_LEN_U64),
        ))
        .map_err(|e| format!("{}: {e}", self.path.display()))?;
        let mut at = from;
        let mut buf = Vec::new();
        while at < to {
            let count = to.saturating_sub(at).min(INDEX_CHUNK_RECORDS);
            buf.clear();
            (&mut file)
                .take(count.saturating_mul(RECORD_LEN_U64))
                .read_to_end(&mut buf)
                .map_err(|e| format!("{}: {e}", self.path.display()))?;
            let whole = as_u64(buf.len()) / RECORD_LEN_U64;
            if whole == 0 {
                return Err(format!(
                    "{}: the file ended at record {at} of {to}",
                    self.path.display()
                ));
            }
            for (n, chunk) in buf.chunks_exact(RECORD_LEN).enumerate() {
                let mut image = [0u8; RECORD_LEN];
                for (dst, src) in image.iter_mut().zip(chunk) {
                    *dst = *src;
                }
                batch.file(at.saturating_add(as_u64(n)), &image);
            }
            batch.write(dir)?;
            at = at.saturating_add(whole);
        }
        Ok(())
    }

    /// Builds the index of the journal's first `upto` records from scratch in
    /// a directory of its own, then puts it in place: an old index is moved
    /// aside under a name with its replacement's time, never deleted.
    fn rebuild(&self, identity: (u64, u64), upto: u64, why: &str) -> Result<IndexBuilt, String> {
        let dir = self.index_dir();
        let building = dir.with_extension(format!("{INDEX_SUFFIX}.building"));
        // A BUILD A CRASH LEFT HALF-WRITTEN is derived bytes nobody reads:
        // it is cleared, never finished.
        match std::fs::remove_dir_all(&building) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
                return Err(format!("{}: {e}", building.display()));
            }
            _ => {}
        }
        std::fs::create_dir_all(&building)
            .and_then(|()| std::fs::write(building.join("ident"), ident_text(identity)))
            .map_err(|e| format!("{}: {e}", building.display()))?;
        let mut batch = Batch::default();
        self.file_range(0, upto, &mut batch, &building)?;
        write_cover(&building, upto)?;
        if dir.exists() {
            let nanos = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos());
            // A NAME OF ITS OWN, so a second replacement in one instant never
            // lands on the first one's.
            let aside = (0u32..1_000)
                .map(|n| dir.with_extension(format!("{INDEX_SUFFIX}.replaced-{nanos}-{n}")))
                .find(|aside| !aside.exists())
                .ok_or_else(|| format!("{}: no free name to set it aside", dir.display()))?;
            std::fs::rename(&dir, &aside).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        std::fs::rename(&building, &dir).map_err(|e| format!("{}: {e}", building.display()))?;
        let mut built = batch.built;
        built.from_scratch = true;
        built.why = Some(why.to_owned());
        built.covered = upto;
        Ok(built)
    }
}

/// The whole of a small file, or nothing when it is not there.
fn read_or_empty(path: &Path) -> Result<Vec<u8>, String> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(bytes),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(format!("{}: {e}", path.display())),
    }
}

/// Builds the journal's index under `root` in the background, once, when it
/// is absent or behind: what `api serve` calls at startup (WP-33c). The
/// outcome is logged ([`note_index`]); a lock a pull holds leaves the build to
/// that pull's append. Returns the thread so a test can wait on it.
#[must_use = "the thread is detached when its handle is dropped; a test waits on it"]
pub fn ensure_index(root: &Path) -> Option<std::thread::JoinHandle<()>> {
    let journal = Journal::at(root);
    if !journal.path.exists() {
        return None;
    }
    std::thread::Builder::new()
        .name("audit-index".to_owned())
        .spawn(move || note_index(&journal.path, &journal.build_index()))
        .ok()
}

/// The index's own line: built from scratch (with why and what it took), or
/// not written at all. `Info` with a lifecycle word when built, so the audit
/// stream keeps it; `Error` when it failed, which degrades every reader to a
/// walk it names. Silent for a catch-up of a few records and for a busy lock:
/// that is every append, and it is no news.
fn note_index(path: &Path, outcome: &Result<IndexBuilt, String>) {
    let event = match *outcome {
        Ok(ref built) if built.from_scratch => {
            telemetry::Event::info("api.audit", "journal index build finished")
                .with("records", telemetry::Value::Uint(built.records))
                .with("runs", telemetry::Value::Uint(built.runs))
                .with("refusals", telemetry::Value::Uint(built.refusals))
                .with("faults", telemetry::Value::Uint(built.faults))
                .with("micros", telemetry::Value::Uint(built.micros))
                .with(
                    "why",
                    telemetry::Value::Str(built.why.as_deref().unwrap_or("")),
                )
        }
        Ok(_) => return,
        Err(ref why) => telemetry::Event::error("api.audit", "journal index not written")
            .with("why", telemetry::Value::Str(why)),
    };
    let shown = path.display().to_string();
    let _dropped_when_filtered =
        telemetry::emit(&event.with("path", telemetry::Value::Str(&shown)));
}

#[cfg(test)]
#[allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic
)]
mod tests {
    use super::*;
    use pull::ingest::Failure;
    use pull::session::Day;
    use std::io::Write as _;

    fn d(y: u16, m: u8, day: u8) -> Day {
        Day::new(y, m, day).expect("a real date")
    }

    fn window() -> Window {
        Window::new(d(2025, 7, 1), d(2025, 7, 1)).expect("forwards")
    }

    fn at(secs: i64) -> SystemTime {
        if secs >= 0 {
            SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(secs.unsigned_abs())
        } else {
            SystemTime::UNIX_EPOCH - std::time::Duration::from_secs(secs.unsigned_abs())
        }
    }

    fn census(before_open: u32, after_close: u32, before_w: u32, after_w: u32) -> DropCensus {
        let mut c = DropCensus::new();
        for _ in 0..before_open {
            c.count(DropReason::BeforeSessionOpen);
        }
        for _ in 0..after_close {
            c.count(DropReason::AtOrAfterSessionClose);
        }
        for _ in 0..before_w {
            c.count(DropReason::BeforeWindow);
        }
        for _ in 0..after_w {
            c.count(DropReason::AfterWindow);
        }
        c
    }

    /// A CLEAN RUN THAT STORED NOTHING IS NOT `STORED`.
    ///
    /// The outcome was `if failures.is_empty() && balances() { Stored }`. A run
    /// over zero rows has no failures and balances trivially — 0 = 0 + 0 + 0 —
    /// so a window that landed nothing reported success in the journal and on
    /// the page.
    ///
    /// Across the ~11,200 windows of the stated backfill that is the difference
    /// between a complete history and a gap nobody finds: the month file
    /// exists, the census counts it, and the receipt said it worked.
    ///
    /// Asserted at the boundary. One bar stored is `STORED`; zero is not.
    #[test]
    fn a_run_that_stored_nothing_says_so_instead_of_reporting_success() {
        let empty = Ingested::default();
        assert!(
            empty.failures.is_empty() && empty.balances(),
            "the premise: a zero-row run is clean and balances, which is why \
             the old condition called it Stored"
        );

        let record = Record::of_run(Scope::Spot, at(5), 1, "folder", window(), &empty);
        assert_eq!(
            record.outcome,
            Outcome::Empty,
            "a clean run that stored no bars is STORED NOTHING, not STORED"
        );
        assert!(
            record.outcome.is_loud(),
            "and it is loud: not an error, but the operator's next action is \
             not `carry on`"
        );
        assert_eq!(record.outcome.label(), "STORED NOTHING");

        // ONE BAR IS THE OTHER SIDE OF THE BOUNDARY.
        let one = Ingested {
            bars_stored: 1,
            rows_read: 1,
            ..Ingested::default()
        };
        assert_eq!(
            Record::of_run(Scope::Spot, at(5), 1, "folder", window(), &one).outcome,
            Outcome::Stored,
            "one bar stored is a stored run"
        );

        // AND THE CODE IS APPENDED, NOT INSERTED. These are on-disk values in
        // an append-only journal; renumbering would re-read every record
        // written before today as a different outcome.
        assert_eq!(Outcome::Stored.code(), 0);
        assert_eq!(Outcome::Empty.code(), 4);

        // AND IT COMES BACK OFF DISK. Writing byte 4 was only ever half the
        // job: `of_code` did not know the byte, so a stored `Empty` decoded as
        // `UnknownOutcome { code: 4 }` and the row that exists to say STORED
        // NOTHING rendered as a faulted record instead. The journal is
        // append-only — a record the reader cannot name is lost for good.
        assert_eq!(
            Record::decode(&record.image()).expect("decodes").outcome,
            Outcome::Empty,
            "a STORED NOTHING run must read back as one"
        );
    }

    fn run() -> Ingested {
        Ingested {
            pending: None,
            bars_committed: 0,
            derived_files: 0,
            members: 194,
            rows_read: 354_675,
            bars_stored: 62_978,
            rows_folded: 291_527,
            counted: 194,
            census: census(0, 170, 0, 0),
            failures: Vec::new(),
        }
    }

    /// A store root nobody else in this process shares, emptied first.
    fn scratch(name: &str) -> PathBuf {
        let dir = crate::scratch::path(name);
        let _ignored = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a scratch directory");
        dir
    }

    /// THE NINE REASONS THAT USED TO BE NOWHERE.
    ///
    /// `of_run` keeps `failures.first()` and the stride is fixed, so a run that
    /// refused ten members left nine reasons off the disk entirely — which
    /// months had failed had to be recovered by decoding bar files. One
    /// `MemberFailure` record per failure puts every (instrument, month) pair
    /// in the journal at the same stride.
    #[test]
    fn every_failed_member_gets_its_own_record_at_the_same_stride() {
        let why = "bars span 2025-12 to 2026-01; the store addresses one month \
                   per file and splitting is the caller's decision, not this one's";
        let record = Record::member_failure(Scope::Spot, at(9), "BANKNIFTY", window(), why);

        assert_eq!(record.kind, Kind::MemberFailure);
        assert_eq!(record.outcome, Outcome::Failed);
        assert_eq!(record.source, "BANKNIFTY", "the instrument is recoverable");
        assert_eq!(record.failures, 1);
        assert_eq!(
            (record.members, record.rows_read, record.bars_stored),
            (0, 0, 0),
            "the counters belong to the run; repeating them here would give a \
             reader two places to add the same numbers up from"
        );
        assert!(
            record.note_was_cut(),
            "the premise: this reason is longer than the note field, which is \
             why the `pull.spot` event carries the untruncated one"
        );
        assert_eq!(
            usize::from(record.note_bytes),
            why.len(),
            "and says by how much"
        );

        let image = record.image();
        assert_eq!(
            image.len(),
            RECORD_LEN,
            "SAME STRIDE — the append stays O(1)"
        );
        assert_eq!(image[OFF_KIND], 1);
        assert_eq!(
            Record::decode(&image).expect("its own bytes"),
            record,
            "round-trips field for field"
        );
    }

    /// A reason in the shape the operator's journal holds for NIFTY on
    /// 2020-11-14 (145 bytes, cut to 60): synthetic wording, longer than the
    /// note field. D-3341, DB-G23.
    const LONG_WHY: &str = "request minute coverage UNVERIFIED on 2020-11-14: exceptional \
         session on a Saturday whose length no record states; 0 minute row(s) withheld";

    /// **THE WHOLE REASON SURVIVES THE 60-BYTE NOTE** (D-3341, DB-G23).
    ///
    /// Every one of the operator's 4.2 million member records was cut, so a
    /// month cell that wanted to say why Zerodha refused 2020-11-14 had 60 bytes
    /// of it. A member failure whose reason does not fit now keeps the whole
    /// reason in `pull.notes`, names its offset, and is written as version 3;
    /// one whose reason fits, and every run, stays version 2 byte for byte.
    #[test]
    fn a_cut_member_reason_is_kept_whole_beside_the_journal_and_read_back() {
        let root = scratch("audit-whole-note");
        let journal = Journal::at(&root);
        let cut = Record::member_failure(Scope::Spot, at(5), "NIFTY", window(), LONG_WHY);
        assert!(cut.note_was_cut(), "the premise: the note field cuts it");
        journal.append_member(&cut, LONG_WHY).expect("appends");
        let short = Record::member_failure(Scope::Spot, at(6), "NIFTY", window(), "refused");
        journal.append_member(&short, "refused").expect("appends");
        journal
            .append(&Record::refused(
                Scope::Spot,
                Outcome::Refused,
                at(7),
                "run",
                "r",
            ))
            .expect("appends");

        let records = journal.look().records();
        assert_eq!(records, 3, "one record each, at the same stride");
        let page = journal.page(records, 0, 10).expect("a page");
        let [run, fits, whole] = page.as_slice() else {
            panic!("three entries: {page:?}");
        };
        let whole_record = whole.decoded.clone().expect("decodes");
        assert_eq!(whole_record.version, VERSION_WHOLE_NOTE);
        assert_eq!(
            whole_record.counted, 0,
            "the offset is never read as a count"
        );
        assert_eq!(
            whole_record.note_at,
            Some(0),
            "the first entry of the notes file"
        );
        assert_eq!(
            whole.whole_note,
            Some(Ok(LONG_WHY.to_owned())),
            "the whole reason, read back by its offset"
        );
        let fits_record = fits.decoded.clone().expect("decodes");
        assert_eq!(
            fits_record.version, VERSION,
            "a reason that fits is version 2"
        );
        assert_eq!((fits_record.note_at, &fits.whole_note), (None, &None));
        assert_eq!(
            run.decoded.clone().expect("decodes").version,
            VERSION,
            "a run is never version 3"
        );
        assert_eq!(
            journal.whole_note(2, 0),
            Err(format!(
                "{}: the note at byte 0 belongs to record 0, not record 2, so it is not shown",
                journal.notes_path().display()
            )),
            "an offset followed from the wrong record is refused, never shown"
        );
    }

    /// A version-3 image is a member failure's alone: a run carrying it would
    /// have its census count read as an offset, so it is refused by name.
    #[test]
    fn a_whole_note_version_on_a_run_is_refused_rather_than_decoded() {
        let record = Record::of_run(Scope::Spot, at(1), 2, "src", window(), &run());
        let mut image = record.image();
        write_at(&mut image, OFF_VERSION, VERSION_WHOLE_NOTE.to_le_bytes());
        let crc = crc32c(&covered(&image));
        write_at(&mut image, OFF_CRC, crc.to_le_bytes());
        assert_eq!(Record::decode(&image), Err(RecordFault::WholeNoteOnARun));
    }

    /// A reason past [`MAX_WHOLE_NOTE`] is kept to that bound and comes back
    /// saying it was cut, with its whole length — never passed off as whole.
    #[test]
    fn a_reason_past_the_whole_note_bound_says_it_was_cut() {
        let root = scratch("audit-whole-note-bound");
        let journal = Journal::at(&root);
        let why = "x".repeat(MAX_WHOLE_NOTE + 10);
        let record = Record::member_failure(Scope::Spot, at(5), "NIFTY", window(), &why);
        journal.append_member(&record, &why).expect("appends");
        let page = journal.page(1, 0, 1).expect("a page");
        let read = page
            .first()
            .and_then(|entry| entry.whole_note.clone())
            .expect("a whole note")
            .expect("reads");
        assert!(
            read.ends_with(&format!("… (cut from {} bytes)", MAX_WHOLE_NOTE + 10)),
            "{}",
            &read[read.len().saturating_sub(40)..]
        );
        assert_eq!(
            read.len(),
            MAX_WHOLE_NOTE + "… (cut from 16394 bytes)".len()
        );
    }

    /// **A RUN IS ON THE JOURNAL WHILE IT RUNS, AND ITS FAILURES STAY FILED
    /// UNDER IT** (D-3341, DB-055, DB-G46).
    ///
    /// The run opens with an `OPENED` record; each failure follows as its
    /// member lands. When another writer appends in between — a second run of
    /// another feed — the opening record is restated before the next failure,
    /// so a reader that files a member under the run record before it never
    /// files this run's failure under the other run's feed.
    #[test]
    fn a_run_opens_before_its_members_and_restates_itself_after_another_writer() {
        let root = scratch("audit-thread");
        let journal = Journal::at(&root);
        let head = Record::opened(
            Scope::Spot,
            at(1),
            "https://api.kite.trade/x",
            window(),
            2,
            0,
        );
        assert_eq!(head.kind, Kind::Run);
        assert_eq!(head.outcome, Outcome::Opened);
        assert!(!head.outcome.is_loud(), "a run beginning is not a verdict");
        let mut thread = journal.open_run(&head).expect("opens");
        assert_eq!((thread.first(), thread.last()), (0, 0));

        let first = Record::member_failure(Scope::Spot, at(2), "NIFTY", window(), LONG_WHY);
        journal
            .append_failure(&mut thread, &first, LONG_WHY)
            .expect("appends");
        assert_eq!(thread.last(), 1, "straight behind its opening record");

        journal
            .append(&Record::refused(
                Scope::Spot,
                Outcome::Refused,
                at(3),
                "other feed",
                "x",
            ))
            .expect("another writer");

        let second = Record::member_failure(Scope::Spot, at(4), "BANKNIFTY", window(), "refused");
        journal
            .append_failure(&mut thread, &second, "refused")
            .expect("appends");
        assert_eq!(thread.last(), 4, "the restated head at 3, the failure at 4");

        let records = journal.look().records();
        let page = journal.page(records, 0, 10).expect("a page");
        let decoded: Vec<Record> = page
            .iter()
            .rev()
            .map(|entry| entry.decoded.clone().expect("decodes"))
            .collect();
        let shape: Vec<(Kind, Outcome, &str)> = decoded
            .iter()
            .map(|r| (r.kind, r.outcome, r.source.as_str()))
            .collect();
        assert_eq!(
            shape,
            [
                (Kind::Run, Outcome::Opened, "https://api.kite.trade/x"),
                (Kind::MemberFailure, Outcome::Failed, "NIFTY"),
                (Kind::Run, Outcome::Refused, "other feed"),
                (Kind::Run, Outcome::Opened, "https://api.kite.trade/x"),
                (Kind::MemberFailure, Outcome::Failed, "BANKNIFTY"),
            ]
        );
        assert_eq!(
            decoded.get(3).map(|r| r.note.as_str()),
            Some("continues the run opened at record 0")
        );
        assert_eq!(
            page.iter().rev().nth(1).and_then(|e| e.whole_note.clone()),
            Some(Ok(LONG_WHY.to_owned())),
            "and the threaded failure keeps its whole reason"
        );
    }

    /// **A RUN'S CLOSE FOLLOWS A RECORD OF ITS OWN RUN** (DB-B34, repair 6;
    /// D-4950). Through the run's thread, a close lands straight behind the
    /// run's last record, or behind its opening record written again when
    /// another writer appended since; a run whose opening record was refused
    /// closes behind that record written late, marked `; late`, in one write.
    /// A refused append writes nothing and leaves the thread where it was.
    #[test]
    fn a_close_lands_behind_a_record_of_its_own_run() {
        let root = scratch("audit-close-run");
        let journal = Journal::at(&root);
        let head = Record::opened(Scope::Spot, at(1), "kite", window(), 2, 0xa5);
        let close = Record::refused(Scope::Spot, Outcome::Stored, at(9), "kite", "done")
            .with_window(window());

        // Nothing between: straight behind the opening record.
        let mut thread = journal.open_run(&head).expect("opens");
        journal.close_run(&mut thread, &close).expect("closes");
        assert_eq!(thread.last(), 1);

        // Another writer between: the opening record again, then the close.
        let mut thread = journal.open_run(&head).expect("opens");
        journal
            .append(&Record::refused(
                Scope::Spot,
                Outcome::Refused,
                at(3),
                "other",
                "x",
            ))
            .expect("another writer");
        journal.close_run(&mut thread, &close).expect("closes");
        assert_eq!(thread.last(), 5, "the restated head at 4, the close at 5");

        // Refused at the start, written late with its close.
        journal
            .close_unopened(&head, &close)
            .expect("the late head and the close");
        let records = journal.look().records();
        let shape: Vec<(Outcome, String)> = journal
            .page(records, 0, 16)
            .expect("a page")
            .iter()
            .rev()
            .map(|entry| {
                let record = entry.decoded.clone().expect("decodes");
                (record.outcome, record.note)
            })
            .collect();
        let opened = ask_note("opened for 2 instruments", 0xa5);
        assert_eq!(
            shape,
            [
                (Outcome::Opened, opened.clone()),
                (Outcome::Stored, "done".to_owned()),
                (Outcome::Opened, opened.clone()),
                (Outcome::Refused, "x".to_owned()),
                (
                    Outcome::Opened,
                    "continues the run opened at record 2".to_owned()
                ),
                (Outcome::Stored, "done".to_owned()),
                (Outcome::Opened, format!("{opened}; late")),
                (Outcome::Stored, "done".to_owned()),
            ]
        );

        // A held lock refuses the close: nothing lands, the thread stays.
        let mut thread = journal.open_run(&head).expect("opens");
        let before = (journal.look().records(), thread.last());
        let holder = std::fs::OpenOptions::new()
            .read(true)
            .append(true)
            .open(&journal.path)
            .expect("the journal exists");
        holder.try_lock().expect("this test holds the append lock");
        assert!(journal.close_run(&mut thread, &close).is_err());
        assert!(journal.close_unopened(&head, &close).is_err());
        assert_eq!((journal.look().records(), thread.last()), before);
        holder.unlock().expect("released");
        let _ = std::fs::remove_dir_all(&root);
    }

    /// **THE ASK ON AN OPENING RECORD** (DB-B34, repair 6; D-4950): written
    /// as sixteen hexadecimal digits after `; ask `, read back exactly, and
    /// read as NONE from every note that does not carry exactly that: an
    /// opening record before D-4950, a restatement, a digest cut short or run
    /// on. Two different lists of parts never share an input.
    #[test]
    fn an_opening_records_ask_is_written_and_read_back_exactly() {
        for ask in [0, 1, 0xa5, u64::MAX, ask_digest(&["spot", "Zerodha"])] {
            let note = ask_note("opened for 213 instruments", ask);
            assert!(note.len() <= NOTE_CAPACITY, "{note}");
            assert_eq!(ask_of(&note), Some(ask), "{note}");
            assert_eq!(ask_of(&format!("{note}; late")), Some(ask), "{note}");
            let head = Record::opened(Scope::Spot, at(1), "src", window(), 213, ask);
            let back = Record::decode(&head.image()).expect("decodes");
            assert!(!back.note_was_cut(), "{}", back.note);
            assert_eq!(ask_of(&back.note), Some(ask));
        }
        // The widest note this build writes still fits whole.
        let widest = format!(
            "{}; late",
            ask_note("opened for 99999 instruments", u64::MAX)
        );
        assert!(widest.len() <= NOTE_CAPACITY, "{widest}");
        assert_eq!(ask_of(&widest), Some(u64::MAX));
        for none in [
            "opened for 7 instruments; failures follow as they land",
            "continues the run opened at record 4217027",
            "expired F&O walk opened; its outcome record closes it",
            "opened for 7 instruments; ask 0123456789abcde",
            "opened for 7 instruments; ask 0123456789abcdef0",
            "opened for 7 instruments; ask 0123456789abcdeg",
            "opened for 7 instruments; ask ",
            "",
        ] {
            assert_eq!(ask_of(none), None, "{none:?}");
        }
        assert_ne!(ask_digest(&["a", "b"]), ask_digest(&["ab"]));
        assert_ne!(ask_digest(&["a", "b"]), ask_digest(&["b", "a"]));
        assert_ne!(ask_digest(&["a"]), ask_digest(&["a", ""]));
        assert_eq!(ask_digest(&["a", "b"]), ask_digest(&["a", "b"]));
        // FNV-1a's own published value for the empty input, before the
        // separator: the basis. One part "" is the basis folded with 0x1F.
        assert_eq!(ask_digest(&[]), 0xcbf2_9ce4_8422_2325);
        assert_eq!(
            ask_digest(&[""]),
            (0xcbf2_9ce4_8422_2325_u64 ^ 0x1f).wrapping_mul(0x0000_0100_0000_01b3)
        );
    }

    /// The new outcome round-trips at the code appended for it, and every code
    /// before it still means what it meant.
    #[test]
    fn the_opened_outcome_is_appended_to_the_code_space() {
        assert_eq!(Outcome::Opened.code(), 5);
        assert_eq!(Outcome::of_code(5), Some(Outcome::Opened));
        assert_eq!(Outcome::of_code(4), Some(Outcome::Empty));
        assert_eq!(Outcome::Opened.label(), "OPENED");
        let head = Record::opened(Scope::Spot, at(1), "src", window(), 7, 0x5eed);
        assert_eq!(Record::decode(&head.image()), Ok(head));
    }

    /// A journal written before `Kind` existed must decode unchanged.
    ///
    /// Byte 118 was reserved and zero, and `Kind::Run` is code 0 — so this is
    /// not a compatibility hope, it is arithmetic. Proven by zeroing the byte
    /// on a real run record, which is exactly what an older writer left there.
    #[test]
    fn a_record_written_before_the_kind_byte_existed_still_reads_as_a_run() {
        let record = Record::of_run(Scope::Spot, at(1), 2, "src", window(), &run());
        let mut image = record.image();
        assert_eq!(
            image[OFF_KIND], 0,
            "a run writes the byte an older build left"
        );

        // Re-stamp it exactly as a pre-`Kind` writer would: byte 118 untouched
        // at zero, and the CRC recomputed over the record it actually wrote.
        image[OFF_KIND] = 0;
        let crc = crc32c(&covered(&image));
        image[OFF_CRC..].copy_from_slice(&crc.to_le_bytes());

        let back = Record::decode(&image).expect("an older journal still reads");
        assert_eq!(back.kind, Kind::Run);
        assert_eq!(back, record, "and nothing else moved");
    }

    /// A kind this build does not know is REFUSED, never rendered as a run.
    #[test]
    fn a_record_kind_from_a_later_build_is_refused_rather_than_guessed() {
        let record = Record::of_run(Scope::Spot, at(1), 2, "src", window(), &run());
        let mut image = record.image();
        image[OFF_KIND] = 7;
        let crc = crc32c(&covered(&image));
        image[OFF_CRC..].copy_from_slice(&crc.to_le_bytes());

        assert_eq!(
            Record::decode(&image),
            Err(RecordFault::UnknownKind { code: 7 }),
            "guessing would put a later build's fields on the page as a run's \
             numbers, which is worse than a named refusal"
        );
    }

    /// EXACTLY ONE BYTE OF THIS RECORD IS FREE, WHICH IS WHY NO IDENTITY FITS.
    ///
    /// The module header states, under `CLAUDE.md` §3 rule 6, that this journal
    /// carries none of the eight terms of §3 rule 3 and that recording them
    /// would be a stride change. The load-bearing half of that claim is
    /// arithmetic — a `blake3` output is 32 bytes and this record has one byte
    /// spare — and a layout claim written only in prose goes stale the first
    /// time an offset moves. So it is asserted, in two halves that catch
    /// different mistakes.
    ///
    /// This proves nothing about whether an identity SHOULD be recorded. It
    /// proves only that it cannot be recorded at this stride, which is the one
    /// thing the header asserts on its own authority.
    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "one field map restated whole; version 2's two counters took it past 100"
    )]
    fn the_record_has_exactly_one_free_byte() {
        // HALF ONE — THE DECLARED MAP. Every field constant with its width,
        // marked off against the stride. This catches an offset MOVED (two
        // fields overlap, or a hole opens) and it catches a field WIDENED. It
        // restates the constants, which is the price of checking them against
        // each other; the existing `const _` blocks only pin the three
        // adjacencies at the tail and say nothing about the counter block.
        let map: [(usize, usize); 28] = [
            (OFF_MAGIC, 4),
            (OFF_VERSION, 2),
            (OFF_SCOPE, 1),
            (OFF_OUTCOME, 1),
            (OFF_AT, 8),
            (OFF_ELAPSED, 8),
            (OFF_MEMBERS, 8),
            (OFF_ROWS_READ, 8),
            (OFF_BARS_STORED, 8),
            (OFF_ROWS_FOLDED, 8),
            (OFF_COUNTED, 8),
            (OFF_BEFORE_OPEN, 8),
            (OFF_AFTER_CLOSE, 8),
            (OFF_BEFORE_WINDOW, 8),
            (OFF_AFTER_WINDOW, 8),
            (OFF_FAILURES, 8),
            (OFF_FROM_DAYS, 4),
            (OFF_TO_DAYS, 4),
            (OFF_SOURCE_LEN, 2),
            (OFF_NOTE_LEN, 2),
            (OFF_SOURCE_KEPT, 1),
            (OFF_NOTE_KEPT, 1),
            (OFF_KIND, 1),
            (OFF_SOURCE, SOURCE_CAPACITY),
            (OFF_CLOSED_DAY, 4),
            (OFF_UNCLASSIFIED_KEPT, 4),
            (OFF_NOTE, NOTE_CAPACITY),
            (OFF_CRC, 4),
        ];
        let mut used = [false; RECORD_LEN];
        for (offset, width) in map {
            // Marked through the slice's own iterator rather than by index,
            // so the bound is checked once here instead of on every byte --
            // `skip`/`take` would otherwise mark NOTHING for a field declared
            // past the stride and the overlap check would pass vacuously.
            assert!(
                offset.saturating_add(width) <= RECORD_LEN,
                "a field at {offset} of {width} bytes runs past the stride"
            );
            for (b, slot) in used.iter_mut().enumerate().skip(offset).take(width) {
                assert!(
                    !*slot,
                    "byte {b} is claimed by two fields -- one of them is \
                     overwriting the other on every append"
                );
                *slot = true;
            }
        }
        let free: Vec<usize> = (0..RECORD_LEN).filter(|b| !used[*b]).collect();
        assert_eq!(
            free,
            vec![119],
            "the header's claim is that byte 119 is the WHOLE of the free \
             space. A `blake3` run identity is 32 bytes, so if this list ever \
             grows the header is what has to change with it"
        );

        // HALF TWO — WHAT THE WRITER ACTUALLY TOUCHES. Half one is a
        // restatement and can be forgotten; this is derived from `image()`
        // itself, so a field added to the writer and not to the list above is
        // still caught. Every value is saturated and both texts are filled to
        // capacity, so the only bytes that stay zero are bytes nothing writes.
        let mut record = Record::of_run(
            Scope::Fno,
            at(1_751_356_800),
            u64::MAX,
            &"S".repeat(SOURCE_CAPACITY * 2),
            window(),
            &run(),
        );
        record.members = u64::MAX;
        record.rows_read = u64::MAX;
        record.bars_stored = u64::MAX;
        record.rows_folded = u64::MAX;
        record.counted = u64::MAX;
        record.failures = u64::MAX;
        record.from_days = u32::MAX;
        record.to_days = u32::MAX;
        record.source_bytes = u16::MAX;
        record.note_bytes = u16::MAX;
        record.drops = Drops {
            before_open: u64::MAX,
            after_close: u64::MAX,
            before_window: u64::MAX,
            after_window: u64::MAX,
            on_closed_day: Some(u64::MAX),
            unclassified_kept: Some(u64::MAX),
        };
        record.note = keep(&"N".repeat(NOTE_CAPACITY * 2), NOTE_CAPACITY);
        assert_eq!(record.source.len(), SOURCE_CAPACITY, "the text is full");
        assert_eq!(record.note.len(), NOTE_CAPACITY, "and so is the note");

        // Across every shape this build can write, so a byte written only for
        // one kind, scope or outcome cannot hide behind a single sample.
        for kind in [Kind::Run, Kind::MemberFailure] {
            for scope in Scope::ALL {
                for outcome in Outcome::ALL {
                    record.kind = kind;
                    record.scope = scope;
                    record.outcome = outcome;
                    let image = record.image();
                    assert_eq!(
                        image[119],
                        0,
                        "byte 119 is the free byte: nothing this build writes \
                         may land there, and a {} / {} / {} record put \
                         something in it",
                        kind.label(),
                        scope.label(),
                        outcome.label()
                    );
                }
            }
        }
    }

    #[test]
    fn a_record_survives_the_round_trip_field_for_field() {
        let record = Record::of_run(
            Scope::Spot,
            at(1_751_356_800),
            4_512_903,
            "/Users/you/Downloads/GDFL/Futures/-III",
            window(),
            &run(),
        );
        let image = record.image();
        assert_eq!(image.len(), 256, "one record is one stride");
        let back = Record::decode(&image).expect("its own bytes");
        assert_eq!(back, record, "every field, not merely the counters");
        assert_eq!(back.bars_stored, 62_978);
        assert_eq!(back.rows_folded, 291_527);
        assert_eq!(back.drops.after_close, 170);
        assert_eq!(back.drops.total(), 170);
        assert_eq!(back.outcome, Outcome::Stored);
        assert_eq!(back.from_days, d(2025, 7, 1).days_from_epoch());
        assert_eq!(back.elapsed_micros, 4_512_903);
    }

    #[test]
    fn the_covered_domain_is_the_record_minus_its_checksum() {
        // A checksum over its own bytes is a number that always agrees with
        // itself. The domain is 0..252 and nothing else, and this is what says
        // so — changing the four checksum bytes must NOT change the covered
        // run, and changing any other byte must.
        let record = Record::refused(Scope::Fno, Outcome::Refused, at(10), "NIFTY", "expired");
        let image = record.image();
        assert_eq!(covered(&image).len(), 252);
        let mut poked = image;
        poked[OFF_CRC] ^= 0xff;
        assert_eq!(
            covered(&poked),
            covered(&image),
            "the checksum bytes are outside the domain"
        );
        let mut elsewhere = image;
        elsewhere[OFF_MEMBERS] ^= 0x01;
        assert_ne!(covered(&elsewhere), covered(&image));
    }

    #[test]
    fn a_flipped_bit_is_refused_by_name_and_both_numbers_are_reported() {
        let record = Record::refused(Scope::Spot, Outcome::NotStarted, at(0), "swept", "no http");
        let mut image = record.image();
        image[OFF_ROWS_READ] ^= 0x40;
        // BOTH NUMBERS ARE NAMED, and the assertion states both rather than
        // matching a shape: the stored checksum is the one still in the poked
        // bytes and the computed one is what those bytes now produce, so an
        // equality here is exact and there is no arm nothing reaches.
        let stored = u32::from_le_bytes(le4(&image, OFF_CRC));
        let computed = crc32c(&covered(&image));
        assert_ne!(stored, computed, "a flipped bit changes the checksum");
        assert_eq!(
            Record::decode(&image),
            Err(RecordFault::Checksum { stored, computed })
        );
        let said = RecordFault::Checksum { stored, computed }.to_string();
        assert!(said.contains("record checksum"), "{said}");
        assert!(said.contains("!="), "and both numbers, not one: {said}");
    }

    #[test]
    fn a_file_that_is_not_this_journal_is_refused_before_its_fields_are_read() {
        let image = [0u8; RECORD_LEN];
        assert_eq!(
            Record::decode(&image),
            Err(RecordFault::NotAnAuditRecord),
            "zeroes are not a record"
        );
        assert_eq!(
            RecordFault::NotAnAuditRecord.to_string(),
            "not an audit record"
        );
    }

    #[test]
    fn a_version_this_build_does_not_know_is_refused_and_named() {
        let mut image = Record::refused(Scope::Spot, Outcome::Refused, at(1), "x", "y").image();
        image[OFF_VERSION] = 9;
        // The checksum has to be re-sealed, or the version arm is unreachable:
        // the checksum is verified first, deliberately.
        let crc = crc32c(&covered(&image));
        for (dst, src) in image.iter_mut().skip(OFF_CRC).zip(crc.to_le_bytes()) {
            *dst = src;
        }
        assert_eq!(
            Record::decode(&image),
            Err(RecordFault::UnknownVersion { version: 9 })
        );
        assert!(
            RecordFault::UnknownVersion { version: 9 }
                .to_string()
                .contains("this build writes 2")
        );
    }

    #[test]
    fn an_unknown_scope_or_outcome_byte_is_refused_rather_than_guessed() {
        for (offset, expect) in [
            (OFF_SCOPE, RecordFault::UnknownScope { code: 7 }),
            (OFF_OUTCOME, RecordFault::UnknownOutcome { code: 7 }),
        ] {
            let mut image = Record::refused(Scope::Spot, Outcome::Refused, at(1), "x", "y").image();
            image[offset] = 7;
            let crc = crc32c(&covered(&image));
            for (dst, src) in image.iter_mut().skip(OFF_CRC).zip(crc.to_le_bytes()) {
                *dst = src;
            }
            assert_eq!(Record::decode(&image), Err(expect));
            assert!(expect.to_string().contains('7'), "{expect:?}");
        }
    }

    #[test]
    fn a_kept_length_past_its_field_is_refused_and_a_broken_byte_is_not_text() {
        let mut image = Record::refused(Scope::Spot, Outcome::Refused, at(1), "x", "y").image();
        image[OFF_SOURCE_KEPT] = 200;
        let crc = crc32c(&covered(&image));
        for (dst, src) in image.iter_mut().skip(OFF_CRC).zip(crc.to_le_bytes()) {
            *dst = src;
        }
        assert_eq!(
            Record::decode(&image),
            Err(RecordFault::KeptPastCapacity {
                kept: 200,
                capacity: 64
            })
        );
        assert_eq!(
            RecordFault::KeptPastCapacity {
                kept: 200,
                capacity: 64
            }
            .to_string(),
            "kept length 200 past a 64-byte field",
            "a refusal an operator can act on names both numbers"
        );

        let mut broken = Record::refused(Scope::Spot, Outcome::Refused, at(1), "abc", "y").image();
        broken[OFF_SOURCE] = 0xff;
        let crc = crc32c(&covered(&broken));
        for (dst, src) in broken.iter_mut().skip(OFF_CRC).zip(crc.to_le_bytes()) {
            *dst = src;
        }
        assert_eq!(Record::decode(&broken), Err(RecordFault::NotText));
        assert_eq!(
            RecordFault::NotText.to_string(),
            "a text field is not UTF-8"
        );

        // THE NOTE FIELD IS CHECKED TOO, not only the source. Two fields share
        // one decoder, so there are two call sites, and a `?` dropped at the
        // second would let a record with unreadable bytes render as an EMPTY
        // reason — which on the page reads exactly like a run that had nothing
        // to say.
        let mut note_broken =
            Record::refused(Scope::Spot, Outcome::Refused, at(1), "x", "why").image();
        note_broken[OFF_NOTE] = 0x80;
        let crc = crc32c(&covered(&note_broken));
        for (dst, src) in note_broken.iter_mut().skip(OFF_CRC).zip(crc.to_le_bytes()) {
            *dst = src;
        }
        assert_eq!(Record::decode(&note_broken), Err(RecordFault::NotText));
    }

    #[test]
    fn an_over_long_source_is_cut_on_a_character_boundary_and_says_it_was_cut() {
        // Every character here is three bytes, so a 64-byte field holds 21 of
        // them and cutting at 64 would land inside the 22nd. A record holding
        // half a character is a record that will not decode.
        let long = "अ".repeat(40);
        assert_eq!(long.len(), 120);
        let record = Record::refused(Scope::Spot, Outcome::Refused, at(1), &long, "y");
        assert_eq!(record.source.len(), 63, "21 whole characters, not 64 bytes");
        assert_eq!(record.source_bytes, 120, "the original length is kept");
        assert!(record.source_was_cut());
        assert!(!record.note_was_cut());
        let back = Record::decode(&record.image()).expect("still decodes");
        assert_eq!(back.source.chars().count(), 21);
        assert_eq!(back.source_bytes, 120);
        assert!(back.source_was_cut());
    }

    #[test]
    fn a_failed_member_becomes_the_note_and_the_outcome_is_not_stored() {
        let mut done = run();
        done.failures.push(Failure {
            instrument: "ABB-III".to_owned(),
            why: "the census refused the record".to_owned(),
            month: None,
            day: None,
        });
        let record = Record::of_run(Scope::Spot, at(5), 1, "folder", window(), &done);
        assert_eq!(record.outcome, Outcome::Failed);
        assert_eq!(record.failures, 1);
        assert!(
            record.note.starts_with("ABB-III — the census"),
            "{}",
            record.note
        );
        assert!(Outcome::Failed.is_loud());
        assert!(!Outcome::Stored.is_loud());
    }

    #[test]
    fn books_that_do_not_balance_are_recorded_as_failed_even_with_no_named_member() {
        let mut done = run();
        done.rows_read += 1; // one row with nowhere to be
        assert!(!done.balances());
        let record = Record::of_run(Scope::Spot, at(5), 1, "folder", window(), &done);
        assert_eq!(record.outcome, Outcome::Failed);
        assert_eq!(
            record.failures, 0,
            "no member failed, and it still does not balance"
        );
        assert!(record.note.contains("DO NOT BALANCE"), "{}", record.note);
    }

    #[test]
    fn an_absent_journal_is_a_state_and_not_a_failure() {
        let root = scratch("audit-absent");
        let journal = Journal::at(&root);
        assert_eq!(journal.look(), Log::Absent);
        assert_eq!(journal.look().records(), 0);
        assert!(!journal.look().is_loud());
        assert!(journal.page(0, 0, 10).expect("no records").is_empty());
        assert!(
            journal.path.ends_with("audit/pull.journal"),
            "{:?}",
            journal.path
        );
    }

    /// **A missing store root is refused, never recreated.** W1-api1-9, D-0954.
    ///
    /// `create_dir_all(root/audit)` made every missing ancestor, so a store
    /// root that vanished (an unmounted volume whose mountpoint path is still
    /// writable) was recreated and the journal split onto another device. A
    /// root that is gone, a root two levels gone, and a root that is a dangling
    /// symlink are each refused by name with nothing created at any level; an
    /// existing root with no `audit/` still gets it (the first append of a
    /// fresh store), an existing `audit/` is reused, and a symlinked `audit/`
    /// that resolves to a directory is accepted as before.
    #[test]
    fn a_missing_store_root_is_refused_and_never_recreated() {
        let base = scratch("audit-missing-root");
        let record = Record::refused(Scope::Spot, Outcome::Stored, at(1), "x", "");
        let gone = base.join("volume");
        let deeper = base.join("mnt").join("volume");
        let dangling = base.join("link");
        std::os::unix::fs::symlink(base.join("nowhere"), &dangling).expect("a dangling link");
        for root in [&gone, &deeper, &dangling] {
            let why = Journal::at(root).append(&record).expect_err("refused");
            assert!(why.contains("store root is missing"), "{why}");
            assert!(why.contains("pull.journal"), "names the path: {why}");
            assert!(!root.exists(), "{} was recreated", root.display());
        }
        assert!(!base.join("mnt").exists(), "no ancestor was created");
        assert!(
            !base.join("nowhere").exists(),
            "a dangling link's target was not created"
        );

        let fresh = base.join("fresh");
        std::fs::create_dir(&fresh).expect("an existing root");
        Journal::at(&fresh)
            .append(&record)
            .expect("creates audit/ beneath it");
        Journal::at(&fresh).append(&record).expect("reuses audit/");
        assert!(matches!(
            Journal::at(&fresh).look(),
            Log::Held { records: 2, .. }
        ));

        let linked = base.join("linked");
        std::fs::create_dir_all(base.join("elsewhere")).expect("a real audit directory");
        std::fs::create_dir(&linked).expect("an existing root");
        std::os::unix::fs::symlink(base.join("elsewhere"), linked.join("audit"))
            .expect("a symlinked audit directory");
        Journal::at(&linked)
            .append(&record)
            .expect("a symlinked audit dir is a directory");
        assert!(base.join("elsewhere").join("pull.journal").is_file());
    }

    #[test]
    fn appending_creates_the_directory_and_the_count_is_the_length_divided() {
        let root = scratch("audit-append");
        let journal = Journal::at(&root);
        for i in 0..5u32 {
            let record = Record::refused(
                Scope::Spot,
                Outcome::Refused,
                at(i64::from(i)),
                "swept",
                "nothing yet",
            );
            journal.append(&record).expect("appends");
        }
        assert_eq!(
            journal.look(),
            Log::Held {
                records: 5,
                bytes: 5 * 256,
                torn: None,
            },
            "five whole records and nothing past them"
        );
    }

    #[test]
    fn the_tail_reads_only_the_records_it_shows() {
        // The bound this file exists for, asserted as an offset rather than as
        // a timing: page zero of a 40-record journal reads records 30..40 and
        // nothing before them, so the byte span is 10 x 256 whatever the file
        // holds.
        let root = scratch("audit-tail");
        let journal = Journal::at(&root);
        for i in 0..40u32 {
            journal
                .append(&Record::refused(
                    Scope::Spot,
                    Outcome::Refused,
                    at(i64::from(i)),
                    &format!("run-{i}"),
                    "nothing yet",
                ))
                .expect("appends");
        }
        let records = journal.look().records();
        assert_eq!(records, 40);

        let page = journal.page(records, 0, 10).expect("a page");
        assert_eq!(page.len(), 10, "ten rows, not forty");
        assert_eq!(page.first().map(|e| e.ordinal), Some(39), "newest first");
        assert_eq!(page.last().map(|e| e.ordinal), Some(30));
        let newest = page
            .first()
            .expect("a row")
            .decoded
            .clone()
            .expect("decodes");
        assert_eq!(newest.source, "run-39");

        let second = journal.page(records, 10, 10).expect("a page");
        assert_eq!(second.first().map(|e| e.ordinal), Some(29));
        assert_eq!(second.last().map(|e| e.ordinal), Some(20));

        // Past the end is empty rather than an error or a wrap.
        assert!(journal.page(records, 40, 10).expect("empty").is_empty());
        // And a caller asking for more than the ceiling gets the ceiling.
        let all = journal.page(records, 0, u64::MAX).expect("capped");
        assert_eq!(all.len(), 40, "40 records, capped at {MAX_PAGE_RECORDS}");
    }

    /// AHA-09 (h-cli-4, D-1854). A failed or short record write is truncated
    /// back to the length measured under the lock, so the journal is never
    /// left torn by its own writer and the next append lands whole; a failed
    /// truncation names both errors and changes nothing else.
    #[test]
    fn a_failed_record_write_is_truncated_back_and_the_next_append_lands() {
        let root = scratch("audit-rollback");
        let journal = Journal::at(&root);
        let record = Record::refused(Scope::Spot, Outcome::Stored, at(1), "whole", "");
        journal.append(&record).expect("appends");
        let before = std::fs::read(&journal.path).expect("reads");
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&journal.path)
            .expect("opens");
        let refusal =
            write_rolled_back(&mut file, &journal.path, 256, &record.image(), |f, raw| {
                f.write_all(raw.get(..100).expect("partial"))?;
                Err(std::io::Error::other("injected short write"))
            })
            .expect_err("a short write refuses");
        assert!(
            refusal.contains("injected short write")
                && refusal.contains("truncated back to 256 bytes"),
            "{refusal}"
        );
        drop(file);
        assert_eq!(std::fs::read(&journal.path).expect("reads"), before);
        assert_eq!(
            journal.look(),
            Log::Held {
                records: 1,
                bytes: 256,
                torn: None,
            }
        );
        journal
            .append(&record)
            .expect("the next append lands whole");
        assert_eq!(
            journal.look(),
            Log::Held {
                records: 2,
                bytes: 512,
                torn: None,
            }
        );

        let grown = std::fs::read(&journal.path).expect("reads");
        let mut read_only = std::fs::File::open(&journal.path).expect("opens read-only");
        let refusal = write_rolled_back(
            &mut read_only,
            &journal.path,
            512,
            &record.image(),
            std::io::Write::write_all,
        )
        .expect_err("a read-only handle refuses");
        assert!(
            refusal.contains("truncating its partial bytes back to 512 bytes also failed"),
            "{refusal}"
        );
        assert_eq!(std::fs::read(&journal.path).expect("reads"), grown);
    }

    #[test]
    fn a_torn_tail_is_named_and_the_whole_records_before_it_still_read() {
        let root = scratch("audit-torn");
        let journal = Journal::at(&root);
        journal
            .append(&Record::refused(
                Scope::Spot,
                Outcome::Stored,
                at(1),
                "whole",
                "",
            ))
            .expect("appends");
        // Half a record, exactly what a process killed inside a write leaves.
        {
            let mut f = std::fs::OpenOptions::new()
                .append(true)
                .open(&journal.path)
                .expect("opens");
            f.write_all(&[0u8; 100]).expect("writes");
        }
        assert_eq!(
            journal.look(),
            Log::Held {
                records: 1,
                bytes: 356,
                torn: Some(100),
            },
            "the whole record before the tear still counts, and the tear is \
             named in bytes rather than swallowed"
        );
        assert!(journal.look().is_loud(), "a tear is loud");
        let page = journal.page(1, 0, 10).expect("still reads");
        assert_eq!(page.len(), 1);
        assert_eq!(
            page.first()
                .expect("a row")
                .decoded
                .clone()
                .expect("decodes")
                .source,
            "whole"
        );

        let before = std::fs::read(&journal.path).expect("the torn journal");
        let why = journal
            .append(&Record::refused(
                Scope::Spot,
                Outcome::Stored,
                at(2),
                "must-not-land",
                "",
            ))
            .expect_err("a torn tail blocks later appends");
        assert!(why.contains("100 trailing byte(s)"), "{why}");
        assert!(why.contains("refused to append"), "{why}");
        assert!(why.contains("append-only"), "{why}");
        assert_eq!(
            std::fs::read(&journal.path).expect("still readable"),
            before,
            "the refusal neither appends after the tear nor repairs history"
        );
    }

    #[test]
    fn a_held_journal_lock_refuses_a_second_writer_without_appending() {
        let root = scratch("audit-locked");
        let journal = Journal::at(&root);
        journal
            .append(&Record::refused(
                Scope::Spot,
                Outcome::Stored,
                at(1),
                "first",
                "",
            ))
            .expect("the first record");

        // A guard, so the `drop` below unlocks explicitly: this test binary
        // spawns children, and a close-only release can survive in one of
        // them and refuse the append that must succeed (D-0693).
        let squatter = store::flock::Flock::try_lock(
            std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(&journal.path)
                .expect("the journal handle"),
            journal.path.clone(),
        )
        .expect("nothing else holds it");

        let why = journal
            .append(&Record::refused(
                Scope::Spot,
                Outcome::Stored,
                at(2),
                "blocked",
                "",
            ))
            .expect_err("the second writer is refused");
        assert!(why.contains("cannot take the journal append lock"), "{why}");
        assert!(why.contains("refused rather than interleaved"), "{why}");
        assert_eq!(
            std::fs::metadata(&journal.path).expect("metadata").len(),
            RECORD_LEN_U64,
            "a refused concurrent writer leaves no partial record"
        );

        drop(squatter);
        journal
            .append(&Record::refused(
                Scope::Spot,
                Outcome::Stored,
                at(3),
                "after-release",
                "",
            ))
            .expect("the OS releases the advisory lock with its handle");
        assert_eq!(
            journal.look(),
            Log::Held {
                records: 2,
                bytes: 2 * RECORD_LEN_U64,
                torn: None,
            }
        );
    }

    /// ONCE THE RECORD IS SYNCED, NOTHING MAY REFUSE THE APPEND. D-0695.
    ///
    /// `appended` ended with `file.release().map_err(|u| u.to_string())`, after
    /// `write_all` and `sync_all` had made the record durable, so a refused
    /// unlock was returned as a record "NOT in the journal". No test in this
    /// crate can make an unlock refuse, so this reads `appended`'s own source:
    /// after the `sync_all` statement, its only code is the guard's drop and
    /// `Ok(())`. A `?`, an `Err` or a `release` there fails this.
    #[test]
    fn nothing_after_the_sync_can_refuse_an_append() {
        let body = include_str!("audit.rs")
            .split_once(
                "    fn appended_images(&self, build: impl FnOnce(u64) -> Vec<u8>) -> Result<u64, String> {\n",
            )
            .expect("`appended_images` is in this file")
            .1
            .split_once("\n    }\n")
            .expect("`appended` ends")
            .0;
        let after_sync = body
            .split_once("file.sync_all()")
            .expect("`appended` syncs the record")
            .1
            .split_once(";\n")
            .expect("the sync statement ends")
            .1;
        let code: Vec<&str> = after_sync
            .lines()
            .map(str::trim)
            .filter(|line| !line.starts_with("//"))
            .collect();
        // THE INDEX CALL RETURNS `()` (WP-33c): it logs its own failure and
        // cannot carry one back, so it is the one other line allowed here.
        assert_eq!(
            code,
            [
                "self.index_after_sync(&meta, first, &images);",
                "drop(file);",
                "Ok(first)"
            ],
            "after the sync: {after_sync}"
        );
    }

    #[test]
    fn one_damaged_record_does_not_blank_the_rows_around_it() {
        let root = scratch("audit-damaged");
        let journal = Journal::at(&root);
        for i in 0..3u32 {
            journal
                .append(&Record::refused(
                    Scope::Spot,
                    Outcome::Stored,
                    at(i64::from(i)),
                    &format!("run-{i}"),
                    "",
                ))
                .expect("appends");
        }
        // Corrupt the middle record in place, byte 300 — inside record 1.
        let mut bytes = std::fs::read(&journal.path).expect("reads");
        bytes[300] ^= 0xff;
        std::fs::write(&journal.path, &bytes).expect("writes");

        let page = journal.page(3, 0, 10).expect("a page");
        assert_eq!(page.len(), 3, "all three rows are still there");
        assert!(page[0].decoded.is_ok(), "the newest is fine");
        assert!(page[2].decoded.is_ok(), "the oldest is fine");
        let middle = format!("{:?}", page[1].decoded);
        assert!(
            matches!(page[1].decoded, Err(RecordFault::Checksum { .. })),
            "the middle record is damaged and says so: {middle}"
        );
    }

    #[test]
    fn a_journal_path_that_cannot_be_written_is_a_named_refusal() {
        let root = scratch("audit-refused");
        // A FILE where the audit DIRECTORY has to be. `create_dir_all` refuses
        // it, and the pull page has to say so rather than lose the record.
        std::fs::write(root.join("audit"), b"not a directory").expect("writes");
        let journal = Journal::at(&root);
        let why = journal
            .append(&Record::refused(
                Scope::Spot,
                Outcome::Stored,
                at(1),
                "x",
                "",
            ))
            .expect_err("cannot create the directory");
        assert!(why.contains("cannot create the audit directory"), "{why}");
        assert!(why.contains("pull.journal"), "and names the path: {why}");
        // And looking at it is Unreadable or Absent, never a crash.
        assert!(!matches!(journal.look(), Log::Held { .. }));

        // A PATH WITH NO PARENT AT ALL. The root directory is the one path
        // `Path::parent` answers `None` for, so it is what drives the arm that
        // skips `create_dir_all` — and the open then refuses by name rather
        // than by assuming a directory step happened.
        let rootish = Journal {
            path: PathBuf::from("/"),
        };
        let why = rootish
            .append(&Record::refused(
                Scope::Spot,
                Outcome::Stored,
                at(1),
                "x",
                "",
            ))
            .expect_err("the root directory is not a journal");
        assert!(why.starts_with("/: cannot open the journal — "), "{why}");
    }

    #[test]
    fn a_page_over_a_journal_that_is_not_there_is_a_named_refusal() {
        let root = scratch("audit-missing");
        let journal = Journal::at(&root);
        // `records` says there are ten; the file says otherwise. That mismatch
        // is what a deleted journal between the look and the read looks like.
        let why = journal.page(10, 0, 5).expect_err("no file");
        assert!(why.contains("pull.journal"), "{why}");
    }

    #[test]
    fn a_short_file_refuses_the_read_rather_than_inventing_records() {
        let root = scratch("audit-short");
        let journal = Journal::at(&root);
        std::fs::create_dir_all(journal.path.parent().expect("a parent")).expect("dir");
        std::fs::write(&journal.path, [0u8; 64]).expect("writes");
        let why = journal.page(4, 0, 4).expect_err("not four records");
        assert!(why.contains("cannot read 4 record(s)"), "{why}");
    }

    #[test]
    fn every_drop_reason_this_build_counts_is_carried_and_shared_by_peak() {
        let drops = Drops::of_census(census(3, 40, 0, 10));
        assert_eq!(drops.of(DropReason::BeforeSessionOpen), 3);
        assert_eq!(drops.of(DropReason::AtOrAfterSessionClose), 40);
        assert_eq!(drops.of(DropReason::BeforeWindow), 0);
        assert_eq!(drops.of(DropReason::AfterWindow), 10);
        assert_eq!(drops.total(), 53);
        assert_eq!(drops.peak(), 40);
        assert_eq!(Drops::default().peak(), 1, "never a division by zero");
        assert_eq!(Drops::default().total(), 0);
        assert_eq!(DROP_REASONS.len(), 5);

        // EVERY REASON CAN BE THE PEAK. A `peak` that only ever noticed the
        // first two counters would draw every share bar against the wrong
        // denominator for the other two, and the bar would still look
        // plausible — so each of the four is made the largest in turn.
        assert_eq!(
            Drops::of_census(census(9, 1, 1, 1)).peak(),
            9,
            "before open"
        );
        assert_eq!(
            Drops::of_census(census(1, 9, 1, 1)).peak(),
            9,
            "after close"
        );
        assert_eq!(
            Drops::of_census(census(1, 1, 9, 1)).peak(),
            9,
            "before window"
        );
        assert_eq!(
            Drops::of_census(census(1, 1, 1, 9)).peak(),
            9,
            "after window"
        );
        assert_eq!(
            Drops::of_census(census(1, 1, 9, 1)).of(DropReason::BeforeWindow),
            9
        );
        assert_eq!(Drops::of_census(census(1, 1, 9, 1)).total(), 12);
    }

    /// P-03, D-2673: the closed-day drop and the unclassified-day keep are
    /// carried by a version-2 record, round-trip, and the drop joins the total
    /// and the peak while the keep never does.
    #[test]
    fn a_version_two_record_carries_the_closed_day_drop_and_the_unclassified_keep() {
        let mut record = Record::refused(Scope::Spot, Outcome::Stored, at(7), "NIFTY", "ok");
        assert_eq!(record.version, VERSION);
        record.drops = Drops {
            before_open: 1,
            after_close: 2,
            before_window: 3,
            after_window: 4,
            on_closed_day: Some(375),
            unclassified_kept: Some(12),
        };
        let image = record.image();
        assert_eq!(
            u16::from_le_bytes(le2(&image, OFF_VERSION)),
            2,
            "written as v2"
        );
        assert_eq!(u32::from_le_bytes(le4(&image, OFF_CLOSED_DAY)), 375);
        assert_eq!(u32::from_le_bytes(le4(&image, OFF_UNCLASSIFIED_KEPT)), 12);
        let back = Record::decode(&image).expect("its own bytes");
        assert_eq!(back, record);
        assert_eq!(back.drops.total(), 385, "the closed-day drop is a drop");
        assert_eq!(back.drops.peak(), 375, "and can be the peak");
        assert_eq!(back.drops.recorded(DropReason::OnClosedDay), Some(375));
        assert_eq!(back.drops.unclassified_kept, Some(12));
        assert_eq!(back.note_capacity(), NOTE_CAPACITY);
        // A figure past `u32` saturates rather than wrapping smaller.
        record.drops.on_closed_day = Some(u64::MAX);
        let wide = Record::decode(&record.image()).expect("bytes");
        assert_eq!(wide.drops.on_closed_day, Some(u64::from(u32::MAX)));
        // The census feeds both.
        let mut census = DropCensus::new();
        census.count(DropReason::OnClosedDay);
        census.count_unclassified_kept();
        let drops = Drops::of_census(census);
        assert_eq!(drops.on_closed_day, Some(1));
        assert_eq!(drops.unclassified_kept, Some(1));
        assert_eq!(drops.total(), 1);
    }

    /// A version-1 record, built byte by byte in the version-1 map rather than
    /// through `image()`, which writes only the current version.
    fn version_one_image(note: &str) -> [u8; RECORD_LEN] {
        let mut image = [0u8; RECORD_LEN];
        image[OFF_MAGIC..OFF_MAGIC + 4].copy_from_slice(&MAGIC);
        image[OFF_VERSION..OFF_VERSION + 2].copy_from_slice(&VERSION_ONE.to_le_bytes());
        image[OFF_SCOPE] = Scope::Spot.code();
        image[OFF_OUTCOME] = Outcome::Stored.code();
        image[OFF_KIND] = Kind::Run.code();
        image[OFF_AT..OFF_AT + 8].copy_from_slice(&42_i64.to_le_bytes());
        image[OFF_AFTER_CLOSE..OFF_AFTER_CLOSE + 8].copy_from_slice(&170_u64.to_le_bytes());
        image[OFF_AFTER_WINDOW..OFF_AFTER_WINDOW + 8].copy_from_slice(&5_u64.to_le_bytes());
        let source = b"NIFTY";
        image[OFF_SOURCE..OFF_SOURCE + source.len()].copy_from_slice(source);
        image[OFF_SOURCE_KEPT] = 5;
        image[OFF_SOURCE_LEN..OFF_SOURCE_LEN + 2].copy_from_slice(&5_u16.to_le_bytes());
        image[OFF_NOTE_V1..OFF_NOTE_V1 + NOTE_CAPACITY_V1].copy_from_slice(note.as_bytes());
        image[OFF_NOTE_KEPT] = 68;
        image[OFF_NOTE_LEN..OFF_NOTE_LEN + 2].copy_from_slice(&68_u16.to_le_bytes());
        let crc = crc32c(&covered(&image));
        image[OFF_CRC..OFF_CRC + 4].copy_from_slice(&crc.to_le_bytes());

        image
    }

    /// A version-1 record, written before the closed-day filter existed, still
    /// reads: its four counters and its 68-byte note exactly as written, and
    /// its two newer counters as UNKNOWN (`None`), never as a claimed zero.
    /// Rewriting it produces a version-2 image with the note cut to 60 bytes.
    #[test]
    fn a_version_one_record_still_reads_and_its_newer_counters_are_unknown() {
        let note = "V".repeat(NOTE_CAPACITY_V1);
        let image = version_one_image(&note);

        let old = Record::decode(&image).expect("a version-1 record still reads");
        assert_eq!(old.version, VERSION_ONE);
        assert_eq!(old.note, note, "all 68 bytes of the old note");
        assert_eq!(old.note_capacity(), NOTE_CAPACITY_V1);
        assert!(!old.note_was_cut());
        assert_eq!(old.source, "NIFTY");
        assert_eq!(old.drops.after_close, 170);
        assert_eq!(old.drops.after_window, 5);
        assert_eq!(old.drops.on_closed_day, None, "unknown, not zero");
        assert_eq!(old.drops.unclassified_kept, None);
        assert_eq!(old.drops.recorded(DropReason::OnClosedDay), None);
        assert_eq!(old.drops.of(DropReason::OnClosedDay), 0);
        assert_eq!(old.drops.total(), 175);

        // Written again, it is a version-2 image whose note fits its field.
        let again = Record::decode(&old.image()).expect("rewritten");
        assert_eq!(again.version, VERSION);
        assert_eq!(again.note.len(), NOTE_CAPACITY);
        assert!(again.note_was_cut());
        assert_eq!(again.drops.on_closed_day, Some(0));
        // VERSION 3 IS READ SINCE D-3341 (a member failure whose whole note
        // is kept), so the first version this build does not know is 4.
        let unknown = RecordFault::UnknownVersion { version: 4 }.to_string();
        assert!(unknown.contains("reads 1, 2 and 3"), "{unknown}");
    }

    /// ITERATED, NOT HAND-LISTED. This test used to name four outcomes and then
    /// assert `of_code(4) == None`, which is how `Empty` — stored as byte 4 —
    /// was written to an append-only journal it could never be read out of. The
    /// hand-written list was the defect: it agreed with the reader instead of
    /// checking it. `ALL` is now the single list, and the `const` assertions
    /// beside `Outcome::of_code` refuse to build if it drifts from the codec.
    #[test]
    fn scopes_and_outcomes_round_trip_through_their_stored_byte() {
        // WALKING `ALL` IN BYTE ORDER PROVES THREE THINGS AT ONCE: every
        // variant reads back as itself, the codes are dense from zero with no
        // gap and no duplicate, and one past the last is refused rather than
        // silently read as some other variant.
        let mut code = 0;
        for scope in Scope::ALL {
            assert_eq!(scope.code(), code, "ALL is in stored-byte order");
            assert_eq!(Scope::of_code(code), Some(scope), "{scope:?}");
            assert!(!scope.label().is_empty());
            code += 1;
        }
        assert_eq!(Scope::of_code(code), None, "one past the last is unused");

        let mut code = 0;
        for outcome in Outcome::ALL {
            assert_eq!(outcome.code(), code, "ALL is in stored-byte order");
            assert_eq!(
                Outcome::of_code(code),
                Some(outcome),
                "{outcome:?} is written as byte {code} and must read back as \
                 itself — the journal is append-only, so a byte the reader \
                 does not know is a record lost for good"
            );
            assert!(!outcome.label().is_empty());
            code += 1;
        }
        assert_eq!(Outcome::of_code(code), None, "one past the last is unused");

        assert_eq!(Scope::Fno.label(), "expired F&O");
        assert_eq!(Outcome::NotStarted.label(), "NOT STARTED");
        assert_eq!(Outcome::Empty.label(), "STORED NOTHING");
    }

    #[test]
    fn a_window_can_be_attached_to_a_refusal_after_the_fact() {
        let record = Record::refused(Scope::Fno, Outcome::Refused, at(1), "NIFTY", "expired")
            .with_window(window());
        assert_eq!(record.from_days, d(2025, 7, 1).days_from_epoch());
        assert_eq!(record.to_days, d(2025, 7, 1).days_from_epoch());
        let back = Record::decode(&record.image()).expect("decodes");
        assert_eq!(back.from_days, record.from_days);
    }

    #[test]
    fn a_clock_before_the_epoch_is_recorded_as_a_negative_second_not_a_wrap() {
        let record = Record::refused(Scope::Spot, Outcome::Refused, at(-86_400), "x", "y");
        assert_eq!(record.at_unix_secs, -86_400);
        let back = Record::decode(&record.image()).expect("decodes");
        assert_eq!(back.at_unix_secs, -86_400, "signed, not wrapped");
    }

    #[test]
    fn an_unreadable_log_is_loud_and_a_held_one_is_not() {
        assert!(
            Log::Unreadable {
                reason: "x".to_owned()
            }
            .is_loud()
        );
        assert_eq!(
            Log::Unreadable {
                reason: "x".to_owned()
            }
            .records(),
            0
        );
        assert!(
            !Log::Held {
                records: 3,
                bytes: 768,
                torn: None
            }
            .is_loud()
        );
        assert_eq!(
            Log::Held {
                records: 3,
                bytes: 768,
                torn: None
            }
            .records(),
            3
        );
    }

    // -----------------------------------------------------------------------
    // THE JOURNAL'S INDEX (WP-33c; DB-A32, DB-A34)
    // -----------------------------------------------------------------------

    /// A run record whose source names Zerodha by its policy.
    fn zerodha_run(secs: i64) -> Record {
        Record::refused(
            Scope::Spot,
            Outcome::Failed,
            at(secs),
            "zerodha_nse_checked fixture",
            "fixture run",
        )
    }

    /// A member failure of `symbol` with `note`.
    fn member(symbol: &str, note: &str) -> Record {
        Record::member_failure(Scope::Spot, at(1_791_266_622), symbol, window(), note)
    }

    /// **EVERY APPEND KEEPS THE INDEX, AND A READER GETS THE RUNS FROM IT**:
    /// the runs with the vendor their source names, a member failure that may
    /// name a refusal filed under its symbol, one that does not filed nowhere,
    /// and the cover at the journal's length, all without a walk.
    #[test]
    fn every_append_keeps_the_index_and_the_runs_are_read_from_it() {
        let root = scratch("audit-index-append");
        let journal = Journal::at(&root);
        journal.append(&zerodha_run(1_000)).unwrap();
        journal
            .append(&member(
                "ZZOIL",
                "2020-11-14: 60 minute row(s) refused: fixture",
            ))
            .unwrap();
        journal
            .append(&member(
                "ZZOIL",
                "request minute coverage UNVERIFIED on 2018-11-07",
            ))
            .unwrap();
        journal
            .append(&Record::refused(
                Scope::Spot,
                Outcome::Failed,
                at(2_000),
                "Reference indices",
                "no vendor",
            ))
            .unwrap();
        journal
            .append(&member(
                "ZZOTHER",
                "conflicting vendor candles at 1578853800",
            ))
            .unwrap();
        let indexed = journal.read_index(5).expect("an index");
        assert_eq!(indexed.covered, 5);
        assert_eq!(
            indexed.runs,
            vec![(0, Some(brutex_core::vendor::Vendor::Zerodha)), (3, None)]
        );
        assert_eq!(
            indexed.vendor_before(1),
            Some(brutex_core::vendor::Vendor::Zerodha)
        );
        assert_eq!(indexed.vendor_before(4), None);
        assert_eq!(indexed.faults, 0);
        assert_eq!(journal.refusal_ordinals("ZZOIL", 5).unwrap(), vec![1]);
        assert_eq!(journal.refusal_ordinals("ZZOTHER", 5).unwrap(), vec![4]);
        assert!(journal.refusal_ordinals("NOBODY", 5).unwrap().is_empty());
        assert_eq!(
            journal.refusal_ordinals("ZZOTHER", 4).unwrap(),
            Vec::<u64>::new(),
            "nothing past the cover asked"
        );
        let read = journal.records_at(&[1, 4]).unwrap();
        assert_eq!(read[0].0, 1);
        assert_eq!(read[1].1.as_ref().unwrap().source, "ZZOTHER");
    }

    /// **A JOURNAL WRITTEN BEFORE THE INDEX IS INDEXED ONCE, WHOLE**: a
    /// journal with no index (the operator's, before WP-33c) is built by
    /// `build_index` from its first record; a second call has nothing to do;
    /// a record appended behind the index's back (an older build) is caught up
    /// by the next build and by the next append.
    #[test]
    fn a_journal_with_no_index_is_built_once_and_caught_up() {
        let root = scratch("audit-index-backfill");
        let journal = Journal::at(&root);
        journal.append(&zerodha_run(1_000)).unwrap();
        journal
            .append(&member("ZZOIL", "refused: fixture"))
            .unwrap();
        std::fs::remove_dir_all(journal.index_dir()).unwrap();
        assert!(
            journal
                .read_index(2)
                .is_err_and(|why| why.contains("no index")),
            "absent is said, never read as empty"
        );
        let built = journal.build_index().unwrap();
        assert!(built.from_scratch);
        assert_eq!(
            (built.records, built.runs, built.refusals, built.covered),
            (2, 1, 1, 2)
        );
        let again = journal.build_index().unwrap();
        assert!(!again.from_scratch);
        assert_eq!((again.records, again.covered), (0, 2));
        // A RECORD WRITTEN BY A BUILD WITH NO INDEX: past the cover.
        let mut bytes = std::fs::read(&journal.path).unwrap();
        bytes.extend_from_slice(&member("ZZOIL", "refused: late").image());
        std::fs::write(&journal.path, &bytes).unwrap();
        assert_eq!(journal.read_index(3).unwrap().covered, 2);
        journal.append(&member("ZZOIL", "refused: next")).unwrap();
        let indexed = journal.read_index(4).unwrap();
        assert_eq!(indexed.covered, 4);
        assert_eq!(journal.refusal_ordinals("ZZOIL", 4).unwrap(), vec![1, 2, 3]);
    }

    /// **AN INDEX OF ANOTHER FILE, OR AHEAD OF ITS JOURNAL, IS REFUSED, AND THE
    /// NEXT BUILD REPLACES IT, KEEPING THE OLD ONE ASIDE**; an unreadable cover
    /// is refused by name; a record that will not decode is counted.
    #[test]
    fn a_stale_index_is_refused_and_replaced_never_trusted() {
        let root = scratch("audit-index-stale");
        let journal = Journal::at(&root);
        journal.append(&zerodha_run(1_000)).unwrap();
        journal
            .append(&member("ZZOIL", "refused: fixture"))
            .unwrap();
        // ANOTHER FILE: the journal replaced by a copy (a new inode).
        let copy = journal.path.with_extension("copy");
        std::fs::copy(&journal.path, &copy).unwrap();
        std::fs::rename(&copy, &journal.path).unwrap();
        assert!(
            journal
                .read_index(2)
                .is_err_and(|why| why.contains("another file")),
        );
        let rebuilt = journal.build_index().unwrap();
        assert!(rebuilt.from_scratch);
        assert!(rebuilt.why.unwrap().contains("another file"));
        assert_eq!(journal.read_index(2).unwrap().covered, 2);
        let aside = std::fs::read_dir(journal.path.parent().unwrap())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .contains(".idx.replaced-")
            })
            .count();
        assert_eq!(aside, 1, "the old index is kept aside, never deleted");
        // AHEAD OF ITS JOURNAL: a reader asked about fewer records.
        assert!(
            journal
                .read_index(1)
                .is_err_and(|why| why.contains("ahead of its journal")),
        );
        // A TORN COVER.
        std::fs::write(journal.index_dir().join("cover"), b"torn").unwrap();
        assert!(
            journal
                .read_index(2)
                .is_err_and(|why| why.contains("not a cover")),
        );
        // A RECORD THAT WILL NOT DECODE is counted.
        assert!(journal.build_index().unwrap().from_scratch);
        let mut broken = member("ZZOIL", "refused: broken").image();
        broken[60] ^= 0xff;
        let mut bytes = std::fs::read(&journal.path).unwrap();
        bytes.extend_from_slice(&broken);
        std::fs::write(&journal.path, &bytes).unwrap();
        let caught = journal.build_index().unwrap();
        assert_eq!((caught.records, caught.faults), (1, 1));
        assert_eq!(journal.read_index(3).unwrap().faults, 1);
    }

    /// **AN INDEX ENTRY IS ITS OWN, BY MARK AND CHECKSUM**: a run entry and an
    /// ordinal entry round-trip; a flipped bit or another kind's mark is not
    /// read; a source is a file name that cannot leave `refusals/`; a vendor
    /// code this build does not know is not read as any vendor.
    #[test]
    fn an_index_entry_is_read_only_when_it_is_one() {
        let run = run_entry(7, 1_000, Some(brutex_core::vendor::Vendor::Gdfl));
        let (ordinal, code, rule) = read_run_entry(&run).unwrap();
        assert_eq!((ordinal, rule), (7, ATTRIBUTION_RULE));
        assert_eq!(
            vendor_of_code(code),
            Ok(Some(brutex_core::vendor::Vendor::Gdfl))
        );
        assert_eq!(vendor_of_code(0), Ok(None));
        assert_eq!(vendor_of_code(250), Err(250));
        let mut flipped = run;
        flipped[3] ^= 1;
        assert_eq!(read_run_entry(&flipped), None);
        let refusal = ordinal_entry(9, MARK_REFUSAL);
        assert_eq!(read_ordinal_entry(&refusal, MARK_REFUSAL), Some(9));
        assert_eq!(read_ordinal_entry(&refusal, MARK_FAULT), None);
        assert_eq!(read_ordinal_entry(&refusal[..15], MARK_REFUSAL), None);
        assert_eq!(source_file("M&M").as_deref(), Some("M%26M.idx"));
        assert_eq!(source_file("../x").as_deref(), Some("%2E%2E%2Fx.idx"));
        assert_eq!(source_file(""), None);
        assert!(may_name_a_refusal(
            "2020-11-14: 60 minute row(s) refused: x"
        ));
        assert!(may_name_a_refusal("off-grid broker candle at 1578853800"));
        assert!(!may_name_a_refusal("request minute coverage UNVERIFIED"));
    }
}

/// Tests added by the cloud mutation pass (cloud/mut-api-b-20261016): each one
/// pins a behaviour a surviving mutant broke, or reaches a line no other test
/// reached.
#[cfg(test)]
#[allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "a test that cannot panic cannot fail, and these lints exist to \
              keep panics out of the crate rather than out of its tests"
)]
mod cloud_mutant_tests {
    use super::*;

    /// A store root nobody else in this process shares, emptied first.
    fn scratch(name: &str) -> PathBuf {
        let dir = crate::scratch::path(&format!("audit-cloud-{name}"));
        let _ignored = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a scratch directory");
        dir
    }

    fn at(secs: u64) -> SystemTime {
        SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(secs)
    }

    fn window() -> Window {
        let day = pull::session::Day::from_days(20_000).expect("a day");
        Window::new(day, day).expect("a window")
    }

    fn member(note: &str) -> Record {
        Record::member_failure(Scope::Spot, at(100), "NIFTY", window(), note)
    }

    fn run_record() -> Record {
        Record::refused(
            Scope::Spot,
            Outcome::Failed,
            at(90),
            "zerodha_kite",
            "closed",
        )
        .with_window(window())
    }

    /// The image's checksum recomputed after a field was edited in place.
    fn reseal(image: &mut [u8; RECORD_LEN]) {
        let crc = crc32c(&covered(image));
        write_at(image, OFF_CRC, crc.to_le_bytes());
    }

    /// The two faults no test had printed say exactly what they are.
    #[test]
    fn the_kind_and_whole_note_faults_say_what_they_are() {
        assert_eq!(
            RecordFault::UnknownKind { code: 9 }.to_string(),
            "record-kind byte 9 is not one this build writes — a later build wrote this \
             record and it is not read rather than being rendered as a run it is not"
        );
        assert_eq!(
            RecordFault::WholeNoteOnARun.to_string(),
            "a version-3 record that is not a member failure: only a member failure \
             carries a whole-note offset, so this record is not read"
        );
    }

    /// A version-1 record whose kept note runs past the version-1 field is
    /// refused by name, measured against the longer version-1 field.
    #[test]
    fn a_version_one_note_past_its_field_is_refused() {
        let mut image = run_record().image();
        write_at(&mut image, OFF_VERSION, VERSION_ONE.to_le_bytes());
        let past = u8::try_from(NOTE_CAPACITY_V1 + 1).expect("fits a byte");
        write_at(&mut image, OFF_NOTE_KEPT, [past]);
        reseal(&mut image);
        assert_eq!(
            Record::decode(&image),
            Err(RecordFault::KeptPastCapacity {
                kept: past,
                capacity: u8::try_from(NOTE_CAPACITY_V1).expect("fits a byte"),
            })
        );
    }

    /// A whole note exactly [`MAX_WHOLE_NOTE`] long is kept whole and read
    /// back as written (kills the cap's `<=` turned to `<`).
    #[test]
    fn a_whole_note_of_exactly_the_cap_reads_back_whole() {
        let root = scratch("note-at-cap");
        let journal = Journal::at(&root);
        let whole = "y".repeat(MAX_WHOLE_NOTE);
        journal
            .append_member(&member(&whole), &whole)
            .expect("the record lands");
        let page = journal.page(1, 0, 1).expect("a page");
        let record = page[0].decoded.as_ref().expect("decodes");
        assert_eq!(record.version, VERSION_WHOLE_NOTE);
        assert_eq!(page[0].whole_note, Some(Ok(whole.clone())));
        assert_eq!(journal.whole_note(0, 0), Ok(whole));
    }

    /// A notes entry that is not one, of another layout, claiming more than
    /// the cap, or failing its checksum is refused by name; and a notes file
    /// that is gone is named on the page's row rather than dropped.
    #[test]
    fn a_damaged_notes_entry_is_refused_by_name() {
        let root = scratch("note-damaged");
        let journal = Journal::at(&root);
        let whole = "x".repeat(NOTE_CAPACITY + 40);
        journal
            .append_member(&member(&whole), &whole)
            .expect("the record lands");
        let notes = journal.notes_path();
        let good = std::fs::read(&notes).expect("the notes file");
        let refused = |edit: &dyn Fn(&mut Vec<u8>)| {
            let mut bad = good.clone();
            edit(&mut bad);
            std::fs::write(&notes, &bad).expect("write");
            journal
                .whole_note(0, 0)
                .expect_err("a damaged entry is refused")
        };
        let why = refused(&|bytes| bytes[0] = b'X');
        assert!(
            why.ends_with("the note at byte 0 is not a note entry"),
            "{why}"
        );
        let why = refused(&|bytes| bytes[4] = 9);
        assert!(
            why.ends_with("has entry layout 9; this build reads 1"),
            "{why}"
        );
        let claimed = u32::try_from(MAX_WHOLE_NOTE + 1).expect("fits");
        let why = refused(&|bytes| bytes[20..24].copy_from_slice(&claimed.to_le_bytes()));
        assert!(
            why.ends_with(&format!(
                "claims {} bytes, past the {MAX_WHOLE_NOTE} kept",
                MAX_WHOLE_NOTE + 1
            )),
            "{why}"
        );
        let why = refused(&|bytes| bytes[NOTE_HEAD] ^= 1);
        assert!(why.contains("fails its checksum"), "{why}");

        std::fs::remove_file(&notes).expect("the notes file is removed");
        let page = journal.page(1, 0, 1).expect("the journal still pages");
        let Some(Err(why)) = &page[0].whole_note else {
            panic!(
                "a notes file that is gone is named: {:?}",
                page[0].whole_note
            );
        };
        assert!(why.contains("pull.notes"), "{why}");
    }

    /// A notes file whose lock another holder has: the record still lands,
    /// as version 2 with its note cut, and names no whole note.
    #[test]
    fn a_whole_note_that_cannot_be_locked_leaves_the_record_cut() {
        let root = scratch("note-locked");
        let journal = Journal::at(&root);
        std::fs::create_dir_all(root.join("audit")).expect("audit/");
        let notes = journal.notes_path();
        std::fs::write(&notes, b"").expect("an empty notes file");
        let holder = Flock::try_lock(
            std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(&notes)
                .expect("the notes handle"),
            notes.clone(),
        )
        .expect("nothing else holds it");
        let whole = "z".repeat(NOTE_CAPACITY + 40);
        let landed = journal.append_member(&member(&whole), &whole);
        drop(holder);
        landed.expect("the record lands without its whole note");
        let page = journal.page(1, 0, 1).expect("a page");
        let record = page[0].decoded.as_ref().expect("decodes");
        assert_eq!(record.version, VERSION);
        assert_eq!(record.note_at, None);
        assert_eq!(record.note, "z".repeat(NOTE_CAPACITY));
        assert_eq!(page[0].whole_note, None);
        assert_eq!(
            std::fs::read(&notes).expect("read"),
            b"",
            "nothing was written"
        );
    }

    /// A cover whose checksum fails is not read as a cover.
    #[test]
    fn a_cover_failing_its_checksum_is_not_read() {
        let root = scratch("cover-crc");
        let journal = Journal::at(&root);
        journal.append(&run_record()).expect("lands");
        let cover = journal.index_dir().join("cover");
        let mut bytes = std::fs::read(&cover).expect("the cover");
        bytes[13] ^= 0xFF;
        std::fs::write(&cover, bytes).expect("write");
        let why = journal.read_index(1).expect_err("refused");
        assert!(why.ends_with("not a cover this build wrote"), "{why}");
    }

    /// An `ident` that is there and cannot be read is named in its own
    /// words, never called "no index".
    #[test]
    fn an_ident_that_cannot_be_read_is_named() {
        let root = scratch("ident-dir");
        let journal = Journal::at(&root);
        journal.append(&run_record()).expect("lands");
        let ident = journal.index_dir().join("ident");
        std::fs::remove_file(&ident).expect("rm");
        std::fs::create_dir(&ident).expect("a directory where ident was");
        let why = journal.read_index(1).expect_err("refused");
        assert!(why.contains("ident"), "{why}");
        assert!(!why.starts_with("no index at"), "{why}");
    }

    /// A `runs` file that is a directory is named, never read as empty.
    #[test]
    fn an_index_file_that_cannot_be_read_is_named() {
        let root = scratch("runs-dir");
        let journal = Journal::at(&root);
        journal.append(&run_record()).expect("lands");
        let runs = journal.index_dir().join("runs");
        std::fs::remove_file(&runs).expect("rm");
        std::fs::create_dir(&runs).expect("a directory where runs was");
        let why = journal.read_index(1).expect_err("refused");
        assert!(why.contains("runs"), "{why}");
    }

    /// A `runs` entry filed under another attribution rule, or naming a
    /// vendor code this build does not know, has its vendor read from the
    /// record itself; one whose record will not decode is malformed.
    #[test]
    fn a_run_entry_of_another_rule_reads_its_vendor_from_the_record() {
        use std::os::unix::fs::FileExt as _;
        let root = scratch("runs-rule");
        let journal = Journal::at(&root);
        journal.append(&run_record()).expect("lands");
        let runs = journal.index_dir().join("runs");
        let rewrite = |code: u8, rule: u8| {
            let mut entry = run_entry(0, 90, None);
            entry[16] = code;
            entry[17] = rule;
            let crc = crc32c(&entry[..RUN_ENTRY - 4]);
            entry[RUN_ENTRY - 4..].copy_from_slice(&crc.to_le_bytes());
            std::fs::write(&runs, entry).expect("write");
        };
        for (code, rule) in [(0, ATTRIBUTION_RULE + 1), (200, ATTRIBUTION_RULE)] {
            rewrite(code, rule);
            let index = journal.read_index(1).expect("reads");
            assert_eq!(
                index.runs,
                vec![(0, Some(brutex_core::vendor::Vendor::Zerodha))],
                "code {code} rule {rule}"
            );
            assert_eq!(index.malformed, 0);
        }
        // The record itself damaged: nothing to read the vendor from.
        std::fs::OpenOptions::new()
            .write(true)
            .open(&journal.path)
            .expect("open")
            .write_all_at(&[0xFF; RECORD_LEN], 0)
            .expect("damage");
        let index = journal.read_index(1).expect("reads");
        assert!(index.runs.is_empty());
        assert_eq!(index.malformed, 1);
    }

    /// No source names no refusal file; a record past the end is not read;
    /// an absent journal builds no index.
    #[test]
    fn edges_of_the_index_readers_answer_rather_than_guess() {
        let root = scratch("index-edges");
        let journal = Journal::at(&root);
        assert_eq!(journal.build_index(), Ok(IndexBuilt::default()));
        assert!(
            !journal.index_dir().exists(),
            "an absent journal builds nothing"
        );
        journal.append(&run_record()).expect("lands");
        assert_eq!(journal.refusal_ordinals("", 10), Ok(Vec::new()));
        let why = journal.records_at(&[5]).expect_err("past the end");
        assert!(why.contains("cannot read record 5"), "{why}");
    }

    /// A half-built index that cannot be cleared refuses the rebuild by name.
    #[test]
    fn a_half_built_index_that_cannot_be_cleared_refuses_the_rebuild() {
        let root = scratch("building-file");
        let journal = Journal::at(&root);
        journal.append(&run_record()).expect("lands");
        let dir = journal.index_dir();
        std::fs::remove_dir_all(&dir).expect("the index is removed");
        let building = dir.with_extension(format!("{INDEX_SUFFIX}.building"));
        std::fs::write(&building, b"a file, not a half-built index").expect("write");
        let why = journal.build_index().expect_err("refused");
        assert!(why.contains(".building"), "{why}");
    }

    /// A catch-up that must file a refusal where `refusals/` cannot be made
    /// refuses by name.
    #[test]
    fn a_catch_up_whose_refusals_directory_cannot_be_made_is_refused() {
        use std::io::Write as _;
        let root = scratch("refusals-file");
        let journal = Journal::at(&root);
        journal.append(&run_record()).expect("lands");
        let refusals = journal.index_dir().join("refusals");
        let _ignored = std::fs::remove_dir_all(&refusals);
        std::fs::write(&refusals, b"a file").expect("a file where refusals/ goes");
        std::fs::OpenOptions::new()
            .append(true)
            .open(&journal.path)
            .expect("open")
            .write_all(&member("refused by the vendor").image())
            .expect("a raw append the index has not seen");
        let why = journal.build_index().expect_err("refused");
        assert!(why.contains("refusals: cannot create"), "{why}");
    }

    /// `ensure_index` builds an absent index once, in the background, and
    /// says so once; a second call over an index already whole says nothing;
    /// a store with no journal starts nothing.
    #[test]
    fn ensure_index_builds_once_and_says_so_once() {
        let root = scratch("ensure");
        assert!(ensure_index(&root).is_none(), "no journal, no thread");
        let path = journal_path(&root);
        std::fs::create_dir_all(path.parent().expect("audit/")).expect("mkdir");
        std::fs::write(&path, run_record().image()).expect("a journal with no index");
        let shown = path.display().to_string();
        let ours = |from| {
            crate::emitted::landed(from, "api.audit", "journal index build finished")
                .into_iter()
                .filter(|record| crate::emitted::says(record, "path", &shown))
                .count()
        };
        let from = crate::emitted::mark();
        ensure_index(&root)
            .expect("a thread")
            .join()
            .expect("the build ran");
        assert!(Journal::at(&root).index_dir().join("cover").is_file());
        assert_eq!(ours(from), 1);
        let again = crate::emitted::mark();
        ensure_index(&root)
            .expect("a thread")
            .join()
            .expect("the catch-up ran");
        assert_eq!(ours(again), 0, "a whole index is no news");
    }

    /// A journal that is there and cannot be opened refuses the index build
    /// by name, and `ensure_index` logs that at Error. Root ignores modes, so
    /// this bites for a non-root user, as the suite is meant to run.
    #[test]
    fn an_unreadable_journal_refuses_the_index_build_and_says_so() {
        use std::os::unix::fs::PermissionsExt as _;
        let root = scratch("index-unreadable");
        let path = journal_path(&root);
        std::fs::create_dir_all(path.parent().expect("audit/")).expect("mkdir");
        std::fs::write(&path, run_record().image()).expect("a journal");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000)).expect("chmod 000");
        let why = Journal::at(&root).build_index();
        let shown = path.display().to_string();
        let from = crate::emitted::mark();
        let thread = ensure_index(&root).expect("a thread");
        thread.join().expect("the build ran");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644))
            .expect("chmod back");
        let why = why.expect_err("refused");
        assert!(why.starts_with(&format!("{shown}: ")), "{why}");
        assert!(why.contains("ermission denied"), "{why}");
        let logged = crate::emitted::landed(from, "api.audit", "journal index not written")
            .into_iter()
            .filter(|record| crate::emitted::says(record, "path", &shown))
            .collect::<Vec<_>>();
        assert_eq!(logged.len(), 1, "{logged:?}");
        assert!(crate::emitted::says(&logged[0], "why", "ermission denied"));
        assert_eq!(logged[0].level, telemetry::Level::Error);
    }

    /// A range read past the journal's end is refused by name, naming the
    /// record it ended at, never filed as if the rest were there.
    #[test]
    fn a_range_past_the_journal_end_is_refused_by_name() {
        let root = scratch("range-past-end");
        let journal = Journal::at(&root);
        journal.append(&run_record()).expect("lands");
        let dir = journal.index_dir();
        let mut batch = Batch::default();
        let why = journal
            .file_range(0, 10, &mut batch, &dir)
            .expect_err("only one record is there");
        assert_eq!(
            why,
            format!(
                "{}: the file ended at record 1 of 10",
                journal.path.display()
            )
        );
        assert_eq!(
            batch.built.records, 1,
            "the one record that was there was filed"
        );
    }

    /// A run opened behind earlier records names its own opening record's
    /// ordinal, and its last record moves with each failure it appends
    /// (kills `Thread::first -> 0`).
    #[test]
    fn a_threads_first_record_is_its_own_opening_records_ordinal() {
        let root = scratch("thread-first");
        let journal = Journal::at(&root);
        journal.append(&run_record()).expect("an earlier record");
        journal.append(&run_record()).expect("another");
        let head = Record::opened(Scope::Spot, at(200), "kite", window(), 1, 7);
        let mut thread = journal.open_run(&head).expect("opened");
        assert_eq!((thread.first(), thread.last()), (2, 2));
        journal
            .append_failure(&mut thread, &member("a gap"), "a gap")
            .expect("a failure lands");
        assert_eq!((thread.first(), thread.last()), (2, 3));
    }

    /// A record AT a run record's ordinal belongs to the run before it (the
    /// run record itself names no member), and one after it to it (kills the
    /// partition's `<` turned to `<=`).
    #[test]
    fn a_record_belongs_to_the_run_record_strictly_before_it() {
        use brutex_core::vendor::Vendor;
        let index = Indexed {
            runs: vec![(0, Some(Vendor::Dhan)), (5, Some(Vendor::Groww))],
            ..Indexed::default()
        };
        assert_eq!(index.vendor_before(0), None, "nothing before the first");
        assert_eq!(index.vendor_before(5), Some(Vendor::Dhan));
        assert_eq!(index.vendor_before(6), Some(Vendor::Groww));
    }

    /// A `runs` entry of the right length and a checksum that verifies, but
    /// another mark, is not a run entry (kills the `||` turned to `&&`).
    #[test]
    fn a_run_entry_with_another_mark_is_not_read() {
        let good = run_entry(3, 90, None);
        assert_eq!(read_run_entry(&good), Some((3, 0, ATTRIBUTION_RULE)));
        let mut other = good;
        other[24..28].copy_from_slice(b"XXXX");
        let crc = crc32c(&other[..RUN_ENTRY - 4]);
        other[RUN_ENTRY - 4..].copy_from_slice(&crc.to_le_bytes());
        assert_eq!(read_run_entry(&other), None);
        assert_eq!(read_run_entry(&good[..RUN_ENTRY - 1]), None, "short");
    }

    /// A source's letters, digits, `-` and `_` name its refusal file as they
    /// are, and only every other byte is escaped (kills the `||` turned to
    /// `&&`).
    #[test]
    fn a_source_file_keeps_letters_digits_dash_and_underscore() {
        assert_eq!(source_file("kite_x-9Z"), Some("kite_x-9Z.idx".to_owned()));
        assert_eq!(source_file("a/b c"), Some("a%2Fb%20c.idx".to_owned()));
        assert_eq!(source_file(""), None);
    }

    /// An index whose cover is behind entries it holds (a crash between an
    /// entry's sync and the cover's) reads neither a run nor a fault at or
    /// past the cover (kills the runs guard `ordinal >= covered` turned to
    /// `false` and the faults filter's `<` turned to `<=`).
    #[test]
    fn entries_at_or_past_the_cover_are_not_read() {
        use std::os::unix::fs::FileExt as _;
        let root = scratch("past-cover");
        let journal = Journal::at(&root);
        for _ in 0..3 {
            journal.append(&member("a gap")).expect("a member failure");
        }
        journal.append(&run_record()).expect("a run record at 3");
        journal
            .append(&member("a gap"))
            .expect("a member failure at 4");
        std::fs::OpenOptions::new()
            .write(true)
            .open(&journal.path)
            .expect("open")
            .write_all_at(&[0xFF; RECORD_LEN], 4 * RECORD_LEN_U64)
            .expect("record 4 damaged");
        let dir = journal.index_dir();
        std::fs::remove_dir_all(&dir).expect("the index goes");
        let built = journal.build_index().expect("rebuilt over the damage");
        assert_eq!((built.runs, built.faults, built.covered), (1, 1, 5));
        let whole = journal.read_index(5).expect("reads");
        assert_eq!(whole.runs.len(), 1);
        assert_eq!(whole.fault_ordinals, vec![4]);

        write_cover(&dir, 4).expect("the cover steps back to 4");
        let at_four = journal.read_index(5).expect("reads");
        assert_eq!(at_four.covered, 4);
        assert_eq!(at_four.runs.len(), 1, "the run at 3 is under the cover");
        assert!(
            at_four.fault_ordinals.is_empty(),
            "the fault AT the cover is not read"
        );
        assert_eq!(at_four.faults, 0);

        write_cover(&dir, 3).expect("the cover steps back to 3");
        let at_three = journal.read_index(5).expect("reads");
        assert!(at_three.runs.is_empty(), "the run AT the cover is not read");
        assert_eq!(at_three.malformed, 0);
    }

    /// A build that meets another writer's lock does nothing and SAYS it was
    /// busy (kills the deleted `busy` field).
    #[test]
    fn a_build_under_another_writers_lock_says_it_was_busy() {
        let root = scratch("build-busy");
        let journal = Journal::at(&root);
        journal.append(&run_record()).expect("lands");
        let holder = Flock::try_lock(
            std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(&journal.path)
                .expect("the journal handle"),
            journal.path.clone(),
        )
        .expect("nothing else holds it");
        let built = journal.build_index().expect("a busy lock is no failure");
        drop(holder);
        assert!(built.busy);
        assert_eq!((built.records, built.covered), (0, 0), "nothing was done");
    }

    /// An index AHEAD of its journal (the journal cut back in place, same
    /// inode) is rebuilt from scratch, saying why, never trusted as it is
    /// (kills the guard `covered <= first` turned to `true`).
    #[test]
    fn an_index_ahead_of_its_journal_is_rebuilt_from_scratch() {
        let root = scratch("index-ahead");
        let journal = Journal::at(&root);
        journal.append(&member("a gap")).expect("lands");
        journal.append(&member("a gap")).expect("lands");
        journal.append(&run_record()).expect("a run at 2");
        std::fs::OpenOptions::new()
            .write(true)
            .open(&journal.path)
            .expect("open")
            .set_len(RECORD_LEN_U64)
            .expect("cut back to one record");
        let built = journal.build_index().expect("rebuilt");
        assert!(built.from_scratch);
        assert_eq!(
            built.why.as_deref(),
            Some("the index covered 3 records and the journal held 1")
        );
        assert_eq!(built.covered, 1);
        let index = journal.read_index(1).expect("reads");
        assert!(index.runs.is_empty());
    }

    /// A half-built index that cannot be cleared is refused with the CLEAR's
    /// own error, not with what a build over it would have met later (kills
    /// the guard `e.kind() != NotFound` turned to `false`).
    #[test]
    fn a_half_built_index_that_cannot_be_cleared_is_refused_by_the_clears_error() {
        let root = scratch("building-file-error");
        let journal = Journal::at(&root);
        journal.append(&run_record()).expect("lands");
        let dir = journal.index_dir();
        std::fs::remove_dir_all(&dir).expect("the index is removed");
        let building = dir.with_extension(format!("{INDEX_SUFFIX}.building"));
        std::fs::write(&building, b"a file, not a half-built index").expect("write");
        let why = journal.build_index().expect_err("refused");
        assert_eq!(
            why,
            format!("{}: Not a directory (os error 20)", building.display())
        );
    }

    /// `f` once, and again while another holder of `/dev/full`'s one inode
    /// (a parallel run of this suite) has the append lock for its microsecond.
    #[cfg(target_os = "linux")]
    fn past_a_shared_lock<T>(mut f: impl FnMut() -> Result<T, String>) -> Result<T, String> {
        for _ in 0..200 {
            match f() {
                Err(why) if why.contains("append lock") => {
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
                answered => return answered,
            }
        }
        f()
    }

    /// A JOURNAL WHOSE DEVICE IS FULL REFUSES THE RECORD BY NAME: the write
    /// fails, the truncation back fails too on a device that is not a regular
    /// file, and the append answers both (`appended`'s refused
    /// `write_rolled_back`). Linux's `/dev/full` refuses every write with
    /// ENOSPC, the failure an operator's full disk gives.
    #[cfg(target_os = "linux")]
    #[test]
    fn a_journal_on_a_full_device_refuses_the_record_and_says_the_truncation_failed() {
        let root = scratch("journal-full-device");
        let journal = Journal::at(&root);
        std::fs::create_dir_all(journal.path.parent().expect("audit/")).expect("mkdir");
        std::os::unix::fs::symlink("/dev/full", &journal.path).expect("a link to /dev/full");
        let shown = journal.path.display().to_string();
        let why = past_a_shared_lock(|| journal.append(&run_record())).expect_err("refused");
        assert_eq!(
            why,
            format!(
                "{shown}: cannot append the record — No space left on device (os error 28); \
                 truncating its partial bytes back to 0 bytes also failed: Invalid argument \
                 (os error 22); the journal may now end torn and the next append refuses it"
            )
        );
        assert!(
            !journal.notes_path().exists(),
            "a run record files no whole note"
        );
    }

    /// A WHOLE NOTE THE NOTES FILE CANNOT TAKE LEAVES THE MEMBER RECORD AS IT
    /// WAS, CUT, and is logged at Error: the record still lands, as version 2
    /// with no offset, and nothing names a note that is not there
    /// (`append_note`'s refused `write_rolled_back`, `note_whole_failed`).
    #[cfg(target_os = "linux")]
    #[test]
    fn a_whole_note_the_notes_file_cannot_take_leaves_the_member_record_cut() {
        let root = scratch("notes-full-device");
        let journal = Journal::at(&root);
        std::fs::create_dir_all(journal.path.parent().expect("audit/")).expect("mkdir");
        std::os::unix::fs::symlink("/dev/full", journal.notes_path()).expect("a link");
        let long = "the vendor refused this member over a window it says it does not \
                    serve, in words longer than the record's own note field holds";
        let cut = Record::member_failure(Scope::Spot, at(5), "NIFTY", window(), long);
        assert!(cut.note_was_cut(), "the premise: the note field cuts it");
        let from = crate::emitted::mark();
        past_a_shared_lock(|| journal.append_member(&cut, long)).expect("the record lands");
        let page = journal.page(1, 0, 1).expect("a page");
        let [entry] = page.as_slice() else {
            panic!("one entry: {page:?}");
        };
        let landed = entry.decoded.clone().expect("decodes");
        assert_eq!(
            (landed.version, landed.note_at, &entry.whole_note),
            (VERSION, None, &None),
            "{landed:?}"
        );
        assert_eq!(landed.note, cut.note, "the cut note, as it was");
        let notes = journal.notes_path().display().to_string();
        let logged = crate::emitted::landed(from, "api.audit", "whole note not appended")
            .into_iter()
            .filter(|record| crate::emitted::says(record, "path", &notes))
            .collect::<Vec<_>>();
        assert_eq!(logged.len(), 1, "{logged:?}");
        assert_eq!(logged[0].level, telemetry::Level::Error);
        assert!(crate::emitted::says(&logged[0], "instrument", "NIFTY"));
        assert!(
            crate::emitted::says(&logged[0], "why", "No space left on device"),
            "{logged:?}"
        );
    }
}

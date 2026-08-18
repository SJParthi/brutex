//! Reading a vendor's CSVs off a local directory — the transport that needs no
//! socket, no token and no rate limiter.
//!
//! # Half the vendors are not APIs
//!
//! Dhan and Groww answer HTTP. `TrueData` and GDFL sell **folders of files**.
//! A descriptor that assumed every feed was an endpoint could not express them
//! at all, so [`crate::vendor::Transport`] carries both and this module is the
//! local half. It produces exactly what an HTTP source produces —
//! [`crate::fetch::RawRow`]s — so everything downstream is transport-blind.
//!
//! # `__MACOSX` is a parsing hazard, not untidiness
//!
//! `GDFL.zip` lists 24,292 entries of which **12,145 are `__MACOSX`**: one
//! `AppleDouble` stub per real file, written by macOS when it re-zips. They
//! **end in `.csv`** and they are binary. A reader that globs by extension
//! opens all 12,145 and parses resource forks as text.
//!
//! [`crate::csv::is_ghost`] is applied here, at the walk, before anything is
//! opened. It was found by getting a *count* wrong — the counting bug and the
//! parsing bug were the same bug.
//!
//! # An honest word about the cost
//!
//! **Enumerating a directory is O(files), and that is correct here.** The O(1)
//! law in `docs/07-o1-architecture.md` is about answering a *question* — "where
//! is bar N", "how many months do I hold" — without a walk. A bulk import of
//! twelve thousand contracts genuinely has to visit twelve thousand files;
//! pretending otherwise would be the false-claim shape this repository keeps
//! catching itself in.
//!
//! What is bounded: [`MAX_MEMBERS`] caps the walk, one file is open at a time,
//! and each file's rows are decoded and handed on rather than accumulated
//! across the whole directory. Peak memory is one file, not one archive.
//!
//! Ordering the members is O(n log n) and **it happens once per walk**, at the
//! foot of [`walk`] rather than at the foot of each directory. It used to sit
//! in `descend`, which is entered once per directory and sorts the accumulator
//! the whole walk shares — O(d · n log n) where O(n log n) was wanted.
//!
//! HOW BIG `d` IS DIFFERS BY VENDOR, AND ONLY ONE OF THE TWO IS MEASURED. GDFL
//! puts its members two and three levels below the feed folder — the layout
//! recorded on `descend`, seven directories for one day. `TrueData` declares
//! `Nesting::ZipOfDailyZips` with `MemberPattern::SymbolAtRoot`, one member per
//! instrument at the archive's own root, so its `d` is smaller and nobody has
//! counted it. An earlier draft of this paragraph said the members of BOTH
//! vendors sat two and three levels down; `crate::vendor` says otherwise and is
//! the record. The factor is superlinear either way and measured on one of
//! them, which is the most this can claim under `CLAUDE.md` §3 rule 1.
//!
//! The order that comes out is the same one either way, which is why no
//! ordering test caught it and why the test that does is a call count.
//!
//! Those two are the positive bounds this section claims, and each is held
//! rather than asserted here:
//! `pull::pipeline::a_directory_past_the_member_cap_is_refused_at_the_cap` for
//! the cap — a directory one member past [`MAX_MEMBERS`] is refused at the cap
//! rather than walked to the end and then complained about — and
//! `tests::the_members_are_sorted_once_per_walk_not_once_per_directory` for the
//! ordering.

use std::fs;
use std::path::{Path, PathBuf};

use crate::csv::{self, Columns, CsvError};
use crate::fetch::RawRow;

/// The most members one walk will visit.
///
/// `docs/07-o1-architecture.md` law 5 — bound every input at the boundary. One
/// GDFL day holds 12,132 contracts, so this is roughly four such days and still
/// refuses a directory somebody pointed at their home folder.
pub const MAX_MEMBERS: usize = 50_000;

/// Why a directory did not yield rows.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ArchiveError {
    /// The directory is not there, or is not a directory.
    NotADirectory {
        /// What was pointed at.
        path: PathBuf,
    },
    /// The directory could not be listed.
    Unreadable {
        /// What was pointed at.
        path: PathBuf,
        /// The operating system's own words.
        detail: String,
    },
    /// A member could not be read.
    MemberUnreadable {
        /// Which one.
        path: PathBuf,
        /// The operating system's own words.
        detail: String,
    },
    /// A member is not valid UTF-8.
    ///
    /// Usually an `AppleDouble` stub that [`csv::is_ghost`] did not catch,
    /// which makes it worth its own variant rather than a generic parse
    /// failure: it names a *different* fix.
    MemberNotText {
        /// Which one.
        path: PathBuf,
    },
    /// A member's rows did not decode.
    MemberMalformed {
        /// Which one.
        path: PathBuf,
        /// The decoder's own refusal, which names the line.
        why: CsvError,
    },
    /// More members than [`MAX_MEMBERS`].
    TooManyMembers {
        /// How many were seen before stopping.
        members: usize,
        /// The bound.
        cap: usize,
    },
    /// A member path escapes the directory it was walked from.
    ///
    /// Refused before the file is opened. A path containing a parent-directory
    /// component is the archive-extraction attack — harmless from a paid vendor
    /// and unrecoverable if it ever is not, so the check costs nothing and the
    /// absence of it costs everything.
    PathEscapes {
        /// The offending path.
        path: PathBuf,
    },
}

impl core::fmt::Display for ArchiveError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match *self {
            Self::NotADirectory { ref path } => {
                write!(f, "{} is not a directory", path.display())
            }
            Self::Unreadable {
                ref path,
                ref detail,
            } => write!(f, "{} could not be listed: {detail}", path.display()),
            Self::MemberUnreadable {
                ref path,
                ref detail,
            } => write!(f, "{} could not be read: {detail}", path.display()),
            Self::MemberNotText { ref path } => write!(
                f,
                "{} is not text — most likely an AppleDouble stub that the \
                 ghost filter missed",
                path.display()
            ),
            Self::MemberMalformed { ref path, ref why } => {
                write!(f, "{}: {why}", path.display())
            }
            Self::TooManyMembers { members, cap } => write!(
                f,
                "the directory holds at least {members} members; the cap is {cap}"
            ),
            Self::PathEscapes { ref path } => write!(
                f,
                "{} escapes the directory it was walked from — refused before \
                 it was opened",
                path.display()
            ),
        }
    }
}

impl core::error::Error for ArchiveError {}

/// One member's worth of decoded rows, and where they came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Member {
    /// The file, so a refusal downstream can name it.
    pub path: PathBuf,
    /// The instrument, taken from the file name with its extensions removed.
    ///
    /// `NIFTY25SEP2525700PE.NFO.csv` becomes `NIFTY25SEP2525700PE`. Decomposing
    /// that into underlying, expiry, strike and type is the *vocabulary's* job,
    /// not this module's — a reader that also parsed contract grammar would be
    /// two things, and the second one would be wrong first.
    pub instrument: String,
    /// The rows, in file order.
    ///
    /// **File order is the only order there is.** These feeds are one-second
    /// snapshots with two to four rows sharing a second and no tiebreaker, so
    /// any re-sort destroys arrival order that was never written down.
    pub rows: Vec<RawRow>,
}

/// The instrument a member file names, with the strike decimal intact.
///
/// # A fixed number of strips is wrong for BOTH vendors, in opposite directions
///
/// Splitting at the FIRST dot truncated every half-strike: both
/// `ABCAPITAL31JUL25267.5CE.NFO.csv` and `...267.5PE.NFO.csv` became
/// `ABCAPITAL31JUL25267`, so a CALL and a PUT merged into one price series
/// under one name. Measured on the real GDFL folder: 11,490 files collapsed to
/// 11,259 names — 228 collision groups, 459 files.
///
/// The fix was to strip TWICE, which is right for `GDFL` and wrong for
/// `TrueData` — and it was tuned to `GDFL` because `GDFL` was the only folder
/// on hand. `GDFL` writes `<name>.NFO.csv`, two extensions; `TrueData` writes
/// `<name>.csv`, one. So the second strip, which was meant to remove `.NFO`,
/// removes the STRIKE DECIMAL instead: `BANKBARODA250626256.65CE.csv` and
/// `...65PE.csv` both become `BANKBARODA250626256`. Measured on the real
/// `TrueData` options folder: 14,214 files collapse to 13,999 names — 215
/// collision groups, and every one of them is a call merged with its own put.
///
/// That is worse than the bug it replaced, because it is silent for longer. The
/// store's append guard rejects a bar that does not follow the last one, so an
/// overlapping pair is caught — by luck, not by design. A call and a put whose
/// timestamp ranges do NOT overlap concatenate into one monotonic series and
/// nothing objects.
///
/// # The rule
///
/// Strip `.csv`, then strip a second extension ONLY if it is entirely ASCII
/// letters. An exchange suffix (`NFO`, `NSE`, `BSE`) always is; a strike decimal
/// never is, because the character after the dot is a digit. That is a property
/// of the two shapes rather than a count, so it is right for a vendor whose
/// files carry one extension and for a vendor whose files carry two, without
/// being told which is which.
///
/// A dotless name — `VEDL25JULFUT.csv`, `ATGL-III.csv` — keeps its whole stem.
fn instrument_name(path: &std::path::Path) -> String {
    let Some(stem) = path.file_stem() else {
        return String::new();
    };
    let stem = std::path::Path::new(stem);
    match (stem.file_stem(), stem.extension()) {
        // A trailing all-letters extension is an exchange suffix: drop it.
        (Some(root), Some(ext))
            if !ext.is_empty()
                && ext
                    .to_string_lossy()
                    .chars()
                    .all(|c| c.is_ascii_alphabetic()) =>
        {
            root.to_string_lossy().into_owned()
        }
        // Anything else — no dot at all, or a dot introducing a strike decimal
        // — is part of the name.
        _ => stem.to_string_lossy().into_owned(),
    }
}

/// What one walk passed over, in plain integers.
///
/// A struct rather than two adjacent `u64`: two numbers of the same type
/// transpose without a compiler complaint, and a ghost count reported as a
/// non-CSV count sends an operator to the wrong vendor.
///
/// **These are counted separately on purpose.** A `__MACOSX` stub and a stray
/// `README.txt` are both "not a bar", and they mean opposite things: 12,145 of
/// the first is a folder that was re-zipped on a Mac and is behaving exactly as
/// expected, while one of the second is a folder somebody assembled by hand.
#[derive(Debug, Clone, Copy)]
struct Passed {
    /// `__MACOSX` stubs and `AppleDouble` shadows — [`csv::is_ghost`].
    ghosts: u64,
    /// Entries that are not a `.csv` file at all: subdirectories, and anything
    /// with another extension.
    skipped: u64,
}

/// One walked folder, on the rolling log — at `Info`.
///
/// # THE EVENT THAT TELLS "IT DID NOTHING" FROM "THERE WAS NOTHING"
///
/// A walk over an empty folder and a walk over the wrong folder both returned
/// an empty vector in silence, and every symptom downstream was the same: no
/// bars, no refusal, no reason. `dir` and `members` answer it on one line.
///
/// # Why `Info`, when the members themselves are `Debug`
///
/// This fires ONCE PER FOLDER, not once per file. A one-minute backfill is
/// ~62,600 members and `crate::ingest` logs each at `Debug` for exactly that
/// reason — but there is one walk, so it can afford the level an operator sees
/// without asking. It is the run's own milestone: the moment the import knows
/// how much work it has.
///
/// # Why the ghosts are counted here after being skipped silently everywhere
/// # else
///
/// The paragraph above [`read_dir`] is about the *census*, which describes
/// bars: 12,145 `__MACOSX` entries in it would drown the numbers it exists to
/// report. A log line is the other thing. `GDFL.zip` lists 24,292 entries and
/// holds 12,133 CSVs, and the whole `is_ghost` rule was found by getting that
/// subtraction wrong — so the two halves of it are now written down where the
/// walk that performs it can be checked against them.
///
/// `rows` is the sum over members, walked once. The walk is already O(members)
/// — the module doc says so plainly — so this adds no order and answers "the
/// folder held files and every one of them was empty", which `members` alone
/// cannot.
fn note_walked(dir: &Path, members: &[Member], passed: Passed) {
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::info("pull.archive", "folder walked")
            .with("dir", telemetry::Value::Str(&dir.display().to_string()))
            .with("members", telemetry::Value::Uint(members.len() as u64))
            .with("rows", telemetry::Value::Uint(total_rows(members) as u64))
            .with("ghosts", telemetry::Value::Uint(passed.ghosts))
            .with("skipped", telemetry::Value::Uint(passed.skipped)),
    );
}

/// One folder that refused, on the rolling log — at `Error`.
///
/// A malformed member refuses the **whole walk**, so this is the end of the
/// import and not a note about one file. `members` is how many had already
/// been accepted when it stopped, which is the number that separates "the
/// eleventh file of twelve thousand is corrupt" from "the first one is" — the
/// error itself names the file and never says how far the walk got.
///
/// `why` is the [`ArchiveError`]'s own rendering. It leads with the path,
/// which is what an operator needs to open next; a 128-byte ceiling trims the
/// tail of the sentence and the encoder flags the line when it does, so a
/// shortened reason is never mistaken for the whole one.
fn note_refused(dir: &Path, members: &[Member], why: &ArchiveError) {
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::error("pull.archive", "folder refused")
            .with("dir", telemetry::Value::Str(&dir.display().to_string()))
            .with("members", telemetry::Value::Uint(members.len() as u64))
            .with("why", telemetry::Value::Str(&why.to_string())),
    );
}

/// Every real CSV directly inside `dir`, decoded.
///
/// Ghost members are skipped silently *by design* — they are not data and
/// counting them as skipped would put 12,145 entries in a census that is meant
/// to describe bars.
///
/// # Errors
///
/// Any [`ArchiveError`]. A malformed member refuses the **whole walk**: a
/// directory that yielded some of its contracts is not a smaller import, it is
/// an import nobody can characterise afterwards.
///
/// # Cost
///
/// O(members) — a bulk import visits every file, and that is inherent. One file
/// is open at a time and peak memory is one file's rows, not the directory's.
pub fn read_dir(dir: &Path, columns: Columns) -> Result<Vec<Member>, ArchiveError> {
    let mut out = Vec::new();
    let mut passed = Passed {
        ghosts: 0,
        skipped: 0,
    };
    // ONE EVENT PER FOLDER, ON EITHER OUTCOME, AND `out` IS BORROWED RATHER
    // THAN RETURNED so that a refusal can still say how many members had been
    // accepted when it stopped. The loop inside walks files and counts in two
    // plain integers; nothing in it logs.
    match walk(
        dir,
        columns,
        &mut out,
        &mut passed,
        Malformed::Refuse,
        &mut Vec::new(),
    ) {
        Ok(()) => {
            note_walked(dir, &out, passed);
            Ok(out)
        }
        Err(why) => {
            note_refused(dir, &out, &why);
            Err(why)
        }
    }
}

/// A member that would not decode against this vendor's layout, kept as a
/// finding instead of ending the walk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rejected {
    /// The file, so the operator can look at the one that is different.
    pub path: PathBuf,
    /// Why, in the decoder's own words — the field count it found and expected.
    pub why: String,
}

/// WHAT A MEMBER THAT WILL NOT DECODE DOES TO THE WALK.
///
/// The two callers want opposite things and both are right, which is why this
/// is a parameter rather than a policy baked into [`walk`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Malformed {
    /// End the walk. The INGEST reading: a folder read in part is a store
    /// written in part, and the operator asked for the folder.
    Refuse,
    /// Skip it and keep it as a finding. The CENSUS reading — see
    /// [`read_dir_reporting`].
    Collect,
}

/// The walk a CENSUS wants: one bad member is a finding, not the end.
///
/// # Why the census and the ingest disagree, and both are right
///
/// [`read_dir`] refuses a folder whose first undecodable member it meets,
/// because it is the ingest path: a folder read in part is a store written in
/// part, silently short, and nothing downstream could tell that from a folder
/// that was genuinely small.
///
/// A census is the opposite question — *what is in here* — and answering it
/// with nothing because one file is a different product is the failure this
/// route exists to end. Measured on this operator's own folder: GDFL's
/// `GFDLNFO_TICK_01072025` members decode exactly against the declared layout
/// (`Ticker,Date,Time,LTP,BuyPrice,BuyQty,SellPrice,SellQty,LTQ,OpenInterest`,
/// ten fields), and one loose `GFDLNFO_BACKADJUSTED_01072025.csv` sits beside
/// them carrying `Ticker,Date,Time,Open,High,Low,Close,Volume,Open Interest` —
/// nine fields, and an OHLC product rather than a tick one. That single file
/// made the whole folder unreadable and the page said so about the folder.
///
/// **ONLY [`ArchiveError::MemberMalformed`] IS COLLECTED.** Every other refusal
/// still ends the walk, and the distinction is deliberate: a wrong field count
/// is *a different product in the folder*, which is a fact about what was
/// bought. A path that escapes its directory, a member that is not text, an
/// unreadable directory and a member cap are faults, and a census that swallowed
/// those would be the `CLAUDE.md` §4 fallback.
///
/// # Errors
///
/// Every [`read_dir`] error except [`ArchiveError::MemberMalformed`].
pub fn read_dir_reporting(
    dir: &Path,
    columns: Columns,
) -> Result<(Vec<Member>, Vec<Rejected>), ArchiveError> {
    let mut out = Vec::new();
    let mut rejected = Vec::new();
    let mut passed = Passed {
        ghosts: 0,
        skipped: 0,
    };
    match walk(
        dir,
        columns,
        &mut out,
        &mut passed,
        Malformed::Collect,
        &mut rejected,
    ) {
        Ok(()) => {
            note_walked(dir, &out, passed);
            Ok((out, rejected))
        }
        Err(why) => {
            note_refused(dir, &out, &why);
            Err(why)
        }
    }
}

/// [`read_dir`]'s walk, split out for one reason: the two `note_*` helpers
/// must report what it found whether it finished or refused, and a `?` on the
/// way past cannot do that.
///
/// # Errors
///
/// Whatever [`read_dir`] documents; this is its body.
/// HOW DEEP THE WALK DESCENDS, and it is a bound rather than a guess.
///
/// `docs/07-o1-architecture.md` law 5 — bound every input at the boundary. A
/// recursive walk with no depth bound is a walk that follows a symlink loop
/// forever, and BOTH feed folders on this operator's machine are symlinks, so
/// that is a live hazard rather than a theoretical one.
///
/// Four, measured. The deepest real member sits three below the feed folder —
/// `gdfl/GFDLNFO_TICK_01072025/Futures/-I/AARTIIND-I.NFO.csv` — and four leaves
/// exactly one level of headroom for a vendor that adds a wrapper. A fifth
/// would be room for a mistake rather than for a vendor.
///
/// Deeper members are SKIPPED and counted, never silently dropped:
/// [`Passed::skipped`] is on the walk's own log line.
pub const MAX_DEPTH: usize = 4;

fn walk(
    dir: &Path,
    columns: Columns,
    out: &mut Vec<Member>,
    passed: &mut Passed,
    on_malformed: Malformed,
    rejected: &mut Vec<Rejected>,
) -> Result<(), ArchiveError> {
    descend(dir, columns, out, passed, on_malformed, rejected, 0)?;
    // ═══ ONE ORDERING PER WALK, NOT ONE PER DIRECTORY ═══
    //
    // THE FINAL ORDER IS THE SAME ONE, and that is stated first because it is a
    // reader's first worry. This line used to sit at the foot of `descend`, so
    // the outermost call ordered the whole accumulator *after* every nested
    // call had returned — its result was already "every member, by path". The
    // inner sorts only re-ordered a prefix that the outer one was about to
    // order again, and nothing between them reads a position: the loop only
    // pushes, and the `MAX_MEMBERS` guard reads `out.len()`. `sort_by` is
    // stable, so even two members carrying the same path — which a filesystem
    // cannot yield, though the type permits it — come out in the same relative
    // order either way. CLAUDE.md §3 rule 5, same inputs same outputs byte for
    // byte, is untouched by this move.
    //
    // WHAT IT COSTS INSTEAD. With `d` directories the old placement paid
    // O(d · n log n) over the accumulator the whole walk shares, not over the
    // directory's own members. Counted off the layout recorded on `descend`
    // above — the feed folder, `GFDLNFO_TICK_01072025`, `Options`, `Futures`
    // and its `-I`/`-II`/`-III` month folders — that is seven directories over
    // one day's 12,132 contracts, so seven orderings where one was needed.
    //
    // HOW LARGE EACH OF THE SEVEN WAS IS A RANGE, NOT A NUMBER. Each sorted the
    // accumulator as it stood when that directory finished, so the sizes turn
    // on the order `fs::read_dir` handed back `Options` and `Futures` — the
    // very order this sort exists because nobody can predict. At least three of
    // the seven are over the full 12,132 whichever way it falls, because the
    // last group folder, the stem above it and the feed folder all finish after
    // the final push; the rest are smaller by an amount nobody has measured. An
    // earlier draft of this comment said all seven were twelve-thousand-element
    // sorts, which was one measurement more than anybody took. SEVEN IS THE
    // NUMBER THAT IS CERTAIN, and seven-to-one is the whole claim.
    //
    // WHAT IT DOES NOT FIX: the walk is still O(members). It opens every file
    // and that is inherent — the module doc says so plainly. This removes a
    // superlinear factor from the ORDERING alone.
    //
    // ON A REFUSAL NOTHING IS ORDERED NOW, where before each directory that had
    // completed had ordered a prefix before a later one refused. That is
    // unobservable rather than merely unlikely: the `?` above hands the error
    // to `read_dir` and `read_dir_reporting`, both of which drop `out` and pass
    // it only to `note_refused`, which reads its length. Held by
    // `tests::a_malformed_member_below_the_root_still_refuses_the_whole_walk`,
    // because "unobservable" is a claim.
    sort_members(out);
    Ok(())
}

/// One directory level of [`walk`], and its own subdirectories under
/// [`MAX_DEPTH`].
///
/// `depth` is the level BELOW the folder the walk started at: the feed folder
/// itself is 0, so a member at `GFDLNFO_TICK_01072025/Options/x.csv` is reached
/// with `depth == 2`.
#[allow(
    clippy::too_many_arguments,
    reason = "six of the seven are the walk's own state — the accumulator, the \
              two counters, the malformed policy, the rejects and the depth — \
              and bundling them into a struct would put a mutable borrow of the \
              accumulator inside the same value as the thing being pushed to it. \
              The recursion is what makes them parameters rather than locals."
)]
fn descend(
    dir: &Path,
    columns: Columns,
    out: &mut Vec<Member>,
    passed: &mut Passed,
    on_malformed: Malformed,
    rejected: &mut Vec<Rejected>,
    depth: usize,
) -> Result<(), ArchiveError> {
    if depth >= MAX_DEPTH {
        // COUNTED, NOT DROPPED. The walk's log line carries `skipped`, so a
        // folder nested past the bound says so rather than reading as empty.
        passed.skipped = passed.skipped.saturating_add(1);
        return Ok(());
    }
    if !dir.is_dir() {
        return Err(ArchiveError::NotADirectory {
            path: dir.to_path_buf(),
        });
    }
    let entries = fs::read_dir(dir).map_err(|e| ArchiveError::Unreadable {
        path: dir.to_path_buf(),
        detail: e.to_string(),
    })?;

    for entry in entries {
        let entry = entry.map_err(|e| ArchiveError::Unreadable {
            path: dir.to_path_buf(),
            detail: e.to_string(),
        })?;
        let path = entry.path();

        // THE GHOST FILTER, before anything is opened. Counted in a plain
        // integer rather than logged: `GDFL.zip` holds 12,145 of these, and one
        // line each would bury the walk's own result in its own noise.
        let name = path.to_string_lossy();
        if csv::is_ghost(&name) {
            passed.ghosts = passed.ghosts.saturating_add(1);
            continue;
        }
        // ═══ A SUBFOLDER IS DESCENDED INTO, NOT SKIPPED ═══
        //
        // This read `!path.is_file() || extension != csv` and counted a
        // directory as `skipped`, so the walk saw exactly one level and these
        // vendors do not ship one level.
        //
        // MEASURED ON THE OPERATOR'S DISK, 14 Aug 2026:
        //
        //   vendor-data/gdfl/                              0 csv files here
        //   └── GFDLNFO_TICK_01072025/
        //       ├── Options/                          11,491 csv files
        //       └── Futures/{-I,-II,-III}/               the near, mid and far
        //                                                 month, one level lower
        //
        // So every member was two or three levels down and the walk found NONE
        // of them. `read_census` answered `Empty` for a folder holding eleven
        // and a half thousand contracts, which is the exact shape of a precise
        // and wrong answer.
        //
        // THE DESCRIPTOR ALREADY DESCRIBED THIS. GDFL declares
        // `Nesting::ZipOfSegmentFolders` and `groups: [(Options, "Options"),
        // (Futures, "Futures")]`. The structure was stated in `crate::vendor`
        // and this walker never read it — which is why the fix is a descent and
        // not a question for anybody.
        //
        // DEPTH-FIRST, IN DIRECTORY ORDER, and the ordering does not matter
        // because `sort_members` orders the accumulator by path once the whole
        // recursion has returned — in `walk`, for exactly this reason.
        if path.is_dir() {
            descend(
                &path,
                columns,
                out,
                passed,
                on_malformed,
                rejected,
                depth + 1,
            )?;
            continue;
        }
        if path.extension().is_none_or(|e| e != "csv") {
            passed.skipped = passed.skipped.saturating_add(1);
            continue;
        }
        // A member must stay under the directory it was walked from. `..` in a
        // name is the extraction attack; cheap to refuse, unrecoverable if not.
        if path
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            return Err(ArchiveError::PathEscapes { path });
        }
        if out.len() >= MAX_MEMBERS {
            return Err(ArchiveError::TooManyMembers {
                members: out.len(),
                cap: MAX_MEMBERS,
            });
        }

        let bytes = fs::read(&path).map_err(|e| ArchiveError::MemberUnreadable {
            path: path.clone(),
            detail: e.to_string(),
        })?;
        let text = String::from_utf8(bytes)
            .map_err(|_| ArchiveError::MemberNotText { path: path.clone() })?;
        // THE ONE REFUSAL THE CENSUS TURNS INTO A FINDING. `Refuse` is
        // byte-for-byte what this line did before the parameter existed.
        let rows = match csv::decode(&text, columns) {
            Ok(rows) => rows,
            Err(why) => match on_malformed {
                Malformed::Refuse => {
                    return Err(ArchiveError::MemberMalformed { path, why });
                }
                Malformed::Collect => {
                    rejected.push(Rejected {
                        why: why.to_string(),
                        path,
                    });
                    continue;
                }
            },
        };

        let instrument = instrument_name(&path);

        out.push(Member {
            path,
            instrument,
            rows,
        });
    }

    // NO ORDERING HERE, DELIBERATELY. The members do have to be ordered by path
    // — `fs::read_dir` yields in filesystem order and CLAUDE.md §3 rule 5 wants
    // the same vector twice — but this function is entered once per directory,
    // so ordering here ordered the whole walk's accumulator once per directory.
    // It happens exactly once now, in `walk`, after this recursion unwinds.
    Ok(())
}

/// Order a walk's members by path — the ONE place it happens, and the one
/// place a test can count.
///
/// [`fs::read_dir`] yields in filesystem order. That is not a stable order: it
/// differs between machines and between runs, so two operators pointed at the
/// same folder get two different vectors and neither is wrong. Sorting by path
/// makes an import reproducible — CLAUDE.md §3 rule 5, same inputs same outputs
/// byte for byte.
///
/// This orders the MEMBERS, never the rows inside one. These feeds are
/// snapshots with several rows sharing a second and no tiebreaker, so a re-sort
/// of the rows would destroy arrival order that was never written down;
/// [`Member::rows`] says the same thing where the field is declared.
///
/// It takes `&mut [Member]` rather than `&mut Vec<Member>` because it never
/// changes the length, which is the only thing a caller could get wrong here.
fn sort_members(out: &mut [Member]) {
    // HOW OFTEN THIS RUNS IS THE THING THAT WAS WRONG, and it cannot be seen in
    // the answer: an ordering performed once and an ordering performed seven
    // times return the identical vector. So the test holds a COUNT, and this is
    // where the count comes from — the same device
    // `crates/greeks/src/bsm.rs::MODEL_EVALUATIONS` uses, for the same reason.
    //
    // IT DOES NOT EXIST OUTSIDE `cargo test`. `#[cfg(test)]` on a statement
    // removes the statement, so a release walk pays nothing at all for it.
    #[cfg(test)]
    SORTS.with(|n| n.set(n.get().saturating_add(1)));
    out.sort_by(|a, b| a.path.cmp(&b.path));
}

#[cfg(test)]
thread_local! {
    /// How many times [`sort_members`] has run on THIS thread.
    ///
    /// Exists so that
    /// `tests::the_members_are_sorted_once_per_walk_not_once_per_directory`
    /// can hold a number the walker does not report. A test that only reads the
    /// vector cannot see the vector being ordered seven times over, which is
    /// exactly how a per-directory sort survived every ordering test this crate
    /// already had.
    ///
    /// Thread-local because `cargo test` runs tests in parallel and a shared
    /// counter would measure other tests' walks. The reader resets it rather
    /// than assuming zero, so it also holds under `--test-threads=1`, where
    /// every test shares one thread.
    static SORTS: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

/// How many rows a walk produced, across every member.
#[must_use]
pub fn total_rows(members: &[Member]) -> usize {
    members.iter().map(|m| m.rows.len()).sum()
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "a test that cannot panic cannot fail, and these lints exist to \
              keep panics out of the crate rather than out of its tests"
)]
mod tests {
    use super::{ArchiveError, SORTS, read_dir};
    use crate::csv::Columns;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU32, Ordering};

    /// One `TrueData` index row — `YYYYMMDD,HH:MM:SS,price,volume,open_interest`,
    /// five fields and no header. Copied from `tests/folder.rs` rather than
    /// shared: an integration test's helpers are not visible from a unit test
    /// module, and this crate has no dev-dependency to put them in.
    const ONE_ROW: &str = "20221003,09:15:01,38445.65,0,0\n";

    /// Scratch roots must not collide between tests running at the same time.
    static NEXT: AtomicU32 = AtomicU32::new(0);

    /// A temporary directory that removes itself, mirroring `Scratch` in
    /// `tests/folder.rs`.
    ///
    /// Hand-rolled because `crates/pull/Cargo.toml` declares no
    /// dev-dependencies and states why, so there is no `tempfile` to reach for.
    /// The drop is best-effort: a scratch directory that outlives a crashed
    /// test is litter, and failing a test on litter reports the wrong thing.
    struct Scratch {
        root: PathBuf,
    }

    impl Scratch {
        fn new() -> Self {
            let serial = NEXT.fetch_add(1, Ordering::Relaxed);
            let mut root = std::env::temp_dir();
            root.push(format!(
                "brutex-archive-sort-{}-{serial}",
                std::process::id()
            ));
            fs::create_dir_all(&root).expect("a scratch root");
            Self { root }
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _best_effort = fs::remove_dir_all(&self.root);
        }
    }

    /// THE FIXTURE, and its shape carries the whole argument.
    ///
    /// ```text
    /// feed/ALPHA.csv
    /// feed/Futures/C.csv
    /// feed/Futures/D.csv
    /// feed/Options/A.csv
    /// feed/Options/B.csv
    /// feed/ZULU.csv
    /// ```
    ///
    /// Three directories, which is what makes the sort count of 1 different
    /// from the sort count of 3 the old placement produced.
    ///
    /// And the two members that live in the ROOT sort on either side of the
    /// four below it: as path components `ALPHA.csv` < `Futures` < `Options` <
    /// `ZULU.csv`. Any scheme that ordered each level and appended the levels
    /// would put `ALPHA.csv` and `ZULU.csv` next to each other; only an
    /// ordering over the whole accumulator interleaves them. That is the
    /// property the ordering test holds, and it is the reason the fixture is
    /// not simply two flat directories.
    ///
    /// The names are written in the wrong order on purpose, so the fixture does
    /// not lean on whatever order the filesystem hands back.
    fn two_levels(scratch: &Scratch) -> PathBuf {
        let root = scratch.root.join("feed");
        let options = root.join("Options");
        let futures = root.join("Futures");
        fs::create_dir_all(&options).expect("Options");
        fs::create_dir_all(&futures).expect("Futures");
        for (dir, names) in [
            (&root, ["ZULU", "ALPHA"]),
            (&options, ["B", "A"]),
            (&futures, ["D", "C"]),
        ] {
            for name in names {
                fs::write(dir.join(format!("{name}.csv")), ONE_ROW).expect("a member");
            }
        }
        root
    }

    /// THE ORDER IS OVER THE WHOLE WALK, NOT OVER ONE DIRECTORY.
    ///
    /// This is the behaviour-preserving half of moving the sort out of
    /// `descend`: the vector this pins is the vector the per-directory
    /// placement produced, byte for byte, because the outermost `descend`
    /// ordered everything last anyway. CLAUDE.md §3 rule 5 is a claim about
    /// THIS vector, so it is written down rather than described.
    ///
    /// It asserts the interleaving specifically — the root's own two members
    /// land at the ends with the nested four between them — because a walk that
    /// ordered each level separately would still return something sorted-looking
    /// and would fail here.
    #[test]
    fn the_walk_orders_members_by_path_across_every_level() {
        let scratch = Scratch::new();
        let root = two_levels(&scratch);

        let members = read_dir(&root, Columns::TrueDataIndex).expect("a two-level folder walks");

        let paths: Vec<PathBuf> = members.iter().map(|m| m.path.clone()).collect();
        assert_eq!(
            paths,
            vec![
                root.join("ALPHA.csv"),
                root.join("Futures").join("C.csv"),
                root.join("Futures").join("D.csv"),
                root.join("Options").join("A.csv"),
                root.join("Options").join("B.csv"),
                root.join("ZULU.csv"),
            ],
            "the root's own members belong at either end, which only one \
             ordering over the whole accumulator produces"
        );
    }

    /// THE ACCUMULATOR IS ORDERED ONCE PER WALK, NOT ONCE PER DIRECTORY.
    ///
    /// The defect this test exists for could not be seen in the answer. The
    /// sort sat at the foot of `descend`, which is entered once per directory,
    /// so a walk over `d` directories sorted the whole shared accumulator `d`
    /// times — O(d · n log n) where O(n log n) was needed — and returned the
    /// identical vector every time. Every ordering test this crate had passed
    /// throughout. So the assertion here is a COUNT, not a shape.
    ///
    /// Three directories in the fixture, so this read 3 before the sort moved
    /// into `walk` and reads 1 after. The counter is reset here rather than
    /// assumed zero, which is what makes it hold under `--test-threads=1` as
    /// well, where every test shares one thread.
    #[test]
    fn the_members_are_sorted_once_per_walk_not_once_per_directory() {
        let scratch = Scratch::new();
        let root = two_levels(&scratch);

        SORTS.with(|n| n.set(0));
        let members = read_dir(&root, Columns::TrueDataIndex).expect("a two-level folder walks");
        let sorts = SORTS.with(std::cell::Cell::get);

        assert_eq!(
            members.len(),
            6,
            "the fixture itself, so a walk that found nothing cannot pass by \
             sorting nothing"
        );
        assert_eq!(
            sorts, 1,
            "one ordering for the walk; 3 — one per directory — is the \
             superlinear shape this moved out of `descend`"
        );
    }

    /// A REFUSAL BELOW THE ROOT STILL REFUSES THE WHOLE WALK, AND STILL NAMES
    /// THE MEMBER.
    ///
    /// Moving the sort also removed the partial orderings the completed
    /// directories used to perform before a later one refused. The comment in
    /// `walk` argues that is unobservable — `read_dir` drops the accumulator on
    /// `Err` and `note_refused` reads only its length — and an argument is not
    /// a check, so the refusal itself is held here. The happy path above is the
    /// other half: this pair is what says the move changed neither outcome.
    #[test]
    fn a_malformed_member_below_the_root_still_refuses_the_whole_walk() {
        let scratch = Scratch::new();
        let root = two_levels(&scratch);
        let odd = root.join("Options").join("WRONG.csv");
        fs::write(&odd, "20221003,09:15:01,38445.65\n").expect("three fields, not five");

        let why = read_dir(&root, Columns::TrueDataIndex).expect_err("three fields is not five");

        assert!(
            matches!(why, ArchiveError::MemberMalformed { ref path, .. } if *path == odd),
            "the refusal must name the member a level down, not the folder: {why}"
        );
    }
}

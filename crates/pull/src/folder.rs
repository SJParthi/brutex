//! A folder vendor: where its folder is, how far it reaches, and the words for
//! a source nobody pulls from.
//!
//! # The operator's rule, 12 Aug 2026, verbatim
//!
//! > "for truedata and gdfl alone, one and only, we will pull the data
//! > especially entirely from csv files from the precise folder — because we
//! > will buy those data from them as csv files and we will put that into the
//! > specified folder, only from there it should be read."
//! >
//! > "except these two alone only, for all other vendors or brokers feeds it
//! > should be always REST."
//!
//! [`crate::vendor::SourceKind`] is that rule as a type. This module is the
//! half of it that touches a disk.
//!
//! # What a folder does NOT have, and each absence is a behaviour
//!
//! **No history floor.** A REST vendor publishes how far back it answers and
//! [`crate::vendor::HistoryFloor`] records the claim with its source. Nobody
//! publishes anything about a directory on this machine. So the reach of a
//! folder feed is [`Reach`], and it is READ — an empty folder has an empty
//! reach and says so, and there is no vendor constant anywhere for it to fall
//! back to. The descriptor's `history` table is empty for both folder rows
//! precisely so that there is nothing to fall back TO.
//!
//! **No token.** `CLAUDE.md` §8's credential machinery does not run for a
//! folder — see `crate::config`, which requires a credential table only of
//! feeds whose kind is [`crate::vendor::SourceKind::Rest`]. A missing
//! credential must not block a source that has nothing to authenticate
//! against.
//!
//! **No quota, and nothing to rate-limit.** There is no request, so there is
//! no 429 and no budget to spend.
//!
//! **No pull.** The verb is [`crate::vendor::SourceKind::verb`], which is
//! `read` on this side. Calling it a pull put every failure in the wrong
//! diagnostic frame: the first questions asked were about tokens,
//! entitlements and outages, and the answer was always a path.
//!
//! # A missing folder is a halt, not an empty answer
//!
//! `CLAUDE.md` §4 bans a fallback that hides a failure, and the failure this
//! module exists to make loud is the quiet one: a folder that is not there and
//! a folder that is there and empty both used to produce no bars and no
//! reason. [`read_reach`] refuses the first BY PATH and answers [`Reach::Empty`]
//! for the second, and both leave a line in the log.
//!
//! # Cost
//!
//! Resolving the folder is string joining and no filesystem call at all.
//! [`reach_of`] is one pass over an already-decoded walk. [`read_reach`]
//! performs that walk, which is O(members) — a bulk import genuinely visits
//! every file, `crate::archive` says so plainly, and this claims no better.
//! `pull::folder::the_reach_of_a_walk_is_one_pass_over_its_rows` is the pass;
//! the walk's own bound is [`crate::archive::MAX_MEMBERS`].

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::archive::{self, ArchiveError, Member};
use crate::csv::Columns;
use crate::session::{Day, IstMoment, Window};
use crate::vendor::Feed;

/// The variable that names the folder root, when the operator sets one.
///
/// Spelled exactly as `BRUTEX_MASTERS` and `BRUTEX_STORE` are, and read in
/// exactly one place — [`root`] — so every function below takes a directory as
/// an argument and nothing else has to touch process-wide state to be
/// deterministic. It could not anyway: `set_var` is `unsafe` under edition
/// 2024 and this crate carries `#![forbid(unsafe_code)]`.
pub const ROOT_ENV: &str = "BRUTEX_ARCHIVES";

/// The folder root's directory name under the operator's home.
///
/// `~/.brutex/vendor-data`, which is where `docs/08-vendor-samples.md` records
/// the purchased archives already living — beside `masters/` and `store/` in
/// the tree this project already owns, rather than in a downloads folder one
/// cleanup would empty.
pub const ROOT_DIR: &str = "vendor-data";

/// Why a folder could not be located, read, or asked about.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum FolderError {
    /// Neither [`ROOT_ENV`] nor `HOME` says where the folder root is.
    ///
    /// # Why this refuses where the store and the masters fall back
    ///
    /// `api::server::store_dir_from` and `masters_dir_from` fall back to `.`
    /// when `HOME` is unset, and that is right for them: a missing master
    /// renders `UNAVAILABLE` and a missing store is created on write, so the
    /// working directory is a wrong answer that announces itself immediately.
    ///
    /// A folder feed's whole reach IS the folder. Pointed at the process's
    /// working directory it would report an honest, precise and completely
    /// wrong reach — most often `Empty`, which is indistinguishable from "the
    /// operator has not bought that month yet". That is the silent fallback
    /// `CLAUDE.md` §4 bans, so this arm refuses instead, and the deviation
    /// from the two siblings is named here rather than left to be discovered.
    NoHome,
    /// The feed is REST. It has no folder at all.
    ///
    /// Refused rather than answered with a path, because a path invented for a
    /// broker would come back as "that folder is missing" — the same words as
    /// a real missing folder, for a completely different reason.
    NotAFolderFeed {
        /// Which feed was asked.
        feed: Feed,
    },
    /// The folder is missing, unreadable, or holds something that will not
    /// decode. Carries the walker's own refusal, which names the path.
    Walk {
        /// The folder that was named.
        path: PathBuf,
        /// Why, in [`crate::archive`]'s words.
        why: ArchiveError,
    },
    /// A row carries a timestamp that is not a moment on this calendar.
    ///
    /// A column read at the wrong offset looks exactly like this, so it
    /// refuses the whole reach rather than skipping the row: a reach computed
    /// from the rows that happened to parse is a narrower answer wearing the
    /// same words as a complete one.
    Untimed {
        /// Which member.
        path: PathBuf,
        /// The value that is not a moment.
        secs: i64,
    },
}

impl core::fmt::Display for FolderError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match *self {
            Self::NoHome => write!(
                f,
                "neither {ROOT_ENV} nor HOME says where the bought files are, \
                 and there is no default folder — a folder feed's whole reach \
                 is its folder, so a guess here would report a precise and \
                 wrong answer"
            ),
            Self::NotAFolderFeed { feed } => write!(
                f,
                "{feed} is a {}, so it has no folder — its bars are {} over the \
                 network",
                feed.source_kind().label(),
                feed.source_kind().verb_past()
            ),
            Self::Walk { ref path, ref why } => {
                write!(f, "{}: {why}", path.display())
            }
            Self::Untimed { ref path, secs } => write!(
                f,
                "{}: {secs} is not a moment on this calendar — a column read \
                 at the wrong offset looks exactly like this",
                path.display()
            ),
        }
    }
}

impl core::error::Error for FolderError {}

/// The folder root, from the environment.
///
/// [`ROOT_ENV`], else `$HOME/.brutex/vendor-data`. The only place the
/// environment is consulted.
///
/// # Errors
///
/// [`FolderError::NoHome`] when neither is set. There is no default.
pub fn root() -> Result<PathBuf, FolderError> {
    root_from(std::env::var_os(ROOT_ENV), std::env::var_os(HOME_ENV))
}

/// The variable the home directory is read from.
///
/// A constant so [`root`] and its doc cannot drift apart, and upper-case, so
/// it is not the segment-shaped literal CI gate 1d scans for.
const HOME_ENV: &str = "HOME";

/// The root implied by values of [`ROOT_ENV`] and `HOME`.
///
/// Split from [`root`] for the reason `api::server::store_dir_from` is split
/// from `store_dir`: every outcome has to be testable, and a test cannot set
/// either variable — `set_var` is `unsafe` under edition 2024, this crate
/// forbids `unsafe`, and mutating process-wide state would race every other
/// test in the binary.
///
/// # Errors
///
/// [`FolderError::NoHome`] when both are absent.
fn root_from(value: Option<OsString>, home: Option<OsString>) -> Result<PathBuf, FolderError> {
    if let Some(named) = value {
        return Ok(PathBuf::from(named));
    }
    let Some(home) = home else {
        return Err(FolderError::NoHome);
    };
    Ok(PathBuf::from(home)
        .join(crate::config::CONFIG_DIR)
        .join(ROOT_DIR))
}

/// Where one folder feed's bought files live under `root`.
///
/// One subdirectory per feed, named by [`Feed::wire`] — the same shape
/// `api::server::master_paths` uses to place one master per vendor under the
/// masters root, and derived from the feed registry for the same reason:
/// a hand-written list is a list somebody forgets to extend.
///
/// # Errors
///
/// [`FolderError::NotAFolderFeed`] for a REST feed.
pub fn folder_of(root: &Path, feed: Feed) -> Result<PathBuf, FolderError> {
    if !matches!(feed.source_kind(), crate::vendor::SourceKind::Folder) {
        return Err(FolderError::NotAFolderFeed { feed });
    }
    Ok(root.join(feed.wire()))
}

/// HOW FAR A FOLDER FEED REACHES — read off the disk, never declared.
///
/// # Three arms, and two of them are not `nothing`
///
/// A folder with no members, a folder of empty members and a folder of real
/// days all used to be the same silence downstream: no bars, no refusal, no
/// reason. They are three different things an operator does three different
/// things about — buy the month, re-download the files, or nothing at all —
/// so they are three arms here.
///
/// There is deliberately no fourth arm meaning *unknown*. The folder is on
/// this machine; whatever it holds is knowable by looking, and a reach that
/// could say `unknown` is one a caller would have to treat as `unbounded`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reach {
    /// The folder is there and holds no member this feed reads.
    ///
    /// An ANSWER, not a missing one: the reach is empty, and the operator has
    /// not put the files there yet.
    Empty,
    /// Members are present and not one of them carries a row.
    ///
    /// Different from [`Self::Empty`] by exactly the fact that matters: the
    /// files were bought and delivered, and they are blank.
    Blank {
        /// How many members.
        files: usize,
    },
    /// The days this folder answers for, both ends inclusive.
    Days {
        /// The earliest day any row lands on.
        earliest: Day,
        /// The latest day any row lands on.
        latest: Day,
        /// How many members contributed.
        files: usize,
        /// How many rows were read.
        rows: usize,
    },
}

impl Reach {
    /// Whether this folder answers for no day at all.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        !matches!(self, Self::Days { .. })
    }

    /// How many members the folder held.
    #[must_use]
    pub const fn files(self) -> usize {
        match self {
            Self::Empty => 0,
            Self::Blank { files } | Self::Days { files, .. } => files,
        }
    }

    /// How many rows those members held.
    #[must_use]
    pub const fn rows(self) -> usize {
        match self {
            Self::Empty | Self::Blank { .. } => 0,
            Self::Days { rows, .. } => rows,
        }
    }

    /// The day window this folder answers, or `None` when it answers none.
    ///
    /// `None` is the honest rendering of an empty reach, and a caller must
    /// treat it as "this folder answers for nothing" rather than as an absent
    /// bound — the two read the same in a JSON field and mean opposite things,
    /// which is the distinction [`crate::vendor::HistoryFloor`] makes on the
    /// REST side.
    #[must_use]
    pub fn window(self) -> Option<Window> {
        match self {
            Self::Empty | Self::Blank { .. } => None,
            Self::Days {
                earliest, latest, ..
            } => Window::new(earliest, latest).ok(),
        }
    }
}

/// WHAT A FOLDER HOLDS: how far it reaches, and **what it names**.
///
/// # Why the names had to become reachable
///
/// A folder feed publishes no instrument master. `Vendor::MASTERED` is the
/// three REST feeds and nothing else, so the ISIN-keyed join in `crates/api`
/// answers `lacks` for every name an archive is asked about — correctly, and
/// about a master that does not exist rather than about the folder that does.
///
/// The file name is therefore **the only identity an archive has**. There is no
/// ISIN in these files, no security id, and no ticker column: `Member` takes the
/// instrument off the file name because that is where it lives. [`reach_of`]
/// walked every one of them, counted them, and threw the names away — so the
/// one fact that answers "what is in this feed" was read on every walk and
/// reachable by nobody. `docs/05-decisions.md` D-0141.
///
/// # Why this is not a field on [`Reach`]
///
/// [`Reach`] is `Copy`, and [`Reach::files`], [`Reach::rows`] and
/// [`Reach::is_empty`] are `const fn` taking `self` **by value**. A `Vec` ends
/// all four of those at once. The reach is a bound and the census is a set;
/// keeping them apart lets a caller that wants only the bound keep paying only
/// for the bound.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Census {
    /// How far the folder reaches — exactly what [`reach_of`] answers.
    pub reach: Reach,
    /// Every DISTINCT instrument the folder names, sorted.
    ///
    /// Sorted rather than left in walk order because `read_dir` order is the
    /// filesystem's and is not stable between machines or between runs, and
    /// `CLAUDE.md` §3 rule 5 makes the same input owe the same output. File
    /// order is load-bearing for ROWS inside a member — it is the only order a
    /// one-second feed has — and carries nothing at all across members.
    pub instruments: Vec<String>,
    /// How many members named an instrument some other member had already
    /// named.
    ///
    /// **Reported rather than swallowed.** Deduplicating in silence is the
    /// §4 shape: two members claiming one instrument is exactly the `ambiguous`
    /// bucket D-0141 describes on the folder side — a GDFL stem that appears
    /// under both `Options/` and `Futures/`, say — and a caller that sees only
    /// the deduplicated list cannot tell a clean folder from a colliding one.
    /// Zero on every folder where each file names its own instrument.
    pub collisions: usize,
}

/// The census of an already-decoded walk.
///
/// One pass for the reach, then one pass to collect the names, then a sort. The
/// sort is `O(members log members)` and is the only super-linear step in the
/// walk; it is taken once per folder read, never per file or per row, and
/// `docs/06-limits.md` records the walk's cost.
///
/// # Errors
///
/// Whatever [`reach_of`] refuses, unchanged — a census of a folder whose days
/// cannot be read is not a smaller census, it is no answer at all.
pub fn census_of(members: &[Member]) -> Result<Census, FolderError> {
    let reach = reach_of(members)?;
    let mut instruments: Vec<String> = members
        .iter()
        .map(|member| member.instrument.clone())
        .collect();
    instruments.sort_unstable();
    let before = instruments.len();
    instruments.dedup();
    Ok(Census {
        collisions: before - instruments.len(),
        reach,
        instruments,
    })
}

/// The reach of an already-decoded walk.
///
/// One pass over the rows, comparing two [`Day`]s and keeping the extremes.
/// Nothing is sorted and nothing is collected: the rows stay in file order,
/// which for a one-second feed is the only order there is.
///
/// # Errors
///
/// [`FolderError::Untimed`] naming the member and the value, when a row's
/// timestamp is not a moment on this calendar.
pub fn reach_of(members: &[Member]) -> Result<Reach, FolderError> {
    if members.is_empty() {
        return Ok(Reach::Empty);
    }
    let mut span: Option<(Day, Day)> = None;
    let mut rows = 0usize;
    for member in members {
        for row in &member.rows {
            rows += 1;
            let Ok(at) = IstMoment::from_epoch_secs(row.timestamp) else {
                return Err(FolderError::Untimed {
                    path: member.path.clone(),
                    secs: row.timestamp,
                });
            };
            let day = at.day();
            span = Some(match span {
                None => (day, day),
                Some((earliest, latest)) => (earliest.min(day), latest.max(day)),
            });
        }
    }
    let Some((earliest, latest)) = span else {
        return Ok(Reach::Blank {
            files: members.len(),
        });
    };
    Ok(Reach::Days {
        earliest,
        latest,
        files: members.len(),
        rows,
    })
}

/// One folder that answered, on the rolling log — at `Info`.
///
/// The line that separates "there is nothing there" from "it did nothing". It
/// fires once per folder, never per file, so it can afford the level an
/// operator sees without asking — the same argument
/// [`crate::archive`]'s walk event makes.
///
/// `verb` is on the line deliberately. A log read after the fact is where the
/// wrong diagnostic frame does its damage, and the word `read` beside the path
/// is what stops the next hour being spent on a token.
fn note_read(dir: &Path, feed: Feed, reach: Reach) {
    let ends = match reach {
        Reach::Days {
            earliest, latest, ..
        } => (earliest.to_string(), latest.to_string()),
        Reach::Empty | Reach::Blank { .. } => (String::new(), String::new()),
    };
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::info("pull.folder", "folder reach read")
            .with("dir", telemetry::Value::Str(&dir.display().to_string()))
            .with("verb", telemetry::Value::Str(feed.source_kind().verb()))
            .with("files", telemetry::Value::Uint(reach.files() as u64))
            .with("rows", telemetry::Value::Uint(reach.rows() as u64))
            .with("earliest", telemetry::Value::Str(&ends.0))
            .with("latest", telemetry::Value::Str(&ends.1)),
    );
}

/// One folder that refused, on the rolling log — at `Error`.
///
/// It names the PATH, which is the one thing an operator needs next and the
/// one thing a run that produced no bars never used to say.
fn note_refused(dir: &Path, feed: Feed, why: &FolderError) {
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::error("pull.folder", "folder refused")
            .with("dir", telemetry::Value::Str(&dir.display().to_string()))
            .with("verb", telemetry::Value::Str(feed.source_kind().verb()))
            .with("why", telemetry::Value::Str(&why.to_string())),
    );
}

/// Reads `dir` and reports how far this feed reaches.
///
/// The only way to ask a folder feed how far back it goes. There is no
/// constant, no vendor claim and no floor — see the module header.
///
/// # Errors
///
/// [`FolderError::NotAFolderFeed`] for a REST feed, which has no folder.
/// [`FolderError::Walk`] naming the path when the folder is missing,
/// unreadable, or holds a member that will not decode — loudly, and never as
/// an empty reach. [`FolderError::Untimed`] from [`reach_of`].
pub fn read_reach(dir: &Path, feed: Feed, columns: Columns) -> Result<Reach, FolderError> {
    let members = walk(dir, feed, columns)?;
    match reach_of(&members) {
        Ok(reach) => {
            note_read(dir, feed, reach);
            Ok(reach)
        }
        Err(why) => {
            note_refused(dir, feed, &why);
            Err(why)
        }
    }
}

/// The folder's reach **and the instruments it names**, in ONE walk.
///
/// The route `crates/api/src/folder.rs` serves calls this rather than
/// [`read_reach`], because a browser asking what a folder holds needs both and
/// walking twice for them would double a cost `docs/06-limits.md` already
/// records. [`read_reach`] stays for a caller that wants only the bound: it
/// allocates no names, which is the whole reason the two are separate
/// functions and not one with a discarded field.
///
/// # Errors
///
/// Exactly [`read_reach`]'s, unchanged: a REST feed has no folder, a walk that
/// fails names the path, and a row whose timestamp is not on this calendar
/// refuses rather than being skipped.
pub fn read_census(dir: &Path, feed: Feed, columns: Columns) -> Result<Census, FolderError> {
    let members = walk(dir, feed, columns)?;
    match census_of(&members) {
        Ok(census) => {
            note_read(dir, feed, census.reach);
            Ok(census)
        }
        Err(why) => {
            note_refused(dir, feed, &why);
            Err(why)
        }
    }
}

/// The two refusals that come BEFORE anything is decoded, shared by both
/// readers so they cannot drift into refusing differently for the same folder.
///
/// A REST feed is refused first and without touching the disk — it has no
/// folder to walk, and probing one would invent a path for a vendor that has
/// none.
fn walk(dir: &Path, feed: Feed, columns: Columns) -> Result<Vec<Member>, FolderError> {
    let refuse = |why: FolderError| {
        note_refused(dir, feed, &why);
        why
    };
    if !matches!(feed.source_kind(), crate::vendor::SourceKind::Folder) {
        return Err(refuse(FolderError::NotAFolderFeed { feed }));
    }
    archive::read_dir(dir, columns).map_err(|why| {
        refuse(FolderError::Walk {
            path: dir.to_path_buf(),
            why,
        })
    })
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
    use super::{FolderError, ROOT_DIR, ROOT_ENV, root, root_from};
    use std::path::PathBuf;

    /// The root follows the store and the masters exactly: the variable wins,
    /// then the home directory.
    #[test]
    fn the_root_is_the_variable_then_the_home_directory() {
        let named =
            root_from(Some("/bought".into()), Some("/home/x".into())).expect("an explicit root");
        assert_eq!(
            named,
            PathBuf::from("/bought"),
            "the variable wins outright"
        );

        let derived = root_from(None, Some("/home/x".into())).expect("a home-derived root");
        assert_eq!(
            derived,
            PathBuf::from("/home/x").join(".brutex").join(ROOT_DIR),
            "beside masters/ and store/, in the tree this project already owns"
        );
        // AND IT IS NOT A HARDCODED ABSOLUTE LITERAL. The only fixed parts are
        // the two directory names; everything above them comes from outside.
        assert!(
            !derived.starts_with("/Users") && !derived.starts_with("/home/y"),
            "the home half must come from the caller"
        );
    }

    /// With neither set there is NO default folder, and the halt says so.
    #[test]
    fn with_no_variable_and_no_home_there_is_no_default_folder() {
        let why = root_from(None, None).expect_err("there is no default");
        assert!(matches!(why, FolderError::NoHome));
        let said = why.to_string();
        assert!(
            said.contains(ROOT_ENV) && said.contains("HOME"),
            "the halt must name both variables an operator can set: {said}"
        );
        assert!(
            said.contains("no default folder"),
            "and it must say plainly that nothing is assumed: {said}"
        );
    }

    /// The one place the environment is read agrees with the pure resolver it
    /// delegates to, whatever this machine's environment happens to be.
    ///
    /// It cannot assert a value — `set_var` is `unsafe` under edition 2024 and
    /// this crate forbids `unsafe`, so a test cannot arrange either variable.
    /// What it CAN hold is that the two never disagree, which is the wiring
    /// mistake a split like this exists to make impossible.
    #[test]
    fn the_environment_reader_and_the_pure_resolver_agree() {
        let direct = root_from(
            std::env::var_os(ROOT_ENV),
            std::env::var_os(super::HOME_ENV),
        );
        assert_eq!(root(), direct, "one of the two reads a different variable");
    }
}

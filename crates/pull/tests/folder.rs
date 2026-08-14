//! The SOURCE KIND, and everything that follows from a folder being a folder.
//!
//! # What is proven here
//!
//! The operator's rule of 12 Aug 2026 has five consequences, and each one is a
//! behaviour rather than a label:
//!
//! | | Held by |
//! |---|---|
//! | two kinds, and every feed is on the right side | `every_feed_is_rest_or_folder_and_the_two_named_vendors_are_the_folders` |
//! | no history floor — the reach is READ | `a_folder_feeds_reach_comes_from_the_folder_and_never_from_a_floor` |
//! | no token, and a missing one blocks nothing | `a_folder_feed_needs_no_credential_and_a_rest_feed_does` |
//! | the verb is not pull | `the_verb_for_a_folder_is_read_and_never_pull` |
//! | a missing folder halts and names the path | `a_missing_folder_halts_loudly_and_names_the_path` |
//! | a shared second is expected, and last wins | `a_shared_second_folds_last_price_wins` |
//!
//! # No server, no network, no vendor
//!
//! Every test here writes its own files into a scratch directory it removes.
//! Nothing opens a socket, nothing reads a credential and nothing touches the
//! operator's real folders.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test in this workspace takes: a test \
              that cannot panic cannot fail."
)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use pull::archive::Member;
use pull::csv::Columns;
use pull::fetch::RawRow;
use pull::fold::{Bucket, fold};
use pull::folder::{self, FolderError, Reach};
use pull::session::Day;
use pull::vendor::{Feed, Granularity, SourceKind};
use store::format::Bar;

// ===========================================================================
// Scratch
// ===========================================================================

/// Distinguishes two scratch trees taken in the same process.
static NEXT: AtomicU64 = AtomicU64::new(0);

/// A temporary directory that removes itself.
struct Scratch {
    root: PathBuf,
}

impl Scratch {
    fn new() -> Self {
        let serial = NEXT.fetch_add(1, Ordering::Relaxed);
        let mut root = std::env::temp_dir();
        root.push(format!("brutex-folder-{}-{serial}", std::process::id()));
        fs::create_dir_all(&root).expect("a scratch root");
        Self { root }
    }

    /// A directory under the root, created.
    fn dir(&self, name: &str) -> PathBuf {
        let dir = self.root.join(name);
        fs::create_dir_all(&dir).expect("a scratch directory");
        dir
    }

    /// A directory holding one `<name>.csv` per pair.
    fn folder(&self, name: &str, members: &[(&str, &str)]) -> PathBuf {
        let dir = self.dir(name);
        for (member, body) in members {
            fs::write(dir.join(format!("{member}.csv")), body).expect("a member");
        }
        dir
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _best_effort = fs::remove_dir_all(&self.root);
    }
}

/// A `TrueData` index body: `YYYYMMDD,HH:MM:SS,price,volume,open_interest`.
const TWO_DAYS: &str = "20221003,09:15:01,38445.65,0,0\n20221004,15:29:59,38600.00,0,0\n";

/// The same shape, one earlier day, so a folder can hold two members whose
/// spans differ and the reach has to take the union rather than the first.
const ONE_EARLIER_DAY: &str = "20220930,09:15:00,38000.00,0,0\n";

fn day(year: u16, month: u8, of_month: u8) -> Day {
    Day::new(year, month, of_month).expect("a date that exists")
}

// ===========================================================================
// The kind itself
// ===========================================================================

/// Every feed is one of the two kinds, and the two the operator named are the
/// folders.
#[test]
fn every_feed_is_rest_or_folder_and_the_two_named_vendors_are_the_folders() {
    for feed in Feed::ALL {
        let kind = feed.source_kind();
        assert!(
            matches!(kind, SourceKind::Rest | SourceKind::Folder),
            "{feed} is neither kind"
        );
        // The transport and the feed must agree — they are the same fact and
        // a page reads one while a pull reads the other.
        assert_eq!(
            kind,
            feed.descriptor().transport.kind(),
            "{feed}: the feed and its transport disagree about what it is"
        );
    }
    assert_eq!(Feed::TrueData.source_kind(), SourceKind::Folder);
    assert_eq!(Feed::Gdfl.source_kind(), SourceKind::Folder);
    assert_eq!(Feed::Dhan.source_kind(), SourceKind::Rest);
    assert_eq!(Feed::Groww.source_kind(), SourceKind::Rest);

    // Exactly two, so a third folder feed added without a decision entry
    // fails here rather than being absorbed silently.
    assert_eq!(
        Feed::ALL
            .into_iter()
            .filter(|feed| feed.source_kind() == SourceKind::Folder)
            .count(),
        2,
        "the operator named TrueData and GDFL, and those two alone"
    );
}

/// The kind's five consequences, each read off the type rather than off a
/// vendor name.
#[test]
fn the_kind_decides_credential_quota_floor_and_finest_rung() {
    assert!(SourceKind::Rest.needs_credential());
    assert!(SourceKind::Rest.needs_quota());
    assert!(SourceKind::Rest.has_history_floor());
    assert_eq!(SourceKind::Rest.finest_possible(), Granularity::Minute1);

    assert!(!SourceKind::Folder.needs_credential());
    assert!(!SourceKind::Folder.needs_quota());
    assert!(!SourceKind::Folder.has_history_floor());
    assert_eq!(SourceKind::Folder.finest_possible(), Granularity::Second1);

    // The labels are distinct and neither is empty: a page that renders both
    // must not render them the same.
    assert_ne!(SourceKind::Rest.label(), SourceKind::Folder.label());
    assert!(!SourceKind::Rest.label().is_empty());
    assert_eq!(SourceKind::Folder.to_string(), SourceKind::Folder.label());
    assert_eq!(SourceKind::Rest.to_string(), SourceKind::Rest.label());
}

/// The verb, which is the whole of rule 3.
#[test]
fn the_verb_for_a_folder_is_read_and_never_pull() {
    assert_eq!(SourceKind::Folder.verb(), "read");
    assert_eq!(SourceKind::Folder.verb_past(), "read");
    assert_eq!(SourceKind::Rest.verb(), "pull");
    assert_eq!(SourceKind::Rest.verb_past(), "pulled");

    for feed in Feed::ALL {
        let verb = feed.source_kind().verb();
        if feed.source_kind() == SourceKind::Folder {
            assert_ne!(verb, "pull", "{feed} is a folder and nothing is pulled");
        }
    }

    // And the refusal a REST feed gets when asked for a folder uses the past
    // tense, so the sentence reads.
    let why = folder::folder_of(Path::new("/nowhere"), Feed::Dhan)
        .expect_err("a REST feed has no folder");
    let said = why.to_string();
    assert!(
        said.contains("pulled"),
        "the refusal must say how Dhan's bars DO arrive: {said}"
    );
    assert!(matches!(why, FolderError::NotAFolderFeed { feed } if feed == Feed::Dhan));
}

/// A tick is never a rung anybody can ask for, on either kind.
#[test]
fn tick_is_never_a_rung_anybody_can_ask_for() {
    assert!(!Granularity::Tick.is_requestable());
    for rung in Granularity::ALL {
        if rung == Granularity::Tick {
            continue;
        }
        assert!(rung.is_requestable(), "{rung} must be askable");
    }
    // And no feed serves it, whatever its kind.
    for feed in Feed::ALL {
        assert!(
            !feed.serves(Granularity::Tick),
            "{feed} declares a rung nobody may ask for"
        );
        assert!(
            feed.finest_rung() as u8 >= feed.source_kind().finest_possible() as u8,
            "{feed} claims a rung finer than its kind can carry"
        );
    }
}

// ===========================================================================
// Where the folder is
// ===========================================================================

/// One subfolder per folder feed, named by its wire word, and a REST feed gets
/// no path at all.
#[test]
fn each_folder_feed_owns_a_subfolder_and_a_rest_feed_owns_none() {
    let root = Path::new("/bought");
    assert_eq!(
        folder::folder_of(root, Feed::TrueData).expect("a folder feed has a folder"),
        root.join("truedata")
    );
    assert_eq!(
        folder::folder_of(root, Feed::Gdfl).expect("a folder feed has a folder"),
        root.join("gdfl")
    );
    for feed in [Feed::Dhan, Feed::Groww] {
        assert!(
            folder::folder_of(root, feed).is_err(),
            "{feed} is REST and must not be handed a path that would then be \
             reported missing"
        );
    }
    // Two folder feeds never share a directory: one vendor's history must be
    // deletable without touching the other's — D-0019.
    assert_ne!(
        folder::folder_of(root, Feed::TrueData).expect("a folder"),
        folder::folder_of(root, Feed::Gdfl).expect("a folder")
    );
}

// ===========================================================================
// The reach
// ===========================================================================

/// The reach comes from the files, and it is the union across members.
#[test]
fn a_folder_feeds_reach_comes_from_the_folder_and_never_from_a_floor() {
    let scratch = Scratch::new();
    let dir = scratch.folder(
        "held",
        &[("BANKNIFTY", TWO_DAYS), ("NIFTY", ONE_EARLIER_DAY)],
    );

    let reach = folder::read_reach(&dir, Feed::TrueData, Columns::TrueDataIndex)
        .expect("the folder is there and decodes");

    let Reach::Days {
        earliest,
        latest,
        files,
        rows,
    } = reach
    else {
        panic!("a folder holding two days must report them: {reach:?}")
    };
    assert_eq!(
        earliest,
        day(2022, 9, 30),
        "the union, not the first member"
    );
    assert_eq!(latest, day(2022, 10, 4));
    assert_eq!(files, 2);
    assert_eq!(rows, 3);
    assert!(!reach.is_empty());
    assert_eq!(reach.files(), 2);
    assert_eq!(reach.rows(), 3);

    let window = reach.window().expect("a reach with days has a window");
    assert_eq!(window.from(), earliest);
    assert_eq!(window.to(), latest);

    // AND THE FLOOR IS NOT CONSULTED, because there is none to consult: the
    // descriptor records no history row for either folder feed, at any rung.
    for feed in [Feed::TrueData, Feed::Gdfl] {
        assert!(!feed.source_kind().has_history_floor());
        for rung in Granularity::ALL {
            assert!(
                feed.descriptor().history_row(rung).is_none(),
                "{feed} records a floor at {rung}; a folder's reach must come \
                 from the folder"
            );
        }
    }
}

/// An empty folder has an EMPTY reach and says so. It does not fall back.
#[test]
fn an_empty_folder_has_an_empty_reach_and_says_so() {
    let scratch = Scratch::new();
    let dir = scratch.folder("bought-nothing", &[]);

    let reach = folder::read_reach(&dir, Feed::TrueData, Columns::TrueDataIndex)
        .expect("an empty folder is not an error");

    assert_eq!(reach, Reach::Empty);
    assert!(reach.is_empty());
    assert_eq!(reach.files(), 0);
    assert_eq!(reach.rows(), 0);
    assert!(
        reach.window().is_none(),
        "an empty reach answers for no day — never for every day"
    );
    // The empty reach must not be a REST-shaped floor wearing different words.
    assert!(!Feed::TrueData.source_kind().has_history_floor());
}

/// Files present and every one blank is a THIRD thing, and it is not `Empty`.
#[test]
fn a_folder_of_blank_files_is_not_the_same_as_an_empty_folder() {
    let scratch = Scratch::new();
    let dir = scratch.folder("blank", &[("BANKNIFTY", ""), ("NIFTY", "")]);

    let reach = folder::read_reach(&dir, Feed::TrueData, Columns::TrueDataIndex)
        .expect("blank members decode to no rows");

    assert_eq!(reach, Reach::Blank { files: 2 });
    assert!(reach.is_empty(), "it answers for no day");
    assert_eq!(reach.files(), 2, "and the files WERE delivered");
    assert_eq!(reach.rows(), 0);
    assert!(reach.window().is_none());
    assert_ne!(reach, Reach::Empty, "bought-and-blank is not not-bought");
}

/// A missing folder halts loudly and names the path. It is never an empty
/// reach.
#[test]
fn a_missing_folder_halts_loudly_and_names_the_path() {
    let scratch = Scratch::new();
    let dir = scratch.root.join("never-created");

    let why = folder::read_reach(&dir, Feed::TrueData, Columns::TrueDataIndex)
        .expect_err("a folder that is not there is a halt");

    let said = why.to_string();
    assert!(
        said.contains(&dir.display().to_string()),
        "the halt must name the path an operator opens next: {said}"
    );
    assert!(matches!(why, FolderError::Walk { .. }));
}

/// A folder that is a FILE is the same halt, named the same way. An operator
/// who pointed the variable at the zip instead of the directory gets a path,
/// not a shrug.
#[test]
fn a_folder_that_is_not_a_directory_halts_and_names_the_path() {
    let scratch = Scratch::new();
    let not_a_dir = scratch.root.join("bought.csv");
    fs::write(&not_a_dir, TWO_DAYS).expect("a file where a folder should be");

    let why = folder::read_reach(&not_a_dir, Feed::TrueData, Columns::TrueDataIndex)
        .expect_err("a file is not a folder");
    assert!(why.to_string().contains(&not_a_dir.display().to_string()));
}

/// A member that will not decode refuses the whole reach, naming the member.
#[test]
fn a_member_that_will_not_decode_refuses_the_reach_by_name() {
    let scratch = Scratch::new();
    let dir = scratch.folder("bent", &[("BANKNIFTY", "20221003,09:15:01\n")]);

    let why = folder::read_reach(&dir, Feed::TrueData, Columns::TrueDataIndex)
        .expect_err("two fields where five are declared");
    let said = why.to_string();
    assert!(
        said.contains("BANKNIFTY"),
        "the refusal must name the member, not the folder alone: {said}"
    );
}

/// A REST feed cannot be asked to read a folder, even one that exists.
#[test]
fn a_rest_feed_cannot_be_asked_to_read_a_folder() {
    let scratch = Scratch::new();
    let dir = scratch.folder("wrong-kind", &[("BANKNIFTY", TWO_DAYS)]);

    let why = folder::read_reach(&dir, Feed::Groww, Columns::TrueDataIndex)
        .expect_err("Groww is REST and has no folder");
    assert!(matches!(why, FolderError::NotAFolderFeed { feed } if feed == Feed::Groww));
}

/// A timestamp that is not a moment refuses the whole reach rather than being
/// skipped: a column read at the wrong offset looks exactly like this.
#[test]
fn a_row_whose_timestamp_is_not_a_moment_refuses_the_whole_reach() {
    let member = Member {
        path: PathBuf::from("/bought/BANKNIFTY.csv"),
        instrument: "BANKNIFTY".to_owned(),
        rows: vec![RawRow {
            timestamp: i64::MIN,
            open: 1,
            high: 1,
            low: 1,
            close: 1,
            volume: 0,
            open_interest: None,
        }],
    };
    let why = folder::reach_of(std::slice::from_ref(&member))
        .expect_err("i64::MIN is not a moment on any calendar");
    let said = why.to_string();
    assert!(said.contains("BANKNIFTY"), "name the member: {said}");
    assert!(matches!(why, FolderError::Untimed { secs, .. } if secs == i64::MIN));
}

/// The reach of an already-decoded walk is one pass over its rows, and it
/// takes the extremes rather than the ends of the last member read.
#[test]
fn the_reach_of_a_walk_is_one_pass_over_its_rows() {
    let rows = |stamps: &[i64]| {
        stamps
            .iter()
            .map(|&ts| RawRow {
                timestamp: ts,
                open: 1,
                high: 1,
                low: 1,
                close: 1,
                volume: 0,
                open_interest: None,
            })
            .collect::<Vec<_>>()
    };
    // 2022-10-03 09:15:01 IST and 2022-09-30 09:15:00 IST, as epoch seconds.
    let later = 1_664_768_701;
    let earlier = 1_664_509_500;
    let members = vec![
        Member {
            path: PathBuf::from("/bought/A.csv"),
            instrument: "A".to_owned(),
            rows: rows(&[later]),
        },
        Member {
            path: PathBuf::from("/bought/B.csv"),
            instrument: "B".to_owned(),
            rows: rows(&[earlier]),
        },
    ];
    let reach = folder::reach_of(&members).expect("both are moments");
    let Reach::Days {
        earliest, latest, ..
    } = reach
    else {
        panic!("{reach:?}")
    };
    assert!(
        earliest < latest,
        "the extremes, not the last member's ends"
    );
    assert_eq!(earliest, day(2022, 9, 30));
    assert_eq!(latest, day(2022, 10, 3));

    assert_eq!(
        folder::reach_of(&[]).expect("no members is not an error"),
        Reach::Empty
    );
}

// ===========================================================================
// No token
// ===========================================================================

/// A folder feed needs no credential, and a REST feed does. This is what stops
/// `CLAUDE.md` §8's machinery blocking a source with nothing to authenticate
/// against.
#[test]
fn a_folder_feed_needs_no_credential_and_a_rest_feed_does() {
    for feed in Feed::ALL {
        let kind = feed.source_kind();
        assert_eq!(
            kind.needs_credential(),
            feed.descriptor().transport.needs_credential(),
            "{feed}: the kind and the transport disagree about the token"
        );
        assert_eq!(
            kind.needs_quota(),
            feed.descriptor().transport.needs_governor(),
            "{feed}: the kind and the transport disagree about the quota"
        );
        if kind == SourceKind::Folder {
            assert!(!kind.needs_credential(), "{feed} has nothing to sign in to");
            assert!(!kind.needs_quota(), "{feed} spends nobody's budget");
        }
    }
}

// ===========================================================================
// The shared second
// ===========================================================================

/// TWO ROWS SHARING A SECOND ARE EXPECTED INPUT, AND LAST WINS FOR THE PRICE.
///
/// The operator, 12 Aug 2026: *"for truedata or gdfl also we will have multiple
/// ticks but anyhow always the timestamp will be mapped as second."*
#[test]
fn a_shared_second_folds_last_price_wins() {
    let at = |secs: i64| secs * 1_000_000;
    let snap = |ts: i64, price: i64, volume: i64, oi: i64| Bar {
        ts_micros: ts,
        open: price,
        high: price,
        low: price,
        close: price,
        volume,
        open_interest: oi,
    };
    // Three rows in ONE second — the shape `docs/08-vendor-samples.md`
    // measured — followed by one in the next.
    let snapshots = vec![
        snap(at(1), 100, 5, i64::MIN),
        snap(at(1), 130, 7, 2_000),
        snap(at(1), 90, 3, i64::MIN),
        snap(at(2), 111, 1, 2_100),
    ];

    let bars = fold(&snapshots, Bucket::SECOND).expect("a shared second is not corruption");

    assert_eq!(bars.len(), 2, "one second in, one record out");
    assert_eq!(bars[0].ts_micros, at(1), "stamped at the second it covers");
    assert_eq!(bars[0].open, 100, "the FIRST in file order");
    assert_eq!(bars[0].high, 130);
    assert_eq!(bars[0].low, 90);
    assert_eq!(bars[0].close, 90, "LAST WINS — the last row in file order");
    assert_eq!(bars[0].volume, 15, "summed, so nothing is discarded");
    assert_eq!(
        bars[0].open_interest, 2_000,
        "the last row that CARRIED one; i64::MIN is the null and never a value"
    );
    assert_eq!(bars[1].close, 111, "the next second is its own record");

    // AND IT IS NOT REFUSED. That is the whole point: refusing a shared second
    // would refuse every file the operator bought.
    assert!(fold(&snapshots, Bucket::SECOND).is_ok());
}

/// A second's worth of rows folds onward to the rung the store can carry, and
/// the one-second rung refuses at the write boundary BY NAME rather than being
/// filed under a rung it is not.
#[test]
fn the_second_rung_has_no_directory_and_refuses_rather_than_substituting() {
    assert!(
        Granularity::Second1.store_timeframe().is_none(),
        "crates/store ships no one-second stride yet"
    );
    assert!(Granularity::Minute1.store_timeframe().is_some());
    assert!(Granularity::Day1.store_timeframe().is_some());
    // Which is why the folder feeds' seconds reach disk as minutes: the same
    // rows, folded once more.
    let at = |secs: i64| secs * 1_000_000;
    let snap = |ts: i64, price: i64| Bar {
        ts_micros: ts,
        open: price,
        high: price,
        low: price,
        close: price,
        volume: 1,
        open_interest: i64::MIN,
    };
    let seconds = fold(
        &[snap(at(0), 100), snap(at(0), 130), snap(at(59), 90)],
        Bucket::SECOND,
    )
    .expect("seconds fold");
    assert_eq!(seconds.len(), 2);
    let minutes = fold(&seconds, Bucket::MINUTE).expect("and then minutes");
    assert_eq!(minutes.len(), 1);
    assert_eq!(minutes[0].open, 100);
    assert_eq!(minutes[0].high, 130);
    assert_eq!(minutes[0].close, 90);
    assert_eq!(minutes[0].volume, 3, "and every row is still counted");
}

/// An UNREADABLE folder halts and names the path, exactly as a missing one
/// does — and it is never an empty reach.
///
/// The two failures are different (`is_dir` answers yes, `read_dir` answers
/// `EACCES`) and they reach the operator the same way, which is the point: a
/// folder feed whose folder cannot be listed has no reach, and reporting one
/// would be the fallback `CLAUDE.md` §4 bans.
#[cfg(unix)]
#[test]
fn an_unreadable_folder_halts_loudly_and_names_the_path() {
    use std::os::unix::fs::PermissionsExt;

    let scratch = Scratch::new();
    let dir = scratch.folder("sealed", &[("BANKNIFTY", TWO_DAYS)]);
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o000)).expect("seal the folder");

    // A PRIVILEGED PROCESS DEFEATS THE ARRANGEMENT, so say that rather than
    // passing on an assertion that never ran. CI is a non-root user on
    // ubuntu-24.04 and so is every developer machine this repository targets.
    let sealed = fs::read_dir(&dir).is_err();

    let outcome = folder::read_reach(&dir, Feed::TrueData, Columns::TrueDataIndex);
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).expect("unseal, so Drop can");

    assert!(
        sealed,
        "this process can list a mode-000 directory, so the refusal under test \
         cannot be produced here — run the suite as an unprivileged user"
    );
    let why = outcome.expect_err("a folder that cannot be listed is a halt");
    let said = why.to_string();
    assert!(
        said.contains(&dir.display().to_string()),
        "the halt must name the path an operator opens next: {said}"
    );
    assert!(matches!(why, FolderError::Walk { .. }));
}

/// THE CENSUS NAMES WHAT THE WALK ALREADY KNEW, AND SORTS IT.
///
/// `Member::instrument` is taken off the file name because a file name is the
/// ONLY identity an archive has — no ISIN, no security id, no master, and for
/// `TrueData` not even a ticker column. `reach_of` walked every member, counted
/// them, and discarded that name, so the one fact answering "what is in this
/// feed" was read on every walk and reachable by nobody. D-0141.
///
/// SORTED, because `read_dir` order is the filesystem's: it is not stable
/// between machines and `CLAUDE.md` §3 rule 5 makes the same input owe the same
/// output. Asserted against a deliberately unsorted input so a walk that
/// happened to arrive sorted could not pass this by luck.
#[test]
fn a_census_names_every_instrument_the_folder_holds_in_a_stable_order() {
    let members = vec![
        member("NIFTY", 1_664_768_701),
        member("BANKNIFTY", 1_664_509_500),
        member("MIDCPNIFTY", 1_664_768_701),
    ];
    let census = folder::census_of(&members, Vec::new()).expect("all three are moments");
    assert_eq!(
        census.instruments,
        vec![
            "BANKNIFTY".to_owned(),
            "MIDCPNIFTY".to_owned(),
            "NIFTY".to_owned()
        ],
        "sorted, not in walk order"
    );
    assert_eq!(census.collisions, 0, "each file names its own instrument");
    assert_eq!(census.reach, folder::reach_of(&members).expect("same walk"));
}

/// TWO MEMBERS CLAIMING ONE INSTRUMENT ARE COUNTED, NEVER SWALLOWED.
///
/// GDFL nests `Options/` and `Futures/`, so one stem can appear twice. Silently
/// deduplicating would leave a caller unable to tell a clean folder from a
/// colliding one — and a collision here is exactly the `ambiguous` bucket
/// D-0141 names on the folder side, which is the point of counting it.
#[test]
fn two_members_naming_one_instrument_are_counted_rather_than_hidden() {
    let members = vec![
        member("NIFTY", 1_664_768_701),
        member("NIFTY", 1_664_509_500),
        member("BANKNIFTY", 1_664_768_701),
    ];
    let census = folder::census_of(&members, Vec::new()).expect("both are moments");
    assert_eq!(
        census.instruments,
        vec!["BANKNIFTY".to_owned(), "NIFTY".to_owned()],
        "the list is distinct"
    );
    assert_eq!(census.collisions, 1, "and the duplicate is REPORTED");
}

/// AN EMPTY FOLDER CENSUSES TO AN EMPTY LIST, NOT TO A REFUSAL.
///
/// The same distinction `Reach::Empty` exists for: a folder that is there and
/// holds nothing is an ANSWER — the months have not been bought yet — and it
/// must not arrive looking like a walk that failed.
#[test]
fn an_empty_folder_censuses_to_an_empty_list_and_an_empty_reach() {
    let census = folder::census_of(&[], Vec::new()).expect("an empty folder is an answer");
    assert_eq!(census.reach, Reach::Empty);
    assert!(census.instruments.is_empty());
    assert_eq!(census.collisions, 0);
}

/// A CENSUS REFUSES EXACTLY WHERE THE REACH DOES.
///
/// A census of a folder whose days cannot be read is not a smaller census, it
/// is no answer at all — so the untimed row refuses the whole thing rather than
/// yielding a name list beside a reach nobody could compute.
#[test]
fn a_census_refuses_wherever_the_reach_refuses() {
    let broken = member("BANKNIFTY", i64::MIN);
    let why = folder::census_of(std::slice::from_ref(&broken), Vec::new())
        .expect_err("i64::MIN is not a moment on any calendar");
    assert!(matches!(why, FolderError::Untimed { secs, .. } if secs == i64::MIN));
    assert!(why.to_string().contains("BANKNIFTY"), "name the member");
}

/// One member holding one row at `stamp`, for the census tests above.
fn member(instrument: &str, stamp: i64) -> Member {
    Member {
        path: PathBuf::from(format!("/bought/{instrument}.csv")),
        instrument: instrument.to_owned(),
        rows: vec![RawRow {
            timestamp: stamp,
            open: 1,
            high: 1,
            low: 1,
            close: 1,
            volume: 0,
            open_interest: None,
        }],
    }
}

/// READING A REAL FOLDER ANSWERS THE BOUND AND THE NAMES IN ONE WALK.
///
/// The route `crates/api/src/folder.rs` serves calls `read_census` rather than
/// `read_reach` because a browser asking what a folder holds needs both, and
/// walking twice would double a cost `docs/06-limits.md` records. This asserts
/// the two halves agree with the two functions that answer them separately, so
/// the shared walk cannot drift from either.
#[test]
fn reading_a_folder_answers_its_bound_and_its_names_in_one_walk() {
    let scratch = Scratch::new();
    let dir = scratch.folder(
        "censused",
        &[("BANKNIFTY", TWO_DAYS), ("NIFTY", ONE_EARLIER_DAY)],
    );

    let census = folder::read_census(&dir, Feed::TrueData, Columns::TrueDataIndex)
        .expect("the folder is there and decodes");

    assert_eq!(
        census.instruments,
        vec!["BANKNIFTY".to_owned(), "NIFTY".to_owned()],
        "the names off the file names, sorted"
    );
    assert_eq!(census.collisions, 0);
    assert_eq!(
        census.reach,
        folder::read_reach(&dir, Feed::TrueData, Columns::TrueDataIndex).expect("same folder"),
        "the bound is the one read_reach answers, unchanged"
    );
}

/// A REST FEED IS REFUSED BY THE CENSUS EXACTLY AS BY THE REACH, and without
/// touching the disk: it has no folder, and probing one would invent a path for
/// a vendor that has none. The shared `walk` is what makes the two refusals one.
#[test]
fn a_rest_feed_cannot_be_asked_to_census_a_folder_either() {
    let scratch = Scratch::new();
    let dir = scratch.folder("broker", &[("BANKNIFTY", TWO_DAYS)]);
    let why = folder::read_census(&dir, Feed::Groww, Columns::TrueDataIndex)
        .expect_err("a broker has no folder");
    assert!(matches!(why, FolderError::NotAFolderFeed { .. }), "{why:?}");
}

/// A FILE OF A DIFFERENT PRODUCT IS A FINDING, NOT THE END OF THE FOLDER.
///
/// MEASURED ON THE OPERATOR'S OWN DISK, 14 Aug 2026. `vendor-data/gdfl/` holds
/// `GFDLNFO_TICK_01072025/`, whose members decode exactly against the declared
/// ten-field layout — `Ticker,Date,Time,LTP,BuyPrice,BuyQty,SellPrice,SellQty,
/// LTQ,OpenInterest` — and one loose `GFDLNFO_BACKADJUSTED_01072025.csv` beside
/// them carrying `Ticker,Date,Time,Open,High,Low,Close,Volume,Open Interest`:
/// nine fields, and an OHLC product rather than a tick one.
///
/// `/folder.json` answered the WHOLE FOLDER with that one file's refusal, so a
/// census of a folder the operator had bought correctly reported nothing at
/// all. A census asks *what is in here*; one file of another product does not
/// make the answer unknown, it makes it a shorter list with a named exception.
///
/// The exception is NAMED, never swallowed — `CLAUDE.md` §4. The good member
/// still lands, the stray still travels with the decoder's own words, and the
/// ingest path is unchanged: `read_dir` still refuses, because a folder read in
/// part is a store written in part.
#[test]
fn one_file_of_another_product_is_reported_and_the_rest_of_the_folder_still_reads() {
    let scratch = Scratch::new();
    let dir = scratch.folder(
        "mixed",
        &[
            ("BANKNIFTY", TWO_DAYS),
            // Nine fields where five are declared: a different product, which
            // is exactly the shape of the file on the operator's disk.
            ("STRAY", "20250616,09:07:42,1526.56,0,0,0.00,0,0.00,0\n"),
        ],
    );

    // THE INGEST PATH IS UNCHANGED AND STILL REFUSES THE WHOLE FOLDER.
    let why = folder::read_reach(&dir, Feed::TrueData, Columns::TrueDataIndex)
        .expect_err("the ingest walk refuses a member it cannot decode");
    assert!(why.to_string().contains("STRAY"), "name the file: {why}");

    // THE CENSUS READS THE REST AND REPORTS THE ONE.
    let census = folder::read_census(&dir, Feed::TrueData, Columns::TrueDataIndex)
        .expect("one stray member does not make the folder unreadable");
    assert_eq!(
        census.instruments,
        vec!["BANKNIFTY".to_owned()],
        "the member that decodes still lands"
    );
    assert_eq!(
        census.rejected.len(),
        1,
        "and the one that does not is kept"
    );
    let bad = &census.rejected[0];
    assert!(
        bad.path.to_string_lossy().contains("STRAY"),
        "which file: {bad:?}"
    );
    assert!(
        !bad.why.is_empty(),
        "and why, in the decoder's own words: {bad:?}"
    );
    assert!(
        !census.reach.is_empty(),
        "the reach comes from the members that read, not from zero"
    );
}

#![cfg(test)]
//! BASE-COV-PULL-PS3 (G3): the import journal at its edges: a whole-tree key
//! of the closing auction's era ended only by whole-tree closes, a load past
//! its stated size, refreshes that read a restated key, a line too short to
//! name a day, a member key's close and one of two filters reopened, keys
//! with no day, a refusal whose day entry is gone, a write of nothing, a lock
//! that cannot be taken, a path with no folder, a folder the host will not
//! make and a `mkdir` that loses its race. Every symbol is invented
//! (`ZZQ...`); every journal line is one this journal writes or reads.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail"
)]

use std::collections::BTreeSet;
use std::io::Write as _;

use super::*;
use crate::gdfl_fixtures::scratch;

/// A journal file holding `lines`, each with its newline, in a folder of its
/// own: `(that folder, the file)`.
fn journal_file(tag: &str, lines: &[&str]) -> (PathBuf, PathBuf) {
    let root = scratch(tag);
    let path = root.join("imports").join("gdfl.journal");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut text = String::new();
    for line in lines {
        text.push_str(line);
        text.push('\n');
    }
    std::fs::write(&path, text).unwrap();
    (root, path)
}

/// Appends `text` to the file at `path`, as another process would.
fn grow(path: &Path, text: &str) {
    let mut file = std::fs::OpenOptions::new().append(true).open(path).unwrap();
    file.write_all(text.as_bytes()).unwrap();
}

/// A WHOLE-TREE KEY OF THE AUCTION ERA ENDED ONLY BY WHOLE-TREE CLOSES STANDS
/// FOR THE WHOLE TREE: begun under definition 3 and closed under 5 with a
/// scope, no probe of one share can tell that the scope held every share,
/// so a `*` filter meets it whatever the scopes held; a names filter meets
/// it only for a share no such scope held.
#[test]
fn a_whole_tree_key_ended_only_by_whole_tree_closes_stands_for_the_whole_tree() {
    let (root, path) = journal_file(
        "cov-journal-whole",
        &[
            "run 1 definition=3",
            "begin stocks 2026-08-04 *",
            "run 2 definition=5",
            "done stocks 2026-08-04 * definition=5 scope=s1",
        ],
    );
    let journal = Journal::load(path).unwrap();
    let restated = &journal.auction_restated["stocks 2026-08-04"];
    assert!(!restated.every);
    assert!(restated.names.is_empty());
    assert!(restated.whole_named.is_empty());
    assert_eq!(restated.whole_every, [vec!["s1".to_owned()]]);
    assert!(restated.meets("*", &|_, _| true));
    assert!(!restated.meets("ZZQA", &|scope, _| scope == "s1"));
    assert!(restated.meets("ZZQA", &|_, _| false));
    std::fs::remove_dir_all(root).unwrap();
}

/// A close under definition 5 of a whole tree whose scope was noted already
/// notes it once: the same scope read twice is one scope, not two.
#[test]
fn a_whole_tree_scope_read_twice_is_noted_once() {
    let (root, path) = journal_file(
        "cov-journal-scope-twice",
        &[
            "run 1 definition=3",
            "begin stocks 2026-08-04 ZZQA",
            "done stocks 2026-08-04 * definition=5 scope=s1",
            "done stocks 2026-08-04 * definition=5 scope=s1",
            "done stocks 2026-08-04 * definition=5 scope=s2",
        ],
    );
    let journal = Journal::load(path).unwrap();
    let restated = &journal.auction_restated["stocks 2026-08-04"];
    assert_eq!(
        restated.whole_named["ZZQA"],
        [vec!["s1".to_owned(), "s2".to_owned()]]
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// A run's OWN refusal of a key it began under an earlier definition gives
/// the key back the state it had before that `begin`: here the whole-tree
/// close under 5 that had ended it for the shares of `s1`. Another run's
/// refusal says nothing of what the early run wrote, and leaves the key
/// begun, standing for every share it names.
#[test]
fn a_runs_own_refusal_gives_its_key_back_the_state_before_its_begin() {
    let lines = |refused_by: u64| {
        [
            "run 1 definition=5".to_owned(),
            "done stocks 2026-08-04 ZZQA definition=3".to_owned(),
            "done stocks 2026-08-04 * definition=5 scope=s1".to_owned(),
            "run 2 definition=3".to_owned(),
            "begin stocks 2026-08-04 ZZQA".to_owned(),
            format!("refused stocks 2026-08-04 ZZQA run={refused_by} stage=vendor reason=x"),
        ]
    };
    let held_by_s1 = |scope: &str, share: &str| (scope, share) == ("s1", "ZZQA");
    let own: Vec<String> = lines(2).to_vec();
    let own: Vec<&str> = own.iter().map(String::as_str).collect();
    let (root, path) = journal_file("cov-journal-own-refusal", &own);
    let journal = Journal::load(path).unwrap();
    let restated = &journal.auction_restated["stocks 2026-08-04"];
    assert_eq!(restated.whole_named["ZZQA"], [vec!["s1".to_owned()]]);
    assert!(
        !restated.meets("ZZQA", &held_by_s1),
        "ended where s1 held it"
    );
    assert!(restated.meets("ZZQA", &|_, _| false));
    let other: Vec<String> = lines(3).to_vec();
    let other: Vec<&str> = other.iter().map(String::as_str).collect();
    let (root2, path2) = journal_file("cov-journal-other-refusal", &other);
    let journal = Journal::load(path2).unwrap();
    let restated = &journal.auction_restated["stocks 2026-08-04"];
    assert!(restated.names.contains("ZZQA"));
    assert!(
        restated.meets("ZZQA", &held_by_s1),
        "begun again, and standing"
    );
    std::fs::remove_dir_all(root).unwrap();
    std::fs::remove_dir_all(root2).unwrap();
}

/// A refusal by the run that began a key, when the day's entry for that key
/// is gone, undoes the `begin` and makes no entry of its own. (Reached by a
/// direct call: no line the journal reads leaves a begun key without its
/// day's entry.)
#[test]
fn a_refusal_whose_days_entry_is_gone_undoes_the_begin_and_makes_none() {
    let mut journal = Journal::default();
    journal.begun_early.insert(
        "stocks 2026-08-04 ZZQA".to_owned(),
        (Some(Ended::default()), Some(2)),
    );
    journal.auction_line(
        "refused",
        ("stocks", "2026-08-04", "ZZQA"),
        (None, Some(2), None),
    );
    assert!(journal.begun_early.is_empty());
    assert!(journal.auction_keys.is_empty());
}

/// A done key with no day (no space) is filed under no day, by a load's
/// derivation and by a refresh's note alike; a key with one is.
#[test]
fn a_done_key_with_no_day_is_filed_under_no_day() {
    let mut journal = Journal::default();
    journal.done.insert("nospace".to_owned());
    journal.done.insert("indices 2024-03-01 *".to_owned());
    journal.derive();
    assert_eq!(journal.closed_by.len(), 1);
    assert_eq!(
        journal.closed_by["indices 2024-03-01"],
        BTreeSet::from(["*".to_owned()])
    );
    let before = journal.closed_by.clone();
    journal.note_closed_by("nospace");
    assert_eq!(journal.closed_by, before);
}

/// A REFRESH THAT READS A RESTATED KEY OF THE AUCTION ERA FILES IT FOR
/// ADMISSION, as a load would: a share's day from 2026-08-03 closed under
/// definition 3 stands for its share until a close under 5 that reads the
/// share ends it, and the refresh that reads that close drops the day.
#[test]
fn a_refresh_that_reads_a_restated_auction_key_files_it_for_admission() {
    let (root, path) = journal_file("cov-journal-refresh-restated", &["run 1 definition=5"]);
    let mut journal = Journal::read_only(path.clone()).unwrap();
    assert!(journal.auction_restated.is_empty());
    grow(&path, "done stocks 2026-08-04 ZZQA definition=3\n");
    assert_eq!(journal.refresh().unwrap(), Refreshed::Read { lines: 1 });
    let restated = &journal.auction_restated["stocks 2026-08-04"];
    assert!(restated.names.contains("ZZQA"));
    assert!(restated.meets("ZZQA", &|_, _| true));
    assert!(!restated.meets("ZZQB", &|_, _| true));
    grow(&path, "done stocks 2026-08-04 ZZQA definition=5\n");
    assert_eq!(journal.refresh().unwrap(), Refreshed::Read { lines: 1 });
    assert!(!journal.auction_restated.contains_key("stocks 2026-08-04"));
    std::fs::remove_dir_all(root).unwrap();
}

/// A refresh of lines too short to name a key (a blank line, a `run` line
/// with its id alone) reads them and names no day: the run is known, no
/// day's filters move.
#[test]
fn a_refresh_of_lines_too_short_to_name_a_key_names_no_day() {
    let (root, path) = journal_file(
        "cov-journal-refresh-short",
        &["done indices 2024-03-01 * definition=5"],
    );
    let mut journal = Journal::read_only(path.clone()).unwrap();
    let before = journal.closed_by.clone();
    grow(&path, "\nrun 9\n");
    assert_eq!(journal.refresh().unwrap(), Refreshed::Read { lines: 2 });
    assert!(journal.runs.contains(&9));
    assert_eq!(journal.closed_by, before);
    std::fs::remove_dir_all(root).unwrap();
}

/// A refresh that reads a MEMBER key's close (`<filter>+<member>`) leaves
/// the day's filters as they were: a member key is its base key's, never a
/// filter of its own; the member is noted under its base key.
#[test]
fn a_refresh_of_a_member_keys_close_leaves_the_days_filters_alone() {
    let (root, path) = journal_file(
        "cov-journal-refresh-member",
        &["done stocks 2024-03-01 * definition=5"],
    );
    let mut journal = Journal::read_only(path.clone()).unwrap();
    grow(
        &path,
        "done stocks 2024-03-01 *+NSE-CASH-ZZQA definition=5\n",
    );
    assert_eq!(journal.refresh().unwrap(), Refreshed::Read { lines: 1 });
    assert_eq!(
        journal.closed_by["stocks 2024-03-01"],
        BTreeSet::from(["*".to_owned()])
    );
    assert_eq!(
        journal.member_keys["stocks 2024-03-01 *"],
        BTreeSet::from(["NSE-CASH-ZZQA".to_owned()])
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// A refresh that reopens one of a day's two done filters keeps the other;
/// reopening the second drops the day.
#[test]
fn a_refresh_that_reopens_one_of_two_filters_keeps_the_other() {
    let (root, path) = journal_file(
        "cov-journal-refresh-reopen",
        &[
            "done indices 2024-03-01 * definition=5",
            "done indices 2024-03-01 ZZQA definition=5",
        ],
    );
    let mut journal = Journal::read_only(path.clone()).unwrap();
    assert_eq!(journal.closed_by["indices 2024-03-01"].len(), 2);
    grow(
        &path,
        "incomplete indices 2024-03-01 ZZQA definition=5 stage=vendor reason=x\n",
    );
    assert_eq!(journal.refresh().unwrap(), Refreshed::Read { lines: 1 });
    assert_eq!(
        journal.closed_by["indices 2024-03-01"],
        BTreeSet::from(["*".to_owned()])
    );
    grow(
        &path,
        "incomplete indices 2024-03-01 * definition=5 stage=vendor reason=x\n",
    );
    assert_eq!(journal.refresh().unwrap(), Refreshed::Read { lines: 1 });
    assert!(!journal.closed_by.contains_key("indices 2024-03-01"));
    std::fs::remove_dir_all(root).unwrap();
}

/// WRITING NOTHING writes nothing: a close with no line asked of a journal
/// whose last line is whole, or of an empty one, leaves the file byte for
/// byte as it was and what the journal knows of it unchanged.
#[test]
fn writing_nothing_to_a_journal_whose_last_line_is_whole_writes_nothing() {
    let (root, path) = journal_file("cov-journal-write-none", &["run 1 definition=5"]);
    for text in [&b"run 1 definition=5\n"[..], &b""[..]] {
        std::fs::write(&path, text).unwrap();
        let journal = Journal::load(path.clone()).unwrap();
        let known = journal.known.get();
        journal.write(None).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), text);
        assert_eq!(journal.known.get(), known);
    }
    std::fs::remove_dir_all(root).unwrap();
}

/// A JOURNAL LOCK THAT CANNOT BE TAKEN (a folder at its name) refuses the
/// append by the journal's path and the lock's own words, and the journal
/// is left as it was.
#[test]
fn a_lock_that_cannot_be_taken_refuses_the_append_by_the_journals_path() {
    let (root, path) = journal_file("cov-journal-lock", &["run 1 definition=5"]);
    let journal = Journal::load(path.clone()).unwrap();
    std::fs::create_dir(journal_lock_path(&path)).unwrap();
    let why = journal
        .append("run 2 definition=5")
        .unwrap_err()
        .to_string();
    assert!(
        why.starts_with(&format!("the import journal: {}: ", path.display())),
        "{why}"
    );
    assert!(why.contains("gdfl.journal.lock"), "{why}");
    assert_eq!(std::fs::read(&path).unwrap(), b"run 1 definition=5\n");
    std::fs::remove_dir_all(root).unwrap();
}

/// The variable the child of the no-folder test runs its body under.
const NO_FOLDER: &str = "BRUTEX_JOURNAL_COV_NO_FOLDER";

/// That test's name, which its child runs `--exact`.
const NO_FOLDER_TEST: &str =
    "gdfl_import::journal::cov_tests::a_path_with_no_folder_makes_none_and_is_refused_by_its_open";

/// OUT OF THE BOX: a journal and a ledger at the EMPTY path, which has no
/// folder. Neither makes one; each takes its lock beside the path (in the
/// working folder, which the child is given as a scratch folder of its
/// own) and is refused by its own open, the path named as it is: empty.
#[test]
fn a_path_with_no_folder_makes_none_and_is_refused_by_its_open() {
    if std::env::var_os(NO_FOLDER).is_some() {
        // The host's own words for opening the empty path.
        let open = std::fs::File::open("").unwrap_err().to_string();
        let journal = Journal::load(PathBuf::new()).unwrap();
        assert_eq!(journal.known.get(), (0, 0), "no file: an empty journal");
        let why = journal
            .append("run 1 definition=5")
            .unwrap_err()
            .to_string();
        assert_eq!(why, format!("the import journal: : {open}"));
        let why = append_lines_within(
            Path::new(""),
            &["a line".to_owned()],
            std::time::Duration::from_secs(1),
        )
        .unwrap_err();
        assert_eq!(why, format!(": {open}"));
        return;
    }
    let cwd = scratch("cov-journal-no-folder");
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", NO_FOLDER_TEST, "--nocapture", "--test-threads=1"])
        .env(NO_FOLDER, "1")
        .current_dir(&cwd)
        .output()
        .unwrap();
    let said = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.status.success(), "{said}");
    assert!(said.contains("1 passed"), "{said}");
    assert!(cwd.join("gdfl.journal.lock").is_file());
    assert!(!cwd.join("imports").exists());
    std::fs::remove_dir_all(cwd).unwrap();
}

/// A FOLDER THE HOST WILL NOT MAKE (its parent closed to writing) is the
/// host's refusal, by its own kind, never taken for one already made. Run
/// where the permission bits bind (D-0995): as root the `mkdir` succeeds.
#[test]
fn a_folder_the_host_will_not_make_is_its_refusal() {
    crate::support::where_permission_binds(
        "gdfl_import::journal::cov_tests::a_folder_the_host_will_not_make_is_its_refusal",
        a_folder_the_host_will_not_make_is_its_refusal_body,
    );
}

/// The test above, run where the mode bits bind.
fn a_folder_the_host_will_not_make_is_its_refusal_body() {
    use std::os::unix::fs::PermissionsExt as _;
    let root = scratch("cov-journal-closed-root");
    std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o555)).unwrap();
    let why = make_leaf_dir(&root.join("imports")).unwrap_err();
    std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(why.kind(), std::io::ErrorKind::PermissionDenied);
    assert!(!root.join("imports").exists());
    std::fs::remove_dir_all(root).unwrap();
}

/// A `mkdir` that loses its race to another writer (the folder made between
/// the check and the `mkdir`, which answers "already exists") is the folder
/// made, and nothing is refused; one answered "already exists" with no
/// folder there (a file at the name) is still refused.
#[test]
fn a_mkdir_that_loses_its_race_to_another_writer_is_the_folder_made() {
    let root = scratch("cov-journal-race");
    let raced = |dir: &Path| -> std::io::Result<()> {
        std::fs::create_dir(dir)?;
        Err(std::io::ErrorKind::AlreadyExists.into())
    };
    let dir = root.join("imports");
    assert!(make_leaf_dir_with(&dir, raced).is_ok());
    assert!(dir.is_dir());
    let file = root.join("a-file");
    let lying = |_: &Path| -> std::io::Result<()> { Err(std::io::ErrorKind::AlreadyExists.into()) };
    let why = make_leaf_dir_with(&file, lying).unwrap_err();
    assert_eq!(why.kind(), std::io::ErrorKind::AlreadyExists);
    assert!(!file.exists());
    std::fs::remove_dir_all(root).unwrap();
}

/// A LOAD PAST ITS STATED SIZE SAYS SO IN ONE `warn` (L-IA-2), naming the
/// journal, its bytes and whole lines and the bound, and reads on; one at
/// the bound exactly is read in silence. The journal here is one whole line
/// and a torn tail that starts a `begin` line, which a reader keeps back
/// unread, so it is read for its size alone. Run in a process of its own
/// with a sink installed, the one install there: the event is emitted
/// through `telemetry::emit` and only a sink keeps it.
#[test]
fn a_load_past_its_stated_size_says_so_in_one_warn() {
    crate::support::where_permission_binds(
        "gdfl_import::journal::cov_tests::a_load_past_its_stated_size_says_so_in_one_warn",
        a_load_past_its_stated_size_says_so_in_one_warn_body,
    );
}

/// The test above, in its child.
fn a_load_past_its_stated_size_says_so_in_one_warn_body() {
    let home = scratch("cov-journal-big-home");
    let sink =
        telemetry::install(&telemetry::Config::new(&home).with_min_level(telemetry::Level::Trace))
            .expect("the only install in this process");
    let root = scratch("cov-journal-big");
    let path = root.join("gdfl.journal");
    let bound = usize::try_from(JOURNAL_LOAD_WARN_BYTES).unwrap();
    let mut text = b"run 7 definition=5\nbegin ".to_vec();
    text.resize(bound + 1, b'x');
    let warned = |path: &Path| -> Vec<telemetry::Record> {
        let records = telemetry::tail(
            &home,
            sink.keep_files(),
            &telemetry::Query::last(telemetry::MAX_LIMIT).from_target(TARGET),
        )
        .records;
        records
            .into_iter()
            .filter(|record| record.message == "journal past its stated load size")
            .filter(|record| {
                record
                    .field("journal")
                    .and_then(telemetry::OwnedValue::as_str)
                    == Some(path.display().to_string().as_str())
            })
            .collect()
    };
    std::fs::write(&path, &text[..bound]).unwrap();
    let at_bound = Journal::read_only(path.clone()).unwrap();
    assert_eq!(at_bound.known.get(), (JOURNAL_LOAD_WARN_BYTES, 1));
    assert!(warned(&path).is_empty(), "at the bound: silent");
    std::fs::write(&path, &text).unwrap();
    let past = Journal::read_only(path.clone()).unwrap();
    assert_eq!(past.known.get(), (JOURNAL_LOAD_WARN_BYTES + 1, 1));
    assert_eq!(past.loaded(), 19, "the whole line, not the tail");
    assert!(past.runs.contains(&7));
    let said = warned(&path);
    assert_eq!(said.len(), 1);
    let count = |key: &str| said[0].field(key).and_then(telemetry::OwnedValue::as_u64);
    assert_eq!(
        (count("bytes"), count("lines"), count("bound")),
        (
            Some(JOURNAL_LOAD_WARN_BYTES + 1),
            Some(1),
            Some(JOURNAL_LOAD_WARN_BYTES)
        )
    );
    assert_eq!(
        said[0]
            .field("limit")
            .and_then(telemetry::OwnedValue::as_str),
        Some("L-IA-2")
    );
    std::fs::remove_dir_all(root).unwrap();
}

//! The rows BASE-STACK-1028 wrote at the data parts' cut point stay as they
//! were written, and each correction of them is a row of its own (D-7412).
//!
//! The five ledgers are append-only (the landing procedure's rule D1 (B)). Every
//! data part and FNO-SEAM were cut from [`CUT`] and append right after this
//! part's rows, so a row rewritten there is one the family's landing tool
//! refuses as "not an append". A repair of this part once rewrote them to
//! correct their figures, and every gate it ran was green: nothing in the
//! workspace read them against the commit the other parts hold. This does.
//!
//! The compare is by lines, in order. A line another part puts between two of
//! these rows is no loss, and a landing may do that; a rewritten, removed or
//! moved row is.
//!
//! One cell is the agreed exception, [`AGREED`] (D-7413): gate 10 reads only a
//! row's own proof cell, so that one row could not be cured by a new row.
//!
//! # Where it runs
//!
//! It asks `git`, by the rule `store/tests/cited_commits.rs` and
//! `core/tests/findings.rs` already hold (D-7414). Where `git` cannot be run,
//! and in a tree with no `.git`, which is what `cargo-mutants` copies, it says
//! which on stderr, past the harness's capture, and compares nothing. Where the
//! tree holds a `.git` that `git` cannot read, it fails with git's own words,
//! and it refuses a shallow clone by name: a skip in either would pass every
//! row unread.
//!
//! A full clone need not hold [`CUT`]. `main`'s squash merge keeps no commit of
//! this branch, so a failure there would turn `main` red for good on the day
//! this lands. Such a clone compares with `main`'s copy of the rows instead, and
//! says so. One whose `main` does not hold them, or does not resolve, fails by
//! name, since nothing in it can say what they were.

#![allow(clippy::expect_used, clippy::indexing_slicing)]

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The commit the data parts and FNO-SEAM were cut from.
const CUT: &str = "bcac15e2c4d9daf8bf4a100ca3f18729c61810e4";

/// What a clone that does not hold [`CUT`] compares with.
const MAIN: &str = "refs/remotes/origin/main";

/// Each ledger, the start of the line this part's group opens with at [`CUT`],
/// and how many lines that group held there: to the end of the file, each
/// being the last group of its ledger at that commit.
const GROUPS: [(&str, &str, usize); 3] = [
    ("docs/04-invariants.md", "| STACK1028-01 |", 7),
    ("docs/05-decisions.md", "### D-7410 ", 83),
    (
        "docs/06-limits.md",
        "## The native search's stack, measured",
        15,
    ),
];

/// The one agreed change to a row of [`GROUPS`] (D-7413): the start of that row,
/// its proof cell as [`CUT`] held it, and that cell as it reads now.
///
/// Only the two cell texts are held, so the row must still read as [`CUT`] held
/// it everywhere else: another cell changed, this cell written any other way, or
/// the row put back as it was are each still a lost row.
const AGREED: (&str, &str, &str) = (
    "| STACK1028-03 |",
    "| the bounded-stack tests of `boolean_grammar_batch_tests.rs` and `index_stop_search_checkpoint_tests.rs` |",
    "| `crates/cli/src/boolean_grammar_batch_tests.rs`: `empty_single_many_and_terminal_batches_replay_exactly_on_a_bounded_stack`, `many_programs_paused_by_nodes_then_resumed_twice_match_one_whole_batch_on_a_bounded_stack`, `concurrent_bounded_stack_replays_of_one_batch_agree_byte_for_byte`; `crates/cli/src/index_stop_search_checkpoint_tests.rs`: `ancestries_of_one_and_two_frames_recover_exactly_on_a_bounded_stack`, `the_deepest_ancestry_is_walked_in_a_loop_and_refused_at_its_oldest_unverified_child`, `a_deep_ancestry_of_full_batches_at_exact_record_and_byte_admission_fits_a_bounded_stack`, `a_pending_reservation_cut_and_reopened_twice_recovers_the_same_bytes_on_a_bounded_stack`, `a_second_owner_is_refused_while_a_deep_recovery_holds_the_journal_and_then_agrees` |",
);

/// The repository root: `crates/cli` is two levels below it.
fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crates/cli is two levels below the repository root")
        .to_owned()
}

/// A tree to read history in: its root, and the environment each `git` there
/// runs under. `None` is this process's own, which is what a real run meets. A
/// fixture names its own, so a `GIT_DIR` it sets reaches nothing else this
/// process runs, and one this process inherits reaches no fixture.
#[derive(Clone, Copy)]
struct Tree<'a> {
    root: &'a Path,
    env: Option<&'a [(&'a str, &'a str)]>,
}

impl Tree<'_> {
    /// `git` in this tree, not yet run. It is fenced by
    /// `GIT_CEILING_DIRECTORIES`, so it cannot walk up past a `.git` it cannot
    /// read to a repository enclosing the tree, and by
    /// `GIT_NO_REPLACE_OBJECTS`, so a replace ref cannot make a commit show a
    /// copy of the rows it never held (check r7, E1).
    fn git(self) -> Command {
        let mut git = Command::new("git");
        if let Some(env) = self.env {
            git.env_clear().envs(env.iter().copied());
        }
        git.arg("-C").arg(self.root);
        git.envs(
            self.root
                .parent()
                .map(|above| ("GIT_CEILING_DIRECTORIES", above)),
        );
        git.env("GIT_NO_REPLACE_OBJECTS", "1");
        git
    }
}

/// What `command` printed and how it ended. [`why_no_history`] saw `git` run.
fn ran(command: &mut Command) -> Output {
    command.output().expect("`git` ran a moment ago")
}

/// Writes what a compare said to `out`. The real run gives it this process's
/// stderr, which the test harness does not capture, so a plain run prints it: a
/// line held back with a passing test's output says nothing to anyone.
fn tell(out: &mut impl Write, said: Option<&str>) {
    if let Some(line) = said {
        writeln!(out, "{line}").expect("the stream takes a line");
    }
}

/// Why `tree`'s history cannot be asked about, or `None` when it can.
///
/// The rule `history_is_at` holds in `store/tests/cited_commits.rs` and
/// `core/tests/findings.rs`. It skips only where `git` cannot be run and where
/// the tree holds no `.git`, read by `symlink_metadata`, so a dangling symlink
/// is one it holds. Where the tree holds one and `git rev-parse --git-dir` fails
/// there, as it does under a `GIT_DIR` naming nothing or with a `.git` file or
/// symlink pointing nowhere, it fails with git's own words. It refuses a shallow
/// clone, where a commit that exists need not resolve.
fn why_no_history(tree: Tree) -> Option<String> {
    if let Err(why) = tree.git().arg("--version").output() {
        return Some(format!("`git` cannot be run here ({why})"));
    }
    let absent = matches!(
        std::fs::symlink_metadata(tree.root.join(".git")),
        Err(why) if why.kind() == std::io::ErrorKind::NotFound
    );
    if absent {
        return Some(format!("{} holds no `.git`", tree.root.display()));
    }
    let asked = ran(tree.git().args(["rev-parse", "--git-dir"]));
    assert!(
        asked.status.success(),
        "{} holds a `.git`, and `git rev-parse --git-dir` failed there, so no row can be \
         compared and a skip would pass them all unread: {}",
        tree.root.display(),
        String::from_utf8_lossy(&asked.stderr).trim()
    );
    let shallow = ran(tree.git().args(["rev-parse", "--is-shallow-repository"]));
    assert_ne!(
        String::from_utf8_lossy(&shallow.stdout).trim(),
        "true",
        "this is a SHALLOW clone, so a commit that exists need not resolve and a skip \
         would pass every row unread. The checkout needs `fetch-depth: 0`."
    );
    None
}

/// Whether `commit` names a commit in `tree`.
fn holds(tree: Tree, commit: &str) -> bool {
    let commit = format!("{commit}^{{commit}}");
    ran(tree
        .git()
        .args(["rev-parse", "--verify", "--quiet", &commit]))
    .status
    .success()
}

/// The commit to read the rows from, and what to say of it: `cut`, or [`MAIN`]
/// in a full clone that does not hold it, as no clone of `main` will after its
/// squash merge.
fn reference<'a>(tree: Tree, cut: &'a str) -> (&'a str, Option<String>) {
    if holds(tree, cut) {
        return (cut, None);
    }
    assert!(
        holds(tree, MAIN),
        "{cut} is not in this clone and {MAIN} does not resolve, so nothing here can say \
         what the rows were. Run `git fetch origin main`; if this clone holds both, the \
         objects `git` reads here are not its own (`GIT_OBJECT_DIRECTORY` or an alternate \
         names another store) (check r7, E3)."
    );
    let said = format!(
        "{cut} is not in this clone, as after `main`'s squash merge: the rows are \
         compared with {MAIN}."
    );
    (MAIN, Some(said))
}

/// `file` as `commit` holds it. `tree` holds `commit`, so a failure here is a
/// damaged clone or a ledger that commit never had, not an absence to step past.
fn copy_at(tree: Tree, commit: &str, file: &str) -> String {
    let shown = ran(tree.git().args(["show", &format!("{commit}:{file}")]));
    assert!(
        shown.status.success(),
        "`git show {commit}:{file}` failed, so those rows cannot be compared and a skip \
         would pass them unread: {}",
        String::from_utf8_lossy(&shown.stderr).trim()
    );
    String::from_utf8_lossy(&shown.stdout).into_owned()
}

/// The lines of `old` from the first that starts with `first` to its end.
fn group<'a>(old: &'a str, first: &str) -> Vec<&'a str> {
    old.lines()
        .skip_while(|line| !line.starts_with(first))
        .collect()
}

/// The lines of `rows` that `now` no longer holds in their order. Lines between
/// them are no loss. Each line of `now` answers for one row, so a row held twice
/// must be there twice.
fn lost<'a>(rows: &[&'a str], now: &str) -> Vec<&'a str> {
    let held: Vec<&str> = now.lines().collect();
    let mut from = 0;
    let mut gone = Vec::new();
    for row in rows {
        match held[from..].iter().position(|line| line == row) {
            Some(at) => from += at + 1,
            None => gone.push(*row),
        }
    }
    gone
}

/// The rows of one of [`GROUPS`] in the ledger's old copy, refused by its file
/// when they are not as many as the group held: a marker that opens nothing
/// would otherwise compare no rows and pass.
fn rows_of<'a>(old: &'a str, (file, first, lines): (&str, &str, usize)) -> Vec<&'a str> {
    let rows = group(old, first);
    assert_eq!(rows.len(), lines, "{file}: the rows this part wrote");
    rows
}

/// The rows of one of [`GROUPS`] in `main`'s copy of its ledger, which goes on
/// past them: the group's own number of lines from its first. A copy holding
/// fewer does not hold the rows, so nothing in this clone can say what they were.
fn rows_main_holds<'a>(old: &'a str, (file, first, lines): (&str, &str, usize)) -> Vec<&'a str> {
    let mut rows = group(old, first);
    assert!(
        rows.len() >= lines,
        "{MAIN}'s {file} does not hold the {lines} lines from {first:?} and this clone does \
         not hold the commit they were cut from, so nothing here can say what they were. \
         Fetch that commit's branch."
    );
    rows.truncate(lines);
    rows
}

/// `row` as it must read now: as it was, unless it is the row `agreed` names,
/// and then with that one cell changed. A row that no longer holds the old cell
/// is left as it is, so it answers for nothing.
fn as_agreed(row: &str, (first, was, now): (&str, &str, &str)) -> String {
    if row.starts_with(first) {
        row.replacen(was, now, 1)
    } else {
        row.to_owned()
    }
}

/// What one ledger's compare found.
#[derive(Debug)]
struct Compared {
    /// The rows lost, each line as it must read now.
    lost: Vec<String>,
    /// What the reader must see: why it compared nothing, or with what.
    said: Option<String>,
}

/// What `tree`'s copy of the group's ledger has lost of what `cut` held. Only a
/// tree [`why_no_history`] excuses compares nothing, and it says so.
fn lost_since(tree: Tree, cut: &str, ledger: (&str, &str, usize)) -> Compared {
    let file = ledger.0;
    if let Some(why) = why_no_history(tree) {
        return Compared {
            lost: Vec::new(),
            said: Some(format!(
                "SKIPPING: {why}, so the rows of {file} are not compared."
            )),
        };
    }
    let (commit, said) = reference(tree, cut);
    let old = copy_at(tree, commit, file);
    let rows = if said.is_none() {
        rows_of(&old, ledger)
    } else {
        rows_main_holds(&old, ledger)
    };
    let rows: Vec<String> = rows.into_iter().map(|row| as_agreed(row, AGREED)).collect();
    let rows: Vec<&str> = rows.iter().map(String::as_str).collect();
    let now = std::fs::read_to_string(tree.root.join(file)).expect("the ledger is readable");
    let lost = lost(&rows, &now).into_iter().map(str::to_owned).collect();
    Compared { lost, said }
}

/// **THIS PART'S ROWS AT THE CUT POINT ARE KEPT BYTE FOR BYTE, AND IN ORDER**,
/// but for the one cell [`AGREED`] names.
///
/// Fails on the repair that rewrote the rows of L-STACK1028-01 to -05, their
/// preamble, D-7410, STACK1028-04 and STACK1028-05 to correct their figures.
#[test]
fn the_rows_at_the_cut_point_are_kept_byte_for_byte() {
    let root = repo();
    let tree = Tree {
        root: &root,
        env: None,
    };
    for ledger in GROUPS {
        let Compared { lost, said } = lost_since(tree, CUT, ledger);
        tell(&mut std::io::stderr(), said.as_deref());
        assert!(lost.is_empty(), "{}: {lost:#?}", ledger.0);
    }
}

/// **EACH FIGURE THAT WENT STALE HAS A NEWER ROW SAYING SO.** A row kept as it
/// was still says what it said, so the correction must stand beside it and name
/// it, or a reader takes the old figure for the current one.
#[test]
fn each_stale_figure_has_a_newer_row_that_names_it() {
    let limits = std::fs::read_to_string(repo().join("docs/06-limits.md")).expect("the limits");
    let invariants =
        std::fs::read_to_string(repo().join("docs/04-invariants.md")).expect("the invariants");
    for (text, row, names, figure) in [
        (&limits, "| L-STACK1028-06 |", "L-STACK1028-04", "690,096 B"),
        (&limits, "| L-STACK1028-06 |", "611,584 B", "1536 KiB"),
        (&limits, "| L-STACK1028-07 |", "L-STACK1028-01", "736 KiB"),
        (&limits, "| L-STACK1028-08 |", "L-STACK1028-02", "224 KiB"),
        (&limits, "| L-STACK1028-09 |", "L-STACK1028-03", "D-7412"),
        (&limits, "| L-STACK1028-09 |", "L-STACK1028-04", "this tip"),
        (&limits, "| L-STACK1028-10 |", "L-STACK1028-05", "D-7412"),
        (&limits, "| L-STACK1028-11 |", "L-STACK1028-01", "12,286 B"),
        (&limits, "| L-STACK1028-14 |", "L-STACK1028-13", "15,000 ms"),
        (&invariants, "| STACK1028-08 |", "STACK1028-04", "736 KiB"),
        (&invariants, "| STACK1028-09 |", "STACK1028-05", "690,096 B"),
        (&invariants, "| STACK1028-14 |", "STACK1028-06", "22 cases"),
        (&invariants, "| STACK1028-18 |", "STACK1028-17", "15,000 ms"),
    ] {
        let line = text
            .lines()
            .find(|line| line.starts_with(row))
            .expect("the correcting row exists");
        assert!(line.contains(names), "{row} names {names}");
        assert!(line.contains(figure), "{row} states {figure}");
    }
}

/// The ledger of the fixtures: its file, the start of its group, its lines.
const LEDGER: (&str, &str, usize) = ("ledger.md", "| A |", 3);

/// The ledger as a fixture's cut commit holds it.
const AT_CUT: &str = "before\n| A | one |\n| B | two |\n| C | three |\n";

/// The row [`rewrite`] changes, which no compare may pass.
const ROW: &str = "| B | two |";

/// A commit no fixture holds.
const ABSENT: &str = "0000000000000000000000000000000000000000";

/// What a tree holding a `.git` that `git` cannot read fails with.
const UNREAD: &str = "holds a `.git`, and `git rev-parse --git-dir` failed there";

/// One synthetic history for the compare to read: a row appended after the cut.
struct History {
    root: PathBuf,
    /// The commit that wrote [`AT_CUT`].
    cut: String,
    /// The ledger's hash at `cut`: an object of this history, and no commit.
    blob: String,
}

/// A scratch tree root under this test's own target directory, emptied.
fn scratch_root(tag: &str) -> PathBuf {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("stack-rows-{tag}-{}", std::process::id()));
    let _stale = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("a scratch root");
    root
}

/// What the fixtures' `git` runs under: the `PATH` that finds it, no outside
/// config, then `more`, which wins.
fn fixture_env<'a>(path: &'a str, more: &[(&'a str, &'a str)]) -> Vec<(&'a str, &'a str)> {
    let mut env = vec![
        ("PATH", path),
        ("GIT_CONFIG_NOSYSTEM", "1"),
        ("GIT_CONFIG_GLOBAL", "/dev/null"),
    ];
    env.extend_from_slice(more);
    env
}

/// This process's `PATH`, for [`fixture_env`].
fn path() -> String {
    std::env::var("PATH").expect("this process has a PATH")
}

/// A fixture command: local identity, no hooks or signing, no outside config.
fn fixture_git(root: &Path, args: &[&str]) -> String {
    let path = path();
    let env = fixture_env(&path, &[]);
    let tree = Tree {
        root,
        env: Some(&env),
    };
    let out = ran(tree
        .git()
        .args([
            "-c",
            "user.name=Kept rows test",
            "-c",
            "user.email=kept-rows@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "-c",
            "core.hooksPath=/dev/null",
        ])
        .args(args));
    let said = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "{args:?}: {said}");
    String::from_utf8_lossy(&out.stdout).trim().to_owned()
}

/// Writes the fixture's ledger.
fn write_ledger(root: &Path, text: &str) {
    std::fs::write(root.join(LEDGER.0), text).expect("the fixture's ledger");
}

/// The fixture's ledger with [`ROW`] changed.
fn rewrite(root: &Path) {
    let ledger = std::fs::read_to_string(root.join(LEDGER.0)).expect("the fixture's ledger");
    assert!(ledger.contains(ROW), "{ledger}");
    write_ledger(root, &ledger.replace(ROW, "| B | rewritten |"));
}

/// Commits `text` as the ledger, and makes that commit [`MAIN`].
fn commit_to_main(root: &Path, text: &str) {
    write_ledger(root, text);
    fixture_git(root, &["commit", "-a", "-m", "the ledger on main"]);
    fixture_git(root, &["update-ref", MAIN, "HEAD"]);
}

/// Whether `git` runs here, which a fixture asks before it builds a history.
fn git_runs() -> bool {
    Command::new("git").arg("--version").output().is_ok()
}

/// Runs `body` over a fresh [`History`], or says why it cannot. `git` is what
/// [`git_runs`] answered, so a machine with none is a case of its own rather
/// than a branch no run reaches.
fn with_history(git: bool, tag: &str, body: impl FnOnce(&History)) {
    if !git {
        // On this process's stderr, past the harness's capture, as `tell` writes
        // (check r7: `println!` here was held back with a passing test's output).
        tell(
            &mut std::io::stderr(),
            Some("SKIPPING: git cannot build the synthetic history."),
        );
        return;
    }
    let root = scratch_root(tag);
    fixture_git(&root, &["init", "--initial-branch=main"]);
    write_ledger(&root, AT_CUT);
    fixture_git(&root, &["add", "."]);
    fixture_git(&root, &["commit", "-m", "the cut"]);
    let cut = fixture_git(&root, &["rev-parse", "HEAD"]);
    let blob = fixture_git(&root, &["rev-parse", &format!("HEAD:{}", LEDGER.0)]);
    write_ledger(&root, &format!("{AT_CUT}| D | appended |\n"));
    fixture_git(&root, &["commit", "-a", "-m", "a row appended"]);
    let history = History { root, cut, blob };
    body(&history);
    std::fs::remove_dir_all(&history.root).expect("the fixture is removed");
}

/// What the compare finds of `root`'s fixture ledger, `git` running under `more`.
fn compared(root: &Path, cut: &str, more: &[(&str, &str)]) -> Compared {
    let path = path();
    let env = fixture_env(&path, more);
    let tree = Tree {
        root,
        env: Some(&env),
    };
    lost_since(tree, cut, LEDGER)
}

/// The rows [`compared`] finds lost.
fn compare(root: &Path, cut: &str, more: &[(&str, &str)]) -> Vec<String> {
    compared(root, cut, more).lost
}

/// The line [`compared`] says.
fn said(root: &Path, cut: &str, more: &[(&str, &str)]) -> String {
    compared(root, cut, more)
        .said
        .expect("this compare has something to say")
}

/// What `body` failed with, which it must: passing is the defect these guard.
fn refusal<T: std::fmt::Debug>(body: impl FnOnce() -> T + std::panic::UnwindSafe) -> String {
    std::panic::catch_unwind(body)
        .expect_err("this must fail, not pass")
        .downcast_ref::<String>()
        .cloned()
        .unwrap_or_default()
}

/// Fails unless `said` holds each of `words`.
fn assert_says(said: &str, words: &[&str]) {
    for word in words {
        assert!(said.contains(word), "{word:?} in {said}");
    }
}

/// What [`compare`] failed with, which it must.
fn refused(root: &Path, cut: &str, more: &[(&str, &str)]) -> String {
    refusal(|| compare(root, cut, more))
}

/// **WHERE `git` ANSWERS, A REWRITTEN ROW IS NAMED.** The control of the tests
/// below it: a row appended after the cut is no loss.
#[test]
fn a_rewritten_row_is_named_where_git_answers() {
    with_history(git_runs(), "answers", |history| {
        let root = &history.root;
        assert!(compare(root, &history.cut, &[]).is_empty());
        rewrite(root);
        assert_eq!(compare(root, &history.cut, &[]), [ROW]);
    });
}

/// **A `GIT_DIR` NAMING NOTHING FAILS WITH GIT'S OWN WORDS.** This passed with
/// every row unread, and a rewritten row with them: any failed `git show` read
/// as nothing lost. A commit that does not resolve and a `main` that holds the
/// rows change nothing, because the tree is refused before either is read.
#[test]
fn a_git_dir_naming_nothing_fails_with_gits_words() {
    with_history(git_runs(), "git-dir", |history| {
        let root = &history.root;
        rewrite(root);
        let nothing = [("GIT_DIR", "/nonexistent/x")];
        for commit in [&*history.cut, ABSENT] {
            let said = refused(root, commit, &nothing);
            assert_says(&said, &[UNREAD, "not a git repository"]);
        }
    });
}

/// **A `.git` THE TREE CANNOT READ FAILS WITH GIT'S OWN WORDS**: a file pointing
/// nowhere, a symlink naming nothing and an empty folder, which `git` passes over
/// to look further up unless it is fenced. Each holds a rewritten row.
#[test]
fn a_git_the_tree_cannot_read_fails_with_gits_words() {
    with_history(git_runs(), "unreadable", |history| {
        let root = &history.root;
        rewrite(root);
        let git = root.join(".git");
        let nowhere = root.join("nowhere");
        std::fs::rename(&git, root.join("moved.git")).expect("the history moved aside");

        std::fs::write(&git, format!("gitdir: {}\n", nowhere.display())).expect("a `.git` file");
        let said = refused(root, &history.cut, &[]);
        assert_says(&said, &[UNREAD, "not a git repository"]);
        std::fs::remove_file(&git).expect("the `.git` file is removed");

        std::os::unix::fs::symlink(&nowhere, &git).expect("a `.git` symlink naming nothing");
        let said = refused(root, &history.cut, &[]);
        assert_says(&said, &[UNREAD, "not a git repository"]);
        std::fs::remove_file(&git).expect("the `.git` symlink is removed");

        std::fs::create_dir(&git).expect("an empty `.git`");
        let said = refused(root, &history.cut, &[]);
        assert_says(&said, &[UNREAD, "not a git repository"]);
    });
}

/// **A SHALLOW CLONE FAILS BY NAME, WHETHER OR NOT IT HOLDS THE COMMIT.** A
/// depth-1 clone holds its tip alone, so this passed with a row rewritten, and
/// a commit it happens to hold or a `main` holding the rows does not excuse it.
#[test]
fn a_shallow_clone_fails_by_name() {
    with_history(git_runs(), "shallow", |history| {
        let clone = scratch_root("shallow-clone");
        let source = format!("file://{}", history.root.display());
        fixture_git(&clone, &["clone", "--depth", "1", &source, "."]);
        rewrite(&clone);
        let tip = fixture_git(&clone, &["rev-parse", "HEAD"]);
        fixture_git(&clone, &["update-ref", MAIN, "HEAD"]);
        for commit in [&*history.cut, &*tip, ABSENT] {
            let said = refused(&clone, commit, &[]);
            assert!(said.contains("SHALLOW clone"), "{said}");
        }
        std::fs::remove_dir_all(&clone).expect("the clone is removed");
    });
}

/// **A FULL CLONE WITHOUT THE COMMIT COMPARES WITH `main`'S COPY.** Such a clone
/// is what `main`'s squash merge leaves, so failing here would turn `main` red
/// for good, and comparing nothing would be the skip this file refuses. A
/// rewritten row is named. So it is when the commit given is a blob's hash, an
/// object that names no commit. What `main` holds past the group's own lines is
/// no row of this group.
#[test]
fn a_clone_without_the_commit_compares_with_mains_copy() {
    with_history(git_runs(), "no-cut", |history| {
        let root = &history.root;
        fixture_git(root, &["update-ref", MAIN, "HEAD"]);
        assert!(compare(root, ABSENT, &[]).is_empty());
        assert!(said(root, ABSENT, &[]).contains("the rows are compared with"));
        write_ledger(root, AT_CUT);
        assert!(compare(root, ABSENT, &[]).is_empty());
        rewrite(root);
        assert_eq!(compare(root, ABSENT, &[]), [ROW]);
        assert_eq!(compare(root, &history.blob, &[]), [ROW]);
        assert_eq!(compared(root, &history.cut, &[]).said, None);
    });
}

/// **A FULL CLONE WITH NEITHER THE COMMIT NOR `main` FAILS BY NAME**, with the
/// fetch that fixes it: nothing in it can say what the rows were.
#[test]
fn a_clone_with_neither_the_commit_nor_main_fails_by_name() {
    with_history(git_runs(), "neither", |history| {
        rewrite(&history.root);
        let said = refused(&history.root, ABSENT, &[]);
        assert_says(&said, &["is not in this clone", "git fetch origin main"]);
    });
}

/// **AN OBJECT STORE POINTED ELSEWHERE IS NAMED BESIDE THE FETCH** (check r7,
/// E3): a clone that holds the commit and `main` reads as one holding neither
/// when `git` reads another store, so the failure names that cause too.
#[test]
fn an_object_store_pointed_elsewhere_is_named_beside_the_fetch() {
    with_history(git_runs(), "elsewhere", |history| {
        let root = &history.root;
        commit_to_main(root, AT_CUT);
        assert!(compare(root, &history.cut, &[]).is_empty());
        let empty = root.join("elsewhere-objects");
        std::fs::create_dir(&empty).expect("an empty object store");
        let store = empty.to_str().expect("the fixture's path is UTF-8");
        let said = refused(root, &history.cut, &[("GIT_OBJECT_DIRECTORY", store)]);
        assert_says(
            &said,
            &[
                "is not in this clone",
                "git fetch origin main",
                "GIT_OBJECT_DIRECTORY",
            ],
        );
    });
}

/// **A REPLACE REF CANNOT STAND FOR THE ROWS AT THE CUT POINT** (check r7, E1):
/// `git replace` makes `git show <cut>:<ledger>` print any copy, one holding the
/// rewritten row among them, and such a copy passes every compare. The
/// compare's `git` reads the commit's own object, so the rewrite is still named.
#[test]
fn a_replace_ref_cannot_stand_for_the_rows_at_the_cut_point() {
    with_history(git_runs(), "replaced", |history| {
        let root = &history.root;
        rewrite(root);
        let forged = AT_CUT.replace(ROW, "| B | rewritten |");
        std::fs::write(root.join("forged.md"), &forged).expect("the forged copy");
        let copy = fixture_git(root, &["hash-object", "-w", "forged.md"]);
        fixture_git(root, &["replace", &history.blob, &copy]);
        // A `git` that honours replace refs now shows the forged copy for the cut.
        let path = path();
        let shown = ran(Command::new("git")
            .env_clear()
            .envs(fixture_env(&path, &[]))
            .arg("-C")
            .arg(root)
            .args(["show", &format!("{}:{}", history.cut, LEDGER.0)]));
        assert_eq!(String::from_utf8_lossy(&shown.stdout), forged);
        assert_eq!(compare(root, &history.cut, &[]), [ROW]);
    });
}

/// **A `main` THAT DOES NOT HOLD THE ROWS FAILS BY NAME**: one line short of the
/// group, and without its first line at all. Such a clone holds a branch that
/// lost the commit, and no copy can say what the rows were.
#[test]
fn a_main_that_does_not_hold_the_rows_fails_by_name() {
    with_history(git_runs(), "main-without", |history| {
        let root = &history.root;
        for text in ["before\n| A | one |\n| B | two |\n", "before\n"] {
            commit_to_main(root, text);
            let said = refused(root, ABSENT, &[]);
            assert!(said.contains("does not hold the 3 lines from"), "{said}");
        }
    });
}

/// **ONLY A TREE WITH NO `.git` SKIPS, AND IT SAYS WHY.** A commit that cannot
/// be shown there compares nothing, as `cargo-mutants`'s copy of the tree must,
/// whatever `GIT_DIR` names, because the rule reads the tree's `.git` first. A
/// row rewritten there passes, and L-STACK1028-12 says so. Anywhere else a commit
/// that cannot be shown fails: the tests above this one drive each case.
#[test]
fn a_commit_that_cannot_be_shown_compares_nothing() {
    let nowhere = Tree {
        root: Path::new("/nonexistent-root-of-no-tree"),
        env: None,
    };
    for ledger in GROUPS {
        let Compared { lost, said } = lost_since(nowhere, CUT, ledger);
        assert!(lost.is_empty(), "{lost:#?}");
        assert!(said.is_some_and(|said| said.starts_with("SKIPPING: ")));
    }
    with_history(git_runs(), "no-dot-git", |history| {
        let root = &history.root;
        rewrite(root);
        std::fs::rename(root.join(".git"), root.join("moved.git")).expect("the history aside");
        for (commit, more) in [
            (&*history.cut, &[][..]),
            (ABSENT, &[("GIT_DIR", "/nonexistent/x")][..]),
        ] {
            assert!(compare(root, commit, more).is_empty());
            let said = said(root, commit, more);
            assert_says(&said, &["SKIPPING: ", "holds no `.git`", LEDGER.0]);
        }
    });
}

/// **A MACHINE WITH NO `git` COMPARES NOTHING AND SAYS WHICH**, whatever else is
/// wrong at once: a `.git` pointing nowhere, a rewritten row, a commit nothing
/// holds. Running nothing, it needs no fixture of `git`'s.
#[test]
fn a_machine_with_no_git_compares_nothing_and_says_which() {
    let root = scratch_root("no-git");
    std::fs::write(root.join(".git"), "gitdir: /nonexistent/x\n").expect("a `.git` file");
    write_ledger(&root, "| A | rewritten |\n");
    let tree = Tree {
        root: &root,
        env: Some(&[("PATH", "/nonexistent")]),
    };
    let Compared { lost, said } = lost_since(tree, ABSENT, LEDGER);
    assert!(lost.is_empty(), "{lost:#?}");
    let said = said.expect("no `git`, so a reason");
    assert!(said.contains("`git` cannot be run here"), "{said}");
    std::fs::remove_dir_all(&root).expect("the scratch root is removed");
}

/// **A `.git` FILE OR A `GIT_DIR` NAMING A REAL HISTORY IS READ.** A linked
/// worktree is such a file, and every package tree here is one, so the refusals
/// above must not catch it.
#[test]
fn a_git_file_or_git_dir_naming_a_real_history_is_read() {
    with_history(git_runs(), "linked", |history| {
        let root = &history.root;
        rewrite(root);
        let moved = root.join("moved.git");
        std::fs::rename(root.join(".git"), &moved).expect("the history moved aside");
        std::fs::write(root.join(".git"), format!("gitdir: {}\n", moved.display()))
            .expect("a `.git` file");
        assert_eq!(compare(root, &history.cut, &[]), [ROW]);
        let named = moved.display().to_string();
        assert_eq!(compare(root, &history.cut, &[("GIT_DIR", &named)]), [ROW]);
    });
}

/// Out of the box: a root that is a file holds no `.git`, yet not by `NotFound`,
/// so it is no tree to skip. A commit the clone holds that lost the ledger's
/// object, and a ledger that commit never had, each fail with git's own words:
/// the old compare read both as nothing lost.
#[test]
fn a_damaged_clone_or_a_root_that_is_no_folder_fails_with_gits_words() {
    with_history(git_runs(), "damaged", |history| {
        let root = &history.root;
        let said = refused(&root.join(LEDGER.0), &history.cut, &[]);
        assert_says(&said, &[UNREAD, "cannot change to"]);

        rewrite(root);
        let path = path();
        let env = fixture_env(&path, &[]);
        let tree = Tree {
            root,
            env: Some(&env),
        };
        let said = refusal(|| copy_at(tree, &history.cut, "no-such-ledger.md"));
        assert_says(&said, &["`git show", "no-such-ledger.md"]);

        let (folder, name) = history.blob.split_at(2);
        let object = root.join(".git/objects").join(folder).join(name);
        std::fs::rename(object, root.join("moved-object")).expect("the object moved aside");
        let said = refused(root, &history.cut, &[]);
        assert_says(&said, &["`git show", "unread: "]);
        assert!(
            !said.ends_with("unread: "),
            "git's own words follow: {said}"
        );
    });
}

/// **A MACHINE WITH NO `git` SAYS SO AND BUILDS NO FIXTURE.**
#[test]
fn a_fixture_on_a_machine_with_no_git_runs_nothing() {
    let ran = std::cell::Cell::new(false);
    with_history(false, "no-history", |_| ran.set(true));
    assert!(!ran.get(), "no history can be built without git");
}

/// **WHAT A COMPARE SAYS REACHES THE STREAM IT IS GIVEN**, and one that has
/// nothing to say writes nothing, so a plain run of a whole tree is silent and a
/// line in it means what it says.
#[test]
fn a_line_a_compare_says_reaches_the_stream_and_silence_writes_nothing() {
    let mut out = Vec::new();
    tell(&mut out, None);
    assert!(out.is_empty());
    tell(&mut out, Some("SKIPPING: a reason"));
    assert_eq!(out, b"SKIPPING: a reason\n");
}

/// The rows of `main`'s copy stop at the group's own lines, and fewer is refused.
#[test]
fn the_rows_main_holds_stop_at_the_groups_own_lines() {
    let old = "before\n| A |\n| B |\n| later |\n";
    assert_eq!(
        rows_main_holds(old, ("a ledger", "| A |", 2)),
        ["| A |", "| B |"]
    );
    assert_eq!(
        rows_main_holds(old, ("a ledger", "| A |", 3)),
        ["| A |", "| B |", "| later |"]
    );
    let said = refusal(|| rows_main_holds(old, ("a ledger", "| A |", 4)));
    assert!(
        said.contains("a ledger does not hold the 4 lines"),
        "{said}"
    );
}

/// **A GROUP OF THE WRONG LENGTH IS REFUSED BY ITS FILE.** One row more or fewer
/// than the group held, and a marker that opens nothing.
#[test]
fn a_group_of_another_length_is_refused() {
    let old = "before\n| A |\n| B |\n";
    assert_eq!(rows_of(old, ("a ledger", "| A |", 2)), ["| A |", "| B |"]);
    for (first, lines) in [("| A |", 3), ("| A |", 1), ("| NO SUCH ROW |", 2)] {
        let refused = std::panic::catch_unwind(|| rows_of(old, ("a ledger", first, lines)))
            .expect_err("the wrong length must fail, not pass");
        let said = refused.downcast_ref::<String>().map_or("", String::as_str);
        assert!(
            said.contains("a ledger: the rows this part wrote"),
            "{said}"
        );
    }
}

/// `group` starts at a line's start, runs to the end, and finds nothing past
/// the end. A marker inside a line opens nothing.
#[test]
fn a_group_runs_from_the_line_its_marker_starts_to_the_end() {
    let text = "before\n| A | one |\nmiddle | A | inside\n| B | two |\n";
    assert_eq!(
        group(text, "| A |"),
        ["| A | one |", "middle | A | inside", "| B | two |"]
    );
    assert_eq!(group(text, "| B |"), ["| B | two |"]);
    assert!(group(text, "| C |").is_empty());
    assert!(group(text, "inside").is_empty());
    assert!(group("", "| A |").is_empty());
}

/// Whatever is kept, appended after or put between is no loss.
#[test]
fn a_row_kept_with_lines_around_it_is_no_loss() {
    let rows = ["| A |", "", "| B |", "| C |"];
    for now in [
        "| A |\n\n| B |\n| C |\n",
        "| A |\n\n| B |\n| C |\n| D, appended |\n",
        "above\n| A |\n\n| B |\n| C |",
        "| A |\n| between |\n\n| between |\n| B |\n| between |\n| C |\n",
    ] {
        assert!(lost(&rows, now).is_empty(), "{now}");
    }
    assert!(lost(&[], "| A |\n").is_empty(), "no rows, nothing to lose");
}

/// Each way of changing a row is a loss, named as the row was.
#[test]
fn a_row_rewritten_removed_or_moved_is_lost() {
    let rows = ["| A | 611,584 B |", "| B |", "| C |"];
    for (now, gone) in [
        ("| A | 690,096 B |\n| B |\n| C |\n", "| A | 611,584 B |"),
        ("| A | 611,584 B |\n| C |\n", "| B |"),
        ("| A | 611,584 B |\n| B | \n| C |\n", "| B |"),
        ("| A | 611,584 B |\n| C |\n| B |\n", "| C |"),
    ] {
        assert_eq!(lost(&rows, now), [gone], "{now}");
    }
    assert_eq!(lost(&rows, ""), rows, "an empty ledger has lost them all");
}

/// A rewritten row, a removed one and another part's row all at once: exactly
/// the two are named, and the rows after them are still found.
#[test]
fn a_rewrite_a_removal_and_an_insertion_at_once_name_exactly_two_rows() {
    let rows = ["| A |", "| B |", "| C |", "| D |"];
    let now = "| A | rewritten |\n| between |\n| C |\n| D |\n| E, appended |\n";
    assert_eq!(lost(&rows, now), ["| A |", "| B |"]);
}

/// A row held twice must be there twice, and a second copy in the wrong place
/// answers for neither.
#[test]
fn a_row_held_twice_must_be_there_twice() {
    let rows = ["", "| A |", ""];
    assert!(lost(&rows, "\n| A |\n\n").is_empty());
    assert_eq!(lost(&rows, "\n| A |\n"), [""]);
    assert_eq!(lost(&rows, "| A |\n\n"), ["| A |", ""]);
}

/// Out of the box: a line ending `\r\n` is the line without its `\r`, as
/// `str::lines` reads it, so a change of line endings alone loses nothing, and a
/// row that only differs by a character outside ASCII does.
#[test]
fn line_endings_lose_nothing_and_one_character_outside_ascii_does() {
    let rows = ["| A | \u{2713} |", "| B |"];
    assert!(lost(&rows, "| A | \u{2713} |\r\n| B |\r\n").is_empty());
    assert_eq!(
        lost(&rows, "| A | \u{2717} |\n| B |\n"),
        ["| A | \u{2713} |"]
    );
}

/// A row shaped as the agreed one, with `claim` and `proof` in its cells.
fn agreed_shape(claim: &str, proof: &str) -> String {
    format!("{} {claim} {proof} \u{2713} |", AGREED.0)
}

/// **THE EXCEPTION CHANGES ONE CELL OF ONE ROW AND NOTHING ELSE.** The claim and
/// the status survive it, and no other row is touched, even one holding that
/// very cell.
#[test]
fn the_agreed_change_touches_one_cell_of_one_row() {
    let (_, was, now) = AGREED;
    assert_eq!(
        as_agreed(&agreed_shape("| a claim", was), AGREED),
        agreed_shape("| a claim", now)
    );
    let other = format!("| STACK1028-02 | a claim {was} \u{2713} |");
    assert_eq!(as_agreed(&other, AGREED), other);
    assert_eq!(as_agreed("", AGREED), "");
    assert_eq!(as_agreed(AGREED.0, AGREED), AGREED.0);
}

/// **ONLY THE AGREED TEXT PASSES.** The row as agreed is no loss; the same row
/// with another cell changed, with its proof written another way, or put back as
/// it was is lost, and is named.
#[test]
fn only_the_agreed_text_of_that_row_passes() {
    let (_, was, now) = AGREED;
    let old = agreed_shape("| a claim", was);
    let must_read = as_agreed(&old, AGREED);
    let rows = [must_read.as_str()];
    assert!(lost(&rows, &agreed_shape("| a claim", now)).is_empty());
    for changed in [
        agreed_shape("| another claim", now),
        agreed_shape("| a claim", "| another proof |"),
        agreed_shape("| a claim", &now.replace("`, `", "`; `")),
        agreed_shape("| a claim", &now[..now.len() - 3]),
        agreed_shape("| a claim", was),
        must_read.replace('\u{2713}', "\u{2717}"),
    ] {
        assert_eq!(lost(&rows, &changed), [must_read.as_str()], "{changed}");
    }
}

/// A second row changed the same way is no exception: naming one row allows one.
#[test]
fn the_same_change_to_any_other_row_is_still_lost() {
    let (_, was, now) = AGREED;
    let other = format!("| STACK1028-04 | a claim {was} \u{2713} |");
    let rows = [as_agreed(&other, AGREED)];
    let rows: Vec<&str> = rows.iter().map(String::as_str).collect();
    let rewritten = other.replace(was, now);
    assert_eq!(lost(&rows, &rewritten), [other.as_str()]);
}

/// Every extreme at once. The compare is in order, so the agreed row moved below
/// the rows after it loses those rows, while a rewrite beside it and another
/// part's row between them change nothing about what is named. Put back as it
/// was, in its place, it is lost itself and the row after the rewrite is found.
#[test]
fn a_moved_or_restored_agreed_row_and_a_rewrite_beside_it_name_exactly_those_rows() {
    let (_, was, _) = AGREED;
    let agreed = as_agreed(&agreed_shape("| a claim", was), AGREED);
    let rows = ["| B | one |", agreed.as_str(), "| C | two |", "| D |"];
    let kept = format!(
        "| between |\n| B | one |\n{agreed}\n| between |\n| C | changed |\n| between |\n| D |\n"
    );
    assert_eq!(lost(&rows, &kept), ["| C | two |"]);
    let moved = format!("| B | one |\n| between |\n| C | changed |\n| D |\n{agreed}\n");
    assert_eq!(lost(&rows, &moved), ["| C | two |", "| D |"]);
    let restored = format!(
        "| B | one |\n{}\n| C | changed |\n| D |\n",
        agreed_shape("| a claim", was)
    );
    assert_eq!(lost(&rows, &restored), [agreed.as_str(), "| C | two |"]);
}

/// Out of the box: an old cell written twice changes only once, so a row that
/// somehow held it twice cannot pass by a blanket replace; and a row that no
/// longer holds the old cell is left as it is and agrees with nothing new.
#[test]
fn the_old_cell_changes_once_and_a_row_without_it_stays() {
    let (_, was, now) = AGREED;
    let twice = agreed_shape("| a claim", &format!("{was} {was}"));
    assert_eq!(
        as_agreed(&twice, AGREED),
        agreed_shape("| a claim", &format!("{now} {was}"))
    );
    let without = agreed_shape("| a claim", "| no such cell |");
    assert_eq!(as_agreed(&without, AGREED), without);
}

/// **THE LEDGER HOLDS THE AGREED ROW ONCE, AS AGREED.** The old cell is gone from
/// it, so gate 10 reads the new one, and each test the cell names is declared
/// in the file the cell names for it.
#[test]
fn the_ledger_holds_the_agreed_row_once_and_its_tests_exist() {
    let (first, was, now) = AGREED;
    let ledger =
        std::fs::read_to_string(repo().join("docs/04-invariants.md")).expect("the invariants");
    let held: Vec<&str> = ledger
        .lines()
        .filter(|line| line.starts_with(first))
        .collect();
    assert_eq!(held.len(), 1, "{first} is one row");
    assert!(held[0].contains(now), "{first} reads as agreed");
    assert!(!held[0].contains(was), "{first} no longer names bare files");
    for files in now.trim_matches(['|', ' ']).split("; ") {
        let (file, tests) = files.split_once(": ").expect("a file, then its tests");
        let source = std::fs::read_to_string(repo().join(file.trim_matches('`')))
            .expect("the file the cell names");
        for test in tests.split(", ") {
            let test = test.trim_matches('`');
            assert!(declares(&source, test), "{file}: {test}");
        }
    }
}

/// Whether `source` declares `test` as a test: `#[test]` alone on a line, then
/// its `fn` on the next. A line comment or a string spelling the name is no
/// declaration. Gate 10 reads the same names from a lexed source; this keeps the
/// row's own test from reading a planted name as one (D-7414).
///
/// No other attribute may stand above its `#[test]`, over blank and comment
/// lines: `#[cfg(any())]` compiles the test out and `#[ignore]` keeps it out of
/// a run, so a test one of them holds proves nothing there (check r7). The
/// tests this part's rows name carry `#[test]` alone.
fn declares(source: &str, test: &str) -> bool {
    let declared = format!("fn {test}(");
    let lines: Vec<&str> = source.lines().map(str::trim).collect();
    lines.iter().enumerate().any(|(at, line)| {
        *line == "#[test]"
            && lines
                .get(at + 1)
                .is_some_and(|next| next.starts_with(&declared))
            && !lines[..at]
                .iter()
                .rev()
                .find(|above| !above.is_empty() && !above.starts_with("//"))
                .is_some_and(|above| above.starts_with("#[") || above.ends_with(']'))
    })
}

/// A name planted in a comment or a string declares no test, and neither does
/// an `fn` that is no `#[test]`, or one whose name only starts the same.
#[test]
fn a_name_planted_in_a_comment_declares_no_test() {
    assert!(declares("#[test]\nfn named() {}\n", "named"));
    assert!(declares(
        "mod t {\n    #[test]\n    fn named() {\n",
        "named"
    ));
    for source in [
        "// fn named() {}\n",
        "/// fn named() {}\n",
        "#[test]\n// fn named() {}\n",
        "#[test]\n/// fn named() {}\nfn other() {}\n",
        "#[test]\nconst SAID: &str = \"fn named()\";\n",
        "fn named() {}\n",
        "#[test]\nfn named_more() {}\n",
        "#[test]\nfn other() { named() }\n",
        "#[test]\n",
        "",
    ] {
        assert!(!declares(source, "named"), "{source}");
    }
}

/// **A TEST AN ATTRIBUTE CAN KEEP OUT OF A RUN IS NOT DECLARED** (check r7):
/// `#[cfg(any())]`, `#[ignore]`, `#[cfg_attr(..)]` or any other attribute above
/// its `#[test]`, over blank and comment lines, on one line or over several. One
/// with only comments, blank lines or the end of the item before it is declared.
#[test]
fn a_test_an_attribute_can_keep_out_of_a_run_is_not_declared() {
    for source in [
        "#[cfg(any())]\n#[test]\nfn named() {}\n",
        "#[ignore]\n#[test]\nfn named() {}\n",
        "#[cfg_attr(unix, ignore)]\n#[test]\nfn named() {}\n",
        "#[cfg(any())] // kept out\n#[test]\nfn named() {}\n",
        "#[cfg(any())]\n\n// a note\n/// a doc line\n#[test]\nfn named() {}\n",
        "#[cfg(\n    any()\n)]\n#[test]\nfn named() {}\n",
        "#[allow(dead_code)]\n#[test]\nfn named() {}\n",
    ] {
        assert!(!declares(source, "named"), "{source}");
    }
    for source in [
        "fn before() {}\n\n/// a doc line\n// a note\n#[test]\nfn named() {}\n",
        "#[cfg(any())]\nfn before() {}\n\n#[test]\nfn named() {}\n",
        "mod t {\n    #[test]\n    fn named() {}\n}\n",
        "#[test]\nfn named() {}\n",
    ] {
        assert!(declares(source, "named"), "{source}");
    }
}

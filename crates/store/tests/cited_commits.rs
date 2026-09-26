//! Every commit the store's records cite is on `main`, or is listed here with
//! the reason it is not, and every sentence citing it gives that reason.
//!
//! # The defect
//!
//! The entry above D-0693, the `.grk` libm key's, first cited a commit for its
//! reading of the store that was on no branch anyone could fetch, and its own
//! correction says so. D-0693 keyed its list of the bracket lock sites it left
//! unchanged to the same commit, and AF-47, AF-48 and `tail_proof.rs` cited
//! the commits of a piece branch whose work this branch carries only as
//! cherry-picks, with other hashes. A measurement cited to a commit no reader
//! can check out cannot be checked (`CLAUDE.md` §3 rule 6). `libm_key.rs`
//! refuses the libm-key entry's withdrawn citations by their wordings; this
//! refuses the class in the texts it reads.
//!
//! This file first accepted a commit on any remote-tracking branch, and so
//! this branch's own commits, which the texts then cited as its copies of
//! those measurements. `main` takes squash merges (`auto-merge.yml` runs
//! `gh pr merge` with `--squash`), so none of those commits ever becomes an
//! ancestor of `main`. After the merge they would pass only while this
//! branch stayed on the remote: a review ran the first version on a squash of
//! this branch onto `main`, and it refused five citations there.
//!
//! # What it reads
//!
//! The D-0693 and D-0694 entries of `docs/05-decisions.md`, the whole of
//! `docs/04-invariants.md` and `docs/06-limits.md`, and the comment lines of
//! `crates/store/tests/tail_proof.rs`. Each is read as sentences, split after
//! a full stop that a space follows, with every run of whitespace made one
//! space and the comment markers `//!`, `///` and `//` dropped, so a phrase a
//! comment wraps across lines reads whole.
//!
//! A citation is a word of 7 to 12 hexadecimal characters, or of 40, holding
//! at least one digit and one of `a` to `f`, with no letter, digit or
//! underscore on either side. So a digest of 16 or 64 characters, a date such
//! as `20200816` and a word such as `decade` are not read as one. Nor is a
//! hash of 13 to 39 characters, or one of digits alone or of letters alone:
//! this file checks only the citations it recognises.
//!
//! It does not read the libm-key entry. `libm_key.rs` requires itself to be
//! the only file under `crates/` that names that entry, and that entry's
//! argument that no other test can see the key enforced rests on it.
//!
//! # What passes
//!
//! A citation of an ancestor of `refs/remotes/origin/main`. A squash merge
//! makes no commit of the merged branch an ancestor of it, so a citation of
//! one passes here only as a listed commit, on the branch as it would on
//! `main`, and no other branch has to stay on the remote for a citation to
//! pass.
//! `core/tests/findings.rs` holds its ledger's commits to the same ref for
//! the same reason (D-0680).
//!
//! Any other citation must be one [`NOT_ON_MAIN`] lists, and it is exempt
//! only for the reason listed with it:
//!
//! - [`Off::Neither`], in neither `main`'s history nor the branch being
//!   merged. Every sentence citing it says the commit "is in neither `main`'s
//!   history nor this" entry's, or row's, branch, in the words the libm-key
//!   entry's correction uses. If it resolves, it is no ancestor of `HEAD`.
//! - [`Off::Squashed`], a commit of this change's branch. Every sentence
//!   citing it says that "`main`'s squash merge does not keep" it.
//! - [`Off::MainsOwnText`], cited by text `main` already held, which the
//!   append-only rule never edits, so it cannot be made to say so. Every
//!   sentence citing it is one `main`'s copy of the same file holds, word for
//!   word, so no new sentence can cite it and pass.
//!
//! A listed commit that resolves must not be an ancestor of `main`, and each
//! listed commit must still be cited. A commit that does not resolve, as this
//! branch's own commits need not in a clone of `main`, is checked by its
//! sentences alone.
//!
//! `d_0693_lists_each_bracket_site_at_a_commit_where_its_line_takes_a_lock`
//! reads D-0693's list of bracket sites at the commit the entry keys it to,
//! and requires every listed line to make a lock call there.
//!
//! # Where it runs
//!
//! It asks `git`, as `core/tests/findings.rs` does, and meets the same
//! environments. In a full clone it checks every citation. Where `git` cannot
//! be run, and in a tree with no `.git`, which is what `cargo-mutants`
//! copies, it prints which, and checks only what needs no history: the count
//! of the citations and of D-0693's sites, that each listed commit is cited,
//! and that each sentence citing a listed commit gives its reason's words.
//! Where the tree holds a `.git` and `git rev-parse --git-dir` fails there, it
//! fails with git's own words rather than skipping.
//! It refuses a shallow clone, where no commit resolves and the checkout needs
//! `fetch-depth: 0`, and a clone with no `refs/remotes/origin/main`, with the
//! fetch that fixes it.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Why a commit the texts cite is not on `main`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Off {
    /// In neither `main`'s history nor the branch being merged.
    Neither,
    /// A commit of this change's branch, which the squash merge drops.
    Squashed,
    /// Cited only by sentences `main` already held.
    MainsOwnText,
}

impl Off {
    /// The words every sentence citing such a commit uses, if any.
    const fn says(self) -> Option<&'static str> {
        match self {
            Self::Neither => Some("is in neither `main`'s history nor this"),
            Self::Squashed => Some("`main`'s squash merge does not keep"),
            Self::MainsOwnText => None,
        }
    }
}

/// Every commit the texts cite that `main` does not hold, and why.
const NOT_ON_MAIN: [(&str, Off); 24] = [
    ("224b6760", Off::Neither),
    ("28c362c1", Off::Neither),
    ("da28ae95", Off::Neither),
    ("b1d9ac70", Off::Neither),
    ("7f617f01", Off::Squashed),
    ("9fc9233b", Off::Squashed),
    ("f6716814", Off::Squashed),
    ("fd70a1bd", Off::Squashed),
    ("eecca4da", Off::Squashed),
    ("08a4258", Off::MainsOwnText),
    ("0d4fef13", Off::MainsOwnText),
    ("11feb080", Off::MainsOwnText),
    ("126152a", Off::MainsOwnText),
    ("1f98bb5", Off::MainsOwnText),
    ("219790b7", Off::MainsOwnText),
    ("36f2350b", Off::MainsOwnText),
    ("4b3220b", Off::MainsOwnText),
    ("6076efd", Off::MainsOwnText),
    ("7461f57", Off::MainsOwnText),
    ("79c5e80", Off::MainsOwnText),
    ("a2b912f", Off::MainsOwnText),
    ("a8814e64", Off::MainsOwnText),
    ("eb95996", Off::MainsOwnText),
    ("f046b36", Off::MainsOwnText),
];

/// The history the texts are read against.
const MAIN: &str = "refs/remotes/origin/main";

/// The decisions this file reads, each up to the next entry's heading.
const ENTRIES: [&str; 2] = ["D-0693", "D-0694"];

/// The documents this file reads whole, from the repository root.
const WHOLE: [&str; 2] = ["docs/04-invariants.md", "docs/06-limits.md"];

/// The test file whose comment lines this file reads.
const COMMENTED: &str = "crates/store/tests/tail_proof.rs";

/// The repository root: `crates/store` is two levels below it.
fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crates/store is two levels below the repository root")
        .to_owned()
}

fn read(relative: &str) -> String {
    std::fs::read_to_string(repo().join(relative))
        .unwrap_or_else(|why| panic!("{relative} is readable: {why}"))
}

/// One `git` call in the repository, its trimmed stdout when it succeeds.
///
/// `None` both when `git` answers with a failure and when it cannot be run;
/// [`history_is_here`] tells the two apart before any other call is made.
fn git(args: &[&str]) -> Option<String> {
    git_at(&repo(), args)
}

/// [`git`] in the tree whose root is `root`.
fn git_at(root: &Path, args: &[&str]) -> Option<String> {
    Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .ok()
        .filter(|out| out.status.success())
        .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_owned())
}

/// Whether history can be asked about here, refusing a shallow clone and a
/// clone without [`MAIN`].
fn history_is_here() -> bool {
    history_is_at(&repo())
}

/// Whether history can be asked about in the tree whose root is `root`.
///
/// It skips only where `git` cannot be run and where `root` holds no `.git`,
/// which is where looking up `.git` itself, not what it names, finds nothing.
/// Where `root` holds one and `git rev-parse --git-dir` fails there, as it
/// does under a `GIT_DIR` naming nothing or with a `.git` file or symlink
/// pointing nowhere, it fails with git's own words: a skip there would pass
/// every history check unread. That call is made with
/// `GIT_CEILING_DIRECTORIES` set to `root`'s parent, so `git` does not walk
/// up past a `.git` it cannot read to a repository enclosing `root`. It
/// refuses a shallow clone and a clone without [`MAIN`].
fn history_is_at(root: &Path) -> bool {
    if let Err(why) = Command::new("git").arg("--version").output() {
        println!("SKIPPING: `git` cannot be run here ({why}), so no commit can be resolved.");
        return false;
    }
    let absent = matches!(
        std::fs::symlink_metadata(root.join(".git")),
        Err(why) if why.kind() == std::io::ErrorKind::NotFound
    );
    if absent {
        println!(
            "SKIPPING: {} holds no `.git`, so no commit can be resolved. This is \
             what `cargo-mutants` looks like: it copies the tree without `.git`.",
            root.display()
        );
        return false;
    }
    let mut ask = Command::new("git");
    ask.arg("-C").arg(root).args(["rev-parse", "--git-dir"]);
    if let Some(above) = root.parent() {
        ask.env("GIT_CEILING_DIRECTORIES", above);
    }
    let asked = ask.output().expect("`git` ran a moment ago");
    assert!(
        asked.status.success(),
        "{} holds a `.git`, and `git rev-parse --git-dir` failed there, so no commit \
         can be resolved and a skip would pass every history check unread: {}",
        root.display(),
        String::from_utf8_lossy(&asked.stderr).trim()
    );
    assert_ne!(
        git_at(root, &["rev-parse", "--is-shallow-repository"]).as_deref(),
        Some("true"),
        "this is a SHALLOW clone, so no cited commit resolves and this test would \
         refuse commits that exist. The checkout needs `fetch-depth: 0`."
    );
    assert!(
        resolves_at(root, MAIN),
        "{MAIN} does not resolve, so no citation can be checked against the history a \
         squash merge keeps. Run `git fetch origin main`."
    );
    true
}

/// A scratch tree root under this test's own target directory, emptied.
fn scratch_root(tag: &str) -> PathBuf {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("cited-commits-{tag}-{}", std::process::id()));
    let _stale = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("a scratch root");
    root
}

/// **A TREE WITH NO `.git` SKIPS THE HISTORY, AND A `.git` GIT CANNOT READ
/// FAILS.**
///
/// The skip was once taken whenever `git rev-parse --git-dir` failed, so a
/// work tree whose `.git` git could not read, run under a `GIT_DIR` naming
/// nothing, printed that it was no git work tree and passed with every
/// history check skipped. A `.git` file pointing at a directory that does not
/// exist is the same refusal, made without touching the environment.
///
/// So is a `.git` symlink naming nothing. `Path::exists` follows a symlink,
/// so the presence check once read that `.git` as absent, printed that the
/// tree held none and passed with every history check skipped.
#[test]
fn only_a_tree_with_no_git_dir_skips_the_history() {
    let bare = scratch_root("bare");
    assert!(
        !history_is_at(&bare),
        "a tree with no `.git` has no history to ask about"
    );
    std::fs::remove_dir_all(&bare).expect("the scratch root is removed");

    let broken = scratch_root("broken");
    std::fs::write(
        broken.join(".git"),
        format!("gitdir: {}\n", broken.join("nowhere").display()),
    )
    .expect("a `.git` file");
    let refused = std::panic::catch_unwind(|| history_is_at(&broken))
        .expect_err("a `.git` git cannot read must fail, not skip");
    let said = refused.downcast_ref::<String>().map_or("", String::as_str);
    assert!(
        said.contains("holds a `.git`, and `git rev-parse --git-dir` failed there")
            && said.contains("not a git repository"),
        "the failure names the tree's `.git` and carries git's words: {said}"
    );
    std::fs::remove_dir_all(&broken).expect("the scratch root is removed");

    let dangling = scratch_root("dangling");
    std::os::unix::fs::symlink(dangling.join("nowhere"), dangling.join(".git"))
        .expect("a `.git` symlink naming nothing");
    let refused = std::panic::catch_unwind(|| history_is_at(&dangling))
        .expect_err("a `.git` symlink naming nothing must fail, not skip");
    let said = refused.downcast_ref::<String>().map_or("", String::as_str);
    assert!(
        said.contains("holds a `.git`, and `git rev-parse --git-dir` failed there")
            && said.contains("not a git repository"),
        "the failure names the tree's `.git` and carries git's words: {said}"
    );
    std::fs::remove_dir_all(&dangling).expect("the scratch root is removed");
}

/// Whether `commit` names a commit in this clone.
fn resolves(commit: &str) -> bool {
    resolves_at(&repo(), commit)
}

/// Whether `commit` names a commit in the clone whose root is `root`.
fn resolves_at(root: &Path, commit: &str) -> bool {
    git_at(
        root,
        &[
            "rev-parse",
            "--verify",
            "--quiet",
            &format!("{commit}^{{commit}}"),
        ],
    )
    .is_some()
}

/// Whether `commit` resolves and is an ancestor of `of`.
fn is_ancestor(commit: &str, of: &str) -> bool {
    resolves(commit) && git(&["merge-base", "--is-ancestor", commit, of]).is_some()
}

/// One decision, from its heading up to the next entry's.
fn entry(id: &str) -> String {
    let decisions = read("docs/05-decisions.md");
    let heading = format!("\n### {id} ");
    let start = decisions
        .find(&heading)
        .unwrap_or_else(|| panic!("{id} has a heading"));
    let body = &decisions[start + 1..];
    let end = body
        .get(1..)
        .and_then(|after| after.find("\n### D-"))
        .map_or(body.len(), |at| at + 1);
    body[..end].to_owned()
}

/// The text with every run of whitespace, line breaks included, made one space.
fn collapse(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The comment markers a comment line starts with, which a wrapped phrase
/// would otherwise carry in its middle.
const MARKERS: [&str; 3] = ["//!", "///", "//"];

/// The text's words, the comment markers dropped, joined by one space.
fn words(text: &str) -> String {
    text.split_whitespace()
        .filter(|word| !MARKERS.contains(word))
        .collect::<Vec<_>>()
        .join(" ")
}

/// The text's sentences: its [`words`], split after a full stop that a space
/// follows.
fn sentences(text: &str) -> Vec<String> {
    words(text).split(". ").map(str::to_owned).collect()
}

/// The comment lines of a Rust source.
fn comments(source: &str) -> String {
    source
        .lines()
        .filter(|line| line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Every citation in one sentence, as the module header defines one.
fn citations(sentence: &str) -> Vec<String> {
    let mut found = Vec::new();
    let bytes = sentence.as_bytes();
    let mut at = 0;
    while at < bytes.len() {
        if !bytes[at].is_ascii_alphanumeric() && bytes[at] != b'_' {
            at += 1;
            continue;
        }
        let start = at;
        while at < bytes.len() && (bytes[at].is_ascii_alphanumeric() || bytes[at] == b'_') {
            at += 1;
        }
        let word = &sentence[start..at];
        let hex = word
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
        let sized = (7..=12).contains(&word.len()) || word.len() == 40;
        let mixed = word.bytes().any(|b| b.is_ascii_digit())
            && word.bytes().any(|b| (b'a'..=b'f').contains(&b));
        if hex && sized && mixed {
            found.push(word.to_owned());
        }
    }
    found
}

/// One text this file reads.
struct Text {
    /// The name a failure gives it.
    name: String,
    /// The file it is read from, from the repository root.
    file: &'static str,
    /// What is read of that file.
    body: String,
}

/// Every text this file reads.
fn texts() -> Vec<Text> {
    let mut texts: Vec<Text> = ENTRIES
        .iter()
        .map(|id| Text {
            name: (*id).to_owned(),
            file: "docs/05-decisions.md",
            body: entry(id),
        })
        .collect();
    texts.extend(WHOLE.iter().map(|path| Text {
        name: (*path).to_owned(),
        file: path,
        body: read(path),
    }));
    texts.push(Text {
        name: format!("{COMMENTED}'s comments"),
        file: COMMENTED,
        body: comments(&read(COMMENTED)),
    });
    texts
}

/// One sentence citing a commit, and where it was read.
struct Citing {
    name: String,
    file: &'static str,
    sentence: String,
}

/// Every citation the texts make, each with every sentence making it.
fn cited(texts: &[Text]) -> BTreeMap<String, Vec<Citing>> {
    let mut cited: BTreeMap<String, Vec<Citing>> = BTreeMap::new();
    for text in texts {
        for sentence in sentences(&text.body) {
            for commit in citations(&sentence) {
                cited.entry(commit).or_default().push(Citing {
                    name: text.name.clone(),
                    file: text.file,
                    sentence: sentence.clone(),
                });
            }
        }
    }
    cited
}

/// Why `commit` is not on `main`, if [`NOT_ON_MAIN`] lists it.
fn listed(commit: &str) -> Option<Off> {
    NOT_ON_MAIN
        .iter()
        .find(|(listed, _)| *listed == commit)
        .map(|(_, off)| *off)
}

/// What the listing claims that can be read without history: each listed
/// commit is cited, and every sentence citing it says why it is not on
/// `main`, where its reason has words.
fn listing_faults(cited: &BTreeMap<String, Vec<Citing>>) -> Vec<String> {
    let mut faults = Vec::new();
    for (commit, off) in NOT_ON_MAIN {
        let Some(citing) = cited.get(commit) else {
            faults.push(format!(
                "{commit} is listed as {off:?}, and no text cites it"
            ));
            continue;
        };
        let Some(says) = off.says() else { continue };
        for at in citing.iter().filter(|at| !at.sentence.contains(says)) {
            faults.push(format!(
                "{commit} is cited in {} by a sentence that does not say \"{says}\": {}",
                at.name, at.sentence
            ));
        }
    }
    faults
}

/// What needs history: each citation not listed is on `main`, and each listed
/// one is as its reason says.
fn history_faults(cited: &BTreeMap<String, Vec<Citing>>) -> Vec<String> {
    let mut faults = Vec::new();
    let mut mains: BTreeMap<&str, String> = BTreeMap::new();
    for (commit, citing) in cited {
        let Some(off) = listed(commit) else {
            if !is_ancestor(commit, MAIN) {
                let names: BTreeSet<&str> = citing.iter().map(|at| at.name.as_str()).collect();
                faults.push(format!(
                    "{commit}, cited in {names:?}, is not an ancestor of {MAIN} and \
                     NOT_ON_MAIN does not list it"
                ));
            }
            continue;
        };
        if is_ancestor(commit, MAIN) {
            faults.push(format!("{commit} is listed as {off:?}, and it is on main"));
        }
        if off == Off::Neither && is_ancestor(commit, "HEAD") {
            faults.push(format!(
                "{commit} is listed as {off:?}, and it is an ancestor of HEAD"
            ));
        }
        if off != Off::MainsOwnText {
            continue;
        }
        for at in citing {
            let main = mains.entry(at.file).or_insert_with(|| {
                let copy = git(&["show", &format!("{MAIN}:{}", at.file)]).unwrap_or_default();
                words(&if at.file == COMMENTED {
                    comments(&copy)
                } else {
                    copy
                })
            });
            if !main.contains(&at.sentence) {
                faults.push(format!(
                    "{commit} is listed as {off:?}, and {} cites it in a sentence \
                     main's copy of {} does not hold: {}",
                    at.name, at.file, at.sentence
                ));
            }
        }
    }
    faults
}

/// **EVERY COMMIT THE STORE'S RECORDS CITE IS ON `main`, OR IS LISTED WITH
/// THE REASON IT IS NOT, AND EVERY SENTENCE CITING IT GIVES THAT REASON.**
///
/// Fails on D-0693's list keyed to a commit on no pushed branch, and on the
/// AF-47, AF-48 and `tail_proof.rs` citations of commits this branch carries
/// only as cherry-picks, each cited without saying so. Fails too on a
/// citation of this branch's own commits that does not say the squash merge
/// drops them, which the first version of this file passed while the branch
/// was on the remote.
#[test]
fn every_commit_the_store_records_cite_is_on_main_or_says_why_not() {
    let cited = cited(&texts());
    assert!(
        cited.len() >= 20,
        "the texts cite {} commits, and they cited at least 20 when this was \
         written: a reader that finds none is reading nothing",
        cited.len()
    );
    let mut faults = listing_faults(&cited);
    if history_is_here() {
        faults.extend(history_faults(&cited));
    }
    assert!(
        faults.is_empty(),
        "cite a commit on main, or list it in NOT_ON_MAIN and say why in every \
         sentence citing it: {faults:#?}"
    );
}

/// D-0693's bracket sites: each module and the lines listed for it.
fn bracket_sites(entry: &str) -> (String, Vec<(String, usize)>) {
    let text = collapse(entry);
    let list = text
        .split_once("The sites are:")
        .expect("D-0693 lists its bracket sites")
        .1;
    let (list, rest) = list
        .split_once("Line numbers are as of `")
        .expect("D-0693 says which commit its line numbers are from");
    let commit = rest
        .split_once('`')
        .expect("the commit is in backticks")
        .0
        .to_owned();
    let mut sites = Vec::new();
    let (mut module, mut family) = (String::new(), String::new());
    for token in list.split_whitespace() {
        let token = token.trim_end_matches([',', ';', '.']);
        if let Some(name) = token.strip_prefix('`') {
            module.clear();
            module.push_str(name.trim_end_matches('`'));
            family.clone_from(&module);
        } else if token.len() == 2 && token.starts_with('v') {
            let stem = family
                .rsplit_once("_v")
                .map_or(family.as_str(), |(stem, _)| stem);
            module = format!("{stem}_{token}");
        } else if !token.is_empty() && token.bytes().all(|b| b.is_ascii_digit() || b == b'/') {
            for line in token.split('/') {
                sites.push((module.clone(), line.parse().expect("a line number")));
            }
        }
    }
    (commit, sites)
}

/// The calls that take an advisory file lock at D-0693's bracket sites: the
/// four `File` lock calls, and `candidate_trades`'s own `lock_for_read`.
const LOCK_CALLS: [&str; 5] = [
    ".lock()",
    ".lock_shared()",
    ".try_lock()",
    ".try_lock_shared()",
    "lock_for_read(",
];

/// Whether the code of `line`, before any `//`, makes one of [`LOCK_CALLS`].
///
/// It reads the call's shape, not its receiver's type, so a `Mutex`'s
/// `.lock()` would pass. What it refuses is a line that only mentions a lock:
/// a comment, a word such as "clock" or "block", or `.unlock()`, none of
/// which makes a call above.
fn takes_a_lock(line: &str) -> bool {
    let code = line.split_once("//").map_or(line, |(code, _)| code);
    LOCK_CALLS.iter().any(|call| code.contains(call))
}

/// **A LINE THAT ONLY MENTIONS A LOCK IS NOT A LOCK SITE.**
///
/// The check below once accepted any line holding "lock" and not "unlock".
/// Lines 13 and 192 of `result_set.rs` at `96194c11` are comments, holding
/// "block" and "clock", and passed it as sites. So would the comment "Unlocked
/// on both paths", whose capital escaped the "unlock" test.
#[test]
fn a_comment_or_a_word_holding_lock_is_not_a_lock_call() {
    for mention in [
        "//! parent, receipt, and the receipt's exact block count. An interrupted receipt",
        "/// Unix and Windows expose a stable file identity plus a change clock. Other",
        "        // Unlocked on both paths, including the refusals inside `append_locked`.",
        "        file.unlock()?;",
        "        let blocked = clock.tick(); // then .lock() the file",
    ] {
        assert!(!takes_a_lock(mention), "not a lock call: {mention}");
    }
    for call in [
        "            .lock()",
        "            .lock_shared()",
        "        let writer_observed = match owner.try_lock_shared() {",
        "        match file.try_lock() {",
        "        lock_for_read(&self.file)?;",
    ] {
        assert!(takes_a_lock(call), "a lock call: {call}");
    }
}

/// **D-0693 LISTS EACH BRACKET SITE AT A COMMIT WHERE ITS LINE TAKES A LOCK.**
///
/// The list was keyed to a commit on no pushed branch, and on this branch the
/// conversion had moved some of its lines off their lock calls. It is keyed
/// now to `main`'s `96194c11`, where every module it names is byte-identical
/// to the commit the list was taken at. This reads each listed line at the
/// commit the entry names, requires its code to make one of [`LOCK_CALLS`],
/// and requires the entry to give the count it reads.
#[test]
fn d_0693_lists_each_bracket_site_at_a_commit_where_its_line_takes_a_lock() {
    let text = entry("D-0693");
    let (commit, sites) = bracket_sites(&text);
    assert!(
        collapse(&text).contains(&format!("The list names {} lines", sites.len())),
        "D-0693 must give the number of lines its list names, {}",
        sites.len()
    );
    if !history_is_here() {
        return;
    }
    assert!(
        is_ancestor(&commit, MAIN),
        "D-0693 keys its line numbers to {commit}, which is not in {MAIN}'s history"
    );
    let mut sources: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut wrong = Vec::new();
    for (module, number) in &sites {
        let lines = sources.entry(module.clone()).or_insert_with(|| {
            git(&["show", &format!("{commit}:crates/cli/src/{module}.rs")])
                .unwrap_or_else(|| panic!("{module}.rs exists at {commit}"))
                .lines()
                .map(str::to_owned)
                .collect()
        });
        let line = lines.get(number - 1).map_or("", String::as_str);
        if !takes_a_lock(line) {
            wrong.push(format!("{module}.rs:{number}: {}", line.trim()));
        }
    }
    assert!(
        wrong.is_empty(),
        "at {commit} these listed lines take no lock: {wrong:#?}"
    );
}

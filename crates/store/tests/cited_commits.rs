//! Every commit the store's records cite is one a reader can fetch.
//!
//! # The defect
//!
//! The entry above D-0693, the `.grk` libm key's, first cited a commit for its
//! reading of the store that was on no branch anyone could fetch, and its own
//! correction says so. D-0693 keyed its list of the bracket lock sites it left
//! unchanged to the same commit, and AF-47, AF-48 and `tail_proof.rs` cited
//! the commits of a piece branch whose work this branch carries only as
//! cherry-picks, with other hashes. `main` lands a change as one squashed
//! commit, so a commit that is neither in the history being merged nor on a
//! pushed branch is one no reader can check out, and a measurement cited to
//! it cannot be checked (`CLAUDE.md` §3 rule 6). `libm_key.rs` refuses the
//! libm-key entry's withdrawn citations by their wordings; this refuses the
//! class in the texts it reads.
//!
//! # What it reads
//!
//! The D-0693 and D-0694 entries of `docs/05-decisions.md`, the whole of
//! `docs/04-invariants.md` and `docs/06-limits.md`, and
//! `crates/store/tests/tail_proof.rs`. A citation is a word of 7 to 12
//! hexadecimal characters, or of 40, holding at least one digit and one of
//! `a` to `f`, with no letter, digit or underscore on either side. So a
//! digest of 16 or 64 characters, a date such as `20200816` and a word such
//! as `decade` are not read as one.
//!
//! It does not read the libm-key entry. `libm_key.rs` requires itself to be
//! the only file under `crates/` that names that entry, and that entry's
//! argument that no other test can see the key enforced rests on it.
//!
//! # What passes
//!
//! A commit that is an ancestor of `HEAD`, or of a remote-tracking branch: the
//! history being merged, or a branch that was pushed. A citation that is
//! neither passes only in a sentence that says so, in the words the libm-key
//! entry's correction uses: the commit "is in neither `main`'s history nor
//! this" entry's, or row's, branch.
//!
//! On `main`, after a squash merge, `HEAD` no longer holds this branch's
//! commits, so one of them cited here passes only while a remote-tracking
//! branch in that clone still holds it.
//!
//! The second test reads D-0693's list of bracket sites at the commit the
//! entry keys it to, and requires every listed line to take a lock there.
//!
//! # Where it runs
//!
//! It asks `git`, as `core/tests/findings.rs` does, and meets the same three
//! environments: a full clone, where it checks every citation; a tree with no
//! `.git`, which is what `cargo-mutants` copies, where it prints why and
//! checks nothing; and a shallow clone, where it refuses, because there no
//! commit resolves and the checkout needs `fetch-depth: 0`.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

/// The words a sentence uses to say it cites a commit no reader can fetch.
const DISOWNED: &str = "is in neither `main`'s history nor this";

/// The decisions this file reads, each up to the next entry's heading.
const ENTRIES: [&str; 2] = ["D-0693", "D-0694"];

/// The files this file reads whole, from the repository root.
const WHOLE: [&str; 3] = [
    "docs/04-invariants.md",
    "docs/06-limits.md",
    "crates/store/tests/tail_proof.rs",
];

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
fn git(args: &[&str]) -> Option<String> {
    Command::new("git")
        .arg("-C")
        .arg(repo())
        .args(args)
        .output()
        .ok()
        .filter(|out| out.status.success())
        .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_owned())
}

/// Whether history can be asked about here, refusing a shallow clone.
fn history_is_here() -> bool {
    if git(&["rev-parse", "--git-dir"]).is_none() {
        println!(
            "SKIPPING: {} is not a git work tree, so no commit can be resolved. This \
             is what `cargo-mutants` looks like: it copies the tree without `.git`.",
            repo().display()
        );
        return false;
    }
    assert_ne!(
        git(&["rev-parse", "--is-shallow-repository"]).as_deref(),
        Some("true"),
        "this is a SHALLOW clone, so no cited commit resolves and this test would \
         refuse commits that exist. The checkout needs `fetch-depth: 0`."
    );
    true
}

/// A reader can fetch it: an ancestor of `HEAD` or of a remote-tracking branch.
fn fetchable(commit: &str) -> bool {
    let object = format!("{commit}^{{commit}}");
    if git(&["rev-parse", "--verify", "--quiet", &object]).is_none() {
        return false;
    }
    git(&["merge-base", "--is-ancestor", commit, "HEAD"]).is_some()
        || git(&[
            "for-each-ref",
            "--count=1",
            "--contains",
            commit,
            "refs/remotes",
        ])
        .is_some_and(|refs| !refs.is_empty())
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

/// The text's sentences: split after a full stop that a space follows.
fn sentences(text: &str) -> Vec<String> {
    collapse(text).split(". ").map(str::to_owned).collect()
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

/// Every text this file reads, by the name a failure gives it.
fn texts() -> Vec<(String, String)> {
    ENTRIES
        .iter()
        .map(|id| ((*id).to_owned(), entry(id)))
        .chain(WHOLE.iter().map(|path| ((*path).to_owned(), read(path))))
        .collect()
}

/// **EVERY COMMIT THE STORE'S RECORDS CITE IS IN A HISTORY A READER CAN FETCH,
/// OR THE SENTENCE CITING IT SAYS IT IS NOT.**
///
/// Fails on D-0693's list keyed to a commit on no pushed branch, and on the
/// AF-47, AF-48 and `tail_proof.rs` citations of commits this branch carries
/// only as cherry-picks, each cited without saying so.
#[test]
fn every_commit_the_store_records_cite_is_in_a_history_a_reader_can_fetch() {
    let (checked, unfetchable) = cited();
    assert!(
        checked.len() >= 20,
        "the texts cite {} commits, and they cited at least 20 when this was \
         written: a reader that finds none is reading nothing",
        checked.len()
    );
    if !history_is_here() {
        return;
    }
    let mut refused = Vec::new();
    for (commit, cited_in) in &unfetchable {
        if !fetchable(commit) {
            let cited_in: Vec<&str> = cited_in.iter().map(String::as_str).collect();
            refused.push(format!("{commit} in {}", cited_in.join(", ")));
        }
    }
    assert!(
        refused.is_empty(),
        "these citations name a commit that is neither an ancestor of HEAD nor on \
         any remote-tracking branch, and their sentence does not say so: {refused:#?}"
    );
}

/// Every citation not disowned in its own sentence, with the texts citing it,
/// and every citation read at all.
fn cited() -> (BTreeSet<String>, BTreeMap<String, BTreeSet<String>>) {
    let mut checked = BTreeSet::new();
    let mut unfetchable: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (name, text) in texts() {
        for sentence in sentences(&text) {
            for commit in citations(&sentence) {
                checked.insert(commit.clone());
                if !sentence.contains(DISOWNED) {
                    unfetchable.entry(commit).or_default().insert(name.clone());
                }
            }
        }
    }
    (checked, unfetchable)
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
        fetchable(&commit),
        "D-0693 keys its line numbers to {commit}, which no reader can fetch"
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

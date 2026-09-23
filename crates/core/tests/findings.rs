//! `docs/11-findings.md`, checked so that a finding cannot be lost.
//!
//! # Why this file exists
//!
//! The 2026-08-11 sweep produced 114 findings. 26 were killed, 99 stood, 37 of those can
//! lose a real combination or invent a false one. That list lived in a published page and
//! in a conversation, and **nothing in the repository tracked what had been done about any
//! of them.**
//!
//! A list with no dispositions cannot answer "did we miss one", which is the only
//! question worth asking of it. Worse, the natural failure is silent: a row gets deleted
//! because it was fixed, or because it was decided against, or by accident, and afterwards
//! there is no way to tell which. `docs/05-decisions.md` already carries the scar of that
//! shape — D-0076, D-0077 and D-0078 were each issued twice and forty citations became
//! ambiguous, recorded in D-0104.
//!
//! So the ledger is append-only and this file enforces the properties that make
//! "nothing was missed" a checkable claim rather than a reassurance:
//!
//! * every row carries a disposition from a closed set — `OPEN` is allowed, silence is not;
//! * the counts in the prose match the rows in the tables;
//! * a `FIXED` row names a commit that exists, is in HEAD's history, and is on `main`;
//! * ids are unique, and derived from the finding's own title rather than its position, so
//!   reordering the table cannot renumber a row and a citation cannot come to mean
//!   something else;
//! * the number of rows never goes DOWN.
//!
//! # What it deliberately does not check
//!
//! Whether a fix is correct. That is what the test beside each fix is for, and what
//! `docs/11-findings.md` says about itself: a `FIXED` row claims a test exists that was
//! **shown to fail** before the fix, which is a smaller claim than correctness and the only
//! one that can be made mechanically.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]

use std::collections::BTreeMap;

/// The ledger. Read at compile time so a rename fails the build rather than skipping.
const LEDGER: &str = include_str!("../../../docs/11-findings.md");

/// The dispositions a row may carry. `OPEN` is a real answer; an empty cell is not.
const DISPOSITIONS: [&str; 6] = [
    "FIXED",
    "PARTLY FIXED",
    "IN PROGRESS",
    "NEEDS A DECISION",
    "OPEN",
    "REFUTED",
];

/// The severities the sweep assigned.
const SEVERITIES: [&str; 4] = ["law", "wrong", "unguarded", "gap"];

/// The number of surviving findings at the commit that introduced this test.
///
/// A floor, never an equality: the ledger is append-only, so a later sweep may add rows
/// and must never remove one. If this number ever needs lowering, a finding was deleted
/// and that is the thing this test exists to prevent.
const ROWS_AT_INTRODUCTION: usize = 99;

/// One parsed row.
struct Row {
    id: String,
    severity: String,
    disposition: String,
}

/// Every finding row in the ledger.
///
/// A row is a table line whose first cell is a backticked `F-` id. The prose and the
/// killed-findings table are skipped by that shape rather than by line number, so
/// reordering the document cannot change what is parsed.
fn rows() -> Vec<Row> {
    let mut out = Vec::new();
    for line in LEDGER.lines() {
        if !line.starts_with("| `F-") {
            continue;
        }
        let cells: Vec<&str> = line.split('|').map(str::trim).collect();
        // ["", id, sev, finding, where, disposition, ""]
        assert!(
            cells.len() >= 6,
            "a finding row has {} cells and needs at least 6 (id, sev, finding, where, \
             disposition): {line}",
            cells.len()
        );
        out.push(Row {
            id: cells[1].trim_matches('`').to_owned(),
            severity: cells[2].trim_matches('`').to_owned(),
            disposition: cells[5].to_owned(),
        });
    }
    out
}

/// Every row carries a disposition from the closed set.
///
/// This is the whole point of the file. A row with an empty disposition cell is a finding
/// nobody has decided anything about, and it is indistinguishable from one that was
/// handled and not written down. `OPEN` is a perfectly good answer — "confirmed, not
/// started" is information. Silence is not.
#[test]
fn every_finding_has_a_disposition() {
    let rows = rows();
    assert!(
        !rows.is_empty(),
        "no finding rows parsed, so this test proves nothing"
    );
    for row in &rows {
        assert!(
            DISPOSITIONS.iter().any(|d| row.disposition.starts_with(d)),
            "finding {} has disposition {:?}, which is not one of {DISPOSITIONS:?}. An \
             empty or unrecognised cell is a finding nobody has decided anything about, \
             and it cannot be told apart from one that was handled and not recorded.",
            row.id,
            row.disposition
        );
    }
}

/// Which disposition a cell carries, by LONGEST matching prefix.
///
/// Longest rather than first-match, so the answer cannot be changed by reordering
/// `DISPOSITIONS`. `PARTLY FIXED` and `FIXED` happen to be unambiguous under
/// `starts_with` today; a future disposition that EXTENDS an existing one would not be,
/// and `every_fixed_finding_names_a_commit` already records being bitten once by a check
/// that narrowed silently as this vocabulary grew.
fn disposition_kind(cell: &str) -> Option<&'static str> {
    DISPOSITIONS
        .iter()
        .filter(|d| cell.starts_with(**d))
        .max_by_key(|d| d.len())
        .copied()
}

/// The disposition tally is stated in the document, so a mass flip is a visible diff.
///
/// # What this catches, and it was demonstrated rather than imagined
///
/// An adversarial audit flipped **all 80 `OPEN` rows to `FIXED f5874ca`** and ran the
/// suite. Zero failures. The row digest was byte-identical, the section counts were
/// untouched, `FIXED` is a known disposition, seven hex characters is a commit, and
/// `f5874ca` resolves. The prose still said "99 stood".
///
/// Widening the digest is the wrong repair. Disposition sits outside it deliberately --
/// it is the one column that changes legitimately, and putting it inside would make every
/// honest update require a digest recompute, which trains a reader to regenerate the line
/// without reading it.
///
/// So this applies the digest's own trick to the column the digest cannot cover. State the
/// tally. Hiding a disposition change now requires editing this line in the same commit,
/// where a reader sees it -- which is the entire mechanism, and the only one available for
/// a value that is supposed to move.
///
/// Zero counts are stated too, so the first `REFUTED` row also changes the line.
#[test]
fn the_disposition_tally_matches_the_rows() {
    let rows = rows();
    assert!(
        !rows.is_empty(),
        "no finding rows parsed, so this test proves nothing"
    );

    let mut counted: BTreeMap<&str, usize> = DISPOSITIONS.iter().map(|d| (*d, 0)).collect();
    for row in &rows {
        // No panic arm here: `every_finding_has_a_disposition` owns the unrecognised case,
        // and a panic this build cannot reach is a coverage region no test can close. An
        // unrecognised cell adds a key the tally line does not state, so it still fails --
        // loudly, and with a readable diff.
        let kind = disposition_kind(&row.disposition).unwrap_or("UNRECOGNISED");
        *counted.entry(kind).or_insert(0) += 1;
    }

    let mut want = counted
        .iter()
        .map(|(kind, n)| format!("{kind} {n}"))
        .collect::<Vec<_>>()
        .join(" \u{b7} ");
    want.push_str(" \u{b7} total ");
    want.push_str(&rows.len().to_string());

    let stated = LEDGER
        .split("<!-- dispositions: ")
        .nth(1)
        .and_then(|rest| rest.split_once(" -->"))
        .map(|(tally, _)| tally.trim());
    assert_eq!(
        stated,
        Some(want.as_str()),
        "the stated disposition tally does not match the rows. If a disposition genuinely \
         changed, update the comment in the same commit -- that visible line IS the guard. \
         If it did not, some row's disposition moved without anyone saying so."
    );
}

/// A `FIXED` row names the commit that fixed it.
///
/// Without this, `FIXED` is a claim with nothing behind it. With it, anyone can run
/// `git show <sha>` and read the test that was proved to fail beforehand.
#[test]
fn every_fixed_finding_names_a_commit() {
    // `contains` and not `starts_with`: `PARTLY FIXED` must name a commit too. Filtering on
    // the prefix let the partial ones through the moment that disposition was introduced,
    // which is the check silently narrowing as the vocabulary grows — the same shape as a
    // parser that stops matching rows it was written before.
    for row in rows().iter().filter(|r| r.disposition.contains("FIXED")) {
        let after = row
            .disposition
            .split_once("FIXED")
            .map_or("", |(_, rest)| rest.trim_start());
        let sha: String = after.chars().take_while(char::is_ascii_hexdigit).collect();
        assert!(
            sha.len() >= 7,
            "finding {} says {:?} but names no commit. `FIXED` without a sha is a claim \
             with nothing behind it.",
            row.id,
            row.disposition
        );
    }
}

/// Ids are unique, so a citation means one thing.
///
/// D-0104 exists because three decision numbers were each issued twice and forty
/// citations became ambiguous. The ledger's ids are hashes of the findings' own titles
/// precisely so that cannot recur — this test is what says the scheme is holding.
#[test]
fn finding_ids_are_unique() {
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    for row in rows() {
        *seen.entry(row.id).or_insert(0) += 1;
    }
    let dupes: Vec<(&String, &usize)> = seen.iter().filter(|(_, n)| **n > 1).collect();
    assert!(
        dupes.is_empty(),
        "these finding ids are issued more than once: {dupes:?}. An ambiguous id is \
         D-0104's defect in a new document."
    );
}

/// Every severity is one the sweep actually assigned.
#[test]
fn every_severity_is_a_known_one() {
    for row in rows() {
        assert!(
            SEVERITIES.contains(&row.severity.as_str()),
            "finding {} has severity {:?}, which is not one of {SEVERITIES:?}",
            row.id,
            row.severity
        );
    }
}

/// The ledger never shrinks.
///
/// The failure this file was written to prevent: a row removed because it was fixed, or
/// decided against, or by accident — after which nothing can tell which happened. A fixed
/// finding is marked `FIXED`, a wrong one is marked `REFUTED` with the reason, and both
/// stay.
#[test]
fn the_ledger_never_loses_a_row() {
    let n = rows().len();
    assert!(
        n >= ROWS_AT_INTRODUCTION,
        "the ledger has {n} finding rows and had {ROWS_AT_INTRODUCTION} when this test was \
         written. A row was deleted. Mark it `FIXED <sha>` or `REFUTED <why>` instead — \
         the document's own header says no row is ever deleted, and a ledger that drops \
         its entries cannot answer whether anything was missed."
    );
}

/// The counts in the prose are the counts in the tables.
///
/// The `docs/03-vocabulary.md` failure, in a new document: a number written once in prose
/// and never read again is a number that goes stale. D-0102 recorded the rule that closes
/// it, and this is that rule applied here.
#[test]
fn the_stated_counts_are_the_real_ones() {
    let rows = rows();
    let flat: String = LEDGER.split_whitespace().collect::<Vec<_>>().join(" ");

    // "**26 were killed. 99 stood.**"
    let stood = format!("{} stood", rows.len());
    assert!(
        flat.contains(&stood),
        "the ledger has {} finding rows and its prose does not say \"{stood}\"",
        rows.len()
    );

    // The costing-a-combination table's own heading states its size.
    let costs = LEDGER
        .split("## The 37 that can cost a combination")
        .nth(1)
        .expect("the costing-a-combination section exists under that exact heading");
    let counted = costs
        .split("\n---")
        .next()
        .unwrap_or(costs)
        .lines()
        .filter(|l| l.starts_with("| `F-"))
        .count();
    assert_eq!(
        counted, 37,
        "the heading says 37 findings can cost a combination and the table under it has \
         {counted} rows"
    );

    let others = LEDGER
        .split("## The other 74")
        .nth(1)
        .expect("the remainder section exists under that exact heading");
    let rest = others
        .split("\n---")
        .next()
        .unwrap_or(others)
        .lines()
        .filter(|l| l.starts_with("| `F-"))
        .count();
    assert_eq!(
        rest, 74,
        "the heading says 74 and the table under it has {rest} rows"
    );
    assert_eq!(
        counted + rest,
        rows.len(),
        "the two tables hold {} rows between them and the ledger has {}",
        counted + rest,
        rows.len()
    );
}

/// The killed findings are recorded, with a reason each.
///
/// A sweep that publishes only what survived is hiding its own error rate, and a killed
/// finding is the one most likely to be raised again by the next reader.
#[test]
fn every_killed_finding_carries_its_reason() {
    let section = LEDGER
        .split("## The 26 that were killed")
        .nth(1)
        .expect("the killed section exists under that exact heading");
    let rows: Vec<&str> = section
        .split("\n---")
        .next()
        .unwrap_or(section)
        .lines()
        .filter(|l| l.starts_with("| ") && !l.starts_with("|---") && !l.starts_with("| Finding"))
        .collect();
    assert_eq!(
        rows.len(),
        26,
        "the heading says 26 killed findings and the table has {}",
        rows.len()
    );
    for row in rows {
        let cells: Vec<&str> = row.split('|').map(str::trim).collect();
        assert!(
            cells.len() >= 3 && cells[2].len() > 20,
            "a killed finding has no reason beside it, so a later reader cannot tell \
             whether it was wrong or merely inconvenient: {row}"
        );
    }
}

/// FNV-1a 64, so a row's content can be checked without a dependency.
///
/// `crates/core` declares no dependencies and must not gain one for a test, so the hash is
/// written out. FNV-1a is not cryptographic and does not need to be: this defends against an
/// accidental or careless edit, not against someone who wants to forge a digest — and someone
/// editing the ledger to hide a finding has to edit the digest line too, which is a visible
/// diff and the whole point.
fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// Every row's id, severity, finding and location, joined — the content the digest covers.
///
/// The DISPOSITION is deliberately excluded: it changes legitimately every time a finding is
/// worked on, and a digest that had to be updated for each would be updated without being
/// read. What must not change silently is what a row SAYS.
fn digest_payload() -> String {
    LEDGER
        .lines()
        .filter(|l| l.starts_with("| `F-"))
        .map(|l| {
            let c: Vec<&str> = l.split('|').map(str::trim).collect();
            format!("{}|{}|{}|{}", c[1], c[2], c[3], c[4])
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The rows are the rows the digest was taken over.
///
/// # What this catches that nothing else did
///
/// A verification sweep applied five edits to this document and **the other six tests caught
/// none of them**: swapping two rows' id cells so every citation resolves to the wrong
/// finding; repurposing a row's text while keeping the count at 99; downgrading the one `law`
/// severity to `gap`; and two disposition forgeries. The other tests check that the document
/// has the right SHAPE. None of them could see that it had the wrong CONTENT.
///
/// The digest is over id, severity, finding and location — not disposition, which changes
/// legitimately. So a disposition update needs no digest change, and altering what a row
/// says does. Regenerating the digest is a one-line visible diff, which is the point: hiding
/// a finding now requires editing the digest in the same commit, where a reader sees it.
#[test]
fn the_rows_are_the_rows_the_digest_covers() {
    let stated = LEDGER
        .split("<!-- rows-digest: ")
        .nth(1)
        .and_then(|rest| rest.split(' ').next())
        .expect("the digest marker exists at the end of the document");
    let actual = format!("{:016x}", fnv1a64(digest_payload().as_bytes()));
    assert_eq!(
        stated, actual,
        "the finding rows do not hash to the digest this document states. A row's id, \
         severity, text or location changed. If that was deliberate — a correction, or a new \
         finding appended — recompute the digest in the SAME commit so the change is visible. \
         If it was not, a finding has been altered or swapped."
    );
}

/// A `REFUTED` row carries its reason, exactly as a `FIXED` row carries its commit.
///
/// `REFUTED` with nothing after it says a finding was wrong and refuses to say why, which is
/// the one disposition where the reason IS the content. Caught nothing until now.
#[test]
fn every_refuted_finding_carries_its_reason() {
    for row in rows()
        .iter()
        .filter(|r| r.disposition.starts_with("REFUTED"))
    {
        let reason = row.disposition.trim_start_matches("REFUTED").trim();
        assert!(
            reason.len() >= 20,
            "finding {} says {:?} and gives no reason. A refutation without one cannot be \
             checked, and the next reader will raise the finding again.",
            row.id,
            row.disposition
        );
    }
}

/// Every commit a row names actually exists in this repository.
///
/// `FIXED deadbeef` passed every other test. `git rev-parse --verify` is the only thing that
/// can tell a real sha from a plausible one, so this test runs it — the single place in this
/// file that reaches outside `include_str!`.
///
/// Skipped, loudly, when git is unavailable: a test that silently passes because its tool is
/// missing is the fallback §4 bans, so the absence is printed rather than swallowed.
#[test]
fn every_named_commit_exists() {
    let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("crates/core is two levels below the repository root")
        .to_owned();

    // THREE ENVIRONMENTS, and they need three different answers. This test reads git
    // history, and `cargo test` runs in places where that history is absent for reasons
    // that are nothing to do with the ledger.
    //
    //   a full clone      -> check every sha, which is the point
    //   NOT a work tree   -> skip, printing why. `cargo-mutants` copies the tree to
    //                        /tmp WITHOUT `.git`, so no amount of configuration helps
    //                        and refusing there would fail gate 18 on a green ledger.
    //   a SHALLOW clone   -> REFUSE, naming the remedy. `actions/checkout` defaults to
    //                        depth 1, and at that depth no sha resolves -- both of these
    //                        tests reported "cannot resolve it" for commits that plainly
    //                        exist and took `ci-ok` down twice. That is a misconfigured
    //                        environment, not an impossible one, so it fails loudly.
    let git = |args: &[&str]| {
        std::process::Command::new("git")
            .arg("-C")
            .arg(&repo)
            .args(args)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
    };
    if git(&["rev-parse", "--git-dir"]).is_none() {
        println!(
            "SKIPPING: {} is not a git work tree, so no commit can be resolved. This is \
             what `cargo-mutants` looks like -- it copies the tree without `.git`.",
            repo.display()
        );
        return;
    }
    assert_ne!(
        git(&["rev-parse", "--is-shallow-repository"]).as_deref(),
        Some("true"),
        "this is a SHALLOW clone, so no commit below can be resolved and this test would \
         refuse commits that exist. The checkout needs `fetch-depth: 0`."
    );

    let mut checked = 0_u32;
    for row in rows().iter().filter(|r| r.disposition.contains("FIXED")) {
        let after = row
            .disposition
            .split_once("FIXED")
            .map_or("", |(_, rest)| rest.trim_start());
        let sha: String = after.chars().take_while(char::is_ascii_hexdigit).collect();
        if sha.len() < 7 {
            continue; // the sha-presence test owns this case
        }
        let out = std::process::Command::new("git")
            .arg("-C")
            .arg(&repo)
            .args(["rev-parse", "--verify", "--quiet"])
            .arg(format!("{sha}^{{commit}}"))
            .output();
        let Ok(out) = out else {
            println!(
                "SKIPPED: git is not runnable here, so {} was not verified",
                row.id
            );
            continue;
        };
        assert!(
            out.status.success(),
            "finding {} names commit {sha} and `git rev-parse` cannot resolve it. A row marked \
             FIXED against a commit that does not exist is the worst failure this ledger can \
             have: it reads as done and points at nothing.",
            row.id
        );
        checked += 1;
    }
    assert!(
        checked > 0,
        "no commit was verified, so this test proves nothing — either every FIXED row lost its \
         sha or git could not run at all"
    );
}

/// A named commit is in THIS branch's history, not merely an object that resolves.
///
/// `git rev-parse --verify` answers "does this object exist". It says nothing about where.
/// It resolves a commit on an abandoned branch, one that was reverted, one reachable only
/// from a stash, and any of the other 180 commits in this repository's history. An
/// adversarial audit used exactly that: `FIXED <any real sha>` satisfied the check whose
/// whole purpose was to stop that claim.
///
/// `merge-base --is-ancestor` is the stronger question, and it is the one a reader means
/// when they read a sha in this column: is the fix actually in the history I am looking at.
///
/// What it still cannot say, stated rather than implied: that the commit is RELEVANT. The
/// obvious strengthening -- the commit must touch a file the row's `where` cell names --
/// was tried and rejected, because it false-positives on three honest rows here. `F-72F226`
/// names the last of four commits that fixed four sites, and that one touched only the
/// fourth. `F-0C28E4` and `F-506ED7` are PARTLY FIXED by a commit that published a boundary
/// in `crates/indicators` for a defect whose `where` is `crates/engine` -- the partial fix
/// legitimately lives in a different crate from the finding. A guard that refuses correct
/// rows gets an exemption list, and an exemption list is where guarantees go to die.
#[test]
fn every_named_commit_is_in_this_branchs_history() {
    let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("crates/core is two levels below the repository root")
        .to_owned();

    // THREE ENVIRONMENTS, and they need three different answers. This test reads git
    // history, and `cargo test` runs in places where that history is absent for reasons
    // that are nothing to do with the ledger.
    //
    //   a full clone      -> check every sha, which is the point
    //   NOT a work tree   -> skip, printing why. `cargo-mutants` copies the tree to
    //                        /tmp WITHOUT `.git`, so no amount of configuration helps
    //                        and refusing there would fail gate 18 on a green ledger.
    //   a SHALLOW clone   -> REFUSE, naming the remedy. `actions/checkout` defaults to
    //                        depth 1, and at that depth no sha resolves -- both of these
    //                        tests reported "cannot resolve it" for commits that plainly
    //                        exist and took `ci-ok` down twice. That is a misconfigured
    //                        environment, not an impossible one, so it fails loudly.
    let git = |args: &[&str]| {
        std::process::Command::new("git")
            .arg("-C")
            .arg(&repo)
            .args(args)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
    };
    if git(&["rev-parse", "--git-dir"]).is_none() {
        println!(
            "SKIPPING: {} is not a git work tree, so no commit can be resolved. This is \
             what `cargo-mutants` looks like -- it copies the tree without `.git`.",
            repo.display()
        );
        return;
    }
    assert_ne!(
        git(&["rev-parse", "--is-shallow-repository"]).as_deref(),
        Some("true"),
        "this is a SHALLOW clone, so no commit below can be resolved and this test would \
         refuse commits that exist. The checkout needs `fetch-depth: 0`."
    );

    let mut checked = 0_u32;
    for row in rows().iter().filter(|r| r.disposition.contains("FIXED")) {
        let after = row
            .disposition
            .split_once("FIXED")
            .map_or("", |(_, rest)| rest.trim_start());
        let sha: String = after.chars().take_while(char::is_ascii_hexdigit).collect();
        if sha.len() < 7 {
            continue; // the sha-presence test owns this case
        }
        let out = std::process::Command::new("git")
            .arg("-C")
            .arg(&repo)
            .args(["merge-base", "--is-ancestor"])
            .arg(&sha)
            .arg("HEAD")
            .output();
        let Ok(out) = out else {
            println!(
                "SKIPPED: git is not runnable here, so {} was not verified",
                row.id
            );
            continue;
        };
        assert!(
            out.status.success(),
            "finding {} says it was fixed by {sha}, and that commit is not an ancestor of \
             HEAD. It exists somewhere -- a dropped branch, a revert, a stash -- but the fix \
             is not in the history this ledger describes.",
            row.id
        );
        checked += 1;
    }
    assert!(
        checked > 0,
        "no commit was checked for ancestry, so this test proves nothing"
    );
}

/// A named commit is on `main`, not only on the branch being tested.
///
/// `every_named_commit_is_in_this_branchs_history` asks about HEAD, and on a pull request
/// HEAD contains the branch's own commits. `main` takes squash merges, so none of those
/// commits ever becomes an ancestor of `main`. A row that names one passes on the pull
/// request and fails on `main` the moment it merges. That is not hypothetical: every
/// `FIXED` row named a commit on #13's branch, all 26 passed there, and the post-merge run
/// on `main` went red at gate 1e on the first of them (D-0680).
///
/// This test moves that failure to before the merge. `refs/remotes/origin/main` is the
/// history the ledger is read against, and every CI job that runs this checks out with
/// `fetch-depth: 0`, which fetches it. A clone without that ref is refused with the fetch
/// that fixes it, for the same reason a shallow clone is: skipping would pass a ledger
/// that `main` rejects.
#[test]
fn every_named_commit_is_on_main() {
    let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("crates/core is two levels below the repository root")
        .to_owned();

    // The same three environments as the two tests above, with the same three answers.
    let git = |args: &[&str]| {
        std::process::Command::new("git")
            .arg("-C")
            .arg(&repo)
            .args(args)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
    };
    if git(&["rev-parse", "--git-dir"]).is_none() {
        println!(
            "SKIPPING: {} is not a git work tree, so no commit can be resolved. This is \
             what `cargo-mutants` looks like -- it copies the tree without `.git`.",
            repo.display()
        );
        return;
    }
    assert_ne!(
        git(&["rev-parse", "--is-shallow-repository"]).as_deref(),
        Some("true"),
        "this is a SHALLOW clone, so no commit below can be resolved and this test would \
         refuse commits that exist. The checkout needs `fetch-depth: 0`."
    );
    let main = "refs/remotes/origin/main";
    assert!(
        git(&["rev-parse", "--verify", "--quiet", main]).is_some(),
        "{main} does not resolve, so no row can be checked against the history the ledger \
         is read on. Run `git fetch origin main`. CI's `fetch-depth: 0` checkout fetches it."
    );

    let mut checked = 0_u32;
    for row in rows().iter().filter(|r| r.disposition.contains("FIXED")) {
        let after = row
            .disposition
            .split_once("FIXED")
            .map_or("", |(_, rest)| rest.trim_start());
        let sha: String = after.chars().take_while(char::is_ascii_hexdigit).collect();
        if sha.len() < 7 {
            continue; // the sha-presence test owns this case
        }
        let out = std::process::Command::new("git")
            .arg("-C")
            .arg(&repo)
            .args(["merge-base", "--is-ancestor"])
            .arg(&sha)
            .arg(main)
            .output();
        let Ok(out) = out else {
            println!(
                "SKIPPED: git is not runnable here, so {} was not verified",
                row.id
            );
            continue;
        };
        assert!(
            out.status.success(),
            "finding {} says it was fixed by {sha}, and that commit is not an ancestor of \
             {main}. `main` takes squash merges, so a commit on a pull request's branch never \
             reaches it. Mark the row IN PROGRESS naming {sha}, and name the squash commit \
             once it merges. If the fix is already on `main`, fetch it: `git fetch origin \
             main`.",
            row.id
        );
        checked += 1;
    }
    assert!(
        checked > 0,
        "no commit was checked against {main}, so this test proves nothing"
    );
}

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
//! * a `FIXED` row names a commit;
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
const DISPOSITIONS: [&str; 5] = [
    "FIXED",
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

/// A `FIXED` row names the commit that fixed it.
///
/// Without this, `FIXED` is a claim with nothing behind it. With it, anyone can run
/// `git show <sha>` and read the test that was proved to fail beforehand.
#[test]
fn every_fixed_finding_names_a_commit() {
    for row in rows().iter().filter(|r| r.disposition.starts_with("FIXED")) {
        let sha: String = row
            .disposition
            .chars()
            .skip("FIXED ".len())
            .take_while(char::is_ascii_hexdigit)
            .collect();
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
        .split("## The other 62")
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
        rest, 62,
        "the heading says 62 and the table under it has {rest} rows"
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

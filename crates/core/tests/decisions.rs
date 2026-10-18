//! Every decision number heads exactly one entry of `docs/05-decisions.md`
//! (FIB-1, FM-F008; D-6311).
//!
//! # Why this file exists beside gate 27b
//!
//! Gate 27b reads the same rule from a CI step. FM-F008 found two plans that
//! had each reserved the same range of numbers (the FIB plan's blocks and the
//! PREC plan's), and asked that `cargo test` refuse a collision too, so a
//! package that merges a second entry under a used number fails on the
//! machine it was written on and not only in CI. The five numbers issued twice
//! before the ledger was read back stay pinned at their exact counts, exactly
//! as gate 27b's `DECISION_PINS` pins them: a third copy fails, and so does a
//! removed one.
//!
//! An entry heading is gate 27b's: two to four `#`, one or more spaces, `D-`
//! and four digits, outside a fenced block.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]

use std::collections::BTreeMap;

/// The decision ledger.
const LEDGER: &str = include_str!("../../../docs/05-decisions.md");

/// The numbers issued twice before the ledger was read back, at their exact
/// counts (gate 27b's `DECISION_PINS`, D-0104, D-0684).
const PINS: &[(&str, usize)] = &[
    ("D-0076", 2),
    ("D-0077", 2),
    ("D-0078", 2),
    ("D-0370", 2),
    ("D-0372", 2),
];

/// The number an entry heading heads, when `line` is one.
fn heading_number(line: &str) -> Option<&str> {
    let hashes = line.bytes().take_while(|b| *b == b'#').count();
    if !(2..=4).contains(&hashes) {
        return None;
    }
    let rest = &line[hashes..];
    let body = rest.trim_start_matches(' ');
    if body.len() == rest.len() || !body.starts_with("D-") {
        return None;
    }
    let number = body.get(..6)?;
    number[2..]
        .bytes()
        .all(|b| b.is_ascii_digit())
        .then_some(number)
}

/// How many entries each number heads, outside fenced blocks.
fn headings(ledger: &str) -> BTreeMap<&str, usize> {
    let mut counts = BTreeMap::new();
    let mut fence: Option<(u8, usize)> = None;
    for line in ledger.lines() {
        let trimmed = line.trim_start();
        let mark = trimmed.bytes().next().filter(|b| matches!(b, b'`' | b'~'));
        if let Some(mark) = mark {
            let run = trimmed.bytes().take_while(|b| *b == mark).count();
            match fence {
                None if run >= 3 => {
                    fence = Some((mark, run));
                    continue;
                }
                Some((open, len)) if open == mark && run >= len && trimmed.len() == run => {
                    fence = None;
                    continue;
                }
                _ => {}
            }
        }
        if fence.is_some() {
            continue;
        }
        if let Some(number) = heading_number(line) {
            *counts.entry(number).or_insert(0) += 1;
        }
    }
    counts
}

/// The numbers heading more or fewer entries than their pin allows.
fn misissued<'a>(counts: &BTreeMap<&'a str, usize>) -> Vec<(&'a str, usize)> {
    let mut wrong: Vec<(&str, usize)> = counts
        .iter()
        .filter(|(number, n)| {
            let pinned = PINS.iter().find(|(pin, _)| pin == *number).map(|(_, n)| *n);
            **n != pinned.unwrap_or(1)
        })
        .map(|(number, n)| (*number, *n))
        .collect();
    for (pin, _) in PINS {
        if !counts.contains_key(pin) {
            wrong.push((pin, 0));
        }
    }
    wrong
}

#[test]
fn every_decision_id_is_issued_once() {
    let counts = headings(LEDGER);
    assert!(
        counts.len() > 1_000,
        "the ledger was read: {}",
        counts.len()
    );
    assert_eq!(
        misissued(&counts),
        [],
        "a decision number heads two entries"
    );
    // FIB-1's block, moved by D-6311, is present once each.
    for n in 6300..=6318 {
        let number = format!("D-{n}");
        assert_eq!(counts.get(number.as_str()), Some(&1), "{number}");
    }
}

/// A FIXTURE WITH TWO ENTRIES UNDER ONE NUMBER IS REFUSED, and so is a pinned
/// number that lost an entry; a heading inside a fence, a level-five heading
/// and a five-digit number head nothing.
#[test]
fn a_fixture_with_one_number_issued_twice_is_refused() {
    let twice = format!(
        "## {d}0076 a\n## {d}0076 b\n## {d}0077 a\n## {d}0077 b\n## {d}0078 a\n\
         ## {d}0078 b\n## {d}0370 a\n## {d}0370 b\n## {d}0372 a\n## {d}0372 b\n\
         ### {d}5520 first\n### {d}5520 second\n#### {d}5521 once\n",
        d = "D-"
    );
    let counts = headings(&twice);
    let wrong = misissued(&counts);
    assert_eq!(wrong, [(&format!("{}5520", "D-")[..], 2)]);
    let lost = twice.replacen(&format!("## {}0372 b\n", "D-"), "", 1);
    let counts = headings(&lost);
    assert_eq!(
        misissued(&counts),
        [
            (&format!("{}0372", "D-")[..], 1),
            (&format!("{}5520", "D-")[..], 2)
        ]
    );
    let ignored = format!(
        "```\n## {d}9001 fenced\n```\n##### {d}9002 deep\n## {d}90031 long\n#not {d}9004\n## x {d}9005\n",
        d = "D-"
    );
    let counts = headings(&ignored);
    assert_eq!(
        counts.get(&format!("{}9003", "D-")[..]),
        Some(&1),
        "four digits are read"
    );
    assert_eq!(counts.len(), 1, "{counts:?}");
    let pins_missing = misissued(&BTreeMap::new());
    assert_eq!(pins_missing.len(), PINS.len(), "every pin is missing");
}

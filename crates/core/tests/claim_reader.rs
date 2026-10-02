//! Gates 12 and 14 read cost claims with one reader, and gate 12 excuses a claim by
//! the item it documents. D-1116.
//!
//! Gate 14 said it used gate 12's trigger "character for character" while it
//! scrubbed with one `sed` where gate 12 ran five, so the two gates disagreed
//! about what a claim is. And gate 12's allowlist was `FILE COUNT`, so a slot one
//! block gave up went silently to the next unproven claim in that file. The
//! workflow now carries the reader once per step between two markers, and gate 14
//! compares the two at run time; this test holds the same facts under
//! `cargo test`, where a broken workflow is seen before CI runs it.
//!
//! It lives in `core` for the reason `banned_lists.rs` gives: gate 22 clause D
//! refuses a sweep crate's compile-time include of the workflow.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]

const WORKFLOW: &str = include_str!("../../../.github/workflows/ci.yml");

const OPEN: &str = "# >>> CLAIM READER";
const CLOSE: &str = "# <<< CLAIM READER";

/// Every marked region, from its opening marker line to its closing one.
fn regions(workflow: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current: Option<Vec<&str>> = None;
    for line in workflow.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with(OPEN) {
            current = Some(Vec::new());
        }
        if let Some(lines) = current.as_mut() {
            lines.push(line);
        }
        if trimmed.starts_with(CLOSE)
            && let Some(lines) = current.take()
        {
            out.push(lines.join("\n"));
        }
    }
    out
}

/// The text of one step, from its `- name:` line to the next step's.
fn step<'a>(workflow: &'a str, header: &str) -> &'a str {
    let start = workflow
        .find(&format!("- name: {header}"))
        .expect("the workflow has the step");
    let rest = &workflow[start + 1..];
    let end = rest.find("\n      - ").map_or(rest.len(), |at| at + 1);
    &rest[..end]
}

/// `allow_claim`'s entries, one `(file, item)` per non-blank line.
fn allow_claim(workflow: &str) -> Vec<Vec<String>> {
    let gate = step(workflow, "Gate 12 —");
    let (_, rest) = gate
        .split_once("\n          allow_claim='")
        .expect("gate 12 declares allow_claim");
    let (list, _) = rest.split_once('\'').expect("the list closes");
    list.lines()
        .map(|line| {
            line.split_whitespace()
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .filter(|fields| !fields.is_empty())
        .collect()
}

#[test]
fn gates_12_and_14_carry_one_claim_reader() {
    let found = regions(WORKFLOW);
    assert_eq!(found.len(), 2, "exactly two marked claim readers");
    assert_eq!(found[0], found[1], "the two claim readers differ");
    assert!(
        step(WORKFLOW, "Gate 12 —").contains(&found[0]),
        "gate 12 carries the reader"
    );
    assert!(
        step(WORKFLOW, "Gate 14 —").contains(&found[1]),
        "gate 14 carries the reader"
    );
    // The reader defines what both gates call, and neither step defines its own.
    for name in [
        "claim_blocks() {",
        "claim_scrub() {",
        "trigger='",
        "notclaim='",
    ] {
        assert!(found[0].contains(name), "the reader defines {name}");
        for header in ["Gate 12 —", "Gate 14 —"] {
            assert_eq!(
                step(WORKFLOW, header).matches(name).count(),
                1,
                "{header} defines {name} once, inside the reader"
            );
        }
    }
}

#[test]
fn the_reader_check_sees_a_divergent_copy() {
    let found = regions(WORKFLOW);
    let altered = WORKFLOW.replacen("s/[Ww]orst[ -][Cc]ase//g", "s/[Ww]orst//g", 1);
    let after = regions(&altered);
    assert_eq!(after.len(), 2);
    assert_ne!(after[0], after[1], "one copy changed, so the two differ");
    assert_eq!(after[1], found[1], "only the first copy changed");
    // And a missing copy is seen as a count, not as agreement.
    let one = WORKFLOW.replacen(OPEN, "# --- CLAIM READER", 1);
    assert_eq!(regions(&one).len(), 1);
}

#[test]
fn every_claim_allowance_names_one_item() {
    let entries = allow_claim(WORKFLOW);
    assert!(!entries.is_empty(), "the list is read");
    for fields in &entries {
        assert_eq!(fields.len(), 2, "FILE ITEM, not a count: {fields:?}");
        assert!(fields[0].starts_with("crates/"), "{fields:?}");
        assert!(
            fields[1] != "-" && !fields[1].bytes().all(|b| b.is_ascii_digit()),
            "the item is a name, not `-` and not a count: {fields:?}"
        );
    }
}

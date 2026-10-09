//! Gates 12 and 14 read cost claims with one reader, and gate 12 excuses a claim by
//! the item it documents. D-1116, D-2314.
//!
//! Gate 14 said it used gate 12's trigger "character for character" while it
//! scrubbed with one `sed` where gate 12 ran five, so the two gates disagreed
//! about what a claim is. And gate 12's allowlist was `FILE COUNT`, so a slot one
//! block gave up went silently to the next unproven claim in that file. Since
//! D-2314 both gates are subcommands of `.github/gates_bounds.rs`, whose one
//! `claim_blocks` / `is_claim` pair both call; this test holds that under
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
const TOOL: &str = include_str!("../../../.github/gates_bounds.rs");

/// The text of one step, from its `- name:` line to the next step's.
fn step<'a>(workflow: &'a str, header: &str) -> &'a str {
    let start = workflow
        .find(&format!("- name: {header}"))
        .expect("the workflow has the step");
    let rest = &workflow[start + 1..];
    let end = rest.find("\n      - ").map_or(rest.len(), |at| at + 1);
    &rest[..end]
}

/// The body of `fn NAME(` in the tool, to its closing brace at column 0.
fn function<'a>(src: &'a str, name: &str) -> &'a str {
    let start = src
        .find(&format!("\nfn {name}("))
        .unwrap_or_else(|| panic!("the tool defines fn {name}"));
    let rest = &src[start + 1..];
    &rest[..rest.find("\n}\n").expect("the function closes") + 2]
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
    // Each step runs its subcommand, and neither carries a reader of its own.
    assert!(step(WORKFLOW, "Gate 12 —").contains("\"$tool\" gate12 \"$allow_claim\""));
    assert!(step(WORKFLOW, "Gate 14 —").contains("gates-bounds\" gate14 \"$cover\""));
    for header in ["Gate 12 —", "Gate 14 —"] {
        for name in [
            "claim_blocks() {",
            "claim_scrub() {",
            "trigger='",
            "notclaim='",
        ] {
            assert!(
                !step(WORKFLOW, header).contains(name),
                "{header} defines {name}"
            );
        }
    }
    // The tool defines the reader once, and both gates call it.
    for name in [
        "fn claim_blocks(",
        "fn claim_scrub(",
        "fn is_claim(",
        "fn triggers(",
    ] {
        assert_eq!(TOOL.matches(name).count(), 1, "one {name}");
    }
    for gate in ["gate12", "gate14"] {
        let body = function(TOOL, gate);
        assert!(
            body.contains("claim_blocks("),
            "{gate} reads blocks with the reader"
        );
        assert!(
            body.contains("is_claim("),
            "{gate} decides claims with the reader"
        );
    }
}

#[test]
fn the_reader_check_sees_a_second_reader() {
    // A step that grew its own trigger again is seen, and so is a tool that
    // defined a second `is_claim`.
    let altered = WORKFLOW.replacen(
        "\"$tool\" gate12 \"$allow_claim\"",
        "trigger='O'\n          \"$tool\" gate12 \"$allow_claim\"",
        1,
    );
    assert!(step(&altered, "Gate 12 —").contains("trigger='"));
    let doubled = format!("{TOOL}\nfn is_claim(_: &str) -> bool {{ false }}\n");
    assert_eq!(doubled.matches("fn is_claim(").count(), 2);
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

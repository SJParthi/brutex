//! Ledger append, page and lookup costs that lane 1-b found stated wrongly,
//! read where they stand and checked against the source they describe.
//!
//! * W2-cli3-4 and W2-cli3-8 (D-1680): `docs/06-limits.md` §150 said a
//!   Candidate Universe append is proportional to the new block and that the
//!   generation check also hashes the data files. The production append door
//!   opened the ledger twice, and the generation is metadata only.
//!
//! Every file a constant below names is read at compile time, so a rename
//! fails the build rather than skipping the check. A separate test crate, as
//! `crates/cli/tests/limits_doc_drift.rs` is, because a document read into the
//! library's own tests would rebuild them whenever a line of it changed.

#![allow(
    clippy::expect_used,
    clippy::panic,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail. An integration test file is \
              its own crate root, so the attribute cannot be inherited."
)]

const LIMITS: &str = include_str!("../../../docs/06-limits.md");
const CANDIDATE: &str = include_str!("../src/candidate_universe.rs");

/// The body of one `### §N —` section, up to the next `### ` heading.
fn section(number: u32) -> &'static str {
    let heading = format!("### §{number} —");
    let start = LIMITS
        .find(&heading)
        .unwrap_or_else(|| panic!("docs/06-limits.md has no `{heading}` heading"));
    let body = LIMITS.get(start + heading.len()..).expect("a suffix");
    let end = body.find("\n### ").unwrap_or(body.len());
    body.get(..end).expect("the section")
}

/// `text` with every `//!` and `///` dropped and every run of whitespace
/// folded to one space, so a sentence re-wrapped across lines is still found.
fn flat(text: &str) -> String {
    text.replace("//!", " ")
        .replace("///", " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// The body of the free function opened by `signature` in `source`, up to the
/// next line that closes at the left margin.
fn function<'a>(source: &'a str, signature: &str) -> &'a str {
    let start = source
        .find(signature)
        .unwrap_or_else(|| panic!("no `{signature}` in the source"));
    let body = source.get(start..).expect("a suffix");
    let end = body.find("\n}\n").expect("the function closes");
    body.get(..end).expect("the body")
}

#[test]
fn section_150_states_one_open_per_production_append_and_no_data_hash() {
    // The source first: what the section must describe.
    let door = function(CANDIDATE, "fn append_prepared_and_reverify(");
    assert_eq!(
        door.matches("CandidateUniverseLedgerV1::open").count(),
        1,
        "the production append door opens the ledger more than once again; \
         re-measure §150 before changing this test"
    );
    assert!(door.contains("reverify_committed"));
    let generation = function(CANDIDATE, "fn file_generation(");
    assert!(
        !generation.contains("hash") && !generation.contains("blake3"),
        "the candidate file generation hashes data now; §150 must say so"
    );

    let text = flat(section(150));
    for stale in [
        "and also hashes the data files",
        "Append, hashing, canonical-order validation and durability are proportional to the new block",
    ] {
        assert!(!text.contains(stale), "§150 still says `{stale}`");
    }
    for needed in [
        "one production append is O(R+C) for that open plus O(new rows)",
        "It is metadata only and hashes no data file",
        "16 per run",
    ] {
        assert!(text.contains(needed), "§150 no longer says `{needed}`");
    }
    let header = flat(CANDIDATE.get(..2_000).expect("the module header"));
    assert!(!header.contains("the sealed internal append is O(new rows)"));
    assert!(header.contains("one production append costs O(rows + receipts)"));
}

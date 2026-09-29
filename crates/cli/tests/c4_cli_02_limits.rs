//! D-0910: three resumable Boolean paths re-verify all completed work on every
//! step, and `docs/06-limits.md` says so in the source's own words.

#![allow(
    clippy::panic,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail. An integration test file is \
              its own crate root, so the attribute cannot be inherited."
)]

const LIMITS: &str = include_str!("../../../docs/06-limits.md");
const GRAMMAR: &str = include_str!("../src/boolean_grammar_campaign.rs");
const SEARCH: &str = include_str!("../src/boolean_search_command.rs");
const READER: &str = include_str!("../src/boolean_search_reader.rs");
const OOS: &str = include_str!("../src/boolean_oos_v1.rs");
const CANDIDATE: &str = include_str!("../src/boolean_candidate_v1.rs");

const HEADING: &str =
    "## Three resumable Boolean paths re-verify all completed work on every step — D-0910";

fn section() -> &'static str {
    let start = LIMITS
        .find(HEADING)
        .unwrap_or_else(|| panic!("docs/06-limits.md has no D-0910 section"));
    let rest = &LIMITS[start + HEADING.len()..];
    rest.find("\n## ").map_or(rest, |end| &rest[..end])
}

/// Collapse the section's line breaks so a quote wrapped across lines matches.
fn flat(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[test]
fn each_quoted_line_is_in_the_source_it_names() {
    let section = flat(section());
    for (name, source, quote) in [
        (
            "grammar",
            GRAMMAR,
            "for (sequence, pin) in chain.into_iter().rev() {",
        ),
        ("grammar", GRAMMAR, "complete(id, pin, batch.programs())?;"),
        (
            "grammar",
            GRAMMAR,
            "let expected = prepare(&request.campaign(programs), &mut String::new())?;",
        ),
        (
            "grammar",
            GRAMMAR,
            "crate::boolean_campaign::verify_complete(request.root, id, pin, ancestry_bytes)",
        ),
        (
            "search",
            SEARCH,
            "let prior = Reader::open(request.input.output, identity, observe, records)?;",
        ),
        ("search", SEARCH, "prior.verify_batch(batch as u64)?;"),
        ("search", SEARCH, "saved.verify_batch(batch as u64)?;"),
        ("reader", READER, "for old in &self.history {"),
        ("reader", READER, "QualifiedCampaign::open("),
        (
            "reader",
            READER,
            "drop(open_rung(&self.root, &record, rung, allowance)?);",
        ),
        (
            "oos",
            OOS,
            "for (group, anchor) in training.anchors.iter().enumerate() {",
        ),
        ("oos", OOS, "training.require_current()?;"),
        ("oos", OOS, "training.anchors.len()"),
        ("oos", OOS, ".checked_mul(2)"),
        (
            "oos",
            OOS,
            "return Err(\"Boolean training anchors incomplete\".into());",
        ),
        ("candidate", CANDIDATE, "for row in &self.rows {"),
        ("candidate", CANDIDATE, "persistence::verify("),
    ] {
        assert!(
            section.contains(&format!("`{quote}`")),
            "the D-0910 section does not quote `{quote}`"
        );
        assert!(
            source.contains(quote),
            "{name} source no longer holds `{quote}`"
        );
    }
}

#[test]
fn the_verify_loops_the_section_describes_are_the_ones_in_the_source() {
    // The search command verifies every completed batch before and after it
    // publishes, and every `verify_batch` ends by rereading the history.
    assert!(SEARCH.contains("for batch in 0..prior.completed_batches() {"));
    assert!(SEARCH.contains("for batch in 0..saved.completed_batches() {"));
    let verify = READER
        .find("pub fn verify_batch(")
        .unwrap_or_else(|| panic!("verify_batch is gone"));
    let body = &READER[verify..];
    let body = &body[..body
        .find("\n    }\n")
        .unwrap_or_else(|| panic!("verify_batch has no end"))];
    assert_eq!(body.matches("self.require_current()").count(), 2, "{body}");
    assert!(
        body.trim_end().ends_with("self.require_current()"),
        "{body}"
    );
    // The grammar's recovery comment names the growth, not only the bound.
    let restore = GRAMMAR
        .find("fn restore(")
        .unwrap_or_else(|| panic!("restore is gone"));
    let comment = &GRAMMAR[..restore];
    let comment = &comment[comment
        .rfind("/// Cold recovery")
        .unwrap_or_else(|| panic!("restore's comment is gone"))..];
    let comment = flat(&comment.replace("///", ""));
    for phrase in [
        "`complete` runs once for every nonempty completed batch in the chain",
        "prepares that batch's campaign sources again",
        "Invocation n re-verifies the n - 1 batches before it (D-0910)",
    ] {
        assert!(comment.contains(phrase), "{comment}");
    }
    assert!(flat(section()).contains(
        "`cli::boolean_grammar_campaign::tests::every_invocation_reverifies_every_completed_batch_in_order`"
    ));
}

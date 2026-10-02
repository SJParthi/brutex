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

/// The brace-balanced block that `opener`, ending in `{`, begins in `source`.
/// Panics unless `opener` occurs exactly once, so a quote cannot silently bind
/// to a second, unrelated copy.
fn block<'a>(source: &'a str, opener: &str) -> &'a str {
    assert_eq!(
        source.matches(opener).count(),
        1,
        "`{opener}` is not unique"
    );
    assert!(opener.ends_with('{'), "`{opener}` opens no block");
    let start = source
        .find(opener)
        .unwrap_or_else(|| panic!("`{opener}` is gone"))
        + opener.len();
    let mut depth = 1_usize;
    for (at, byte) in source[start..].bytes().enumerate() {
        match byte {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return &source[start..start + at];
                }
            }
            _ => {}
        }
    }
    panic!("`{opener}` never closes")
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
        ("grammar", GRAMMAR, "if batch.programs().is_empty() {"),
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
        (
            "search",
            SEARCH,
            "let reader = Reader::open(request.input.output, identity, observe, records)?;",
        ),
        (
            "search",
            SEARCH,
            "for batch in 0..reader.completed_batches() {",
        ),
        ("search", SEARCH, "reader.verify_batch(batch as u64)?;"),
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
    // Each named loop is the one that makes the call the section says it does.
    for (source, opener, call) in [
        (
            SEARCH,
            "for batch in 0..reader.completed_batches() {",
            "reader.verify_batch(batch as u64)?;",
        ),
        (
            SEARCH,
            "for batch in 0..prior.completed_batches() {",
            "prior.verify_batch(batch as u64)?;",
        ),
        (
            SEARCH,
            "for batch in 0..saved.completed_batches() {",
            "saved.verify_batch(batch as u64)?;",
        ),
        (
            READER,
            "pub fn verify_batch(&self, batch: u64) -> Result<(), String> {",
            "QualifiedCampaign::open(",
        ),
        (
            READER,
            "pub fn verify_batch(&self, batch: u64) -> Result<(), String> {",
            "drop(open_rung(&self.root, &record, rung, allowance)?);",
        ),
        (
            READER,
            "pub fn require_current(&self) -> Result<(), String> {",
            "for old in &self.history {",
        ),
        (
            OOS,
            "for (group, anchor) in training.anchors.iter().enumerate() {",
            "training.require_current()?;",
        ),
        (
            GRAMMAR,
            ") -> Result<Restored, String> {",
            "for (sequence, pin) in chain.into_iter().rev() {",
        ),
        (
            GRAMMAR,
            ") -> Result<Restored, String> {",
            "if batch.programs().is_empty() {",
        ),
        (
            GRAMMAR,
            ") -> Result<Restored, String> {",
            "complete(id, pin, batch.programs())?;",
        ),
    ] {
        assert!(
            block(source, opener).contains(call),
            "`{opener}` no longer holds `{call}`"
        );
    }
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
        "Invocation n re-verifies every nonempty batch completed before it (at most n - 1)",
        "an empty batch takes the `batch.programs().is_empty()` branch and never reaches `complete` (D-0910)",
    ] {
        assert!(comment.contains(phrase), "{comment}");
    }
    assert!(flat(section()).contains(
        "`cli::boolean_grammar_campaign::tests::every_invocation_reverifies_every_completed_batch_in_order`"
    ));
}

const CI: &str = include_str!("../../../.github/workflows/ci.yml");
const MUTATION_GATE: &str = include_str!("../../../.github/mutation_gate.rs");
const DECISIONS: &str = include_str!("../../../docs/05-decisions.md");
const CHILD: &str = include_str!("../src/boolean_search_integration_tests.rs");
const RECEIPTS: &str = include_str!("../src/checksum_receipts_tests.rs");
const AUDIT: &str = include_str!("../../store/src/checksum_audit_tests.rs");
const LAUNCH: &str = include_str!("../../api/src/booleanlaunch_tests.rs");
const INDEXSTOP: &str = include_str!("../../api/src/indexstoplaunch_tests.rs");

const D0911: &str = "## The generated search child has no time bound of its own — D-0911";

/// One top-level job of `ci.yml`, from its `  name:` line to the next.
fn job(name: &str) -> &'static str {
    let opener = format!("\n  {name}:\n");
    let start = CI
        .find(&opener)
        .unwrap_or_else(|| panic!("ci.yml has no `{name}` job"))
        + opener.len();
    let rest = &CI[start..];
    let end = rest
        .match_indices("\n  ")
        .find(|(at, _)| {
            rest[at + 3..]
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_lowercase())
        })
        .map_or(rest.len(), |(at, _)| at);
    &rest[..end]
}

#[test]
fn the_d0911_costs_are_the_ones_in_ci_and_the_source() {
    let start = LIMITS
        .find(D0911)
        .unwrap_or_else(|| panic!("docs/06-limits.md has no D-0911 section"));
    let rest = &LIMITS[start + D0911.len()..];
    let section = flat(rest.find("\n## ").map_or(rest, |end| &rest[..end]));
    let decision = &DECISIONS[DECISIONS
        .find("### D-0911 ")
        .unwrap_or_else(|| panic!("D-0911 is gone"))..];
    assert!(
        flat(decision)
            .contains("`c4_cli_02_limits::the_d0911_costs_are_the_ones_in_ci_and_the_source`")
    );
    // The three jobs that run the tests declare no bound of their own.
    for (name, runs) in [
        ("language-purity", "cargo test --workspace --locked"),
        ("build", "cargo test --workspace --locked"),
        ("coverage", "cargo llvm-cov --workspace --locked"),
    ] {
        let body = job(name);
        assert!(body.contains(runs), "`{name}` no longer runs `{runs}`");
        assert!(
            !body.contains("timeout-minutes"),
            "`{name}` now declares a timeout; D-0911's limit is stale"
        );
        assert!(section.contains(&format!("`{name}`")), "{name}");
    }
    // The control: a job that does declare one is seen by the same reader.
    assert!(job("mutants").contains("timeout-minutes: 240"));
    for (name, source, quote) in [
        (
            "ci",
            CI,
            "tout=\"$(PATH=\"$stub:$PATH\" cargo test --workspace --locked 2>&1)\"",
        ),
        (
            "ci",
            CI,
            "--minimum-test-timeout 900 --timeout-multiplier 2",
        ),
        (
            "mutation gate",
            MUTATION_GATE,
            "if status != \"0\" || !missed.is_empty() || !timeout.is_empty() {",
        ),
        (
            "child",
            CHILD,
            ".stdout(file.try_clone().map_err(display)?)",
        ),
        ("child", CHILD, ".stderr(file)"),
        (
            "receipts",
            RECEIPTS,
            "if started.elapsed() > std::time::Duration::from_secs(2) {",
        ),
        (
            "audit",
            AUDIT,
            "if started.elapsed() > std::time::Duration::from_secs(2) {",
        ),
        (
            "launch",
            LAUNCH,
            "if start.elapsed() > std::time::Duration::from_secs(45)",
        ),
        (
            "indexstop",
            INDEXSTOP,
            "if start.elapsed() > std::time::Duration::from_secs(45)",
        ),
    ] {
        assert!(
            section.contains(&format!("`{quote}`")),
            "the D-0911 section does not quote `{quote}`"
        );
        assert!(
            source.contains(quote),
            "{name} source no longer holds `{quote}`"
        );
    }
    // The child's two streams are the log file, inside run_fixture_child.
    let child = &CHILD[CHILD
        .find("\nfn run_fixture_child(")
        .unwrap_or_else(|| panic!("run_fixture_child is gone"))..];
    let child = &child[..child[1..]
        .find("\nfn ")
        .unwrap_or_else(|| panic!("nothing follows run_fixture_child"))];
    for stream in [
        ".stdout(file.try_clone().map_err(display)?)",
        ".stderr(file)",
    ] {
        assert!(child.contains(stream), "run_fixture_child lost `{stream}`");
    }
    // Gate 1e prints its captured run only after `cargo test` has returned.
    let gate = job("language-purity");
    let captured = gate
        .find("tout=\"$(PATH=\"$stub:$PATH\" cargo test --workspace --locked 2>&1)\"")
        .unwrap_or_else(|| panic!("Gate 1e's capture is gone"));
    // A command substitution returns only when `cargo test` does, and the
    // gate's next line takes its status: nothing is printed in between.
    let next = gate[captured..]
        .lines()
        .nth(1)
        .unwrap_or_else(|| panic!("Gate 1e ends at its capture"));
    assert_eq!(next.trim(), "tstatus=$?");
}

//! The parallel rungs' repeated reads of one minute series, stated where they
//! are paid (audit o1cli-3).
//!
//! `sweep_rungs` runs every rung through `one_rung` in parallel, and each rung
//! reads the same one-minute span for itself, several times. Nothing stated
//! it. This file holds `docs/06-limits.md` to the code: the section must state
//! the count, and every read it counts must still be where it says.
//!
//! Since D-1840 the rungs share those reads: this file now holds the fix.

#![allow(
    clippy::expect_used,
    clippy::panic,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail. An integration test file is \
              its own crate root, so the attribute cannot be inherited."
)]

const LIB: &str = include_str!("../src/lib.rs");
const LIMITS: &str = include_str!("../../../docs/06-limits.md");

/// `text` with every run of whitespace folded to one space, so a sentence
/// compares by its words wherever it is wrapped.
fn words(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The `docs/06-limits.md` section headed `heading`, up to the next `## `,
/// folded.
fn limit(heading: &str) -> String {
    let from = LIMITS
        .find(heading)
        .unwrap_or_else(|| panic!("docs/06-limits.md has no `{heading}`"));
    let rest = LIMITS.get(from + heading.len()..).expect("a suffix");
    words(
        rest.get(..rest.find("\n## ").unwrap_or(rest.len()))
            .expect("the section"),
    )
}

/// The body of the top-level function whose head is `head`, up to its first
/// closing brace in column zero.
fn body(head: &str) -> &'static str {
    let from = LIB
        .find(head)
        .unwrap_or_else(|| panic!("no `{head}` in lib.rs"));
    LIB.get(from..)
        .and_then(|rest| rest.find("\n}\n").and_then(|to| rest.get(..to)))
        .expect("its body")
}

/// THE RUNGS OF ONE COMMAND READ THE SHARED SPANS ONCE. D-1840.
///
/// Every rung read the execution series, and the minute and daily context
/// spans once per build pass and once more after; an all-rungs command now
/// hands every rung one `SpanShare`, and a build pass reads nothing.
#[test]
fn the_rungs_of_one_command_read_the_shared_spans_once() {
    let limit = limit("## Parallel rungs each re-read the same one-minute span (audit o1cli-3)");
    for sentence in [
        "Fixed by D-1840",
        "`sweep_rungs` hands every rung one `SpanShare`",
        "three reads per command",
        "`rungs_share_their_reads_and_build_their_column_once`",
    ] {
        assert!(
            limit.contains(sentence),
            "the limit must say: {sentence}\n{limit}"
        );
    }
    let sweep = body("\nfn sweep_rungs(");
    // D-1556: the rungs run as ordered lanes, still in parallel.
    assert!(sweep.contains("ordered::map(") && sweep.contains("one_rung_cached("));
    assert!(sweep.contains("SpanShare::default()") && sweep.contains("AuditCache::sharing("));
    let build = body("\nfn column_withholding_at_build(");
    let retried = build
        .split_once("for _ in 0..ATTEMPTS")
        .expect("the retry loop")
        .1;
    assert!(retried.contains("share.span(") && !retried.contains("stored::load_"));
    let kernel = body("\nfn load_audit_inputs(");
    assert!(kernel.contains("share.span(at.of(EXECUTION_RUNG, (from, to)))"));
    assert!(!kernel.contains("stored::load_span(root, vendor, underlying, EXECUTION_RUNG"));
    assert!(!kernel.contains("exact_minute_withholding_unsourceable_days("));
    let share = body("\nimpl SpanShare {");
    assert!(share.contains(".find(|(key, _)| *key == at)") && share.contains("held.push("));
}

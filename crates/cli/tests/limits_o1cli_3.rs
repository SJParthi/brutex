//! The parallel rungs' repeated reads of one minute series, stated where they
//! are paid (audit o1cli-3).
//!
//! `sweep_rungs` runs every rung through `one_rung` (one at a time since
//! D-1701; in parallel before), and each rung
//! reads the same one-minute span for itself, several times. Nothing stated
//! it. This file holds `docs/06-limits.md` to the code: the section must state
//! the count, and every read it counts must still be where it says.

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

const STORED: &str = include_str!("../src/stored.rs");

/// THE REPEATED ONE-MINUTE READS ACROSS PARALLEL RUNGS ARE STATED, AND STILL
/// PAID.
#[test]
fn the_parallel_rungs_repeated_minute_reads_are_stated_and_still_paid() {
    let limit = limit("## Parallel rungs each re-read the same one-minute span (audit o1cli-3)");
    for sentence in [
        "`sweep_rungs` runs every rung through `one_rung`, one rung at a time in input order",
        "the execution series `audit_range_kernel` loads",
        "one per attempt of the column build",
        "one per attempt of `exact_minute_withholding_unsourceable_days`",
        "at least three reads of that rung's one-minute span, derived support or named",
        "some 24 reads of identical minutes per command",
        "up to 64 attempts",
    ] {
        assert!(
            limit.contains(sentence),
            "the limit must say: {sentence}\n{limit}"
        );
    }
    let sweep = body("\nfn sweep_rungs(");
    // One rung at a time since D-1701 (GAP13-13): the reads are the same
    // count, paid in sequence rather than at once.
    assert!(
        sweep.contains("in_input_order(")
            && sweep.contains("one_rung(")
            && !sweep.contains("par_iter")
    );
    let exact = STORED
        .split_once("pub fn load_exact_minute_context(")
        .and_then(|(_, rest)| rest.split_once("\n}\n"))
        .expect("`load_exact_minute_context` is in stored.rs")
        .0;
    assert!(
        exact.contains("load_span(root, vendor, underlying, \"1min\""),
        "the exact-minute context no longer reads the minute span: update the limit"
    );
    let build = body("\nfn column_withholding_at_build(");
    let retried = build
        .split_once("for _ in 0..ATTEMPTS")
        .expect("the retry loop")
        .1;
    assert!(retried.contains("load_exact_minute_context("));
    assert!(build.contains("const ATTEMPTS: usize = 64;"));
    let withholding = body("\nfn exact_minute_withholding_unsourceable_days(");
    let retried = withholding
        .split_once("for _ in 0..ATTEMPTS")
        .expect("its retry loop")
        .1;
    assert!(retried.contains("load_exact_minute_context("));
    // D-1557: the kernel's loads moved into its cached loader.
    let kernel = body("\nfn load_audit_inputs(");
    for call in [
        "stored::load_span(root, vendor, underlying, EXECUTION_RUNG",
        "column_withholding_at_build(",
        "exact_minute_withholding_unsourceable_days(",
    ] {
        assert!(
            kernel.contains(call),
            "the kernel no longer calls {call}: update the limit"
        );
    }
    // D-4719: a derived support reads the kernel's own build, so it adds no
    // read of its own.
    assert!(body("\nfn one_rung_cached(").contains("load_audit_inputs("));
    assert!(!LIB.contains("fn column_withholding_unsourceable_days("));
}

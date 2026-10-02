//! The audit kernel's second read of its daily and minute contexts, stated
//! where it is paid (audit o1cli-4).
//!
//! `column_withholding_at_build` loads the daily and exact-minute contexts,
//! digests them and drops them; the kernel then loads both again and
//! recomputes the digest to prove nothing changed. Nothing stated that cost.
//! This file holds `docs/06-limits.md` to the code.

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

/// THE KERNEL'S SECOND CONTEXT READ AND DIGEST ARE STATED, AND STILL PAID.
#[test]
fn the_kernels_second_context_read_is_stated_and_still_paid() {
    let limit =
        limit("## The audit kernel reads its daily and minute contexts twice (audit o1cli-4)");
    for sentence in [
        "`column_withholding_at_build` loads the daily context and the exact-minute context",
        "digests them with `stored_anchored_digest`, and drops them",
        "`exact_minute_withholding_unsourceable_days` and `stored::load_daily_context` then load both again",
        "recomputes the digest and compares it with the preparation digest",
        "one extra read of the rung's one-minute span (from the month before the span) and of its daily context",
        "O(minute bars + signal bars)",
    ] {
        assert!(
            limit.contains(sentence),
            "the limit must say: {sentence}\n{limit}"
        );
    }
    let build = body("\nfn column_withholding_at_build(");
    for call in [
        "stored::load_daily_context(",
        "stored::load_exact_minute_context(",
        "stored_anchored_digest(",
    ] {
        assert!(
            build.contains(call),
            "the build no longer calls {call}: update the limit"
        );
    }
    assert!(
        build.contains("return Ok((column, digest));"),
        "the build hands back the column and digest, not the contexts: update the limit"
    );
    let kernel = body("\nfn audit_range_kernel(");
    let after = kernel
        .split_once("column_withholding_at_build(")
        .expect("the kernel builds its column")
        .1;
    for call in [
        "exact_minute_withholding_unsourceable_days(",
        "stored::load_daily_context(",
        "stored_anchored_digest(&span.bars, &exact_minute, &daily)? != preparation_digest",
    ] {
        assert!(
            after.contains(call),
            "the kernel no longer calls {call} after the build: update the limit"
        );
    }
}

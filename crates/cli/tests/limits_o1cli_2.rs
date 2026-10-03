//! The all-rungs support probe's second load and build, stated where it is
//! paid (audit o1cli-2).
//!
//! `one_rung` loads a rung's span and, when no support is named, builds its
//! column to size the affordable floor; `audit_range_for_attempt` then loads
//! and builds the same span again. Only a code comment admitted it. This file
//! holds `docs/06-limits.md` to the code: the section must state both costs,
//! and the calls it describes must still be where it says, so the day the
//! span is threaded through, this fails and the limit is withdrawn with it.

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

/// THE SECOND LOAD AND BUILD PER RUNG ARE STATED, AND STILL TRUE.
#[test]
fn a_rungs_second_load_and_build_are_stated_and_still_paid() {
    let limit =
        limit("## A rung loads its span twice and may build its column twice (audit o1cli-2)");
    for sentence in [
        "`one_rung` loads the rung's span with `stored::load_span`",
        "the audit kernel `audit_range_kernel` then loads the same span again",
        "When no support is named, the column is also built twice",
        "`column_withholding_unsourceable_days` for `affordable_min_hits`, then `column_withholding_at_build`",
        "two span loads per rung always, and two column builds",
        "O(rung bars) each",
    ] {
        assert!(
            limit.contains(sentence),
            "the limit must say: {sentence}\n{limit}"
        );
    }
    let rung = body("\nfn one_rung(");
    for call in [
        "stored::load_span(",
        "column_withholding_unsourceable_days(",
        "affordable_min_hits(",
        "audit_range_for_attempt(",
    ] {
        assert!(
            rung.contains(call),
            "`one_rung` no longer calls {call}: update the limit"
        );
    }
    assert!(
        rung.find("stored::load_span(") < rung.find("named_ppm.is_some()"),
        "the span is loaded before the named-support branch, so even a named support pays the first load"
    );
    let kernel = body("\nfn audit_range_kernel(");
    for call in ["stored::load_span(", "column_withholding_at_build("] {
        assert!(
            kernel.contains(call),
            "the kernel no longer calls {call}: update the limit"
        );
    }
}

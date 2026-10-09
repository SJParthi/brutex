//! The all-rungs support probe's second load and build, stated where it is
//! paid (audit o1cli-2).
//!
//! `one_rung` loads a rung's span; the audit kernel then loads the same span
//! again. Until D-4719 a derived support also built its own column first, and
//! the kernel built it again. This file holds `docs/06-limits.md` to the code:
//! the section must state the costs, and the calls it describes must still be
//! where it says, so the day the span is threaded through, this fails and the
//! limit is withdrawn with it.

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

/// THE SECOND LOAD PER RUNG IS STATED, AND STILL TRUE; THE SECOND BUILD IS
/// GONE (D-4719).
#[test]
fn a_rungs_second_load_and_build_are_stated_and_still_paid() {
    let limit = limit("## A rung loads its span twice and builds its column once (audit o1cli-2)");
    for sentence in [
        "`one_rung` loads the rung's span with `stored::load_span`",
        "the audit kernel `audit_range_kernel` then loads the same span again",
        "The column is built once per rung whether or not a support is named",
        "both branches of `one_rung_cached` read `load_audit_inputs` through the `AuditCache` the audit then reads",
        "two span loads per rung always, and one column build",
        "O(rung bars) each",
    ] {
        assert!(
            limit.contains(sentence),
            "the limit must say: {sentence}\n{limit}"
        );
    }
    // D-1557: `one_rung` is `one_rung_cached` with a fresh cache.
    let rung = body("\nfn one_rung_cached(");
    for call in [
        "stored::load_span(",
        "affordable_min_hits(",
        "audit_range_cached(",
    ] {
        assert!(
            rung.contains(call),
            "`one_rung` no longer calls {call}: update the limit"
        );
    }
    // D-4719: both support branches prepare through the audit's own loader,
    // and the derived branch's private build is gone.
    assert_eq!(
        rung.matches("load_audit_inputs(").count(),
        2,
        "each support branch reads the audit's inputs: update the limit"
    );
    assert!(
        !LIB.contains("fn column_withholding_unsourceable_days("),
        "a second column build is back: update the limit"
    );
    assert!(
        rung.find("stored::load_span(") < rung.find("named_ppm.is_some()"),
        "the span is loaded before the named-support branch, so even a named support pays the first load"
    );
    // D-1557: the kernel's loads moved into its cached loader.
    let kernel = body("\nfn load_audit_inputs(");
    for call in ["stored::load_span(", "column_withholding_at_build("] {
        assert!(
            kernel.contains(call),
            "the kernel no longer calls {call}: update the limit"
        );
    }
}

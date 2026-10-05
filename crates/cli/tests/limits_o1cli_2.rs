//! The all-rungs support probe's second load and build, stated where it is
//! paid (audit o1cli-2).
//!
//! `one_rung` loads a rung's span and, when no support is named, builds its
//! column to size the affordable floor; the audit kernel then loads
//! and builds the same span again. Only a code comment admitted it. This file
//! holds `docs/06-limits.md` to the code: the section must state both costs,
//! and the calls it describes must still be where it says, so the day the
//! span is threaded through, this fails and the limit is withdrawn with it.
//!
//! Since D-1840 the span is threaded through: this file now holds the fix.

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

/// A RUNG READS ITS SPAN AND BUILDS ITS COLUMN ONCE. D-1840.
///
/// `one_rung` read the raw span and, for a derived support, built its own
/// column; the kernel then read and built again. `one_rung_cached` now asks
/// the kernel's cached preparation and sizes the probe on its column.
#[test]
fn a_rung_reads_its_span_and_builds_its_column_once() {
    let limit =
        limit("## A rung loads its span twice and may build its column twice (audit o1cli-2)");
    for sentence in [
        "Fixed by D-1840",
        "asks the kernel's own preparation through `AuditCache::inputs`",
        "`rungs_share_their_reads_and_build_their_column_once`",
    ] {
        assert!(
            limit.contains(sentence),
            "the limit must say: {sentence}\n{limit}"
        );
    }
    let rung = body("\nfn one_rung_cached(");
    assert!(
        !rung.contains("stored::load_span("),
        "the rung reads its own span again"
    );
    assert!(
        !rung.contains("column_withholding"),
        "the rung builds its own column again"
    );
    assert!(rung.contains("cache.inputs(") && rung.contains("load_audit_inputs("));
    let probe = rung
        .split_once("affordable_min_hits(")
        .expect("the derived support is still probed")
        .1;
    assert!(probe.trim_start().starts_with("&inputs.column,"));
    assert!(probe.contains("inputs.preparation_digest"));
    assert!(
        rung.find("cache.inputs(") < rung.find("named_ppm.is_some()"),
        "both supports read the one preparation"
    );
    assert!(!LIB.contains("\nfn column_withholding_unsourceable_days("));
    assert!(!LIB.contains("raw: Option<(AuditKey"));
}

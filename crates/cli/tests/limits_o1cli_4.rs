//! The audit kernel's second read of its daily and minute contexts, stated
//! where it is paid (audit o1cli-4).
//!
//! `column_withholding_at_build` loads the daily and exact-minute contexts,
//! digests them and drops them; the kernel then loads both again and
//! recomputes the digest to prove nothing changed. Nothing stated that cost.
//! This file holds `docs/06-limits.md` to the code.
//!
//! Since D-1840 the build hands its contexts back: this file now holds the fix.

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

/// THE KERNEL SWEEPS THE CONTEXTS ITS COLUMN WAS BUILT FROM. D-1840.
///
/// The build loaded, digested and dropped both contexts, and the kernel read
/// them again and compared digests. The build now hands back the contexts of
/// the pass that built, and the kernel reads neither again.
#[test]
fn the_kernel_sweeps_the_contexts_its_column_was_built_from() {
    let limit =
        limit("## The audit kernel reads its daily and minute contexts twice (audit o1cli-4)");
    for sentence in [
        "Fixed by D-1840",
        "hands back the contexts of the pass that built",
        "`a_build_that_withholds_a_day_reads_nothing_more`",
    ] {
        assert!(
            limit.contains(sentence),
            "the limit must say: {sentence}\n{limit}"
        );
    }
    let build = body("\nfn column_withholding_at_build(");
    assert!(
        build.contains("Ok(PreparedColumn {")
            && build.contains("exact,\n                    daily,")
    );
    let kernel = body("\nfn load_audit_inputs(");
    let after = kernel
        .split_once("column_withholding_at_build(")
        .expect("the kernel builds its column")
        .1;
    for gone in [
        "exact_minute_withholding_unsourceable_days(",
        "stored::load_daily_context(",
        "stored::load_exact_minute_context(",
        "!= preparation_digest",
    ] {
        assert!(
            !after.contains(gone),
            "the kernel still calls {gone} after the build"
        );
    }
    assert!(kernel.contains("exact: exact_minute,") && kernel.contains("withheld: unsourceable,"));
}

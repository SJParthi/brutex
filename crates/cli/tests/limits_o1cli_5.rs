//! The span loaded only to read a reference price or a bar count, stated
//! where it is paid (audit o1cli-5).
//!
//! Four entry points load a whole stored span, read one number off it and
//! drop it, and the work they hand off loads the same span again. This file
//! holds `docs/06-limits.md` to the code.

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

/// THE EXTRA SPAN LOAD AT EACH OF THE FOUR ENTRY POINTS IS STATED, AND STILL
/// PAID.
#[test]
fn the_span_loaded_for_one_number_is_stated_and_still_paid() {
    let limit =
        limit("## Four commands load a span for one number, then load it again (audit o1cli-5)");
    for sentence in [
        "`elite_descend_in_points_inner` loads the span for `reference_price`, drops it",
        "`reference_of_span`, which `screen_arm` calls before `screen_range`",
        "`screen_range_in_points` loads the span for `reference_price` and the derived rules, then calls `screen_range`",
        "`descent_bar_count` loads the span for its bar count and the derived floors",
        "one extra full span load per command, O(span bars)",
        "the record count in a month's header is not the swept bar count",
    ] {
        assert!(
            limit.contains(sentence),
            "the limit must say: {sentence}\n{limit}"
        );
    }
    let points = body("\nfn elite_descend_in_points_inner(");
    assert!(points.contains("stored::load_span(") && points.contains("drop(span);"));
    assert!(points.contains("elite_descend_with_attempt("));
    let reference = body("\nfn reference_of_span(");
    assert!(reference.contains("stored::load_span(") && reference.contains("drop(loaded);"));
    let arm = body("\nfn screen_arm(");
    let after = arm
        .split_once("reference_of_span(")
        .expect("the arm asks for a reference")
        .1;
    assert!(
        after.contains("screen_range("),
        "the arm no longer screens after the reference: update the limit"
    );
    let screen = body("\npub fn screen_range_in_points(");
    let after = screen
        .split_once("stored::load_span(")
        .expect("it loads the span")
        .1;
    assert!(after.contains("drop(span);") && after.contains("screen_range("));
    let count = body("\nfn descent_bar_count(");
    assert!(count.contains("stored::load_span(") && count.contains("loaded.bars.len()"));
    assert!(body("\nfn elite_descend_with_attempt(").contains("descent_bar_count("));
    assert!(body("\npub fn screen_range(").contains("screen_range_inner("));
    // Since o1cli-1 (D-0997) the screen reaches its load through a per-command
    // cache: `screen_range_kernel_cached` loads once per key through
    // `load_screen_inputs`, so a lone screen still loads its own span.
    assert!(body("\nfn screen_range_inner(").contains("screen_range_kernel_cached("));
    // D-2101 routes that load through `ScreenCache::inputs`, which the
    // descent's swept-row count shares, so the kernel asks the cache and the
    // cache's one method calls the loader.
    assert!(body("\nfn screen_range_kernel_cached(").contains("cache.inputs("));
    let cache = LIB
        .split_once("\nimpl ScreenCache {")
        .expect("the screen cache has methods")
        .1
        .split_once("\n}\n")
        .expect("the impl closes")
        .0;
    assert!(cache.contains("load_screen_inputs("));
    assert!(
        body("\nfn load_screen_inputs(").contains("stored::load_span("),
        "the screen no longer loads its own span: update the limit"
    );
}

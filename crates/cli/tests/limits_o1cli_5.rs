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

/// THE SPAN READ FOR ONE NUMBER IS THE SPAN THE HANDED-OFF WORK READS. D-1839.
///
/// Each of the four entry points read a whole span for one number, dropped it,
/// and the work it handed off read the same months again. Each now reads it
/// through `read_signal_span` once and seeds the `ScreenCache` that work reads,
/// and nothing drops it to read it again.
#[test]
fn the_span_loaded_for_one_number_seeds_the_work_it_hands_off() {
    let limit =
        limit("## Four commands load a span for one number, then load it again (audit o1cli-5)");
    for sentence in [
        "Fixed by D-1839",
        "seeds the `ScreenCache` the handed-off work reads",
        "the record count in a month's header is not the swept bar count",
        "`a_span_read_for_one_number_seeds_the_screen_that_follows`",
    ] {
        assert!(
            limit.contains(sentence),
            "the limit must say: {sentence}\n{limit}"
        );
    }
    for head in [
        "\nfn elite_descend_in_points_inner(",
        "\nfn reference_of_span(",
        "\npub fn screen_range_in_points(",
        "\nfn descent_bar_count(",
        "\nfn descent_bar_count_at(",
    ] {
        let found = body(head);
        assert!(
            !found.contains("stored::load_span(") && !found.contains("drop(loaded);"),
            "{head} reads or drops the span outside the shared read"
        );
    }
    let points = body("\nfn elite_descend_in_points_inner(");
    assert!(points.contains("read_signal_span(") && points.contains("ScreenCache::seeded("));
    assert!(!points.contains("drop(span);"));
    assert!(points.contains("elite_descend_seeded("));
    let reference = body("\nfn reference_of_span(");
    assert!(reference.contains("read_signal_span(") && reference.contains("ScreenCache::seeded("));
    let arm = body("\nfn screen_arm(");
    let after = arm
        .split_once("reference_of_span(")
        .expect("the arm asks for a reference")
        .1;
    assert!(after.contains("screen_range_seeded(") && after.contains("seeded,"));
    let screen = body("\npub fn screen_range_in_points(");
    let after = screen
        .split_once("read_signal_span(")
        .expect("it reads the span")
        .1;
    assert!(!after.contains("drop(span);\n    let policy"));
    assert!(after.contains("ScreenCache::seeded(") && after.contains("screen_range_seeded("));
    assert!(body("\nfn descent_bar_count_at(").contains("cache\n        .signal_span(key)"));
    let descent = body("\nfn elite_descend_seeded(");
    assert!(descent.contains("&mut cache,") && !descent.contains("ScreenCache::default()"));
    assert!(body("\nfn load_screen_inputs(").contains("Some(span) => span,"));
    assert!(body("\nfn screen_range_kernel_cached(").contains("Some((held, span)) if held == key"));
}

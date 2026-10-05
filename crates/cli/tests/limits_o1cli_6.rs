//! The screen's two full sorts, stated where they are paid (audit o1cli-6).
//!
//! `screen` sorts every priced row on the money key and again on the
//! calendar key although only the top rows are measured and printed. Neither
//! sort was documented as a cost. This file holds `docs/06-limits.md` to the
//! code: the section must state both sorts and why the full order is needed,
//! and the code must still sort and read the whole order where it says.
//!
//! Since D-1842 the screen sorts only what it keeps: this file now holds the fix.

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

/// THE SCREEN SORTS ONLY WHAT IT KEEPS. D-1842.
///
/// The screen sorted every priced row twice; it now selects its measured band
/// and its printed top, sorts only those, and finds the final selection in
/// the unordered rest by key.
#[test]
fn the_screen_sorts_only_what_it_keeps() {
    let limit = limit("## The screen sorts every priced row twice (audit o1cli-6)");
    for sentence in [
        "Fixed by D-1842",
        "O(n + band log band + top log top)",
        "`the_screens_selections_give_exactly_what_its_two_full_sorts_gave`",
    ] {
        assert!(
            limit.contains(sentence),
            "the limit must say: {sentence}\n{limit}"
        );
    }
    let screen = body("\nfn screen<'a>(");
    assert!(
        !screen.contains("sort_by_key("),
        "the screen sorts every row again"
    );
    assert!(screen.contains("rank_screened(") && screen.contains("final_selection_split("));
    let ranked = body("\nfn rank_screened(");
    assert_eq!(ranked.matches("least_first(").count(), 2);
    assert!(ranked.contains("(money_key(&r.cell), r.rank)"));
    assert!(ranked.contains("least_first(rows, shown, screen_order_key);"));
    let least = body("\nfn least_first<");
    assert!(
        least.contains("select_nth_unstable_by_key(")
            && least.contains("head.sort_unstable_by_key(")
    );
    let split = body("\nfn final_selection_split<'a>(");
    assert!(split.contains(".min_by_key(|row| screen_order_key(row))"));
    assert!(body("\nfn screen_order_key(").contains("r.rank,"));
}

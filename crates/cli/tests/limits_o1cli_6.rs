//! The screen's two full sorts, stated where they are paid (audit o1cli-6).
//!
//! `screen` sorts every priced row on the money key and again on the
//! calendar key although only the top rows are measured and printed. Neither
//! sort was documented as a cost. This file holds `docs/06-limits.md` to the
//! code: the section must state both sorts and why the full order is needed,
//! and the code must still sort and read the whole order where it says.

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

/// THE SCREEN'S TWO FULL SORTS ARE STATED, AND THE FULL ORDER IS STILL READ.
#[test]
fn the_screens_two_full_sorts_are_stated_and_the_full_order_still_read() {
    let limit = limit("## The screen sorts every priced row twice (audit o1cli-6)");
    for sentence in [
        "`screen` sorts every priced row twice with a stable `sort_by_key`",
        "once on `money_key`, then on the calendar key",
        "O(n log n) for n priced rows, up to `SCREEN_CAP_CEILING` (10,000,000)",
        "`final_selection` falls back past the top rows",
        "an unmeasured row that passed the rules can rise above measured rows that did not",
        "once per screen, not per candidate or per bar",
    ] {
        assert!(
            limit.contains(sentence),
            "the limit must say: {sentence}\n{limit}"
        );
    }
    assert!(LIB.contains("const SCREEN_CAP_CEILING: usize = 10_000_000;"));
    // D-1734 split `screen` into `price_grids`, `tier_rows` and
    // `finish_screen`; the two sorts live in the last, which every screen
    // (and every finished tier of the walk) runs once.
    assert!(
        body("\nfn screen<'a>(").contains("finish_screen(rows,"),
        "`screen` finishes through `finish_screen`"
    );
    let screen = body("\nfn finish_screen<'a>(");
    assert_eq!(
        screen.matches("rows.sort_by_key(").count(),
        2,
        "the screen no longer sorts every row twice: update the limit"
    );
    let (_, after_money) = screen
        .split_once("rows.sort_by_key(|r| money_key(&r.cell));")
        .expect("the money sort");
    assert!(after_money.contains("measure_top(&mut rows"));
    let (_, after_calendar) = after_money
        .split_once("rows.sort_by_key(|r| {")
        .expect("the calendar sort");
    assert!(
        after_calendar.contains("r.admitted,"),
        "the calendar key leads with `admitted`"
    );
    assert!(after_calendar.contains("final_selection(&rows, rules)"));
    let selection = body("\nfn final_selection<'a>(");
    assert!(
        selection.contains(".or_else(|| rows.iter().find(|row| row.cell.trades > 0))"),
        "the selection no longer falls back past the top: update the limit"
    );
}

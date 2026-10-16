//! Three sentences of `docs/06-limits.md` that D-1204 found false, read where
//! they stand and checked against the source they describe.
//!
//! * §126 said each fold's execution prefix is cut by a binary
//!   `partition_point`, O(log E). The cut is a forward-only cursor,
//!   `MonotonicExecutionPrefix` in `crates/runner/src/validate.rs`, whose index only grows:
//!   all cuts of one family together cost O(E + F), amortised O(1) per
//!   execution bar (finding o1runner-9).
//! * §113 said a trade walk is linear in the signals it must decide. `walk_core`
//!   in `crates/runner/src/trade.rs` visits every row of the column and asks `fires` of each,
//!   so it is linear in the column's rows; the signal count bounds only the
//!   work after a row fires (finding o1runner-10).
//! * §96 said no build script exists in this repository and that gate 13 layer
//!   3 refuses one existing at all. `crates/cli/build.rs` exists, and gate 13
//!   allow-lists it by name (finding rustonly-3).
//!
//! Every file a constant below names is read at compile time, so a rename fails
//! the build rather than skipping the check. A separate test crate, as
//! `crates/cli/tests/pool_and_escape_docs.rs` is, because a document read into
//! the library's own tests would rebuild them whenever a line of it changed.
//! It lives under `cli`, not `runner`: gate 22 keeps the sweep crates from
//! including anything but documents and source, and this test reads the CI
//! workflow (D-1450).

#![allow(
    clippy::expect_used,
    clippy::panic,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail. An integration test file is \
              its own crate root, so the attribute cannot be inherited."
)]

const LIMITS: &str = include_str!("../../../docs/06-limits.md");
const VALIDATE: &str = include_str!("../../runner/src/validate.rs");
const TRADE: &str = include_str!("../../runner/src/trade.rs");
const CLI_BUILD: &str = include_str!("../build.rs");
const CI: &str = include_str!("../../../.github/workflows/ci.yml");

/// The body of one `### §N —` section, up to the next `### ` heading.
fn section(number: u32) -> &'static str {
    let heading = format!("### §{number} —");
    let start = LIMITS
        .find(&heading)
        .unwrap_or_else(|| panic!("docs/06-limits.md has no `{heading}` heading"));
    let body = &LIMITS[start + heading.len()..];
    let end = body.find("\n### ").unwrap_or(body.len());
    &body[..end]
}

/// `text` with every run of whitespace folded to one space, so a sentence
/// re-wrapped across lines is still found.
fn flat(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The body of `fn name(` in `source`, up to the next line that opens at the
/// left margin with `}`.
fn function<'a>(source: &'a str, signature: &str) -> &'a str {
    let start = source
        .find(signature)
        .unwrap_or_else(|| panic!("no `{signature}` in the source"));
    let body = &source[start..];
    let end = body.find("\n}\n").expect("the function closes");
    &body[..end]
}

#[test]
fn section_126_names_the_forward_cursor_that_cuts_fold_prefixes_not_a_binary_search() {
    // The source first: what the section must describe.
    let cursor = function(VALIDATE, "fn advance<'a>(");
    assert!(
        cursor.contains("while let Some(bar) = execution.get(self.end)")
            && cursor.contains("self.end = self.end.checked_add(1)?"),
        "MonotonicExecutionPrefix::advance is no longer a forward-only walk; \
         re-measure §126 before changing this test",
    );
    assert!(
        cursor.contains("boundary < previous"),
        "the cursor no longer refuses a decreasing boundary",
    );
    let search = concat!("partition", "_point");
    assert!(
        !VALIDATE.contains(search),
        "validate.rs cuts with a binary search again; §126 must say so",
    );

    let text = flat(section(126));
    assert!(
        !text.contains(search),
        "§126 still names a binary search the code does not perform",
    );
    assert!(
        !text.contains("O(log E)"),
        "§126 still prices the fold cut at O(log E)",
    );
    assert!(
        text.contains("MonotonicExecutionPrefix"),
        "§126 does not name the cursor that cuts the prefix",
    );
    assert!(
        text.contains("O(E + F)") && text.contains("amortised O(1) per execution bar"),
        "§126 does not state the cursor's bound: O(E + F) in all, amortised \
         O(1) per execution bar",
    );
    assert!(
        text.contains("not worst-case O(1) per cut"),
        "§126 must say one cut can walk many bars",
    );
}

#[test]
fn section_113_prices_a_trade_walk_by_the_column_rows_it_visits() {
    let walk = function(TRADE, "fn walk_core(");
    // Since D-1186 the walk starts at `first_row`, the first row that can
    // fire, and visits every row from there; `index` stays the column row.
    let row_loop = "for (offset, (bits, &signal)) in rows.enumerate() {";
    let at = walk
        .find(row_loop)
        .expect("walk_core no longer loops over the column rows; re-measure §113");
    assert!(
        flat(&walk[..at]).contains(".get(first_row..)"),
        "the walk no longer starts at its first live row; re-measure §113",
    );
    let after = flat(&walk[at + row_loop.len()..]);
    assert!(
        after.starts_with(
            "let index = first_row.saturating_add(offset); if !fires(bits, index) { continue; }"
        ),
        "the row loop no longer asks `fires` of every row it visits first",
    );

    let text = flat(section(113));
    assert!(
        !text.contains("linear in the signals it must decide"),
        "§113 still prices a trade walk by its signals alone",
    );
    assert!(
        text.contains("A trade walk is linear in the rows of the column it walks"),
        "§113 does not state the row-linear bound",
    );
}

#[test]
fn section_96_names_the_one_build_script_gate_13_allows() {
    // The script exists: this constant could not compile otherwise.
    assert!(
        CLI_BUILD.contains("BRUTEX_COMMIT"),
        "crates/cli/build.rs no longer stamps the commit",
    );
    assert!(
        CI.contains("allow_build='crates/cli/build.rs 1"),
        "gate 13 no longer allow-lists crates/cli/build.rs by name",
    );

    let text = flat(section(96));
    assert!(
        !text.contains("No build script in this repository exists at all"),
        "§96 still says no build script exists",
    );
    assert!(
        !text.contains("refuses a tracked build script existing at all"),
        "§96 still says gate 13 refuses every build script",
    );
    assert!(
        text.contains("`crates/cli/build.rs`") && text.contains("D-0348"),
        "§96 does not name the one allowed build script and its decision",
    );
}

#[test]
fn the_helpers_find_what_they_are_asked_for_and_nothing_else() {
    assert!(section(113).contains("execution membership"));
    assert!(!section(113).contains("### §114"));
    assert_eq!(flat(" a\n  b\tc "), "a b c");
    let source = "fn one() {\n    1\n}\nfn two() {\n}\n";
    assert_eq!(function(source, "fn one("), "fn one() {\n    1");
}

#[test]
#[should_panic(expected = "has no `### §9999 —` heading")]
fn a_missing_section_is_a_failure_not_an_empty_body() {
    let _ = section(9999);
}

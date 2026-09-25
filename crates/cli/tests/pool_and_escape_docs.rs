//! Sentences D-0696's fifth correction found false, read where they stand.
//!
//! Each described code in this crate and said more than the code does, and no
//! other test reads a doc comment, an invariant row, a decision or a limit. So
//! each is read here beside the code it describes, at compile time: a rename
//! fails the build rather than skipping the check. A separate test crate,
//! because a document read into the library's own tests would rebuild them
//! whenever any line of the ledger changed.

#![allow(
    clippy::expect_used,
    clippy::panic,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail. An integration test file is \
              its own crate root, so the attribute cannot be inherited."
)]

const POOL: &str = include_str!("../src/pool.rs");
const STORED: &str = include_str!("../src/stored.rs");
const LIB: &str = include_str!("../src/lib.rs");
const SEARCH_READER: &str = include_str!("../src/index_stop_search_reader.rs");
const SNAPSHOT_CODEC: &str = include_str!("../src/index_stop_source_context_codec.rs");
const INVARIANTS: &str = include_str!("../../../docs/04-invariants.md");
const DECISIONS: &str = include_str!("../../../docs/05-decisions.md");
const LIMITS: &str = include_str!("../../../docs/06-limits.md");

/// `text` with every `///` dropped and every run of whitespace folded to one
/// space, so a sentence compares by its words wherever it is wrapped.
fn words(text: &str) -> String {
    text.replace("///", " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// The text from `start` up to the next `end` after it, folded.
fn section(doc: &str, start: &str, end: &str) -> String {
    let from = doc
        .find(start)
        .unwrap_or_else(|| panic!("no `{start}` in the document"));
    let rest = doc.get(from..).expect("a suffix");
    let to = rest
        .get(start.len()..)
        .and_then(|after| after.find(end))
        .map_or(rest.len(), |at| at + start.len());
    words(rest.get(..to).expect("the section"))
}

/// The doc comment directly above `item` in `source`, folded: everything
/// from the blank line before it.
fn doc_above(source: &str, item: &str) -> String {
    let at = source
        .find(item)
        .unwrap_or_else(|| panic!("no `{item}` in the source"));
    let head = source.get(..at).expect("a prefix");
    let from = head.rfind("\n\n").map_or(0, |gap| gap + 2);
    let doc = words(head.get(from..).expect("its doc"));
    assert!(
        doc.len() > 200,
        "the doc of `{item}` is {} bytes",
        doc.len()
    );
    doc
}

/// The body of the function whose head is `head`, up to its closing brace at
/// `indent`.
fn body<'a>(source: &'a str, head: &str, indent: &str) -> &'a str {
    let from = source
        .find(head)
        .unwrap_or_else(|| panic!("no `{head}` in the source"));
    let close = format!("\n{indent}}}\n");
    source
        .get(from..)
        .and_then(|rest| rest.find(&close).and_then(|to| rest.get(..to)))
        .expect("its body")
}

/// **Every `BRUTEX_` variable the driven `pool` test gives its child is named
/// wherever the child is described.** D-0696.
///
/// The doc of `the_pool_verb_prints_its_whole_page_on_a_generated_store`,
/// AF-39 and D-0696's fourth correction said the child's environment names a
/// generated store and carries no other `BRUTEX_` variable. The test gives it
/// `BRUTEX_LOG_DIR` and its own marker as well (found by a review). The names
/// are read from the test's own `.env(` calls, so a variable added there
/// fails here until each of the three texts names it.
#[test]
fn each_variable_the_driven_pool_test_gives_its_child_is_named() {
    let head = "    #[test]\n    fn the_pool_verb_prints_its_whole_page_on_a_generated_store()";
    let test = body(POOL, head, "    ");
    let marker = test
        .split("const CHILD: &str = \"")
        .nth(1)
        .and_then(|rest| rest.split('"').next())
        .expect("the child's marker");
    let mut given = vec![marker];
    for call in test.split(".env(").skip(1) {
        if let Some(literal) = call.strip_prefix('"') {
            given.push(literal.split('"').next().expect("a literal name"));
        } else {
            assert!(
                call.starts_with("CHILD, "),
                "an `.env(` naming neither `CHILD` nor a literal: {call:.60}"
            );
        }
    }
    given.sort_unstable();
    given.dedup();
    assert_eq!(
        given,
        [
            "BRUTEX_LOG_DIR",
            "BRUTEX_STORE",
            "BRUTEX_TEST_POOL_VERB_PAGE"
        ],
        "premise: the names the child is given were read"
    );
    assert!(
        test.contains("if name.to_string_lossy().starts_with(\"BRUTEX_\") {")
            && test.contains("child.env_remove(name);"),
        "the child inherits no `BRUTEX_` variable from the shell:\n{test}"
    );

    let texts = [
        ("the driven test's doc", doc_above(POOL, head)),
        ("AF-39", section(INVARIANTS, "| AF-39 |", "\n")),
        (
            "D-0696's fourth correction, item 1",
            section(
                DECISIONS,
                "**Fourth correction, 2026-09-24: what a fourth review upheld.**",
                "\n2. *The last two links",
            ),
        ),
    ];
    for (place, text) in &texts {
        assert!(
            !text.contains("no other `BRUTEX_` variable"),
            "{place} says the child is given no other `BRUTEX_` variable"
        );
        assert!(
            text.contains("inherits no `BRUTEX_` variable from the shell that ran the suite"),
            "{place} says what the child inherits"
        );
        for name in &given {
            assert!(
                text.contains(&format!("`{name}`")),
                "{place} does not name `{name}`, which the child is given"
            );
        }
    }
}

/// **A word is said to be quoted as typed only when `escape_debug` prints each
/// of its characters as itself.** D-0696.
///
/// Five texts said a word or name "of printable characters", with no quote
/// and no backslash, is quoted as before. A combining mark prints as itself
/// inside a word and is escaped when it opens one: U+0301 followed by `NIFTY`
/// is quoted escaped, as `a_refused_word_is_quoted_escaped_on_one_line`
/// asserts (found by a review). Each text now names that rule and the
/// combining mark.
#[test]
fn no_text_says_a_printable_word_is_quoted_as_typed() {
    let texts = [
        (
            "`clipped`'s doc",
            doc_above(STORED, "\npub(crate) fn clipped("),
        ),
        (
            "`misfiled`'s doc",
            doc_above(STORED, "\npub(crate) fn misfiled("),
        ),
        (
            "the misfiled test's doc",
            doc_above(
                STORED,
                "    #[test]\n    fn a_misfiled_directory_name_is_escaped_onto_one_line(",
            ),
        ),
        (
            "the refused-word test's doc",
            doc_above(
                STORED,
                "    #[test]\n    fn a_refused_word_is_quoted_escaped_on_one_line(",
            ),
        ),
        (
            "D-0696's second correction, item 2",
            section(
                DECISIONS,
                "2. *Item 5: a directory name is printed raw.*",
                "\n3. *The first correction's item 4",
            ),
        ),
    ];
    for (place, text) in &texts {
        assert!(
            !text.contains("printable"),
            "{place} still speaks of printable characters: {text}"
        );
        assert!(
            text.contains("`escape_debug` prints each of its characters as itself")
                || text.contains("`escape_debug` prints every character of it as itself"),
            "{place} states when a word is quoted as typed: {text}"
        );
        assert!(
            text.contains("combining mark that opens"),
            "{place} names the combining mark that opens a word: {text}"
        );
    }
}

/// **The limit on raw quotes names `parse_vendor`'s two callers that hand it
/// stored bytes, and they still do.** D-0696.
///
/// The `docs/06-limits.md` bullet ended by saying that whether any caller
/// hands a raw quote a word that was not typed "was not examined". Two
/// callers hand `parse_vendor` a feed word decoded from stored bytes (found
/// by a review), and it now quotes through `stored::clipped`. The two calls,
/// `parse_vendor`'s quote and the bullet are read together, so moving any of
/// them fails here.
#[test]
fn the_raw_quote_limit_names_parse_vendors_stored_callers() {
    assert!(
        SEARCH_READER.contains("let feed = raw.text()?;\n        crate::parse_vendor(&feed)?;"),
        "a saved declaration's feed is decoded and handed to `parse_vendor`"
    );
    assert!(
        SNAPSHOT_CODEC.contains("let feed = crate::parse_vendor(read.text()?)?;"),
        "a saved snapshot's feed is decoded and handed to `parse_vendor`"
    );
    let parse_vendor = body(LIB, "\nfn parse_vendor(", "");
    assert!(
        parse_vendor.contains("stored::clipped(word),") && !parse_vendor.contains("`{word}`"),
        "`parse_vendor` quotes the word through `clipped` alone:\n{parse_vendor}"
    );
    let bullet = section(
        LIMITS,
        "- **Not every refusal quotes through `stored::clipped`",
        "\n- **",
    );
    assert!(!bullet.contains("was not examined"), "{bullet}");
    for named in [
        "`index_stop_search_reader.rs`",
        "`index_stop_source_context_codec.rs`",
        "`/index-stop-ranking.json`",
        "`/index-stop-candles.json`",
        "Either refusal refuses that whole response",
        "`parse_vendor` does",
    ] {
        assert!(bullet.contains(named), "the bullet names {named}: {bullet}");
    }
}

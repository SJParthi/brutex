//! Sentences D-0696's fifth, sixth and seventh corrections found false, read
//! where they stand.
//!
//! Each said more than holds, so each is read here. Every document and source
//! file a constant below names is read at compile time: a rename fails the
//! build rather than skipping the check. The `.rs` files under `src` are also
//! read when the test runs, so that a call of `cli::swept_rung` added in a new
//! file is counted too. A separate test crate, because a document read into
//! the library's own tests would rebuild them whenever any line of the ledger
//! changed.

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
const THIS: &str = include_str!("pool_and_escape_docs.rs");
const AUDIT: &str = include_str!("../../runner/src/audit.rs");
const EQUITY_TESTS: &str = include_str!("../src/equity_statement_tests.rs");

/// Counts this crate spells out, from zero.
const NUMBERS: [&str; 21] = [
    "zero",
    "one",
    "two",
    "three",
    "four",
    "five",
    "six",
    "seven",
    "eight",
    "nine",
    "ten",
    "eleven",
    "twelve",
    "thirteen",
    "fourteen",
    "fifteen",
    "sixteen",
    "seventeen",
    "eighteen",
    "nineteen",
    "twenty",
];

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

/// Every `.rs` file under this crate's `src`, as its path below `src` and
/// its text, read when the test runs, in path order.
fn sources() -> Vec<(String, String)> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut pending = vec![root.clone()];
    let mut found = Vec::new();
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(&dir).expect("a source directory") {
            let path = entry.expect("a directory entry").path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                let name = path
                    .strip_prefix(&root)
                    .expect("under src")
                    .to_string_lossy()
                    .into_owned();
                let text = std::fs::read_to_string(&path).expect("a source file");
                found.push((name, text));
            }
        }
    }
    found.sort();
    found
}

/// One call of `cli::swept_rung` in this crate's source.
struct Call {
    /// The file it is in, as its path below `src`.
    file: String,
    /// The name of the function it is in.
    function: String,
    /// What it hands `swept_rung`.
    argument: String,
    /// That function's text, from its name up to the call.
    lead: String,
}

/// The function whose head is the last to come before the end of `before`,
/// as its name and its text from that name on.
fn enclosing_fn(before: &str) -> (String, &str) {
    let head = [
        "\nfn ",
        "\npub fn ",
        "\npub(crate) fn ",
        "\n    fn ",
        "\n    pub fn ",
        "\n    pub(crate) fn ",
    ]
    .iter()
    .filter_map(|head| before.rfind(head).map(|at| at + head.len()))
    .max()
    .expect("a function around the call");
    let lead = before.get(head..).expect("its text");
    let name = lead.split(['(', '<']).next().expect("its name");
    (name.to_owned(), lead)
}

/// Each call of `cli::swept_rung` in this crate's source, in path order.
///
/// `batch.rs` has a `swept_rung` of its own, so there only a call through
/// `crate::` is one of these.
fn swept_rung_calls() -> Vec<Call> {
    let mut calls = Vec::new();
    for (file, text) in sources() {
        let needle = if file.starts_with("batch") {
            "crate::swept_rung("
        } else {
            "swept_rung("
        };
        for (at, _) in text.match_indices(needle) {
            let before = text.get(..at).expect("a prefix");
            // The definition is not a call, and neither is a unit test's
            // `super::swept_rung(..)` assertion on it (D-2724's test asserts
            // which rungs it accepts): the bullet counts production callers
            // that hand it a rung to refuse (D-2722).
            if before.ends_with("fn ") || before.ends_with("super::") {
                continue;
            }
            let argument = text
                .get(at + needle.len()..)
                .and_then(|after| after.split(')').next())
                .expect("an argument");
            let (function, lead) = enclosing_fn(before);
            calls.push(Call {
                file: file.clone(),
                function,
                argument: argument.to_owned(),
                lead: lead.to_owned(),
            });
        }
    }
    calls
}

/// **Every `BRUTEX_` variable the driven `pool` test gives its child is named
/// wherever the child is described.** D-0696.
///
/// The doc of `the_pool_verb_prints_its_whole_page_on_a_generated_store`,
/// AF-39 and D-0696's fourth correction said the child's environment names a
/// generated store and carries no other `BRUTEX_` variable. The test gives it
/// `BRUTEX_LOG_DIR` and its own marker as well (found by a review). The names
/// are read from the test's own `.env(` calls, so a variable added there
/// fails here until the list below and each of the three texts name it.
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

/// The five texts that say when a word is quoted as it was before `clipped`
/// escaped what it keeps, each folded.
fn quoted_as_before_texts() -> [(&'static str, String); 5] {
    [
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
    ]
}

/// **A word is said to be quoted as before only when `escape_debug` prints
/// each of the characters `clipped` keeps of it as itself.** D-0696.
///
/// Five texts said a word or name "of printable characters", with no quote
/// and no backslash, is quoted as before. A combining mark prints as itself
/// inside a word and is escaped when it opens one: U+0301 followed by `NIFTY`
/// is quoted escaped, as `a_refused_word_is_quoted_escaped_on_one_line`
/// asserts (found by a review). The fifth correction then said a word is
/// quoted as before only when `escape_debug` prints each of its characters
/// as itself, and that was false as well: `clipped` cuts before it escapes,
/// so 64 `N`s and then a newline are quoted as they always were, and the
/// refused-word test asserts exactly that (found by a later review). Each
/// text now names the characters `clipped` keeps, says what becomes of one
/// past the cut, and names the combining mark.
#[test]
fn no_text_says_a_printable_word_is_quoted_as_typed() {
    for (place, text) in &quoted_as_before_texts() {
        assert!(
            !text.contains("printable"),
            "{place} still speaks of printable characters: {text}"
        );
        assert!(
            !text.contains("prints each of its characters as itself")
                && !text.contains("prints every character of it as itself"),
            "{place} says a word is quoted as before only when every character of \
             it prints as itself: {text}"
        );
        assert!(
            text.contains("`escape_debug` prints each of the characters `clipped` keeps"),
            "{place} states when a word is quoted as before: {text}"
        );
        assert!(
            text.contains("past the cut"),
            "{place} says what becomes of a character past the cut: {text}"
        );
        assert!(
            text.contains("combining mark that opens"),
            "{place} names the combining mark that opens a word: {text}"
        );
    }
}

/// **Each text that names what `escape_debug` escapes names only what it
/// escapes.** D-0696.
///
/// The texts said it escapes a combining mark that opens a word, and some a
/// format or separator character. At the start of a word it escapes a mark
/// only when the mark has Unicode's `Grapheme_Extend` property: U+0903, a
/// spacing mark without it, prints as itself there. And the space is a
/// separator and prints as itself (found by a review, which measured both
/// with `rustc` outside the repository). The fifth correction also said that
/// it had measured the combining mark with `rustc`, and no record of that
/// measurement was found (found by the same review). So each text that names
/// the opening mark names the property, each separator it names is one other
/// than the space, and the fifth correction no longer claims the measurement.
/// `a_refused_word_is_quoted_escaped_on_one_line` and
/// `a_word_is_quoted_as_before_exactly_when_what_clipped_keeps_prints_as_itself`
/// must still quote the cases D-0696's sixth correction names.
#[test]
fn each_text_names_only_the_characters_escape_debug_escapes() {
    let fifth = section(
        DECISIONS,
        "2. *\"Printable\" words.*",
        "\n3. *`parse_vendor`'s callers that are not typed.*",
    );
    assert!(
        !fifth.contains("and this correction each measured"),
        "D-0696's fifth correction, item 2 claims a measurement it did not record: {fifth}"
    );
    let refused = body(
        STORED,
        "    fn a_refused_word_is_quoted_escaped_on_one_line(",
        "    ",
    );
    let as_before = body(
        STORED,
        "    fn a_word_is_quoted_as_before_exactly_when_what_clipped_keeps_prints_as_itself(",
        "    ",
    );
    for (test, case) in [
        (refused, "\"\\u{903}NIFTY\""),
        (refused, "\"\\u{93f}NIFTY\""),
        (refused, "\"NIFTY X\""),
        (refused, "\"\\u{20dd}NIFTY\""),
        (refused, "\"\\u{9be}NIFTY\""),
        (refused, "\"N\\u{9be}IFTY\""),
        (as_before, "\"\\u{903}NIFTY\""),
        (as_before, "for c in '\\0'..=char::MAX {"),
        (as_before, "'\\u{3000}'"),
        (as_before, "'\\u{2029}'"),
    ] {
        assert!(test.contains(case), "the test quotes {case}:\n{test}");
    }
    let mut texts = Vec::from(quoted_as_before_texts());
    texts.extend([
        ("AF-39", section(INVARIANTS, "| AF-39 |", "\n")),
        (
            "D-0696's fourth correction, item 4",
            section(
                DECISIONS,
                "   - *What a reader sees change, restated.*",
                "\n\n**Tests, and what each is proven against (fourth correction).**",
            ),
        ),
        ("D-0696's fifth correction, item 2", fifth),
        (
            "the refused-word test's comments",
            words(&refused.replace("//", " ")),
        ),
    ]);
    for (place, text) in &texts {
        assert!(
            text.contains("combining mark that opens"),
            "premise: {place} names the combining mark that opens a word: {text}"
        );
        assert!(
            text.contains("`Grapheme_Extend`"),
            "{place} names the opening combining mark without the property that \
             decides whether it is escaped: {text}"
        );
        assert!(
            !text.contains("format or separator"),
            "{place} says every separator is escaped: {text}"
        );
        assert_eq!(
            text.matches("a separator").count(),
            text.matches("a separator other than the space").count(),
            "{place} names a separator without excepting the space: {text}"
        );
    }
}

/// **This crate's own docs say no more than holds.** D-0696.
///
/// Its module doc said that no other test reads a doc comment, an invariant
/// row, a decision or a limit, and other tests in this workspace read each
/// of those (found by a review). The doc of
/// `each_variable_the_driven_pool_test_gives_its_child_is_named` said a
/// variable given to the child fails that test until the three texts name
/// it; it fails until the list in that test names it too (found by the same
/// review).
#[test]
fn this_crates_own_docs_say_no_more_than_holds() {
    let module = words(
        &THIS
            .lines()
            .take_while(|line| line.starts_with("//!"))
            .map(|line| line.trim_start_matches("//!"))
            .collect::<Vec<_>>()
            .join("\n"),
    );
    assert!(
        module.contains("Sentences D-0696's"),
        "premise: the module doc was read: {module}"
    );
    assert!(
        !module_claims_exclusive_readership(&module),
        "the module doc says no other test reads a document: {module}"
    );
    let named = doc_above(
        THIS,
        "#[test]\nfn each_variable_the_driven_pool_test_gives_its_child_is_named(",
    );
    assert!(
        named.contains("fails here until the list below and each of the three texts name it"),
        "the doc says what a variable added to the child fails until: {named}"
    );
}

/// Whether the module text contains the disallowed readership claim.
fn module_claims_exclusive_readership(module: &str) -> bool {
    module.to_ascii_lowercase().contains("no other test reads")
}

#[test]
fn the_module_doc_claim_guard_catches_each_ascii_case() {
    for claim in [
        "no other test reads a document",
        "No other test reads a document",
        "NO OTHER TEST READS a document",
    ] {
        assert!(module_claims_exclusive_readership(claim), "{claim}");
    }
    assert!(!module_claims_exclusive_readership(
        "Each document named below is read here."
    ));
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
    let bullet = raw_quote_bullet();
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

/// The `docs/06-limits.md` bullet on the refusals that do not quote through
/// `stored::clipped`, folded.
fn raw_quote_bullet() -> String {
    section(
        LIMITS,
        "- **Not every refusal quotes through `stored::clipped`",
        "\n- **",
    )
}

/// **The limit on raw quotes counts and names every call of
/// `cli::swept_rung`, and says where each call's rung comes from.** D-0696.
///
/// The bullet and D-0696's fifth correction said `cli::swept_rung` has eight
/// call sites, and the bullet that each takes the rung as a parameter. It had
/// eleven (found by two reviews), and `sweep_audited_stored` takes its rung
/// from the command's own argument list (found by one of them). The calls are
/// now found in the `.rs` files under `src` when this runs. Both texts must
/// give their count in words, the bullet must name each file, and each
/// `lib.rs` function, that calls it, and each call's rung must come from where
/// the bullet says.
#[test]
fn the_raw_quote_limit_counts_and_names_every_call_of_swept_rung() {
    let bullet = raw_quote_bullet();
    let calls = swept_rung_calls();
    let sites: Vec<String> = calls
        .iter()
        .map(|call| format!("{} in {}", call.function, call.file))
        .collect();
    let count = *NUMBERS
        .get(calls.len())
        .expect("a count this test can spell");
    let item_3 = section(
        DECISIONS,
        "3. *`parse_vendor`'s callers that are not typed.*",
        "\n\nThe three tests in",
    );
    for (place, text) in [
        ("the bullet", &bullet),
        ("D-0696's fifth correction, item 3", &item_3),
    ] {
        let said: Vec<&str> = NUMBERS
            .iter()
            .copied()
            .filter(|number| text.contains(&format!(" {number} call sites")))
            .collect();
        assert_eq!(
            said,
            [count],
            "{place} counts `cli::swept_rung`'s call sites, and {count} were \
             found: {sites:?}\n{text}"
        );
    }
    assert!(
        !bullet.contains("the rung as a parameter"),
        "the bullet says every caller takes the rung as a parameter: {bullet}"
    );
    for call in &calls {
        let named = if call.file == "lib.rs" {
            format!("`{}`", call.function)
        } else {
            assert_eq!(
                calls.iter().filter(|other| other.file == call.file).count(),
                1,
                "the bullet says one call each outside `lib.rs`: {sites:?}"
            );
            format!("`{}`", call.file)
        };
        assert!(
            bullet.contains(&named),
            "the bullet does not name {named}, which calls `cli::swept_rung`: {bullet}"
        );
        // Where the rung it hands on comes from: a parameter, a field of one,
        // or -- in `sweep_audited_stored` alone -- the command's arguments.
        let signature = call.lead.split(" {\n").next().expect("its signature");
        let from = match call.argument.as_str() {
            "rung" if call.function == "sweep_audited_stored" => {
                call.lead.contains("        rung,\n") && call.lead.contains("] = arguments\n")
            }
            "rung" => signature.contains("rung: &") || signature.contains("rung: Option<&"),
            "request.rung" => signature.contains("(request: "),
            "args.rung" => signature.contains("(args: &"),
            _ => false,
        };
        assert!(
            from,
            "`{}` in {} hands `swept_rung` `{}`; the bullet does not say where \
             that comes from:\n{signature}",
            call.function, call.file, call.argument
        );
    }
    let others = *NUMBERS
        .get(calls.len().saturating_sub(1))
        .expect("a count this test can spell");
    assert!(
        bullet.contains(&format!("The other {others} are")),
        "the bullet counts the calls besides `pool::run`'s: {bullet}"
    );
    assert!(
        LIB.contains(
            "\n        [\"sweep-audited-stored\", arguments @ ..] if arguments.len() == 9 => {\n            \
             command_report(out, sweep_audited_stored(arguments), \"AUDITED SWEEP\")\n"
        ),
        "`sweep_audited_stored` is handed the command's own argument list"
    );
}

/// **The `pool` verb reaches `pool::run`'s call of `cli::swept_rung` only
/// with a rung that call accepts, and the limit on raw quotes says so.**
/// D-0696.
///
/// `pool::run`'s call is one of those the test above counts. Each link by
/// which the verb
/// reaches it -- `dispatch`'s one `pool` arm, `pool_arm`'s search of
/// `EVERY_RUNG`, `pool::pool` handing `run` its rung as it is, and
/// `swept_rung`'s acceptance of every `EVERY_RUNG` entry -- is read from the
/// source.
#[test]
fn the_pool_verb_reaches_swept_rung_only_with_a_rung_it_accepts() {
    let bullet = raw_quote_bullet();
    let calls = swept_rung_calls();
    let sites: Vec<String> = calls
        .iter()
        .map(|call| format!("{} in {}", call.function, call.file))
        .collect();
    assert!(
        calls
            .iter()
            .any(|call| call.file == "pool.rs" && call.function == "run"),
        "premise: `pool::run` calls `cli::swept_rung`: {sites:?}"
    );
    assert_eq!(LIB.matches("[\"pool\", ").count(), 1, "one `pool` arm");
    assert!(
        LIB.contains(
            "\n        [\"pool\", v, r, fy, fm, ty, tm, mh] => pool_arm(out, v, r, (fy, fm), (ty, tm), mh),\n"
        ) && LIB.matches("pool_arm(").count() == 2,
        "`dispatch`'s `pool` arm is `pool_arm`'s one caller"
    );
    let arm = body(LIB, "\nfn pool_arm(", "");
    assert!(
        arm.contains("let Some(known) = EVERY_RUNG.iter().find(|r| **r == rung) else {")
            && arm.matches("pool::pool(").count() == 1
            && arm.contains("pool::pool(vendor, known, "),
        "`pool_arm` hands `pool::pool` a rung it found in `EVERY_RUNG`:\n{arm}"
    );
    let verb = body(POOL, "\npub fn pool(", "");
    assert!(
        verb.contains("    match run(vendor_word, rung, from, to, support_ppm) {"),
        "`pool::pool` hands `run` its rung as it is:\n{verb}"
    );
    let guard = body(LIB, "\nfn swept_rung(", "");
    assert!(
        guard.contains("    if EVERY_RUNG.contains(&rung) {\n        return Ok(());\n    }"),
        "`swept_rung` accepts every `EVERY_RUNG` entry:\n{guard}"
    );
    assert!(
        bullet.contains("`EVERY_RUNG`") && bullet.contains("never reaches that refusal"),
        "the bullet says where `pool::run`'s rung comes from: {bullet}"
    );
}

/// **No text says `pool` pass 1 lifts a section out of a report.** D-0696.
///
/// Pass 1 keeps each instrument's `one_rung_cached(..).outcome`, a ledger
/// record, and prints a table of its fields; no FINDINGS block reaches a pool
/// page. Since D-4700 that loop is `pool::screen_pass_one`, which `run_under`
/// calls; until then it was inline in `run_under` and called `one_rung`.
/// `range-all` and `range-rung` are what print sections lifted through
/// `validation_note`. Five texts said `range-all` and `pool` pass 1 keep
/// lifted sections (found by a review): a reader who believed them could
/// drop the pool opening's own statement.
#[test]
fn no_text_says_pool_pass_1_lifts_a_section() {
    let run_under = body(POOL, "\nfn run_under(", "");
    assert!(
        run_under.contains("let screened = screen_pass_one("),
        "premise: `pool` pass 1 is `screen_pass_one`:\n{run_under}"
    );
    let pass_one = body(POOL, "\npub(crate) fn screen_pass_one(", "");
    assert!(
        pass_one.contains("outcome: crate::one_rung_cached(")
            && pass_one.contains("\n            .outcome,\n        }\n"),
        "premise: pass 1 keeps each rung's outcome alone:\n{pass_one}"
    );
    // Cut at the tests module, not at the first `#[cfg(test)]`: since D-4700
    // pass 1's hold-back seam sits under `#[cfg(test)]` near the top of the
    // file, and a cut there scanned a sixth of the module.
    let production = POOL
        .split_once("\nmod tests {")
        .map(|(production, _)| production)
        .expect("the module keeps its tests below it");
    assert!(
        production.contains("\nfn price_all(") && production.contains("\npub(crate) fn mask_hex("),
        "premise: the scan reaches the end of the production module"
    );
    for lift in [
        "section_note(",
        "validation_note(",
        ".validation",
        ".retention",
    ] {
        assert!(
            !production.contains(lift),
            "premise: `pool` lifts no section, and `{lift}` is in it"
        );
    }
    let texts = [
        (
            "`equity_ranking_statement`'s doc",
            doc_above(LIB, "\nfn equity_ranking_statement("),
        ),
        (
            "the lifted-AUDIT test's doc in `runner::audit`",
            doc_above(
                AUDIT,
                "\n    #[test]\n    fn every_charge_statement_line_is_indented_so_a_lifted_audit_block_keeps_it_whole(",
            ),
        ),
        (
            "the FINDINGS test's doc",
            doc_above(
                EQUITY_TESTS,
                "\n#[test]\nfn a_stock_ranking_states_corporate_actions_inside_its_findings_block(",
            ),
        ),
        (
            "the limits bullet on three copies",
            section(
                LIMITS,
                "- **A stock report can say it up to three times.**",
                "\n- **",
            ),
        ),
        (
            "D-0696's first item",
            section(
                DECISIONS,
                "`validation_note` lifts FINDINGS whole, so the rung notes",
                "exit status changes.",
            ),
        ),
    ];
    for (place, text) in &texts {
        assert!(
            text.contains("`range-all`"),
            "premise: {place} was read: {text}"
        );
        assert!(
            !text.contains("and `pool` pass 1 keep")
                && !text.contains("and `pool` pass 1 discard")
                && !text.contains("and `pool` keep"),
            "{place} says `pool` pass 1 lifts a section: {text}"
        );
    }
    assert!(
        texts[0].1.contains("`pool` pass 1 is NOT such a lift"),
        "`equity_ranking_statement`'s doc says pass 1 is no lift"
    );
}

/// **No invariant row says its change has no decision entry.** D-0696.
///
/// AF-31 and AF-32 ended "the extension to these pages has no decision entry
/// yet" and "the surface change has no decision entry yet", while D-0696 in
/// the same tree records both changes and quotes those words as the reason
/// it exists (found by a review). `CLAUDE.md` §9 asks for an entry for every
/// locked choice, so a row may not say one is missing.
#[test]
fn no_invariant_row_says_its_change_has_no_decision_entry() {
    assert_eq!(
        INVARIANTS.matches("no decision entry yet").count(),
        0,
        "an invariant row says its change has no decision entry"
    );
    for (row, change) in [
        ("| AF-31 |", "D-0696 records the extension to these pages"),
        ("| AF-32 |", "D-0696 records the surface change"),
    ] {
        let text = section(INVARIANTS, row, "\n");
        assert!(text.contains(change), "{row} names its entry: {text}");
    }
}

/// **D-0696 cites the commit it records by its hash in this history.**
/// D-0696.
///
/// It named c8c5383c thirteen times. That commit is on no branch this one
/// merges into: this history carries its cherry-pick, 12916123, whose
/// `git patch-id --stable` is the same (found by two reviews; the ancestry
/// and the patch ids were measured with `git` when this was written, and
/// this test runs no `git`). So the entry names 12916123, and c8c5383c
/// appears only where it says what 12916123 was cherry-picked from.
#[test]
fn d_0696_cites_the_commit_it_records_by_its_hash_in_this_history() {
    let entry = DECISIONS
        .split("\n### D-0696 ")
        .nth(1)
        .expect("D-0696")
        .split("\n### D-")
        .next()
        .expect("its text");
    assert_eq!(
        entry.matches("c8c5383c").count(),
        entry.matches("cherry-picked from c8c5383c").count(),
        "D-0696 cites c8c5383c other than as what 12916123 was cherry-picked from"
    );
    assert!(
        entry.contains("Commit 12916123, cherry-picked from c8c5383c"),
        "D-0696 opens by naming the commit in this history"
    );
    assert!(
        entry.matches("12916123").count() >= 13,
        "every citation names 12916123"
    );
}

//! `docs/10-shared-core.md`, checked against the crate it describes.
//!
//! # Why this file exists
//!
//! That document records the boundary another repository consumes across, and **nothing
//! read it**. Four of its numbers went stale within a day of being written: it claimed
//! seven modules where there are nine position sources, 205 positions where there are
//! 234, 274 allocated where there are 276, and a 1024-byte evaluator budget that had
//! already been raised to 1792.
//!
//! Every one of those was correct when written. That is the whole problem: a number
//! copied out of the code cannot be wrong at the moment of copying and cannot stay
//! right afterwards. A consumer reading a stale boundary document sizes its own buffers
//! against a number this crate no longer honours.
//!
//! So the numbers are read back out of the prose and compared to the code. A drift is a
//! **build failure**, not something a reader might notice.
//!
//! # The helpers are checked too
//!
//! Every check below reads the document through five short functions, and a helper that
//! quietly returns the wrong window turns a real drift into a green build. That is the
//! one failure this file cannot notice about itself, so the helpers are exercised
//! directly at the end -- including the failure path, which is the one a reader meets
//! first when the document is reworded.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "the same exception every test module in this workspace takes: a test \
              that cannot panic cannot fail."
)]

use indicators::evaluator::Evaluator;

/// The boundary document. Read at compile time so a rename fails the build rather
/// than skipping the check.
const SHARED_CORE_DOC: &str = include_str!("../../../docs/10-shared-core.md");

/// The evaluator's own source, for the one claim that is about a line of code rather
/// than a measurement: the ceiling in its `const` assertion.
const EVALUATOR_SRC: &str = include_str!("../src/evaluator.rs");

/// Every integer in `text`, in order.
fn numbers(text: &str) -> Vec<usize> {
    text.split(|c: char| !c.is_ascii_digit())
        .filter(|t| !t.is_empty())
        .filter_map(|t| t.parse().ok())
        .collect()
}

/// The document with every run of whitespace collapsed to one space.
///
/// Markdown wraps prose at the column, so `276` and `108 free` sit on different
/// physical lines of one sentence. A line-based check reads half a claim and reports a
/// number as missing when it is one newline away -- which is a false failure, and a
/// false failure teaches a reader to disable the test.
fn flat() -> String {
    SHARED_CORE_DOC
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Where `needle` starts, or a failure that names the phrase which is gone.
///
/// The only place this file panics for a reworded document, and every window below
/// opens at an offset this returns. `.expect` on the `Option` cannot say WHICH phrase
/// disappeared, and `called Option::unwrap() on a None value` sends the reader to this
/// file instead of to the document that changed.
fn offset(doc: &str, needle: &str) -> usize {
    doc.find(needle)
        .unwrap_or_else(|| panic!("`docs/10-shared-core.md` does not contain `{needle}`"))
}

/// The text from the first occurrence of `needle` onwards, for quoting in a failure.
///
/// `span` is the width of the WHOLE window in characters, the needle included -- not
/// the count after it. Long enough to carry the numbers belonging to the claim, short
/// enough not to reach the next one. A span wider than what remains is clamped to the
/// end of the document rather than slicing past it.
fn near<'a>(doc: &'a str, needle: &str, span: usize) -> &'a str {
    let at = offset(doc, needle);
    let end = doc[at..]
        .char_indices()
        .map(|(i, _)| at + i)
        .nth(span)
        .unwrap_or(doc.len());
    &doc[at..end]
}

/// `"Nine"` and `"nine"` are numbers too. Prose reads better with the word, and a
/// document written for a human is allowed to spell it -- so the test learns the words
/// rather than the document losing them.
///
/// The word is taken from where it appears in `text`, not from the order of the table
/// below. Asking `contains` in table order reads the wrong count out of any claim
/// naming two number words, and section 1's own heading is one: "The **six** crates
/// that are shareable, and the **one** change that made them so" answers 1 under table
/// order, because `one` is the first entry.
fn word_number(text: &str) -> Option<usize> {
    const WORDS: [(&str, usize); 12] = [
        ("one", 1),
        ("two", 2),
        ("three", 3),
        ("four", 4),
        ("five", 5),
        ("six", 6),
        ("seven", 7),
        ("eight", 8),
        ("nine", 9),
        ("ten", 10),
        ("eleven", 11),
        ("twelve", 12),
    ];
    let lower = text.to_ascii_lowercase();
    WORDS
        .iter()
        .filter_map(|(word, count)| lower.find(*word).map(|at| (at, *count)))
        .min_by_key(|(at, _)| *at)
        .map(|(_, count)| count)
}

/// The count a claim states, spelled or in digits.
///
/// The document spells the source count today, but the row could as easily have read
/// "9 position sources". Both readings are accepted so that rewording the prose is not
/// a build failure while the number is still right.
fn stated_count(claim: &str) -> Option<usize> {
    word_number(claim).or_else(|| numbers(claim).first().copied())
}

/// The position count the document advertises is the count the evaluator produces.
///
/// This is the number a consumer allocates against. If it is low, the consumer drops
/// conditions it was told did not exist.
#[test]
fn the_documented_position_count_is_the_count_the_evaluator_emits() {
    let computed = Evaluator::positions().len();
    let doc = flat();
    let claim = near(&doc, "position sources", 40);
    let quoted = numbers(claim);
    assert!(
        quoted.contains(&computed),
        "the document's `indicators` row reads {quoted:?} and `Evaluator::positions()` \
         returns {computed}. Claim: {claim}"
    );
}

/// The union is over as many sources as the document says it is.
///
/// Counted from the code, not asserted as a literal: every `all.extend(` in
/// `Evaluator::positions` is one source. The count was "seven" in prose while the
/// function had nine `extend` calls, which is exactly the drift this catches.
#[test]
fn the_documented_source_count_is_the_number_of_extends() {
    let sources = EVALUATOR_SRC.matches("all.extend(").count();
    let doc = flat();
    // The count sits BEFORE the phrase -- "nine position sources" -- so the window
    // opens ahead of it.
    let at = offset(&doc, "position sources");
    let claim = &doc[at.saturating_sub(24)..at + 16];
    let stated = stated_count(claim);
    assert_eq!(
        stated,
        Some(sources),
        "`Evaluator::positions` unions {sources} sources and the document says \
         {stated:?}. Claim: {claim}"
    );
    assert!(
        sources >= 2,
        "if `all.extend(` stopped being how sources are added, this test silently \
         measures nothing -- it found {sources}"
    );
}

/// The evaluator's size budget, and the measurement beside it.
///
/// Two separate claims. The ceiling must equal the `const` assertion in the source,
/// and the measured size must be the real `size_of`. Recording both is what makes the
/// assertion a live guard rather than a number rounded so high it can never fire.
#[test]
fn the_documented_evaluator_budget_is_the_assertion_and_the_measurement() {
    let measured = core::mem::size_of::<Evaluator>();

    let ceiling = EVALUATOR_SRC
        .split("size_of::<Evaluator>() <=")
        .nth(1)
        .map(numbers)
        .and_then(|n| n.first().copied())
        .expect("`evaluator.rs` asserts a `size_of::<Evaluator>()` ceiling");

    let doc = flat();
    // The assertion as the document quotes it, not the table row that merely mentions
    // the function by name.
    let claim = near(&doc, "assert!(size_of::<Evaluator>()", 60);
    let stated = numbers(claim);
    assert!(
        stated.contains(&ceiling),
        "the source asserts a ceiling of {ceiling} and the document quotes {stated:?}. \
         Claim: {claim}"
    );

    // The measurement sits BEFORE the phrase -- "measures 1664 bytes today" -- so the
    // window opens ahead of it. `near` has already established the phrase is present.
    let taken = near(&doc, "bytes today", 40);
    let at = offset(&doc, "bytes today");
    let before = &doc[at.saturating_sub(24)..at];
    let measurement = numbers(before);
    assert!(
        measurement.contains(&measured),
        "`size_of::<Evaluator>()` is {measured} and the document's measurement reads \
         {measurement:?}. §3 rule 6 forbids reporting a measurement that was not \
         taken. Claim: {before}{taken}"
    );

    assert!(
        measured <= ceiling,
        "the evaluator is {measured} bytes against a {ceiling}-byte ceiling; the \
         `const` assertion in `evaluator.rs` should already have failed the build"
    );
}

/// The mask width and the allocated count, both of which the document quotes twice.
///
/// Every occurrence is checked, not just the first: the two `276`s were written in
/// different sections and rotted together, so a check that stops at the first one
/// would have passed on a half-corrected document.
#[test]
fn every_quoted_vocabulary_number_matches_the_table() {
    let allocated = usize::from(vocab::table::NEXT_FREE);
    let width = usize::try_from(vocab::ConditionMask::BITS).unwrap();
    let free = width - allocated;

    let doc = flat();

    // Both phrasings quote the allocated count. The count sits BEFORE each phrase, so
    // each window opens ahead of it.
    let phrases = ["allocated positions", "fixed array of"];
    let mut checked = 0;
    for phrase in phrases {
        let at = offset(&doc, phrase);
        let window = &doc[at.saturating_sub(30)..(at + phrase.len() + 30).min(doc.len())];
        let quoted = numbers(window);
        assert!(
            quoted.contains(&allocated),
            "`NEXT_FREE` is {allocated} and `{phrase}` quotes {quoted:?}. \
             Window: {window}"
        );
        checked += 1;
    }
    assert_eq!(
        checked,
        phrases.len(),
        "both phrasings must be checked, or a half-corrected document passes"
    );

    let headroom = near(&doc, "free.", 8);
    let at = offset(&doc, "free.");
    let window = &doc[at.saturating_sub(12)..at + 5];
    let quoted = numbers(window);
    assert!(
        quoted.contains(&free),
        "{width} bits minus {allocated} allocated is {free} free, and the document \
         reads {quoted:?}. Window: {window}{headroom}"
    );
}

/// The document lists as many shareable crates as its own heading claims, and every
/// one of them really is in the workspace.
///
/// The heading said "four crates" while the table listed six, because `core` and
/// `costs` were argued back in (D-0095) and the heading was not touched.
#[test]
fn the_shareable_table_is_as_long_as_the_heading_says() {
    // A table rather than a match arm per count: an arm for a heading that does not
    // exist is a branch no run can take, and this file's own §4 forbids code that
    // cannot execute while the document is right. Order is the order a heading naming
    // two counts is read in, and `word_number` deliberately does not decide it -- a
    // heading is prose about a table, not a claim about a count.
    const COUNTS: [(&str, usize); 4] = [("six", 6), ("five", 5), ("four", 4), ("seven", 7)];
    let heading = SHARED_CORE_DOC
        .lines()
        .find(|l| l.contains("crates that are shareable"))
        .expect("the shareable-crates heading exists");
    let named = COUNTS
        .iter()
        .find(|(word, _)| heading.contains(*word))
        .map(|(_, count)| *count);
    assert!(
        named.is_some(),
        "the heading names no count between four and seven, so there is nothing to \
         check the table against: {heading}"
    );
    let claimed = named.expect("the assertion above rejected `None`");

    // The table rows are the ones opening with a backticked crate name.
    let listed: Vec<&str> = SHARED_CORE_DOC
        .lines()
        .skip_while(|l| !l.contains("crates that are shareable"))
        .take_while(|l| !l.contains("Not shareable"))
        .filter(|l| l.starts_with('`') && l.contains('|'))
        .collect();

    let rows = listed.len();
    assert_eq!(
        rows, claimed,
        "the heading claims {claimed} shareable crates and the table lists {rows}: \
         {listed:?}"
    );

    for row in listed {
        let name = row
            .split('`')
            .nth(1)
            .expect("the row opens with a backtick");
        let manifest = std::path::Path::new("..").join(name).join("Cargo.toml");
        assert!(
            manifest.exists(),
            "the table lists `{name}` as shareable and `crates/{name}/Cargo.toml` does \
             not exist"
        );
    }
}

// ---------------------------------------------------------------------------
// THE HELPERS THEMSELVES.
//
// Everything above is only as honest as the five functions it reads the prose
// through. A window off by a word, a count taken from the wrong place, or a
// missing phrase that unwraps a `None` instead of naming itself all turn a real
// drift into either a green build or a failure that blames this file.
// ---------------------------------------------------------------------------

/// A digit run too long for a `usize` is dropped, not turned into a failure.
///
/// `numbers` is pointed at arbitrary prose, and this repository's documents quote
/// 64-hex-digit identities and long decimal counts elsewhere. Were the `parse`
/// unwrapped instead of filtered, one long run of digits added to the document would
/// abort every check in this file, and the reader would be sent here rather than to
/// the line that was edited.
#[test]
fn a_digit_run_too_long_for_a_usize_is_dropped_instead_of_ending_the_read() {
    // 35 nines: past `usize::MAX`, which is twenty digits wide.
    let claim = format!("7 {} 8", "9".repeat(35));
    let got = numbers(&claim);
    assert_eq!(
        got,
        vec![7, 8],
        "a digit run wider than a `usize` must be skipped and its neighbours kept; \
         `numbers` returned {got:?} for {claim}"
    );
}

/// Digits separated by anything at all are separate numbers, and the empty pieces the
/// split leaves behind are not read as zeroes.
///
/// Without the `is_empty` filter every separator contributes an empty token. Those
/// parse as nothing today, so the counts stay right -- but a zero in the list would
/// match a claim of zero anywhere in the document, and nothing else in this file would
/// notice.
#[test]
fn separators_split_the_numbers_and_leave_no_zero_behind() {
    let got = numbers(", 234 positions -- 108 free.");
    assert_eq!(
        got,
        vec![234, 108],
        "punctuation and dashes separate two numbers and contribute none of their \
         own; `numbers` returned {got:?}"
    );
    let none = numbers("no digits at all");
    assert!(
        none.is_empty(),
        "prose with no digit in it has no numbers in it; `numbers` returned {none:?}"
    );
}

/// Flattening collapses every wrap and loses no word.
///
/// This is what lets a window of 24 characters carry a whole claim. A newline left in
/// would put half a claim in the window, and a dropped word would move every number
/// relative to the phrase the window is anchored on.
#[test]
fn flattening_collapses_every_wrap_without_losing_a_word() {
    let doc = flat();
    assert!(
        !doc.contains('\n'),
        "a newline survived the collapse, so a claim can still be split across two \
         windows"
    );
    assert!(
        !doc.contains("  "),
        "two spaces in a row means a run of whitespace survived the collapse, and \
         every window past it is short by the difference"
    );
    let words = doc.split(' ').count();
    let source_words = SHARED_CORE_DOC.split_whitespace().count();
    assert_eq!(
        words, source_words,
        "flattening dropped or invented a word: {source_words} in the document, \
         {words} after"
    );
}

/// `span` counts from the start of the needle, not from the end of it.
///
/// Read the other way, `near(doc, "position sources", 40)` would be a 56-character
/// window, and a window that reaches into the next claim picks up a number belonging
/// to a different sentence -- which passes a check that should have failed.
#[test]
fn the_span_is_the_width_of_the_window_and_not_the_run_after_the_needle() {
    let got = near("ab cdefghij", "cd", 6);
    assert_eq!(
        got, "cdefgh",
        "a span of 6 from a two-character needle is six characters in total, four of \
         them past the needle; `near` returned {got}"
    );
}

/// A span wider than what is left of the document is clamped to its end.
///
/// The last claim in the document has nothing after it. A window that ran past the end
/// would panic inside a slice, which reads as a bug in this file rather than as the
/// document being short.
#[test]
fn a_span_wider_than_the_document_stops_at_its_end() {
    let doc = flat();
    let at = offset(&doc, "Not shareable");
    let tail = &doc[at..];
    let got = near(&doc, "Not shareable", doc.len());
    assert_eq!(
        got, tail,
        "a span wider than the document must run to its end and stop there"
    );
}

/// A phrase the document no longer contains names itself in the failure.
///
/// This is the failure every check above reaches when the document is reworded, and it
/// is the whole reason the lookup is one function. `called Option::unwrap() on a None
/// value` names nothing, and a reader who cannot see which phrase moved edits the test
/// instead of the document.
#[test]
#[should_panic(expected = "does not contain `a phrase this document never had`")]
fn a_phrase_the_document_lost_is_named_in_the_failure() {
    offset(&flat(), "a phrase this document never had");
}

/// A spelled count is read from where it sits in the claim, not from the order of the
/// word table.
///
/// Section 1's real heading names two number words, and `one` is the first entry in
/// the table: a helper that asked `contains` in table order answered 1 for a heading
/// whose count is six. Nothing exercised it, because the only caller passes a window
/// with one word in it -- so the next caller would have been the one to find out.
#[test]
fn a_spelled_count_comes_from_the_claim_and_not_from_the_word_table_order() {
    let heading = "## 1. The six crates that are shareable, and the one change that made them so";
    let got = word_number(heading);
    assert_eq!(
        got,
        Some(6),
        "the heading's count is `six`, and `one` inside `the one change` sits later in \
         the line and must not outrank it; `word_number` read {got:?}"
    );
}

/// Prose that spells no count is `None` rather than some default.
///
/// A default would let the source-count check compare the document against a number
/// the document does not contain, and pass whenever the two happened to agree.
#[test]
fn prose_that_spells_no_count_is_none_rather_than_a_default() {
    let got = word_number("crates that are shareable");
    assert_eq!(
        got, None,
        "there is no number word in that phrase, so there is no count to report; \
         `word_number` read {got:?}"
    );
}

/// A count written in digits is read when the prose does not spell it.
///
/// The digit fallback is the half of `stated_count` the document does not exercise:
/// the `indicators` row spells "Nine" today. Nothing but this test keeps the other
/// reading working, and a reworded row is exactly when it is needed.
#[test]
fn a_count_in_digits_is_read_when_the_prose_does_not_spell_it() {
    let digits = stated_count("out. 9 position sources");
    assert_eq!(
        digits,
        Some(9),
        "a row reading `9 position sources` states nine; `stated_count` read {digits:?}"
    );
    let spelled = stated_count("out. Nine position sources");
    assert_eq!(
        spelled,
        Some(9),
        "a row reading `Nine position sources` states the same nine; `stated_count` \
         read {spelled:?}"
    );
    let neither = stated_count("out. position sources");
    assert_eq!(
        neither, None,
        "a claim with no count in it states nothing, and must not be read as zero; \
         `stated_count` read {neither:?}"
    );
}

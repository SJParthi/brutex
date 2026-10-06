//! Three re-verifications kept as inherent, held to the code and to
//! `docs/06-limits.md` (W2-cli6-0 D-1846, W2-cli2-5 D-1847, W2-cli14-1
//! D-1848).
//!
//! Each is a before/after or per-child proof that a read's inputs still say
//! what they said when they were acknowledged. The limits argue why no
//! cheaper check gives the same guarantee; these tests fail if the code stops
//! paying the cost the limits state, or the limits stop stating it.

#![allow(
    clippy::expect_used,
    clippy::panic,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail. An integration test file is \
              its own crate root, so the attribute cannot be inherited."
)]

const CHECKPOINT: &str = include_str!("../src/index_stop_search_checkpoint.rs");
const PROJECTION: &str = include_str!("../src/boolean_search_projection.rs");
const SELECTION_V5: &str = include_str!("../src/selection_v5.rs");
const LIMITS: &str = include_str!("../../../docs/06-limits.md");

/// `text` with every run of whitespace folded to one space.
fn words(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The limits section headed `heading`, up to the next `## `, folded.
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

/// The text of `source` from `head` to the first line that closes an item at
/// `indent` spaces.
fn body<'a>(source: &'a str, head: &str, indent: usize) -> &'a str {
    let from = source
        .find(head)
        .unwrap_or_else(|| panic!("no `{head}` in the source"));
    let close = format!("\n{}}}\n", " ".repeat(indent));
    source
        .get(from..)
        .and_then(|rest| rest.find(&close).and_then(|to| rest.get(..to)))
        .expect("its body")
}

fn says(section: &str, sentence: &str) {
    assert!(
        section.contains(&words(sentence)),
        "the limit must say: {sentence}\n{section}"
    );
}

/// W2-cli6-0, D-1846: every launch verifies every acknowledged child of every
/// completed frame, and the limit says why that is the proof.
#[test]
fn a_single_stop_launch_verifies_every_acknowledged_child() {
    let recover = body(CHECKPOINT, "pub(super) fn recover(", 0);
    let replay = recover
        .split_once("for mut frame in history {")
        .expect("the replay walks the whole history")
        .1;
    assert!(replay.contains("if !frame.pending {"));
    assert!(replay.contains("for (rung, link) in frame.links.iter().enumerate() {"));
    assert_eq!(
        replay
            .matches("qualification::verify_search_slot_bounded(")
            .count(),
        1
    );
    let section =
        limit("## A single-stop search re-verifies its whole acknowledged history on every launch");
    says(&section, "Argued inherent by D-1846.");
    says(
        &section,
        "an acknowledged child is a separate durable journal that the parent pins by seal",
    );
}

/// W2-cli2-5, D-1847: a page checks every retained journal record before and
/// after it reads.
#[test]
fn a_boolean_search_page_rechecks_its_journals_on_both_sides() {
    let rows = body(PROJECTION, "    pub fn rows(", 4);
    assert_eq!(
        rows.matches("self.require_current()?;").count(),
        2,
        "{rows}"
    );
    let (before, after) = rows
        .split_once(".rows(pin, start, limit)?")
        .expect("one page read");
    assert!(before.contains("self.require_current()?;"));
    assert!(after.contains("self.require_current()?;"));
    let section =
        limit("## A Boolean search detail page rechecks every retained journal record twice");
    says(&section, "Argued inherent by D-1847.");
    says(
        &section,
        "the closing recheck is what proves the page was read from records that held throughout",
    );
}

/// W2-cli14-1, D-1848: a Selection V5 Top-25 read derives its selection from
/// the committed Execution before and after it reads the winners.
#[test]
fn a_selection_v5_read_derives_its_selection_on_both_sides() {
    let top = body(SELECTION_V5, "    pub(crate) fn top_twenty_five(", 4);
    assert_eq!(
        top.matches(
            "PreparedSelectionV5::from_committed_execution(&mut self.source, self.policy)?"
        )
        .count(),
        2,
        "{top}"
    );
    let section = limit("## Selection V5 and V6 reads and commits replay their sources");
    says(&section, "argued inherent beyond the snapshot.");
    says(
        &section,
        "the pair IS the read's proof: Selection holds no copy of its upstream",
    );
}

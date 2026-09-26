//! D-0694's record of the stored doors, checked against the `cli` source it
//! describes.
//!
//! D-0694 put [`runner::audit::CORPORATE_ACTIONS_UNCHECKED`] on every stored
//! report that ranks or audits a stock, and made one more door withhold
//! sessions with an intraday minute hole. Its record named which doors do
//! what, and a review found two places where the record said more, or less,
//! than the source does:
//!
//! - AF-16 said every stored report over one stock opens with the banner and
//!   the note, and D-0694's item 3 opens by saying so of every stored report
//!   over one instrument. The explicit expression report, `expression-stored`,
//!   loads a stock's month and opens with the bare banner.
//! - The list of stored doors that still keep holed sessions, in D-0694 and in
//!   `docs/06-limits.md`, left out the strict audited range,
//!   `audit-audited-range`, which never calls `crate::minute_gaps`.
//!
//! Each of those two tests reads the record and the source together, so the
//! record fails here when the source moves under it. A third reads the record
//! alone: two of D-0694's corrections said nothing above them was edited,
//! and a sentence above them was edited in place after they were written.

#![allow(clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

use std::path::{Path, PathBuf};

/// The repository root: `crates/runner` is two levels below it.
fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crates/runner is two levels below the repository root")
        .to_owned()
}

fn read(relative: &str) -> String {
    std::fs::read_to_string(repo().join(relative))
        .unwrap_or_else(|why| panic!("{relative} is readable: {why}"))
}

/// The text with every run of whitespace, line breaks included, made one space.
fn collapse(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// One invariant row, whole.
fn row(id: &str) -> String {
    let invariants = read("docs/04-invariants.md");
    invariants
        .lines()
        .find(|line| line.starts_with(&format!("| {id} |")))
        .unwrap_or_else(|| panic!("{id} has a row"))
        .to_owned()
}

/// D-0694, from its heading up to the next entry's.
fn d_0694() -> String {
    let decisions = read("docs/05-decisions.md");
    let start = decisions
        .find("\n### D-0694 ")
        .expect("D-0694 has a heading");
    let body = &decisions[start + 1..];
    let end = body
        .get(1..)
        .and_then(|after| after.find("\n### D-"))
        .map_or(body.len(), |at| at + 1);
    collapse(&body[..end])
}

/// The D-0694 section of `docs/06-limits.md`, up to the next section.
fn limits_d_0694() -> String {
    let limits = read("docs/06-limits.md");
    let start = limits
        .find("\n## Corporate actions are stated, not detected")
        .expect("the limits register has a D-0694 section");
    let body = &limits[start + 1..];
    let end = body
        .get(1..)
        .and_then(|after| after.find("\n## "))
        .map_or(body.len(), |at| at + 1);
    collapse(&body[..end])
}

/// **AF-16 SCOPES THE STOCK BANNER TO THE STORED REPORTS THAT RANK OR AUDIT,
/// AND NAMES THE EXPLICIT EXPRESSION REPORT AS ONE THAT OPENS BARE.**
///
/// `cli expression-stored` loads one instrument-month through `stored::load`,
/// which admits the F&O shares, and builds its report from
/// `STORED_PROVENANCE` alone. It ranks nothing, which is the operator's scope
/// for the note ("every report that ranks or audits a cash equity"), and
/// D-0694 lists Expression V1 as unchanged. AF-16 said every stored report
/// over one stock carries the note, and D-0694's item 3 still opens by saying
/// it of every stored report over one instrument. This checks the row's
/// scope, that D-0694's correction scopes item 3's opening sentence to the
/// reports its list names, and that the report AF-16 names as the exception
/// still opens bare, so the row fails here if that report gains the note or
/// the row loses its exception.
#[test]
fn af_16_names_the_explicit_expression_report_as_one_that_opens_with_the_bare_banner() {
    let af_16 = collapse(&row("AF-16"));
    assert!(
        af_16.contains(
            "Every stored report that ranks or audits one stock opens with the provenance \
             banner, then gross of every charge, then that sentence."
        ),
        "AF-16 must scope the banner sentence to the reports that rank or audit: {af_16}"
    );
    assert!(
        !af_16.contains("Every stored report over one stock opens"),
        "AF-16 again says every stored report over one stock carries the note: {af_16}"
    );
    assert!(
        af_16.contains(
            "The explicit expression report, `expression-stored` (Expression V1), ranks \
             nothing and opens with the bare `STORED_PROVENANCE` banner, over a stock as \
             over an index."
        ),
        "AF-16 must name the explicit expression report as the exception: {af_16}"
    );
    let decision = d_0694();
    assert!(
        decision.contains(
            "3. Every stored report over one instrument now opens with `STORED_PROVENANCE` \
             and then, for a stock only, the note."
        ),
        "the premise: item 3 opens with the universal sentence its correction scopes"
    );
    assert!(
        decision.contains(
            "Item 3's opening sentence says the same of every stored report over one \
             instrument, and it covers the reports its list names, which Expression V1 \
             is not among."
        ),
        "D-0694's correction must scope item 3's opening sentence, which says every \
         stored report over one instrument opens with the note"
    );

    let source = read("crates/cli/src/expression.rs");
    assert!(
        source.contains(
            "let loaded = crate::stored::load(&root, vendor, underlying, rung, year, month)?;"
        ),
        "the explicit expression report no longer loads through `stored::load`, \
         which admits a share, so AF-16's exception may be stale"
    );
    assert!(
        source.contains("let mut report = String::from(crate::STORED_PROVENANCE);"),
        "the explicit expression report no longer opens with the bare banner, so \
         AF-16's exception is stale"
    );
    for note in ["stored_provenance(", "equity_note"] {
        assert!(
            !source.contains(note),
            "the explicit expression report now names {note:?}; AF-16 says it \
             opens bare and must be corrected with it"
        );
    }
}

/// Every non-test source under `dir` that names `minute_gaps::`, by its path
/// below `root`. A test file is one named `tests.rs` or ending `_tests.rs`.
fn naming_minute_gaps(root: &Path, dir: &Path, into: &mut Vec<String>) {
    for entry in std::fs::read_dir(dir).expect("a readable source directory") {
        let path = entry.expect("a directory entry").path();
        let kind = std::fs::symlink_metadata(&path).expect("a file type");
        if kind.is_dir() {
            naming_minute_gaps(root, &path, into);
            continue;
        }
        let relative = path
            .strip_prefix(root)
            .expect("below the root")
            .to_str()
            .expect("a UTF-8 path")
            .to_owned();
        let is_source = path.extension().is_some_and(|ext| ext == "rs");
        let is_test = relative.ends_with("_tests.rs")
            || path.file_name().is_some_and(|name| name == "tests.rs");
        if is_source
            && !is_test
            && std::fs::read_to_string(&path)
                .expect("a readable source")
                .contains("minute_gaps::")
        {
            into.push(relative);
        }
    }
}

/// **D-0694 AND THE LIMITS REGISTER NAME THE STRICT AUDITED RANGE AMONG THE
/// STORED DOORS THAT STILL KEEP HOLED SESSIONS.**
///
/// D-0694 says `crate::minute_gaps` is called only from `lib.rs` and
/// `pool.rs`, and lists the stored doors that never call it. The strict
/// audited range, `audit-audited-range`, loads its span through
/// `audited_range::RangeInputs::load` and builds its column from those bars
/// with nothing withheld, and neither list named it. This reads which `cli`
/// sources name `minute_gaps::` at all, outside the test files, and requires
/// the two lists to name the door whose sources never do.
#[test]
fn d_0694_names_the_strict_audited_range_among_the_doors_that_keep_holed_sessions() {
    let src = repo().join("crates/cli/src");
    let mut naming = Vec::new();
    naming_minute_gaps(&src, &src, &mut naming);
    naming.sort();
    assert_eq!(
        naming,
        ["lib.rs", "pool.rs"],
        "D-0694 says `crate::minute_gaps` is called only from lib.rs and pool.rs"
    );
    for file in ["audited_range.rs", "audited_range_command.rs"] {
        let text = read(&format!("crates/cli/src/{file}"));
        assert!(
            !text.contains("minute_gaps"),
            "{file} names minute_gaps now: the strict audited range may withhold, and \
             D-0694's list of doors that keep holed sessions must move it"
        );
    }

    let door = "the strict audited range (`audit-audited-range`, `audited_range_command::run`)";
    let decision = d_0694();
    assert!(
        decision.contains(&format!(
            "{door} never calls `crate::minute_gaps` either, and keeps holed sessions"
        )),
        "D-0694 must add the strict audited range to its doors that keep holed sessions"
    );
    let limits = limits_d_0694();
    assert!(
        limits.contains(&format!(
            "{door} keeps holed sessions too: it never calls `crate::minute_gaps`"
        )),
        "the limits register must add the strict audited range to the same list"
    );
}

/// **NO CORRECTION OF D-0694 WRITTEN SINCE 2026-09-25 SAYS NOTHING ABOVE IT IS
/// EDITED.**
///
/// The corrections of 2026-09-25 and of 2026-09-26 each said that each item
/// corrects a statement above by adding text and that nothing above is edited.
/// After both were written, the sentence on `eecca4da` in item 10 of the
/// 2026-09-24 correction was edited in place, which the entry allows, being
/// new in its change, and which made both statements false. This requires
/// neither correction, nor any later one, to say it, and the record to name
/// the sentence that was edited.
#[test]
fn no_correction_of_d_0694_since_2026_09_25_says_nothing_above_it_is_edited() {
    let decision = d_0694();
    let (_, since) = decision
        .split_once("**Correction, 2026-09-25.**")
        .expect("D-0694 has its 2026-09-25 correction");
    assert!(
        !since.contains("nothing above is edited"),
        "a correction since 2026-09-25 says nothing above it is edited, and a \
         sentence above them was edited in place"
    );
    assert!(
        since.contains(
            "the sentence on eecca4da in item 10 of the 2026-09-24 correction was edited in \
             place"
        ),
        "D-0694 must name the sentence edited in place above its later corrections"
    );
}

//! The rows BASE-STACK-1028 wrote at the data parts' cut point stay as they
//! were written, and each correction of them is a row of its own (D-7412).
//!
//! The five ledgers are append-only (the landing procedure's rule D1 (B)). Every
//! data part and FNO-SEAM were cut from [`CUT`] and append right after this
//! part's rows, so a row rewritten there is one the family's landing tool
//! refuses as "not an append". A repair of this part once rewrote them to
//! correct their figures, and every gate it ran was green: nothing in the
//! workspace read them against the commit the other parts hold. This does.
//!
//! The compare is by lines, in order. A line another part puts between two of
//! these rows is no loss, and a landing may do that; a rewritten, removed or
//! moved row is.
//!
//! It asks `git`, so a tree that cannot answer, because it holds no history or
//! a clone that does not hold [`CUT`], says so and compares nothing.
//!
//! One cell is the agreed exception, [`AGREED`] (D-7413): gate 10 reads only a
//! row's own proof cell, so that one row could not be cured by a new row.

#![allow(clippy::expect_used, clippy::indexing_slicing)]

use std::path::{Path, PathBuf};
use std::process::Command;

/// The commit the data parts and FNO-SEAM were cut from.
const CUT: &str = "bcac15e2c4d9daf8bf4a100ca3f18729c61810e4";

/// Each ledger, the start of the line this part's group opens with at [`CUT`],
/// and how many lines that group held there: to the end of the file, each
/// being the last group of its ledger at that commit.
const GROUPS: [(&str, &str, usize); 3] = [
    ("docs/04-invariants.md", "| STACK1028-01 |", 7),
    ("docs/05-decisions.md", "### D-7410 ", 83),
    (
        "docs/06-limits.md",
        "## The native search's stack, measured",
        15,
    ),
];

/// The one agreed change to a row of [`GROUPS`] (D-7413): the start of that row,
/// its proof cell as [`CUT`] held it, and that cell as it reads now.
///
/// Only the two cell texts are held, so the row must still read as [`CUT`] held
/// it everywhere else: another cell changed, this cell written any other way, or
/// the row put back as it was are each still a lost row.
const AGREED: (&str, &str, &str) = (
    "| STACK1028-03 |",
    "| the bounded-stack tests of `boolean_grammar_batch_tests.rs` and `index_stop_search_checkpoint_tests.rs` |",
    "| `crates/cli/src/boolean_grammar_batch_tests.rs`: `empty_single_many_and_terminal_batches_replay_exactly_on_a_bounded_stack`, `many_programs_paused_by_nodes_then_resumed_twice_match_one_whole_batch_on_a_bounded_stack`, `concurrent_bounded_stack_replays_of_one_batch_agree_byte_for_byte`; `crates/cli/src/index_stop_search_checkpoint_tests.rs`: `ancestries_of_one_and_two_frames_recover_exactly_on_a_bounded_stack`, `the_deepest_ancestry_is_walked_in_a_loop_and_refused_at_its_oldest_unverified_child`, `a_deep_ancestry_of_full_batches_at_exact_record_and_byte_admission_fits_a_bounded_stack`, `a_pending_reservation_cut_and_reopened_twice_recovers_the_same_bytes_on_a_bounded_stack`, `a_second_owner_is_refused_while_a_deep_recovery_holds_the_journal_and_then_agrees` |",
);

/// The repository root: `crates/cli` is two levels below it.
fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crates/cli is two levels below the repository root")
        .to_owned()
}

/// `file` as commit `cut` holds it, if `git` can say here.
fn copy_at(root: &Path, cut: &str, file: &str) -> Option<String> {
    Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["show", &format!("{cut}:{file}")])
        .output()
        .ok()
        .filter(|out| out.status.success())
        .map(|out| String::from_utf8_lossy(&out.stdout).into_owned())
}

/// The lines of `old` from the first that starts with `first` to its end.
fn group<'a>(old: &'a str, first: &str) -> Vec<&'a str> {
    old.lines()
        .skip_while(|line| !line.starts_with(first))
        .collect()
}

/// The lines of `rows` that `now` no longer holds in their order. Lines between
/// them are no loss. Each line of `now` answers for one row, so a row held twice
/// must be there twice.
fn lost<'a>(rows: &[&'a str], now: &str) -> Vec<&'a str> {
    let held: Vec<&str> = now.lines().collect();
    let mut from = 0;
    let mut gone = Vec::new();
    for row in rows {
        match held[from..].iter().position(|line| line == row) {
            Some(at) => from += at + 1,
            None => gone.push(*row),
        }
    }
    gone
}

/// The rows of one of [`GROUPS`] in the ledger's old copy, refused by its file
/// when they are not as many as the group held: a marker that opens nothing
/// would otherwise compare no rows and pass.
fn rows_of<'a>(old: &'a str, (file, first, lines): (&str, &str, usize)) -> Vec<&'a str> {
    let rows = group(old, first);
    assert_eq!(rows.len(), lines, "{file}: the rows this part wrote");
    rows
}

/// `row` as it must read now: as it was, unless it is the row `agreed` names,
/// and then with that one cell changed. A row that no longer holds the old cell
/// is left as it is, so it answers for nothing.
fn as_agreed(row: &str, (first, was, now): (&str, &str, &str)) -> String {
    if row.starts_with(first) {
        row.replacen(was, now, 1)
    } else {
        row.to_owned()
    }
}

/// What `root`'s copy of the group's ledger has lost of what `cut` held, each
/// line as it must read now. A commit `git` cannot show here compares nothing,
/// and says so.
fn lost_since(root: &Path, cut: &str, ledger: (&str, &str, usize)) -> Vec<String> {
    let file = ledger.0;
    let Some(old) = copy_at(root, cut, file) else {
        println!("SKIPPING: {file} at {cut} cannot be read here, so its rows are not compared.");
        return Vec::new();
    };
    let rows: Vec<String> = rows_of(&old, ledger)
        .into_iter()
        .map(|row| as_agreed(row, AGREED))
        .collect();
    let rows: Vec<&str> = rows.iter().map(String::as_str).collect();
    let now = std::fs::read_to_string(root.join(file)).expect("the ledger is readable");
    lost(&rows, &now).into_iter().map(str::to_owned).collect()
}

/// **THIS PART'S ROWS AT THE CUT POINT ARE KEPT BYTE FOR BYTE, AND IN ORDER**,
/// but for the one cell [`AGREED`] names.
///
/// Fails on the repair that rewrote the rows of L-STACK1028-01 to -05, their
/// preamble, D-7410, STACK1028-04 and STACK1028-05 to correct their figures.
#[test]
fn the_rows_at_the_cut_point_are_kept_byte_for_byte() {
    for ledger in GROUPS {
        let gone = lost_since(&repo(), CUT, ledger);
        assert!(gone.is_empty(), "{}: {gone:#?}", ledger.0);
    }
}

/// **EACH FIGURE THAT WENT STALE HAS A NEWER ROW SAYING SO.** A row kept as it
/// was still says what it said, so the correction must stand beside it and name
/// it, or a reader takes the old figure for the current one.
#[test]
fn each_stale_figure_has_a_newer_row_that_names_it() {
    let limits = std::fs::read_to_string(repo().join("docs/06-limits.md")).expect("the limits");
    let invariants =
        std::fs::read_to_string(repo().join("docs/04-invariants.md")).expect("the invariants");
    for (text, row, names, figure) in [
        (&limits, "| L-STACK1028-06 |", "L-STACK1028-04", "690,096 B"),
        (&limits, "| L-STACK1028-06 |", "611,584 B", "1536 KiB"),
        (&limits, "| L-STACK1028-07 |", "L-STACK1028-01", "736 KiB"),
        (&limits, "| L-STACK1028-08 |", "L-STACK1028-02", "224 KiB"),
        (&limits, "| L-STACK1028-09 |", "L-STACK1028-03", "D-7412"),
        (&limits, "| L-STACK1028-09 |", "L-STACK1028-04", "this tip"),
        (&limits, "| L-STACK1028-10 |", "L-STACK1028-05", "D-7412"),
        (&limits, "| L-STACK1028-11 |", "L-STACK1028-01", "12,286 B"),
        (&invariants, "| STACK1028-08 |", "STACK1028-04", "736 KiB"),
        (&invariants, "| STACK1028-09 |", "STACK1028-05", "690,096 B"),
    ] {
        let line = text
            .lines()
            .find(|line| line.starts_with(row))
            .expect("the correcting row exists");
        assert!(line.contains(names), "{row} names {names}");
        assert!(line.contains(figure), "{row} states {figure}");
    }
}

/// **A COMMIT THAT CANNOT BE SHOWN HERE COMPARES NOTHING, AND SAYS SO.** It is
/// what a clone without [`CUT`] meets, and a folder holding no history.
#[test]
fn a_commit_that_cannot_be_shown_compares_nothing() {
    for ledger in GROUPS {
        assert!(lost_since(&repo(), "0000000000000000000000000000000000000000", ledger).is_empty());
        assert!(lost_since(Path::new("/nonexistent-root-of-no-tree"), CUT, ledger).is_empty());
    }
}

/// **A GROUP OF THE WRONG LENGTH IS REFUSED BY ITS FILE.** One row more or fewer
/// than the group held, and a marker that opens nothing.
#[test]
fn a_group_of_another_length_is_refused() {
    let old = "before\n| A |\n| B |\n";
    assert_eq!(rows_of(old, ("a ledger", "| A |", 2)), ["| A |", "| B |"]);
    for (first, lines) in [("| A |", 3), ("| A |", 1), ("| NO SUCH ROW |", 2)] {
        let refused = std::panic::catch_unwind(|| rows_of(old, ("a ledger", first, lines)))
            .expect_err("the wrong length must fail, not pass");
        let said = refused.downcast_ref::<String>().map_or("", String::as_str);
        assert!(
            said.contains("a ledger: the rows this part wrote"),
            "{said}"
        );
    }
}

/// `group` starts at a line's start, runs to the end, and finds nothing past
/// the end. A marker inside a line opens nothing.
#[test]
fn a_group_runs_from_the_line_its_marker_starts_to_the_end() {
    let text = "before\n| A | one |\nmiddle | A | inside\n| B | two |\n";
    assert_eq!(
        group(text, "| A |"),
        ["| A | one |", "middle | A | inside", "| B | two |"]
    );
    assert_eq!(group(text, "| B |"), ["| B | two |"]);
    assert!(group(text, "| C |").is_empty());
    assert!(group(text, "inside").is_empty());
    assert!(group("", "| A |").is_empty());
}

/// Whatever is kept, appended after or put between is no loss.
#[test]
fn a_row_kept_with_lines_around_it_is_no_loss() {
    let rows = ["| A |", "", "| B |", "| C |"];
    for now in [
        "| A |\n\n| B |\n| C |\n",
        "| A |\n\n| B |\n| C |\n| D, appended |\n",
        "above\n| A |\n\n| B |\n| C |",
        "| A |\n| between |\n\n| between |\n| B |\n| between |\n| C |\n",
    ] {
        assert!(lost(&rows, now).is_empty(), "{now}");
    }
    assert!(lost(&[], "| A |\n").is_empty(), "no rows, nothing to lose");
}

/// Each way of changing a row is a loss, named as the row was.
#[test]
fn a_row_rewritten_removed_or_moved_is_lost() {
    let rows = ["| A | 611,584 B |", "| B |", "| C |"];
    for (now, gone) in [
        ("| A | 690,096 B |\n| B |\n| C |\n", "| A | 611,584 B |"),
        ("| A | 611,584 B |\n| C |\n", "| B |"),
        ("| A | 611,584 B |\n| B | \n| C |\n", "| B |"),
        ("| A | 611,584 B |\n| C |\n| B |\n", "| C |"),
    ] {
        assert_eq!(lost(&rows, now), [gone], "{now}");
    }
    assert_eq!(lost(&rows, ""), rows, "an empty ledger has lost them all");
}

/// A rewritten row, a removed one and another part's row all at once: exactly
/// the two are named, and the rows after them are still found.
#[test]
fn a_rewrite_a_removal_and_an_insertion_at_once_name_exactly_two_rows() {
    let rows = ["| A |", "| B |", "| C |", "| D |"];
    let now = "| A | rewritten |\n| between |\n| C |\n| D |\n| E, appended |\n";
    assert_eq!(lost(&rows, now), ["| A |", "| B |"]);
}

/// A row held twice must be there twice, and a second copy in the wrong place
/// answers for neither.
#[test]
fn a_row_held_twice_must_be_there_twice() {
    let rows = ["", "| A |", ""];
    assert!(lost(&rows, "\n| A |\n\n").is_empty());
    assert_eq!(lost(&rows, "\n| A |\n"), [""]);
    assert_eq!(lost(&rows, "| A |\n\n"), ["| A |", ""]);
}

/// Out of the box: a line ending `\r\n` is the line without its `\r`, as
/// `str::lines` reads it, so a change of line endings alone loses nothing, and a
/// row that only differs by a character outside ASCII does.
#[test]
fn line_endings_lose_nothing_and_one_character_outside_ascii_does() {
    let rows = ["| A | \u{2713} |", "| B |"];
    assert!(lost(&rows, "| A | \u{2713} |\r\n| B |\r\n").is_empty());
    assert_eq!(
        lost(&rows, "| A | \u{2717} |\n| B |\n"),
        ["| A | \u{2713} |"]
    );
}

/// A row shaped as the agreed one, with `claim` and `proof` in its cells.
fn agreed_shape(claim: &str, proof: &str) -> String {
    format!("{} {claim} {proof} \u{2713} |", AGREED.0)
}

/// **THE EXCEPTION CHANGES ONE CELL OF ONE ROW AND NOTHING ELSE.** The claim and
/// the status survive it, and no other row is touched, even one holding that
/// very cell.
#[test]
fn the_agreed_change_touches_one_cell_of_one_row() {
    let (_, was, now) = AGREED;
    assert_eq!(
        as_agreed(&agreed_shape("| a claim", was), AGREED),
        agreed_shape("| a claim", now)
    );
    let other = format!("| STACK1028-02 | a claim {was} \u{2713} |");
    assert_eq!(as_agreed(&other, AGREED), other);
    assert_eq!(as_agreed("", AGREED), "");
    assert_eq!(as_agreed(AGREED.0, AGREED), AGREED.0);
}

/// **ONLY THE AGREED TEXT PASSES.** The row as agreed is no loss; the same row
/// with another cell changed, with its proof written another way, or put back as
/// it was is lost, and is named.
#[test]
fn only_the_agreed_text_of_that_row_passes() {
    let (_, was, now) = AGREED;
    let old = agreed_shape("| a claim", was);
    let must_read = as_agreed(&old, AGREED);
    let rows = [must_read.as_str()];
    assert!(lost(&rows, &agreed_shape("| a claim", now)).is_empty());
    for changed in [
        agreed_shape("| another claim", now),
        agreed_shape("| a claim", "| another proof |"),
        agreed_shape("| a claim", &now.replace("`, `", "`; `")),
        agreed_shape("| a claim", &now[..now.len() - 3]),
        agreed_shape("| a claim", was),
        must_read.replace('\u{2713}', "\u{2717}"),
    ] {
        assert_eq!(lost(&rows, &changed), [must_read.as_str()], "{changed}");
    }
}

/// A second row changed the same way is no exception: naming one row allows one.
#[test]
fn the_same_change_to_any_other_row_is_still_lost() {
    let (_, was, now) = AGREED;
    let other = format!("| STACK1028-04 | a claim {was} \u{2713} |");
    let rows = [as_agreed(&other, AGREED)];
    let rows: Vec<&str> = rows.iter().map(String::as_str).collect();
    let rewritten = other.replace(was, now);
    assert_eq!(lost(&rows, &rewritten), [other.as_str()]);
}

/// Every extreme at once. The compare is in order, so the agreed row moved below
/// the rows after it loses those rows, while a rewrite beside it and another
/// part's row between them change nothing about what is named. Put back as it
/// was, in its place, it is lost itself and the row after the rewrite is found.
#[test]
fn a_moved_or_restored_agreed_row_and_a_rewrite_beside_it_name_exactly_those_rows() {
    let (_, was, _) = AGREED;
    let agreed = as_agreed(&agreed_shape("| a claim", was), AGREED);
    let rows = ["| B | one |", agreed.as_str(), "| C | two |", "| D |"];
    let kept = format!(
        "| between |\n| B | one |\n{agreed}\n| between |\n| C | changed |\n| between |\n| D |\n"
    );
    assert_eq!(lost(&rows, &kept), ["| C | two |"]);
    let moved = format!("| B | one |\n| between |\n| C | changed |\n| D |\n{agreed}\n");
    assert_eq!(lost(&rows, &moved), ["| C | two |", "| D |"]);
    let restored = format!(
        "| B | one |\n{}\n| C | changed |\n| D |\n",
        agreed_shape("| a claim", was)
    );
    assert_eq!(lost(&rows, &restored), [agreed.as_str(), "| C | two |"]);
}

/// Out of the box: an old cell written twice changes only once, so a row that
/// somehow held it twice cannot pass by a blanket replace; and a row that no
/// longer holds the old cell is left as it is and agrees with nothing new.
#[test]
fn the_old_cell_changes_once_and_a_row_without_it_stays() {
    let (_, was, now) = AGREED;
    let twice = agreed_shape("| a claim", &format!("{was} {was}"));
    assert_eq!(
        as_agreed(&twice, AGREED),
        agreed_shape("| a claim", &format!("{now} {was}"))
    );
    let without = agreed_shape("| a claim", "| no such cell |");
    assert_eq!(as_agreed(&without, AGREED), without);
}

/// **THE LEDGER HOLDS THE AGREED ROW ONCE, AS AGREED.** The old cell is gone from
/// it, so gate 10 reads the new one, and each test the cell names is declared
/// in the file the cell names for it.
#[test]
fn the_ledger_holds_the_agreed_row_once_and_its_tests_exist() {
    let (first, was, now) = AGREED;
    let ledger =
        std::fs::read_to_string(repo().join("docs/04-invariants.md")).expect("the invariants");
    let held: Vec<&str> = ledger
        .lines()
        .filter(|line| line.starts_with(first))
        .collect();
    assert_eq!(held.len(), 1, "{first} is one row");
    assert!(held[0].contains(now), "{first} reads as agreed");
    assert!(!held[0].contains(was), "{first} no longer names bare files");
    for files in now.trim_matches(['|', ' ']).split("; ") {
        let (file, tests) = files.split_once(": ").expect("a file, then its tests");
        let source = std::fs::read_to_string(repo().join(file.trim_matches('`')))
            .expect("the file the cell names");
        for test in tests.split(", ") {
            let declared = format!("fn {}(", test.trim_matches('`'));
            assert!(source.contains(&declared), "{file}: {declared}");
        }
    }
}

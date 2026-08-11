//! The table's own invariants, and the transcription checked against the
//! document it was transcribed from.
//!
//! Every test here is written so that it FAILS THE BUILD on a specific way of
//! getting the table wrong. A test that cannot fail is not a test
//! (`CLAUDE.md` §4), so each one names the mistake it catches.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]

use std::collections::BTreeMap;
use std::collections::BTreeSet;

use vocab::ConditionMask;
use vocab::table::{self, BitDef, BitStatus, COUNT, Kind, LIVE, NEXT_FREE, TABLE};

/// The shipped half of the table is not this crate's to choose, so it is read
/// out of the document rather than trusted to a careful copy-paste.
const VOCABULARY_DOC: &str = include_str!("../../../docs/03-vocabulary.md");

/// The three retirements, and what each one duplicated.
const TOMBSTONES: [(u16, u16); 3] = [(6, 62), (19, 17), (25, 18)];

/// Parse the `| 6 | near_pivot_p |` rows out of §5 of the vocabulary document.
fn shipped_names_from_the_document() -> Vec<(u16, String)> {
    // The WHOLE document, not just §5. The shipped table lives in §5 with three
    // columns; the appended positions live in §7 with four. Restricting the scan
    // to §5 is what let 200 positions sit undocumented while this test stayed
    // green.
    let section = VOCABULARY_DOC;

    let mut rows = Vec::new();
    for line in section.lines() {
        let cells: Vec<&str> = line.split('|').map(str::trim).collect();
        // `| 7 | `near_pivot_r1` |` splits to 4 cells; the §7 shape
        // `| 74 | `name` | — | live |` splits to 6. Accept any row whose second
        // cell is an index and whose third is a backticked name, so a new column
        // does not silently stop the scan.
        if cells.len() < 4 {
            continue;
        }
        let Ok(index) = cells[1].parse::<u16>() else {
            continue;
        };
        let Some(name) = cells[2].strip_prefix('`').and_then(|n| n.strip_suffix('`')) else {
            continue;
        };
        rows.push((index, name.to_owned()));
    }
    rows
}

/// **The shipped 74 are transcribed exactly, and nothing is undocumented.**
///
/// Two directions, and they are not the same check.
///
/// * **74 shipped positions: the DOCUMENT is the source.** Each is compared by
///   index against its row in `docs/03-vocabulary.md` §5. Rename one, drop one,
///   or slide a group boundary and this fails naming the index.
/// * **Positions 74–273: the CODE is the source**, and §7 of the document is the
///   record. So the check is COMPLETENESS — every live position has a row —
///   rather than transcription. Comparing a code-derived document back to the
///   code would prove nothing, and pretending otherwise would be worse than not
///   checking at all.
#[test]
fn the_shipped_74_are_the_document_character_for_character() {
    let doc = shipped_names_from_the_document();
    assert!(
        doc.len() >= 74,
        "the parser matched only {} rows; a silent zero or a short read here \
         would let a wrong transcription pass",
        doc.len(),
    );

    // Direction one: the shipped 74 must match character for character.
    let shipped: Vec<(u16, String)> = doc.iter().filter(|(i, _)| *i < 74).cloned().collect();
    assert_eq!(
        shipped.len(),
        74,
        "the document no longer defines all 74 shipped positions",
    );
    for (index, name) in shipped {
        let row = &TABLE[usize::from(index)];
        assert_eq!(row.index, index, "row {index} carries the wrong index");
        assert_eq!(
            row.name, name,
            "position {index} is `{}` here and `{name}` in the document",
            row.name,
        );
    }

    // Direction two: no position in the table may be absent from the document,
    // and no row in the document may name a position the table does not have.
    let documented: BTreeSet<u16> = doc.iter().map(|(i, _)| *i).collect();
    for def in &TABLE {
        assert!(
            documented.contains(&def.index),
            "position {} (`{}`) appears in no row of docs/03-vocabulary.md — an \
             undocumented condition is one nobody can audit",
            def.index,
            def.name,
        );
    }
    for index in &documented {
        assert!(
            usize::from(*index) < TABLE.len(),
            "docs/03-vocabulary.md documents position {index}, which the table \
             does not have",
        );
    }

    // And where the document names a position past the shipped 74, the name must
    // still agree — a wrong name in the record is a wrong audit trail even when
    // the record is not the source.
    for (index, name) in doc.iter().filter(|(i, _)| *i >= 74) {
        let row = &TABLE[usize::from(*index)];
        assert_eq!(
            row.name, *name,
            "position {index} is `{}` in the table and `{name}` in the document",
            row.name,
        );
    }
}

/// **No gap and no repeat, 0..=273 exactly.** The array's own length cannot
/// prove this: a row that repeats an index or skips one leaves the length
/// unchanged and quietly renumbers everything after it.
#[test]
fn the_table_is_a_contiguous_run_of_indices() {
    assert_eq!(COUNT, 274);
    let seen: BTreeSet<u16> = TABLE.iter().map(|d| d.index).collect();
    assert_eq!(seen.len(), COUNT, "an index is repeated");
    for (position, def) in TABLE.iter().enumerate() {
        assert_eq!(
            usize::from(def.index),
            position,
            "row {position} declares index {}, so every row after it means \
             something other than its position",
            def.index,
        );
    }
    assert_eq!(seen.first().copied(), Some(0));
    assert_eq!(seen.last().copied(), Some(273));
    assert_eq!(usize::from(NEXT_FREE), COUNT, "the next append goes at 274");
}

/// **No two live positions share a name.** Two rows with one name is two
/// conditions a ranked result cannot tell apart.
#[test]
fn no_two_live_positions_share_a_name() {
    let mut by_name: BTreeMap<&str, Vec<u16>> = BTreeMap::new();
    for def in TABLE.iter().filter(|d| d.status == BitStatus::Live) {
        by_name.entry(def.name).or_default().push(def.index);
    }
    for (name, indices) in &by_name {
        assert_eq!(
            indices.len(),
            1,
            "`{name}` is live at more than one position: {indices:?}",
        );
    }
    assert_eq!(
        by_name.len(),
        232,
        "274 positions, less three tombstones and less 39 void forming-pivot rows"
    );
}

/// **A tombstone is false, and its index is not reusable.**
///
/// Four separate ways of getting at the position, because the contract has to
/// hold however the caller arrives: the row still exists and still carries its
/// original name, it is not live, it cannot be set, and any mask that somehow
/// carries it is scrubbed.
#[test]
fn a_tombstone_keeps_its_index_and_always_evaluates_false() {
    // RETIRED and VOID are both "not live" and are NOT interchangeable. A
    // tombstone duplicates a live position and names it; a void position
    // duplicates nothing and is a constant. Asserting them separately is the
    // point — collapsing them would let a void row be introduced as a tombstone
    // with a made-up `duplicate_of`.
    let retired: BTreeSet<u16> = TABLE
        .iter()
        .filter(|d| matches!(d.status, BitStatus::Retired { .. }))
        .map(|d| d.index)
        .collect();
    let expected: BTreeSet<u16> = TOMBSTONES.iter().map(|&(index, _)| index).collect();
    assert_eq!(
        retired, expected,
        "the retired set changed; retiring a fourth position needs a decisions \
         entry, and un-retiring one is a renumbering by another name",
    );

    // The void set is the whole forming-day pivot family, 235..=273, and nothing
    // else. D-0080 records why every one of them is a constant.
    let void: BTreeSet<u16> = TABLE
        .iter()
        .filter(|d| matches!(d.status, BitStatus::Void { .. }))
        .map(|d| d.index)
        .collect();
    let expected_void: BTreeSet<u16> = (235..=273).collect();
    assert_eq!(
        void, expected_void,
        "the void set changed; a definitionally-constant position needs a \
         decisions entry naming the identity that makes it constant",
    );
    for index in 235..=273u16 {
        assert!(
            !table::is_live(index),
            "void position {index} reads as live"
        );
        assert!(
            table::set_exact(ConditionMask::ZERO, index).is_err(),
            "void position {index} could be set",
        );
        let def = table::definition(index).expect("a void position is occupied, not absent");
        let BitStatus::Void { reason } = def.status else {
            unreachable!("235..=273 are void")
        };
        assert!(!reason.is_empty(), "void position {index} gives no reason");
    }

    for (index, duplicate_of) in TOMBSTONES {
        let def: &BitDef = table::definition(index).expect("a tombstone is occupied, not absent");
        assert_eq!(def.index, index, "the position is still its own index");
        assert_eq!(
            def.status,
            BitStatus::Retired { duplicate_of },
            "position {index} must name the position it duplicated",
        );
        assert!(!table::is_live(index));
        assert!(table::set_exact(ConditionMask::ZERO, index).is_err());
        assert!(
            !LIVE.get(u32::from(index)),
            "the live mask excludes {index}"
        );

        let smuggled = ConditionMask::ZERO.with_bit(u32::from(index));
        assert!(
            table::only_live(smuggled).is_empty(),
            "position {index} survived the live filter",
        );

        // The position it duplicated is live, and is the one to set.
        assert!(
            table::is_live(duplicate_of),
            "a tombstone that points at another tombstone points nowhere",
        );
    }
}

/// **A retirement frees nothing.** The specific accident this guards: someone
/// appends a new condition and, seeing that 6, 19 and 25 are "unused", puts it
/// there. Every stored mask that carries bit 6 would then mean the new
/// condition.
#[test]
fn no_live_row_occupies_a_retired_position() {
    for (index, _) in TOMBSTONES {
        let def = &TABLE[usize::from(index)];
        assert!(
            !matches!(def.status, BitStatus::Live),
            "position {index} has been re-used by `{}`",
            def.name,
        );
    }
    let last = TABLE.last().expect("the table is not empty");
    assert_eq!(last.index, 273);
    assert_eq!(
        usize::from(NEXT_FREE),
        COUNT,
        "the next free position is past the end and not a recycled tombstone",
    );
}

/// The `Kind` a row declares matches the name it carries. Without this the
/// tolerance gate is one typo away from letting a `near_*` position be set as
/// though it decided on its own.
#[test]
fn every_near_name_is_a_near_kind_and_no_other_is() {
    let mut near = 0;
    for def in &TABLE {
        let looks_near = def.name.contains("near");
        assert_eq!(
            def.kind == Kind::Near,
            looks_near,
            "position {} is `{}` and declares {:?}",
            def.index,
            def.name,
            def.kind,
        );
        if looks_near {
            near += 1;
        }
    }
    assert_eq!(near, 97, "97 of the 274 positions need the tolerance");
    let live_near = TABLE
        .iter()
        .filter(|d| d.kind == Kind::Near && d.status == BitStatus::Live)
        .count();
    assert_eq!(live_near, 81, "three of the 76 are tombstones");
}

/// Names are `snake_case` ASCII. A stray capital or space is the sort of thing
/// that survives review and then fails to parse in whatever reads a mask back.
#[test]
fn every_name_is_snake_case_ascii() {
    for def in &TABLE {
        let name = def.name;
        assert!(!name.is_empty(), "position {} has no name", def.index);
        assert!(
            name.bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_'),
            "`{name}` is not lowercase ASCII, digits and underscores",
        );
        assert!(
            !name.starts_with('_') && !name.ends_with('_') && !name.contains("__"),
            "`{name}` has a stray underscore",
        );
    }
}

/// The 200 appended positions are in the groups they were specified in, at the
/// boundaries they were specified at. This is what catches a group that is one
/// row short: the count still reaches 274 because the next group absorbed the
/// difference, and every name after the seam means the wrong thing.
#[test]
fn the_appended_groups_start_and_end_where_they_are_specified() {
    let groups: [(u16, u16, &str); 13] = [
        (74, 85, "_band"),
        (86, 105, "orb"),
        (106, 109, "near_fib_bull_"),
        (110, 120, "near_fib_prev5_"),
        (121, 131, "near_fib_curday_"),
        (132, 142, "near_fib_gap_"),
        (143, 152, "vwap"),
        (153, 177, "pat_"),
        (178, 187, "pivot_"),
        (188, 189, "cpr_"),
        (190, 197, "vwap_band"),
        (198, 234, "pat_"),
        (235, 273, "forming_pivot_"),
    ];
    let mut covered = 0;
    for (first, last, marker) in groups {
        for index in first..=last {
            let name = TABLE[usize::from(index)].name;
            assert!(
                name.contains(marker),
                "position {index} is `{name}`, which is not in the {first}–{last} \
                 group (expected to contain `{marker}`)",
            );
            covered += 1;
        }
        // The row before the group must NOT look like it belongs to it, or the
        // boundary has slid.
        if first > 74 {
            let before = TABLE[usize::from(first) - 1].name;
            assert!(
                !before.contains(marker),
                "position {} is `{before}` and reads as part of the {first}–{last} \
                 group, so the boundary has moved",
                first - 1,
            );
        }
    }
    assert_eq!(covered, 200, "the appended range is 74..=273");
    assert_eq!(COUNT - 74, 200);
}

/// The eleven-rung Fibonacci ladders are eleven rungs, in one order, three
/// times over. A missing rung in one of them is otherwise invisible: the
/// group's row count is checked above, so a duplicate would fill the hole.
#[test]
fn the_three_fibonacci_ladders_carry_the_same_eleven_rungs() {
    const RUNGS: [&str; 11] = [
        "0", "236", "382", "50", "618", "786", "100", "1272", "1618", "200", "2618",
    ];
    for (first, prefix) in [
        (110u16, "near_fib_prev5_"),
        (121, "near_fib_curday_"),
        (132, "near_fib_gap_"),
    ] {
        for (offset, rung) in RUNGS.iter().enumerate() {
            let index = usize::from(first) + offset;
            assert_eq!(
                TABLE[index].name,
                format!("{prefix}{rung}"),
                "rung {rung} of the {prefix} ladder is not at position {index}",
            );
        }
    }
}

/// Two positions with different names but the same predicate are what the
/// three tombstones are for, so the pairs that motivated them must not have
/// quietly come back as new rows.
#[test]
fn the_retired_duplicates_were_not_re_added_under_new_names() {
    let live: BTreeSet<&str> = TABLE
        .iter()
        .filter(|d| d.status == BitStatus::Live)
        .map(|d| d.name)
        .collect();
    for name in ["near_pivot_p", "near_fib_0", "near_fib_100"] {
        assert!(
            !live.contains(name),
            "`{name}` is live again; it was retired because it duplicates \
             another position exactly",
        );
    }
}

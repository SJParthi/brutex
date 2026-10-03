//! The timestamp lookup's cost is stated where the limits live — W3-store1-0,
//! W3-store1-1, ET-bars-candles-store-12, D-1434.
//!
//! `store::file::first_at_or_after` is a bisection and `already_stored` is
//! that bisection plus a batch-long comparison. Neither is O(1) and both said
//! so only in source comments: `docs/06-limits.md` had no entry. Two documents
//! and one source comment also said "no bench in this repository times a
//! syscall", which C-28 and C-29 contradict — both time
//! `BarFile::read_record`, and that is one `pread` per call.
//!
//! The probe COUNT is proven in-module by
//! `store::file::first_at_or_after_never_probes_more_than_the_bisection_height`.
//! This file proves the WORDS: that the register carries the bound and that
//! the stale sentence is gone from every copy.

// A missing document section is a test failure, said by name; the same
// allowance every other file in `crates/store/tests` carries.
#![allow(clippy::expect_used, clippy::panic)]

/// The limits register.
const LIMITS: &str = include_str!("../../../docs/06-limits.md");
/// The invariant table.
const INVARIANTS: &str = include_str!("../../../docs/04-invariants.md");
/// The source that holds the bisection and `read_record`.
const FILE_RS: &str = include_str!("../src/file.rs");

/// Every run of whitespace collapsed to one space, so a phrase is found
/// wherever Markdown or a doc comment wrapped it. `///` markers are removed
/// first so a sentence split across two doc-comment lines reads whole.
fn flat(text: &str) -> String {
    text.replace("///", " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// The register's D-1434 section, from its heading to the next `## `.
fn section() -> String {
    let start = "## Timestamp lookup is a bisection, not O(1) — D-1434";
    let from = LIMITS
        .find(start)
        .unwrap_or_else(|| panic!("docs/06-limits.md has no `{start}` section"));
    let rest = &LIMITS[from + start.len()..];
    let to = rest.find("\n## ").unwrap_or(rest.len());
    flat(&rest[..to])
}

#[test]
fn the_limits_register_states_both_lookup_bounds_and_their_callers() {
    let body = section();
    for needle in [
        // The two functions and the bound each carries.
        "`first_at_or_after`",
        "`already_stored`",
        "`ceil(log2(n_valid + 1))`",
        "`ceil(log2(n_valid + 1)) + batch`",
        // The month ceiling, as a number a reader can check.
        "31 × 375 = 11,625",
        "fourteen",
        // The per-probe cost the bound hides.
        "cold block verify",
        "4,088",
        // The callers.
        "`/bars.json`",
        "twice per request",
        "`BarFile::append`",
        // The proof, by name, and what remains unmeasured.
        "first_at_or_after_never_probes_more_than_the_bisection_height",
        "UNVERIFIED",
    ] {
        assert!(
            body.contains(needle),
            "docs/06-limits.md D-1434 section does not say `{needle}`"
        );
    }
}

#[test]
fn no_copy_still_says_no_bench_times_a_syscall() {
    // C-28 and C-29 time `read_record`, which is one positional read. The
    // sentence was true when it was written and stopped being true when
    // those rows landed; three copies kept saying it.
    let stale = "no bench in this repository times a syscall";
    for (name, text) in [
        ("docs/06-limits.md", LIMITS),
        ("docs/04-invariants.md", INVARIANTS),
        ("crates/store/src/file.rs", FILE_RS),
    ] {
        let lower = flat(text).to_lowercase();
        assert!(
            !lower.contains(stale),
            "{name} still claims `{stale}`; C-28/C-29 time a pread"
        );
    }
    // And the correction names what IS timed, so the sentence was replaced
    // rather than deleted.
    let source = flat(FILE_RS);
    assert!(
        source.contains("C-28 and C-29 time it"),
        "file.rs `read_record` does not name the rows that time its pread"
    );
}

#[test]
fn the_invariant_row_names_the_probe_test() {
    let row = INVARIANTS
        .lines()
        .find(|line| line.starts_with("| S-BISECT-01 |"))
        .expect("docs/04-invariants.md has no S-BISECT-01 row");
    assert!(
        row.contains("store::file::first_at_or_after_never_probes_more_than_the_bisection_height"),
        "S-BISECT-01 does not name the test that proves it"
    );
}

/// audit-20261003 o1store2-1 (D-1527). The register's ceiling counted session
/// minutes only, but the store admits any on-grid stamp inside the month
/// (D-0915): a one-minute month can hold every minute of 31 days and a
/// one-second month every second. The section states both ceilings and their
/// heights, so "fourteen reads" is not read as the bound for every file.
#[test]
fn the_bisection_ceiling_counts_every_minute_the_store_admits() {
    let body = section();
    for needle in [
        "31 × 1,440 = 44,640",
        "sixteen",
        "2,678,400",
        "twenty-two",
        "D-0915",
    ] {
        assert!(
            body.contains(needle),
            "docs/06-limits.md D-1434 section does not say `{needle}`"
        );
    }
}

/// audit-20261003 o1store2-2 (D-1527). Since D-1433 and D-0914 the cold
/// verify reads into the handle's fixed buffer and a stack array, and
/// `crates/store/tests/cold_read.rs` fails the build on a `vec!` in either
/// function. The register must not still say the path allocates.
#[test]
fn no_register_passage_still_puts_a_heap_buffer_on_the_cold_verify() {
    let limits = flat(LIMITS);
    for stale in ["a heap buffer of the same size", "two heap buffers"] {
        assert!(
            !limits.contains(stale),
            "docs/06-limits.md still says `{stale}` of the cold verify"
        );
    }
}

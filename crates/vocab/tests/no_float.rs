//! There is no float in this crate, proved by reading its own source.
//!
//! `CLAUDE.md` §7 makes prices paisa integers, and this crate holds no price
//! at all -- a position is a `u16`, a band is thousandths of an `i64`. So the
//! rule here is stricter than the workspace's: not "floats are confined to one
//! boundary" but **there is no float, anywhere, including in the tests and in
//! the doc examples**.
//!
//! Nothing about a type can state that. `clippy::float_arithmetic` catches
//! arithmetic and not a declaration; a `struct S(f64)` with no operators is
//! clean under every lint this workspace runs. So the source is read.
//!
//! The scan closes its own list two ways, because a scan that quietly stops
//! covering what it claims to cover is worse than no scan: the files below are
//! checked against the `pub mod` lines in `lib.rs`, and against the directory
//! on disk. Adding `src/pivot.rs` is a failing test until someone lists it
//! here and looks at it.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]

use std::collections::BTreeSet;

/// Every source file in the crate, by hand because `include_str!` takes a
/// literal, and closed against `lib.rs` and the directory below.
const SOURCES: [(&str, &str); 6] = [
    ("lib.rs", include_str!("../src/lib.rs")),
    ("error.rs", include_str!("../src/error.rs")),
    ("implication.rs", include_str!("../src/implication.rs")),
    ("mask.rs", include_str!("../src/mask.rs")),
    ("table.rs", include_str!("../src/table.rs")),
    ("tolerance.rs", include_str!("../src/tolerance.rs")),
];

/// Every test file, held to the same rule. A float that only appears in a test
/// is still a float in this crate, and a test is where one would arrive first:
/// it is the natural place to reach for `0.618` rather than `618`.
const TEST_SOURCES: [(&str, &str); 4] = [
    ("tests/mask.rs", include_str!("mask.rs")),
    ("tests/no_float.rs", include_str!("no_float.rs")),
    ("tests/table.rs", include_str!("table.rs")),
    (
        "tests/workspace_is_rust.rs",
        include_str!("workspace_is_rust.rs"),
    ),
];

/// The two tokens, assembled rather than written, for the same reason CI's
/// gate 15 assembles the word it bans: a file that scans for a token it also
/// contains cannot pass its own scan, and the repair for that is always to
/// exempt the scanner -- which is the one file best placed to hide things.
fn banned_tokens() -> [String; 2] {
    [format!("f{}", 32), format!("f{}", 64)]
}

/// Whole-line comments are dropped and nothing else is. That is the same
/// boundary CI gate 11 draws: a doc comment may say the word "float", and a
/// line of code may not contain the type.
fn code_lines(text: &str) -> impl Iterator<Item = (usize, &str)> {
    text.lines()
        .enumerate()
        .map(|(i, l)| (i + 1, l))
        .filter(|(_, l)| !l.trim_start().starts_with("//"))
}

#[test]
fn no_float_anywhere_in_the_crate() {
    let tokens = banned_tokens();
    let mut scanned = 0;
    for (name, text) in SOURCES.iter().chain(TEST_SOURCES.iter()) {
        for (number, line) in code_lines(text) {
            scanned += 1;
            for token in &tokens {
                assert!(
                    !line.contains(token.as_str()),
                    "{name}:{number} names `{token}`. This crate holds positions \
                     and integer bands, and CLAUDE.md §7 leaves no float path \
                     for it to be on the way to: {line}",
                );
            }
        }
    }
    assert!(
        scanned > 500,
        "the scan read {scanned} lines of code, which is not this crate. A \
         silent near-zero is not a pass",
    );
}

/// The hand-written list is the whole crate, checked against `lib.rs`.
#[test]
fn the_source_list_matches_the_module_declarations() {
    let lib = SOURCES[0].1;
    let declared: BTreeSet<String> = lib
        .lines()
        .filter_map(|l| l.trim().strip_prefix("pub mod "))
        .filter_map(|l| l.strip_suffix(';'))
        .map(|m| format!("{m}.rs"))
        .collect();
    assert!(
        !declared.is_empty(),
        "the `pub mod` pattern stopped matching, so this check measures \
         nothing -- a silent zero is not a pass",
    );

    let scanned: BTreeSet<&str> = SOURCES.iter().map(|(n, _)| *n).collect();
    for module in &declared {
        assert!(
            scanned.contains(module.as_str()),
            "{module} is declared in lib.rs and the float scan never opens it",
        );
    }
    for module in &scanned {
        assert!(
            *module == "lib.rs" || declared.contains(*module),
            "{module} is scanned and lib.rs no longer declares it",
        );
    }
}

/// And against the directory, which catches the file `lib.rs` never mentions.
///
/// `include_str!` cannot walk a tree, but a test can read one -- and reading
/// it is what makes the list above a claim about the crate rather than a claim
/// about five paths somebody typed.
#[test]
fn the_source_list_matches_the_directory() {
    for (dir, listed) in [
        (
            concat!(env!("CARGO_MANIFEST_DIR"), "/src"),
            SOURCES
                .iter()
                .map(|(n, _)| (*n).to_owned())
                .collect::<BTreeSet<String>>(),
        ),
        (
            concat!(env!("CARGO_MANIFEST_DIR"), "/tests"),
            TEST_SOURCES
                .iter()
                .map(|(n, _)| n.trim_start_matches("tests/").to_owned())
                .collect::<BTreeSet<String>>(),
        ),
    ] {
        let mut on_disk = BTreeSet::new();
        for entry in std::fs::read_dir(dir).expect("the crate's own directory is readable") {
            let entry = entry.expect("a directory entry is readable");
            let name = entry.file_name().to_string_lossy().into_owned();
            if std::path::Path::new(&name)
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("rs"))
            {
                on_disk.insert(name);
            }
        }
        assert!(!on_disk.is_empty(), "{dir} holds no source file");
        assert_eq!(
            on_disk, listed,
            "{dir} and the list in this file disagree. A source file the scan \
             never opens is a float it cannot see",
        );
    }
}

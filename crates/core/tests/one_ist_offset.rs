//! The IST offset is spelled by its three named authorities (and one pin of
//! them) and nowhere else
//! in production code.
//!
//! D-3512 (ONEAUTH-13). `pull::session::IST_OFFSET_SECS` and
//! `store::path::IST_OFFSET_SECS` are pinned to each other (`session.rs`), and
//! `indicators::IST_OFFSET_MICROS` documents itself as "one definition, three
//! crates". Lens L4 found the same 5 h 30 min re-typed as a bare literal in
//! production code across `pull`, `api`, `indicators`, `cli` and `runner`,
//! each agreeing today and none tied to an authority. This test walks every
//! crate's `src/` up to the first `#[cfg(test)]` item of each file, skips the
//! `*_tests.rs` files, and refuses any spelling of the offset outside the
//! three definition lines.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]

use std::path::{Path, PathBuf};

/// The three authorities: `(file, the defining line's text)`.
const AUTHORITIES: [(&str, &str); 4] = [
    (
        "crates/pull/src/session.rs",
        "pub const IST_OFFSET_SECS: i64 = 5 * 3_600 + 30 * 60;",
    ),
    (
        "crates/pull/src/session.rs",
        "const _: () = assert!(IST_OFFSET_SECS == 19_800);",
    ),
    (
        "crates/store/src/path.rs",
        "pub const IST_OFFSET_SECS: i64 = 5 * 3_600 + 30 * 60;",
    ),
    (
        "crates/indicators/src/lib.rs",
        "pub const IST_OFFSET_MICROS: i64 = 19_800 * 1_000_000;",
    ),
];

/// Spellings of 19,800 seconds, with and without separators, in seconds,
/// minutes and the two-factor forms the tree used.
const SPELLINGS: [&str; 6] = [
    "19_800",
    "19800",
    "5 * 3600 + 30 * 60",
    "5 * 3_600 + 30 * 60",
    "5 * 60 * 60 + 30 * 60",
    "330 * 60",
];

/// Every `.rs` under `dir`, sorted.
fn sources(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)
        .expect("a source directory")
        .map(|e| e.expect("an entry").path())
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            sources(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// The production lines of one file: everything before its first
/// `#[cfg(test)]`, with `//` comments removed. `(1-based line, code)`.
fn production_lines(text: &str) -> Vec<(usize, &str)> {
    text.lines()
        .enumerate()
        .take_while(|(_, line)| line.trim() != "#[cfg(test)]")
        .map(|(n, line)| (n + 1, line.split("//").next().unwrap_or(line)))
        .collect()
}

/// Each production line in `text` (from `file`) that spells the offset and is
/// not one of the authorities.
fn restatements(file: &str, text: &str) -> Vec<String> {
    production_lines(text)
        .into_iter()
        .filter(|(_, code)| SPELLINGS.iter().any(|s| code.contains(s)))
        .filter(|(_, code)| {
            !AUTHORITIES
                .iter()
                .any(|(f, line)| *f == file && code.trim() == *line)
        })
        .map(|(n, code)| format!("{file}:{n}: {}", code.trim()))
        .collect()
}

#[test]
fn the_ist_offset_is_spelled_only_by_its_authorities() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let root = root.canonicalize().expect("the repository root");
    let mut found = Vec::new();
    let mut authorities_seen = 0;
    let mut crates: Vec<PathBuf> = std::fs::read_dir(root.join("crates"))
        .expect("crates/")
        .map(|e| e.expect("an entry").path().join("src"))
        .filter(|src| src.is_dir())
        .collect();
    crates.sort();
    for src in crates {
        let mut files = Vec::new();
        sources(&src, &mut files);
        for path in files {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name.ends_with("_tests.rs") {
                continue;
            }
            let file = path.strip_prefix(&root).expect("under the root");
            let file = file.to_string_lossy().replace('\\', "/");
            let text = std::fs::read_to_string(&path).expect("readable source");
            authorities_seen += AUTHORITIES
                .iter()
                .filter(|(f, line)| *f == file && text.lines().any(|l| l.trim() == *line))
                .count();
            found.extend(restatements(&file, &text));
        }
    }
    assert_eq!(
        authorities_seen,
        AUTHORITIES.len(),
        "an authority line moved"
    );
    assert_eq!(found, Vec::<String>::new());
}

#[test]
fn the_reader_skips_tests_comments_and_the_authorities() {
    let text = "const A: i64 = 19_800; // x\n// 19800 in prose\nfn f() { 5 * 3600 + 30 * 60 }\n\
                #[cfg(test)]\nmod tests { const B: i64 = 19_800; }\n";
    assert_eq!(
        restatements("crates/x/src/a.rs", text),
        vec![
            "crates/x/src/a.rs:1: const A: i64 = 19_800;".to_owned(),
            "crates/x/src/a.rs:3: fn f() { 5 * 3600 + 30 * 60 }".to_owned(),
        ]
    );
    let authority = format!("    {}\n", AUTHORITIES[0].1);
    assert!(restatements(AUTHORITIES[0].0, &authority).is_empty());
    assert_eq!(restatements("crates/x/src/b.rs", &authority).len(), 1);
}

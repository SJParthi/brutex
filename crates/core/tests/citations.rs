//! Every decision a tracked file cites heads an entry, and every `I-` invariant
//! the crates cite has a row.
//!
//! # Why this file exists
//!
//! Gate 27b proves a decision number heads at most one entry, and
//! `docs/06-limits.md` says what it does not prove: "A number that heads nothing
//! is not reported." Lens L4 (D-3504) resolved every `D-NNNN` in the tracked tree
//! against `docs/05-decisions.md` and found two citations of numbers that head
//! nothing — `docs/04-invariants.md`'s "D-2710 onward" (the run starts at
//! D-2712) and `.github/source_scan.rs`'s "D-1600..D-1619" (the run ends at
//! D-1614) — and `crates/core/src/vendor.rs` citing invariant `I-41`, which had
//! no row. A citation that resolves to nothing reads exactly like one that
//! resolves; only reading the target tells them apart, so this test reads it.
//!
//! # What it reads
//!
//! The root documents and manifests, `docs/` except the ledger itself,
//! `.github/`, and every `.rs`, `.toml` and `.md` under `crates/`. `web/` is not
//! read: it is the unrestricted front end (D-0053) and cites no decision as
//! authority. Three numbers are named in [`UNWRITTEN`] with the reason each
//! heads nothing; any other unresolved citation fails.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// The decision ledger, whose headings are the numbers that exist.
const DECISIONS: &str = include_str!("../../../docs/05-decisions.md");

/// The invariant document, whose first cells are the invariant ids.
const INVARIANTS: &str = include_str!("../../../docs/04-invariants.md");

/// Cited numbers that head nothing, each for a stated reason.
const UNWRITTEN: [(&str, &str); 3] = [
    ("D-0051", "never written; D-0684 records the gap"),
    ("D-0676", "held by a parked change; D-0684 records it"),
    ("D-1234", "a fixture number inside a gate tool's own test"),
];

/// The repository root, two levels above this crate's manifest.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every `D-NNNN` in `text` that is a whole token: no letter, digit or `_`
/// before the `D`, and no digit after the fourth.
fn decision_tokens(text: &str) -> BTreeSet<String> {
    tokens(text, "D-", |digits| digits.len() == 4)
}

/// Every `I-N…` in `text` that is a whole token, with an optional lowercase
/// suffix letter, as `docs/04-invariants.md` writes them (`I-16a`).
fn invariant_tokens(text: &str) -> BTreeSet<String> {
    tokens(text, "I-", |digits| !digits.is_empty())
}

/// The shared scanner: `prefix`, then the ASCII digits `ok` accepts, then (for
/// the `I-` form) at most one lowercase letter, then no identifier character.
fn tokens(text: &str, prefix: &str, ok: impl Fn(&str) -> bool) -> BTreeSet<String> {
    let bytes = text.as_bytes();
    let word = |b: u8| b.is_ascii_alphanumeric() || b == b'_';
    let mut out = BTreeSet::new();
    let mut from = 0;
    while let Some(found) = text.get(from..).and_then(|rest| rest.find(prefix)) {
        let at = from + found;
        from = at + prefix.len();
        if at > 0 && word(bytes[at - 1]) {
            continue;
        }
        let digits_end = (from..bytes.len())
            .find(|&k| !bytes[k].is_ascii_digit())
            .unwrap_or(bytes.len());
        let mut end = digits_end;
        if prefix == "I-" && end < bytes.len() && bytes[end].is_ascii_lowercase() {
            end += 1;
        }
        if end < bytes.len() && word(bytes[end]) {
            continue;
        }
        if ok(&text[from..digits_end]) {
            out.insert(text[at..end].to_owned());
        }
    }
    out
}

/// The numbers `docs/05-decisions.md` heads: `#`-led lines whose first word
/// after the hashes is `D-NNNN`.
fn headed(ledger: &str) -> BTreeSet<String> {
    ledger
        .lines()
        .filter_map(|line| {
            let rest = line.trim_start_matches('#');
            (rest.len() < line.len()).then(|| rest.trim_start())
        })
        .filter_map(|rest| decision_tokens(rest.get(..6)?).into_iter().next())
        .collect()
}

/// The `I-` ids `docs/04-invariants.md` gives a row: a table line whose first
/// cell is the id, backticked or not, alone or before ` — `.
fn invariant_rows(document: &str) -> BTreeSet<String> {
    document
        .lines()
        .filter_map(|line| line.strip_prefix('|'))
        .filter_map(|rest| rest.split('|').next())
        .map(|cell| {
            cell.split(" — ")
                .next()
                .unwrap_or(cell)
                .trim()
                .trim_matches('`')
        })
        .filter(|cell| invariant_tokens(cell).len() == 1 && invariant_tokens(cell).contains(*cell))
        .map(str::to_owned)
        .collect()
}

/// Every file under `dir` with one of `extensions`, depth first, sorted, never
/// following into `target`.
fn walk(dir: &Path, extensions: &[&str], out: &mut Vec<PathBuf>) {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("{} cannot be listed: {e}", dir.display()))
        .map(|entry| entry.expect("a directory entry").path())
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            if path.file_name().is_some_and(|n| n != "target") {
                walk(&path, extensions, out);
            }
        } else if path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| extensions.contains(&e))
        {
            out.push(path);
        }
    }
}

/// The files whose citations are checked, as described in the module header.
fn cited_files() -> Vec<PathBuf> {
    let root = root();
    let mut files: Vec<PathBuf> = [
        "CLAUDE.md",
        "AGENTS.md",
        "README.md",
        "Cargo.toml",
        "deny.toml",
    ]
    .iter()
    .map(|name| root.join(name))
    .collect();
    walk(&root.join("docs"), &["md"], &mut files);
    walk(&root.join(".github"), &["rs", "yml"], &mut files);
    walk(&root.join("crates"), &["rs", "toml", "md"], &mut files);
    // This file names the dangling numbers it found, as text a reader needs.
    files.retain(|path| {
        !path.ends_with("docs/05-decisions.md") && !path.ends_with("core/tests/citations.rs")
    });
    files
}

/// D-3504 (ONEAUTH-05). Every `D-NNNN` cited in the files above heads an entry
/// in `docs/05-decisions.md`, or is one of the [`UNWRITTEN`] three.
#[test]
fn every_cited_decision_heads_an_entry() {
    let heads = headed(DECISIONS);
    assert!(heads.len() > 1000, "read only {} headings", heads.len());
    let unwritten: BTreeSet<&str> = UNWRITTEN.iter().map(|(n, _)| *n).collect();
    for number in &unwritten {
        assert!(
            !heads.contains(*number),
            "{number} now heads an entry; drop it from UNWRITTEN"
        );
    }
    let files = cited_files();
    assert!(files.len() > 500, "read only {} files", files.len());
    let mut dangling = Vec::new();
    for path in files {
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("{} cannot be read: {e}", path.display()));
        for number in decision_tokens(&text) {
            if !heads.contains(&number) && !unwritten.contains(number.as_str()) {
                dangling.push(format!("{} cites {number}", path.display()));
            }
        }
    }
    assert_eq!(dangling, Vec::<String>::new());
}

/// D-3504 (ONEAUTH-05). Every `I-` invariant cited under `crates/` has a row.
#[test]
fn every_invariant_the_crates_cite_has_a_row() {
    let rows = invariant_rows(INVARIANTS);
    assert!(rows.contains("I-01") && rows.contains("I-39"), "{rows:?}");
    let mut files = Vec::new();
    walk(&root().join("crates"), &["rs"], &mut files);
    files.retain(|path| !path.ends_with("core/tests/citations.rs"));
    let mut dangling = Vec::new();
    for path in files {
        let text = std::fs::read_to_string(&path).expect("readable source");
        for id in invariant_tokens(&text) {
            if !rows.contains(&id) {
                dangling.push(format!("{} cites {id}", path.display()));
            }
        }
    }
    assert_eq!(dangling, Vec::<String>::new());
}

/// The readers themselves, on every boundary the tests above rely on.
#[test]
fn the_citation_readers_read_whole_tokens_only() {
    let set = |xs: &[&str]| xs.iter().map(|x| (*x).to_owned()).collect::<BTreeSet<_>>();
    assert_eq!(
        decision_tokens("D-0001 (D-0002), xD-0003 D-00045 D-004 _D-0005 D-0006a D-0007\n"),
        set(&["D-0001", "D-0002", "D-0007"])
    );
    assert_eq!(decision_tokens("D-"), set(&[]));
    assert_eq!(
        invariant_tokens("I-1 I-16a I-16ab XI-2 I- I-3_ `I-41`"),
        set(&["I-1", "I-16a", "I-41"])
    );
    assert_eq!(
        headed("### D-0001 — a\n## D-0002\nD-0003 prose\n#D-0004\n### see D-0005\n"),
        set(&["D-0001", "D-0002", "D-0004"])
    );
    assert_eq!(
        invariant_rows("| I-01 | a |\n| `I-02` | b |\n| I-03 — c | t |\n| I-04 x | d |\n| X-1 |\n"),
        set(&["I-01", "I-02", "I-03"])
    );
}

/// `CLAUDE.md`, whose §10 says which documents carry authority.
const LAW: &str = include_str!("../../../CLAUDE.md");

/// §10's table rows (`docs/…` paths with authority) and the inclusive range of
/// report numbers its "hold no authority" sentence names, read from the text.
fn authority_table(law: &str) -> (BTreeSet<String>, Option<(u32, u32)>) {
    let section = law.split_once("\n## 10.").map_or("", |(_, rest)| rest);
    let rows = section
        .lines()
        .filter_map(|line| line.strip_prefix("| `docs/"))
        .filter_map(|rest| rest.split_once('`'))
        .map(|(name, _)| format!("docs/{name}"))
        .collect();
    let range = section.split_once("**`docs/").and_then(|(_, rest)| {
        let (low, rest) = rest.split_once("-` to `docs/")?;
        let (high, _) = rest.split_once("-`")?;
        Some((low.parse().ok()?, high.parse().ok()?))
    });
    (rows, range)
}

/// How §10 classifies one `docs/` path: in the table, a numbered report in the
/// stated range, or under `docs/research-policy/`; `false` for none of them.
fn classified(path: &str, rows: &BTreeSet<String>, range: (u32, u32)) -> bool {
    if rows.contains(path) || path.starts_with("docs/research-policy/") {
        return true;
    }
    path.strip_prefix("docs/")
        .and_then(|name| name.split_once('-'))
        .and_then(|(number, _)| number.parse::<u32>().ok())
        .is_some_and(|n| (range.0..=range.1).contains(&n))
}

/// D-3509 (ONEAUTH-10). `CLAUDE.md` §10 is the one statement of which
/// documents bind. It listed eight of fourteen (D-0212) and then fourteen of
/// thirty-nine (D-1764) before anyone noticed, because nothing compared it with
/// the directory. Every table row names a file that exists, and every file
/// under `docs/` is a row, a numbered report inside the stated range, or under
/// `docs/research-policy/`.
#[test]
fn every_document_is_classified_by_the_law() {
    let (rows, range) = authority_table(LAW);
    assert_eq!(rows.len(), 14, "§10's table: {rows:?}");
    let range = range.expect("§10 names the report range");
    let root = root();
    for row in &rows {
        assert!(
            root.join(row).is_file(),
            "§10 names {row}, which does not exist"
        );
    }
    let mut files = Vec::new();
    walk(&root.join("docs"), &["md"], &mut files);
    let unclassified: Vec<String> = files
        .iter()
        .filter_map(|path| path.strip_prefix(&root).ok())
        .map(|path| path.to_string_lossy().replace('\\', "/"))
        .filter(|path| !classified(path, &rows, range))
        .collect();
    assert_eq!(
        unclassified,
        Vec::<String>::new(),
        "§10 says nothing about these"
    );
}

/// The §10 reader and classifier on their edges.
#[test]
fn the_authority_reader_reads_the_table_and_the_range() {
    let law = "## 9. x\n| `docs/99-no.md` | x |\n## 10. Documents\n| File | A |\n\
               | `docs/00-a.md` | a |\n| `docs/01-b.md` | b |\n\
               **`docs/12-` to `docs/35-` and more** text\n## 11. y\n";
    let (rows, range) = authority_table(law);
    let set = |xs: &[&str]| xs.iter().map(|x| (*x).to_owned()).collect::<BTreeSet<_>>();
    assert_eq!(rows, set(&["docs/00-a.md", "docs/01-b.md"]));
    assert_eq!(range, Some((12, 35)));
    assert_eq!(authority_table("no section"), (BTreeSet::new(), None));
    let range = (12, 35);
    assert!(classified("docs/00-a.md", &rows, range));
    assert!(classified("docs/12-x.md", &rows, range));
    assert!(classified("docs/35-x.md", &rows, range));
    assert!(classified("docs/research-policy/r.md", &rows, range));
    assert!(!classified("docs/36-x.md", &rows, range));
    assert!(!classified("docs/11-x.md", &rows, range));
    assert!(!classified("docs/notes.md", &rows, range));
}

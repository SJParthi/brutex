//! Two session facts are spelled by their named authorities (and the pins
//! that tie them) and nowhere else in production code: the IST offset and the
//! 09:15 open.
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
//!
//! D-3517 (ONEAUTH-18). Round 4 found `runner::synthetic::IST_OPEN_UTC_MICROS`
//! written as `(555 - 330) * 60 * 1_000_000`: the offset in MINUTES, a form the
//! spelling list did not hold. It also found the walk reading, as production,
//! every file a `#[cfg(test)] mod name;` declares under a name that is not
//! `*_tests.rs` (`pull/src/emit_sites.rs`, `store/src/emits.rs`,
//! `engine/src/manifest.rs` and three in `api`), a `pub(super) mod tests`, and
//! a test module with a comment among its attributes; those are skipped now.
//!
//! D-3518 (ONEAUTH-19). The 09:15 open had six production definitions:
//! `pull::session`, `pull::calendar`, `store::path`, a private copy in
//! `indicators::orb`, a private copy in `runner::resample` and the `555` inside
//! `runner::synthetic`, and a test-only seventh in `runner::exit_grid_policy`.
//! The last four were tied to nothing. `indicators` now holds
//! `SESSION_OPEN_MINUTE` for the sweep side of the graph and `runner` names it;
//! `crates/cli/tests/one_session_open.rs` holds every definition to
//! `pull::session`'s, and this test refuses another.

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

/// The offset's three authorities and `session`'s pin: `(file, the line)`.
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
/// minutes and the two-factor forms the tree used; and 330 minutes beside an
/// operator, the form `runner::synthetic` used (D-3517).
const SPELLINGS: [&str; 11] = [
    "19_800",
    "19800",
    "5 * 3600 + 30 * 60",
    "5 * 3_600 + 30 * 60",
    "5 * 60 * 60 + 30 * 60",
    "330 * 60",
    "5 * 3600 + 1800",
    "5 * 3_600 + 1_800",
    "(5 * 60 + 30) * 60",
    "- 330",
    "+ 330",
];

/// The open's four definitions, the two pins written as the literal, the
/// frozen V2 calendar-policy constant (its digest is a stored value, so it may
/// not follow a moved open), and the dated irregular sessions of
/// `pull::calendar` (each is what happened on its day, not the regular open).
const OPEN_AUTHORITIES: [(&str, &str); 9] = [
    (
        "crates/pull/src/session.rs",
        "pub const SESSION_OPEN_MINUTE: u32 = 9 * 60 + 15;",
    ),
    (
        "crates/pull/src/session.rs",
        "const _: () = assert!(SESSION_OPEN_MINUTE == 555 && SESSION_CLOSE_MINUTE == 930);",
    ),
    (
        "crates/pull/src/calendar.rs",
        "pub const OPEN_MINUTE: u16 = 9 * 60 + 15;",
    ),
    (
        "crates/pull/src/calendar.rs",
        "const _: () = assert!(OPEN_MINUTE == 555);",
    ),
    (
        "crates/store/src/path.rs",
        "pub const OPEN_MINUTES_PAST_IST_MIDNIGHT: u32 = 9 * 60 + 15;",
    ),
    (
        "crates/indicators/src/lib.rs",
        "pub const SESSION_OPEN_MINUTE: i64 = 9 * 60 + 15;",
    ),
    (
        "crates/cli/src/stored.rs",
        "const NSE_OPEN_MINUTE_V2: i64 = 555;",
    ),
    (
        "crates/pull/src/calendar.rs",
        "Window { from: 555, to: 699 },",
    ),
    (
        "crates/pull/src/calendar.rs",
        "windows: [Window { from: 555, to: 599 }, Window { from: 690, to: 749 }],",
    ),
];

/// Spellings of the open other than the whole number 555: the minute
/// arithmetic, and 33,300 seconds.
const OPEN_SPELLINGS: [&str; 3] = ["9 * 60 + 15", "33_300", "33300"];

/// One fact: the lines allowed to spell it and how a line spells it.
struct Fact {
    authorities: &'static [(&'static str, &'static str)],
    spells: fn(&str) -> bool,
}

/// The IST offset (D-3512, D-3517).
const OFFSET: Fact = Fact {
    authorities: &AUTHORITIES,
    spells: spells_offset,
};

/// The 09:15 open (D-3518).
const OPEN: Fact = Fact {
    authorities: &OPEN_AUTHORITIES,
    spells: spells_open,
};

fn spells_offset(code: &str) -> bool {
    SPELLINGS.iter().any(|s| code.contains(s))
}

fn spells_open(code: &str) -> bool {
    OPEN_SPELLINGS.iter().any(|s| code.contains(s)) || has_number(code, "555")
}

/// Whether `code` holds `digits` as a whole number: no identifier character
/// (and so no digit and no `_` separator) on either side. `INE555B01013` and
/// `5_555` do not hold 555.
fn has_number(code: &str, digits: &str) -> bool {
    let bytes = code.as_bytes();
    let word = |b: u8| b.is_ascii_alphanumeric() || b == b'_';
    code.match_indices(digits).any(|(at, _)| {
        let end = at + digits.len();
        (at == 0 || !word(bytes[at - 1])) && (end == bytes.len() || !word(bytes[end]))
    })
}

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

/// The line just past the attributes that start at `from`, however many lines
/// each spans, and the `#[path = "…"]` among them if there is one. A `//` line
/// among them is passed over as Rust passes it: `api::pullrun` comments
/// between its `#[cfg(test)]` and its `#[allow(…)]`.
fn past_attributes<'a>(lines: &[&'a str], from: usize) -> (usize, Option<&'a str>) {
    let mut item = from;
    let mut path = None;
    loop {
        while lines.get(item).is_some_and(|l| l.starts_with("//")) {
            item += 1;
        }
        if !lines.get(item).is_some_and(|l| l.starts_with("#[")) {
            break;
        }
        if let Some(p) = lines[item]
            .strip_prefix("#[path = \"")
            .and_then(|r| r.strip_suffix("\"]"))
        {
            path = Some(p);
        }
        let mut depth = 0_usize;
        while let Some(l) = lines.get(item) {
            depth = (depth + l.matches('[').count()).saturating_sub(l.matches(']').count());
            item += 1;
            if depth == 0 {
                break;
            }
        }
    }
    (item, path)
}

/// The name a line declares as a module, with the rest of the line: `mod x`,
/// `pub mod x`, or `pub(…) mod x` at any visibility (`indicators::column`'s
/// tests are `pub(super) mod tests`).
fn declared_mod(line: &str) -> Option<&str> {
    let rest = match line.strip_prefix("pub") {
        Some(after) => {
            let after = match after.strip_prefix('(') {
                Some(inner) => inner.split_once(')')?.1,
                None => after,
            };
            after.strip_prefix(' ')?
        }
        None => line,
    };
    rest.strip_prefix("mod ")
}

/// The production lines of one file, with `//` comments removed:
/// `(1-based line, code)`. A test module is skipped: from a column-0
/// `#[cfg(test)]` whose item, past any further attributes, is a `mod`, to the
/// column-0 `}` that closes it (or the line itself for `mod tests;`). A
/// column-0 `#[cfg(test)]` item that is not a module is read: strict rather
/// than blind (`runner::exit_grid_policy`'s test-only open was found so). An
/// indented `#[cfg(test)]` statement inside a production function does not
/// end the reading (the first version stopped there and missed `api`).
fn production_lines(text: &str) -> Vec<(usize, &str)> {
    let lines: Vec<&str> = text.lines().collect();
    let mut out = Vec::new();
    let mut n = 0;
    while n < lines.len() {
        if lines[n] == "#[cfg(test)]" {
            let (item, _) = past_attributes(&lines, n + 1);
            if let Some(rest) = lines.get(item).and_then(|l| declared_mod(l)) {
                n = if rest.trim_end().ends_with(';') {
                    item + 1
                } else {
                    (item + 1..lines.len())
                        .find(|&k| lines[k].starts_with('}'))
                        .map_or(lines.len(), |k| k + 1)
                };
                continue;
            }
        }
        let line = lines[n];
        out.push((n + 1, line.split("//").next().unwrap_or(line)));
        n += 1;
    }
    out
}

/// The files `text` (the source at `file`) declares with a column-0
/// `#[cfg(test)]` and `mod name;`: compiled only under test, whatever their
/// name. Both places Rust looks are returned, or the `#[path]` given, which
/// is relative to the declaring file's directory (D-3517).
fn test_module_files(file: &Path, text: &str) -> Vec<PathBuf> {
    let lines: Vec<&str> = text.lines().collect();
    let dir = file.parent().unwrap_or_else(|| Path::new(""));
    let stem = file.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    let mut out = Vec::new();
    for (n, line) in lines.iter().enumerate() {
        if *line != "#[cfg(test)]" {
            continue;
        }
        let (item, path) = past_attributes(&lines, n + 1);
        let Some(name) = lines
            .get(item)
            .and_then(|l| declared_mod(l))
            .and_then(|rest| rest.trim_end().strip_suffix(';'))
        else {
            continue;
        };
        if let Some(path) = path {
            out.push(dir.join(path));
        } else {
            let base = if matches!(stem, "lib" | "main" | "mod") {
                dir.to_path_buf()
            } else {
                dir.join(stem)
            };
            out.push(base.join(format!("{name}.rs")));
            out.push(base.join(name).join("mod.rs"));
        }
    }
    out
}

/// Each production line in `text` (from `file`) that spells `fact` and is
/// not one of its authorities.
fn restatements(fact: &Fact, file: &str, text: &str) -> Vec<String> {
    production_lines(text)
        .into_iter()
        .filter(|(_, code)| (fact.spells)(code))
        .filter(|(_, code)| {
            !fact
                .authorities
                .iter()
                .any(|(f, line)| *f == file && code.trim() == *line)
        })
        .map(|(n, code)| format!("{file}:{n}: {}", code.trim()))
        .collect()
}

/// Every production source under every crate's `src/`, as
/// `(repository-relative path, text)`: `*_tests.rs` files and the files a
/// `#[cfg(test)] mod name;` declares are left out.
fn production_sources() -> Vec<(String, String)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let root = root.canonicalize().expect("the repository root");
    let mut crates: Vec<PathBuf> = std::fs::read_dir(root.join("crates"))
        .expect("crates/")
        .map(|e| e.expect("an entry").path().join("src"))
        .filter(|src| src.is_dir())
        .collect();
    crates.sort();
    let mut files = Vec::new();
    for src in crates {
        sources(&src, &mut files);
    }
    let texts: Vec<(PathBuf, String)> = files
        .into_iter()
        .map(|path| {
            let text = std::fs::read_to_string(&path).expect("readable source");
            (path, text)
        })
        .collect();
    let test_only: BTreeSet<PathBuf> = texts
        .iter()
        .flat_map(|(path, text)| test_module_files(path, text))
        .filter_map(|path| path.canonicalize().ok())
        .collect();
    assert!(
        test_only.len() >= 6,
        "read only {} test-only module files",
        test_only.len()
    );
    texts
        .into_iter()
        .filter(|(path, _)| {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            !name.ends_with("_tests.rs") && !test_only.contains(path)
        })
        .map(|(path, text)| {
            let file = path.strip_prefix(&root).expect("under the root");
            (file.to_string_lossy().replace('\\', "/"), text)
        })
        .collect()
}

/// Every restatement of `fact` across the production sources, after checking
/// that each of its authority lines is still where it is named.
fn walk(fact: &Fact) -> Vec<String> {
    let mut found = Vec::new();
    let mut authorities_seen = 0;
    for (file, text) in production_sources() {
        authorities_seen += fact
            .authorities
            .iter()
            .filter(|(f, line)| *f == file && text.lines().any(|l| l.trim() == *line))
            .count();
        found.extend(restatements(fact, &file, &text));
    }
    assert_eq!(
        authorities_seen,
        fact.authorities.len(),
        "an authority line moved"
    );
    found
}

#[test]
fn the_ist_offset_is_spelled_only_by_its_authorities() {
    assert_eq!(walk(&OFFSET), Vec::<String>::new());
}

/// D-3518 (ONEAUTH-19).
#[test]
fn the_session_open_is_spelled_only_by_its_authorities() {
    assert_eq!(walk(&OPEN), Vec::<String>::new());
}

#[test]
fn the_reader_skips_tests_comments_and_the_authorities() {
    let text = "const A: i64 = 19_800; // x\n// 19800 in prose\nfn f() { 5 * 3600 + 30 * 60 }\n\
                #[cfg(test)]\nmod tests {\n    const B: i64 = 19_800;\n}\n\
                fn g() {\n    #[cfg(test)]\n    let x = 1;\n    5 * 3600 + 1800\n}\n\
                #[cfg(test)]\n#[path = \"t.rs\"]\nmod t;\nconst C: i64 = 19800;\n\
                #[cfg(test)]\n#[allow(\n    clippy::panic,\n)]\nmod u {\n    const D: i64 = 19_800;\n}\n\
                #[cfg(test)]\npub(crate) mod v {\n    const E: i64 = 19_800;\n}\n\
                #[cfg(test)]\n// a comment among the attributes\n#[allow(clippy::panic)]\n\
                pub(super) mod w {\n    const G: i64 = 19_800;\n}\n\
                #[cfg(test)]\nconst H: i64 = 19_800;\n\
                pub const F: i64 = (555 - 330) * 60 * 1_000_000;\n";
    assert_eq!(
        restatements(&OFFSET, "crates/x/src/a.rs", text),
        vec![
            "crates/x/src/a.rs:1: const A: i64 = 19_800;".to_owned(),
            "crates/x/src/a.rs:3: fn f() { 5 * 3600 + 30 * 60 }".to_owned(),
            "crates/x/src/a.rs:11: 5 * 3600 + 1800".to_owned(),
            "crates/x/src/a.rs:16: const C: i64 = 19800;".to_owned(),
            // A test-only item that is not a module is read: strict, not blind.
            "crates/x/src/a.rs:35: const H: i64 = 19_800;".to_owned(),
            "crates/x/src/a.rs:36: pub const F: i64 = (555 - 330) * 60 * 1_000_000;".to_owned(),
        ]
    );
    let authority = format!("    {}\n", AUTHORITIES[0].1);
    assert!(restatements(&OFFSET, AUTHORITIES[0].0, &authority).is_empty());
    assert_eq!(
        restatements(&OFFSET, "crates/x/src/b.rs", &authority).len(),
        1
    );
}

/// D-3518 (ONEAUTH-19): the open's spellings, and the numbers that only look
/// like one.
#[test]
fn the_open_reader_reads_whole_numbers_and_its_authorities() {
    let text = "const A: i64 = 555;\nlet isin = \"INE555B01013\";\n\
                const B: u16 = 9 * 60 + 15;\nlet x = 5_555 + 5550 + 1555;\n\
                let s = 33_300;\nfor m in 555..930 {}\n// 555 in prose\n";
    assert_eq!(
        restatements(&OPEN, "crates/x/src/a.rs", text),
        vec![
            "crates/x/src/a.rs:1: const A: i64 = 555;".to_owned(),
            "crates/x/src/a.rs:3: const B: u16 = 9 * 60 + 15;".to_owned(),
            "crates/x/src/a.rs:5: let s = 33_300;".to_owned(),
            "crates/x/src/a.rs:6: for m in 555..930 {}".to_owned(),
        ]
    );
    for (file, line) in OPEN_AUTHORITIES {
        assert!(
            restatements(&OPEN, file, &format!("    {line}\n")).is_empty(),
            "{file}: {line}"
        );
        assert_eq!(
            restatements(&OPEN, "crates/x/src/b.rs", &format!("{line}\n")).len(),
            1,
            "{line} is a spelling the reader sees"
        );
    }
}

/// D-3517 (ONEAUTH-18): which files a `#[cfg(test)] mod name;` declares.
#[test]
fn a_test_only_module_file_is_found_wherever_it_is_declared() {
    let lib = "#[cfg(test)]\nmod emit_sites;\n#[cfg(test)]\n#[path = \"a_tests.rs\"]\nmod tests;\n\
               pub mod real;\n#[cfg(test)]\npub(crate) mod scratch;\n#[cfg(test)]\nmod inline {\n}\n\
               #[cfg(test)]\n// why\npub(in crate::x) mod hidden;\n#[cfg(test)]\npubx mod not_a_module;\n\
               #[cfg(all(test, unix))]\nmod not_this;\n";
    assert_eq!(
        test_module_files(Path::new("crates/x/src/lib.rs"), lib),
        vec![
            PathBuf::from("crates/x/src/emit_sites.rs"),
            PathBuf::from("crates/x/src/emit_sites/mod.rs"),
            PathBuf::from("crates/x/src/a_tests.rs"),
            PathBuf::from("crates/x/src/scratch.rs"),
            PathBuf::from("crates/x/src/scratch/mod.rs"),
            PathBuf::from("crates/x/src/hidden.rs"),
            PathBuf::from("crates/x/src/hidden/mod.rs"),
        ]
    );
    assert_eq!(
        test_module_files(
            Path::new("crates/x/src/server.rs"),
            "#[cfg(test)]\nmod serve_edge_tests;\n"
        ),
        vec![
            PathBuf::from("crates/x/src/server/serve_edge_tests.rs"),
            PathBuf::from("crates/x/src/server/serve_edge_tests/mod.rs"),
        ]
    );
    assert!(test_module_files(Path::new("crates/x/src/lib.rs"), "mod real;\n").is_empty());
}

//! One function builds each shared results path.
//!
//! D-3506 (ONEAUTH-07). `results/population-write.lock` serialises every
//! population, admission and execution writer; four writers each built it in a
//! private `lock_path` and two readers inline. `results/runs.bin` was built by
//! `cli::results::Results::path` and again by `api::backtest::path_in`. The six
//! and the two agreed, and nothing kept them agreeing: a renamed lock in one
//! writer would lock a file nobody else locks, and every writer would still
//! succeed. Each construction now lives in one function and the others call it;
//! this test counts the construction across every crate's `src/`.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]

use std::path::{Path, PathBuf};

/// Each shared file name as the end of a string literal — `"…runs.bin"` in any
/// spelling: a `.join`, a `Path::new("results/runs.bin")`, a `format!` — and
/// the file that holds its one construction. The first version counted only
/// `.join("runs.bin")` and passed `.join("results/runs.bin")` (D-3515).
const OWNED: [(&str, &str); 2] = [
    ("population-write.lock\"", "crates/cli/src/population.rs"),
    ("runs.bin\"", "crates/cli/src/results.rs"),
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

/// The production lines of one source, `//` comments removed: a column-0
/// `#[cfg(test)]` whose item, past its further attributes (however many lines
/// each spans), is a `mod` is skipped to the column-0 `}` that closes it, or
/// past the `;` of `mod tests;`. `(1-based line, code)`.
fn production_lines(text: &str) -> Vec<(usize, &str)> {
    let lines: Vec<&str> = text.lines().collect();
    let mut out = Vec::new();
    let mut n = 0;
    while n < lines.len() {
        if lines[n] == "#[cfg(test)]" {
            let mut item = n + 1;
            while lines.get(item).is_some_and(|l| l.starts_with("#[")) {
                let mut depth = 0_usize;
                while let Some(l) = lines.get(item) {
                    depth = (depth + l.matches('[').count()).saturating_sub(l.matches(']').count());
                    item += 1;
                    if depth == 0 {
                        break;
                    }
                }
            }
            if lines
                .get(item)
                .is_some_and(|l| l.starts_with("mod ") || l.starts_with("pub mod "))
            {
                n = if lines[item].trim_end().ends_with(';') {
                    item + 1
                } else {
                    (item + 1..lines.len())
                        .find(|&k| lines[k].starts_with('}'))
                        .map_or(lines.len(), |k| k + 1)
                };
                continue;
            }
        }
        out.push((n + 1, lines[n].split("//").next().unwrap_or(lines[n])));
        n += 1;
    }
    out
}

/// Each production `file:line` that holds `needle`, skipping `*_tests.rs`
/// files and test modules, across every crate's `src/`.
fn constructions(root: &Path, needle: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut members: Vec<PathBuf> = std::fs::read_dir(root.join("crates"))
        .expect("crates/")
        .map(|e| e.expect("an entry").path().join("src"))
        .filter(|src| src.is_dir())
        .collect();
    members.sort();
    for src in members {
        let mut files = Vec::new();
        sources(&src, &mut files);
        for file in files {
            if file
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.ends_with("_tests.rs"))
            {
                continue;
            }
            let text = std::fs::read_to_string(&file).expect("readable source");
            for (line, code) in production_lines(&text) {
                if code.contains(needle) {
                    let shown = file.strip_prefix(root).unwrap_or(&file).display();
                    found.push(format!("{shown}:{line}"));
                }
            }
        }
    }
    found
}

#[test]
fn each_shared_results_path_is_built_in_exactly_one_place() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let root = root.canonicalize().expect("the repository root");
    for (needle, owner) in OWNED {
        let found = constructions(&root, needle);
        assert_eq!(found.len(), 1, "{needle} is built at {found:?}");
        assert!(
            found[0].starts_with(owner),
            "{needle} is built at {found:?}"
        );
    }
}

/// The reader on its edges: a production spelling the old needle missed, a
/// test module behind a multi-line attribute, and code after it.
#[test]
fn the_reader_skips_test_modules_but_not_a_production_spelling() {
    let text = "fn a() { Path::new(\"results/runs.bin\"); }\n#[cfg(test)]\n#[allow(\n    x,\n)]\n\
                mod tests {\n    const P: &str = \"/f/runs.bin\";\n}\nfn b() {}\n\
                #[cfg(test)]\nmod t;\nconst Q: &str = \"x/population-write.lock\"; // y\n";
    let hits = |needle: &str| -> Vec<usize> {
        production_lines(text)
            .into_iter()
            .filter(|(_, code)| code.contains(needle))
            .map(|(n, _)| n)
            .collect()
    };
    assert_eq!(hits(OWNED[1].0), vec![1]);
    assert_eq!(hits(OWNED[0].0), vec![12]);
    assert!(production_lines(text).iter().any(|(n, _)| *n == 9));
}

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

/// Each shared file name, as its one construction spells it, and the file that
/// holds that construction.
const OWNED: [(&str, &str); 2] = [
    (
        ".join(\"population-write.lock\")",
        "crates/cli/src/population.rs",
    ),
    (".join(\"runs.bin\")", "crates/cli/src/results.rs"),
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

/// Each `(file, line)` outside a `//` comment that holds `needle`.
fn constructions(root: &Path, needle: &str) -> Vec<String> {
    let mut found = Vec::new();
    let crates = root.join("crates");
    let mut members: Vec<PathBuf> = std::fs::read_dir(&crates)
        .expect("crates/")
        .map(|e| e.expect("an entry").path().join("src"))
        .filter(|src| src.is_dir())
        .collect();
    members.sort();
    for src in members {
        let mut files = Vec::new();
        sources(&src, &mut files);
        for file in files {
            let text = std::fs::read_to_string(&file).expect("readable source");
            for (index, line) in text.lines().enumerate() {
                let code = line.split("//").next().unwrap_or(line);
                if code.contains(needle) {
                    let shown = file.strip_prefix(root).unwrap_or(&file).display();
                    found.push(format!("{shown}:{}", index + 1));
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

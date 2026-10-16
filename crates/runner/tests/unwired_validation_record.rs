//! D-1544's record of five validation primitives with no production caller,
//! checked against the source it describes (audit-20261003 gaps-3).
//!
//! Benjamini-Hochberg, the anchored walk-forward bottom-half rate, the V3
//! projected walk-forward door and the two admission projections are built
//! and tested, and no `cli` verb or `api` route reaches them. A reader of the
//! runner API would otherwise take FDR control and those doors to be
//! available. Each one's own documentation now says so, and this test fails
//! the moment either half moves: the sentence goes missing, or a `cli` or
//! `api` source starts naming the function, at which point the sentence is
//! false and must go.

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

fn read(path: &Path) -> String {
    std::fs::read_to_string(path)
        .unwrap_or_else(|why| panic!("{} is readable: {why}", path.display()))
}

/// Every `.rs` file under `dir` that is not a test file.
fn production_sources(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = std::fs::read_dir(dir).unwrap_or_else(|why| panic!("{}: {why}", dir.display()));
    for entry in entries {
        let path = entry.expect("a directory entry").path();
        if path.is_dir() {
            if path.file_name().is_some_and(|name| name != "tests") {
                production_sources(&path, out);
            }
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default();
            if !name.ends_with("_tests.rs") && name != "tests.rs" {
                out.push(path);
            }
        }
    }
}

/// The doc block directly above `signature` in `file`, as one string.
fn doc_above(file: &str, signature: &str) -> String {
    let at = file
        .find(signature)
        .unwrap_or_else(|| panic!("{signature} exists"));
    let before = &file[..at];
    let mut lines: Vec<&str> = before
        .lines()
        .rev()
        .skip_while(|line| line.trim().is_empty())
        .take_while(|line| {
            let line = line.trim_start();
            line.starts_with("///") || line.starts_with("#[")
        })
        .collect();
    lines.reverse();
    lines.join("\n")
}

#[test]
fn every_unwired_validation_primitive_says_so_and_has_no_cli_or_api_caller() {
    let unwired = [
        (
            "crates/runner/src/significance.rs",
            "pub fn benjamini_hochberg(",
            "benjamini_hochberg",
        ),
        (
            "crates/runner/src/pbo.rs",
            "pub fn anchored_walk_forward_bottom_half_rate_v1(",
            "anchored_walk_forward_bottom_half_rate_v1",
        ),
        (
            "crates/runner/src/validate.rs",
            "pub fn walk_forward_projected_prepared_anchored_search_v3(",
            "walk_forward_projected_prepared_anchored_search_v3",
        ),
        (
            "crates/runner/src/admission.rs",
            "pub fn evaluate_v2_projection(",
            "evaluate_v2_projection",
        ),
        (
            "crates/runner/src/admission.rs",
            "pub fn evaluate_v3_projection(",
            "evaluate_v3_projection",
        ),
    ];
    let mut callers = Vec::new();
    for crate_dir in ["crates/cli/src", "crates/api/src"] {
        production_sources(&repo().join(crate_dir), &mut callers);
    }
    assert!(callers.len() > 10, "premise: the caller crates were read");
    let callers: Vec<(PathBuf, String)> = callers
        .into_iter()
        .map(|path| {
            // Everything from the first test-only item on is a test, by this
            // workspace's convention of a trailing `#[cfg(test)] mod tests`.
            let text = read(&path);
            let production = text
                .split_once("\n#[cfg(test)]")
                .map_or(text.as_str(), |(head, _)| head)
                .to_owned();
            (path, production)
        })
        .collect();
    for (home, signature, name) in unwired {
        let doc = doc_above(&read(&repo().join(home)), signature);
        assert!(
            doc.contains("**No production caller (D-1544).**"),
            "{name}'s doc no longer states it is unwired:\n{doc}"
        );
        for (path, text) in &callers {
            assert!(
                !text.contains(name),
                "{} names {name}: it is wired now, so D-1544's sentence in its doc is \
                 false and must be replaced",
                path.display()
            );
        }
    }
}

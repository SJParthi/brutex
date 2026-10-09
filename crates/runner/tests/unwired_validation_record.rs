//! D-1544's record of validation primitives with no production caller,
//! checked against the source it describes (audit-20261003 gaps-3).
//!
//! Benjamini-Hochberg, the V3 projected walk-forward door and the two
//! admission projections are built and tested, and no `cli` verb or `api`
//! route reaches them. The anchored walk-forward bottom-half rate was the
//! fifth until D-1724 wired it into the stored audit; this scan could not see
//! that, because it read each file only up to its first `#[cfg(test)]`, and
//! now asserts the wiring its doc names instead (G3-3, D-4737). A reader of the
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

/// `source` with every `#[cfg(test)]` item removed: the attribute run, then
/// the item it covers, which ends at its own `;` or at the first later line
/// closing its brace at the same indentation (`cargo fmt --check` makes that
/// exact). Everything else is kept, so a test module declared near the top of a
/// file does not hide the production code below it (G3-3, D-4737).
fn release_text(source: &str) -> String {
    let lines: Vec<&str> = source.lines().collect();
    let mut kept = String::with_capacity(source.len());
    let mut at = 0_usize;
    while let Some(line) = lines.get(at) {
        let trimmed = line.trim_start();
        if !(trimmed.starts_with("#[cfg(test") || trimmed.starts_with("#[cfg(all(test")) {
            kept.push_str(line);
            kept.push('\n');
            at += 1;
            continue;
        }
        let mut item = at + 1;
        while let Some(next) = lines.get(item) {
            let next = next.trim_start();
            if next.starts_with("#[") {
                while lines
                    .get(item)
                    .is_some_and(|line| !line.trim_end().ends_with(']'))
                {
                    item += 1;
                }
                item += 1;
            } else if next.starts_with("//") {
                item += 1;
            } else {
                break;
            }
        }
        let Some(head) = lines.get(item) else { break };
        at = item + 1;
        if head.trim_end().ends_with('{') {
            let indent = &head[..head.len() - head.trim_start().len()];
            let close = format!("{indent}}}");
            while let Some(body) = lines.get(at) {
                at += 1;
                let body = body.trim_end();
                if body == close || body == format!("{close};") {
                    break;
                }
            }
        }
    }
    kept
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
            // Only the test items are dropped. Cutting at the first
            // `#[cfg(test)]` kept 56 lines of `cli/src/lib.rs`, whose first
            // test module is declared on line 57, so the scan saw almost
            // nothing of the file that calls the most (G3-3, D-4737).
            let production = release_text(&read(&path));
            (path, production)
        })
        .collect();
    let lib = callers
        .iter()
        .find(|(path, _)| path.ends_with("crates/cli/src/lib.rs"))
        .map(|(_, text)| text.lines().count());
    assert!(
        lib.is_some_and(|lines| lines > 10_000),
        "premise: cli/src/lib.rs is read past its first test module ({lib:?} lines)"
    );
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

    // The one D-1724 wired: its doc names the caller, and the caller exists in
    // the release text and makes the call.
    let doc = doc_above(
        &read(&repo().join("crates/runner/src/pbo.rs")),
        "pub fn anchored_walk_forward_bottom_half_rate_v1(",
    );
    assert!(
        doc.contains("**Production caller: `cli`'s `overfitting_of` (D-1724).**")
            && !doc.contains("No production caller"),
        "the wired bottom-half rate's doc must name its caller:\n{doc}"
    );
    let lib = callers
        .iter()
        .find(|(path, _)| path.ends_with("crates/cli/src/lib.rs"))
        .map(|(_, text)| text.as_str())
        .unwrap_or_default();
    let body = lib
        .split_once("fn overfitting_of(")
        .map(|(_, rest)| rest.split_once("\nfn ").map_or(rest, |(body, _)| body))
        .expect("cli::overfitting_of exists in release code");
    assert!(
        body.contains("runner::pbo::anchored_walk_forward_bottom_half_rate_v1("),
        "overfitting_of no longer calls the exact bottom-half rate"
    );
}

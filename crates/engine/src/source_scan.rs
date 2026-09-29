//! Test-only reading of the workspace's own source, for the guards that pin
//! "no production path calls this" to the tree rather than to a sentence.
//!
//! The rule the scan applies is deliberately conservative in the direction
//! that fails loudly:
//!
//! - a file under `crates/engine/src` is cut at its first line that is exactly
//!   the test attribute, and only the region before that line is read.
//!   `nothing_after_an_engine_files_first_test_gate_ships` pins that every
//!   top-level item after that line carries the attribute itself, so no
//!   shipping item can hide behind the cut;
//! - a file under any OTHER crate's `src` is read whole, tests included. A
//!   mention there, even in a test, fails the guard, because no other crate
//!   names these items today and one that starts to is the day the doc must be
//!   read again.
//!
//! Line comments (`//`, which includes `///` and `//!`) are stripped before
//! counting, so a doc link is not a call.

use std::path::{Path, PathBuf};

/// The exact line that opens a test-only item.
const TEST_GATE: &str = "#[cfg(test)]";

/// Every `.rs` file under `crates/*/src`, sorted, with its crate directory name.
pub(crate) fn sources() -> Vec<(String, PathBuf)> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push(path);
            }
        }
    }
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut found = Vec::new();
    for member in std::fs::read_dir(&crates).into_iter().flatten().flatten() {
        let name = member.file_name().to_string_lossy().into_owned();
        let mut files = Vec::new();
        walk(&member.path().join("src"), &mut files);
        found.extend(files.into_iter().map(|f| (name.clone(), f)));
    }
    found.sort();
    found
}

/// The code of `text` with every line comment removed.
fn code_of(text: &str) -> String {
    text.lines()
        .map(|l| l.split_once("//").map_or(l, |(code, _)| code))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The region of `text` the scan reads for a file of crate `krate`.
fn region(krate: &str, text: &str) -> String {
    if krate == "engine" {
        text.lines()
            .take_while(|l| *l != TEST_GATE)
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        text.to_owned()
    }
}

/// Every file whose scanned region names any of `patterns` more often than it
/// names `definition`, as `path (count)`.
///
/// `definition` is subtracted once per occurrence so the item's own `fn` line
/// is not counted as a call of itself; an empty `definition` subtracts nothing.
pub(crate) fn mentions(patterns: &[&str], definition: &str) -> Vec<String> {
    let files = sources();
    assert!(
        files.len() > 13,
        "the walk found only {} source files under crates/*/src, so it did not read \
         the workspace",
        files.len()
    );
    let mut hits = Vec::new();
    for (krate, file) in &files {
        let text = std::fs::read_to_string(file).unwrap_or_default();
        let code = code_of(&region(krate, &text));
        let named: usize = patterns.iter().map(|p| code.matches(p).count()).sum();
        let defined = if definition.is_empty() {
            0
        } else {
            code.matches(definition).count()
        };
        let calls = named.saturating_sub(defined);
        if calls > 0 {
            hits.push(format!("{} ({calls})", file.display()));
        }
    }
    hits
}

#[cfg(test)]
mod tests {
    use super::{TEST_GATE, code_of, mentions, region, sources};

    /// The cut at the first test gate is only sound if nothing that ships
    /// follows it, so in every engine source file each top-level item after that
    /// line must carry the gate in its own attribute block.
    #[test]
    fn nothing_after_an_engine_files_first_test_gate_ships() {
        let mut checked = 0_u32;
        for (krate, file) in sources() {
            if krate != "engine" {
                continue;
            }
            let text = std::fs::read_to_string(&file).unwrap_or_default();
            let lines: Vec<&str> = text.lines().collect();
            let Some(first) = lines.iter().position(|l| *l == TEST_GATE) else {
                continue;
            };
            for (at, line) in lines.iter().enumerate().skip(first) {
                let starts_item = !line.is_empty()
                    && !line.starts_with(char::is_whitespace)
                    && !["#", "/", "}", ")", "]"]
                        .iter()
                        .any(|p| line.starts_with(p));
                if !starts_item {
                    continue;
                }
                let gated = lines
                    .get(..at)
                    .unwrap_or_default()
                    .iter()
                    .rev()
                    .take_while(|l| {
                        !l.is_empty()
                            && (l.starts_with(char::is_whitespace)
                                || !(l.ends_with(';') || l.ends_with('}')))
                    })
                    .any(|l| *l == TEST_GATE);
                assert!(
                    gated,
                    "{}:{}: `{line}` follows the first test gate without one of its \
                     own, so the scan's cut would hide shipping code",
                    file.display(),
                    at + 1
                );
                checked = checked.saturating_add(1);
            }
        }
        assert!(checked >= 4, "only {checked} gated items were checked");
    }

    /// A comment is not a call, an engine test region is not read, another crate
    /// is read whole, and an empty definition subtracts nothing.
    #[test]
    fn the_scan_reads_code_not_comments_and_cuts_only_the_engine() {
        assert_eq!(code_of("a(); // b()\n/// c()\nd()"), "a(); \n\nd()");
        let text = "x();\n#[cfg(test)]\nmod tests { y(); }";
        assert_eq!(region("engine", text), "x();");
        assert_eq!(region("runner", text), text);
        assert!(
            mentions(&["this_pattern_is_spelled_only_in_a_test_region("], "").is_empty(),
            "a pattern spelled only in a test region is found nowhere"
        );
        let shipped = mentions(&["fn mentions("], "");
        assert_eq!(
            shipped.len(),
            1,
            "this file's own shipping region names `fn mentions(` once, and an empty \
             definition must not cancel it: {shipped:?}"
        );
        assert!(
            mentions(&["fn mentions("], "fn mentions(").is_empty(),
            "a definition cancels itself"
        );
    }
}

#![allow(
    clippy::expect_used,
    reason = "the same exception every test module in this workspace takes -- a \
              test that cannot panic cannot fail."
)]

//! GUARDS THAT PIN "NO PRODUCTION PATH CALLS THIS" TO THE TREE, NOT TO A SENTENCE.
//!
//! # Why this is an integration test and not a unit test
//!
//! The guards read the workspace's own source files, and CI Gate 22 clause B
//! refuses any filesystem call site in `crates/engine/src` or
//! `crates/engine/benches`, test-only or not. `tests/` is outside that scan, so
//! the reading lives here and the engine's `src` keeps no filesystem capability.
//!
//! # The rule the scan applies
//!
//! It is deliberately conservative in the direction that fails loudly:
//!
//! - a file under `crates/engine/src` is cut at its first line that is exactly
//!   the test attribute, and only the region before that line is read.
//!   `nothing_after_an_engine_files_first_test_gate_ships` pins that every
//!   top-level item after that line carries the attribute itself, so no
//!   shipping item can hide behind the cut;
//! - a file under any OTHER crate's `src` is read whole, tests included. A
//!   mention there, even in a test, fails the guard;
//! - line comments (`//`, which includes `///` and `//!`) are stripped, so a
//!   doc link is not a call;
//! - a name counts only as a whole identifier token, so `Best` is not found
//!   inside `BestRow` and is found after `keep::`, after `use`, or bare.

use std::path::{Path, PathBuf};

/// The exact line that opens a test-only item.
const TEST_GATE: &str = "#[cfg(test)]";

/// One source file: its crate directory name, its path, and its text.
struct Source {
    krate: String,
    path: PathBuf,
    text: String,
}

/// Every `.rs` file under `dir`, at any depth, into `out`, in the order the
/// host lists them. A path that cannot be listed adds nothing.
fn rust_files_under(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            rust_files_under(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// Every `.rs` file under `crates/*/src`, sorted by path, read.
fn sources() -> Vec<Source> {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut found = Vec::new();
    for member in std::fs::read_dir(&crates).into_iter().flatten().flatten() {
        let krate = member.file_name().to_string_lossy().into_owned();
        let mut files = Vec::new();
        rust_files_under(&member.path().join("src"), &mut files);
        found.extend(files.into_iter().map(|path| {
            let text = std::fs::read_to_string(&path)
                .map_err(|e| format!("{}: {e}", path.display()))
                .expect("every source file under crates/*/src reads");
            Source {
                krate: krate.clone(),
                path,
                text,
            }
        }));
    }
    found.sort_by(|a, b| a.path.cmp(&b.path));
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

/// How many times `token` occurs in `code` as a whole identifier.
fn tokens(code: &str, token: &str) -> usize {
    let ident = |c: char| c.is_alphanumeric() || c == '_';
    code.match_indices(token)
        .filter(|(at, _)| {
            let before = code.get(..*at).and_then(|s| s.chars().next_back());
            let after = code.get(at + token.len()..).and_then(|s| s.chars().next());
            !before.is_some_and(ident) && !after.is_some_and(ident)
        })
        .count()
}

/// Every file of `files` whose scanned region names `token` more often than it
/// spells the `definitions`, as `path (count)`.
///
/// Each occurrence of each definition is subtracted once, so an item's own `fn`,
/// `struct` or `impl` line is not counted as a use of itself; an empty list
/// subtracts nothing.
fn mentions_in(files: &[Source], token: &str, definitions: &[&str]) -> Vec<String> {
    let mut hits = Vec::new();
    for file in files {
        let code = code_of(&region(&file.krate, &file.text));
        let named = tokens(&code, token);
        let defined: usize = definitions.iter().map(|d| code.matches(d).count()).sum();
        let calls = named.saturating_sub(defined);
        if calls > 0 {
            hits.push(format!("{} ({calls})", file.path.display()));
        }
    }
    hits
}

/// [`mentions_in`] over the real workspace.
fn mentions(token: &str, definitions: &[&str]) -> Vec<String> {
    let files = sources();
    let engine = files.iter().filter(|f| f.krate == "engine").count();
    let others = files.iter().filter(|f| f.krate != "engine").count();
    assert!(
        engine > 1 && others > 1,
        "the walk found {engine} engine and {others} other source files under \
         crates/*/src, so it did not read the workspace"
    );
    mentions_in(&files, token, definitions)
}

/// The text between the first `from` and the next `to` after it, or empty.
fn between<'a>(text: &'a str, from: &str, to: &str) -> &'a str {
    text.split_once(from)
        .and_then(|(_, tail)| tail.split_once(to))
        .map_or("", |(doc, _)| doc)
}

fn source(krate: &str, path: &str, text: &str) -> Source {
    Source {
        krate: krate.to_owned(),
        path: PathBuf::from(path),
        text: text.to_owned(),
    }
}

/// The cut at the first test gate is only sound if nothing that ships follows
/// it, so in every engine source file each top-level item after that line must
/// carry the gate in its own attribute block.
#[test]
fn nothing_after_an_engine_files_first_test_gate_ships() {
    let mut checked = 0_u32;
    for file in sources() {
        if file.krate != "engine" {
            continue;
        }
        let (ungated, gated) = ungated_after_the_first_gate(&file.text);
        assert!(
            ungated.is_empty(),
            "{}: these follow the first test gate without one of their own, so the \
             scan's cut would hide shipping code: {ungated:?}",
            file.path.display()
        );
        checked = checked.saturating_add(gated);
    }
    assert!(checked >= 4, "only {checked} gated items were checked");
}

/// Every top-level item after `text`'s first test gate that does not carry
/// the gate in its own attribute block, as (line from 1, the line); and how
/// many items after it do. A function of the text, so the refusal is checked
/// on text that breaks the rule (BASE-COV-LIBS-LS4).
fn ungated_after_the_first_gate(text: &str) -> (Vec<(usize, String)>, u32) {
    let mut ungated = Vec::new();
    let mut checked = 0_u32;
    let lines: Vec<&str> = text.lines().collect();
    let Some(first) = lines.iter().position(|l| *l == TEST_GATE) else {
        return (ungated, checked);
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
        if gated {
            checked = checked.saturating_add(1);
        } else {
            ungated.push((at + 1, (*line).to_owned()));
        }
    }
    (ungated, checked)
}

/// A comment is not a call, an engine test region is not read, another crate is
/// read whole, a token must be whole, a definition cancels itself once per
/// occurrence, an empty list cancels nothing, and every listed definition is
/// subtracted.
#[test]
fn the_scan_reads_code_not_comments_and_cuts_only_the_engine() {
    assert_eq!(code_of("a(); // b()\n/// c()\nd()"), "a(); \n\nd()");
    let text = "x();\n#[cfg(test)]\nmod tests { y(); }";
    assert_eq!(region("engine", text), "x();");
    assert_eq!(region("runner", text), text);

    assert_eq!(
        tokens("Best BestRow keep::Best OurBest Best_ (Best)", "Best"),
        3
    );

    let files = [
        source(
            "engine",
            "e/src/a.rs",
            "fn f() {}\n#[cfg(test)]\nmod t { g(); }",
        ),
        source(
            "runner",
            "r/src/b.rs",
            "fn h() {}\n#[cfg(test)]\nmod t { g(); }",
        ),
        source("engine", "e/src/c.rs", "fn g() {}\nfn k() { g(); } // g()"),
    ];
    assert_eq!(
        mentions_in(&files, "g", &["fn g"]),
        ["r/src/b.rs (1)", "e/src/c.rs (1)"].map(String::from),
        "the engine test region is cut, another crate's test region is read, the \
         definition cancels itself once and the comment is not a call"
    );
    assert_eq!(
        mentions_in(&files, "g", &[]),
        ["r/src/b.rs (1)", "e/src/c.rs (2)"].map(String::from),
        "an empty list cancels nothing"
    );
    assert_eq!(
        mentions_in(&files, "g", &["fn g", "k() { g"]),
        ["r/src/b.rs (1)"].map(String::from),
        "every listed definition is subtracted"
    );
}

/// **The fingerprinted path has no production caller, and its doc says so**
/// (ET-masks-evaluation-sweep-11, D-0760).
///
/// `Column::support_fingerprinted` and `HitSet` are exercised by the column's
/// tests and by the C-E-07 bench row only. D-0454 described the fingerprint as if
/// a run relied on it, which no run did. This reads the engine's shipping source
/// and every other crate's whole `src`, requires that none of it names the method
/// outside its own definition, and requires the method's doc to say so. The day
/// a production caller appears, this fails and the doc sentence must be
/// revisited rather than left standing.
#[test]
fn the_fingerprinted_path_has_no_production_caller_and_says_so() {
    let calls = mentions("support_fingerprinted", &["fn support_fingerprinted"]);
    assert!(
        calls.is_empty(),
        "support_fingerprinted now has a production caller: {calls:?}. Revisit its \
         doc and D-0760 before keeping this test's claim"
    );
    let doc = between(
        include_str!("../src/column.rs"),
        "/// The support count AND the identity of the bars it counted",
        "pub fn support_fingerprinted",
    );
    assert!(
        doc.contains("/// **No production path calls this.**"),
        "the doc must state that no production path calls it"
    );
}

/// **`keep::Best` has no production caller, and its doc says so**
/// (ET-masks-evaluation-sweep-13, D-0762).
///
/// Every use of `Best` outside `keep.rs` is in the engine's test module, and
/// `runner::rank` keeps its own heap. This reads the engine's shipping source
/// and every other crate's whole `src`, requires that none of it names `Best` as
/// a token except the type's own `pub struct Best` and `impl Best` lines, and
/// requires the type's doc to say no production path calls it.
#[test]
fn best_has_no_production_caller_and_says_so() {
    let uses = mentions("Best", &["pub struct Best", "impl Best"]);
    assert!(
        uses.is_empty(),
        "keep::Best now has a production caller: {uses:?}. Revisit its doc and \
         D-0762 before keeping this test's claim"
    );
    let doc = between(
        include_str!("../src/keep.rs"),
        "/// The best `cap` itemsets offered, in memory proportional to `cap`.",
        "pub struct Best {",
    );
    assert!(
        doc.contains("/// **No production path calls this.**"),
        "the doc must state that no production path constructs `Best`"
    );
}

/// BASE-COV-LIBS-LS4: **AN ITEM AFTER THE FIRST GATE WITHOUT ONE OF ITS OWN IS
/// NAMED**, whether a blank line or the previous item's closing line stands
/// between it and the gate; one whose attribute block holds the gate, over
/// several lines or under a doc comment, is checked and passes.
#[test]
fn an_item_after_the_first_gate_without_its_own_is_named() {
    let text = "fn shipped() {}\n\
                #[cfg(test)]\n\
                mod tests {\n\
                }\n\
                #[cfg(test)]\n\
                \n\
                fn after_a_blank() {}\n\
                #[cfg(test)]\n\
                /// A doc line.\n\
                fn gated() {}\n\
                const AFTER_A_BRACE: u8 = 1;\n\
                #[cfg(test)]\n\
                use std::fmt;\n\
                fn after_a_use() {}\n\
                #[cfg(test)]\n\
                #[allow(\n    clippy::all,\n)]\n\
                fn gated_over_lines() {}\n";
    let (ungated, checked) = ungated_after_the_first_gate(text);
    assert_eq!(
        ungated,
        [
            (7, "fn after_a_blank() {}".to_owned()),
            (11, "const AFTER_A_BRACE: u8 = 1;".to_owned()),
            (14, "fn after_a_use() {}".to_owned()),
        ]
    );
    assert_eq!(
        checked, 4,
        "mod tests, gated, the gated use and gated_over_lines"
    );
    // COMBINED: no gate at all checks nothing and names nothing.
    assert_eq!(
        ungated_after_the_first_gate(
            text.split_once("#[cfg(test)]")
                .expect("the text holds a gate")
                .0
        ),
        (Vec::new(), 0)
    );
    // OUT OF THE BOX: the gate on the last line.
    assert_eq!(
        ungated_after_the_first_gate("fn a() {}\n#[cfg(test)]"),
        (Vec::new(), 0)
    );
}

/// BASE-COV-LIBS-LS4: the source walk reads Rust at every depth, passes over
/// what is not Rust, and a path that cannot be listed adds nothing.
#[test]
fn the_source_walk_reads_rust_at_every_depth_and_nothing_else() {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("production-callers-walk-{}", std::process::id()));
    let _stale = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("deep/deeper")).expect("a scratch tree");
    for file in ["top.rs", "notes.txt", "deep/deeper/low.rs", "deep/README"] {
        std::fs::write(root.join(file), "").expect("a scratch file");
    }
    let mut found = Vec::new();
    rust_files_under(&root, &mut found);
    found.sort();
    assert_eq!(
        found,
        [root.join("deep/deeper/low.rs"), root.join("top.rs")]
    );
    let mut none = Vec::new();
    rust_files_under(&root.join("no-such-folder"), &mut none);
    assert!(none.is_empty());
    std::fs::remove_dir_all(&root).expect("the scratch tree is removed");
}

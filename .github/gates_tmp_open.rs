//! Gate 28 — one way to create a temporary file (R3-16; rules audit RA-287,
//! RA-290). A CI-only executable with no dependencies, built directly by
//! rustc and proved by its own `rustc --test` run before the step trusts it.
//!
//! # The hole this keeps shut
//!
//! A writer that opens its temporary "create, truncate" opens whatever is
//! already at that name: a second hard link of another file is cut to zero
//! and written through. The sort index's second check lost a bar file that
//! way, the verdict store had the same open (RA-290), and the ingest design
//! adds more temporaries (RA-287). `store::staging::create` is the one door
//! that opens nothing already there. This gate fails a crate file that names
//! a temporary and does not create it through that door.
//!
//! # The rule, and what it is not
//!
//! For every tracked `.rs` file under `crates/` that is not test code, it
//! reads TOKENS (comments skipped, an item under `#[cfg(test)]` skipped) and
//! counts two things: the string literals that name a temporary (`"tmp"`, as
//! `with_extension("tmp")` spells one, or any text holding `.tmp`), and the
//! calls `staging::create`. A file passes when it names no temporary, when
//! it calls the door at least once per temporary it names, or when it is in
//! [`LEGACY`] with exactly the count written there.
//!
//! It COUNTS, it does not trace: a name spelt once and a call made once pass
//! together even when the call creates another file. What it guarantees is
//! that no file gains a temporary without gaining a call of the door, and
//! that the list of writers still opening their own may only shrink: a
//! [`LEGACY`] count that is higher than the file's is refused too, so a
//! converted writer is struck from the list in the change that converts it.
//!
//! # Usage
//!
//! `gates_tmp_open check` from the repository root: lists `git ls-files -z
//! -- crates` (gate 0's spawn rule, D-2344, lets a `.github` tool start git
//! and no other program) and prints one line per file it read a temporary
//! in, then the verdict. Exit 0 is a pass, 1 a refusal, 2 a misuse.

#![forbid(unsafe_code)]

use std::process::{Command, ExitCode, Stdio};

/// Every writer that still opens its temporaries its own way, with the
/// count of temporary names the gate reads in it, and what it opens. Each is
/// converted to `store::staging::create` by the package that next changes
/// it, which lowers its count here in the same change; the gate refuses a
/// count that grows and a count that is higher than the file's. Named for
/// `docs/06-limits.md` by R3-16.
const LEGACY: [(&str, usize, &str); 5] = [
    (
        "crates/cli/src/live.rs",
        1,
        "`<hex>.<pid>.tmp` by `File::create`: create and truncate, the RA-290 open",
    ),
    (
        "crates/cli/src/search_checkpoint.rs",
        1,
        "`complete.tmp` by `File::create_new`: refuses a leftover rather than clearing it",
    ),
    (
        "crates/pull/src/alerts.rs",
        1,
        "`alerts.<pid>.tmp` by its own `create_fresh`, which removes any name in its way \
         (R3-21, R4-05 own the file)",
    ),
    (
        "crates/pull/src/census_store.rs",
        2,
        "`<vendor>.man.seal.tmp` and `SEALED.tmp` by `create_new`, and the epoch temporary \
         by `File::create`: create and truncate, the RA-290 open (R3-19 owns the file)",
    ),
    (
        "crates/pull/src/manifest.rs",
        1,
        "`EPOCH_TMP_SUFFIX`, the epoch temporary's name that census_store opens \
         (R3-19 owns the file)",
    ),
];

// ------------------------------------------------------------- tokens --

/// One token of Rust source, as far as this gate needs to tell them apart.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Tok {
    /// An identifier or a keyword (a raw `r#name` loses its `r#`).
    Ident(String),
    /// A string literal's text between its quotes, raw or not, unescaped
    /// only as far as a backslash keeps the next character in it.
    Str(String),
    /// One punctuation character.
    Punct(char),
    /// A character literal, a lifetime or a number: never a name.
    Other,
}

/// The tokens of `src`, with every comment dropped.
fn lex(src: &str) -> Vec<Tok> {
    let chars: Vec<char> = src.chars().collect();
    let at = |i: usize| chars.get(i).copied();
    let mut out = Vec::new();
    let mut i = 0;
    while let Some(c) = at(i) {
        if c.is_whitespace() {
            i += 1;
        } else if c == '/' && at(i + 1) == Some('/') {
            while at(i).is_some_and(|c| c != '\n') {
                i += 1;
            }
        } else if c == '/' && at(i + 1) == Some('*') {
            i = skip_block_comment(&chars, i);
        } else if let Some((text, next)) = string_at(&chars, i) {
            out.push(Tok::Str(text));
            i = next;
        } else if c == '\'' {
            i = skip_quote(&chars, i);
            out.push(Tok::Other);
        } else if c.is_alphabetic() || c == '_' {
            let start = if c == 'r' && at(i + 1) == Some('#') {
                i + 2
            } else {
                i
            };
            let mut end = start;
            while at(end).is_some_and(|c| c.is_alphanumeric() || c == '_') {
                end += 1;
            }
            out.push(Tok::Ident(chars[start..end].iter().collect()));
            i = end.max(i + 1);
        } else if c.is_ascii_digit() {
            while at(i).is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '.') {
                i += 1;
            }
            out.push(Tok::Other);
        } else {
            out.push(Tok::Punct(c));
            i += 1;
        }
    }
    out
}

/// The index after the block comment opening at `i`; comments nest.
fn skip_block_comment(chars: &[char], mut i: usize) -> usize {
    let mut depth = 0usize;
    while i < chars.len() {
        if chars.get(i) == Some(&'/') && chars.get(i + 1) == Some(&'*') {
            depth += 1;
            i += 2;
        } else if chars.get(i) == Some(&'*') && chars.get(i + 1) == Some(&'/') {
            depth -= 1;
            i += 2;
            if depth == 0 {
                return i;
            }
        } else {
            i += 1;
        }
    }
    i
}

/// A string literal starting at `i` (`"`, `b"`, `c"`, `r"`, `r#"`, `br#"`,
/// `cr#"`), its text and the index after it; `None` when none starts there.
fn string_at(chars: &[char], i: usize) -> Option<(String, usize)> {
    let mut j = i;
    if matches!(chars.get(j), Some('b' | 'c')) {
        j += 1;
    }
    let raw = chars.get(j) == Some(&'r');
    if raw {
        j += 1;
    }
    let mut hashes = 0usize;
    while raw && chars.get(j) == Some(&'#') {
        hashes += 1;
        j += 1;
    }
    if chars.get(j) != Some(&'"') {
        return None;
    }
    // A prefix glued to an identifier (`abr"..."` cannot occur, but `xr`
    // can end a name): only a prefix that starts the token counts.
    if i > 0
        && j > i
        && chars
            .get(i - 1)
            .is_some_and(|c| c.is_alphanumeric() || *c == '_')
    {
        return None;
    }
    j += 1;
    let mut text = String::new();
    while let Some(&c) = chars.get(j) {
        if c == '"' {
            let closes = (0..hashes).all(|k| chars.get(j + 1 + k) == Some(&'#'));
            if closes {
                return Some((text, j + 1 + hashes));
            }
        }
        if c == '\\'
            && !raw
            && let Some(&next) = chars.get(j + 1)
        {
            text.push(c);
            text.push(next);
            j += 2;
            continue;
        }
        text.push(c);
        j += 1;
    }
    Some((text, j))
}

/// The index after a character literal or a lifetime starting at `i`.
fn skip_quote(chars: &[char], i: usize) -> usize {
    // `'\n'`, `'\''`, `'\u{..}'`: an escape is always a literal.
    if chars.get(i + 1) == Some(&'\\') {
        let mut j = i + 2;
        while j < chars.len() && chars.get(j) != Some(&'\'') {
            j += 1;
        }
        return j + 1;
    }
    // `'x'` is a literal; `'a` followed by anything else is a lifetime.
    if chars.get(i + 2) == Some(&'\'') {
        return i + 3;
    }
    let mut j = i + 1;
    while chars
        .get(j)
        .is_some_and(|c| c.is_alphanumeric() || *c == '_')
    {
        j += 1;
    }
    j.max(i + 1)
}

/// Whether the attribute tokens `attr` (inside `#[...]`) are a `cfg` that
/// holds only under test: `cfg(test)`, `cfg(all(test, ..))`, and never one
/// that says `not`.
fn is_test_cfg(attr: &[Tok]) -> bool {
    attr.first() == Some(&Tok::Ident("cfg".to_owned()))
        && attr.contains(&Tok::Ident("test".to_owned()))
        && !attr.contains(&Tok::Ident("not".to_owned()))
}

/// The index after the group opening at `i` (`(`, `[` or `{`), balanced.
fn skip_group(toks: &[Tok], i: usize) -> usize {
    let mut depth = 0usize;
    let mut j = i;
    while let Some(tok) = toks.get(j) {
        match tok {
            Tok::Punct('(' | '[' | '{') => depth += 1,
            Tok::Punct(')' | ']' | '}') => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return j + 1;
                }
            }
            _ => {}
        }
        j += 1;
    }
    j
}

/// `toks` with every item under a test-only `cfg` removed, or nothing when
/// the file opens with `#![cfg(test)]`.
fn without_test_items(toks: &[Tok]) -> Vec<Tok> {
    let mut out = Vec::new();
    let mut i = 0;
    while let Some(tok) = toks.get(i) {
        let inner = toks.get(i + 1) == Some(&Tok::Punct('!'));
        let open = if inner { i + 2 } else { i + 1 };
        if *tok == Tok::Punct('#') && toks.get(open) == Some(&Tok::Punct('[')) {
            let end = skip_group(toks, open);
            let attr = toks.get(open + 1..end.saturating_sub(1)).unwrap_or(&[]);
            if is_test_cfg(attr) {
                if inner {
                    return Vec::new();
                }
                i = skip_item(toks, end);
                continue;
            }
            out.extend_from_slice(toks.get(i..end).unwrap_or(&[]));
            i = end;
            continue;
        }
        out.push(tok.clone());
        i += 1;
    }
    out
}

/// The index after the item starting at `i`: its further attributes, then
/// everything to its first `;` or through its first balanced `{...}`.
fn skip_item(toks: &[Tok], mut i: usize) -> usize {
    while toks.get(i) == Some(&Tok::Punct('#')) && toks.get(i + 1) == Some(&Tok::Punct('[')) {
        i = skip_group(toks, i + 1);
    }
    while let Some(tok) = toks.get(i) {
        match tok {
            Tok::Punct(';') => return i + 1,
            Tok::Punct('{') => return skip_group(toks, i),
            Tok::Punct('(' | '[') => i = skip_group(toks, i),
            _ => i += 1,
        }
    }
    i
}

// ------------------------------------------------------------ verdict --

/// Whether a string literal's text names a temporary.
fn names_a_temporary(text: &str) -> bool {
    text == "tmp" || text.contains(".tmp")
}

/// What one file holds: temporary names, and calls of the door.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Counts {
    temporaries: usize,
    door: usize,
}

/// The counts of `src`, test code and comments excluded.
fn counts(src: &str) -> Counts {
    let toks = without_test_items(&lex(src));
    let mut out = Counts::default();
    for (i, tok) in toks.iter().enumerate() {
        match tok {
            Tok::Str(text) if names_a_temporary(text) => out.temporaries += 1,
            Tok::Ident(name) if name == "staging" => {
                let path = (toks.get(i + 1), toks.get(i + 2), toks.get(i + 3));
                if path
                    == (
                        Some(&Tok::Punct(':')),
                        Some(&Tok::Punct(':')),
                        Some(&Tok::Ident("create".to_owned())),
                    )
                {
                    out.door += 1;
                }
            }
            _ => {}
        }
    }
    out
}

/// Whether `path` is test code by where it lies.
fn is_test_path(path: &str) -> bool {
    path.ends_with("_tests.rs")
        || path.ends_with("/tests.rs")
        || path.contains("/tests/")
        || path.contains("/benches/")
}

/// The gate's verdict over `files` (path, source): its report lines, and
/// whether it refused.
fn verdict(files: &[(String, String)], legacy: &[(&str, usize, &str)]) -> (Vec<String>, bool) {
    let mut lines = Vec::new();
    let mut refused = false;
    let mut seen: Vec<&str> = Vec::new();
    for (path, src) in files {
        if !path.ends_with(".rs") || is_test_path(path) {
            continue;
        }
        let held = counts(src);
        let listed = legacy.iter().find(|(at, _, _)| at == path);
        if let Some((at, allowed, what)) = listed {
            seen.push(at);
            if held.temporaries > *allowed {
                refused = true;
                lines.push(format!(
                    "REFUSED {path}: {} temporary name(s), {allowed} allowed while it opens \
                     its own ({what}): a NEW temporary is created by store::staging::create",
                    held.temporaries
                ));
            } else if held.temporaries < *allowed {
                refused = true;
                lines.push(format!(
                    "REFUSED {path}: {} temporary name(s) where LEGACY says {allowed}: lower \
                     its count (or strike it) in .github/gates_tmp_open.rs, so the list only \
                     shrinks",
                    held.temporaries
                ));
            } else {
                lines.push(format!(
                    "legacy  {path}: {allowed} temporary name(s), its own open ({what})"
                ));
            }
            continue;
        }
        if held.temporaries == 0 {
            continue;
        }
        if held.door >= held.temporaries {
            lines.push(format!(
                "ok      {path}: {} temporary name(s), {} call(s) of store::staging::create",
                held.temporaries, held.door
            ));
        } else {
            refused = true;
            lines.push(format!(
                "REFUSED {path}: {} temporary name(s) and {} call(s) of store::staging::create: \
                 a temporary opened any other way can be a hard link of another file, cut to \
                 zero and written through (RA-290)",
                held.temporaries, held.door
            ));
        }
    }
    for (at, _, _) in legacy {
        if !seen.contains(at) {
            refused = true;
            lines.push(format!(
                "REFUSED {at}: listed in LEGACY and not read (no such tracked file): strike it"
            ));
        }
    }
    (lines, refused)
}

// --------------------------------------------------------------- main --

/// The tracked files under `crates/`, by `git ls-files -z`.
fn tracked() -> Result<Vec<String>, String> {
    let out = Command::new("git")
        .args(["ls-files", "-z", "--", "crates"])
        .stderr(Stdio::inherit())
        .output()
        .map_err(|e| format!("git ls-files: {e}"))?;
    if !out.status.success() {
        return Err(format!("git ls-files failed: {}", out.status));
    }
    Ok(String::from_utf8_lossy(&out.stdout)
        .split('\0')
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect())
}

fn check() -> Result<bool, String> {
    let paths = tracked()?;
    if paths.is_empty() {
        return Err("GATE 28 READ NO FILE under crates/".to_owned());
    }
    let mut files = Vec::new();
    for path in paths.into_iter().filter(|p| p.ends_with(".rs")) {
        let src = std::fs::read_to_string(&path).map_err(|e| format!("{path}: {e}"))?;
        files.push((path, src));
    }
    let (lines, refused) = verdict(&files, &LEGACY);
    println!("gate 28: {} Rust file(s) read under crates/", files.len());
    for line in lines {
        println!("  {line}");
    }
    Ok(refused)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("check") if args.len() == 1 => match check() {
            Ok(false) => {
                println!("OK — every temporary is born by store::staging::create or listed");
                ExitCode::SUCCESS
            }
            Ok(true) => {
                println!("GATE 28 REFUSED — see the lines marked REFUSED");
                ExitCode::from(1)
            }
            Err(why) => {
                eprintln!("{why}");
                ExitCode::from(1)
            }
        },
        _ => {
            eprintln!("usage: gates_tmp_open check");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(path: &str, src: &str) -> (String, String) {
        (path.to_owned(), src.to_owned())
    }

    #[test]
    fn a_comment_or_a_doc_line_names_no_temporary() {
        let src = "// a.tmp\n/// b.tmp\n/* c.tmp /* nested d.tmp */ e.tmp */\n//! f.tmp\nfn x() {}";
        assert_eq!(counts(src), Counts::default());
    }

    #[test]
    fn every_string_form_is_read_and_a_char_or_lifetime_is_not_one() {
        let src = r####"
            fn a<'x>(p: &'x str) -> char {
                let _ = "a.tmp";
                let _ = r#"b.tmp"#;
                let _ = br##"c "quoted" .tmp"##;
                let _ = b"d.tmp";
                let _ = p.with_extension("tmp");
                let _ = "escaped \" quote, then .tmp";
                let _ = "not a temporary: tmpfs, /tmp/x";
                '"'
            }
        "####;
        assert_eq!(counts(src).temporaries, 6, "{:?}", lex(src));
    }

    #[test]
    fn test_items_are_skipped_and_a_not_test_item_is_read() {
        let src = r#"
            #[cfg(test)]
            mod tests { fn t() { let _ = "a.tmp"; } }
            #[cfg(test)]
            #[path = "x_tests.rs"]
            mod more;
            #[cfg(all(test, unix))]
            fn helper() { let _ = "b.tmp"; }
            #[cfg(not(test))]
            fn real() { let _ = "c.tmp"; }
            #[derive(Debug)]
            struct S;
            fn after() { let _ = "d.tmp"; }
        "#;
        assert_eq!(counts(src).temporaries, 2);
        assert_eq!(
            counts("#![cfg(test)]\nfn t() { let _ = \"a.tmp\"; }").temporaries,
            0
        );
    }

    #[test]
    fn the_door_is_counted_by_its_path_only() {
        let src = "use crate::staging::{self, StagingError};\n\
                   fn w(p: &Path) { let _ = staging::create(&tmp(p, \".tmp\")); }\n\
                   fn other() { create(\"x\"); staging::Staged; }";
        assert_eq!(
            counts(src),
            Counts {
                temporaries: 1,
                door: 1
            }
        );
    }

    /// The rule, one case per row: a new temporary without the door is
    /// refused, with it passes; a LEGACY file that grows or shrinks is
    /// refused, one that holds its count passes; a LEGACY row with no file
    /// is refused; test files are not read.
    #[test]
    fn the_verdict_refuses_a_new_temporary_and_holds_the_list_to_shrinking() {
        let legacy = [("crates/a/src/old.rs", 1usize, "its own")];
        let old = file("crates/a/src/old.rs", "fn o() { let _ = \"x.tmp\"; }");
        type Case = (Vec<(String, String)>, bool, &'static str);
        let cases: [Case; 7] = [
            (
                vec![
                    old.clone(),
                    file("crates/a/src/new.rs", "fn n() { let _ = \"y.tmp\"; }"),
                ],
                true,
                "REFUSED crates/a/src/new.rs: 1 temporary name(s) and 0 call(s)",
            ),
            (
                vec![
                    old.clone(),
                    file(
                        "crates/a/src/new.rs",
                        "fn n() { staging::create(&p.with_extension(\"tmp\")); }",
                    ),
                ],
                false,
                "ok      crates/a/src/new.rs",
            ),
            (
                vec![file(
                    "crates/a/src/old.rs",
                    "fn o() { let _ = \"x.tmp\"; let _ = \"y.tmp\"; }",
                )],
                true,
                "2 temporary name(s), 1 allowed",
            ),
            (
                vec![file("crates/a/src/old.rs", "fn o() {}")],
                true,
                "lower its count",
            ),
            (vec![old.clone()], false, "legacy  crates/a/src/old.rs"),
            (
                vec![file("crates/a/src/other.rs", "fn o() {}")],
                true,
                "not read (no such tracked file)",
            ),
            (
                vec![
                    old,
                    file("crates/a/src/x_tests.rs", "fn t() { let _ = \"t.tmp\"; }"),
                    file("crates/a/tests/it.rs", "fn t() { let _ = \"t.tmp\"; }"),
                    file("crates/a/benches/b.rs", "fn t() { let _ = \"t.tmp\"; }"),
                ],
                false,
                "legacy",
            ),
        ];
        for (files, refuses, says) in cases {
            let (lines, refused) = verdict(&files, &legacy);
            assert_eq!(refused, refuses, "{lines:?}");
            assert!(lines.iter().any(|l| l.contains(says)), "{says}: {lines:?}");
        }
    }

    #[test]
    fn the_legacy_list_names_each_file_once() {
        for (i, (at, count, what)) in LEGACY.iter().enumerate() {
            assert!(at.starts_with("crates/") && at.ends_with(".rs"), "{at}");
            assert!(*count > 0 && !what.is_empty(), "{at}");
            assert!(
                LEGACY.iter().skip(i + 1).all(|(other, _, _)| other != at),
                "{at} twice"
            );
        }
    }
}

//! **No backend code starts another program at run time, except the two the
//! allowlist below names.** D-5240, D-5243.
//!
//! # Why this exists
//!
//! `CLAUDE.md` §2 makes Rust the only language outside `web/`, and gate 2 holds
//! the build-script half of that rule: no `build.rs` here invokes an external
//! process. Nothing held the same line at RUN time. A scan of `feat/db-fix` at
//! `4756cce0` (16 Oct 2026) found every tracked file conforming and two places
//! in shipped code that started an outside program anyway:
//! `crates/api/src/disk_route.rs` ran `/bin/df -Pk` for `/disk.json`'s free
//! space, and `crates/api/src/server.rs` opens the operator's browser at start.
//! The first is now one `statvfs(2)` call (D-5240). The second is the front end
//! being launched, never a build or data step, and is an entry in [`ALLOWED`],
//! with its reason. When this gate met `feat/db-fix` again on 17 Oct 2026 it
//! failed on a third, `crates/cli/src/watchdog_command.rs` `fn notify`, which
//! R3-32 (D-4996) added to hand the watchdog's notice page to `open`: the same
//! kind of start, a page shown in the operator's browser, and the second entry
//! (D-5243).
//!
//! # What it reads
//!
//! Every `.rs` file under `crates/` (a `target` directory skipped), read from
//! the disk rather than from `include_str!`, so a new file is scanned the day it
//! appears. Each file is lexed: comments, string literals, raw strings, byte and
//! C strings, char literals and lifetimes are recognised, so prose and text that
//! merely spell the path neither trip the scan nor hide code after them. A file
//! the lexer cannot read (an unclosed comment or string) is a failure, not a
//! pass.
//!
//! A **process spawn** is any naming, in code, of the type the `process` modules
//! of `std` and `tokio` construct a child with: the path `process::<type>` with
//! any spacing or a raw identifier, a `process::{..}` import group holding it,
//! and the two spellings that would hide it from those two, a glob import
//! `process::*` and a renamed module `process as ..`. A construction needs the
//! type in scope, so naming it is the earliest point the scan can refuse; a mere
//! mention in a signature is refused too, which errs towards failing.
//!
//! # How it decides what is test code
//!
//! Test code is what only the test harness compiles, and the scan follows
//! `rustc`'s own rule for that rather than a file-name convention:
//!
//! * **an integration test**: any file under `crates/<crate>/tests/`;
//! * **an item behind a test attribute**: an item, statement, field, arm or
//!   argument that carries `#[test]`, or `#[cfg(P)]` where `P` holds only
//!   under `test` — `test` itself, `all(..)` with such a member, or `any(..)`
//!   all of whose members are such. `not(test)`, `any(test, X)` and every other
//!   predicate are shipped code. The item ends at its own `;`, at the brace that
//!   closes its body, or (for what is not an item) at its `,`; it never extends
//!   past a bracket that closes something it is inside, so a gated field cannot
//!   swallow the field after it;
//! * **a module file a test-only declaration names**: a `mod name;` behind such
//!   an attribute, or inside a test-only inline module, names its file by
//!   `rustc`'s path rules (`#[path]` relative to the declaring file's directory;
//!   otherwise `name.rs` or `name/mod.rs` under the module's directory, a
//!   non-`mod.rs` file adding its own stem, an inline module its name). Every
//!   module that file declares is test code too, to a fixed point. A test-only
//!   declaration whose file is not there is a failure;
//! * **a file whose first attributes include `#![cfg(test)]`**.
//!
//! `*_tests.rs` is exempt by name only BECAUSE it is reached by that rule: a
//! file named like a test outside `tests/` that no test-only declaration names
//! is compiled into the build, so the gate fails on its name as well as on any
//! spawn in it. `benches/`, `examples/`, `build.rs` and `src/bin/` are not test
//! code and are scanned whole.
//!
//! # What it cannot see
//!
//! Said plainly, because a gate that oversells itself is worse than none:
//!
//! * **A dependency's own code.** A crate in `Cargo.lock` that spawns at run
//!   time is not under `crates/`. `crates/vocab/tests/workspace_is_rust.rs`
//!   pins the dependency set, so a new one is reviewed, but this scan does not
//!   read it.
//! * **A path assembled out of reach of the lexer**: a `use std as s;` crate
//!   alias followed by `s::process::..` is still caught, but a re-export from a
//!   dependency under another name, code a procedural macro generates, or a file
//!   `include!`d from outside `crates/` is not.
//! * **A foreign call** (`posix_spawn` through FFI). Gate 16 makes every crate
//!   root `#![forbid(unsafe_code)]`, which is what closes that door; this scan
//!   does not repeat it.

#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail"
)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};

/// The module the spawning type lives in, and the type. Assembled, so this
/// file's own text spells neither path and a scan of it would find nothing.
const MODULE: &str = concat!("proc", "ess");
const TYPE: &str = concat!("Comm", "and");

/// One function allowed to start a program, how many namings of the spawning
/// type it holds, and why.
struct Allowed {
    path: &'static str,
    function: &'static str,
    count: usize,
    reason: &'static str,
}

/// **The whole allowlist.** Two entries, both handing a page to the operator's
/// own browser through the platform's URL handler: the server's opener at
/// start, and the watchdog's notice that the app is down or back (D-4996,
/// R3-32, admitted by D-5243 when the two branches met). Anything else that
/// names the spawning type outside test code fails, and so does an entry
/// whose function stops naming it exactly `count` times.
const ALLOWED: &[Allowed] = &[
    Allowed {
        path: "crates/api/src/server.rs",
        function: "open_unless_suppressed",
        count: 1,
        reason: "opens the operator's own browser, front-end launch, never a build or data step",
    },
    Allowed {
        path: "crates/cli/src/watchdog_command.rs",
        function: "notify",
        count: 1,
        reason: "opens the watchdog's notice page in the operator's own browser, never a build \
                 or data step (D-4996)",
    },
];

// ------------------------------------------------------------------ lexer --

#[derive(Debug, Clone, PartialEq, Eq)]
enum Tok {
    /// An identifier; `raw` for an `r#` spelling, which is never a keyword.
    Ident {
        name: String,
        raw: bool,
    },
    Punct(char),
    /// A string literal's value, escapes decoded as far as this scan needs.
    Str(String),
    /// A number, a char literal or a lifetime: nothing this scan reads.
    Other,
}

#[derive(Debug, Clone)]
struct Token {
    tok: Tok,
    line: usize,
}

fn is_ident_start(c: char) -> bool {
    c == '_' || c.is_alphabetic()
}

fn is_ident_continue(c: char) -> bool {
    c == '_' || c.is_alphanumeric()
}

/// The tokens of `src`, comments dropped.
///
/// # Errors
///
/// A sentence naming the line of a comment, string or char that never closes.
fn lex(src: &str) -> Result<Vec<Token>, String> {
    let c: Vec<char> = src.chars().collect();
    let at = |k: usize| c.get(k).copied();
    let mut out = Vec::new();
    let mut i = 0;
    let mut line = 1;
    while let Some(ch) = at(i) {
        let start = line;
        if ch == '\n' {
            line += 1;
            i += 1;
        } else if ch.is_whitespace() {
            i += 1;
        } else if ch == '/' && at(i + 1) == Some('/') {
            while at(i).is_some_and(|x| x != '\n') {
                i += 1;
            }
        } else if ch == '/' && at(i + 1) == Some('*') {
            i = block_comment(&c, i, &mut line)
                .ok_or_else(|| format!("line {start}: a block comment never closes"))?;
        } else if ch == '"' {
            let (value, next) = cooked(&c, i + 1, &mut line)
                .ok_or_else(|| format!("line {start}: a string never closes"))?;
            out.push(Token {
                tok: Tok::Str(value),
                line: start,
            });
            i = next;
        } else if ch == '\'' {
            i = quote(&c, i).ok_or_else(|| format!("line {start}: a char literal never closes"))?;
            out.push(Token {
                tok: Tok::Other,
                line: start,
            });
        } else if ch.is_ascii_digit() {
            while at(i).is_some_and(|x| x == '_' || x.is_ascii_alphanumeric())
                || (at(i) == Some('.') && at(i + 1).is_some_and(|x| x.is_ascii_digit()))
            {
                i += 1;
            }
            out.push(Token {
                tok: Tok::Other,
                line,
            });
        } else if is_ident_start(ch) {
            let (tok, next) = word(&c, i, &mut line)
                .ok_or_else(|| format!("line {start}: a prefixed literal never closes"))?;
            out.push(Token { tok, line: start });
            i = next;
        } else {
            out.push(Token {
                tok: Tok::Punct(ch),
                line,
            });
            i += 1;
        }
    }
    Ok(out)
}

/// Past the block comment opening at `at`, nested comments included.
fn block_comment(c: &[char], at: usize, line: &mut usize) -> Option<usize> {
    let mut i = at;
    let mut depth = 0_usize;
    loop {
        match (c.get(i)?, c.get(i + 1)) {
            ('/', Some('*')) => {
                depth += 1;
                i += 2;
            }
            ('*', Some('/')) => {
                depth -= 1;
                i += 2;
                if depth == 0 {
                    return Some(i);
                }
            }
            ('\n', _) => {
                *line += 1;
                i += 1;
            }
            _ => i += 1,
        }
    }
}

/// The word starting at `at`: an identifier, a raw identifier, or the string,
/// raw string or byte literal its prefix (`b`, `c`, `r`, `br`, `cr`) opens.
fn word(c: &[char], at: usize, line: &mut usize) -> Option<(Tok, usize)> {
    let get = |k: usize| c.get(k).copied();
    let mut i = at;
    while get(i).is_some_and(is_ident_continue) {
        i += 1;
    }
    let word: String = c[at..i].iter().collect();
    let hashes = c.iter().skip(i).take_while(|&&x| x == '#').count();
    Some(match (word.as_str(), get(i)) {
        ("b" | "c", Some('"')) => {
            let (value, next) = cooked(c, i + 1, line)?;
            (Tok::Str(value), next)
        }
        ("r" | "br" | "cr", Some('"' | '#')) if get(i + hashes) == Some('"') => {
            let (value, next) = raw(c, i + hashes + 1, hashes, line)?;
            (Tok::Str(value), next)
        }
        ("r", Some('#')) if hashes == 1 && get(i + 1).is_some_and(is_ident_start) => {
            let mut end = i + 1;
            while get(end).is_some_and(is_ident_continue) {
                end += 1;
            }
            let name: String = c[i + 1..end].iter().collect();
            (Tok::Ident { name, raw: true }, end)
        }
        ("b", Some('\'')) => (Tok::Other, quote(c, i)?),
        _ => (
            Tok::Ident {
                name: word,
                raw: false,
            },
            i,
        ),
    })
}

/// A non-raw string body from `from` (just past its opening quote): its value
/// and the index just past its closing quote.
fn cooked(c: &[char], from: usize, line: &mut usize) -> Option<(String, usize)> {
    let mut value = String::new();
    let mut i = from;
    loop {
        match *c.get(i)? {
            '"' => return Some((value, i + 1)),
            '\\' => {
                let escaped = *c.get(i + 1)?;
                match escaped {
                    'n' => value.push('\n'),
                    't' => value.push('\t'),
                    '\n' => *line += 1,
                    other => value.push(other),
                }
                i += 2;
            }
            ch => {
                if ch == '\n' {
                    *line += 1;
                }
                value.push(ch);
                i += 1;
            }
        }
    }
}

/// A raw string body from `from` (just past its opening quote), closed by a
/// quote and `hashes` hashes.
fn raw(c: &[char], from: usize, hashes: usize, line: &mut usize) -> Option<(String, usize)> {
    let mut i = from;
    loop {
        let ch = *c.get(i)?;
        if ch == '"' && (1..=hashes).all(|k| c.get(i + k) == Some(&'#')) {
            return Some((c[from..i].iter().collect(), i + 1 + hashes));
        }
        if ch == '\n' {
            *line += 1;
        }
        i += 1;
    }
}

/// From the quote at `at`: past a char literal (`'x'`, `'\n'`, `'\u{..}'`), or
/// past a lifetime or label (`'a`), whichever it is.
fn quote(c: &[char], at: usize) -> Option<usize> {
    let q = c.iter().skip(at).position(|&x| x == '\'')? + at;
    match (c.get(q + 1), c.get(q + 2)) {
        (Some('\\'), _) => {
            let close = c.iter().skip(q + 3).position(|&x| x == '\'')? + q + 3;
            Some(close + 1)
        }
        (Some(_), Some('\'')) => Some(q + 3),
        _ => {
            let mut i = q + 1;
            while c.get(i).copied().is_some_and(is_ident_continue) {
                i += 1;
            }
            Some(i)
        }
    }
}

// ---------------------------------------------------------- token helpers --

fn ident(tok: Option<&Token>) -> Option<&str> {
    match &tok?.tok {
        Tok::Ident { name, .. } => Some(name),
        _ => None,
    }
}

fn keyword(t: &[Token], i: usize, word: &str) -> bool {
    matches!(t.get(i).map(|x| &x.tok), Some(Tok::Ident { name, raw: false }) if name == word)
}

fn punct(t: &[Token], i: usize, ch: char) -> bool {
    matches!(t.get(i).map(|x| &x.tok), Some(Tok::Punct(p)) if *p == ch)
}

fn path_sep(t: &[Token], i: usize) -> bool {
    punct(t, i, ':') && punct(t, i + 1, ':')
}

/// The index of the bracket closing the one at `open`, or `None` when it never
/// closes.
fn matching(t: &[Token], open: usize) -> Option<usize> {
    let mut depth = 0_usize;
    for (k, tok) in t.iter().enumerate().skip(open) {
        match tok.tok {
            Tok::Punct('(' | '[' | '{') => depth += 1,
            Tok::Punct(')' | ']' | '}') => {
                depth -= 1;
                if depth == 0 {
                    return Some(k);
                }
            }
            _ => {}
        }
    }
    None
}

// ----------------------------------------------------- attributes and cfg --

/// Whether a `cfg` predicate holds only under `test`, and where it ends.
fn predicate(t: &[Token], i: usize) -> (bool, usize) {
    let Some(name) = ident(t.get(i)) else {
        return (false, i + 1);
    };
    if punct(t, i + 1, '=') {
        return (false, i + 3);
    }
    if !punct(t, i + 1, '(') {
        return (name == "test", i + 1);
    }
    let mut members = Vec::new();
    let mut k = i + 2;
    while k < t.len() && !punct(t, k, ')') {
        let (holds, next) = predicate(t, k);
        members.push(holds);
        k = next;
        if punct(t, k, ',') {
            k += 1;
        }
    }
    let holds = match name {
        "all" => members.iter().any(|&m| m),
        "any" => !members.is_empty() && members.iter().all(|&m| m),
        _ => false,
    };
    (holds, k + 1)
}

/// Whether the attribute body `attr` (between its brackets) makes what it is
/// on test code: `test`, or `cfg(P)` with `P` holding only under `test`.
fn attr_is_test(attr: &[Token]) -> bool {
    if attr.len() == 1 && keyword(attr, 0, "test") {
        return true;
    }
    keyword(attr, 0, "cfg") && punct(attr, 1, '(') && predicate(attr, 2).0
}

/// The value of a `path = ".."` attribute.
fn attr_path(attr: &[Token]) -> Option<String> {
    if !(keyword(attr, 0, "path") && punct(attr, 1, '=')) {
        return None;
    }
    match &attr.get(2)?.tok {
        Tok::Str(value) => Some(value.clone()),
        _ => None,
    }
}

/// The keywords an item can begin with, after its visibility.
const ITEM_KEYWORDS: &[&str] = &[
    "fn",
    "struct",
    "enum",
    "union",
    "trait",
    "type",
    "mod",
    "const",
    "static",
    "use",
    "impl",
    "extern",
    "unsafe",
    "async",
    "macro_rules",
    "auto",
];

/// Whether the tokens from `i` begin an item rather than a field, an arm, an
/// argument or an expression.
fn is_item(t: &[Token], i: usize) -> bool {
    let mut k = i;
    if keyword(t, k, "pub") {
        k += 1;
        if punct(t, k, '(') {
            k = matching(t, k).map_or(t.len(), |close| close + 1);
        }
    }
    ITEM_KEYWORDS.iter().any(|word| keyword(t, k, word))
}

/// Where the gated thing starting at `from` ends: just past its `;`, just past
/// the brace that closes its body, just past its `,` when it is not an item,
/// or just before a bracket that closes something it is inside.
fn item_end(t: &[Token], from: usize) -> usize {
    let item = is_item(t, from);
    let mut depth = 0_usize;
    let mut i = from;
    while let Some(tok) = t.get(i) {
        match tok.tok {
            Tok::Punct('(' | '[' | '{') => depth += 1,
            Tok::Punct(close @ (')' | ']' | '}')) => {
                if depth == 0 {
                    return i;
                }
                depth -= 1;
                if close == '}' && depth == 0 {
                    return i + 1;
                }
            }
            Tok::Punct(';') if depth == 0 => return i + 1,
            Tok::Punct(',') if depth == 0 && !item => return i + 1,
            _ => {}
        }
        i += 1;
    }
    i
}

// ------------------------------------------------------------- the needle --

/// Whether the tokens at `i` name the spawning type, and if so what that
/// spelling is and how many tokens it spans.
fn naming(t: &[Token], i: usize) -> Option<(&'static str, usize)> {
    if ident(t.get(i)) != Some(MODULE) {
        return None;
    }
    if keyword(t, i + 1, "as") {
        return Some((
            "renames the process module, which hides its spawns from this scan",
            2,
        ));
    }
    if !path_sep(t, i + 1) {
        return None;
    }
    if ident(t.get(i + 3)) == Some(TYPE) {
        return Some(("names the process-spawning type", 4));
    }
    if punct(t, i + 3, '*') {
        return Some((
            "glob-imports the process module, the spawning type with it",
            4,
        ));
    }
    if punct(t, i + 3, '{') {
        let close = matching(t, i + 3).unwrap_or(t.len());
        if t[i + 3..close].iter().any(|x| ident(Some(x)) == Some(TYPE)) {
            return Some(("imports the process-spawning type", close - i));
        }
    }
    None
}

// ----------------------------------------------------------- one file pass --

/// Where a file sits, as module resolution needs it.
struct Place {
    dir: PathBuf,
    /// A `lib.rs`, `main.rs`, `mod.rs` or crate root: its modules sit beside
    /// it, not in a directory named after it.
    mod_rs: bool,
    stem: String,
}

/// One `mod name;` declaration: the files it may name, and whether only the
/// test harness compiles it.
#[derive(Debug)]
struct Decl {
    line: usize,
    name: String,
    candidates: Vec<String>,
    test: bool,
}

/// One naming of the spawning type outside test code.
#[derive(Debug)]
struct Found {
    line: usize,
    function: Option<String>,
    what: &'static str,
}

#[derive(Debug, Default)]
struct FileScan {
    whole_file_test: bool,
    decls: Vec<Decl>,
    found: Vec<Found>,
    /// Namings inside test code, counted so a scan that sees nothing cannot
    /// pass for one that exempts everything.
    in_tests: usize,
}

enum Scope {
    Fn(String),
    Mod(String),
}

/// The files `mod name;` may name, by `rustc`'s rules.
fn candidates(place: &Place, inline: &[String], name: &str, path: Option<&str>) -> Vec<String> {
    let mut base = place.dir.clone();
    if path.is_none() || !inline.is_empty() {
        if !place.mod_rs {
            base.push(&place.stem);
        }
        base.extend(inline);
    }
    match path {
        Some(path) => vec![normal(&base.join(path))],
        None => vec![
            normal(&base.join(format!("{name}.rs"))),
            normal(&base.join(name).join("mod.rs")),
        ],
    }
}

/// `path` with `.` and `..` folded away, `/`-separated.
fn normal(path: &Path) -> String {
    let mut parts: Vec<String> = Vec::new();
    for part in path.components() {
        match part {
            Component::ParentDir => {
                parts.pop();
            }
            Component::Normal(name) => parts.push(name.to_string_lossy().into_owned()),
            _ => {}
        }
    }
    parts.join("/")
}

/// The attributes read so far before the next item.
#[derive(Default)]
struct Run {
    test: bool,
    path: Option<String>,
}

/// Where a pass stands: its bracket depth, the functions and inline modules
/// open around it, and the one whose body the next `{` opens.
struct Nesting<'a> {
    inline: &'a [String],
    depth: usize,
    scopes: Vec<(Scope, usize)>,
    pending: Option<Scope>,
}

impl Nesting<'_> {
    /// The inline modules around this point, outermost first.
    fn modules(&self) -> Vec<String> {
        self.inline
            .iter()
            .cloned()
            .chain(self.scopes.iter().filter_map(|(scope, _)| match scope {
                Scope::Mod(name) => Some(name.clone()),
                Scope::Fn(_) => None,
            }))
            .collect()
    }

    /// The innermost function around this point.
    fn function(&self) -> Option<String> {
        self.scopes.iter().rev().find_map(|(scope, _)| match scope {
            Scope::Fn(name) => Some(name.clone()),
            Scope::Mod(_) => None,
        })
    }

    /// One token that is none of the things a pass reads for itself.
    fn step(&mut self, tok: &Tok) {
        match tok {
            Tok::Punct('{') => {
                self.depth += 1;
                if let Some(scope) = self.pending.take() {
                    self.scopes.push((scope, self.depth));
                }
            }
            Tok::Punct('(' | '[') => self.depth += 1,
            Tok::Punct('}') => {
                if self.scopes.last().is_some_and(|(_, at)| *at == self.depth) {
                    self.scopes.pop();
                }
                self.depth = self.depth.saturating_sub(1);
            }
            Tok::Punct(')' | ']') => self.depth = self.depth.saturating_sub(1),
            Tok::Punct(';') => self.pending = None,
            _ => {}
        }
    }
}

/// The attribute at `i`: whether it is inner (`#![..]`), its body between the
/// brackets, and the index past it.
fn attribute(t: &[Token], i: usize) -> Option<(bool, &[Token], usize)> {
    if !punct(t, i, '#') {
        return None;
    }
    let inner = punct(t, i + 1, '!');
    let open = if inner { i + 2 } else { i + 1 };
    if !punct(t, open, '[') {
        return None;
    }
    let close = matching(t, open).unwrap_or(t.len());
    Some((inner, &t[open + 1..close], close + 1))
}

/// The file an `include!("..")` at `i` pastes.
fn included(t: &[Token], i: usize) -> Option<&str> {
    let pasted = keyword(t, i, "include")
        && punct(t, i + 1, '!')
        && punct(t, i + 2, '(')
        && punct(t, i + 4, ')');
    match &t.get(i + 3)?.tok {
        Tok::Str(file) if pasted => Some(file),
        _ => None,
    }
}

/// Reads `t`, a file or a test-only item (`test`), with the inline modules
/// around it in `inline`.
fn pass(t: &[Token], test: bool, inline: &[String], place: &Place, out: &mut FileScan) {
    let mut i = 0;
    let mut at = Nesting {
        inline,
        depth: 0,
        scopes: Vec::new(),
        pending: None,
    };
    let mut run = Run::default();
    while i < t.len() {
        if let Some((inner, attr, next)) = attribute(t, i) {
            if inner {
                out.whole_file_test |= at.depth == 0 && inline.is_empty() && attr_is_test(attr);
            } else {
                run.test |= attr_is_test(attr);
                run.path = attr_path(attr).or(run.path.take());
            }
            i = next;
            continue;
        }
        let attrs = std::mem::take(&mut run);
        let end = if attrs.test && !test {
            item_end(t, i)
        } else {
            i
        };
        if end > i {
            gated(&t[i..end], &at.modules(), attrs.path.as_deref(), place, out);
            i = end;
            continue;
        }
        // `include!` PASTES A FILE HERE, so the file is test code exactly when
        // this spot is. Its path is relative to this file's directory.
        if let Some(file) = included(t, i) {
            out.decls.push(Decl {
                line: t[i].line,
                name: format!("include!(\"{file}\")"),
                candidates: vec![normal(&place.dir.join(file))],
                test,
            });
            i += 5;
            continue;
        }
        let name = ident(t.get(i + 1)).map(str::to_owned);
        if keyword(t, i, "mod")
            && punct(t, i + 2, ';')
            && let Some(name) = &name
        {
            out.decls.push(Decl {
                line: t[i].line,
                candidates: candidates(place, &at.modules(), name, attrs.path.as_deref()),
                name: name.clone(),
                test,
            });
            i += 3;
            continue;
        }
        let opens = if keyword(t, i, "fn") {
            name.map(Scope::Fn)
        } else if keyword(t, i, "mod") && punct(t, i + 2, '{') {
            name.map(Scope::Mod)
        } else {
            None
        };
        if let Some(scope) = opens {
            at.pending = Some(scope);
            i += 2;
            continue;
        }
        if let Some((what, span)) = naming(t, i) {
            if test {
                out.in_tests += 1;
            } else {
                out.found.push(Found {
                    line: t[i].line,
                    function: at.function(),
                    what,
                });
            }
            i += span;
            continue;
        }
        at.step(&t[i].tok);
        i += 1;
    }
}

/// A test-only item, read as test code: a `mod name;` is a test-only
/// declaration, a `mod name { .. }` body is read inside its module, and any
/// other item is read where it stands, for the files it pastes and the
/// namings it holds.
fn gated(item: &[Token], inline: &[String], path: Option<&str>, place: &Place, out: &mut FileScan) {
    let mut k = 0;
    if keyword(item, k, "pub") {
        k += 1;
        if punct(item, k, '(') {
            k = matching(item, k).map_or(item.len(), |close| close + 1);
        }
    }
    let name = ident(item.get(k + 1)).filter(|_| keyword(item, k, "mod"));
    match name {
        Some(name) if punct(item, k + 2, ';') => out.decls.push(Decl {
            line: item[k].line,
            name: name.to_owned(),
            candidates: candidates(place, inline, name, path),
            test: true,
        }),
        Some(name) if punct(item, k + 2, '{') => {
            let close = matching(item, k + 2).unwrap_or(item.len());
            let mut nested = inline.to_vec();
            nested.push(name.to_owned());
            pass(&item[k + 3..close], true, &nested, place, out);
        }
        _ => pass(item, true, inline, place, out),
    }
}

/// Where `path` (`/`-separated, from the repository root) sits.
fn place_of(path: &str) -> Place {
    let parts: Vec<&str> = path.split('/').collect();
    let file = parts.last().copied().unwrap_or_default();
    let crate_root = matches!(
        parts.as_slice(),
        ["crates", _, "build.rs"]
            | ["crates", _, "tests" | "benches" | "examples", _]
            | ["crates", _, "src", "bin", _]
    );
    Place {
        dir: Path::new(path)
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_default(),
        mod_rs: crate_root || matches!(file, "lib.rs" | "main.rs" | "mod.rs"),
        stem: file.strip_suffix(".rs").unwrap_or(file).to_owned(),
    }
}

fn under_tests_dir(path: &str) -> bool {
    matches!(
        path.split('/').collect::<Vec<_>>().as_slice(),
        ["crates", _, "tests", ..]
    )
}

// ---------------------------------------------------------------- verdict --

/// Every failure of the tree `files` (path → text) against `allowed`, and how
/// many namings it found inside test code.
fn judge(files: &BTreeMap<String, String>, allowed: &[Allowed]) -> (Vec<String>, usize) {
    let mut failures = Vec::new();
    let mut scans: BTreeMap<&str, FileScan> = BTreeMap::new();
    for (path, text) in files {
        match lex(text) {
            Ok(tokens) => {
                let mut scan = FileScan::default();
                pass(&tokens, false, &[], &place_of(path), &mut scan);
                scans.insert(path, scan);
            }
            Err(why) => failures.push(format!("{path} cannot be read by this scan: {why}")),
        }
    }
    let first_existing = |decl: &Decl| {
        decl.candidates
            .iter()
            .find(|candidate| files.contains_key(candidate.as_str()))
            .cloned()
    };

    // THE TEST FILES: integration tests, whole-file `#![cfg(test)]`, and what
    // a test-only declaration names, closed over every declaration each of
    // them makes in turn — the first two included, which are expanded like
    // the rest rather than taken as already done.
    let mut test_files: BTreeSet<String> = BTreeSet::new();
    let mut queue: Vec<String> = scans
        .iter()
        .filter(|(path, scan)| under_tests_dir(path) || scan.whole_file_test)
        .map(|(path, _)| (*path).to_owned())
        .collect();
    for (path, scan) in &scans {
        for decl in scan.decls.iter().filter(|decl| decl.test) {
            match first_existing(decl) {
                Some(file) => queue.push(file),
                None => failures.push(format!(
                    "{path}:{} names `{}` as test code, and no file is at {}",
                    decl.line,
                    decl.name,
                    decl.candidates.join(" or ")
                )),
            }
        }
    }
    while let Some(file) = queue.pop() {
        if test_files.insert(file.clone())
            && let Some(scan) = scans.get(file.as_str())
        {
            queue.extend(scan.decls.iter().filter_map(first_existing));
        }
    }

    for path in files.keys() {
        let named_as_test = path.ends_with("_tests.rs");
        if named_as_test && !under_tests_dir(path) && !test_files.contains(path) {
            failures.push(format!(
                "{path} is named like a test file, but no #[cfg(test)] declaration reaches \
                 it, so the build compiles it as shipped code"
            ));
        }
    }

    let mut in_tests = 0;
    let mut by_function: BTreeMap<(&str, String), Vec<&Found>> = BTreeMap::new();
    for (path, scan) in &scans {
        in_tests += scan.in_tests;
        if test_files.contains(*path) {
            in_tests += scan.found.len();
            continue;
        }
        for found in &scan.found {
            let function = found.function.clone().unwrap_or_else(|| "-".to_owned());
            by_function.entry((path, function)).or_default().push(found);
        }
    }
    for ((path, function), found) in &by_function {
        let allows = allowed
            .iter()
            .find(|entry| entry.path == *path && entry.function == function);
        if allows.is_some_and(|entry| entry.count == found.len()) {
            continue;
        }
        let said = allows.map_or_else(
            || "is not allowed".to_owned(),
            |entry| format!("is allowed {} and holds {}", entry.count, found.len()),
        );
        for one in found {
            failures.push(format!(
                "{path}:{} in fn {function} {}: a process spawn outside test code {said}",
                one.line, one.what
            ));
        }
    }
    for entry in allowed {
        if !by_function.contains_key(&(entry.path, entry.function.to_owned())) {
            failures.push(format!(
                "the allowlist names fn {} in {}, which starts no program any more: remove \
                 the entry rather than leave it to excuse a later one",
                entry.function, entry.path
            ));
        }
    }
    (failures, in_tests)
}

// ------------------------------------------------------------ the real tree --

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every `.rs` file under `crates/`, keyed by its `/`-separated path from the
/// repository root. A `target` directory is skipped and a link is not followed.
fn workspace_sources() -> BTreeMap<String, String> {
    let root = root();
    let mut out = BTreeMap::new();
    let mut dirs = vec![root.join("crates")];
    while let Some(dir) = dirs.pop() {
        let listing = std::fs::read_dir(&dir)
            .unwrap_or_else(|why| panic!("{} cannot be listed: {why}", dir.display()));
        for entry in listing {
            let entry = entry.expect("a directory entry");
            let kind = entry.file_type().expect("a file type");
            let path = entry.path();
            if kind.is_dir() && entry.file_name() != "target" {
                dirs.push(path);
            } else if kind.is_file() && path.extension().is_some_and(|ext| ext == "rs") {
                let text = std::fs::read_to_string(&path)
                    .unwrap_or_else(|why| panic!("{} cannot be read: {why}", path.display()));
                let relative = path.strip_prefix(&root).expect("under the root");
                out.insert(normal(relative), text);
            }
        }
    }
    out
}

/// **THE GATE.** No shipped code under `crates/` names the process-spawning
/// type except in [`ALLOWED`]'s functions, each its own number of times.
#[test]
fn no_shipped_code_starts_a_program_outside_the_allowlist() {
    let files = workspace_sources();
    assert!(
        files.len() > 500,
        "only {} source files were found under crates/ — the walk stopped walking",
        files.len()
    );
    let (failures, in_tests) = judge(&files, ALLOWED);
    assert!(
        failures.is_empty(),
        "CLAUDE.md §2, at run time (D-5240): shipped code starts no outside program but the \
         ones ALLOWED names. Replace the spawn with a Rust call, or argue it into ALLOWED with \
         its reason and a docs/05-decisions.md entry:\n  {}",
        failures.join("\n  ")
    );
    // A SCAN THAT SEES NOTHING CANNOT PASS FOR ONE THAT EXEMPTS EVERYTHING:
    // test code in this tree starts `mkfifo`, `git` and its own binary.
    assert!(
        in_tests >= 20,
        "only {in_tests} namings were found in test code, where dozens exist — the scan is \
         not reading what it says it reads"
    );
}

/// The two entries are the two browser openers, each with the reason it is
/// allowed, each once, and each file still defines its function. A third
/// entry fails here before it can excuse anything (D-5243).
#[test]
fn the_allowlist_is_the_two_browser_openers_alone() {
    let entries: Vec<(&str, &str, usize, &str)> = ALLOWED
        .iter()
        .map(|entry| (entry.path, entry.function, entry.count, entry.reason))
        .collect();
    assert_eq!(
        entries,
        [
            (
                "crates/api/src/server.rs",
                "open_unless_suppressed",
                1,
                "opens the operator's own browser, front-end launch, never a build or data step",
            ),
            (
                "crates/cli/src/watchdog_command.rs",
                "notify",
                1,
                "opens the watchdog's notice page in the operator's own browser, never a build \
                 or data step (D-4996)",
            ),
        ]
    );
    for entry in ALLOWED {
        let text = std::fs::read_to_string(root().join(entry.path)).expect(entry.path);
        assert!(
            text.contains(&format!("fn {}(", entry.function)),
            "{} no longer defines fn {}",
            entry.path,
            entry.function
        );
    }
}

// ------------------------------------------------- the gate can fail: proof --

fn tree(files: &[(&str, &str)]) -> BTreeMap<String, String> {
    files
        .iter()
        .map(|(path, text)| ((*path).to_owned(), (*text).to_owned()))
        .collect()
}

/// The failures of one planted `crates/x/src/lib.rs`, with nothing allowed.
fn planted(src: &str) -> Vec<String> {
    judge(&tree(&[("crates/x/src/lib.rs", src)]), &[]).0
}

/// **A PLANTED SPAWN FAILS, BY FILE, LINE AND FUNCTION.** Each spelling the
/// scan claims to see is planted in shipped code and must be refused; each
/// place it claims to exempt must not be.
#[test]
fn a_planted_spawn_is_refused_by_file_line_and_function() {
    let spawn = format!("std::{MODULE}::{TYPE}::new(\"sh\")");
    let refused = [
        format!("pub fn run() {{\n    let _ = {spawn}.status();\n}}"),
        format!("fn run() {{ std :: {MODULE} :: {TYPE} :: new(\"sh\"); }}"),
        format!("fn run() {{ tokio::{MODULE}::{TYPE}::new(\"sh\"); }}"),
        format!("fn run() {{ std::{MODULE}::r#{TYPE}::new(\"sh\"); }}"),
        format!("use std::{MODULE}::{TYPE};\nfn run() {{ {TYPE}::new(\"sh\"); }}"),
        format!("use std::{MODULE}::{{Stdio, {TYPE}}};\nfn run() {{}}"),
        format!("use std::{MODULE}::*;\nfn run() {{}}"),
        format!("use std::{MODULE} as p;\nfn run() {{ p::{TYPE}::new(\"sh\"); }}"),
        // `any(test, ..)` and `not(test)` are compiled into the build.
        format!("#[cfg(any(test, feature = \"x\"))]\nfn run() {{ {spawn}; }}"),
        format!("#[cfg(not(test))]\nfn run() {{ {spawn}; }}"),
        // A quote in a char literal does not open a string that hides code.
        format!("fn run() {{ let q = '\"'; {spawn}; let r = '\"'; }}"),
        format!("fn run<'a>(x: &'a str) {{ let _ = x; {spawn}; }}"),
        // A gated field ends at its comma; the next field is shipped.
        format!("struct S {{ #[cfg(test)] a: u8, b: std::{MODULE}::{TYPE} }}\nfn run() {{}}"),
        // A macro's body is code.
        format!("macro_rules! go {{ () => {{ {spawn} }} }}\nfn run() {{}}"),
    ];
    for src in &refused {
        let failures = planted(src);
        assert!(
            failures
                .iter()
                .any(|f| f.starts_with("crates/x/src/lib.rs:") && f.contains("outside test code")),
            "a planted spawn was not refused:\n{src}\n{failures:?}"
        );
    }
    let first = planted(&refused[0]);
    assert_eq!(
        first,
        vec![format!(
            "crates/x/src/lib.rs:2 in fn run names the process-spawning type: a process spawn \
             outside test code is not allowed"
        )]
    );

    let exempt = [
        format!("#[cfg(test)]\nmod tests {{\n    fn t() {{ {spawn}; }}\n}}"),
        format!("#[cfg(test)]\n#[allow(dead_code)]\nmod tests {{ fn t() {{ {spawn}; }} }}"),
        format!("#[test]\nfn t() {{ {spawn}; }}"),
        format!("#[cfg(all(test, unix))]\nfn t() {{ {spawn}; }}"),
        format!("#[cfg(any(test, all(test, unix)))]\nfn t() {{ {spawn}; }}"),
        format!("#[cfg(test)]\nfn t<A, B>() {{ {spawn}; }}"),
        format!("#![cfg(test)]\nfn t() {{ {spawn}; }}"),
        format!("fn run() {{\n    #[cfg(test)]\n    {spawn};\n}}"),
        // Prose and text are not code.
        format!("// {spawn}\nfn run() {{}}"),
        format!("/* outer /* {spawn} */ still a comment */\nfn run() {{}}"),
        format!(
            "fn run() -> &'static str {{ \"{}\" }}",
            spawn.replace('"', "\\\"")
        ),
        format!("fn run() -> &'static str {{ r#\"{spawn}\"# }}"),
        // The modules' other members start nothing, and another type may
        // share the name.
        format!(
            "use std::{MODULE}::Stdio;\nuse std::os::unix::{MODULE}::{TYPE}Ext;\n\
             fn run() {{ sweeprun::{TYPE}::new(); std::{MODULE}::exit(0); }}"
        ),
    ];
    for src in &exempt {
        assert_eq!(planted(src), Vec::<String>::new(), "refused:\n{src}");
    }
    // And the exempt spawns were seen, not missed.
    let (_, seen) = judge(&tree(&[("crates/x/src/lib.rs", &exempt[0])]), &[]);
    assert_eq!(seen, 1);

    // A file the lexer cannot read is a failure, not a pass.
    for broken in [
        "/* never closed",
        "fn f() { \"never closed }",
        "fn f() { r#\"x\" }",
    ] {
        let failures = planted(broken);
        assert!(
            failures
                .iter()
                .any(|f| f.contains("cannot be read by this scan")),
            "{broken:?}: {failures:?}"
        );
    }
}

/// **TEST CODE IS WHAT ONLY THE TEST HARNESS COMPILES**, followed through
/// module files by `rustc`'s path rules, and nothing else is exempt.
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one table of every way a module file is reached, or is not"
)]
fn test_code_is_what_cfg_test_compiles_followed_through_module_files() {
    let spawn = format!("pub fn go() {{ std::{MODULE}::{TYPE}::new(\"x\"); }}");
    let quiet = |files: &[(&str, &str)]| judge(&tree(files), &[]).0;

    // A test-only declaration exempts its file, and every file that one
    // declares in turn.
    assert_eq!(
        quiet(&[
            ("crates/x/src/lib.rs", "#[cfg(test)]\nmod helper;"),
            ("crates/x/src/helper.rs", &format!("mod deeper;\n{spawn}")),
            ("crates/x/src/helper/deeper.rs", &spawn),
        ]),
        Vec::<String>::new()
    );
    // The same declaration without the attribute ships both.
    let shipped = quiet(&[
        ("crates/x/src/lib.rs", "mod helper;"),
        ("crates/x/src/helper.rs", &spawn),
    ]);
    assert_eq!(shipped.len(), 1, "{shipped:?}");
    assert!(shipped[0].starts_with("crates/x/src/helper.rs:1 in fn go"));

    // `#[path]` is relative to the declaring file's directory; a default path
    // from a non-`mod.rs` file adds its stem; an inline module adds its name.
    assert_eq!(
        quiet(&[
            (
                "crates/x/src/lib.rs",
                "mod b;\nmod outer { #[cfg(test)] mod inner; }"
            ),
            (
                "crates/x/src/b.rs",
                "#[cfg(test)]\n#[path = \"b_tests.rs\"]\nmod tests;\n#[cfg(test)] mod c;"
            ),
            ("crates/x/src/b_tests.rs", &spawn),
            ("crates/x/src/b/c.rs", &spawn),
            ("crates/x/src/outer/inner.rs", &spawn),
        ]),
        Vec::<String>::new()
    );
    // Inside a test-only inline module every declaration is test-only.
    assert_eq!(
        quiet(&[
            (
                "crates/x/src/lib.rs",
                "#[cfg(test)]\nmod tests {\n    mod fixtures;\n    #[path = \"../t_tests.rs\"] mod t;\n}"
            ),
            ("crates/x/src/tests/fixtures.rs", &spawn),
            ("crates/x/src/t_tests.rs", &spawn),
        ]),
        Vec::<String>::new()
    );
    // A file pasted by `include!` into test code is test code, and a file
    // that is test code by `#![cfg(test)]` exempts what it declares.
    assert_eq!(
        quiet(&[
            (
                "crates/x/src/lib.rs",
                "mod whole;\n#[cfg(test)]\nmod tests {\n    include!(\"pasted_tests.rs\");\n}"
            ),
            ("crates/x/src/pasted_tests.rs", &spawn),
            (
                "crates/x/src/whole.rs",
                &format!("#![cfg(test)]\nmod more;\n{spawn}")
            ),
            ("crates/x/src/whole/more.rs", &spawn),
        ]),
        Vec::<String>::new()
    );
    // The same `include!` in shipped code ships the file.
    let pasted = quiet(&[
        ("crates/x/src/lib.rs", "include!(\"pasted.rs\");"),
        ("crates/x/src/pasted.rs", &spawn),
    ]);
    assert_eq!(pasted.len(), 1, "{pasted:?}");
    // A crate root's modules sit beside it.
    assert_eq!(
        quiet(&[
            ("crates/x/build.rs", "#[cfg(test)] mod probe;"),
            ("crates/x/probe.rs", &spawn),
        ]),
        Vec::<String>::new()
    );

    // NAMED LIKE A TEST IS NOT A TEST: undeclared, it ships, and says so.
    let named = quiet(&[
        ("crates/x/src/lib.rs", ""),
        ("crates/x/src/x_tests.rs", &spawn),
    ]);
    assert!(
        named
            .iter()
            .any(|f| f.contains("x_tests.rs is named like a test file")),
        "{named:?}"
    );
    assert!(
        named
            .iter()
            .any(|f| f.starts_with("crates/x/src/x_tests.rs:1 in fn go"))
    );

    // A test-only declaration naming no file is refused by name.
    let missing = quiet(&[("crates/x/src/lib.rs", "#[cfg(test)]\nmod gone;")]);
    assert_eq!(
        missing,
        vec![
            "crates/x/src/lib.rs:2 names `gone` as test code, and no file is at \
             crates/x/src/gone.rs or crates/x/src/gone/mod.rs"
                .to_owned()
        ]
    );
    let missing = quiet(&[(
        "crates/x/src/lib.rs",
        "#[cfg(test)]\nmod tests { include!(\"gone_tests.rs\"); }",
    )]);
    assert_eq!(
        missing,
        vec![
            "crates/x/src/lib.rs:2 names `include!(\"gone_tests.rs\")` as test code, and no \
             file is at crates/x/src/gone_tests.rs"
                .to_owned()
        ]
    );

    // Integration tests are exempt; benches, examples and binaries are not.
    assert_eq!(
        quiet(&[("crates/x/tests/it.rs", &spawn)]),
        Vec::<String>::new()
    );
    for shipped in [
        "crates/x/benches/ratio.rs",
        "crates/x/examples/e.rs",
        "crates/x/src/bin/tool.rs",
        "crates/x/src/tests/stray.rs",
    ] {
        assert_eq!(quiet(&[(shipped, &spawn)]).len(), 1, "{shipped}");
    }
}

/// **THE ALLOWLIST IS EXACT**: its function, its file and its count, and an
/// entry that excuses nothing is itself a failure.
#[test]
fn the_allowlist_is_exact_about_file_function_and_count() {
    let allowed = [Allowed {
        path: "crates/x/src/lib.rs",
        function: "open",
        count: 1,
        reason: "a fixture",
    }];
    let spawn = format!("std::{MODULE}::{TYPE}::new(\"x\")");
    let verdict = |src: &str, path: &str| judge(&tree(&[(path, src)]), &allowed).0;
    let lib = "crates/x/src/lib.rs";

    assert_eq!(
        verdict(&format!("fn open() {{ {spawn}; }}"), lib),
        Vec::<String>::new()
    );
    let twice = verdict(&format!("fn open() {{ {spawn}; {spawn}; }}"), lib);
    assert_eq!(twice.len(), 2, "{twice:?}");
    assert!(twice[0].contains("is allowed 1 and holds 2"));
    let elsewhere = verdict(
        &format!("fn open() {{ {spawn}; }}\nfn other() {{ {spawn}; }}"),
        lib,
    );
    assert_eq!(
        elsewhere,
        vec![format!(
            "crates/x/src/lib.rs:2 in fn other names the process-spawning type: a process spawn \
             outside test code is not allowed"
        )]
    );
    // A nested function is its own scope; leaving it returns to the outer one.
    let nested = verdict(
        &format!("fn open() {{ fn inner() {{ {spawn}; }} {spawn}; }}"),
        lib,
    );
    assert_eq!(nested.len(), 1, "{nested:?}");
    assert!(nested[0].contains("in fn inner"));
    // The same function name in another file is not the allowed one.
    let moved = verdict(
        &format!("fn open() {{ {spawn}; }}"),
        "crates/x/src/other.rs",
    );
    assert!(
        moved
            .iter()
            .any(|f| f.starts_with("crates/x/src/other.rs:1 in fn open"))
    );
    assert!(
        moved
            .iter()
            .any(|f| f.contains("which starts no program any more"))
    );
    // A stale entry is a failure.
    assert_eq!(
        verdict("fn open() {}", lib),
        vec![
            "the allowlist names fn open in crates/x/src/lib.rs, which starts no program any \
             more: remove the entry rather than leave it to excuse a later one"
                .to_owned()
        ]
    );
}

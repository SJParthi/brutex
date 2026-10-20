//! **No backend code starts another program at run time, except the two the
//! allowlist below names.** D-5240, D-5243, D-5244, D-5246.
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
//! failed on a third, `crates/cli/src/watchdog_command.rs`
//! `<Notify for Opener>::notify`, which R3-32 (D-4996) added to hand the
//! watchdog's notice page to `open`: the same kind of start, a page shown in
//! the operator's browser, and the second entry (D-5243).
//!
//! # What it is, and what it claims
//!
//! A MODEL of how `rustc` and Cargo find and read code, written as a lexer
//! and a few path rules, not the compiler. It claims exactly the spellings and
//! the placements listed below, each with a planted case in this file's own
//! tests, and no more: a spelling this header does not list is not claimed to
//! be seen. Two re-checks of this file (19 Oct 2026, D-5244) each found
//! spellings an earlier version passed, and a third (20 Oct 2026, D-5246)
//! found attributes it placed on more than they are on: a gated generic or
//! closure parameter, a field or an argument named `union`, and a `#[test]`
//! re-pointed by an import. Where its model of the compiler cannot place a
//! file or an attribute, it now refuses rather than guesses.
//!
//! # What it reads
//!
//! Every `.rs` file under `crates/`, every `Cargo.toml` there at any depth and
//! the workspace's, read from the disk rather than from `include_str!`, so a
//! new file is scanned the day it appears; every other file under `crates/`
//! is listed, not read, so a target or a module file of another kind is known
//! to be there. NOTHING UNDER `crates/` IS SKIPPED: a directory named `target`
//! is read like any other (the workspace's build directory is at the root,
//! outside `crates/`), and a link is not followed but refused by name, since a
//! module behind it would go unread (D-5244).
//!
//! Each file is lexed as `rustc_lexer` reads the parts this scan uses. A
//! leading byte-order mark and then a shebang line are dropped as
//! `rustc_lexer::strip_shebang` drops one: `#!` whose next token, past
//! whitespace and plain (non-doc) comments, is not `[`; `#!` before `[` is an
//! inner attribute and is read. `\r\n` is read as `\n`. Whitespace is
//! `rustc`'s set (`Pattern_White_Space`, the two direction marks among it).
//! Comments, string literals (their escapes decoded: `\x`, `\u{..}`, a line
//! continuation), raw strings, byte and C strings, char literals and
//! lifetimes are recognised, so prose and text that merely spell the path
//! neither trip the scan nor hide code after them. A file the lexer cannot
//! read (an unclosed comment or string) is a failure, not a pass.
//!
//! A **process spawn** is any of these spellings, in code or in an attribute,
//! each of which can bring into scope the type the `process` modules of `std`
//! and `tokio` construct a child with:
//!
//! * the path `process::<type>`, with any spacing or a raw identifier;
//! * a `process::{..}` import group holding, at any nesting, the type, a `*`,
//!   a `self as ..` or a macro variable;
//! * a glob import `process::*` and a renamed module `process as ..`;
//! * a path that goes on from `process::` with anything but a plain name: a
//!   macro variable (`process::$t`) or a token this scan cannot read; and the
//!   module's name straight before a macro variable or a repetition
//!   (`process $(::$t)*`, `process $sep $t`);
//! * the type where something other than a plain name gives it its path:
//!   after a `::` that follows no identifier (`[<..>]::`, `)::`, `$(..)*::`),
//!   straight after a repetition (`$($s::)*<type>`) or straight after a macro
//!   variable (`$m::<type>`, or `$s <type>` where `$s` can be `::`).
//!   `x::<type>`, `$crate::<type>` and `<S as T>::<type>` are plain paths;
//! * two macro variables joined (`$m::$t`), and a `use` built from a variable;
//! * a macro invocation handed the bare module name and the type apart, or
//!   either of them beside a macro variable that can carry the other in from
//!   an outer call, which its body can join;
//! * an `include!` of anything but a plain string, whose file cannot be named,
//!   and a `use` that names `include` (under another name it pastes a file
//!   this scan would not see being pasted).
//!
//! A construction needs the type in scope, so naming it is the earliest point
//! the scan can refuse; a mere mention in a signature is refused too, which
//! errs towards failing.
//!
//! **Each naming is placed by its whole path in the file**: inline modules,
//! traits, `impl` headers (`<Notify for Opener>`, generics and `where` dropped)
//! and the functions around it. An allowlist entry is that path, so a function
//! of the same name in a nested module, another `impl` or a trait never takes
//! the entry over (D-5244).
//!
//! # How it decides what is test code
//!
//! Test code is what only the test harness compiles, and the scan follows
//! `rustc`'s own rule for that rather than a file-name convention:
//!
//! * **an integration test**: a file under `crates/<crate>/tests/` that no
//!   shipped declaration reaches (below);
//! * **what `#[cfg(P)]` is on**, where `P` holds only under `test` — `test`
//!   itself, `all(..)` with such a member, or `any(..)` all of whose members
//!   are such: an item, a statement, an arm, a field (named, tuple or in a
//!   struct expression), a variant, a function's or a closure's parameter, or
//!   a generic parameter (a type, a const or a lifetime, on a function, a
//!   struct, an `impl` or a trait). `not(test)`, `any(test, X)` and every other
//!   predicate are shipped code. `cfg` is a name `rustc` reserves, so no
//!   import re-points it. WHERE THE ATTRIBUTE STANDS DECIDES WHAT IT IS ON
//!   (D-5246): only at the start, or after `;`, `{`, `}` or another
//!   attribute, can it be on an item, and there `union`, `auto` and
//!   `macro_rules` begin one only in item syntax (`union Name {`, `auto
//!   trait`, `macro_rules!`), since a field, an argument, a variant or a
//!   binding may be named by any of them. An item ends at its own `;`, at the
//!   brace that closes its body, or at a `,` outside its generics and its
//!   `where` clause; anything else also ends at its `,`, and before a `>`
//!   (not of `->` or `=>`) or a `|`, so a gated generic parameter ends at the
//!   `>` closing its list and a closure's at the `|` closing its own. Nothing
//!   gated extends past a bracket that closes something it is inside, so a
//!   gated field cannot swallow the field after it, nor a gated parameter the
//!   body after it;
//! * **a free function under the built-in `#[test]`**, and nothing else under
//!   a bare `#[test]` (D-5246). What `test` names is name resolution's, not
//!   the spelling's: `rustc` 1.97.1 compiled a `use .. as test;`, the same in
//!   a function body, `#[macro_use] extern crate` of a crate with a macro
//!   named `test`, and a derive helper named `test` each into a build without
//!   `--test`, the item kept. So a bare `#[test]` exempts only a `fn` written
//!   directly in a module (the file's or an inline one's: not in a function,
//!   an `impl`, a trait, a block or a macro body), in a file that holds no
//!   `use` or `extern crate` spelling `test` and no `include!`. A glob import
//!   or a macro's expansion cannot re-point it (`rustc` refuses either as
//!   ambiguous with the prelude, E0659); `#[macro_use] extern crate`, which
//!   re-points it in every file of a crate, is refused by name, and so is a
//!   `fn` only a bare `#[test]` exempts in a file an `include!` pastes into
//!   shipped code, where `test` resolves in the file that pastes it;
//! * **a module file a test-only declaration names**: a `mod name;` behind such
//!   an attribute, or inside a test-only inline module, names its file by
//!   `rustc`'s path rules (`#[path]` relative to the declaring file's directory;
//!   otherwise `name.rs` or `name/mod.rs` under the module's directory, a
//!   non-`mod.rs` file adding its own stem, an inline module its name). Every
//!   module that file declares is test code too, to a fixed point. A test-only
//!   declaration whose file is not there is a failure;
//! * **a file whose first attributes include `#![cfg(test)]`**.
//!
//! **WHAT SHIPPED CODE REACHES IS SHIPPED, wherever it sits and whatever its
//! name** (D-5244): a `mod` declaration (by its default paths, its first
//! `#[path]` and every later one, and any `path = ..` inside a `cfg_attr`;
//! `r#path` is `path`) or an `include!` (in any delimiters; `r#include` is
//! `include`) in shipped code ships the file it names, even under `tests/`,
//! closed over the modules that file declares; `rustc` reads a file a
//! `#[path]` loads as a `mod.rs`, so what a non-`mod.rs` file declares is
//! looked for both under its stem and as a `mod.rs` would name it. So does a
//! `path` or `build` value any manifest gives outside a `[[test]]` table,
//! read as TOML (quoted and dotted keys, a `#` inside a string, `\u` escapes,
//! a header only at a line's start and outside an array). A crate root the
//! build compiles — the library, a binary, the build script, a bench or an
//! example — is shipped whatever else names it, so a test that `#[path]`s one
//! in exempts nothing. Only a file under `#![cfg(test)]` stays test code when
//! shipped code declares it, since the compiler drops it.
//!
//! **WHAT IT CANNOT PLACE IS REFUSED BY NAME**, never passed over: a shipped
//! declaration with no `.rs` file where this model looks (a name in another
//! case, which a case-insensitive disk finds; a file of another kind); an
//! inline module given a `#[path]`, or any module an inner `#![path]`, which
//! moves the directory its module files are read from; an `include!` or a
//! `mod name;` inside a `macro_rules!` body, which `rustc` reads from wherever
//! the macro is called; a manifest target of another kind, or naming no
//! source; a manifest value given as a multi-line string; a shipped
//! `#[macro_use] extern crate`, and a `fn` only a bare `#[test]` exempts in a
//! file an `include!` pastes into shipped code (D-5246); and a shipped
//! declaration, a manifest target, a path dependency, a `[patch]` or a
//! workspace member outside `crates/`.
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
//! * **A spelling not listed above.** The list is what is claimed and planted;
//!   this header does not claim that no other spelling exists.
//! * **A dependency's own code.** A crate in `Cargo.lock` that spawns at run
//!   time is not under `crates/`. `crates/vocab/tests/workspace_is_rust.rs`
//!   pins the dependency set, so a new one is reviewed, but this scan does not
//!   read it, nor a dependency's re-export of the type under another name, nor
//!   a dependency's macro whose body spells the type (only what the call site
//!   hands it is read).
//! * **Code that does not exist as text under `crates/`**: what a procedural
//!   macro or a build script generates. Its usual door, `include!` of a path
//!   built by `concat!(env!("OUT_DIR"), ..)`, is refused; a procedural macro's
//!   output is not. Nor is what an attribute macro does with the item it is
//!   on: one on an inline module or a function may drop or rewrite a
//!   `#[cfg(test)]` or a `#[test]` inside it that this scan honoured.
//! * **A foreign call** (`posix_spawn` through FFI). Gate 16 makes every crate
//!   root `#![forbid(unsafe_code)]`, which is what closes that door; this scan
//!   does not repeat it.
//! * **A `cfg` it does not evaluate.** Only predicates that hold solely under
//!   `test` exempt code; every other one is read as shipped, so a spawn behind
//!   `cfg(windows)` fails here too. That errs towards failing, never passing.
//! * **A build that turns `test` on outside the harness**: `--cfg test` in
//!   `RUSTFLAGS` or a `.cargo/config.toml`, or a build script printing
//!   `cargo:rustc-cfg=test`. The scan takes `test` to hold only under the
//!   harness; none of those is read here.
//! * **What Cargo reads unasked outside `crates/`.** A declaration or a
//!   manifest pointing outside `crates/` is refused, but a file Cargo reads
//!   without being pointed at, such as a `.cargo/config.toml` overriding a
//!   dependency's source, is not read here.

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

/// One function allowed to start a program, by its whole path in its file
/// (see [`Nesting::function`]), how many namings of the spawning type it
/// holds, and why.
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
        function: "<Notify for Opener>::notify",
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

/// Whitespace as `rustc_lexer::is_whitespace` has it: Unicode
/// `Pattern_White_Space`. Not `char::is_whitespace`, which leaves out the two
/// direction marks `rustc` skips (so `process\u{200e}::` would split the
/// path here and compile there) and takes in characters `rustc` refuses.
fn is_whitespace(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n'
            | '\u{b}'
            | '\u{c}'
            | '\r'
            | ' '
            | '\u{85}'
            | '\u{200e}'
            | '\u{200f}'
            | '\u{2028}'
            | '\u{2029}'
    )
}

fn is_ident_start(c: char) -> bool {
    c == '_' || c.is_alphabetic()
}

fn is_ident_continue(c: char) -> bool {
    c == '_' || c.is_alphanumeric()
}

/// The tokens of `src`, comments dropped.
///
/// `rustc` drops a byte-order mark and then a shebang line before its lexer
/// sees a token, and so does this scan: read as code, a quote on that line
/// would open a string that hides everything after it. It also reads `\r\n`
/// as `\n`, as `rustc` normalises it, so a line continuation in a string ends
/// the same way in both.
///
/// # Errors
///
/// A sentence naming the line of a comment, string or char that never closes.
fn lex(src: &str) -> Result<Vec<Token>, String> {
    let src = src.strip_prefix('\u{feff}').unwrap_or(src);
    let src = src.replace("\r\n", "\n");
    let c: Vec<char> = src.chars().collect();
    let at = |k: usize| c.get(k).copied();
    let mut out = Vec::new();
    let mut i = shebang_end(&c);
    let mut line = 1;
    while let Some(ch) = at(i) {
        let start = line;
        if ch == '\n' {
            line += 1;
            i += 1;
        } else if is_whitespace(ch) {
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

/// Where the code of `c` starts: past its first line when it opens with a
/// shebang as `rustc_lexer::strip_shebang` reads one — `#!` whose next token,
/// past whitespace and plain comments, is not `[` (a doc comment is a token
/// there, as it is to `rustc`) — and 0 otherwise. The line's own newline is
/// kept, so every line number is still the file's.
fn shebang_end(c: &[char]) -> usize {
    let at = |k: usize| c.get(k).copied();
    if !(at(0) == Some('#') && at(1) == Some('!')) {
        return 0;
    }
    let mut i = 2;
    let mut lines = 0;
    loop {
        match (at(i), at(i + 1), at(i + 2)) {
            (Some(ch), _, _) if is_whitespace(ch) => i += 1,
            (Some('/'), Some('/'), third) => {
                if third == Some('!') || (third == Some('/') && at(i + 3) != Some('/')) {
                    break;
                }
                while at(i).is_some_and(|x| x != '\n') {
                    i += 1;
                }
            }
            (Some('/'), Some('*'), third) => {
                let doc = third == Some('!')
                    || (third == Some('*') && !matches!(at(i + 3), Some('*' | '/')));
                match block_comment(c, i, &mut lines) {
                    Some(next) if !doc => i = next,
                    _ => break,
                }
            }
            (Some('['), _, _) => return 0,
            _ => break,
        }
    }
    c.iter().position(|&x| x == '\n').unwrap_or(c.len())
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
///
/// Every escape is DECODED as `rustc` decodes it, since the value can be a
/// `#[path]` or an `include!` file: `"..\x2ftests/h.rs"` names
/// `../tests/h.rs` to the compiler and must name it here. A line continuation
/// drops the whitespace after it. An escape `rustc` refuses is kept as
/// written: such a file does not compile.
fn cooked(c: &[char], from: usize, line: &mut usize) -> Option<(String, usize)> {
    let mut value = String::new();
    let mut i = from;
    loop {
        match *c.get(i)? {
            '"' => return Some((value, i + 1)),
            '\\' => {
                let escaped = *c.get(i + 1)?;
                i += 2;
                match escaped {
                    'n' => value.push('\n'),
                    't' => value.push('\t'),
                    'r' => value.push('\r'),
                    '0' => value.push('\0'),
                    'x' => {
                        let hex: String = c.get(i..i + 2)?.iter().collect();
                        match u8::from_str_radix(&hex, 16) {
                            Ok(byte) => {
                                value.push(char::from(byte));
                                i += 2;
                            }
                            Err(_) => value.push('x'),
                        }
                    }
                    'u' if c.get(i) == Some(&'{') => {
                        let close = c.iter().skip(i).position(|&x| x == '}')? + i;
                        let digits: String =
                            c[i + 1..close].iter().filter(|&&x| x != '_').collect();
                        match u32::from_str_radix(&digits, 16)
                            .ok()
                            .and_then(char::from_u32)
                        {
                            Some(decoded) => {
                                value.push(decoded);
                                i = close + 1;
                            }
                            None => value.push('u'),
                        }
                    }
                    '\n' => {
                        *line += 1;
                        while let Some(&skipped) =
                            c.get(i).filter(|x| matches!(x, ' ' | '\t' | '\n' | '\r'))
                        {
                            if skipped == '\n' {
                                *line += 1;
                            }
                            i += 1;
                        }
                    }
                    other => value.push(other),
                }
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

/// Whether the token at `i` is the spawning type's name, raw or not.
fn is_type(t: &[Token], i: usize) -> bool {
    ident(t.get(i)) == Some(TYPE)
}

/// Whether `$` at `i` opens a macro variable other than `$crate`.
fn variable(t: &[Token], i: usize) -> bool {
    punct(t, i, '$') && ident(t.get(i + 1)).is_some_and(|name| name != "crate")
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

/// Whether the attribute body `attr` (between its brackets) is `cfg(P)` with
/// `P` holding only under `test`. No import can re-point `cfg`: `rustc`
/// reserves the name in the attribute namespace (`use m::cfg;` is refused
/// with "name `cfg` is reserved in attribute namespace", rustc 1.97.1, 20 Oct
/// 2026), so the spelling is the attribute.
fn attr_is_cfg_test(attr: &[Token]) -> bool {
    keyword(attr, 0, "cfg") && punct(attr, 1, '(') && predicate(attr, 2).0
}

/// Whether the attribute body `attr` is the bare `test`. Unlike `cfg`, WHAT
/// IT NAMES IS NAME RESOLUTION'S, NOT ITS SPELLING'S (D-5246): rustc 1.97.1
/// compiled `use pm::passthru as test; #[test] pub fn run() {..}`, the same
/// `use` inside a function body, `#[macro_use] extern crate pm;` with a macro
/// named `test`, and a field marked `#[test]` under a derive whose helper is
/// named `test`, each into a build without `--test`, the item kept. So
/// [`pass`] lets it exempt only where none of those reaches.
fn attr_is_bare_test(attr: &[Token]) -> bool {
    attr.len() == 1 && keyword(attr, 0, "test")
}

/// Whether this file holds something that can bind the name `test` where its
/// own `#[test]` resolves: a `use` or an `extern crate` that spells the name
/// anywhere (a group member, a rename, a raw `r#test`, inside a function body
/// or a macro body), or an `include!`, which pastes another file's text, and
/// so its imports, here. A glob import and a macro's expansion cannot: rustc
/// 1.97.1 refuses both as ambiguous with the prelude's `test` (E0659), so a
/// build that compiles has neither re-pointing it. Erring wide only exempts
/// less.
fn may_rebind_test(t: &[Token]) -> bool {
    (0..t.len()).any(|k| {
        let declares = (keyword(t, k, "use") && !punct(t, k + 1, '<'))
            || (keyword(t, k, "extern") && keyword(t, k + 1, "crate"));
        includes(t, k)
            || (declares
                && t[k..]
                    .iter()
                    .take_while(|x| x.tok != Tok::Punct(';'))
                    .any(|x| matches!(&x.tok, Tok::Ident { name, .. } if name == "test")))
    })
}

/// Whether the tokens at `i` begin a function item: `fn`, after any of
/// `const`, `async`, `unsafe`, `safe` and `extern "abi"`. The built-in
/// `#[test]` is refused on anything else ("the `#[test]` attribute may only
/// be used on a free function", rustc 1.97.1), so a `#[test]` that compiles
/// on anything else is not the built-in one.
fn fn_item(t: &[Token], i: usize) -> bool {
    let mut k = i;
    loop {
        if ["const", "async", "unsafe", "safe"]
            .iter()
            .any(|word| keyword(t, k, word))
        {
            k += 1;
        } else if keyword(t, k, "extern") {
            k += 1;
            if matches!(t.get(k).map(|x| &x.tok), Some(Tok::Str(_))) {
                k += 1;
            }
        } else {
            return keyword(t, k, "fn");
        }
    }
}

/// Whether the `]` at `close` closes an attribute, `#[..]` or `#![..]`.
fn closes_attribute(t: &[Token], close: usize) -> bool {
    let mut depth = 0_usize;
    for k in (0..=close).rev() {
        match t[k].tok {
            Tok::Punct(')' | ']' | '}') => depth += 1,
            Tok::Punct('(' | '[' | '{') => {
                depth -= 1;
                if depth == 0 {
                    let hash = |at: Option<usize>| at.is_some_and(|at| punct(t, at, '#'));
                    let before = k.checked_sub(1);
                    return punct(t, k, '[')
                        && (hash(before)
                            || (before.is_some_and(|at| punct(t, at, '!'))
                                && hash(before.and_then(|at| at.checked_sub(1)))));
                }
            }
            _ => {}
        }
    }
    false
}

/// Whether an attribute that opens at `at` stands where an item or a
/// statement can begin: at the start, or straight after `;`, `{`, `}` or the
/// `]` that closes another attribute. ANYWHERE ELSE — after `<`, `,`, `(`,
/// `|`, `=` — it is on a generic parameter, an argument, a tuple field, a
/// closure's parameter, a later field or an expression, and never on an
/// item, whatever keyword follows it (`<#[cfg(test)] const N: usize>`).
fn statement_position(t: &[Token], at: usize) -> bool {
    let Some(before) = at.checked_sub(1) else {
        return true;
    };
    punct(t, before, ';')
        || punct(t, before, '{')
        || punct(t, before, '}')
        || (punct(t, before, ']') && closes_attribute(t, before))
}

/// The files the attribute body `attr` names for the `mod` it is on: the value
/// of a whole `path = ".."` attribute, which `rustc` always follows, and the
/// value of every `path = ".."` inside it, such as `cfg_attr(unix, path =
/// "..")`, which it follows when the predicate holds and which this scan reads
/// as reached either way.
fn attr_paths(attr: &[Token]) -> (Option<String>, Vec<String>) {
    let value = |k: usize| match attr.get(k + 2).map(|x| &x.tok) {
        Some(Tok::Str(file)) if ident(attr.get(k)) == Some("path") && punct(attr, k + 1, '=') => {
            Some(file.clone())
        }
        _ => None,
    };
    (value(0), (1..attr.len()).filter_map(value).collect())
}

/// The strict keywords an item can begin with, after its visibility. None can
/// name a field, an argument, a variant or a binding (only its raw spelling,
/// which [`keyword`] does not match).
const ITEM_KEYWORDS: &[&str] = &[
    "fn", "struct", "enum", "trait", "type", "mod", "const", "static", "use", "impl", "extern",
    "unsafe", "async",
];

/// Rust 2024's strict and reserved keywords: never an item's name.
const KEYWORDS: &[&str] = &[
    "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn", "for",
    "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return",
    "self", "Self", "static", "struct", "super", "trait", "true", "type", "unsafe", "use", "where",
    "while", "async", "await", "dyn", "abstract", "become", "box", "do", "final", "macro",
    "override", "priv", "typeof", "unsized", "virtual", "yield", "try", "gen",
];

/// Whether the token at `k` can be an item's name.
fn item_name(t: &[Token], k: usize) -> bool {
    matches!(t.get(k).map(|x| &x.tok), Some(Tok::Ident { name, raw }) if *raw || !KEYWORDS.contains(&name.as_str()))
}

/// Whether the tokens from `i` begin an item rather than a field, an arm, an
/// argument or an expression. `union`, `auto` and `macro_rules` are keywords
/// ONLY IN ITEM SYNTAX, and a field, an argument, a variant or a binding may
/// be named by any of them (D-5246): each counts only as `union Name {`,
/// `union Name<` or `union Name where`, `auto trait` and `macro_rules!`.
fn is_item(t: &[Token], i: usize) -> bool {
    let mut k = i;
    if keyword(t, k, "pub") {
        k += 1;
        if punct(t, k, '(') {
            k = matching(t, k).map_or(t.len(), |close| close + 1);
        }
    }
    ITEM_KEYWORDS.iter().any(|word| keyword(t, k, word))
        || (keyword(t, k, "union")
            && item_name(t, k + 1)
            && (punct(t, k + 2, '{') || punct(t, k + 2, '<') || keyword(t, k + 2, "where")))
        || (keyword(t, k, "auto") && keyword(t, k + 1, "trait"))
        || (keyword(t, k, "macro_rules") && punct(t, k + 1, '!'))
}

/// Whether the `>` at `i` is the second half of `->` or `=>`.
fn arrow(t: &[Token], i: usize) -> bool {
    i.checked_sub(1)
        .is_some_and(|before| punct(t, before, '-') || punct(t, before, '='))
}

/// Where the gated thing starting at `from` ends: just past its `;`, just past
/// the brace that closes its body, or just before a bracket that closes
/// something it is inside. WHAT IS NOT AN ITEM (`item` false) ALSO ENDS just
/// past its `,`, and just before a `>` that is not half of `->` or `=>` and a
/// `|`, neither of which this scan counts as a bracket: so a gated generic
/// parameter (`<#[cfg(test)] T>`) ends at the `>` that closes the list, and a
/// closure's gated parameter at the `|` that closes it, never running on
/// through the body after them (D-5246). AN ITEM ENDS AT A `,` TOO, unless the
/// comma is in its generics (`<A, B>`) or its `where` clause, the only places
/// an item has one outside brackets: so an item a macro is handed in a list
/// (`m! { #[cfg(test)] struct A, struct B(..) }`) does not run on into the
/// next. Ending early only ever reads more code as shipped.
fn item_end(t: &[Token], from: usize, item: bool) -> usize {
    let mut depth = 0_usize;
    let mut angle = 0_usize;
    let mut clause = false;
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
            Tok::Punct(',') if depth == 0 && (!item || (angle == 0 && !clause)) => return i + 1,
            Tok::Punct('<') if depth == 0 => angle += 1,
            Tok::Punct('>') if depth == 0 && !arrow(t, i) => {
                if !item {
                    return i;
                }
                angle = angle.saturating_sub(1);
            }
            Tok::Punct('|') if depth == 0 && !item => return i,
            _ if depth == 0 && keyword(t, i, "where") => clause = true,
            _ => {}
        }
        i += 1;
    }
    i
}

// ------------------------------------------------------------- the needle --

/// Whether the tokens at `i` can bring the spawning type into scope, and if
/// so what that spelling does and how many tokens it spans. Each span is
/// bracket-balanced, so skipping it keeps the depth a pass tracks.
fn naming(t: &[Token], i: usize) -> Option<(&'static str, usize)> {
    through_module(t, i)
        .or_else(|| module_before_variable(t, i))
        .or_else(|| through_variable(t, i))
        .or_else(|| use_through_variable(t, i))
        .or_else(|| use_of_include(t, i))
        .or_else(|| handed_to_a_macro(t, i))
        .or_else(|| type_after_join(t, i))
}

/// The type where something other than a plain name gives it its path, which
/// a macro can make the module: after a `::` that follows no identifier (a
/// generated segment `[<..>]::`, a group `)::`, a repetition `$(..)*::`),
/// straight after a repetition (`$($s::)*<type>`), or straight after a macro
/// variable, which can be the `::` itself. `x::<type>`, `$crate::<type>` and
/// `<S as T>::<type>` are plain paths and pass.
fn type_after_join(t: &[Token], i: usize) -> Option<(&'static str, usize)> {
    if !is_type(t, i) || i == 0 {
        return None;
    }
    let joined = if i >= 2 && path_sep(t, i - 2) {
        let plain_name = i >= 3 && ident(t.get(i - 3)).is_some() && !(i >= 4 && variable(t, i - 4));
        !(plain_name || (i >= 3 && punct(t, i - 3, '>')))
    } else {
        matches!(t[i - 1].tok, Tok::Punct('*' | '+' | '?')) || (i >= 2 && variable(t, i - 2))
    };
    joined.then_some((
        "joins the process-spawning type to a path this scan cannot read (a generated \
         segment, a repetition or a macro variable)",
        1,
    ))
}

/// A `use` that names the `include!` macro: under another name it pastes a
/// file this scan follows only by that name.
fn use_of_include(t: &[Token], i: usize) -> Option<(&'static str, usize)> {
    if !keyword(t, i, "use") || punct(t, i + 1, '<') {
        return None;
    }
    let end = (i..t.len()).find(|&k| punct(t, k, ';')).unwrap_or(t.len());
    (i..end)
        .any(|k| ident(t.get(k)) == Some("include"))
        .then_some((
            "imports the include! macro, which can paste a file under a name this scan does not \
         follow",
            1,
        ))
}

/// A spelling that starts with the module's name.
fn through_module(t: &[Token], i: usize) -> Option<(&'static str, usize)> {
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
    match t.get(i + 3).map(|x| &x.tok) {
        Some(Tok::Ident { name, .. }) if name == TYPE => {
            Some(("names the process-spawning type", 4))
        }
        Some(Tok::Ident { .. }) => None,
        Some(Tok::Punct('*')) => Some((
            "glob-imports the process module, the spawning type with it",
            4,
        )),
        Some(Tok::Punct('{')) => {
            let close = matching(t, i + 3).unwrap_or(t.len());
            group_hides(&t[i + 3..close]).map(|what| (what, close - i))
        }
        Some(Tok::Punct('$')) => Some((
            "joins a macro variable to the process module, which can name the spawning type",
            4,
        )),
        _ => Some((
            "goes on from the process module with a token this scan cannot read",
            3,
        )),
    }
}

/// The module's name straight before a macro variable or a repetition
/// (`process $(::$t)*`, `process $sep $t`): what follows it is made at the
/// call site and can be `::` and the spawning type.
fn module_before_variable(t: &[Token], i: usize) -> Option<(&'static str, usize)> {
    (ident(t.get(i)) == Some(MODULE) && punct(t, i + 1, '$')).then_some((
        "puts a macro variable or a repetition after the process module, which can carry \
         the path on to the spawning type",
        1,
    ))
}

/// What a `process::{..}` group hides, at any nesting: the type, a glob, the
/// module renamed by `self as`, or a macro variable.
fn group_hides(group: &[Token]) -> Option<&'static str> {
    (0..group.len()).find_map(|k| match group[k].tok {
        Tok::Punct('*') => Some("glob-imports the process module in an import group"),
        Tok::Punct('$') => Some("imports from the process module through a macro variable"),
        _ if is_type(group, k) => Some("imports the process-spawning type"),
        _ if keyword(group, k, "self") && keyword(group, k + 1, "as") => Some(
            "renames the process module in an import group, which hides its spawns from this scan",
        ),
        _ => None,
    })
}

/// A macro variable joined to the type, or to a second variable: either can
/// assemble the path at the macro's call site.
fn through_variable(t: &[Token], i: usize) -> Option<(&'static str, usize)> {
    if !(variable(t, i) && path_sep(t, i + 2)) {
        return None;
    }
    if is_type(t, i + 4) {
        return Some(("joins a macro variable to the process-spawning type", 5));
    }
    variable(t, i + 4).then_some((
        "joins two macro variables into a path, which can be the process module and the \
         spawning type",
        4,
    ))
}

/// A `use` declaration built from a macro variable: it can rename or
/// glob-import the module without spelling it. `use<..>` (precise capturing in
/// an `impl` bound) is no declaration.
fn use_through_variable(t: &[Token], i: usize) -> Option<(&'static str, usize)> {
    if !keyword(t, i, "use") || punct(t, i + 1, '<') {
        return None;
    }
    let end = (i..t.len()).find(|&k| punct(t, k, ';')).unwrap_or(t.len());
    (i..end).any(|k| variable(t, k)).then_some((
        "imports through a macro variable, which can rename or glob-import the process module",
        1,
    ))
}

/// A macro invocation handed the bare module name and the type apart, or
/// either of them beside a macro variable, which can carry the other in from
/// an outer call: its body can join them, so the call site is where they are
/// refused. A macro body that hands `$($s)* <type>` or `std <module> $t` on to
/// another macro is refused here.
fn handed_to_a_macro(t: &[Token], i: usize) -> Option<(&'static str, usize)> {
    let invoked = ident(t.get(i)).is_some_and(|name| name != "macro_rules")
        && punct(t, i + 1, '!')
        && ['(', '[', '{'].iter().any(|&open| punct(t, i + 2, open));
    if !invoked {
        return None;
    }
    let close = matching(t, i + 2).unwrap_or(t.len());
    let inside = &t[i + 3..close];
    let has_type = (0..inside.len()).any(|k| is_type(inside, k));
    let bare_module = (0..inside.len()).any(|k| {
        ident(inside.get(k)) == Some(MODULE)
            && !path_sep(inside, k + 1)
            && !keyword(inside, k + 1, "as")
    });
    let has_variable = (0..inside.len()).any(|k| variable(inside, k));
    ((has_type && (bare_module || has_variable)) || (bare_module && has_variable)).then_some((
        "hands a macro the process module or the spawning type beside the other or a macro \
         variable, which its body can join",
        1,
    ))
}

/// Every naming in the attribute body `attr`, with its line.
fn namings_in(attr: &[Token]) -> Vec<(usize, &'static str)> {
    let mut out = Vec::new();
    let mut k = 0;
    while k < attr.len() {
        if let Some((what, span)) = naming(attr, k) {
            out.push((attr[k].line, what));
            k += span;
        } else {
            k += 1;
        }
    }
    out
}

// ----------------------------------------------------------- one file pass --

/// Where a file sits, as module resolution needs it.
#[derive(Clone)]
struct Place {
    dir: PathBuf,
    /// A `lib.rs`, `main.rs`, `mod.rs` or crate root: its modules sit beside
    /// it, not in a directory named after it.
    mod_rs: bool,
    stem: String,
}

/// One `mod name;` declaration or `include!`: the files it names, and whether
/// only the test harness compiles it.
#[derive(Debug)]
struct Decl {
    line: usize,
    name: String,
    /// The files `rustc` may load for it, as this file's place names them.
    candidates: Vec<String>,
    /// Files it may also reach: a `cfg_attr` path, and the modules beside a
    /// non-`mod.rs` file, which is where they are when a `#[path]` loaded it.
    also: Vec<String>,
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
    /// The lines of each `fn` that only a bare `#[test]` exempted. If an
    /// `include!` pastes this file, `rustc` resolves `test` in the file that
    /// pastes it, so [`judge`] refuses them there.
    bare_tests: Vec<usize>,
}

impl FileScan {
    fn record(&mut self, test: bool, line: usize, function: Option<String>, what: &'static str) {
        if test {
            self.in_tests += 1;
        } else {
            self.found.push(Found {
                line,
                function,
                what,
            });
        }
    }
}

enum Scope {
    Fn(String),
    Mod(String),
    Trait(String),
    Impl(String),
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
    /// Where the first of them opens, which says whether they stand where an
    /// item can ([`statement_position`]).
    start: Option<usize>,
    /// A `cfg` that holds only under `test`.
    test: bool,
    /// The bare `#[test]`, which exempts only where [`pass`] lets it.
    bare_test: bool,
    /// `#[macro_use]`, refused on an `extern crate`.
    macro_use: bool,
    path: Option<String>,
    also: Vec<String>,
    namings: Vec<(usize, &'static str)>,
}

/// Where a pass stands: its bracket depth, the functions, traits, impls and
/// inline modules open around it, and the one whose body the next `{` opens.
struct Nesting<'a> {
    inline: &'a [String],
    depth: usize,
    scopes: Vec<(Scope, usize)>,
    pending: Option<Scope>,
}

impl Nesting<'_> {
    /// Whether this point is directly in a module's body (the file's, or an
    /// inline module's), not inside a function, a trait, an `impl`, a type,
    /// a block or a macro body: where the built-in `#[test]` takes a free
    /// function, and where no derive's helper attribute is in scope.
    fn in_module_body(&self) -> bool {
        self.pending.is_none()
            && self
                .scopes
                .iter()
                .all(|(scope, _)| matches!(scope, Scope::Mod(_)))
            && self.depth == self.scopes.last().map_or(0, |(_, at)| *at)
    }

    /// The inline modules around this point, outermost first.
    fn modules(&self) -> Vec<String> {
        self.inline
            .iter()
            .cloned()
            .chain(self.scopes.iter().filter_map(|(scope, _)| match scope {
                Scope::Mod(name) => Some(name.clone()),
                _ => None,
            }))
            .collect()
    }

    /// The innermost function around this point, by its whole path in the
    /// file: `open`, `tests::t`, `<Notify for Opener>::notify`, `Trait::f`,
    /// `outer::inner`. A point in a function's signature, before its body
    /// opens, is in that function (repair round 2, D-5246).
    fn function(&self) -> Option<String> {
        let name = |scope: &Scope| match scope {
            Scope::Fn(name) | Scope::Mod(name) | Scope::Trait(name) => name.clone(),
            Scope::Impl(header) => format!("<{header}>"),
        };
        if let Some(signature @ Scope::Fn(_)) = &self.pending {
            let parts: Vec<String> = self
                .scopes
                .iter()
                .map(|(scope, _)| name(scope))
                .chain(std::iter::once(name(signature)))
                .collect();
            return Some(parts.join("::"));
        }
        let innermost = self
            .scopes
            .iter()
            .rposition(|(scope, _)| matches!(scope, Scope::Fn(_)))?;
        let parts: Vec<String> = self.scopes[..=innermost]
            .iter()
            .map(|(scope, _)| name(scope))
            .collect();
        Some(parts.join("::"))
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

/// The header of the `impl` whose first token after the keyword is at `from`,
/// as its scope's name: generic parameters and the `where` clause dropped, so
/// `impl<T> Notify for Opener<T> where T: Copy {` is `Notify for Opener<T>`.
fn impl_header(t: &[Token], from: usize) -> String {
    let mut k = from;
    if punct(t, k, '<') {
        let mut depth = 0_usize;
        while let Some(tok) = t.get(k) {
            match tok.tok {
                Tok::Punct('<') => depth += 1,
                Tok::Punct('>') if !punct(t, k - 1, '-') => depth = depth.saturating_sub(1),
                _ => {}
            }
            k += 1;
            if depth == 0 {
                break;
            }
        }
    }
    let mut out = String::new();
    let mut after_word = false;
    while let Some(tok) = t.get(k) {
        match &tok.tok {
            Tok::Punct('{' | ';') => break,
            Tok::Ident { name, .. } if name == "where" => break,
            Tok::Ident { name, .. } => {
                if after_word {
                    out.push(' ');
                }
                out.push_str(name);
                after_word = true;
            }
            Tok::Punct(p) => {
                out.push(*p);
                after_word = false;
            }
            Tok::Str(_) | Tok::Other => {
                out.push('_');
                after_word = false;
            }
        }
        k += 1;
    }
    out
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

/// Whether `include!` opens at `i`, with any of the three delimiters a macro
/// call may take: `include!(..)`, `include![..]` and `include!{..}` all paste.
fn includes(t: &[Token], i: usize) -> bool {
    ident(t.get(i)) == Some("include")
        && punct(t, i + 1, '!')
        && ['(', '[', '{'].iter().any(|&open| punct(t, i + 2, open))
}

/// The file an `include!("..")` at `i` pastes, in whichever delimiters.
fn included(t: &[Token], i: usize) -> Option<&str> {
    let pasted = includes(t, i) && matching(t, i + 2) == Some(i + 4);
    match &t.get(i + 3)?.tok {
        Tok::Str(file) if pasted => Some(file),
        _ => None,
    }
}

/// Where a `macro_rules!` body at `i` pastes a file or declares a module file.
/// `rustc` reads an `include!` path, and a module file, from wherever the
/// macro is CALLED, which can be another file in another directory, so this
/// scan, reading the body where it is written, cannot place either.
fn macro_body_reaches(t: &[Token], i: usize) -> Vec<usize> {
    let open = i + 3;
    let body = keyword(t, i, "macro_rules")
        && punct(t, i + 1, '!')
        && ['(', '[', '{'].iter().any(|&o| punct(t, open, o));
    if !body {
        return Vec::new();
    }
    let close = matching(t, open).unwrap_or(t.len());
    (open + 1..close)
        .filter(|&k| {
            includes(t, k)
                || (keyword(t, k, "mod") && ident(t.get(k + 1)).is_some() && punct(t, k + 2, ';'))
        })
        .collect()
}

/// The scope a token at `i` opens, and how many tokens to step past: a
/// function, a trait or an inline module past its name; an `impl` past its
/// keyword only, so its header is read for namings like any other code.
fn opens(t: &[Token], i: usize, pending: bool) -> Option<(Scope, usize)> {
    let name = ident(t.get(i + 1)).map(str::to_owned);
    if keyword(t, i, "fn") {
        name.map(|name| (Scope::Fn(name), 2))
    } else if keyword(t, i, "trait") {
        name.map(|name| (Scope::Trait(name), 2))
    } else if keyword(t, i, "mod") && punct(t, i + 2, '{') {
        name.map(|name| (Scope::Mod(name), 2))
    } else if keyword(t, i, "impl") && !pending {
        Some((Scope::Impl(impl_header(t, i + 1)), 1))
    } else {
        None
    }
}

impl Run {
    /// Takes in the outer attribute whose body is `attr` and which opens at
    /// `at`.
    fn read(&mut self, attr: &[Token], at: usize) {
        self.start.get_or_insert(at);
        self.test |= attr_is_cfg_test(attr);
        self.bare_test |= attr_is_bare_test(attr);
        self.macro_use |= keyword(attr, 0, "macro_use");
        // `rustc` follows the FIRST `#[path]` and ignores the rest; each
        // later one is kept as a file the declaration may reach.
        let (path, also) = attr_paths(attr);
        match (&self.path, path) {
            (None, path) => self.path = path,
            (Some(_), Some(later)) => self.also.push(later),
            (Some(_), None) => {}
        }
        self.also.extend(also);
        self.namings.extend(namings_in(attr));
    }
}

/// The inner attribute at `i`, whose body is `attr`: whether it makes the
/// whole file test code, what it names, and an inner `#![path]`, refused.
fn inner_attribute(
    t: &[Token],
    i: usize,
    attr: &[Token],
    test: bool,
    at: &Nesting<'_>,
    out: &mut FileScan,
) {
    // `#![test]` is no test attribute: rustc 1.97.1 refuses it at a crate's
    // top ("cannot be used at crate level") and inside a module ("inner
    // macro attributes are unstable").
    out.whole_file_test |= at.depth == 0 && at.inline.is_empty() && attr_is_cfg_test(attr);
    for (line, what) in namings_in(attr) {
        out.record(test, line, at.function(), what);
    }
    // `mod t { #![path = "../tests"] mod h; }` reads `tests/h.rs` (rustc
    // 1.97.1, 20 Oct 2026): an inner `#![path]` moves a module's directory as
    // an outer one does, unfollowed here.
    let (path, also) = attr_paths(attr);
    if path.is_some() || !also.is_empty() {
        out.record(
            test,
            attr.first().map_or(t[i].line, |tok| tok.line),
            at.function(),
            "gives a module an inner #![path], which moves where its module files are read \
             from: this scan does not follow it",
        );
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
    let rebinds_test = may_rebind_test(t);
    while i < t.len() {
        if let Some((inner, attr, next)) = attribute(t, i) {
            if inner {
                inner_attribute(t, i, attr, test, &at, out);
            } else {
                run.read(attr, i);
            }
            i = next;
            continue;
        }
        // A VISIBILITY BELONGS TO THE ITEM AFTER IT, attributes and all: read
        // as the item's start, it took `#[path]` off `pub mod name;`.
        if keyword(t, i, "pub") {
            i += 1;
            if punct(t, i, '(') {
                i = matching(t, i).map_or(t.len(), |close| close + 1);
            }
            continue;
        }
        let attrs = std::mem::take(&mut run);
        // WHERE THE ATTRIBUTES STAND DECIDES WHAT THEY ARE ON (D-5246): only
        // where an item can begin can an item keyword start one.
        let statement = attrs
            .start
            .is_some_and(|start| statement_position(t, start));
        // A bare `#[test]` exempts only what the built-in one would take: a
        // free `fn` directly in a module, in a file where nothing can re-point
        // the name ([`may_rebind_test`]).
        let by_test_fn = !attrs.test
            && attrs.bare_test
            && !rebinds_test
            && statement
            && at.in_module_body()
            && fn_item(t, i);
        let gate_here = (attrs.test || by_test_fn) && !test;
        for &(line, what) in &attrs.namings {
            out.record(test || gate_here, line, at.function(), what);
        }
        if attrs.macro_use && keyword(t, i, "extern") && keyword(t, i + 1, "crate") {
            out.record(
                test || gate_here,
                t[i].line,
                at.function(),
                "loads a crate's macros with #[macro_use] extern crate, which re-points #[test] \
                 in every file of the crate when that crate has a macro named test: this scan \
                 cannot tell what #[test] names there",
            );
        }
        let end = if gate_here {
            item_end(t, i, statement && is_item(t, i))
        } else {
            i
        };
        if end > i {
            if by_test_fn {
                out.bare_tests.push(t[i].line);
            }
            gated(&t[i..end], &at.modules(), attrs.path.as_deref(), place, out);
            i = end;
            continue;
        }
        for k in macro_body_reaches(t, i) {
            out.record(
                test,
                t[k].line,
                at.function(),
                "pastes a file or declares a module file inside a macro body, which rustc reads \
                 from wherever the macro is called: this scan cannot place it",
            );
        }
        if let Some(step) = declaration(t, i, test, &at, &attrs, place, out) {
            i += step;
            continue;
        }
        if let Some((scope, step)) = opens(t, i, at.pending.is_some()) {
            // `#[path]` on an inline module moves the directory its module
            // files are read from (`#[path = "../tests"] mod t { mod h; }`
            // reads `tests/h.rs`), which this scan does not follow.
            if matches!(scope, Scope::Mod(_)) && (attrs.path.is_some() || !attrs.also.is_empty()) {
                out.record(
                    test,
                    t[i].line,
                    at.function(),
                    "gives an inline module a #[path], which moves where the module files \
                     inside it are read from: this scan does not follow it",
                );
            }
            at.pending = Some(scope);
            i += step;
            continue;
        }
        if let Some((what, span)) = naming(t, i) {
            out.record(test, t[i].line, at.function(), what);
            i += span;
            continue;
        }
        at.step(&t[i].tok);
        i += 1;
    }
}

/// A `mod name;` or an `include!` at `i`, recorded as a declaration, and how
/// many tokens it spans. `include!` PASTES A FILE HERE, so the file is test
/// code exactly when this spot is; its path is relative to this file's
/// directory, and one that is no plain string is refused as unnamed.
fn declaration(
    t: &[Token],
    i: usize,
    test: bool,
    at: &Nesting<'_>,
    attrs: &Run,
    place: &Place,
    out: &mut FileScan,
) -> Option<usize> {
    if includes(t, i) {
        let Some(file) = included(t, i) else {
            out.record(
                test,
                t[i].line,
                at.function(),
                "pastes a file this scan cannot name: an include! of no plain string",
            );
            return Some(3);
        };
        out.decls.push(Decl {
            line: t[i].line,
            name: format!("include!(\"{file}\")"),
            candidates: vec![normal(&place.dir.join(file))],
            also: Vec::new(),
            test,
        });
        return Some(5);
    }
    if keyword(t, i, "mod") && variable(t, i + 1) && punct(t, i + 3, ';') {
        out.record(
            test,
            t[i].line,
            at.function(),
            "declares a module file a macro variable names, which this scan cannot name",
        );
        return Some(4);
    }
    let name = ident(t.get(i + 1))?;
    if !(keyword(t, i, "mod") && punct(t, i + 2, ';')) {
        return None;
    }
    let modules = at.modules();
    let mut also: Vec<String> = attrs
        .also
        .iter()
        .flat_map(|path| candidates(place, &modules, name, Some(path)))
        .collect();
    // A non-`mod.rs` file that a `#[path]` loaded is a `mod.rs` to `rustc`,
    // so every file its declarations name is also looked for as one would
    // name it: beside it, and inside an inline module without its stem.
    if !place.mod_rs {
        let beside = Place {
            mod_rs: true,
            ..place.clone()
        };
        also.extend(candidates(&beside, &modules, name, attrs.path.as_deref()));
        also.extend(
            attrs
                .also
                .iter()
                .flat_map(|path| candidates(&beside, &modules, name, Some(path))),
        );
    }
    out.decls.push(Decl {
        line: t[i].line,
        candidates: candidates(place, &modules, name, attrs.path.as_deref()),
        name: name.to_owned(),
        also,
        test,
    });
    Some(3)
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
            also: Vec::new(),
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

/// Whether `path` names a Rust source file.
fn is_rs(path: &str) -> bool {
    Path::new(path).extension().is_some_and(|ext| ext == "rs")
}

fn under_tests_dir(path: &str) -> bool {
    matches!(
        path.split('/').collect::<Vec<_>>().as_slice(),
        ["crates", _, "tests", ..]
    )
}

// ------------------------------------------------------------- manifests --

/// The crate whose own manifest `path` is (`crates/<crate>/Cargo.toml`).
fn manifest_crate(path: &str) -> Option<&str> {
    path.strip_prefix("crates/")?
        .strip_suffix("/Cargo.toml")
        .filter(|name| !name.contains('/'))
}

/// Whether `path` is a manifest this scan reads: the workspace's, or any
/// `Cargo.toml` under `crates/` at any depth, a nested package's among them.
fn is_manifest(path: &str) -> bool {
    path == "Cargo.toml" || (path.starts_with("crates/") && path.ends_with("/Cargo.toml"))
}

/// One token of a manifest, as far as this scan reads TOML.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Toml {
    /// A bare key or value.
    Word(String),
    /// A one-line string, basic or literal, its escapes decoded.
    Text(String),
    /// A multi-line string, or one that never closes: not read.
    Unread,
    Punct(char),
    Newline,
}

/// The tokens of a manifest with their lines: comments dropped, a `#` inside
/// a string kept, basic strings decoded (`\uXXXX`, `\UXXXXXXXX` and the short
/// escapes, so `"tests/h.rs"` names `tests/h.rs` here as it does to
/// Cargo), a quoted key read like a bare one by the caller.
fn toml_tokens(text: &str) -> Vec<(usize, Toml)> {
    let c: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    let mut line = 1;
    while let Some(&ch) = c.get(i) {
        match ch {
            '\n' => {
                out.push((line, Toml::Newline));
                line += 1;
                i += 1;
            }
            '#' => {
                while c.get(i).is_some_and(|&x| x != '\n') {
                    i += 1;
                }
            }
            '"' | '\'' if c.get(i + 1) == Some(&ch) && c.get(i + 2) == Some(&ch) => {
                let start = line;
                i += 3;
                while let Some(&x) = c.get(i) {
                    if x == ch && c.get(i + 1) == Some(&ch) && c.get(i + 2) == Some(&ch) {
                        i += 3;
                        break;
                    }
                    if x == '\n' {
                        line += 1;
                    }
                    i += if x == '\\' && ch == '"' && c.get(i + 1) != Some(&'\n') {
                        2
                    } else {
                        1
                    };
                }
                out.push((start, Toml::Unread));
            }
            '"' | '\'' => {
                let (tok, next) = toml_string(&c, i, ch);
                out.push((line, tok));
                i = next;
            }
            _ if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' => {
                let start = i;
                while c
                    .get(i)
                    .is_some_and(|x| x.is_ascii_alphanumeric() || *x == '_' || *x == '-')
                {
                    i += 1;
                }
                out.push((line, Toml::Word(c[start..i].iter().collect())));
            }
            _ if ch.is_whitespace() => i += 1,
            _ => {
                out.push((line, Toml::Punct(ch)));
                i += 1;
            }
        }
    }
    out
}

/// The one-line string opening with `quote` at `at`, and the index past it.
fn toml_string(c: &[char], at: usize, quote: char) -> (Toml, usize) {
    let hex = |from: usize, digits: usize| -> Option<char> {
        let text: String = c.get(from..from + digits)?.iter().collect();
        u32::from_str_radix(&text, 16).ok().and_then(char::from_u32)
    };
    let mut value = String::new();
    let mut i = at + 1;
    loop {
        match c.get(i).copied() {
            None | Some('\n') => return (Toml::Unread, i),
            Some(x) if x == quote => return (Toml::Text(value), i + 1),
            Some('\\') if quote == '"' => {
                let (decoded, step) = match c.get(i + 1).copied() {
                    Some('b') => (Some('\u{8}'), 2),
                    Some('t') => (Some('\t'), 2),
                    Some('n') => (Some('\n'), 2),
                    Some('f') => (Some('\u{c}'), 2),
                    Some('r') => (Some('\r'), 2),
                    Some('e') => (Some('\u{1b}'), 2),
                    Some('u') => (hex(i + 2, 4), 6),
                    Some('U') => (hex(i + 2, 8), 10),
                    other => (other, 2),
                };
                value.extend(decoded);
                i += step;
            }
            Some(x) => {
                value.push(x);
                i += 1;
            }
        }
    }
}

/// What a manifest names, from the root.
#[derive(Default)]
struct Named {
    /// Every `path` or `build` value outside a `[[test]]` table: a target file,
    /// or a path dependency's directory, told apart by what is on the disk.
    targets: Vec<String>,
    /// Every path it gives at all, a `[[test]]`'s, a `[patch]` and a workspace
    /// member among them.
    every: Vec<String>,
    /// What it says that this scan does not read, by line.
    unread: Vec<String>,
}

/// What the manifest at `manifest` (the workspace's at the root, or one under
/// `crates/`) names: each `path` or `build` key's value, quoted keys and dotted
/// ones (`lib.path`) included, inline tables included, and every workspace
/// member. Read towards more, never fewer: a `[lib] path = "tests/x.rs"` ships
/// that file, and a value given as a multi-line string is refused by name.
fn manifest_paths(manifest: &str, text: &str) -> Named {
    let dir = Path::new(manifest)
        .parent()
        .unwrap_or_else(|| Path::new(""));
    let tokens = toml_tokens(text);
    let word = |k: usize| match tokens.get(k).map(|(_, tok)| tok) {
        Some(Toml::Word(w) | Toml::Text(w)) => Some(w.as_str()),
        _ => None,
    };
    let is = |k: usize, tok: &Toml| tokens.get(k).is_some_and(|(_, t)| t == tok);
    let mut named = Named::default();
    let mut in_test = false;
    // A `[` opens a table header only at the start of a line and outside any
    // array or inline table, so a value spelling `["test"]` on a line of its
    // own cannot move the keys after it into a `[[test]]`.
    let mut depth = 0_usize;
    let mut k = 0;
    while k < tokens.len() {
        let line_start = k == 0 || is(k - 1, &Toml::Newline);
        if depth == 0 && line_start && is(k, &Toml::Punct('[')) {
            let end = (k..tokens.len())
                .find(|&h| is(h, &Toml::Newline))
                .unwrap_or(tokens.len());
            let header: Vec<&str> = (k..end).filter_map(word).collect();
            in_test = header == ["test"];
            k = end;
            continue;
        }
        match tokens[k].1 {
            Toml::Punct('[' | '{') => depth += 1,
            Toml::Punct(']' | '}') => depth = depth.saturating_sub(1),
            _ => {}
        }
        let line = tokens[k].0;
        k += 1;
        let Some(key) = word(k - 1) else { continue };
        if !is(k, &Toml::Punct('=')) {
            continue;
        }
        let k = k - 1;
        if matches!(key, "path" | "build") {
            match tokens.get(k + 2).map(|(_, tok)| tok) {
                Some(Toml::Text(value)) => {
                    let path = normal(&dir.join(value));
                    if !in_test {
                        named.targets.push(path.clone());
                    }
                    named.every.push(path);
                }
                Some(Toml::Unread) => named.unread.push(format!(
                    "{manifest}:{line} gives `{key}` a multi-line or unclosed string, which this \
                     scan does not read"
                )),
                _ => {}
            }
        }
        if key == "members" && is(k + 2, &Toml::Punct('[')) {
            for member in (k + 3..tokens.len())
                .take_while(|&m| !is(m, &Toml::Punct(']')))
                .filter_map(|m| match tokens.get(m).map(|(_, tok)| tok) {
                    Some(Toml::Text(member)) => Some(member),
                    _ => None,
                })
            {
                named.every.push(normal(&dir.join(member)));
            }
        }
    }
    named
}

// ---------------------------------------------------------------- verdict --

fn first_existing(decl: &Decl, files: &BTreeMap<String, String>) -> Option<String> {
    decl.candidates
        .iter()
        .find(|candidate| files.contains_key(candidate.as_str()))
        .cloned()
}

/// THE TEST FILES: integration tests, whole-file `#![cfg(test)]`, and what a
/// test-only declaration names, closed over every declaration each of them
/// makes in turn — the first two included, which are expanded like the rest
/// rather than taken as already done.
fn test_files(
    scans: &BTreeMap<&str, FileScan>,
    files: &BTreeMap<String, String>,
    failures: &mut Vec<String>,
) -> BTreeSet<String> {
    let mut found: BTreeSet<String> = BTreeSet::new();
    let mut queue: Vec<String> = scans
        .iter()
        .filter(|(path, scan)| under_tests_dir(path) || scan.whole_file_test)
        .map(|(path, _)| (*path).to_owned())
        .collect();
    for (path, scan) in scans {
        for decl in scan.decls.iter().filter(|decl| decl.test) {
            match first_existing(decl, files) {
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
        if found.insert(file.clone())
            && let Some(scan) = scans.get(file.as_str())
        {
            queue.extend(
                scan.decls
                    .iter()
                    .filter_map(|decl| first_existing(decl, files)),
            );
        }
    }
    found
}

/// A crate root the build compiles outside the test harness: the library, the
/// main binary, another binary, the build script, a bench or an example. It
/// ships whatever else names it, so a test that `#[path]`s it in exempts
/// nothing.
fn shipped_root(path: &str) -> bool {
    matches!(
        path.split('/').collect::<Vec<_>>().as_slice(),
        ["crates", _, "src", "lib.rs" | "main.rs"]
            | ["crates", _, "build.rs"]
            | ["crates", _, "src", "bin", _]
            | ["crates", _, "src", "bin", _, "main.rs"]
            | ["crates", _, "benches" | "examples", _]
            | ["crates", _, "benches" | "examples", _, "main.rs"]
    )
}

/// THE SHIPPED FILES: every file that is not test code, every crate root the
/// build compiles ([`shipped_root`]) and every manifest target, and every file
/// a shipped declaration reaches, closed over the declarations each makes in
/// turn — under `tests/` or not. A file under `#![cfg(test)]` is dropped by
/// the compiler, so what it declares ships nothing.
fn shipped_files(
    scans: &BTreeMap<&str, FileScan>,
    tests: &BTreeSet<String>,
    targets: Vec<String>,
    failures: &mut Vec<String>,
) -> BTreeSet<String> {
    let mut shipped: BTreeSet<String> = BTreeSet::new();
    let mut queue: Vec<String> = scans
        .keys()
        .filter(|path| !tests.contains(**path) || shipped_root(path))
        .map(|path| (*path).to_owned())
        .chain(targets)
        .collect();
    while let Some(file) = queue.pop() {
        if !shipped.insert(file.clone()) {
            continue;
        }
        let Some(scan) = scans
            .get(file.as_str())
            .filter(|scan| !scan.whole_file_test)
        else {
            continue;
        };
        for decl in scan.decls.iter().filter(|decl| !decl.test) {
            let mut placed = false;
            for candidate in decl.candidates.iter().chain(&decl.also) {
                if !candidate.starts_with("crates/") {
                    placed = true;
                    failures.push(format!(
                        "{file}:{} declares `{}` at {candidate}, outside crates/, which this scan \
                         does not read",
                        decl.line, decl.name
                    ));
                } else if scans.contains_key(candidate.as_str()) {
                    placed = true;
                    queue.push(candidate.clone());
                }
            }
            // A SHIPPED DECLARATION THIS SCAN CANNOT PLACE IS REFUSED, never
            // passed over: `rustc` found its file somewhere (a name in another
            // case on a case-insensitive disk, a file that is not `.rs`, a
            // path this model of `rustc` gets wrong) or a `cfg` removes it.
            if !placed {
                failures.push(format!(
                    "{file}:{} declares `{}`, and no .rs file this scan reads is at {}: rustc \
                     would read it from somewhere this scan cannot place, or a cfg removes it; \
                     give it its file or remove the declaration",
                    decl.line,
                    decl.name,
                    decl.candidates.join(" or ")
                ));
            }
        }
    }
    shipped
}

/// Every failure of the tree `files` (path → text) against `allowed`, and how
/// many namings it found inside test code.
fn judge(files: &BTreeMap<String, String>, allowed: &[Allowed]) -> (Vec<String>, usize) {
    let mut failures = Vec::new();
    let mut scans: BTreeMap<&str, FileScan> = BTreeMap::new();
    let mut named_by: Vec<(&str, String)> = Vec::new();
    for (path, text) in files {
        if is_manifest(path) {
            let named = manifest_paths(path, text);
            named_by.extend(
                named
                    .targets
                    .into_iter()
                    .map(|target| (path.as_str(), target)),
            );
            failures.extend(named.unread);
            for outside in named
                .every
                .iter()
                .filter(|named| !named.starts_with("crates/"))
            {
                failures.push(format!(
                    "{path} names {outside}, outside crates/, which this scan does not read"
                ));
            }
        } else if is_rs(path) {
            match lex(text) {
                Ok(tokens) => {
                    let mut scan = FileScan::default();
                    pass(&tokens, false, &[], &place_of(path), &mut scan);
                    scans.insert(path, scan);
                }
                Err(why) => failures.push(format!("{path} cannot be read by this scan: {why}")),
            }
        }
    }
    // A manifest's `path` or `build` value is a target when a source is
    // there. A file of any other kind there is a target this scan cannot
    // read, and a `.rs` name with nothing there one it cannot place: both are
    // refused, never passed over. Anything else is a directory (a path
    // dependency), or nothing.
    let mut targets = Vec::new();
    // One outside `crates/` was refused above, by that name.
    let inside = named_by
        .into_iter()
        .filter(|(_, target)| target.starts_with("crates/"));
    for (manifest, target) in inside {
        if scans.contains_key(target.as_str()) {
            targets.push(target);
        } else if files.contains_key(target.as_str()) && !is_manifest(&target) {
            failures.push(format!(
                "{manifest} makes {target} a target, a file this scan does not read as Rust"
            ));
        } else if is_rs(&target) {
            failures.push(format!(
                "{manifest} makes {target} a target, and no .rs file this scan reads is there"
            ));
        }
    }
    let tests = test_files(&scans, files, &mut failures);
    let shipped = shipped_files(&scans, &tests, targets, &mut failures);

    // `include!` pastes a file's text where it stands, so a bare `#[test]` in
    // the pasted file names whatever `test` is in the file that pastes it,
    // where an import can re-point it (D-5246). Refused in shipped code.
    let pasted: BTreeSet<&str> = scans
        .values()
        .flat_map(|scan| &scan.decls)
        .filter(|decl| decl.name.starts_with("include!("))
        .flat_map(|decl| decl.candidates.iter().map(String::as_str))
        .collect();
    for (path, scan) in &scans {
        if pasted.contains(path) && shipped.contains(*path) && !scan.whole_file_test {
            for line in &scan.bare_tests {
                failures.push(format!(
                    "{path}:{line} is a fn only #[test] exempts, in a file an include! pastes: \
                     rustc resolves `test` in the file that pastes it, where an import can \
                     re-point the name, so this scan cannot tell it is test code"
                ));
            }
        }
    }

    for path in scans.keys() {
        if path.ends_with("_tests.rs") && !under_tests_dir(path) && !tests.contains(*path) {
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
        if scan.whole_file_test || !shipped.contains(*path) {
            in_tests += scan.found.len();
            continue;
        }
        for found in &scan.found {
            let function = found.function.clone().unwrap_or_else(|| "-".to_owned());
            by_function.entry((path, function)).or_default().push(found);
        }
    }
    failures.extend(verdict(&by_function, allowed));
    (failures, in_tests)
}

/// The failures of the shipped namings, grouped by file and function, against
/// `allowed`.
fn verdict(
    by_function: &BTreeMap<(&str, String), Vec<&Found>>,
    allowed: &[Allowed],
) -> Vec<String> {
    let mut failures = Vec::new();
    for ((path, function), found) in by_function {
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
    failures
}

// ------------------------------------------------------------ the real tree --

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every `.rs` file and every `Cargo.toml` under `root`'s `crates/`, and the
/// workspace's own `Cargo.toml` at `root`, keyed by its `/`-separated path
/// from `root` with its text; every other file there by the same key with no
/// text, so a target or a module file of another kind is known to be there
/// (and refused, never passed over); and every link under `crates/` by the
/// same key. Nothing is skipped by name; a link is named, never followed.
fn sources_under(root: &Path) -> (BTreeMap<String, String>, Vec<String>) {
    let mut out = BTreeMap::new();
    // The workspace manifest names the members, `[patch]` and path
    // dependencies this scan refuses outside `crates/`: one it cannot read is
    // a failure, never a manifest passed over.
    let workspace = root.join("Cargo.toml");
    let text = std::fs::read_to_string(&workspace)
        .unwrap_or_else(|why| panic!("{} cannot be read: {why}", workspace.display()));
    out.insert("Cargo.toml".to_owned(), text);
    let mut links = Vec::new();
    let mut dirs = vec![root.join("crates")];
    while let Some(dir) = dirs.pop() {
        let listing = std::fs::read_dir(&dir)
            .unwrap_or_else(|why| panic!("{} cannot be listed: {why}", dir.display()));
        for entry in listing {
            let entry = entry.expect("a directory entry");
            let kind = entry.file_type().expect("a file type");
            let path = entry.path();
            let relative = normal(path.strip_prefix(root).expect("under the root"));
            let wanted = path.extension().is_some_and(|ext| ext == "rs")
                || entry.file_name() == "Cargo.toml";
            if kind.is_symlink() {
                links.push(relative);
            } else if kind.is_dir() {
                dirs.push(path);
            } else if kind.is_file() && wanted {
                let text = std::fs::read_to_string(&path)
                    .unwrap_or_else(|why| panic!("{} cannot be read: {why}", path.display()));
                out.insert(relative, text);
            } else {
                out.insert(relative, String::new());
            }
        }
    }
    links.sort();
    (out, links)
}

/// **THE GATE.** No shipped code under `crates/` names the process-spawning
/// type except in [`ALLOWED`]'s functions, each its own number of times.
#[test]
fn no_shipped_code_starts_a_program_outside_the_allowlist() {
    let (files, links) = sources_under(&root());
    let sources: Vec<&String> = files.keys().filter(|path| is_rs(path)).collect();
    assert!(
        sources.len() > 500,
        "only {} source files were found under crates/ — the walk stopped walking",
        sources.len()
    );
    assert!(
        links.is_empty(),
        "links under crates/, which this scan does not follow, so a module behind one would go \
         unread (D-5244): {links:?}"
    );
    // EVERY MANIFEST IS READ, the workspace's and each crate's, or a target
    // or a path it names would go unread.
    assert!(
        files.contains_key("Cargo.toml"),
        "the workspace's Cargo.toml was not read"
    );
    let crates: BTreeSet<&str> = sources
        .iter()
        .filter_map(|path| path.split('/').nth(1))
        .collect();
    let manifests: BTreeSet<&str> = files.keys().filter_map(|p| manifest_crate(p)).collect();
    assert_eq!(crates, manifests, "a crate's Cargo.toml was not read");
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
/// allowed, each once, and each file still defines its function, inside the
/// `impl` its path names. A third entry fails here before it can excuse
/// anything (D-5243).
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
                "<Notify for Opener>::notify",
                1,
                "opens the watchdog's notice page in the operator's own browser, never a build \
                 or data step (D-4996)",
            ),
        ]
    );
    for entry in ALLOWED {
        let text = std::fs::read_to_string(root().join(entry.path)).expect(entry.path);
        let (outer, function) = entry
            .function
            .rsplit_once("::")
            .unwrap_or(("", entry.function));
        assert!(
            text.contains(&format!("fn {function}(")),
            "{} no longer defines fn {function}",
            entry.path
        );
        if let Some(header) = outer.strip_prefix('<').and_then(|h| h.strip_suffix('>')) {
            assert!(
                text.contains(&format!("impl {header} {{")),
                "{} no longer has impl {header}",
                entry.path
            );
        }
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
#[expect(
    clippy::too_many_lines,
    reason = "one table of spellings, each planted and each with its verdict"
)]
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
        // D-5244. The module renamed or globbed inside an import group, from
        // `std` or `tokio`, at any nesting.
        format!("use std::{MODULE}::{{self as q}};\nfn run() {{ q::{TYPE}::new(\"sh\"); }}"),
        format!("use tokio::{MODULE}::{{self as tp}};\nfn run() {{}}"),
        format!("use std::{MODULE}::{{*}};\nfn run() {{ {TYPE}::new(\"sh\"); }}"),
        format!("use std::{{io, {MODULE}::{{Stdio, {{self as q}}}}}};\nfn run() {{}}"),
        // A macro variable joined to either half, two joined, or a `use`
        // built from one.
        format!(
            "macro_rules! mk {{ ($t:ident) => {{ std::{MODULE}::$t::new(\"sh\") }} }}\n\
             fn run() {{ let _ = mk!({TYPE}); }}"
        ),
        format!(
            "macro_rules! mk {{ ($m:ident) => {{ std::$m::{TYPE}::new(\"sh\") }} }}\n\
             fn run() {{ let _ = mk!({MODULE}); }}"
        ),
        "macro_rules! mk { ($m:ident, $t:ident) => { std::$m::$t::new(\"sh\") } }\nfn run() {}"
            .to_owned(),
        "macro_rules! mk { ($m:path) => { use $m as q; } }\nfn run() {}".to_owned(),
        // A macro handed both halves apart, at the call site.
        format!("fn run() {{ let _ = other_crate::mk!(std, {MODULE}, {TYPE}); }}"),
        // A path through the module this scan cannot read.
        format!("fn run() {{ std::{MODULE}::[<{TYPE}>]::new(\"sh\"); }}"),
        // A shebang line is dropped as `rustc` drops it, a quote with it.
        format!("#!/usr/bin/env \"x\nfn run() {{ {spawn}; }} // \"\n"),
        format!("\u{feff}#! shebang \"\nfn run() {{ {spawn}; }} // \"\n"),
        // An attribute is read for what it names.
        format!("#[a(std::{MODULE}::{TYPE})]\nfn run() {{}}"),
        // An `include!` of no plain string names no file to read.
        "include!(concat!(env!(\"OUT_DIR\"), \"/g.rs\"));\nfn run() {}".to_owned(),
        // A generated segment before the type, and a module file a macro
        // variable names, are text this scan cannot read.
        format!("fn run() {{ std::[<{MODULE}>]::{TYPE}::new(\"sh\"); }}"),
        "macro_rules! m { ($n:ident) => { mod $n; } }\nfn run() {}".to_owned(),
        // Repair round 1b (D-5244). A repetition joins a path as well as a
        // variable does: the type after one (`$($s::)*<type>`), after `::`
        // that follows one (`$(::$b)*::<type>`, `$($p)*::<type>`), straight
        // after a variable (which can be `::`), and the module before one.
        format!(
            "macro_rules! b {{ ($($s:ident)*) => {{ $($s::)*{TYPE}::new(\"sh\") }} }}\n\
             fn run() {{ let _ = b!(std {MODULE}); }}"
        ),
        format!(
            "macro_rules! b {{ ($a:ident $($b:ident)*) => {{ $a $(::$b)*::{TYPE}::new(\"sh\") }} }}\n\
             fn run() {{ let _ = b!(std {MODULE}); }}"
        ),
        format!(
            "macro_rules! b {{ ($($p:tt)*) => {{ $($p)*::{TYPE}::new(\"sh\") }} }}\n\
             fn run() {{ let _ = b!(std::{MODULE}); }}"
        ),
        format!(
            "macro_rules! b {{ ($s:tt $t:tt) => {{ std::{MODULE} $s $t::new(\"sh\") }} }}\n\
             fn run() {{ let _ = b!(:: {TYPE}); }}"
        ),
        format!(
            "macro_rules! b {{ ($($t:ident)*) => {{ std::{MODULE} $(::$t)*::new(\"sh\") }} }}\n\
             fn run() {{ let _ = b!({TYPE}); }}"
        ),
        format!("macro_rules! b {{ ($s:tt) => {{ $s {TYPE} }} }}\nfn run() {{}}"),
        // A macro body that hands one half on beside a variable carrying the
        // other, to a macro that joins them, is refused where it hands it.
        format!(
            "macro_rules! j {{ ($($s:ident)*) => {{ $($s)::*::new(\"sh\") }} }}\n\
             macro_rules! b {{ ($($s:ident)*) => {{ j!($($s)* {TYPE}) }} }}\n\
             fn run() {{ let _ = b!(std {MODULE}); }}"
        ),
        format!(
            "macro_rules! b {{ ($t:ident) => {{ j!(std {MODULE}, $t) }} }}\n\
             fn run() {{ let _ = b!({TYPE}); }}"
        ),
        format!(
            "macro_rules! b {{ ($($s:ident)*) => {{ k!({TYPE}, $($s)*) }} }}\n\
             fn run() {{ let _ = b!(std {MODULE}); }}"
        ),
        // The two direction marks are whitespace to `rustc`, which the path
        // goes on across.
        format!("fn run() {{ std::{MODULE}\u{200e}::{TYPE}::new(\"sh\"); }}"),
        format!("fn run() {{ std::{MODULE}::\u{200f}{TYPE}::new(\"sh\"); }}"),
        // `\r\n` is `\n` to `rustc`, so a line continuation ends there too.
        format!("fn run() {{ let _s = \"x\\\r\n   y\"; {spawn}; }}\r\n"),
        // `include!` under another name pastes a file this scan does not see
        // being pasted.
        "use std::include as paste;\nfn run() {}".to_owned(),
        // Repair round 2 (D-5246). rustc 1.97.1 (20 Oct 2026) compiled each
        // shape below without `--test`, the program kept, unless its comment
        // says otherwise; the closure with a second parameter and the tuple
        // field were refused before too. A gated generic
        // parameter (a type, a const or a lifetime; on a fn, a struct, an
        // impl or a trait) ends at the `>` closing the list, and a closure's
        // gated parameter at its `|`: the body after them is shipped.
        format!("pub fn run<#[cfg(test)] T>() {{ {spawn}; }}"),
        format!("pub fn run<#[cfg(test)] const N: usize>() {{ {spawn}; }}"),
        format!("pub fn run<#[cfg(test)] 'a>() {{ {spawn}; }}"),
        format!("pub fn run<A, #[cfg(test)] const N: usize>() {{ {spawn}; }}"),
        format!("pub struct S;\nimpl<#[cfg(test)] 'a> S {{ pub fn run() {{ {spawn}; }} }}"),
        format!("pub struct S;\nimpl<#[cfg(test)] T> S {{ pub fn run() {{ {spawn}; }} }}"),
        format!("pub struct S<#[cfg(test)] T> {{ pub c: std::{MODULE}::{TYPE} }}"),
        format!("pub trait Tr<#[cfg(test)] T> {{ fn run() {{ {spawn}; }} }}"),
        format!("pub fn run() {{ let f = |#[cfg(test)] a: u8| {{ {spawn}; }}; let _ = f; }}"),
        format!(
            "pub fn run() {{ let f = |#[cfg(test)] a: u8, b: u8| {{ {spawn}; }}; let _ = f; }}"
        ),
        // `union`, `auto` and `macro_rules` begin an item only in item
        // syntax; a field, an argument, a struct expression's field, a
        // variant or a binding named by one ends at its comma.
        format!("pub struct S {{ #[cfg(test)] union: u8, pub c: std::{MODULE}::{TYPE} }}"),
        format!("pub struct S {{ #[cfg(test)] pub union: u8, pub c: std::{MODULE}::{TYPE} }}"),
        format!(
            "pub struct S {{ #[cfg(test)] union: u8, pub c: u8 }}\n\
             pub fn run() -> S {{ S {{ #[cfg(test)] union: 1, c: {{ {spawn}; 0 }} }} }}"
        ),
        format!("pub fn run(#[cfg(test)] union: u8, c: std::{MODULE}::{TYPE}) {{ let _ = c; }}"),
        format!("pub struct S {{ #[cfg(test)] macro_rules: u8, pub c: std::{MODULE}::{TYPE} }}"),
        format!("pub struct S {{ #[cfg(test)] auto: u8, pub c: std::{MODULE}::{TYPE} }}"),
        format!("pub enum E {{ #[cfg(test)] union, B(std::{MODULE}::{TYPE}) }}"),
        format!("pub enum E {{ V {{ #[cfg(test)] union: u8, c: std::{MODULE}::{TYPE} }} }}"),
        format!(
            "pub fn run(#[cfg(test)] macro_rules: u8, c: std::{MODULE}::{TYPE}) {{ let _ = c; }}"
        ),
        format!(
            "pub fn run(x: u8) {{ match x {{ #[cfg(test)] union => 0, _ => {{ {spawn}; 1 }} }}; }}"
        ),
        format!("pub struct S(#[cfg(test)] pub u8, pub std::{MODULE}::{TYPE});"),
        // An item a macro is handed in a list ends at its comma (macro input,
        // so no binary: the shape is planted, not compiled).
        format!("m! {{ #[cfg(test)] struct A, struct B(std::{MODULE}::{TYPE}) }}\nfn run() {{}}"),
        // A bare `#[test]` is the built-in one only on a free fn directly in
        // a module, in a file nothing in which can re-point the name. rustc
        // compiled each of the next eight with the program kept, the name
        // re-pointed (by `tokio::main`, or a proc-macro crate `pm` standing in
        // for any attribute macro; the `impl` and the macro body with `use
        // pm::passthru as test;` beside them). The paste needs its file, and
        // rustc refuses the built-in `#[test]` on a statement and `#![test]`
        // outright, so those three plant only that this scan errs towards
        // failing.
        format!("use tokio::main as test;\n#[test]\npub async fn launch() {{ {spawn}; }}"),
        format!("use pm::{{passthru as r#test}};\n#[test]\npub fn launch() {{ {spawn}; }}"),
        format!("use pm::test;\n#[test]\npub fn launch() {{ {spawn}; }}"),
        format!(
            "pub fn run() {{ use pm::passthru as test; #[test] fn inner() {{ {spawn}; }} inner(); }}"
        ),
        format!("#[macro_use]\nextern crate pm;\n#[test]\npub fn launch() {{ {spawn}; }}"),
        format!("#[derive(pm::D)]\npub struct S {{ #[test] pub c: std::{MODULE}::{TYPE} }}"),
        format!("pub struct S;\nimpl S {{ #[test] pub fn run() {{ {spawn}; }} }}"),
        format!("macro_rules! m {{ () => {{ #[test] pub fn launch() {{ {spawn}; }} }} }}"),
        format!("include!(\"h.rs\");\n#[test]\npub fn launch() {{ {spawn}; }}"),
        format!("pub fn run() {{ #[test] let _a = 1; {spawn}; }}"),
        format!("#![test]\npub fn run() {{ {spawn}; }}"),
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
    // The shebang line is dropped and the file's line numbers are kept.
    let shebang = planted(&format!(
        "#!/usr/bin/env \"x\nfn run() {{ {spawn}; }} // \"\n"
    ));
    assert_eq!(
        shebang,
        vec![
            "crates/x/src/lib.rs:2 in fn run names the process-spawning type: a process spawn \
             outside test code is not allowed"
                .to_owned()
        ]
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
        // `#!` before `[`, past whitespace or a plain comment, is no shebang:
        // it is an inner attribute, and read as one.
        format!("#! /* c */ [cfg(test)]\nfn t() {{ {spawn}; }}"),
        format!("#!\n[cfg(test)]\nfn t() {{ {spawn}; }}"),
        // An attribute on test code is test code.
        format!("#[cfg(test)]\n#[a(std::{MODULE}::{TYPE})]\nfn t() {{}}"),
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
        // `self` in a group is the module itself, which the path then names;
        // `$crate` is the defining crate; a macro handed the module as a path,
        // or another type of the same name, joins nothing; `use<..>` in a
        // bound is no `use` declaration.
        format!(
            "use std::{MODULE}::{{self, Stdio}};\n\
             macro_rules! mk {{ ($x:ident) => {{ $crate::{TYPE}::new($x) }} }}\n\
             fn run() {{ go!({MODULE}::exit(0), {TYPE}::Serve); }}\n\
             macro_rules! cap {{ ($b:block) => {{ fn f<'a>() -> impl Sized + use<'a> $b }} }}"
        ),
        // An associated item of the same name behind a qualified path is not
        // the module's type.
        format!("fn run() {{ <S as T>::{TYPE}::new(); }}"),
        // An enum of the same name, used the ways `api::sweeprun` uses its
        // own, and a macro handing the module's `exit` a variable, join
        // nothing (repair round 1b).
        format!(
            "pub enum {TYPE} {{ A, B }}\n\
             fn run(c: &{TYPE}) -> Result<{TYPE}, u8> {{\n\
                 match c {{ {TYPE}::A | {TYPE}::B => Ok(self::{TYPE}::A) }}\n\
             }}\n\
             fn kind() -> u8 {{ k(crate::sweeprun::Kind::{TYPE}) }}\n\
             macro_rules! q {{ ($c:expr) => {{ std::{MODULE}::exit($c) }} }}"
        ),
        // Repair round 2 (D-5246): what a gated parameter, field or variant
        // holds is test code, up to the `>`, `|` or `,` that ends it.
        format!("pub fn run<#[cfg(test)] T: Into<std::{MODULE}::{TYPE}>>() {{}}"),
        format!("pub fn run<A, #[cfg(test)] T: Fn() -> std::{MODULE}::{TYPE}>() {{}}"),
        format!("pub fn run(#[cfg(test)] c: std::{MODULE}::{TYPE}, b: u8) {{ let _ = b; }}"),
        format!(
            "pub fn run() {{ let f = |#[cfg(test)] c: std::{MODULE}::{TYPE}, b: u8| b; let _ = f; }}"
        ),
        format!("pub struct S {{ #[cfg(test)] pub union: std::{MODULE}::{TYPE}, pub b: u8 }}"),
        format!("pub enum E {{ #[cfg(test)] union(std::{MODULE}::{TYPE}), B }}"),
        format!(
            "pub fn run(x: u8) -> u8 {{ match x {{ #[cfg(test)] union => {{ {spawn}; 0 }} _ => 1 }} }}"
        ),
        // An item a contextual keyword begins is still an item.
        format!("#[cfg(test)]\nunion U {{ a: u8, b: std::{MODULE}::{TYPE} }}"),
        format!("#[cfg(test)]\npub union U<T> {{ a: u8, b: std::{MODULE}::{TYPE} }}"),
        format!("#[cfg(test)]\nmacro_rules! m {{ () => {{ {spawn} }} }}"),
        // An item's own commas, in its generics and its `where` clause, do
        // not end it.
        format!("#[cfg(test)]\nfn t<A, B>() -> Result<A, B> where A: Copy, B: Copy {{ {spawn}; }}"),
        format!("#[cfg(test)]\ntype M = std::collections::HashMap<u8, std::{MODULE}::{TYPE}>;"),
        // The built-in `#[test]` on a free fn, in the file or an inline
        // module, where nothing re-points the name; an unrelated `use`.
        format!("use std::fmt;\nmod tests {{\n    #[test]\n    fn t() {{ {spawn}; }}\n}}"),
        format!("#![allow(dead_code)]\n#[test]\npub const unsafe fn t() {{ {spawn}; }}"),
    ];
    for src in &exempt {
        assert_eq!(planted(src), Vec::<String>::new(), "refused:\n{src}");
    }
    // And the exempt spawns were seen, not missed.
    let (_, seen) = judge(&tree(&[("crates/x/src/lib.rs", &exempt[0])]), &[]);
    assert_eq!(seen, 1);
    let (_, seen) = judge(&tree(&[("crates/x/src/lib.rs", &exempt[10])]), &[]);
    assert_eq!(seen, 1);

    // Repair round 2 (D-5246): the planted shapes fail by file, line and
    // function, not merely somewhere.
    let at = |src: &str| -> Vec<String> {
        planted(src)
            .into_iter()
            .filter_map(|f| f.split(" names ").next().map(str::to_owned))
            .collect()
    };
    assert_eq!(
        at(&format!(
            "pub fn run<#[cfg(test)] T>() {{\n    {spawn};\n}}"
        )),
        ["crates/x/src/lib.rs:2 in fn run"]
    );
    assert_eq!(
        at(&format!(
            "pub struct S {{\n    #[cfg(test)] union: u8,\n    pub c: std::{MODULE}::{TYPE},\n}}"
        )),
        ["crates/x/src/lib.rs:3 in fn -"]
    );
    // A naming in a signature is in that function.
    assert_eq!(
        at(&format!(
            "pub fn run(\n    #[cfg(test)] union: u8,\n    c: std::{MODULE}::{TYPE},\n) {{}}"
        )),
        ["crates/x/src/lib.rs:3 in fn run"]
    );
    assert_eq!(
        at(&format!(
            "mod m {{ pub trait Tr {{\n    fn go(c: std::{MODULE}::{TYPE});\n}} }}\nfn after() {{}}"
        )),
        ["crates/x/src/lib.rs:2 in fn m::Tr::go"]
    );
    assert_eq!(
        at(&format!(
            "use tokio::main as test;\n#[test]\npub async fn launch() {{\n    {spawn};\n}}"
        )),
        ["crates/x/src/lib.rs:4 in fn launch"]
    );
    assert_eq!(
        planted("#[macro_use]\nextern crate pm;"),
        [
            "crates/x/src/lib.rs:2 in fn - loads a crate's macros with #[macro_use] extern crate, \
          which re-points #[test] in every file of the crate when that crate has a macro named \
          test: this scan cannot tell what #[test] names there: a process spawn outside test \
          code is not allowed"
        ]
    );
    // A file an `include!` pastes into shipped code resolves `test` where it
    // is pasted, so a bare `#[test]` there is refused by name; pasted into
    // test code, it is test code like the rest of that file.
    let pasted = |lib: &str| {
        judge(
            &tree(&[
                ("crates/x/src/lib.rs", lib),
                ("crates/x/src/h.rs", "#[test]\npub fn launch() {}"),
            ]),
            &[],
        )
        .0
    };
    assert_eq!(
        pasted("use pm::passthru as test;\ninclude!(\"h.rs\");"),
        [
            "crates/x/src/h.rs:2 is a fn only #[test] exempts, in a file an include! pastes: rustc \
          resolves `test` in the file that pastes it, where an import can re-point the name, so \
          this scan cannot tell it is test code"
        ]
    );
    assert_eq!(
        pasted("#[cfg(test)]\nmod tests { include!(\"h.rs\"); }"),
        Vec::<String>::new()
    );
    // And `mod h;` is no paste: the module resolves `test` for itself.
    assert_eq!(pasted("mod h;"), Vec::<String>::new());

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

/// **WHAT SHIPPED CODE REACHES IS SHIPPED, EVEN UNDER `tests/`** (D-5244): a
/// shipped `mod` by `#[path]` or `cfg_attr`, an `include!`, a module beside a
/// `#[path]`-loaded file, and a manifest target all ship the file they name;
/// only `#![cfg(test)]` keeps a declared file test code; a reach outside
/// `crates/` is refused by name.
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one table of spellings, each planted and each with its verdict"
)]
fn what_shipped_code_reaches_is_shipped_even_under_tests() {
    let spawn = format!("pub fn go() {{ std::{MODULE}::{TYPE}::new(\"x\"); }}");
    let quiet = |files: &[(&str, &str)]| judge(&tree(files), &[]).0;
    let refused = |file: &str| {
        format!(
            "{file}:1 in fn go names the process-spawning type: a process spawn outside test \
             code is not allowed"
        )
    };

    for lib in [
        "#[path = \"../tests/h.rs\"] pub mod h;",
        "#[cfg_attr(unix, path = \"../tests/h.rs\")] mod h;",
        "include!(\"../tests/h.rs\");",
        // Repair round 1b: any delimiter pastes, and a path's escapes are
        // decoded as `rustc` decodes them.
        "include![\"../tests/h.rs\"];",
        "include!{\"../tests/h.rs\"}",
        "#[path = \"..\\x2ftests/h.rs\"] mod h;",
        "#[path = \"..\\u{2f}tests/h.rs\"] mod h;",
        "#[path = \"../te\\\n    sts/h.rs\"] mod h;",
        // A raw identifier is the same name to `rustc` (`r#include!` and
        // `#[r#path]` both work, rustc 1.97.1), and of two `#[path]`s it
        // follows the first.
        "#[r#path = \"../tests/h.rs\"] mod h;",
        "r#include!(\"../tests/h.rs\");",
        "#[path = \"../tests/h.rs\"]\n#[path = \"h.rs\"]\nmod h;",
    ] {
        assert_eq!(
            quiet(&[
                ("crates/x/src/lib.rs", lib),
                ("crates/x/tests/h.rs", &spawn)
            ]),
            vec![refused("crates/x/tests/h.rs")],
            "{lib}"
        );
    }
    // A `#[path]`-loaded file that is no `mod.rs` by name is one to `rustc`:
    // a `#[path]` inside its inline module is read without its stem
    // (`src/inl/../../tests/h.rs`, rustc 1.97.1, 20 Oct 2026), so a file at
    // the stem's place cannot stand in for it.
    assert_eq!(
        quiet(&[
            ("crates/x/src/lib.rs", "#[path = \"b.rs\"] mod q;"),
            (
                "crates/x/src/b.rs",
                "pub mod inl { #[path = \"../../tests/h.rs\"] pub mod h; }"
            ),
            ("crates/x/src/tests/h.rs", ""),
            ("crates/x/tests/h.rs", &spawn),
        ]),
        vec![refused("crates/x/tests/h.rs")]
    );
    // Closed over what the reached file declares: a `#[path]`-loaded file is
    // a `mod.rs` to `rustc`, its modules beside it, wherever it sits.
    assert_eq!(
        quiet(&[
            (
                "crates/x/src/lib.rs",
                "#[path = \"../tests/sub/h.rs\"] mod h;"
            ),
            ("crates/x/tests/sub/h.rs", "mod d;"),
            ("crates/x/tests/sub/d.rs", &spawn),
        ]),
        vec![refused("crates/x/tests/sub/d.rs")]
    );
    // A crate root ships whatever names it: a test that `#[path]`s the library,
    // a binary or the build script in, test-only or not, exempts none of them,
    // nor the modules they declare.
    assert_eq!(
        quiet(&[
            (
                "crates/x/tests/it.rs",
                "#[path = \"../src/lib.rs\"] mod l;\n#[path = \"../build.rs\"] mod b;"
            ),
            (
                "crates/x/src/main.rs",
                "#[cfg(test)]\n#[path = \"bin/t.rs\"]\nmod t;"
            ),
            ("crates/x/src/lib.rs", &format!("mod m;\n{spawn}")),
            ("crates/x/src/m.rs", &spawn),
            ("crates/x/build.rs", &spawn),
            ("crates/x/src/bin/t.rs", &spawn),
        ]),
        vec![
            refused("crates/x/build.rs"),
            refused("crates/x/src/bin/t.rs"),
            "crates/x/src/lib.rs:2 in fn go names the process-spawning type: a process spawn \
             outside test code is not allowed"
                .to_owned(),
            refused("crates/x/src/m.rs"),
        ]
    );
    // A file under `#![cfg(test)]` stays test code when shipped code declares
    // it, and so does what it declares.
    assert_eq!(
        quiet(&[
            ("crates/x/src/lib.rs", "mod whole_tests;"),
            (
                "crates/x/src/whole_tests.rs",
                &format!("#![cfg(test)]\nmod more;\n{spawn}")
            ),
            ("crates/x/src/whole_tests/more.rs", &spawn),
        ]),
        Vec::<String>::new()
    );
    // A manifest target ships its file, `[lib]`, `[[bin]]` and `build` alike;
    // a `[[test]]` target does not.
    let manifest = "[package]\nname = \"x\"\nbuild = \"tools/gen.rs\"\n\n[lib]\n\
                    path = \"tests/lib.rs\"\n\n[[bin]]\nname = \"b\"\npath = 'tests/b.rs'\n\n\
                    [[test]]\nname = \"t\"\npath = \"tests/t.rs\"\n\n[dependencies]\n\
                    core = { path = \"../core\" }\n";
    assert_eq!(
        quiet(&[
            ("crates/x/Cargo.toml", manifest),
            ("crates/x/tests/lib.rs", &spawn),
            ("crates/x/tests/b.rs", &spawn),
            ("crates/x/tests/t.rs", &spawn),
            ("crates/x/tools/gen.rs", &spawn),
        ]),
        vec![
            refused("crates/x/tests/b.rs"),
            refused("crates/x/tests/lib.rs"),
            refused("crates/x/tools/gen.rs"),
        ]
    );
    // A reach outside `crates/` is refused by name, from code or a manifest.
    assert_eq!(
        quiet(&[(
            "crates/x/src/lib.rs",
            "#[path = \"../../../../elsewhere.rs\"] mod e;"
        )]),
        vec![
            "crates/x/src/lib.rs:1 declares `e` at elsewhere.rs, outside crates/, which this \
             scan does not read"
                .to_owned()
        ]
    );
    for (manifest, text, outside) in [
        (
            "crates/x/Cargo.toml",
            "[lib]\npath = \"../../lib.rs\"\n",
            "lib.rs",
        ),
        (
            "crates/x/Cargo.toml",
            "[dependencies]\nv = { path = \"../../vendor/v\" } # a vendored crate\n",
            "vendor/v",
        ),
        (
            "Cargo.toml",
            "[workspace]\nmembers = [\"crates/x\"]\n\n[patch.crates-io]\nv = { path = \"vendor/v\" }\n",
            "vendor/v",
        ),
    ] {
        assert_eq!(
            quiet(&[(manifest, text)]),
            vec![format!(
                "{manifest} names {outside}, outside crates/, which this scan does not read"
            )],
            "{text}"
        );
    }
    // A path dependency inside `crates/`, and a comment, name nothing outside.
    assert_eq!(
        quiet(&[(
            "crates/x/Cargo.toml",
            "[dependencies]\ncore = { path = \"../core\" } # path = \"../../not/read\"\n"
        )]),
        Vec::<String>::new()
    );
    // Repair round 1b. A manifest is read as TOML is: a quoted key, a dotted
    // key, a `#` inside a string, an escape, a nested package's manifest,
    // and a value that spells `["test"]` on a line of its own, which is no
    // table header and moves nothing into a `[[test]]`.
    for (manifest, text, target) in [
        (
            "crates/x/Cargo.toml",
            "[lib]\n\"path\" = \"tests/h.rs\"\n",
            "crates/x/tests/h.rs",
        ),
        (
            "crates/x/Cargo.toml",
            "lib.path = 'tests/h.rs'\n",
            "crates/x/tests/h.rs",
        ),
        (
            "crates/x/Cargo.toml",
            "[lib]\npath = \"tests/h#x.rs\" # a comment\n",
            "crates/x/tests/h#x.rs",
        ),
        (
            "crates/x/Cargo.toml",
            "[lib]\npath = \"tests\\u002Fh.rs\"\n",
            "crates/x/tests/h.rs",
        ),
        (
            "crates/x/sub/Cargo.toml",
            "[lib]\npath = \"../tests/h.rs\"\n",
            "crates/x/tests/h.rs",
        ),
        (
            "crates/x/Cargo.toml",
            "[lib]\nx = [\n[\"test\"]]\npath = \"tests/h.rs\"\n",
            "crates/x/tests/h.rs",
        ),
    ] {
        assert_eq!(
            quiet(&[(manifest, text), (target, &spawn)]),
            vec![refused(target)],
            "{text}"
        );
    }
    // What a manifest says that this scan cannot read, or cannot place, is
    // refused by name: a multi-line string, a target of another kind, a
    // target with no source there, and a workspace member outside `crates/`.
    for (files, failure) in [
        (
            vec![(
                "crates/x/Cargo.toml",
                "[lib]\npath = \"\"\"tests/h.rs\"\"\"\n",
            )],
            "crates/x/Cargo.toml:2 gives `path` a multi-line or unclosed string, which this \
             scan does not read",
        ),
        (
            vec![
                (
                    "crates/x/Cargo.toml",
                    "[[bin]]\nname = \"b\"\npath = \"tests/b.md\"\n",
                ),
                ("crates/x/tests/b.md", ""),
            ],
            "crates/x/Cargo.toml makes crates/x/tests/b.md a target, a file this scan does not \
             read as Rust",
        ),
        (
            vec![("crates/x/Cargo.toml", "[lib]\npath = \"tests/Lib.rs\"\n")],
            "crates/x/Cargo.toml makes crates/x/tests/Lib.rs a target, and no .rs file this \
             scan reads is there",
        ),
        (
            vec![(
                "Cargo.toml",
                "[workspace]\nmembers = [\n    \"crates/x\", # one\n    \"tools/y\",\n]\n",
            )],
            "Cargo.toml names tools/y, outside crates/, which this scan does not read",
        ),
    ] {
        assert_eq!(quiet(&files), vec![failure.to_owned()], "{files:?}");
    }
}

/// **WHAT THIS SCAN CANNOT PLACE IS REFUSED BY NAME** (repair round 1b,
/// D-5244): a shipped declaration with no `.rs` file where this model of
/// `rustc` looks (a name in another case, which the operator's
/// case-insensitive disk would find, a file of another kind), an inline
/// module given a `#[path]`, and an `include!` or a module file inside a
/// macro body, which `rustc` reads from wherever the macro is called.
#[test]
fn what_this_scan_cannot_place_is_refused_by_name() {
    let spawn = format!("pub fn go() {{ std::{MODULE}::{TYPE}::new(\"x\"); }}");
    let quiet = |files: &[(&str, &str)]| judge(&tree(files), &[]).0;
    let placed = |file: &str, line: usize, name: &str, at: &str| {
        format!(
            "{file}:{line} declares `{name}`, and no .rs file this scan reads is at {at}: rustc \
             would read it from somewhere this scan cannot place, or a cfg removes it; give it \
             its file or remove the declaration"
        )
    };
    assert_eq!(
        quiet(&[
            ("crates/x/src/lib.rs", "#[path = \"../Tests/H.rs\"] mod h;"),
            ("crates/x/tests/h.rs", &spawn),
        ]),
        vec![placed("crates/x/src/lib.rs", 1, "h", "crates/x/Tests/H.rs")]
    );
    assert_eq!(
        quiet(&[
            ("crates/x/src/lib.rs", "mod n;\ninclude!(\"../notes.md\");"),
            ("crates/x/notes.md", ""),
        ]),
        vec![
            placed(
                "crates/x/src/lib.rs",
                1,
                "n",
                "crates/x/src/n.rs or crates/x/src/n/mod.rs"
            ),
            placed(
                "crates/x/src/lib.rs",
                2,
                "include!(\"../notes.md\")",
                "crates/x/notes.md"
            ),
        ]
    );
    // An inline module with a `#[path]` reads its modules from that path,
    // which this scan does not follow.
    let inline = quiet(&[
        (
            "crates/x/src/lib.rs",
            "#[path = \"../tests\"]\nmod t { pub mod h; }",
        ),
        ("crates/x/tests/h.rs", &spawn),
    ]);
    assert!(
        inline.contains(
            &"crates/x/src/lib.rs:2 in fn - gives an inline module a #[path], which moves where \
              the module files inside it are read from: this scan does not follow it: a process \
              spawn outside test code is not allowed"
                .to_owned()
        ),
        "{inline:?}"
    );
    // So does an inner `#![path]` (`mod t { #![path = "../tests"] mod h; }`
    // reads `tests/h.rs`, rustc 1.97.1), even with a file where this scan
    // would look.
    let inner = quiet(&[
        (
            "crates/x/src/lib.rs",
            "mod t {\n    #![path = \"../tests\"]\n    pub mod h;\n}",
        ),
        ("crates/x/src/t/h.rs", ""),
        ("crates/x/tests/h.rs", &spawn),
    ]);
    assert_eq!(
        inner,
        vec![
            "crates/x/src/lib.rs:2 in fn - gives a module an inner #![path], which moves where \
             its module files are read from: this scan does not follow it: a process spawn \
             outside test code is not allowed"
                .to_owned()
        ]
    );
}

/// **WHAT A MACRO BODY PASTES OR DECLARES IS REFUSED BY NAME** (repair round
/// 1b, D-5244): `rustc` reads an `include!` path, and a module file, from
/// wherever the macro is CALLED, which may be another file in another
/// directory, so this scan, reading the body where it is written, cannot
/// place either. The same in test code is counted, never refused.
#[test]
fn what_a_macro_body_pastes_is_refused_by_name() {
    let spawn = format!("pub fn go() {{ std::{MODULE}::{TYPE}::new(\"x\"); }}");
    let quiet = |files: &[(&str, &str)]| judge(&tree(files), &[]).0;
    for body in [
        "macro_rules! paste { () => { include!(\"../tests/h.rs\"); } }",
        "macro_rules! declare { () => { #[path = \"../tests/h.rs\"] mod h; } }",
    ] {
        let failures = quiet(&[
            (
                "crates/x/src/lib.rs",
                "mod deep { #[macro_use] pub mod a; }",
            ),
            ("crates/x/src/deep/a.rs", body),
            ("crates/x/tests/h.rs", &spawn),
        ]);
        assert!(
            failures.contains(
                &"crates/x/src/deep/a.rs:1 in fn - pastes a file or declares a module file \
                  inside a macro body, which rustc reads from wherever the macro is called: this \
                  scan cannot place it: a process spawn outside test code is not allowed"
                    .to_owned()
            ),
            "{body}: {failures:?}"
        );
    }
    // The same in test code is test code, counted and not refused.
    let (failures, seen) = judge(
        &tree(&[
            (
                "crates/x/src/lib.rs",
                "#[cfg(test)]\nmod t { macro_rules! p { () => { include!(\"h.rs\"); } } }",
            ),
            ("crates/x/src/h.rs", ""),
        ]),
        &[],
    );
    assert_eq!((failures, seen), (Vec::<String>::new(), 1));
}

/// **THE ALLOWLIST IS EXACT**: its function's whole path, its file and its
/// count, and an entry that excuses nothing is itself a failure.
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one table of spellings, each planted and each with its verdict"
)]
fn the_allowlist_is_exact_about_file_function_and_count() {
    let allowed = [
        Allowed {
            path: "crates/x/src/lib.rs",
            function: "open",
            count: 1,
            reason: "a fixture",
        },
        Allowed {
            path: "crates/x/src/notice.rs",
            function: "<Notify for Opener>::notify",
            count: 1,
            reason: "a fixture",
        },
    ];
    let spawn = format!("std::{MODULE}::{TYPE}::new(\"x\")");
    let notice = format!("impl Notify for Opener {{ fn notify(&self) {{ {spawn}; }} }}");
    let verdict = |src: &str, path: &str| {
        let notice_file = ("crates/x/src/notice.rs", notice.as_str());
        let files = if path == notice_file.0 {
            tree(&[(path, src)])
        } else {
            tree(&[(path, src), notice_file])
        };
        judge(&files, &allowed).0
    };
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
    // A nested function is its own scope, by its whole path; leaving it
    // returns to the outer one.
    let nested = verdict(
        &format!("fn open() {{ fn inner() {{ {spawn}; }} {spawn}; }}"),
        lib,
    );
    assert_eq!(nested.len(), 1, "{nested:?}");
    assert!(nested[0].contains("in fn open::inner names"), "{nested:?}");
    // A FUNCTION OF THE SAME NAME ELSEWHERE IN THE FILE NEVER TAKES THE ENTRY
    // OVER (D-5244): in an inline module, an `impl` or a trait it is another
    // path, and the entry it would have excused is stale.
    for (src, path) in [
        (
            format!("fn open() {{}}\nmod sneaky {{ pub fn open() {{ {spawn}; }} }}"),
            "sneaky::open",
        ),
        (
            format!("fn open() {{}}\nimpl S {{ fn open() {{ {spawn}; }} }}"),
            "<S>::open",
        ),
        (
            format!("fn open() {{}}\ntrait T {{ fn open() {{ {spawn}; }} }}"),
            "T::open",
        ),
    ] {
        let spoof = verdict(&src, lib);
        assert_eq!(
            spoof,
            vec![
                format!(
                    "crates/x/src/lib.rs:2 in fn {path} names the process-spawning type: a \
                     process spawn outside test code is not allowed"
                ),
                "the allowlist names fn open in crates/x/src/lib.rs, which starts no program \
                 any more: remove the entry rather than leave it to excuse a later one"
                    .to_owned(),
            ],
            "{src}"
        );
    }
    // An `impl` entry is its header: another type's `notify` is not it, and
    // generics and `where` do not change the header's name.
    let notices = verdict(
        &format!(
            "{notice}\nimpl Notify for RecordOnly {{ fn notify(&self) {{ {spawn}; }} }}\n\
             impl<T: Fn() -> u8> Notify for Later<T> where T: Copy {{ fn notify(&self) {{ {spawn}; }} }}"
        ),
        "crates/x/src/notice.rs",
    );
    assert_eq!(notices.len(), 3, "{notices:?}");
    assert!(notices[0].contains("in fn <Notify for Later<T>>::notify names"));
    assert!(notices[1].contains("in fn <Notify for RecordOnly>::notify names"));
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

/// **THE WALK SKIPS NOTHING UNDER `crates/` AND NAMES EVERY LINK** (D-5244):
/// a directory called `target` is read like any other, a crate's
/// `Cargo.toml` is read, and a link is listed rather than followed or passed
/// over. Built in a fresh temporary directory, never in the tree.
#[test]
fn the_walk_reads_everything_under_crates_and_names_each_link() {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_nanos());
    let root = std::env::temp_dir().join(format!(
        "brutex-no-runtime-spawn-walk-{}-{nanos}",
        std::process::id()
    ));
    let src = root.join("crates/x/src");
    std::fs::create_dir_all(src.join("target")).expect("a scratch tree");
    std::fs::write(
        root.join("crates/x/Cargo.toml"),
        "[package]\nname = \"x\"\n",
    )
    .expect("a manifest");
    std::fs::write(src.join("lib.rs"), "mod target;\n").expect("a crate root");
    std::fs::write(
        src.join("target/mod.rs"),
        format!("pub fn go() {{ std::{MODULE}::{TYPE}::new(\"x\"); }}"),
    )
    .expect("a module named target");
    std::fs::write(src.join("notes.txt"), "not a source").expect("another file");
    #[cfg(unix)]
    std::os::unix::fs::symlink("lib.rs", src.join("linked.rs")).expect("a link");
    // A workspace manifest that cannot be read fails the walk, loudly, rather
    // than being passed over (repair round 2, D-5246).
    let unread = std::panic::catch_unwind(|| sources_under(&root));
    std::fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/x\"]\n",
    )
    .expect("a workspace manifest");
    let (files, links) = sources_under(&root);
    let _ = std::fs::remove_dir_all(&root);
    let why = unread.expect_err("a missing workspace Cargo.toml must fail the walk");
    let said = why
        .downcast_ref::<String>()
        .expect("a panic message")
        .as_str();
    assert!(
        said.ends_with("Cargo.toml cannot be read: No such file or directory (os error 2)"),
        "{said}"
    );

    assert_eq!(
        files.keys().map(String::as_str).collect::<Vec<_>>(),
        [
            "Cargo.toml",
            "crates/x/Cargo.toml",
            "crates/x/src/lib.rs",
            "crates/x/src/notes.txt",
            "crates/x/src/target/mod.rs"
        ]
    );
    // A file of another kind is listed, so a declaration or a target naming
    // it is known to name something, and is not read.
    assert_eq!(files["crates/x/src/notes.txt"], "");
    #[cfg(unix)]
    assert_eq!(links, ["crates/x/src/linked.rs"]);
    assert_eq!(
        judge(&files, &[]).0,
        vec![
            "crates/x/src/target/mod.rs:1 in fn go names the process-spawning type: a process \
             spawn outside test code is not allowed"
                .to_owned()
        ]
    );
}

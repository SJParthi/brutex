//! Gate 31 — the macOS rows run on macOS (GF-1; FM-236, CRIT-044, D-4990).
//! A CI-only executable with no dependencies, built directly by rustc and
//! proved by its own `rustc --test` run before the step trusts it.
//!
//! # The hole this keeps shut
//!
//! Every other CI job runs on `ubuntu-24.04`, and the store's production
//! host is macOS on APFS. A test under `#[cfg(target_os = "macos")]` is not
//! compiled on Linux at all, so a green Linux run says nothing about it: it
//! was never run, and it cannot fail there before its fix. A test that only
//! BRANCHES on the platform inside its body (`cfg!(target_os = "macos")`, a
//! `#[cfg]` statement) runs on both hosts but takes its macOS branch only on
//! macOS. Both kinds are "macOS rows": this gate names every one of them,
//! says on which host each runs and on which it is compiled out, and refuses
//! a workflow with no job that runs them on macOS.
//!
//! # The rule, and what it is not
//!
//! The INVENTORY reads TOKENS of every tracked `.rs` file under `crates/`
//! (comments skipped, strings kept whole, so a brace in a literal is not a
//! brace). A test is a function marked `#[test]` or `#[<path>::test]`. Its
//! platform condition is the conjunction of every `cfg` above it: its own,
//! each enclosing inline module's, `impl`'s or block's, its file's inner
//! `#![cfg]`, and every `cfg` on the `mod` declarations that lead to its
//! file, followed from each crate root through `#[path]`. A `cfg_attr(p,
//! ignore)` ignores it where `p` holds. Each condition is evaluated for the
//! two hosts that matter, `linux` and `macos`: `target_os` and
//! `target_family` (`unix`, `windows`) decide, and any other predicate
//! (`test`, a feature) is taken to hold on both, so it can never make a test
//! look platform-specific. A test is PLATFORM-GATED when it runs on one host
//! and not the other, and PLATFORM-BRANCHING when its body asks the platform.
//!
//! It READS, it does not compile: a test written by a macro is not seen, and
//! a `cfg` it cannot parse is taken to hold. Both are limits, named in
//! `docs/06-limits.md` (L-GF1-31), never a pass on their own: `ran` below
//! checks the macOS run's own log for every macOS row this inventory names.
//!
//! # Usage
//!
//! From the repository root (gate 0's spawn rule, D-2344, lets a `.github`
//! tool start `git` and no other program):
//!
//! * `gates_platform inventory` prints every macOS row with the hosts it runs
//!   on and is compiled out on, then one line per host counting the rows NOT
//!   RUN there. A row compiled out on a host is counted NOT RUN, never green.
//! * `gates_platform verdict <workflow>` refuses a workflow in which no job
//!   on a `macos` runner runs `cargo test --workspace` and this tool's `ran`,
//!   or whose macOS job may fail without failing `ci-ok`.
//! * `gates_platform ran <log>` reads a `cargo test` log written on this host
//!   and refuses unless every row that runs here passed in it, naming the
//!   host each one ran on.
//!
//! Exit 0 is a pass, 1 a refusal, 2 a misuse.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::process::{Command, ExitCode, Stdio};

/// The two hosts that matter: CI's and the store's.
const HOSTS: [&str; 2] = ["linux", "macos"];

// ------------------------------------------------------------- tokens --

/// One token of Rust source, as far as this gate needs to tell them apart.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Tok {
    /// An identifier or a keyword (a raw `r#name` loses its `r#`).
    Ident(String),
    /// A string literal's text between its quotes.
    Str(String),
    /// One punctuation character.
    Punct(char),
    /// A character literal, a lifetime or a number: never a name.
    Other,
}

/// The tokens of `src` with the 1-based line each starts on, comments
/// dropped.
fn lex(src: &str) -> Vec<(Tok, usize)> {
    let chars: Vec<char> = src.chars().collect();
    let at = |i: usize| chars.get(i).copied();
    let mut out = Vec::new();
    let mut i = 0;
    let mut line = 1;
    let mut counted = 0;
    let mut line_at = |upto: usize, line: &mut usize| {
        while counted < upto {
            if chars.get(counted) == Some(&'\n') {
                *line += 1;
            }
            counted += 1;
        }
        *line
    };
    while let Some(c) = at(i) {
        let here = line_at(i, &mut line);
        if c.is_whitespace() {
            i += 1;
        } else if c == '/' && at(i + 1) == Some('/') {
            while at(i).is_some_and(|c| c != '\n') {
                i += 1;
            }
        } else if c == '/' && at(i + 1) == Some('*') {
            i = skip_block_comment(&chars, i);
        } else if let Some((text, next)) = string_at(&chars, i) {
            out.push((Tok::Str(text), here));
            i = next;
        } else if c == '\'' {
            i = skip_quote(&chars, i);
            out.push((Tok::Other, here));
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
            out.push((Tok::Ident(chars[start..end].iter().collect()), here));
            i = end.max(i + 1);
        } else if c.is_ascii_digit() {
            while at(i).is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '.') {
                i += 1;
            }
            out.push((Tok::Other, here));
        } else {
            out.push((Tok::Punct(c), here));
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
    if chars.get(i + 1) == Some(&'\\') {
        let mut j = i + 2;
        while j < chars.len() && chars.get(j) != Some(&'\'') {
            j += 1;
        }
        return j + 1;
    }
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

/// The index after the group opening at `i` (`(`, `[` or `{`), balanced.
fn skip_group(toks: &[(Tok, usize)], i: usize) -> usize {
    let mut depth = 0usize;
    let mut j = i;
    while let Some((tok, _)) = toks.get(j) {
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

// --------------------------------------------------------- predicates --

/// A `cfg` predicate, as far as the platform decides it.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Pred {
    /// `target_os = "<os>"`.
    Os(String),
    /// `unix`, `windows`, `target_family = "<family>"`.
    Family(String),
    Any(Vec<Pred>),
    All(Vec<Pred>),
    Not(Box<Pred>),
    /// Anything the platform does not decide: taken to hold on every host.
    Other,
}

impl Pred {
    fn holds(&self, os: &str) -> bool {
        match self {
            Pred::Os(want) => want == os,
            // Both hosts that matter are Unix.
            Pred::Family(family) => family == "unix",
            Pred::Any(all) => all.iter().any(|p| p.holds(os)),
            Pred::All(all) => all.iter().all(|p| p.holds(os)),
            Pred::Not(p) => !p.holds(os),
            Pred::Other => true,
        }
    }

    /// Whether the two hosts disagree on it.
    fn splits_hosts(&self) -> bool {
        self.holds(HOSTS[0]) != self.holds(HOSTS[1])
    }
}

/// The predicate whose tokens start at `toks[*i]`; `*i` ends after it.
fn parse_pred(toks: &[(Tok, usize)], i: &mut usize) -> Pred {
    let Some((Tok::Ident(name), _)) = toks.get(*i) else {
        // Not a predicate: step over one token or group.
        *i = match toks.get(*i) {
            Some((Tok::Punct('(' | '[' | '{'), _)) => skip_group(toks, *i),
            _ => *i + 1,
        };
        return Pred::Other;
    };
    let name = name.clone();
    *i += 1;
    match toks.get(*i) {
        Some((Tok::Punct('('), _)) if matches!(name.as_str(), "any" | "all" | "not") => {
            *i += 1;
            let mut list = Vec::new();
            while let Some((tok, _)) = toks.get(*i) {
                match tok {
                    Tok::Punct(')') => {
                        *i += 1;
                        break;
                    }
                    Tok::Punct(',') => *i += 1,
                    _ => list.push(parse_pred(toks, i)),
                }
            }
            match name.as_str() {
                "any" => Pred::Any(list),
                "all" => Pred::All(list),
                _ => Pred::Not(Box::new(list.into_iter().next().unwrap_or(Pred::Other))),
            }
        }
        Some((Tok::Punct('='), _)) => {
            *i += 1;
            let value = match toks.get(*i) {
                Some((Tok::Str(s), _)) => s.clone(),
                _ => String::new(),
            };
            *i += 1;
            match name.as_str() {
                "target_os" => Pred::Os(value),
                "target_family" => Pred::Family(value),
                _ => Pred::Other,
            }
        }
        Some((Tok::Punct('('), _)) => {
            *i = skip_group(toks, *i);
            Pred::Other
        }
        _ => match name.as_str() {
            "unix" | "windows" => Pred::Family(name),
            _ => Pred::Other,
        },
    }
}

/// What one attribute's tokens (inside `#[...]`) say to this gate.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Attr {
    Cfg(Pred),
    /// `cfg_attr(p, .., ignore, ..)`: ignored where `p` holds.
    IgnoreWhere(Pred),
    /// `test`, `tokio::test`, `<path>::test(..)`.
    Test,
    /// `path = "<file>"`.
    Path(String),
    None,
}

fn attr(body: &[(Tok, usize)]) -> Attr {
    let ident = |k: usize| match body.get(k) {
        Some((Tok::Ident(s), _)) => Some(s.as_str()),
        _ => None,
    };
    match (ident(0), body.get(1).map(|t| &t.0)) {
        (Some("cfg"), Some(Tok::Punct('('))) => {
            let mut i = 2;
            Attr::Cfg(parse_pred(body, &mut i))
        }
        (Some("cfg_attr"), Some(Tok::Punct('('))) => {
            let mut i = 2;
            let pred = parse_pred(body, &mut i);
            let ignores = body
                .get(i..)
                .unwrap_or(&[])
                .iter()
                .any(|(t, _)| *t == Tok::Ident("ignore".to_owned()));
            if ignores {
                Attr::IgnoreWhere(pred)
            } else {
                Attr::None
            }
        }
        (Some("path"), Some(Tok::Punct('='))) => match body.get(2) {
            Some((Tok::Str(s), _)) => Attr::Path(s.clone()),
            _ => Attr::None,
        },
        _ => {
            // A path `a::b::test`, then the end or an argument list.
            let mut k = 0;
            let mut last = None;
            while let Some(name) = ident(k) {
                last = Some(name);
                if body.get(k + 1).map(|t| &t.0) == Some(&Tok::Punct(':'))
                    && body.get(k + 2).map(|t| &t.0) == Some(&Tok::Punct(':'))
                {
                    k += 3;
                } else {
                    k += 1;
                    break;
                }
            }
            let ends = matches!(body.get(k).map(|t| &t.0), None | Some(Tok::Punct('(')));
            if last == Some("test") && ends {
                Attr::Test
            } else {
                Attr::None
            }
        }
    }
}

// ---------------------------------------------------------- inventory --

/// One test function the gate read.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Test {
    file: String,
    line: usize,
    name: String,
    /// Every `cfg` inside its file above it.
    cfg: Vec<Pred>,
    /// Every `cfg_attr(.., ignore)` on it.
    ignore: Vec<Pred>,
    /// Whether its body asks the platform.
    branches: bool,
}

/// A `mod name;` that names another file.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ModDecl {
    file: String,
    name: String,
    path: Option<String>,
    /// The inline modules it is declared inside, outermost first.
    inline: Vec<String>,
    cfg: Vec<Pred>,
}

/// What one file holds.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct FileFacts {
    inner: Vec<Pred>,
    tests: Vec<Test>,
    mods: Vec<ModDecl>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Item {
    Mod,
    Fn,
    Macro,
    Other,
}

#[derive(Debug, Default)]
struct Pending {
    cfg: Vec<Pred>,
    ignore: Vec<Pred>,
    test: bool,
    path: Option<String>,
}

#[derive(Debug)]
struct Frame {
    cfg: Vec<Pred>,
    /// The index in `FileFacts::tests` of the test this body is.
    test: Option<usize>,
    module: Option<String>,
    macro_body: bool,
}

/// The tests, inner `cfg`s and file modules of one file.
fn facts(file: &str, src: &str) -> FileFacts {
    let toks = lex(src);
    let mut out = FileFacts::default();
    let mut frames: Vec<Frame> = Vec::new();
    let mut pending = Pending::default();
    let mut item: Option<(Item, String, usize)> = None;
    let mut nest = 0usize;
    let mut i = 0;
    let in_test = |frames: &[Frame]| frames.iter().rev().find_map(|f| f.test);
    let in_macro = |frames: &[Frame]| frames.iter().any(|f| f.macro_body);
    while let Some((tok, line)) = toks.get(i) {
        let next = |k: usize| toks.get(i + k).map(|t| &t.0);
        match tok {
            Tok::Punct('#')
                if next(1) == Some(&Tok::Punct('['))
                    || (next(1) == Some(&Tok::Punct('!')) && next(2) == Some(&Tok::Punct('['))) =>
            {
                let inner = next(1) == Some(&Tok::Punct('!'));
                let open = if inner { i + 2 } else { i + 1 };
                let end = skip_group(&toks, open);
                let body = toks.get(open + 1..end.saturating_sub(1)).unwrap_or(&[]);
                let said = attr(body);
                if let (Attr::Cfg(p), Some(t)) = (&said, in_test(&frames))
                    && p.splits_hosts()
                    && let Some(test) = out.tests.get_mut(t)
                {
                    test.branches = true;
                }
                match (said, inner) {
                    (Attr::Cfg(p), true) => match frames.last_mut() {
                        Some(frame) => frame.cfg.push(p),
                        None => out.inner.push(p),
                    },
                    (Attr::Cfg(p), false) => pending.cfg.push(p),
                    (Attr::IgnoreWhere(p), false) => pending.ignore.push(p),
                    (Attr::Test, false) => pending.test = true,
                    (Attr::Path(p), false) => pending.path = Some(p),
                    _ => {}
                }
                i = end;
                continue;
            }
            Tok::Ident(word) if word == "cfg" && next(1) == Some(&Tok::Punct('!')) => {
                let mut k = i + 3;
                let pred = parse_pred(&toks, &mut k);
                if pred.splits_hosts()
                    && let Some(t) = in_test(&frames)
                    && let Some(test) = out.tests.get_mut(t)
                {
                    test.branches = true;
                }
            }
            Tok::Ident(word) if item.is_none() => {
                let kind = match word.as_str() {
                    "mod" => Some(Item::Mod),
                    "fn" => Some(Item::Fn),
                    "macro_rules" => Some(Item::Macro),
                    "impl" | "struct" | "enum" | "trait" | "const" | "static" | "type" | "use"
                    | "let" | "extern" | "union" => Some(Item::Other),
                    _ => None,
                };
                if let Some(kind) = kind {
                    let name = match next(1) {
                        Some(Tok::Ident(n)) => n.clone(),
                        _ => String::new(),
                    };
                    item = Some((kind, name, *line));
                }
            }
            Tok::Punct('(' | '[') => nest += 1,
            Tok::Punct(')' | ']') => nest = nest.saturating_sub(1),
            Tok::Punct('{') => {
                let mut frame = Frame {
                    cfg: std::mem::take(&mut pending.cfg),
                    test: None,
                    module: None,
                    macro_body: false,
                };
                match item.take() {
                    Some((Item::Fn, name, at)) if pending.test && !in_macro(&frames) => {
                        let mut cfg: Vec<Pred> =
                            frames.iter().flat_map(|f| f.cfg.iter().cloned()).collect();
                        cfg.extend(frame.cfg.iter().cloned());
                        out.tests.push(Test {
                            file: file.to_owned(),
                            line: at,
                            name,
                            cfg,
                            ignore: std::mem::take(&mut pending.ignore),
                            branches: false,
                        });
                        frame.test = Some(out.tests.len() - 1);
                    }
                    Some((Item::Mod, name, _)) => frame.module = Some(name),
                    Some((Item::Macro, _, _)) => frame.macro_body = true,
                    _ => {}
                }
                pending = Pending::default();
                frames.push(frame);
            }
            Tok::Punct('}') => {
                frames.pop();
                pending = Pending::default();
                item = None;
            }
            Tok::Punct(';') if nest == 0 => {
                if let Some((Item::Mod, name, _)) = &item
                    && !in_macro(&frames)
                {
                    let mut cfg: Vec<Pred> =
                        frames.iter().flat_map(|f| f.cfg.iter().cloned()).collect();
                    cfg.append(&mut pending.cfg);
                    out.mods.push(ModDecl {
                        file: file.to_owned(),
                        name: name.clone(),
                        path: pending.path.take(),
                        inline: frames.iter().filter_map(|f| f.module.clone()).collect(),
                        cfg,
                    });
                }
                pending = Pending::default();
                item = None;
            }
            Tok::Punct(',') if nest == 0 && item.is_none() => pending = Pending::default(),
            _ => {}
        }
        i += 1;
    }
    out
}

/// `a/b/../c/./d` as `a/c/d`.
fn normal(path: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            _ => parts.push(part),
        }
    }
    parts.join("/")
}

/// Whether `file` is a crate root, whose `mod x;` resolve beside it.
fn is_root(file: &str) -> bool {
    let parts: Vec<&str> = file.split('/').collect();
    matches!(
        parts.as_slice(),
        ["crates", _, "build.rs"]
            | ["crates", _, "src", "lib.rs" | "main.rs"]
            | ["crates", _, "src", "bin", _]
            | ["crates", _, "tests" | "benches" | "examples", _]
    )
}

/// The file a declaration names, among `tracked`.
fn child_of(decl: &ModDecl, tracked: &BTreeSet<String>) -> Option<String> {
    let (dir, stem) = match decl.file.rsplit_once('/') {
        Some((dir, base)) => (dir.to_owned(), base.trim_end_matches(".rs").to_owned()),
        None => (String::new(), decl.file.trim_end_matches(".rs").to_owned()),
    };
    let mod_rs = is_root(&decl.file) || decl.file.ends_with("/mod.rs");
    let mut base = if mod_rs || (decl.path.is_some() && decl.inline.is_empty()) {
        dir
    } else {
        format!("{dir}/{stem}")
    };
    for inline in &decl.inline {
        base = format!("{base}/{inline}");
    }
    let wanted = match &decl.path {
        Some(path) => vec![normal(&format!("{base}/{path}"))],
        None => vec![
            normal(&format!("{base}/{}.rs", decl.name)),
            normal(&format!("{base}/{}/mod.rs", decl.name)),
        ],
    };
    wanted.into_iter().find(|w| tracked.contains(w))
}

/// One test's verdict on each host.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Row {
    test: Test,
    /// For each of [`HOSTS`]: whether it is compiled and run there.
    runs: [bool; 2],
}

impl Row {
    fn gated(&self) -> bool {
        self.runs[0] != self.runs[1]
    }
    fn runs_on(&self, os: &str) -> bool {
        HOSTS
            .iter()
            .position(|h| *h == os)
            .is_some_and(|k| self.runs[k])
    }
}

/// Every macOS row of `files` (path, source): each test that is gated or
/// branches on the platform, with where it runs.
fn inventory(files: &[(String, String)]) -> Vec<Row> {
    let tracked: BTreeSet<String> = files.iter().map(|(p, _)| p.clone()).collect();
    let all: BTreeMap<String, FileFacts> = files
        .iter()
        .filter(|(p, _)| p.ends_with(".rs"))
        .map(|(p, s)| (p.clone(), facts(p, s)))
        .collect();
    // Who declares each file, and under which condition.
    let mut parent: BTreeMap<String, (String, Vec<Pred>)> = BTreeMap::new();
    for facts in all.values() {
        for decl in &facts.mods {
            if let Some(child) = child_of(decl, &tracked) {
                parent
                    .entry(child)
                    .or_insert_with(|| (decl.file.clone(), decl.cfg.clone()));
            }
        }
    }
    let gate_of = |file: &str| -> Vec<Pred> {
        let mut gate = Vec::new();
        let mut at = file.to_owned();
        let mut seen = BTreeSet::new();
        while seen.insert(at.clone()) {
            if let Some(f) = all.get(&at) {
                gate.extend(f.inner.iter().cloned());
            }
            match parent.get(&at) {
                Some((up, cfg)) => {
                    gate.extend(cfg.iter().cloned());
                    at = up.clone();
                }
                None => break,
            }
        }
        gate
    };
    let mut rows = Vec::new();
    for (file, facts) in &all {
        let gate = gate_of(file);
        for test in &facts.tests {
            let runs = HOSTS.map(|os| {
                gate.iter().all(|p| p.holds(os))
                    && test.cfg.iter().all(|p| p.holds(os))
                    && !test.ignore.iter().any(|p| p.holds(os))
            });
            let row = Row {
                test: test.clone(),
                runs,
            };
            if row.gated() || test.branches {
                rows.push(row);
            }
        }
    }
    rows
}

/// The inventory's report: one line per row, then one per host.
fn report(rows: &[Row]) -> Vec<String> {
    let mut out = Vec::new();
    for row in rows {
        let hosts: Vec<String> = HOSTS
            .iter()
            .zip(row.runs)
            .map(|(h, r)| format!("{h}={}", if r { "runs" } else { "NOT RUN" }))
            .collect();
        let kind = if row.gated() { "gated" } else { "branches" };
        out.push(format!(
            "{}:{} {} [{kind}] {}",
            row.test.file,
            row.test.line,
            row.test.name,
            hosts.join(" ")
        ));
    }
    for (k, host) in HOSTS.iter().enumerate() {
        let not_run = rows.iter().filter(|r| !r.runs[k]).count();
        out.push(format!(
            "host {host}: {} macOS rows, {} run there, {not_run} compiled out or ignored there: \
             counted NOT RUN, never green",
            rows.len(),
            rows.len() - not_run
        ));
    }
    out
}

// ----------------------------------------------------------- workflow --

/// The jobs of a workflow: name, then the lines of its body.
fn jobs(workflow: &str) -> Vec<(String, Vec<String>)> {
    let mut out: Vec<(String, Vec<String>)> = Vec::new();
    let mut inside = false;
    for line in workflow.split('\n') {
        if !line.starts_with(' ') && !line.trim().is_empty() && !line.starts_with('#') {
            inside = line.trim_end() == "jobs:";
            continue;
        }
        if !inside {
            continue;
        }
        let key = line.strip_prefix("  ").filter(|rest| {
            !rest.starts_with(' ') && !rest.starts_with('#') && rest.trim_end().ends_with(':')
        });
        match key {
            Some(name) => out.push((name.trim_end().trim_end_matches(':').to_owned(), Vec::new())),
            None => {
                if let Some((_, body)) = out.last_mut() {
                    body.push(line.to_owned());
                }
            }
        }
    }
    out
}

/// The value of the job-level key `key` (`    key: value`).
fn job_key<'a>(body: &'a [String], key: &str) -> Option<&'a str> {
    body.iter().find_map(|line| {
        line.strip_prefix("    ")
            .filter(|rest| !rest.starts_with(' '))
            .and_then(|rest| rest.strip_prefix(key))
            .and_then(|rest| rest.strip_prefix(':'))
            .map(str::trim)
    })
}

/// The lines of a job that are commands rather than comments.
fn commands(body: &[String]) -> impl Iterator<Item = &str> {
    body.iter()
        .map(|l| l.trim())
        .filter(|l| !l.starts_with('#'))
}

/// Why `workflow` does not run the macOS rows on macOS, empty when it does.
fn verdict(workflow: &str, rows: &[Row]) -> Vec<String> {
    let jobs = jobs(workflow);
    let mut why = Vec::new();
    let needs: BTreeSet<String> = jobs
        .iter()
        .find(|(name, _)| name == "ci-ok")
        .and_then(|(_, body)| job_key(body, "needs"))
        .map(|list| {
            list.trim_matches(|c| c == '[' || c == ']')
                .split(',')
                .map(|s| s.trim().to_owned())
                .collect()
        })
        .unwrap_or_default();
    let tests_workspace = |body: &[String]| {
        commands(body).any(|c| c.contains("cargo test") && c.contains("--workspace"))
    };
    let on = |prefix: &str| -> Vec<&(String, Vec<String>)> {
        jobs.iter()
            .filter(|(_, body)| job_key(body, "runs-on").is_some_and(|r| r.starts_with(prefix)))
            .collect()
    };
    let mac = on("macos");
    if mac.is_empty() {
        why.push("no job runs on a macos runner: the macOS rows are never run".to_owned());
    }
    for (name, body) in &mac {
        if !tests_workspace(body) {
            why.push(format!(
                "job {name} runs on macOS but not `cargo test --workspace`"
            ));
        }
        if !commands(body).any(|c| c.contains("gates_platform") || c.contains("gates-platform"))
            || !commands(body).any(|c| c.contains(" ran "))
        {
            why.push(format!(
                "job {name} does not check its own log with `gates_platform ran`"
            ));
        }
        if job_key(body, "continue-on-error").is_some_and(|v| v == "true") {
            why.push(format!("job {name} may fail without failing the run"));
        }
        if job_key(body, "if").is_some_and(|v| v.contains("false")) {
            why.push(format!("job {name} is switched off"));
        }
        if !needs.contains(name.as_str()) {
            why.push(format!("job {name} is not among ci-ok's needs"));
        }
    }
    let linux_only = rows.iter().any(|r| r.runs[0] && !r.runs[1]);
    if linux_only && !on("ubuntu").iter().any(|(_, body)| tests_workspace(body)) {
        why.push(
            "rows that run only on Linux, and no ubuntu job runs `cargo test --workspace`"
                .to_owned(),
        );
    }
    why
}

/// Whether `log` (a `cargo test` log) reports the test `name` passing.
fn passed(log: &str, name: &str) -> bool {
    let suffix = format!("::{name} ... ok");
    let bare = format!("test {name} ... ok");
    log.split('\n').any(|line| {
        let line = line.trim_end();
        line.starts_with("test ") && (line.ends_with(&suffix) || line == bare)
    })
}

/// Why the run on `os` is not proved by `log`, empty when it is, and the
/// rows it proved.
fn ran(log: &str, os: &str, rows: &[Row]) -> (Vec<String>, Vec<String>) {
    let mut why = Vec::new();
    let mut proved = Vec::new();
    for row in rows.iter().filter(|r| r.runs_on(os)) {
        let t = &row.test;
        if passed(log, &t.name) {
            proved.push(format!("{}:{} {} ran on {os}: ok", t.file, t.line, t.name));
        } else {
            why.push(format!(
                "{}:{} {} runs on {os} and did not pass in the log",
                t.file, t.line, t.name
            ));
        }
    }
    (why, proved)
}

// --------------------------------------------------------------- main --

fn tracked() -> Result<Vec<(String, String)>, String> {
    let out = Command::new("git")
        .args(["ls-files", "-z", "--", "crates"])
        .stderr(Stdio::inherit())
        .output()
        .map_err(|e| format!("git ls-files: {e}"))?;
    if !out.status.success() {
        return Err(format!("git ls-files failed: {}", out.status));
    }
    let mut files = Vec::new();
    for path in out.stdout.split(|b| *b == 0).filter(|p| !p.is_empty()) {
        let path = String::from_utf8_lossy(path).into_owned();
        if !path.ends_with(".rs") {
            continue;
        }
        let src = std::fs::read(&path).map_err(|e| format!("{path}: {e}"))?;
        files.push((path, String::from_utf8_lossy(&src).into_owned()));
    }
    if files.is_empty() {
        return Err("git ls-files listed no .rs file under crates/".to_owned());
    }
    Ok(files)
}

fn run(args: &[String]) -> Result<bool, String> {
    let rows = inventory(&tracked()?);
    let read = |p: &str| {
        std::fs::read(p)
            .map(|b| String::from_utf8_lossy(&b).into_owned())
            .map_err(|e| format!("{p}: {e}"))
    };
    let (why, said) = match args {
        [cmd] if cmd == "inventory" => (Vec::new(), report(&rows)),
        [cmd, workflow] if cmd == "verdict" => (verdict(&read(workflow)?, &rows), report(&rows)),
        [cmd, log] if cmd == "ran" => {
            let log = read(log)?;
            if log.trim().is_empty() {
                return Err("the test log is empty".to_owned());
            }
            ran(&log, std::env::consts::OS, &rows)
        }
        _ => {
            return Err(
                "usage: gates_platform inventory | verdict <workflow> | ran <log>".to_owned(),
            );
        }
    };
    let mut stdout = std::io::stdout().lock();
    for line in &said {
        use std::io::Write as _;
        writeln!(stdout, "{line}").map_err(|e| e.to_string())?;
    }
    for line in &why {
        eprintln!("REFUSED: {line}");
    }
    Ok(why.is_empty())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(1),
        Err(why) => {
            eprintln!("gates_platform: {why}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one(path: &str, src: &str) -> Vec<(String, String)> {
        vec![(path.to_owned(), src.to_owned())]
    }

    fn names(rows: &[Row]) -> Vec<(&str, [bool; 2])> {
        rows.iter()
            .map(|r| (r.test.name.as_str(), r.runs))
            .collect()
    }

    const GOOD: &str = "on: push\njobs:\n  build:\n    runs-on: ubuntu-24.04\n    steps:\n      \
                        - run: cargo test --workspace --locked\n  macos:\n    runs-on: macos-15\n    \
                        steps:\n      - run: |\n          \"$tool\" verdict ci.yml\n          \
                        cargo test --workspace --locked 2>&1 | tee log\n          \
                        rustc .github/gates_platform.rs -o t; t ran log\n  ci-ok:\n    \
                        runs-on: ubuntu-24.04\n    needs: [build, macos]\n";

    #[test]
    fn ci_runs_macos_rows_on_macos_and_counts_cfg_skipped_tests() {
        let src = "#[cfg(target_os = \"macos\")]\n#[test]\nfn only_mac() {}\n\
                   #[cfg(target_os = \"linux\")]\n#[test]\nfn only_linux() {}\n\
                   #[test]\nfn both() {}\n\
                   #[test]\nfn asks() { if cfg!(target_os = \"macos\") { } }\n";
        let rows = inventory(&one("crates/a/src/lib.rs", src));
        assert_eq!(
            names(&rows),
            vec![
                ("only_mac", [false, true]),
                ("only_linux", [true, false]),
                ("asks", [true, true]),
            ],
            "`both` asks nothing of the platform and is not a row"
        );
        let said = report(&rows);
        assert!(
            said[0].ends_with("only_mac [gated] linux=NOT RUN macos=runs"),
            "{said:?}"
        );
        assert!(said[2].contains("[branches]"), "{said:?}");
        assert_eq!(
            said[3],
            "host linux: 3 macOS rows, 2 run there, 1 compiled out or ignored there: counted \
             NOT RUN, never green"
        );
        assert!(said[4].starts_with("host macos: 3 macOS rows, 2 run there, 1 compiled out"));

        // The workflow: the good one passes, and each way of not running
        // the rows on macOS is refused by name.
        assert_eq!(verdict(GOOD, &rows), Vec::<String>::new());
        let cases = [
            (
                "runs-on: macos-15",
                "runs-on: ubuntu-24.04",
                "no job runs on a macos runner",
            ),
            (
                "cargo test --workspace --locked 2>&1",
                "cargo test -p a 2>&1",
                "not `cargo test --workspace`",
            ),
            ("t ran log", "t log", "does not check its own log"),
            (
                "needs: [build, macos]",
                "needs: [build]",
                "not among ci-ok's needs",
            ),
            (
                "    runs-on: macos-15\n",
                "    runs-on: macos-15\n    continue-on-error: true\n",
                "may fail without failing",
            ),
            (
                "    runs-on: macos-15\n",
                "    runs-on: macos-15\n    if: false\n",
                "switched off",
            ),
            (
                "      - run: cargo test --workspace --locked\n  macos",
                "      - run: cargo build\n  macos",
                "only on Linux",
            ),
        ];
        for (from, to, says) in cases {
            assert!(GOOD.contains(from), "{from}");
            let bad = GOOD.replacen(from, to, 1);
            let why = verdict(&bad, &rows);
            assert!(why.iter().any(|w| w.contains(says)), "{says}: {why:?}");
        }
        // A commented command does not count as running one.
        let commented = GOOD.replace(
            "cargo test --workspace --locked 2>&1",
            "# cargo test --workspace",
        );
        assert!(!verdict(&commented, &rows).is_empty());

        // The run's own log: every row that runs on the host must pass in it.
        let log = "test a::only_mac ... ok\ntest a::asks ... ok\n";
        let (why, proved) = ran(log, "macos", &rows);
        assert!(why.is_empty(), "{why:?}");
        assert_eq!(proved.len(), 2);
        assert!(
            proved[0].ends_with("only_mac ran on macos: ok"),
            "{proved:?}"
        );
        let (why, _) = ran(
            "test a::only_mac ... FAILED\ntest a::asks ... ok\n",
            "macos",
            &rows,
        );
        assert_eq!(why.len(), 1, "a failed row is not proved");
        let (why, _) = ran(log, "linux", &rows);
        assert_eq!(why.len(), 1, "only_linux is not in this log: {why:?}");
    }

    #[test]
    fn the_workflow_in_this_repository_runs_the_macos_rows_on_macos() {
        // Run from the repository root, as every CI step runs it.
        let workflow = std::fs::read_to_string(".github/workflows/ci.yml")
            .expect("run from the repository root");
        let rows = inventory(&one(
            "crates/a/src/lib.rs",
            "#[cfg(target_os = \"macos\")] #[test] fn m() {}\n\
             #[cfg(target_os = \"linux\")] #[test] fn l() {}\n",
        ));
        assert_eq!(verdict(&workflow, &rows), Vec::<String>::new());
    }

    #[test]
    fn a_gate_reaches_a_test_through_modules_files_and_paths() {
        let files = vec![
            (
                "crates/a/src/lib.rs".to_owned(),
                "#[cfg(target_os = \"macos\")]\nmod mac;\n#[cfg(not(target_os = \"macos\"))]\n\
                 #[path = \"../tests/support/mod.rs\"]\nmod support;\nmod plain;\n\
                 #[cfg(unix)] mod unixy;\n"
                    .to_owned(),
            ),
            (
                "crates/a/src/mac.rs".to_owned(),
                "mod deeper;\n#[test] fn in_mac() {}\n".to_owned(),
            ),
            (
                "crates/a/src/mac/deeper.rs".to_owned(),
                "#[test] fn two_down() {}\n".to_owned(),
            ),
            (
                "crates/a/tests/support/mod.rs".to_owned(),
                "#[test] fn not_on_mac() {}\n".to_owned(),
            ),
            (
                "crates/a/src/plain/mod.rs".to_owned(),
                "#![cfg(target_os = \"linux\")]\n#[test] fn inner_gate() {}\n".to_owned(),
            ),
            (
                "crates/a/src/unixy.rs".to_owned(),
                "#[test] fn on_both() {}\n".to_owned(),
            ),
        ];
        let rows = inventory(&files);
        let mut got = names(&rows);
        got.sort();
        assert_eq!(
            got,
            vec![
                ("in_mac", [false, true]),
                ("inner_gate", [true, false]),
                ("not_on_mac", [true, false]),
                ("two_down", [false, true]),
            ],
            "`unix` holds on both hosts, so `on_both` is not a row"
        );
    }

    #[test]
    fn inline_modules_impls_blocks_and_ignores_carry_the_condition() {
        let src = "#[cfg(test)]\nmod tests {\n    #[cfg(target_os = \"macos\")]\n    mod mac {\n        \
                   #[tokio::test]\n        async fn awaited() {}\n        \
                   #[test]\n        fn plain() { let s = \"} { }\"; let c = '{'; }\n    }\n    \
                   #[test]\n    #[cfg_attr(target_os = \"linux\", ignore = \"needs APFS\")]\n    \
                   fn ignored_on_linux() {}\n    \
                   #[test]\n    #[ignore]\n    fn ignored_everywhere() {}\n    \
                   #[test]\n    fn after() {\n        #[cfg(target_os = \"linux\")]\n        let x = 1;\n    }\n\
                   }\n\
                   #[cfg(any(target_os = \"linux\", target_os = \"android\"))]\nimpl S {\n    \
                   #[test]\n    fn in_impl() {}\n}\n\
                   macro_rules! made { () => { #[test] fn by_macro() {} }; }\n\
                   /* #[cfg(target_os = \"macos\")] #[test] fn commented() {} */\n";
        let rows = inventory(&one("crates/a/src/x.rs", src));
        assert_eq!(
            names(&rows),
            vec![
                ("awaited", [false, true]),
                ("plain", [false, true]),
                ("ignored_on_linux", [false, true]),
                ("after", [true, true]),
                ("in_impl", [true, false]),
            ],
            "a brace in a string or a char is not a brace; a macro's and a comment's tests \
             are not read; `#[ignore]` everywhere is not a platform split"
        );
        assert!(rows[3].test.branches);
        assert_eq!(rows[0].test.line, 6, "the line of the `fn`");
    }

    #[test]
    fn predicates_are_decided_by_the_platform_alone() {
        let parse = |s: &str| {
            let toks = lex(s);
            let mut i = 0;
            parse_pred(&toks, &mut i)
        };
        let cases = [
            ("target_os = \"macos\"", [false, true]),
            ("not(target_os = \"macos\")", [true, false]),
            ("unix", [true, true]),
            ("windows", [false, false]),
            ("target_family = \"unix\"", [true, true]),
            ("all(test, target_os = \"linux\")", [true, false]),
            ("any(feature = \"x\", target_os = \"macos\")", [true, true]),
            (
                "all(not(windows), any(target_os = \"macos\", target_os = \"ios\"))",
                [false, true],
            ),
            ("debug_assertions", [true, true]),
            ("target_pointer_width(\"64\")", [true, true]),
        ];
        for (src, want) in cases {
            let p = parse(src);
            assert_eq!(HOSTS.map(|h| p.holds(h)), want, "{src}: {p:?}");
            assert_eq!(p.splits_hosts(), want[0] != want[1], "{src}");
        }
    }

    #[test]
    fn a_log_line_names_the_test_it_passed() {
        assert!(passed("test a::b::name ... ok\n", "name"));
        assert!(passed("test name ... ok\r\n", "name"));
        assert!(
            !passed("test a::b::other_name ... ok\n", "name"),
            "a suffix of a longer name"
        );
        assert!(!passed("test a::b::name ... ignored\n", "name"));
        assert!(!passed("test a::b::name ... FAILED\n", "name"));
        assert!(!passed(
            "    test a::b::name ... ok in a doc line\n",
            "name"
        ));
    }

    #[test]
    fn jobs_and_their_keys_are_read_from_the_jobs_block_only() {
        let wf = "env:\n  macos:\n    runs-on: macos-15\njobs:\n  # a comment: x\n  a:\n    \
                  runs-on: macos-14\n    needs: [b]\n    steps:\n      - name: x\n        \
                  runs-on: not-a-key\n";
        let got = jobs(wf);
        assert_eq!(got.len(), 1, "{got:?}");
        assert_eq!(got[0].0, "a");
        assert_eq!(job_key(&got[0].1, "runs-on"), Some("macos-14"));
        assert_eq!(job_key(&got[0].1, "needs"), Some("[b]"));
        assert_eq!(
            normal("crates/a/src/../tests/./s/mod.rs"),
            "crates/a/tests/s/mod.rs"
        );
        assert!(is_root("crates/a/tests/x.rs") && !is_root("crates/a/tests/s/mod.rs"));
    }
}

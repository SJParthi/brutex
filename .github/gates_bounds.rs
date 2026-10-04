//! Gates 12 and 14 of the language-purity job (D-2314): every cost claim under
//! `crates/` names the test that proves it, and every crate that claims a bound
//! names a bench that re-measures it. This CI-only executable has no
//! dependencies, is built directly by rustc, and proves itself with
//! `rustc --test` before either gate trusts an answer from it.
//!
//! Both gates were inline text programs in `.github/workflows/ci.yml`. They are
//! ported here rule for rule; the rationale for each rule stays beside the step
//! in the workflow, and this file documents the mechanics. Every pattern below
//! names the regular expression it replaces and is matched with the same
//! leftmost-longest reading those programs had, byte by byte, with the word
//! characters `[A-Za-z0-9_]` (the C-locale reading the local harness measured).
//!
//! ONE CLAIM READER (D-1116). Gates 12 and 14 used to carry two copies of the
//! reader in the workflow, and gate 14 refused unless the two copies were the
//! same bytes. Both gates now call the one `claim_blocks` / `is_claim` pair
//! below, so the two copies cannot diverge: there is only one.
//!
//! WHAT IT MAY NOT DO ITSELF. Gate 0's `spawns` check admits `git` and a short
//! list of named programs, so this tool runs `git ls-files` and nothing else.
//! The source scanner's lexed view of each bench and its `step-runs` verdicts
//! are produced by the step and handed over as files.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::process::{Command, ExitCode, Stdio};

const INVARIANTS: &str = "docs/04-invariants.md";
const WORKFLOW: &str = ".github/workflows/ci.yml";

// ------------------------------------------------------------ repository --

/// What the gates read: tracked paths, a pathspec probe and file contents.
trait Repo {
    /// `git ls-files PATHSPEC`, in index order.
    fn ls(&self, pathspec: &str) -> Result<Vec<String>, String>;
    /// `git ls-files --error-unmatch PATHSPEC` succeeded.
    fn tracked(&self, pathspec: &str) -> Result<bool, String>;
    /// The file's text. Not UTF-8, or holding a NUL, is an error and never an
    /// empty file: rustc refuses such a source, and the old line tools read
    /// one as a binary file and silently dropped its lines.
    fn read(&self, path: &str) -> Result<String, String>;
}

struct Git;

impl Repo for Git {
    fn ls(&self, pathspec: &str) -> Result<Vec<String>, String> {
        let out = Command::new("git")
            .args(["ls-files", "-z", "--", pathspec])
            .stderr(Stdio::inherit())
            .output()
            .map_err(|e| format!("git ls-files {pathspec}: {e}"))?;
        if !out.status.success() {
            return Err(format!("git ls-files {pathspec} exited {}", out.status));
        }
        let text = String::from_utf8(out.stdout)
            .map_err(|e| format!("git ls-files {pathspec}: not UTF-8: {e}"))?;
        Ok(text
            .split('\0')
            .filter(|p| !p.is_empty())
            .map(str::to_owned)
            .collect())
    }

    fn tracked(&self, pathspec: &str) -> Result<bool, String> {
        let status = Command::new("git")
            .args(["ls-files", "--error-unmatch", "--", pathspec])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map_err(|e| format!("git ls-files --error-unmatch {pathspec}: {e}"))?;
        Ok(status.success())
    }

    fn read(&self, path: &str) -> Result<String, String> {
        let bytes = std::fs::read(path).map_err(|e| format!("{path}: {e}"))?;
        text_of(path, bytes)
    }
}

fn text_of(path: &str, bytes: Vec<u8>) -> Result<String, String> {
    if bytes.contains(&0) {
        return Err(format!(
            "{path}: holds a NUL byte, so it is not source text"
        ));
    }
    String::from_utf8(bytes).map_err(|e| format!("{path}: not UTF-8: {e}"))
}

// ------------------------------------------------------- text primitives --

fn word(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_'
}

fn lower_(c: u8) -> bool {
    c.is_ascii_lowercase() || c == b'_'
}

fn ident_start(c: u8) -> bool {
    c.is_ascii_alphabetic() || c == b'_'
}

/// POSIX `[[:space:]]` in the C locale.
fn space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// The end of the run of bytes from `i` that satisfy `p`.
fn run(b: &[u8], i: usize, p: impl Fn(u8) -> bool) -> usize {
    let mut e = i;
    while e < b.len() && p(b[e]) {
        e += 1;
    }
    e
}

/// `\b` before position `i`, given that `b[i]` is a word character.
fn bound_before(b: &[u8], i: usize) -> bool {
    i == 0 || !word(b[i - 1])
}

/// `\b` at position `e`, given that `b[e - 1]` is a word character.
fn bound_after(b: &[u8], e: usize) -> bool {
    b.get(e).is_none_or(|&c| !word(c))
}

/// Records as a line-oriented reader sees them: split on `\n`, with no empty
/// record after a final newline.
fn lines(src: &str) -> Vec<&str> {
    let mut v: Vec<&str> = src.split('\n').collect();
    if v.last() == Some(&"") {
        v.pop();
    }
    v
}

/// Every non-overlapping leftmost match of `at`, which returns the match's
/// end, scanning left to right and resuming at each match's end.
fn find_all(b: &[u8], at: impl Fn(&[u8], usize) -> Option<usize>) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        match at(b, i) {
            Some(e) if e > i => {
                out.push((i, e));
                i = e;
            }
            _ => i += 1,
        }
    }
    out
}

/// One global substitution: every match of `at` becomes `rep`. The replaced
/// output is never searched again, as a global substitution behaves.
fn substitute(b: &[u8], rep: &[u8], at: impl Fn(&[u8], usize) -> Option<usize>) -> Vec<u8> {
    let mut out = Vec::with_capacity(b.len());
    let mut last = 0;
    for (s, e) in find_all(b, at) {
        out.extend_from_slice(&b[last..s]);
        out.extend_from_slice(rep);
        last = e;
    }
    out.extend_from_slice(&b[last..]);
    out
}

fn has_at(b: &[u8], i: usize, lit: &[u8]) -> bool {
    b.get(i..).is_some_and(|r| r.starts_with(lit))
}

// ------------------------------------------------------------ the reader --

/// One maximal run of `///` or `//!` lines.
#[derive(Debug, Clone, PartialEq)]
struct Block {
    file: String,
    line: usize,
    item: String,
    text: String,
}

fn brackets(l: &str) -> i64 {
    let open = l.bytes().filter(|&c| c == b'[').count();
    let close = l.bytes().filter(|&c| c == b']').count();
    i64::try_from(open).unwrap_or(i64::MAX) - i64::try_from(close).unwrap_or(i64::MAX)
}

/// The name the first code line below a block declares: after the first
/// `(^|[^A-Za-z0-9_])fn[ \t]+[A-Za-z_][A-Za-z0-9_]*`, else after the first
/// `struct`/`enum`/`trait`/`const`/`static`/`type`/`mod`/`union`/
/// `macro_rules!` in the same shape; `impl` for an impl; `-` otherwise.
fn item(t: &str) -> String {
    let b = t.as_bytes();
    let named = |p: usize, kw: &[u8]| -> Option<String> {
        if !bound_before(b, p) || !has_at(b, p, kw) {
            return None;
        }
        let s = p + kw.len();
        let e = run(b, s, |c| c == b' ' || c == b'\t');
        if e > s && b.get(e).is_some_and(|&c| ident_start(c)) {
            return Some(t[e..run(b, e, word)].to_owned());
        }
        None
    };
    if let Some(name) = (0..b.len()).find_map(|p| named(p, b"fn")) {
        return name;
    }
    const KEYWORDS: [&[u8]; 9] = [
        b"struct",
        b"enum",
        b"trait",
        b"const",
        b"static",
        b"type",
        b"mod",
        b"union",
        b"macro_rules!",
    ];
    if let Some(name) = (0..b.len()).find_map(|p| KEYWORDS.iter().find_map(|kw| named(p, kw))) {
        return name;
    }
    let is_impl = |p: usize| bound_before(b, p) && has_at(b, p, b"impl") && bound_after(b, p + 4);
    if (0..b.len()).any(is_impl) {
        return "impl".to_owned();
    }
    "-".to_owned()
}

/// Every doc block of one file: its first line, the item it documents and its
/// joined text. Leading blanks and tabs are stripped from each line and tabs
/// become spaces. An attribute below a block is skipped whole, its brackets
/// counted across lines; blank lines and `//` lines are passed over; an inner
/// (`//!`) block ends at the first line that is not a doc line.
fn claim_blocks(file: &str, src: &str) -> Vec<Block> {
    let mut out = Vec::new();
    let mut n = 0usize;
    let mut start = 0usize;
    let mut inner = false;
    let mut pending = false;
    let mut depth = 0i64;
    let mut text = String::new();
    let emit = |name: &str, start: usize, text: &mut String, out: &mut Vec<Block>| {
        out.push(Block {
            file: file.to_owned(),
            line: start,
            item: name.to_owned(),
            text: std::mem::take(text),
        });
    };
    for (k, raw) in lines(src).into_iter().enumerate() {
        let l = raw.trim_start_matches([' ', '\t']).replace('\t', " ");
        if (l.starts_with("///") || l.starts_with("//!")) && depth == 0 {
            if pending {
                emit("-", start, &mut text, &mut out);
                (n, pending, depth) = (0, false, 0);
            }
            if n == 0 {
                start = k + 1;
                inner = l.starts_with("//!");
            }
            text.push(' ');
            text.push_str(&l[3..]);
            n += 1;
            continue;
        }
        if n == 0 {
            continue;
        }
        if inner {
            emit("//!", start, &mut text, &mut out);
            (n, pending, depth) = (0, false, 0);
            continue;
        }
        pending = true;
        if depth > 0 {
            depth += brackets(&l);
            continue;
        }
        if l.starts_with("#[") || l.starts_with("#![") {
            depth = brackets(&l);
            continue;
        }
        if l.is_empty() || l.starts_with("//") {
            continue;
        }
        emit(&item(&l), start, &mut text, &mut out);
        (n, pending, depth) = (0, false, 0);
    }
    if n > 0 {
        emit(if inner { "//!" } else { "-" }, start, &mut text, &mut out);
    }
    out
}

const NOTCLAIM: [&[u8]; 17] = [
    b"struct",
    b"table",
    b"bar",
    b"file",
    b"field",
    b"list",
    b"array",
    b"layout",
    b"namespace",
    b"case",
    b"month",
    b"day",
    b"session",
    b"price",
    b"close",
    b"directory",
    b"directories",
];

/// `[Ff]lat` at `i`, then the end of `' '+` after it (`None` without one).
fn flat_spaces(b: &[u8], i: usize) -> Option<usize> {
    if !matches!(b.get(i), Some(b'F' | b'f')) || !has_at(b, i + 1, b"lat") {
        return None;
    }
    let e = run(b, i + 4, |c| c == b' ');
    (e > i + 4).then_some(e)
}

/// The non-cost senses of `flat` and `worst case`, removed before the trigger
/// runs, as six global substitutions in this order:
///   `[A-Za-z0-9_]::flat` -> ` `, `[Ff]lat[ ]*\(` -> ` (`,
///   `[Ww]orst[ -][Cc]ase`, `[Ff]lat +NOUN s?` (the `notclaim` nouns),
///   `[Ff]lat +(JSON|json) object`, and
///   `(exactly|books the trade|came out|row-major and|is charged|A) [Ff]lat`.
fn claim_scrub(text: &str) -> Vec<u8> {
    let b = text.as_bytes();
    let b = substitute(b, b" ", |b, i| {
        (word(b[i]) && has_at(b, i + 1, b"::flat")).then_some(i + 7)
    });
    let b = substitute(&b, b" (", |b, i| {
        if !matches!(b[i], b'F' | b'f') || !has_at(b, i + 1, b"lat") {
            return None;
        }
        let e = run(b, i + 4, |c| c == b' ');
        (b.get(e) == Some(&b'(')).then_some(e + 1)
    });
    let b = substitute(&b, b"", |b, i| {
        (matches!(b[i], b'W' | b'w')
            && has_at(b, i + 1, b"orst")
            && matches!(b.get(i + 5), Some(b' ' | b'-'))
            && matches!(b.get(i + 6), Some(b'C' | b'c'))
            && has_at(b, i + 7, b"ase"))
        .then_some(i + 10)
    });
    let b = substitute(&b, b"", |b, i| {
        let s = flat_spaces(b, i)?;
        let e = NOTCLAIM
            .iter()
            .filter(|noun| has_at(b, s, noun))
            .map(|noun| s + noun.len())
            .max()?;
        Some(if b.get(e) == Some(&b's') { e + 1 } else { e })
    });
    let b = substitute(&b, b"", |b, i| {
        let s = flat_spaces(b, i)?;
        ((has_at(b, s, b"JSON") || has_at(b, s, b"json")) && has_at(b, s + 4, b" object"))
            .then_some(s + 11)
    });
    substitute(&b, b"", |b, i| {
        [
            &b"exactly"[..],
            b"books the trade",
            b"came out",
            b"row-major and",
            b"is charged",
            b"A",
        ]
        .iter()
        .filter(|lead| {
            has_at(b, i, lead)
                && b.get(i + lead.len()) == Some(&b' ')
                && matches!(b.get(i + lead.len() + 1), Some(b'F' | b'f'))
                && has_at(b, i + lead.len() + 2, b"lat")
        })
        .map(|lead| i + lead.len() + 5)
        .max()
    })
}

/// `[Oo]\(1\)|constant[ -]time|never scans?|worst[ -]case|\bflat\b`.
fn triggers(b: &[u8]) -> bool {
    (0..b.len()).any(|i| {
        has_at(b, i, b"O(1)")
            || has_at(b, i, b"o(1)")
            || has_at(b, i, b"constant time")
            || has_at(b, i, b"constant-time")
            || has_at(b, i, b"never scan")
            || has_at(b, i, b"worst case")
            || has_at(b, i, b"worst-case")
            || (has_at(b, i, b"flat") && bound_before(b, i) && bound_after(b, i + 4))
    })
}

/// Does this block's text make a cost claim? The one definition both gates use.
fn is_claim(text: &str) -> bool {
    triggers(&claim_scrub(text))
}

// ---------------------------------------------------------------- gate 12 --

/// `\bfn [a-z_][a-z_0-9]*`: every match's name.
fn fn_names(line: &str) -> Vec<&str> {
    let b = line.as_bytes();
    find_all(b, |b, i| {
        (bound_before(b, i) && has_at(b, i, b"fn ") && b.get(i + 3).is_some_and(|&c| lower_(c)))
            .then(|| run(b, i + 3, |c| lower_(c) || c.is_ascii_digit()))
    })
    .into_iter()
    .map(|(s, e)| &line[s + 3..e])
    .collect()
}

/// `(^|[^A-Za-z0-9_])fn [a-z_]` anywhere in the line.
fn declares_fn(t: &str) -> bool {
    let b = t.as_bytes();
    (0..b.len()).any(|p| {
        bound_before(b, p) && has_at(b, p, b"fn ") && b.get(p + 3).is_some_and(|&c| lower_(c))
    })
}

/// A comment line, `^[[:space:]]*(//|\*)`, dropped before either table reads.
fn comment_line(l: &str) -> bool {
    let b = l.as_bytes();
    let s = run(b, 0, space);
    has_at(b, s, b"//") || has_at(b, s, b"*")
}

/// The `(crate, fn)` pairs of one file, and the subset that can prove: an `fn`
/// whose attribute run (tracked forward: an attribute mentioning `test` or
/// `bench` sets the flag, a `fn` line consumes it, any other non-blank line
/// clears it) marks it, or any `fn` in a `tests/` or `benches/` file.
fn fn_tables(
    krate: &str,
    path: &str,
    src: &str,
    fns: &mut BTreeSet<(String, String)>,
    proving: &mut BTreeSet<(String, String)>,
) {
    let harness = path.contains("/tests/") || path.contains("/benches/");
    let mut flag = false;
    for line in lines(src).into_iter().filter(|l| !comment_line(l)) {
        for name in fn_names(line) {
            fns.insert((krate.to_owned(), name.to_owned()));
        }
        let t = line.trim_start_matches([' ', '\t']);
        if t.starts_with("#[") {
            if t.contains("test") || t.contains("bench") {
                flag = true;
            }
            continue;
        }
        if declares_fn(t) {
            if flag || harness {
                for name in fn_names(line) {
                    proving.insert((krate.to_owned(), name.to_owned()));
                }
            }
            flag = false;
            continue;
        }
        if !t.is_empty() {
            flag = false;
        }
    }
}

/// `\b[a-z_]+::[a-z_]+::[a-z_0-9]+\b`.
fn qualified(text: &str) -> Vec<&str> {
    let b = text.as_bytes();
    find_all(b, |b, i| {
        if !lower_(b[i]) || !bound_before(b, i) {
            return None;
        }
        let a = run(b, i, lower_);
        if !has_at(b, a, b"::") || !b.get(a + 2).is_some_and(|&c| lower_(c)) {
            return None;
        }
        let m = run(b, a + 2, lower_);
        if !has_at(b, m, b"::") {
            return None;
        }
        let e = run(b, m + 2, |c| lower_(c) || c.is_ascii_digit());
        (e > m + 2 && bound_after(b, e)).then_some(e)
    })
    .into_iter()
    .map(|(s, e)| &text[s..e])
    .collect()
}

/// `` `[a-z_]+::[a-z_]+::[a-z_0-9]+` ``, backticks removed.
fn ticked(text: &str) -> Vec<&str> {
    let b = text.as_bytes();
    find_all(b, |b, i| {
        if b[i] != b'`' {
            return None;
        }
        let a = run(b, i + 1, lower_);
        if a == i + 1 || !has_at(b, a, b"::") {
            return None;
        }
        let m = run(b, a + 2, lower_);
        if m == a + 2 || !has_at(b, m, b"::") {
            return None;
        }
        let e = run(b, m + 2, |c| lower_(c) || c.is_ascii_digit());
        (e > m + 2 && b.get(e) == Some(&b'`')).then_some(e + 1)
    })
    .into_iter()
    .map(|(s, e)| &text[s + 1..e - 1])
    .collect()
}

/// `crates/[A-Za-z0-9_./-]*/(tests|benches)/[A-Za-z0-9_.-]+\.rs`, longest.
fn harness_paths(text: &str) -> Vec<&str> {
    let class = |c: u8| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'.' | b'/' | b'-');
    let valid = |p: &[u8]| -> bool {
        let Some(q) = p.strip_suffix(b".rs") else {
            return false;
        };
        let Some(slash) = q.iter().rposition(|&c| c == b'/') else {
            return false;
        };
        let before = &q[..slash];
        slash + 1 < q.len() && (before.ends_with(b"/tests") || before.ends_with(b"/benches"))
    };
    let b = text.as_bytes();
    find_all(b, |b, i| {
        if !has_at(b, i, b"crates/") {
            return None;
        }
        let s = i + 7;
        let r = run(b, s, class);
        (s + 1..=r).rev().find(|&e| valid(&b[s..e]))
    })
    .into_iter()
    .map(|(s, e)| &text[s..e])
    .collect()
}

/// `\b[A-Z]+(-[A-Z]+)?-[0-9][0-9]\b`, the longer form preferred.
fn row_ids(text: &str) -> Vec<&str> {
    let b = text.as_bytes();
    let digits = |b: &[u8], j: usize| -> Option<usize> {
        (b.get(j) == Some(&b'-')
            && b.get(j + 1).is_some_and(u8::is_ascii_digit)
            && b.get(j + 2).is_some_and(u8::is_ascii_digit)
            && bound_after(b, j + 3))
        .then_some(j + 3)
    };
    find_all(b, |b, i| {
        if !b[i].is_ascii_uppercase() || !bound_before(b, i) {
            return None;
        }
        let a = run(b, i, |c| c.is_ascii_uppercase());
        let two = (b.get(a) == Some(&b'-'))
            .then(|| run(b, a + 1, |c| c.is_ascii_uppercase()))
            .filter(|&k| k > a + 1)
            .and_then(|k| digits(b, k));
        two.or_else(|| digits(b, a))
    })
    .into_iter()
    .map(|(s, e)| &text[s..e])
    .collect()
}

/// The first `^\| *ID *\|` line of the invariants document.
fn invariant_row<'a>(inv: &'a str, id: &str) -> Option<&'a str> {
    lines(inv).into_iter().find(|l| {
        let b = l.as_bytes();
        if b.first() != Some(&b'|') {
            return false;
        }
        let s = run(b, 1, |c| c == b' ');
        has_at(b, s, id.as_bytes()) && {
            let e = run(b, s + id.len(), |c| c == b' ');
            b.get(e) == Some(&b'|')
        }
    })
}

#[derive(Debug, PartialEq)]
enum Proof {
    Test,
    Path,
    Row,
}

fn first_and_last(token: &str) -> (String, String) {
    let first = token.split("::").next().unwrap_or("");
    let last = token.rsplit("::").next().unwrap_or("");
    (first.to_owned(), last.to_owned())
}

/// The kind of proof a block names, tried in rule order, or none.
fn prove(
    repo: &dyn Repo,
    text: &str,
    proving: &BTreeSet<(String, String)>,
    inv: &str,
) -> Result<Option<Proof>, String> {
    if qualified(text)
        .into_iter()
        .any(|t| proving.contains(&first_and_last(t)))
    {
        return Ok(Some(Proof::Test));
    }
    for p in harness_paths(text) {
        if repo.tracked(p)? {
            return Ok(Some(Proof::Path));
        }
    }
    for r in row_ids(text) {
        if let Some(row) = invariant_row(inv, r)
            && ticked(row)
                .into_iter()
                .any(|t| proving.contains(&first_and_last(t)))
        {
            return Ok(Some(Proof::Row));
        }
    }
    Ok(None)
}

/// Fields as the default field splitter sees them: runs of blank, tab, newline.
fn fields(l: &str) -> Vec<&str> {
    l.split([' ', '\t', '\n'])
        .filter(|f| !f.is_empty())
        .collect()
}

fn gate12(repo: &dyn Repo, allow: &str, out: &mut Vec<String>) -> Result<bool, String> {
    let inv = repo.read(INVARIANTS)?;
    let mut fns = BTreeSet::new();
    let mut proving = BTreeSet::new();
    for manifest in repo.ls("crates/*/Cargo.toml")? {
        let krate = manifest
            .strip_prefix("crates/")
            .and_then(|m| m.strip_suffix("/Cargo.toml"))
            .ok_or_else(|| format!("{manifest}: not a crates/*/Cargo.toml path"))?;
        for path in repo.ls(&format!("crates/{krate}/*.rs"))? {
            fn_tables(krate, &path, &repo.read(&path)?, &mut fns, &mut proving);
        }
    }
    let mut blocks = Vec::new();
    let files = repo.ls("crates/*.rs")?;
    for f in &files {
        blocks.extend(claim_blocks(f, &repo.read(f)?));
    }
    out.push(format!(
        "walked {} tracked .rs file(s) under crates/",
        files.len()
    ));
    out.push(format!(
        "  {} (crate, fn) pair(s) in the workspace,",
        fns.len()
    ));
    out.push(format!(
        "  {} of them a test or a bench — only those prove a claim",
        proving.len()
    ));
    out.push(format!("  {} doc block(s)", blocks.len()));
    if files.is_empty() || blocks.is_empty() {
        out.push("GATE 12 WALKED NOTHING. A gate that reads no doc block is not a gate.".into());
        return Ok(false);
    }

    let mut refused: Vec<(&str, usize, &str)> = Vec::new();
    let (mut claimed, mut by_test, mut by_path, mut by_row, mut by_unverified) = (0, 0, 0, 0, 0);
    for b in &blocks {
        if !is_claim(&b.text) {
            continue;
        }
        claimed += 1;
        if b.text.contains("UNVERIFIED") {
            by_unverified += 1;
            continue;
        }
        match prove(repo, &b.text, &proving, &inv)? {
            Some(Proof::Test) => by_test += 1,
            Some(Proof::Path) => by_path += 1,
            Some(Proof::Row) => by_row += 1,
            None => refused.push((&b.file, b.line, &b.item)),
        }
    }
    out.push(format!("  {claimed} block(s) make a cost claim"));
    out.push(format!("    {by_test} name a test that exists"));
    out.push(format!(
        "    {by_row} name an invariant row whose test exists"
    ));
    out.push(format!("    {by_path} name a tracked test or bench file"));
    out.push(format!("    {by_unverified} say UNVERIFIED"));
    out.push(format!("    {} name nothing", refused.len()));
    if claimed == 0 {
        out.push(String::new());
        out.push(format!(
            "GATE 12 FOUND NO CLAIM AT ALL in {} doc blocks. Either every",
            blocks.len()
        ));
        out.push("comment stopped saying O(1) in the same commit, or the trigger".into());
        out.push("stopped matching. Both need a human; a silent zero does not.".into());
        return Ok(false);
    }

    let mut bad = false;
    out.push(String::new());
    // KEYED BY ITEM, NOT BY COUNT (D-1116): an entry excuses the one block
    // whose item it names, and never a block with no item.
    let entries: Vec<Vec<&str>> = allow.lines().map(fields).collect();
    for &(f, ln, it) in &refused {
        if it != "-"
            && entries
                .iter()
                .any(|e| e.first() == Some(&f) && e.get(1) == Some(&it))
        {
            out.push(format!("  allowed  {f}:{ln} ({it}) — declared above"));
        } else {
            out.push(format!(
                "  REFUSED  {f}:{ln} ({it}) — a cost claim that names no proof"
            ));
            bad = true;
        }
    }
    // The two self-audits: an entry naming a file that is gone is a failure,
    // an entry that no longer matches is only loose.
    for e in &entries {
        let Some(&p) = e.first() else {
            continue;
        };
        let it = e.get(1).copied().unwrap_or("");
        let extra = e.get(2..).map(|x| x.join(" ")).unwrap_or_default();
        if it.is_empty() || !extra.is_empty() || it == "-" {
            out.push(format!(
                "  MALFORMED ALLOWLIST ENTRY — \"{p} {it} {extra}\" is not FILE ITEM"
            ));
            bad = true;
        } else if !repo.tracked(p)? {
            out.push(format!(
                "  STALE ALLOWLIST ENTRY — {p} is not a tracked file"
            ));
            bad = true;
        } else if !refused.iter().any(|&(f, _, i)| f == p && i == it) {
            out.push(format!(
                "  ::warning title=Gate 12 allowlist is loose::{p} {it} claims nothing unproven any more — drop the entry"
            ));
        }
    }

    out.push(String::new());
    if bad {
        out.extend(
            [
                "GATE 12 FAILED.",
                "",
                "A cost claim with no proof beside it is the defect this gate",
                "was written for. Name the test — crate::module::name, an",
                "invariant row id, or the bench file — or write UNVERIFIED and",
                "record it in docs/06-limits.md. CLAUDE.md section 3 rule 6.",
            ]
            .map(String::from),
        );
        return Ok(false);
    }
    out.push("OK — every cost claim names its proof or says UNVERIFIED.".into());
    Ok(true)
}

// ---------------------------------------------------------------- gate 14 --

/// The `name` and `harness` of every `[[bench]]` table, read the way the old
/// line reader read them: a table ends at the next line beginning `[`, a
/// second `[[bench]]` straight after a first discards the first (so a bench
/// cannot be vouched for by a table it does not end), `name` is the text
/// between the last `="` and the next `"`, and `harness` the word after the
/// last `=`, cut at whitespace or `#`.
fn bench_decls(manifest: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut inb = false;
    let (mut nm, mut hz) = (String::new(), String::new());
    let key = |l: &str, k: &str| -> bool {
        let b = l.as_bytes();
        let s = run(b, 0, space);
        has_at(b, s, k.as_bytes()) && b.get(run(b, s + k.len(), space)) == Some(&b'=')
    };
    for l in lines(manifest) {
        if l.starts_with("[[bench]]") {
            inb = true;
            nm.clear();
            hz.clear();
            continue;
        }
        if l.starts_with('[') && inb {
            out.push((std::mem::take(&mut nm), std::mem::take(&mut hz)));
            inb = false;
        }
        if !inb {
            continue;
        }
        let b = l.as_bytes();
        if key(l, "name") {
            // sub(/.*=[[:space:]]*"/, ""): the last `"` that follows `=`.
            let cut = (0..b.len()).rev().find(|&q| {
                b[q] == b'"' && {
                    let mut k = q;
                    while k > 0 && space(b[k - 1]) {
                        k -= 1;
                    }
                    k > 0 && b[k - 1] == b'='
                }
            });
            let v = cut.map_or(l, |q| &l[q + 1..]);
            nm = v.split('"').next().unwrap_or("").to_owned();
        }
        if key(l, "harness") {
            // sub(/.*=[[:space:]]*/, "") then sub(/[[:space:]#].*/, "").
            let v = l.rfind('=').map_or(l, |q| &l[q + 1..]);
            let v = &v[run(v.as_bytes(), 0, space)..];
            let end = v
                .bytes()
                .position(|c| space(c) || c == b'#')
                .unwrap_or(v.len());
            hz = v[..end].to_owned();
        }
    }
    if inb {
        out.push((nm, hz));
    }
    out
}

/// Measurement points: lines matching `ratio[ \t]*\(` and not declaring a
/// helper, `(^|[^A-Za-z0-9_])fn[ \t]+[A-Za-z0-9_]*ratio[ \t]*[(<]`.
fn ratio_points(code: &str) -> usize {
    let blank = |c: u8| c == b' ' || c == b'\t';
    lines(code)
        .into_iter()
        .filter(|l| {
            let b = l.as_bytes();
            let call = (0..b.len())
                .any(|i| has_at(b, i, b"ratio") && b.get(run(b, i + 5, blank)) == Some(&b'('));
            let decl = (0..b.len()).any(|p| {
                if !bound_before(b, p) || !has_at(b, p, b"fn") {
                    return false;
                }
                let s = run(b, p + 2, blank);
                let e = run(b, s, word);
                s > p + 2
                    && b[s..e].ends_with(b"ratio")
                    && matches!(b.get(run(b, e, blank)), Some(b'(' | b'<'))
            });
            call && !decl
        })
        .count()
}

/// `cut -d: -f3-` of one line, then its first blank-separated field.
fn string_head(line: &str) -> &str {
    let rest = if line.contains(':') {
        line.splitn(3, ':').nth(2).unwrap_or("")
    } else {
        line
    };
    fields(rest).first().copied().unwrap_or("")
}

/// The scanner's lexed view of the benches, split back into one per file.
/// `code` keeps every newline of its input (it blanks comment bytes and drops
/// `r#` prefixes, never a line), so each file's view is exactly as many lines
/// as the file has; a file that is not last must end in a newline, or its last
/// line would run into the next file's first.
#[derive(Debug, Default)]
struct Lexed {
    code: BTreeMap<String, String>,
    strings: Vec<String>,
}

fn split_lexed(
    repo: &dyn Repo,
    listing: &[String],
    code: &str,
    strings: &str,
) -> Result<Lexed, String> {
    let mut rest = code;
    let mut views = BTreeMap::new();
    for (k, f) in listing.iter().enumerate() {
        let src = repo.read(f)?;
        let last = k + 1 == listing.len();
        if !last && !src.is_empty() && !src.ends_with('\n') {
            return Err(format!(
                "{f} does not end in a newline, so its lexed view cannot be told apart from the next file's"
            ));
        }
        let mut cut = 0;
        for _ in 0..src.matches('\n').count() {
            cut += rest[cut..]
                .find('\n')
                .ok_or_else(|| format!("the lexed view ends inside {f}"))?
                + 1;
        }
        if !src.ends_with('\n') && !src.is_empty() {
            if rest[cut..].contains('\n') {
                return Err(format!("the lexed view of {f} has more lines than {f}"));
            }
            cut = rest.len();
        }
        views.insert(f.clone(), rest[..cut].to_owned());
        rest = &rest[cut..];
    }
    if !rest.is_empty() {
        return Err("the lexed view is longer than the files it was made from".into());
    }
    Ok(Lexed {
        code: views,
        strings: lines(strings).into_iter().map(str::to_owned).collect(),
    })
}

/// Did one `source_scan step-runs` call answer yes? It prints exactly one
/// `present` line when it does, and `REFUSED` lines or nothing when not.
fn step_runs_ok(output: &str) -> bool {
    let l = lines(output);
    !l.is_empty() && l.iter().all(|x| x.starts_with("present  "))
}

/// `^[[:space:]]+LINE[[:space:]]*$` in the workflow.
fn exact_line(wf: &str, needle: &str) -> bool {
    lines(wf).into_iter().any(|l| {
        let b = l.as_bytes();
        let s = run(b, 0, space);
        s > 0 && has_at(b, s, needle.as_bytes()) && run(b, s + needle.len(), space) == b.len()
    })
}

const BENCH_LINE: &str = "cargo bench --workspace --locked";
const EMPTY_LINE: &str = "git ls-files --error-unmatch 'crates/*/benches/*.rs' > /dev/null";

fn gate14(
    repo: &dyn Repo,
    cover: &str,
    lexed: &Lexed,
    runs: [&str; 2],
    out: &mut Vec<String>,
) -> Result<bool, String> {
    let inv = repo.read(INVARIANTS)?;
    let mut bad = false;
    out.push("the claim reader is one function, shared by gates 12 and 14".into());

    let files: Vec<String> = repo
        .ls("crates/*.rs")?
        .into_iter()
        .filter(|f| f.contains("/src/") && !f.contains("/tests/") && !f.contains("/benches/"))
        .collect();
    let mut nb = 0;
    // (crate, file, line), crate being the second `/` field of the path.
    let mut claims: Vec<(String, String, usize)> = Vec::new();
    for f in &files {
        for b in claim_blocks(f, &repo.read(f)?) {
            nb += 1;
            if is_claim(&b.text) {
                let krate = f.split('/').nth(1).unwrap_or(f).to_owned();
                claims.push((krate, b.file, b.line));
            }
        }
    }
    let ncr = repo.ls("crates/*/Cargo.toml")?.len();
    let nfield = cover.lines().filter(|l| !fields(l).is_empty()).count();
    out.push(format!(
        "walked {} tracked source file(s) under crates/*/src/",
        files.len()
    ));
    out.push(format!(
        "  {nb} doc block(s), {} of them make a cost claim",
        claims.len()
    ));
    out.push(format!("  {ncr} tracked crate manifest(s)"));
    out.push(format!("  {nfield} row(s) in the coverage table"));
    if files.is_empty() || ncr == 0 {
        out.push("GATE 14 WALKED NOTHING. A gate that reads no source file and".into());
        out.push("no manifest is not a gate.".into());
        return Ok(false);
    }
    if claims.is_empty() {
        out.push(format!(
            "GATE 14 FOUND NO COST CLAIM AT ALL in {nb} doc blocks. Either"
        ));
        out.push("every crate stopped claiming a bound in the same commit, or".into());
        out.push("the trigger stopped matching gate 12's. Both need a human.".into());
        return Ok(false);
    }

    // The table's rows: lines with a non-space byte, split into fields.
    let rows: Vec<Vec<&str>> = cover
        .lines()
        .filter(|l| !l.bytes().all(space))
        .map(fields)
        .collect();

    out.push(String::new());
    out.push("layer 1: a crate that claims a bound is named in the table".into());
    let crates: BTreeSet<&str> = claims.iter().map(|(c, _, _)| c.as_str()).collect();
    for c in crates {
        let mine: Vec<_> = claims.iter().filter(|(k, _, _)| k == c).collect();
        if rows.iter().any(|r| r.first() == Some(&c)) {
            out.push(format!(
                "  covered  crates/{c} — {} cost claim(s)",
                mine.len()
            ));
        } else {
            out.push(format!(
                "  REFUSED  crates/{c} — {} cost claim(s), no row in the table",
                mine.len()
            ));
            for (_, f, ln) in mine.iter().take(8) {
                out.push(format!("             doc block at {f}:{ln}"));
            }
            bad = true;
        }
    }

    out.push(String::new());
    out.push("layer 2: the named bench is tracked and its exit code reaches cargo".into());
    out.push("layer 3: it has the shape of a ratio measurement".into());
    out.push("layer 4: the invariant rows it claims to print are real and present".into());
    for row in &rows {
        let c = row.first().copied().unwrap_or("");
        let b = row.get(1).copied().unwrap_or("");
        let pts = row.get(2).copied().unwrap_or("");
        let ids = row.get(3..).unwrap_or(&[]);
        let m = format!("crates/{c}/Cargo.toml");

        // A row naming a crate or a bench that is not tracked makes this gate
        // read stronger than it is, so it is a failure and not a note.
        if !repo.tracked(&m)? {
            out.push(format!(
                "  STALE TABLE ROW — crates/{c} has no tracked manifest"
            ));
            bad = true;
            continue;
        }
        if !repo.tracked(b)? {
            out.push(format!(
                "  REFUSED  {b} is not tracked — the row promises a bench that"
            ));
            out.push("           does not exist in this checkout".into());
            bad = true;
            continue;
        }
        if !claims.iter().any(|(k, _, _)| k == c) {
            out.push(format!(
                "  ::warning title=Gate 14 table row is loose::crates/{c} claims no bound any more — drop the row"
            ));
        }

        // layer 2b. `harness = false` under a [[bench]] named for the file.
        let stem = b.rsplit('/').next().unwrap_or(b);
        let stem = stem.strip_suffix(".rs").unwrap_or(stem);
        let declared = repo
            .read(&m)
            .map(|t| bench_decls(&t))
            .unwrap_or_default()
            .iter()
            .any(|(nm, hz)| nm == stem && hz == "false");
        if declared {
            out.push(format!(
                "  ok       {b} — [[bench]] {stem}, harness = false"
            ));
        } else {
            out.push(format!(
                "  REFUSED  {b} — {m} declares no [[bench]] named {stem} with"
            ));
            out.push("           harness = false. Under the default harness the".into());
            out.push("           bench cannot fail the build at all.".into());
            bad = true;
        }

        // layer 3. THE SHAPE, not the truth: call sites in the comment-blanked
        // view, a ceiling named twice, timing, black_box, printing, exiting.
        let Some(code) = lexed.code.get(b) else {
            out.push(format!(
                "  REFUSED  {b} could not be lexed (only tracked crates/*/benches/*.rs are)"
            ));
            bad = true;
            continue;
        };
        let raw = repo.read(b)?;
        let npt = ratio_points(code);
        let nce = lines(&raw)
            .into_iter()
            .filter(|l| l.contains("CEILING"))
            .count();
        let mut miss = String::new();
        if nce < 2 {
            miss.push_str(" a-named-ceiling-compared-against");
        }
        if pts.parse::<i64>().ok() != i64::try_from(npt).ok() {
            miss.push_str(&format!(" {npt}-measurement-points-but-pinned-at-{pts}"));
        }
        for (needle, why) in [
            ("Instant", " nothing-is-timed"),
            ("black_box", " no-black_box"),
            ("println!", " prints-nothing"),
            ("process::exit", " cannot-exit-non-zero"),
        ] {
            if !raw.contains(needle) {
                miss.push_str(why);
            }
        }
        if miss.is_empty() {
            out.push(format!(
                "  ok       {b} — {npt} point(s) = pin {pts}, ceiling compared, prints, exits"
            ));
        } else {
            out.push(format!("  REFUSED  {b} is not a ratio bench:{miss}"));
            bad = true;
        }

        // layer 4. Each id is a real row AND the first word of a string
        // literal the bench prints (D-1117).
        let heads: BTreeSet<&str> = lexed
            .strings
            .iter()
            .filter(|l| {
                l.strip_prefix(b)
                    .and_then(|r| r.strip_prefix(':'))
                    .is_some_and(|r| {
                        let d = run(r.as_bytes(), 0, |c| c.is_ascii_digit());
                        d > 0 && r.as_bytes().get(d) == Some(&b':')
                    })
            })
            .map(|l| string_head(l))
            .collect();
        for r in ids {
            let inrow = u8::from(invariant_row(&inv, r).is_some());
            let inbench = u8::from(heads.contains(r));
            if inrow == 1 && inbench == 1 {
                out.push(format!(
                    "  ok       {b} — {r} is a row in {INVARIANTS} and is printed"
                ));
            } else {
                out.push(format!(
                    "  REFUSED  {b} — {r}: in-invariants={inrow} in-bench={inbench}"
                ));
                bad = true;
            }
        }
    }
    if rows.is_empty() {
        out.push("  THE COVERAGE TABLE IS EMPTY. Layer 1 above then passes only".into());
        out.push("  when no crate claims a bound, and that is not this repository.".into());
        bad = true;
    }

    // layer 5: gate 8 still runs what this gate counted, unconditionally
    // (`source_scan step-runs`), and as an exact line, so a `|| true` tail
    // cannot satisfy it.
    out.push(String::new());
    out.push("layer 5: gate 8 still runs what this gate counted".into());
    let wf = repo.read(WORKFLOW)?;
    for l in lines(runs[0]) {
        out.push(format!("  {l}"));
    }
    if step_runs_ok(runs[0]) && exact_line(&wf, BENCH_LINE) {
        out.push(
            "  present  gate 8 invokes cargo bench --workspace --locked, unconditionally".into(),
        );
    } else {
        out.push("  REFUSED  nothing in this workflow RUNS the benches so that their".into());
        out.push("           failure fails the run. Every ceiling above is then an".into());
        out.push("           assertion no machine makes. Reasons above.".into());
        bad = true;
    }
    for l in lines(runs[1]) {
        out.push(format!("  {l}"));
    }
    if step_runs_ok(runs[1]) && exact_line(&wf, EMPTY_LINE) {
        out.push("  present  gate 8 still refuses an empty bench set".into());
    } else {
        out.push("  REFUSED  gate 8 no longer fails when no bench exists. A skip".into());
        out.push("           that reports success is the fallback CLAUDE.md".into());
        out.push("           section 4 bans, and it is what gate 8 shipped with.".into());
        bad = true;
    }

    out.push(String::new());
    if bad {
        out.extend(
            [
                "GATE 14 FAILED.",
                "",
                "A crate that promises a constant bound and never re-measures it",
                "is the defect this gate was written for — the manifest append",
                "and the page render both regressed inside a green build. Write",
                "the bench, wire it with harness = false, and add its row to the",
                "table above. If the bound is being abandoned, say so in",
                "docs/05-decisions.md and delete the claim; do not delete the",
                "row and keep the sentence. CLAUDE.md section 3 rules 4 and 6.",
            ]
            .map(String::from),
        );
        return Ok(false);
    }
    out.push("OK — every crate that claims a bound names a bench that measures it.".into());
    Ok(true)
}

// ------------------------------------------------------------------- main --

fn read_work(dir: &str, name: &str) -> Result<String, String> {
    Git.read(&format!("{dir}/{name}"))
}

fn run_gate(args: &[String], out: &mut Vec<String>) -> Result<bool, String> {
    match args {
        [cmd, allow] if cmd == "gate12" => gate12(&Git, allow, out),
        [cmd, cover, work] if cmd == "gate14" => {
            let listing: Vec<String> = std::fs::read(format!("{work}/benches"))
                .map_err(|e| format!("{work}/benches: {e}"))?
                .split(|&c| c == 0)
                .filter(|p| !p.is_empty())
                .map(|p| String::from_utf8_lossy(p).into_owned())
                .collect();
            let lexed = split_lexed(
                &Git,
                &listing,
                &read_work(work, "benches.code")?,
                &read_work(work, "benches.strings")?,
            )?;
            let runs = [
                read_work(work, "runs-bench")?,
                read_work(work, "runs-empty")?,
            ];
            gate14(&Git, cover, &lexed, [&runs[0], &runs[1]], out)
        }
        _ => Err("usage: gates_bounds gate12 ALLOW | gate14 COVER WORKDIR".into()),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut out = Vec::new();
    let verdict = run_gate(&args, &mut out);
    for l in &out {
        println!("{l}");
    }
    match verdict {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(1),
        Err(e) => {
            eprintln!("gates_bounds: {e}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tracked tree in memory. A pathspec `*` matches across `/`, as git's
    /// default pathspec does; `tracked` also accepts a directory prefix.
    struct Fake(BTreeMap<String, String>);

    fn glob(p: &[u8], s: &[u8]) -> bool {
        match p.split_first() {
            None => s.is_empty(),
            Some((b'*', rest)) => (0..=s.len()).any(|k| glob(rest, &s[k..])),
            Some((c, rest)) => s.first() == Some(c) && glob(rest, &s[1..]),
        }
    }

    impl Repo for Fake {
        fn ls(&self, pathspec: &str) -> Result<Vec<String>, String> {
            Ok(self
                .0
                .keys()
                .filter(|k| glob(pathspec.as_bytes(), k.as_bytes()))
                .cloned()
                .collect())
        }
        fn tracked(&self, p: &str) -> Result<bool, String> {
            Ok(!p.is_empty()
                && self
                    .0
                    .keys()
                    .any(|k| k == p || k.starts_with(&format!("{p}/"))))
        }
        fn read(&self, path: &str) -> Result<String, String> {
            self.0
                .get(path)
                .cloned()
                .ok_or_else(|| format!("{path}: missing"))
        }
    }

    fn fake(files: &[(&str, &str)]) -> Fake {
        Fake(
            files
                .iter()
                .map(|(a, b)| ((*a).to_owned(), (*b).to_owned()))
                .collect(),
        )
    }

    fn with(base: &[(&'static str, &'static str)], extra: &[(&'static str, &'static str)]) -> Fake {
        let mut v = base.to_vec();
        for e in extra {
            v.retain(|(k, _)| k != &e.0);
            v.push(*e);
        }
        fake(&v)
    }

    // ---- the reader ----

    #[test]
    fn a_block_names_the_item_below_it() {
        let src = "/// O(1).\n#[inline]\n#[cfg(all(\n  test,\n))]\n\n// note\npub(crate) fn hot_path() {}\n";
        let b = claim_blocks("f.rs", src);
        assert_eq!(b.len(), 1);
        assert_eq!(b[0].line, 1);
        assert_eq!(b[0].item, "hot_path");
        assert_eq!(b[0].text, "  O(1).");
    }

    #[test]
    fn items_of_every_kind_and_none() {
        for (code, want) in [
            ("pub struct Ledger {", "Ledger"),
            ("enum E { A }", "E"),
            ("pub const fn x() {}", "x"),
            ("static S: u8 = 0;", "S"),
            ("macro_rules! m {", "m"),
            ("impl Foo {", "impl"),
            ("impl<T> Foo for T {", "impl"),
            ("let x = 1;", "-"),
            ("myfn x() {}", "-"),
            ("pub mod  inner;", "inner"),
            ("type\tT = u8;", "T"),
        ] {
            let src = format!("/// doc\n{code}\n");
            assert_eq!(claim_blocks("f.rs", &src)[0].item, want, "{code}");
        }
    }

    #[test]
    fn an_inner_block_ends_at_the_first_other_line() {
        let src = "//! O(1) module.\n//! more\n\n/// O(1)\nfn a() {}\n";
        let b = claim_blocks("f.rs", src);
        assert_eq!(b.len(), 2);
        assert_eq!((b[0].item.as_str(), b[0].line), ("//!", 1));
        assert_eq!(b[0].text, "  O(1) module.  more");
        assert_eq!((b[1].item.as_str(), b[1].line), ("a", 4));
    }

    #[test]
    fn a_block_separated_from_the_next_by_blanks_has_no_item() {
        let src = "/// first\n\n/// second\nfn b() {}\n";
        let b = claim_blocks("f.rs", src);
        assert_eq!(b[0].item, "-");
        assert_eq!(b[1].item, "b");
        // A doc line inside an open attribute is part of the attribute.
        let src = "/// a\n#[doc = [\n/// inside\n]]\nfn c() {}\n";
        let b = claim_blocks("f.rs", src);
        assert_eq!(b.len(), 1);
        assert_eq!(b[0].item, "c");
        // A block at end of file is still emitted.
        assert_eq!(claim_blocks("f.rs", "fn x() {}\n/// tail")[0].item, "-");
    }

    #[test]
    fn crlf_tabs_and_unicode_are_read_as_bytes() {
        // A `\r`-only line is not blank, so it is the "code line" below.
        let src = "/// O(1)\r\n\r\nfn a() {}\r\n";
        assert_eq!(claim_blocks("f.rs", src)[0].item, "-");
        let src = "\t///\tflat\t— é\nfn é() {}\n";
        let b = claim_blocks("f.rs", src);
        assert_eq!(b[0].text, "  flat — é");
        assert_eq!(b[0].item, "-");
        // `////` is a doc line too, read from its fourth byte.
        assert_eq!(claim_blocks("f.rs", "//// x\nfn y() {}\n")[0].text, " / x");
    }

    #[test]
    fn the_scrub_removes_only_the_named_senses() {
        let s = |t: &str| String::from_utf8(claim_scrub(t)).unwrap();
        assert_eq!(s("Bar::flat(x)"), "Ba (x)");
        assert_eq!(s("a flat  (x)"), "a  (x)");
        assert_eq!(s("the worst-case FILLS and Worst Case"), "the  FILLS and ");
        assert_eq!(
            s("flat files, flat  directories, Flat tablespoon"),
            ", , poon"
        );
        assert_eq!(s("a flat JSON object here"), "a  here");
        assert_eq!(s("came out flat; is charged Flat; rA flat"), "; ; r");
        // Not rescanned: removing the inner phrase re-forms the outer one.
        assert_eq!(s("worst worst casecase"), "worst case");
        assert!(is_claim("worst worst casecase"));
    }

    #[test]
    fn the_trigger_and_its_word_boundaries() {
        for yes in [
            "is O(1)",
            "o(1) amortised",
            "constant-time",
            "constant time",
            "never scans",
            "the fee is flat",
            "flat.",
            "é flat",
        ] {
            assert!(is_claim(yes), "{yes}");
        }
        for no in [
            "O(n)",
            "inflate",
            "flatten",
            "Flat",
            "a flat table",
            "worst case fills",
            "Bar::flat",
            "flat_map",
        ] {
            assert!(!is_claim(no), "{no}");
        }
    }

    // ---- gate 12's tokens ----

    #[test]
    fn qualified_tokens_need_word_boundaries() {
        assert_eq!(
            qualified("see core::a::b_1, Xcore::a::b and x::y::zA and a::b::c::d"),
            vec!["core::a::b_1", "a::b::c"]
        );
        assert_eq!(qualified("ab9::x::y"), Vec::<&str>::new());
    }

    #[test]
    fn harness_paths_are_longest_and_need_a_test_dir() {
        assert_eq!(
            harness_paths("see crates/x/tests/a.rs.rs and crates//benches/b.rs, crates/x/src/c.rs"),
            vec!["crates/x/tests/a.rs.rs", "crates//benches/b.rs"]
        );
        assert_eq!(harness_paths("crates/x/tests/.rs"), Vec::<&str>::new());
    }

    #[test]
    fn row_ids_prefer_the_cost_family() {
        assert_eq!(
            row_ids("C-E-07 E-07 D-0095 C-09b X-C-E-07 AU-05 C-CLI-06"),
            vec!["C-E-07", "E-07", "C-E-07", "AU-05", "C-CLI-06"]
        );
    }

    #[test]
    fn ticked_tokens_are_whole_spans() {
        assert_eq!(
            ticked("| x | `a::b::c`, `d::e::f``g::h::i` `j::K::l` `x `m::n::o`"),
            vec!["a::b::c", "d::e::f", "g::h::i", "m::n::o"]
        );
    }

    #[test]
    fn invariant_rows_match_whole_ids() {
        let inv = "| E-07 | wrong |\n|  C-E-07  | right |\n";
        assert_eq!(invariant_row(inv, "C-E-07"), Some("|  C-E-07  | right |"));
        assert_eq!(invariant_row(inv, "C-E-0"), None);
    }

    #[test]
    fn only_tests_and_benches_prove() {
        let mut fns = BTreeSet::new();
        let mut proving = BTreeSet::new();
        let src = "#[test]\n\nfn a_test() {}\nfn production() {}\n#[cfg(test)]\nmod tests {\n    fn helper() {}\n    #[tokio::test]\n    async fn b_test() {}\n}\n// fn in_prose() {}\n    * fn starred() {}\n#[doc = \"latest\"]\npub fn counted_by_its_attribute() {}\n";
        fn_tables("api", "crates/api/src/lib.rs", src, &mut fns, &mut proving);
        let names =
            |s: &BTreeSet<(String, String)>| s.iter().map(|(_, n)| n.clone()).collect::<Vec<_>>();
        assert_eq!(
            names(&proving),
            vec!["a_test", "b_test", "counted_by_its_attribute"]
        );
        assert!(names(&fns).contains(&"production".to_owned()));
        assert!(!names(&fns).contains(&"in_prose".to_owned()));
        assert!(!names(&fns).contains(&"starred".to_owned()));
        fn_tables(
            "api",
            "crates/api/benches/r.rs",
            "fn main() {}\n",
            &mut fns,
            &mut proving,
        );
        assert!(proving.contains(&("api".to_owned(), "main".to_owned())));
    }

    // ---- gate 12 end to end ----

    const G12: &[(&str, &str)] = &[
        ("crates/api/Cargo.toml", ""),
        (
            "crates/api/src/lib.rs",
            "/// O(1); see `api::tests::fast`.\nfn hot() {}\n#[test]\nfn fast() {}\n",
        ),
        ("crates/api/tests/t.rs", "fn proof() {}\n"),
        ("crates/core/Cargo.toml", ""),
        ("crates/core/src/lib.rs", "/// Plain prose.\nfn x() {}\n"),
        (
            "docs/04-invariants.md",
            "| C-01 | x | `api::t::proof` |\n| C-02 | x | `api::lib::hot` |\n",
        ),
    ];

    fn g12(tree: &Fake, allow: &str) -> (bool, String) {
        let mut out = Vec::new();
        let ok = gate12(tree, allow, &mut out).unwrap();
        (ok, out.join("\n"))
    }

    #[test]
    fn gate12_passes_a_proven_tree_and_counts_each_kind() {
        let tree = with(
            G12,
            &[(
                "crates/core/src/more.rs",
                "/// constant-time, see crates/api/tests/t.rs\nfn p() {}\n/// O(1), row C-01.\nfn r() {}\n/// O(1) UNVERIFIED\nfn u() {}\n",
            )],
        );
        let (ok, log) = g12(&tree, "");
        assert!(ok, "{log}");
        assert!(log.contains("  4 block(s) make a cost claim"), "{log}");
        assert!(log.contains("    1 name a test that exists"), "{log}");
        assert!(
            log.contains("    1 name an invariant row whose test exists"),
            "{log}"
        );
        assert!(
            log.contains("    1 name a tracked test or bench file"),
            "{log}"
        );
        assert!(log.contains("    1 say UNVERIFIED"), "{log}");
    }

    #[test]
    fn gate12_refuses_a_claim_proven_by_its_own_subject() {
        for text in [
            "/// O(1); see `api::lib::hot`.\nfn hot2() {}\n",
            "/// O(1); see crates/api/tests/gone.rs\nfn hot2() {}\n",
            "/// O(1); see C-02, whose test is production code.\nfn hot2() {}\n",
            "/// O(1); see E-01, a row that does not exist.\nfn hot2() {}\n",
            "/// It is flat in cost.\nfn hot2() {}\n",
        ] {
            let tree = with(G12, &[("crates/core/src/lib.rs", text)]);
            let (ok, log) = g12(&tree, "");
            assert!(!ok, "{text}");
            assert!(
                log.contains(
                    "  REFUSED  crates/core/src/lib.rs:1 (hot2) — a cost claim that names no proof"
                ),
                "{log}"
            );
            assert!(log.contains("GATE 12 FAILED."), "{log}");
        }
    }

    /// The `g12-adv` adversarial tree of D-2314, each block refused by the
    /// old step too: CRLF and non-ASCII text, a proof token split across two
    /// doc lines, a multi-line attribute holding brackets in a string, an
    /// inner block whose only trigger is the predicate `flat`, an untracked
    /// bench path, row ids that do not exist (and `XC-E-07`, which is not an
    /// id at all), and a production function named as its own proof.
    #[test]
    fn gate12_refuses_every_adversarial_block() {
        let adv = "//! The fee is flat, and Bar::flat(x) is not the word.\n\n/// O(1); see core::adv::\n/// split_proof, which is a test.\nfn split() {}\n\n#[test]\nfn split_proof() {}\n\n/// never scans the map\n#[cfg_attr(\n    test,\n    doc = \"][\"\n)]\nfn multi() {}\n\n/// O(1): crates/core/benches/missing.rs measures it.\nfn pathed() {}\n\n/// O(1); see C-ZZ-01 and XC-E-07.\nfn rowed() {}\n\n/// Amortised O(1) — é — see `core::adv::lookup`.\nfn évalue() {}\n";
        let crlf = "/// Résumé lookup is constant-time; see core::adv::lookup.\r\n#[inline]\r\npub fn lookup() {}\r\n";
        let tree = with(
            G12,
            &[
                ("crates/core/src/adv.rs", adv),
                ("crates/core/src/adv_crlf.rs", crlf),
                (
                    "docs/04-invariants.md",
                    "| C-E-07 | x | `api::t::proof` |\n",
                ),
            ],
        );
        let (ok, log) = g12(&tree, "");
        assert!(!ok);
        for want in [
            "crates/core/src/adv.rs:1 (//!)",
            "crates/core/src/adv.rs:3 (split)",
            "crates/core/src/adv.rs:10 (multi)",
            "crates/core/src/adv.rs:17 (pathed)",
            "crates/core/src/adv.rs:20 (rowed)",
            "crates/core/src/adv.rs:23 (-)",
            "crates/core/src/adv_crlf.rs:1 (lookup)",
        ] {
            assert!(
                log.contains(&format!(
                    "  REFUSED  {want} — a cost claim that names no proof"
                )),
                "{want}: {log}"
            );
        }
        assert!(log.contains("    7 name nothing"), "{log}");
    }

    /// The `g12-proof` tree: a cited test loses its `#[test]`, and a cited
    /// test file is no longer tracked.
    #[test]
    fn gate12_refuses_when_a_cited_proof_is_withdrawn() {
        let claim = "/// O(1); see `api::lib::fast` and crates/api/tests/t.rs.\nfn hot3() {}\n";
        let tree = with(G12, &[("crates/core/src/lib.rs", claim)]);
        assert!(g12(&tree, "").0, "proven twice over");
        let demoted = "/// O(1); see `api::tests::fast`.\nfn hot() {}\nfn fast() {}\n";
        let mut files = G12.to_vec();
        files.retain(|(k, _)| *k != "crates/api/tests/t.rs");
        let tree = with(
            &files,
            &[
                ("crates/core/src/lib.rs", claim),
                ("crates/api/src/lib.rs", demoted),
            ],
        );
        let (ok, log) = g12(&tree, "");
        assert!(!ok);
        assert!(
            log.contains("  REFUSED  crates/core/src/lib.rs:1 (hot3)"),
            "{log}"
        );
        assert!(
            log.contains("  REFUSED  crates/api/src/lib.rs:1 (hot)"),
            "{log}"
        );
    }

    #[test]
    fn gate12_allowlist_is_keyed_by_item() {
        let tree = with(
            G12,
            &[(
                "crates/core/src/lib.rs",
                "/// O(1)\nfn excused() {}\n/// O(1)\nfn other() {}\n",
            )],
        );
        let (ok, log) = g12(&tree, "\n  crates/core/src/lib.rs excused\n");
        assert!(!ok);
        assert!(log.contains("  allowed  crates/core/src/lib.rs:1 (excused) — declared above"));
        assert!(log.contains("  REFUSED  crates/core/src/lib.rs:3 (other)"));
        let (ok, log) = g12(
            &tree,
            "crates/core/src/lib.rs excused\ncrates/core/src/lib.rs other\n",
        );
        assert!(ok, "{log}");
        // An item-less block is never excused.
        let tree = with(G12, &[("crates/core/src/lib.rs", "/// O(1)\n")]);
        let (ok, log) = g12(&tree, "crates/core/src/lib.rs -\n");
        assert!(!ok);
        assert!(
            log.contains("REFUSED  crates/core/src/lib.rs:1 (-)"),
            "{log}"
        );
        assert!(
            log.contains("MALFORMED ALLOWLIST ENTRY — \"crates/core/src/lib.rs - \""),
            "{log}"
        );
    }

    #[test]
    fn gate12_audits_its_own_allowlist() {
        let tree = fake(G12);
        let (ok, log) = g12(&tree, "crates/core/src/lib.rs x extra\n");
        assert!(!ok);
        assert!(
            log.contains(
                "MALFORMED ALLOWLIST ENTRY — \"crates/core/src/lib.rs x extra\" is not FILE ITEM"
            ),
            "{log}"
        );
        let (ok, log) = g12(&tree, "crates/core/src/lib.rs\n");
        assert!(!ok);
        assert!(
            log.contains("MALFORMED ALLOWLIST ENTRY — \"crates/core/src/lib.rs  \""),
            "{log}"
        );
        let (ok, log) = g12(&tree, "crates/gone.rs item\n");
        assert!(!ok);
        assert!(
            log.contains("  STALE ALLOWLIST ENTRY — crates/gone.rs is not a tracked file"),
            "{log}"
        );
        let (ok, log) = g12(&tree, "crates/core/src/lib.rs x\n");
        assert!(ok, "a loose entry only warns");
        assert!(log.contains("::warning title=Gate 12 allowlist is loose::crates/core/src/lib.rs x claims nothing unproven any more"), "{log}");
    }

    #[test]
    fn gate12_refuses_to_read_nothing() {
        let (ok, log) = g12(&fake(&[("docs/04-invariants.md", "")]), "");
        assert!(!ok);
        assert!(log.contains("GATE 12 WALKED NOTHING."));
        let tree = fake(&[
            ("crates/a/src/lib.rs", "/// prose\nfn a() {}\n"),
            ("docs/04-invariants.md", ""),
        ]);
        let (ok, log) = g12(&tree, "");
        assert!(!ok);
        assert!(
            log.contains("GATE 12 FOUND NO CLAIM AT ALL in 1 doc blocks."),
            "{log}"
        );
        let mut out = Vec::new();
        assert!(
            gate12(&fake(&[]), "", &mut out).is_err(),
            "no invariants file is an error"
        );
    }

    // ---- gate 14's pieces ----

    #[test]
    fn bench_declarations_read_like_the_old_reader() {
        let toml = "[package]\nname = \"x\"\n[[bench]]\nname = \"ratio\"\r\nharness = false # load-bearing\r\n";
        assert_eq!(bench_decls(toml), vec![("ratio".into(), "false".into())]);
        // A second table straight after the first discards the first.
        let toml = "[[bench]]\nname = \"ratio\"\nharness = false\n[[bench]]\nname = \"other\"\n";
        assert_eq!(bench_decls(toml), vec![("other".into(), String::new())]);
        let toml = "[[bench]]\nname = 'ratio'\nharness=false\n[dev-dependencies]\n";
        assert_eq!(
            bench_decls(toml),
            vec![("name = 'ratio'".into(), "false".into())]
        );
    }

    #[test]
    fn ratio_points_count_calls_not_declarations() {
        let code = "fn ratio(a: u64) {}\nfn growth_ratio<T>(x: T) {}\nlet r = ratio (1);\nlet s = growth_ratio(2); let t = ratio(3);\n           \nratio\n";
        assert_eq!(ratio_points(code), 2);
    }

    #[test]
    fn string_heads_are_cut_like_the_old_pipeline() {
        assert_eq!(
            string_head("crates/x/benches/r.rs:12:C-01 measured"),
            "C-01"
        );
        assert_eq!(string_head("crates/x/benches/r.rs:12:  C-01:x y"), "C-01:x");
        assert_eq!(string_head("a:b"), "");
        assert_eq!(string_head("no colon here"), "no");
    }

    #[test]
    fn the_lexed_view_splits_back_per_file() {
        let tree = fake(&[("a.rs", "x\ny\n"), ("b.rs", "z")]);
        let files = vec!["a.rs".to_owned(), "b.rs".to_owned()];
        let l = split_lexed(&tree, &files, "X\nY\nZ", "").unwrap();
        assert_eq!(l.code["a.rs"], "X\nY\n");
        assert_eq!(l.code["b.rs"], "Z");
        assert!(split_lexed(&tree, &files, "X\nY\nZ\nextra\n", "").is_err());
        assert!(split_lexed(&tree, &files, "X\n", "").is_err());
        let files = vec!["b.rs".to_owned(), "a.rs".to_owned()];
        assert!(split_lexed(&tree, &files, "ZX\nY\n", "").is_err());
    }

    #[test]
    fn step_runs_and_exact_lines() {
        assert!(step_runs_ok(
            "present  1 step(s) run `x` unconditionally and blocking\n"
        ));
        assert!(!step_runs_ok(""));
        assert!(!step_runs_ok(
            "REFUSED  no step runs a line beginning `x`\n"
        ));
        assert!(exact_line(
            "    cargo bench --workspace --locked  \r\n",
            BENCH_LINE
        ));
        assert!(!exact_line(
            "cargo bench --workspace --locked\n",
            BENCH_LINE
        ));
        assert!(!exact_line(
            "  cargo bench --workspace --locked || true\n",
            BENCH_LINE
        ));
    }

    // ---- gate 14 end to end ----

    const BENCH: &str = "use std::time::Instant;\nconst CEILING: u64 = 3;\n// ratio(9) in prose\nfn ratio(a: u64) { if a > CEILING { std::process::exit(1) } }\nfn main() { std::hint::black_box(1); ratio(1);\nratio(2); println!(\"C-01 measured\"); }\n";
    const BENCH_CODE: &str = "use std::time::Instant;\nconst CEILING: u64 = 3;\n                   \nfn ratio(a: u64) { if a > CEILING { std::process::exit(1) } }\nfn main() { std::hint::black_box(1); ratio(1);\nratio(2); println!(\"C-01 measured\"); }\n";
    const WF: &str = "      run: |\n          cargo bench --workspace --locked\n          git ls-files --error-unmatch 'crates/*/benches/*.rs' > /dev/null\n";
    const PRESENT: &str = "present  1 step(s) run `x` unconditionally and blocking\n";
    const G14: &[(&str, &str)] = &[
        (
            "crates/core/Cargo.toml",
            "[package]\nname = \"core\"\n[[bench]]\nname = \"ratio\"\nharness = false\n",
        ),
        ("crates/core/src/lib.rs", "/// O(1)\nfn hot() {}\n"),
        ("crates/core/benches/ratio.rs", BENCH),
        ("docs/04-invariants.md", "| C-01 | x | y |\n"),
        (".github/workflows/ci.yml", WF),
    ];
    const COVER: &str = "\n  core crates/core/benches/ratio.rs 2 C-01\n";

    fn g14_with(
        tree: &Fake,
        cover: &str,
        code: &str,
        strings: &str,
        runs: [&str; 2],
    ) -> (bool, String) {
        let listing: Vec<String> = tree.ls("crates/*/benches/*.rs").unwrap();
        let lexed = split_lexed(tree, &listing, code, strings).unwrap();
        let mut out = Vec::new();
        let ok = gate14(tree, cover, &lexed, runs, &mut out).unwrap();
        (ok, out.join("\n"))
    }

    fn g14(tree: &Fake, cover: &str) -> (bool, String) {
        let strings = "crates/core/benches/ratio.rs:5:C-01 measured\n";
        g14_with(tree, cover, BENCH_CODE, strings, [PRESENT, PRESENT])
    }

    #[test]
    fn gate14_passes_a_measured_tree() {
        let (ok, log) = g14(&fake(G14), COVER);
        assert!(ok, "{log}");
        assert!(log.contains("  covered  crates/core — 1 cost claim(s)"));
        assert!(log.contains("  ok       crates/core/benches/ratio.rs — 2 point(s) = pin 2, ceiling compared, prints, exits"));
        assert!(log.contains("  ok       crates/core/benches/ratio.rs — C-01 is a row in docs/04-invariants.md and is printed"));
    }

    #[test]
    fn gate14_refuses_an_unlisted_claiming_crate() {
        let tree = with(
            G14,
            &[
                ("crates/api/Cargo.toml", ""),
                (
                    "crates/api/src/a.rs",
                    "/// O(1)\nfn a() {}\n/// flat\nfn b() {}\n",
                ),
            ],
        );
        let (ok, log) = g14(&tree, COVER);
        assert!(!ok);
        assert!(
            log.contains("  REFUSED  crates/api — 2 cost claim(s), no row in the table"),
            "{log}"
        );
        assert!(
            log.contains("             doc block at crates/api/src/a.rs:3"),
            "{log}"
        );
        // A claim inside a test or bench path is not a promise.
        let tree = with(
            G14,
            &[("crates/api/src/tests/a.rs", "/// O(1)\nfn a() {}\n")],
        );
        assert!(g14(&tree, COVER).0);
    }

    #[test]
    fn gate14_refuses_a_row_that_does_not_hold() {
        for (row, want) in [
            (
                "gone crates/gone/benches/r.rs 2 C-01",
                "  STALE TABLE ROW — crates/gone has no tracked manifest",
            ),
            (
                "core crates/core/benches/none.rs 2 C-01",
                "  REFUSED  crates/core/benches/none.rs is not tracked",
            ),
            ("core", "  REFUSED   is not tracked"),
            (
                "core crates/core/benches/ratio.rs 3 C-01",
                "2-measurement-points-but-pinned-at-3",
            ),
            (
                "core crates/core/benches/ratio.rs 1 C-01",
                "2-measurement-points-but-pinned-at-1",
            ),
            (
                "core crates/core/benches/ratio.rs x C-01",
                "2-measurement-points-but-pinned-at-x",
            ),
            (
                "core crates/core/benches/ratio.rs 2 C-02",
                "C-02: in-invariants=0 in-bench=0",
            ),
            (
                "core crates/core/benches/ratio.rs 2 C-01 C-09",
                "C-09: in-invariants=0 in-bench=0",
            ),
        ] {
            let cover = format!("core crates/core/benches/ratio.rs 2 C-01\n{row}\n");
            let (ok, log) = g14(&fake(G14), &cover);
            assert!(!ok, "{row}");
            assert!(log.contains(want), "{row}: {log}");
            assert!(log.contains("GATE 14 FAILED."));
        }
    }

    #[test]
    fn gate14_refuses_a_bench_that_cannot_fail_the_build() {
        for manifest in [
            "[[bench]]\nname = \"ratio\"\n",
            "[[bench]]\nname = \"ratio\"\nharness = true\n",
            "[[bench]]\nname = \"other\"\nharness = false\n",
            "[[bench]]\nname = \"ratio\"\nharness = false\n[[bench]]\nname = \"b\"\nharness = false\n",
        ] {
            let tree = with(G14, &[("crates/core/Cargo.toml", manifest)]);
            let (ok, log) = g14(&tree, COVER);
            assert!(!ok, "{manifest}");
            assert!(
                log.contains("crates/core/Cargo.toml declares no [[bench]] named ratio"),
                "{log}"
            );
        }
    }

    #[test]
    fn gate14_refuses_a_bench_without_the_shape() {
        for (from, to, want) in [
            (
                "const CEILING: u64 = 3;",
                "const LIMIT: u64 = 3;",
                "a-named-ceiling-compared-against",
            ),
            (
                "use std::time::Instant;",
                "use std::time::Duration;",
                "nothing-is-timed",
            ),
            ("std::hint::black_box(1);", "", "no-black_box"),
            ("println!(\"C-01 measured\");", "", "prints-nothing"),
            ("std::process::exit(1)", "panic!()", "cannot-exit-non-zero"),
        ] {
            let bench = BENCH.replace(from, to);
            let code = BENCH_CODE.replace(from, to);
            let tree = with(
                G14,
                &[(
                    "crates/core/benches/ratio.rs",
                    Box::leak(bench.into_boxed_str()),
                )],
            );
            let strings = "crates/core/benches/ratio.rs:5:C-01 measured\n";
            let (ok, log) = g14_with(&tree, COVER, &code, strings, [PRESENT, PRESENT]);
            assert!(!ok, "{want}");
            assert!(
                log.contains(&format!("is not a ratio bench: {want}")) || log.contains(want),
                "{want}: {log}"
            );
        }
    }

    #[test]
    fn gate14_counts_points_and_ids_in_code_not_comments() {
        // A ratio call that exists only in a comment is blank in the lexed
        // view, and an id named only in a comment is not a printed string.
        let bench = BENCH.replace("println!(\"C-01 measured\")", "println!(\"done\") // C-01");
        let tree = with(
            G14,
            &[(
                "crates/core/benches/ratio.rs",
                Box::leak(bench.into_boxed_str()),
            )],
        );
        let code = BENCH_CODE.replace("println!(\"C-01 measured\")", "println!(\"done\")      ");
        let (ok, log) = g14_with(
            &tree,
            COVER,
            &code,
            "crates/core/benches/ratio.rs:5:done\n",
            [PRESENT, PRESENT],
        );
        assert!(!ok);
        assert!(log.contains("C-01: in-invariants=1 in-bench=0"), "{log}");
        // A string of ANOTHER bench does not count for this one.
        let (ok, _) = g14_with(
            &fake(G14),
            COVER,
            BENCH_CODE,
            "crates/core/benches/ratio.rs.bak:5:C-01\n",
            [PRESENT, PRESENT],
        );
        assert!(!ok);
    }

    #[test]
    fn gate14_refuses_when_gate_8_is_disarmed() {
        let strings = "crates/core/benches/ratio.rs:5:C-01 measured\n";
        let refused =
            "REFUSED  step at line 9 in job `x` carries `if: false`, which can switch it off\n";
        let (ok, log) = g14_with(&fake(G14), COVER, BENCH_CODE, strings, [refused, PRESENT]);
        assert!(!ok);
        assert!(log.contains("  REFUSED  step at line 9"), "{log}");
        assert!(
            log.contains("  REFUSED  nothing in this workflow RUNS the benches"),
            "{log}"
        );
        let (ok, log) = g14_with(&fake(G14), COVER, BENCH_CODE, strings, [PRESENT, ""]);
        assert!(!ok);
        assert!(
            log.contains("  REFUSED  gate 8 no longer fails when no bench exists."),
            "{log}"
        );
        let wf = WF.replace("--locked\n", "--locked || true\n");
        let tree = with(
            G14,
            &[(".github/workflows/ci.yml", Box::leak(wf.into_boxed_str()))],
        );
        let (ok, log) = g14_with(&tree, COVER, BENCH_CODE, strings, [PRESENT, PRESENT]);
        assert!(!ok);
        assert!(
            log.contains("  REFUSED  nothing in this workflow RUNS the benches"),
            "{log}"
        );
        let wf = WF.replace(" > /dev/null", "");
        let tree = with(
            G14,
            &[(".github/workflows/ci.yml", Box::leak(wf.into_boxed_str()))],
        );
        assert!(!g14_with(&tree, COVER, BENCH_CODE, strings, [PRESENT, PRESENT]).0);
    }

    #[test]
    fn gate14_refuses_an_empty_table_and_an_empty_walk() {
        let (ok, log) = g14(&fake(G14), "\n   \n");
        assert!(!ok);
        assert!(log.contains("  THE COVERAGE TABLE IS EMPTY."), "{log}");
        assert!(log.contains("  REFUSED  crates/core — 1 cost claim(s), no row in the table"));
        let tree = with(G14, &[("crates/core/src/lib.rs", "/// prose\nfn x() {}\n")]);
        let (ok, log) = g14(&tree, COVER);
        assert!(!ok);
        assert!(
            log.contains("GATE 14 FOUND NO COST CLAIM AT ALL in 1 doc blocks."),
            "{log}"
        );
        let tree = fake(&[
            ("docs/04-invariants.md", ""),
            (".github/workflows/ci.yml", WF),
        ]);
        let (ok, log) = g14_with(&tree, COVER, "", "", [PRESENT, PRESENT]);
        assert!(!ok);
        assert!(log.contains("GATE 14 WALKED NOTHING."), "{log}");
    }

    #[test]
    fn gate14_warns_on_a_loose_row_and_refuses_an_unlexed_bench() {
        let tree = with(
            G14,
            &[
                (
                    "crates/api/Cargo.toml",
                    "[[bench]]\nname = \"ratio\"\nharness = false\n",
                ),
                ("crates/api/src/ratio.rs", BENCH),
            ],
        );
        let cover = format!("{COVER}api crates/api/src/ratio.rs 2 C-01\n");
        let (ok, log) = g14(&tree, &cover);
        assert!(!ok);
        assert!(
            log.contains(
                "::warning title=Gate 14 table row is loose::crates/api claims no bound any more"
            ),
            "{log}"
        );
        assert!(
            log.contains("  REFUSED  crates/api/src/ratio.rs could not be lexed"),
            "{log}"
        );
    }

    #[test]
    fn text_that_is_not_source_is_an_error() {
        assert!(text_of("x", b"a\0b".to_vec()).is_err());
        assert!(text_of("x", vec![0xff, b'a']).is_err());
        assert_eq!(text_of("x", b"ok".to_vec()).unwrap(), "ok");
    }
}

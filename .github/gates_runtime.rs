//! The language-purity gates 13, 15, 16, 17, 21, 22, 23 and 24, as one
//! dependency-free program built by rustc (D-2312).
//!
//! Each of those steps used to be a shell program: awk, sed, grep, cut, sort,
//! tr, loops and conditionals, a second language in a tracked file outside
//! `web/`. The rules live here now, one subcommand per gate, and the step is
//! the build, the tests and one invocation. What each gate DECLARES -- its
//! banned names, allowlists and declared surfaces -- stays in the workflow
//! beside the paragraphs that justify every entry, and is handed in as an
//! argument; nothing in this file widens or narrows a list.
//!
//! THE SAME QUESTIONS, ASKED THE SAME WAY. Every pattern the shell steps
//! matched with a POSIX extended regular expression is a function below that
//! names the expression it replaces in its doc comment, and is pinned by a
//! test that feeds it the cases the expression accepted and refused. Where
//! the shell was LOOSER than its own comment -- a path word-split on a space,
//! a listing that git quoted, a binary file grep would not print, an error
//! swallowed by `|| true` -- this reads the stricter way and the difference
//! is recorded in D-2312. Nothing here passes a tree the shell refused.
//!
//! THE SCANNER IS THE ONE GATE 0 BUILT AND TESTED. Tokens, module closures,
//! canonical paths and parsed manifests come from `.github/source_scan.rs`,
//! never from a second lexer: one copy of a fragile mechanism. It is started
//! as a child process, and its path is bound when this file is COMPILED,
//! through `CARGO_BIN_EXE_source_scan` -- the mechanism cargo itself uses to
//! hand a test its own package's binaries, and one of the three shapes gate
//! 0's spawn rule (D-2344) accepts. The workflow exports it from gate 0's
//! `SOURCE_SCAN` immediately before the rustc line, so the program this
//! starts is a binary built by rustc from a tracked file in the same job. A
//! build without it (gate 6c's clippy pass) still compiles, and every scanner
//! call then refuses.
//!
//! Inputs are `git ls-files -z` (git is started by name, as gate 0 allows),
//! the files it lists, read directly, and for gate 22's clauses B to E the
//! directory walk `grep -r` made. Nothing from an event payload is read.

#![forbid(unsafe_code)]

use std::cell::OnceCell;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::process::{Command, ExitCode, Stdio};

// ------------------------------------------------------------- report --

/// What a gate prints, and whether it refused.
#[derive(Debug, Default)]
struct Report {
    lines: Vec<String>,
    bad: bool,
}

impl Report {
    fn say(&mut self, s: impl Into<String>) {
        self.lines.push(s.into());
    }

    fn refuse(&mut self, s: impl Into<String>) {
        self.bad = true;
        self.say(s);
    }

    /// Each of `items` on its own line behind `indent` spaces.
    fn indented<S: AsRef<str>>(&mut self, indent: usize, items: &[S]) {
        for i in items {
            self.say(format!("{}{}", " ".repeat(indent), i.as_ref()));
        }
    }

    fn text(&self) -> String {
        self.lines.join("\n")
    }
}

// --------------------------------------------------------------- repo --

/// A finished scanner run: its exit status and what it printed on stdout.
#[derive(Clone, Debug)]
struct Scan {
    code: i32,
    out: String,
}

impl Scan {
    fn ok(&self) -> bool {
        self.code == 0
    }
}

/// Everything a gate reads. The real one asks git, the scanner and the disk;
/// the tests hand in a tree held in memory.
trait Repo {
    /// `git ls-files -z -- SPECS`; no spec lists every tracked file. Git's
    /// pathspec `*` matches `/` too, exactly as the shell steps relied on.
    fn ls(&self, specs: &[&str]) -> Result<Vec<String>, String>;
    /// The bytes of a file, or `None` if it cannot be read.
    fn read(&self, path: &str) -> Option<Vec<u8>>;
    /// The source scanner, run with `args`.
    fn scan(&self, args: &[&str]) -> Result<Scan, String>;
    /// A NUL-separated file naming every tracked path, for `closure`.
    fn listing(&self) -> Result<String, String>;
    /// `grep -r`'s walk: every regular file under `dir`, symbolic links met
    /// on the way skipped; `None` when `dir` is not a directory (`[ -d ]`).
    fn walk(&self, dir: &str) -> Result<Option<Vec<String>>, String>;
}

/// The program gate 0 built: `${RUNNER_TEMP}/source-scan`, bound at compile
/// time. `None` in a build that was not given it.
fn scanner() -> Option<Command> {
    let path = option_env!("CARGO_BIN_EXE_source_scan")?;
    Some(Command::new(path))
}

fn git() -> Command {
    Command::new("git")
}

struct Real {
    dir: PathBuf,
    listing: OnceCell<Result<String, String>>,
}

impl Real {
    fn new() -> Self {
        let base = std::env::var_os("RUNNER_TEMP")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        Self {
            dir: base.join(format!("gates-runtime-{}", std::process::id())),
            listing: OnceCell::new(),
        }
    }
}

impl Drop for Real {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

impl Repo for Real {
    fn ls(&self, specs: &[&str]) -> Result<Vec<String>, String> {
        let out = git()
            .args(["ls-files", "-z", "--"])
            .args(specs)
            .stderr(Stdio::inherit())
            .output()
            .map_err(|e| format!("git ls-files: {e}"))?;
        if !out.status.success() {
            return Err(format!("git ls-files {specs:?} failed: {}", out.status));
        }
        let text = String::from_utf8(out.stdout)
            .map_err(|_| "git ls-files listed a path that is not UTF-8".to_owned())?;
        Ok(text
            .split('\0')
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .collect())
    }

    fn read(&self, path: &str) -> Option<Vec<u8>> {
        std::fs::read(path).ok()
    }

    fn scan(&self, args: &[&str]) -> Result<Scan, String> {
        let mut cmd = scanner()
            .ok_or("this tool was built without CARGO_BIN_EXE_source_scan, so it has no scanner")?;
        let out = cmd
            .args(args)
            .stderr(Stdio::inherit())
            .output()
            .map_err(|e| format!("source_scan: {e}"))?;
        Ok(Scan {
            code: out.status.code().unwrap_or(-1),
            out: String::from_utf8_lossy(&out.stdout).into_owned(),
        })
    }

    fn listing(&self) -> Result<String, String> {
        self.listing
            .get_or_init(|| {
                let tracked = self.ls(&[])?;
                std::fs::create_dir_all(&self.dir)
                    .map_err(|e| format!("{}: {e}", self.dir.display()))?;
                let path = self.dir.join("tracked.z");
                let mut body = tracked.join("\0");
                body.push('\0');
                std::fs::write(&path, body).map_err(|e| format!("{}: {e}", path.display()))?;
                path.to_str()
                    .map(str::to_owned)
                    .ok_or_else(|| "the temporary directory is not UTF-8".to_owned())
            })
            .clone()
    }

    fn walk(&self, dir: &str) -> Result<Option<Vec<String>>, String> {
        match std::fs::metadata(dir) {
            Ok(m) if m.is_dir() => {}
            _ => return Ok(None),
        }
        let mut out = Vec::new();
        let mut stack = vec![dir.to_owned()];
        while let Some(d) = stack.pop() {
            let entries = std::fs::read_dir(&d).map_err(|e| format!("{d}: {e}"))?;
            for e in entries {
                let e = e.map_err(|e| format!("{d}: {e}"))?;
                let p = format!("{d}/{}", e.file_name().to_string_lossy());
                let t = e.file_type().map_err(|e| format!("{p}: {e}"))?;
                if t.is_dir() {
                    stack.push(p);
                } else if t.is_file() {
                    out.push(p);
                }
            }
        }
        out.sort();
        Ok(Some(out))
    }
}

// ------------------------------------------------------------ helpers --

/// The records awk and grep read: split on `\n`, a final unterminated line
/// kept, no empty record after a final newline. A `\r` stays in its line.
fn lines_of(s: &str) -> Vec<&str> {
    let mut v: Vec<&str> = s.split('\n').collect();
    if v.last() == Some(&"") {
        v.pop();
    }
    v
}

/// `[[:space:]]`.
fn is_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

fn skip_space(s: &[u8], mut i: usize) -> usize {
    while i < s.len() && is_space(s[i]) {
        i += 1;
    }
    i
}

/// `[A-Za-z0-9_-]`, the characters of a crate name.
fn is_name(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'-'
}

/// Every position at which `needle` starts in `hay`, overlaps included.
fn find_all<'a>(hay: &'a [u8], needle: &'a [u8]) -> impl Iterator<Item = usize> + 'a {
    hay.windows(needle.len().max(1))
        .enumerate()
        .filter(move |(_, w)| !needle.is_empty() && *w == needle)
        .map(|(i, _)| i)
}

fn contains(hay: &[u8], needle: &[u8]) -> bool {
    find_all(hay, needle).next().is_some()
}

/// Is `s[i]` the byte `b`?
fn at(s: &[u8], i: usize, b: u8) -> bool {
    s.get(i) == Some(&b)
}

/// Every `Y` such that the line is `X:DIGITS:Y` with `X` non-empty: the
/// readings of `^.+:[0-9]+:Y$`, which a path printed by the scanner is.
fn tagged_tails(line: &str) -> Vec<&str> {
    let b = line.as_bytes();
    let mut out = Vec::new();
    for (i, &c) in b.iter().enumerate().skip(1) {
        if c != b':' {
            continue;
        }
        let digits = b[i + 1..].iter().take_while(|d| d.is_ascii_digit()).count();
        let k = i + 1 + digits;
        if digits > 0 && at(b, k, b':') {
            out.push(&line[k + 1..]);
        }
    }
    out
}

/// awk's `sub(/^[^:]*:[0-9]+:/, "", p)`: the text after the first colon's
/// `:DIGITS:` tag, or the whole line when the first colon is not one.
fn after_tag(line: &str) -> &str {
    let b = line.as_bytes();
    let Some(i) = line.find(':') else {
        return line;
    };
    let digits = b[i + 1..].iter().take_while(|d| d.is_ascii_digit()).count();
    let k = i + 1 + digits;
    if digits > 0 && at(b, k, b':') {
        &line[k + 1..]
    } else {
        line
    }
}

/// awk's `sub(/:[0-9]+:.*$/, "", f)`: the text before the leftmost
/// `:DIGITS:`, or the whole line.
fn before_tag(line: &str) -> &str {
    let b = line.as_bytes();
    for (i, &c) in b.iter().enumerate() {
        if c != b':' {
            continue;
        }
        let digits = b[i + 1..].iter().take_while(|d| d.is_ascii_digit()).count();
        if digits > 0 && at(b, i + 1 + digits, b':') {
            return &line[..i];
        }
    }
    line
}

/// One `path count` allowlist as awk read it: per non-blank line, the first
/// two blank-separated fields.
fn allow_entries(list: &str) -> Vec<(String, Option<String>)> {
    list.split('\n')
        .filter_map(|l| {
            let mut f = l.split([' ', '\t']).filter(|w| !w.is_empty());
            let path = f.next()?;
            Some((path.to_owned(), f.next().map(str::to_owned)))
        })
        .collect()
}

/// The count an allowlist grants `file`: its first entry's second field, 0
/// when the file is not listed or the field is absent. A count that is not
/// an integer is an error: the shell's `[ c -gt a ]` read it as "not
/// greater" and so ALLOWED everything.
fn allowed(list: &str, file: &str) -> Result<i64, String> {
    match allow_entries(list).into_iter().find(|(p, _)| p == file) {
        None | Some((_, None)) => Ok(0),
        Some((p, Some(n))) => n.parse::<i64>().map_err(|_| {
            format!("the allowlist entry for {p} has the count `{n}`, not an integer")
        }),
    }
}

/// Does `line` belong to `file` in a `path:...` record list?
fn of_file(line: &str, file: &str) -> bool {
    line == file || line.strip_prefix(file).is_some_and(|r| r.starts_with(':'))
}

/// Lines of `out` beginning with `prefix`, and the rest, non-empty only.
fn partition<'a>(out: &'a str, prefix: &str) -> (Vec<&'a str>, Vec<&'a str>) {
    lines_of(out)
        .into_iter()
        .filter(|l| !l.is_empty())
        .partition(|l| l.starts_with(prefix))
}

/// The scanner's `closure` of `roots` over every tracked file: the files,
/// and the `UNRESOLVED` lines when it did not resolve.
fn closure(
    repo: &dyn Repo,
    roots: &[&str],
    unresolved: &str,
) -> Result<(Vec<String>, Option<Vec<String>>), String> {
    let listing = repo.listing()?;
    let mut args = vec!["closure", listing.as_str()];
    args.extend_from_slice(roots);
    let s = repo.scan(&args)?;
    let (bad, files) = partition(&s.out, unresolved);
    let files = files.into_iter().map(str::to_owned).collect();
    let bad: Vec<String> = bad.into_iter().map(str::to_owned).collect();
    Ok((files, (!s.ok()).then_some(bad)))
}

/// Every `FILE:LINE:path` the scanner reads in `files`, and the files it
/// could not lex. Each file is its own run, so one unreadable file hides no
/// other file's paths (the shell's `xargs` stopped at the first).
fn paths_of(repo: &dyn Repo, sub: &str, files: &[String]) -> (Vec<String>, Vec<String>) {
    let mut lines = Vec::new();
    let mut failed = Vec::new();
    for f in files {
        match repo.scan(&[sub, f]) {
            Ok(s) if s.ok() => lines.extend(lines_of(&s.out).into_iter().map(str::to_owned)),
            _ => failed.push(f.clone()),
        }
    }
    (lines, failed)
}

/// A sorted multiset compared the way `diff` printed it: `<` declared only,
/// `>` measured only.
fn difference(want: &[String], got: &[String]) -> Vec<String> {
    fn count(v: &[String]) -> BTreeMap<&str, i64> {
        let mut m: BTreeMap<&str, i64> = BTreeMap::new();
        for x in v {
            *m.entry(x.as_str()).or_default() += 1;
        }
        m
    }
    let (w, g) = (count(want), count(got));
    let keys: BTreeSet<&str> = w.keys().chain(g.keys()).copied().collect();
    let mut out = Vec::new();
    for k in keys {
        let d = w.get(k).copied().unwrap_or(0) - g.get(k).copied().unwrap_or(0);
        for _ in 0..d.max(0) {
            out.push(format!("< {k}"));
        }
        for _ in 0..(-d).max(0) {
            out.push(format!("> {k}"));
        }
    }
    out
}

/// The whitespace-separated words of a declared list, sorted.
fn words_sorted(list: &str) -> Vec<String> {
    let mut v: Vec<String> = list.split_whitespace().map(str::to_owned).collect();
    v.sort();
    v
}

// ------------------------------------------------------------ options --

/// `--name value` pairs.
#[derive(Debug, Default)]
struct Opts(BTreeMap<String, String>);

impl Opts {
    fn parse(args: &[String]) -> Result<Self, String> {
        let mut m = BTreeMap::new();
        let mut it = args.iter();
        while let Some(k) = it.next() {
            let name = k
                .strip_prefix("--")
                .ok_or_else(|| format!("expected --NAME, got `{k}`"))?;
            let v = it.next().ok_or_else(|| format!("--{name} has no value"))?;
            if m.insert(name.to_owned(), v.clone()).is_some() {
                return Err(format!("--{name} given twice"));
            }
        }
        Ok(Self(m))
    }

    fn need(&self, k: &str) -> Result<&str, String> {
        self.0
            .get(k)
            .map(String::as_str)
            .ok_or_else(|| format!("--{k} is required"))
    }

    /// Refuses a name the gate does not read: a misspelt option is a list
    /// the gate silently ignores.
    fn only(&self, names: &[&str]) -> Result<(), String> {
        match self.0.keys().find(|k| !names.contains(&k.as_str())) {
            Some(k) => Err(format!("--{k} is not an option of this gate")),
            None => Ok(()),
        }
    }
}

// ------------------------------------------------------------ gate 13 --

/// Every end of the family `(n|n[-_][A-Za-z0-9_-]+)` starting at `i`.
fn fam_ends(s: &[u8], i: usize, n: &[u8]) -> Vec<usize> {
    let mut out = Vec::new();
    if i > s.len() || !s[i..].starts_with(n) {
        return out;
    }
    let e = i + n.len();
    out.push(e);
    if at(s, e, b'-') || at(s, e, b'_') {
        let mut k = e + 1;
        while k < s.len() && is_name(s[k]) {
            k += 1;
            out.push(k);
        }
    }
    out
}

/// `[[:space:]]*=` at `k`.
fn eq_at(s: &[u8], k: usize) -> bool {
    at(s, skip_space(s, k), b'=')
}

/// `[^A-Za-z0-9_-]FAM(\.[A-Za-z_-]+)?[[:space:]]*=`: a dependency key.
fn decl_key(s: &[u8], n: &[u8]) -> bool {
    s.iter().enumerate().any(|(i, &c)| {
        !is_name(c)
            && fam_ends(s, i + 1, n).into_iter().any(|e| {
                if eq_at(s, e) {
                    return true;
                }
                if !at(s, e, b'.') {
                    return false;
                }
                let mut k = e + 1;
                while k < s.len() && (s[k].is_ascii_alphabetic() || s[k] == b'_' || s[k] == b'-') {
                    k += 1;
                    if eq_at(s, k) {
                        return true;
                    }
                }
                false
            })
    })
}

/// `\[[^]]*dependencies[^]]*\.FAM\]`: a dependency table header.
fn decl_table(s: &[u8], n: &[u8]) -> bool {
    s.iter().enumerate().any(|(i, &c)| {
        if c != b'[' {
            return false;
        }
        let Some(r) = s[i + 1..].iter().position(|&b| b == b']') else {
            return false;
        };
        let seg = &s[i + 1..i + 1 + r];
        seg.iter().enumerate().any(|(q, &d)| {
            d == b'.'
                && fam_ends(seg, q + 1, n).contains(&seg.len())
                && contains(&seg[..q], b"dependencies")
        })
    })
}

/// `package[[:space:]]*=[[:space:]]*"FAM"`: a renamed dependency.
fn decl_package(s: &[u8], n: &[u8]) -> bool {
    find_all(s, b"package").any(|p| quoted_value(s, p + 7, n))
}

/// `[[:space:]]*=[[:space:]]*"FAM"` at `k`.
fn quoted_value(s: &[u8], k: usize, n: &[u8]) -> bool {
    let k = skip_space(s, k);
    if !at(s, k, b'=') {
        return false;
    }
    let k = skip_space(s, k + 1);
    at(s, k, b'"') && fam_ends(s, k + 1, n).into_iter().any(|e| at(s, e, b'"'))
}

/// `(:FAM:[^:]+|:FAM)$` on a `FILE:LINE:kind:name:package` declaration.
fn parsed_hit(s: &[u8], n: &[u8]) -> bool {
    s.iter().enumerate().any(|(i, &c)| {
        c == b':'
            && fam_ends(s, i + 1, n).into_iter().any(|e| {
                e == s.len() || (at(s, e, b':') && e + 1 < s.len() && !s[e + 1..].contains(&b':'))
            })
    })
}

/// `:name[[:space:]]*=[[:space:]]*"FAM"` on a `FILE:LINE:text` lock line.
fn lock_hit(s: &[u8], n: &[u8]) -> bool {
    find_all(s, b":name").any(|p| quoted_value(s, p + 5, n))
}

/// `:[[:space:]]*(build|links)[[:space:]]*=`: a manifest naming a build
/// script, or declaring the `links` that requires one.
fn build_key(s: &[u8]) -> bool {
    s.iter().enumerate().any(|(i, &c)| {
        if c != b':' {
            return false;
        }
        let k = skip_space(s, i + 1);
        [&b"build"[..], b"links"]
            .iter()
            .any(|w| s[k..].starts_with(w) && eq_at(s, k + w.len()))
    })
}

/// `^[[:space:]]*KEY[[:space:]]*=[[:space:]]*VALUE` (VALUE a prefix).
fn key_is(line: &[u8], key: &[u8], value: &[u8]) -> bool {
    let k = skip_space(line, 0);
    if !line[k..].starts_with(key) {
        return false;
    }
    let k = skip_space(line, k + key.len());
    at(line, k, b'=') && line[skip_space(line, k + 1)..].starts_with(value)
}

/// `name[[:space:]]*=[[:space:]]*"N"` anywhere in `text`.
fn names_exactly(text: &[u8], n: &[u8]) -> bool {
    find_all(text, b"name").any(|p| {
        let k = skip_space(text, p + 4);
        if !at(text, k, b'=') {
            return false;
        }
        let k = skip_space(text, k + 1);
        at(text, k, b'"') && text[k + 1..].starts_with(n) && at(text, k + 1 + n.len(), b'"')
    })
}

/// `^[[:space:]]+cargo install cargo-deny --version [0-9]`.
fn pins_cargo_deny(line: &[u8]) -> bool {
    let k = skip_space(line, 0);
    let rest = &line[k..];
    let lead = b"cargo install cargo-deny --version ";
    k > 0 && rest.starts_with(lead) && rest.get(lead.len()).is_some_and(u8::is_ascii_digit)
}

/// Gate 13's `path:line:text` records against a `path count` allowlist,
/// with the allowlist audited for stale and loose entries.
fn verdict(
    r: &mut Report,
    gate: &str,
    layer: &str,
    hits: &[String],
    allow: &str,
    tracked: &BTreeSet<String>,
) {
    let files: BTreeSet<&str> = hits
        .iter()
        .filter(|h| !h.is_empty())
        .map(|h| h.split(':').next().unwrap_or(h))
        .collect();
    for f in files {
        let mine: Vec<&String> = hits.iter().filter(|h| of_file(h, f)).collect();
        let c = i64::try_from(mine.len()).unwrap_or(i64::MAX);
        match allowed(allow, f) {
            Err(e) => r.refuse(format!("  REFUSED  {e}")),
            Ok(a) if c > a => {
                r.refuse(format!("  REFUSED  {f} — {c} occurrence(s), {a} allowed"));
                let shown: Vec<&String> = mine.into_iter().take(8).collect();
                r.indented(13, &shown);
            }
            Ok(a) => r.say(format!("  allowed  {f} — {c} of {a}")),
        }
    }
    for (p, _) in allow_entries(allow) {
        if !tracked.contains(&p) {
            r.refuse(format!(
                "  STALE ALLOWLIST ENTRY — {p} is not a tracked file"
            ));
        } else if !hits.iter().any(|h| of_file(h, &p)) {
            r.say(format!(
                "  ::warning title={gate} allowlist is loose::{p} no longer matches layer {layer} — drop the entry"
            ));
        }
    }
}

/// Is `n` a plain crate name? The shell spliced each into a regex, so a name
/// holding a metacharacter matched something other than itself.
fn plain_name(n: &str) -> bool {
    !n.is_empty() && n.bytes().all(is_name)
}

/// The workflow gate 13 layer 4 reads for gate 3's steps.
const WORKFLOW: &str = ".github/workflows/ci.yml";

/// The names on gate 13's own list that `deny.toml` must keep banning.
const DENY_MUST: [&str; 11] = [
    "pyo3",
    "mlua",
    "rlua",
    "deno_core",
    "v8",
    "rusty_v8",
    "quickjs-rs",
    "boa_engine",
    "node-bindgen",
    "neon",
    "jni",
];

/// Gate 13: no foreign runtime is declared, resolved or built for by name.
fn gate13(repo: &dyn Repo, o: &Opts) -> Result<Report, String> {
    o.only(&["banned", "allow-decl", "allow-lock", "allow-build"])?;
    let banned: Vec<&str> = o.need("banned")?.split_whitespace().collect();
    let (allow_decl, allow_lock, allow_build) = (
        o.need("allow-decl")?,
        o.need("allow-lock")?,
        o.need("allow-build")?,
    );
    let mut r = Report::default();
    if let Some(n) = banned.iter().find(|n| !plain_name(n)) {
        r.refuse(format!(
            "GATE 13'S LIST NAMES `{n}`, which is not a plain crate name."
        ));
        return Ok(r);
    }
    let all = repo.ls(&[])?;
    let tracked: BTreeSet<String> = all.iter().cloned().collect();

    // Layer 1's corpus: every tracked Cargo.toml, comment-stripped, and the
    // same manifests parsed by the scanner.
    let mut decl: Vec<String> = Vec::new();
    let mut parsed: Vec<String> = Vec::new();
    let mut nt = 0usize;
    for f in repo.ls(&["*Cargo.toml"])? {
        if f.rsplit('/').next() != Some("Cargo.toml") {
            continue;
        }
        nt += 1;
        match repo.read(&f) {
            Some(b) => {
                let text = String::from_utf8_lossy(&b);
                for (n, l) in lines_of(&text).into_iter().enumerate() {
                    let l = l.split('#').next().unwrap_or(l);
                    decl.push(format!("{f}:{}:{l}", n + 1));
                }
            }
            None => r.refuse(format!("  REFUSED  {f} could not be read")),
        }
        match repo.scan(&["deps", &f]) {
            Ok(s) if s.ok() => parsed.extend(lines_of(&s.out).into_iter().map(str::to_owned)),
            _ => r.refuse(format!("  REFUSED  {f} could not be read as TOML")),
        }
    }
    // Layer 2's corpus: every tracked lock, line by line.
    let mut lock: Vec<String> = Vec::new();
    let mut nl = 0usize;
    for f in repo.ls(&["*Cargo.lock"])? {
        nl += 1;
        match repo.read(&f) {
            Some(b) => {
                let text = String::from_utf8_lossy(&b);
                for (n, l) in lines_of(&text).into_iter().enumerate() {
                    lock.push(format!("{f}:{}:{l}", n + 1));
                }
            }
            None => r.refuse(format!("  REFUSED  {f} could not be read")),
        }
    }
    r.say(format!(
        "walked {nt} tracked Cargo.toml and {nl} tracked Cargo.lock"
    ));
    r.say(format!(
        "  {} manifest line(s) after comment stripping",
        decl.len()
    ));
    r.say(format!("  {} lock line(s)", lock.len()));
    r.say(format!(
        "  {} banned crate name(s), each matched with its -* and _* family",
        banned.len()
    ));
    if nt == 0 {
        r.refuse("GATE 13 WALKED NO MANIFEST. A repository with no Cargo.toml is");
        r.say("not a Rust workspace; a silent zero is not a pass.");
        return Ok(r);
    }
    if nl == 0 {
        r.refuse("GATE 13 WALKED NO LOCK FILE. Cargo.lock is the ONLY place a");
        r.say("transitive binding is visible, and gate 4 builds --locked.");
        r.say("An untracked lock disarms the layer that matters most.");
        return Ok(r);
    }

    let mut dh: Vec<String> = Vec::new();
    let mut lh: Vec<String> = Vec::new();
    for n in &banned {
        let n = n.as_bytes();
        for l in &decl {
            let b = l.as_bytes();
            if decl_key(b, n) || decl_table(b, n) || decl_package(b, n) {
                dh.push(l.clone());
            }
        }
        dh.extend(
            parsed
                .iter()
                .filter(|l| parsed_hit(l.as_bytes(), n))
                .cloned(),
        );
        lh.extend(lock.iter().filter(|l| lock_hit(l.as_bytes(), n)).cloned());
    }

    r.say("");
    r.say("layer 1: a manifest declares a binding or an embedded runtime");
    if dh.is_empty() {
        r.say("  none");
    }
    verdict(&mut r, "Gate 13", "1", &dh, allow_decl, &tracked);
    r.say("");
    r.say("layer 2: the lock resolves one, directly or transitively");
    if lh.is_empty() {
        r.say("  none");
    }
    verdict(&mut r, "Gate 13", "2", &lh, allow_lock, &tracked);

    // Layer 3: a build script by its default name, every file its closure
    // compiles into it, and a manifest `build =` or `links =` key.
    let roots: Vec<&str> = all
        .iter()
        .filter(|p| p.rsplit('/').next() == Some("build.rs"))
        .map(String::as_str)
        .collect();
    let mut bh: Vec<String> = roots
        .iter()
        .map(|p| format!("{p}:1:a build script exists"))
        .collect();
    if !roots.is_empty() {
        let (files, unresolved) = closure(repo, &roots, "UNRESOLVED ")?;
        if let Some(u) = unresolved {
            r.refuse("  REFUSED  a build script pulls in code layer 3 cannot resolve:");
            r.indented(13, &u);
        }
        for f in files {
            if !roots.contains(&f.as_str()) {
                bh.push(format!("{f}:1:a module compiled into a build script"));
            }
        }
    }
    bh.extend(decl.iter().filter(|l| build_key(l.as_bytes())).cloned());
    r.say("");
    r.say("layer 3: a build script exists anywhere in the tree, by file or by manifest key");
    if bh.is_empty() {
        r.say("  none");
    }
    verdict(&mut r, "Gate 13", "3", &bh, allow_build, &tracked);

    // Layer 4: deny.toml and the step that runs it.
    r.say("");
    r.say("layer 4: deny.toml and the step that runs it");
    let d = "deny.toml";
    if tracked.contains(d) {
        let text = repo.read(d).unwrap_or_default();
        if lines_of(&String::from_utf8_lossy(&text))
            .iter()
            .any(|l| key_is(l.as_bytes(), b"wildcards", b"\"deny\""))
        {
            r.say("  present  deny.toml sets wildcards = \"deny\"");
        } else {
            r.refuse("  REFUSED  deny.toml no longer sets wildcards = \"deny\"");
            r.say("           Every path dependency in this workspace carries a");
            r.say("           version BECAUSE of that line — crates/api and");
            r.say("           crates/pull both say so in their manifests. Removing");
            r.say("           it turns those comments into false claims.");
        }
        let names = |n: &str| {
            lines_of(&String::from_utf8_lossy(&text))
                .iter()
                .any(|l| names_exactly(l.as_bytes(), n.as_bytes()))
        };
        for n in DENY_MUST {
            if names(n) {
                r.say(format!("  present  deny.toml bans {n}"));
            } else {
                r.refuse(format!("  REFUSED  deny.toml no longer bans {n}"));
            }
        }
        for n in banned.iter().filter(|n| !DENY_MUST.contains(n)) {
            if !names(n) {
                r.say(format!(
                    "  ::warning title=Gate 13 is ahead of deny.toml::this gate refuses {n} and deny.toml does not list it"
                ));
            }
        }
    } else {
        r.refuse("  REFUSED  deny.toml is not tracked — gate 3 has no policy to read");
    }
    let runs = |needle: &str, r: &mut Report| -> bool {
        match repo.scan(&["step-runs", WORKFLOW, needle]) {
            Ok(s) => {
                let shown: Vec<&str> = lines_of(&s.out);
                r.indented(2, &shown);
                s.ok()
            }
            Err(e) => {
                r.say(format!("  {e}"));
                false
            }
        }
    };
    if runs("cargo deny check", &mut r) {
        r.say("  present  gate 3 invokes cargo deny check, unconditionally");
    } else {
        r.refuse("  REFUSED  nothing in this workflow RUNS cargo deny check so that");
        r.say("           its failure fails the run. Reasons above.");
    }
    let pinned = runs("cargo install cargo-deny --version ", &mut r)
        && repo.read(WORKFLOW).is_some_and(|w| {
            lines_of(&String::from_utf8_lossy(&w))
                .iter()
                .any(|l| pins_cargo_deny(l.as_bytes()))
        });
    if pinned {
        r.say("  present  cargo-deny is installed at a pinned version, unconditionally");
    } else {
        r.refuse("  REFUSED  cargo-deny is installed unpinned — a security verdict");
        r.say("           that can change with no commit is a verdict nobody");
        r.say("           can read. See D-0034.");
    }

    r.say("");
    if r.bad {
        r.say("GATE 13 FAILED.");
        r.say("");
        r.say("CLAUDE.md section 2 has no exception for this. A binding does");
        r.say("not become acceptable by being transitive, by being a");
        r.say("dev-dependency, or by being convenient. Remove it, or write");
        r.say("the allowlist line above WITH the reason and a");
        r.say("docs/05-decisions.md entry beside it.");
    } else {
        r.say("OK — no foreign runtime is declared or resolved by name (gates 13b and 13c read what is built).");
    }
    Ok(r)
}

// ------------------------------------------------------------ gate 15 --

/// The banned word, assembled from two halves that mean nothing apart.
/// This file is a tracked file and gate 15 reads it too, so the word must
/// never be written whole here. If you are here to tidy this into one
/// literal, stop: read the paragraph beginning IF YOU ARE HERE TO TIDY THIS
/// in gate 15's step.
const WORD: &str = concat!("py", "thon");

/// The tripwire for an edited half: a blanked one would leave a token that
/// matches every line, or an empty one that matches nothing.
const WORD_LEN: usize = 6;

/// Non-overlapping case-insensitive occurrences of `needle`: `grep -aoiF`.
fn count_ci(hay: &[u8], needle: &[u8]) -> usize {
    let mut n = 0;
    let mut i = 0;
    while !needle.is_empty() && i + needle.len() <= hay.len() {
        if hay[i..i + needle.len()].eq_ignore_ascii_case(needle) {
            n += 1;
            i += needle.len();
        } else {
            i += 1;
        }
    }
    n
}

/// `line` with every case-insensitive `needle` replaced: the log masks the
/// word wherever it echoes an offending line.
fn mask(line: &[u8], needle: &[u8]) -> String {
    let mut out = Vec::new();
    let mut i = 0;
    while i < line.len() {
        if i + needle.len() <= line.len() && line[i..i + needle.len()].eq_ignore_ascii_case(needle)
        {
            out.extend_from_slice(b"[BANNED WORD]");
            i += needle.len();
        } else {
            out.push(line[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// What `grep -I` would skip: empty, a NUL byte, not UTF-8, or no character
/// but newlines. Counted and printed only; every file is read regardless.
fn binary_or_empty(b: &[u8]) -> bool {
    b.iter().all(|&c| c == b'\n') || b.contains(&0) || std::str::from_utf8(b).is_err()
}

/// Gate 15: the forbidden word appears in no tracked file.
fn gate15(repo: &dyn Repo, o: &Opts) -> Result<Report, String> {
    o.only(&["allow"])?;
    let allow = o.need("allow")?;
    let mut r = Report::default();
    if WORD.len() != WORD_LEN {
        r.refuse(format!(
            "GATE 15 CANNOT ASSEMBLE ITS OWN TOKEN — got {} characters.",
            WORD.len()
        ));
        r.say("Somebody edited one of the two halves of WORD in .github/gates_runtime.rs.");
        return Ok(r);
    }
    let tok = WORD.as_bytes();
    let all = repo.ls(&[])?;
    let tracked: BTreeSet<String> = all.iter().cloned().collect();
    let (mut walked, mut skipped) = (0usize, 0usize);
    let mut refused: Vec<(usize, String, Vec<u8>)> = Vec::new();
    for f in &all {
        walked += 1;
        let b = repo.read(f).unwrap_or_default();
        if binary_or_empty(&b) {
            skipped += 1;
        }
        let n = count_ci(&b, tok);
        if n > 0 {
            refused.push((n, f.clone(), b));
        }
    }
    r.say(format!("walked {walked} tracked file(s)"));
    r.say(format!(
        "  {skipped} binary or empty, each read byte for byte all the same"
    ));
    r.say(format!(
        "  {} file(s) contain the banned token",
        refused.len()
    ));
    if walked == 0 {
        r.refuse("GATE 15 WALKED NOTHING. A gate that reads no file is not a gate.");
        return Ok(r);
    }
    r.say("");
    for (c, f, b) in &refused {
        let c = i64::try_from(*c).unwrap_or(i64::MAX);
        match allowed(allow, f) {
            Err(e) => r.refuse(format!("  REFUSED  {e}")),
            Ok(a) if c > a => {
                r.refuse(format!("  REFUSED  {f} — {c} occurrence(s), {a} allowed"));
                let shown: Vec<String> = b
                    .split(|&x| x == b'\n')
                    .enumerate()
                    .filter(|(_, l)| count_ci(l, tok) > 0)
                    .take(8)
                    .map(|(n, l)| format!("{}:{}", n + 1, mask(l, tok)))
                    .collect();
                r.indented(13, &shown);
            }
            Ok(a) => r.say(format!("  allowed  {f} — {c} of {a}")),
        }
    }
    for (p, _) in allow_entries(allow) {
        if !tracked.contains(&p) {
            r.refuse(format!(
                "  STALE ALLOWLIST ENTRY — {p} is not a tracked file"
            ));
        } else if !refused.iter().any(|(_, f, _)| *f == p) {
            r.say(format!(
                "  ::warning title=Gate 15 allowlist is loose::{p} no longer contains the token — drop the entry"
            ));
        }
    }
    r.say("");
    if r.bad {
        r.say("GATE 15 FAILED.");
        r.say("");
        r.say("The word is banned outright, including in prose about a");
        r.say("predecessor repository. Reword it — 'the predecessor', 'the");
        r.say("prior implementation' — or, if the occurrence genuinely has");
        r.say("to stay, add its file to the allowlist above with the reason");
        r.say("beside it. Do not spell the word into this workflow to make");
        r.say("the match narrower: this file is scanned too.");
    } else {
        r.say("OK — the banned token appears in no tracked file.");
    }
    Ok(r)
}

// ------------------------------------------------------------ gate 16 --

/// Is `c` a word character for `\b`: a letter, a digit or `_`?
fn word_char(c: Option<char>) -> bool {
    c.is_some_and(|c| c.is_alphanumeric() || c == '_')
}

/// `^#!\[forbid\([^]]*\bunsafe_code\b` on some line of `src`.
fn forbids_unsafe(src: &str) -> bool {
    src.split('\n').any(|l| {
        let Some(rest) = l.strip_prefix("#![forbid(") else {
            return false;
        };
        let body = rest.split(']').next().unwrap_or(rest);
        let w = "unsafe_code";
        body.match_indices(w).any(|(p, _)| {
            let before = if p == 0 {
                Some('(')
            } else {
                body[..p].chars().next_back()
            };
            !word_char(before) && !word_char(body[p + w.len()..].chars().next())
        })
    })
}

/// awk's walk of a manifest: a line beginning `[lints]` opens the table,
/// any other line beginning `[` closes it, and inside it a line matching
/// `^[[:space:]]*workspace[[:space:]]*=[[:space:]]*true` is the opt-in.
fn inherits_lints(src: &str) -> bool {
    let mut inside = false;
    let mut ok = false;
    for l in lines_of(src) {
        if l.starts_with("[lints]") {
            inside = true;
            continue;
        }
        if l.starts_with('[') {
            inside = false;
        }
        if inside && key_is(l.as_bytes(), b"workspace", b"true") {
            ok = true;
        }
    }
    ok
}

/// Layer 1b's roots: the awk filter on `/`-separated fields of a listing of
/// `build.rs`, tests, benches, examples and `src/bin` files.
fn other_root(p: &str) -> bool {
    let f: Vec<&str> = p.split('/').collect();
    let field = |i: usize| f.get(i - 1).copied();
    match f.len() {
        3 => true,
        4 => field(3) != Some("src"),
        5 => field(4) == Some("bin") || field(5) == Some("main.rs"),
        _ => false,
    }
}

/// Gate 16: every crate root forbids unsafe outright.
fn gate16(repo: &dyn Repo, o: &Opts) -> Result<Report, String> {
    o.only(&["allow-root", "allow-unsafe-test-root"])?;
    let allow_root: Vec<String> = allow_entries(o.need("allow-root")?)
        .into_iter()
        .map(|(p, _)| p)
        .collect();
    let excepted_root = o.need("allow-unsafe-test-root")?;
    let mut r = Report::default();
    let tracked: BTreeSet<String> = repo.ls(&[])?.into_iter().collect();
    let read = |p: &str| {
        repo.read(p)
            .map(|b| String::from_utf8_lossy(&b).into_owned())
    };
    let manifests = repo.ls(&["crates/*/Cargo.toml"])?;

    r.say("layer 1: every crate root carries #![forbid(unsafe_code)]");
    let (mut nm, mut nr) = (0usize, 0usize);
    for m in &manifests {
        nm += 1;
        let c = m.split('/').nth(1).unwrap_or("");
        let mut found = 0;
        for base in ["lib", "main"] {
            let f = format!("crates/{c}/src/{base}.rs");
            if !tracked.contains(&f) {
                continue;
            }
            found += 1;
            nr += 1;
            if read(&f).is_some_and(|s| forbids_unsafe(&s)) {
                r.say(format!("  ok       {f}"));
            } else if allow_root.contains(&f) {
                r.say(format!("  allowed  {f} — no forbid, listed above"));
            } else {
                r.refuse(format!("  REFUSED  {f} carries no #![forbid(unsafe_code)]"));
            }
        }
        if found == 0 {
            r.refuse(format!(
                "  REFUSED  {m} — neither src/lib.rs nor src/main.rs is tracked."
            ));
            r.say("           This gate will not guess where the crate root is.");
        }
    }
    r.say(format!(
        "  {nm} crate manifest(s), {nr} crate root(s) examined"
    ));
    if nm == 0 || nr == 0 {
        r.refuse("GATE 16 EXAMINED NO CRATE ROOT. A gate that reads no lib.rs and");
        r.say("no main.rs is not a gate.");
        return Ok(r);
    }

    r.say("");
    r.say("layer 1b: no other crate root, nor any file it compiles, uses unsafe");
    let other: Vec<String> = repo
        .ls(&[
            "crates/*/build.rs",
            "crates/*/tests/*.rs",
            "crates/*/benches/*.rs",
            "crates/*/examples/*.rs",
            "crates/*/src/bin/*.rs",
        ])?
        .into_iter()
        .filter(|p| other_root(p))
        .collect();
    if other.is_empty() {
        r.refuse("  REFUSED  no test, bench or build root found — a gate that reads");
        r.say("           none is not a gate");
    } else {
        let roots: Vec<&str> = other.iter().map(String::as_str).collect();
        let (files, unresolved) = closure(repo, &roots, "UNRESOLVED ")?;
        if let Some(u) = unresolved {
            r.refuse("  REFUSED  a root pulls in code this layer cannot resolve:");
            r.indented(13, &u);
        }
        r.say(format!(
            "  {} root(s), {} file(s) in their closure",
            other.len(),
            files.len()
        ));
        let mut found: Vec<String> = Vec::new();
        if !files.is_empty() {
            let mut args = vec!["unsafe"];
            args.extend(files.iter().map(String::as_str));
            match repo.scan(&args) {
                // 1 is "found something"; anything else is a file the
                // scanner could not read, which the shell's `|| true` hid.
                Ok(s) if s.code == 0 || s.code == 1 => {
                    found.extend(
                        lines_of(&s.out)
                            .into_iter()
                            .filter(|l| !l.is_empty())
                            .map(str::to_owned),
                    );
                }
                _ => r.refuse("  REFUSED  a file in the closure could not be lexed"),
            }
        }
        let tag = format!("{excepted_root}:");
        let (excepted, refused): (Vec<&String>, Vec<&String>) =
            found.iter().partition(|l| l.contains(&tag));
        let beyond: Vec<&&String> = excepted
            .iter()
            .filter(|l| !l.ends_with(": `unsafe`"))
            .collect();
        if !beyond.is_empty() {
            r.refuse("  REFUSED  the excepted allocator file holds more than unsafe:");
            r.indented(13, &beyond);
        }
        if refused.is_empty() {
            r.say(format!(
                "  ok       none outside {excepted_root} ({} site(s) there, D-1450)",
                excepted.len()
            ));
        } else {
            r.refuse("  REFUSED  unsafe outside a forbidding crate root:");
            r.indented(13, &refused);
        }
    }
    for p in &allow_root {
        if !tracked.contains(p) {
            r.refuse(format!(
                "  STALE ALLOWLIST ENTRY — {p} is not a tracked file"
            ));
        } else if read(p).is_some_and(|s| forbids_unsafe(&s)) {
            r.say(format!(
                "  ::warning title=Gate 16 allowlist is loose::{p} forbids unsafe after all — drop the entry"
            ));
        }
    }

    r.say("");
    r.say("layer 1c: every .github/*.rs gate tool forbids unsafe and holds none (D-2349)");
    let tools = repo.ls(&[".github/*.rs"])?;
    if tools.is_empty() {
        r.refuse("  REFUSED  no .github/*.rs tool read");
    }
    for f in &tools {
        let clean = read(f).is_some_and(|s| forbids_unsafe(&s))
            && repo.scan(&["unsafe", f]).is_ok_and(|s| s.ok());
        if clean {
            r.say(format!("  ok       {f}"));
        } else {
            r.refuse(format!(
                "  REFUSED  {f} lacks #![forbid(unsafe_code)] or uses unsafe"
            ));
        }
    }

    r.say("");
    r.say("layer 2: the workspace lint table, and every member inheriting it");
    let root_denies = read("Cargo.toml").is_some_and(|s| {
        lines_of(&s)
            .iter()
            .any(|l| key_is(l.as_bytes(), b"unsafe_code", b"\"deny\""))
    });
    if root_denies {
        r.say("  present  root Cargo.toml sets unsafe_code = \"deny\"");
    } else {
        r.refuse("  REFUSED  root Cargo.toml no longer denies unsafe_code. Gate 5");
        r.say("           states that line already fails the build, and gate 5");
        r.say("           cannot check its own premise.");
    }
    for m in &manifests {
        if read(m).is_some_and(|s| inherits_lints(&s)) {
            r.say(format!("  present  {m} inherits [lints] workspace = true"));
        } else {
            r.refuse(format!(
                "  REFUSED  {m} does not write [lints] workspace = true, so the"
            ));
            r.say("           workspace lint table applies to none of it");
        }
    }

    r.say("");
    if r.bad {
        r.say("GATE 16 FAILED.");
        r.say("");
        r.say("Add #![forbid(unsafe_code)] to the crate root — every crate");
        r.say("root, a binary being its own — and keep [lints] workspace =");
        r.say("true in the manifest. A deny that one attribute switches off");
        r.say("is not the guarantee CLAUDE.md section 9 is read as making.");
    } else {
        r.say("OK — every crate root forbids unsafe and inherits the lint table.");
    }
    Ok(r)
}

// ------------------------------------------------------------ gate 17 --

/// `y` itself, and `y` without a leading `std::`, `core::` or `alloc::`:
/// the readings of `((std|core|alloc)::)?`.
fn rooted(y: &str) -> Vec<&str> {
    let mut v = vec![y];
    v.extend(
        ["std::", "core::", "alloc::"]
            .iter()
            .filter_map(|r| y.strip_prefix(r)),
    );
    v
}

const PRINTS: [&str; 5] = ["print!", "println!", "eprint!", "eprintln!", "dbg!"];
const HANDLES: [&str; 2] = ["io::stdout", "io::stderr"];

/// Gate 17's rule on a canonical path:
/// `((telemetry|log|tracing)(::.+|!)|((std|core|alloc)::)?(e?print(ln)?|dbg)!|((std|core|alloc)::)?io::(stdout|stderr))`.
fn logs(y: &str) -> bool {
    let logger = ["telemetry", "log", "tracing"].iter().any(|w| {
        y.strip_prefix(w)
            .is_some_and(|t| t == "!" || (t.starts_with("::") && t.len() > 2))
    });
    logger
        || rooted(y)
            .iter()
            .any(|c| PRINTS.contains(c) || HANDLES.contains(c))
}

/// The swept crates' tracked `src/` files plus the module closure of their
/// `src/lib.rs` (gates 17 and 22), with the closure's refusal, if any.
fn swept_files(
    repo: &dyn Repo,
    c: &str,
    specs: &[String],
) -> Result<(Vec<String>, Option<Vec<String>>), String> {
    let specs: Vec<&str> = specs.iter().map(String::as_str).collect();
    let mut files: BTreeSet<String> = repo.ls(&specs)?.into_iter().collect();
    let lib = format!("crates/{c}/src/lib.rs");
    let (more, unresolved) = closure(repo, &[&lib], "UNRESOLVED")?;
    files.extend(more);
    Ok((files.into_iter().collect(), unresolved))
}

/// Gate 17: nothing logs from inside the sweep.
fn gate17(repo: &dyn Repo, o: &Opts) -> Result<Report, String> {
    o.only(&["swept"])?;
    let swept: Vec<&str> = o.need("swept")?.split_whitespace().collect();
    let mut r = Report::default();
    let tracked: BTreeSet<String> = repo.ls(&[])?.into_iter().collect();
    if tracked.is_empty() {
        r.refuse("GATE 17 READ NO TRACKED FILE.");
        return Ok(r);
    }
    let mut scanned = 0usize;
    for c in &swept {
        if !tracked.contains(&format!("crates/{c}/src/lib.rs")) {
            r.refuse(format!(
                "  REFUSED  crates/{c}/src/lib.rs is not tracked. A swept crate"
            ));
            r.say("           cannot be covered by its absence.");
            continue;
        }
        let (files, unresolved) = swept_files(repo, c, &[format!("crates/{c}/src/*.rs")])?;
        if let Some(u) = unresolved {
            r.refuse(format!(
                "  REFUSED  crates/{c}: the module closure did not resolve:"
            ));
            r.indented(11, &u);
        }
        scanned += files.len();
        let (lines, failed) = paths_of(repo, "paths", &files);
        if !failed.is_empty() {
            r.refuse(format!(
                "  REFUSED  crates/{c}: a source file could not be lexed ({})",
                failed.join(", ")
            ));
        }
        let hits: Vec<&String> = lines
            .iter()
            .filter(|l| tagged_tails(l).into_iter().any(logs))
            .collect();
        if !hits.is_empty() {
            r.refuse(format!("LOGGING INSIDE THE SWEEP: crates/{c}"));
            r.indented(0, &hits);
        }
    }
    if r.bad {
        r.say("");
        r.say("GATE 17 FAILED.");
        r.say("");
        r.say("The sweep evaluates a mask per (bar, combination). A call that");
        r.say("is O(1) is still a call, and there are billions of them.");
        r.say("Count in a plain integer and emit ONE aggregate at the k-level");
        r.say("boundary — the shape pull::session::DropCensus already uses.");
    } else {
        r.say(format!(
            "OK — {scanned} swept source file(s) scanned, none logs per combination."
        ));
        r.say("     (vocab, engine, indicators AND runner — the per-bar and");
        r.say("      per-trial loops are in the last of those.)");
    }
    Ok(r)
}

// ------------------------------------------------------------ gate 23 --

/// `^[^:]*:[0-9]+:dependencies:[^:]*:telemetry$`: a plain dependency that
/// links the package `telemetry`.
fn links_telemetry(line: &str) -> bool {
    let Some((_, rest)) = line.split_once(':') else {
        return false;
    };
    let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
    if digits == 0 {
        return false;
    }
    rest[digits..]
        .strip_prefix(":dependencies:")
        .and_then(|t| t.strip_suffix(":telemetry"))
        .is_some_and(|name| !name.contains(':'))
}

/// `:(eprint!|eprintln!|handle):[0-9]+$`: a declared stderr writer.
fn writes_stderr(line: &str) -> bool {
    line.match_indices(':').any(|(i, _)| {
        ["eprint!", "eprintln!", "handle"].iter().any(|m| {
            line[i + 1..]
                .strip_prefix(m)
                .and_then(|t| t.strip_prefix(':'))
                .is_some_and(|d| !d.is_empty() && d.bytes().all(|b| b.is_ascii_digit()))
        })
    })
}

/// Gate 23 clause C's window: every `env::temp_dir()` line of `code` with no
/// `process::id()` on it or on the `window` lines below, 1-based.
fn fixed_temp_lines(code: &str, window: usize) -> Vec<usize> {
    let lines = lines_of(code);
    let mut out = Vec::new();
    for (i, l) in lines.iter().enumerate() {
        if !l.contains("env::temp_dir()") {
            continue;
        }
        let end = (i + window).min(lines.len().saturating_sub(1));
        if !lines[i..=end].iter().any(|l| l.contains("process::id()")) {
            out.push(i + 1);
        }
    }
    out
}

/// Gate 23: stderr is not a log, and a new crate is covered the day it
/// appears.
fn gate23(repo: &dyn Repo, o: &Opts) -> Result<Report, String> {
    o.only(&["declared", "declared-handles", "allow-fixed", "window"])?;
    let declared = o.need("declared")?;
    let declared_handles = o.need("declared-handles")?;
    let allow_fixed = o.need("allow-fixed")?;
    let window: usize = o
        .need("window")?
        .trim()
        .parse()
        .map_err(|_| "--window is not a line count".to_owned())?;
    let mut r = Report::default();
    let tracked: BTreeSet<String> = repo.ls(&[])?.into_iter().collect();

    let files = repo.ls(&["crates/*/src/*.rs"])?;
    if files.is_empty() {
        r.refuse("GATE 23 READ NO FILE under crates/*/src.");
        return Ok(r);
    }
    let (lines, failed) = paths_of(repo, "paths", &files);
    if !failed.is_empty() {
        r.refuse(format!(
            "  REFUSED  a source file under crates/*/src could not be lexed ({})",
            failed.join(", ")
        ));
    }
    let mut prints: BTreeMap<String, usize> = BTreeMap::new();
    let mut handles: BTreeMap<String, usize> = BTreeMap::new();
    for l in &lines {
        let p = after_tag(l);
        let f = before_tag(l);
        let f = f.strip_prefix("crates/").unwrap_or(f);
        let core = rooted(p);
        if core.iter().any(|c| PRINTS.contains(c)) {
            let m = p.rsplit("::").next().unwrap_or(p);
            *prints.entry(format!("{f}:{m}")).or_default() += 1;
        }
        if core.iter().any(|c| HANDLES.contains(c)) {
            *handles.entry(format!("{f}:handle")).or_default() += 1;
        }
    }
    let prints: Vec<String> = prints.iter().map(|(k, n)| format!("{k}:{n}")).collect();
    let handles: Vec<String> = handles.iter().map(|(k, n)| format!("{k}:{n}")).collect();

    // Clause A: the print surface is exactly the declared one.
    let want = words_sorted(declared);
    r.say("  declared:");
    r.indented(4, &want);
    r.say("  measured:");
    r.indented(4, &prints);
    if want != prints {
        r.say("");
        r.refuse("  REFUSED  the print surface is not the declared one.");
        let d = difference(&want, &prints);
        r.indented(0, &d);
        r.say("");
        r.say("           A NEW print: if it is a DIAGNOSTIC on a production");
        r.say("           path, emit an event too — stderr is seen by whoever");
        r.say("           is watching, the FILE is what gets handed to a");
        r.say("           diagnosis afterwards. Then declare it above saying");
        r.say("           which kind it is. A print that is GONE: delete its");
        r.say("           line, because a declaration nobody can trace to code");
        r.say("           is the stale lie gate 1d refuses.");
    }

    // Clause A2: the same question of the stream handles.
    let want_h = words_sorted(declared_handles);
    let none = |v: &[String]| {
        if v.is_empty() {
            "<none>".to_owned()
        } else {
            v.join("\n")
        }
    };
    r.say(format!("  declared stream handles: {}", none(&want_h)));
    r.say(format!("  measured stream handles: {}", none(&handles)));
    if want_h != handles {
        r.say("");
        r.refuse("  REFUSED  the set of writes to a process stream is not the");
        r.say("           declared one.");
        let d = difference(&want_h, &handles);
        r.indented(0, &d);
        r.say("");
        r.say("           writeln!(io::stderr(), ...) is a diagnostic wearing");
        r.say("           a different macro. If it is on a production path,");
        r.say("           emit the event beside it and declare it above; if it");
        r.say("           is gone, delete its line.");
    }

    // Clause B: a crate that writes to stderr must be able to log.
    let crates: BTreeSet<&str> = prints
        .iter()
        .chain(&handles)
        .filter(|l| writes_stderr(l))
        .map(|l| l.split('/').next().unwrap_or(l))
        .collect();
    for c in crates {
        if c == "telemetry" {
            continue;
        }
        let m = format!("crates/{c}/Cargo.toml");
        let can_log = tracked.contains(&m)
            && repo
                .scan(&["deps", &m])
                .is_ok_and(|s| s.ok() && lines_of(&s.out).into_iter().any(links_telemetry));
        if can_log {
            r.say(format!("  ok       {c} writes to stderr and can also emit"));
        } else {
            r.refuse(format!(
                "  REFUSED  crate '{c}' writes to stderr but declares no"
            ));
            r.say("           telemetry dependency in its [dependencies] table,");
            r.say("           so stderr is the ONLY channel it has in a shipping");
            r.say("           build. Add the dependency and emit the event.");
            r.say("           A dev-dependency does not answer this: it links");
            r.say("           into tests and into no release build.");
        }
    }

    // Clause C: a temporary path names the process that made it.
    let files = repo.ls(&["crates/**/*.rs"])?;
    if files.is_empty() {
        r.refuse("GATE 23 CLAUSE C READ NO FILE.");
        return Ok(r);
    }
    let mut fixed: Vec<(String, usize)> = Vec::new();
    for f in &files {
        match repo.scan(&["code", f]) {
            Ok(s) if s.ok() => {
                let name = f.strip_prefix("crates/").unwrap_or(f);
                fixed.extend(
                    fixed_temp_lines(&s.out, window)
                        .into_iter()
                        .map(|i| (name.to_owned(), i)),
                );
            }
            _ => r.refuse(format!("  REFUSED  {f} could not be lexed")),
        }
    }
    r.say(format!("  clause C: {} temporary path(s)", fixed.len()));
    r.say("            that do not name their process, over");
    r.say(format!(
        "            {} tracked source file(s)",
        files.len()
    ));
    let names: BTreeSet<&str> = fixed.iter().map(|(n, _)| n.as_str()).collect();
    for p in names {
        let mine: Vec<String> = fixed
            .iter()
            .filter(|(n, _)| n == p)
            .map(|(n, i)| format!("crates/{n}:{i}"))
            .collect();
        let c = i64::try_from(mine.len()).unwrap_or(i64::MAX);
        match allowed(allow_fixed, p) {
            Err(e) => r.refuse(format!("  REFUSED  {e}")),
            Ok(a) if c > a => {
                r.refuse(format!(
                    "  REFUSED  {p} — {c} fixed temporary path(s), {a} allowed"
                ));
                let shown: Vec<&String> = mine.iter().take(6).collect();
                r.indented(13, &shown);
            }
            Ok(a) => r.say(format!("  allowed  {p} — {c} of {a}")),
        }
    }
    for (p, _) in allow_entries(allow_fixed) {
        if !tracked.contains(&format!("crates/{p}")) {
            r.refuse(format!("  STALE ENTRY — crates/{p} is not a tracked file"));
        } else if !fixed.iter().any(|(n, _)| *n == p) {
            r.say(format!(
                "::warning title=Gate 23 clause C is loose::crates/{p} no longer names a fixed temporary path — drop the entry"
            ));
        }
    }

    if r.bad {
        r.say("");
        r.say("GATE 23 FAILED.");
        r.say("A diagnostic that exists only on a terminal nobody kept is a");
        r.say("fact this repository did not record. CLAUDE.md section 4.");
    } else {
        r.say("OK - every print is declared, and stderr is never a crate's only channel.");
    }
    Ok(r)
}

// ------------------------------------------------------------ gate 21 --

/// `:std::(fs|process|net|os)(::[A-Za-z0-9_]+)*::\*$`: a glob import of a
/// capability module.
fn capability_glob(line: &str) -> bool {
    line.match_indices(':').any(|(i, _)| {
        let Some(r) = line[i + 1..].strip_prefix("std::") else {
            return false;
        };
        ["fs", "process", "net", "os"].iter().any(|m| {
            r.strip_prefix(m)
                .and_then(|t| t.strip_suffix("::*"))
                .is_some_and(|mid| {
                    mid.is_empty()
                        || mid.strip_prefix("::").is_some_and(|m| {
                            m.split("::").all(|seg| {
                                !seg.is_empty()
                                    && seg.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
                            })
                        })
                })
        })
    })
}

/// `^std::(fs|process|net|os)::` and `::[a-z_][a-z0-9_]*!?$`: a capability
/// function, by its canonical path.
fn capability_fn(p: &str) -> bool {
    let module = ["fs", "process", "net", "os"].iter().any(|m| {
        p.strip_prefix("std::")
            .and_then(|t| t.strip_prefix(m))
            .is_some_and(|t| t.starts_with("::"))
    });
    module
        && p.match_indices("::").any(|(k, _)| {
            let t = &p[k + 2..];
            let t = t.strip_suffix('!').unwrap_or(t);
            let b = t.as_bytes();
            !b.is_empty()
                && (b[0].is_ascii_lowercase() || b[0] == b'_')
                && b[1..]
                    .iter()
                    .all(|&c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_')
        })
}

/// Gate 21: the logger opens nothing but its own files.
fn gate21(repo: &dyn Repo, o: &Opts) -> Result<Report, String> {
    o.only(&["declared"])?;
    let declared = o.need("declared")?;
    let mut r = Report::default();
    let tracked: BTreeSet<String> = repo.ls(&[])?.into_iter().collect();

    // Clause A: crates/telemetry declares no dependencies.
    let m = "crates/telemetry/Cargo.toml";
    let deps = match repo.scan(&["deps", m]) {
        Ok(s) if s.ok() && tracked.contains(m) => s.out.trim_end_matches('\n').to_owned(),
        _ => {
            r.say(format!(
                "  REFUSED  {m} is absent or could not be read as TOML"
            ));
            "(unreadable)".to_owned()
        }
    };
    if deps.is_empty() {
        r.say("  ok       crates/telemetry declares no dependencies - std only");
    } else {
        r.refuse("  REFUSED  crates/telemetry declares dependencies:");
        r.indented(13, &lines_of(&deps));
        r.say("           A logger that can reach the store can read a bar.");
    }

    // Clause B: the production capability surface is exactly the declared one.
    let files = repo.ls(&["crates/telemetry/src/*.rs"])?;
    if files.is_empty() {
        r.refuse("  REFUSED  crates/telemetry/src holds no tracked file");
        return Ok(r);
    }
    let (lines, failed) = paths_of(repo, "paths-prod", &files);
    if !failed.is_empty() {
        r.refuse(format!(
            "  REFUSED  a crates/telemetry source file could not be lexed ({})",
            failed.join(", ")
        ));
    }
    let globs: Vec<&String> = lines.iter().filter(|l| capability_glob(l)).collect();
    if !globs.is_empty() {
        r.refuse("  REFUSED  a glob import hides which capability is used:");
        r.indented(0, &globs);
    }
    let mut sites: BTreeMap<String, usize> = BTreeMap::new();
    for l in &lines {
        let f = before_tag(l);
        let f = f.strip_prefix("crates/telemetry/src/").unwrap_or(f);
        let p = after_tag(l);
        if capability_fn(p) {
            *sites.entry(format!("{f}:{p}")).or_default() += 1;
        }
    }
    let got: Vec<String> = sites.iter().map(|(k, n)| format!("{k}:{n}")).collect();
    let want = words_sorted(declared);
    r.say("  declared:");
    r.indented(4, &want);
    r.say("  measured:");
    r.indented(4, &got);
    if want != got {
        r.say("");
        r.refuse("  REFUSED  the logger's file-opening surface is not the declared one.");
        let d = difference(&want, &got);
        r.indented(0, &d);
        r.say("");
        r.say("           A NEW site: say which path it opens and why, and add");
        r.say("           it above in the same commit. A site that is GONE:");
        r.say("           delete its line, because a declaration nobody can");
        r.say("           trace to code is the stale lie gate 1d refuses.");
    }
    if r.bad {
        r.say("");
        r.say("GATE 21 FAILED.");
        r.say("CLAUDE.md section 4: no fallback that hides a failure. A logger");
        r.say("that can reach real bars can be silently wrong about them.");
    } else {
        r.say("OK - the logger depends on nothing and opens only its own files.");
    }
    Ok(r)
}

// ------------------------------------------------------------ gate 22 --

/// `$3 ":" $NF` of each non-empty declaration, sorted and unique, each
/// followed by a space: clause A's dependency set.
fn dependency_set(raw: &str) -> String {
    let set: BTreeSet<String> = lines_of(raw)
        .into_iter()
        .filter(|l| !l.is_empty())
        .map(|l| {
            let f: Vec<&str> = l.split(':').collect();
            format!(
                "{}:{}",
                f.get(2).copied().unwrap_or(""),
                f.last().copied().unwrap_or("")
            )
        })
        .collect();
    set.into_iter().map(|d| format!("{d} ")).collect()
}

/// Clause C's import:
/// `^[[:space:]]*(pub[[:space:]]+)?use[[:space:]]+(brutex_)?(store|pull|lake|api|telemetry)([^A-Za-z0-9_]|$)`.
fn imports_store(line: &[u8]) -> bool {
    let mut k = skip_space(line, 0);
    if line[k..].starts_with(b"pub") {
        let j = skip_space(line, k + 3);
        if j > k + 3 {
            k = j;
        }
    }
    if !line[k..].starts_with(b"use") {
        return false;
    }
    let j = skip_space(line, k + 3);
    if j == k + 3 {
        return false;
    }
    let mut k = j;
    if line[k..].starts_with(b"brutex_") {
        k += 7;
    }
    [&b"store"[..], b"pull", b"lake", b"api", b"telemetry"]
        .iter()
        .any(|w| {
            line[k..].starts_with(w)
                && line
                    .get(k + w.len())
                    .is_none_or(|&b| !(b.is_ascii_alphanumeric() || b == b'_'))
        })
}

/// `(::.*)?$` after a name.
fn rest_of_path(t: &str) -> bool {
    t.is_empty() || t.starts_with("::")
}

/// Clause B2's path: `((std|core|alloc)::(fs|os|net)(::.*)?|(std|core|alloc)::process(::.*)?|libc(::.*)?|memmap[0-9]*(::.*)?|include_bytes!)`.
fn reaches_fs(y: &str) -> bool {
    let rooted = ["std::", "core::", "alloc::"]
        .iter()
        .filter_map(|r| y.strip_prefix(r))
        .any(|t| {
            ["fs", "os", "net", "process"]
                .iter()
                .any(|m| t.strip_prefix(m).is_some_and(rest_of_path))
        });
    let memmap = y
        .strip_prefix("memmap")
        .is_some_and(|t| rest_of_path(t.trim_start_matches(|c: char| c.is_ascii_digit())));
    rooted || y.strip_prefix("libc").is_some_and(rest_of_path) || memmap || y == "include_bytes!"
}

/// Clause B2's exemption on the whole line:
/// `:(std|core|alloc)::process(::(exit|abort|id|ExitCode|ExitStatus|Termination))?$`.
fn starts_nothing(line: &str) -> bool {
    line.match_indices(':').any(|(i, _)| {
        ["std::", "core::", "alloc::"]
            .iter()
            .filter_map(|r| line[i + 1..].strip_prefix(r))
            .filter_map(|t| t.strip_prefix("process"))
            .any(|t| {
                t.is_empty()
                    || [
                        "exit",
                        "abort",
                        "id",
                        "ExitCode",
                        "ExitStatus",
                        "Termination",
                    ]
                    .iter()
                    .any(|m| t.strip_prefix("::") == Some(m))
            })
    })
}

/// Clause C2's path: `(brutex_)?(store|pull|lake|api|telemetry)::.+`.
fn names_store(y: &str) -> bool {
    let y = y.strip_prefix("brutex_").unwrap_or(y);
    ["store", "pull", "lake", "api", "telemetry"]
        .iter()
        .any(|w| {
            y.strip_prefix(w)
                .and_then(|t| t.strip_prefix("::"))
                .is_some_and(|t| !t.is_empty())
        })
}

/// `grep -rn`: every line of every file under `dir` that `hit` accepts, as
/// `path:line:text`; `None` when `dir` is not a directory.
fn grep_tree(
    repo: &dyn Repo,
    dir: &str,
    hit: &dyn Fn(&[u8]) -> bool,
) -> Result<Option<Vec<String>>, String> {
    Ok(tree_lines(repo, dir)?.map(|lines| {
        lines
            .iter()
            .filter(|l| hit(&l.text))
            .map(TreeLine::shown)
            .collect()
    }))
}

/// One line `grep -r` read. Every byte is read, a NUL or invalid UTF-8
/// included: grep 3.5 and later reports a match in such a file on stderr,
/// which the shell steps discarded, so a binary file matched in silence.
struct TreeLine {
    path: String,
    number: usize,
    text: Vec<u8>,
}

impl TreeLine {
    fn shown(&self) -> String {
        format!(
            "{}:{}:{}",
            self.path,
            self.number,
            String::from_utf8_lossy(&self.text)
        )
    }
}

/// Every line of every file under `dir`; `None` when it is not a directory.
fn tree_lines(repo: &dyn Repo, dir: &str) -> Result<Option<Vec<TreeLine>>, String> {
    let Some(files) = repo.walk(dir)? else {
        return Ok(None);
    };
    let mut out = Vec::new();
    for f in files {
        let b = repo
            .read(&f)
            .ok_or_else(|| format!("{f}: could not be read"))?;
        for (n, l) in b.split(|&c| c == b'\n').enumerate() {
            out.push(TreeLine {
                path: f.clone(),
                number: n + 1,
                text: l.to_vec(),
            });
        }
    }
    Ok(Some(out))
}

/// `include_str!\([^")]`: an include whose argument is not a literal.
fn computed_include(line: &[u8]) -> bool {
    find_all(line, b"include_str!(")
        .any(|p| line.get(p + 13).is_some_and(|&b| b != b'"' && b != b')'))
}

/// `grep -o 'include_str!\("[^"]+"\)'` with the sed that strips the macro:
/// the literal paths a line includes.
fn included_paths(line: &[u8]) -> Vec<String> {
    let open = b"include_str!(\"";
    let mut out = Vec::new();
    let mut i = 0;
    while i + open.len() <= line.len() {
        if line[i..].starts_with(open) {
            let s = i + open.len();
            if let Some(q) = line[s..].iter().position(|&b| b == b'"').map(|q| s + q)
                && q > s
                && at(line, q + 1, b')')
            {
                out.push(String::from_utf8_lossy(&line[s..q]).into_owned());
                i = q + 2;
                continue;
            }
        }
        i += 1;
    }
    out
}

/// Gate 22: the sweep crates cannot read a bar.
fn gate22(repo: &dyn Repo, o: &Opts) -> Result<Report, String> {
    o.only(&["pinned", "sweep", "banned"])?;
    let pinned: Vec<&str> = o.need("pinned")?.split_whitespace().collect();
    let sweep: Vec<&str> = o.need("sweep")?.split_whitespace().collect();
    let banned: Vec<&str> = o.need("banned")?.split('|').collect();
    let mut r = Report::default();
    if let Some(b) = banned.iter().find(|b| {
        b.is_empty()
            || b.contains([
                '.', '[', ']', '(', ')', '*', '+', '?', '{', '}', '^', '$', '\\',
            ])
    }) {
        r.refuse(format!(
            "GATE 22'S BANNED LIST HOLDS `{b}`, which is not a literal spelling."
        ));
        return Ok(r);
    }
    let tracked: BTreeSet<String> = repo.ls(&[])?.into_iter().collect();

    // Clause A: the pinned crates' dependency sets.
    for c in &pinned {
        let m = format!("crates/{c}/Cargo.toml");
        if !tracked.contains(&m) {
            r.refuse(format!(
                "REFUSED  {m} is not a tracked file. A pinned crate cannot be pinned by its absence."
            ));
            continue;
        }
        let raw = match repo.scan(&["deps", &m]) {
            Ok(s) if s.ok() => s.out,
            _ => {
                r.refuse(format!(
                    "REFUSED  {m} could not be read as TOML, so its dependency set is unknown."
                ));
                continue;
            }
        };
        let deps = dependency_set(&raw);
        let want = match *c {
            "vocab" => "",
            "indicators" | "engine" => "dependencies:vocab ",
            _ => {
                r.refuse(format!(
                    "REFUSED  crates/{c} is pinned, but this gate writes no dependency set for it."
                ));
                continue;
            }
        };
        if deps != want {
            r.refuse(format!(
                "REFUSED  crates/{c} declares [{deps}] and must declare [{want}]"
            ));
            r.indented(11, &lines_of(&raw));
            r.say("         A new dependency here is how a bar reaches the sweep.");
        }
    }

    // Clause B: no filesystem call site in src/ or benches/, by text.
    let fs_text = |l: &[u8]| banned.iter().any(|b| contains(l, b.as_bytes()));
    for c in &sweep {
        for dir in ["src", "benches"] {
            let d = format!("crates/{c}/{dir}");
            if let Some(hits) = grep_tree(repo, &d, &fs_text)?
                && !hits.is_empty()
            {
                r.refuse(format!("REFUSED  {d} reaches the filesystem:"));
                r.indented(11, &hits);
            }
        }
    }

    // Clause C: no import of a crate that reads the store, by text.
    for c in &sweep {
        let mut hits = Vec::new();
        for dir in ["src", "benches"] {
            hits.extend(
                grep_tree(repo, &format!("crates/{c}/{dir}"), &imports_store)?.unwrap_or_default(),
            );
        }
        if !hits.is_empty() {
            r.refuse(format!(
                "REFUSED  crates/{c} imports a crate that reads the store:"
            ));
            r.indented(11, &hits);
        }
    }

    // Clauses B2 and C2: the same two questions, asked of canonical paths.
    if tracked.is_empty() {
        r.refuse("GATE 22 READ NO TRACKED FILE.");
        return Ok(r);
    }
    for c in &sweep {
        if !tracked.contains(&format!("crates/{c}/src/lib.rs")) {
            r.refuse(format!(
                "REFUSED  crates/{c}/src/lib.rs is not tracked. A swept crate cannot"
            ));
            r.say("         be held to these clauses by its absence.");
            continue;
        }
        let (files, unresolved) = swept_files(
            repo,
            c,
            &[
                format!("crates/{c}/src/*.rs"),
                format!("crates/{c}/benches/*.rs"),
            ],
        )?;
        if let Some(u) = unresolved {
            r.refuse(format!(
                "REFUSED  crates/{c}: the module closure did not resolve:"
            ));
            r.indented(11, &u);
        }
        let (lines, failed) = paths_of(repo, "paths", &files);
        if !failed.is_empty() {
            r.refuse(format!(
                "REFUSED  crates/{c}: a source file could not be lexed ({})",
                failed.join(", ")
            ));
        }
        let fs: Vec<&String> = lines
            .iter()
            .filter(|l| tagged_tails(l).into_iter().any(reaches_fs) && !starts_nothing(l))
            .collect();
        if !fs.is_empty() {
            r.refuse(format!(
                "REFUSED  crates/{c} reaches the filesystem or a process, read as a path:"
            ));
            r.indented(11, &fs);
        }
        let store: Vec<&String> = lines
            .iter()
            .filter(|l| tagged_tails(l).into_iter().any(names_store))
            .collect();
        if !store.is_empty() {
            r.refuse(format!(
                "REFUSED  crates/{c} names a crate that reads the store, read as a path:"
            ));
            r.indented(11, &store);
        }
    }

    // Clause D: every compile-time include is a document, by a literal.
    for c in &sweep {
        for dir in ["src", "benches", "tests"] {
            let d = format!("crates/{c}/{dir}");
            let Some(lines) = tree_lines(repo, &d)? else {
                continue;
            };
            let computed: Vec<String> = lines
                .iter()
                .filter(|l| computed_include(&l.text))
                .map(TreeLine::shown)
                .collect();
            if !computed.is_empty() {
                r.refuse(format!(
                    "REFUSED  {d} computes an include path instead of writing one:"
                ));
                r.indented(11, &computed);
            }
            let paths: Vec<String> = lines.iter().flat_map(|l| included_paths(&l.text)).collect();
            // The shell word-split each path; a piece holding a glob
            // character was expanded against the working directory, so it
            // is refused here rather than guessed at.
            for p in paths
                .iter()
                .flat_map(|p| p.split([' ', '\t', '\n']).filter(|w| !w.is_empty()))
            {
                if p.contains(['*', '?', '[']) {
                    r.refuse(format!(
                        "REFUSED  {d} includes '{p}', which a shell would expand as a pattern"
                    ));
                    continue;
                }
                if ![".md", ".rs", ".toml", ".lock"]
                    .iter()
                    .any(|e| p.ends_with(e))
                {
                    r.refuse(format!(
                        "REFUSED  {d} includes '{p}', not a document or a source file"
                    ));
                }
                if p.contains("bars") || [".bin", ".parquet", ".csv"].iter().any(|e| p.ends_with(e))
                {
                    r.refuse(format!("REFUSED  {d} includes '{p}', which is bar-shaped"));
                }
            }
        }
    }

    // Clause E: tests/ may read the disk, bounded.
    let outside = |l: &[u8]| {
        [
            &b"env!(\"HOME\")"[..],
            b"var(\"HOME\")",
            b".brutex",
            b"store/bars",
            b".bin\"",
            b".parquet\"",
        ]
        .iter()
        .any(|n| contains(l, n))
    };
    for c in &sweep {
        let d = format!("crates/{c}/tests");
        if let Some(hits) = grep_tree(repo, &d, &outside)?
            && !hits.is_empty()
        {
            r.refuse(format!("REFUSED  {d} names a path outside the repository:"));
            r.indented(11, &hits);
        }
    }

    if r.bad {
        r.say("");
        r.say("GATE 22 FAILED.");
        r.say("");
        r.say("A sweep crate that can reach the filesystem can be handed real");
        r.say("bars by accident, and the owner's constraint is that even bars");
        r.say("pulled by mistake are never used. Keep the capability absent.");
    } else {
        r.say("OK — vocab, indicators, engine and runner cannot reach a bar,");
        r.say("     and the first three declare exactly the dependencies pinned above.");
    }
    Ok(r)
}

// ------------------------------------------------------------ gate 24 --

/// `MmapMut|MmapRaw|(^|[^A-Za-z0-9_])map_mut\(|(^|[^A-Za-z0-9_])map_anon(_mut)?\(|(^|[^A-Za-z0-9_])map_copy\(`.
fn maps_writably(line: &[u8]) -> bool {
    if contains(line, b"MmapMut") || contains(line, b"MmapRaw") {
        return true;
    }
    [
        &b"map_mut("[..],
        b"map_anon(",
        b"map_anon_mut(",
        b"map_copy(",
    ]
    .iter()
    .any(|n| {
        find_all(line, n)
            .any(|p| p == 0 || !(line[p - 1].is_ascii_alphanumeric() || line[p - 1] == b'_'))
    })
}

/// Gate 24: no writable memory mapping.
fn gate24(repo: &dyn Repo, o: &Opts) -> Result<Report, String> {
    o.only(&[])?;
    let mut r = Report::default();
    let mut hits: Vec<String> = Vec::new();
    let mut scanned = 0usize;
    for f in repo.ls(&["crates/**/*.rs"])? {
        scanned += 1;
        match repo.scan(&["code", &f]) {
            Ok(s) if s.ok() => {
                for (n, l) in lines_of(&s.out).into_iter().enumerate() {
                    if maps_writably(l.as_bytes()) {
                        hits.push(format!("{f}:{}:{l}", n + 1));
                    }
                }
            }
            _ => {
                r.say(format!("  REFUSED  {f} could not be lexed"));
                hits.push(format!("{f}: (unlexable)"));
            }
        }
    }
    r.say(format!(
        "scanned {scanned} tracked source file(s) under crates/"
    ));
    if scanned == 0 {
        r.refuse("GATE 24 SCANNED NOTHING. A gate that reads no file is not a gate.");
        return Ok(r);
    }
    if hits.is_empty() {
        r.say("OK — no crate maps a file or an anonymous region writably.");
    } else {
        r.refuse("WRITABLE MEMORY MAPPING:");
        r.indented(2, &hits);
        r.say("");
        r.say("GATE 24 FAILED.");
        r.say("");
        r.say("CLAUDE.md section 4 bans a writable memory mapping without");
        r.say("exception: it raises SIGBUS on a full disk and a signal cannot");
        r.say("be caught, least of all under panic = abort. Write through the");
        r.say("file API, check the error, and refuse loudly.");
    }
    Ok(r)
}

// --------------------------------------------------------------- main --

type Gate = fn(&dyn Repo, &Opts) -> Result<Report, String>;

const GATES: [(&str, Gate); 8] = [
    ("gate13", gate13),
    ("gate15", gate15),
    ("gate16", gate16),
    ("gate17", gate17),
    ("gate21", gate21),
    ("gate22", gate22),
    ("gate23", gate23),
    ("gate24", gate24),
];

fn run(repo: &dyn Repo, args: &[String]) -> Result<Report, String> {
    let (name, rest) = args.split_first().ok_or("no gate named")?;
    let gate = GATES
        .iter()
        .find(|(n, _)| n == name)
        .map(|(_, g)| *g)
        .ok_or_else(|| format!("unknown gate {name}"))?;
    gate(repo, &Opts::parse(rest)?)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let repo = Real::new();
    match run(&repo, &args) {
        Ok(r) => {
            println!("{}", r.text());
            if r.bad {
                ExitCode::from(1)
            } else {
                ExitCode::SUCCESS
            }
        }
        Err(e) => {
            let names: Vec<&str> = GATES.iter().map(|(n, _)| *n).collect();
            eprintln!("gates_runtime: {e}");
            eprintln!(
                "usage: gates_runtime <{}> [--NAME VALUE]...",
                names.join("|")
            );
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ----------------------------------------------------- the fake tree --

    /// A tree held in memory. `scans` answers the scanner by its joined
    /// arguments; with no canned answer, `code` returns the file itself,
    /// `paths`, `paths-prod` and `deps` return nothing for a file that
    /// exists, `unsafe` finds nothing and `closure` returns its roots.
    #[derive(Default)]
    struct Fake {
        files: BTreeMap<String, Vec<u8>>,
        untracked: BTreeSet<String>,
        scans: BTreeMap<String, Scan>,
    }

    /// Git's default pathspec match: `*` matches any run, `/` included.
    fn glob(pat: &str, p: &str) -> bool {
        let (pb, sb) = (pat.as_bytes(), p.as_bytes());
        let (mut i, mut j) = (0, 0);
        let mut star: Option<(usize, usize)> = None;
        while j < sb.len() {
            if i < pb.len() && pb[i] == b'*' {
                star = Some((i, j));
                i += 1;
            } else if i < pb.len() && pb[i] == sb[j] {
                i += 1;
                j += 1;
            } else if let Some((s, m)) = star {
                i = s + 1;
                j = m + 1;
                star = Some((s, m + 1));
            } else {
                return false;
            }
        }
        pb[i..].iter().all(|&c| c == b'*')
    }

    impl Fake {
        fn file(self, p: &str, body: &str) -> Self {
            self.bytes(p, body.as_bytes())
        }

        fn bytes(mut self, p: &str, b: &[u8]) -> Self {
            self.files.insert(p.to_owned(), b.to_vec());
            self
        }

        fn scan(mut self, args: &str, code: i32, out: &str) -> Self {
            self.scans.insert(
                args.to_owned(),
                Scan {
                    code,
                    out: out.to_owned(),
                },
            );
            self
        }

        fn untracked(mut self, p: &str) -> Self {
            self.untracked.insert(p.to_owned());
            self
        }

        fn without(mut self, p: &str) -> Self {
            self.files.remove(p);
            self
        }
    }

    impl Repo for Fake {
        fn ls(&self, specs: &[&str]) -> Result<Vec<String>, String> {
            Ok(self
                .files
                .keys()
                .filter(|p| !self.untracked.contains(*p))
                .filter(|p| specs.is_empty() || specs.iter().any(|s| glob(s, p)))
                .cloned()
                .collect())
        }

        fn read(&self, path: &str) -> Option<Vec<u8>> {
            self.files.get(path).cloned()
        }

        fn scan(&self, args: &[&str]) -> Result<Scan, String> {
            let key = args.join(" ");
            if let Some(s) = self.scans.get(&key) {
                return Ok(s.clone());
            }
            let answer = |code: i32, out: String| Ok(Scan { code, out });
            match args {
                ["code", f] => match self.files.get(*f) {
                    Some(b) => answer(0, String::from_utf8_lossy(b).into_owned()),
                    None => answer(2, String::new()),
                },
                ["paths" | "paths-prod" | "deps", f] => answer(
                    if self.files.contains_key(*f) { 0 } else { 2 },
                    String::new(),
                ),
                ["unsafe", ..] => answer(0, String::new()),
                ["closure", _, roots @ ..] => {
                    answer(0, roots.iter().map(|r| format!("{r}\n")).collect())
                }
                _ => Err(format!("no canned scan for `{key}`")),
            }
        }

        fn listing(&self) -> Result<String, String> {
            Ok("LISTING".to_owned())
        }

        fn walk(&self, dir: &str) -> Result<Option<Vec<String>>, String> {
            let pre = format!("{dir}/");
            let v: Vec<String> = self
                .files
                .keys()
                .filter(|p| p.starts_with(&pre))
                .cloned()
                .collect();
            Ok((!v.is_empty()).then_some(v))
        }
    }

    fn opts(pairs: &[(&str, &str)]) -> Opts {
        Opts(
            pairs
                .iter()
                .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
                .collect(),
        )
    }

    fn ran(r: Result<Report, String>) -> Report {
        r.unwrap_or_else(|e| panic!("the gate did not run: {e}"))
    }

    /// The report refused, and says `what`.
    fn refused(r: &Report, what: &str) {
        let t = r.text();
        assert!(
            r.bad,
            "expected a refusal saying {what:?}, got a pass:\n{t}"
        );
        assert!(t.contains(what), "expected {what:?} in:\n{t}");
    }

    /// The report passed, and says `what`.
    fn passed(r: &Report, what: &str) {
        let t = r.text();
        assert!(!r.bad, "expected a pass, got a refusal:\n{t}");
        assert!(t.contains(what), "expected {what:?} in:\n{t}");
    }

    // ---------------------------------------------------------- helpers --

    #[test]
    fn the_fake_globs_like_git() {
        assert!(glob("crates/*/Cargo.toml", "crates/a/Cargo.toml"));
        assert!(glob("crates/*/Cargo.toml", "crates/a/b/Cargo.toml"));
        assert!(!glob("crates/*/Cargo.toml", "crates/Cargo.toml"));
        assert!(glob("*Cargo.lock", "fuzz/Cargo.lock"));
        assert!(glob("crates/**/*.rs", "crates/a/src/x.rs"));
        assert!(!glob("crates/**/*.rs", "crates/x.rs"));
    }

    #[test]
    fn records_are_read_as_awk_and_grep_read_them() {
        assert_eq!(lines_of(""), Vec::<&str>::new());
        assert_eq!(lines_of("a"), vec!["a"]);
        assert_eq!(lines_of("a\n"), vec!["a"]);
        assert_eq!(lines_of("a\r\n\nb"), vec!["a\r", "", "b"]);
        assert!(is_space(b'\r') && is_space(0x0b) && !is_space(b'x'));
        assert_eq!(skip_space(b" \t x", 0), 3);
        assert!(is_name(b'-') && is_name(b'_') && !is_name(b'.'));
        assert_eq!(find_all(b"aaa", b"aa").collect::<Vec<_>>(), vec![0, 1]);
        assert!(!contains(b"ab", b""));
        assert!(at(b"ab", 1, b'b') && !at(b"ab", 2, b'b'));
    }

    #[test]
    fn a_scanner_line_is_split_at_its_line_tag() {
        assert_eq!(tagged_tails("f.rs:12:std::fs::read"), vec!["std::fs::read"]);
        assert_eq!(tagged_tails("a:1:b:2:c"), vec!["b:2:c", "c"]);
        assert_eq!(tagged_tails(":1:x"), Vec::<&str>::new());
        assert_eq!(tagged_tails("f:x:1:y"), vec!["y"]);
        assert_eq!(after_tag("crates/a.rs:3:std::x"), "std::x");
        assert_eq!(after_tag("a:b:1:c"), "a:b:1:c");
        assert_eq!(after_tag("no tag"), "no tag");
        assert_eq!(before_tag("crates/a.rs:3:std::x"), "crates/a.rs");
        assert_eq!(before_tag("a:b:1:c"), "a:b");
        assert_eq!(before_tag("no tag"), "no tag");
    }

    #[test]
    fn an_allowlist_grants_its_first_entry_and_refuses_a_count_that_is_not_one() {
        let list = "\n  a.rs 2\nb.rs\n\ta.rs 9\nc.rs x\n";
        assert_eq!(
            allow_entries(list),
            vec![
                ("a.rs".to_owned(), Some("2".to_owned())),
                ("b.rs".to_owned(), None),
                ("a.rs".to_owned(), Some("9".to_owned())),
                ("c.rs".to_owned(), Some("x".to_owned())),
            ]
        );
        assert_eq!(allowed(list, "a.rs"), Ok(2));
        assert_eq!(allowed(list, "b.rs"), Ok(0));
        assert_eq!(allowed(list, "z.rs"), Ok(0));
        assert!(allowed(list, "c.rs").is_err());
        assert!(of_file("a.rs:1:x", "a.rs") && of_file("a.rs", "a.rs"));
        assert!(!of_file("a.rsx:1:x", "a.rs"));
    }

    #[test]
    fn a_declared_list_and_a_measured_one_differ_as_multisets() {
        let s = |v: &[&str]| v.iter().map(|x| (*x).to_owned()).collect::<Vec<_>>();
        assert_eq!(
            difference(&s(&["a", "b", "b"]), &s(&["b", "c"])),
            s(&["< a", "< b", "> c"])
        );
        assert!(difference(&s(&["a"]), &s(&["a"])).is_empty());
        assert_eq!(words_sorted(" b\n a\t"), s(&["a", "b"]));
        assert_eq!(
            partition("UNRESOLVED x\n\ny\n", "UNRESOLVED").0,
            vec!["UNRESOLVED x"]
        );
    }

    #[test]
    fn options_are_named_pairs_and_a_misspelt_one_is_refused() {
        let a = |v: &[&str]| v.iter().map(|x| (*x).to_owned()).collect::<Vec<_>>();
        let o = Opts::parse(&a(&["--x", "1", "--y", ""])).unwrap_or_default();
        assert_eq!(o.need("x"), Ok("1"));
        assert_eq!(o.need("y"), Ok(""));
        assert!(o.need("z").is_err());
        assert!(o.only(&["x", "y"]).is_ok());
        assert!(o.only(&["x"]).is_err());
        assert!(Opts::parse(&a(&["x", "1"])).is_err());
        assert!(Opts::parse(&a(&["--x"])).is_err());
        assert!(Opts::parse(&a(&["--x", "1", "--x", "2"])).is_err());
    }

    #[test]
    fn the_runner_names_every_gate_and_refuses_an_unknown_one() {
        let f = Fake::default();
        assert!(run(&f, &[]).is_err());
        assert!(run(&f, &["gate99".to_owned()]).is_err());
        assert!(run(&f, &["gate24".to_owned(), "--x".to_owned(), "1".to_owned()]).is_err());
        let r = ran(run(&f, &["gate24".to_owned()]));
        refused(&r, "GATE 24 SCANNED NOTHING");
        let names: Vec<&str> = GATES.iter().map(|(n, _)| *n).collect();
        assert_eq!(
            names,
            [
                "gate13", "gate15", "gate16", "gate17", "gate21", "gate22", "gate23", "gate24"
            ]
        );
    }

    #[test]
    fn the_real_tree_is_read_through_git_and_the_bound_scanner() {
        let real = Real::new();
        let me = real.ls(&[".github/gates_runtime.rs"]);
        assert_eq!(me, Ok(vec![".github/gates_runtime.rs".to_owned()]));
        assert!(real.read(".github/gates_runtime.rs").is_some());
        let walked = real.walk(".github").unwrap_or_default().unwrap_or_default();
        assert!(walked.contains(&".github/gates_runtime.rs".to_owned()));
        assert_eq!(real.walk(".github/gates_runtime.rs"), Ok(None));
        let listing = real.listing().unwrap_or_default();
        let held = std::fs::read_to_string(&listing).unwrap_or_default();
        assert!(held.split('\0').any(|p| p == ".github/gates_runtime.rs"));
        let scanned = real.scan(&["code", ".github/gates_runtime.rs"]);
        if scanner().is_some() {
            assert!(scanned.is_ok_and(|s| s.ok() && s.out.contains("fn scanner")));
        } else {
            assert!(scanned.is_err());
        }
    }

    // ---------------------------------------------------------- gate 13 --

    #[test]
    fn a_dependency_key_is_matched_with_its_family_and_not_inside_a_longer_name() {
        let k = |l: &str, n: &str| decl_key(l.as_bytes(), n.as_bytes());
        assert!(k("F:1:pyo3 = \"1\"", "pyo3"));
        assert!(k("F:1:  pyo3.workspace = true", "pyo3"));
        assert!(k("F:1:deps = { pyo3 = \"1\" }", "pyo3"));
        assert!(k("F:1:pyo3-ffi\t= \"1\"", "pyo3"));
        assert!(k("F:1:mlua_sys.version = \"1\"\r", "mlua"));
        assert!(!k("F:1:mpyo3 = 1", "pyo3"));
        assert!(!k("F:1:pyo3- = 1", "pyo3"));
        assert!(!k("F:1:pyo3.v1 = 1", "pyo3"));
        assert!(!k("F:1:pyo3", "pyo3"));
        assert_eq!(fam_ends(b"x-y", 0, b"x"), vec![1, 3]);
        assert!(fam_ends(b"x", 2, b"x").is_empty());
        assert!(eq_at(b"  =", 0) && !eq_at(b" x=", 0));
    }

    #[test]
    fn a_dependency_table_header_is_matched_in_every_table_shape() {
        let t = |l: &str, n: &str| decl_table(l.as_bytes(), n.as_bytes());
        assert!(t("F:1:[dependencies.pyo3]", "pyo3"));
        assert!(t("F:1:[dev-dependencies.mlua-sys]\r", "mlua"));
        assert!(t("F:1:[target.'cfg(unix)'.dependencies.v8]", "v8"));
        assert!(t("F:1:[workspace.dependencies.jni]", "jni"));
        assert!(!t("F:1:[dependencies.pyo3.x]", "pyo3"));
        assert!(!t("F:1:[package.pyo3]", "pyo3"));
        assert!(!t("F:1:[dependencies.pyo3", "pyo3"));
        assert!(!t("F:1:[pyo3.dependencies]", "pyo3"));
    }

    #[test]
    fn a_rename_a_parsed_declaration_and_a_lock_name_are_each_matched() {
        let b = |s: &str| s.as_bytes().to_vec();
        assert!(decl_package(
            &b("F:1:js = { package = \"boa_engine\" }"),
            b"boa_engine"
        ));
        assert!(!decl_package(
            &b("F:1:js = { package = 'boa_engine' }"),
            b"boa_engine"
        ));
        assert!(parsed_hit(
            &b("F:3:dev-dependencies:js:boa_engine"),
            b"boa_engine"
        ));
        assert!(parsed_hit(&b("F:3:dependencies:pyo3:pyo3"), b"pyo3"));
        assert!(parsed_hit(&b("F:3:dependencies:pyo3:x"), b"pyo3"));
        assert!(!parsed_hit(&b("F:3:dependencies:pyo3x:pyo3x"), b"pyo3"));
        assert!(!parsed_hit(&b("F:3:dependencies:x:y"), b"pyo3"));
        assert!(lock_hit(
            &b("Cargo.lock:10:name = \"wasmer-types\""),
            b"wasmer"
        ));
        assert!(!lock_hit(
            &b("Cargo.lock:10:name = \"notwasmer\""),
            b"wasmer"
        ));
        assert!(!lock_hit(&b("Cargo.lock:10: name = \"v8\""), b"v8"));
        assert!(build_key(&b("F:3:build = \"gen.rs\"")));
        assert!(build_key(&b("F:3:  links = \"z\"")));
        assert!(!build_key(&b("F:3:build-dependencies = 1")));
        assert!(!build_key(&b("F:3:rebuild = 1")));
        assert!(key_is(
            b"wildcards         = \"deny\"",
            b"wildcards",
            b"\"deny\""
        ));
        assert!(!key_is(
            b"wildcards = \"warn\" # was \"deny\"",
            b"wildcards",
            b"\"deny\""
        ));
        assert!(names_exactly(b"  { name = \"v8\" },", b"v8"));
        assert!(!names_exactly(b"  { name = \"v8-x\" },", b"v8"));
        assert!(pins_cargo_deny(
            b"    cargo install cargo-deny --version 0.18.3"
        ));
        assert!(!pins_cargo_deny(
            b"cargo install cargo-deny --version 0.18.3"
        ));
        assert!(!pins_cargo_deny(
            b"    cargo install cargo-deny --version x"
        ));
        assert!(plain_name("boa_engine") && !plain_name("py.o3") && !plain_name(""));
    }

    fn deny_toml() -> String {
        let mut s = String::from("[bans]\nwildcards         = \"deny\"\ndeny = [\n");
        for n in DENY_MUST {
            s.push_str(&format!("    {{ name = \"{n}\" }},\n"));
        }
        s.push_str("]\n");
        s
    }

    fn clean13() -> Fake {
        Fake::default()
            .file(
                "Cargo.toml",
                "[workspace]\nmembers = [\"crates/a\"]\n# pyo3 = \"0.20\" bound the predecessor\n",
            )
            .file(
                "crates/a/Cargo.toml",
                "[package]\nname = \"a\"\n\n[dependencies]\nserde = \"1\"\n",
            )
            .file("Cargo.lock", "[[package]]\nname = \"serde\"\nversion = \"1.0.0\"\n")
            .file("deny.toml", &deny_toml())
            .file(
                WORKFLOW,
                "      - run: |\n          cargo install cargo-deny --version 0.18.3\n",
            )
            .file("crates/cli/build.rs", "fn main() {}\n")
            .scan(
                "deps crates/a/Cargo.toml",
                0,
                "crates/a/Cargo.toml:5:dependencies:serde:serde\n",
            )
            .scan(
                &format!("step-runs {WORKFLOW} cargo deny check"),
                0,
                "present  1 step(s) run `cargo deny check` unconditionally and blocking\n",
            )
            .scan(
                &format!("step-runs {WORKFLOW} cargo install cargo-deny --version "),
                0,
                "present  1 step(s) run `cargo install cargo-deny --version ` unconditionally and blocking\n",
            )
    }

    const BANNED13: &str = "pyo3 mlua lua rquickjs boa_engine wasmer v8";

    fn g13_with(f: &Fake, banned: &str, decl: &str, build: &str) -> Report {
        ran(gate13(
            f,
            &opts(&[
                ("banned", banned),
                ("allow-decl", decl),
                ("allow-lock", ""),
                ("allow-build", build),
            ]),
        ))
    }

    fn g13(f: &Fake) -> Report {
        g13_with(f, BANNED13, "", "crates/cli/build.rs 1")
    }

    #[test]
    fn gate13_passes_a_clean_tree_and_reads_a_comment_as_prose() {
        let r = g13(&clean13());
        passed(
            &r,
            "OK — no foreign runtime is declared or resolved by name",
        );
        passed(&r, "allowed  crates/cli/build.rs — 1 of 1");
        passed(&r, "this gate refuses lua and deny.toml does not list it");
        passed(&r, "7 banned crate name(s)");
    }

    #[test]
    fn gate13_refuses_a_declared_binding_by_line_and_by_parse() {
        let f = clean13().file(
            "crates/a/Cargo.toml",
            "[package]\nname = \"a\"\n\n[dependencies]\nmlua-sys = \"0.9\"\n",
        );
        refused(
            &g13(&f),
            "REFUSED  crates/a/Cargo.toml — 1 occurrence(s), 0 allowed",
        );
        let f = clean13()
            .file(
                "crates/a/Cargo.toml",
                "[dependencies]\njs = { package = 'boa_engine' }\n",
            )
            .scan(
                "deps crates/a/Cargo.toml",
                0,
                "crates/a/Cargo.toml:2:dependencies:js:boa_engine\n",
            );
        let r = g13(&f);
        refused(&r, "crates/a/Cargo.toml:2:dependencies:js:boa_engine");
        let f = clean13().file(
            "tools/odd dir/Cargo.toml",
            "[dependencies]\npyo3 = \"0.20\"\n",
        );
        refused(
            &g13(&f),
            "REFUSED  tools/odd dir/Cargo.toml — 1 occurrence(s)",
        );
    }

    #[test]
    fn gate13_refuses_a_binding_in_the_lock() {
        let f = clean13().file(
            "Cargo.lock",
            "[[package]]\nname = \"serde\"\n\n[[package]]\nname = \"wasmer-types\"\n",
        );
        refused(&g13(&f), "REFUSED  Cargo.lock — 1 occurrence(s), 0 allowed");
    }

    #[test]
    fn gate13_refuses_every_shape_of_an_unlisted_build_script() {
        let f = clean13().file("crates/a/build.rs", "fn main() {}\n");
        refused(
            &g13(&f),
            "REFUSED  crates/a/build.rs — 1 occurrence(s), 0 allowed",
        );
        let f = clean13().scan(
            "closure LISTING crates/cli/build.rs",
            0,
            "crates/cli/build.rs\ncrates/cli/helper.rs\n",
        );
        refused(
            &g13(&f),
            "crates/cli/helper.rs:1:a module compiled into a build script",
        );
        let f = clean13().scan(
            "closure LISTING crates/cli/build.rs",
            1,
            "UNRESOLVED crates/cli/build.rs:3: mod gone\ncrates/cli/build.rs\n",
        );
        refused(
            &g13(&f),
            "a build script pulls in code layer 3 cannot resolve",
        );
        let f = clean13().file(
            "crates/a/Cargo.toml",
            "[package]\nname = \"a\"\nbuild = \"gen.rs\"\n",
        );
        refused(&g13(&f), "crates/a/Cargo.toml:3:build = \"gen.rs\"");
    }

    #[test]
    fn gate13_audits_its_allowlists() {
        let r = g13_with(
            &clean13(),
            BANNED13,
            "",
            "crates/cli/build.rs 1\ncrates/cli/gone.rs 1",
        );
        refused(
            &r,
            "STALE ALLOWLIST ENTRY — crates/cli/gone.rs is not a tracked file",
        );
        let r = g13_with(
            &clean13(),
            BANNED13,
            "crates/a/Cargo.toml 1",
            "crates/cli/build.rs 1",
        );
        passed(
            &r,
            "crates/a/Cargo.toml no longer matches layer 1 — drop the entry",
        );
        let r = g13_with(&clean13(), BANNED13, "", "crates/cli/build.rs one");
        refused(&r, "has the count `one`, not an integer");
        let r = g13_with(&clean13(), "pyo3 py.o3", "", "crates/cli/build.rs 1");
        refused(&r, "GATE 13'S LIST NAMES `py.o3`");
    }

    #[test]
    fn gate13_refuses_a_disarmed_deny_toml_or_gate_3() {
        let f = clean13().file("deny.toml", &deny_toml().replace("{ name = \"jni\" },", ""));
        refused(&g13(&f), "REFUSED  deny.toml no longer bans jni");
        let f = clean13().file(
            "deny.toml",
            &deny_toml().replace("\"deny\"\n", "\"warn\"\n"),
        );
        refused(&g13(&f), "deny.toml no longer sets wildcards = \"deny\"");
        refused(
            &g13(&clean13().without("deny.toml")),
            "deny.toml is not tracked",
        );
        let f = clean13().scan(
            &format!("step-runs {WORKFLOW} cargo deny check"),
            1,
            "REFUSED  the step sets continue-on-error\n",
        );
        refused(&g13(&f), "nothing in this workflow RUNS cargo deny check");
        let f = clean13().file(WORKFLOW, "      - run: cargo install cargo-deny\n");
        refused(&g13(&f), "cargo-deny is installed unpinned");
    }

    #[test]
    fn gate13_refuses_a_tree_it_cannot_read() {
        let f = Fake::default().file("Cargo.lock", "");
        refused(&g13(&f), "GATE 13 WALKED NO MANIFEST");
        refused(
            &g13(&clean13().without("Cargo.lock")),
            "GATE 13 WALKED NO LOCK FILE",
        );
        let f = clean13().scan("deps crates/a/Cargo.toml", 2, "");
        refused(
            &g13(&f),
            "REFUSED  crates/a/Cargo.toml could not be read as TOML",
        );
    }

    // ---------------------------------------------------------- gate 15 --

    #[test]
    fn the_word_is_assembled_and_this_file_never_spells_it() {
        assert_eq!(WORD.len(), WORD_LEN);
        let me = include_str!("gates_runtime.rs").to_ascii_lowercase();
        assert_eq!(count_ci(me.as_bytes(), WORD.as_bytes()), 0);
        let upper = WORD.to_ascii_uppercase();
        let doubled = format!("{upper}{WORD}");
        assert_eq!(count_ci(doubled.as_bytes(), WORD.as_bytes()), 2);
        assert_eq!(count_ci(b"", WORD.as_bytes()), 0);
        assert_eq!(
            mask(doubled.as_bytes(), WORD.as_bytes()),
            "[BANNED WORD][BANNED WORD]"
        );
        assert!(binary_or_empty(b"") && binary_or_empty(b"\n\n") && binary_or_empty(b"a\0"));
        assert!(binary_or_empty(b"\xff") && !binary_or_empty(b"text\n"));
    }

    fn clean15() -> Fake {
        Fake::default().file("README.md", "plain\n").file(
            "crates/pull/src/kite.rs",
            &format!("// {WORD} sdk\n// the {WORD} one\n"),
        )
    }

    fn g15(f: &Fake, allow: &str) -> Report {
        ran(gate15(f, &opts(&[("allow", allow)])))
    }

    #[test]
    fn gate15_passes_an_allowed_file_and_warns_on_a_loose_entry() {
        let r = g15(&clean15(), "\ncrates/pull/src/kite.rs 2\n");
        passed(&r, "allowed  crates/pull/src/kite.rs — 2 of 2");
        passed(&r, "OK — the banned token appears in no tracked file.");
        let r = g15(&clean15(), "crates/pull/src/kite.rs 2\nREADME.md 1");
        passed(
            &r,
            "README.md no longer contains the token — drop the entry",
        );
    }

    #[test]
    fn gate15_refuses_the_word_in_any_case_and_in_a_binary_file_and_masks_it() {
        let shout = WORD.to_ascii_uppercase();
        let f = clean15().file(
            "docs/notes with space.md",
            &format!("a\r\nPorted from {shout}\r\n"),
        );
        let r = g15(&f, "crates/pull/src/kite.rs 2");
        refused(
            &r,
            "REFUSED  docs/notes with space.md — 1 occurrence(s), 0 allowed",
        );
        refused(&r, "2:Ported from [BANNED WORD]\r");
        assert_eq!(count_ci(r.text().as_bytes(), WORD.as_bytes()), 0);
        let f = clean15().bytes("docs/blob.md", format!("x\0y {shout} z\n").as_bytes());
        let r = g15(&f, "crates/pull/src/kite.rs 2");
        refused(&r, "REFUSED  docs/blob.md — 1 occurrence(s), 0 allowed");
        refused(&r, "1 binary or empty");
    }

    #[test]
    fn gate15_refuses_a_count_over_its_ceiling_and_a_bad_or_stale_entry() {
        refused(
            &g15(&clean15(), "crates/pull/src/kite.rs 1"),
            "2 occurrence(s), 1 allowed",
        );
        refused(
            &g15(&clean15(), "crates/pull/src/kite.rs two"),
            "not an integer",
        );
        let r = g15(
            &clean15(),
            "crates/pull/src/kite.rs 2\ncrates/pull/src/gone.rs 1",
        );
        refused(&r, "STALE ALLOWLIST ENTRY — crates/pull/src/gone.rs");
        refused(&g15(&Fake::default(), ""), "GATE 15 WALKED NOTHING");
    }

    // ---------------------------------------------------------- gate 16 --

    #[test]
    fn the_forbid_is_read_as_the_line_regex_read_it() {
        assert!(forbids_unsafe("//! x\n#![forbid(unsafe_code)]\n"));
        assert!(forbids_unsafe("#![forbid(missing_docs, unsafe_code)]"));
        assert!(forbids_unsafe("#![forbid(unsafe_code)]\r\n"));
        assert!(!forbids_unsafe("#![forbid(unsafe_codex)]"));
        assert!(!forbids_unsafe("#![forbid(xunsafe_code)]"));
        assert!(!forbids_unsafe("#![forbid(missing_docs)] // unsafe_code"));
        assert!(!forbids_unsafe(" #![forbid(unsafe_code)]"));
        assert!(!forbids_unsafe("#![deny(unsafe_code)]"));
        assert!(!forbids_unsafe("#![forbid(unsafe_codeé)]"));
        assert!(word_char(Some('é')) && !word_char(None) && !word_char(Some(']')));
    }

    #[test]
    fn the_lint_opt_in_and_the_other_roots_are_read_as_awk_read_them() {
        assert!(inherits_lints("[package]\n[lints]\nworkspace = true\n"));
        assert!(inherits_lints("[lints] # x\n  workspace   =   true\n"));
        assert!(!inherits_lints("[lints]\n[x]\nworkspace = true\n"));
        assert!(!inherits_lints("workspace = true\n[lints]\n"));
        assert!(!inherits_lints("[lints]\nworkspace = false\n"));
        assert!(other_root("crates/a/build.rs"));
        assert!(other_root("crates/a/tests/t.rs"));
        assert!(!other_root("crates/a/src/x.rs"));
        assert!(other_root("crates/a/src/bin/x.rs"));
        assert!(other_root("crates/a/tests/t/main.rs"));
        assert!(!other_root("crates/a/tests/t/other.rs"));
        assert!(!other_root("crates/a/b/c/d/e.rs"));
    }

    const ALLOC: &str = "crates/pull/tests/allocation.rs";
    const ROOTS16: &str = "crates/a/tests/t.rs crates/pull/tests/allocation.rs";

    fn clean16() -> Fake {
        Fake::default()
            .file(
                "Cargo.toml",
                "[workspace.lints.rust]\nunsafe_code = \"deny\"\n",
            )
            .file(
                "crates/a/Cargo.toml",
                "[package]\nname = \"a\"\n\n[lints]\nworkspace = true\n",
            )
            .file("crates/a/src/lib.rs", "//! a\n\n#![forbid(unsafe_code)]\n")
            .file("crates/a/tests/t.rs", "#[test]\nfn t() {}\n")
            .file(ALLOC, "#![allow(unsafe_code)]\n")
            .file(".github/tool.rs", "#![forbid(unsafe_code)]\nfn main() {}\n")
            .scan(
                &format!("unsafe {ROOTS16}"),
                1,
                "crates/pull/tests/allocation.rs:3: `unsafe`\n",
            )
    }

    fn g16(f: &Fake, allow: &str) -> Report {
        ran(gate16(
            f,
            &opts(&[("allow-root", allow), ("allow-unsafe-test-root", ALLOC)]),
        ))
    }

    #[test]
    fn gate16_passes_a_clean_tree_and_the_one_excepted_allocator() {
        let r = g16(&clean16(), "\n");
        passed(
            &r,
            "OK — every crate root forbids unsafe and inherits the lint table.",
        );
        passed(
            &r,
            "none outside crates/pull/tests/allocation.rs (1 site(s) there, D-1450)",
        );
        passed(&r, "2 root(s), 2 file(s) in their closure");
    }

    #[test]
    fn gate16_refuses_a_crate_root_without_the_forbid() {
        let f = clean16().file("crates/a/src/main.rs", "fn main() {}\n");
        refused(
            &g16(&f, ""),
            "REFUSED  crates/a/src/main.rs carries no #![forbid(unsafe_code)]",
        );
        let r = g16(&f, "crates/a/src/main.rs");
        passed(
            &r,
            "allowed  crates/a/src/main.rs — no forbid, listed above",
        );
        let r = g16(&clean16(), "crates/a/src/lib.rs");
        passed(
            &r,
            "crates/a/src/lib.rs forbids unsafe after all — drop the entry",
        );
        refused(
            &g16(&clean16(), "crates/a/src/gone.rs"),
            "STALE ALLOWLIST ENTRY",
        );
        let f = clean16().file("crates/b/Cargo.toml", "[lints]\nworkspace = true\n");
        refused(
            &g16(&f, ""),
            "crates/b/Cargo.toml — neither src/lib.rs nor src/main.rs",
        );
        let f = Fake::default().file("crates/a/src/lib.rs", "#![forbid(unsafe_code)]");
        refused(&g16(&f, ""), "GATE 16 EXAMINED NO CRATE ROOT");
    }

    #[test]
    fn gate16_refuses_unsafe_in_another_root_or_beyond_the_excepted_file() {
        let f = clean16().scan(
            &format!("unsafe {ROOTS16}"),
            1,
            "crates/a/tests/t.rs:2: `unsafe`\n",
        );
        refused(&g16(&f, ""), "crates/a/tests/t.rs:2: `unsafe`");
        let f = clean16().scan(
            &format!("unsafe {ROOTS16}"),
            1,
            "crates/pull/tests/allocation.rs:9: a foreign `extern` interface\n",
        );
        refused(
            &g16(&f, ""),
            "the excepted allocator file holds more than unsafe",
        );
        let f = clean16().scan(&format!("unsafe {ROOTS16}"), 2, "");
        refused(&g16(&f, ""), "a file in the closure could not be lexed");
        let f = clean16().scan(&format!("closure LISTING {ROOTS16}"), 1, "UNRESOLVED x\n");
        refused(
            &g16(&f, ""),
            "a root pulls in code this layer cannot resolve",
        );
        let f = clean16().without("crates/a/tests/t.rs").without(ALLOC);
        refused(&g16(&f, ""), "no test, bench or build root found");
    }

    #[test]
    fn gate16_refuses_an_unarmed_tool_or_lint_table() {
        let f = clean16().file(".github/zz.rs", "fn main() {}\n");
        refused(
            &g16(&f, ""),
            "REFUSED  .github/zz.rs lacks #![forbid(unsafe_code)]",
        );
        let f = clean16().scan("unsafe .github/tool.rs", 1, ".github/tool.rs:2: `unsafe`\n");
        refused(&g16(&f, ""), "REFUSED  .github/tool.rs lacks");
        let f = clean16().without(".github/tool.rs");
        refused(&g16(&f, ""), "no .github/*.rs tool read");
        let f = clean16().file("Cargo.toml", "unsafe_code = \"warn\"\n");
        refused(&g16(&f, ""), "root Cargo.toml no longer denies unsafe_code");
        let f = clean16().file("crates/a/Cargo.toml", "[package]\nname = \"a\"\n");
        refused(
            &g16(&f, ""),
            "crates/a/Cargo.toml does not write [lints] workspace = true",
        );
    }

    // ---------------------------------------------------------- gate 17 --

    #[test]
    fn the_logging_rule_reads_canonical_paths() {
        for y in [
            "telemetry::emit",
            "telemetry!",
            "log::info!",
            "tracing::span",
            "std::println!",
            "core::dbg!",
            "eprint!",
            "alloc::eprintln!",
            "std::io::stderr",
            "io::stdout",
        ] {
            assert!(logs(y), "{y}");
        }
        for y in [
            "telemetry",
            "telemetry::",
            "println",
            "my::io::stderr",
            "logger::x",
            "std::fmt",
        ] {
            assert!(!logs(y), "{y}");
        }
        assert_eq!(rooted("std::x"), vec!["std::x", "x"]);
    }

    fn clean17() -> Fake {
        Fake::default()
            .file("crates/vocab/src/lib.rs", "pub fn f() {}\n")
            .scan(
                "paths crates/vocab/src/lib.rs",
                0,
                "crates/vocab/src/lib.rs:1:std::vec::Vec\n",
            )
    }

    fn g17(f: &Fake, swept: &str) -> Report {
        ran(gate17(f, &opts(&[("swept", swept)])))
    }

    #[test]
    fn gate17_passes_a_silent_sweep() {
        passed(
            &g17(&clean17(), "vocab"),
            "OK — 1 swept source file(s) scanned",
        );
    }

    #[test]
    fn gate17_refuses_a_print_or_an_event_in_the_sweep_however_it_is_mounted() {
        let f = clean17().scan(
            "paths crates/vocab/src/lib.rs",
            0,
            "crates/vocab/src/lib.rs:3:std::eprintln!\n",
        );
        refused(&g17(&f, "vocab"), "LOGGING INSIDE THE SWEEP: crates/vocab");
        let f = clean17()
            .file("crates/vocab/extra/m.rs", "x")
            .scan(
                "closure LISTING crates/vocab/src/lib.rs",
                0,
                "crates/vocab/src/lib.rs\ncrates/vocab/extra/m.rs\n",
            )
            .scan(
                "paths crates/vocab/extra/m.rs",
                0,
                "crates/vocab/extra/m.rs:2:dbg!\n",
            );
        refused(&g17(&f, "vocab"), "crates/vocab/extra/m.rs:2:dbg!");
    }

    #[test]
    fn gate17_refuses_what_it_cannot_read() {
        refused(
            &g17(&clean17(), "vocab engine"),
            "crates/engine/src/lib.rs is not tracked",
        );
        let f = clean17().scan(
            "closure LISTING crates/vocab/src/lib.rs",
            1,
            "UNRESOLVED x\n",
        );
        refused(&g17(&f, "vocab"), "the module closure did not resolve");
        let f = clean17().scan("paths crates/vocab/src/lib.rs", 2, "");
        refused(&g17(&f, "vocab"), "a source file could not be lexed");
        refused(
            &g17(&Fake::default(), "vocab"),
            "GATE 17 READ NO TRACKED FILE",
        );
    }

    // ---------------------------------------------------------- gate 23 --

    #[test]
    fn the_clause_b_and_c_readers_match_their_expressions() {
        assert!(links_telemetry(
            "crates/a/Cargo.toml:7:dependencies:telemetry:telemetry"
        ));
        assert!(links_telemetry("m:7:dependencies:log:telemetry"));
        assert!(!links_telemetry("m:7:dev-dependencies:telemetry:telemetry"));
        assert!(!links_telemetry("m:7:dependencies:telemetry:store"));
        assert!(!links_telemetry("a:b:7:dependencies:x:telemetry"));
        assert!(!links_telemetry("m::dependencies:x:telemetry"));
        assert!(writes_stderr("api/src/x.rs:eprintln!:2"));
        assert!(writes_stderr("x:handle:3"));
        assert!(!writes_stderr("api/src/x.rs:println!:2"));
        assert!(!writes_stderr("x:eprint!:"));
        let code = "let d = std::env::temp_dir();\na\nb\nc\nd.push(std::process::id());\n";
        assert_eq!(fixed_temp_lines(code, 3), vec![1]);
        assert!(fixed_temp_lines(code, 4).is_empty());
        assert!(fixed_temp_lines("x(env::temp_dir(), process::id())", 3).is_empty());
        assert_eq!(fixed_temp_lines("a\nenv::temp_dir()", 3), vec![2]);
    }

    fn clean23() -> Fake {
        Fake::default()
            .file("crates/api/src/server.rs", "x")
            .file("crates/api/Cargo.toml", "x")
            .file("crates/telemetry/src/sink.rs", "x")
            .file(
                "crates/api/tests/t.rs",
                "let d = std::env::temp_dir().join(format!(\"x-{}\", std::process::id()));\n",
            )
            .scan(
                "paths crates/api/src/server.rs",
                0,
                "crates/api/src/server.rs:3:std::eprintln!\ncrates/api/src/server.rs:9:std::io::stdout\n",
            )
            .scan(
                "paths crates/telemetry/src/sink.rs",
                0,
                "crates/telemetry/src/sink.rs:4:eprintln!\n",
            )
            .scan(
                "deps crates/api/Cargo.toml",
                0,
                "crates/api/Cargo.toml:7:dependencies:telemetry:telemetry\n",
            )
    }

    const DECLARED23: &str = "api/src/server.rs:eprintln!:1\n  telemetry/src/sink.rs:eprintln!:1";

    fn g23_with(f: &Fake, declared: &str, handles: &str, fixed: &str) -> Report {
        ran(gate23(
            f,
            &opts(&[
                ("declared", declared),
                ("declared-handles", handles),
                ("allow-fixed", fixed),
                ("window", "3"),
            ]),
        ))
    }

    fn g23(f: &Fake) -> Report {
        g23_with(f, DECLARED23, "api/src/server.rs:handle:1", "\n")
    }

    #[test]
    fn gate23_passes_the_declared_surface() {
        let r = g23(&clean23());
        passed(&r, "OK - every print is declared");
        passed(&r, "ok       api writes to stderr and can also emit");
        passed(&r, "clause C: 0 temporary path(s)");
    }

    #[test]
    fn gate23_refuses_a_print_or_a_handle_that_is_not_declared_exactly() {
        let r = g23_with(
            &clean23(),
            "api/src/server.rs:eprintln!:1",
            "api/src/server.rs:handle:1",
            "",
        );
        refused(&r, "the print surface is not the declared one");
        refused(&r, "> telemetry/src/sink.rs:eprintln!:1");
        let r = g23_with(
            &clean23(),
            &format!("{DECLARED23} x.rs:dbg!:1"),
            "api/src/server.rs:handle:1",
            "",
        );
        refused(&r, "< x.rs:dbg!:1");
        let r = g23_with(&clean23(), DECLARED23, "", "");
        refused(&r, "the set of writes to a process stream is not the");
        let w = |e: &str| {
            ran(gate23(
                &clean23(),
                &opts(&[
                    ("declared", ""),
                    ("declared-handles", ""),
                    ("allow-fixed", ""),
                    ("window", e),
                ]),
            ))
        };
        assert!(
            gate23(
                &clean23(),
                &opts(&[
                    ("declared", ""),
                    ("declared-handles", ""),
                    ("allow-fixed", ""),
                    ("window", "x")
                ])
            )
            .is_err()
        );
        assert!(w("3").bad);
    }

    #[test]
    fn gate23_refuses_a_crate_whose_only_channel_is_stderr() {
        let f = clean23().scan(
            "deps crates/api/Cargo.toml",
            0,
            "crates/api/Cargo.toml:7:dev-dependencies:telemetry:telemetry\n",
        );
        refused(
            &g23(&f),
            "REFUSED  crate 'api' writes to stderr but declares no",
        );
        let f = clean23().untracked("crates/api/Cargo.toml");
        refused(&g23(&f), "REFUSED  crate 'api' writes to stderr");
    }

    #[test]
    fn gate23_refuses_a_temporary_path_that_names_no_process() {
        let f = clean23().file(
            "crates/core/tests/zz.rs",
            "fn t() {\n    let d = std::env::temp_dir().join(\"fixed\");\n}\n",
        );
        let r = g23(&f);
        refused(
            &r,
            "REFUSED  core/tests/zz.rs — 1 fixed temporary path(s), 0 allowed",
        );
        refused(&r, "             crates/core/tests/zz.rs:2");
        let r = g23_with(
            &f,
            DECLARED23,
            "api/src/server.rs:handle:1",
            "core/tests/zz.rs 1",
        );
        passed(&r, "allowed  core/tests/zz.rs — 1 of 1");
        let r = g23_with(
            &clean23(),
            DECLARED23,
            "api/src/server.rs:handle:1",
            "api/tests/t.rs 1",
        );
        passed(
            &r,
            "crates/api/tests/t.rs no longer names a fixed temporary path",
        );
        let r = g23_with(
            &clean23(),
            DECLARED23,
            "api/src/server.rs:handle:1",
            "gone.rs 1",
        );
        refused(&r, "STALE ENTRY — crates/gone.rs is not a tracked file");
        let r = g23_with(
            &f,
            DECLARED23,
            "api/src/server.rs:handle:1",
            "core/tests/zz.rs x",
        );
        refused(&r, "not an integer");
    }

    #[test]
    fn gate23_refuses_what_it_cannot_read() {
        refused(
            &g23(&Fake::default()),
            "GATE 23 READ NO FILE under crates/*/src.",
        );
        let f = clean23().scan("paths crates/api/src/server.rs", 2, "");
        refused(
            &g23(&f),
            "a source file under crates/*/src could not be lexed",
        );
        let f = clean23().scan("code crates/api/tests/t.rs", 2, "");
        refused(
            &g23(&f),
            "REFUSED  crates/api/tests/t.rs could not be lexed",
        );
    }

    // ---------------------------------------------------------- gate 21 --

    #[test]
    fn the_capability_readers_match_their_expressions() {
        for p in [
            "std::fs::read",
            "std::fs::File::open",
            "std::process::exit",
            "std::fs::write!",
            "std::net::TcpStream::connect",
        ] {
            assert!(capability_fn(p), "{p}");
        }
        for p in [
            "std::fs::File",
            "std::io::stdout",
            "std::fs::",
            "core::fs::read",
            "std::fsx::read",
        ] {
            assert!(!capability_fn(p), "{p}");
        }
        assert!(capability_glob("f:1:std::fs::*"));
        assert!(capability_glob("f:1:std::os::unix::fs::*"));
        assert!(!capability_glob("f:1:std::io::*"));
        assert!(!capability_glob("f:1:std::fs::File"));
        assert!(!capability_glob("f:1:std::fs::::*"));
    }

    fn clean21() -> Fake {
        Fake::default()
            .file("crates/telemetry/Cargo.toml", "[package]\n")
            .file("crates/telemetry/src/sink.rs", "x")
            .scan(
                "paths-prod crates/telemetry/src/sink.rs",
                0,
                "crates/telemetry/src/sink.rs:3:std::fs::OpenOptions::new\ncrates/telemetry/src/sink.rs:4:std::fs::File\n",
            )
    }

    fn g21(f: &Fake, declared: &str) -> Report {
        ran(gate21(f, &opts(&[("declared", declared)])))
    }

    #[test]
    fn gate21_passes_the_declared_surface() {
        let r = g21(&clean21(), "sink.rs:std::fs::OpenOptions::new:1");
        passed(
            &r,
            "OK - the logger depends on nothing and opens only its own files.",
        );
    }

    #[test]
    fn gate21_refuses_a_new_capability_a_glob_or_a_dependency() {
        refused(
            &g21(&clean21(), ""),
            "the logger's file-opening surface is not the declared one",
        );
        let f = clean21().scan(
            "paths-prod crates/telemetry/src/sink.rs",
            0,
            "crates/telemetry/src/sink.rs:3:std::fs::OpenOptions::new\ncrates/telemetry/src/sink.rs:9:std::fs::*\n",
        );
        refused(
            &g21(&f, "sink.rs:std::fs::OpenOptions::new:1"),
            "a glob import hides which capability",
        );
        let f = clean21().scan(
            "deps crates/telemetry/Cargo.toml",
            0,
            "crates/telemetry/Cargo.toml:4:dependencies:store:store\n",
        );
        refused(
            &g21(&f, "sink.rs:std::fs::OpenOptions::new:1"),
            "crates/telemetry declares dependencies:",
        );
        let f = clean21().untracked("crates/telemetry/Cargo.toml");
        refused(
            &g21(&f, "sink.rs:std::fs::OpenOptions::new:1"),
            "(unreadable)",
        );
    }

    #[test]
    fn gate21_refuses_what_it_cannot_read() {
        let f = Fake::default().file("crates/telemetry/Cargo.toml", "");
        refused(&g21(&f, ""), "crates/telemetry/src holds no tracked file");
        let f = clean21().scan("paths-prod crates/telemetry/src/sink.rs", 2, "");
        refused(
            &g21(&f, ""),
            "a crates/telemetry source file could not be lexed",
        );
    }

    // ---------------------------------------------------------- gate 22 --

    #[test]
    fn the_clause_readers_match_their_expressions() {
        assert_eq!(dependency_set(""), "");
        assert_eq!(
            dependency_set("f:1:dependencies:vocab:vocab\nf:2:dependencies:vocab:vocab\n"),
            "dependencies:vocab "
        );
        assert_eq!(
            dependency_set("f:1:dev-dependencies:x:store\n"),
            "dev-dependencies:store "
        );
        for l in [
            "pub use store::x;",
            "  use brutex_pull as p;",
            "use store",
            "pub  use\ttelemetry::x",
        ] {
            assert!(imports_store(l.as_bytes()), "{l}");
        }
        for l in [
            "use storey::x",
            "pub(crate) use store::x",
            "// use store::x",
            "usestore",
            "pubuse store",
        ] {
            assert!(!imports_store(l.as_bytes()), "{l}");
        }
        for y in [
            "std::fs",
            "core::os::unix",
            "std::process::Command",
            "libc::write",
            "memmap2::MmapMut",
            "memmap",
            "include_bytes!",
            "std::fs::",
        ] {
            assert!(reaches_fs(y), "{y}");
        }
        for y in ["libcx", "std::fsx", "std::io::Read", "include_str!"] {
            assert!(!reaches_fs(y), "{y}");
        }
        assert!(rest_of_path("") && rest_of_path("::x") && !rest_of_path("x"));
        assert!(starts_nothing("f:1:std::process::exit"));
        assert!(starts_nothing("f:1:std::process"));
        assert!(starts_nothing("f:1:alloc::process::id"));
        assert!(!starts_nothing("f:1:std::process::Command"));
        assert!(names_store("store::x") && names_store("brutex_lake::y"));
        assert!(!names_store("store::") && !names_store("restore::x") && !names_store("store"));
        assert!(computed_include(b"include_str!(x)"));
        assert!(!computed_include(b"include_str!(\"a\")"));
        assert!(!computed_include(b"include_str!()"));
        assert!(!computed_include(b"include_str!("));
        assert_eq!(
            included_paths(b"include_str!(\"a.md\") + include_str!(\"b.rs\")"),
            vec!["a.md", "b.rs"]
        );
        assert_eq!(included_paths(b"include_str!(\"a)b\")"), vec!["a)b"]);
        assert!(included_paths(b"include_str!(\"\")").is_empty());
        assert!(included_paths(b"include_str!(\"a\" )").is_empty());
    }

    const BANNED22: &str = "File::open|File::create|OpenOptions|fs::read|fs::write|fs::copy|fs::rename|fs::remove|fs::File|fs::metadata|fs::canonicalize|read_dir|std::fs|MmapOptions|Mmap::|memmap|include_bytes!|process::Command|Command::new|std::os::|libc::";

    fn clean22() -> Fake {
        Fake::default()
            .file("crates/vocab/Cargo.toml", "[package]\n")
            .file(
                "crates/vocab/src/lib.rs",
                "pub const D: &str = include_str!(\"../Cargo.toml\");\npub fn f() { std::process::exit(1) }\n",
            )
            .file("crates/vocab/tests/t.rs", "fn t() {}\n")
            .file("crates/engine/Cargo.toml", "[package]\n")
            .file("crates/engine/src/lib.rs", "pub fn g() {}\n")
            .scan(
                "deps crates/engine/Cargo.toml",
                0,
                "crates/engine/Cargo.toml:6:dependencies:vocab:vocab\n",
            )
            .scan(
                "paths crates/vocab/src/lib.rs",
                0,
                "crates/vocab/src/lib.rs:2:std::process::exit\n",
            )
    }

    fn g22_with(f: &Fake, pinned: &str, sweep: &str, banned: &str) -> Report {
        ran(gate22(
            f,
            &opts(&[("pinned", pinned), ("sweep", sweep), ("banned", banned)]),
        ))
    }

    fn g22(f: &Fake) -> Report {
        g22_with(f, "vocab engine", "vocab engine", BANNED22)
    }

    #[test]
    fn gate22_passes_a_sweep_that_cannot_reach_a_bar() {
        passed(
            &g22(&clean22()),
            "OK — vocab, indicators, engine and runner cannot reach a bar,",
        );
    }

    #[test]
    fn gate22_refuses_a_dependency_set_that_is_not_the_pinned_one() {
        let f = clean22().scan(
            "deps crates/engine/Cargo.toml",
            0,
            "crates/engine/Cargo.toml:6:dependencies:vocab:vocab\ncrates/engine/Cargo.toml:9:dependencies:store:store\n",
        );
        refused(
            &g22(&f),
            "crates/engine declares [dependencies:store dependencies:vocab ]",
        );
        let f = clean22().scan(
            "deps crates/vocab/Cargo.toml",
            0,
            "crates/vocab/Cargo.toml:3:dependencies:x:store\n",
        );
        refused(
            &g22(&f),
            "crates/vocab declares [dependencies:store ] and must declare []",
        );
        refused(
            &g22(&clean22().untracked("crates/engine/Cargo.toml")),
            "is not a tracked file. A pinned crate",
        );
        let f = clean22().scan("deps crates/engine/Cargo.toml", 2, "");
        refused(
            &g22(&f),
            "could not be read as TOML, so its dependency set is unknown",
        );
        let r = g22_with(&clean22(), "vocab runner", "vocab", BANNED22);
        refused(&r, "crates/runner/Cargo.toml is not a tracked file");
        let f = clean22().file("crates/runner/Cargo.toml", "");
        let r = g22_with(&f, "runner", "vocab", BANNED22);
        refused(
            &r,
            "crates/runner is pinned, but this gate writes no dependency set for it",
        );
        let r = g22_with(&clean22(), "vocab", "vocab", "fs::read|a.b");
        refused(&r, "GATE 22'S BANNED LIST HOLDS `a.b`");
    }

    #[test]
    fn gate22_refuses_a_filesystem_or_store_reach_by_text_and_by_path() {
        let f = clean22().file("crates/engine/src/x.rs", "let b = std::fs::read(p);\n");
        refused(
            &g22(&f),
            "crates/engine/src/x.rs:1:let b = std::fs::read(p);",
        );
        let f = clean22().bytes("crates/engine/benches/blob.dat", b"\0 File::open(x)\n");
        refused(
            &g22(&f),
            "REFUSED  crates/engine/benches reaches the filesystem:",
        );
        let f = clean22().file(
            "crates/engine/src/x.rs",
            "pub use store::file::BarFile as Bf;\n",
        );
        refused(
            &g22(&f),
            "REFUSED  crates/engine imports a crate that reads the store:",
        );
        let f = clean22().scan(
            "paths crates/vocab/src/lib.rs",
            0,
            "crates/vocab/src/lib.rs:2:std::process::exit\ncrates/vocab/src/lib.rs:7:std::fs::read\n",
        );
        refused(
            &g22(&f),
            "reaches the filesystem or a process, read as a path:",
        );
        let f = clean22().scan(
            "paths crates/engine/src/lib.rs",
            0,
            "crates/engine/src/lib.rs:4:store::file::BarFile\n",
        );
        refused(
            &g22(&f),
            "names a crate that reads the store, read as a path:",
        );
    }

    #[test]
    fn gate22_refuses_an_include_that_is_computed_bar_shaped_or_not_a_document() {
        let inc = |p: &str| {
            clean22().file(
                "crates/engine/src/x.rs",
                &format!("const X: &str = include_str!({p});\n"),
            )
        };
        refused(
            &g22(&inc("concat!(env!(\"HOME\"), \"/x.bin\")")),
            "computes an include path",
        );
        let r = g22(&inc("\"../data/bars.csv\""));
        refused(
            &r,
            "includes '../data/bars.csv', not a document or a source file",
        );
        refused(&r, "includes '../data/bars.csv', which is bar-shaped");
        refused(
            &g22(&inc("\"bars.md\"")),
            "includes 'bars.md', which is bar-shaped",
        );
        refused(&g22(&inc("\"x .md\"")), "includes 'x', not a document");
        refused(
            &g22(&inc("\"*.md\"")),
            "includes '*.md', which a shell would expand",
        );
        let f = clean22().file(
            "crates/vocab/tests/t.rs",
            "const X: &str = include_str!(\"a.bin\");\n",
        );
        refused(&g22(&f), "REFUSED  crates/vocab/tests includes 'a.bin'");
    }

    #[test]
    fn gate22_refuses_a_test_that_reaches_outside_the_repository() {
        let f = clean22().file(
            "crates/vocab/tests/t.rs",
            "let h = std::env::var(\"HOME\");\n",
        );
        refused(
            &g22(&f),
            "REFUSED  crates/vocab/tests names a path outside the repository:",
        );
        let f = clean22().file("crates/vocab/tests/t.rs", "let p = \"x.parquet\";\n");
        refused(&g22(&f), "crates/vocab/tests/t.rs:1:");
    }

    #[test]
    fn gate22_refuses_what_it_cannot_read() {
        let r = g22_with(&clean22(), "vocab", "vocab runner", BANNED22);
        refused(&r, "crates/runner/src/lib.rs is not tracked");
        let f = clean22().scan(
            "closure LISTING crates/vocab/src/lib.rs",
            1,
            "UNRESOLVED x\n",
        );
        refused(
            &g22(&f),
            "REFUSED  crates/vocab: the module closure did not resolve:",
        );
        let f = clean22().scan("paths crates/engine/src/lib.rs", 2, "");
        refused(
            &g22(&f),
            "REFUSED  crates/engine: a source file could not be lexed",
        );
        let r = g22_with(&Fake::default(), "", "", BANNED22);
        refused(&r, "GATE 22 READ NO TRACKED FILE.");
    }

    // ---------------------------------------------------------- gate 24 --

    #[test]
    fn the_writable_map_spellings_match_their_expression() {
        for l in [
            "MmapMut",
            "x.map_mut(",
            "map_anon(",
            "y.map_anon_mut(",
            "map_copy(",
            "MmapRaw",
            "\u{e9} map_anon(",
        ] {
            assert!(maps_writably(l.as_bytes()), "{l}");
        }
        for l in [
            "remap_mut(",
            "map_copy_read_only(",
            "_map_copy(",
            "Mmap::map(&f)",
            "map_anon",
        ] {
            assert!(!maps_writably(l.as_bytes()), "{l}");
        }
    }

    #[test]
    fn gate24_passes_a_read_only_map_and_refuses_a_writable_one() {
        let f = Fake::default().file("crates/store/src/lib.rs", "pub fn f(m: memmap2::Mmap) {}\n");
        passed(
            &ran(gate24(&f, &opts(&[]))),
            "OK — no crate maps a file or an anonymous region writably.",
        );
        let f = f.file(
            "crates/store/src/x.rs",
            "a\n*slot = Some(MmapOptions::new().map_anon()?);\r\n",
        );
        let r = ran(gate24(&f, &opts(&[])));
        refused(
            &r,
            "crates/store/src/x.rs:2:*slot = Some(MmapOptions::new().map_anon()?);\r",
        );
        let f = Fake::default().file("crates/store/src/lib.rs", "x").scan(
            "code crates/store/src/lib.rs",
            2,
            "",
        );
        refused(
            &ran(gate24(&f, &opts(&[]))),
            "crates/store/src/lib.rs: (unlexable)",
        );
    }
}

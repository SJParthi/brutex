//! Gates 25, 27, 27b, 26, 19, 10 and 11 of the language-purity job, read in
//! Rust. This CI-only executable has no dependencies, is built directly by
//! rustc, and proves itself with `rustc --test` before any gate trusts it.
//!
//! These seven gates used to be workflow steps whose logic was written in
//! inline text-processing programs: line patterns, pipelines of filters, a
//! test-module stripper and four state machines. That is a second language in
//! a tracked file outside `web/`, which CLAUDE.md section 2 forbids, so the
//! logic moved here and each step's `run:` only builds this file, runs its
//! tests and names one subcommand (D-2313). The reasons each gate exists are
//! still written beside its step in `.github/workflows/ci.yml`; the reasons
//! for each mechanical choice, which used to sit beside the shell that made
//! it, sit beside the Rust that makes it now.
//!
//! FIDELITY IS THE RULE. Every subcommand answers exactly what its step
//! answered: the same files walked, the same lines matched, the same counts
//! and the same refusal text. Where the old step had a quirk that let more
//! through, this tool keeps the stricter reading and says so at the site:
//! it never passes a tree the old step refused.
//!
//! Subcommands, one per gate, all run from the repository root:
//!
//! * `release-profile SCANNER`              gate 25
//! * `invariant-ids`                        gate 27
//! * `decision-numbers`                     gate 27b
//! * `tls-provider`                         gate 26
//! * `failure-events PROD STATUS`         gate 19
//! * `invariant-tests SCANNER PATHS_TOOL`   gate 10
//! * `banned-constructs PROD STATUS`      gate 11
//!
//! `SCANNER` is the `.github/source_scan.rs` binary gate 0 builds, and
//! `PATHS_TOOL` is the `.github/invariant_paths.rs` binary gate 10 builds.
//! `PROD` is the output of `source_scan prod-files` and `STATUS` its exit
//! status (D-1938).

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::io::Write as _;
use std::process::{Command, ExitCode, Stdio};

// ------------------------------------------------------------- output --

/// Every line a gate prints. In CI it is echoed as it is said, so a
/// subprocess's own output interleaves in order; in tests it is only kept.
struct Out {
    echo: bool,
    to_stderr: bool,
    text: String,
}

impl Out {
    fn new(echo: bool) -> Self {
        Self {
            echo,
            to_stderr: false,
            text: String::new(),
        }
    }

    /// For a filter whose stdout is its data: diagnostics go to stderr.
    fn stderr() -> Self {
        Self {
            echo: true,
            to_stderr: true,
            text: String::new(),
        }
    }

    fn say(&mut self, line: &str) {
        self.text.push_str(line);
        self.text.push('\n');
        if self.echo {
            let mut o: Box<dyn std::io::Write> = if self.to_stderr {
                Box::new(std::io::stderr().lock())
            } else {
                Box::new(std::io::stdout().lock())
            };
            let _ = o.write_all(line.as_bytes());
            let _ = o.write_all(b"\n");
            let _ = o.flush();
        }
    }
}

macro_rules! say {
    ($out:expr, $($arg:tt)*) => { $out.say(&format!($($arg)*)) };
}

// --------------------------------------------------------------- tree --

/// What a gate may read: the tracked list, in `git ls-files` order, and
/// file bytes. The real tree reads git and the disk; tests use a map.
trait Tree {
    fn tracked(&self) -> &[String];
    fn read(&self, path: &str) -> Option<Vec<u8>>;
    fn is_file(&self, path: &str) -> bool;
}

struct Disk {
    tracked: Vec<String>,
}

impl Disk {
    /// `git ls-files -z`: NUL-separated, so a path with a space, a quote or
    /// a non-ASCII byte is read as itself rather than as git's quoted form.
    fn load() -> Result<Self, String> {
        let o = Command::new("git")
            .args(["ls-files", "-z"])
            .stderr(Stdio::inherit())
            .output()
            .map_err(|e| format!("could not run git ls-files: {e}"))?;
        if !o.status.success() {
            return Err(format!("git ls-files failed: {}", o.status));
        }
        let tracked = String::from_utf8_lossy(&o.stdout)
            .split('\0')
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .collect();
        Ok(Self { tracked })
    }
}

impl Tree for Disk {
    fn tracked(&self) -> &[String] {
        &self.tracked
    }
    fn read(&self, path: &str) -> Option<Vec<u8>> {
        std::fs::read(path).ok()
    }
    fn is_file(&self, path: &str) -> bool {
        std::path::Path::new(path).is_file()
    }
}

/// A git pathspec with no magic: `*` matches any run of characters, `/`
/// included, and `?` one character. Every pathspec these gates used has a
/// `*`, so git's literal-prefix rule for a wildcard-free spec never applies.
fn glob(pattern: &str, path: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let s: Vec<char> = path.chars().collect();
    let (mut i, mut j) = (0, 0);
    let mut star: Option<(usize, usize)> = None;
    while j < s.len() {
        if i < p.len() && (p[i] == '?' || p[i] == s[j]) && p[i] != '*' {
            i += 1;
            j += 1;
        } else if i < p.len() && p[i] == '*' {
            star = Some((i, j));
            i += 1;
        } else if let Some((si, sj)) = star {
            i = si + 1;
            j = sj + 1;
            star = Some((si, sj + 1));
        } else {
            return false;
        }
    }
    p[i..].iter().all(|c| *c == '*')
}

/// The tracked paths any of `patterns` names, in tracked order, each once.
fn listed(tree: &dyn Tree, patterns: &[&str]) -> Vec<String> {
    tree.tracked()
        .iter()
        .filter(|f| patterns.iter().any(|p| glob(p, f)))
        .cloned()
        .collect()
}

fn text_of(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// A file's lines as a line-oriented reader sees them: split on `\n` only,
/// so a `\r` stays part of its line, and a last line with no newline is
/// still a line. `wc -l` counted newlines and reported a one-line file with
/// no trailing newline as ZERO lines, which once let gate 11 skip it whole.
fn records(text: &str) -> Vec<&str> {
    let mut v: Vec<&str> = text.split('\n').collect();
    if v.last() == Some(&"") {
        v.pop();
    }
    v
}

fn ltrim(s: &str) -> &str {
    s.trim_start_matches([' ', '\t'])
}

/// The first `n` bytes of a line, as `cut -c1-110` printed it.
fn cut(line: &str, n: usize) -> String {
    let b = line.as_bytes();
    text_of(&b[..b.len().min(n)])
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn is_ascii_word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// `[[:space:]]`.
fn is_space(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\x0b' | '\x0c' | '\r')
}

/// The character before byte offset `at`, if any.
fn before(s: &str, at: usize) -> Option<char> {
    s[..at].chars().next_back()
}

// ------------------------------------------------------------ gate 25 --

/// Pieces, so this file never spells the whole environment key or codegen
/// flag gate 25 refuses in a workflow.
const ENV_HEAD: &str = concat!("CARGO_PRO", "FILE_");
const ENV_TAIL: &str = concat!("_OVER", "FLOW_CHECKS");
const FLAG_KEY: &str = concat!("over", "flow-checks");
/// The flag's two words, which rustc joins with `-` or `_` (D-2323).
const FLAG_HEAD: &str = concat!("over", "flow");
const FLAG_TAIL: &str = "checks";

/// `^FILE:[0-9]*:REST$` with FILE given: the REST of a scanner leaf.
fn leaf_of<'a>(line: &'a str, file: &str) -> Option<&'a str> {
    let rest = line.strip_prefix(file)?.strip_prefix(':')?;
    let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
    rest[digits..].strip_prefix(':')
}

/// `^[^:]+:[0-9]+:REST`: any manifest's leaf.
fn any_leaf(line: &str) -> Option<&str> {
    let (file, rest) = line.split_once(':')?;
    if file.is_empty() {
        return None;
    }
    let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
    if digits == 0 {
        return None;
    }
    rest[digits..].strip_prefix(':')
}

/// `profile\.(.*\.)?overflow-checks = ` at the start of a leaf: the key
/// directly under `profile.`, or under any dotted path below it, which is
/// how a `[profile.release.package.engine]` override prints.
fn turns_overflow_key(leaf: &str) -> bool {
    let Some(tail) = leaf.strip_prefix("profile.") else {
        return false;
    };
    let key = format!("{FLAG_KEY} = ");
    tail.match_indices(&key)
        .any(|(at, _)| at == 0 || tail.as_bytes()[at - 1] == b'.')
}

/// The environment key: the head, then `[A-Z0-9_]*`, then the tail.
fn env_override(line: &str) -> bool {
    line.match_indices(ENV_HEAD).any(|(at, _)| {
        let run: String = line[at + ENV_HEAD.len()..]
            .chars()
            .take_while(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || *c == '_')
            .collect();
        run.contains(ENV_TAIL)
    })
}

/// The codegen flag set to `off`, `no`, `n`, `false` or `0`, spaces allowed
/// around the `=` and one quote allowed before the value. As before it is a
/// prefix test: `n` covers `no`. The two words of the key are joined by a
/// hyphen OR an underscore (P15-09, D-2323): rustc normalises `-C` option
/// names, so `-C over..._checks=off` is the same option, comes after cargo's
/// own `-C ...=on`, and wraps. The key is found anywhere in the line, so the
/// `-C` written with no space before it is read too.
fn flag_override(line: &str) -> bool {
    line.match_indices(FLAG_HEAD).any(|(at, _)| {
        let Some(rest) = line[at + FLAG_HEAD.len()..]
            .strip_prefix(['-', '_'])
            .and_then(|r| r.strip_prefix(FLAG_TAIL))
        else {
            return false;
        };
        let rest = rest.trim_start_matches(is_space);
        let Some(rest) = rest.strip_prefix('=') else {
            return false;
        };
        let rest = rest.trim_start_matches(is_space);
        let rest = rest.strip_prefix(['"', '\'']).unwrap_or(rest);
        ["off", "n", "false", "0"]
            .iter()
            .any(|v| rest.starts_with(v))
    })
}

/// `(saturating|checked|wrapping)_[a-z_]+`, counted the way `grep -o`
/// counts: leftmost, longest, never overlapping.
fn census(line: &str) -> usize {
    let b = line.as_bytes();
    let (mut i, mut n) = (0, 0);
    'scan: while i < b.len() {
        for w in ["saturating_", "checked_", "wrapping_"] {
            if b[i..].starts_with(w.as_bytes()) {
                let start = i + w.len();
                let end = start
                    + b[start..]
                        .iter()
                        .take_while(|c| c.is_ascii_lowercase() || **c == b'_')
                        .count();
                if end > start {
                    n += 1;
                    i = end;
                    continue 'scan;
                }
            }
        }
        i += 1;
    }
    n
}

/// Gate 25. `leaves` is what `source_scan toml` printed for every tracked
/// `*Cargo.toml`, `FILE:LINE:dotted.path = value` per leaf. The step runs the
/// scanner, because this tool starts no program but git (gate 0's `spawns`
/// rule), and a manifest the scanner cannot read fails the step there, with
/// the scanner's own message naming the file. Only leaves of a file named
/// exactly `Cargo.toml` are read, the manifests the old step scanned.
fn gate25(tree: &dyn Tree, leaves: &str, out: &mut Out) -> bool {
    if !tree.is_file("Cargo.toml") {
        out.say("GATE 25: no root Cargo.toml");
        return false;
    }
    let manifests = listed(tree, &["*Cargo.toml"]);
    if manifests.is_empty() {
        out.say("GATE 25 READ NO MANIFEST.");
        return false;
    }
    let mut bad = false;
    let leaves: Vec<&str> = records(leaves)
        .into_iter()
        .filter(|l| {
            l.split(':')
                .next()
                .is_some_and(|f| f.rsplit('/').next() == Some("Cargo.toml"))
        })
        .collect();
    let root = |want: &str| {
        leaves
            .iter()
            .any(|l| leaf_of(l, "Cargo.toml") == Some(want))
    };
    if !root("profile.release = {table}") {
        out.say("  REFUSED  Cargo.toml has no [profile.release] table at all");
        bad = true;
    }
    if root(&format!("profile.release.{FLAG_KEY} = true")) {
        say!(out, "  present  [profile.release] {FLAG_KEY} = true");
    } else {
        say!(
            out,
            "  REFUSED  [profile.release] no longer sets {FLAG_KEY} = true."
        );
        out.say("           release would WRAP where dev panics, and every");
        out.say("           checked_/saturating_ site in the workspace was");
        out.say("           written believing the opposite.");
        bad = true;
    }
    if root("profile.release.panic = \"abort\"") {
        out.say("  present  [profile.release] panic = \"abort\"");
    } else {
        out.say("  REFUSED  [profile.release] no longer sets panic = \"abort\".");
        out.say("           That is a decision, and docs/05-decisions.md is");
        out.say("           where a decision goes.");
        bad = true;
    }
    let off: Vec<&&str> = leaves
        .iter()
        .filter(|l| any_leaf(l).is_some_and(turns_overflow_key) && !l.ends_with(" = true"))
        .collect();
    if off.is_empty() {
        say!(
            out,
            "  present  no profile in any tracked manifest sets {FLAG_KEY} off"
        );
    } else {
        out.say("  REFUSED  a profile switches overflow checking back off:");
        for l in off {
            out.say(l);
        }
        bad = true;
    }
    let mut envs = Vec::new();
    // Every tracked `.yml` under `.github/` too, composite actions included,
    // as gate 0 and gate 1g read them (D-2341, D-2323).
    for w in listed(tree, &[".github/workflows/*", ".github/*.yml"]) {
        let Some(bytes) = tree.read(&w) else {
            say!(out, "  REFUSED  {w} could not be read");
            bad = true;
            continue;
        };
        let text = text_of(&bytes);
        for (n, line) in records(&text).iter().enumerate() {
            if env_override(line) || flag_override(line) {
                envs.push(format!("{w}:{}:{line}", n + 1));
            }
        }
    }
    if envs.is_empty() {
        out.say("  present  no workflow overrides overflow checking from the environment");
    } else {
        out.say("  REFUSED  a workflow overrides overflow checking from the environment:");
        for e in &envs {
            out.say(e);
        }
        bad = true;
    }
    // The census is REPORTING, not a rule: the size of what the two keys
    // protect, so a reader can see the number move.
    let sat: usize = listed(tree, &["crates/**/*.rs"])
        .iter()
        .filter_map(|f| tree.read(f))
        .map(|b| {
            records(&text_of(&b))
                .iter()
                .map(|l| census(l))
                .sum::<usize>()
        })
        .sum();
    say!(
        out,
        "  context  {sat} explicit saturating_/checked_/wrapping_ site(s) in tracked sources"
    );
    if bad {
        out.say("");
        out.say("GATE 25 FAILED.");
        out.say("");
        out.say("Silent wrapping is not an acceptable answer to an arithmetic");
        out.say("mistake in a backtesting engine. CLAUDE.md section 4.");
        return false;
    }
    out.say("OK — the release profile still panics on overflow and aborts on panic.");
    true
}

// ------------------------------------------------------------ gate 27 --

const INVARIANTS: &str = "docs/04-invariants.md";
const DECISIONS: &str = "docs/05-decisions.md";

/// `[A-Z][A-Z0-9]*(-[A-Za-z0-9]+)+`, the whole token. WIDENED BY D-2667
/// (P6-04), carried here by D-1936: the old `[A-Z][A-Z0-9-]*-[0-9]{2,3}[a-z]?`
/// required a two- or three-digit tail, so ids with a letter suffix after a
/// hyphen or a longer tail (`S-30-session`, `RUST-UC7-a`, `AF-1203-a`,
/// `CU-SV4-CLOSE-D0961`) were invisible to uniqueness, the blind spot D-1608
/// closed once for 179 others. Any hyphenated upper-case id counts; header
/// words (`ID`, `Invariant`, `Point`) have no hyphen.
fn id_grammar(tok: &str) -> bool {
    let Some((head, tail)) = tok.split_once('-') else {
        return false;
    };
    let mut h = head.chars();
    h.next().is_some_and(|c| c.is_ascii_uppercase())
        && h.all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
        && tail
            .split('-')
            .all(|g| !g.is_empty() && g.chars().all(|c| c.is_ascii_alphanumeric()))
}

/// The id of an invariant row: `^\| *`?ID`? *\|`, one per line at most.
fn row_id(line: &str) -> Option<&str> {
    let rest = line.strip_prefix('|')?.trim_start_matches(' ');
    let rest = rest.strip_prefix('`').unwrap_or(rest);
    let end = rest
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-'))
        .unwrap_or(rest.len());
    let (tok, after) = rest.split_at(end);
    if !id_grammar(tok) {
        return None;
    }
    let after = after.strip_prefix('`').unwrap_or(after);
    after
        .trim_start_matches(' ')
        .starts_with('|')
        .then_some(tok)
}

/// Gate 27: every invariant row id names exactly one invariant.
fn gate27(tree: &dyn Tree, out: &mut Out) -> bool {
    let Some(bytes) = tree.read(INVARIANTS) else {
        say!(out, "GATE 27 could not read {INVARIANTS}.");
        return false;
    };
    let text = text_of(&bytes);
    let lines = records(&text);
    let ids: Vec<&str> = lines.iter().filter_map(|l| row_id(l)).collect();
    let n = ids.len();
    // A pattern that stops matching is the failure mode gate 8 shipped
    // with: it reported success while measuring nothing.
    if n == 0 {
        out.say("GATE 27 FOUND NO ROWS. Either the document moved or the pattern");
        out.say("stopped matching. A silent zero is not a pass.");
        return false;
    }
    let mut count: BTreeMap<&str, usize> = BTreeMap::new();
    for id in &ids {
        *count.entry(id).or_default() += 1;
    }
    let dupes: Vec<&str> = count
        .iter()
        .filter(|(_, c)| **c > 1)
        .map(|(d, _)| *d)
        .collect();
    say!(out, "read {n} invariant row id(s) in {INVARIANTS}");
    if !dupes.is_empty() {
        out.say("");
        out.say("AN ID NAMES MORE THAN ONE INVARIANT:");
        for d in dupes {
            say!(out, "  {d}");
            for (i, l) in lines.iter().enumerate() {
                if row_id(l) == Some(d) {
                    say!(out, "      {}", cut(&format!("{}:{l}", i + 1), 110));
                }
            }
        }
        out.say("");
        out.say("A citation to a duplicated id resolves to whichever row comes");
        out.say("first, which is a coincidence and not a citation. Renumber the");
        out.say("NEWER family onto free ids by the evidence rule D-0045 set and");
        out.say("D-0215 reapplied: whichever side is cited from OUTSIDE this");
        out.say("document keeps the letter. C-18..C-25 stay retired.");
        return false;
    }
    say!(out, "OK — {n} rows, all ids unique.");
    true
}

// ----------------------------------------------------------- gate 27b --

/// The five numbers issued twice before the ledger was read back (D-0104,
/// D-0684). Entries are never edited, so each stays, pinned at its EXACT
/// count: a third copy fails, and so does a removed one.
const DECISION_PINS: &[(&str, usize)] = &[
    ("D-0076", 2),
    ("D-0077", 2),
    ("D-0078", 2),
    ("D-0370", 2),
    ("D-0372", 2),
];

/// `^#{2,4} +D-[0-9]{4}`: the number a heading carries, its first four
/// digits only, as the old extraction kept them.
fn heading_number(line: &str) -> Option<&str> {
    let hashes = line.bytes().take_while(|b| *b == b'#').count();
    if !(2..=4).contains(&hashes) {
        return None;
    }
    let rest = &line[hashes..];
    let body = rest.trim_start_matches(' ');
    if body.len() == rest.len() || !body.starts_with("D-") {
        return None;
    }
    let d = body.get(2..6)?;
    d.bytes().all(|b| b.is_ascii_digit()).then(|| &body[..6])
}

/// `^#{2,4} +NUMBER([^0-9]|$)`: the lines printed under a refused number.
fn heads(line: &str, number: &str) -> bool {
    let hashes = line.bytes().take_while(|b| *b == b'#').count();
    if !(2..=4).contains(&hashes) {
        return false;
    }
    let rest = &line[hashes..];
    let body = rest.trim_start_matches(' ');
    body.len() != rest.len()
        && body
            .strip_prefix(number)
            .is_some_and(|after| !after.starts_with(|c: char| c.is_ascii_digit()))
}

/// Gate 27b: every decision number heads exactly one entry, the pins aside.
fn gate27b(tree: &dyn Tree, pins: &[(&str, usize)], out: &mut Out) -> bool {
    let Some(bytes) = tree.read(DECISIONS) else {
        say!(out, "GATE 27B could not read {DECISIONS}.");
        return false;
    };
    let text = text_of(&bytes);
    let lines = records(&text);
    let nums: Vec<&str> = lines.iter().filter_map(|l| heading_number(l)).collect();
    let n = nums.len();
    if n == 0 {
        out.say("GATE 27B FOUND NO DECISION HEADINGS. Either the document moved");
        out.say("or the pattern stopped matching. A silent zero is not a pass.");
        return false;
    }
    let mut seen: BTreeMap<&str, usize> = BTreeMap::new();
    for d in &nums {
        *seen.entry(d).or_default() += 1;
    }
    let want = |d: &str| pins.iter().find(|(p, _)| *p == d).map_or(1, |(_, w)| *w);
    let mut bad: Vec<(String, usize, usize)> = seen
        .iter()
        .filter(|(d, c)| **c != want(d))
        .map(|(d, c)| ((*d).to_owned(), *c, want(d)))
        .collect();
    for (d, w) in pins {
        if !seen.contains_key(d) {
            bad.push(((*d).to_owned(), 0, *w));
        }
    }
    bad.sort();
    say!(out, "read {n} decision heading(s) in {DECISIONS}");
    if !bad.is_empty() {
        out.say("");
        out.say("A DECISION NUMBER HEADS THE WRONG NUMBER OF ENTRIES:");
        for (d, c, w) in &bad {
            say!(out, "  {d}: {c} heading(s), allowed {w}");
            for (i, l) in lines.iter().enumerate() {
                if heads(l, d) {
                    say!(out, "      {}", cut(&format!("{}:{l}", i + 1), 110));
                }
            }
        }
        out.say("");
        out.say("Never renumber or delete an entry. Give the NEWER one the next");
        out.say("free number, max(existing) + 1 read from the merged tree. D-0684.");
        return false;
    }
    say!(
        out,
        "OK — {n} headings; every number heads one entry, the five pinned by D-0684 aside."
    );
    true
}

// ------------------------------------------------------------ gate 26 --

/// Lines of `text` containing `needle`, as `grep -c` counts them.
fn lines_with(text: &str, needle: &str) -> usize {
    records(text).iter().filter(|l| l.contains(needle)).count()
}

/// Gate 26: every file under `crates/` that builds a client calls
/// `ensure_tls_provider()` at least as often. Per file, by occurrence count:
/// it can miss a helper in another file, it cannot falsely accuse.
fn gate26(tree: &dyn Tree, out: &mut Out) -> bool {
    let (mut bad, mut sites, mut files) = (false, 0, 0);
    for f in listed(tree, &["crates/*.rs"]) {
        let Some(bytes) = tree.read(&f) else {
            say!(out, "UNREADABLE  {f}");
            bad = true;
            continue;
        };
        let text = text_of(&bytes);
        let n = lines_with(&text, "Client::builder()");
        if n == 0 {
            continue;
        }
        files += 1;
        sites += n;
        let g = lines_with(&text, "ensure_tls_provider()");
        if g < n {
            say!(
                out,
                "UNGUARDED CLIENT  {f} — {n} Client::builder() call(s), {g} ensure_tls_provider()"
            );
            bad = true;
        } else {
            say!(out, "  ok  {f} — {n} site(s), {g} guard(s)");
        }
    }
    say!(
        out,
        "checked {sites} client construction site(s) in {files} file(s)"
    );
    if sites == 0 {
        out.say("GATE 26 FOUND NO CLIENT SITES. Either reqwest left the workspace");
        out.say("or the pattern stopped matching. A silent zero is not a pass.");
        return false;
    }
    if bad {
        out.say("");
        out.say("Call pull::ensure_tls_provider() before building the client.");
        out.say("See docs/05-decisions.md D-0211.");
        return false;
    }
    out.say("OK — every client construction is guarded.");
    true
}

// ------------------------------------------- the test-module stripper --

/// One kept line of a source file, stamped with its original line number.
struct Line {
    no: usize,
    text: String,
}

/// A file with its `#[cfg(test)] mod NAME { .. }` blocks subtracted.
/// `scope` keeps comments; `corpus` drops whole-line `//` comments.
struct Stripped {
    scope: Vec<Line>,
    corpus: Vec<Line>,
    dropped: usize,
    total: usize,
}

/// `pub`, `pub(...)` or `pub (...)` and the blanks after it, if `s` starts
/// with one: `pub([ \t]*\([^)]*\))?[ \t]+`.
fn after_pub(s: &str) -> Option<&str> {
    let r = s.strip_prefix("pub")?;
    let blanks = |x: &str| x.len() - ltrim(x).len();
    let p = ltrim(r);
    if let Some(inner) = p.strip_prefix('(')
        && let Some(close) = inner.find(')')
    {
        let after = &inner[close + 1..];
        if blanks(after) > 0 {
            return Some(ltrim(after));
        }
    }
    (blanks(r) > 0).then(|| ltrim(r))
}

/// `^[ \t]*(pub...)?mod[ \t]+[A-Za-z0-9_]+[ \t]*\{[ \t]*$`. One-line
/// `#[cfg(test)] mod tests {` is NOT this shape and its body is scanned:
/// loud, never silent.
fn opens_inline_mod(line: &str) -> bool {
    let s = ltrim(line);
    let s = after_pub(s).unwrap_or(s);
    let Some(r) = s.strip_prefix("mod") else {
        return false;
    };
    let name = ltrim(r);
    if name.len() == r.len() {
        return false;
    }
    let end = name.find(|c: char| !is_ascii_word(c)).unwrap_or(name.len());
    if end == 0 {
        return false;
    }
    let tail = ltrim(&name[end..]);
    tail.strip_prefix('{').is_some_and(|t| ltrim(t).is_empty())
}

fn brackets(line: &str) -> i64 {
    let open = line.matches('[').count();
    let close = line.matches(']').count();
    i64::try_from(open).unwrap_or(i64::MAX) - i64::try_from(close).unwrap_or(i64::MAX)
}

/// The subtraction gates 11 and 19 share. From a `#[cfg(test)]` line, blank
/// lines, `//` comments and attributes are held to find the item it covers.
/// If that item is an inline `mod NAME {`, everything through the matching
/// `}` at the same indent is dropped; otherwise the held lines and the item
/// are kept and the walk carries on. A file may hold any number of test
/// modules in any position and none of them hides a line after it.
///
/// `None` is a REFUSAL: a module still open at end of file, an attribute
/// with no item under it, or a line of code at or above the module's indent
/// before its closing brace. Matching a later brace at the same indent would
/// swallow whatever lies between, silently; "I could not tell where this
/// module ends" is a thing a gate may say, quietly scanning half of it is
/// not. Blank and `//` lines are exempt from the indent check: neither can
/// hide a construct.
///
/// The patterns read `\t` and space as blanks and nothing else, so a CRLF
/// file's `#[cfg(test)]\r` is not a test attribute and is scanned whole.
fn strip(text: &str) -> Option<Stripped> {
    let lines = records(text);
    let mut s = Stripped {
        scope: Vec::new(),
        corpus: Vec::new(),
        dropped: 0,
        total: lines.len(),
    };
    let emit = |s: &mut Stripped, no: usize, line: &str| {
        s.scope.push(Line {
            no,
            text: line.to_owned(),
        });
        if !ltrim(line).starts_with("//") {
            s.corpus.push(Line {
                no,
                text: line.to_owned(),
            });
        }
    };
    let mut st = 0u8;
    let mut depth: i64 = 0;
    let mut held: Vec<(usize, &str)> = Vec::new();
    let mut ind = String::new();
    for (i, line) in lines.iter().enumerate() {
        let no = i + 1;
        let t = ltrim(line);
        if st == 2 {
            s.dropped += 1;
            let lead = line.len() - t.len();
            if line.strip_prefix(ind.as_str()).is_some_and(|r| {
                r.strip_prefix('}')
                    .is_some_and(|after| after.chars().all(|c| c == ' ' || c == '\t'))
            }) {
                st = 0;
                continue;
            }
            if !t.is_empty() && !t.starts_with("//") && lead <= ind.len() {
                return None;
            }
            continue;
        }
        if st == 0 {
            let is_cfg = t
                .strip_prefix("#[cfg(test)]")
                .is_some_and(|r| r.chars().all(|c| c == ' ' || c == '\t'));
            if is_cfg {
                st = 1;
                depth = 0;
                held.clear();
                held.push((no, line));
            } else {
                emit(&mut s, no, line);
            }
            continue;
        }
        if depth > 0 {
            depth += brackets(line);
            held.push((no, line));
            continue;
        }
        if t.is_empty() || t.starts_with("//") {
            held.push((no, line));
            continue;
        }
        if t.starts_with("#[") {
            depth = brackets(line);
            held.push((no, line));
            continue;
        }
        if opens_inline_mod(line) {
            ind = line[..line.len() - t.len()].to_owned();
            s.dropped += held.len() + 1;
            held.clear();
            st = 2;
            continue;
        }
        for (n, l) in held.drain(..) {
            emit(&mut s, n, l);
        }
        emit(&mut s, no, line);
        st = 0;
    }
    (st == 0).then_some(s)
}

/// THE FILE LIST IS THE COMPILER'S PRODUCTION CLOSURE (P1-08-04, D-2660,
/// carried here by D-1938). Gates 11 and 19 read a `/src/` glob with a
/// "whole-file test module" exempted by its file STEM, so a production
/// `resume/manifest.rs` sharing a stem with a crate-root
/// `#[cfg(test)] mod manifest;` was skipped, and `crates/cli/commit_stamp.rs`,
/// mounted by `#[path]` from outside `src/` and compiled into every `cli`
/// build, was never read. `source_scan prod-files` follows every `mod`,
/// `#[path]` and `include!` from each crate's lib/main/bin root, skips what
/// sits under `#[cfg(test)]` or `#[cfg(all(test, ..))]`, and refuses what it
/// cannot resolve; the step hands its output and exit status here. A scanner
/// that failed, or a closure with no file, is a refusal.
fn prod_list(raw: &str, status: &str, out: &mut Out) -> Option<Vec<String>> {
    let ok = status.trim() == "0";
    let lines = records(raw);
    if !ok {
        out.say("REFUSED  the production closure could not be resolved:");
        for l in lines.iter().filter(|l| l.starts_with("UNRESOLVED ")) {
            say!(out, "  {l}");
        }
        if status.trim().parse::<i32>().is_err() {
            say!(out, "  `{}` is not an exit status", status.trim());
        }
        return None;
    }
    let files: Vec<String> = lines
        .into_iter()
        .filter(|l| !l.is_empty() && !l.starts_with("UNRESOLVED "))
        .map(str::to_owned)
        .collect();
    if files.is_empty() {
        out.say("READ NO PRODUCTION FILE.");
        return None;
    }
    Some(files)
}

/// A file whose first line is `#![cfg(test)]` is compiled out of every
/// non-test build. The production closure already leaves such a file out;
/// this is the same reading, kept so a list handed in by hand cannot put one
/// back.
fn compiled_out(tree: &dyn Tree, path: &str) -> bool {
    tree.read(path)
        .is_some_and(|b| text_of(&b).split('\n').next() == Some("#![cfg(test)]"))
}

/// The walk gates 11 and 19 share: strip each file, refuse the ones that
/// cannot be delimited, total what was kept and dropped.
struct Walk {
    files: Vec<(String, Stripped)>,
    walked: usize,
    kept: usize,
    dropped: usize,
    undelimited: usize,
}

fn walk(tree: &dyn Tree, candidates: &[String], out: &mut Out) -> Walk {
    let mut w = Walk {
        files: Vec::new(),
        walked: 0,
        kept: 0,
        dropped: 0,
        undelimited: 0,
    };
    for f in candidates {
        if compiled_out(tree, f) {
            continue;
        }
        w.walked += 1;
        // NOTHING IS KEPT UNTIL THE FILE PARSES. A partial corpus from a
        // file the stripper could not delimit would be a scan that quietly
        // covered less than it reported.
        let stripped = tree.read(f).and_then(|b| strip(&text_of(&b)));
        match stripped {
            Some(s) => {
                w.dropped += s.dropped;
                w.kept += s.total - s.dropped;
                w.files.push((f.clone(), s));
            }
            None => {
                say!(out, "UNDELIMITED TEST MODULE  {f}");
                w.undelimited += 1;
            }
        }
    }
    w
}

const UNDELIMITED: [&str; 3] = [
    "A #[cfg(test)] module above has no closing brace at the same",
    "indent as its own `mod` keyword, so this scanner cannot tell",
    "where it ends and will not guess. `cargo fmt` does not emit",
];

// ------------------------------------------------------------ gate 19 --

/// The lines an emit may sit above a failure, in ORIGINAL source lines: a
/// comment block or a test module between them still costs its height.
const WINDOW: usize = 12;

/// `note_[a-z_]+\(` at the start of `r`.
fn note_call_at(r: &str) -> bool {
    let Some(after) = r.strip_prefix("note_") else {
        return false;
    };
    let run = after
        .bytes()
        .take_while(|b| b.is_ascii_lowercase() || *b == b'_')
        .count();
    run > 0 && after[run..].starts_with('(')
}

/// `note_[a-z_]+\(` anywhere in `t`.
fn calls_note(t: &str) -> bool {
    t.match_indices("note_")
        .any(|(at, _)| note_call_at(&t[at..]))
}

/// `^[[:space:]]*fn[[:space:]]+note_[a-z_]+\(`: a helper's own signature,
/// which is not a call. Counting it would let a file satisfy itself by
/// declaring helpers. `pub fn note_x(` is not this shape, exactly as before,
/// and so still counts as an event.
fn defines_note(t: &str) -> bool {
    let Some(r) = t.trim_start_matches(is_space).strip_prefix("fn") else {
        return false;
    };
    let rest = r.trim_start_matches(is_space);
    rest.len() != r.len() && note_call_at(rest)
}

/// The failure sites of one file with no event of their own. Each event is
/// claimed at most once, nearest-first: five failures and four events is a
/// failure wherever the four sit, and a site cannot be satisfied by the
/// event of the site above it.
fn unlogged(corpus: &[Line]) -> (usize, Vec<usize>) {
    let (mut emits, mut fails) = (Vec::new(), Vec::new());
    for l in corpus {
        let t = l.text.as_str();
        if defines_note(t) {
            continue;
        }
        if t.contains("telemetry::emit(") || calls_note(t) {
            emits.push(l.no);
            continue;
        }
        if t.contains("failures.push(") || t.contains("failures: vec![") {
            fails.push(l.no);
        }
    }
    let mut claimed = vec![false; emits.len()];
    let mut missing = Vec::new();
    for p in &fails {
        let mut ok = false;
        for j in (0..emits.len()).rev() {
            if claimed[j] || emits[j] >= *p {
                continue;
            }
            if p - emits[j] > WINDOW {
                break;
            }
            claimed[j] = true;
            ok = true;
            break;
        }
        if !ok {
            missing.push(*p);
        }
    }
    (fails.len(), missing)
}

/// Gate 19: a failure an operator can see is a failure the log records.
/// `prod` is the production closure ([`prod_list`]); gate 19 reads its
/// `crates/pull/` and `crates/api/` files.
fn gate19(tree: &dyn Tree, prod: &[String], out: &mut Out) -> bool {
    let candidates: Vec<String> = prod
        .iter()
        .filter(|f| f.starts_with("crates/pull/") || f.starts_with("crates/api/"))
        .cloned()
        .collect();
    let w = walk(tree, &candidates, out);
    let mut counted = 0;
    let mut found = Vec::new();
    for (f, s) in &w.files {
        let (n, missing) = unlogged(&s.corpus);
        counted += n;
        found.extend(missing.iter().map(|p| format!("{f}:{p}")));
    }
    // A gate must say what it walked. A glob that matches nothing is the
    // failure mode gate 8 shipped with, and it reported success throughout.
    say!(
        out,
        "walked {} production source file(s) of crates/{{pull,api}}",
        w.walked
    );
    say!(
        out,
        "  {} line(s) in scope, {} inside a #[cfg(test)] module",
        w.kept,
        w.dropped
    );
    if w.walked == 0 {
        out.say("GATE 19 WALKED NOTHING. A gate that scans no file is not a gate.");
        return false;
    }
    if w.undelimited != 0 {
        out.say("");
        for l in &UNDELIMITED {
            out.say(l);
        }
        out.say("that shape; if it is deliberate, the boundary is what has to");
        out.say("change, not this message.");
        return false;
    }
    say!(
        out,
        "checked {counted} production failure-recording site(s)"
    );
    if !found.is_empty() {
        out.say("");
        out.say("A FAILURE IS RECORDED HERE AND NOTHING IS LOGGED:");
        for f in &found {
            say!(out, "  {f}");
        }
        out.say("");
        out.say("The receipt will show it and the log will not, so an operator");
        out.say("reading /logs after a failed run sees a quiet file. Emit an");
        out.say("event beside it — Error or Warn, so it clears the default");
        out.say("floor — or extract a note_* helper as pull::ingest does.");
        out.say("CLAUDE.md section 4: degrade loudly and name the reason.");
        return false;
    }
    out.say("OK — every recorded failure has its own event beside it.");
    true
}

// ------------------------------------------------------------ gate 10 --

/// Rows exempted ONLY HERE, never by a glyph in the document. One row id
/// per entry with the reason beside it: adding one is a visible diff and the
/// moment to ask whether the test should just be written. An entry is
/// honest ONLY while its subject does not exist; when the closing condition
/// is met, the entry goes and the test is written in the same change. The
/// allowlist keys on the ROW, so a row naming two tests has both silenced,
/// and both are printed by name every run.
///
/// P-03 LEFT THIS LIST (D-2673, carried here by D-1938). Its closing
/// condition was met: the measured, sourced calendar is pull::calendar
/// (D-1769; the charter records the trading-holiday authority), the filter is
/// Window::verdict dropping a bar on a day kind_of records Closed, and
/// pull::unit::calendar_filter drives it. The row is checked again.
const ALLOW_PENDING: &[(&str, &str)] = &[(
    "X-13",
    "no cross-vendor bar comparison exists. The reader is there -- BarFile::read_record, walked at every index by S-02, and X-12 already files each vendor under its own path prefix -- but nothing opens two vendors months and matches them bar for bar, so there is no code for a test to drive. This is NOT ruled out by docs/07-plan.md R-6: that forbids SHOWING two feeds side by side and is enforced by a picker and a column, while api::merge already refuses a universe on a cross-vendor ISIN conflict. CLOSED BY: a two-BarFile comparison in crates/store that refuses and names the first divergent timestamp, then store::unit::vendor_disagreement_refuses.",
)];

/// `^(.*):[0-9]+:([A-Za-z_][A-Za-z0-9_]*)$` -> `FILE<TAB>name`; any other
/// line passes through unchanged, as the substitution left it.
fn path_fn_line(line: &str) -> String {
    let parsed = line.rsplit_once(':').and_then(|(head, name)| {
        let (file, digits) = head.rsplit_once(':')?;
        let mut c = name.chars();
        let ident = c
            .next()
            .is_some_and(|f| f.is_ascii_alphabetic() || f == '_')
            && c.all(is_ascii_word);
        let num = !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit());
        (ident && num).then(|| format!("{file}\t{name}"))
    });
    parsed.unwrap_or_else(|| line.to_owned())
}

/// `crate name` for each `crates/<crate>/...<TAB>name` line.
fn crate_fn(line: &str) -> Option<String> {
    let mut fields = line.split('\t');
    let path = fields.next()?;
    let name = fields.next().unwrap_or("");
    let rest = path.strip_prefix("crates/")?;
    let (krate, _) = rest.split_once('/')?;
    (!krate.is_empty()).then(|| format!("{krate} {name}"))
}

/// One path segment: `[a-z_][a-z_0-9]*`.
fn segment(s: &str) -> bool {
    let mut c = s.chars();
    c.next().is_some_and(|f| f.is_ascii_lowercase() || f == '_')
        && c.all(|x| x.is_ascii_lowercase() || x.is_ascii_digit() || x == '_')
}

/// The backticked `a::b::c` tokens of a line, three segments or more,
/// leftmost first and never overlapping: `` `[a-z_][a-z_0-9]*(::...){2,}` ``.
fn qualified_tokens(line: &str) -> Vec<&str> {
    let ticks: Vec<usize> = line.match_indices('`').map(|(i, _)| i).collect();
    let mut v = Vec::new();
    let mut k = 0;
    while k + 1 < ticks.len() {
        let body = &line[ticks[k] + 1..ticks[k + 1]];
        let parts: Vec<&str> = body.split("::").collect();
        if parts.len() >= 3 && parts.iter().all(|p| segment(p)) {
            v.push(body);
            k += 2;
        } else {
            k += 1;
        }
    }
    v
}

/// `^\| *ID *\|`.
fn is_row_of(line: &str, id: &str) -> bool {
    line.strip_prefix('|')
        .map(|r| r.trim_start_matches(' '))
        .and_then(|r| r.strip_prefix(id))
        .is_some_and(|r| r.trim_start_matches(' ').starts_with('|'))
}

/// `path-declarations`: the scanner's `fns` output, `FILE:LINE:name` per
/// `fn` keyword token, as the `FILE<TAB>name` table `invariant_paths.rs`
/// reads and gate 10's crate index is built from.
fn path_declarations(fn_lines: &str) -> String {
    records(fn_lines)
        .iter()
        .map(|l| path_fn_line(l) + "\n")
        .collect()
}

/// `module-table`: the scanner's `modules` output, each compiled file and the
/// module path it is mounted as, following `#[path]` (D-1114). An empty table
/// or an UNRESOLVED mounting is a refusal, not a smaller table, and each
/// unresolved line is named: the scanner exits non-zero on one but prints it
/// only into the table.
fn module_table(modules: &str, out: &mut Out) -> bool {
    if modules.is_empty() {
        out.say("GATE 10 READ NO MODULE MOUNTING.");
        return false;
    }
    let unresolved: Vec<&str> = records(modules)
        .into_iter()
        .filter(|l| l.contains("UNRESOLVED"))
        .collect();
    if unresolved.is_empty() {
        return true;
    }
    for l in unresolved {
        out.say(l);
    }
    out.say("GATE 10: a module mounting could not be resolved.");
    false
}

/// Gate 10: every invariant names a test that exists. `path_fns` is the
/// `path-declarations` table; `invariant_paths.rs` has already read it, with
/// the module table, by the time this runs.
/// `middles` is `invariant_paths.rs --middles`, one token per line: every
/// crate-first token whose middle segments name no module of the file that
/// declares its test. The crate-and-name check reads no middle segment, so
/// `store::file::x_tests::name` passed on any `name` in `store` (D-2100,
/// carried onto this tool by D-2105).
fn gate10(
    tree: &dyn Tree,
    path_fns: &str,
    middles: &str,
    allow: &[(&str, &str)],
    out: &mut Out,
) -> bool {
    if listed(tree, &["crates/*.rs", ".github/*.rs"]).is_empty() {
        out.say("GATE 10 READ NO SOURCE FILE.");
        return false;
    }
    let fns: BTreeSet<String> = records(path_fns)
        .iter()
        .filter_map(|l| crate_fn(l))
        .collect();
    let Some(bytes) = tree.read(INVARIANTS) else {
        say!(out, "GATE 10 could not read {INVARIANTS}.");
        return false;
    };
    let doc = text_of(&bytes);
    let wrong_module: BTreeSet<&str> = records(middles).into_iter().collect();
    let tracked: BTreeSet<&str> = tree.tracked().iter().map(String::as_str).collect();
    let (mut rows, mut checked, mut missing, mut pending, mut exempt) = (0, 0, 0, 0, 0);
    // Every line, the last one included even with no newline after it: the
    // old reader skipped an unterminated last line, which only hid a row.
    for line in records(&doc) {
        if !line.starts_with('|') {
            continue;
        }
        rows += 1;
        let id: String = line.split('|').nth(1).unwrap_or("").replace(' ', "");
        for t in qualified_tokens(line) {
            let c = t.split("::").next().unwrap_or(t);
            let f = t.rsplit("::").next().unwrap_or(t);
            // A first segment that is not a tracked crate is a MODULE path,
            // `server::tests::x`, resolved by invariant_paths.rs above.
            if !tracked.contains(format!("crates/{c}/Cargo.toml").as_str()) {
                pending += 1;
                continue;
            }
            // An empty id is never an allowlist entry. The old lookup
            // matched an empty id against a blank line of the list.
            if !id.is_empty() && allow.iter().any(|(a, _)| *a == id) {
                exempt += 1;
                say!(out, "EXEMPT (allowlisted) {id} — {t}");
                continue;
            }
            checked += 1;
            if !fns.contains(&format!("{c} {f}")) {
                say!(
                    out,
                    "INVARIANT POINTS AT A TEST THAT DOES NOT EXIST: {t} ({id})"
                );
                missing += 1;
            } else if wrong_module.contains(t) {
                say!(
                    out,
                    "INVARIANT NAMES A MODULE ITS TEST IS NOT IN: {t} ({id})"
                );
                missing += 1;
            }
        }
    }
    say!(out, "read {rows} table row(s) in {INVARIANTS}");
    say!(
        out,
        "  {checked} named test(s) checked against a tracked crate"
    );
    say!(
        out,
        "  {pending} module-first, resolved by invariant_paths.rs above"
    );
    say!(out, "  {exempt} exempted by the allowlist in this file");
    say!(out, "  {missing} missing");
    if checked == 0 {
        out.say("GATE 10 CHECKED NOTHING. Either every crate vanished or the row");
        out.say("pattern stopped matching. A silent zero is not a pass.");
        return false;
    }
    let mut bad = missing != 0;
    for (i, _) in allow {
        if !records(&doc).iter().any(|l| is_row_of(l, i)) {
            say!(
                out,
                "STALE ALLOWLIST ENTRY — {i} is not a row in {INVARIANTS}"
            );
            bad = true;
        }
    }
    if bad {
        out.say("");
        out.say("Either write the test, or record abandoning the invariant in");
        out.say("docs/05-decisions.md. Rows are never deleted to go green, and");
        out.say("a status glyph no longer silences one.");
        return false;
    }
    out.say("OK — checked named invariant references resolve to tracked declarations.");
    true
}

// ------------------------------------------------------------ gate 11 --

/// A file and the occurrences it is allowed of one rule. The reason for
/// every entry, and its history, is in docs/06-limits.md under "Gate 11
/// allowlist reasons", moved there verbatim because GitHub will not start a
/// workflow file that large (D-1451). A new or changed entry still gets its
/// reason THERE, in the same change, and a count here without one is the
/// drift D-1451 forbids.
type Allow = &'static [(&'static str, usize)];

/// The eight allowlists of gate 11, by rule.
struct Allowlists {
    search: Allow,
    float: Allow,
    unsized_maps: Allow,
    sort: Allow,
    panic: Allow,
    disarm: Allow,
    assert: Allow,
    scan: Allow,
    scan_unread: Allow,
    member: Allow,
}

/// Rule 1. docs/07 layer 4: never `binary_search`.
const ALLOW_SEARCH: Allow = &[("crates/store/src/file.rs", 1)];

/// Rule 2. CLAUDE.md section 7, both halves of it.
const ALLOW_FLOAT: Allow = &[
    ("crates/api/src/ingest.rs", 1),
    ("crates/api/src/server.rs", 4),
    ("crates/core/src/price.rs", 4),
    ("crates/greeks/src/bsm.rs", 37),
    ("crates/greeks/src/normal.rs", 21),
    ("crates/greeks/src/error.rs", 17),
    ("crates/greeks/src/solver.rs", 15),
    ("crates/greeks/src/moneyness.rs", 9),
    ("crates/pull/src/pricing.rs", 23),
    ("crates/pull/src/tenor.rs", 5),
    ("crates/runner/src/grid.rs", 4),
    ("crates/runner/src/significance.rs", 41),
    ("crates/runner/src/admission.rs", 4),
    ("crates/runner/src/bootstrap.rs", 52),
    ("crates/runner/src/bootstrap_family_pass.rs", 17),
    ("crates/runner/src/outcome.rs", 26),
    ("crates/runner/src/report.rs", 4),
    ("crates/runner/src/validate.rs", 1),
    ("crates/store/src/format.rs", 14),
    ("crates/lake/src/bar.rs", 9),
    ("crates/lake/src/reader.rs", 3),
    ("crates/telemetry/src/value.rs", 5),
    ("crates/telemetry/src/encode.rs", 1),
    ("crates/telemetry/src/record.rs", 1),
    ("crates/cli/src/live.rs", 1),
    ("crates/cli/src/institutional_evidence.rs", 8),
    ("crates/cli/src/institutional_statistics.rs", 19),
    ("crates/cli/src/population_statistics_v2.rs", 18),
    ("crates/cli/src/population_statistics_v3.rs", 11),
    ("crates/cli/src/boolean_statistics_reader.rs", 3),
    ("crates/cli/src/boolean_qualification_wire.rs", 5),
    ("crates/cli/src/boolean_qualification_reader.rs", 1),
    ("crates/api/src/booleanevidencejson.rs", 1),
];

/// Rule 3. docs/07 law 2: pre-size every map.
const ALLOW_UNSIZED: Allow = &[
    ("crates/api/src/catalog.rs", 1),
    ("crates/pull/src/manifest.rs", 1),
    ("crates/pull/src/chain.rs", 2),
    ("crates/pull/src/pricing.rs", 1),
    ("crates/api/src/autopilot.rs", 1),
    ("crates/api/src/ladder.rs", 1),
    ("crates/api/src/server.rs", 4),
    ("crates/cli/src/admission_store.rs", 5),
    ("crates/cli/src/all_rung_selection_v5.rs", 6),
    ("crates/cli/src/anchored_search_lineage_v2.rs", 1),
    ("crates/cli/src/anchored_search_lineage_v3.rs", 1),
    ("crates/cli/src/anchored_search_lineage_v4.rs", 1),
    ("crates/cli/src/candidate_universe.rs", 2),
    ("crates/cli/src/execution_capability.rs", 2),
    ("crates/cli/src/execution_disposition_v2.rs", 4),
    ("crates/cli/src/execution_v3.rs", 8),
    ("crates/cli/src/execution_v4.rs", 9),
    ("crates/cli/src/frontier.rs", 1),
    ("crates/cli/src/global_replay.rs", 9),
    ("crates/cli/src/global_replay_v2.rs", 10),
    ("crates/cli/src/global_replay_v3.rs", 10),
    ("crates/cli/src/institutional_statistics.rs", 2),
    ("crates/cli/src/knobs.rs", 1),
    ("crates/cli/src/population.rs", 13),
    ("crates/cli/src/population_admission_v2.rs", 6),
    ("crates/cli/src/population_admission_v3.rs", 2),
    ("crates/cli/src/population_admission_v4.rs", 1),
    ("crates/cli/src/population_base_evidence_ledger_v2.rs", 1),
    ("crates/cli/src/population_base_evidence_v2.rs", 1),
    ("crates/cli/src/population_finalization_v2.rs", 4),
    ("crates/cli/src/population_finalization_v3.rs", 2),
    ("crates/cli/src/population_finalization_v4.rs", 2),
    ("crates/cli/src/population_observations_v1.rs", 6),
    ("crates/cli/src/population_statistics_v2.rs", 3),
    ("crates/cli/src/population_statistics_v3.rs", 1),
    ("crates/cli/src/population_v5.rs", 2),
    ("crates/cli/src/population_v6.rs", 4),
    ("crates/cli/src/pre_admission_data.rs", 4),
    ("crates/cli/src/selection.rs", 4),
    ("crates/cli/src/selection_v3.rs", 2),
    ("crates/cli/src/selection_v4.rs", 2),
    ("crates/cli/src/selection_v5.rs", 14),
    ("crates/cli/src/stored_data_completeness.rs", 1),
    ("crates/cli/src/trades.rs", 1),
    ("crates/pull/src/nse.rs", 1),
    ("crates/runner/src/lib.rs", 1),
    ("crates/runner/src/rank.rs", 1),
    ("crates/cli/src/pool.rs", 1),
    ("crates/pull/src/cash_auction.rs", 3),
    ("crates/engine/src/lib.rs", 1),
    ("crates/cli/src/all_rung_selection_v6.rs", 1),
    ("crates/cli/src/global_replay_v4.rs", 1),
    ("crates/cli/src/boolean_candidate_v1.rs", 1),
    ("crates/cli/src/index_stop.rs", 1),
    ("crates/api/src/recovery.rs", 1),
    ("crates/cli/src/boolean_catalog_command.rs", 1),
    ("crates/engine/src/resume.rs", 2),
    ("crates/cli/src/selection_v6_source.rs", 2),
    ("crates/cli/src/boolean_candidate_reader.rs", 1),
    ("crates/pull/src/cash_session_cache.rs", 2),
    ("crates/cli/src/selection_v6.rs", 1),
    ("crates/cli/src/index_stop_store.rs", 1),
    ("crates/api/src/recovery_journal.rs", 1),
    ("crates/pull/src/http.rs", 1),
];

/// Rule 4. docs/07 layer 12: bounded page, never O(universe).
const ALLOW_SORT: Allow = &[
    ("crates/runner/src/audit.rs", 2),
    ("crates/runner/src/bootstrap.rs", 4),
    ("crates/runner/src/bootstrap_family_pass.rs", 1),
    ("crates/runner/src/outcome.rs", 4),
    ("crates/runner/src/pbo.rs", 2),
    ("crates/runner/src/excursion.rs", 1),
    ("crates/pull/src/folder.rs", 2),
    ("crates/api/src/bars.rs", 3),
    ("crates/api/src/calendar_of.rs", 3),
    ("crates/api/src/constituents.rs", 3),
    ("crates/api/src/logs.rs", 1),
    ("crates/cli/src/frontier.rs", 1),
    ("crates/cli/src/global_replay_v3.rs", 1),
    ("crates/cli/src/knobs.rs", 1),
    ("crates/cli/src/population_statistics_v3.rs", 1),
    ("crates/cli/src/trades.rs", 1),
    ("crates/cli/src/live.rs", 2),
    ("crates/api/src/coverage.rs", 1),
    ("crates/api/src/indexmap.rs", 1),
    ("crates/api/src/pullrun.rs", 1),
    ("crates/api/src/server.rs", 12),
    // 8 since D-1728: `first_accepted_in_order` sorts only each selected
    // window of `top` keys, never the whole input (D-2105 carried the count
    // over from the shell allowlist the sweep replaced).
    ("crates/cli/src/lib.rs", 8),
    ("crates/engine/src/lib.rs", 2),
    ("crates/pull/src/nseindex.rs", 1),
    ("crates/runner/src/grid.rs", 3),
    ("crates/runner/src/exit_grid_policy.rs", 3),
    ("crates/runner/src/rank.rs", 3),
    ("crates/runner/src/significance.rs", 1),
    ("crates/indicators/src/evaluator.rs", 1),
    ("crates/api/src/autopilot.rs", 1),
    ("crates/api/src/catalog.rs", 2),
    ("crates/api/src/census.rs", 2),
    ("crates/api/src/master.rs", 1),
    ("crates/api/src/merge.rs", 4),
    ("crates/api/src/render.rs", 1),
    ("crates/pull/src/archive.rs", 1),
    ("crates/pull/src/manifest.rs", 1),
    ("crates/store/src/catalog.rs", 1),
    ("crates/cli/src/boolean_statistics_v1.rs", 1),
    ("crates/pull/src/ingest.rs", 1),
    ("crates/api/src/recovery.rs", 2),
    ("crates/api/src/indexstopqualificationjson.rs", 1),
    ("crates/runner/src/research_family.rs", 1),
    ("crates/api/src/indexstoprankingjson.rs", 1),
    ("crates/cli/src/index_stop_vix.rs", 1),
    ("crates/cli/src/boolean_observation_file.rs", 1),
    ("crates/cli/src/global_replay_v4.rs", 1),
    ("crates/pull/src/cash_session_cache.rs", 1),
    ("crates/cli/src/boolean_qualified_observer.rs", 1),
    ("crates/cli/src/index_stop_vix_codec.rs", 1),
    ("crates/cli/src/boolean_qualified_journal.rs", 1),
    ("crates/cli/src/pool.rs", 2),
    ("crates/cli/src/admission_store.rs", 1),
    ("crates/cli/src/population.rs", 1),
];

/// Rule 5. CLAUDE.md section 4: refuse, never die.
const ALLOW_PANIC: Allow = &[
    ("crates/api/src/server.rs", 2),
    ("crates/pull/src/ssm.rs", 1),
    ("crates/runner/src/rank.rs", 1),
    ("crates/telemetry/src/json.rs", 5),
    ("crates/telemetry/src/record.rs", 4),
];

/// Rule 5c. Nothing disarms those lints outside a test module. Two entries,
/// each the same line rule 5 allows. ssm.rs carries
/// a local allow of `clippy::expect_used` with a `reason` on `hmac`, and
/// api/src/server.rs carries one on `note_header` for its `.expect`. The
/// walk counts only a real attribute at bracket depth 0, so unlike rule 5
/// this count is exact and holds no prose. Both would read better as
/// `#[expect]`, which reports an exemption nobody needs any more; that is
/// their owners' call. Kept apart from rule 5's list because the two ask
/// different questions, "this construct appears" and "the lint that would
/// have caught it is switched off here", and a file could earn one without
/// the other.
const ALLOW_DISARM: Allow = &[
    ("crates/api/src/server.rs", 1),
    ("crates/pull/src/ssm.rs", 1),
];

/// Rule 5d. `assert!` is a panic that clippy does not lint.
const ALLOW_ASSERT: Allow = &[
    ("crates/core/src/universe.rs", 2),
    ("crates/pull/src/manifest.rs", 1),
    ("crates/store/src/layout.rs", 1),
    ("crates/store/src/file.rs", 1),
    ("crates/cli/src/candidate_universe.rs", 1),
    ("crates/cli/src/population_admission_v3.rs", 1),
];

/// Rules 6 and 7. The linear membership scan.
const ALLOW_SCAN: Allow = &[
    ("crates/api/src/audit_json.rs", 1),
    ("crates/api/src/autopilot.rs", 2),
    ("crates/api/src/backtest.rs", 1),
    ("crates/api/src/booleanjson.rs", 2),
    ("crates/api/src/expressionsearchjson.rs", 1),
    ("crates/api/src/indexstoprankingjson.rs", 1),
    ("crates/api/src/pullrun.rs", 1),
    ("crates/api/src/server.rs", 9),
    ("crates/api/src/store_wire.rs", 1),
    ("crates/cli/src/boolean_campaign_codec.rs", 2),
    ("crates/cli/src/boolean_qualification_reader.rs", 1),
    ("crates/cli/src/boolean_rung_scope.rs", 1),
    ("crates/cli/src/candidate_universe.rs", 1),
    ("crates/cli/src/global_replay_v4.rs", 1),
    ("crates/cli/src/index_stop_qualification_numeric.rs", 1),
    ("crates/cli/src/index_stop_search.rs", 1),
    ("crates/cli/src/index_stop_store.rs", 1),
    ("crates/cli/src/index_stop_vix_codec.rs", 1),
    ("crates/cli/src/lib.rs", 7),
    ("crates/cli/src/population_admission_v2.rs", 1),
    ("crates/cli/src/population_finalization_v2.rs", 1),
    ("crates/cli/src/population_v6.rs", 1),
    ("crates/cli/src/pre_admission_data.rs", 1),
    ("crates/cli/src/results.rs", 1),
    ("crates/cli/src/stored.rs", 1),
    ("crates/cli/src/strict_range_knobs.rs", 1),
    ("crates/pull/src/cash_auction.rs", 2),
    ("crates/pull/src/config.rs", 3),
    ("crates/pull/src/manifest.rs", 1),
    ("crates/pull/src/resolve.rs", 1),
    ("crates/pull/src/rolling.rs", 3),
    ("crates/pull/src/ssm.rs", 1),
    ("crates/pull/src/vendor.rs", 8),
    ("crates/runner/src/exit_grid_policy.rs", 1),
    ("crates/runner/src/grid.rs", 2),
    ("crates/runner/src/research_family.rs", 1),
    ("crates/runner/src/signal_candle_stop.rs", 1),
    ("crates/runner/src/validate.rs", 5),
    ("crates/telemetry/src/record.rs", 1),
    ("crates/telemetry/src/sink.rs", 1),
];

/// EMPTY SINCE D-1121. D-1115 pinned the 41 chained searches rule 6 first
/// saw, unread; each was opened and moved into `allow_scan` with its bound.
/// The list stays, empty, so a future change to rule 6 that surfaces sites
/// it cannot classify at once has a place that says so rather than a silent
/// raise of `allow_scan`. It may only shrink, and it is at zero.
const ALLOW_SCAN_UNREAD: Allow = &[];

/// Rule 7: the ambiguous membership test.
const ALLOW_MEMBER: Allow = &[
    ("crates/api/src/assets.rs", 1),
    ("crates/api/src/calendar_of.rs", 2),
    ("crates/api/src/constituents.rs", 2),
    ("crates/api/src/coverage.rs", 1),
    ("crates/api/src/credential_law.rs", 1),
    ("crates/api/src/merge.rs", 2),
    ("crates/api/src/server.rs", 2),
    ("crates/api/src/sweeprun.rs", 5),
    ("crates/cli/src/batch.rs", 1),
    ("crates/cli/src/candidate_universe.rs", 1),
    ("crates/cli/src/execution_capability.rs", 1),
    ("crates/cli/src/global_replay_v2.rs", 1),
    ("crates/cli/src/global_replay_v3.rs", 1),
    ("crates/cli/src/institutional_evidence.rs", 2),
    ("crates/cli/src/institutional_statistics.rs", 1),
    ("crates/cli/src/lib.rs", 7),
    ("crates/cli/src/population.rs", 3),
    ("crates/cli/src/population_admission_v2.rs", 2),
    ("crates/cli/src/population_admission_v3.rs", 2),
    ("crates/cli/src/population_finalization_v2.rs", 2),
    ("crates/cli/src/population_observations_v1.rs", 1),
    ("crates/cli/src/population_statistics_v2.rs", 7),
    ("crates/cli/src/population_statistics_v3.rs", 3),
    ("crates/cli/src/pre_admission_data.rs", 2),
    ("crates/cli/src/selection.rs", 2),
    ("crates/cli/src/selection_v3.rs", 1),
    ("crates/cli/src/selection_v4.rs", 2),
    ("crates/cli/src/selection_v5.rs", 3),
    ("crates/cli/src/stored.rs", 5),
    ("crates/cli/src/stored_data_completeness.rs", 1),
    ("crates/core/src/vendor.rs", 2),
    ("crates/costs/src/day.rs", 2),
    ("crates/engine/src/lib.rs", 1),
    ("crates/greeks/src/solver.rs", 1),
    ("crates/pull/src/calendar.rs", 1),
    ("crates/pull/src/fnowork.rs", 1),
    ("crates/pull/src/pricing.rs", 4),
    ("crates/runner/src/closed.rs", 1),
    ("crates/runner/src/lib.rs", 1),
    ("crates/runner/src/outcome.rs", 1),
    ("crates/runner/src/rank.rs", 1),
    ("crates/runner/src/split.rs", 1),
    ("crates/telemetry/src/json.rs", 3),
    ("crates/telemetry/src/tail.rs", 1),
    ("crates/cli/src/expression_search_reader.rs", 2),
    ("crates/cli/src/boolean_qualified_observer.rs", 1),
    ("crates/cli/src/index_stop_qualification.rs", 1),
    ("crates/cli/src/boolean_qualification_plan.rs", 2),
    ("crates/cli/src/index_stop_source_context.rs", 1),
    ("crates/cli/src/boolean_qualified_journal.rs", 1),
    ("crates/runner/src/expression_validation.rs", 1),
    ("crates/cli/src/boolean_statistics_v1.rs", 2),
    ("crates/cli/src/boolean_statistics_reader.rs", 1),
    ("crates/api/src/indexstopvixjson.rs", 1),
    ("crates/cli/src/boolean_qualification_wire.rs", 1),
    ("crates/cli/src/boolean_catalog_prepared.rs", 1),
    ("crates/cli/src/index_stop_qualification_codec.rs", 2),
    ("crates/api/src/booleansearchjson.rs", 1),
    ("crates/cli/src/boolean_candidate_v1.rs", 1),
    ("crates/api/src/indexstopjson.rs", 1),
    ("crates/cli/src/operation_audit.rs", 2),
    ("crates/api/src/indexstoplaunch.rs", 1),
    ("crates/cli/src/minute_gaps.rs", 1),
    ("crates/api/src/indexstopqualificationjson.rs", 1),
    ("crates/api/src/index_consistency_projection.rs", 1),
    ("crates/cli/src/index_stop_search_reader.rs", 1),
    ("crates/api/src/pullrun.rs", 3),
    ("crates/cli/src/selection_v6_source.rs", 1),
    ("crates/api/src/booleanjson.rs", 1),
    ("crates/cli/src/audited_range.rs", 1),
    ("crates/cli/src/index_consistency.rs", 2),
    ("crates/cli/src/boolean_search_record.rs", 1),
    ("crates/cli/src/boolean_campaign.rs", 2),
    ("crates/api/src/recovery.rs", 8),
    ("crates/api/src/booleanevidencejson.rs", 1),
    ("crates/cli/src/index_stop.rs", 2),
    ("crates/api/src/booleanoosjson.rs", 1),
    ("crates/cli/src/index_stop_qualification_numeric.rs", 1),
    ("crates/api/src/booleanlaunch.rs", 1),
    ("crates/cli/src/index_stop_source_context_codec.rs", 3),
    ("crates/api/src/expressionsearchjson.rs", 1),
    ("crates/engine/src/resume.rs", 1),
    ("crates/cli/src/index_stop_store.rs", 1),
    ("crates/indicators/src/column.rs", 1),
];

const ALLOWLISTS: Allowlists = Allowlists {
    search: ALLOW_SEARCH,
    float: ALLOW_FLOAT,
    unsized_maps: ALLOW_UNSIZED,
    sort: ALLOW_SORT,
    panic: ALLOW_PANIC,
    disarm: ALLOW_DISARM,
    assert: ALLOW_ASSERT,
    scan: ALLOW_SCAN,
    scan_unread: ALLOW_SCAN_UNREAD,
    member: ALLOW_MEMBER,
};

/// Rule 1: `\.(binary_search|partition_point)`.
fn rule1(r: &str) -> bool {
    r.contains(".binary_search") || r.contains(".partition_point")
}

/// `\bWORD` and, with `end`, `WORD\b`, word characters as a UTF-8 locale
/// reads them (letters of any script, digits, `_`).
fn bounded(r: &str, word: &str, end: bool) -> Vec<usize> {
    r.match_indices(word)
        .filter(|(at, _)| {
            let starts = before(r, *at).is_none_or(|c| !is_word(c));
            let after = r[at + word.len()..].chars().next();
            starts && (!end || after.is_none_or(|c| !is_word(c)))
        })
        .map(|(at, _)| at + word.len())
        .collect()
}

/// Rule 2: `\b(f32|f64)\b`.
fn rule2(r: &str) -> bool {
    !bounded(r, "f32", true).is_empty() || !bounded(r, "f64", true).is_empty()
}

/// Rule 3: `\b(HashMap|HashSet)::(new\(\)|default\(\)|with_capacity\(0\))`.
fn rule3(r: &str) -> bool {
    ["HashMap::", "HashSet::"].iter().any(|w| {
        bounded(r, w, false).iter().any(|end| {
            let t = &r[*end..];
            t.starts_with("new()")
                || t.starts_with("default()")
                || t.starts_with("with_capacity(0)")
        })
    })
}

/// Rule 4: `\.sort(_[a-z_]+)?\(|select_nth_unstable|BinaryHeap|collect::<BTree(Set|Map)`.
fn rule4(r: &str) -> bool {
    let sort = r.match_indices(".sort").any(|(at, _)| {
        let t = &r[at + 5..];
        if t.starts_with('(') {
            return true;
        }
        let Some(u) = t.strip_prefix('_') else {
            return false;
        };
        let run = u
            .bytes()
            .take_while(|b| b.is_ascii_lowercase() || *b == b'_')
            .count();
        run > 0 && u[run..].starts_with('(')
    });
    sort || r.contains("select_nth_unstable")
        || r.contains("BinaryHeap")
        || r.contains("collect::<BTreeSet")
        || r.contains("collect::<BTreeMap")
}

/// `(^|[^A-Za-z0-9_])WORD`.
fn ascii_led(r: &str, word: &str) -> Vec<usize> {
    r.match_indices(word)
        .filter(|(at, _)| before(r, *at).is_none_or(|c| !is_ascii_word(c)))
        .map(|(at, _)| at + word.len())
        .collect()
}

/// Rule 5: `(^|[^A-Za-z0-9_])(panic!|todo!|unimplemented!|unreachable!)|\.unwrap\(\)|\.expect\(`.
fn rule5(r: &str) -> bool {
    ["panic!", "todo!", "unimplemented!", "unreachable!"]
        .iter()
        .any(|w| !ascii_led(r, w).is_empty())
        || r.contains(".unwrap()")
        || r.contains(".expect(")
}

/// Rule 5d: `(^|[^A-Za-z0-9_])assert(_eq|_ne)?!`.
fn rule5d(r: &str) -> bool {
    ascii_led(r, "assert").iter().any(|end| {
        let t = &r[*end..];
        t.starts_with('!') || t.starts_with("_eq!") || t.starts_with("_ne!")
    })
}

/// Rule 6: `\.iter\(\)\.(find|position|rposition)\(`.
fn rule6(r: &str) -> bool {
    [".iter().find(", ".iter().position(", ".iter().rposition("]
        .iter()
        .any(|w| r.contains(w))
}

/// Rule 7: `\.contains\(&`.
fn rule7(r: &str) -> bool {
    r.contains(".contains(&")
}

/// Rule 5c's matcher: every record it is given already names a lint.
fn rule5c(r: &str) -> bool {
    r.contains("clippy::")
}

/// RULE 6 READS METHOD CHAINS, NOT LINES (D-1115). Past 100 columns `cargo
/// fmt` breaks a chain before every `.`, so `xs\n    .iter()\n    .find(`,
/// the formatter's default and not an evasion, never matched a one-line
/// pattern. Each record whose code starts with `.` (leading blanks dropped)
/// is folded onto the record above it in the same file, keeping that
/// record's `FILE:LINE`, so a chain is one record and is counted once.
fn chain(records: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut last_file: Option<&str> = None;
    for r in records {
        let (file, rest) = r.split_once(':').unwrap_or((r.as_str(), ""));
        let code = rest.split_once(':').map_or(rest, |(_, c)| c);
        let t = ltrim(code);
        if let Some(buf) = out.last_mut()
            && last_file == Some(file)
            && t.starts_with('.')
        {
            buf.push_str(t);
            continue;
        }
        out.push(r.clone());
        last_file = Some(file);
    }
    out
}

/// The fold's self-test, run before the corpus is read: three chains, one
/// across a file boundary that must NOT join. A broken fold is a refusal,
/// not a smaller count.
fn chain_self_test() -> Result<(), (usize, Vec<String>)> {
    let fixture: Vec<String> = [
        "a.rs:1:    let x = xs",
        "a.rs:2:        .iter()",
        "a.rs:3:        .position(is_one);",
        "a.rs:4:    let y = ys.iter().find(ok);",
        "b.rs:1:        .iter()",
        "b.rs:2:    .find(ok)",
    ]
    .iter()
    .map(|s| (*s).to_owned())
    .collect();
    let got = chain(&fixture);
    let hits = got.iter().filter(|r| rule6(r)).count();
    let joined = got
        .iter()
        .any(|r| r == "a.rs:1:    let x = xs.iter().position(is_one);");
    if hits == 3 && joined {
        Ok(())
    } else {
        Err((hits, got))
    }
}

/// Rule 5c's corpus: from a line beginning with an allow or an expect attribute, count
/// `[` against `]` until the attribute closes, and keep the whole joined
/// attribute when it names one of the five lints. `[^)]*` on one line could
/// not cross a newline, so every multi-line attribute, the shape `cargo fmt`
/// produces the moment one carries a `reason =`, was invisible. Read over
/// `scope`, because an attribute list may carry a `//` comment inside it and
/// a bracket count over stripped text loses its place. The depth carries
/// across file boundaries exactly as one walk over the whole scope did.
fn disarms(scope: &[(String, &Line)]) -> Vec<String> {
    let mut out = Vec::new();
    let mut d: i64 = 0;
    let (mut acc, mut sp, mut sn) = (String::new(), String::new(), 0);
    for (file, l) in scope {
        let t = ltrim(&l.text);
        if d <= 0 {
            if !(t.starts_with(ALLOW_ATTR) || t.starts_with("#[expect(")) {
                continue;
            }
            acc = t.to_owned();
            sp.clone_from(file);
            sn = l.no;
            d = brackets(t);
        } else {
            acc.push(' ');
            acc.push_str(t);
            d += brackets(t);
        }
        if d <= 0
            && [
                "unwrap_used",
                "expect_used",
                "panic",
                "todo",
                "unimplemented",
            ]
            .iter()
            .any(|lint| acc.contains(&format!("clippy::{lint}")))
        {
            out.push(format!("{sp}:{sn}:{acc}"));
        }
    }
    out
}

/// `^(pub...)?const[ \t]+[A-Za-z_][A-Za-z0-9_]*[ \t]*:`.
fn opens_const(s: &str) -> bool {
    let s = after_pub(s).unwrap_or(s);
    let Some(r) = s.strip_prefix("const") else {
        return false;
    };
    let name = ltrim(r);
    if name.len() == r.len() {
        return false;
    }
    let mut c = name.chars();
    if !c
        .next()
        .is_some_and(|f| f.is_ascii_alphabetic() || f == '_')
    {
        return false;
    }
    let end = name.find(|c: char| !is_ascii_word(c)).unwrap_or(name.len());
    ltrim(&name[end..]).starts_with(':')
}

/// `(^|[^A-Za-z0-9_])fn[ \t]+[a-z_]`.
fn opens_fn(s: &str) -> bool {
    ascii_led(s, "fn").iter().any(|end| {
        let r = &s[*end..];
        let t = ltrim(r);
        t.len() != r.len() && t.starts_with(|c: char| c.is_ascii_lowercase() || c == '_')
    })
}

/// Rule 5d's corpus: the lines whose nearest enclosing item is an `fn`. A
/// `const` item's assertion is evaluated at compile time and cannot panic
/// at run time; one whose header ends the line with `;` is a one-line const
/// and leaves the kind alone. Read over the corpus, so a const assertion
/// QUOTED IN A COMMENT enters neither side of the count.
fn fn_scope(files: &[(String, Stripped)]) -> Vec<String> {
    let mut out = Vec::new();
    for (f, s) in files {
        let mut is_fn = true;
        for l in &s.corpus {
            let t = l.text.trim_matches([' ', '\t']);
            if opens_const(t) {
                if !t.ends_with(';') {
                    is_fn = false;
                }
                continue;
            }
            if opens_fn(t) {
                is_fn = true;
            }
            if is_fn {
                out.push(format!("{f}:{}:{}", l.no, l.text));
            }
        }
    }
    out
}

/// `check` from the old step: count each file's hits of one rule against
/// its allowance, refuse an excess, refuse an entry naming an untracked
/// file, and warn on an entry that no longer matches. True when it refused.
fn check(rule: &Rule<'_>, src: &[String], tracked: &BTreeSet<&str>, out: &mut Out) -> bool {
    let Rule {
        name: rule,
        matches,
        allow,
        law,
    } = *rule;
    let mut bad = false;
    let hits: Vec<&String> = src.iter().filter(|r| matches(r)).collect();
    let file_of = |r: &str| r.split(':').next().unwrap_or("").to_owned();
    out.say("");
    say!(out, "rule {rule}: {} occurrence(s) — {law}", hits.len());
    let files: BTreeSet<String> = hits.iter().map(|r| file_of(r)).collect();
    for f in &files {
        let mine: Vec<&&String> = hits.iter().filter(|r| file_of(r) == *f).collect();
        let c = mine.len();
        let a = allow.iter().find(|(p, _)| p == f).map_or(0, |(_, n)| *n);
        if c > a {
            say!(out, "  REFUSED  {f} — {c} occurrence(s), {a} allowed");
            for r in mine.iter().take(8) {
                let mut parts = r.splitn(3, ':');
                let p = parts.next().unwrap_or("");
                let n = parts.next().unwrap_or("");
                say!(out, "           {p}:{n}");
            }
            bad = true;
        } else {
            say!(out, "  allowed  {f} — {c} of {a}");
        }
    }
    // An entry naming a file that no longer exists makes this gate read
    // stronger than it is, so it is a failure and not a note. An entry whose
    // file no longer matches is only loose, and says so.
    for (p, _) in allow {
        if !tracked.contains(p) {
            say!(out, "  STALE ALLOWLIST ENTRY — {p} is not a tracked file");
            bad = true;
        } else if !files.contains(*p) {
            say!(
                out,
                "  ::warning title=Gate 11 allowlist is loose::{p} no longer matches rule {rule} — tighten it"
            );
        }
    }
    bad
}

/// One rule of gate 11: its name, its pattern, its allowlist and its law.
#[derive(Clone, Copy)]
struct Rule<'a> {
    name: &'a str,
    matches: fn(&str) -> bool,
    allow: &'a [(&'a str, usize)],
    law: &'a str,
}

/// The six lints the workspace root denies and rule 5 relies on.
const DENIED_LINTS: [&str; 6] = [
    "unwrap_used",
    "expect_used",
    "panic",
    "todo",
    "unimplemented",
    "indexing_slicing",
];

/// The opening of an allow attribute, in pieces so this file never carries
/// one whole.
const ALLOW_ATTR: &str = concat!("#[", "allow(");

/// `^LINT[[:space:]]*=[[:space:]]*"deny"`.
fn denies(manifest: &str, lint: &str) -> bool {
    records(manifest).iter().any(|l| {
        l.strip_prefix(lint)
            .map(|r| r.trim_start_matches(is_space))
            .and_then(|r| r.strip_prefix('='))
            .is_some_and(|r| r.trim_start_matches(is_space).starts_with("\"deny\""))
    })
}

/// Gate 11: banned constructs on the O(1) paths.
/// `prod` is the production closure ([`prod_list`], P1-08-04, D-1938).
fn gate11(tree: &dyn Tree, lists: &Allowlists, prod: &[String], out: &mut Out) -> bool {
    let w = walk(tree, prod, out);
    let render = |f: &str, l: &Line| format!("{f}:{}:{}", l.no, l.text);
    let corpus: Vec<String> = w
        .files
        .iter()
        .flat_map(|(f, s)| s.corpus.iter().map(move |l| render(f, l)))
        .collect();
    say!(
        out,
        "walked {} production source file(s) of the crates",
        w.walked
    );
    say!(
        out,
        "  {} line(s) in scope, {} inside a #[cfg(test)] module",
        w.kept,
        w.dropped
    );
    say!(out, "  {} line(s) after comment stripping", corpus.len());
    if w.walked == 0 {
        out.say("GATE 11 WALKED NOTHING. A gate that scans no file is not a gate.");
        return false;
    }
    if w.undelimited != 0 {
        out.say("");
        for l in &UNDELIMITED {
            out.say(l);
        }
        out.say("that shape; if it is deliberate, the boundary above is what");
        out.say("has to change, not this message.");
        return false;
    }
    let tracked: BTreeSet<&str> = tree.tracked().iter().map(String::as_str).collect();
    let mut bad = false;
    let t = &tracked;
    let rules = [
        Rule {
            name: "1",
            matches: rule1,
            allow: lists.search,
            law: "docs/07 layer 4 — never binary_search",
        },
        Rule {
            name: "2",
            matches: rule2,
            allow: lists.float,
            law: "CLAUDE.md section 7 — prices are paisa i64 and never a float; statistics keep full precision",
        },
        Rule {
            name: "3",
            matches: rule3,
            allow: lists.unsized_maps,
            law: "docs/07 law 2 — pre-size every map",
        },
        Rule {
            name: "4",
            matches: rule4,
            allow: lists.sort,
            law: "docs/07 layer 12 — bounded page, never O(universe)",
        },
        Rule {
            name: "5",
            matches: rule5,
            allow: lists.panic,
            law: "CLAUDE.md section 4 — degrade loudly or refuse, never die",
        },
    ];
    for rule in &rules {
        bad |= check(rule, &corpus, t, out);
    }
    // One entry per file for rule 6: the two lists summed.
    let mut scan_all: BTreeMap<&str, usize> = BTreeMap::new();
    for (p, n) in lists.scan.iter().chain(lists.scan_unread) {
        *scan_all.entry(p).or_default() += n;
    }
    let scan_all: Vec<(&str, usize)> = scan_all.into_iter().collect();
    if let Err((got, folded)) = chain_self_test() {
        say!(
            out,
            "GATE 11 RULE 6's CHAIN FOLD FAILED ITS SELF-TEST (expected 3 hits, one per chain and none across files, got {got}):"
        );
        for r in folded {
            out.say(&r);
        }
        return false;
    }
    let chained = chain(&corpus);
    let six = Rule {
        name: "6",
        matches: rule6,
        allow: &scan_all,
        law: "CLAUDE.md section 3 rule 4 — a search must be bounded by a compile-time table, never by the data",
    };
    bad |= check(&six, &chained, t, out);
    let seven = Rule {
        name: "7",
        matches: rule7,
        allow: lists.member,
        law: "CLAUDE.md section 3 rule 4 — a membership test is a Range, a hash probe, or a declared bound",
    };
    bad |= check(&seven, &corpus, t, out);
    // Rule 5b: the lint table. Rule 5's construct is already denied by
    // clippy at the workspace root, so its count finding nothing is not
    // evidence this gate works. What clippy cannot defend is clippy:
    // deleting one line of the table disarms the deny everywhere.
    out.say("");
    out.say("rule 5b: the lint table that denies rule 5 workspace-wide");
    let manifest = tree
        .read("Cargo.toml")
        .map(|b| text_of(&b))
        .unwrap_or_default();
    for lint in DENIED_LINTS {
        if denies(&manifest, lint) {
            say!(out, "  present  clippy::{lint} = deny");
        } else {
            say!(out, "  REFUSED  Cargo.toml no longer denies clippy::{lint}");
            bad = true;
        }
    }
    let scope: Vec<(String, &Line)> = w
        .files
        .iter()
        .flat_map(|(f, s)| s.scope.iter().map(move |l| (f.clone(), l)))
        .collect();
    let disarm = disarms(&scope);
    let five_c = Rule {
        name: "5c",
        matches: rule5c,
        allow: lists.disarm,
        law: "CLAUDE.md section 4 — a local allow is a panic the lint table cannot show",
    };
    bad |= check(&five_c, &disarm, t, out);
    // A rule whose healthy state prints one word is a rule a reader can
    // confirm at a glance.
    if disarm.is_empty() {
        out.say("  none");
    }
    let fnscope = fn_scope(&w.files);
    out.say("");
    say!(
        out,
        "rule 5d scope: {} line(s) inside an fn, {}",
        fnscope.len(),
        corpus.len() - fnscope.len()
    );
    out.say("               inside a const item and therefore compile-time");
    // A classifier that stops matching would silently drop every line out
    // of scope and this rule would print a clean zero over nothing at all.
    if fnscope.is_empty() {
        out.say("  REFUSED  rule 5d classified NO line as fn scope. The classifier");
        out.say("           above stopped matching; a silent zero is not a pass.");
        bad = true;
    }
    let five_d = Rule {
        name: "5d",
        matches: rule5d,
        allow: lists.assert,
        law: "CLAUDE.md section 4 — a runtime assert is a panic clippy::panic does not lint",
    };
    bad |= check(&five_d, &fnscope, t, out);
    out.say("");
    if bad {
        out.say("GATE 11 FAILED.");
        out.say("");
        out.say("Either the construct goes, or its file earns a line in the");
        out.say("allowlist in .github/gates_ledger.rs WITH the reason it is bounded. A count is");
        out.say("raised only by someone who can say why. See CLAUDE.md");
        out.say("section 3 rule 4 and docs/07-o1-architecture.md.");
        return false;
    }
    out.say("OK — no banned construct outside the allowlist.");
    true
}

// --------------------------------------------------------------- main --

fn usage() -> String {
    "usage: gates_ledger release-profile LEAVES | invariant-ids | decision-numbers | \
     tls-provider | failure-events PROD STATUS | path-declarations < FNS | \
     module-table < MODULES | invariant-tests PATH_DECLARATIONS MIDDLES | \
     banned-constructs PROD STATUS"
        .to_owned()
}

fn read_text(path: &str) -> Result<String, String> {
    std::fs::read(path)
        .map(|b| text_of(&b))
        .map_err(|e| format!("{path}: {e}"))
}

fn read_stdin() -> Result<String, String> {
    let mut b = Vec::new();
    std::io::Read::read_to_end(&mut std::io::stdin(), &mut b).map_err(|e| format!("stdin: {e}"))?;
    Ok(text_of(&b))
}

fn emit_stdout(text: &str) -> Result<(), String> {
    let mut o = std::io::stdout().lock();
    o.write_all(text.as_bytes())
        .and_then(|()| o.flush())
        .map_err(|e| format!("stdout: {e}"))
}

fn run(args: &[String]) -> Result<bool, String> {
    let sub = args.first().map(String::as_str);
    // The two filters read stdin, write stdout, and need no tracked list.
    match (sub, args.len()) {
        (Some("path-declarations"), 1) => {
            emit_stdout(&path_declarations(&read_stdin()?))?;
            return Ok(true);
        }
        (Some("module-table"), 1) => {
            let modules = read_stdin()?;
            emit_stdout(&modules)?;
            let mut out = Out::stderr();
            return Ok(module_table(&modules, &mut out));
        }
        _ => {}
    }
    let tree = Disk::load()?;
    let mut out = Out::new(true);
    let passed = match (sub, args.len()) {
        (Some("release-profile"), 2) => gate25(&tree, &read_text(&args[1])?, &mut out),
        (Some("invariant-ids"), 1) => gate27(&tree, &mut out),
        (Some("decision-numbers"), 1) => gate27b(&tree, DECISION_PINS, &mut out),
        (Some("tls-provider"), 1) => gate26(&tree, &mut out),
        (Some("failure-events"), 3) => match prod_list(&read_text(&args[1])?, &args[2], &mut out) {
            Some(prod) => gate19(&tree, &prod, &mut out),
            None => false,
        },
        (Some("invariant-tests"), 3) => gate10(
            &tree,
            &read_text(&args[1])?,
            &read_text(&args[2])?,
            ALLOW_PENDING,
            &mut out,
        ),
        (Some("banned-constructs"), 3) => {
            match prod_list(&read_text(&args[1])?, &args[2], &mut out) {
                Some(prod) => gate11(&tree, &ALLOWLISTS, &prod, &mut out),
                None => false,
            }
        }
        _ => return Err(usage()),
    };
    Ok(passed)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(why) => {
            eprintln!("gates_ledger refused: {why}");
            ExitCode::FAILURE
        }
    }
}

// -------------------------------------------------------------- tests --

#[cfg(test)]
mod tests {
    use super::*;

    struct Mem(BTreeMap<String, Vec<u8>>, Vec<String>);

    impl Mem {
        fn new(files: &[(&str, &str)]) -> Self {
            let map: BTreeMap<String, Vec<u8>> = files
                .iter()
                .map(|(p, t)| ((*p).to_owned(), t.as_bytes().to_vec()))
                .collect();
            let tracked = map.keys().cloned().collect();
            Self(map, tracked)
        }
    }

    impl Tree for Mem {
        fn tracked(&self) -> &[String] {
            &self.1
        }
        fn read(&self, path: &str) -> Option<Vec<u8>> {
            self.0.get(path).cloned()
        }
        fn is_file(&self, path: &str) -> bool {
            self.0.contains_key(path)
        }
    }

    fn quiet() -> Out {
        Out::new(false)
    }

    // ---- shared ----

    #[test]
    fn a_pathspec_star_crosses_directories_as_git_reads_it() {
        assert!(glob("crates/*.rs", "crates/a/src/deep/x.rs"));
        assert!(glob("*Cargo.toml", "crates/a/Cargo.toml"));
        assert!(glob("*Cargo.toml", "Cargo.toml"));
        assert!(glob("crates/**/*.rs", "crates/a/b.rs"));
        assert!(!glob("crates/**/*.rs", "crates/b.rs"));
        assert!(!glob("crates/*.rs", "crates/a/x.rsx"));
        assert!(glob(".github/workflows/*", ".github/workflows/ci.yml"));
        assert!(!glob("crates/pull/src/*.rs", "crates/api/src/x.rs"));
        assert!(glob("crates/pull/src/*.rs", "crates/pull/src/a b/ü.rs"));
    }

    #[test]
    fn records_keep_a_carriage_return_and_an_unterminated_last_line() {
        assert_eq!(records("a\r\nb"), vec!["a\r", "b"]);
        assert_eq!(records("a\n"), vec!["a"]);
        assert!(records("").is_empty());
        assert_eq!(records("a\n\n"), vec!["a", ""]);
    }

    // ---- gate 25 ----

    const GOOD_LEAVES: &str = "Cargo.toml:1:profile.release = {table}\nCargo.toml:2:profile.release.overflow-checks = true\nCargo.toml:3:profile.release.panic = \"abort\"\n";

    fn g25(files: &[(&str, &str)], leaves: &str) -> (bool, String) {
        let tree = Mem::new(files);
        let mut out = quiet();
        let ok = gate25(&tree, leaves, &mut out);
        (ok, out.text)
    }

    const WF: (&str, &str) = (".github/workflows/ci.yml", "jobs:\n  x:\n");
    const ROOT: (&str, &str) = ("Cargo.toml", "");

    #[test]
    fn gate25_passes_the_two_keys_and_counts_the_census() {
        let (ok, text) = g25(
            &[
                ROOT,
                WF,
                (
                    "crates/a/src/x.rs",
                    "a.checked_add(b).saturating_sub(c); unchecked_x\n",
                ),
            ],
            GOOD_LEAVES,
        );
        assert!(ok, "{text}");
        assert!(text.contains("  context  3 explicit"), "{text}");
        assert!(text.ends_with("aborts on panic.\n"));
        assert_eq!(census("checked_ wrapping__ saturating_add1x checked_A"), 2);
    }

    #[test]
    fn gate25_refuses_a_missing_root_manifest_and_an_empty_listing() {
        let (ok, text) = g25(&[WF], GOOD_LEAVES);
        assert!(!ok);
        assert_eq!(text, "GATE 25: no root Cargo.toml\n");
        let tree = Mem(
            [("Cargo.toml".to_owned(), Vec::new())]
                .into_iter()
                .collect(),
            Vec::new(),
        );
        let mut out = quiet();
        assert!(!gate25(&tree, GOOD_LEAVES, &mut out));
        assert_eq!(out.text, "GATE 25 READ NO MANIFEST.\n");
    }

    #[test]
    fn gate25_refuses_each_missing_key() {
        let (ok, text) = g25(&[ROOT, WF], "");
        assert!(!ok);
        assert!(text.contains("has no [profile.release] table at all"));
        assert!(text.contains("no longer sets overflow-checks = true."));
        assert!(text.contains("no longer sets panic = \"abort\"."));
        assert!(text.contains("GATE 25 FAILED."));
        // A member manifest holding the keys is not the root's, and a file
        // whose name merely ends in `Cargo.toml` is not a manifest at all.
        for member in ["crates/a/Cargo.toml:", "fooCargo.toml:"] {
            let leaves = GOOD_LEAVES.replace("Cargo.toml:", member);
            let (ok, text) = g25(&[ROOT, ("crates/a/Cargo.toml", ""), WF], &leaves);
            assert!(!ok);
            assert!(
                text.contains("no longer sets overflow-checks = true."),
                "{text}"
            );
        }
        // A value other than exactly `true` is not presence.
        let off = GOOD_LEAVES.replace("checks = true", "checks = false");
        let (ok, text) = g25(&[ROOT, WF], &off);
        assert!(!ok);
        assert!(text.contains("no longer sets overflow-checks = true."));
        assert!(text.contains("REFUSED  a profile switches overflow checking back off:\nCargo.toml:2:profile.release.overflow-checks = false\n"));
        let unwind = GOOD_LEAVES.replace("\"abort\"", "\"unwind\"");
        let (ok, text) = g25(&[ROOT, WF], &unwind);
        assert!(!ok);
        assert!(text.contains("  REFUSED  [profile.release] no longer sets panic = \"abort\".\n"));
    }

    #[test]
    fn gate25_refuses_a_package_override_or_another_profile_turning_checks_off() {
        for leaf in [
            "crates/e/Cargo.toml:9:profile.release.package.engine.overflow-checks = false",
            "Cargo.toml:9:profile.bench.overflow-checks = false",
            "Cargo.toml:9:profile.release.package.\"*\".overflow-checks = 0",
        ] {
            let leaves = format!("{GOOD_LEAVES}{leaf}\n");
            let (ok, text) = g25(&[ROOT, WF], &leaves);
            assert!(!ok, "{leaf}");
            assert!(text.contains(&format!("back off:\n{leaf}\n")), "{text}");
        }
        // `true` anywhere, a key that merely ends in the name, a key outside
        // `profile`, or a file not named Cargo.toml, is not a refusal.
        for leaf in [
            "Cargo.toml:9:profile.bench.overflow-checks = true",
            "Cargo.toml:9:profile.bench.no-overflow-checks = false",
            "Cargo.toml:9:workspace.overflow-checks = false",
            "fooCargo.toml:9:profile.bench.overflow-checks = false",
        ] {
            let leaves = format!("{GOOD_LEAVES}{leaf}\n");
            let (ok, text) = g25(&[ROOT, WF], &leaves);
            assert!(ok, "{leaf}: {text}");
        }
    }

    #[test]
    fn gate25_refuses_a_workflow_that_overrides_checks_from_the_environment() {
        let env = format!("      {ENV_HEAD}RELEASE{ENV_TAIL}: false\n");
        let env2 = format!("      {ENV_HEAD}{ENV_TAIL}: false\n");
        let flag = format!("      RUSTFLAGS: -C {FLAG_KEY}=off\n");
        let flag2 = format!("      RUSTFLAGS: -C {FLAG_KEY} =\tno\n");
        let flag3 = format!("      x: \"{FLAG_KEY}=0\"\r\n");
        for body in [&env, &env2, &flag, &flag2, &flag3] {
            let wf = format!("jobs:\n{body}");
            let (ok, text) = g25(&[ROOT, (".github/workflows/ci.yml", &wf)], GOOD_LEAVES);
            assert!(!ok, "{body}");
            assert!(
                text.contains(&format!(
                    "from the environment:\n.github/workflows/ci.yml:2:{}\n",
                    body.trim_end_matches('\n')
                )),
                "{text}"
            );
        }
        for body in [
            format!("# the {FLAG_KEY} = true default\n"),
            format!("{ENV_HEAD}OVERFLOW_CHECKS: 1\n"),
            format!("{FLAG_KEY}=yes\n"),
        ] {
            let wf = format!("jobs:\n{body}");
            let (ok, text) = g25(&[ROOT, (".github/workflows/ci.yml", &wf)], GOOD_LEAVES);
            assert!(ok, "{body}: {text}");
        }
    }

    #[test]
    fn gate25_refuses_every_spelling_of_the_codegen_flag() {
        // P15-09, D-2323. rustc reads `_` as `-` in a `-C` option name.
        let under = FLAG_KEY.replace('-', "_");
        for body in [
            format!("      RUSTFLAGS: -D warnings -C {under}=off\n"),
            format!("      RUSTFLAGS: -C{under}=off\n"),
            format!("      RUSTFLAGS: -C{FLAG_KEY}=no\n"),
            format!("      RUSTFLAGS: -C {under} = false\n"),
            format!("      RUSTFLAGS: \"-C {FLAG_KEY}='off'\"\n"),
            format!("      RUSTFLAGS: -C {under}=\"n\"\n"),
        ] {
            let wf = format!("jobs:\n{body}");
            let (ok, text) = g25(&[ROOT, (".github/workflows/ci.yml", &wf)], GOOD_LEAVES);
            assert!(!ok, "{body}");
            assert!(text.contains("from the environment:\n"), "{text}");
        }
        // A composite action is a tracked `.yml` under `.github/` too.
        let act = format!("runs:\n  steps:\n    - run: RUSTFLAGS='-C {under}=off' x\n");
        let (ok, text) = g25(
            &[ROOT, WF, (".github/actions/a/action.yml", &act)],
            GOOD_LEAVES,
        );
        assert!(!ok, "{text}");
        assert!(text.contains(".github/actions/a/action.yml:3:"), "{text}");
        for body in [
            format!("RUSTFLAGS: -C {under}=on\n"),
            format!("RUSTFLAGS: -C{under}=yes\n"),
            format!("x: {under}s=off\n"),
            format!("x: {FLAG_HEAD} {FLAG_TAIL}=off\n"),
        ] {
            let wf = format!("jobs:\n{body}");
            let (ok, text) = g25(&[ROOT, (".github/workflows/ci.yml", &wf)], GOOD_LEAVES);
            assert!(ok, "{body}: {text}");
        }
    }

    // ---- gate 27 ----

    fn g27(doc: &str) -> (bool, String) {
        let tree = Mem::new(&[(INVARIANTS, doc)]);
        let mut out = quiet();
        (gate27(&tree, &mut out), out.text)
    }

    #[test]
    fn gate27_reads_every_id_shape_and_passes_unique_ones() {
        let doc = "| C-01 | a |\n| `C-E-02b` | b |\n|FV4-01|c|\n| C-02 | d |\r\n| x | not a row id |\n| c-03 | lower |\n| C-1 | short |\n";
        let (ok, text) = g27(doc);
        assert!(ok, "{text}");
        // `C-1` counts since D-2667 (D-1936): any hyphenated upper-case id.
        assert!(text.starts_with("read 5 invariant row id(s)"), "{text}");
    }

    #[test]
    fn gate27_reads_the_widened_id_shape() {
        // D-2667 (P6-04), carried here by D-1936.
        for id in [
            "S-30-session",
            "RUST-UC7-a",
            "AF-1203-a",
            "CU-SV4-CLOSE-D0961",
            "FV4-01",
            "C-1",
        ] {
            assert!(id_grammar(id), "{id}");
        }
        for id in [
            "ID",
            "Invariant",
            "c-01",
            "C-",
            "C--1",
            "C-1-",
            "Ab-1",
            "-1",
        ] {
            assert!(!id_grammar(id), "{id}");
        }
        let doc = "| S-30-session | a |\n| `S-30-session` | b |\n";
        let (ok, text) = g27(doc);
        assert!(!ok);
        assert!(
            text.contains("AN ID NAMES MORE THAN ONE INVARIANT:\n  S-30-session\n"),
            "{text}"
        );
    }

    #[test]
    fn gate27_refuses_a_duplicate_however_it_is_quoted() {
        let doc = "| C-01 | a |\n| `C-01` | b |\n| C-01a | c |\n";
        let (ok, text) = g27(doc);
        assert!(!ok);
        assert!(text.contains("AN ID NAMES MORE THAN ONE INVARIANT:\n  C-01\n      1:| C-01 | a |\n      2:| `C-01` | b |\n\n"), "{text}");
        assert!(!text.contains("3:|"));
        let long = format!("| A-10 | {} |\n| A-10 | y |\n", "x".repeat(200));
        let (ok, text) = g27(&long);
        assert!(!ok);
        assert!(text.contains(&format!("      1:| A-10 | {}\n", "x".repeat(99))));
    }

    #[test]
    fn gate27_refuses_a_silent_zero_and_a_missing_document() {
        let (ok, text) = g27("no rows here\n| lower-01 |\n");
        assert!(!ok);
        assert!(text.starts_with("GATE 27 FOUND NO ROWS."));
        let tree = Mem::new(&[]);
        let mut out = quiet();
        assert!(!gate27(&tree, &mut out));
    }

    // ---- gate 27b ----

    fn g27b(doc: &str, pins: &[(&str, usize)]) -> (bool, String) {
        let tree = Mem::new(&[(DECISIONS, doc)]);
        let mut out = quiet();
        (gate27b(&tree, pins, &mut out), out.text)
    }

    #[test]
    fn gate27b_passes_unique_numbers_and_exact_pins() {
        let doc = "## D-0001 — a\n### D-0002 — b\n#### D-0003\n### D-0009 — x\n### D-0009 — y\n##### D-0001 five hashes are not a heading\n#D-0001\n##D-0001 no space\n";
        let (ok, text) = g27b(doc, &[("D-0009", 2)]);
        assert!(ok, "{text}");
        assert!(text.starts_with("read 5 decision heading(s)"));
    }

    #[test]
    fn gate27b_refuses_a_reused_number_a_third_copy_and_a_removed_pin() {
        let (ok, text) = g27b("## D-0001 a\n### D-0001 b\n", &[]);
        assert!(!ok);
        assert!(
            text.contains(
                "  D-0001: 2 heading(s), allowed 1\n      1:## D-0001 a\n      2:### D-0001 b\n"
            ),
            "{text}"
        );
        let (ok, text) = g27b("## D-0009\n## D-0009\n## D-0009\n", &[("D-0009", 2)]);
        assert!(!ok);
        assert!(text.contains("  D-0009: 3 heading(s), allowed 2"));
        let (ok, text) = g27b("## D-0001\n", &[("D-0009", 2)]);
        assert!(!ok);
        assert!(
            text.contains("  D-0009: 0 heading(s), allowed 2\n"),
            "{text}"
        );
        // Five digits head the four-digit number, as the old extraction read
        // it; the listing then shows only the exact heading.
        let (ok, text) = g27b("## D-1234 a\n## D-12345 b\n", &[]);
        assert!(!ok);
        assert!(
            text.contains("  D-1234: 2 heading(s), allowed 1\n      1:## D-1234 a\n\n"),
            "{text}"
        );
    }

    #[test]
    fn gate27b_refuses_a_silent_zero() {
        let (ok, text) = g27b("# D-0001\nD-0002\n", &[]);
        assert!(!ok);
        assert!(text.starts_with("GATE 27B FOUND NO DECISION HEADINGS."));
    }

    // ---- gate 26 ----

    fn g26(files: &[(&str, &str)]) -> (bool, String) {
        let tree = Mem::new(files);
        let mut out = quiet();
        (gate26(&tree, &mut out), out.text)
    }

    #[test]
    fn gate26_passes_a_guarded_client_and_refuses_an_unguarded_one() {
        let (ok, text) = g26(&[(
            "crates/pull/src/http.rs",
            "fn a() { pull::ensure_tls_provider(); let c = Client::builder(); }\n",
        )]);
        assert!(ok, "{text}");
        assert!(text.contains("  ok  crates/pull/src/http.rs — 1 site(s), 1 guard(s)"));
        let (ok, text) = g26(&[
            (
                "crates/pull/src/a.rs",
                "ensure_tls_provider();\nClient::builder()\n",
            ),
            (
                "crates/pull/src/b.rs",
                "Client::builder()\nClient::builder()\nensure_tls_provider()\n",
            ),
        ]);
        assert!(!ok);
        assert!(text.contains("UNGUARDED CLIENT  crates/pull/src/b.rs — 2 Client::builder() call(s), 1 ensure_tls_provider()"), "{text}");
        assert!(text.contains("checked 3 client construction site(s) in 2 file(s)"));
    }

    #[test]
    fn gate26_refuses_a_silent_zero_and_reads_only_crates() {
        let (ok, text) = g26(&[
            (".github/x.rs", "Client::builder()\n"),
            ("crates/a/src/x.rs", "\n"),
        ]);
        assert!(!ok);
        assert!(text.contains("GATE 26 FOUND NO CLIENT SITES."));
    }

    // ---- the stripper ----

    #[test]
    fn the_stripper_drops_a_test_module_and_keeps_what_follows() {
        let src = "fn a() {}\n#[cfg(test)]\n// note\n#[expect(\n  x,\n)]\nmod tests {\n    fn t() {}\n\n}\nfn b() {}\n#[cfg(test)]\nconst X: u8 = 1;\n// c\n";
        let s = strip(src).unwrap();
        let kept: Vec<usize> = s.scope.iter().map(|l| l.no).collect();
        assert_eq!(kept, vec![1, 11, 12, 13, 14]);
        let code: Vec<usize> = s.corpus.iter().map(|l| l.no).collect();
        assert_eq!(code, vec![1, 11, 12, 13]);
        assert_eq!((s.dropped, s.total), (9, 14));
    }

    #[test]
    fn the_stripper_refuses_what_it_cannot_delimit() {
        // A module still open at end of file.
        assert!(strip("#[cfg(test)]\nmod t {\n    fn x() {}\n").is_none());
        // An attribute with no item under it.
        assert!(strip("#[cfg(test)]\n").is_none());
        // Code at the module's indent before its own closing brace.
        assert!(strip("#[cfg(test)]\nmod t {\nfn x() {}\n}\n").is_none());
        // A brace at a deeper indent does not close it; a shallower one
        // inside is still a refusal.
        assert!(strip("    #[cfg(test)]\n    mod t {\n  }\n    }\n").is_none());
        // Blank and comment lines at the indent are not code.
        assert!(strip("#[cfg(test)]\nmod t {\n\n// x\n}\n").is_some());
    }

    #[test]
    fn the_stripper_reads_the_module_shapes_the_old_pattern_read() {
        for open in [
            "pub mod t {",
            "pub(crate) mod t {",
            "pub (super) mod t { ",
            "\tmod t_1 {\t",
        ] {
            let ind = if open.starts_with('\t') { "\t" } else { "" };
            let src = format!("#[cfg(test)]\n{open}\n    x.unwrap();\n{ind}}}\n");
            let s = strip(&src).unwrap_or_else(|| panic!("{open}"));
            assert!(s.corpus.is_empty(), "{open}");
        }
        // Not the shape: one line, a body on the line, a CRLF attribute.
        for src in [
            "#[cfg(test)] mod t {\n    x.unwrap();\n}\n",
            "#[cfg(test)]\nmod t { fn a() {} }\nx.unwrap();\n",
            "#[cfg(test)]\r\nmod t {\r\n    x.unwrap();\r\n}\r\n",
            "#[cfg(test)]\nfn t() {\n    x.unwrap();\n}\n",
        ] {
            let s = strip(src).unwrap_or_else(|| panic!("{src}"));
            assert!(s.corpus.iter().any(|l| l.text.contains("unwrap")), "{src}");
        }
        // A CRLF module that is recognised but whose brace is `}\r` cannot
        // be closed, so it refuses rather than swallowing what follows.
        assert!(strip("#[cfg(test)]\nmod t {\n    x;\n}\r\nfn y() {}\n").is_none());
    }

    // ---- gate 19 ----

    fn names(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| (*s).to_owned()).collect()
    }

    fn g19_of(files: &[(&str, &str)], prod: &[String]) -> (bool, String) {
        let tree = Mem::new(files);
        let mut out = quiet();
        (gate19(&tree, prod, &mut out), out.text)
    }

    fn g19(files: &[(&str, &str)]) -> (bool, String) {
        g19_of(files, &prod_of(files))
    }

    #[test]
    fn a_production_closure_is_read_and_a_failed_or_empty_one_refused() {
        // P1-08-04 (D-2660, D-1938).
        let mut out = quiet();
        assert_eq!(
            prod_list(
                "crates/a/src/lib.rs\ncrates/cli/commit_stamp.rs\n",
                "0",
                &mut out
            ),
            Some(names(&[
                "crates/a/src/lib.rs",
                "crates/cli/commit_stamp.rs"
            ]))
        );
        let mut out = quiet();
        assert_eq!(
            prod_list(
                "UNRESOLVED crates/a/src/lib.rs: mod gone\ncrates/a/src/lib.rs\n",
                "1",
                &mut out
            ),
            None
        );
        assert!(out.text.contains("REFUSED  the production closure could not be resolved:\n  UNRESOLVED crates/a/src/lib.rs: mod gone\n"), "{}", out.text);
        let mut out = quiet();
        assert_eq!(prod_list("", "0", &mut out), None);
        assert_eq!(out.text, "READ NO PRODUCTION FILE.\n");
        let mut out = quiet();
        assert_eq!(prod_list("a.rs\n", "x", &mut out), None);
        assert!(out.text.contains("`x` is not an exit status"));
    }

    #[test]
    fn gates_11_and_19_read_the_production_closure_not_a_stem() {
        // P1-08-04 (D-2660, D-1938): a production file sharing its stem with
        // a crate-root `#[cfg(test)] mod manifest;` is read, and so is a
        // `#[path]` file outside `src/`.
        let lib = "#[cfg(test)]\nmod manifest;\n";
        let files = [
            ("crates/pull/src/lib.rs", lib),
            ("crates/pull/src/resume/manifest.rs", "failures.push(f);\n"),
            ("crates/pull/stamp.rs", "x.unwrap();\n"),
        ];
        let prod = names(&[
            "crates/pull/src/lib.rs",
            "crates/pull/src/resume/manifest.rs",
            "crates/pull/stamp.rs",
        ]);
        let (ok, text) = g19_of(&files, &prod);
        assert!(!ok);
        assert!(
            text.contains("  crates/pull/src/resume/manifest.rs:1\n"),
            "{text}"
        );
        assert!(text.starts_with("walked 3 production"), "{text}");
        let mut all = vec![("Cargo.toml", LINTS)];
        all.extend_from_slice(&files);
        let tree = Mem::new(&all);
        let mut out = quiet();
        assert!(!gate11(&tree, &lists(), &prod, &mut out));
        assert!(out.text.contains("crates/pull/stamp.rs:1"), "{}", out.text);
    }

    #[test]
    fn gate19_passes_a_failure_with_its_own_event() {
        let src = "fn a() {\n    telemetry::emit(x);\n    failures.push(f);\n    note_bad(y);\n    receipt.failures.push(g);\n}\n";
        let (ok, text) = g19(&[("crates/pull/src/a.rs", src)]);
        assert!(ok, "{text}");
        assert!(text.contains("checked 2 production failure-recording site(s)"));
    }

    #[test]
    fn gate19_refuses_a_failure_that_borrows_or_lacks_an_event() {
        // Two failures, one event: the second cannot borrow the first's.
        let shared =
            "fn a() {\n    telemetry::emit(x);\n    failures.push(f);\n    failures: vec![g],\n}\n";
        let (ok, text) = g19(&[("crates/api/src/a.rs", shared)]);
        assert!(!ok);
        assert!(
            text.contains(
                "A FAILURE IS RECORDED HERE AND NOTHING IS LOGGED:\n  crates/api/src/a.rs:4\n"
            ),
            "{text}"
        );
        // An event below the failure, on its line, past the window, in a
        // whole-line comment or a helper's own signature is no event.
        let far = format!(
            "telemetry::emit(x);\n{}failures.push(f);\n",
            "\n".repeat(12)
        );
        for src in [
            "failures.push(f);\ntelemetry::emit(x);\n",
            "failures.push(f); // see telemetry::emit\n",
            far.as_str(),
            "// telemetry::emit(x);\nfailures.push(f);\n",
            "fn note_bad(x: u8) {\n}\nfailures.push(f);\n",
        ] {
            let (ok, text) = g19(&[("crates/pull/src/a.rs", src)]);
            assert!(!ok, "{src}");
            assert!(text.contains("  crates/pull/src/a.rs:"), "{text}");
        }
        // Exactly twelve lines above is inside the window.
        let near = format!(
            "telemetry::emit(x);\n{}failures.push(f);\n",
            "\n".repeat(11)
        );
        assert!(g19(&[("crates/pull/src/a.rs", &near)]).0);
    }

    #[test]
    fn gate19_scans_past_a_test_module_and_skips_only_compiled_out_files() {
        let src = "#[cfg(test)]\nmod tests {\n    failures.push(f);\n}\nfn b() {\n    failures.push(g);\n}\n";
        let (ok, text) = g19(&[("crates/pull/src/a.rs", src)]);
        assert!(!ok);
        assert!(text.contains("  crates/pull/src/a.rs:6\n"), "{text}");
        assert!(text.contains("  3 line(s) in scope, 4 inside a #[cfg(test)] module"));
        // A file the closure leaves out is not walked; one whose first line
        // is `#![cfg(test)]` is not walked even when listed.
        let lib = "#[cfg(test)]\nmod emit_sites;\n";
        let files = [
            ("crates/pull/src/lib.rs", lib),
            ("crates/pull/src/emit_sites.rs", "failures.push(f);\n"),
            (
                "crates/pull/src/inner.rs",
                "#![cfg(test)]\nfailures.push(f);\n",
            ),
            (
                "crates/pull/src/b.rs",
                "telemetry::emit(x);\nfailures.push(f);\n",
            ),
        ];
        let prod = names(&[
            "crates/pull/src/lib.rs",
            "crates/pull/src/inner.rs",
            "crates/pull/src/b.rs",
        ]);
        let (ok, text) = g19_of(&files, &prod);
        assert!(ok, "{text}");
        assert!(text.starts_with("walked 2 production"));
        // Only a CRLF attribute: the module is not recognised, so it is read.
        let crlf = "#[cfg(test)]\r\nmod tests {\r\n    failures.push(f);\r\n}\r\n";
        assert!(!g19(&[("crates/pull/src/a.rs", crlf)]).0);
    }

    #[test]
    fn gate19_refuses_an_undelimited_module_and_a_silent_walk() {
        let (ok, text) = g19(&[(
            "crates/api/src/a.rs",
            "#[cfg(test)]\nmod t {\nfn x() {}\n}\n",
        )]);
        assert!(!ok);
        assert!(text.contains("UNDELIMITED TEST MODULE  crates/api/src/a.rs\n"));
        assert!(text.contains("the boundary is what has to\nchange, not this message.\n"));
        let (ok, text) = g19(&[("crates/store/src/a.rs", "failures.push(f);\n")]);
        assert!(!ok);
        assert!(text.contains("GATE 19 WALKED NOTHING."));
    }

    // ---- gate 10 ----

    const FNS: &str = "crates/store/src/unit.rs:3:reads\n.github/t.rs:1:tool_fn\nodd line\n";

    fn g10(doc: &str, allow: &[(&str, &str)]) -> (bool, String) {
        let tree = Mem::new(&[
            (INVARIANTS, doc),
            ("crates/store/Cargo.toml", ""),
            ("crates/store/src/unit.rs", ""),
        ]);
        let mut out = quiet();
        (
            gate10(&tree, &path_declarations(FNS), "", allow, &mut out),
            out.text,
        )
    }

    #[test]
    fn gate10_passes_a_row_whose_test_exists_in_its_crate() {
        let doc = "| S-01 | x | `store::unit::reads` | ✓ |\n| M-01 | `server::tests::x` |\n| S-02 | `store::other::reads` and `a::b` |";
        let (ok, text) = g10(doc, &[]);
        assert!(ok, "{text}");
        assert!(
            text.starts_with("read 3 table row(s) in docs/04-invariants.md\n"),
            "{text}"
        );
        assert!(text.contains("  2 named test(s) checked"));
        assert!(text.contains("  1 module-first"));
    }

    #[test]
    fn gate10_refuses_a_missing_test_and_a_test_in_the_wrong_crate() {
        let doc = "| S-01 | `store::unit::gone` |\n| S-02 | `store::unit::tool_fn` |\n";
        let (ok, text) = g10(doc, &[]);
        assert!(!ok);
        assert!(
            text.contains(
                "INVARIANT POINTS AT A TEST THAT DOES NOT EXIST: store::unit::gone (S-01)"
            )
        );
        assert!(text.contains(
            "INVARIANT POINTS AT A TEST THAT DOES NOT EXIST: store::unit::tool_fn (S-02)"
        ));
        assert!(text.contains("  2 missing"));
        // The last row counts even with no newline after it.
        let (ok, _) = g10(
            "| S-01 | `store::unit::reads` |\n| S-02 | `store::unit::gone` |",
            &[],
        );
        assert!(!ok);
    }

    #[test]
    fn gate10_refuses_a_token_whose_middle_names_no_module_of_its_test() {
        let doc = "| S-01 | `store::unit::reads` |\n| S-02 | `store::wrong::reads` |\n";
        let tree = Mem::new(&[
            (INVARIANTS, doc),
            ("crates/store/Cargo.toml", ""),
            ("crates/store/src/unit.rs", ""),
        ]);
        let mut out = quiet();
        let ok = gate10(
            &tree,
            &path_declarations(FNS),
            "store::wrong::reads\n",
            &[],
            &mut out,
        );
        assert!(!ok, "{}", out.text);
        assert!(
            out.text.contains(
                "INVARIANT NAMES A MODULE ITS TEST IS NOT IN: store::wrong::reads (S-02)"
            ),
            "{}",
            out.text
        );
        assert!(
            !out.text.contains("store::unit::reads (S-01)"),
            "{}",
            out.text
        );
        assert!(out.text.contains("  1 missing"), "{}", out.text);
    }

    #[test]
    fn gate10_exempts_only_a_listed_row_and_refuses_a_stale_entry() {
        let doc = "| P-03 | `store::unit::gone` |\n| S-01 | `store::unit::reads` |\n";
        let (ok, text) = g10(doc, &[("P-03", "why")]);
        assert!(ok, "{text}");
        assert!(text.contains("EXEMPT (allowlisted) P-03 — store::unit::gone"));
        let (ok, text) = g10(doc, &[("P-03", "why"), ("X-13", "why")]);
        assert!(!ok);
        assert!(
            text.contains("STALE ALLOWLIST ENTRY — X-13 is not a row in docs/04-invariants.md")
        );
        // An empty id is never exempt.
        let (ok, _) = g10(
            "|  | `store::unit::gone` |\n| S-01 | `store::unit::reads` |\n",
            &[("P-03", "")],
        );
        assert!(!ok);
    }

    #[test]
    fn gate10_refuses_a_silent_zero_a_missing_source_and_a_bad_module_table() {
        let (ok, text) = g10("| S-01 | `store::unit` only two |\n", &[]);
        assert!(!ok);
        assert!(text.contains("GATE 10 CHECKED NOTHING."));
        let tree = Mem::new(&[(INVARIANTS, "| S-01 | `store::unit::reads` |\n")]);
        let mut out = quiet();
        assert!(!gate10(&tree, "", "", &[], &mut out));
        assert_eq!(out.text, "GATE 10 READ NO SOURCE FILE.\n");
        let mut out = quiet();
        assert!(!module_table("", &mut out));
        assert_eq!(out.text, "GATE 10 READ NO MODULE MOUNTING.\n");
        let mut out = quiet();
        assert!(!module_table("a.rs\tx\nUNRESOLVED b.rs: mod y\n", &mut out));
        assert_eq!(
            out.text,
            "UNRESOLVED b.rs: mod y\nGATE 10: a module mounting could not be resolved.\n"
        );
        let mut out = quiet();
        assert!(module_table("a.rs\tx\n", &mut out));
        assert!(out.text.is_empty());
    }

    #[test]
    fn gate10_token_and_table_parsing_match_the_old_patterns() {
        assert_eq!(
            qualified_tokens("`a::b::c` x `d::e` `f::g::h::i` `A::b::c`"),
            vec!["a::b::c", "f::g::h::i"]
        );
        assert_eq!(qualified_tokens("`x` `a::b::c` `"), vec!["a::b::c"]);
        assert_eq!(qualified_tokens("` `a::b::c`"), vec!["a::b::c"]);
        assert_eq!(
            path_fn_line("crates/a/src/x.rs:12:name"),
            "crates/a/src/x.rs\tname"
        );
        assert_eq!(path_fn_line("odd line"), "odd line");
        assert_eq!(
            path_declarations(FNS),
            "crates/store/src/unit.rs\treads\n.github/t.rs\ttool_fn\nodd line\n"
        );
        assert_eq!(
            crate_fn("crates/a/src/x.rs\tname").as_deref(),
            Some("a name")
        );
        assert_eq!(crate_fn(".github/x.rs\tname"), None);
        assert!(is_row_of("|  P-03 | x", "P-03"));
        assert!(!is_row_of("| P-030 | x", "P-03"));
    }

    // ---- gate 11 ----

    const NONE: Allow = &[];

    fn lists() -> Allowlists {
        Allowlists {
            search: NONE,
            float: NONE,
            unsized_maps: NONE,
            sort: NONE,
            panic: NONE,
            disarm: NONE,
            assert: NONE,
            scan: NONE,
            scan_unread: NONE,
            member: NONE,
        }
    }

    const LINTS: &str = "unwrap_used = \"deny\"\nexpect_used = \"deny\"\npanic = \"deny\"\ntodo = \"deny\"\nunimplemented = \"deny\"\nindexing_slicing   =   \"deny\"\n";

    /// What `source_scan prod-files` names for a tree with no `mod`
    /// tables: every file under a crate's `src/` outside a test directory.
    fn prod_of(files: &[(&str, &str)]) -> Vec<String> {
        files
            .iter()
            .map(|(p, _)| (*p).to_owned())
            .filter(|f| {
                f.starts_with("crates/")
                    && f.ends_with(".rs")
                    && f.contains("/src/")
                    && !f.contains("/tests/")
                    && !f.contains("/benches/")
            })
            .collect()
    }

    fn g11(files: &[(&str, &str)], l: &Allowlists) -> (bool, String) {
        let mut all = vec![("Cargo.toml", LINTS)];
        all.extend_from_slice(files);
        let tree = Mem::new(&all);
        let mut out = quiet();
        (gate11(&tree, l, &prod_of(files), &mut out), out.text)
    }

    #[test]
    fn gate11_passes_clean_code_and_ignores_tests_comments_and_test_dirs() {
        let src = "fn a() -> u8 {\n    // x.unwrap() and f64 in prose\n    1\n}\n#[cfg(test)]\nmod tests {\n    fn t() { x.unwrap(); let y: f64 = 1.0; }\n}\n";
        let (ok, text) = g11(
            &[
                ("crates/a/src/lib.rs", src),
                ("crates/a/tests/t.rs", "x.unwrap();\n"),
                ("crates/a/src/benches/b.rs", "x.unwrap();\n"),
                ("crates/a/benches/b.rs", "x.unwrap();\n"),
            ],
            &lists(),
        );
        assert!(ok, "{text}");
        assert!(text.starts_with("walked 1 production source file(s) of the crates\n  4 line(s) in scope, 4 inside a #[cfg(test)] module\n  3 line(s) after comment stripping\n"), "{text}");
        assert!(text.contains("\n  none\n"));
    }

    fn refused(src: &str, rule: &str) {
        let (ok, text) = g11(&[("crates/a/src/x.rs", src)], &lists());
        assert!(!ok, "rule {rule}: {src}");
        let head = format!("rule {rule}: ");
        let at = text.find(&head).unwrap_or_else(|| panic!("{text}"));
        let block = &text[at..];
        assert!(
            block.contains("  REFUSED  crates/a/src/x.rs — 1 occurrence(s), 0 allowed\n           crates/a/src/x.rs:1\n"),
            "rule {rule}: {src}\n{text}"
        );
    }

    #[test]
    fn gate11_refuses_each_rule_with_its_file_and_line() {
        refused("let i = xs.binary_search(&k);\n", "1");
        refused("let i = xs.partition_point(p);\n", "1");
        refused("let x: f64 = 1.0;\n", "2");
        refused("fn a(x: f32) {}\n", "2");
        refused("let m = HashMap::new();\n", "3");
        refused(
            "let m = std::collections::HashSet::with_capacity(0);\n",
            "3",
        );
        refused("xs.sort();\n", "4");
        refused("xs.sort_unstable_by_key(k);\n", "4");
        refused("let h = BinaryHeap::with_capacity(9);\n", "4");
        refused("let s = it.collect::<BTreeSet<_>>();\n", "4");
        refused("x.unwrap();\n", "5");
        refused("x.expect(\"y\");\n", "5");
        refused("unreachable!(\"z\");\n", "5");
        refused("    let p = xs.iter().position(|x| x == y);\n", "6");
        refused("if xs.contains(&y) {}\n", "7");
        refused(
            "fn f() {\n    assert!(x);\n}\n"
                .replace("fn f() {\n", "")
                .as_str(),
            "5d",
        );
        refused("assert_eq!(a, b);\n", "5d");
    }

    #[test]
    fn gate11_reads_word_boundaries_as_the_old_patterns_did() {
        for clean in [
            "let x_f64 = 1;\n",
            "let f64x = 1;\n",
            "let éf64 = 1;\n",
            "let m = MyHashMap::new();\n",
            "let m = HashMap::with_capacity(10);\n",
            "xs.sorted();\n",
            "xs.sort_(x);\n",
            "my_panic!(x);\n",
            "debug_assert!(x);\n",
            "x.unwrap_or(0);\n",
            "xs.iter().any(f);\n",
            "xs.contains(y);\n",
        ] {
            let (ok, text) = g11(&[("crates/a/src/x.rs", clean)], &lists());
            assert!(ok, "{clean}: {text}");
        }
        for hit in [
            "let a = [f64; 2];\n",
            "(f32)\n",
            "x:f64\n",
            "core::panic!(x);\n",
            "xs.sort_by(f);\n",
        ] {
            let (ok, _) = g11(&[("crates/a/src/x.rs", hit)], &lists());
            assert!(!ok, "{hit}");
        }
    }

    #[test]
    fn gate11_folds_a_formatted_chain_into_one_record() {
        let src = "fn a() {\n    let p = xs\n        .iter()\n        .find(|x| x.ok());\n}\n";
        let (ok, text) = g11(&[("crates/a/src/x.rs", src)], &lists());
        assert!(!ok);
        assert!(text.contains("rule 6: 1 occurrence(s)"), "{text}");
        assert!(text.contains("           crates/a/src/x.rs:2\n"));
        // A chain never joins across a file boundary.
        let (ok, text) = g11(
            &[
                ("crates/a/src/x.rs", "let p = xs\n"),
                ("crates/a/src/y.rs", "    .iter().find(f);\n"),
            ],
            &lists(),
        );
        assert!(!ok);
        assert!(text.contains("  REFUSED  crates/a/src/y.rs — 1"), "{text}");
        assert!(chain_self_test().is_ok());
    }

    #[test]
    fn gate11_allows_up_to_the_count_and_refuses_one_more() {
        const ONE: Allow = &[("crates/a/src/x.rs", 1)];
        let l = Allowlists {
            panic: ONE,
            ..lists()
        };
        let (ok, text) = g11(&[("crates/a/src/x.rs", "x.unwrap();\n")], &l);
        assert!(ok, "{text}");
        assert!(text.contains("  allowed  crates/a/src/x.rs — 1 of 1"));
        let (ok, text) = g11(&[("crates/a/src/x.rs", "x.unwrap();\ny.unwrap();\n")], &l);
        assert!(!ok);
        assert!(text.contains("  REFUSED  crates/a/src/x.rs — 2 occurrence(s), 1 allowed"));
        // Both unread and read rule-6 entries count toward one file.
        const HALF: Allow = &[("crates/a/src/x.rs", 1)];
        let l = Allowlists {
            scan: HALF,
            scan_unread: HALF,
            ..lists()
        };
        let src = "xs.iter().find(a);\nxs.iter().find(b);\n";
        assert!(g11(&[("crates/a/src/x.rs", src)], &l).0);
    }

    #[test]
    fn gate11_refuses_a_stale_entry_and_warns_on_a_loose_one() {
        const STALE: Allow = &[("crates/a/src/gone.rs", 1), ("crates/a/src/x.rs", 1)];
        let l = Allowlists {
            search: STALE,
            ..lists()
        };
        let (ok, text) = g11(&[("crates/a/src/x.rs", "fn a() {}\n")], &l);
        assert!(!ok);
        assert!(
            text.contains("  STALE ALLOWLIST ENTRY — crates/a/src/gone.rs is not a tracked file")
        );
        assert!(text.contains("  ::warning title=Gate 11 allowlist is loose::crates/a/src/x.rs no longer matches rule 1 — tighten it"));
    }

    #[test]
    fn gate11_guards_the_lint_table() {
        for lint in DENIED_LINTS {
            let table = LINTS.replace(&format!("{lint} "), &format!("# {lint} "));
            let tree = Mem::new(&[("Cargo.toml", &table), ("crates/a/src/x.rs", "fn a() {}\n")]);
            let mut out = quiet();
            assert!(
                !gate11(&tree, &lists(), &names(&["crates/a/src/x.rs"]), &mut out),
                "{lint}"
            );
            assert!(out.text.contains(&format!(
                "  REFUSED  Cargo.toml no longer denies clippy::{lint}\n"
            )));
        }
        assert!(!denies("unwrap_used = \"warn\"\n", "unwrap_used"));
        assert!(!denies("panic_in_result_fn = \"deny\"\n", "panic"));
        let tree = Mem::new(&[("crates/a/src/x.rs", "fn a() {}\n")]);
        let mut out = quiet();
        assert!(!gate11(
            &tree,
            &lists(),
            &names(&["crates/a/src/x.rs"]),
            &mut out
        ));
    }

    #[test]
    fn gate11_sees_a_local_allow_on_one_line_or_many() {
        for src in [
            format!("{ALLOW_ATTR}clippy::expect_used)]\nfn a() {{}}\n"),
            "#[expect(\n    clippy::unwrap_used,\n    reason = \"x\"\n)]\nfn a() {}\n".to_owned(),
            "#[expect(\n    // a comment [ inside\n    clippy::todo,\n    reason = \"x\"\n)]]\nfn a() {}\n".to_owned(),
            format!("    {ALLOW_ATTR}dead_code, clippy::panic)]\n"),
        ] {
            let (ok, text) = g11(&[("crates/a/src/x.rs", &src)], &lists());
            assert!(!ok, "{src}");
            assert!(text.contains("rule 5c: 1 occurrence(s)"), "{src}\n{text}");
        }
        let clean = format!("{ALLOW_ATTR}dead_code)]\nfn a() {{}}\n");
        let (ok, text) = g11(&[("crates/a/src/x.rs", &clean)], &lists());
        assert!(ok, "{text}");
        // An allow inside a test module is not shipping code.
        let src = format!(
            "#[cfg(test)]\nmod t {{\n    {ALLOW_ATTR}clippy::unwrap_used)]\n    fn a() {{}}\n}}\nfn b() {{}}\n"
        );
        assert!(g11(&[("crates/a/src/x.rs", &src)], &lists()).0);
    }

    #[test]
    fn gate11_counts_an_assert_only_inside_a_function() {
        let src = "const _: () = {\n    assert!(N > 0);\n};\npub const ONE: u8 = 1;\nfn f() {\n    assert!(x);\n}\n";
        let (ok, text) = g11(&[("crates/a/src/x.rs", src)], &lists());
        assert!(!ok);
        assert!(text.contains("rule 5d: 1 occurrence(s)"), "{text}");
        assert!(text.contains("           crates/a/src/x.rs:6\n"));
        assert!(
            text.contains("rule 5d scope: 3 line(s) inside an fn, 4\n"),
            "{text}"
        );
        let quoted = "fn f() {}\n// const X: () = assert!(true);\n";
        assert!(g11(&[("crates/a/src/x.rs", quoted)], &lists()).0);
    }

    #[test]
    fn gate11_refuses_an_empty_fn_scope_an_undelimited_file_and_a_silent_walk() {
        let (ok, text) = g11(
            &[("crates/a/src/x.rs", "const A: u8 = {\n1\n};\n")],
            &lists(),
        );
        assert!(!ok);
        assert!(
            text.contains("  REFUSED  rule 5d classified NO line as fn scope."),
            "{text}"
        );
        let (ok, text) = g11(
            &[("crates/a/src/x.rs", "#[cfg(test)]\nmod t {\n")],
            &lists(),
        );
        assert!(!ok);
        assert!(text.contains("UNDELIMITED TEST MODULE  crates/a/src/x.rs\n"));
        assert!(text.contains("the boundary above is what\nhas to change, not this message.\n"));
        let (ok, text) = g11(&[("crates/a/tests/x.rs", "x.unwrap();\n")], &lists());
        assert!(!ok);
        assert!(text.contains("GATE 11 WALKED NOTHING."));
    }

    #[test]
    fn gate11_reads_a_one_line_file_with_no_newline() {
        let (ok, text) = g11(&[("crates/a/src/x.rs", "fn f() { x.unwrap() }")], &lists());
        assert!(!ok);
        assert!(text.contains("  1 line(s) in scope, 0 inside"), "{text}");
    }

    #[test]
    fn every_shipped_allowlist_entry_is_a_crate_source_with_a_count() {
        let all = [
            ALLOW_SEARCH,
            ALLOW_FLOAT,
            ALLOW_UNSIZED,
            ALLOW_SORT,
            ALLOW_PANIC,
            ALLOW_DISARM,
            ALLOW_ASSERT,
            ALLOW_SCAN,
            ALLOW_SCAN_UNREAD,
            ALLOW_MEMBER,
        ];
        for list in all {
            let mut seen = BTreeSet::new();
            for (p, n) in list {
                assert!(
                    p.starts_with("crates/") && p.contains("/src/") && p.ends_with(".rs"),
                    "{p}"
                );
                assert!(*n > 0, "{p}");
                assert!(seen.insert(*p), "{p} listed twice");
            }
        }
        assert!(ALLOW_SCAN_UNREAD.is_empty());
        assert!(
            ALLOW_PENDING
                .iter()
                .all(|(id, why)| id_grammar(id) && why.contains("CLOSED BY"))
        );
    }

    #[test]
    fn p_03_left_the_pending_allowlist() {
        // D-2673, carried here by D-1938: the calendar exists and
        // pull::unit::calendar_filter drives it, so the row is checked.
        assert!(ALLOW_PENDING.iter().all(|(id, _)| *id != "P-03"));
        assert!(ALLOW_PENDING.iter().any(|(id, _)| *id == "X-13"));
    }

    #[test]
    fn gate11_allowlists_carry_the_counts_the_merged_code_needs() {
        // D-1958, D-1932 and the zero-work counts, carried by D-1938.
        let count = |l: Allow, f: &str| l.iter().find(|(p, _)| *p == f).map(|(_, n)| *n);
        assert_eq!(count(ALLOW_FLOAT, "crates/pull/src/pricing.rs"), Some(23));
        assert_eq!(count(ALLOW_FLOAT, "crates/runner/src/report.rs"), Some(4));
        assert_eq!(
            count(ALLOW_FLOAT, "crates/runner/src/significance.rs"),
            Some(41)
        );
        assert_eq!(count(ALLOW_MEMBER, "crates/api/src/server.rs"), Some(2));
        assert_eq!(count(ALLOW_MEMBER, "crates/cli/src/lib.rs"), Some(7));
        assert_eq!(
            count(ALLOW_MEMBER, "crates/indicators/src/column.rs"),
            Some(1)
        );
        assert_eq!(count(ALLOW_SCAN, "crates/api/src/sweeprun.rs"), None);
        assert_eq!(count(ALLOW_SORT, "crates/pull/src/folder.rs"), Some(2));
    }
}

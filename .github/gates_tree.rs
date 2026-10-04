//! The tree-reading gates of the `language-purity` job: gates 0, 1, 1e, 1b,
//! 1g, 1f, 1c, 1d, 2, 9, 9b, 10b and 7 of `.github/workflows/ci.yml`
//! (D-2311). This CI-only executable has no dependencies, is built directly
//! by rustc, and proves itself with `rustc --test` in gate 0 before any of
//! those steps trusts an answer from it.
//!
//! Those steps used to decide with shell text processing: inline programs in
//! a text-processing language, stream edits, and `grep`/`cut`/`sort`/`uniq`/
//! `tr` pipelines inside loops. CLAUDE.md section 2 makes Rust the only
//! language outside `web/`, and a program written inline in a tracked
//! workflow is a second one, so every listing, pattern and verdict those
//! steps computed is computed here, and each step only invokes compiled
//! programs.
//!
//! WHAT STAYS IN A STEP, AND WHY. Gate 0's spawn rule (D-2344) lets a
//! `.github/*.rs` tool start `git` and no other program by name, so this tool
//! cannot run `source_scan`. Where a gate needs the scanner, the step runs it
//! between a `prepare` call here that writes its input listing and a
//! `verdict` call that reads its output file and exit status
//! (`st=0; "$scan" .. > out || st=$?`). Any non-zero status is a refusal,
//! never a pass. Gate 1e also keeps its two `cargo` captures in the step,
//! because D-0911 and `c4_cli_02_limits` pin those exact lines.
//!
//! EVERY PATTERN IS MATCHED BY HAND, WITH NO REGEX ENGINE. Each matcher names
//! the extended regular expression it replaces. Where the old answer depended
//! on the runner's locale -- a word boundary, a case-blind match, which
//! characters are space, binary-file detection -- the matcher takes the
//! reading that refuses more, so no locale can make a gate here looser than
//! the shell it replaced. Every listing is NUL-separated, so a path git would
//! have quoted is read as itself.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

// -------------------------------------------------------------- report --

/// What a gate prints, and whether it refused.
#[derive(Debug, Default)]
struct Report {
    lines: Vec<String>,
    refused: bool,
}

impl Report {
    fn say(&mut self, line: impl Into<String>) {
        self.lines.push(line.into());
    }

    fn refuse(&mut self, line: impl Into<String>) {
        self.say(line);
        self.refused = true;
    }

    #[cfg(test)]
    fn text(&self) -> String {
        self.lines.join("\n")
    }
}

/// One `source_scan` run a step made for a verdict here: the standard output
/// it redirected to a file, and the exit status it captured.
#[derive(Debug, Default)]
struct Scan {
    out: String,
    status: i32,
}

impl Scan {
    fn read(path: &str, status: &str) -> Result<Scan, String> {
        let status = status
            .trim()
            .parse::<i32>()
            .map_err(|_| format!("`{status}` is not an exit status"))?;
        Ok(Scan {
            out: read_text(Path::new(path))?,
            status,
        })
    }

    fn ok(&self) -> bool {
        self.status == 0
    }

    fn lines(&self) -> Vec<&str> {
        lines_of(&self.out)
    }
}

// ------------------------------------------------------------- helpers --

/// The lines of `text` as a line tool reads them: split on `\n`, a trailing
/// newline ending the last line rather than opening an empty one, and a
/// carriage return kept as part of its line.
fn lines_of(text: &str) -> Vec<&str> {
    let mut v: Vec<&str> = text.split('\n').collect();
    if v.last() == Some(&"") {
        v.pop();
    }
    v
}

fn read_bytes(p: &Path) -> Result<Vec<u8>, String> {
    std::fs::read(p).map_err(|e| format!("{}: {e}", p.display()))
}

fn read_text(p: &Path) -> Result<String, String> {
    read_bytes(p).map(|b| String::from_utf8_lossy(&b).into_owned())
}

/// `[[:space:]]` in the C locale.
fn ascii_space(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\u{b}' | '\u{c}' | '\r')
}

/// `[[:space:]]` in a UTF-8 locale, where glibc's `iswspace` also holds for
/// the Unicode spaces that are not no-break spaces.
fn unicode_space(c: char) -> bool {
    ascii_space(c)
        || matches!(c, '\u{85}' | '\u{1680}' | '\u{2000}'..='\u{2006}' | '\u{2008}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{205f}' | '\u{3000}')
}

/// Does the pattern `m` match under either reading of `[[:space:]]`? The
/// runner's locale decides which one grep used, so a line either reading
/// refused is refused.
fn either_space(m: impl Fn(&dyn Fn(char) -> bool) -> bool) -> bool {
    m(&ascii_space) || m(&unicode_space)
}

/// A case-blind fold that is a superset of `grep -i` in either locale: ASCII
/// lower case, plus the four characters a UTF-8 locale folds onto ASCII.
fn fold(c: char) -> char {
    match c {
        '\u{17f}' => 's',
        '\u{212a}' => 'k',
        '\u{130}' | '\u{131}' => 'i',
        _ => c.to_ascii_lowercase(),
    }
}

fn folded(s: &str) -> Vec<char> {
    s.chars().map(fold).collect()
}

fn at(c: &[char], i: usize, pat: &str) -> bool {
    (i..).zip(pat.chars()).all(|(k, p)| c.get(k) == Some(&p))
}

fn holds(c: &[char], pat: &str) -> bool {
    (0..c.len()).any(|i| at(c, i, pat))
}

/// `[A-Za-z0-9_]`: ASCII in both locales (rational range interpretation).
fn ident_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

// ------------------------------------------------------------ listings --

fn git(args: &[&str]) -> Result<Vec<u8>, String> {
    let out = Command::new("git")
        .args(args)
        .output()
        .map_err(|e| format!("git {}: {e}", args.join(" ")))?;
    if !out.status.success() {
        return Err(format!(
            "git {} failed ({}): {}",
            args.join(" "),
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(out.stdout)
}

/// A NUL-separated listing as names. A name that is not UTF-8 is refused:
/// gate 1's scanner cannot read such a listing either.
fn nul_names(raw: &[u8]) -> Result<Vec<String>, String> {
    raw.split(|b| *b == 0)
        .filter(|s| !s.is_empty())
        .map(|s| {
            String::from_utf8(s.to_vec()).map_err(|_| {
                format!(
                    "a tracked path is not UTF-8: {}",
                    String::from_utf8_lossy(s)
                )
            })
        })
        .collect()
}

/// `git ls-files -z -- <pathspec>..`. Git does the pathspec matching, so a
/// pattern means exactly what it meant on the shell command line it replaces
/// (`*` crosses `/`, `crates/pull` is a directory prefix).
fn ls_files(pathspecs: &[&str]) -> Result<Vec<String>, String> {
    let mut args = vec!["ls-files", "-z", "--"];
    args.extend_from_slice(pathspecs);
    nul_names(&git(&args)?)
}

fn write_nul(path: &Path, names: &[String]) -> Result<(), String> {
    let mut bytes = Vec::new();
    for n in names {
        bytes.extend_from_slice(n.as_bytes());
        bytes.push(0);
    }
    std::fs::write(path, bytes).map_err(|e| format!("{}: {e}", path.display()))
}

fn read_nul(path: &Path) -> Result<Vec<String>, String> {
    nul_names(&read_bytes(path)?)
}

/// Each tracked file's content, read from the working tree.
fn disk(p: &str) -> Result<Vec<u8>, String> {
    read_bytes(Path::new(p))
}

// -------------------------------------------------------------- gate 0 --

/// The listings gate 0's scanner calls read: every tracked `.yml` under
/// `.github/` (D-2341), and every crate and tool `.rs` for the spawn check.
fn gate_0_prepare(workflows: &[String], spawns: &[String], work: &Path) -> Result<Report, String> {
    let mut r = Report::default();
    if workflows.is_empty() {
        r.refuse("GATE 0 READ NO WORKFLOW.");
        return Ok(r);
    }
    write_nul(&work.join("workflows.z"), workflows)?;
    write_nul(&work.join("spawns.z"), spawns)?;
    Ok(r)
}

fn gate_0_verdict(workflow: &Scan, aggregator: &Scan, spawns: &Scan) -> Report {
    let mut r = Report::default();
    let checks = [
        (
            workflow,
            "GATE 0 FAILED: the workflow lines above are refused. See D-1100.",
        ),
        (
            aggregator,
            "GATE 0 FAILED: ci-ok no longer guards every gate. See D-1601.",
        ),
        (
            spawns,
            "GATE 0 FAILED: crate code above starts a shell or interpreter. D-1603.",
        ),
    ];
    for (scan, why) in checks {
        for l in scan.lines() {
            r.say(l);
        }
        if !scan.ok() {
            r.refuse("");
            r.refuse(why);
        }
    }
    if !r.refused {
        r.say("OK — the scanner passed its own tests and the workflow is clean.");
    }
    r
}

// -------------------------------------------------------------- gate 1 --

/// CLAUDE.md section 2's extensions outside `web/`.
const ALLOWED: [&str; 6] = ["rs", "toml", "md", "lock", "html", "css"];

/// Which list applies is decided by the PATH, not by the file: `.yml` only
/// under `.github/`, `.json` only under `.claude/`, anything under `web/`.
fn extension_ok(f: &str, ext: &str) -> (bool, &'static str) {
    if f.starts_with("web/") {
        return (true, "web/");
    }
    let (extra, place) = if f.starts_with(".github/") {
        (Some("yml"), ".github/")
    } else if f.starts_with(".claude/") {
        (Some("json"), ".claude/")
    } else {
        (None, "outside web/")
    };
    (ALLOWED.contains(&ext) || extra == Some(ext), place)
}

/// One `git ls-files -s` record: `mode SP object SP stage TAB path`.
fn gate_1_record(rec: &str, r: &mut Report) {
    let mode = rec.split(' ').next().unwrap_or("");
    let f = rec.split_once('\t').map_or(rec, |(_, p)| p);
    let base = f.rsplit('/').next().unwrap_or(f);
    let web = f.starts_with("web/");
    // A MODE IS CONTENT TOO (D-1104).
    if !web && mode != "100644" {
        r.refuse(format!(
            "FORBIDDEN MODE  {f}  ({mode}; only 100644 outside web/)"
        ));
    }
    // The old test was a line grep of the basename, so a name holding a
    // newline matched on any of its lines; that reading is kept.
    let named = |names: &[&str]| base.split('\n').any(|l| names.contains(&l));
    if named(&[".gitignore", ".gitattributes"]) {
        if !(f == ".gitignore" || f == ".gitattributes" || web) {
            r.refuse(format!(
                "FORBIDDEN FILE  {f}  ({base} is allowed only at the root)"
            ));
        }
        return;
    }
    if named(&["LICENSE", "CODEOWNERS"]) {
        let home = matches!(
            f,
            "LICENSE" | "CODEOWNERS" | ".github/CODEOWNERS" | "docs/CODEOWNERS"
        );
        if !(home || web) {
            r.refuse(format!(
                "FORBIDDEN FILE  {f}  ({base} is allowed only where it means something)"
            ));
        }
        return;
    }
    let ext = base.rsplit_once('.').map_or("", |(_, e)| e);
    let (ok, place) = extension_ok(f, ext);
    if !ok {
        let shown = if ext.is_empty() { "<none>" } else { ext };
        r.refuse(format!("FORBIDDEN FILE  {f}  (extension {shown}, {place})"));
    }
}

fn gate_1_verdict(staged: &[u8], content: &Scan, orphans: &Scan) -> Result<Report, String> {
    let mut r = Report::default();
    let records = nul_names(staged)?;
    for rec in &records {
        gate_1_record(rec, &mut r);
    }
    if records.is_empty() {
        r.refuse("GATE 1 READ NO TRACKED FILE. A gate that reads nothing is not a gate.");
        return Ok(r);
    }
    r.say(format!("read {} tracked file(s)", records.len()));
    for l in content.lines() {
        r.say(l);
    }
    if !content.ok() {
        r.refuse("FORBIDDEN CONTENT above. See CLAUDE.md section 2 and D-1104.");
    }
    for l in orphans.lines() {
        r.say(l);
    }
    if !orphans.ok() {
        r.refuse("A TRACKED .rs ABOVE IS COMPILED BY NOTHING, or a module could not");
        r.say("be resolved. Give it a crate root, or remove it. D-1104.");
    }
    if r.refused {
        let all = ALLOWED.join(" ");
        for l in [
            String::new(),
            "This repository is Rust only outside web/. See CLAUDE.md section 2".to_owned(),
            "and docs/05-decisions.md D-0052.".to_owned(),
            format!("Outside web/:  {all}"),
            format!("Under .github/: {all} yml"),
            format!("Under .claude/: {all} json"),
            "Under  web/:    .*".to_owned(),
        ] {
            r.say(l);
        }
    } else {
        r.say("OK — every tracked file is in the allowlist for where it lives, has");
        r.say("     an ordinary mode, carries no content its name hides, and every");
        r.say("     .rs outside web/ is compiled.");
    }
    Ok(r)
}

// ------------------------------------------------------------- gate 1e --

const PY: &str = concat!("py", "thon");

/// Every program a crate could reach for to run front-end or interpreted
/// code, and sh and bash too (D-2344). Each is shadowed on PATH by a copy of
/// this binary that logs its own name and exits 127.
fn stub_names() -> Vec<String> {
    let mut v: Vec<String> = [
        "node", "npm", "npx", "yarn", "pnpm", "bun", "deno", "vite", "webpack", "esbuild",
        "rollup", "tsc",
    ]
    .iter()
    .map(|s| (*s).to_owned())
    .collect();
    v.push(PY.to_owned());
    v.push(format!("{PY}3"));
    for s in ["perl", "ruby", "php", "lua", "luajit", "sh", "bash"] {
        v.push(s.to_owned());
    }
    v
}

/// The stub's log, beside the stubs. Cargo suppresses a build script's
/// stderr unless the script fails, so a `build.rs` that ignores a failed
/// `Command::new("vite").status()` leaves no trace in the captured output; a
/// file on disk is outside cargo's capture.
const INVOKED: &str = ".invoked";

/// Is this process one of the stubs? The copy's own file name says so.
fn stub_identity() -> Option<(String, Option<PathBuf>)> {
    let exe = std::env::current_exe().ok();
    let names = stub_names();
    let base = |p: &Path| p.file_name().and_then(|n| n.to_str()).map(str::to_owned);
    let from_exe = exe.as_deref().and_then(base);
    let from_arg = std::env::args_os().next().and_then(|a| base(Path::new(&a)));
    let name = [from_exe, from_arg]
        .into_iter()
        .flatten()
        .find(|n| names.contains(n))?;
    Some((name, exe))
}

/// Act as the stub `name`: append the name to the log beside `exe` and fail.
fn act_as_stub(name: &str, exe: Option<&Path>) -> u8 {
    if let Some(dir) = exe.and_then(Path::parent) {
        let log = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(dir.join(INVOKED));
        if let Ok(mut f) = log {
            let _ = writeln!(f, "{name}");
        }
    }
    eprintln!("GATE-1E-INVOKED: {name}");
    127
}

/// RO-6, D-2345: a worktree of this commit with `web/` deleted, and the stub
/// directory filled.
fn gate_1e_prepare(tree: &Path, stub: &Path) -> Result<Report, String> {
    let mut r = Report::default();
    let status = Command::new("git")
        .args(["worktree", "add", "--detach"])
        .arg(tree)
        .arg("HEAD")
        .status()
        .map_err(|e| format!("git worktree add: {e}"))?;
    if !status.success() {
        return Err(format!(
            "git worktree add {} HEAD failed ({status})",
            tree.display()
        ));
    }
    let web = tree.join("web");
    if let Ok(meta) = std::fs::symlink_metadata(&web) {
        let gone = if meta.is_dir() {
            std::fs::remove_dir_all(&web)
        } else {
            std::fs::remove_file(&web)
        };
        gone.map_err(|e| format!("{}: {e}", web.display()))?;
    }
    if std::fs::symlink_metadata(&web).is_ok() {
        r.refuse("GATE 1E: web/ was not removed");
        return Ok(r);
    }
    let me = std::env::current_exe().map_err(|e| format!("current_exe: {e}"))?;
    let mut names = stub_names();
    for name in &names {
        let to = stub.join(name);
        std::fs::copy(&me, &to).map_err(|e| format!("{}: {e}", to.display()))?;
        make_executable(&to)?;
    }
    names.sort();
    r.say(format!("shadowed: {} ", names.join(" ")));
    Ok(r)
}

fn make_executable(p: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(p, std::fs::Permissions::from_mode(0o755))
        .map_err(|e| format!("{}: {e}", p.display()))
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Phase {
    Build,
    Test,
}

/// `grep -qiE 'node_modules|yarn\.lock|pnpm-lock|package\.json'`, case-blind.
fn names_package_metadata(line: &str) -> bool {
    let c = folded(line);
    ["node_modules", "yarn.lock", "pnpm-lock", "package.json"]
        .iter()
        .any(|w| holds(&c, w))
}

/// The three clauses, in the order the step read them: no stub invoked, the
/// command succeeded, and no front-end package metadata in its output.
fn gate_1e_verdict(phase: Phase, invoked: Option<&str>, status: i32, out: &str) -> Report {
    let mut r = Report::default();
    let (cmd, during, tail) = match phase {
        Phase::Build => ("cargo build", "the build", "The tail of the build:"),
        Phase::Test => ("cargo test", "the test run", "The tail of the run:"),
    };
    if let Some(log) = invoked.filter(|s| !s.is_empty()) {
        r.refuse(match phase {
            Phase::Build => "FORBIDDEN: a crate invoked a front-end toolchain during the build:",
            Phase::Test => "FORBIDDEN: a crate invoked a front-end toolchain while its tests ran:",
        });
        let names: BTreeSet<&str> = lines_of(log).into_iter().collect();
        for n in names {
            r.say(format!("  {n}"));
        }
        if phase == Phase::Test {
            r.say("The build was clean and the test run was not, which means the");
            r.say("call is in test code rather than in a build script.");
        }
        return r;
    }
    let lines = lines_of(out);
    if status != 0 {
        r.refuse(format!(
            "GATE 1E FAILED: {cmd} exited {status} with the front-end"
        ));
        r.say("toolchain shadowed. §2 requires it to pass on a machine with no");
        r.say(format!("Node and no package manager. {tail}"));
        for l in &lines[lines.len().saturating_sub(40)..] {
            r.say(*l);
        }
        return r;
    }
    let hits: Vec<&&str> = lines.iter().filter(|l| names_package_metadata(l)).collect();
    if !hits.is_empty() {
        r.refuse(format!(
            "FORBIDDEN: {during} read front-end package metadata."
        ));
        for l in hits {
            r.say(*l);
        }
        return r;
    }
    if phase == Phase::Test {
        r.say("OK — the workspace builds AND tests with web/ deleted, every");
        r.say("     front-end tool, interpreter and shell shadowed, and no stub invoked.");
    }
    r
}

// ------------------------------------------------------------- gate 1b --

/// `grep -E '\.(json|yml)$' | grep -Ev '^(\.github/|\.claude/|web/)'`, read
/// per line of each name as the old newline listing was.
fn gate_1b(listing: &[String]) -> Report {
    let mut r = Report::default();
    if listing.is_empty() {
        r.refuse("GATE 1B READ NO TRACKED FILE.");
        return r;
    }
    let bad: Vec<&str> = listing
        .iter()
        .flat_map(|n| n.split('\n'))
        .filter(|l| l.ends_with(".json") || l.ends_with(".yml"))
        .filter(|l| {
            ![".github/", ".claude/", "web/"]
                .iter()
                .any(|p| l.starts_with(p))
        })
        .collect();
    if bad.is_empty() {
        r.say("OK.");
    } else {
        r.refuse("CONFIG OUTSIDE ITS HOME:");
        for b in bad {
            r.say(b);
        }
    }
    r
}

// ------------------------------------------------------------- gate 1g --

const TOOLCHAIN: &str = "rust-toolchain.toml";
const NEXTEST: &str = ".config/nextest.toml";

/// `(^|/)\.cargo(/|$)`: a `.cargo` component anywhere.
fn under_cargo_dir(line: &str) -> bool {
    line.split('/').any(|c| c == ".cargo")
}

/// `^\.config/` other than the one nextest file, and a tool file by its
/// basename (`(^|/)(rust-toolchain(\.toml)?|nextest\.toml)$`) other than at
/// its one place.
fn misplaced_tool_file(line: &str) -> bool {
    let config = line.starts_with(".config/") && line != NEXTEST;
    let base = line.rsplit('/').next().unwrap_or(line);
    let tool = matches!(
        base,
        "rust-toolchain" | "rust-toolchain.toml" | "nextest.toml"
    ) && line != TOOLCHAIN
        && line != NEXTEST;
    config || tool
}

/// `toolchain|toolchain\.(channel|components|targets|profile)`, whole line.
fn toolchain_key(k: &str) -> bool {
    matches!(
        k,
        "toolchain"
            | "toolchain.channel"
            | "toolchain.components"
            | "toolchain.targets"
            | "toolchain.profile"
    )
}

/// `profile\.[A-Za-z0-9_-]+\.overrides\[\](\.(filter|priority))?`, whole line.
fn nextest_key(k: &str) -> bool {
    let Some(rest) = k.strip_prefix("profile.") else {
        return false;
    };
    let Some((name, tail)) = rest.split_once('.') else {
        return false;
    };
    !name.is_empty()
        && name.chars().all(|c| ident_char(c) || c == '-')
        && matches!(
            tail,
            "overrides[]" | "overrides[].filter" | "overrides[].priority"
        )
}

/// `s/^[^:]*:[0-9]+:([^ ]*) = .*$/\1/` on one `source_scan toml` line: the
/// key, or the line itself when it does not have that shape.
fn toml_key(line: &str) -> &str {
    let shaped = || -> Option<&str> {
        let (_, rest) = line.split_once(':')?;
        let (num, rest) = rest.split_once(':')?;
        if num.is_empty() || !num.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        let key = &rest[..rest.find(' ').unwrap_or(rest.len())];
        rest[key.len()..].starts_with(" = ").then_some(key)
    };
    shaped().unwrap_or(line)
}

/// A tool file may carry only the keys its allowlist names.
fn check_keys(file: &str, present: bool, keys: &Scan, allow: fn(&str) -> bool, r: &mut Report) {
    if !present {
        return;
    }
    if !keys.ok() {
        r.refuse(format!("REFUSED  {file} is not TOML this gate can read"));
        return;
    }
    let lines: Vec<&str> = keys.lines().into_iter().filter(|l| !l.is_empty()).collect();
    let extra: Vec<&str> = lines
        .iter()
        .map(|l| toml_key(l))
        .filter(|k| !allow(k))
        .collect();
    if extra.is_empty() {
        r.say(format!(
            "ok       {file} — {} key(s), all allowed",
            lines.len()
        ));
    } else {
        r.refuse(format!("REFUSED  {file} sets a key outside its allowlist:"));
        for e in extra {
            r.say(format!("           {e}"));
        }
    }
}

/// After `NAME`: `[[:space:]]*[:=]`.
fn then_sets(c: &[char], mut i: usize, space: &dyn Fn(char) -> bool) -> bool {
    while c.get(i).is_some_and(|x| space(*x)) {
        i += 1;
    }
    matches!(c.get(i), Some(':' | '='))
}

const WRAPPERS: [&str; 11] = [
    "RUSTC_WRAPPER",
    "RUSTC_WORKSPACE_WRAPPER",
    "CARGO_BUILD_RUSTC_WRAPPER",
    "CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER",
    "CARGO_BUILD_RUSTC",
    "CARGO_BUILD_RUSTDOC",
    "RUSTC",
    "RUSTDOC",
    "RUSTC_LINKER",
    "RUSTUP_TOOLCHAIN",
    "RUSTUP_HOME",
];

/// `(^|[^A-Z0-9_])(<WRAPPERS>|CARGO_TARGET_[A-Z0-9_]+_(RUNNER|LINKER))([^A-Za-z0-9_]|$)`:
/// the NAME in any position, not only before `[:=]` (P15-08, D-2322). A
/// `printf '%s=%s\n' RUSTC_WRAPPER /tmp/z >> "$GITHUB_ENV"`, or a value
/// assembled on one line and written to `GITHUB_ENV` by a sanctioned line
/// later, sets the variable without the name ever being followed by `=`. No
/// workflow line names one of these for any other reason. The left boundary
/// is an upper-case environment-name character, so `$'x\nRUSTC=y'` (the
/// escape's `n` before the name) is read too, and `MY_RUSTC` is still another
/// variable. `[A-Z0-9_]+` is maximal, so the character after `_RUNNER` is
/// never one it could absorb.
fn names_wrapper(c: &[char]) -> bool {
    let env_char = |x: char| x.is_ascii_uppercase() || x.is_ascii_digit() || x == '_';
    let ends = |k: usize| !c.get(k).is_some_and(|x| ident_char(*x));
    (0..c.len()).any(|i| {
        if i > 0 && env_char(c[i - 1]) {
            return false;
        }
        if WRAPPERS
            .iter()
            .any(|w| at(c, i, w) && ends(i + w.chars().count()))
        {
            return true;
        }
        if !at(c, i, "CARGO_TARGET_") {
            return false;
        }
        let from = i + "CARGO_TARGET_".len();
        let mut end = from;
        while c.get(end).is_some_and(|x| env_char(*x)) {
            end += 1;
        }
        let run: String = c[from..end].iter().collect();
        run.len() >= 8 && (run.ends_with("_RUNNER") || run.ends_with("_LINKER")) && ends(end)
    })
}

/// The text after each `word` up to the first character of `stops`: the
/// reach of `word[^<stops>]*`.
fn reaches<'a>(c: &'a [char], word: &str, stops: &[char]) -> Vec<&'a [char]> {
    let n = word.chars().count();
    (0..c.len())
        .filter(|&i| at(c, i, word))
        .map(|i| {
            let from = i + n;
            let to = (from..c.len())
                .find(|&k| stops.contains(&c[k]))
                .unwrap_or(c.len());
            &c[from..to]
        })
        .collect()
}

/// `cargo[^#|;]*[[:space:]]--config[[:space:]=]`. Every character of the
/// match after `cargo` is outside `#|;`, so it lies inside one reach.
fn cargo_config(c: &[char], space: &dyn Fn(char) -> bool) -> bool {
    reaches(c, "cargo", &['#', '|', ';']).iter().any(|seg| {
        (0..seg.len()).any(|k| {
            space(seg[k])
                && at(seg, k + 1, "--config")
                && seg.get(k + 9).is_some_and(|x| space(*x) || *x == '=')
        })
    })
}

/// `(^|[[:space:];&|(])(export|declare|typeset|readonly|local|env)([[:space:]]+-[A-Za-z]+)*[[:space:]]+[^=[:space:]]*\$[^=[:space:]]*=`:
/// an environment NAME assembled from a shell variable.
fn computed_name(c: &[char], space: &dyn Fn(char) -> bool) -> bool {
    for i in 0..c.len() {
        if i > 0 && !(space(c[i - 1]) || matches!(c[i - 1], ';' | '&' | '|' | '(')) {
            continue;
        }
        for kw in ["export", "declare", "typeset", "readonly", "local", "env"] {
            if !at(c, i, kw) {
                continue;
            }
            let mut p = i + kw.len();
            loop {
                let s = p;
                while c.get(p).is_some_and(|x| space(*x)) {
                    p += 1;
                }
                if p == s {
                    break;
                }
                // The final word: a run of non-`=` non-space with a `$` in it,
                // ended by `=`.
                let mut q = p;
                let mut dollar = false;
                while c.get(q).is_some_and(|x| *x != '=' && !space(*x)) {
                    dollar |= c[q] == '$';
                    q += 1;
                }
                if dollar && c.get(q) == Some(&'=') {
                    return true;
                }
                // Or one more `-flags` group, which a space must follow.
                if c.get(p) != Some(&'-') {
                    break;
                }
                let mut f = p + 1;
                while c.get(f).is_some_and(char::is_ascii_alphabetic) {
                    f += 1;
                }
                if f == p + 1 || !c.get(f).is_some_and(|x| space(*x)) {
                    break;
                }
                p = f;
            }
        }
    }
    false
}

/// The step's first pattern: a compiler wrapper, runner or linker by its
/// environment name, or `cargo --config`. Read on comment lines too.
fn wrapper_line(line: &str) -> bool {
    let c: Vec<char> = line.chars().collect();
    names_wrapper(&c) || either_space(|sp| cargo_config(&c, sp))
}

/// D-1612 and D-2346: the same doors by other names. Comment lines skipped.
fn other_door_line(line: &str) -> bool {
    let c: Vec<char> = line.chars().collect();
    if c.iter().find(|x| !ascii_space(**x)) == Some(&'#') {
        return false;
    }
    let home = |sp: &dyn Fn(char) -> bool| {
        (0..c.len()).any(|i| {
            (i == 0 || !ident_char(c[i - 1]))
                && at(&c, i, "CARGO_HOME")
                && then_sets(&c, i + "CARGO_HOME".len(), sp)
        })
    };
    let flags = ["RUSTFLAGS", "RUSTDOCFLAGS"].iter().any(|f| {
        reaches(&c, f, &['#']).iter().any(|seg| {
            [
                "linker",
                "link-arg",
                "fuse-ld",
                "runtool",
                "link-self-contained",
            ]
            .iter()
            .any(|w| holds(seg, w))
        })
    });
    let runtool = holds(&c, "--runtool") || holds(&c, "--test-runtool");
    let nextest = reaches(&c, "nextest", &['#', '|', ';'])
        .iter()
        .any(|seg| holds(seg, "--config-file") || holds(seg, "--tool-config-file"));
    let printf_v = |sp: &dyn Fn(char) -> bool| {
        (0..c.len()).any(|i| {
            if !at(&c, i, "printf") {
                return false;
            }
            let mut p = i + 6;
            while c.get(p).is_some_and(|x| sp(*x)) {
                p += 1;
            }
            p > i + 6 && at(&c, p, "-v")
        })
    };
    flags
        || runtool
        || nextest
        || holds(&c, "GITHUB_PATH")
        || either_space(|sp| home(sp) || computed_name(&c, sp) || printf_v(sp))
}

/// `printf .(SOURCE_SCAN|CARGO_TARGET_DIR)=%s\\n. "\$[a-z_]+" >> "\$GITHUB_ENV"$`.
/// Each `.` must be one ASCII character: in the C locale `.` is one byte, so
/// a wider character there was refused, and it still is.
fn sanctioned_env_write(rest: &str) -> bool {
    let c: Vec<char> = rest.chars().collect();
    if !at(&c, 0, "printf ") || !c.get(7).is_some_and(|x| x.is_ascii() && *x != '\n') {
        return false;
    }
    let mut p = 8;
    let Some(name) = ["SOURCE_SCAN", "CARGO_TARGET_DIR"]
        .iter()
        .find(|n| at(&c, p, n))
    else {
        return false;
    };
    p += name.len();
    if !at(&c, p, "=%s\\n") {
        return false;
    }
    p += 5;
    if !c.get(p).is_some_and(|x| x.is_ascii() && *x != '\n') {
        return false;
    }
    p += 1;
    if !at(&c, p, " \"$") {
        return false;
    }
    p += 3;
    let from = p;
    while c
        .get(p)
        .is_some_and(|x| x.is_ascii_lowercase() || *x == '_')
    {
        p += 1;
    }
    let tail = "\" >> \"$GITHUB_ENV\"";
    p > from && at(&c, p, tail) && p + tail.len() == c.len()
}

/// `GITHUB_ENV` anywhere, unless the line is a comment, the one guard, or one
/// of the two sanctioned writes.
fn env_file_line(line: &str) -> bool {
    if !line.contains("GITHUB_ENV") {
        return false;
    }
    let rest = line.trim_start_matches(ascii_space);
    !(rest.starts_with('#')
        || rest == "if [ -n \"${GITHUB_ENV:-}\" ]; then"
        || sanctioned_env_write(rest))
}

/// Every workflow line that sets a compiler wrapper, runner or linker, in the
/// step's order: the first pattern over every file, then the second, then the
/// third.
fn workflow_doors(workflows: &[(String, String)]) -> Vec<String> {
    let mut out = Vec::new();
    let rules: [fn(&str) -> bool; 3] = [wrapper_line, other_door_line, env_file_line];
    for rule in rules {
        for (name, text) in workflows {
            for (n, l) in lines_of(text).into_iter().enumerate() {
                if rule(l) {
                    out.push(format!("{name}:{}:{l}", n + 1));
                }
            }
        }
    }
    out
}

struct ToolFiles<'a> {
    toolchain: (bool, &'a Scan),
    nextest: (bool, &'a Scan),
}

fn gate_1g_verdict(
    listing: &[String],
    tools: &ToolFiles,
    workflows: &[(String, String)],
) -> Report {
    let mut r = Report::default();
    if listing.is_empty() {
        r.refuse("GATE 1G READ NO TRACKED FILE.");
        return r;
    }
    let lines: Vec<&str> = listing.iter().flat_map(|n| n.split('\n')).collect();
    let cargo: Vec<&&str> = lines.iter().filter(|l| under_cargo_dir(l)).collect();
    if !cargo.is_empty() {
        r.refuse("REFUSED  a tracked cargo configuration:");
        for l in cargo {
            r.say(*l);
        }
    }
    let misplaced: Vec<&&str> = lines.iter().filter(|l| misplaced_tool_file(l)).collect();
    if !misplaced.is_empty() {
        r.refuse("REFUSED  a tool configuration outside its one place:");
        for l in misplaced {
            r.say(*l);
        }
    }
    check_keys(
        TOOLCHAIN,
        tools.toolchain.0,
        tools.toolchain.1,
        toolchain_key,
        &mut r,
    );
    check_keys(
        NEXTEST,
        tools.nextest.0,
        tools.nextest.1,
        nextest_key,
        &mut r,
    );
    if workflows.is_empty() {
        r.refuse("REFUSED  no tracked workflow was read");
    }
    let doors = workflow_doors(workflows);
    if !doors.is_empty() {
        r.refuse("REFUSED  a workflow sets a compiler wrapper, runner or linker:");
        for d in doors {
            r.say(d);
        }
    }
    if r.refused {
        r.say("");
        r.say("GATE 1G FAILED. See D-1105.");
    } else {
        r.say("OK — no tracked configuration names a program for cargo, rustup or");
        r.say("     nextest to run.");
    }
    r
}

// ------------------------------------------------------------- gate 1f --

fn gate_1f_verdict(files: usize, browser: &Scan) -> Report {
    let mut r = Report::default();
    if !browser.ok() {
        r.refuse("BROWSER CODE IN A RUST FILE:");
        for l in browser.lines() {
            r.say(l);
        }
    }
    r.say(format!("scanned {files} tracked .rs file(s) under crates/"));
    if r.refused {
        for l in [
            "",
            "Browser code belongs under web/, which CLAUDE.md section 2",
            "makes unrestricted. Serve it the way /typeahead.js and",
            "/masters.js are served: a file on disk, read at REQUEST time by",
            "assets::Assets, referenced as <script src=\"...\" defer>. Never",
            "include_str! and never inline, so no crate depends on the front",
            "end to BUILD -- which is the other half of the same section.",
        ] {
            r.say(l);
        }
    } else {
        r.say("OK — no <script> body, no browser API and no inline handler");
        r.say("     appears in any .rs production region under crates/.");
    }
    r
}

// ------------------------------------------------------------- gate 1c --

/// The ten environment words a real parameter path's `<env>` segment takes.
const ENV_WORDS: [&str; 10] = [
    "prod",
    "production",
    "dev",
    "development",
    "stage",
    "staging",
    "uat",
    "qa",
    "sandbox",
    "live",
];

/// `grep -iE '/[A-Za-z0-9_-][A-Za-z0-9_.-]*/(<ENV_WORDS>)/'`. The segment's
/// class excludes `/`, so it runs exactly from one slash to the next.
fn credential_path(line: &str) -> bool {
    let c = folded(line);
    let first = |x: char| x.is_ascii_lowercase() || x.is_ascii_digit() || x == '_' || x == '-';
    (0..c.len()).any(|i| {
        if c[i] != '/' || !c.get(i + 1).is_some_and(|x| first(*x)) {
            return false;
        }
        let mut j = i + 2;
        while c.get(j).is_some_and(|x| first(*x) || *x == '.') {
            j += 1;
        }
        c.get(j) == Some(&'/')
            && ENV_WORDS
                .iter()
                .any(|w| at(&c, j + 1, w) && c.get(j + 1 + w.len()) == Some(&'/'))
    })
}

fn gate_1c(tracked: &[String], read: &dyn Fn(&str) -> Result<Vec<u8>, String>) -> Report {
    let mut r = Report::default();
    if tracked.is_empty() {
        r.refuse("GATE 1C READ NO TRACKED FILE.");
        return r;
    }
    let mut hits = Vec::new();
    for f in tracked {
        match read(f) {
            Ok(bytes) => {
                let text = String::from_utf8_lossy(&bytes);
                for (n, l) in lines_of(&text).into_iter().enumerate() {
                    if credential_path(l) {
                        hits.push(format!("{f}:{}:{l}", n + 1));
                    }
                }
            }
            Err(e) => hits.push(format!("UNREADABLE {e}")),
        }
    }
    if !hits.is_empty() {
        r.refuse("LITERAL CREDENTIAL PATH IN A TRACKED FILE:");
        for h in hits {
            r.say(h);
        }
        r.say("");
        r.say("Use the shape /<org>/<env>/<vendor>/<field>. The real");
        r.say("segments are resolved at runtime. See D-0013.");
        return r;
    }
    // The runtime configuration itself is never tracked:
    // `(^|/)credentials\.toml$` on every line of the listing.
    let config: Vec<&str> = tracked
        .iter()
        .flat_map(|n| n.split('\n'))
        .filter(|l| *l == "credentials.toml" || l.ends_with("/credentials.toml"))
        .collect();
    if !config.is_empty() {
        r.refuse("CREDENTIAL CONFIG IS TRACKED:");
        for c in config {
            r.say(c);
        }
        return r;
    }
    r.say("OK — no literal credential path, no tracked credential config.");
    r
}

// ------------------------------------------------------------- gate 1d --
//
// Every literal under crates/pull that has the shape of a parameter-path
// segment must be declared below. Each group says why its words are not a
// segment; declaring a literal is that claim, made by hand. These lists and
// their reasons moved here from the step's shell variables unchanged (D-2311).

const SEG_SHAPE: &str = "org env region vendor fields ap-south-1";

// ---- group 2: vendor wire names. THE ONES THIS GATE EXISTS FOR.
// Each is a data vendor's own public product name, written down here
// as the name its bars are filed under — `Descriptor::wire` in
// crates/pull/src/vendor.rs. `groww` and `dhan` carry the reasoning
// above. `truedata` and `gdfl` are the same case and were checked the
// same way before being added: both are already tracked and public in
// docs/08-vendor-samples.md and docs/05-decisions.md, so removing
// them from crates/pull would leak nothing back. Whether either is
// ALSO an operator's real `<vendor>` segment is operator-side and
// unknowable from inside this repository — docs/06-limits.md §18.
// Note for the reader: CLAUDE.md §8 says crates/pull "holds no org,
// env, or vendor literal". Four vendor literals are on this line. The
// sentence and the tree disagree and the tree is not being hidden.
// charts, v1 and v2 joined as URL PATH COMPONENTS of a vendor's candle
// endpoint: PathSegment::Literal("charts") at vendor.rs:3640 and the
// two API version segments beside it at 3639 and 3837. They name a
// route on a broker's public API — not an account, not an environment,
// not a field — and gate 1c passes on the same tree. Read before
// adding: the question this list answers is whether the word is a
// VENDOR'S, and these three are its URL and nothing else.
// `instruments` joins for the same reason `charts` did: it is a
// `PathSegment::Literal` in a vendor's URL (vendor.rs:4330), a route on a
// public API rather than anything about an account.
const VENDOR_WIRE: &str = "groww dhan truedata gdfl charts v1 v2 instruments";

// ---- group 2b: the CLAIM STANDINGS, which are wire words about WHO
// made a claim and are not vendor, org, env or field names at all.
//
// `pull::vendor::ClaimStanding::word` emits `vendor_doc` and
// `operator` on `/feeds.json` so a page can say which of two
// disagreeing history floors binds without matching prose against the
// string "the operator" — D-0131. `vendor_doc` trips this gate for
// the same reason `groww` does: it is lower-case, underscored and
// segment-shaped. It names a KIND OF SOURCE, never a path segment,
// and no Parameter Store path this repository reads has a `vendor_doc`
// or an `operator` component.
//
// Declared rather than excluded by pattern: an allowlist that grows by
// rule stops being an allowlist, which is the failure mode group 1's
// own comment names.
const CLAIM_STANDING: &str = "vendor_doc operator";

// ---- group 3: invented configuration fixtures. Nonsense by
//   1 2 3 9 -- FOUR BARE DIGITS, and each was read before it was added.
//   "1", "2" and "9" are invented vendor_id values in resolve.rs and
//   universe.rs test rows beside invented ISINs (INE000A01001). "3" is
//   the VALUE of the X-Kite-Version header in vendor.rs -- a public API
//   version number, not a credential. Group 10's note explains why an
//   all-digit literal is scanned rather than waved through: a twelve-
//   digit account number is a perfectly ordinary <org> segment. These
//   four are one digit each and name no account.
// construction — no operator's path reads `orgone/testenv/vendorone`.
// 5633 and 999 sit beside 1 2 3 9: invented instrument_id and vendor_id
// values in http.rs and universe.rs fixtures, next to invented ISINs.
const CONFIG_FIXTURE: &str = "
    orgone testenv vendorone vendortwo fieldone fieldtwo fieldthree
    field-one_2 a-value
    1 2 3 9 5633 999
";

// ---- group 4: deliberately secret-SHAPED fixtures, which exist to
// prove the secret backstop rejects them. Keyboard walks and the
// oldest joke password in the trade; none is a credential.
const SECRET_FIXTURE: &str = "
    k7f2p9q1w8e3-5t6y0u4i2o9 k7f2p9q1w8e3r5t6y0u4i2o
    k7f2p9q1w8e3r5t6y0u4i2o9 kfpqwerstyuioasdfghjklzx
    hunter2 hunter2-and-then-some stale fresh 9876
    shhh the-token default-secret other-token
";

// ---- group 5: test scaffolding — expect() messages, assert labels,
// fixture names and words quoted inside prose. The gate scans comments
// on purpose, so a word quoted in a doc comment lands here too.
// `brutex-no-such-dir-ever` is a path chosen to NOT exist.
//
// The last two lines are the socket and decoder tests in
// crates/pull/src/http.rs, and every one of them is a word this
// repository chose rather than one a vendor did:
//   x            — a JSON string standing where a number belongs
//   nz snapshots — fold.rs refusal text asserted back
//   decodes unchanged records — expect() messages
//   cached       — the SECOND object in the two-object splice fixture
//                  (D-0049). Deliberately not `live`, which group 7
//                  already carries for a different reason.
//   stolen       — the path a test redirect points at, named to read
//                  as what following it would have meant (D-0050)
//   redirect window_async — substrings a refusal must contain
// before broker censused declares indistinguishable mixed nested options
// too-deep u unused -- scratch directories and assertion substrings,
// every one inside a test. folder.rs builds directories with those
// names; nse.rs and resolve.rs assert that refusal messages CONTAIN
// before, declares and indistinguishable; http.rs passes u and unused as
// throwaway URL arguments it never reads.
const SCAFFOLD: &str = "
    a v one two no ok big good invalid genesis manifest write read-only deny
    anything mkdir notutf8 atthebound brutex-pull june july nifty
    core store pull ratio
    real next small bounded three summed everything february
    non-zero non-decreasing held centuries negative per-day per-second
    brutex-no-such-dir-ever
    x nz snapshots decodes unchanged records
    cached stolen redirect window_async
    b h 2026 half nope other reads signs forwards readable
    bars empty idempotent lands paise shared lock-held
    brutex-ssm-no-such-file-ever
    before broker censused declares indistinguishable mixed nested options
    too-deep u unused
    ab archive-extra-second broker-clean broker-conflict broker-duplicate
    broker-extra-second broker-shifted cash-cas cash-ordinary
    cash-unknown-a cash-unknown-b cash-unmeasured-calendar ceiling client
    coarse-unverified conflicting correction daily-midnight daily-open date
    elsewhere five-minute flag fold-absent-bucket fold-absent-tail gap-receipt
    gapfill headers historical-candidate-hole historical-incomplete
    historical-partial historical-tail historical-unverified-schedule
    incomplete local-ist loopback millisecond must-not-be-created not-a-date
    not-a-price okay receipt record request request-cash-eligibility
    request-derivative-exempt request-empty request-exception request-holiday
    request-intervals request-span request-unmeasured request-unordered
    response resume runtime runtime-truncated runtime-unverified shared-archive
    short test trailing true unavailable unreadable-rows versioned whole width
";
// D-0610: reviewed fixture symbols and version-refusal inputs.
const SCAFFOLD_D0610: &str = "
    abb forcemot irfc
    brutex-nse-cash-session-v1 brutex-nse-cash-session-v2
";

// ---- group 6: granularity and session tokens. `Granularity::dir`
// and `SessionKind::label` in crates/pull/src/vendor.rs. These name a
// bar interval and are the STABLE ON-DISK DIRECTORY NAMES, which
// CLAUDE.md §3 rule 8 forbids renaming.
//
// `1d` and `day` are NOT directory names and never reach the disk.
// They appear in a doc comment on `Granularity::wire` that explains
// why the wire word is DECLARED per vendor rather than derived from
// the directory name: `1day`, `1d` and `day` are all real spellings
// used by real brokers for the same rung, so there is no rule that
// turns one into the others and inventing one would be CLAUDE.md
// section 3 rule 1. The gate scans comments on purpose, so prose
// about a spelling lands here exactly as a constant would -- which is
// the correct behaviour, because a real segment pasted into a doc
// comment publishes it just as effectively.
//
// `60min` REPLACED `1hr` ON 14 AUG 2026, AND THE PARAGRAPH ABOVE IS
// WHY THAT NEEDED AN ARGUMENT RATHER THAN AN EDIT.
//
// It says these are the stable on-disk directory names and that §3
// rule 8 forbids renaming one. `1hr` was NOT one. It was
// `Granularity::dir`'s word for the hour rung, and the directory is
// created from `store::path::Timeframe`, whose word for that rung has
// been `60min` since the store was widened. The two disagreed, and
// nothing noticed because `Granularity::store_timeframe` answered
// `None` for the rung -- so no bar could ever be written under either
// spelling and `1hr` never named a directory on any disk.
//
// Rule 8 protects HISTORY. There is none here to protect: the rename
// orphans nothing, because nothing was ever filed. What it removes is
// a second name for one rung, which `web/src/routes/db/+page.svelte`
// had been absorbing by hand in an alias table. `1hr` stays on this
// list -- the doc comment recording the rename quotes it, and a word
// this gate has already seen must not start failing because the code
// explaining it was written. D-0132.
const TIMEFRAME: &str = "
    tick 1s 5s 1min 2min 3min 5min 10min 15min 30min 60min 1hr 1day 1week continuous
    1d day intraday
";

// ---- group 7: rate-governor display strings, crates/pull/src/rate.rs.
// `WindowSpan::{Second,Minute,Day}` and `RequestKind::{Live,
// Historical}` rendering themselves for a log line or a refusal.
// `live` IS one of the ten environment words gate 1c matches on, and
// that is why it was looked at rather than waved through: here it is
// the live-quote endpoint group as opposed to the historical one, in
// a two-variant enum, and it is joined to no path.
const GOVERNOR: &str = "second minute day live historical";

// ---- group 8: the vendors' own record field names. CLAUDE.md §8
// says in as many words that crates/pull "holds the shape and the
// field names". `FieldNames`/`ResponseShape` in vendor.rs, the
// column labels fetch.rs quotes back in a refusal, and the file
// extension archive.rs checks. None is a credential field.
//
// `data` is an ENVELOPE key — the object a vendor hangs its bars
// under, exactly as `payload` above already is, and since D-0049 the
// decoder reads only the one the descriptor names. `from` and `to`
// are the two query parameters a GET vendor takes its window in
// (`Method::Get` in http.rs). All three are wire vocabulary, which is
// the category this group exists for.
// symbol interval p success -- each read against its own call site.
// symbol is an NSE record field (nse.rs:632); interval a vendor query
// placeholder (vendor.rs:4337); p the page parameter NSE endpoints take
// (nse.rs:1057); success a status VALUE in a canned response body
// (http.rs:1744). All four are vendor vocabulary on the wire. None names
// an account, an environment or a credential field.
const RECORD_FIELD: &str = "
    open high low close volume timestamp open_interest payload csv
    data from to candles
    status error code message
    symbol interval p success
    duplicates
";

// ---- group 9: an HTTP HEADER NAME, not a value. One broker sends
// its token in a header called `access-token`; the token itself is
// read from Parameter Store and never appears in this repository.
// Read this line twice before adding anything to it.
//
// `x-token` was read twice. It is the header name on the INVENTED
// descriptor the http.rs tests are built from — `x-` is the
// conventional prefix for a name no registry owns, and no vendor in
// crates/pull/src/vendor.rs uses it. The other broker's header is
// `Authorization`, which this gate never sees because its capital
// letter puts it outside the lower-case pattern being scanned; that
// is the scan's shape, not an exemption, and the descriptor spells it
// that way because that is the name on the wire.
const HEADER_NAME: &str = "access-token x-token";

// ---- group 10: bare integers a refusal message must contain, so the
// test asserts the refused value is IN the message. These are domain
// bounds — u32::MAX, the year-9999 epoch-millisecond ceiling, the two
// lengths of February. All-digit literals are NOT excluded from the
// scan by shape, deliberately: a twelve-digit account number is a
// perfectly ordinary `<org>` segment, and `253402281000` is twelve
// digits. It is on this list because it was read, not because a
// pattern let it past.
const BOUND_NUMBER: &str = "
    10000 1970 9999 28 29 4294967295 253402281000 48 750
    0 1e300
";
// D-0610: fixture IDs, malformed dates/flags, timestamps and bounds.
const BOUND_NUMBER_D0610: &str = "
    00000000 01 12345 1250640000 1296345600
    1296345600000 1296345601 1333 1577923200 18446744073709551616
    20210129 2029 2147483648 502070400 86400
";

// ---- group 11: FIELD LABELS IN A `Debug` RENDERING, not values.
// `HttpSource`'s Debug impl is hand-written precisely so the token is
// never printed, and these two strings are the captions it prints
// instead: `base_url` beside the descriptor's URL, and `token` beside
// the fixed string `<redacted>`. `token` is on an allowlist in a
// credential gate and that is worth stopping on — it is the word
// "token", four characters, standing where a value would be if this
// impl were derived. That it appears here is the evidence the redact
// is in place, not evidence of a leak.
//
// `key` is the third caption, and the paragraph above argues it
// unchanged: it is the word standing where a value would be if this
// impl were derived, and the value beside it is the fixed string
// `<redacted>` — crates/pull/src/http.rs:87 reads
// `.field("key", &self.key.as_ref().map(|_| "<redacted>"))`. Its
// presence on this list is the evidence the redact exists, not
// evidence of a leak, which is exactly what the note about `token`
// says and is the reason both are read rather than waved through.
const DEBUG_LABEL: &str = "base_url token key";

// ---- group 12: DATES ON THE WIRE, in the two compact formats.
// `DateFormat::{CompactYmd,CompactDmy}` render 2025-07-01 as
// `20250701` and `01072025`, and the test that pins all four formats
// asserts both. Group 10's note explains why all-digit literals are
// scanned rather than waved through, and the `YYYY-MM-DD` exclusion
// above is anchored to that one spelling on purpose — so these two
// are declared here by hand. Both are the same calendar date this
// repository already writes in the clear a dozen times over; neither
// names an account, an environment, a vendor or a field.
const WIRE_DATE: &str = "20250701 01072025";

// ---- group 13: AWS SigV4's own wire vocabulary, crates/pull/src/ssm.rs.
// Every word here is published by AWS and fixed by the signing
// algorithm. None is an operator's choice, so none can be one of their
// path segments. `ssm` is the service code that appears in the
// credential scope; `aws4_request` is the terminator the algorithm
// mandates; the rest are HTTP header names AWS defines.
//
// `us-east-1`, `20150830` and `20150831` are lifted VERBATIM from
// AWS's own published SigV4 test vectors, which is what ssm.rs checks
// the implementation against — they are the only external proof this
// repository has that the signature is computed correctly, and
// changing them to something invented would delete that proof.
// CLAUDE.md §8 names ap-south-1 as the region this repository reads
// from; us-east-1 appears nowhere but inside those vectors.
const AWS_SIGV4: &str = "
    ssm aws4_request authorization content-type host
    x-amz-date x-amz-target x-amz-security-token
    us-east-1 20150830 20150831
";

// ---- group 14: the AWS shared-credentials file format. These are the
// INI keys and the default profile name AWS itself defines for
// ~/.aws/credentials, read by `AwsIdentity::from_credentials_file`.
// Only the names of the keys are here; gate 1c already refuses the
// file itself from ever being tracked.
const AWS_SHARED: &str = "
    credentials profiles default
    aws_access_key_id aws_secret_access_key aws_session_token
";

// ---- group 15: the redacting `Debug`'s own field labels.
// `AwsIdentity` hand-writes `Debug` so the secret and the session
// token render as `<redacted>`; these are the labels printed beside
// them. That `secret` and `session_token` appear as literals AT ALL is
// that redaction working, not a value escaping.
const REDACT_LABEL: &str = "key_id secret session_token";

// ---- group 16: published hash test vectors. `abc` is the NIST
// SHA-256 input whose digest ssm.rs pins, and `000fff` is the
// three-byte lower-case hex-padding check beside it.
const HASH_VECTOR: &str = "abc 000fff";

// ---- group 17: the vendors' REQUEST field names, and one published
// example value. Group 8 above carries the names each vendor uses in
// its RESPONSE; these are the names it requires in the REQUEST, and
// they arrived with the parameter map that stops Dhan answering
// DH-905. Every one is read first-hand from the vendor's own
// documentation page:
//   securityId exchangeSegment instrument   dhanhq.co/docs/v2/historical-data
//   exchange segment trading_symbol start_time end_time
//                                             groww.in/trade-api/docs
// `exchange`, `segment` and `instrument` are ordinary English words a
// vendor chose for a field; they name no account and no environment.
// `13` is the securityId of NIFTY in Dhan's OWN worked example, used
// in a test so the request can be asserted against a value the vendor
// published rather than one invented here.
//
// The twelve month TOKENS are the calendar, lower-cased. Groww writes a
// contract expiry as `04Jan24` and `pull::fno::read_contract` reads it
// into this store's `2024-01-04`, so the table it compares against is
// the twelve English month abbreviations and nothing else. `19200` is
// the strike in the vendor's OWN worked example, used in the tests so
// the paisa conversion is asserted against a number the vendor
// published rather than one invented here -- the same reason Dhan's
// `13` is on this list. `2020` is the year Groww states its F&O history
// begins and `2019` is the year one before it, asserted in the refusal
// so the message names BOTH ends of the gap an operator just typed --
// a refusal that says only "before the floor" sends them to the
// vendor's dashboard instead of to the field. Two calendar years. A month name is not a secret in any sense
// section 8 is about.
//
// `expiries`, `contracts`, `underlying_symbol`, `expiry_date`, `year`
// and `month` are Groww's expired-F&O contract lookup -- two path
// segments and four query fields, read from
// groww.in/trade-api/docs/curl/backtesting. An expired option is in no
// instrument master, so its name has to be discovered before its bars
// can be asked for; `pull::vendor::FnoDiscovery` is the shape that
// records how. Six ordinary English words a vendor chose. They name no
// org, no env and no account, which is what section 8 is about.
//
// `underlying` joined them as the field key on the `pull.fno discovery
// refused` event -- the one that carries a failed walk's WHOLE reason,
// because the journal's note is stride-bound to 68 bytes and a
// discovery failure's reason begins with the URL, so the vendor's
// status was exactly the part that fell off. It holds an instrument
// name, which is public market vocabulary.
const REQUEST_FIELD: &str = "
    securityId exchangeSegment instrument
    exchange segment trading_symbol start_time end_time
    13
    groww_symbol candle_interval 1minute
    expiries contracts underlying_symbol expiry_date year month underlying
    jan feb mar apr may jun jul aug sep oct nov dec
    19200 2019 2020
";

// ---- group 18: RFC 6238 APPENDIX B, and the words beside it.
// crates/pull/src/totp.rs is an RFC 6238 code generator that nothing
// outside its own tests calls: no path in this build trades a code
// for a token, and nothing uses it to recognise a stale one either
// (a stale token is re-read from Parameter Store, CLAUDE.md section
// 8). It never mints -- section 8 forbids it, because a local mint
// would invalidate the token another system shares; the test
// pull::totp::no_path_outside_this_module_computes_a_code pins
// that, D-1447. This gate seeing these six numbers is the evidence
// that the implementation is pinned to the published vectors rather
// than to whatever it happens to produce.
//
// Every value below is printed in RFC 6238, a public standards
// document, and reproduces from the RFC's own key at the RFC's own
// timestamps. They name no account, no environment, no vendor and no
// field. `gezdgnbvgy3tqojqgezdgnbvgy3tqojq` is the base32 spelling of
// the RFC's 32-byte SHA-256 test key -- the string "12345678901234567890"
// repeated, which is the least secret secret in cryptography.
//
// A real TOTP secret can never appear here: section 8 puts it in
// Parameter Store, never in a tracked file, so a six-digit literal in
// this crate is either a published vector or a defect. That is exactly
// the distinction this gate exists to force someone to make by hand.
//
// `legal`, `malformed` and `provably` are that module's own test
// names and refusal words -- base32 alphabets that are legal, inputs
// that are malformed, and a bound that is provably reached.
const TOTP_VECTOR: &str = "
    755224 287082 359152 969429 338314 254676 287922
    162583 399871 520489 005924 050471 081804 279037 353130
    gezdgnbvgy3tqojqgezdgnbvgy3tqojq
    legal malformed provably
";

// ---- group 19: two more test scaffolding words that arrived with the
// census and the null-price decoder. `census-incremental` is a
// temporary directory name chosen for the test that proves the census
// APPENDS rather than rewrites -- the one that asserts on the file's
// INODE, because asserting on its size passed under both strategies
// and let a mutant survive. `fits` and `skipped` are assertion labels:
// a window that fits inside both the vendor cap and one month, and a
// bar the Groww decoder skipped because a price cell held null.
// `daily` and `unstorable` are `Scratch::new(...)` temp-directory
// names in crates/pull/tests/broker.rs -- one for the test that a
// daily pull lands under `1day/` and folds a session into one bar,
// one for the test that a rung the store cannot carry is refused BY
// NAME rather than filed under the nearest directory that exists.
// Both name a scratch path this repository chose, not a segment any
// vendor or account uses.
const LATE_SCAFFOLD: &str = "census-incremental fits skipped daily unstorable all counts";
// `crates/pull/src/folder.rs` and its tests, added by another line of work. Read in
// context before declaring, one at a time, because declaring a literal is a claim
// that it is not a parameter-path segment:
//
//   vendor-data     `pub const ROOT_DIR` -- a LOCAL filesystem directory the folder
//                   scanner walks. Not a Parameter Store segment; nothing joins it
//                   into a `/<org>/<env>/<vendor>/<field>` path.
//   bent            scratch directory names in `tests/folder.rs`, each naming the
//   bought-nothing  case it sets up -- a folder whose files are bent, one that
//   wrong-kind      bought nothing, one holding the wrong kind, one never created,
//   never-created   one already sealed.
//   sealed
//   earliest        ordinary field and label words in the same module.
//   latest
//   files
//   pulled
//   read
//   verb
//
// Declared rather than left refusing. The distinction this used to draw against
// gate 11's `crates/pull/src/totp.rs` entry no longer has a second half: that one
// WAS a real §4 violation left visible for its owner, and it has since been fixed
// and its allowance deleted. The principle it illustrated stands -- blessing a
// defect hides it, so an allowlist entry must be a report with an owner. These are not
// defects -- a scratch directory called `sealed` is not a credential path, and saying
// so is what this gate asks for. Their owner should still confirm the list.
const FOLDER_SCAFFOLD: &str = "vendor-data bent bought-nothing wrong-kind never-created sealed earliest latest files pulled read verb";

// ---- group 20: THE MANIFEST FORMAT'S OWN FIELD NAMES, and nothing
// else. `pull::manifest::Layout::declare` refuses a degenerate format
// geometry and NAMES THE FIELD it refused —
// `ManifestError::DegenerateLayout { field }` — exactly as
// `store::layout::Layout::declare` does one crate away, and for the
// reason stated there: a `const` panic cannot format a field name, so
// the fallible door is where an operator finds out which of the four
// numbers was wrong.
//
// All four are field names on a struct in THIS repository, spelled the
// way the struct spells them. None is an account, an environment, a
// vendor or a field of a vendor's record: `magic` is the eight bytes a
// manifest begins with, `version` is its format number, and the two
// strides are the byte width of one entry in the two widths this
// workspace computes offsets in. D-0067.
const LAYOUT_FIELD: &str = "magic version entry_stride entry_stride_len";

// ---- group 22: TELEMETRY FIELD KEYS AND OUTCOME WORDS. D-0075.
// `crates/pull` gained a telemetry dependency and 15 emit sites, and
// every `.with("key", ...)` puts a bare lower-case word in the source
// — which is exactly the shape this gate scans for. None of these is
// a path segment: they are the LEFT side of a log field, and the
// right side is the value. They are declared one by one rather than
// waved through with a pattern, because "it looks like a log field"
// is precisely the excuse a real credential segment would wear.
//
// WHAT IS DELIBERATELY NOT HERE. No org, no environment, no vendor
// and no field name from the Parameter Store shape. `value_len` is
// the closest this list comes to a credential and it is a LENGTH:
// `crates/pull/src/ssm.rs` logs how long the token is and never the
// token, and never the parameter name. Gate 1c still refuses the
// path itself in any tracked file, and this group does not touch it.
const TELEMETRY_FIELD: &str = "
    bars bars_out before_open before_window after_window at_or_after_close
    bucket_secs bytes cap_days chunk chunks dropped entries epoch
    first_chunk_to folded from_version generation ghosts header
    instrument_id keys landed members outcome per_day per_minute
    per_second reason refused region_bytes repairing representable
    rows rows_in snapshots url value_len why absent degraded dir
    field file kept loaded max rotated vendors
    asked blank byte_len chars duplicate fault
    about census rows_read slices
    not-base32 too-long hmac-refused trailing-bits
    at
    unreadable_volume unreadable_oi
    timeframe
    agreed differed day_absent minute_absent
";

// `agreed differed day_absent minute_absent` — `ingest.rs`'s
// `pull.daycheck` line (D-3001): four COUNTS of days, the pulled day bar
// against the days its minute bars fold to. Integers on the right.

// THE SECOND BATCH, and why each is not a path segment. The emit
// sites in `secret.rs`, `totp.rs`, `work.rs` and `config.rs` landed
// after this group was first written, so the gate correctly went red
// on all fifteen. Every one is the LEFT side of a `.with(...)`:
//
//   asked blank duplicate kept  — `work.rs` counts, one plan's
//     four outcomes. Integers on the right, never a name.
//   byte_len chars max          — LENGTHS. `secret.rs` logs how many
//     bytes the credential is and never the credential; `totp.rs`
//     logs how many characters were offered against `MAX_SECRET_LEN`.
//   fault not-base32 too-long   — `totp.rs` outcome words: a
//   hmac-refused                  `TotpError` rendered to a stable
//     token so a page can group by it. They name a FAILURE, not a
//     location. `hmac-refused` joined them when the `unreachable!`
//     that gate 11 names in this file became a refusal: it says the
//     BUILD is wrong, not the operator's secret, which is why it is a
//     separate word rather than folded into one of the other three.
//   trailing-bits               — `totp.rs`: a secret whose bits do not
//     end on a whole byte (h-pull-2, D-2271). A failure, not a place.
//   file loaded vendors         — `config.rs`, reporting that the
//     untracked local configuration was read and how many vendors it
//     named. The count, never the contents.
//   rotated                     — `secret.rs`, a bool: whether a
//     re-read returned a different value. CLAUDE.md §8's "a stale
//     token is re-read; if the re-read returns the same dead value,
//     the pull halts loudly".
//
// `field` IS THE ONE WORTH ARGUING ABOUT, so it is argued here. §8's
// shape is `/<org>/<env>/<vendor>/<field>`, and `secret.rs` logs
// `.with("field", <the field's name>)` — the fourth segment's VALUE,
// not just the key. That is deliberate and it is inside §8's own
// carve-out: "`crates/pull` holds the shape and the field names; it
// holds no `org`, `env`, or `vendor` literal." The field names are
// already tracked source. The two segments §8 keeps out of this
// repository — `org` and `env` — are read at run time from the
// untracked configuration and appear in NO event on any of these
// paths. A log handed to somebody still cannot be turned into a
// parameter path.

// THE THIRD BATCH — the four run-scoped failure events, D-0087.
// `refused_whole` and the two census arms returned `Err` and logged
// NOTHING, so a backfill that landed zero bars left a quiet log. All
// four are the LEFT side of a `.with(...)`:
//
//   about      — the folder or path a refused RUN was about. A local
//     archive directory the operator typed, or a store path this
//     repository rendered. Never a Parameter Store segment.
//   census     — the census file's own path on disk, same shape.
//   rows_read  — a COUNT. The rows a refused run did read, so the
//     receipt cannot blame a vendor that answered perfectly.
//   slices     — a COUNT of appended slices whose census went
//     unpublished.
//
// Two counts and two filesystem paths. No org, no env, no vendor, no
// field of a vendor's record.

// THE FOURTH BATCH — one word, `at`, from `resolve.rs`.
//
// `pull::resolve::note_refused` emits the six refusals `crawl` can
// record, and it names the document that refused with
// `.with("at", &at)` — the same word as `IndexFailure::at`, which is
// the field this event exists to make visible. Gate 19 refused all
// six sites for logging nothing at all; fixing that necessarily put a
// new bare lower-case word in the source, which is the shape gate 1d
// scans for. The two gates pulling in opposite directions is the
// system working: one demands the event, the other demands the event
// be accounted for.
//
// It is not a path segment. The RIGHT side is a URL the crawl built
// from `INDEX_HOST` plus a category path — an exchange address on
// the public internet, and the same value `url` already carries in
// this group. The LEFT side is a two-letter English preposition, and
// §8's shape has no `at` in it: the four segments are `org`, `env`,
// `vendor` and `field`, none of which this event carries. Nothing on
// this path reads Parameter Store, and `resolve.rs` holds no
// credential of any kind — it crawls a public index site.
//
// THE SIXTH BATCH -- `timeframe`, from `ingest.rs`. The rung a spot pull
// ran at, on the once-per-target event. It is the LEFT side of a
// `.with(...)` whose right side is `rung.as_str()`, one of the words in
// the `timeframe` group above, and section 8`s shape has no `timeframe` in
// it: the four segments are org, env, vendor and field.
//
// `2min` and `10min` joined the timeframe group in the same commit. They
// are exchange rungs the other session added to `Timeframe::KNOWN`, and
// they are periods of time -- not an org, an env, a vendor or a field.
//
// THE FIFTH BATCH — `unreadable_volume` and `unreadable_oi`, from
// `csv.rs`. Two counts on the once-per-file decode event, and both name
// a SUBSTITUTION this build made: a volume field that would not parse
// is stored as `0` and an open interest as absent. CLAUDE.md §4 permits
// the degrade and forbids it being silent, and these are what stop it
// being silent — `rows` including rows whose volume was invented is not
// the same fact as `rows` where every field was read.
//
// Not path segments. Both are the LEFT side of a `.with(...)` and the
// right side is an integer COUNT, never a name; `unreadable_oi` is
// abbreviated the way the struct field is not, because the wire key is
// read in a column and the struct field is read in prose. §8's shape is
// `/<org>/<env>/<vendor>/<field>` and neither word can be any of those
// four: `csv.rs` decodes a body that is already in memory, reads no
// Parameter Store, and holds no credential.

// ---- group 24: AN INVENTED AWS ERROR BODY, WHOSE WHOLE PURPOSE IS TO
// PROVE A PATH IS NOT LEAKED. crates/pull/src/ssm.rs test module.
//
// READ THIS ONE TWICE, because it is the only group whose entries are
// shaped exactly like the four segments CLAUDE.md §8 keeps out of this
// repository. That is not a coincidence and it is not a slip: the
// fixture is a fabricated `AccessDeniedException` body carrying a full
// parameter ARN, and the test asserts that the renderer emits NONE of
// it. `ssm.rs:893` reads
//
//     for leak in ["SomeFutureException", "parameter/", "anorg", "arn:aws"]
//
// and fails if any appears in the message an operator sees. So `anorg`
// is tracked precisely so that a build which starts leaking an org
// segment goes red.
//
// Each is transparently invented -- `anorg`, `anenv`, `avendor` and
// `afield` are "an org", "an env", "a vendor", "a field" spelled as
// identifiers, and no vendor in crates/pull/src/vendor.rs is named
// `avendor`. `000000000000` is a zeroed AWS account id, the value AWS
// documentation itself uses as a placeholder, and twelve digits is
// the account-id width. `assumed-role` is a fixed ARN component of an
// STS role session, not a segment anyone chose.
//
// Gate 1c still refuses the real path in any tracked file and this
// group does not touch it: none of these six is a real org, env,
// vendor or field, and the file that holds them is the one proving
// the real ones never reach a log.
const SSM_LEAK_FIXTURE: &str = "anorg anenv avendor afield 000000000000 assumed-role";

// ---- group 23: A YEAR-MONTH IN AN ASSERTION. D-0070.
// `pull::unit::a_daily_bucket_is_an_ist_day_...` asserts the month a
// folded bar derives to, and that assertion is the whole proof of the
// fold fix: before it, a bar stamped 00:00 IST on 2026-01-01 derived
// to 2025-12. The literal is a `store::path::YearMonth` rendering,
// not a path segment — the store's own month component is written
// from a `YearMonth` and never from a caller's string.
const YEAR_MONTH: &str = "2026-01";

// ---- group 20: THE EXCHANGE'S OWN PUBLISHED FILENAMES, and three
// test words that arrived with them. crates/pull/src/nse.rs.
//
// `ind_niftybanklist` and `ind_niftytotalmarket_list` are the names
// NSE Indices Limited gives two of its constituent files, published at
// niftyindices.com and read 14 Aug 2026 (docs/00-charter.md §4d).
// They are here for a reason worth reading twice: they are quoted in a
// REFUSAL MESSAGE, because the argument for refusing is that the two
// disagree with each other -- one is word-joined and the other is
// underscore-separated, so no rule turns an index's name into its
// filename and any rule guessed from one is already wrong about the
// other. A message that made that argument without showing both
// spellings would be asking the reader to take it on trust.
//
// Neither is a credential path segment and neither could become one:
// they name a public file on a public exchange host, they contain no
// org, env, vendor or field, and nse.rs COMPOSES NO FILENAME AT ALL --
// `constituent_link` reads the URL the exchange published and refuses
// a page that carries none. That is the whole point of the module and
// these two literals are the evidence for it, not an exception to it.
//
// `nifty-bank` is the page name in that refusal's own test. `doctype`
// and `transport` are assertion substrings: the first proves an
// interstitial is refused AS HTML rather than read as an index with no
// members, and the second proves an empty document and a failed fetch
// print as different facts.
const NSE_PUBLISHED: &str = "
    ind_niftybanklist ind_niftytotalmarket_list
    nifty-bank doctype transport
";

// ---- group 21: THE JOIN'S OWN VOCABULARY. crates/pull/src/universe.rs.
//
// `isin`, `symbol` (already on group 17), `matched`, `lacks`,
// `no-isin` and `ambiguous` are the words a resolution emits for WHAT
// BECAME OF A NAME. They name a verdict, never a path component, and
// no Parameter Store path this repository reads has an `ambiguous` or
// a `no-isin` segment. They are on the wire because a page has to
// render the bucket, and D-0105's rule -- the slugs are the wire's own
// words -- makes emitting them the correct choice rather than
// inventing a second vocabulary for the browser.
//
// `isin` is worth its own sentence. It is the International Securities
// Identification Number, an ISO 6166 identifier the EXCHANGE issues
// and publishes in every constituent file. It identifies an
// instrument, not an account: it is printed on public filings, it is
// the join key D-0125 already chose, and it carries none of the four
// things this gate protects.
//
// `kite` is the wire name of a vendor whose descriptor this build does
// not yet carry, used in a TEST as the feed a symbol-keyed join is
// attributed to. It is on the same footing as `groww` and `dhan` in
// group 2, and for the same reason: it is that vendor's own public
// product name, already tracked in docs/00-charter.md section 4z.
//
// `350` and `400` are the two counts in the collapse test -- a
// universe that shrank from 750 to 350, losing 400 -- asserted to
// appear IN the refusal so the message cannot stop naming them.
const JOIN_VERDICT: &str = "
    isin no-isin matched lacks ambiguous
    kite 350 400
";

// ---- group 22: TWO WORDS FROM THE EXCHANGE'S OWN NAVIGATION.
// crates/pull/src/resolve.rs tests.
//
// `sectoral-indices` and `strategy` are asserted substrings: a test
// that breaks one category's listing checks the failure it collects
// NAMES that category, so a refusal cannot stop saying which family
// went missing. They are components of a public exchange URL --
// niftyindices.com/indices/equity/sectoral-indices -- and the four
// such paths live in `nse::Category::path`, where they are slash-
// joined and therefore never reach this scan; these two are the bare
// words a test quotes.
//
// Neither names an org, an env, a vendor or a field, and no Parameter
// Store path this repository reads has a `strategy` component. They
// describe a family of stock-market indices.
const NAV_WORD: &str = "sectoral-indices strategy";

// ---- group 23: TWO HTTP STATUS CODES, asserted to appear IN a
// refusal. crates/pull/src/resolve.rs tests.
//
// `302` and `403` are read back out of the message the document
// transport produces, so a refusal cannot stop naming the status it
// refused on -- the same device group 10's bounds use. A three-digit
// HTTP status names no account, no environment, no vendor and no
// field; it is defined by RFC 9110 and is the same number for every
// host on the internet.
const HTTP_STATUS: &str = "302 403";

// ---- group 24: THE WITHDRAWN-INDEX REGRESSION. nse.rs tests.
//
// `nifty-ipo` is a real index the exchange has COMMENTED OUT of its
// own thematic listing, copied into a test fixture from the live page
// on 14 Aug 2026 along with the markup around it. It is the regression
// that caught `without_comments`: a byte scanner reads inside an HTML
// comment where a browser does not, and the crawl found 62 index links
// on a page that lists 44. A withdrawn page answering 404 makes every
// pass incomplete, which makes a snapshot permanently unpublishable.
//
// It is the public name of a stock-market index on a public exchange
// site, it names no org, env, vendor or field, and it is on this list
// because the fixture must keep quoting it -- a regression test that
// stops naming the case it guards stops guarding it.
//
// `an-index` is the placeholder page name in the sibling test.
const WITHDRAWN_INDEX: &str = "nifty-ipo an-index";

// ---- group 25: THE THIRD BROKER'S ROW. crates/pull/src/vendor.rs.
//
// `zerodha` is that vendor's own public product name, written down as
// the name its bars are filed under -- `Descriptor::wire`, and the
// store prefix `core::vendor::Vendor::as_str`. Same footing as `groww`
// and `dhan` in group 2 and checked the same way: it is already
// tracked and public in docs/00-charter.md section 4z. Whether it is
// ALSO an operator's real <vendor> segment is operator-side and
// unknowable from inside this repository -- docs/06-limits.md
// section 18, exactly as for the other four.
//
// `instrument_token` is the vendor's own name for the numeric id it
// issues, used as the PLACEHOLDER in the bars path
// (/instruments/historical/:instrument_token/:interval) and as a
// column header in its instrument dump. It names an instrument, not
// an account.
//
// `api-key` IS A CREDENTIAL FIELD AND IS THE LINE TO READ TWICE.
//
// It is the <field> of /<org>/<env>/<vendor>/<field> -- the LAST
// component of the shape, and the only one of the four that is not
// operator-specific. CLAUDE.md section 8 says in as many words that
// crates/pull "holds the shape and the field names"; this is a field
// name, and it is here because `Auth::key_field` now carries it so the
// descriptor decides how many secrets a scheme needs rather than a
// match on the feed in crates/api.
//
// It names no org, no env and no vendor, and it is ALREADY PUBLIC in
// this repository: docs/00-charter.md section 4 lists Groww's fields as
// api-key, totp-secret and access-token, and `access-token` has been on
// group 9 since the first broker row. The VALUE is read from Parameter
// Store at runtime and appears in no tracked file, no environment
// variable and no log -- section 8, and `pull::http::Credential`
// redacts it in Debug.
const ZERODHA_ROW: &str = "zerodha instrument_token api-key";

// ---- group 24: DHAN'S ROLLING-OPTION VOCABULARY, and it is wire
// vocabulary in the same sense every row above is.
//
// `rollingoption` is the path segment of Dhan's expired-options
// endpoint (`docs/14-expired-options-data.md`). `ce` and `pe` are the
// two objects one answer carries -- the vendor's spelling of CALL and
// PUT, which the REQUEST spells the other way, and `rolling::side_key`
// is the one place those two spellings meet. `iv`, `oi`, `spot` and
// `strike` are four of the nine words its `requiredData` array admits,
// named in the request because the vendor DEFAULTS that array to a
// subset and a default is a decision taken elsewhere that changes what
// lands on disk.
//
// None names an org, an env, an account or a field of a credential.
// They are seven words a broker chose for its own JSON, and this
// repository can no more rename them than it can rename `open`.
// `1` `2` `3` are Dhan's expiry ORDINALS -- near, next, far -- and `4`
// is the one past the end, present only in the test that proves an
// ordinal the vendor does not serve is REFUSED rather than guessed at.
// Four single digits. None can be an org, an env or a vendor.
const ROLLING_WORD: &str = "rollingoption ce pe iv oi spot strike 1 2 3 4";

// ---- group 25: TEST SCAFFOLD NAMES AND ONE BOUND.
//
// `contractpath` is a scratch directory in `tests/broker.rs`; `bin` is
// the extension that test counts to prove two contracts do not share a
// file. `call` and `put` are the two labels its loop prints so a
// failure names which side it was on -- English words in an assertion
// message, never a path.
//
// `9223372036854775807` is `i64::MAX` written out, and it is there to
// prove `shift_six` REFUSES it rather than wrapping: a volatility that
// overflows is not a volatility. A number with no decimal point cannot
// be a parameter path segment in any case; it is listed because the
// scanner is shape-based and shape alone cannot tell a bound from a
// name, which is the same reason `19200` and `2019` are already here.
const ROLLING_SCAFFOLD: &str = "contractpath bin call put 9223372036854775807";

// ---- group 26: TEST FIXTURE NAMES AND PLACEHOLDER TOKENS.
//
// Every one is a value a TEST constructs, and each is named for the
// fault it arranges rather than for anything a vendor or an account is
// called:
//
//   dir-is-a-file lock-is-a-dir  scratch roots in `ingest.rs`, each
//     named for the filesystem shape it puts in the writer's way -- a
//     directory where a file belongs and the reverse.
//   census-lock-dup              a scratch root in `ingest.rs`, named
//     for the duplicated census-lock descriptor it arranges: a second
//     reference to the lock's open file description, the model of a
//     spawned child's copy (D-0693).
//   interleaved                  a word asserted to appear IN a refusal
//     sentence, so the message keeps naming the condition it refused.
//   feed                         a scratch subdirectory in
//     `archive.rs`. The English noun, not a vendor.
//   t                            a placeholder CREDENTIAL VALUE in
//     `http.rs` tests -- one character, chosen so it can never be
//     mistaken for a real token, and `Credential` redacts it in Debug
//     regardless. Section 8 is about real secrets reaching tracked
//     files; a single letter is the opposite of that.
//   z zzz                        the filler `http.rs` repeats to build
//     an over-long body and prove the refusal is CUT rather than
//     echoed whole.
//
// None is an org, an env, a vendor or a field of a credential. They are
// listed because the scanner is shape-based, and a short lowercase word
// has the same shape as a path segment whatever it means.
const PULL_FIXTURE: &str = "
    dir-is-a-file lock-is-a-dir census-lock-dup interleaved
    feed t z zzz
";

// SIXTY-SEVEN MORE, IN FOUR GROUPS, ALL OF THEM ACCUMULATED WHILE THIS
// GATE COULD NOT RUN. It sits behind gate 1e in the same job, so every
// literal added since the workspace stopped compiling arrived
// undeclared. Not one is an org, an env, a vendor or a credential
// field; they are listed because the scanner is SHAPE-based and a short
// lowercase word has a path segment's shape whatever it means.
//
//   `vendor_error` — the broker's OWN error vocabulary, quoted so a
//     refusal can be classified by what the vendor actually said.
//     `ga001` is Groww's failure code, `dh-901` and the 800-815 block
//     are Dhan's, `tokenexception` and `not_entitled` are Kite's
//     exception NAMES, and `pykiteconnect` is the SDK those names are
//     documented in rather than on the wire. Charter §3 territory:
//     these are verified external facts, and dropping them to satisfy
//     a scanner would delete the provenance that says why two
//     different failures are one event.
const VENDOR_ERROR: &str = "
    ga001 dh-901 tokenexception not_entitled session_dead
    throttled retry_bounded request_wrong error_name error_type
    reason_given pykiteconnect metadata methods null source 0x
";
//
//   `event_field` — telemetry KEY names and the values they carry.
//     `stage` is the newest: `pull::ingest::note_not_filed` takes it so
//     one helper can say WHERE a filing gave up — address, bars or
//     overlay — rather than three helpers whose event shapes drift.
//     The rest are counters and units on lines `/logs` already draws.
const EVENT_FIELD: &str = "
    address captures closed committed failures ingested
    outside-session outside-window outside_session overlay ovl paisa
    premium rupees runlog sent stage unmeasured vendor-hole name
    envelope outside
";
//
//   `pull_scratch` — scratch ROOTS a test builds, each named for the
//     fault it arranges: a store that cannot be written, a solver
//     given an unsolvable quote, bytes kept verbatim. Same class as
//     `dir-is-a-file` above and named the same way.
const PULL_SCRATCH: &str = "fromrows plausible solvable unwritable verbatim";
//
//   `bare_number` — digits the scanner cannot tell from a segment.
//     Dhan's 8xx status block, Kite's 7030, a `nifty-50` index name,
//     and round bounds in test fixtures. A number is not a path.
const BARE_NUMBER: &str = "
    001 100 200 250 50 6 7030 8 800 803 804 805 806 807 808
    809 810 811 812 813 814 815 901 nifty-50
";

// ---- group 27: FOUR TELEMETRY FIELD KEYS AND ONE FAILURE-KIND VALUE,
// and these are the only entries below that are in PRODUCTION code.
// Every one was read at its call site before it was written down.
//
//   kind             `crates/pull/src/capture.rs:162`, the LEFT side of
//     `.with("kind", ...)` on the `pull.capture` refusal event, and the
//     name of the `&str` parameter `note_refused` takes. That event
//     deliberately carries no URL, no body and no header — the URL and
//     body ARE the evidence that failed to land, and a header can carry
//     the credential — so feed, kind and the local I/O reason are all
//     that is left of it. Four letters naming a field on a log line.
//     §8's shape is `/<org>/<env>/<vendor>/<field>` and has no `kind`.
//   rung             `ingest.rs:529`, the LEFT side of
//     `.with("rung", ...)` on the once-per-target `pull.run` event. Its
//     right side is `Timeframe::as_str`, one of the words already in
//     the `timeframe` group. Same case as `timeframe` itself, which the
//     sixth batch above argues at length.
//   corrected        `http.rs:964` and `csv.rs:464`, both LEFT sides
//   negative_volume  and both COUNTS on the right — how many bars
//     carried a negative volume and were recorded as zero, and how many
//     rows were skipped outright. CLAUDE.md §4 permits the degrade and
//     forbids it being silent; these two are what stop it being silent,
//     exactly as `unreadable_volume` and `unreadable_oi` already do.
//
// `unreadable` is the one VALUE here rather than a key:
// `capture.rs:444` and `http.rs:2744` pass it as the `kind` argument
// above, so it names a CLASS OF FAILURE — the captured body could not
// be written — and `capture.rs:550` reuses the word for that test's
// scratch directory. It names no account, no environment, no vendor and
// no field, and the crate joins it into no path.
const CAPTURE_FIELD: &str = "kind rung corrected negative_volume unreadable";

// ---- group 28: SCRATCH ROOTS AND ONE TEMPORARY FILE STEM. Every word
// is the argument to `scratch(...)`, `Scratch::new(...)` or `tmp(...)`
// in a test, and every one is named for the case it arranges rather
// than for anything a vendor or an account is called. Same class as
// `dir-is-a-file` and `pull_scratch` above and read the same way:
//
//   restart-collision concurrent-names all-names-held   capture.rs
//     — a writer restarted onto an occupied name, several writers
//     racing for names at once, and every bounded name already taken.
//   short-body not-csv json-body index-lands no-partial              masters.rs
//   same-source-lock lock-names json-will-not-convert no-comma
//   wrong-shape
//     — a truncated answer, a body that is not CSV, one that is JSON,
//     a landing that reaches the index, a landing that leaves no
//     `.partial`, two landings contending for one source's lock, the
//     names a lock takes, JSON that will not convert, a row with no
//     comma, and a document of the wrong shape.
//   partialcount     ingest.rs, the partial-batch counter's own test.
//   spanning-months  tests/broker.rs, a batch that crosses a month end.
//   identity         tests/unit.rs:121-122, the STEM handed to `tmp`
//     twice by the test that proves two calls never collide. It is a
//     temporary file name in the OS temp directory, carrying the pid
//     and a serial; nothing joins it into a Parameter Store path.
//
// None is an org, an env, a vendor or a field. They are listed because
// the scanner is SHAPE-based and a short lower-case word has a path
// segment's shape whatever it means.
const PULL_SCRATCH_2: &str = "
    restart-collision concurrent-names all-names-held
    short-body not-csv json-body index-lands no-partial
    same-source-lock lock-names json-will-not-convert no-comma
    wrong-shape partialcount spanning-months identity
";

// ---- group 29: ASSERTION LABELS, A FILENAME PREFIX AND TWO
// DELIBERATELY MALFORMED FIXTURES.
//
//   zerodha-unreadable-0  `capture.rs:727`, the PREFIX a test builds
//     capture filenames from while it occupies every bounded name. It
//     is `<feed wire name>-<failure kind>-<index>` — `zerodha` is
//     already declared in group 25 and `unreadable` in group 27 — and
//     it names a file on the operator's own disk, never a segment of a
//     parameter path.
//   partial      `masters.rs:1770`, matched as a FILENAME SUBSTRING:
//     the test reads the directory and asserts no `.partial` survived
//     a landing. The English adjective, not a path component.
//   splittable   `session.rs`, an `expect()` message on three calls.
//   untouched    `http.rs:4176`, an `assert_eq!` label.
//   emit-sites   `emit_sites.rs:1412`, the `origin` argument of
//     `ingest::from_rows` — a label folded into a refusal message as
//     "(from {origin})", and here it is that test module's own name.
//   novalue      `masters.rs:2474` and `:2489`, two of the four
//   not-a-pair     deliberately malformed `Set-Cookie` values the
//     cookie jar must skip. They are invented nonsense whose whole
//     purpose is to not parse.
const PULL_ASSERT: &str = "
    zerodha-unreadable-0 partial splittable untouched
    emit-sites novalue not-a-pair
";

// ---- group 30: TWO YEAR-MONTHS AND TWO BARE NUMBERS, all four
// asserted rather than emitted.
//
// `2025-07` and `2025-08` are `store::path::YearMonth` renderings read
// back off disk by `tests/broker.rs:1270,1274` — the test that proves a
// batch spanning a month end lands one bar in each file. Same case as
// `year_month` above, and they are scanned rather than excluded because
// the `YYYY-MM-DD` exclusion is anchored to that one spelling on
// purpose: a `YYYY-MM` is two characters short of it.
//
// `401` is an HTTP status asserted to appear IN a refusal, the same
// device `302` and `403` already are: a credentialed source offered a
// transport that sends no token must name the cause rather than let the
// vendor answer it. `42` is asserted NOT to appear — it is the value of
// a JSON key that is not a list, and the test proves a number does not
// become an index name. A three-digit RFC 9110 status and a two-digit
// counter-example name no account, no environment, no vendor and no
// field.
const PULL_ASSERTED_NUMBER: &str = "2025-07 2025-08 401 42";

// ---- group 31: ONE WRONG REGION AND THREE CASE LABELS, all four
// offered by `ssm.rs` tests rather than emitted. D-1371, D-1372.
//
// `ap-south-2` is a real AWS region that is NOT the one CLAUDE.md §8
// names. `ssm.rs:1212` offers it beside `us-east-1` and two malformed
// spellings of `ap-south-1` to prove the client refuses to sign for
// any region but `ap-south-1`: the near miss is the point of the test,
// and it names no account, environment, vendor or field.
//
// `empty-id`, `empty-secret` and `empty-token` are the `assert!`
// labels of the three cases proving an empty AWS access key id,
// secret key or session token is refused before a request is signed.
// Hyphenated test-case names, never a segment of a parameter path.
// Declared on the owner's decision (PR #35), not widened by a gate.
const SSM_REGION_FIXTURE: &str = "ap-south-2 empty-id empty-secret empty-token";
// D-1109 surfaced these when the gate began reading DECODED literals.
// `orgtwo`, `vendorx` and `fieldx` are config-fixture segments written
// inside a larger fixture literal as `\"orgtwo\"`, which the line regex
// could not see (a backslash sat before each quote); `note`, `exact` and
// `__type` are JSON and CSV keys inside fixture bodies, `__type` being
// the AWS JSON error field. None names a real account, environment or
// vendor.
const INNER_LITERAL: &str = "orgtwo vendorx fieldx note exact __type";
// Negative numbers, now that a segment may begin with `-` (D-1109):
// parse and overflow fixtures, `i64::MIN` among them. No digit string
// is a path segment this repository configures.
const SIGNED_NUMBER: &str = "-1 -125 -2450 -19801 -1e300 -9223372036854775808";
// Met only in the combined fold (D-1450): branches written before
// D-1109 let a segment begin with `-` carried these, and none is a
// path segment. `-0`, `-5`, `-0945` and `-1400` are numeric
// fixtures: a calendar date's month and day pieces, a malformed CSV
// minute and a negative open interest, and the UTC offsets
// `kind_of`'s offset parser is driven with (D-0957). `-token` is the
// tail the decoded scan reads from the invented `access-token` and
// `x-token` header names `header_name` already declares whole.
// `--exact`, `--nocapture` and `--test-threads=1` are the libtest
// flags `crates/pull/tests/support/mod.rs` hands the re-executed test
// binary it runs as an unprivileged user (root-permission tests).
const FOLD_LITERAL: &str = "-0 -5 -0945 -1400 -token --exact --nocapture";
// Met when the audit fixes landed (83cb7a5). `-354` is a
// negative rupee price `rolling.rs` must refuse rather than snap to
// zero. `-7` is the tail the decoded scan reads from invented
// contract fixtures (`...-7000-CE`, `...-700000-CE`) in chain.rs and
// fno.rs. `repeats` is a refusal word asserted in http.rs and prose
// in a session.rs reason string. None is a path segment.
const AUDIT_LITERAL: &str = "-354 -7 repeats";

/// Every declared group, in the order the step joined them.
const DECLARED: [&str; 49] = [
    SEG_SHAPE,
    VENDOR_WIRE,
    CLAIM_STANDING,
    CONFIG_FIXTURE,
    SECRET_FIXTURE,
    SCAFFOLD,
    SCAFFOLD_D0610,
    TIMEFRAME,
    GOVERNOR,
    RECORD_FIELD,
    HEADER_NAME,
    BOUND_NUMBER,
    BOUND_NUMBER_D0610,
    DEBUG_LABEL,
    WIRE_DATE,
    AWS_SIGV4,
    AWS_SHARED,
    REDACT_LABEL,
    HASH_VECTOR,
    REQUEST_FIELD,
    TOTP_VECTOR,
    LATE_SCAFFOLD,
    FOLDER_SCAFFOLD,
    LAYOUT_FIELD,
    TELEMETRY_FIELD,
    YEAR_MONTH,
    NSE_PUBLISHED,
    JOIN_VERDICT,
    NAV_WORD,
    HTTP_STATUS,
    WITHDRAWN_INDEX,
    ZERODHA_ROW,
    ROLLING_WORD,
    ROLLING_SCAFFOLD,
    PULL_FIXTURE,
    SSM_LEAK_FIXTURE,
    VENDOR_ERROR,
    EVENT_FIELD,
    PULL_SCRATCH,
    BARE_NUMBER,
    CAPTURE_FIELD,
    PULL_SCRATCH_2,
    PULL_ASSERT,
    PULL_ASSERTED_NUMBER,
    SSM_REGION_FIXTURE,
    INNER_LITERAL,
    SIGNED_NUMBER,
    FOLD_LITERAL,
    AUDIT_LITERAL,
];

fn declared() -> BTreeSet<&'static str> {
    DECLARED.iter().flat_map(|g| g.split_whitespace()).collect()
}

/// `[a-z0-9_-]`, which is `pull::config::check_segment`'s lower-case alphabet.
fn segment_byte(b: u8) -> bool {
    b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-'
}

/// `grep -oE '<q>[a-z0-9_-][a-z0-9_-]{0,31}<q>'`: leftmost, non-overlapping,
/// where `<q>` is `"` in a file and `\"` inside an escaped value. The class
/// holds neither quote nor backslash, so a match is one whole run between
/// two quotes, of 1 to 32 characters.
fn quoted_words(text: &[u8], quote: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < text.len() {
        if text[i..].starts_with(quote) {
            let from = i + quote.len();
            let mut j = from;
            while j < text.len() && segment_byte(text[j]) {
                j += 1;
            }
            if (1..=32).contains(&(j - from)) && text[j..].starts_with(quote) {
                out.push(String::from_utf8_lossy(&text[from..j]).into_owned());
                i = j + quote.len();
                continue;
            }
        }
        i += 1;
    }
    out
}

/// `^[a-z0-9_-][a-z0-9_-]{0,31}$`.
fn whole_segment(v: &str) -> bool {
    (1..=32).contains(&v.len()) && v.bytes().all(segment_byte)
}

/// `^[0-9]{4}-[0-9]{2}-[0-9]{2}$`, the one date spelling not scanned.
fn iso_date(w: &str) -> bool {
    let b = w.as_bytes();
    b.len() == 10
        && b.iter().enumerate().all(|(i, c)| {
            if i == 4 || i == 7 {
                *c == b'-'
            } else {
                c.is_ascii_digit()
            }
        })
}

/// `cut -d: -f3-` on one `source_scan strings` line: `file:line:value`.
fn value_of(line: &str) -> &str {
    match line.split_once(':') {
        None => line,
        Some((_, rest)) => rest.split_once(':').map_or("", |(_, v)| v),
    }
}

/// Every segment-shaped literal under crates/pull, decoded by the scanner
/// for `.rs` and read as quoted text elsewhere (D-1109), must be declared.
fn gate_1d_verdict(
    list: &[String],
    read: &dyn Fn(&str) -> Result<Vec<u8>, String>,
    strings: &Scan,
) -> Result<Report, String> {
    let mut r = Report::default();
    if !strings.ok() {
        r.refuse("GATE 1D: source_scan strings failed, so no literal under crates/pull can be vouched for.");
        return Ok(r);
    }
    let mut words = Vec::new();
    for f in list.iter().filter(|f| !f.ends_with(".rs")) {
        words.extend(quoted_words(&read(f)?, b"\""));
    }
    let lines = strings.lines();
    if lines.is_empty() {
        r.refuse("GATE 1D READ NO STRING LITERAL under crates/pull.");
        return Ok(r);
    }
    for l in &lines {
        let v = value_of(l);
        if whole_segment(v) {
            words.push(v.to_owned());
        }
        words.extend(quoted_words(v.as_bytes(), b"\\\""));
    }
    let found: BTreeSet<String> = words.into_iter().filter(|w| !iso_date(w)).collect();
    let allowed = declared();
    for w in &found {
        if !allowed.contains(w.as_str()) {
            r.refuse(format!(
                "UNDECLARED SEGMENT-SHAPED LITERAL IN crates/pull: {w}"
            ));
            for l in lines.iter().filter(|l| l.contains(w.as_str())).take(3) {
                r.say(*l);
            }
        }
    }
    let files = list
        .iter()
        .flat_map(|n| n.split('\n'))
        .filter(|l| !l.is_empty())
        .count();
    r.say(format!(
        "read {files} tracked file(s), {} distinct segment-shaped literal(s)",
        found.len()
    ));
    if r.refused {
        r.say("");
        r.say("Either the literal is invented — declare it in .github/gates_tree.rs");
        r.say("with its reason — or it is a real path segment and must not be");
        r.say("tracked. See CLAUDE.md section 8.");
    } else {
        r.say("OK — every segment-shaped literal under crates/pull is declared.");
    }
    Ok(r)
}

// -------------------------------------------------------------- gate 2 --

/// The files `source_scan closure` resolved, for `source_scan build`.
fn closure_files(closure: &str) -> Vec<String> {
    lines_of(closure)
        .into_iter()
        .filter(|l| !l.is_empty() && !l.starts_with("UNRESOLVED "))
        .map(str::to_owned)
        .collect()
}

/// `:[[:space:]]*(build|links)[[:space:]]*=` anywhere in `FILE:LINE:text`.
fn names_build_key(rec: &str) -> bool {
    let c: Vec<char> = rec.chars().collect();
    either_space(|sp| {
        (0..c.len()).any(|i| {
            if c[i] != ':' {
                return false;
            }
            let mut p = i + 1;
            while c.get(p).is_some_and(|x| sp(*x)) {
                p += 1;
            }
            ["build", "links"].iter().any(|k| {
                if !at(&c, p, k) {
                    return false;
                }
                let mut q = p + 5;
                while c.get(q).is_some_and(|x| sp(*x)) {
                    q += 1;
                }
                c.get(q) == Some(&'=')
            })
        })
    })
}

/// A manifest's lines, each with everything from its first `#` removed, as
/// `FILE:LINE:text`, where the line names a build script or a native
/// library. The cost the step stated still holds: a `#` inside a quoted value
/// truncates its line, which can only cause a miss. A line that is not UTF-8
/// is read whole, the reading that refuses more.
fn manifest_keys(name: &str, bytes: &[u8]) -> Vec<String> {
    let mut recs: Vec<&[u8]> = bytes.split(|b| *b == b'\n').collect();
    if recs.last().is_some_and(|l| l.is_empty()) {
        recs.pop();
    }
    let mut out = Vec::new();
    for (n, rec) in recs.iter().enumerate() {
        let l = match std::str::from_utf8(rec) {
            Ok(s) => s[..s.find('#').unwrap_or(s.len())].to_owned(),
            Err(_) => String::from_utf8_lossy(rec).into_owned(),
        };
        let line = format!("{name}:{}:{l}", n + 1);
        if names_build_key(&line) {
            out.push(line);
        }
    }
    out
}

/// `(^|/)Cargo\.toml$`: the glob `*Cargo.toml` also matches `NotCargo.toml`.
fn is_manifest(name: &str) -> bool {
    name == "Cargo.toml" || name.ends_with("/Cargo.toml")
}

fn gate_2_verdict(
    roots: &[String],
    closure: &Scan,
    build: &Scan,
    manifests: &[(String, Vec<u8>)],
) -> Report {
    let mut r = Report::default();
    r.say("every file compiled into a build script:");
    if roots.is_empty() {
        r.say("  none — no tracked build.rs");
    } else {
        if !closure.ok() {
            r.refuse("  A BUILD SCRIPT PULLS IN CODE THIS GATE CANNOT RESOLVE:");
            for l in closure
                .lines()
                .into_iter()
                .filter(|l| l.starts_with("UNRESOLVED "))
            {
                r.say(format!("    {l}"));
            }
        }
        for l in closure
            .lines()
            .into_iter()
            .filter(|l| !l.starts_with("UNRESOLVED "))
        {
            r.say(format!("  {l}"));
        }
        if !build.ok() {
            r.refuse("BUILD SCRIPT INVOKES, OR CAN REACH, AN EXTERNAL PROCESS:");
            for l in build.lines() {
                r.say(format!("  {l}"));
            }
            r.say("A build script is Rust, so gate 1 passes it — and then it runs");
            r.say("something else. That is the side door. See CLAUDE.md section 2.");
        }
    }
    let keys: Vec<String> = manifests
        .iter()
        .flat_map(|(n, b)| manifest_keys(n, b))
        .collect();
    if !keys.is_empty() {
        r.refuse("A MANIFEST NAMES A BUILD SCRIPT OR A NATIVE LIBRARY:");
        for k in keys {
            r.say(k);
        }
        for l in [
            "A `build =` key renames the script out of every filename check",
            "in this file, and a `links =` key requires one to exist at all.",
            "Nothing here needs either: there is no C to compile, no protocol",
            "to generate from, and no version to stamp that",
            "env!(\"CARGO_PKG_VERSION\") does not already carry.",
            "See CLAUDE.md section 2 and gate 13 layer 3.",
        ] {
            r.say(l);
        }
    }
    if !r.refused {
        r.say("OK — no build script invokes an external process, and no manifest");
        r.say("     names one under another filename.");
    }
    r
}

// ---------------------------------------------------------- gates 9, 9b --

/// A crate that may declare no dependency of any kind.
struct Standalone {
    manifest: &'static str,
    found: &'static str,
    why: &'static [&'static str],
}

const CORE: Standalone = Standalone {
    manifest: "crates/core/Cargo.toml",
    found: "crates/core MUST DEPEND ON NOTHING. Found:",
    why: &[
        "core is compiled to wasm32 through crates/web. See D-0009.",
        "A DEV-DEPENDENCY IS NOT A TECHNICALITY: it is compiled into every",
        "test and every bench, so a crate that declares one CAN REACH IT",
        "whatever the shipping graph says.",
    ],
};

const GREEKS: Standalone = Standalone {
    manifest: "crates/greeks/Cargo.toml",
    found: "crates/greeks MUST DEPEND ON NOTHING. Found:",
    why: &["It is shared with another repository by git URL. See D-0046."],
};

/// ABSENT IS A REFUSAL, an unreadable manifest is a refusal, and any line
/// `source_scan deps` prints is a declaration.
fn standalone(spec: &Standalone, tracked: bool, deps: &Scan) -> Report {
    let mut r = Report::default();
    let f = spec.manifest;
    if !tracked {
        r.refuse(format!(
            "REFUSED  {f} is not a tracked file. The crate this gate guards is gone."
        ));
    } else if !deps.ok() {
        r.refuse(format!(
            "REFUSED  {f} could not be read as TOML, so nothing it declares can be vouched for."
        ));
    } else if !deps.out.trim_end_matches('\n').is_empty() {
        r.refuse(spec.found);
        for l in lines_of(deps.out.trim_end_matches('\n')) {
            r.say(l);
        }
        for l in spec.why {
            r.say(*l);
        }
    }
    r
}

/// `\b(brutex_core|core::(price|instrument|universe|vendor))\b`. A word
/// character is ASCII here: the C locale's reading, which sees a boundary
/// beside every other character and so refuses more.
fn names_workspace_type(line: &str) -> bool {
    let b = line.as_bytes();
    let word = |x: u8| x.is_ascii_alphanumeric() || x == b'_';
    (0..b.len()).any(|i| {
        (i == 0 || !word(b[i - 1]))
            && [
                "brutex_core",
                "core::price",
                "core::instrument",
                "core::universe",
                "core::vendor",
            ]
            .iter()
            .any(|w| {
                b[i..].starts_with(w.as_bytes()) && !b.get(i + w.len()).is_some_and(|x| word(*x))
            })
    })
}

fn greeks_leaks(files: &[(String, Vec<u8>)]) -> Vec<String> {
    let mut out = Vec::new();
    for (f, bytes) in files {
        let text = String::from_utf8_lossy(bytes);
        for (n, l) in lines_of(&text).into_iter().enumerate() {
            if names_workspace_type(l) {
                out.push(format!("{f}:{}:{l}", n + 1));
            }
        }
    }
    out
}

fn gate_9(tracked: bool, deps: &Scan) -> Report {
    let mut r = standalone(&CORE, tracked, deps);
    if !r.refused {
        r.say("OK — core declares no dependency of any kind, in any spelling.");
    }
    r
}

fn gate_9b(tracked: bool, deps: &Scan, sources: &[(String, Vec<u8>)]) -> Report {
    let mut r = standalone(&GREEKS, tracked, deps);
    if r.refused {
        return r;
    }
    let leaks = greeks_leaks(sources);
    if leaks.is_empty() {
        r.say("OK — greeks depends on nothing and names no workspace type.");
    } else {
        r.refuse("A WORKSPACE TYPE APPEARS IN crates/greeks:");
        for l in leaks {
            r.say(l);
        }
    }
    r
}

// ------------------------------------------------------------- gate 10b --

const INVARIANTS: &str = "docs/04-invariants.md";

/// `^\| [A-Z]+-[0-9]+[a-z]? `, the identifier without its bar and spaces.
fn invariant_id(line: &str) -> Option<&str> {
    let rest = line.strip_prefix("| ")?;
    let b = rest.as_bytes();
    let mut i = 0;
    while b.get(i).is_some_and(u8::is_ascii_uppercase) {
        i += 1;
    }
    if i == 0 || b.get(i) != Some(&b'-') {
        return None;
    }
    i += 1;
    let digits = i;
    while b.get(i).is_some_and(u8::is_ascii_digit) {
        i += 1;
    }
    if i == digits {
        return None;
    }
    if b.get(i).is_some_and(u8::is_ascii_lowercase) && b.get(i + 1) == Some(&b' ') {
        i += 1;
    }
    (b.get(i) == Some(&b' ')).then(|| &rest[..i])
}

fn gate_10b(text: &str) -> Report {
    let mut r = Report::default();
    let mut seen: BTreeMap<&str, usize> = BTreeMap::new();
    for l in lines_of(text) {
        if let Some(id) = invariant_id(l) {
            *seen.entry(id).or_default() += 1;
        }
    }
    if seen.is_empty() {
        r.refuse(format!(
            "GATE 10B READ NO INVARIANT IDENTIFIER in {INVARIANTS}. A gate that reads nothing is not a gate."
        ));
        return r;
    }
    let dups: Vec<&str> = seen
        .iter()
        .filter(|(_, n)| **n > 1)
        .map(|(id, _)| *id)
        .collect();
    if dups.is_empty() {
        r.say("OK — every invariant identifier is unique.");
    } else {
        r.refuse("INVARIANT IDENTIFIER DEFINED MORE THAN ONCE:");
        for d in dups {
            r.say(d);
        }
        r.say("");
        r.say("Give the LATER row an unused number. Never renumber the");
        r.say("earlier one -- that rewrites history. See D-0329.");
    }
    r
}

// -------------------------------------------------------------- gate 7 --

const WEB_MANIFEST: &str = "crates/web/Cargo.toml";

/// The first field of every line with an `=` inside a flat `[dependencies]`
/// table, exactly the old reading -- dotted keys and other tables included
/// in what it does not see. The crate does not exist (D-0052), so the gate
/// skips; this keeps its rule for the day a crate of this name returns.
fn web_dependencies(text: &str) -> Vec<&str> {
    let mut inside = false;
    let mut out = Vec::new();
    for l in lines_of(text) {
        if l.starts_with("[dependencies]") {
            inside = true;
            continue;
        }
        if l.starts_with('[') {
            inside = false;
        }
        if inside
            && l.contains('=')
            && let Some(w) = l.split([' ', '\t']).find(|w| !w.is_empty())
        {
            out.push(w);
        }
    }
    out
}

fn gate_7(manifest: Option<&str>) -> Report {
    let mut r = Report::default();
    let Some(text) = manifest else {
        r.say("skip — crates/web does not exist (D-0052 moved it to web/)");
        return r;
    };
    let extra: Vec<&str> = web_dependencies(text)
        .into_iter()
        .filter(|d| *d != "core")
        .collect();
    if extra.is_empty() {
        r.say("OK.");
    } else {
        r.refuse("crates/web MAY ONLY DEPEND ON core. Found:");
        for e in extra {
            r.say(e);
        }
        r.say("It compiles to wasm32 where the filesystem does not exist.");
    }
    r
}

// ---------------------------------------------------------------- main --

fn arg(args: &[String], i: usize) -> Result<&str, String> {
    args.get(i)
        .map(String::as_str)
        .ok_or_else(|| format!("missing argument {i}: {USAGE}"))
}

const USAGE: &str = "gates_tree <gate-0|gate-1|gate-1e|gate-1b|gate-1g|gate-1f|gate-1c|gate-1d|gate-2|gate-9|gate-9b|gate-10b|gate-7> [phase] [args..]";

fn scan_in(work: &Path, file: &str, status: &str) -> Result<Scan, String> {
    Scan::read(&work.join(file).to_string_lossy(), status)
}

fn read_all(files: &[String]) -> Result<Vec<(String, Vec<u8>)>, String> {
    files.iter().map(|f| Ok((f.clone(), disk(f)?))).collect()
}

fn exactly(given: &str, expected: &str) -> Result<(), String> {
    if given == expected {
        Ok(())
    } else {
        Err(format!(
            "this gate reads {expected}, and the step named {given}"
        ))
    }
}

fn run(args: &[String]) -> Result<Report, String> {
    let a = |i| arg(args, i);
    let dir = |i| a(i).map(PathBuf::from);
    let phase = args.get(1).map_or("", String::as_str);
    match (a(0)?, phase) {
        ("gate-0", "prepare") => gate_0_prepare(
            &ls_files(&[".github/*.yml"])?,
            &ls_files(&["crates/*.rs", ".github/*.rs"])?,
            &dir(2)?,
        ),
        ("gate-0", "verdict") => {
            let w = dir(2)?;
            Ok(gate_0_verdict(
                &scan_in(&w, "workflow", a(3)?)?,
                &scan_in(&w, "aggregator", a(4)?)?,
                &scan_in(&w, "spawns", a(5)?)?,
            ))
        }
        ("gate-1", "prepare") => {
            let raw = git(&["ls-files", "-z"])?;
            let to = dir(2)?.join("tracked");
            std::fs::write(&to, raw).map_err(|e| format!("{}: {e}", to.display()))?;
            Ok(Report::default())
        }
        ("gate-1", "verdict") => {
            let w = dir(2)?;
            gate_1_verdict(
                &git(&["ls-files", "-s", "-z"])?,
                &scan_in(&w, "content", a(3)?)?,
                &scan_in(&w, "orphans", a(4)?)?,
            )
        }
        ("gate-1e", "prepare") => gate_1e_prepare(&dir(2)?, &dir(3)?),
        ("gate-1e", "verdict") => {
            let phase = match a(2)? {
                "build" => Phase::Build,
                "test" => Phase::Test,
                other => return Err(format!("gate-1e verdict takes build or test, not {other}")),
            };
            let log = dir(3)?.join(INVOKED);
            let invoked = match std::fs::read(&log) {
                Ok(b) => Some(String::from_utf8_lossy(&b).into_owned()),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
                Err(e) => return Err(format!("{}: {e}", log.display())),
            };
            let status = a(4)?
                .parse::<i32>()
                .map_err(|_| format!("`{}` is not an exit status", a(4).unwrap_or("")))?;
            let mut out = Vec::new();
            std::io::stdin()
                .read_to_end(&mut out)
                .map_err(|e| format!("stdin: {e}"))?;
            Ok(gate_1e_verdict(
                phase,
                invoked.as_deref(),
                status,
                &String::from_utf8_lossy(&out),
            ))
        }
        ("gate-1b", _) => Ok(gate_1b(&ls_files(&[])?)),
        ("gate-1g", "prepare") => {
            let w = dir(2)?;
            for (file, list) in [(TOOLCHAIN, "toolchain.z"), (NEXTEST, "nextest.z")] {
                let present: Vec<String> = Path::new(file)
                    .is_file()
                    .then(|| file.to_owned())
                    .into_iter()
                    .collect();
                write_nul(&w.join(list), &present)?;
            }
            Ok(Report::default())
        }
        ("gate-1g", "verdict") => {
            let w = dir(2)?;
            let toolchain = scan_in(&w, "toolchain", a(3)?)?;
            let nextest = scan_in(&w, "nextest", a(4)?)?;
            let tools = ToolFiles {
                toolchain: (!read_nul(&w.join("toolchain.z"))?.is_empty(), &toolchain),
                nextest: (!read_nul(&w.join("nextest.z"))?.is_empty(), &nextest),
            };
            let workflows = ls_files(&[".github/*.yml"])?
                .into_iter()
                .map(|f| Ok((f.clone(), read_text(Path::new(&f))?)))
                .collect::<Result<Vec<_>, String>>()?;
            Ok(gate_1g_verdict(&ls_files(&[])?, &tools, &workflows))
        }
        ("gate-1f", "prepare") => {
            let files = ls_files(&["crates/*.rs"])?;
            let mut r = Report::default();
            if files.is_empty() {
                r.refuse("GATE 1F READ NO FILE.");
            } else {
                write_nul(&dir(2)?.join("rs.z"), &files)?;
            }
            Ok(r)
        }
        ("gate-1f", "verdict") => {
            let w = dir(2)?;
            Ok(gate_1f_verdict(
                read_nul(&w.join("rs.z"))?.len(),
                &scan_in(&w, "browser", a(3)?)?,
            ))
        }
        ("gate-1c", _) => Ok(gate_1c(&ls_files(&[])?, &disk)),
        ("gate-1d", "prepare") => {
            let list = ls_files(&["crates/pull"])?;
            let mut r = Report::default();
            if list.is_empty() {
                r.refuse("GATE 1D READ NO FILE under crates/pull.");
                return Ok(r);
            }
            let w = dir(2)?;
            write_nul(&w.join("list.z"), &list)?;
            let rs: Vec<String> = list.into_iter().filter(|f| f.ends_with(".rs")).collect();
            write_nul(&w.join("rs.z"), &rs)?;
            Ok(r)
        }
        ("gate-1d", "verdict") => {
            let w = dir(2)?;
            gate_1d_verdict(
                &read_nul(&w.join("list.z"))?,
                &disk,
                &scan_in(&w, "strings", a(3)?)?,
            )
        }
        ("gate-2", "prepare") => {
            let w = dir(2)?;
            let raw = git(&["ls-files", "-z"])?;
            let mut r = Report::default();
            if nul_names(&raw)?.is_empty() {
                r.refuse("GATE 2 READ NO TRACKED FILE.");
                return Ok(r);
            }
            let to = w.join("tracked");
            std::fs::write(&to, raw).map_err(|e| format!("{}: {e}", to.display()))?;
            write_nul(&w.join("roots.z"), &ls_files(&["*build.rs"])?)?;
            Ok(r)
        }
        ("gate-2", "files") => {
            let w = dir(2)?;
            let closure = read_text(&w.join("closure"))?;
            write_nul(&w.join("files.z"), &closure_files(&closure))?;
            Ok(Report::default())
        }
        ("gate-2", "verdict") => {
            let w = dir(2)?;
            let manifests: Vec<String> = ls_files(&["*Cargo.toml"])?
                .into_iter()
                .filter(|m| is_manifest(m))
                .collect();
            Ok(gate_2_verdict(
                &read_nul(&w.join("roots.z"))?,
                &scan_in(&w, "closure", a(3)?)?,
                &scan_in(&w, "found", a(4)?)?,
                &read_all(&manifests)?,
            ))
        }
        ("gate-9", _) => {
            exactly(a(1)?, CORE.manifest)?;
            let tracked = !ls_files(&[CORE.manifest])?.is_empty();
            Ok(gate_9(tracked, &Scan::read(a(2)?, a(3)?)?))
        }
        ("gate-9b", _) => {
            exactly(a(1)?, GREEKS.manifest)?;
            let tracked = !ls_files(&[GREEKS.manifest])?.is_empty();
            let sources = read_all(&ls_files(&["crates/greeks/**/*.rs"])?)?;
            Ok(gate_9b(tracked, &Scan::read(a(2)?, a(3)?)?, &sources))
        }
        ("gate-10b", _) => Ok(gate_10b(&read_text(Path::new(INVARIANTS))?)),
        ("gate-7", _) => {
            let p = Path::new(WEB_MANIFEST);
            let text = if p.is_file() {
                Some(read_text(p)?)
            } else {
                None
            };
            Ok(gate_7(text.as_deref()))
        }
        (gate, phase) => Err(format!("unknown gate or phase `{gate} {phase}`: {USAGE}")),
    }
}

fn main() -> ExitCode {
    if let Some((name, exe)) = stub_identity() {
        return ExitCode::from(act_as_stub(&name, exe.as_deref()));
    }
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(r) => {
            let mut out = std::io::stdout().lock();
            for l in &r.lines {
                let _ = writeln!(out, "{l}");
            }
            if r.refused {
                ExitCode::from(1)
            } else {
                ExitCode::SUCCESS
            }
        }
        Err(e) => {
            eprintln!("gates_tree: {e}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scan(out: &str, status: i32) -> Scan {
        Scan {
            out: out.to_owned(),
            status,
        }
    }

    fn ok() -> Scan {
        scan("", 0)
    }

    fn names(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| (*s).to_owned()).collect()
    }

    /// A slash-joined path built at run time, so no fixture in this file is
    /// a literal gate 1c would refuse.
    fn joined(parts: &[&str]) -> String {
        parts.join("/")
    }

    fn scratch(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("gates-tree-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    // ---- shared readings ----

    #[test]
    fn lines_are_read_as_a_line_tool_reads_them() {
        assert_eq!(lines_of("a\nb\n"), vec!["a", "b"]);
        assert_eq!(lines_of("a\nb"), vec!["a", "b"]);
        assert_eq!(lines_of("a\r\n\nb"), vec!["a\r", "", "b"]);
        assert!(lines_of("").is_empty());
    }

    #[test]
    fn a_listing_is_nul_separated_and_must_be_utf8() {
        assert_eq!(
            nul_names(b"a b.rs\0x\ny\0\0").unwrap(),
            vec!["a b.rs", "x\ny"]
        );
        assert!(nul_names(b"ok\0\xff\0").is_err());
        let d = scratch("nul");
        let list = names(&["crates/a b.rs", "\u{e9}.rs"]);
        write_nul(&d.join("l.z"), &list).unwrap();
        assert_eq!(read_nul(&d.join("l.z")).unwrap(), list);
        assert!(read_nul(&d.join("missing.z")).is_err());
    }

    #[test]
    fn a_scan_status_that_is_not_a_number_is_an_error_not_a_pass() {
        let d = scratch("scan");
        std::fs::write(d.join("out"), "x\n").unwrap();
        let s = scan_in(&d, "out", "1").unwrap();
        assert!(!s.ok());
        assert_eq!(s.lines(), vec!["x"]);
        assert!(scan_in(&d, "out", "").is_err());
        assert!(scan_in(&d, "out", "zero").is_err());
        assert!(scan_in(&d, "absent", "0").is_err());
    }

    #[test]
    fn either_space_reading_is_tried() {
        assert!(either_space(|sp| sp('\u{2003}')));
        assert!(!either_space(|sp| sp('\u{a0}')));
        assert!(either_space(|sp| sp('\u{b}')));
    }

    // ---- gate 0 ----

    #[test]
    fn gate_0_refuses_an_empty_workflow_listing() {
        let d = scratch("g0");
        let r = gate_0_prepare(&[], &[], &d).unwrap();
        assert!(r.refused);
        assert!(r.text().contains("GATE 0 READ NO WORKFLOW."));
        let r = gate_0_prepare(&names(&[".github/workflows/ci.yml"]), &[], &d).unwrap();
        assert!(!r.refused);
        assert_eq!(
            read_nul(&d.join("workflows.z")).unwrap(),
            names(&[".github/workflows/ci.yml"])
        );
    }

    #[test]
    fn gate_0_refuses_each_failing_scan_by_name() {
        assert!(!gate_0_verdict(&ok(), &ok(), &ok()).refused);
        let cases = [
            (
                gate_0_verdict(&scan("ci.yml:3: continue-on-error\n", 1), &ok(), &ok()),
                "the workflow lines above are refused",
            ),
            (
                gate_0_verdict(&ok(), &scan("x\n", 1), &ok()),
                "ci-ok no longer guards every gate",
            ),
            (
                gate_0_verdict(&ok(), &ok(), &scan("", 2)),
                "starts a shell or interpreter",
            ),
        ];
        for (r, why) in cases {
            assert!(r.refused, "{why}");
            assert!(r.text().contains(why), "{}", r.text());
            assert!(!r.text().contains("OK —"));
        }
        let r = gate_0_verdict(&scan("ci.yml:3: continue-on-error\n", 1), &ok(), &ok());
        assert!(r.text().contains("ci.yml:3: continue-on-error"));
    }

    // ---- gate 1 ----

    fn gate_1_of(records: &[&str]) -> Report {
        let mut raw = Vec::new();
        for r in records {
            raw.extend_from_slice(r.as_bytes());
            raw.push(0);
        }
        gate_1_verdict(&raw, &ok(), &ok()).unwrap()
    }

    fn rec(mode: &str, path: &str) -> String {
        format!("{mode} 0123456789abcdef0123456789abcdef01234567 0\t{path}")
    }

    fn refused_1(mode: &str, path: &str) -> String {
        let r = gate_1_of(&[&rec(mode, path)]);
        assert!(r.refused, "{mode} {path} passed");
        r.text()
    }

    fn passed_1(mode: &str, path: &str) {
        let r = gate_1_of(&[&rec(mode, path)]);
        assert!(!r.refused, "{mode} {path} refused: {}", r.text());
    }

    #[test]
    fn gate_1_refuses_every_mode_but_an_ordinary_file_outside_web() {
        for mode in ["100755", "120000", "160000"] {
            let t = refused_1(mode, "crates/x/src/lib.rs");
            assert!(
                t.contains(&format!(
                    "FORBIDDEN MODE  crates/x/src/lib.rs  ({mode}; only 100644 outside web/)"
                )),
                "{t}"
            );
        }
        passed_1("120000", "web/link");
        passed_1("100755", "web/run.sh");
    }

    #[test]
    fn gate_1_places_the_extensionless_names() {
        for p in [
            ".gitignore",
            ".gitattributes",
            "web/.gitignore",
            "web/x/.gitattributes",
        ] {
            passed_1("100644", p);
        }
        let t = refused_1("100644", "crates/x/.gitignore");
        assert!(
            t.contains("(.gitignore is allowed only at the root)"),
            "{t}"
        );
        for p in [
            "LICENSE",
            "CODEOWNERS",
            ".github/CODEOWNERS",
            "docs/CODEOWNERS",
            "web/LICENSE",
        ] {
            passed_1("100644", p);
        }
        for p in [
            "crates/x/LICENSE",
            ".github/LICENSE",
            "docs/LICENSE",
            "a/CODEOWNERS",
        ] {
            let t = refused_1("100644", p);
            assert!(
                t.contains("is allowed only where it means something"),
                "{t}"
            );
        }
        // A name the old line grep read on any of its lines.
        let t = refused_1("100644", "crates/LICENSE\nfoo.rs");
        assert!(t.contains("allowed only where it means something"), "{t}");
    }

    #[test]
    fn gate_1_decides_the_extension_list_by_path() {
        for p in [
            "crates/a/src/lib.rs",
            "Cargo.toml",
            "Cargo.lock",
            "docs/x.md",
            "docs/x.html",
            "docs/x.css",
            ".github/workflows/ci.yml",
            ".github/tool.rs",
            ".claude/launch.json",
            "web/src/app.ts",
            "web/package.json",
            "web/noext",
            "crates/a b/src/\u{e9}.rs",
            ".rustfmt.toml",
        ] {
            passed_1("100644", p);
        }
        for (p, why) in [
            ("crates/a/src/app.ts", "(extension ts, outside web/)"),
            ("x.yml", "(extension yml, outside web/)"),
            ("crates/x.yml", "(extension yml, outside web/)"),
            ("x.json", "(extension json, outside web/)"),
            (".github/x.json", "(extension json, .github/)"),
            (".claude/x.yml", "(extension yml, .claude/)"),
            ("Makefile", "(extension <none>, outside web/)"),
            ("crates/x/notes.", "(extension <none>, outside web/)"),
            ("a b.sh", "(extension sh, outside web/)"),
            ("crates/x.RS", "(extension RS, outside web/)"),
            // The old here-string grep accepted an extension on any of its
            // lines; the whole extension is read now.
            ("crates/x.rs\nevil", "(extension rs\nevil, outside web/)"),
        ] {
            let t = refused_1("100644", p);
            assert!(t.contains(why), "{p}: {t}");
        }
    }

    #[test]
    fn gate_1_reports_its_scanners_and_refuses_an_empty_listing() {
        let r = gate_1_verdict(b"", &ok(), &ok()).unwrap();
        assert!(r.refused);
        assert!(r.text().contains("GATE 1 READ NO TRACKED FILE."));
        let one = rec("100644", "a.rs");
        let raw = format!("{one}\0");
        let r = gate_1_verdict(raw.as_bytes(), &scan("a.rs: NUL byte\n", 1), &ok()).unwrap();
        assert!(r.refused);
        assert!(
            r.text()
                .contains("a.rs: NUL byte\nFORBIDDEN CONTENT above.")
        );
        assert!(
            r.text()
                .contains("Under .claude/: rs toml md lock html css json")
        );
        let r = gate_1_verdict(raw.as_bytes(), &ok(), &scan("ORPHAN b.rs\n", 1)).unwrap();
        assert!(r.refused);
        assert!(
            r.text()
                .contains("ORPHAN b.rs\nA TRACKED .rs ABOVE IS COMPILED BY NOTHING")
        );
        let r = gate_1_verdict(raw.as_bytes(), &ok(), &ok()).unwrap();
        assert!(!r.refused);
        assert!(r.text().starts_with("read 1 tracked file(s)\nOK —"));
        assert!(gate_1_verdict(b"\xff\0", &ok(), &ok()).is_err());
    }

    // ---- gate 1e ----

    #[test]
    fn gate_1e_shadows_every_toolchain_interpreter_and_shell() {
        let s = stub_names();
        assert_eq!(s.len(), 21);
        for n in ["node", "npm", "vite", "perl", "lua", "sh", "bash", "tsc"] {
            assert!(s.iter().any(|x| x == n), "{n}");
        }
        assert!(s.contains(&PY.to_owned()));
        assert!(s.contains(&format!("{PY}3")));
        assert_eq!(PY.len(), 6);
    }

    #[test]
    fn a_stub_logs_its_name_beside_itself_and_fails() {
        let d = scratch("stub");
        let exe = d.join("vite");
        assert_eq!(act_as_stub("vite", Some(&exe)), 127);
        assert_eq!(act_as_stub("node", Some(&exe)), 127);
        assert_eq!(
            std::fs::read_to_string(d.join(INVOKED)).unwrap(),
            "vite\nnode\n"
        );
        assert_eq!(act_as_stub("node", None), 127);
    }

    #[test]
    fn gate_1e_refuses_an_invoked_stub_first() {
        let r = gate_1e_verdict(Phase::Build, Some("vite\nnode\nvite\n"), 0, "fine");
        assert!(r.refused);
        assert_eq!(
            r.text(),
            "FORBIDDEN: a crate invoked a front-end toolchain during the build:\n  node\n  vite"
        );
        let r = gate_1e_verdict(Phase::Test, Some("npm\n"), 101, "");
        assert!(
            r.text()
                .contains("while its tests ran:\n  npm\nThe build was clean")
        );
        // An empty log is no invocation.
        assert!(!gate_1e_verdict(Phase::Build, Some(""), 0, "ok").refused);
    }

    #[test]
    fn gate_1e_reads_the_exit_status_the_old_gate_discarded() {
        let out: String = (1..=50).map(|i| format!("line {i}\n")).collect();
        let r = gate_1e_verdict(Phase::Build, None, 101, &out);
        assert!(r.refused);
        let t = r.text();
        assert!(t.starts_with("GATE 1E FAILED: cargo build exited 101 with the front-end"));
        assert!(t.contains("The tail of the build:\nline 11\n"));
        assert!(!t.contains("line 10\n"));
        assert!(t.ends_with("line 50"));
        let r = gate_1e_verdict(Phase::Test, None, 127, "x");
        assert!(r.text().contains("cargo test exited 127"));
        assert!(r.text().contains("The tail of the run:\nx"));
    }

    #[test]
    fn gate_1e_reads_package_metadata_case_blind() {
        for line in [
            "reading node_modules/x",
            "Found YARN.LOCK",
            "pnpm-lock.yaml",
            "  Package.Json",
            "PAC\u{212a}AGE.JSON",
        ] {
            let r = gate_1e_verdict(Phase::Build, None, 0, &format!("ok\n{line}\n"));
            assert!(r.refused, "{line}");
            assert_eq!(
                r.text(),
                format!("FORBIDDEN: the build read front-end package metadata.\n{line}")
            );
        }
        let r = gate_1e_verdict(Phase::Test, None, 0, "package-json\nyarnXlock");
        assert!(!r.refused);
        assert!(r.text().contains("OK — the workspace builds AND tests"));
        assert!(
            gate_1e_verdict(Phase::Build, None, 0, "Compiling x")
                .lines
                .is_empty()
        );
        let r = gate_1e_verdict(Phase::Test, None, 0, "node_modules");
        assert!(r.text().contains("the test run read"));
    }

    // ---- gate 1b ----

    #[test]
    fn gate_1b_confines_json_and_yml() {
        assert!(gate_1b(&[]).refused);
        let r = gate_1b(&names(&[
            ".github/workflows/ci.yml",
            ".claude/launch.json",
            "web/package.json",
            "web/a/b.yml",
            "Cargo.toml",
        ]));
        assert!(!r.refused, "{}", r.text());
        for p in [
            "x.yml",
            "crates/a/x.json",
            "a/.github/x.yml",
            "docs/.claude/x.json",
            "crates/web/x.json",
            "web/ok\nevil.yml",
        ] {
            let r = gate_1b(&names(&[p]));
            assert!(r.refused, "{p}");
            assert!(r.text().starts_with("CONFIG OUTSIDE ITS HOME:\n"));
        }
        assert!(!gate_1b(&names(&["x.yaml", "x.jsonl", "x.yml.md"])).refused);
    }

    // ---- gate 1g ----

    fn tools_absent() -> (Scan, Scan) {
        (ok(), ok())
    }

    fn gate_1g_of(listing: &[&str], workflow: &str) -> Report {
        let (a, b) = tools_absent();
        let tools = ToolFiles {
            toolchain: (false, &a),
            nextest: (false, &b),
        };
        gate_1g_verdict(
            &names(listing),
            &tools,
            &[(".github/workflows/w.yml".to_owned(), workflow.to_owned())],
        )
    }

    #[test]
    fn gate_1g_refuses_a_tracked_cargo_configuration_anywhere() {
        for p in [
            ".cargo/config.toml",
            "crates/x/.cargo/config",
            "a/.cargo",
            ".cargo",
        ] {
            let r = gate_1g_of(&[p], "");
            assert!(r.refused, "{p}");
            assert!(r.text().contains("REFUSED  a tracked cargo configuration:"));
        }
        assert!(!gate_1g_of(&["x/.cargo-old/a", "x.cargo/b", "Cargo.toml"], "").refused);
    }

    #[test]
    fn gate_1g_allows_each_tool_file_only_at_its_one_place() {
        assert!(!gate_1g_of(&[TOOLCHAIN, NEXTEST], "").refused);
        for p in [
            ".config/other.toml",
            ".config/a/nextest.toml",
            "crates/x/rust-toolchain",
            "crates/x/rust-toolchain.toml",
            "rust-toolchain",
            "x/nextest.toml",
            "nextest.toml",
        ] {
            let r = gate_1g_of(&[p], "");
            assert!(r.refused, "{p}");
            assert!(r.text().contains("outside its one place"));
        }
    }

    #[test]
    fn gate_1g_reads_each_tool_files_keys() {
        let good_t = scan(
            "rust-toolchain.toml:3:toolchain = {table}\nrust-toolchain.toml:4:toolchain.channel = \"1\"\n",
            0,
        );
        let good_n = scan(
            ".config/nextest.toml:4:profile.default.overrides[] = {table}\n.config/nextest.toml:5:profile.ci-x_1.overrides[].priority = 1\n",
            0,
        );
        let tools = ToolFiles {
            toolchain: (true, &good_t),
            nextest: (true, &good_n),
        };
        let r = gate_1g_verdict(&names(&["a"]), &tools, &[("w".into(), String::new())]);
        assert!(!r.refused, "{}", r.text());
        assert!(
            r.text()
                .contains("ok       rust-toolchain.toml — 2 key(s), all allowed")
        );
        for (t, n, why) in [
            (
                "x:1:toolchain.path = \"/x\"\n",
                "",
                "           toolchain.path",
            ),
            (
                "",
                "x:1:scripts.setup.db.command = \"x\"\n",
                "           scripts.setup.db.command",
            ),
            (
                "",
                "x:1:profile.default.junit.path = \"x\"\n",
                "profile.default.junit.path",
            ),
            ("", "x:1:profile..overrides[] = 1\n", "profile..overrides[]"),
            (
                "",
                "x:1:profile.a.b.overrides[] = 1\n",
                "profile.a.b.overrides[]",
            ),
            ("x:1:\"tool chain\" = 1\n", "", "x:1:\"tool chain\" = 1"),
        ] {
            let (ts, ns) = (scan(t, 0), scan(n, 0));
            let tools = ToolFiles {
                toolchain: (true, &ts),
                nextest: (true, &ns),
            };
            let r = gate_1g_verdict(&names(&["a"]), &tools, &[("w".into(), String::new())]);
            assert!(r.refused, "{why}");
            assert!(r.text().contains("sets a key outside its allowlist:"));
            assert!(r.text().contains(why), "{}", r.text());
        }
        let bad = scan("", 2);
        let tools = ToolFiles {
            toolchain: (true, &bad),
            nextest: (false, &bad),
        };
        let r = gate_1g_verdict(&names(&["a"]), &tools, &[("w".into(), String::new())]);
        assert!(
            r.text()
                .contains("REFUSED  rust-toolchain.toml is not TOML this gate can read")
        );
        assert!(!r.text().contains(".config/nextest.toml"));
    }

    #[test]
    fn a_toml_key_is_read_from_the_scanners_line() {
        assert_eq!(toml_key("f:12:a.b = 1"), "a.b");
        assert_eq!(toml_key("f:x:a.b = 1"), "f:x:a.b = 1");
        assert_eq!(toml_key("f::a = 1"), "f::a = 1");
        assert_eq!(toml_key("f:1:a =1"), "f:1:a =1");
        assert_eq!(toml_key("f:1: = 1"), "");
    }

    #[test]
    fn gate_1g_refuses_every_spelling_of_a_wrapper_runner_or_linker() {
        for l in [
            "  RUSTC_WRAPPER: sccache",
            "    env RUSTC=foo cargo build",
            "CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER = qemu",
            "CARGO_TARGET_A_LINKER: x",
            "CARGO_BUILD_RUSTC_WRAPPER\t=x",
            "RUSTUP_TOOLCHAIN: nightly",
            "RUSTDOC = x",
            "# RUSTC_WRAPPER: also refused in a comment",
            "RUSTC\u{2003}= x",
            "x;RUSTC_LINKER=cc",
            "run: cargo build --config 'build.rustc-wrapper=\"x\"'",
            "cargo test\t--config=x",
            "  RUSTFLAGS: -C linker=x",
            "RUSTDOCFLAGS='-Z unstable-options --runtool x'",
            "cargo test -- --test-runtool x",
            "cargo nextest run --tool-config-file x",
            "cargo nextest run --config-file x",
            "CARGO_HOME: /x",
            "a CARGO_HOME =x",
            "export \"$n\"=x",
            "declare -x $n=1",
            "local -r -a ${n}=1",
            "x; typeset a$b=1",
            "(readonly $x=1)",
            "env $v=1 cargo build",
            "printf -v x '%s' y",
            "printf\t\t-v x",
            "echo /x >> \"$GITHUB_PATH\"",
            "echo X=1 >> \"$GITHUB_ENV\"",
            "if [ -n \"${GITHUB_ENV:-}\" ]; then\r",
            "printf 'SOURCE_SCAN=%s\\n' \"$tool\" >> \"$GITHUB_ENV\" # extra",
            "printf 'RUSTC_WRAPPER=%s\\n' \"$x\" >> \"$GITHUB_ENV\"",
            "printf \u{2019}SOURCE_SCAN=%s\\n' \"$tool\" >> \"$GITHUB_ENV\"",
        ] {
            let r = gate_1g_of(&["a"], &format!("x\n{l}\n"));
            assert!(r.refused, "passed: {l:?}");
            assert!(
                r.text().contains(&format!(".github/workflows/w.yml:2:{l}")),
                "{}",
                r.text()
            );
            assert!(r.text().contains("GATE 1G FAILED. See D-1105."));
        }
    }

    #[test]
    fn gate_1g_refuses_a_wrapper_name_in_any_position() {
        // P15-08, D-2322. The finding's three lines, then the forms that set
        // a listed variable with no `NAME=` and no `GITHUB_ENV` on the line:
        // a value assembled here and written by a sanctioned line later.
        for l in [
            "CARGO_BUILD_RUSTDOC: /tmp/x",
            "RUSTC_WRAPPER: /tmp/y",
            "printf '%s=%s\\n' RUSTC_WRAPPER /tmp/z >> \"$GITHUB_ENV\"",
            "tool=$(printf '%s=%s' RUSTC_WRAPPER /tmp/z)",
            "tool=$'x\\nRUSTC_WRAPPER=/tmp/z'",
            "set -- CARGO_BUILD_RUSTDOC /tmp/x",
            "n=\"CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER\"",
            "echo RUSTDOC",
            "k=RUSTUP_TOOLCHAIN",
        ] {
            let r = gate_1g_of(&["a"], &format!("x\n{l}\n"));
            assert!(r.refused, "passed: {l:?}");
            assert!(names_wrapper(&l.chars().collect::<Vec<_>>()), "{l:?}");
        }
        for l in [
            "RUSTC_WRAPPERS x",
            "MY_RUSTC_WRAPPER x",
            "echo $RUSTC_VERSION",
            "rustc --edition=2024 x.rs",
            "CARGO_TARGET_DIR x",
            "CARGO_TARGET__RUNNER x",
            "CARGO_TARGET_X_RUNNER_2 x",
        ] {
            assert!(!names_wrapper(&l.chars().collect::<Vec<_>>()), "{l:?}");
        }
    }

    #[test]
    fn gate_1g_passes_what_the_old_patterns_passed() {
        for l in [
            "MY_RUSTC_WRAPPER: x",
            "RUSTC_WRAPPERS: x",
            "CARGO_TARGET_DIR: target",
            "CARGO_TARGET__RUNNER: x",
            "cargo build # --config x",
            "cargo build | tee --config x",
            "cargo build --configs x",
            "# CARGO_HOME: /x",
            "  # RUSTFLAGS: -C linker=x",
            "RUSTFLAGS: -D warnings # linker",
            "export FOO=1",
            "export -n $x",
            "env:",
            "environment $x=1",
            "printf '%s' -v",
            "  # GITHUB_ENV is written below",
            "if [ -n \"${GITHUB_ENV:-}\" ]; then",
            "  printf 'SOURCE_SCAN=%s\\n' \"$tool\" >> \"$GITHUB_ENV\"",
            "printf \"CARGO_TARGET_DIR=%s\\n\" \"$target_dir\" >> \"$GITHUB_ENV\"",
            "RUSTC_VERSION:",
        ] {
            let r = gate_1g_of(&["a"], l);
            assert!(!r.refused, "refused: {l:?}\n{}", r.text());
        }
        assert!(
            gate_1g_of(&[], "")
                .text()
                .contains("GATE 1G READ NO TRACKED FILE.")
        );
        let (a, b) = tools_absent();
        let tools = ToolFiles {
            toolchain: (false, &a),
            nextest: (false, &b),
        };
        assert!(gate_1g_verdict(&names(&["a"]), &tools, &[]).refused);
    }

    #[test]
    fn gate_1g_reports_in_the_steps_order() {
        let wf = vec![
            ("a.yml".to_owned(), "GITHUB_PATH\nRUSTC=1\n".to_owned()),
            ("b.yml".to_owned(), "RUSTC=2\n".to_owned()),
        ];
        assert_eq!(
            workflow_doors(&wf),
            vec!["a.yml:2:RUSTC=1", "b.yml:1:RUSTC=2", "a.yml:1:GITHUB_PATH"]
        );
    }

    // ---- gate 1f ----

    #[test]
    fn gate_1f_refuses_what_the_scanner_refused() {
        let r = gate_1f_verdict(3, &scan("crates/a.rs:1: <script>\n", 1));
        assert!(r.refused);
        assert!(r.text().starts_with(
            "BROWSER CODE IN A RUST FILE:\ncrates/a.rs:1: <script>\nscanned 3 tracked"
        ));
        assert!(r.text().contains("Browser code belongs under web/"));
        let r = gate_1f_verdict(3, &scan("", 2));
        assert!(r.refused);
        let r = gate_1f_verdict(3, &ok());
        assert!(!r.refused);
        assert!(r.text().contains("OK — no <script> body"));
    }

    // ---- gate 1c ----

    fn gate_1c_of(files: &[(&str, &str)]) -> Report {
        let map: BTreeMap<String, Vec<u8>> = files
            .iter()
            .map(|(a, b)| ((*a).to_owned(), b.as_bytes().to_vec()))
            .collect();
        let list: Vec<String> = map.keys().cloned().collect();
        gate_1c(&list, &|p: &str| {
            map.get(p).cloned().ok_or_else(|| format!("{p}: missing"))
        })
    }

    #[test]
    fn gate_1c_refuses_a_literal_parameter_path_case_blind() {
        for p in [
            joined(&["", "acmeorg", "prod", "groww", "api-key"]),
            joined(&["x ", "acmeorg", "PROD", "y"]),
            joined(&["", "_acmeorg", "staging", ""]),
            joined(&["", "-acme", "Live", ""]),
            joined(&["", "a.b", "qa", ""]),
            joined(&["", "acme", "Development", ""]),
            joined(&["", "acme", "\u{17f}tage", ""]),
            joined(&["", "acme", "SandBox", ""]),
            joined(&["", "ACME", "Uat", "x"]),
        ] {
            let r = gate_1c_of(&[("docs/x.md", &format!("ok\n{p}\n"))]);
            assert!(r.refused, "passed: {p}");
            assert!(
                r.text().contains(&format!("docs/x.md:2:{p}")),
                "{}",
                r.text()
            );
            assert!(
                r.text()
                    .contains("Use the shape /<org>/<env>/<vendor>/<field>.")
            );
        }
    }

    #[test]
    fn gate_1c_passes_the_generic_shape_and_near_misses() {
        for p in [
            "/<org>/<env>/<vendor>/<field>".to_owned(),
            joined(&["", "acme", "products", ""]),
            joined(&["", ".acme", "prod", ""]),
            joined(&["", "acme", "", "prod", ""]),
            joined(&["", "acme", "prod"]),
            joined(&["", "ac me", "prod", ""]),
            joined(&["", "acme", "dev2", ""]),
        ] {
            let r = gate_1c_of(&[("docs/x.md", &p)]);
            assert!(!r.refused, "refused: {p}");
        }
    }

    #[test]
    fn gate_1c_refuses_a_tracked_credential_config_and_reads_every_file() {
        assert!(
            gate_1c(&[], &disk)
                .text()
                .contains("GATE 1C READ NO TRACKED FILE.")
        );
        for p in [
            "credentials.toml",
            "crates/pull/credentials.toml",
            "x\ncredentials.toml",
        ] {
            let r = gate_1c_of(&[(p, "")]);
            assert!(r.refused, "{p}");
            assert!(r.text().contains("CREDENTIAL CONFIG IS TRACKED:"));
        }
        assert!(
            !gate_1c_of(&[("credentials.toml.example", ""), ("mycredentials.toml", "")]).refused
        );
        let r = gate_1c(
            &names(&["gone.md"]),
            &|p: &str| -> Result<Vec<u8>, String> { Err(format!("{p}: missing")) },
        );
        assert!(r.refused);
        assert!(r.text().contains("UNREADABLE gone.md: missing"));
        let ok = gate_1c_of(&[("a.md", "fine\n")]);
        assert_eq!(
            ok.text(),
            "OK — no literal credential path, no tracked credential config."
        );
    }

    // ---- gate 1d ----

    #[test]
    fn quoted_words_are_read_as_grep_o_read_them() {
        assert_eq!(
            quoted_words(br#"x "abc" "Abc" "a b" "_x""-y""#, b"\""),
            vec!["abc", "_x", "-y"]
        );
        let long = format!("\"{}\" \"{}\"", "a".repeat(33), "b".repeat(32));
        assert_eq!(quoted_words(long.as_bytes(), b"\""), vec!["b".repeat(32)]);
        assert_eq!(quoted_words(b"\"a\"b\"", b"\""), vec!["a"]);
        assert_eq!(quoted_words(b"\"\"", b"\""), Vec::<String>::new());
        assert_eq!(
            quoted_words(br#"{\"orgtwo\": \"x\", \\\"q\"}"#, b"\\\""),
            vec!["orgtwo", "x", "q"]
        );
        assert_eq!(quoted_words(b"\"a\nb\"", b"\""), Vec::<String>::new());
    }

    #[test]
    fn a_value_and_a_date_are_read_as_cut_and_grep_read_them() {
        assert_eq!(value_of("crates/pull/a.rs:12:x:y"), "x:y");
        assert_eq!(value_of("a:b"), "");
        assert_eq!(value_of("ab"), "ab");
        assert!(whole_segment("ab-c_9"));
        assert!(!whole_segment(""));
        assert!(!whole_segment("Ab"));
        assert!(!whole_segment(&"a".repeat(33)));
        assert!(iso_date("2026-01-02"));
        assert!(!iso_date("2026-01"));
        assert!(!iso_date("2026_01_02"));
    }

    #[test]
    fn every_declared_group_is_read() {
        let d = declared();
        for w in [
            "org",
            "groww",
            "vendor_doc",
            "abb",
            "brutex-nse-cash-session-v2",
            "86400",
            "1e300",
            "-9223372036854775808",
            "--nocapture",
            "repeats",
            "api-key",
        ] {
            assert!(d.contains(w), "{w}");
        }
        assert!(!d.contains("acmeorg"));
    }

    fn gate_1d_of(files: &[(&str, &str)], strings: &str) -> Report {
        let map: BTreeMap<String, Vec<u8>> = files
            .iter()
            .map(|(a, b)| ((*a).to_owned(), b.as_bytes().to_vec()))
            .collect();
        let list: Vec<String> = map.keys().cloned().collect();
        gate_1d_verdict(
            &list,
            &|p: &str| map.get(p).cloned().ok_or_else(|| p.to_owned()),
            &scan(strings, 0),
        )
        .unwrap()
    }

    #[test]
    fn gate_1d_refuses_an_undeclared_literal_wherever_it_sits() {
        let rs = ("crates/pull/src/a.rs", "");
        for (files, strings, word) in [
            (vec![rs], "crates/pull/src/a.rs:3:acmeorg\n", "acmeorg"),
            (vec![rs], "crates/pull/src/a.rs:3:_acmeorg\n", "_acmeorg"),
            (
                vec![rs],
                "crates/pull/src/a.rs:3:{\\\"orgtwo2\\\": 1}\n",
                "orgtwo2",
            ),
            (
                vec![rs, ("crates/pull/Cargo.toml", "name = \"acmeorg\"\n")],
                "crates/pull/src/a.rs:1:groww\n",
                "acmeorg",
            ),
        ] {
            let r = gate_1d_of(&files, strings);
            assert!(r.refused, "{word}");
            assert!(
                r.text().contains(&format!(
                    "UNDECLARED SEGMENT-SHAPED LITERAL IN crates/pull: {word}"
                )),
                "{}",
                r.text()
            );
        }
        let r = gate_1d_of(
            &[rs],
            "crates/pull/src/a.rs:3:acmeorg\ncrates/pull/src/a.rs:4:acmeorg\n",
        );
        assert!(r.text().contains("acmeorg\ncrates/pull/src/a.rs:3:acmeorg\ncrates/pull/src/a.rs:4:acmeorg\nread 1 tracked file(s), 1 distinct"));
    }

    #[test]
    fn gate_1d_passes_declared_dates_and_unshaped_literals() {
        let r = gate_1d_of(
            &[
                ("crates/pull/src/a.rs", ""),
                ("crates/pull/Cargo.toml", "x = \"groww\"\n"),
            ],
            "f:1:groww\nf:2:2026-01-02\nf:3:Has Upper\nf:4:{\\\"dhan\\\"}\nf:5:a/b\n",
        );
        assert!(!r.refused, "{}", r.text());
        assert!(
            r.text()
                .contains("read 2 tracked file(s), 2 distinct segment-shaped literal(s)")
        );
    }

    #[test]
    fn gate_1d_refuses_a_failed_or_empty_scan() {
        let list = names(&["crates/pull/src/a.rs"]);
        let none = |p: &str| -> Result<Vec<u8>, String> { Err(p.to_owned()) };
        let r = gate_1d_verdict(&list, &none, &scan("f:1:x\n", 2)).unwrap();
        assert!(r.refused);
        let r = gate_1d_verdict(&list, &none, &ok()).unwrap();
        assert!(r.text().contains("GATE 1D READ NO STRING LITERAL"));
        assert!(
            gate_1d_verdict(
                &names(&["crates/pull/Cargo.toml"]),
                &none,
                &scan("f:1:x\n", 0)
            )
            .is_err()
        );
    }

    // ---- gate 2 ----

    #[test]
    fn gate_2_reads_a_build_or_links_key_in_any_spacing() {
        for (text, n) in [
            ("[package]\nbuild = \"gen.rs\"\n", 1),
            ("links=\"z\"\n", 1),
            ("  build\t =x\n", 1),
            ("build\u{2003}= 1\n", 1),
            ("x = 1\r\nlinks = 'a'\r\n", 1),
            ("# build = \"x\"\nname = \"a\" # links = 1\n", 0),
            ("build.workspace = true\n", 0),
            ("rebuild = 1\n", 0),
            ("x = \"a:build = 1\"\n", 1),
        ] {
            assert_eq!(
                manifest_keys("crates/x/Cargo.toml", text.as_bytes()).len(),
                n,
                "{text:?}"
            );
        }
        assert_eq!(manifest_keys("m", b"a\nbuild = 1\n"), vec!["m:2:build = 1"]);
        assert_eq!(manifest_keys("m", b"build = 1 # \xff\n").len(), 1);
        assert!(is_manifest("Cargo.toml"));
        assert!(is_manifest("crates/a/Cargo.toml"));
        assert!(!is_manifest("crates/a/NotCargo.toml"));
    }

    #[test]
    fn gate_2_reports_the_closure_and_every_refusal() {
        let roots = names(&["crates/cli/build.rs"]);
        let r = gate_2_verdict(&[], &ok(), &ok(), &[]);
        assert!(!r.refused);
        assert!(r.text().contains("  none — no tracked build.rs"));
        let r = gate_2_verdict(
            &roots,
            &scan("crates/cli/build.rs\ncrates/cli/x.rs\n", 0),
            &ok(),
            &[],
        );
        assert!(!r.refused);
        assert!(
            r.text()
                .contains("  crates/cli/build.rs\n  crates/cli/x.rs\nOK —")
        );
        let r = gate_2_verdict(
            &roots,
            &scan("UNRESOLVED a: b\ncrates/cli/build.rs\n", 1),
            &ok(),
            &[],
        );
        assert!(r.refused);
        assert!(
            r.text()
                .contains("CANNOT RESOLVE:\n    UNRESOLVED a: b\n  crates/cli/build.rs")
        );
        let r = gate_2_verdict(
            &roots,
            &scan("crates/cli/build.rs\n", 0),
            &scan("crates/cli/build.rs:3: names Command\n", 1),
            &[],
        );
        assert!(r.text().contains(
            "EXTERNAL PROCESS:\n  crates/cli/build.rs:3: names Command\nA build script is Rust"
        ));
        let m = vec![(
            "crates/x/Cargo.toml".to_owned(),
            b"links = \"z\"\n".to_vec(),
        )];
        let r = gate_2_verdict(&[], &ok(), &ok(), &m);
        assert!(r.refused);
        assert!(r.text().contains("A MANIFEST NAMES A BUILD SCRIPT OR A NATIVE LIBRARY:\ncrates/x/Cargo.toml:1:links = \"z\""));
        assert_eq!(
            closure_files("UNRESOLVED x\na b.rs\n\nc.rs\n"),
            vec!["a b.rs", "c.rs"]
        );
    }

    // ---- gates 9, 9b ----

    #[test]
    fn a_standalone_crate_refuses_absence_unreadability_and_any_declaration() {
        let r = gate_9(false, &ok());
        assert!(
            r.text()
                .contains("REFUSED  crates/core/Cargo.toml is not a tracked file.")
        );
        let r = gate_9(true, &scan("", 2));
        assert!(r.text().contains("could not be read as TOML"));
        let r = gate_9(
            true,
            &scan("crates/core/Cargo.toml:9:dev-dependencies:x:x\n\n", 0),
        );
        assert!(r.refused);
        assert!(
            r.text().contains(
                "Found:\ncrates/core/Cargo.toml:9:dev-dependencies:x:x\ncore is compiled"
            )
        );
        let r = gate_9(true, &scan("\n", 0));
        assert!(!r.refused);
        assert!(r.text().contains("OK — core declares no dependency"));
        let r = gate_9b(true, &scan("m:1:dependencies:serde:serde\n", 0), &[]);
        assert!(
            r.text()
                .contains("crates/greeks MUST DEPEND ON NOTHING. Found:")
        );
        assert!(r.text().contains("See D-0046."));
        assert!(gate_9b(false, &ok(), &[]).refused);
    }

    #[test]
    fn gate_9b_refuses_a_workspace_type_at_a_word_boundary() {
        for l in [
            "pub use brutex_core::price::Paisa;",
            "x: core::price::Paisa",
            "(core::instrument)",
            "core::universe",
            "a core::vendor::Vendor",
            "\u{e9}brutex_core",
        ] {
            let src = vec![(
                "crates/greeks/src/lib.rs".to_owned(),
                format!("//! x\n{l}\n").into_bytes(),
            )];
            let r = gate_9b(true, &ok(), &src);
            assert!(r.refused, "{l}");
            assert!(
                r.text()
                    .contains(&format!("crates/greeks/src/lib.rs:2:{l}"))
            );
        }
        for l in [
            "brutex_core2",
            "my_core::price",
            "core::prices",
            "core::vendors",
            "xcore::price",
            "std::core::x",
        ] {
            let src = vec![("a.rs".to_owned(), l.as_bytes().to_vec())];
            assert!(!gate_9b(true, &ok(), &src).refused, "{l}");
        }
    }

    // ---- gate 10b ----

    #[test]
    fn gate_10b_refuses_an_identifier_defined_twice() {
        let t = "| I-16 | a |\n| I-16a | b |\n| X-08 | c |\n| I-16 | d |\n| X-08 | e |\n";
        let r = gate_10b(t);
        assert!(r.refused);
        assert!(
            r.text()
                .starts_with("INVARIANT IDENTIFIER DEFINED MORE THAN ONCE:\nI-16\nX-08\n")
        );
        let r = gate_10b("| I-1 | a |\n| I-1a | b |\n| I-1b | c |\n");
        assert!(!r.refused);
        assert!(gate_10b("no table\n").refused);
        for l in [
            "|  I-1 ", "| i-1 ", "| I-1ab ", "| I1 ", "| I- ", "| I-1", "x| I-1 ", "| I-1| ",
        ] {
            assert_eq!(invariant_id(l), None, "{l}");
        }
        assert_eq!(invariant_id("| AFG-42 | x"), Some("AFG-42"));
        assert_eq!(invariant_id("| CIG-13a |"), Some("CIG-13a"));
    }

    // ---- gate 7 ----

    #[test]
    fn gate_7_skips_when_the_crate_is_absent_and_holds_it_to_core() {
        let r = gate_7(None);
        assert!(!r.refused);
        assert!(r.text().starts_with("skip — crates/web does not exist"));
        assert!(
            !gate_7(Some(
                "[package]\nname = \"web\"\n[dependencies]\ncore = { path = \"../core\" }\n"
            ))
            .refused
        );
        let r = gate_7(Some(
            "[dependencies]\ncore = 1\nserde = \"1\"\n\t tokio\t= 1\n[dev-dependencies]\nx = 1\n",
        ));
        assert!(r.refused);
        assert_eq!(
            r.text(),
            "crates/web MAY ONLY DEPEND ON core. Found:\nserde\ntokio\nIt compiles to wasm32 where the filesystem does not exist."
        );
    }

    // ---- dispatch ----

    #[test]
    fn an_unknown_gate_or_a_missing_argument_is_an_error() {
        assert!(run(&[]).is_err());
        assert!(run(&names(&["gate-99"])).is_err());
        assert!(run(&names(&["gate-1", "sideways"])).is_err());
        assert!(run(&names(&["gate-1e", "verdict", "lint", "/x", "0"])).is_err());
        assert!(exactly("crates/web/Cargo.toml", CORE.manifest).is_err());
        assert!(exactly(CORE.manifest, CORE.manifest).is_ok());
    }
}

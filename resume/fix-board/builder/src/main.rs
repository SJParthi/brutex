//! fixboard: rebuilds the Brutex Fix Board's per-finding ledger.
//!
//! One row per finding, with owner, state (found, fixing, branch, pushed, green,
//! doc, partial) and the flags no-owner / stuck / new. The spec it follows is
//! `resume/memory-20261004/brutex-fix-board-artifact.md` on fix-queue.
//!
//! Inputs, applied in this order (each later one wins):
//! 1. the catalog: the `ledger-data` block of the board page (every finding the
//!    earlier sources ever produced; rows are never dropped),
//! 2. finding files that keep growing (`--zero-dir`): new ids are appended,
//! 3. the previous snapshot (`--snapshot`), which also supplies `prev`,
//! 4. per-thread status TSVs (`--status`, repeatable, id/state/commit/note),
//! 5. git evidence (`--repo`, `--pr-ref`): a status commit that is an ancestor
//!    of the PR head makes the row `pushed`; a `pushed` claim whose commit is
//!    provably not on the head is demoted to `branch` and flagged,
//! 6. CI (`--checks`): when the `ci-ok` row passed, `pushed` becomes `green`.
//!
//! Outputs: the page with both data blocks replaced, and the new snapshot.

use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

const DONE: [&str; 3] = ["pushed", "green", "doc"];
const STATES: [&str; 7] = [
    "found", "fixing", "branch", "partial", "pushed", "green", "doc",
];
const ZERO_OWNER: &str = "Audit rounds until zero findings";

#[derive(Default)]
struct Args {
    board: PathBuf,
    out: PathBuf,
    snapshot: Option<PathBuf>,
    snapshot_out: Option<PathBuf>,
    status: Vec<PathBuf>,
    zero_dir: Option<PathBuf>,
    repo: Option<PathBuf>,
    pr_ref: String,
    checks: Option<PathBuf>,
    pr: Option<PathBuf>,
    streams: Option<PathBuf>,
    corrections: Option<PathBuf>,
    /// The PR head the corrections were checked against; a status commit on the current
    /// head that this ref does not contain is newer evidence and lifts the correction.
    checked_at: Option<String>,
    same_as: Option<PathBuf>,
    not_a_fix: Vec<String>,
    base_ref: String,
    as_of: String,
    link_base: String,
}

fn parse_args() -> Result<Args, String> {
    let mut a = Args {
        checked_at: None,
        pr_ref: "origin/final/all-fixes".into(),
        base_ref: "origin/main".into(),
        link_base: "https://github.com/SJParthi/brutex/blob/fix-queue/".into(),
        ..Args::default()
    };
    let mut it = std::env::args().skip(1);
    while let Some(k) = it.next() {
        let mut v = || it.next().ok_or_else(|| format!("{k} needs a value"));
        match k.as_str() {
            "--board" => a.board = v()?.into(),
            "--out" => a.out = v()?.into(),
            "--snapshot" => a.snapshot = Some(v()?.into()),
            "--snapshot-out" => a.snapshot_out = Some(v()?.into()),
            "--status" => a.status.push(v()?.into()),
            "--zero-dir" => a.zero_dir = Some(v()?.into()),
            "--repo" => a.repo = Some(v()?.into()),
            "--pr-ref" => a.pr_ref = v()?,
            "--checks" => a.checks = Some(v()?.into()),
            "--pr" => a.pr = Some(v()?.into()),
            "--streams" => a.streams = Some(v()?.into()),
            "--corrections" => a.corrections = Some(v()?.into()),
            "--checked-at" => a.checked_at = Some(v()?),
            "--same-as" => a.same_as = Some(v()?.into()),
            "--not-a-fix" => a.not_a_fix.push(v()?),
            "--base-ref" => a.base_ref = v()?,
            "--as-of" => a.as_of = v()?,
            "--link-base" => a.link_base = v()?,
            _ => return Err(format!("unknown argument {k}")),
        }
    }
    if a.board.as_os_str().is_empty() || a.out.as_os_str().is_empty() || a.as_of.is_empty() {
        return Err("--board, --out and --as-of are required".into());
    }
    Ok(a)
}

fn read(p: &Path) -> Result<String, String> {
    fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()))
}

/// The JSON text between `<script ... id="NAME">` and `</script>`.
fn script_span(html: &str, id: &str) -> Option<(usize, usize)> {
    let tag = format!("id=\"{id}\">");
    let open = html.find(&tag)? + tag.len();
    let close = open + html[open..].find("</script>")?;
    Some((open, close))
}

/// A finding parsed from a growing source file.
struct Found {
    id: String,
    group: &'static str,
    sev: String,
    title: String,
    link: String,
    area: String,
}

fn finding_bullet(l: &str) -> bool {
    l.trim_start()
        .strip_prefix("- **")
        .and_then(|r| r.find("**").map(|e| id_like(&r[..e])))
        .unwrap_or(false)
}

/// The lines of the section a finding heads: its own heading or bullet line, up to
/// the next heading or the next finding bullet. A range heading (`CE-18..CE-22`) is
/// not the section of CE-18; its bullet is.
fn section<'a>(text: &'a str, id: &str) -> Vec<&'a str> {
    let bullet = format!("- **{id}**");
    let mut lines = text.lines().skip_while(|l| {
        let h = l.trim_start_matches('#');
        let is_head = h.len() != l.len() && h.trim().starts_with(id) && {
            let r = &h.trim()[id.len()..];
            r.is_empty()
                || r.starts_with([' ', ':', '('])
                || (r.starts_with('.') && !r.starts_with(".."))
        };
        !(is_head || l.trim_start().starts_with(&bullet))
    });
    let Some(first) = lines.next() else {
        return Vec::new();
    };
    std::iter::once(first)
        .chain(lines.take_while(|l| !l.starts_with('#') && !finding_bullet(l)))
        .collect()
}

/// The first source path (`crates/x/src/f.rs:12` or `x/src/f.rs`) in a finding's section,
/// else the first bare file (`autopilot.rs:2642`), which the caller places in its crate.
fn area_for(text: &str, id: &str) -> String {
    area_in(section(text, id))
}

/// The first crate path in `lines`; else a bare `name.rs:line` (placed later from the
/// tree); else the first repository document or workflow the lines name.
fn area_in<'a>(lines: impl IntoIterator<Item = &'a str>) -> String {
    let mut bare = String::new();
    let mut doc = String::new();
    for l in lines {
        for tok in l.split(|c: char| {
            c.is_whitespace() || matches!(c, '`' | ',' | '(' | ')' | ';' | '*' | '[' | ']')
        }) {
            let tok = tok.trim_end_matches(['.', ':']);
            if tok.contains("/src/") || (tok.starts_with("crates/") && tok.len() > 7) {
                return tok.trim_start_matches("crates/").to_string();
            }
            let file = tok.split(':').next().unwrap_or("");
            if bare.is_empty()
                && file.len() > 3
                && file.ends_with(".rs")
                && !file.contains('/')
                && file[..file.len() - 3]
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_')
            {
                bare = tok.to_string();
            }
            if doc.is_empty()
                && (tok.starts_with("docs/") && tok.len() > 5
                    || tok.starts_with(".github/") && tok.len() > 8
                    || matches!(file, "CLAUDE.md" | "AGENTS.md")
                    || file.ends_with(".yml") && !file.contains(char::is_whitespace))
            {
                doc = tok.to_string();
            }
        }
    }
    if bare.is_empty() {
        doc
    } else {
        bare
    }
}

/// `name.rs` -> `crate/src/.../name.rs` for every file name that exists exactly once
/// under `crates/` on the PR head.
fn unique_files(repo: &Path, head: &str) -> HashMap<String, String> {
    let Ok(out) = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["ls-tree", "-r", "--name-only", head, "crates"])
        .output()
    else {
        return HashMap::new();
    };
    let mut seen: HashMap<String, Option<String>> = HashMap::new();
    for path in String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|p| p.ends_with(".rs"))
    {
        let name = path.rsplit('/').next().unwrap_or(path).to_string();
        let rel = path.trim_start_matches("crates/").to_string();
        seen.entry(name)
            .and_modify(|v| *v = None)
            .or_insert(Some(rel));
    }
    seen.into_iter()
        .filter_map(|(k, v)| v.map(|v| (k, v)))
        .collect()
}

/// A bare `name.rs:12` area placed in its crate when the name is unique on the head.
fn place(area: &str, files: &HashMap<String, String>) -> Option<String> {
    let (file, rest) = area.split_once(':').unwrap_or((area, ""));
    if file.contains('/') {
        return None;
    }
    let full = files.get(file)?;
    Some(if rest.is_empty() {
        full.clone()
    } else {
        format!("{full}:{rest}")
    })
}

/// A `Severity: high` line inside a finding's own section.
fn section_severity(text: &str, id: &str) -> String {
    for l in section(text, id) {
        let low = l.to_ascii_lowercase();
        if let Some(i) = low.find("severity") {
            let sev = severity(
                &low[i + "severity".len()..]
                    .chars()
                    .take(30)
                    .collect::<String>(),
            );
            if !sev.is_empty() {
                return sev;
            }
        }
    }
    String::new()
}

/// The first severity word in `s`, matched as a whole word ("follows" is not "low").
fn severity(s: &str) -> String {
    let l = s.to_ascii_lowercase();
    let b = l.as_bytes();
    let mut best: Option<(usize, &str)> = None;
    for w in ["high", "medium", "low", "info"] {
        let mut from = 0;
        while let Some(k) = l[from..].find(w) {
            let i = from + k;
            let end = i + w.len();
            let edge = |c: Option<&u8>| c.is_none_or(|c| !c.is_ascii_alphanumeric());
            if edge(i.checked_sub(1).and_then(|j| b.get(j))) && edge(b.get(end)) {
                if best.is_none_or(|(j, _)| i < j) {
                    best = Some((i, w));
                }
                break;
            }
            from = end;
        }
    }
    best.map_or("", |(_, w)| w).to_string()
}

fn clean_title(s: &str) -> String {
    let mut t = s.trim();
    // strip a leading "(sev...)", "[sev]", ":", "—", "-", or a bare sev word
    loop {
        let before = t;
        if t.starts_with('(') || t.starts_with('[') {
            let close = if t.starts_with('(') { ')' } else { ']' };
            if let Some(i) = t.find(close) {
                t = &t[i + 1..];
            }
        }
        t = t.trim_start_matches([':', '—', '-', '.', ' ']);
        for w in ["high", "medium", "low", "info"] {
            if let Some(r) = t.strip_prefix(w) {
                if r.starts_with([' ', ':', '.', ',', ';']) || r.starts_with(" —") {
                    t = r;
                }
            }
        }
        t = t.trim_start_matches([':', '—', '-', '.', ',', ';', ' ']);
        if t == before {
            break;
        }
    }
    let t = t.replace('`', "");
    if t.chars().count() > 160 {
        t.chars().take(159).collect::<String>() + "…"
    } else {
        t
    }
}

/// An id shaped like `word-N`, `word1-N`, `CE-N`, `P1-02-03` or `Fn`.
fn id_like(s: &str) -> bool {
    let parts: Vec<&str> = s.split('-').collect();
    parts.len() >= 2
        && parts[0].chars().all(|c| c.is_ascii_alphanumeric())
        && parts[0]
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic())
        && parts[1..]
            .iter()
            .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
}

/// Headings `## ID ...` / `### ID ...` in a markdown finding file.
fn headings(text: &str, accept: impl Fn(&str) -> bool) -> Vec<(String, String, String)> {
    let mut out = Vec::new();
    for line in text.lines() {
        let h = line.trim_start_matches('#');
        if h.len() == line.len() || !(2..=4).contains(&(line.len() - h.len())) {
            continue;
        }
        let h = h.trim();
        let end = h.find([' ', ':', '(']).unwrap_or(h.len());
        let id = h[..end].trim_end_matches(['.', ':']);
        if !accept(id) {
            continue;
        }
        let rest = &h[end..];
        // only a heading that names a severity or carries a title separator is a finding
        let head = rest.chars().take(40).collect::<String>();
        if !(rest.starts_with(':')
            || rest.starts_with(" (")
            || rest.starts_with('(')
            || !severity(&head).is_empty())
        {
            continue;
        }
        out.push((id.to_string(), severity(&head), clean_title(rest)));
    }
    out
}

/// `- **CE-37** text` bullets (crash-edge pass 4 lists several ids under one heading).
fn bullets(text: &str, prefix: &str) -> Vec<(String, String, String)> {
    let mut out = Vec::new();
    let mut group_sev = String::new();
    for line in text.lines() {
        if line.starts_with('#') {
            // a range heading such as "CE-36..CE-39: ... (all low; ...)" rates its bullets
            group_sev = severity(line);
            continue;
        }
        let Some(r) = line.trim_start().strip_prefix("- **") else {
            continue;
        };
        let Some(end) = r.find("**") else { continue };
        let id = &r[..end];
        if id.starts_with(prefix) && id_like(id) {
            let mut sev = severity(&r[end..].chars().take(40).collect::<String>());
            if sev.is_empty() {
                sev = group_sev.clone();
            }
            out.push((id.to_string(), sev, clean_title(&r[end + 2..])));
        }
    }
    out
}

/// Markdown table rows `| id | sev | where | what | ...` under a header naming `sev`.
fn sev_tables(text: &str) -> Vec<(String, String, String, String)> {
    let mut out = Vec::new();
    let mut in_sev_table = false;
    for line in text.lines() {
        let t = line.trim();
        if !t.starts_with('|') {
            in_sev_table = false;
            continue;
        }
        let cells: Vec<&str> = t.trim_matches('|').split('|').map(str::trim).collect();
        if cells.first() == Some(&"id") {
            in_sev_table = cells.get(1) == Some(&"sev");
            continue;
        }
        if in_sev_table && cells.len() >= 4 && id_like(cells[0]) {
            out.push((
                cells[0].to_string(),
                cells[1].to_string(),
                cells[2].to_string(),
                clean_title(cells[3]),
            ));
        }
    }
    out
}

/// The pass a helper file belongs to: the number after the last `pass` in its path, else 1.
fn pass_of(name: &str) -> u32 {
    name.rmatch_indices("pass")
        .find_map(|(at, _)| {
            let digits: String = name[at + 4..]
                .chars()
                .take_while(char::is_ascii_digit)
                .collect();
            digits.parse().ok()
        })
        .unwrap_or(1)
}

fn files_in(dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = fs::read_dir(dir)
        .map(|r| r.filter_map(|e| e.ok().map(|e| e.path())).collect())
        .unwrap_or_default();
    v.sort();
    v
}

fn zero_sources(dir: &Path, link_base: &str, rel: &str) -> Result<Vec<Found>, String> {
    let mut out = Vec::new();
    // a live folder outside the repo is linked by its own path
    let link = |f: &str| {
        if rel.is_empty() {
            dir.join(f).display().to_string()
        } else {
            format!("{link_base}{rel}/{f}")
        }
    };
    // every file of a family: `crash-edge.md`, then `crash-edge-pass5.md`, ... in name order
    let family = |stem: &str| -> Vec<PathBuf> {
        files_in(dir)
            .into_iter()
            .filter(|f| {
                let n = f
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                n == format!("{stem}.md")
                    || (n.starts_with(&format!("{stem}-")) && n.ends_with(".md"))
            })
            .collect()
    };
    let push = |out: &mut Vec<Found>,
                group,
                (id, sev, title): (String, String, String),
                l: String,
                area: String| {
        out.push(Found {
            id,
            group,
            sev,
            title,
            link: l,
            area,
        })
    };
    // concurrency helper: summary file, then every pass directory
    let mut conc = vec![dir.join("concurrency.md")];
    for p in files_in(dir).into_iter().filter(|p| {
        p.is_dir()
            && p.file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with("conc-pass"))
    }) {
        conc.extend(
            files_in(&p)
                .into_iter()
                .filter(|f| f.extension().is_some_and(|e| e == "md")),
        );
    }
    for f in conc.iter().filter(|f| f.exists()) {
        let rel_f = f.strip_prefix(dir).unwrap_or(f).display().to_string();
        let t = read(f)?;
        for h in headings(&t, |id| {
            id_like(id) && id.chars().next().is_some_and(|c| c.is_ascii_lowercase())
        }) {
            let area = area_for(&t, &h.0);
            let mut h = h;
            if h.1.is_empty() {
                h.1 = section_severity(&t, &h.0);
            }
            push(
                &mut out,
                "Zero-rounds: concurrency",
                (format!("conc:{}", h.0), h.1, h.2),
                link(&rel_f),
                area,
            );
        }
    }
    for ce in family("crash-edge") {
        let name = ce
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let t = read(&ce)?;
        for h in headings(&t, |id| id.starts_with("CE-") && id_like(id))
            .into_iter()
            .chain(bullets(&t, "CE-"))
        {
            let area = area_for(&t, &h.0);
            let mut h = h;
            if h.1.is_empty() {
                h.1 = section_severity(&t, &h.0);
            }
            push(
                &mut out,
                "Zero-rounds: crashes and edge inputs",
                h,
                link(&name),
                area,
            );
        }
    }
    for td in family("tests-docs-security") {
        let name = td
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let t = read(&td)?;
        let accept = |id: &str| {
            id.starts_with('P') && id_like(id) && (2..=3).contains(&id.split('-').count())
        };
        for h in headings(&t, accept) {
            let area = area_for(&t, &h.0);
            let mut h = h;
            if h.1.is_empty() {
                h.1 = section_severity(&t, &h.0);
            }
            push(
                &mut out,
                "Zero-rounds: tests, docs, security",
                h,
                link(&name),
                area,
            );
        }
    }
    for np in family("numeric")
        .into_iter()
        .filter(|f| !f.ends_with("numeric-complexity.md"))
    {
        let name = np
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let t = read(&np)?;
        for h in headings(&t, |id| {
            id_like(id) && id.chars().next().is_some_and(|c| c.is_ascii_lowercase())
        }) {
            let area = area_for(&t, &h.0);
            let mut h = h;
            if h.1.is_empty() {
                h.1 = section_severity(&t, &h.0);
            }
            push(
                &mut out,
                "Zero-rounds: numbers and complexity",
                (format!("num:{}", h.0), h.1, h.2),
                link(&name),
                area,
            );
        }
    }
    let nc = dir.join("numeric-complexity.md");
    if nc.exists() {
        for (id, sev, area, title) in sev_tables(&read(&nc)?) {
            push(
                &mut out,
                "Zero-rounds: numbers and complexity",
                (format!("num:{id}"), sev, title),
                link("numeric-complexity.md"),
                area,
            );
        }
    }
    // round-1 slices of the zero-findings loop itself: `## F1 [sev] title` in sliceNN.md
    for f in files_in(&dir.join("r1-slices")) {
        let name = f
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let Some(stem) = name.strip_suffix(".md") else {
            continue;
        };
        if !stem.starts_with("slice") || stem.contains('.') {
            continue;
        }
        let t = read(&f)?;
        for h in headings(&t, |id| {
            id.len() >= 2 && id.starts_with('F') && id[1..].chars().all(|c| c.is_ascii_digit())
        }) {
            let area = area_for(&t, &h.0);
            let mut h = h;
            if h.1.is_empty() {
                h.1 = section_severity(&t, &h.0);
            }
            push(
                &mut out,
                "Zero-findings loop: round 1",
                (format!("Z1-{stem}-{}", h.0), h.1, h.2),
                link(&format!("r1-slices/{name}")),
                area,
            );
        }
    }
    Ok(out)
}

/// A helper's verification verdict on one finding at a named head.
#[derive(Debug, PartialEq, Clone, Copy)]
enum Verdict {
    Fixed,
    Partial,
    NotFixed,
}

/// Rows of every verification table (`| id | state | evidence |`, `| CE | state | ...`,
/// `| id | claimed by | verdict | evidence |`) in the helpers' pass files.
fn verdict_rows(text: &str) -> Vec<(String, Verdict, String)> {
    let mut out = Vec::new();
    let mut col: Option<usize> = None;
    for line in text.lines() {
        let l = line.trim();
        if !l.starts_with('|') {
            col = None;
            continue;
        }
        let cells: Vec<&str> = l.trim_matches('|').split('|').map(str::trim).collect();
        let first = cells
            .first()
            .map(|c| c.to_ascii_lowercase())
            .unwrap_or_default();
        if first == "id" || first == "ce" {
            col = cells.iter().position(|c| {
                let c = c.to_ascii_lowercase();
                c == "state" || c == "verdict"
            });
            continue;
        }
        let Some(k) = col else { continue };
        let (Some(id), Some(v)) = (cells.first(), cells.get(k)) else {
            continue;
        };
        let up = v.to_ascii_uppercase();
        let verdict =
            if up.starts_with("NOT FIXED") || up.starts_with("OPEN") || up.starts_with("STILL") {
                Verdict::NotFixed
            } else if up.starts_with("PARTIAL") {
                Verdict::Partial
            } else if up.starts_with("FIXED") {
                Verdict::Fixed
            } else {
                continue;
            };
        let evidence = cells.get(k + 1).map_or(String::new(), |e| clean_title(e));
        out.push((id.to_string(), verdict, evidence));
    }
    out
}

/// Ids a status line names: `A`, `A / -8`, `D-1631 (W2-cli1-2/-3)`, `W2-cli14-1/2/3`.
fn status_ids(raw: &str) -> Vec<String> {
    let mut out = Vec::new();
    let flat = raw.replace(['(', ')', ','], " ").replace(" / ", "/");
    for tok in flat.split_whitespace() {
        let mut base = String::new();
        for (i, part) in tok.split('/').enumerate() {
            if i == 0 {
                base = part.to_string();
                out.push(base.clone());
            } else if let Some(cut) = base.rfind('-') {
                let suffix = part.trim_start_matches('-');
                if !suffix.is_empty() && suffix.chars().all(|c| c.is_ascii_digit()) {
                    out.push(format!("{}-{suffix}", &base[..cut]));
                }
            }
        }
    }
    out
}

fn hex_tokens(s: &str) -> Vec<String> {
    s.split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|t| {
            (7..=40).contains(&t.len())
                && t.chars()
                    .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        })
        .filter(|t| t.chars().any(|c| c.is_ascii_alphabetic()) || t.len() >= 9)
        .map(str::to_string)
        .collect()
}

#[derive(Clone, Copy, PartialEq)]
enum OnHead {
    Yes,
    No,
    Unknown,
}

fn git_ok(repo: &Path, args: &[&str]) -> Option<bool> {
    Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .ok()
        .map(|o| o.status.success())
}

fn on_head(repo: &Path, head: &str, sha: &str, cache: &mut HashMap<String, OnHead>) -> OnHead {
    if let Some(v) = cache.get(sha) {
        return *v;
    }
    let v = if git_ok(repo, &["cat-file", "-e", &format!("{sha}^{{commit}}")]) != Some(true) {
        OnHead::Unknown
    } else if git_ok(repo, &["merge-base", "--is-ancestor", sha, head]) == Some(true) {
        OnHead::Yes
    } else {
        OnHead::No
    };
    cache.insert(sha.to_string(), v);
    v
}

/// Tab-separated rows after the header. A line whose first cell opens more
/// parentheses than it closes was wrapped by its writer: it is joined with the
/// next line, whose other cells are the entry's.
fn tsv(p: &Path) -> Result<Vec<Vec<String>>, String> {
    let text = read(p)?;
    let mut out: Vec<Vec<String>> = Vec::new();
    let mut open: Option<String> = None;
    for l in text.lines().skip(1).filter(|l| !l.trim().is_empty()) {
        let mut cells: Vec<String> = l.split('\t').map(|c| c.trim().to_string()).collect();
        if let Some(head) = open.take() {
            cells[0] = format!("{head} {}", cells[0]);
        }
        let depth = cells[0].matches('(').count() as i64 - cells[0].matches(')').count() as i64;
        if depth > 0 {
            open = Some(cells[0].clone());
            continue;
        }
        out.push(cells);
    }
    if let Some(head) = open {
        return Err(format!(
            "{}: unbalanced parentheses in the last entry {head}",
            p.display()
        ));
    }
    Ok(out)
}

/// found < fixing = partial < branch < doc < pushed < green
fn rank(state: &str) -> u8 {
    match state {
        "found" => 0,
        "fixing" | "partial" => 1,
        "branch" => 2,
        "doc" => 3,
        "pushed" => 4,
        "green" => 5,
        _ => 0,
    }
}

/// Finding ids named on a commit-message line that reads as a fix. A line that says
/// the finding is still open, not fixed, or reopened does not count.
const NOT_A_FIX: [&str; 25] = [
    "still open",
    "not fixed",
    "reopen",
    "todo",
    "left open",
    "not yet",
    "unfixed",
    "deferred",
    "remains open",
    "not this",
    "stated",
    "documented",
    "blocked",
    "not changed",
    " half",
    "limits",
    "unverified",
    "queued",
    "state that",
    "unwired",
    "not wired",
    "they stay",
    "detect no",
    "refuse no",
    "and leave",
];

/// Finding ids named on a fix line of each chunk `label\0text\x1e` (a commit
/// message, or a decision entry added on the PR).
fn message_mentions(log: &str) -> HashMap<String, (String, String)> {
    let mut out: HashMap<String, (String, String)> = HashMap::new();
    for commit in log.split('\u{1e}') {
        let Some((sha, body)) = commit.trim_start().split_once('\0') else {
            continue;
        };
        for line in body.lines() {
            let low = line.to_ascii_lowercase();
            if NOT_A_FIX.iter().any(|w| low.contains(w)) {
                continue;
            }
            for tok in line.split(|c: char| !(c.is_ascii_alphanumeric() || c == '-')) {
                let tok = tok.trim_matches('-');
                let parts: Vec<&str> = tok.split('-').collect();
                let shaped = parts.len() >= 2
                    && tok.starts_with(|c: char| c.is_ascii_alphabetic())
                    && parts
                        .iter()
                        .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_alphanumeric()))
                    && parts
                        .last()
                        .is_some_and(|p| p.chars().all(|c| c.is_ascii_digit()));
                if shaped && !tok.starts_with("D-") {
                    out.entry(tok.to_string())
                        .or_insert_with(|| (sha.to_string(), line.trim().to_string()));
                }
            }
        }
    }
    out
}

/// Added decision entries from a diff of `docs/05-decisions.md`, as
/// `decision D-n on head\0lines\x1e` chunks, skipping entries whose title is not a fix.
fn decision_chunks(diff: &str) -> String {
    let mut out = String::new();
    let mut cur: Option<(String, String)> = None;
    let flush = |cur: &mut Option<(String, String)>, out: &mut String| {
        if let Some((label, body)) = cur.take() {
            out.push_str(&format!("{label}\0{body}\u{1e}"));
        }
    };
    for l in diff.lines() {
        let Some(added) = l.strip_prefix('+') else {
            continue;
        };
        if let Some(head) = added.strip_prefix("### D-") {
            flush(&mut cur, &mut out);
            let num: String = head.chars().take_while(char::is_ascii_digit).collect();
            let low = head.to_ascii_lowercase();
            cur = (!NOT_A_FIX.iter().any(|w| low.contains(w)))
                .then(|| (format!("decision D-{num} on head"), String::new()));
        } else if let Some((_, body)) = cur.as_mut() {
            body.push_str(added);
            body.push('\n');
        }
    }
    flush(&mut cur, &mut out);
    out
}

fn s<'a>(r: &'a Map<String, Value>, k: &str) -> &'a str {
    r.get(k).and_then(Value::as_str).unwrap_or("")
}

fn run() -> Result<(), String> {
    let a = parse_args()?;
    let html = read(&a.board)?;
    let (lo, hi) = script_span(&html, "ledger-data").ok_or("board has no ledger-data block")?;
    let old: Value =
        serde_json::from_str(&html[lo..hi]).map_err(|e| format!("ledger-data: {e}"))?;
    let mut rows: Vec<Map<String, Value>> = old["rows"]
        .as_array()
        .ok_or("ledger-data has no rows")?
        .iter()
        .filter_map(|r| r.as_object().cloned())
        .collect();
    let catalog_rows = rows.len();
    // Every run rebuilds each note from the note the row first came with, so a
    // second run over its own output adds nothing.
    for r in rows.iter_mut() {
        let base = match r.get("base_note").and_then(Value::as_str) {
            Some(b) => b.to_string(),
            None => s(r, "note").to_string(),
        };
        r.insert("base_note".into(), json!(base));
        r.insert("note".into(), json!(base));
        r.remove("same_as");
    }
    // The old project's shared folder and the old account's artifacts are gone for
    // this account; their copies live on fix-queue.
    const OLD: [(&str, &str); 2] = [
        (
            "https://claude.ai/artifact/YYYZhcv7YjfW5txZL12Ki1",
            "resume/workspace-audit-20261003/brutex-workspace-audit.html.md",
        ),
        (
            "https://claude.ai/artifact/K6QyihZvvh1d4UXQnApVZT",
            "resume/audit-20261003/brutex-audit-ledger.html",
        ),
    ];
    for r in rows.iter_mut() {
        let link = s(r, "link").to_string();
        let new = if let Some(rest) = link.strip_prefix("/mnt/project-files/") {
            Some(format!(
                "{}resume/project-files-20261004/{rest}",
                a.link_base
            ))
        } else {
            OLD.iter()
                .find(|(u, _)| link == *u)
                .map(|(_, p)| format!("{}{p}", a.link_base))
        };
        if let Some(l) = new {
            r.insert("link".into(), json!(l));
        }
    }

    // 2. growing finding files
    let mut index: HashMap<String, usize> = rows
        .iter()
        .enumerate()
        .map(|(i, r)| (s(r, "id").to_string(), i))
        .collect();
    let mut added = 0usize;
    if let Some(dir) = &a.zero_dir {
        let rel = dir
            .to_string_lossy()
            .split("resume/")
            .nth(1)
            .map(|t| format!("resume/{t}"))
            .unwrap_or_default();
        for f in zero_sources(dir, &a.link_base, rel.trim_end_matches('/'))? {
            if let Some(&i) = index.get(&f.id) {
                // fill a blank severity or title from the source, never overwrite one
                let r = &mut rows[i];
                if s(r, "sev").is_empty() && !f.sev.is_empty() {
                    r.insert("sev".into(), json!(f.sev));
                }
                if s(r, "title").is_empty() {
                    r.insert("title".into(), json!(f.title));
                }
                if s(r, "area").is_empty() && !f.area.is_empty() {
                    r.insert("area".into(), json!(f.area));
                }
                continue;
            }
            index.insert(f.id.clone(), rows.len());
            added += 1;
            let mut r = Map::new();
            for (k, v) in [
                ("id", json!(f.id)),
                ("group", json!(f.group)),
                ("sev", json!(f.sev)),
                ("area", json!(f.area)),
                ("title", json!(f.title)),
                ("link", json!(f.link)),
                ("was", json!("new")),
                ("owner", json!(ZERO_OWNER)),
                ("state", json!("found")),
                ("note", json!("Added from its source file")),
                ("base_note", json!("Added from its source file")),
            ] {
                r.insert(k.into(), v);
            }
            rows.push(r);
        }
    }

    // 3. previous snapshot
    let mut prev: HashMap<String, String> = HashMap::new();
    let mut prev_as_of = old["as_of"].as_str().unwrap_or("").to_string();
    if let Some(p) = &a.snapshot {
        let snap: Value = serde_json::from_str(&read(p)?).map_err(|e| format!("snapshot: {e}"))?;
        prev_as_of = snap["as_of"].as_str().unwrap_or("").to_string();
        if let Some(m) = snap["states"].as_object() {
            for (k, v) in m {
                prev.insert(k.clone(), v.as_str().unwrap_or("").to_string());
            }
        }
    }
    // Every run starts each row from `base_state`, the state it had before this builder
    // first saw it (the old board's last snapshot), never from a state an earlier run
    // derived, so dropping a same-as pair or a correction cannot leave its effect behind.
    // The snapshot itself only supplies `prev`.
    for r in rows.iter_mut() {
        let key = format!("{}|{}", s(r, "group"), s(r, "id"));
        let base = match r.get("base_state").and_then(Value::as_str) {
            Some(b) => b.to_string(),
            None => prev
                .get(&key)
                .cloned()
                .unwrap_or_else(|| s(r, "state").to_string()),
        };
        r.insert("base_state".into(), json!(base));
        r.insert("state".into(), json!(base));
    }

    // 4. status files
    let resolve = |raw: &str, index: &HashMap<String, usize>| -> Vec<usize> {
        let mut hits = Vec::new();
        for id in status_ids(raw) {
            for cand in [id.clone(), format!("conc:{id}"), format!("num:{id}")] {
                if let Some(&i) = index.get(&cand) {
                    if !hits.contains(&i) {
                        hits.push(i);
                    }
                    break;
                }
            }
        }
        hits
    };
    let mut commits: Vec<Vec<String>> = vec![Vec::new(); rows.len()];
    let mut backed: Vec<Option<u8>> = vec![None; rows.len()];
    let mut unmatched: Vec<String> = Vec::new();
    let mut applied = 0usize;
    for p in &a.status {
        let name = p
            .file_name()
            .map(|n| {
                n.to_string_lossy()
                    .replace(".tsv.md", "")
                    .replace(".tsv", "")
            })
            .unwrap_or_default();
        for line in tsv(p)? {
            let (Some(raw), Some(st)) = (line.first(), line.get(1)) else {
                continue;
            };
            if !STATES.contains(&st.as_str()) {
                unmatched.push(format!("{name}: {raw} (unknown state {st})"));
                continue;
            }
            let commit = line.get(2).cloned().unwrap_or_default();
            let note = line.get(3).cloned().unwrap_or_default();
            let hits = resolve(raw, &index);
            let mut hits = hits;
            if hits.is_empty() {
                // A finding named only by a status file is still a finding: give it a row
                // rather than dropping it. A bare decision number is not a finding id.
                for id in status_ids(raw) {
                    let bare_decision =
                        id.starts_with("D-") && id_like(&id) && id.matches('-').count() == 1;
                    if bare_decision || id.is_empty() || index.contains_key(&id) {
                        continue;
                    }
                    let owner = match name.as_str() {
                        "attack-audit" => "Finish the attack audit",
                        "sweep" => "Whole-codebase sweep",
                        "lane1" => "Lane 1 fixes",
                        _ => ZERO_OWNER,
                    };
                    let title = if note.trim_matches(['|', ' ']).is_empty() {
                        format!("Named only in the {name} status file; no description there")
                    } else {
                        clean_title(note.trim_matches(['|', ' ']))
                    };
                    let mut r = Map::new();
                    for (k, v) in [
                        ("id", json!(id)),
                        ("group", json!("Named only in a status file")),
                        ("sev", json!("")),
                        ("area", json!("")),
                        ("title", json!(title)),
                        ("link", json!("")),
                        ("was", json!("new")),
                        ("owner", json!(owner)),
                        ("state", json!("found")),
                        ("note", json!("")),
                        ("base_note", json!("")),
                    ] {
                        r.insert(k.into(), v);
                    }
                    index.insert(id, rows.len());
                    hits.push(rows.len());
                    rows.push(r);
                    commits.push(Vec::new());
                    backed.push(None);
                    added += 1;
                }
                if hits.is_empty() {
                    unmatched.push(format!("{name}: {raw}"));
                    continue;
                }
            }
            let line_commits = hex_tokens(&commit);
            for i in hits {
                applied += 1;
                let r = &mut rows[i];
                // A later line never lowers a state that an earlier line backed with a commit.
                if !line_commits.is_empty() {
                    backed[i] = backed[i].max(Some(rank(st)));
                }
                let keep = backed[i].is_some_and(|b| rank(st) < b);
                if !keep {
                    r.insert("state".into(), json!(st));
                }
                let mut n = format!("Status from {name}");
                let c = commit.trim_matches('-').trim();
                if !c.is_empty() {
                    n += &format!(", {c}");
                }
                if !note.trim_matches(['|', ' ']).is_empty() {
                    n += &format!(": {}", note.trim_matches(['|', ' ']));
                }
                if keep {
                    n += &format!(
                        " (not applied: an earlier status line backed '{}' with a commit)",
                        s(r, "state")
                    );
                    if let Some(old) = r.get("note").and_then(Value::as_str) {
                        n = format!("{old}; {n}");
                    }
                }
                r.insert("note".into(), json!(n));
                for c in &line_commits {
                    if !commits[i].contains(c) {
                        commits[i].push(c.clone());
                    }
                }
            }
        }
    }

    // 4b. corrections: hand-checked states, each with its evidence in the note. They
    // win over every status line and are never re-promoted by commit-message evidence.
    let mut pinned = vec![false; rows.len()];
    let mut later: Vec<Vec<String>> = vec![Vec::new(); rows.len()];
    if let Some(p) = &a.corrections {
        for line in tsv(p)? {
            let (Some(id), Some(st)) = (line.first(), line.get(1)) else {
                continue;
            };
            if !STATES.contains(&st.as_str()) {
                return Err(format!("corrections: {id} has unknown state {st}"));
            }
            let Some(&i) = index.get(id) else {
                return Err(format!("corrections: {id} is not a ledger row"));
            };
            let r = &mut rows[i];
            r.insert("state".into(), json!(st));
            let note = line.get(3).cloned().unwrap_or_default();
            r.insert("note".into(), json!(format!("Checked by hand: {note}")));
            if let Some(title) = line.get(4).filter(|t| !t.is_empty()) {
                r.insert("title".into(), json!(title));
            }
            if let Some(link) = line.get(5).filter(|t| !t.is_empty()) {
                r.insert("link".into(), json!(link));
            }
            let fixed = hex_tokens(line.get(2).map_or("", String::as_str));
            later[i] = commits[i]
                .iter()
                .filter(|c| {
                    !fixed
                        .iter()
                        .any(|f| f.starts_with(c.as_str()) || c.starts_with(f.as_str()))
                })
                .cloned()
                .collect();
            commits[i] = fixed;
            pinned[i] = true;
        }
    }
    for c in &a.not_a_fix {
        for list in commits.iter_mut().chain(later.iter_mut()) {
            list.retain(|x| !c.starts_with(x.as_str()) && !x.starts_with(c.as_str()));
        }
    }

    // 5. git evidence against the PR head
    let mut evidence = 0usize;
    let mut demoted = 0usize;
    let mut lifted = 0usize;
    let mut head_short = String::new();
    if let Some(repo) = &a.repo {
        let out = Command::new("git")
            .arg("-C")
            .arg(repo)
            .args(["rev-parse", "--short=7", &a.pr_ref])
            .output()
            .map_err(|e| e.to_string())?;
        if !out.status.success() {
            return Err(format!("git cannot resolve {}", a.pr_ref));
        }
        head_short = String::from_utf8_lossy(&out.stdout).trim().to_string();
        let files = unique_files(repo, &a.pr_ref);
        for r in rows.iter_mut() {
            if s(r, "area").is_empty() {
                let found = area_in([s(r, "title"), s(r, "base_note")]);
                r.insert("area".into(), json!(found));
            }
            if let Some(full) = place(s(r, "area"), &files) {
                r.insert("area".into(), json!(full));
            }
        }
        let mut cache = HashMap::new();
        let mut then = HashMap::new();
        for (i, r) in rows.iter_mut().enumerate() {
            let Some(at) = a.checked_at.as_deref().filter(|_| pinned[i]) else {
                continue;
            };
            let newer = later[i].iter().find(|c| {
                on_head(repo, &a.pr_ref, c, &mut cache) == OnHead::Yes
                    && on_head(repo, at, c, &mut then) == OnHead::No
            });
            if let Some(c) = newer {
                pinned[i] = false;
                lifted += 1;
                let n = format!(
                    "{}; correction lifted: status commit {} reached PR #74 after it was checked",
                    s(r, "note"),
                    &c[..7.min(c.len())]
                );
                r.insert("note".into(), json!(n));
                commits[i].push(c.clone());
            }
        }
        for (i, r) in rows.iter_mut().enumerate() {
            let mut flags: Vec<Value> = Vec::new();
            let st = s(r, "state").to_string();
            let verdicts: Vec<(String, OnHead)> = commits[i]
                .iter()
                .map(|c| (c.clone(), on_head(repo, &a.pr_ref, c, &mut cache)))
                .collect();
            if let Some((c, _)) = verdicts.iter().find(|(_, v)| *v == OnHead::Yes) {
                if !DONE.contains(&st.as_str()) && !pinned[i] {
                    evidence += 1;
                    r.insert("state".into(), json!("pushed"));
                }
                let n = format!(
                    "{}; commit {} verified on PR #74 head {head_short}",
                    s(r, "note"),
                    &c[..7]
                );
                r.insert("note".into(), json!(n));
            } else if st == "pushed"
                && !verdicts.is_empty()
                && verdicts.iter().all(|(_, v)| *v == OnHead::No)
            {
                demoted += 1;
                r.insert("state".into(), json!("branch"));
                flags.push(json!("claim-mismatch"));
                let n = format!(
                    "{}; claimed on PR #74 but commit {} is not on head {head_short}",
                    s(r, "note"),
                    &verdicts[0].0[..7]
                );
                r.insert("note".into(), json!(n));
            }
            r.insert("flags".into(), Value::Array(flags));
        }
    }

    // 5b. a commit on the PR (base..head) whose message names the finding as fixed
    let mut by_message = 0usize;
    if let Some(repo) = &a.repo {
        let out = Command::new("git")
            .arg("-C")
            .arg(repo)
            .args([
                "log",
                "--format=head commit %h%x00%B%x1e",
                &format!("{}..{}", a.base_ref, a.pr_ref),
            ])
            .output()
            .map_err(|e| e.to_string())?;
        if !out.status.success() {
            return Err(format!("git log {}..{} failed", a.base_ref, a.pr_ref));
        }
        let mut log = String::from_utf8_lossy(&out.stdout).to_string();
        // decision entries the PR added; an entry whose title says the finding is only
        // stated, blocked or not changed is not a fix
        let diff = Command::new("git")
            .arg("-C")
            .arg(repo)
            .args(["diff", &a.base_ref, &a.pr_ref, "--", "docs/05-decisions.md"])
            .output()
            .map_err(|e| e.to_string())?;
        let diff = String::from_utf8_lossy(&diff.stdout).to_string();
        log.push_str(&decision_chunks(&diff));
        let mentions = message_mentions(&log);
        for (i, r) in rows.iter_mut().enumerate() {
            let st = s(r, "state").to_string();
            if pinned[i] || DONE.contains(&st.as_str()) || r.get("same_as").is_some() {
                continue;
            }
            let id = s(r, "id");
            let bare = id
                .strip_prefix("conc:")
                .or_else(|| id.strip_prefix("num:"))
                .unwrap_or(id);
            if bare.len() < 5 || bare.split('-').count() < 2 {
                continue;
            }
            if let Some((sha, line)) = mentions.get(bare) {
                by_message += 1;
                let n = format!(
                    "{}; {sha} names it as fixed: \"{}\"",
                    s(r, "note"),
                    clean_title(line)
                );
                r.insert("state".into(), json!("pushed"));
                r.insert("note".into(), json!(n));
            }
        }
    }

    // 5c. the same defect filed twice: both rows take the further state, and the
    // duplicate is marked so the page counts the pair once.
    let mut same = 0usize;
    if let Some(p) = &a.same_as {
        for line in tsv(p)? {
            let (Some(dup), Some(primary)) = (line.first(), line.get(1)) else {
                continue;
            };
            let (Some(&d), Some(&q)) = (index.get(dup), index.get(primary)) else {
                return Err(format!("same-as: {dup} or {primary} is not a ledger row"));
            };
            let (sd, sq) = (
                s(&rows[d], "state").to_string(),
                s(&rows[q], "state").to_string(),
            );
            let (best, from) = if rank(&sd) > rank(&sq) {
                (sd, d)
            } else {
                (sq, q)
            };
            let carried = s(&rows[from], "note").to_string();
            for i in [d, q] {
                if s(&rows[i], "state") != best {
                    rows[i].insert("state".into(), json!(best));
                    rows[i].insert(
                        "note".into(),
                        json!(format!(
                            "Same defect as {}: {carried}",
                            if i == d { primary } else { dup }
                        )),
                    );
                }
            }
            rows[d].insert("same_as".into(), json!(primary));
            let ev = line.get(2).cloned().unwrap_or_default();
            let n = format!(
                "{} [counted once with {primary}: {ev}]",
                s(&rows[d], "note")
            );
            rows[d].insert("note".into(), json!(n));
            same += 1;
        }
    }

    // 5d. the helpers' verification tables: a dated, file-and-line verdict per finding
    let mut verdicts_applied = 0usize;
    let mut found_verdicts: Vec<(usize, u32, String, Verdict, String)> = Vec::new();
    if let Some(dir) = &a.zero_dir {
        let mut files: Vec<PathBuf> = files_in(dir).into_iter().filter(|f| f.is_file()).collect();
        for d in files_in(dir).into_iter().filter(|d| d.is_dir()) {
            files.extend(files_in(&d).into_iter().filter(|f| f.is_file()));
        }
        for f in files
            .iter()
            .filter(|f| f.extension().is_some_and(|e| e == "md"))
        {
            let name = f.strip_prefix(dir).unwrap_or(f).display().to_string();
            for (id, v, ev) in verdict_rows(&read(f)?) {
                let Some(i) = [id.clone(), format!("conc:{id}"), format!("num:{id}")]
                    .iter()
                    .find_map(|c| index.get(c).copied())
                else {
                    continue;
                };
                found_verdicts.push((i, pass_of(&name), name.clone(), v, ev));
            }
        }
    }
    // Two passes can check one finding at different heads. The highest pass decides the
    // state and the flag; every pass is still quoted in the note, oldest first.
    found_verdicts.sort_by(|x, y| (x.0, x.1, &x.2).cmp(&(y.0, y.1, &y.2)));
    for (k, (i, _, name, v, ev)) in found_verdicts.iter().enumerate() {
        let (i, v) = (*i, *v);
        verdicts_applied += 1;
        let r = &mut rows[i];
        let said = match v {
            Verdict::Fixed => "FIXED",
            Verdict::Partial => "PARTIAL",
            Verdict::NotFixed => "NOT FIXED",
        };
        let mut n = format!("{}; helper check {name}: {said} ({ev})", s(r, "note"));
        let latest = found_verdicts.get(k + 1).is_none_or(|next| next.0 != i);
        if latest {
            let st = s(r, "state").to_string();
            if !pinned[i] {
                match v {
                    Verdict::Fixed if rank(&st) < rank("branch") => {
                        r.insert("state".into(), json!("branch"));
                    }
                    Verdict::Partial if st == "found" => {
                        r.insert("state".into(), json!("partial"));
                    }
                    _ => {}
                }
            }
            if v == Verdict::NotFixed && DONE.contains(&st.as_str()) && st != "doc" {
                n += " [the helper's latest check contradicts the on-PR state]";
                let mut flags = r
                    .get("flags")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                flags.push(json!("audit-disagrees"));
                r.insert("flags".into(), Value::Array(flags));
            }
        }
        r.insert("note".into(), json!(n));
    }

    // 6. CI
    let mut checks: Vec<Value> = Vec::new();
    let mut ci_green = false;
    if let Some(p) = &a.checks {
        for c in tsv(p)? {
            if c.len() >= 5 {
                if c[0].starts_with("ci-ok") && c[1] == "pass" {
                    ci_green = true;
                }
                checks.push(json!(c[..5]));
            }
        }
    }

    // flags and prev
    let mut owner_of: HashMap<String, String> = HashMap::new();
    for r in &rows {
        if !s(r, "owner").is_empty() {
            owner_of
                .entry(s(r, "group").to_string())
                .or_insert_with(|| s(r, "owner").to_string());
        }
    }
    for r in rows.iter_mut() {
        if ci_green && s(r, "state") == "pushed" {
            r.insert("state".into(), json!("green"));
        }
        let key = format!("{}|{}", s(r, "group"), s(r, "id"));
        let st = s(r, "state").to_string();
        let done = DONE.contains(&st.as_str());
        if done && s(r, "owner").is_empty() {
            // nothing
        } else if !done && s(r, "owner").is_empty() {
            if let Some(o) = owner_of.get(s(r, "group")) {
                r.insert("owner".into(), json!(o));
            }
        }
        let mut flags: Vec<Value> = r
            .get("flags")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        match prev.get(&key) {
            None => flags.push(json!("new")),
            Some(p) => {
                r.insert("prev".into(), json!(p));
                if *p == st && !done {
                    flags.push(json!("stuck"));
                }
            }
        }
        if !done && s(r, "owner").is_empty() {
            flags.push(json!("no-owner"));
        }
        r.insert("flags".into(), Value::Array(flags));
    }

    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut order: Vec<String> = Vec::new();
    for r in rows.iter().filter(|r| !r.contains_key("same_as")) {
        let g = s(r, "group").to_string();
        if !counts.contains_key(&g) {
            order.push(g.clone());
        }
        *counts.entry(g).or_default() += 1;
    }
    let mut counts_o = Map::new();
    for g in &order {
        counts_o.insert(g.clone(), json!(counts[g]));
    }
    let ledger = json!({
        "as_of": a.as_of,
        "prev_as_of": prev_as_of,
        "pr_head": head_short,
        "ci_green": ci_green,
        "counts": counts_o,
        "unmatched_status": unmatched,
        "rows": rows,
    });

    let tsv_rows = |p: &Option<PathBuf>| -> Result<Vec<Value>, String> {
        Ok(match p {
            Some(p) => tsv(p)?.into_iter().map(|r| json!(r)).collect(),
            None => Vec::new(),
        })
    };
    let board = json!({
        "as_of": a.as_of,
        "pr": tsv_rows(&a.pr)?,
        "checks": checks,
        "streams": tsv_rows(&a.streams)?,
    });

    let safe = |v: &Value| serde_json::to_string(v).map(|t| t.replace("</", "<\\/"));
    let mut page = String::with_capacity(html.len() + 4096);
    page.push_str(&html[..lo]);
    page.push_str(&safe(&ledger).map_err(|e| e.to_string())?);
    page.push_str(&html[hi..]);
    if let Some((blo, bhi)) = script_span(&page, "board-data") {
        page = format!(
            "{}{}{}",
            &page[..blo],
            safe(&board).map_err(|e| e.to_string())?,
            &page[bhi..]
        );
    }
    fs::write(&a.out, page).map_err(|e| format!("{}: {e}", a.out.display()))?;

    if let Some(p) = &a.snapshot_out {
        let mut states = Map::new();
        for r in ledger["rows"].as_array().into_iter().flatten() {
            states.insert(
                format!(
                    "{}|{}",
                    r["group"].as_str().unwrap_or(""),
                    r["id"].as_str().unwrap_or("")
                ),
                r["state"].clone(),
            );
        }
        let snap = json!({"as_of": a.as_of, "prev_as_of": prev_as_of, "states": states});
        fs::write(
            p,
            serde_json::to_string_pretty(&snap).map_err(|e| e.to_string())? + "\n",
        )
        .map_err(|e| e.to_string())?;
    }

    let mut by_state: BTreeMap<&str, usize> = BTreeMap::new();
    for r in ledger["rows"].as_array().into_iter().flatten() {
        *by_state
            .entry(r["state"].as_str().unwrap_or("?"))
            .or_default() += 1;
    }
    eprintln!(
        "fixboard: {} rows ({catalog_rows} catalog + {added} new from sources); {applied} status lines applied, {} unmatched; \
         {evidence} promoted by status commits on head, {by_message} by head commit messages, {demoted} demoted, {lifted} corrections lifted, {same} same-as pairs, {verdicts_applied} helper verdicts; head {head_short}; ci green: {ci_green}; states {by_state:?}",
        ledger["rows"].as_array().map_or(0, Vec::len),
        unmatched.len()
    );
    for u in &unmatched {
        eprintln!("  unmatched: {u}");
    }
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("fixboard: {e}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_ids_expand_suffix_lists() {
        assert_eq!(
            status_ids("W2-cli14-1/2/3"),
            ["W2-cli14-1", "W2-cli14-2", "W2-cli14-3"]
        );
        assert_eq!(
            status_ids("D-1631 (W2-cli1-2/-3)"),
            ["D-1631", "W2-cli1-2", "W2-cli1-3"]
        );
        assert_eq!(
            status_ids("ET-bars-candles-store-1 / -8"),
            ["ET-bars-candles-store-1", "ET-bars-candles-store-8"]
        );
    }

    #[test]
    fn headings_read_each_source_shape() {
        let t = "## engine-1 (medium): a halt\n### sel-1 — medium — append wedge\n## cli2-1: medium. short write\n## What I checked\n";
        let h = headings(t, id_like);
        assert_eq!(
            h.iter().map(|x| x.0.as_str()).collect::<Vec<_>>(),
            ["engine-1", "sel-1", "cli2-1"]
        );
        assert_eq!(h[0].1, "medium");
        assert_eq!(h[1].2, "append wedge");
        assert_eq!(h[2].2, "short write");
        let p = headings("### P1-03-1 low `Expect` re-arms\n", |id| {
            id.starts_with('P') && id_like(id)
        });
        assert_eq!(
            (p[0].1.as_str(), p[0].2.as_str()),
            ("low", "Expect re-arms")
        );
    }

    #[test]
    fn area_comes_from_the_findings_own_section() {
        let t = "## a-1 (low): x\nsee `crates/api/src/server.rs:12`\n## a-2 (low): y\nno path\n- **CE-9** bad cli/src/lib.rs:4\n";
        assert_eq!(area_for(t, "a-1"), "api/src/server.rs:12");
        assert_eq!(area_for(t, "a-2"), "");
        assert_eq!(area_for(t, "CE-9"), "cli/src/lib.rs:4");
        assert_eq!(area_for(t, "a-3"), "");
        let b = "## b-1 (low): x\nin autopilot.rs:2642 and lib.rs\n";
        assert_eq!(area_for(b, "b-1"), "autopilot.rs:2642");
        let files = HashMap::from([(
            "autopilot.rs".to_string(),
            "api/src/autopilot.rs".to_string(),
        )]);
        assert_eq!(
            place("autopilot.rs:2642", &files).as_deref(),
            Some("api/src/autopilot.rs:2642")
        );
        assert_eq!(place("lib.rs", &files), None);
        assert_eq!(
            area_in(["gate 1 in .github/workflows/ci.yml:40"]),
            ".github/workflows/ci.yml:40"
        );
        assert_eq!(area_in(["CLAUDE.md §10 and docs/07-plan.md"]), "CLAUDE.md");
        assert_eq!(area_in(["docs/x.md then sweep.rs:9"]), "sweep.rs:9");
        assert_eq!(
            area_in(["shape in population_observations_v1, sweep_evidence"]),
            ""
        );
        assert_eq!(place("cli/src/lib.rs:4", &files), None);
    }

    #[test]
    fn severity_is_a_whole_word() {
        assert_eq!(severity("it follows the rule (high)"), "high");
        assert_eq!(severity("a below-the-bar row (medium)"), "medium");
        assert_eq!(severity("allow flow highlight information"), "");
        assert_eq!(severity("CE-36..CE-39: config (all low; re-read)"), "low");
    }

    #[test]
    fn severity_comes_from_the_section_and_the_range_heading() {
        let t = "### CE-23: torn file\n- Severity: high (wedges the rung)\n### CE-36..CE-39: empty values (all low)\n- **CE-36** an empty BRUTEX_LOGS writes into crates/api/src/server.rs:18380\n";
        assert_eq!(section_severity(t, "CE-23"), "high");
        assert_eq!(bullets(t, "CE-")[0].1, "low");
        assert_eq!(area_for(t, "CE-36"), "api/src/server.rs:18380");
    }

    #[test]
    fn a_wrapped_status_line_is_joined() {
        let dir = std::env::temp_dir().join(format!("fixboard-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let f = dir.join("a.tsv");
        fs::write(
            &f,
            "id\tstate\ncli-14 (W2-cli11-0/-1\nW2-cli12-3/-4)\tfound\nX-1\tbranch\n",
        )
        .unwrap();
        let rows = tsv(&f).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0][0], "cli-14 (W2-cli11-0/-1 W2-cli12-3/-4)");
        assert_eq!(
            status_ids(&rows[0][0]),
            [
                "cli-14",
                "W2-cli11-0",
                "W2-cli11-1",
                "W2-cli12-3",
                "W2-cli12-4"
            ]
        );
        fs::write(&f, "id\tstate\nA-1 (B-2\tfound\n").unwrap();
        assert!(tsv(&f).is_err());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn commit_messages_count_only_fix_lines() {
        let log = "abc1234\0Fix the ledger (GAP13-16, which D-1420 left)\nGAP11-0 is still open\n\u{1e}def5678\0KNOWN W2-cli13-5, re-reported by hunt-cli-b\nW2-cli9-9 not fixed here\n\u{1e}";
        let m = message_mentions(log);
        assert_eq!(m.get("GAP13-16").map(|x| x.0.as_str()), Some("abc1234"));
        assert!(!m.contains_key("GAP11-0"));
        assert_eq!(m.get("W2-cli13-5").map(|x| x.0.as_str()), Some("def5678"));
        assert!(!m.contains_key("W2-cli9-9"));
        assert!(!m.contains_key("D-1420"));
    }

    #[test]
    fn decision_entries_count_unless_their_title_is_not_a_fix() {
        let diff = "+### D-1563 — Markers appear whole — 2026\n+**What happened.** KNOWN GAP11-0 and W2-cli13-5, re-reported\n+### D-1631 — Lineage costs are stated, not fixed — 2026\n+W2-cli1-2 here\n context W9-x-1\n";
        let m = message_mentions(&decision_chunks(diff));
        assert_eq!(
            m.get("GAP11-0").map(|x| x.0.as_str()),
            Some("decision D-1563 on head")
        );
        assert!(m.contains_key("W2-cli13-5"));
        assert!(!m.contains_key("W2-cli1-2"));
        assert!(!m.contains_key("W9-x-1"));
    }

    #[test]
    fn verification_tables_are_read_by_their_state_column() {
        let t = "| CE | state | evidence |\n|---|---|---|\n| CE-1 | FIXED | 3f22aed8 folds it |\n| CE-9 | NOT FIXED | lib.rs:2371 |\n\n| id | claimed by | verdict | evidence |\n| pop1-4 | x | PARTIAL | half |\n| foo | x | maybe | y |\n";
        let v = verdict_rows(t);
        assert_eq!(v.len(), 3);
        assert_eq!((v[0].0.as_str(), &v[0].1), ("CE-1", &Verdict::Fixed));
        assert_eq!(v[1].1, Verdict::NotFixed);
        assert_eq!(pass_of("tests-docs-security.md"), 1);
        assert_eq!(pass_of("tests-docs-security-pass4.md"), 4);
        assert_eq!(pass_of("conc-pass4/ledgers-locks.md"), 4);
        assert_eq!(pass_of("crash-edge-pass12.md"), 12);
        assert_eq!((v[2].0.as_str(), &v[2].1), ("pop1-4", &Verdict::Partial));
    }

    #[test]
    fn states_rank_in_fix_order() {
        assert!(rank("found") < rank("fixing") && rank("fixing") == rank("partial"));
        assert!(rank("partial") < rank("branch") && rank("branch") < rank("doc"));
        assert!(rank("doc") < rank("pushed") && rank("pushed") < rank("green"));
    }

    #[test]
    fn only_tables_with_a_sev_column_count() {
        let t = "| id | sev | where | what |\n|---|---|---|---|\n| pst-1 | medium | a.rs | slow |\n\n| id | where | status |\n| gaps-7 | core | open |\n";
        let r = sev_tables(t);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].0, "pst-1");
    }

    #[test]
    fn hex_tokens_skip_words_and_short_numbers() {
        assert_eq!(
            hex_tokens("zero/cli-edges-2 ed5f3b38 (not merged)"),
            ["ed5f3b38"]
        );
        assert_eq!(hex_tokens("D-1644 deadbeef 1234567"), ["deadbeef"]);
    }
}

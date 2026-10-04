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
    as_of: String,
    link_base: String,
}

fn parse_args() -> Result<Args, String> {
    let mut a = Args {
        pr_ref: "origin/final/all-fixes".into(),
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
}

fn severity(s: &str) -> String {
    let l = s.to_ascii_lowercase();
    // the first severity word wins; "(plausible, design call)" carries none
    let mut best: Option<(usize, &str)> = None;
    for w in ["high", "medium", "low", "info"] {
        if let Some(i) = l.find(w) {
            if best.is_none_or(|(j, _)| i < j) {
                best = Some((i, w));
            }
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
    for line in text.lines() {
        let Some(r) = line.trim_start().strip_prefix("- **") else {
            continue;
        };
        let Some(end) = r.find("**") else { continue };
        let id = &r[..end];
        if id.starts_with(prefix) && id_like(id) {
            let sev = severity(&r[end..].chars().take(40).collect::<String>());
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

fn files_in(dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = fs::read_dir(dir)
        .map(|r| r.filter_map(|e| e.ok().map(|e| e.path())).collect())
        .unwrap_or_default();
    v.sort();
    v
}

fn zero_sources(dir: &Path, link_base: &str, rel: &str) -> Result<Vec<Found>, String> {
    let mut out = Vec::new();
    let link = |f: &str| format!("{link_base}{rel}/{f}");
    let push =
        |out: &mut Vec<Found>, group, (id, sev, title): (String, String, String), l: String| {
            out.push(Found {
                id,
                group,
                sev,
                title,
                link: l,
            })
        };
    // concurrency helper: summary file, then every pass directory
    let mut conc = vec![dir.join("concurrency.md")];
    for p in [
        "conc-pass1",
        "conc-pass2",
        "conc-pass3",
        "conc-pass4",
        "conc-pass5",
    ] {
        conc.extend(
            files_in(&dir.join(p))
                .into_iter()
                .filter(|f| f.extension().is_some_and(|e| e == "md")),
        );
    }
    for f in conc.iter().filter(|f| f.exists()) {
        let rel_f = f.strip_prefix(dir).unwrap_or(f).display().to_string();
        for h in headings(&read(f)?, |id| {
            id_like(id) && id.chars().next().is_some_and(|c| c.is_ascii_lowercase())
        }) {
            push(
                &mut out,
                "Zero-rounds: concurrency",
                (format!("conc:{}", h.0), h.1, h.2),
                link(&rel_f),
            );
        }
    }
    let ce = dir.join("crash-edge.md");
    if ce.exists() {
        let t = read(&ce)?;
        for h in headings(&t, |id| id.starts_with("CE-") && id_like(id))
            .into_iter()
            .chain(bullets(&t, "CE-"))
        {
            push(
                &mut out,
                "Zero-rounds: crashes and edge inputs",
                h,
                link("crash-edge.md"),
            );
        }
    }
    let td = dir.join("tests-docs-security.md");
    if td.exists() {
        let accept = |id: &str| id.starts_with('P') && id_like(id) && id.split('-').count() == 3;
        for h in headings(&read(&td)?, accept) {
            push(
                &mut out,
                "Zero-rounds: tests, docs, security",
                h,
                link("tests-docs-security.md"),
            );
        }
    }
    let nc = dir.join("numeric-complexity.md");
    if nc.exists() {
        for (id, sev, area, title) in sev_tables(&read(&nc)?) {
            let _ = area;
            push(
                &mut out,
                "Zero-rounds: numbers and complexity",
                (format!("num:{id}"), sev, title),
                link("numeric-complexity.md"),
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
        for h in headings(&read(&f)?, |id| {
            id.len() >= 2 && id.starts_with('F') && id[1..].chars().all(|c| c.is_ascii_digit())
        }) {
            push(
                &mut out,
                "Zero-findings loop: round 1",
                (format!("Z1-{stem}-{}", h.0), h.1, h.2),
                link(&format!("r1-slices/{name}")),
            );
        }
    }
    Ok(out)
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

fn tsv(p: &Path) -> Result<Vec<Vec<String>>, String> {
    let text = read(p)?;
    Ok(text
        .lines()
        .skip(1)
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.split('\t').map(|c| c.trim().to_string()).collect())
        .collect())
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
    // The old project's shared folder is gone; its copy lives on fix-queue.
    for r in rows.iter_mut() {
        if let Some(rest) = s(r, "link").strip_prefix("/mnt/project-files/") {
            let l = format!("{}resume/project-files-20261004/{rest}", a.link_base);
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
                continue;
            }
            index.insert(f.id.clone(), rows.len());
            added += 1;
            let mut r = Map::new();
            for (k, v) in [
                ("id", json!(f.id)),
                ("group", json!(f.group)),
                ("sev", json!(f.sev)),
                ("area", json!("")),
                ("title", json!(f.title)),
                ("link", json!(f.link)),
                ("was", json!("new")),
                ("owner", json!(ZERO_OWNER)),
                ("state", json!("found")),
                ("note", json!("Added from its source file at this refresh")),
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
    for r in rows.iter_mut() {
        let key = format!("{}|{}", s(r, "group"), s(r, "id"));
        if let Some(st) = prev.get(&key) {
            r.insert("state".into(), json!(st));
        }
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
                    ] {
                        r.insert(k.into(), v);
                    }
                    index.insert(id, rows.len());
                    hits.push(rows.len());
                    rows.push(r);
                    commits.push(Vec::new());
                    added += 1;
                }
                if hits.is_empty() {
                    unmatched.push(format!("{name}: {raw}"));
                    continue;
                }
            }
            for i in hits {
                applied += 1;
                let r = &mut rows[i];
                r.insert("state".into(), json!(st));
                let mut n = format!("Status from {name}");
                let c = commit.trim_matches('-').trim();
                if !c.is_empty() {
                    n += &format!(", {c}");
                }
                if !note.trim_matches(['|', ' ']).is_empty() {
                    n += &format!(": {}", note.trim_matches(['|', ' ']));
                }
                r.insert("note".into(), json!(n));
                commits[i] = hex_tokens(&format!("{commit} {note}"));
            }
        }
    }

    // 5. git evidence against the PR head
    let mut evidence = 0usize;
    let mut demoted = 0usize;
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
        let mut cache = HashMap::new();
        for (i, r) in rows.iter_mut().enumerate() {
            let mut flags: Vec<Value> = Vec::new();
            let st = s(r, "state").to_string();
            let verdicts: Vec<(String, OnHead)> = commits[i]
                .iter()
                .map(|c| (c.clone(), on_head(repo, &a.pr_ref, c, &mut cache)))
                .collect();
            if let Some((c, _)) = verdicts.iter().find(|(_, v)| *v == OnHead::Yes) {
                if !DONE.contains(&st.as_str()) {
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
    for r in &rows {
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
         {evidence} promoted by git evidence, {demoted} demoted; head {head_short}; ci green: {ci_green}; states {by_state:?}",
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

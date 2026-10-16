//! Reads the JSON the GitHub CLI prints and answers the questions the
//! auto-merge and main re-check workflows ask of it (RO-10, D-2343).
//!
//! Those workflows used to hand `gh --jq` a query program written inline, the
//! approval rule among them: a second language in a tracked file outside
//! `web/`, which CLAUDE.md section 2 forbids. The rules live here now, in Rust,
//! built by rustc from the checkout of `main` (never of the pull request), and
//! pinned by the tests below, which gate 0 runs. A bare field path is read
//! here too (`field KEY`), not by `--jq`: a path is still a jq expression,
//! evaluated by the jq engine inside gh, and gate 0 refuses every `--jq`
//! operand (D-2320).
//!
//! Every reader fails closed: a document that does not parse, or lacks a field
//! a rule needs, is an error on stderr and exit status 1, never an empty
//! answer, so `if ! x=$(gh ... | gh-json ...)` stops the job.
//!
//! `gh api --paginate` prints one JSON document per page with nothing between
//! them, so every reader takes a STREAM of documents.

#![forbid(unsafe_code)]

use std::io::Read as _;
use std::process::ExitCode;

#[derive(Clone, Debug, PartialEq)]
enum Json {
    Null,
    Bool(bool),
    Num(String),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

impl Json {
    fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Obj(fields) => fields.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    fn at(&self, path: &[&str]) -> Option<&Json> {
        path.iter().try_fold(self, |v, k| v.get(k))
    }

    fn str_at(&self, path: &[&str]) -> Result<&str, String> {
        match self.at(path) {
            Some(Json::Str(s)) => Ok(s),
            other => Err(format!("`.{}` is not a string: {other:?}", path.join("."))),
        }
    }

    fn bool_at(&self, key: &str) -> Result<bool, String> {
        match self.get(key) {
            Some(Json::Bool(b)) => Ok(*b),
            other => Err(format!("`.{key}` is not a boolean: {other:?}")),
        }
    }

    fn items(&self) -> Result<&[Json], String> {
        match self {
            Json::Arr(items) => Ok(items),
            other => Err(format!("expected an array, read {other:?}")),
        }
    }
}

struct Parser<'a> {
    s: &'a [u8],
    i: usize,
}

impl Parser<'_> {
    fn ws(&mut self) {
        while self.s.get(self.i).is_some_and(u8::is_ascii_whitespace) {
            self.i += 1;
        }
    }

    fn eat(&mut self, lit: &str) -> Result<(), String> {
        if self.s[self.i..].starts_with(lit.as_bytes()) {
            self.i += lit.len();
            Ok(())
        } else {
            Err(format!("expected `{lit}` at byte {}", self.i))
        }
    }

    fn value(&mut self) -> Result<Json, String> {
        self.ws();
        match self.s.get(self.i) {
            Some(b'n') => self.eat("null").map(|()| Json::Null),
            Some(b't') => self.eat("true").map(|()| Json::Bool(true)),
            Some(b'f') => self.eat("false").map(|()| Json::Bool(false)),
            Some(b'"') => self.string().map(Json::Str),
            Some(b'[') => {
                self.i += 1;
                let mut items = Vec::new();
                self.ws();
                if self.s.get(self.i) == Some(&b']') {
                    self.i += 1;
                    return Ok(Json::Arr(items));
                }
                loop {
                    items.push(self.value()?);
                    self.ws();
                    match self.s.get(self.i) {
                        Some(b',') => self.i += 1,
                        Some(b']') => {
                            self.i += 1;
                            return Ok(Json::Arr(items));
                        }
                        _ => return Err(format!("expected `,` or `]` at byte {}", self.i)),
                    }
                }
            }
            Some(b'{') => {
                self.i += 1;
                let mut fields = Vec::new();
                self.ws();
                if self.s.get(self.i) == Some(&b'}') {
                    self.i += 1;
                    return Ok(Json::Obj(fields));
                }
                loop {
                    self.ws();
                    let key = self.string()?;
                    self.ws();
                    self.eat(":")?;
                    fields.push((key, self.value()?));
                    self.ws();
                    match self.s.get(self.i) {
                        Some(b',') => self.i += 1,
                        Some(b'}') => {
                            self.i += 1;
                            return Ok(Json::Obj(fields));
                        }
                        _ => return Err(format!("expected `,` or `}}` at byte {}", self.i)),
                    }
                }
            }
            Some(c) if *c == b'-' || c.is_ascii_digit() => {
                let start = self.i;
                while self
                    .s
                    .get(self.i)
                    .is_some_and(|c| c.is_ascii_digit() || b"+-.eE".contains(c))
                {
                    self.i += 1;
                }
                let n = std::str::from_utf8(&self.s[start..self.i]).map_err(|e| e.to_string())?;
                n.parse::<f64>()
                    .map_err(|e| format!("bad number {n:?}: {e}"))?;
                Ok(Json::Num(n.to_owned()))
            }
            other => Err(format!("unexpected {other:?} at byte {}", self.i)),
        }
    }

    fn hex4(&mut self) -> Result<u32, String> {
        let h = self
            .s
            .get(self.i..self.i + 4)
            .ok_or("a truncated \\u escape")?;
        self.i += 4;
        let h = std::str::from_utf8(h).map_err(|e| e.to_string())?;
        u32::from_str_radix(h, 16).map_err(|e| format!("bad \\u escape {h:?}: {e}"))
    }

    fn string(&mut self) -> Result<String, String> {
        self.eat("\"")?;
        let mut out = String::new();
        loop {
            let start = self.i;
            while self
                .s
                .get(self.i)
                .is_some_and(|c| *c != b'"' && *c != b'\\' && *c >= 0x20)
            {
                self.i += 1;
            }
            out.push_str(std::str::from_utf8(&self.s[start..self.i]).map_err(|e| e.to_string())?);
            match self.s.get(self.i) {
                Some(b'"') => {
                    self.i += 1;
                    return Ok(out);
                }
                Some(b'\\') => {
                    let e = *self.s.get(self.i + 1).ok_or("a truncated escape")?;
                    self.i += 2;
                    match e {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{8}'),
                        b'f' => out.push('\u{c}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => {
                            let hi = self.hex4()?;
                            let code = if (0xD800..0xDC00).contains(&hi) {
                                self.eat("\\u")?;
                                let lo = self.hex4()?;
                                if !(0xDC00..0xE000).contains(&lo) {
                                    return Err("an unpaired surrogate".to_owned());
                                }
                                0x10000 + ((hi - 0xD800) << 10) + (lo - 0xDC00)
                            } else {
                                hi
                            };
                            out.push(char::from_u32(code).ok_or("an unpaired surrogate")?);
                        }
                        other => return Err(format!("bad escape \\{}", other as char)),
                    }
                }
                _ => return Err(format!("an unterminated string at byte {start}")),
            }
        }
    }
}

/// Every JSON document in `text`, in order. Empty input is zero documents.
fn documents(text: &str) -> Result<Vec<Json>, String> {
    let mut p = Parser {
        s: text.as_bytes(),
        i: 0,
    };
    let mut out = Vec::new();
    loop {
        p.ws();
        if p.i == p.s.len() {
            return Ok(out);
        }
        out.push(p.value()?);
    }
}

/// Exactly one document.
fn one(text: &str) -> Result<Json, String> {
    let mut docs = documents(text)?;
    match docs.len() {
        1 => Ok(docs.remove(0)),
        n => Err(format!("expected one JSON document, read {n}")),
    }
}

/// A field for a tab-separated line, escaped as jq's `@tsv` escapes it.
fn tsv(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('\t', "\\t")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

/// `gh pr view --json autoMergeRequest`: is auto-merge armed?
fn armed(text: &str) -> Result<Vec<String>, String> {
    let doc = one(text)?;
    let yes = match doc.get("autoMergeRequest") {
        Some(Json::Null) => false,
        Some(Json::Obj(_)) => true,
        other => {
            return Err(format!(
                "`.autoMergeRequest` is neither null nor an object: {other:?}"
            ));
        }
    };
    Ok(vec![if yes { "yes" } else { "no" }.to_owned()])
}

/// `commits/{sha}/pulls`: the open pull request whose HEAD is `sha`, not one
/// that merely contains it (hunt-ci-9, D-1604). None is an empty answer.
fn pr_for_head(text: &str, sha: &str) -> Result<Vec<String>, String> {
    for doc in documents(text)? {
        for pr in doc.items()? {
            if pr.str_at(&["state"])? == "open" && pr.str_at(&["head", "sha"])? == sha {
                return match pr.get("number") {
                    Some(Json::Num(n)) => Ok(vec![n.clone()]),
                    other => Err(format!("`.number` is not a number: {other:?}")),
                };
            }
        }
    }
    Ok(Vec::new())
}

/// `gh pr view --json state,isDraft,isCrossRepository,headRefOid`.
fn pr_fields(text: &str) -> Result<Vec<String>, String> {
    let doc = one(text)?;
    Ok(vec![format!(
        "{}\t{}\t{}\t{}",
        tsv(doc.str_at(&["state"])?),
        doc.bool_at("isDraft")?,
        doc.bool_at("isCrossRepository")?,
        tsv(doc.str_at(&["headRefOid"])?)
    )])
}

/// `commits/{sha}/check-runs --paginate`: name, status, conclusion (`none`
/// while a run has none).
fn check_runs(text: &str) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    for doc in documents(text)? {
        let runs = doc
            .get("check_runs")
            .ok_or("a page without `.check_runs`")?;
        for run in runs.items()? {
            let conclusion = match run.get("conclusion") {
                None | Some(Json::Null) => "none",
                Some(Json::Str(s)) => s,
                other => return Err(format!("`.conclusion` is not a string: {other:?}")),
            };
            out.push(format!(
                "{}\t{}\t{}",
                tsv(run.str_at(&["name"])?),
                tsv(run.str_at(&["status"])?),
                tsv(conclusion)
            ));
        }
    }
    Ok(out)
}

/// `pulls/{n}/files --paginate`: every path a pull request touches, the old
/// name of a rename included, because moving a gate file away changes it.
fn filenames(text: &str) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    for doc in documents(text)? {
        for f in doc.items()? {
            out.push(f.str_at(&["filename"])?.to_owned());
            match f.get("previous_filename") {
                None | Some(Json::Null) => {}
                Some(Json::Str(p)) => out.push(p.clone()),
                other => return Err(format!("`.previous_filename` is not a string: {other:?}")),
            }
        }
    }
    Ok(out)
}

/// `pulls/{n}/reviews --paginate`: the reviewers whose LATEST verdict on
/// `head` is an approval (D-1604). A comment or a pending review is not a
/// verdict and is skipped; a review of an older head approves code that is no
/// longer the pull request and is skipped; a later `CHANGES_REQUESTED` or
/// `DISMISSED` replaces an earlier approval. Sorted by login.
fn approvers(text: &str, head: &str) -> Result<Vec<String>, String> {
    let mut latest: std::collections::BTreeMap<String, (String, String)> =
        std::collections::BTreeMap::new();
    for doc in documents(text)? {
        for r in doc.items()? {
            if r.str_at(&["commit_id"])? != head {
                continue;
            }
            let state = r.str_at(&["state"])?;
            if matches!(state, "COMMENTED" | "PENDING") {
                continue;
            }
            let login = r.str_at(&["user", "login"])?;
            let at = r.str_at(&["submitted_at"])?;
            let newer = latest.get(login).is_none_or(|(t, _)| at >= t.as_str());
            if newer {
                latest.insert(login.to_owned(), (at.to_owned(), state.to_owned()));
            }
        }
    }
    Ok(latest
        .into_iter()
        .filter(|(_, (_, s))| s == "APPROVED")
        .map(|(login, _)| login)
        .collect())
}

/// `actions/workflows/ci.yml/runs`: how many runs a push, a dispatch or the
/// schedule started (D-1605).
fn ci_run_count(text: &str) -> Result<Vec<String>, String> {
    let mut n = 0usize;
    for doc in documents(text)? {
        let runs = doc
            .get("workflow_runs")
            .ok_or("a page without `.workflow_runs`")?;
        for run in runs.items()? {
            if matches!(
                run.str_at(&["event"])?,
                "push" | "workflow_dispatch" | "schedule"
            ) {
                n += 1;
            }
        }
    }
    Ok(vec![n.to_string()])
}

/// One top-level field of one document, as the line a workflow compares:
/// `compare` gives `.behind_by`, `commits/main` gives `.sha`, `pr view --json
/// state` gives `.state` (D-2320). A string comes back as its text and a
/// number as its digits; anything else, a missing field, or a string a shell
/// line could not carry whole is refused, never an empty answer.
fn field(text: &str, key: &str) -> Result<Vec<String>, String> {
    let doc = one(text)?;
    match doc.get(key) {
        Some(Json::Str(s)) if !s.is_empty() && !s.chars().any(char::is_control) => {
            Ok(vec![s.clone()])
        }
        Some(Json::Num(n)) => Ok(vec![n.clone()]),
        other => Err(format!(
            "`.{key}` is not a one-line string or a number: {other:?}"
        )),
    }
}

fn answer(args: &[String], text: &str) -> Result<Vec<String>, String> {
    let arg = |k: usize| {
        args.get(k)
            .filter(|a| !a.is_empty())
            .ok_or_else(|| format!("`{}` needs an argument", args[0]))
    };
    match args.first().map(String::as_str) {
        Some("armed") => armed(text),
        Some("pr-for-head") => pr_for_head(text, arg(1)?),
        Some("pr-fields") => pr_fields(text),
        Some("check-runs") => check_runs(text),
        Some("filenames") => filenames(text),
        Some("approvers") => approvers(text, arg(1)?),
        Some("ci-run-count") => ci_run_count(text),
        Some("field") => field(text, arg(1)?),
        _ => Err(
            "usage: gh_json <armed|pr-for-head SHA|pr-fields|check-runs|filenames|approvers SHA|ci-run-count|field KEY> < json"
                .to_owned(),
        ),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut text = String::new();
    if let Err(e) = std::io::stdin().read_to_string(&mut text) {
        eprintln!("gh_json: standard input: {e}");
        return ExitCode::from(1);
    }
    match answer(&args, &text) {
        Ok(lines) => {
            for l in lines {
                println!("{l}");
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("gh_json: {e}");
            ExitCode::from(1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(args: &[&str], text: &str) -> Result<Vec<String>, String> {
        let args: Vec<String> = args.iter().map(|a| (*a).to_owned()).collect();
        answer(&args, text)
    }

    #[test]
    fn the_parser_reads_every_json_shape_and_a_page_stream() {
        let docs =
            documents("{\"a\":[1,-2.5e3,true,false,null,\"x\\n\\u00e9\\ud83d\\ude00\"]} [] {}")
                .unwrap();
        assert_eq!(docs.len(), 3);
        assert_eq!(
            docs[0].get("a").unwrap().items().unwrap()[5],
            Json::Str("x\n\u{e9}\u{1F600}".to_owned())
        );
        assert_eq!(documents(" \n").unwrap(), Vec::new());
        for bad in [
            "{",
            "[1,]",
            "{\"a\" 1}",
            "\"\\ud800\"",
            "tru",
            "\"a",
            "01x",
            "[1 2]",
        ] {
            assert!(documents(bad).is_err(), "parsed: {bad}");
        }
    }

    #[test]
    fn armed_reads_null_as_no_and_refuses_a_missing_field() {
        assert_eq!(
            run(&["armed"], "{\"autoMergeRequest\":null}").unwrap(),
            ["no"]
        );
        assert_eq!(
            run(
                &["armed"],
                "{\"autoMergeRequest\":{\"mergeMethod\":\"SQUASH\"}}"
            )
            .unwrap(),
            ["yes"]
        );
        assert!(run(&["armed"], "{}").is_err());
        assert!(run(&["armed"], "").is_err());
    }

    #[test]
    fn only_the_open_pull_request_whose_head_is_the_sha_is_chosen() {
        let page = "[{\"number\":7,\"state\":\"open\",\"head\":{\"sha\":\"b\"}},\
                    {\"number\":8,\"state\":\"closed\",\"head\":{\"sha\":\"a\"}},\
                    {\"number\":9,\"state\":\"open\",\"head\":{\"sha\":\"a\"}}]";
        assert_eq!(run(&["pr-for-head", "a"], page).unwrap(), ["9"]);
        assert_eq!(
            run(&["pr-for-head", "c"], page).unwrap(),
            Vec::<String>::new()
        );
        assert!(run(&["pr-for-head", ""], page).is_err());
        assert!(run(&["pr-for-head", "a"], "[{\"state\":\"open\"}]").is_err());
    }

    #[test]
    fn pr_fields_are_one_tab_separated_line() {
        let doc = "{\"state\":\"OPEN\",\"isDraft\":false,\"isCrossRepository\":true,\"headRefOid\":\"abc\"}";
        assert_eq!(
            run(&["pr-fields"], doc).unwrap(),
            ["OPEN\tfalse\ttrue\tabc"]
        );
        assert!(run(&["pr-fields"], "{\"state\":\"OPEN\"}").is_err());
    }

    #[test]
    fn check_runs_read_every_page_and_name_a_missing_conclusion_none() {
        let pages = "{\"check_runs\":[{\"name\":\"a\",\"status\":\"completed\",\"conclusion\":\"failure\"}]}\
                     {\"check_runs\":[{\"name\":\"b\\tc\",\"status\":\"queued\",\"conclusion\":null}]}";
        assert_eq!(
            run(&["check-runs"], pages).unwrap(),
            ["a\tcompleted\tfailure", "b\\tc\tqueued\tnone"]
        );
        assert!(run(&["check-runs"], "{}").is_err());
    }

    #[test]
    fn filenames_include_the_old_name_of_a_rename() {
        let pages = "[{\"filename\":\"a\"},{\"filename\":\"b\",\"previous_filename\":\".github/x.yml\"}][{\"filename\":\"c\",\"previous_filename\":null}]";
        assert_eq!(
            run(&["filenames"], pages).unwrap(),
            ["a", "b", ".github/x.yml", "c"]
        );
    }

    #[test]
    fn the_approval_rule_takes_each_reviewers_latest_verdict_on_this_head() {
        let r = |login: &str, state: &str, commit: &str, at: &str| {
            format!(
                "{{\"user\":{{\"login\":\"{login}\"}},\"state\":\"{state}\",\"commit_id\":\"{commit}\",\"submitted_at\":\"{at}\"}}"
            )
        };
        let page1 = format!(
            "[{},{},{},{}]",
            r("ann", "APPROVED", "h", "2026-10-01T00:00:00Z"),
            r("bob", "APPROVED", "h", "2026-10-01T00:00:00Z"),
            r("bob", "CHANGES_REQUESTED", "h", "2026-10-02T00:00:00Z"),
            r("cy", "APPROVED", "old", "2026-10-03T00:00:00Z"),
        );
        let page2 = format!(
            "[{},{},{}]",
            r("ann", "COMMENTED", "h", "2026-10-04T00:00:00Z"),
            r("dee", "CHANGES_REQUESTED", "h", "2026-10-01T00:00:00Z"),
            r("dee", "APPROVED", "h", "2026-10-05T00:00:00Z"),
        );
        let text = format!("{page1}{page2}");
        // ann: approval stands (a later comment is no verdict); bob: overturned;
        // cy: approved an older head; dee: a later approval replaces a refusal.
        assert_eq!(run(&["approvers", "h"], &text).unwrap(), ["ann", "dee"]);
        assert_eq!(
            run(&["approvers", "x"], &text).unwrap(),
            Vec::<String>::new()
        );
        let dismissed = format!(
            "[{},{}]",
            r("ann", "APPROVED", "h", "2026-10-01T00:00:00Z"),
            r("ann", "DISMISSED", "h", "2026-10-02T00:00:00Z")
        );
        assert_eq!(
            run(&["approvers", "h"], &dismissed).unwrap(),
            Vec::<String>::new()
        );
        assert!(run(&["approvers", "h"], "[{\"commit_id\":\"h\"}]").is_err());
        assert!(run(&["approvers"], "[]").is_err());
    }

    #[test]
    fn field_reads_one_string_or_number_and_refuses_everything_else() {
        assert_eq!(
            run(&["field", "sha"], "{\"sha\":\"abc\"}").unwrap(),
            ["abc"]
        );
        assert_eq!(
            run(&["field", "behind_by"], "{\"behind_by\":0}").unwrap(),
            ["0"]
        );
        assert_eq!(
            run(&["field", "state"], "{\"state\":\"MERGED\",\"x\":1}").unwrap(),
            ["MERGED"]
        );
        for (args, doc) in [
            (&["field", "sha"][..], "{}"),
            (&["field", "sha"], "{\"sha\":null}"),
            (&["field", "sha"], "{\"sha\":\"\"}"),
            (&["field", "sha"], "{\"sha\":\"a\\nb\"}"),
            (&["field", "sha"], "{\"sha\":true}"),
            (&["field", "sha"], "{\"sha\":[\"a\"]}"),
            (&["field", "sha"], "{\"sha\":\"a\"}{\"sha\":\"b\"}"),
            (&["field", "sha"], "[]"),
            (&["field"], "{\"sha\":\"a\"}"),
            (&["field", ""], "{\"\":\"a\"}"),
        ] {
            assert!(run(args, doc).is_err(), "answered: {args:?} {doc}");
        }
    }
    #[test]
    fn ci_run_count_counts_push_dispatch_and_schedule_only() {
        let page = "{\"workflow_runs\":[{\"event\":\"push\"},{\"event\":\"pull_request\"},{\"event\":\"workflow_dispatch\"},{\"event\":\"schedule\"}]}";
        assert_eq!(run(&["ci-run-count"], page).unwrap(), ["3"]);
        assert!(run(&["ci-run-count"], "{}").is_err());
        assert!(run(&["nonsense"], "{}").is_err());
    }
}

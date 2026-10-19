//! Every confirmed finding and every audit row has an owner (R3-14; NEW-37,
//! RA-325).
//!
//! # What was missing
//!
//! The audit's confirmed findings and the rules audit's rows are the board's
//! list of what is wrong. The fix plan named most of them, the round-3 plan
//! more, and nothing compared the lists: on 12 October 28 confirmed findings
//! were in no package's list, so for those nobody could say "fixed by package
//! X and test Y", "still open" or "will not be fixed, because", and the board
//! could not prove that every corner was covered.
//!
//! # The rule
//!
//! An id is OWNED when one of these names it (every one that does is kept, the
//! first is shown first):
//!
//! 1. a package of the fix plan (`work_packages[].finding_ids`);
//! 2. a package or an addition of the round-3 plan (`packages[].answers`,
//!    `additions[].answers`);
//! 3. the round-3 plan's own line for a leftover (`unowned_findings`);
//! 4. for an audit row, the section of the rules audit it is listed under: a
//!    package (`## WP-81: ...`), a critic resolution or gap, a NEW package
//!    (owned by the round-3 package whose answers name it), or `## Rows that
//!    need nothing`;
//! 5. [`VERDICTS`] below: a package, `covered by <package> and <test>`, or
//!    `will not fix, because ...`.
//!
//! RED is an id none of those names, a [`VERDICTS`] line for an id that is in
//! no input or that a plan already owns (a stale line hides the plan's own
//! answer), and an input that does not have the shape read here: a renamed
//! key would otherwise read as "no findings" and pass.
//!
//! The round-3 plan's `audit_rows_only_listed_for_rejudgement` rows are shown
//! with the word `rejudge` beside their owner: they are owned, and their
//! verdict is to be taken again once that owner merges.
//!
//! # Running it
//!
//! Std only, built by rustc (`rustc --edition=2024 --test` proves it first);
//! it reads files and starts no program. The board lives outside the
//! repository, so it is named on the command line:
//!
//! ```text
//! findings_owner <audit-result.json> <fix-plan.json> <plan-round3.json> \
//!     <rules-audit.md> [<result file>]
//! ```
//!
//! It prints one `id<TAB>owner<TAB>source` line per id and a summary, writes
//! the same to the result file when one is named, and exits 1 on any red, 2
//! when an input cannot be read.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::process::ExitCode;

/// The owner lines this check itself carries, for ids no plan names. Each is
/// a package, `covered by <package> and <test>`, or `will not fix, because`.
/// Written on 13 October 2026 from each finding's own `fix_hint` and the
/// board as it stood; a plan that later names the id makes the line stale,
/// and the check says so until it is removed.
pub const VERDICTS: &[(&str, &str)] = &[
    (
        "CMP-13",
        "IA-4 (options per-row parity): a whole option file is refused for one bad row or a \
         padded header; WP-12a's per-row refusal covers capital-market files only. Then a \
         --retry-refused store step on the operator's yes",
    ),
    (
        "CMP-18",
        "WP-13d (Zerodha series identity) and WP-13a (refuse non-positive cash prices), as the \
         finding's own hint names; the reused-token cause stays UNVERIFIED until WP-13d reads it",
    ),
    (
        "CMP-20",
        "will not fix in the repository, because the stale lines are in \
         state/design/ingest-automation.md, a document outside it: the plan's owner (the main \
         session) corrects the two figures",
    ),
];

/// A JSON value, as far as this check reads one.
#[derive(Debug, Clone, PartialEq)]
pub enum Json {
    /// `null`, `true` or `false`, and every number: never read here.
    Other,
    /// A string.
    Str(String),
    /// An array.
    Arr(Vec<Json>),
    /// An object, its keys in document order.
    Obj(Vec<(String, Json)>),
}

impl Json {
    /// The value under `key`, when this is an object holding it.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Obj(fields) => fields.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    /// The string, when this is one.
    #[must_use]
    pub fn str(&self) -> Option<&str> {
        match self {
            Json::Str(s) => Some(s),
            _ => None,
        }
    }

    /// The items, when this is an array.
    #[must_use]
    pub fn arr(&self) -> Option<&[Json]> {
        match self {
            Json::Arr(items) => Some(items),
            _ => None,
        }
    }
}

/// Parses one JSON document.
///
/// # Errors
///
/// The byte offset and what was expected there.
pub fn parse(text: &str) -> Result<Json, String> {
    let mut p = Parser {
        b: text.as_bytes(),
        i: 0,
        depth: 0,
    };
    let value = p.value()?;
    p.ws();
    if p.i != p.b.len() {
        return Err(format!("byte {}: text after the document", p.i));
    }
    Ok(value)
}

struct Parser<'a> {
    b: &'a [u8],
    i: usize,
    depth: usize,
}

impl Parser<'_> {
    fn ws(&mut self) {
        while matches!(self.b.get(self.i), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.i += 1;
        }
    }

    fn eat(&mut self, want: u8) -> Result<(), String> {
        self.ws();
        if self.b.get(self.i) == Some(&want) {
            self.i += 1;
            Ok(())
        } else {
            Err(format!("byte {}: expected `{}`", self.i, want as char))
        }
    }

    fn value(&mut self) -> Result<Json, String> {
        self.ws();
        self.depth += 1;
        if self.depth > 256 {
            return Err(format!("byte {}: nested past 256 levels", self.i));
        }
        let out = match self.b.get(self.i) {
            Some(b'{') => self.object(),
            Some(b'[') => self.array(),
            Some(b'"') => self.string().map(Json::Str),
            Some(b't') => self.word("true"),
            Some(b'f') => self.word("false"),
            Some(b'n') => self.word("null"),
            Some(b'-' | b'0'..=b'9') => {
                while matches!(
                    self.b.get(self.i),
                    Some(b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9')
                ) {
                    self.i += 1;
                }
                Ok(Json::Other)
            }
            _ => Err(format!("byte {}: expected a value", self.i)),
        };
        self.depth -= 1;
        out
    }

    fn word(&mut self, w: &str) -> Result<Json, String> {
        if self.b.get(self.i..self.i + w.len()) == Some(w.as_bytes()) {
            self.i += w.len();
            Ok(Json::Other)
        } else {
            Err(format!("byte {}: expected `{w}`", self.i))
        }
    }

    fn object(&mut self) -> Result<Json, String> {
        self.eat(b'{')?;
        let mut fields = Vec::new();
        self.ws();
        if self.b.get(self.i) == Some(&b'}') {
            self.i += 1;
            return Ok(Json::Obj(fields));
        }
        loop {
            self.ws();
            let key = self.string()?;
            self.eat(b':')?;
            let value = self.value()?;
            fields.push((key, value));
            self.ws();
            match self.b.get(self.i) {
                Some(b',') => self.i += 1,
                Some(b'}') => {
                    self.i += 1;
                    return Ok(Json::Obj(fields));
                }
                _ => return Err(format!("byte {}: expected `,` or `}}`", self.i)),
            }
        }
    }

    fn array(&mut self) -> Result<Json, String> {
        self.eat(b'[')?;
        let mut items = Vec::new();
        self.ws();
        if self.b.get(self.i) == Some(&b']') {
            self.i += 1;
            return Ok(Json::Arr(items));
        }
        loop {
            items.push(self.value()?);
            self.ws();
            match self.b.get(self.i) {
                Some(b',') => self.i += 1,
                Some(b']') => {
                    self.i += 1;
                    return Ok(Json::Arr(items));
                }
                _ => return Err(format!("byte {}: expected `,` or `]`", self.i)),
            }
        }
    }

    fn hex4(&mut self) -> Result<u32, String> {
        let digits = self
            .b
            .get(self.i..self.i + 4)
            .and_then(|d| std::str::from_utf8(d).ok())
            .and_then(|d| u32::from_str_radix(d, 16).ok())
            .ok_or_else(|| format!("byte {}: expected four hex digits", self.i))?;
        self.i += 4;
        Ok(digits)
    }

    fn string(&mut self) -> Result<String, String> {
        if self.b.get(self.i) != Some(&b'"') {
            return Err(format!("byte {}: expected a string", self.i));
        }
        self.i += 1;
        let mut out: Vec<u8> = Vec::new();
        loop {
            match self.b.get(self.i) {
                None => return Err("the text ends inside a string".to_owned()),
                Some(b'"') => {
                    self.i += 1;
                    return String::from_utf8(out)
                        .map_err(|_| format!("byte {}: a string that is not UTF-8", self.i));
                }
                Some(b'\\') => {
                    let esc = self.b.get(self.i + 1).copied();
                    self.i += 2;
                    let ch = match esc {
                        Some(b'"') => '"',
                        Some(b'\\') => '\\',
                        Some(b'/') => '/',
                        Some(b'b') => '\u{8}',
                        Some(b'f') => '\u{c}',
                        Some(b'n') => '\n',
                        Some(b'r') => '\r',
                        Some(b't') => '\t',
                        Some(b'u') => {
                            let hi = self.hex4()?;
                            let code = if (0xD800..0xDC00).contains(&hi)
                                && self.b.get(self.i..self.i + 2) == Some(b"\\u")
                            {
                                self.i += 2;
                                let lo = self.hex4()?;
                                0x10000 + ((hi - 0xD800) << 10) + lo.wrapping_sub(0xDC00)
                            } else {
                                hi
                            };
                            char::from_u32(code).unwrap_or('\u{fffd}')
                        }
                        _ => return Err(format!("byte {}: an unknown escape", self.i)),
                    };
                    let mut buf = [0u8; 4];
                    out.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                }
                Some(&c) => {
                    out.push(c);
                    self.i += 1;
                }
            }
        }
    }
}

/// One owner of one id, and where it was read.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Owner {
    /// The owner line: a package, a verdict, a covering line.
    pub who: String,
    /// Which input said so.
    pub source: &'static str,
}

/// What the inputs hold, read.
#[derive(Debug, Default)]
pub struct Board {
    /// Every confirmed finding id.
    pub confirmed: BTreeSet<String>,
    /// Every audit row id.
    pub rows: BTreeSet<String>,
    /// Every owner of every id.
    pub owners: BTreeMap<String, Vec<Owner>>,
    /// The rows the round-3 plan lists for rejudgement.
    pub rejudge: BTreeSet<String>,
}

impl Board {
    fn own(&mut self, id: &str, who: impl Into<String>, source: &'static str) {
        let owner = Owner {
            who: who.into(),
            source,
        };
        let list = self.owners.entry(id.to_owned()).or_default();
        if !list.contains(&owner) {
            list.push(owner);
        }
    }
}

fn need<'a>(doc: &'a Json, key: &str, file: &str) -> Result<&'a Json, String> {
    doc.get(key)
        .ok_or_else(|| format!("{file} has no `{key}`: it is not the shape this check reads"))
}

fn list<'a>(doc: &'a Json, key: &str, file: &str) -> Result<&'a [Json], String> {
    need(doc, key, file)?
        .arr()
        .ok_or_else(|| format!("{file}'s `{key}` is not a list"))
}

fn strings(doc: &Json, key: &str) -> Vec<String> {
    doc.get(key)
        .and_then(Json::arr)
        .map(|items| {
            items
                .iter()
                .filter_map(Json::str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

/// Whether `word` reads as a package id: `WP-81`, `IA-10a`, `R3-14`, `NEW-02`.
#[must_use]
pub fn is_package(word: &str) -> bool {
    let Some((head, tail)) = word.split_once('-') else {
        return false;
    };
    let digits = tail.trim_end_matches(|c: char| c.is_ascii_lowercase());
    matches!(head, "WP" | "IA" | "R3" | "NEW")
        && !digits.is_empty()
        && digits.bytes().all(|b| b.is_ascii_digit())
}

/// The lead a rules-audit section heading names, or `None` for a heading
/// that holds no owned rows (the how-to-read and summary sections).
#[must_use]
pub fn section_lead(heading: &str) -> Option<String> {
    let text = heading.trim_start_matches('#').trim();
    if text.starts_with("Rows that need nothing") {
        return Some("needs nothing (the rules audit's own section)".to_owned());
    }
    if let Some(rest) = text.strip_prefix("Critic resolution ") {
        let id = rest.split(':').next().unwrap_or("").trim();
        return (!id.is_empty()).then(|| format!("critic resolution {id}"));
    }
    if let Some(rest) = text.strip_prefix("Critic gap ") {
        let id = rest.split_whitespace().next().unwrap_or("");
        return (!id.is_empty()).then(|| format!("critic gap {id}"));
    }
    let first = text.split(':').next().unwrap_or("").trim();
    is_package(first).then(|| first.to_owned())
}

/// Reads the four inputs into a [`Board`].
///
/// # Errors
///
/// An input that does not parse or lacks a key this check reads.
pub fn read_board(audit: &str, fix_plan: &str, round3: &str, rules: &str) -> Result<Board, String> {
    let audit = parse(audit).map_err(|e| format!("audit-result: {e}"))?;
    let fix_plan = parse(fix_plan).map_err(|e| format!("fix-plan: {e}"))?;
    let round3 = parse(round3).map_err(|e| format!("round-3 plan: {e}"))?;
    let mut board = Board::default();

    for finding in list(&audit, "confirmed", "audit-result")? {
        let id = need(finding, "id", "a confirmed finding")?
            .str()
            .ok_or("a confirmed finding's id is not a string")?;
        board.confirmed.insert(id.to_owned());
    }
    if board.confirmed.is_empty() {
        return Err("audit-result holds no confirmed finding: a blind read, refused".to_owned());
    }

    for package in list(&fix_plan, "work_packages", "fix-plan")? {
        let name = need(package, "name", "a fix-plan package")?
            .str()
            .unwrap_or("")
            .split_whitespace()
            .next()
            .unwrap_or("")
            .to_owned();
        for id in strings(package, "finding_ids") {
            board.own(&id, name.clone(), "fix-plan");
        }
    }

    let mut new_owner: BTreeMap<String, String> = BTreeMap::new();
    for package in list(&round3, "packages", "round-3 plan")? {
        let name = need(package, "name", "a round-3 package")?
            .str()
            .unwrap_or("")
            .to_owned();
        for id in strings(package, "answers") {
            if id.starts_with("NEW-") {
                let _first = new_owner.entry(id.clone()).or_insert_with(|| name.clone());
            }
            board.own(&id, name.clone(), "round-3 answers");
        }
    }
    for addition in list(&round3, "additions", "round-3 plan")? {
        let name = need(addition, "package", "a round-3 addition")?
            .str()
            .unwrap_or("")
            .to_owned();
        for id in strings(addition, "answers") {
            board.own(&id, format!("{name} (addition)"), "round-3 answers");
        }
    }
    match need(&round3, "unowned_findings", "round-3 plan")? {
        Json::Obj(fields) => {
            for (id, line) in fields {
                board.own(id, line.str().unwrap_or(""), "round-3 unowned_findings");
            }
        }
        _ => return Err("round-3 plan's `unowned_findings` is not an object".to_owned()),
    }
    if let Some(rows) = round3
        .get("not_covered")
        .and_then(|n| n.get("audit_rows_only_listed_for_rejudgement"))
        .and_then(Json::arr)
    {
        for row in rows.iter().filter_map(Json::str) {
            let id = row.split_whitespace().next().unwrap_or("");
            board.rejudge.insert(id.to_owned());
        }
    }

    let mut lead: Option<String> = None;
    for line in rules.lines() {
        if line.starts_with("## ") {
            lead = section_lead(line);
            continue;
        }
        // A row is a table row or, in the sections that list rows that meet
        // the rule, a list item: `| RA-001 ...` or `- RA-106 ...`.
        let Some(cell) = line
            .strip_prefix("| RA-")
            .or_else(|| line.strip_prefix("- RA-"))
        else {
            continue;
        };
        let digits: String = cell.chars().take_while(char::is_ascii_digit).collect();
        if digits.len() != 3 {
            continue;
        }
        let id = format!("RA-{digits}");
        board.rows.insert(id.clone());
        if let Some(lead) = &lead {
            let who = match new_owner.get(lead) {
                Some(package) => format!("{package} (as {lead})"),
                None => lead.clone(),
            };
            board.own(&id, who, "rules-audit section");
        }
    }
    if board.rows.is_empty() {
        return Err("the rules audit holds no `| RA-` row: a blind read, refused".to_owned());
    }
    Ok(board)
}

/// One id's verdict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    /// The id.
    pub id: String,
    /// Its owners, the first shown first; empty is red.
    pub owners: Vec<Owner>,
    /// Listed by the round-3 plan for rejudgement.
    pub rejudge: bool,
}

/// Every id with its owners, and every red finding.
#[must_use]
pub fn judge(board: &Board, verdicts: &[(&str, &str)]) -> (Vec<Line>, Vec<String>) {
    let mut red = Vec::new();
    let mut owners = board.owners.clone();
    let known: BTreeSet<&String> = board.confirmed.iter().chain(&board.rows).collect();
    for (id, verdict) in verdicts {
        let id = (*id).to_owned();
        if !known.contains(&id) {
            red.push(format!(
                "VERDICTS names {id}, which no input holds: remove the line"
            ));
            continue;
        }
        if owners.get(&id).is_some_and(|o| !o.is_empty()) {
            red.push(format!(
                "VERDICTS names {id}, which a plan already owns: remove the line, the plan's \
                 answer stands"
            ));
            continue;
        }
        owners.entry(id).or_default().push(Owner {
            who: (*verdict).to_owned(),
            source: "findings_owner VERDICTS",
        });
    }
    let mut lines = Vec::new();
    for id in board.confirmed.iter().chain(&board.rows) {
        let mine = owners.get(id).cloned().unwrap_or_default();
        if mine.is_empty() {
            red.push(format!(
                "{id} has no owner: name a package, `covered by <package> and <test>` or `will \
                 not fix, because ...` in a plan or in VERDICTS"
            ));
        }
        lines.push(Line {
            id: id.clone(),
            owners: mine,
            rejudge: board.rejudge.contains(id),
        });
    }
    (lines, red)
}

/// The printed table: one line per id, then the summary.
#[must_use]
pub fn render(lines: &[Line], red: &[String]) -> String {
    let mut out = String::from("id\towner\tsource\n");
    for line in lines {
        let first = line.owners.first();
        let more = line.owners.len().saturating_sub(1);
        out.push_str(&format!(
            "{}\t{}{}{}\t{}\n",
            line.id,
            first.map_or("NO OWNER", |o| o.who.as_str()),
            if more > 0 {
                format!(" (+{more} more)")
            } else {
                String::new()
            },
            if line.rejudge { " [rejudge]" } else { "" },
            first.map_or("-", |o| o.source),
        ));
    }
    let findings = lines.iter().filter(|l| !l.id.starts_with("RA-")).count();
    out.push_str(&format!(
        "\n{} ids ({findings} confirmed findings, {} audit rows); {} red\n",
        lines.len(),
        lines.len() - findings,
        red.len()
    ));
    for line in red {
        out.push_str(&format!("RED {line}\n"));
    }
    out
}

#[allow(
    dead_code,
    reason = "the entry point when built by rustc; unused under --test"
)]
fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 4 {
        eprintln!(
            "usage: findings_owner <audit-result.json> <fix-plan.json> <plan-round3.json> \
             <rules-audit.md> [<result file>]"
        );
        return ExitCode::from(2);
    }
    let read = |p: &String| std::fs::read_to_string(p).map_err(|why| format!("{p}: {why}"));
    let board = match (|| {
        read_board(
            &read(&args[0])?,
            &read(&args[1])?,
            &read(&args[2])?,
            &read(&args[3])?,
        )
    })() {
        Ok(board) => board,
        Err(why) => {
            eprintln!("findings_owner refused: {why}");
            return ExitCode::from(2);
        }
    };
    let (lines, red) = judge(&board, VERDICTS);
    let text = render(&lines, &red);
    print!("{text}");
    if let Some(out) = args.get(4)
        && let Err(why) = std::fs::write(out, &text)
    {
        eprintln!("findings_owner: the result file {out} was not written: {why}");
        return ExitCode::from(2);
    }
    if red.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const AUDIT: &str = r#"{"counts":{},"confirmed":[
        {"id":"DB-1","severity":"high"},{"id":"DB-2"},{"id":"DB-3"},
        {"id":"DB-4"},{"id":"DB-5"},{"id":"CMP-9"}]}"#;
    const FIX_PLAN: &str = r#"{"work_packages":[
        {"name":"WP-10 one thing","finding_ids":["DB-1"]},
        {"name":"WP-11a another","finding_ids":["DB-2","DB-1"]}]}"#;
    const ROUND3: &str = r#"{"packages":[
        {"name":"R3-14","answers":["NEW-02","DB-3","RA-002"]}],
      "additions":[{"package":"WP-62","answers":["DB-4"]}],
      "unowned_findings":{"DB-5":"fixed (leftover check)"},
      "not_covered":{"audit_rows_only_listed_for_rejudgement":["RA-004"]}}"#;
    const RULES: &str = "\
# Rules audit
## NEW packages proposed
| NEW-02 | x | 3 rows: RA-001 |
## WP-81: ops safety
| RA-004 high. a row | x | WP-81 |
## NEW-02: Limits rows (NEW). A build check
| RA-001 medium. a row | x | NEW-02 |
## Critic resolution D-4891: Gate 14's cover line
| RA-003 medium. a row | x | x |
## How to read the sections
| RA-005 low. a row in no owned section | x | x |
## Rows that need nothing
| RA-006 low. a row | x | x |
- RA-008 measured, listed as an item
| RA-07 not an id | x | x |
";

    fn board() -> Board {
        read_board(AUDIT, FIX_PLAN, ROUND3, RULES).expect("the fixtures read")
    }

    fn owner_of<'a>(lines: &'a [Line], id: &str) -> Option<&'a Owner> {
        lines
            .iter()
            .find(|l| l.id == id)
            .and_then(|l| l.owners.first())
    }

    /// **EVERY CONFIRMED FINDING HAS AN OWNER** (R3-14, NEW-37): through a
    /// fix-plan package, a round-3 answer or addition, the round-3 plan's own
    /// leftover line, or a verdict here; and every audit row through those or
    /// its rules-audit section. An id nobody names is red, and so is a
    /// verdict line that is stale.
    #[test]
    fn every_confirmed_finding_has_an_owner() {
        let board = board();
        assert_eq!(board.confirmed.len(), 6);
        assert_eq!(
            board.rows.iter().cloned().collect::<Vec<_>>(),
            ["RA-001", "RA-003", "RA-004", "RA-005", "RA-006", "RA-008"],
            "six rows, one a list item; `RA-07` is not an id"
        );

        let (lines, red) = judge(&board, &[]);
        let who = |id| owner_of(&lines, id).map(|o| (o.who.as_str(), o.source));
        assert_eq!(who("DB-1"), Some(("WP-10", "fix-plan")));
        assert_eq!(
            lines
                .iter()
                .find(|l| l.id == "DB-1")
                .map(|l| l.owners.len()),
            Some(2),
            "both packages that name it are kept"
        );
        assert_eq!(who("DB-3"), Some(("R3-14", "round-3 answers")));
        assert_eq!(who("DB-4"), Some(("WP-62 (addition)", "round-3 answers")));
        assert_eq!(
            who("DB-5"),
            Some(("fixed (leftover check)", "round-3 unowned_findings"))
        );
        assert_eq!(
            who("RA-001"),
            Some(("R3-14 (as NEW-02)", "rules-audit section")),
            "a NEW section is owned by the round-3 package that answers it"
        );
        assert_eq!(who("RA-003").map(|w| w.0), Some("critic resolution D-4891"));
        assert_eq!(who("RA-004").map(|w| w.0), Some("WP-81"));
        assert!(lines.iter().any(|l| l.id == "RA-004" && l.rejudge));
        assert_eq!(
            who("RA-006").map(|w| w.0),
            Some("needs nothing (the rules audit's own section)")
        );
        assert_eq!(
            who("RA-008"),
            who("RA-006"),
            "a list item is owned by its section too"
        );
        assert_eq!(who("CMP-9"), None);
        assert_eq!(who("RA-005"), None);
        assert_eq!(red.len(), 2, "{red:?}");
        assert!(red.iter().any(|r| r.starts_with("CMP-9 has no owner")));
        assert!(red.iter().any(|r| r.starts_with("RA-005 has no owner")));

        let verdicts: &[(&str, &str)] = &[
            (
                "CMP-9",
                "will not fix, because the document is outside the repository",
            ),
            (
                "RA-005",
                "covered by R3-14 and every_route_has_a_limits_row_and_a_growth_bench",
            ),
        ];
        let (lines, red) = judge(&board, verdicts);
        assert!(red.is_empty(), "{red:?}");
        assert_eq!(
            owner_of(&lines, "CMP-9").map(|o| o.source),
            Some("findings_owner VERDICTS")
        );
        let text = render(&lines, &red);
        assert!(
            text.contains("RA-004\tWP-81 [rejudge]\trules-audit section\n"),
            "{text}"
        );
        assert!(
            text.contains("12 ids (6 confirmed findings, 6 audit rows); 0 red"),
            "{text}"
        );

        let stale: &[(&str, &str)] = &[("DB-1", "x"), ("DB-404", "y")];
        let (_, red) = judge(&board, stale);
        assert!(
            red.iter()
                .any(|r| r.contains("DB-1, which a plan already owns")),
            "{red:?}"
        );
        assert!(
            red.iter()
                .any(|r| r.contains("DB-404, which no input holds")),
            "{red:?}"
        );
    }

    /// A renamed key or an empty list is a refusal, never "no findings".
    #[test]
    fn a_board_of_another_shape_is_refused_not_read_as_empty() {
        assert!(read_board(r#"{"confirmd":[]}"#, FIX_PLAN, ROUND3, RULES).is_err());
        assert!(read_board(r#"{"confirmed":[]}"#, FIX_PLAN, ROUND3, RULES).is_err());
        assert!(read_board(AUDIT, r#"{"packages":[]}"#, ROUND3, RULES).is_err());
        assert!(read_board(AUDIT, FIX_PLAN, r#"{"packages":[],"additions":[]}"#, RULES).is_err());
        assert!(read_board(AUDIT, FIX_PLAN, ROUND3, "## WP-1: x\n").is_err());
        assert!(read_board("{", FIX_PLAN, ROUND3, RULES).is_err());
    }

    #[test]
    fn the_parser_reads_escapes_and_refuses_garbage() {
        let doc = parse(r#"{"a":"x\"y\\zé😀\n","b":[1,-2.5e3,true,null],"c":{}}"#).expect("valid");
        assert_eq!(
            doc.get("a").and_then(Json::str),
            Some("x\"y\\z\u{e9}\u{1F600}\n")
        );
        assert_eq!(doc.get("b").and_then(Json::arr).map(<[Json]>::len), Some(4));
        assert_eq!(doc.get("c"), Some(&Json::Obj(Vec::new())));
        for bad in ["", "{", "[1,]", r#"{"a" 1}"#, r#""\q""#, "[1] 2", "tru"] {
            assert!(parse(bad).is_err(), "{bad:?} is refused");
        }
        let deep = "[".repeat(300) + &"]".repeat(300);
        assert!(parse(&deep).is_err(), "nesting is bounded");
    }

    #[test]
    fn package_words_and_section_leads() {
        for yes in ["WP-81", "IA-10a", "R3-14", "NEW-02", "WP-12f"] {
            assert!(is_package(yes), "{yes}");
        }
        for no in ["WP-", "XX-1", "RA-001", "WP-1A", "Critic"] {
            assert!(!is_package(no), "{no}");
        }
        assert_eq!(
            section_lead("## WP-81: ops safety").as_deref(),
            Some("WP-81")
        );
        assert_eq!(section_lead("## How to read the sections"), None);
        assert_eq!(
            section_lead("## Critic gap IA-1-4 (round 7; no decision)").as_deref(),
            Some("critic gap IA-1-4")
        );
    }
}

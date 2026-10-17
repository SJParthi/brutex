//! Every decision number cited under `crates/` heads an entry of
//! `docs/05-decisions.md`, or is named pending in one of the ledger's own
//! "Cited, never written" notes.
//!
//! # Why this file exists
//!
//! On 14 October 2026 the code cited 32 four-digit decision numbers that headed
//! no entry and were named in no pending note (assurance findings law-3 and
//! law-4; their scan counted 34, adding D-0051, which D-0684 names, and the
//! five-digit D-06961). D-3350 alone was cited 117
//! times, and is quoted in the refusal the operator reads when a GDFL back-fill
//! is refused, with no ledger text to check that refusal against. Gate 27b
//! checks that a heading number is unique; nothing checked that a cited one
//! exists. `CLAUDE.md` §9 asks for a ledger entry for every locked choice, and
//! `docs/05-decisions.md` D-0102 for a test wherever a document states what the
//! code also knows. This is that test for the citation side.
//!
//! # The rule, and what each word means
//!
//! * A **citation** is `D-` and exactly four ASCII digits, with no ASCII
//!   letter, digit or `_` immediately before the `D` or after the digits.
//!   `SOURCED-2018` is not one (its `D` follows a letter), and neither is
//!   `D-06961` (five digits: `cli`'s `sweep_wiring_tests` pins that a
//!   five-digit "decision" is a number like any other). Gate 27b's heading
//!   pattern has four digits too.
//! * An **entry** is a line outside a fenced code block that starts with two
//!   to four `#`, one or more spaces and a citation: gate 27b's pattern
//!   `^#{2,4} +D-[0-9]{4}` with the citation's trailing boundary, which gate
//!   27b does not apply (`### D-00012` heads nothing here and D-0001 there).
//!   A level-five heading and a heading inside a fence head nothing. A fence
//!   opens on three or more `` ` `` or `~` and closes only on a run of the
//!   same character at least as long with nothing after it, so a `~~~` inside
//!   a block fenced by three backticks closes nothing.
//! * A **pending** number is one named in the leading bold span of a list item
//!   (`* **D-0051.** ...`) inside a section whose heading reads
//!   `Cited, never written`, up to the next heading line. That is D-0684's
//!   form. **The ledger is the one pending list: this file holds none**, so a
//!   number is resolved by writing its entry or by naming it, with where it
//!   resolves, in the ledger, never by an edit here.
//!
//! Walked: every `.rs`, `.toml` and `.md` file under `crates/`, in path order,
//! skipping a directory named `target` and any whose name starts with `.`. A
//! symbolic link under `crates/` is refused by name rather than followed or
//! skipped. A failure names each unresolved number once, at its first file and
//! line in that order.
//!
//! # What it does not check
//!
//! What a citation means: a comment can cite a number that heads an entry about
//! something else, and this passes. Citations in `docs/`, in `web/` and in
//! `.github/` are not read. `docs/06-limits.md` records both (AF-07).

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// The decision ledger, whose headings and pending notes are the claim under
/// test. `include_str!` makes an edit of the ledger rebuild this test.
const LEDGER: &str = include_str!("../../../docs/05-decisions.md");

/// The workspace root: `CARGO_MANIFEST_DIR` is `<root>/crates/core`.
fn workspace_root() -> PathBuf {
    let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    root.pop();
    root.pop();
    root
}

/// Whether `b` is an ASCII word character, the `\b` of the rule.
const fn is_word(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Every citation in `text`, with its 1-based line number, in text order.
fn citations(text: &str) -> Vec<(String, usize)> {
    let mut out = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let bytes = line.as_bytes();
        let mut at = 0;
        while let Some(found) = line.get(at..).and_then(|rest| rest.find("D-")) {
            let start = at + found;
            at = start + 2;
            if start > 0 && is_word(bytes[start - 1]) {
                continue;
            }
            let digits = bytes[start + 2..]
                .iter()
                .take_while(|b| b.is_ascii_digit())
                .count();
            if digits != 4 {
                continue;
            }
            if bytes.get(start + 6).copied().is_some_and(is_word) {
                continue;
            }
            // `D-` and four ASCII digits: six bytes on char boundaries.
            out.push((line[start..start + 6].to_owned(), index + 1));
        }
    }
    out
}

/// The number an entry heading heads, when `line` is one.
///
/// Gate 27b's pattern with the citation's trailing boundary: `### D-00012`
/// and `### D-0001x` head nothing here, where gate 27b's extraction (`its
/// first four digits only`) would count each as D-0001. The ledger holds no
/// such heading, so the two read the same set today; the difference is that a
/// malformed heading cannot resolve a citation of the number it starts with.
fn heading_number(line: &str) -> Option<&str> {
    let hashes = line.bytes().take_while(|&b| b == b'#').count();
    if !(2..=4).contains(&hashes) {
        return None;
    }
    let rest = &line[hashes..];
    let text = rest.trim_start_matches(' ');
    if text.len() == rest.len() {
        return None;
    }
    let id = text.get(..6)?;
    let bounded = !text.as_bytes().get(6).copied().is_some_and(is_word);
    (bounded && id.starts_with("D-") && id.bytes().skip(2).all(|b| b.is_ascii_digit()))
        .then_some(id)
}

/// A fence line's marker, when `line` is one: its character (`` ` `` or `~`),
/// the length of its run (three or more) and the text after the run.
fn fence(line: &str) -> Option<(u8, usize, &str)> {
    let text = line.trim_start();
    let mark = *text.as_bytes().first()?;
    if mark != b'`' && mark != b'~' {
        return None;
    }
    let run = text.bytes().take_while(|&b| b == mark).count();
    (run >= 3).then(|| (mark, run, &text[run..]))
}

/// The leading bold span of a list item, `* **...**` or `- **...**`.
fn leading_bold(line: &str) -> Option<&str> {
    let item = line.trim_start();
    let item = item
        .strip_prefix("* ")
        .or_else(|| item.strip_prefix("- "))?;
    let body = item.trim_start().strip_prefix("**")?;
    let end = body.find("**")?;
    Some(&body[..end])
}

/// What the ledger resolves: the numbers that head an entry, and the numbers
/// named pending, each with the ledger line that names it first.
struct Ledger {
    entries: BTreeSet<String>,
    pending: BTreeMap<String, usize>,
}

/// Reads `text` as the ledger's rule above reads it.
fn read_ledger(text: &str) -> Ledger {
    let mut entries = BTreeSet::new();
    let mut pending = BTreeMap::new();
    // The open fence's character and run. A fence closes only on a line of
    // the same character, at least as long, with nothing after it but
    // spaces (CommonMark's rule), so a `~~~` inside a ``` block, or a ```
    // inside a ```` block, is fenced text and closes nothing.
    let mut fenced: Option<(u8, usize)> = None;
    let mut in_note = false;
    for (index, line) in text.lines().enumerate() {
        if let Some((open_mark, open_run)) = fenced {
            if fence(line).is_some_and(|(mark, run, after)| {
                mark == open_mark && run >= open_run && after.trim().is_empty()
            }) {
                fenced = None;
            }
            continue;
        }
        if let Some((mark, run, _)) = fence(line) {
            fenced = Some((mark, run));
            continue;
        }
        if line.starts_with('#') {
            if let Some(id) = heading_number(line) {
                entries.insert(id.to_owned());
            }
            in_note = line
                .trim_start_matches('#')
                .trim()
                .starts_with("Cited, never written");
            continue;
        }
        if in_note && let Some(span) = leading_bold(line) {
            for (id, _) in citations(span) {
                pending.entry(id).or_insert(index + 1);
            }
        }
    }
    Ledger { entries, pending }
}

/// Every number `files` cites that `ledger` does not resolve, each with the
/// first file and line citing it, in the order `files` is given.
fn unresolved(ledger: &Ledger, files: &[(String, String)]) -> BTreeMap<String, (String, usize)> {
    let mut first = BTreeMap::new();
    for (path, text) in files {
        for (id, line) in citations(text) {
            if !ledger.entries.contains(&id) && !ledger.pending.contains_key(&id) {
                first.entry(id).or_insert_with(|| (path.clone(), line));
            }
        }
    }
    first
}

/// Every `.rs`, `.toml` and `.md` file below `dir`, in path order.
fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut paths: Vec<(PathBuf, std::fs::FileType)> = std::fs::read_dir(dir)
        .unwrap_or_else(|why| panic!("{} must be readable: {why}", dir.display()))
        .map(|entry| {
            let entry = entry.unwrap_or_else(|why| panic!("{} must list: {why}", dir.display()));
            let kind = entry
                .file_type()
                .unwrap_or_else(|why| panic!("{} has no type: {why}", entry.path().display()));
            (entry.path(), kind)
        })
        .collect();
    paths.sort_by(|a, b| a.0.cmp(&b.0));
    for (path, kind) in paths {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_else(|| panic!("{} is not a UTF-8 name", path.display()));
        assert!(
            !kind.is_symlink(),
            "{} is a symbolic link under crates/: it is neither followed nor skipped \
             silently; move it or say in this file why it is walked",
            path.display()
        );
        if kind.is_dir() {
            if name != "target" && !name.starts_with('.') {
                walk(&path, out);
            }
        } else if matches!(
            path.extension().and_then(|e| e.to_str()),
            Some("rs" | "toml" | "md")
        ) {
            out.push(path);
        }
    }
}

/// Every walked file under `crates/`, as (path from the root, text).
fn crate_sources() -> Vec<(String, String)> {
    let root = workspace_root();
    let mut paths = Vec::new();
    walk(&root.join("crates"), &mut paths);
    paths
        .into_iter()
        .map(|path| {
            let text = std::fs::read_to_string(&path)
                .unwrap_or_else(|why| panic!("{} must be readable text: {why}", path.display()));
            let shown = path
                .strip_prefix(&root)
                .unwrap_or(&path)
                .display()
                .to_string();
            (shown, text)
        })
        .collect()
}

/// **Every decision number cited under `crates/` heads an entry, or is named
/// in a "Cited, never written" note** (AF-07; law-3, law-4). Red on
/// feat/db-fix before AF-07 (14 Oct 2026) with 32 numbers, among them D-3349
/// to D-3353.
#[test]
fn every_cited_decision_heads_an_entry_or_is_named_pending() {
    let started = std::time::Instant::now();
    let ledger = read_ledger(LEDGER);
    // NOT VACUOUS: the ledger's first entry and D-0684's pending D-0051 are read.
    assert!(
        ledger.entries.contains("D-0001"),
        "no entry D-0001 was read"
    );
    assert!(
        ledger.pending.contains_key("D-0051"),
        "D-0684's note naming D-0051 was not read"
    );
    let files = crate_sources();
    let cited: BTreeSet<String> = files
        .iter()
        .flat_map(|(_, text)| citations(text).into_iter().map(|(id, _)| id))
        .collect();
    assert!(
        cited.len() > 100,
        "only {} distinct numbers cited under crates/: the walk read too little",
        cited.len()
    );
    let missing = unresolved(&ledger, &files);
    let mut said = String::new();
    for (id, (path, line)) in &missing {
        let _ = writeln!(said, "  {id}  first cited at {path}:{line}");
    }
    assert!(
        missing.is_empty(),
        "{} decision number(s) cited under crates/ head no entry of \
         docs/05-decisions.md and are named in no 'Cited, never written' note. \
         Write the entry, or name the number pending in the ledger with where it \
         resolves (CLAUDE.md §9; AF-07):\n{said}",
        missing.len()
    );
    eprintln!(
        "{} files, {} distinct numbers cited, {} entries, {} pending, {:?}",
        files.len(),
        cited.len(),
        ledger.entries.len(),
        ledger.pending.len(),
        started.elapsed()
    );
}

/// The reader on invented text: what counts as an entry, as pending, and as a
/// citation, and that an unresolved citation is named with its line.
#[test]
fn the_reader_resolves_entries_and_pending_names_and_nothing_else() {
    const FIXTURE: &str = r"# 05 — Decision ledger

### D-0001 — An entry
Text citing D-0002 in passing.

##### D-0003 — five hashes head nothing

```text
### D-0004 — a heading inside a fence heads nothing
```

####D-0009 — no space heads nothing

#### Cited, never written

* **D-0005.** Pending, named in the bold lead.
- **D-0010 and D-0011.** Two named in one lead.
* Not bold: D-0006.
Text D-0007 in the note's body.

#### From here

* **D-0008.** After the note: not pending.

### D-00012 — five digits head nothing
### D-0012x — a letter after the four heads nothing

```text
~~~
### D-0013 — a tilde run closes no backtick fence
```

````markdown
```
### D-0014 — a shorter run closes no longer fence
````

~~~
### D-0015 — inside a tilde fence
```
~~~ not a closer: text follows the run
### D-0016 — still inside
~~~
### D-0017 — after the fence closed, an entry
";
    let ledger = read_ledger(FIXTURE);
    assert_eq!(
        ledger
            .entries
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["D-0001", "D-0017"]
    );
    assert_eq!(
        ledger
            .pending
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["D-0005", "D-0010", "D-0011"]
    );
    assert_eq!(ledger.pending.get("D-0005"), Some(&16));

    // A cited number with an entry, and one named pending, pass.
    let clean = vec![(
        "crates/x.rs".to_owned(),
        "// D-0001, (D-0005; D-0010) and D-0011.".to_owned(),
    )];
    assert!(unresolved(&ledger, &clean).is_empty());

    // Not citations: a `D` after a letter, five digits, three digits, a digit
    // or a letter or `_` straight after the four.
    let none = vec![(
        "crates/y.rs".to_owned(),
        "SOURCED-2018 XD-0002 D-06961 D-002 D-0002x D-0002_ D-00021".to_owned(),
    )];
    assert!(
        citations(&none[0].1).is_empty(),
        "{:?}",
        citations(&none[0].1)
    );
    assert!(unresolved(&ledger, &none).is_empty());

    // Every other number, cited, fails, named once at its first file and line.
    let cited = vec![
        (
            "crates/a.rs".to_owned(),
            "fn a() {}\n// D-0002 D-0003\n// D-0004".to_owned(),
        ),
        (
            "crates/b.rs".to_owned(),
            "//! D-0002 again, D-0006 D-0007 D-0008 D-0009".to_owned(),
        ),
    ];
    let missing = unresolved(&ledger, &cited);
    let got: Vec<(&str, &str, usize)> = missing
        .iter()
        .map(|(id, (path, line))| (id.as_str(), path.as_str(), *line))
        .collect();
    assert_eq!(
        got,
        [
            ("D-0002", "crates/a.rs", 2),
            ("D-0003", "crates/a.rs", 2),
            ("D-0004", "crates/a.rs", 3),
            ("D-0006", "crates/b.rs", 1),
            ("D-0007", "crates/b.rs", 1),
            ("D-0008", "crates/b.rs", 1),
            ("D-0009", "crates/b.rs", 1),
        ]
    );
}

/// Days from 1970-01-01 to the civil date, the proleptic Gregorian count
/// `pull::calendar` keeps its days in (H. Hinnant's `days_from_civil`).
const fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Each figure the WP-62 entry states, as (file, constant). The value is read
/// from the constant's own declaration, never written here, so this list
/// names WHERE a figure lives and the code says what it is.
const PULL_RUNTIME_FIGURES: [(&str, &str); 10] = [
    ("crates/api/src/server.rs", "STORE_BREAKER"),
    ("crates/api/src/server.rs", "STORE_REFUSED_BREAKER"),
    ("crates/api/src/server.rs", "SETTLE_MARGIN_MINUTES"),
    ("crates/api/src/pullrun.rs", "MAX_PASSES"),
    ("crates/api/src/pullrun.rs", "CLEAN_EMPTY_PASSES"),
    ("crates/api/src/pullrun.rs", "INTERRUPTED_SCAN"),
    ("crates/api/src/audit.rs", "MAX_PAGE_RECORDS"),
    ("crates/api/src/pullrun.rs", "INTERRUPTED_RUNS"),
    ("crates/pull/src/calendar.rs", "SOURCED_LAST_DAY"),
    ("crates/pull/src/calendar.rs", "RECORD_ENDS_SOON_DAYS"),
];

/// The right-hand side of `name`'s one declaration in `text`
/// (`const NAME: T = <rhs>;`, `pub` or not), or why there is none.
fn declared<'a>(text: &'a str, name: &str) -> Result<&'a str, String> {
    let head = format!("const {name}: ");
    let found: Vec<&str> = text
        .lines()
        .map(str::trim_start)
        .map(|line| line.strip_prefix("pub ").unwrap_or(line))
        .filter(|line| line.starts_with(&head))
        .collect();
    let [line] = found.as_slice() else {
        return Err(format!(
            "`{name}` is declared {} times, not once",
            found.len()
        ));
    };
    line.split_once(" = ")
        .and_then(|(_, rhs)| rhs.strip_suffix(';'))
        .ok_or_else(|| format!("`{line}` is not `const {name}: T = <value>;`"))
}

/// The value of `name` as `file` declares it: integer literals (with `_`)
/// and `crate::<module>::<NAME>` paths, joined by ` * `. A path is read from
/// the same crate's `src/<module>.rs`. Anything else fails, naming it.
fn code_figure(root: &Path, file: &str, name: &str) -> i64 {
    let text = std::fs::read_to_string(root.join(file))
        .unwrap_or_else(|why| panic!("{file} must be readable: {why}"));
    let rhs = declared(&text, name).unwrap_or_else(|why| panic!("{file}: {why}"));
    rhs.split(" * ")
        .map(|term| {
            if let Ok(value) = term.replace('_', "").parse::<i64>() {
                return value;
            }
            let (module, constant) = term
                .strip_prefix("crate::")
                .and_then(|path| path.rsplit_once("::"))
                .unwrap_or_else(|| {
                    panic!("{file}: `{name}` = `{rhs}` holds `{term}`, which this test cannot read")
                });
            let src = file.find("/src/").map_or_else(
                || panic!("{file} is not under a crate's src/"),
                |at| &file[..at + "/src/".len()],
            );
            code_figure(
                root,
                &format!("{src}{}.rs", module.replace("::", "/")),
                constant,
            )
        })
        .product()
}

/// Every figure `text` states for `name` as `` `NAME` = <number> ``, the
/// number read WHOLE: its digits, with a `,` or `_` only between two digits.
/// So `` `X` = 30 `` is 30, never a 3 that a substring check would accept.
fn stated_figures(text: &str, name: &str) -> Vec<Result<i64, String>> {
    let marker = format!("`{name}` = ");
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(found) = text.get(from..).and_then(|rest| rest.find(&marker)) {
        let start = from + found + marker.len();
        let bytes = text.as_bytes();
        let mut end = start;
        while let Some(&b) = bytes.get(end) {
            let separator = matches!(b, b',' | b'_')
                && end > start
                && bytes.get(end + 1).is_some_and(u8::is_ascii_digit);
            if !b.is_ascii_digit() && !separator {
                break;
            }
            end += 1;
        }
        let digits = text[start..end].replace([',', '_'], "");
        out.push(
            digits
                .parse()
                .map_err(|_| format!("\"{marker}\" is followed by no number")),
        );
        from = start;
    }
    out
}

/// Fails unless `text` states `name` at least once and every time as `value`.
fn holds_figure(text: &str, what: &str, name: &str, value: i64, file: &str) {
    let stated = stated_figures(text, name);
    assert!(
        !stated.is_empty(),
        "{what} does not state `{name}`, which {file} holds as {value}"
    );
    for figure in stated {
        assert_eq!(
            figure,
            Ok(value),
            "{what} states `{name}` as {figure:?}; {file} holds {value}"
        );
    }
}

/// **The figures the ledger records for the pull runtime are the code's**
/// (AF-07; law-5). WP-62 landed them with no entry; the entry that records them
/// is found by `(WP-62)` in its heading, not by its number, which is taken at
/// landing. Each figure is read from its constant's declaration and must be
/// what the entry states, read as a whole number, every time it is stated;
/// the `docs/04-invariants.md` rows PRC-01 to PRC-07 are held the same way
/// wherever they state one. The calendar's last day is checked as the date
/// the entry names, by a day count that agrees with the calendar's own first
/// day.
#[test]
fn the_pull_runtime_figures_the_ledger_states_are_the_codes() {
    let mut lines = LEDGER.lines();
    let heading = lines
        .by_ref()
        .find(|line| heading_number(line).is_some() && line.contains("(WP-62)"))
        .expect("an entry whose heading names (WP-62)");
    let entry: String = lines
        .take_while(|line| heading_number(line).is_none())
        .collect::<Vec<_>>()
        .join("\n");
    let root = workspace_root();
    let invariants =
        std::fs::read_to_string(root.join("docs/04-invariants.md")).expect("docs/04-invariants.md");
    let rows: String = invariants
        .lines()
        .filter(|line| line.starts_with("| PRC-"))
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(
        rows.lines().count(),
        7,
        "docs/04-invariants.md must hold the seven rows PRC-01 to PRC-07"
    );
    for (file, name) in PULL_RUNTIME_FIGURES {
        let value = code_figure(&root, file, name);
        holds_figure(&entry, &format!("`{heading}`"), name, value, file);
        for figure in stated_figures(&rows, name) {
            assert_eq!(
                figure,
                Ok(value),
                "a PRC row of docs/04-invariants.md states `{name}` as {figure:?}; {file} holds {value}"
            );
        }
    }
    let calendar = "crates/pull/src/calendar.rs";
    assert_eq!(
        days_from_civil(2018, 1, 1),
        code_figure(&root, calendar, "SOURCED_FIRST_DAY"),
        "the day count disagrees with the calendar's own first day, 2018-01-01"
    );
    assert_eq!(
        days_from_civil(2026, 12, 31),
        code_figure(&root, calendar, "SOURCED_LAST_DAY"),
        "the calendar of record no longer ends on 2026-12-31"
    );
    assert!(
        entry.contains("2026-12-31"),
        "`{heading}` does not name the calendar of record's last day, 2026-12-31"
    );
}

/// The figure readers on invented text: a figure is read whole, a separator
/// only between digits; a declaration is found once or refused by name.
#[test]
fn the_figure_readers_read_whole_numbers_and_one_declaration() {
    let text = "`X` = 30 and `X` = 3. `Y` = 20,818; `Z` = 1_000, `W` = none `XX` = 9";
    assert_eq!(stated_figures(text, "X"), [Ok(30), Ok(3)]);
    assert_eq!(stated_figures(text, "Y"), [Ok(20_818)]);
    assert_eq!(stated_figures(text, "Z"), [Ok(1_000)]);
    assert!(stated_figures(text, "W")[0].is_err());
    assert!(stated_figures(text, "V").is_empty());

    let source = "pub const A: u64 = 4 * crate::audit::B;\n    const B: u64 = 1_000;\n\
                  const C: u32 = 1;\nconst C: u32 = 2;\nconst D: u32 = 5";
    assert_eq!(declared(source, "A"), Ok("4 * crate::audit::B"));
    assert_eq!(declared(source, "B"), Ok("1_000"));
    assert_eq!(
        declared(source, "C"),
        Err("`C` is declared 2 times, not once".to_owned())
    );
    assert_eq!(
        declared(source, "E"),
        Err("`E` is declared 0 times, not once".to_owned())
    );
    assert!(declared(source, "D").is_err());
}

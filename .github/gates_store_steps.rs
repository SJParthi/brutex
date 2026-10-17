//! `gates_store_steps`: the store steps are one checked list (R3-36).
//!
//! `config/store-steps.toml` is the one list of the writes a person runs on
//! the operator's store by hand. The runbooks that lived only in plans (WP-80
//! in the fix plan, IA-14's table in section 6.8 of the ingest design, neither
//! in the run script) are entries in it, one per step: what the step writes,
//! the exact command, the packages that must be merged and deployed first, the
//! dry run, the time and bytes that dry run took on a private copy, how to
//! check the store afterwards, and the findings and audit rows it closes.
//! Nothing in the repository runs a step. Each runs only on the operator's
//! yes, and a step is closed only by its run: `state = "done"` carries the
//! run's own record.
//!
//! This tool reads the file and refuses, each by name:
//!
//! * a step without a command, a dry run, a prerequisite, a check or a row it
//!   closes (`every_store_step_names_its_command_prerequisite_check_and_rows`);
//! * a leftover the file lists as waiting for a store run that no step closes,
//!   or that two steps close, and a closed row the list of leftovers does not
//!   hold (`every_leftover_waiting_for_a_store_run_is_named_by_one_step`);
//! * a pending step without rehearsal figures that does not name the unmet
//!   prerequisite it waits for, figures beside an unmerged prerequisite, and a
//!   step whose prerequisites are all met that still has no figures
//!   (`a_step_without_rehearsal_figures_names_what_it_waits_for`);
//! * a step that copies or snapshots the store (`tmutil`, `rsync`) without
//!   every writer stopped while the copy is taken, or whose check does not
//!   read a restore back (census recount, block scrub) before any writer
//!   starts (`a_backup_taken_under_writers_is_refused_or_verified_on_restore`,
//!   FM-221);
//! * a `cli census-recount gdfl` span, in a command, a dry run or a check,
//!   that covers a day D-3317 item 5 excludes, read from the decision ledger
//!   (`a_recount_span_never_covers_a_day_the_ledger_excludes`,
//!   verify-R3-36-r3), and a check line that runs a store writer without
//!   saying it writes;
//! * a run that did not close its step recorded as anything but the pending
//!   step's `last_run`, and a step done before a step it follows that does
//!   not say so in `ran_out_of_order`;
//! * a package whose `merged` flag the merge history contradicts, a rule kept
//!   past the package that ends it, a `cli` command whose verb is not one of
//!   the cli's store writers, a dry run that would write the real store, an
//!   `after` that names no step or closes a cycle, an environment variable or
//!   a package nothing uses, and every key or value the reader does not know.
//!
//! It starts no program. The workflow step hands it the merge subjects
//! (`git log --format=%s --merges`), the cli's writer table
//! (`crates/cli/src/store_write.rs`) and the decision ledger
//! (`docs/05-decisions.md`) as files. A history that names no package
//! merge (a squash-merged branch) leaves the `merged` flags as written and the
//! report says so in a line of its own; it is never read as "nothing merged".
//!
//! The reader takes a strict subset of TOML, which every TOML reader also
//! reads: `[[table]]` headers, `key = value` with a basic string, an array of
//! basic strings, a non-negative integer or a boolean, and `#` comments.
//! Anything else is refused with its line number.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::process::ExitCode;

// ------------------------------------------------------------------ reader --

/// One value the reader accepts.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Value {
    Text(String),
    List(Vec<String>),
    Number(u64),
    Flag(bool),
}

impl Value {
    fn kind(&self) -> &'static str {
        match self {
            Value::Text(_) => "a string",
            Value::List(_) => "an array of strings",
            Value::Number(_) => "an integer",
            Value::Flag(_) => "a boolean",
        }
    }
}

/// One table of the file: `kind` is empty for the keys before the first
/// header, else the header's name.
#[derive(Debug, Clone, Default)]
struct Entry {
    kind: String,
    line: usize,
    fields: BTreeMap<String, (Value, usize)>,
}

impl Entry {
    fn text(&self, key: &str) -> Option<&str> {
        match self.fields.get(key) {
            Some((Value::Text(t), _)) => Some(t),
            _ => None,
        }
    }

    fn list(&self, key: &str) -> &[String] {
        match self.fields.get(key) {
            Some((Value::List(l), _)) => l,
            _ => &[],
        }
    }

    fn number(&self, key: &str) -> Option<u64> {
        match self.fields.get(key) {
            Some((Value::Number(n), _)) => Some(*n),
            _ => None,
        }
    }

    fn flag(&self, key: &str) -> Option<bool> {
        match self.fields.get(key) {
            Some((Value::Flag(f), _)) => Some(*f),
            _ => None,
        }
    }

    fn has(&self, key: &str) -> bool {
        self.fields.contains_key(key)
    }

    /// The name a problem uses for this entry: its id or name, else its line.
    fn label(&self) -> String {
        if self.kind.is_empty() {
            return "the top level".to_owned();
        }
        match self.text("id").or_else(|| self.text("name")) {
            Some(id) if !id.is_empty() => format!("{} {id}", self.kind),
            _ => format!("{} at line {}", self.kind, self.line),
        }
    }
}

/// The tables the file may hold, in the order the file is read.
const TABLES: [&str; 5] = ["env", "package", "rule", "step", "owed"];

struct Reader<'a> {
    chars: std::iter::Peekable<std::str::Chars<'a>>,
    line: usize,
}

impl Reader<'_> {
    fn peek(&mut self) -> Option<char> {
        self.chars.peek().copied()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.chars.next();
        if c == Some('\n') {
            self.line += 1;
        }
        c
    }

    fn fail<T>(&self, why: &str) -> Result<T, String> {
        Err(format!("line {}: {why}", self.line))
    }

    fn spaces(&mut self) {
        while matches!(self.peek(), Some(' ' | '\t')) {
            self.bump();
        }
    }

    fn comment(&mut self) {
        if self.peek() == Some('#') {
            while !matches!(self.peek(), None | Some('\n')) {
                self.bump();
            }
        }
    }

    /// Blanks, comments and line ends, as between the items of an array.
    fn gaps(&mut self) {
        loop {
            self.spaces();
            self.comment();
            if self.peek() == Some('\n') {
                self.bump();
            } else {
                return;
            }
        }
    }

    fn line_end(&mut self) -> Result<(), String> {
        self.spaces();
        self.comment();
        match self.peek() {
            None => Ok(()),
            Some('\n') => {
                self.bump();
                Ok(())
            }
            Some('\r') => self.fail("a carriage return; the file is read with LF line ends only"),
            Some(c) => self.fail(&format!("{c:?} after a complete line")),
        }
    }

    fn word(&mut self) -> String {
        let mut w = String::new();
        while let Some(c) = self.peek() {
            if c.is_ascii_alphanumeric() || c == '_' {
                w.push(c);
                self.bump();
            } else {
                break;
            }
        }
        w
    }

    fn string(&mut self) -> Result<String, String> {
        self.bump();
        let mut s = String::new();
        loop {
            match self.bump() {
                None => return self.fail("a string is not closed before the end of the file"),
                Some('\n') => return self.fail("a string is not closed on its own line"),
                Some('"') => return Ok(s),
                Some('\\') => match self.bump() {
                    Some('"') => s.push('"'),
                    Some('\\') => s.push('\\'),
                    Some('n') => s.push('\n'),
                    Some('t') => s.push('\t'),
                    other => {
                        return self.fail(&format!(
                            "escape {other:?} is not read; the escapes are \\\" \\\\ \\n \\t"
                        ));
                    }
                },
                Some(c) if c.is_control() && c != '\t' => {
                    return self.fail(&format!("control character {c:?} inside a string"));
                }
                Some(c) => s.push(c),
            }
        }
    }

    fn list(&mut self) -> Result<Vec<String>, String> {
        self.bump();
        let mut items = Vec::new();
        loop {
            self.gaps();
            match self.peek() {
                Some(']') => {
                    self.bump();
                    return Ok(items);
                }
                Some('"') => items.push(self.string()?),
                None => return self.fail("an array is not closed before the end of the file"),
                Some(c) => return self.fail(&format!("{c:?} inside an array of strings")),
            }
            self.gaps();
            match self.peek() {
                Some(',') => {
                    self.bump();
                }
                Some(']') => {}
                None => return self.fail("an array is not closed before the end of the file"),
                Some(c) => return self.fail(&format!("{c:?} where a comma or ] belongs")),
            }
        }
    }

    fn value(&mut self) -> Result<Value, String> {
        match self.peek() {
            Some('"') => Ok(Value::Text(self.string()?)),
            Some('[') => Ok(Value::List(self.list()?)),
            Some(c) if c.is_ascii_digit() => {
                let w = self.word();
                let digits = w.replace('_', "");
                // TOML refuses a leading zero, so the subset does too. The
                // word opens with the digit just seen, so it cannot open with
                // `_`; and `u64` parses only ASCII digits here, because a word
                // holds no `+`.
                let tidy = !w.ends_with('_')
                    && !w.contains("__")
                    && (digits.len() == 1 || !digits.starts_with('0'));
                match digits.parse::<u64>() {
                    Ok(n) if tidy => Ok(Value::Number(n)),
                    _ => self.fail(&format!("{w:?} is not a non-negative integer")),
                }
            }
            Some(c) if c.is_ascii_alphabetic() => match self.word().as_str() {
                "true" => Ok(Value::Flag(true)),
                "false" => Ok(Value::Flag(false)),
                w => self.fail(&format!("{w:?} is not a value; strings are quoted")),
            },
            None => self.fail("a key has no value"),
            Some(c) => self.fail(&format!("{c:?} does not start a value")),
        }
    }
}

/// Every table of `text`, the top-level keys first.
fn read(text: &str) -> Result<Vec<Entry>, String> {
    let mut r = Reader {
        chars: text.chars().peekable(),
        line: 1,
    };
    let mut entries = vec![Entry {
        line: 1,
        ..Entry::default()
    }];
    loop {
        r.spaces();
        r.comment();
        match r.peek() {
            None => break,
            Some('\n') => {
                r.bump();
            }
            Some('[') => {
                let line = r.line;
                r.bump();
                if r.bump() != Some('[') {
                    return r.fail("only [[table]] headers are read");
                }
                let name = r.word();
                if r.bump() != Some(']') || r.bump() != Some(']') {
                    return r.fail("a [[table]] header is not closed by ]]");
                }
                if !TABLES.contains(&name.as_str()) {
                    return r.fail(&format!(
                        "[[{name}]] is not a table this file holds ({})",
                        TABLES.join(", ")
                    ));
                }
                r.line_end()?;
                entries.push(Entry {
                    kind: name,
                    line,
                    fields: BTreeMap::new(),
                });
            }
            Some(c) if c.is_ascii_lowercase() => {
                let line = r.line;
                let key = r.word();
                if !key
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
                {
                    return r.fail(&format!("key {key:?} is not lower_case"));
                }
                r.spaces();
                if r.bump() != Some('=') {
                    return r.fail(&format!("key {key:?} is not followed by ="));
                }
                r.spaces();
                let value = r.value()?;
                r.line_end()?;
                let entry = entries
                    .last_mut()
                    .expect("the top-level entry is never removed");
                if entry.fields.insert(key.clone(), (value, line)).is_some() {
                    return Err(format!(
                        "line {line}: key {key:?} is given twice in one table"
                    ));
                }
            }
            Some(c) => return r.fail(&format!("{c:?} does not start a key or a [[table]]")),
        }
    }
    Ok(entries)
}

// ----------------------------------------------------------------- schema --

#[derive(Clone, Copy, PartialEq, Eq)]
enum Shape {
    Text,
    List,
    Number,
    Flag,
}

impl Shape {
    fn fits(self, v: &Value) -> bool {
        matches!(
            (self, v),
            (Shape::Text, Value::Text(_))
                | (Shape::List, Value::List(_))
                | (Shape::Number, Value::Number(_))
                | (Shape::Flag, Value::Flag(_))
        )
    }

    fn name(self) -> &'static str {
        match self {
            Shape::Text => "a string",
            Shape::List => "an array of strings",
            Shape::Number => "an integer",
            Shape::Flag => "a boolean",
        }
    }
}

/// The keys each table may hold, with their shape and whether every entry of
/// that table must hold it. The step's state-dependent keys are checked below.
fn keys(kind: &str) -> &'static [(&'static str, Shape, bool)] {
    use Shape::{Flag, List, Number, Text};
    match kind {
        "" => &[("format", Number, true), ("leftovers", List, true)],
        "env" => &[("name", Text, true), ("means", Text, true)],
        "package" => &[
            ("name", Text, true),
            ("merged", Flag, true),
            ("title", Text, true),
        ],
        "rule" => &[
            ("id", Text, true),
            ("text", Text, true),
            ("source", List, true),
            ("until", Text, true),
            ("retired", Text, false),
        ],
        "step" => &[
            ("id", Text, true),
            ("title", Text, true),
            ("state", Text, true),
            ("writes", Text, true),
            ("command", List, true),
            ("dry_run", List, true),
            ("env", List, false),
            ("prerequisites", List, true),
            ("after", List, false),
            ("check", List, true),
            ("closes", List, true),
            ("helps", List, false),
            ("rehearsed_ms", Number, false),
            ("rehearsed_bytes", Number, false),
            ("rehearsed_files", Number, false),
            ("rehearsed_on", Text, false),
            ("rehearsed_result", Text, false),
            ("not_rehearsed", Text, false),
            ("disk", Text, false),
            ("ran", Text, false),
            ("ran_ms", Number, false),
            ("ran_out_of_order", Text, false),
            ("last_run", Text, false),
            ("approval", Text, false),
            ("withdrawn_because", Text, false),
            ("replaced_by", Text, false),
            ("note", Text, false),
            ("kind", Text, false),
            ("rehearsed_commit", Text, false),
            ("writer_changed", Text, false),
            ("snapshot", Text, false),
        ],
        "owed" => &[
            ("id", Text, true),
            ("what", Text, true),
            ("source", List, true),
            ("note", Text, false),
        ],
        _ => &[],
    }
}

/// The four rehearsal figures, written together or not at all.
const FIGURES: [&str; 4] = [
    "rehearsed_ms",
    "rehearsed_bytes",
    "rehearsed_on",
    "rehearsed_result",
];

/// How a command line may begin: a store-writing cli verb, a request to the
/// app's own form route, or either of those that a package has not built yet
/// (`planned: `). Nothing else: no script, no system tool, no prose (R4-06
/// item 3, AUT-07). The snapshot and the copy are `cli` verbs too, planned
/// until R4-09 builds `cli backup` and a package builds the snapshot verb.
const HEADS: [&str; 4] = ["cli ", "POST ", "planned: cli ", "planned: POST "];

/// The prefix that marks a prerequisite met only by the operator.
const OPERATOR: &str = "operator: ";

/// How a command that copies or snapshots the store begins: the copy is
/// taken while nothing writes, and a restore is checked before anything does.
/// `cli backup` is R4-09's verb; the snapshot verb has no package yet.
const COPIES: [&str; 3] = [
    "cli backup ",
    "planned: cli backup ",
    "planned: cli snapshot ",
];

/// The prerequisite every copy or snapshot step names, word for word (FM-221,
/// HUMAN-078): a copy taken while a writer appends holds torn months or a
/// census that disagrees with them, and restoring it rolls back silently.
const STOPPED: &str =
    "operator: every writer stopped (the app and every cli run) while the copy is taken";

/// The two read-backs a copy or snapshot step's check names, so a restore is
/// verified before any writer starts: the census recounted from the months
/// and every block read against its checksum.
const RESTORE_CHECKS: [&str; 2] = ["cli census-recount ", "cli scrub-blocks "];

/// The wording a check line that runs a store writer carries, followed by
/// what it writes, so whoever runs the check runs it as the write it is
/// (verify-R3-36-r3: the restore read-back is `cli census-recount`, which
/// rebuilds census rows and appends `recounted` journal lines, and it was
/// written as a read).
const CHECK_WRITES: &str = "this check writes ";

/// How a census recount begins, wherever a line names one.
const RECOUNT: &str = "cli census-recount ";

/// The vendor whose import journal a recount closes days in: the cli's
/// recount appends `recounted` lines for this vendor alone
/// (`pull::census_store::recount`, `if vendor == Vendor::Gdfl`), so its span
/// is the one the ledger bounds.
const JOURNALED: &str = "gdfl";

/// The decision-ledger entry, and the item of it, that name the days a GDFL
/// recount's span must leave out: the vendor-refused days whose journal
/// lines carry no stage, which the recount cannot tell from a census refusal
/// (D-3317 item 5).
const EXCLUDING_ENTRY: &str = "### D-3317 ";
const EXCLUDING_ITEM: &str = "5. ";

/// What may end a day written in prose (`--to 2018-11-05,`): the punctuation
/// ends the command as well as the word.
const DAY_ENDS: [char; 5] = [',', ';', ':', '.', ')'];

/// The rsync options that remove a file, on either side: a copy of an
/// append-only store never removes one (CLAUDE.md section 3 rule 8).
/// Matched as prefixes, so `--del` covers every `--delete-*`.
const RSYNC_REMOVES: [&str; 3] = ["--del", "--remove-source-files", "--force"];

// ------------------------------------------------------------------ checks --

/// One refusal: the named rule it breaks and what, by name.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Problem {
    rule: &'static str,
    what: String,
}

fn problem(rule: &'static str, what: String) -> Problem {
    Problem { rule, what }
}

/// What the workflow hands the tool besides the list.
#[derive(Debug, Default)]
struct Evidence {
    /// The packages a `Merge <name>: ...` subject names, or `None` when no
    /// subjects were read.
    merged: Option<BTreeSet<String>>,
    /// The cli's store-writing verbs and its rule for when each writes, or
    /// `None` when the cli source was not read.
    writers: Option<Writers>,
    /// Rows the fix reports mark as needing a store write.
    marked: Vec<String>,
    /// The days no GDFL recount span may cover, read from D-3317 item 5 of
    /// the decision ledger, or `None` when the ledger was not read.
    excluded: Option<Vec<String>>,
    /// The owed runs of `config/owed-runs.toml` (R4-06, AUT-07): each id, or
    /// `None` when that list was not read.
    owed: Option<Vec<String>>,
}

/// The tables of one kind.
fn of<'a>(entries: &'a [Entry], kind: &str) -> Vec<&'a Entry> {
    entries.iter().filter(|e| e.kind == kind).collect()
}

/// Whether `id` is a finding or audit row id: an uppercase head (`DB`,
/// `SC`, `CMP`) and one or more `-`-joined parts of uppercase letters and
/// digits (`DB-A01`, `SC-CLOSE-21`).
fn is_row_id(id: &str) -> bool {
    let Some((head, rest)) = id.split_once('-') else {
        return false;
    };
    head.chars().next().is_some_and(|c| c.is_ascii_uppercase())
        && head
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
        && rest.split('-').all(|p| {
            !p.is_empty()
                && p.chars()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
        })
}

fn is_package_name(name: &str) -> bool {
    let Some((head, tail)) = name.split_once('-') else {
        return false;
    };
    let digits = tail.trim_end_matches(|c: char| c.is_ascii_lowercase());
    head.chars().next().is_some_and(|c| c.is_ascii_uppercase())
        && head
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
        && !digits.is_empty()
        && digits.chars().all(|c| c.is_ascii_digit())
        && tail.len() - digits.len() <= 1
}

fn is_numbered(id: &str, prefix: &str) -> bool {
    id.strip_prefix(prefix)
        .is_some_and(|n| n.len() >= 2 && n.chars().all(|c| c.is_ascii_digit()))
}

/// The one wording that puts a dry run on a private copy of the store.
const PRIVATE: &str = " with BRUTEX_STORE on a private copy";

/// Words that, anywhere in a dry run, put it back on the operator's store.
const REAL_STORE: [&str; 4] = [
    "real store",
    "operator's store",
    "live store",
    "runtime/store",
];

/// The only options an `rsync` dry run may give, each one that reads or
/// reports and opens no file of its own. `--dry-run` stops rsync changing
/// the destination, not rsync writing a file an option names:
/// `--log-file=FILE` writes FILE under `--dry-run` (verify-R3-36-r2, run on
/// this machine), and `--write-batch` and `--only-write-batch` exist to
/// write a file. So the list names what is allowed, and every other option,
/// a short-option cluster such as `-an` included, is refused until a
/// reason to allow it is written here.
const RSYNC_DRY_OPTIONS: [&str; 10] = [
    "-a",
    "--archive",
    "--dry-run",
    "--stats",
    "-v",
    "--verbose",
    "-i",
    "--itemize-changes",
    "-h",
    "--human-readable",
];

/// Words that deny the clause they stand in.
const DENIALS: [&str; 4] = ["not", "never", "nor", "instead"];

/// What may follow a `no` that answers ("No: the operator's own"), where a
/// `no` that qualifies a noun ("no snapshot was taken") is followed by a
/// space and denies nothing.
const ANSWERS: &str = ":,.;!?)";

/// The two apostrophes a `n't` denial is written with: the typewriter one and
/// the typographic one (U+2019), which an editor substitutes as you type
/// (verify-R3-36-r3: `isn` U+2019 `t` passed as a word that denies nothing).
const APOSTROPHES: [char; 2] = ['\'', '\u{2019}'];

/// Characters that chain, substitute or redirect a second command. A
/// newline (the reader's `\n` escape) ends one shell command and starts the
/// next, as `;` does.
const CHAINS: [&str; 7] = [";", "&", "|", "`", ">", "$(", "\n"];

/// Whether `c` reaches a program as itself when a shell reads a command's
/// words. A quote, a backslash, a `$`, a glob, a brace, a `#` and every
/// character outside ASCII are rewritten, expanded or cut by the shell
/// first: `cli gdfl-rekey "--apply"` hands the cli the word --apply, and
/// `rsync -a src dst # --dry-run` hands rsync no --dry-run at all. A dry run
/// holding one is not judged by the words it shows (verify-R3-36-r1, b01 to
/// b03).
fn plain(c: char) -> bool {
    c.is_ascii_alphanumeric() || " -_./:=,+@%".contains(c)
}

/// Whether `word` denies its clause: one of [`DENIALS`] or a `n't` form,
/// with either apostrophe.
fn denies(word: &str) -> bool {
    let w = word.to_ascii_lowercase();
    DENIALS.contains(&w.as_str()) || APOSTROPHES.iter().any(|a| w.ends_with(&format!("n{a}t")))
}

/// Whether `c` is part of a word for [`denies`]: a letter, a digit or an
/// apostrophe.
fn in_word(c: char) -> bool {
    c.is_ascii_alphanumeric() || APOSTROPHES.contains(&c)
}

/// The first word of `text` that denies it: one [`denies`] names, or a `no`
/// that answers, followed by one of [`ANSWERS`] or by the end of the text
/// (verify-R3-36-r3: `No:` after the private-copy wording passed).
fn denial(text: &str) -> Option<&str> {
    let mut at = 0;
    for word in text.split(|c: char| !in_word(c)) {
        let end = at + word.len();
        let next = text[end..].chars().next();
        at = end + next.map_or(0, char::len_utf8);
        if denies(word)
            || (word.eq_ignore_ascii_case("no") && next.is_none_or(|c| ANSWERS.contains(c)))
        {
            return Some(word);
        }
    }
    None
}

/// How one verb of the cli's `writes_the_store` lets its arguments choose a
/// read-only form, read from the cli's own match arm.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Form {
    /// `!arguments.contains(&"F")`: writes unless the word F is given.
    Unless(String),
    /// `arguments.contains(&"F")`: writes only when the word F is given.
    With(String),
    /// `matches!(arguments.first(), Some(&("A" | "B")))`: writes only when
    /// the first argument is one of these words.
    First(Vec<String>),
}

/// The cli's store writers and its rule for when each writes, both read from
/// `crates/cli/src/store_write.rs`, so the gate keeps no second copy of
/// either: a new read-only form, or a verb that loses one, reaches the gate
/// with the cli change that makes it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Writers {
    /// The verbs of `WRITERS`.
    verbs: BTreeSet<String>,
    /// The verbs whose arguments can choose a read-only form, with the form.
    forms: BTreeMap<String, Form>,
}

impl Writers {
    /// `writes_the_store(verb, arguments)`, arm for arm, over whole words.
    fn writes(&self, verb: &str, arguments: &[&str]) -> bool {
        match self.forms.get(verb) {
            Some(Form::Unless(word)) => !arguments.contains(&word.as_str()),
            Some(Form::With(word)) => arguments.contains(&word.as_str()),
            Some(Form::First(words)) => arguments
                .first()
                .is_some_and(|a| words.iter().any(|w| w == a)),
            None => self.verbs.contains(verb),
        }
    }
}

/// `text` with every `//` and `/./` in a path made one `/`, until none is
/// left: the spellings of one directory a path resolver treats as the same
/// (verify-R3-36-r3: `runtime//store` and `runtime/./store` named the
/// operator's store and passed as a private copy).
fn squeezed(text: &str) -> String {
    let mut s = text.to_owned();
    loop {
        let Some((at, len)) = ["//", "/./"]
            .iter()
            .find_map(|p| s.find(p).map(|at| (at, p.len())))
        else {
            return s;
        };
        s.replace_range(at..at + len, "/");
    }
}

/// Why `line` does not put its store on a private copy, or `Ok` when it
/// does: it says [`PRIVATE`] exactly once, as words of their own; it sets no
/// `BRUTEX_STORE` itself; nothing in it names the operator's store; and no
/// word after the wording denies it.
fn private_copy(line: &str) -> Result<(), String> {
    let Some((before, after)) = line
        .split_once(PRIVATE)
        .filter(|(_, after)| !after.contains(PRIVATE))
    else {
        return Err(format!("it does not say {PRIVATE:?} exactly once"));
    };
    if !(after.is_empty() || after.starts_with(' ')) {
        return Err(format!("{PRIVATE:?} runs on into {after:?}"));
    }
    let lower = squeezed(&line.to_ascii_lowercase());
    if let Some(w) = REAL_STORE.iter().find(|w| lower.contains(*w)) {
        return Err(format!("it also says {w:?}"));
    }
    if before.contains("BRUTEX_STORE=") {
        return Err("it sets BRUTEX_STORE itself".to_owned());
    }
    if let Some(w) = denial(after) {
        return Err(format!("{w:?} after the wording denies it"));
    }
    Ok(())
}

/// Why the dry run `line` could write the operator's store, or `None` when it
/// cannot. Decided by whole words, the way the cli decides:
///
/// * `planned: ` names a form no package has built, so nothing runs it;
/// * a `POST` or a `tmutil` line is a write whatever follows it;
/// * an `rsync` line writes its destination unless the word `--dry-run` is
///   one of its options (before any `--`), every option is one of
///   [`RSYNC_DRY_OPTIONS`], and every character of it reaches rsync as
///   itself ([`plain`]);
/// * a `cli` line chains nothing anywhere ([`CHAINS`]), and is read-only
///   when its command is [`plain`] and the cli's own `writes_the_store` says
///   its verb and words write nothing (a verb outside `WRITERS`, or the
///   verb's own read-only form); else only on a private copy
///   ([`private_copy`]). Without the cli source no read-only form is known,
///   so every `cli` dry run needs the private copy;
/// * a description (`the ...`, the step's commands in words) runs them on
///   the operator's store unless it puts them on a private copy;
/// * anything else is refused.
fn dry_run_writes(line: &str, writers: Option<&Writers>) -> Option<String> {
    if line.starts_with("planned: ") {
        return None;
    }
    if line.starts_with("POST ") || line.starts_with("tmutil ") {
        return Some("a POST or a tmutil line writes, whatever follows it".to_owned());
    }
    if let Some(rest) = line.strip_prefix("rsync ") {
        if let Some(c) = rest.chars().find(|c| !plain(*c)) {
            return Some(format!(
                "{c:?} is rewritten by a shell before rsync reads its words, so --dry-run may not reach it"
            ));
        }
        // `--` ends rsync's options: a --dry-run after it names a file.
        let options: Vec<&str> = rest
            .split_whitespace()
            .take_while(|w| *w != "--")
            .filter(|w| w.starts_with('-'))
            .collect();
        if let Some(o) = options.iter().find(|o| !RSYNC_DRY_OPTIONS.contains(*o)) {
            return Some(format!(
                "rsync option {o} is not one of {} (an option can write a file of its own under --dry-run)",
                RSYNC_DRY_OPTIONS.join(" ")
            ));
        }
        return (!options.contains(&"--dry-run"))
            .then(|| "rsync without the option --dry-run writes its destination".to_owned());
    }
    if let Some(rest) = line.strip_prefix("cli ") {
        // The whole line: a chain after the private-copy wording runs a
        // second command on whatever store it names (verify-R3-36-r1, b05).
        if let Some(c) = CHAINS.iter().find(|c| line.contains(*c)) {
            return Some(format!("{c:?} chains or redirects a second command"));
        }
        let command = rest
            .split_once(PRIVATE)
            .map_or(rest, |(command, _)| command);
        let mut words = command.split_whitespace();
        let Some(verb) = words.next() else {
            return Some("it names no cli verb".to_owned());
        };
        let arguments: Vec<&str> = words.collect();
        let known = match (writers, command.chars().find(|c| !plain(*c))) {
            (_, Some(c)) => format!(
                "{c:?} is rewritten by a shell before the cli reads its words, so the words it shows are not the words the cli receives"
            ),
            (Some(w), None) if !w.writes(verb, &arguments) => return None,
            (Some(_), None) => format!("cli {verb} writes with these words"),
            (None, None) => {
                "the cli's writer table was not given, so no read-only form is known".to_owned()
            }
        };
        return private_copy(line).err().map(|why| {
            format!("{known}; it names no read-only form of the cli's rule, and {why}")
        });
    }
    if line.starts_with("the ") {
        return private_copy(line).err().map(|why| {
            format!("a description of the step's commands runs them on the operator's store unless it says otherwise, and {why}")
        });
    }
    Some(
        "a dry run begins with cli, rsync, planned: or 'the ' (the step's commands in words), never with anything else"
            .to_owned(),
    )
}

/// Whether `why` says, in one clause of its own (clauses end at `.` and
/// `;`), that the step ran before `id`: the clause holds the word `ran`,
/// names `id` in the run of step ids that follows a `before`, and holds no
/// word that denies it ([`denial`]).
fn says_ran_before(why: &str, id: &str) -> bool {
    why.split(['.', ';']).any(|clause| {
        let words: Vec<&str> = clause
            .split(|c: char| !(in_word(c) || c == '-'))
            .filter(|w| !w.is_empty())
            .collect();
        if denial(clause).is_some() || !words.iter().any(|w| w.eq_ignore_ascii_case("ran")) {
            return false;
        }
        let mut listing = false;
        for w in words {
            let w = w
                .strip_suffix("'s")
                .or_else(|| w.strip_suffix("\u{2019}s"))
                .unwrap_or(w);
            if w.eq_ignore_ascii_case("before") {
                listing = true;
            } else if listing && w == id {
                return true;
            } else if !(w == "and" || w == "or" || is_numbered(w, "S-")) {
                listing = false;
            }
        }
        false
    })
}

/// Every refusal the list earns against `evidence`.
fn check(entries: &[Entry], evidence: &Evidence) -> Vec<Problem> {
    let mut out = Vec::new();
    shapes(entries, &mut out);
    let packages = packages(entries, evidence, &mut out);
    let steps = of(entries, "step");
    let ids = step_ids(&steps, &mut out);
    for step in &steps {
        fields(step, &packages, &mut out);
        rehearsal(step, &packages, &mut out);
        commands(step, &packages, evidence, &mut out);
        copies(step, &mut out);
        recounts(step, evidence, &mut out);
        check_writes(step, evidence, &mut out);
        forms(step, &mut out);
        records(step, &mut out);
    }
    order(&steps, &ids, &mut out);
    leftovers(entries, &steps, evidence, &mut out);
    rules(entries, &packages, &mut out);
    kinds(&steps, &mut out);
    owed(entries, evidence, &mut out);
    uses(entries, &steps, &mut out);
    out
}

// ------------------------------------------------------- R4-06 (AUT-07) --

/// Words a command line never holds: a loop written in prose. A store step
/// is one line per day or a range verb, never "for each" (R4-06 item 3).
const LOOP_WORDS: [&str; 4] = ["each", "every", "foreach", "loop"];

/// What marks a script or a build path in a command line: the cli is the
/// deployed release binary named `cli`, run with the [[env]] roots, never a
/// wrapper script, a build folder or a binary under the work folder.
const NOT_THE_CLI: [&str; 5] = [".sh", "tgt/", "target/", "/release/", "state/"];

/// Words that make a kind one no scheduler starts: a deletion, a pruning, a
/// retirement or a credential change (AUT-05). The same list is
/// `pull::rehearsal::NEVER_AUTOMATIC`, which refuses them at run time.
const NEVER_AUTOMATIC: [&str; 8] = [
    "delete",
    "remove",
    "prune",
    "retire",
    "credential",
    "token",
    "secret",
    "password",
];

/// Where a run's record is kept: a run id, or one of the store's own
/// records (the list's header names them).
const RECORDS: [&str; 7] = [
    "logs/cli/",
    "cli.lifecycle",
    "imports/gdfl.journal",
    "imports/gdfl.lineage",
    "audit/",
    "events.ndjson",
    "run=",
];

/// `line` with each single-quoted span read as one word `Q`: the shell
/// hands what is inside to the program as itself, so its `&` (`'GVT&D'`) or
/// spaces are not the shell's.
fn outside_quotes(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut quoted = false;
    for c in line.chars() {
        if c == '\'' {
            if !quoted {
                out.push('Q');
            }
            quoted = !quoted;
        } else if !quoted {
            out.push(c);
        }
    }
    out
}

/// Whether `text` names a run id: `run <digits>`, `runs <digits>` or
/// `run=<id>`.
fn names_a_run(text: &str) -> bool {
    let words: Vec<&str> = text.split_whitespace().collect();
    words.windows(2).any(|pair| {
        let head = pair[0].trim_start_matches('(');
        (head == "run" || head == "runs")
            && pair[1]
                .trim_end_matches(|c: char| !c.is_ascii_digit())
                .chars()
                .all(|c| c.is_ascii_digit())
            && pair[1].starts_with(|c: char| c.is_ascii_digit())
    })
}

/// Every command line is a bare `cli <verb>` or a `POST <route>` of the app,
/// built or `planned:` ([`HEADS`], refused by the `commands` rule), and none
/// is a script, a build path, an absolute path under the operator's work
/// folder, or a loop: one line per day or a range verb (R4-06 item 3;
/// AUT-07). A `planned:` line may explain itself in words, and a loop word
/// is refused there too: what is planned is one command, not a recipe.
fn forms(step: &Entry, out: &mut Vec<Problem>) {
    let label = step.label();
    for line in step.list("command") {
        let body = line.strip_prefix("planned: ").unwrap_or(line);
        let bare = outside_quotes(&outside_placeholders(body));
        let lower = bare.to_ascii_lowercase();
        if let Some(word) = lower
            .split(|c: char| !c.is_ascii_alphanumeric())
            .find(|w| LOOP_WORDS.contains(w))
        {
            out.push(problem(
                "forms",
                format!("{label}: command {line:?} says {word:?}: a loop is one line per day or a range verb, never prose"),
            ));
        }
        if let Some(mark) = NOT_THE_CLI.iter().find(|m| lower.contains(*m)) {
            out.push(problem(
                "forms",
                format!("{label}: command {line:?} holds {mark:?}: the cli is the deployed release binary run as `cli`, never a script or a build path"),
            ));
        }
        let paths: Vec<&str> = if body.starts_with("cli ") {
            bare.split_whitespace()
                .filter(|w| w.starts_with('/'))
                .collect()
        } else if body.starts_with("POST ") {
            bare.split_whitespace()
                .skip(2)
                .filter(|w| w.starts_with('/'))
                .collect()
        } else {
            bare.split_whitespace()
                .filter(|w| w.contains("/brutex") || w.contains("/work-"))
                .collect()
        };
        if let Some(path) = paths.first() {
            out.push(problem(
                "forms",
                format!("{label}: command {line:?} names the path {path:?}: name a root by its [[env]] variable"),
            ));
        }
    }
}

/// A run is recorded with its record: a done step's `ran` and a pending
/// step's `last_run` name a run id or the store record that holds the run
/// (R4-06: a run step carries its run id and its check; the check itself is
/// required of every step by [`fields`]).
fn records(step: &Entry, out: &mut Vec<Problem>) {
    let label = step.label();
    for key in ["ran", "last_run"] {
        if let Some(text) = step.text(key)
            && !names_a_run(text)
            && !RECORDS.iter().any(|r| text.contains(r))
        {
            out.push(problem(
                "records",
                format!(
                    "{label}: {key} names no run id and no store record it was read from ({})",
                    RECORDS.join(", ")
                ),
            ));
        }
    }
}

/// A `kind` (the name an automatic starter asks the rehearsal gate for) is
/// named by one step, is never a deletion or a credential change, and its
/// stamps read as `<name> <unix seconds>` (AUT-05).
fn kinds(steps: &[&Entry], out: &mut Vec<Problem>) {
    let mut seen: BTreeMap<&str, &str> = BTreeMap::new();
    for step in steps {
        let label = step.label();
        let Some(kind) = step.text("kind") else {
            for key in ["rehearsed_commit", "writer_changed", "snapshot"] {
                if step.has(key) {
                    out.push(problem(
                        "kinds",
                        format!(
                            "{label}: {key} without a kind: only an automatic kind is judged by it"
                        ),
                    ));
                }
            }
            continue;
        };
        let id = step.text("id").unwrap_or("?");
        if kind.is_empty()
            || !kind
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            out.push(problem(
                "kinds",
                format!("{label}: kind {kind:?} is not lower-case words joined by -"),
            ));
        }
        let lower = kind.to_ascii_lowercase();
        if let Some(word) = NEVER_AUTOMATIC.iter().find(|w| lower.contains(*w)) {
            out.push(problem(
                "kinds",
                format!("{label}: kind {kind:?} is a {word} kind: deletions and credential changes are never automatic"),
            ));
        }
        if let Some(other) = seen.insert(kind, id) {
            out.push(problem(
                "kinds",
                format!("kind {kind:?} is named by {other} and {id}"),
            ));
        }
        for key in ["rehearsed_commit", "writer_changed", "snapshot"] {
            if let Some(text) = step.text(key) {
                let words: Vec<&str> = text.split_whitespace().collect();
                let reads = words.len() == 2 && words[1].parse::<i64>().is_ok();
                if !reads {
                    out.push(problem(
                        "kinds",
                        format!("{label}: {key} {text:?} is not \"<name> <unix seconds>\""),
                    ));
                }
            }
        }
    }
}

/// The owed runs of `config/owed-runs.toml`, by id, or why not: format 1,
/// one or more [[owed]] tables, each id `OWED-<two or more digits>` once.
fn owed_from(text: &str) -> Result<Vec<String>, String> {
    let entries = read(text).map_err(|why| format!("the owed runs: {why}"))?;
    let mut problems = Vec::new();
    shapes(&entries, &mut problems);
    let mut ids = Vec::new();
    for entry in &entries[1..] {
        if entry.kind != "owed" {
            return Err(format!(
                "the owed runs: [[{}]] is not an [[owed]] table",
                entry.kind
            ));
        }
        let Some(id) = entry.text("id") else { continue };
        if !is_numbered(id, "OWED-") {
            return Err(format!(
                "the owed runs: id {id:?} is not OWED-<two or more digits>"
            ));
        }
        if ids.iter().any(|i| i == id) {
            return Err(format!("the owed runs: {id} is given twice"));
        }
        ids.push(id.to_owned());
    }
    // The owed file's top level holds `format` alone: its schema is the
    // owed table's, not the steps list's, so the leftovers key is not owed.
    problems.retain(|p| !p.what.contains("no leftovers"));
    if let Some(p) = problems.first() {
        return Err(format!("the owed runs: {}", p.what));
    }
    if ids.is_empty() {
        return Err("the owed runs: no [[owed]] table: an empty list proves nothing".to_owned());
    }
    Ok(ids)
}

/// Every owed run is a leftover (so exactly one step closes it, by the
/// leftovers rule), and every `OWED-` leftover is an owed run; the steps
/// list itself holds no [[owed]] table.
fn owed(entries: &[Entry], evidence: &Evidence, out: &mut Vec<Problem>) {
    for entry in of(entries, "owed") {
        out.push(problem(
            "owed",
            format!(
                "{}: owed runs live in config/owed-runs.toml, not in the steps list",
                entry.label()
            ),
        ));
    }
    let Some(ids) = &evidence.owed else {
        return;
    };
    let listed = entries[0].list("leftovers");
    for id in ids {
        if !listed.contains(id) {
            out.push(problem(
                "owed",
                format!("owed run {id} is not a leftover, so no step is held to name it"),
            ));
        }
    }
    for row in listed.iter().filter(|r| r.starts_with("OWED-")) {
        if !ids.contains(row) {
            out.push(problem(
                "owed",
                format!("leftover {row} is no owed run of config/owed-runs.toml"),
            ));
        }
    }
}

fn shapes(entries: &[Entry], out: &mut Vec<Problem>) {
    let top = &entries[0];
    for entry in entries {
        let allowed = keys(&entry.kind);
        for (key, (value, line)) in &entry.fields {
            match allowed.iter().find(|(k, _, _)| k == key) {
                None => out.push(problem(
                    "reader",
                    format!(
                        "{}: key {key:?} (line {line}) is not one this table holds",
                        entry.label()
                    ),
                )),
                Some((_, shape, _)) if !shape.fits(value) => out.push(problem(
                    "reader",
                    format!(
                        "{}: {key} (line {line}) is {}, not {}",
                        entry.label(),
                        value.kind(),
                        shape.name()
                    ),
                )),
                Some(_) => {}
            }
        }
        for (key, _, required) in allowed {
            if *required && !entry.has(key) {
                out.push(problem(
                    if entry.kind == "step" {
                        "fields"
                    } else {
                        "reader"
                    },
                    format!("{}: no {key}", entry.label()),
                ));
            }
        }
    }
    if top.number("format").is_some_and(|f| f != 1) {
        out.push(problem(
            "reader",
            "format is not 1: this reader knows format 1 only".to_owned(),
        ));
    }
}

/// The declared packages, name to merged, with their checks.
fn packages(
    entries: &[Entry],
    evidence: &Evidence,
    out: &mut Vec<Problem>,
) -> BTreeMap<String, bool> {
    let mut map = BTreeMap::new();
    for p in of(entries, "package") {
        let (Some(name), Some(merged)) = (p.text("name"), p.flag("merged")) else {
            continue;
        };
        if !is_package_name(name) {
            out.push(problem(
                "packages",
                format!("{}: {name:?} is not a package name", p.label()),
            ));
        }
        if map.insert(name.to_owned(), merged).is_some() {
            out.push(problem(
                "packages",
                format!("package {name} is declared twice"),
            ));
        }
        if let Some(seen) = evidence.merged.as_ref().filter(|s| !s.is_empty()) {
            let in_history = seen.contains(name);
            if in_history && !merged {
                out.push(problem(
                    "packages",
                    format!(
                        "package {name} is merged (a 'Merge {name}:' subject is in the history) but the list says merged = false: set it true and rehearse every step that waited for it"
                    ),
                ));
            } else if !in_history && merged {
                out.push(problem(
                    "packages",
                    format!(
                        "package {name} says merged = true but no 'Merge {name}:' subject is in the history read"
                    ),
                ));
            }
        }
    }
    map
}

fn step_ids(steps: &[&Entry], out: &mut Vec<Problem>) -> BTreeMap<String, String> {
    let mut ids = BTreeMap::new();
    for s in steps {
        let Some(id) = s.text("id") else { continue };
        if !is_numbered(id, "S-") {
            out.push(problem(
                "fields",
                format!("{}: id {id:?} is not S-<two or more digits>", s.label()),
            ));
        }
        let state = s.text("state").unwrap_or_default().to_owned();
        if ids.insert(id.to_owned(), state).is_some() {
            out.push(problem("fields", format!("step {id} is given twice")));
        }
    }
    ids
}

/// The prerequisites of `step` that are not met: unmerged or undeclared
/// packages by name, and whether an operator item is among them.
fn unmet<'a>(step: &'a Entry, packages: &BTreeMap<String, bool>) -> (Vec<&'a str>, bool) {
    let mut open = Vec::new();
    let mut operator = false;
    for p in step.list("prerequisites") {
        if p.starts_with(OPERATOR) {
            operator = true;
        } else if packages.get(p) != Some(&true) {
            open.push(p.as_str());
        }
    }
    (open, operator)
}

fn fields(step: &Entry, packages: &BTreeMap<String, bool>, out: &mut Vec<Problem>) {
    let label = step.label();
    let state = step.text("state").unwrap_or_default();
    for key in ["title", "writes"] {
        if step.text(key).is_some_and(|t| t.trim().is_empty()) {
            out.push(problem("fields", format!("{label}: {key} is empty")));
        }
    }
    for key in ["command", "dry_run", "prerequisites", "check"] {
        if step.has(key) && step.list(key).iter().all(|l| l.trim().is_empty()) {
            out.push(problem("fields", format!("{label}: {key} names nothing")));
        }
    }
    for p in step.list("prerequisites") {
        if let Some(what) = p.strip_prefix(OPERATOR) {
            if what.trim().is_empty() {
                out.push(problem(
                    "fields",
                    format!("{label}: an operator prerequisite says nothing"),
                ));
            }
        } else if !packages.contains_key(p) {
            out.push(problem(
                "fields",
                format!("{label}: prerequisite {p:?} is neither a declared package nor 'operator: <what>'"),
            ));
        }
    }
    for id in step.list("closes").iter().chain(step.list("helps")) {
        if !is_row_id(id) {
            out.push(problem(
                "fields",
                format!("{label}: {id:?} is not a finding or audit row id"),
            ));
        }
    }
    for id in step.list("helps") {
        if step.list("closes").contains(id) {
            out.push(problem(
                "fields",
                format!("{label}: {id} is both closed and helped"),
            ));
        }
    }
    match state {
        "pending" | "done" => {
            if step.has("closes") && step.list("closes").is_empty() {
                out.push(problem("fields", format!("{label}: closes no row")));
            }
            if step.has("withdrawn_because") || step.has("replaced_by") {
                out.push(problem(
                    "fields",
                    format!("{label}: a {state} step is not withdrawn"),
                ));
            }
        }
        "withdrawn" => {
            if !step.list("closes").is_empty() {
                out.push(problem(
                    "fields",
                    format!("{label}: a withdrawn step closes nothing; its rows go to the step that replaces it"),
                ));
            }
            for key in ["withdrawn_because", "replaced_by"] {
                if !step.has(key) {
                    out.push(problem(
                        "fields",
                        format!("{label}: a withdrawn step needs {key}"),
                    ));
                }
            }
        }
        other => out.push(problem(
            "fields",
            format!("{label}: state {other:?} is not pending, done or withdrawn"),
        )),
    }
    if state == "done" {
        for key in ["ran", "ran_ms"] {
            if !step.has(key) {
                out.push(problem(
                    "fields",
                    format!(
                        "{label}: a step is closed only by its run, and a done step needs {key}"
                    ),
                ));
            }
        }
    } else if step.has("ran") || step.has("ran_ms") {
        out.push(problem(
            "fields",
            format!("{label}: only a done step carries ran and ran_ms"),
        ));
    }
    // A run that did not close the step is recorded on the pending step as
    // `last_run`, never as `ran`: a step is closed only by its run.
    if step.has("last_run") && state != "pending" {
        out.push(problem(
            "fields",
            format!("{label}: only a pending step carries last_run, the run that did not close it"),
        ));
    }
    if step.has("ran_out_of_order") && state != "done" {
        out.push(problem(
            "fields",
            format!("{label}: only a done step carries ran_out_of_order"),
        ));
    }
    for key in ["ran", "last_run", "ran_out_of_order"] {
        if step.text(key).is_some_and(|t| t.trim().is_empty()) {
            out.push(problem("fields", format!("{label}: {key} is empty")));
        }
    }
    if state == "done" && step.number("ran_ms") == Some(0) {
        out.push(problem(
            "fields",
            format!("{label}: ran_ms is 0: a run that took no time is no record of a run"),
        ));
    }
    // A pending step's check is read before its run and after it: one that
    // reads a page records what the page answers today, so a check that
    // already passes is seen as one before anyone trusts it.
    if state == "pending" {
        for line in step.list("check") {
            if reads_a_page(line) && !records_today(line) {
                out.push(problem(
                    "fields",
                    format!("{label}: check {line:?} reads a page and does not record what it reads today"),
                ));
            }
        }
    }
    if state == "pending" && !step.has("disk") {
        out.push(problem(
            "fields",
            format!("{label}: a pending step needs disk, its projection of the store's growth"),
        ));
    }
}

/// What may open a word before the page it names: a parenthesis, a bracket,
/// a brace, an angle bracket, a quote or a backtick (`(/db`, `` `/db` ``,
/// `[GET`).
const OPENERS: [char; 7] = ['(', '[', '{', '<', '"', '\'', '`'];

/// Whether a check line reads one of the app's pages: a `GET` (alone, or
/// joined to its path as `GET:/x` or `get/x`), a `curl` or a `wget`, a URL,
/// or a path of the app's own (`/db`, `/store.json`), any of them opened by
/// [`OPENERS`] (verify-R3-36-r1, b09: a check written as curl was not seen;
/// verify-R3-36-r2: a path inside parentheses, backticks or brackets, and
/// `GET:/x` and wget, were not seen).
///
/// The rule is syntactic, and two limits are kept on purpose. A path is the
/// app's when a lowercase letter follows its `/`: every route literal in
/// `crates/api/src/server.rs` begins that way (119 of them on 15 Oct, none
/// uppercase), and an uppercase one (`/Volumes/WD_BLACK`) is a file path
/// that a `tmutil` or `ls` check names, not a page. A word that opens with a
/// letter before its `/` (`x(/db`) is not split.
fn reads_a_page(line: &str) -> bool {
    line.split_whitespace().any(|w| {
        let w = w.trim_start_matches(OPENERS);
        let lower = w.to_ascii_lowercase();
        let bare = lower.trim_end_matches(|c: char| !c.is_ascii_alphanumeric());
        w == "GET"
            || lower
                .strip_prefix("get")
                .is_some_and(|rest| rest.starts_with([':', '/']))
            || bare == "curl"
            || bare == "wget"
            || lower.starts_with("http://")
            || lower.starts_with("https://")
            || w.strip_prefix('/')
                .is_some_and(|p| p.starts_with(|c: char| c.is_ascii_lowercase()))
    })
}

/// Whether `word` is a value a page answered: it holds a digit, it is `[]`,
/// `true` or `false`, or it is `key:value` with both sides written. Only the
/// parenthesis that opens `(0 today)` is trimmed: a quoted value ends in its
/// quote too, so trimming the opening one alone would read nothing more.
fn is_value(word: &str) -> bool {
    let w = word.trim_start_matches('(');
    w.chars().any(|c| c.is_ascii_digit())
        || ["[]", "true", "false"].contains(&w)
        || w.split_once(':')
            .is_some_and(|(k, v)| !k.is_empty() && !v.is_empty())
}

/// Whether `line` records the value its page answers today: the word
/// `today` stands right after a value in the same parentheses (`(0
/// today)`, `(recorded:false today)`), or `today:` right before one
/// (`today: 0`). The word alone is no record (verify-R3-36-r1, b08: `(not
/// read today)` passed), and neither is the check's own target before it
/// (`total above 0 (today)`).
fn records_today(line: &str) -> bool {
    let words: Vec<&str> = line.split_whitespace().collect();
    words.windows(2).any(|pair| {
        let bare = |w: &str| {
            w.trim_matches(|c: char| !c.is_ascii_alphanumeric())
                .to_owned()
        };
        (bare(pair[1]) == "today" && !pair[1].starts_with('(') && is_value(pair[0]))
            || (pair[0].trim_start_matches('(') == "today:" && is_value(pair[1]))
    })
}

fn rehearsal(step: &Entry, packages: &BTreeMap<String, bool>, out: &mut Vec<Problem>) {
    let label = step.label();
    let held = FIGURES.iter().filter(|k| step.has(k)).count();
    let figures = held == FIGURES.len();
    if held != 0 && !figures {
        out.push(problem(
            "rehearsal",
            format!(
                "{label}: rehearsal figures come together: {}",
                FIGURES.join(", ")
            ),
        ));
    }
    // Figures come from a dry run that ran: a `planned:` one names a form no
    // package has built, so nothing timed it (verify-R3-36-r1, b07).
    if figures
        && step
            .list("dry_run")
            .iter()
            .any(|l| l.starts_with("planned: "))
    {
        out.push(problem(
            "rehearsal",
            format!("{label}: rehearsal figures beside a planned dry run: nothing has run a planned dry run, so nothing timed it"),
        ));
    }
    // A dry run that ran took time: 0 ms is a figure nobody measured
    // (verify-R3-36-r3).
    if step.number("rehearsed_ms") == Some(0) {
        out.push(problem(
            "rehearsal",
            format!("{label}: rehearsed_ms = 0: write the time the dry run took"),
        ));
    }
    if step.has("rehearsed_files") && !figures {
        out.push(problem(
            "rehearsal",
            format!("{label}: rehearsed_files without the figures"),
        ));
    }
    if step.text("state") != Some("pending") {
        return;
    }
    let (open, operator) = unmet(step, packages);
    let waits = step.text("not_rehearsed");
    if figures && waits.is_some() {
        out.push(problem(
            "rehearsal",
            format!("{label}: figures and not_rehearsed at once"),
        ));
    }
    if figures && !open.is_empty() {
        out.push(problem(
            "rehearsal",
            format!(
                "{label}: figures beside an unmerged prerequisite ({}): a build without it did not rehearse this step",
                open.join(", ")
            ),
        ));
    }
    if figures {
        return;
    }
    let Some(waits) = waits else {
        out.push(problem(
            "rehearsal",
            if open.is_empty() && !operator {
                format!(
                    "{label}: every prerequisite is merged: rehearse it on a private copy and write rehearsed_ms, rehearsed_bytes, rehearsed_on and rehearsed_result"
                )
            } else {
                format!("{label}: no rehearsal figures and no not_rehearsed naming what it waits for")
            },
        ));
        return;
    };
    let Some(named) = waits.strip_prefix("waits for ") else {
        out.push(problem(
            "rehearsal",
            format!(
                "{label}: not_rehearsed must begin 'waits for <package>' or 'waits for operator'"
            ),
        ));
        return;
    };
    let name = named.split([':', ' ', ',', ';']).next().unwrap_or_default();
    let fits = if name == "operator" {
        operator
    } else {
        open.contains(&name)
    };
    if !fits {
        out.push(problem(
            "rehearsal",
            if name == "operator" {
                format!("{label}: waits for operator, but no prerequisite is 'operator: <what>'")
            } else if packages.get(name) == Some(&true) {
                format!("{label}: waits for {name}, which is merged: rehearse it")
            } else {
                format!(
                    "{label}: waits for {name:?}, which is not an unmet prerequisite of this step"
                )
            },
        ));
    }
}

/// `line` with each `<NAME>` placeholder (no space and no `<` inside) read
/// as the word `NAME`: the operator fills it, so its `|` (`<INDEX|CASH>`)
/// and its brackets are not the shell's. A `<` that opens no placeholder is
/// kept as written.
fn outside_placeholders(line: &str) -> String {
    let mut parts = line.split('<');
    let mut out = parts.next().unwrap_or_default().to_owned();
    for part in parts {
        match part.split_once('>') {
            Some((name, rest)) if !name.is_empty() && !name.contains(char::is_whitespace) => {
                out.push_str("NAME");
                out.push_str(rest);
            }
            _ => {
                out.push('<');
                out.push_str(part);
            }
        }
    }
    out
}

/// The name after each `$` in `line` (`$NAME` or `${NAME}`): the letters,
/// digits and `_` that follow, empty for a bare `$`.
fn variables(line: &str) -> Vec<&str> {
    line.match_indices('$')
        .map(|(at, _)| {
            let name = line[at + 1..].trim_start_matches('{');
            let end = name
                .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                .unwrap_or(name.len());
            &name[..end]
        })
        .collect()
}

fn commands(
    step: &Entry,
    packages: &BTreeMap<String, bool>,
    evidence: &Evidence,
    out: &mut Vec<Problem>,
) {
    let label = step.label();
    let (open, operator) = unmet(step, packages);
    let met = open.is_empty() && !operator;
    for line in step.list("command") {
        if !HEADS.iter().any(|h| line.starts_with(h)) {
            out.push(problem(
                "commands",
                format!(
                    "{label}: command {line:?} begins with none of {}",
                    HEADS.join("| ")
                ),
            ));
        }
        // A done step ran its command, so the command is one a package
        // built: a planned line there names a run that cannot have happened.
        // A command runs as written: nothing chained after it, no variable
        // the step does not declare, and no rsync option that removes a file
        // (verify-R3-36-r3). A `POST` line is a form's body, whose `&` joins
        // fields, and a `planned:` line names no command yet.
        if line.starts_with("cli ") || COPIES.iter().any(|h| line.starts_with(h)) {
            // A single-quoted word is the program's own (`'GVT&D'`): the
            // shell hands it over whole, so its `&` chains nothing (R4-06).
            let bare = outside_quotes(&outside_placeholders(line));
            if let Some(c) = CHAINS.iter().find(|c| bare.contains(*c)) {
                out.push(problem(
                    "commands",
                    format!(
                        "{label}: command {line:?}: {c:?} chains or redirects a second command"
                    ),
                ));
            }
            for name in variables(&bare) {
                if !step.list("env").iter().any(|e| e == name) {
                    out.push(problem(
                        "commands",
                        format!("{label}: command {line:?} uses ${name}, which the step's env does not declare"),
                    ));
                }
            }
        }
        if let Some(o) = line.strip_prefix("rsync ").and_then(|rest| {
            rest.split_whitespace()
                .take_while(|w| *w != "--")
                .find(|w| RSYNC_REMOVES.iter().any(|r| w.starts_with(r)))
        }) {
            out.push(problem(
                "commands",
                format!("{label}: command {line:?}: rsync {o} removes files, and a copy of an append-only store removes none"),
            ));
        }
        if line.starts_with("planned: ") && step.text("state") == Some("done") {
            out.push(problem(
                "commands",
                format!("{label}: a done step ran its command, but {line:?} is still planned: write the command that ran"),
            ));
        }
        if line.starts_with("planned: ") && step.text("state") == Some("pending") && met {
            out.push(problem(
                "commands",
                format!("{label}: every prerequisite is met, but the command is still planned: write the real one"),
            ));
        }
        if let (Some(verb), Some(writers)) = (line.strip_prefix("cli "), &evidence.writers) {
            let verb = verb.split_whitespace().next().unwrap_or_default();
            if !writers.verbs.contains(verb) {
                out.push(problem(
                    "commands",
                    format!("{label}: cli {verb} is not one of the cli's store writers"),
                ));
            }
        }
        if step.list("dry_run").contains(line) {
            out.push(problem(
                "commands",
                format!("{label}: the dry run {line:?} is the command itself: on the real store that is the run"),
            ));
        }
    }
    for line in step.list("dry_run") {
        if let Some(why) = dry_run_writes(line, evidence.writers.as_ref()) {
            out.push(problem(
                "commands",
                format!("{label}: dry run {line:?} could write the operator's store: {why}"),
            ));
        }
    }
    for name in step.list("env") {
        if !name
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
            || name.is_empty()
        {
            out.push(problem(
                "commands",
                format!("{label}: env {name:?} is not a variable name"),
            ));
        }
    }
}

fn order(steps: &[&Entry], ids: &BTreeMap<String, String>, out: &mut Vec<Problem>) {
    let mut next: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for step in steps {
        let Some(id) = step.text("id") else { continue };
        let state = step.text("state").unwrap_or_default();
        for before in step.list("after") {
            match ids.get(before) {
                None => out.push(problem("after", format!("step {id}: after {before}, which is no step"))),
                Some(_) if before == id => out.push(problem("after", format!("step {id} is after itself"))),
                Some(s) if s == "withdrawn" => out.push(problem(
                    "after",
                    format!("step {id}: after {before}, which is withdrawn: name the step that replaces it"),
                )),
                Some(s)
                    if state == "done"
                        && s != "done"
                        && !step
                            .text("ran_out_of_order")
                            .is_some_and(|why| says_ran_before(why, before)) =>
                {
                    out.push(problem(
                        "after",
                        format!(
                            "step {id} is done, but {before}, which it follows, is not: if it really ran first, say so in ran_out_of_order, naming {before} and what that left"
                        ),
                    ));
                    next.entry(id).or_default().push(before.as_str());
                }
                Some(_) => next.entry(id).or_default().push(before.as_str()),
            }
        }
        if let Some(by) = step.text("replaced_by") {
            match ids.get(by) {
                None => out.push(problem(
                    "after",
                    format!("step {id}: replaced by {by}, which is no step"),
                )),
                Some(s) if s == "withdrawn" => out.push(problem(
                    "after",
                    format!("step {id}: replaced by {by}, which is withdrawn too"),
                )),
                Some(_) => {}
            }
        }
    }
    // A cycle: walk every step's predecessors, three colours.
    let mut colour: BTreeMap<&str, u8> = BTreeMap::new();
    for start in next.keys() {
        // Between walks every step reached is black, and so is every step it
        // follows, so walking a black start again reports nothing: this skip
        // saves work and changes no answer. Its `if false` mutant is
        // equivalent and no test can tell it apart (r3 and r4 sweeps).
        if colour.get(start).copied().unwrap_or(0) != 0 {
            continue;
        }
        let mut stack: Vec<(&str, usize)> = vec![(start, 0)];
        colour.insert(start, 1);
        while let Some((node, i)) = stack.pop() {
            let succ = next.get(node).map_or(&[][..], Vec::as_slice);
            if let Some(&n) = succ.get(i) {
                stack.push((node, i + 1));
                match colour.get(n).copied().unwrap_or(0) {
                    0 => {
                        colour.insert(n, 1);
                        stack.push((n, 0));
                    }
                    1 => out.push(problem(
                        "after",
                        format!("steps {node} and {n} are each after the other"),
                    )),
                    _ => {}
                }
            } else {
                colour.insert(node, 2);
            }
        }
    }
}

/// A step that copies or snapshots the store (FM-221): the copy is taken with
/// every writer stopped, and its check reads the restored store back (census
/// recount and block scrub) before any writer starts. A copy taken under a
/// writer is refused here, by the list, because no verb takes every census
/// lock for it (NOT DONE in code: named in the report's notes).
fn copies(step: &Entry, out: &mut Vec<Problem>) {
    if step.text("state") == Some("withdrawn")
        || !step
            .list("command")
            .iter()
            .any(|c| COPIES.iter().any(|h| c.starts_with(h)))
    {
        return;
    }
    let label = step.label();
    if !step.list("prerequisites").iter().any(|p| p == STOPPED) {
        out.push(problem(
            "copies",
            format!("{label}: a copy or snapshot of the store taken while a writer appends is torn: name the prerequisite {STOPPED:?}"),
        ));
    }
    for head in RESTORE_CHECKS {
        if !step.list("check").iter().any(|c| c.starts_with(head)) {
            out.push(problem(
                "copies",
                format!("{label}: a restore is read back before any writer starts: name a check line beginning {:?}", head.trim_end()),
            ));
        }
    }
}

/// Whether `word` is a `YYYY-MM-DD` day as the cli spells one: four, two
/// and two ASCII digits joined by `-`. Days written so compare as text in
/// the order they fall.
fn is_day(word: &str) -> bool {
    let b = word.as_bytes();
    b.len() == 10
        && b.iter().enumerate().all(|(i, c)| {
            if i == 4 || i == 7 {
                *c == b'-'
            } else {
                c.is_ascii_digit()
            }
        })
}

/// The days D-3317 item 5 says a GDFL recount's span must leave out, read
/// from the decision ledger so the gate keeps no copy of them: every
/// `YYYY-MM-DD` of the item, from its `5. ` line to the first line that is
/// not indented under it, in the entry that begins `### D-3317 `. Refused
/// when the entry, the item or any day in it is missing, so a ledger that
/// moved the item cannot let every span pass.
fn excluded_from(ledger: &str) -> Result<Vec<String>, String> {
    let mut lines = ledger
        .lines()
        .skip_while(|l| !l.starts_with(EXCLUDING_ENTRY));
    if lines.next().is_none() {
        return Err(format!(
            "the decision ledger holds no entry beginning {EXCLUDING_ENTRY:?}"
        ));
    }
    let mut item = lines
        .take_while(|l| !l.starts_with("#"))
        .skip_while(|l| !l.starts_with(EXCLUDING_ITEM))
        .peekable();
    let Some(first) = item.next() else {
        return Err(format!(
            "{} holds no item beginning {EXCLUDING_ITEM:?}",
            EXCLUDING_ENTRY.trim_end()
        ));
    };
    let text: Vec<&str> = std::iter::once(first)
        .chain(item.take_while(|l| l.starts_with(' ')))
        .collect();
    let days: Vec<String> = text
        .iter()
        .flat_map(|l| l.split(|c: char| !(c.is_ascii_digit() || c == '-')))
        .filter(|w| is_day(w))
        .map(str::to_owned)
        .collect();
    if days.is_empty() {
        return Err(format!(
            "item {EXCLUDING_ITEM:?} of {} names no YYYY-MM-DD day",
            EXCLUDING_ENTRY.trim_end()
        ));
    }
    Ok(days)
}

/// One `cli census-recount` a line names: its vendor word and its `--from`
/// and `--to` days, each `None` when the words do not give it.
#[derive(Debug, PartialEq, Eq)]
struct Span<'a> {
    vendor: &'a str,
    from: Option<&'a str>,
    to: Option<&'a str>,
}

/// The span of the recount whose words follow [`RECOUNT`] in `rest`: the
/// vendor, then `--from DAY` and `--to DAY` in any order, as the cli reads
/// them. The first word that is neither ends the command, and so does a day
/// written with prose punctuation after it ([`DAY_ENDS`]). A value that is
/// not a day, or a flag given twice, is why the cli would refuse it.
fn span_of(rest: &str) -> Result<Span<'_>, String> {
    let mut words = rest.split_whitespace();
    let vendor = words.next().unwrap_or_default().trim_end_matches(DAY_ENDS);
    let mut span = Span {
        vendor,
        from: None,
        to: None,
    };
    while let Some(flag) = words.next() {
        let slot = match flag {
            "--from" => &mut span.from,
            "--to" => &mut span.to,
            _ => break,
        };
        let value = words.next().unwrap_or_default();
        let day = value.trim_end_matches(DAY_ENDS);
        if !is_day(day) {
            return Err(format!("{flag} {value:?} is not a YYYY-MM-DD day"));
        }
        if slot.replace(day).is_some() {
            return Err(format!("{flag} is given twice"));
        }
        if day.len() < value.len() {
            break;
        }
    }
    Ok(span)
}

/// Every census recount a step names, in its commands, dry runs and checks,
/// against D-3317 item 5 (verify-R3-36-r3). The recount closes as
/// `recounted` every day of its span whose last journal line is a stageless
/// `incomplete`, and the two vendor-refused days of November 2018 end in
/// one, so a span over either marks it done and no import retries its
/// refused files: a failure hidden. Refused: a recount without `--from` (the
/// cli refuses it, and a span with no start closes every day before its
/// end), a `--to` before its `--from`, a GDFL span that covers an excluded
/// day, and a GDFL span when the ledger was not read (NOT CHECKED is not a
/// pass). A withdrawn step runs nothing, so it is not read.
fn recounts(step: &Entry, evidence: &Evidence, out: &mut Vec<Problem>) {
    if step.text("state") == Some("withdrawn") {
        return;
    }
    let label = step.label();
    for key in ["command", "dry_run", "check"] {
        for line in step.list(key) {
            for (at, _) in line.match_indices(RECOUNT) {
                let mut refuse = |why: String| {
                    out.push(problem(
                        "recount",
                        format!("{label}: {key} {line:?}: {why}"),
                    ));
                };
                let span = match span_of(&line[at + RECOUNT.len()..]) {
                    Ok(span) => span,
                    Err(why) => {
                        refuse(why);
                        continue;
                    }
                };
                let Some(from) = span.from else {
                    refuse("a recount with no --from is refused by the cli, and a span with no start closes every refused day before its end (D-3317)".to_owned());
                    continue;
                };
                if let Some(to) = span.to.filter(|to| *to < from) {
                    refuse(format!("--to {to} is before --from {from}"));
                    continue;
                }
                if span.vendor != JOURNALED {
                    continue;
                }
                let Some(excluded) = &evidence.excluded else {
                    refuse("the days D-3317 item 5 excludes were not read (give --decisions docs/05-decisions.md), so this span is NOT CHECKED".to_owned());
                    continue;
                };
                for day in excluded {
                    if from <= day.as_str() && span.to.is_none_or(|to| day.as_str() <= to) {
                        refuse(format!(
                            "the span covers {day}, which D-3317 item 5 excludes: the recount would close it as recounted and no import would retry its refused files"
                        ));
                    }
                }
            }
        }
    }
}

/// A check line that runs a store writer says so ([`CHECK_WRITES`]): a check
/// is read as a read, and one that writes the census or the journal is the
/// first writer after whatever it checks (verify-R3-36-r3). Decided by the
/// cli's own rule, over the line's words with prose punctuation trimmed;
/// without the cli's table no verb is known and the report says so.
fn check_writes(step: &Entry, evidence: &Evidence, out: &mut Vec<Problem>) {
    let Some(writers) = &evidence.writers else {
        return;
    };
    for line in step.list("check") {
        let Some(rest) = line.strip_prefix("cli ") else {
            continue;
        };
        let words: Vec<&str> = rest
            .split_whitespace()
            .map(|w| w.trim_end_matches(DAY_ENDS))
            .collect();
        if let [verb, arguments @ ..] = words.as_slice()
            && writers.writes(verb, arguments)
            && !line.contains(CHECK_WRITES)
        {
            out.push(problem(
                "checks",
                format!("{}: check {line:?} runs cli {verb}, which writes the store with these words: say {CHECK_WRITES:?} and what it writes, so it is run as the write it is", step.label()),
            ));
        }
    }
}

fn leftovers(entries: &[Entry], steps: &[&Entry], evidence: &Evidence, out: &mut Vec<Problem>) {
    let listed = entries[0].list("leftovers");
    let mut closers: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for step in steps {
        if step.text("state") == Some("withdrawn") {
            continue;
        }
        let id = step.text("id").unwrap_or("?");
        for row in step.list("closes") {
            closers.entry(row).or_default().push(id);
        }
    }
    let mut seen = BTreeSet::new();
    for row in listed {
        if !seen.insert(row.as_str()) {
            out.push(problem(
                "leftovers",
                format!("leftover {row} is listed twice"),
            ));
        }
        if !is_row_id(row) {
            out.push(problem(
                "leftovers",
                format!("leftover {row:?} is not a finding or audit row id"),
            ));
        }
        match closers.get(row.as_str()).map_or(0, Vec::len) {
            1 => {}
            0 => out.push(problem(
                "leftovers",
                format!("leftover {row} waits for a store run that no step names"),
            )),
            _ => out.push(problem(
                "leftovers",
                format!(
                    "leftover {row} is closed by {} steps at once: {}",
                    closers[row.as_str()].len(),
                    closers[row.as_str()].join(", ")
                ),
            )),
        }
    }
    for (row, by) in &closers {
        if !seen.contains(row) {
            out.push(problem(
                "leftovers",
                format!(
                    "step {} closes {row}, which the list of leftovers does not hold",
                    by.join(", ")
                ),
            ));
        }
    }
    for row in &evidence.marked {
        if !seen.contains(row.as_str()) {
            out.push(problem(
                "leftovers",
                format!("{row} is marked as needing a store write, and the list of leftovers does not hold it"),
            ));
        }
    }
}

fn rules(entries: &[Entry], packages: &BTreeMap<String, bool>, out: &mut Vec<Problem>) {
    let mut ids = BTreeSet::new();
    for rule in of(entries, "rule") {
        let label = rule.label();
        if let Some(id) = rule.text("id") {
            if !is_numbered(id, "R-") {
                out.push(problem(
                    "rules",
                    format!("{label}: id is not R-<two or more digits>"),
                ));
            }
            if !ids.insert(id.to_owned()) {
                out.push(problem("rules", format!("rule {id} is given twice")));
            }
        }
        if rule.has("source") && rule.list("source").is_empty() {
            out.push(problem("rules", format!("{label}: names no source row")));
        }
        for s in rule.list("source") {
            if !is_row_id(s) {
                out.push(problem(
                    "rules",
                    format!("{label}: source {s:?} is not a row or decision id"),
                ));
            }
        }
        match rule.text("until") {
            None | Some("never") => {}
            Some(p) => match packages.get(p) {
                None => out.push(problem(
                    "rules",
                    format!("{label}: until {p:?}, which is no declared package"),
                )),
                Some(true) if !rule.has("retired") => out.push(problem(
                    "rules",
                    format!(
                        "{label} lasts until {p}, which is merged: retire it (retired = \"<why>\")"
                    ),
                )),
                Some(_) => {}
            },
        }
    }
}

/// Every declared package and variable is used, and every variable a step
/// names is declared.
fn uses(entries: &[Entry], steps: &[&Entry], out: &mut Vec<Problem>) {
    let mut named: BTreeSet<&str> = steps
        .iter()
        .flat_map(|s| s.list("prerequisites"))
        .map(String::as_str)
        .collect();
    named.extend(of(entries, "rule").iter().filter_map(|r| r.text("until")));
    for p in of(entries, "package") {
        if let Some(name) = p.text("name")
            && !named.contains(name)
        {
            out.push(problem(
                "packages",
                format!("package {name} is named by no step and no rule"),
            ));
        }
    }
    let mut times: BTreeMap<&str, usize> = BTreeMap::new();
    for name in of(entries, "env").iter().filter_map(|e| e.text("name")) {
        *times.entry(name).or_default() += 1;
    }
    for (name, n) in &times {
        if *n > 1 {
            out.push(problem(
                "env",
                format!("variable {name} is declared {n} times"),
            ));
        }
    }
    let declared: BTreeSet<&str> = times.keys().copied().collect();
    let used: BTreeSet<&str> = steps
        .iter()
        .flat_map(|s| s.list("env"))
        .map(String::as_str)
        .collect();
    for name in &used {
        if !declared.contains(name) {
            out.push(problem(
                "env",
                format!("variable {name} is named by a step and declared by no [[env]]"),
            ));
        }
    }
    for name in &declared {
        if !used.contains(name) {
            out.push(problem(
                "env",
                format!("variable {name} is declared and named by no step"),
            ));
        }
    }
}

// --------------------------------------------------------------- evidence --

/// The packages `Merge <name>: ...` subjects name.
fn merged_from(subjects: &str) -> BTreeSet<String> {
    subjects
        .lines()
        .filter_map(|l| l.strip_prefix("Merge "))
        .filter_map(|rest| rest.split_once(':').map(|(name, _)| name))
        .filter(|name| is_package_name(name))
        .map(str::to_owned)
        .collect()
}

/// The verbs of `WRITERS` in the cli's `store_write.rs`.
fn writers_from(source: &str) -> Result<BTreeSet<String>, String> {
    const HEAD: &str = "pub(crate) const WRITERS: [&str; ";
    let start = source
        .find(HEAD)
        .ok_or_else(|| format!("the writer table ({HEAD}N]) is not in the file read"))?;
    let rest = &source[start + HEAD.len()..];
    let (count, rest) = rest
        .split_once(']')
        .ok_or_else(|| "the writer table's length is not closed".to_owned())?;
    let count: usize = count
        .trim()
        .parse()
        .map_err(|_| format!("the writer table's length {count:?} is not a number"))?;
    let body = rest
        .split_once("];")
        .map(|(b, _)| b)
        .ok_or_else(|| "the writer table is not closed by ];".to_owned())?;
    let verbs: BTreeSet<String> = body
        .split('"')
        .skip(1)
        .step_by(2)
        .map(str::to_owned)
        .collect();
    if verbs.len() != count {
        return Err(format!(
            "the writer table says {count} verbs and holds {} distinct ones",
            verbs.len()
        ));
    }
    Ok(verbs)
}

/// The form one arm of `writes_the_store` gives its verbs, or `None` when the
/// arm is not one of the three shapes [`Form`] reads.
fn form_of(arm: &str) -> Option<Form> {
    let word = |s: &str| -> Option<String> {
        s.strip_prefix('"')?
            .strip_suffix('"')
            .filter(|w| !w.is_empty() && !w.contains(['"', ' ']))
            .map(str::to_owned)
    };
    if let Some(w) = arm
        .strip_prefix("!arguments.contains(&")
        .and_then(|r| r.strip_suffix("),"))
    {
        return word(w).map(Form::Unless);
    }
    if let Some(w) = arm
        .strip_prefix("arguments.contains(&")
        .and_then(|r| r.strip_suffix("),"))
    {
        return word(w).map(Form::With);
    }
    let words = arm
        .strip_prefix("matches!(arguments.first(), Some(&(")?
        .strip_suffix("))),")?;
    words
        .split(" | ")
        .map(word)
        .collect::<Option<Vec<_>>>()
        .map(Form::First)
}

/// The arms of the cli's `writes_the_store`, read from its source. An arm of
/// any other shape is refused by name, so a cli change the gate cannot read
/// stops the gate instead of passing a dry run it no longer understands.
fn forms_from(source: &str, verbs: &BTreeSet<String>) -> Result<BTreeMap<String, Form>, String> {
    const HEAD: &str = "pub(crate) fn writes_the_store(verb: &str, arguments: &[&str]) -> bool {";
    const LAST: &str = "other => WRITERS.contains(&other),";
    let start = source
        .find(HEAD)
        .ok_or_else(|| format!("the cli's write rule ({HEAD}) is not in the file read"))?;
    let mut lines = source[start + HEAD.len()..]
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty());
    if lines.next() != Some("match verb {") {
        return Err("the cli's write rule does not begin with `match verb {`".to_owned());
    }
    let mut forms = BTreeMap::new();
    for line in lines {
        if line == LAST {
            return Ok(forms);
        }
        if line == "}" {
            break;
        }
        let unread = || {
            format!(
                "the cli's write rule has an arm this gate does not read: {line:?}; teach the gate its form"
            )
        };
        let (left, right) = line.split_once(" => ").ok_or_else(unread)?;
        let form = form_of(right).ok_or_else(unread)?;
        for verb in left.split(" | ") {
            let verb = verb
                .strip_prefix('"')
                .and_then(|v| v.strip_suffix('"'))
                .filter(|v| !v.is_empty())
                .ok_or_else(unread)?;
            if !verbs.contains(verb) {
                return Err(format!(
                    "the cli's write rule names {verb}, which is not in WRITERS"
                ));
            }
            if forms.insert(verb.to_owned(), form.clone()).is_some() {
                return Err(format!("the cli's write rule names {verb} twice"));
            }
        }
    }
    Err(format!("the cli's write rule does not end with {LAST:?}"))
}

/// [`writers_from`] and [`forms_from`] over one cli source.
fn table_from(source: &str) -> Result<Writers, String> {
    let verbs = writers_from(source)?;
    let forms = forms_from(source, &verbs)?;
    Ok(Writers { verbs, forms })
}

/// One row id per line; blank lines and `#` comments skipped.
fn marked_from(text: &str) -> Vec<String> {
    text.lines()
        .map(|l| l.split('#').next().unwrap_or_default().trim())
        .filter(|l| !l.is_empty())
        .map(str::to_owned)
        .collect()
}

// ----------------------------------------------------------------- report --

/// The plain table the workflow prints, then each refusal.
fn report(entries: &[Entry], evidence: &Evidence, problems: &[Problem]) -> String {
    let mut s = String::new();
    let steps = of(entries, "step");
    let count = |state: &str| {
        steps
            .iter()
            .filter(|e| e.text("state") == Some(state))
            .count()
    };
    let packages = of(entries, "package");
    let merged = packages
        .iter()
        .filter(|p| p.flag("merged") == Some(true))
        .count();
    let _ = writeln!(
        s,
        "store steps: {} (pending {}, done {}, withdrawn {}); rules {}; packages {} (merged {}); leftovers {}",
        steps.len(),
        count("pending"),
        count("done"),
        count("withdrawn"),
        of(entries, "rule").len(),
        packages.len(),
        merged,
        entries[0].list("leftovers").len()
    );
    let _ = writeln!(s, "| step | state | rehearsal | closes |");
    let _ = writeln!(s, "|---|---|---|---|");
    for step in &steps {
        let rehearsal = match (step.number("rehearsed_ms"), step.number("rehearsed_bytes")) {
            (Some(ms), Some(bytes)) => format!("{ms} ms, {bytes} bytes"),
            _ => step
                .text("not_rehearsed")
                .map(|w| w.split(':').next().unwrap_or(w).to_owned())
                .or_else(|| step.number("ran_ms").map(|ms| format!("ran in {ms} ms")))
                .unwrap_or_else(|| "-".to_owned()),
        };
        let _ = writeln!(
            s,
            "| {} | {} | {} | {} |",
            step.text("id").unwrap_or("?"),
            match (
                step.text("state"),
                step.has("last_run"),
                step.has("ran_out_of_order")
            ) {
                (Some("pending"), true, _) => "pending, partly run".to_owned(),
                (Some("done"), _, true) => "done, out of order".to_owned(),
                (state, _, _) => state.unwrap_or("?").to_owned(),
            },
            rehearsal,
            step.list("closes").join(" ")
        );
    }
    let planned = steps
        .iter()
        .filter(|e| e.list("command").iter().any(|c| c.starts_with("planned: ")))
        .count();
    let _ = writeln!(
        s,
        "steps whose command a package has not built yet: {planned}"
    );
    for rule in [
        "fields",
        "leftovers",
        "rehearsal",
        "copies",
        "recount",
        "forms",
        "records",
        "kinds",
        "owed",
    ] {
        let n = problems.iter().filter(|p| p.rule == rule).count();
        let _ = writeln!(s, "{rule} refusals: {n}");
    }
    match &evidence.merged {
        Some(m) if m.is_empty() => {
            let _ = writeln!(
                s,
                "merge history: names no package merge (a squash-merged history): the merged flags stand as written, NOT CHECKED against git"
            );
        }
        Some(m) => {
            let _ = writeln!(
                s,
                "merge history: {} package merges read; merged flags checked",
                m.len()
            );
        }
        None => {
            let _ = writeln!(s, "merge history: not given; merged flags NOT CHECKED");
        }
    }
    if evidence.writers.is_none() {
        let _ = writeln!(
            s,
            "cli writer table: not given; cli verbs and the writes of check lines NOT CHECKED, and a cli dry run passes only on a private copy"
        );
    }
    match &evidence.excluded {
        Some(days) => {
            let _ = writeln!(
                s,
                "decision ledger: D-3317 item 5 read; no census-recount {JOURNALED} span may cover {}",
                days.join(", ")
            );
        }
        None => {
            let _ = writeln!(
                s,
                "decision ledger: not given; every census-recount {JOURNALED} span is refused as NOT CHECKED"
            );
        }
    }
    // CI has no board to read, so it runs without --marked; the report says
    // which of the two checks ran (verify-R3-36-r2, followup).
    if evidence.marked.is_empty() {
        let _ = writeln!(
            s,
            "rows marked as needing a store write: none given; board rows outside the leftovers NOT CHECKED"
        );
    } else {
        let _ = writeln!(
            s,
            "rows marked as needing a store write: {} read; each is checked to be a leftover",
            evidence.marked.len()
        );
    }
    match &evidence.owed {
        Some(ids) => {
            let _ = writeln!(
                s,
                "owed runs: {} read from config/owed-runs.toml; each is a leftover exactly one step closes",
                ids.len()
            );
        }
        None => {
            let _ = writeln!(s, "owed runs: not given; NOT CHECKED");
        }
    }
    for p in problems {
        let _ = writeln!(s, "REFUSED [{}] {}", p.rule, p.what);
    }
    s
}

/// The list read and checked against `evidence`: the report, and whether it
/// passed.
fn run(text: &str, evidence: &Evidence) -> (String, bool) {
    match read(text) {
        Err(why) => (format!("REFUSED [reader] {why}\n"), false),
        Ok(entries) => {
            let problems = check(&entries, evidence);
            (report(&entries, evidence, &problems), problems.is_empty())
        }
    }
}

const USAGE: &str = "usage: gates_store_steps check LIST [--merges SUBJECTS] [--writers STORE_WRITE_RS] [--decisions LEDGER] [--marked IDS] [--owed OWED_RUNS]";

/// Each evidence flag with the file it names.
type Flags<'a> = Vec<(&'a str, &'a str)>;

/// The list's path and each evidence flag with the file it names, refused
/// before any file is opened unless every word is one the tool reads.
fn parse_arguments(args: &[String]) -> Result<(&str, Flags<'_>), String> {
    let [verb, list, rest @ ..] = args else {
        return Err(USAGE.to_owned());
    };
    if verb != "check" || rest.len() % 2 != 0 {
        return Err(USAGE.to_owned());
    }
    let mut flags = Vec::new();
    let mut seen = BTreeSet::new();
    for pair in rest.chunks(2) {
        let flag = pair[0].as_str();
        if !["--merges", "--writers", "--decisions", "--marked", "--owed"].contains(&flag)
            || !seen.insert(flag)
        {
            return Err(USAGE.to_owned());
        }
        flags.push((flag, pair[1].as_str()));
    }
    Ok((list, flags))
}

/// How the tool reads a file it is named: whole, as text, or why not.
type ReadFile<'a> = &'a dyn Fn(&str) -> Result<String, String>;

/// The file at `path`, read from disk.
fn from_disk(path: &str) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))
}

fn arguments(args: &[String], read_file: ReadFile<'_>) -> Result<(String, Evidence), String> {
    let (list, flags) = parse_arguments(args)?;
    let mut evidence = Evidence::default();
    for (flag, path) in flags {
        let text = read_file(path)?;
        match flag {
            "--merges" => evidence.merged = Some(merged_from(&text)),
            "--writers" => evidence.writers = Some(table_from(&text)?),
            "--decisions" => evidence.excluded = Some(excluded_from(&text)?),
            "--owed" => evidence.owed = Some(owed_from(&text)?),
            _ => evidence.marked = marked_from(&text),
        }
    }
    Ok((read_file(list)?, evidence))
}

/// The whole tool over `args`, reading each file it is named with
/// `read_file`: what it prints, what it prints as an error, and its exit
/// code (0 when the list passes, 1 when it is refused, 2 when it was not
/// checked). `main` only prints these, so every decision is under test.
fn gate(args: &[String], read_file: ReadFile<'_>) -> (String, String, u8) {
    match arguments(args, read_file) {
        Err(why) => (String::new(), format!("gates_store_steps: {why}\n"), 2),
        Ok((text, evidence)) => {
            let (mut report, passed) = run(&text, &evidence);
            if passed {
                report.push_str(
                    "OK — every store step names its command, prerequisite, check and rows.\n",
                );
                (report, String::new(), 0)
            } else {
                report.push_str("REFUSED — fix config/store-steps.toml (R3-36).\n");
                (report, String::new(), 1)
            }
        }
    }
}

/// Writes what [`gate`] gave to `stdout` and `stderr`, and returns the exit
/// code: `gate`'s own when both reached their reader, 2 when either could
/// not be written. A report that did not reach CI was not given, so its run
/// cannot count as one that passed; the reason goes to `stderr` when only
/// `stdout` failed. `main` only joins this to the process, so no decision
/// it makes is outside a test (verify-R3-36-r2: a mutant of the old
/// `print!`/`eprint!` lines in `main` survived, because no test runs
/// `main`).
///
/// LIMIT (verify-R3-36-r3, followup): a stdout CLOSED by the caller
/// (`gss check ... >&-`) exits 0, and no code in this tool can tell. Before
/// `main` runs, std's runtime opens `/dev/null` on every standard descriptor
/// it finds closed, so the report is written to `/dev/null` and every write
/// succeeds. Measured on this machine (rustc 1.97.1, macOS), 15 Oct: with
/// `>&-`, descriptor 1 is a character device whose `rdev` equals
/// `/dev/null`'s, and the tool exits 0, exactly as with `>/dev/null`. The two
/// are the same descriptor to the process; the workflow keeps its stdout.
fn emit(
    (out, err, code): (String, String, u8),
    stdout: &mut dyn std::io::Write,
    stderr: &mut dyn std::io::Write,
) -> u8 {
    if let Err(e) = stdout
        .write_all(out.as_bytes())
        .and_then(|()| stdout.flush())
    {
        let _ = stderr.write_all(
            format!("{err}gates_store_steps: the report was not written: {e}\n").as_bytes(),
        );
        return 2;
    }
    match stderr.write_all(err.as_bytes()) {
        Ok(()) => code,
        Err(_) => 2,
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    ExitCode::from(emit(
        gate(&args, &from_disk),
        &mut std::io::stdout(),
        &mut std::io::stderr(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The list in this repository, read the way the workflow reads it.
    const REAL: &str = include_str!("../config/store-steps.toml");
    /// The cli source whose `WRITERS` table names the store-writing verbs.
    const CLI_WRITERS: &str = include_str!("../crates/cli/src/store_write.rs");
    /// The decision ledger whose D-3317 item 5 names the days no GDFL
    /// recount span may cover.
    const LEDGER: &str = include_str!("../docs/05-decisions.md");

    /// A small list that passes: one step ready and rehearsed, one waiting.
    const GOOD: &str = r#"
format = 1
leftovers = ["DB-A01", "DB-A02"] # rows waiting for a store run

[[env]]
name = "BRUTEX_STORE"
means = "the store the step writes"

[[package]]
name = "WP-01"
merged = true
title = "a merged package"

[[package]]
name = "WP-02"
merged = false
title = "an unmerged package"

[[rule]]
id = "R-01"
text = "one command-line run at a time"
source = ["RA-118"]
until = "WP-02"

[[step]]
id = "S-01"
title = "a ready step"
state = "pending"
writes = "new month generations"
command = ["cli gdfl-import options zip 2018-11-07 2018-11-07 NIFTY BANKNIFTY"]
dry_run = ["cli gdfl-import options zip 2018-11-07 2018-11-07 NIFTY BANKNIFTY with BRUTEX_STORE on a private copy"]
env = ["BRUTEX_STORE"]
prerequisites = ["WP-01"]
check = ["GET /bars/window.json -> total above 0 (0 today)"]
closes = ["DB-A01"]
rehearsed_ms = 1_200
rehearsed_bytes = 4096
rehearsed_on = "a private copy"
rehearsed_result = "1 day imported"
disk = "4 KiB"

[[step]]
id = "S-02"
title = "a waiting step"
state = "pending"
writes = "new month generations"
command = [
  "planned: cli gdfl-restate options 2018-11-07 2018-11-07", # one line per command
]
dry_run = ["the same, with BRUTEX_STORE on a private copy"]
prerequisites = ["WP-02"]
after = ["S-01"]
check = ["GET /x (0 today)"]
closes = ["DB-A02"]
not_rehearsed = "waits for WP-02: the door is not built"
disk = "unknown until rehearsed"
"#;

    /// The evidence CI hands the tool: the cli's writer table and the days
    /// the decision ledger excludes from a GDFL recount.
    fn writers() -> Evidence {
        Evidence {
            writers: Some(table_from(CLI_WRITERS).expect("the cli's writer table and rule read")),
            excluded: Some(excluded_from(LEDGER).expect("D-3317 item 5 reads")),
            ..Evidence::default()
        }
    }

    fn problems_of(text: &str, evidence: &Evidence) -> Vec<Problem> {
        let entries = read(text).expect("the list reads");
        check(&entries, evidence)
    }

    /// `text` with `from` replaced by `to`; `from` must be there, so a test
    /// never passes on an edit that changed nothing.
    fn with(text: &str, from: &str, to: &str) -> String {
        assert!(text.contains(from), "{from:?} is not in the list");
        text.replacen(from, to, 1)
    }

    fn refused(problems: &[Problem], rule: &str, part: &str) -> bool {
        problems
            .iter()
            .any(|p| p.rule == rule && p.what.contains(part))
    }

    fn by_rule<'a>(problems: &'a [Problem], rule: &str) -> Vec<&'a Problem> {
        problems.iter().filter(|p| p.rule == rule).collect()
    }

    #[test]
    fn the_good_list_passes() {
        let p = problems_of(GOOD, &writers());
        assert_eq!(p, Vec::<Problem>::new());
        let (report, passed) = run(GOOD, &writers());
        assert!(passed);
        assert!(report.contains("store steps: 2 (pending 2, done 0, withdrawn 0); rules 1; packages 2 (merged 1); leftovers 2"));
        assert!(report.contains("| S-01 | pending | 1200 ms, 4096 bytes | DB-A01 |"));
        assert!(report.contains("| S-02 | pending | waits for WP-02 | DB-A02 |"));
        assert!(report.contains("steps whose command a package has not built yet: 1"));
        assert!(report.contains("merge history: not given; merged flags NOT CHECKED"));
        assert!(report.contains("rows marked as needing a store write: none given; board rows outside the leftovers NOT CHECKED"));
    }

    #[test]
    fn every_store_step_names_its_command_prerequisite_check_and_rows() {
        let s01 = &GOOD[GOOD.find("id = \"S-01\"").expect("S-01 is in the list")..];
        let line = |key: &str| {
            s01.lines()
                .find(|l| l.starts_with(&format!("{key} = ")))
                .expect("S-01 holds the key")
                .to_owned()
        };
        for key in ["command", "dry_run", "prerequisites", "check", "closes"] {
            let gone = problems_of(&with(GOOD, &line(key), ""), &writers());
            assert!(
                refused(&gone, "fields", &format!("step S-01: no {key}")),
                "{key}: {gone:?}"
            );
            let empty = problems_of(&with(GOOD, &line(key), &format!("{key} = []")), &writers());
            assert!(
                refused(&empty, "fields", "S-01: closes no row")
                    || refused(&empty, "fields", &format!("S-01: {key} names nothing")),
                "{key}: {empty:?}"
            );
        }
        for key in ["title", "writes", "state"] {
            let gone = problems_of(&with(GOOD, &line(key), ""), &writers());
            assert!(
                refused(&gone, "fields", &format!("S-01: no {key}")),
                "{key}: {gone:?}"
            );
        }
        let blank = problems_of(
            &with(GOOD, "title = \"a ready step\"", "title = \"  \""),
            &writers(),
        );
        assert!(refused(&blank, "fields", "S-01: title is empty"));
        let unwritten = problems_of(
            &with(
                GOOD,
                "writes = \"new month generations\"\ncommand = [\"cli",
                "writes = \" \"\ncommand = [\"cli",
            ),
            &writers(),
        );
        assert!(
            refused(&unwritten, "fields", "S-01: writes is empty"),
            "{unwritten:?}"
        );
        let nameless = problems_of(
            &with(GOOD, "check = [\"GET /x (0 today)\"]", "check = [\"\"]"),
            &writers(),
        );
        assert!(refused(&nameless, "fields", "S-02: check names nothing"));
        let stranger = problems_of(
            &with(
                GOOD,
                "prerequisites = [\"WP-02\"]",
                "prerequisites = [\"WP-02\", \"someone\"]",
            ),
            &writers(),
        );
        assert!(refused(
            &stranger,
            "fields",
            "\"someone\" is neither a declared package"
        ));
        let mute = problems_of(
            &with(
                GOOD,
                "prerequisites = [\"WP-02\"]",
                "prerequisites = [\"WP-02\", \"operator: \"]",
            ),
            &writers(),
        );
        assert!(refused(
            &mute,
            "fields",
            "an operator prerequisite says nothing"
        ));
        let not_a_row = problems_of(
            &with(
                GOOD,
                "closes = [\"DB-A02\"]",
                "closes = [\"DB-A02\", \"later\"]",
            ),
            &writers(),
        );
        assert!(refused(
            &not_a_row,
            "fields",
            "\"later\" is not a finding or audit row id"
        ));
        let bad_id = problems_of(&with(GOOD, "id = \"S-02\"", "id = \"S2\""), &writers());
        assert!(refused(
            &bad_id,
            "fields",
            "id \"S2\" is not S-<two or more digits>"
        ));
        let twice = problems_of(&with(GOOD, "id = \"S-02\"", "id = \"S-01\""), &writers());
        assert!(refused(&twice, "fields", "step S-01 is given twice"));
        let no_disk = problems_of(&with(GOOD, "disk = \"4 KiB\"", ""), &writers());
        assert!(refused(
            &no_disk,
            "fields",
            "S-01: a pending step needs disk"
        ));

        // The list in this repository: every step names all five.
        let entries = read(REAL).expect("config/store-steps.toml reads");
        let steps = of(&entries, "step");
        assert!(steps.len() >= 20, "the list holds {} steps", steps.len());
        for s in &steps {
            for key in ["command", "dry_run", "prerequisites", "check"] {
                assert!(
                    s.list(key).iter().any(|l| !l.trim().is_empty()),
                    "{}: {key}",
                    s.label()
                );
            }
            if s.text("state") != Some("withdrawn") {
                assert!(!s.list("closes").is_empty(), "{} closes no row", s.label());
            }
        }
        let real = check(&entries, &writers());
        assert_eq!(by_rule(&real, "fields"), Vec::<&Problem>::new());
    }

    #[test]
    fn every_leftover_waiting_for_a_store_run_is_named_by_one_step() {
        let unnamed = problems_of(
            &with(GOOD, "\"DB-A02\"]", "\"DB-A02\", \"DB-A03\"]"),
            &writers(),
        );
        assert!(refused(
            &unnamed,
            "leftovers",
            "leftover DB-A03 waits for a store run that no step names"
        ));
        let twice = problems_of(
            &with(
                GOOD,
                "closes = [\"DB-A02\"]",
                "closes = [\"DB-A02\", \"DB-A01\"]",
            ),
            &writers(),
        );
        assert!(refused(
            &twice,
            "leftovers",
            "leftover DB-A01 is closed by 2 steps at once: S-01, S-02"
        ));
        let unlisted = problems_of(
            &with(
                GOOD,
                "closes = [\"DB-A02\"]",
                "closes = [\"DB-A02\", \"DB-A09\"]",
            ),
            &writers(),
        );
        assert!(refused(
            &unlisted,
            "leftovers",
            "step S-02 closes DB-A09, which the list of leftovers does not hold"
        ));
        let listed_twice = problems_of(
            &with(GOOD, "\"DB-A02\"]", "\"DB-A02\", \"DB-A02\"]"),
            &writers(),
        );
        assert!(refused(
            &listed_twice,
            "leftovers",
            "leftover DB-A02 is listed twice"
        ));
        let odd = problems_of(
            &with(GOOD, "\"DB-A02\"]", "\"DB-A02\", \"later\"]"),
            &writers(),
        );
        assert!(refused(
            &odd,
            "leftovers",
            "leftover \"later\" is not a finding or audit row id"
        ));
        let mut marked = writers();
        marked.marked = vec!["DB-A01".to_owned(), "DB-B65".to_owned()];
        let missing = problems_of(GOOD, &marked);
        assert!(refused(
            &missing,
            "leftovers",
            "DB-B65 is marked as needing a store write"
        ));
        assert_eq!(by_rule(&missing, "leftovers").len(), 1);
        let (report, passed) = run(GOOD, &marked);
        assert!(!passed);
        assert!(
            report.contains(
                "rows marked as needing a store write: 2 read; each is checked to be a leftover"
            ),
            "{report}"
        );
        assert!(!report.contains("board rows outside the leftovers NOT CHECKED"));
        // A withdrawn step closes nothing, so it cannot take a leftover away
        // from the step that replaced it.
        let withdrawn = with(
            &with(
                GOOD,
                "not_rehearsed = \"waits for WP-02: the door is not built\"",
                "withdrawn_because = \"replaced\"\nreplaced_by = \"S-01\"",
            ),
            "state = \"pending\"\nwrites = \"new month generations\"\ncommand = [\n",
            "state = \"withdrawn\"\nwrites = \"new month generations\"\ncommand = [\n",
        );
        let w = problems_of(&withdrawn, &writers());
        assert!(refused(
            &w,
            "fields",
            "S-02: a withdrawn step closes nothing"
        ));
        assert!(refused(
            &w,
            "leftovers",
            "leftover DB-A02 waits for a store run that no step names"
        ));

        // The list in this repository: the rows R3-36 was given (its findings
        // and the leftovers the fix plan named), each closed by exactly one
        // step that is not withdrawn.
        let entries = read(REAL).expect("config/store-steps.toml reads");
        let listed = entries[0].list("leftovers");
        let steps = of(&entries, "step");
        for row in [
            "DB-A02",
            "DB-A03",
            "DB-A10",
            "DB-A11",
            "DB-A13",
            "DB-A20",
            "DB-A26",
            "DB-A27",
            "DB-G69",
            "DB-G87",
            "SC-CLOSE-1",
            "SC-CLOSE-21",
            "CMP-03",
            "CMP-12",
            "RA-288",
            "RA-329",
        ] {
            assert!(
                listed.iter().any(|l| l == row),
                "{row} is not a listed leftover"
            );
            let closers: Vec<&str> = steps
                .iter()
                .filter(|s| {
                    s.text("state") != Some("withdrawn")
                        && s.list("closes").iter().any(|c| c == row)
                })
                .filter_map(|s| s.text("id"))
                .collect();
            assert_eq!(closers.len(), 1, "{row} is closed by {closers:?}");
        }
        let real = check(&entries, &writers());
        assert_eq!(by_rule(&real, "leftovers"), Vec::<&Problem>::new());
    }

    #[test]
    fn a_step_without_rehearsal_figures_names_what_it_waits_for() {
        let figures = "rehearsed_ms = 1_200\nrehearsed_bytes = 4096\nrehearsed_on = \"a private copy\"\nrehearsed_result = \"1 day imported\"\n";
        let bare = problems_of(&with(GOOD, figures, ""), &writers());
        assert!(refused(
            &bare,
            "rehearsal",
            "S-01: every prerequisite is merged: rehearse it on a private copy"
        ));
        let half = problems_of(&with(GOOD, "rehearsed_bytes = 4096\n", ""), &writers());
        assert!(refused(
            &half,
            "rehearsal",
            "S-01: rehearsal figures come together"
        ));
        let files_alone = problems_of(&with(GOOD, figures, "rehearsed_files = 3\n"), &writers());
        assert!(refused(
            &files_alone,
            "rehearsal",
            "S-01: rehearsed_files without the figures"
        ));
        let both = problems_of(
            &with(
                GOOD,
                "disk = \"4 KiB\"",
                "disk = \"4 KiB\"\nnot_rehearsed = \"waits for WP-01\"",
            ),
            &writers(),
        );
        assert!(refused(
            &both,
            "rehearsal",
            "S-01: figures and not_rehearsed at once"
        ));
        let early = problems_of(
            &with(
                GOOD,
                "not_rehearsed = \"waits for WP-02: the door is not built\"",
                figures,
            ),
            &writers(),
        );
        assert!(refused(
            &early,
            "rehearsal",
            "S-02: figures beside an unmerged prerequisite (WP-02)"
        ));
        let merged = problems_of(
            &with(
                GOOD,
                "prerequisites = [\"WP-02\"]",
                "prerequisites = [\"WP-02\", \"WP-01\"]",
            )
            .replacen("waits for WP-02:", "waits for WP-01:", 1),
            &writers(),
        );
        assert!(refused(
            &merged,
            "rehearsal",
            "S-02: waits for WP-01, which is merged: rehearse it"
        ));
        let stranger = problems_of(
            &with(GOOD, "waits for WP-02:", "waits for WP-09:"),
            &writers(),
        );
        assert!(refused(
            &stranger,
            "rehearsal",
            "S-02: waits for \"WP-09\", which is not an unmet prerequisite"
        ));
        let no_operator = problems_of(
            &with(GOOD, "waits for WP-02:", "waits for operator:"),
            &writers(),
        );
        assert!(refused(
            &no_operator,
            "rehearsal",
            "S-02: waits for operator, but no prerequisite is 'operator: <what>'"
        ));
        let shapeless = problems_of(
            &with(GOOD, "waits for WP-02: the door is not built", "later"),
            &writers(),
        );
        assert!(refused(
            &shapeless,
            "rehearsal",
            "S-02: not_rehearsed must begin 'waits for <package>'"
        ));
        let silent = problems_of(
            &with(
                GOOD,
                "not_rehearsed = \"waits for WP-02: the door is not built\"\n",
                "",
            ),
            &writers(),
        );
        assert!(refused(
            &silent,
            "rehearsal",
            "S-02: no rehearsal figures and no not_rehearsed"
        ));
        // An operator item (a sign-in, a drill) is a wait the file can name.
        let kite = with(
            &with(
                GOOD,
                "prerequisites = [\"WP-02\"]",
                "prerequisites = [\"WP-01\", \"operator: a Kite sign-in valid on the day\"]",
            ),
            "waits for WP-02: the door is not built",
            "waits for operator: a Kite sign-in",
        )
        .replacen(
            "planned: cli gdfl-restate options 2018-11-07 2018-11-07",
            "POST /pull/repair member=NIFTY",
            1,
        );
        assert_eq!(problems_of(&kite, &writers()), Vec::<Problem>::new());
        // A done step is closed by its run, not by a rehearsal.
        let done = with(
            &with(
                GOOD,
                "state = \"pending\"\nwrites = \"new month generations\"\ncommand = [\"cli",
                "state = \"done\"\nwrites = \"new month generations\"\ncommand = [\"cli",
            ),
            figures,
            "ran = \"13 Oct 2026, run 1\"\nran_ms = 900\n",
        );
        assert_eq!(problems_of(&done, &writers()), Vec::<Problem>::new());

        // The list in this repository.
        let entries = read(REAL).expect("config/store-steps.toml reads");
        for s in of(&entries, "step")
            .iter()
            .filter(|s| s.text("state") == Some("pending"))
        {
            let figured = FIGURES.iter().all(|k| s.has(k));
            let waits = s
                .text("not_rehearsed")
                .is_some_and(|w| w.starts_with("waits for "));
            assert!(
                figured != waits,
                "{}: figures {figured}, waits {waits}",
                s.label()
            );
        }
        let real = check(&entries, &writers());
        assert_eq!(by_rule(&real, "rehearsal"), Vec::<&Problem>::new());
    }

    #[test]
    fn the_reader_refuses_what_it_does_not_read() {
        let err = |text: &str| read(text).expect_err("the reader refuses it");
        assert!(
            err("x = \"open").contains("line 1: a string is not closed before the end of the file")
        );
        assert!(
            err("x = \"open\nmore\"").contains("line 2: a string is not closed on its own line")
        );
        assert!(
            err("[[steps]]\n").contains(
                "[[steps]] is not a table this file holds (env, package, rule, step, owed)"
            )
        );
        assert!(err("aB = 1\n").contains("line 1: key \"aB\" is not lower_case"));
        assert!(err("[step]\n").contains("only [[table]] headers are read"));
        assert!(err("[[step]\n").contains("is not closed by ]]"));
        assert!(err("x \"y\"\n").contains("key \"x\" is not followed by ="));
        assert!(err("x = yes\n").contains("\"yes\" is not a value; strings are quoted"));
        assert!(err("x = -1\n").contains("'-' does not start a value"));
        assert!(err("x = 1__0\n").contains("\"1__0\" is not a non-negative integer"));
        assert!(err("x = 1_\n").contains("is not a non-negative integer"));
        assert!(err("x = 12ab\n").contains("\"12ab\" is not a non-negative integer"));
        assert!(err("x = 1\r\n").contains("a carriage return"));
        assert!(err("x = \"a\u{7}b\"\n").contains("control character"));
        assert!(err("x = \"a\\qb\"\n").contains("escape Some('q') is not read"));
        assert!(
            err("x = [\"a\",\n\"b\"").contains("an array is not closed before the end of the file")
        );
        assert!(err("x = [\"a\"").contains("an array is not closed before the end of the file"));
        // The end can come where an item belongs, too.
        assert!(err("x = [").contains("line 1: an array is not closed before the end of the file"));
        assert!(
            err("x = [\"a\",\n")
                .contains("line 2: an array is not closed before the end of the file")
        );
        assert!(err("x = [1]\n").contains("'1' inside an array of strings"));
        assert!(err("x = [\"a\" \"b\"]\n").contains("where a comma or ] belongs"));
        assert!(err("x = 1\nx = 2\n").contains("line 2: key \"x\" is given twice in one table"));
        assert!(err("x = 1 y\n").contains("'y' after a complete line"));
        assert!(err("x =\n").contains("does not start a value"));
        assert!(err("x =").contains("a key has no value"));
        assert!(err("X = 1\n").contains("'X' does not start a key or a [[table]]"));
        // Empty, single and commented input read; the checks then name what
        // is missing rather than passing an empty list.
        let empty = read("").expect("empty input reads");
        assert_eq!(empty.len(), 1);
        let p = check(&empty, &Evidence::default());
        assert!(refused(&p, "reader", "the top level: no format"));
        assert!(refused(&p, "reader", "the top level: no leftovers"));
        let entries =
            read("# only a comment\nformat = 1 # one\nleftovers = [ # none\n]\n").expect("reads");
        assert_eq!(entries[0].number("format"), Some(1));
        assert!(entries[0].list("leftovers").is_empty());
        let escaped = read("x = \"a\\\"b\\\\c\\nd\\te\"\n").expect("escapes read");
        assert_eq!(escaped[0].text("x"), Some("a\"b\\c\nd\te"));
        // A tab is a blank between words and a character inside a string.
        let tabs = read("x\t=\t\"a\tb\"\t# a tab\n").expect("tabs read");
        assert_eq!(tabs[0].text("x"), Some("a\tb"));
        let lines = read("[[step]]\n\n[[step]]\nid = \"S-01\"\n").expect("reads");
        assert_eq!(
            (lines[1].line, lines[2].line, lines[2].fields["id"].1),
            (1, 3, 4)
        );
    }

    #[test]
    fn a_key_the_table_does_not_hold_or_of_the_wrong_shape_is_refused() {
        let typo = problems_of(
            &with(
                GOOD,
                "check = [\"GET /x (0 today)\"]",
                "chek = [\"GET /x\"]",
            ),
            &writers(),
        );
        assert!(refused(&typo, "reader", "step S-02: key \"chek\""));
        assert!(refused(&typo, "fields", "step S-02: no check"));
        let shape = problems_of(&with(GOOD, "merged = true", "merged = \"yes\""), &writers());
        assert!(refused(&shape, "reader", "package WP-01: merged (line"));
        assert!(refused(&shape, "reader", "is a string, not a boolean"));
        // Each shape is named as what it is, so the refusal says what to fix.
        for (value, kind) in [
            ("[\"yes\"]", "is an array of strings, not a boolean"),
            ("1", "is an integer, not a boolean"),
        ] {
            let p = problems_of(
                &with(GOOD, "merged = true", &format!("merged = {value}")),
                &writers(),
            );
            assert!(refused(&p, "reader", kind), "{value}: {p:?}");
        }
        for (from, to, kind) in [
            (
                "title = \"a merged package\"",
                "title = [\"a\"]",
                "package WP-01: title (line",
            ),
            (
                "closes = [\"DB-A01\"]",
                "closes = \"DB-A01\"",
                "S-01: closes (line",
            ),
        ] {
            let p = problems_of(&with(GOOD, from, to), &writers());
            assert!(refused(&p, "reader", kind), "{to}: {p:?}");
        }
        assert!(refused(
            &problems_of(
                &with(GOOD, "title = \"a merged package\"", "title = [\"a\"]"),
                &writers()
            ),
            "reader",
            "is an array of strings, not a string"
        ));
        assert!(refused(
            &problems_of(
                &with(GOOD, "closes = [\"DB-A01\"]", "closes = \"DB-A01\""),
                &writers()
            ),
            "reader",
            "is a string, not an array of strings"
        ));
        let flag = problems_of(
            &with(GOOD, "rehearsed_bytes = 4096", "rehearsed_bytes = true"),
            &writers(),
        );
        assert!(
            refused(&flag, "reader", "is a boolean, not an integer"),
            "{flag:?}"
        );
        let format = problems_of(&with(GOOD, "format = 1", "format = 2"), &writers());
        assert!(refused(&format, "reader", "format is not 1"));
        let number = problems_of(
            &with(
                GOOD,
                "rehearsed_bytes = 4096",
                "rehearsed_bytes = \"4 KiB\"",
            ),
            &writers(),
        );
        assert!(refused(&number, "reader", "rehearsed_bytes (line"));
        let bad_package = problems_of(
            &with(GOOD, "name = \"WP-02\"", "name = \"wp two\""),
            &writers(),
        );
        assert!(refused(
            &bad_package,
            "packages",
            "\"wp two\" is not a package name"
        ));
        let twice = problems_of(
            &with(GOOD, "name = \"WP-02\"", "name = \"WP-01\""),
            &writers(),
        );
        assert!(refused(
            &twice,
            "packages",
            "package WP-01 is declared twice"
        ));
    }

    #[test]
    fn merged_flags_follow_the_history_when_it_names_merges() {
        let mut evidence = writers();
        evidence.merged = Some(merged_from("Merge WP-01: one\nMerge WP-02: two\n"));
        let late = problems_of(GOOD, &evidence);
        assert!(refused(
            &late,
            "packages",
            "package WP-02 is merged (a 'Merge WP-02:' subject is in the history) but the list says merged = false"
        ));
        evidence.merged = Some(merged_from("Merge WP-09: another\n"));
        let claimed = problems_of(GOOD, &evidence);
        assert!(refused(
            &claimed,
            "packages",
            "package WP-01 says merged = true but no 'Merge WP-01:' subject is in the history read"
        ));
        // A squash-merged history names no package merge: the flags stand as
        // written, and the report says they were not checked.
        evidence.merged = Some(merged_from("one squashed commit\n"));
        assert!(by_rule(&problems_of(GOOD, &evidence), "packages").is_empty());
        let (report, passed) = run(GOOD, &evidence);
        assert!(passed);
        assert!(report.contains("the merged flags stand as written, NOT CHECKED against git"));
        evidence.merged = Some(merged_from("Merge WP-01: one\n"));
        let (report, passed) = run(GOOD, &evidence);
        assert!(passed, "{report}");
        assert!(report.contains("merge history: 1 package merges read; merged flags checked"));
    }

    #[test]
    fn a_rule_past_the_package_that_ends_it_is_refused() {
        let ended = problems_of(
            &with(GOOD, "until = \"WP-02\"", "until = \"WP-01\""),
            &writers(),
        );
        assert!(refused(
            &ended,
            "rules",
            "rule R-01 lasts until WP-01, which is merged: retire it"
        ));
        let retired = problems_of(
            &with(
                GOOD,
                "until = \"WP-02\"",
                "until = \"WP-01\"\nretired = \"WP-01 gives each run its own log\"",
            ),
            &writers(),
        );
        assert_eq!(retired, Vec::<Problem>::new());
        let unknown = problems_of(
            &with(GOOD, "until = \"WP-02\"", "until = \"WP-77\""),
            &writers(),
        );
        assert!(refused(
            &unknown,
            "rules",
            "until \"WP-77\", which is no declared package"
        ));
        let forever = problems_of(
            &with(GOOD, "until = \"WP-02\"", "until = \"never\""),
            &writers(),
        );
        assert!(by_rule(&forever, "rules").is_empty());
        let sourceless = problems_of(
            &with(GOOD, "source = [\"RA-118\"]", "source = []"),
            &writers(),
        );
        assert!(refused(
            &sourceless,
            "rules",
            "rule R-01: names no source row"
        ));
        let odd = problems_of(
            &with(GOOD, "source = [\"RA-118\"]", "source = [\"a talk\"]"),
            &writers(),
        );
        assert!(refused(
            &odd,
            "rules",
            "source \"a talk\" is not a row or decision id"
        ));
        let id = problems_of(
            &with(GOOD, "id = \"R-01\"", "id = \"rule one\""),
            &writers(),
        );
        assert!(refused(&id, "rules", "id is not R-<two or more digits>"));
    }

    #[test]
    fn step_order_names_steps_and_has_no_cycle() {
        let unknown = problems_of(
            &with(GOOD, "after = [\"S-01\"]", "after = [\"S-07\"]"),
            &writers(),
        );
        assert!(refused(
            &unknown,
            "after",
            "step S-02: after S-07, which is no step"
        ));
        let itself = problems_of(
            &with(GOOD, "after = [\"S-01\"]", "after = [\"S-02\"]"),
            &writers(),
        );
        assert!(refused(&itself, "after", "step S-02 is after itself"));
        let cycle = problems_of(
            &with(
                GOOD,
                "disk = \"4 KiB\"",
                "disk = \"4 KiB\"\nafter = [\"S-02\"]",
            ),
            &writers(),
        );
        assert!(refused(&cycle, "after", "are each after the other"));
        let early = with(
            &with(
                GOOD,
                "\"planned: cli gdfl-restate options 2018-11-07 2018-11-07\", # one line per command",
                "\"cli gdfl-import indices zip 2018-09-03 2026-09-24\",",
            ),
            "not_rehearsed = \"waits for WP-02: the door is not built\"\ndisk = \"unknown until rehearsed\"",
            "ran = \"13 Oct, run 7\"\nran_ms = 5",
        );
        let early = with(
            &early,
            "state = \"pending\"\nwrites = \"new month generations\"\ncommand = [\n",
            "state = \"done\"\nwrites = \"new month generations\"\ncommand = [\n",
        );
        let p = problems_of(&early, &writers());
        assert!(refused(
            &p,
            "after",
            "step S-02 is done, but S-01, which it follows, is not"
        ));
        let chain = "[[step]]\nid = \"S-0N\"\nafter = [\"S-0M\"]\n";
        let three: String = [("1", "3"), ("2", "1"), ("3", "2")]
            .iter()
            .map(|(n, m)| chain.replace('N', n).replace('M', m))
            .collect();
        let entries = read(&format!("format = 1\nleftovers = []\n{three}")).expect("reads");
        let p = check(&entries, &Evidence::default());
        assert_eq!(by_rule(&p, "after").len(), 1, "{p:?}");
    }

    #[test]
    fn a_done_step_carries_its_run_and_a_withdrawn_step_names_its_replacement() {
        let to_done = |text: &str| {
            with(
                text,
                "state = \"pending\"\nwrites = \"new month generations\"\ncommand = [\"cli",
                "state = \"done\"\nwrites = \"new month generations\"\ncommand = [\"cli",
            )
        };
        let runless = problems_of(&to_done(GOOD), &writers());
        assert!(refused(
            &runless,
            "fields",
            "S-01: a step is closed only by its run, and a done step needs ran"
        ));
        assert!(refused(&runless, "fields", "a done step needs ran_ms"));
        let early_run = problems_of(
            &with(
                GOOD,
                "disk = \"4 KiB\"",
                "disk = \"4 KiB\"\nran = \"today\"",
            ),
            &writers(),
        );
        assert!(refused(
            &early_run,
            "fields",
            "S-01: only a done step carries ran and ran_ms"
        ));
        let state = problems_of(
            &with(GOOD, "state = \"pending\"", "state = \"maybe\""),
            &writers(),
        );
        assert!(refused(
            &state,
            "fields",
            "state \"maybe\" is not pending, done or withdrawn"
        ));
        let withdrawn = with(
            GOOD,
            "state = \"pending\"\nwrites = \"new month generations\"\ncommand = [\"cli",
            "state = \"withdrawn\"\nwrites = \"new month generations\"\ncommand = [\"cli",
        );
        let p = problems_of(&withdrawn, &writers());
        assert!(refused(
            &p,
            "fields",
            "S-01: a withdrawn step needs withdrawn_because"
        ));
        assert!(refused(
            &p,
            "fields",
            "S-01: a withdrawn step needs replaced_by"
        ));
        assert!(refused(
            &p,
            "fields",
            "S-01: a withdrawn step closes nothing"
        ));
        assert!(refused(
            &p,
            "after",
            "step S-02: after S-01, which is withdrawn"
        ));
        let replaced = problems_of(
            &with(
                &withdrawn,
                "disk = \"4 KiB\"",
                "disk = \"4 KiB\"\nwithdrawn_because = \"sealing\"\nreplaced_by = \"S-09\"",
            ),
            &writers(),
        );
        assert!(refused(
            &replaced,
            "after",
            "step S-01: replaced by S-09, which is no step"
        ));
        let pending_withdrawn = problems_of(
            &with(
                GOOD,
                "disk = \"4 KiB\"",
                "disk = \"4 KiB\"\nreplaced_by = \"S-02\"",
            ),
            &writers(),
        );
        assert!(refused(
            &pending_withdrawn,
            "fields",
            "S-01: a pending step is not withdrawn"
        ));
    }

    #[test]
    fn commands_name_a_store_writer_and_dry_runs_never_write_the_real_store() {
        let reader = problems_of(
            &with(
                GOOD,
                "command = [\"cli gdfl-import",
                "command = [\"cli sweep-stored",
            ),
            &writers(),
        );
        assert!(refused(
            &reader,
            "commands",
            "S-01: cli sweep-stored is not one of the cli's store writers"
        ));
        let shell = problems_of(
            &with(
                GOOD,
                "command = [\"cli gdfl-import",
                "command = [\"sh -c gdfl-import",
            ),
            &writers(),
        );
        assert!(refused(
            &shell,
            "commands",
            "begins with none of cli | POST | planned: cli | planned: POST "
        ));
        let same = problems_of(
            &with(
                GOOD,
                "NIFTY BANKNIFTY with BRUTEX_STORE on a private copy\"]",
                "NIFTY BANKNIFTY\"]",
            ),
            &writers(),
        );
        assert!(refused(&same, "commands", "is the command itself"));
        assert!(refused(&same, "commands", "names no read-only form"));
        let still_planned = problems_of(
            &with(
                GOOD,
                "prerequisites = [\"WP-02\"]",
                "prerequisites = [\"WP-01\"]",
            ),
            &writers(),
        );
        assert!(refused(
            &still_planned,
            "commands",
            "S-02: every prerequisite is met, but the command is still planned"
        ));
        let bad_env = problems_of(
            &with(
                GOOD,
                "env = [\"BRUTEX_STORE\"]",
                "env = [\"BRUTEX_STORE\", \"brutex store\"]",
            ),
            &writers(),
        );
        assert!(refused(
            &bad_env,
            "commands",
            "env \"brutex store\" is not a variable name"
        ));
        for line in [
            "cli tix-rebuild gdfl NSE/INDEX/NIFTY 1s 2018-09 2024-07 --dry-run",
            "cli srt-build gdfl NSE/INDEX/NIFTY 1s 2018-09 2026-09 --dry-run",
            "cli gdfl-rekey --plan --underlying NIFTY",
            "cli zerodha-repair NIFTY 2018-01-01 2026-09-30",
            "cli gdfl-import indices zip 2022-03-07 2022-03-07 with BRUTEX_STORE on a private copy",
            "the drill with BRUTEX_STORE on a private copy on a scratch volume",
        ] {
            assert_eq!(
                dry_run_writes(line, writers().writers.as_ref()),
                None,
                "{line}"
            );
        }
        for line in [
            "cli gdfl-import indices zip 2022-03-07 2022-03-07",
            "cli gdfl-import indices zip 2022-03-07 2022-03-07 on a private copy",
            "cli zerodha-repair NIFTY 2021-02-24 2021-02-24 --from-store x --reason y",
            "cli census-migrate gdfl --seal",
            "the drill on a scratch volume",
        ] {
            assert!(
                dry_run_writes(line, writers().writers.as_ref()).is_some(),
                "{line}"
            );
        }
        // Without the writer table, verbs are not judged, and the report says so.
        let (report, passed) = run(
            &with(
                GOOD,
                "command = [\"cli gdfl-import",
                "command = [\"cli sweep-stored",
            ),
            &Evidence::default(),
        );
        assert!(passed);
        assert!(report.contains(
            "cli writer table: not given; cli verbs and the writes of check lines NOT CHECKED"
        ));
    }

    #[test]
    fn declared_names_are_used_and_used_names_declared() {
        let undeclared = problems_of(
            &with(
                GOOD,
                "env = [\"BRUTEX_STORE\"]",
                "env = [\"BRUTEX_STORE\", \"BRUTEX_OTHER\"]",
            ),
            &writers(),
        );
        assert!(refused(
            &undeclared,
            "env",
            "variable BRUTEX_OTHER is named by a step and declared by no [[env]]"
        ));
        let unused = problems_of(&with(GOOD, "env = [\"BRUTEX_STORE\"]\n", ""), &writers());
        assert!(refused(
            &unused,
            "env",
            "variable BRUTEX_STORE is declared and named by no step"
        ));
        let idle = problems_of(
            &format!("{GOOD}\n[[package]]\nname = \"WP-03\"\nmerged = false\ntitle = \"idle\"\n"),
            &writers(),
        );
        assert!(refused(
            &idle,
            "packages",
            "package WP-03 is named by no step and no rule"
        ));
    }

    #[test]
    fn the_writer_table_is_read_from_the_cli_source() {
        let verbs = writers_from(CLI_WRITERS).expect("the table reads");
        for verb in [
            "gdfl-import",
            "zerodha-repair",
            "census-migrate",
            "gdfl-rekey",
            "scrub-blocks",
            "gdfl-dayflags",
        ] {
            assert!(verbs.contains(verb), "{verb}");
        }
        assert!(!verbs.contains("sweep-stored"));
        assert!(
            writers_from("no table here")
                .unwrap_err()
                .contains("is not in the file read")
        );
        let short = "pub(crate) const WRITERS: [&str; 2] = [\n    \"a\",\n];\n";
        assert!(
            writers_from(short)
                .unwrap_err()
                .contains("says 2 verbs and holds 1")
        );
        let open = "pub(crate) const WRITERS: [&str; 1] = [\n    \"a\",\n";
        assert!(writers_from(open).unwrap_err().contains("not closed by ];"));
        let length = "pub(crate) const WRITERS: [&str; x] = [];";
        assert!(
            writers_from(length)
                .unwrap_err()
                .contains("is not a number")
        );
        let ok = "pub(crate) const WRITERS: [&str; 2] = [\n    \"a\",\n    \"b\",\n];\n";
        assert_eq!(writers_from(ok).expect("reads").len(), 2);
    }

    #[test]
    fn the_write_rule_is_the_clis_own_read_from_its_source() {
        let table = table_from(CLI_WRITERS).expect("the cli's rule reads");
        // The cli's own classification table (store_write.rs tests), case
        // for case: one rule, read twice, never written twice.
        for (verb, words, writes) in [
            ("gdfl-import", &["indices", "zip"][..], true),
            ("census-migrate", &["gdfl", "--seal"][..], true),
            ("tix-rebuild", &["gdfl", "--dry-run"][..], false),
            ("tix-rebuild", &["gdfl", "--dry-run-later"][..], true),
            ("srt-build", &["gdfl", "--dry-run"][..], false),
            ("gdfl-rekey", &["--plan"][..], false),
            ("gdfl-rekey", &["--apply", "--plan-later"][..], true),
            ("gdfl-rekey", &["--plan-later", "--apply"][..], true),
            ("zerodha-repair", &["NIFTY"][..], false),
            ("zerodha-repair", &["NIFTY", "--from-store", "x"][..], true),
            ("alerts", &[][..], false),
            ("alerts", &["--ack", "3"][..], true),
            ("alerts", &["--replace-damaged"][..], true),
            ("alerts", &["3", "--ack"][..], false),
            ("no-such-verb", &["--apply"][..], false),
        ] {
            assert_eq!(table.writes(verb, words), writes, "{verb} {words:?}");
        }
        let rule = |arms: &str| {
            let verbs = "pub(crate) const WRITERS: [&str; 2] = [\n    \"a\",\n    \"b\",\n];\n";
            table_from(&format!(
                "{verbs}pub(crate) fn writes_the_store(verb: &str, arguments: &[&str]) -> bool {{\n    match verb {{\n{arms}    }}\n}}\n"
            ))
        };
        let two = rule("        \"a\" | \"b\" => !arguments.contains(&\"--dry-run\"),\n        other => WRITERS.contains(&other),\n")
            .expect("reads");
        assert_eq!(two.forms.len(), 2);
        assert!(!two.writes("b", &["--dry-run"]));
        for (arms, why) in [
            (
                "        \"a\" => arguments.len() > 2,\n        other => WRITERS.contains(&other),\n",
                "an arm this gate does not read",
            ),
            (
                "        \"a\" => arguments.contains(&\"x y\"),\n        other => WRITERS.contains(&other),\n",
                "an arm this gate does not read",
            ),
            (
                "        \"c\" => arguments.contains(&\"--apply\"),\n        other => WRITERS.contains(&other),\n",
                "names c, which is not in WRITERS",
            ),
            (
                "        \"a\" => arguments.contains(&\"--apply\"),\n        \"a\" => arguments.contains(&\"--go\"),\n        other => WRITERS.contains(&other),\n",
                "names a twice",
            ),
            (
                "        \"a\" => arguments.contains(&\"--apply\"),\n",
                "does not end with",
            ),
            (
                "        a => arguments.contains(&\"--apply\"),\n        other => WRITERS.contains(&other),\n",
                "an arm this gate does not read",
            ),
        ] {
            let got = rule(arms).expect_err(why);
            assert!(got.contains(why), "{got}");
        }
        let no_rule = "pub(crate) const WRITERS: [&str; 1] = [\n    \"a\",\n];\n";
        assert!(
            table_from(no_rule)
                .unwrap_err()
                .contains("is not in the file read")
        );
        let no_match = format!(
            "{no_rule}pub(crate) fn writes_the_store(verb: &str, arguments: &[&str]) -> bool {{\n    true\n}}\n"
        );
        assert!(
            table_from(&no_match)
                .unwrap_err()
                .contains("does not begin with")
        );
    }

    #[test]
    fn merge_subjects_and_marked_rows_are_read_in_their_one_form() {
        let m = merged_from(
            "Merge WP-12d: GDFL back-fill\nMerge feat/db-fix (WP-61, WP-12d) into fix/db-WP-39\nWIP WP-02: z\nMerge IA-10a: w\nMerge R3-22 the door\nMerge SC-0: rule\n",
        );
        let want: BTreeSet<String> = ["IA-10a", "SC-0", "WP-12d"]
            .iter()
            .map(|s| (*s).to_owned())
            .collect();
        assert_eq!(m, want);
        assert_eq!(
            marked_from("DB-A01\n\n# a note\nDB-B65 # WP-81\n"),
            vec!["DB-A01", "DB-B65"]
        );
        for name in ["WP-12f", "IA-10a", "R3-22", "SC-0", "WP-81"] {
            assert!(is_package_name(name), "{name}");
        }
        // "WP-100ab": two letters after three digits, which a quotient of
        // the lengths (5 / 3 = 1) would let through (verify-R3-36-r3).
        for name in ["WP-12fx", "wp-1", "WP-", "WP-a", "WP12", "-1", "WP-100ab"] {
            assert!(!is_package_name(name), "{name}");
        }
        for id in [
            "DB-A02",
            "SC-CLOSE-1",
            "RA-288",
            "D-3342",
            "CMP-03",
            "DB-012",
        ] {
            assert!(is_row_id(id), "{id}");
        }
        for id in ["db-a02", "DB-", "DB", "DB--1", "-1", "DB-a1"] {
            assert!(!is_row_id(id), "{id}");
        }
    }

    #[test]
    fn arguments_are_refused_unless_well_formed() {
        let args = |words: &[&str]| {
            words
                .iter()
                .map(|w| (*w).to_owned())
                .collect::<Vec<String>>()
        };
        assert_eq!(parse_arguments(&args(&[])).unwrap_err(), USAGE);
        assert_eq!(parse_arguments(&args(&["check"])).unwrap_err(), USAGE);
        assert_eq!(parse_arguments(&args(&["run", "list"])).unwrap_err(), USAGE);
        assert_eq!(
            parse_arguments(&args(&["check", "list", "--merges"])).unwrap_err(),
            USAGE
        );
        assert_eq!(
            parse_arguments(&args(&["check", "list", "--bogus", "x"])).unwrap_err(),
            USAGE
        );
        assert_eq!(
            parse_arguments(&args(&["check", "list", "--merges", "a", "--merges", "b"]))
                .unwrap_err(),
            USAGE
        );
        let words = args(&["check", "list", "--writers", "w", "--marked", "m"]);
        let (list, flags) = parse_arguments(&words).expect("reads");
        assert_eq!(
            (list, flags),
            ("list", vec![("--writers", "w"), ("--marked", "m")])
        );
        assert!(
            arguments(
                &args(&["check", "/nonexistent/store-steps.toml"]),
                &from_disk
            )
            .unwrap_err()
            .contains("/nonexistent/store-steps.toml")
        );
    }

    #[test]
    fn the_tool_prints_its_report_and_exits_by_what_it_found() {
        let args = |words: &[&str]| words.iter().map(|w| (*w).to_owned()).collect::<Vec<_>>();
        let files = |path: &str| -> Result<String, String> {
            match path {
                "good" => Ok(GOOD.to_owned()),
                "bad" => Ok(with(GOOD, "title = \"a ready step\"", "title = \" \"")),
                "merges" => Ok("Merge WP-01: one\n".to_owned()),
                "writers" => Ok(CLI_WRITERS.to_owned()),
                "decisions" => Ok("### D-3317 — x\n5. **y** (2018-11-06)\n".to_owned()),
                "marked" => Ok("DB-A01 # a row\n".to_owned()),
                "garbage" => Ok("no table here".to_owned()),
                other => Err(format!("{other}: no such file")),
            }
        };
        let all = args(&[
            "check",
            "good",
            "--merges",
            "merges",
            "--writers",
            "writers",
            "--decisions",
            "decisions",
            "--marked",
            "marked",
        ]);
        let (out, err, code) = gate(&all, &files);
        assert_eq!((err.as_str(), code), ("", 0), "{out}");
        assert!(
            out.contains("decision ledger: D-3317 item 5 read; no census-recount gdfl span may cover 2018-11-06\n"),
            "{out}"
        );
        assert!(
            out.starts_with("store steps: 2 (pending 2, done 0, withdrawn 0)"),
            "{out}"
        );
        assert!(out.contains("merge history: 1 package merges read; merged flags checked\n"));
        assert!(out.contains(
            "rows marked as needing a store write: 1 read; each is checked to be a leftover\n"
        ));
        assert!(!out.contains("cli writer table: not given"));
        assert!(out.ends_with(
            "\nOK — every store step names its command, prerequisite, check and rows.\n"
        ));
        let (out, err, code) = gate(&args(&["check", "bad"]), &files);
        assert_eq!((err.as_str(), code), ("", 1));
        assert!(
            out.contains("REFUSED [fields] step S-01: title is empty\n"),
            "{out}"
        );
        assert!(out.ends_with("\nREFUSED — fix config/store-steps.toml (R3-36).\n"));
        // Not checked at all: nothing on the report's side, the reason on
        // the error's, and exit 2.
        for (words, why) in [
            (vec!["check"], format!("gates_store_steps: {USAGE}\n")),
            (vec!["check", "nowhere"], "gates_store_steps: nowhere: no such file\n".to_owned()),
            (
                vec!["check", "good", "--merges", "nowhere"],
                "gates_store_steps: nowhere: no such file\n".to_owned(),
            ),
            (
                vec!["check", "good", "--writers", "garbage"],
                "gates_store_steps: the writer table (pub(crate) const WRITERS: [&str; N]) is not in the file read\n".to_owned(),
            ),
            (
                vec!["check", "good", "--decisions", "garbage"],
                "gates_store_steps: the decision ledger holds no entry beginning \"### D-3317 \"\n".to_owned(),
            ),
            (
                vec!["check", "good", "--decisions", "decisions", "--decisions", "decisions"],
                format!("gates_store_steps: {USAGE}\n"),
            ),
        ] {
            assert_eq!(gate(&args(&words), &files), (String::new(), why, 2), "{words:?}");
        }
        assert_eq!(
            USAGE,
            "usage: gates_store_steps check LIST [--merges SUBJECTS] [--writers STORE_WRITE_RS] [--decisions LEDGER] [--marked IDS] [--owed OWED_RUNS]"
        );
    }

    /// A writer that refuses every write, as a closed pipe does.
    struct Closed;

    impl std::io::Write for Closed {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "closed",
            ))
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    /// A writer that takes every write and refuses to flush.
    struct Unflushed(Vec<u8>);

    impl std::io::Write for Unflushed {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Err(std::io::Error::other("disk full"))
        }
    }

    #[test]
    fn the_report_reaches_its_reader_or_the_run_does_not_pass() {
        let given = |out: &str, err: &str, code: u8| (out.to_owned(), err.to_owned(), code);
        // Both written: each stream holds exactly its own text, and the
        // code is the gate's, whichever it was.
        for code in [0, 1, 2] {
            let (mut out, mut err) = (Vec::new(), Vec::new());
            assert_eq!(
                emit(given("report\n", "why\n", code), &mut out, &mut err),
                code
            );
            assert_eq!(
                (out.as_slice(), err.as_slice()),
                (&b"report\n"[..], &b"why\n"[..])
            );
        }
        // The report cannot be written: exit 2 though the list passed, and
        // stderr carries the gate's own error, then the reason.
        let mut err = Vec::new();
        assert_eq!(
            emit(given("OK\n", "earlier\n", 0), &mut Closed, &mut err),
            2
        );
        assert_eq!(
            String::from_utf8(err).unwrap(),
            "earlier\ngates_store_steps: the report was not written: closed\n"
        );
        // Written but not flushed (a full disk behind a redirect): exit 2.
        let (mut out, mut err) = (Unflushed(Vec::new()), Vec::new());
        assert_eq!(emit(given("OK\n", "", 0), &mut out, &mut err), 2);
        assert_eq!(out.0, b"OK\n");
        assert_eq!(
            String::from_utf8(err).unwrap(),
            "gates_store_steps: the report was not written: disk full\n"
        );
        // The error stream cannot be written: exit 2 too, the report kept.
        // (An empty error is no write at all, so it needs no stream.)
        let mut out = Vec::new();
        assert_eq!(
            emit(given("REFUSED\n", "why\n", 1), &mut out, &mut Closed),
            2
        );
        assert_eq!(out, b"REFUSED\n");
        let mut out = Vec::new();
        assert_eq!(emit(given("OK\n", "", 0), &mut out, &mut Closed), 0);
        assert_eq!(out, b"OK\n");
        // Nothing to say on either stream still writes nothing and passes.
        let (mut out, mut err) = (Vec::new(), Vec::new());
        assert_eq!(emit(given("", "", 0), &mut out, &mut err), 0);
        assert!(out.is_empty() && err.is_empty());
    }

    /// GOOD with a third step that copies the store, ready to be broken.
    fn with_copy(command: &str, prerequisite: &str, checks: &[&str]) -> String {
        let checks: Vec<String> = checks.iter().map(|c| format!("{c:?}")).collect();
        format!(
            "{}\n[[step]]\nid = \"S-03\"\ntitle = \"a copy\"\nstate = \"pending\"\nwrites = \"a copy on a second drive\"\ncommand = [{command:?}]\ndry_run = [\"rsync -a --dry-run STORE DRIVE\"]\nprerequisites = [\"WP-01\", {prerequisite:?}]\ncheck = [{}]\ncloses = [\"DB-A09\"]\nnot_rehearsed = \"waits for operator: a second drive\"\ndisk = \"on the second drive\"\n",
            with(GOOD, "\"DB-A02\"]", "\"DB-A02\", \"DB-A09\"]"),
            checks.join(", ")
        )
    }

    #[test]
    fn a_backup_taken_under_writers_is_refused_or_verified_on_restore() {
        let recount = "cli census-recount gdfl --from 2018-11-10 on the restored copy, before any writer starts (this check writes the census and the journal)";
        let scrub =
            "cli scrub-blocks gdfl on the restored copy (this check writes its log and cursor)";
        let rsync = "planned: cli backup <drive>";
        let good = with_copy(rsync, STOPPED, &[recount, scrub]);
        assert_eq!(problems_of(&good, &writers()), Vec::<Problem>::new());
        // Taken while a writer appends: no stopped-writers prerequisite.
        let torn = problems_of(
            &with_copy(rsync, "operator: a second drive", &[recount, scrub]),
            &writers(),
        );
        assert!(refused(
            &torn,
            "copies",
            "S-03: a copy or snapshot of the store taken while a writer appends is torn"
        ));
        assert_eq!(by_rule(&torn, "copies").len(), 1);
        // A snapshot is a copy too, and an unverified restore is refused once
        // per missing read-back.
        let unread = problems_of(
            &with_copy(
                "planned: cli snapshot take",
                STOPPED,
                &["tmutil listlocalsnapshots /Volumes/X"],
            ),
            &writers(),
        );
        assert!(refused(
            &unread,
            "copies",
            "S-03: a restore is read back before any writer starts: name a check line beginning \"cli census-recount\""
        ));
        assert!(refused(&unread, "copies", "beginning \"cli scrub-blocks\""));
        assert_eq!(by_rule(&unread, "copies").len(), 2);
        let half = problems_of(&with_copy(rsync, STOPPED, &[recount]), &writers());
        assert_eq!(by_rule(&half, "copies").len(), 1);
        // A step that copies nothing is not judged by the rule.
        assert!(by_rule(&problems_of(GOOD, &writers()), "copies").is_empty());
        let (report, passed) = run(
            &with_copy(rsync, "operator: a second drive", &[recount, scrub]),
            &writers(),
        );
        assert!(!passed);
        assert!(report.contains("copies refusals: 1"), "{report}");

        // The list in this repository: the snapshot and the second-drive
        // copy both stop every writer and read a restore back.
        let entries = read(REAL).expect("config/store-steps.toml reads");
        let copying: Vec<String> = of(&entries, "step")
            .iter()
            .filter(|s| {
                s.list("command")
                    .iter()
                    .any(|c| COPIES.iter().any(|h| c.starts_with(h)))
            })
            .filter_map(|s| s.text("id").map(str::to_owned))
            .collect();
        assert_eq!(copying, ["S-01", "S-17"]);
        assert_eq!(
            by_rule(&check(&entries, &writers()), "copies"),
            Vec::<&Problem>::new()
        );
    }

    #[test]
    fn a_partial_run_stays_pending_and_a_run_out_of_order_says_so() {
        let partial = with(
            GOOD,
            "disk = \"unknown until rehearsed\"",
            "disk = \"unknown until rehearsed\"\nlast_run = \"14 Oct, run 7: 10 of 13 keys moved, 3 refused by name\"",
        );
        assert_eq!(problems_of(&partial, &writers()), Vec::<Problem>::new());
        let (report, _) = run(&partial, &writers());
        assert!(
            report.contains("| S-02 | pending, partly run | waits for WP-02 | DB-A02 |"),
            "{report}"
        );
        let blank = problems_of(
            &with(
                &partial,
                "14 Oct, run 7: 10 of 13 keys moved, 3 refused by name",
                " ",
            ),
            &writers(),
        );
        assert!(refused(&blank, "fields", "S-02: last_run is empty"));

        // S-02 done while S-01, which it follows, is still pending.
        let early = with(
            &with(
                GOOD,
                "\"planned: cli gdfl-restate options 2018-11-07 2018-11-07\", # one line per command",
                "\"cli gdfl-import indices zip 2018-09-03 2026-09-24\",",
            ),
            "not_rehearsed = \"waits for WP-02: the door is not built\"\ndisk = \"unknown until rehearsed\"",
            "ran = \"14 Oct, run 7\"\nran_ms = 5",
        );
        let early = with(
            &early,
            "state = \"pending\"\nwrites = \"new month generations\"\ncommand = [\n",
            "state = \"done\"\nwrites = \"new month generations\"\ncommand = [\n",
        );
        let silent = problems_of(&early, &writers());
        assert!(refused(
            &silent,
            "after",
            "step S-02 is done, but S-01, which it follows, is not: if it really ran first, say so in ran_out_of_order"
        ));
        let said = with(
            &early,
            "ran_ms = 5",
            "ran_ms = 5\nran_out_of_order = \"ran before S-01; S-01's run back-fills what this left\"",
        );
        assert_eq!(problems_of(&said, &writers()), Vec::<Problem>::new());
        let (report, _) = run(&said, &writers());
        assert!(
            report.contains("| S-02 | done, out of order | ran in 5 ms | DB-A02 |"),
            "{report}"
        );
        // Naming another step, or a longer id that merely contains S-01, is
        // not saying so.
        let other = with(
            &early,
            "ran_ms = 5",
            "ran_ms = 5\nran_out_of_order = \"ran before S-011\"",
        );
        assert!(refused(
            &problems_of(&other, &writers()),
            "after",
            "step S-02 is done, but S-01"
        ));
        // last_run belongs to a pending step, ran_out_of_order to a done one.
        let done_partial = problems_of(
            &with(
                &said,
                "ran_ms = 5",
                "ran_ms = 5\nlast_run = \"an earlier try\"",
            ),
            &writers(),
        );
        assert!(refused(
            &done_partial,
            "fields",
            "S-02: only a pending step carries last_run"
        ));
        let pending_order = problems_of(
            &with(
                GOOD,
                "disk = \"4 KiB\"",
                "disk = \"4 KiB\"\nran_out_of_order = \"S-02\"",
            ),
            &writers(),
        );
        assert!(refused(
            &pending_order,
            "fields",
            "S-01: only a done step carries ran_out_of_order"
        ));
    }

    #[test]
    fn the_real_list_passes_against_the_cli_writer_table_and_its_own_history() {
        let (report, passed) = run(REAL, &with_owed());
        assert!(report.contains("owed refusals: 0"), "{report}");
        assert!(report.contains("forms refusals: 0"), "{report}");
        assert!(passed, "{report}");
        assert!(report.contains("fields refusals: 0"));
        assert!(report.contains("leftovers refusals: 0"));
        assert!(report.contains("rehearsal refusals: 0"));
        // The history the flags claim passes; one package more merged than
        // the list says is refused until the list catches up.
        let entries = read(REAL).expect("reads");
        let merged: Vec<&str> = of(&entries, "package")
            .iter()
            .filter(|p| p.flag("merged") == Some(true))
            .filter_map(|p| p.text("name"))
            .collect();
        let subjects: String = merged.iter().map(|m| format!("Merge {m}: x\n")).collect();
        let mut evidence = writers();
        evidence.merged = Some(merged_from(&subjects));
        assert_eq!(check(&entries, &evidence), Vec::<Problem>::new());
        let waiting = of(&entries, "package")
            .iter()
            .find(|p| p.flag("merged") == Some(false))
            .and_then(|p| p.text("name"))
            .expect("the list names an unmerged package");
        evidence.merged = Some(merged_from(&format!("{subjects}Merge {waiting}: lands\n")));
        let p = check(&entries, &evidence);
        assert!(
            refused(&p, "packages", &format!("package {waiting} is merged")),
            "{p:?}"
        );
    }

    /// `GOOD` with S-01's dry run replaced by `line`, judged against the
    /// cli's own write rule.
    fn dry(line: &str, evidence: &Evidence) -> Vec<Problem> {
        let text = with(
            GOOD,
            "dry_run = [\"cli gdfl-import options zip 2018-11-07 2018-11-07 NIFTY BANKNIFTY with BRUTEX_STORE on a private copy\"]",
            &format!("dry_run = [{line:?}]"),
        );
        problems_of(&text, evidence)
    }

    #[test]
    fn a_dry_run_is_read_only_by_the_clis_own_rule_read_in_whole_words() {
        // The three lines the second check passed (verify-R3-36-r1, a1 to
        // a3), each put where it found them in the real list.
        for (from, to) in [
            (
                "dry_run = [\"cli gdfl-rekey --plan\"]",
                "dry_run = [\"cli gdfl-rekey --apply --plan-later\"]",
            ),
            (
                "dry_run = [\"cli zerodha-repair NIFTY 2018-01-01 2026-09-30\"]",
                "dry_run = [\"POST /pull/repair target=indices&vendor=zerodha&granularity=1min&member=NIFTY&from=2020-12-01&to=2020-12-31&repair_reason=x\"]",
            ),
            (
                "dry_run = [\"the same two commands with BRUTEX_STORE on a private copy holding those months\"]",
                "dry_run = [\"cli gdfl-import options zip 2018-11-07 2018-11-07 with BRUTEX_STORE on the real store, not a private copy\"]",
            ),
        ] {
            let p = problems_of(&with(REAL, from, to), &writers());
            assert!(refused(&p, "commands", "dry run"), "{to}: {p:?}");
        }
        for line in [
            "tmutil localsnapshot",
            "rsync -a --delete-after /src /dst",
            "rsync -a --dry-run-later /src /dst",
            "cli gdfl-rekey --apply",
            "cli zerodha-repair NIFTY 2021-02-24 2021-02-24 --from-store x --reason y",
            "cli tix-rebuild gdfl NSE/INDEX/NIFTY 1s 2018-09 2024-07 --dry-run-later",
            "cli srt-build gdfl NSE/INDEX/NIFTY 1s 2018-09 2026-09 --dryrun",
            "cli alerts --ack 3",
            "cli alerts --replace-damaged",
            "cli census-migrate gdfl --seal",
            "cli gdfl-import indices zip 2022-03-07 2022-03-07 --plan",
            "cli gdfl-import indices zip 2022-03-07 2022-03-07 --dry-run",
            "cli gdfl-import indices zip 2022-03-07 2022-03-07 on a private copy",
            "cli gdfl-import x with BRUTEX_STORE on a private copy, then the real store",
            "cli gdfl-import x with BRUTEX_STORE on a private copy holding nothing, never the copy",
            "cli gdfl-import x with BRUTEX_STORE on a private copy of the operator's store",
            "cli gdfl-import x with BRUTEX_STORE on a private copy of runtime/store",
            "cli gdfl-import x BRUTEX_STORE=/real with BRUTEX_STORE on a private copy",
            "cli gdfl-import x with BRUTEX_STORE on a private copy with BRUTEX_STORE on a private copy",
            "cli gdfl-import x with BRUTEX_STORE on a private copyholding",
            "cli gdfl-import x with BRUTEX_STORE on a private copy (on the REAL STORE)",
            "the same two commands",
            "the same two commands on the real store",
            "sh -c 'cli gdfl-import x' with BRUTEX_STORE on a private copy",
            "curl -X POST http://127.0.0.1:8080/pull/repair with BRUTEX_STORE on a private copy",
            "",
            // A shell rewrites these before the cli or rsync reads a word
            // (verify-R3-36-r1, b01 to b03): each hands the program a write.
            "cli gdfl-rekey \"--apply\"",
            "cli gdfl-rekey '--apply'",
            "cli gdfl-rekey --ap\\ply",
            "cli gdfl-rekey {--apply,}",
            "cli gdfl-rekey $ARGS",
            "cli srt-build gdfl NSE/FNO/NIFTY/* 1s 2018-09 2026-09 --dry-run",
            "cli gdfl-rekey --plan # --apply",
            "rsync -a /src /dst # --dry-run",
            "rsync -a \"--dry-run\" /src /dst",
            "rsync -a -- --dry-run /dst",
            // A chain after the private-copy wording runs a second command
            // on whatever store it names (b05), and a newline is a chain.
            "cli gdfl-import x with BRUTEX_STORE on a private copy && cli census-migrate gdfl --seal",
            "cli gdfl-import x with BRUTEX_STORE on a private copy; cli gdfl-rekey --apply",
            "cli gdfl-rekey --plan\ncli gdfl-rekey --apply",
            "cli gdfl-import x with BRUTEX_STORE on a private copy > /x/manifest/gdfl.man",
            "rsync -a --dry-run /src /dst; rsync -a /src /dst",
        ] {
            assert!(
                refused(&dry(line, &writers()), "commands", "dry run"),
                "{line:?}"
            );
        }
        // Each rule refuses by its own reason, so a rule whose test line some
        // other rule also refuses is still seen to hold (verify-R3-36-r2: the
        // newline and the n't form survived their removal).
        for (line, why) in [
            // The newline after the wording and a space: the part before
            // the wording is plain, so only the chain rule sees it.
            (
                "cli gdfl-import x with BRUTEX_STORE on a private copy \ncli gdfl-rekey --apply",
                "\"\\n\" chains or redirects a second command",
            ),
            (
                "cli gdfl-import x with BRUTEX_STORE on a private copy | cli gdfl-rekey --apply",
                "\"|\" chains or redirects a second command",
            ),
            (
                "cli gdfl-import x with BRUTEX_STORE on a private copy ; cli gdfl-rekey --apply",
                "\";\" chains or redirects a second command",
            ),
            (
                "cli gdfl-import x with BRUTEX_STORE on a private copy `cli gdfl-rekey --apply`",
                "\"`\" chains or redirects a second command",
            ),
            (
                "cli gdfl-import x with BRUTEX_STORE on a private copy $(cli gdfl-rekey --apply)",
                "\"$(\" chains or redirects a second command",
            ),
            (
                "cli gdfl-import x with BRUTEX_STORE on a private copy & cli gdfl-rekey --apply",
                "\"&\" chains or redirects a second command",
            ),
            (
                "cli gdfl-import x with BRUTEX_STORE on a private copy >> /x/manifest/gdfl.man",
                "\">\" chains or redirects a second command",
            ),
            (
                "cli gdfl-import x with BRUTEX_STORE on a private copy nor its manifest",
                "\"nor\" after the wording denies it",
            ),
            (
                "cli gdfl-import x with BRUTEX_STORE on a private copy instead",
                "\"instead\" after the wording denies it",
            ),
            (
                "cli gdfl-import x with BRUTEX_STORE on a private copy it never touches",
                "\"never\" after the wording denies it",
            ),
            (
                "cli gdfl-import x with BRUTEX_STORE on a private copy of the real store",
                "it also says \"real store\"",
            ),
            (
                "cli gdfl-import x with BRUTEX_STORE on a private copy of the operator's store",
                "it also says \"operator's store\"",
            ),
            (
                "cli gdfl-import x with BRUTEX_STORE on a private copy of the live store",
                "it also says \"live store\"",
            ),
            (
                "cli gdfl-import x with BRUTEX_STORE on a private copy of runtime/store",
                "it also says \"runtime/store\"",
            ),
            (
                "cli gdfl-import x with BRUTEX_STORE on a private copy with BRUTEX_STORE on a private copy",
                "does not say \" with BRUTEX_STORE on a private copy\" exactly once",
            ),
            (
                "cli gdfl-import x on a private copy",
                "does not say \" with BRUTEX_STORE on a private copy\" exactly once",
            ),
            (
                "cli gdfl-import x BRUTEX_STORE=/real with BRUTEX_STORE on a private copy",
                "it sets BRUTEX_STORE itself",
            ),
            (
                "POST /pull/repair with BRUTEX_STORE on a private copy",
                "a POST or a tmutil line writes, whatever follows it",
            ),
            (
                "tmutil localsnapshot with BRUTEX_STORE on a private copy",
                "a POST or a tmutil line writes, whatever follows it",
            ),
            (
                "rsync -a --dry-run /src /dst # x",
                "'#' is rewritten by a shell before rsync reads its words, so --dry-run may not reach it",
            ),
            (
                "rsync -a --dry-run -z /src /dst",
                "rsync option -z is not one of -a --archive --dry-run --stats -v --verbose -i --itemize-changes -h --human-readable (an option can write a file of its own under --dry-run)",
            ),
            ("cli ", "it names no cli verb"),
            (
                "cli gdfl-rekey '--apply'",
                "'\\'' is rewritten by a shell before the cli reads its words, so the words it shows are not the words the cli receives; it names no read-only form of the cli's rule, and it does not say",
            ),
            (
                "cli gdfl-rekey --apply",
                "cli gdfl-rekey writes with these words; it names no read-only form",
            ),
            (
                "the same two commands",
                "a description of the step's commands runs them on the operator's store unless it says otherwise, and it does not say",
            ),
            (
                "sh -c x",
                "a dry run begins with cli, rsync, planned: or 'the ' (the step's commands in words), never with anything else",
            ),
            (
                "cli gdfl-import x with BRUTEX_STORE on a private copy which isn't used",
                "\"isn't\" after the wording denies it",
            ),
            (
                "cli gdfl-import x with BRUTEX_STORE on a private copy that is NOT used",
                "\"NOT\" after the wording denies it",
            ),
            (
                "cli gdfl-import x with BRUTEX_STORE on a private copy,",
                "runs on into",
            ),
            // An rsync option that writes a file of its own under --dry-run
            // (verify-R3-36-r2 ran --log-file=FILE and saw FILE written).
            (
                "rsync -a --dry-run --log-file=/x/manifest/gdfl.man /src /dst",
                "rsync option --log-file=/x/manifest/gdfl.man is not one of",
            ),
            (
                "rsync -a --dry-run --only-write-batch=/x/b /src /dst",
                "rsync option --only-write-batch=/x/b is not one of",
            ),
            (
                "rsync -a --dry-run --write-batch=/x/b /src /dst",
                "rsync option --write-batch=/x/b is not one of",
            ),
            ("rsync -an /src /dst", "rsync option -an is not one of"),
            (
                "rsync -a --dry-run -e ssh /src host:/dst",
                "rsync option -e is not one of",
            ),
            (
                "rsync -a -- --dry-run /dst",
                "rsync without the option --dry-run writes its destination",
            ),
            (
                "rsync -a /src /dst",
                "rsync without the option --dry-run writes its destination",
            ),
        ] {
            let p = dry(line, &writers());
            assert!(refused(&p, "commands", why), "{line:?}: {p:?}");
        }
        for line in [
            "cli gdfl-rekey --plan",
            "cli gdfl-rekey --plan --underlying NIFTY",
            "cli zerodha-repair NIFTY 2018-01-01 2026-09-30",
            "cli tix-rebuild gdfl NSE/INDEX/NIFTY 1s 2018-09 2024-07 --dry-run",
            "cli srt-build gdfl NSE/INDEX/NIFTY 1s 2018-09 2026-09 --dry-run",
            "cli alerts",
            "cli alerts 3 --ack",
            "cli sweep-stored NIFTY",
            "cli zerodha-repair NIFTY 2021-02-24 2021-02-24 --from-store $BRUTEX_STORE --reason 'r' with BRUTEX_STORE on a private copy holding NIFTY's months",
            "cli gdfl-import indices zip 2022-03-07 2022-03-07 with BRUTEX_STORE on a private copy",
            "rsync -a --dry-run --stats /src /dst",
            "rsync --archive --dry-run --verbose -v -i --itemize-changes -h --human-readable --stats /src /dst",
            "rsync -a --dry-run --stats -- /src -dst",
            "the same two commands with BRUTEX_STORE on a private copy of manifest/ (a rehearsal)",
        ] {
            assert_eq!(dry(line, &writers()), Vec::<Problem>::new(), "{line:?}");
        }
        // Without the cli's table no verb's read-only form is known, so a cli
        // dry run passes only on the private copy, and the report says so.
        let blind = Evidence::default();
        assert!(refused(
            &dry("cli gdfl-rekey --plan", &blind),
            "commands",
            "the cli's writer table was not given, so no read-only form is known; it names no read-only form of the cli's rule, and it does not say"
        ));
        assert_eq!(
            dry(
                "cli gdfl-import x with BRUTEX_STORE on a private copy",
                &blind
            ),
            Vec::<Problem>::new()
        );
        // A planned dry run writes nothing (it is refused beside figures by
        // the rehearsal rule instead).
        assert_eq!(
            dry_run_writes("planned: the restate plan", writers().writers.as_ref()),
            None
        );
        // A space a shell does not split on is no space to it.
        assert!(dry_run_writes("cli gdfl-rekey\u{a0}--plan", writers().writers.as_ref()).is_some());
        // A quoted or expanded word is judged only by its private copy, and
        // the refusal names the character the shell rewrites.
        let quoted = dry("cli gdfl-rekey \"--apply\"", &writers());
        assert!(
            refused(&quoted, "commands", "'\"' is rewritten by a shell"),
            "{quoted:?}"
        );
        assert_eq!(
            dry(
                "cli srt-build gdfl 'NSE/FNO/NIFTY/*' 1s 2018-09 2026-09 --dry-run with BRUTEX_STORE on a private copy",
                &writers()
            ),
            Vec::<Problem>::new()
        );
    }

    #[test]
    fn rehearsal_figures_come_from_a_dry_run_that_ran() {
        // S-01 carries figures; a planned dry run is a form nothing has
        // built, so nothing timed it (verify-R3-36-r1, b07).
        let planned = with(
            GOOD,
            "dry_run = [\"cli gdfl-import options zip 2018-11-07 2018-11-07 NIFTY BANKNIFTY with BRUTEX_STORE on a private copy\"]",
            "dry_run = [\"planned: cli scrub-blocks gdfl\"]",
        );
        let p = problems_of(&planned, &writers());
        assert!(
            refused(
                &p,
                "rehearsal",
                "S-01: rehearsal figures beside a planned dry run"
            ),
            "{p:?}"
        );
        // One planned line among real ones is the same claim.
        let mixed = with(
            GOOD,
            "dry_run = [\"cli gdfl-import options zip 2018-11-07 2018-11-07 NIFTY BANKNIFTY with BRUTEX_STORE on a private copy\"]",
            "dry_run = [\"cli gdfl-import options zip 2018-11-07 2018-11-07 NIFTY BANKNIFTY with BRUTEX_STORE on a private copy\", \"planned: the rest\"]",
        );
        assert!(refused(
            &problems_of(&mixed, &writers()),
            "rehearsal",
            "S-01: rehearsal figures beside a planned dry run"
        ));
        // A step that waits keeps its planned dry run and earns nothing.
        assert_eq!(problems_of(GOOD, &writers()), Vec::<Problem>::new());
    }

    #[test]
    fn a_pending_check_that_reads_a_page_records_what_it_reads_today() {
        let silent = problems_of(
            &with(GOOD, "total above 0 (0 today)", "total above 0"),
            &writers(),
        );
        assert!(
            refused(
                &silent,
                "fields",
                "S-01: check \"GET /bars/window.json -> total above 0\" reads a page and does not record what it reads today"
            ),
            "{silent:?}"
        );
        // The word alone is no record (verify-R3-36-r1, b08), and a page read
        // with curl, a URL or the app's own path is a page read (b09).
        for check in [
            "GET /bars/window.json -> total above 0 (not read today)",
            "GET /bars/window.json -> total above 0 (nothing today)",
            "GET /bars/window.json -> total above 0 (today)",
            "GET /bars/window.json -> total above 0, as today",
            "curl -s 127.0.0.1:8080/bars/window.json -> total above 0",
            "the page http://127.0.0.1:8080/store.json answers held",
            "/db shows one series",
            "and /db lists stock option expiries after 2020-09",
            // A path opened by a parenthesis, a bracket, a brace, an angle
            // bracket, a quote or a backtick, GET joined to its path, wget
            // and https (verify-R3-36-r2: the opener trim survived its
            // removal, and these forms were not seen).
            "(/db shows one series)",
            "[/db] shows one series",
            "{/db} shows one series",
            "</db> shows one series",
            // Written into the list's string, so its quotes are escaped.
            "\\\"/db\\\" shows one series",
            "'/db' shows one series",
            "`/db` shows one series",
            "GET:/bars/window.json answers total above 0",
            "get:/bars/window.json answers total above 0",
            "(GET 127.0.0.1:8080 answers held)",
            "`curl` 127.0.0.1:8080 answers held",
            "wget -qO- 127.0.0.1:8080 answers held",
            "the page https://127.0.0.1:8080 answers held",
        ] {
            let p = problems_of(
                &with(
                    GOOD,
                    "GET /bars/window.json -> total above 0 (0 today)",
                    check,
                ),
                &writers(),
            );
            assert!(
                refused(&p, "fields", "does not record what it reads today"),
                "{check}: {p:?}"
            );
        }
        for check in [
            "GET /bars/window.json -> total above 0 (0 today)",
            "GET /scrub.json answers recorded:true (recorded:false today)",
            "GET /bars/window.json -> missing_months [] (today: 2025-11..2026-08)",
            "GET /x.json answers [] ([] today)",
            "curl /x.json -> total 4,125 (3,750 today)",
            "store imports/gdfl.journal holds a done line for the day",
            "cli tix-rebuild gdfl NSE/INDEX/NIFTY 1s 2018-09 2026-09 --dry-run says Indexed 97",
            // An uppercase path is a file path, never a route of the app,
            // and a word that only begins with get names no page.
            "tmutil listlocalsnapshots /Volumes/WD_BLACK lists the new snapshot",
            "store imports/gdfl.journal is getting a done line",
            "the budget/plan holds",
        ] {
            assert_eq!(
                problems_of(
                    &with(
                        GOOD,
                        "GET /bars/window.json -> total above 0 (0 today)",
                        check
                    ),
                    &writers()
                ),
                Vec::<Problem>::new(),
                "{check}"
            );
        }
        // A done step's check is what its run was read against: no today.
        let mut done = with(
            GOOD,
            "state = \"pending\"\nwrites = \"new month generations\"\ncommand = [\"cli",
            "state = \"done\"\nwrites = \"new month generations\"\ncommand = [\"cli",
        );
        done = with(&done, "total above 0 (0 today)", "total above 0");
        done = with(
            &done,
            "disk = \"4 KiB\"",
            "ran = \"run 1, journal seq 5\"\nran_ms = 9236",
        );
        assert_eq!(problems_of(&done, &writers()), Vec::<Problem>::new());
        // A blank run and a run that took no time are no record of a run.
        let blank = with(&done, "ran = \"run 1, journal seq 5\"", "ran = \" \"");
        assert!(refused(
            &problems_of(&blank, &writers()),
            "fields",
            "S-01: ran is empty"
        ));
        let instant = with(&done, "ran_ms = 9236", "ran_ms = 0");
        assert!(refused(
            &problems_of(&instant, &writers()),
            "fields",
            "S-01: ran_ms is 0"
        ));
    }

    #[test]
    fn a_variable_declared_twice_a_denied_order_and_a_padded_number_are_refused() {
        let twice = with(
            GOOD,
            "[[package]]\nname = \"WP-01\"",
            "[[env]]\nname = \"BRUTEX_STORE\"\nmeans = \"declared again\"\n\n[[package]]\nname = \"WP-01\"",
        );
        assert!(refused(
            &problems_of(&twice, &writers()),
            "env",
            "variable BRUTEX_STORE is declared 2 times"
        ));
        // TOML refuses a leading zero, so the reader does too.
        for padded in ["04096", "0_4096", "00"] {
            let why = read(&with(
                GOOD,
                "rehearsed_bytes = 4096",
                &format!("rehearsed_bytes = {padded}"),
            ))
            .expect_err(padded);
            assert!(why.contains("is not a non-negative integer"), "{why}");
        }
        assert!(read(&with(GOOD, "rehearsed_bytes = 4096", "rehearsed_bytes = 0")).is_ok());
        // S-02 done while S-01, which it follows, is still pending: a
        // sentence that names S-01 while denying the order says nothing.
        let early = with(
            &with(
                GOOD,
                "\"planned: cli gdfl-restate options 2018-11-07 2018-11-07\", # one line per command",
                "\"cli gdfl-import indices zip 2018-09-03 2026-09-24\",",
            ),
            "not_rehearsed = \"waits for WP-02: the door is not built\"\ndisk = \"unknown until rehearsed\"",
            "ran = \"14 Oct, run 7\"\nran_ms = 5",
        );
        let early = with(
            &early,
            "state = \"pending\"\nwrites = \"new month generations\"\ncommand = [\n",
            "state = \"done\"\nwrites = \"new month generations\"\ncommand = [\n",
        );
        for why in [
            "it is not true that this ran before S-01",
            "this never ran before S-01",
            "it isn't so that this ran before S-01",
            "S-01 ran first; this ran after it",
            "ran first. S-01 didn't, before S-01 was due",
            // A clause ends at a full stop and at a semicolon.
            "it ran. before S-01 came the snapshot",
            "it ran; before S-01 came the snapshot",
            // The run of step ids ends at the first word that is not one.
            "this ran before the copy of S-01",
            "before S-01",
        ] {
            let said = with(
                &early,
                "ran_ms = 5",
                &format!("ran_ms = 5\nran_out_of_order = {why:?}"),
            );
            assert!(
                refused(
                    &problems_of(&said, &writers()),
                    "after",
                    "step S-02 is done, but S-01"
                ),
                "{why}"
            );
        }
        for why in [
            "ran before S-01; S-01's run back-fills what this left",
            "the share half ran before S-03 and S-01 (no snapshot taken)",
            "it ran before S-03 or S-01",
            "it ran before S-01's run",
        ] {
            let said = with(
                &early,
                "ran_ms = 5",
                &format!("ran_ms = 5\nran_out_of_order = {why:?}"),
            );
            assert_eq!(
                problems_of(&said, &writers()),
                Vec::<Problem>::new(),
                "{why}"
            );
        }
    }

    /// `GOOD` with a third package, a done step run out of order, a
    /// withdrawn one and a partly run one: every row shape the report has.
    fn every_shape() -> String {
        let text = with(
            GOOD,
            "leftovers = [\"DB-A01\", \"DB-A02\"]",
            "leftovers = [\"DB-A01\", \"DB-A02\", \"DB-A03\", \"DB-A04\", \"DB-A05\"]",
        );
        let text = with(
            &text,
            "[[rule]]",
            "[[package]]\nname = \"WP-03\"\nmerged = true\ntitle = \"another merged package\"\n\n[[rule]]",
        );
        format!(
            "{text}
[[step]]
id = \"S-03\"
title = \"a step run before the one it follows\"
state = \"done\"
writes = \"new month generations\"
command = [\"cli gdfl-import indices zip 2018-09-03 2026-09-24\"]
dry_run = [\"cli gdfl-import indices zip 2018-09-03 2026-09-24 with BRUTEX_STORE on a private copy\"]
prerequisites = [\"WP-03\"]
after = [\"S-01\"]
check = [\"store imports/gdfl.journal holds a done line\"]
closes = [\"DB-A03\", \"DB-A04\"]
ran = \"run 1, journal seq 9\"
ran_ms = 77
ran_out_of_order = \"ran before S-01; S-01's run back-fills what this left\"

[[step]]
id = \"S-04\"
title = \"a step withdrawn\"
state = \"withdrawn\"
writes = \"nothing\"
command = [\"planned: cli gdfl-restate stocks 2018-11-07 2018-11-07\"]
dry_run = [\"planned: the old door's plan\"]
prerequisites = [\"WP-02\"]
check = [\"store imports/gdfl.journal holds nothing new\"]
closes = []
withdrawn_because = \"S-01 does it\"
replaced_by = \"S-01\"

[[step]]
id = \"S-05\"
title = \"a step partly run\"
state = \"pending\"
writes = \"new month generations\"
command = [\"planned: cli gdfl-restate indices 2018-09-03 2018-09-03\", \"cli gdfl-import indices zip 2018-09-03 2026-09-24\"]
dry_run = [\"the same with BRUTEX_STORE on a private copy\"]
prerequisites = [\"WP-02\"]
check = [\"store imports/gdfl.journal holds a done line\"]
closes = [\"DB-A05\"]
last_run = \"run 2 stopped at 2019-05\"
not_rehearsed = \"waits for WP-02\"
disk = \"unknown until rehearsed\"
"
        )
    }

    #[test]
    fn the_report_is_one_table_with_every_count_and_what_was_not_checked() {
        let mut evidence = writers();
        evidence.merged = Some(merged_from("Merge WP-01: one\nMerge WP-03: three\n"));
        let (report, passed) = run(&every_shape(), &evidence);
        assert_eq!(
            report,
            "store steps: 5 (pending 3, done 1, withdrawn 1); rules 1; packages 3 (merged 2); leftovers 5
| step | state | rehearsal | closes |
|---|---|---|---|
| S-01 | pending | 1200 ms, 4096 bytes | DB-A01 |
| S-02 | pending | waits for WP-02 | DB-A02 |
| S-03 | done, out of order | ran in 77 ms | DB-A03 DB-A04 |
| S-04 | withdrawn | - |  |
| S-05 | pending, partly run | waits for WP-02 | DB-A05 |
steps whose command a package has not built yet: 3
fields refusals: 0
leftovers refusals: 0
rehearsal refusals: 0
copies refusals: 0
recount refusals: 0
forms refusals: 0
records refusals: 0
kinds refusals: 0
owed refusals: 0
merge history: 2 package merges read; merged flags checked
decision ledger: D-3317 item 5 read; no census-recount gdfl span may cover 2018-11-06, 2018-11-09
rows marked as needing a store write: none given; board rows outside the leftovers NOT CHECKED
owed runs: not given; NOT CHECKED
"
        );
        assert!(passed);
        // A step with no id or no state is a row of the table still, and the
        // refusal under it says what is missing.
        let anonymous = with(&every_shape(), "id = \"S-05\"\n", "");
        let (report, passed) = run(&anonymous, &evidence);
        assert!(!passed);
        assert!(
            report.contains("\n| ? | pending, partly run | waits for WP-02 | DB-A05 |\n"),
            "{report}"
        );
        assert!(
            report.contains("REFUSED [fields] step at line 91: no id\n"),
            "{report}"
        );
        // An empty id names nothing either, so the step is still named by
        // its line (verify-R3-36-r3: the guard's mutant to `true` printed
        // "step :").
        let empty = with(&every_shape(), "id = \"S-05\"\n", "id = \"\"\n");
        let (report, passed) = run(&empty, &evidence);
        assert!(!passed);
        assert!(
            report.contains("REFUSED [fields] step at line 91: "),
            "{report}"
        );
        assert!(!report.contains("step :"), "{report}");
        let stateless = with(&every_shape(), "state = \"withdrawn\"\n", "");
        let (report, _) = run(&stateless, &evidence);
        assert!(report.contains("\n| S-04 | ? | - |  |\n"), "{report}");
        // Without the history or the cli's table, the report says which
        // checks did not run.
        let (report, _) = run(&every_shape(), &Evidence::default());
        assert!(report.contains("merge history: not given; merged flags NOT CHECKED\n"));
        assert!(report.contains("cli writer table: not given; cli verbs and the writes of check lines NOT CHECKED, and a cli dry run passes only on a private copy\n"));
        evidence.merged = Some(BTreeSet::new());
        let (report, _) = run(&every_shape(), &evidence);
        assert!(report.contains("merge history: names no package merge (a squash-merged history): the merged flags stand as written, NOT CHECKED against git\n"));
    }

    /// The refusals of `text` against the cli's table.
    fn of_text(text: &str) -> Vec<Problem> {
        problems_of(text, &writers())
    }

    #[test]
    fn each_rule_refuses_by_its_own_reason() {
        // A pending step carries no withdrawal and no run of its own.
        for (key, why) in [
            (
                "withdrawn_because = \"x\"",
                "S-01: a pending step is not withdrawn",
            ),
            (
                "replaced_by = \"S-02\"",
                "S-01: a pending step is not withdrawn",
            ),
            (
                "ran_ms = 5",
                "S-01: only a done step carries ran and ran_ms",
            ),
            (
                "ran = \"x\"",
                "S-01: only a done step carries ran and ran_ms",
            ),
        ] {
            let p = of_text(&with(
                GOOD,
                "disk = \"4 KiB\"",
                &format!("disk = \"4 KiB\"\n{key}"),
            ));
            assert!(refused(&p, "fields", why), "{key}: {p:?}");
        }
        let blank = with(
            &every_shape(),
            "ran_out_of_order = \"ran before S-01; S-01's run back-fills what this left\"",
            "ran_out_of_order = \" \"",
        );
        assert!(refused(
            &of_text(&blank),
            "fields",
            "S-03: ran_out_of_order is empty"
        ));
        // Rehearsal figures come together, and beside merged prerequisites.
        let three = of_text(&with(GOOD, "rehearsed_result = \"1 day imported\"\n", ""));
        assert!(refused(
            &three,
            "rehearsal",
            "S-01: rehearsal figures come together: rehearsed_ms, rehearsed_bytes, rehearsed_on, rehearsed_result"
        ));
        let unmerged = of_text(&with(
            GOOD,
            "prerequisites = [\"WP-01\"]",
            "prerequisites = [\"WP-02\", \"WP-77\"]",
        ));
        assert!(refused(
            &unmerged,
            "rehearsal",
            "S-01: figures beside an unmerged prerequisite (WP-02, WP-77)"
        ));
        // The package a step waits for ends at a colon, a space, a comma or
        // a semicolon.
        for waits in [
            "waits for WP-02",
            "waits for WP-02 to land",
            "waits for WP-02, then a rehearsal",
            "waits for WP-02; then a rehearsal",
        ] {
            assert_eq!(
                of_text(&with(GOOD, "waits for WP-02: the door is not built", waits)),
                Vec::<Problem>::new(),
                "{waits}"
            );
        }
        // A variable name is upper case, digits and underscores.
        let env = of_text(&with(
            GOOD,
            "env = [\"BRUTEX_STORE\"]",
            "env = [\"BRUTEX_store\"]",
        ));
        assert!(refused(
            &env,
            "commands",
            "S-01: env \"BRUTEX_store\" is not a variable name"
        ));
        // A done step after a done step needs no word on its order.
        let both = with(
            &with(
                &with(
                    &every_shape(),
                    "state = \"pending\"\nwrites = \"new month generations\"\ncommand = [\"cli",
                    "state = \"done\"\nwrites = \"new month generations\"\ncommand = [\"cli",
                ),
                "disk = \"4 KiB\"",
                "ran = \"run 0\"\nran_ms = 5",
            ),
            "ran_out_of_order = \"ran before S-01; S-01's run back-fills what this left\"\n",
            "",
        );
        assert_eq!(by_rule(&of_text(&both), "after"), Vec::<&Problem>::new());
        // A step replaced by a withdrawn one names nothing that runs.
        let twice = of_text(&with(
            &every_shape(),
            "replaced_by = \"S-01\"",
            "replaced_by = \"S-04\"",
        ));
        assert!(refused(
            &twice,
            "after",
            "step S-04: replaced by S-04, which is withdrawn too"
        ));
        // A withdrawn copy is no copy; one copy among other commands is.
        let withdrawn_copy = with(
            &every_shape(),
            "command = [\"planned: cli gdfl-restate stocks 2018-11-07 2018-11-07\"]",
            "command = [\"cli backup DRIVE\"]",
        );
        assert_eq!(
            by_rule(&of_text(&withdrawn_copy), "copies"),
            Vec::<&Problem>::new()
        );
        let mixed = of_text(&with(
            GOOD,
            "command = [\"cli gdfl-import options zip 2018-11-07 2018-11-07 NIFTY BANKNIFTY\"]",
            "command = [\"cli gdfl-import options zip 2018-11-07 2018-11-07 NIFTY BANKNIFTY\", \"cli backup DRIVE\"]",
        ));
        assert!(refused(
            &mixed,
            "copies",
            "S-01: a copy or snapshot of the store taken while a writer appends is torn"
        ));
        // A row no leftover holds names every step that closes it, and a
        // step with no id as `?`.
        let stray = of_text(&with(
            &with(
                GOOD,
                "closes = [\"DB-A01\"]",
                "closes = [\"DB-A01\", \"DB-A09\"]",
            ),
            "closes = [\"DB-A02\"]",
            "closes = [\"DB-A02\", \"DB-A09\"]",
        ));
        assert!(refused(
            &stray,
            "leftovers",
            "step S-01, S-02 closes DB-A09, which the list of leftovers does not hold"
        ));
        let nameless = of_text(&with(
            &with(&every_shape(), "id = \"S-05\"\n", ""),
            "closes = [\"DB-A05\"]",
            "closes = [\"DB-A09\"]",
        ));
        assert!(
            refused(
                &nameless,
                "leftovers",
                "step ? closes DB-A09, which the list of leftovers does not hold"
            ),
            "{nameless:?}"
        );
        // The cli's table and rule are read in their one form.
        assert!(
            writers_from("pub(crate) const WRITERS: [&str; 3")
                .unwrap_err()
                .contains("the writer table's length is not closed")
        );
        for arm in [
            "!arguments.contains(&\"a\"b\"),",
            "!arguments.contains(&\"a b\"),",
            "!arguments.contains(&\"\"),",
        ] {
            assert_eq!(form_of(arm), None, "{arm}");
        }
        assert_eq!(
            form_of("!arguments.contains(&\"--plan\"),"),
            Some(Form::Unless("--plan".to_owned()))
        );
        // A page read joined to its path, and a value that is a word.
        for (check, records) in [
            ("GET/bars/window.json answers total above 0", false),
            ("GET /x.json answers held (held: today)", false),
            ("GET /x.json answers held (:held today)", false),
            ("GET /x.json answers held (true today)", true),
            ("GET /x.json answers held (false today)", true),
        ] {
            let p = of_text(&with(
                GOOD,
                "GET /bars/window.json -> total above 0 (0 today)",
                check,
            ));
            assert_eq!(
                refused(&p, "fields", "does not record what it reads today"),
                !records,
                "{check}: {p:?}"
            );
        }
    }

    #[test]
    fn row_ids_package_names_and_step_numbers_are_read_in_their_one_form() {
        for id in [
            "DB-A01",
            "SC-CLOSE-21",
            "CMP-03",
            "DB-069",
            "FM-221",
            "R3X-1",
        ] {
            assert!(is_row_id(id), "{id}");
        }
        for id in [
            "", "DB", "DB-", "-A01", "db-A01", "Db-A01", "D_B-A01", "1DB-A01", "DB-a01", "DB-A01-",
            "DB--A01", "DB-A 01",
        ] {
            assert!(!is_row_id(id), "{id}");
        }
        for name in ["WP-02", "WP-12f", "R3-36", "SC-0", "IA-12a"] {
            assert!(is_package_name(name), "{name}");
        }
        for name in [
            "", "WP", "WP-", "wp-02", "Wp-02", "W_P-02", "1WP-02", "WP-0x2", "WP-02ab", "WP-f",
            "WP-02F",
        ] {
            assert!(!is_package_name(name), "{name}");
        }
        for id in ["S-01", "S-32", "S-100"] {
            assert!(is_numbered(id, "S-"), "{id}");
        }
        for id in ["S-1", "S-", "S-0x", "S-x1", "S-011a", "R-01"] {
            assert!(!is_numbered(id, "S-"), "{id}");
        }
    }

    #[test]
    fn a_row_is_closed_or_helped_never_both_and_a_done_command_is_never_planned() {
        // A step that closes a row and also says it only helps it says two
        // things about one row (verify-R3-36-r2: this refusal survived its
        // removal).
        let both = with(
            GOOD,
            "closes = [\"DB-A01\"]",
            "closes = [\"DB-A01\"]\nhelps = [\"DB-A01\"]",
        );
        assert!(
            refused(
                &problems_of(&both, &writers()),
                "fields",
                "S-01: DB-A01 is both closed and helped"
            ),
            "{both}"
        );
        let other = with(
            GOOD,
            "closes = [\"DB-A01\"]",
            "closes = [\"DB-A01\"]\nhelps = [\"DB-A02\"]",
        );
        assert_eq!(problems_of(&other, &writers()), Vec::<Problem>::new());
        // A helped row is a row id, as a closed one is.
        let loose = with(
            GOOD,
            "closes = [\"DB-A01\"]",
            "closes = [\"DB-A01\"]\nhelps = [\"later\"]",
        );
        assert!(refused(
            &problems_of(&loose, &writers()),
            "fields",
            "S-01: \"later\" is not a finding or audit row id"
        ));
        // A done step ran its command, so a planned line in it names a run
        // that cannot have happened (verify-R3-36-r2, followup).
        let done = with(
            &with(
                GOOD,
                "not_rehearsed = \"waits for WP-02: the door is not built\"\ndisk = \"unknown until rehearsed\"",
                "ran = \"14 Oct, run 7\"\nran_ms = 5",
            ),
            "state = \"pending\"\nwrites = \"new month generations\"\ncommand = [\n",
            "state = \"done\"\nwrites = \"new month generations\"\ncommand = [\n",
        );
        let p = problems_of(&done, &writers());
        assert!(
            refused(
                &p,
                "commands",
                "S-02: a done step ran its command, but \"planned: cli gdfl-restate options 2018-11-07 2018-11-07\" is still planned"
            ),
            "{p:?}"
        );
        // The same step with the command that ran earns no such refusal.
        let ran = with(
            &done,
            "\"planned: cli gdfl-restate options 2018-11-07 2018-11-07\", # one line per command",
            "\"cli gdfl-import indices zip 2018-09-03 2026-09-24\",",
        );
        assert!(!refused(
            &problems_of(&ran, &writers()),
            "commands",
            "still planned"
        ));
        // A pending step keeps its planned command while it waits.
        assert_eq!(problems_of(GOOD, &writers()), Vec::<Problem>::new());
    }

    /// GOOD with S-01's one check line replaced by `lines`.
    fn with_checks(lines: &[&str]) -> String {
        let lines: Vec<String> = lines.iter().map(|l| format!("{l:?}")).collect();
        with(
            GOOD,
            "check = [\"GET /bars/window.json -> total above 0 (0 today)\"]",
            &format!("check = [{}]", lines.join(", ")),
        )
    }

    /// The recount refusals of GOOD with S-01's check replaced by `line`.
    fn recount_refusals(line: &str, evidence: &Evidence) -> Vec<String> {
        problems_of(&with_checks(&[line]), evidence)
            .into_iter()
            .filter(|p| p.rule == "recount")
            .map(|p| p.what)
            .collect()
    }

    #[test]
    fn a_recount_span_never_covers_a_day_the_ledger_excludes() {
        // The ledger in this repository: D-3317 item 5 names the two
        // vendor-refused option days of November 2018, and nothing else.
        assert_eq!(
            excluded_from(LEDGER),
            Ok(vec!["2018-11-06".to_owned(), "2018-11-09".to_owned()])
        );
        // The item is read from its `5. ` line to the first line not indented
        // under it, inside the D-3317 entry only: the heading's own date, an
        // earlier entry's item 5, the next item and the next entry are not
        // read.
        let ledger = "### D-3316 — a\n5. **x** 2017-01-01\n### D-3317 — b — 2026-10-07\n\n1. **y** 2017-02-02\n5. **z** closes (2018-11-06\n   and 2018-11-09).\n6. **w** 2018-12-01\n   2018-12-02\n### D-3318 — c\n5. 2019-01-01\n";
        assert_eq!(
            excluded_from(ledger),
            Ok(vec!["2018-11-06".to_owned(), "2018-11-09".to_owned()])
        );
        let ended = "### D-3317 — b\n5. **z** 2018-11-06\n\n   2018-11-07\n";
        assert_eq!(excluded_from(ended), Ok(vec!["2018-11-06".to_owned()]));
        assert_eq!(
            excluded_from("### D-3316 — a\n5. 2018-11-06\n"),
            Err("the decision ledger holds no entry beginning \"### D-3317 \"".to_owned())
        );
        assert_eq!(
            excluded_from("### D-3317 — b\n4. 2018-11-06\n## next\n5. 2018-11-07\n"),
            Err("### D-3317 holds no item beginning \"5. \"".to_owned())
        );
        assert_eq!(
            excluded_from("### D-3317 — b\n5. no day here, D-3317 and 2018-1-06\n"),
            Err("item \"5. \" of ### D-3317 names no YYYY-MM-DD day".to_owned())
        );
        for day in ["2018-11-06", "0000-00-00"] {
            assert!(is_day(day), "{day}");
        }
        for word in [
            "",
            "2018-11-6",
            "2018/11/06",
            "20181-1-06",
            "2018-11-06x",
            "x018-11-06",
            "2018-1a-06",
            "2018-11-0-",
            "2018-11--6",
            "-2018-11-0",
        ] {
            assert!(!is_day(word), "{word}");
        }

        // The words of a recount, as the cli reads them.
        let span = |vendor, from, to| Ok(Span { vendor, from, to });
        assert_eq!(
            span_of("gdfl --from 2018-09-03 --to 2018-11-05, on the restored store"),
            span("gdfl", Some("2018-09-03"), Some("2018-11-05"))
        );
        assert_eq!(
            span_of("gdfl --to 2018-11-05 --from 2018-09-03"),
            span("gdfl", Some("2018-09-03"), Some("2018-11-05"))
        );
        assert_eq!(span_of("gdfl, on the store"), span("gdfl", None, None));
        assert_eq!(span_of(""), span("", None, None));
        // Prose punctuation after a day ends the command: a `--to` after it
        // is a word of the sentence, not of the cli, even where it follows
        // at once.
        assert_eq!(
            span_of("gdfl --from 2018-11-10, then --to 2018-11-05"),
            span("gdfl", Some("2018-11-10"), None)
        );
        assert_eq!(
            span_of("gdfl --from 2018-11-10, --to 2018-11-05"),
            span("gdfl", Some("2018-11-10"), None)
        );
        assert_eq!(
            span_of("zerodha --from 2015-01-01 (the same)"),
            span("zerodha", Some("2015-01-01"), None)
        );
        assert_eq!(
            span_of("gdfl --from 2018-9-3"),
            Err("--from \"2018-9-3\" is not a YYYY-MM-DD day".to_owned())
        );
        assert_eq!(
            span_of("gdfl --from"),
            Err("--from \"\" is not a YYYY-MM-DD day".to_owned())
        );
        assert_eq!(
            span_of("gdfl --to 2018-11-05 --to 2018-11-08"),
            Err("--to is given twice".to_owned())
        );

        // verify-R3-36-r3, blocking 1: the line the restore read-back carried
        // covers both days and is refused once for each.
        let evidence = writers();
        let old = "cli census-recount gdfl --from 2018-09-03 --to 2026-09-30, on a store restored from the snapshot and before any writer starts (the same for zerodha): every count equals the census the snapshot holds";
        let p = recount_refusals(old, &evidence);
        assert_eq!(p.len(), 2, "{p:?}");
        assert!(p[0].ends_with(": the span covers 2018-11-06, which D-3317 item 5 excludes: the recount would close it as recounted and no import would retry its refused files"), "{p:?}");
        assert!(
            p[0].starts_with("step S-01: check \"cli census-recount gdfl --from 2018-09-03"),
            "{p:?}"
        );
        assert!(p[1].contains("the span covers 2018-11-09, which"), "{p:?}");
        // Exactly the two days (the attack span of the check), each bound
        // inclusive, an open end, and the spans either side.
        for (words, covered) in [
            (
                "--from 2018-11-06 --to 2018-11-09",
                vec!["2018-11-06", "2018-11-09"],
            ),
            ("--from 2018-11-06 --to 2018-11-06", vec!["2018-11-06"]),
            ("--from 2018-11-09 --to 2018-11-09", vec!["2018-11-09"]),
            ("--from 2018-09-03 --to 2018-11-06", vec!["2018-11-06"]),
            ("--from 2018-11-09", vec!["2018-11-09"]),
            ("--from 2018-09-03", vec!["2018-11-06", "2018-11-09"]),
            ("--from 2018-09-03 --to 2018-11-05", vec![]),
            ("--from 2018-11-07 --to 2018-11-08", vec![]),
            ("--from 2018-11-10", vec![]),
            ("--from 2018-11-10 --to 2026-09-30", vec![]),
        ] {
            let p = recount_refusals(&format!("cli census-recount gdfl {words}"), &evidence);
            let days: Vec<&str> = covered
                .iter()
                .filter(|d| {
                    p.iter()
                        .any(|w| w.contains(&format!("the span covers {d},")))
                })
                .copied()
                .collect();
            assert_eq!(
                (p.len(), days),
                (covered.len(), covered.clone()),
                "{words}: {p:?}"
            );
        }
        // The recount closes journal days for gdfl alone, so a Zerodha span
        // may run over the two days; every vendor's still needs --from.
        assert_eq!(
            recount_refusals("cli census-recount zerodha --from 2015-01-01", &evidence),
            Vec::<String>::new()
        );
        for line in [
            "cli census-recount zerodha --to 2026-09-30",
            "cli census-recount gdfl, on the restored store",
        ] {
            let p = recount_refusals(line, &evidence);
            assert_eq!(p.len(), 1, "{p:?}");
            assert!(p[0].ends_with(": a recount with no --from is refused by the cli, and a span with no start closes every refused day before its end (D-3317)"), "{p:?}");
        }
        let p = recount_refusals(
            "cli census-recount gdfl --from 2018-11-10 --to 2018-11-07",
            &evidence,
        );
        assert_eq!(p.len(), 1, "{p:?}");
        assert!(
            p[0].ends_with(": --to 2018-11-07 is before --from 2018-11-10"),
            "{p:?}"
        );
        // A day on the --from boundary is not before it.
        assert_eq!(
            recount_refusals(
                "cli census-recount gdfl --from 2018-11-10 --to 2018-11-10",
                &evidence
            ),
            Vec::<String>::new()
        );
        let p = recount_refusals("cli census-recount gdfl --from 2018-11-1", &evidence);
        assert_eq!(p.len(), 1, "{p:?}");
        assert!(
            p[0].ends_with(": --from \"2018-11-1\" is not a YYYY-MM-DD day"),
            "{p:?}"
        );
        // Anywhere in a line, and every one a line names.
        let p = recount_refusals(
            "after the restore, cli census-recount gdfl --from 2018-11-06 --to 2018-11-06 and then cli census-recount gdfl --from 2018-11-09",
            &evidence,
        );
        assert_eq!(p.len(), 2, "{p:?}");
        // Without the ledger a GDFL span is NOT CHECKED, which is refused,
        // and a Zerodha one is not judged by it.
        let blind = Evidence {
            writers: evidence.writers.clone(),
            ..Evidence::default()
        };
        let p = recount_refusals("cli census-recount gdfl --from 2018-11-10", &blind);
        assert_eq!(p.len(), 1, "{p:?}");
        assert!(p[0].ends_with(": the days D-3317 item 5 excludes were not read (give --decisions docs/05-decisions.md), so this span is NOT CHECKED"), "{p:?}");
        assert_eq!(
            recount_refusals("cli census-recount zerodha --from 2015-01-01", &blind),
            Vec::<String>::new()
        );
        // A command and a dry run are read as a check is.
        let command = with(
            GOOD,
            "command = [\"cli gdfl-import options zip 2018-11-07 2018-11-07 NIFTY BANKNIFTY\"]",
            "command = [\"cli census-recount gdfl --from 2018-11-09\"]",
        );
        assert!(refused(
            &problems_of(&command, &evidence),
            "recount",
            "step S-01: command \"cli census-recount gdfl --from 2018-11-09\": the span covers 2018-11-09"
        ));
        let dry = with(
            GOOD,
            "dry_run = [\"cli gdfl-import options zip 2018-11-07 2018-11-07 NIFTY BANKNIFTY with BRUTEX_STORE on a private copy\"]",
            "dry_run = [\"cli census-recount gdfl --from 2018-11-06 with BRUTEX_STORE on a private copy\"]",
        );
        assert_eq!(by_rule(&problems_of(&dry, &evidence), "recount").len(), 2);
        // A withdrawn step runs nothing, so its words are not judged.
        let entries = read(&with(
            &with_checks(&["cli census-recount gdfl --from 2018-11-06"]),
            "state = \"pending\"\nwrites = \"new month generations\"\ncommand = [\"cli gdfl-import",
            "state = \"withdrawn\"\nwrites = \"new month generations\"\ncommand = [\"cli gdfl-import",
        ))
        .expect("the list reads");
        let step = of(&entries, "step")[0];
        assert_eq!(step.text("state"), Some("withdrawn"));
        let mut out = Vec::new();
        recounts(step, &evidence, &mut out);
        assert_eq!(out, Vec::<Problem>::new());
        // The report counts them.
        let (report, passed) = run(&with_checks(&[old]), &evidence);
        assert!(!passed);
        assert!(report.contains("\nrecount refusals: 2\n"), "{report}");
        let (report, _) = run(GOOD, &Evidence::default());
        assert!(
            report.contains("\ndecision ledger: not given; every census-recount gdfl span is refused as NOT CHECKED\n"),
            "{report}"
        );

        // The list in this repository: the two restore read-backs recount
        // GDFL in three spans around the two days and Zerodha from its oldest
        // month, and no line anywhere covers an excluded day.
        let entries = read(REAL).expect("config/store-steps.toml reads");
        assert_eq!(
            by_rule(&check(&entries, &evidence), "recount"),
            Vec::<&Problem>::new()
        );
        for id in ["S-01", "S-17"] {
            let step = of(&entries, "step")
                .into_iter()
                .find(|s| s.text("id") == Some(id))
                .expect("the step is in the list");
            let spans: Vec<(&str, Option<&str>, Option<&str>)> = step
                .list("check")
                .iter()
                .filter_map(|l| l.strip_prefix(RECOUNT))
                .map(|rest| span_of(rest).expect("the span reads"))
                .map(|s| (s.vendor, s.from, s.to))
                .collect();
            assert_eq!(
                spans,
                [
                    ("gdfl", Some("2018-09-03"), Some("2018-11-05")),
                    ("gdfl", Some("2018-11-07"), Some("2018-11-08")),
                    ("gdfl", Some("2018-11-10"), None),
                    ("zerodha", Some("2015-01-01"), None),
                ],
                "{id}"
            );
        }
    }

    #[test]
    fn a_check_that_runs_a_writer_says_it_writes() {
        let evidence = writers();
        let checks = |lines: &[&str]| -> Vec<String> {
            problems_of(&with_checks(lines), &evidence)
                .into_iter()
                .filter(|p| p.rule == "checks")
                .map(|p| p.what)
                .collect()
        };
        let p = checks(&["cli scrub-blocks gdfl on the restored store: 0 bad"]);
        assert_eq!(
            p,
            [
                "step S-01: check \"cli scrub-blocks gdfl on the restored store: 0 bad\" runs cli scrub-blocks, which writes the store with these words: say \"this check writes \" and what it writes, so it is run as the write it is"
            ]
        );
        assert_eq!(
            checks(&[
                "cli scrub-blocks gdfl on the restored store (this check writes its log and cursor): 0 bad"
            ]),
            Vec::<String>::new()
        );
        // The cli's own read-only forms read, and prose punctuation after a
        // word does not hide one.
        assert_eq!(
            checks(&[
                "cli tix-rebuild gdfl NSE/INDEX/NIFTY 1s 2018-09 2026-09 --dry-run says Indexed 97",
                "cli tix-rebuild gdfl NSE/INDEX/NIFTY 1s 2018-09 2026-09 --dry-run, says Indexed 97",
                "cli gdfl-rekey --plan judges 0 to move",
                "cli audit-stored NIFTY",
                "GET /x answers 0 (0 today)",
            ]),
            Vec::<String>::new()
        );
        let p = checks(&[
            "cli gdfl-rekey --apply, then the plan",
            "cli tix-rebuild gdfl NSE/INDEX/NIFTY 1s 2018-09 2026-09 says Indexed 97",
        ]);
        assert_eq!(p.len(), 2, "{p:?}");
        assert!(p[0].contains("runs cli gdfl-rekey, which writes"), "{p:?}");
        assert!(p[1].contains("runs cli tix-rebuild, which writes"), "{p:?}");
        // Without the cli's table no verb is known, so nothing is judged and
        // the report says so.
        let blind = Evidence {
            excluded: evidence.excluded.clone(),
            ..Evidence::default()
        };
        assert!(
            by_rule(
                &problems_of(&with_checks(&["cli scrub-blocks gdfl"]), &blind),
                "checks"
            )
            .is_empty()
        );
        let (report, _) = run(GOOD, &blind);
        assert!(
            report.contains(
                "cli writer table: not given; cli verbs and the writes of check lines NOT CHECKED"
            ),
            "{report}"
        );
        // The list in this repository: every writing check says so.
        let entries = read(REAL).expect("config/store-steps.toml reads");
        assert_eq!(
            by_rule(&check(&entries, &evidence), "checks"),
            Vec::<&Problem>::new()
        );
    }

    #[test]
    fn a_command_runs_as_written_and_a_copy_removes_nothing() {
        let evidence = writers();
        let command = |line: &str| -> Vec<String> {
            problems_of(
                &with(
                    GOOD,
                    "command = [\"cli gdfl-import options zip 2018-11-07 2018-11-07 NIFTY BANKNIFTY\"]",
                    &format!("command = [{line:?}]"),
                ),
                &evidence,
            )
            .into_iter()
            .filter(|p| p.rule == "commands")
            .map(|p| p.what)
            .collect()
        };
        assert_eq!(
            command("cli gdfl-import options zip 2018-11-07 2018-11-07; cli gdfl-rekey --apply"),
            [
                "step S-01: command \"cli gdfl-import options zip 2018-11-07 2018-11-07; cli gdfl-rekey --apply\": \";\" chains or redirects a second command"
            ]
        );
        for (line, c) in [
            (
                "cli gdfl-import options zip 2018-11-07 2018-11-07 > out",
                ">",
            ),
            ("cli gdfl-import options zip <a b> 2018-11-07", ">"),
            ("cli gdfl-import options zip <> 2018-11-07", ">"),
            ("planned: cli snapshot take && cli gdfl-rekey --apply", "&"),
            ("planned: cli backup A B | tee x", "|"),
            ("cli gdfl-import options zip $(date) 2018-11-07", "$("),
            (
                "cli gdfl-import options zip 2018-11-07\ncli gdfl-rekey --apply",
                "\n",
            ),
        ] {
            let p = command(line);
            assert!(
                p.iter()
                    .any(|w| w.ends_with(&format!(": {c:?} chains or redirects a second command"))),
                "{line:?}: {p:?}"
            );
        }
        // A placeholder's `|` and brackets are the operator's, and a declared
        // variable is the step's own.
        assert_eq!(
            command(
                "cli zerodha-repair <SYMBOL> 2018-01-01 2019-11-30 --segment <INDEX|CASH> --from-store $BRUTEX_STORE --reason 'S-16 refold', for each series"
            ),
            Vec::<String>::new()
        );
        assert_eq!(
            command("cli gdfl-import x ${BRUTEX_STORE}"),
            Vec::<String>::new()
        );
        assert_eq!(
            command("cli gdfl-import x $OTHER_1"),
            [
                "step S-01: command \"cli gdfl-import x $OTHER_1\" uses $OTHER_1, which the step's env does not declare"
            ]
        );
        assert_eq!(
            command("planned: cli snapshot take $"),
            [
                "step S-01: command \"planned: cli snapshot take $\" uses $, which the step's env does not declare",
                "step S-01: every prerequisite is met, but the command is still planned: write the real one"
            ]
        );
        // A form's body joins its fields with `&`, and a planned line runs
        // nothing yet.
        assert!(
            !command("planned: cli gdfl-restate options 2018-11-07 2018-11-07; then the check")
                .iter()
                .any(|w| w.contains("chains"))
        );
        assert!(
            !command("POST /pull/repair target=indices&vendor=zerodha")
                .iter()
                .any(|w| w.contains("chains"))
        );
        for o in [
            "--delete",
            "--delete-after",
            "--del",
            "--remove-source-files",
            "--force",
        ] {
            assert_eq!(
                command(&format!("rsync -a {o} A B")),
                [
                    format!(
                        "step S-01: command \"rsync -a {o} A B\" begins with none of cli | POST | planned: cli | planned: POST "
                    ),
                    format!(
                        "step S-01: command \"rsync -a {o} A B\": rsync {o} removes files, and a copy of an append-only store removes none"
                    ),
                ]
            );
        }
        // After `--`, a word is a file name.
        assert_eq!(
            command("rsync -a A B -- --delete"),
            [
                "step S-01: command \"rsync -a A B -- --delete\" begins with none of cli | POST | planned: cli | planned: POST "
            ]
        );

        assert_eq!(outside_placeholders("a <B|C> d"), "a NAME d");
        assert_eq!(outside_placeholders("<A><B>"), "NAMENAME");
        assert_eq!(outside_placeholders("a <b c> d"), "a <b c> d");
        assert_eq!(outside_placeholders("x <> y"), "x <> y");
        assert_eq!(outside_placeholders("x <y"), "x <y");
        assert_eq!(outside_placeholders("x <a<b> y"), "x <aNAME y");
        assert_eq!(outside_placeholders("é <Á b> é"), "é <Á b> é");
        assert_eq!(outside_placeholders("é <A> é"), "é NAME é");
        assert_eq!(variables("$A ${B_1} $ $C-d x"), ["A", "B_1", "", "C"]);
        assert_eq!(variables("no variable"), Vec::<&str>::new());

        // The list in this repository runs every command as written.
        let entries = read(REAL).expect("config/store-steps.toml reads");
        assert_eq!(
            by_rule(&check(&entries, &evidence), "commands"),
            Vec::<&Problem>::new()
        );
    }

    #[test]
    fn a_figure_nobody_measured_and_a_denial_in_any_spelling_are_refused() {
        let zero = with(GOOD, "rehearsed_ms = 1_200", "rehearsed_ms = 0");
        assert!(refused(
            &problems_of(&zero, &writers()),
            "rehearsal",
            "S-01: rehearsed_ms = 0: write the time the dry run took"
        ));
        let one = with(GOOD, "rehearsed_ms = 1_200", "rehearsed_ms = 1");
        assert_eq!(problems_of(&one, &writers()), Vec::<Problem>::new());

        let copy = "cli x with BRUTEX_STORE on a private copy";
        for (tail, word) in [
            (" which isn\u{2019}t used", "isn\u{2019}t"),
            (" which ISN'T used", "ISN'T"),
            (" (really? No: the one the app reads)", "No"),
            (" so: no.", "no"),
            (" so: no", "no"),
            (" (no)", "no"),
        ] {
            assert_eq!(
                private_copy(&format!("{copy}{tail}")),
                Err(format!("{word:?} after the wording denies it")),
                "{tail:?}"
            );
        }
        // A `no` that qualifies a noun denies nothing.
        assert_eq!(private_copy(&format!("{copy} with no app running")), Ok(()));
        assert_eq!(private_copy(&format!("{copy} and nothing else")), Ok(()));
        assert_eq!(private_copy(&format!("{copy} (no app)")), Ok(()));
        for spelling in [
            "runtime//store",
            "runtime/./store",
            "runtime/.//./store",
            "RUNTIME///STORE",
        ] {
            assert_eq!(
                private_copy(&format!("{copy} of {spelling}")),
                Err("it also says \"runtime/store\"".to_owned()),
                "{spelling}"
            );
        }
        assert_eq!(private_copy(&format!("{copy} of runtime/.store")), Ok(()));
        assert_eq!(squeezed("a//b/./c///d/././e"), "a/b/c/d/e");
        assert_eq!(squeezed("x/.y/..z"), "x/.y/..z");
        assert_eq!(denial("a no b"), None);
        assert_eq!(denial("é no:"), Some("no"));
        assert_eq!(denial("éé no b, never"), Some("never"));
        assert_eq!(denial(""), None);

        // A clause that says a run came first is not denied by a `no` that
        // qualifies a noun, and is by one that answers.
        assert!(says_ran_before(
            "ran before S-01: no snapshot was taken",
            "S-01"
        ));
        assert!(!says_ran_before("it ran before S-01? No", "S-01"));
        assert!(says_ran_before(
            "it ran before S-01\u{2019}s snapshot",
            "S-01"
        ));
        assert!(!says_ran_before(
            "it isn\u{2019}t so that this ran before S-01",
            "S-01"
        ));
    }

    // ------------------------------------------------- R4-06 (AUT-07) --

    /// The owed runs CI hands the tool with `--owed`.
    const OWED: &str = include_str!("../config/owed-runs.toml");

    /// [`writers`] with the tracked owed runs.
    fn with_owed() -> Evidence {
        Evidence {
            owed: Some(owed_from(OWED).expect("config/owed-runs.toml reads")),
            ..writers()
        }
    }

    /// An owed-runs file holding `ids`.
    fn owed_file(ids: &[&str]) -> String {
        let mut s = String::from("format = 1\n");
        for id in ids {
            s.push_str(&format!(
                "\n[[owed]]\nid = {id:?}\nwhat = \"a run\"\nsource = [\"census\"]\n"
            ));
        }
        s
    }

    /// **EVERY OWED CENSUS RUN IS NAMED BY ONE STEP** (AUT-07): the tracked
    /// list names each of the tracked owed runs by exactly one step; an owed
    /// run that is no leftover, an `OWED-` leftover no owed run holds, one no
    /// step closes and one two steps close are each refused by name; and an
    /// owed file that is empty, renumbered twice or of another table is
    /// refused whole.
    #[test]
    fn every_owed_census_run_is_named_by_one_step() {
        let ids = owed_from(OWED).expect("reads");
        assert!(ids.len() >= 31, "{ids:?}");
        let entries = read(REAL).expect("reads");
        let mut closers: BTreeMap<&str, usize> = BTreeMap::new();
        for step in of(&entries, "step") {
            if step.text("state") == Some("withdrawn") {
                continue;
            }
            for row in step.list("closes") {
                *closers.entry(row.as_str()).or_default() += 1;
            }
        }
        for id in &ids {
            assert_eq!(
                closers.get(id.as_str()),
                Some(&1),
                "{id}: closed by one step"
            );
        }
        let (report, passed) = run(REAL, &with_owed());
        assert!(passed, "{report}");
        assert!(report.contains(&format!(
            "owed runs: {} read from config/owed-runs.toml",
            ids.len()
        )));
        // A FIXTURE: GOOD's two leftovers, plus owed runs.
        let owed = |ids: &[&str]| Evidence {
            owed: Some(owed_from(&owed_file(ids)).expect("reads")),
            ..writers()
        };
        let listed = with(
            GOOD,
            "leftovers = [\"DB-A01\", \"DB-A02\"]",
            "leftovers = [\"DB-A01\", \"DB-A02\", \"OWED-01\"]",
        );
        let closed = with(
            &listed,
            "closes = [\"DB-A01\"]",
            "closes = [\"DB-A01\", \"OWED-01\"]",
        );
        assert_eq!(
            problems_of(&closed, &owed(&["OWED-01"])),
            Vec::<Problem>::new()
        );
        // An owed run that is no leftover.
        let p = problems_of(&closed, &owed(&["OWED-01", "OWED-02"]));
        assert!(
            refused(&p, "owed", "owed run OWED-02 is not a leftover"),
            "{p:?}"
        );
        // No owed list given: not checked, and the report says so.
        assert!(by_rule(&problems_of(&closed, &writers()), "owed").is_empty());
        // An OWED- leftover that no owed run holds.
        let p = problems_of(
            &with(&closed, "\"OWED-01\"]", "\"OWED-01\", \"OWED-03\"]"),
            &owed(&["OWED-01"]),
        );
        assert!(
            refused(&p, "owed", "leftover OWED-03 is no owed run"),
            "{p:?}"
        );
        // No step closes it, or two do.
        let p = problems_of(&listed, &owed(&["OWED-01"]));
        assert!(refused(&p, "leftovers", "OWED-01"), "{p:?}");
        let twice = with(
            &closed,
            "closes = [\"DB-A02\"]",
            "closes = [\"DB-A02\", \"OWED-01\"]",
        );
        let p = problems_of(&twice, &owed(&["OWED-01"]));
        assert!(refused(&p, "leftovers", "OWED-01"), "{p:?}");
        // The steps list holds no [[owed]] table of its own.
        let inside =
            format!("{closed}\n[[owed]]\nid = \"OWED-01\"\nwhat = \"x\"\nsource = [\"y\"]\n");
        assert!(refused(
            &problems_of(&inside, &owed(&["OWED-01"])),
            "owed",
            "owed runs live in config/owed-runs.toml"
        ));
        // The owed file itself.
        assert!(
            owed_from("format = 1\n")
                .unwrap_err()
                .contains("an empty list proves nothing")
        );
        assert!(
            owed_from(&owed_file(&["OWED-01", "OWED-01"]))
                .unwrap_err()
                .contains("OWED-01 is given twice")
        );
        assert!(
            owed_from(&owed_file(&["OWED-1"]))
                .unwrap_err()
                .contains("is not OWED-<two or more digits>")
        );
        assert!(owed_from("format = 1\n\n[[step]]\nid = \"S-01\"\n").is_err());
        let (report, _) = run(&closed, &owed(&["OWED-01"]));
        assert!(
            report.contains("owed runs: 1 read from config/owed-runs.toml"),
            "{report}"
        );
    }

    /// **EVERY STORE STEP COMMAND IS A CLI VERB OR A POST** (AUT-07): a bare
    /// `cli <verb>` or `POST <route>`, built or planned; never a script, a
    /// system tool, prose, a build path, an absolute path under the work
    /// folder, or a loop written in words. The tracked list holds none.
    #[test]
    fn every_store_step_command_is_a_cli_verb_or_a_post() {
        let entries = read(REAL).expect("reads");
        for step in of(&entries, "step") {
            for line in step.list("command") {
                assert!(
                    HEADS.iter().any(|h| line.starts_with(h)),
                    "{}: {line:?}",
                    step.label()
                );
            }
        }
        let p = problems_of(REAL, &with_owed());
        assert!(by_rule(&p, "forms").is_empty(), "{p:?}");
        assert!(by_rule(&p, "commands").is_empty(), "{p:?}");
        let at =
            "command = [\"cli gdfl-import options zip 2018-11-07 2018-11-07 NIFTY BANKNIFTY\"]";
        let one = |line: &str| {
            problems_of(
                &with(GOOD, at, &format!("command = [{line:?}]")),
                &writers(),
            )
        };
        for (line, rule, says) in [
            (
                "bash state/data-census-20261013/cli.sh gdfl-import",
                "commands",
                "begins with none of",
            ),
            (
                "state/data-census-20261013/cli.sh gdfl-import options zip",
                "forms",
                "holds \".sh\"",
            ),
            (
                "cli gdfl-import options zip 2018-11-07 2018-11-07 --scratch state/x",
                "forms",
                "holds \"state/\"",
            ),
            ("tmutil localsnapshot", "commands", "begins with none of"),
            (
                "planned: the restate door",
                "commands",
                "begins with none of",
            ),
            (
                "planned: R3-22's restate door for option months",
                "commands",
                "begins with none of",
            ),
            (
                "cli gdfl-import options zip <DAY> <DAY>, once for each of 2018-11-07 2019-10-27",
                "forms",
                "says \"each\"",
            ),
            (
                "cli gdfl-import options zip 2018-11-07 2018-11-07 for every underlying",
                "forms",
                "says \"every\"",
            ),
            (
                "tgt/app18/release/cli gdfl-import options zip 2018-11-07 2018-11-07",
                "forms",
                "holds \"tgt/\"",
            ),
            (
                "cli gdfl-import options zip 2018-11-07 2018-11-07 --from /Volumes/WD_BLACK/brutex/work-20260925/x",
                "forms",
                "names the path",
            ),
            (
                "POST /pull/repair target=indices /Volumes/WD_BLACK/x",
                "forms",
                "names the path",
            ),
        ] {
            let p = one(line);
            assert!(refused(&p, rule, says), "{line}: {p:?}");
        }
        // The forms the list is written in pass: a range verb, one line a
        // day, a POST of the app's route, a quoted '&' or 'each' that is the
        // program's own word, a planned cli verb (WP-02 is unmerged).
        for line in [
            "cli gdfl-import options zip 2018-11-07 2018-11-07 NIFTY 'GVT&D'",
            "cli zerodha-repair NIFTY 2021-02-24 2021-02-24 --from-store $BRUTEX_STORE --reason 'refold each rung'",
            "POST /pull/repair target=indices&vendor=zerodha&member=NIFTY&from=2019-04-01&to=2019-04-30",
        ] {
            let p = one(line);
            assert!(
                by_rule(&p, "forms").is_empty() && !refused(&p, "commands", "begins with none of"),
                "{line}: {p:?}"
            );
        }
        let planned = with(
            GOOD,
            "\"planned: cli gdfl-restate options 2018-11-07 2018-11-07\",",
            "\"planned: POST /restate days=2018-11-07\",",
        );
        assert!(by_rule(&problems_of(&planned, &writers()), "forms").is_empty());
        assert!(!refused(
            &problems_of(&planned, &writers()),
            "commands",
            "begins with none of"
        ));
    }

    /// **A RUN STEP CARRIES ITS RUN ID AND ITS CHECK** (AUT-07): a done
    /// step's `ran` and a pending step's `last_run` name the run id or the
    /// store record the run was read from; a step without a check is
    /// refused; and the tracked list's every run record passes.
    #[test]
    fn a_run_step_carries_its_run_id_and_check() {
        let p = problems_of(REAL, &with_owed());
        assert!(by_rule(&p, "records").is_empty(), "{p:?}");
        let done = with(
            GOOD,
            "not_rehearsed = \"waits for WP-02: the door is not built\"\ndisk = \"unknown until rehearsed\"",
            "ran = \"14 Oct, run 1791943060\"\nran_ms = 5",
        );
        let done = with(
            &done,
            "state = \"pending\"\nwrites = \"new month generations\"\ncommand = [\n",
            "state = \"done\"\nwrites = \"new month generations\"\ncommand = [\n",
        );
        let done = with(
            &done,
            "\"planned: cli gdfl-restate options 2018-11-07 2018-11-07\",",
            "\"cli gdfl-import options zip 2018-11-08 2018-11-08\",",
        );
        let done = with(
            &done,
            "prerequisites = [\"WP-02\"]",
            "prerequisites = [\"WP-01\"]",
        );
        assert!(by_rule(&problems_of(&done, &writers()), "records").is_empty());
        for ran in ["14 Oct, by the main session", "run of 14 Oct", "runs many"] {
            let p = problems_of(&with(&done, "14 Oct, run 1791943060", ran), &writers());
            assert!(
                refused(&p, "records", "ran names no run id and no store record"),
                "{ran}: {p:?}"
            );
        }
        for ran in [
            "imports/gdfl.journal lines 1 to 9",
            "logs/cli/audit.ndjson 'run finished'",
            "runs 7 and 8",
            "(run 12)",
        ] {
            let p = problems_of(&with(&done, "14 Oct, run 1791943060", ran), &writers());
            assert!(by_rule(&p, "records").is_empty(), "{ran}: {p:?}");
        }
        let partly = with(
            GOOD,
            "disk = \"unknown until rehearsed\"",
            "disk = \"unknown until rehearsed\"\nlast_run = \"14 Oct: 10 of 13 keys moved\"",
        );
        assert!(refused(
            &problems_of(&partly, &writers()),
            "records",
            "last_run names no run id"
        ));
        // NO CHECK, NO STEP: the key is required and an empty one is refused.
        let unchecked = with(GOOD, "check = [\"GET /x (0 today)\"]\n", "");
        assert!(read(&unchecked).is_err() || !problems_of(&unchecked, &writers()).is_empty());
        let blank = with(GOOD, "check = [\"GET /x (0 today)\"]", "check = [\" \"]");
        assert!(refused(
            &problems_of(&blank, &writers()),
            "fields",
            "check names nothing"
        ));
    }

    /// **A KIND IS NAMED ONCE AND NEVER A DELETION** (AUT-05): the gate's
    /// own copy of `pull::rehearsal`'s rule.
    #[test]
    fn a_kind_is_named_once_never_a_deletion_and_its_stamps_read() {
        let kinded = with(
            GOOD,
            "disk = \"4 KiB\"",
            "disk = \"4 KiB\"\nkind = \"gdfl-options-day\"\nrehearsed_commit = \"abc 10\"\nwriter_changed = \"def 9\"\nsnapshot = \"s 11\"",
        );
        assert!(by_rule(&problems_of(&kinded, &writers()), "kinds").is_empty());
        for (from, to, says) in [
            (
                "kind = \"gdfl-options-day\"",
                "kind = \"prune-logs\"",
                "is a prune kind",
            ),
            (
                "kind = \"gdfl-options-day\"",
                "kind = \"token-refresh\"",
                "is a token kind",
            ),
            (
                "kind = \"gdfl-options-day\"",
                "kind = \"Gdfl_Day\"",
                "is not lower-case words",
            ),
            (
                "writer_changed = \"def 9\"",
                "writer_changed = \"def\"",
                "is not \"<name> <unix seconds>\"",
            ),
        ] {
            let p = problems_of(&with(&kinded, from, to), &writers());
            assert!(refused(&p, "kinds", says), "{to}: {p:?}");
        }
        let twice = with(
            &kinded,
            "disk = \"unknown until rehearsed\"",
            "disk = \"unknown until rehearsed\"\nkind = \"gdfl-options-day\"",
        );
        assert!(refused(
            &problems_of(&twice, &writers()),
            "kinds",
            "is named by S-01 and S-02"
        ));
        let stray = with(
            GOOD,
            "disk = \"4 KiB\"",
            "disk = \"4 KiB\"\nsnapshot = \"s 11\"",
        );
        assert!(refused(
            &problems_of(&stray, &writers()),
            "kinds",
            "snapshot without a kind"
        ));
        assert_eq!(NEVER_AUTOMATIC.len(), 8);
    }
}

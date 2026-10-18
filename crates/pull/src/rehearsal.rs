//! The rehearse-then-run gate (AUT-05, R4-06 item 1).
//!
//! # The rule
//!
//! A store-writing KIND starts by itself — from a watcher, a chain, a
//! scheduler, a gap worker, an upkeep runner, anything that is not a person
//! pressing a button — only when its `config/store-steps.toml` step (the one
//! whose `kind` names it) holds all of:
//!
//! 1. a dry run on a private copy, timed: `rehearsed_ms` and
//!    `rehearsed_bytes`;
//! 2. the build that dry run ran on, `rehearsed_commit = "<sha> <unix>"`, at
//!    or after the last change to the kind's writer,
//!    `writer_changed = "<sha> <unix>"` — a recorded commit compare, so a
//!    deploy that leaves the writer alone does not stale the rehearsal, and
//!    one that changes it does (FM-264);
//! 3. a snapshot reference, `snapshot = "<name> <unix>"`, newer than the
//!    store's last bulk write, which the caller reads from the store's own
//!    records and passes in [`Facts`];
//! 4. a `check`: what is read after the run.
//!
//! Every prerequisite package must be merged, and the kind must not be a
//! deletion or a credential change: those are never automatic, whatever the
//! list says ([`NEVER_AUTOMATIC`]). The step's `state` must be `pending`: a
//! `done` step's run is not done twice and a `withdrawn` one never runs.
//!
//! **An `operator: <what>` prerequisite is never met silently.** Each one is
//! met only by the step's own recorded yes for that item — an `operator_yes`
//! entry `"<the item's words, exactly> | <who, when, and the words of the
//! yes>"` — or by the caller attesting, at the moment of the start, that the
//! condition holds ([`Facts::attested`], the item's words exactly: a Kite
//! token read and found valid, every writer stopped). An item that is a
//! person's decision ([`PERSON_ONLY`]: an approval, a yes, an owner, an item
//! no agent runs) is never met by an attestation. Anything less is
//! [`Refusal::OperatorPending`] naming each unmet item, and an admitted kind
//! hands every item back to its starter with how it was met
//! ([`Admitted::operator`]).
//!
//! Figures must be positive, every stamp after 1970 and not after the
//! caller's clock ([`Facts::now`]), and a commit stamp's name a commit hash:
//! a forged future snapshot or a negative dry run is refused, not admitted.
//! Anything else is a [`Refusal`] naming the missing part — never a silent
//! skip and never a start — and each refusal is one `pull.rehearsal` event
//! naming the kind, the code and the part.
//!
//! After an automatic run, [`after_run`] evaluates the step's check through
//! the caller and appends the result as one line to a ledger file
//! ([`record_check`]), so a run is never recorded without its check.
//!
//! # Where the list comes from
//!
//! [`LIST`] is the tracked file, compiled in: the build that starts a kind is
//! the build whose list it obeys, and the map is built once per process
//! ([`Gate::embedded`]) — the file's stamp is the build's. A recorded
//! rehearsal therefore reaches a running starter through a build, never by
//! editing a file under it.
//!
//! # Cost
//!
//! [`Gate::admit`] is one hash lookup and a fixed number of comparisons:
//! O(1) per start. A kind longer than [`MAX_KIND_BYTES`] is refused before it
//! is read, so the name scan is bounded too; the operator items are the
//! step's own (a property of the list, not of the data) and each is matched
//! against the step's yes entries and the caller's attested items. Building
//! the map is O(list length), once. Proof:
//! `pull::rehearsal::tests::the_gate_is_one_lookup_per_start` times n = 1,000
//! admits over a 1-kind and a 10,001-kind list and bounds the p50 ratio.

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::io::Write as _;
use std::path::Path;
use std::sync::OnceLock;

/// The tracked store-steps list, compiled in.
pub const LIST: &str = include_str!("../../../config/store-steps.toml");

/// Words that, inside a kind's name, make it a kind no scheduler starts: a
/// deletion, a pruning, a retirement, or a credential change (AUT-05: R3-28's
/// pruning and SC-9's retirement are never kinds a scheduler starts).
pub const NEVER_AUTOMATIC: [&str; 8] = [
    "delete",
    "remove",
    "prune",
    "retire",
    "credential",
    "token",
    "secret",
    "password",
];

/// The longest `why` a check line keeps, in bytes.
pub const MAX_WHY_BYTES: usize = 512;

/// The longest kind the gate reads, in bytes: a longer one is no kind of the
/// list (the list's kinds are a few lower-case words) and is refused unread.
pub const MAX_KIND_BYTES: usize = 64;

/// The prefix that marks a prerequisite met only by the operator.
pub const OPERATOR: &str = "operator: ";

/// What separates an `operator_yes` entry's item words from who said yes.
pub const YES_SEPARATOR: &str = " | ";

/// Words that make an `operator: <what>` item a person's decision, read in
/// any case: an approval, a yes, a decision, an owner, a sign-off, or an item
/// that says no agent runs it. Such an item is met only by the step's
/// recorded `operator_yes`; a starter attesting its words is not a person
/// saying yes, so the attestation does not meet it.
pub const PERSON_ONLY: [&str; 7] = [
    "approv", "yes", "decision", "owner", "sign-off", "sign off", "no agent",
];

/// Whether the operator item `words` is a person's decision ([`PERSON_ONLY`]).
#[must_use]
pub fn needs_a_person(words: &str) -> bool {
    let lower = words.to_ascii_lowercase();
    PERSON_ONLY.iter().any(|w| lower.contains(w))
}

/// The shortest and longest commit hash a commit stamp may name.
const COMMIT_HEX: std::ops::RangeInclusive<usize> = 7..=40;

/// A recorded commit or snapshot: a name and the moment it stands for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stamp {
    /// The commit hash, or the snapshot's name.
    pub name: String,
    /// Seconds since 1970 (the commit time, or when the snapshot was taken).
    pub unix: i64,
}

impl Stamp {
    /// `"<name> <unix>"`, or why not.
    ///
    /// # Errors
    ///
    /// The value is not one name and one integer.
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut words = text.split_whitespace();
        let (Some(name), Some(unix), None) = (words.next(), words.next(), words.next()) else {
            return Err(format!("{text:?} is not \"<name> <unix seconds>\""));
        };
        let unix = unix
            .parse::<i64>()
            .map_err(|_| format!("{text:?}: {unix:?} is not unix seconds"))?;
        Ok(Self {
            name: name.to_owned(),
            unix,
        })
    }
}

impl fmt::Display for Stamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} at {}", self.name, self.unix)
    }
}

/// One step of the list that names a kind.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Entry {
    step: String,
    kind: String,
    state: Option<String>,
    prerequisites: Vec<String>,
    operator_yes: Vec<String>,
    check: Vec<String>,
    rehearsed_ms: Option<i64>,
    rehearsed_bytes: Option<i64>,
    rehearsed_commit: Option<String>,
    writer_changed: Option<String>,
    snapshot: Option<String>,
}

/// The part a refused kind lacks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    /// No step of the list names the kind.
    Entry,
    /// No timed dry run (`rehearsed_ms` and `rehearsed_bytes`).
    DryRun,
    /// No `rehearsed_commit`, or one that does not read.
    RehearsedCommit,
    /// No `writer_changed`, or one that does not read.
    WriterChanged,
    /// No `snapshot`, or one that does not read.
    Snapshot,
    /// The snapshot is not newer than the last bulk write.
    SnapshotOlder,
    /// The store's last bulk write could not be read, so no snapshot can be
    /// judged newer than it.
    BulkWriteUnknown,
    /// No `check` line.
    Check,
}

impl Part {
    /// The word a refusal names.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Entry => "entry",
            Self::DryRun => "dry_run",
            Self::RehearsedCommit => "rehearsed_commit",
            Self::WriterChanged => "writer_changed",
            Self::Snapshot => "snapshot",
            Self::SnapshotOlder => "snapshot_older_than_last_bulk_write",
            Self::BulkWriteUnknown => "last_bulk_write_unknown",
            Self::Check => "check",
        }
    }
}

/// Why a kind does not start by itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// A part of the rehearsal is missing.
    NotRehearsed {
        /// The kind asked for.
        kind: String,
        /// What it lacks.
        part: Part,
        /// The words, naming the step and the field.
        why: String,
    },
    /// The dry run ran on a build older than the last change to the writer.
    Stale {
        /// The kind asked for.
        kind: String,
        /// The build the dry run ran on.
        rehearsed: Stamp,
        /// The last change to the kind's writer.
        writer: Stamp,
    },
    /// A prerequisite package is not merged.
    NotReady {
        /// The kind asked for.
        kind: String,
        /// The unmerged packages, by name.
        waits_for: Vec<String>,
    },
    /// An `operator: <what>` prerequisite has neither a recorded yes nor the
    /// caller's attestation.
    OperatorPending {
        /// The kind asked for.
        kind: String,
        /// The step that names it.
        step: String,
        /// Each unmet item, by its words after `operator: `.
        items: Vec<String>,
    },
    /// The step is not `pending`: done, withdrawn, or no state at all.
    StepClosed {
        /// The kind asked for.
        kind: String,
        /// The step that names it.
        step: String,
        /// Its state, or `(no state)`.
        state: String,
    },
    /// A deletion or a credential change: never automatic.
    NeverAutomatic {
        /// The kind asked for.
        kind: String,
        /// The word that makes it so.
        word: &'static str,
    },
    /// The list itself did not read, so nothing starts by itself.
    ListUnreadable {
        /// The kind asked for.
        kind: String,
        /// The reader's words.
        why: String,
    },
}

impl Refusal {
    /// The code: `not_rehearsed`, `stale`, `not_ready`, `operator_pending`,
    /// `step_closed`, `never_automatic` or `list_unreadable`.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::NotRehearsed { .. } => "not_rehearsed",
            Self::Stale { .. } => "stale",
            Self::NotReady { .. } => "not_ready",
            Self::OperatorPending { .. } => "operator_pending",
            Self::StepClosed { .. } => "step_closed",
            Self::NeverAutomatic { .. } => "never_automatic",
            Self::ListUnreadable { .. } => "list_unreadable",
        }
    }

    /// The part named: the missing [`Part`]'s word, or the code's own.
    #[must_use]
    pub const fn part(&self) -> &'static str {
        match self {
            Self::NotRehearsed { part, .. } => part.word(),
            Self::Stale { .. } => "rehearsed_commit_older_than_writer_changed",
            Self::NotReady { .. } => "prerequisites",
            Self::OperatorPending { .. } => "operator",
            Self::StepClosed { .. } => "state",
            Self::NeverAutomatic { .. } => "kind",
            Self::ListUnreadable { .. } => "list",
        }
    }

    /// The kind refused.
    #[must_use]
    pub fn kind(&self) -> &str {
        match self {
            Self::NotRehearsed { kind, .. }
            | Self::Stale { kind, .. }
            | Self::NotReady { kind, .. }
            | Self::OperatorPending { kind, .. }
            | Self::StepClosed { kind, .. }
            | Self::NeverAutomatic { kind, .. }
            | Self::ListUnreadable { kind, .. } => kind,
        }
    }
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotRehearsed { kind, why, .. } => write!(f, "not rehearsed: {kind}: {why}"),
            Self::Stale {
                kind,
                rehearsed,
                writer,
            } => write!(
                f,
                "stale: {kind} was rehearsed on {rehearsed}, before its writer changed in {writer}"
            ),
            Self::NotReady { kind, waits_for } => {
                write!(f, "not ready: {kind} waits for {}", waits_for.join(", "))
            }
            Self::OperatorPending { kind, step, items } => write!(
                f,
                "operator pending: {kind} ({step}) waits for the operator: {}; met only by the step's operator_yes for that item or, for a condition the starter can read, by the starter attesting it now",
                items
                    .iter()
                    .map(|i| if needs_a_person(i) {
                        format!("{i} (a person's decision: only operator_yes meets it)")
                    } else {
                        i.clone()
                    })
                    .collect::<Vec<_>>()
                    .join("; ")
            ),
            Self::StepClosed { kind, step, state } => write!(
                f,
                "step closed: {kind} ({step}) is {state}; only a pending step's kind starts by itself"
            ),
            Self::NeverAutomatic { kind, word } => write!(
                f,
                "never automatic: {kind} is a {word} kind, which only a person starts"
            ),
            Self::ListUnreadable { kind, why } => {
                write!(f, "list unreadable: {kind} cannot be judged: {why}")
            }
        }
    }
}

/// The store's last bulk write, as the caller read it from the store's own
/// records.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LastBulkWrite {
    /// The last bulk write ended at this moment (unix seconds).
    At(i64),
    /// The records hold none.
    NoneRecorded,
    /// The records did not read: the words why.
    Unknown(String),
}

/// What the caller knows at the moment of a start.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Facts {
    /// The store's last bulk write.
    pub last_bulk_write: LastBulkWrite,
    /// The caller's clock, unix seconds: no stamp may be after it.
    pub now: i64,
    /// The `operator: <what>` items the starter has checked and found true at
    /// this moment, each by its words after `operator: `, exactly. Only a
    /// condition the starter can read for itself belongs here (a Kite token
    /// read from Parameter Store and found valid, every writer stopped); a
    /// person's approval is the step's `operator_yes`, never an attestation.
    pub attested: Vec<String>,
}

/// How an `operator: <what>` item of an admitted kind was met.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MetBy {
    /// The step's `operator_yes` entry: who said yes, when, and the words.
    Recorded(String),
    /// The starter attested it at the moment of the start.
    Attested,
}

/// One operator item of an admitted kind, handed back to its starter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperatorMet {
    /// The item's words after `operator: `.
    pub item: String,
    /// How it was met.
    pub by: MetBy,
}

/// A kind the gate lets start by itself, with what its run must record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Admitted {
    /// The kind.
    pub kind: String,
    /// The step of the list that names it.
    pub step: String,
    /// The check read after the run.
    pub check: Vec<String>,
    /// The dry run's milliseconds.
    pub rehearsed_ms: i64,
    /// The dry run's bytes.
    pub rehearsed_bytes: i64,
    /// Every `operator: <what>` prerequisite of the step, in list order, with
    /// how it was met: the starter records these with its run.
    pub operator: Vec<OperatorMet>,
}

/// The yes `yes` records for the item `words`: the text after
/// `"<words> | "`, when it is not blank.
fn yes_for<'a>(yes: &'a [String], words: &str) -> Option<&'a str> {
    yes.iter().find_map(|entry| {
        entry
            .strip_prefix(words)
            .and_then(|rest| rest.strip_prefix(YES_SEPARATOR))
            .map(str::trim)
            .filter(|who| !who.is_empty())
    })
}

/// Whether `name` is a commit hash: 7 to 40 lower-case hex digits.
fn is_commit(name: &str) -> bool {
    COMMIT_HEX.contains(&name.len())
        && name
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// The gate: every kind of the list, by name.
#[derive(Debug, Clone, Default)]
pub struct Gate {
    kinds: HashMap<String, Entry>,
    merged: HashMap<String, bool>,
}

/// One value of the list's strict TOML subset.
enum Value {
    Text(String),
    List(Vec<String>),
    Number(i64),
    Flag(bool),
}

/// The text of a basic string that opens at `rest[0] == '"'`, and what
/// follows its closing quote.
fn string_at(rest: &str) -> Result<(String, &str), String> {
    let mut out = String::new();
    let mut chars = rest.char_indices().skip(1);
    while let Some((at, c)) = chars.next() {
        match c {
            '"' => return Ok((out, &rest[at + 1..])),
            '\\' => match chars.next() {
                Some((_, '"')) => out.push('"'),
                Some((_, '\\')) => out.push('\\'),
                Some((_, 'n')) => out.push('\n'),
                Some((_, 't')) => out.push('\t'),
                other => return Err(format!("escape {other:?} is not one this reader knows")),
            },
            _ => out.push(c),
        }
    }
    Err("a string is not closed on its line".to_owned())
}

/// `rest` after a value, which must be blank or a comment.
fn tail_is_blank(rest: &str) -> bool {
    let rest = rest.trim_start();
    rest.is_empty() || rest.starts_with('#')
}

/// The items of an array whose text (after `[`) runs over `lines`.
fn array_from<'a>(
    first: &'a str,
    lines: &mut impl Iterator<Item = (usize, &'a str)>,
) -> Result<Vec<String>, String> {
    let mut items = Vec::new();
    let mut rest = first;
    loop {
        rest = rest.trim_start();
        if rest.is_empty() || rest.starts_with('#') {
            match lines.next() {
                Some((_, line)) => {
                    rest = line;
                    continue;
                }
                None => return Err("an array is not closed".to_owned()),
            }
        }
        if let Some(after) = rest.strip_prefix(']') {
            if !tail_is_blank(after) {
                return Err("text after an array's ]".to_owned());
            }
            return Ok(items);
        }
        if let Some(after) = rest.strip_prefix(',') {
            rest = after;
            continue;
        }
        if rest.starts_with('"') {
            let (text, after) = string_at(rest)?;
            items.push(text);
            rest = after;
            continue;
        }
        return Err(format!("{rest:?} is not a string item"));
    }
}

/// The value that opens `text`, reading further lines for an array.
fn value_from<'a>(
    text: &'a str,
    lines: &mut impl Iterator<Item = (usize, &'a str)>,
) -> Result<Value, String> {
    let text = text.trim_start();
    if text.starts_with('"') {
        let (s, after) = string_at(text)?;
        if !tail_is_blank(after) {
            return Err("text after a string".to_owned());
        }
        return Ok(Value::Text(s));
    }
    if let Some(after) = text.strip_prefix('[') {
        return array_from(after, lines).map(Value::List);
    }
    let word = text.split('#').next().unwrap_or_default().trim();
    match word {
        "true" => Ok(Value::Flag(true)),
        "false" => Ok(Value::Flag(false)),
        _ => word
            .parse::<i64>()
            .map(Value::Number)
            .map_err(|_| format!("{word:?} is not a string, an array, an integer or a flag")),
    }
}

impl Gate {
    /// The gate over `text`, a store-steps list.
    ///
    /// # Errors
    ///
    /// The line and words of the first thing the reader cannot read, a key
    /// written twice in one table, a package declared twice, a kind two
    /// steps name, or an `operator_yes` entry that names no operator item of
    /// its step word for word.
    pub fn from_list(text: &str) -> Result<Self, String> {
        let mut gate = Self::default();
        let mut table = String::new();
        let mut entry = Entry::default();
        let mut package: (Option<String>, Option<bool>) = (None, None);
        let mut keys: HashSet<String> = HashSet::new();
        let mut lines = text.lines().enumerate().map(|(n, l)| (n + 1, l));
        while let Some((number, line)) = lines.next() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            if let Some(name) = trimmed
                .strip_prefix("[[")
                .and_then(|r| r.strip_suffix("]]"))
            {
                gate.close(
                    &table,
                    std::mem::take(&mut entry),
                    std::mem::take(&mut package),
                )
                .map_err(|why| format!("line {number}: {why}"))?;
                name.clone_into(&mut table);
                keys.clear();
                continue;
            }
            let (key, rest) = trimmed
                .split_once('=')
                .ok_or_else(|| format!("line {number}: no `key = value`"))?;
            if !keys.insert(key.trim().to_owned()) {
                return Err(format!(
                    "line {number}: key {:?} is written twice in one table: which one is meant cannot be told",
                    key.trim()
                ));
            }
            let value =
                value_from(rest, &mut lines).map_err(|why| format!("line {number}: {why}"))?;
            match (table.as_str(), key.trim(), value) {
                ("step", "id", Value::Text(t)) => entry.step = t,
                ("step", "kind", Value::Text(t)) => entry.kind = t,
                ("step", "state", Value::Text(t)) => entry.state = Some(t),
                ("step", "prerequisites", Value::List(l)) => entry.prerequisites = l,
                ("step", "operator_yes", Value::List(l)) => entry.operator_yes = l,
                ("step", "check", Value::List(l)) => entry.check = l,
                ("step", "rehearsed_ms", Value::Number(n)) => entry.rehearsed_ms = Some(n),
                ("step", "rehearsed_bytes", Value::Number(n)) => entry.rehearsed_bytes = Some(n),
                ("step", "rehearsed_commit", Value::Text(t)) => entry.rehearsed_commit = Some(t),
                ("step", "writer_changed", Value::Text(t)) => entry.writer_changed = Some(t),
                ("step", "snapshot", Value::Text(t)) => entry.snapshot = Some(t),
                ("package", "name", Value::Text(t)) => package.0 = Some(t),
                ("package", "merged", Value::Flag(f)) => package.1 = Some(f),
                _ => {}
            }
        }
        gate.close(&table, entry, package)?;
        Ok(gate)
    }

    /// File the table just read.
    fn close(
        &mut self,
        table: &str,
        entry: Entry,
        package: (Option<String>, Option<bool>),
    ) -> Result<(), String> {
        match table {
            "step" if !entry.kind.is_empty() => {
                for yes in &entry.operator_yes {
                    let names_an_item = entry.prerequisites.iter().any(|p| {
                        p.strip_prefix(OPERATOR).is_some_and(|words| {
                            yes_for(std::slice::from_ref(yes), words).is_some()
                        })
                    });
                    if !names_an_item {
                        return Err(format!(
                            "{}: operator_yes {yes:?} is not \"<an operator item's words, exactly>{YES_SEPARATOR}<who, when, the words of the yes>\" for an `operator: ` prerequisite of the step",
                            entry.step
                        ));
                    }
                }
                let kind = entry.kind.clone();
                if let Some(other) = self.kinds.insert(kind.clone(), entry) {
                    return Err(format!(
                        "kind {kind:?} is named by two steps ({} and another)",
                        other.step
                    ));
                }
            }
            "package" => {
                if let (Some(name), Some(merged)) = package
                    && self.merged.insert(name.clone(), merged).is_some()
                {
                    return Err(format!(
                        "package {name:?} is declared twice: whether it is merged cannot be told"
                    ));
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// The gate over [`LIST`], built once per process.
    ///
    /// # Errors
    ///
    /// The list did not read: the reader's words.
    pub fn embedded() -> Result<&'static Self, &'static str> {
        static GATE: OnceLock<Result<Gate, String>> = OnceLock::new();
        GATE.get_or_init(|| Self::from_list(LIST))
            .as_ref()
            .map_err(String::as_str)
    }

    /// How many kinds the list names.
    #[must_use]
    pub fn kinds(&self) -> usize {
        self.kinds.len()
    }

    /// Whether `kind` may start by itself now: one lookup.
    ///
    /// # Errors
    ///
    /// The [`Refusal`] naming the missing part.
    pub fn admit(&self, kind: &str, facts: &Facts) -> Result<Admitted, Refusal> {
        if kind.len() > MAX_KIND_BYTES {
            return Err(Refusal::NotRehearsed {
                kind: kind.chars().take(MAX_KIND_BYTES).collect(),
                part: Part::Entry,
                why: format!(
                    "a kind of {} bytes is longer than {MAX_KIND_BYTES}: no step names it",
                    kind.len()
                ),
            });
        }
        let lower = kind.to_ascii_lowercase();
        if let Some(word) = NEVER_AUTOMATIC.iter().find(|w| lower.contains(*w)) {
            return Err(Refusal::NeverAutomatic {
                kind: kind.to_owned(),
                word,
            });
        }
        let Some(entry) = self.kinds.get(kind) else {
            return Err(Refusal::NotRehearsed {
                kind: kind.to_owned(),
                part: Part::Entry,
                why: "no step of config/store-steps.toml names this kind".to_owned(),
            });
        };
        entry.open_and_ready(kind, &self.merged)?;
        let operator = entry.operator_met(kind, facts)?;
        let (rehearsed_ms, rehearsed_bytes) = entry.rehearsed(kind, facts)?;
        Ok(Admitted {
            kind: kind.to_owned(),
            step: entry.step.clone(),
            check: entry.check.clone(),
            rehearsed_ms,
            rehearsed_bytes,
            operator,
        })
    }
}

impl Entry {
    /// A part of this step's rehearsal is missing: the refusal.
    fn refused(kind: &str, part: Part, why: String) -> Refusal {
        Refusal::NotRehearsed {
            kind: kind.to_owned(),
            part,
            why,
        }
    }

    /// The step is `pending` and every prerequisite package is merged.
    fn open_and_ready(&self, kind: &str, merged: &HashMap<String, bool>) -> Result<(), Refusal> {
        if self.state.as_deref() != Some("pending") {
            return Err(Refusal::StepClosed {
                kind: kind.to_owned(),
                step: self.step.clone(),
                state: self
                    .state
                    .as_ref()
                    .map_or_else(|| "(no state)".to_owned(), |s| format!("{s:?}")),
            });
        }
        let waits_for: Vec<String> = self
            .prerequisites
            .iter()
            .filter(|p| !p.starts_with(OPERATOR) && merged.get(*p) != Some(&true))
            .cloned()
            .collect();
        if waits_for.is_empty() {
            Ok(())
        } else {
            Err(Refusal::NotReady {
                kind: kind.to_owned(),
                waits_for,
            })
        }
    }

    /// Every `operator: <what>` prerequisite, met by the step's recorded yes
    /// or the starter's attestation, in list order; or the unmet ones.
    fn operator_met(&self, kind: &str, facts: &Facts) -> Result<Vec<OperatorMet>, Refusal> {
        let mut met = Vec::new();
        let mut pending = Vec::new();
        for words in self
            .prerequisites
            .iter()
            .filter_map(|p| p.strip_prefix(OPERATOR))
        {
            let by = if let Some(yes) = yes_for(&self.operator_yes, words) {
                MetBy::Recorded(yes.to_owned())
            } else if !needs_a_person(words) && facts.attested.iter().any(|a| a == words) {
                MetBy::Attested
            } else {
                pending.push(words.to_owned());
                continue;
            };
            met.push(OperatorMet {
                item: words.to_owned(),
                by,
            });
        }
        if pending.is_empty() {
            Ok(met)
        } else {
            Err(Refusal::OperatorPending {
                kind: kind.to_owned(),
                step: self.step.clone(),
                items: pending,
            })
        }
    }

    /// The stamp `field` holds for `part`: present, `"<name> <unix>"`, in
    /// 1970..`now`, and (`commit`) naming a commit hash.
    fn stamp(
        &self,
        kind: &str,
        field: Option<&str>,
        part: Part,
        commit: bool,
        now: i64,
    ) -> Result<Stamp, Refusal> {
        let (step, word) = (&self.step, part.word());
        let text =
            field.ok_or_else(|| Self::refused(kind, part, format!("{step} has no {word}")))?;
        let read = Stamp::parse(text)
            .map_err(|why| Self::refused(kind, part, format!("{step}: {word}: {why}")))?;
        if read.unix <= 0 || read.unix > now {
            return Err(Self::refused(
                kind,
                part,
                format!(
                    "{step}: {word} {read} is not a moment between 1970 and now ({now}): no build or snapshot is stamped there"
                ),
            ));
        }
        if commit && !is_commit(&read.name) {
            return Err(Self::refused(
                kind,
                part,
                format!(
                    "{step}: {word} {:?} is not a commit hash (7 to 40 lower-case hex digits)",
                    read.name
                ),
            ));
        }
        Ok(read)
    }

    /// The step's timed dry run, current against its writer, with a check
    /// and a snapshot newer than the last bulk write: the dry run's figures.
    fn rehearsed(&self, kind: &str, facts: &Facts) -> Result<(i64, i64), Refusal> {
        let step = &self.step;
        let (Some(ms), Some(bytes)) = (self.rehearsed_ms, self.rehearsed_bytes) else {
            return Err(Self::refused(
                kind,
                Part::DryRun,
                format!(
                    "{step} has no timed dry run on a private copy (rehearsed_ms, rehearsed_bytes)"
                ),
            ));
        };
        if ms <= 0 || bytes <= 0 {
            return Err(Self::refused(
                kind,
                Part::DryRun,
                format!(
                    "{step}: rehearsed_ms {ms} and rehearsed_bytes {bytes} must both be above 0: a dry run that took no time or wrote nothing rehearsed no write"
                ),
            ));
        }
        let now = facts.now;
        let rehearsed = self.stamp(
            kind,
            self.rehearsed_commit.as_deref(),
            Part::RehearsedCommit,
            true,
            now,
        )?;
        let writer = self.stamp(
            kind,
            self.writer_changed.as_deref(),
            Part::WriterChanged,
            true,
            now,
        )?;
        if rehearsed.unix < writer.unix {
            return Err(Refusal::Stale {
                kind: kind.to_owned(),
                rehearsed,
                writer,
            });
        }
        if self.check.iter().all(|line| line.trim().is_empty()) {
            return Err(Self::refused(
                kind,
                Part::Check,
                format!("{step} has no check"),
            ));
        }
        let snapshot = self.stamp(kind, self.snapshot.as_deref(), Part::Snapshot, false, now)?;
        match &facts.last_bulk_write {
            LastBulkWrite::Unknown(why) => Err(Self::refused(
                kind,
                Part::BulkWriteUnknown,
                format!("the store's last bulk write did not read: {why}"),
            )),
            LastBulkWrite::At(at) if snapshot.unix <= *at => Err(Self::refused(
                kind,
                Part::SnapshotOlder,
                format!(
                    "{step}: snapshot {snapshot} is not newer than the last bulk write at {at}"
                ),
            )),
            LastBulkWrite::At(_) | LastBulkWrite::NoneRecorded => Ok((ms, bytes)),
        }
    }
}

/// [`Gate::admit`] over the embedded list, with the refusal recorded as one
/// `pull.rehearsal` event (the kind, the code, the part; nothing else).
///
/// # Errors
///
/// The [`Refusal`]; a list that did not read refuses every kind.
pub fn may_start_by_itself(kind: &str, facts: &Facts) -> Result<Admitted, Refusal> {
    let answer = match Gate::embedded() {
        Ok(gate) => gate.admit(kind, facts),
        Err(why) => Err(Refusal::ListUnreadable {
            kind: kind.to_owned(),
            why: why.to_owned(),
        }),
    };
    if let Err(refusal) = &answer {
        note_refused(refusal);
    }
    answer
}

/// The one event a refusal writes.
pub(crate) fn note_refused(refusal: &Refusal) {
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::warn("pull.rehearsal", "kind refused to start by itself")
            .with("kind", telemetry::Value::Str(refusal.kind()))
            .with("code", telemetry::Value::Str(refusal.code()))
            .with("part", telemetry::Value::Str(refusal.part())),
    );
}

/// What a check read after an automatic run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckOutcome {
    /// Every check line held.
    Passed,
    /// A line did not: the words.
    Failed(String),
    /// The check could not be read: the words.
    NotEvaluated(String),
}

impl CheckOutcome {
    /// `passed`, `failed` or `not_evaluated`.
    #[must_use]
    pub const fn word(&self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Failed(_) => "failed",
            Self::NotEvaluated(_) => "not_evaluated",
        }
    }

    fn why(&self) -> &str {
        match self {
            Self::Passed => "",
            Self::Failed(why) | Self::NotEvaluated(why) => why,
        }
    }
}

/// One stored check result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckRecord<'a> {
    /// The run's own id.
    pub run: &'a str,
    /// When the check was read (unix seconds).
    pub at: i64,
    /// The kind.
    pub kind: &'a str,
    /// The step.
    pub step: &'a str,
    /// What it read.
    pub outcome: &'a CheckOutcome,
}

/// `text` as one field of a check line: `%`, a space, `=`, a tab and a line
/// break are written `%25`, `%20`, `%3D`, `%09`, `%0A` and `%0D`, so a run id
/// such as `r1 outcome=passed` cannot forge a field a reader would take for
/// the line's own.
fn field(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '%' => out.push_str("%25"),
            ' ' => out.push_str("%20"),
            '=' => out.push_str("%3D"),
            '\t' => out.push_str("%09"),
            '\n' => out.push_str("%0A"),
            '\r' => out.push_str("%0D"),
            other => out.push(other),
        }
    }
    out
}

/// The line [`record_check`] appends: `v=1 at=.. run=.. kind=.. step=..
/// outcome=.. why=..`, one line. `run`, `kind` and `step` are escaped
/// ([`field`]); the why comes last, so it may hold spaces and `=`, and is
/// bounded at [`MAX_WHY_BYTES`] with line breaks spaced out.
#[must_use]
pub fn check_line(record: &CheckRecord<'_>) -> String {
    let flat = |text: &str| text.replace(['\n', '\r'], " ");
    let mut why = flat(record.outcome.why());
    if why.len() > MAX_WHY_BYTES {
        let mut cut = MAX_WHY_BYTES;
        while !why.is_char_boundary(cut) {
            cut -= 1;
        }
        why.truncate(cut);
    }
    format!(
        "v=1 at={} run={} kind={} step={} outcome={} why={why}\n",
        record.at,
        field(record.run),
        field(record.kind),
        field(record.step),
        record.outcome.word(),
    )
}

/// Append `record` to `ledger` as one line, and make it durable: the file is
/// synced, and when this append created it, so is the folder that names it
/// (a created file whose folder entry was not synced can vanish whole at a
/// power cut).
///
/// # Errors
///
/// The host's words: the ledger could not be opened, written or synced, or
/// its folder could not be synced after the ledger was created.
pub fn record_check(ledger: &Path, record: &CheckRecord<'_>) -> std::io::Result<()> {
    let created = std::fs::symlink_metadata(ledger).is_err();
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(ledger)?;
    file.write_all(check_line(record).as_bytes())?;
    file.sync_all()?;
    if created {
        let folder = ledger
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        std::fs::File::open(folder)?.sync_all()?;
    }
    Ok(())
}

/// After an automatic run of `admitted`: read its check through `evaluate`,
/// and store the result in `ledger`. The result is returned either way.
///
/// # Errors
///
/// The result could not be stored: the host's words. The run is then not
/// recorded as checked, and the caller says so.
pub fn after_run(
    admitted: &Admitted,
    run: &str,
    at: i64,
    evaluate: impl FnOnce(&[String]) -> CheckOutcome,
    ledger: &Path,
) -> std::io::Result<CheckOutcome> {
    let outcome = evaluate(&admitted.check);
    record_check(
        ledger,
        &CheckRecord {
            run,
            at,
            kind: &admitted.kind,
            step: &admitted.step,
            outcome: &outcome,
        },
    )?;
    Ok(outcome)
}

#[cfg(test)]
#[path = "rehearsal_tests.rs"]
mod tests;

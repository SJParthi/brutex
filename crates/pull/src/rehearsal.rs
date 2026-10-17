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
//! list says ([`NEVER_AUTOMATIC`]). Anything else is a [`Refusal`] naming the
//! missing part — never a silent skip and never a start — and each refusal is
//! one `pull.rehearsal` event naming the kind, the code and the part.
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
//! O(1) per start. Building the map is O(list length), once.

use std::collections::HashMap;
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
    prerequisites: Vec<String>,
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
    /// The code: `not_rehearsed`, `stale`, `not_ready`, `never_automatic` or
    /// `list_unreadable`.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::NotRehearsed { .. } => "not_rehearsed",
            Self::Stale { .. } => "stale",
            Self::NotReady { .. } => "not_ready",
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
    /// The line and words of the first thing the reader cannot read, or a
    /// kind two steps name.
    pub fn from_list(text: &str) -> Result<Self, String> {
        let mut gate = Self::default();
        let mut table = String::new();
        let mut entry = Entry::default();
        let mut package: (Option<String>, Option<bool>) = (None, None);
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
                continue;
            }
            let (key, rest) = trimmed
                .split_once('=')
                .ok_or_else(|| format!("line {number}: no `key = value`"))?;
            let value =
                value_from(rest, &mut lines).map_err(|why| format!("line {number}: {why}"))?;
            match (table.as_str(), key.trim(), value) {
                ("step", "id", Value::Text(t)) => entry.step = t,
                ("step", "kind", Value::Text(t)) => entry.kind = t,
                ("step", "prerequisites", Value::List(l)) => entry.prerequisites = l,
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
                let kind = entry.kind.clone();
                if let Some(other) = self.kinds.insert(kind.clone(), entry) {
                    return Err(format!(
                        "kind {kind:?} is named by two steps ({} and another)",
                        other.step
                    ));
                }
            }
            "package" => {
                if let (Some(name), Some(merged)) = package {
                    self.merged.insert(name, merged);
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
        let refused = |part: Part, why: String| Refusal::NotRehearsed {
            kind: kind.to_owned(),
            part,
            why,
        };
        let lower = kind.to_ascii_lowercase();
        if let Some(word) = NEVER_AUTOMATIC.iter().find(|w| lower.contains(*w)) {
            return Err(Refusal::NeverAutomatic {
                kind: kind.to_owned(),
                word,
            });
        }
        let Some(entry) = self.kinds.get(kind) else {
            return Err(refused(
                Part::Entry,
                "no step of config/store-steps.toml names this kind".to_owned(),
            ));
        };
        let step = &entry.step;
        let waits_for: Vec<String> = entry
            .prerequisites
            .iter()
            .filter(|p| !p.starts_with("operator: ") && self.merged.get(*p) != Some(&true))
            .cloned()
            .collect();
        if !waits_for.is_empty() {
            return Err(Refusal::NotReady {
                kind: kind.to_owned(),
                waits_for,
            });
        }
        let (Some(rehearsed_ms), Some(rehearsed_bytes)) =
            (entry.rehearsed_ms, entry.rehearsed_bytes)
        else {
            return Err(refused(
                Part::DryRun,
                format!(
                    "{step} has no timed dry run on a private copy (rehearsed_ms, rehearsed_bytes)"
                ),
            ));
        };
        let stamp = |field: Option<&String>, part: Part| {
            field
                .ok_or_else(|| refused(part, format!("{step} has no {}", part.word())))
                .and_then(|t| {
                    Stamp::parse(t)
                        .map_err(|why| refused(part, format!("{step}: {}: {why}", part.word())))
                })
        };
        let rehearsed = stamp(entry.rehearsed_commit.as_ref(), Part::RehearsedCommit)?;
        let writer = stamp(entry.writer_changed.as_ref(), Part::WriterChanged)?;
        if rehearsed.unix < writer.unix {
            return Err(Refusal::Stale {
                kind: kind.to_owned(),
                rehearsed,
                writer,
            });
        }
        if entry.check.iter().all(|line| line.trim().is_empty()) {
            return Err(refused(Part::Check, format!("{step} has no check")));
        }
        let snapshot = stamp(entry.snapshot.as_ref(), Part::Snapshot)?;
        match &facts.last_bulk_write {
            LastBulkWrite::Unknown(why) => {
                return Err(refused(
                    Part::BulkWriteUnknown,
                    format!("the store's last bulk write did not read: {why}"),
                ));
            }
            LastBulkWrite::At(at) if snapshot.unix <= *at => {
                return Err(refused(
                    Part::SnapshotOlder,
                    format!(
                        "{step}: snapshot {snapshot} is not newer than the last bulk write at {at}"
                    ),
                ));
            }
            LastBulkWrite::At(_) | LastBulkWrite::NoneRecorded => {}
        }
        Ok(Admitted {
            kind: kind.to_owned(),
            step: step.clone(),
            check: entry.check.clone(),
            rehearsed_ms,
            rehearsed_bytes,
        })
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

/// The line [`record_check`] appends: `v=1 at=.. run=.. kind=.. step=..
/// outcome=.. why=..`, one line, the why bounded at [`MAX_WHY_BYTES`] with
/// line breaks spaced out.
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
        flat(record.run),
        flat(record.kind),
        flat(record.step),
        record.outcome.word(),
    )
}

/// Append `record` to `ledger` as one line, and make it durable.
///
/// # Errors
///
/// The host's words: the ledger could not be opened, written or synced.
pub fn record_check(ledger: &Path, record: &CheckRecord<'_>) -> std::io::Result<()> {
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(ledger)?;
    file.write_all(check_line(record).as_bytes())?;
    file.sync_all()
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

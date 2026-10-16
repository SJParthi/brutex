//! The reader of the GDFL import journal, `imports/gdfl.journal` under the
//! store root ([`pull::gdfl_import::journal_path`]), for `/imports.json`,
//! `/audit.json?feed=gdfl` and the routes that name a day's import outcome.
//!
//! # Why the API reads a file `pull` writes
//!
//! The journal is GDFL's own audit trail: one line per day begun, one per day
//! closed, and since D-3317 one per run. Until WP-33a nothing in `crates/api`
//! read it (DB-G22, DB-054): a day whose every file was refused, a day the
//! census refused and a day still being imported all looked the same on every
//! page, a hole. This module is the one reader, so `/imports.json` and
//! `/audit.json` cannot disagree about a day.
//!
//! # A reader, not the loader
//!
//! `pull::gdfl_import`'s loader decides what a run imports next and REFUSES a
//! journal holding any line it does not write (D-2814): a run must not guess.
//! A page must not go blank either, so this reader keeps every line it can
//! name, counts and names the rest ([`Parsed::foreign`], [`Parsed::torn`]) and
//! answers. It reads the same lines with the same rule, the last closing line
//! of a key wins, and the field names are D-3317's. `<key>` is `<kind> <day>
//! <filter>`.
//!
//! | line | what it says about the key |
//! |---|---|
//! | `run <id> k=v ...` | a run's provenance (no key) |
//! | `begin <key>` | begun; open until a closing line |
//! | `done <key> <stats>` | done, or done with refusals when `vendor_refused=` is above 0 |
//! | `incomplete <key> <stats> stage= reason=` | wrote, and is not done |
//! | `refused <key> definition= stage= reason= failures=` | wrote nothing |
//! | `recounted <key> definition= months=` | closed by the census recount: done |
//!
//! Lines written before D-3317 carry no `stage=`, `reason=` or `run=`; their
//! row says so with `stage: null`, and `/imports.json` names the evidence the
//! importer logged for the day instead ([`crate::logs::import_refusals`]).
//!
//! # Cost: a tail, never a re-read
//!
//! The journal is append-only, so [`snapshot`] keeps what it parsed, keyed by
//! the file's path, and on the next call reads only the bytes appended since:
//! one `metadata` call, and one read of the new bytes. A file that shrank or
//! was replaced (another inode) is read again from the start. Rows are held in
//! a `BTreeMap` keyed by day, so a page of `from..=to` walks only that range:
//! O(log n + days in the range), never the whole journal. The first call reads
//! the whole file once: 601,540 bytes and 8,944 lines on the operator's store,
//! 9 Oct 2026.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};

use pull::gdfl_import::{ImportKind, Outcome, definition_is_current};
use pull::session::Day;

/// The most foreign or torn line numbers one snapshot keeps by name; the rest
/// are counted.
pub(crate) const MAX_NAMED_FAULTS: usize = 32;

/// The longest text value (a stage or a reason) a row keeps; a longer one is
/// cut and the row says so with `cut`.
const MAX_VALUE_BYTES: usize = 96;

/// How a key stands after the journal's last line about it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) enum DayOutcome {
    /// Closed `done` with no failure, or `recounted`.
    Done,
    /// Closed `done` with `vendor_refused=` above zero.
    DoneWithRefusals,
    /// Closed `incomplete`.
    Incomplete,
    /// Closed `refused`.
    Refused,
    /// Begun and never closed: a run in progress, or one that stopped (or
    /// was stopped) part way through the day.
    Open,
}

impl DayOutcome {
    /// Every outcome, in the order a count lists them.
    pub(crate) const ALL: [Self; 5] = [
        Self::Done,
        Self::DoneWithRefusals,
        Self::Incomplete,
        Self::Refused,
        Self::Open,
    ];

    /// The word the wire names it by: `pull`'s own for the four closes
    /// (D-3335's `outcome`), and `open` for a day begun and not closed.
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Done => Outcome::Done.as_str(),
            Self::DoneWithRefusals => Outcome::DoneWithRefusals.as_str(),
            Self::Incomplete => Outcome::Incomplete.as_str(),
            Self::Refused => Outcome::Refused.as_str(),
            Self::Open => "open",
        }
    }

    /// The outcome a word names.
    pub(crate) fn parse(word: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|outcome| outcome.as_str() == word)
    }

    /// Whether the day still owes an import.
    pub(crate) const fn is_done(self) -> bool {
        matches!(self, Self::Done | Self::DoneWithRefusals)
    }
}

/// The counts a closing line carries, each `None` when the line has none.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Stats {
    /// `definition=`: the bar definition that built the day.
    pub(crate) definition: Option<u64>,
    /// `files=`.
    pub(crate) files: Option<u64>,
    /// `seconds=`: one-second bars written.
    pub(crate) seconds: Option<u64>,
    /// `failures=`.
    pub(crate) failures: Option<u64>,
    /// `late=`: rows placed late.
    pub(crate) late: Option<u64>,
    /// `late_unresolved=`: late rows dropped.
    pub(crate) late_unresolved: Option<u64>,
    /// `max_back_s=`: the furthest a late row stepped back.
    pub(crate) max_back_s: Option<u64>,
    /// `vendor_refused=`: the refusals of the vendor's own bytes a day done
    /// with refusals closed over.
    pub(crate) vendor_refused: Option<u64>,
    /// `months=`: the months a recount found held.
    pub(crate) months: Option<u64>,
    /// `run=`: the run line this close belongs to.
    pub(crate) run: Option<u64>,
}

/// One key's standing: the last closing line about it, and its history.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DayRow {
    /// The import kind.
    pub(crate) kind: ImportKind,
    /// The day.
    pub(crate) day: Day,
    /// The instrument filter (`*` for the whole tree).
    pub(crate) filter: String,
    /// How it stands.
    pub(crate) outcome: DayOutcome,
    /// Whether the deciding line is a `recounted` line.
    pub(crate) recounted: bool,
    /// The deciding line's counts.
    pub(crate) stats: Stats,
    /// The deciding line's `stage=`, when it carries one.
    pub(crate) stage: Option<String>,
    /// The deciding line's `reason=`, when it carries one.
    pub(crate) reason: Option<String>,
    /// Whether a stage or reason was cut at [`MAX_VALUE_BYTES`].
    pub(crate) cut: bool,
    /// The deciding line's number, from 1.
    pub(crate) line: u64,
    /// `begin` lines for this key: one per attempt.
    pub(crate) attempts: u32,
    /// Closing lines for this key, the deciding one included.
    pub(crate) closes: u32,
    /// Closing lines for this key that named `stage=census`.
    pub(crate) census_closes: u32,
    /// The outcome the key had before its deciding line, when it was closed
    /// before (a day `incomplete` and then `recounted`).
    pub(crate) previous: Option<DayOutcome>,
}

impl DayRow {
    /// Whether the deciding line is `done` under a definition the importer
    /// does not take as current: it imports the day again (D-2822). The one
    /// rule is `pull::gdfl_import::definition_is_current`, asked with the
    /// key's day: an index day under 2 or 3 is current (D-3342), and a
    /// share's day from 2026-08-03 under 2 or 3 is not, its closing auction's
    /// band having changed what it folds (D-4878 item 10, CMP-07).
    pub(crate) fn restated(&self) -> bool {
        self.outcome.is_done()
            && !self.recounted
            && self
                .stats
                .definition
                .is_some_and(|definition| !definition_is_current(self.kind, self.day, definition))
    }
}

/// One `run` line: its id, its line and its fields, decoded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RunLine {
    /// The run's id.
    pub(crate) id: u64,
    /// Its line number, from 1.
    pub(crate) line: u64,
    /// Its `name=value` fields in order, each value percent-decoded.
    pub(crate) fields: Vec<(String, String)>,
}

/// The key a row is held under: day first, so a range of days is a range of
/// the map.
type Key = (u32, ImportKindOrder, String);

/// [`ImportKind`] in import order, as a sortable key part.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct ImportKindOrder(u8);

impl ImportKindOrder {
    /// The order of `kind` in [`ImportKind::ALL`].
    fn of(kind: ImportKind) -> Self {
        Self(
            ImportKind::ALL
                .iter()
                .position(|known| *known == kind)
                .and_then(|at| u8::try_from(at).ok())
                .unwrap_or(u8::MAX),
        )
    }
}

/// Everything one read of the journal found.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Parsed {
    rows: BTreeMap<Key, DayRow>,
    /// Every `run` line, in file order.
    pub(crate) runs: Vec<RunLine>,
    /// Where each run id's lines are in [`Self::runs`], oldest first: kept as
    /// each `run` line is parsed, one probe and one push, so `run=<id>` and a
    /// `runs_before=<id>` cursor find an id's lines without decoding any
    /// other (D-4813 round 6, R6; IA-LG-03). Two processes that start in one
    /// second share an id, which is why a value is a list.
    runs_by_id: std::collections::HashMap<u64, Vec<u32>>,
    /// Whole lines read.
    pub(crate) lines: u64,
    /// Lines this reader cannot name: the first [`MAX_NAMED_FAULTS`], by
    /// number with their first bytes.
    pub(crate) foreign: Vec<(u64, String)>,
    /// Every such line, counted.
    pub(crate) foreign_count: u64,
    /// Lines a crash cut short and a later load closed with ` (torn)`: the
    /// first [`MAX_NAMED_FAULTS`] by number.
    pub(crate) torn: Vec<u64>,
    /// Every such line, counted.
    pub(crate) torn_count: u64,
    /// Per kind, in [`ImportKind::ALL`] order: the last line about that kind
    /// (its number, day and verb), so a page can say where each kind stands.
    pub(crate) last: [Option<(u64, Day, String)>; 3],
    /// `run` lines past the 4,294,967,295th, which [`Self::runs_of`] cannot
    /// name: counted, never silently missing from a `run=` answer.
    pub(crate) runs_unindexed: u64,
}

impl Parsed {
    /// Every row of `from..=to`, oldest day first. O(log n + rows in range).
    pub(crate) fn range(&self, from: Day, to: Day) -> impl DoubleEndedIterator<Item = &DayRow> {
        let low = (from.days_from_epoch(), ImportKindOrder(0), String::new());
        let high = (
            to.days_from_epoch().saturating_add(1),
            ImportKindOrder(0),
            String::new(),
        );
        self.rows.range(low..high).map(|(_, row)| row)
    }

    /// The first and last day any row names.
    pub(crate) fn span(&self) -> Option<(Day, Day)> {
        let first = self.rows.values().next()?.day;
        let last = self.rows.values().next_back()?.day;
        Some((first, last))
    }

    /// How many keys the journal names.
    pub(crate) fn keys(&self) -> usize {
        self.rows.len()
    }

    /// The `run` lines of one id, oldest first, through the id index: one
    /// hash probe and the id's own lines, whatever the journal holds
    /// (D-4813 round 6; IA-LG-03).
    pub(crate) fn runs_of(&self, id: u64) -> impl DoubleEndedIterator<Item = &RunLine> {
        self.runs_by_id
            .get(&id)
            .map_or(&[][..], Vec::as_slice)
            .iter()
            .filter_map(|at| self.runs.get(usize::try_from(*at).ok()?))
    }

    /// How many `run` lines are older than the `nth` line (from 0, oldest
    /// first) of run `id`: where a `runs_before=<id>` page (`nth` 0), or one
    /// continued inside an id that has several lines (`<id>.<nth>`), starts.
    /// `None` for an id, or a line of it, the journal does not hold. One hash
    /// probe and one index.
    pub(crate) fn runs_before(&self, id: u64, nth: usize) -> Option<usize> {
        let at = self.runs_by_id.get(&id)?.get(nth)?;
        usize::try_from(*at).ok()
    }

    /// Which of its id's lines (from 0, oldest first) the `run` line at
    /// position `at` of [`Self::runs`] is: what a page that ends there names
    /// as its cursor. A walk of that id's own lines only.
    pub(crate) fn nth_of(&self, at: usize) -> Option<usize> {
        let id = self.runs.get(at)?.id;
        let wanted = u32::try_from(at).ok()?;
        self.runs_by_id
            .get(&id)?
            .iter()
            .position(|held| *held == wanted)
    }

    /// Folds one whole line in. `number` counts from 1.
    fn line(&mut self, number: u64, text: &str) {
        self.lines = number;
        if text.ends_with(" (torn)") {
            self.torn_count = self.torn_count.saturating_add(1);
            if self.torn.len() < MAX_NAMED_FAULTS {
                self.torn.push(number);
            }
            return;
        }
        if text.is_empty() {
            return;
        }
        if self.fold(number, text).is_none() {
            self.foreign_count = self.foreign_count.saturating_add(1);
            if self.foreign.len() < MAX_NAMED_FAULTS {
                let shown: String = text.chars().take(80).collect();
                self.foreign.push((number, shown));
            }
        }
    }

    /// One line this reader can name, or `None`.
    fn fold(&mut self, number: u64, text: &str) -> Option<()> {
        let fields: Vec<&str> = text.split(' ').collect();
        match fields.as_slice() {
            ["run", id, rest @ ..] => {
                let id = canonical_count(id)?;
                let mut decoded = Vec::with_capacity(rest.len());
                for field in rest {
                    let (name, value) = field.split_once('=')?;
                    decoded.push((name.to_owned(), percent_decoded(value)?));
                }
                // THE INDEX BESIDE THE LIST, in the same step: a line past
                // `u32::MAX` runs is not indexed and `runs_unindexed` says so,
                // rather than a position that wraps and names another run.
                match u32::try_from(self.runs.len()) {
                    Ok(at) => self.runs_by_id.entry(id).or_default().push(at),
                    Err(_) => self.runs_unindexed = self.runs_unindexed.saturating_add(1),
                }
                self.runs.push(RunLine {
                    id,
                    line: number,
                    fields: decoded,
                });
            }
            ["begin", kind, day, filter] => {
                let (kind, day) = key_parts(kind, day)?;
                self.note_last(kind, number, day, "begin");
                let row = self.entry(kind, day, filter, number, DayOutcome::Open);
                row.attempts = row.attempts.saturating_add(1);
                if row.closes > 0 {
                    row.previous = Some(row.outcome);
                }
                row.outcome = DayOutcome::Open;
                row.line = number;
            }
            [
                verb @ ("done" | "incomplete" | "refused" | "recounted"),
                kind,
                day,
                filter,
                rest @ ..,
            ] => {
                let (kind, day) = key_parts(kind, day)?;
                let close = Close::read(rest)?;
                self.note_last(kind, number, day, verb);
                self.close(kind, day, filter, number, verb, close);
            }
            _ => return None,
        }
        Some(())
    }

    /// The row of a key, made `outcome` when it is new.
    fn entry(
        &mut self,
        kind: ImportKind,
        day: Day,
        filter: &str,
        number: u64,
        outcome: DayOutcome,
    ) -> &mut DayRow {
        let key = (
            day.days_from_epoch(),
            ImportKindOrder::of(kind),
            filter.to_owned(),
        );
        self.rows.entry(key).or_insert_with(|| DayRow {
            kind,
            day,
            filter: filter.to_owned(),
            outcome,
            recounted: false,
            stats: Stats::default(),
            stage: None,
            reason: None,
            cut: false,
            line: number,
            attempts: 0,
            closes: 0,
            census_closes: 0,
            previous: None,
        })
    }

    /// Folds one closing line into its key's row: the last close wins.
    fn close(
        &mut self,
        kind: ImportKind,
        day: Day,
        filter: &str,
        number: u64,
        verb: &str,
        close: Close,
    ) {
        let outcome = match verb {
            "done" if close.stats.vendor_refused.is_some_and(|n| n > 0) => {
                DayOutcome::DoneWithRefusals
            }
            "done" | "recounted" => DayOutcome::Done,
            "incomplete" => DayOutcome::Incomplete,
            _ => DayOutcome::Refused,
        };
        let row = self.entry(kind, day, filter, number, outcome);
        if row.closes > 0 {
            row.previous = Some(row.outcome);
        }
        row.closes = row.closes.saturating_add(1);
        if close.stage.as_deref() == Some(pull::gdfl_import::Stage::Census.as_str()) {
            row.census_closes = row.census_closes.saturating_add(1);
        }
        row.outcome = outcome;
        row.recounted = verb == "recounted";
        row.stats = close.stats;
        row.stage = close.stage;
        row.reason = close.reason;
        row.cut = close.cut;
        row.line = number;
    }

    /// Records the last line about `kind`.
    fn note_last(&mut self, kind: ImportKind, number: u64, day: Day, verb: &str) {
        let slot = usize::from(ImportKindOrder::of(kind).0);
        if let Some(last) = self.last.get_mut(slot) {
            *last = Some((number, day, verb.to_owned()));
        }
    }
}

/// What a closing line's fields say.
#[derive(Debug, Default)]
struct Close {
    stats: Stats,
    stage: Option<String>,
    reason: Option<String>,
    cut: bool,
}

impl Close {
    /// The fields after a key, or `None` for a field that is not `name=value`
    /// or a count that is not one as the journal writes it.
    fn read(fields: &[&str]) -> Option<Self> {
        let mut close = Self::default();
        for field in fields {
            let (name, value) = field.split_once('=')?;
            let count = || canonical_count(value);
            let stats = &mut close.stats;
            match name {
                "definition" => stats.definition = Some(count()?),
                "files" => stats.files = Some(count()?),
                "seconds" => stats.seconds = Some(count()?),
                "failures" => stats.failures = Some(count()?),
                "late" => stats.late = Some(count()?),
                "late_unresolved" => stats.late_unresolved = Some(count()?),
                "max_back_s" => stats.max_back_s = Some(count()?),
                "vendor_refused" => stats.vendor_refused = Some(count()?),
                "months" => stats.months = Some(count()?),
                "run" => stats.run = Some(count()?),
                "stage" | "reason" => {
                    let decoded = percent_decoded(value)?;
                    let (kept, cut) = capped(decoded);
                    close.cut |= cut;
                    if name == "stage" {
                        close.stage = Some(kept);
                    } else {
                        close.reason = Some(kept);
                    }
                }
                // A FIELD A LATER BUILD ADDS IS KEPT OUT OF THE ROW, NOT A
                // REASON TO DROP THE LINE: the loader refuses what it cannot
                // read because it decides what to import; a page only shows.
                _ => {}
            }
        }
        Some(close)
    }
}

/// `text` cut at [`MAX_VALUE_BYTES`] on a character boundary, and whether it
/// was cut.
fn capped(text: String) -> (String, bool) {
    if text.len() <= MAX_VALUE_BYTES {
        return (text, false);
    }
    let mut end = MAX_VALUE_BYTES;
    while !text.is_char_boundary(end) {
        end = end.saturating_sub(1);
    }
    (text.get(..end).unwrap_or_default().to_owned(), true)
}

/// A count as the journal writes one: decimal, no sign, no leading zero but
/// `0` itself.
fn canonical_count(text: &str) -> Option<u64> {
    let value: u64 = text.parse().ok()?;
    (value.to_string() == text).then_some(value)
}

/// A key's kind and day, or `None` for a word that is not a kind or a day
/// that is not `YYYY-MM-DD`.
fn key_parts(kind: &str, day: &str) -> Option<(ImportKind, Day)> {
    Some((ImportKind::parse(kind).ok()?, parse_day(day)?))
}

/// `YYYY-MM-DD`, exactly as [`Day`] displays one.
pub(crate) fn parse_day(text: &str) -> Option<Day> {
    let mut parts = text.splitn(3, '-');
    let year: u16 = parts.next()?.parse().ok()?;
    let month: u8 = parts.next()?.parse().ok()?;
    let day: u8 = parts.next()?.parse().ok()?;
    let parsed = Day::new(year, month, day).ok()?;
    (parsed.to_string() == text).then_some(parsed)
}

/// A percent-encoded value decoded, or `None` for a malformed escape or bytes
/// that are not UTF-8. D-3317 writes every byte outside the unreserved set,
/// letters, digits and `-._~/`, as `%XX`.
fn percent_decoded(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut at = 0usize;
    while let Some(&byte) = bytes.get(at) {
        if byte == b'%' {
            let high = hex_value(*bytes.get(at.checked_add(1)?)?)?;
            let low = hex_value(*bytes.get(at.checked_add(2)?)?)?;
            out.push(high.checked_mul(16)?.checked_add(low)?);
            at = at.checked_add(3)?;
        } else {
            out.push(byte);
            at = at.checked_add(1)?;
        }
    }
    String::from_utf8(out).ok()
}

/// One hex digit's value.
const fn hex_value(digit: u8) -> Option<u8> {
    match digit {
        b'0'..=b'9' => Some(digit - b'0'),
        b'a'..=b'f' => Some(digit - b'a' + 10),
        b'A'..=b'F' => Some(digit - b'A' + 10),
        _ => None,
    }
}

/// One read of the journal: what it holds, and how the file stood.
#[derive(Debug, Clone)]
pub(crate) struct Snapshot {
    /// The journal's path.
    pub(crate) path: PathBuf,
    /// Whether the file is there. An absent journal is a store no import has
    /// run against, which is a fact, not a failure.
    pub(crate) present: bool,
    /// Its length when read.
    pub(crate) bytes: u64,
    /// Bytes after its last newline: a line being written, or a crash's
    /// fragment the importer's next load closes. Never parsed.
    pub(crate) unterminated: u64,
    /// Bytes this call read: the whole file the first time, the new bytes
    /// after that.
    pub(crate) read_now: u64,
    /// What every whole line says.
    pub(crate) parsed: Arc<Parsed>,
}

/// What [`snapshot`] keeps between calls for one path.
#[derive(Debug)]
struct Tail {
    /// The file's identity when read: a replaced file is read again.
    identity: Option<(u64, u64)>,
    /// Bytes parsed, through the last newline.
    upto: u64,
    parsed: Arc<Parsed>,
}

/// Every path read so far. Keyed by path, so a test's store and the served
/// store never share a parse.
fn tails() -> &'static Mutex<std::collections::HashMap<PathBuf, Tail>> {
    static TAILS: OnceLock<Mutex<std::collections::HashMap<PathBuf, Tail>>> = OnceLock::new();
    TAILS.get_or_init(|| Mutex::new(std::collections::HashMap::new()))
}

/// The journal under `store_root`, read through its tail. See the module
/// header for the cost.
///
/// # Errors
///
/// A file that is there and cannot be measured or read, in the failure's own
/// words. An absent journal is not an error: it is an empty snapshot with
/// `present: false`.
pub(crate) fn snapshot(store_root: &Path) -> Result<Snapshot, String> {
    snapshot_at(&pull::gdfl_import::journal_path(store_root))
}

/// [`snapshot`] of a journal at `path`.
fn snapshot_at(path: &Path) -> Result<Snapshot, String> {
    use std::io::{Read as _, Seek as _};
    let meta = match std::fs::metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Snapshot {
                path: path.to_path_buf(),
                present: false,
                bytes: 0,
                unterminated: 0,
                read_now: 0,
                parsed: Arc::new(Parsed::default()),
            });
        }
        Err(e) => return Err(format!("{}: {e}", path.display())),
        Ok(meta) => meta,
    };
    let identity = Some(identity_of(&meta));
    let len = meta.len();
    let mut held = tails().lock().unwrap_or_else(PoisonError::into_inner);
    let tail = held.entry(path.to_path_buf()).or_insert_with(|| Tail {
        identity,
        upto: 0,
        parsed: Arc::new(Parsed::default()),
    });
    // A FILE THAT SHRANK OR IS ANOTHER FILE IS READ AGAIN FROM THE START. The
    // journal is append-only, so neither happens to a journal the importer
    // writes; a restored copy or a test's fixture is exactly that case.
    if tail.identity != identity || len < tail.upto {
        *tail = Tail {
            identity,
            upto: 0,
            parsed: Arc::new(Parsed::default()),
        };
    }
    let mut read_now = 0u64;
    if len > tail.upto {
        let mut file = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
        file.seek(std::io::SeekFrom::Start(tail.upto))
            .map_err(|e| format!("{}: {e}", path.display()))?;
        let mut fresh = Vec::new();
        file.take(len.saturating_sub(tail.upto))
            .read_to_end(&mut fresh)
            .map_err(|e| format!("{}: {e}", path.display()))?;
        read_now = u64::try_from(fresh.len()).unwrap_or(u64::MAX);
        if let Some(last) = fresh.iter().rposition(|&byte| byte == b'\n') {
            let whole = fresh.get(..last).unwrap_or_default();
            let parsed = Arc::make_mut(&mut tail.parsed);
            let mut number = parsed.lines;
            for line in whole.split(|&byte| byte == b'\n') {
                number = number.saturating_add(1);
                match std::str::from_utf8(line) {
                    Ok(text) => parsed.line(number, text),
                    // NOT UTF-8 IS NOT A LINE THIS READER CAN NAME. Counted
                    // and shown lossily, never dropped in silence.
                    Err(_) => parsed.line(number, &String::from_utf8_lossy(line)),
                }
            }
            tail.upto = tail
                .upto
                .saturating_add(u64::try_from(last).unwrap_or(u64::MAX))
                .saturating_add(1);
        }
    }
    Ok(Snapshot {
        path: path.to_path_buf(),
        present: true,
        bytes: len,
        unterminated: len.saturating_sub(tail.upto),
        read_now,
        parsed: Arc::clone(&tail.parsed),
    })
}

/// A file's device and inode, so a replaced file reads as another file.
fn identity_of(meta: &std::fs::Metadata) -> (u64, u64) {
    use std::os::unix::fs::MetadataExt as _;
    (meta.dev(), meta.ino())
}

#[cfg(test)]
#[allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "a test that cannot panic cannot fail, and these lints exist to \
              keep panics out of the crate rather than out of its tests"
)]
pub(crate) mod tests {
    use super::*;

    /// A store root of its own, with `lines` as its journal.
    pub(crate) fn journal(name: &str, lines: &str) -> PathBuf {
        let root = crate::scratch::path(&format!("importjournal-{name}"));
        let _ = std::fs::remove_dir_all(&root);
        let path = pull::gdfl_import::journal_path(&root);
        std::fs::create_dir_all(path.parent().expect("imports/")).expect("mkdir");
        std::fs::write(&path, lines).expect("write the journal");
        root
    }

    fn day(text: &str) -> Day {
        parse_day(text).expect("a day")
    }

    /// The row of one key of the whole-tree filter, by walking its one day.
    fn row_of(parsed: &Parsed, kind: ImportKind, on: Day) -> Option<DayRow> {
        parsed
            .range(on, on)
            .find(|row| row.kind == kind && row.filter == "*")
            .cloned()
    }

    /// The journal's shapes as the importer writes them, before and after
    /// D-3317, in one file. The numbers are invented.
    pub(crate) const MIXED: &str = "\
        begin indices 2022-03-04 *\n\
        done indices 2022-03-04 * files=2 seconds=44891 failures=0 late=0 late_unresolved=0 max_back_s=0\n\
        begin options 2019-11-13 *\n\
        done options 2019-11-13 * definition=2 files=11 seconds=222 failures=0 late=0 late_unresolved=0 max_back_s=0\n\
        begin options 2019-11-14 *\n\
        incomplete options 2019-11-14 * definition=2 files=12 seconds=333 failures=1 late=0 late_unresolved=0 max_back_s=0\n\
        run 1791300000 source=zip archive=/tmp/a%20b.zip archive_bytes=7 commit=abc started=1791300000 archive_cd_blake3=00\n\
        begin indices 2022-03-07 *\n\
        refused indices 2022-03-07 * definition=2 stage=vendor reason=price_refused failures=2 run=1791300000\n\
        begin options 2019-11-15 *\n\
        incomplete options 2019-11-15 * definition=2 files=9 seconds=99 failures=1 late=0 late_unresolved=0 max_back_s=0 stage=census reason=too_many_entries run=1791300000\n\
        begin options 2018-11-06 *\n\
        done options 2018-11-06 * definition=2 files=5 seconds=55 failures=1 late=0 late_unresolved=0 max_back_s=0 vendor_refused=1 reason=price_refused run=1791300000\n\
        recounted options 2019-11-14 * definition=2 months=3 run=1791300001\n\
        begin options 2019-11-18 *\n\
    ";

    /// **EVERY LINE SHAPE D-3317 FIXES, AND THE OLDER ONES, READ BY NAME.**
    /// The last closing line of a key wins, as the importer's loader rules; a
    /// `recounted` day is done and remembers it was incomplete; a key begun
    /// and not closed is `open`; a run line's archive path is decoded.
    #[test]
    fn every_line_the_importer_writes_is_read_and_the_last_close_wins() {
        let root = journal("shapes", MIXED);
        let snap = snapshot(&root).expect("reads");
        assert!(snap.present);
        let parsed = &snap.parsed;
        assert_eq!(parsed.lines, 15);
        assert_eq!(parsed.foreign_count, 0, "{:?}", parsed.foreign);
        assert_eq!(parsed.keys(), 7);

        let legacy = row_of(parsed, ImportKind::Indices, day("2022-03-04")).expect("a row");
        assert_eq!(legacy.outcome, DayOutcome::Done);
        assert_eq!(legacy.stats.files, Some(2));
        assert_eq!(legacy.stats.definition, None);
        assert_eq!((legacy.stage.as_deref(), legacy.stats.run), (None, None));

        let refused = row_of(parsed, ImportKind::Indices, day("2022-03-07"))
            .expect("a refused day has a row");
        assert_eq!(refused.outcome, DayOutcome::Refused);
        assert_eq!(refused.stage.as_deref(), Some("vendor"));
        assert_eq!(refused.reason.as_deref(), Some("price_refused"));
        assert_eq!(refused.stats.failures, Some(2));
        assert_eq!(refused.line, 9);

        let census = row_of(parsed, ImportKind::Options, day("2019-11-15")).expect("a row");
        assert_eq!(census.outcome, DayOutcome::Incomplete);
        assert_eq!(census.stage.as_deref(), Some("census"));
        assert_eq!(census.census_closes, 1);

        let with_refusals = row_of(parsed, ImportKind::Options, day("2018-11-06")).expect("a row");
        assert_eq!(with_refusals.outcome, DayOutcome::DoneWithRefusals);
        assert_eq!(with_refusals.stats.vendor_refused, Some(1));

        let recounted = row_of(parsed, ImportKind::Options, day("2019-11-14")).expect("a row");
        assert_eq!(recounted.outcome, DayOutcome::Done);
        assert!(recounted.recounted);
        assert_eq!(recounted.previous, Some(DayOutcome::Incomplete));
        assert_eq!(recounted.stats.months, Some(3));
        assert_eq!(recounted.closes, 2);

        let open = row_of(parsed, ImportKind::Options, day("2019-11-18")).expect("a row");
        assert_eq!(open.outcome, DayOutcome::Open);
        assert_eq!(open.attempts, 1);

        assert_eq!(parsed.runs.len(), 1);
        assert_eq!(parsed.runs[0].id, 1_791_300_000);
        assert_eq!(
            parsed.runs[0]
                .fields
                .iter()
                .find(|(k, _)| k == "archive")
                .map(|(_, v)| v.as_str()),
            Some("/tmp/a b.zip")
        );
        assert_eq!(
            parsed.runs[0]
                .fields
                .iter()
                .find(|(k, _)| k == "source")
                .map(|(_, v)| v.as_str()),
            Some("zip")
        );
        let options = parsed.last[2].as_ref().expect("options last");
        assert_eq!((options.0, options.2.as_str()), (15, "begin"));
    }

    /// **A RANGE IS A RANGE OF THE MAP**: only the days asked for, oldest
    /// first, and both ends inclusive.
    #[test]
    fn a_range_holds_exactly_its_days_both_ends_included() {
        let root = journal("range", MIXED);
        let snap = snapshot(&root).expect("reads");
        let days: Vec<String> = snap
            .parsed
            .range(day("2019-11-13"), day("2019-11-15"))
            .map(|row| row.day.to_string())
            .collect();
        assert_eq!(days, ["2019-11-13", "2019-11-14", "2019-11-15"]);
        assert_eq!(
            snap.parsed
                .range(day("2019-11-16"), day("2019-11-17"))
                .count(),
            0
        );
        assert_eq!(
            snap.parsed.span(),
            Some((day("2018-11-06"), day("2022-03-07")))
        );
    }

    /// **A LINE THIS READER CANNOT NAME IS COUNTED AND NAMED, AND THE PAGE
    /// STILL ANSWERS**; a line a crash cut and a load closed is counted as
    /// torn; the bytes after the last newline are never parsed.
    #[test]
    fn foreign_torn_and_unterminated_lines_are_counted_never_guessed() {
        let root = journal(
            "foreign",
            "begin indices 2022-03-04 *\nhello world\ndone ind (torn)\n\
             done indices 2022-03-04 * files=x\nbegin options 2019-11-1",
        );
        let snap = snapshot(&root).expect("reads");
        let parsed = &snap.parsed;
        assert_eq!(parsed.lines, 4);
        assert_eq!(parsed.foreign_count, 2, "{:?}", parsed.foreign);
        assert_eq!(parsed.foreign[0], (2, "hello world".to_owned()));
        assert_eq!(parsed.foreign[1].0, 4);
        assert_eq!((parsed.torn_count, parsed.torn.clone()), (1, vec![3]));
        assert_eq!(snap.unterminated, "begin options 2019-11-1".len() as u64);
        assert_eq!(
            row_of(parsed, ImportKind::Indices, day("2022-03-04"))
                .expect("row")
                .outcome,
            DayOutcome::Open
        );
    }

    /// **THE TAIL READS ONLY WHAT WAS APPENDED**, finishes a line it saw half
    /// of, and a replaced file is read again from the start.
    #[test]
    fn the_tail_reads_only_appended_bytes_and_a_replaced_file_again() {
        use std::io::Write as _;
        let root = journal("tail", "begin indices 2022-03-04 *\ndone indices 2022");
        let path = pull::gdfl_import::journal_path(&root);
        let first = snapshot(&root).expect("reads");
        assert_eq!(first.parsed.lines, 1);
        let whole = std::fs::metadata(&path).expect("len").len();
        assert_eq!(first.read_now, whole);

        let rest = "-03-04 * files=1 seconds=1 failures=0 late=0 late_unresolved=0 max_back_s=0\n";
        std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .expect("open")
            .write_all(rest.as_bytes())
            .expect("append");
        let second = snapshot(&root).expect("reads");
        assert_eq!(
            second.read_now,
            ("done indices 2022".len() + rest.len()) as u64,
            "the half line is read again, nothing before it"
        );
        assert_eq!(second.unterminated, 0);
        assert_eq!(
            row_of(&second.parsed, ImportKind::Indices, day("2022-03-04"))
                .expect("row")
                .outcome,
            DayOutcome::Done
        );
        let third = snapshot(&root).expect("reads");
        assert_eq!(third.read_now, 0, "nothing new, nothing read");
        assert!(Arc::ptr_eq(&second.parsed, &third.parsed));

        // REPLACED: a new file at the same path is another inode.
        std::fs::remove_file(&path).expect("remove");
        std::fs::write(&path, "begin stocks 2020-01-01 *\n").expect("write");
        let fourth = snapshot(&root).expect("reads");
        assert_eq!(fourth.parsed.lines, 1);
        assert!(row_of(&fourth.parsed, ImportKind::Indices, day("2022-03-04")).is_none());
    }

    /// An absent journal is a present `false` and an empty parse, not an
    /// error; a journal path that is a directory is an error by name.
    #[test]
    fn an_absent_journal_is_a_fact_and_an_unreadable_one_is_named() {
        let root = crate::scratch::path("importjournal-absent");
        let _ = std::fs::remove_dir_all(&root);
        let snap = snapshot(&root).expect("absent is not an error");
        assert!(!snap.present);
        assert_eq!(snap.parsed.keys(), 0);

        std::fs::create_dir_all(pull::gdfl_import::journal_path(&root)).expect("a directory");
        let why = snapshot(&root).expect_err("a directory is not a journal");
        assert!(why.contains("gdfl.journal"), "{why}");
    }

    /// Each word the wire uses is `pull`'s own, and parses back.
    #[test]
    fn every_outcome_word_round_trips_and_four_are_pulls_own() {
        for outcome in DayOutcome::ALL {
            assert_eq!(DayOutcome::parse(outcome.as_str()), Some(outcome));
        }
        assert_eq!(DayOutcome::Done.as_str(), "done");
        assert_eq!(DayOutcome::DoneWithRefusals.as_str(), "done_with_refusals");
        assert_eq!(DayOutcome::parse("finished"), None);
        assert!(DayOutcome::DoneWithRefusals.is_done());
        assert!(!DayOutcome::Open.is_done());
    }

    /// Percent escapes decode; a broken one refuses the line.
    #[test]
    fn a_percent_escape_decodes_and_a_broken_one_is_refused() {
        assert_eq!(percent_decoded("a%20b%2Fc").as_deref(), Some("a b/c"));
        assert_eq!(percent_decoded("%zz"), None);
        assert_eq!(percent_decoded("%2"), None);
        assert_eq!(percent_decoded("%FF"), None, "not UTF-8");
        assert_eq!(canonical_count("007"), None);
        assert_eq!(canonical_count("7"), Some(7));
        assert_eq!(parse_day("2019-1-14"), None);
        let (kept, cut) = capped("é".repeat(60));
        assert!(cut && kept.len() <= MAX_VALUE_BYTES && kept.is_char_boundary(kept.len()));
    }

    /// A `done` under another definition is restated: the importer reads it
    /// again, and the row says so.
    #[test]
    fn a_done_line_under_another_definition_is_restated() {
        let root = journal(
            "restated",
            "done stocks 2020-01-01 * definition=1 files=1\n\
             done stocks 2020-01-02 * definition=2 files=1\n",
        );
        let snap = snapshot(&root).expect("reads");
        let old = row_of(&snap.parsed, ImportKind::Stocks, day("2020-01-01")).expect("row");
        assert!(old.restated());
        let new = row_of(&snap.parsed, ImportKind::Stocks, day("2020-01-02")).expect("row");
        assert!(!new.restated());
    }

    /// CMP-07: a share's day from 2026-08-03 built under 2 or 3 is restated
    /// (its closing auction's band changed what it folds, D-4878 item 10),
    /// while the same definitions stay current for a share's day before it,
    /// for an index day and, under 3, for an option day; under this build's
    /// definition nothing is restated.
    #[test]
    fn a_share_day_of_the_auction_era_under_an_older_definition_is_restated() {
        let root = journal(
            "auction-era",
            "done stocks 2026-07-31 * definition=3 files=1\n\
             done stocks 2026-08-03 * definition=2 files=1\n\
             done stocks 2026-08-04 * definition=3 files=1\n\
             done stocks 2026-08-05 * definition=4 files=1\n\
             done indices 2026-08-04 * definition=2 files=2\n\
             done options 2026-08-04 * definition=3 files=2\n",
        );
        let snap = snapshot(&root).expect("reads");
        let restated = |kind, text| {
            row_of(&snap.parsed, kind, day(text))
                .expect("row")
                .restated()
        };
        assert!(!restated(ImportKind::Stocks, "2026-07-31"));
        assert!(restated(ImportKind::Stocks, "2026-08-03"));
        assert!(restated(ImportKind::Stocks, "2026-08-04"));
        assert!(!restated(ImportKind::Stocks, "2026-08-05"));
        assert!(!restated(ImportKind::Indices, "2026-08-04"));
        assert!(!restated(ImportKind::Options, "2026-08-04"));
    }
}

/// Tests added by the cloud mutation pass (cloud/mut-api-b-20261016): each one
/// pins a behaviour a surviving mutant broke, or reaches a line no other test
/// reached. The name says the behaviour; the mutant it kills is named beside
/// it.
#[cfg(test)]
#[allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "a test that cannot panic cannot fail, and these lints exist to \
              keep panics out of the crate rather than out of its tests"
)]
mod cloud_mutant_tests {
    use super::tests::journal;
    use super::*;

    fn day(text: &str) -> Day {
        parse_day(text).expect("a day")
    }

    fn row(parsed: &Parsed, kind: ImportKind, on: &str) -> DayRow {
        parsed
            .range(day(on), day(on))
            .find(|row| row.kind == kind && row.filter == "*")
            .cloned()
            .expect("a row for the key")
    }

    /// An empty line is skipped: it is counted as a line and is never a
    /// foreign one (kills deleting the `return` for an empty line).
    #[test]
    fn an_empty_line_is_counted_and_never_foreign() {
        let root = journal(
            "cloud-empty-line",
            "begin indices 2022-03-04 *\n\nbegin stocks 2022-03-04 *\n",
        );
        let parsed = snapshot(&root).expect("reads").parsed;
        assert_eq!(parsed.lines, 3);
        assert_eq!(parsed.foreign_count, 0, "{:?}", parsed.foreign);
        assert!(parsed.foreign.is_empty());
        assert_eq!(parsed.keys(), 2);
    }

    /// Past [`MAX_NAMED_FAULTS`] torn and foreign lines are counted but no
    /// longer named: exactly 32 of each are kept by number (kills `<` to `<=`
    /// on both caps).
    #[test]
    fn faults_past_the_cap_are_counted_and_only_the_first_32_named() {
        let mut text = String::new();
        for n in 0..40 {
            text.push_str(&format!("done ind{n} (torn)\n"));
        }
        for n in 0..40 {
            text.push_str(&format!("nonsense {n}\n"));
        }
        let root = journal("cloud-fault-cap", &text);
        let parsed = snapshot(&root).expect("reads").parsed;
        assert_eq!(parsed.torn_count, 40);
        assert_eq!(parsed.torn.len(), MAX_NAMED_FAULTS);
        assert_eq!(parsed.torn, (1..=32).collect::<Vec<u64>>());
        assert_eq!(parsed.foreign_count, 40);
        assert_eq!(parsed.foreign.len(), MAX_NAMED_FAULTS);
        assert_eq!(parsed.foreign[31], (72, "nonsense 31".to_owned()));
    }

    /// A key's first close has no previous outcome; a `begin` after a close
    /// remembers the close it reopens, and a first `begin` remembers nothing
    /// (kills `>` to `>=`, `==` and `<` on both `closes > 0` guards).
    #[test]
    fn previous_is_set_only_when_an_earlier_close_existed() {
        let root = journal(
            "cloud-previous",
            "done stocks 2020-01-01 * definition=4 files=1\n\
             begin stocks 2020-01-02 *\n\
             done stocks 2020-01-03 * definition=4 files=1\n\
             begin stocks 2020-01-03 *\n",
        );
        let parsed = snapshot(&root).expect("reads").parsed;
        let first_close = row(&parsed, ImportKind::Stocks, "2020-01-01");
        assert_eq!(first_close.outcome, DayOutcome::Done);
        assert_eq!(first_close.previous, None);
        assert_eq!((first_close.closes, first_close.attempts), (1, 0));

        let first_begin = row(&parsed, ImportKind::Stocks, "2020-01-02");
        assert_eq!(first_begin.outcome, DayOutcome::Open);
        assert_eq!(first_begin.previous, None);
        assert_eq!((first_begin.closes, first_begin.attempts), (0, 1));

        let reopened = row(&parsed, ImportKind::Stocks, "2020-01-03");
        assert_eq!(reopened.outcome, DayOutcome::Open);
        assert_eq!(reopened.previous, Some(DayOutcome::Done));
        assert_eq!((reopened.closes, reopened.attempts), (1, 1));
        assert_eq!(reopened.line, 4);
    }

    /// Every count a closing line can carry is read into its own field, an
    /// unknown field is kept out of the row without dropping the line, and
    /// `vendor_refused=0` is a plain `done` (kills each deleted `Close::read`
    /// arm and `> 0` to `>= 0` or a constant guard).
    #[test]
    fn every_count_is_read_by_name_and_zero_vendor_refusals_is_plain_done() {
        let root = journal(
            "cloud-every-count",
            "done stocks 2020-01-06 * definition=4 files=1 seconds=2 failures=3 late=4 \
             late_unresolved=5 max_back_s=6 vendor_refused=0 months=7 run=8 \
             stage=fold reason=none future_field=kept%20out\n",
        );
        let parsed = snapshot(&root).expect("reads").parsed;
        assert_eq!(parsed.foreign_count, 0, "{:?}", parsed.foreign);
        let done = row(&parsed, ImportKind::Stocks, "2020-01-06");
        assert_eq!(
            done.stats,
            Stats {
                definition: Some(4),
                files: Some(1),
                seconds: Some(2),
                failures: Some(3),
                late: Some(4),
                late_unresolved: Some(5),
                max_back_s: Some(6),
                vendor_refused: Some(0),
                months: Some(7),
                run: Some(8),
            }
        );
        assert_eq!(done.outcome, DayOutcome::Done);
        assert!(!done.recounted);
        assert_eq!(done.stage.as_deref(), Some("fold"));
        assert_eq!(done.reason.as_deref(), Some("none"));
        assert!(!done.cut);
        assert_eq!(done.census_closes, 0);
    }

    /// A stage or reason longer than [`MAX_VALUE_BYTES`] is cut and the row
    /// says so, whichever of the two was long (kills `|=` to `&=`).
    #[test]
    fn a_long_stage_or_reason_is_cut_and_the_row_says_so() {
        let long = "x".repeat(MAX_VALUE_BYTES + 4);
        let root = journal(
            "cloud-cut",
            &format!(
                "refused stocks 2020-01-07 * definition=4 stage={long} reason=short failures=1\n\
                 refused stocks 2020-01-08 * definition=4 stage=short reason={long} failures=1\n\
                 refused stocks 2020-01-09 * definition=4 stage=short reason=short failures=1\n"
            ),
        );
        let parsed = snapshot(&root).expect("reads").parsed;
        let long_stage = row(&parsed, ImportKind::Stocks, "2020-01-07");
        assert!(long_stage.cut);
        assert_eq!(
            long_stage.stage.as_deref().map(str::len),
            Some(MAX_VALUE_BYTES)
        );
        assert_eq!(long_stage.reason.as_deref(), Some("short"));
        let long_reason = row(&parsed, ImportKind::Stocks, "2020-01-08");
        assert!(long_reason.cut);
        assert_eq!(
            long_reason.reason.as_deref().map(str::len),
            Some(MAX_VALUE_BYTES)
        );
        let neither = row(&parsed, ImportKind::Stocks, "2020-01-09");
        assert!(!neither.cut);
        assert_eq!(neither.outcome, DayOutcome::Refused);
    }

    /// `capped` keeps a value of exactly the cap whole and uncut, cuts one
    /// byte more to the cap, and steps back off a split character rather
    /// than answering empty (kills `<=` to `>` and the deleted `!`).
    #[test]
    fn capped_keeps_the_cap_and_steps_back_off_a_split_character() {
        let at_cap = "a".repeat(MAX_VALUE_BYTES);
        assert_eq!(capped(at_cap.clone()), (at_cap.clone(), false));
        assert_eq!(capped("short".to_owned()), ("short".to_owned(), false));
        assert_eq!(capped(format!("{at_cap}b")), (at_cap, true));
        // One ASCII byte then two-byte characters: byte 96 falls inside one.
        let split = format!("a{}", "é".repeat(60));
        let (kept, cut) = capped(split);
        assert!(cut);
        assert_eq!(kept, format!("a{}", "é".repeat(47)));
        assert_eq!(kept.len(), MAX_VALUE_BYTES - 1);
    }

    /// Lowercase escapes decode exactly as uppercase ones do (kills the
    /// deleted `a..=f` arm and its arithmetic mutants).
    #[test]
    fn lowercase_and_uppercase_escapes_decode_alike() {
        assert_eq!(percent_decoded("%2f%2F%3a%3A").as_deref(), Some("//::"));
        assert_eq!(percent_decoded("%7e%5f").as_deref(), Some("~_"));
        assert_eq!(hex_value(b'a'), Some(10));
        assert_eq!(hex_value(b'f'), Some(15));
        assert_eq!(hex_value(b'A'), Some(10));
        assert_eq!(hex_value(b'F'), Some(15));
        assert_eq!(hex_value(b'0'), Some(0));
        assert_eq!(hex_value(b'9'), Some(9));
        assert_eq!(hex_value(b'g'), None);
        assert_eq!(hex_value(b'G'), None);
    }

    /// `restated` is about a done day only: a refused day and a recounted day
    /// under an old definition are not restated (kills `&&` to `||` on the
    /// first conjunction and the deleted `!` before `recounted`).
    #[test]
    fn only_a_done_unrecounted_day_under_an_old_definition_is_restated() {
        let root = journal(
            "cloud-restated",
            "refused stocks 2020-01-05 * definition=1 stage=x reason=y failures=1\n\
             recounted stocks 2020-01-06 * definition=1 months=1\n\
             done stocks 2020-01-07 * definition=1 files=1\n\
             begin stocks 2020-01-08 *\n",
        );
        let parsed = snapshot(&root).expect("reads").parsed;
        assert!(!row(&parsed, ImportKind::Stocks, "2020-01-05").restated());
        let recounted = row(&parsed, ImportKind::Stocks, "2020-01-06");
        assert!(recounted.recounted);
        assert!(!recounted.restated());
        assert!(row(&parsed, ImportKind::Stocks, "2020-01-07").restated());
        assert!(!row(&parsed, ImportKind::Stocks, "2020-01-08").restated());
    }

    /// The run-id index: `runs_of`, `runs_before` and `nth_of` answer from
    /// the positions each id's lines were filed at, and `None` for an id or a
    /// line of it the journal does not hold.
    #[test]
    fn the_run_index_names_each_ids_lines_and_nothing_else() {
        let root = journal(
            "cloud-run-index",
            "run 5 a=1\nrun 7 b=2\nrun 5 c=3\nbegin stocks 2020-01-01 *\n",
        );
        let parsed = snapshot(&root).expect("reads").parsed;
        assert_eq!(parsed.runs.len(), 3);
        assert_eq!(parsed.runs_unindexed, 0);
        let lines: Vec<u64> = parsed.runs_of(5).map(|run| run.line).collect();
        assert_eq!(lines, [1, 3]);
        assert_eq!(
            parsed.runs_of(7).map(|run| run.line).collect::<Vec<_>>(),
            [2]
        );
        assert_eq!(parsed.runs_of(9).count(), 0);
        assert_eq!(parsed.runs_before(5, 0), Some(0));
        assert_eq!(parsed.runs_before(5, 1), Some(2));
        assert_eq!(parsed.runs_before(7, 0), Some(1));
        assert_eq!(parsed.runs_before(5, 2), None);
        assert_eq!(parsed.runs_before(9, 0), None);
        assert_eq!(parsed.nth_of(0), Some(0));
        assert_eq!(parsed.nth_of(1), Some(0));
        assert_eq!(parsed.nth_of(2), Some(1));
        assert_eq!(parsed.nth_of(3), None);
    }

    /// A journal path that cannot be measured for any reason but absence is
    /// an error by name, never an absent journal (kills the `NotFound` guard
    /// replaced with `true`).
    #[test]
    fn a_journal_that_cannot_be_measured_is_named_not_called_absent() {
        let root = crate::scratch::path("importjournal-cloud-notdir");
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_file(&root);
        // The store root is a FILE, so `imports/gdfl.journal` under it is
        // ENOTDIR — not NotFound.
        std::fs::write(&root, b"not a directory").expect("a file");
        let why = snapshot(&root).expect_err("a path under a file is not absent");
        assert!(why.contains("gdfl.journal"), "{why}");
        std::fs::remove_file(&root).expect("cleanup");
    }

    /// A line that is not UTF-8 is counted as foreign and shown lossily.
    #[test]
    fn a_line_that_is_not_utf8_is_foreign_and_shown_lossily() {
        let root = journal("cloud-not-utf8", "begin indices 2022-03-04 *\n");
        let path = pull::gdfl_import::journal_path(&root);
        let mut bytes = b"begin indices 2022-03-04 *\n".to_vec();
        bytes.extend_from_slice(b"\xff\xfebad\n");
        std::fs::write(&path, bytes).expect("write");
        let parsed = snapshot(&root).expect("reads").parsed;
        assert_eq!(parsed.lines, 2);
        assert_eq!(parsed.foreign_count, 1);
        assert_eq!(parsed.foreign[0], (2, "\u{fffd}\u{fffd}bad".to_owned()));
    }

    /// Bytes appended without a newline are read, held as unterminated and
    /// never parsed.
    #[test]
    fn an_append_with_no_newline_is_read_and_held_unterminated() {
        use std::io::Write as _;
        let root = journal("cloud-no-newline", "begin indices 2022-03-04 *\n");
        let path = pull::gdfl_import::journal_path(&root);
        let first = snapshot(&root).expect("reads");
        assert_eq!((first.parsed.lines, first.unterminated), (1, 0));
        std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .expect("open")
            .write_all(b"done ind")
            .expect("append");
        let second = snapshot(&root).expect("reads");
        assert_eq!(second.read_now, 8);
        assert_eq!(second.unterminated, 8);
        assert_eq!(second.parsed.lines, 1);
        assert!(Arc::ptr_eq(&first.parsed, &second.parsed));
    }

    /// A journal truncated in place (same inode, shorter) is read again from
    /// the start, never answered from the stale parse (kills `||` to `&&` and
    /// `len < upto` to `==` or `>`).
    #[test]
    fn a_journal_truncated_in_place_is_read_again() {
        let root = journal(
            "cloud-truncated",
            "begin indices 2022-03-04 *\nbegin options 2019-11-13 *\n",
        );
        let path = pull::gdfl_import::journal_path(&root);
        assert_eq!(snapshot(&root).expect("reads").parsed.lines, 2);
        // `fs::write` truncates the SAME file: the inode does not change.
        std::fs::write(&path, "begin stocks 2020-01-01 *\n").expect("rewrite");
        let again = snapshot(&root).expect("reads");
        assert_eq!(again.parsed.lines, 1);
        assert_eq!(again.read_now, 26);
        assert_eq!(again.parsed.keys(), 1);
        assert_eq!(
            row(&again.parsed, ImportKind::Stocks, "2020-01-01").outcome,
            DayOutcome::Open
        );
    }

    /// A journal replaced by a LONGER file (another inode) is read again from
    /// the start: only the identity can tell, since the length grew (kills
    /// `identity_of` returning a constant and `||` to `&&`).
    #[test]
    fn a_journal_replaced_by_a_longer_file_is_read_again() {
        let root = journal("cloud-replaced-longer", "begin indices 2022-03-04 *\n");
        let path = pull::gdfl_import::journal_path(&root);
        assert_eq!(snapshot(&root).expect("reads").parsed.lines, 1);
        // Written beside it and renamed over it, so both inodes exist at once
        // and the new one cannot reuse the old one's number.
        let beside = path.with_extension("next");
        std::fs::write(
            &beside,
            "begin stocks 2020-01-01 *\nbegin stocks 2020-01-02 *\n",
        )
        .expect("write the replacement");
        std::fs::rename(&beside, &path).expect("replace");
        let again = snapshot(&root).expect("reads");
        assert_eq!(again.parsed.lines, 2);
        assert_eq!(again.parsed.foreign_count, 0, "{:?}", again.parsed.foreign);
        assert!(
            again
                .parsed
                .range(day("2022-03-04"), day("2022-03-04"))
                .next()
                .is_none(),
            "the replaced file's rows must not survive"
        );
        assert_eq!(again.parsed.keys(), 2);
    }

    /// A journal that has not grown is answered from what was parsed without
    /// opening it again: a journal that can no longer be opened still answers
    /// (kills `len > upto` to `>=`). Root ignores modes, so this only bites
    /// for a non-root user — which is how the suite is meant to run.
    #[test]
    fn a_journal_that_has_not_grown_is_not_opened_again() {
        use std::os::unix::fs::PermissionsExt as _;
        let root = journal("cloud-not-reopened", "begin indices 2022-03-04 *\n");
        let path = pull::gdfl_import::journal_path(&root);
        let first = snapshot(&root).expect("reads");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000)).expect("chmod 000");
        let second = snapshot(&root);
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644))
            .expect("chmod back");
        let second = second.expect("nothing new: answered from the parse, not the file");
        assert_eq!(second.read_now, 0);
        assert!(Arc::ptr_eq(&first.parsed, &second.parsed));
    }
}

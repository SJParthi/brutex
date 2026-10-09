//! `GET /imports.json?feed=gdfl&from=<day>&to=<day>` -- the GDFL import
//! journal, by day: each day's kind, outcome (`done`, `done_with_refusals`,
//! `incomplete`, `refused` or `open`), stage, reason, files, seconds,
//! failures, late rows and `max_back_s`, the days the census refused, and the
//! runs that wrote them. DB-025, DB-G09, DB-G22; D-3317, D-3341.
//!
//! # Why this route exists
//!
//! The import journal was GDFL's whole audit trail and nothing served it: a
//! day whose every file was refused, a day the census refused and a day not
//! yet imported were the same hole on `/db`, `/audit` and `/ingest`, and the
//! reasons lived only in the `cli` log (DB-025, DB-G22). This route reads the
//! journal through [`crate::server::import_journal`], the one reader, and names each
//! day's standing as the journal records it.
//!
//! # A line with no stage, and the evidence beside it
//!
//! Every line written before D-3317 says `failures=1` and nothing else, so a
//! census refusal and a vendor's malformed row closed alike (DB-G09). The
//! importer's own `refused` events say which; this route puts them beside the
//! row (`logged`) and, when they settle it, fills `stage` with
//! `stage_from: "log"`: `census` when the importer refused at the census,
//! `calendar` when the calendar declined the day, and the event's own `stage`
//! when it carries one (D-3317 builds). A line whose evidence settles nothing
//! keeps `stage: null` and `stage_from: null`: unrecorded, never guessed.
//!
//! # Cost
//!
//! One `metadata` call and the journal's new bytes ([`crate::server::import_journal`]),
//! one walk of the asked range of days (O(rows in it)), and one hash probe per
//! row for its logged evidence ([`crate::logs::import_refusals`], which reads
//! the `cli` ring's new bytes on the way). A row the ring has rolled past is
//! probed in the `cli` audit stream's `day` posting: one more file per such
//! row. A page holds at most [`PAGE_ROWS`] rows.

use crate::logs::ImportRefusals;
use crate::server::import_journal::{DayOutcome, DayRow, Parsed, Snapshot};
use crate::server::{Loaded, param};
use pull::gdfl_import::{ImportKind, Stage};
use pull::session::Day;
use serde_json::{Value, json};

/// The most rows one page holds.
pub(crate) const PAGE_ROWS: usize = 200;

/// The content type every answer here carries.
fn json_header() -> [(axum::http::HeaderName, &'static str); 1] {
    [(
        axum::http::header::CONTENT_TYPE,
        "application/json; charset=utf-8",
    )]
}

/// What a request asked for, read and checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Asked {
    /// The first day, when named.
    pub(crate) from: Option<Day>,
    /// The last day, when named.
    pub(crate) to: Option<Day>,
    /// One kind, when named.
    pub(crate) kind: Option<ImportKind>,
    /// One outcome, when named.
    pub(crate) outcome: Option<DayOutcome>,
    /// Newest day first unless `order=asc`.
    pub(crate) newest_first: bool,
    /// The page, from 0.
    pub(crate) page: usize,
}

/// Reads a query string, or names the parameter it cannot read.
///
/// `feed` is required and must be `gdfl`: the import journal is GDFL's, and
/// every other feed's runs are broker pulls in `/audit.json`. A missing feed is
/// refused rather than defaulted, for the reason `/audit.json` gives.
///
/// # Errors
///
/// The parameter and why, as one sentence for a 400.
pub(crate) fn asked(query: &str) -> Result<Asked, String> {
    let feed = param(query, "feed");
    let gdfl = brutex_core::vendor::Vendor::Gdfl.as_str();
    if feed.is_empty() {
        return Err(format!(
            "/imports.json needs ?feed={gdfl} and never guesses one: the import journal is \
             {gdfl}'s; every other feed's runs are broker pulls, in /audit.json?feed=<wire>"
        ));
    }
    if !feed.eq_ignore_ascii_case(gdfl) {
        return Err(format!(
            "feed {feed:?} has no import journal: only {gdfl} is imported (cli gdfl-import, \
             imports/gdfl.journal). Its runs are broker pulls, in /audit.json?feed={feed}"
        ));
    }
    let day = |name: &str| -> Result<Option<Day>, String> {
        let text = param(query, name);
        if text.is_empty() {
            return Ok(None);
        }
        crate::server::import_journal::parse_day(&text)
            .map(Some)
            .ok_or_else(|| format!("{name}={text:?} is not a day as YYYY-MM-DD"))
    };
    let from = day("from")?;
    let to = day("to")?;
    if let (Some(from), Some(to)) = (from, to)
        && from > to
    {
        return Err(format!("from={from} is after to={to}"));
    }
    let kind_word = param(query, "kind");
    let kind = if kind_word.is_empty() {
        None
    } else {
        Some(ImportKind::parse(&kind_word).map_err(|why| why.to_string())?)
    };
    let outcome_word = param(query, "outcome");
    let outcome = if outcome_word.is_empty() {
        None
    } else {
        Some(DayOutcome::parse(&outcome_word).ok_or_else(|| {
            format!(
                "outcome={outcome_word:?} is not one of {}",
                DayOutcome::ALL.map(DayOutcome::as_str).join(", ")
            )
        })?)
    };
    let order = param(query, "order");
    let newest_first = match order.as_str() {
        "" | "desc" => true,
        "asc" => false,
        other => return Err(format!("order={other:?} is not asc or desc")),
    };
    let page_word = param(query, "page");
    let page = if page_word.is_empty() {
        0
    } else {
        page_word
            .parse::<usize>()
            .ok()
            .filter(|page| page.to_string() == page_word)
            .ok_or_else(|| format!("page={page_word:?} is not a whole number"))?
    };
    Ok(Asked {
        from,
        to,
        kind,
        outcome,
        newest_first,
        page,
    })
}

/// `GET /imports.json`.
pub(super) async fn imports_json(
    axum::extract::State(site): axum::extract::State<Loaded>,
    uri: axum::http::Uri,
) -> (
    axum::http::StatusCode,
    [(axum::http::HeaderName, &'static str); 1],
    String,
) {
    let query = uri.query().unwrap_or("").to_owned();
    let asked = match asked(&query) {
        Ok(asked) => asked,
        // THE REFUSAL IS THE BODY'S, and `logs::note_request` records the
        // path and the 400. No event of its own: this crate counts its emit
        // sites (`emitted`), and a refused poll is not a refusal of data.
        Err(why) => {
            return (
                axum::http::StatusCode::BAD_REQUEST,
                json_header(),
                json!({ "error": why }).to_string(),
            );
        }
    };
    let root = site.store_root.clone();
    match crate::detail::run_store_read(move || answer(&root, &asked)).await {
        Ok(Ok(body)) => (axum::http::StatusCode::OK, json_header(), body.to_string()),
        Ok(Err(why)) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            json_header(),
            json!({ "error": why }).to_string(),
        ),
        Err(why) => {
            let (status, body) = crate::detail::admission_refused(
                "import journal read",
                crate::detail::MAX_STORE_READ_CONCURRENT,
                &why,
            );
            (status, json_header(), body)
        }
    }
}

/// The whole answer over the store at `root`.
///
/// # Errors
///
/// A journal that is there and cannot be read, in its own words.
pub(crate) fn answer(root: &std::path::Path, asked: &Asked) -> Result<Value, String> {
    let snapshot = crate::server::import_journal::snapshot(root)?;
    let evidence = crate::logs::import_refusals(root);
    Ok(body(root, &snapshot, &evidence, asked))
}

impl Asked {
    /// The whole journal, newest day first, page 0.
    pub(crate) const fn whole() -> Self {
        Self {
            from: None,
            to: None,
            kind: None,
            outcome: None,
            newest_first: true,
            page: 0,
        }
    }
}

/// The import journal in brief, for `/audit.json?feed=gdfl` (DB-054, DB-G07,
/// D-3341 item 4): the journal's state, the count of days by outcome, the
/// days the census refused, where each kind stands, the newest runs, and the
/// newest day that is not done with its stage and reason. Never an error: a
/// journal that cannot be read is a `{"error": ...}` the console prints.
pub(crate) fn summary(root: &std::path::Path) -> Value {
    let whole = match answer(root, &Asked::whole()) {
        Ok(whole) => whole,
        Err(why) => return json!({ "error": why, "route": "/imports.json?feed=gdfl" }),
    };
    // THE NEWEST DAY THAT IS NOT DONE: the newest `incomplete`, `refused` or
    // `open` row, by its deciding line, so a run stopped at the census names
    // that day and why (DB-G07: "the last importer refusal").
    let newest = [
        DayOutcome::Incomplete,
        DayOutcome::Refused,
        DayOutcome::Open,
    ]
    .into_iter()
    .filter_map(|outcome| {
        let asked = Asked {
            outcome: Some(outcome),
            ..Asked::whole()
        };
        answer(root, &asked).ok()?.get("rows")?.get(0).cloned()
    })
    .max_by_key(|row| row.get("line").and_then(Value::as_u64).unwrap_or(0));
    let take = |key: &str| whole.get(key).cloned().unwrap_or(Value::Null);
    let asked = take("asked");
    let runs = take("runs");
    let census_then_done = take("census_refused_then_done");
    json!({
        "route": "/imports.json?feed=gdfl",
        "journal": take("journal"),
        "span": { "from": asked.get("from"), "to": asked.get("to") },
        "counts": take("counts"),
        "census_refused": take("census_refused"),
        "census_refused_then_done": census_then_done.get("count"),
        "kinds": take("kinds"),
        "runs": runs.as_array().map(|runs| runs.iter().take(20).cloned().collect::<Vec<_>>()),
        "runs_total": runs.as_array().map_or(0, Vec::len),
        "newest_not_done": newest,
        "notes": take("notes"),
    })
}

/// How a row's stage was settled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StageFrom {
    /// The journal line carries `stage=`.
    Journal,
    /// The importer's own `refused` event settles it.
    Log,
}

/// A row's stage and where it came from, or `None` when nothing settles it.
pub(crate) fn stage_of(row: &DayRow, logged: &[telemetry::Record]) -> Option<(String, StageFrom)> {
    if let Some(stage) = row.stage.clone() {
        return Some((stage, StageFrom::Journal));
    }
    if row.outcome.is_done() && row.stats.failures.is_none_or(|n| n == 0) {
        return None;
    }
    let field = |record: &telemetry::Record, key: &str| {
        record
            .field(key)
            .and_then(telemetry::OwnedValue::as_str)
            .map(str::to_owned)
    };
    // THE CENSUS DECIDES A DAY BEFORE ANY OTHER STAGE (D-3317's `decisive`):
    // a day the census refused is that, whatever else its files met.
    let census = Stage::Census.as_str();
    if logged.iter().any(|record| {
        field(record, "stage").as_deref() == Some(census)
            || field(record, "about").as_deref() == Some(census)
    }) {
        return Some((census.to_owned(), StageFrom::Log));
    }
    if let Some(stage) = logged.iter().find_map(|record| field(record, "stage")) {
        return Some((stage, StageFrom::Log));
    }
    let calendar = Stage::Calendar.as_str();
    logged
        .iter()
        .any(|record| field(record, "about").as_deref() == Some(calendar))
        .then(|| (calendar.to_owned(), StageFrom::Log))
}

/// The refusals the importer logged for a row: the `cli` ring's, or, when the
/// ring holds none and the line names a failure, the `cli` audit stream's.
fn logged_for(
    root: &std::path::Path,
    row: &DayRow,
    evidence: &ImportRefusals,
) -> (Vec<telemetry::Record>, &'static str) {
    let kind = row.kind.as_str();
    let day = row.day.to_string();
    let ring = evidence.of(kind, &day);
    if !ring.is_empty() {
        return (ring.to_vec(), "cli ring");
    }
    let failed = !row.outcome.is_done() || row.stats.failures.is_some_and(|n| n > 0);
    if failed && row.outcome != DayOutcome::Open {
        let audited = crate::logs::audited_import_refusals(root, kind, &day);
        if !audited.is_empty() {
            return (audited, "cli audit stream");
        }
    }
    (Vec::new(), "none")
}

/// One count as JSON: a number, or `null` when the line has none.
fn count(value: Option<u64>) -> Value {
    value.map_or(Value::Null, Value::from)
}

/// One row as the wire carries it.
fn row_json(root: &std::path::Path, row: &DayRow, evidence: &ImportRefusals) -> Value {
    let (logged, source) = logged_for(root, row, evidence);
    let stage = stage_of(row, &logged);
    let reason = row.reason.clone().or_else(|| {
        // A REASON CODE ONLY WHEN THE IMPORTER WROTE ONE (D-3317 builds log
        // `code`); a sentence is never turned into a code here.
        stage.as_ref().filter(|(_, from)| *from == StageFrom::Log)?;
        logged.iter().find_map(|record| {
            record
                .field("code")
                .and_then(telemetry::OwnedValue::as_str)
                .map(str::to_owned)
        })
    });
    let field = |record: &telemetry::Record, key: &str| -> Value {
        record
            .field(key)
            .and_then(telemetry::OwnedValue::as_str)
            .map_or(Value::Null, |text| Value::from(text.to_owned()))
    };
    let logged_json: Vec<Value> = logged
        .iter()
        .map(|record| {
            json!({
                "seq": record.seq,
                "at_ms": record.at_unix_millis,
                "message": record.message,
                "about": field(record, "about"),
                "why": field(record, "why"),
                "stage": field(record, "stage"),
                "code": field(record, "code"),
            })
        })
        .collect();
    json!({
        "kind": row.kind.as_str(),
        "day": row.day.to_string(),
        "filter": row.filter,
        "outcome": row.outcome.as_str(),
        "stage": stage.as_ref().map(|(stage, _)| stage.clone()),
        "stage_from": stage.as_ref().map(|(_, from)| match from {
            StageFrom::Journal => "journal",
            StageFrom::Log => "log",
        }),
        "reason": reason,
        "files": count(row.stats.files),
        "seconds": count(row.stats.seconds),
        "failures": count(row.stats.failures),
        "late": count(row.stats.late),
        "late_unresolved": count(row.stats.late_unresolved),
        "max_back_s": count(row.stats.max_back_s),
        "vendor_refused": count(row.stats.vendor_refused),
        "months": count(row.stats.months),
        "definition": count(row.stats.definition),
        "run": count(row.stats.run),
        "recounted": row.recounted,
        "restated": row.restated(),
        "line": (row.line > 0).then_some(row.line),
        "outcome_from": if row.line > 0 { "journal" } else { "log" },
        "attempts": row.attempts,
        "closes": row.closes,
        "census_closes": row.census_closes,
        "previous": row.previous.map(DayOutcome::as_str),
        "cut": row.cut,
        "logged": logged_json,
        "logged_from": source,
    })
}

/// Whether a row is one the request asked for.
fn wanted(row: &DayRow, asked: &Asked) -> bool {
    asked.kind.is_none_or(|kind| kind == row.kind)
        && asked.outcome.is_none_or(|outcome| outcome == row.outcome)
}

/// A day with a logged refusal and no journal line: the shape of every day a
/// build before D-3317 refused whole (every file refused, or the calendar
/// declined it), which closed with no line at all (DB-025). It wrote nothing,
/// which is D-3317's `refused`; the row says it is read from the log, with
/// `line: null` and `outcome_from: log`, and carries the importer's own words.
/// Its `failures` is the distinct refusals (`about`, `why`) the log names.
fn logged_only(kind: ImportKind, day: Day, failures: usize) -> DayRow {
    DayRow {
        kind,
        day,
        filter: "*".to_owned(),
        outcome: DayOutcome::Refused,
        recounted: false,
        stats: crate::server::import_journal::Stats {
            failures: u64::try_from(failures).ok(),
            ..crate::server::import_journal::Stats::default()
        },
        stage: None,
        reason: None,
        cut: false,
        line: 0,
        attempts: 0,
        closes: 0,
        census_closes: 0,
        previous: None,
    }
}

/// What one walk of the asked range found.
#[derive(Debug, Default)]
struct Walk<'p> {
    /// Rows by outcome, in [`DayOutcome::ALL`] order, over the range and kind.
    counts: [u64; DayOutcome::ALL.len()],
    /// `<kind> <day>` of every day the census refused and is not done.
    census_refused: Vec<String>,
    /// `<kind> <day>` of every day the census refused and that later closed.
    census_then_done: Vec<String>,
    /// The rows the request asked for, oldest first: the journal's, and a
    /// [`logged_only`] row for each day the log alone names.
    rows: Vec<std::borrow::Cow<'p, DayRow>>,
}

/// ONE WALK OF THE RANGE: the counts, the census days and the rows asked
/// for. O(rows in the range), one evidence probe each.
fn walk<'p>(
    parsed: &'p Parsed,
    range: Option<(Day, Day)>,
    asked: &Asked,
    evidence: &ImportRefusals,
) -> Walk<'p> {
    let mut out = Walk::default();
    let Some((from, to)) = range.filter(|(from, to)| from <= to) else {
        return out;
    };
    for row in merged(parsed, from, to, evidence) {
        if asked.kind.is_some_and(|kind| kind != row.kind) {
            continue;
        }
        if let Some(slot) = DayOutcome::ALL
            .iter()
            .position(|outcome| *outcome == row.outcome)
            .and_then(|at| out.counts.get_mut(at))
        {
            *slot = slot.saturating_add(1);
        }
        let logged = evidence.of(row.kind.as_str(), &row.day.to_string());
        let census =
            stage_of(&row, logged).is_some_and(|(stage, _)| stage == Stage::Census.as_str());
        let name = format!("{} {}", row.kind.as_str(), row.day);
        if census && !row.outcome.is_done() {
            out.census_refused.push(name);
        } else if row.census_closes > 0 || (row.recounted && row.previous.is_some()) {
            out.census_then_done.push(name);
        }
        if wanted(&row, asked) {
            out.rows.push(row);
        }
    }
    out
}

/// The journal's rows of `from..=to` and a [`logged_only`] row for each day
/// the importer's log names and the journal does not, in day order, then
/// kind order. O(rows + logged days in the range), each logged day checked
/// against the journal by one range probe.
fn merged<'p>(
    parsed: &'p Parsed,
    from: Day,
    to: Day,
    evidence: &ImportRefusals,
) -> Vec<std::borrow::Cow<'p, DayRow>> {
    let mut rows: Vec<std::borrow::Cow<'p, DayRow>> = parsed
        .range(from, to)
        .map(std::borrow::Cow::Borrowed)
        .collect();
    for (kind, day) in evidence.days(&from.to_string(), &to.to_string()) {
        let (Ok(kind), Some(day)) = (
            ImportKind::parse(kind),
            crate::server::import_journal::parse_day(day),
        ) else {
            continue;
        };
        let journalled = parsed
            .range(day, day)
            .any(|row| row.kind == kind && row.filter == "*");
        if !journalled {
            // DISTINCT REFUSALS, NOT LOG LINES: a day attempted by two runs
            // logs each refusal twice, and `logged` lists every line.
            let failures = evidence
                .of(kind.as_str(), &day.to_string())
                .iter()
                .map(|record| {
                    (
                        record
                            .field("about")
                            .and_then(telemetry::OwnedValue::as_str),
                        record.field("why").and_then(telemetry::OwnedValue::as_str),
                    )
                })
                .collect::<std::collections::BTreeSet<_>>()
                .len();
            rows.push(std::borrow::Cow::Owned(logged_only(kind, day, failures)));
        }
    }
    rows.sort_by_key(|row| {
        (
            row.day,
            ImportKind::ALL.iter().position(|known| *known == row.kind),
            row.filter.clone(),
        )
    });
    rows
}

/// Where each kind stands: its last line, and its pace, measured, never
/// projected: the gaps between the importer's own newest `day imported`
/// events. No ETA: the days still owed depend on the archive's last day and
/// the calendar, neither of which this journal records.
fn kinds_json(parsed: &Parsed, evidence: &ImportRefusals) -> Vec<Value> {
    ImportKind::ALL
        .iter()
        .zip(parsed.last.iter())
        .map(|(kind, last)| {
            let measured = evidence.pace(kind.as_str()).map(|(gaps, p50, p90)| {
                json!({
                    "gaps": gaps,
                    "p50_ms_per_day": p50,
                    "p90_ms_per_day": p90,
                    "days_per_hour_at_p50": if p50 > 0 { 3_600_000 / p50 } else { 0 },
                    "newest_at_ms": evidence.newest_import(kind.as_str()),
                    "from": "the importer's `day imported` events in the cli log",
                })
            });
            json!({
                "kind": kind.as_str(),
                "last_line": last.as_ref().map(|last| last.0),
                "last_day": last.as_ref().map(|last| last.1.to_string()),
                "last_verb": last.as_ref().map(|last| last.2.clone()),
                "pace": measured,
            })
        })
        .collect()
}

/// Every `run` line, newest first, its fields decoded.
fn runs_json(parsed: &Parsed) -> Vec<Value> {
    parsed
        .runs
        .iter()
        .rev()
        .map(|run| {
            let fields: serde_json::Map<String, Value> = run
                .fields
                .iter()
                .map(|(name, value)| (name.clone(), Value::from(value.clone())))
                .collect();
            json!({ "id": run.id, "line": run.line, "fields": fields })
        })
        .collect()
}

/// The journal file's own state.
fn journal_json(snapshot: &Snapshot) -> Value {
    let parsed = &snapshot.parsed;
    let span = parsed.span();
    json!({
        "path": snapshot.path.display().to_string(),
        "present": snapshot.present,
        "bytes": snapshot.bytes,
        "lines": parsed.lines,
        "keys": parsed.keys(),
        "unterminated": snapshot.unterminated,
        "read_now": snapshot.read_now,
        "first_day": span.map(|(first, _)| first.to_string()),
        "last_day": span.map(|(_, last)| last.to_string()),
        "foreign": {
            "count": parsed.foreign_count,
            "lines": parsed
                .foreign
                .iter()
                .map(|(line, text)| json!({ "line": line, "text": text }))
                .collect::<Vec<_>>(),
        },
        "torn": { "count": parsed.torn_count, "lines": parsed.torn },
    })
}

/// The sentences an answer owes its reader about what it could not read.
fn notes_of(snapshot: &Snapshot, rows: &[Value]) -> Vec<String> {
    let mut notes: Vec<String> = Vec::new();
    if !snapshot.present {
        notes.push(format!(
            "No import journal at {}: no GDFL import has run against this store.",
            snapshot.path.display()
        ));
    }
    if snapshot.parsed.foreign_count > 0 {
        notes.push(format!(
            "{} line(s) of the journal are not lines this reader names; they are listed under \
             journal.foreign and left out of every row. The importer refuses such a journal \
             (D-2814), so its next run will name them too.",
            snapshot.parsed.foreign_count
        ));
    }
    if snapshot.unterminated > 0 {
        notes.push(format!(
            "The journal's last {} byte(s) end without a newline: a line being written, or a \
             crash's fragment the importer closes on its next load. Not read.",
            snapshot.unterminated
        ));
    }
    let unsettled = rows.iter().any(|row| {
        row.get("stage").is_some_and(Value::is_null)
            && row.get("outcome").and_then(Value::as_str) != Some(DayOutcome::Done.as_str())
    });
    if unsettled {
        notes.push(
            "A row with stage null and stage_from null was closed by a line written before \
             D-3317, which records no stage, and the importer's log holds no refusal that \
             settles it: unrecorded, not guessed."
                .to_owned(),
        );
    }
    notes
}

/// The answer, from a snapshot and the log's evidence, with no clock and no
/// I/O beyond [`logged_for`]'s audit probe.
pub(crate) fn body(
    root: &std::path::Path,
    snapshot: &Snapshot,
    evidence: &ImportRefusals,
    asked: &Asked,
) -> Value {
    let parsed: &Parsed = &snapshot.parsed;
    // THE RANGE ECHOED IS THE RANGE WALKED: the asked ends, else the journal's
    // own first and last day. An empty journal with no asked ends has none.
    let range = match (parsed.span(), asked.from, asked.to) {
        (Some((first, last)), from, to) => Some((from.unwrap_or(first), to.unwrap_or(last))),
        (None, Some(from), Some(to)) => Some((from, to)),
        (None, _, _) => None,
    };
    let mut walked = walk(parsed, range, asked, evidence);
    let total = walked.rows.len();
    let pages = total.saturating_sub(1) / PAGE_ROWS + 1;
    // CLAMPED, NOT REFUSED: a page past the end is a stale bookmark, and the
    // answer says which page it is.
    let page = asked.page.min(pages.saturating_sub(1));
    if asked.newest_first {
        walked.rows.reverse();
    }
    let rows: Vec<Value> = walked
        .rows
        .iter()
        .skip(page.saturating_mul(PAGE_ROWS))
        .take(PAGE_ROWS)
        .map(|row| row_json(root, row, evidence))
        .collect();
    let notes = notes_of(snapshot, &rows);
    json!({
        "feed": brutex_core::vendor::Vendor::Gdfl.as_str(),
        "journal": journal_json(snapshot),
        "asked": {
            "from": range.map(|(from, _)| from.to_string()),
            "to": range.map(|(_, to)| to.to_string()),
            "kind": asked.kind.map(ImportKind::as_str),
            "outcome": asked.outcome.map(DayOutcome::as_str),
            "order": if asked.newest_first { "desc" } else { "asc" },
        },
        "page": page,
        "pages": pages,
        "page_rows": PAGE_ROWS,
        "rows_in_range": total,
        "counts": DayOutcome::ALL
            .iter()
            .zip(walked.counts)
            .map(|(outcome, n)| (outcome.as_str().to_owned(), Value::from(n)))
            .collect::<serde_json::Map<String, Value>>(),
        "census_refused": { "count": walked.census_refused.len(), "days": walked.census_refused },
        "census_refused_then_done": {
            "count": walked.census_then_done.len(),
            "days": walked.census_then_done,
        },
        "kinds": kinds_json(parsed, evidence),
        "runs": runs_json(parsed),
        "rows": rows,
        "evidence": {
            "cli_ring_files": evidence.files,
            "cli_ring_bytes": evidence.bytes,
            "malformed": evidence.malformed,
            "errors": evidence.errors,
        },
        "notes": notes,
    })
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
mod tests {
    use super::*;
    use crate::server::tests::{agreeing, site};

    fn ask(query: &str) -> Asked {
        asked(query).unwrap_or_else(|why| panic!("{query}: {why}"))
    }

    /// The census days of the operator's journal, as the shape of it: legacy
    /// lines with no stage, one day done in the middle, a vendor-refused day
    /// before, and the run stopped mid-day at the end. Invented numbers.
    const JOURNAL: &str = "\
begin options 2018-11-06 *
incomplete options 2018-11-06 * definition=2 files=3 seconds=30 failures=1 late=0 late_unresolved=0 max_back_s=0
begin options 2019-11-13 *
done options 2019-11-13 * definition=2 files=4 seconds=40 failures=0 late=0 late_unresolved=0 max_back_s=0
begin options 2019-11-14 *
incomplete options 2019-11-14 * definition=2 files=5 seconds=50 failures=1 late=0 late_unresolved=0 max_back_s=0
begin options 2019-12-27 *
done options 2019-12-27 * definition=2 files=6 seconds=60 failures=0 late=0 late_unresolved=0 max_back_s=0
begin options 2020-01-01 *
incomplete options 2020-01-01 * definition=2 files=7 seconds=70 failures=1 late=2 late_unresolved=0 max_back_s=3
begin indices 2022-03-04 *
done indices 2022-03-04 * files=2 seconds=20 failures=0 late=0 late_unresolved=0 max_back_s=0
begin indices 2022-03-07 *
refused indices 2022-03-07 * definition=2 stage=vendor reason=price_refused failures=2 run=7
begin options 2020-09-16 *
";

    /// The importer's own refusal events for those days, as the `cli` ring
    /// holds them: the census by `about`, a vendor row by its ticker. Invented.
    fn cli_log(root: &std::path::Path) {
        let dir = root.join("logs").join("cli");
        std::fs::create_dir_all(&dir).expect("mkdir");
        let line = |seq: u32, kind: &str, day: &str, about: &str, why: &str| {
            format!(
                "{{\"seq\":{seq},\"ts\":\"2026-10-05T03:31:18.967Z\",\"ms\":{},\"level\":\"error\",\
                 \"target\":\"gdfl.import\",\"msg\":\"refused\",\"fields\":{{\"kind\":\"{kind}\",\
                 \"day\":\"{day}\",\"about\":\"{about}\",\"why\":\"{why}\"}}}}\n",
                1_791_171_078_967_u64 + u64::from(seq)
            )
        };
        let mut text = String::new();
        text.push_str(&line(
            1,
            "options",
            "2018-11-06",
            "ZZTEST15NOV1899900CE",
            "line 9: the LTP is not a two-place decimal",
        ));
        text.push_str(&line(
            2,
            "options",
            "2019-11-14",
            "census",
            "n_valid is 9; this build holds at most 8",
        ));
        text.push_str(&line(
            3,
            "options",
            "2020-01-01",
            "census",
            "n_valid is 9; this build holds at most 8",
        ));
        text.push_str("{\"seq\":4,\"ts\":\"2026-10-05T03:31:18.967Z\",\"ms\":1791171078971,\"level\":\"info\",\"target\":\"gdfl.import\",\"msg\":\"day imported\",\"fields\":{\"kind\":\"options\",\"day\":\"2019-11-14\"}}\n");
        std::fs::write(dir.join("events.ndjson"), text).expect("write the cli log");
    }

    fn store(name: &str) -> std::path::PathBuf {
        let root =
            crate::server::import_journal::tests::journal(&format!("imports-{name}"), JOURNAL);
        cli_log(&root);
        root
    }

    /// **THE OPERATOR'S CHECK, ON THE JOURNAL'S OWN SHAPE**: the days the
    /// census refused are `incomplete` with `stage: census` (from the
    /// importer's log, the lines carrying none), the one day done in their
    /// middle is `done`, and the census days are listed. DB-025, DB-G09,
    /// DB-G22.
    #[test]
    fn the_census_days_read_incomplete_at_the_census_and_the_day_between_done() {
        let root = store("census");
        let got = answer(&root, &ask("feed=gdfl&from=2019-11-01&to=2020-08-31")).expect("reads");
        let rows = got["rows"].as_array().expect("rows");
        assert_eq!(rows.len(), 4, "{got:#}");
        // NEWEST FIRST.
        assert_eq!(rows[0]["day"], "2020-01-01");
        assert_eq!(rows[0]["outcome"], "incomplete");
        assert_eq!(rows[0]["stage"], "census");
        assert_eq!(rows[0]["stage_from"], "log");
        assert_eq!(rows[0]["reason"], Value::Null, "no code is invented");
        assert_eq!(rows[0]["late"], 2);
        assert_eq!(rows[0]["max_back_s"], 3);
        assert_eq!(rows[0]["logged"][0]["about"], "census");
        assert_eq!(rows[0]["logged_from"], "cli ring");
        assert_eq!(rows[1]["day"], "2019-12-27");
        assert_eq!(rows[1]["outcome"], "done");
        assert_eq!(rows[1]["stage"], Value::Null);
        assert_eq!(rows[2]["day"], "2019-11-14");
        assert_eq!(rows[2]["stage"], "census");
        assert_eq!(
            got["census_refused"]["days"],
            json!(["options 2019-11-14", "options 2020-01-01"])
        );
        assert_eq!(rows[3]["day"], "2019-11-13");
        assert_eq!(rows[3]["outcome"], "done");
        assert_eq!(got["counts"]["incomplete"], 2);
        assert_eq!(got["counts"]["done"], 2);
        assert_eq!(got["rows_in_range"], 4);
        assert_eq!(got["journal"]["present"], true);
    }

    /// **A DAY WHOSE EVERY FILE WAS REFUSED HAS A ROW**, with the journal's
    /// own stage and reason (`stage_from: journal`); a vendor refusal before
    /// D-3317 keeps its evidence and a `null` stage it cannot settle; a day
    /// begun and never closed is `open`. DB-025.
    #[test]
    fn a_refused_day_an_unsettled_one_and_an_open_one_each_say_so() {
        let root = store("shapes");
        let got = answer(&root, &ask("feed=gdfl&order=asc")).expect("reads");
        let rows = got["rows"].as_array().expect("rows");
        let by_day = |day: &str| {
            rows.iter()
                .find(|row| row["day"] == day)
                .unwrap_or_else(|| panic!("{day} in {got:#}"))
        };
        let refused = by_day("2022-03-07");
        assert_eq!(refused["outcome"], "refused");
        assert_eq!(refused["stage"], "vendor");
        assert_eq!(refused["stage_from"], "journal");
        assert_eq!(refused["reason"], "price_refused");
        assert_eq!(refused["run"], 7);

        let unsettled = by_day("2018-11-06");
        assert_eq!(unsettled["outcome"], "incomplete");
        assert_eq!(unsettled["stage"], Value::Null);
        assert_eq!(unsettled["stage_from"], Value::Null);
        assert_eq!(unsettled["logged"][0]["about"], "ZZTEST15NOV1899900CE");

        let open = by_day("2020-09-16");
        assert_eq!(open["outcome"], "open");
        assert_eq!(got["counts"]["open"], 1);
        assert_eq!(rows[0]["day"], "2018-11-06", "oldest first when asked");
        let notes = got["notes"].as_array().expect("notes");
        assert!(
            notes
                .iter()
                .any(|n| n.as_str().unwrap().contains("unrecorded")),
            "{notes:?}"
        );
        assert_eq!(got["kinds"][2]["last_day"], "2020-09-16");
        assert_eq!(got["kinds"][2]["last_verb"], "begin");
    }

    /// **A DAY REFUSED WHOLE BEFORE D-3317 LEFT NO JOURNAL LINE, AND STILL
    /// HAS A ROW** (DB-025): read from the importer's log, `refused`, with
    /// `line: null`, `outcome_from: log` and its refusals; the calendar's
    /// refusal settles its stage. A day the journal names keeps its line.
    #[test]
    fn a_day_refused_whole_with_no_journal_line_is_read_from_the_log() {
        let root = store("logged-only");
        let dir = root.join("logs").join("cli");
        let extra = "{\"seq\":9,\"ts\":\"2026-10-05T03:31:18.967Z\",\"ms\":1791171078999,\
                     \"level\":\"error\",\"target\":\"gdfl.import\",\"msg\":\"refused\",\
                     \"fields\":{\"kind\":\"indices\",\"day\":\"2018-11-07\",\"about\":\"calendar\",\
                     \"why\":\"the calendar says OpenLengthUnmeasured, not a regular session\"}}\n";
        let mut text = std::fs::read_to_string(dir.join("events.ndjson")).unwrap();
        // TWICE: a second run refused the same day the same way.
        text.push_str(extra);
        text.push_str(&extra.replace("\"seq\":9", "\"seq\":10"));
        std::fs::write(dir.join("events.ndjson"), text).unwrap();
        let got = answer(&root, &ask("feed=gdfl&kind=indices&order=asc")).expect("reads");
        let rows = got["rows"].as_array().unwrap();
        assert_eq!(rows[0]["day"], "2018-11-07", "{got:#}");
        assert_eq!(rows[0]["outcome"], "refused");
        assert_eq!(rows[0]["outcome_from"], "log");
        assert_eq!(rows[0]["line"], Value::Null);
        assert_eq!(rows[0]["stage"], "calendar");
        assert_eq!(rows[0]["stage_from"], "log");
        assert_eq!(rows[0]["failures"], 1, "one refusal, logged twice");
        assert_eq!(rows[0]["logged"].as_array().unwrap().len(), 2);
        assert_eq!(rows[1]["day"], "2022-03-04");
        assert_eq!(rows[1]["outcome_from"], "journal");
        assert_eq!(got["counts"]["refused"], 2);
    }

    /// **A KIND, AN OUTCOME AND A PAGE NARROW THE ROWS AND NOT THE COUNTS**,
    /// which stay the range's; a page past the end is the last page.
    #[test]
    fn kind_outcome_and_page_narrow_the_rows() {
        let root = store("narrow");
        let got = answer(&root, &ask("feed=gdfl&kind=options&outcome=incomplete")).expect("reads");
        let days: Vec<&str> = got["rows"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| row["day"].as_str().unwrap())
            .collect();
        assert_eq!(days, ["2020-01-01", "2019-11-14", "2018-11-06"]);
        assert_eq!(
            got["counts"]["done"], 2,
            "counts are the kind's, not the outcome's"
        );
        let indices = answer(&root, &ask("feed=gdfl&kind=indices&page=9")).expect("reads");
        assert_eq!(indices["page"], 0);
        assert_eq!(indices["pages"], 1);
        assert_eq!(indices["rows"].as_array().unwrap().len(), 2);
    }

    /// **EVERY PARAMETER IS READ OR REFUSED BY NAME**: a missing or foreign
    /// feed, a day that is not one, a backwards range, an unknown kind,
    /// outcome or order, a page that is not a number.
    #[test]
    fn every_parameter_is_read_or_refused_by_name() {
        for (query, said) in [
            ("", "needs ?feed=gdfl"),
            ("feed=zerodha", "has no import journal"),
            ("feed=gdfl&from=2019-1-1", "from="),
            ("feed=gdfl&from=2020-01-02&to=2020-01-01", "is after"),
            ("feed=gdfl&kind=futures", "futures"),
            ("feed=gdfl&outcome=finished", "outcome="),
            ("feed=gdfl&order=up", "order="),
            ("feed=gdfl&page=-1", "page="),
        ] {
            let why = asked(query).expect_err(query);
            assert!(why.contains(said), "{query}: {why}");
        }
        let fine = ask("feed=GDFL&kind=options&outcome=done_with_refusals&order=asc&page=2");
        assert_eq!(fine.kind, Some(ImportKind::Options));
        assert_eq!(fine.outcome, Some(DayOutcome::DoneWithRefusals));
        assert!(!fine.newest_first);
        assert_eq!(fine.page, 2);
    }

    /// **THE AUDIT SUMMARY NAMES THE NEWEST DAY NOT DONE AND THE CENSUS
    /// DAYS**: the run stopped mid-day is `open` and newest by line; the
    /// census days are counted.
    #[test]
    fn the_summary_names_the_newest_day_not_done_and_the_census_days() {
        let root = store("summary");
        let got = summary(&root);
        assert_eq!(got["newest_not_done"]["day"], "2020-09-16", "{got:#}");
        assert_eq!(got["newest_not_done"]["outcome"], "open");
        assert_eq!(got["census_refused"]["count"], 2);
        assert_eq!(got["counts"]["refused"], 1);
        assert_eq!(got["span"]["from"], "2018-11-06");
        assert_eq!(got["journal"]["lines"], 15);
    }

    /// An absent journal answers, says so, and lists no row.
    #[test]
    fn an_absent_journal_answers_with_a_note_and_no_rows() {
        let root = crate::scratch::path("imports-absent");
        let _ = std::fs::remove_dir_all(&root);
        let got = answer(&root, &ask("feed=gdfl")).expect("absent is an answer");
        assert_eq!(got["journal"]["present"], false);
        assert_eq!(got["rows"], json!([]));
        assert!(
            got["notes"][0]
                .as_str()
                .unwrap()
                .contains("No import journal")
        );
    }

    /// **THE ROUTE, OVER THE REAL ROUTER STATE**: 200 with rows for gdfl, 400
    /// naming the feed for any other.
    #[tokio::test]
    async fn the_route_answers_gdfl_and_refuses_another_feed() {
        let dir = agreeing("imports-route");
        let mut built = site("imports-route", &dir);
        built.store_root = store("route");
        let loaded: Loaded = std::sync::Arc::new(built);
        let (status, _, body) = imports_json(
            axum::extract::State(std::sync::Arc::clone(&loaded)),
            "/imports.json?feed=gdfl&from=2019-11-01&to=2019-12-31"
                .parse::<axum::http::Uri>()
                .unwrap(),
        )
        .await;
        assert_eq!(status, axum::http::StatusCode::OK, "{body}");
        let got: Value = serde_json::from_str(&body).unwrap();
        assert_eq!(got["rows"][0]["day"], "2019-12-27");
        let (status, _, body) = imports_json(
            axum::extract::State(loaded),
            "/imports.json?feed=zerodha"
                .parse::<axum::http::Uri>()
                .unwrap(),
        )
        .await;
        assert_eq!(status, axum::http::StatusCode::BAD_REQUEST);
        assert!(body.contains("zerodha"), "{body}");
    }
}

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
    /// One run, when named (WP-33c; DB-A41): the journal's own run id on a
    /// day's closing line, or a `cli` attempt the importer's lines were logged
    /// under, or the importer run that attempt opened.
    pub(crate) run: Option<u64>,
    /// One stage, when named (DB-A47): the row's settled stage.
    pub(crate) stage: Option<String>,
    /// One refusal reason code, when named (DB-A47): the journal's `reason=`,
    /// the importer's logged `code`, or the reason the census's own wording
    /// names (see [`reason_of`]).
    pub(crate) reason: Option<String>,
    /// Text a logged refusal's `why` holds, when named (DB-A47): the search
    /// for a reason no code names, on rows written before codes existed.
    pub(crate) why: Option<String>,
    /// Which `run` lines the answer carries (D-4813 round 6; IA-11r R6).
    pub(crate) runs: RunsAsked,
}

/// Which `run` lines an answer carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RunsAsked {
    /// The newest [`RUNS_PAGE`], or with a cursor the [`RUNS_PAGE`] older than
    /// it: what `/imports.json` returns. The cursor is `runs_before=<id>`
    /// (older than run `id`'s oldest line) or `<id>.<n>` (older than its
    /// `n`-th line, from 0, for an id with several lines), and the answer's
    /// `runs_next` is the next one. O(page).
    Page(Option<(u64, usize)>),
    /// Every run, for a caller inside this crate that resolves a run by time
    /// and so needs the whole list: `/dayflags.json`'s day band
    /// (`dayflags::journal_rows`). O(runs), the cost it had before the page
    /// existed; never what the route returns.
    Every,
}

/// How many `run` lines one answer decodes and returns: the newest, or the
/// ones before a `runs_before=` cursor (D-4813 round 6; IA-LG-03).
pub(crate) const RUNS_PAGE: usize = 20;

/// Every parameter `/imports.json` reads. Any other is refused by name
/// (WP-33c, DB-A45): a filter this route does not apply must never come back
/// as an answer that looks filtered.
pub(crate) const PARAMS: [&str; 12] = [
    "feed",
    "from",
    "to",
    "kind",
    "outcome",
    "order",
    "page",
    "run",
    "stage",
    "reason",
    "why",
    "runs_before",
];

/// The longest `reason` or `why` a request may name.
pub(crate) const MAX_TEXT_ASKED: usize = 120;

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
    let unknown = crate::logs::unknown_params(query, &PARAMS);
    if !unknown.is_empty() {
        return Err(format!(
            "/imports.json does not read {}: refused rather than answered unfiltered. It reads {}",
            unknown.join(", "),
            PARAMS.join(", ")
        ));
    }
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
    let (run, stage, reason, why) = narrowing(query)?;
    Ok(Asked {
        from,
        to,
        kind,
        outcome,
        newest_first,
        page,
        run,
        stage,
        reason,
        why,
        runs: RunsAsked::Page(runs_cursor(&param(query, "runs_before"))?),
    })
}

/// A `runs_before=` word: `<id>` or `<id>.<n>`, each a whole number written
/// the one way, the id above 0. Empty is no cursor; anything else is refused
/// by name, never read as "the newest page".
fn runs_cursor(word: &str) -> Result<Option<(u64, usize)>, String> {
    if word.is_empty() {
        return Ok(None);
    }
    let refused = || {
        format!(
            "runs_before={word:?} is not a run cursor: a run id (a whole number above 0), or \
             <id>.<n> as an answer's runs_next gives it"
        )
    };
    let (id_word, nth_word) = word.split_once('.').unwrap_or((word, "0"));
    let id = id_word
        .parse::<u64>()
        .ok()
        .filter(|id| *id != 0 && id.to_string() == id_word)
        .ok_or_else(refused)?;
    let nth = nth_word
        .parse::<usize>()
        .ok()
        .filter(|nth| nth.to_string() == nth_word)
        .ok_or_else(refused)?;
    Ok(Some((id, nth)))
}

/// The run, stage, reason and logged words a query narrows the rows by
/// (WP-33c; DB-A41, DB-A47), each read or refused by name.
///
/// # Errors
///
/// The parameter and why, as one sentence for a 400.
#[expect(
    clippy::type_complexity,
    reason = "four independent narrowings, each optional, read in one pass"
)]
fn narrowing(
    query: &str,
) -> Result<(Option<u64>, Option<String>, Option<String>, Option<String>), String> {
    let run_word = param(query, "run");
    let run = if run_word.is_empty() {
        None
    } else {
        Some(
            run_word
                .parse::<u64>()
                .ok()
                .filter(|run| *run != 0 && run.to_string() == run_word)
                .ok_or_else(|| {
                    format!("run={run_word:?} is not a run id (a whole number above 0)")
                })?,
        )
    };
    let stage_word = param(query, "stage");
    let stage = if stage_word.is_empty() {
        None
    } else if Stage::ALL.iter().any(|stage| stage.as_str() == stage_word) {
        Some(stage_word)
    } else {
        return Err(format!(
            "stage={stage_word:?} is not one of {}",
            Stage::ALL.map(Stage::as_str).join(", ")
        ));
    };
    let text = |name: &str| -> Result<Option<String>, String> {
        let word = param(query, name);
        if word.is_empty() {
            return Ok(None);
        }
        if word.len() > MAX_TEXT_ASKED {
            return Err(format!(
                "{name}= is {} bytes; at most {MAX_TEXT_ASKED} are read",
                word.len()
            ));
        }
        Ok(Some(word))
    };
    Ok((run, stage, text("reason")?, text("why")?))
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
            run: None,
            stage: None,
            reason: None,
            why: None,
            runs: RunsAsked::Every,
        }
    }
}

/// The import journal in brief, for `/audit.json?feed=gdfl` (DB-054, DB-G07,
/// D-3341 item 4): the journal's state, the count of days by outcome, the
/// days the census refused, where each kind stands, the newest runs, and the
/// newest day that is not done with its stage and reason. Never an error: a
/// journal that cannot be read is a `{"error": ...}` the console prints.
///
/// # Cost (WP-33c; DB-A32)
///
/// It was four whole-journal [`answer`] walks per request, each rendering 200
/// rows (and probing the `cli` audit stream for each failed one): p50 8.6 ms
/// on the operator's journal, paid on every `/audit.json?feed=gdfl` poll. Now
/// it is ONE walk of the journal's days ([`summary_of`]), rendering one row,
/// and its answer is kept against the journal's parse and the importer log's
/// evidence it was computed from: while neither moves (every poll between
/// imports) a request is two tail reads that find nothing new, a lock and a
/// clone of the kept answer. Either moving recomputes it once.
pub(crate) fn summary(root: &std::path::Path) -> Value {
    type Kept = (
        std::sync::Arc<Parsed>,
        std::sync::Arc<ImportRefusals>,
        Value,
    );
    static KEPT: std::sync::OnceLock<
        std::sync::Mutex<std::collections::HashMap<std::path::PathBuf, Kept>>,
    > = std::sync::OnceLock::new();
    let snapshot = match crate::server::import_journal::snapshot(root) {
        Ok(snapshot) => snapshot,
        Err(why) => return json!({ "error": why, "route": "/imports.json?feed=gdfl" }),
    };
    let evidence = crate::logs::import_refusals(root);
    let kept = KEPT.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()));
    // THE SAME PARSE AND THE SAME EVIDENCE, BY POINTER. Both tails mutate
    // through `Arc::make_mut`, and the strong `Arc`s kept here make any change
    // a copy at a new address, so pointer equality is content equality.
    if let Some((parsed, refusals, value)) = kept
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .get(&snapshot.path)
        && std::sync::Arc::ptr_eq(parsed, &snapshot.parsed)
        && std::sync::Arc::ptr_eq(refusals, &evidence)
        && snapshot.present
    {
        return with_read_now(value.clone(), &snapshot);
    }
    let value = summary_of(root, &snapshot, &evidence);
    let _replaced = kept
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .insert(
            snapshot.path.clone(),
            (
                std::sync::Arc::clone(&snapshot.parsed),
                std::sync::Arc::clone(&evidence),
                value.clone(),
            ),
        );
    with_read_now(value, &snapshot)
}

/// `value` with `journal.read_now` set to what THIS call's read of the
/// journal read: a kept summary must not repeat the bytes the call that
/// computed it read.
fn with_read_now(mut value: Value, snapshot: &Snapshot) -> Value {
    if let Some(journal) = value.get_mut("journal").and_then(Value::as_object_mut) {
        let _replaced = journal.insert("read_now".to_owned(), Value::from(snapshot.read_now));
    }
    value
}

/// [`summary`]'s answer from one walk of every day the journal and the log
/// name: the counts, the census days, the newest row of each not-done outcome
/// and whether any not-done row is unsettled. One row is rendered: the newest
/// not done, by its deciding line.
pub(crate) fn summary_of(
    root: &std::path::Path,
    snapshot: &Snapshot,
    evidence: &ImportRefusals,
) -> Value {
    let parsed: &Parsed = &snapshot.parsed;
    let whole = Asked::whole();
    let range = parsed.span();
    let walked = walk(parsed, range, &whole, evidence);
    // THE NEWEST DAY THAT IS NOT DONE: the newest `incomplete`, `refused` or
    // `open` row, by its deciding line, so a run stopped at the census names
    // that day and why (DB-G07: "the last importer refusal"). Per outcome its
    // newest row by day (the last in the walk's day order), then the one of
    // those with the latest line.
    let newest = [
        DayOutcome::Incomplete,
        DayOutcome::Refused,
        DayOutcome::Open,
    ]
    .into_iter()
    .filter_map(|outcome| walked.rows.iter().rev().find(|row| row.outcome == outcome))
    .max_by_key(|row| row.line)
    .map(|row| row_json(root, row, evidence));
    let unsettled = walked.rows.iter().any(|row| {
        !row.outcome.is_done()
            && stage_of(row, &evidence.of(row.kind.as_str(), &row.day.to_string())).is_none()
    });
    let mut notes = notes_of(snapshot, &[]);
    if unsettled {
        notes.push(UNSETTLED.to_owned());
    }
    let runs = runs_page(parsed, RunsAsked::Page(None));
    json!({
        "route": "/imports.json?feed=gdfl",
        "journal": journal_json(snapshot),
        "span": {
            "from": range.map(|(from, _)| from.to_string()),
            "to": range.map(|(_, to)| to.to_string()),
        },
        "counts": DayOutcome::ALL
            .iter()
            .zip(walked.counts)
            .map(|(outcome, n)| (outcome.as_str().to_owned(), Value::from(n)))
            .collect::<serde_json::Map<String, Value>>(),
        "census_refused": {
            "count": walked.census_refused.len(),
            "days": walked.census_refused,
        },
        "census_refused_then_done": walked.census_then_done.len(),
        "kinds": kinds_json(parsed, evidence),
        "runs": runs.runs,
        "runs_total": runs.total,
        "runs_next": runs.next,
        "newest_not_done": newest,
        "notes": notes,
    })
}

/// The note a not-done row with no settled stage owes its reader.
const UNSETTLED: &str = "A row with stage null and stage_from null was closed by a line written \
     before D-3317, which records no stage, and the importer's log holds no refusal that \
     settles it: unrecorded, not guessed.";

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

/// The refusals the importer logged for a row, and where they were read:
/// the ring of each directory a run of the row's kind can write (`logs/cli`,
/// `logs/gdfl/<kind>`, `logs/gdfl/store`), or, when no ring holds any and the
/// line names a failure, those directories' audit streams (D-4862 round 6;
/// IA-11r R5).
///
/// # "none" means none
///
/// The source reads `"none"` only when every one of those directories was
/// read and holds nothing for the row. A directory that does not exist yet
/// (no run of that kind has logged there) or cannot be read is named in the
/// source instead, so "I looked and found nothing" and "I could not look"
/// never share a word (F78; `logs::cli_half`'s rule).
fn logged_for(
    root: &std::path::Path,
    row: &DayRow,
    evidence: &ImportRefusals,
) -> (Vec<telemetry::Record>, String) {
    let kind = row.kind.as_str();
    let day = row.day.to_string();
    let sources = |from: &[String], what: &str| -> String {
        from.iter()
            .map(|dir| format!("{dir} {what}"))
            .collect::<Vec<_>>()
            .join(" + ")
    };
    let ring = evidence.logged(kind, &day);
    if !ring.records.is_empty() {
        let source = sources(&ring.from, "ring");
        return (ring.records.into_owned(), source);
    }
    let mut unread = evidence.unread(kind);
    let failed = !row.outcome.is_done() || row.stats.failures.is_some_and(|n| n > 0);
    if failed && row.outcome != DayOutcome::Open {
        let audited = crate::logs::audited_import_refusals(root, kind, &day);
        if !audited.records.is_empty() {
            let source = sources(&audited.from, "audit stream");
            return (audited.records, source);
        }
        unread.extend(audited.unread);
    }
    if unread.is_empty() {
        return (Vec::new(), NONE_LOGGED.to_owned());
    }
    (
        Vec::new(),
        format!("nothing in the directories read; {}", unread.join("; ")),
    )
}

/// What a row's `logged_from` says when every directory a run of its kind
/// can write was read and none holds a refusal for it.
pub(crate) const NONE_LOGGED: &str = "none";

/// One count as JSON: a number, or `null` when the line has none.
fn count(value: Option<u64>) -> Value {
    value.map_or(Value::Null, Value::from)
}

/// A row's refusal reason code, and where it was read (DB-A47):
///
/// * `journal` -- the closing line's own `reason=` (D-3317 builds);
/// * `log code` -- the importer's logged `code`, when the log also settled
///   the row's stage (D-3317 builds log `code` on every refusal);
/// * `census wording` -- a row written before codes existed whose logged
///   census refusal is in the census's own words for a full census
///   (`pull::manifest::ManifestError::TooManyEntries`'s `Display`, in each
///   form it has had: [`CENSUS_FULL_WORDS`]): the importer codes that refusal
///   `too_many_entries` (`gdfl_import::census_code`). Read from the
///   producer's own wording, never from a guess at a sentence's meaning: the
///   census's other `n_valid is` refusal (`CounterExceedsRegion`, "the region
///   holds") matches none of them and keeps `reason: null`.
///
/// `None` when nothing settles it: unrecorded, not guessed.
pub(crate) fn reason_of(
    row: &DayRow,
    logged: &[telemetry::Record],
    stage: Option<&(String, StageFrom)>,
) -> Option<(String, &'static str)> {
    if let Some(reason) = row.reason.clone() {
        return Some((reason, "journal"));
    }
    let field = |record: &telemetry::Record, key: &str| {
        record
            .field(key)
            .and_then(telemetry::OwnedValue::as_str)
            .map(str::to_owned)
    };
    if stage.is_some_and(|(_, from)| *from == StageFrom::Log)
        && let Some(code) = logged.iter().find_map(|record| field(record, "code"))
    {
        return Some((code, "log code"));
    }
    let census = Stage::Census.as_str();
    logged
        .iter()
        .any(|record| {
            field(record, "about").as_deref() == Some(census)
                && field(record, "why").is_some_and(|why| {
                    why.starts_with("n_valid is ")
                        && CENSUS_FULL_WORDS.iter().any(|words| why.contains(words))
                })
        })
        .then(|| (CENSUS_FULL_CODE.to_owned(), "census wording"))
}

/// The census's own words for a full census, after `n_valid is <n>`, in
/// every form `pull::manifest::ManifestError::TooManyEntries` has written:
/// before WP-10 (`0a189286`) "; this build holds at most <limit>", since then
/// "; this census holds at most <limit>" for a census already past its limit
/// and " of a capacity of <limit> (...)" for a batch that does not fit. The
/// operator's `cli` log holds 210 census refusals, all in the first form
/// (counted 10 Oct 2026).
const CENSUS_FULL_WORDS: [&str; 3] = [
    "; this build holds at most ",
    "; this census holds at most ",
    " of a capacity of ",
];

/// The code the importer gives that refusal (`gdfl_import::census_code`).
const CENSUS_FULL_CODE: &str = "too_many_entries";

/// One row as the wire carries it.
fn row_json(root: &std::path::Path, row: &DayRow, evidence: &ImportRefusals) -> Value {
    let (logged, source) = logged_for(root, row, evidence);
    let stage = stage_of(row, &logged);
    let reason = reason_of(row, &logged, stage.as_ref());
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
        "reason": reason.as_ref().map(|(reason, _)| reason.clone()),
        "reason_from": reason.as_ref().map(|(_, from)| *from),
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
        "cli_attempts": evidence.attempts_of(row.kind.as_str(), &row.day.to_string()),
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

/// Whether a row is one the request asked for: its kind and outcome, and
/// (WP-33c) its run, stage, reason and logged words. The stage, reason and
/// words are read from the journal and the `cli` ring's evidence, with no I/O;
/// `run_attempt` is the `cli` attempt that opened the asked importer run, when
/// the ring names one.
fn wanted(
    row: &DayRow,
    asked: &Asked,
    logged: &[telemetry::Record],
    evidence: &ImportRefusals,
    run_attempt: Option<u64>,
) -> bool {
    let kind_day = || (row.kind.as_str(), row.day.to_string());
    asked.kind.is_none_or(|kind| kind == row.kind)
        && asked.outcome.is_none_or(|outcome| outcome == row.outcome)
        && asked.run.is_none_or(|run| {
            let (kind, day) = kind_day();
            let attempts = evidence.attempts_of(kind, &day);
            let attempts = attempts.as_slice();
            row.stats.run == Some(run)
                || attempts.contains(&run)
                || run_attempt.is_some_and(|attempt| attempts.contains(&attempt))
        })
        && asked
            .stage
            .as_deref()
            .is_none_or(|want| stage_of(row, logged).is_some_and(|(stage, _)| stage == want))
        && asked.reason.as_deref().is_none_or(|want| {
            let stage = stage_of(row, logged);
            reason_of(row, logged, stage.as_ref()).is_some_and(|(reason, _)| reason == want)
        })
        && asked.why.as_deref().is_none_or(|want| {
            logged.iter().any(|record| {
                record
                    .field("why")
                    .and_then(telemetry::OwnedValue::as_str)
                    .is_some_and(|why| why.contains(want))
            })
        })
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
    let run_attempt = asked
        .run
        .and_then(|run| evidence.attempt_of_importer_run(run));
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
        let logged: &[telemetry::Record] = &logged;
        let census =
            stage_of(&row, logged).is_some_and(|(stage, _)| stage == Stage::Census.as_str());
        let name = format!("{} {}", row.kind.as_str(), row.day);
        if census && !row.outcome.is_done() {
            out.census_refused.push(name);
        } else if row.census_closes > 0 || (row.recounted && row.previous.is_some()) {
            out.census_then_done.push(name);
        }
        if wanted(&row, asked, logged, evidence, run_attempt) {
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
            let measured = evidence.pace(kind.as_str()).map(|pace| {
                json!({
                    "gaps": pace.gaps,
                    "p50_ms_per_day": pace.p50,
                    "p90_ms_per_day": pace.p90,
                    "days_per_hour_at_p50": if pace.p50 > 0 { 3_600_000 / pace.p50 } else { 0 },
                    "newest_at_ms": pace.newest,
                    // THE DIRECTORY READ IS NAMED: a pace measured from
                    // `logs/cli` alone would be a frozen one, shown as
                    // measured, the day a kind's runs log elsewhere (F78).
                    "from": format!(
                        "the importer's `day imported` events in {}",
                        pace.from.join(", ")
                    ),
                    "from_dirs": pace.from,
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

/// What the journal and the logs say about one run id (DB-A41; D-4862 and
/// D-4813 round 6): the journal's `run` lines of that id, found through the
/// id index and no other line decoded; the `cli` attempt of that id (or the
/// one that opened that importer run), with the directory whose ring holds
/// it, its start, its finish or that none was logged; and the run's own
/// lifecycle events from `run claimed` to `finished`, read keyed through the
/// `run` posting of every directory a GDFL run can log to, never a ring walk.
fn run_json(root: &std::path::Path, parsed: &Parsed, evidence: &ImportRefusals, run: u64) -> Value {
    let lines: Vec<Value> = parsed.runs_of(run).rev().map(run_line_json).collect();
    let attempt_id = evidence
        .attempt(run)
        .map(|_| run)
        .or_else(|| evidence.attempt_of_importer_run(run));
    let attempt = attempt_id.and_then(|id| evidence.attempt(id).map(|found| (id, found)));
    let lifecycle = crate::logs::run_lifecycle(root, run);
    let shown = lifecycle.records.len().min(MAX_LIFECYCLE_SHOWN);
    let events: Vec<Value> = lifecycle
        .records
        .iter()
        .skip(lifecycle.records.len().saturating_sub(shown))
        .map(|record| {
            let fields: serde_json::Map<String, Value> = record
                .fields
                .iter()
                .map(|(name, value)| (name.clone(), field_json(value)))
                .collect();
            json!({
                "seq": record.seq,
                "at_ms": record.at_unix_millis,
                "level": record.level.label(),
                "message": record.message,
                "fields": fields,
            })
        })
        .collect();
    json!({
        "id": run,
        "journal_lines": lines,
        "attempt": attempt.map(|(id, (dir, attempt))| json!({
            "id": id,
            "command": attempt.command,
            "started_ms": attempt.started_ms,
            "finished_ms": attempt.finished_ms,
            "phase": attempt.phase,
            "finished": if attempt.finished_ms.is_some() {
                "command finished is logged"
            } else {
                "no command finished is logged: the attempt was stopped, or is still running"
            },
            "importer_runs": attempt.importer_runs,
            "importer_lines": attempt.lines,
            "dir": dir,
            "from": format!(
                "the {dir} ring: every line between this attempt's command started and its \
                 command finished is its own, because one process at a time writes {dir}"
            ),
        })),
        "lifecycle": {
            "events": events,
            "events_total": lifecycle.records.len(),
            "from": lifecycle.from,
            "unread": lifecycle.unread,
            "capped": lifecycle.capped,
            "read": format!(
                "the `run` posting of the audit stream of each of {}: keyed, at most {} \
                 pointers a directory, the newest {MAX_LIFECYCLE_SHOWN} events shown",
                crate::logs::import_dirs()
                    .map(|dir| format!("logs/{dir}"))
                    .collect::<Vec<_>>()
                    .join(", "),
                crate::logs::MAX_KEYED_READS,
            ),
        },
    })
}

/// `/dayflags.json`'s body for `query` over the store at `root`: the day
/// band's read, which calls [`answer`] with [`RunsAsked::Every`]. Here, and
/// not in `dayflags`, only because this module is the one of the two that a
/// caller outside `server` can reach; `logs::dayflags_json_over` is that
/// caller, for the bench that measures `docs/06-limits.md` L-IA-39.
pub(crate) fn day_band_json(root: &std::path::Path, query: &str) -> Result<String, String> {
    let asked = super::dayflags::asked(query)?;
    Ok(super::dayflags::answer(root, &asked).to_string())
}

/// How many of a run's lifecycle events `run=` shows: the newest.
/// `events_total` says how many were read.
const MAX_LIFECYCLE_SHOWN: usize = 64;

/// One log field as JSON: a number as a number, anything else as its text.
fn field_json(value: &telemetry::OwnedValue) -> Value {
    value.as_u64().map_or_else(
        || {
            value
                .as_str()
                .map_or_else(|| Value::from(format!("{value:?}")), Value::from)
        },
        Value::from,
    )
}

#[cfg(test)]
thread_local! {
    /// `run` lines decoded on this thread: what IA-LG-03 counts.
    static RUN_LINES_DECODED: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// One `run` line, its fields decoded.
fn run_line_json(run: &crate::server::import_journal::RunLine) -> Value {
    #[cfg(test)]
    RUN_LINES_DECODED.with(|count| count.set(count.get().saturating_add(1)));
    let fields: serde_json::Map<String, Value> = run
        .fields
        .iter()
        .map(|(name, value)| (name.clone(), Value::from(value.clone())))
        .collect();
    json!({ "id": run.id, "line": run.line, "fields": fields })
}

/// One page of `run` lines, newest first.
struct RunsPage {
    /// The lines of the page, decoded: at most [`RUNS_PAGE`] unless every run
    /// was asked for.
    runs: Vec<Value>,
    /// Every `run` line the journal holds, counted.
    total: usize,
    /// The cursor of the next older page, when there is one.
    next: Option<String>,
    /// Why a cursor could not be honoured, when it could not.
    refused: Option<String>,
}

/// The `run` lines an answer carries, newest first: only the lines of the
/// page are decoded, whatever the journal holds (D-4813 round 6; IA-11r R6;
/// IA-LG-03). `parsed.runs` is in file order, so the newest page is its end
/// and a cursor is a position the id index gives in one probe.
fn runs_page(parsed: &Parsed, asked: RunsAsked) -> RunsPage {
    let total = parsed.runs.len();
    let (end, take, refused) = match asked {
        RunsAsked::Every => (total, total, None),
        RunsAsked::Page(None) => (total, RUNS_PAGE, None),
        RunsAsked::Page(Some((id, nth))) => match parsed.runs_before(id, nth) {
            Some(end) => (end, RUNS_PAGE, None),
            // A CURSOR THAT NAMES NOTHING SHOWS NOTHING AND SAYS SO: the
            // newest page in its place would be read as the page asked for.
            None => (
                0,
                RUNS_PAGE,
                Some(format!(
                    "runs_before={} names no `run` line this journal holds, so no older page is \
                     shown; ask without runs_before for the newest runs",
                    // AS IT WAS ASKED: a bare id is not echoed as `<id>.0`.
                    if nth == 0 {
                        id.to_string()
                    } else {
                        format!("{id}.{nth}")
                    }
                )),
            ),
        },
    };
    let runs: Vec<Value> = parsed
        .runs
        .get(..end)
        .unwrap_or_default()
        .iter()
        .rev()
        .take(take)
        .map(run_line_json)
        .collect();
    // THE NEXT PAGE STARTS BELOW THE OLDEST LINE SHOWN, named by its id and,
    // when that id has older lines too, by which of its lines it is.
    let oldest_shown = end.saturating_sub(runs.len());
    let next = (oldest_shown > 0 && !runs.is_empty())
        .then(|| {
            let id = parsed.runs.get(oldest_shown)?.id;
            match parsed.nth_of(oldest_shown)? {
                0 => Some(id.to_string()),
                nth => Some(format!("{id}.{nth}")),
            }
        })
        .flatten();
    RunsPage {
        runs,
        total,
        next,
        refused,
    }
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
        notes.push(UNSETTLED.to_owned());
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
    let runs = runs_page(parsed, asked.runs);
    let dirs: Vec<Value> = evidence
        .dirs()
        .iter()
        .map(|dir| {
            json!({
                "dir": dir.name(),
                "path": dir.path.display().to_string(),
                "state": dir.state.word(),
                "error": match &dir.state {
                    crate::logs::DirState::Unreadable(why) => Value::from(why.clone()),
                    crate::logs::DirState::Read | crate::logs::DirState::Absent => Value::Null,
                },
                "ring_files": dir.files(),
                "ring_bytes": dir.bytes(),
                "malformed": dir.malformed(),
                "errors": dir.errors(),
            })
        })
        .collect();
    json!({
        "feed": brutex_core::vendor::Vendor::Gdfl.as_str(),
        "journal": journal_json(snapshot),
        "asked": {
            "from": range.map(|(from, _)| from.to_string()),
            "to": range.map(|(_, to)| to.to_string()),
            "kind": asked.kind.map(ImportKind::as_str),
            "outcome": asked.outcome.map(DayOutcome::as_str),
            "order": if asked.newest_first { "desc" } else { "asc" },
            "run": asked.run,
            "stage": asked.stage,
            "reason": asked.reason,
            "why": asked.why,
        },
        "run": asked.run.map(|run| run_json(root, parsed, evidence, run)),
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
        "runs": runs.runs,
        "runs_total": runs.total,
        "runs_page": RUNS_PAGE,
        "runs_next": runs.next,
        "runs_cursor_refused": runs.refused,
        "rows": rows,
        // EVERY DIRECTORY THE IMPORTER'S EVIDENCE WAS READ FROM, each `read`,
        // `absent` or `unreadable`, with what its ring held (D-4862 round 6).
        "evidence": {
            "dirs": dirs,
            "malformed": evidence.dirs().iter().map(crate::logs::DirEvidence::malformed).sum::<u64>(),
            "errors": evidence
                .dirs()
                .iter()
                .flat_map(|dir| dir.errors().iter().cloned())
                .collect::<Vec<_>>(),
            "none_means": "a row's logged_from is \"none\" only when every directory a run of \
                           its kind can write was read and holds nothing for it; a directory \
                           absent or unreadable is named in its place",
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
        begin options 2018-11-06 *\n\
        incomplete options 2018-11-06 * definition=2 files=3 seconds=30 failures=1 late=0 late_unresolved=0 max_back_s=0\n\
        begin options 2019-11-13 *\n\
        done options 2019-11-13 * definition=2 files=4 seconds=40 failures=0 late=0 late_unresolved=0 max_back_s=0\n\
        begin options 2019-11-14 *\n\
        incomplete options 2019-11-14 * definition=2 files=5 seconds=50 failures=1 late=0 late_unresolved=0 max_back_s=0\n\
        begin options 2019-12-27 *\n\
        done options 2019-12-27 * definition=2 files=6 seconds=60 failures=0 late=0 late_unresolved=0 max_back_s=0\n\
        begin options 2020-01-01 *\n\
        incomplete options 2020-01-01 * definition=2 files=7 seconds=70 failures=1 late=2 late_unresolved=0 max_back_s=3\n\
        begin indices 2022-03-04 *\n\
        done indices 2022-03-04 * files=2 seconds=20 failures=0 late=0 late_unresolved=0 max_back_s=0\n\
        begin indices 2022-03-07 *\n\
        refused indices 2022-03-07 * definition=2 stage=vendor reason=price_refused failures=2 run=7\n\
        begin options 2020-09-16 *\n\
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
        // THE CENSUS'S OWN WORDS NAME ITS CODE (WP-33c, DB-A47): a line
        // written before codes existed, whose logged refusal opens
        // `n_valid is`, is the census's full refusal, which the importer codes
        // `too_many_entries`; read from that wording and said so.
        assert_eq!(rows[0]["reason"], "too_many_entries");
        assert_eq!(rows[0]["reason_from"], "census wording");
        assert_eq!(rows[0]["late"], 2);
        assert_eq!(rows[0]["max_back_s"], 3);
        assert_eq!(rows[0]["logged"][0]["about"], "census");
        assert_eq!(rows[0]["logged_from"], "logs/cli ring");
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
        assert_eq!(rows[0]["reason"], Value::Null, "no code is invented");
        assert_eq!(rows[0]["reason_from"], Value::Null);
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

    /// Appends lines to the `cli` ring of `root`.
    fn cli_append(root: &std::path::Path, text: &str) {
        use std::io::Write as _;
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(root.join("logs").join("cli").join("events.ndjson"))
            .expect("the cli log");
        file.write_all(text.as_bytes()).expect("append");
    }

    /// One `cli.lifecycle` line of attempt `run`. Invented.
    fn lifecycle(seq: u32, run: u64, message: &str) -> String {
        format!(
            "{{\"seq\":{seq},\"run\":{run},\"ts\":\"2026-10-06T08:57:38.499Z\",\"ms\":{},\
             \"level\":\"info\",\"target\":\"cli.lifecycle\",\"msg\":\"{message}\",\
             \"fields\":{{\"command\":\"gdfl-import\",\"phase\":\"completed\"}}}}\n",
            1_791_270_000_000_u64 + u64::from(seq)
        )
    }

    /// **A RUN FINDS ITS DAYS, ITS REFUSALS, ITS START AND ITS END**
    /// (WP-33c, DB-A41): the journal's own `run=` on a closing line, and a
    /// `cli` attempt whose importer lines carry no run of their own but are
    /// filed under it by position, the `cli` half having one writer. An
    /// attempt with no `command finished` says so; a run nobody logged finds
    /// nothing; a run that is not a number is refused by name.
    #[test]
    fn a_run_finds_its_days_its_refusals_its_start_and_its_end() {
        let root = store("run");
        cli_append(&root, &lifecycle(20, 4025, "command started"));
        cli_append(
            &root,
            "{\"seq\":21,\"ts\":\"2026-10-06T08:57:38.502Z\",\"ms\":1791270000021,\
             \"level\":\"info\",\"target\":\"gdfl.import\",\"msg\":\"run started\",\
             \"fields\":{\"kind\":\"options\",\"run\":1791270000}}\n",
        );
        cli_append(
            &root,
            "{\"seq\":22,\"ts\":\"2026-10-06T08:57:39.000Z\",\"ms\":1791270000022,\
             \"level\":\"error\",\"target\":\"gdfl.import\",\"msg\":\"refused\",\
             \"fields\":{\"kind\":\"options\",\"day\":\"2020-01-01\",\"about\":\"census\",\
             \"why\":\"n_valid is 9; this build holds at most 8\"}}\n",
        );
        let by_journal = answer(&root, &ask("feed=gdfl&run=7")).expect("reads");
        let days: Vec<&str> = by_journal["rows"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| row["day"].as_str().unwrap())
            .collect();
        assert_eq!(days, ["2022-03-07"], "{by_journal:#}");
        assert_eq!(by_journal["asked"]["run"], 7);

        let by_attempt = answer(&root, &ask("feed=gdfl&run=4025")).expect("reads");
        let rows = by_attempt["rows"].as_array().unwrap();
        assert_eq!(rows.len(), 1, "{by_attempt:#}");
        assert_eq!(rows[0]["day"], "2020-01-01");
        assert_eq!(rows[0]["cli_attempts"], json!([4025]));
        assert_eq!(rows[0]["logged"].as_array().unwrap().len(), 2);
        let attempt = &by_attempt["run"]["attempt"];
        assert_eq!(attempt["id"], 4025);
        assert_eq!(attempt["importer_runs"], json!([1_791_270_000_u64]));
        assert_eq!(attempt["finished_ms"], Value::Null);
        assert!(
            attempt["finished"]
                .as_str()
                .unwrap()
                .contains("no command finished is logged")
        );
        // THE IMPORTER'S OWN RUN ID FINDS THE SAME DAYS, THROUGH ITS ATTEMPT.
        let by_importer = answer(&root, &ask("feed=gdfl&run=1791270000")).expect("reads");
        assert_eq!(by_importer["rows"].as_array().unwrap().len(), 1);
        assert_eq!(by_importer["run"]["attempt"]["id"], 4025);
        // AND ITS END, ONCE LOGGED.
        cli_append(&root, &lifecycle(23, 4025, "command finished"));
        let ended = answer(&root, &ask("feed=gdfl&run=4025")).expect("reads");
        assert_eq!(ended["run"]["attempt"]["phase"], "completed");
        assert!(ended["run"]["attempt"]["finished_ms"].is_u64());
        let none = answer(&root, &ask("feed=gdfl&run=999")).expect("reads");
        assert_eq!(none["rows"], json!([]));
        assert_eq!(none["run"]["attempt"], Value::Null);
        assert!(asked("feed=gdfl&run=04").is_err_and(|why| why.contains("run=")));
        assert!(asked("feed=gdfl&run=0").is_err_and(|why| why.contains("run=")));
    }

    /// **EVERY DAY REFUSED FOR ONE REASON IS ONE QUERY** (WP-33c, DB-A47): by
    /// stage, by reason code (the journal's, or the census's own words for a
    /// full census on a line written before codes), and by the words the
    /// refusal logged; the census's OTHER `n_valid` refusal is no full census
    /// and names no code.
    #[test]
    fn every_day_refused_for_one_reason_is_one_query() {
        let root = store("reasons");
        cli_append(
            &root,
            "{\"seq\":30,\"ts\":\"2026-10-05T03:31:18.967Z\",\"ms\":1791171079000,\
             \"level\":\"error\",\"target\":\"gdfl.import\",\"msg\":\"refused\",\
             \"fields\":{\"kind\":\"options\",\"day\":\"2018-11-06\",\"about\":\"census\",\
             \"why\":\"n_valid is 9; the region holds 8\"}}\n",
        );
        let days = |query: &str| -> Vec<String> {
            let got = answer(&root, &ask(query)).expect("reads");
            got["rows"]
                .as_array()
                .unwrap()
                .iter()
                .map(|row| row["day"].as_str().unwrap().to_owned())
                .collect()
        };
        assert_eq!(
            days("feed=gdfl&order=asc&stage=census"),
            ["2018-11-06", "2019-11-14", "2020-01-01"]
        );
        assert_eq!(
            days("feed=gdfl&order=asc&reason=too_many_entries"),
            ["2019-11-14", "2020-01-01"],
            "the region refusal of 2018-11-06 is not a full census"
        );
        assert_eq!(days("feed=gdfl&reason=price_refused"), ["2022-03-07"]);
        assert_eq!(days("feed=gdfl&why=LTP"), ["2018-11-06"]);
        assert_eq!(
            days("feed=gdfl&order=asc&why=n_valid"),
            ["2018-11-06", "2019-11-14", "2020-01-01"]
        );
        let region = answer(&root, &ask("feed=gdfl&from=2018-11-06&to=2018-11-06")).unwrap();
        assert_eq!(region["rows"][0]["stage"], "census");
        assert_eq!(region["rows"][0]["reason"], Value::Null);
        assert!(asked("feed=gdfl&stage=censuss").is_err_and(|why| why.contains("census")));
        assert!(
            asked(&format!("feed=gdfl&why={}", "x".repeat(MAX_TEXT_ASKED + 1)))
                .is_err_and(|why| why.contains("at most"))
        );
    }

    /// **A PARAMETER THIS ROUTE DOES NOT READ IS REFUSED BY NAME** (WP-33c,
    /// DB-A45): `reason`'s old misspelling and any other name are never
    /// answered as if they had filtered.
    #[test]
    fn a_parameter_this_route_does_not_read_is_refused_by_name() {
        let why = asked("feed=gdfl&bogus=1&reasn=census").expect_err("unknown names");
        assert!(why.contains("bogus") && why.contains("reasn"), "{why}");
        assert!(
            why.contains("refused rather than answered unfiltered"),
            "{why}"
        );
        assert!(asked("feed=gdfl&run=7&stage=census&reason=x&why=y&order=asc").is_ok());
    }

    /// **THE AUDIT SUMMARY IS KEPT WHILE NOTHING MOVES, AND MOVES WITH THE
    /// JOURNAL** (WP-33c, DB-A32): two calls over an unchanged journal and log
    /// answer the same value (the second from what was kept); a closing line
    /// appended changes it; and it is the same answer one walk gives.
    #[test]
    fn the_summary_is_kept_while_nothing_moves_and_moves_with_the_journal() {
        use std::io::Write as _;
        let root = store("summary-kept");
        let mut first = summary(&root);
        let mut again = summary(&root);
        assert_eq!(
            again["journal"]["read_now"], 0,
            "this call read nothing new"
        );
        first["journal"]["read_now"] = Value::Null;
        again["journal"]["read_now"] = Value::Null;
        assert_eq!(first, again);
        let snapshot = crate::server::import_journal::snapshot(&root).unwrap();
        let evidence = crate::logs::import_refusals(&root);
        let mut walked = summary_of(&root, &snapshot, &evidence);
        walked["journal"]["read_now"] = Value::Null;
        assert_eq!(first, walked);
        assert_eq!(first["newest_not_done"]["day"], "2020-09-16");
        let mut journal = std::fs::OpenOptions::new()
            .append(true)
            .open(pull::gdfl_import::journal_path(&root))
            .unwrap();
        journal
            .write_all(
                b"done options 2020-09-16 * definition=2 files=1 seconds=1 failures=0 late=0 late_unresolved=0 max_back_s=0\n",
            )
            .unwrap();
        let moved = summary(&root);
        assert_ne!(first, moved);
        assert_eq!(moved["counts"]["open"], 0);
        assert_eq!(moved["newest_not_done"]["day"], "2022-03-07");
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

    // -----------------------------------------------------------------------
    // IA-11r, R5 (D-4862 round 6): THE IMPORTER'S EVIDENCE IS READ WHERE THE
    // RUN LOGGED IT. One fixture store, one named test per reader (IA-LG-02).
    // -----------------------------------------------------------------------

    /// The indices run of the fixture.
    const INDICES_RUN: u64 = 1_791_700_000;

    /// A journal for [`run_store`]: an indices run whose day 2022-03-07 closed
    /// `incomplete` on a line with no stage, a stocks day and an indices day
    /// done, and an older options day the census refused. Invented.
    const RUN_JOURNAL: &str = "\
        run 1791700000 source=zip started=1791700000\n\
        begin options 2019-11-14 *\n\
        incomplete options 2019-11-14 * definition=2 files=5 seconds=50 failures=1 late=0 late_unresolved=0 max_back_s=0\n\
        begin stocks 2022-03-04 *\n\
        done stocks 2022-03-04 * files=2 seconds=20 failures=0 late=0 late_unresolved=0 max_back_s=0\n\
        begin indices 2022-03-04 *\n\
        done indices 2022-03-04 * files=2 seconds=20 failures=0 late=0 late_unresolved=0 max_back_s=0\n\
        begin indices 2022-03-07 *\n\
        incomplete indices 2022-03-07 * files=2 seconds=20 failures=1 late=0 late_unresolved=0 max_back_s=0\n\
    ";

    /// One store where an INDICES run logged to `logs/gdfl/indices` through a
    /// real sink (its `run claimed`, three `day imported` events, a refusal on
    /// 2022-03-07 and one on 2022-03-08, a day it crashed before closing) and
    /// an OLDER options run logged to `logs/cli` (a census refusal on
    /// 2019-11-14). `logs/gdfl/store` exists and is empty; `logs/gdfl/stocks`
    /// and `logs/gdfl/options` were never created.
    fn run_store(name: &str) -> std::path::PathBuf {
        let root =
            crate::server::import_journal::tests::journal(&format!("imports-{name}"), RUN_JOURNAL);
        let logs = root.join("logs");
        std::fs::create_dir_all(logs.join("cli")).expect("mkdir");
        std::fs::write(
            logs.join("cli").join("events.ndjson"),
            "{\"seq\":1,\"ts\":\"2026-10-05T03:31:18.967Z\",\"ms\":1791171078968,\
             \"level\":\"error\",\"target\":\"gdfl.import\",\"msg\":\"refused\",\
             \"fields\":{\"kind\":\"options\",\"day\":\"2019-11-14\",\"about\":\"census\",\
             \"why\":\"n_valid is 9; this build holds at most 8\"}}\n",
        )
        .expect("the cli log");
        std::fs::create_dir_all(logs.join("gdfl").join("store")).expect("mkdir");
        let own = logs.join("gdfl").join("indices");
        std::fs::create_dir_all(&own).expect("mkdir");
        let sink = telemetry::Sink::open(
            &telemetry::Config::new(&own).with_min_level(telemetry::Level::Trace),
        )
        .expect("the run's own sink");
        let said = |event: telemetry::Event<'_>| {
            assert!(sink.emit(&event).is_written(), "the fixture logs");
        };
        said(
            telemetry::Event::info(crate::logs::RUN_LIFECYCLE_TARGET, "run claimed")
                .with("run", INDICES_RUN)
                .with("kind", "indices")
                .with("mode", "import"),
        );
        for day in ["2022-03-02", "2022-03-03", "2022-03-04"] {
            said(
                telemetry::Event::info("gdfl.import", "day imported")
                    .with("run", INDICES_RUN)
                    .with("kind", "indices")
                    .with("day", day),
            );
        }
        for day in ["2022-03-07", "2022-03-08"] {
            said(
                telemetry::Event::error("gdfl.import", "refused")
                    .with("run", INDICES_RUN)
                    .with("kind", "indices")
                    .with("day", day)
                    .with("about", "ZZ FIXTURE 50.NSE_IDX.csv")
                    .with("stage", "vendor")
                    .with("code", "malformed_row")
                    .with("why", "line 9: the LTP is not a two-place decimal"),
            );
        }
        drop(sink);
        root
    }

    /// The row of `kind` and `day` in an answer.
    fn row_of<'a>(got: &'a Value, kind: &str, day: &str) -> &'a Value {
        got["rows"]
            .as_array()
            .expect("rows")
            .iter()
            .find(|row| row["kind"] == kind && row["day"] == day)
            .unwrap_or_else(|| panic!("{kind} {day} in {got:#}"))
    }

    /// **`/imports.json` READS THE IMPORTER'S EVIDENCE WHERE THE RUN LOGGED
    /// IT** (IA-LG-02; F78): the indices run's refusal is on its day's row
    /// with its directory named as the source, never `"none"`; the day it
    /// crashed before closing is a logged-only row; the indices pace is
    /// measured from that directory and says so; `run=` answers the run's
    /// lifecycle from its own audit stream; the older options run is still
    /// read from `logs/cli`; a row whose kind's own directory was never
    /// created names it `absent` and does not say `"none"`, while a row all
    /// of whose directories were read does. Every assertion on the indices
    /// run fails when the reader reads `logs/cli` alone.
    #[test]
    fn imports_json_reads_importer_evidence_where_the_run_logged() {
        let root = run_store("where-the-run-logged");
        let got = answer(
            &root,
            &ask("feed=gdfl&order=asc&from=2019-11-01&to=2022-03-31"),
        )
        .expect("reads");

        // (1) THE REFUSAL, FROM THE RUN'S OWN RING, source named.
        let refused = row_of(&got, "indices", "2022-03-07");
        assert_eq!(
            refused["logged_from"], "logs/gdfl/indices ring",
            "{refused:#}"
        );
        assert_eq!(refused["logged"].as_array().unwrap().len(), 1);
        assert_eq!(refused["logged"][0]["about"], "ZZ FIXTURE 50.NSE_IDX.csv");
        assert_eq!(refused["logged"][0]["code"], "malformed_row");
        assert_eq!(refused["stage"], "vendor", "settled from the log");
        assert_eq!(refused["stage_from"], "log");

        // (2) THE DAY IT CRASHED BEFORE CLOSING is a row, from the log alone.
        let crashed = row_of(&got, "indices", "2022-03-08");
        assert_eq!(crashed["outcome_from"], "log", "{crashed:#}");
        assert_eq!(crashed["logged_from"], "logs/gdfl/indices ring");

        // (3) THE OLDER RUN is still read from `logs/cli`.
        let older = row_of(&got, "options", "2019-11-14");
        assert_eq!(older["logged_from"], "logs/cli ring", "{older:#}");
        assert_eq!(older["stage"], "census");

        // (4) "NONE" ONLY WHEN EVERY DIRECTORY OF THE KIND WAS READ. Indices:
        // `logs/cli`, `logs/gdfl/indices` and `logs/gdfl/store` all exist.
        let done = row_of(&got, "indices", "2022-03-04");
        assert_eq!(done["logged_from"], NONE_LOGGED, "{done:#}");
        // Stocks: `logs/gdfl/stocks` was never created, and is named.
        let stocks = row_of(&got, "stocks", "2022-03-04");
        let source = stocks["logged_from"].as_str().expect("a source");
        assert_ne!(source, NONE_LOGGED, "{stocks:#}");
        assert!(source.contains("logs/gdfl/stocks absent"), "{source}");
        assert!(
            !source.contains("logs/gdfl/indices"),
            "not its directory: {source}"
        );

        // (5) THE PACE, from the directory the run logged to, which it names.
        let kinds = got["kinds"].as_array().expect("kinds");
        let pace_of = |kind: &str| -> &Value {
            &kinds.iter().find(|k| k["kind"] == kind).expect("the kind")["pace"]
        };
        let pace = pace_of("indices");
        assert_eq!(pace["gaps"], 2, "{pace:#}");
        assert_eq!(pace["from_dirs"], json!(["logs/gdfl/indices"]));
        assert!(
            pace["from"].as_str().unwrap().contains("logs/gdfl/indices"),
            "{pace:#}"
        );
        assert_eq!(pace_of("options"), &Value::Null, "one event is no pace");

        // (6) EVERY DIRECTORY READ IS NAMED, with its state.
        let dirs = got["evidence"]["dirs"].as_array().expect("dirs");
        let states: Vec<(&str, &str)> = dirs
            .iter()
            .map(|dir| (dir["dir"].as_str().unwrap(), dir["state"].as_str().unwrap()))
            .collect();
        assert_eq!(
            states,
            [
                ("logs/cli", "read"),
                ("logs/gdfl/indices", "read"),
                ("logs/gdfl/stocks", "absent"),
                ("logs/gdfl/options", "absent"),
                ("logs/gdfl/store", "read"),
            ]
        );
        assert!(dirs[1]["ring_bytes"].as_u64().unwrap() > 0, "{dirs:#?}");
        assert_eq!(dirs[2]["ring_files"], 0);

        // (7) `run=`: the run's lifecycle, keyed, from its own audit stream.
        let by_run = answer(&root, &ask(&format!("feed=gdfl&run={INDICES_RUN}"))).expect("reads");
        let lifecycle = &by_run["run"]["lifecycle"];
        assert_eq!(
            lifecycle["from"],
            json!(["logs/gdfl/indices"]),
            "{by_run:#}"
        );
        assert_eq!(lifecycle["events_total"], 1);
        assert_eq!(lifecycle["events"][0]["message"], "run claimed");
        assert_eq!(lifecycle["events"][0]["fields"]["kind"], "indices");
        assert_eq!(lifecycle["unread"], json!([]));
        assert_eq!(lifecycle["capped"], false);
        assert_eq!(by_run["run"]["journal_lines"].as_array().unwrap().len(), 1);
        // A run nobody logged has no lifecycle, and says where it looked.
        let nobody = answer(&root, &ask("feed=gdfl&run=99")).expect("reads");
        assert_eq!(nobody["run"]["lifecycle"]["events_total"], 0);
        assert_eq!(nobody["run"]["attempt"], Value::Null);

        // (8) A RING ROLLED AWAY: the audit stream of the run's own directory
        // answers, and is named as the source.
        std::fs::remove_file(root.join("logs/gdfl/indices/events.ndjson")).expect("the ring");
        let later = answer(&root, &ask("feed=gdfl&from=2022-03-07&to=2022-03-07")).expect("reads");
        let audited = row_of(&later, "indices", "2022-03-07");
        assert_eq!(
            audited["logged_from"], "logs/gdfl/indices audit stream",
            "{audited:#}"
        );
        assert_eq!(audited["logged"][0]["about"], "ZZ FIXTURE 50.NSE_IDX.csv");
        let _ignored = std::fs::remove_dir_all(&root);
    }

    /// **THE `/db` DAY BAND READS THE IMPORTER'S EVIDENCE WHERE THE RUN
    /// LOGGED IT** (IA-LG-02): `/dayflags.json` takes its import rows through
    /// [`answer`], so the refusal an indices run logged to
    /// `logs/gdfl/indices` is on its day, and the day it crashed before
    /// closing reads from the log. Fails when the reader reads `logs/cli`
    /// alone: both days would carry no refusal.
    #[test]
    fn dayflags_day_band_reads_importer_evidence_where_the_run_logged() {
        let root = run_store("day-band");
        let asked = crate::server::dayflags::asked(
            "feed=gdfl&instrument=NSE-INDEX-NIFTY&from=2022-03&to=2022-03",
        )
        .expect("a day band request");
        let got = crate::server::dayflags::answer(&root, &asked);
        let on = |day: &str| -> &Value {
            got["days"]
                .as_array()
                .expect("days")
                .iter()
                .find(|record| record["day"] == day)
                .unwrap_or_else(|| panic!("{day} in {got:#}"))
        };
        let refused = on("2022-03-07");
        assert_eq!(refused["import"]["outcome"], "incomplete", "{refused:#}");
        let refusals = &refused["import"]["refusals"];
        assert_eq!(
            refusals["this_count"].as_u64().unwrap_or(0)
                + refusals["other_count"].as_u64().unwrap_or(0),
            1,
            "the run's refusal is on its day: {refused:#}"
        );
        let crashed = on("2022-03-08");
        assert_eq!(crashed["import"]["from"], "log", "{crashed:#}");
        let _ignored = std::fs::remove_dir_all(&root);
    }

    /// **`/store`'S BEHIND READS THE IMPORTER'S EVIDENCE WHERE THE RUN LOGGED
    /// IT** (IA-LG-02): a day closed `incomplete` on a line with no stage is
    /// behind when the importer's own `refused` event names the census, and
    /// that event is in the run's directory. Here an options run logged its
    /// census refusal to `logs/gdfl/options` and nothing to `logs/cli`: the
    /// day counts. Reading `logs/cli` alone counts 0. A second read with
    /// nothing changed is the kept answer, and a new line in the run's
    /// directory moves it.
    #[test]
    fn store_behind_reads_importer_evidence_where_the_run_logged() {
        let root = crate::server::import_journal::tests::journal(
            "imports-store-behind",
            "begin options 2019-11-14 *\n\
             incomplete options 2019-11-14 * definition=2 files=5 seconds=50 failures=1 late=0 \
             late_unresolved=0 max_back_s=0\n\
             begin options 2019-11-15 *\n\
             incomplete options 2019-11-15 * definition=2 files=5 seconds=50 failures=1 late=0 \
             late_unresolved=0 max_back_s=0\n",
        );
        let own = root.join("logs").join("gdfl").join("options");
        std::fs::create_dir_all(&own).expect("mkdir");
        std::fs::create_dir_all(root.join("logs").join("cli")).expect("mkdir");
        let census = |seq: u32, day: &str| {
            format!(
                "{{\"seq\":{seq},\"ts\":\"2026-10-05T03:31:18.967Z\",\"ms\":{},\
                 \"level\":\"error\",\"target\":\"gdfl.import\",\"msg\":\"refused\",\
                 \"fields\":{{\"kind\":\"options\",\"day\":\"{day}\",\"about\":\"census\",\
                 \"why\":\"n_valid is 9; this build holds at most 8\"}}}}\n",
                1_791_171_078_967_u64 + u64::from(seq)
            )
        };
        std::fs::write(own.join("events.ndjson"), census(1, "2019-11-14")).expect("the run's log");
        let dir = agreeing("imports-store-behind");
        let mut built = site("imports-store-behind", &dir);
        built.store_root = root.clone();
        let feed = brutex_core::vendor::Vendor::Gdfl;
        let behind = crate::server::census_behind(&built, feed);
        assert_eq!(behind.days, 1, "{behind:?}");
        assert_eq!(
            behind.from,
            crate::server::import_journal::parse_day("2019-11-14")
        );
        assert_eq!(crate::server::census_behind(&built, feed).days, 1, "kept");
        // THE SAME EVIDENCE AGAIN IS THE SAME `Arc`: the cache key stands for
        // the tuple of the per-directory reads.
        let first = crate::logs::import_refusals(&root);
        let again = crate::logs::import_refusals(&root);
        assert!(std::sync::Arc::ptr_eq(&first, &again));
        // A NEW LINE IN THE RUN'S OWN DIRECTORY moves the answer.
        {
            use std::io::Write as _;
            let mut file = std::fs::OpenOptions::new()
                .append(true)
                .open(own.join("events.ndjson"))
                .expect("the run's log");
            file.write_all(census(2, "2019-11-15").as_bytes())
                .expect("append");
        }
        let moved = crate::logs::import_refusals(&root);
        assert!(!std::sync::Arc::ptr_eq(&first, &moved), "a directory moved");
        assert_eq!(crate::server::census_behind(&built, feed).days, 2);
        let _ignored = std::fs::remove_dir_all(&root);
    }

    // -----------------------------------------------------------------------
    // IA-11r, R6 (D-4813 round 6): THE RUNS PAGE DECODES ONLY ITS RUNS
    // -----------------------------------------------------------------------

    /// `run` lines decoded on this thread since the last call.
    fn decoded() -> usize {
        RUN_LINES_DECODED.with(|count| count.replace(0))
    }

    /// **THE RUNS PAGE READS ONLY ITS RUNS** (IA-LG-03; F79): over a journal
    /// of 100,000 `run` lines the newest page decodes 20 lines, each older
    /// page 20 more with no line skipped or shown twice across an id that has
    /// three lines, and `run=` decodes exactly the id's own lines. A cursor
    /// that names nothing shows nothing and says so; one that is not a cursor
    /// is refused; the route's answer carries the page and the count. A
    /// caller inside the crate that asks for every run still gets every run.
    #[test]
    fn the_runs_page_reads_only_its_runs() {
        use std::fmt::Write as _;
        const RUNS: u64 = 100_000;
        // Ids 1..=100,000 in file order, and id 77,777 twice more among the
        // newest lines, so one id's lines straddle a page end.
        let mut text = String::with_capacity(4 << 20);
        for id in 1..=RUNS {
            let _ = writeln!(text, "run {id} source=zip started={id}");
            if id == RUNS - 19 || id == RUNS - 10 {
                text.push_str("run 77777 source=zip started=77777 resumed=1\n");
            }
        }
        let root = crate::server::import_journal::tests::journal("imports-runs-page", &text);
        let snapshot = crate::server::import_journal::snapshot(&root).expect("reads");
        let parsed: &Parsed = &snapshot.parsed;
        let total = usize::try_from(RUNS).unwrap() + 2;
        assert_eq!(parsed.runs.len(), total);

        let _reset = decoded();
        let ids = |page: &RunsPage| -> Vec<u64> {
            page.runs
                .iter()
                .map(|run| run["id"].as_u64().unwrap())
                .collect()
        };
        // THE NEWEST PAGE: 20 lines decoded, of 100,002.
        let newest = runs_page(parsed, RunsAsked::Page(None));
        assert_eq!(decoded(), RUNS_PAGE, "only the page is decoded");
        assert_eq!(newest.total, total);
        assert_eq!(newest.runs.len(), RUNS_PAGE);
        assert_eq!(newest.refused, None);
        // In file order the tail is: ..., 99,981, 77,777, 99,982 ... 99,990,
        // 77,777, 99,991 ... 100,000. Newest first, 20 lines end at 99,982.
        let mut expected: Vec<u64> = (RUNS - 9..=RUNS).rev().collect();
        expected.push(77_777);
        expected.extend((RUNS - 18..=RUNS - 10).rev());
        assert_eq!(ids(&newest), expected);
        assert_eq!(newest.next.as_deref(), Some("99982"));

        // THE NEXT PAGE starts at the line below, which is 77,777's SECOND
        // line: shown once, then 99,981 down.
        let cursor = runs_cursor(newest.next.as_deref().unwrap()).expect("a cursor");
        let second = runs_page(parsed, RunsAsked::Page(cursor));
        assert_eq!(decoded(), RUNS_PAGE);
        let mut expected: Vec<u64> = vec![77_777];
        expected.extend((RUNS - 37..=RUNS - 19).rev());
        assert_eq!(ids(&second), expected);

        // A PAGE THAT ENDS INSIDE AN ID WITH SEVERAL LINES names the line:
        // one page that ends at 77,777's second line (its `.1`).
        let ends_inside = runs_page(parsed, RunsAsked::Page(Some((RUNS, 0))));
        assert_eq!(
            ids(&ends_inside).last(),
            Some(&77_777),
            "{:?}",
            ids(&ends_inside)
        );
        assert_eq!(ends_inside.next.as_deref(), Some("77777.1"));
        let _counted = decoded();
        let after = runs_page(
            parsed,
            RunsAsked::Page(runs_cursor("77777.1").expect("a cursor")),
        );
        assert_eq!(ids(&after).first(), Some(&(RUNS - 19)), "nothing skipped");
        assert!(!ids(&after).contains(&77_777), "and none shown twice");
        // `runs_before=77777` alone is older than its OLDEST line.
        let oldest = runs_page(parsed, RunsAsked::Page(Some((77_777, 0))));
        assert_eq!(ids(&oldest).first(), Some(&77_776));
        let _counted = decoded();

        // THE OLDEST PAGE has no next.
        let last = runs_page(parsed, RunsAsked::Page(Some((5, 0))));
        assert_eq!(ids(&last), [4, 3, 2, 1]);
        assert_eq!(last.next, None);
        let _counted = decoded();

        // A CURSOR THAT NAMES NOTHING shows nothing, and says so.
        let nothing = runs_page(parsed, RunsAsked::Page(Some((999_999_999, 0))));
        assert!(nothing.runs.is_empty());
        assert!(
            nothing.refused.as_deref().unwrap().contains("999999999"),
            "{:?}",
            nothing.refused
        );
        assert_eq!(decoded(), 0, "and decodes nothing");
        for garbage in ["abc", "0", "5.x", "-1", "5.", "05", " 5"] {
            assert!(runs_cursor(garbage).is_err(), "{garbage:?}");
        }
        assert_eq!(runs_cursor(""), Ok(None));
        assert!(asked("feed=gdfl&runs_before=abc").is_err());

        // `run=`: the id's own lines, through the id index, and no other.
        let evidence = crate::logs::import_refusals(&root);
        let one = run_json(&root, parsed, &evidence, 77_777);
        assert_eq!(decoded(), 3, "the id's three lines");
        assert_eq!(one["journal_lines"].as_array().unwrap().len(), 3);
        let none = run_json(&root, parsed, &evidence, 999_999_999);
        assert_eq!(decoded(), 0);
        assert_eq!(none["journal_lines"], json!([]));

        // THE ROUTE'S ANSWER carries one page; the summary too.
        let route = answer(&root, &ask("feed=gdfl")).expect("reads");
        assert_eq!(decoded(), RUNS_PAGE);
        assert_eq!(route["runs"].as_array().unwrap().len(), RUNS_PAGE);
        assert_eq!(route["runs_total"], total);
        assert_eq!(route["runs_next"], "99982");
        let paged = answer(&root, &ask("feed=gdfl&runs_before=99982")).expect("reads");
        assert_eq!(paged["runs"][0]["id"], 77_777);
        assert_eq!(paged["runs_cursor_refused"], Value::Null);
        let _counted = decoded();
        let brief = summary_of(&root, &snapshot, &evidence);
        assert_eq!(decoded(), RUNS_PAGE);
        assert_eq!(brief["runs_total"], total);

        // A CALLER THAT RESOLVES A RUN BY TIME (the day band) asks for every
        // run and gets every run.
        let every = runs_page(parsed, RunsAsked::Every);
        assert_eq!(every.runs.len(), total);
        assert_eq!(every.next, None);
        let _ignored = std::fs::remove_dir_all(&root);
    }
}

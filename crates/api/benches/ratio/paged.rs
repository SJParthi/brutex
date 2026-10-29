//! A whole month read the way a client must read it: page by page through
//! `/bars/window.json` (BASE-BENCH-1028, D-7400).
//!
//! # Why the bench reads it this way
//!
//! Gate 8's `reads` rows asked `/bars.json` for the whole one-second month at
//! 1,024, 10,240 and 102,400 bars. Since R3-08 (D-4973's route, RA-060) one
//! `/bars.json` answer serves at most [`MONTH_ANSWER_CAP_ROWS`] rows and
//! refuses a larger window `400 too_many_rows`, naming this route. So the
//! 100x case measured nothing and the bench exited 1 (found by the quiet-Mac
//! run of 28 Oct 2026). The cap is right and is kept: a whole one-second
//! month of real data is 36 to 71 MB of JSON. What a client that wants every
//! row does instead is page, and that is what is timed now: [`read_month`]
//! asks page 0, learns the month's `total` from it, and asks each next page
//! until it has every row, [`api::bars::MAX_WINDOW_LIMIT`] rows a page, in
//! time order.
//!
//! # What it refuses, so a wrong answer is never timed as a read
//!
//! A refusal or a fault on any page; a page with no `total`; a `total` that
//! changes between pages (the month changed under the read); a missing month
//! or a faulted record the route reports; a page holding more rows than asked
//! or than the month holds; a short page before the month's end (a pager
//! stepping by what it asked would skip rows); a row that does not follow the
//! one before it (a row repeated or out of order across a page edge); and a
//! query that brings its own paging keys. Each is named in the sentence
//! returned, and the bench stops on it.
//!
//! # Its own cost
//!
//! One request per [`api::bars::MAX_WINDOW_LIMIT`] rows, so a read is
//! `ceil(rows / 1,000)` requests, at least one; the client's own work is one
//! pass over each page's bytes. Both are O(rows of the month) by design, the
//! read being the month: the bench gates its cost PER ROW READ, at 1x, 10x
//! and 100x (`docs/04-invariants.md`, `docs/06-limits.md`). Every loop is
//! bounded: each page either ends the read, refuses, or advances the offset
//! by exactly one full page toward a `total` that may not change.
//!
//! Mounted by the bench (`benches/ratio.rs`) and by
//! `crates/api/tests/bench_paged_month.rs`, which proves it against the
//! production router; it names nothing from the bench, so both can mount it.

/// The most rows one `/bars.json` answer serves (R3-08):
/// `MAX_MONTH_ANSWER_ROWS` in `crates/api/src/bars_route.rs`, private to the
/// crate. Mirrored, never trusted: the bench's refusal case and the test
/// check the route's own refusal sentence names exactly this number
/// ([`cap_sentence`]), so a cap moved on either side stops both.
pub(crate) const MONTH_ANSWER_CAP_ROWS: usize = 60_000;

/// One answer to one page: its status and its body.
pub(crate) struct Page {
    pub(crate) status: u16,
    pub(crate) body: String,
}

/// What reading the month took: its rows, the requests asked, and the first
/// and last stamps read (epoch seconds), so a caller can check the ends.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct MonthRead {
    pub(crate) rows: usize,
    pub(crate) pages: usize,
    pub(crate) first_t: Option<i64>,
    pub(crate) last_t: Option<i64>,
}

/// The keys [`read_month`] sets itself. A query carrying one is refused:
/// a second `dir=` or `offset=` would make the route answer a different
/// question than the pager thinks it asked.
const PAGING_KEYS: [&str; 4] = ["dir", "limit", "offset", "sort"];

/// The words `/bars.json`'s refusal names its cap in.
pub(crate) fn cap_sentence(cap: usize) -> String {
    format!("past the {cap} one /bars.json answer serves")
}

/// The request for the page at `offset` of `query`, oldest first.
fn page_path(query: &str, offset: usize) -> String {
    format!(
        "/bars/window.json?{query}&dir=asc&limit={}&offset={offset}",
        api::bars::MAX_WINDOW_LIMIT
    )
}

/// The first 160 characters of `body`, for a refusal's sentence.
fn excerpt(body: &str) -> String {
    body.chars().take(160).collect()
}

/// The whole number right after the first `key` in `body`.
fn number_after(body: &str, key: &str) -> Result<usize, String> {
    let at = body
        .find(key)
        .ok_or_else(|| format!("no {key} in the answer: {}", excerpt(body)))?;
    let rest = body.get(at + key.len()..).unwrap_or("");
    let end = rest
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(rest.len());
    rest.get(..end)
        .unwrap_or("")
        .parse()
        .map_err(|_| format!("{key} is not a whole number: {}", excerpt(rest)))
}

/// Every row's stamp on one page, in the order sent.
fn stamps(body: &str) -> Result<Vec<i64>, String> {
    const ROW: &str = "{\"t\":";
    body.match_indices(ROW)
        .map(|(at, _)| {
            let rest = body.get(at + ROW.len()..).unwrap_or("");
            let end = rest.find([',', '}']).unwrap_or(rest.len());
            rest.get(..end)
                .unwrap_or("")
                .parse::<i64>()
                .map_err(|_| format!("a stamp that is not a number: {}", excerpt(rest)))
        })
        .collect()
}

/// Reads every row of `query`'s window, page by page, through `ask`.
///
/// `query` names the series and the months (`feed`, the series keys,
/// `timeframe`, `from`, `to`); the pager adds the order, the page size and
/// the offset. Returns what the read took, or the sentence naming the first
/// thing wrong.
pub(crate) fn read_month(
    query: &str,
    mut ask: impl FnMut(&str) -> Page,
) -> Result<MonthRead, String> {
    if let Some(key) = query.split('&').find_map(|pair| {
        let name = pair.split('=').next().unwrap_or("");
        PAGING_KEYS.into_iter().find(|&key| key == name)
    }) {
        return Err(format!(
            "the query brings its own {key}=, which the pager sets: {query}"
        ));
    }
    let limit = api::bars::MAX_WINDOW_LIMIT;
    let mut read = MonthRead {
        rows: 0,
        pages: 0,
        first_t: None,
        last_t: None,
    };
    let mut total = None;
    loop {
        let path = page_path(query, read.rows);
        let page = ask(&path);
        read.pages += 1;
        if page.status != 200 {
            return Err(format!(
                "{path} answered {}: {}",
                page.status,
                excerpt(&page.body)
            ));
        }
        let held = number_after(&page.body, "\"total\":")?;
        if let Some(before) = total.filter(|&before| before != held) {
            return Err(format!(
                "the month changed under the read: {before} rows, then {held} at offset {}",
                read.rows
            ));
        }
        total = Some(held);
        for key in ["\"months_missing\":", "\"faulted_records\":"] {
            let count = number_after(&page.body, key)?;
            if count != 0 {
                return Err(format!("{path} reports {key}{count}"));
            }
        }
        let on_page = stamps(&page.body)?;
        if on_page.len() > limit {
            return Err(format!(
                "{path} sent {} rows, more rows than the {limit} asked",
                on_page.len()
            ));
        }
        for &t in &on_page {
            if let Some(before) = read.last_t.filter(|&before| t <= before) {
                return Err(format!(
                    "{path}: row t={t} after t={before}, a row repeated or out of order"
                ));
            }
            let _first = read.first_t.get_or_insert(t);
            read.last_t = Some(t);
        }
        read.rows += on_page.len();
        if read.rows > held {
            return Err(format!(
                "{path} reads {} rows, past the {held} rows the month holds",
                read.rows
            ));
        }
        if read.rows == held {
            return Ok(read);
        }
        if on_page.len() < limit {
            return Err(format!(
                "{path}: a short page of {} rows at {} of {held}, which a pager would step past",
                on_page.len(),
                read.rows
            ));
        }
    }
}

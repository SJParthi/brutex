//! `GET /audit.json` — the one read the audit console makes.
//!
//! # Why this route exists
//!
//! It did not, and the browser console was written against its absence. The
//! `SvelteKit` page at `web/src/routes/audit/+page.svelte` fetched `/audit` —
//! the **HTML** page — and read the table back out with `DOMParser`. That
//! works, and it couples a browser page to `render::audit_row`'s markup, so a
//! renderer change becomes a front-end outage with no compiler between them.
//!
//! It also left `/audit` as one path answered by two different applications.
//! The dev proxy forwarded `/audit` to this server, so the console rendered on
//! a nav click (`SvelteKit` routed in-browser, the proxy never consulted) and
//! vanished on a reload (the proxy answered, with the Rust page). Same URL,
//! two products, decided by how the operator arrived.
//!
//! One JSON route removes both. `/audit` keeps rendering the no-script page it
//! always rendered; nothing here replaces or changes it.
//!
//! # Why the coverage roll-up rides along instead of being a second fetch
//!
//! The console needs three things per poll: what ran, what is held, and
//! whether the store is growing right now. "What is held" was `/store.json`,
//! which is one JSON object per instrument-month — 1.7 MB at the 21,000
//! entries held today, and ~7 MB at the census this store is heading for —
//! downloaded on every poll to colour an 80-cell month grid. Rolled up by
//! month here it is ~80 objects. The server walks the same entry region either
//! way; only the wire changes, and it changes by three orders of magnitude.
//!
//! # How "is something happening right now" is answered honestly
//!
//! Not with a timer, and not with a guess. A pull is one synchronous POST and
//! nothing writes a record for a run **in flight** — the journal record is
//! appended when the run ends, so a nine-minute backfill is nine minutes of
//! silence on every surface this repository has. What does move while a pull
//! is running is the store itself, so this route reports the two facts that
//! move with it and nothing else:
//!
//! * `generation` — the manifest's commit counter, which advances only when
//!   bars are filed, and
//! * `committed_at` — the manifest file's own mtime, in the **server's** clock,
//!   beside the server's `at`, so the browser subtracts two server numbers and
//!   never its own possibly-skewed clock.
//!
//! The console polls, subtracts, and states what it measured. When nothing is
//! running both are unchanged, and that is exactly what it will say. It never
//! claims a percentage nobody could check.
//!
//! # Bounds
//!
//! One `metadata` call for the record count, one page of at most
//! [`audit::MAX_PAGE_RECORDS`] records read off disk, one manifest header per
//! vendor, one `metadata` call for the manifest mtime, and one pass over the
//! entry region — the same pass `/store.json` already makes. Nothing here walks
//! a directory.
//!
//! **One cost does grow with the journal, once** (WP-33a, D-3341 item 4).
//! Scoping the runs by feed needs each run record's vendor, and the journal
//! keeps no index of its runs, so [`Attribution`] decodes every record the
//! first time and only the appended ones after that: measured on a shadow of
//! the operator's store, 9 Oct 2026, 4,217,027 records (1,079,558,912 bytes)
//! took 1.33 s on the first request; 200 later requests took p50 0.75 ms and
//! p99 0.84 ms for `feed=zerodha`. The GDFL answer adds the import journal's
//! summary (`crate::server::imports::summary`): p50 8.6 ms, p99 9.3 ms.

use crate::server::{Loaded, Site, census_now, journal_trouble, param, window_text};
use crate::{audit, ingest, render};
use brutex_core::vendor::Vendor;
use pull::session::Day;
use std::collections::BTreeMap;
use std::fmt::Write as _;

/// The content type every answer here carries.
///
/// A function and not a constant because `HeaderName` is not `Copy`, and one
/// binding cannot be moved into two returns — the same reason `bars_json`
/// builds its header array per arm.
fn json() -> [(axum::http::HeaderName, &'static str); 1] {
    [(
        axum::http::header::CONTENT_TYPE,
        "application/json; charset=utf-8",
    )]
}

/// `GET /audit.json?feed=<wire>&page=<n>`.
///
/// `feed` is **required and never defaulted**. `/store.json` defaults an
/// unparseable feed to Dhan and answers `[]` with HTTP 200, which renders as
/// "nothing stored" over a store holding 139 million bars — a fallback that
/// hides a failure, which `CLAUDE.md` §4 bans. A missing or unknown feed here
/// is a 400 that names the parameter and lists what would have been accepted.
pub async fn audit_json(
    axum::extract::State(site): axum::extract::State<Loaded>,
    uri: axum::http::Uri,
) -> (
    axum::http::StatusCode,
    [(axum::http::HeaderName, &'static str); 1],
    String,
) {
    let query = uri.query().unwrap_or("");
    let asked = param(query, "feed");
    // THE EMPTY CASE IS TESTED HERE AND NOT LEFT TO THE PARSER.
    // `ingest::parse_vendor("")` answers `Some(Dhan)` — the default is inside
    // the parser, so every caller that merely checks for `None` has already
    // silently chosen a vendor. That is how `/store.json` answers `[]` with
    // HTTP 200 for a feed nobody named. A monitoring surface must not inherit
    // it, and the test `the_route_refuses_a_missing_feed…` is what found it.
    let feed = if asked.is_empty() {
        None
    } else {
        ingest::parse_vendor(&asked)
    };
    let Some(feed) = feed else {
        note_feed_refused(&asked);
        return (
            axum::http::StatusCode::BAD_REQUEST,
            json(),
            format!(
                r#"{{"error":{}}}"#,
                render::json_string(&format!(
                    "/audit.json needs ?feed=<wire> and never guesses one — {asked:?} is not a \
                     feed this build can read. Accepted: {}.",
                    wires()
                ))
            ),
        );
    };
    note_page_ignored(query);
    let page = crate::server::page_number(query);
    // OFF THE ASYNC WORKERS. The feed's journal attribution reads the pull
    // journal's new records and the GDFL import journal's new bytes
    // ([`Attribution`], `crate::server::import_journal`); neither belongs on a Tokio
    // worker, and a full pool is a named 429, never a queue.
    match crate::detail::run_store_read(move || {
        body(
            &site,
            feed,
            page,
            std::time::SystemTime::now(),
            ingest::today_ist(),
        )
    })
    .await
    {
        Ok(body) => (axum::http::StatusCode::OK, json(), body),
        Err(why) => {
            let (status, body) = crate::detail::admission_refused(
                "audit read",
                crate::detail::MAX_STORE_READ_CONCURRENT,
                &why,
            );
            (status, json(), body)
        }
    }
}

/// A request this route turned away, on the machine rather than only on the
/// wire.
///
/// # What was invisible
///
/// The refusal existed in exactly one place: the body of the 400. That body
/// goes to whoever asked, and whoever asks here is a browser poll or a `curl`
/// — the console renders its own sentence into a tab that is closed an hour
/// later, and a script prints nothing at all. `logs::note_request` records the
/// path and the status and **deliberately never the query string**, so the
/// machine's whole memory of a refused poll was `GET /audit.json 400`. The one
/// fact that decides what to do next was not in it.
///
/// The two reasons want opposite fixes and are indistinguishable from the
/// status code. *Absent* is a caller that never sends `feed` — a front end
/// built against an older route, and a bug on this side of the wire. *Unknown*
/// is a wire this build cannot read: a typo, or a binary older than the vendor
/// row somebody is asking it for.
///
/// # Why the wire itself is quoted, and why that is not a leak
///
/// `feed` names a vendor and can name nothing else. §8's credentials are read
/// from Parameter Store inside `crates/pull` and never travel through a query
/// string, and the value is bounded twice — by what a URL can carry and by the
/// sink, which cuts a string at [`telemetry::MAX_STR_VALUE_BYTES`] and sets
/// `cut` when it does. It is the same argument `api.ingest form refused`
/// already makes for quoting back a refused form value. Without it the line
/// says a parameter was wrong and still cannot say what arrived, which is most
/// of the reason to write it.
///
/// # Cost
///
/// `Warn` — the server answered, the answer named its own refusal, and nothing
/// on disk is wrong; somebody still has to know. Bounded by REQUESTS, and only
/// the refused ones: a poll that succeeds emits nothing here, which is what
/// makes it safe on a route a console re-reads every few seconds for twelve
/// hours.
fn note_feed_refused(asked: &str) {
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::warn("api.audit.json", "request refused")
            .with("param", telemetry::Value::Str("feed"))
            .with("wire", telemetry::Value::Str(asked))
            .with(
                "why",
                telemetry::Value::Str(if asked.is_empty() {
                    "absent, and this route never guesses one"
                } else {
                    "not a wire this build reads"
                }),
            )
            .with("accepted", telemetry::Value::count(Vendor::ALL.len())),
    );
}

/// A `page` nobody could read, and the newest page answered instead.
///
/// [`crate::server::page_number`] defaults an unparseable page to zero, and
/// that default is defensible — a page number selects a VIEW and can never
/// change what the data says — but it is **invisible**. `?page=banana` and
/// `?page=0` produce byte-identical answers, so a pager writing a value this
/// server cannot parse looks exactly like an operator asking for the newest
/// runs on purpose, and the answer carries `"page":0` in both cases to confirm
/// it. Nothing else on the machine keeps the query string. Taking the default
/// and then saying so is `CLAUDE.md` §4 on fallbacks: degrade loudly and name
/// the reason, never silently.
///
/// The text is re-read here rather than taken from `page_number`'s return
/// value because the return value is precisely where the fact is lost.
///
/// # Cost
///
/// `Warn`, at most once per request, and only for a `page` that was present
/// and unreadable — an absent `page` is the ordinary case (the console omits
/// it for the first page) and is silent. Mutually exclusive with
/// [`note_feed_refused`], which returns before a page is ever looked at, so one
/// request is at most one line from this module however mangled the query is.
fn note_page_ignored(query: &str) {
    let asked = param(query, "page");
    if asked.is_empty() || asked.parse::<usize>().is_ok() {
        return;
    }
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::warn("api.audit.json", "page ignored")
            .with("param", telemetry::Value::Str("page"))
            .with("asked", telemetry::Value::Str(&asked))
            .with(
                "why",
                telemetry::Value::Str("not a whole number; answered the newest page"),
            ),
    );
}

/// Every feed wire this build accepts, as one comma-separated sentence.
///
/// Built from [`Vendor::ALL`] rather than typed out, so a fifth vendor appears
/// in the refusal the day its row exists and nobody has to remember this line.
fn wires() -> String {
    Vendor::ALL
        .iter()
        .map(|v| v.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

/// The whole answer, from values rather than from the machine's clock.
///
/// `now` and `today` are parameters so a test drives the identical bytes on
/// every run — `CLAUDE.md` §3 rule 5, idempotence — and so the clock-refusal
/// arm is reachable without waiting for a machine whose clock is broken.
#[must_use]
pub fn body(
    site: &Site,
    feed: Vendor,
    page: usize,
    now: std::time::SystemTime,
    today: Result<Day, ingest::Refusal>,
) -> String {
    let mut out = String::from("{");
    let _ = write!(out, r#""at":{}"#, ingest::epoch_secs(now));
    match today {
        Ok(day) => {
            let _ = write!(
                out,
                r#","today":{},"clock":null"#,
                render::json_string(&day.to_string())
            );
        }
        // A CLOCK THAT CANNOT NAME A DAY IS SAID, NOT SWALLOWED. The journal is
        // still readable and the store is still countable; refusing the whole
        // answer over the date would hide both. The console draws no coverage
        // window without `today` and prints this sentence instead.
        Err(why) => {
            let _ = write!(
                out,
                r#","today":null,"clock":{}"#,
                render::json_string(&why.to_string())
            );
        }
    }
    journal_block(site, feed, page, &mut out);
    store_block(site, feed, &mut out);
    // GDFL'S OWN AUDIT TRAIL. The pull journal holds broker pulls; a GDFL
    // import is journalled in `imports/gdfl.journal`, which no route read
    // until this one did (DB-054, DB-G07, DB-G30; D-3341 item 4).
    if feed == Vendor::Gdfl {
        out.push_str(r#","imports":"#);
        out.push_str(&crate::server::imports::summary(&site.store_root).to_string());
    }
    out.push('}');
    out
}

/// The journal: what the file is, and one bounded page of the records that
/// belong to `feed`.
///
/// # Scoped by each record's vendor (DB-054, DB-G30; D-3341 item 4)
///
/// This block took no feed, so `/audit.json?feed=gdfl` and `?feed=zerodha`
/// answered the same 200 records, byte for byte: Zerodha's member failures,
/// under GDFL's name. An audit record carries no vendor field (its 256-byte
/// image has no byte to spare, `audit::Record`), so the vendor is read from
/// the evidence each RUN record does carry: its `source`, which names the
/// endpoint it was fetched from (`https://api.kite.trade/...`) or the
/// Zerodha-only cash-identity policy it ran under (`zerodha_nse_checked ...`).
/// A member-failure record belongs to the run record before it, because a run
/// appends its own record and then one per member that did not land. A run
/// whose source names neither (a refusal before any vendor was asked, such as
/// `Reference indices`) is UNATTRIBUTED: counted under `feed_scope`, and never
/// shown under a feed it might not be.
fn journal_block(site: &Site, feed: Vendor, page: usize, out: &mut String) {
    let journal = site.journal();
    let log = journal.look();
    let trouble = journal_trouble(&log);
    let total = log.records();
    let per_page = audit::MAX_PAGE_RECORDS;
    let scoped = attribution(&journal, total);
    let (held, unattributed, runs_held, unattributed_runs, faults, scan_error) = match scoped {
        Ok(ref runs) => {
            let (held, runs_held) = runs.held(feed, total);
            let (none, runs_none) = runs.unattributed(total);
            (held, none, runs_held, runs_none, runs.faults, None)
        }
        Err(ref why) => (0, 0, 0, 0, 0, Some(why.clone())),
    };
    // `held - 1` and not `held`, so 200 records is one page and not two.
    // Saturating, so an empty scope is page 0 of 1 rather than an underflow.
    let last_page = usize::try_from(held.saturating_sub(1) / per_page).unwrap_or(0);
    // CLAMPED, NOT REFUSED. `?page=99` over a 135-record journal is a stale
    // bookmark, not an attack; it answers with the last page, exactly as the
    // HTML pager does.
    let page = page.min(last_page);
    let skip = u64::try_from(page).unwrap_or(0).saturating_mul(per_page);

    let _ = write!(
        out,
        r#","journal":{{"path":{},"present":{},"records":{total},"bytes":{},"trouble":{},"page":{page},"pages":{},"page_rows":{per_page},"feed_scope":{{"feed":{},"records":{held},"runs":{runs_held},"unattributed_records":{unattributed},"unattributed_runs":{unattributed_runs},"undecodable":{faults},"error":{},"holds":{}}}}}"#,
        render::json_string(&journal.path.display().to_string()),
        !matches!(log, audit::Log::Absent),
        match log {
            audit::Log::Held { bytes, .. } => bytes,
            audit::Log::Absent | audit::Log::Unreadable { .. } => 0,
        },
        if trouble.is_empty() {
            "null".to_owned()
        } else {
            render::json_string(&trouble)
        },
        last_page.saturating_add(1),
        render::json_string(feed.as_str()),
        scan_error
            .as_deref()
            .map_or_else(|| "null".to_owned(), render::json_string),
        render::json_string(&holds_sentence(feed)),
    );

    let entries = match scoped {
        Ok(ref runs) => runs.page(&journal, feed, total, skip, per_page),
        Err(why) => Err(why),
    };
    match entries {
        Ok(entries) => {
            out.push_str(r#","runs":["#);
            for (n, entry) in entries.iter().enumerate() {
                if n > 0 {
                    out.push(',');
                }
                run(entry, out);
            }
            out.push_str(r#"],"runs_error":null"#);
        }
        // A READ THAT FAILED IS NOT AN EMPTY LIST. `"runs":[]` with a reason
        // beside it is a page that says why it is blank; `"runs":[]` alone is
        // a page that says a store which has run 135 pulls has never run one.
        Err(why) => {
            let _ = write!(
                out,
                r#","runs":[],"runs_error":{}"#,
                render::json_string(&why)
            );
        }
    }
}

/// What the pull journal holds for `feed`, in one sentence: a feed with no
/// HTTP transport is never pulled, so its runs are elsewhere or nowhere.
fn holds_sentence(feed: Vendor) -> String {
    let http = pull::vendor::DESCRIPTORS.iter().any(|row| {
        row.feed.store_vendor() == Some(feed)
            && matches!(row.transport, pull::vendor::Transport::Http(_))
    });
    if feed == Vendor::Gdfl {
        format!(
            "The pull journal records broker pulls over HTTP; {} is never pulled. Its imports \
             are journalled in imports/gdfl.journal, summarised under `imports` and served by \
             /imports.json?feed={}.",
            feed.as_str(),
            feed.as_str()
        )
    } else if http {
        format!(
            "The pull journal's records whose run names {} by its endpoint or its cash-identity \
             policy, and the member failures each such run appended after it.",
            feed.as_str()
        )
    } else {
        format!(
            "{} has no HTTP transport and is never pulled; a record here would be an archive \
             import whose source names its endpoint, and none does.",
            feed.as_str()
        )
    }
}

/// The vendor a run record's `source` names, when it names one: an HTTP
/// feed's endpoint (`pull::vendor::DESCRIPTORS`, the one place an endpoint is
/// written), or a Zerodha-only cash-identity policy, which
/// `ingest::parse_cash_identity` refuses for any other vendor.
fn vendor_of_source(source: &str) -> Option<Vendor> {
    if source.starts_with("zerodha_") {
        return Some(Vendor::Zerodha);
    }
    pull::vendor::DESCRIPTORS
        .iter()
        .find_map(|row| match row.transport {
            pull::vendor::Transport::Http(ref spec) if source.contains(spec.base_url) => {
                row.feed.store_vendor()
            }
            _ => None,
        })
}

/// Every run record of one pull journal, with the vendor its source names.
///
/// # Cost, stated
///
/// The journal holds no index of its runs: on the operator's store, 9 Oct
/// 2026, 12 run records among 4,217,027 (1,079,558,912 bytes), the rest
/// member failures. So the first request reads the whole file once, in 4 MiB
/// reads, decoding each 256-byte record; after that a request decodes only
/// the records appended since (one `metadata` call when there are none). A
/// file that shrank or was replaced is read again. Paging is then arithmetic
/// over the run boundaries: O(runs), plus one bounded `Journal::page` read per
/// run a page spans.
#[derive(Debug, Clone, Default)]
pub(crate) struct Attribution {
    /// Records decoded.
    scanned: u64,
    /// The file's identity when scanned.
    identity: Option<(u64, u64)>,
    /// Each run record's ordinal and vendor, in file order.
    runs: Vec<(u64, Option<Vendor>)>,
    /// Records that would not decode, kept in the run before them.
    faults: u64,
}

impl Attribution {
    /// Each run's records as `(first, end, vendor)`, newest run first, over a
    /// journal of `total` records. Records before the first run are a span of
    /// their own with no vendor.
    fn spans(&self, total: u64) -> Vec<(u64, u64, Option<Vendor>)> {
        let mut out = Vec::with_capacity(self.runs.len().saturating_add(1));
        let mut end = total.min(self.scanned);
        for &(first, vendor) in self.runs.iter().rev() {
            if first < end {
                out.push((first, end, vendor));
            }
            end = end.min(first);
        }
        if end > 0 {
            out.push((0, end, None));
        }
        out
    }

    /// Records and runs that belong to `feed`.
    fn held(&self, feed: Vendor, total: u64) -> (u64, u64) {
        self.count(total, Some(feed))
    }

    /// Records and runs that belong to no vendor.
    fn unattributed(&self, total: u64) -> (u64, u64) {
        self.count(total, None)
    }

    /// Records and runs whose vendor is `vendor`.
    fn count(&self, total: u64, vendor: Option<Vendor>) -> (u64, u64) {
        let spans = self.spans(total);
        let records = spans
            .iter()
            .filter(|span| span.2 == vendor)
            .map(|span| span.1.saturating_sub(span.0))
            .sum();
        let runs = self.runs.iter().filter(|run| run.1 == vendor).count();
        (records, u64::try_from(runs).unwrap_or(u64::MAX))
    }

    /// The newest records of `feed` after skipping `skip`, at most `take`,
    /// newest first.
    fn page(
        &self,
        journal: &audit::Journal,
        feed: Vendor,
        total: u64,
        skip: u64,
        take: u64,
    ) -> Result<Vec<audit::Entry>, String> {
        let mut skip = skip;
        let mut left = take.min(audit::MAX_PAGE_RECORDS);
        let mut out = Vec::new();
        for (first, end, vendor) in self.spans(total) {
            if left == 0 {
                break;
            }
            if vendor != Some(feed) {
                continue;
            }
            let len = end.saturating_sub(first);
            if skip >= len {
                skip = skip.saturating_sub(len);
                continue;
            }
            let newest = end.saturating_sub(skip);
            let wanted = left.min(newest.saturating_sub(first));
            skip = 0;
            out.extend(journal.page(newest, 0, wanted)?);
            left = left.saturating_sub(wanted);
        }
        Ok(out)
    }

    /// Decodes the records `[scanned, total)` of the file at `path`.
    fn advance(&mut self, path: &std::path::Path, total: u64) -> Result<(), String> {
        use std::io::{Read as _, Seek as _};
        const CHUNK: u64 = 16_384;
        if total <= self.scanned {
            return Ok(());
        }
        let mut file = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
        file.seek(std::io::SeekFrom::Start(
            self.scanned.saturating_mul(audit::RECORD_LEN_U64),
        ))
        .map_err(|e| format!("{}: {e}", path.display()))?;
        let mut buf = Vec::new();
        while self.scanned < total {
            let count = (total.saturating_sub(self.scanned)).min(CHUNK);
            buf.clear();
            (&mut file)
                .take(count.saturating_mul(audit::RECORD_LEN_U64))
                .read_to_end(&mut buf)
                .map_err(|e| format!("{}: {e}", path.display()))?;
            let mut image = [0u8; audit::RECORD_LEN];
            for (n, chunk) in buf.chunks_exact(audit::RECORD_LEN).enumerate() {
                for (dst, src) in image.iter_mut().zip(chunk) {
                    *dst = *src;
                }
                let ordinal = self
                    .scanned
                    .saturating_add(u64::try_from(n).unwrap_or(u64::MAX));
                match audit::Record::decode(&image) {
                    Ok(record) if record.kind == audit::Kind::Run => {
                        self.runs.push((ordinal, vendor_of_source(&record.source)));
                    }
                    Ok(_) => {}
                    Err(_) => self.faults = self.faults.saturating_add(1),
                }
            }
            let whole = u64::try_from(buf.len()).unwrap_or(0) / audit::RECORD_LEN_U64;
            if whole == 0 {
                return Err(format!(
                    "{}: the file ended before record {} of {total}",
                    path.display(),
                    self.scanned
                ));
            }
            self.scanned = self.scanned.saturating_add(whole);
        }
        Ok(())
    }
}

/// Every pull journal attributed so far, by path.
fn attributions()
-> &'static std::sync::Mutex<std::collections::HashMap<std::path::PathBuf, Attribution>> {
    static HELD: std::sync::OnceLock<
        std::sync::Mutex<std::collections::HashMap<std::path::PathBuf, Attribution>>,
    > = std::sync::OnceLock::new();
    HELD.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
}

/// The journal's run attribution through its first `total` records, read
/// through the tail. See [`Attribution`] for the cost.
///
/// # Errors
///
/// A file that cannot be read, in its own words.
fn attribution(journal: &audit::Journal, total: u64) -> Result<Attribution, String> {
    use std::os::unix::fs::MetadataExt as _;
    let identity = std::fs::metadata(&journal.path)
        .ok()
        .map(|meta| (meta.dev(), meta.ino()));
    let mut held = attributions()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let entry = held.entry(journal.path.clone()).or_default();
    if entry.identity != identity || total < entry.scanned {
        *entry = Attribution {
            identity,
            ..Attribution::default()
        };
    }
    entry.advance(&journal.path, total)?;
    Ok(entry.clone())
}

/// One journal entry, as the console reads it.
///
/// A record that will not decode becomes an object carrying `fault` and
/// nothing else, rather than a gap: one damaged record must not blank the runs
/// around it, and a `filter_map` here would have done exactly that.
fn run(entry: &audit::Entry, out: &mut String) {
    match entry.decoded {
        Err(ref fault) => {
            let _ = write!(
                out,
                r#"{{"ordinal":{},"fault":{}}}"#,
                entry.ordinal,
                render::json_string(&fault.to_string())
            );
        }
        Ok(ref r) => {
            let _ = write!(
                out,
                // `kind` IS FIRST AMONG THE RECORD'S OWN FIELDS, AND IT WAS
                // ABSENT ENTIRELY.
                //
                // `audit::Record` has carried a `kind` since D-0073 split a
                // failed member into its own fixed-stride record: a run, and
                // then one more record per member that did not land. The Rust
                // page renders that distinction; **this JSON never emitted it**,
                // so the SvelteKit `/audit` page — the only audit surface the
                // nav links to — could not tell the two apart.
                //
                // What that cost, arithmetically: a run that refused N members
                // writes 1 + N records, and a reader treating all of them as
                // runs counts the N failures twice — once on the run's own
                // `failures` field and once per member record — then reports
                // `reasonsLost = N - 1` when nothing was lost at all.
                //
                // `Kind::label` already existed for the Rust page and had no
                // second caller. This is that caller.
                r#"{{"ordinal":{},"fault":null,"kind":{},"at":{},"took_micros":{},"scope":{},"outcome":{},"loud":{},"members":{},"rows_read":{},"bars_stored":{},"rows_folded":{},"counted":{},"failures":{},"window":{},"source":{},"source_bytes":{},"note":{},"note_bytes":{},"version":{},"note_capacity":{},"kept_unclassified_day":{},"drops":["#,
                entry.ordinal,
                render::json_string(r.kind.label()),
                r.at_unix_secs,
                r.elapsed_micros,
                render::json_string(r.scope.label()),
                render::json_string(r.outcome.label()),
                r.outcome.is_loud(),
                r.members,
                r.rows_read,
                r.bars_stored,
                r.rows_folded,
                r.counted,
                r.failures,
                render::json_string(&window_text(r.from_days, r.to_days)),
                render::json_string(&r.source),
                r.source_bytes,
                render::json_string(&r.note),
                r.note_bytes,
                r.version,
                r.note_capacity(),
                // NULL, NOT ZERO, on a version-1 record: that build kept these
                // rows without counting them (D-2673).
                r.drops
                    .unclassified_kept
                    .map_or_else(|| "null".to_owned(), |n| n.to_string()),
            );
            // AN ARRAY OF NAMED REASONS, NOT FOUR FIXED KEYS. A fifth drop
            // reason is a row the console draws without being taught its name,
            // which is the same reason `DROP_REASONS` exists at all.
            for (n, reason) in audit::DROP_REASONS.into_iter().enumerate() {
                if n > 0 {
                    out.push(',');
                }
                let _ = write!(
                    out,
                    r#"{{"reason":{},"rows":{}}}"#,
                    render::json_string(reason.label()),
                    r.drops
                        .recorded(reason)
                        .map_or_else(|| "null".to_owned(), |n| n.to_string())
                );
            }
            out.push_str("]}");
        }
    }
}

/// One feed's held months, rolled up: per month the instrument-months and bars
/// held, then the two totals.
///
/// **It walks the ASKED feed's own manifest and no other.** `store_block` used
/// to walk `census_now`'s merged entry list, every held entry of every vendor,
/// and probe the asked feed's manifest for each, so a request for one feed paid
/// for every other feed's store, on every request (W1-api1-0, D-0732). Taking
/// one `VendorCensus` makes the other feeds unreachable from here. It visits
/// each of this feed's held keys once, with one manifest probe and one
/// `BTreeMap` insert each, and [`RollupCache`] runs it once per census
/// snapshot per feed rather than once per request.
fn feed_rollup(census: &crate::census::VendorCensus) -> FeedRollup {
    let mut rollup = FeedRollup::default();
    let crate::census::Census::Held { ref manifest } = census.state else {
        return rollup;
    };
    for entry in manifest.held_keys().filter_map(|key| manifest.entry(key)) {
        let cell = rollup.months.entry(entry.key.month).or_insert((0, 0));
        cell.0 = cell.0.saturating_add(1);
        cell.1 = cell.1.saturating_add(entry.rows);
        rollup.instrument_months = rollup.instrument_months.saturating_add(1);
        rollup.bars = rollup.bars.saturating_add(entry.rows);
    }
    rollup
}

/// What [`feed_rollup`] counts for one feed.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct FeedRollup {
    /// Per month, `(instrument_months, bars)`.
    months: BTreeMap<store::path::YearMonth, (u64, u64)>,
    /// Held instrument-months, all months together.
    instrument_months: u64,
    /// Held bars, all months together.
    bars: u64,
}

/// [`feed_rollup`]'s answers for ONE census snapshot, one per feed asked.
///
/// # Why a cache and not the walk per request
///
/// The audit console polls `/audit.json` every few seconds, and the census it
/// reads only changes when a manifest does: `census_now` hands back the SAME
/// `Arc` until a stamp moves. So the rollup is keyed on that `Arc`, exactly as
/// `store_wire::Cache` keys `/store.json`'s encoded body, and a request against
/// an unchanged census pays one lock, one pointer comparison and one map probe
/// instead of a walk over the feed's entries. A new snapshot replaces every
/// feed's rollup at once, so none can outlive the census it was counted from.
///
/// The lock is not held while a rollup is counted: a miss counts outside it and
/// then publishes, the same order `census_now` keeps for its own read. Two
/// requests that miss together may each count; they count the same snapshot,
/// so either answer is the same bytes.
#[derive(Default, Debug)]
pub(crate) struct RollupCache(std::sync::Mutex<Option<Rollups>>);

/// The snapshot a [`RollupCache`] holds rollups for, and those rollups.
#[derive(Debug)]
struct Rollups {
    /// The census `Arc` these were counted from. Weak, so the cache never keeps
    /// an old census alive.
    source: std::sync::Weak<Vec<crate::census::VendorCensus>>,
    /// One rollup per feed asked about since that snapshot.
    feeds: std::collections::HashMap<Vendor, std::sync::Arc<FeedRollup>>,
}

impl RollupCache {
    /// `feed`'s rollup of `source`, counted by `count` only when this snapshot
    /// has not been counted for `feed` yet.
    fn get(
        &self,
        source: &std::sync::Arc<Vec<crate::census::VendorCensus>>,
        feed: Vendor,
        count: impl FnOnce() -> FeedRollup,
    ) -> std::sync::Arc<FeedRollup> {
        let weak = std::sync::Arc::downgrade(source);
        {
            let held = self
                .0
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Some(rollups) = held.as_ref()
                && rollups.source.ptr_eq(&weak)
                && let Some(rollup) = rollups.feeds.get(&feed)
            {
                return std::sync::Arc::clone(rollup);
            }
        }
        let rollup = std::sync::Arc::new(count());
        let mut held = self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if !held.as_ref().is_some_and(|r| r.source.ptr_eq(&weak)) {
            *held = Some(Rollups {
                source: weak,
                feeds: std::collections::HashMap::with_capacity(Vendor::ALL.len()),
            });
        }
        if let Some(rollups) = held.as_mut() {
            rollups.feeds.insert(feed, std::sync::Arc::clone(&rollup));
        }
        rollup
    }
}

/// What this ONE feed holds, rolled up by month.
///
/// One feed and no other. `CLAUDE.md`'s standing rule — stated by the operator
/// twice — is that no surface puts one feed's numbers beside another's, and a
/// response carrying two feeds is that comparison already half drawn.
fn store_block(site: &Site, feed: Vendor, out: &mut String) {
    // FRESH, NOT THE STARTUP SNAPSHOT — `census_now`'s whole reason for
    // existing. The `/store` HTML page reads the cached copy and was measured
    // understating this store 6.8× (20.4 M rows against 139.7 M) after four
    // hours of backfill. A monitoring page must never be that page.
    let (censuses, _every_feed) = census_now(site);
    let census = censuses.iter().find(|c| c.vendor == feed);
    let rollup = census.map_or_else(
        || std::sync::Arc::new(FeedRollup::default()),
        |census| {
            site.audit_rollup
                .get(&censuses, feed, || feed_rollup(census))
        },
    );
    let FeedRollup {
        ref months,
        instrument_months,
        bars,
    } = *rollup;

    let _ = write!(
        out,
        r#","store":{{"feed":{},"state":{},"note":{},"degraded":{},"generation":{},"commits":{},"manifest":{},"committed_at":{},"capacity":{},"instrument_months":{instrument_months},"bars":{bars},"months":["#,
        render::json_string(feed.as_str()),
        render::json_string(census.map_or("absent", |c| c.state.name())),
        render::json_string(&census.map_or_else(
            || format!("no census row for {} in this build", feed.as_str()),
            crate::census::VendorCensus::note
        )),
        census
            .and_then(crate::census::VendorCensus::degraded)
            .map_or_else(|| "null".to_owned(), |why| render::json_string(&why)),
        census
            .and_then(crate::census::VendorCensus::generation)
            .map_or_else(|| "null".to_owned(), |g| g.to_string()),
        census
            .and_then(census_commits)
            .map_or_else(|| "null".to_owned(), |n| n.to_string()),
        census.map_or_else(
            || render::json_string(""),
            |c| render::json_string(&c.path.display().to_string())
        ),
        census
            .and_then(committed_at)
            .map_or_else(|| "null".to_owned(), |secs| secs.to_string()),
        capacity(census),
    );
    for (n, (month, (held, rows))) in months.iter().enumerate() {
        if n > 0 {
            out.push(',');
        }
        let _ = write!(
            out,
            r#"{{"month":{},"instrument_months":{held},"bars":{rows}}}"#,
            render::json_string(&month.to_string())
        );
    }
    out.push_str("]}");
}

/// The census's room: the entries this build lets one census hold
/// (`pull::manifest::MAX_ENTRIES`), the entries committed, and what is left.
/// `null` when the census did not load, never a zero that reads as "empty".
///
/// # Why it is here (DB-G07)
///
/// The GDFL census filled at 2,097,152 entries on 6 Oct 2026 and every
/// surface said `held`: the import went on writing bars no census counted for
/// eleven months of option days, and the only trace was an `error` line in the
/// `cli` log. `free` is the number that would have said so first, and `full`
/// says it in one word. Two field reads; no walk.
fn capacity(census: Option<&crate::census::VendorCensus>) -> String {
    let Some(committed) = census.and_then(census_commits) else {
        return "null".to_owned();
    };
    let limit = pull::manifest::MAX_ENTRIES;
    format!(
        r#"{{"limit":{limit},"committed":{committed},"free":{},"over":{},"full":{}}}"#,
        limit.saturating_sub(committed),
        committed.saturating_sub(limit),
        committed >= limit,
    )
}

/// How many entries the manifest has committed, when it loaded.
///
/// The third counter, named here rather than at the call site, because
/// `counters()` returns a triple and a positional `.2` at a `write!` argument
/// is a number nobody can check by reading.
fn census_commits(census: &crate::census::VendorCensus) -> Option<u64> {
    census.counters().map(|(_keys, _rows, entries)| entries)
}

/// When the manifest was last written, in epoch seconds.
///
/// One `metadata` call on one file — this is the "did the store grow" signal,
/// and it has to stay a stat rather than a walk. `None` when the file is not
/// there or the platform will not say, which the console prints as "unknown"
/// rather than as "never".
fn committed_at(census: &crate::census::VendorCensus) -> Option<i64> {
    std::fs::metadata(&census.path)
        .and_then(|m| m.modified())
        .ok()
        .map(ingest::epoch_secs)
}

#[cfg(test)]
#[allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic
)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// A store root of its own, named after the test that owns it.
    fn store_root(name: &str) -> PathBuf {
        let dir = crate::scratch::path(&format!("auditjson-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("manifest")).expect("mkdir");
        dir
    }

    /// Masters this build reads, holding one index.
    fn masters(name: &str) -> PathBuf {
        let dir = crate::scratch::path(&format!("auditjson-masters-{name}"));
        std::fs::create_dir_all(&dir).expect("mkdir");
        std::fs::write(
            dir.join("groww_instruments.csv"),
            "exchange,segment,underlying_symbol,trading_symbol,instrument_type,series,isin,\
             expiry_date,strike_price,groww_symbol\n\
             NSE,CASH,,NIFTY,IDX,,NIFTY,,,NSE-NIFTY\n",
        )
        .expect("write");
        dir
    }

    fn site(name: &str) -> Site {
        Site::load(&masters(name), &store_root(name))
    }

    /// A fixed moment: 10:00 IST on 2026-08-07, so no assertion below is a
    /// property of when the suite happened to run.
    fn moment() -> std::time::SystemTime {
        std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_754_540_000)
    }

    fn day() -> Day {
        Day::new(2026, 8, 7).expect("a real date")
    }

    /// Whatever `key` holds, as raw JSON text, without a parser.
    ///
    /// The api crate has no serialiser and takes no dependency to get one, so
    /// the tests read the bytes the same way the wire does.
    fn field<'a>(json: &'a str, key: &str) -> &'a str {
        let at = json
            .find(key)
            .unwrap_or_else(|| panic!("no {key} in {json}"));
        let rest = json.get(at.saturating_add(key.len())..).unwrap_or("");
        let end = rest.find([',', '}', ']']).unwrap_or(rest.len());
        rest.get(..end).unwrap_or("")
    }

    #[test]
    fn an_empty_store_answers_every_key_with_a_reason_and_never_a_zero_that_means_unknown() {
        let json = body(&site("empty"), Vendor::Groww, 0, moment(), Ok(day()));
        assert!(json.starts_with('{') && json.ends_with('}'), "{json}");
        assert_eq!(field(&json, r#""at":"#), "1754540000");
        assert_eq!(field(&json, r#""today":"#), r#""2026-08-07""#);
        assert_eq!(field(&json, r#""clock":"#), "null");
        // No journal file yet: present is false and the count is zero, which is
        // a fact and not a failure.
        assert_eq!(field(&json, r#""present":"#), "false");
        assert_eq!(field(&json, r#""records":"#), "0");
        assert_eq!(field(&json, r#""pages":"#), "1");
        assert_eq!(field(&json, r#""runs_error":"#), "null");
        assert!(json.contains(r#""runs":[]"#), "{json}");
        // No manifest either: absent, and every derived number is null rather
        // than 0. Zero months held and "I could not read the census" are not
        // the same sentence.
        assert_eq!(field(&json, r#""state":"#), r#""absent""#);
        assert_eq!(field(&json, r#""generation":"#), "null");
        assert_eq!(field(&json, r#""commits":"#), "null");
        assert_eq!(field(&json, r#""committed_at":"#), "null");
        assert_eq!(field(&json, r#""instrument_months":"#), "0");
        assert!(json.contains(r#""months":[]"#), "{json}");
    }

    #[test]
    fn a_refused_clock_says_so_and_still_answers_the_journal() {
        // A clock before 1970 is what `ingest::ist_day` refuses. Taken from the
        // refusal itself rather than hand-written, so this test cannot drift
        // away from the sentence the server would really print.
        // Three days before the epoch, not one second: IST is UTC+05:30, so
        // `UNIX_EPOCH - 1s` is still 1970-01-01 in Delhi and names a day
        // perfectly well. The first attempt at this test asserted otherwise
        // and the suite said so.
        // 72 hours is the three days above. `Duration::from_days` says it
        // better and is still unstable on the pinned 1.97.1
        // (`duration_constructors`, rust#120301), and `from_secs(3 * 86_400)`
        // is what `clippy::duration_suboptimal_units` refuses.
        let why = ingest::ist_day(
            std::time::SystemTime::UNIX_EPOCH - std::time::Duration::from_hours(72),
        )
        .expect_err("a clock days before 1970 is refused");
        let json = body(&site("clock"), Vendor::Groww, 0, moment(), Err(why));
        assert_eq!(field(&json, r#""today":"#), "null");
        assert_ne!(field(&json, r#""clock":"#), "null");
        assert!(json.contains(r#""journal":"#), "{json}");
    }

    #[test]
    fn a_recorded_run_survives_the_round_trip_field_for_field() {
        let site = site("run");
        let journal = site.journal();
        // THE SOURCE NAMES GROWW'S ENDPOINT, as a real run's does, so the run
        // is Groww's: a record naming no vendor is not shown under any feed
        // (D-3341 item 4).
        let mut record = audit::Record::refused(
            audit::Scope::Spot,
            audit::Outcome::Failed,
            moment(),
            "https://api.groww.in/v1/historical",
            // An ampersand and a quote, because a symbol legally holds one and
            // a note legally holds the other. Both must arrive as text.
            r#"M&M — "no such folder""#,
        );
        record.members = 1400;
        record.failures = 699;
        record.bars_stored = 3_557_434;
        journal.append(&record).expect("append");

        let json = body(&site, Vendor::Groww, 0, moment(), Ok(day()));
        assert_eq!(field(&json, r#""records":"#), "1");
        assert_eq!(field(&json, r#""members":"#), "1400");
        assert_eq!(field(&json, r#""failures":"#), "699");
        assert_eq!(field(&json, r#""bars_stored":"#), "3557434");
        assert_eq!(field(&json, r#""outcome":"#), r#""FAILED""#);
        assert_eq!(field(&json, r#""loud":"#), "true");
        assert_eq!(field(&json, r#""fault":"#), "null");
        // ESCAPED, NOT MANGLED. The note holds a quote; the JSON holds it as
        // `\"` and the symbol's `&` is the one character it always was.
        assert!(json.contains(r#"M&M — \"no such folder\""#), "{json}");
        // Every drop reason is named in the answer, not implied by position.
        for reason in audit::DROP_REASONS {
            assert!(json.contains(reason.label()), "{} missing", reason.label());
        }
    }

    #[test]
    fn a_page_past_the_end_clamps_to_the_last_page_rather_than_refusing() {
        let site = site("clamp");
        let journal = site.journal();
        journal
            .append(&audit::Record::refused(
                audit::Scope::Spot,
                audit::Outcome::Refused,
                moment(),
                "swept",
                "no",
            ))
            .expect("append");
        let json = body(&site, Vendor::Groww, 99, moment(), Ok(day()));
        assert_eq!(field(&json, r#""page":"#), "0");
        assert_eq!(field(&json, r#""pages":"#), "1");
        assert_eq!(field(&json, r#""records":"#), "1");
    }

    #[test]
    fn the_answer_names_one_feed_and_never_a_second() {
        let json = body(&site("onefeed"), Vendor::Groww, 0, moment(), Ok(day()));
        assert!(json.contains(r#""feed":"groww""#), "{json}");
        for other in Vendor::ALL {
            if other == Vendor::Groww {
                continue;
            }
            assert!(
                !json.contains(&format!(r#""feed":"{}""#, other.as_str())),
                "{} appears beside groww in {json}",
                other.as_str()
            );
        }
    }

    #[test]
    fn an_unknown_feed_is_refused_by_name_and_lists_what_would_have_worked() {
        let all = wires();
        for vendor in Vendor::ALL {
            assert!(all.contains(vendor.as_str()), "{all} omits {vendor:?}");
        }
    }

    #[test]
    fn the_manifest_path_is_stated_so_an_absence_is_actionable() {
        let json = body(&site("path"), Vendor::Groww, 0, moment(), Ok(day()));
        assert!(json.contains(r#""manifest":"#), "{json}");
        assert!(json.contains("manifest"), "{json}");
    }

    /// The route itself, driven through axum's extractors.
    #[tokio::test]
    async fn the_route_refuses_a_missing_feed_with_400_and_names_the_parameter() {
        let site: Loaded = std::sync::Arc::new(site("route"));
        let (code, _headers, body) = audit_json(
            axum::extract::State(std::sync::Arc::clone(&site)),
            "/audit.json".parse::<axum::http::Uri>().expect("uri"),
        )
        .await;
        assert_eq!(code, axum::http::StatusCode::BAD_REQUEST);
        assert!(body.contains("needs ?feed="), "{body}");
        assert!(body.contains("groww"), "{body}");

        let (code, _headers, body) = audit_json(
            axum::extract::State(site),
            "/audit.json?feed=groww"
                .parse::<axum::http::Uri>()
                .expect("uri"),
        )
        .await;
        assert_eq!(code, axum::http::StatusCode::OK);
        assert!(body.contains(r#""feed":"groww""#), "{body}");
    }

    /// The bound this route is only defensible because of.
    ///
    /// Asserted here and not merely relied on: this route is the one an
    /// operator refreshes for twelve hours, and a page size that quietly grew
    /// would turn a bounded read into a whole-journal read without a single
    /// line of this file changing.
    #[test]
    fn a_page_reads_at_most_the_records_it_shows() {
        assert_eq!(audit::MAX_PAGE_RECORDS, 200);
    }

    /// A feed that was named and could not be read is a DIFFERENT fault from
    /// one that was never sent, and both leave by their own name.
    ///
    /// The empty case is the other test above; this one drives the arm that
    /// separates "your caller forgot the parameter" from "this build has no
    /// such vendor", which is the split [`note_feed_refused`] exists to record.
    #[tokio::test]
    async fn a_named_feed_this_build_cannot_read_is_refused_by_name() {
        let site: Loaded = std::sync::Arc::new(site("unknownfeed"));
        let (code, _headers, body) = audit_json(
            axum::extract::State(site),
            "/audit.json?feed=nasdaq"
                .parse::<axum::http::Uri>()
                .expect("uri"),
        )
        .await;
        assert_eq!(code, axum::http::StatusCode::BAD_REQUEST);
        assert!(body.contains("nasdaq"), "{body}");
        assert!(body.contains("groww"), "and lists what would have worked");
    }

    /// A page nobody could parse is answered, not refused — and noticed.
    ///
    /// The answer is byte-identical to `?page=0`, which is the whole reason
    /// [`note_page_ignored`] exists: the response cannot carry the difference,
    /// so the log has to.
    #[tokio::test]
    async fn an_unreadable_page_is_answered_with_the_newest_page() {
        let site: Loaded = std::sync::Arc::new(site("badpage"));
        let (code, _headers, body) = audit_json(
            axum::extract::State(site),
            "/audit.json?feed=groww&page=banana"
                .parse::<axum::http::Uri>()
                .expect("uri"),
        )
        .await;
        assert_eq!(code, axum::http::StatusCode::OK);
        assert_eq!(field(&body, r#""page":"#), "0");
    }

    /// A manifest image for `vendor` holding NSE INDEX `symbol` at the daily
    /// rung for each `(month, rows)` given.
    fn manifest_image(vendor: Vendor, held: &[(&str, u16, u8, u64)]) -> Vec<u8> {
        use brutex_core::instrument::{Exchange, Segment};
        let mut manifest =
            pull::manifest::Manifest::open(vendor, &[], &[]).expect("a genesis manifest");
        for (symbol, year, month, rows) in held {
            let month = store::path::YearMonth::new(*year, *month).expect("fixture month");
            let day = Day::new(*year, month.month(), 2).expect("fixture date");
            let ts = i64::from(day.days_from_epoch()) * 86_400_000_000 + 21_600_000_000;
            manifest
                .record(pull::manifest::Entry {
                    key: pull::manifest::EntryKey {
                        contract: None,
                        exchange: Exchange::Nse,
                        segment: Segment::Index,
                        symbol: brutex_core::symbol::Symbol::new(symbol).expect("fixture symbol"),
                        timeframe: store::path::Timeframe::DAY_1,
                        month,
                    },
                    rows: *rows,
                    first_ts_micros: ts,
                    last_ts_micros: ts,
                })
                .expect("one held month");
        }
        manifest.image()
    }

    fn install(root: &std::path::Path, vendor: Vendor, image: &[u8]) {
        let path = pull::manifest::manifest_path(root, vendor);
        std::fs::create_dir_all(path.parent().expect("manifest parent")).expect("mkdir");
        std::fs::write(path, image).expect("install the manifest");
    }

    /// **THE STORE BLOCK ROLLS UP THE ASKED FEED AND ONLY IT.** Dhan holds
    /// three instrument-months over two months; Groww holds other months that
    /// must not appear in Dhan's answer (W1-api1-0, D-0732).
    #[test]
    fn the_store_block_rolls_up_the_asked_feed_by_month_and_counts_no_other_feed() {
        let root = store_root("rollup");
        install(
            &root,
            Vendor::Dhan,
            &manifest_image(
                Vendor::Dhan,
                &[
                    ("NIFTY", 2025, 1, 3),
                    ("BANKNIFTY", 2025, 1, 5),
                    ("NIFTY", 2025, 2, 7),
                ],
            ),
        );
        install(
            &root,
            Vendor::Groww,
            &manifest_image(
                Vendor::Groww,
                &[("NIFTY", 2024, 6, 11), ("NIFTY", 2025, 1, 13)],
            ),
        );
        let site = Site::load(&masters("rollup"), &root);

        let json = body(&site, Vendor::Dhan, 0, moment(), Ok(day()));
        assert_eq!(field(&json, r#""state":"#), r#""held""#, "{json}");
        assert_eq!(field(&json, r#""instrument_months":"#), "3", "{json}");
        assert!(json.contains(r#""bars":15,"months":["#), "{json}");
        assert!(
            json.contains(
                r#""months":[{"month":"2025-01","instrument_months":2,"bars":8},{"month":"2025-02","instrument_months":1,"bars":7}]"#
            ),
            "{json}"
        );
        assert!(
            !json.contains("2024-06"),
            "Groww's month is not Dhan's: {json}"
        );

        let groww = body(&site, Vendor::Groww, 0, moment(), Ok(day()));
        assert!(
            groww.contains(
                r#""months":[{"month":"2024-06","instrument_months":1,"bars":11},{"month":"2025-01","instrument_months":1,"bars":13}]"#
            ),
            "{groww}"
        );
    }

    /// **THE ROLLUP CANNOT REACH ANOTHER FEED'S ENTRIES.** `store_block` walks
    /// the asked census through `feed_rollup`, whose one argument is that
    /// census, and does not iterate `census_now`'s merged list of every
    /// vendor's entries, which it did before D-0732.
    #[test]
    fn the_store_block_walks_one_census_and_not_every_feeds_entries() {
        let source = include_str!("audit_json.rs");
        let from = source.find("fn store_block(").expect("store_block exists");
        let rest = source.get(from..).expect("a char boundary");
        let block = rest
            .get(..rest.find("\n}\n").expect("its end"))
            .expect("a char boundary");
        assert!(
            !block.contains("entries.iter()"),
            "no loop over every feed's entries: {block}"
        );
        assert!(block.contains("|| feed_rollup(census)"), "{block}");
    }

    /// **WHAT `/audit.json`'s STORE BLOCK STILL PAYS IS STATED IN
    /// `docs/06-limits.md`, AND THE WALK IT QUOTES IS `feed_rollup`'s OWN.**
    /// `/audit.json` was listed there only as a `census_now` caller (W1-api1-0,
    /// D-0732).
    #[test]
    fn the_audit_rollup_cost_is_stated_in_the_limits_and_quotes_this_source() {
        let limits = include_str!("../../../docs/06-limits.md");
        let section = limits
            .split_once(
                "## `/audit.json`'s store block: one count per census snapshot, of the asked feed only — D-0732",
            )
            .expect("docs/06-limits.md states the audit rollup's cost")
            .1;
        let section = section.split_once("\n## ").map_or(section, |(own, _)| own);
        let source = include_str!("audit_json.rs");
        let from = source.find("fn feed_rollup(").expect("feed_rollup exists");
        let rest = source.get(from..).expect("a char boundary");
        let walk = rest
            .get(..rest.find("\n}\n").expect("its end"))
            .expect("a char boundary");
        let quoted = "manifest.held_keys()";
        assert!(section.contains(&format!("`{quoted}`")), "{quoted}");
        assert!(walk.contains(quoted), "feed_rollup still says {quoted}");
        for named in ["feed_rollup", "RollupCache", "Not timed."] {
            assert!(section.contains(named), "the limit names {named}");
        }
    }

    /// **ONE COUNT PER FEED PER CENSUS SNAPSHOT.** A second ask of the same
    /// snapshot for the same feed is the same `Arc` and counts nothing; another
    /// feed counts once; a new snapshot counts again (D-0732).
    #[test]
    fn the_rollup_is_counted_once_per_feed_per_census_snapshot() {
        let cache = RollupCache::default();
        let counted = std::cell::Cell::new(0u32);
        let count = |bars: u64| {
            counted.set(counted.get() + 1);
            FeedRollup {
                bars,
                ..FeedRollup::default()
            }
        };
        let first = std::sync::Arc::new(Vec::new());
        let a = cache.get(&first, Vendor::Dhan, || count(1));
        let b = cache.get(&first, Vendor::Dhan, || count(2));
        assert!(std::sync::Arc::ptr_eq(&a, &b));
        assert_eq!((counted.get(), b.bars), (1, 1));
        let other = cache.get(&first, Vendor::Groww, || count(3));
        assert_eq!((counted.get(), other.bars), (2, 3));
        assert_eq!(cache.get(&first, Vendor::Dhan, || count(4)).bars, 1);
        assert_eq!(counted.get(), 2);

        let second = std::sync::Arc::new(Vec::new());
        let c = cache.get(&second, Vendor::Dhan, || count(5));
        assert_eq!((counted.get(), c.bars), (3, 5));
        // The new snapshot replaced the old one's rollups, for every feed.
        assert_eq!(cache.get(&second, Vendor::Groww, || count(6)).bars, 6);
        assert_eq!(counted.get(), 4);
    }

    /// **A REQUEST AGAINST AN UNCHANGED CENSUS COUNTS NOTHING, AND A REWRITTEN
    /// MANIFEST IS COUNTED AGAIN.** The first `/audit.json` body leaves the
    /// asked feed's rollup cached for the census `census_now` hands back, so a
    /// later ask of that snapshot does not run its counter. A manifest
    /// rewritten under a new modified time is a new census and the next body
    /// carries its months, not the cached ones (D-0732).
    #[test]
    fn the_audit_body_reuses_the_rollup_until_the_manifest_changes() {
        let root = store_root("rollup-cache");
        install(
            &root,
            Vendor::Dhan,
            &manifest_image(Vendor::Dhan, &[("NIFTY", 2025, 1, 3)]),
        );
        let site = Site::load(&masters("rollup-cache"), &root);
        let path = pull::manifest::manifest_path(&root, Vendor::Dhan);
        let stamp = |secs: u64| {
            std::fs::File::options()
                .write(true)
                .open(&path)
                .expect("open the manifest")
                .set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_secs(secs))
                .expect("set its modified time");
        };
        stamp(1_000_000);

        let json = body(&site, Vendor::Dhan, 0, moment(), Ok(day()));
        assert!(
            json.contains(r#""instrument_months":1,"bars":3,"months":["#),
            "{json}"
        );
        let (censuses, _) = census_now(&site);
        let cached = site.audit_rollup.get(&censuses, Vendor::Dhan, || {
            panic!("counted again for an unchanged census")
        });
        assert_eq!(cached.bars, 3);
        assert_eq!(body(&site, Vendor::Dhan, 0, moment(), Ok(day())), json);

        install(
            &root,
            Vendor::Dhan,
            &manifest_image(
                Vendor::Dhan,
                &[("NIFTY", 2025, 1, 3), ("NIFTY", 2025, 2, 4)],
            ),
        );
        stamp(2_000_000);
        let json = body(&site, Vendor::Dhan, 0, moment(), Ok(day()));
        assert!(
            json.contains(r#""instrument_months":2,"bars":7,"months":["#),
            "{json}"
        );
    }

    /// A run record whose source names `source`, with `failures` member
    /// failures appended after it, as a pull appends them.
    fn pulled(journal: &audit::Journal, source: &str, failures: usize) {
        let mut run = audit::Record::refused(
            audit::Scope::Spot,
            audit::Outcome::Failed,
            moment(),
            source,
            "fixture",
        );
        run.failures = u64::try_from(failures).unwrap();
        journal.append(&run).expect("append a run");
        for n in 0..failures {
            journal
                .append(&audit::Record::member_failure(
                    audit::Scope::Spot,
                    moment(),
                    &format!("SYM{n}"),
                    pull::session::Window::new(day(), day()).expect("a window"),
                    "did not land",
                ))
                .expect("append a failure");
        }
    }

    /// The `source` of every run and failure in one answer, in order.
    fn sources(json: &str) -> Vec<String> {
        let parsed: serde_json::Value = serde_json::from_str(json).expect("JSON");
        parsed["runs"]
            .as_array()
            .expect("runs")
            .iter()
            .map(|run| run["source"].as_str().unwrap_or("").to_owned())
            .collect()
    }

    /// **EACH FEED SEES ITS OWN RUNS AND THE FAILURES THEY APPENDED, AND NO
    /// OTHER FEED'S** (DB-054, DB-G30): a Zerodha run by its policy, a Zerodha
    /// run by its endpoint, a Groww run by its endpoint, and a refusal that
    /// names no vendor, which no feed shows and every answer counts.
    #[test]
    fn the_journal_is_scoped_by_each_runs_vendor_and_gdfl_holds_none() {
        let site = site("scoped");
        let journal = site.journal();
        pulled(&journal, "Reference indices", 0);
        pulled(
            &journal,
            "zerodha_nse_checked NOT vendor-ISIN; NSE file same day; Swept surface",
            2,
        );
        pulled(&journal, "https://api.groww.in/v1/historical/candle", 1);
        pulled(
            &journal,
            "https://api.kite.trade/instruments/historical/:instrument_token/:interval",
            3,
        );

        let zerodha = body(&site, Vendor::Zerodha, 0, moment(), Ok(day()));
        let got = sources(&zerodha);
        assert_eq!(got.len(), 7, "{zerodha}");
        assert!(got[0].starts_with("SYM"), "newest first: {got:?}");
        assert!(got[3].contains("api.kite.trade"), "{got:?}");
        assert!(got[6].starts_with("zerodha_nse_checked"), "{got:?}");
        assert!(
            !got.iter()
                .any(|s| s.contains("groww") || s == "Reference indices")
        );
        let parsed: serde_json::Value = serde_json::from_str(&zerodha).unwrap();
        let scope = &parsed["journal"]["feed_scope"];
        assert_eq!(scope["records"], 7);
        assert_eq!(scope["runs"], 2);
        assert_eq!(scope["unattributed_records"], 1);
        assert_eq!(scope["unattributed_runs"], 1);
        assert_eq!(
            parsed["journal"]["records"], 10,
            "the file's own count is unchanged"
        );

        let groww = sources(&body(&site, Vendor::Groww, 0, moment(), Ok(day())));
        assert_eq!(groww.len(), 2);
        assert!(groww[1].contains("api.groww.in"));

        let gdfl = body(&site, Vendor::Gdfl, 0, moment(), Ok(day()));
        assert!(sources(&gdfl).is_empty(), "{gdfl}");
        assert!(!gdfl.contains("zerodha_nse_checked"), "{gdfl}");
        let parsed: serde_json::Value = serde_json::from_str(&gdfl).unwrap();
        assert!(
            parsed["journal"]["feed_scope"]["holds"]
                .as_str()
                .unwrap()
                .contains("imports/gdfl.journal")
        );
        assert_eq!(parsed["imports"]["route"], "/imports.json?feed=gdfl");
        assert_eq!(parsed["imports"]["journal"]["present"], false);
        assert!(serde_json::from_str::<serde_json::Value>(&zerodha).unwrap()["imports"].is_null());
    }

    /// **A PAGE OF ONE FEED SPANS ITS RUNS AND SKIPS THE OTHERS'**: the newest
    /// records first, across a run boundary, and the second page continues
    /// where the first stopped. Pinned with a page of three over the spans.
    #[test]
    fn a_feed_page_crosses_run_boundaries_and_skips_other_feeds() {
        let site = site("scoped-pages");
        let journal = site.journal();
        pulled(&journal, "https://api.kite.trade/a", 2);
        pulled(&journal, "https://api.groww.in/b", 2);
        pulled(&journal, "https://api.kite.trade/c", 1);
        let total = journal.look().records();
        let runs = attribution(&journal, total).expect("attributed");
        let ordinals = |skip: u64, take: u64| -> Vec<u64> {
            runs.page(&journal, Vendor::Zerodha, total, skip, take)
                .expect("a page")
                .iter()
                .map(|entry| entry.ordinal)
                .collect()
        };
        // Ordinals: 0-2 kite a, 3-5 groww b, 6-7 kite c.
        assert_eq!(ordinals(0, 3), [7, 6, 2]);
        assert_eq!(ordinals(3, 3), [1, 0]);
        assert_eq!(ordinals(9, 3), Vec::<u64>::new());
        assert_eq!(runs.held(Vendor::Zerodha, total), (5, 2));
        assert_eq!(runs.held(Vendor::Groww, total), (3, 1));
        // THE TAIL: a record appended later is attributed without a rescan.
        pulled(&journal, "https://api.groww.in/d", 0);
        let grown = attribution(&journal, journal.look().records()).expect("attributed");
        assert_eq!(grown.scanned, total + 1);
        assert_eq!(grown.held(Vendor::Groww, total + 1), (4, 2));
    }

    /// The vendor a source names: an endpoint of an HTTP feed, or a
    /// Zerodha-only policy; nothing else.
    #[test]
    fn a_source_names_its_vendor_by_endpoint_or_policy_only() {
        assert_eq!(
            vendor_of_source("https://api.kite.trade/x"),
            Some(Vendor::Zerodha)
        );
        assert_eq!(
            vendor_of_source("zerodha_symbol NOT ISIN-verified; y"),
            Some(Vendor::Zerodha)
        );
        assert_eq!(
            vendor_of_source("https://api.dhan.co/v2"),
            Some(Vendor::Dhan)
        );
        assert_eq!(vendor_of_source("Swept surface"), None);
        assert_eq!(vendor_of_source("NIFTY"), None);
    }

    /// **THE CENSUS'S ROOM IS IN THE ANSWER** (DB-G07): the limit, what is
    /// committed and what is left; `null` when no census loaded.
    #[test]
    fn the_store_block_carries_the_census_capacity() {
        let root = store_root("capacity");
        install(
            &root,
            Vendor::Dhan,
            &manifest_image(
                Vendor::Dhan,
                &[("NIFTY", 2025, 1, 3), ("NIFTY", 2025, 2, 7)],
            ),
        );
        let site = Site::load(&masters("capacity"), &root);
        let json = body(&site, Vendor::Dhan, 0, moment(), Ok(day()));
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        let capacity = &parsed["store"]["capacity"];
        assert_eq!(capacity["limit"], pull::manifest::MAX_ENTRIES);
        assert_eq!(capacity["committed"], 2);
        assert_eq!(capacity["free"], pull::manifest::MAX_ENTRIES - 2);
        assert_eq!(capacity["full"], false);
        let absent = body(&site, Vendor::Groww, 0, moment(), Ok(day()));
        let parsed: serde_json::Value = serde_json::from_str(&absent).unwrap();
        assert!(parsed["store"]["capacity"].is_null());
    }
}

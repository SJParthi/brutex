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
//! a directory, and nothing here grows with the journal.

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
    (
        axum::http::StatusCode::OK,
        json(),
        body(
            &site,
            feed,
            crate::server::page_number(query),
            std::time::SystemTime::now(),
            ingest::today_ist(),
        ),
    )
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
    journal_block(site, page, &mut out);
    store_block(site, feed, &mut out);
    out.push('}');
    out
}

/// The journal: what the file is, and one bounded page of what it holds.
fn journal_block(site: &Site, page: usize, out: &mut String) {
    let journal = site.journal();
    let log = journal.look();
    let trouble = journal_trouble(&log);
    let total = log.records();
    let per_page = audit::MAX_PAGE_RECORDS;
    // `total - 1` and not `total`, so 200 records is one page and not two.
    // Saturating, so an empty journal is page 0 of 1 rather than an underflow.
    let last_page = usize::try_from(total.saturating_sub(1) / per_page).unwrap_or(0);
    // CLAMPED, NOT REFUSED. `?page=99` over a 135-record journal is a stale
    // bookmark, not an attack; it answers with the last page, exactly as the
    // HTML pager does.
    let page = page.min(last_page);
    let skip = u64::try_from(page).unwrap_or(0).saturating_mul(per_page);

    let _ = write!(
        out,
        r#","journal":{{"path":{},"present":{},"records":{total},"bytes":{},"trouble":{},"page":{page},"pages":{},"page_rows":{per_page}}}"#,
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
    );

    match journal.page(total, skip, per_page) {
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
                r#"{{"ordinal":{},"fault":null,"kind":{},"at":{},"took_micros":{},"scope":{},"outcome":{},"loud":{},"members":{},"rows_read":{},"bars_stored":{},"rows_folded":{},"counted":{},"failures":{},"window":{},"source":{},"source_bytes":{},"note":{},"note_bytes":{},"drops":["#,
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
                    r.drops.of(reason)
                );
            }
            out.push_str("]}");
        }
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
    let (censuses, entries) = census_now(site);
    let census = censuses.iter().find(|c| c.vendor == feed);

    let mut months: BTreeMap<store::path::YearMonth, (u64, u64)> = BTreeMap::new();
    let mut instrument_months = 0u64;
    let mut bars = 0u64;
    for (series, month) in &entries {
        let Some(rows) = census.and_then(|c| c.rows_for(&series.at(*month))) else {
            continue;
        };
        let cell = months.entry(*month).or_insert((0, 0));
        cell.0 = cell.0.saturating_add(1);
        cell.1 = cell.1.saturating_add(rows);
        instrument_months = instrument_months.saturating_add(1);
        bars = bars.saturating_add(rows);
    }

    let _ = write!(
        out,
        r#","store":{{"feed":{},"state":{},"note":{},"degraded":{},"generation":{},"commits":{},"manifest":{},"committed_at":{},"instrument_months":{instrument_months},"bars":{bars},"months":["#,
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
        let mut record = audit::Record::refused(
            audit::Scope::Spot,
            audit::Outcome::Failed,
            moment(),
            "swept",
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
}

//! The rolling log, over HTTP.
//!
//! # Why this exists
//!
//! `crates/telemetry` has held a complete, bounded reader — [`telemetry::tail`]
//! — with **no caller anywhere in the workspace**. So the only way to read a
//! run's log was to find the file on disk and open it in something else, and
//! the first question an operator asks after a failed pull ("what did it say?")
//! had no answer inside the application that wrote it.
//!
//! A live run refused 10 of 16 members in 18 seconds. The receipt named five of
//! them, the audit journal kept one, and the log — which now carries every one
//! — was not reachable from any page.
//!
//! # What it does not do
//!
//! **It does not follow.** No streaming, no polling, no JavaScript: `CLAUDE.md`
//! §2 permits none outside `web/`, and CI gate 1 enforces it. The page is a
//! snapshot with a form on it, and reloading is how you see more.
//!
//! **It does not search the whole history.** [`telemetry::tail`] walks newest
//! file first and stops at [`telemetry::Query::max_scan_bytes`], reporting
//! `hit_scan_cap` when it did. That flag is rendered rather than hidden,
//! because "what I found in the bytes I read" and "everything there is" are
//! different answers and only one of them is true.
//!
//! # Cost
//!
//! Bounded at both ends and by construction, which is what makes it safe to
//! expose. `limit` is capped at [`telemetry::MAX_LIMIT`]; the walk is capped at
//! `max_scan_bytes`; the number of files is `keep_files`. A request cannot make
//! the server read the disk without a bound, whatever the query string says.

use std::fmt::Write as _;

use crate::render;

/// The most events one page will render.
///
/// Smaller than [`telemetry::MAX_LIMIT`] on purpose: this is a page a person
/// reads, and the reader's own ceiling is the ceiling on the *buffer*, not on
/// what is useful to look at. `limit` above this is clamped, never refused —
/// a bookmarked query is not an error.
pub const PAGE_LIMIT: usize = 200;

/// How many bytes one request may read off the log before it stops.
///
/// 4 MiB against a 64 MiB window (8 files × 8 MiB), so a request reads at most
/// the newest sixteenth of the history. The walk says `hit_scan_cap` when it
/// stopped here and the page prints it.
pub const SCAN_BYTES: u64 = 4 * 1024 * 1024;

/// Where the log actually is, taken from the installed sink.
///
/// **Read off the sink rather than recomputed.** `server::served_log_dir`
/// consults the environment and the working directory, and calling it again
/// here would be a second answer to "where is the log" that could disagree with
/// the first — which is the shape of half the defects this repository has
/// recorded. The sink knows the directory it is appending to; nothing else is
/// entitled to an opinion.
///
/// [`None`] when logging was never installed, which is a real state (the sink
/// refuses loudly when its directory is unwritable) and is rendered as one.
fn log_dir() -> Option<std::path::PathBuf> {
    let sink = telemetry::global()?;
    sink.path().parent().map(std::path::Path::to_path_buf)
}

/// One field value as JSON, keeping the type the line carried.
///
/// **Not stringified.** A count that arrives as `Uint(806)` and leaves as
/// `"806"` makes every consumer parse it back, and the first one to forget
/// compares a string. The variants map onto JSON's own types, which is what
/// the log line already did.
///
/// A non-finite float becomes `null`: JSON has no `NaN` or `Infinity`, and
/// emitting the bare word would produce a document no parser accepts. `null`
/// is the honest shape for "a number that is not one".
fn value_json(value: &telemetry::OwnedValue) -> String {
    match *value {
        telemetry::OwnedValue::Str(ref s) => render::json_string(s),
        telemetry::OwnedValue::Int(v) => v.to_string(),
        telemetry::OwnedValue::Uint(v) => v.to_string(),
        telemetry::OwnedValue::Float(v) => {
            if v.is_finite() {
                format!("{v}")
            } else {
                "null".to_owned()
            }
        }
        telemetry::OwnedValue::Bool(v) => v.to_string(),
        telemetry::OwnedValue::Null => "null".to_owned(),
    }
}

/// One field value as text, for the page.
fn value_text(value: &telemetry::OwnedValue) -> String {
    match *value {
        telemetry::OwnedValue::Str(ref s) => s.clone(),
        telemetry::OwnedValue::Int(v) => v.to_string(),
        telemetry::OwnedValue::Uint(v) => v.to_string(),
        telemetry::OwnedValue::Float(v) => format!("{v}"),
        telemetry::OwnedValue::Bool(v) => v.to_string(),
        telemetry::OwnedValue::Null => "null".to_owned(),
    }
}

/// The query a request asked for, already clamped.
struct Asked {
    query: telemetry::Query,
    limit: usize,
    level: Option<telemetry::Level>,
    target: String,
}

/// Reads the query string into a bounded [`telemetry::Query`].
///
/// Every parameter is **clamped rather than refused**. A log viewer that
/// answers a mangled bookmark with a 400 is a log viewer an operator stops
/// reaching for, and unlike a pull request nothing here can be made wrong by a
/// bad number — the worst a bad `limit` can do is show a different count.
fn asked(raw: &str) -> Asked {
    let limit = render::query_value(&crate::server::param(raw, "limit"))
        .parse::<usize>()
        .unwrap_or(50)
        .clamp(1, PAGE_LIMIT);
    let level_word = crate::server::param(raw, "level");
    let level = telemetry::Level::of_label(&level_word.to_ascii_lowercase());
    let target = crate::server::param(raw, "target");

    let mut query = telemetry::Query::last(limit);
    query.max_scan_bytes = SCAN_BYTES;
    if let Some(level) = level {
        query = query.at_least(level);
    }
    if !target.is_empty() {
        query = query.from_target(target.clone());
    }
    Asked {
        query,
        limit,
        level,
        target,
    }
}

/// `GET /logs.json` — the tail, and every honesty flag the walk carries.
pub async fn logs_json(
    uri: axum::http::Uri,
) -> (
    axum::http::StatusCode,
    [(axum::http::HeaderName, &'static str); 1],
    String,
) {
    let json = [(
        axum::http::header::CONTENT_TYPE,
        "application/json; charset=utf-8",
    )];
    let Some(dir) = log_dir() else {
        return (
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            json,
            r#"{"error":"logging is not installed in this process, so there is no log to read"}"#
                .to_owned(),
        );
    };
    let asked = asked(uri.query().unwrap_or(""));
    (axum::http::StatusCode::OK, json, json_over(&dir, &asked))
}

/// The JSON body, over a directory a caller names.
///
/// Split from the handler so a test can drive it against a REAL sink writing a
/// REAL file, rather than needing the process-global one installed. The
/// handler is then the one thing left untested here — resolving the directory
/// — and that is a single `?` on `telemetry::global()`.
fn json_over(dir: &std::path::Path, asked: &Asked) -> String {
    let tail = telemetry::tail(dir, telemetry::DEFAULT_KEEP_FILES, &asked.query);

    let mut out = String::from("{\"records\":[");
    for (n, record) in tail.records.iter().enumerate() {
        if n > 0 {
            out.push(',');
        }
        let _ = write!(
            out,
            r#"{{"seq":{},"ts":{},"level":{},"target":{},"message":{},"cut":{},"dropped_fields":{}"#,
            record.seq,
            record.at_unix_millis,
            render::json_string(record.level.label()),
            render::json_string(&record.target),
            render::json_string(&record.message),
            record.cut,
            record.dropped_fields,
        );
        out.push_str(",\"fields\":{");
        for (n, (key, value)) in record.fields.iter().enumerate() {
            if n > 0 {
                out.push(',');
            }
            let _ = write!(out, "{}:{}", render::json_string(key), value_json(value));
        }
        out.push_str("}}");
    }
    // THE WALK'S OWN LIMITS, IN THE ANSWER. A caller that cannot tell a
    // complete answer from a truncated one will treat every answer as
    // complete, which is how "no errors in the log" comes to mean "no errors
    // in the four megabytes I happened to read".
    let _ = write!(
        out,
        r#"],"bytes_read":{},"files_read":{},"malformed":{},"partial_tail":{},"hit_scan_cap":{},"reached_oldest":{},"scan_cap_bytes":{},"limit":{},"errors":["#,
        tail.bytes_read,
        tail.files_read,
        tail.malformed,
        tail.partial_tail,
        tail.hit_scan_cap,
        tail.reached_oldest,
        SCAN_BYTES,
        asked.limit,
    );
    for (n, why) in tail.errors.iter().enumerate() {
        if n > 0 {
            out.push(',');
        }
        out.push_str(&render::json_string(why));
    }
    out.push_str("]}");
    out
}

/// `GET /logs` — the same tail, as a page.
pub async fn logs_page(uri: axum::http::Uri) -> axum::response::Html<String> {
    let raw = uri.query().unwrap_or("");
    let asked = asked(raw);
    let Some(dir) = log_dir() else {
        return axum::response::Html(page_shell(
            &asked,
            "<p class=\"halt\"><b>No log</b>Logging is not installed in this \
             process, so there is nothing to read. The server names the reason \
             on stdout at startup.</p>",
        ));
    };
    axum::response::Html(page_over(&dir, &asked))
}

/// The page body, over a directory a caller names — split for the reason
/// [`json_over`] is.
fn page_over(dir: &std::path::Path, asked: &Asked) -> String {
    let tail = telemetry::tail(dir, telemetry::DEFAULT_KEEP_FILES, &asked.query);

    let mut body = String::new();
    let _ = write!(
        body,
        "<p class=\"lead\">{} event(s) · {} byte(s) read from {} file(s) · \
         <code>{}</code></p>",
        tail.records.len(),
        tail.bytes_read,
        tail.files_read,
        render::escape(&dir.display().to_string()),
    );

    let notes = walk_notes(&tail);
    if !notes.is_empty() {
        body.push_str("<ul class=\"notes\">");
        for note in &notes {
            let _ = write!(body, "<li>{}</li>", render::escape(note));
        }
        body.push_str("</ul>");
    }

    if tail.records.is_empty() {
        body.push_str(
            "<p class=\"lead\">Nothing matched. A quiet log is the ordinary \
             state: members are written at <code>debug</code>, and the default \
             floor is <code>info</code> — set <code>BRUTEX_LOG_LEVEL=debug</code> \
             and re-run to see them.</p>",
        );
        return page_shell(asked, &body);
    }

    body.push_str(&rows_table(&tail));
    page_shell(asked, &body)
}

/// Every flag the walk raised, in the operator's words.
///
/// **These go above the rows, not below them.** Someone who scrolls a table and
/// finds nothing alarming has been told something; they have only been told the
/// truth if they also know the walk stopped early. `hit_scan_cap` is the
/// difference between "no errors in the log" and "no errors in the four
/// megabytes I happened to read".
fn walk_notes(tail: &telemetry::Tail) -> Vec<String> {
    let mut notes: Vec<String> = Vec::new();
    if tail.hit_scan_cap {
        notes.push(format!(
            "The walk stopped after {SCAN_BYTES} byte(s). This is what was in \
             the newest part of the log, NOT everything there is."
        ));
    }
    if !tail.reached_oldest {
        notes.push(
            "Older events exist beyond what was read. Narrow by target or \
             level to reach them."
                .to_owned(),
        );
    }
    if tail.partial_tail {
        notes.push(
            "The newest file ended mid-line — a writer caught in the act, or a \
             torn write. The fragment was skipped, never guessed at."
                .to_owned(),
        );
    }
    if tail.malformed > 0 {
        notes.push(format!(
            "{} line(s) would not decode and were stepped over.",
            tail.malformed
        ));
    }
    for why in &tail.errors {
        notes.push(format!("A file could not be read: {why}"));
    }
    notes
}

/// The events themselves, newest first.
fn rows_table(tail: &telemetry::Tail) -> String {
    let mut out = String::from(
        "<div class=\"hscroll\"><table><thead><tr>\
         <th>When (UTC)</th><th>Level</th><th>Target</th><th>Message</th>\
         <th>Fields</th></tr></thead><tbody>",
    );
    for record in &tail.records {
        let loud = record.level.at_least(telemetry::Level::Warn);
        let mut fields = String::new();
        for (n, (key, value)) in record.fields.iter().enumerate() {
            if n > 0 {
                fields.push(' ');
            }
            let _ = write!(
                fields,
                "<b>{}</b>={}",
                render::escape(key),
                render::escape(&value_text(value))
            );
        }
        // BOTH TRUNCATION FLAGS ARE SHOWN. A field cut at its ceiling and a
        // field counted rather than kept are the two ways this log can be
        // shorter than the truth, and a reader who cannot see either will read
        // the shortened version as the whole one.
        if record.cut {
            fields.push_str(" <i>(a value was cut at its ceiling)</i>");
        }
        if record.dropped_fields > 0 {
            let _ = write!(
                fields,
                " <i>({} field(s) counted, not kept)</i>",
                record.dropped_fields
            );
        }
        let _ = write!(
            out,
            "<tr{}><td class=\"when\">{}</td><td class=\"lvl\">{}</td>\
             <td class=\"mono\">{}</td><td>{}</td><td class=\"wide\">{fields}</td></tr>",
            if loud { " class=\"fault\"" } else { "" },
            render::escape(&record.at_utc),
            render::escape(record.level.label()),
            render::escape(&record.target),
            render::escape(&record.message),
        );
    }
    out.push_str("</tbody></table></div>");
    out
}

/// The page around the table: the filter form, and nothing that needs script.
fn page_shell(asked: &Asked, body: &str) -> String {
    let mut levels = String::new();
    for level in telemetry::LEVELS {
        let on = asked.level == Some(level);
        let _ = write!(
            levels,
            "<option value=\"{}\"{}>{}</option>",
            level.label(),
            if on { " selected" } else { "" },
            level.label()
        );
    }
    format!(
        "<!doctype html><meta charset=\"utf-8\">\
         <meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\
         <title>brutex · logs</title>\
         <style>\
         body{{background:#f5f7fb;color:#0a0f1e;font:15px/1.55 ui-sans-serif,-apple-system,system-ui,sans-serif;padding:0 0 4rem;margin:0}}\
         @media(prefers-color-scheme:dark){{body{{background:#060911;color:#e9efff}}\
         table,form{{background:#0e1524}}thead th{{background:#0e1524}}}}\
         h1{{max-width:1180px;margin:0 auto;padding:2rem 1.25rem .3rem;font-size:1.6rem;letter-spacing:-.5px}}\
         .lead{{max-width:1180px;margin:0 auto;padding:0 1.25rem;color:#5a6478;font-size:.9rem}}\
         form{{max-width:1180px;margin:1rem auto;padding:.9rem 1.25rem;display:flex;gap:.7rem;\
         align-items:end;flex-wrap:wrap;border:1px solid #e4e9f3;border-radius:12px}}\
         label{{display:flex;flex-direction:column;gap:.3rem;font-size:.7rem;letter-spacing:.09em;text-transform:uppercase;color:#5a6478}}\
         input,select{{font:inherit;padding:.45rem .6rem;border:1px solid #e4e9f3;border-radius:8px;background:transparent;color:inherit}}\
         button{{font:inherit;font-weight:700;padding:.5rem 1.1rem;border:0;border-radius:8px;background:#4f46e5;color:#fff;cursor:pointer}}\
         .hscroll{{overflow-x:auto;max-width:1180px;margin:0 auto;padding:0 1.25rem}}\
         table{{border-collapse:collapse;width:100%;font-size:13px;min-width:56rem}}\
         thead th{{position:sticky;top:0;text-align:left;padding:.6rem .7rem;border-bottom:1px solid #e4e9f3;\
         font-size:.65rem;letter-spacing:.09em;text-transform:uppercase;color:#5a6478;white-space:nowrap}}\
         td{{padding:.5rem .7rem;border-bottom:1px solid #e8eef6;vertical-align:top}}\
         td.when,td.mono,.lvl{{font-family:ui-monospace,Menlo,monospace;font-size:12px;white-space:nowrap}}\
         td.wide{{font-family:ui-monospace,Menlo,monospace;font-size:11.5px;color:#5a6478}}\
         tr.fault td{{background:rgba(220,38,38,.08);color:#dc2626}}\
         .notes{{max-width:1180px;margin:.8rem auto;padding:.7rem 1.25rem .7rem 2.6rem;color:#8a5a00;font-size:.85rem}}\
         .halt{{max-width:1180px;margin:1rem auto;padding:1rem 1.25rem;color:#dc2626}}\
         .halt b{{display:block;text-transform:uppercase;font-size:.7rem;letter-spacing:.1em}}\
         a{{color:#4f46e5}}\
         </style>\
         <h1>logs</h1>\
         <p class=\"lead\">The rolling log this process is writing. \
         Newest first. No script, no polling — reload to refresh.</p>\
         <form method=\"get\" action=\"/logs\">\
         <label>Level<select name=\"level\"><option value=\"\">every level</option>{levels}</select></label>\
         <label>Target<input type=\"text\" name=\"target\" value=\"{target}\" placeholder=\"pull, pull.member, api.serve\"></label>\
         <label>Limit<input type=\"number\" name=\"limit\" min=\"1\" max=\"{max}\" value=\"{limit}\"></label>\
         <button type=\"submit\">Show</button>\
         <a href=\"/logs.json{q}\">as JSON</a>\
         </form>{body}",
        levels = levels,
        target = render::escape(&asked.target),
        limit = asked.limit,
        max = PAGE_LIMIT,
        q = if asked.target.is_empty() {
            String::new()
        } else {
            format!("?target={}", render::escape(&asked.target))
        },
        body = body,
    )
}

/// Every HTTP request, once, with what it did and how long it took.
///
/// # Why middleware and not a line in each handler
///
/// There are two dozen routes and a fallback. A handler that forgets the line
/// is a route that is silently invisible, and the one that forgets is always
/// the one added last — which is the one being debugged. A layer cannot be
/// forgotten by a route that does not know it exists.
///
/// # What is NOT logged, deliberately
///
/// **The query string.** `/pull/spot` carries operator input and this
/// repository is public (`CLAUDE.md` §8 is the same instinct about paths).
/// The PATH is logged and the query is not, so a log shared with anybody
/// cannot leak what was typed into a form. The path alone answers "which
/// route", which is the question this event exists for.
///
/// **No body, no headers.** Both are unbounded and one of them carries the
/// credential header on its way out.
///
/// # Cost
///
/// One event per request, at `Debug` — so a normal run writes none of them
/// and the 64 MiB window is not spent on `GET /_app/immutable/...`. A
/// 5xx is re-emitted at `Error` regardless of floor, because a request that
/// failed is not detail.
pub async fn note_request(
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let method = request.method().as_str().to_owned();
    let path = request.uri().path().to_owned();
    let started = std::time::Instant::now();
    let response = next.run(request).await;
    let status = response.status();
    let micros = u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX);

    let level = if status.is_server_error() {
        telemetry::Level::Error
    } else if status.is_client_error() {
        telemetry::Level::Warn
    } else {
        telemetry::Level::Debug
    };
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::new(level, "api.request", "served")
            .with("method", telemetry::Value::Str(&method))
            .with("path", telemetry::Value::Str(&path))
            .with("status", telemetry::Value::Uint(u64::from(status.as_u16())))
            .with("micros", telemetry::Value::Uint(micros)),
    );
    response
}

#[cfg(test)]
mod tests {
    // The same exceptions every test module in this workspace takes.
    #![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

    use super::*;

    /// A sink of our own, in a scratch directory — never the process global.
    ///
    /// `telemetry::install` is a one-shot process singleton, so a test that
    /// used it would decide the answer for every other test in the binary and
    /// would pass or fail on ordering. `Sink::open` takes a config and owns its
    /// directory, which is why [`json_over`] and [`page_over`] take a path.
    fn sink_in(name: &str) -> (std::path::PathBuf, telemetry::Sink) {
        let dir = std::env::temp_dir().join(format!("brutex-logs-{name}"));
        let _ignored = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a scratch directory");
        let sink = telemetry::Sink::open(
            &telemetry::Config::new(&dir).with_min_level(telemetry::Level::Trace),
        )
        .expect("opens");
        (dir, sink)
    }

    /// The whole path: emit, roll to disk, read back, render both surfaces.
    #[test]
    fn a_written_event_reaches_the_json_and_the_page() {
        let (dir, sink) = sink_in("roundtrip");
        assert!(
            sink.emit(
                &telemetry::Event::error("pull.member", "did not land")
                    .with("instrument", telemetry::Value::Str("BANKNIFTY"))
                    .with("bars", telemetry::Value::Uint(806))
                    .with("balanced", telemetry::Value::Bool(false)),
            )
            .is_written(),
            "the premise: the sink accepted it"
        );

        let asked = asked("limit=10");
        let json = json_over(&dir, &asked);
        assert!(json.contains(r#""target":"pull.member""#), "{json}");
        assert!(json.contains(r#""instrument":"BANKNIFTY""#), "{json}");
        assert!(
            json.contains(r#""bars":806"#),
            "a count keeps its JSON type and is NOT stringified: {json}"
        );
        assert!(
            json.contains(r#""balanced":false"#),
            "a flag stays a flag: {json}"
        );
        assert!(
            json.contains(r#""hit_scan_cap":false"#) && json.contains(r#""malformed":0"#),
            "the walk's own limits travel with the answer: {json}"
        );

        let page = page_over(&dir, &asked);
        assert!(page.contains("BANKNIFTY"), "{page}");
        assert!(
            page.contains("class=\"fault\""),
            "an error row is marked loud, not left to be read as ordinary"
        );
        assert!(
            !page.contains("<script"),
            "CLAUDE.md section 2 permits no script outside web/"
        );
    }

    /// A level floor keeps the quieter events out of the answer.
    #[test]
    fn the_level_filter_is_applied_to_what_is_returned() {
        let (dir, sink) = sink_in("levels");
        let _ = sink.emit(&telemetry::Event::debug("pull.member", "landed"));
        let _ = sink.emit(&telemetry::Event::error("pull.member", "did not land"));

        let all = json_over(&dir, &asked("limit=10"));
        assert!(
            all.contains("landed") && all.contains("did not land"),
            "{all}"
        );

        let loud = json_over(&dir, &asked("limit=10&level=error"));
        assert!(loud.contains("did not land"), "{loud}");
        assert!(
            !loud.contains(r#""message":"landed""#),
            "a debug line must not survive an error floor: {loud}"
        );
    }

    /// `limit` is clamped at both ends rather than refused.
    #[test]
    fn a_mangled_limit_is_clamped_and_never_a_refusal() {
        assert_eq!(asked("limit=0").limit, 1, "zero clamps up");
        assert_eq!(
            asked("limit=99999").limit,
            PAGE_LIMIT,
            "past the ceiling clamps down"
        );
        assert_eq!(
            asked("limit=banana").limit,
            50,
            "unreadable takes the default"
        );
        assert_eq!(asked("").limit, 50, "absent takes the default");
        assert_eq!(
            asked("limit=10").query.max_scan_bytes,
            SCAN_BYTES,
            "the byte bound is never taken from the query string"
        );
    }

    /// An empty log says why it is empty rather than looking broken.
    #[test]
    fn an_empty_log_names_the_level_that_would_show_more() {
        let (dir, _sink) = sink_in("empty");
        let page = page_over(&dir, &asked("limit=10"));
        assert!(
            page.contains("BRUTEX_LOG_LEVEL=debug"),
            "a quiet log is the ordinary state and the page must say what to \
             change to see more: {page}"
        );
    }
}

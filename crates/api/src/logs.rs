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
    /// The run the reader narrowed to, or zero for every run.
    run: u64,
}

/// Reads the query string into a bounded [`telemetry::Query`].
///
/// Every parameter is **clamped rather than refused**. A log viewer that
/// answers a mangled bookmark with a 400 is a log viewer an operator stops
/// reaching for, and unlike a pull request nothing here can be made wrong by a
/// bad number — the worst a bad `limit` can do is show a different count.
fn asked(raw: &str) -> Asked {
    // NOT THROUGH `render::query_value`. That is a percent-ENCODER, and
    // `param` has already decoded; encoding a decoded value and then parsing it
    // as a number is backwards. It happened to work for a plain integer — every
    // digit is in the unreserved set — and would have turned any other input
    // into a `%XX` soup that parses to the default, which is the same answer for
    // the wrong reason.
    let limit = crate::server::param(raw, "limit")
        .parse::<usize>()
        .unwrap_or(50)
        .clamp(1, PAGE_LIMIT);
    let level_word = crate::server::param(raw, "level");
    let level = telemetry::Level::of_label(&level_word.to_ascii_lowercase());
    let target = crate::server::param(raw, "target");
    // ZERO IS "EVERY RUN", not run zero. An event outside a backfill omits the
    // key entirely, so there is no run zero to ask for and the value is free to
    // mean the absence of a filter.
    let run = crate::server::param(raw, "run").parse::<u64>().unwrap_or(0);

    let mut query = telemetry::Query::last(limit);
    query.max_scan_bytes = SCAN_BYTES;
    if let Some(level) = level {
        query = query.at_least(level);
    }
    if !target.is_empty() {
        query = query.from_target(target.clone());
    }
    if run != 0 {
        query = query.from_run(run);
    }
    Asked {
        query,
        limit,
        level,
        target,
        run,
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
    (
        axum::http::StatusCode::OK,
        json,
        json_over(
            &dir,
            crate::server::store_dir()
                .ok()
                .map(|s| cli_half(&s))
                .as_deref(),
            &asked,
            sink_health().as_ref(),
        ),
    )
}

/// The JSON body, over a directory a caller names.
///
/// Split from the handler so a test can drive it against a REAL sink writing a
/// REAL file, rather than needing the process-global one installed. The
/// handler is then the one thing left untested here — resolving the directory
/// — and that is a single `?` on `telemetry::global()`.
fn json_over(
    dir: &std::path::Path,
    cli_dir: Option<&std::path::Path>,
    asked: &Asked,
    health: Option<&telemetry::Health>,
) -> String {
    let tail = both_halves(dir, cli_dir, &asked.query);
    json_of(&tail, asked, health, (dir, cli_dir))
}

/// Where `crates/cli` ACTUALLY writes, resolved from the STORE root.
///
/// # The join this replaces was one level off, and it hid 485 events
///
/// [`both_halves`] used to build its second half as `dir.join(CLI_SUBDIR)`,
/// where `dir` is the directory the SERVER's own sink resolved. Under the run
/// configuration this repository ships — `.claude/launch.json` starts `api`
/// from the workspace root — `server::log_dir_from` answers `<workspace>/logs`,
/// so the reader looked in `<workspace>/logs/cli`. That directory does not
/// exist and never has.
///
/// `cli::log_dir_from` resolves `<store>/logs/cli`. Measured on this machine:
/// `~/.brutex/store/logs/cli/events.ndjson` held **485 `cli.*` records** —
/// 392 `ladder level`, 42 `stored span loaded`, 32 `rung sweeping`, 12
/// `threshold search over stored bars`, 7 `exit grid entered` — while the
/// directory `/logs` was reading held **zero**.
///
/// # Why it was silent, and why that is the worse half
///
/// `telemetry::tail` is documented never to fail and to render a missing
/// directory as no records. That is the right behaviour for a store where
/// nobody has run `cli`, and it is exactly what made this invisible: the page
/// answered "no `cli` events" in the same words for "none were written" and
/// "I looked in the wrong place". `CLAUDE.md` §4 bans a fallback that hides a
/// failure, and an absent-directory default that cannot tell those two apart is
/// one wearing a legitimate default's clothes.
///
/// The two halves are resolved from DIFFERENT roots now, deliberately: the
/// server's from its own sink, the `cli` half from the store. Deriving one from
/// the other is what made them able to disagree.
fn cli_half(store: &std::path::Path) -> std::path::PathBuf {
    store.join("logs").join(CLI_SUBDIR)
}

/// Where a CLI-run command writes its records, for a caller outside this module.
///
/// `cli_half` needs the store root and `log_dir` returns the SERVED half, so
/// every caller wanting the other half repeated the same two lines. Two of them
/// already existed here (`logs_json` and `logs_page`); `sweeprun::run_json` is
/// the third, and it is in another module.
///
/// Returns `None` for the same reason those two do -- the store may not be
/// configured -- and never creates: a reader that creates a directory reports a
/// sweep's log as present when nothing has ever written one.
#[must_use]
pub fn cli_log_dir() -> Option<std::path::PathBuf> {
    crate::server::store_dir().ok().map(|s| cli_half(&s))
}

/// One log field, JSON-escaped, for a caller building a response by hand.
///
/// A log MESSAGE is the one string in this crate that an operator's own data
/// reaches: a refusal quotes a vendor's text, a path, a store error. Pasting it
/// into a response with `format!` and no escape is how a stray quote turns a
/// JSON body into a parse error the page reports as "sweep unknown".
#[must_use]
pub fn quoted(raw: &str) -> String {
    crate::render::json_string(raw)
}

/// The subdirectory `cli` appends to, beside the server's own.
///
/// `crates/cli`'s `log_dir` resolves `<store>/logs/cli` and its banner has said
/// so on every run: *"The /logs page reads whatever directory `api` resolved,
/// which is NOT this one."* Its doc names the fix in the same breath — *"until
/// it walks `logs/` and `logs/cli/` the page shows the server's half"* — and
/// this is that walk.
const CLI_SUBDIR: &str = "cli";

/// Both halves of the log, merged newest-first.
///
/// # The half of the audit trail the dashboard could not see
///
/// `telemetry::Config` takes a DIRECTORY and `sink::BASENAME` is a constant, so
/// two live processes appending to one directory would interleave lines in one
/// file — a corrupted record of the one thing that exists to say what happened.
/// `cli` therefore owns `<store>/logs/cli` and the server owns `<store>/logs`.
/// That split is correct and is not changed here.
///
/// What was missing is the READER. `/logs` walked the server's directory alone,
/// so every event a terminal-run sweep wrote — every span loaded, every derived
/// threshold, every month found or missing — was on disk and invisible on the
/// page that exists to show it. `CLAUDE.md`'s own requirement is that if it
/// runs, it must be auditable; a log written where nothing reads it meets the
/// letter and not the rule.
///
/// # Merged on TIME, never on sequence
///
/// Each sink numbers its own events from its own run, so `seq` is unique within
/// a directory and meaningless across two. Ordering the union by `seq` would
/// interleave a `cli` event from this morning with a server event from last
/// week wherever the counters happened to collide. `at_unix_millis` is the one
/// field both sets measure against the same clock.
///
/// The tie-break is `seq` descending, so two events stamped in the same
/// millisecond keep a stable order rather than one the sort chose — §3 rule 5
/// applies to a page's row order as much as to a total.
///
/// # `missing` is SUMMED, and that is the honest reading
///
/// Each half computes its own count of events its own sequence says are gone.
/// Those are two independent facts about two independent files, and their sum
/// is "events this page cannot show you that the sinks say existed". `None`
/// from a half means the question is unanswerable there — under a filter, or
/// with no directory — and a half that cannot answer contributes nothing rather
/// than a zero, because zero would read as "none lost".
fn both_halves(
    dir: &std::path::Path,
    cli_dir: Option<&std::path::Path>,
    query: &telemetry::Query,
) -> telemetry::Tail {
    let served = telemetry::tail(dir, telemetry::DEFAULT_KEEP_FILES, query);
    // AN ABSENT DIRECTORY IS AN EMPTY ANSWER, not an error: `telemetry::tail`
    // is documented never to fail and to render a missing directory as no
    // records. A store where nobody has run `cli` simply has no second half.
    let ran = cli_dir.map_or_else(telemetry::Tail::default, |d| {
        telemetry::tail(d, telemetry::DEFAULT_KEEP_FILES, query)
    });

    let mut records = served.records;
    records.extend(ran.records);
    records.sort_by(|left, right| {
        right
            .at_unix_millis
            .cmp(&left.at_unix_millis)
            .then_with(|| right.seq.cmp(&left.seq))
    });
    // THE LIMIT APPLIES TO THE UNION. Each half already honoured it, so without
    // this the page would return up to twice what was asked for -- and the
    // caller's `limit` is what bounds the response, not a suggestion.
    records.truncate(query.limit);

    telemetry::Tail {
        records,
        bytes_read: served.bytes_read.saturating_add(ran.bytes_read),
        files_read: served.files_read.saturating_add(ran.files_read),
        malformed: served.malformed.saturating_add(ran.malformed),
        // EITHER HALF HITTING ITS CAP CAPS THE ANSWER. A page that said `false`
        // because the server's half fitted, while the `cli` half stopped short,
        // would promise completeness it does not have.
        hit_scan_cap: served.hit_scan_cap || ran.hit_scan_cap,
        // AND ONLY BOTH REACHING THE OLDEST MEANS THE OLDEST WAS REACHED.
        // `walked` initialises this `true` and clears it when it finds an older
        // file still holding bytes, so a directory that does not exist -- a
        // store where nobody has run `cli` -- answers `true` and cannot drag
        // the merged answer down. Checked rather than assumed: the same field
        // was once `false` on ordinary full pages, which is the defect its own
        // doc records.
        reached_oldest: served.reached_oldest && ran.reached_oldest,
        partial_tail: served.partial_tail || ran.partial_tail,
        errors: {
            let mut errors = served.errors;
            errors.extend(ran.errors);
            errors
        },
        missing: match (served.missing, ran.missing) {
            (Some(left), Some(right)) => Some(left.saturating_add(right)),
            (Some(only), None) | (None, Some(only)) => Some(only),
            (None, None) => None,
        },
    }
}

/// [`json_over`] over a walk a caller already has.
///
/// **Split so a test can state a walk it cannot cause.** `hit_scan_cap` needs
/// four megabytes of log, `partial_tail` needs a writer interrupted mid-line,
/// and `errors` needs an unreadable file — none of them reachable from a unit
/// test, so every branch that reports one would otherwise render for the first
/// time in production. `Tail`'s fields are `pub`; the state can be stated.
fn json_of(
    tail: &telemetry::Tail,
    asked: &Asked,
    health: Option<&telemetry::Health>,
    read: (&std::path::Path, Option<&std::path::Path>),
) -> String {
    let mut out = String::from("{\"records\":[");
    for (n, record) in tail.records.iter().enumerate() {
        if n > 0 {
            out.push(',');
        }
        let _ = write!(
            out,
            r#"{{"seq":{},"run":{},"ts":{},"level":{},"target":{},"message":{},"cut":{},"dropped_fields":{}"#,
            record.seq,
            record.run,
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
        r#"],"served_dir":{},"cli_dir":{},"bytes_read":{},"files_read":{},"malformed":{},"partial_tail":{},"hit_scan_cap":{},"reached_oldest":{},"scan_cap_bytes":{},"limit":{},"missing":{},"errors":["#,
        render::json_string(&read.0.display().to_string()),
        read.1.map_or_else(
            || String::from("null"),
            |p| render::json_string(&p.display().to_string()),
        ),
        tail.bytes_read,
        tail.files_read,
        tail.malformed,
        tail.partial_tail,
        tail.hit_scan_cap,
        tail.reached_oldest,
        SCAN_BYTES,
        asked.limit,
        // `null` under a filter, because a filter skips records on purpose and
        // a gap then means nothing. A NUMBER is an exact count of events the
        // sink numbered and this file does not hold.
        tail.missing
            .map_or_else(|| "null".to_owned(), |n| n.to_string()),
    );
    for (n, why) in tail.errors.iter().enumerate() {
        if n > 0 {
            out.push(',');
        }
        out.push_str(&render::json_string(why));
    }
    out.push_str("],\"sink\":");
    out.push_str(&sink_json(health));
    out.push('}');
    out
}

/// The WRITE side of the log, as JSON. `null` when this process has no sink.
fn sink_json(health: Option<&telemetry::Health>) -> String {
    let Some(h) = health else {
        return "null".to_owned();
    };
    let mut out = String::new();
    let _ = write!(
        out,
        r#"{{"written":{},"dropped":{},"rotations":{},"rotation_failures":{},"current_bytes":{},"next_seq":{},"loud":{},"last_error":{}}}"#,
        h.written,
        h.dropped,
        h.rotations,
        h.rotation_failures,
        h.current_bytes,
        h.next_seq,
        h.is_loud(),
        h.last_error
            .as_deref()
            .map_or_else(|| "null".to_owned(), render::json_string),
    );
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
    // THE SECOND HALF IS RESOLVED FROM THE STORE, exactly as `logs_json` does
    // it. Bound to a local rather than written inline so the `Option<PathBuf>`
    // it borrows from plainly outlives the call -- and named `cli_dir` rather
    // than `cli`, which is a crate this one depends on.
    let cli_dir = crate::server::store_dir().ok().map(|s| cli_half(&s));
    axum::response::Html(page_over(
        &dir,
        cli_dir.as_deref(),
        &asked,
        sink_health().as_ref(),
    ))
}

/// The page body, over the directories a caller names — split for the reason
/// [`json_over`] is.
///
/// # It read ONE directory, and every `cli` event was invisible on the page
/// that exists to show them
///
/// [`both_halves`] was written for exactly this and was wired only to
/// [`json_over`]. So `/logs.json` merged the server's directory with
/// `<store>/logs/cli` and `/logs` — the surface an operator actually opens —
/// walked the server's alone. MEASURED on 2026-09-01: the page showed nothing
/// newer than 31 August while **66 sweep events sat unread** in
/// `<store>/logs/cli/events.ndjson`, and every `cli` run printed *"The /logs
/// page walks BOTH halves … so these events appear there"* the whole time.
///
/// **The test that guarded it asserted on the wrong function.**
/// `the_page_shows_the_cli_half_and_not_only_the_servers` is named for the page
/// and drove `json_over`, which never had the defect. That is how this survived
/// being written down twice; the test drives [`page_over`] now.
fn page_over(
    dir: &std::path::Path,
    cli_dir: Option<&std::path::Path>,
    asked: &Asked,
    health: Option<&telemetry::Health>,
) -> String {
    let tail = both_halves(dir, cli_dir, &asked.query);
    page_of(&tail, asked, health, (dir, cli_dir))
}

/// [`page_over`] over a walk a caller already has — split for [`json_of`]'s
/// reason, and the two must stay in step: a flag reported by one surface and
/// not the other is a reader who gets a different answer depending on which URL
/// they happened to open.
///
/// `read` is the pair of directories the walk covered, in [`json_of`]'s order
/// and for its reason: a page that names one of two directories invites the
/// reader to conclude the other was empty.
fn page_of(
    tail: &telemetry::Tail,
    asked: &Asked,
    health: Option<&telemetry::Health>,
    read: (&std::path::Path, Option<&std::path::Path>),
) -> String {
    let mut body = String::new();
    let _ = write!(
        body,
        "<p class=\"lead\">{} event(s) · {} byte(s) read from {} file(s) · \
         <code>{}</code>",
        tail.records.len(),
        tail.bytes_read,
        tail.files_read,
        render::escape(&read.0.display().to_string()),
    );
    // BOTH DIRECTORIES ARE NAMED, for the reason `json_of` serves both: the
    // walk covers two and a page that names one lets a reader conclude the
    // other was empty — which is the reading that made this defect invisible
    // for as long as it lasted.
    match read.1 {
        Some(cli) => {
            let _ = write!(
                body,
                " and <code>{}</code></p>",
                render::escape(&cli.display().to_string()),
            );
        }
        // NOT SILENCE. No store root resolved means the `cli` half was never
        // opened, and a page that said nothing would be reporting "no terminal
        // events" in the same words it uses for "there were none" — the
        // fallback that hides a failure CLAUDE.md §4 bans.
        None => body.push_str(
            " · <b>the cli half was NOT read</b> — no store root resolved, so \
             nothing a terminal-run sweep wrote is below.</p>",
        ),
    }

    body.push_str(&health_banner(health));

    let notes = walk_notes(tail, health);
    if !notes.is_empty() {
        body.push_str("<ul class=\"notes\">");
        for note in &notes {
            let _ = write!(body, "<li>{}</li>", render::escape(note));
        }
        body.push_str("</ul>");
    }

    if tail.records.is_empty() {
        // NAME THE FILTER THAT ACTUALLY EMPTIED THE PAGE.
        //
        // This used to point at `BRUTEX_LOG_LEVEL` unconditionally. Under
        // `?target=pull.http` matching nothing, that sends the reader to change
        // the WRITE floor and re-run a job, when the cause is a read filter they
        // can clear in one click. Worse, under `?level=warn` the suggested
        // remedy provably cannot change the answer: a `debug` line written to
        // the file is still excluded by the reader's own floor. `CLAUDE.md` §4
        // asks for the reason to be named, not for a remedy that cannot work.
        let filtered = asked.level.is_some() || !asked.target.is_empty() || asked.run != 0;
        if filtered {
            let mut said =
                String::from("<p class=\"lead\">Nothing matched <b>the filters in force</b>:");
            if let Some(level) = asked.level {
                let _ = write!(said, " level at or above <code>{}</code>;", level.label());
            }
            if !asked.target.is_empty() {
                let _ = write!(
                    said,
                    " target <code>{}</code>;",
                    render::escape(&asked.target)
                );
            }
            if asked.run != 0 {
                let _ = write!(said, " run <code>{}</code>;", asked.run);
            }
            said.push_str(" widening or clearing them is the first thing to try.");
            // ONLY WHEN IT COULD HELP. With a read floor above `debug`, raising
            // the WRITE floor changes nothing the reader would then admit.
            if asked
                .level
                .is_none_or(|l| telemetry::Level::Debug.at_least(l))
            {
                said.push_str(
                    " Members are also written at <code>debug</code>, below the \
                     default <code>info</code> write floor — set \
                     <code>BRUTEX_LOG_LEVEL=debug</code> and re-run to record them.",
                );
            }
            said.push_str("</p>");
            body.push_str(&said);
        } else {
            body.push_str(
                "<p class=\"lead\">Nothing matched. A quiet log is the ordinary \
                 state: members are written at <code>debug</code>, and the default \
                 floor is <code>info</code> — set <code>BRUTEX_LOG_LEVEL=debug</code> \
                 and re-run to see them.</p>",
            );
        }
        return page_shell(asked, &body);
    }

    body.push_str(&rows_table(tail));
    page_shell(asked, &body)
}

/// Every flag the walk raised, in the operator's words.
///
/// **These go above the rows, not below them.** Someone who scrolls a table and
/// finds nothing alarming has been told something; they have only been told the
/// truth if they also know the walk stopped early. `hit_scan_cap` is the
/// difference between "no errors in the log" and "no errors in the four
/// megabytes I happened to read".
fn walk_notes(tail: &telemetry::Tail, health: Option<&telemetry::Health>) -> Vec<String> {
    let mut notes: Vec<String> = Vec::new();
    // FIRST, BECAUSE IT IS THE FIRST QUESTION. Every other note here reports a
    // limit of the READER — how far it walked, what it could not decode. This
    // one reports a loss by the WRITER, and it decides whether anything below
    // can be trusted. A log handed to somebody who did not run the job is
    // evidence, and evidence with silent holes is worse than none.
    if let Some(missing) = tail.missing.filter(|n| *n > 0) {
        // WHERE TO LOOK DEPENDS ON WHETHER THE BANNER CAN ANSWER.
        //
        // `tail.missing` is DURABLE: it is computed from the sequence hole in
        // the FILE, so it survives a restart. `Health::dropped` is PER-PROCESS —
        // `Sink::open` resumes `seq` from the file but starts `dropped` at zero.
        // After the restart that follows a failed run, which is the routine
        // action, the two disagree, and this note used to send the reader to a
        // banner reading "0 dropped" for a count it did not hold. The page
        // contradicted itself and pointed at the half that was wrong.
        let whither = if health.is_some_and(telemetry::Health::is_loud) {
            "see the sink banner above for the count and the reason."
        } else {
            "the banner above counts only THIS process's losses and counts none, \
             so these were lost by an earlier run and no counter on this page \
             holds the reason."
        };
        notes.push(format!(
            "{missing} event(s) are MISSING from this range. The sink numbered \
             them and the file does not hold them, so they were dropped rather \
             than filtered — {whither} Nothing below is a complete picture of \
             what happened."
        ));
    }
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

/// This process's own sink, if it has one.
///
/// `telemetry::global()` is an `OnceLock` read and `Sink::health` is a handful
/// of relaxed atomic loads plus one uncontended mutex — no `metadata` call, no
/// directory scan, no walk of the file. O(1), and it stays O(1) as the log
/// grows, which is the only reason it is safe to do on every page render.
///
/// **Proved by `telemetry::sink::the_roll_decision_reads_the_running_count_and_never_the_files_size`**
/// (invariant T-01), which grows the log behind the sink's back and then asserts
/// `health().current_bytes < on_disk` — "the count is the sink's own, not the
/// file's". A `metadata` call anywhere in `health` would fail it. That is the
/// load-bearing half of the claim: the cost is independent of the file, which
/// is the size that grows all day.
fn sink_health() -> Option<telemetry::Health> {
    telemetry::global().map(telemetry::Sink::health)
}

/// **What the WRITER lost, which the reader cannot see.**
///
/// `telemetry::tail` reads what reached the file. It is structurally incapable
/// of reporting what never got there — a dropped event leaves no line to count.
/// So a page built only from the tail shows `0 events` for two opposite worlds:
/// nothing happened, and everything was thrown away. `Health::dropped` is the
/// only trace the second one leaves, and its own doc comment says "this is the
/// number a page must show". Until now no page showed it, and `is_loud` had no
/// caller at all.
///
/// `CLAUDE.md` §4 bans "a fallback that hides a failure — degrade loudly and
/// name the reason, or refuse. Never both silently." A sink that drops events
/// and renders a clean page is exactly that fallback.
///
/// The quiet case still prints a line. "Nothing was dropped" is a claim worth
/// making explicitly: an absent banner is indistinguishable from a banner this
/// page forgot to render, and the operator cannot tell those apart.
fn health_banner(health: Option<&telemetry::Health>) -> String {
    let Some(h) = health else {
        return "<p class=\"halt\"><b>No sink</b>This process is not writing a \
                log, so the events below — if any — are from an older run. The \
                server names the reason on stdout at startup.</p>"
            .to_owned();
    };
    if !h.is_loud() {
        return format!(
            "<p class=\"lead\">Sink healthy · {} written · 0 dropped · {} \
             rotation(s) · {} byte(s) in the current file.</p>",
            h.written, h.rotations, h.current_bytes,
        );
    }
    let mut out = String::from("<p class=\"halt\"><b>The log is incomplete</b>");
    if h.dropped > 0 {
        let _ = write!(
            out,
            "{} event(s) happened and were never written. They are gone — this \
             page cannot show them, and no filter will bring them back. ",
            h.dropped,
        );
    }
    if h.rotation_failures > 0 {
        let _ = write!(
            out,
            "{} roll(s) failed, so the current file is past its bound and the \
             oldest events may already have been overwritten. ",
            h.rotation_failures,
        );
    }
    if let Some(why) = h.last_error.as_deref() {
        let _ = write!(out, "Most recent failure: {}.", render::escape(why));
    }
    out.push_str("</p>");
    out
}

/// The events themselves, newest first.
fn rows_table(tail: &telemetry::Tail) -> String {
    let mut out = String::from(
        "<div class=\"hscroll\"><table><thead><tr>\
         <th>When (IST)</th><th>Level</th><th>Target</th><th>Message</th>\
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
            // IST, THROUGH THE SAME FUNCTION EVERY OTHER PAGE USES.
            //
            // This column rendered `record.at_utc` — the line's own UTC text —
            // while `/audit` rendered the same instant through `ist_stamp`. An
            // operator reading a failed run on one page and its log on the
            // other was reading two clocks 5h30m apart, and nothing on either
            // page said so. That is the "two answers to one question" shape
            // this repository treats as a defect rather than a nicety.
            //
            // The STORED value is untouched and stays UTC — `at_unix_millis`
            // on the record, `ts` in the file, and `/logs.json` still answers
            // in epoch milliseconds. One representation on disk, one clock on
            // every human surface, and the conversion happens once at the
            // boundary. See `crate::server::ist_stamp`.
            render::escape(&crate::server::ist_stamp(
                record.at_unix_millis.div_euclid(1_000),
            )),
            render::escape(record.level.label()),
            render::escape(&record.target),
            render::escape(&record.message),
        );
    }
    out.push_str("</tbody></table></div>");
    out
}

/// The filter the page is showing, as a query string `/logs.json` will honour.
///
/// **Two defects lived in the one expression this replaces.**
///
/// It read `format!("?target={}", render::escape(&asked.target))`.
///
/// 1. It dropped the level and the limit. An operator who filtered the page to
///    `level=error&limit=200` and then followed "as JSON" got the unfiltered
///    default 50 — a different answer to the same question, from the link whose
///    entire purpose is to be the same answer in another format.
/// 2. It HTML-escaped where a URL needs percent-encoding. `render::escape`
///    rewrites five characters into entities; a target containing `&` became
///    `&amp;`, which a browser sends as a literal `&` and the server then reads
///    as a PARAMETER SEPARATOR. `render::query_value` is the encoder for this
///    position, and its own doc says the result must not then be passed through
///    `escape` — so the value is percent-encoded here and the surrounding
///    attribute is safe as it stands.
///
/// The level is written by label rather than by rank because that is what
/// `Level::of_label` reads back, and the limit is the CLAMPED number rather than
/// the one asked for, so the link reproduces the page rather than the request.
fn json_query(asked: &Asked) -> String {
    let mut q = format!("?limit={}", asked.limit);
    if let Some(level) = asked.level {
        let _ = write!(q, "&level={}", level.label());
    }
    if !asked.target.is_empty() {
        let _ = write!(q, "&target={}", render::query_value(&asked.target));
    }
    if asked.run != 0 {
        let _ = write!(q, "&run={}", asked.run);
    }
    q
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
         <label>Run<input type=\"text\" name=\"run\" value=\"{run}\" placeholder=\"every run\"></label>\
         <label>Limit<input type=\"number\" name=\"limit\" min=\"1\" max=\"{max}\" value=\"{limit}\"></label>\
         <button type=\"submit\">Show</button>\
         <a href=\"/logs.json{q}\">as JSON</a>\
         </form>{body}",
        levels = levels,
        target = render::escape(&asked.target),
        run = if asked.run == 0 {
            String::new()
        } else {
            asked.run.to_string()
        },
        limit = asked.limit,
        max = PAGE_LIMIT,
        q = json_query(asked),
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
    ///
    /// # Why the path comes from `crate::scratch::path` and not from here
    ///
    /// It used to be `std::env::temp_dir().join(format!("brutex-logs-{name}"))`
    /// — one fixed directory per test name, in a directory every process on the
    /// machine shares — and the first thing this helper does to it is
    /// `remove_dir_all`. Two `api` test binaries alive at once therefore delete
    /// each other's sink mid-test: two `cargo test` runs, a `cargo llvm-cov`
    /// beside a `cargo test`, or two agents working one checkout are each
    /// enough. The failure it produces is not a clear one — the loser's events
    /// are gone by the time `json_over` reads the directory back, so it surfaces
    /// as a rendered page missing a row that was demonstrably emitted, which
    /// reads as a bug in the renderer and is not one.
    ///
    /// `crate::scratch::path` stamps the process id into the name and adds the
    /// `brutex-` prefix, so no two live processes name the same directory. That
    /// is the failure `crate::scratch` was written for; this call site was
    /// simply missed when the rest of the crate moved over.
    ///
    /// **It does not make `name` optional.** The stamp is the process, not the
    /// test, so two tests in this module passing the same `name` still share one
    /// sink and still delete each other's events — the collision just moves
    /// inside the binary, where it is at least deterministic. Every caller below
    /// passes a name no other one does, and a new caller must too.
    fn sink_in(name: &str) -> (std::path::PathBuf, telemetry::Sink) {
        let dir = crate::scratch::path(&format!("logs-{name}"));
        let _ignored = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a scratch directory");
        let sink = telemetry::Sink::open(
            &telemetry::Config::new(&dir).with_min_level(telemetry::Level::Trace),
        )
        .expect("opens");
        (dir, sink)
    }

    /// THE FIXTURE DIRECTORY NAMES THE PROCESS THAT OPENED IT.
    ///
    /// This is the one property of [`sink_in`] no other test in this module can
    /// observe, because the collision it prevents is between two *processes* and
    /// every test here runs in one. Without the `crate::scratch::path` call the
    /// directory is `brutex-logs-pid-stamp` for every process on the machine, a
    /// name this helper opens by deleting — so the assertion below is the only
    /// thing standing between a future edit and the intermittent, misattributed
    /// failure described on `sink_in`.
    ///
    /// The rest of the naming contract — that two names differ, that the parent
    /// is the temporary directory, that one name asked for twice is one path —
    /// belongs to `crate::scratch` and is asserted there. Restating it here
    /// would be a second copy of a rule that already has one.
    #[test]
    fn a_sink_directory_names_the_process_that_opened_it() {
        let (dir, _sink) = sink_in("pid-stamp");
        let text = dir.to_string_lossy().into_owned();
        assert!(
            text.contains(&std::process::id().to_string()),
            "a fixture two processes can both name is one they can both delete: {text}"
        );
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
        let json = json_over(&dir, Some(&dir.join(super::CLI_SUBDIR)), &asked, None);
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

        let page = page_over(&dir, Some(&dir.join(super::CLI_SUBDIR)), &asked, None);
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

    /// EVERY HUMAN SURFACE READS THE SAME CLOCK, AND IT IS IST.
    ///
    /// This column rendered the line's own UTC text while `/audit` rendered
    /// the same instant through `server::ist_stamp` — two clocks 5h30m apart
    /// on two pages describing one run, with nothing on either saying so.
    ///
    /// The stored value is deliberately NOT converted: `at_unix_millis` stays
    /// UTC on the record, `ts` stays UTC in the file, and `/logs.json` still
    /// answers in epoch milliseconds. One representation on disk, one clock on
    /// every page, conversion exactly once at the boundary.
    #[test]
    fn the_page_renders_ist_and_the_json_stays_utc() {
        // 2026-08-10 04:58:45 UTC is 10:28:45 IST — an instant whose IST hour
        // differs from its UTC hour, so a page that forgot to convert cannot
        // pass by coincidence.
        const UTC_SECS: i64 = 1_786_337_925;
        let (dir, sink) = sink_in("ist");
        assert!(
            sink.emit(&telemetry::Event::error("pull.member", "did not land"))
                .is_written(),
            "the premise: the sink accepted it"
        );

        let asked = asked("limit=10");
        let page = page_over(&dir, Some(&dir.join(super::CLI_SUBDIR)), &asked, None);
        assert!(
            page.contains("When (IST)"),
            "the column says which clock it is: {page}"
        );
        assert!(
            !page.contains("When (UTC)"),
            "and never claims to be the other one"
        );

        // The renderer is the SHARED one, so /audit and /logs cannot drift.
        let expect = crate::server::ist_stamp(UTC_SECS);
        assert!(
            expect.ends_with(" IST"),
            "the shared renderer labels its own output: {expect}"
        );
        assert!(
            expect.starts_with("2026-08-10 10:28:45"),
            "10:28:45 IST is 04:58:45 UTC — the +5:30 is applied once: {expect}"
        );

        // AND THE MACHINE SURFACE IS UNTOUCHED. A consumer parsing this must
        // not have to guess which zone a number is in.
        let json = json_over(&dir, Some(&dir.join(super::CLI_SUBDIR)), &asked, None);
        assert!(
            json.contains(r#""ts":"#),
            "the JSON carries epoch millis, not a rendered string: {json}"
        );
        assert!(
            !json.contains("IST"),
            "and never a zone label, because it is UTC epoch: {json}"
        );
    }

    /// **The PAGE shows the `cli` half — and this test used to prove it about
    /// the JSON.**
    ///
    /// # The defect the test's own name describes, and did not cover
    ///
    /// `both_halves` was wired to `json_over` alone. `page_over` called
    /// `telemetry::tail` on the server's directory and nothing else, so `/logs`
    /// — the surface an operator opens — showed one half while `/logs.json`
    /// showed two. MEASURED on 2026-09-01: the page showed nothing newer than
    /// 31 August while **66 sweep events sat unread** in
    /// `<store>/logs/cli/events.ndjson`, and every `cli` run printed *"The
    /// /logs page walks BOTH halves … so these events appear there. D-0301."*
    ///
    /// This test was named `the_page_shows_the_cli_half…` and asserted on
    /// `json_over`. **A test that guards the wrong function is how a defect
    /// written down twice survives being written down twice**: the name says
    /// the property is covered, the assertion covers the surface that already
    /// had it, and nothing red ever appears. Both surfaces are driven below,
    /// and the page is asserted first because it is the one that was wrong.
    #[test]
    fn the_page_shows_the_cli_half_and_not_only_the_servers() {
        // THE REPRODUCED CASE, printed by `cli` on every single run:
        //   "events -> <store>/logs/cli
        //    The /logs page reads whatever directory `api` resolved, which is
        //    NOT this one."
        //
        // Every event a terminal-run sweep wrote -- every span loaded, every
        // derived threshold, every month found or missing -- was on disk and
        // invisible on the page that exists to show it.
        let (dir, served) = sink_in("both-halves");
        let cli_dir = dir.join(super::CLI_SUBDIR);
        std::fs::create_dir_all(&cli_dir).expect("the cli half");
        let ran = telemetry::Sink::open(
            &telemetry::Config::new(&cli_dir).with_min_level(telemetry::Level::Trace),
        )
        .expect("opens");

        let _ = served.emit(&telemetry::Event::info("api.serve", "the server said this"));
        let _ = ran.emit(&telemetry::Event::info(
            "cli.sweep",
            "the terminal said this",
        ));

        // THE PAGE FIRST, because the page is what was broken. `page_over` is
        // the function `logs_page` calls; asserting on `json_over` here proved
        // a property that surface already had.
        let page = page_over(
            &dir,
            Some(&dir.join(super::CLI_SUBDIR)),
            &asked("limit=10"),
            None,
        );
        assert!(
            page.contains("the terminal said this"),
            "THE DEFECT: `/logs` walked the server's directory alone, so every \
             event a terminal-run sweep wrote was on disk and invisible on the \
             page that exists to show it: {page}"
        );
        assert!(
            page.contains("the server said this"),
            "and the server's half must not be lost to the merge: {page}"
        );
        assert!(
            page.contains(&dir.join(super::CLI_SUBDIR).display().to_string()),
            "the page names the second directory it read in full, so a reader \
             can tell an empty half from an unread one: {page}"
        );

        let json = json_over(
            &dir,
            Some(&dir.join(super::CLI_SUBDIR)),
            &asked("limit=10"),
            None,
        );
        assert!(
            json.contains("the server said this"),
            "the server's half must not be lost to the merge: {json}"
        );
        assert!(
            json.contains("the terminal said this"),
            "this is the whole point of the walk: {json}"
        );
    }

    #[test]
    fn a_store_where_nobody_ran_cli_is_an_empty_half_and_not_an_error() {
        // `telemetry::tail` is documented never to fail and to render a missing
        // directory as no records. The merge must not turn that into a failure,
        // and must not report older events existing because a directory that
        // was never created could not reach its own oldest file.
        let (dir, served) = sink_in("no-cli-half");
        assert!(!dir.join(super::CLI_SUBDIR).exists(), "no cli half here");
        let _ = served.emit(&telemetry::Event::info("api.serve", "alone"));

        let json = json_over(
            &dir,
            Some(&dir.join(super::CLI_SUBDIR)),
            &asked("limit=10"),
            None,
        );
        assert!(json.contains("alone"), "{json}");
        assert!(
            !json.contains(r#""reached_oldest":false"#),
            "an absent half must not claim older events exist: {json}"
        );
    }

    #[test]
    fn the_merged_answer_is_newest_first_across_both_halves() {
        // MERGED ON TIME, NEVER ON SEQUENCE. Each sink numbers its own events
        // from its own run, so `seq` is unique within a directory and
        // meaningless across two -- ordering the union by it would interleave a
        // terminal event from this morning with a server event from last week
        // wherever the two counters happened to collide.
        let (dir, served) = sink_in("merge-order");
        let cli_dir = dir.join(super::CLI_SUBDIR);
        std::fs::create_dir_all(&cli_dir).expect("the cli half");
        let ran = telemetry::Sink::open(
            &telemetry::Config::new(&cli_dir).with_min_level(telemetry::Level::Trace),
        )
        .expect("opens");

        // Interleaved in time, alternating sinks, so a merge that ordered by
        // anything but the clock would show them grouped by source.
        let _ = served.emit(&telemetry::Event::info("api.serve", "first"));
        let _ = ran.emit(&telemetry::Event::info("cli.sweep", "second"));
        let _ = served.emit(&telemetry::Event::info("api.serve", "third"));

        let tail = super::both_halves(
            &dir,
            Some(&dir.join(super::CLI_SUBDIR)),
            &asked("limit=10").query,
        );
        let messages: Vec<&str> = tail
            .records
            .iter()
            .map(|record| record.message.as_str())
            .collect();
        assert_eq!(messages.len(), 3, "every event, once: {messages:?}");

        // NEWEST FIRST, which is the order every other reader in this module
        // promises. Timestamps are milliseconds and three emits can share one,
        // so the assertion is on the ORDERING RELATION rather than on an exact
        // sequence -- a tie broken by `seq` is still deterministic, and pinning
        // a permutation that depends on clock granularity is a flaky test.
        // `windows(2)` yields pairs and the lint denies indexing even where the
        // length is guaranteed, so the pair is destructured rather than indexed
        // -- which is also the spelling that cannot go wrong if the width ever
        // changes.
        for pair in tail.records.windows(2) {
            if let [newer, older] = pair {
                assert!(
                    newer.at_unix_millis >= older.at_unix_millis,
                    "out of order: {messages:?}"
                );
            }
        }
    }

    #[test]
    fn the_limit_bounds_the_union_and_not_each_half() {
        // Each half already honours the limit, so without a truncation on the
        // merged set the page returns up to TWICE what was asked for -- and a
        // caller's `limit` bounds the response, it is not a suggestion.
        let (dir, served) = sink_in("merge-limit");
        let cli_dir = dir.join(super::CLI_SUBDIR);
        std::fs::create_dir_all(&cli_dir).expect("the cli half");
        let ran = telemetry::Sink::open(
            &telemetry::Config::new(&cli_dir).with_min_level(telemetry::Level::Trace),
        )
        .expect("opens");
        for index in 0..6 {
            let _ = served.emit(&telemetry::Event::info("api.serve", "server line"));
            let _ = ran.emit(&telemetry::Event::info("cli.sweep", "cli line"));
            let _ = index;
        }

        let tail = super::both_halves(
            &dir,
            Some(&dir.join(super::CLI_SUBDIR)),
            &asked("limit=4").query,
        );
        assert_eq!(tail.records.len(), 4, "the union is what the limit bounds");
    }

    #[test]
    fn the_level_filter_is_applied_to_what_is_returned() {
        let (dir, sink) = sink_in("levels");
        let _ = sink.emit(&telemetry::Event::debug("pull.member", "landed"));
        let _ = sink.emit(&telemetry::Event::error("pull.member", "did not land"));

        let all = json_over(
            &dir,
            Some(&dir.join(super::CLI_SUBDIR)),
            &asked("limit=10"),
            None,
        );
        assert!(
            all.contains("landed") && all.contains("did not land"),
            "{all}"
        );

        let loud = json_over(
            &dir,
            Some(&dir.join(super::CLI_SUBDIR)),
            &asked("limit=10&level=error"),
            None,
        );
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
        let page = page_over(
            &dir,
            Some(&dir.join(super::CLI_SUBDIR)),
            &asked("limit=10"),
            None,
        );
        assert!(
            page.contains("BRUTEX_LOG_LEVEL=debug"),
            "a quiet log is the ordinary state and the page must say what to \
             change to see more: {page}"
        );
    }

    /// **An empty page blames the filter that emptied it, not always the writer.**
    ///
    /// The message used to name `BRUTEX_LOG_LEVEL` whatever the query was. Under
    /// `?target=` that sends the reader to change the WRITE floor and re-run a
    /// job when the cause is a read filter they can clear in one click; under
    /// `?level=warn` it is worse than unhelpful, because a `debug` line written
    /// to the file would still be excluded by the reader's own floor — a remedy
    /// that provably cannot change the answer.
    #[test]
    fn an_empty_page_names_the_filter_in_force_rather_than_always_the_write_floor() {
        let (dir, _sink) = sink_in("empty-filtered");

        // A TARGET FILTER. The write floor is not the cause and is not the cure,
        // but debug lines could still be admitted by the reader, so the write
        // hint is allowed to stay beside the real reason.
        let by_target = page_over(
            &dir,
            Some(&dir.join(super::CLI_SUBDIR)),
            &asked("limit=10&target=pull.http"),
            None,
        );
        assert!(
            by_target.contains("the filters in force"),
            "the page must say the filters are why it is empty: {by_target}"
        );
        assert!(
            by_target.contains("pull.http"),
            "and name the target it actually filtered on: {by_target}"
        );

        // A LEVEL FILTER ABOVE DEBUG. Raising the write floor cannot help, so the
        // suggestion must not appear at all.
        let by_level = page_over(
            &dir,
            Some(&dir.join(super::CLI_SUBDIR)),
            &asked("limit=10&level=warn"),
            None,
        );
        assert!(
            by_level.contains("the filters in force"),
            "the level filter is named as the cause: {by_level}"
        );
        assert!(
            !by_level.contains("BRUTEX_LOG_LEVEL"),
            "a `debug` line would STILL be excluded by the reader's own floor, so \
             offering the write floor as the remedy is advice that cannot work: {by_level}"
        );

        // A RUN FILTER.
        let by_run = page_over(
            &dir,
            Some(&dir.join(super::CLI_SUBDIR)),
            &asked("limit=10&run=42"),
            None,
        );
        assert!(
            by_run.contains("42"),
            "the run narrowed to is named: {by_run}"
        );

        // AND THE UNFILTERED CASE IS UNCHANGED — the original wording still
        // applies when nothing the reader chose is responsible.
        let plain = page_over(
            &dir,
            Some(&dir.join(super::CLI_SUBDIR)),
            &asked("limit=10"),
            None,
        );
        assert!(
            plain.contains("BRUTEX_LOG_LEVEL=debug") && !plain.contains("the filters in force"),
            "with no filter the write floor IS the thing to change: {plain}"
        );
    }

    /// A `Health` this test owns, so it can describe a sink no test can cause.
    ///
    /// A dropped event needs a full disk or a failing write; a failed roll needs
    /// a rename to fail. Neither is reachable from a unit test, and a branch
    /// that is only reachable in production is a branch nobody has ever seen
    /// render. The fields are `pub`, so the state can be stated directly.
    /// The one note, without indexing — `clippy::indexing_slicing` is denied
    /// workspace-wide and a test is not exempt from it.
    fn note_of(notes: &[String]) -> &str {
        notes.first().map_or("<no note at all>", String::as_str)
    }

    /// **The MISSING note must not send a reader to a banner that cannot answer.**
    ///
    /// `Tail::missing` is computed from the sequence hole in the FILE and so
    /// survives a restart. `Health::dropped` is per-process and starts at zero.
    /// The routine action after a failed run is a restart, so the pair
    /// "5 missing / 0 dropped" is the NORMAL state, not an exotic one — and the
    /// note used to point at the banner for a count the banner did not hold.
    ///
    /// Both branches are pinned. A fix that only ever produced one of them would
    /// pass a test that checked the other.
    #[test]
    fn the_missing_note_points_at_the_banner_only_when_the_banner_knows() {
        let mut lost = walk(Vec::new());
        lost.missing = Some(5);

        // THE RESTART CASE. The file remembers the hole; this process does not.
        let after_restart = walk_notes(&lost, Some(&health(0, 0, None)));
        assert_eq!(
            after_restart.len(),
            1,
            "one flag, one note: {after_restart:?}"
        );
        assert!(
            note_of(&after_restart).contains("counts only THIS process's losses"),
            "a banner reading 0 dropped must not be offered as the source of a \
             count it does not have: {}",
            note_of(&after_restart)
        );
        assert!(
            !note_of(&after_restart).contains("see the sink banner above"),
            "and it must not ALSO say the opposite: {}",
            note_of(&after_restart)
        );

        // THE SAME-PROCESS CASE. The banner really does hold the count.
        let same_process = walk_notes(&lost, Some(&health(5, 0, None)));
        assert_eq!(same_process.len(), 1, "one flag, one note");
        assert!(
            note_of(&same_process).contains("see the sink banner above"),
            "when the banner is loud it IS the place to look: {}",
            note_of(&same_process)
        );

        // A ROTATION FAILURE ALSO MAKES THE BANNER LOUD, so it can answer too —
        // `is_loud` is an OR, and pinning only `dropped` would let a mutant
        // turn it into `dropped > 0` and survive.
        let rolled = walk_notes(&lost, Some(&health(0, 3, None)));
        assert!(
            note_of(&rolled).contains("see the sink banner above"),
            "rotation_failures makes the banner loud as well: {}",
            note_of(&rolled)
        );

        // NO SINK AT ALL. There is no banner to consult, so it must not be cited.
        let none = walk_notes(&lost, None);
        assert!(
            note_of(&none).contains("counts only THIS process's losses"),
            "with no sink there is nothing to point at: {}",
            note_of(&none)
        );

        // And zero missing is still not a note at all.
        assert!(
            walk_notes(&walk(Vec::new()), Some(&health(9, 9, None))).is_empty(),
            "a loud banner does not invent a MISSING note"
        );
    }

    fn health(dropped: u64, rotation_failures: u64, last_error: Option<&str>) -> telemetry::Health {
        telemetry::Health {
            path: std::path::PathBuf::from("events.ndjson"),
            written: 1_200,
            dropped,
            rotations: 3,
            rotation_failures,
            last_error: last_error.map(str::to_owned),
            current_bytes: 4_096,
            next_seq: 1_201,
        }
    }

    /// **THE WRITER'S LOSSES REACH BOTH SURFACES.**
    ///
    /// `telemetry::tail` reads what reached the file, so a page built from it
    /// alone renders `0 events` both when nothing happened and when everything
    /// was dropped. `Health::dropped` is the only thing that separates them —
    /// its own doc says "this is the number a page must show" — and before this
    /// test no page showed it and `Health::is_loud` had no caller anywhere.
    #[test]
    fn a_sink_that_lost_events_says_so_on_the_page_and_in_the_json() {
        // Healthy: the quiet case still makes its claim out loud, because an
        // absent banner and a forgotten banner look identical.
        let ok = health(0, 0, None);
        let page = health_banner(Some(&ok));
        assert!(page.contains("Sink healthy"), "{page}");
        assert!(
            page.contains("0 dropped"),
            "the zero is stated, not implied: {page}"
        );
        assert!(
            !page.contains("halt"),
            "a healthy sink is not an alarm: {page}"
        );

        // Lost events: loud, and honest that they are unrecoverable.
        let lost = health(17, 0, None);
        let page = health_banner(Some(&lost));
        assert!(
            page.contains("halt"),
            "a loss must be styled as a fault: {page}"
        );
        assert!(page.contains("17 event(s)"), "the count is named: {page}");
        assert!(
            page.contains("no filter will bring them back"),
            "and it must not read as something the operator can query around: {page}"
        );
        // The ABSENCE is the half that separates `> 0` from `>= 0`. Asserting
        // only that the banner says what it should leaves a guard that fires
        // unconditionally indistinguishable from one that fires correctly.
        assert!(
            !page.contains("roll(s) failed"),
            "and it must not claim a failed roll that did not happen: {page}"
        );

        // A failed roll is the OTHER way the log lies, and is reported apart
        // from a drop — the causes differ and so does the remedy.
        let torn = health(0, 2, Some("events.ndjson.1: Permission denied"));
        let page = health_banner(Some(&torn));
        assert!(
            page.contains("halt") && page.contains("2 roll(s) failed"),
            "{page}"
        );
        assert!(
            !page.contains("were never written"),
            "and it must not claim a drop that did not happen — `dropped > 0` is \
             a different question from `rotation_failures > 0`: {page}"
        );
        assert!(
            page.contains("Permission denied"),
            "the failure's own words: {page}"
        );

        // No sink at all is its own state, and NOT a healthy one: events on the
        // page then belong to an older run.
        let none = health_banner(None);
        assert!(
            none.contains("No sink") && none.contains("older run"),
            "{none}"
        );

        // The JSON carries the same facts, machine-readable, and `null` when
        // there is no sink — never a zeroed object, which would read as healthy.
        assert_eq!(sink_json(None), "null");
        let json = sink_json(Some(&lost));
        assert!(json.contains(r#""dropped":17"#), "{json}");
        assert!(json.contains(r#""loud":true"#), "{json}");
        assert!(json.contains(r#""last_error":null"#), "{json}");
        let json = sink_json(Some(&torn));
        assert!(json.contains(r#""rotation_failures":2"#), "{json}");
        assert!(json.contains("Permission denied"), "{json}");
        assert!(sink_json(Some(&ok)).contains(r#""loud":false"#));
    }

    /// A `Record` this test owns.
    fn rec(seq: u64, level: telemetry::Level, fields: &[(&str, i64)]) -> telemetry::Record {
        telemetry::Record {
            seq,
            run: 0,
            at_unix_millis: 1_767_205_800_000,
            at_utc: "2025-12-31T18:30:00Z".to_owned(),
            level,
            target: "pull.member".to_owned(),
            message: "did not land".to_owned(),
            fields: fields
                .iter()
                .map(|(k, v)| ((*k).to_owned(), telemetry::OwnedValue::Int(*v)))
                .collect(),
            cut: false,
            dropped_fields: 0,
        }
    }

    /// A `Tail` this test owns — see [`json_of`] for why it is stated, not caused.
    fn walk(records: Vec<telemetry::Record>) -> telemetry::Tail {
        telemetry::Tail {
            records,
            bytes_read: 512,
            files_read: 1,
            malformed: 0,
            partial_tail: false,
            hit_scan_cap: false,
            reached_oldest: true,
            errors: Vec::new(),
            // A hand-built walk has no holes by construction.
            missing: Some(0),
        }
    }

    /// **THE JSON IS WELL-FORMED, NOT MERELY CONTAINING THE RIGHT WORDS.**
    ///
    /// Every `n > 0` in this file is a comma separator, and a `contains` assertion
    /// cannot see one in the wrong place: `{,"a":1}` still contains `"a":1`. The
    /// separators are therefore pinned by exact shape at all three levels — the
    /// record array, the field object, and the error array — which is the only
    /// form of the assertion that a misplaced comma fails.
    #[test]
    fn every_comma_in_the_json_is_pinned_by_shape_and_not_by_substring() {
        let two = walk(vec![
            rec(1, telemetry::Level::Error, &[("bars", 806), ("gaps", 2)]),
            rec(2, telemetry::Level::Info, &[("bars", 12)]),
        ]);
        let json = json_of(
            &two,
            &asked("limit=10"),
            None,
            (std::path::Path::new("/served"), None),
        );

        // Two records: exactly one comma between them, none before or after.
        assert!(json.starts_with(r#"{"records":[{"seq":1,"#), "{json}");
        assert!(
            json.contains(r#"}},{"seq":2,"#),
            "one comma between records: {json}"
        );
        // Two fields: exactly one comma between them, none after the brace.
        assert!(
            json.contains(r#""fields":{"bars":806,"gaps":2}}"#),
            "{json}"
        );
        // One field: no comma at all.
        assert!(json.contains(r#""fields":{"bars":12}}"#), "{json}");
        // Empty: neither a stray comma nor a missing brace.
        assert!(
            json_of(
                &walk(Vec::new()),
                &asked(""),
                None,
                (std::path::Path::new("/served"), None)
            )
            .starts_with(r#"{"records":[],"#),
            "an empty walk is an empty array"
        );

        // The error array is the third separator, and needs two unreadable files
        // — a state no unit test can cause, which is why the walk is stated.
        let mut torn = walk(Vec::new());
        torn.errors = vec!["one.ndjson: bad".to_owned(), "two.ndjson: bad".to_owned()];
        let json = json_of(
            &torn,
            &asked(""),
            None,
            (std::path::Path::new("/served"), None),
        );
        assert!(
            json.contains(r#""errors":["one.ndjson: bad","two.ndjson: bad"]"#),
            "{json}"
        );
        assert!(
            json_of(
                &walk(Vec::new()),
                &asked(""),
                None,
                (std::path::Path::new("/served"), None)
            )
            .contains(r#""errors":[]"#),
            "and no stray comma when there are none"
        );
    }

    /// **EVERY WALK FLAG REACHES THE PAGE, AND EACH ONE ALONE.**
    ///
    /// `walk_notes` had six surviving mutants — including `-> vec![]`, which is
    /// "silently report nothing went wrong" and is precisely the failure
    /// `CLAUDE.md` §4 bans. Each flag is asserted on its own, so a note that
    /// fires for the wrong reason is not covered by a note that fires for the
    /// right one.
    #[test]
    fn each_walk_flag_produces_its_own_note_and_reaches_the_page() {
        assert!(
            walk_notes(&walk(Vec::new()), None).is_empty(),
            "a clean walk says nothing — otherwise every page cries wolf"
        );

        // `only` asserts the count as well as the text, so a flag that raises
        // two notes — or raises one belonging to a different flag — fails here
        // rather than passing on a `contains` that happened to match.
        let only = |tail: &telemetry::Tail| -> String {
            let notes = walk_notes(tail, None);
            assert_eq!(notes.len(), 1, "one flag, one note: {notes:?}");
            notes.into_iter().next().unwrap_or_default()
        };

        let mut capped = walk(Vec::new());
        capped.hit_scan_cap = true;
        assert!(only(&capped).contains("NOT everything there is"));

        let mut older = walk(Vec::new());
        older.reached_oldest = false;
        assert!(only(&older).contains("Older events exist"));

        let mut torn = walk(Vec::new());
        torn.partial_tail = true;
        assert!(only(&torn).contains("mid-line"));

        // `malformed > 0` — the zero case must produce NO note, which is what
        // separates `> 0` from `>= 0`.
        let mut bad = walk(Vec::new());
        bad.malformed = 3;
        assert!(only(&bad).contains("3 line(s)"));
        assert!(
            walk_notes(&walk(Vec::new()), None).is_empty(),
            "zero malformed lines is not a note"
        );

        let mut unreadable = walk(Vec::new());
        unreadable.errors = vec!["events.ndjson.2: Permission denied".to_owned()];
        assert!(only(&unreadable).contains("Permission denied"));

        // All of them at once, and all of them on the page — the `!is_empty()`
        // guard in `page_of` must not drop the block.
        let mut every = walk(vec![rec(1, telemetry::Level::Warn, &[("bars", 1)])]);
        every.hit_scan_cap = true;
        every.reached_oldest = false;
        every.partial_tail = true;
        every.malformed = 3;
        every.errors = vec!["events.ndjson.2: Permission denied".to_owned()];
        assert_eq!(walk_notes(&every, None).len(), 5, "five flags, five notes");
        let page = page_of(
            &every,
            &asked("limit=10"),
            None,
            (std::path::Path::new("/tmp/x"), None),
        );
        for want in [
            "NOT everything there is",
            "Older events exist",
            "mid-line",
            "3 line(s)",
            "Permission denied",
        ] {
            assert!(page.contains(want), "the page dropped {want}: {page}");
        }
        assert!(
            !page_of(
                &walk(Vec::new()),
                &asked(""),
                None,
                (std::path::Path::new("/tmp/x"), None)
            )
            .contains("<ul class=\"notes\">"),
            "and a clean walk renders no note list at all"
        );

        // THE PAIR IS RENDERED AS A PAIR. A page that names one of two
        // directories lets a reader conclude the other was empty, and `None`
        // has to SAY it was never opened rather than say nothing at all.
        let both = page_of(
            &walk(Vec::new()),
            &asked(""),
            None,
            (
                std::path::Path::new("/tmp/served"),
                Some(std::path::Path::new("/tmp/served/cli")),
            ),
        );
        assert!(
            both.contains("<code>/tmp/served</code>")
                && both.contains("<code>/tmp/served/cli</code>"),
            "both halves of the walk are named on the page, and each in its own \
             element — one path that happens to be a prefix of the other must \
             not be able to satisfy this by itself: {both}"
        );
        assert!(
            !both.contains("was NOT read"),
            "and a half that WAS read is not announced as missing: {both}"
        );
        assert!(
            page_of(
                &walk(Vec::new()),
                &asked(""),
                None,
                (std::path::Path::new("/tmp/served"), None)
            )
            .contains("the cli half was NOT read"),
            "an unresolved store root is named, not rendered as an empty half"
        );
    }

    /// The row rendering: field separators, and the two truncation marks.
    #[test]
    fn a_row_separates_its_fields_and_marks_what_was_not_kept() {
        let two = walk(vec![rec(
            1,
            telemetry::Level::Error,
            &[("bars", 806), ("gaps", 2)],
        )]);
        let html = rows_table(&two);
        assert!(
            html.contains("<b>bars</b>=806 <b>gaps</b>=2"),
            "exactly one space between fields, none leading: {html}"
        );
        assert!(
            rows_table(&walk(vec![rec(1, telemetry::Level::Info, &[("bars", 1)])]))
                .contains("<td class=\"wide\"><b>bars</b>=1<"),
            "and no leading space when there is only one"
        );

        // `dropped_fields > 0`: the zero case must add no mark, which is what
        // separates it from `>= 0`.
        let mut marked = rec(1, telemetry::Level::Warn, &[("bars", 1)]);
        marked.dropped_fields = 4;
        marked.cut = true;
        let html = rows_table(&walk(vec![marked]));
        assert!(html.contains("4 field(s) counted, not kept"), "{html}");
        assert!(html.contains("a value was cut at its ceiling"), "{html}");
        assert!(
            !rows_table(&two).contains("counted, not kept"),
            "no mark when nothing was dropped"
        );
        assert!(!rows_table(&two).contains("cut at its ceiling"));
    }

    /// The level `<select>` marks the level that was asked for, and only it.
    #[test]
    fn the_level_filter_remembers_what_was_asked_for() {
        let page = page_shell(&asked("level=warn"), "");
        assert!(
            page.contains("<option value=\"warn\" selected>warn</option>"),
            "the asked level is the selected one: {page}"
        );
        assert_eq!(
            page.matches(" selected>").count(),
            1,
            "and it is the only one"
        );
        assert_eq!(
            page_shell(&asked(""), "").matches(" selected>").count(),
            0,
            "no level asked for, nothing selected"
        );
    }

    /// The scan cap is a sixteenth of the window, and that is the claim its own
    /// doc comment makes. Pinned as a RELATION so it cannot be restated as a
    /// tautology: change either side and this fails.
    #[test]
    fn the_scan_cap_is_one_sixteenth_of_the_window_the_sink_keeps() {
        let window = u64::from(telemetry::DEFAULT_KEEP_FILES) * telemetry::DEFAULT_MAX_FILE_BYTES;
        assert_eq!(window, 64 * 1024 * 1024, "8 files x 8 MiB");
        assert_eq!(SCAN_BYTES * 16, window, "the doc says the newest sixteenth");
        assert_eq!(SCAN_BYTES, 4_194_304);
    }

    /// **THE HANDLERS FIND THE INSTALLED SINK.**
    ///
    /// `log_dir` and `sink_health` are the two functions that read the process
    /// global, and [`sink_in`] deliberately avoids installing one — so both had
    /// surviving mutants, including `log_dir -> None`, which would render "no
    /// log" on every request forever.
    ///
    /// # It no longer installs its own
    ///
    /// It used to open `brutex-logs-installed-<pid>` at the DEFAULT floor and
    /// call `telemetry::install` itself. That was correct while it was the only
    /// install in the binary and stopped being correct the moment a second test
    /// needed the global: `install` refuses a second call, so whichever test ran
    /// first decided the floor for the whole process — and at the default `Info`
    /// floor every `Debug` and `Trace` site writes nothing, so a test asserting
    /// against one of them would have been asserting the winner of a race.
    /// [`crate::emitted::sink`] is now the single owner, at `Trace`, and this
    /// test asserts against it. The claims below are unchanged.
    #[test]
    fn the_page_reads_the_directory_and_the_health_off_the_installed_sink() {
        let live = crate::emitted::sink();
        assert_eq!(
            log_dir().as_deref(),
            live.path().parent(),
            "the page must read the directory off the sink, never recompute it"
        );
        let health = sink_health().expect("an installed sink has health");
        assert_eq!(
            health.path,
            live.path(),
            "and the health is that same sink's"
        );
        assert!(
            !health.is_loud(),
            "and the sink every other test shares has lost nothing: {health:?}"
        );
    }

    /// **THE TWO HANDLERS THIS MODULE EXISTS FOR, ACTUALLY EXECUTED.**
    ///
    /// A coverage run on 2026-08-10 put `logs_json` and `logs_page` at **0%** —
    /// every line of both, under both compilations of this crate. Their only
    /// reference anywhere was the route registration in `server.rs`. Every test
    /// above drives `json_over` / `page_over`, which were split out *precisely*
    /// so a test could own the directory — and the split left the handlers, the
    /// part an operator's browser actually reaches, unexecuted.
    ///
    /// What only the handler holds, and what nothing else could have caught:
    /// the `log_dir()` resolution off the installed sink, the `503` when there
    /// is none, the `Content-Type`, and the fact that the query string reaches
    /// `asked` at all.
    ///
    /// Uses [`crate::emitted::sink`] rather than installing, because
    /// `telemetry::install` refuses a second call and this binary already has
    /// an owner for its process global.
    #[tokio::test]
    async fn the_handlers_answer_over_the_installed_sink_and_carry_their_content_type() {
        let sink = crate::emitted::sink();
        assert!(
            sink.emit(
                &telemetry::Event::error("api.handler", "handler probe")
                    .with("probe", telemetry::Value::Str("handler-probe-marker")),
            )
            .is_written(),
            "the premise: this binary has a sink and it accepted a line"
        );

        // /logs.json — status, content type, and a body that PARSES.
        let uri: axum::http::Uri = "/logs.json?limit=50&target=api.handler"
            .parse()
            .expect("a legal uri");
        let (status, headers, body) = logs_json(uri).await;
        assert_eq!(status, axum::http::StatusCode::OK);
        assert_eq!(
            headers[0].1, "application/json; charset=utf-8",
            "a JSON body must not be served as text/plain"
        );
        assert!(
            body.contains("handler-probe-marker"),
            "the query string reached `asked` and the walk found this test's own \
             record — which is the only thing that proves the handler is wired \
             to the installed sink rather than to nothing: {body}"
        );
        assert!(
            body.starts_with(r#"{"records":["#) && body.ends_with('}'),
            "and the document is whole"
        );

        // /logs — the same instant, as a page.
        let uri: axum::http::Uri = "/logs?limit=50&target=api.handler&level=error"
            .parse()
            .expect("a legal uri");
        let page = logs_page(uri).await.0;
        assert!(page.contains("handler-probe-marker"), "{page}");
        assert!(
            page.contains("<title>brutex · logs</title>"),
            "the shell is rendered, not just the rows"
        );
        assert!(
            page.contains("<option value=\"error\" selected>"),
            "and the level the URL asked for is the one the form remembers"
        );
    }

    /// A query string the handler cannot honour is still answered, not refused.
    ///
    /// Clamping rather than a `400` is this module's stated rule — "a log viewer
    /// that answers a mangled bookmark with a 400 is a log viewer an operator
    /// stops reaching for" — and until now that rule was only exercised through
    /// `asked`, never through the door an operator's browser knocks on.
    #[tokio::test]
    async fn a_mangled_query_string_still_gets_an_answer_from_the_handler() {
        let _sink = crate::emitted::sink();
        let uri: axum::http::Uri = "/logs.json?limit=-9999999&level=NOPE&target="
            .parse()
            .expect("a legal uri");
        let (status, _headers, body) = logs_json(uri).await;
        assert_eq!(
            status,
            axum::http::StatusCode::OK,
            "a mangled bookmark is not an error"
        );
        assert!(
            body.contains(r#""limit":50"#),
            "the mangled limit fell back to the default rather than refusing: {body}"
        );

        let uri: axum::http::Uri = "/logs?limit=99999999".parse().expect("a legal uri");
        let page = logs_page(uri).await.0;
        assert!(
            page.contains(&format!("value=\"{PAGE_LIMIT}\"")),
            "a limit past the page ceiling is clamped to it, and the form shows \
             the number actually used: {page}"
        );
    }

    /// **THE "AS JSON" LINK REPRODUCES THE PAGE, NOT THE DEFAULT.**
    ///
    /// The link used to carry only the target, and to HTML-escape it. Two
    /// separate wrongs in one expression: an operator who filtered to
    /// `level=error&limit=200` and followed the link got the unfiltered default
    /// 50, and a target containing `&` was entity-escaped into something a
    /// browser sends as a literal `&` — which the server then reads as a
    /// parameter separator, silently truncating the filter.
    ///
    /// This walks the round trip the way a browser does: render the page,
    /// extract the `href`, entity-decode the attribute, parse it back through
    /// the same `asked` the handler uses, and demand the three fields match.
    #[test]
    fn the_json_link_carries_the_whole_filter_and_percent_encodes_it() {
        for raw in [
            "limit=200&level=error&target=pull.member",
            "limit=7&target=pull",
            "level=warn",
            "",
        ] {
            let page_asked = asked(raw);
            let page = page_shell(&page_asked, "");

            let at = page.find("/logs.json").expect("the link is on the page");
            let end = page[at..].find('"').expect("the attribute closes") + at;
            let href = &page[at..end];
            // What a browser actually sends: the attribute, entity-decoded.
            let sent = href
                .replace("&amp;", "&")
                .replace("&lt;", "<")
                .replace("&gt;", ">")
                .replace("&quot;", "\"")
                .replace("&#39;", "'");
            let link_asked = asked(sent.split_once('?').map_or("", |(_, q)| q));

            assert_eq!(link_asked.limit, page_asked.limit, "limit lost: {sent}");
            assert_eq!(link_asked.level, page_asked.level, "level lost: {sent}");
            assert_eq!(link_asked.target, page_asked.target, "target lost: {sent}");
        }
    }

    /// A target carrying the two characters that break a URL survives the link.
    ///
    /// `&` is the parameter separator and `=` the assignment; percent-encoding
    /// is what stops the value being read as structure. HTML escaping — which is
    /// what this did — leaves both intact after the browser decodes the
    /// attribute, so the filter was silently cut at the first `&`.
    #[test]
    fn a_target_holding_a_url_separator_is_percent_encoded_not_entity_escaped() {
        let page_asked = asked("target=a%26b%3Dc&limit=9");
        assert_eq!(page_asked.target, "a&b=c", "the premise: it decoded");

        let link = json_query(&page_asked);
        assert!(
            link.contains("target=a%26b%3Dc"),
            "the separator must be percent-encoded in the link: {link}"
        );
        assert!(
            !link.contains("&amp;"),
            "and never entity-escaped, which a browser would hand back as a \
             literal separator: {link}"
        );
        assert_eq!(
            asked(link.trim_start_matches('?')).target,
            "a&b=c",
            "so the value round-trips whole"
        );
    }

    /// **THE RUN FILTER IS REACHABLE FROM A URL AND FROM THE FORM.**
    ///
    /// `Query::from_run` existed for an hour with no caller on the HTTP
    /// surface — the same "built and unreachable" shape this module's own
    /// header records about `telemetry::tail`, and that D-0081 records about
    /// per-target levels. A filter nobody can ask for is not a feature.
    ///
    /// The round trip matters as much as the parse: an operator who narrows to
    /// one backfill and then follows "as JSON" must not silently get every run
    /// back, which is exactly the defect the level and limit had.
    #[test]
    fn a_run_can_be_asked_for_by_url_and_survives_the_json_link() {
        let narrowed = asked("run=1786197791427&limit=25");
        assert_eq!(narrowed.run, 1_786_197_791_427);
        assert_eq!(
            narrowed.query.run,
            Some(1_786_197_791_427),
            "and it reaches the walk, not just the struct"
        );

        // ZERO AND ABSENT BOTH MEAN EVERY RUN. There is no run zero to ask for
        // — an event outside a backfill omits the key — so the value is free to
        // mean the absence of a filter.
        assert_eq!(asked("").run, 0);
        assert_eq!(asked("").query.run, None, "absent is no filter");
        assert_eq!(asked("run=0").query.run, None, "and zero is no filter");
        assert_eq!(
            asked("run=notanumber").query.run,
            None,
            "a mangled run is no filter, not a refusal — this page clamps"
        );

        // THE ROUND TRIP, the way a browser makes it.
        let page = page_shell(&narrowed, "");
        let at = page.find("/logs.json").expect("the link is on the page");
        let end = page[at..].find('"').expect("the attribute closes") + at;
        let sent = page[at..end].replace("&amp;", "&");
        let back = asked(sent.split_once('?').map_or("", |(_, q)| q));
        assert_eq!(
            back.run, narrowed.run,
            "following the link must not widen the view back to every run: {sent}"
        );
        assert_eq!(back.limit, narrowed.limit);

        // And the form remembers it, so it is reachable without hand-editing a URL.
        assert!(
            page.contains(r#"name="run" value="1786197791427""#),
            "the form shows the run in force: {page}"
        );
        assert!(
            page_shell(&asked(""), "").contains(r#"name="run" value=""#),
            "and is empty when every run is shown"
        );
    }
}

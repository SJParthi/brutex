//! A sweep started from the browser, and what it is doing while it runs.
//!
//! # What this closes
//!
//! `/backtest` could show every recorded run and could not cause one. The
//! operator read the console, decided to try another rung, left for a terminal,
//! typed `cli range-all`, waited, and came back to refresh. A console that
//! reports on work it cannot start is half a console, and the missing half is
//! the one that makes the other useful.
//!
//! # The shape is [`crate::pullrun`]'s, deliberately
//!
//! That module already solved this problem for the vendor pull: one slot on the
//! site behind a mutex, `Some(progress)` with no `finished` meaning a run is in
//! flight, a background task that owns the work, and a JSON route the page
//! polls. Inventing a second shape for the same problem would leave two
//! answers to "is something running", and the second one to be updated would be
//! wrong. This is the same shape with a different payload.
//!
//! # Why the work happens in a task and not in the handler
//!
//! A sweep is seconds to hours. `docs/HANDOVER` measures a one-day span at
//! 43 ms and a fifteen-minute span at 4.7% support at 390 s per month — so a
//! seven-year span at a fine rung is a request no browser will wait for and no
//! reverse proxy will hold open. The handler returns as soon as the slot is
//! claimed; everything after that is the task's.
//!
//! # ONE WRITER TO THE LEDGER, AND THIS IS THE SECOND
//!
//! `cli::results` is append-only with no file lock: it seeks to the end and
//! writes. That is safe while exactly one process appends. This module makes
//! the server a second appender, so a browser-started sweep finishing at the
//! same moment as a hand-run `cli` can interleave two records.
//!
//! **The slot below does not prevent that** — it is one server's slot, and the
//! terminal does not consult it. What it prevents is two sweeps from THIS
//! process. The cross-process case is named in `docs/06-limits.md` rather than
//! papered over, because a lock file is a store-format change and belongs with
//! the crate that owns the format.

use std::fmt::Write as _;

/// What one browser-started sweep is doing.
///
/// Held in `Site::sweep` behind a mutex. `finished == None` is the one reading
/// of "in flight", exactly as [`crate::pullrun::Progress`] uses it, and a
/// finished run is LEFT in the slot rather than cleared so the page can read
/// its summary after it ends.
#[derive(Clone, Debug)]
pub struct Progress {
    /// The feed word the sweep was asked for.
    pub feed: String,
    /// The instrument.
    pub underlying: String,
    /// First month of the span.
    pub from: (u16, u8),
    /// Last month of the span.
    pub to: (u16, u8),
    /// Support threshold, in parts per million of bars.
    pub support_ppm: u64,
    /// When the run was accepted, microseconds since the epoch.
    pub started_micros: i64,
    /// When it ended, or [`None`] while it is still going.
    pub finished_micros: Option<i64>,
    /// The report `cli` produced, once there is one.
    pub report: Option<String>,
    /// Why it could not run, when that is the answer.
    pub refusal: Option<String>,
}

impl Progress {
    /// A run just accepted, with nothing decided yet.
    #[must_use]
    pub fn started(
        feed: &str,
        underlying: &str,
        from: (u16, u8),
        to: (u16, u8),
        support_ppm: u64,
        now_micros: i64,
    ) -> Self {
        Self {
            feed: feed.to_owned(),
            underlying: underlying.to_owned(),
            from,
            to,
            support_ppm,
            started_micros: now_micros,
            finished_micros: None,
            report: None,
            refusal: None,
        }
    }

    /// Whether this run is still going.
    #[must_use]
    pub const fn in_flight(&self) -> bool {
        self.finished_micros.is_none()
    }

    /// This run as the JSON `/backtest/run.json` answers with.
    #[must_use]
    pub fn to_json(&self) -> String {
        let mut out = String::with_capacity(512);
        out.push('{');
        let _ = write!(out, r#""feed":{}"#, crate::render::json_string(&self.feed));
        let _ = write!(
            out,
            r#","underlying":{}"#,
            crate::render::json_string(&self.underlying)
        );
        let _ = write!(out, r#","from_year":{}"#, self.from.0);
        let _ = write!(out, r#","from_month":{}"#, self.from.1);
        let _ = write!(out, r#","to_year":{}"#, self.to.0);
        let _ = write!(out, r#","to_month":{}"#, self.to.1);
        let _ = write!(out, r#","support_ppm":{}"#, self.support_ppm);
        let _ = write!(out, r#","started_micros":{}"#, self.started_micros);
        let _ = write!(out, r#","in_flight":{}"#, self.in_flight());
        match self.finished_micros {
            Some(at) => {
                let _ = write!(out, r#","finished_micros":{at}"#);
            }
            None => out.push_str(r#","finished_micros":null"#),
        }
        match self.report {
            Some(ref text) => {
                let _ = write!(out, r#","report":{}"#, crate::render::json_string(text));
            }
            None => out.push_str(r#","report":null"#),
        }
        match self.refusal {
            Some(ref why) => {
                let _ = write!(out, r#","refusal":{}"#, crate::render::json_string(why));
            }
            None => out.push_str(r#","refusal":null"#),
        }
        out.push('}');
        out
    }
}

/// Everything a request has to get right before a sweep may start.
///
/// Each is a REFUSAL with a sentence rather than a clamp. A pull can sensibly
/// clamp a page size; a sweep cannot sensibly guess a span, and a run recorded
/// under a span the operator did not ask for is worse than no run at all —
/// `CLAUDE.md` §3 rule 3 makes the span part of the run's identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The body was not the four fields this route needs.
    Malformed(String),
    /// A month outside `1..=12`, or a `to` before its `from`.
    Span(String),
    /// A support threshold of zero, which would make every combination
    /// frequent and the ladder infinite.
    Support(String),
    /// A run is already in flight in this process.
    Busy(String),
}

impl Refusal {
    /// The sentence an operator reads.
    #[must_use]
    pub fn why(&self) -> &str {
        match *self {
            Self::Malformed(ref s)
            | Self::Span(ref s)
            | Self::Support(ref s)
            | Self::Busy(ref s) => s,
        }
    }

    /// The status this refusal answers with.
    ///
    /// `Busy` is 409 and the rest are 400: a second press is a CONFLICT with
    /// work already happening, not a malformed request, and answering it 400
    /// would tell the operator to fix a body that is perfectly good.
    #[must_use]
    pub const fn status(&self) -> axum::http::StatusCode {
        match *self {
            Self::Busy(_) => axum::http::StatusCode::CONFLICT,
            _ => axum::http::StatusCode::BAD_REQUEST,
        }
    }
}

/// What one `POST /backtest/run` body asked for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Asked {
    /// The feed's directory word.
    pub feed: String,
    /// The instrument.
    pub underlying: String,
    /// First month of the span.
    pub from: (u16, u8),
    /// Last month of the span.
    pub to: (u16, u8),
    /// Support threshold in parts per million of bars.
    pub support_ppm: u64,
}

/// One field out of a flat JSON object, as text.
///
/// A hand parser rather than a dependency: this body has five scalar fields and
/// `crate::pullrun::legs_from` already parses its own by hand for the same
/// reason. Nothing here has to survive nesting.
fn field(body: &str, name: &str) -> Option<String> {
    let key = format!("\"{name}\"");
    let at = body.find(&key)? + key.len();
    let rest = body.get(at..)?.trim_start();
    let rest = rest.strip_prefix(':')?.trim_start();
    if let Some(text) = rest.strip_prefix('"') {
        let end = text.find('"')?;
        return text.get(..end).map(str::to_owned);
    }
    let end = rest.find([',', '}']).unwrap_or(rest.len());
    Some(rest.get(..end)?.trim().to_owned())
}

/// The request body, or the first thing wrong with it.
///
/// # Errors
///
/// A missing or unparseable field, a month outside `1..=12`, a `to` before its
/// `from`, or a support threshold of zero.
pub fn asked_from(body: &str) -> Result<Asked, Refusal> {
    let feed = field(body, "feed")
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            Refusal::Malformed(
                "no `feed` in the request. A run is stamped with the feed its bars came from — \
             CLAUDE.md §3 rule 3 makes it one of the nine terms in the run's identity — so \
             it cannot be defaulted."
                    .to_owned(),
            )
        })?;
    let underlying = field(body, "underlying")
        .filter(|s| !s.is_empty())
        .ok_or_else(|| Refusal::Malformed("no `underlying` in the request.".to_owned()))?;

    let num = |name: &str| -> Result<u64, Refusal> {
        field(body, name)
            .and_then(|s| s.parse::<u64>().ok())
            .ok_or_else(|| Refusal::Malformed(format!("`{name}` is missing or not a number.")))
    };
    let from_year = num("from_year")?;
    let from_month = num("from_month")?;
    let to_year = num("to_year")?;
    let to_month = num("to_month")?;
    let support_ppm = num("support_ppm")?;

    let month_ok = |m: u64| (1..=12).contains(&m);
    if !month_ok(from_month) || !month_ok(to_month) {
        return Err(Refusal::Span(format!(
            "months must be 1..=12; this asked for {from_month} and {to_month}."
        )));
    }
    let from_key = from_year * 12 + from_month;
    let to_key = to_year * 12 + to_month;
    if to_key < from_key {
        return Err(Refusal::Span(format!(
            "the span ends before it starts: {from_year}-{from_month:02} to \
             {to_year}-{to_month:02}. A backwards span is not an empty one, so it is \
             refused rather than swept as nothing."
        )));
    }
    if support_ppm == 0 {
        return Err(Refusal::Support(
            "a support threshold of zero makes every combination frequent, so the ladder \
             never empties and the walk has no end. CLAUDE.md §6: depth is decided by \
             extinction, and extinction needs a threshold above zero."
                .to_owned(),
        ));
    }

    // `u16`/`u8` by construction: the month is checked above and a year past
    // u16 is not a year this store can file a month under.
    let year = |y: u64, name: &str| -> Result<u16, Refusal> {
        u16::try_from(y).map_err(|_| Refusal::Span(format!("`{name}` is not a year: {y}.")))
    };
    let month = |m: u64| -> u8 { u8::try_from(m).unwrap_or(1) };

    Ok(Asked {
        feed,
        underlying,
        from: (year(from_year, "from_year")?, month(from_month)),
        to: (year(to_year, "to_year")?, month(to_month)),
        support_ppm,
    })
}

/// Runs the sweep and records what it produced.
///
/// **Blocking on purpose, and the caller must place it accordingly.**
/// `cli::range_all` is CPU-bound over millions of bars; running it directly on
/// a tokio worker would hold that thread for the whole sweep and starve every
/// other request on it. The handler puts this on a blocking thread.
///
/// The report `cli` returns is kept whole. It is the same text `cli range-all`
/// prints in a terminal, including the `STORED_PROVENANCE` banner that says
/// the bars were REAL — `CLAUDE.md` §5 makes that banner the only thing
/// separating a real sweep from a generated one, so it travels with the report
/// rather than being stripped for the page.
#[must_use]
pub fn conduct(asked: &Asked, now_micros: i64) -> Progress {
    let mut progress = Progress::started(
        &asked.feed,
        &asked.underlying,
        asked.from,
        asked.to,
        asked.support_ppm,
        now_micros,
    );
    let report = cli::range_all(
        &asked.feed,
        &asked.underlying,
        asked.from,
        asked.to,
        asked.support_ppm,
    );
    progress.report = Some(report);
    progress.finished_micros = Some(now_micros);
    progress
}

/// Microseconds since the epoch, or zero when the clock is before it.
///
/// Zero rather than a panic: a clock behind the epoch is a broken machine, and
/// refusing to record a completed sweep because of it would throw away real
/// work. The page renders a zero stamp as `not stamped`.
#[must_use]
pub fn now_micros() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|d| i64::try_from(d.as_micros()).ok())
        .unwrap_or(0)
}

/// The JSON content type both routes answer with.
type JsonHeaders = [(axum::http::HeaderName, &'static str); 1];

fn json_headers() -> JsonHeaders {
    [(
        axum::http::header::CONTENT_TYPE,
        "application/json; charset=utf-8",
    )]
}

/// `POST /backtest/run` — start a sweep, and answer before it finishes.
///
/// # Cost
///
/// **O(1) on the request path, and the sweep is not on it.** The handler takes
/// one uncontended lock, reads one `Option`, writes one, and spawns. It does
/// not touch the store, the ledger or a bar file. The sweep that follows is
/// the brute force itself — `CLAUDE.md` §6 makes its depth a matter of
/// extinction rather than a parameter, so its cost is the work, not the route
/// — and it runs on a blocking thread where it cannot starve a worker.
///
/// # Why it answers immediately
///
/// A one-day span is 43 ms and a fifteen-minute span at 4.7% support is 390 s
/// per month. Holding the connection would exceed every proxy timeout in the
/// path and give the operator a spinner with no way to ask what it was doing.
/// The page polls [`run_json`] instead, which is one lock take.
pub async fn run(
    axum::extract::State(site): axum::extract::State<crate::server::Loaded>,
    body: String,
) -> (axum::http::StatusCode, JsonHeaders, String) {
    let asked = match asked_from(&body) {
        Ok(asked) => asked,
        Err(why) => {
            let _ = telemetry::emit_if!(
                telemetry::Level::Warn,
                "api.sweep",
                "a sweep was refused before it started",
                "why" => telemetry::Value::Str(why.why()),
            );
            return (
                why.status(),
                json_headers(),
                format!(
                    r#"{{"accepted":false,"refusal":{}}}"#,
                    crate::render::json_string(why.why())
                ),
            );
        }
    };

    // THE SLOT IS CLAIMED UNDER THE LOCK AND THE WORK STARTS OUTSIDE IT.
    // Holding a std mutex across an await is the deadlock this pattern exists
    // to avoid, so the guard is dropped before anything is spawned.
    {
        let mut held = match site.sweep.lock() {
            Ok(held) => held,
            Err(poisoned) => poisoned.into_inner(),
        };
        if held.as_ref().is_some_and(Progress::in_flight) {
            let why = "a sweep is already running in this process. It appends to the same \
                       append-only ledger, and two of them finishing together can interleave \
                       two records, so the second press is refused rather than queued."
                .to_owned();
            let _ = telemetry::emit_if!(
                telemetry::Level::Warn,
                "api.sweep",
                "a second sweep was refused while one was in flight",
                "why" => telemetry::Value::Str(&why),
            );
            return (
                Refusal::Busy(String::new()).status(),
                json_headers(),
                format!(
                    r#"{{"accepted":false,"refusal":{}}}"#,
                    crate::render::json_string(&why)
                ),
            );
        }
        *held = Some(Progress::started(
            &asked.feed,
            &asked.underlying,
            asked.from,
            asked.to,
            asked.support_ppm,
            now_micros(),
        ));
    }

    // ONE EVENT PER RUN, NOT ONE PER BAR. Gate 17 silences `vocab engine
    // indicators runner` because those hold the loops; this is the boundary
    // where an event is affordable, and the granularity D-0226 prescribes.
    let _ = telemetry::emit_if!(
        telemetry::Level::Info,
        "api.sweep",
        "a sweep was accepted from the browser",
        "feed" => telemetry::Value::Str(&asked.feed),
        "underlying" => telemetry::Value::Str(&asked.underlying),
        "from_year" => telemetry::Value::Uint(u64::from(asked.from.0)),
        "from_month" => telemetry::Value::Uint(u64::from(asked.from.1)),
        "to_year" => telemetry::Value::Uint(u64::from(asked.to.0)),
        "to_month" => telemetry::Value::Uint(u64::from(asked.to.1)),
        "support_ppm" => telemetry::Value::Uint(asked.support_ppm),
    );

    // A BLOCKING THREAD, NOT A WORKER. `cli::range_all` is CPU-bound over
    // millions of bars; on a worker it would hold that thread for the whole
    // sweep and every other request sharing it would wait.
    let held_site = std::sync::Arc::clone(&site);
    let started = now_micros();
    tokio::task::spawn_blocking(move || {
        let finished = conduct(&asked, started);
        let mut slot = match held_site.sweep.lock() {
            Ok(slot) => slot,
            Err(poisoned) => poisoned.into_inner(),
        };
        let elapsed = now_micros().saturating_sub(started);
        let _ = telemetry::emit_if!(
            telemetry::Level::Info,
            "api.sweep",
            "a sweep finished and its record is in the ledger",
            "feed" => telemetry::Value::Str(&finished.feed),
            "underlying" => telemetry::Value::Str(&finished.underlying),
            "elapsed_micros" => telemetry::Value::Uint(elapsed.max(0).unsigned_abs()),
        );
        let mut done = finished;
        done.finished_micros = Some(now_micros());
        *slot = Some(done);
    });

    (
        axum::http::StatusCode::ACCEPTED,
        json_headers(),
        r#"{"accepted":true,"refusal":null}"#.to_owned(),
    )
}

/// `GET /backtest/run.json` — what the sweep is doing, for the page to poll.
///
/// **O(1).** One lock take and one struct read. It never consults the store,
/// so an operator refreshing this every second through an hour-long sweep
/// costs the disk nothing.
pub async fn run_json(
    axum::extract::State(site): axum::extract::State<crate::server::Loaded>,
) -> (axum::http::StatusCode, JsonHeaders, String) {
    let held = match site.sweep.lock() {
        Ok(held) => held,
        Err(poisoned) => poisoned.into_inner(),
    };
    let body = held.as_ref().map_or_else(
        // NO SWEEP IS A STATE, NOT AN ABSENCE. A page that got `null` would
        // have to decide what that meant; this says it.
        || r#"{"running":null,"why":"no sweep has been started from this console"}"#.to_owned(),
        |progress| format!(r#"{{"running":{}}}"#, progress.to_json()),
    );
    (axum::http::StatusCode::OK, json_headers(), body)
}

#[cfg(test)]
#[expect(
    clippy::expect_used,
    reason = "the exception every test module in this workspace takes: a test \
              that cannot panic cannot fail"
)]
mod tests {
    use super::{Asked, Progress, Refusal, asked_from, field, now_micros};

    fn body(feed: &str, span: &str, support: &str) -> String {
        format!(r#"{{"feed":"{feed}","underlying":"NIFTY",{span},"support_ppm":{support}}}"#)
    }

    const SPAN: &str = r#""from_year":2019,"from_month":12,"to_year":2026,"to_month":8"#;

    /* ==================== the body parser ==================== */

    #[test]
    fn a_whole_body_parses_into_the_five_things_a_sweep_needs() {
        let asked = asked_from(&body("zerodha", SPAN, "200000")).expect("a good body");
        assert_eq!(
            asked,
            Asked {
                feed: "zerodha".to_owned(),
                underlying: "NIFTY".to_owned(),
                from: (2019, 12),
                to: (2026, 8),
                support_ppm: 200_000,
            }
        );
    }

    #[test]
    fn whitespace_between_the_key_and_its_value_is_tolerated() {
        // A hand-written body, or one from a formatter, is not malformed.
        let raw = r#"{ "feed" : "dhan" , "underlying" : "BANKNIFTY" ,
            "from_year" : 2020 , "from_month" : 1 ,
            "to_year" : 2020 , "to_month" : 3 , "support_ppm" : 50000 }"#;
        let asked = asked_from(raw).expect("a spaced body");
        assert_eq!(asked.feed, "dhan");
        assert_eq!(asked.underlying, "BANKNIFTY");
        assert_eq!(asked.from, (2020, 1));
        assert_eq!(asked.support_ppm, 50_000);
    }

    #[test]
    fn a_missing_feed_is_refused_because_it_names_the_run() {
        let raw = format!(r#"{{"underlying":"NIFTY",{SPAN},"support_ppm":200000}}"#);
        let why = asked_from(&raw).expect_err("a refusal");
        assert!(matches!(why, Refusal::Malformed(_)));
        assert!(why.why().contains("identity"), "{}", why.why());
        assert_eq!(why.status(), axum::http::StatusCode::BAD_REQUEST);
    }

    #[test]
    fn an_empty_feed_is_as_absent_as_a_missing_one() {
        let why = asked_from(&body("", SPAN, "200000")).expect_err("a refusal");
        assert!(matches!(why, Refusal::Malformed(_)));
    }

    #[test]
    fn a_missing_underlying_is_refused() {
        let raw = format!(r#"{{"feed":"zerodha",{SPAN},"support_ppm":200000}}"#);
        let why = asked_from(&raw).expect_err("a refusal");
        assert!(why.why().contains("underlying"), "{}", why.why());
    }

    #[test]
    fn every_numeric_field_is_required_and_named_when_absent() {
        for missing in [
            "from_year",
            "from_month",
            "to_year",
            "to_month",
            "support_ppm",
        ] {
            let raw = body("zerodha", SPAN, "200000").replace(missing, "x_gone");
            let why = asked_from(&raw).expect_err("a refusal");
            assert!(
                why.why().contains(missing),
                "the refusal must name the field it wanted: {}",
                why.why()
            );
        }
    }

    #[test]
    fn a_number_that_is_not_a_number_is_refused_rather_than_defaulted() {
        let why = asked_from(&body("zerodha", SPAN, "\"lots\"")).expect_err("a refusal");
        assert!(matches!(why, Refusal::Malformed(_)));
        assert!(why.why().contains("support_ppm"), "{}", why.why());
    }

    #[test]
    fn a_month_outside_one_to_twelve_is_refused() {
        for span in [
            r#""from_year":2019,"from_month":0,"to_year":2026,"to_month":8"#,
            r#""from_year":2019,"from_month":13,"to_year":2026,"to_month":8"#,
            r#""from_year":2019,"from_month":1,"to_year":2026,"to_month":0"#,
            r#""from_year":2019,"from_month":1,"to_year":2026,"to_month":99"#,
        ] {
            let why = asked_from(&body("zerodha", span, "200000")).expect_err("a refusal");
            assert!(matches!(why, Refusal::Span(_)), "{span}");
            assert!(why.why().contains("1..=12"), "{}", why.why());
        }
    }

    #[test]
    fn a_span_that_ends_before_it_starts_is_refused_rather_than_swept_as_nothing() {
        let span = r#""from_year":2026,"from_month":8,"to_year":2019,"to_month":12"#;
        let why = asked_from(&body("zerodha", span, "200000")).expect_err("a refusal");
        assert!(matches!(why, Refusal::Span(_)));
        assert!(why.why().contains("ends before it starts"), "{}", why.why());
    }

    #[test]
    fn a_span_of_one_month_is_legal() {
        // The boundary: `to` EQUAL to `from` is a one-month span, not a
        // backwards one, and refusing it would refuse the cheapest useful run.
        let span = r#""from_year":2026,"from_month":8,"to_year":2026,"to_month":8"#;
        let asked = asked_from(&body("zerodha", span, "200000")).expect("one month");
        assert_eq!(asked.from, asked.to);
    }

    #[test]
    fn a_month_earlier_in_a_later_year_is_still_forward() {
        // 2019-12 -> 2020-01 crosses a year with a SMALLER month, which a
        // naive month-only comparison would call backwards.
        let span = r#""from_year":2019,"from_month":12,"to_year":2020,"to_month":1"#;
        assert!(asked_from(&body("zerodha", span, "200000")).is_ok());
    }

    #[test]
    fn a_support_threshold_of_zero_is_refused_because_the_ladder_would_not_end() {
        let why = asked_from(&body("zerodha", SPAN, "0")).expect_err("a refusal");
        assert!(matches!(why, Refusal::Support(_)));
        assert!(why.why().contains("extinction"), "{}", why.why());
        assert_eq!(why.status(), axum::http::StatusCode::BAD_REQUEST);
    }

    #[test]
    fn a_year_past_u16_is_refused_rather_than_truncated() {
        let span = r#""from_year":99999999,"from_month":1,"to_year":99999999,"to_month":2"#;
        let why = asked_from(&body("zerodha", span, "200000")).expect_err("a refusal");
        assert!(matches!(why, Refusal::Span(_)));
        assert!(why.why().contains("not a year"), "{}", why.why());
    }

    #[test]
    fn a_busy_refusal_is_a_conflict_and_not_a_bad_request() {
        // A second press is a CONFLICT with work already happening. Answering
        // 400 would tell the operator to fix a body that is perfectly good.
        let busy = Refusal::Busy("one is running".to_owned());
        assert_eq!(busy.status(), axum::http::StatusCode::CONFLICT);
        assert_eq!(busy.why(), "one is running");
    }

    #[test]
    fn the_field_reader_handles_both_shapes_and_gives_up_cleanly() {
        assert_eq!(field(r#"{"a":"text"}"#, "a").as_deref(), Some("text"));
        assert_eq!(field(r#"{"a":42}"#, "a").as_deref(), Some("42"));
        assert_eq!(field(r#"{"a":42,"b":1}"#, "a").as_deref(), Some("42"));
        assert_eq!(field(r#"{"a":"x"}"#, "b"), None);
        assert_eq!(field("", "a"), None);
        // A key with no colon after it is not a field.
        assert_eq!(field(r#"{"a" "x"}"#, "a"), None);
        // An unterminated string is not a value.
        assert_eq!(field(r#"{"a":"x"#, "a"), None);
    }

    /* ==================== progress ==================== */

    #[test]
    fn a_started_run_is_in_flight_until_it_is_finished() {
        let mut p = Progress::started("zerodha", "NIFTY", (2019, 12), (2026, 8), 200_000, 42);
        assert!(p.in_flight());
        assert_eq!(p.started_micros, 42);
        assert_eq!(p.finished_micros, None);
        assert_eq!(p.report, None);
        p.finished_micros = Some(99);
        assert!(!p.in_flight());
    }

    #[test]
    fn the_progress_json_carries_every_field_and_nulls_what_has_not_happened() {
        let p = Progress::started("zerodha", "NIFTY", (2019, 12), (2026, 8), 200_000, 42);
        let json = p.to_json();
        for fragment in [
            r#""feed":"zerodha""#,
            r#""underlying":"NIFTY""#,
            r#""from_year":2019"#,
            r#""from_month":12"#,
            r#""to_year":2026"#,
            r#""to_month":8"#,
            r#""support_ppm":200000"#,
            r#""started_micros":42"#,
            r#""in_flight":true"#,
            r#""finished_micros":null"#,
            r#""report":null"#,
            r#""refusal":null"#,
        ] {
            assert!(json.contains(fragment), "missing {fragment} in {json}");
        }
    }

    #[test]
    fn a_finished_run_carries_its_report_and_its_stamp() {
        let mut p = Progress::started("zerodha", "NIFTY", (2020, 1), (2020, 1), 50_000, 1);
        p.finished_micros = Some(500);
        p.report = Some("STORED_PROVENANCE\nrows".to_owned());
        let json = p.to_json();
        assert!(json.contains(r#""in_flight":false"#), "{json}");
        assert!(json.contains(r#""finished_micros":500"#), "{json}");
        assert!(json.contains("STORED_PROVENANCE"), "{json}");
        // The report is JSON-escaped, so its newline does not break the body.
        assert!(!json.contains("PROVENANCE\nrows"), "the newline is escaped");
    }

    #[test]
    fn a_refusal_reaches_the_progress_json_as_a_sentence() {
        let mut p = Progress::started("zerodha", "NIFTY", (2020, 1), (2020, 1), 50_000, 1);
        p.refusal = Some("the store held no bars".to_owned());
        assert!(p.to_json().contains("the store held no bars"));
    }

    #[test]
    fn the_clock_is_after_the_epoch_and_does_not_panic() {
        // Zero is the honest answer on a machine whose clock is before the
        // epoch; anything else here would be a real timestamp.
        assert!(now_micros() > 0);
    }
}

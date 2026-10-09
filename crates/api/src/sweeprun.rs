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
//! Result appends use file locks, and the in-process slot excludes a second
//! browser command. `cli::execution_lease` now also holds one store-scoped OS
//! lease throughout each cooperating CLI/HTTP command and its terminal audit.
//! Historical telemetry remains outcome evidence; it does not own that lease.
//! Older binaries and callers bypassing these entry points do not participate.

use std::fmt::Write as _;

/// Which of the two commands a slot is holding.
///
/// # Why the page has to be told, rather than inferring it
///
/// Both commands write the same [`Progress`] into the same slot, because both
/// append to the same append-only ledger and two of them finishing together can
/// interleave two records — the refusal the slot exists to give is the same
/// refusal for both. But they answer DIFFERENT QUESTIONS, and a page that
/// rendered the descent's walk under the sweep's heading would be labelling a
/// hunt for a rare setup as a nine-rung comparison.
///
/// `support_ppm` cannot carry it either: a sweep fixes that number and a descent
/// WALKS it, so the field means "the threshold" in one case and "where the walk
/// began" in the other. Encoding the difference in a magic value of a numeric
/// field is the shape `CLAUDE.md` §4 bans.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// `cli range-all` — every rung at one fixed support.
    Sweep,
    /// `cli elite` with the threshold walked — one rung, support descended.
    Descent,
    /// One of the five stored-data commands `/engine/command` serves.
    Command,
}

impl Kind {
    /// The word this kind travels to the page as.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Sweep => "sweep",
            Self::Descent => "descent",
            Self::Command => "command",
        }
    }
}

/// What one browser-started sweep or descent is doing.
///
/// Held in `Site::sweep` behind a mutex. `finished == None` is the one reading
/// of "in flight", exactly as [`crate::pullrun::Progress`] uses it, and a
/// finished run is LEFT in the slot rather than cleared so the page can read
/// its summary after it ends.
///
/// **One slot serves both commands**, because both append to the same
/// append-only ledger — see [`Kind`] for what that costs the page and how it is
/// paid.
#[derive(Clone, Debug)]
pub struct Progress {
    /// Which command produced this run.
    pub kind: Kind,
    /// The feed word the sweep was asked for.
    pub feed: String,
    /// The instrument.
    pub underlying: String,
    /// First month of the span.
    pub from: (u16, u8),
    /// Last month of the span.
    pub to: (u16, u8),
    /// Support threshold, in parts per million of bars.
    pub support_ppm: Option<u64>,
    /// When the run was accepted, microseconds since the epoch.
    pub started_micros: i64,
    /// Opaque exact attempt token shared by status and structural events.
    pub attempt: u64,
    /// When it ended, or [`None`] while it is still going.
    pub finished_micros: Option<i64>,
    /// The report `cli` produced, once there is one.
    ///
    /// SHARED, NOT OWNED: every `/backtest/run.json` poll clones the whole
    /// [`Progress`] while it holds the slot's std mutex on an async worker, and
    /// a `sweep-all` report grows by a line per instrument-month. As a `String`
    /// that clone copied every byte of the report under the lock on every poll;
    /// as an `Arc<str>` it is one reference-count increment whatever the
    /// report's length, as
    /// `api::sweeprun::a_poll_snapshot_shares_the_report_and_refusal_bytes_with_the_slot`
    /// shows. W1-api6-1, D-0954.
    pub report: Option<std::sync::Arc<str>>,
    /// Why it could not run, when that is the answer. Shared for the reason
    /// [`Self::report`] is: a refusal can carry the report it replaced.
    pub refusal: Option<std::sync::Arc<str>>,
    /// Exact declared Boolean request and journal observations, when applicable.
    pub boolean_search: Option<Box<crate::booleanlaunch::Status>>,
    /// Additive single-stop request and pinned journal progress, when applicable.
    pub index_stop: Option<Box<crate::indexstoplaunch::Status>>,
}

impl Progress {
    /// A run just accepted, with nothing decided yet.
    #[must_use]
    pub fn started(
        feed: &str,
        underlying: &str,
        from: (u16, u8),
        to: (u16, u8),
        support_ppm: Option<u64>,
        now_micros: i64,
        attempt: u64,
    ) -> Self {
        Self {
            // THE DEFAULT IS THE COMMAND THAT EXISTED FIRST, and a descent
            // overrides it through `of_kind`. Eight call sites build a
            // `Progress` and seven of them mean a sweep; widening the signature
            // for the eighth would edit seven correct lines to say what they
            // already say.
            kind: Kind::Sweep,
            feed: feed.to_owned(),
            underlying: underlying.to_owned(),
            from,
            to,
            support_ppm,
            started_micros: now_micros,
            attempt,
            finished_micros: None,
            report: None,
            refusal: None,
            boolean_search: None,
            index_stop: None,
        }
    }

    /// The same run, marked as the command that actually produced it.
    #[must_use]
    pub const fn of_kind(mut self, kind: Kind) -> Self {
        self.kind = kind;
        self
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
        // FIRST, because it decides how every field after it should be read —
        // `support_ppm` is a fixed threshold on a sweep and the ceiling a walk
        // STARTED from on a descent.
        let _ = write!(out, r#""kind":"{}","where":"browser""#, self.kind.word());
        let _ = write!(out, r#","feed":{}"#, crate::render::json_string(&self.feed));
        let _ = write!(
            out,
            r#","underlying":{}"#,
            crate::render::json_string(&self.underlying)
        );
        let _ = write!(out, r#","from_year":{}"#, self.from.0);
        let _ = write!(out, r#","from_month":{}"#, self.from.1);
        let _ = write!(out, r#","to_year":{}"#, self.to.0);
        let _ = write!(out, r#","to_month":{}"#, self.to.1);
        match self.support_ppm {
            Some(ppm) => {
                let _ = write!(out, r#","support_ppm":{ppm}"#);
            }
            None => out.push_str(r#","support_ppm":null"#),
        }
        let _ = write!(out, r#","started_micros":{}"#, self.started_micros);
        let _ = write!(out, r#","attempt":{}"#, self.attempt);
        // Preserve the exact u64 correlation key beyond JavaScript's safe
        // integer range. The numeric field remains for existing readers.
        let _ = write!(out, r#","attempt_key":"{}""#, self.attempt);
        let _ = write!(out, r#","in_flight":{}"#, self.in_flight());
        if let Some(search) = self.boolean_search.as_ref()
            && let Some(fields) = search.fields().as_object()
        {
            for (key, value) in fields {
                let _ = write!(out, ",{}:{value}", crate::render::json_string(key));
            }
        }
        if let Some(search) = self.index_stop.as_ref() {
            let mut fields = search.fields();
            if let Some(object) = fields.as_object_mut() {
                let elapsed = self
                    .finished_micros
                    .unwrap_or_else(now_micros)
                    .checked_sub(self.started_micros)
                    .and_then(|value| u64::try_from(value).ok());
                object.insert(
                    "elapsed_micros".into(),
                    serde_json::json!(elapsed.map(|value| value.to_string())),
                );
                object.insert("elapsed_basis".into(), serde_json::json!("wall-clock"));
            }
            let _ = write!(
                out,
                r#","command":{},"index_stop":{}"#,
                crate::render::json_string(crate::indexstoplaunch::COMMAND),
                fields
            );
        }
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
    /// A run is already in flight in this process.
    Busy(String),
    /// This binary carries no commit stamp, so no run may be recorded.
    ///
    /// Not the caller's fault and not a bad body — the SERVER cannot do the
    /// work in the state it was built in, which is what 503 says and 400 does
    /// not.
    Unstamped(String),
    /// No exact telemetry attempt key can be allocated.
    Unobservable(String),
    /// This server's own environment sets something no recorded run may take,
    /// so every run the route could start would refuse.
    ///
    /// 503 for the reason [`Self::Unstamped`] gives: the body is fine and
    /// nothing is in flight, and the fix -- unset it and restart -- is on the
    /// server's side.
    Environment(String),
    //
    // THERE WAS A `Support` VARIANT HERE AND IT IS GONE. It refused a
    // threshold of zero, which makes every combination frequent so the
    // frontier never empties and the walk has no end. The refusal was right
    // and it is now unreachable: the threshold is `SUPPORT_PPM`, a constant,
    // and no request can name it.
    //
    // Deleted rather than kept as a variant nothing constructs. A dead arm is
    // a region that can never run, which is the 100% coverage floor `CLAUDE.md`
    // §9 sets, and it would also tell the next reader that a request can still
    // get this wrong. It cannot.
}

impl Refusal {
    /// The sentence an operator reads.
    #[must_use]
    pub fn why(&self) -> &str {
        match *self {
            Self::Malformed(ref s)
            | Self::Span(ref s)
            | Self::Busy(ref s)
            | Self::Unstamped(ref s)
            | Self::Unobservable(ref s)
            | Self::Environment(ref s) => s,
        }
    }

    /// The status this refusal answers with.
    ///
    /// `Busy` is 409, `Unstamped`, `Unobservable` and `Environment` are 503,
    /// and the rest are 400. A second press
    /// is a CONFLICT with work already happening, not a malformed request, and
    /// answering it 400 would tell the operator to fix a body that is perfectly
    /// good. An unstamped build is neither: the body is fine and nothing is in
    /// flight — this SERVER cannot record a run in the state it was built in,
    /// and 503 is the code that says the fix is on this side.
    #[must_use]
    pub const fn status(&self) -> axum::http::StatusCode {
        match *self {
            Self::Busy(_) => axum::http::StatusCode::CONFLICT,
            Self::Unstamped(_) | Self::Unobservable(_) | Self::Environment(_) => {
                axum::http::StatusCode::SERVICE_UNAVAILABLE
            }
            _ => axum::http::StatusCode::BAD_REQUEST,
        }
    }
}

/// The refusal an unstamped build owes the operator, before any bar is read.
///
/// # The cost of finding out late
///
/// `CLAUDE.md` §3 rule 3 puts `commit` in every run's identity, and
/// `cli::commit_stamp` resolves `option_env!("BRUTEX_COMMIT")` **at compile
/// time** — a runtime read would stamp a result with a commit whose source
/// never produced it. `None` means the build was not stamped, and `cli` refuses
/// every recorded run on that basis. Correctly.
///
/// What it does not do is refuse EARLY. The gate sits inside `audit_range`, so
/// a browser sweep claimed the slot, spawned its thread, loaded nine spans and
/// refused nine times — once per rung — for a fact that was decided when the
/// binary was compiled. Measured: `cli audit-range` over 121 months of daily
/// bars refused on exactly this, and the answer never depended on a single bar
/// it read.
///
/// This is the same shape as D-0296's ledger check, one layer further out: a
/// condition knowable before the work, answered before the work.
///
/// # Why it takes the stamp rather than reading it
///
/// `cargo test` WAS an unstamped build — `crates/runner`'s own doc records that
/// this is why `run_ranked` shipped with zero coverage. A guard that called
/// `commit_stamp()` directly would therefore have one arm the suite can never
/// reach, which is the 100% floor §9 sets. Taking the value makes both arms
/// ordinary.
///
/// # That premise is no longer true, and the design outlives it
///
/// `crates/cli/build.rs` has stamped `BRUTEX_COMMIT` from `.git/HEAD` at compile
/// time since `086149d5`, and it stamps the TEST HARNESS as well as the binary,
/// so on any machine with a `.git` both this crate's tests and `cli`'s are
/// stamped. The unreachable arm is now the OTHER one.
///
/// Taking the stamp as an argument is still right, and more clearly so: it is
/// the only way either arm is reachable, whichever way the build happens to be
/// stamped, and it does not depend on a fact about the toolchain that a file in
/// another crate can change without touching this one. It did exactly that —
/// `cli`'s `a_well_formed_range_refuses_at_the_identity_gate_and_says_so` read
/// the stamp implicitly, and when `build.rs` landed 120 commits later that test
/// silently began sweeping 618,296 real bars and appending to the operator's
/// ledger. Every reader of the stamp should take it, not read it.
fn stamp_refusal(stamp: Option<&str>) -> Option<Refusal> {
    if stamp.is_some_and(cli::is_canonical_commit_stamp) {
        return None;
    }
    Some(Refusal::Unstamped(
        "this server was built without one canonical, verified BRUTEX_COMMIT, so no sweep it runs can \
         be recorded: CLAUDE.md §3 rule 3 makes the commit part of every run's \
         identity, and it is read at COMPILE time so it cannot be filled in \
         now. Nothing was swept, because the answer did not depend on any bar. \
         Restore every Rust/Cargo input to HEAD (normally by committing the \
         intended change), rebuild, and restart the server. An explicit \
         BRUTEX_COMMIT is accepted only when it exactly equals clean HEAD."
            .to_owned(),
    ))
}

/// The refusal a usable `BRUTEX_SCREEN_BUDGET_MS` in this server's own
/// environment owes a route whose run prices the exit-grid screen, before the
/// run slot, the execution lease and the run's invocation record. D-0685.
///
/// # Why here, when `cli` already refuses it
///
/// `cli::one_rung` and the stored kernels refuse a usable budget on every run
/// that records, and they are right to. But they refuse inside the run, and the
/// run starts only after this crate has claimed the in-process slot, taken the
/// execution lease and written an invocation record. A budget set in the
/// server's environment cannot change while it runs, so every press would take
/// the slot and write the store for a refusal no wait can fix -- the case
/// [`stamp_refusal`] already refuses early for an unstamped build.
///
/// # The predicate is `cli`'s own, called and not restated. D-0695.
///
/// This used to restate `cli`'s rule -- the same reader, then trimmed, a `u64`,
/// above zero -- because the function that holds it was private. A restated
/// rule is a second authority for one fact: the first change to `cli`'s
/// `positive_count` (a ceiling, say) would have left this copy behind, and a
/// route would then answer 503 to a value the engine accepts.
/// [`cli::recorded_budget_refusal`] is public now, and this asks it, as
/// [`stamp_refusal`] asks [`cli::is_canonical_commit_stamp`]. A value that rule
/// cannot use is no budget -- `cli` names it under `KNOB REFUSED` and runs
/// without one -- so it is not refused here either.
///
/// # What the sentence claims, and what it no longer does
///
/// It said "nothing was written". On the production router that was false the
/// moment it was sent: `operation_audit::note_request` journals every sweep
/// route's request (D-0568) -- its start before the handler runs, its 503
/// after. What this refusal keeps unwritten is the RUN's state: no slot, no
/// lease, no run invocation record. That is all the sentence says now, and it
/// names the journal row that is written. D-0695.
fn environment_budget_refusal() -> Option<Refusal> {
    cli::recorded_budget_refusal().is_err().then(|| {
        Refusal::Environment(format!(
            "BRUTEX_SCREEN_BUDGET_MS is set in this server's environment, and is refused: \
             {BUDGET_NOT_RECORDABLE}. The engine refuses it on every run that records \
             (D-0685). Refused before the run slot, the execution lease and the run's \
             invocation record were taken, so no run started; this server's HTTP request \
             journal still records the request itself (D-0568). Unset it and restart the \
             server, and bound the screen with BRUTEX_SCREEN_CAP, a stated count."
        ))
    })
}

/// Why a recorded run takes no screen budget, in this file's one wording.
///
/// Both budget refusals here state it -- [`refuse_screen_budget`] for a body
/// that names the field, [`environment_budget_refusal`] for this server's own
/// environment -- and each adds its own subject and its own remedy. It was
/// written out once in each, as two different sentences for one reason, which
/// a review of D-0695 upheld as two sources of wording for one fact.
/// `cli`'s private `SCREEN_BUDGET_NOT_RECORDABLE` gives the engine's own
/// refusal in `cli`'s words; it names no route, so it is not this sentence.
/// D-0695.
const BUDGET_NOT_RECORDABLE: &str = "every run this route starts is recorded, and a screen \
     budget derives how many candidates are priced from a \
     wall-clock calibration the run identity cannot name, so one identity could \
     record different answers";

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
    /// The rungs to sweep, in `cli::EVERY_RUNG`'s own order.
    ///
    /// # Why this is a `Vec` and not an `Option`
    ///
    /// It is never empty. An absent `rungs` field means every rung, which is
    /// what this route did before it could be asked anything else, so an old
    /// client keeps its exact behaviour — and a body that names an empty list
    /// is refused at the parse rather than being quietly widened back to all
    /// eight. `None` and `[]` would otherwise both have to mean "all", and one
    /// of them would be a caller who asked for nothing and got everything.
    ///
    /// The entries are `&'static str` borrowed from [`cli::EVERY_RUNG`],
    /// because `cli::one_rung` stores the name in the row it returns. That
    /// makes an unknown rung unrepresentable here rather than checked later:
    /// a name that does not match the table cannot be put in this field at
    /// all.
    pub rungs: Vec<&'static str>,
    /// The engine knobs this request sets, as `(BRUTEX_NAME, value)`.
    ///
    /// # Why the request carries them at all
    ///
    /// Because otherwise they live in the PROCESS ENVIRONMENT, and a knob in
    /// the environment can only be changed by restarting the process.
    ///
    /// MEASURED, 2026-08-29: the operator's server was started from their IDE
    /// with no `BRUTEX_` variables whatsoever, so every knob took its built-in
    /// default — and two of those defaults compose into a run that cannot
    /// finish. `screen_cap` defaults to 10,000 and `validate` defaults to ON
    /// (`raw.is_none_or(|v| v.trim() != "0")` — *unset means true*), so the
    /// page's own Run Sweep button would have priced twenty times more
    /// combinations than the previous run and put walk-forward, PBO and the
    /// bootstrap on every one. The button was reachable. The configuration was
    /// not.
    ///
    /// # The table below IS the allowlist
    ///
    /// [`KNOBS`] maps a request field to a `BRUTEX_` name, and a name absent
    /// from it cannot be set however the body is written. `BRUTEX_STORE` and
    /// `BRUTEX_LOG_DIR` are deliberately absent: they name the process's own
    /// files rather than this run's parameters, and an HTTP request must not be
    /// able to move where this engine reads bars from or writes its log.
    pub knobs: Vec<(&'static str, String)>,
}

/// Every knob a sweep request may set, as `(request field, BRUTEX name)`.
///
/// Named in the request in the page's own vocabulary rather than by their
/// environment spelling, so the browser sends `"support_ppm": 100000` and not a
/// shell variable. The mapping is one array index per field — `CLAUDE.md` §3
/// rule 4's constant cost, with fourteen as the bound.
///
/// Ordered as an operator reads them: what to search, how much of it to price,
/// how much to keep, then the admission rules.
///
/// # `screen_budget_ms` was here, and is now refused by name -- D-0685
///
/// Every run these routes start records. A screen budget derives how many
/// candidates are priced from a wall-clock calibration that the run identity
/// cannot name, so one identity could record two different answers. The field
/// is therefore not in this table, and [`refuse_screen_budget`] refuses a body
/// that names it rather than letting serde drop it: a budget the operator typed
/// that silently did nothing would be the fallback `CLAUDE.md` §4 bans.
/// `screen_cap` remains, because it is a stated count the identity folds.
const KNOBS: [(&str, &str); 15] = [
    ("support_ppm", "BRUTEX_SUPPORT_PPM"),
    ("ceiling", "BRUTEX_CEILING"),
    ("screen_cap", "BRUTEX_SCREEN_CAP"),
    ("top", "BRUTEX_TOP"),
    ("validate", "BRUTEX_VALIDATE"),
    ("horizon_bars", "BRUTEX_HORIZON_BARS"),
    ("grid_rungs", "BRUTEX_GRID_RUNGS"),
    ("grid_resolution", "BRUTEX_GRID_RESOLUTION"),
    ("sizing_rate_bp", "BRUTEX_SIZING_RATE_BP"),
    ("min_rr_bp", "BRUTEX_MIN_RR_BP"),
    ("min_win_rate_bp", "BRUTEX_MIN_WIN_RATE_BP"),
    ("min_trades", "BRUTEX_MIN_TRADES"),
    ("min_ret_over_dd_bp", "BRUTEX_MIN_RET_OVER_DD_BP"),
    ("min_weakest_bp", "BRUTEX_MIN_WEAKEST_BP"),
    ("max_mae_ppm", "BRUTEX_MAX_MAE_PPM"),
];

/// One scalar accepted by the browser engine request schema.
///
/// Numeric strings remain accepted because the original route accepted them,
/// while arrays, objects, null and fractional numbers are rejected by the JSON
/// decoder before any engine setting can silently disappear. `serde_json` owns
/// JSON number syntax; the semantic integer bounds remain with the existing
/// field parsers below.
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(untagged)]
enum WireScalar {
    /// A JSON string, decoded including escapes.
    Text(String),
    /// A non-negative JSON integer.
    Unsigned(u64),
    /// A negative JSON integer.
    Signed(i64),
    /// A JSON boolean, used by `validate`.
    Boolean(bool),
}

impl WireScalar {
    fn text(&self) -> String {
        match *self {
            Self::Text(ref value) => value.clone(),
            Self::Unsigned(value) => value.to_string(),
            Self::Signed(value) => value.to_string(),
            Self::Boolean(value) => value.to_string(),
        }
    }
}

/// Distinguishes an absent JSON member from a present value.
///
/// `Option<T>` cannot do that: serde maps both a missing member and an explicit
/// `null` to `None`. That would make `"rungs": null` indistinguishable from an
/// omitted list and silently widen it to every rung. A present member therefore
/// deserializes directly as `T`; `null` fails that type instead of becoming
/// [`Self::Missing`].
#[derive(Clone, Debug, Default)]
enum WireField<T> {
    /// The object did not contain this member.
    #[default]
    Missing,
    /// The object contained one value of the declared type.
    Value(T),
}

impl<T> WireField<T> {
    fn as_ref(&self) -> Option<&T> {
        match *self {
            Self::Missing => None,
            Self::Value(ref value) => Some(value),
        }
    }
}

impl<'de, T> serde::Deserialize<'de> for WireField<T>
where
    T: serde::Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        T::deserialize(deserializer).map(Self::Value)
    }
}

/// The union of the three browser-engine request bodies.
///
/// Serde's struct decoder is the boundary: the input must be one complete JSON
/// object, every declared field has one JSON type, and a repeated declared key
/// is a duplicate-field error. Unknown fields remain ignored for compatibility
/// with newer clients and with the existing store/log-dir safety test.
#[derive(Clone, Debug, Default, serde::Deserialize)]
#[serde(default)]
struct WireBody {
    feed: WireField<String>,
    underlying: WireField<String>,
    from_year: WireField<WireScalar>,
    from_month: WireField<WireScalar>,
    to_year: WireField<WireScalar>,
    to_month: WireField<WireScalar>,
    rungs: WireField<Vec<String>>,
    command: WireField<String>,
    rung: WireField<String>,
    min_hits: WireField<WireScalar>,
    max_points: WireField<WireScalar>,
    support_ppm: WireField<WireScalar>,
    ceiling: WireField<WireScalar>,
    screen_cap: WireField<WireScalar>,
    /// ANY JSON VALUE, because naming the field is the error. Decoded as a
    /// `WireScalar`, a `null`, a fraction, an array, an object or an integer
    /// past `u64::MAX` failed the decode first and got the decoder's generic
    /// sentence instead of [`refuse_screen_budget`]'s. D-0685.
    screen_budget_ms: WireField<serde::de::IgnoredAny>,
    top: WireField<WireScalar>,
    validate: WireField<WireScalar>,
    horizon_bars: WireField<WireScalar>,
    grid_rungs: WireField<WireScalar>,
    grid_resolution: WireField<WireScalar>,
    sizing_rate_bp: WireField<WireScalar>,
    min_rr_bp: WireField<WireScalar>,
    min_win_rate_bp: WireField<WireScalar>,
    min_trades: WireField<WireScalar>,
    min_ret_over_dd_bp: WireField<WireScalar>,
    min_weakest_bp: WireField<WireScalar>,
    max_mae_ppm: WireField<WireScalar>,
}

impl WireBody {
    fn string(&self, name: &str) -> Option<&str> {
        match name {
            "feed" => self.feed.as_ref().map(String::as_str),
            "underlying" => self.underlying.as_ref().map(String::as_str),
            "command" => self.command.as_ref().map(String::as_str),
            "rung" => self.rung.as_ref().map(String::as_str),
            _ => None,
        }
    }

    fn scalar(&self, name: &str) -> Option<&WireScalar> {
        match name {
            "from_year" => self.from_year.as_ref(),
            "from_month" => self.from_month.as_ref(),
            "to_year" => self.to_year.as_ref(),
            "to_month" => self.to_month.as_ref(),
            "min_hits" => self.min_hits.as_ref(),
            "max_points" => self.max_points.as_ref(),
            "support_ppm" => self.support_ppm.as_ref(),
            "ceiling" => self.ceiling.as_ref(),
            "screen_cap" => self.screen_cap.as_ref(),
            "top" => self.top.as_ref(),
            "validate" => self.validate.as_ref(),
            "horizon_bars" => self.horizon_bars.as_ref(),
            "grid_rungs" => self.grid_rungs.as_ref(),
            "grid_resolution" => self.grid_resolution.as_ref(),
            "sizing_rate_bp" => self.sizing_rate_bp.as_ref(),
            "min_rr_bp" => self.min_rr_bp.as_ref(),
            "min_win_rate_bp" => self.min_win_rate_bp.as_ref(),
            "min_trades" => self.min_trades.as_ref(),
            "min_ret_over_dd_bp" => self.min_ret_over_dd_bp.as_ref(),
            "min_weakest_bp" => self.min_weakest_bp.as_ref(),
            "max_mae_ppm" => self.max_mae_ppm.as_ref(),
            _ => None,
        }
    }
}

/// Refuses a body that names `screen_budget_ms`, whatever its value. D-0685.
///
/// Kept in [`WireBody`] only so that it can be refused: dropped from the schema,
/// serde would ignore it and a budget the operator typed would silently do
/// nothing. Any value refuses, an empty one included, because this setting
/// cannot be set at all -- naming it is the error.
fn refuse_screen_budget(body: &WireBody) -> Result<(), Refusal> {
    if body.screen_budget_ms.as_ref().is_some() {
        return Err(Refusal::Malformed(format!(
            "`screen_budget_ms` is refused: {BUDGET_NOT_RECORDABLE}. Omit it and bound \
             the screen with `screen_cap`, a stated count. No setting was ignored."
        )));
    }
    Ok(())
}

/// Parse exactly one complete JSON object before reading any field from it.
fn wire_body(body: &str) -> Result<WireBody, Refusal> {
    serde_json::from_str(body).map_err(|why| {
        Refusal::Malformed(format!(
            "the request body must be one complete JSON object with no duplicate known field: {why}"
        ))
    })
}

/// Read every knob the body names, in [`KNOBS`] order.
///
/// # Why `validate` is normalised and the rest are not
///
/// `cli::validates` is `raw.is_none_or(|v| v.trim() != "0")`, so **any** string
/// that is not exactly `"0"` means ON — and JSON's own `false` is the string
/// `"false"`, which is not `"0"`. A page sending `"validate": false` would
/// therefore turn validation ON, which is the precise opposite of what it
/// asked. Every other knob is a number whose text parses the same on both
/// sides, so only this one needs the translation.
fn knobs_in(body: &WireBody) -> Vec<(&'static str, String)> {
    let mut out: Vec<(&'static str, String)> = Vec::with_capacity(KNOBS.len());
    for (asked, name) in KNOBS {
        let Some(raw) = field(body, asked) else {
            continue;
        };
        let value = if name == "BRUTEX_VALIDATE" {
            match raw.trim().to_ascii_lowercase().as_str() {
                "0" | "false" | "off" | "no" => "0".to_owned(),
                other => other.to_owned(),
            }
        } else {
            raw.trim().to_owned()
        };
        // An empty value is ABSENCE. `cli::knobs::set` makes the same reading,
        // and this skips the round trip rather than relying on it.
        if !value.is_empty() {
            out.push((name, value));
        }
    }
    out
}

/// Every member [`WireBody`] declares except `screen_budget_ms`, which
/// [`refuse_screen_budget`] refuses on every route before this list is read.
///
/// # Why a route refuses a KNOWN field it does not read -- P3-01-02, D-1972
///
/// [`WireBody`] is the union of three routes' bodies, so every route decoded
/// every member and read only its own. `{"rung":"5min"}` on `/backtest/run`,
/// which reads `rungs`, swept all eight rungs; `"validate":"0"` on a descent,
/// which applies no knob, was validated and dropped. Each route now names the
/// members it reads and refuses any other KNOWN member by name, as
/// [`strict_knobs`] already did for its one word. Unknown members stay ignored
/// for compatibility (BE-03, D-0685): they are not settings anyone here offers.
const WIRE_FIELDS: [&str; 26] = [
    "feed",
    "underlying",
    "from_year",
    "from_month",
    "to_year",
    "to_month",
    "rungs",
    "command",
    "rung",
    "min_hits",
    "max_points",
    "support_ppm",
    "ceiling",
    "screen_cap",
    "top",
    "validate",
    "horizon_bars",
    "grid_rungs",
    "grid_resolution",
    "sizing_rate_bp",
    "min_rr_bp",
    "min_win_rate_bp",
    "min_trades",
    "min_ret_over_dd_bp",
    "min_weakest_bp",
    "max_mae_ppm",
];

/// What a span-taking command word reads: its word, feed, instrument, span and
/// one rung.
const COMMAND_SPAN_RUNG: [&str; 8] = [
    "command",
    "feed",
    "underlying",
    "from_year",
    "from_month",
    "to_year",
    "to_month",
    "rung",
];

/// [`COMMAND_SPAN_RUNG`] and a `min_hits` count.
const COMMAND_SPAN_RUNG_HITS: [&str; 9] = [
    "command",
    "feed",
    "underlying",
    "from_year",
    "from_month",
    "to_year",
    "to_month",
    "rung",
    "min_hits",
];

/// What `screen` reads: [`COMMAND_SPAN_RUNG`] and its three own numbers.
const COMMAND_SCREEN: [&str; 11] = [
    "command",
    "feed",
    "underlying",
    "from_year",
    "from_month",
    "to_year",
    "to_month",
    "rung",
    "support_ppm",
    "max_points",
    "top",
];

/// What `sweep-all` reads. `cli sweep-all VENDOR RUNG MIN_HITS` takes no
/// instrument and no span, so a body naming either is refused: a scoped-looking
/// request must not become a whole-store batch (P3-02-06, D-1972).
const COMMAND_SWEEP_ALL: [&str; 4] = ["command", "feed", "rung", "min_hits"];

/// Whether a route applies the [`KNOBS`] it is sent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Knobs {
    /// Every [`KNOBS`] member is read (and validated by the route's own rule).
    Applied,
    /// None is read beyond the members the route names itself.
    NotApplied,
}

/// Refuses, by name, the first known member `route` does not read.
///
/// `reads` lists the members the route consumes; with [`Knobs::Applied`] every
/// [`KNOBS`] member is read as well. See [`WIRE_FIELDS`] for why. D-1972.
fn refuse_unread(
    body: &WireBody,
    route: &str,
    reads: &[&str],
    knobs: Knobs,
) -> Result<(), Refusal> {
    for name in WIRE_FIELDS {
        let present = body.string(name).is_some()
            || body.scalar(name).is_some()
            || list_field(body, name).is_some();
        let read = reads.contains(&name)
            || (knobs == Knobs::Applied && KNOBS.iter().any(|&(knob, _)| knob == name));
        if present && !read {
            return Err(Refusal::Malformed(format!(
                "`{name}` is not read by {route}, so it is refused rather than \
                 dropped: a setting that silently did nothing would be the \
                 fallback CLAUDE.md §4 bans. Omit it. No setting was ignored."
            )));
        }
    }
    Ok(())
}

/// What one `POST /backtest/descend` body asked for.
///
/// # Three fields the sweep does not take, and why each is a FACT and not a knob
///
/// `SUPPORT_PPM`'s doc argues that the operator asks for an instrument and a
/// span because those are facts about what they want to study, while a support
/// percentage is a knob on the machine. These three pass that test:
///
/// * **`rung`** — a descent walks ONE rung's threshold. Which timeframe you are
///   hunting on is the question, not a setting.
/// * **`max_points`** — the widest adverse excursion the operator will accept,
///   stated the way a stop is spoken. It is their risk, not the machine's.
/// * **`top`** — how many rows to be shown. A display bound.
///
/// **The support is still absent, and that is the whole point of the command.**
/// `cli::elite_descend` derives its floor from what the statistics can support
/// and walks down to it, so the number §6 refuses to let anyone type is the one
/// number this route also never accepts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AskedDescent {
    /// The feed's directory word.
    pub feed: String,
    /// The instrument.
    pub underlying: String,
    /// The single rung whose threshold is walked.
    pub rung: String,
    /// First month of the span.
    pub from: (u16, u8),
    /// Last month of the span.
    pub to: (u16, u8),
    /// The stop ceiling, in whole index points. Zero is no ceiling (D-1732).
    pub max_points: i64,
    /// How many rows the listing shows.
    pub top: usize,
}

/* ==================================================================
   THERE IS NO SUPPORT CONSTANT HERE, AND ITS ABSENCE IS THE FEATURE
   ==================================================================

   A `const SUPPORT_PPM: u64 = 200_000` stood at this line and the argument for
   it is kept below, because every sentence of it was right about the PARAMETER
   and wrong about the CONSTANT.

   §6 refuses a settable depth because "a parameter that can be set can be set
   wrongly and silently". A constant has the same defect one step earlier: it
   was set wrongly ONCE, by whoever wrote it, and then nobody could see it at
   all. `cli::elite_descend`'s doc records what this one cost -- a run launched
   at a TENTH of 200,000 ppm "cannot report a once-a-week setup no matter how
   long it runs -- the setup was pruned in the first level of the ladder, and
   the report says nothing about it because nothing counted it". The browser's
   only control launched at ten times that.

   The threshold is now neither typed nor baked. `cli::range_all` takes an
   `Option<u64>` and this route passes `None`, so each rung derives its own
   floor from its own bars through `cli::statistical_support_floor`: the lowest
   support at which the stated win rate could still clear its own confidence
   bound. On the shipped Wilson bound that is FOUR round trips, against the
   4,324 hits 20% demanded on the 1-minute rung.

   `Progress::support_ppm` is an `Option` for the same reason, and a sweep
   carries `None` rather than a magic zero: there is no single number to report
   because there is no single number. Nine rungs derive nine floors. D-0303.

   ---- the original argument, kept because it still holds for a PARAMETER ----

   The support threshold, in parts per million of a rung's own bars.

   # It is not a request field, and that is the whole point
///
/// `CLAUDE.md` §6 refuses a depth parameter in these words: *"A parameter that
/// can be set can be set wrongly and silently."* It gives the history — a flag
/// that defaulted to a dynamic token, a width guard that tripped on every real
/// run, and a silent fall back to a hardcoded `k = [1, 2]` that nobody could
/// see from the flag. Support is the same shape of parameter with the same
/// failure available to it: too low and the frequent frontier never empties,
/// too high and the sweep finds nothing and reports that as an answer.
///
/// So the operator asks for an instrument and a span. Those are FACTS about
/// what they want to study. A support percentage is a knob on the machine, and
/// a machine that needs its knobs set by the person asking the question is not
/// finished.
///
/// # What stays dynamic, which is the part that matters
///
/// This is a percentage, never a count. `cli::min_hits_for` turns it into an
/// absolute threshold as `bars × ppm / 1_000_000`, so the number of hits a mask
/// must actually clear is derived per rung, per span, per instrument, at the
/// moment of the run:
///
/// | rung | bars in a span | threshold at 20% |
/// |---|---|---|
/// | 1min | 21,620 | 4,324 |
/// | 15min | 11,643 | 2,328 |
/// | 1day | 1,671 | 334 |
///
/// Nine rungs, nine different thresholds, none of them written down anywhere.
/// A longer span raises it; a coarser rung lowers it. Fixing the PERCENTAGE is
/// what makes those nine comparable — it is the reason `/backtest` can group
/// rungs into one comparison at all, and grouping on the absolute count instead
/// is a bug this page has already had and fixed.
///
/// # 20%, and it is reported rather than hidden
///
/// The value the ladder ran at is written into [`Progress`] and rendered on the
/// page. A constant nobody can see is the hidden default §6 objects to; a
/// constant printed beside its result is a stated condition of the run.
*/

/// One decoded scalar field as the text the existing semantic parsers consume.
fn field(body: &WireBody, name: &str) -> Option<String> {
    body.string(name)
        .map(str::to_owned)
        .or_else(|| body.scalar(name).map(WireScalar::text))
}

/// One decoded list field.
///
/// `None` means the key is absent, which callers read as "not asked for".
/// `Some(vec![])` means the key is present and empty — a DIFFERENT fact, and
/// the one the rung parser refuses rather than widening back to everything.
///
/// The JSON decoder has already required every member to be a string and the
/// closing bracket to exist. A malformed/truncated array therefore never
/// reaches this function and cannot widen to the absent/all-rungs policy.
fn list_field(body: &WireBody, name: &str) -> Option<Vec<String>> {
    match name {
        "rungs" => body.rungs.as_ref().cloned(),
        _ => None,
    }
}

/// The request body, or the first thing wrong with it.
///
/// # Errors
///
/// A missing or unparseable field, a month outside `1..=12`, a `to` before its
/// `from`, a support threshold of zero, or a rung the engine does not sweep.
pub fn asked_from(body: &str) -> Result<Asked, Refusal> {
    let body = wire_body(body)?;
    // REFUSED BEFORE ANY OTHER FIELD IS READ, on this route and on the two
    // other body routes, [`descent_from`] and [`command_from`]. Those apply no
    // knob, and that is why they once dropped the field instead: a budget the
    // operator typed then silently did nothing. D-0685.
    //
    // READ, NOT DECODED: `wire_body` above has already checked every known
    // field's JSON type, so a wrong-typed known field beside the budget
    // (`"feed":5`, `"rungs":"5min"`) is refused as a malformed body before this
    // line runs. The budget itself decodes as any value, so it is never the
    // field that fails the decode. D-0695.
    refuse_screen_budget(&body)?;
    // `rung` (singular) is descend's and command's member: read here it was
    // dropped and the run swept all eight rungs. D-1972.
    refuse_unread(
        &body,
        "`POST /backtest/run`",
        &[
            "feed",
            "underlying",
            "from_year",
            "from_month",
            "to_year",
            "to_month",
            "rungs",
        ],
        Knobs::Applied,
    )?;
    asked_from_wire(&body)
}

/// The feed every body names, or the refusal that says why it is required.
fn feed_from(body: &WireBody) -> Result<String, Refusal> {
    field(body, "feed")
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            Refusal::Malformed(
                "no `feed` in the request. A run is stamped with the feed its bars came from — \
             CLAUDE.md §3 rule 3 makes it one of the nine terms in the run's identity — so \
             it cannot be defaulted."
                    .to_owned(),
            )
        })
}

/// [`asked_from`] after the strict JSON boundary has been crossed once.
fn asked_from_wire(body: &WireBody) -> Result<Asked, Refusal> {
    let feed = feed_from(body)?;
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

    let month_ok = |m: u64| matches!(m, 1..=12);
    if !month_ok(from_month) || !month_ok(to_month) {
        return Err(Refusal::Span(format!(
            "months must be 1..=12; this asked for {from_month} and {to_month}."
        )));
    }
    // BOUNDED BEFORE THE MULTIPLY, NOT AFTER IT. This read
    // `let from_key = from_year * 12 + from_month;` with the `u16` bound below
    // at the `Asked` construction, and the ordering was the whole defect: the
    // multiply saw a `u64` straight off the wire. `overflow-checks = true` is
    // set for `release` as well as `debug`, and `panic = "abort"` is set beside
    // it, so `{"from_year":18446744073709551615}` did not return a refusal --
    // it called `abort()` and took every other in-flight request on the process
    // with it. Any year above `u64::MAX / 12` does it.
    //
    // It could not be caught by the suite, either: `cargo test` builds `dev`,
    // which UNWINDS, so the panic died inside hyper's connection task and only
    // that connection noticed. The abort exists only in the shipped binary, and
    // `a_year_past_u16_is_refused_rather_than_truncated` uses 99,999,999 --
    // eight orders of magnitude below the boundary, so it exercised the `u16`
    // refusal and never the multiply.
    //
    // Bounding first makes the overflow unrepresentable rather than checked:
    // `u16::MAX * 12 + 12` is 786,432, which no `u64` arithmetic can carry out
    // of range. `asked_from` is written as one named refusal per malformed
    // field, and this keeps that shape -- a `saturating_mul` would answer a
    // malformed year with a silently clamped span, which is the fallback that
    // hides a failure `CLAUDE.md` §4 bans.
    let year = |y: u64, name: &str| -> Result<u16, Refusal> {
        u16::try_from(y).map_err(|_| Refusal::Span(format!("`{name}` is not a year: {y}.")))
    };
    let from_y = year(from_year, "from_year")?;
    let to_y = year(to_year, "to_year")?;

    let from_key = u64::from(from_y) * 12 + from_month;
    let to_key = u64::from(to_y) * 12 + to_month;
    if to_key < from_key {
        return Err(Refusal::Span(format!(
            "the span ends before it starts: {from_year}-{from_month:02} to \
             {to_year}-{to_month:02}. A backwards span is not an empty one, so it is \
             refused rather than swept as nothing."
        )));
    }
    // NO SUPPORT CHECK, BECAUSE THERE IS NO SUPPORT FIELD. This used to refuse
    // a threshold of zero -- which makes every combination frequent, so the
    // frontier never empties and the walk has no end. That refusal was correct
    // and it is now unreachable: `SUPPORT_PPM` is a constant above zero and no
    // request can name it. Removing the parameter removed the failure, which is
    // exactly what §6 claims for `k` and is the reason to prefer absence over a
    // validated default.

    // `u8` by construction: the month is checked above. The year is already a
    // `u16` -- it was bounded before the key multiply, which is what that
    // ordering buys and why the bound does not appear here any more.
    let month = |m: u64| -> u8 { u8::try_from(m).unwrap_or(1) };

    // THE RUNGS, RESOLVED AGAINST THE ENGINE'S OWN TABLE.
    //
    // Absent means every rung, which is what this route did before it could be
    // asked anything else — an older client keeps its exact behaviour. Present
    // and EMPTY is a refusal: a sweep over no timeframe is not a sweep, and
    // widening `[]` back to all eight would answer a caller who asked for
    // nothing by giving them everything.
    //
    // Each name is resolved to the `&'static str` in `EVERY_RUNG` rather than
    // kept as the caller's `String`, so an unknown rung cannot reach `Asked`
    // at all. `cli::one_rung` needs `&'static str` for the row it returns, and
    // this is where that requirement is satisfied honestly instead of by an
    // allocation that outlives the request.
    let rungs = match list_field(body, "rungs") {
        None => EVERY_RUNG.to_vec(),
        Some(asked) if asked.is_empty() => {
            return Err(Refusal::Malformed(
                "`rungs` is present and empty. A sweep over no timeframe is not a sweep; \
                 omit the field to sweep every rung."
                    .to_owned(),
            ));
        }
        Some(asked) => {
            let mut out: Vec<&'static str> = Vec::with_capacity(asked.len());
            for name in &asked {
                let Some(known) = EVERY_RUNG.iter().copied().find(|k| *k == name.as_str()) else {
                    return Err(Refusal::Malformed(format!(
                        "`{name}` is not a rung this engine sweeps. The eight are: {}.",
                        EVERY_RUNG.join(", ")
                    )));
                };
                if !out.contains(&known) {
                    out.push(known);
                }
            }
            out
        }
    };

    Ok(Asked {
        feed,
        underlying,
        from: (from_y, month(from_month)),
        to: (to_y, month(to_month)),
        rungs,
        knobs: knobs_in(body),
    })
}

/// The eight rungs a descent may walk — **the engine's own list**.
///
/// # This was a copy, and the copy had drifted
///
/// It read, here, as its own `const`:
///
/// ```text
/// api : 1min, 2min, 3min, 5min, 10min, 15min, 60min, 1day
/// cli : 1min, 2min, 3min, 5min, 10min, 15min, 30min, 60min
/// ```
///
/// **Missing `30min`, and accepting `1day`** — a rung the engine never sweeps,
/// which is on disk to feed `indicators::daily` with the previous session's
/// OHLC. So `POST /backtest/descend` refused a rung the engine walks and
/// accepted one it does not, which is the likeliest route by which a `1day`
/// record reached the results ledger at all.
///
/// The old comment justified the copy: "`cli::EVERY_RUNG` is private and making
/// it public to save eight strings would widen that crate's surface for one
/// caller", and promised "the test below pins it against a refusal from `cli`
/// itself". It did not. `cli::elite_descend_in_points` never checks the rung
/// against `EVERY_RUNG` — it hands the name to `stored::load_span`, whose
/// refusal names the STORE's nine timeframes. So the test asserted that api's
/// eight appear among the store's nine: `1day` is a store timeframe and passed,
/// and a missing `30min` was invisible because the check ran in one direction
/// only.
///
/// That is the two-vocabularies failure `CLAUDE.md` §5 exists to refuse, with
/// its own justification written above it. There is one list now.
pub use cli::EVERY_RUNG;

/// The descent request body, or the first thing wrong with it.
///
/// # Errors
///
/// Everything [`asked_from`] refuses, plus a rung that is not one of the eight,
/// a negative stop ceiling, and a listing bound of zero. A ceiling of zero is
/// no ceiling (D-1732).
pub fn descent_from(body: &str) -> Result<AskedDescent, Refusal> {
    let body = wire_body(body)?;
    // A DESCENT RECORDS AND ITS WALK PRICES THE SCREEN, so a budget named in its
    // body is refused by name here, as [`asked_from`] refuses it, rather than
    // decoded and dropped. D-0685.
    refuse_screen_budget(&body)?;
    // A DESCENT APPLIES NO KNOB. `AskedDescent` has no field for one and
    // `conduct_descent` sets none, so a typed `validate` or `screen_cap` is
    // refused here rather than validated and dropped. D-1972.
    refuse_unread(
        &body,
        "`POST /backtest/descend`",
        &[
            "feed",
            "underlying",
            "from_year",
            "from_month",
            "to_year",
            "to_month",
            "rung",
            "max_points",
            "top",
        ],
        Knobs::NotApplied,
    )?;
    descent_from_wire(&body)
}

/// [`descent_from`] over the same decoded object [`asked_from_wire`] reads.
fn descent_from_wire(body: &WireBody) -> Result<AskedDescent, Refusal> {
    // THE SPAN AND FEED RULES ARE NOT RESTATED, THEY ARE REUSED. Two parsers
    // for one span is two places for a month bound to drift, and the overflow
    // this one already survives -- `{"from_year":18446744073709551615}` calling
    // `abort()` in a release build -- is exactly the kind that comes back when
    // a second copy is written from memory.
    let asked = asked_from_wire(body)?;

    let rung = field(body, "rung")
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            Refusal::Malformed(format!(
                "a descent walks ONE rung's threshold, so `rung` is required. \
                 The eight are: {}.",
                EVERY_RUNG.join(", ")
            ))
        })?;
    if !EVERY_RUNG.contains(&rung.as_str()) {
        return Err(Refusal::Malformed(format!(
            "`{rung}` is not a rung this engine sweeps. The eight are: {}.",
            EVERY_RUNG.join(", ")
        )));
    }

    // A CEILING IS A DISTANCE, SO IT IS PARSED AS ONE AND REFUSED BELOW ZERO.
    // `cli` converts it against the midpoint of the span's own bars; this side
    // never sees a ppm, which is the whole reason that entry point exists.
    //
    // ZERO IS NO CEILING, NOT A REFUSAL (D-1732). This once refused zero as "a
    // ceiling of zero admits no trade". That is false: `Rules::admits` reads a
    // zero `max_mae_ppm` as no ceiling, `grid::Levels::forced` forces nothing
    // at zero, and `cli`'s `USAGE` documents `MAX_POINTS = 0` that way. The
    // browser could not ask for the run the command line offers.
    let max_points = field(body, "max_points")
        .and_then(|text| text.parse::<i64>().ok())
        .ok_or_else(|| {
            Refusal::Malformed(
                "`max_points` must be a whole number of index points — the \
                 widest adverse excursion this run may accept."
                    .to_owned(),
            )
        })?;
    if max_points < 0 {
        return Err(Refusal::Malformed(format!(
            "`max_points` is {max_points}. A negative ceiling is not a \
             distance; 0 means no ceiling beyond the ladder the bars derive."
        )));
    }

    let top = field(body, "top")
        .and_then(|text| text.parse::<usize>().ok())
        .ok_or_else(|| {
            Refusal::Malformed("`top` must be a whole number of rows to list.".to_owned())
        })?;
    if top == 0 {
        return Err(Refusal::Malformed(
            "`top` is 0. A listing of no rows is not a shorter answer, it is \
             no answer."
                .to_owned(),
        ));
    }

    Ok(AskedDescent {
        feed: asked.feed,
        underlying: asked.underlying,
        rung,
        from: asked.from,
        to: asked.to,
        max_points,
        top,
    })
}

/// The word every `cli` refusal opens with.
///
/// `cli`'s own argv layer decides an exit code with `starts_with("refused: ")`.
/// The WORD is matched here and not the word with its colon, because
/// `range_all` is not the only shape a refusal takes — `descend` opens one
/// `"refused at the ceiling"` — and a looser test cannot produce a false
/// positive here: a report opens with `STORED_PROVENANCE`, whose first line is
/// the provenance banner, and never with this word.
const REFUSED: &str = "refused";

/// The answer left behind when a browser-started engine task disappears.
///
/// This is deliberately a refusal and not a report: a panic can happen after
/// the ledger has begun an append, so neither "nothing was written" nor "the
/// run completed" is an honest general claim. The operator is told both facts
/// that are known: this task did not record its final answer, and the result
/// files must be checked before it is retried.
const ABNORMAL_END: &str = "the engine task stopped abnormally before it recorded a final answer — a panic, \
     cancellation, or server shutdown. The in-process slot was released so another \
     run is not blocked forever. Work may have reached the append-only result files \
     before the stop; inspect their integrity before retrying.";

/// Releases the browser engine slot however its background task ends.
///
/// # Why ownership begins before `spawn_blocking`
///
/// Constructing this guard before the closure is handed to Tokio covers both
/// failure windows: a panic while the closure runs, and a runtime shutdown that
/// drops a queued closure before it starts. In either case the captured guard
/// is dropped and an in-flight slot becomes a visible refusal. The panic window
/// exists only where a panic unwinds (`dev`, `test`); `release` sets
/// `panic = "abort"`, so there a panic ends the process instead (poison-1,
/// D-1771).
///
/// # Why normal completion disarms rather than relying on the slot
///
/// A completed slot may be replaced by a new accepted run immediately. If a
/// still-armed old guard then inspected only `Progress::in_flight`, it could
/// mark that NEW run as failed. [`Self::finish`] writes the completed value and
/// disarms while it still owns the guard, so its subsequent `Drop` is inert.
#[must_use = "the finisher must be moved into the background task"]
struct TaskFinisher {
    /// The site whose one shared engine slot this task owns.
    site: crate::server::Loaded,
    /// False only after this task installed its normal final value.
    armed: bool,
    /// Durable invocation ownership travels with the queued/active closure.
    audit: Option<cli::operation_audit::Attempt>,
    /// Excludes cooperating CLI/HTTP writers until this guard is dropped.
    lease: Option<cli::execution_lease::Lease>,
    /// This task's place in [`engine_tasks_running`], given back on drop.
    _counted: EngineTaskCount,
}

/// How many [`TaskFinisher`]s exist: engine tasks queued or running on a
/// blocking thread. audit-20261003 hunt-api-2, D-1582: the stopping process
/// reads it to bound and to name what it abandons
/// ([`crate::server::end_runtime`]).
static ENGINE_TASKS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// Engine tasks (sweeps, descents, commands) not yet finished.
pub(crate) fn engine_tasks_running() -> usize {
    ENGINE_TASKS.load(std::sync::atomic::Ordering::Acquire)
}

/// One count in [`ENGINE_TASKS`], held for exactly as long as its finisher.
#[derive(Debug)]
struct EngineTaskCount;

impl EngineTaskCount {
    fn take() -> Self {
        ENGINE_TASKS.fetch_add(1, std::sync::atomic::Ordering::AcqRel);
        Self
    }
}

impl Drop for EngineTaskCount {
    fn drop(&mut self) {
        ENGINE_TASKS.fetch_sub(1, std::sync::atomic::Ordering::AcqRel);
    }
}

impl TaskFinisher {
    /// Arms one guard for the already-claimed slot.
    #[cfg(test)]
    fn new(site: crate::server::Loaded) -> Self {
        Self {
            _counted: EngineTaskCount::take(),
            site,
            armed: true,
            audit: None,
            lease: None,
        }
    }

    fn audited(site: crate::server::Loaded, audit: cli::operation_audit::Attempt) -> Self {
        Self {
            _counted: EngineTaskCount::take(),
            site,
            armed: true,
            audit: Some(audit),
            lease: None,
        }
    }

    fn with_lease(mut self, lease: cli::execution_lease::Lease) -> Self {
        self.lease = Some(lease);
        self
    }

    fn enter<T>(&self, work: impl FnOnce() -> T) -> T {
        match self.audit.as_ref() {
            Some(audit) => audit.enter(work),
            None => work(),
        }
    }

    /// Runs one engine task's work and, when the process was asked to stop
    /// while it ran, answers with the cancellation instead of its output.
    ///
    /// audit-20261003 hunt-api-2, D-1551. The work itself stops at its next
    /// structural boundary (`cli::cancel`), and a cancelled `cli` answer
    /// already names itself. This covers the rest: work whose last boundary
    /// passed before the stop, or a command whose own refusal hid the
    /// cancelled one. Either way its output is not presented as a complete
    /// result, and its slot and invocation audit say CANCELLED.
    fn conduct(&self, work: impl FnOnce() -> Progress) -> Progress {
        let mut done = self.enter(work);
        cancelled_at_stop(&mut done);
        done
    }

    /// Installs a normal result and prevents `Drop` from painting over it.
    fn finish(mut self, mut done: Progress) {
        use cli::operation_audit::Phase;
        let phase = match completion_audit(&done).outcome {
            "report" => Phase::Completed,
            "refused" => Phase::Refused,
            "cancelled" => Phase::Cancelled,
            _ => Phase::Failed,
        };
        if let Some(audit) = self.audit.as_mut()
            && let Err(why) = audit.finish(phase, 0)
        {
            let original = done.refusal.take().or_else(|| done.report.take());
            done.report = None;
            done.refusal = Some(
                format!(
                    "refused: required terminal invocation audit is unconfirmed: {why}. Existing computation evidence was not removed.\n{}",
                    original
                        .as_deref()
                        .unwrap_or("No final report was returned.")
                )
                .into(),
            );
        }
        // THE LEASE IS GIVEN BACK BEFORE THE SLOT SAYS FINISHED. The other
        // order published `in_flight == false` while this task still owned
        // the store's execution lease, so a press admitted in that gap passed
        // the slot check and was refused `Busy` by a run that had ended.
        // conc:runs-2, D-2778.
        drop(self.lease.take());
        let mut slot = self
            .site
            .sweep
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *slot = Some(done);
        self.armed = false;
    }
}

impl Drop for TaskFinisher {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        let audit_failure = self.audit.as_mut().and_then(|audit| {
            let phase = if std::thread::panicking() {
                cli::operation_audit::Phase::Failed
            } else {
                cli::operation_audit::Phase::Cancelled
            };
            audit.finish(phase, 0).err()
        });
        // Given back before the slot says ended, as in `finish` (D-2778).
        drop(self.lease.take());
        let mut slot = self
            .site
            .sweep
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let Some(progress) = slot.as_mut().filter(|progress| progress.in_flight()) else {
            return;
        };
        progress.finished_micros = Some(now_micros());
        progress.report = None;
        progress.refusal = Some(audit_failure.map_or_else(|| ABNORMAL_END.into(), |why| format!("{ABNORMAL_END}\nThe required terminal invocation audit is also unconfirmed: {why}").into()));
    }
}

/// Replaces an engine answer with the named cancellation once a stop was asked.
///
/// Leaves an answer that already names [`cli::cancel::CANCELLED`] as it is.
fn cancelled_at_stop(done: &mut Progress) {
    if !cli::cancel::requested()
        || done
            .refusal
            .as_deref()
            .is_some_and(|why| why.contains(cli::cancel::CANCELLED))
    {
        return;
    }
    done.report = None;
    done.refusal = Some(
        format!(
            "{REFUSED}: {} Stopped at: the end of the task, before its answer was recorded.",
            cli::cancel::CANCELLED
        )
        .into(),
    );
}

/// Files `cli`'s answer under the field that describes it, and ends the run.
///
/// # The bug this exists to make impossible
///
/// [`conduct`] assigned `cli`'s answer to `report` for EVERY outcome and left
/// `refusal` at [`None`] always. `range_all` refuses the whole command when
/// every rung refused — an unknown feed word, a span the store holds no month
/// of, a backwards range — and returns that sentence in place of a table. So a
/// refused sweep reached `/backtest/run.json` as `"refusal":null` with
/// `"in_flight":false`, and `in_flight` is the only thing the page tests. It
/// printed **"Sweep finished"** in green over a ledger that gained no row.
///
/// That is `CLAUDE.md` §4's failure wearing a success's clothes, sitting on the
/// one control an operator presses. The two fields are mutually exclusive now:
/// exactly one of them is [`Some`], and WHICH one is the outcome.
///
/// # Why it is split out of [`conduct`]
///
/// `conduct` sweeps millions of bars, so a test that drove it would be a sweep
/// rather than a test — and neither outcome could be provoked without a store
/// shaped to provoke it. This takes the text and is total over both shapes.
fn settle(progress: &mut Progress, text: String, finished_micros: i64) {
    if text.starts_with(REFUSED) {
        progress.refusal = Some(text.into());
    } else {
        progress.report = Some(text.into());
    }
    progress.finished_micros = Some(finished_micros);
}

/// The facts a completion event may claim from one settled engine task.
///
/// A report is deliberately called a `report`, not a committed ledger row.
/// Range and descent reports contain one recording outcome per attempted
/// result, and a mixed report can therefore be a valid completion without one
/// universal "saved" statement. The event records the outcome shape that is
/// actually present and leaves the per-result claims inside that report.
struct CompletionAudit<'a> {
    level: telemetry::Level,
    outcome: &'static str,
    why: &'a str,
}

/// Classifies a completed slot without interpreting `cli`'s prose again.
fn completion_audit(progress: &Progress) -> CompletionAudit<'_> {
    match (progress.report.as_ref(), progress.refusal.as_deref()) {
        (Some(_), None) => CompletionAudit {
            level: telemetry::Level::Info,
            outcome: "report",
            why: "",
        },
        // A STOP IS NOT A REFUSAL OF THE REQUEST: it is named as its own
        // outcome so the log and the invocation audit say the run was
        // cancelled, not that it was judged and refused (hunt-api-2, D-1551).
        (None, Some(why)) if why.contains(cli::cancel::CANCELLED) => CompletionAudit {
            level: telemetry::Level::Error,
            outcome: "cancelled",
            why,
        },
        (None, Some(why)) => CompletionAudit {
            level: telemetry::Level::Warn,
            outcome: "refused",
            why,
        },
        (Some(_), Some(_)) => CompletionAudit {
            level: telemetry::Level::Error,
            outcome: "invalid",
            why: "both report and refusal were set",
        },
        (None, None) => CompletionAudit {
            level: telemetry::Level::Error,
            outcome: "invalid",
            why: "neither report nor refusal was set",
        },
    }
}

/// Emits one honest completion event shared by sweeps, descents and commands.
///
/// Kept as one production emit site so all three entry points use the same
/// vocabulary and the emit-site census can drive it without running a sweep.
pub(crate) fn emit_completion(
    progress: &Progress,
    operation: &str,
    began: std::time::Instant,
) -> telemetry::Emitted {
    let audit = completion_audit(progress);
    // A duration taken from `Instant`, never from two wall-clock reads: a
    // backward clock step used to record "took 0 µs" through `.max(0)`, a
    // measurement nobody took (`CLAUDE.md` §3 rule 6). D-2754 (CE-86).
    let elapsed_micros = u64::try_from(began.elapsed().as_micros()).unwrap_or(u64::MAX);
    telemetry::emit_for_run(
        progress.attempt,
        &telemetry::Event::new(audit.level, "api.sweep", "an engine task finished")
            .with("attempt", progress.attempt)
            .with("operation", operation)
            .with("feed", progress.feed.as_str())
            .with("underlying", progress.underlying.as_str())
            .with("outcome", audit.outcome)
            .with("why", audit.why)
            .with("elapsed_micros", elapsed_micros)
            .with("elapsed_basis", "monotonic"),
    )
}

/// Runs the sweep and records what it produced.
///
/// **Blocking on purpose, and the caller must place it accordingly.**
/// `cli::range_all` is CPU-bound over millions of bars; running it directly on
/// a tokio worker would hold that thread for the whole sweep and starve every
/// other request on it. The handler puts this on a blocking thread.
///
/// The text `cli` returns is kept whole, whichever field [`settle`] files it
/// under. It is the same text `cli range-all` prints in a terminal, including
/// the `STORED_PROVENANCE` banner that says the bars were REAL — `CLAUDE.md`
/// §5 makes that banner the only thing separating a real sweep from a
/// generated one, so it travels with the report rather than being stripped for
/// the page.
/// Set this request's knobs for the run about to happen, and say so.
///
/// # Why this is its own function
///
/// So a test can drive it WITHOUT starting a sweep. The emit-site census in
/// `emitted.rs` requires every telemetry line in this binary to be either driven
/// by a test or listed as unreachable, and the only other way to reach this one
/// was through [`conduct`] — which calls `cli::range_over` on the real store two
/// lines later. On the operator's own machine that store holds eighty-one months
/// of one-minute bars, so a test that drove this emit through `conduct` would
/// have launched a multi-hour sweep from `cargo test`.
///
/// A census that can only be satisfied by an expensive test is a census people
/// route around, which is how a row ends up on the unreachable list for a reason
/// that is really "it was awkward".
///
/// # Ordering
///
/// Set BEFORE the run and reported before it too, because the run is the thing
/// that might take hours and the settings are what an operator watching it needs
/// to see. The run's identity already covers the knobs that move the answer, but
/// reading an identity back into settings means recomputing a blake3 hash, which
/// is not something a person can do from a log page.
///
/// `clear_all` first, so a previous request that set a knob cannot leak into this
/// one through a path that returned early.
pub fn apply_knobs(asked: &Asked) -> Applied {
    cli::knobs::clear_all();
    for (name, value) in &asked.knobs {
        cli::knobs::set(name, value);
    }
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::info("api.sweep", "knobs set for this run")
            .with("feed", asked.feed.as_str())
            .with("underlying", asked.underlying.as_str())
            .with(
                "rungs",
                u64::try_from(asked.rungs.len()).unwrap_or(u64::MAX),
            )
            .with("knobs", cli::knobs::describe().as_str()),
    );
    Applied
}

/// Clears every knob when it is dropped, however the run ends.
///
/// # Why a guard and not a call after the sweep
///
/// Because `conduct` used to clear them on the line AFTER `cli::range_over`
/// returned, and that line does not run when `range_over` panics.
///
/// MEASURED by attacking the route: a panic there left every knob set for the
/// LIFE OF THE PROCESS, so the next request -- including a descent or a command,
/// neither of which sets knobs at all -- silently inherited a dead request's
/// screen cap, support floor and validation setting. A run steered by settings
/// nobody chose, with an audit line naming a different request's.
///
/// `Drop` runs on normal and early return in every build, and during unwinding
/// where a panic unwinds (`dev`, `test`). `release` sets `panic = "abort"`, so
/// there a panic ends the process and no later request exists to inherit the
/// knobs (poison-1, D-1771).
#[must_use = "the guard must be held for the run, not dropped immediately"]
pub struct Applied;

impl Drop for Applied {
    fn drop(&mut self) {
        cli::knobs::clear_all();
    }
}

/// Runs the sweep and records what it produced.
///
/// **Blocking on purpose.** `cli::range_over` is CPU-bound over millions of
/// bars; running it directly on a tokio worker would hold that thread for the
/// whole sweep and starve every other request on it. The handler puts this on a
/// blocking thread.
///
/// The text `cli` returns is kept whole, whichever field [`settle`] files it
/// under. It is the same text `cli range-all` prints in a terminal, including
/// the `STORED_PROVENANCE` banner that says the bars were REAL — `CLAUDE.md` §5
/// makes that banner the only thing separating a real sweep from a generated
/// one, so it travels with the report rather than being stripped for the page.
///
/// [`apply_knobs`] runs first and its `clear_all` runs last, so this request's
/// settings cannot outlive it.
#[must_use]
pub fn conduct(asked: &Asked, now_micros: i64, attempt: u64) -> Progress {
    let mut progress = Progress::started(
        &asked.feed,
        &asked.underlying,
        asked.from,
        asked.to,
        None,
        now_micros,
        attempt,
    );
    // THE REQUEST'S OWN KNOBS, SET FOR THIS RUN AND CLEARED AFTER IT.
    //
    // Set here rather than at parse time because parsing a body must not change
    // how this process behaves — a malformed request that is refused three lines
    // later would otherwise have already moved the screen cap for whatever runs
    // next. By the time control reaches this function the request is whole and
    // a run is definitely about to happen.
    //
    // Safe as process-wide state because a sweep is already one-at-a-time:
    // `holding` refuses a second run while `in_flight`, in three places, so
    // there is never a moment when two runs want different values for one knob.
    // HELD FOR THE WHOLE RUN. The guard clears on drop, so a panic inside
    // `range_over` cannot leave this request's settings behind for the next one.
    let _knobs = apply_knobs(asked);

    // `range_over` AND NOT `range_all`, because the request can now name the
    // rungs. A body with no `rungs` field parses to `EVERY_RUNG`, so this is
    // byte-identical to the old call for every existing client — `range_all`
    // is itself `range_over(&EVERY_RUNG, ..)` now, so there is one code path
    // rather than two that must be kept agreeing.
    let text = cli::range_over_for_attempt(
        &asked.feed,
        &asked.underlying,
        &asked.rungs,
        asked.from,
        asked.to,
        None,
        attempt,
    );
    settle(&mut progress, text, now_micros);
    progress
}

/// Runs one descent and records what it produced.
///
/// **Blocking on purpose**, exactly as [`conduct`] is, and for the same reason:
/// this is CPU-bound over the same bars, and more of them — a descent is a full
/// screen per rung of the support ladder, not one.
///
/// `support_ppm` on the returned [`Progress`] is the CEILING the walk began
/// from, not a threshold the run held. [`Kind::Descent`] is what tells the page
/// to read it that way.
#[must_use]
pub fn conduct_descent(asked: &AskedDescent, now_micros: i64, attempt: u64) -> Progress {
    let mut progress = Progress::started(
        &asked.feed,
        &asked.underlying,
        asked.from,
        asked.to,
        None,
        now_micros,
        attempt,
    )
    .of_kind(Kind::Descent);
    // POINTS, NEVER PPM. `cli::elite_descend_in_points` converts against the
    // midpoint of the span's own bars; a ppm computed on this side would be a
    // second answer to a question only the bars can settle, and its own doc
    // records what that has already cost.
    let text = cli::elite_descend_in_points_for_attempt(
        &asked.feed,
        &asked.underlying,
        &asked.rung,
        (asked.from, asked.to),
        asked.max_points,
        asked.top,
        attempt,
    );
    settle(&mut progress, text, now_micros);
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

/// [`now_micros`] in whole milliseconds, the unit the rolling log stamps
/// records in. One function, so the two observation paths cannot divide
/// differently and a test pins the unit against the system clock. G18-api-22.
fn now_millis() -> i64 {
    now_micros() / 1_000
}

/// Reserves one exact attempt token.
///
/// The append-only invocation index owns the durable sequence space, so a
/// restart cannot reuse an earlier attempt ID. This reservation is separate
/// from rotating telemetry and from computation identity. A failed required
/// audit refuses before an engine task can be queued.
fn reserve_invocation(
    site: &crate::server::Loaded,
    label: &str,
) -> Result<cli::operation_audit::Attempt, Refusal> {
    cli::operation_audit::begin(
        &site.store_root,
        cli::operation_audit::Origin::Browser,
        label,
    )
    .map_err(|why| {
        Refusal::Unobservable(format!(
            "required durable invocation audit could not start: {why}. No engine work was started."
        ))
    })
}

/// The exact bracket the live reducer requires before admitting any rung.
fn attempt_started_event(progress: &Progress) -> telemetry::Event<'_> {
    telemetry::Event::info("cli.audit", "sweep attempt started")
        .with("attempt", progress.attempt)
        .with("kind", progress.kind.word())
        .with("feed", progress.feed.as_str())
        .with("underlying", progress.underlying.as_str())
        .with("from_year", u64::from(progress.from.0))
        .with("from_month", u64::from(progress.from.1))
        .with("to_year", u64::from(progress.to.0))
        .with("to_month", u64::from(progress.to.1))
}

/// Writes [`attempt_started_event`] under the same opaque key status exposes.
pub(crate) fn emit_attempt_started(progress: &Progress) -> telemetry::Emitted {
    telemetry::emit_for_run(progress.attempt, &attempt_started_event(progress))
}

/// Turns a missing required attempt marker into a loud admission refusal.
///
/// The browser's reducer deliberately accepts no live event until it sees this
/// exact marker. Starting work after the marker was filtered, dropped, or had
/// nowhere to land would therefore create a run that the shipped monitor can
/// never identify. That is an observability failure before it is a logging
/// preference, so the work does not start.
fn marker_refusal(outcome: telemetry::Emitted) -> Option<Refusal> {
    let why = match outcome {
        telemetry::Emitted::Written => return None,
        telemetry::Emitted::Filtered => {
            "the required sweep-attempt marker was filtered by the telemetry policy. Enable \
             info events for `cli.audit`; no engine work was started."
        }
        telemetry::Emitted::Dropped => {
            "the required sweep-attempt marker could not be written. Telemetry health carries \
             the storage failure; no engine work was started."
        }
        telemetry::Emitted::NotInstalled => {
            "no telemetry sink is installed, so the required sweep-attempt marker has nowhere \
             to be recorded. No engine work was started."
        }
    };
    Some(Refusal::Unobservable(why.to_owned()))
}

/// The JSON content type both routes answer with.
type JsonHeaders = [(axum::http::HeaderName, &'static str); 1];

/// One route answer, as [`refused`] and the handlers build it.
type Answer = (axum::http::StatusCode, JsonHeaders, String);

/// Admits one run: refuses while one is in flight, prepares it with the slot
/// UNLOCKED, and installs it.
///
/// `prepare` does every fallible, I/O-bound admission step and returns the
/// accepted [`Progress`] with whatever the caller keeps. Under the site's
/// admission lock nothing else installs into the slot between the busy check and the install,
/// and a [`TaskFinisher`] only writes a slot whose run is in flight, which the
/// busy check has just ruled out. A refusal from `prepare` leaves the slot as
/// it was. The slot's own lock is taken twice, each time for O(1) work:
/// `api::sweeprun::admission_io_runs_with_the_slot_unlocked_and_admissions_still_exclude_each_other`.
///
/// # The admission lock serialises browser ADMISSIONS, so the slot's own mutex never has to
///
/// Admission does file-system work no constant bounds: two canonicalizations,
/// the execution lease, an external-log walk of up to 8 MiB, a launch
/// preparation, a durable audit `begin` with its syncs, and a telemetry marker.
/// All three POST handlers used to do it while holding `site.sweep`, and
/// `run_json` takes that same std mutex ON AN ASYNC WORKER for every poll, so a
/// poll waited out a stranger's admission I/O with a Tokio worker blocked
/// (W1-api6-2, D-0954). Admissions still exclude each other, which is what the
/// slot lock was doing there; the slot itself is now held only for one read
/// and one write.
///
/// `try_lock`, NOT `lock`: refused rather than queued. Each POST holds one of
/// the four shared `detail::run` permits before it gets here, so a press that
/// WAITED for another admission parked a permit for that admission's whole
/// I/O, and three such presses answered `Saturated` on every unrelated
/// `detail::run` route, the run's own status poll included. A press that
/// meets another admission is refused as `Busy` at once, as it would be by the
/// run that admission is about to install. The lock is per `Site`
/// ([`crate::server::Site::sweep_admission`]), so two test sites admitting at
/// once never refuse each other. conc:runs-4, D-2776.
fn admit<T>(
    site: &crate::server::Loaded,
    busy: impl FnOnce() -> Answer,
    prepare: impl FnOnce() -> Result<(Progress, T), Answer>,
) -> Result<T, Answer> {
    let _admitting = match site.sweep_admission.try_lock() {
        Ok(admitting) => admitting,
        Err(std::sync::TryLockError::Poisoned(poisoned)) => poisoned.into_inner(),
        Err(std::sync::TryLockError::WouldBlock) => return Err(busy()),
    };
    let in_flight = site
        .sweep
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .as_ref()
        .is_some_and(Progress::in_flight);
    if in_flight {
        return Err(busy());
    }
    let (accepted, kept) = prepare()?;
    *site
        .sweep
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(accepted);
    Ok(kept)
}

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
/// Admission writes a bounded number of fixed-size audit records on the
/// shared blocking worker door. File synchronization and lock acquisition
/// do not have a constant-latency guarantee. No bar or candidate scan runs
/// on this request path. The sweep runs separately on a blocking thread;
/// its total work remains proportional to the search it must perform.
///
/// # Why it answers before the sweep finishes
///
/// Once required admission barriers succeed, the response identifies the
/// detached task. The page polls [`run_json`] for that exact attempt. A
/// process restart can expose the persisted outcome, but cannot reconstruct
/// lost report prose or establish that an unconfirmed task is still running.
pub async fn run(
    axum::extract::State(site): axum::extract::State<crate::server::Loaded>,
    body: String,
) -> (axum::http::StatusCode, JsonHeaders, String) {
    match crate::detail::run(move || run_with(&site, &body, cli::commit_stamp())).await {
        Ok(response) => response,
        Err(why) => refused(&Refusal::Unobservable(format!(
            "bounded sweep admission is unavailable: {why:?}; no engine command was dispatched"
        ))),
    }
}

/// One refusal, as the answer this route gives.
///
/// Three call sites wrote this `format!` identically. Collapsed because the
/// body shape is a CONTRACT with the page — it reads `accepted` and `refusal`
/// and nothing else — and three copies of a contract drift one at a time.
fn refused(why: &Refusal) -> (axum::http::StatusCode, JsonHeaders, String) {
    (
        why.status(),
        json_headers(),
        format!(
            r#"{{"accepted":false,"refusal":{}}}"#,
            crate::render::json_string(why.why())
        ),
    )
}

/// [`refused`], said first: one `api.sweep` Warn event carrying `message` and
/// the refusal's own sentence as `why`.
///
/// conc13-3, D-2595. The refusals at the budget environment, the stamp of a
/// descent or command, the busy slot of a descent or command, the execution
/// lease, the invocation journal, a command's launch preparation and the
/// start marker returned [`refused`] with no event, so `/logs` held only the
/// middleware's `served ... 503` with no reason, while the page told the
/// operator to read the logs. Every refusal arm of the three routes now says
/// why, once, at the route's own boundary (gate 17 does not cover `api`).
///
/// EXCEPT THE TWO ABOUT THIS SERVER. The unstamped build and the environment
/// budget were closed by both sides at once — conc13-3 through this helper,
/// sobs-9 (D-4447) through [`refused_for_this_server`], which also names the
/// route — so each of those arms wrote two lines. They go through that one
/// writer on all three routes; every other arm stays here. D-4627.
fn refuse_logged(
    message: &'static str,
    why: &Refusal,
) -> (axum::http::StatusCode, JsonHeaders, String) {
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::new(telemetry::Level::Warn, "api.sweep", message)
            .with("why", telemetry::Value::Str(why.why())),
    );
    refused(why)
}

/// The three launch routes, as the `route` field of a refusal names them.
const RUN_ROUTE: &str = "/backtest/run";
/// See [`RUN_ROUTE`].
const DESCEND_ROUTE: &str = "/backtest/descend";
/// See [`RUN_ROUTE`].
const COMMAND_ROUTE: &str = "/engine/command";

/// A launch refused for a fact about THIS SERVER, logged once, on every route.
///
/// # What was silent (sobs-9, D-4447)
///
/// An unstamped build and a screen budget in the server's own environment are
/// decided before any request arrives, and each refuses every launch. Only
/// `/backtest/run`'s unstamped arm wrote an event; `/backtest/descend` and
/// `/engine/command` refused both silently, and no route logged the budget, so
/// `/logs` showed `api.request` 503s with no reason beside them. One event per
/// refused press, carrying the route and the refusal's own sentence, before
/// anything is claimed. The message keeps `/backtest/run`'s wording so a
/// reader filtering on it finds every route.
fn refused_for_this_server(
    route: &str,
    why: &Refusal,
) -> (axum::http::StatusCode, JsonHeaders, String) {
    let _ = telemetry::emit_if!(
        telemetry::Level::Warn,
        "api.sweep",
        if matches!(why, Refusal::Unstamped(_)) {
            "a sweep was refused because this build carries no commit stamp"
        } else {
            "a sweep was refused because this server's environment sets a screen budget"
        },
        "route" => telemetry::Value::Str(route),
        "why" => telemetry::Value::Str(why.why()),
    );
    refused(why)
}

/// The same physical store is claimed before the start audit and before spawn.
/// A positive GET snapshot never substitutes for this atomic POST admission.
fn claim_execution(
    site: &crate::server::Loaded,
    uses_configured_store: bool,
) -> Result<cli::execution_lease::Lease, Refusal> {
    if uses_configured_store {
        let configured = cli::preflight_store_root().map_err(Refusal::Unobservable)?;
        require_same_execution_store(&site.store_root, &configured)?;
    }
    let lease = cli::execution_lease::Lease::acquire(&site.store_root).map_err(|why| {
        if matches!(why, cli::execution_lease::Refusal::Busy) {
            Refusal::Busy(why.to_string())
        } else {
            Refusal::Unobservable(why.to_string())
        }
    })?;
    if let Some(dir) = crate::logs::cli_log_dir() {
        admit_external(&site.store_root, &observe_elsewhere(&dir, now_millis()))?;
    }
    Ok(lease)
}

/// Ordinary library calls resolve their process store internally; their lease
/// and API evidence must name that same canonical directory before dispatch.
fn require_same_execution_store(
    site_root: &std::path::Path,
    configured_root: &std::path::Path,
) -> Result<(), Refusal> {
    let canonical = |path: &std::path::Path| {
        std::fs::canonicalize(path).map_err(|why| {
            Refusal::Unobservable(format!("execution store cannot be canonicalized: {why}"))
        })
    };
    if canonical(site_root)? != canonical(configured_root)? {
        return Err(Refusal::Unobservable("the API evidence store differs from the CLI execution store; no lease was claimed and no command was dispatched".to_owned()));
    }
    Ok(())
}

/// [`run`], with the build's commit stamp passed in.
///
/// # Why the split exists, and it is not stylistic
///
/// `cargo test` is an UNSTAMPED build — `crates/runner`'s own doc records that
/// this is why `run_ranked` once shipped with zero coverage — so a handler that
/// read `cli::commit_stamp()` directly would take the unstamped arm in every
/// test, forever. That does not merely leave the stamped arm uncovered: it
/// makes the `Busy` refusal **unreachable**, because a press that never claims
/// the slot cannot make the next press a conflict, and `crate::emitted` proves
/// that emit site by driving exactly that sequence.
///
/// The same shape as `root_from` beside `store_root`, and for the same reason:
/// the decision is testable without the process it depends on.
pub(crate) fn run_with(
    site: &crate::server::Loaded,
    body: &str,
    stamp: Option<&str>,
) -> (axum::http::StatusCode, JsonHeaders, String) {
    let asked = match asked_from(body) {
        Ok(asked) => asked,
        Err(why) => {
            let _ = telemetry::emit_if!(
                telemetry::Level::Warn,
                "api.sweep",
                "a sweep was refused before it started",
                "why" => telemetry::Value::Str(why.why()),
            );
            return refused(&why);
        }
    };

    // BEFORE THE SLOT, BECAUSE A REFUSAL THAT CANNOT CHANGE MUST NOT OCCUPY IT.
    //
    // An unstamped build refuses every run it could ever start, so claiming the
    // slot first would make this route answer 409 `Busy` to a second press
    // while the first was busy failing for a reason no wait can fix.
    if let Some(why) = stamp_refusal(stamp) {
        return refused_for_this_server(RUN_ROUTE, &why);
    }

    // BEFORE THE SLOT FOR THE SAME REASON: a budget in the server's own
    // environment refuses every rung this run could sweep. D-0685.
    // One writer for this arm, with the route (D-4447; D-4627).
    if let Some(why) = environment_budget_refusal() {
        return refused_for_this_server(RUN_ROUTE, &why);
    }

    // THE SLOT IS CLAIMED UNDER THE LOCK AND THE WORK STARTS OUTSIDE IT.
    // Holding a std mutex across an await is the deadlock this pattern exists
    // to avoid, so the guard is dropped before anything is spawned.
    let busy = || {
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
        refused(&Refusal::Busy(why))
    };
    let admitted = admit(site, busy, || {
        let lease = claim_execution(site, true)
            .map_err(|why| refuse_logged("a sweep was refused at its execution lease", &why))?;
        let mut audit = reserve_invocation(site, "sweep")
            .map_err(|why| refuse_logged("a sweep was refused at its invocation journal", &why))?;
        let attempt = audit.id();
        let started = now_micros();
        let accepted = Progress::started(
            &asked.feed,
            &asked.underlying,
            asked.from,
            asked.to,
            None,
            started,
            attempt,
        );
        if let Some(why) = marker_refusal(emit_attempt_started(&accepted)) {
            let _terminal = audit.finish(cli::operation_audit::Phase::Refused, 0);
            return Err(refuse_logged(
                "a sweep was refused because its start could not be recorded",
                &why,
            ));
        }
        Ok((accepted, (started, attempt, audit, lease)))
    });
    let (started, attempt, audit, lease) = match admitted {
        Ok(admitted) => admitted,
        Err(answer) => return answer,
    };

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
        "attempt" => telemetry::Value::Uint(attempt),
        // DERIVED PER RUNG, so there is no one number to log. Logging a
        // constant here would put a figure in the audit trail that no rung
        // actually used -- D-0303.
        "support" => telemetry::Value::Str("derived per rung from its own bars"),
    );

    // A BLOCKING THREAD, NOT A WORKER. `cli::range_all` is CPU-bound over
    // millions of bars; on a worker it would hold that thread for the whole
    // sweep and every other request sharing it would wait.
    // ARMED BEFORE SPAWN. If Tokio drops a queued closure during shutdown, the
    // captured guard still releases the slot even though the closure body never
    // begins. A guard constructed inside the closure would miss that window.
    // MONOTONIC, for the duration only: `started` stays the wall-clock stamp
    // the page shows, and an NTP step during the run cannot make the recorded
    // `elapsed_micros` a clamped 0 or an inflated figure. D-2754 (CE-86).
    let began = std::time::Instant::now();
    let guard = TaskFinisher::audited(std::sync::Arc::clone(site), audit).with_lease(lease);
    tokio::task::spawn_blocking(move || {
        let finished = guard.conduct(|| conduct(&asked, started, attempt));
        let _outcome = emit_completion(&finished, "sweep", began);
        let mut done = finished;
        done.finished_micros = Some(now_micros());
        guard.finish(done);
    });

    (
        axum::http::StatusCode::ACCEPTED,
        json_headers(),
        command_acceptance(attempt),
    )
}

/// `POST /backtest/descend` — walk one rung's support down, and answer at once.
///
/// # The command the console could not reach
///
/// `cli::elite_descend`'s own doc states the case this closes: *"A run launched
/// at 20,000 ppm cannot report a once-a-week setup no matter how long it runs
/// — the setup was pruned in the first level of the ladder, and the report says
/// nothing about it because nothing counted it."* [`run`] launches at
/// `SUPPORT_PPM`, which is **200,000** — ten times that figure. So the one
/// control the browser had could not, by construction, find the rare setup this
/// engine exists to hunt, and the command that can had **no HTTP route at all**.
///
/// # Additive, and deliberately so
///
/// [`run`] keeps the nine-rung comparison at one fixed support, which is the
/// right shape for asking *"which timeframe carries the edge"* — nine rungs are
/// only comparable on equal terms. This asks the other question, on one rung,
/// with the threshold walked instead of fixed. Neither replaces the other and
/// no recorded run changes meaning.
///
/// # It shares the slot, because it shares the ledger
///
/// A descent appends to the same append-only file, so two of them — or one of
/// each — finishing together can interleave two records. The slot, the busy
/// refusal and the commit gate are the same for both, and that is not a
/// convenience: it is the reason the slot exists.
pub async fn descend(
    axum::extract::State(site): axum::extract::State<crate::server::Loaded>,
    body: String,
) -> (axum::http::StatusCode, JsonHeaders, String) {
    match crate::detail::run(move || descend_with(&site, &body, cli::commit_stamp())).await {
        Ok(response) => response,
        Err(why) => refused(&Refusal::Unobservable(format!(
            "bounded descent admission is unavailable: {why:?}; no engine command was dispatched"
        ))),
    }
}

/// [`descend`], with the build's commit stamp passed in.
///
/// Split for the reason [`run_with`] gives, and it applies identically here:
/// A handler reading the stamp itself would have an arm the suite cannot reach
/// whichever way the build is stamped -- see [`stamp_refusal`], which records
/// that `build.rs` moved WHICH arm that is,
/// would take one arm forever and make the other unreachable.
pub(crate) fn descend_with(
    site: &crate::server::Loaded,
    body: &str,
    stamp: Option<&str>,
) -> (axum::http::StatusCode, JsonHeaders, String) {
    let asked = match descent_from(body) {
        Ok(asked) => asked,
        Err(why) => {
            let _ = telemetry::emit_if!(
                telemetry::Level::Warn,
                "api.sweep",
                "a descent was refused before it started",
                "why" => telemetry::Value::Str(why.why()),
            );
            return refused(&why);
        }
    };

    // Both server refusals: one writer each, with the route (D-4447; D-4627).
    if let Some(why) = stamp_refusal(stamp) {
        return refused_for_this_server(DESCEND_ROUTE, &why);
    }
    // A descent's first step is `cli::one_rung`, which refuses a server budget;
    // refused here instead, before the slot, as `run_with` does. D-0685.
    if let Some(why) = environment_budget_refusal() {
        return refused_for_this_server(DESCEND_ROUTE, &why);
    }

    let busy = || {
        refuse_logged(
            "a descent was refused while a run was in flight",
            &Refusal::Busy(
                "a sweep or descent is already running in this process. \
                 Both append to the same append-only ledger, and two of \
                 them finishing together can interleave two records, so \
                 the second press is refused rather than queued."
                    .to_owned(),
            ),
        )
    };
    let admitted = admit(site, busy, || {
        let lease = claim_execution(site, true)
            .map_err(|why| refuse_logged("a descent was refused at its execution lease", &why))?;
        let mut audit = reserve_invocation(site, "descent").map_err(|why| {
            refuse_logged("a descent was refused at its invocation journal", &why)
        })?;
        let attempt = audit.id();
        let started = now_micros();
        let accepted = Progress::started(
            &asked.feed,
            &asked.underlying,
            asked.from,
            asked.to,
            None,
            started,
            attempt,
        )
        .of_kind(Kind::Descent);
        if let Some(why) = marker_refusal(emit_attempt_started(&accepted)) {
            let _terminal = audit.finish(cli::operation_audit::Phase::Refused, 0);
            return Err(refuse_logged(
                "a descent was refused because its start could not be recorded",
                &why,
            ));
        }
        Ok((accepted, (started, attempt, audit, lease)))
    });
    let (started, attempt, audit, lease) = match admitted {
        Ok(admitted) => admitted,
        Err(answer) => return answer,
    };

    let _ = telemetry::emit_if!(
        telemetry::Level::Info,
        "api.sweep",
        "a descent was accepted from the browser",
        "feed" => telemetry::Value::Str(&asked.feed),
        "underlying" => telemetry::Value::Str(&asked.underlying),
        "rung" => telemetry::Value::Str(&asked.rung),
        "max_points" => telemetry::Value::Int(asked.max_points),
        "attempt" => telemetry::Value::Uint(attempt),
    );

    // MONOTONIC, for the duration only: `started` stays the wall-clock stamp
    // the page shows, and an NTP step during the run cannot make the recorded
    // `elapsed_micros` a clamped 0 or an inflated figure. D-2754 (CE-86).
    let began = std::time::Instant::now();
    let guard = TaskFinisher::audited(std::sync::Arc::clone(site), audit).with_lease(lease);
    tokio::task::spawn_blocking(move || {
        let finished = guard.conduct(|| conduct_descent(&asked, started, attempt));
        let _outcome = emit_completion(&finished, "descent", began);
        let mut done = finished;
        done.finished_micros = Some(now_micros());
        guard.finish(done);
    });

    (
        axum::http::StatusCode::ACCEPTED,
        json_headers(),
        command_acceptance(attempt),
    )
}

/// `GET /backtest/run.json` — what the sweep is doing, for the page to poll.
///
/// An active browser slot is read directly. Otherwise a bounded telemetry tail
/// is inspected on the shared blocking worker door. The tail costs O(bytes
/// scanned), up to its stated cap; unreadable evidence is unknown, never idle.
/// A canonical `?attempt=<u64>` selects only that browser attempt. An absent
/// local attempt in the durable namespace is looked up on the blocking door;
/// its saved outcome does not reconstruct report prose or prove liveness.
/// External activity cannot hide or replace the selected identity.
pub async fn run_json(
    axum::extract::State(site): axum::extract::State<crate::server::Loaded>,
    uri: axum::http::Uri,
) -> (axum::http::StatusCode, JsonHeaders, String) {
    let selected = match requested_attempt(uri.query()) {
        Ok(selected) => selected,
        Err(why) => {
            return (
                axum::http::StatusCode::BAD_REQUEST,
                json_headers(),
                browser_attempt_unknown(None, why),
            );
        }
    };
    let local = site
        .sweep
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone();
    if let Some(attempt) = selected {
        if local
            .as_ref()
            .is_some_and(|progress| progress.attempt == attempt)
        {
            return (
                axum::http::StatusCode::OK,
                json_headers(),
                selected_browser_status(local.as_ref(), attempt),
            );
        }
        if attempt <= cli::operation_audit::ID_BASE {
            return (
                axum::http::StatusCode::OK,
                json_headers(),
                selected_browser_status(None, attempt),
            );
        }
        let root = site.store_root.clone();
        let observed =
            crate::detail::run(move || crate::operation_audit::persisted_status(&root, attempt))
                .await;
        let (status, body) = match observed {
            Ok(Ok(Some(body))) => (axum::http::StatusCode::OK, body),
            Ok(Ok(None)) => (
                axum::http::StatusCode::OK,
                selected_browser_status(None, attempt),
            ),
            Ok(Err(why)) => (
                axum::http::StatusCode::SERVICE_UNAVAILABLE,
                browser_attempt_unknown(Some(attempt), &why),
            ),
            Err(why) => (
                axum::http::StatusCode::SERVICE_UNAVAILABLE,
                browser_attempt_unknown(
                    Some(attempt),
                    &format!("persistent invocation read unavailable: {why:?}"),
                ),
            ),
        };
        return (status, json_headers(), body);
    }
    if let Some(progress) = local.as_ref().filter(|progress| progress.in_flight()) {
        return (
            axum::http::StatusCode::OK,
            json_headers(),
            format!(r#"{{"running":{}}}"#, progress.to_json()),
        );
    }
    let dir = crate::logs::cli_log_dir();
    let root = site.store_root.clone();
    match crate::detail::run(move || {
        observed_status_with_admission(&root, local.as_ref(), dir.as_deref(), now_millis())
    })
    .await
    {
        Ok(body) => (axum::http::StatusCode::OK, json_headers(), body),
        Err(why) => {
            let why = format!(
                "external sweep status is unavailable: {why:?}; no idle or successful completion is inferred"
            );
            (
                axum::http::StatusCode::SERVICE_UNAVAILABLE,
                json_headers(),
                unknown_status(&why),
            )
        }
    }
}

/// This door accepts one canonical decimal token, not a lossy numeric value or
/// an ambiguous/duplicated query. An empty query retains the global view.
fn requested_attempt(query: Option<&str>) -> Result<Option<u64>, &'static str> {
    let Some(query) = query.filter(|query| !query.is_empty()) else {
        return Ok(None);
    };
    let invalid = "attempt status requires exactly one canonical positive u64 decimal query: ?attempt=<token>";
    if query.len() > 28 {
        return Err(invalid);
    }
    let raw = query.strip_prefix("attempt=").ok_or(invalid)?;
    if raw.is_empty() || raw.starts_with('0') || !raw.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(invalid);
    }
    raw.parse::<u64>().map(Some).map_err(|_| invalid)
}

fn selected_browser_status(local: Option<&Progress>, attempt: u64) -> String {
    match local.filter(|progress| progress.attempt == attempt) {
        Some(progress) => format!(r#"{{"running":{}}}"#, progress.to_json()),
        None => browser_attempt_unknown(
            Some(attempt),
            "this exact browser attempt is not retained in this process; no other attempt or external command is a substitute, and no completion is inferred",
        ),
    }
}

fn browser_attempt_unknown(attempt: Option<u64>, why: &str) -> String {
    serde_json::json!({
        "running": {
            "where": "browser",
            "status": "unknown",
            "requested_attempt": attempt.map(|value| value.to_string()),
            "in_flight": false,
            "why": why,
            "refusal": null,
            "report": null,
        }
    })
    .to_string()
}

fn external_observation(dir: Option<&std::path::Path>, now: i64) -> ExternalObservation {
    dir.map_or_else(
        || ExternalObservation { at_millis: None, uncertain: true, in_flight: false, launch_clear: false, unterminated: None, body: unknown_status("the CLI telemetry directory is not configured; external execution state is unknown") },
        |dir| observe_elsewhere(dir, now),
    )
}

fn observed_status(local: Option<&Progress>, external: ExternalObservation) -> String {
    if let Some(local) = local
        && !external.uncertain
        && !external.in_flight
        && external
            .at_millis
            .is_none_or(|at| at <= local.finished_micros.unwrap_or(local.started_micros) / 1_000)
    {
        return format!(r#"{{"running":{}}}"#, local.to_json());
    }
    external.body
}

fn observed_status_with_admission(
    root: &std::path::Path,
    local: Option<&Progress>,
    dir: Option<&std::path::Path>,
    now: i64,
) -> String {
    let external = external_observation(dir, now);
    let (available, why) = match cli::execution_lease::probe(root) {
        Err(why) => (false, why.to_string()),
        Ok(()) if !external.launch_clear => match ended_without_terminal(root, &external) {
            None => (false, "External activity or damaged execution evidence remains unresolved. A new sweep has not been admitted.".to_owned()),
            Some(ended) => (true, format!("The store execution lease is currently free, and the newest external sweep marker names CLI invocation {ended}, which ended without its terminal marker: it held this lease while it ran, so it is not running now. How it ended is not known. A launch rechecks and claims the lease atomically.")),
        },
        Ok(()) => (true, "The store execution lease is currently free. A launch rechecks and claims it atomically. Historical status is separate; this does not establish that older binaries or bypassing callers are idle.".to_owned()),
    };
    let mut body = observed_status(local, external);
    // Every local status serializer above produces one complete JSON object.
    // Add a separate authority; never rewrite unknown history into completion.
    body.pop();
    let _ = write!(
        body,
        r#","admission":{{"schema_version":1,"scope":"cooperating-store-writers","available":{available},"why":{}}}}}"#,
        crate::render::json_string(&why)
    );
    body
}

struct ExternalObservation {
    at_millis: Option<i64>,
    uncertain: bool,
    in_flight: bool,
    /// Healthy history without an unresolved sweep marker does not own a lease.
    launch_clear: bool,
    /// The newest marker is a named sweep's `command started` under a durable
    /// invocation id (`cli::operation_audit::ID_BASE` and above), with no
    /// terminal marker after it: that id and the command word. Only a caller
    /// HOLDING the store's execution lease may act on it. D-2764.
    unterminated: Option<(u64, String)>,
    body: String,
}

fn unknown_status(why: &str) -> String {
    format!(
        r#"{{"running":{{"where":"cli","status":"unknown","in_flight":false,"why":{},"refusal":null,"report":null}}}}"#,
        crate::render::json_string(why)
    )
}

/// Absence of retained observations is not a census of every external process.
const NO_SWEEP: &str = r#"{"running":null,"why":"no local sweep or retained external lifecycle has been observed; this is not proof that every external process is idle"}"#;

/// The target prefix `cli` stamps on the records a sweep emits as it advances.
///
/// **`cli` and not `cli.audit`.** `telemetry::Query`'s target filter matches an
/// exact target OR a prefix followed by a dot, so this one word covers every
/// verb: `cli.audit` (audit-stored, audit-range, screen, elite, descend,
/// range-all), `cli.sweep` (sweep-stored, sweep-all) and `cli.auto`
/// (auto-stored).
///
/// It was `cli.audit`, and that narrowed the fallback to six of the nine verbs.
/// `sweep-stored` and `sweep-all` — the two an operator reaches for to sweep one
/// stored month — stamp `cli.sweep`, so the console went on reporting an idle
/// machine for exactly the runs this was written to make visible.
const CLI_SWEEP_TARGET: &str = "cli";

/// How long a silence may run before the page should doubt the sweep.
///
/// A start without sufficiently recent correlated activity becomes unknown.
/// Uncorrelated legacy rung events cannot refresh a different command's token.
/// This never turns silence into a terminal success or a process-death claim.
const STALE_AFTER_MILLIS: i64 = 15 * 60 * 1_000;

/// A sweep this process did not start, recovered from the telemetry log.
///
/// # Why the log and not the ledger
///
/// `results/runs.bin` gains a row when a rung FINISHES. A sweep four hours into
/// its first rung has written nothing there, which is exactly the case the
/// console needs to show. The telemetry log is the only surface a running CLI
/// sweep touches WHILE it runs — one record per rung entered and ten more as
/// the grid advances — so it is where "still moving" lives.
///
/// # It reports what it can prove and nothing more
///
/// `running` is deliberately NOT `Progress::to_json`'s shape. A `Progress`
/// carries a `Kind`, an attempt number and a support ppm this process never
/// chose and cannot read off a log line without inventing them. Naming the
/// source in `where` lets the page say *"a sweep is running outside this
/// console"* instead of implying it owns one.
fn status_tail(
    dir: &std::path::Path,
    target: &str,
    run: Option<u64>,
    limit: usize,
) -> telemetry::Tail {
    let query = telemetry::Query::last(limit)
        .from_target(target.to_owned())
        .scanning_at_most(crate::logs::SCAN_BYTES);
    let query = if let Some(run) = run {
        query.from_run(run)
    } else {
        query
    };
    settled_tail(|| telemetry::tail(dir, telemetry::DEFAULT_KEEP_FILES, &query))
}

/// One read, and one more only when the first ended on a partial line.
///
/// A partial last line is what a reader sees while the CLI is inside the
/// `write` of its newest record. Counted as damage on one read, it refused a
/// browser launch and turned `/backtest/run.json` to `unknown` for no fault at
/// all (conc9-2, D-1774). A torn line left by a writer that DIED is still
/// there on the second read (only the next writer to open terminates it), so
/// it still refuses: that fragment may be a newer marker, which is why
/// `partial_tail` is a fault. The second read narrows the window to a write
/// that spans both reads; it does not close it, and no lock is taken to close
/// it, because probing `events.lock` would refuse a CLI that opens its sink at
/// that instant. At most two bounded reads.
fn settled_tail(read: impl Fn() -> telemetry::Tail) -> telemetry::Tail {
    let first = read();
    if first.partial_tail { read() } else { first }
}

fn tail_fault(tail: &telemetry::Tail) -> Option<String> {
    tail_fault_unless_answered(tail, false)
}

/// [`tail_fault`], for a caller that already holds the record it searched for.
///
/// THE SCAN CAP IS A FAULT ONLY WHEN THE ANSWER WAS NOT FOUND. The walk runs
/// newest first, so a record it returned is the newest of its kind whatever
/// lies past the cap: the cap says older bytes went unread, and nothing older
/// can outrank what was found. Treating it as damage regardless made every
/// browser launch refuse, and `/backtest/run.json` say `unknown`, whenever the
/// newest 4 MiB of a healthy CLI log held fewer than 256 lifecycle records, a
/// sweep's own marker among them (W1-api6-4, D-0954). Every other fault —
/// unreadable files, malformed or clipped records, a partial tail — still
/// counts, answered or not, because each can hide a record NEWER than the one
/// found.
fn tail_fault_unless_answered(tail: &telemetry::Tail, answered: bool) -> Option<String> {
    let clipped = tail
        .records
        .iter()
        .any(|row| row.cut || row.dropped_fields != 0);
    if !tail.errors.is_empty()
        || tail.malformed > 0
        || tail.partial_tail
        || (tail.hit_scan_cap && !answered)
        || clipped
    {
        Some(format!(
            "external sweep log is incomplete: {} unreadable files, {} malformed records, partial tail {}, scan cap {}, clipped records {}. {}",
            tail.errors.len(),
            tail.malformed,
            tail.partial_tail,
            tail.hit_scan_cap,
            clipped,
            tail.errors.join("; ")
        ))
    } else {
        None
    }
}

/// Damaged or unanswerable evidence: never clears a launch.
fn uncertain_observation(at_millis: Option<i64>, why: &str) -> ExternalObservation {
    ExternalObservation {
        at_millis,
        uncertain: true,
        in_flight: false,
        launch_clear: false,
        unterminated: None,
        body: unknown_status(why),
    }
}

/// The bounded lifecycle window and the index of its newest sweep-command
/// marker, the window trimmed to end at that marker when damage behind it is
/// all that would otherwise refuse (CE-11, D-1914).
fn newest_sweep_marker(dir: &std::path::Path) -> (telemetry::Tail, Option<usize>) {
    marker_window(|limit| status_tail(dir, "cli.lifecycle", None, limit))
}

/// [`newest_sweep_marker`] over the bounded read it makes at most twice:
/// `read(limit)` answers the newest `limit` lifecycle records of the log.
///
/// A PARAMETER SO THE RACE IT GUARDS CAN BE STAGED. The same-record check below
/// exists for a CLI that appends a lifecycle record BETWEEN the window read and
/// the trimmed re-read. No single-threaded fixture lands a record there, so
/// with the read hard-wired no test could reach the check, and a mutant that
/// let any trimmed walk through survived (R1286-api-01, D-4130). Production
/// passes the same `status_tail` call the body used to make; nothing changed
/// about what is read or how often.
fn marker_window(
    mut read: impl FnMut(usize) -> telemetry::Tail,
) -> (telemetry::Tail, Option<usize>) {
    let mut lifecycle = read(256);
    let mut found = None;
    for (index, record) in lifecycle.records.iter().enumerate() {
        let is_marker = matches!(
            record.message.as_str(),
            "command started" | "command finished"
        ) && match record.field("command") {
            Some(telemetry::OwnedValue::Str(command)) => cli::is_sweep_command(command),
            // A malformed marker is not an inspection command we can skip.
            // Retain it so status becomes unknown instead of borrowing an
            // older sweep's successful completion.
            _ => true,
        };
        if is_marker {
            found = Some(index);
            break;
        }
    }
    // ONLY DAMAGE NEWER THAN THE MARKER CAN HIDE A NEWER MARKER (CE-11,
    // D-1914). File order is sequence order, so a line torn by a killed CLI
    // command and stepped over BEHIND the newest marker cannot outrank it, yet
    // it blocked every browser launch until ~128 further commands pushed it out
    // of the window. The window is re-walked to stop exactly at the marker, so
    // only what lies between it and the newest end is judged. A walk that no
    // longer ends on the same record (a newer one landed in between) keeps the
    // whole window's verdict.
    if let Some(index) = found
        && tail_fault_unless_answered(&lifecycle, true).is_some()
    {
        let through = read(index.saturating_add(1));
        let same = |tail: &telemetry::Tail| tail.records.get(index).map(|record| record.seq);
        if same(&through).is_some() && same(&through) == same(&lifecycle) {
            lifecycle = through;
        }
    }
    (lifecycle, found)
}

fn observe_elsewhere(dir: &std::path::Path, now: i64) -> ExternalObservation {
    // Durable run/probe evidence uses this same target. Its completion cannot
    // replace a whole-command marker, nor can its uncorrelated token refresh
    // that command's activity. Search a bounded retained window explicitly.
    let (lifecycle, found) = newest_sweep_marker(dir);
    let marker = found.and_then(|index| lifecycle.records.get(index));
    // A marker found inside the scanned window is the newest one, so the scan
    // cap cannot hide a newer one. With no marker found the cap is still the
    // answer: the latest sweep's marker may lie in the bytes it left unread.
    if let Some(why) = tail_fault_unless_answered(&lifecycle, marker.is_some()) {
        return uncertain_observation(None, &why);
    }
    let Some(marker) = marker else {
        let legacy = status_tail(dir, CLI_SWEEP_TARGET, None, 1);
        let fault = tail_fault(&legacy);
        let launch_clear = fault.is_none();
        let why = fault.or_else(|| legacy.records.first().map(|record| {
            format!("latest CLI event: {}. The bounded 256-event lifecycle window has no authoritative sweep-command marker; completion and current activity are unknown", record.message)
        }));
        return ExternalObservation {
            at_millis: legacy.records.first().map(|record| record.at_unix_millis),
            uncertain: why.is_some(),
            in_flight: false,
            launch_clear,
            unterminated: None,
            body: why.map_or_else(|| NO_SWEEP.to_owned(), |why| unknown_status(&why)),
        };
    };
    let activity = status_tail(dir, CLI_SWEEP_TARGET, Some(marker.run), 1);
    // ANSWERED BY THE MARKER when the walk found no activity. The marker lies
    // inside the window this walk also reads from its newest end, and a run's
    // activity follows its `command started`, so any activity newer than the
    // marker is found before the cap. Reaching the cap empty-handed means none
    // is newer, and `last` falls back to the marker, the newest fact there is.
    if let Some(why) = tail_fault_unless_answered(&activity, true) {
        return uncertain_observation(Some(marker.at_unix_millis), &why);
    }
    let last = activity.records.first().unwrap_or(marker);
    let phase = match marker.field("phase") {
        Some(telemetry::OwnedValue::Str(phase)) => phase.as_str(),
        _ => "unknown",
    };
    let named_sweep = matches!(marker.field("command"), Some(telemetry::OwnedValue::Str(command)) if cli::is_sweep_command(command));
    // SIGNED. The CLI's sink clamps stamps up to a floor it resumes from disk,
    // so after a backward clock step its events can be stamped AHEAD of this
    // clock. `.max(0)` used to read that as "age 0", and a dead CLI stayed
    // "running" for the size of the step. A negative age is now named as an
    // unageable one. D-2755 (CE-87).
    let age = now.saturating_sub(last.at_unix_millis);
    let ahead = (age < 0).then(|| {
        format!(
            "the newest CLI event is stamped {} ms ahead of this server's clock, so its activity cannot be aged; silence is not completion",
            age.unsigned_abs()
        )
    });
    let (status, why) = match (marker.message.as_str(), phase) {
        ("command started", "running") if ahead.is_some() && marker.run > 0 && named_sweep => {
            ("unknown", ahead.as_deref().unwrap_or_default())
        }
        ("command started", "running")
            if age <= STALE_AFTER_MILLIS && marker.run > 0 && named_sweep =>
        {
            ("running", "")
        }
        ("command finished", "completed") if marker.run > 0 && named_sweep => ("completed", ""),
        ("command finished", "refused") if marker.run > 0 && named_sweep => (
            "refused",
            "the external command reported a refusal; inspect its lifecycle and result logs",
        ),
        _ => (
            "unknown",
            "the external command has no usable terminal receipt or recent activity; silence is not completion",
        ),
    };
    let body = format!(
        r#"{{"running":{{"where":"cli","status":"{status}","observation_scope":"latest-command","in_flight":{},"attempt":{},"attempt_key":"{}","run":{},"message":{},"at_unix_millis":{},"age_millis":{age},"stale_after_millis":{},"why":{},"refusal":{},"report":{}}}}}"#,
        status == "running",
        marker.run,
        marker.run,
        marker.run,
        crate::logs::quoted(&last.message),
        last.at_unix_millis,
        STALE_AFTER_MILLIS,
        crate::render::json_string(why),
        if status == "refused" {
            crate::render::json_string(why)
        } else {
            "null".to_owned()
        },
        if status == "completed" {
            crate::render::json_string(
                "The external command recorded successful completion. Individual result receipts determine which outputs committed.",
            )
        } else {
            "null".to_owned()
        }
    );
    let unterminated = unterminated_marker(marker, named_sweep);
    ExternalObservation {
        at_millis: Some(last.at_unix_millis),
        uncertain: status == "unknown",
        in_flight: status == "running",
        launch_clear: matches!(status, "completed" | "refused"),
        unterminated,
        body,
    }
}

/// A named sweep's newest `command started` under a durable invocation id,
/// as `(id, command)`; anything else is `None`. D-2764.
fn unterminated_marker(marker: &telemetry::Record, named_sweep: bool) -> Option<(u64, String)> {
    match marker.field("command") {
        Some(telemetry::OwnedValue::Str(command))
            if marker.message == "command started"
                && named_sweep
                && marker.run > cli::operation_audit::ID_BASE =>
        {
            Some((marker.run, command.clone()))
        }
        _ => None,
    }
}

/// The durable invocation a lease holder may prove is not running, if any.
///
/// # Why holding the lease is the proof
///
/// `cli::run_durable` takes the store's execution lease BEFORE it begins its
/// durable invocation, writes `command started` inside it, and releases the
/// lease only after the invocation's terminal is written. The kernel releases
/// the flock when its holder dies. So a caller that holds THIS store's lease,
/// and finds the newest marker to be a `command started` whose durable id names
/// a CLI invocation of the same command in THIS store's audit, has found a
/// command that cannot be running here: a Ctrl-C, a kill, an OOM or a panic
/// ended it without its `command finished`. Before D-2764 that marker refused
/// every browser launch forever, while the lease the refusal guarded was free.
///
/// Everything short of that proof keeps refusing: a legacy run id outside the
/// durable namespace (older binaries, bypassing callers), an id this store's
/// audit does not hold, a different origin or command word, and any audit read
/// that fails. It claims nothing about HOW the command ended and rewrites no
/// history: the status document still reports what the log shows. D-2764.
fn ended_without_terminal(root: &std::path::Path, observed: &ExternalObservation) -> Option<u64> {
    let (run, command) = observed.unterminated.as_ref()?;
    match cli::operation_audit::read(root, *run) {
        Ok(Some(record))
            if record.origin == cli::operation_audit::Origin::Cli && record.label == *command =>
        {
            Some(*run)
        }
        _ => None,
    }
}

/// The external-evidence half of admission, for a caller that HOLDS `root`'s
/// execution lease. D-2764.
fn admit_external(root: &std::path::Path, observed: &ExternalObservation) -> Result<(), Refusal> {
    if observed.launch_clear {
        return Ok(());
    }
    // NOT SILENT: the status poll's `admission.why` names the invocation it
    // found ended, and the log and the invocation audit are left as they are.
    if ended_without_terminal(root, observed).is_none() {
        return Err(Refusal::Unobservable(
            "the external command evidence still reports activity or is damaged/unconfirmed; inspect the execution-status note before starting another run".to_owned(),
        ));
    }
    Ok(())
}

/* ==================================================================
EVERY OTHER COMMAND THE ENGINE HAS, AND WHY THEY SHARE ONE ROUTE
================================================================== */

/// One engine command the browser may start, with its arguments already
/// checked.
///
/// # Why a dispatcher and not seven routes
///
/// Every variant below does the same four things: refuse a malformed body,
/// refuse an unstamped build, take the ONE slot that serialises writers to the
/// append-only ledger, and hand `cli` a validated call. Seven handlers would be
/// seven copies of that sequence, and the slot discipline is exactly the kind of
/// thing that gets forgotten in the seventh copy. `CLAUDE.md` §4's ban on a
/// silent fallback applies to a route that forgets the busy check as much as to
/// one that swallows an error.
///
/// The dispatch is CLOSED — a fixed enum, parsed from a `command` word against
/// a fixed list — so it is not a generic "run anything" surface. An unknown
/// word is refused by name and lists what is accepted.
///
/// # THE THREE COMMANDS DELIBERATELY ABSENT
///
/// `sweep`, `audit` and `auto` run over **generated** bars. `CLAUDE.md` §5 makes
/// the provenance banner *"the only thing separating"* a real sweep from an
/// invented one, and
/// `the_generated_and_stored_banners_make_opposite_claims` fails the build if
/// the two ever converge. Putting a synthetic-data command on a console whose
/// every other surface reads the store is an invitation to read a generated
/// figure as a measured one — the failure wearing a success's clothes §4 bans,
/// arriving through the front door. They stay terminal-only, where the operator
/// typed the word and knows what they asked for. D-0300.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    /// One index, all declared Boolean programs and the fixed signal-candle stop.
    IndexStopQualifiedSearch {
        /// Exact selected timeframes and original/later observation periods.
        request: Box<crate::indexstoplaunch::Asked>,
    },
    /// Fixed AND/OR/NOT grammar with one declared search-wide research policy.
    BooleanQualifiedSearch {
        /// All selected families and both explicit observation periods.
        request: Box<crate::booleanlaunch::Asked>,
    },
    /// One rung, full validated audit — walk-forward, PBO and the bootstrap.
    AuditRange {
        /// Feed word, instrument and rung.
        span: AskedRung,
        /// Absolute hit floor for this rung.
        min_hits: u64,
    },
    /// One checksum-admitted range using explicit server-owned physical limits.
    AuditAuditedRange {
        /// Feed word, instrument and intraday rung.
        span: AskedRung,
        /// Explicit positive support count.
        min_hits: u64,
        /// Supported existing runtime settings, fixed for this command's lifetime.
        knobs: Vec<(&'static str, String)>,
    },
    /// The elite rules applied at ONE fixed support.
    Screen {
        /// Feed word, instrument and rung.
        span: AskedRung,
        /// The threshold, in parts per million of the rung's own bars.
        support_ppm: u64,
        /// The operator's stop ceiling, in whole index points.
        max_points: i64,
        /// How many rows to list.
        top: usize,
    },
    /// The threshold search over stored bars.
    AutoStored {
        /// Feed word, instrument and rung.
        span: AskedRung,
    },
    /// One stored month's raw ladder walk.
    SweepStored {
        /// Feed word, instrument and rung.
        span: AskedRung,
        /// Absolute hit floor.
        min_hits: u64,
    },
    /// Every stored month for one feed and rung, in one batch.
    SweepAll {
        /// The feed's directory word.
        feed: String,
        /// The rung.
        rung: String,
        /// Absolute hit floor.
        min_hits: u64,
    },
}

/// A feed, an instrument, a rung and a span — what four of the five need.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AskedRung {
    /// The feed's directory word.
    pub feed: String,
    /// The instrument.
    pub underlying: String,
    /// The rung.
    pub rung: String,
    /// First month.
    pub from: (u16, u8),
    /// Last month.
    pub to: (u16, u8),
}

impl Command {
    /// The word this command was asked for by, for the page and the log.
    #[must_use]
    pub const fn word(&self) -> &'static str {
        match *self {
            Self::IndexStopQualifiedSearch { .. } => crate::indexstoplaunch::COMMAND,
            Self::BooleanQualifiedSearch { .. } => crate::booleanlaunch::COMMAND,
            Self::AuditRange { .. } => "audit-range",
            Self::AuditAuditedRange { .. } => "audit-audited-range",
            Self::Screen { .. } => "screen",
            Self::AutoStored { .. } => "auto-stored",
            Self::SweepStored { .. } => "sweep-stored",
            Self::SweepAll { .. } => "sweep-all",
        }
    }

    /// Whether this ordinary command's run prices the exit-grid screen, and so
    /// is refused by `cli` when `BRUTEX_SCREEN_BUDGET_MS` is usable. D-0685.
    ///
    /// `audit-range` reaches `audit_range_kernel` and `screen` reaches
    /// `screen_range_kernel`, and both refuse the budget first. `auto-stored`,
    /// `sweep-stored` and `sweep-all` sweep without pricing a screen, so a
    /// budget cannot change what they record and they are not refused for one.
    /// The strict word refuses the budget through its own admission, and the
    /// two declared searches run other engines.
    ///
    /// THIS IS STILL A RESTATEMENT, of which `cli` kernels call
    /// `cli::recorded_budget_refusal`, and it is named as one. Unlike the
    /// budget rule it cannot be one call: the kernels sit behind five `cli`
    /// entry points, and asking one of them from here would run it. What is
    /// pinned is this crate's side: under a usable server budget the two words
    /// here answer the 503, and every other word reaches its own admission at
    /// the route (`server_budget_child`). A kernel that starts or stops
    /// refusing a budget must change this list. D-0695.
    const fn prices_a_screen(&self) -> bool {
        matches!(*self, Self::AuditRange { .. } | Self::Screen { .. })
    }

    /// The instrument this command reports against.
    ///
    /// `sweep-all` walks every instrument the feed holds at one rung, so it has
    /// none — and `ALL` is written rather than an empty string, because a blank
    /// on the page reads as a field that failed to load.
    #[must_use]
    pub fn underlying(&self) -> &str {
        match *self {
            Self::AuditRange { ref span, .. }
            | Self::AuditAuditedRange { ref span, .. }
            | Self::Screen { ref span, .. }
            | Self::AutoStored { ref span }
            | Self::SweepStored { ref span, .. } => &span.underlying,
            Self::SweepAll { .. } => "ALL",
            Self::IndexStopQualifiedSearch { ref request } => &request.index,
            Self::BooleanQualifiedSearch { .. } => "SELECTED",
        }
    }

    /// The feed word.
    #[must_use]
    pub fn feed(&self) -> &str {
        match *self {
            Self::AuditRange { ref span, .. }
            | Self::AuditAuditedRange { ref span, .. }
            | Self::Screen { ref span, .. }
            | Self::AutoStored { ref span }
            | Self::SweepStored { ref span, .. } => &span.feed,
            Self::SweepAll { ref feed, .. } => feed,
            Self::IndexStopQualifiedSearch { ref request } => &request.feed,
            Self::BooleanQualifiedSearch { ref request } => &request.feed,
        }
    }

    /// The span, or the whole store for a batch.
    #[must_use]
    pub fn window(&self) -> ((u16, u8), (u16, u8)) {
        match *self {
            Self::AuditRange { ref span, .. }
            | Self::AuditAuditedRange { ref span, .. }
            | Self::Screen { ref span, .. }
            | Self::AutoStored { ref span }
            | Self::SweepStored { ref span, .. } => (span.from, span.to),
            // A batch is not a span, and (0,1)..(0,1) is a value no real month
            // can take -- year zero -- so the page cannot render it as one.
            Self::SweepAll { .. } => ((0, 1), (0, 1)),
            Self::IndexStopQualifiedSearch { ref request } => request.window(),
            Self::BooleanQualifiedSearch { ref request } => request.window(),
        }
    }
}

/// A whole number field, or the refusal naming it.
fn whole<T: std::str::FromStr>(body: &WireBody, name: &str, what: &str) -> Result<T, Refusal> {
    field(body, name)
        .and_then(|text| text.parse::<T>().ok())
        .ok_or_else(|| Refusal::Malformed(format!("`{name}` must be {what}.")))
}

/// A positive hit floor shared by every command that accepts `min_hits`.
///
/// The engine defensively raises zero to one, but the HTTP boundary must not
/// accept a value the computation will change. Refusing here keeps the request,
/// audit trail, and run identity about the value that actually runs.
fn positive_min_hits(body: &WireBody) -> Result<u64, Refusal> {
    let min_hits: u64 = whole(body, "min_hits", "a whole number of bars a mask must hit")?;
    if min_hits == 0 {
        return Err(Refusal::Malformed(
            "`min_hits` must be 1 or more; 0 would disable extinction and is not silently \
             changed to 1."
                .to_owned(),
        ));
    }
    Ok(min_hits)
}

/// The feed, instrument, rung and span every span-taking command needs.
fn rung_from(body: &WireBody) -> Result<AskedRung, Refusal> {
    // DELEGATES FOR THE FEED AND THE SPAN, exactly as `descent_from` does. One
    // month bound, one place.
    let asked = asked_from_wire(body)?;
    let rung = field(body, "rung")
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            Refusal::Malformed(format!(
                "`rung` is required. The eight are: {}.",
                EVERY_RUNG.join(", ")
            ))
        })?;
    if !EVERY_RUNG.contains(&rung.as_str()) {
        return Err(Refusal::Malformed(format!(
            "`{rung}` is not a rung this engine sweeps. The eight are: {}.",
            EVERY_RUNG.join(", ")
        )));
    }
    Ok(AskedRung {
        feed: asked.feed,
        underlying: asked.underlying,
        rung,
        from: asked.from,
        to: asked.to,
    })
}

/// Every command word this route accepts, in the order the refusal lists them.
const EVERY_COMMAND: [&str; 8] = [
    crate::indexstoplaunch::COMMAND,
    crate::booleanlaunch::COMMAND,
    "audit-range",
    "audit-audited-range",
    "screen",
    "auto-stored",
    "sweep-stored",
    "sweep-all",
];

/// The command request body, or the first thing wrong with it.
///
/// # Errors
///
/// An unknown or missing `command`, anything [`rung_from`] refuses, and any
/// numeric field a command needs that is absent or not a whole number. A
/// generated-bar command is refused with the reason rather than as an unknown
/// word — an operator who asks for `sweep` deserves to be told WHY it is not
/// here, not merely that it is not.
pub fn command_from(body: &str) -> Result<Command, Refusal> {
    let wire = wire_body(body)?;
    if wire.string("command") == Some(crate::indexstoplaunch::COMMAND) {
        return crate::indexstoplaunch::parse(body)
            .map(|request| Command::IndexStopQualifiedSearch {
                request: Box::new(request),
            })
            .map_err(Refusal::Malformed);
    }
    if wire.string("command") == Some(crate::booleanlaunch::COMMAND) {
        return crate::booleanlaunch::parse(body)
            .map(|request| Command::BooleanQualifiedSearch {
                request: Box::new(request),
            })
            .map_err(Refusal::Malformed);
    }
    command_from_wire(&wire)
}

/// [`command_from`] after strict JSON decoding and duplicate detection.
fn command_from_wire(body: &WireBody) -> Result<Command, Refusal> {
    // EVERY ORDINARY WORD, NOT ONLY THE STRICT ONE. `audit-range` and `screen`
    // run the exit-grid screen a budget would bound, and every word here
    // records; a budget named in the body is refused by name before the word
    // is read, as [`asked_from`] refuses it. `strict_knobs` keeps its own
    // refusal for the strict word. D-0685.
    refuse_screen_budget(body)?;
    let word = field(body, "command")
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            Refusal::Malformed(format!(
                "`command` is required. This route accepts: {}.",
                EVERY_COMMAND.join(", ")
            ))
        })?;

    // NAMED, NOT LUMPED WITH A TYPO. These three exist and are deliberately not
    // here; saying so is the difference between a rule and a gap.
    if matches!(word.as_str(), "sweep" | "audit" | "auto") {
        return Err(Refusal::Malformed(format!(
            "`{word}` runs over GENERATED bars, and this console reads the \
             store on every other surface. CLAUDE.md §5 makes the provenance \
             banner the only thing separating a real sweep from an invented \
             one, so a synthetic-data command is not offered here. Run it from \
             a terminal, where you typed the word. The stored equivalents are: \
             {}.",
            EVERY_COMMAND.join(", ")
        )));
    }

    if let Some((reads, knobs)) = command_reads(&word) {
        refuse_unread(body, &format!("the `{word}` command"), reads, knobs)?;
    }

    match word.as_str() {
        "audit-range" => Ok(Command::AuditRange {
            span: rung_from(body)?,
            min_hits: positive_min_hits(body)?,
        }),
        "audit-audited-range" => Ok(Command::AuditAuditedRange {
            span: rung_from(body)?,
            min_hits: positive_min_hits(body)?,
            knobs: strict_knobs(body)?,
        }),
        "screen" => {
            let support_ppm: u64 = whole(
                body,
                "support_ppm",
                "a whole number of parts per million of this rung's own bars",
            )?;
            if support_ppm == 0 {
                return Err(Refusal::Malformed(
                    "`support_ppm` is 0. Every combination is then frequent, \
                     the frontier never empties and the walk has no end."
                        .to_owned(),
                ));
            }
            let max_points: i64 = whole(body, "max_points", "a whole number of index points")?;
            let top: usize = whole(body, "top", "a whole number of rows to list")?;
            // Zero is no ceiling, as at the descent door and in `cli` (D-1732).
            if max_points < 0 || top == 0 {
                return Err(Refusal::Malformed(
                    "`max_points` must be 0 (no ceiling) or more index points \
                     and `top` must be 1 row or more."
                        .to_owned(),
                ));
            }
            Ok(Command::Screen {
                span: rung_from(body)?,
                support_ppm,
                max_points,
                top,
            })
        }
        "auto-stored" => Ok(Command::AutoStored {
            span: rung_from(body)?,
        }),
        "sweep-stored" => Ok(Command::SweepStored {
            span: rung_from(body)?,
            min_hits: positive_min_hits(body)?,
        }),
        "sweep-all" => {
            // THE FEED ALONE. This called `asked_from_wire`, which REQUIRED an
            // instrument and a span that `cli sweep-all` does not take, then
            // kept only the feed. Naming either is refused above. D-1972.
            let feed = feed_from(body)?;
            let rung = field(body, "rung")
                .filter(|s| EVERY_RUNG.contains(&s.as_str()))
                .ok_or_else(|| {
                    Refusal::Malformed(format!(
                        "`rung` is required and must be one of: {}.",
                        EVERY_RUNG.join(", ")
                    ))
                })?;
            Ok(Command::SweepAll {
                feed,
                rung,
                min_hits: positive_min_hits(body)?,
            })
        }
        other => Err(Refusal::Malformed(format!(
            "`{other}` is not a command this route runs. It accepts: {}.",
            EVERY_COMMAND.join(", ")
        ))),
    }
}

/// The members an ordinary command word reads, or `None` for a word
/// [`command_from_wire`] refuses on its own. See [`WIRE_FIELDS`]. D-1972.
fn command_reads(word: &str) -> Option<(&'static [&'static str], Knobs)> {
    match word {
        "audit-range" | "sweep-stored" => Some((&COMMAND_SPAN_RUNG_HITS, Knobs::NotApplied)),
        // `strict_knobs` validates every knob and refuses its two by name.
        "audit-audited-range" => Some((&COMMAND_SPAN_RUNG_HITS, Knobs::Applied)),
        "screen" => Some((&COMMAND_SCREEN, Knobs::NotApplied)),
        "auto-stored" => Some((&COMMAND_SPAN_RUNG, Knobs::NotApplied)),
        "sweep-all" => Some((&COMMAND_SWEEP_ALL, Knobs::NotApplied)),
        _ => None,
    }
}

/// Runs one command and records what it produced.
///
/// **Blocking on purpose**, exactly as [`conduct`] and [`conduct_descent`] are.
/// Every arm here reads stored bars and several walk a ladder over them.
#[must_use]
pub fn conduct_command(asked: &Command, now_micros: i64, attempt: u64) -> Progress {
    let (from, to) = asked.window();
    let mut progress = Progress::started(
        asked.feed(),
        asked.underlying(),
        from,
        to,
        None,
        now_micros,
        attempt,
    )
    .of_kind(Kind::Command);
    let text = match *asked {
        Command::IndexStopQualifiedSearch { .. } => {
            "refused: single-stop research requires its prepared audited browser dispatch".into()
        }
        Command::BooleanQualifiedSearch { .. } => {
            "refused: declared Boolean research requires its prepared audited browser dispatch"
                .into()
        }
        Command::AuditAuditedRange { .. } => {
            let configuration = cli::audited_range_command::StrictConfig::from_env();
            let root = crate::server::store_dir();
            return match (configuration, root) {
                (Ok(configuration), Ok(root)) => {
                    conduct_strict_command(asked, now_micros, attempt, &configuration, &root)
                }
                (Err(why), _) => {
                    settle(&mut progress, format!("refused: {why}"), now_micros);
                    progress
                }
                (_, Err(why)) => {
                    settle(&mut progress, format!("refused: {why}"), now_micros);
                    progress
                }
            };
        }
        Command::AuditRange { ref span, min_hits } => cli::audit_range(
            &span.feed,
            &span.underlying,
            &span.rung,
            span.from,
            span.to,
            min_hits,
        ),
        // POINTS, NEVER PPM, for the reason `elite_descend_in_points` records.
        Command::Screen {
            ref span,
            support_ppm,
            max_points,
            top,
        } => cli::screen_range_in_points(
            &span.feed,
            &span.underlying,
            &span.rung,
            (span.from, span.to),
            support_ppm,
            max_points,
            top,
        ),
        Command::AutoStored { ref span } => {
            cli::auto_stored(&span.feed, &span.underlying, &span.rung, span.from, span.to)
        }
        // ONE MONTH, AND IT IS THE SPAN'S FIRST. `cli::sweep_stored` takes a
        // year and a month rather than a range, so a request naming a longer
        // span would silently sweep only its opening month. The `to` is
        // therefore required to equal the `from` rather than ignored.
        Command::SweepStored { ref span, min_hits } => {
            if span.from == span.to {
                cli::sweep_stored(
                    &span.feed,
                    &span.underlying,
                    &span.rung,
                    span.from.0,
                    span.from.1,
                    min_hits,
                )
            } else {
                format!(
                    "refused: `sweep-stored` walks ONE month, and this asked for \
                     {}-{:02}..{}-{:02}. Ignoring the rest would sweep a shorter \
                     span than the request names and record it under the \
                     request's identity. Ask for one month, or use `audit-range` \
                     for a span.\n",
                    span.from.0, span.from.1, span.to.0, span.to.1
                )
            }
        }
        Command::SweepAll {
            ref feed,
            ref rung,
            min_hits,
        } => cli::batch::sweep_all(feed, rung, min_hits),
    };
    settle(&mut progress, text, now_micros);
    progress
}

/// `POST /engine/command` — start any stored-data engine command.
///
/// See [`Command`] for why one route serves five, and for the three that are
/// deliberately not among them.
pub async fn command(
    axum::extract::State(site): axum::extract::State<crate::server::Loaded>,
    body: String,
) -> (axum::http::StatusCode, JsonHeaders, String) {
    match crate::detail::run(move || command_with(&site, &body, cli::commit_stamp())).await {
        Ok(response) => response,
        Err(why) => refused(&Refusal::Unobservable(format!(
            "bounded command admission is unavailable: {why:?}; no engine command was dispatched"
        ))),
    }
}

/// [`command`], with the build's commit stamp passed in.
///
/// Split for the reason [`run_with`] gives, and see [`stamp_refusal`] for why
/// that reason survived `build.rs` changing which arm is the reachable one.
pub(crate) fn command_with(
    site: &crate::server::Loaded,
    body: &str,
    stamp: Option<&str>,
) -> (axum::http::StatusCode, JsonHeaders, String) {
    command_with_configuration(
        site,
        body,
        stamp,
        cli::audited_range_command::StrictConfig::from_env,
    )
}

#[expect(
    clippy::large_enum_variant,
    reason = "the shared lease admits one fixed-size prepared command; inline payloads avoid a separate per-command allocation"
)]
enum PreparedCommand {
    Boolean(cli::boolean_search_launch::Admission),
    IndexStop(cli::index_stop_search::Launch),
}

#[expect(
    clippy::too_many_lines,
    reason = "one shared admission lock and audited dispatch for every browser command"
)]
fn command_with_configuration(
    site: &crate::server::Loaded,
    body: &str,
    stamp: Option<&str>,
    configure: impl FnOnce() -> Result<
        cli::audited_range_command::StrictConfig,
        cli::audited_range_command::ConfigRefusal,
    >,
) -> (axum::http::StatusCode, JsonHeaders, String) {
    let asked = match command_from(body) {
        Ok(asked) => asked,
        Err(why) => {
            note_command_refusal(why.why());
            return refused(&why);
        }
    };

    // Both server refusals: one writer each, with the route (D-4447; D-4627).
    if let Some(why) = stamp_refusal(stamp) {
        return refused_for_this_server(COMMAND_ROUTE, &why);
    }
    // Only the words whose run prices the screen; the strict word refuses the
    // budget at every value through `validate_runtime` below. D-0685.
    if asked.prices_a_screen()
        && let Some(why) = environment_budget_refusal()
    {
        return refused_for_this_server(COMMAND_ROUTE, &why);
    }

    // Resolve only the explicit strict command, once, before claiming a slot or
    // emitting a start. Ordinary commands do not depend on this configuration.
    let strict = if let Command::AuditAuditedRange { ref knobs, .. } = asked {
        if let Err(why) = cli::audited_range_command::validate_runtime(knobs) {
            return strict_settings_refused(&why);
        }
        match configure() {
            Ok(config) => Some(config),
            Err(why) => return strict_configuration_refused(&why),
        }
    } else {
        None
    };

    let busy = || {
        refuse_logged(
            "a command was refused while a run was in flight",
            &Refusal::Busy(
                "a sweep, descent or command is already running in this \
                 process. All of them append to the same append-only ledger, \
                 and two finishing together can interleave two records, so the \
                 second press is refused rather than queued."
                    .to_owned(),
            ),
        )
    };
    let admitted = admit(site, busy, || {
        let uses_configured_store = !matches!(
            asked,
            Command::AuditAuditedRange { .. }
                | Command::BooleanQualifiedSearch { .. }
                | Command::IndexStopQualifiedSearch { .. }
        );
        let lease = claim_execution(site, uses_configured_store)
            .map_err(|why| refuse_logged("a command was refused at its execution lease", &why))?;
        let launch = match &asked {
            Command::BooleanQualifiedSearch { request } => {
                crate::booleanlaunch::prepare(request, &site.store_root)
                    .map(|value| Some(PreparedCommand::Boolean(value)))
            }
            Command::IndexStopQualifiedSearch { request } => {
                crate::indexstoplaunch::prepare(request, &site.store_root)
                    .map(|value| Some(PreparedCommand::IndexStop(value)))
            }
            _ => Ok(None),
        }
        .map_err(|why| {
            refuse_logged(
                "a command was refused while preparing its launch",
                &Refusal::Unobservable(why),
            )
        })?;
        let (from, to) = asked.window();
        let mut audit = reserve_invocation(site, asked.word()).map_err(|why| {
            refuse_logged("a command was refused at its invocation journal", &why)
        })?;
        let attempt = audit.id();
        let started = now_micros();
        let mut accepted = Progress::started(
            asked.feed(),
            asked.underlying(),
            from,
            to,
            None,
            started,
            attempt,
        )
        .of_kind(Kind::Command);
        if let Command::BooleanQualifiedSearch { ref request } = asked {
            accepted.boolean_search = Some(Box::new(crate::booleanlaunch::Status::new(request)));
        }
        if let Command::IndexStopQualifiedSearch { ref request } = asked {
            accepted.index_stop = Some(Box::new(crate::indexstoplaunch::Status::new(request)));
        }
        if let Some(why) = marker_refusal(emit_attempt_started(&accepted)) {
            let _terminal = audit.finish(cli::operation_audit::Phase::Refused, 0);
            return Err(refuse_logged(
                "a command was refused because its start could not be recorded",
                &why,
            ));
        }
        Ok((accepted, (started, attempt, audit, launch, lease)))
    });
    let (started, attempt, audit, launch, lease) = match admitted {
        Ok(admitted) => admitted,
        Err(answer) => return answer,
    };

    let _ = telemetry::emit_if!(
        telemetry::Level::Info,
        "api.sweep",
        "an engine command was accepted from the browser",
        "command" => telemetry::Value::Str(asked.word()),
        "feed" => telemetry::Value::Str(asked.feed()),
        "underlying" => telemetry::Value::Str(asked.underlying()),
        "attempt" => telemetry::Value::Uint(attempt),
    );

    // MONOTONIC, for the duration only: `started` stays the wall-clock stamp
    // the page shows, and an NTP step during the run cannot make the recorded
    // `elapsed_micros` a clamped 0 or an inflated figure. D-2754 (CE-86).
    let began = std::time::Instant::now();
    let guard = TaskFinisher::audited(std::sync::Arc::clone(site), audit).with_lease(lease);
    let store_root = site.store_root.clone();
    let launch_site = std::sync::Arc::clone(site);
    tokio::task::spawn_blocking(move || {
        let finished = guard.conduct(|| match (launch, &asked) {
            (
                Some(PreparedCommand::Boolean(admission)),
                Command::BooleanQualifiedSearch { request },
            ) => crate::booleanlaunch::conduct(request, admission, started, attempt, &launch_site),
            (
                Some(PreparedCommand::IndexStop(admission)),
                Command::IndexStopQualifiedSearch { request },
            ) => {
                crate::indexstoplaunch::conduct(request, admission, started, attempt, &launch_site)
            }
            _ => match strict.as_ref() {
                Some(config) => {
                    conduct_strict_command(&asked, started, attempt, config, &store_root)
                }
                None => conduct_command(&asked, started, attempt),
            },
        });
        let emitted = emit_completion(&finished, asked.word(), began);
        let mut done = finished;
        command_terminal_audit(&asked, &mut done, emitted);
        done.finished_micros = Some(now_micros());
        guard.finish(done);
    });

    (
        axum::http::StatusCode::ACCEPTED,
        json_headers(),
        command_acceptance(attempt),
    )
}

/// The reserved worker token is returned from admission itself, never inferred
/// from the mutable latest-status slot. Decimal text preserves every u64 bit.
fn command_acceptance(attempt: u64) -> String {
    format!(r#"{{"accepted":true,"refusal":null,"attempt":"{attempt}"}}"#)
}

pub(crate) fn command_terminal_audit(
    command: &Command,
    progress: &mut Progress,
    emitted: telemetry::Emitted,
) {
    if matches!(
        command,
        Command::AuditAuditedRange { .. }
            | Command::BooleanQualifiedSearch { .. }
            | Command::IndexStopQualifiedSearch { .. }
    ) {
        strict_terminal_audit(progress, emitted);
    }
}

fn strict_terminal_audit(progress: &mut Progress, emitted: telemetry::Emitted) {
    let reason = match emitted {
        telemetry::Emitted::Written => return,
        telemetry::Emitted::Filtered => "the terminal audit event was filtered by the log level",
        telemetry::Emitted::Dropped => "the terminal audit event could not be written",
        telemetry::Emitted::NotInstalled => {
            "no audit-log sink was installed for the terminal event"
        }
    };
    let original = progress.refusal.take().or_else(|| progress.report.take());
    progress.report = None;
    progress.refusal = Some(
        format!(
            "refused: strict command terminal audit is missing: {reason}. Existing result evidence was not removed. This refusal is visible in this process; it cannot assert a durable terminal event.\n{}",
            original
                .as_deref()
                .unwrap_or("No computation report or refusal was returned.")
        )
        .into(),
    );
}

fn strict_configuration_refused(
    why: &cli::audited_range_command::ConfigRefusal,
) -> (axum::http::StatusCode, JsonHeaders, String) {
    note_command_refusal(&why.to_string());
    (
        axum::http::StatusCode::SERVICE_UNAVAILABLE,
        json_headers(),
        serde_json::json!({
            "accepted": false,
            "started": false,
            "code": "strict_input_configuration_missing_or_invalid",
            "refusal": why.to_string(),
            "missing": why.missing,
            "invalid": why.invalid,
        })
        .to_string(),
    )
}

fn strict_settings_refused(
    why: &cli::audited_range_command::KnobRefusal,
) -> (axum::http::StatusCode, JsonHeaders, String) {
    note_command_refusal(&why.to_string());
    (
        axum::http::StatusCode::SERVICE_UNAVAILABLE,
        json_headers(),
        serde_json::json!({
            "accepted": false,
            "started": false,
            "code": "strict_runtime_settings_invalid",
            "refusal": why.to_string(),
            "invalid": why.invalid,
        })
        .to_string(),
    )
}

fn note_command_refusal(why: &str) {
    let _ = telemetry::emit_if!(
        telemetry::Level::Warn,
        "api.sweep",
        "an engine command was refused before it started",
        "why" => telemetry::Value::Str(why),
    );
}

fn conduct_strict_command(
    asked: &Command,
    started: i64,
    attempt: u64,
    config: &cli::audited_range_command::StrictConfig,
    store_root: &std::path::Path,
) -> Progress {
    let _knobs = strict_knob_context(asked).map(|context| apply_knobs(&context));
    let (from, to) = asked.window();
    let mut progress = Progress::started(
        asked.feed(),
        asked.underlying(),
        from,
        to,
        None,
        started,
        attempt,
    )
    .of_kind(Kind::Command);
    let result = strict_request(asked, attempt, store_root)
        .and_then(|request| cli::audited_range_command::audit(request, config));
    settle_strict_result(&mut progress, result, started);
    progress
}

fn strict_request<'a>(
    asked: &'a Command,
    attempt: u64,
    store_root: &'a std::path::Path,
) -> Result<cli::audited_range_command::Request<'a>, String> {
    match asked {
        Command::AuditAuditedRange { span, min_hits, .. } => {
            Ok(cli::audited_range_command::Request {
                store_root,
                vendor: &span.feed,
                underlying: &span.underlying,
                rung: &span.rung,
                from: span.from,
                to: span.to,
                min_hits: *min_hits,
                attempt: Some(attempt),
            })
        }
        _ => Err(
            "strict input configuration requires the explicit audit-audited-range command"
                .to_owned(),
        ),
    }
}

fn strict_knobs(body: &WireBody) -> Result<Vec<(&'static str, String)>, Refusal> {
    refuse_screen_budget(body)?;
    for field_name in ["support_ppm", "sizing_rate_bp"] {
        if body.scalar(field_name).is_some() {
            return Err(Refusal::Malformed(format!(
                "`{field_name}` does not apply to audit-audited-range: this command uses the explicit min_hits count. No setting was ignored"
            )));
        }
    }
    for (field_name, name) in KNOBS {
        if let Some(raw) = body.scalar(field_name)
            && !cli::audited_range_command::request_value(name, &raw.text())
        {
            return Err(Refusal::Malformed(format!(
                "`{field_name}` is not a usable value for the existing runtime setting; omit it to use the existing policy. No setting was ignored"
            )));
        }
    }
    let mut knobs = knobs_in(body);
    for (name, value) in &mut knobs {
        if *name == "BRUTEX_VALIDATE" && value != "0" {
            "1".clone_into(value);
        }
    }
    Ok(knobs)
}

fn strict_knob_context(asked: &Command) -> Option<Asked> {
    let Command::AuditAuditedRange { span, knobs, .. } = asked else {
        return None;
    };
    Some(Asked {
        feed: span.feed.clone(),
        underlying: span.underlying.clone(),
        from: span.from,
        to: span.to,
        rungs: EVERY_RUNG
            .iter()
            .copied()
            .filter(|rung| *rung == span.rung)
            .collect(),
        knobs: knobs.clone(),
    })
}

fn settle_strict_result(progress: &mut Progress, result: Result<String, String>, finished: i64) {
    progress.report = None;
    progress.refusal = None;
    match result {
        Ok(report) => progress.report = Some(report.into()),
        Err(why) => progress.refusal = Some(format!("refused: {why}").into()),
    }
    progress.finished_micros = Some(finished);
}

#[cfg(test)]
#[path = "strict_sweep_tests.rs"]
mod strict_tests;

/// `GET /engine/top.json` — the ranked frontier, as `cli top` prints it.
///
/// This read takes the shared bounded detail-worker slot but no write/commit
/// gate. `feed` and `underlying` are optional and filter together:
/// `cli top` takes both or neither, and this keeps that shape rather than
/// inventing a third case the CLI has no answer for.
pub async fn top_json(uri: axum::http::Uri) -> (axum::http::StatusCode, JsonHeaders, String) {
    crate::topjson::top_json(uri).await
}

#[cfg(test)]
#[path = "sweeprun_admission_tests.rs"]
mod admission_tests;

#[cfg(test)]
#[expect(
    clippy::expect_used,
    reason = "the exception every test module in this workspace takes: a test \
              that cannot panic cannot fail"
)]
mod tests {
    use super::{
        ABNORMAL_END, Asked, AskedDescent, EVERY_COMMAND, EVERY_RUNG, KNOBS, Kind, Progress,
        Refusal, TaskFinisher, asked_from, attempt_started_event, command_from, completion_audit,
        conduct_command, descent_from, marker_refusal, now_micros, now_millis, settle,
        stamp_refusal, unterminated_marker,
    };

    /// **THE OBSERVATION CLOCK IS THE SYSTEM CLOCK IN MILLISECONDS.** Read
    /// between two system-clock readings, it lies between them. G18-api-22.
    #[test]
    fn the_observation_clock_is_unix_milliseconds() {
        let wall = || {
            i64::try_from(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("after the epoch")
                    .as_millis(),
            )
            .expect("fits")
        };
        let before = wall();
        let got = now_millis();
        let after = wall();
        assert!(
            before <= got && got <= after,
            "{before} <= {got} <= {after}"
        );
    }

    /// **ONLY A NAMED SWEEP'S `command started` UNDER A DURABLE ID IS AN
    /// UNTERMINATED MARKER.** Each of the three conditions is necessary on its
    /// own: another message, an unnamed sweep, or a run id at or below
    /// `ID_BASE` answers `None`. G18-api-23.
    #[test]
    fn an_unterminated_marker_needs_all_three_conditions() {
        let durable = cli::operation_audit::ID_BASE + 7;
        let marker = |message: &str, run: u64| telemetry::Record {
            seq: 0,
            run,
            at_unix_millis: 0,
            at_utc: String::new(),
            level: telemetry::Level::Info,
            target: "cli.command".to_owned(),
            message: message.to_owned(),
            fields: vec![(
                "command".to_owned(),
                telemetry::OwnedValue::Str("sweep-all".to_owned()),
            )],
            cut: false,
            dropped_fields: 0,
        };
        assert_eq!(
            unterminated_marker(&marker("command started", durable), true),
            Some((durable, "sweep-all".to_owned()))
        );
        assert_eq!(
            unterminated_marker(&marker("command finished", durable), true),
            None
        );
        assert_eq!(
            unterminated_marker(&marker("command started", durable), false),
            None
        );
        assert_eq!(
            unterminated_marker(
                &marker("command started", cli::operation_audit::ID_BASE),
                true
            ),
            None
        );
        assert_eq!(
            unterminated_marker(&marker("command started", 3), true),
            None
        );
    }

    /// SF-13 (P12-02, D-1791): a derived threshold reaches the wire as
    /// `null`, never as a magic zero, and a fixed one as its number.
    #[test]
    fn a_derived_threshold_is_null_on_the_wire_and_never_a_zero() {
        let at = |support| {
            Progress::started("zerodha", "NIFTY", (2024, 1), (2024, 1), support, 0, 1).to_json()
        };
        let derived = at(None);
        assert!(
            derived.contains(r#","support_ppm":null,"started_micros""#),
            "{derived}"
        );
        let fixed = at(Some(47_000));
        assert!(
            fixed.contains(r#","support_ppm":47000,"started_micros""#),
            "{fixed}"
        );
    }

    /// SW-14 (P12-02, D-1791): THE COMMIT GATE RUNS BEFORE THE SLOT IS
    /// CLAIMED. Two unstamped presses are both refused 503 for the build, the
    /// second is never answered 409 `Busy`, and the slot is still empty after
    /// both. Claiming first would make the second press a conflict with a run
    /// that was only ever going to fail.
    #[test]
    fn an_unstamped_press_is_refused_before_the_slot_and_never_reads_as_busy() {
        let site = finisher_site("sw14-unstamped");
        let run = format!(r#"{{"feed":"zerodha","underlying":"NIFTY",{SPAN}}}"#);
        for press in ["first", "second"] {
            let (status, _headers, body) = super::run_with(&site, &run, None);
            assert_eq!(
                status,
                axum::http::StatusCode::SERVICE_UNAVAILABLE,
                "{press}: {body}"
            );
            assert!(body.contains("\"accepted\":false"), "{press}: {body}");
        }
        assert!(
            site.sweep.lock().expect("private slot").is_none(),
            "an unstamped build claimed the slot"
        );
    }

    /// sobs-9, D-4447: AN UNSTAMPED BUILD IS LOGGED ON EVERY LAUNCH ROUTE, not
    /// only on `/backtest/run`. One `api.sweep` Warn per press, naming the
    /// route and carrying the refusal's sentence, read back from the binary's
    /// installed sink. (The environment-budget arm is proven in the child of
    /// `a_usable_server_budget_is_refused_before_the_slot_and_writes_nothing`,
    /// the one process here whose environment may carry a budget.)
    #[test]
    fn an_unstamped_build_is_logged_on_every_launch_route() {
        let _installed = crate::emitted::sink();
        let site = finisher_site("sobs9-unstamped-routes");
        let run = format!(r#"{{"feed":"zerodha","underlying":"NIFTY",{SPAN}}}"#);
        let descent = descent_body(r#""rung":"15min","max_points":20,"top":25"#);
        let audit = command_body(r#""command":"audit-range","rung":"15min","min_hits":500"#);
        let presses: [(&str, Route<'_>); 3] = [
            (super::RUN_ROUTE, &|| super::run_with(&site, &run, None)),
            (super::DESCEND_ROUTE, &|| {
                super::descend_with(&site, &descent, None)
            }),
            (super::COMMAND_ROUTE, &|| {
                super::command_with(&site, &audit, None)
            }),
        ];
        for (route, press) in presses {
            let from = crate::emitted::mark();
            let (status, _, body) = press();
            assert_eq!(
                status,
                axum::http::StatusCode::SERVICE_UNAVAILABLE,
                "{route}: {body}"
            );
            let lines: Vec<telemetry::Record> = crate::emitted::landed(
                from,
                "api.sweep",
                "a sweep was refused because this build carries no commit stamp",
            )
            .into_iter()
            .filter(|record| crate::emitted::says(record, "route", route))
            .collect();
            assert_eq!(lines.len(), 1, "{route}: one line per press: {lines:?}");
            let line = lines.first().expect("the one line");
            assert_eq!(line.level, telemetry::Level::Warn, "{route}");
            assert!(
                crate::emitted::says(line, "why", "BRUTEX_COMMIT"),
                "{route}: the refusal's own sentence: {line:?}"
            );
        }
        assert!(site.sweep.lock().expect("private slot").is_none());
    }

    /// audit-20261003 hunt-api-2, D-1582: CTRL-C ENDS THE PROCESS WHILE A
    /// SWEEP RUNS. A blocking task holding an engine-task count sleeps far past
    /// the grace; dropping a runtime would wait for all of it (tokio's
    /// documented `Drop`), while `end_runtime` returns within the grace and
    /// names at least that one task as abandoned.
    #[test]
    fn stopping_does_not_wait_out_a_running_engine_task() {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .expect("runtime");
        let (started, running) = std::sync::mpsc::channel::<()>();
        let _sweep = runtime.spawn_blocking(move || {
            let _counted = super::EngineTaskCount::take();
            let _told = started.send(());
            std::thread::sleep(std::time::Duration::from_secs(30));
        });
        running
            .recv_timeout(std::time::Duration::from_secs(5))
            .expect("the task started");
        assert!(super::engine_tasks_running() >= 1);
        let began = std::time::Instant::now();
        let abandoned =
            crate::server::wait_then_end(runtime, std::time::Duration::from_millis(300));
        let took = began.elapsed();
        assert!(abandoned >= 1, "the running sweep was not named");
        assert!(
            took < std::time::Duration::from_secs(5),
            "shutdown waited {took:?} for a sweep"
        );
        assert!(took >= std::time::Duration::from_millis(250), "{took:?}");
        assert!(crate::server::SHUTDOWN_GRACE <= std::time::Duration::from_secs(30));
    }

    /// audit-20261003 hunt-api-2, D-1551: SHUTDOWN DURING A SWEEP CANCELS IT,
    /// PROMPTLY AND BY NAME. In a child process, because the stop is
    /// process-wide and never withdrawn. A finisher-guarded task walks
    /// generated months through the real `cli::cancel` boundary and would run
    /// for a minute; `end_runtime` stops it within two seconds with nothing
    /// abandoned, the slot answers CANCELLED naming the month it stopped at,
    /// no report survives, and the invocation audit is `Cancelled`. A task that
    /// passed its last boundary before the stop is answered CANCELLED too,
    /// never as a complete result.
    #[test]
    fn shutdown_during_a_sweep_cancels_it_promptly_and_names_the_cancellation() {
        const CHILD: &str = "BRUTEX_TEST_SHUTDOWN_CANCELS_SWEEP";
        if std::env::var_os(CHILD).is_none() {
            assert!(!cli::cancel::requested(), "the stop leaked into the parent");
            let result = std::process::Command::new(std::env::current_exe().expect("test binary"))
                .args([
                    "--exact",
                    "sweeprun::tests::shutdown_during_a_sweep_cancels_it_promptly_and_names_the_cancellation",
                    "--nocapture",
                    "--test-threads=1",
                ])
                .env(CHILD, "child")
                .output()
                .expect("child test ran");
            let stdout = String::from_utf8_lossy(&result.stdout);
            assert!(
                result.status.success(),
                "{stdout}{}",
                String::from_utf8_lossy(&result.stderr)
            );
            assert!(stdout.contains("1 passed"), "{stdout}");
            return;
        }
        let (site, id, guard) = durable_finisher("shutdown-cancels-sweep");
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .expect("runtime");
        let (started, running) = std::sync::mpsc::channel::<()>();
        let _sweep = runtime.spawn_blocking(move || {
            let done = guard.conduct(|| {
                let mut progress =
                    Progress::started("zerodha", "NIFTY", (2020, 1), (2020, 1), None, 7, id);
                let _told = started.send(());
                let mut month = 0_u32;
                let text = loop {
                    month += 1;
                    if let Err(why) = cli::cancel::check(|| format!("generated month {month}")) {
                        break format!("refused: {why}");
                    }
                    if month > 3_000 {
                        break "a complete report the stop never reached".to_owned();
                    }
                    std::thread::sleep(std::time::Duration::from_millis(20));
                };
                settle(&mut progress, text, now_micros());
                progress
            });
            guard.finish(done);
        });
        running
            .recv_timeout(std::time::Duration::from_secs(5))
            .expect("the sweep started");
        let began = std::time::Instant::now();
        let abandoned = crate::server::end_runtime(runtime, crate::server::SHUTDOWN_GRACE);
        let took = began.elapsed();
        assert_eq!(abandoned, 0, "the sweep was abandoned, not cancelled");
        assert!(
            took < std::time::Duration::from_secs(2),
            "cancellation took {took:?}"
        );
        assert!(cli::cancel::observed() >= 1);
        let slot = site.sweep.lock().expect("slot").clone().expect("finished");
        assert!(slot.report.is_none(), "a cancelled sweep kept a report");
        let why = slot.refusal.expect("the cancellation is the answer");
        assert!(why.starts_with("refused: "), "{why}");
        assert!(why.contains(cli::cancel::CANCELLED), "{why}");
        assert!(why.contains("Stopped at: generated month"), "{why}");
        assert_eq!(completion_audit(&slot_of(&site)).outcome, "cancelled");
        let saved = cli::operation_audit::read(&site.store_root, id)
            .expect("read exact")
            .expect("saved");
        assert_eq!(saved.phase, cli::operation_audit::Phase::Cancelled);

        // Work whose last boundary passed before the stop: its finished report
        // is still not presented as complete.
        let (late, late_id, late_guard) = durable_finisher("shutdown-cancels-late");
        let done = late_guard.conduct(|| {
            let mut progress =
                Progress::started("zerodha", "NIFTY", (2020, 1), (2020, 1), None, 7, late_id);
            settle(&mut progress, "a finished table".to_owned(), now_micros());
            progress
        });
        assert!(done.report.is_none());
        let why = done.refusal.clone().expect("cancelled");
        assert!(why.contains(cli::cancel::CANCELLED), "{why}");
        assert!(why.contains("before its answer was recorded"), "{why}");
        late_guard.finish(done);
        let saved = cli::operation_audit::read(&late.store_root, late_id)
            .expect("read exact")
            .expect("saved");
        assert_eq!(saved.phase, cli::operation_audit::Phase::Cancelled);
        let _ = std::fs::remove_dir_all(&site.store_root);
        let _ = std::fs::remove_dir_all(&late.store_root);
    }

    fn slot_of(site: &crate::server::Loaded) -> Progress {
        site.sweep.lock().expect("slot").clone().expect("finished")
    }

    /// **Admission I/O runs with the slot UNLOCKED, and admissions still
    /// exclude each other.** W1-api6-2, D-0954.
    ///
    /// A first admission is parked inside its `prepare`, where the lease, the
    /// log walk, the audit `begin` and the marker run. While it is parked: a
    /// poll takes the slot at once (it used to wait out the whole admission on
    /// an async worker); a second admission does not reach its own `prepare`
    /// and is refused `Busy` at once rather than queued behind the first,
    /// because admissions are still one at a time and a queued one parked a
    /// shared `detail::run` permit (conc:runs-4, D-2776). Released, the first
    /// installs its in-flight run. A refusing `prepare` leaves
    /// a finished slot exactly as it was, and a busy slot never runs `prepare`.
    #[test]
    fn admission_io_runs_with_the_slot_unlocked_and_admissions_still_exclude_each_other() {
        use std::sync::atomic::{AtomicBool, Ordering};
        let site = finisher_site("admission-unlocked");
        let running =
            |attempt| Progress::started("zerodha", "NIFTY", (2020, 1), (2020, 1), None, 1, attempt);
        let busy = || super::refused(&Refusal::Busy("busy".to_owned()));
        let (parked, parked_rx) = std::sync::mpsc::channel();
        let (release, release_rx) = std::sync::mpsc::channel::<()>();
        let second_prepared = AtomicBool::new(false);
        let first_busy = AtomicBool::new(false);
        let (site_ref, first_busy_ref) = (&site, &first_busy);
        std::thread::scope(|scope| {
            let first = scope.spawn(move || {
                super::admit(
                    site_ref,
                    || {
                        first_busy_ref.store(true, Ordering::SeqCst);
                        busy()
                    },
                    || {
                        parked.send(()).expect("the test is listening");
                        release_rx.recv().expect("the test releases");
                        Ok((running(1), "first"))
                    },
                )
            });
            parked_rx
                .recv()
                .expect("the first admission is inside prepare");
            let polled = site.sweep.try_lock().map(|slot| slot.clone());
            assert!(
                polled.is_ok_and(|slot| slot.is_none()),
                "a poll during admission I/O must take the slot at once and see no run yet"
            );
            // REFUSED, NOT QUEUED (conc:runs-4, D-2776). A second admission
            // that waited parked a shared `detail::run` permit for the
            // first's whole I/O. The bound only turns a regression (a wait)
            // into a failure rather than a hang; the fixed path never meets it.
            let (answered, answer) = std::sync::mpsc::channel();
            let second_prepared = &second_prepared;
            let second = scope.spawn(move || {
                let refused = super::admit(site_ref, busy, || {
                    second_prepared.store(true, Ordering::SeqCst);
                    Ok((running(2), "second"))
                });
                answered.send(()).expect("the test is listening");
                refused
            });
            let refused_at_once = answer
                .recv_timeout(std::time::Duration::from_mins(1))
                .is_ok();
            release.send(()).expect("the first admission is waiting");
            assert_eq!(first.join().expect("first").ok(), Some("first"));
            let (status, _, body) = second
                .join()
                .expect("second")
                .expect_err("another admission is in progress");
            assert!(
                refused_at_once,
                "a second admission is refused while the first is admitting, not queued"
            );
            assert_eq!(status, axum::http::StatusCode::CONFLICT, "{body}");
        });
        assert!(!first_busy.load(Ordering::SeqCst));
        assert!(
            !second_prepared.load(Ordering::SeqCst),
            "Busy never prepares"
        );
        let installed = site.sweep.lock().expect("slot").clone().expect("installed");
        assert!(installed.in_flight() && installed.attempt == 1);

        let mut finished = running(3);
        finished.finished_micros = Some(9);
        finished.report = Some("kept".to_owned().into());
        *site.sweep.lock().expect("slot") = Some(finished);
        let refusal = super::admit(&site, busy, || {
            Err::<(Progress, ()), _>(super::refused(&Refusal::Unobservable(
                "no lease".to_owned(),
            )))
        })
        .expect_err("prepare refused");
        assert_eq!(refusal.0, axum::http::StatusCode::SERVICE_UNAVAILABLE);
        let kept = site.sweep.lock().expect("slot").clone().expect("kept");
        assert_eq!(
            (kept.attempt, kept.finished_micros, kept.report.as_deref()),
            (3, Some(9), Some("kept"))
        );
    }

    /// **No POST handler holds the slot across admission.** W1-api6-2, D-0954.
    ///
    /// `run_with`, `descend_with` and `command_with_configuration` each took
    /// `site.sweep.lock()` and kept it through every admission step. Each body
    /// must now name `admit(` and must not lock the slot itself, so a fourth
    /// copy of the old block fails here, not in a poll that hangs.
    #[test]
    fn no_post_handler_locks_the_slot_across_its_admission() {
        let production = include_str!("sweeprun.rs")
            .split_once("\nmod tests {")
            .expect("this file declares its tests module")
            .0;
        for name in [
            "pub(crate) fn run_with(",
            "pub(crate) fn descend_with(",
            "fn command_with_configuration(",
        ] {
            let body = production
                .split_once(name)
                .expect("the handler exists")
                .1
                .split_once("\n}\n")
                .expect("the handler ends")
                .0;
            assert!(
                body.contains("admit(site,"),
                "{name} admits through `admit`"
            );
            assert!(
                !body.contains("sweep.lock()") && !body.contains(".sweep\n"),
                "{name} locks the slot itself"
            );
        }
    }

    /// **A poll's snapshot of the slot shares the report; it does not copy
    /// it.** W1-api6-1, D-0954.
    ///
    /// `run_json` clones the whole `Progress` while it holds the slot's std
    /// mutex on an async worker. With the report a `String` that clone copied
    /// every byte of it on every poll, and a `sweep-all` report grows by a line
    /// per instrument-month. An 8 MiB report and an 8 MiB refusal, each with a
    /// quote, a backslash, a newline and a multibyte character at its ends,
    /// are cloned the way the poll clones them: both clones point at the SAME
    /// bytes as the slot, and the serialised status is byte-identical to the
    /// slot's own, so sharing changed nothing a reader sees.
    #[test]
    fn a_poll_snapshot_shares_the_report_and_refusal_bytes_with_the_slot() {
        let edge = |fill: char| {
            let mut text = String::from("\"\\\n\u{20b9}");
            text.extend(std::iter::repeat_n(fill, 8 << 20));
            text.push_str("\u{20b9}\n\\\"");
            text
        };
        let mut done = Progress::started("zerodha", "NIFTY", (2020, 1), (2020, 12), None, 1, 7);
        done.finished_micros = Some(2);
        done.report = Some(edge('r').into());
        done.refusal = Some(edge('f').into());
        let slot = std::sync::Mutex::new(Some(done));
        let snapshot = slot.lock().expect("private slot").clone();
        let held = slot.lock().expect("private slot");
        let (held, snapshot) = (
            held.as_ref().expect("held"),
            snapshot.as_ref().expect("snapshot"),
        );
        for (name, slot_text, poll_text) in [
            ("report", held.report.as_deref(), snapshot.report.as_deref()),
            (
                "refusal",
                held.refusal.as_deref(),
                snapshot.refusal.as_deref(),
            ),
        ] {
            let (slot_text, poll_text) = (slot_text.expect(name), poll_text.expect(name));
            assert_eq!(poll_text.len(), slot_text.len(), "{name}");
            assert!(
                std::ptr::eq(slot_text.as_ptr(), poll_text.as_ptr()),
                "{name}: the poll's snapshot copied {} bytes under the slot lock",
                poll_text.len()
            );
        }
        assert_eq!(snapshot.to_json(), held.to_json());
        let json: serde_json::Value =
            serde_json::from_str(&held.to_json()).expect("a valid status");
        assert_eq!(
            json.get("report").and_then(serde_json::Value::as_str),
            held.report.as_deref()
        );
    }

    fn elsewhere_over(dir: &std::path::Path, now_millis: i64) -> String {
        super::observe_elsewhere(dir, now_millis).body
    }

    fn finisher_site(name: &str) -> crate::server::Loaded {
        let masters = crate::scratch::path(&format!("sweep-finisher-masters-{name}"));
        let store = crate::scratch::path(&format!("sweep-finisher-store-{name}"));
        std::sync::Arc::new(crate::server::Site::load(&masters, &store))
    }

    fn durable_finisher(name: &str) -> (crate::server::Loaded, u64, TaskFinisher) {
        let site = finisher_site(name);
        std::fs::create_dir_all(&site.store_root).expect("private invocation store");
        let audit = super::reserve_invocation(&site, "sweep").expect("durable admission");
        let id = audit.id();
        *site.sweep.lock().expect("private slot") = Some(Progress::started(
            "zerodha",
            "NIFTY",
            (2020, 1),
            (2020, 1),
            None,
            7,
            id,
        ));
        let guard = TaskFinisher::audited(std::sync::Arc::clone(&site), audit);
        (site, id, guard)
    }

    #[test]
    fn durable_task_finisher_publishes_only_after_its_terminal_barrier() {
        let (site, id, guard) = durable_finisher("durable-task-complete");
        guard.enter(cli::operation_audit::completed_boundary);
        let mut done = site
            .sweep
            .lock()
            .expect("private slot")
            .clone()
            .expect("started");
        settle(
            &mut done,
            "private lifecycle fixture; no market computation".to_owned(),
            10,
        );
        guard.finish(done);
        let saved = cli::operation_audit::read(&site.store_root, id)
            .expect("read exact")
            .expect("saved");
        assert_eq!(saved.phase, cli::operation_audit::Phase::Completed);
        assert_eq!(saved.completed_boundaries, 1);
        let slot = site.sweep.lock().expect("private slot");
        assert!(slot.as_ref().expect("completed").report.is_some());
        drop(slot);
        std::fs::remove_dir_all(&site.store_root).expect("private cleanup");
    }

    /// **A finished task gives its execution lease back before its slot says
    /// finished.** conc:runs-2, D-2778.
    ///
    /// The slot was written with `in_flight == false` while the finisher still
    /// owned the lease, so a press admitted in that gap passed the slot check
    /// and was refused `Busy` by a run that had ended. The test holds the slot
    /// lock, so `finish` stops exactly where it publishes; the lease must
    /// already be free there. The bound only turns a regression (a lease held
    /// until the slot is written) into a failure rather than a hang.
    #[test]
    fn a_finished_task_frees_the_execution_lease_before_its_slot_says_finished() {
        let (site, _id, guard) = durable_finisher("durable-task-lease-first");
        let lease = cli::execution_lease::Lease::acquire(&site.store_root).expect("a free store");
        let guard = guard.with_lease(lease);
        guard.enter(cli::operation_audit::completed_boundary);
        let mut done = site
            .sweep
            .lock()
            .expect("private slot")
            .clone()
            .expect("started");
        settle(&mut done, "private lease-order fixture".to_owned(), 10);
        let slot = site.sweep.lock().expect("private slot");
        std::thread::scope(|scope| {
            let finishing = scope.spawn(move || guard.finish(done));
            let deadline = std::time::Instant::now() + std::time::Duration::from_mins(1);
            let freed = loop {
                match cli::execution_lease::Lease::acquire(&site.store_root) {
                    Ok(next) => break Some(next),
                    Err(cli::execution_lease::Refusal::Busy)
                        if std::time::Instant::now() < deadline =>
                    {
                        std::thread::yield_now();
                    }
                    Err(_) => break None,
                }
            };
            assert!(
                slot.as_ref().is_some_and(Progress::in_flight),
                "the premise: the slot has not been published yet"
            );
            drop(slot);
            finishing.join().expect("finish");
            assert!(
                freed.is_some(),
                "the lease is free while the finished run is still being published"
            );
        });
        assert!(
            !site
                .sweep
                .lock()
                .expect("private slot")
                .as_ref()
                .expect("published")
                .in_flight()
        );
        std::fs::remove_dir_all(&site.store_root).expect("private cleanup");
    }

    #[test]
    fn durable_task_finisher_does_not_publish_success_after_a_torn_audit() {
        use std::io::Write as _;
        let (site, id, guard) = durable_finisher("durable-task-torn");
        let path = site
            .store_root
            .join(format!("audit/invocations-v1/{id:020}.bin"));
        std::fs::OpenOptions::new()
            .append(true)
            .open(path)
            .expect("private journal")
            .write_all(&[1])
            .expect("private torn-write injection");
        let mut done = site
            .sweep
            .lock()
            .expect("private slot")
            .clone()
            .expect("started");
        settle(
            &mut done,
            "private completed handler fixture".to_owned(),
            10,
        );
        guard.finish(done);
        let slot = site.sweep.lock().expect("private slot");
        let progress = slot.as_ref().expect("refused terminal");
        assert!(progress.report.is_none());
        assert!(
            progress
                .refusal
                .as_deref()
                .expect("refusal")
                .contains("terminal invocation audit is unconfirmed")
        );
        assert!(!progress.in_flight());
        assert!(cli::operation_audit::read(&site.store_root, id).is_err());
        drop(slot);
        std::fs::remove_dir_all(&site.store_root).expect("private cleanup");
    }

    #[test]
    fn durable_task_finisher_records_panic_and_cancel_without_claiming_a_report() {
        for (name, panic, expected) in [
            (
                "durable-task-cancel",
                false,
                cli::operation_audit::Phase::Cancelled,
            ),
            (
                "durable-task-panic",
                true,
                cli::operation_audit::Phase::Failed,
            ),
        ] {
            let (site, id, guard) = durable_finisher(name);
            if panic {
                let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
                    let _guard = guard;
                    std::panic::resume_unwind(Box::new("private lifecycle panic"));
                }));
                assert!(caught.is_err());
            } else {
                drop(guard);
            }
            assert_eq!(
                cli::operation_audit::read(&site.store_root, id)
                    .expect("read")
                    .expect("saved")
                    .phase,
                expected
            );
            let slot = site.sweep.lock().expect("private slot");
            assert!(slot.as_ref().expect("terminal").report.is_none());
            assert!(slot.as_ref().expect("terminal").refusal.is_some());
            drop(slot);
            std::fs::remove_dir_all(&site.store_root).expect("private cleanup");
        }
    }

    fn body(feed: &str, span: &str) -> String {
        format!(r#"{{"feed":"{feed}","underlying":"NIFTY",{span}}}"#)
    }

    /// [`body`] with extra JSON spliced in before the closing brace.
    ///
    /// The `extra` carries its own leading comma, so a caller can add one
    /// field or none without this helper guessing which.
    fn body_with(feed: &str, span: &str, extra: &str) -> String {
        format!(r#"{{"feed":"{feed}","underlying":"NIFTY",{span}{extra}}}"#)
    }

    const SPAN: &str = r#""from_year":2019,"from_month":12,"to_year":2026,"to_month":8"#;

    /* ==================== the body parser ==================== */

    #[test]
    fn a_whole_body_parses_into_the_five_things_a_sweep_needs() {
        let asked = asked_from(&body("zerodha", SPAN)).expect("a good body");
        assert_eq!(
            asked,
            Asked {
                feed: "zerodha".to_owned(),
                underlying: "NIFTY".to_owned(),
                from: (2019, 12),
                to: (2026, 8),
                // A BODY WITH NO `rungs` SWEEPS EVERY RUNG, which is what this
                // route did before it could be asked anything else. Pinned in
                // the parser's own equality test so the compatibility is a
                // property under test rather than a claim in a comment.
                rungs: EVERY_RUNG.to_vec(),
                knobs: Vec::new(),
            }
        );
    }

    #[test]
    fn a_named_rung_subset_is_resolved_against_the_engines_table() {
        let asked = asked_from(&body_with(
            "zerodha",
            SPAN,
            r#","rungs":["15min","1min","15min"]"#,
        ))
        .expect("a good body");
        // ORDER IS THE CALLER'S AND DUPLICATES COLLAPSE. `range_over` maps over
        // an indexed parallel iterator, so the rows come out in this order --
        // and a rung asked for twice must be swept once, not twice into the
        // same ledger.
        assert_eq!(asked.rungs, vec!["15min", "1min"]);
    }

    #[test]
    fn a_rung_the_engine_does_not_sweep_is_refused_by_name() {
        // `1day` IS THE ONE THAT MATTERS. It is a real store timeframe, so a
        // check written against the store would accept it; the engine does not
        // sweep it, and a `1day` record in the results ledger is how this was
        // noticed at all.
        let why = asked_from(&body_with("zerodha", SPAN, r#","rungs":["1day"]"#))
            .expect_err("1day is not swept");
        assert!(why.why().contains("1day"), "{}", why.why());
        assert!(
            why.why().contains("30min"),
            "the eight are named: {}",
            why.why()
        );

        let bad = asked_from(&body_with("zerodha", SPAN, r#","rungs":["7min"]"#))
            .expect_err("7min is not a rung");
        assert!(bad.why().contains("7min"), "{}", bad.why());
    }

    #[test]
    fn an_empty_rung_list_is_refused_rather_than_widened_to_all_eight() {
        // PRESENT-AND-EMPTY IS NOT ABSENT. Widening `[]` back to every rung
        // would answer a caller who asked for nothing by giving them
        // everything, which is the silent-default shape §6 objects to.
        let why = asked_from(&body_with("zerodha", SPAN, r#","rungs":[]"#))
            .expect_err("an empty list is not a sweep");
        assert!(why.why().contains("rungs"), "{}", why.why());
        // And absent still means all eight, which is the other half of the rule.
        assert_eq!(
            asked_from(&body("zerodha", SPAN))
                .expect("absent is fine")
                .rungs,
            EVERY_RUNG.to_vec()
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
        assert_eq!(asked.to, (2020, 3));
        // The body above still carries a spaced `"support_ppm" : 50000`, and it
        // is read past like any other field this route does not want. A sweep
        // takes no support from a request AND no longer holds one of its own:
        // it is derived per rung from that rung's bars. D-0303.
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
        let why = asked_from(&body("", SPAN)).expect_err("a refusal");
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
        for missing in ["from_year", "from_month", "to_year", "to_month"] {
            let raw = body("zerodha", SPAN).replace(missing, "x_gone");
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
        let span = r#""from_year":"lots","from_month":12,"to_year":2026,"to_month":8"#;
        let why = asked_from(&body("zerodha", span)).expect_err("a refusal");
        assert!(matches!(why, Refusal::Malformed(_)));
        assert!(why.why().contains("from_year"), "{}", why.why());
    }

    #[test]
    fn a_support_field_in_the_body_is_now_obeyed_and_named() {
        // THIS TEST USED TO PIN THE OPPOSITE, and the argument it made was
        // right: *"a hidden settable parameter is worse than a visible one:
        // nothing on the page would show it was in play."*
        //
        // What changed is not the argument but the answer to it. The parameter
        // is no longer hidden. It is one of `KNOBS`, the page renders a control
        // for it, `conduct` logs every knob it set under `api.sweep` before the
        // run starts, and the run's own identity covers it. The condition the
        // old test demanded — that nothing steer the engine invisibly — is met
        // by making it VISIBLE rather than by making it inert.
        //
        // The alternative it was defending against is what the operator's
        // machine actually did on 2026-08-29: a server started with no
        // `BRUTEX_` variables, every knob at a default nobody chose, and two of
        // those defaults composing into a run that could not finish. That is
        // the same invisibility, one level down.
        let with =
            format!(r#"{{"feed":"zerodha","underlying":"NIFTY",{SPAN},"support_ppm":999999}}"#);
        let asked = asked_from(&with).expect("a body naming a knob is a good body");
        assert_eq!(
            asked.knobs,
            vec![("BRUTEX_SUPPORT_PPM", "999999".to_owned())],
            "the field must reach the engine under its own name"
        );

        // AND A BODY THAT NAMES NONE SETS NONE, so every existing client keeps
        // its exact behaviour and the environment still decides for them.
        let without = asked_from(&body("zerodha", SPAN)).expect("a good body");
        assert!(
            without.knobs.is_empty(),
            "a body with no knobs must set no knobs"
        );
        assert_eq!(
            Asked {
                knobs: Vec::new(),
                ..asked
            },
            without,
            "the extra field must change nothing about what was asked"
        );
    }

    /// THE TABLE IS THE ALLOWLIST, and this is what makes that a property
    /// rather than a claim.
    ///
    /// `BRUTEX_STORE` names where this engine reads bars from and
    /// `BRUTEX_LOG_DIR` where it writes its audit trail. Neither is a parameter
    /// of a run, and an HTTP body must not be able to move either — a request
    /// that could repoint the store would make every provenance banner on the
    /// page a claim about a directory the operator did not choose.
    ///
    /// They are absent from `KNOBS`, and `cli` reads both through
    /// `std::env::var_os` rather than through the knob store, so there are two
    /// independent reasons this cannot happen. This test pins the first.
    #[test]
    fn a_body_cannot_move_the_store_or_the_log_directory() {
        let hostile = format!(
            r#"{{"feed":"zerodha","underlying":"NIFTY",{SPAN},"store":"/tmp/evil",
               "BRUTEX_STORE":"/tmp/evil","log_dir":"/tmp/evil","BRUTEX_LOG_DIR":"/tmp/evil"}}"#
        );
        let asked = asked_from(&hostile).expect("unknown fields are ignored, not refused");
        assert!(
            asked.knobs.is_empty(),
            "no field outside KNOBS may set anything: {:?}",
            asked.knobs
        );
        for (_, name) in KNOBS {
            assert!(
                name != "BRUTEX_STORE" && name != "BRUTEX_LOG_DIR",
                "{name} names the process's own files and must never be settable"
            );
        }
    }

    /// `cli::validates` is `raw.is_none_or(|v| v.trim() != "0")`, so ANY string
    /// that is not exactly `"0"` means validation is ON — and JSON's own
    /// `false` is the string `"false"`.
    ///
    /// A page sending `"validate": false` would therefore have turned
    /// validation ON, which is the exact opposite of what it asked, and the
    /// symptom would have been a run that never finished rather than an error.
    /// That is the failure §4 bans, so the translation is tested rather than
    /// commented.
    #[test]
    fn a_json_false_for_validate_becomes_the_off_the_engine_recognises() {
        for written in ["false", "\"false\"", "0", "\"off\"", "\"NO\""] {
            let raw =
                format!(r#"{{"feed":"zerodha","underlying":"NIFTY",{SPAN},"validate":{written}}}"#);
            let asked = asked_from(&raw).expect("a good body");
            assert_eq!(
                asked.knobs,
                vec![("BRUTEX_VALIDATE", "0".to_owned())],
                "`{written}` must reach the engine as the only string it reads as off"
            );
        }
    }

    /// The screen budget is refused by name rather than dropped: every run this
    /// route starts records, and a budget's cap is decided by timing the run
    /// identity cannot name. The stated screen cap still reaches the engine.
    /// D-0685.
    #[test]
    fn a_screen_budget_is_refused_by_name_and_the_screen_cap_still_applies() {
        assert!(
            KNOBS
                .iter()
                .all(|(asked, name)| *asked != "screen_budget_ms"
                    && *name != "BRUTEX_SCREEN_BUDGET_MS"),
            "no request may set the budget"
        );
        for written in ["5000", "\"5000\"", "\"\"", "\"not-a-budget\""] {
            let raw = format!(
                r#"{{"feed":"zerodha","underlying":"NIFTY",{SPAN},"screen_budget_ms":{written}}}"#
            );
            let why = asked_from(&raw).expect_err("a budget cannot reach a recorded run");
            assert_eq!(why.status(), axum::http::StatusCode::BAD_REQUEST);
            assert!(
                why.why().starts_with("`screen_budget_ms` is refused"),
                "{written}: {}",
                why.why()
            );
            assert!(
                why.why().contains("No setting was ignored"),
                "{}",
                why.why()
            );
        }
        let capped =
            format!(r#"{{"feed":"zerodha","underlying":"NIFTY",{SPAN},"screen_cap":"7"}}"#);
        assert_eq!(
            asked_from(&capped)
                .expect("a stated cap is a good body")
                .knobs,
            vec![("BRUTEX_SCREEN_CAP", "7".to_owned())]
        );
    }

    /// The one refusal a body naming `screen_budget_ms` must get: 400, and the
    /// budget's own sentence rather than a generic decoder complaint.
    fn assert_budget_named<T: std::fmt::Debug>(result: Result<T, Refusal>, context: &str) {
        let why = result.expect_err(context);
        assert_eq!(
            why.status(),
            axum::http::StatusCode::BAD_REQUEST,
            "{context}"
        );
        assert!(
            why.why().starts_with("`screen_budget_ms` is refused"),
            "{context}: {}",
            why.why()
        );
        assert!(
            why.why().contains(super::BUDGET_NOT_RECORDABLE),
            "{context}: the reason is the file's one wording: {}",
            why.why()
        );
    }

    /// THE BUDGET'S REASON IS WORDED ONCE in this file's production code, and
    /// both of its refusals state it. D-0695.
    ///
    /// The body refusal and the environment refusal each wrote the reason out,
    /// as two different sentences for one fact. `BUDGET_NOT_RECORDABLE` now
    /// holds it. `assert_budget_named` checks that the body refusal states it,
    /// and `server_budget_through_the_journal` that the environment refusal
    /// does. This counts the reason's own words in this file's production
    /// source, everything before `mod tests`, which includes `top_json` below
    /// the first `#[cfg(test)]`. Before counting, the `//!`, `///` or `//`
    /// that opens a line is dropped, a `\` that ends a line is dropped with
    /// the newline and the next line's leading whitespace, as the compiler
    /// drops them inside a string literal, and every other run of whitespace
    /// is made one space. So a second copy of those words fails here wrapped
    /// across lines with a `\`, without one, or across the lines of a `//!`,
    /// `///` or `//` comment. A paraphrase, a copy assembled from pieces, or a
    /// copy wrapped behind any other mark that opens a line, such as the `*`
    /// of a block comment, is not counted.
    #[test]
    fn the_budget_reason_is_worded_once() {
        let production = include_str!("sweeprun.rs")
            .split_once("\nmod tests {")
            .expect("this file declares its tests module")
            .0;
        assert!(
            production.contains("pub async fn top_json("),
            "the production source reaches past the first `#[cfg(test)]`"
        );
        assert_eq!(
            as_read(production)
                .matches("wall-clock calibration the run identity cannot name")
                .count(),
            1,
            "the budget's reason is spelt out more than once"
        );
    }

    /// `source` with the `//!`, `///` or `//` that opens a line removed, each
    /// `\`-newline continuation and the whitespace after it removed, and
    /// every other run of whitespace made one space.
    fn as_read(source: &str) -> String {
        let uncommented = source
            .lines()
            .map(|line| {
                let line = line.trim_start();
                ["//!", "///", "//"]
                    .into_iter()
                    .find_map(|marker| line.strip_prefix(marker))
                    .unwrap_or(line)
            })
            .collect::<Vec<_>>()
            .join("\n");
        let mut pieces = uncommented.split("\\\n");
        let mut joined = pieces.next().unwrap_or_default().to_owned();
        for piece in pieces {
            joined.push_str(piece.trim_start());
        }
        joined.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    /// `as_read` joins what the compiler joins inside a string literal, and
    /// the lines of a line comment, and nothing else.
    #[test]
    fn a_wrapped_copy_reads_as_one_line() {
        assert_eq!(
            as_read("\"wall-clock \\\n        calibration the\n    run  identity\""),
            "\"wall-clock calibration the run identity\""
        );
        assert_eq!(as_read("first\\\n  second\\\nthird"), "firstsecondthird");
        assert_eq!(as_read("no continuation"), "no continuation");
        // A LINE COMMENT'S LINES READ AS ONE, the marker that opens each line
        // dropped, so a copy in a comment wrapped across lines is counted as a
        // copy on one line is. A marker inside a line is kept. D-0695.
        assert_eq!(
            as_read("    /// wall-clock calibration the\n    //! run identity\n// cannot name"),
            "wall-clock calibration the run identity cannot name"
        );
        assert_eq!(
            as_read("call(); // wall-clock\n    //    calibration"),
            "call(); // wall-clock calibration"
        );
        // Any other mark that opens a line is kept, and parts the words.
        assert_eq!(
            as_read("/* wall-clock\n * calibration */"),
            "/* wall-clock * calibration */"
        );
    }

    /// Every ordinary `POST /engine/command` word, as a good body names it.
    const ORDINARY_WORDS: [&str; 6] = [
        r#""command":"audit-range","rung":"15min","min_hits":500"#,
        r#""command":"screen","rung":"15min","support_ppm":50000,"max_points":20,"top":25"#,
        r#""command":"auto-stored","rung":"1min""#,
        r#""command":"sweep-stored","rung":"15min","min_hits":500"#,
        r#""command":"sweep-all","rung":"15min","min_hits":500"#,
        r#""command":"audit-audited-range","rung":"5min","min_hits":500"#,
    ];

    /// NAMING THE FIELD IS THE ERROR, WHATEVER JSON VALUE CARRIES IT, ON EVERY
    /// BODY ROUTE.
    ///
    /// `refuse_screen_budget` says any value refuses, an empty one included.
    /// That held only for the four JSON types `WireScalar` decodes: `null`, a
    /// fraction, an array, an object or an integer past `u64::MAX` failed the
    /// scalar decode first and were refused with the decoder's generic sentence,
    /// which never says the budget is not settable. Every value below is refused
    /// with the budget's own sentence. A repeated key and a truncated body are
    /// still refused as malformed JSON, because the object never parsed.
    ///
    /// AF-21 claimed this value by value for `/backtest/descend` and every
    /// ordinary command word too, while only the run body was sent each value;
    /// the other two held only because the three parsers share `wire_body`.
    /// Every value now goes to all three parsers, with every ordinary word, AND
    /// to the three route handlers, whose status and body are what the browser
    /// receives. D-0695.
    #[test]
    fn a_screen_budget_is_refused_by_name_whatever_json_value_carries_it() {
        let site = finisher_site("budget-any-value");
        for written in [
            "-1",
            "0",
            "true",
            "false",
            "18446744073709551615",
            "\"  \"",
            "\" 5000 \"",
            "null",
            "1.5",
            "[5000]",
            "{}",
            "18446744073709551616",
        ] {
            let budget = format!(r#""screen_budget_ms":{written}"#);
            let run = format!(r#"{{"feed":"zerodha","underlying":"NIFTY",{SPAN},{budget}}}"#);
            assert_budget_named(asked_from(&run), written);
            let descent = descent_body(&format!(
                r#""rung":"15min","max_points":20,"top":25,{budget}"#
            ));
            assert_budget_named(descent_from(&descent), &format!("descend {written}"));
            // NO STAMP, deliberately: a body's refusal comes before the build's,
            // so a handler that answered anything but the budget's 400 here
            // would be reading past the body it was sent.
            let mut answers = vec![
                ("run", super::run_with(&site, &run, None)),
                ("descend", super::descend_with(&site, &descent, None)),
            ];
            for words in ORDINARY_WORDS {
                let command = command_body(&format!("{words},{budget}"));
                assert_budget_named(command_from(&command), &format!("{words} {written}"));
                answers.push((words, super::command_with(&site, &command, None)));
            }
            for (route, (status, _, body)) in answers {
                assert_eq!(
                    status,
                    axum::http::StatusCode::BAD_REQUEST,
                    "{route} {written}: {body}"
                );
                assert!(
                    body.contains("`screen_budget_ms` is refused"),
                    "{route} {written}: {body}"
                );
            }
        }
        for malformed in [
            format!(
                r#"{{"feed":"zerodha","underlying":"NIFTY",{SPAN},"screen_budget_ms":1,"screen_budget_ms":2}}"#
            ),
            format!(r#"{{"feed":"zerodha","underlying":"NIFTY",{SPAN},"screen_budget_ms":5000"#),
        ] {
            let why = asked_from(&malformed).expect_err("not one complete JSON object");
            assert_eq!(why.status(), axum::http::StatusCode::BAD_REQUEST);
            assert!(
                why.why()
                    .starts_with("the request body must be one complete JSON object"),
                "{malformed}: {}",
                why.why()
            );
        }
    }

    /// EVERY BODY ROUTE REFUSES THE BUDGET, NOT ONLY THE ONE THAT APPLIES KNOBS.
    ///
    /// `docs/06-limits.md` (D-0685) says the HTTP routes refuse a body that
    /// names `screen_budget_ms`. `POST /backtest/descend` and every ordinary
    /// `POST /engine/command` word decoded the field and then dropped it, so a
    /// budget the operator typed silently did nothing on the descent and on
    /// `audit-range` and `screen` -- the two words that run the exit-grid screen
    /// a budget would bound. That is the fallback `refuse_screen_budget` exists
    /// to refuse.
    #[test]
    fn a_screen_budget_is_refused_by_name_on_every_body_route() {
        let descent = descent_body(r#""rung":"15min","max_points":20,"top":25"#);
        assert!(descent_from(&descent).is_ok(), "the control body is good");
        assert_budget_named(
            descent_from(&descent_body(
                r#""rung":"15min","max_points":20,"top":25,"screen_budget_ms":5000"#,
            )),
            "descend",
        );
        for words in ORDINARY_WORDS {
            assert!(
                command_from(&command_body(words)).is_ok(),
                "control {words}"
            );
            assert_budget_named(
                command_from(&command_body(&format!(
                    r#"{words},"screen_budget_ms":5000"#
                ))),
                words,
            );
        }
    }

    /// WHICH REFUSAL A BODY WITH TWO FAULTS GETS, pinned on every body route.
    ///
    /// A body that is not one complete JSON object is refused as that first:
    /// no field of it can be read. Past that, the budget is refused before any
    /// other field is checked, so a budget beside a missing feed, a missing
    /// command word or a generated-bar word names the budget. One order on all
    /// three routes, so the same two faults never get two different answers.
    ///
    /// "Past that" includes every known field's JSON TYPE, which the decode
    /// checks with the syntax; the test below pins that half. D-0695.
    #[test]
    fn the_budget_is_refused_before_every_other_field_and_after_the_json() {
        let budget = r#""screen_budget_ms":5000"#;
        assert_budget_named(
            asked_from(&format!(r#"{{"underlying":"NIFTY",{SPAN},{budget}}}"#)),
            "run without a feed",
        );
        assert_budget_named(
            descent_from(&format!(r#"{{"underlying":"NIFTY",{SPAN},{budget}}}"#)),
            "descent without a feed or rung",
        );
        for words in [
            r#""rung":"15min""#,
            r#""command":"sweep""#,
            r#""command":"conquer""#,
        ] {
            assert_budget_named(
                command_from(&command_body(&format!("{words},{budget}"))),
                words,
            );
        }
        let truncated = format!(r#"{{"feed":"zerodha","underlying":"NIFTY",{budget}"#);
        for why in [
            asked_from(&truncated).expect_err("truncated"),
            descent_from(&truncated).expect_err("truncated"),
            command_from(&truncated).expect_err("truncated"),
        ] {
            assert!(
                why.why()
                    .starts_with("the request body must be one complete JSON object"),
                "{}",
                why.why()
            );
        }
    }

    /// A KNOWN FIELD OF THE WRONG JSON TYPE BESIDE THE BUDGET IS REFUSED FIRST,
    /// AS A MALFORMED BODY, on every body route.
    ///
    /// AF-21 said the budget is named "before any other field" and that only a
    /// body that is not one complete JSON object is refused before it. That is
    /// not the order the code keeps: `wire_body` decodes every known field's
    /// JSON type before `refuse_screen_budget` reads the result, so a `feed`
    /// that is a number, a `from_year` that is an array or `rungs` that is a
    /// string fails the decode and gets the decoder's sentence. The budget is
    /// named first only among bodies whose known fields are well typed, and it
    /// can never be the field that fails the decode, because it decodes as any
    /// value at all. D-0695 states this order; this pins it.
    #[test]
    fn a_wrong_typed_known_field_beside_the_budget_is_refused_as_malformed_first() {
        let budget = r#""screen_budget_ms":5000"#;
        for fault in [
            r#""feed":5,"underlying":"NIFTY""#,
            r#""feed":"zerodha","underlying":"NIFTY","from_year":[1]"#,
            r#""feed":"zerodha","underlying":"NIFTY","rungs":"5min""#,
        ] {
            let body = format!("{{{fault},{budget}}}");
            let commanded = format!(r#"{{{fault},"command":"screen",{budget}}}"#);
            for (route, why) in [
                ("run", asked_from(&body).expect_err("a wrong-typed field")),
                (
                    "descend",
                    descent_from(&body).expect_err("a wrong-typed field"),
                ),
                (
                    "command",
                    command_from(&commanded).expect_err("a wrong-typed field"),
                ),
            ] {
                assert_eq!(
                    why.status(),
                    axum::http::StatusCode::BAD_REQUEST,
                    "{route} {fault}"
                );
                assert!(
                    why.why()
                        .starts_with("the request body must be one complete JSON object"),
                    "{route} {fault}: {}",
                    why.why()
                );
            }
        }
    }

    /// The child half of the test below reads its fixture root from this.
    const SERVER_BUDGET_CHILD: &str = "BRUTEX_API_SERVER_BUDGET_CHILD";

    /// A USABLE BUDGET IN THE SERVER'S OWN ENVIRONMENT IS REFUSED BEFORE THE
    /// SLOT, AND THE STORE IS NOT TOUCHED.
    ///
    /// With `BRUTEX_SCREEN_BUDGET_MS=5000` in the server's environment and none
    /// in the body, `POST /backtest/run` answered 202, took the execution lease
    /// (`.sweep-execution-v1.lock`) and wrote an invocation record under
    /// `audit/invocations-v1/`, and only then did `cli::one_rung` refuse the
    /// budget. That refusal can never change while the server runs, so it must
    /// not occupy the slot -- the rule `run_with` already keeps for an
    /// unstamped build -- and D-0685's "before it reads its source or writes
    /// anything" held only at the `cli` layer. The descent and the two command
    /// words whose runs price the screen took the same path.
    ///
    /// In a child process because the budget must be in the ENVIRONMENT, which
    /// a test cannot set in its own process; see `crate::isolated`.
    ///
    /// # One child per spelling, and why the padded two are here
    ///
    /// The rule trims before it parses, and nothing pinned that. The value
    /// table in [`the_budget_rule_is_clis`] hands every value to
    /// `cli::knobs::set`, which trims before it stores, so its `"\u{a0}5000"`
    /// reached the rule already trimmed, and the only environment value any
    /// child saw was an unpadded `5000`. Only the environment carries a value
    /// untrimmed: `knobs::var` falls through to `std::env::var_os` and trims
    /// nothing. A review removed the route's trim and this test stayed green,
    /// while a padded server budget walked past the route's guard, took the
    /// execution lease and wrote a run invocation record. Each
    /// spelling below is a child of its own, with no knob set, and each must be
    /// refused by all four routes and by the engine. D-0695.
    #[tokio::test]
    async fn a_usable_server_budget_is_refused_before_the_slot_and_writes_nothing() {
        if let Some(root) = std::env::var_os(SERVER_BUDGET_CHILD) {
            server_budget_child(std::path::Path::new(&root)).await;
            return;
        }
        for (at, budget) in ["5000", " 5000\t", "\u{a0}5000"].into_iter().enumerate() {
            let root = crate::scratch::path(&format!("server-screen-budget-{at}"));
            let _ = std::fs::remove_dir_all(&root);
            let store = root.join("store");
            std::fs::create_dir_all(&store).expect("the child's empty store");
            std::fs::create_dir_all(root.join("masters")).expect("the child's empty masters");
            let out = crate::isolated::rerun(
                "sweeprun::tests::a_usable_server_budget_is_refused_before_the_slot_and_writes_nothing",
                &[
                    (SERVER_BUDGET_CHILD, root.as_os_str()),
                    ("BRUTEX_STORE", store.as_os_str()),
                    ("BRUTEX_SCREEN_BUDGET_MS", std::ffi::OsStr::new(budget)),
                ],
            );
            assert!(
                out.contains("SERVER-BUDGET refused 4 routes"),
                "{budget:?}: {out}"
            );
            let _ = std::fs::remove_dir_all(&root);
        }
    }

    /// Every file under `root`, with its length, in a stable order.
    fn listing(root: &std::path::Path) -> Vec<(std::path::PathBuf, u64)> {
        let mut out = Vec::new();
        let mut pending = vec![root.to_path_buf()];
        while let Some(dir) = pending.pop() {
            for entry in std::fs::read_dir(&dir).expect("a readable fixture directory") {
                let entry = entry.expect("a readable fixture entry");
                let meta = entry.metadata().expect("fixture metadata");
                if meta.is_dir() {
                    pending.push(entry.path());
                }
                out.push((entry.path(), meta.len()));
            }
        }
        out.sort();
        out
    }

    /// One sweep route, called by the child below.
    type Route<'a> = &'a dyn Fn() -> (axum::http::StatusCode, super::JsonHeaders, String);

    /// The child half of the test above: the budget is in this process's
    /// environment, and each route is asked with it there.
    ///
    /// # Why the child installs a sink
    ///
    /// AF-22 said the four routes "leave the in-process slot empty", and with
    /// no sink that could not fail: a route that skipped the budget guard took
    /// the lease, wrote the invocation record and then refused as
    /// `Unobservable`, because its attempt marker had nowhere to land, all
    /// before the slot is filled. A server always has a sink. With one here, a
    /// route past the guard fills the slot and starts a run, so the slot
    /// assertion is one the store listing does not stand in for. D-0695.
    async fn server_budget_child(root: &std::path::Path) {
        const STAMP: &str = "0123456789abcdef0123456789abcdef01234567";
        telemetry::install(&telemetry::Config::new(root.join("logs")))
            .expect("this child's one sink");
        let store = root.join("store");
        let site = std::sync::Arc::new(crate::server::Site::load(&root.join("masters"), &store));
        let before = listing(&store);
        let run = format!(r#"{{"feed":"zerodha","underlying":"NIFTY",{SPAN},"rungs":["5min"]}}"#);
        let descent = descent_body(r#""rung":"15min","max_points":20,"top":25"#);
        let audit = command_body(r#""command":"audit-range","rung":"15min","min_hits":500"#);
        let screen = command_body(
            r#""command":"screen","rung":"15min","support_ppm":50000,"max_points":20,"top":25"#,
        );
        // AN UNSTAMPED BUILD IS NAMED FIRST: the build is a fact decided before
        // the environment, and the order is one on every route.
        let (status, _, body) = super::descend_with(&site, &descent, None);
        assert_eq!(status, axum::http::StatusCode::SERVICE_UNAVAILABLE);
        assert!(body.contains("BRUTEX_COMMIT"), "{body}");
        assert!(!body.contains("BRUTEX_SCREEN_BUDGET_MS"), "{body}");
        assert_eq!(listing(&store), before, "unstamped descend wrote");
        // sobs-9, D-4447: THE BUDGET'S REFUSAL REACHES THIS CHILD'S LOG, once
        // per route and naming it, beside the unstamped one above.
        assert!(
            budget_refusals(root).is_empty(),
            "nothing refused for a budget yet"
        );

        // ONE ROUTE AT A TIME, each checked before the next is called, so a
        // route that writes is the one named.
        let routes: [(&str, Route<'_>); 4] = [
            ("run", &|| super::run_with(&site, &run, Some(STAMP))),
            ("descend", &|| {
                super::descend_with(&site, &descent, Some(STAMP))
            }),
            ("audit-range", &|| {
                super::command_with(&site, &audit, Some(STAMP))
            }),
            ("screen", &|| {
                super::command_with(&site, &screen, Some(STAMP))
            }),
        ];
        for (route, answer) in routes {
            let (status, _, body) = answer();
            assert_eq!(
                status,
                axum::http::StatusCode::SERVICE_UNAVAILABLE,
                "{route}: {body}"
            );
            assert!(body.contains(r#""accepted":false"#), "{route}: {body}");
            assert!(
                body.contains("BRUTEX_SCREEN_BUDGET_MS is set in this server's environment"),
                "{route}: {body}"
            );
            assert!(
                site.sweep.lock().expect("the slot").is_none(),
                "{route} occupied the slot"
            );
            assert_eq!(listing(&store), before, "{route} wrote into the store");
        }
        assert_eq!(
            budget_refusal_routes(&budget_refusals(root)),
            [
                super::RUN_ROUTE,
                super::DESCEND_ROUTE,
                super::COMMAND_ROUTE,
                super::COMMAND_ROUTE
            ],
            "one budget refusal per press, naming its route"
        );

        // THE ENVIRONMENT'S OWN SPELLING, WITH NO KNOB SET, READ BY THE ENGINE
        // TOO. This child's value may be padded; only here does the rule's trim
        // meet it, and the engine must refuse exactly what the routes refused.
        assert!(
            cli::range_over("zerodha", "NIFTY", &["5min"], (2025, 5), (2025, 5), None)
                .contains("BRUTEX_SCREEN_BUDGET_MS is set"),
            "the engine's reading of this environment"
        );

        server_budget_through_the_journal(&site, &store, &run).await;
        only_the_screen_words_carry_the_budget();
        the_budget_rule_is_clis();

        // PAST THE GUARD, EVERY ROUTE MEETS ITS OWN ADMISSION, which here is
        // the slot: one run is planted in flight, so a route that reaches the
        // slot answers 409 and starts nothing. Planted rather than started, as
        // `crate::emitted` plants its busy row.
        *site.sweep.lock().expect("the slot") = Some(super::Progress::started(
            "zerodha",
            "NIFTY",
            (2025, 5),
            (2025, 5),
            None,
            0,
            1,
        ));

        the_unpriced_words_are_not_refused_a_budget(&site, STAMP);

        // A VALUE THE BUDGET READER CANNOT USE REFUSES NO ROUTE: it is no
        // budget, and `cli` names it under KNOB REFUSED and runs without one.
        // Each route goes on to its own admission, the planted run's slot.
        cli::knobs::set("BRUTEX_SCREEN_BUDGET_MS", "0");
        for (route, answer) in routes {
            let (status, _, body) = answer();
            assert!(
                !body.contains("BRUTEX_SCREEN_BUDGET_MS"),
                "{route} refused an unusable budget: {body}"
            );
            assert_eq!(status, axum::http::StatusCode::CONFLICT, "{route}: {body}");
        }
        cli::knobs::clear_all();
        println!("SERVER-BUDGET refused 4 routes");
    }

    /// Every screen-budget refusal in [`server_budget_child`]'s own log,
    /// newest first, as `tail` answers.
    fn budget_refusals(root: &std::path::Path) -> Vec<telemetry::Record> {
        telemetry::tail(
            &root.join("logs"),
            telemetry::global()
                .expect("this child's one sink")
                .keep_files(),
            &telemetry::Query::last(telemetry::MAX_LIMIT).from_target("api.sweep"),
        )
        .records
        .into_iter()
        .filter(|record| {
            record.message
                == "a sweep was refused because this server's environment sets a screen budget"
        })
        .collect()
    }

    /// The route each budget refusal names, in the order the presses were
    /// made, each line checked to be a `Warn` carrying the refusal's sentence.
    fn budget_refusal_routes(lines: &[telemetry::Record]) -> Vec<String> {
        // `tail` answers newest first; the presses are listed in the order made.
        lines
            .iter()
            .rev()
            .map(|record| {
                assert!(
                    record
                        .field("why")
                        .and_then(telemetry::OwnedValue::as_str)
                        .is_some_and(|why| why.contains("BRUTEX_SCREEN_BUDGET_MS is set")),
                    "the refusal's own sentence rides on the line: {record:?}"
                );
                assert_eq!(record.level, telemetry::Level::Warn, "{record:?}");
                record
                    .field("route")
                    .and_then(telemetry::OwnedValue::as_str)
                    .unwrap_or_default()
                    .to_owned()
            })
            .collect()
    }

    /// THE WORDS WHOSE RUN PRICES NO SCREEN ARE NOT REFUSED FOR ONE, at the
    /// ROUTE and not only in `prices_a_screen`: nothing asked whether
    /// `command_with_configuration` consults the predicate at all, and with
    /// the guard applied to every word this suite stayed green. D-0695. The
    /// strict word refuses the budget through its own admission, in its own
    /// words and before the slot, and never with this route's sentence.
    fn the_unpriced_words_are_not_refused_a_budget(site: &crate::server::Loaded, stamp: &str) {
        for words in ORDINARY_WORDS.iter().skip(2) {
            let (status, _, body) = super::command_with(site, &command_body(words), Some(stamp));
            assert!(
                !body.contains("BRUTEX_SCREEN_BUDGET_MS is set in this server's environment"),
                "{words} was refused a budget its run never prices: {body}"
            );
            if words.contains("audit-audited-range") {
                assert!(
                    body.contains("strict_runtime_settings_invalid")
                        && body.contains("BRUTEX_SCREEN_BUDGET_MS"),
                    "the strict word names the budget through its own admission: {body}"
                );
            } else {
                assert_eq!(status, axum::http::StatusCode::CONFLICT, "{words}: {body}");
            }
        }
    }

    /// The server-budget refusal through the request journal the production
    /// router wraps every sweep route in, and what that journal then holds.
    ///
    /// The refusal said "nothing was written". `audited_router_serving` wraps
    /// `/backtest/run`, `/backtest/descend` and `/engine/command` in
    /// `operation_audit::note_request`, which journals the request's start
    /// before the handler runs and its 503 after, so on the router the operator
    /// uses that sentence was false the moment it was sent -- and AF-22's
    /// unchanged store held only because its test called the handler bare. The
    /// sentence now claims only the run's state. What must hold is exactly
    /// that: ONE journal row, the HTTP request's own, `Failed` at 503, no run
    /// invocation beside it, and nothing new under the store outside `audit/`.
    /// D-0695.
    async fn server_budget_through_the_journal(
        site: &crate::server::Loaded,
        store: &std::path::Path,
        run: &str,
    ) {
        const STAMP: &str = "0123456789abcdef0123456789abcdef01234567";
        let before = listing(store);
        let response = crate::operation_audit::request_audited(
            store.to_path_buf(),
            "POST /backtest/run".to_owned(),
            async {
                axum::response::IntoResponse::into_response(super::run_with(site, run, Some(STAMP)))
            },
        )
        .await;
        assert_eq!(
            response.status(),
            axum::http::StatusCode::SERVICE_UNAVAILABLE
        );
        let body = axum::body::to_bytes(response.into_body(), 1 << 16)
            .await
            .expect("the refusal's body");
        let body = String::from_utf8_lossy(&body);
        assert!(
            body.contains("BRUTEX_SCREEN_BUDGET_MS is set in this server's environment"),
            "{body}"
        );
        assert!(
            body.contains(super::BUDGET_NOT_RECORDABLE),
            "the reason is the file's one wording, as the body refusal's is: {body}"
        );
        assert!(
            body.contains("HTTP request journal still records the request itself"),
            "the sentence names the row the journal kept: {body}"
        );
        assert!(
            !body.contains("nothing was written"),
            "the journal wrote, so the sentence may not say otherwise: {body}"
        );
        assert!(
            site.sweep.lock().expect("the slot").is_none(),
            "the journalled refusal occupied the slot"
        );
        let rows = cli::operation_audit::page(store, None, 32).expect("the journal reads");
        assert_eq!(
            rows.len(),
            1,
            "one request, one row, and no run invocation beside it: {rows:?}"
        );
        let row = rows.first().expect("the request's own row");
        assert_eq!(row.origin, cli::operation_audit::Origin::Http);
        assert_eq!(row.label, "POST /backtest/run");
        assert_eq!(row.phase, cli::operation_audit::Phase::Failed);
        assert_eq!(row.response_status, 503);
        let journal = store.join("audit");
        for (path, _) in listing(store) {
            assert!(
                path.starts_with(&journal) || before.iter().any(|(was, _)| *was == path),
                "{} appeared outside the request journal",
                path.display()
            );
        }
    }

    /// The command words whose run prices a screen, and only those, carry the
    /// server-budget refusal.
    fn only_the_screen_words_carry_the_budget() {
        // THE WORDS WHOSE RUN PRICES NO SCREEN ARE NOT REFUSED FOR ONE: their
        // commands carry no budget and the predicate says so for each.
        for (words, prices) in [
            (
                r#""command":"audit-range","rung":"15min","min_hits":500"#,
                true,
            ),
            (
                r#""command":"screen","rung":"15min","support_ppm":50000,"max_points":20,"top":25"#,
                true,
            ),
            (r#""command":"auto-stored","rung":"1min""#, false),
            (
                r#""command":"sweep-stored","rung":"15min","min_hits":500"#,
                false,
            ),
            (
                r#""command":"sweep-all","rung":"15min","min_hits":500"#,
                false,
            ),
            (
                r#""command":"audit-audited-range","rung":"5min","min_hits":500"#,
                false,
            ),
        ] {
            let asked = super::command_from(&command_body(words)).expect("a good command");
            assert_eq!(asked.prices_a_screen(), prices, "{words}");
        }
    }

    /// The server-budget predicate is `cli`'s own rule, value by value.
    ///
    /// By construction since D-0695, because the route calls
    /// `cli::recorded_budget_refusal`; kept as the guard against a copy of the
    /// rule growing back. It cannot reach the TRIM: `cli::knobs::set` trims
    /// before it stores, so every value below arrives trimmed. The padded
    /// spellings the parent test puts in the ENVIRONMENT are what reach it.
    fn the_budget_rule_is_clis() {
        // THE SAME RULE AS `cli`'s, value by value. The knob store precedes the
        // environment in `cli::knobs::var`, so each value is set there and both
        // readers see it; `cli::range_over` refuses a usable budget in
        // `one_rung` before it resolves the store.
        for (value, usable) in [
            ("5000", true),
            ("+5000", true),
            ("00001", true),
            ("1", true),
            ("18446744073709551615", true),
            ("\u{a0}5000", true),
            ("0", false),
            ("-1", false),
            ("abc", false),
            ("18446744073709551616", false),
            ("5_000", false),
            ("0x10", false),
            ("1e3", false),
            ("\u{660}", false),
            ("\u{ff11}\u{ff12}", false),
        ] {
            cli::knobs::set("BRUTEX_SCREEN_BUDGET_MS", value);
            let here = super::environment_budget_refusal().is_some();
            let there = cli::range_over("zerodha", "NIFTY", &["5min"], (2025, 5), (2025, 5), None)
                .contains("BRUTEX_SCREEN_BUDGET_MS is set");
            assert_eq!((here, there), (usable, usable), "{value:?}");
        }
        cli::knobs::clear_all();
        assert!(
            super::environment_budget_refusal().is_some(),
            "cleared, the environment's own budget is read again"
        );
    }

    /// A MISSPELT BUDGET KEY IS AN UNKNOWN FIELD, AND AN UNKNOWN FIELD SETS
    /// NOTHING.
    ///
    /// `WireBody` ignores unknown fields for compatibility, so these spellings
    /// are accepted and dropped rather than refused. What must hold is that
    /// none of them reaches the knob store, so the run's identity and the
    /// candidates it prices are exactly those of the same body without it. The
    /// environment spelling is the likeliest slip and is included.
    #[test]
    fn a_misspelt_budget_key_reaches_no_knob() {
        let plain = asked_from(&format!(
            r#"{{"feed":"zerodha","underlying":"NIFTY",{SPAN}}}"#
        ))
        .expect("a good body");
        for key in [
            "Screen_Budget_Ms",
            "SCREEN_BUDGET_MS",
            "screen_budget_ms ",
            "screen-budget-ms",
            "screen_budget",
            "BRUTEX_SCREEN_BUDGET_MS",
        ] {
            let raw = format!(r#"{{"feed":"zerodha","underlying":"NIFTY",{SPAN},"{key}":5000}}"#);
            let asked = asked_from(&raw).expect("an unknown field is ignored");
            assert!(asked.knobs.is_empty(), "{key}: {:?}", asked.knobs);
            assert_eq!(asked, plain, "{key} changed what was asked");
        }
    }

    /// Every knob in the table is reachable from a body, so a control the page
    /// renders cannot be one the parser silently drops.
    #[test]
    fn every_knob_in_the_table_can_be_set_from_a_body() {
        for (asked_name, env_name) in KNOBS {
            let raw =
                format!(r#"{{"feed":"zerodha","underlying":"NIFTY",{SPAN},"{asked_name}":"7"}}"#);
            let asked = asked_from(&raw).expect("a good body");
            assert_eq!(
                asked.knobs,
                vec![(env_name, "7".to_owned())],
                "`{asked_name}` must reach the engine as `{env_name}`"
            );
        }
    }

    /// An empty value is ABSENCE. A browser field the operator cleared arrives
    /// as `""`, and setting a knob to the empty string would have every parse
    /// of it fail into a default — a silent fallback wearing a setting's
    /// clothes.
    #[test]
    fn an_empty_knob_value_sets_nothing() {
        for written in ["\"\"", "\"   \""] {
            let raw =
                format!(r#"{{"feed":"zerodha","underlying":"NIFTY",{SPAN},"top":{written}}}"#);
            let asked = asked_from(&raw).expect("a good body");
            assert!(
                asked.knobs.is_empty(),
                "`{written}` is nothing, not a value: {:?}",
                asked.knobs
            );
        }
    }

    #[test]
    fn the_threshold_is_derived_and_admits_what_the_constant_pruned() {
        // THE CONSTANT WAS 200,000 PPM -- 20% of every rung's bars -- and
        // `cli::elite_descend`'s own doc records that a figure a TENTH of that
        // "cannot report a once-a-week setup no matter how long it runs".
        //
        // The floor is derived now: the lowest support at which the stated win
        // rate could still clear its own confidence bound. These are this
        // store's own bar counts for the recorded span.
        let old_constant_hits = |bars: u64| bars * 200_000 / 1_000_000;
        let derived_hits = |bars: u64| bars * cli::statistical_support_floor(bars) / 1_000_000;

        for (bars, rung) in [(21_620_u64, "1min"), (11_643, "15min"), (1_671, "1day")] {
            let old = old_constant_hits(bars);
            let now = derived_hits(bars);
            assert!(
                now < old,
                "{rung}: the derived floor must admit rarer combinations than \
                 the constant did -- {now} hits against {old}"
            );
            // A HANDFUL OF ROUND TRIPS, NOT THOUSANDS. The shipped Wilson bound
            // needs four for an 80% rate; the constant demanded 4,324 on the
            // 1min rung, which is every setup an operator would call rare,
            // pruned before it was ever counted.
            assert!(
                now < 100,
                "{rung}: a floor of {now} hits is still a cadence nobody calls \
                 rare"
            );
        }

        // AND IT IS STILL A RATIO PER RUNG, which is what keeps the rungs
        // comparable at all: held as one absolute count, a 1min rung and a 1day
        // rung would clear the same number of hits from twenty times different
        // bar counts, and the daily rung would find nothing for a reason that
        // has nothing to do with the market.
        assert_ne!(
            cli::statistical_support_floor(21_620),
            cli::statistical_support_floor(1_671),
            "a floor derived from bar count cannot be equal across a 13x spread"
        );
    }

    #[test]
    fn a_month_outside_one_to_twelve_is_refused() {
        for span in [
            r#""from_year":2019,"from_month":0,"to_year":2026,"to_month":8"#,
            r#""from_year":2019,"from_month":13,"to_year":2026,"to_month":8"#,
            r#""from_year":2019,"from_month":1,"to_year":2026,"to_month":0"#,
            r#""from_year":2019,"from_month":1,"to_year":2026,"to_month":99"#,
        ] {
            let why = asked_from(&body("zerodha", span)).expect_err("a refusal");
            assert!(matches!(why, Refusal::Span(_)), "{span}");
            assert!(why.why().contains("1..=12"), "{}", why.why());
        }
    }

    #[test]
    fn a_span_that_ends_before_it_starts_is_refused_rather_than_swept_as_nothing() {
        let span = r#""from_year":2026,"from_month":8,"to_year":2019,"to_month":12"#;
        let why = asked_from(&body("zerodha", span)).expect_err("a refusal");
        assert!(matches!(why, Refusal::Span(_)));
        assert!(why.why().contains("ends before it starts"), "{}", why.why());
    }

    #[test]
    fn a_span_of_one_month_is_legal() {
        // The boundary: `to` EQUAL to `from` is a one-month span, not a
        // backwards one, and refusing it would refuse the cheapest useful run.
        let span = r#""from_year":2026,"from_month":8,"to_year":2026,"to_month":8"#;
        let asked = asked_from(&body("zerodha", span)).expect("one month");
        assert_eq!(asked.from, asked.to);
    }

    #[test]
    fn a_month_earlier_in_a_later_year_is_still_forward() {
        // 2019-12 -> 2020-01 crosses a year with a SMALLER month, which a
        // naive month-only comparison would call backwards.
        let span = r#""from_year":2019,"from_month":12,"to_year":2020,"to_month":1"#;
        assert!(asked_from(&body("zerodha", span)).is_ok());
    }

    #[test]
    fn a_support_threshold_of_zero_cannot_be_asked_for_at_all() {
        // THIS TEST USED TO ASSERT A REFUSAL. It sent `"support_ppm":0` and
        // checked for `Refusal::Support`, because zero makes every combination
        // frequent, so the frontier never empties and the walk has no end.
        //
        // The refusal is gone and the test is kept, because what it guards is
        // not gone: the property is now that the failure is UNREACHABLE rather
        // than caught. Deleting the test with the variant would have left
        // nothing saying why the constant may never be zero.
        // The property is unchanged and its subject moved: it used to be that
        // the CONSTANT could never be zero, and it is now that the DERIVED
        // floor never is. `statistical_support_floor` ends in `.max(1)` for
        // exactly this reason, and a bar count of zero -- a span the store
        // could not fill -- must not become a threshold of zero either.
        for bars in [0_u64, 1, 1_671, 21_620, 1_444_200] {
            assert!(
                cli::statistical_support_floor(bars) > 0,
                "extinction needs a threshold above zero, and {bars} bars gave \
                 none"
            );
        }
        // And a body that tries is simply a body with a field this route does
        // not read -- accepted, ignored, and swept at the real threshold.
        let raw = format!(r#"{{"feed":"zerodha","underlying":"NIFTY",{SPAN},"support_ppm":0}}"#);
        assert!(
            asked_from(&raw).is_ok(),
            "a stale field is not a malformed request"
        );
    }

    #[test]
    fn a_year_past_u16_is_refused_rather_than_truncated() {
        let span = r#""from_year":99999999,"from_month":1,"to_year":99999999,"to_month":2"#;
        let why = asked_from(&body("zerodha", span)).expect_err("a refusal");
        assert!(matches!(why, Refusal::Span(_)));
        assert!(why.why().contains("not a year"), "{}", why.why());
    }

    /// A year at the top of `u64` is refused, and does not reach a multiply.
    ///
    /// # Why the test above did not already cover this
    ///
    /// It uses 99,999,999. `99_999_999 * 12` is 1.2e9 -- comfortably inside
    /// `u64`, so it reached the `u16` bound and refused. The keys were computed
    /// as `from_year * 12` on the raw `u64` BEFORE that bound, so the input that
    /// mattered was one no test named: any year above `u64::MAX / 12`, which is
    /// 1,537,228,672,809,129,301. `overflow-checks = true` holds in `release`
    /// too, and `panic = "abort"` sits beside it, so the shipped binary answered
    /// that body by aborting the process rather than by refusing the field.
    ///
    /// **And `cargo test` could not see it**, which is the reason to write the
    /// case down rather than trust the profile. Tests build `dev`, which unwinds
    /// rather than aborting, so the panic surfaced inside hyper's connection
    /// task and killed one connection. The outage existed only in the profile no
    /// test runs. This asserts the refusal, which is the same answer in both.
    #[test]
    fn a_year_at_the_top_of_u64_is_refused_before_it_reaches_the_key_multiply() {
        for span in [
            // `u64::MAX`, the value that overflowed `* 12` first.
            r#""from_year":18446744073709551615,"from_month":1,"to_year":2026,"to_month":8"#,
            // One past `u64::MAX / 12` -- the exact boundary, from the other side.
            r#""from_year":1537228672809129302,"from_month":1,"to_year":2026,"to_month":8"#,
            // The same on `to_year`, because both keys are multiplied.
            r#""from_year":2019,"from_month":12,"to_year":18446744073709551615,"to_month":1"#,
        ] {
            let why = asked_from(&body("zerodha", span)).expect_err("a refusal");
            assert!(matches!(why, Refusal::Span(_)), "{}", why.why());
            assert!(why.why().contains("not a year"), "{}", why.why());
        }
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
    fn malformed_non_object_and_truncated_json_never_reaches_semantic_defaults() {
        let valid = r#"{"feed":"zerodha","underlying":"NIFTY","from_year":2026,
            "from_month":1,"to_year":2026,"to_month":1}"#;
        for raw in [
            // The old substring reader found every required key in this text
            // and accepted it even though it is not a JSON value at all.
            r#"garbage "feed":"zerodha","underlying":"NIFTY","from_year":2026,
               "from_month":1,"to_year":2026,"to_month":1 trailing"#,
            // A JSON value, but not the object this route's schema declares.
            r#"[{"feed":"zerodha","underlying":"NIFTY","from_year":2026,
                "from_month":1,"to_year":2026,"to_month":1}]"#,
            // The old list reader extracted `1min` and ignored the invalid tail.
            r#"{"feed":"zerodha","underlying":"NIFTY","from_year":2026,
                "from_month":1,"to_year":2026,"to_month":1,
                "rungs":["1min",garbage]}"#,
            // The old list reader treated a missing `]` as an absent list and
            // widened the request to every rung.
            r#"{"feed":"zerodha","underlying":"NIFTY","from_year":2026,
                "from_month":1,"to_year":2026,"to_month":1,"rungs":["1min""#,
        ] {
            let why = asked_from(raw).expect_err("the strict JSON boundary must refuse");
            assert!(
                why.why().contains("complete JSON object"),
                "the refusal names the structural boundary: {}",
                why.why()
            );
        }

        let trailing = format!("{valid} trailing");
        assert!(
            asked_from(&trailing).is_err(),
            "trailing non-JSON bytes cannot be ignored"
        );
    }

    #[test]
    fn duplicate_known_fields_are_refused_before_any_engine_work() {
        for raw in [
            r#"{"feed":"zerodha","feed":"dhan","underlying":"NIFTY",
                "from_year":2026,"from_month":1,"to_year":2026,"to_month":1}"#,
            r#"{"feed":"zerodha","underlying":"NIFTY","from_year":2026,
                "from_month":1,"to_year":2026,"to_month":1,
                "rungs":["1min"],"rungs":["15min"]}"#,
            r#"{"feed":"zerodha","underlying":"NIFTY","from_year":2026,
                "from_month":1,"to_year":2026,"to_month":1,"top":10,"top":25}"#,
        ] {
            let why = asked_from(raw).expect_err("a known key cannot decide twice");
            assert!(
                why.why().contains("duplicate field"),
                "serde must name the ambiguity: {}",
                why.why()
            );
        }
    }

    #[test]
    fn null_or_wrong_typed_known_fields_are_refused_not_treated_as_absent() {
        for raw in [
            r#"{"feed":"zerodha","underlying":"NIFTY","from_year":2026,
                "from_month":1,"to_year":2026,"to_month":1,"rungs":null}"#,
            r#"{"feed":"zerodha","underlying":"NIFTY","from_year":2026,
                "from_month":1,"to_year":2026,"to_month":1,"rungs":["1min",7]}"#,
            r#"{"feed":"zerodha","underlying":"NIFTY","from_year":2026,
                "from_month":1,"to_year":2026,"to_month":1,"top":{"n":25}}"#,
        ] {
            assert!(
                asked_from(raw).is_err(),
                "a present invalid field must not become the absent/default policy: {raw}"
            );
        }
    }

    #[test]
    fn escaped_strings_and_unknown_fields_keep_valid_request_compatibility() {
        let raw = r#"{"feed":"zero\u0064ha","underlying":"NI\u0046TY",
            "from_year":"2026","from_month":1,"to_year":2026,"to_month":"1",
            "rungs":["1\u006din"],"future":{"nested":[1,true]},
            "future_duplicate":1,"future_duplicate":2}"#;
        let asked = asked_from(raw).expect("valid JSON with unknown fields remains compatible");
        assert_eq!(asked.feed, "zerodha");
        assert_eq!(asked.underlying, "NIFTY");
        assert_eq!(asked.from, (2026, 1));
        assert_eq!(asked.to, (2026, 1));
        assert_eq!(asked.rungs, vec!["1min"]);
    }

    /* ==================== progress ==================== */

    #[test]
    fn a_started_run_is_in_flight_until_it_is_finished() {
        let mut p = Progress::started(
            "zerodha",
            "NIFTY",
            (2019, 12),
            (2026, 8),
            Some(200_000),
            42,
            43,
        );
        assert!(p.in_flight());
        assert_eq!(p.started_micros, 42);
        assert_eq!(p.attempt, 43);
        assert_eq!(p.finished_micros, None);
        assert_eq!(p.report, None);
        p.finished_micros = Some(99);
        assert!(!p.in_flight());
    }

    /// The adversarial path the guard exists for: control unwinds before the
    /// task reaches any of its ordinary completion writes.
    #[test]
    fn a_panicking_engine_task_becomes_a_visible_refusal_and_releases_the_slot() {
        let site = finisher_site("panic");
        *site
            .sweep
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(Progress::started(
            "zerodha",
            "NIFTY",
            (2020, 1),
            (2020, 1),
            None,
            7,
            70,
        ));

        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe({
            let site = std::sync::Arc::clone(&site);
            move || {
                let _finisher = TaskFinisher::new(site);
                std::panic::resume_unwind(Box::new(
                    "engine panic injected after the slot was claimed",
                ));
            }
        }));
        assert!(caught.is_err(), "the injected panic must actually unwind");

        let held = site
            .sweep
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let progress = held.as_ref().expect("the failed run remains visible");
        assert!(
            !progress.in_flight(),
            "a dead task must not keep the slot busy"
        );
        assert!(progress.finished_micros.is_some(), "the end is timestamped");
        assert_eq!(progress.report, None, "an abnormal end is not a report");
        assert_eq!(progress.refusal.as_deref(), Some(ABNORMAL_END));
        assert_eq!(
            progress.started_micros, 7,
            "the accepted run stays identifiable"
        );
    }

    /// Normal completion and abnormal completion race through one destructor.
    /// Disarming before that destructor runs is what prevents an old guard from
    /// failing a new run which claimed the now-finished slot immediately.
    #[test]
    fn a_normal_finisher_preserves_its_answer_and_cannot_abort_the_next_run() {
        let site = finisher_site("normal");
        *site
            .sweep
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(Progress::started(
            "zerodha",
            "NIFTY",
            (2020, 1),
            (2020, 1),
            None,
            7,
            70,
        ));

        let mut done = Progress::started("zerodha", "NIFTY", (2020, 1), (2020, 1), None, 7, 70);
        done.finished_micros = Some(8);
        done.report = Some("STORED_PROVENANCE\ncomplete".into());
        TaskFinisher::new(std::sync::Arc::clone(&site)).finish(done);

        {
            let held = site
                .sweep
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let progress = held.as_ref().expect("the normal answer remains visible");
            assert_eq!(progress.finished_micros, Some(8));
            assert_eq!(
                progress.report.as_deref(),
                Some("STORED_PROVENANCE\ncomplete")
            );
            assert_eq!(progress.refusal, None);
        }

        *site
            .sweep
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(Progress::started(
            "dhan",
            "BANKNIFTY",
            (2020, 2),
            (2020, 2),
            None,
            9,
            90,
        ));
        let held = site
            .sweep
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let next = held.as_ref().expect("the next run owns the slot");
        assert!(
            next.in_flight(),
            "the disarmed old guard cannot fail the next run"
        );
        assert_eq!(next.started_micros, 9);
        assert_eq!(next.refusal, None);
    }

    #[test]
    fn an_armed_guard_does_not_rewrite_a_slot_that_is_already_finished() {
        let site = finisher_site("already-finished");
        let mut done = Progress::started("zerodha", "NIFTY", (2020, 1), (2020, 1), None, 7, 70);
        done.finished_micros = Some(8);
        done.refusal = Some("the engine gave its own refusal".into());
        *site
            .sweep
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(done);

        drop(TaskFinisher::new(std::sync::Arc::clone(&site)));

        let held = site
            .sweep
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let progress = held.as_ref().expect("the answer remains visible");
        assert_eq!(progress.finished_micros, Some(8));
        assert_eq!(
            progress.refusal.as_deref(),
            Some("the engine gave its own refusal")
        );
    }

    #[test]
    fn all_three_browser_engine_tasks_arm_and_disarm_the_same_finisher() {
        // The three producers share one slot: sweep, descent and command. A
        // guard on two of them is still one route that can wedge the process,
        // so pin the production census against the module before its tests.
        let production = include_str!("sweeprun.rs")
            .split_once("\n#[cfg(test)]")
            .expect("this module has one test boundary")
            .0;
        let armed = ["let guard = Task", "Finisher::audited"].concat();
        let disarmed = ["guard.", "finish(done);"].concat();
        assert_eq!(production.matches(&armed).count(), 3, "one guard per task");
        let before_spawn = [
            armed.as_str(),
            "(std::sync::Arc::clone(site), audit).with_lease(lease);\n    tokio::task::spawn_blocking(move || {",
        ]
        .concat();
        assert_eq!(
            production.matches(&before_spawn).count(),
            2,
            "sweep and descent arm immediately before queueing, with no intervening await"
        );
        let command_before_spawn = [
            armed.as_str(),
            "(std::sync::Arc::clone(site), audit).with_lease(lease);\n    let store_root = site.store_root.clone();\n    let launch_site = std::sync::Arc::clone(site);\n    tokio::task::spawn_blocking(move || {",
        ]
        .concat();
        assert_eq!(
            production.matches(&command_before_spawn).count(),
            1,
            "the command guard also protects the synchronous store-path and status-slot clones; no await, return, or other work may intervene before queueing"
        );
        assert_eq!(
            production.matches(&disarmed).count(),
            3,
            "every normal exit must disarm its guard"
        );
        assert_eq!(
            production
                .matches("marker_refusal(emit_attempt_started(&accepted))")
                .count(),
            3,
            "every task must durably bracket its exact attempt before occupying the slot"
        );
    }

    #[test]
    fn the_progress_json_carries_every_field_and_nulls_what_has_not_happened() {
        let p = Progress::started(
            "zerodha",
            "NIFTY",
            (2019, 12),
            (2026, 8),
            Some(200_000),
            42,
            43,
        );
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
            r#""attempt":43"#,
            r#""in_flight":true"#,
            r#""finished_micros":null"#,
            r#""report":null"#,
            r#""refusal":null"#,
        ] {
            assert!(json.contains(fragment), "missing {fragment} in {json}");
        }
    }

    #[test]
    fn the_attempt_marker_fits_the_field_ceiling_and_carries_the_whole_question() {
        let progress = Progress::started("zerodha", "NIFTY", (2019, 12), (2026, 8), None, 42, 43)
            .of_kind(Kind::Descent);
        let event = attempt_started_event(&progress);
        assert_eq!(event.target(), "cli.audit");
        assert_eq!(event.message(), "sweep attempt started");
        assert_eq!(event.fields().len(), 8);
        assert_eq!(event.dropped_fields(), 0);
        let keys: Vec<&str> = event.fields().iter().map(|(key, _)| *key).collect();
        assert_eq!(
            keys,
            [
                "attempt",
                "kind",
                "feed",
                "underlying",
                "from_year",
                "from_month",
                "to_year",
                "to_month"
            ]
        );
    }

    #[test]
    fn an_engine_task_starts_only_after_its_required_attempt_marker_was_written() {
        assert_eq!(marker_refusal(telemetry::Emitted::Written), None);
        for (outcome, phrase) in [
            (telemetry::Emitted::Filtered, "filtered"),
            (telemetry::Emitted::Dropped, "could not be written"),
            (telemetry::Emitted::NotInstalled, "no telemetry sink"),
        ] {
            let refusal = marker_refusal(outcome).expect("every missing marker refuses");
            assert_eq!(
                refusal.status(),
                axum::http::StatusCode::SERVICE_UNAVAILABLE
            );
            assert!(refusal.why().contains(phrase), "{}", refusal.why());
            assert!(
                refusal.why().contains("no engine work was started")
                    || refusal.why().contains("No engine work was started"),
                "{}",
                refusal.why()
            );
        }
    }

    #[test]
    fn a_finished_run_carries_its_report_and_its_stamp() {
        let mut p = Progress::started("zerodha", "NIFTY", (2020, 1), (2020, 1), Some(50_000), 1, 2);
        p.finished_micros = Some(500);
        p.report = Some("STORED_PROVENANCE\nrows".into());
        let json = p.to_json();
        assert!(json.contains(r#""in_flight":false"#), "{json}");
        assert!(json.contains(r#""finished_micros":500"#), "{json}");
        assert!(json.contains("STORED_PROVENANCE"), "{json}");
        // The report is JSON-escaped, so its newline does not break the body.
        assert!(!json.contains("PROVENANCE\nrows"), "the newline is escaped");
    }

    #[test]
    fn a_refusal_reaches_the_progress_json_as_a_sentence() {
        let mut p = Progress::started("zerodha", "NIFTY", (2020, 1), (2020, 1), Some(50_000), 1, 2);
        p.refusal = Some("the store held no bars".into());
        assert!(p.to_json().contains("the store held no bars"));
    }

    #[test]
    fn the_clock_is_after_the_epoch_and_does_not_panic() {
        // Zero is the honest answer on a machine whose clock is before the
        // epoch; anything else here would be a real timestamp.
        assert!(now_micros() > 0);
    }

    #[test]
    fn a_backwards_span_is_refused_however_the_months_fall() {
        // MUTATION TESTING FOUND THE HOLE THIS CLOSES. `from_key` is
        // `year * 12 + month`; the mutant made it `year * 12 - month` and
        // every existing span test still passed, because they all compared
        // two keys that moved together.
        //
        // 2020-11 -> 2019-12 is the case that separates them. The `to` key is
        // 24240; the `from` key is 24251 correct and 24229 mutated, so the
        // comparison lands on opposite sides and a BACKWARDS SPAN IS ACCEPTED
        // by the mutant. That is a real defect, not a formality: the sweep
        // would have run over a window nobody asked for and recorded it under
        // an identity naming the span it was given.
        for span in [
            r#""from_year":2020,"from_month":11,"to_year":2019,"to_month":12"#,
            r#""from_year":2020,"from_month":2,"to_year":2019,"to_month":11"#,
            r#""from_year":2026,"from_month":1,"to_year":2025,"to_month":12"#,
        ] {
            let why =
                asked_from(&body("zerodha", span)).expect_err("a backwards span must be refused");
            assert!(matches!(why, Refusal::Span(_)), "{span}");
            assert!(why.why().contains("ends before it starts"), "{}", why.why());
        }

        // And the mirrors, which must all be ACCEPTED — a rule that refuses a
        // backwards span by refusing everything is not a rule.
        for span in [
            r#""from_year":2019,"from_month":12,"to_year":2020,"to_month":11"#,
            r#""from_year":2019,"from_month":11,"to_year":2020,"to_month":2"#,
            r#""from_year":2025,"from_month":12,"to_year":2026,"to_month":1"#,
        ] {
            assert!(
                asked_from(&body("zerodha", span)).is_ok(),
                "forward span refused: {span}"
            );
        }
    }

    #[test]
    fn the_clock_returns_a_real_epoch_stamp_and_not_a_placeholder() {
        // MUTATION TESTING FOUND THIS TOO. The old assertion was `> 0`, and
        // the mutant `now_micros() -> 1` satisfies it. A stamp of 1 is 1
        // microsecond after 1970: it would sort every run before every other
        // run and render as 01 Jan 1970 on the page.
        //
        // The floor is 2023-11-14, which is in the past and will stay there,
        // so this cannot fail on a correct clock. The ceiling catches the
        // other direction: seconds or milliseconds returned where micros were
        // promised would land far below it.
        let now = now_micros();
        assert!(
            now > 1_700_000_000_000_000,
            "not a microsecond epoch stamp: {now}"
        );
        assert!(
            now < 100_000_000_000_000_000,
            "implausibly far in the future: {now}"
        );
        // Two reads are ordered, which a constant could not be.
        assert!(now_micros() >= now);
    }

    /* ==================== the outcome fields ==================== */

    /// Current range refusal: no completed result is confirmed, with the
    /// concrete reason retained even when source reads already happened.
    const REFUSAL: &str = "refused: every one of the 8 rungs refused. No completed \
                          result could be confirmed.\n  first reason: \
                          `nosuchfeed` is not a feed this engine reads.\n";

    /// Previously recorded refusal wording remains a refusal when replayed.
    const LEGACY_REFUSAL: &str = "refused: every one of the 9 rungs refused. Nothing was \
                           read and no row was recorded.\n  first reason: \
                           `nosuchfeed` is not a feed this engine reads.\n";

    /// A report opens with the provenance banner, which is what makes it one.
    const REPORT: &str = "THESE BARS ARE REAL MARKET DATA, READ FROM THE STORE\n\
                          feed zerodha · NIFTY · ALL EIGHT INTRADAY RUNGS\n";

    fn settled(text: &str) -> Progress {
        let mut progress = Progress::started("zerodha", "NIFTY", (2019, 12), (2026, 8), None, 1, 2);
        settle(&mut progress, text.to_owned(), 2);
        progress
    }

    #[test]
    fn a_refused_sweep_is_a_refusal_and_never_a_report() {
        for text in [REFUSAL, LEGACY_REFUSAL] {
            let progress = settled(text);
            assert_eq!(progress.refusal.as_deref(), Some(text));
            assert!(
                progress.report.is_none(),
                "a refusal filed as a report is the green `Sweep finished` over an \
                 empty ledger that this split exists to prevent: {:?}",
                progress.report
            );
        }
    }

    #[test]
    fn a_real_report_is_a_report_and_never_a_refusal() {
        let progress = settled(REPORT);
        assert_eq!(progress.report.as_deref(), Some(REPORT));
        assert!(
            progress.refusal.is_none(),
            "a completed sweep marked refused would send the operator hunting a \
             fault that is not there: {:?}",
            progress.refusal
        );
    }

    #[test]
    fn completion_audit_never_invents_a_ledger_commit() {
        let report = settled(REPORT);
        let reported = completion_audit(&report);
        assert_eq!(reported.level, telemetry::Level::Info);
        assert_eq!(reported.outcome, "report");
        assert_eq!(reported.why, "");

        let refused = settled(REFUSAL);
        let refusal = completion_audit(&refused);
        assert_eq!(refusal.level, telemetry::Level::Warn);
        assert_eq!(refusal.outcome, "refused");
        assert_eq!(refusal.why, REFUSAL);
    }

    #[test]
    fn contradictory_or_missing_completion_state_is_an_error() {
        let mut both = settled(REPORT);
        both.refusal = Some(REFUSAL.into());
        let contradiction = completion_audit(&both);
        assert_eq!(contradiction.level, telemetry::Level::Error);
        assert_eq!(contradiction.outcome, "invalid");
        assert!(contradiction.why.contains("both"));

        let empty = Progress::started("zerodha", "NIFTY", (2026, 1), (2026, 1), None, 1, 2);
        let missing = completion_audit(&empty);
        assert_eq!(missing.level, telemetry::Level::Error);
        assert_eq!(missing.outcome, "invalid");
        assert!(missing.why.contains("neither"));
    }

    #[test]
    fn exactly_one_outcome_field_is_set_whatever_cli_returned() {
        // THE INVARIANT IS `EXACTLY ONE`, WHICH IS TWO FAILURES AND NOT ONE.
        // Both set is a contradiction the page would resolve arbitrarily;
        // neither set is the original bug in its other direction, because the
        // page falls through to `done` when it finds no refusal.
        for text in [
            REFUSAL,
            REPORT,
            "",
            "refused",
            "refused at the ceiling, so no floor could be derived\n",
        ] {
            let progress = settled(text);
            assert_eq!(
                usize::from(progress.report.is_some()) + usize::from(progress.refusal.is_some()),
                1,
                "both or neither were set for {text:?}"
            );
        }
    }

    #[test]
    fn a_settled_run_is_no_longer_in_flight_whichever_way_it_ended() {
        // `in_flight` is the page's ONLY test of doneness, so a settle that
        // left the stamp unset would poll for ever against a finished sweep.
        assert!(!settled(REPORT).in_flight());
        assert!(!settled(REFUSAL).in_flight());
    }

    #[test]
    fn a_refusal_reaches_the_page_as_a_refusal_and_a_null_report() {
        for (text, field) in [
            (REFUSAL, r#""refusal":"refused: every one of the 8 rungs"#),
            (
                LEGACY_REFUSAL,
                r#""refusal":"refused: every one of the 9 rungs"#,
            ),
        ] {
            let json = settled(text).to_json();
            assert!(
                json.contains(r#""report":null"#),
                "a refusal must not also arrive as a report: {json}"
            );
            assert!(
                json.contains(field),
                "this is the field the page reads to decide it failed: {json}"
            );
            assert!(json.contains(r#""in_flight":false"#), "{json}");
        }
    }

    /* ==================== the commit gate ==================== */

    #[test]
    fn an_unstamped_build_refuses_before_a_single_bar_is_read() {
        // MEASURED, by running the command this route calls: `cli audit-range`
        // over 121 months of daily bars refused with exactly this cause, and the
        // answer never depended on one bar it read. The gate lives inside
        // `audit_range`, so a browser sweep claimed the slot, spawned a thread,
        // loaded nine spans and refused nine times for a fact decided when the
        // binary was compiled.
        let why = stamp_refusal(None).expect("an unstamped build must refuse");

        assert!(
            why.why().contains("BRUTEX_COMMIT"),
            "the operator cannot act on a refusal that does not name the \
             variable: {}",
            why.why()
        );
        assert!(
            why.why().contains("Nothing was swept"),
            "saying nothing ran is the difference between this and the nine-rung \
             refusal it replaces: {}",
            why.why()
        );
    }

    #[test]
    fn only_a_canonical_stamped_build_does_not_refuse() {
        assert!(stamp_refusal(Some("0123456789abcdef0123456789abcdef01234567")).is_none());
        for invalid in [
            "",
            "3b8f5af",
            "0123456789abcdef0123456789abcdef0123456g",
            "0123456789ABCDEF0123456789ABCDEF01234567",
            " 0123456789abcdef0123456789abcdef01234567",
        ] {
            let why = stamp_refusal(Some(invalid)).expect("invalid stamp must refuse");
            assert_eq!(why.status(), axum::http::StatusCode::SERVICE_UNAVAILABLE);
            assert!(why.why().contains("canonical"), "{}", why.why());
        }
    }

    #[test]
    fn an_unstamped_build_is_503_and_never_a_bad_request() {
        // 400 WOULD BLAME THE BODY, WHICH IS PERFECT. The fix is on the server's
        // side -- a rebuild and a restart -- and 503 is the code that says so.
        let why = stamp_refusal(None).expect("refuses");
        assert_eq!(
            why.status(),
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            "{}",
            why.why()
        );
        assert_ne!(why.status(), axum::http::StatusCode::BAD_REQUEST);
        assert_ne!(
            why.status(),
            axum::http::StatusCode::CONFLICT,
            "a second press is not what is wrong"
        );
    }

    /* ==================== the descent ==================== */

    fn descent_body(extra: &str) -> String {
        format!(r#"{{"feed":"zerodha","underlying":"NIFTY",{SPAN},{extra}}}"#)
    }

    #[test]
    fn a_whole_descent_body_parses_into_the_seven_things_it_needs() {
        let asked = descent_from(&descent_body(r#""rung":"15min","max_points":20,"top":25"#))
            .expect("a good body");

        assert_eq!(
            asked,
            AskedDescent {
                feed: "zerodha".to_owned(),
                underlying: "NIFTY".to_owned(),
                rung: "15min".to_owned(),
                from: (2019, 12),
                to: (2026, 8),
                max_points: 20,
                top: 25,
            }
        );
    }

    #[test]
    fn the_descent_reuses_the_sweeps_span_rules_rather_than_restating_them() {
        // TWO PARSERS FOR ONE SPAN IS TWO PLACES FOR A BOUND TO DRIFT. The year
        // that once called `abort()` in a release build must be refused here for
        // free, because this parser delegates rather than re-deriving.
        let body = r#"{"feed":"zerodha","underlying":"NIFTY","from_year":18446744073709551615,
                       "from_month":1,"to_year":2026,"to_month":8,
                       "rung":"15min","max_points":20,"top":25}"#;
        assert!(
            descent_from(body).is_err(),
            "the span bound must still bite"
        );

        // And a backwards span, which is the sweep's other span refusal.
        let backwards = r#"{"feed":"zerodha","underlying":"NIFTY","from_year":2026,
                            "from_month":8,"to_year":2019,"to_month":12,
                            "rung":"15min","max_points":20,"top":25}"#;
        assert!(descent_from(backwards).is_err());
    }

    #[test]
    fn a_descent_without_a_rung_is_refused_and_names_the_eight() {
        let why = descent_from(&descent_body(r#""max_points":20,"top":25"#))
            .expect_err("a descent walks ONE rung, so it must be told which");

        assert!(why.why().contains("rung"), "{}", why.why());
        // `30min` REPLACES `1day` IN THIS LIST, and the swap is the bug this
        // test used to encode. It asserted the refusal named `1day` — so when
        // api's copy of the rung table drifted to include `1day` and drop
        // `30min`, this test AGREED with the drift instead of catching it.
        // An assertion written against the wrong list defends the wrong list.
        for rung in ["1min", "15min", "30min", "60min"] {
            assert!(
                why.why().contains(rung),
                "the operator cannot pick from a list they are not shown: {}",
                why.why()
            );
        }
        assert!(
            !why.why().contains("1day"),
            "`1day` is not swept and must not be offered as a choice: {}",
            why.why()
        );
    }

    #[test]
    fn a_rung_this_engine_does_not_sweep_is_refused_by_name() {
        let why = descent_from(&descent_body(r#""rung":"4h","max_points":20,"top":25"#))
            .expect_err("4h is not one of the eight");
        assert!(why.why().contains("4h"), "{}", why.why());
        assert_eq!(why.status(), axum::http::StatusCode::BAD_REQUEST);
    }

    #[test]
    fn a_negative_stop_ceiling_is_refused_rather_than_swept_with() {
        for points in ["-1", "-20"] {
            let why = descent_from(&descent_body(&format!(
                r#""rung":"15min","max_points":{points},"top":25"#
            )))
            .expect_err("a negative ceiling is not a distance");
            assert!(why.why().contains("max_points"), "{}", why.why());
        }
        // AND A MISSING ONE IS NOT ZERO. Defaulting it would pick the
        // operator's risk for them, silently.
        assert!(descent_from(&descent_body(r#""rung":"15min","top":25"#)).is_err());
    }

    /// D-1732. `cli` and `Rules::admits` read a zero ceiling as NO ceiling,
    /// and `USAGE` documents it so. The api refused it at both doors with "a
    /// ceiling of zero admits no trade", which is false: the browser could not
    /// ask for the run the command line offers.
    #[test]
    fn a_stop_ceiling_of_zero_means_no_ceiling_as_cli_reads_it() {
        let asked = descent_from(&descent_body(r#""rung":"15min","max_points":0,"top":25"#))
            .expect("zero is no ceiling, not a refusal");
        assert_eq!(asked.max_points, 0);
        let screen = command_from(&command_body(
            r#""command":"screen","rung":"15min","support_ppm":50000,"max_points":0,"top":25"#,
        ))
        .expect("the screen door reads zero the same way");
        assert!(
            matches!(screen, super::Command::Screen { max_points: 0, .. }),
            "{screen:?}"
        );
    }

    #[test]
    fn a_listing_bound_of_zero_is_refused() {
        let why = descent_from(&descent_body(r#""rung":"15min","max_points":20,"top":0"#))
            .expect_err("zero rows is no answer");
        assert!(why.why().contains("top"), "{}", why.why());
        assert!(descent_from(&descent_body(r#""rung":"15min","max_points":20"#)).is_err());
    }

    #[test]
    fn the_rung_list_here_is_the_one_cli_sweeps_by_and_not_a_copy_of_it() {
        // THIS TEST REPLACES ONE THAT PASSED WHILE THE LIST WAS WRONG, and how
        // it passed is the point.
        //
        // The old version asked `cli::elite_descend_in_points` about
        // `no-such-rung` and asserted that every name in api's own copy
        // appeared in the refusal. Two things defeated it:
        //
        //   * `elite_descend_in_points` does NOT validate against `EVERY_RUNG`.
        //     It hands the name to `stored::load_span`, whose refusal lists the
        //     STORE's nine timeframes. So the assertion compared api's list to
        //     the store's, not to the engine's.
        //   * It ran in ONE DIRECTION -- api ⊆ refusal. A name the engine
        //     sweeps and api omitted was invisible.
        //
        // Both together let the copy sit at
        //     1min 2min 3min 5min 10min 15min 60min 1day
        // against the engine's
        //     1min 2min 3min 5min 10min 15min 30min 60min
        // -- missing `30min`, accepting `1day` -- while the suite stayed green.
        //
        // There is no copy now: `EVERY_RUNG` is re-exported from `cli`. This
        // asserts THAT, so the day someone reintroduces a local array to avoid
        // the dependency, this fails rather than measuring the wrong thing.
        assert_eq!(
            EVERY_RUNG,
            cli::EVERY_RUNG,
            "the rung table must BE cli's, not agree with it"
        );

        // And the engine's own list is the eight intraday rungs. `1day` is on
        // disk to define the previous session's OHLC for them and is never
        // swept; if it ever appears here, a `1day` run can reach the ledger
        // through this route again.
        assert!(
            !EVERY_RUNG.contains(&"1day"),
            "`1day` is not a signal timeframe and must not be offered: {EVERY_RUNG:?}"
        );
        assert!(
            EVERY_RUNG.contains(&"30min"),
            "`30min` is a rung the engine sweeps: {EVERY_RUNG:?}"
        );
        assert!(
            EVERY_RUNG.iter().all(|r| r.ends_with("min")),
            "every swept rung is intraday: {EVERY_RUNG:?}"
        );
    }

    #[test]
    fn a_descent_and_a_sweep_are_told_apart_on_the_wire() {
        // `support_ppm` MEANS DIFFERENT THINGS IN THE TWO, so the page cannot
        // read it correctly without being told which it is looking at.
        let sweep = Progress::started("zerodha", "NIFTY", (2019, 12), (2026, 8), None, 1, 2);
        assert_eq!(sweep.kind, Kind::Sweep, "the default is the older command");
        assert!(sweep.to_json().contains(r#""kind":"sweep""#));

        let descent = sweep.clone().of_kind(Kind::Descent);
        assert_eq!(descent.kind, Kind::Descent);
        assert!(descent.to_json().contains(r#""kind":"descent""#));
        assert_ne!(Kind::Sweep.word(), Kind::Descent.word());

        // AND NOTHING ELSE MOVED. `of_kind` changes one field.
        assert_eq!(descent.feed, sweep.feed);
        assert_eq!(descent.support_ppm, sweep.support_ppm);
        assert_eq!(descent.started_micros, sweep.started_micros);
    }

    /* ==================== the command dispatcher ==================== */

    /// A good body for `extra`'s word. `sweep-all` takes no instrument and no
    /// span, and naming either is refused (D-1972), so it gets neither.
    fn command_body(extra: &str) -> String {
        if extra.contains(r#""command":"sweep-all""#) {
            return format!(r#"{{"feed":"zerodha",{extra}}}"#);
        }
        format!(r#"{{"feed":"zerodha","underlying":"NIFTY",{SPAN},{extra}}}"#)
    }

    /// P3-01-02 and P3-02-06, D-1972. Every route refuses, BY NAME, a known
    /// member it does not read, instead of decoding it and dropping it.
    #[test]
    fn every_route_refuses_a_known_field_it_does_not_read_by_name() {
        // `/backtest/run` reads `rungs`; a singular `rung` swept all eight.
        let why = asked_from(&body_with("zerodha", SPAN, r#","rung":"5min""#))
            .expect_err("rung is descend's member");
        assert!(
            why.why()
                .contains("`rung` is not read by `POST /backtest/run`"),
            "{}",
            why.why()
        );
        assert!(
            why.why().contains("No setting was ignored"),
            "{}",
            why.why()
        );
        let why = asked_from(&body_with("zerodha", SPAN, r#","max_points":50"#))
            .expect_err("a sweep has no stop ceiling");
        assert!(why.why().contains("`max_points`"), "{}", why.why());
        // ...while every knob it applies is still taken.
        assert!(
            asked_from(&body_with(
                "zerodha",
                SPAN,
                r#","validate":"0","screen_cap":7"#
            ))
            .is_ok()
        );

        // A descent applies no knob.
        let descent = |extra: &str| {
            descent_from(&format!(
                r#"{{"feed":"dhan","underlying":"NIFTY",{SPAN},"rung":"5min","max_points":50,"top":5{extra}}}"#
            ))
        };
        assert!(descent("").is_ok());
        for (extra, name) in [
            (r#","validate":"0""#, "validate"),
            (r#","screen_cap":7"#, "screen_cap"),
            (r#","support_ppm":100"#, "support_ppm"),
            (r#","rungs":["5min"]"#, "rungs"),
            (r#","min_hits":5"#, "min_hits"),
        ] {
            let why = descent(extra).expect_err("a descent applies no knob");
            assert!(
                why.why()
                    .contains(&format!("`{name}` is not read by `POST /backtest/descend`")),
                "{}",
                why.why()
            );
        }

        // The ordinary command words, each with a member it does not read.
        for (words, name) in [
            (
                r#""command":"audit-range","rung":"15min","min_hits":5,"validate":"0""#,
                "validate",
            ),
            (
                r#""command":"sweep-stored","rung":"15min","min_hits":5,"screen_cap":7"#,
                "screen_cap",
            ),
            (
                r#""command":"auto-stored","rung":"1min","min_hits":5"#,
                "min_hits",
            ),
            (
                r#""command":"screen","rung":"15min","support_ppm":5,"max_points":2,"top":2,"ceiling":3"#,
                "ceiling",
            ),
            (
                r#""command":"audit-audited-range","rung":"5min","min_hits":5,"rungs":["5min"]"#,
                "rungs",
            ),
            (
                r#""command":"sweep-all","rung":"15min","min_hits":5,"validate":"0""#,
                "validate",
            ),
        ] {
            let why = command_from(&command_body(words)).expect_err(words);
            assert!(
                why.why()
                    .contains(&format!("`{name}` is not read by the `")),
                "{words}: {}",
                why.why()
            );
        }
        // The strict word still takes its knobs.
        assert!(
            command_from(&command_body(
                r#""command":"audit-audited-range","rung":"5min","min_hits":5,"validate":"0""#
            ))
            .is_ok()
        );
    }

    /// P3-02-06, D-1972. `sweep-all` takes what `cli sweep-all VENDOR RUNG
    /// MIN_HITS` takes. A body naming an instrument or a span is refused,
    /// because it would otherwise become a whole-store batch.
    #[test]
    fn sweep_all_takes_no_instrument_and_no_span() {
        let documented =
            r#"{"command":"sweep-all","feed":"zerodha","rung":"15min","min_hits":500}"#;
        let batch = command_from(documented).expect("the documented shape parses");
        assert_eq!(batch.word(), "sweep-all");
        assert_eq!(batch.feed(), "zerodha");
        for extra in [
            r#","underlying":"BANKNIFTY""#,
            r#","from_year":2024"#,
            r#","from_month":1"#,
            r#","to_year":2024"#,
            r#","to_month":1"#,
        ] {
            let body = format!(
                r#"{{"command":"sweep-all","feed":"zerodha","rung":"15min","min_hits":500{extra}}}"#
            );
            let why = command_from(&body).expect_err("a scoped-looking batch");
            assert!(
                why.why().contains("is not read by the `sweep-all` command"),
                "{}",
                why.why()
            );
        }
        let why = command_from(r#"{"command":"sweep-all","rung":"15min","min_hits":500}"#)
            .expect_err("the feed is still required");
        assert!(why.why().contains("no `feed`"), "{}", why.why());
    }

    #[test]
    fn every_stored_command_parses_into_its_own_shape() {
        let audit = command_from(&command_body(
            r#""command":"audit-range","rung":"15min","min_hits":500"#,
        ))
        .expect("audit-range");
        assert_eq!(audit.word(), "audit-range");
        assert_eq!(audit.feed(), "zerodha");
        assert_eq!(audit.underlying(), "NIFTY");

        let screen = command_from(&command_body(
            r#""command":"screen","rung":"15min","support_ppm":50000,"max_points":20,"top":25"#,
        ))
        .expect("screen");
        assert_eq!(screen.word(), "screen");

        let auto = command_from(&command_body(r#""command":"auto-stored","rung":"1min""#))
            .expect("auto-stored");
        assert_eq!(auto.word(), "auto-stored");

        let batch = command_from(&command_body(
            r#""command":"sweep-all","rung":"15min","min_hits":500"#,
        ))
        .expect("sweep-all");
        assert_eq!(batch.word(), "sweep-all");
        // A BATCH HAS NO ONE INSTRUMENT, and a blank would read as a field that
        // failed to load rather than as a fact.
        assert_eq!(batch.underlying(), "ALL");
        assert_eq!(batch.window(), ((0, 1), (0, 1)), "year zero is no month");
    }

    #[test]
    fn a_generated_bar_command_is_refused_with_the_reason_not_as_a_typo() {
        // `sweep`, `audit` and `auto` EXIST. Refusing them as unknown words
        // would tell an operator they mistyped a command they typed correctly.
        for word in ["sweep", "audit", "auto"] {
            let why = command_from(&command_body(&format!(r#""command":"{word}""#)))
                .expect_err("a generated-bar command is not served here");
            assert!(
                why.why().contains("GENERATED"),
                "the reason must be the provenance rule, not a spelling \
                 complaint: {}",
                why.why()
            );
            assert!(
                why.why().contains("audit-range"),
                "and it must name the stored equivalents: {}",
                why.why()
            );
        }
    }

    #[test]
    fn an_unknown_command_is_refused_and_lists_what_is_accepted() {
        let why = command_from(&command_body(r#""command":"conquer""#)).expect_err("not a command");
        assert!(why.why().contains("conquer"), "{}", why.why());
        for word in EVERY_COMMAND {
            assert!(why.why().contains(word), "{} omits {word}", why.why());
        }
        assert!(
            command_from(&command_body(r#""rung":"15min""#)).is_err(),
            "no command word"
        );
    }

    #[test]
    fn sweep_stored_refuses_a_span_longer_than_the_one_month_it_walks() {
        // `cli::sweep_stored` takes a YEAR and a MONTH, not a range. Sweeping
        // the opening month and recording it under a request that named eighty
        // would be a shorter answer wearing the request's identity.
        let asked = command_from(&command_body(
            r#""command":"sweep-stored","rung":"15min","min_hits":500"#,
        ))
        .expect("parses");
        let progress = conduct_command(&asked, 1, 2);
        let why = progress.refusal.expect("a multi-month ask must refuse");
        assert!(why.contains("ONE month"), "{why}");
        assert!(progress.report.is_none(), "a refusal is not a report");
    }

    #[test]
    fn a_screen_without_a_support_or_a_ceiling_is_refused() {
        for extra in [
            r#""command":"screen","rung":"15min","max_points":20,"top":25"#,
            r#""command":"screen","rung":"15min","support_ppm":0,"max_points":20,"top":25"#,
            r#""command":"screen","rung":"15min","support_ppm":50000,"max_points":-1,"top":25"#,
            r#""command":"screen","rung":"15min","support_ppm":50000,"max_points":20,"top":0"#,
        ] {
            assert!(
                command_from(&command_body(extra)).is_err(),
                "accepted a screen it cannot run: {extra}"
            );
        }
    }

    #[test]
    fn a_command_needing_a_rung_is_refused_without_one() {
        for word in ["audit-range", "auto-stored", "sweep-stored", "sweep-all"] {
            // `auto-stored` reads no `min_hits`, and naming it is refused
            // (D-1972), so its body omits the field.
            let hits = if word == "auto-stored" {
                ""
            } else {
                r#","min_hits":500"#
            };
            let why = command_from(&command_body(&format!(r#""command":"{word}"{hits}"#)))
                .expect_err("{word} needs a rung");
            assert!(why.why().contains("`rung`"), "{}", why.why());
        }
    }

    #[test]
    fn a_command_needing_a_hit_floor_is_refused_without_one() {
        for word in ["audit-range", "sweep-stored", "sweep-all"] {
            let why = command_from(&command_body(&format!(
                r#""command":"{word}","rung":"15min""#
            )))
            .expect_err("needs min_hits");
            assert!(why.why().contains("min_hits"), "{}", why.why());
        }
        // AND `auto-stored` NEEDS NONE — it searches for the threshold, which
        // is the whole reason it exists.
        assert!(command_from(&command_body(r#""command":"auto-stored","rung":"15min""#)).is_ok());
    }

    #[test]
    fn every_http_hit_floor_refuses_zero_instead_of_running_at_one() {
        for word in ["audit-range", "sweep-stored", "sweep-all"] {
            let extra = format!(r#""command":"{word}","rung":"15min","min_hits":0"#);
            let why = command_from(&command_body(&extra)).expect_err("zero is not honoured");
            assert!(why.why().contains("min_hits"), "{}", why.why());
            assert!(why.why().contains("1 or more"), "{}", why.why());
            assert!(why.why().contains("not silently changed"), "{}", why.why());
        }
    }

    #[test]
    fn a_command_run_is_marked_as_one_on_the_wire() {
        let asked = command_from(&command_body(
            r#""command":"audit-range","rung":"15min","min_hits":500"#,
        ))
        .expect("parses");
        let (from, to) = asked.window();
        let progress = Progress::started(asked.feed(), asked.underlying(), from, to, None, 1, 2)
            .of_kind(Kind::Command);
        assert!(progress.to_json().contains(r#""kind":"command""#));
        assert_eq!(Kind::Command.word(), "command");
    }

    #[test]
    fn every_refusal_variant_carries_its_sentence() {
        // `why` matches on all five; a variant added without an arm would not
        // compile, and one added to the arm without a sentence would return an
        // empty string here.
        for why in [
            Refusal::Malformed("m".to_owned()),
            Refusal::Span("s".to_owned()),
            Refusal::Busy("b".to_owned()),
            Refusal::Unstamped("u".to_owned()),
            Refusal::Unobservable("o".to_owned()),
        ] {
            assert!(!why.why().is_empty(), "{why:?} has no sentence");
        }
    }

    /// A CLI sweep is visible to the console even though this process did not
    /// start it.
    ///
    /// The slot `run_json` reads is one server's `Mutex`, so before this the
    /// page drew an idle console over a machine hours into a grid — a POSITIVE
    /// claim that nothing was running, made by a surface that had not looked.
    ///
    /// All three halves are asserted, and the two negative ones matter as much:
    /// a fallback that reported a sweep whenever the log had EVER been written,
    /// or whenever ANY record existed, is the same defect pointing the other
    /// way.
    #[test]
    fn a_sweep_running_outside_this_process_is_reported_from_the_log() {
        let dir = crate::scratch::path("sweeprun-elsewhere");
        let _ignored = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a scratch directory");

        // AN EMPTY LOG IS NOT A RUNNING SWEEP.
        assert_eq!(
            elsewhere_over(&dir, 1_000_000),
            super::NO_SWEEP,
            "a store nobody has swept in must say so, not guess"
        );

        let sink = telemetry::Sink::open(
            &telemetry::Config::new(&dir).with_min_level(telemetry::Level::Trace),
        )
        .expect("opens");

        // NOR IS A RECORD ON ANOTHER TARGET. Every served page emits one, so
        // without this the console would report itself as a running sweep.
        let _ = sink.emit(&telemetry::Event::info("api.serve", "not a sweep"));
        assert_eq!(
            elsewhere_over(&dir, 1_000_000),
            super::NO_SWEEP,
            "only the sweep target counts, or serving the page reads as a run"
        );

        let _ = sink.emit_for_run(
            42,
            &telemetry::Event::info("cli.lifecycle", "command started")
                .with("phase", "running")
                .with("command", "sweep-stored"),
        );
        let _ = sink.emit_for_run(
            42,
            &telemetry::Event::info(super::CLI_SWEEP_TARGET, r#"exit grid "entered""#),
        );
        let json = elsewhere_over(&dir, i64::MAX);
        assert!(
            json.contains(r#""where":"cli""#),
            "the page must be told the sweep is not this console's: {json}"
        );
        assert!(
            json.contains(r#"\"entered\""#),
            "a message carrying a quote must be escaped, or the body does not \
             parse and the page reads a running sweep as unknown: {json}"
        );
        assert!(
            json.contains(r#""stale_after_millis":900000"#),
            "the window is reported so the page decides, not this: {json}"
        );

        // A CLOCK BEHIND THE STORE'S cannot age the newest event, so it is
        // neither a sweep in the future nor a live one: CE-87 / D-2755. The
        // age is reported signed, and the status names the clock.
        let skewed = elsewhere_over(&dir, 0);
        assert!(
            skewed.contains(r#""age_millis":-"#),
            "a stamp ahead of this clock is reported as a negative age: {skewed}"
        );
        assert!(skewed.contains(r#""status":"unknown""#), "{skewed}");
        assert!(
            skewed.contains("ahead of this server's clock"),
            "the reason names the clock: {skewed}"
        );
        assert!(
            elsewhere_over(&dir, super::now_micros() / 1_000).contains(r#""status":"running""#),
            "the same marker, aged by a clock that is not behind it, is running"
        );
        assert!(
            json.contains(r#""status":"unknown""#),
            "stale is not completed"
        );
    }

    #[test]
    fn external_lifecycle_replaces_a_finished_browser_slot_and_distinguishes_terminal_outcomes() {
        let dir = crate::scratch::path("sweep-external-lifecycle");
        let _ = std::fs::remove_dir_all(&dir);
        let sink = telemetry::Sink::open(&telemetry::Config::new(&dir)).expect("sink");
        let mut local = Progress::started("zerodha", "NIFTY", (2020, 1), (2020, 1), None, 1, 1);
        local.finished_micros = Some(2);
        local.report = Some("old completed browser run".into());
        let emit = |message, phase| {
            assert_eq!(
                sink.emit_for_run(
                    44,
                    &telemetry::Event::info("cli.lifecycle", message)
                        .with("phase", phase)
                        .with("command", "sweep-stored")
                ),
                telemetry::Emitted::Written
            );
        };
        emit("command started", "running");
        let status = super::observed_status(
            Some(&local),
            super::external_observation(Some(&dir), super::now_micros() / 1_000),
        );
        assert!(
            status.contains(r#""attempt":44"#),
            "new CLI attempt replaces finished browser slot: {status}"
        );
        assert!(status.contains(r#""status":"running""#));
        local.finished_micros = Some(i64::MAX);
        let overlapping = super::observed_status(
            Some(&local),
            super::external_observation(Some(&dir), super::now_micros() / 1_000),
        );
        assert!(
            overlapping.contains(r#""status":"running""#),
            "a CLI command that started before browser completion remains active: {overlapping}"
        );
        local.finished_micros = Some(2);
        let _ = sink.emit_for_run(44, &telemetry::Event::info("cli.audit", "rung finished"));
        assert!(
            elsewhere_over(&dir, super::now_micros() / 1_000).contains(r#""status":"running""#),
            "rung completion does not finish its command"
        );
        emit("command finished", "completed");
        let completed = elsewhere_over(&dir, i64::MAX);
        assert!(completed.contains(r#""status":"completed""#));
        assert!(completed.contains(r#""in_flight":false"#));
        emit("command finished", "refused");
        let refused = elsewhere_over(&dir, super::now_micros() / 1_000);
        assert!(refused.contains(r#""status":"refused""#));
        assert!(!refused.contains(r#""refusal":null"#));
        local.finished_micros = Some(i64::MAX);
        assert!(
            super::observed_status(
                Some(&local),
                super::external_observation(Some(&dir), super::now_micros() / 1_000)
            )
            .contains("old completed browser run")
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn attempt_lifecycle_cannot_hide_or_complete_a_command_and_the_marker_window_is_bounded() {
        let dir = crate::scratch::path("sweep-command-attempt-lifecycle");
        let _ = std::fs::remove_dir_all(&dir);
        let sink = telemetry::Sink::open(&telemetry::Config::new(&dir)).expect("sink");
        let started = telemetry::Event::info("cli.lifecycle", "command started")
            .with("phase", "running")
            .with("command", "audit-range");
        assert_eq!(sink.emit_for_run(91, &started), telemetry::Emitted::Written);
        let attempt = telemetry::Event::info("cli.lifecycle", "sweep evidence attempt finished")
            .with("attempt", 92_u64)
            .with("completion", "completed");
        for _ in 0..255 {
            assert_eq!(sink.emit_for_run(0, &attempt), telemetry::Emitted::Written);
        }
        let observed = elsewhere_over(&dir, super::now_micros() / 1_000);
        assert!(observed.contains(r#""status":"running""#), "{observed}");
        assert!(observed.contains(r#""attempt":91"#), "{observed}");
        assert!(!observed.contains(r#""attempt":92"#), "{observed}");
        assert_eq!(sink.emit_for_run(0, &attempt), telemetry::Emitted::Written);
        let capped = elsewhere_over(&dir, super::now_micros() / 1_000);
        assert!(capped.contains(r#""status":"unknown""#), "{capped}");
        let finished = telemetry::Event::info("cli.lifecycle", "command finished")
            .with("phase", "completed")
            .with("command", "audit-range");
        assert_eq!(
            sink.emit_for_run(91, &finished),
            telemetry::Emitted::Written
        );
        assert_eq!(sink.emit_for_run(0, &attempt), telemetry::Emitted::Written);
        let completed = elsewhere_over(&dir, i64::MAX);
        assert!(completed.contains(r#""status":"completed""#), "{completed}");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn inspection_commands_and_incomplete_markers_cannot_become_sweep_completion() {
        let dir = crate::scratch::path("sweep-inspection-command-lifecycle");
        let _ = std::fs::remove_dir_all(&dir);
        let sink = telemetry::Sink::open(&telemetry::Config::new(&dir)).expect("sink");
        let started = telemetry::Event::info("cli.lifecycle", "command started")
            .with("phase", "running")
            .with("command", "sweep-stored");
        assert_eq!(
            sink.emit_for_run(101, &started),
            telemetry::Emitted::Written
        );
        for command in ["top", "results", "research-plan", "verify", "fold-audit"] {
            let inspected = telemetry::Event::info("cli.lifecycle", "command finished")
                .with("phase", "completed")
                .with("command", command);
            assert_eq!(
                sink.emit_for_run(102, &inspected),
                telemetry::Emitted::Written
            );
            let observed = elsewhere_over(&dir, super::now_micros() / 1_000);
            assert!(
                observed.contains(r#""status":"running""#),
                "{command}: {observed}"
            );
            assert!(
                observed.contains(r#""attempt":101"#),
                "{command}: {observed}"
            );
        }
        let malformed =
            telemetry::Event::info("cli.lifecycle", "command finished").with("phase", "completed");
        assert_eq!(
            sink.emit_for_run(103, &malformed),
            telemetry::Emitted::Written
        );
        assert!(
            elsewhere_over(&dir, super::now_micros() / 1_000).contains(r#""status":"unknown""#)
        );
        let mut tail = super::status_tail(&dir, "cli.lifecycle", None, 1);
        assert!(super::tail_fault(&tail).is_none());
        tail.records.first_mut().expect("latest fixture record").cut = true;
        assert!(super::tail_fault(&tail).is_some());
        let latest = tail.records.first_mut().expect("latest fixture record");
        latest.cut = false;
        latest.dropped_fields = 1;
        assert!(super::tail_fault(&tail).is_some());
        let _ = std::fs::remove_dir_all(dir);
    }

    /// conc9-2: a partial last line seen once is re-read, and only one still
    /// there on the second read counts as damage.
    #[test]
    fn a_line_being_written_is_reread_and_a_torn_one_still_refuses() {
        let partial = telemetry::Tail {
            partial_tail: true,
            ..telemetry::Tail::default()
        };
        let whole = telemetry::Tail::default();
        let reads = std::cell::Cell::new(0_u8);
        let settled = super::settled_tail(|| {
            reads.set(reads.get() + 1);
            if reads.get() == 1 {
                partial.clone()
            } else {
                whole.clone()
            }
        });
        assert_eq!(reads.get(), 2, "a partial tail is read once more");
        assert!(
            super::tail_fault(&settled).is_none(),
            "the finished write is no fault"
        );

        reads.set(0);
        let torn = super::settled_tail(|| {
            reads.set(reads.get() + 1);
            partial.clone()
        });
        assert_eq!(reads.get(), 2);
        assert!(
            super::tail_fault(&torn).is_some(),
            "a torn tail still refuses"
        );

        reads.set(0);
        let clean = super::settled_tail(|| {
            reads.set(reads.get() + 1);
            whole.clone()
        });
        assert_eq!(reads.get(), 1, "a whole tail is read once");
        assert!(super::tail_fault(&clean).is_none());
    }

    /// **A sweep marker inside the scanned window is the answer, however large
    /// the log behind it.** W1-api6-4, D-0954.
    ///
    /// The lifecycle walk asks for 256 records under a 4 MiB scan cap, and a
    /// healthy log whose newest 4 MiB held fewer than 256 lifecycle records
    /// hit the cap every time. The cap was read as damage, so every browser
    /// run, descent and command refused and the status said `unknown`, even
    /// with the newest sweep's own terminal marker in hand. Driven over 5 MiB
    /// of other targets' records: a newest `command finished` is `completed`
    /// and clears launch; a newest `command started` with no activity is
    /// `running` (the activity walk's own cap is answered by the marker) and
    /// blocks launch; then the boundaries that must stay refused: a marker
    /// pushed past the window by 5 MiB of newer records is `unknown` and blocks
    /// launch, and a malformed line inside the window still blocks launch
    /// with the marker found.
    #[test]
    fn a_marker_inside_the_scan_window_answers_however_large_the_log_behind_it() {
        let dir = crate::scratch::path("sweep-external-scan-cap");
        let _ = std::fs::remove_dir_all(&dir);
        let sink = telemetry::Sink::open(&telemetry::Config::new(&dir)).expect("sink");
        let pad = "P".repeat(500);
        let fill = |bytes: u64| {
            let mut written = 0_u64;
            while written < bytes {
                let mut event = telemetry::Event::info("api.serve", "unrelated traffic");
                for key in ["a", "b", "c", "d", "e", "f", "g", "h"] {
                    event = event.with(key, pad.as_str());
                }
                assert_eq!(sink.emit(&event), telemetry::Emitted::Written);
                written += 8 * 500;
            }
        };
        let marker = |message, phase| {
            let event = telemetry::Event::info("cli.lifecycle", message)
                .with("phase", phase)
                .with("command", "sweep-stored");
            assert_eq!(sink.emit_for_run(61, &event), telemetry::Emitted::Written);
        };
        let over_cap = crate::logs::SCAN_BYTES + (1 << 20);
        fill(over_cap);
        marker("command finished", "completed");
        let tail = super::status_tail(&dir, "cli.lifecycle", None, 256);
        assert!(tail.hit_scan_cap, "premise: the walk reaches the cap");
        assert_eq!(
            tail.records.len(),
            1,
            "premise: one lifecycle record in the window"
        );
        let done = super::observe_elsewhere(&dir, i64::MAX);
        assert!(
            done.body.contains(r#""status":"completed""#),
            "{}",
            done.body
        );
        assert!(
            done.launch_clear && !done.uncertain && !done.in_flight,
            "{}",
            done.body
        );

        marker("command started", "running");
        let running = super::observe_elsewhere(&dir, super::now_micros() / 1_000);
        assert!(
            running.body.contains(r#""status":"running""#),
            "{}",
            running.body
        );
        assert!(running.body.contains(r#""attempt":61"#), "{}", running.body);
        assert!(
            running.in_flight && !running.launch_clear,
            "{}",
            running.body
        );

        // PAST THE WINDOW: newer traffic pushes the marker out of the newest
        // 4 MiB, and no marker found means the cap still answers `unknown`.
        fill(over_cap);
        let lost = super::observe_elsewhere(&dir, super::now_micros() / 1_000);
        assert!(lost.body.contains(r#""status":"unknown""#), "{}", lost.body);
        assert!(lost.body.contains("scan cap true"), "{}", lost.body);
        assert!(lost.uncertain && !lost.launch_clear, "{}", lost.body);

        // OTHER DAMAGE STILL COUNTS WITH THE MARKER FOUND: a malformed line
        // could be a newer marker this reader cannot decode.
        marker("command finished", "completed");
        assert!(super::observe_elsewhere(&dir, i64::MAX).launch_clear);
        std::io::Write::write_all(
            &mut std::fs::OpenOptions::new()
                .append(true)
                .open(telemetry::current_path(&dir))
                .expect("the current log"),
            b"not a record\n",
        )
        .expect("one malformed line");
        let damaged = super::observe_elsewhere(&dir, i64::MAX);
        assert!(
            damaged.body.contains(r#""status":"unknown""#),
            "{}",
            damaged.body
        );
        assert!(
            damaged.body.contains("1 malformed records"),
            "{}",
            damaged.body
        );
        assert!(!damaged.launch_clear, "{}", damaged.body);
        drop(sink);
        let _ = std::fs::remove_dir_all(dir);
    }

    /// CE-11, D-1914: a line torn by a killed CLI command, OLDER than the
    /// newest sweep marker, no longer blocks launch; the same damage NEWER
    /// than the marker still does.
    #[test]
    fn damage_older_than_the_newest_marker_does_not_block_launch() {
        let dir = crate::scratch::path("sweep-external-old-tear");
        let _ = std::fs::remove_dir_all(&dir);
        let sink = telemetry::Sink::open(&telemetry::Config::new(&dir)).expect("sink");
        let marker = |message, phase| {
            let event = telemetry::Event::info("cli.lifecycle", message)
                .with("phase", phase)
                .with("command", "sweep-stored");
            assert_eq!(sink.emit_for_run(62, &event), telemetry::Emitted::Written);
        };
        let tear = || {
            std::io::Write::write_all(
                &mut std::fs::OpenOptions::new()
                    .append(true)
                    .open(telemetry::current_path(&dir))
                    .expect("the current log"),
                b"{\"torn\n",
            )
            .expect("one malformed line");
        };
        marker("command started", "running");
        tear();
        marker("command started", "running");
        marker("command finished", "completed");
        let premise = super::status_tail(&dir, "cli.lifecycle", None, 256);
        assert_eq!(
            premise.malformed, 1,
            "premise: the torn line is in the window"
        );
        let clear = super::observe_elsewhere(&dir, i64::MAX);
        assert!(
            clear.body.contains(r#""status":"completed""#),
            "{}",
            clear.body
        );
        assert!(clear.launch_clear && !clear.uncertain, "{}", clear.body);
        tear();
        let newer = super::observe_elsewhere(&dir, i64::MAX);
        assert!(newer.body.contains("1 malformed records"), "{}", newer.body);
        assert!(!newer.launch_clear, "{}", newer.body);
        drop(sink);
        let _ = std::fs::remove_dir_all(dir);
    }

    /// R1286-api-01, D-4130: a lifecycle record that lands BETWEEN the window
    /// read and the trimmed re-read moves every newest-first index by one, so
    /// the trimmed walk no longer ends on the marker. The whole window is kept,
    /// torn line and all, and `found` still names the marker in it. Taking the
    /// trimmed walk instead would hand the caller a clean window whose `found`
    /// index names the newcomer, and a launch cleared on a marker it never read.
    #[test]
    fn a_record_landing_between_the_two_reads_keeps_the_whole_window() {
        let dir = crate::scratch::path("sweep-external-landed-between");
        let _ = std::fs::remove_dir_all(&dir);
        let sink = telemetry::Sink::open(&telemetry::Config::new(&dir)).expect("sink");
        let marker = |message, phase, run| {
            let event = telemetry::Event::info("cli.lifecycle", message)
                .with("phase", phase)
                .with("command", "sweep-stored");
            assert_eq!(sink.emit_for_run(run, &event), telemetry::Emitted::Written);
        };
        marker("command started", "running", 63);
        std::io::Write::write_all(
            &mut std::fs::OpenOptions::new()
                .append(true)
                .open(telemetry::current_path(&dir))
                .expect("the current log"),
            b"{\"torn\n",
        )
        .expect("one malformed line");
        marker("command started", "running", 64);
        marker("command finished", "completed", 64);

        // PREMISE: with nothing landing in between, the walk is trimmed to end
        // at the marker, so the torn line behind it is out of the window.
        let (quiet, found) = super::newest_sweep_marker(&dir);
        assert_eq!(found, Some(0), "the newest record is the marker");
        assert_eq!(quiet.records.len(), 1, "trimmed to the marker");
        assert_eq!(quiet.malformed, 0, "the torn line is behind the marker");
        let newest = quiet.records.first().expect("the marker");
        assert_eq!(newest.message, "command finished");
        let finished = newest.seq;

        // THE RACE: a newer sweep starts between the two reads.
        let mut reads = Vec::new();
        let (raced, found) = super::marker_window(|limit| {
            if !reads.is_empty() {
                marker("command started", "running", 65);
            }
            reads.push(limit);
            super::status_tail(&dir, "cli.lifecycle", None, limit)
        });
        assert_eq!(reads, vec![256, 1], "the window, then the trimmed re-read");
        assert_eq!(found, Some(0));
        assert_eq!(raced.records.len(), 3, "the whole first window is kept");
        assert_eq!(raced.malformed, 1, "with its torn line");
        assert_eq!(
            raced.records.first().map(|record| record.seq),
            Some(finished),
            "`found` still names the marker the walk found"
        );
        assert!(
            super::tail_fault_unless_answered(&raced, true).is_some(),
            "the damage the trimmed walk would have hidden still refuses"
        );

        // The landed record is real: a fresh look reads the newer sweep, not
        // the finished one, and does not clear a launch over it.
        let now = super::observe_elsewhere(&dir, i64::MAX);
        assert!(now.body.contains(r#""run":65"#), "{}", now.body);
        assert!(
            !now.body.contains(r#""status":"completed""#),
            "{}",
            now.body
        );
        assert!(!now.launch_clear, "{}", now.body);
        drop(sink);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn unreadable_partial_capped_and_legacy_external_evidence_never_becomes_idle_or_done() {
        for tail in [
            telemetry::Tail {
                errors: vec!["permission denied".to_owned()],
                ..telemetry::Tail::default()
            },
            telemetry::Tail {
                malformed: 1,
                ..telemetry::Tail::default()
            },
            telemetry::Tail {
                partial_tail: true,
                ..telemetry::Tail::default()
            },
            telemetry::Tail {
                hit_scan_cap: true,
                ..telemetry::Tail::default()
            },
        ] {
            assert!(super::tail_fault(&tail).is_some());
        }
        let dir = crate::scratch::path("sweep-external-legacy");
        let _ = std::fs::remove_dir_all(&dir);
        let sink = telemetry::Sink::open(&telemetry::Config::new(&dir)).expect("sink");
        let _ = sink.emit_for_run(45, &telemetry::Event::info("cli.audit", "rung finished"));
        assert!(
            elsewhere_over(&dir, super::now_micros() / 1_000).contains(r#""status":"unknown""#)
        );
        assert!(
            super::observed_status(None, super::external_observation(None, 0))
                .contains(r#""status":"unknown""#)
        );
        std::fs::remove_file(telemetry::current_path(&dir)).expect("remove fixture log");
        std::fs::create_dir(telemetry::current_path(&dir)).expect("unreadable log fixture");
        assert!(
            elsewhere_over(&dir, super::now_micros() / 1_000).contains(r#""status":"unknown""#)
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    /// conc13-3, D-2595. On the old code a sweep refused at its execution
    /// lease answered 409/503 with no event at all: `/logs` held only the
    /// middleware's `served` line. The lease is held here (and the CLI's own
    /// store differs from this scratch store too), so the press is refused at
    /// `claim_execution` whichever check fires, and one `api.sweep` Warn
    /// carrying the same sentence as the body must land. No run can start.
    #[test]
    fn a_lease_refusal_is_logged_with_its_reason() {
        const STAMP: &str = "0123456789abcdef0123456789abcdef01234567";
        let _sink = crate::emitted::sink();
        let site = finisher_site("conc13-3-lease");
        std::fs::create_dir_all(&site.store_root).expect("private store");
        let _held = cli::execution_lease::Lease::acquire(&site.store_root)
            .expect("the test owns this private store's lease");
        let run = format!(r#"{{"feed":"zerodha","underlying":"NIFTY",{SPAN},"rungs":["5min"]}}"#);
        let from = crate::emitted::mark();
        let (status, _, body) = super::run_with(&site, &run, Some(STAMP));
        assert_ne!(status, axum::http::StatusCode::ACCEPTED, "{body}");
        assert!(body.contains(r#""accepted":false"#), "{body}");
        assert!(site.sweep.lock().expect("slot").is_none(), "no run started");
        // Other tests in this binary may press concurrently, so the event is
        // found by its sentence, which is this press's own body.
        let said = crate::emitted::landed(
            from,
            "api.sweep",
            "a sweep was refused at its execution lease",
        );
        let mut ours = 0;
        for record in &said {
            let why = record
                .field("why")
                .and_then(telemetry::OwnedValue::as_str)
                .unwrap_or("");
            if !why.is_empty() && body.contains(&crate::render::json_string(why)) {
                ours += 1;
                assert_eq!(record.level, telemetry::Level::Warn, "{record:?}");
            }
        }
        assert!(
            ours >= 1,
            "the refusal is logged with its reason: {said:?} / {body}"
        );
        // And the helper itself, for every refusal class: one Warn, its
        // sentence, and the same answer `refused` gives.
        for why in [
            Refusal::Malformed("m".to_owned()),
            Refusal::Span("s".to_owned()),
            Refusal::Busy("b".to_owned()),
            Refusal::Unstamped("u".to_owned()),
            Refusal::Unobservable("o".to_owned()),
            Refusal::Environment("e".to_owned()),
        ] {
            let from = crate::emitted::mark();
            let logged = super::refuse_logged("conc13-3 helper probe", &why);
            assert_eq!(logged, super::refused(&why));
            let said = crate::emitted::landed(from, "api.sweep", "conc13-3 helper probe");
            let mut matched = 0;
            for record in &said {
                if crate::emitted::says(record, "why", why.why()) {
                    matched += 1;
                    assert_eq!(record.level, telemetry::Level::Warn, "{why:?}");
                }
            }
            assert_eq!(matched, 1, "{why:?}: {said:?}");
        }
        let _ = std::fs::remove_dir_all(&site.store_root);
    }
}

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
        let _ = write!(out, r#""kind":"{}""#, self.kind.word());
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
            | Self::Unobservable(ref s) => s,
        }
    }

    /// The status this refusal answers with.
    ///
    /// `Busy` is 409, `Unstamped` is 503 and the rest are 400. A second press
    /// is a CONFLICT with work already happening, not a malformed request, and
    /// answering it 400 would tell the operator to fix a body that is perfectly
    /// good. An unstamped build is neither: the body is fine and nothing is in
    /// flight — this SERVER cannot record a run in the state it was built in,
    /// and 503 is the code that says the fix is on this side.
    #[must_use]
    pub const fn status(&self) -> axum::http::StatusCode {
        match *self {
            Self::Busy(_) => axum::http::StatusCode::CONFLICT,
            Self::Unstamped(_) | Self::Unobservable(_) => {
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
const KNOBS: [(&str, &str); 16] = [
    ("support_ppm", "BRUTEX_SUPPORT_PPM"),
    ("ceiling", "BRUTEX_CEILING"),
    ("screen_cap", "BRUTEX_SCREEN_CAP"),
    ("screen_budget_ms", "BRUTEX_SCREEN_BUDGET_MS"),
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
    screen_budget_ms: WireField<WireScalar>,
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
            "screen_budget_ms" => self.screen_budget_ms.as_ref(),
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
    /// The stop ceiling, in whole index points.
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
    asked_from_wire(&body)
}

/// [`asked_from`] after the strict JSON boundary has been crossed once.
fn asked_from_wire(body: &WireBody) -> Result<Asked, Refusal> {
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

    let month_ok = |m: u64| (1..=12).contains(&m);
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
/// a stop ceiling below one point, and a listing bound of zero.
pub fn descent_from(body: &str) -> Result<AskedDescent, Refusal> {
    let body = wire_body(body)?;
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

    // A CEILING IS A DISTANCE, SO IT IS PARSED AS ONE AND REFUSED AT ZERO.
    // `cli` converts it against the midpoint of the span's own bars; this side
    // never sees a ppm, which is the whole reason that entry point exists.
    let max_points = field(body, "max_points")
        .and_then(|text| text.parse::<i64>().ok())
        .ok_or_else(|| {
            Refusal::Malformed(
                "`max_points` must be a whole number of index points — the \
                 widest adverse excursion this run may accept."
                    .to_owned(),
            )
        })?;
    if max_points <= 0 {
        return Err(Refusal::Malformed(format!(
            "`max_points` is {max_points}. A ceiling of zero admits no trade \
             and a negative one is not a distance."
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
/// is dropped and an in-flight slot becomes a visible refusal.
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
}

impl TaskFinisher {
    /// Arms one guard for the already-claimed slot.
    fn new(site: crate::server::Loaded) -> Self {
        Self { site, armed: true }
    }

    /// Installs a normal result and prevents `Drop` from painting over it.
    fn finish(mut self, done: Progress) {
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
        progress.refusal = Some(ABNORMAL_END.to_owned());
    }
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
        progress.refusal = Some(text);
    } else {
        progress.report = Some(text);
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
pub(crate) fn emit_completion(progress: &Progress, operation: &str, elapsed_micros: u64) {
    let audit = completion_audit(progress);
    let _outcome = telemetry::emit_for_run(
        progress.attempt,
        &telemetry::Event::new(audit.level, "api.sweep", "an engine task finished")
            .with("attempt", progress.attempt)
            .with("operation", operation)
            .with("feed", progress.feed.as_str())
            .with("underlying", progress.underlying.as_str())
            .with("outcome", audit.outcome)
            .with("why", audit.why)
            .with("elapsed_micros", elapsed_micros),
    );
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
/// `Drop` runs during unwinding, so this holds on every exit: normal return,
/// early return, and panic.
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

/// Reserves one exact attempt token.
///
/// The installed sink owns the durable sequence space, so a restart resumes
/// above every attempt still retained in the log. There is deliberately no
/// process-local fallback: an attempt with no log cannot satisfy the exact
/// marker contract the live monitor requires.
fn reserve_attempt() -> Result<u64, Refusal> {
    let sink = telemetry::global().ok_or_else(|| {
        Refusal::Unobservable(
            "no telemetry sink is installed, so no exact auditable attempt can be reserved. \
             No engine work was started."
                .to_owned(),
        )
    })?;
    sink.reserve_run_id().ok_or_else(|| {
        Refusal::Unobservable(
            "the exact telemetry attempt id space is exhausted, so this run cannot be \
             separated from earlier log records. No engine work was started."
                .to_owned(),
        )
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
///
/// **UNVERIFIED as a measurement.** The bound is argued from the
/// shape of the code and no bench in this workspace times it.
/// `CLAUDE.md` §3 rule 6: a structural argument is not a
/// measurement, however sound it is.
pub async fn run(
    axum::extract::State(site): axum::extract::State<crate::server::Loaded>,
    body: String,
) -> (axum::http::StatusCode, JsonHeaders, String) {
    run_with(&site, &body, cli::commit_stamp())
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
        let _ = telemetry::emit_if!(
            telemetry::Level::Warn,
            "api.sweep",
            "a sweep was refused because this build carries no commit stamp",
            "why" => telemetry::Value::Str(why.why()),
        );
        return refused(&why);
    }

    // THE SLOT IS CLAIMED UNDER THE LOCK AND THE WORK STARTS OUTSIDE IT.
    // Holding a std mutex across an await is the deadlock this pattern exists
    // to avoid, so the guard is dropped before anything is spawned.
    let (started, attempt) = {
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
            return refused(&Refusal::Busy(why));
        }
        let attempt = match reserve_attempt() {
            Ok(attempt) => attempt,
            Err(why) => return refused(&why),
        };
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
            return refused(&why);
        }
        *held = Some(accepted.clone());
        (started, attempt)
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
    let guard = TaskFinisher::new(std::sync::Arc::clone(site));
    tokio::task::spawn_blocking(move || {
        let finished = conduct(&asked, started, attempt);
        let elapsed = now_micros().saturating_sub(started);
        emit_completion(&finished, "sweep", elapsed.max(0).unsigned_abs());
        let mut done = finished;
        done.finished_micros = Some(now_micros());
        guard.finish(done);
    });

    (
        axum::http::StatusCode::ACCEPTED,
        json_headers(),
        r#"{"accepted":true,"refusal":null}"#.to_owned(),
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
    descend_with(&site, &body, cli::commit_stamp())
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

    if let Some(why) = stamp_refusal(stamp) {
        return refused(&why);
    }

    let (started, attempt) = {
        let mut held = match site.sweep.lock() {
            Ok(held) => held,
            Err(poisoned) => poisoned.into_inner(),
        };
        if held.as_ref().is_some_and(Progress::in_flight) {
            let why = "a sweep or descent is already running in this process. \
                       Both append to the same append-only ledger, and two of \
                       them finishing together can interleave two records, so \
                       the second press is refused rather than queued."
                .to_owned();
            return refused(&Refusal::Busy(why));
        }
        let attempt = match reserve_attempt() {
            Ok(attempt) => attempt,
            Err(why) => return refused(&why),
        };
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
            return refused(&why);
        }
        *held = Some(accepted.clone());
        (started, attempt)
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

    let guard = TaskFinisher::new(std::sync::Arc::clone(site));
    tokio::task::spawn_blocking(move || {
        let finished = conduct_descent(&asked, started, attempt);
        let elapsed = now_micros().saturating_sub(started);
        emit_completion(&finished, "descent", elapsed.max(0).unsigned_abs());
        let mut done = finished;
        done.finished_micros = Some(now_micros());
        guard.finish(done);
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
///
/// **UNVERIFIED as a measurement.** The bound is argued from the
/// shape of the code and no bench in this workspace times it.
/// `CLAUDE.md` §3 rule 6: a structural argument is not a
/// measurement, however sound it is.
pub async fn run_json(
    axum::extract::State(site): axum::extract::State<crate::server::Loaded>,
) -> (axum::http::StatusCode, JsonHeaders, String) {
    let held = match site.sweep.lock() {
        Ok(held) => held,
        Err(poisoned) => poisoned.into_inner(),
    };
    let body = held.as_ref().map_or_else(
        // NO SWEEP IN THIS PROCESS IS NOT NO SWEEP. The slot above is one
        // server's `Mutex`, so it knows only about runs the browser itself
        // started -- and this module's own header says so: *"the slot below
        // does not prevent that -- it is one server's slot"*.
        //
        // The operator runs long sweeps from the CLI, which cannot reach this
        // memory. The page then read `"running":null` and drew an idle console
        // over a machine at 1,300% CPU four hours into a grid. That is the
        // failure wearing a success's clothes `CLAUDE.md` §4 bans: not a
        // missing feature, but a POSITIVE statement that nothing is running,
        // made by a surface that had not looked.
        //
        // The store is the shared thing, so the fallback reads it.
        elsewhere_json,
        |progress| format!(r#"{{"running":{}}}"#, progress.to_json()),
    );
    (axum::http::StatusCode::OK, json_headers(), body)
}

/// The answer when nothing is running anywhere this process can see.
const NO_SWEEP: &str = r#"{"running":null,"why":"no sweep has been started from this console"}"#;

/// The target `cli` stamps on the records a sweep emits as it advances.
///
/// `cli.audit` and not `cli.sweep`: the sweep verbs render through `audit_bars`,
/// which is where both the per-rung bracket and the ten-per-rung grid progress
/// records are emitted. `cli.sweep` covers the synthetic verbs, which never run
/// here.
const CLI_SWEEP_TARGET: &str = "cli.audit";

/// How long a silence may run before the page should doubt the sweep.
///
/// Reported TO the page rather than applied here, because this cannot tell a
/// stopped sweep from a stretch that emits nothing, and deciding which would be
/// inventing the distinction. Fifteen minutes is longer than the widest measured
/// gap between records on a live `range-all` — the one-minute rung went twenty
/// minutes silent BEFORE grid progress existed, and ten records per rung is what
/// closed it.
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
fn elsewhere_json() -> String {
    crate::logs::cli_log_dir().map_or_else(
        || NO_SWEEP.to_owned(),
        |dir| elsewhere_over(&dir, now_micros() / 1_000),
    )
}

/// What [`elsewhere_json`] says, with the directory and the clock handed in.
///
/// Split out because the outer function reads `BRUTEX_STORE` through a process
/// global and stamps the wall clock, and a test that has to set both can only
/// run alone. This takes them, so the behaviour is testable without a global and
/// the age arithmetic is checkable against a fixed `now`.
fn elsewhere_over(dir: &std::path::Path, now_millis: i64) -> String {
    let tail = telemetry::tail(
        dir,
        telemetry::DEFAULT_KEEP_FILES,
        &telemetry::Query {
            target: Some(CLI_SWEEP_TARGET.to_owned()),
            ..telemetry::Query::last(1)
        },
    );
    let Some(last) = tail.records.last() else {
        return NO_SWEEP.to_owned();
    };
    // Clamped at zero: a store written by a machine whose clock is ahead must
    // not report a sweep in the future.
    let age = now_millis.saturating_sub(last.at_unix_millis).max(0);
    format!(
        r#"{{"running":{{"where":"cli","run":{},"message":{},"at_unix_millis":{},"age_millis":{},"stale_after_millis":{}}}}}"#,
        last.run,
        crate::logs::quoted(&last.message),
        last.at_unix_millis,
        age,
        STALE_AFTER_MILLIS,
    )
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
    /// One rung, full validated audit — walk-forward, PBO and the bootstrap.
    AuditRange {
        /// Feed word, instrument and rung.
        span: AskedRung,
        /// Absolute hit floor for this rung.
        min_hits: u64,
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
            Self::AuditRange { .. } => "audit-range",
            Self::Screen { .. } => "screen",
            Self::AutoStored { .. } => "auto-stored",
            Self::SweepStored { .. } => "sweep-stored",
            Self::SweepAll { .. } => "sweep-all",
        }
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
            | Self::Screen { ref span, .. }
            | Self::AutoStored { ref span }
            | Self::SweepStored { ref span, .. } => &span.underlying,
            Self::SweepAll { .. } => "ALL",
        }
    }

    /// The feed word.
    #[must_use]
    pub fn feed(&self) -> &str {
        match *self {
            Self::AuditRange { ref span, .. }
            | Self::Screen { ref span, .. }
            | Self::AutoStored { ref span }
            | Self::SweepStored { ref span, .. } => &span.feed,
            Self::SweepAll { ref feed, .. } => feed,
        }
    }

    /// The span, or the whole store for a batch.
    #[must_use]
    pub fn window(&self) -> ((u16, u8), (u16, u8)) {
        match *self {
            Self::AuditRange { ref span, .. }
            | Self::Screen { ref span, .. }
            | Self::AutoStored { ref span }
            | Self::SweepStored { ref span, .. } => (span.from, span.to),
            // A batch is not a span, and (0,1)..(0,1) is a value no real month
            // can take -- year zero -- so the page cannot render it as one.
            Self::SweepAll { .. } => ((0, 1), (0, 1)),
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
const EVERY_COMMAND: [&str; 5] = [
    "audit-range",
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
    let body = wire_body(body)?;
    command_from_wire(&body)
}

/// [`command_from`] after strict JSON decoding and duplicate detection.
fn command_from_wire(body: &WireBody) -> Result<Command, Refusal> {
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

    match word.as_str() {
        "audit-range" => Ok(Command::AuditRange {
            span: rung_from(body)?,
            min_hits: positive_min_hits(body)?,
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
            if max_points <= 0 || top == 0 {
                return Err(Refusal::Malformed(
                    "`max_points` must be 1 index point or more and `top` must \
                     be 1 row or more."
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
            let asked = asked_from_wire(body)?;
            let rung = field(body, "rung")
                .filter(|s| EVERY_RUNG.contains(&s.as_str()))
                .ok_or_else(|| {
                    Refusal::Malformed(format!(
                        "`rung` is required and must be one of: {}.",
                        EVERY_RUNG.join(", ")
                    ))
                })?;
            Ok(Command::SweepAll {
                feed: asked.feed,
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
    command_with(&site, &body, cli::commit_stamp())
}

/// [`command`], with the build's commit stamp passed in.
///
/// Split for the reason [`run_with`] gives, and see [`stamp_refusal`] for why/// that reason survived `build.rs` changing which arm is the reachable one.
pub(crate) fn command_with(
    site: &crate::server::Loaded,
    body: &str,
    stamp: Option<&str>,
) -> (axum::http::StatusCode, JsonHeaders, String) {
    let asked = match command_from(body) {
        Ok(asked) => asked,
        Err(why) => {
            let _ = telemetry::emit_if!(
                telemetry::Level::Warn,
                "api.sweep",
                "an engine command was refused before it started",
                "why" => telemetry::Value::Str(why.why()),
            );
            return refused(&why);
        }
    };

    if let Some(why) = stamp_refusal(stamp) {
        return refused(&why);
    }

    let (started, attempt) = {
        let mut held = match site.sweep.lock() {
            Ok(held) => held,
            Err(poisoned) => poisoned.into_inner(),
        };
        if held.as_ref().is_some_and(Progress::in_flight) {
            return refused(&Refusal::Busy(
                "a sweep, descent or command is already running in this \
                 process. All of them append to the same append-only ledger, \
                 and two finishing together can interleave two records, so the \
                 second press is refused rather than queued."
                    .to_owned(),
            ));
        }
        let (from, to) = asked.window();
        let attempt = match reserve_attempt() {
            Ok(attempt) => attempt,
            Err(why) => return refused(&why),
        };
        let started = now_micros();
        let accepted = Progress::started(
            asked.feed(),
            asked.underlying(),
            from,
            to,
            None,
            started,
            attempt,
        )
        .of_kind(Kind::Command);
        if let Some(why) = marker_refusal(emit_attempt_started(&accepted)) {
            return refused(&why);
        }
        *held = Some(accepted.clone());
        (started, attempt)
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

    let guard = TaskFinisher::new(std::sync::Arc::clone(site));
    tokio::task::spawn_blocking(move || {
        let finished = conduct_command(&asked, started, attempt);
        let elapsed = now_micros().saturating_sub(started);
        emit_completion(&finished, asked.word(), elapsed.max(0).unsigned_abs());
        let mut done = finished;
        done.finished_micros = Some(now_micros());
        guard.finish(done);
    });

    (
        axum::http::StatusCode::ACCEPTED,
        json_headers(),
        r#"{"accepted":true,"refusal":null}"#.to_owned(),
    )
}

/// `GET /engine/top.json` — the ranked frontier, as `cli top` prints it.
///
/// **A READ, so it takes no slot and no commit gate.** It records nothing, so
/// §3 rule 3's identity requirement does not bind and an unstamped build can
/// serve it honestly. `feed` and `underlying` are optional and filter together:
/// `cli top` takes both or neither, and this keeps that shape rather than
/// inventing a third case the CLI has no answer for.
pub async fn top_json(uri: axum::http::Uri) -> (axum::http::StatusCode, JsonHeaders, String) {
    let query = uri.query().unwrap_or_default();
    let param = |name: &str| -> Option<String> {
        query.split('&').find_map(|pair| {
            pair.split_once('=')
                .filter(|(key, _)| *key == name)
                .map(|(_, value)| value.to_owned())
        })
    };
    let feed = param("feed");
    let underlying = param("underlying");
    let text = match (feed.as_deref(), underlying.as_deref()) {
        (Some(f), Some(u)) => cli::top_list(Some(f), Some(u)),
        (None, None) => cli::top_list(None, None),
        _ => {
            return (
                axum::http::StatusCode::BAD_REQUEST,
                json_headers(),
                format!(
                    r#"{{"report":null,"refusal":{}}}"#,
                    crate::render::json_string(
                        "`feed` and `underlying` filter together: give both or \
                         neither. `cli top` has no answer for one alone, and \
                         inventing one here would make this page disagree with \
                         the terminal about the same file."
                    )
                ),
            );
        }
    };
    let refused_it = text.starts_with(REFUSED);
    let body = if refused_it {
        format!(
            r#"{{"report":null,"refusal":{}}}"#,
            crate::render::json_string(&text)
        )
    } else {
        format!(
            r#"{{"report":{},"refusal":null}}"#,
            crate::render::json_string(&text)
        )
    };
    let status = if refused_it {
        axum::http::StatusCode::BAD_REQUEST
    } else {
        axum::http::StatusCode::OK
    };
    (status, json_headers(), body)
}

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
        conduct_command, descent_from, marker_refusal, now_micros, settle, stamp_refusal,
    };

    fn finisher_site(name: &str) -> crate::server::Loaded {
        let masters = crate::scratch::path(&format!("sweep-finisher-masters-{name}"));
        let store = crate::scratch::path(&format!("sweep-finisher-store-{name}"));
        std::sync::Arc::new(crate::server::Site::load(&masters, &store))
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
        done.report = Some("STORED_PROVENANCE\ncomplete".to_owned());
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
        done.refusal = Some("the engine gave its own refusal".to_owned());
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
            .split_once("#[cfg(test)]")
            .expect("this module has one test boundary")
            .0;
        let armed = ["let guard = Task", "Finisher::new"].concat();
        let disarmed = ["guard.", "finish(done);"].concat();
        assert_eq!(production.matches(&armed).count(), 3, "one guard per task");
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
        let mut p = Progress::started("zerodha", "NIFTY", (2020, 1), (2020, 1), Some(50_000), 1, 2);
        p.refusal = Some("the store held no bars".to_owned());
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

    /// The refusal `cli::range_all` returns when nothing was read.
    ///
    /// Taken in SHAPE from that function's own `format!` rather than invented:
    /// it opens with the word, names the rung count, and says outright that no
    /// row was recorded.
    const REFUSAL: &str = "refused: every one of the 9 rungs refused. Nothing was \
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
        let progress = settled(REFUSAL);
        assert_eq!(progress.refusal.as_deref(), Some(REFUSAL));
        assert!(
            progress.report.is_none(),
            "a refusal filed as a report is the green `Sweep finished` over an \
             empty ledger that this split exists to prevent: {:?}",
            progress.report
        );
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
        both.refusal = Some(REFUSAL.to_owned());
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
        let json = settled(REFUSAL).to_json();
        assert!(
            json.contains(r#""report":null"#),
            "a refusal must not also arrive as a report: {json}"
        );
        assert!(
            json.contains(r#""refusal":"refused: every one of the 9 rungs"#),
            "this is the field the page reads to decide it failed: {json}"
        );
        assert!(json.contains(r#""in_flight":false"#), "{json}");
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
    fn a_stop_ceiling_of_zero_or_less_is_refused_rather_than_swept_with() {
        for points in ["0", "-20"] {
            let why = descent_from(&descent_body(&format!(
                r#""rung":"15min","max_points":{points},"top":25"#
            )))
            .expect_err("a ceiling of zero admits no trade");
            assert!(why.why().contains("max_points"), "{}", why.why());
        }
        // AND A MISSING ONE IS NOT ZERO. Defaulting it would pick the
        // operator's risk for them, silently.
        assert!(descent_from(&descent_body(r#""rung":"15min","top":25"#)).is_err());
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

    fn command_body(extra: &str) -> String {
        format!(r#"{{"feed":"zerodha","underlying":"NIFTY",{SPAN},{extra}}}"#)
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
            r#""command":"screen","rung":"15min","support_ppm":50000,"max_points":0,"top":25"#,
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
            let why = command_from(&command_body(&format!(
                r#""command":"{word}","min_hits":500"#
            )))
            .expect_err("{word} needs a rung");
            assert!(why.why().contains("rung"), "{}", why.why());
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
            super::elsewhere_over(&dir, 1_000_000),
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
            super::elsewhere_over(&dir, 1_000_000),
            super::NO_SWEEP,
            "only the sweep target counts, or serving the page reads as a run"
        );

        let _ = sink.emit(&telemetry::Event::info(
            super::CLI_SWEEP_TARGET,
            r#"exit grid "entered""#,
        ));
        let json = super::elsewhere_over(&dir, i64::MAX);
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

        // A CLOCK BEHIND THE STORE'S must not read as a sweep in the future.
        let skewed = super::elsewhere_over(&dir, 0);
        assert!(
            skewed.contains(r#""age_millis":0"#),
            "age is clamped at zero under clock skew: {skewed}"
        );
    }
}

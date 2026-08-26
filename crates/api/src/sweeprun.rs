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
}

impl Kind {
    /// The word this kind travels to the page as.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Sweep => "sweep",
            Self::Descent => "descent",
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
    /// A run is already in flight in this process.
    Busy(String),
    /// This binary carries no commit stamp, so no run may be recorded.
    ///
    /// Not the caller's fault and not a bad body — the SERVER cannot do the
    /// work in the state it was built in, which is what 503 says and 400 does
    /// not.
    Unstamped(String),
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
            | Self::Unstamped(ref s) => s,
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
            Self::Unstamped(_) => axum::http::StatusCode::SERVICE_UNAVAILABLE,
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
/// `cargo test` is an unstamped build — `crates/runner`'s own doc records that
/// this is why `run_ranked` shipped with zero coverage. A guard that called
/// `commit_stamp()` directly would therefore have one arm the suite can never
/// reach, which is the 100% floor §9 sets. Taking the value makes both arms
/// ordinary.
fn stamp_refusal(stamp: Option<&str>) -> Option<Refusal> {
    if stamp.is_some() {
        return None;
    }
    Some(Refusal::Unstamped(
        "this server was built without BRUTEX_COMMIT, so no sweep it runs can \
         be recorded: CLAUDE.md §3 rule 3 makes the commit part of every run's \
         identity, and it is read at COMPILE time so it cannot be filled in \
         now. Nothing was swept, because the answer did not depend on any bar. \
         Rebuild and restart the server with \
         `BRUTEX_COMMIT=$(git rev-parse HEAD) cargo run --release -p api -- serve`."
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

/// The support threshold, in parts per million of a rung's own bars.
///
/// # It is not a request field, and that is the whole point
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
const SUPPORT_PPM: u64 = 200_000;

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

    Ok(Asked {
        feed,
        underlying,
        from: (from_y, month(from_month)),
        to: (to_y, month(to_month)),
    })
}

/// The eight rungs a descent may walk.
///
/// Held here rather than read from `cli`, because `cli::EVERY_RUNG` is private
/// and making it public to save eight strings would widen that crate's surface
/// for one caller. `cli::elite_descend_in_points` validates the rung again and
/// refuses by name, so this list being stale would produce a refusal naming the
/// eight it accepts — a wrong sentence, never a wrong sweep. The test below
/// pins it against a refusal from `cli` itself.
const EVERY_RUNG: [&str; 8] = [
    "1min", "2min", "3min", "5min", "10min", "15min", "60min", "1day",
];

/// The descent request body, or the first thing wrong with it.
///
/// # Errors
///
/// Everything [`asked_from`] refuses, plus a rung that is not one of the eight,
/// a stop ceiling below one point, and a listing bound of zero.
pub fn descent_from(body: &str) -> Result<AskedDescent, Refusal> {
    // THE SPAN AND FEED RULES ARE NOT RESTATED, THEY ARE REUSED. Two parsers
    // for one span is two places for a month bound to drift, and the overflow
    // this one already survives -- `{"from_year":18446744073709551615}` calling
    // `abort()` in a release build -- is exactly the kind that comes back when
    // a second copy is written from memory.
    let asked = asked_from(body)?;

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
#[must_use]
pub fn conduct(asked: &Asked, now_micros: i64) -> Progress {
    let mut progress = Progress::started(
        &asked.feed,
        &asked.underlying,
        asked.from,
        asked.to,
        SUPPORT_PPM,
        now_micros,
    );
    let text = cli::range_all(
        &asked.feed,
        &asked.underlying,
        asked.from,
        asked.to,
        SUPPORT_PPM,
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
pub fn conduct_descent(asked: &AskedDescent, now_micros: i64) -> Progress {
    let mut progress = Progress::started(
        &asked.feed,
        &asked.underlying,
        asked.from,
        asked.to,
        SUPPORT_PPM,
        now_micros,
    )
    .of_kind(Kind::Descent);
    // POINTS, NEVER PPM. `cli::elite_descend_in_points` converts against the
    // midpoint of the span's own bars; a ppm computed on this side would be a
    // second answer to a question only the bars can settle, and its own doc
    // records what that has already cost.
    let text = cli::elite_descend_in_points(
        &asked.feed,
        &asked.underlying,
        &asked.rung,
        asked.from,
        asked.to,
        asked.max_points,
        asked.top,
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
            return refused(&Refusal::Busy(why));
        }
        *held = Some(Progress::started(
            &asked.feed,
            &asked.underlying,
            asked.from,
            asked.to,
            SUPPORT_PPM,
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
        "support_ppm" => telemetry::Value::Uint(SUPPORT_PPM),
    );

    // A BLOCKING THREAD, NOT A WORKER. `cli::range_all` is CPU-bound over
    // millions of bars; on a worker it would hold that thread for the whole
    // sweep and every other request sharing it would wait.
    let held_site = std::sync::Arc::clone(site);
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
/// `cargo test` is an unstamped build, so a handler reading the stamp itself
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

    {
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
        *held = Some(
            Progress::started(
                &asked.feed,
                &asked.underlying,
                asked.from,
                asked.to,
                SUPPORT_PPM,
                now_micros(),
            )
            .of_kind(Kind::Descent),
        );
    }

    let _ = telemetry::emit_if!(
        telemetry::Level::Info,
        "api.sweep",
        "a descent was accepted from the browser",
        "feed" => telemetry::Value::Str(&asked.feed),
        "underlying" => telemetry::Value::Str(&asked.underlying),
        "rung" => telemetry::Value::Str(&asked.rung),
        "max_points" => telemetry::Value::Int(asked.max_points),
    );

    let held_site = std::sync::Arc::clone(site);
    let started = now_micros();
    tokio::task::spawn_blocking(move || {
        let finished = conduct_descent(&asked, started);
        let mut slot = match held_site.sweep.lock() {
            Ok(slot) => slot,
            Err(poisoned) => poisoned.into_inner(),
        };
        let elapsed = now_micros().saturating_sub(started);
        let _ = telemetry::emit_if!(
            telemetry::Level::Info,
            "api.sweep",
            "a descent finished and its record is in the ledger",
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
    use super::{
        Asked, AskedDescent, EVERY_RUNG, Kind, Progress, Refusal, SUPPORT_PPM, asked_from,
        descent_from, field, now_micros, settle, stamp_refusal,
    };

    fn body(feed: &str, span: &str) -> String {
        format!(r#"{{"feed":"{feed}","underlying":"NIFTY",{span}}}"#)
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
        assert_eq!(asked.to, (2020, 3));
        // The body above still carries a spaced `"support_ppm" : 50000`, and
        // it is read past like any other field this route does not want. The
        // run is swept at `SUPPORT_PPM` regardless.
        assert_ne!(SUPPORT_PPM, 50_000);
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
    fn a_support_field_in_the_body_is_ignored_rather_than_obeyed() {
        // THE OLD PAGE IS STILL OUT THERE, and so is anyone's curl. A body
        // that still carries `support_ppm` must not fail -- it names a field
        // this route no longer has, which is not the same as being malformed.
        //
        // It must ALSO not take effect. A request that could still set the
        // threshold would mean the parameter had merely been hidden from the
        // form rather than removed, and a hidden settable parameter is worse
        // than a visible one: nothing on the page would show it was in play.
        let with =
            format!(r#"{{"feed":"zerodha","underlying":"NIFTY",{SPAN},"support_ppm":999999}}"#);
        let without = asked_from(&body("zerodha", SPAN)).expect("a good body");
        assert_eq!(
            asked_from(&with).expect("a body with a stale field is still good"),
            without,
            "the extra field must change nothing about what was asked"
        );
    }

    #[test]
    fn the_threshold_is_a_percentage_so_every_rung_gets_its_own_count() {
        // The constant is a RATIO, and that is what keeps nine rungs
        // comparable. Held as an absolute count instead, a 1min rung and a
        // 1day rung would be asked to clear the same number of hits from
        // twenty times different bar counts, and the daily rung would find
        // nothing for a reason that has nothing to do with the market.
        //
        // These are this store's own bar counts for the recorded span.
        let hits = |bars: u64| bars * SUPPORT_PPM / 1_000_000;
        assert_eq!(hits(21_620), 4_324, "1min");
        assert_eq!(hits(11_643), 2_328, "15min");
        assert_eq!(hits(1_671), 334, "1day");
        // Three rungs, three thresholds, one ratio, and nobody typed any of
        // the three.
        assert_ne!(hits(21_620), hits(1_671));
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
        const { assert!(SUPPORT_PPM > 0, "extinction needs a threshold above zero") };
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
        let mut progress =
            Progress::started("zerodha", "NIFTY", (2019, 12), (2026, 8), SUPPORT_PPM, 1);
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
    fn a_stamped_build_does_not_refuse() {
        assert!(stamp_refusal(Some("3b8f5af")).is_none());
        // AND AN EMPTY STAMP IS STILL A STAMP. `option_env!` yields `Some("")`
        // for `BRUTEX_COMMIT=`, which is a build someone stamped with nothing.
        // Refusing it here would be this route inventing a rule `cli` does not
        // have — `cli::commit_stamp` is the authority and it tests presence.
        assert!(stamp_refusal(Some("")).is_none());
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
                       "rung":"1day","max_points":20,"top":25}"#;
        assert!(
            descent_from(body).is_err(),
            "the span bound must still bite"
        );

        // And a backwards span, which is the sweep's other span refusal.
        let backwards = r#"{"feed":"zerodha","underlying":"NIFTY","from_year":2026,
                            "from_month":8,"to_year":2019,"to_month":12,
                            "rung":"1day","max_points":20,"top":25}"#;
        assert!(descent_from(backwards).is_err());
    }

    #[test]
    fn a_descent_without_a_rung_is_refused_and_names_the_eight() {
        let why = descent_from(&descent_body(r#""max_points":20,"top":25"#))
            .expect_err("a descent walks ONE rung, so it must be told which");

        assert!(why.why().contains("rung"), "{}", why.why());
        for rung in ["1min", "15min", "1day"] {
            assert!(
                why.why().contains(rung),
                "the operator cannot pick from a list they are not shown: {}",
                why.why()
            );
        }
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
                r#""rung":"1day","max_points":{points},"top":25"#
            )))
            .expect_err("a ceiling of zero admits no trade");
            assert!(why.why().contains("max_points"), "{}", why.why());
        }
        // AND A MISSING ONE IS NOT ZERO. Defaulting it would pick the
        // operator's risk for them, silently.
        assert!(descent_from(&descent_body(r#""rung":"1day","top":25"#)).is_err());
    }

    #[test]
    fn a_listing_bound_of_zero_is_refused() {
        let why = descent_from(&descent_body(r#""rung":"1day","max_points":20,"top":0"#))
            .expect_err("zero rows is no answer");
        assert!(why.why().contains("top"), "{}", why.why());
        assert!(descent_from(&descent_body(r#""rung":"1day","max_points":20"#)).is_err());
    }

    #[test]
    fn the_rung_list_here_agrees_with_the_one_cli_refuses_by() {
        // THIS LIST IS A COPY AND COPIES DRIFT. `cli` holds the authority and
        // its refusal names the eight it accepts, so asking it about a rung it
        // cannot sweep gives the real list to compare against -- and it refuses
        // BEFORE opening any span, so this costs no bars.
        let refusal = cli::elite_descend_in_points(
            "zerodha",
            "NIFTY",
            "no-such-rung",
            (2026, 8),
            (2026, 8),
            20,
            25,
        );
        for rung in EVERY_RUNG {
            assert!(
                refusal.contains(rung),
                "`{rung}` is offered here and `cli` does not list it: {refusal}"
            );
        }
    }

    #[test]
    fn a_descent_and_a_sweep_are_told_apart_on_the_wire() {
        // `support_ppm` MEANS DIFFERENT THINGS IN THE TWO, so the page cannot
        // read it correctly without being told which it is looking at.
        let sweep = Progress::started("zerodha", "NIFTY", (2019, 12), (2026, 8), SUPPORT_PPM, 1);
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

    #[test]
    fn every_refusal_variant_carries_its_sentence() {
        // `why` matches on all four; a variant added without an arm would not
        // compile, and one added to the arm without a sentence would return an
        // empty string here.
        for why in [
            Refusal::Malformed("m".to_owned()),
            Refusal::Span("s".to_owned()),
            Refusal::Busy("b".to_owned()),
            Refusal::Unstamped("u".to_owned()),
        ] {
            assert!(!why.why().is_empty(), "{why:?} has no sentence");
        }
    }
}

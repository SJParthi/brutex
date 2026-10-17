//! `GET /spot.json` — the underlying's stored close at an option bar's own
//! instant, so `/db` can name moneyness, intrinsic and extrinsic (D-2807,
//! D-3328).
//!
//! # What it answers, and what it never answers
//!
//! For each asked instant `t` (UTC seconds, an option bar's own stamp) it reads
//! the UNDERLYING's bar file on the same feed, at the same rung, in the same
//! month file. The underlying's segment and store key come from the sourced
//! alias table, `brutex_core::alias::option_underlying` (D-3332): an index
//! option (`NIFTY`, `FINNIFTY`, `NIFTYIT`, `NIFTYFPI`, ...) reads
//! `NSE/INDEX/<store key>` -- `NIFTYFINSERVICE` for FINNIFTY -- and every other
//! option underlying is a share, read under `NSE/CASH/<key>`, the newest symbol
//! its dated changes reach (`TMPV` for TATAMOTORS). The body names the key it
//! read. Until D-3332 the index set was the five names of
//! `brutex_core::universe::FNO_INDEX_UNDERLYINGS`, so NIFTYIT and NIFTYFPI were
//! read as shares and FINNIFTY under a key no feed files (DB-G25, DB-G54).
//!
//! **Two ways to take a spot, and the default is the strict one** (D-3328):
//!
//! * `asof=exact` (the default, D-2807): the close of the bar stamped at
//!   exactly `t`, or a named absence (`no_spot_bar`, `no_spot_file`,
//!   `spot_unreadable`, `spot_order_broken`).
//! * `asof=prev` (opted in): the close of the underlying's LAST bar at or
//!   before `t` within `t`'s own IST session (its IST trade day), with the
//!   stamp it came from (`spot_t`) and its age (`spot_age_s`, `t - spot_t`).
//!   No such bar in that session answers `no_spot_in_session`, and an
//!   instant off the rung's grid -- no option bar's own stamp at that rung --
//!   answers `t_off_grid`, because the bar before it may close after it. A
//!   daily rung holds one bar per session, closing at the session's close, so
//!   there `prev` answers exactly what `exact` does.
//!
//! Every instant carries `spot_t` and `spot_age_s` (0 for an exact hit, `null`
//! with no close), and the body says which rule answered (`asof`), so an
//! earlier close can never pass for one stamped at `t`.
//!
//! **Never a later bar, never another month.** No look-ahead (`CLAUDE.md` §3
//! rule 7): the close returned is that of a bar that opens at or before `t`,
//! complete no later than the option bar at `t` is. The lookup may READ the
//! record after `t` to LOCATE `t`, and returns none of its values. An instant
//! whose IST month is not the asked `month` is a wrong question, answered
//! `t_outside_month` before any record is read: until D-3328 it was answered
//! `no_spot_bar`, as if the file were held and empty at `t` (DB-039).
//!
//! # Cost, stated rather than claimed
//!
//! The file is opened once per request. Each instant is one
//! [`store::file::BarFile::first_at_or_after`] lookup -- O(1) through the
//! month's `.tix` time index when it has one (D-2329), and otherwise a
//! bisection of at most `ceil(log2(n_valid + 1))` record reads, which the
//! store names per handle (`BarFile::time_lookup`) -- plus at most two record
//! reads to confirm the hit or bracket the miss. `asof=prev` reads nothing
//! more: the bar before `t` is the record the bracket already read.
//! Instants per request are capped at [`MAX_SPOT_TIMES`]. The work runs on
//! the blocking pool behind `crate::detail::run_store_read`'s admission
//! (DB-066, DB-G80): a full pool is a named `429` with `Retry-After`, never a
//! stalled async worker. The wall-clock time is measured by WP-32's report
//! and gate 8 (`benches/ratio/spot.rs`); no figure is restated here.

use super::{Loaded, ingest, param, render, timeframe_param};
use crate::bars::OpenRefusalKind;
use std::fmt::Write as _;
use store::file::BarFile;
use store::path::{IST_OFFSET_SECS, PathParts, Timeframe, YearMonth};

/// The most instants one request may ask for.
///
/// Bounded by the request target, not by taste: an instant is ten digits and
/// a comma, and [`super::MAX_REQUEST_TARGET_BYTES`] is 8 KiB, so 400 leaves
/// room for the other fields. `/db` asks for its page in chunks of at most
/// this many.
pub const MAX_SPOT_TIMES: usize = 400;

/// Microseconds in a second.
const MICROS: i64 = 1_000_000;

/// Seconds in a day.
const SECONDS_PER_DAY: i64 = 86_400;

/// Instants answered between two looks at a request's cancellation flag
/// (DB-N15, D-3349).
pub const CANCEL_EVERY: usize = 64;

/// The store segment an option underlying's own price series lives under.
///
/// An index underlying has no cash equity and its spot is the index; every
/// other option underlying is a share and its spot is its cash series
/// (`CLAUDE.md` §1, D-0682). Which is which is the sourced alias table's
/// answer (`brutex_core::alias::option_underlying`, D-3332), not five names.
/// Not a fallback: the CASH answer is the definition of a stock option's
/// underlying, and the response names the path it read, so a store that holds
/// no such series says `no_spot_file` against that path.
#[must_use]
pub fn underlying_segment(symbol: &str) -> &'static str {
    brutex_core::alias::option_underlying(symbol)
        .segment
        .as_str()
}

/// The store key an option underlying's own price series is filed under:
/// `NIFTYFINSERVICE` for FINNIFTY, `TMPV` for TATAMOTORS, the symbol itself
/// where the table names no other (D-3332).
#[must_use]
pub fn underlying_key(symbol: &str) -> &str {
    brutex_core::alias::option_underlying(symbol).key
}

/// Which bar an instant's spot is taken from. D-3328.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AsOf {
    /// Only the bar stamped exactly at `t` (D-2807). The default.
    Exact,
    /// The underlying's last bar at or before `t` within `t`'s IST session,
    /// opted in, always shown with its stamp and age (D-3328). Never a later
    /// bar, never another session or month.
    Prev,
}

impl AsOf {
    /// The `asof` field of `query`: absent or `exact` is [`Self::Exact`],
    /// `prev` is [`Self::Prev`].
    ///
    /// # Errors
    ///
    /// Any other word, by name. Not read as the default: a misspelt opt-in
    /// silently answered exact would hide every earlier close it asked for.
    pub fn parse(query: &str) -> Result<Self, String> {
        match param(query, "asof").as_str() {
            "" | "exact" => Ok(Self::Exact),
            "prev" => Ok(Self::Prev),
            other => Err(format!(
                "{other:?} is not an asof: exact (the default, the bar stamped at t, \
                 D-2807) or prev (the last bar at or before t in its IST session, D-3328)"
            )),
        }
    }

    /// The wire word.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::Prev => "prev",
        }
    }
}

/// One instant's answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Spot {
    /// The underlying's close, in paisa, of the bar stamped `ts_micros`:
    /// `t` itself, or under [`AsOf::Prev`] the last bar of `t`'s IST
    /// session at or before it.
    At {
        /// The close, paisa.
        close: i64,
        /// The stamp of the bar it came from, microseconds.
        ts_micros: i64,
    },
    /// The file is held and has no bar at `t`, which lies inside its month:
    /// the records either side bracket it, which is what makes the absence a
    /// fact and not a guess.
    NoBar,
    /// Under [`AsOf::Prev`]: the file is held, and no bar of `t`'s IST
    /// session lies at or before `t` (the session had not printed yet, or
    /// the bar before `t` belongs to an earlier day).
    NoBarInSession,
    /// `t`'s IST month is not the month asked: a wrong question, answered
    /// before any record is read (D-3328, DB-039).
    OutsideMonth,
    /// Under [`AsOf::Prev`], `t` is not a stamp the rung's grid holds, so it
    /// is no option bar's own stamp at that rung, and the bar before it may
    /// close after it: refused rather than answered with a later close.
    OffGrid,
    /// The underlying's month file is not held.
    NoFile,
    /// A record needed to answer would not read; the reason is in `faults`.
    Unreadable,
    /// The records around `t` are not strictly increasing (the zero-filled
    /// extent `store::file` documents), so neither a hit nor an absence can be
    /// stated without a scan this route does not do.
    OrderBroken,
}

impl Spot {
    /// The close, in paisa, when one was found.
    #[must_use]
    pub const fn close(&self) -> Option<i64> {
        match self {
            Self::At { close, .. } => Some(*close),
            _ => None,
        }
    }

    /// The stamp, microseconds, of the bar the close came from.
    #[must_use]
    pub const fn stamp_micros(&self) -> Option<i64> {
        match self {
            Self::At { ts_micros, .. } => Some(*ts_micros),
            _ => None,
        }
    }

    /// The wire's reason word, `None` for a close.
    #[must_use]
    pub const fn why(&self) -> Option<&'static str> {
        match self {
            Self::At { .. } => None,
            Self::NoBar => Some("no_spot_bar"),
            Self::NoBarInSession => Some("no_spot_in_session"),
            Self::OutsideMonth => Some("t_outside_month"),
            Self::OffGrid => Some("t_off_grid"),
            Self::NoFile => Some("no_spot_file"),
            Self::Unreadable => Some("spot_unreadable"),
            Self::OrderBroken => Some("spot_order_broken"),
        }
    }
}

/// Whether the instant `ts_micros` lies in `month`'s IST span,
/// `[00:00 IST on the 1st, 00:00 IST on the next 1st)` -- the span the store
/// files a bar of that month in (`store::file`'s admission).
#[must_use]
pub const fn in_month(month: YearMonth, ts_micros: i64) -> bool {
    let (from, until) = month.ist_bounds_micros();
    ts_micros >= from && ts_micros < until
}

/// The IST day number (days since 1970-01-01 IST) of an instant.
#[must_use]
pub const fn ist_day(ts_micros: i64) -> i64 {
    (ts_micros.div_euclid(MICROS) + IST_OFFSET_SECS).div_euclid(SECONDS_PER_DAY)
}

/// The IST month an instant lies in, or `None` for an instant no calendar
/// here can place.
#[must_use]
pub fn month_of(ts_micros: i64) -> Option<YearMonth> {
    pull::session::IstMoment::from_epoch_secs(ts_micros.div_euclid(MICROS))
        .ok()
        .and_then(|at| at.day().year_month().ok())
}

/// Whether `ts_micros` is a stamp a rung of `timeframe_secs` holds: the
/// store's own admission grid (`store::file`, D-0915) -- an intraday rung's
/// bars open on the 09:15 IST-anchored grid of its width; a daily rung
/// admits any whole second.
#[must_use]
pub fn on_grid(timeframe_secs: u32, ts_micros: i64) -> bool {
    if timeframe_secs >= 86_400 || timeframe_secs == 0 {
        return ts_micros.rem_euclid(MICROS) == 0;
    }
    let anchor =
        (IST_OFFSET_SECS - i64::from(Timeframe::OPEN_MINUTES_PAST_IST_MIDNIGHT) * 60) * MICROS;
    ts_micros
        .saturating_add(anchor)
        .rem_euclid(i64::from(timeframe_secs) * MICROS)
        == 0
}

/// The underlying's close at exactly `ts_micros` (D-2807), read from
/// `file`, whose path names `month`.
///
/// # Errors
///
/// The store's refusal for a record the answer needed and could not read.
pub fn close_at(
    file: &BarFile,
    month: YearMonth,
    ts_micros: i64,
) -> Result<Spot, store::file::StoreError> {
    spot_at(file, month, ts_micros, AsOf::Exact)
}

/// One instant's spot under `asof`, read from `file`, whose path names
/// `month`. See the module header.
///
/// An instant outside `month` is [`Spot::OutsideMonth`] and reads nothing,
/// so [`Spot::NoBar`] and [`Spot::NoBarInSession`] are only ever answered for
/// an instant inside the file's own span (DB-039).
///
/// # Errors
///
/// The store's refusal for a record the answer needed and could not read.
pub fn spot_at(
    file: &BarFile,
    month: YearMonth,
    ts_micros: i64,
    asof: AsOf,
) -> Result<Spot, store::file::StoreError> {
    if !in_month(month, ts_micros) {
        return Ok(Spot::OutsideMonth);
    }
    // NO LATER CLOSE THROUGH AN OFF-GRID QUESTION: the bar before an
    // off-grid `t` can be the bar that CONTAINS `t`, which closes after it.
    // On the grid, a bar stamped before `t` has closed by `t`, and one
    // stamped at `t` closes with the option bar at `t` (D-3328).
    let width = file.header().timeframe_secs;
    if asof == AsOf::Prev && !on_grid(width, ts_micros) {
        return Ok(Spot::OffGrid);
    }
    // A DAILY RUNG HOLDS ONE BAR PER SESSION, closing at the session's
    // close: a bar of `t`'s session stamped before `t` is that same close,
    // not an earlier price, so `prev` answers only what `exact` does.
    let asof = if width >= 86_400 { AsOf::Exact } else { asof };
    let at = file.first_at_or_after(ts_micros)?;
    let next = if at < file.header().n_valid {
        let bar = file.read_record(at)?;
        if bar.ts_micros == ts_micros {
            return Ok(Spot::At {
                close: bar.close,
                ts_micros,
            });
        }
        Some(bar.ts_micros)
    } else {
        None
    };
    let previous = match at.checked_sub(1) {
        Some(index) => Some(file.read_record(index)?),
        None => None,
    };
    let answered = verdict(previous.as_ref().map(|bar| bar.ts_micros), next, ts_micros);
    if answered != Spot::NoBar {
        return Ok(answered);
    }
    Ok(match (asof, previous) {
        (AsOf::Exact, _) => Spot::NoBar,
        // THE BRACKET ABOVE PROVED `previous < t < next` over strictly
        // increasing stamps, so `previous` IS the last bar before `t`. It is
        // taken only from `t`'s own IST day: a bar of an earlier session is
        // yesterday's close, which this route never offers as today's.
        (AsOf::Prev, Some(bar)) if ist_day(bar.ts_micros) == ist_day(ts_micros) => Spot::At {
            close: bar.close,
            ts_micros: bar.ts_micros,
        },
        (AsOf::Prev, _) => Spot::NoBarInSession,
    })
}

/// What a miss at `ts_micros` means, from the stamps either side of where the
/// lookup landed.
///
/// THE ABSENCE IS BRACKETED, NOT ASSUMED: `previous < ts < next`, either side
/// allowed to be the file's edge. A real bar is stamped after 1970, so a zero
/// stamp is the zero-filled extent `store::file` documents, where the
/// lookup's premise (strictly increasing stamps) does not hold — and then
/// neither a hit nor an absence is stated. The caller has already placed
/// `ts_micros` inside the file's month ([`spot_at`]), so a file edge here is
/// the month's own edge and not a different file's question.
#[must_use]
pub fn verdict(previous: Option<i64>, next: Option<i64>, ts_micros: i64) -> Spot {
    let before = previous.is_none_or(|ts| ts > 0 && ts < ts_micros);
    let after = next.is_none_or(|ts| ts > ts_micros);
    if before && after {
        Spot::NoBar
    } else {
        Spot::OrderBroken
    }
}

/// The three JSON fields every instant carries for its spot: the close
/// under the key `close_key`, `spot_t` and `spot_age_s`, for the instant
/// `ts_micros` and the spot read there (its close and its bar's stamp), or
/// three `null`s when none was read.
#[must_use]
pub fn spot_fields(close_key: &str, read: Option<(i64, i64)>, ts_micros: i64) -> String {
    match read {
        Some((close, stamp)) => format!(
            r#""{close_key}":{close},"spot_t":{},"spot_age_s":{}"#,
            stamp.div_euclid(MICROS),
            (ts_micros - stamp).div_euclid(MICROS)
        ),
        None => format!(r#""{close_key}":null,"spot_t":null,"spot_age_s":null"#),
    }
}

impl Spot {
    /// The close and its bar's stamp, when a close was found.
    #[must_use]
    pub const fn read(&self) -> Option<(i64, i64)> {
        match self {
            Self::At { close, ts_micros } => Some((*close, *ts_micros)),
            _ => None,
        }
    }
}

/// Reads the comma-separated UTC seconds of `t`, at most `cap` of them, as
/// microseconds.
///
/// # Errors
///
/// An absent `t`, a word that is not a second, or more than `cap`, by name.
pub fn times_param(query: &str, field: &str, cap: usize) -> Result<Vec<i64>, String> {
    let raw = param(query, field);
    if raw.is_empty() {
        return Err(format!(
            "{field} is required: one or more UTC seconds, comma separated"
        ));
    }
    let mut times = Vec::new();
    for word in raw.split(',') {
        if times.len() == cap {
            return Err(format!(
                "more than {cap} instants in {field}; ask in chunks"
            ));
        }
        let micros = word
            .parse::<i64>()
            .ok()
            .and_then(|secs| secs.checked_mul(MICROS))
            .ok_or_else(|| format!("{word:?} in {field} is not a UTC second"))?;
        times.push(micros);
    }
    Ok(times)
}

/// Everything `/spot.json` reads off the query string. Every field refuses
/// rather than defaults, but `asof`, whose default is D-2807's exact rule:
/// the rung in particular has NO default here, because a spot read at a rung
/// the option was not read at is a different instant's close.
#[derive(Debug)]
pub struct SpotAsk {
    vendor: brutex_core::vendor::Vendor,
    exchange: String,
    symbol: String,
    timeframe: Timeframe,
    month: YearMonth,
    asof: AsOf,
    times: Vec<i64>,
}

impl SpotAsk {
    /// Reads one, or names the first thing wrong with it.
    ///
    /// # Errors
    ///
    /// The field that is missing or malformed, by name.
    pub fn parse(query: &str) -> Result<Self, String> {
        let Some(vendor) = ingest::parse_vendor(&param(query, "feed")) else {
            return Err(format!(
                "{:?} is not a feed this build can read",
                param(query, "feed")
            ));
        };
        let exchange = param(query, "exchange");
        let symbol = param(query, "symbol");
        if exchange.is_empty() || symbol.is_empty() {
            return Err("exchange and symbol (the option's underlying) are both required".into());
        }
        if param(query, "timeframe").is_empty() {
            return Err(
                "timeframe is required: the spot is read at the option bar's own rung".into(),
            );
        }
        let timeframe = timeframe_param(query)?;
        let month = month_param(query)?;
        let asof = AsOf::parse(query)?;
        let times = times_param(query, "t", MAX_SPOT_TIMES)?;
        Ok(Self {
            vendor,
            exchange,
            symbol,
            timeframe,
            month,
            asof,
            times,
        })
    }

    /// Answers every instant against the underlying's one month file, and
    /// the faults met, named.
    ///
    /// # Errors
    ///
    /// The store path the question names is not a valid address — a
    /// malformed question, refused before any file is touched.
    pub fn answer(&self, store_root: &std::path::Path) -> Result<(Vec<Spot>, Vec<String>), String> {
        self.answer_with(store_root, &crate::bars::Cancel::never())
    }

    /// [`Self::answer`], looking at `cancel` every [`CANCEL_EVERY`] instants
    /// and stopping by name once the client went away (DB-N15, D-3349).
    ///
    /// # Errors
    ///
    /// What [`Self::answer`] refuses, and `cancelled`.
    pub fn answer_with(
        &self,
        store_root: &std::path::Path,
        cancel: &crate::bars::Cancel,
    ) -> Result<(Vec<Spot>, Vec<String>), String> {
        let opened = crate::bars::open_classified(
            store_root,
            PathParts {
                vendor: self.vendor,
                exchange: &self.exchange,
                segment: underlying_segment(&self.symbol),
                symbol: underlying_key(&self.symbol),
                contract: None,
                timeframe: self.timeframe,
                month: self.month,
                file: store::path::FileKind::Bars,
            },
        );
        // A WRONG MONTH IS A WRONG QUESTION WHETHER OR NOT THE FILE IS HELD.
        let outside_or = |absent: Spot| {
            self.times
                .iter()
                .map(|&ts| {
                    if in_month(self.month, ts) {
                        absent.clone()
                    } else {
                        Spot::OutsideMonth
                    }
                })
                .collect::<Vec<_>>()
        };
        let file = match opened {
            Ok(file) => file,
            Err(refusal) => {
                return match refusal.kind {
                    OpenRefusalKind::Invalid => Err(refusal.message),
                    OpenRefusalKind::Absent => Ok((outside_or(Spot::NoFile), Vec::new())),
                    OpenRefusalKind::Unreadable => {
                        Ok((outside_or(Spot::Unreadable), vec![refusal.message]))
                    }
                };
            }
        };
        let mut faults = Vec::new();
        let mut spots = Vec::with_capacity(self.times.len());
        for (at, &ts) in self.times.iter().enumerate() {
            if at % CANCEL_EVERY == 0 {
                cancel.check()?;
            }
            spots.push(
                spot_at(&file, self.month, ts, self.asof).unwrap_or_else(|why| {
                    faults.push(format!(
                        "{} t={}: {why}",
                        file.path().display(),
                        ts.div_euclid(MICROS)
                    ));
                    Spot::Unreadable
                }),
            );
        }
        Ok((spots, faults))
    }

    /// The response body for `spots` and `faults`.
    #[must_use]
    pub fn render(&self, spots: &[Spot], faults: &[String]) -> String {
        let mut rows = String::with_capacity(spots.len() * 80 + 2);
        rows.push('[');
        for (n, (&ts, spot)) in self.times.iter().zip(spots).enumerate() {
            if n > 0 {
                rows.push(',');
            }
            let why = spot
                .why()
                .map_or_else(|| "null".to_owned(), render::json_string);
            let _ = write!(
                rows,
                r#"{{"t":{},{},"why":{why}}}"#,
                ts.div_euclid(MICROS),
                spot_fields("close", spot.read(), ts),
            );
        }
        rows.push(']');
        format!(
            r#"{{"exchange":{},"segment":{},"symbol":{},"key":{},"timeframe":{},"month":{},"asof":{},"faults":{},"spots":{rows}}}"#,
            render::json_string(&self.exchange),
            render::json_string(underlying_segment(&self.symbol)),
            render::json_string(&self.symbol),
            render::json_string(underlying_key(&self.symbol)),
            render::json_string(self.timeframe.as_str()),
            render::json_string(&self.month.to_string()),
            render::json_string(self.asof.as_str()),
            if faults.is_empty() {
                "null".to_owned()
            } else {
                render::json_string(&faults.join("; "))
            }
        )
    }
}

/// The `month` field, `YYYY-MM`.
///
/// # Errors
///
/// Anything else, by name.
///
/// The server's one month parser, not a third copy of it: this one split on
/// `-` and took `2024-3`, `2024-003` and a `+` sign, so `/spot.json` and
/// `/greeks.json` answered months the sentence says they refuse (app-7, AF-04).
pub fn month_param(query: &str) -> Result<YearMonth, String> {
    super::month_param(query, "month")
}

/// What the three option analytics routes answer: status, headers, body.
pub type Answered = (axum::http::StatusCode, axum::http::HeaderMap, String);

/// The JSON content type, as a header map.
#[must_use]
pub fn json_headers() -> axum::http::HeaderMap {
    let mut headers = axum::http::HeaderMap::new();
    headers.insert(
        axum::http::header::CONTENT_TYPE,
        axum::http::HeaderValue::from_static("application/json; charset=utf-8"),
    );
    headers
}

/// `400` with the reason in an `error` key.
#[must_use]
pub fn bad_request(why: &str) -> Answered {
    (
        axum::http::StatusCode::BAD_REQUEST,
        json_headers(),
        format!(r#"{{"error":{}}}"#, render::json_string(why)),
    )
}

/// What a route answers when its store read was not admitted to the
/// blocking pool: `429` with `Retry-After: 1` when every slot is taken, `503`
/// when the task could not be joined, the reason named either way (DB-066).
#[must_use]
pub fn not_admitted(what: &str, why: &crate::detail::RunError) -> Answered {
    let (status, body) =
        crate::detail::admission_refused(what, crate::detail::MAX_STORE_READ_CONCURRENT, why);
    let mut headers = json_headers();
    if matches!(why, crate::detail::RunError::Saturated) {
        headers.insert(
            axum::http::header::RETRY_AFTER,
            axum::http::HeaderValue::from_static("1"),
        );
    }
    (status, headers, body)
}

/// The handler's body, off the async workers.
#[must_use]
pub fn respond(store_root: &std::path::Path, query: &str) -> Answered {
    respond_with(store_root, query, &crate::bars::Cancel::never())
}

/// [`respond`], stopping once `cancel` is set (DB-N15, D-3349).
#[must_use]
pub fn respond_with(
    store_root: &std::path::Path,
    query: &str,
    cancel: &crate::bars::Cancel,
) -> Answered {
    let asked = match SpotAsk::parse(query) {
        Ok(asked) => asked,
        Err(why) => return bad_request(&why),
    };
    let (spots, faults) = match asked.answer_with(store_root, cancel) {
        Ok(answered) => answered,
        Err(why) => return bad_request(&why),
    };
    let status = if faults.is_empty() {
        axum::http::StatusCode::OK
    } else {
        axum::http::StatusCode::PARTIAL_CONTENT
    };
    (status, json_headers(), asked.render(&spots, &faults))
}

/// `GET /spot.json?feed&exchange&symbol&timeframe&month&t=<s>,<s>…[&asof=prev]`
///
/// 400 for a malformed question, 206 when any record would not read (the
/// answers are real and the set is not whole, as `/bars.json` says it), 200
/// otherwise — including when every answer is a named absence. 429 with
/// `Retry-After` when the store-read pool is full (DB-066, DB-G80).
pub async fn spot_json(
    axum::extract::State(site): axum::extract::State<Loaded>,
    uri: axum::http::Uri,
) -> Answered {
    let query = uri.query().unwrap_or("").to_owned();
    let root = site.store_root.clone();
    // OFF THE ASYNC WORKERS (DB-066, DB-G80): opening a month file, its
    // lock and up to 400 lookups are blocking file reads. STOPPED WHEN THE
    // CLIENT GOES (DB-N15, D-3349): the lookups look at the flag every
    // `CANCEL_EVERY` instants.
    let answered =
        super::bars_route::cancellable(false, move |cancel| respond_with(&root, &query, cancel))
            .await;
    match answered {
        Ok(answered) => answered,
        Err(why) => not_admitted("a spot read", &why),
    }
}

/// Tests written for mutants a cloud mutation pass left alive in this file
/// (`reports/cloud-mut-api-a-20261017.md`). Every bar is generated, in a
/// scratch store the test removes.
#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "a fixture that cannot be planted is a test that cannot run, and \
              must fail loudly"
)]
mod cloud_mutant_tests {
    use super::*;
    use crate::bars::{Cancel, cancel_trip};
    use brutex_core::vendor::Vendor;
    use store::format::{Bar, OI_NULL};
    use store::path::{FileKind, StorePath};

    /// 2025-05-02 09:15 IST, in UTC seconds.
    fn open_second() -> i64 {
        i64::from(
            pull::session::Day::new(2025, 5, 2)
                .expect("a generated day")
                .days_from_epoch(),
        ) * 86_400
            + 13_500
    }

    /// The ask for `count` instants a minute apart from the open.
    fn ask(count: usize) -> SpotAsk {
        let times: Vec<String> = (0..count)
            .map(|n| (open_second() + i64::try_from(n).expect("a short ask") * 60).to_string())
            .collect();
        SpotAsk::parse(&format!(
            "feed=dhan&exchange=NSE&symbol=NIFTY&timeframe=1min&month=2025-05&t={}",
            times.join(",")
        ))
        .expect("a well-formed ask")
    }

    /// **A CLIENT THAT GOES AWAY IN THE MIDDLE OF A SPOT READ IS SEEN AT THE
    /// NEXT LOOK OF THE STRIDE: NOT AT EVERY INSTANT, AND NOT NEVER.** The
    /// read looks at its flag on instants `0, CANCEL_EVERY, 2·CANCEL_EVERY …`
    /// (DB-N15). The flag here answers "not cancelled" at its first look and
    /// "cancelled" at every look after it — a client that left while instant
    /// 0 was answered. An ask of `CANCEL_EVERY` instants takes one look and is
    /// answered whole; one more instant takes a second look, on instant
    /// `CANCEL_EVERY`, and is refused by name. Kills `% with /` in
    /// `answer_with`, which looked at every instant of the first stride (and
    /// refused the two-instant ask) and never after it (and answered the long
    /// one).
    #[test]
    fn a_client_gone_mid_read_is_seen_at_the_next_look_of_the_stride() {
        let root = crate::scratch::path("cloud-spot-stride");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("a scratch root");
        let path = StorePath::new(PathParts {
            vendor: Vendor::Dhan,
            exchange: "NSE",
            segment: "INDEX",
            symbol: "NIFTY",
            contract: None,
            timeframe: Timeframe::MINUTE_1,
            month: YearMonth::new(2025, 5).expect("a month"),
            file: FileKind::Bars,
        })
        .expect("a generated path");
        let id = u32::try_from(brutex_core::universe::fnv1a("NIFTY") & u64::from(u32::MAX))
            .expect("the low 32 bits");
        let mut file = BarFile::open_or_create(&root, path, id).expect("a generated month");
        file.append(&[Bar {
            ts_micros: open_second() * 1_000_000,
            open: 2_400_000,
            high: 2_400_005,
            low: 2_399_995,
            close: 2_400_000,
            volume: 0,
            open_interest: OI_NULL,
        }])
        .expect("a generated record");
        drop(file);

        let (cancel, guard) = Cancel::pair();
        for (count, answered) in [(2, true), (CANCEL_EVERY, true), (CANCEL_EVERY + 1, false)] {
            cancel_trip::arm(1);
            let got = ask(count).answer_with(&root, &cancel);
            let left = cancel_trip::disarm();
            if answered {
                let (spots, faults) = got.expect("one look, before the client left");
                assert_eq!(spots.len(), count, "every instant is answered");
                assert!(faults.is_empty(), "{faults:?}");
                assert_eq!(left, Some(0), "exactly one look for {count} instants");
            } else {
                let refused = got.expect_err("the second look sees the client gone");
                assert!(refused.starts_with("cancelled"), "{refused}");
            }
        }
        guard.disarm();
        let _ = std::fs::remove_dir_all(&root);
    }
}

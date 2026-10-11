//! `GET /greeks.json` — one option bar's implied volatility and greeks, solved
//! in Rust from the bar's own close, so `/db` can show them (D-2830, D-3330).
//!
//! # The five inputs, and where each one comes from
//!
//! * **Premium** — the option bar's own stored close at `t`, read from the
//!   option's month file. The page does not send a price: a figure the server
//!   did not read is not one it can stand behind.
//! * **Spot** — the underlying's stored close at exactly `t`, on the same feed,
//!   rung and month, by [`crate::server::spot::spot_at`] — the same reader and
//!   the same named absences as `/spot.json` (D-2807). With `asof=prev`, the
//!   underlying's last bar at or before `t` in `t`'s IST session, its stamp and
//!   age on the row (D-3328). Never a later close.
//! * **Time to expiry** — [`pull::tenor::Tenor::to_settled`]: from the bar's
//!   own stamp to the close of the NSE derivatives session on the day the
//!   contract SETTLED (D-3331: MIDCPNIFTY `22JAN24` settled on 2024-01-20), read
//!   from the dated session table (15:30, and 15:40 from 2026-08-03). Under
//!   `YearBasis::Calendar365`. D-2831. Each row carries it in seconds
//!   (`tenor_s`) and in years (`tenor_y`).
//! * **Rate** — the RBI 91-day T-bill cut-off yield as of the latest release
//!   strictly before the bar's IST trade day, [`pull::riskfree::Series::on`]
//!   (D-2810), from the table the operator names in [`RATE_ENV`]. **No
//!   default and no fallback**: unset, empty or unreadable is a named refusal
//!   in every cell (`rate_unset`, `rate_unreadable`), never a guessed rate.
//!   The table is loaded once and reloaded when its file stamp changes
//!   ([`RateTable::current`], DB-067), and every answer carries its sha256
//!   (`rate_sha256`), so two chunks of one page can be shown to have used one
//!   table. A tenor longer than the rate's own 91 days is flagged
//!   `above_rate_tenor` (D-3330, DB-G64): the rate is still the dated 91-day
//!   yield, and the flag says it is applied beyond its maturity.
//! * **Carry** — zero. `docs/00-charter.md` records no dividend yield, so for a
//!   stock option the zero is "none sourced", said on the wire (`carry_source`)
//!   and in D-2830, not a measurement.
//!
//! The model is [`pull::pricing::bar_greeks`], which is `greeks::bsm` through
//! `pull`'s existing arrow; `api` gains no arrow and the page computes nothing.
//!
//! # Every cell is a figure or a named reason
//!
//! A bar that cannot be priced says why, per instant: an instant outside the
//! asked month (`t_outside_month`, D-3328), the option's or the spot's absence
//! (`no_option_bar`, `no_spot_bar`, `no_spot_in_session`, …), `tenor_refused`
//! (expired, unsettled or unverified hours), `rate_before_series` /
//! `rate_stale`, `below_zero_carry_bound` (a premium at or below the zero-carry
//! discounted intrinsic: a limit of the model's inputs, not a verdict on the
//! print, D-3330), or `iv_unsolved` with the model's own sentence.
//!
//! # Cost, stated rather than claimed
//!
//! Per request: the rate table's file stamp (one `stat`; the table is read
//! and indexed only when the stamp moved, O(rows + days)), and two month
//! files opened. Per instant: two `first_at_or_after` lookups (O(1) through a
//! month's `.tix`, the bisection otherwise, as `/spot.json` states), one O(1)
//! dense-table rate lookup (C-RF-01), one O(1) settled tenor, and one solve
//! bounded by `greeks::solver::MAX_ITERATIONS`. Instants per request are capped
//! at [`MAX_GREEK_TIMES`]. The work runs on the blocking pool behind
//! `crate::detail::run_store_read`'s admission (DB-066, DB-G80). Measured
//! timings are in WP-32's report, not restated here.

use super::{Loaded, ingest, param, render, timeframe_param};
use crate::bars::OpenRefusalKind;
use crate::server::spot::{
    Answered, AsOf, Spot, bad_request, in_month, json_headers, month_param, not_admitted, spot_at,
    spot_fields, times_param, underlying_key, underlying_segment,
};
use brutex_core::instrument::{Contract, Expiry, OptionSide};
use pull::expiry_alias::{SettleRefusal, Settled};
use pull::pricing::{BarGreeks, BarQuote};
use pull::riskfree::Series;
use pull::tenor::{Tenor, YearBasis};
use std::fmt::Write as _;
use std::sync::{Arc, Mutex, PoisonError};
use store::file::BarFile;
use store::path::{PathParts, Timeframe, YearMonth};

/// The variable naming the RBI 91-day T-bill table (`pull::riskfree::HEADER`).
///
/// No default: unset or empty refuses every cell by name (D-2830, the D-2804
/// pattern).
pub const RATE_ENV: &str = "BRUTEX_TBILL91_SERIES";

/// The most instants one request may ask for — the `/spot.json` bound, for
/// the same 8 KiB request target.
pub const MAX_GREEK_TIMES: usize = crate::server::spot::MAX_SPOT_TIMES;

/// The rate's own maturity, in days: the RBI 91-day T-bill (D-2810). A tenor
/// longer than this is solved with a rate shorter than itself, and says so
/// (`above_rate_tenor`, D-3330).
pub const RATE_TENOR_DAYS: i64 = 91;

/// Seconds in a day.
const SECONDS_PER_DAY: i64 = 86_400;

/// Microseconds in a second.
const MICROS: i64 = 1_000_000;

/// The rate table as this request found it.
#[derive(Debug)]
pub enum RateTable {
    /// Read and indexed.
    Loaded(Series),
    /// [`RATE_ENV`] is unset or empty.
    Unset,
    /// The path was named and the table refused; the reason, verbatim.
    Unreadable(String),
}

/// What a file's stamp is compared on: its length and its modification
/// time. A table rewritten in place moves at least one of them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FileStamp {
    len: u64,
    modified: std::time::SystemTime,
}

/// The table [`RateTable::current`] last loaded, the path it named and the
/// stamp it was read under.
#[derive(Debug)]
struct RateKept {
    path: std::ffi::OsString,
    stamp: FileStamp,
    table: Arc<RateTable>,
    /// How many times the table has been read from disk since the process
    /// started: what a test reads to prove a hit read nothing.
    loads: u64,
}

/// The process's one kept rate table (DB-067). One lock, held across a
/// reload so concurrent misses read the file once.
static RATE_KEPT: Mutex<Option<RateKept>> = Mutex::new(None);

impl RateTable {
    /// The table a value of [`RATE_ENV`] names, read now.
    ///
    /// Split from the handler so every arm is testable without mutating the
    /// environment, which edition 2024 makes `unsafe` and this crate forbids.
    #[must_use]
    pub fn from_var(value: Option<std::ffi::OsString>) -> Self {
        match value {
            Some(path) if !path.is_empty() => match Series::read(std::path::Path::new(&path)) {
                Ok(series) => Self::Loaded(series),
                Err(why) => {
                    Self::Unreadable(format!("{}: {why}", std::path::Path::new(&path).display()))
                }
            },
            _ => Self::Unset,
        }
    }

    /// The table a value of [`RATE_ENV`] names, LOADED ONCE AND RELOADED
    /// WHEN ITS FILE STAMP MOVES (DB-067, D-3330).
    ///
    /// Until D-3330 every request re-read, re-parsed and re-hashed the table
    /// (about 0.3 ms of a refused request's 0.38 ms), and two chunks of one
    /// page could be solved against two tables. Now: one `stat` per request,
    /// and the read only when the path or its stamp (length, modification
    /// time) differs from the kept one. A file whose modification time the
    /// platform cannot report is read every time -- never kept under a stamp
    /// that cannot move. A path that will not `stat` is `Unreadable` with the
    /// reason, never the kept table.
    #[must_use]
    pub fn current(value: Option<std::ffi::OsString>) -> Arc<Self> {
        Self::current_counted(value).0
    }

    /// [`Self::current`], and how many disk loads the process has made.
    fn current_counted(value: Option<std::ffi::OsString>) -> (Arc<Self>, u64) {
        let Some(path) = value.filter(|path| !path.is_empty()) else {
            return (Arc::new(Self::Unset), 0);
        };
        let stamp = match std::fs::metadata(&path) {
            Ok(meta) => meta.modified().ok().map(|modified| FileStamp {
                len: meta.len(),
                modified,
            }),
            Err(why) => {
                return (
                    Arc::new(Self::Unreadable(format!(
                        "{}: {why}",
                        std::path::Path::new(&path).display()
                    ))),
                    0,
                );
            }
        };
        let mut kept = RATE_KEPT.lock().unwrap_or_else(PoisonError::into_inner);
        if let (Some(stamp), Some(held)) = (stamp, kept.as_ref())
            && held.path == path
            && held.stamp == stamp
        {
            return (Arc::clone(&held.table), held.loads);
        }
        let table = Arc::new(Self::from_var(Some(path.clone())));
        let loads = kept.as_ref().map_or(0, |held| held.loads).saturating_add(1);
        match stamp {
            Some(stamp) => {
                *kept = Some(RateKept {
                    path,
                    stamp,
                    table: Arc::clone(&table),
                    loads,
                });
            }
            None => {
                if let Some(held) = kept.as_mut() {
                    held.loads = loads;
                }
            }
        }
        (table, loads)
    }

    /// The wire's state word.
    #[must_use]
    pub const fn state(&self) -> &'static str {
        match self {
            Self::Loaded(_) => "loaded",
            Self::Unset => "unset",
            Self::Unreadable(_) => "unreadable",
        }
    }

    /// The loaded table's sha256, as hex.
    #[must_use]
    pub fn sha256_hex(&self) -> Option<String> {
        match self {
            Self::Loaded(series) => Some(crate::server::hex32(series.sha256())),
            Self::Unset | Self::Unreadable(_) => None,
        }
    }
}

/// How many times [`RateTable::current`] has read a table from disk in this
/// process, for the test that proves a hit reads nothing.
#[cfg(test)]
pub(crate) fn rate_loads(value: Option<std::ffi::OsString>) -> (Arc<RateTable>, u64) {
    RateTable::current_counted(value)
}

/// The spot a cell was priced or refused against: its close and the stamp of
/// the bar it came from, microseconds.
type SpotRead = (i64, i64);

/// One instant's answer.
#[derive(Debug, Clone, PartialEq)]
pub enum Cell {
    /// Solved.
    Priced {
        /// The option bar's close, paisa.
        premium: i64,
        /// The underlying's close and its bar's stamp (`t`, or under
        /// `asof=prev` the session's last bar at or before `t`).
        spot: SpotRead,
        /// From the bar's stamp to the settled expiry session's close.
        tenor: Tenor,
        /// The continuous rate used.
        rate: f64,
        /// The T-bill release that rate came from.
        released: pull::pricing::TradeDay,
        /// The volatility and the greeks.
        greeks: BarGreeks,
    },
    /// Not solved, and why.
    Refused {
        /// The wire's reason word.
        why: &'static str,
        /// The reader's or the model's own sentence, when it has one.
        detail: Option<String>,
        /// The option bar's close, when it was read.
        premium: Option<i64>,
        /// The underlying's close and stamp, when they were read.
        spot: Option<SpotRead>,
        /// The tenor, when it was measured.
        tenor: Option<Tenor>,
    },
}

impl Cell {
    fn refused(why: &'static str, detail: Option<String>) -> Self {
        Self::Refused {
            why,
            detail,
            premium: None,
            spot: None,
            tenor: None,
        }
    }
}

/// The expiry, strike (paisa) and side an option contract segment names, or
/// `None` for a future or anything else.
///
/// The grammar is the store's own (`core::instrument::Contract`'s rendering):
/// `YYYY-MM-DD-<paisa>-CE|PE`.
#[must_use]
pub fn option_parts(text: &str) -> Option<(Expiry, i64, OptionSide)> {
    let (rest, side) = text.rsplit_once('-')?;
    let side = match side {
        "CE" => OptionSide::Call,
        "PE" => OptionSide::Put,
        _ => return None,
    };
    let (date, strike) = rest.rsplit_once('-')?;
    if strike.is_empty() || !strike.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let strike = strike.parse::<i64>().ok().filter(|paisa| *paisa > 0)?;
    let mut pieces = date.split('-');
    let (Some(y), Some(m), Some(d), None) =
        (pieces.next(), pieces.next(), pieces.next(), pieces.next())
    else {
        return None;
    };
    if y.len() != 4 || m.len() != 2 || d.len() != 2 {
        return None;
    }
    let expiry = Expiry::new(y.parse().ok()?, m.parse().ok()?, d.parse().ok()?).ok()?;
    Some((expiry, strike, side))
}

/// The day the contract `symbol` `expiry` settled on (D-3331), as
/// [`pull::expiry_alias::settle`] answers it for a tenor reader (no trade
/// day).
fn settled_of(symbol: &str, expiry: Expiry) -> Result<Settled, SettleRefusal> {
    let stated = pull::session::Day::new(expiry.year(), expiry.month(), expiry.day())
        .map_err(|_| SettleRefusal::RowDayInvalid)?;
    pull::expiry_alias::settle(symbol, stated, None)
}

/// Everything `/greeks.json` reads off the query string. Every field refuses
/// rather than defaults, but `asof`, whose default is D-2807's exact rule.
#[derive(Debug)]
pub struct GreeksAsk {
    vendor: brutex_core::vendor::Vendor,
    exchange: String,
    segment: String,
    symbol: String,
    contract: Contract,
    expiry: Expiry,
    strike: i64,
    side: OptionSide,
    timeframe: Timeframe,
    month: YearMonth,
    asof: AsOf,
    times: Vec<i64>,
    /// The day the contract settled on, or why it could not be settled.
    settled: Result<Settled, SettleRefusal>,
}

impl GreeksAsk {
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
        let segment = param(query, "segment");
        let symbol = param(query, "symbol");
        if exchange.is_empty() || segment.is_empty() || symbol.is_empty() {
            return Err(
                "exchange, segment and symbol (the option's own, as the store files it) are all required"
                    .into(),
            );
        }
        let raw = param(query, "contract");
        let (Some(contract), Some((expiry, strike, side))) =
            (Contract::parse(&raw), option_parts(&raw))
        else {
            return Err(format!(
                "{raw:?} is not an option contract segment: it must be \
                 YYYY-MM-DD-<strike paisa>-CE|PE, the part below the symbol"
            ));
        };
        if param(query, "timeframe").is_empty() {
            return Err(
                "timeframe is required: the greeks are read at the option bar's own rung".into(),
            );
        }
        let timeframe = timeframe_param(query)?;
        let month = month_param(query)?;
        let asof = AsOf::parse(query)?;
        let times = times_param(query, "t", MAX_GREEK_TIMES)?;
        let settled = settled_of(&symbol, expiry);
        Ok(Self {
            vendor,
            exchange,
            segment,
            symbol,
            contract,
            expiry,
            strike,
            side,
            timeframe,
            month,
            asof,
            times,
            settled,
        })
    }

    fn parts<'a>(&'a self, segment: &'a str, contract: Option<Contract>) -> PathParts<'a> {
        PathParts {
            vendor: self.vendor,
            exchange: &self.exchange,
            segment,
            symbol: &self.symbol,
            contract,
            timeframe: self.timeframe,
            month: self.month,
            file: store::path::FileKind::Bars,
        }
    }

    /// Answers every instant, and the faults met, named.
    ///
    /// # Errors
    ///
    /// A store path the question names that is not a valid address — refused
    /// before any file is read.
    pub fn answer(
        &self,
        store_root: &std::path::Path,
        rate: &RateTable,
    ) -> Result<(Vec<Cell>, Vec<String>), String> {
        self.answer_with(store_root, rate, &crate::bars::Cancel::never())
    }

    /// [`Self::answer`], looking at `cancel` every
    /// [`crate::server::spot::CANCEL_EVERY`] instants and stopping by name
    /// once the client went away (DB-N15, D-3349).
    ///
    /// # Errors
    ///
    /// What [`Self::answer`] refuses, and `cancelled`.
    pub fn answer_with(
        &self,
        store_root: &std::path::Path,
        rate: &RateTable,
        cancel: &crate::bars::Cancel,
    ) -> Result<(Vec<Cell>, Vec<String>), String> {
        let mut faults = Vec::new();
        let option = match crate::bars::open_classified(
            store_root,
            self.parts(&self.segment, Some(self.contract)),
        ) {
            Ok(file) => Ok(file),
            Err(refusal) => match refusal.kind {
                OpenRefusalKind::Invalid => return Err(refusal.message),
                OpenRefusalKind::Absent => Err(("no_option_file", None)),
                OpenRefusalKind::Unreadable => {
                    faults.push(refusal.message.clone());
                    Err(("option_unreadable", Some(refusal.message)))
                }
            },
        };
        let underlying = match crate::bars::open_classified(
            store_root,
            // The underlying's key from the alias table (D-3332): FINNIFTY's
            // spot is filed as NIFTYFINSERVICE, TATAMOTORS's as TMPV.
            PathParts {
                symbol: underlying_key(&self.symbol),
                ..self.parts(underlying_segment(&self.symbol), None)
            },
        ) {
            Ok(file) => Ok(file),
            Err(refusal) => match refusal.kind {
                OpenRefusalKind::Invalid => return Err(refusal.message),
                OpenRefusalKind::Absent => Err(Spot::NoFile),
                OpenRefusalKind::Unreadable => {
                    faults.push(refusal.message);
                    Err(Spot::Unreadable)
                }
            },
        };
        let mut cells = Vec::with_capacity(self.times.len());
        for (at, &ts) in self.times.iter().enumerate() {
            if at % crate::server::spot::CANCEL_EVERY == 0 {
                cancel.check()?;
            }
            // A WRONG MONTH IS A WRONG QUESTION (D-3328, DB-039), answered
            // before any file is consulted, held or not.
            if !in_month(self.month, ts) {
                cells.push(Cell::refused(
                    "t_outside_month",
                    Some(format!(
                        "t={} is not in the IST month {} this request names; ask it in its own month",
                        ts.div_euclid(MICROS),
                        self.month
                    )),
                ));
                continue;
            }
            cells.push(match &option {
                Ok(file) => self.cell(ts, file, underlying.as_ref(), rate, &mut faults),
                Err((why, detail)) => Cell::refused(why, detail.clone()),
            });
        }
        Ok((cells, faults))
    }

    /// One instant, already inside the month.
    #[expect(
        clippy::too_many_lines,
        reason = "the cell's inputs in the order a refusal names them -- premium, spot, \
                  tenor, rate, solve -- each refusal beside the read it refuses"
    )]
    fn cell(
        &self,
        ts: i64,
        option: &BarFile,
        underlying: Result<&BarFile, &Spot>,
        rate: &RateTable,
        faults: &mut Vec<String>,
    ) -> Cell {
        let mut read = |file: &BarFile, asof: AsOf| {
            spot_at(file, self.month, ts, asof).unwrap_or_else(|why| {
                faults.push(format!(
                    "{} t={}: {why}",
                    file.path().display(),
                    ts.div_euclid(MICROS)
                ));
                Spot::Unreadable
            })
        };
        // THE PREMIUM IS ALWAYS THE BAR AT `t` ITSELF: `asof` is the spot's
        // rule, never the option's.
        let premium = match read(option, AsOf::Exact) {
            Spot::At { close, .. } => close,
            Spot::NoBar | Spot::NoBarInSession => return Cell::refused("no_option_bar", None),
            Spot::OutsideMonth => return Cell::refused("t_outside_month", None),
            Spot::OffGrid => return Cell::refused("t_off_grid", None),
            Spot::OrderBroken => return Cell::refused("option_order_broken", None),
            Spot::NoFile => return Cell::refused("no_option_file", None),
            Spot::Unreadable => return Cell::refused("option_unreadable", None),
        };
        let found = match underlying {
            Ok(file) => read(file, self.asof),
            Err(absent) => absent.clone(),
        };
        let Spot::At {
            close: spot,
            ts_micros: spot_ts,
        } = found
        else {
            return Cell::Refused {
                // `why` is `None` only for `At`, which the `else` excludes.
                why: found.why().unwrap_or("spot_unreadable"),
                detail: None,
                premium: Some(premium),
                spot: None,
                tenor: None,
            };
        };
        let refused_at = |why: &'static str, detail: String, tenor: Option<Tenor>| Cell::Refused {
            why,
            detail: Some(detail),
            premium: Some(premium),
            spot: Some((spot, spot_ts)),
            tenor,
        };
        // MEASURED TO THE SETTLED EXPIRY (D-3331, DB-G51), before the rate,
        // so a cell refused for its rate still shows how long it had to run.
        let tenor = match Tenor::to_settled(ts, &self.symbol, self.expiry) {
            Ok((tenor, _)) => tenor,
            Err(why) => return refused_at("tenor_refused", why.to_string(), None),
        };
        let refused = |why: &'static str, detail: String| refused_at(why, detail, Some(tenor));
        let series = match rate {
            RateTable::Loaded(series) => series,
            RateTable::Unset => {
                return refused(
                    "rate_unset",
                    format!(
                        "{RATE_ENV} is not set, so no risk-free rate is known and none is guessed"
                    ),
                );
            }
            RateTable::Unreadable(why) => return refused("rate_unreadable", why.clone()),
        };
        // The bar's IST trade day. `Tenor::to_settled` already placed this
        // stamp on the calendar, so the refusal arm is the same fact said twice.
        let Some(day) = pull::session::IstMoment::from_epoch_secs(ts.div_euclid(MICROS))
            .ok()
            .and_then(|at| pull::pricing::trade_day_of(at.day()))
        else {
            return refused(
                "tenor_refused",
                format!("the stamp {ts} is not a trade day the rate calendar holds"),
            );
        };
        let as_of = match series.on(day) {
            Ok(as_of) => as_of,
            Err(why @ pull::riskfree::LookupError::BeforeSeries { .. }) => {
                return refused("rate_before_series", why.to_string());
            }
            Err(why @ pull::riskfree::LookupError::Stale { .. }) => {
                return refused("rate_stale", why.to_string());
            }
        };
        let bar = BarQuote {
            spot,
            strike: self.strike,
            premium,
            tenor,
            side: self.side,
        };
        match pull::pricing::bar_greeks(bar, as_of.rate, YearBasis::Calendar365) {
            Ok(greeks) => Cell::Priced {
                premium,
                spot: (spot, spot_ts),
                tenor,
                rate: as_of.rate.annual(),
                released: as_of.auction.released,
                greeks,
            },
            // THE ZERO-CARRY FLOOR, NAMED APART (D-3330, DB-G64): the page
            // read `iv_unsolved` on a deep in-the-money long-dated bar as a
            // bad print, when it is the model's zero carry that puts the
            // floor above the premium.
            Err(why) => match why.below_discounted_intrinsic() {
                Some((price, floor)) => refused(
                    "below_zero_carry_bound",
                    format!(
                        "the premium {price} paisa is at or below {floor} paisa, the intrinsic value \
                         discounted at the rate with ZERO carry (D-2830: no dividend or carry is \
                         sourced), so no volatility reproduces it under this model's inputs; a limit \
                         of the model's inputs, not a verdict on the print (D-3330)"
                    ),
                ),
                None => refused("iv_unsolved", why.to_string()),
            },
        }
    }

    /// The response body.
    #[must_use]
    #[expect(
        clippy::too_many_lines,
        reason = "one body written in the order the wire carries it: the rows, the rate \
                  table, the settlement, then the header fields"
    )]
    pub fn render(&self, cells: &[Cell], faults: &[String], rate: &RateTable) -> String {
        let mut rows = String::with_capacity(cells.len() * 320 + 2);
        rows.push('[');
        for (n, (&ts, cell)) in self.times.iter().zip(cells).enumerate() {
            if n > 0 {
                rows.push(',');
            }
            let paisa = |v: Option<i64>| v.map_or_else(|| "null".to_owned(), |p| p.to_string());
            match cell {
                Cell::Priced {
                    premium,
                    spot,
                    tenor,
                    rate,
                    released,
                    greeks,
                } => {
                    let _ = write!(
                        rows,
                        r#"{{"t":{},"premium":{premium},{},{},"rate":{rate},"auction":{},"iv":{},"delta":{},"gamma":{},"theta":{},"vega":{},"rho":{},"iterations":{},"below_band":{},"why":null,"detail":null}}"#,
                        ts.div_euclid(MICROS),
                        spot_fields("spot", Some(*spot), ts),
                        tenor_fields(Some(*tenor)),
                        render::json_string(&released.to_string()),
                        greeks.volatility,
                        greeks.delta,
                        greeks.gamma_per_rupee,
                        greeks.theta_rupees_per_day,
                        greeks.vega_rupees_per_vol_point,
                        greeks.rho_rupees_per_rate_point,
                        greeks.iterations,
                        greeks.below_validated_band,
                    );
                }
                Cell::Refused {
                    why,
                    detail,
                    premium,
                    spot,
                    tenor,
                } => {
                    let _ = write!(
                        rows,
                        r#"{{"t":{},"premium":{},{},{},"why":{},"detail":{}}}"#,
                        ts.div_euclid(MICROS),
                        paisa(*premium),
                        spot_fields("spot", *spot, ts),
                        tenor_fields(*tenor),
                        render::json_string(why),
                        detail
                            .as_deref()
                            .map_or_else(|| "null".to_owned(), render::json_string),
                    );
                }
            }
        }
        rows.push(']');
        let table = match rate {
            RateTable::Loaded(series) => format!(
                r#"{{"variable":{},"state":"loaded","error":null,"sha256":{},"auctions":{},"first_release":{},"last_release":{},"cite":{}}}"#,
                render::json_string(RATE_ENV),
                render::json_string(&crate::server::hex32(series.sha256())),
                series.auction_count(),
                render::json_string(&series.first_release().to_string()),
                render::json_string(&series.last_release().to_string()),
                render::json_string(pull::riskfree::CHARTER_CITE),
            ),
            RateTable::Unset => format!(
                r#"{{"variable":{},"state":"unset","error":{}}}"#,
                render::json_string(RATE_ENV),
                render::json_string(&format!("{RATE_ENV} is not set; no rate is guessed")),
            ),
            RateTable::Unreadable(why) => format!(
                r#"{{"variable":{},"state":"unreadable","error":{}}}"#,
                render::json_string(RATE_ENV),
                render::json_string(why),
            ),
        };
        let (settled, settlement) = match &self.settled {
            Ok(settled) => (
                render::json_string(&settled.expiry.to_string()),
                render::json_string(settled.settlement.as_str()),
            ),
            Err(why) => (
                "null".to_owned(),
                render::json_string(&format!("unsettled: {why}")),
            ),
        };
        format!(
            r#"{{"exchange":{},"segment":{},"symbol":{},"contract":{},"timeframe":{},"month":{},"asof":{},"spot_segment":{},"spot_key":{},"expiry":{},"settled_expiry":{settled},"settlement":{settlement},"basis":"calendar365","carry":0,"carry_source":{},"expiry_close":{},"rate_tenor_days":{RATE_TENOR_DAYS},"rate_sha256":{},"rate":{table},"faults":{},"rows":{rows}}}"#,
            render::json_string(&self.exchange),
            render::json_string(&self.segment),
            render::json_string(&self.symbol),
            render::json_string(self.contract.as_str()),
            render::json_string(self.timeframe.as_str()),
            render::json_string(&self.month.to_string()),
            render::json_string(self.asof.as_str()),
            render::json_string(underlying_segment(&self.symbol)),
            render::json_string(underlying_key(&self.symbol)),
            render::json_string(&self.expiry.to_string()),
            render::json_string(CARRY_SOURCE),
            render::json_string(EXPIRY_CLOSE),
            rate.sha256_hex()
                .map_or_else(|| "null".to_owned(), |hex| render::json_string(&hex)),
            if faults.is_empty() {
                "null".to_owned()
            } else {
                render::json_string(&faults.join("; "))
            }
        )
    }
}

/// `"tenor_s"`, `"tenor_y"` and `"above_rate_tenor"` for one row, `null`
/// each when the tenor was not measured (D-3330).
fn tenor_fields(tenor: Option<Tenor>) -> String {
    match tenor {
        Some(tenor) => format!(
            r#""tenor_s":{},"tenor_y":{},"above_rate_tenor":{}"#,
            tenor.seconds(),
            tenor.years(YearBasis::Calendar365),
            above_rate_tenor(tenor),
        ),
        None => r#""tenor_s":null,"tenor_y":null,"above_rate_tenor":null"#.to_owned(),
    }
}

/// Whether `tenor` runs past the rate's own maturity, [`RATE_TENOR_DAYS`]
/// (D-3330): the 91-day yield applied to a longer contract is a rate
/// mismatched to its tenor, flagged and never re-derived.
#[must_use]
pub const fn above_rate_tenor(tenor: Tenor) -> bool {
    tenor.seconds() > RATE_TENOR_DAYS * SECONDS_PER_DAY
}

/// What the zero carry is, said on the wire. D-2830.
pub const CARRY_SOURCE: &str = "none sourced: docs/00-charter.md records no dividend yield, so the carry is zero (D-2830); for a stock option that is an unsourced zero, not a measurement";

/// The time-to-expiry convention, said on the wire. D-2831, D-3331.
pub const EXPIRY_CLOSE: &str = "from the bar's own stamp to the close of the NSE derivatives session on the day the contract settled (D-3331), from the dated session table (15:30; 15:40 from 2026-08-03), over a 365-day year (D-2831)";

/// The handler's body, with the rate table handed in so a test can name it.
#[must_use]
pub fn respond(store_root: &std::path::Path, query: &str, rate: &RateTable) -> Answered {
    respond_with(store_root, query, rate, &crate::bars::Cancel::never())
}

/// [`respond`], stopping once `cancel` is set (DB-N15, D-3349).
#[must_use]
pub fn respond_with(
    store_root: &std::path::Path,
    query: &str,
    rate: &RateTable,
    cancel: &crate::bars::Cancel,
) -> Answered {
    let asked = match GreeksAsk::parse(query) {
        Ok(asked) => asked,
        Err(why) => return bad_request(&why),
    };
    let (cells, faults) = match asked.answer_with(store_root, rate, cancel) {
        Ok(answered) => answered,
        // A CONTRACT THE DECODER REFUSES IS GONE, as on `/bars/window.json`
        // and `/bars.json`: 410 and the refusal the re-key recorded, never a
        // 400 that reads as a malformed question (D-3331; WP-12c, DB-A18).
        Err(why) if pull::gdfl_lineage::is_contract_refused(&why) => {
            return (
                axum::http::StatusCode::GONE,
                json_headers(),
                format!(r#"{{"error":{}}}"#, render::json_string(&why)),
            );
        }
        Err(why) => return bad_request(&why),
    };
    let status = if faults.is_empty() {
        axum::http::StatusCode::OK
    } else {
        axum::http::StatusCode::PARTIAL_CONTENT
    };
    (status, json_headers(), asked.render(&cells, &faults, rate))
}

/// `GET /greeks.json?feed&exchange&segment&symbol&contract&timeframe&month&t=<s>,…[&asof=prev]`
///
/// 400 for a malformed question, 206 when any record would not read (the
/// answers are real and the set is not whole), 200 otherwise — including
/// when every cell is a named refusal, the rate's among them. 429 with
/// `Retry-After` when the store-read pool is full (DB-066, DB-G80).
pub async fn greeks_json(
    axum::extract::State(site): axum::extract::State<Loaded>,
    uri: axum::http::Uri,
) -> Answered {
    let query = uri.query().unwrap_or("").to_owned();
    let root = site.store_root.clone();
    // OFF THE ASYNC WORKERS (DB-066, DB-G80): the rate table's stamp, two
    // month files and up to 400 lookups and solves are blocking work.
    // STOPPED WHEN THE CLIENT GOES (DB-N15, D-3349).
    let answered = super::bars_route::cancellable(false, move |cancel| {
        respond_with(
            &root,
            &query,
            &RateTable::current(std::env::var_os(RATE_ENV)),
            cancel,
        )
    })
    .await;
    match answered {
        Ok(answered) => answered,
        Err(why) => not_admitted("a greeks read", &why),
    }
}

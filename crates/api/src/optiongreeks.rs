//! `GET /greeks.json` — one option bar's implied volatility and greeks, solved
//! in Rust from the bar's own close, so `/db` can show them (D-2830).
//!
//! # The five inputs, and where each one comes from
//!
//! * **Premium** — the option bar's own stored close at `t`, read from the
//!   option's month file. The page does not send a price: a figure the server
//!   did not read is not one it can stand behind.
//! * **Spot** — the underlying's stored close at exactly `t`, on the same feed,
//!   rung and month, by [`crate::server::spot::close_at`] — the same reader and
//!   the same named absences as `/spot.json` (D-2807). Never a nearest close.
//! * **Time to expiry** — [`pull::tenor::Tenor::between`]: from the bar's own
//!   stamp to the close of the NSE derivatives session on the expiry day, read
//!   from the dated session table (15:30, and 15:40 from 2026-08-03). Under
//!   `YearBasis::Calendar365`. D-2831.
//! * **Rate** — the RBI 91-day T-bill cut-off yield as of the latest release
//!   strictly before the bar's IST trade day, [`pull::riskfree::Series::on`]
//!   (D-2810), from the table the operator names in [`RATE_ENV`]. **No
//!   default and no fallback**: unset, empty or unreadable is a named refusal
//!   in every cell (`rate_unset`, `rate_unreadable`), never a guessed rate.
//! * **Carry** — zero. `docs/00-charter.md` records no dividend yield, so for a
//!   stock option the zero is "none sourced", said on the wire (`carry_source`)
//!   and in D-2830, not a measurement.
//!
//! The model is [`pull::pricing::bar_greeks`], which is `greeks::bsm` through
//! `pull`'s existing arrow; `api` gains no arrow and the page computes nothing.
//!
//! # Every cell is a figure or a named reason
//!
//! A bar that cannot be priced says why, per instant: the option's or the
//! spot's absence (`no_option_bar`, `no_spot_bar`, …), `tenor_refused` (expired
//! or unverified hours), `rate_before_series` / `rate_stale`, or `iv_unsolved`
//! with the model's own sentence (a close below intrinsic is the common one).
//!
//! # Cost, stated rather than claimed
//!
//! Per request: the rate table is read and indexed once, O(rows + days) — a
//! few hundred rows and a few thousand day slots — and two month files are
//! opened. Per instant: two `first_at_or_after` bisections, `O(log n_valid)` each
//! (the `/spot.json` bound, D-2807 — NOT O(1)), one O(1) dense-table rate
//! lookup (C-RF-01), one O(1) tenor, and one solve bounded by
//! `greeks::solver::MAX_ITERATIONS`. Instants per request are capped at
//! [`MAX_GREEK_TIMES`]. The wall-clock time is UNVERIFIED by any gate: the one
//! measurement taken was ad hoc (`docs/06-limits.md`, D-2830).

use super::{Loaded, ingest, param, render, timeframe_param};
use crate::bars::OpenRefusalKind;
use crate::server::spot::{Spot, close_at, underlying_segment};
use brutex_core::instrument::{Contract, Expiry, OptionSide};
use pull::pricing::{BarGreeks, BarQuote};
use pull::riskfree::Series;
use pull::tenor::{Tenor, YearBasis};
use std::fmt::Write as _;
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

impl RateTable {
    /// The table a value of [`RATE_ENV`] names.
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

    /// The wire's state word.
    #[must_use]
    pub const fn state(&self) -> &'static str {
        match self {
            Self::Loaded(_) => "loaded",
            Self::Unset => "unset",
            Self::Unreadable(_) => "unreadable",
        }
    }
}

/// One instant's answer.
#[derive(Debug, Clone, PartialEq)]
pub enum Cell {
    /// Solved.
    Priced {
        /// The option bar's close, paisa.
        premium: i64,
        /// The underlying's close at the same instant, paisa.
        spot: i64,
        /// Seconds from the bar's stamp to the expiry session's close.
        tenor_seconds: i64,
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
        /// The underlying's close, when it was read.
        spot: Option<i64>,
    },
}

impl Cell {
    fn refused(why: &'static str, detail: Option<String>) -> Self {
        Self::Refused {
            why,
            detail,
            premium: None,
            spot: None,
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

/// Everything `/greeks.json` reads off the query string. Every field refuses
/// rather than defaults.
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
    times: Vec<i64>,
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
        let raw = param(query, "month");
        let month = raw
            .split_once('-')
            .and_then(|(y, m)| YearMonth::new(y.parse().ok()?, m.parse().ok()?).ok())
            .ok_or_else(|| format!("{raw:?} is not a YYYY-MM month for \"month\""))?;
        let raw = param(query, "t");
        if raw.is_empty() {
            return Err("t is required: one or more UTC seconds, comma separated".into());
        }
        let mut times = Vec::new();
        for word in raw.split(',') {
            if times.len() == MAX_GREEK_TIMES {
                return Err(format!(
                    "more than {MAX_GREEK_TIMES} instants in t; ask in chunks"
                ));
            }
            let micros = word
                .parse::<i64>()
                .ok()
                .and_then(|secs| secs.checked_mul(1_000_000))
                .ok_or_else(|| format!("{word:?} in t is not a UTC second"))?;
            times.push(micros);
        }
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
            times,
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
        let mut faults = Vec::new();
        let option = match crate::bars::open_classified(
            store_root,
            self.parts(&self.segment, Some(self.contract)),
        ) {
            Ok(file) => file,
            Err(refusal) => {
                let (why, detail) = match refusal.kind {
                    OpenRefusalKind::Invalid => return Err(refusal.message),
                    OpenRefusalKind::Absent => ("no_option_file", None),
                    OpenRefusalKind::Unreadable => {
                        faults.push(refusal.message.clone());
                        ("option_unreadable", Some(refusal.message))
                    }
                };
                return Ok((vec![Cell::refused(why, detail); self.times.len()], faults));
            }
        };
        let underlying = match crate::bars::open_classified(
            store_root,
            self.parts(underlying_segment(&self.symbol), None),
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
        let cells = self
            .times
            .iter()
            .map(|&ts| self.cell(ts, &option, underlying.as_ref(), rate, &mut faults))
            .collect();
        Ok((cells, faults))
    }

    /// One instant.
    fn cell(
        &self,
        ts: i64,
        option: &BarFile,
        underlying: Result<&BarFile, &Spot>,
        rate: &RateTable,
        faults: &mut Vec<String>,
    ) -> Cell {
        let mut read = |file: &BarFile| {
            close_at(file, ts).unwrap_or_else(|why| {
                faults.push(format!(
                    "{} t={}: {why}",
                    file.path().display(),
                    ts / 1_000_000
                ));
                Spot::Unreadable
            })
        };
        let premium = match read(option) {
            Spot::At(close) => close,
            Spot::NoBar => return Cell::refused("no_option_bar", None),
            Spot::OrderBroken => return Cell::refused("option_order_broken", None),
            Spot::NoFile => return Cell::refused("no_option_file", None),
            Spot::Unreadable => return Cell::refused("option_unreadable", None),
        };
        let found = match underlying {
            Ok(file) => read(file),
            Err(absent) => absent.clone(),
        };
        let Spot::At(spot) = found else {
            return Cell::Refused {
                // `why` is `None` only for `At`, which the `else` excludes.
                why: found.why().unwrap_or("spot_unreadable"),
                detail: None,
                premium: Some(premium),
                spot: None,
            };
        };
        let refused = |why: &'static str, detail: String| Cell::Refused {
            why,
            detail: Some(detail),
            premium: Some(premium),
            spot: Some(spot),
        };
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
        let tenor = match Tenor::between(ts, self.expiry) {
            Ok(tenor) => tenor,
            Err(why) => return refused("tenor_refused", why.to_string()),
        };
        // The bar's IST trade day. `Tenor::between` already placed this stamp
        // on the calendar, so the refusal arm is the same fact said twice.
        let Some(day) = pull::session::IstMoment::from_epoch_secs(ts.div_euclid(1_000_000))
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
                spot,
                tenor_seconds: tenor.seconds(),
                rate: as_of.rate.annual(),
                released: as_of.auction.released,
                greeks,
            },
            Err(why) => refused("iv_unsolved", why.to_string()),
        }
    }

    /// The response body.
    #[must_use]
    pub fn render(&self, cells: &[Cell], faults: &[String], rate: &RateTable) -> String {
        let mut rows = String::with_capacity(cells.len() * 256 + 2);
        rows.push('[');
        for (n, (ts, cell)) in self.times.iter().zip(cells).enumerate() {
            if n > 0 {
                rows.push(',');
            }
            let paisa = |v: Option<i64>| v.map_or_else(|| "null".to_owned(), |p| p.to_string());
            match cell {
                Cell::Priced {
                    premium,
                    spot,
                    tenor_seconds,
                    rate,
                    released,
                    greeks,
                } => {
                    let _ = write!(
                        rows,
                        r#"{{"t":{},"premium":{premium},"spot":{spot},"tenor_s":{tenor_seconds},"rate":{rate},"auction":{},"iv":{},"delta":{},"gamma":{},"theta":{},"vega":{},"rho":{},"iterations":{},"below_band":{},"why":null,"detail":null}}"#,
                        ts / 1_000_000,
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
                } => {
                    let _ = write!(
                        rows,
                        r#"{{"t":{},"premium":{},"spot":{},"why":{},"detail":{}}}"#,
                        ts / 1_000_000,
                        paisa(*premium),
                        paisa(*spot),
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
        format!(
            r#"{{"exchange":{},"segment":{},"symbol":{},"contract":{},"timeframe":{},"month":{},"basis":"calendar365","carry":0,"carry_source":{},"expiry_close":{},"rate":{table},"faults":{},"rows":{rows}}}"#,
            render::json_string(&self.exchange),
            render::json_string(&self.segment),
            render::json_string(&self.symbol),
            render::json_string(self.contract.as_str()),
            render::json_string(self.timeframe.as_str()),
            render::json_string(&self.month.to_string()),
            render::json_string(CARRY_SOURCE),
            render::json_string(EXPIRY_CLOSE),
            if faults.is_empty() {
                "null".to_owned()
            } else {
                render::json_string(&faults.join("; "))
            }
        )
    }
}

/// What the zero carry is, said on the wire. D-2830.
pub const CARRY_SOURCE: &str = "none sourced: docs/00-charter.md records no dividend yield, so the carry is zero (D-2830); for a stock option that is an unsourced zero, not a measurement";

/// The time-to-expiry convention, said on the wire. D-2831.
pub const EXPIRY_CLOSE: &str = "from the bar's own stamp to the close of the NSE derivatives session on the expiry day, from the dated session table (15:30; 15:40 from 2026-08-03), over a 365-day year (D-2831)";

/// `GET /greeks.json?feed&exchange&segment&symbol&contract&timeframe&month&t=<s>,…`
///
/// 400 for a malformed question, 206 when any record would not read (the
/// answers are real and the set is not whole), 200 otherwise — including
/// when every cell is a named refusal, the rate's among them.
pub async fn greeks_json(
    axum::extract::State(site): axum::extract::State<Loaded>,
    uri: axum::http::Uri,
) -> (
    axum::http::StatusCode,
    [(axum::http::HeaderName, &'static str); 1],
    String,
) {
    respond(
        &site.store_root,
        uri.query().unwrap_or(""),
        &RateTable::from_var(std::env::var_os(RATE_ENV)),
    )
}

/// The handler's body, with the rate table handed in so a test can name it.
#[must_use]
pub fn respond(
    store_root: &std::path::Path,
    query: &str,
    rate: &RateTable,
) -> (
    axum::http::StatusCode,
    [(axum::http::HeaderName, &'static str); 1],
    String,
) {
    let json = || {
        [(
            axum::http::header::CONTENT_TYPE,
            "application/json; charset=utf-8",
        )]
    };
    let refuse = |why: String| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            json(),
            format!(r#"{{"error":{}}}"#, render::json_string(&why)),
        )
    };
    let asked = match GreeksAsk::parse(query) {
        Ok(asked) => asked,
        Err(why) => return refuse(why),
    };
    let (cells, faults) = match asked.answer(store_root, rate) {
        Ok(answered) => answered,
        Err(why) => return refuse(why),
    };
    let status = if faults.is_empty() {
        axum::http::StatusCode::OK
    } else {
        axum::http::StatusCode::PARTIAL_CONTENT
    };
    (status, json(), asked.render(&cells, &faults, rate))
}

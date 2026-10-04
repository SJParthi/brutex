//! `GET /spot.json` — the underlying's stored close at an option bar's own
//! instant, so `/db` can name moneyness, intrinsic and extrinsic (D-2807).
//!
//! # What it answers, and what it never answers
//!
//! For each asked instant `t` (UTC seconds, an option bar's own stamp) it reads
//! the UNDERLYING's bar file on the same feed, at the same rung, in the same
//! month file, and returns that bar's close only when a bar is stamped at
//! exactly `t`. The underlying is `NSE/INDEX/<symbol>` for the five F&O index
//! underlyings `brutex_core::universe::FNO_INDEX_UNDERLYINGS` names and
//! `NSE/CASH/<symbol>` for every other option underlying, which is a share.
//!
//! **Never a nearest or an earlier bar.** A missing spot bar is answered by
//! name (`no_spot_bar`, `no_spot_file`, `spot_unreadable`,
//! `spot_order_broken`), never by the previous close: a stale spot would put
//! the option on the wrong rung while looking exactly like a measurement.
//! `CLAUDE.md` §4. **No look-ahead:** the close returned is the close of the
//! bar that opens at `t`, which is complete at the same instant the option
//! bar at `t` is; no later bar's value is returned. The bisection below may
//! READ later records to LOCATE `t`, and returns none of their values.
//!
//! # Cost, stated rather than claimed
//!
//! The file is opened once per request. Each instant is one
//! [`store::file::BarFile::first_at_or_after`] bisection — at most
//! `ceil(log2(n_valid + 1))` record reads, proven by the store's own test
//! `first_at_or_after_never_probes_more_than_the_bisection_height` — plus at
//! most two record reads to confirm the hit or bracket the miss.
//! That is `O(log n_valid)` per instant, NOT `O(1)`: the store is addressed by
//! row index and a month file is sparse in time, so no row offset can be
//! computed from a timestamp alone. `n_valid` is bounded by the month's
//! geometry (`docs/06-limits.md`, D-2807), so the bound is a constant per rung:
//! fourteen probes at the one-minute month ceiling, twenty at the one-second.
//! Instants per request are capped at [`MAX_SPOT_TIMES`]. The wall-clock
//! time is UNVERIFIED by any gate: no bench row times this lookup, and the
//! one measurement taken was ad hoc (`docs/06-limits.md`, D-2807).

use super::{Loaded, ingest, param, render, timeframe_param};
use crate::bars::OpenRefusalKind;
use std::fmt::Write as _;
use store::file::BarFile;
use store::path::{PathParts, Timeframe, YearMonth};

/// The most instants one request may ask for.
///
/// Bounded by the request target, not by taste: an instant is ten digits and
/// a comma, and [`super::MAX_REQUEST_TARGET_BYTES`] is 8 KiB, so 400 leaves
/// room for the other fields. `/db` asks for its page in chunks of at most
/// this many.
pub const MAX_SPOT_TIMES: usize = 400;

/// The store segment an option underlying's own price series lives under.
///
/// An index underlying (`FNO_INDEX_UNDERLYINGS`, five names) has no cash
/// equity and its spot is the index; every other F&O underlying is a share
/// and its spot is its cash series (`CLAUDE.md` §1, D-0682). Not a fallback:
/// the CASH answer is the definition of a stock option's underlying, and the
/// response names the path it read, so a store that holds no such series says
/// `no_spot_file` against that path.
#[must_use]
pub fn underlying_segment(symbol: &str) -> &'static str {
    if brutex_core::universe::FNO_INDEX_UNDERLYINGS.contains(&symbol) {
        "INDEX"
    } else {
        "CASH"
    }
}

/// One instant's answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Spot {
    /// The underlying's close, in paisa, of the bar stamped exactly at `t`.
    At(i64),
    /// The file is held and has no bar at `t`: the records either side
    /// bracket it, which is what makes the absence a fact and not a guess.
    NoBar,
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
    /// The close, in paisa, when one was found at exactly `t`.
    #[must_use]
    pub const fn close(&self) -> Option<i64> {
        match self {
            Self::At(close) => Some(*close),
            _ => None,
        }
    }

    /// The wire's reason word, `None` for a close.
    #[must_use]
    pub const fn why(&self) -> Option<&'static str> {
        match self {
            Self::At(_) => None,
            Self::NoBar => Some("no_spot_bar"),
            Self::NoFile => Some("no_spot_file"),
            Self::Unreadable => Some("spot_unreadable"),
            Self::OrderBroken => Some("spot_order_broken"),
        }
    }
}

/// The underlying's close at exactly `ts_micros`, read from `file`.
///
/// # Errors
///
/// The store's refusal for a record the answer needed and could not read.
pub fn close_at(file: &BarFile, ts_micros: i64) -> Result<Spot, store::file::StoreError> {
    let at = file.first_at_or_after(ts_micros)?;
    let next = if at < file.header().n_valid {
        let bar = file.read_record(at)?;
        if bar.ts_micros == ts_micros {
            return Ok(Spot::At(bar.close));
        }
        Some(bar.ts_micros)
    } else {
        None
    };
    let previous = match at.checked_sub(1) {
        Some(index) => Some(file.read_record(index)?.ts_micros),
        None => None,
    };
    Ok(verdict(previous, next, ts_micros))
}

/// What a miss at `ts_micros` means, from the stamps either side of where the
/// bisection landed.
///
/// THE ABSENCE IS BRACKETED, NOT ASSUMED: `previous < ts < next`, either side
/// allowed to be the file's edge. A real bar is stamped after 1970, so a zero
/// stamp is the zero-filled extent `store::file` documents, where the
/// bisection's premise (strictly increasing stamps) does not hold — and then
/// neither a hit nor an absence is stated.
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

/// Everything `/spot.json` reads off the query string. Every field refuses
/// rather than defaults: the rung in particular has NO default here, because
/// a spot read at a rung the option was not read at is a different instant's
/// close.
#[derive(Debug)]
pub struct SpotAsk {
    vendor: brutex_core::vendor::Vendor,
    exchange: String,
    symbol: String,
    timeframe: Timeframe,
    month: YearMonth,
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
            if times.len() == MAX_SPOT_TIMES {
                return Err(format!(
                    "more than {MAX_SPOT_TIMES} instants in t; ask in chunks"
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
            symbol,
            timeframe,
            month,
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
        let opened = crate::bars::open_classified(
            store_root,
            PathParts {
                vendor: self.vendor,
                exchange: &self.exchange,
                segment: underlying_segment(&self.symbol),
                symbol: &self.symbol,
                contract: None,
                timeframe: self.timeframe,
                month: self.month,
                file: store::path::FileKind::Bars,
            },
        );
        let file = match opened {
            Ok(file) => file,
            Err(refusal) => {
                return match refusal.kind {
                    OpenRefusalKind::Invalid => Err(refusal.message),
                    OpenRefusalKind::Absent => {
                        Ok((vec![Spot::NoFile; self.times.len()], Vec::new()))
                    }
                    OpenRefusalKind::Unreadable => Ok((
                        vec![Spot::Unreadable; self.times.len()],
                        vec![refusal.message],
                    )),
                };
            }
        };
        let mut faults = Vec::new();
        let spots = self
            .times
            .iter()
            .map(|&ts| {
                close_at(&file, ts).unwrap_or_else(|why| {
                    faults.push(format!(
                        "{} t={}: {why}",
                        file.path().display(),
                        ts / 1_000_000
                    ));
                    Spot::Unreadable
                })
            })
            .collect();
        Ok((spots, faults))
    }

    /// The response body for `spots` and `faults`.
    #[must_use]
    pub fn render(&self, spots: &[Spot], faults: &[String]) -> String {
        let mut rows = String::with_capacity(spots.len() * 48 + 2);
        rows.push('[');
        for (n, (ts, spot)) in self.times.iter().zip(spots).enumerate() {
            if n > 0 {
                rows.push(',');
            }
            let close = spot
                .close()
                .map_or_else(|| "null".to_owned(), |paisa| paisa.to_string());
            let why = spot
                .why()
                .map_or_else(|| "null".to_owned(), render::json_string);
            let _ = write!(
                rows,
                r#"{{"t":{},"close":{close},"why":{why}}}"#,
                ts / 1_000_000
            );
        }
        rows.push(']');
        format!(
            r#"{{"exchange":{},"segment":{},"symbol":{},"timeframe":{},"month":{},"faults":{},"spots":{rows}}}"#,
            render::json_string(&self.exchange),
            render::json_string(underlying_segment(&self.symbol)),
            render::json_string(&self.symbol),
            render::json_string(self.timeframe.as_str()),
            render::json_string(&self.month.to_string()),
            if faults.is_empty() {
                "null".to_owned()
            } else {
                render::json_string(&faults.join("; "))
            }
        )
    }
}

/// `GET /spot.json?feed&exchange&symbol&timeframe&month&t=<s>,<s>…`
///
/// 400 for a malformed question, 206 when any record would not read (the
/// answers are real and the set is not whole, as `/bars.json` says it), 200
/// otherwise — including when every answer is a named absence.
pub async fn spot_json(
    axum::extract::State(site): axum::extract::State<Loaded>,
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
    let asked = match SpotAsk::parse(uri.query().unwrap_or("")) {
        Ok(asked) => asked,
        Err(why) => {
            return (
                axum::http::StatusCode::BAD_REQUEST,
                json,
                format!(r#"{{"error":{}}}"#, render::json_string(&why)),
            );
        }
    };
    let (spots, faults) = match asked.answer(&site.store_root) {
        Ok(answered) => answered,
        Err(why) => {
            return (
                axum::http::StatusCode::BAD_REQUEST,
                json,
                format!(r#"{{"error":{}}}"#, render::json_string(&why)),
            );
        }
    };
    let status = if faults.is_empty() {
        axum::http::StatusCode::OK
    } else {
        axum::http::StatusCode::PARTIAL_CONTENT
    };
    (status, json, asked.render(&spots, &faults))
}

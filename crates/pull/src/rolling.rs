//! Dhan's ATM-relative expired-options driver.
//!
//! # Why this is not [`crate::chain`]
//!
//! `chain` walks a month by NAME: ask which expiries it held, then which
//! contracts each expiry held, then fetch each contract by the name the vendor
//! gave back. Groww publishes exactly that and it is what `POST /pull/fno`
//! builds.
//!
//! Dhan publishes nothing of the sort. There is no expiry list and there are no
//! contract names — its expired-options endpoint is addressed by *shape*:
//!
//! ```text
//! POST /v2/charts/rollingoption
//!   securityId      the UNDERLYING's id, never a contract's
//!   instrument      OPTIDX | OPTSTK
//!   expiryFlag      WEEK | MONTH        <- a cadence, not a date
//!   expiryCode      1 near | 2 next | 3 far
//!   strike          ATM | ATM+10 | ATM-3   <- relative to spot, not a price
//!   drvOptionType   CALL | PUT
//! ```
//!
//! So there is nothing to discover and the walk has no first step. The contract
//! set is a CROSS PRODUCT of the descriptor's own lists, and every member of it
//! is a request that can be built without asking the vendor anything first.
//!
//! Trying to serve Dhan through `chain` is what produced the 502s of
//! 2026-08-19: `chain::month` asks for `by_name()` discovery, Dhan's descriptor
//! answers `None`, and the walk refused before a single bar could be fetched.
//! The refusal was correct. The request was the wrong shape.
//!
//! # What one answer carries that a bar cannot hold
//!
//! `data.ce` and `data.pe` each carry `open`, `high`, `low`, `close`, `volume`
//! and `oi` — which [`store::format::Bar`] has columns for — and also `iv` and
//! `spot`, which it does not and must not. Those two land in
//! [`store::format::Overlay`], the sidecar keyed by the same stamp. See its
//! header for why widening `Bar` would have retired every file on disk.
//!
//! # Cost
//!
//! O(1) per request built and O(1) per row read: one pass over parallel arrays
//! with an index, no search and no allocation per row beyond the rows kept.
//! The NUMBER of requests is the cross product and is bounded by the
//! descriptor's own lists — 21 index offsets × 2 sides × 2 cadences × 3
//! ordinals — never by anything a vendor answers.

use crate::vendor::{PriceScale, RollingSpec};
use store::format::{Bar, OI_NULL, Overlay};

/// The expiry a rolling answer does NOT carry, computed from what it does.
///
/// # Why this has to be computed at all
///
/// The request asks by cadence and ordinal — `expiryFlag` WEEK|MONTH,
/// `expiryCode` 1 near | 2 next | 3 far — and the answer carries a strike and a
/// timestamp. Neither names a DATE. But `store::path` files a derivative under
/// `Kind::Option { expiry, strike, side }`, so without a date a contract has no
/// place to go: every expiry of a month would render one path and append into
/// one file, which is the collision the overlay's own header describes.
///
/// # Why `costs` and not a table here
///
/// `costs::expiry` already holds the weekly and monthly regimes, with the dates
/// they were verified from and the citations behind them. A second table in
/// this crate would be a second answer to "when did that contract expire", and
/// the copy that drifts is the one nobody remembers exists. D-0206.
///
/// # Errors
///
/// [`RollingError::NoExpiry`] when the underlying is not one the calendar
/// knows, when the day is before the regime's verified floor, or when the
/// cadence has been withdrawn for that slot. A withdrawn weekly is a REFUSAL
/// and not an empty answer — `costs::expiry` is explicit about that, and
/// guessing a date for a contract that did not exist is how a backtest gets
/// bars filed under a series the exchange never listed.
///
/// # Cost
///
/// O(1). One slot lookup and one calendar step; nothing scans.
pub fn expiry_of(
    underlying: &str,
    flag: &str,
    code: &str,
    on: crate::session::Day,
) -> Result<brutex_core::instrument::Expiry, RollingError> {
    let symbol =
        brutex_core::symbol::Symbol::new(underlying).map_err(|_| RollingError::NoExpiry {
            why: "the underlying is not a symbol this build knows",
        })?;
    let slot = costs::venue::swept_slot(symbol).map_err(|_| RollingError::NoExpiry {
        why: "the underlying has no expiry regime recorded",
    })?;
    let day = costs::day::TradeDay::new(on.year(), on.month(), on.day()).map_err(|_| {
        RollingError::NoExpiry {
            why: "the bar's own day is not a real date",
        }
    })?;

    // THE ORDINAL IS WALKED, NOT INDEXED. "Next" is "the one after near", and
    // the calendar answers only "the next on or after this day" — so stepping
    // is the honest way to reach the second and the third, and a step that
    // finds nothing is a refusal rather than a guess.
    let steps: u8 = match code {
        "1" => 1,
        "2" => 2,
        "3" => 3,
        _ => {
            return Err(RollingError::NoExpiry {
                why: "the expiry ordinal is not one this vendor serves",
            });
        }
    };
    let mut at = day;
    let mut found = None;
    for _ in 0..steps {
        let next = match flag {
            "WEEK" => costs::expiry::next_weekly_expiry(slot, at)
                .map_err(|_| RollingError::NoExpiry {
                    why: "the day is before this weekly regime was verified from",
                })?
                .ok_or(RollingError::NoExpiry {
                    why: "this weekly was withdrawn for that slot, so no contract existed",
                })?,
            "MONTH" => costs::expiry::next_monthly_expiry(slot, at).map_err(|_| {
                RollingError::NoExpiry {
                    why: "the day is before this monthly regime was verified from",
                }
            })?,
            _ => {
                return Err(RollingError::NoExpiry {
                    why: "the expiry cadence is not one this vendor serves",
                });
            }
        };
        found = Some(next);
        // ONE DAY PAST THE ONE JUST FOUND, so the next step cannot return it
        // again. `next_*_expiry` answers "on or after", so stepping from the
        // expiry itself would stand still.
        at = next.plus_days(1).map_err(|_| RollingError::NoExpiry {
            why: "the calendar ran past the last day it can represent",
        })?;
    }
    let settled = found.ok_or(RollingError::NoExpiry {
        why: "no expiry was reached",
    })?;
    brutex_core::instrument::Expiry::new(settled.year(), settled.month(), settled.day()).map_err(
        |_| RollingError::NoExpiry {
            why: "the calendar produced a date this store cannot name",
        },
    )
}

/// One rolling-option request: a shape, not a contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ask {
    /// The UNDERLYING's vendor id. Never a contract's — there is no contract
    /// id to have, which is the whole reason this module exists.
    pub security_id: String,
    /// `OPTIDX` or `OPTSTK`, from the spec's own words.
    pub instrument: &'static str,
    /// `WEEK` or `MONTH`.
    pub expiry_flag: &'static str,
    /// `1` near, `2` next, `3` far.
    pub expiry_code: &'static str,
    /// `ATM`, `ATM+10`, `ATM-3` — relative to spot at that moment.
    pub strike: &'static str,
    /// `CALL` or `PUT`.
    pub side: &'static str,
    /// The vendor's minute interval — `1`, `5`, `15`, `30`, `60`.
    pub interval: &'static str,
    /// First day, `YYYY-MM-DD`.
    pub from: String,
    /// The `toDate` **as it goes on the wire**, which Dhan documents as
    /// NON-INCLUSIVE — `docs/14-expired-options-data.md`, request table:
    /// *"End date (non-inclusive)"*.
    ///
    /// # Why this field is the wire value and not the operator's last day
    ///
    /// The two differ by a day, and getting it wrong loses the last day of
    /// every window silently: the vendor answers, the rows parse, the books
    /// balance, and one session is simply missing. `fetch::wire_end` already
    /// owns that conversion for the bars endpoint — the same vendor, the same
    /// rule — so the caller passes what that function returns rather than this
    /// module growing a second answer to one question.
    ///
    /// At most [`RollingSpec::max_days_per_call`] after [`Self::from`];
    /// splitting a wider window is the caller's job.
    pub to: String,
}

/// Why a rolling answer could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RollingError {
    /// The body was not JSON.
    NotJson,
    /// The side's object was absent. `CALL` asked and no `ce` returned.
    NoSide {
        /// Which key was looked for — `ce` or `pe`.
        key: &'static str,
    },
    /// A field was absent or not an array.
    NotAnArray {
        /// Which field.
        field: &'static str,
    },
    /// The parallel arrays are not the same length.
    ///
    /// **A refusal, never a truncation.** Reading `min(len)` rows would file
    /// bars whose price came from one row and whose volume came from another,
    /// and nothing downstream could ever detect it.
    Ragged {
        /// Which field disagreed.
        field: &'static str,
        /// How long the timestamp array was.
        stamps: usize,
        /// How long this field was.
        found: usize,
    },
    /// The contract's expiry could not be established.
    NoExpiry {
        /// Which step could not answer.
        why: &'static str,
    },
    /// A price would not convert to paisa without loss or overflow.
    Unrepresentable {
        /// Which field.
        field: &'static str,
    },
}

impl core::fmt::Display for RollingError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NotJson => f.write_str("the rolling answer is not JSON"),
            Self::NoSide { key } => write!(
                f,
                "the rolling answer carries no `{key}` object, so the side that \
                 was asked for is not in it"
            ),
            Self::NotAnArray { field } => write!(
                f,
                "the rolling answer's `{field}` is absent or is not an array"
            ),
            Self::Ragged {
                field,
                stamps,
                found,
            } => write!(
                f,
                "the rolling answer holds {stamps} timestamp(s) and {found} \
                 `{field}` value(s). These arrays are read in step, so a \
                 mismatch is refused rather than truncated: taking the shorter \
                 would file a bar whose price came from one minute and whose \
                 volume came from another, and nothing downstream could detect it"
            ),
            Self::NoExpiry { why } => write!(
                f,
                "this answer's contract has no expiry date to file it under — \
                 {why}. The request asked by cadence and ordinal, and the answer \
                 carries a strike and a timestamp, so the date is computed or it \
                 does not exist"
            ),
            Self::Unrepresentable { field } => {
                write!(f, "a `{field}` value does not fit paisa as an i64")
            }
        }
    }
}

/// Which JSON key holds a side's rows.
///
/// The request says `CALL`; the answer says `ce`. Mapping it here rather than
/// at the call site keeps the vendor's two spellings of one idea in one place.
#[must_use]
pub const fn side_key(side: &str) -> &'static str {
    // `match` on bytes because `str` equality is not `const`.
    if side.len() == 4 { "ce" } else { "pe" }
}

/// The endpoint one rolling request is posted to.
///
/// # Why the path is all literals here
///
/// Dhan carries every varying value in the BODY, not in the URL — the
/// underlying, the cadence, the ordinal, the offset and the side are all
/// fields of the JSON. So the path is the descriptor's segments joined to the
/// base, and nothing in it depends on the ask. A feed that later put one of
/// them in the path would be a row in `RollingSpec::path`, not an edit here.
///
/// # Cost
///
/// One allocation, proportional to the URL. O(1) in the request.
#[must_use]
pub fn url(spec: &RollingSpec, base_url: &str) -> String {
    let mut out = String::from(base_url);
    for segment in spec.path {
        out.push('/');
        match *segment {
            crate::vendor::PathSegment::Literal(word) => out.push_str(word),
            // A ROLLING PATH IS ALL LITERALS AT THE ONE FEED THAT HAS ONE, and
            // a placeholder left standing is more honest than a value invented
            // for it — the request then fails at the vendor naming the segment
            // rather than here naming nothing.
            crate::vendor::PathSegment::Value { placeholder, .. } => {
                out.push_str(placeholder);
            }
        }
    }
    out
}

/// The POST body for one ask.
///
/// # Why the fields are written in a fixed order
///
/// `CLAUDE.md` §3 rule 5: the same inputs must produce the same bytes. A map
/// iteration order would make an identical request serialise two ways across
/// runs, and a receipt that quotes the body would then differ for no reason a
/// reader could act on.
#[must_use]
pub fn body(spec: &RollingSpec, ask: &Ask) -> String {
    let mut out = String::with_capacity(320);
    out.push('{');
    push_pair(&mut out, "exchangeSegment", spec.segment_word, true);
    push_pair(&mut out, "instrument", ask.instrument, false);
    push_pair(&mut out, "securityId", &ask.security_id, false);
    push_pair(&mut out, "expiryFlag", ask.expiry_flag, false);
    push_pair(&mut out, "expiryCode", ask.expiry_code, false);
    push_pair(&mut out, "strike", ask.strike, false);
    push_pair(&mut out, "drvOptionType", ask.side, false);
    push_pair(&mut out, "interval", ask.interval, false);
    push_pair(&mut out, "fromDate", &ask.from, false);
    push_pair(&mut out, "toDate", &ask.to, false);
    // EVERY FIELD THIS BUILD FILES, NAMED. The vendor defaults `requiredData`
    // to a subset, and a default is a decision made elsewhere that changes what
    // lands on disk. `iv` and `spot` are here because the overlay exists for
    // exactly them.
    // `strike` IS NOT DECORATION HERE — IT IS THE CONTRACT'S NAME.
    //
    // The request asks by OFFSET (`ATM+10`), which is a question. Only the
    // answer's `strike` array says which price that question resolved to, and
    // without it there is nothing to file the contract under. It was absent
    // from this list and absent from `read`, and `roll_one` used the
    // underlying's SPOT in its place — so all 21 offsets of one expiry resolved
    // to one path and appended into one file.
    out.push_str(
        r#","requiredData":["open","high","low","close","volume","oi","iv","spot","strike"]"#,
    );
    out.push('}');
    out
}

/// One `"key":"value"` pair, JSON-escaped.
fn push_pair(out: &mut String, key: &str, value: &str, first: bool) {
    if !first {
        out.push(',');
    }
    out.push('"');
    out.push_str(key);
    out.push_str("\":\"");
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            other => out.push(other),
        }
    }
    out.push('"');
}

/// One row of a rolling answer: the bar, and what the bar has no column for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Row {
    /// The OHLCV row.
    pub bar: Bar,
    /// The vendor-stated spot and implied volatility for the same stamp.
    pub overlay: Overlay,
    /// The strike this offset RESOLVED TO, in paisa, or `None` when the vendor
    /// sent no `strike` array.
    ///
    /// # Why it is not on [`Overlay`]
    ///
    /// The overlay is a per-BAR sidecar — spot and implied volatility move
    /// every minute. A strike does not: it is a property of the CONTRACT, and
    /// it is what names the file every one of these bars is written into. Two
    /// different lifetimes, so two different places.
    ///
    /// `None` rather than a default, because a contract whose strike is unknown
    /// cannot be filed at all and the caller must refuse rather than pick one.
    /// See `api::server::roll_one`.
    pub strike: Option<i64>,
}

/// Reads one side of a rolling answer into rows.
///
/// # Errors
///
/// [`RollingError`] for a body that is not JSON, a side that is absent, a field
/// that is not an array, arrays that disagree in length, or a price that does
/// not fit paisa.
///
/// # Cost
///
/// One pass, indexed. No search, and the only allocation is the answer itself.
pub fn read(body: &str, side: &str, scale: PriceScale) -> Result<Vec<Row>, RollingError> {
    let root: serde_json::Value = serde_json::from_str(body).map_err(|_| RollingError::NotJson)?;
    // THE ENVELOPE IS OPTIONAL, exactly as `fno::names` treats it, and for the
    // same reason: one reader for a vendor that wraps and one that does not.
    let held = root.get("data").unwrap_or(&root);
    let key = side_key(side);
    let one = held.get(key).ok_or(RollingError::NoSide { key })?;

    let stamps = array(one, "timestamp")?;
    let open = same_length(one, "open", stamps.len())?;
    let high = same_length(one, "high", stamps.len())?;
    let low = same_length(one, "low", stamps.len())?;
    let close = same_length(one, "close", stamps.len())?;
    let volume = same_length(one, "volume", stamps.len())?;
    // OI, IV AND SPOT ARE OPTIONAL AND THE OTHERS ARE NOT. The vendor's own
    // schema marks every field `Required: No`, and measurement is what decides
    // which are really there: a contract with no open interest is ordinary,
    // while a bar with no close is not a bar. An absent optional array reads as
    // "the vendor stated none", which is what the null sentinels are for.
    let oi = optional(one, "oi", stamps.len())?;
    let iv = optional(one, "iv", stamps.len())?;
    let spot = optional(one, "spot", stamps.len())?;
    // OPTIONAL AT THE PARSE, REQUIRED AT THE FILE. Absent here is a fact about
    // the answer and reads as `None`; it becomes a refusal one layer up, where
    // the contract is named and the absence actually bites.
    let strike = optional(one, "strike", stamps.len())?;

    let mut rows = Vec::with_capacity(stamps.len());
    for at in 0..stamps.len() {
        let ts_secs = number(stamps, at);
        let bar = Bar {
            // SECONDS ON THE WIRE, MICROSECONDS IN THE STORE. The same
            // conversion `TimestampEncoding::EpochSecondsUtc` names, done here
            // because this reader does not go through the CSV path that owns it.
            ts_micros: ts_secs.saturating_mul(1_000_000),
            open: paisa(open, at, scale, "open")?,
            high: paisa(high, at, scale, "high")?,
            low: paisa(low, at, scale, "low")?,
            close: paisa(close, at, scale, "close")?,
            volume: number(volume, at),
            open_interest: oi.map_or(OI_NULL, |a| number(a, at)),
        };
        let overlay = Overlay {
            ts_micros: bar.ts_micros,
            spot: match spot {
                Some(a) => paisa(a, at, scale, "spot")?,
                None => OI_NULL,
            },
            // IV IN MILLIONTHS, and the multiply happens on the way in so the
            // store never holds a float. See `Overlay`'s header for why a
            // volatility is stored as an integer despite being a statistic.
            iv_micros: iv.map_or(OI_NULL, |a| a.get(at).map_or(OI_NULL, micros_of)),
        };
        // THE STRIKE FOR THIS STAMP. A rolling series is ATM-relative, so the
        // resolved strike genuinely can differ between two bars of one answer
        // when the underlying crosses a step; the caller takes the FIRST and
        // that choice is recorded there, not silently made here.
        let resolved = match strike {
            Some(a) => Some(paisa(a, at, scale, "strike")?),
            None => None,
        };
        rows.push(Row {
            bar,
            overlay,
            strike: resolved,
        });
    }
    Ok(rows)
}

/// A field that must be an array.
fn array<'a>(
    holder: &'a serde_json::Value,
    field: &'static str,
) -> Result<&'a Vec<serde_json::Value>, RollingError> {
    holder
        .get(field)
        .and_then(serde_json::Value::as_array)
        .ok_or(RollingError::NotAnArray { field })
}

/// A required array, checked against the timestamp count.
fn same_length<'a>(
    holder: &'a serde_json::Value,
    field: &'static str,
    stamps: usize,
) -> Result<&'a Vec<serde_json::Value>, RollingError> {
    let found = array(holder, field)?;
    if found.len() == stamps {
        Ok(found)
    } else {
        Err(RollingError::Ragged {
            field,
            stamps,
            found: found.len(),
        })
    }
}

/// An optional array. Absent is `None`; present-but-ragged is still a refusal.
fn optional<'a>(
    holder: &'a serde_json::Value,
    field: &'static str,
    stamps: usize,
) -> Result<Option<&'a Vec<serde_json::Value>>, RollingError> {
    match holder.get(field) {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(_) => same_length(holder, field, stamps).map(Some),
    }
}

/// One integer out of an array, or zero.
///
/// Zero rather than a refusal because these are counts — a volume or an open
/// interest — where the vendor writing nothing and writing zero mean the same
/// thing, and the NULL case is handled one level up by the array being absent.
fn number(list: &[serde_json::Value], at: usize) -> i64 {
    let Some(cell) = list.get(at) else {
        return 0;
    };
    if let Some(whole) = cell.as_i64() {
        return whole;
    }
    // A COUNT WRITTEN WITH A DECIMAL POINT IS STILL A COUNT — `1234.0`. Read
    // through its TEXT rather than through an `f64`, because `float_arithmetic`
    // is denied workspace-wide and because the text is what the vendor sent:
    // `csv::paisa` shifts two places and the count is the whole part of that.
    crate::csv::paisa(&cell.to_string()).map_or(0, |hundredths| hundredths / 100)
}

/// One price out of an array, in paisa.
fn paisa(
    list: &[serde_json::Value],
    at: usize,
    scale: PriceScale,
    field: &'static str,
) -> Result<i64, RollingError> {
    let cell = list
        .get(at)
        .ok_or(RollingError::Unrepresentable { field })?;
    match scale {
        // Already paisa: an integer count, and nothing to convert.
        PriceScale::Paisa => cell.as_i64().ok_or(RollingError::Unrepresentable { field }),
        // THE TEXT IS THE TRUTH, and `csv::paisa` owns the rule — the same
        // sentence `http::decode_body` writes over the same conversion.
        //
        // Not `(x * 100.0).round()`. `float_arithmetic` is denied workspace-
        // wide, and the reason outlives the lint: a rupee figure that arrived
        // as text has an exact decimal the vendor wrote, and routing it through
        // an f64 to shift two places introduces a representation error into a
        // value that had none. `CLAUDE.md` §7 snaps once, half-up, at the write
        // boundary — and this is that boundary.
        PriceScale::Rupees => {
            crate::csv::paisa(&cell.to_string()).ok_or(RollingError::Unrepresentable { field })
        }
    }
}

/// A volatility as millionths, or the null sentinel when it will not read.
///
/// # Why this is not `csv::paisa`
///
/// That function shifts exactly TWO places, because a price has two. An
/// implied volatility does not: Dhan writes `0.125`, and three decimals through
/// a two-place reader is a refusal — measured, and it returned the null
/// sentinel for every volatility the vendor sent.
///
/// # Why it is not `(v * 1_000_000.0).round()` either
///
/// `float_arithmetic` is denied workspace-wide, and the reason outlives the
/// lint. The vendor wrote an exact decimal; routing it through an `f64` to
/// shift six places introduces a representation error into a value that had
/// none, and `CLAUDE.md` §3 rule 5 wants the same body to yield the same bytes
/// on every rerun. So the shift is done on the TEXT, in integers.
///
/// Half-up at the seventh decimal, which is the same rule `CLAUDE.md` §7 gives
/// for snapping a price — one rounding rule for the whole product.
///
/// # Cost
///
/// One pass over at most a few dozen characters. No allocation.
fn micros_of(cell: &serde_json::Value) -> i64 {
    let text = match cell {
        serde_json::Value::String(text) => text.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        _ => return OI_NULL,
    };
    shift_six(text.trim()).unwrap_or(OI_NULL)
}

/// A decimal string as millionths, half-up, or `None` when it will not read.
fn shift_six(text: &str) -> Option<i64> {
    const PLACES: usize = 6;
    let (negative, digits) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text.strip_prefix('+').unwrap_or(text)),
    };
    let (whole, fraction) = match digits.split_once('.') {
        Some((w, f)) => (w, f),
        None => (digits, ""),
    };
    if whole.is_empty() && fraction.is_empty() {
        return None;
    }
    if !whole.bytes().all(|b| b.is_ascii_digit()) || !fraction.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }

    let mut out: i64 = if whole.is_empty() {
        0
    } else {
        whole.parse::<i64>().ok()?
    };
    for at in 0..PLACES {
        out = out.checked_mul(10)?;
        let digit = fraction
            .as_bytes()
            .get(at)
            .map_or(0, |b| i64::from(*b - b'0'));
        out = out.checked_add(digit)?;
    }
    // HALF-UP ON THE SEVENTH, and only when there is a seventh. A value with
    // six or fewer decimals is exact and must not be nudged.
    if fraction
        .as_bytes()
        .get(PLACES)
        .is_some_and(|next| *next >= b'5')
    {
        out = out.checked_add(1)?;
    }
    if negative {
        out.checked_neg()
    } else {
        Some(out)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use crate::vendor::{Feed, Transport};

    fn spec() -> RollingSpec {
        let Transport::Http(dhan) = Feed::Dhan.descriptor().transport else {
            panic!("Dhan is an HTTP broker");
        };
        dhan.fno.by_offset().expect("Dhan is addressed by offset")
    }

    fn ask() -> Ask {
        Ask {
            security_id: "13".to_owned(),
            instrument: "OPTIDX",
            expiry_flag: "WEEK",
            expiry_code: "1",
            strike: "ATM+10",
            side: "CALL",
            interval: "1",
            from: "2025-09-01".to_owned(),
            to: "2025-09-30".to_owned(),
        }
    }

    /// **THE BODY NAMES EVERY FIELD, IN A FIXED ORDER, EVERY TIME.**
    ///
    /// Order is not taste. `CLAUDE.md` §3 rule 5 makes a rerun byte-identical,
    /// and a body serialised from a map would order two ways across runs — so a
    /// receipt quoting it would differ for a reason no reader could act on.
    #[test]
    fn the_request_body_is_the_same_bytes_every_time_and_asks_for_iv_and_spot() {
        let once = body(&spec(), &ask());
        let twice = body(&spec(), &ask());
        assert_eq!(once, twice, "same ask, same bytes");

        for field in [
            "exchangeSegment",
            "instrument",
            "securityId",
            "expiryFlag",
            "expiryCode",
            "strike",
            "drvOptionType",
            "interval",
            "fromDate",
            "toDate",
        ] {
            assert!(once.contains(field), "the body names `{field}`: {once}");
        }
        // THE TWO THE OVERLAY EXISTS FOR. `requiredData` defaults to a subset,
        // and a default is a decision taken elsewhere that changes what lands
        // on disk.
        assert!(once.contains("\"iv\""), "iv is asked for: {once}");
        assert!(once.contains("\"spot\""), "spot is asked for: {once}");
        assert!(
            serde_json::from_str::<serde_json::Value>(&once).is_ok(),
            "and it is JSON: {once}"
        );
    }

    /// **RAGGED ARRAYS ARE REFUSED, NEVER TRUNCATED.**
    ///
    /// This is the one that would be invisible. Reading `min(len)` rows yields
    /// bars that parse, store, checksum and read back — with a close from one
    /// minute and a volume from another. No CRC catches it and no later gate
    /// can, because the bytes are internally consistent. The only place it can
    /// be caught is here, at the moment the two lengths disagree.
    #[test]
    fn arrays_that_disagree_in_length_are_refused_rather_than_zipped_short() {
        let ragged = r#"{"data":{"ce":{
            "timestamp":[1,2,3],
            "open":[10.0,11.0,12.0],
            "high":[10.5,11.5,12.5],
            "low":[9.5,10.5,11.5],
            "close":[10.2,11.2],
            "volume":[100,200,300]
        }}}"#;
        let got = read(ragged, "CALL", PriceScale::Rupees);
        assert_eq!(
            got,
            Err(RollingError::Ragged {
                field: "close",
                stamps: 3,
                found: 2
            }),
            "and it names the field and both lengths"
        );
        let said = RollingError::Ragged {
            field: "close",
            stamps: 3,
            found: 2,
        }
        .to_string();
        assert!(
            said.contains("refused rather than truncated"),
            "the sentence says what it did NOT do: {said}"
        );
    }

    /// **RUPEES BECOME PAISA EXACTLY, THROUGH THE TEXT.**
    ///
    /// `24650.05` is the case that separates a text conversion from a float
    /// one: `24650.05 * 100.0` is `2465004.9999...` in binary, so a float path
    /// rounds to 2,465,005 only by luck of the rounding mode and silently loses
    /// a paisa on some values. `csv::paisa` reads the decimal the vendor wrote.
    #[test]
    fn a_price_reaches_the_store_with_its_paise_intact() {
        let one = r#"{"data":{"ce":{
            "timestamp":[1700000000],
            "open":[24650.05],"high":[24650.05],"low":[24650.05],"close":[24650.05],
            "volume":[7]
        }}}"#;
        let rows = read(one, "CALL", PriceScale::Rupees).expect("it reads");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].bar.close, 2_465_005, "exactly, not 2_465_004");
        assert_eq!(rows[0].bar.ts_micros, 1_700_000_000_000_000);
        assert_eq!(rows[0].bar.volume, 7);
    }

    /// **AN ABSENT OPTIONAL FIELD IS A NULL, NOT A ZERO.**
    ///
    /// Dhan marks every field optional. A contract whose `oi` the vendor did
    /// not state is ordinary; recording it as `0` would put a fabricated
    /// reading into a backtest that looks exactly like a real one. Same for a
    /// volatility, where zero is itself a real value a deep out-of-the-money
    /// option genuinely prints.
    #[test]
    fn a_field_the_vendor_did_not_send_is_null_and_not_zero() {
        let bare = r#"{"data":{"ce":{
            "timestamp":[1700000000],
            "open":[100.0],"high":[100.0],"low":[100.0],"close":[100.0],
            "volume":[1]
        }}}"#;
        let rows = read(bare, "CALL", PriceScale::Rupees).expect("it reads");
        assert_eq!(rows[0].bar.open_interest, OI_NULL, "absent OI is null");
        assert!(rows[0].bar.oi().is_none());
        assert_eq!(rows[0].overlay.spot, OI_NULL);
        assert_eq!(rows[0].overlay.iv_micros, OI_NULL);
        assert!(
            !rows[0].overlay.states_something(),
            "an overlay stating neither value is not worth a record"
        );
    }

    /// **IV AND SPOT LAND IN THE OVERLAY, KEYED BY THE BAR'S OWN STAMP.**
    ///
    /// The stamp is the join. Position would be faster and wrong the first time
    /// the session filter drops a bar on one side and not the other.
    #[test]
    fn iv_and_spot_land_beside_the_bar_under_the_same_stamp() {
        let full = r#"{"data":{"ce":{
            "timestamp":[1700000000],
            "open":[100.0],"high":[100.0],"low":[100.0],"close":[100.0],
            "volume":[1],"oi":[4200],"iv":[0.125],"spot":[24650.05]
        }}}"#;
        let rows = read(full, "CALL", PriceScale::Rupees).expect("it reads");
        let row = rows[0];
        assert_eq!(row.bar.open_interest, 4200);
        assert_eq!(
            row.overlay.ts_micros, row.bar.ts_micros,
            "the overlay is joined to the bar by STAMP, not by position"
        );
        assert_eq!(row.overlay.spot(), Some(2_465_005));
        // 0.125 -> 125,000 millionths. Through the text, so no float rounding.
        assert_eq!(row.overlay.iv(), Some(125_000));
        assert!(row.overlay.states_something());
    }

    /// **THE SIDE ASKED FOR IS THE SIDE READ.**
    ///
    /// One answer carries `ce` and `pe`. Reading the wrong one files a put's
    /// prices under a call's contract, and every downstream check passes.
    #[test]
    fn the_put_is_read_from_pe_and_the_call_from_ce() {
        let both = r#"{"data":{
            "ce":{"timestamp":[1],"open":[1.0],"high":[1.0],"low":[1.0],"close":[1.0],"volume":[1]},
            "pe":{"timestamp":[1],"open":[2.0],"high":[2.0],"low":[2.0],"close":[2.0],"volume":[2]}
        }}"#;
        let call = read(both, "CALL", PriceScale::Rupees).expect("ce reads");
        let put = read(both, "PUT", PriceScale::Rupees).expect("pe reads");
        assert_eq!(call[0].bar.close, 100, "the call came from ce");
        assert_eq!(put[0].bar.close, 200, "the put came from pe");
        assert_eq!(side_key("CALL"), "ce");
        assert_eq!(side_key("PUT"), "pe");

        // AND AN ABSENT SIDE IS NAMED, not read as an empty answer. A month
        // that returned no puts and a month whose puts were not asked for are
        // different facts.
        let only_ce = r#"{"data":{"ce":{"timestamp":[],"open":[],"high":[],"low":[],"close":[],"volume":[]}}}"#;
        assert_eq!(
            read(only_ce, "PUT", PriceScale::Rupees),
            Err(RollingError::NoSide { key: "pe" })
        );
    }

    /// **THE SIX-PLACE SHIFT IS EXACT, AND ROUNDS HALF-UP ONLY WHEN IT MUST.**
    ///
    /// The first version of this used `csv::paisa`, which shifts exactly two
    /// places because a price has two. A volatility does not — Dhan writes
    /// `0.125` — so every volatility the vendor sent read as absent. The test
    /// above caught it; these pin the edges it would not have.
    #[test]
    fn a_volatility_shifts_six_places_without_a_float_taking_part() {
        let of = |t: &str| shift_six(t);
        assert_eq!(of("0.125"), Some(125_000), "three decimals, exact");
        assert_eq!(of("0"), Some(0), "zero is a real volatility");
        assert_eq!(of("1"), Some(1_000_000), "no decimal point at all");
        assert_eq!(of(".5"), Some(500_000), "no whole part");
        assert_eq!(of("0.000001"), Some(1), "the last place this can hold");
        // HALF-UP ON THE SEVENTH, and NOT on a value that has only six.
        assert_eq!(of("0.0000005"), Some(1), "seventh place rounds up");
        assert_eq!(of("0.0000004"), Some(0), "and down");
        assert_eq!(
            of("0.123456"),
            Some(123_456),
            "six decimals are exact and must not be nudged"
        );
        assert_eq!(of("-0.25"), Some(-250_000), "a sign is carried");
        // AND ANYTHING THAT IS NOT A DECIMAL IS REFUSED rather than guessed at.
        assert_eq!(of("abc"), None);
        assert_eq!(of(""), None);
        assert_eq!(of("1.2.3"), None);
        assert_eq!(of("9223372036854775807"), None, "overflow is not a value");
    }

    /// **THE VENDOR'S `toDate` IS NON-INCLUSIVE, AND THE SAME CONVERTER OWNS IT.**
    ///
    /// `docs/14-expired-options-data.md` states it in the request table, and
    /// `fetch::wire_end` already applies it for this vendor's bars endpoint.
    /// Two answers to one question is how the last day of every window goes
    /// missing while every count still balances.
    #[test]
    fn the_end_date_this_carries_is_the_exclusive_one_the_vendor_documents() {
        use crate::session::Day;
        use crate::vendor::{Feed, RangeEnd, Transport};

        let Transport::Http(dhan) = Feed::Dhan.descriptor().transport else {
            panic!("Dhan is an HTTP broker");
        };
        assert_eq!(
            dhan.range_end,
            RangeEnd::Exclusive,
            "the descriptor already records what the docs say"
        );
        let last = Day::new(2025, 9, 30).expect("a real day");
        let wire = crate::fetch::wire_end(last, dhan.range_end).expect("it converts");
        assert_eq!(
            wire.to_string(),
            "2025-10-01",
            "the day AFTER the operator's last, because the vendor excludes it \
             — an ask that carried 2025-09-30 would lose that whole session"
        );
    }

    /// **THE EXPIRY THE ANSWER DOES NOT CARRY IS COMPUTED, AND THE ORDINALS
    /// WALK FORWARD.**
    ///
    /// Near, next and far must be three DIFFERENT dates. `next_*_expiry`
    /// answers "on or after", so a walk that stepped from the expiry itself
    /// would stand still and file all three ordinals under one date — every
    /// contract of a month rendering one path and appending into one file.
    #[test]
    fn near_next_and_far_are_three_different_dates_in_calendar_order() {
        use crate::session::Day;

        let on = Day::new(2025, 9, 1).expect("a real day");
        let near = expiry_of("NIFTY", "WEEK", "1", on).expect("near resolves");
        let next = expiry_of("NIFTY", "WEEK", "2", on).expect("next resolves");
        let far = expiry_of("NIFTY", "WEEK", "3", on).expect("far resolves");

        assert!(near < next, "next is after near: {near:?} !< {next:?}");
        assert!(next < far, "far is after next: {next:?} !< {far:?}");

        // AND A MONTHLY IS A DIFFERENT ANSWER FROM A WEEKLY. One cadence
        // standing in for the other would file a monthly's bars under a
        // weekly's date, which no later check could detect.
        let monthly = expiry_of("NIFTY", "MONTH", "1", on).expect("monthly resolves");
        assert_ne!(
            monthly, near,
            "the monthly and the near weekly are not the same contract"
        );
    }

    /// **AN UNKNOWN UNDERLYING, CADENCE OR ORDINAL IS REFUSED, NEVER GUESSED.**
    ///
    /// Guessing a date for a contract the exchange never listed files bars
    /// under a series that did not exist — and they would read back as real.
    #[test]
    fn an_expiry_that_cannot_be_established_is_refused_and_says_which_step() {
        use crate::session::Day;

        let on = Day::new(2025, 9, 1).expect("a real day");
        for (u, flag, code) in [
            ("NOTASYMBOL", "WEEK", "1"),
            ("NIFTY", "FORTNIGHT", "1"),
            ("NIFTY", "WEEK", "4"),
        ] {
            let got = expiry_of(u, flag, code, on);
            assert!(
                matches!(got, Err(RollingError::NoExpiry { .. })),
                "{u}/{flag}/{code} is refused rather than guessed: {got:?}"
            );
        }
        let said = RollingError::NoExpiry {
            why: "the underlying has no expiry regime recorded",
        }
        .to_string();
        assert!(
            said.contains("computed or it does not exist"),
            "the sentence explains why there is no date to read: {said}"
        );
    }

    /// **THE CONTRACT SET IS THE DESCRIPTOR'S CROSS PRODUCT, AND IT IS BOUNDED.**
    ///
    /// Nothing a vendor answers changes how many requests a month costs. That
    /// is the difference from `chain`, where the count is whatever the expiry
    /// list returned — and it is why this driver's cost can be stated before it
    /// runs rather than discovered while it runs.
    #[test]
    fn the_request_count_is_fixed_by_the_descriptor_and_not_by_an_answer() {
        let s = spec();
        let index = s.offsets_for(s.index_word).len();
        let stock = s.offsets_for("OPTSTK").len();
        assert_eq!(index, 21, "ATM and ten either side");
        assert_eq!(stock, 7, "ATM and three either side");
        assert_eq!(s.sides.len(), 2);
        assert_eq!(s.expiry_flags.len(), 2, "WEEK and MONTH");
        assert_eq!(s.expiry_codes.len(), 3, "near, next, far");

        // 21 x 2 x 2 x 3 = 252 requests for one index month, known in advance.
        let per_index_month = index * s.sides.len() * s.expiry_flags.len() * s.expiry_codes.len();
        assert_eq!(per_index_month, 252);
        // And a stock month is a third of that, because the vendor serves a
        // narrower strike width — asking outside it returns nothing.
        assert_eq!(
            stock * s.sides.len() * s.expiry_flags.len() * s.expiry_codes.len(),
            84
        );
    }
}

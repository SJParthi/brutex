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
//!
//! **UNVERIFIED as a measurement.** The bound is argued from the
//! shape of the code and no bench in this workspace times it.
//! `CLAUDE.md` §3 rule 6: a structural argument is not a
//! measurement, however sound it is.

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
///
/// **UNVERIFIED as a measurement.** The bound is argued from the
/// shape of the code and no bench in this workspace times it.
/// `CLAUDE.md` §3 rule 6: a structural argument is not a
/// measurement, however sound it is.
pub fn expiry_of(
    underlying: &str,
    spec: &RollingSpec,
    flag: &str,
    code: &str,
    on: crate::session::Day,
) -> Result<brutex_core::instrument::Expiry, RollingError> {
    let (cadence, slot, day) = regime_of(underlying, spec, flag, on)?;

    // THE ORDINAL IS THE CODE'S POSITION IN THE ROW, NOT A SECOND SPELLING OF
    // IT.
    //
    // This was `match code { "1" => 1, "2" => 2, "3" => 3, _ => refuse }`, a
    // copy of `RollingSpec::expiry_codes` written where no row can reach it.
    // `server.rs` celebrates reading the DEPTH from the row — *"A vendor that
    // serves four gets four the day its row says so, with no edit in this
    // file"* — and a `"4"` added there reached this `_` arm **after** the
    // request had been built, sent, paid for out of the vendor's ceiling and
    // answered. The row said four; the resolver said three; the difference was
    // spent.
    //
    // Position is the honest reading: `expiry_codes` is ordered near-to-far by
    // its own doc, so the first is one step, the second is two. A code the row
    // does not name is refused, and now that refusal cannot disagree with what
    // the driver iterated.
    let steps: u8 = spec
        .expiry_codes
        .iter()
        .position(|known| *known == code)
        .and_then(|at| u8::try_from(at.saturating_add(1)).ok())
        .ok_or(RollingError::NoExpiry {
            why: "the expiry ordinal is not one this vendor serves",
        })?;
    let mut at = day;
    let mut found = None;
    for _ in 0..steps {
        // THE CADENCE COMES FROM THE ROW, and the `match` below is now over an
        // ENUM rather than over a spelling — so it is exhaustive, and a vendor
        // that spells its weekly `W` or `WEEKLY` needs no edit here.
        let next = match cadence {
            crate::vendor::ExpiryCadence::Weekly => costs::expiry::next_weekly_expiry(slot, at)
                .map_err(|_| RollingError::NoExpiry {
                    why: "the day is before this weekly regime was verified from",
                })?
                .ok_or(RollingError::NoExpiry {
                    why: "this weekly was withdrawn for that slot, so no contract existed",
                })?,
            crate::vendor::ExpiryCadence::Monthly => costs::expiry::next_monthly_expiry(slot, at)
                .map_err(|_| RollingError::NoExpiry {
                why: "the day is before this monthly regime was verified from",
            })?,
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
    // A CLOSED DAY IS NOT AN EXPIRY. `costs::expiry` returns the plain
    // calendar weekday and leaves holidays to its caller, and this caller never
    // asked, so a holiday week's contract got a closed day as its expiry, was
    // filed under that key, and priced at a tenor ~4.8x too long (CE-14,
    // D-1769). The exchange's holiday-shift rule is not recorded in
    // `docs/00-charter.md`, so the closed day is REFUSED rather than stepped
    // back (`CLAUDE.md` §3 rule 1). A day past the calendar's last measured
    // day cannot be checked and is passed through; `docs/06-limits.md` names
    // that limit.
    //
    // AND A DAY OPEN ONLY FOR A MUHURAT HOUR IS NOT AN EXPIRY EITHER (CE-53,
    // D-2671). Refusing only `Closed` accepted 2021-11-04 (a Muhurat of
    // unmeasured length) and 2025-10-21 (13:45-14:44) as weekly expiries, and
    // bars filed under that key were priced to a 15:30 close the day never
    // had. An expiry must be a FULL regular session; anything short of one is
    // refused the same way, for the same reason.
    match crate::calendar::kind_of(i64::from(settled.ordinal())) {
        crate::calendar::DayKind::Closed => {
            return Err(RollingError::NoExpiry {
                why: "the computed expiry falls on a day the exchange calendar marks closed, and \
                      the rule that moves an expiry off a holiday is not charter-sourced, so no \
                      date is guessed",
            });
        }
        crate::calendar::DayKind::Open(session) if session == crate::calendar::Session::full() => {}
        crate::calendar::DayKind::Open(_) | crate::calendar::DayKind::OpenLengthUnmeasured => {
            return Err(RollingError::NoExpiry {
                why: "the computed expiry falls on a day the exchange calendar records as \
                      something other than a full regular session (a Muhurat hour or an \
                      irregular session), and the rule that moves an expiry off such a day is \
                      not charter-sourced, so no date is guessed",
            });
        }
        crate::calendar::DayKind::Unmeasured => {}
    }
    brutex_core::instrument::Expiry::new(settled.year(), settled.month(), settled.day()).map_err(
        |_| RollingError::NoExpiry {
            why: "the calendar produced a date this store cannot name",
        },
    )
}

/// Whether an underlying lists contracts on a cadence on one day.
///
/// Distinct from [`expiry_of`], and the distinction is CE-43: `expiry_of`
/// answers "which contract", and can refuse a contract that exists (its
/// computed expiry is a closed day, CE-14). A caller asking "is this cadence
/// worth a request at all" must not read that refusal as "no contracts", or a
/// holiday week silently removes a whole cadence from a walk. Only
/// [`Listing::Withdrawn`] means the exchange listed nothing. D-2650.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Listing {
    /// The cadence's regime lists contracts on or after that day.
    Listed,
    /// The cadence was withdrawn for this underlying before that day
    /// (`costs::expiry`'s `WeeklyRegime::Withdrawn`), so no contract exists.
    Withdrawn,
}

/// Whether `underlying` lists contracts on cadence `flag` on `on`.
///
/// # Errors
///
/// The same named refusals as [`expiry_of`] for a cadence this vendor does not
/// serve, an unknown underlying, an unreal day or a day before the regime was
/// verified from. A closed-day expiry is NOT an error here: it is a property of
/// one contract, not of the cadence.
///
/// # Cost
///
/// O(1): one slot lookup and one dated-table step, the first step of
/// [`expiry_of`]'s walk. **UNVERIFIED as a measurement**, as there.
pub fn listing_of(
    underlying: &str,
    spec: &RollingSpec,
    flag: &str,
    on: crate::session::Day,
) -> Result<Listing, RollingError> {
    let (cadence, slot, day) = regime_of(underlying, spec, flag, on)?;
    match cadence {
        crate::vendor::ExpiryCadence::Weekly => {
            match costs::expiry::next_weekly_expiry(slot, day) {
                Ok(Some(_)) => Ok(Listing::Listed),
                Ok(None) => Ok(Listing::Withdrawn),
                Err(_) => Err(RollingError::NoExpiry {
                    why: "the day is before this weekly regime was verified from",
                }),
            }
        }
        crate::vendor::ExpiryCadence::Monthly => costs::expiry::next_monthly_expiry(slot, day)
            .map(|_| Listing::Listed)
            .map_err(|_| RollingError::NoExpiry {
                why: "the day is before this monthly regime was verified from",
            }),
    }
}

/// The cadence, the swept slot and the trade day one rolling question is
/// asked on: the shared first half of [`expiry_of`] and [`listing_of`], so the
/// two cannot drift on which flag, underlying or day they refuse.
fn regime_of(
    underlying: &str,
    spec: &RollingSpec,
    flag: &str,
    on: crate::session::Day,
) -> Result<
    (
        crate::vendor::ExpiryCadence,
        costs::venue::SweptSlot,
        costs::day::TradeDay,
    ),
    RollingError,
> {
    // THE CADENCE THE ROW NAMES, RESOLVED ONCE AND BEFORE THE WALK. Two
    // comparisons against a fixed table — constant work, and a flag the row
    // does not carry is refused here rather than inside the loop.
    let cadence = spec
        .expiry_flags
        .iter()
        .find(|(word, _)| *word == flag)
        .map(|(_, cadence)| *cadence)
        .ok_or(RollingError::NoExpiry {
            why: "the expiry cadence is not one this vendor serves",
        })?;
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
    Ok((cadence, slot, day))
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
    /// A key appears twice inside one object, so the body carries two values
    /// for one field. `serde_json` keeps the last and says nothing; this is
    /// refused by name instead, as `http::decode_body` refuses it (D-1531).
    /// CE-56, D-2680.
    RepeatedKey {
        /// The repeated key, decoded.
        key: String,
    },
    /// The side's object was absent. `CALL` asked and no `ce` returned.
    NoSide {
        /// Which key was looked for — `ce` or `pe`.
        key: &'static str,
    },
    /// The side asked for is not one this feed's row names.
    ///
    /// Distinct from [`Self::NoSide`], and the distinction is the whole of
    /// D-0346: `NoSide` means the vendor answered without that side in it,
    /// which is ordinary. This means the CALLER named a side the descriptor
    /// does not carry, so there is no key to look for — and the derivation this
    /// replaces answered `"pe"` for every such input rather than saying so.
    UnknownSide,
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
    /// A count cell that is not a non-negative whole number, or that is the
    /// store's own null sentinel. GAP16-22, D-0952.
    Uncountable {
        /// Which field.
        field: &'static str,
        /// The cell exactly as the vendor wrote it.
        text: String,
    },
    /// A decimal cell that is present and cannot be read exactly — not text
    /// or a number, or a form the six-place shift does not read (an exponent,
    /// an overflow, junk). It was stored as the null sentinel, which is the
    /// record that the vendor SENT NONE, and pricing then solved its own value
    /// and labelled it so (CE-16, D-1769).
    Undecimal {
        /// Which field.
        field: &'static str,
        /// The cell exactly as the vendor wrote it.
        text: String,
    },
    /// A volatility cell below zero. No implied volatility is negative, so a
    /// sign names a wrong field or a wrong scale; it was stored as read
    /// (STO-2, D-2607).
    NegativeVolatility {
        /// Which field.
        field: &'static str,
        /// The cell exactly as the vendor wrote it.
        text: String,
    },
    /// A price cell that is negative, or is not zero and snaps to zero.
    /// GAP16-23, D-1492.
    NotAPrice {
        /// Which field.
        field: &'static str,
        /// The cell exactly as the vendor wrote it.
        text: String,
    },
    /// A timestamp cell that is not a whole number of seconds whose
    /// microseconds fit `i64`, or is `i64::MIN`. c4a-3, D-1491.
    Unstampable {
        /// Which field.
        field: &'static str,
        /// The cell exactly as the vendor wrote it.
        text: String,
    },
}

impl core::fmt::Display for RollingError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NotJson => f.write_str("the rolling answer is not JSON"),
            Self::RepeatedKey { key } => write!(
                f,
                "the rolling answer repeats the key {key:?} inside one object, \
                 so it carries two values for one field; refused rather than \
                 silently keeping the last"
            ),
            Self::UnknownSide => write!(
                f,
                "that is not a side this feed's descriptor names, so there is no \
                 response key to read it under. Refused rather than guessed: the \
                 side decides which contract's file these bars are written to, \
                 and the store is append-only"
            ),
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
            Self::NegativeVolatility { field, text } => write!(
                f,
                "a `{field}` cell holds {text}, and no implied volatility is \
                 below zero. Refused rather than stored"
            ),
            Self::Undecimal { field, text } => write!(
                f,
                "a `{field}` cell holds {text}, which is not a decimal this build \
                 reads exactly. Refused rather than stored as the absence the \
                 vendor did not state"
            ),
            Self::Uncountable { field, text } => write!(
                f,
                "a `{field}` cell holds {text}, which is not a non-negative whole \
                 count, or is i64::MIN, the store's open-interest null sentinel \
                 (CLAUDE.md §7). Refused rather than stored as a zero, a \
                 truncation or an absence the vendor did not state"
            ),
            Self::NotAPrice { field, text } => write!(
                f,
                "a `{field}` cell holds {text}, which is below zero or is not zero \
                 and smaller than half a paisa. Neither is a price: stored, it \
                 would read as a negative or as a zero the vendor did not send"
            ),
            Self::Unstampable { field, text } => write!(
                f,
                "a `{field}` cell holds {text}, which is not a whole number of \
                 epoch seconds whose microseconds fit an i64. Refused rather than \
                 filed at the epoch or saturated to the end of time"
            ),
        }
    }
}

/// Which JSON key holds a side's rows.
///
/// The request says `CALL`; the answer says `ce`. Mapping it here rather than
/// at the call site keeps the vendor's two spellings of one idea in one place.
#[must_use]
pub fn side_key(spec: &crate::vendor::RollingSpec, side: &str) -> Option<&'static str> {
    // TWO COMPARISONS AGAINST A FIXED TABLE, so this is constant work — the
    // same bound the length test had, and now against the vendor's own row
    // rather than a coincidence of spelling.
    spec.sides
        .iter()
        .find(|(word, _)| *word == side)
        .map(|(_, key)| *key)
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
///
/// **UNVERIFIED as a measurement.** The bound is argued from the
/// shape of the code and no bench in this workspace times it.
/// `CLAUDE.md` §3 rule 6: a structural argument is not a
/// measurement, however sound it is.
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

/// One `"key":"value"` pair, JSON-escaped as RFC 8259 requires.
///
/// Escaped only `"` and `\` until CE-68: a security id is vendor-file text
/// and may hold a control character, which sent Dhan a body no parser accepts
/// and an error that blamed the vendor. Below 0x20 is now `\u00XX`, the same
/// rule as `api::pullrun::quote_for_json` (D-1772).
fn push_pair(out: &mut String, key: &str, value: &str, first: bool) {
    use core::fmt::Write as _;
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
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if u32::from(c) < 0x20 => {
                // Writing into a `String` cannot fail.
                let _cannot_fail = write!(out, "\\u{:04x}", u32::from(c));
            }
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
pub fn read(
    body: &str,
    spec: &crate::vendor::RollingSpec,
    side: &str,
    scale: PriceScale,
) -> Result<Vec<Row>, RollingError> {
    let root: serde_json::Value = serde_json::from_str(body).map_err(|_| RollingError::NotJson)?;
    // A KEY REPEATED INSIDE ONE OBJECT IS TWO ANSWERS IN ONE BODY, refused
    // here exactly as `http::decode_body` refuses it (CE-56, D-2680).
    if let Some(key) = crate::http::repeated_key(body) {
        return Err(RollingError::RepeatedKey { key });
    }
    // THE ENVELOPE IS OPTIONAL, exactly as `fno::names` treats it, and for the
    // same reason: one reader for a vendor that wraps and one that does not.
    let held = root.get("data").unwrap_or(&root);
    // A SIDE THE ROW DOES NOT NAME IS REFUSED, NEVER GUESSED. The old
    // derivation answered `"pe"` for anything that was not four characters
    // long, so an unrecognised spelling silently read the PUT array — and the
    // side names the file these bars are written to.
    let Some(key) = side_key(spec, side) else {
        return Err(RollingError::UnknownSide);
    };
    let one = held.get(key).ok_or(RollingError::NoSide { key })?;
    // A NULL SIDE IS AN EMPTY ANSWER, NOT A MALFORMED ONE.
    //
    // The vendor's own example response carries `"pe": null` — that is the
    // documented shape for "this side listed nothing", and it is the ordinary
    // answer for a strike offset or an expiry ordinal that never existed.
    //
    // It was read as a shape error. `get(key)` returns `Some(Value::Null)`,
    // which passes the `NoSide` check above and then fails on the next line as
    // `NotAnArray { field: "timestamp" }` — so an unremarkable "no such
    // contract" was reported as a vendor answer this build could not
    // understand, counted as a FAILURE, and `fno_roll` turned the month into a
    // 502 reading "the month is incomplete and must not be read as held".
    //
    // On one healthy index month that is 252 planned requests, of which the
    // ones for offsets the vendor never listed are ordinary and expected. False
    // alarms on that scale are worse than no alarm: they train an operator to
    // read 502 as noise, which is exactly when a real one arrives.
    //
    // `roll_one` already has the right handling one layer up — an empty row set
    // returns `Ok(0)` and is explicitly not a fault. This routes the null case
    // to it. An ABSENT key keeps `NoSide`, because a body with no side object
    // at all is a shape this build has not seen and should not quietly accept.
    if one.is_null() {
        return Ok(Vec::new());
    }

    // EVERY NAME FROM THE ROW, NOT FROM THIS FUNCTION'S BODY.
    //
    // All ten were spelled here, so a second vendor answering the same SHAPE
    // under different names — `ts` for `timestamp`, `openInterest` for `oi` —
    // would have needed an edit in this reader rather than a row in
    // `crate::vendor`. `RollingSpec`'s own doc promises the opposite. D-0358.
    let f = spec.fields;
    let stamps = array(one, f.timestamp)?;
    let open = same_length(one, f.open, stamps.len())?;
    let high = same_length(one, f.high, stamps.len())?;
    let low = same_length(one, f.low, stamps.len())?;
    let close = same_length(one, f.close, stamps.len())?;
    let volume = same_length(one, f.volume, stamps.len())?;
    // OI, IV AND SPOT ARE OPTIONAL AND THE OTHERS ARE NOT. The vendor's own
    // schema marks every field `Required: No`, and measurement is what decides
    // which are really there: a contract with no open interest is ordinary,
    // while a bar with no close is not a bar. An absent optional array reads as
    // "the vendor stated none", which is what the null sentinels are for.
    let oi = optional(one, f.open_interest, stamps.len())?;
    let iv = optional(one, f.implied_volatility, stamps.len())?;
    let spot = optional(one, f.spot, stamps.len())?;
    // OPTIONAL AT THE PARSE, REQUIRED AT THE FILE. Absent here is a fact about
    // the answer and reads as `None`; it becomes a refusal one layer up, where
    // the contract is named and the absence actually bites.
    let strike = optional(one, f.strike, stamps.len())?;

    let mut rows = Vec::with_capacity(stamps.len());
    for at in 0..stamps.len() {
        let bar = Bar {
            // SECONDS ON THE WIRE, MICROSECONDS IN THE STORE. The same
            // conversion `TimestampEncoding::EpochSecondsUtc` names, done here
            // because this reader does not go through the CSV path that owns it.
            //
            // REFUSED, NEVER DEFAULTED OR SATURATED (c4a-3, W1-pull3-7,
            // D-1491). This read `number`, which answered `0` for a `null` or
            // text cell — a bar filed at the epoch — and `saturating_mul`
            // turned a stamp past the microsecond range into `i64::MAX`.
            ts_micros: stamp(stamps.get(at), f.timestamp)?,
            open: paisa(open, at, scale, "open")?,
            high: paisa(high, at, scale, "high")?,
            low: paisa(low, at, scale, "low")?,
            close: paisa(close, at, scale, "close")?,
            // A COUNT, READ THE WAY OPEN INTEREST IS (c4a-3, D-1491). `number`
            // answered `0` for a text or `null` cell and passed a negative
            // through. A `null` is refused too: the columnar intraday path for
            // the same vendor refuses a null volume (`http::one_number`), and
            // no vendor document says a null volume here means zero.
            volume: count(
                volume
                    .get(at)
                    .ok_or(RollingError::NotAnArray { field: f.volume })?,
                f.volume,
            )?,
            // A NULL OI **CELL** IS ABSENT, NOT ZERO.
            //
            // `map_or(OI_NULL, ..)` covers only the whole array being missing —
            // which is what `number`'s own doc claims is the only case. It is
            // not: the vendor can send `"oi": [1200, null, 1350]`, and `number`
            // answers `0` for any cell it cannot read, so bar two was stored
            // with an open interest of zero that `Bar::oi()` then reports as a
            // measurement rather than as absence.
            //
            // `spot` two lines down refuses a bad cell outright and `iv`
            // sentinels it; open interest was the one of the three that
            // collapsed absence into a real number. `CLAUDE.md` §7 gives it a
            // sentinel precisely so the two can be told apart.
            //
            // AND A CELL THAT IS PRESENT BUT NOT A COUNT IS REFUSED. `number`
            // also answered `0` for `"4200"` or `true`, truncated `1234.5`, and
            // passed `i64::MIN` straight through — which IS `OI_NULL`, so a
            // number the vendor sent was filed as the absence it did not
            // state. GAP16-22, D-0952.
            open_interest: match oi.and_then(|a| a.get(at)) {
                None | Some(serde_json::Value::Null) => OI_NULL,
                Some(cell) => count(cell, f.open_interest)?,
            },
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
            iv_micros: match iv.and_then(|a| a.get(at)) {
                None | Some(serde_json::Value::Null) => OI_NULL,
                Some(cell) => micros_of(cell, f.implied_volatility)?,
            },
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

/// One timestamp cell, in microseconds, or a refusal naming it.
///
/// Seconds on the wire: an integer, or a decimal that is a whole number. A
/// `null`, text, a fraction, `i64::MIN`, or a value whose microseconds do not
/// fit `i64` is refused. A negative is NOT refused: `session::IstMoment`
/// accepts `-19_800..0` as 1970-01-01 IST, and the session filter is the one
/// authority on which stamps are sessions (`http::one_number` keeps the same
/// rule). It replaced `number`, which answered `0` — the epoch — for every cell
/// it could not read (c4a-3, W1-pull3-7, D-1491).
fn stamp(cell: Option<&serde_json::Value>, field: &'static str) -> Result<i64, RollingError> {
    let refuse = || RollingError::Unstampable {
        field,
        text: cell.map_or_else(|| "an absent cell".to_owned(), serde_json::Value::to_string),
    };
    let cell = cell.ok_or_else(refuse)?;
    let seconds = if let Some(whole) = cell.as_i64() {
        whole
    } else {
        let number = cell.as_number().ok_or_else(refuse)?;
        let hundredths = crate::csv::paisa(&crate::http::number_text(number).ok_or_else(refuse)?)
            .ok_or_else(refuse)?;
        if hundredths % 100 != 0 {
            return Err(refuse());
        }
        hundredths / 100
    };
    if seconds == i64::MIN {
        return Err(refuse());
    }
    seconds.checked_mul(1_000_000).ok_or_else(refuse)
}

/// One count cell — open interest or volume — as a count, or a refusal naming it.
///
/// The same rule `http::one_number` applies on the intraday path: an integer
/// is the value, a decimal is accepted only when it is a whole number, and
/// `i64::MIN` is refused because it is [`OI_NULL`]. A negative is refused too,
/// because `store::format::Bar::counts_are_sane` would refuse it one crate
/// later with no record of which cell it came from. GAP16-22, D-0952.
///
/// An open-interest `null` never reaches here: the caller reads it as the
/// sentinel. A volume `null` does, and is refused (c4a-3, D-1491).
fn count(cell: &serde_json::Value, field: &'static str) -> Result<i64, RollingError> {
    let refuse = || RollingError::Uncountable {
        field,
        text: cell.to_string(),
    };
    let whole = if let Some(whole) = cell.as_i64() {
        whole
    } else {
        let number = cell.as_number().ok_or_else(refuse)?;
        let hundredths = crate::csv::paisa(&crate::http::number_text(number).ok_or_else(refuse)?)
            .ok_or_else(refuse)?;
        if hundredths % 100 != 0 {
            return Err(refuse());
        }
        hundredths / 100
    };
    if whole < 0 {
        return Err(refuse());
    }
    Ok(whole)
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
    let not_a_price = || RollingError::NotAPrice {
        field,
        text: cell.to_string(),
    };
    let paisa = match scale {
        // Already paisa: an integer count, and nothing to convert.
        PriceScale::Paisa => cell
            .as_i64()
            .ok_or(RollingError::Unrepresentable { field })?,
        // THE TEXT IS THE TRUTH, and `core`'s half-up reader owns the rule —
        // the same sentence `http::one_price` writes over the same conversion.
        //
        // THIS COMMENT USED TO CONTRADICT ITS OWN CALL. It ended, as it still
        // does, on `CLAUDE.md` §7 snapping half-up at this boundary — and then
        // called `csv::paisa`, which does not snap at all: it returns `None`
        // past two decimals. The claim and the code disagreed inside one
        // expression. This vendor's OHLC fields are `float` by its own
        // documentation, so the third decimal is the float's error and not the
        // exchange's price; refusing it discarded whole windows over a rounding
        // artifact. See `http::prices` for the measurement. D-0321.
        //
        // Not `(x * 100.0).round()`. `float_arithmetic` is denied workspace-
        // wide, and the reason outlives the lint: a rupee figure that arrived
        // as text has an exact decimal the vendor wrote, and routing it through
        // an f64 to shift two places introduces a representation error into a
        // value that had none. The reader below walks the text digit by digit.
        // The text is the vendor's own digits, recovered by
        // `http::number_text` (D-1570), not an f64's re-rendering.
        PriceScale::Rupees => {
            let text = cell
                .as_number()
                .and_then(crate::http::number_text)
                .ok_or(RollingError::Unrepresentable { field })?;
            let snapped = brutex_core::price::Paisa::from_rupee_text_half_up(&text)
                .map(brutex_core::price::Paisa::raw)
                .map_err(|_| RollingError::Unrepresentable { field })?;
            // A NON-ZERO VALUE THAT SNAPS TO ZERO IS REFUSED, the guard
            // `http::one_price` has always had and this reader did not
            // (GAP16-23, D-1492). `0.0001` and `-0.001` would otherwise be
            // stored as a real zero price, and the negative one would pass the
            // sign guard below because it is already zero. The test is on the
            // text: a value written with a non-zero digit is not zero.
            if snapped == 0 && text.bytes().any(|b| b.is_ascii_digit() && b != b'0') {
                return Err(not_a_price());
            }
            snapped
        }
    };
    // A NEGATIVE PRICE IS NOT A PRICE (GAP16-23, D-1492): the same refusal
    // `http::one_price` makes, on both scales. Nothing this build reads trades
    // below zero, so a negative names a wrong `PriceScale` or a field that is
    // not a price. A spot or strike of zero is still accepted, as on that path.
    if paisa < 0 {
        return Err(not_a_price());
    }
    Ok(paisa)
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
///
/// # Errors
///
/// [`RollingError::Undecimal`] for a present cell that is not text or a
/// number, or that `shift_six` cannot read. A JSON `null` never reaches here:
/// the caller files it as absent, which is what the vendor said.
fn micros_of(cell: &serde_json::Value, field: &'static str) -> Result<i64, RollingError> {
    let refuse = || RollingError::Undecimal {
        field,
        text: cell.to_string(),
    };
    let text = match cell {
        serde_json::Value::String(text) => text.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        _ => return Err(refuse()),
    };
    let micros = shift_six(text.trim()).ok_or_else(refuse)?;
    // A NEGATIVE VOLATILITY IS NOT A VOLATILITY (STO-2, D-2607). Checked on
    // the text as well as the value, so `-0.0000004`, which rounds to zero,
    // is refused for the sign the vendor wrote rather than filed as zero.
    let signed =
        text.trim().starts_with('-') && text.bytes().any(|b| b.is_ascii_digit() && b != b'0');
    if micros < 0 || signed {
        return Err(RollingError::NegativeVolatility {
            field,
            text: cell.to_string(),
        });
    }
    Ok(micros)
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
    /// **A NULL SIDE IS AN EMPTY ANSWER AND NOT A FAILURE.**
    ///
    /// The vendor's own documented response carries `"pe": null`. Reading that
    /// as a shape error turned every strike offset the vendor never listed into
    /// a counted failure, and `fno_roll` turned the month into a 502.
    #[test]
    fn a_side_the_vendor_answered_null_reads_as_no_rows_rather_than_a_bad_shape() {
        let body = r#"{"data":{"ce":{"timestamp":[1756698300],"open":[354],"high":[354],
            "low":[354],"close":[354],"volume":[1]},"pe":null}}"#;

        let put =
            read(body, &spec(), "PUT", PriceScale::Rupees).expect("a null side is an empty answer");
        assert!(put.is_empty(), "no rows, and no error");

        // AND THE OTHER SIDE OF THE SAME BODY STILL READS. A null on one side
        // must not cost the side that answered.
        let call =
            read(body, &spec(), "CALL", PriceScale::Rupees).expect("the answered side reads");
        assert_eq!(call.len(), 1);
    }

    /// audit-20261003 hunt-pull-2 (D-1530). A rupee price that is not zero
    /// and is smaller than half a paisa, or that is below zero, is refused
    /// rather than snapped to a clean zero. `http::one_price` refuses exactly
    /// this; the rolling decoder stored `0.004` and `-0.004` as a price of 0,
    /// and the negative one thereby slipped past every below-zero check.
    #[test]
    fn a_sub_half_paisa_or_negative_rupee_price_is_refused_not_snapped_to_zero() {
        for cell in ["0.004", "-0.004", "-0.01", "-354"] {
            let body = format!(
                r#"{{"data":{{"ce":{{"timestamp":[1756698300],"open":[{cell}],"high":[354],
                "low":[0],"close":[354],"volume":[1]}},"pe":null}}}}"#
            );
            // The refusal is D-1492's `NotAPrice`, which landed on the same
            // guard from the other audit; this test pins that it names the cell.
            assert_eq!(
                read(&body, &spec(), "CALL", PriceScale::Rupees).map(|rows| rows.len()),
                Err(RollingError::NotAPrice {
                    field: "open",
                    text: cell.to_owned(),
                }),
                "{cell} is not a price this decoder may store"
            );
        }
        // A real zero, however it is written, is still a price.
        for cell in ["0", "0.0", "0.00", "-0.0"] {
            let body = format!(
                r#"{{"data":{{"ce":{{"timestamp":[1756698300],"open":[{cell}],"high":[354],
                "low":[0],"close":[354],"volume":[1]}},"pe":null}}}}"#
            );
            let rows = read(&body, &spec(), "CALL", PriceScale::Rupees)
                .unwrap_or_else(|why| panic!("{cell} is a real zero: {why}"));
            assert_eq!(rows.len(), 1, "{cell}");
        }
    }

    /// audit-20261003 attackdata-4 (D-1570). The rolling decoder snaps the
    /// vendor's own digits, not an f64's rendering of them: each text here is
    /// one an `f64` rounds across a half-paisa boundary first.
    #[test]
    fn a_rolling_price_is_snapped_from_the_vendors_own_text() {
        for (cell, want) in [
            ("354.12499999999999999", 35_412_i64), // f64: 354.125 -> 35413
            ("0.0149999999999999999", 1),          // f64: 0.015 -> 2
            ("3.5412499999999999999e2", 35_412),   // f64: 354.125 -> 35413
        ] {
            let body = format!(
                r#"{{"data":{{"ce":{{"timestamp":[1756698300],"open":[{cell}],"high":[{cell}],
                "low":[{cell}],"close":[{cell}],"volume":[1]}},"pe":null}}}}"#
            );
            let rows = read(&body, &spec(), "CALL", PriceScale::Rupees)
                .unwrap_or_else(|why| panic!("{cell}: {why}"));
            let bar = rows.first().expect("one row").bar;
            assert_eq!(
                (bar.open, bar.high, bar.low, bar.close),
                (want, want, want, want),
                "{cell} is {want} paisa by its own text"
            );
        }
    }

    /// An absent side keeps its refusal.
    ///
    /// `null` is the vendor saying "nothing here"; a missing key is a body with
    /// no side object at all, which is a shape this build has not seen and must
    /// not quietly read as empty.
    #[test]
    fn a_side_that_is_absent_entirely_is_still_refused_by_name() {
        let body = r#"{"data":{"ce":{"timestamp":[],"open":[],"high":[],"low":[],
            "close":[],"volume":[]}}}"#;

        assert_eq!(
            read(body, &spec(), "PUT", PriceScale::Rupees),
            Err(RollingError::NoSide { key: "pe" })
        );
    }

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
        let got = read(ragged, &spec(), "CALL", PriceScale::Rupees);
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
        let rows = read(one, &spec(), "CALL", PriceScale::Rupees).expect("it reads");
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
        let rows = read(bare, &spec(), "CALL", PriceScale::Rupees).expect("it reads");
        assert_eq!(rows[0].bar.open_interest, OI_NULL, "absent OI is null");
        assert!(rows[0].bar.oi().is_none());
        assert_eq!(rows[0].overlay.spot, OI_NULL);
        assert_eq!(rows[0].overlay.iv_micros, OI_NULL);
        assert!(
            !rows[0].overlay.states_something(),
            "an overlay stating neither value is not worth a record"
        );
    }

    /// CE-16, D-1769: an implied-volatility cell that is present and cannot
    /// be read refuses the answer by name; only a `null` or a missing cell is
    /// filed as "the vendor sent none".
    #[test]
    fn an_unreadable_volatility_cell_is_refused_not_filed_as_absent() {
        let with = |iv: &str| {
            format!(
                r#"{{"data":{{"ce":{{
                "timestamp":[1700000000],
                "open":[100.0],"high":[100.0],"low":[100.0],"close":[100.0],
                "volume":[1],"iv":[{iv}]
            }}}}}}"#
            )
        };
        for bad in [
            r#""junk""#,
            "1e-7",
            "true",
            r#"{"v":1}"#,
            "99999999999999999999",
        ] {
            let got = read(&with(bad), &spec(), "CALL", PriceScale::Rupees);
            assert!(
                matches!(got, Err(RollingError::Undecimal { field: "iv", .. })),
                "{bad}: {got:?}"
            );
        }
        let rows = read(&with("null"), &spec(), "CALL", PriceScale::Rupees).expect("null reads");
        assert_eq!(rows[0].overlay.iv_micros, OI_NULL);
        let rows = read(&with("0.125"), &spec(), "CALL", PriceScale::Rupees).expect("reads");
        assert_eq!(rows[0].overlay.iv_micros, 125_000);
    }

    /// STO-2, D-2607: a negative volatility cell is refused by name, including
    /// one that rounds to zero at six places; a signed zero is still zero.
    #[test]
    fn a_negative_volatility_cell_is_refused_by_name() {
        let with = |iv: &str| {
            format!(
                r#"{{"data":{{"ce":{{
                "timestamp":[1700000000],
                "open":[100.0],"high":[100.0],"low":[100.0],"close":[100.0],
                "volume":[1],"iv":[{iv}]
            }}}}}}"#
            )
        };
        for bad in ["-0.25", r#""-0.0000005""#, r#""-0.0000004""#] {
            let got = read(&with(bad), &spec(), "CALL", PriceScale::Rupees);
            assert!(
                matches!(got, Err(RollingError::NegativeVolatility { .. })),
                "{bad}: {got:?}"
            );
        }
        for zero in ["0", r#""-0.0""#] {
            let rows = read(&with(zero), &spec(), "CALL", PriceScale::Rupees).expect("zero reads");
            assert_eq!(rows[0].overlay.iv_micros, 0, "{zero}");
        }
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
        let rows = read(full, &spec(), "CALL", PriceScale::Rupees).expect("it reads");
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
        let call = read(both, &spec(), "CALL", PriceScale::Rupees).expect("ce reads");
        let put = read(both, &spec(), "PUT", PriceScale::Rupees).expect("pe reads");
        assert_eq!(call[0].bar.close, 100, "the call came from ce");
        assert_eq!(put[0].bar.close, 200, "the put came from pe");
        assert_eq!(side_key(&spec(), "CALL"), Some("ce"));
        assert_eq!(side_key(&spec(), "PUT"), Some("pe"));

        // **A SPELLING THE ROW DOES NOT NAME IS REFUSED, NOT GUESSED**, and
        // this is the whole of D-0346. The key used to be derived as
        // `if side.len() == 4 { "ce" } else { "pe" }`, which is right for
        // `CALL`/`PUT` by a coincidence of length and wrong for everything
        // else: `"CE"` is two characters and would have read the PUT array.
        // The side names the file an option's bars are written to, and §8 means
        // that directory cannot be renamed afterwards.
        for wrong in ["CE", "PE", "C", "P", "Call", "call", ""] {
            assert_eq!(
                side_key(&spec(), wrong),
                None,
                "{wrong:?} is not a side this row names, so there is no key — \
                 the length test answered `pe` for four of these"
            );
        }
        assert!(
            matches!(
                read(both, &spec(), "CE", PriceScale::Rupees),
                Err(RollingError::UnknownSide)
            ),
            "and `read` refuses it by name rather than returning the other \
             side's prices"
        );

        // AND AN ABSENT SIDE IS NAMED, not read as an empty answer. A month
        // that returned no puts and a month whose puts were not asked for are
        // different facts.
        let only_ce = r#"{"data":{"ce":{"timestamp":[],"open":[],"high":[],"low":[],"close":[],"volume":[]}}}"#;
        assert_eq!(
            read(only_ce, &spec(), "PUT", PriceScale::Rupees),
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
        let near = expiry_of("NIFTY", &spec(), "WEEK", "1", on).expect("near resolves");
        let next = expiry_of("NIFTY", &spec(), "WEEK", "2", on).expect("next resolves");
        let far = expiry_of("NIFTY", &spec(), "WEEK", "3", on).expect("far resolves");

        assert!(near < next, "next is after near: {near:?} !< {next:?}");
        assert!(next < far, "far is after next: {next:?} !< {far:?}");

        // AND A MONTHLY IS A DIFFERENT ANSWER FROM A WEEKLY. One cadence
        // standing in for the other would file a monthly's bars under a
        // weekly's date, which no later check could detect.
        let monthly = expiry_of("NIFTY", &spec(), "MONTH", "1", on).expect("monthly resolves");
        assert_ne!(
            monthly, near,
            "the monthly and the near weekly are not the same contract"
        );
    }

    /// CE-43, D-2650: `listing_of` separates "no contract on this cadence"
    /// from "this contract's expiry is refused". The closed-day weeks that
    /// `expiry_of` refuses are `Listed`; BANKNIFTY weeklies after their
    /// 2024-11-13 withdrawal are `Withdrawn`; a cadence the row does not serve
    /// is refused by name.
    #[test]
    fn a_closed_day_contract_is_listed_and_only_a_withdrawal_is_not() {
        use crate::session::Day;
        for (on, flag) in [
            (Day::new(2024, 8, 14).expect("a real day"), "WEEK"),
            (Day::new(2023, 3, 29).expect("a real day"), "MONTH"),
        ] {
            assert!(expiry_of("NIFTY", &spec(), flag, "1", on).is_err());
            assert_eq!(
                listing_of("NIFTY", &spec(), flag, on),
                Ok(Listing::Listed),
                "{on:?} {flag}"
            );
        }
        let after = Day::new(2026, 1, 5).expect("a real day");
        assert_eq!(
            listing_of("BANKNIFTY", &spec(), "WEEK", after),
            Ok(Listing::Withdrawn)
        );
        assert_eq!(
            listing_of("BANKNIFTY", &spec(), "MONTH", after),
            Ok(Listing::Listed)
        );
        assert!(matches!(
            listing_of("NIFTY", &spec(), "DAILY", after),
            Err(RollingError::NoExpiry { why }) if why.contains("not one this vendor serves")
        ));
    }

    /// CE-14, D-1769: a computed expiry the exchange calendar marks CLOSED is
    /// refused, not filed. NIFTY's weekly from 2024-08-14 lands on 2024-08-15
    /// (Independence Day) and its monthly from 2023-03-29 on 2023-03-30 (Ram
    /// Navami); both are `Closed` in `pull::calendar`.
    #[test]
    fn a_computed_expiry_on_a_closed_day_is_refused() {
        use crate::session::Day;
        for (on, flag) in [
            (Day::new(2024, 8, 14).expect("a real day"), "WEEK"),
            (Day::new(2023, 3, 29).expect("a real day"), "MONTH"),
        ] {
            let got = expiry_of("NIFTY", &spec(), flag, "1", on);
            assert!(
                matches!(
                    got,
                    Err(RollingError::NoExpiry { why }) if why.contains("marks closed")
                ),
                "{on:?} {flag}: {got:?}"
            );
        }
        // An ordinary week still resolves.
        assert!(
            expiry_of(
                "NIFTY",
                &spec(),
                "WEEK",
                "1",
                Day::new(2024, 8, 21).expect("a day")
            )
            .is_ok()
        );
    }

    /// CE-53, D-2671: an expiry must be a FULL regular session. A day that is
    /// open only for a Muhurat hour is refused exactly as a closed day is,
    /// never filed under that key and priced to a 15:30 close it never had.
    /// NIFTY and BANKNIFTY weeklies from 2021-11-01 land on the 2021-11-04
    /// Muhurat (`OpenLengthUnmeasured`); NIFTY's weekly from 2025-10-20 lands
    /// on the 2025-10-21 Muhurat (`Open`, 13:45-14:44). The cadence stays
    /// `Listed`: a refused contract is not a withdrawn cadence (CE-43).
    #[test]
    fn a_computed_expiry_on_a_muhurat_only_day_is_refused_like_a_closed_one() {
        use crate::session::Day;
        for (underlying, on) in [
            ("NIFTY", Day::new(2021, 11, 1).expect("a real day")),
            ("BANKNIFTY", Day::new(2021, 11, 1).expect("a real day")),
            ("NIFTY", Day::new(2025, 10, 20).expect("a real day")),
        ] {
            let got = expiry_of(underlying, &spec(), "WEEK", "1", on);
            assert!(
                matches!(
                    got,
                    Err(RollingError::NoExpiry { why }) if why.contains("full regular session")
                ),
                "{underlying} {on:?}: {got:?}"
            );
            assert_eq!(
                listing_of(underlying, &spec(), "WEEK", on),
                Ok(Listing::Listed),
                "{underlying} {on:?}"
            );
        }
        // The closed-day refusal keeps its own words, and a full day resolves.
        assert!(matches!(
            expiry_of("NIFTY", &spec(), "WEEK", "1", Day::new(2024, 8, 14).expect("a day")),
            Err(RollingError::NoExpiry { why }) if why.contains("marks closed")
        ));
        assert!(
            expiry_of(
                "NIFTY",
                &spec(),
                "WEEK",
                "1",
                Day::new(2021, 11, 8).expect("a day")
            )
            .is_ok()
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
            let got = expiry_of(u, &spec(), flag, code, on);
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

    /// One rolling body whose single `oi` cell is `cell`, read as a CALL.
    fn with_oi(cell: &str) -> Result<Vec<Row>, RollingError> {
        let body = format!(
            r#"{{"data":{{"ce":{{
            "timestamp":[1700000000],
            "open":[100.0],"high":[100.0],"low":[100.0],"close":[100.0],
            "volume":[1],"oi":[{cell}]
        }}}}}}"#
        );
        read(&body, &spec(), "CALL", PriceScale::Rupees)
    }

    /// **AN OPEN-INTEREST CELL THAT IS NOT A COUNT IS REFUSED, NEVER STORED.**
    ///
    /// GAP16-22. `number` answered `0` for a cell it could not read, truncated
    /// `1234.5` to `1234`, and passed the literal `i64::MIN` through — which is
    /// `OI_NULL`, so a vendor's number was filed as the store's absence. Each
    /// of those is a fabricated reading in a column `Bar::oi()` reports as
    /// measured. The controls pin what must NOT change: `0` is a real zero,
    /// `null` is the sentinel, and a whole count spelled `12345.0` is 12,345.
    ///
    /// `i64::MIN` and `u64::MAX` are spelled through `to_string`, and the
    /// string and whole-count cells use `12345`, which Gate 1d already
    /// declares: a quoted digit run under `crates/pull` that Gate 1d does not
    /// declare fails it as a segment-shaped literal.
    #[test]
    fn an_open_interest_cell_that_is_not_a_count_is_refused() {
        for (cell, why) in [
            (r#""12345""#.to_owned(), "a string is not a count"),
            ("1234.5".to_owned(), "a fraction is not a count"),
            (i64::MIN.to_string(), "the store's own null sentinel"),
            (u64::MAX.to_string(), "past i64"),
            ("true".to_owned(), "a boolean is not a count"),
            ("-5".to_owned(), "a count is never negative"),
        ] {
            let cell = cell.as_str();
            let got = with_oi(cell);
            assert!(
                matches!(
                    &got,
                    Err(RollingError::Uncountable { field: "oi", text }) if text == cell
                ),
                "{why}: {cell} gave {got:?}"
            );
        }
        let said = with_oi("1234.5").expect_err("refused").to_string();
        assert!(said.contains("`oi`") && said.contains("1234.5"), "{said}");
        let sentinel = with_oi(&i64::MIN.to_string())
            .expect_err("refused")
            .to_string();
        assert!(sentinel.contains("null sentinel"), "{sentinel}");

        for (cell, want) in [
            ("0", 0),
            ("12345", 12345),
            ("12345.0", 12345),
            ("null", OI_NULL),
        ] {
            let rows = with_oi(cell).unwrap_or_else(|e| panic!("{cell}: {e}"));
            assert_eq!(rows[0].bar.open_interest, want, "{cell}");
        }
    }

    /// An ordinary epoch-seconds stamp for the cell tests below.
    const STAMP: i64 = 1_700_000_000;

    /// One rolling CALL body built from the given timestamp, volume and close
    /// cells, one row.
    fn with_cells(stamp: &str, volume: &str, close: &str) -> Result<Vec<Row>, RollingError> {
        let body = format!(
            r#"{{"data":{{"ce":{{
            "timestamp":[{stamp}],
            "open":[100.0],"high":[100.0],"low":[0.0],"close":[{close}],
            "volume":[{volume}]
        }}}}}}"#
        );
        read(&body, &spec(), "CALL", PriceScale::Rupees)
    }

    /// **AN UNREADABLE TIMESTAMP OR VOLUME IS REFUSED, NEVER FILED AS ZERO**
    /// (c4a-3, W1-pull3-7, D-1491).
    ///
    /// `number` answered `0` for a `null` or text stamp — a bar at the epoch —
    /// and `saturating_mul` turned a stamp past the microsecond range into
    /// `i64::MAX`. A text volume became `0` and a negative one was accepted.
    /// The extremes: the largest stamp that fits and the first that does not,
    /// `i64::MIN`, `-19_800` (a real 1970 IST moment, still accepted), and a
    /// whole-number decimal, still accepted as on the intraday path.
    #[test]
    fn an_unreadable_stamp_or_volume_is_refused_and_never_filed_as_zero() {
        let last = i64::MAX / 1_000_000;
        for (stamp, why) in [
            ("null".to_owned(), "a null stamp"),
            (r#""x""#.to_owned(), "a text stamp"),
            (format!("{STAMP}.5"), "a fractional second"),
            ((last + 1).to_string(), "microseconds past i64"),
            (9_999_999_999_999_i64.to_string(), "the probe's stamp"),
            (i64::MIN.to_string(), "the null sentinel"),
            (u64::MAX.to_string(), "past i64"),
        ] {
            let got = with_cells(&stamp, "1", "100.0");
            assert!(
                matches!(&got, Err(RollingError::Unstampable { field: "timestamp", text }) if *text == stamp),
                "{why}: {stamp} gave {got:?}"
            );
        }
        for (stamp, want) in [
            (last.to_string(), last * 1_000_000),
            ((-19_800_i64).to_string(), -19_800_000_000),
            (format!("{STAMP}.0"), STAMP * 1_000_000),
            ("0".to_owned(), 0),
        ] {
            let rows = with_cells(&stamp, "1", "100.0").unwrap_or_else(|e| panic!("{stamp}: {e}"));
            assert_eq!(rows[0].bar.ts_micros, want, "{stamp}");
        }
        for (volume, why) in [
            (r#""abc""#, "a text volume"),
            ("-1", "a negative volume"),
            ("null", "a null volume"),
            ("2.5", "a fractional volume"),
        ] {
            let got = with_cells(&STAMP.to_string(), volume, "100.0");
            assert!(
                matches!(&got, Err(RollingError::Uncountable { field: "volume", text }) if text == volume),
                "{why}: {volume} gave {got:?}"
            );
        }
        let neg = i64::MIN.to_string();
        assert!(matches!(
            with_cells(&STAMP.to_string(), &neg, "100.0"),
            Err(RollingError::Uncountable {
                field: "volume",
                ..
            })
        ));
        for (volume, want) in [("0", 0), ("7.0", 7), (&*i64::MAX.to_string(), i64::MAX)] {
            let rows = with_cells(&STAMP.to_string(), volume, "100.0")
                .unwrap_or_else(|e| panic!("{volume}: {e}"));
            assert_eq!(rows[0].bar.volume, want, "{volume}");
        }
        let said = with_cells("null", "1", "100.0")
            .expect_err("refused")
            .to_string();
        assert!(
            said.contains("`timestamp`") && said.contains("null"),
            "{said}"
        );
    }

    /// **A NEGATIVE PRICE, OR ONE THAT SNAPS TO A ZERO IT IS NOT, IS REFUSED**
    /// (GAP16-23, D-1492), the two guards `http::one_price` has. `0`, `0.00`
    /// and `-0.0` are a real zero and stay one; half a paisa rounds up to one.
    #[test]
    fn a_rolling_price_below_zero_or_snapping_to_a_false_zero_is_refused() {
        for close in ["-5", "-0.001", "0.0001", "0.004", "-100.25"] {
            let got = with_cells(&STAMP.to_string(), "1", close);
            assert!(
                matches!(&got, Err(RollingError::NotAPrice { field: "close", text }) if text == close),
                "{close} gave {got:?}"
            );
        }
        for (close, want) in [
            ("0", 0),
            ("0.00", 0),
            ("-0.0", 0),
            ("0.005", 1),
            ("100.25", 10_025),
        ] {
            let rows = with_cells(&STAMP.to_string(), "1", close)
                .unwrap_or_else(|e| panic!("{close}: {e}"));
            assert_eq!(rows[0].bar.close, want, "{close}");
        }
        let said = with_cells(&STAMP.to_string(), "1", "-5")
            .expect_err("refused")
            .to_string();
        assert!(said.contains("`close`") && said.contains("-5"), "{said}");
    }

    /// **CE-56. A KEY REPEATED INSIDE ONE OBJECT IS TWO ANSWERS FOR ONE FIELD.**
    ///
    /// `serde_json` keeps the last of a repeated key without saying so, so
    /// `"close":[100.00],"close":[200.00]` read as a close of 200. D-1531
    /// refused this in `http::decode_body`; this reader parses on its own and
    /// never called that check.
    #[test]
    fn a_rolling_answer_repeating_a_key_in_one_object_is_refused_by_name() {
        let body = r#"{"data":{"ce":{"timestamp":[1700000000],"open":[100.00],"high":[100.00],
            "low":[100.00],"close":[100.00],"close":[200.00],"volume":[7]}}}"#;
        let why = read(body, &spec(), "CALL", PriceScale::Rupees)
            .map(|rows| rows.len())
            .expect_err("two closes for one bar is two answers");
        assert!(why.to_string().contains(r#""close""#), "{why}");

        // THE SAME KEY IN TWO OBJECTS IS NOT A REPEAT: both sides carry every
        // field name once each, and that is the vendor's ordinary answer.
        let both = r#"{"data":{"ce":{"timestamp":[1700000000],"open":[1],"high":[1],"low":[1],
            "close":[1],"volume":[1]},"pe":{"timestamp":[1700000000],"open":[1],"high":[1],
            "low":[1],"close":[1],"volume":[1]}}}"#;
        assert_eq!(
            read(both, &spec(), "CALL", PriceScale::Rupees).map(|rows| rows.len()),
            Ok(1)
        );
    }

    /// CE-68: a control character in a value is escaped, so the body parses.
    /// Before the fix `\u{1}` and a newline went out raw and no JSON parser
    /// accepted the request.
    #[test]
    fn a_control_character_in_a_value_still_makes_valid_json() {
        let mut out = String::from("{");
        push_pair(&mut out, "securityId", "13\u{1}\n\t\r\"\\x", true);
        out.push('}');
        assert_eq!(out, r#"{"securityId":"13\u0001\n\t\r\"\\x"}"#);
        let parsed: serde_json::Value = serde_json::from_str(&out).expect("valid JSON");
        assert_eq!(parsed["securityId"], "13\u{1}\n\t\r\"\\x");
    }
}

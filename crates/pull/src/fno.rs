//! THE EXPIRED-F&O CONTRACT LOOKUP — the two requests, and what comes back.
//!
//! # Why this exists at all
//!
//! An expired option is in NO instrument master. It expired; a master lists
//! what is tradable now. So unlike every other instrument in this build, its
//! name cannot be looked up — it has to be **discovered**, and only then can
//! its bars be asked for by the ordinary bars path.
//!
//! [`crate::vendor::FnoDiscovery`] records where to ask. This module builds the
//! two requests and reads the two answers. Nothing here opens a socket: a URL
//! is a `String` and an answer is a `&str`, so every arm below is drivable from
//! a test against the vendor's own published payloads, which is what the tests
//! at the bottom do.
//!
//! # The order, and why it is two calls and not one
//!
//! `expiries` answers which dates a contract expired on, for one underlying in
//! one month. `contracts` answers which contracts existed for ONE of those
//! dates. The second is fed by the first because the vendor keys it that way —
//! there is no call that returns a month's contracts directly.
//!
//! # Cost
//!
//! One request per (underlying, month) for the expiries, and one per expiry for
//! the contracts. Both are properties of the ASK, never of the store: nothing
//! here reads a census, lists a directory, or grows with what is already held.
//! The parse is one pass over the answer and allocates one `String` per name.

use crate::vendor::{FnoDiscovery, HttpSpec, ParamValue, PathSegment};

/// What a discovery request is asked about.
///
/// The four values [`ParamValue::Underlying`], [`ParamValue::Year`],
/// [`ParamValue::Month`] and [`ParamValue::ExpiryDate`] resolve from here and
/// from nowhere else — which is why they refuse in a bars request rather than
/// falling back to something. See `FetchError::NotABarsParam`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ask {
    /// The underlying's plain symbol — `NIFTY`, not a contract.
    pub underlying: String,
    /// The year being asked about.
    pub year: u16,
    /// The month being asked about, 1..=12.
    pub month: u8,
    /// One expiry date, for the contracts call. Empty for the expiries call,
    /// which is the call that produces it.
    pub expiry: String,
}

/// Why a discovery request could not be built or read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FnoError {
    /// This feed publishes no contract lookup.
    ///
    /// A fact about the vendor, not a gap here: Dhan's `rollingoption` answers
    /// an ATM-relative series whose underlying contract changes weekly, so
    /// there is no contract name in it to discover.
    NoLookup,
    /// The year asked for is before the vendor answers.
    ///
    /// Refused rather than sent, because an out-of-range answer and an empty
    /// one are the same bytes and an operator cannot tell them apart.
    BeforeFloor {
        /// The year asked for.
        asked: u16,
        /// The earliest the vendor answers for.
        floor: u16,
    },
    /// A month outside `1..=12`.
    MonthOutOfRange {
        /// The month asked for.
        month: u8,
    },
    /// The contracts call was built with no expiry to key it on.
    NoExpiry,
    /// The answer was not the shape the descriptor names.
    Unreadable {
        /// The field that was looked for.
        field: &'static str,
    },
}

impl core::fmt::Display for FnoError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match *self {
            Self::NoLookup => f.write_str(
                "this feed publishes no expired-contract lookup, so there is no \
                 name to discover and nothing was sent",
            ),
            Self::BeforeFloor { asked, floor } => write!(
                f,
                "this feed answers for {floor} onward and {asked} is before it. \
                 Nothing was sent: an out-of-range answer and an empty one are \
                 the same bytes, so a request that cannot succeed is refused \
                 here rather than reported as a month with no contracts."
            ),
            Self::MonthOutOfRange { month } => {
                write!(f, "month {month} is not 1..=12")
            }
            Self::NoExpiry => f.write_str(
                "the contracts lookup is keyed on one expiry date and none was \
                 given. It is fed by the expiries lookup; calling it first is \
                 the whole shape of this module.",
            ),
            Self::Unreadable { field } => write!(
                f,
                "the answer carries no array named {field:?} where the \
                 descriptor says the names are. Nothing was read rather than a \
                 partial list: a short list of contracts reads exactly like a \
                 month that had fewer."
            ),
        }
    }
}

/// This feed's contract lookup, or the reason it has none.
///
/// # Errors
///
/// [`FnoError::NoLookup`] where the descriptor records none.
pub const fn lookup(spec: &HttpSpec) -> Result<FnoDiscovery, FnoError> {
    match spec.fno {
        Some(d) => Ok(d),
        None => Err(FnoError::NoLookup),
    }
}

/// The URL that asks which expiries exist.
///
/// # Errors
///
/// [`FnoError::NoLookup`], [`FnoError::BeforeFloor`] or
/// [`FnoError::MonthOutOfRange`].
pub fn expiries_url(spec: &HttpSpec, ask: &Ask) -> Result<String, FnoError> {
    let d = lookup(spec)?;
    guard(&d, ask)?;
    Ok(join(spec.base_url, d.expiries_path, d.expiries_params, ask))
}

/// The URL that asks which contracts exist for one expiry.
///
/// # Errors
///
/// As [`expiries_url`], plus [`FnoError::NoExpiry`] where the ask carries no
/// expiry date to key on.
pub fn contracts_url(spec: &HttpSpec, ask: &Ask) -> Result<String, FnoError> {
    let d = lookup(spec)?;
    guard(&d, ask)?;
    if ask.expiry.is_empty() {
        return Err(FnoError::NoExpiry);
    }
    Ok(join(
        spec.base_url,
        d.contracts_path,
        d.contracts_params,
        ask,
    ))
}

/// The names an answer carries, under the field the descriptor names.
///
/// # Errors
///
/// [`FnoError::Unreadable`] where the field is absent or is not an array of
/// strings. Nothing partial is returned: a short list of contracts reads
/// exactly like a month that genuinely had fewer, and there is no way for a
/// later reader to tell the two apart.
pub fn names(body: &str, field: &'static str) -> Result<Vec<String>, FnoError> {
    let root: serde_json::Value =
        serde_json::from_str(body).map_err(|_| FnoError::Unreadable { field })?;
    // THE ENVELOPE IS OPTIONAL AND THE FIELD IS NOT. Groww wraps its answers in
    // `payload`; looking there first and at the root second means a vendor that
    // does not wrap is read by the same code rather than by a second copy of it.
    let holder = root.get("payload").unwrap_or(&root);
    let list = holder
        .get(field)
        .and_then(serde_json::Value::as_array)
        .ok_or(FnoError::Unreadable { field })?;
    let mut out = Vec::with_capacity(list.len());
    for item in list {
        let text = item.as_str().ok_or(FnoError::Unreadable { field })?;
        out.push(text.to_owned());
    }
    Ok(out)
}

/// The two refusals both lookups share.
fn guard(d: &FnoDiscovery, ask: &Ask) -> Result<(), FnoError> {
    if ask.year < d.from_year {
        return Err(FnoError::BeforeFloor {
            asked: ask.year,
            floor: d.from_year,
        });
    }
    if ask.month == 0 || ask.month > 12 {
        return Err(FnoError::MonthOutOfRange { month: ask.month });
    }
    Ok(())
}

/// One URL, path then query, with every value resolved from the ask.
fn join(base: &str, path: &[PathSegment], params: &[Param], ask: &Ask) -> String {
    let mut out = String::from(base);
    for segment in path {
        out.push('/');
        match *segment {
            PathSegment::Literal(word) => out.push_str(word),
            // A DISCOVERY PATH IS ALL LITERALS AT EVERY FEED THIS BUILD HAS.
            // A value segment resolves through the same table the query does,
            // so a feed that puts its underlying in the path is one row rather
            // than a second builder.
            PathSegment::Value { placeholder, value } => {
                out.push_str(&resolve(value, ask).unwrap_or_else(|| placeholder.to_owned()));
            }
        }
    }
    let mut first = true;
    for p in params {
        let Some(value) = resolve(p.value, ask) else {
            continue;
        };
        out.push(if first { '?' } else { '&' });
        first = false;
        out.push_str(p.name);
        out.push('=');
        out.push_str(&value);
    }
    out
}

/// What one parameter is worth for this ask, or [`None`] where this ask does
/// not carry it.
fn resolve(value: ParamValue, ask: &Ask) -> Option<String> {
    match value {
        ParamValue::Fixed(word) => Some(word.to_owned()),
        ParamValue::Underlying => Some(ask.underlying.clone()),
        ParamValue::Year => Some(ask.year.to_string()),
        ParamValue::Month => Some(ask.month.to_string()),
        ParamValue::ExpiryDate => (!ask.expiry.is_empty()).then(|| ask.expiry.clone()),
        // NOT A DISCOVERY VALUE. A rung, a window edge, an instrument id and a
        // listing class all belong to a BARS request; a discovery request has
        // no window and no rung. `None` drops the field rather than inventing
        // one, and no shipped descriptor names any of them here — asserted by
        // `no_discovery_request_names_a_bars_only_field`.
        ParamValue::From
        | ParamValue::To
        | ParamValue::InstrumentId
        | ParamValue::Granularity
        | ParamValue::Segment
        | ParamValue::Kind => None,
    }
}

use crate::vendor::Param;

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "a test that asserts nothing is banned, and a test that cannot \
              fail loudly is a test that asserts nothing"
)]
mod tests {
    use super::*;
    use crate::vendor::{Feed, Transport};

    fn groww() -> HttpSpec {
        let Transport::Http(spec) = Feed::Groww.descriptor().transport else {
            panic!("Groww is an HTTP broker");
        };
        spec
    }

    fn ask() -> Ask {
        Ask {
            underlying: "NIFTY".to_owned(),
            year: 2024,
            month: 1,
            expiry: String::new(),
        }
    }

    /// BOTH URLS ARE THE VENDOR'S OWN, byte for byte.
    ///
    /// Quoted from the curl examples on
    /// groww.in/trade-api/docs/curl/backtesting, which is the only thing that
    /// makes this a test of the descriptor rather than of a fixture.
    #[test]
    fn the_two_discovery_urls_are_the_vendors_own_worked_examples() {
        let spec = groww();
        assert_eq!(
            expiries_url(&spec, &ask()).expect("it builds"),
            "https://api.groww.in/v1/historical/expiries\
             ?exchange=NSE&underlying_symbol=NIFTY&year=2024&month=1"
        );
        let with_expiry = Ask {
            expiry: "2025-01-25".to_owned(),
            year: 2025,
            ..ask()
        };
        assert_eq!(
            contracts_url(&spec, &with_expiry).expect("it builds"),
            "https://api.groww.in/v1/historical/contracts\
             ?exchange=NSE&underlying_symbol=NIFTY&expiry_date=2025-01-25"
        );
    }

    /// THE CONTRACTS CALL IS FED BY THE EXPIRIES CALL, and says so.
    #[test]
    fn the_contracts_lookup_refuses_without_an_expiry_to_key_on() {
        assert_eq!(
            contracts_url(&groww(), &ask()),
            Err(FnoError::NoExpiry),
            "it is keyed on ONE expiry, and the expiries call is what produces it"
        );
    }

    /// A YEAR BEFORE THE VENDOR'S FLOOR IS REFUSED, NOT SENT.
    ///
    /// An out-of-range answer and an empty one are the same bytes. Sending it
    /// would report 2019 as a year NIFTY had no contracts in, which is false.
    #[test]
    fn a_year_before_the_vendors_floor_is_refused_rather_than_answered_empty() {
        let early = Ask {
            year: 2019,
            ..ask()
        };
        assert_eq!(
            expiries_url(&groww(), &early),
            Err(FnoError::BeforeFloor {
                asked: 2019,
                floor: 2020
            })
        );
        // AND THE FLOOR ITSELF IS INSIDE, not one past it.
        let at = Ask {
            year: 2020,
            ..ask()
        };
        assert!(expiries_url(&groww(), &at).is_ok(), "2020 is answerable");
    }

    /// A MONTH OUTSIDE THE CALENDAR IS REFUSED AT BOTH ENDS.
    #[test]
    fn a_month_outside_the_calendar_is_refused_at_both_ends() {
        for bad in [0u8, 13, 255] {
            assert_eq!(
                expiries_url(
                    &groww(),
                    &Ask {
                        month: bad,
                        ..ask()
                    }
                ),
                Err(FnoError::MonthOutOfRange { month: bad }),
                "month {bad}"
            );
        }
        for good in [1u8, 6, 12] {
            assert!(
                expiries_url(
                    &groww(),
                    &Ask {
                        month: good,
                        ..ask()
                    }
                )
                .is_ok(),
                "month {good}"
            );
        }
    }

    /// A FEED WITH NO LOOKUP REFUSES BY NAME, and Dhan is one.
    #[test]
    fn a_feed_that_publishes_no_contract_lookup_refuses_by_name() {
        let Transport::Http(dhan) = Feed::Dhan.descriptor().transport else {
            panic!("Dhan is an HTTP broker");
        };
        assert_eq!(expiries_url(&dhan, &ask()), Err(FnoError::NoLookup));
        assert_eq!(contracts_url(&dhan, &ask()), Err(FnoError::NoLookup));
    }

    /// BOTH ANSWERS PARSE, from the vendor's own published payloads.
    #[test]
    fn the_vendors_own_answers_parse_into_the_names_they_carry() {
        let expiries = r#"{"status":"SUCCESS","payload":{"expiries":[
            "2024-01-25","2024-01-31","2024-02-29","2024-03-28"]}}"#;
        assert_eq!(
            names(expiries, "expiries").expect("it parses"),
            vec!["2024-01-25", "2024-01-31", "2024-02-29", "2024-03-28"]
        );

        let contracts = r#"{"status":"SUCCESS","payload":{"contracts":[
            "NSE-NIFTY-02Jan25-28500-PE","NSE-NIFTY-02Jan25-28150-CE"]}}"#;
        let got = names(contracts, "contracts").expect("it parses");
        assert_eq!(got.len(), 2);
        assert_eq!(
            got,
            vec!["NSE-NIFTY-02Jan25-28500-PE", "NSE-NIFTY-02Jan25-28150-CE"],
            "the contract names arrive whole and in the vendor's own order"
        );

        // AN EMPTY MONTH IS AN ANSWER, not a failure. A month with no expiry is
        // an ordinary fact — a symbol not yet listed, or a holiday month — and
        // reading it as a fault would halt a backfill on a true statement.
        assert_eq!(
            names(r#"{"payload":{"expiries":[]}}"#, "expiries").expect("it parses"),
            Vec::<String>::new()
        );
    }

    /// A MISSING OR WRONG-SHAPED FIELD IS REFUSED, NEVER HALF-READ.
    ///
    /// A short list of contracts reads exactly like a month that had fewer, and
    /// no later reader can tell the two apart — so nothing partial is returned.
    #[test]
    fn an_answer_that_is_not_the_shape_the_descriptor_names_is_refused_whole() {
        for body in [
            r#"{"payload":{}}"#,
            r#"{"payload":{"contracts":"NSE-NIFTY"}}"#,
            r#"{"payload":{"contracts":[1,2,3]}}"#,
            r#"{"payload":{"contracts":["ok",null]}}"#,
            "not json at all",
            "",
        ] {
            assert_eq!(
                names(body, "contracts"),
                Err(FnoError::Unreadable { field: "contracts" }),
                "body: {body}"
            );
        }
    }

    /// AN UNWRAPPED ANSWER READS THROUGH THE SAME CODE.
    #[test]
    fn a_vendor_that_does_not_wrap_its_answer_is_read_by_the_same_path() {
        assert_eq!(
            names(r#"{"expiries":["2024-01-25"]}"#, "expiries").expect("it parses"),
            vec!["2024-01-25"]
        );
    }

    /// NO DISCOVERY REQUEST NAMES A BARS-ONLY FIELD.
    ///
    /// A discovery request has no window and no rung, so `From`, `To`,
    /// `InstrumentId`, `Granularity`, `Segment` and `Kind` resolve to nothing
    /// here. That is safe only while no descriptor names one — a dropped field
    /// is a request the vendor answers differently, silently.
    #[test]
    fn no_discovery_request_names_a_bars_only_field() {
        for feed in Feed::ALL {
            let Transport::Http(spec) = feed.descriptor().transport else {
                continue;
            };
            let Some(d) = spec.fno else { continue };
            for p in d.expiries_params.iter().chain(d.contracts_params) {
                assert!(
                    resolve(
                        p.value,
                        &Ask {
                            expiry: "2025-01-25".to_owned(),
                            ..ask()
                        }
                    )
                    .is_some(),
                    "{feed}'s discovery request names {:?}, which has no value \
                     in a discovery ask and would be dropped from the URL",
                    p.name
                );
            }
        }
    }
}

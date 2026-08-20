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
    // BY-NAME ONLY, AND THE REFUSAL IS NOW MORE PRECISE THAN IT WAS. This
    // module builds the two NAME-discovery requests; a feed addressed by strike
    // offset has no name to discover and is not served here, which is a
    // different sentence from "this vendor has no expired data". D-0193.
    match spec.fno.by_name() {
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

    /// THE VENDOR'S OWN CONTRACT NAMES BECOME THIS STORE'S IDENTITY.
    ///
    /// Every name here is quoted from
    /// groww.in/trade-api/docs/curl/backtesting. Two namings, one contract: the
    /// vendor writes a `DDMmmYY` expiry and a strike in RUPEES, this store
    /// writes an ISO expiry and a strike in PAISA — `CLAUDE.md` §7.
    #[test]
    fn the_vendors_contract_names_translate_into_this_stores_identity() {
        for (name, underlying, contract) in [
            (
                "NSE-NIFTY-04Jan24-19200-CE",
                "NIFTY",
                "2024-01-04-1920000-CE",
            ),
            (
                "NSE-NIFTY-02Jan25-28500-PE",
                "NIFTY",
                "2025-01-02-2850000-PE",
            ),
            ("NSE-NIFTY-30Sep25-FUT", "NIFTY", "2025-09-30-FUT"),
            (
                "BSE-SENSEX-25Sep25-79500-CE",
                "SENSEX",
                "2025-09-25-7950000-CE",
            ),
        ] {
            let got =
                read_contract(name, keyed_on(name)).unwrap_or_else(|| panic!("{name} must read"));
            assert_eq!(got.underlying, underlying, "{name}");
            assert_eq!(got.contract.as_str(), contract, "{name}");
            assert_eq!(
                got.vendor_symbol, name,
                "the vendor's own name is kept whole — it is what goes back on \
                 the wire to ask for bars"
            );
        }
    }

    /// The expiry a discovery would have been keyed on, for a `DDMmmYY` name.
    ///
    /// `read_contract` now takes the date from the contracts call rather than
    /// from the display symbol — see its own comment. These tests pre-date that
    /// and assert on names alone, so this reconstructs the date the vendor
    /// would have answered under, keeping every case asserting what it did.
    fn keyed_on(name: &str) -> brutex_core::instrument::Expiry {
        let token = name.split('-').nth(2).expect("a contract name has a token");
        let day: u8 = token[0..2].parse().expect("DDMmmYY");
        let word = token[2..5].to_ascii_lowercase();
        let month = u8::try_from(
            MONTHS
                .iter()
                .position(|m| *m == word)
                .expect("a real month word")
                + 1,
        )
        .expect("1..=12");
        let year: u16 = token[5..7].parse().expect("YY");
        brutex_core::instrument::Expiry::new(2000 + year, month, day).expect("a real expiry")
    }

    /// **A MONTHLY NAME CARRIES NO DAY, AND IT USED TO BE UNREADABLE.**
    ///
    /// Groww spells weeklies `DDMmmYY` and monthlies `MmmYY`. Its own
    /// instrument dump carries both — `NSE-NIFTY-27Mar25-29050-PE` beside
    /// `NSE-FINNIFTY-Mar25-29600-PE` and `NSE-ASIANPAINT-Feb25-FUT`. The reader
    /// demanded exactly seven characters, so every monthly contract returned
    /// `None`, went to `Chain::unreadable`, and was never fetched — while the
    /// page still called the month stored. Stock F&O and monthly index series
    /// are exactly the ones that disappeared.
    #[test]
    fn a_monthly_name_with_no_day_reads_from_the_expiry_the_call_was_keyed_on() {
        let march = brutex_core::instrument::Expiry::new(2025, 3, 27).expect("a real expiry");
        let found = read_contract("NSE-FINNIFTY-Mar25-29600-PE", march).expect("a monthly reads");

        assert_eq!(found.underlying, "FINNIFTY");
        // THE DAY COMES FROM THE DISCOVERY. It is not in the string at all, and
        // deriving it from an expiry-rule table would be inventing a date this
        // build already had exactly.
        assert_eq!(found.expiry.day(), 27);
        assert_eq!(found.contract.as_str(), "2025-03-27-2960000-PE");

        let feb = brutex_core::instrument::Expiry::new(2025, 2, 27).expect("a real expiry");
        let fut = read_contract("NSE-ASIANPAINT-Feb25-FUT", feb).expect("a monthly future reads");
        assert!(fut.contract.is_future());
        assert_eq!(fut.contract.as_str(), "2025-02-27-FUT");
    }

    /// A NAME THAT CONTRADICTS ITS OWN BATCH IS REFUSED.
    ///
    /// The date is taken from the contracts call, so the token is free to be a
    /// cross-check — and it has to be one. A call keyed on 2024-01-25 answering
    /// a name spelled `27Mar25` is a vendor answer that would otherwise be
    /// filed under January, which is the silent misfiling this whole reader
    /// exists to prevent.
    #[test]
    fn a_name_whose_token_disagrees_with_the_keyed_expiry_is_refused() {
        let january = brutex_core::instrument::Expiry::new(2024, 1, 25).expect("a real expiry");

        assert_eq!(
            read_contract("NSE-NIFTY-27Mar25-29050-PE", january),
            None,
            "a different month and year"
        );
        assert_eq!(
            read_contract("NSE-NIFTY-24Jan24-29050-PE", january),
            None,
            "the right month, the WRONG DAY — the one a monthly cannot state \
             and a weekly must"
        );
        // AND THE MONTHLY FORM OF THE SAME BATCH IS ACCEPTED. It states no day,
        // so there is no day to disagree with — month and year are all it can
        // prove and all that is asked of it.
        let monthly = read_contract("NSE-NIFTY-Jan24-29050-PE", january)
            .expect("a monthly agrees on month and year alone");
        assert_eq!(monthly.expiry.day(), 25, "the day comes from the discovery");
    }

    /// A STRIKE WITH PAISE IS EXACT, AND NEVER GOES THROUGH A FLOAT.
    ///
    /// `19200.05` is not representable in binary floating point. §7 is what
    /// forbids discovering that at the write boundary, so the two halves are
    /// parsed as integers and combined.
    #[test]
    fn a_fractional_strike_is_read_as_exact_paisa_and_a_third_place_is_refused() {
        let paisa = |s: &str| {
            read_contract(
                &format!("NSE-NIFTY-04Jan24-{s}-CE"),
                brutex_core::instrument::Expiry::new(2024, 1, 4).expect("a real expiry"),
            )
            .map(|f| f.contract.as_str().to_owned())
        };
        assert_eq!(paisa("19200"), Some("2024-01-04-1920000-CE".to_owned()));
        assert_eq!(
            paisa("19200.5"),
            Some("2024-01-04-1920050-CE".to_owned()),
            "one place is TENTHS of a rupee — fifty paise, not five"
        );
        assert_eq!(paisa("19200.05"), Some("2024-01-04-1920005-CE".to_owned()));
        assert_eq!(
            paisa("19200.0a"),
            None,
            "a non-digit in the fraction is refused — the digit check is the \
             only thing standing between `.0a` and a strike parsed as 19200.00"
        );
        assert_eq!(
            paisa("19200.005"),
            None,
            "the tick grid is two places, so a third is a name this build does \
             not understand — refused, never rounded"
        );
    }

    /// A NEGATIVE VALUE KEEPS ITS PAISE ON THE CORRECT SIDE OF ZERO.
    ///
    /// This module carried its own copy of the rupee-to-paisa conversion, and
    /// the copy parsed the sign into the rupee half and then ADDED the unsigned
    /// fraction — so `-19200.05` came back as `-1_919_995`, which is
    /// Rs -19,199.95: ten paise closer to zero than the text says, and on the
    /// wrong side of the tick. `crate::csv::paisa` applies the sign to the
    /// combined total, and this asserts the two agree.
    ///
    /// No vendor publishes a negative STRIKE, which is why nothing here caught
    /// it for as long as it stood. `CLAUDE.md` §7 fixes one law for this
    /// conversion regardless of which field happens to exercise it, and a
    /// second spelling that is wrong is worse than no second spelling.
    #[test]
    fn a_negative_value_is_converted_by_the_same_law_as_a_positive_one() {
        assert_eq!(
            paisa_of("-19200.05"),
            Some(-1_920_005),
            "the paise belong on the same side of zero as the rupees"
        );
        assert_eq!(
            paisa_of("-19200.05"),
            crate::csv::paisa("-19200.05"),
            "one law, so the two spellings must agree on every input"
        );
        assert_eq!(paisa_of("19200.05"), Some(1_920_005));
        assert_eq!(paisa_of("-0.05"), Some(-5), "a sign with no whole rupees");
        assert_eq!(
            paisa_of("-19200.005"),
            None,
            "a third place is refused on the negative side too"
        );
    }

    /// A NAME THIS BUILD CANNOT READ IS REFUSED, NEVER GUESSED AT.
    ///
    /// A wrong strike or a wrong expiry names a DIFFERENT contract, and filing
    /// it would merge two series into one file — which no later reader could
    /// undo, because the file would look like one contract that traded more.
    #[test]
    fn a_contract_name_this_build_cannot_read_is_refused_rather_than_guessed() {
        for bad in [
            "NSE-NIFTY-04Jan24-19200-XX",   // not a side
            "NSE-NIFTY-04Xxx24-19200-CE",   // not a month
            "NSE-NIFTY-4Jan24-19200-CE",    // day not two digits
            "NSE-NIFTY-04Jan24-abc-CE",     // strike not a number
            "NSE-NIFTY-32Jan24-19200-CE",   // no such day
            "NSE-NIFTY-04Jan24",            // nothing after the expiry
            "NSE-NIFTY-04Jan24-19200-CE-X", // one field too many
            "NSE--04Jan24-FUT",             // no underlying
            "NIFTY",                        // not a contract at all
            "",
        ] {
            let keyed = brutex_core::instrument::Expiry::new(2024, 1, 4).expect("a real expiry");
            assert_eq!(read_contract(bad, keyed), None, "must refuse: {bad}");
        }
    }

    /// EVERY DISCOVERED NAME ROUND-TRIPS INTO A PATH SEGMENT.
    ///
    /// The contract this produces is the same type `store::path` files bars
    /// under, so a name that reads here is a name that can be STORED — there is
    /// no second validation between discovery and the write boundary that could
    /// disagree with this one.
    #[test]
    fn one_month_holds_several_expiries_and_both_series_and_they_are_told_apart() {
        // WHY THIS EXISTS: `/pull/fno` asks a vendor for a MONTH -- that is the
        // only shape either broker publishes an expiry list in -- while the
        // operator names ONE expiry and ONE series. Narrowing the answer back
        // to the request needs both facts to survive `read_contract`, and the
        // expiry did not: it was decoded, spent on building the path segment,
        // and dropped. A caller then had nothing to compare, so every contract
        // of every expiry in the month looked equally wanted.
        let read = |n: &str| read_contract(n, keyed_on(n)).expect("each name reads");
        let sep4_call = read("NSE-NIFTY-04Sep25-24650-CE");
        let sep4_put = read("NSE-NIFTY-04Sep25-24650-PE");
        let sep11_call = read("NSE-NIFTY-11Sep25-24650-CE");
        let sep30_fut = read("NSE-NIFTY-30Sep25-FUT");

        // THE EXPIRY SURVIVES, and it is what separates the two weeklies.
        assert_eq!(sep4_call.expiry.day(), 4);
        assert_eq!(sep11_call.expiry.day(), 11);
        assert_ne!(
            sep4_call.expiry, sep11_call.expiry,
            "two weeklies in one month are different expiries, and a filter \
             that could not see this would pull both when one was asked for"
        );
        assert_eq!(
            sep4_call.expiry, sep4_put.expiry,
            "a call and a put share one"
        );

        // THE SERIES IS READABLE TOO, off the contract rather than the name.
        assert!(sep4_call.contract.is_option());
        assert!(sep4_put.contract.is_option());
        assert!(
            !sep30_fut.contract.is_option(),
            "the future is not an option, which is the other half of the filter"
        );

        // AND THE NARROWING ACTUALLY NARROWS: four contracts, and asking for
        // 4 Sep options leaves exactly the call and the put.
        let month = [sep4_call, sep4_put, sep11_call, sep30_fut];
        let kept = month
            .iter()
            .filter(|f| f.expiry.day() == 4 && f.contract.is_option())
            .count();
        assert_eq!(
            kept, 2,
            "one expiry's options, not the month's four contracts"
        );
    }

    #[test]
    fn a_read_contract_is_one_the_store_can_file_under() {
        let found = read_contract(
            "NSE-BANKNIFTY-30Sep25-52000-PE",
            brutex_core::instrument::Expiry::new(2025, 9, 30).expect("a real expiry"),
        )
        .expect("it reads");
        assert!(found.contract.is_option() && !found.contract.is_future());
        assert_eq!(found.contract.as_str(), "2025-09-30-5200000-PE");
        // AND IT FITS A PATH SEGMENT, which is the bound that made the contract
        // a segment of its own rather than part of the symbol: this whole name
        // is 26 bytes and would not have fitted a 24-byte symbol.
        assert!(found.contract.as_str().len() <= 24);
        assert!(
            "BANKNIFTY-30Sep25-52000-PE".len() > 24,
            "the joined name would NOT have fitted, which is why it is split"
        );
    }

    /// EVERY REFUSAL SAYS WHAT IT IS, AND NAMES ITS OWN NUMBERS.
    ///
    /// Five arms, five messages, and each is read by an operator who has just
    /// watched a month come back empty. A refusal that does not say WHICH month
    /// or WHICH year sends them to the vendor's dashboard instead of to the
    /// field they typed — so the arithmetic is in the sentence, not just in the
    /// variant.
    #[test]
    fn every_refusal_carries_its_own_reason_and_its_own_numbers() {
        let say = |e: &FnoError| e.to_string();

        let no = say(&FnoError::NoLookup);
        assert!(no.contains("no expired-contract lookup"), "{no}");
        assert!(no.contains("nothing was sent"), "{no}");

        let floor = say(&FnoError::BeforeFloor {
            asked: 2019,
            floor: 2020,
        });
        assert!(
            floor.contains("2019") && floor.contains("2020"),
            "both years, so the operator sees the gap they typed: {floor}"
        );
        assert!(
            floor.contains("same bytes"),
            "and WHY it was refused instead of asked: {floor}"
        );

        let month = say(&FnoError::MonthOutOfRange { month: 13 });
        assert!(month.contains("13"), "the month asked for: {month}");

        let expiry = say(&FnoError::NoExpiry);
        assert!(
            expiry.contains("expiries lookup"),
            "it names the call that produces what is missing: {expiry}"
        );

        let unread = say(&FnoError::Unreadable { field: "contracts" });
        assert!(
            unread.contains("contracts"),
            "the field it looked for: {unread}"
        );
        assert!(
            unread.contains("month that had fewer"),
            "and why nothing partial was returned: {unread}"
        );

        // NO TWO REFUSALS READ ALIKE. A message repeated across arms is a
        // message that cannot tell an operator which arm they are in.
        let all = [no, floor, month, expiry, unread];
        for (i, a) in all.iter().enumerate() {
            for b in all.iter().skip(i + 1) {
                assert_ne!(a, b, "two refusals read identically");
            }
        }
    }

    /// A DISCOVERY PATH MAY CARRY A VALUE SEGMENT, and it resolves through the
    /// same table the query string does.
    ///
    /// No shipped feed puts its underlying in the PATH — Groww's two lookups
    /// are all literals — so this drives it against a spec built here. The arm
    /// exists so a vendor that does is one descriptor row rather than a second
    /// URL builder, and an untested builder arm is one that silently emits a
    /// placeholder instead of a value.
    #[test]
    fn a_value_segment_in_a_discovery_path_resolves_like_any_other_field() {
        let base = groww();
        let d = base.fno.by_name().expect("Groww is addressed by name");
        let spec = HttpSpec {
            fno: crate::vendor::FnoAccess::ByName(FnoDiscovery {
                expiries_path: &[
                    PathSegment::Literal("v1"),
                    PathSegment::Value {
                        placeholder: "underlying_symbol",
                        value: ParamValue::Underlying,
                    },
                    PathSegment::Literal("expiries"),
                ],
                expiries_params: &[],
                ..d
            }),
            ..base
        };
        assert_eq!(
            expiries_url(&spec, &ask()).expect("it builds"),
            "https://api.groww.in/v1/NIFTY/expiries",
            "the segment carries the VALUE, not the placeholder"
        );

        // AND A SEGMENT THIS ASK CANNOT FILL FALLS BACK TO ITS PLACEHOLDER
        // rather than emitting an empty segment — `//` in a path is a different
        // URL, and a vendor would answer it differently.
        let unfillable = HttpSpec {
            fno: crate::vendor::FnoAccess::ByName(FnoDiscovery {
                expiries_path: &[PathSegment::Value {
                    placeholder: "expiry_date",
                    value: ParamValue::ExpiryDate,
                }],
                expiries_params: &[],
                ..d
            }),
            ..base
        };
        assert_eq!(
            expiries_url(&unfillable, &ask()).expect("it builds"),
            "https://api.groww.in/expiry_date",
            "an ask with no expiry leaves the placeholder standing, never an \
             empty segment"
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
            let Some(d) = spec.fno.by_name() else {
                continue;
            };
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

/// One discovered contract, in this repository's own identity.
///
/// # Why a translation is needed at all
///
/// Groww names a contract `NSE-NIFTY-04Jan24-19200-CE`: exchange, underlying,
/// a `DDMmmYY` expiry, a strike in RUPEES, and the side. This store names the
/// same contract `2024-01-04-1920000-CE` — an ISO expiry and a strike in
/// PAISA, because `CLAUDE.md` §7 makes every price an `i64` of paisa and never
/// a float.
///
/// Two namings, one contract. The translation is here, once, so no caller
/// invents a second one — and it is a REFUSAL and not a best effort: a name
/// this build cannot read is skipped and reported, never guessed at, because a
/// wrong strike names a different contract and would merge two series into one
/// file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    /// The vendor's own name, exactly as discovered — what goes back on the
    /// wire to ask for bars.
    pub vendor_symbol: String,
    /// The underlying, for the store's symbol segment.
    pub underlying: String,
    /// This store's contract segment.
    pub contract: brutex_core::instrument::Contract,
    /// When this contract expires.
    ///
    /// # Why it is kept rather than re-parsed
    ///
    /// [`read_contract`] already decodes it on the way to building
    /// [`Self::contract`], and threw it away. A caller that asked for ONE
    /// expiry then had no way to tell this contract's expiry from any other in
    /// the same month, because a rendered `Contract` is a path segment rather
    /// than a value to compare. Recovering it by parsing that segment back
    /// would be re-deriving something this function already had.
    pub expiry: brutex_core::instrument::Expiry,
    /// The strike and side, for an option — `None` for a future.
    ///
    /// # Kept for exactly the reason [`Self::expiry`] is
    ///
    /// [`read_contract`] decodes both on the way to building
    /// [`Self::contract`], and threw them away. A caller that wants to PRICE
    /// the contract needs them, and a rendered `Contract` is a path segment
    /// rather than a pair of values to compute with — recovering them by
    /// parsing that segment back would be re-deriving what this function
    /// already had, and would be a second parser to disagree with the first.
    ///
    /// A tuple rather than two `Option` fields because they are one fact: an
    /// option has both or the contract is a future and has neither, and two
    /// independent `Option`s would make "a strike with no side" representable
    /// when it is not a thing.
    pub option: Option<(
        brutex_core::price::Paisa,
        brutex_core::instrument::OptionSide,
    )>,
}

/// The twelve month tokens Groww writes, lower-cased for comparison.
const MONTHS: [&str; 12] = [
    "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
];

/// Reads one discovered contract name into this store's identity.
///
/// Returns [`None`] for a name this build cannot read. That is a REFUSAL and
/// the caller must report it rather than skip it silently: a contract dropped
/// without a word is a month that looks complete and is not.
///
/// # What is accepted
///
/// `<EXCHANGE>-<UNDERLYING>-<DDMmmYY>-<STRIKE>-<CE|PE>` for an option and
/// `<EXCHANGE>-<UNDERLYING>-<DDMmmYY>-FUT` for a future, which is the grammar
/// the vendor documents and the only one it answers with.
///
/// # The strike is rupees on the wire and paisa in the store
///
/// `19200` becomes `1920000`. A fractional strike — `19200.5` — is read the
/// same way, to `1920050`, because a strike with paise is still an exact
/// decimal and reading it as a float is what §7 forbids. Anything with more
/// than two decimal places is refused rather than rounded: the tick grid is two
/// places, so a third is a name this build does not understand.
#[must_use]
pub fn read_contract(name: &str, known: brutex_core::instrument::Expiry) -> Option<Found> {
    let mut parts = name.split('-');
    let _exchange = parts.next()?;
    let underlying = parts.next()?;
    if underlying.is_empty() {
        return None;
    }
    // THE EXPIRY COMES FROM THE DISCOVERY, NOT FROM THE NAME.
    //
    // The contracts call is keyed on ONE expiry date, so every name it answers
    // with belongs to that date and this build already has it exactly. Parsing
    // it back out of the display symbol was re-deriving something in hand — and
    // it could not be done at all for half the vendor's own names.
    //
    // Groww writes TWO expiry spellings. Weeklies carry `DDMmmYY`
    // (`NSE-NIFTY-27Mar25-29050-PE`), and monthlies carry `MmmYY` with **no day
    // at all** (`NSE-FINNIFTY-Mar25-29600-PE`, `NSE-ASIANPAINT-Feb25-FUT` — the
    // vendor's own instrument dump). `parse_expiry` required exactly seven
    // characters, so every monthly contract returned `None`, landed in
    // `Chain::unreadable`, and was never fetched — while the page still
    // reported the month STORED. Stock F&O and monthly index series are
    // precisely the ones that vanished.
    //
    // This is the same rule `brutex_core::vendor` states for the instrument
    // master: the structured field is exact, the display symbol is never an
    // input to identity. The token is still CHECKED below, because a name that
    // disagrees with the expiry it was returned under is a vendor answer this
    // build should refuse rather than file.
    let token = parts.next()?;
    if !expiry_token_agrees(token, known) {
        return None;
    }
    let expiry = known;
    let tail: Vec<&str> = parts.collect();
    let (contract, option) = match tail.as_slice() {
        // A FUTURE: nothing after the expiry but the word.
        ["FUT"] => (
            brutex_core::instrument::Contract::of(brutex_core::instrument::Kind::Future {
                expiry,
            })?,
            None,
        ),
        [strike, side] => {
            let side = match *side {
                "CE" => brutex_core::instrument::OptionSide::Call,
                "PE" => brutex_core::instrument::OptionSide::Put,
                _ => return None,
            };
            let strike = brutex_core::price::Paisa::from_raw(paisa_of(strike)?);
            (
                brutex_core::instrument::Contract::of(brutex_core::instrument::Kind::Option {
                    expiry,
                    strike,
                    side,
                })?,
                Some((strike, side)),
            )
        }
        _ => return None,
    };
    Some(Found {
        vendor_symbol: name.to_owned(),
        underlying: underlying.to_owned(),
        contract,
        expiry,
        option,
    })
}

/// Whether a name's expiry token can belong to the expiry it was returned under.
///
/// # Why this checks rather than decodes
///
/// The date is already known — see [`read_contract`]. What is worth asking of
/// the name is whether it CONTRADICTS that date, because a contracts call keyed
/// on 2024-01-25 answering a name spelled `27Mar25` is a vendor answer this
/// build must not file under January.
///
/// # The two spellings, and what each can prove
///
/// * `DDMmmYY` — a weekly. Day, month and year are all present and all three
///   are checked.
/// * `MmmYY` — a monthly. The vendor omits the day entirely, so only the month
///   and the year can be checked and the day is taken from the discovery. That
///   is not a weakness of this function: the day is **not in the string**, and
///   inventing one from an expiry-rule table would be the invention `CLAUDE.md`
///   §3 rule 1 forbids when the exact date is already in hand.
///
/// Any other shape is refused. The two-digit year reads as 20xx; Groww answers
/// from 2020 and this build stores nothing older, so there is no century to be
/// ambiguous about.
fn expiry_token_agrees(token: &str, known: brutex_core::instrument::Expiry) -> bool {
    let (day, rest) = match token.len() {
        // `27Mar25` — the day is stated, so it is checked.
        7 => match token.get(0..2).and_then(|d| d.parse::<u8>().ok()) {
            Some(day) => (Some(day), token.get(2..)),
            None => return false,
        },
        // `Mar25` — the vendor states no day for a monthly.
        5 => (None, token.get(0..)),
        _ => return false,
    };
    let Some(rest) = rest else { return false };
    let Some(word) = rest.get(0..3).map(str::to_ascii_lowercase) else {
        return false;
    };
    let Some(month) = MONTHS.iter().position(|m| *m == word) else {
        return false;
    };
    let Ok(month) = u8::try_from(month + 1) else {
        return false;
    };
    let Some(Ok(year)) = rest.get(3..).map(str::parse::<u16>) else {
        return false;
    };
    let Some(year) = 2000u16.checked_add(year) else {
        return false;
    };
    year == known.year() && month == known.month() && day.is_none_or(|stated| stated == known.day())
}

/// A strike in rupees, as an exact number of paisa.
///
/// Integer arithmetic on the two halves rather than a parse to `f64` and a
/// multiply — `19200.05` is not representable in binary floating point, and
/// §7 is what forbids finding that out at the write boundary.
fn paisa_of(text: &str) -> Option<i64> {
    // ONE LAW, NOT TWO. This used to carry its own copy of the conversion, and
    // the copy was wrong for a negative value: it parsed the sign into the rupee
    // half and then ADDED the unsigned fractional part, so `"-19200.05"` came
    // back as -1_919_995 (Rs -19,199.95) instead of -1_920_005. `crate::csv::paisa`
    // strips the sign, combines the halves, and applies the sign to the total,
    // which is the same order `Paisa::from_rupees_half_up` uses.
    //
    // `CLAUDE.md` s7 fixes ONE law for rupees to paisa. A second spelling of it
    // is a second thing to get wrong, and this one already had been -- so the
    // repair is to delete the spelling rather than to correct it in place.
    crate::csv::paisa(text)
}

//! `ingest.rs`'s tests of what no form reaches, the rate box, a clock at the
//! epoch, a date in pieces and the contract filter's named expiry
//! (BASE-COV-API-AS3).
//!
//! Every contract, strike, rate and day below is invented.

use super::*;
use brutex_core::instrument::{Contract, Expiry, Kind, OptionSide};
use brutex_core::price::Paisa;

/// A day, or a failure naming it.
fn day(year: u16, month: u8, of: u8) -> Day {
    Day::new(year, month, of).expect("a real day")
}

/// The day every request below is asked on, as `tests` asks it.
fn today() -> Day {
    day(2026, 8, 7)
}

/// One refusal written by the member writer its formatter delegates to.
struct Member<'a>(&'a Refusal);

impl fmt::Display for Member<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.write_member_refusal(f)
    }
}

/// One refusal written by the rung writer its formatter delegates to.
struct Rung<'a>(&'a Refusal);

impl fmt::Display for Rung<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.write_rung_refusal(f)
    }
}

/// **WHAT NO FORM REACHES STILL SAYS WHAT IT IS.** The target whose members
/// no published universe names answers the empty set, never a bit of another
/// row; and each of the two writers the refusal formatter delegates to, handed
/// a refusal that is not its own, says it cannot describe it rather than
/// borrowing another's sentence.
///
/// Extremes: singles — the target with no universe bit (`Everything`: the
/// empty set, never `INDEX` or `TOTAL_MARKET`), a series refusal handed to
/// the member writer, a window refusal handed to the rung writer;
/// combinations — the member writer's own refusal handed to the rung writer
/// and the rung writer's own handed to the member writer, each at once a
/// legal refusal and the wrong writer; out of the box — a refusal of the
/// machine's clock handed to both writers.
#[test]
fn what_no_form_reaches_still_says_what_it_is() {
    assert_eq!(SpotTarget::Everything.universe(), Universe::NONE);
    assert_ne!(SpotTarget::Everything.universe(), Universe::INDEX);
    assert_ne!(SpotTarget::Everything.universe(), Universe::TOTAL_MARKET);
    let member = "REFUSED · a member field this build cannot describe";
    let rung = "REFUSED · a rung this build cannot describe";
    let series = Refusal::UnknownSeries {
        got: "swaption".to_owned(),
    };
    let window = Refusal::WindowTooLong {
        days: 9_999,
        cap: 3_660,
    };
    let too_many = Refusal::TooManyMembers { got: 751, cap: 750 };
    let one_minute = Refusal::ExpiredSeriesIsOneMinuteOnly {
        got: "1day".to_owned(),
    };
    let clock = Refusal::ClockUnusable {
        why: SessionError::TimestampOutOfRange { secs: 0 },
    };
    assert_eq!(Member(&series).to_string(), member);
    assert_eq!(Member(&one_minute).to_string(), member);
    assert_eq!(Member(&clock).to_string(), member);
    assert_eq!(Rung(&window).to_string(), rung);
    assert_eq!(Rung(&too_many).to_string(), rung);
    assert_eq!(Rung(&clock).to_string(), rung);
    // EACH WRITER'S OWN REFUSAL is still its own sentence.
    assert!(Member(&too_many).to_string().contains("751 instrument(s)"));
    assert!(Rung(&one_minute).to_string().contains("ONE MINUTE only"));
}

/// **THE RATE BOX IS READ OR REFUSED BY NAME, AND THE REFUSAL NAMES ITS
/// CONTROL.** A rate inside the plausible band, edges included, is read; one
/// outside it, one that is not a number and one that is not finite are each
/// refused `UnreadableRate` in the sentence that says no greek is guessed, and
/// the form's one log line names the `rate` box; blanks alone are no rate.
///
/// Extremes: singles — exactly +1 (the band's top: read), exactly -1 (its
/// bottom: read), a percentage typed for a decimal (`9.4617`: refused), one
/// past the top (`1.0000001`: refused), not a number (`abc`), `NaN`, `inf`,
/// blanks alone (no rate, no greek); combinations — a refused rate in a body
/// whose window the clamp would narrow (the rate refused, nothing narrowed),
/// a refused rate and its log line naming the control at once; out of the
/// box — a rate of 400 digits (`f64` reads it as infinite: refused as not
/// finite).
#[test]
fn the_rate_box_is_read_or_refused_by_name_and_names_its_control() {
    let base = "underlying=NIFTY&series=opt&from=2026-07-01&to=2026-07-30";
    let read = |rate: &str| parse_fno(&format!("{base}&rate={rate}"), today());
    for edge in ["1", "-1", "0.0946"] {
        let asked = read(edge).expect("a rate inside the band");
        let rate = asked.rate.expect("a rate");
        assert_eq!(rate.annual().to_string(), edge);
    }
    assert!(read("%20%20").expect("blanks are no rate").rate.is_none());
    let huge = "9".repeat(400);
    for (typed, says) in [
        ("9.4617", "a risk-free rate of 9.4617 is outside"),
        ("1.0000001", "a risk-free rate of 1.0000001 is outside"),
        ("NaN", "is not a finite number"),
        ("inf", "is not a finite number"),
        (huge.as_str(), "is not a finite number"),
        ("abc", "abc"),
    ] {
        let refused = read(typed).expect_err("a rate refused by name");
        let Refusal::UnreadableRate { ref got } = refused else {
            panic!("{typed}: {refused:?}");
        };
        assert!(got.contains(says), "{typed}: {got}");
        let said = refused.to_string();
        assert!(
            said.starts_with("the risk-free rate could not be read: "),
            "{said}"
        );
        assert_eq!(refused_field(&refused), Some("rate"), "{typed}");
    }
    // A WINDOW THE CLAMP WOULD NARROW does not save a refused rate.
    let narrowed = "underlying=NIFTY&series=opt&from=2026-07-01&to=2026-08-07&rate=9.4618";
    let refused = parse_fno(narrowed, today()).expect_err("the rate is refused");
    let Refusal::UnreadableRate { ref got } = refused else {
        panic!("{refused:?}");
    };
    assert!(got.contains("9.4618"), "{got}");
    // THE FORM'S ONE LINE NAMES THE CONTROL.
    let from = crate::emitted::mark();
    let _refused = read("9.4619").expect_err("refused");
    let lines = crate::emitted::landed(from, "api.ingest", "form refused");
    let ours = lines
        .iter()
        .find(|record| crate::emitted::says(record, "why", "9.4619"))
        .expect("the refusal is logged");
    assert!(crate::emitted::says(ours, "field", "rate"));
    assert!(crate::emitted::says(ours, "form", "fno"));
}

/// **A CLOCK AT THE EPOCH IS REFUSED WITHOUT NAMING A CONTROL.** With
/// today in January 1970 there is no earlier month to settle, so an expired
/// series cannot be asked for at all: refused `ClockUnusable`, and the form's
/// log line carries no `field` key, since nothing the operator typed is
/// wrong. A month later January settles and the same kind of window is read.
///
/// Extremes: singles — today in January 1970 (refused), today in February
/// 1970 (January settles: read and narrowed to its last day); combinations —
/// a window wholly before that clock at once with it (the window is fine, the
/// clock is not), the same clock with an expiry named (refused at the
/// expiry's gate); out of the box — the first day the calendar holds, as
/// "today".
#[test]
fn a_clock_at_the_epoch_is_refused_without_naming_a_control() {
    let body = "underlying=NIFTY&series=opt&from=1970-01-01&to=1970-01-02";
    let from = crate::emitted::mark();
    let refused = parse_fno(body, day(1970, 1, 10)).expect_err("no month before the epoch's");
    let Refusal::ClockUnusable { .. } = refused else {
        panic!("{refused:?}");
    };
    assert_eq!(refused_field(&refused), None);
    let said = refused.to_string();
    assert!(said.contains("clock names no usable date"), "{said}");
    let lines = crate::emitted::landed(from, "api.ingest", "form refused");
    let ours = lines
        .iter()
        .find(|record| crate::emitted::says(record, "why", "clock names no usable date"))
        .expect("the refusal is logged");
    assert!(crate::emitted::says(ours, "form", "fno"));
    assert!(ours.field("field").is_none(), "no control is named");
    let named = parse_fno(&format!("{body}&expiry=1970-01-05"), day(1970, 1, 10))
        .expect_err("an expiry against a clock at the epoch");
    let Refusal::ClockUnusable { .. } = named else {
        panic!("{named:?}");
    };
    let first = parse_fno(
        "underlying=NIFTY&series=opt&from=1970-01-01&to=1970-01-01",
        day(1970, 1, 1),
    )
    .expect_err("the first day as today");
    let Refusal::ClockUnusable { .. } = first else {
        panic!("{first:?}");
    };
    let read = parse_fno(
        "underlying=NIFTY&series=opt&from=1970-01-01&to=1970-02-05",
        day(1970, 2, 10),
    )
    .expect("January settles");
    assert_eq!(read.window.to(), day(1970, 1, 31));
    assert_eq!(read.clamped_from, Some(day(1970, 2, 5)));
}

/// **A DATE IN PIECES IS FINISHED OR MISSING, NEVER GUESSED.** Any piece of
/// the picker's triple absent or empty is the field missing; unpadded pieces
/// are padded and read; an expiry sent as its year alone, or its month alone,
/// is the expiry missing, never the month it might have meant.
///
/// Extremes: singles — the year piece missing, the month missing, the day
/// missing, an unpadded month and day (`8`, `6`: read as `08`, `06`);
/// combinations — every piece present and empty at once, an expiry with only
/// its year and one with only its month (each the expiry missing); out of the
/// box — the last day the calendar holds (`9999-12-31`, read) and a day of
/// zeros (`0000-00-00`, impossible).
#[test]
fn a_date_in_pieces_is_finished_or_missing_never_guessed() {
    for body in [
        "from_m=08&from_d=06",
        "from_y=2024&from_d=06",
        "from_y=2024&from_m=08",
        "from_y=&from_m=&from_d=",
    ] {
        assert_eq!(
            parse_day_field(body, "from"),
            Err(Refusal::FieldMissing { field: "from" }),
            "{body}"
        );
    }
    assert_eq!(
        parse_day_field("from_y=2024&from_m=8&from_d=6", "from"),
        Ok(day(2024, 8, 6))
    );
    assert_eq!(parse_day("to", "9999-12-31"), Ok(day(9_999, 12, 31)));
    let zeros = parse_day("to", "0000-00-00").expect_err("no such day");
    let Refusal::DateImpossible { field, ref got, .. } = zeros else {
        panic!("{zeros:?}");
    };
    assert_eq!((field, got.as_str()), ("to", "0000-00-00"));
    for piece in ["expiry_y=2026", "expiry_m=06"] {
        let body = format!("underlying=NIFTY&series=opt&from=2026-06-01&to=2026-06-25&{piece}");
        let refused = parse_fno(&body, today()).expect_err("an expiry in part");
        assert_eq!(
            refused,
            Refusal::FieldMissing { field: "expiry" },
            "{piece}"
        );
    }
}

/// One expired NIFTY future.
fn future(year: u16, month: u8, of: u8) -> pull::fno::Found {
    let expiry = Expiry::new(year, month, of).expect("a real expiry");
    pull::fno::Found {
        vendor_symbol: format!("ZZ-FIXTURE-{year}-{month}-{of}-FUT"),
        underlying: "NIFTY".to_owned(),
        contract: Contract::of(Kind::Future { expiry }).expect("a future has a contract"),
        expiry,
        option: None,
    }
}

/// One expired NIFTY call at `strike` paisa.
fn call(year: u16, month: u8, of: u8, strike: i64) -> pull::fno::Found {
    let expiry = Expiry::new(year, month, of).expect("a real expiry");
    pull::fno::Found {
        vendor_symbol: format!("ZZ-FIXTURE-{year}-{month}-{of}-{strike}-CE"),
        underlying: "NIFTY".to_owned(),
        contract: Contract::of(Kind::Option {
            expiry,
            strike: Paisa::from_raw(strike),
            side: OptionSide::Call,
        })
        .expect("an option has a contract"),
        expiry,
        option: Some((Paisa::from_raw(strike), OptionSide::Call)),
    }
}

/// **ONE NAMED EXPIRY KEEPS ITS OWN SERIES' CONTRACTS OF THAT DAY AND NO
/// OTHER.** A futures request naming 2026-06-25 keeps the one future of that
/// day; an options request naming it keeps the one call of that day. Another
/// year, month or day, the other series, and a contract not yet settled are
/// each dropped, and the expiry is compared as a date, never as text.
///
/// Extremes: singles — another year, another month of the same year, another
/// day of the same month, a contract after the last settled day;
/// combinations — the right expiry in the wrong series (a call for a futures
/// request, a future for an options request), the wrong day in the right
/// series at once; out of the box — a call whose strike digits spell the day
/// asked for (2,026,062,500 paisa), dropped because the expiry is compared as
/// a date.
#[test]
fn one_named_expiry_keeps_its_own_series_of_that_day_and_no_other() {
    let futures = parse_fno(
        "underlying=NIFTY&series=fut&expiry=2026-06-25&from=2026-06-01&to=2026-06-25",
        today(),
    )
    .expect("a settled future");
    assert_eq!(futures.expiry, Some(day(2026, 6, 25)));
    let chain = pull::chain::Chain {
        expiries: vec!["2026-06-25".to_owned()],
        contracts: vec![
            future(2026, 6, 25),
            call(2026, 6, 25, 2_400_000),
            future(2025, 6, 25),
            future(2026, 5, 25),
            future(2026, 6, 18),
            future(2026, 8, 27),
            call(2026, 6, 18, 2_026_062_500),
        ],
        unreadable: Vec::new(),
    };
    let last = last_settled_day(today()).expect("July settles");
    assert_eq!(last, day(2026, 7, 31));
    assert_eq!(matching(&chain, &futures, last), vec![future(2026, 6, 25)]);
    let options = parse_fno(
        "underlying=NIFTY&series=opt&expiry=2026-06-25&from=2026-06-01&to=2026-06-25",
        today(),
    )
    .expect("a settled option");
    assert_eq!(
        matching(&chain, &options, last),
        vec![call(2026, 6, 25, 2_400_000)]
    );
}

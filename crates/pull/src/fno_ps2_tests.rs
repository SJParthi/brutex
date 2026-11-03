#![cfg(test)]
//! BASE-COV-PULL-PS2 (3 Nov 2026): the discovery and futures URL builders'
//! refusals, and the expiry token reader, at the arms Route B found unreached
//! on START (2e7a0b6d3b56). Every symbol and date is invented.

#![allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "a test that cannot panic cannot fail"
)]

use super::*;
use crate::session::{Day, Window};
use crate::vendor::{Feed, Param, ParamValue, PathSegment, Transport};

fn zerodha() -> HttpSpec {
    let Transport::Http(spec) = Feed::Zerodha.descriptor().transport else {
        panic!("Zerodha is an HTTP broker");
    };
    spec
}

fn ask(expiry: &str) -> Ask {
    Ask {
        underlying: "ZZIDX".to_owned(),
        year: 2024,
        month: 1,
        expiry: expiry.to_owned(),
    }
}

fn january() -> Window {
    Window::new(Day::new(2024, 1, 1).unwrap(), Day::new(2024, 1, 5).unwrap()).unwrap()
}

/// PS2-FN-01: a discovery field whose value the ask does not hold (an expiry
/// not yet known) or that belongs to a bars request only is left out of the
/// URL, never sent empty or invented; the next field still joins with `&`.
#[test]
fn a_discovery_value_the_ask_does_not_hold_is_left_out() {
    const PARAMS: &[Param] = &[
        Param {
            name: "symbol",
            value: ParamValue::Underlying,
        },
        Param {
            name: "expiry",
            value: ParamValue::ExpiryDate,
        },
        Param {
            name: "segment",
            value: ParamValue::Segment,
        },
        Param {
            name: "year",
            value: ParamValue::Year,
        },
    ];
    let path = [PathSegment::Literal("expiries")];
    assert_eq!(
        join("https://invented.invalid", &path, PARAMS, &ask("")),
        "https://invented.invalid/expiries?symbol=ZZIDX&year=2024"
    );
    assert_eq!(
        join(
            "https://invented.invalid",
            &path,
            PARAMS,
            &ask("2024-01-25")
        ),
        "https://invented.invalid/expiries?symbol=ZZIDX&expiry=2024-01-25&year=2024"
    );
    for bars_only in [
        ParamValue::From,
        ParamValue::To,
        ParamValue::InstrumentId,
        ParamValue::Granularity,
        ParamValue::Segment,
        ParamValue::Kind,
    ] {
        assert_eq!(
            resolve(bars_only, &ask("2024-01-25")),
            None,
            "{bars_only:?}"
        );
    }
}

/// PS2-FN-02: a futures candle route whose path or query names a word no
/// futures request can spell (the underlying, here) is refused by that word
/// before any URL is made; the feed's own route builds.
#[test]
fn a_futures_word_no_futures_request_can_spell_is_refused() {
    const PATH: &[PathSegment] = &[
        PathSegment::Literal("instruments"),
        PathSegment::Value {
            placeholder: "underlying",
            value: ParamValue::Underlying,
        },
    ];
    const QUERY: &[Param] = &[Param {
        name: "symbol",
        value: ParamValue::Underlying,
    }];
    let own = zerodha();
    assert!(day_candles_url(&own, 8_000_001, january(), false).is_ok());
    let by_path = HttpSpec {
        bars_path: PATH,
        rung_routes: &[],
        ..own
    };
    assert_eq!(
        day_candles_url(&by_path, 8_000_001, january(), false),
        Err("the path word \"underlying\" is not one a futures request can spell".to_owned())
    );
    let by_query = HttpSpec {
        params: QUERY,
        rung_routes: &[],
        ..own
    };
    assert_eq!(
        day_candles_url(&by_query, 8_000_001, january(), true),
        Err("the query word \"symbol\" is not one a futures request can spell".to_owned())
    );
}

/// PS2-FN-03: an expiry token agrees only in its two shapes, `27Mar25` and
/// `Mar25`, with the known date's year, month and (when stated) day: a day
/// that is not two digits, a month word that is not three whole characters,
/// a year that is not a number, a word no month has, or any other length is
/// no agreement.
#[test]
fn an_expiry_token_of_any_other_shape_never_agrees() {
    let known = brutex_core::instrument::Expiry::new(2025, 3, 27).unwrap();
    assert!(expiry_token_agrees("27Mar25", known));
    assert!(
        expiry_token_agrees("27mar25", known),
        "the month word in any case"
    );
    assert!(
        expiry_token_agrees("Mar25", known),
        "a monthly states no day"
    );
    for token in [
        "26Mar25",
        "27Apr25",
        "27Mar26",
        "x7Mar25",
        "\u{e9}\u{20ac}",
        "MarXY",
        "27MarXY",
        "Xyz25",
        "27Mar2",
        "Mar2",
        "",
        "27Mar250",
    ] {
        assert!(!expiry_token_agrees(token, known), "{token:?}");
    }
}

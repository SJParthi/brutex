//! Every `POST /pull/spot` and `POST /pull/fno` command line of
//! `config/store-steps.toml` is a form body the route's own parser accepts
//! (R4-06 repair 1: the bodies of S-33, S-34 and S-48 were NOT PROVED against
//! the parser, `verify-R4-06-r1.json`).
//!
//! The list is compiled in, so an edit of a step's body reruns this test. A
//! `<YESTERDAY>` placeholder is the operator's to fill; it is read here as the
//! day before the test's own fixed today. A `planned:` line names a form no
//! package has built: S-48's is read once, to pin that the route refuses it
//! today (which is why it is planned).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]

use pull::session::Day;

/// The tracked store-steps list.
const LIST: &str = include_str!("../../../config/store-steps.toml");

/// The test's own today, after every window the list names.
fn today() -> Day {
    Day::new(2026, 10, 17).expect("a calendar day")
}

/// Every built (not `planned:`) command line opening `POST <route> `, as its
/// body with `<YESTERDAY>` filled.
fn bodies(route: &str) -> Vec<String> {
    let head = format!("\"POST {route} ");
    LIST.lines()
        .map(str::trim)
        .filter_map(|line| line.find(&head).map(|at| &line[at + head.len()..]))
        .map(|rest| {
            let body = rest.split('"').next().unwrap_or_default();
            body.replace("<YESTERDAY>", "2026-10-16")
        })
        .collect()
}

#[test]
fn every_spot_and_fno_body_of_the_steps_list_parses() {
    let spot = bodies("/pull/spot");
    // S-33 (two rungs from 2026-10-01) and S-34 (two rungs of the equities):
    // four lines today. A count that changes is a step that changed.
    assert_eq!(spot.len(), 4, "{spot:#?}");
    for body in &spot {
        let parsed = api::ingest::parse_spot(body, today());
        assert!(parsed.is_ok(), "{body}: {parsed:?}");
    }
    // No built line posts /pull/fno at this head: S-48's line, read here
    // when it was built, was refused (`series=futures` is no slug, and the
    // route takes one `underlying=` a request), so it is `planned:` until
    // R4-28 builds the form for all underlyings at once.
    let fno = bodies("/pull/fno");
    assert!(fno.is_empty(), "{fno:#?}");
    for body in &fno {
        let parsed = api::ingest::parse_fno(body, today());
        assert!(parsed.is_ok(), "{body}: {parsed:?}");
    }
}

/// S-48's planned body is one today's route refuses, by the field it lacks:
/// when R4-28 builds the form this fails, and its builder makes the line a
/// built one, which the test above then parses.
#[test]
fn the_planned_futures_body_is_refused_by_the_route_today() {
    let head = "\"planned: POST /pull/fno ";
    let planned: Vec<&str> = LIST
        .lines()
        .filter_map(|line| line.find(head).map(|at| &line[at + head.len()..]))
        .map(|rest| rest.split(" (").next().unwrap_or_default())
        .collect();
    assert_eq!(
        planned,
        ["series=fut&vendor=zerodha&granularity=1day&from=2018-01-01&to=2026-09-30"]
    );
    let refused = api::ingest::parse_fno(planned[0], today());
    assert_eq!(
        format!("{refused:?}"),
        "Err(FieldMissing { field: \"underlying\" })"
    );
    // With one underlying named, the route still refuses it: an expired
    // contract is stored at 1min or not at all (the operator's rule of
    // 2026-08-19, in `parse_fno`), so S-48's 1day futures pull is a form the
    // route refuses by rule, not only one it lacks. R4-28 reconciles the two.
    let named = format!("underlying=NIFTY&{}", planned[0]);
    let refused = api::ingest::parse_fno(&named, today());
    assert_eq!(
        format!("{refused:?}"),
        "Err(ExpiredSeriesIsOneMinuteOnly { got: \"1day\" })",
        "{named}"
    );
    let minute = named.replace("granularity=1day", "granularity=1min");
    let parsed = api::ingest::parse_fno(&minute, today());
    assert!(parsed.is_ok(), "{minute}: {parsed:?}");
}

#[test]
fn a_body_the_parser_refuses_is_seen_here() {
    // The reader is not vacuous: a body the parser refuses fails it.
    let refused = api::ingest::parse_spot("target=nowhere&vendor=zerodha", today());
    assert!(refused.is_err(), "{refused:?}");
    assert!(bodies("/pull/nowhere").is_empty());
}

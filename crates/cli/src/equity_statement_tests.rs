//! What a stock's reports say about corporate actions, and how the ordinary
//! stored sweep prepares its month, pinned where no store is needed. D-0694.
//!
//! The stored doors are driven end to end over a generated store in
//! `audited_stored::tests`; this module holds the banners that need no store,
//! the audit header as `audit_bars` receives it, and the order the ordinary
//! sweep's preparation shares with the screen.
#![allow(clippy::expect_used, reason = "a failed fixture must fail its test")]

use super::{
    AuditOptions, Cadence, Rules, STORED_PROVENANCE, audit_bars, descend_banner, descent_banner,
    month_banner, range_opening, render_top_record,
};
use runner::audit::{CORPORATE_ACTIONS_UNCHECKED, CostScope};

/// A recorded row naming `underlying` and nothing else.
fn record(underlying: &str) -> crate::results::Record {
    let mut record = crate::results::Record::from_bytes(&[0; crate::results::STRIDE_BYTES]);
    record.underlying = crate::results::field(underlying);
    record.feed = crate::results::field("zerodha");
    record.timeframe = crate::results::field("5min");
    record
}

/// Every one-instrument banner a stored report opens with, for `underlying`.
fn banners(underlying: &str) -> [(&'static str, String); 5] {
    [
        (
            "range-all",
            range_opening(
                "zerodha",
                underlying,
                &["5min", "1min"],
                ((2025, 1), (2025, 6)),
                None,
            ),
        ),
        (
            "descend",
            descend_banner(
                "zerodha",
                underlying,
                "5min",
                (2025, 1),
                (2025, 6),
                6,
                Cadence::PerWeek(1),
            ),
        ),
        (
            "elite descent",
            descent_banner(
                "zerodha",
                underlying,
                "5min",
                ((2025, 1), (2025, 6)),
                10_000,
                400,
                5,
            ),
        ),
        (
            "audit-stored",
            month_banner("zerodha", underlying, "5min", 2025, 5, 600, "abc123"),
        ),
        ("top", render_top_record(&record(underlying), &[], None, "")),
    ]
}

/// **Every one-instrument stored banner states what a stock's figures are
/// made of, and an index's banner is what it was.** D-0694.
///
/// A stock's banner is the provenance, then gross of every charge, then
/// corporate actions unchecked, before any figure. An index's banner has
/// neither statement: the provenance runs straight into its own next line.
#[test]
fn every_one_instrument_banner_states_corporate_actions_for_a_stock_and_never_for_an_index() {
    let stock_head = format!("{STORED_PROVENANCE}{}", CostScope::CashEquity.report_note());
    for stock in ["RELIANCE", "TCS"] {
        for (surface, text) in banners(stock) {
            assert!(
                text.starts_with(&stock_head),
                "{surface} over {stock}:\n{text}"
            );
            assert_eq!(
                text.matches(CORPORATE_ACTIONS_UNCHECKED).count(),
                1,
                "{surface} over {stock}:\n{text}"
            );
        }
    }
    for index in ["NIFTY", "BANKNIFTY"] {
        for (surface, text) in banners(index) {
            let rest = text
                .strip_prefix(STORED_PROVENANCE)
                .expect("the stored banner leads");
            assert!(
                !rest.starts_with("  CASH EQUITY"),
                "{surface} over {index}: nothing may sit between the provenance and the report:\n{text}"
            );
            for equity_only in ["CORPORATE ACTIONS", "GROSS OF EVERY CHARGE", "D-0694"] {
                assert!(
                    !text.contains(equity_only),
                    "{surface} over {index} must not carry {equity_only:?}:\n{text}"
                );
            }
        }
    }
}

/// **A stock audit's charge header says corporate actions are unchecked,
/// through the whole audit, and an index audit's never does.** D-0694.
///
/// `runner::audit` proves the header renders; this proves the header the
/// caller's cost scope selects reaches the final page with the statement in
/// it, between the gross-of-every-charge paragraph and the first figure.
#[test]
fn a_stock_audit_states_corporate_actions_in_its_charge_header_and_an_index_audit_does_not() {
    let run = |cost: CostScope| {
        audit_bars(
            &super::evaluator(),
            runner::synthetic::sessions(12),
            "GENERATED TEST FIXTURE",
            1_400,
            None,
            AuditOptions {
                prepared_column: None,
                replay: None,
                execution: None,
                native_minute_execution: true,
                recording: None,
                rules: Rules::BASELINE,
                lens: runner::rank::Lens::Detectability,
                ceiling: Some(50_000),
                validate: false,
                cost,
            },
        )
    };
    let stock = run(CostScope::CashEquity);
    let header = stock
        .split_once("\nAUDIT\n")
        .and_then(|(_, rest)| rest.split_once("\nTRADES\n"))
        .map(|(header, _)| header)
        .expect("the stock audit renders its header before TRADES");
    assert!(
        header.starts_with("  CASH EQUITY run. EVERY TOTAL BELOW IS GROSS OF EVERY CHARGE.\n"),
        "{stock}"
    );
    assert!(
        header.ends_with(&format!("{CORPORATE_ACTIONS_UNCHECKED}\n")),
        "{stock}"
    );

    let index = run(CostScope::IndexSpot);
    assert!(
        index.contains("\nAUDIT\n  INDEX SPOT run. There is no brokerage"),
        "{index}"
    );
    assert!(!index.contains("CORPORATE ACTIONS"), "{index}");
}

/// **The ordinary stored sweep prepares its month in the screen's order.**
/// D-0694.
///
/// `sweep-stored` now withholds holed sessions "exactly as the other four
/// doors do". The sequence of checks and loaders in `stored_sweep_inputs` is
/// the one in `screen_range_inner`: validate the execution minutes, measure
/// the holed days from them, withhold those days from the signal bars, and
/// only then derive both reference contexts from what remains. Pinned by name
/// because the two are separate functions, and a context derived before the
/// withholding would describe sessions the column no longer holds.
#[test]
fn the_ordinary_stored_sweep_withholds_in_the_screens_order() {
    const SEQUENCE: [&str; 5] = [
        "validate_one_minute_execution(",
        "minute_gaps::days_with_interior_gaps(",
        "minute_gaps::withhold(",
        "stored::load_daily_context(",
        "stored::load_exact_minute_context(",
    ];
    let lib = include_str!("lib.rs");
    let body = |name: &str| {
        let (_, rest) = lib
            .split_once(&format!("\nfn {name}("))
            .expect("the named function exists");
        rest.split_once("\n}\n")
            .expect("the function has a closing brace")
            .0
    };
    let sweep = body("stored_sweep_inputs");
    let screen = lib
        .split_once("\nfn screen_range_inner(")
        .expect("the screen exists")
        .1;
    let (mut in_sweep, mut in_screen) = (0, 0);
    for step in SEQUENCE {
        let at_sweep = sweep[in_sweep..].find(step).map(|i| i + in_sweep);
        let at_screen = screen[in_screen..].find(step).map(|i| i + in_screen);
        assert!(
            at_sweep.is_some() && at_screen.is_some(),
            "`{step}` must appear in both, after the previous step: sweep={at_sweep:?} screen={at_screen:?}"
        );
        in_sweep = at_sweep.expect("asserted above");
        in_screen = at_screen.expect("asserted above");
    }
    assert_eq!(
        sweep.matches("minute_gaps::withhold(").count(),
        1,
        "the signal bars are withheld once, by one measured day list"
    );
}

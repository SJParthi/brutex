//! What a stock's reports say about corporate actions, and how the ordinary
//! stored sweep prepares its month, pinned where no store is needed. D-0694.
//!
//! The stored doors are driven end to end over a generated store in
//! `audited_stored::tests`; this module holds the banners that need no store,
//! the audit header and the FINDINGS statement as `audit_bars` renders them,
//! and the order the ordinary sweep's preparation shares with the screen.
#![allow(clippy::expect_used, reason = "a failed fixture must fail its test")]

use super::{
    AuditOptions, Cadence, Rules, STORED_PROVENANCE, audit_bars, descend_banner, descent_banner,
    month_banner, range_opening, render_top_record, span_banner,
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

/// Six months over `underlying` holding no bars, for the banner a span opens
/// with.
fn span(underlying: &str) -> crate::stored::Span {
    crate::stored::Span {
        bars: Vec::new(),
        vendor: brutex_core::vendor::Vendor::Zerodha,
        key: crate::stored::swept_index(underlying).expect("a swept instrument"),
        timeframe: "5min",
        asked: 6,
        found: 6,
        missing: Vec::new(),
        excluded: crate::stored::CalendarExclusion::none(),
    }
}

/// Every one-instrument banner a stored report opens with, for `underlying`.
///
/// `audit-range` stands for the span banner, which `screen` and the strict
/// audited range open with too.
fn banners(underlying: &str) -> [(&'static str, String); 6] {
    [
        (
            "audit-range",
            span_banner(
                &span(underlying),
                underlying,
                (2025, 1),
                (2025, 6),
                "abc123",
            ),
        ),
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
            // Nine command arms take their exit code from this scan, so a
            // banner line that read as a refusal would fail every stock run.
            assert!(
                !super::carries_refusal(&text),
                "{surface} over {stock} reads as a refusal:\n{text}"
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

/// The pages whose index banner sets their first heading off from the
/// provenance with a blank line of its own. Every other one-instrument page
/// runs from its provenance straight into its feed line.
const SET_OFF: [&str; 2] = ["elite descent", "top"];

/// **A stock's banner is its index banner with the note put in after the
/// provenance, and the note's closing blank line is the only line that is
/// decided per page.** AF-19.
///
/// The note closes on a blank line, and what that line is depends on the
/// page. `top` and the elite descent set their heading off from the banner
/// with a blank line of their own, and a stock's page carried one more blank
/// line there than its index page did: three before `TOP COMBINATIONS` where
/// an index had two. On those two the note's closing blank line takes the
/// place of the index page's, and is not a second one. `range-all`,
/// `descend`, `audit-stored` and the span banner run from the provenance
/// straight into their feed line, so there the note's closing blank line is
/// one the stock's page has and the index page does not. It sets the note
/// off from the report, and the index page has no note to set off.
///
/// Past that line the two banners are the same bytes with the symbol
/// swapped. Which shape each surface has is asserted, not tolerated: an
/// index page that gained or lost its blank line fails here. The claim is
/// about the banner: the index side is what the index banner was before
/// D-0694 only because the note is empty there, which
/// `every_one_instrument_banner_...` pins.
#[test]
fn a_stock_banner_is_its_index_banner_with_the_note_put_in_and_nothing_else() {
    let note = CostScope::CashEquity.report_note();
    assert!(
        note.ends_with("\n\n") && !note.ends_with("\n\n\n"),
        "premise: the note closes on one blank line:\n{note}"
    );
    let stock_head = format!("{STORED_PROVENANCE}{note}");
    for (stock, index) in [("RELIANCE", "NIFTY"), ("TCS", "BANKNIFTY")] {
        for ((surface, stock_text), (_, index_text)) in
            banners(stock).into_iter().zip(banners(index))
        {
            let stock_rest = stock_text
                .strip_prefix(&stock_head)
                .expect("a stock's banner leads with the provenance and the note");
            let index_rest = index_text
                .strip_prefix(STORED_PROVENANCE)
                .expect("an index's banner leads with the provenance");
            let sets_off = SET_OFF.contains(&surface);
            assert_eq!(
                index_rest.starts_with('\n'),
                sets_off,
                "{surface}: whether {index}'s page puts a blank line after its provenance:\n{index_text}"
            );
            let index_body = if sets_off {
                index_rest
                    .strip_prefix('\n')
                    .expect("the blank line asserted just above")
            } else {
                index_rest
            };
            assert_eq!(
                stock_rest.replace(stock, index),
                index_body,
                "{surface}: {stock}'s banner past its note against {index}'s past its provenance"
            );
        }
    }
}

/// **The note `api` serves beside a recorded run is the stored banner's own.**
/// AF-19.
///
/// `/backtest.json` takes the statement from `cli::equity_note_for`, so the
/// two cannot say different things about one instrument: a stock gets the
/// gross label and the corporate-action sentence, and an index, a symbol no
/// sweep resolves, and an empty word get nothing.
#[test]
fn the_public_equity_note_is_the_stored_banners_own() {
    for stock in ["RELIANCE", "TCS", "reliance"] {
        assert_eq!(
            super::equity_note_for(stock),
            CostScope::CashEquity.report_note(),
            "{stock}"
        );
        assert_eq!(
            super::stored_provenance(stock),
            format!("{STORED_PROVENANCE}{}", super::equity_note_for(stock)),
            "{stock}"
        );
    }
    for other in ["NIFTY", "BANKNIFTY", "FINNIFTY", "ZZQXNOTFNO", ""] {
        assert_eq!(super::equity_note_for(other), "", "{other:?}");
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
    let _knobs = super::knobs::serially();
    super::knobs::clear_all();
    let run = |cost: CostScope| generated_audit(cost, 50_000, 1_400);
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
    super::knobs::clear_all();
}

/// One unrecorded audit of `runner::synthetic` bars at `cost`, bounded by
/// `ceiling`. Nothing is written anywhere. The caller holds
/// `knobs::serially`, because the audit reads the knob store.
fn generated_audit(cost: CostScope, ceiling: usize, min_hits: u64) -> String {
    audit_bars(
        &super::evaluator(),
        runner::synthetic::sessions(12),
        "GENERATED TEST FIXTURE",
        min_hits,
        None,
        AuditOptions {
            prepared_column: None,
            replay: None,
            execution: None,
            native_minute_execution: true,
            recording: None,
            rules: Rules::BASELINE,
            lens: runner::rank::Lens::Detectability,
            ceiling: Some(ceiling),
            validate: false,
            cost,
        },
    )
}

/// **A ranked stock's FINDINGS block says corporate actions are unchecked,
/// right after its gross label, and the rung note keeps both; an index's
/// ranking and an extinct one never say it.** D-0694.
///
/// `range-all` and `pool` pass 1 keep only sections lifted out of a report,
/// so a stock's banner does not travel with its ranking. The FINDINGS label
/// D-0681's follow-up added is what does, and the sentence has to sit inside
/// that block to survive the lift. Checked on a ladder that halts at ceiling
/// 8 -- the page refuses to trade, after the ranking -- and on one that
/// completes and renders its AUDIT header too, where the rung note carries
/// the sentence once in each block.
#[test]
fn a_stock_ranking_states_corporate_actions_inside_its_findings_block() {
    let _knobs = super::knobs::serially();
    super::knobs::clear_all();
    let gross = "CASH EQUITY: EVERY FIGURE IN THIS RANKING IS GROSS OF EVERY CHARGE.";
    for (ceiling, completes) in [(8, false), (50_000, true)] {
        let stock = generated_audit(CostScope::CashEquity, ceiling, 1_400);
        assert_eq!(
            super::carries_refusal(&stock),
            !completes,
            "premise: ceiling {ceiling}:\n{stock}"
        );
        let findings = super::section_note(&stock, "FINDINGS").expect("a ranking");
        assert!(findings.contains("  rank "), "premise:\n{findings}");
        let label = findings.find(gross).expect("the gross label");
        let sentence = findings
            .find(CORPORATE_ACTIONS_UNCHECKED)
            .expect("the corporate-action sentence inside FINDINGS");
        assert!(label < sentence, "beside and after the label:\n{findings}");
        assert!(
            findings.ends_with(CORPORATE_ACTIONS_UNCHECKED),
            "the sentence closes the block, after every ranked row:\n{findings}"
        );
        if completes {
            let lifted = super::validation_note(&stock).expect("the rung note");
            assert_eq!(
                lifted.matches(CORPORATE_ACTIONS_UNCHECKED).count(),
                2,
                "once in the lifted AUDIT header and once in the lifted FINDINGS:\n{lifted}"
            );
        }
        let index = generated_audit(CostScope::IndexSpot, ceiling, 1_400);
        assert!(!index.contains("CORPORATE ACTIONS"), "{index}");
        assert_eq!(
            stock
                .replacen(&super::equity_ranking_statement(), "", 1)
                .contains(CORPORATE_ACTIONS_UNCHECKED),
            completes,
            "ceiling {ceiling}: outside FINDINGS only a rendered AUDIT header says it:\n{stock}"
        );
    }
    let extinct = generated_audit(CostScope::CashEquity, 50_000, u64::MAX);
    assert!(
        extinct.contains("nothing kept — the sweep produced no combination"),
        "premise: extinct:\n{extinct}"
    );
    assert!(!extinct.contains("CORPORATE ACTIONS"), "{extinct}");
    super::knobs::clear_all();
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

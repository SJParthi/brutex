//! Screening, tier-ladder, ranking and report policy: lane 1-b group B
//! (D-1720 onward). Each test names the finding it pins.
#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "a failed fixture must fail its test"
)]

use super::*;

/// A ranked synthetic run and the candidates a screen prices.
struct Ranked {
    bars: Vec<indicators::Candle>,
    run: runner::RankedRun,
}

impl Ranked {
    fn of(sessions: i64) -> Self {
        let bars = runner::synthetic::sessions(sessions);
        let mut ev = evaluator().expect("the synthetic evaluator builds");
        let budget = 4096;
        let ladder = ladder_within(20, Some(budget)).expect("bounded twenty-hit fixture");
        let run = runner::Sweeper::new(ladder).run_ranked(&bars, &mut ev, Horizon::DEFAULT, budget);
        Self { bars, run }
    }

    fn by_evidence(&self, take: usize) -> Vec<&runner::rank::Scored> {
        self.run.ranked.top.iter().take(take).collect()
    }
}

const NO_PRICING: Pricing<'static> = Pricing {
    recording: None,
    capture: None,
};

/// W2-cli8-8. Nothing passes the operator's rules or ANY generated tier, yet
/// rows traded. The cascade printed the strictest tier as MET, because it
/// asked whether a row was SELECTED, and `final_selection` selects the best
/// row that traded whether or not it passed.
#[test]
fn a_cascade_that_admits_nothing_prints_no_tier_as_met() {
    let fixture = Ranked::of(8);
    let by_evidence = fixture.by_evidence(2);
    assert!(!by_evidence.is_empty(), "the fixture must rank candidates");
    let column = &fixture.run.column;
    let bars = &fixture.bars;
    let facts = runner::trade::SliceFacts::of(bars, column);
    // The operator's own rules cannot be met: no cell trades u64::MAX times.
    let mut rules = Rules::elite(400, 25);
    rules.min_trades = u64::MAX;

    // THE FIXTURE'S PRECONDITION, MEASURED rather than assumed: the mildest
    // generated tier admits nothing, and yet some row traded under it.
    let ladder = tiers(bars, by_evidence[0].hits);
    let mildest = ladder
        .last()
        .expect("a generated ladder is never empty here");
    let widest = screen(
        bars,
        column,
        &by_evidence,
        Horizon::DEFAULT,
        mildest.rules(rules.top, reference_price(bars)),
        NO_PRICING,
        &facts,
    )
    .expect("the mildest tier screens");
    assert!(
        !widest.admitted_any,
        "fixture: the mildest tier admits nothing"
    );
    assert!(widest.selected.is_some(), "fixture: some row traded");

    let result = screen_cascade(
        bars,
        column,
        &by_evidence,
        Horizon::DEFAULT,
        rules,
        NO_PRICING,
        true,
        &facts,
    )
    .expect("the cascade runs");
    assert!(!result.admitted_any, "nothing was admitted");
    assert!(
        !result.text.contains(" MET -- "),
        "no tier may be reported MET when nothing passed it:\n{}",
        result.text
    );
    assert!(
        result.text.contains("NO TIER MET, INCLUDING THE MILDEST"),
        "the walk must end on the mildest tier's verdict:\n{}",
        result.text
    );
    // Every generated tier was screened and named UNMET, the mildest last.
    assert_eq!(
        result
            .text
            .lines()
            .filter(|line| line.trim_end().ends_with(" UNMET"))
            .count(),
        ladder.len(),
        "each tier is walked and reported:\n{}",
        result.text
    );
    // The ranking survives: a subject to drill into, flagged as unadmitted.
    let subject = result
        .selected
        .expect("a row traded, so the run has a subject");
    assert_eq!(subject.cell, widest.selected.expect("checked above").cell);
}

/// One fake walk: tier `i` admits when `admits[i]`, every screen is recorded,
/// and the unmet tiers are collected in the order the walk reported them.
fn walk(admits: &[bool]) -> (Vec<usize>, Vec<usize>, Result<String, String>) {
    let ladder: Vec<usize> = (0..admits.len()).collect();
    let mut screened = Vec::new();
    let mut unmet = Vec::new();
    let outcome = walk_ladder(
        &ladder,
        |tier| {
            screened.push(*tier);
            Ok((*tier, admits.get(*tier).copied().unwrap_or(false)))
        },
        |body| body.1,
        |rank, body| unmet.push((rank, body.0)),
    )
    .map(|walk| match walk {
        LadderWalk::NoneAdmit(tier, body) => format!("none {tier} {}", body.0),
        LadderWalk::Met(rank, tier, body) => format!("met {rank} {tier} {}", body.0),
        LadderWalk::Exhausted => "exhausted".to_owned(),
    });
    assert!(
        unmet.iter().all(|(rank, tier)| rank == tier),
        "rank is position"
    );
    (
        screened,
        unmet.into_iter().map(|(rank, _)| rank).collect(),
        outcome,
    )
}

/// W2-cli8-8. Every shape of the walk, decided by admission alone, strictest
/// first and stopping at the first tier that admits (D-1731).
#[test]
fn the_tier_walk_checks_each_tier_in_order_and_stops_at_the_first_admission() {
    // Nothing admits: every tier is screened once, in order. The last screen
    // is handed back as the diagnostic, so the walk never re-screens it.
    let (screened, unmet, outcome) = walk(&[false, false, false, false]);
    assert_eq!(screened, [0, 1, 2, 3]);
    assert_eq!(unmet, [0, 1, 2]);
    assert_eq!(outcome, Ok("none 3 3".to_owned()));
    // The strictest admits: ONE screen, nothing reported unmet.
    let (screened, unmet, outcome) = walk(&[true, true, true, true]);
    assert_eq!(screened, [0]);
    assert!(unmet.is_empty());
    assert_eq!(outcome, Ok("met 0 0 0".to_owned()));
    // Only the mildest admits: every stricter tier is walked and named unmet.
    let (screened, unmet, outcome) = walk(&[false, false, false, true]);
    assert_eq!(screened, [0, 1, 2, 3]);
    assert_eq!(unmet, [0, 1, 2]);
    assert_eq!(outcome, Ok("met 3 3 3".to_owned()));
    // A middle tier is the first to admit.
    let (screened, unmet, outcome) = walk(&[false, true, true, true]);
    assert_eq!(screened, [0, 1]);
    assert_eq!(unmet, [0]);
    assert_eq!(outcome, Ok("met 1 1 1".to_owned()));
    // One tier only: screened once whichever way it answers.
    let (screened, _, outcome) = walk(&[true]);
    assert_eq!(screened, [0]);
    assert_eq!(outcome, Ok("met 0 0 0".to_owned()));
    let (screened, unmet, outcome) = walk(&[false]);
    assert_eq!(screened, [0]);
    assert!(unmet.is_empty());
    assert_eq!(outcome, Ok("none 0 0".to_owned()));
    // An empty ladder screens nothing.
    let (screened, unmet, outcome) = walk(&[]);
    assert!(screened.is_empty() && unmet.is_empty());
    assert_eq!(outcome, Ok("exhausted".to_owned()));
}

/// D-1731. Admission is NOT monotone across the ladder: each tier's
/// `max_mae_ppm` becomes `grid::Levels::forced` and is merged into the stop
/// ladder, so a stricter tier prices a stop rung the mildest tier's grid may
/// lack, and a cell can pass there and nowhere milder. The walk must find that
/// tier. A mildest-first probe settled this ladder as "nothing admits".
#[test]
fn a_stricter_tier_that_alone_admits_is_found_and_reported_met() {
    let (screened, unmet, outcome) = walk(&[false, true, false, false]);
    assert_eq!(screened, [0, 1]);
    assert_eq!(unmet, [0]);
    assert_eq!(outcome, Ok("met 1 1 1".to_owned()));
    let (screened, unmet, outcome) = walk(&[true, false, false, false]);
    assert_eq!(screened, [0]);
    assert!(unmet.is_empty());
    assert_eq!(outcome, Ok("met 0 0 0".to_owned()));
}

/// A refused screen stops the walk with its reason, wherever it falls, and
/// no tier after it is screened.
#[test]
fn the_tier_walk_propagates_refusals() {
    let ladder = [0_usize, 1, 2];
    let mut screened = Vec::new();
    let refused = walk_ladder(
        &ladder,
        |tier| {
            screened.push(*tier);
            if *tier == 1 {
                Err("refused: tier 1".to_owned())
            } else {
                Ok(false)
            }
        },
        |admitted: &bool| *admitted,
        |_, _| {},
    );
    assert!(matches!(refused, Err(why) if why == "refused: tier 1"));
    assert_eq!(screened, [0, 1]);
    let first_refused = walk_ladder(
        &ladder,
        |_| Err::<bool, String>("refused: strictest".to_owned()),
        |admitted| *admitted,
        |_, _| {},
    );
    assert!(matches!(first_refused, Err(why) if why == "refused: strictest"));
}
/// The code lines of one function in `lib.rs`, comments dropped: the bodies
/// keep comments that name old shapes as history.
fn code_of(head: &str) -> String {
    let source = include_str!("lib.rs");
    let body = source
        .split_once(head)
        .map(|(_, rest)| rest.split_once("\n}\n").map_or(rest, |(body, _)| body))
        .unwrap_or_default();
    assert!(!body.is_empty(), "{head} is no longer in lib.rs");
    body.lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// W2-cli8-10. `MAX_POINTS = 0` is documented and accepted by `elite_arm` as
/// "no ceiling beyond the derived ladder", and the inner function refused it.
#[test]
fn a_zero_stop_ceiling_is_no_ceiling_and_never_the_points_refusal() {
    let zero = elite_descend_in_points_inner(
        "zerodha",
        "NIFTY",
        "1min",
        ((2026, 8), (2026, 8)),
        (0, 10),
        runner::rank::Lens::Payoff,
        None,
    );
    assert!(
        !zero.contains("stop ceiling must be"),
        "zero is the documented no-ceiling value: {zero}"
    );
    let negative = elite_descend_in_points_inner(
        "zerodha",
        "NIFTY",
        "1min",
        ((2026, 8), (2026, 8)),
        (-1, 10),
        runner::rank::Lens::Payoff,
        None,
    );
    assert!(
        negative.starts_with("refused:") && negative.contains("negative"),
        "a negative ceiling is still refused, by name: {negative}"
    );
    let words: Vec<String> = [
        "elite", "zerodha", "NIFTY", "1min", "2026", "8", "2026", "8", "0", "10",
    ]
    .iter()
    .map(|word| (*word).to_owned())
    .collect();
    let mut report = String::new();
    let _ = dispatch(&words, &mut report);
    assert!(
        !report.contains("stop ceiling must be"),
        "the CLI door agrees with its own USAGE: {report}"
    );
}

/// W2-cli8-11. One support domain for both doors: argv and the knob agree on
/// every boundary, and a million (100%) is refused by both.
#[test]
fn argv_and_the_knob_share_one_support_domain() {
    assert!(
        parse_support_ppm("1000000").is_err(),
        "100% support is refused"
    );
    let _serial = crate::knobs::serially();
    for text in ["0", "1", "999999", "1000000", "1000001", "+5", "-1", "ten"] {
        crate::knobs::clear_all();
        crate::knobs::set("BRUTEX_SUPPORT_PPM", text);
        assert_eq!(
            parse_support_ppm(text).ok(),
            support_from_knob(),
            "argv and BRUTEX_SUPPORT_PPM disagree on {text:?}"
        );
    }
    crate::knobs::clear_all();
    assert_eq!(parse_support_ppm("999999"), Ok(999_999));
    assert_eq!(parse_support_ppm("1"), Ok(1));
}

fn top_row(mean_milli_paisa: i64, t_milli: i64) -> crate::frontier::Row {
    crate::frontier::Row {
        direction: Direction::Long,
        rules: Rules::elite(0, 25),
        identity: [7; 32],
        rank: 1,
        mask_words: [1, 0, 0, 0, 0, 0],
        hits: 10,
        n: 10,
        mean_milli_paisa,
        t_milli,
        payoff_bp: 100,
        wins: 5,
        trades: 10,
        cell_wins: 5,
        pessimistic: 0,
        worst_trade: 0,
        max_drawdown: 0,
        min_win: 0,
        gross_win: 0,
        gross_loss: 0,
    }
}

fn top_text(mean_milli_paisa: i64, t_milli: i64) -> String {
    let mut record = crate::results::Record::from_bytes(&[0; crate::results::STRIDE_BYTES]);
    record.underlying = crate::results::field("NSE-NIFTY");
    record.feed = crate::results::field("zerodha");
    record.timeframe = crate::results::field("5min");
    render_top_record(&record, &[top_row(mean_milli_paisa, t_milli)], None, "")
}

/// GAP16-25. The top report reduced thousandths by truncating toward zero;
/// it rounds half away from zero now, and keeps the sign it rounds to.
#[test]
fn the_top_report_rounds_its_thousandths_half_away_from_zero() {
    let text = top_text(47_600, 2_999);
    assert!(text.contains("\u{20b9}0.48"), "47.6 paisa is 48: {text}");
    assert!(text.contains("3.00"), "t 2.999 is 3.00: {text}");
    let text = top_text(-600, -2_995);
    assert!(text.contains("-\u{20b9}0.01"), "-0.6 paisa is -1: {text}");
    assert!(text.contains("-3.00"), "t -2.995 is -3.00: {text}");
}

/// GAP16-25. The rounding helper at its edges: exact halves both ways, zero,
/// the extremes of `i64`, a divisor of one, and a divisor that is a defect.
#[test]
fn half_away_from_zero_is_symmetric_and_never_overflows() {
    for (n, d, want) in [
        (0, 1_000, 0),
        (499, 1_000, 0),
        (500, 1_000, 1),
        (501, 1_000, 1),
        (-499, 1_000, 0),
        (-500, 1_000, -1),
        (-501, 1_000, -1),
        (1_500, 1_000, 2),
        (-1_500, 1_000, -2),
        (2_000, 1_000, 2),
        (-2_000, 1_000, -2),
        (47_600, 1_000, 48),
        (2_999, 10, 300),
        (-2_995, 10, -300),
        (-2_994, 10, -299),
        (7, 1, 7),
        (-7, 1, -7),
        (i64::MAX, 1_000, 9_223_372_036_854_776),
        (i64::MIN, 1_000, -9_223_372_036_854_776),
        (i64::MAX, 10, 922_337_203_685_477_581),
        (i64::MIN, 10, -922_337_203_685_477_581),
        (i64::MAX, i64::MAX, 1),
        (i64::MIN, i64::MAX, -1),
        (5, 0, 0),
        (5, -10, 0),
    ] {
        assert_eq!(div_round_half_away(n, d), want, "{n} / {d}");
    }
    for n in -2_000..=2_000_i64 {
        assert_eq!(
            div_round_half_away(-n, 1_000),
            -div_round_half_away(n, 1_000),
            "symmetric in sign at {n}"
        );
    }
}
/// ET-strategies-trades-ranking-costs-3. A winner whose exact out-of-sample
/// midrank is strictly in the bottom half is counted there. The legacy
/// adapter halved the doubled rank and rounded 1.5 of 2 down to the median.
#[test]
fn an_exact_half_rank_in_the_bottom_half_counts_as_overfit() {
    let fold = |in_sample: Vec<i64>, out_of_sample: Vec<i64>| runner::validate::FoldResult {
        in_sample_all: in_sample,
        out_of_sample_all: out_of_sample,
        ..runner::validate::FoldResult::default()
    };
    // Three candidates; the in-sample winner (index 0) ties one other out of
    // sample below a third: midrank 1.5 of a last rank of 2, strictly past 1.
    let validated = runner::validate::Validated {
        folds: vec![fold(vec![10, 1, 1], vec![5, 9, 5])],
        refused: None,
    };
    let measured = overfitting_of(&validated).expect("a rankable fold");
    assert_eq!(measured.folds, 1);
    assert_eq!(
        measured.overfit_folds, 1,
        "midrank 1.5 of 2 is below the median"
    );
    assert_eq!(measured.median_placement, 750_000);
    // Four candidates: midrank 1.5 of 3 is exactly the midpoint, not below it.
    let even = runner::validate::Validated {
        folds: vec![fold(vec![10, 1, 1, 1], vec![5, 9, 5, 0])],
        refused: None,
    };
    let measured = overfitting_of(&even).expect("a rankable fold");
    assert_eq!(
        measured.overfit_folds, 0,
        "the exact midpoint is not below it"
    );
    assert_eq!(measured.median_placement, 500_000);
}

/// ET-strategies-trades-ranking-costs-2. The SAMPLE line names the fold count
/// the walk-forward actually ran, not the five the constant states.
#[test]
fn the_sample_line_states_the_folds_the_walk_forward_ran() {
    let fixture = Ranked::of(8);
    let bars = &fixture.bars;
    let splits = walk_forward_splits(bars.len());
    assert_ne!(
        splits, 5,
        "fixture: a span where the run's folds differ from five"
    );
    let sessions = session_index(bars).len();
    assert!(
        sessions < MIN_AUDIT_SESSIONS,
        "fixture: a thin sample prints SAMPLE"
    );
    let first = fixture.run.ranked.top.first().expect("a ranked candidate");
    let text = traded_preamble(first, Direction::Long, &fixture.run.outcome, sessions, bars);
    assert!(
        text.contains(&format!("walk-forward folds {splits} ")),
        "the SAMPLE line must state the run's own folds: {text}"
    );
    assert!(
        // An anchored walk-forward of `splits` folds cuts the span into
        // `splits + 1` windows; D-1646's divisor, kept by D-2104's merge.
        text.contains(&format!("roughly {} day(s)", sessions / (splits + 1))),
        "and divide by them: {text}"
    );
}

/// W2-cli8-1. The stop ceiling is read once per ladder, not once per rung, and
/// the trade floor once per win rate, not once per tier.
#[test]
fn the_tier_ladder_hoists_its_per_rung_and_per_rate_work() {
    let rungs = code_of("\nfn stop_rungs_in_points(");
    assert_eq!(rungs.matches("max_stop_points(bars)").count(), 1, "{rungs}");
    assert!(
        rungs
            .lines()
            .all(|line| !(line.contains(".filter(") && line.contains("max_stop_points"))),
        "the ceiling is not recomputed inside the per-rung filter: {rungs}"
    );
    let ladder = code_of("\nfn tiers(");
    let (before, inside) = ladder
        .split_once("for &max_points")
        .expect("the tier loop is still there");
    assert!(
        before.contains("trades_needed_for("),
        "memoised per rate: {ladder}"
    );
    assert!(
        !inside.contains("trades_needed_for("),
        "not per tier: {ladder}"
    );

    // And the answer is the one the per-tier call gave.
    let bars = runner::synthetic::sessions(8);
    for tier in tiers(&bars, 400) {
        assert_eq!(
            tier.min_trades,
            runner::grid::trades_needed_for(10_000, tier.min_win_rate_bp, TRADES_SEARCH_CEILING)
        );
    }
}

/// W2-cli8-7. `BRUTEX_TOP` has a named ceiling, refused by name.
#[test]
fn brutex_top_has_a_ceiling_and_says_so() {
    assert!(crate::audited_range_command::request_value(
        "BRUTEX_TOP",
        "1000"
    ));
    assert!(crate::audited_range_command::request_value(
        "BRUTEX_TOP",
        "1"
    ));
    assert!(!crate::audited_range_command::request_value(
        "BRUTEX_TOP",
        "1001"
    ));
    assert!(!crate::audited_range_command::request_value(
        "BRUTEX_TOP",
        "0"
    ));
}

/// W2-cli8-7. One refusal for every door, at both ends and at the ceiling.
#[test]
fn every_top_door_shares_one_ceiling() {
    assert_eq!(top_refusal(0), Some("TOP must be 1 or more"));
    assert_eq!(top_refusal(1), None);
    assert_eq!(top_refusal(TOP_CEILING), None);
    assert!(top_refusal(TOP_CEILING + 1).is_some_and(|why| why.contains("1000 or fewer")));
    assert!(top_refusal(usize::MAX).is_some());
    assert_eq!(measured_band(1), 32, "the floor");
    assert_eq!(
        measured_band(TOP_CEILING),
        8 * TOP_CEILING,
        "the bounded band"
    );

    let _serial = crate::knobs::serially();
    for (raw, want, refused) in [
        ("25", 25, false),
        ("1000", 1_000, false),
        ("1", 1, false),
        ("1001", 25, true),
        ("0", 25, true),
        ("-3", 25, true),
        ("lots", 25, true),
    ] {
        crate::knobs::clear_all();
        crate::knobs::set("BRUTEX_TOP", raw);
        assert_eq!(top_from_knob(), want, "BRUTEX_TOP={raw}");
        let named = crate::knobs::refused().is_some_and(|block| block.contains("BRUTEX_TOP"));
        assert_eq!(
            named, refused,
            "BRUTEX_TOP={raw} is named only when unusable"
        );
    }
    crate::knobs::clear_all();
    assert_eq!(top_from_knob(), 25, "unset is the documented 25");

    let mut report = String::new();
    let words: Vec<String> = [
        "elite", "zerodha", "NIFTY", "1min", "2026", "8", "2026", "8", "20", "1001",
    ]
    .iter()
    .map(|word| (*word).to_owned())
    .collect();
    assert_eq!(dispatch(&words, &mut report), MISUSED);
    assert!(report.contains("1000 or fewer"), "{report}");
}

/// W2-cli8-7. The parallel band measures exactly what the sequential loop did,
/// row for row, and leaves every row past the band unmeasured.
#[test]
fn the_parallel_band_matches_a_sequential_measurement() {
    let fixture = Ranked::of(8);
    let bars = &fixture.bars;
    let column = &fixture.run.column;
    let facts = runner::trade::SliceFacts::of(bars, column);
    let horizon = Horizon::DEFAULT;
    let mut rules = Rules::elite(0, 25);
    rules.top = 1;
    let stops = stop_ladder_ppm(bars, horizon.as_bars() as usize);
    let levels = grid::Levels {
        rungs: grid_rungs(bars),
        step_ppm: Some(grid_step_ppm(bars, horizon.as_bars() as usize)),
        forced: None,
        ratios: true,
        stops_ppm: &stops,
    };
    let mut rows = Vec::new();
    let mut expected = Vec::new();
    for (rank, scored) in fixture.run.ranked.top.iter().take(40).enumerate() {
        let side = Direction::Long;
        let g = grid::evaluate_over(
            bars,
            column,
            &scored.mask,
            horizon,
            side_of_direction(side),
            levels,
            &facts,
        );
        let Some(cell) = g.best().copied() else {
            continue;
        };
        expected.push(consistency_of(
            bars,
            column,
            scored,
            horizon,
            side_of_direction(side),
            &g,
            &cell,
            &facts,
        ));
        rows.push(Screened {
            side,
            scored,
            rank,
            cell,
            tightest: None,
            admitted: false,
            consistency: None,
            steady: true,
            calendar_unmeasured: false,
        });
    }
    assert!(rows.len() > 2, "fixture: rows to measure");
    measure_top(&mut rows, bars, column, horizon, rules, &facts);
    let band = measured_band(rules.top);
    assert!(
        expected.iter().take(band).any(Option::is_some),
        "fixture: something measurable"
    );
    for (at, (row, want)) in rows.iter().zip(&expected).enumerate() {
        if at < band {
            assert_eq!(&row.consistency, want, "row {at}");
        } else {
            assert_eq!(row.consistency, None, "row {at} is past the band");
        }
    }
}
/// GAP13-14. An exact rerun reuses its detail receipt and must not call that
/// receipt the leftover of an interrupted attempt.
#[test]
fn an_exact_rerun_does_not_claim_an_interrupted_attempt() {
    let root = verification_scratch().expect("a private scratch root");
    let identity = [0x5a; 32];
    let first = ensure_detail_receipt(&root, identity, 1, 1, Direction::Short)
        .expect("the first receipt is written");
    assert!(first.contains("prepared and synced"), "{first}");
    let again = ensure_detail_receipt(&root, identity, 1, 1, Direction::Short)
        .expect("the exact rerun reuses it");
    assert!(again.contains("byte-verified and reused"), "{again}");
    assert!(!again.contains("interrupted"), "{again}");
    for head in [
        "\nfn record_trades(",
        "\nfn record_frontier(",
        "\nfn ensure_detail_receipt(",
    ] {
        assert!(!code_of(head).contains("interrupted"), "{head}");
    }
    let _ = std::fs::remove_dir_all(&root);
}

/// W2-cli8-3. The frontier commit evaluates each row's key once and orders
/// only what it writes.
#[test]
fn the_frontier_commit_does_not_sort_every_retained_row() {
    let frontier = code_of("\nfn record_frontier(");
    assert!(!frontier.contains("sort_by_key("), "{frontier}");
}

/// What `record_frontier` did: a stable sort of everything by key, a `retain`
/// of `accept` in that order, then `take(top)`.
fn sort_everything(items: &[(u8, u8)], top: usize, seen_limit: u8) -> Vec<&(u8, u8)> {
    let mut ordered: Vec<&(u8, u8)> = items.iter().collect();
    ordered.sort_by_key(|item| item.0);
    let mut seen = std::collections::HashSet::with_capacity(items.len());
    ordered.retain(|item| item.1 >= seen_limit || seen.insert(item.1));
    ordered.into_iter().take(top).collect()
}

/// W2-cli8-3. Selecting the written prefix gives the old full sort's rows, in
/// its order, over keys full of ties and an `accept` full of rejections --
/// with every key evaluated exactly once.
#[test]
fn the_frontier_prefix_equals_the_full_sort_with_each_key_once() {
    // A fixed linear congruential stream: reproducible without a dependency.
    let mut state: u64 = 0x2545_f491_4f6c_dd1d;
    let mut next = |bound: u64| {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        u8::try_from((state >> 33) % bound).unwrap_or(0)
    };
    for case in 0..400_u64 {
        let len = usize::from(next(60));
        // Few distinct keys, so ties are everywhere; `.1` below `seen_limit` is
        // a "priced result" folded by the dedup set, at or above it "unpriced".
        let items: Vec<(u8, u8)> = (0..len).map(|_| (next(6), next(12))).collect();
        let top = usize::from(next(20));
        let seen_limit = next(13);
        let want = sort_everything(&items, top, seen_limit);
        let mut keys = 0_usize;
        let mut seen = std::collections::HashSet::with_capacity(items.len());
        let got = first_accepted_in_order(
            &items,
            top,
            |item| {
                keys += 1;
                item.0
            },
            |item| item.1 >= seen_limit || seen.insert(item.1),
        );
        assert_eq!(keys, items.len(), "case {case}: each key once");
        assert_eq!(
            got.iter()
                .map(|item| std::ptr::from_ref(*item))
                .collect::<Vec<_>>(),
            want.iter()
                .map(|item| std::ptr::from_ref(*item))
                .collect::<Vec<_>>(),
            "case {case}: the same rows in the same order, ties by position"
        );
    }
    // The edges by name: nothing to order, nothing asked, more asked than held,
    // every key equal, and every row rejected.
    let none: [(u8, u8); 0] = [];
    assert!(first_accepted_in_order(&none, 5, |item| item.0, |_| true).is_empty());
    let items = [(3_u8, 0_u8), (1, 1), (2, 2), (1, 3)];
    assert!(first_accepted_in_order(&items, 0, |item| item.0, |_| true).is_empty());
    let all = first_accepted_in_order(&items, 9, |item| item.0, |_| true);
    assert_eq!(all, [&items[1], &items[3], &items[2], &items[0]]);
    let equal = first_accepted_in_order(&items, 3, |_| 0_u8, |_| true);
    assert_eq!(
        equal,
        [&items[0], &items[1], &items[2]],
        "ties keep input order"
    );
    assert!(first_accepted_in_order(&items, 2, |item| item.0, |_| false).is_empty());
}
/// W2-cli8-5. The listing retains a bounded window, not every matching row.
#[test]
fn the_results_listing_retains_a_bounded_window() {
    let listing = code_of("\nfn results_at(");
    assert!(
        !listing.contains("rows.push(record)"),
        "every matching record is retained: {listing}"
    );
}

/// AC-whp-o1-2. The bootstrap family builds its slice facts once, not once
/// per candidate: the fifth site of the defect `trade.rs` names.
#[test]
fn the_bootstrap_family_builds_its_slice_facts_once() {
    let family = code_of("\nfn bootstrap_family(");
    assert!(!family.contains(concat!("trade::walk", "(")), "{family}");
    assert_eq!(
        family.matches(concat!("SliceFacts", "::of(")).count(),
        1,
        "{family}"
    );
    assert!(
        family.contains(concat!("trade::walk_over", "(")),
        "{family}"
    );
}

// ---------------------------------------------------------------------------
// D-1734: the tier walk builds its grids once per forced stop.
// ---------------------------------------------------------------------------

/// `shown_cell` exactly as it stood before D-1734 split out `fallback_cell`.
fn shown_cell_reference(g: &grid::Grid, rules: Rules) -> Option<(grid::Cell, bool)> {
    if let Some(admitted) = g.best_within(|c| rules.admits(c)).copied() {
        return Some((admitted, true));
    }
    let shown = g
        .best_within(|c| Rules::protects(rules.require_protective_exits, c))
        .copied()
        .or_else(|| g.best().copied())?;
    Some((shown, false))
}

/// THE REFERENCE SCREEN: `screen` exactly as it stood at `02e13b3`, before
/// D-1734 split it into `price_grids`, `tier_rows` and `finish_screen`, with
/// its comment lines dropped. One full grid build per call, every cell kept.
#[expect(clippy::too_many_lines, reason = "the pre-D-1734 body, verbatim")]
fn screen_reference<'a>(
    bars: &[indicators::Candle],
    column: &indicators::column::Column,
    by_evidence: &[&'a runner::rank::Scored],
    horizon: Horizon,
    rules: Rules,
    pricing: Pricing<'_>,
    facts: &runner::trade::SliceFacts,
) -> Result<ScreenResult<'a>, String> {
    let recording = pricing.recording;
    let reference = reference_price(bars);
    let stop_rungs = stop_ladder_ppm(bars, horizon.as_bars() as usize);

    let rungs = grid_rungs(bars);
    let step_ppm = grid_step_ppm(bars, horizon.as_bars() as usize);

    let levels = grid::Levels {
        rungs,
        step_ppm: Some(step_ppm),
        forced: (rules.max_mae_ppm > 0).then_some(rules.max_mae_ppm),
        ratios: true,
        stops_ppm: &stop_rungs,
    };
    let priced_cap = cap_for_budget(bars, column, horizon, by_evidence, &levels, facts);
    let captured_tier = pricing
        .capture
        .map(|capture| {
            capture.tier(candidate_trades::Tier {
                index: 0,
                eligible: by_evidence.len() as u64,
                evaluated: by_evidence.len().min(priced_cap) as u64,
                horizon: u64::from(horizon.as_bars()),
                rungs: rungs as u64,
                step_ppm: levels.step_ppm,
                forced_ppm: levels.forced,
                ratios: levels.ratios,
                rules,
                stops_ppm: stop_rungs.clone(),
            })
        })
        .transpose()?;
    let progress = GridProgress::over(by_evidence.len().min(priced_cap), recording);
    let mut rows: Vec<Screened<'_>> = by_evidence
        .par_iter()
        .take(priced_cap)
        .enumerate()
        .filter_map(|(rank, scored)| {
            if pricing
                .capture
                .is_some_and(|capture| capture.check().is_err())
            {
                return None;
            }
            let priced = [Side::Long, Side::Short].map(|s| -> Result<_, String> {
                if let Some(capture) = pricing.capture {
                    capture.check()?;
                }
                let g = grid::evaluate_over(bars, column, &scored.mask, horizon, s, levels, facts);
                let shown = shown_cell_reference(&g, rules);
                if let (Some(capture), Some(tier)) = (pricing.capture, captured_tier.as_ref()) {
                    capture.record(
                        tier,
                        &candidate_trades::Evaluated {
                            rank: rank as u64 + 1,
                            mask: &scored.mask,
                            direction: direction_of(s),
                            grid: &g,
                            selected: shown,
                        },
                    )?;
                }
                Ok((s, g, shown))
            });
            progress.tick();
            let [long, short] = priced;
            let priced = [long.ok()?, short.ok()?];
            let best = priced
                .into_iter()
                .filter_map(|(s, g, shown)| shown.map(|(cell, admitted)| (s, g, cell, admitted)))
                .filter(|&(_, _, cell, _)| cell.trades > 0)
                .max_by_key(|&(_, _, cell, admitted)| {
                    (
                        admitted,
                        ranked(cell.return_over_drawdown()),
                        ranked(cell.reward_to_risk_bp()),
                        cell.pessimistic,
                    )
                });
            let (side, g, cell, admitted) = best?;
            Some(Screened {
                rank: rank.saturating_add(1),
                side: direction_of(side),
                tightest: g.tightest_containment().copied(),
                admitted,
                cell,
                scored,
                consistency: None,
                steady: true,
                calendar_unmeasured: false,
            })
        })
        .collect();
    if let Some(capture) = pricing.capture {
        capture.check()?;
    }

    let mut priced = std::collections::HashMap::with_capacity(rows.len());
    for row in &rows {
        priced.insert(row.scored.mask.words(), (row.cell, row.side));
    }

    rows.sort_by_key(|r| money_key(&r.cell));

    measure_top(&mut rows, bars, column, horizon, rules, facts);

    rows.sort_by_key(|r| {
        let (weakest, worst_period) = r
            .consistency
            .as_ref()
            .map_or((i64::MIN, i128::MIN), calendar_terms);
        core::cmp::Reverse((
            r.admitted,
            weakest,
            worst_period,
            ranked(r.cell.return_over_drawdown()),
            ranked(r.cell.reward_to_risk_bp()),
            r.cell.pessimistic,
        ))
    });

    for row in rows.iter_mut().take(rules.top) {
        if let Some(ref c) = row.consistency {
            row.steady = c.weakest_bp() >= rules.min_weakest_bp;
            if !row.steady {
                row.admitted = false;
            }
        }
    }

    let passed = rows.iter().filter(|r| r.admitted).count();
    let mut out = rules_banner(rules, passed, rows.len());
    screen_table(&mut out, &rows, rules, reference);
    let _ = writeln!(out);

    append_consistency(&mut out, &rows, rules.top);
    let selected = final_selection(&rows, rules);
    let admitted_any = rows.iter().any(|row| row.admitted);
    Ok(ScreenResult {
        text: out,
        selected,
        priced,
        admitted_any,
    })
}

/// Everything a cascade reads from one walk, rendered so two walks compare
/// byte for byte: the variant, the rank, the page, the verdict, the subject
/// and every priced cell in mask order.
fn walk_shape(walk: &LadderWalk<'_, (Tier, Rules), ScreenResult<'_>>) -> String {
    let body = |result: &ScreenResult<'_>| {
        let mut priced: Vec<_> = result.priced.iter().collect();
        priced.sort_by_key(|(mask, _)| **mask);
        let selected = result.selected.map(|chosen| {
            (
                chosen.scored.mask.words(),
                chosen.direction,
                chosen.cell,
                chosen.rules,
            )
        });
        format!(
            "admitted_any={}\nselected={selected:?}\npriced={priced:?}\n{}",
            result.admitted_any, result.text
        )
    };
    match walk {
        LadderWalk::NoneAdmit(rank, result) => format!("none {rank}\n{}", body(result)),
        LadderWalk::Met(rank, (tier, rules), result) => {
            format!("met {rank} {tier:?} {rules:?}\n{}", body(result))
        }
        LadderWalk::Exhausted => "exhausted".to_owned(),
    }
}

/// One real slice and its candidates, shared by the D-1734 tests.
struct Slice {
    fixture: Ranked,
    facts: runner::trade::SliceFacts,
}

impl Slice {
    fn of(sessions: i64) -> Self {
        let fixture = Ranked::of(sessions);
        let facts = runner::trade::SliceFacts::of(&fixture.bars, &fixture.run.column);
        Self { fixture, facts }
    }

    /// The reference walk: one `screen_reference` per tier, unmet ranks kept.
    fn reference(&self, ladder: &[(Tier, Rules)], take: usize) -> (String, Vec<usize>) {
        let by_evidence = self.fixture.by_evidence(take);
        let mut unmet = Vec::new();
        let walk = walk_ladder(
            ladder,
            |(_, rules)| {
                screen_reference(
                    &self.fixture.bars,
                    &self.fixture.run.column,
                    &by_evidence,
                    Horizon::DEFAULT,
                    *rules,
                    NO_PRICING,
                    &self.facts,
                )
            },
            |body| body.admitted_any,
            |rank, _| unmet.push(rank),
        )
        .map(|walk| walk_shape(&walk))
        .expect("the reference walk runs");
        (walk, unmet)
    }

    /// The production walk, and how many grid passes it took.
    fn cached(&self, ladder: &[(Tier, Rules)], take: usize) -> (String, Vec<usize>, u64) {
        let by_evidence = self.fixture.by_evidence(take);
        let mut unmet = Vec::new();
        let before = GRID_PASSES.with(std::cell::Cell::get);
        let walk = walk_tiers(
            &self.fixture.bars,
            &self.fixture.run.column,
            &by_evidence,
            Horizon::DEFAULT,
            NO_PRICING,
            &self.facts,
            ladder,
            |rank| unmet.push(rank),
        )
        .map(|walk| walk_shape(&walk))
        .expect("the cached walk runs");
        let passes = GRID_PASSES.with(std::cell::Cell::get) - before;
        (walk, unmet, passes)
    }

    /// The generated ladder over this slice, with every tier's rules.
    fn generated(&self, take: usize) -> Vec<(Tier, Rules)> {
        let by_evidence = self.fixture.by_evidence(take);
        let reference = reference_price(&self.fixture.bars);
        tiers(&self.fixture.bars, by_evidence[0].hits)
            .into_iter()
            .map(|tier| (tier, tier.rules(25, reference)))
            .collect()
    }
}

/// A hand-made tier: forced stop `max_mae_ppm`, win-rate floor `rate_bp`,
/// trade floor `min_trades`, and every other floor off, so the fixture slice
/// admits at some stops and not others. Measured on `Slice::of(8)`'s first two
/// candidates: rate 0 admits at 0, 500 and 2,000 ppm and NOT at 39; rates
/// 5,000 and 9,000 admit nowhere. The equivalence tests assert their verdict
/// shapes, so a fixture drift fails loudly rather than proving less.
fn crafted(max_mae_ppm: i64, rate_bp: i64, min_trades: u64) -> (Tier, Rules) {
    let mut rules = Rules::elite(max_mae_ppm, 25);
    rules.min_rr_bp = 0;
    rules.min_win_rate_bp = rate_bp;
    rules.min_trades = min_trades;
    rules.min_assurance_bp = 0;
    rules.min_weakest_bp = 0;
    rules.min_ret_over_dd_bp = i64::MIN;
    rules.require_protective_exits = false;
    rules.min_fill_headroom_bp = 0;
    rules.min_avg_rr_bp = 0;
    let tier = Tier {
        name: "",
        max_points: max_mae_ppm,
        min_rr_bp: 0,
        min_win_rate_bp: rate_bp,
        min_trades,
    };
    (tier, rules)
}

/// Six tiers over three forced stops, none of which admits on the fixture.
fn three_stops_none_admit() -> Vec<(Tier, Rules)> {
    vec![
        crafted(39, 0, 0),
        crafted(500, 9_000, 0),
        crafted(2_000, 5_000, 0),
        crafted(39, 5_000, 0),
        crafted(500, 5_000, 0),
        crafted(2_000, 0, u64::MAX),
    ]
}

fn distinct_keys(ladder: &[(Tier, Rules)]) -> usize {
    ladder
        .iter()
        .map(|(_, rules)| GridKey::of(rules))
        .collect::<std::collections::HashSet<_>>()
        .len()
}

/// D-1734. One SCREEN: the split `screen` answers exactly as the monolithic
/// reference did, under policies that admit nothing, admit at a forced stop,
/// and admit with no forced stop at all.
#[test]
fn the_split_screen_equals_the_monolithic_screen() {
    let slice = Slice::of(8);
    let bars = &slice.fixture.bars;
    let column = &slice.fixture.run.column;
    let by_evidence = slice.fixture.by_evidence(6);
    let ladder = slice.generated(6);
    let mut policies: Vec<Rules> = vec![
        Rules::elite(0, 25),
        Rules::elite(400, 3),
        crafted(0, 0, 0).1,
        crafted(500, 0, 0).1,
        crafted(39, 0, 0).1,
        crafted(2_000, 5_000, 0).1,
    ];
    policies.push(ladder[0].1);
    policies.push(ladder[ladder.len() - 1].1);
    let mut admitted = 0;
    for rules in policies {
        let split = screen(
            bars,
            column,
            &by_evidence,
            Horizon::DEFAULT,
            rules,
            NO_PRICING,
            &slice.facts,
        )
        .expect("the split screen runs");
        let whole = screen_reference(
            bars,
            column,
            &by_evidence,
            Horizon::DEFAULT,
            rules,
            NO_PRICING,
            &slice.facts,
        )
        .expect("the reference screen runs");
        admitted += usize::from(whole.admitted_any);
        let shape = |result: &ScreenResult<'_>| {
            let mut priced: Vec<_> = result.priced.iter().collect();
            priced.sort_by_key(|(mask, _)| **mask);
            format!(
                "{} {:?} {priced:?}\n{}",
                result.admitted_any,
                result
                    .selected
                    .map(|s| (s.scored.mask.words(), s.direction, s.cell, s.rules)),
                result.text
            )
        };
        assert_eq!(shape(&split), shape(&whole), "{rules:?}");
    }
    assert!(admitted > 0, "fixture: at least one policy admits a row");
}

/// D-1734. Pruning a grid to its key's envelope changes no tier's shown cell:
/// for every tier of the generated ladder and every candidate side, the
/// pruned grid shows exactly what the full grid shows.
#[test]
fn a_pruned_grid_shows_every_keyed_tier_what_the_full_grid_shows() {
    let slice = Slice::of(8);
    let bars = &slice.fixture.bars;
    let column = &slice.fixture.run.column;
    let horizon = Horizon::DEFAULT;
    // The generated ladder's first and last tiers, and hand-made tiers over
    // three stops that share keys at different floors, so step 1 of
    // `shown_cell` is exercised against an envelope milder than the tier.
    let generated = slice.generated(4);
    let mut ladder = vec![generated[0], generated[generated.len() - 1]];
    ladder.extend(three_stops_none_admit());
    ladder.extend([crafted(500, 0, 0), crafted(2_000, 0, 0), crafted(0, 0, 0)]);
    // A stricter tier AFTER the mild one on the same key: the envelope is the
    // per-floor minimum, never the last tier's floors.
    ladder.extend([
        crafted(500, 4_800, 0),
        crafted(500, 9_000, 0),
        crafted(0, 0, 64),
        crafted(0, 5_000, u64::MAX),
    ]);
    let rules: Vec<Rules> = ladder.iter().map(|(_, rules)| *rules).collect();
    let stops = stop_ladder_ppm(bars, horizon.as_bars() as usize);
    let keys: std::collections::HashSet<GridKey> = rules.iter().map(GridKey::of).collect();
    let (mut compared, mut admitted, mut pruned_away) = (0_usize, 0_usize, 0_usize);
    for key in keys {
        let envelope = key.envelope(&rules);
        let (mut kept, mut full_cells) = (0_usize, 0_usize);
        let levels = grid::Levels {
            rungs: grid_rungs(bars),
            step_ppm: Some(grid_step_ppm(bars, horizon.as_bars() as usize)),
            forced: (key.max_mae_ppm > 0).then_some(key.max_mae_ppm),
            ratios: true,
            stops_ppm: &stops,
        };
        for scored in slice.fixture.by_evidence(4) {
            for side in [Side::Long, Side::Short] {
                let full = grid::evaluate_over(
                    bars,
                    column,
                    &scored.mask,
                    horizon,
                    side,
                    levels,
                    &slice.facts,
                );
                let pruned = prune_cells(full.clone(), envelope);
                assert_eq!(pruned.stops, full.stops);
                assert_eq!(pruned.targets, full.targets);
                assert_eq!(pruned.trails, full.trails);
                assert_eq!(pruned.signals, full.signals);
                assert_eq!(pruned.refused_paths, full.refused_paths);
                pruned_away += full.cells.len() - pruned.cells.len();
                kept += pruned.cells.len();
                full_cells += full.cells.len();
                for tier in rules.iter().filter(|r| GridKey::of(r) == key) {
                    let want = shown_cell_reference(&full, *tier);
                    assert_eq!(shown_cell(&pruned, *tier), want, "{tier:?}");
                    assert_eq!(shown_cell(&full, *tier), want, "{tier:?}");
                    admitted += usize::from(want.is_some_and(|(_, ok)| ok));
                    compared += 1;
                }
            }
        }
        // WHAT THE WALK KEEPS RESIDENT for this key is exactly these pruned
        // cells, and never more than the full grids held.
        let by_evidence = slice.fixture.by_evidence(4);
        let grids = price_grids(
            bars,
            column,
            &by_evidence,
            horizon,
            envelope,
            NO_PRICING,
            &slice.facts,
        )
        .expect("the key's grids price");
        assert_eq!(grids.cells_held(), kept, "{key:?}");
        assert!(kept <= full_cells, "{key:?}");
    }
    assert!(
        compared > 0 && admitted > 0,
        "fixture: admitted and fallback cells"
    );
    assert!(pruned_away > 0, "fixture: pruning removed cells");
}

/// A named ladder and the verdict prefix its reference walk must give.
type WalkCase = (&'static str, Vec<(Tier, Rules)>, &'static str);

/// D-1734. THE EQUIVALENCE PROOF ON REAL SCREENS: the cached walk returns the
/// reference walk's answer, page and unmet list over every shape of ladder:
/// nothing admits (the generated ladder), the first tier admits, only a later
/// tier admits, only the last admits, ties, two forced stops interleaved, and
/// an empty ladder.
#[test]
fn the_cached_tier_walk_equals_the_full_walk_on_real_screens() {
    let slice = Slice::of(8);
    let generated = slice.generated(2);
    let none = |mae| crafted(mae, 9_000, 0);
    let yes = |mae| crafted(mae, 0, 0);
    let cases: Vec<WalkCase> = vec![
        (
            "generated ladder, one stop, nothing admits",
            generated.clone(),
            "none",
        ),
        (
            "three stops, nothing admits",
            three_stops_none_admit(),
            "none 5",
        ),
        (
            "the first admits",
            vec![yes(500), none(39), none(2_000)],
            "met 0",
        ),
        (
            "only a later tier admits, at a looser stop than an unmet one",
            vec![yes(39), crafted(2_000, 5_000, 0), yes(500), yes(2_000)],
            "met 2",
        ),
        (
            "one key: strict floors unmet, the envelope's own tier met",
            vec![none(500), crafted(500, 5_000, 0), yes(500)],
            "met 2",
        ),
        (
            "only the last admits",
            vec![yes(39), crafted(500, 5_000, 0), none(2_000), yes(0)],
            "met 3",
        ),
        (
            "one key: the mild tier first, a stricter one after it",
            vec![none(39), crafted(500, 4_800, 0), crafted(500, 9_000, 0)],
            "met 1",
        ),
        (
            "one key: a trade floor first, an unreachable one after it",
            vec![crafted(2_000, 0, 64), crafted(2_000, 0, u64::MAX)],
            "met 0",
        ),
        (
            "ties: identical admitting tiers",
            vec![yes(39), yes(500), yes(500)],
            "met 1",
        ),
        ("one tier, unmet", vec![none(500)], "none 0"),
        ("one tier, met", vec![yes(2_000)], "met 0"),
        ("empty", Vec::new(), "exhausted"),
    ];
    for (name, ladder, verdict) in cases {
        let (want, want_unmet) = slice.reference(&ladder, 2);
        assert!(want.starts_with(verdict), "fixture {name}: {want}");
        let (got, got_unmet, passes) = slice.cached(&ladder, 2);
        assert_eq!(got, want, "{name}");
        assert_eq!(got_unmet, want_unmet, "{name}");
        assert!(
            passes <= distinct_keys(&ladder) as u64,
            "{name}: {passes} passes"
        );
        if want.starts_with("none") {
            assert_eq!(
                want_unmet.len() + 1,
                ladder.len(),
                "{name}: every tier walked"
            );
        }
    }
}

/// D-1734. THE COUNT: one grid pass per distinct forced stop, not per tier.
/// The fixture's generated ladder is 2,520 tiers over ONE stop, and the
/// hand-made one six tiers over three; when nothing admits, every tier is
/// judged and every stop priced exactly once.
#[test]
fn the_tier_walk_builds_one_grid_pass_per_distinct_forced_stop() {
    let slice = Slice::of(8);
    let ladder = slice.generated(2);
    let stops: std::collections::HashSet<i64> =
        ladder.iter().map(|(_, rules)| rules.max_mae_ppm).collect();
    assert_eq!(
        distinct_keys(&ladder),
        stops.len(),
        "within one cascade the keys are the forced stops"
    );
    assert!(
        stops.len() < ladder.len(),
        "fixture: {} tiers share {} stops",
        ladder.len(),
        stops.len()
    );
    let (walk, unmet, passes) = slice.cached(&ladder, 2);
    assert!(walk.starts_with("none "), "fixture: nothing admits");
    assert_eq!(unmet.len() + 1, ladder.len(), "every tier was judged");
    assert_eq!(passes, stops.len() as u64, "one grid pass per forced stop");
    // THREE STOPS, SIX TIERS, INTERLEAVED: three passes, not six.
    let three = three_stops_none_admit();
    let (walk, unmet, passes) = slice.cached(&three, 2);
    assert!(walk.starts_with("none "), "fixture: nothing admits");
    assert_eq!(unmet.len() + 1, three.len(), "every tier was judged");
    assert_eq!(distinct_keys(&three), 3);
    assert_eq!(passes, 3, "one grid pass per forced stop, not per tier");
    // And a single screen is one pass.
    let before = GRID_PASSES.with(std::cell::Cell::get);
    let by_evidence = slice.fixture.by_evidence(2);
    let _ = screen(
        &slice.fixture.bars,
        &slice.fixture.run.column,
        &by_evidence,
        Horizon::DEFAULT,
        ladder[0].1,
        NO_PRICING,
        &slice.facts,
    )
    .expect("one screen runs");
    assert_eq!(GRID_PASSES.with(std::cell::Cell::get) - before, 1);
}

/// A fake tier for the generic walk: its cache key, whether it admits, and
/// whether its judgement refuses.
#[derive(Clone, Copy, Debug)]
struct Fake {
    key: u8,
    admits: bool,
    refuses: bool,
}

const fn fake(key: u8, admits: bool) -> Fake {
    Fake {
        key,
        admits,
        refuses: false,
    }
}

type FakeWalk = (Result<String, String>, Vec<usize>, Vec<u8>);

fn fake_shape(walk: Result<LadderWalk<'_, Fake, (usize, bool)>, String>) -> Result<String, String> {
    walk.map(|walk| match walk {
        LadderWalk::NoneAdmit(rank, body) => format!("none {rank} {body:?}"),
        LadderWalk::Met(rank, tier, body) => format!("met {rank} {tier:?} {body:?}"),
        LadderWalk::Exhausted => "exhausted".to_owned(),
    })
}

/// The reference walk over fakes: every screen pays its key's build.
fn fake_reference(ladder: &[Fake]) -> FakeWalk {
    let mut unmet = Vec::new();
    let mut builds = Vec::new();
    let walk = walk_ladder(
        ladder,
        |tier| {
            builds.push(tier.key);
            if tier.refuses {
                return Err(format!("refused: key {}", tier.key));
            }
            Ok((usize::from(tier.key), tier.admits))
        },
        |body| body.1,
        |rank, _| unmet.push(rank),
    );
    (fake_shape(walk), unmet, builds)
}

/// The cached walk over the same fakes, recording each build.
fn fake_cached(ladder: &[Fake], refuse_build: Option<u8>) -> FakeWalk {
    let mut unmet = Vec::new();
    let mut builds = Vec::new();
    let judge = |prepared: &u8, tier: &Fake| -> Result<Option<(usize, bool)>, String> {
        assert_eq!(
            *prepared, tier.key,
            "a tier is judged on its own key's build"
        );
        if tier.refuses {
            return Err(format!("refused: key {}", tier.key));
        }
        Ok(tier.admits.then_some((usize::from(tier.key), true)))
    };
    let walk = walk_ladder_cached(
        ladder,
        |tier| tier.key,
        |key| {
            builds.push(*key);
            if refuse_build == Some(*key) {
                return Err(format!("refused: key {key}"));
            }
            Ok(*key)
        },
        judge,
        |prepared, tier| {
            judge(prepared, tier).map(|body| body.unwrap_or((usize::from(tier.key), false)))
        },
        |body| body.1,
        |rank| unmet.push(rank),
    );
    (fake_shape(walk), unmet, builds)
}

/// D-1734. THE EQUIVALENCE PROOF ON THE WALK ITSELF: over every shape, the
/// cached walk reports what the reference reports, judges the same tiers,
/// and builds each key once.
#[test]
fn the_cached_walk_equals_the_reference_walk_and_builds_each_key_once() {
    let cases: Vec<(&str, Vec<Fake>)> = vec![
        (
            "nothing admits",
            vec![
                fake(0, false),
                fake(1, false),
                fake(0, false),
                fake(1, false),
            ],
        ),
        (
            "the first admits",
            vec![fake(0, true), fake(1, false), fake(0, false)],
        ),
        (
            "only a stricter later tier admits",
            vec![
                fake(0, false),
                fake(1, false),
                fake(2, true),
                fake(0, false),
            ],
        ),
        (
            "only the mildest admits",
            vec![fake(0, false), fake(1, false), fake(0, true)],
        ),
        (
            "ties on one key",
            vec![fake(3, false), fake(3, true), fake(3, true)],
        ),
        (
            "ties on two keys",
            vec![fake(1, false), fake(2, true), fake(1, true)],
        ),
        ("one tier, unmet", vec![fake(0, false)]),
        ("one tier, met", vec![fake(0, true)]),
        ("empty", Vec::new()),
        (
            "a refusal propagates",
            vec![
                fake(0, false),
                Fake {
                    key: 1,
                    admits: true,
                    refuses: true,
                },
                fake(0, true),
            ],
        ),
        (
            "the last tier refuses",
            vec![
                fake(0, false),
                Fake {
                    key: 0,
                    admits: false,
                    refuses: true,
                },
            ],
        ),
    ];
    for (name, ladder) in cases {
        let (want, want_unmet, screened) = fake_reference(&ladder);
        let (got, got_unmet, builds) = fake_cached(&ladder, None);
        assert_eq!(got, want, "{name}");
        assert_eq!(got_unmet, want_unmet, "{name}");
        // One build per DISTINCT key among the tiers the reference screened.
        let mut reached = screened.clone();
        reached.sort_unstable();
        reached.dedup();
        let mut built = builds.clone();
        built.sort_unstable();
        assert_eq!(built, reached, "{name}: each reached key built once");
        // And in first-reached order.
        let mut first: Vec<u8> = Vec::new();
        for key in screened {
            if !first.contains(&key) {
                first.push(key);
            }
        }
        assert_eq!(builds, first, "{name}");
    }
    // A refused BUILD stops the walk with its reason, and nothing after it is
    // judged or built.
    let (got, unmet, builds) =
        fake_cached(&[fake(0, false), fake(1, true), fake(2, true)], Some(1));
    assert_eq!(got, Err("refused: key 1".to_owned()));
    assert_eq!(unmet, [0]);
    assert_eq!(builds, [0, 1]);
}

/// D-1734. The envelope is the per-floor MINIMUM over exactly the rules that
/// share its key, whatever their order: never the last tier's floors, never a
/// floor from another key, and the keyed fields are the key's own.
#[test]
fn the_envelope_is_each_floors_minimum_over_its_own_key() {
    let mut strict = crafted(500, 9_000, 400).1;
    strict.min_rr_bp = 300;
    strict.min_assurance_bp = 7_000;
    strict.min_weakest_bp = 6_000;
    let mut mild = crafted(500, 4_000, 20).1;
    mild.min_rr_bp = 100;
    mild.min_assurance_bp = 2_000;
    mild.min_weakest_bp = 1_000;
    let mut mixed = crafted(500, 6_000, 10).1;
    mixed.min_rr_bp = 200;
    mixed.min_assurance_bp = 3_000;
    mixed.min_weakest_bp = 9_000;
    // Another key, milder on every floor: it must not leak in.
    let other = crafted(2_000, 0, 0).1;
    let key = GridKey::of(&strict);
    assert_eq!(key, GridKey::of(&mild));
    assert_ne!(key, GridKey::of(&other));
    for order in [
        [strict, mild, mixed, other],
        [other, mixed, mild, strict],
        [mild, other, strict, mixed],
    ] {
        let envelope = key.envelope(&order);
        assert_eq!(envelope.min_rr_bp, 100);
        assert_eq!(envelope.min_win_rate_bp, 4_000);
        assert_eq!(envelope.min_trades, 10);
        assert_eq!(envelope.min_assurance_bp, 2_000);
        assert_eq!(envelope.min_weakest_bp, 1_000);
        assert_eq!(
            GridKey::of(&envelope),
            key,
            "the keyed fields are the key's"
        );
    }
    // A key no rule carries admits nothing at all.
    let empty = GridKey::of(&crafted(77, 0, 0).1).envelope(&[strict, other]);
    assert_eq!(
        (empty.min_rr_bp, empty.min_win_rate_bp, empty.min_trades),
        (i64::MAX, i64::MAX, u64::MAX)
    );
    assert_eq!(
        (empty.min_assurance_bp, empty.min_weakest_bp),
        (i64::MAX, i64::MAX)
    );
}

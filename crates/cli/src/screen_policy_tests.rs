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
        0,
        10,
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
        -1,
        10,
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
        text.contains(&format!("roughly {} day(s)", sessions / splits)),
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

fn ledger_row(
    at: u64,
    feed: &str,
    underlying: &str,
    pessimistic: i64,
    trades: u64,
) -> crate::results::Record {
    let mut record = crate::results::Record::from_bytes(&[0; crate::results::STRIDE_BYTES]);
    let low = u8::try_from(at % 251).unwrap_or(0);
    let high = u8::try_from(at / 251).unwrap_or(0);
    record.identity = [low; 32];
    record.identity[0] = high;
    record.finished_micros = i64::try_from(at).unwrap_or(0);
    record.feed = crate::results::field(feed);
    record.underlying = crate::results::field(underlying);
    record.timeframe = crate::results::field("15min");
    record.months_asked = 1;
    record.months_found = 1;
    record.trades = trades;
    record.pessimistic = pessimistic;
    record.optimistic = pessimistic.saturating_add(100);
    record.exit_rungs = [-1; 5];
    record
}

/// The listing as it was before D-1729, kept as the reference: every matching
/// record retained, the best chosen over all of them.
fn listing_reference(
    root: &std::path::Path,
    feed: Option<&str>,
    underlying: Option<&str>,
) -> (Vec<crate::results::Record>, Option<crate::results::Record>) {
    let mut store = crate::results::Results::open_read(root).expect("ledger");
    let count = store.len().expect("length");
    let mut rows = Vec::new();
    for back in 1..=count {
        let record = store.read(count - back).expect("row");
        if feed.is_none_or(|f| crate::results::read_field(&record.feed) == f)
            && underlying.is_none_or(|u| crate::results::read_field(&record.underlying) == u)
        {
            rows.push(record);
        }
    }
    let best = best_complete_newest_first(&rows).copied();
    (rows, best)
}

/// W2-cli8-5. The bounded window answers exactly what the full retention did:
/// the same first forty, the same best (ties to the newest), the same count.
#[test]
fn the_listing_window_equals_full_retention_and_keeps_at_most_forty() {
    let root = verification_scratch().expect("scratch");
    // Empty ledger: nothing matches and nothing is kept.
    {
        let _ = crate::results::Results::open(&root).expect("ledger");
        let mut store = crate::results::Results::open_read(&root).expect("ledger");
        let count = store.len().expect("length");
        let window = listing_window(&mut store, count, None, None).expect("window");
        assert_eq!((window.matching, window.shown.len()), (0, 0));
        assert!(window.best.is_none());
    }
    let mut ledger = crate::results::Results::open(&root).expect("ledger");
    for at in 0..500_u64 {
        let feed = if at % 3 == 0 { "dhan" } else { "zerodha" };
        let underlying = if at % 5 == 0 {
            "NSE-BANKNIFTY"
        } else {
            "NSE-NIFTY"
        };
        // Few distinct totals, so the best is a tie many times over; every
        // seventh row traded nothing and can never be the best.
        let pessimistic = i64::try_from(at % 9).unwrap_or(0) * 100 - 400;
        let trades = if at % 7 == 0 { 0 } else { 3 };
        ledger
            .append(&ledger_row(at, feed, underlying, pessimistic, trades))
            .expect("append");
    }
    drop(ledger);
    for (feed, underlying) in [
        (None, None),
        (Some("zerodha"), Some("NSE-NIFTY")),
        (Some("dhan"), Some("NSE-BANKNIFTY")),
        (Some("absent"), Some("NSE-NIFTY")),
    ] {
        let (rows, best) = listing_reference(&root, feed, underlying);
        let mut store = crate::results::Results::open_read(&root).expect("ledger");
        let count = store.len().expect("length");
        let window = listing_window(&mut store, count, feed, underlying).expect("window");
        assert_eq!(window.matching, rows.len(), "{feed:?} {underlying:?}");
        assert!(window.shown.len() <= LIST_ROWS, "O(1) retained");
        assert_eq!(
            window.shown.capacity(),
            LIST_ROWS,
            "never grown past the window"
        );
        assert_eq!(
            window.shown.as_slice(),
            rows.get(..rows.len().min(LIST_ROWS)).unwrap_or_default()
        );
        assert_eq!(window.best, best, "{feed:?} {underlying:?}");
    }
    // Every row incomplete: the window still lists them and names no best.
    let lone = verification_scratch().expect("scratch");
    let mut ledger = crate::results::Results::open(&lone).expect("ledger");
    for at in 0..3 {
        ledger
            .append(&ledger_row(at, "zerodha", "NSE-NIFTY", 500, 0))
            .expect("append");
    }
    drop(ledger);
    let mut store = crate::results::Results::open_read(&lone).expect("ledger");
    let window = listing_window(&mut store, 3, None, None).expect("window");
    assert_eq!((window.matching, window.shown.len()), (3, 3));
    assert!(window.best.is_none());
    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&lone);
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

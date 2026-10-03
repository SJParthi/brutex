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
    let by_evidence = fixture.by_evidence(8);
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
        result.text.contains("every tier UNMET"),
        "the probe must settle the walk:\n{}",
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

/// W2-cli8-8. Every shape of the walk, decided by admission alone.
#[test]
fn the_tier_walk_stops_on_admission_and_probes_the_mildest_first() {
    // Nothing admits: ONE screen, the mildest, and the walk is settled.
    let (screened, unmet, outcome) = walk(&[false, false, false, false]);
    assert_eq!(screened, [3]);
    assert!(unmet.is_empty());
    assert_eq!(outcome, Ok("none 3 3".to_owned()));
    // The strictest admits: the probe, then tier 0, nothing reported unmet.
    let (screened, unmet, outcome) = walk(&[true, true, true, true]);
    assert_eq!(screened, [3, 0]);
    assert!(unmet.is_empty());
    assert_eq!(outcome, Ok("met 0 0 0".to_owned()));
    // Only the mildest admits: every stricter tier is walked and named unmet.
    let (screened, unmet, outcome) = walk(&[false, false, false, true]);
    assert_eq!(screened, [3, 0, 1, 2, 3]);
    assert_eq!(unmet, [0, 1, 2]);
    assert_eq!(outcome, Ok("met 3 3 3".to_owned()));
    // A middle tier is the first to admit.
    let (screened, unmet, outcome) = walk(&[false, true, true, true]);
    assert_eq!(screened, [3, 0, 1]);
    assert_eq!(unmet, [0]);
    assert_eq!(outcome, Ok("met 1 1 1".to_owned()));
    // One tier only, admitting: the probe and the walk screen it once each.
    let (screened, _, outcome) = walk(&[true]);
    assert_eq!(screened, [0, 0]);
    assert_eq!(outcome, Ok("met 0 0 0".to_owned()));
    // An empty ladder screens nothing.
    let (screened, unmet, outcome) = walk(&[]);
    assert!(screened.is_empty() && unmet.is_empty());
    assert_eq!(outcome, Ok("exhausted".to_owned()));
}

/// The probe admitted and no walked tier did: only a screen that answers
/// differently twice can do that, and it is reported as exhausted, never MET.
/// A refused screen stops the walk with its reason.
#[test]
fn the_tier_walk_reports_an_inconsistent_ladder_and_propagates_refusals() {
    let ladder = [0_usize, 1];
    let mut calls = 0_usize;
    let mut unmet = Vec::new();
    let outcome = walk_ladder(
        &ladder,
        |_| {
            calls += 1;
            Ok(calls == 1)
        },
        |admitted| *admitted,
        |rank, _| unmet.push(rank),
    );
    assert!(matches!(outcome, Ok(LadderWalk::Exhausted)));
    assert_eq!(unmet, [0, 1]);

    let refused = walk_ladder(
        &ladder,
        |tier| {
            if *tier == 1 {
                Ok(true)
            } else {
                Err("refused: tier 0".to_owned())
            }
        },
        |admitted: &bool| *admitted,
        |_, _| {},
    );
    assert!(matches!(refused, Err(why) if why == "refused: tier 0"));
    let probe_refused = walk_ladder(
        &ladder,
        |_| Err::<bool, String>("refused: probe".to_owned()),
        |admitted| *admitted,
        |_, _| {},
    );
    assert!(matches!(probe_refused, Err(why) if why == "refused: probe"));
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
